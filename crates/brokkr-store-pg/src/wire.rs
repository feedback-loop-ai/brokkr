//! Synchronous pure-std PostgreSQL v3 wire protocol client.
//!
//! Implements startup, `AuthenticationOk` (trust),
//! `AuthenticationCleartextPassword`, and `AuthenticationSASL`
//! (`SCRAM-SHA-256`), the Extended Query protocol (`Parse` + `Bind` +
//! `Describe` + `Execute` + `Sync`) with parameterized `$1..$n` bindings,
//! transactions with automatic `ROLLBACK`, fail-closed connection poisoning on
//! incomplete frames or protocol desync, and clean `Terminate` on drop.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::config::PgConfig;
use crate::scram::ScramClient;
use crate::PgError;

const PROTOCOL_VERSION_3_0: i32 = 196_608;
const MAX_FRAME_LEN: u32 = 64 * 1024 * 1024;
const AUTH_OK: i32 = 0;
const AUTH_CLEARTEXT: i32 = 3;
const AUTH_SASL: i32 = 10;
const AUTH_SASL_CONTINUE: i32 = 11;
const AUTH_SASL_FINAL: i32 = 12;

/// One row decoded from a PostgreSQL `DataRow` message (text format).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PgRow {
    columns: Vec<Option<String>>,
}

impl PgRow {
    /// Read a required non-NULL text column at `idx`.
    pub fn get_str(&self, idx: usize) -> Result<&str, PgError> {
        self.get_opt_str(idx)?
            .ok_or_else(|| PgError::Protocol(format!("unexpected NULL at column {idx}")))
    }

    /// Read an optional nullable text column at `idx`.
    pub fn get_opt_str(&self, idx: usize) -> Result<Option<&str>, PgError> {
        self.columns
            .get(idx)
            .map(|col| col.as_deref())
            .ok_or_else(|| PgError::Protocol(format!("missing column {idx}")))
    }

    /// Read a required `BIGINT` (`i64`) column at `idx`.
    pub fn get_i64(&self, idx: usize) -> Result<i64, PgError> {
        let raw = self.get_str(idx)?;
        raw.parse::<i64>()
            .map_err(|_| PgError::Protocol(format!("column {idx} is not i64: '{raw}'")))
    }

    /// Read an optional nullable `BIGINT` (`Option<i64>`) column at `idx`.
    pub fn get_opt_i64(&self, idx: usize) -> Result<Option<i64>, PgError> {
        match self.get_opt_str(idx)? {
            None => Ok(None),
            Some(raw) => raw
                .parse::<i64>()
                .map(Some)
                .map_err(|_| PgError::Protocol(format!("column {idx} is not i64: '{raw}'"))),
        }
    }
}

/// Result of an Extended Query round-trip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    pub rows: Vec<PgRow>,
    pub rows_affected: u64,
}

type RawQuery = (QueryResult, Option<String>);

enum AuthState {
    Initial,
    CleartextSent,
    SaslStarted(ScramClient),
    SaslContinued(ScramClient),
    SaslVerified,
    Authenticated,
}

struct TxPoisonGuard<'a> {
    client: &'a mut PgClient,
    armed: bool,
}

impl Drop for TxPoisonGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.client.close();
        }
    }
}

/// Synchronous PostgreSQL v3 connection over `std::net::TcpStream`.
pub struct PgClient {
    stream: Option<TcpStream>,
    tx_status: u8,
    in_tx: bool,
    tx_compromised: bool,
}

impl PgClient {
    /// Connect to PostgreSQL using `config`, complete startup & authentication,
    /// and wait for `ReadyForQuery`.
    pub fn connect(config: &PgConfig) -> Result<Self, PgError> {
        let addrs: Vec<SocketAddr> = config.host_port().to_socket_addrs()?.collect();
        let stream = connect_first_reachable(&addrs, &config.host, config.connect_timeout)?;
        stream.set_read_timeout(Some(config.io_timeout))?;
        stream.set_write_timeout(Some(config.io_timeout))?;
        stream.set_nodelay(true)?;
        let mut client = Self {
            stream: Some(stream),
            tx_status: b'I',
            in_tx: false,
            tx_compromised: false,
        };
        if let Err(err) = client.startup(config) {
            client.close();
            return Err(err);
        }
        Ok(client)
    }

    /// Execute a parameterized SQL statement using the Extended Query protocol
    /// (`Parse` + `Bind` + `Describe` + `Execute` + `Sync`), with zero SQL
    /// string interpolation.
    pub fn query(&mut self, sql: &str, params: &[Option<&str>]) -> Result<QueryResult, PgError> {
        self.query_raw(sql, params).map(|(res, _)| res)
    }

    fn query_raw(&mut self, sql: &str, params: &[Option<&str>]) -> Result<RawQuery, PgError> {
        if self.stream.is_none() {
            return Err(PgError::ConnectionClosed);
        }
        let param_count =
            i16::try_from(params.len()).map_err(|_| proto("too many query parameters"))?;
        let mut wire = Vec::with_capacity(sql.len() + 64);
        write_parse(&mut wire, sql)?;
        write_bind(&mut wire, param_count, params)?;
        write_frame(&mut wire, b'D', b"P\0")?;
        write_frame(&mut wire, b'E', b"\0\0\0\0\0")?;
        write_frame(&mut wire, b'S', b"")?;
        match self.send_and_read_query(&wire) {
            Ok(res) => {
                if self.in_tx
                    && (self.tx_status == b'I'
                        || matches!(&res, Ok((_, tag)) if is_tx_ending_tag(tag.as_deref())))
                {
                    self.tx_compromised = true;
                }
                res
            }
            Err(err) => {
                self.close();
                Err(err)
            }
        }
    }

    /// Run `f` inside `BEGIN` / `COMMIT`, rolling back on error and poisoning
    /// the connection on unwind. Refuses nested/open transactions, verifies
    /// `BEGIN`/`COMMIT`/`ROLLBACK` tags and `ReadyForQuery` status, and returns
    /// [`PgError::TransactionAborted`] if the transaction was compromised.
    pub fn in_transaction<T, E>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, E>,
        map_pg: impl Fn(PgError) -> E,
    ) -> Result<T, E> {
        self.begin_transaction().map_err(&map_pg)?;
        let outcome = {
            let mut guard = TxPoisonGuard {
                client: self,
                armed: true,
            };
            let res = f(guard.client);
            guard.armed = false;
            res
        };
        self.in_tx = false;
        let compromised = std::mem::replace(&mut self.tx_compromised, false);
        match outcome {
            Ok(val) => {
                self.commit_transaction(compromised).map_err(&map_pg)?;
                Ok(val)
            }
            Err(err) => {
                self.abort_transaction(compromised).map_err(&map_pg)?;
                Err(err)
            }
        }
    }

    fn begin_transaction(&mut self) -> Result<(), PgError> {
        if self.stream.is_none() {
            return Err(PgError::ConnectionClosed);
        }
        if self.in_tx || self.tx_status != b'I' {
            return Err(PgError::TransactionAborted);
        }
        let (_, tag) = self.query_raw("BEGIN", &[])?;
        if tag.as_deref() == Some("BEGIN") && self.tx_status == b'T' {
            self.in_tx = true;
            return Ok(());
        }
        self.close();
        Err(proto(format!(
            "unexpected BEGIN outcome (tag={tag:?}, status=0x{:02x})",
            self.tx_status
        )))
    }

    fn commit_transaction(&mut self, compromised: bool) -> Result<(), PgError> {
        if compromised || self.tx_status != b'T' {
            if self.tx_status != b'I' {
                self.rollback_transaction();
            }
            return Err(PgError::TransactionAborted);
        }
        match self.query_raw("COMMIT", &[]) {
            Ok((_, tag)) if tag.as_deref() == Some("COMMIT") && self.tx_status == b'I' => Ok(()),
            Ok((_, tag)) if tag.as_deref() == Some("ROLLBACK") && self.tx_status == b'I' => {
                Err(PgError::TransactionAborted)
            }
            Ok((_, tag)) => {
                self.close();
                Err(proto(format!(
                    "unexpected COMMIT outcome (tag={tag:?}, status=0x{:02x})",
                    self.tx_status
                )))
            }
            Err(err) => {
                self.close();
                Err(err)
            }
        }
    }

    fn abort_transaction(&mut self, compromised: bool) -> Result<(), PgError> {
        if self.tx_status != b'I' {
            self.rollback_transaction();
        }
        (!compromised)
            .then_some(())
            .ok_or(PgError::TransactionAborted)
    }

    fn rollback_transaction(&mut self) {
        if !matches!(
            self.query_raw("ROLLBACK", &[]),
            Ok((_, tag)) if tag.as_deref() == Some("ROLLBACK") && self.tx_status == b'I'
        ) {
            self.close();
        }
    }

    fn close(&mut self) {
        self.stream = None;
        self.in_tx = false;
    }

    fn write_all(&mut self, data: &[u8]) -> Result<(), PgError> {
        let stream = self.stream.as_mut().ok_or(PgError::ConnectionClosed)?;
        stream.write_all(data)?;
        Ok(())
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), PgError> {
        let stream = self.stream.as_mut().ok_or(PgError::ConnectionClosed)?;
        stream.read_exact(buf)?;
        Ok(())
    }

    fn startup(&mut self, config: &PgConfig) -> Result<(), PgError> {
        let mut body = Vec::new();
        body.extend_from_slice(&PROTOCOL_VERSION_3_0.to_be_bytes());
        for (k, v) in [
            ("user", config.user.as_str()),
            ("database", config.database.as_str()),
            ("client_encoding", "UTF8"),
            ("application_name", "brokkr-store-pg"),
        ] {
            push_cstr(&mut body, k)?;
            push_cstr(&mut body, v)?;
        }
        body.push(0);
        let len = checked_wire_len(body.len(), 4)?;
        let mut packet = Vec::with_capacity(body.len() + 4);
        packet.extend_from_slice(&len.to_be_bytes());
        packet.extend_from_slice(&body);
        self.write_all(&packet)?;
        self.drive_auth(config)
    }

    fn drive_auth(&mut self, config: &PgConfig) -> Result<(), PgError> {
        let mut state = AuthState::Initial;
        loop {
            let (tag, body) = self.read_frame()?;
            match tag {
                b'R' => state = self.handle_auth_frame(&body, config, state)?,
                b'S' | b'K' | b'N' => {}
                b'E' => return Err(parse_error_response(&body)),
                b'Z' => {
                    let status = parse_ready_for_query(&body)?;
                    if !matches!(state, AuthState::Authenticated) {
                        return Err(PgError::Auth(
                            "server sent ReadyForQuery before completing authentication".into(),
                        ));
                    }
                    if status != b'I' {
                        return Err(proto(format!(
                            "unexpected startup ReadyForQuery status 0x{status:02x}"
                        )));
                    }
                    self.tx_status = status;
                    return Ok(());
                }
                other => {
                    return Err(proto(format!(
                        "unexpected startup message tag 0x{other:02x}"
                    )));
                }
            }
        }
    }

    fn handle_auth_frame(
        &mut self,
        body: &[u8],
        config: &PgConfig,
        state: AuthState,
    ) -> Result<AuthState, PgError> {
        match (read_i32(body, 0)?, state) {
            (AUTH_OK, AuthState::Initial | AuthState::CleartextSent | AuthState::SaslVerified) => {
                Ok(AuthState::Authenticated)
            }
            (AUTH_CLEARTEXT, AuthState::Initial) => self.send_cleartext(config),
            (AUTH_SASL, AuthState::Initial) => self.start_sasl(&body[4..], config),
            (AUTH_SASL_CONTINUE, AuthState::SaslStarted(client)) => {
                self.continue_sasl(&body[4..], config, client)
            }
            (AUTH_SASL_FINAL, AuthState::SaslContinued(client)) => {
                verify_sasl_final(&body[4..], &client)?;
                Ok(AuthState::SaslVerified)
            }
            (AUTH_OK | AUTH_CLEARTEXT | AUTH_SASL | AUTH_SASL_CONTINUE | AUTH_SASL_FINAL, _) => {
                Err(PgError::Auth(
                    "unexpected PostgreSQL authentication state transition".into(),
                ))
            }
            (other, _) => Err(PgError::Auth(format!(
                "unsupported PostgreSQL authentication method {other}"
            ))),
        }
    }

    fn send_cleartext(&mut self, config: &PgConfig) -> Result<AuthState, PgError> {
        let password = require_password(config)?;
        let mut payload = Vec::with_capacity(password.len() + 1);
        push_cstr(&mut payload, password)?;
        self.send_frame(b'p', &payload)?;
        Ok(AuthState::CleartextSent)
    }

    fn continue_sasl(
        &mut self,
        payload: &[u8],
        config: &PgConfig,
        mut client: ScramClient,
    ) -> Result<AuthState, PgError> {
        let password = require_password(config)?;
        let server_first = std::str::from_utf8(payload)
            .map_err(|_| PgError::Auth("non-UTF8 SCRAM server-first".into()))?;
        let final_msg = client.process_server_first(password, server_first)?;
        self.send_frame(b'p', final_msg.as_bytes())?;
        Ok(AuthState::SaslContinued(client))
    }

    fn start_sasl(&mut self, mechanisms: &[u8], config: &PgConfig) -> Result<AuthState, PgError> {
        let _ = require_password(config)?;
        let offers_scram = mechanisms
            .split(|&b| b == 0)
            .any(|mech| mech == b"SCRAM-SHA-256");
        if !offers_scram {
            return Err(PgError::Auth(
                "server did not offer SCRAM-SHA-256 SASL mechanism".into(),
            ));
        }
        let nonce = uuid::Uuid::new_v4().simple().to_string();
        let client = ScramClient::new(nonce);
        let initial = client.client_first_message();
        let init_len = checked_wire_len(initial.len(), 0)?;
        let mut payload = Vec::with_capacity(14 + 4 + initial.len());
        push_cstr(&mut payload, "SCRAM-SHA-256")?;
        payload.extend_from_slice(&init_len.to_be_bytes());
        payload.extend_from_slice(initial.as_bytes());
        self.send_frame(b'p', &payload)?;
        Ok(AuthState::SaslStarted(client))
    }

    fn send_and_read_query(&mut self, wire: &[u8]) -> Result<Result<RawQuery, PgError>, PgError> {
        self.write_all(wire)?;
        let mut state = QueryReadState::new(self.tx_status);
        loop {
            let (tag, body) = self.read_frame()?;
            if tag == b'Z' {
                let (status, outcome) = state.finish_ready(&body)?;
                self.tx_status = status;
                return Ok(outcome);
            }
            state.apply_frame(tag, &body)?;
        }
    }

    fn send_frame(&mut self, tag: u8, payload: &[u8]) -> Result<(), PgError> {
        let mut buf = Vec::with_capacity(payload.len() + 5);
        write_frame(&mut buf, tag, payload)?;
        self.write_all(&buf)
    }

    fn read_frame(&mut self) -> Result<(u8, Vec<u8>), PgError> {
        let mut hdr = [0u8; 5];
        self.read_exact(&mut hdr)?;
        let raw_len = u32::from_be_bytes([hdr[1], hdr[2], hdr[3], hdr[4]]);
        if !(4..=MAX_FRAME_LEN).contains(&raw_len) {
            return Err(PgError::Protocol(format!("invalid frame length {raw_len}")));
        }
        let mut body = vec![0u8; (raw_len - 4) as usize];
        self.read_exact(&mut body)?;
        Ok((hdr[0], body))
    }
}

impl Drop for PgClient {
    fn drop(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.write_all(&[b'X', 0, 0, 0, 4]);
        }
    }
}

enum QueryCompletion {
    Pending,
    Empty,
    Command(String),
    Error(PgError),
    SyncError(PgError),
}

struct QueryReadState {
    initial_tx_status: u8,
    result: QueryResult,
    completion: QueryCompletion,
    expected_cols: Option<usize>,
}

impl QueryReadState {
    fn new(initial_tx_status: u8) -> Self {
        Self {
            initial_tx_status,
            result: QueryResult {
                rows: Vec::new(),
                rows_affected: 0,
            },
            completion: QueryCompletion::Pending,
            expected_cols: None,
        }
    }
    fn ensure_pending(&self) -> Result<(), PgError> {
        matches!(self.completion, QueryCompletion::Pending)
            .then_some(())
            .ok_or_else(|| proto("duplicate query completion before ReadyForQuery"))
    }

    fn apply_empty_query(&mut self, body: &[u8]) -> Result<(), PgError> {
        self.ensure_pending()?;
        if self.expected_cols.is_some() {
            return Err(proto("unexpected EmptyQueryResponse after RowDescription"));
        }
        let n = body.len();
        if n > 0 {
            return Err(proto(format!(
                "non-empty EmptyQueryResponse payload ({n} bytes)"
            )));
        }
        self.completion = QueryCompletion::Empty;
        Ok(())
    }

    fn apply_data_row(&mut self, body: &[u8]) -> Result<(), PgError> {
        if !matches!(self.completion, QueryCompletion::Pending) {
            return Err(proto("DataRow after completion"));
        }
        let row = parse_data_row(body)?;
        let got = row.columns.len();
        match self.expected_cols {
            Some(want) if got == want => {
                self.result.rows.push(row);
                Ok(())
            }
            Some(want) => Err(proto(format!(
                "DataRow column count {got} does not match RowDescription count {want}"
            ))),
            None => Err(proto("received DataRow without RowDescription")),
        }
    }

    fn apply_error(&mut self, body: &[u8]) -> Result<(), PgError> {
        use QueryCompletion as C;
        let err = parse_error_response(body);
        self.completion = match &self.completion {
            C::Pending => C::Error(err),
            C::Command(tag) if can_fail_at_sync(self.initial_tx_status, tag) => C::SyncError(err),
            _ => return Err(proto("duplicate query completion before ReadyForQuery")),
        };
        Ok(())
    }

    fn finish_ready(self, body: &[u8]) -> Result<(u8, Result<RawQuery, PgError>), PgError> {
        use QueryCompletion as C;
        let status = parse_ready_for_query(body)?;
        let tag = match self.completion {
            C::Pending => return Err(proto("premature ReadyForQuery before query completion")),
            C::Error(_) if status == b'T' => {
                return Err(proto(
                    "server reported active transaction ('T') after ErrorResponse",
                ));
            }
            C::SyncError(_) if status != b'I' => {
                return Err(proto(format!(
                    "unexpected ReadyForQuery status 0x{status:02x} after Sync ErrorResponse"
                )));
            }
            C::Error(err) | C::SyncError(err) => return Ok((status, Err(err))),
            C::Command(tag) => Some(tag),
            C::Empty => None,
        };
        Ok((status, Ok((self.result, tag))))
    }

    fn apply_frame(&mut self, tag: u8, body: &[u8]) -> Result<(), PgError> {
        match tag {
            b'1' | b'2' | b't' | b'n' | b'S' | b'N' => Ok(()),
            b'I' => self.apply_empty_query(body),
            b'T' => {
                if !matches!(self.completion, QueryCompletion::Pending)
                    || self.expected_cols.is_some()
                {
                    return Err(proto("unexpected RowDescription frame"));
                }
                self.expected_cols = Some(parse_row_description(body)?);
                Ok(())
            }
            b'D' => self.apply_data_row(body),
            b'C' => {
                self.ensure_pending()?;
                let (cmd_tag, affected) = parse_command_complete(body)?;
                self.completion = QueryCompletion::Command(cmd_tag);
                self.result.rows_affected = affected;
                Ok(())
            }
            b'E' => self.apply_error(body),
            other => Err(proto(format!("unexpected query message tag 0x{other:02x}"))),
        }
    }
}

fn proto(msg: impl Into<String>) -> PgError {
    PgError::Protocol(msg.into())
}

fn is_tx_ending_tag(tag: Option<&str>) -> bool {
    matches!(tag, Some("ROLLBACK" | "COMMIT" | "END" | "ABORT"))
}

fn can_fail_at_sync(initial_tx_status: u8, tag: &str) -> bool {
    (initial_tx_status == b'I' && tag != "BEGIN" && !is_tx_ending_tag(Some(tag)))
        || (initial_tx_status == b'T' && matches!(tag, "COMMIT" | "END"))
}

fn parse_ready_for_query(body: &[u8]) -> Result<u8, PgError> {
    let n = body.len();
    match body {
        [status @ (b'I' | b'T' | b'E')] => Ok(*status),
        _ => Err(proto(format!(
            "malformed ReadyForQuery payload ({n} bytes)"
        ))),
    }
}

fn verify_sasl_final(payload: &[u8], client: &ScramClient) -> Result<(), PgError> {
    let server_final = std::str::from_utf8(payload)
        .map_err(|_| PgError::Auth("non-UTF8 SCRAM server-final".into()))?;
    client.verify_server_final(server_final)
}

fn require_password(config: &PgConfig) -> Result<&str, PgError> {
    config
        .password
        .as_deref()
        .ok_or_else(|| PgError::Auth("server requested password authentication".into()))
}

pub(crate) fn push_cstr(buf: &mut Vec<u8>, text: &str) -> Result<(), PgError> {
    if text.as_bytes().contains(&0) {
        return Err(proto("NUL byte not allowed in PostgreSQL protocol string"));
    }
    buf.extend_from_slice(text.as_bytes());
    buf.push(0);
    Ok(())
}

pub(crate) fn checked_wire_len(len: usize, extra: usize) -> Result<i32, PgError> {
    len.checked_add(extra)
        .and_then(|total| i32::try_from(total).ok())
        .ok_or_else(|| PgError::Protocol("PostgreSQL wire length exceeds i32::MAX".into()))
}

fn write_frame(buf: &mut Vec<u8>, tag: u8, payload: &[u8]) -> Result<(), PgError> {
    let len = checked_wire_len(payload.len(), 4)?;
    buf.push(tag);
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(payload);
    Ok(())
}

fn write_parse(wire: &mut Vec<u8>, sql: &str) -> Result<(), PgError> {
    let mut payload = Vec::with_capacity(sql.len() + 4);
    payload.push(0);
    push_cstr(&mut payload, sql)?;
    payload.extend_from_slice(&0i16.to_be_bytes());
    write_frame(wire, b'P', &payload)
}

pub(crate) fn connect_first_reachable(
    addrs: &[SocketAddr],
    host: &str,
    timeout: Duration,
) -> Result<TcpStream, PgError> {
    let mut last_err = None;
    for addr in addrs {
        match TcpStream::connect_timeout(addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.map_or_else(
        || proto(format!("could not resolve host '{host}'")),
        PgError::Io,
    ))
}

fn write_bind(wire: &mut Vec<u8>, count: i16, params: &[Option<&str>]) -> Result<(), PgError> {
    let mut payload = Vec::with_capacity(16 + params.len() * 8);
    payload.extend_from_slice(&[0, 0, 0, 0]);
    payload.extend_from_slice(&count.to_be_bytes());
    for param in params {
        match param {
            None => payload.extend_from_slice(&(-1i32).to_be_bytes()),
            Some(val) => {
                let len = checked_wire_len(val.len(), 0)?;
                payload.extend_from_slice(&len.to_be_bytes());
                payload.extend_from_slice(val.as_bytes());
            }
        }
    }
    payload.extend_from_slice(&0i16.to_be_bytes());
    write_frame(wire, b'B', &payload)
}

fn read_i16(buf: &[u8], offset: usize) -> Result<i16, PgError> {
    let s = buf
        .get(offset..offset + 2)
        .ok_or_else(|| proto("truncated i16 in server frame"))?;
    Ok(i16::from_be_bytes([s[0], s[1]]))
}

fn read_i32(buf: &[u8], offset: usize) -> Result<i32, PgError> {
    let s = buf
        .get(offset..offset + 4)
        .ok_or_else(|| proto("truncated i32 in server frame"))?;
    Ok(i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn parse_row_description(body: &[u8]) -> Result<usize, PgError> {
    let raw_count = read_i16(body, 0)?;
    let count = usize::try_from(raw_count)
        .map_err(|_| proto(format!("negative RowDescription column count {raw_count}")))?;
    let mut offset = 2usize;
    for _ in 0..count {
        parse_column_descriptor(body, &mut offset)?;
    }
    if offset != body.len() {
        return Err(proto("trailing bytes in RowDescription frame"));
    }
    Ok(count)
}

fn parse_column_descriptor(body: &[u8], offset: &mut usize) -> Result<(), PgError> {
    let nul_rel = body[*offset..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| proto("unterminated RowDescription column name"))?;
    let _name = std::str::from_utf8(&body[*offset..*offset + nul_rel])
        .map_err(|_| proto("non-UTF8 RowDescription column name"))?;
    *offset += nul_rel + 1;
    let desc = body
        .get(*offset..*offset + 18)
        .ok_or_else(|| proto("truncated RowDescription column descriptor"))?;
    let format_code = i16::from_be_bytes([desc[16], desc[17]]);
    if format_code != 0 {
        return Err(proto(format!(
            "unsupported RowDescription format code {format_code}"
        )));
    }
    *offset += 18;
    Ok(())
}

fn parse_data_row(body: &[u8]) -> Result<PgRow, PgError> {
    let raw_count = read_i16(body, 0)?;
    let count = usize::try_from(raw_count)
        .map_err(|_| proto(format!("negative DataRow column count {raw_count}")))?;
    let mut offset = 2usize;
    let mut columns = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_i32(body, offset)?;
        offset += 4;
        if len == -1 {
            columns.push(None);
        } else if len < 0 {
            return Err(proto(format!("invalid DataRow field length {len}")));
        } else {
            let end = offset + len as usize;
            let bytes = body
                .get(offset..end)
                .ok_or_else(|| proto("truncated DataRow field"))?;
            let text =
                String::from_utf8(bytes.to_vec()).map_err(|_| proto("non-UTF8 DataRow field"))?;
            columns.push(Some(text));
            offset = end;
        }
    }
    if offset != body.len() {
        return Err(proto("trailing bytes in DataRow frame"));
    }
    Ok(PgRow { columns })
}

fn parse_command_complete(body: &[u8]) -> Result<(String, u64), PgError> {
    let nul = body
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| proto("unterminated CommandComplete tag"))?;
    if nul + 1 != body.len() {
        return Err(proto("trailing bytes after CommandComplete NUL terminator"));
    }
    let tag =
        std::str::from_utf8(&body[..nul]).map_err(|_| proto("non-UTF8 CommandComplete tag"))?;
    let affected = tag
        .split_whitespace()
        .last()
        .and_then(|w| w.parse().ok())
        .unwrap_or(0);
    Ok((tag.to_string(), affected))
}

fn parse_error_response(body: &[u8]) -> PgError {
    let mut sqlstate = String::new();
    let mut message = String::new();
    let mut constraint = None;
    let mut i = 0;
    while i < body.len() {
        let code = body[i];
        if code == 0 {
            break;
        }
        i += 1;
        let end = body[i..]
            .iter()
            .position(|&b| b == 0)
            .map_or(body.len(), |pos| i + pos);
        let value = String::from_utf8_lossy(&body[i..end]).into_owned();
        match code {
            b'C' => sqlstate = value,
            b'M' => message = value,
            b'n' => constraint = Some(value),
            _ => {}
        }
        i = (end + 1).min(body.len());
    }
    PgError::Server {
        sqlstate,
        message,
        constraint,
    }
}
