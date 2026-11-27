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

/// Synchronous PostgreSQL v3 connection over `std::net::TcpStream`.
pub struct PgClient {
    stream: Option<TcpStream>,
}

impl PgClient {
    /// Connect to PostgreSQL using `config`, complete startup & authentication,
    /// and wait for `ReadyForQuery`.
    pub fn connect(config: &PgConfig) -> Result<Self, PgError> {
        let addr = resolve_first_addr(config.host_port().to_socket_addrs()?, &config.host)?;
        let stream = TcpStream::connect_timeout(&addr, config.connect_timeout)?;
        stream.set_read_timeout(Some(config.io_timeout))?;
        stream.set_write_timeout(Some(config.io_timeout))?;
        stream.set_nodelay(true)?;
        let mut client = Self {
            stream: Some(stream),
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
        if self.stream.is_none() {
            return Err(PgError::ConnectionClosed);
        }
        let param_count = i16::try_from(params.len())
            .map_err(|_| PgError::Protocol("too many query parameters".into()))?;
        let mut wire = Vec::with_capacity(sql.len() + 64);
        write_parse(&mut wire, sql)?;
        write_bind(&mut wire, param_count, params)?;
        write_frame(&mut wire, b'D', b"P\0")?;
        write_frame(&mut wire, b'E', b"\0\0\0\0\0")?;
        write_frame(&mut wire, b'S', b"")?;
        match self.send_and_read_query(&wire) {
            Ok(res) => res,
            Err(err) => {
                self.close();
                Err(err)
            }
        }
    }

    /// Run `f` inside `BEGIN` / `COMMIT`, automatically issuing `ROLLBACK` if
    /// `f` returns an error. If `COMMIT` or `ROLLBACK` fails, immediately
    /// poisons/closes the connection so stale frames can never be consumed.
    pub fn in_transaction<T, E>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, E>,
        map_pg: impl Fn(PgError) -> E,
    ) -> Result<T, E> {
        self.query("BEGIN", &[]).map_err(&map_pg)?;
        match f(self) {
            Ok(val) => {
                if let Err(err) = self.query("COMMIT", &[]) {
                    self.close();
                    return Err(map_pg(err));
                }
                Ok(val)
            }
            Err(err) => {
                if self.query("ROLLBACK", &[]).is_err() {
                    self.close();
                }
                Err(err)
            }
        }
    }

    fn close(&mut self) {
        self.stream = None;
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
        let mut auth_ok = false;
        let mut scram: Option<ScramClient> = None;
        let mut sasl_verified = false;
        loop {
            let (tag, body) = self.read_frame()?;
            match tag {
                b'R' => self.handle_auth_frame(
                    &body,
                    config,
                    &mut auth_ok,
                    &mut scram,
                    &mut sasl_verified,
                )?,
                b'S' | b'K' | b'N' => {}
                b'E' => return Err(parse_error_response(&body)),
                b'Z' => {
                    if !auth_ok || (scram.is_some() && !sasl_verified) {
                        return Err(PgError::Auth(
                            "server sent ReadyForQuery before completing authentication".into(),
                        ));
                    }
                    return Ok(());
                }
                other => {
                    return Err(PgError::Protocol(format!(
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
        auth_ok: &mut bool,
        scram: &mut Option<ScramClient>,
        sasl_verified: &mut bool,
    ) -> Result<(), PgError> {
        let code = read_i32(body, 0)?;
        match code {
            AUTH_OK => *auth_ok = true,
            AUTH_CLEARTEXT => {
                let password = require_password(config)?;
                let mut payload = Vec::with_capacity(password.len() + 1);
                push_cstr(&mut payload, password)?;
                self.send_frame(b'p', &payload)?;
            }
            AUTH_SASL => self.start_sasl(&body[4..], config, scram)?,
            AUTH_SASL_CONTINUE => self.continue_sasl(&body[4..], config, scram)?,
            AUTH_SASL_FINAL => {
                verify_sasl_final(&body[4..], scram)?;
                *sasl_verified = true;
            }
            other => {
                return Err(PgError::Auth(format!(
                    "unsupported PostgreSQL authentication method {other}"
                )));
            }
        }
        Ok(())
    }

    fn continue_sasl(
        &mut self,
        payload: &[u8],
        config: &PgConfig,
        scram: &mut Option<ScramClient>,
    ) -> Result<(), PgError> {
        let password = require_password(config)?;
        let server_first = std::str::from_utf8(payload)
            .map_err(|_| PgError::Auth("non-UTF8 SCRAM server-first".into()))?;
        let client = scram
            .as_mut()
            .ok_or_else(|| PgError::Auth("unexpected SASLContinue".into()))?;
        let final_msg = client.process_server_first(password, server_first)?;
        self.send_frame(b'p', final_msg.as_bytes())
    }

    fn start_sasl(
        &mut self,
        mechanisms: &[u8],
        config: &PgConfig,
        scram: &mut Option<ScramClient>,
    ) -> Result<(), PgError> {
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
        *scram = Some(client);
        Ok(())
    }

    fn send_and_read_query(
        &mut self,
        wire: &[u8],
    ) -> Result<Result<QueryResult, PgError>, PgError> {
        self.write_all(wire)?;
        let mut state = QueryReadState::default();
        loop {
            let (tag, body) = self.read_frame()?;
            if state.apply_frame(tag, &body)? {
                break;
            }
        }
        Ok(match state.server_err {
            Some(err) => Err(err),
            None => Ok(QueryResult {
                rows: state.rows,
                rows_affected: state.rows_affected,
            }),
        })
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

#[derive(Default)]
struct QueryReadState {
    rows: Vec<PgRow>,
    rows_affected: u64,
    server_err: Option<PgError>,
    expected_cols: Option<usize>,
}

impl QueryReadState {
    fn apply_frame(&mut self, tag: u8, body: &[u8]) -> Result<bool, PgError> {
        match tag {
            b'1' | b'2' | b't' | b'n' | b'I' | b'S' | b'N' => Ok(false),
            b'T' => {
                self.expected_cols = Some(parse_row_description(body)?);
                Ok(false)
            }
            b'D' => {
                let row = parse_data_row(body)?;
                match self.expected_cols {
                    Some(want) if row.columns.len() == want => {
                        self.rows.push(row);
                        Ok(false)
                    }
                    Some(want) => Err(PgError::Protocol(format!(
                        "DataRow column count {} does not match RowDescription count {want}",
                        row.columns.len()
                    ))),
                    None => Err(PgError::Protocol(
                        "received DataRow without RowDescription".into(),
                    )),
                }
            }
            b'C' => {
                self.rows_affected = parse_command_complete(body);
                Ok(false)
            }
            b'E' => {
                self.server_err = Some(parse_error_response(body));
                Ok(false)
            }
            b'Z' => Ok(true),
            other => Err(PgError::Protocol(format!(
                "unexpected query message tag 0x{other:02x}"
            ))),
        }
    }
}

fn verify_sasl_final(payload: &[u8], scram: &Option<ScramClient>) -> Result<(), PgError> {
    let server_final = std::str::from_utf8(payload)
        .map_err(|_| PgError::Auth("non-UTF8 SCRAM server-final".into()))?;
    let client = scram
        .as_ref()
        .ok_or_else(|| PgError::Auth("unexpected SASLFinal".into()))?;
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
        return Err(PgError::Protocol(
            "NUL byte not allowed in PostgreSQL protocol string".into(),
        ));
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

pub(crate) fn resolve_first_addr(
    mut addrs: impl Iterator<Item = SocketAddr>,
    host: &str,
) -> Result<SocketAddr, PgError> {
    addrs
        .next()
        .ok_or_else(|| PgError::Protocol(format!("could not resolve host '{host}'")))
}

fn write_bind(wire: &mut Vec<u8>, count: i16, params: &[Option<&str>]) -> Result<(), PgError> {
    let mut payload = Vec::with_capacity(16 + params.len() * 8);
    payload.push(0);
    payload.push(0);
    payload.extend_from_slice(&0i16.to_be_bytes());
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
    let slice = buf
        .get(offset..offset + 2)
        .ok_or_else(|| PgError::Protocol("truncated i16 in server frame".into()))?;
    Ok(i16::from_be_bytes([slice[0], slice[1]]))
}

fn read_i32(buf: &[u8], offset: usize) -> Result<i32, PgError> {
    let slice = buf
        .get(offset..offset + 4)
        .ok_or_else(|| PgError::Protocol("truncated i32 in server frame".into()))?;
    Ok(i32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn parse_row_description(body: &[u8]) -> Result<usize, PgError> {
    let raw_count = read_i16(body, 0)?;
    let count = usize::try_from(raw_count).map_err(|_| {
        PgError::Protocol(format!("negative RowDescription column count {raw_count}"))
    })?;
    let mut offset = 2usize;
    for _ in 0..count {
        parse_column_descriptor(body, &mut offset)?;
    }
    if offset != body.len() {
        return Err(PgError::Protocol(
            "trailing bytes in RowDescription frame".into(),
        ));
    }
    Ok(count)
}

fn parse_column_descriptor(body: &[u8], offset: &mut usize) -> Result<(), PgError> {
    let nul_rel = body[*offset..]
        .iter()
        .position(|&b| b == 0)
        .ok_or_else(|| PgError::Protocol("unterminated RowDescription column name".into()))?;
    let _name = std::str::from_utf8(&body[*offset..*offset + nul_rel])
        .map_err(|_| PgError::Protocol("non-UTF8 RowDescription column name".into()))?;
    *offset += nul_rel + 1;
    let desc = body
        .get(*offset..*offset + 18)
        .ok_or_else(|| PgError::Protocol("truncated RowDescription column descriptor".into()))?;
    let _table_oid = i32::from_be_bytes([desc[0], desc[1], desc[2], desc[3]]);
    let _col_attr = i16::from_be_bytes([desc[4], desc[5]]);
    let _type_oid = i32::from_be_bytes([desc[6], desc[7], desc[8], desc[9]]);
    let _type_size = i16::from_be_bytes([desc[10], desc[11]]);
    let _type_mod = i32::from_be_bytes([desc[12], desc[13], desc[14], desc[15]]);
    let format_code = i16::from_be_bytes([desc[16], desc[17]]);
    if format_code != 0 {
        return Err(PgError::Protocol(format!(
            "unsupported RowDescription format code {format_code}"
        )));
    }
    *offset += 18;
    Ok(())
}

fn parse_data_row(body: &[u8]) -> Result<PgRow, PgError> {
    let raw_count = read_i16(body, 0)?;
    let count = usize::try_from(raw_count)
        .map_err(|_| PgError::Protocol(format!("negative DataRow column count {raw_count}")))?;
    let mut offset = 2usize;
    let mut columns = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_i32(body, offset)?;
        offset += 4;
        if len == -1 {
            columns.push(None);
        } else if len < 0 {
            return Err(PgError::Protocol(format!(
                "invalid DataRow field length {len}"
            )));
        } else {
            let end = offset + len as usize;
            let bytes = body
                .get(offset..end)
                .ok_or_else(|| PgError::Protocol("truncated DataRow field".into()))?;
            let text = String::from_utf8(bytes.to_vec())
                .map_err(|_| PgError::Protocol("non-UTF8 DataRow field".into()))?;
            columns.push(Some(text));
            offset = end;
        }
    }
    if offset != body.len() {
        return Err(PgError::Protocol("trailing bytes in DataRow frame".into()));
    }
    Ok(PgRow { columns })
}

fn parse_command_complete(body: &[u8]) -> u64 {
    let end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
    let tag = std::str::from_utf8(&body[..end]).unwrap_or("");
    tag.split_whitespace()
        .last()
        .and_then(|word| word.parse::<u64>().ok())
        .unwrap_or(0)
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
