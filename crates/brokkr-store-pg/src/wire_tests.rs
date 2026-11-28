//! Unit tests for `config`, `scram`, `error`, and `wire` over an in-process
//! mock PostgreSQL v3 TCP wire server (`127.0.0.1:0`).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::config::{PgConfig, DEFAULT_CONNECT_TIMEOUT, DEFAULT_IO_TIMEOUT, DEFAULT_PORT};
use crate::error::PgError;
use crate::schema::{decode_pg_text, encode_pg_text};
use crate::scram::{
    base64_decode, base64_encode, constant_time_eq, hmac_sha256, pbkdf2_hmac_sha256,
    saslprep_password, ScramClient,
};
use crate::wire::{checked_wire_len, connect_first_reachable, push_cstr, PgClient};

pub(crate) fn send_frame(stream: &mut TcpStream, tag: u8, payload: &[u8]) {
    let mut buf = Vec::with_capacity(payload.len() + 5);
    buf.push(tag);
    buf.extend_from_slice(&((payload.len() + 4) as i32).to_be_bytes());
    buf.extend_from_slice(payload);
    stream.write_all(&buf).expect("write server frame");
}

pub(crate) fn read_startup(stream: &mut TcpStream) -> Vec<u8> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).expect("read startup len");
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut body = vec![0u8; len - 4];
    stream.read_exact(&mut body).expect("read startup body");
    body
}

pub(crate) fn read_client_frame(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let mut hdr = [0u8; 5];
    if stream.read_exact(&mut hdr).is_err() {
        return None;
    }
    let len = u32::from_be_bytes([hdr[1], hdr[2], hdr[3], hdr[4]]) as usize;
    let mut body = vec![0u8; len - 4];
    stream
        .read_exact(&mut body)
        .expect("read client frame body");
    Some((hdr[0], body))
}

pub(crate) fn error_payload(sqlstate: &str, msg: &str, constraint: Option<&str>) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(b'S');
    out.extend_from_slice(b"ERROR\0");
    out.push(b'C');
    out.extend_from_slice(sqlstate.as_bytes());
    out.push(0);
    out.push(b'M');
    out.extend_from_slice(msg.as_bytes());
    out.push(0);
    if let Some(c) = constraint {
        out.push(b'n');
        out.extend_from_slice(c.as_bytes());
        out.push(0);
    }
    out.push(0);
    out
}

pub(crate) fn data_row_payload(cols: &[Option<&str>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(cols.len() as i16).to_be_bytes());
    for col in cols {
        match col {
            None => out.extend_from_slice(&(-1i32).to_be_bytes()),
            Some(s) => {
                out.extend_from_slice(&(s.len() as i32).to_be_bytes());
                out.extend_from_slice(s.as_bytes());
            }
        }
    }
    out
}

pub(crate) fn row_description_payload(col_count: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(col_count as i16).to_be_bytes());
    for i in 0..col_count {
        out.extend_from_slice(format!("c{i}").as_bytes());
        out.push(0);
        out.extend_from_slice(&0i32.to_be_bytes());
        out.extend_from_slice(&0i16.to_be_bytes());
        out.extend_from_slice(&25i32.to_be_bytes());
        out.extend_from_slice(&(-1i16).to_be_bytes());
        out.extend_from_slice(&(-1i32).to_be_bytes());
        out.extend_from_slice(&0i16.to_be_bytes());
    }
    out
}

fn complete_trust_startup(s: &mut TcpStream) {
    let _ = read_startup(s);
    send_frame(s, b'R', &0i32.to_be_bytes());
    send_frame(s, b'Z', b"I");
}

fn read_extended_query(s: &mut TcpStream) -> String {
    let (tag, body) = read_client_frame(s).expect("read Parse frame");
    assert_eq!(tag, b'P');
    let stmt_end = body.iter().position(|&b| b == 0).expect("stmt NUL");
    let rest = &body[stmt_end + 1..];
    let sql_end = rest.iter().position(|&b| b == 0).expect("sql NUL");
    let sql = std::str::from_utf8(&rest[..sql_end])
        .expect("utf8 sql")
        .to_string();
    for _ in 0..4 {
        let _ = read_client_frame(s);
    }
    sql
}

fn send_scram_offer(s: &mut TcpStream) {
    let mut sasl = Vec::new();
    sasl.extend_from_slice(&10i32.to_be_bytes());
    sasl.extend_from_slice(b"SCRAM-SHA-256\0\0");
    send_frame(s, b'R', &sasl);
}

fn send_begin_ok(s: &mut TcpStream) {
    assert_eq!(read_extended_query(s), "BEGIN");
    send_frame(s, b'1', b"");
    send_frame(s, b'2', b"");
    send_frame(s, b'n', b"");
    send_frame(s, b'C', b"BEGIN\0");
    send_frame(s, b'Z', b"T");
}

fn send_tx_error_and_rollback(
    s: &mut TcpStream,
    sqlstate: &str,
    msg: &str,
    constraint: Option<&str>,
) {
    send_begin_ok(s);
    let _ = read_extended_query(s);
    send_frame(s, b'E', &error_payload(sqlstate, msg, constraint));
    send_frame(s, b'Z', b"E");
    assert_eq!(read_extended_query(s), "ROLLBACK");
    send_frame(s, b'C', b"ROLLBACK\0");
    send_frame(s, b'Z', b"I");
    let _ = read_client_frame(s);
}

fn expect_query_reply(s: &mut TcpStream, sql: &str, tag: &[u8], status: &[u8]) {
    assert_eq!(read_extended_query(s), sql);
    send_frame(s, b'C', tag);
    send_frame(s, b'Z', status);
}

fn assert_nested_tx_refused(tx: &mut PgClient) {
    assert!(matches!(
        tx.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
        Err(PgError::TransactionAborted)
    ));
}

fn assert_tx_query_aborted(client: &mut PgClient, sql: &str) {
    let err = client
        .in_transaction(|tx| tx.query(sql, &[]).map(|_| ()), |e| e)
        .unwrap_err();
    assert!(matches!(err, PgError::TransactionAborted));
}

fn spawn_tx_single_reply(
    send_begin: bool,
    tag: &'static [u8],
    status: &'static [u8],
) -> (PgClient, thread::JoinHandle<()>) {
    let (port, handle) = spawn_one_shot(move |mut s| {
        complete_trust_startup(&mut s);
        if send_begin {
            send_begin_ok(&mut s);
        }
        read_extended_query(&mut s);
        send_frame(&mut s, b'C', tag);
        send_frame(&mut s, b'Z', status);
        let _ = read_client_frame(&mut s);
    });
    let cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{port}/db")).unwrap();
    (PgClient::connect(&cfg).expect("connect"), handle)
}

fn assert_query_protocol_error_poisons(frames: Vec<(u8, Vec<u8>)>) -> String {
    let (p, h) = spawn_one_shot(move |mut s| {
        complete_trust_startup(&mut s);
        read_extended_query(&mut s);
        let mut wire = Vec::new();
        for (tag, body) in frames {
            wire.push(tag);
            wire.extend_from_slice(&((body.len() + 4) as i32).to_be_bytes());
            wire.extend_from_slice(&body);
        }
        s.write_all(&wire).expect("write server frames");
        let _ = read_client_frame(&mut s);
    });
    let cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p}/db"))
        .unwrap()
        .with_io_timeout(Duration::from_millis(200));
    let mut c = PgClient::connect(&cfg).expect("trust connect");
    let msg = match c.query("SELECT 1", &[]) {
        Err(PgError::Protocol(msg)) => msg,
        other => panic!("expected PgError::Protocol, got {other:?}"),
    };
    assert!(matches!(
        c.query("SELECT 1", &[]),
        Err(PgError::ConnectionClosed)
    ));
    drop(c);
    h.join().unwrap();
    msg
}

fn spawn_one_shot(
    handler: impl FnOnce(TcpStream) + Send + 'static,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept client");
        handler(stream);
    });
    (port, handle)
}

#[test]
fn config_parses_valid_urls_and_refuses_malformed_ones() {
    let cfg = PgConfig::parse(
        "postgres://alice:p%40ss%20word@127.0.0.1:5433/brokkr_db?connect_timeout=5&io_timeout=15",
    )
    .expect("valid url");
    assert_eq!(cfg.user, "alice");
    assert_eq!(cfg.password.as_deref(), Some("p@ss word"));
    assert_eq!(cfg.host, "127.0.0.1");
    assert_eq!(cfg.port, 5433);
    assert_eq!(cfg.database, "brokkr_db");
    assert_eq!(cfg.connect_timeout, Duration::from_secs(5));
    assert_eq!(cfg.io_timeout, Duration::from_secs(15));

    let v6 = PgConfig::parse("postgresql://bob@[::1]/db")
        .expect("ipv6 default port")
        .with_connect_timeout(Duration::from_secs(3))
        .with_io_timeout(Duration::from_secs(4));
    assert_eq!(v6.host, "::1");
    assert_eq!(v6.port, DEFAULT_PORT);
    assert_eq!(v6.password, None);
    assert_eq!(v6.connect_timeout, Duration::from_secs(3));
    assert_eq!(v6.io_timeout, Duration::from_secs(4));

    let v6_port = PgConfig::parse("postgres://bob@[::1]:5444/db?").expect("ipv6 with port");
    assert_eq!(v6_port.port, 5444);
    assert_eq!(v6_port.connect_timeout, DEFAULT_CONNECT_TIMEOUT);
    assert_eq!(v6_port.io_timeout, DEFAULT_IO_TIMEOUT);

    let bad_utf8 = PgConfig::parse("postgres://u:a%AF@localhost/mydb").unwrap_err();
    assert!(matches!(bad_utf8, PgError::InvalidUrl(_)));
    let hex_cases = PgConfig::parse("postgres://u:p%3A%3a%23@localhost/mydb").expect("hex cases");
    assert_eq!(hex_cases.password.as_deref(), Some("p::#"));

    for bad in [
        "mysql://u@host/db",
        "postgres://u@host",
        "postgres://u@host/",
        "postgres://u@host/a/b",
        "postgres://host/db",
        "postgres://:pass@host/db",
        "postgres://u@:5432/db",
        "postgres://u@host:0/db",
        "postgres://u@host:abc/db",
        "postgres://u@[::1/db",
        "postgres://u@[]/db",
        "postgres://u@[::1]extra/db",
        "postgres://u@host/db?badparam",
        "postgres://u@host/db?connect_timeout=abc",
        "postgres://u@host/db?connect_timeout=0",
        "postgres://u@[::1]:abc/db",
        "postgres://u@host/db?sslmode=disable",
        "postgres://u@host/db?sslmode=5",
        "postgres://u%00@host/db",
        "postgres://u%ZZ@host/db",
        "postgres://u%2@host/db",
        "postgres://u@local\0host/db",
        "postgres://u@local%00host/db",
        "postgres://u@[localhost]/db",
        "postgres://u@localhost/db#tail",
        "postgres://u@localhost/db?io_timeout=5#frag",
        "postgres://u@::1:5432/db",
        "postgres://u@fe80::1/db",
        "postgres://u@::1/db",
        "postgres://u@%3A%3A1/db",
    ] {
        let err = PgConfig::parse(bad).unwrap_err();
        assert!(
            matches!(err, PgError::InvalidUrl(_)),
            "bad url {bad}: {err}"
        );
        assert_eq!(err.sqlstate(), None);
        let _ = err.into_store_error();
    }
}

#[test]
fn scram_matches_rfc_7677_test_vector_and_rejects_bad_inputs() {
    let mut client = ScramClient::with_user("user", "rOprNGfwEbeRWgbNEkqO".to_string());
    assert_eq!(
        client.client_first_message(),
        "n,,n=user,r=rOprNGfwEbeRWgbNEkqO"
    );
    let server_first =
        "r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096";
    let client_final = client
        .process_server_first("pencil", server_first)
        .expect("RFC 7677 server-first");
    assert_eq!(
        client_final,
        "c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ="
    );
    client
        .verify_server_final("v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=")
        .expect("RFC 7677 server-final");

    assert_eq!(hmac_sha256(&[0x42u8; 80], b"payload").len(), 32);
    for raw in [&b"a"[..], &b"ab"[..], &b"abc"[..], &b"abcd"[..]] {
        let enc = base64_encode(raw);
        assert_eq!(base64_decode(&enc).expect("roundtrip"), raw);
    }
    assert!(!constant_time_eq(b"abc", b"ab"));
    assert!(!constant_time_eq(b"abc", b"bad"));

    for bad_b64 in [
        "", "abc", "====", "a===", "ab=c", "ab==aaaa", "abc=aaaa", "ab$=",
    ] {
        assert!(matches!(base64_decode(bad_b64), Err(PgError::Auth(_))));
    }

    let mut c2 = ScramClient::new("nonce1".to_string());
    assert!(matches!(
        c2.verify_server_final("v=AAAA"),
        Err(PgError::Auth(_))
    ));
    for bad_sf in [
        "s=AAAA,i=1",
        "r=wrong,s=AAAA,i=1",
        "r=nonce1,s=AAAA,i=1",
        "r=nonce1ext,i=1",
        "r=nonce1ext,s=AAAA",
        "r=nonce1ext,s=AAAA,i=bad",
        "r=nonce1ext,s=AAAA,i=0",
        "r=nonce1ext,s=!!!,i=1",
        "m=mandatory,r=nonce1ext,s=AAAA,i=1",
        "r=nonce1ext,s=AAAA,i=4294967295",
    ] {
        assert!(matches!(
            c2.process_server_first("pw", bad_sf),
            Err(PgError::Auth(_))
        ));
    }
    let _ = c2
        .process_server_first("pw", "r=nonce1ext,s=AAAA,i=1,x=ignored")
        .expect("valid sf");
    assert!(matches!(
        c2.verify_server_final("e=invalid-proof"),
        Err(PgError::Auth(_))
    ));
    assert!(matches!(
        c2.verify_server_final("x=1"),
        Err(PgError::Auth(_))
    ));
    assert!(matches!(
        c2.verify_server_final("v=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="),
        Err(PgError::Auth(_))
    ));

    let escaped = ScramClient::with_user("a=b,c", "n1".into());
    assert_eq!(escaped.client_first_message(), "n,,n=a=3Db=2Cc,r=n1");
    assert_eq!(saslprep_password("pencil").unwrap(), "pencil");
    assert_eq!(saslprep_password("p@ss word!").unwrap(), "p@ss word!");
    for bad_pw in [
        "pass\u{00E9}",
        "cafe\u{0301}",
        "p\u{00AD}w",
        "p\u{00A0}w",
        "\u{00B2}",
        "\u{0627}1",
        "a\x07b",
        "a\x7fb",
    ] {
        assert!(matches!(saslprep_password(bad_pw), Err(PgError::Auth(_))));
        assert!(matches!(
            c2.process_server_first(bad_pw, "r=nonce1ext,s=AAAA,i=1"),
            Err(PgError::Auth(_))
        ));
    }
}

#[test]
fn wire_scram_sha_256_handshake_and_query_round_trip() {
    let (port, handle) = spawn_one_shot(|mut s| {
        let startup = read_startup(&mut s);
        assert!(startup.windows(15).any(|w| w == b"brokkr-store-pg"));
        send_scram_offer(&mut s);

        let (tag, body) = read_client_frame(&mut s).unwrap();
        assert_eq!(tag, b'p');
        let mech_end = body.iter().position(|&b| b == 0).unwrap();
        assert_eq!(&body[..mech_end], b"SCRAM-SHA-256");
        let init_str = std::str::from_utf8(&body[mech_end + 5..]).unwrap();
        let client_bare = init_str.strip_prefix("n,,").unwrap();
        let client_nonce = client_bare.strip_prefix("n=,r=").unwrap();

        let salt = b"0123456789abcdef";
        let server_first = format!("r={client_nonce}srvnonce,s={},i=2", base64_encode(salt));
        let mut cont = Vec::new();
        cont.extend_from_slice(&11i32.to_be_bytes());
        cont.extend_from_slice(server_first.as_bytes());
        send_frame(&mut s, b'R', &cont);

        let (tag2, body2) = read_client_frame(&mut s).unwrap();
        assert_eq!(tag2, b'p');
        let client_final = std::str::from_utf8(&body2).unwrap();
        let (final_no_proof, proof_part) = client_final.rsplit_once(",p=").unwrap();

        let salted = pbkdf2_hmac_sha256(b"secret", salt, 2);
        let client_key = hmac_sha256(&salted, b"Client Key");
        let stored_key: [u8; 32] = Sha256::digest(client_key).into();
        let auth_msg = format!("{client_bare},{server_first},{final_no_proof}");
        let client_sig = hmac_sha256(&stored_key, auth_msg.as_bytes());
        let mut expected_proof = [0u8; 32];
        for i in 0..32 {
            expected_proof[i] = client_key[i] ^ client_sig[i];
        }
        assert_eq!(proof_part, base64_encode(&expected_proof));

        let server_key = hmac_sha256(&salted, b"Server Key");
        let server_sig = hmac_sha256(&server_key, auth_msg.as_bytes());
        let mut fin = Vec::new();
        fin.extend_from_slice(&12i32.to_be_bytes());
        fin.extend_from_slice(format!("v={}", base64_encode(&server_sig)).as_bytes());
        send_frame(&mut s, b'R', &fin);
        send_frame(&mut s, b'R', &0i32.to_be_bytes());
        send_frame(&mut s, b'S', b"server_version\x0018.4\x00");
        send_frame(&mut s, b'K', &[0; 8]);
        send_frame(&mut s, b'N', b"SNOTICE\0");
        send_frame(&mut s, b'Z', b"I");

        for expected_tag in *b"PBDES" {
            assert_eq!(read_client_frame(&mut s).unwrap().0, expected_tag);
        }
        send_frame(&mut s, b'1', b"");
        send_frame(&mut s, b'2', b"");
        send_frame(&mut s, b'T', &row_description_payload(3));
        send_frame(
            &mut s,
            b'D',
            &data_row_payload(&[Some("42"), None, Some("hello")]),
        );
        send_frame(&mut s, b'C', b"SELECT 1\0");
        send_frame(&mut s, b'Z', b"I");

        assert_eq!(read_client_frame(&mut s).unwrap().0, b'X');
    });

    let cfg = PgConfig::parse(&format!("postgres://u:secret@127.0.0.1:{port}/db")).unwrap();
    let mut client = PgClient::connect(&cfg).expect("scram connect");
    let res = client
        .query("SELECT $1, $2", &[Some("x"), None])
        .expect("query ok");
    assert_eq!(res.rows_affected, 1);
    assert_eq!(res.rows[0].get_i64(0).unwrap(), 42);
    assert_eq!(res.rows[0].get_opt_i64(0).unwrap(), Some(42));
    assert_eq!(res.rows[0].get_opt_i64(1).unwrap(), None);
    assert_eq!(res.rows[0].get_opt_str(1).unwrap(), None);
    assert_eq!(res.rows[0].get_str(2).unwrap(), "hello");
    assert!(matches!(res.rows[0].get_str(1), Err(PgError::Protocol(_))));
    assert!(matches!(
        res.rows[0].get_opt_str(9),
        Err(PgError::Protocol(_))
    ));
    assert!(matches!(res.rows[0].get_i64(2), Err(PgError::Protocol(_))));
    assert!(matches!(
        res.rows[0].get_opt_i64(2),
        Err(PgError::Protocol(_))
    ));
    drop(client);
    handle.join().unwrap();
}

#[test]
fn wire_cleartext_auth_transaction_rollback_and_server_errors() {
    let (port, handle) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_frame(&mut s, b'R', &3i32.to_be_bytes());
        let (tag, body) = read_client_frame(&mut s).unwrap();
        assert_eq!(tag, b'p');
        assert_eq!(body, b"pw123\0");
        send_frame(&mut s, b'R', &0i32.to_be_bytes());
        send_frame(&mut s, b'Z', b"I");

        send_tx_error_and_rollback(&mut s, "23505", "dup key", Some("brokkr_runs_pkey"));
    });

    let cfg = PgConfig::parse(&format!("postgres://u:pw123@127.0.0.1:{port}/db")).unwrap();
    let mut client = PgClient::connect(&cfg).expect("cleartext connect");
    let err = client
        .in_transaction(
            |tx| tx.query("INSERT INTO t VALUES ($1)", &[Some("a")]),
            |e| e,
        )
        .unwrap_err();
    assert_eq!(err.sqlstate(), Some("23505"));
    assert!(matches!(
        &err,
        PgError::Server {
            constraint: Some(c),
            ..
        } if c == "brokkr_runs_pkey"
    ));
    let _ = err.into_store_error();
    drop(client);
    handle.join().unwrap();
}

#[test]
fn wire_refuses_protocol_and_auth_anomalies() {
    type AuthCase<'a> = (Option<&'a str>, Vec<(u8, Vec<u8>)>, &'a str);
    let cases: Vec<AuthCase<'_>> = vec![
        (None, vec![(b'R', 5i32.to_be_bytes().to_vec())], "auth"),
        (None, vec![(b'Z', b"I".to_vec())], "auth"),
        (
            None,
            vec![(b'E', error_payload("28P01", "bad pw", None))],
            "server",
        ),
        (None, vec![(b'Q', vec![])], "proto"),
        (None, vec![(b'R', 3i32.to_be_bytes().to_vec())], "auth"),
        (
            Some("pw"),
            vec![(b'R', [&10i32.to_be_bytes()[..], b"PLAIN\0\0"].concat())],
            "auth",
        ),
        (
            Some("pw"),
            vec![(b'R', [&11i32.to_be_bytes()[..], b"r=x,s=AAAA,i=1"].concat())],
            "auth",
        ),
        (
            Some("pw"),
            vec![(b'R', [&11i32.to_be_bytes()[..], &[0xff]].concat())],
            "auth",
        ),
        (
            Some("pw"),
            vec![(b'R', [&12i32.to_be_bytes()[..], b"v=AAAA"].concat())],
            "auth",
        ),
        (
            Some("pw"),
            vec![(b'R', [&12i32.to_be_bytes()[..], &[0xff]].concat())],
            "auth",
        ),
        (None, vec![(b'R', vec![0, 0])], "proto"),
    ];
    for (pw, frames, kind) in cases {
        let (p, h) = spawn_one_shot(move |mut s| {
            let _ = read_startup(&mut s);
            for (tag, body) in frames {
                send_frame(&mut s, tag, &body);
            }
        });
        let url = match pw {
            Some(pass) => format!("postgres://u:{pass}@127.0.0.1:{p}/db"),
            None => format!("postgres://u@127.0.0.1:{p}/db"),
        };
        let cfg = PgConfig::parse(&url).unwrap();
        let err = PgClient::connect(&cfg).err().expect("connect error");
        match kind {
            "auth" => assert!(matches!(err, PgError::Auth(_)), "got {err}"),
            "server" => assert!(matches!(err, PgError::Server { .. }), "got {err}"),
            _ => assert!(matches!(err, PgError::Protocol(_)), "got {err}"),
        }
        h.join().unwrap();
    }

    let (p_len, h_len) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        let _ = s.write_all(&[b'R', 0, 0, 0, 2]);
    });
    let cfg_len = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_len}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_len),
        Err(PgError::Protocol(_))
    ));
    h_len.join().unwrap();

    verify_query_protocol_anomalies_and_io_error();
}

fn verify_query_protocol_anomalies_and_io_error() {
    for bad_frame in [
        (b'?', vec![]),
        (b'D', vec![0]),
        (b'D', (-1i16).to_be_bytes().to_vec()),
        (
            b'D',
            [&1i16.to_be_bytes()[..], &(-2i32).to_be_bytes()[..]].concat(),
        ),
        (
            b'D',
            [&1i16.to_be_bytes()[..], &10i32.to_be_bytes()[..], b"ab"].concat(),
        ),
        (
            b'D',
            [&1i16.to_be_bytes()[..], &1i32.to_be_bytes()[..], &[0xff]].concat(),
        ),
    ] {
        assert_query_protocol_error_poisons(vec![bad_frame]);
    }

    let unused = TcpListener::bind("127.0.0.1:0").unwrap();
    let unused_port = unused.local_addr().unwrap().port();
    drop(unused);
    let bad_cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{unused_port}/db"))
        .unwrap()
        .with_connect_timeout(Duration::from_millis(200));
    let io_err = PgClient::connect(&bad_cfg).err().expect("io error");
    assert!(matches!(io_err, PgError::Io(_)));
    let _ = io_err.into_store_error();
}

#[test]
fn wire_refuses_incomplete_sasl_unterminated_error_and_param_overflow() {
    let (p_skip, h_skip) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_scram_offer(&mut s);
        let _ = read_client_frame(&mut s);
        send_frame(&mut s, b'R', &0i32.to_be_bytes());
        send_frame(&mut s, b'Z', b"I");
    });
    let cfg_skip = PgConfig::parse(&format!("postgres://u:pw@127.0.0.1:{p_skip}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_skip),
        Err(PgError::Auth(_))
    ));
    h_skip.join().unwrap();

    let (p_eof, h_eof) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_frame(&mut s, b'E', b"C28000\x00Mno-nul");
    });
    let cfg_eof = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_eof}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_eof),
        Err(PgError::Server { .. })
    ));
    h_eof.join().unwrap();

    assert!(matches!(
        connect_first_reachable(&[], "empty.invalid", Duration::from_millis(50)),
        Err(PgError::Protocol(_))
    ));

    let (p_many, h_many) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        let _ = read_client_frame(&mut s);
    });
    let cfg_many = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_many}/db")).unwrap();
    let mut c_many = PgClient::connect(&cfg_many).expect("trust connect");
    let many_params = vec![None; usize::from(i16::MAX as u16) + 1];
    assert!(matches!(
        c_many.query("SELECT 1", &many_params),
        Err(PgError::Protocol(_))
    ));
    drop(c_many);
    h_many.join().unwrap();
}

#[test]
fn wire_poisons_connection_on_timeout_stale_reply_and_tx_failures() {
    // 1. Timed-out query leaves delayed reply in TCP buffer; next query MUST fail
    // closed with ConnectionClosed instead of consuming the stale reply!
    let (p_to, h_to) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        read_extended_query(&mut s);
        thread::sleep(Duration::from_millis(120));
        let _ = s.write_all(&[b'T', 0, 0, 0, 6, 0, 0]);
        let _ = s.write_all(&[b'Z', 0, 0, 0, 5, b'I']);
    });
    let cfg_to = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_to}/db"))
        .unwrap()
        .with_io_timeout(Duration::from_millis(40));
    let mut client = PgClient::connect(&cfg_to).expect("connect");
    assert!(matches!(client.query("SELECT 1", &[]), Err(PgError::Io(_))));
    thread::sleep(Duration::from_millis(120));
    let closed = client.query("SELECT 2", &[]).unwrap_err();
    assert!(matches!(closed, PgError::ConnectionClosed));
    assert_eq!(closed.sqlstate(), None);
    let _ = closed.into_store_error();
    h_to.join().unwrap();

    // 2 & 3. Failed COMMIT and failed ROLLBACK both poison the connection.
    for (commit_ok, code, msg) in [
        (true, "40001", "commit failed"),
        (false, "57P01", "rollback failed"),
    ] {
        let (p_tx, h_tx) = spawn_one_shot(move |mut s| {
            complete_trust_startup(&mut s);
            send_begin_ok(&mut s);
            read_extended_query(&mut s);
            send_frame(&mut s, b'E', &error_payload(code, msg, None));
            send_frame(&mut s, b'Z', b"I");
        });
        let cfg_tx = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_tx}/db")).unwrap();
        let mut ctx = PgClient::connect(&cfg_tx).expect("connect tx test");
        if commit_ok {
            assert!(matches!(
                ctx.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
                Err(PgError::Server { .. })
            ));
        } else {
            assert!(matches!(
                ctx.in_transaction(
                    |_| Err::<(), PgError>(PgError::Protocol("boom".into())),
                    |e| e
                ),
                Err(PgError::Protocol(_))
            ));
        }
        assert!(matches!(
            ctx.query("SELECT 1", &[]),
            Err(PgError::ConnectionClosed)
        ));
        h_tx.join().unwrap();
    }
}

#[test]
fn wire_refuses_nul_cstrs_length_overflow_and_bad_row_descriptions() {
    let mut buf = Vec::new();
    assert!(matches!(
        push_cstr(&mut buf, "bad\0str"),
        Err(PgError::Protocol(_))
    ));
    assert!(matches!(
        checked_wire_len(usize::MAX, 4),
        Err(PgError::Protocol(_))
    ));
    assert!(matches!(
        checked_wire_len(i32::MAX as usize, 4),
        Err(PgError::Protocol(_))
    ));
    assert_eq!(checked_wire_len(10, 4).unwrap(), 14);
    assert_eq!(PgError::EmptyWorkerId.sqlstate(), None);

    // Malformed RowDescription / DataRow mismatch cases.
    let mut bad_fmt = row_description_payload(1);
    *bad_fmt.last_mut().unwrap() = 1; // binary format_code != 0
    let mut trailing_rd = row_description_payload(1);
    trailing_rd.push(0x99);
    let mut trailing_dr = data_row_payload(&[Some("a")]);
    trailing_dr.push(0x99);

    let sequences: Vec<Vec<(u8, Vec<u8>)>> = vec![
        vec![(b'T', vec![0])],                                        // truncated count
        vec![(b'T', (-1i16).to_be_bytes().to_vec())],                 // negative count
        vec![(b'T', [&1i16.to_be_bytes()[..], b"nonul"].concat())],   // unterminated name
        vec![(b'T', [&1i16.to_be_bytes()[..], &[0xff, 0]].concat())], // non-UTF8 name
        vec![(b'T', [&1i16.to_be_bytes()[..], b"a\0", &[0; 4]].concat())], // truncated 18-byte desc
        vec![(b'T', bad_fmt)],                                        // binary format code
        vec![(b'T', trailing_rd)], // trailing bytes in RowDescription
        vec![(b'D', data_row_payload(&[Some("a")]))], // DataRow without RowDescription
        vec![
            (b'T', row_description_payload(2)),
            (b'D', data_row_payload(&[Some("only_one")])),
        ], // column count mismatch
        vec![(b'T', row_description_payload(1)), (b'D', trailing_dr)], // trailing bytes in DataRow
    ];
    for frames in sequences {
        assert_query_protocol_error_poisons(frames);
    }

    // Write-side NUL rejection in SQL (and connection write failure poisoning).
    let (p_nul, h_nul) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        let _ = read_client_frame(&mut s);
    });
    let cfg_nul = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_nul}/db")).unwrap();
    let mut c_nul = PgClient::connect(&cfg_nul).expect("connect");
    assert!(matches!(
        c_nul.query("SELECT 1\0; DROP TABLE x", &[]),
        Err(PgError::Protocol(_))
    ));
    drop(c_nul);
    h_nul.join().unwrap();
}

#[test]
fn wire_validates_ready_for_query_and_rejects_premature_ready() {
    // 1. Startup malformed ReadyForQuery payload and non-idle status ('T', 'E') close socket.
    for bad_z in [
        vec![],
        b"II".to_vec(),
        b"X".to_vec(),
        b"T".to_vec(),
        b"E".to_vec(),
    ] {
        let (p, h) = spawn_one_shot(move |mut s| {
            let _ = read_startup(&mut s);
            send_frame(&mut s, b'R', &0i32.to_be_bytes());
            send_frame(&mut s, b'Z', &bad_z);
            assert!(read_client_frame(&mut s).is_none());
        });
        let cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p}/db")).unwrap();
        assert!(matches!(PgClient::connect(&cfg), Err(PgError::Protocol(_))));
        h.join().unwrap();
    }

    // 2. Extended query malformed ReadyForQuery and premature ReadyForQuery poison socket.
    for frames in [
        vec![(b'C', b"SELECT 0\0".to_vec()), (b'Z', vec![])],
        vec![(b'C', b"SELECT 0\0".to_vec()), (b'Z', b"II".to_vec())],
        vec![(b'C', b"SELECT 0\0".to_vec()), (b'Z', b"X".to_vec())],
        vec![(b'Z', b"I".to_vec())],
        vec![(b'1', vec![]), (b'2', vec![]), (b'Z', b"I".to_vec())],
        vec![(b'C', b"SELECT 0".to_vec()), (b'Z', b"I".to_vec())],
        vec![(b'C', b"SELECT 0\0extra".to_vec()), (b'Z', b"I".to_vec())],
        vec![(b'C', b"\xff\0".to_vec()), (b'Z', b"I".to_vec())],
        vec![(b'I', b"x".to_vec()), (b'Z', b"I".to_vec())],
        // Duplicate completion frames (e.g. ROLLBACK followed by SELECT 1) must poison socket.
        vec![
            (b'C', b"ROLLBACK\0".to_vec()),
            (b'C', b"SELECT 1\0".to_vec()),
            (b'Z', b"T".to_vec()),
        ],
        vec![
            (b'C', b"SELECT 1\0".to_vec()),
            (b'I', Vec::new()),
            (b'Z', b"I".to_vec()),
        ],
        vec![
            (b'I', Vec::new()),
            (b'C', b"SELECT 1\0".to_vec()),
            (b'Z', b"I".to_vec()),
        ],
        vec![
            (b'I', Vec::new()),
            (b'I', Vec::new()),
            (b'Z', b"I".to_vec()),
        ],
    ] {
        assert_query_protocol_error_poisons(frames);
    }

    // 3. EmptyQueryResponse ('I') followed by valid ReadyForQuery ('Z' 'I') succeeds.
    let (p_empty, h_empty) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        read_extended_query(&mut s);
        send_frame(&mut s, b'I', b"");
        send_frame(&mut s, b'Z', b"I");
        let _ = read_client_frame(&mut s);
    });
    let cfg_empty = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_empty}/db")).unwrap();
    let mut client = PgClient::connect(&cfg_empty).expect("connect");
    let res = client.query("", &[]).expect("empty query ok");
    assert!(res.rows.is_empty());
    drop(client);
    h_empty.join().unwrap();
}

#[test]
fn wire_transaction_rejects_aborted_commit_rollback_and_unexpected_outcome() {
    // 1. Callback swallows an error leaving tx_status == 'E'; client rolls back and returns TransactionAborted.
    let (p_aborted, h_aborted) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_tx_error_and_rollback(&mut s, "22012", "division by zero", None);
    });
    let cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_aborted}/db")).unwrap();
    let mut client = PgClient::connect(&cfg).expect("connect");
    let err = client
        .in_transaction(
            |tx| {
                let _ = tx.query("SELECT 1/0", &[]);
                Ok::<(), PgError>(())
            },
            |e| e,
        )
        .unwrap_err();
    assert!(matches!(err, PgError::TransactionAborted));
    assert_eq!(err.sqlstate(), None);
    let _ = err.into_store_error();
    drop(client);
    h_aborted.join().unwrap();

    // 2. COMMIT returning unexpected ReadyForQuery status ('T' instead of 'I') poisons socket.
    let (p_bad, h_bad) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        read_extended_query(&mut s);
        send_frame(&mut s, b'C', b"COMMIT\0");
        send_frame(&mut s, b'Z', b"T");
        let _ = read_client_frame(&mut s);
    });
    let cfg_bad = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_bad}/db")).unwrap();
    let mut c_bad = PgClient::connect(&cfg_bad).expect("connect");
    assert!(matches!(
        c_bad.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
        Err(PgError::Protocol(_))
    ));
    assert!(matches!(
        c_bad.query("SELECT 1", &[]),
        Err(PgError::ConnectionClosed)
    ));
    h_bad.join().unwrap();
}

#[test]
fn wire_transaction_tracks_tx_status_begin_self_rollback_and_unwind_poison() {
    // 1. Callback rolls back its own transaction leaving tx_status == 'I': refuses without COMMIT.
    let (mut c_rb, h_self_rb) = spawn_tx_single_reply(true, b"ROLLBACK\0", b"I");
    assert_tx_query_aborted(&mut c_rb, "ROLLBACK");
    drop(c_rb);
    h_self_rb.join().unwrap();

    // 2. Deferred rollback reported at COMMIT time (tag="ROLLBACK", status='I') returns TransactionAborted.
    let (mut c_def, h_def) = spawn_tx_single_reply(true, b"ROLLBACK\0", b"I");
    assert!(matches!(
        c_def.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
        Err(PgError::TransactionAborted)
    ));
    drop(c_def);
    h_def.join().unwrap();

    // 3. Unexpected BEGIN outcomes (wrong tag or non-'T' status) and COMMIT ("ROLLBACK" + 'T') poison socket.
    for (send_begin, tag, status) in [
        (false, &b"SELECT 0\0"[..], &b"T"[..]),
        (false, &b"BEGIN\0"[..], &b"I"[..]),
        (true, &b"ROLLBACK\0"[..], &b"T"[..]),
    ] {
        let (mut c_u, h_u) = spawn_tx_single_reply(send_begin, tag, status);
        assert!(matches!(
            c_u.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
            Err(PgError::Protocol(_))
        ));
        assert!(matches!(
            c_u.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
            Err(PgError::ConnectionClosed)
        ));
        h_u.join().unwrap();
    }

    // 4. ROLLBACK returning non-'I' status or non-ROLLBACK tag poisons the connection.
    for (tag, status) in [
        (&b"ROLLBACK\0"[..], &b"T"[..]),
        (&b"SELECT 1\0"[..], &b"I"[..]),
    ] {
        let (mut c_rbb, h_rb_bad) = spawn_tx_single_reply(true, tag, status);
        assert!(matches!(
            c_rbb.in_transaction(|_| Err::<(), PgError>(PgError::EmptyWorkerId), |e| e),
            Err(PgError::EmptyWorkerId)
        ));
        assert!(matches!(
            c_rbb.query("SELECT 1", &[]),
            Err(PgError::ConnectionClosed)
        ));
        h_rb_bad.join().unwrap();
    }
}

#[test]
fn wire_transaction_refuses_chained_end_and_nested_entry() {
    // 1. ROLLBACK AND CHAIN / COMMIT AND CHAIN / END / ABORT inside callback leaves
    // tx_status == 'T' on the server; client MUST roll back (never COMMIT) and return TransactionAborted.
    for (sql, tag) in [
        ("ROLLBACK AND CHAIN", &b"ROLLBACK\0"[..]),
        ("COMMIT AND CHAIN", &b"COMMIT\0"[..]),
        ("END", &b"END\0"[..]),
        ("ABORT", &b"ABORT\0"[..]),
    ] {
        let (p, h) = spawn_one_shot(move |mut s| {
            complete_trust_startup(&mut s);
            send_begin_ok(&mut s);
            expect_query_reply(&mut s, sql, tag, b"T");
            expect_query_reply(&mut s, "SELECT 1", b"SELECT 1\0", b"T");
            expect_query_reply(&mut s, "ROLLBACK", b"ROLLBACK\0", b"I");
            let _ = read_client_frame(&mut s);
        });
        let cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p}/db")).unwrap();
        let mut client = PgClient::connect(&cfg).expect("connect");
        let err = client
            .in_transaction(
                |tx| {
                    tx.query(sql, &[])?;
                    tx.query("SELECT 1", &[])?;
                    Ok::<(), PgError>(())
                },
                |e| e,
            )
            .unwrap_err();
        assert!(matches!(err, PgError::TransactionAborted));
        drop(client);
        h.join().unwrap();
    }

    // 2. Nested in_transaction inside an active in_transaction is refused before BEGIN
    //    both while tx_status == 'T' and after ROLLBACK drops server tx_status to 'I'.
    let (p_nest, h_nest) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        expect_query_reply(&mut s, "COMMIT", b"COMMIT\0", b"I");
        let _ = read_client_frame(&mut s);
    });
    let cfg_nest = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_nest}/db")).unwrap();
    let mut c_nest = PgClient::connect(&cfg_nest).expect("connect");
    c_nest
        .in_transaction(
            |tx| {
                assert_nested_tx_refused(tx);
                Ok::<(), PgError>(())
            },
            |e| e,
        )
        .expect("outer commits cleanly");
    drop(c_nest);
    h_nest.join().unwrap();

    let (p_idle_nest, h_idle_nest) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        expect_query_reply(&mut s, "ROLLBACK", b"ROLLBACK\0", b"I");
        assert!(matches!(read_client_frame(&mut s), Some((b'X', _))));
    });
    let cfg_idle = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_idle_nest}/db")).unwrap();
    let mut c_idle = PgClient::connect(&cfg_idle).expect("connect");
    let outer_err = c_idle
        .in_transaction(
            |tx| {
                tx.query("ROLLBACK", &[])?;
                assert_nested_tx_refused(tx);
                Ok::<(), PgError>(())
            },
            |e| e,
        )
        .unwrap_err();
    assert!(matches!(outer_err, PgError::TransactionAborted));
    drop(c_idle);
    h_idle_nest.join().unwrap();

    // 3. Entering in_transaction while connection is already in 'T' (manual BEGIN) refuses before BEGIN.
    let (p_open, h_open) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        assert!(matches!(read_client_frame(&mut s), Some((b'X', _))));
    });
    let cfg_open = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_open}/db")).unwrap();
    let mut c_open = PgClient::connect(&cfg_open).expect("connect");
    c_open.query("BEGIN", &[]).expect("manual begin");
    assert!(matches!(
        c_open.in_transaction(|_| Ok::<(), PgError>(()), |e| e),
        Err(PgError::TransactionAborted)
    ));
    drop(c_open);
    h_open.join().unwrap();
}

#[test]
fn wire_transaction_compromise_overrides_callback_error_while_preserving_ordinary_errors() {
    // 1. After a tx-ending tag (ROLLBACK AND CHAIN), a later failing query returning Err
    //    must still roll back and surface TransactionAborted.
    let (p_chain_err, h_chain_err) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        expect_query_reply(&mut s, "ROLLBACK AND CHAIN", b"ROLLBACK\0", b"T");
        assert_eq!(read_extended_query(&mut s), "SELECT 1/0");
        send_frame(
            &mut s,
            b'E',
            &error_payload("22012", "division by zero", None),
        );
        send_frame(&mut s, b'Z', b"E");
        expect_query_reply(&mut s, "ROLLBACK", b"ROLLBACK\0", b"I");
        let _ = read_client_frame(&mut s);
    });
    let cfg_ce = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_chain_err}/db")).unwrap();
    let mut c_ce = PgClient::connect(&cfg_ce).expect("connect");
    let ce_err = c_ce
        .in_transaction(
            |tx| {
                tx.query("ROLLBACK AND CHAIN", &[])?;
                tx.query("SELECT 1/0", &[]).map(|_| ())
            },
            |e| e,
        )
        .unwrap_err();
    assert!(matches!(ce_err, PgError::TransactionAborted));
    drop(c_ce);
    h_chain_err.join().unwrap();

    // 2. After an unexpected idle transition (non-ending tag with status 'I'), a callback Err
    //    skips redundant ROLLBACK and still surfaces TransactionAborted.
    let (mut c_idle_err, h_idle_err) = spawn_tx_single_reply(true, b"SELECT 1\0", b"I");
    let ie_err = c_idle_err
        .in_transaction(
            |tx| {
                tx.query("SELECT 1", &[])?;
                Err::<(), PgError>(PgError::EmptyWorkerId)
            },
            |e| e,
        )
        .unwrap_err();
    assert!(matches!(ie_err, PgError::TransactionAborted));
    drop(c_idle_err);
    h_idle_err.join().unwrap();

    // 3. Ordinary SQL error inside callback (no tx-ending tag) rolls back and preserves original error.
    let (p_ord, h_ord) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_tx_error_and_rollback(&mut s, "22012", "division by zero", None);
    });
    let cfg_ord = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_ord}/db")).unwrap();
    let mut c_ord = PgClient::connect(&cfg_ord).expect("connect");
    let ord_err = c_ord
        .in_transaction(|tx| tx.query("SELECT 1/0", &[]).map(|_| ()), |e| e)
        .unwrap_err();
    assert_eq!(ord_err.sqlstate(), Some("22012"));
    drop(c_ord);
    h_ord.join().unwrap();
}

fn mixed_error_completion_frames() -> [Vec<(u8, Vec<u8>)>; 2] {
    [
        vec![
            (b'C', b"COMMIT\0".to_vec()),
            (b'E', error_payload("40001", "boom", None)),
            (b'Z', b"E".to_vec()),
        ],
        vec![
            (b'E', error_payload("40001", "first", None)),
            (b'E', error_payload("40001", "second", None)),
            (b'Z', b"I".to_vec()),
        ],
    ]
}

#[test]
fn wire_rejects_mixed_error_and_post_completion_row_frames() {
    for frames in mixed_error_completion_frames() {
        assert_eq!(
            assert_query_protocol_error_poisons(frames),
            "duplicate query completion before ReadyForQuery"
        );
    }
    for (frames, want_msg) in [
        (
            vec![
                (b'E', error_payload("40001", "boom", None)),
                (b'C', b"SELECT 1\0".to_vec()),
                (b'Z', b"I".to_vec()),
            ],
            "duplicate query completion before ReadyForQuery",
        ),
        (
            vec![
                (b'T', row_description_payload(1)),
                (b'T', row_description_payload(1)),
                (b'C', b"SELECT 0\0".to_vec()),
                (b'Z', b"I".to_vec()),
            ],
            "unexpected RowDescription frame",
        ),
        (
            vec![
                (b'C', b"SELECT 0\0".to_vec()),
                (b'T', row_description_payload(1)),
                (b'Z', b"I".to_vec()),
            ],
            "unexpected RowDescription frame",
        ),
        (
            vec![
                (b'T', row_description_payload(1)),
                (b'C', b"SELECT 1\0".to_vec()),
                (b'D', data_row_payload(&[Some("a")])),
                (b'Z', b"I".to_vec()),
            ],
            "DataRow after completion",
        ),
        (
            vec![
                (b'T', row_description_payload(1)),
                (b'D', data_row_payload(&[Some("a")])),
                (b'I', Vec::new()),
                (b'Z', b"I".to_vec()),
            ],
            "unexpected EmptyQueryResponse after RowDescription",
        ),
        (
            vec![
                (b'T', row_description_payload(1)),
                (b'I', Vec::new()),
                (b'Z', b"I".to_vec()),
            ],
            "unexpected EmptyQueryResponse after RowDescription",
        ),
        (
            vec![(b'I', b"x".to_vec()), (b'Z', b"I".to_vec())],
            "non-empty EmptyQueryResponse payload (1 bytes)",
        ),
        (
            vec![(b'Z', b"I".to_vec())],
            "premature ReadyForQuery before query completion",
        ),
        (
            vec![(b'D', data_row_payload(&[Some("a")]))],
            "received DataRow without RowDescription",
        ),
        (
            vec![
                (b'T', row_description_payload(2)),
                (b'D', data_row_payload(&[Some("a")])),
            ],
            "DataRow column count 1 does not match RowDescription count 2",
        ),
        (
            vec![(b'?', Vec::new())],
            "unexpected query message tag 0x3f",
        ),
    ] {
        assert_eq!(assert_query_protocol_error_poisons(frames), want_msg);
    }
}

#[test]
fn wire_tx_rejects_active_status_after_error_and_preserves_compromise_on_close() {
    // 1. Inside in_transaction (and on BEGIN itself), mixed/duplicate error completion
    //    or ErrorResponse followed by ReadyForQuery('T') poisons the socket.
    let [ce_frames, ee_frames] = mixed_error_completion_frames();
    for (send_begin, frames) in [
        (true, ce_frames),
        (true, ee_frames),
        (
            true,
            vec![
                (b'E', error_payload("22012", "still active", None)),
                (b'Z', b"T".to_vec()),
            ],
        ),
        (
            false,
            vec![
                (b'E', error_payload("25001", "begin active", None)),
                (b'Z', b"T".to_vec()),
            ],
        ),
    ] {
        let (p_err, h_err) = spawn_one_shot(move |mut s| {
            complete_trust_startup(&mut s);
            if send_begin {
                send_begin_ok(&mut s);
            }
            let _ = read_extended_query(&mut s);
            for (tag, body) in &frames {
                send_frame(&mut s, *tag, body);
            }
            let _ = read_client_frame(&mut s);
        });
        let cfg_err = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_err}/db")).unwrap();
        let mut c_err = PgClient::connect(&cfg_err).expect("connect");
        assert!(matches!(
            c_err.in_transaction(|tx| tx.query("SELECT 1/0", &[]).map(|_| ()), |e| e),
            Err(PgError::Protocol(_))
        ));
        assert!(matches!(
            c_err.query("SELECT 1", &[]),
            Err(PgError::ConnectionClosed)
        ));
        h_err.join().unwrap();
    }

    // 2. Mid-callback EOF (close()) after COMMIT+'I' must NOT clear tx_compromised.
    let (p_eof, h_eof) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        expect_query_reply(&mut s, "COMMIT", b"COMMIT\0", b"I");
        let _ = read_extended_query(&mut s);
    });
    let cfg_eof = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_eof}/db")).unwrap();
    let mut c_eof = PgClient::connect(&cfg_eof).expect("connect");
    let eof_err = c_eof
        .in_transaction(
            |tx| {
                tx.query("COMMIT", &[])?;
                assert!(matches!(tx.query("SELECT 1", &[]), Err(PgError::Io(_))));
                Err::<(), PgError>(PgError::EmptyWorkerId)
            },
            |e| e,
        )
        .unwrap_err();
    assert!(matches!(eof_err, PgError::TransactionAborted));
    assert!(matches!(
        c_eof.query("SELECT 1", &[]),
        Err(PgError::ConnectionClosed)
    ));
    h_eof.join().unwrap();
}

#[test]
fn wire_transaction_poisons_connection_on_callback_panic() {
    // Panicking callback inside in_transaction poisons/closes client on unwind.
    let (p_panic, h_panic) = spawn_one_shot(|mut s| {
        complete_trust_startup(&mut s);
        send_begin_ok(&mut s);
        let _ = read_client_frame(&mut s);
    });
    let cfg_p = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_panic}/db")).unwrap();
    let mut c_p = PgClient::connect(&cfg_p).expect("connect");
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = c_p.in_transaction(
            |_| -> Result<(), PgError> { panic!("tx callback panic") },
            |e| e,
        );
    }));
    assert!(caught.is_err());
    assert!(matches!(
        c_p.query("SELECT 1", &[]),
        Err(PgError::ConnectionClosed)
    ));
    h_panic.join().unwrap();
}

#[test]
fn wire_enforces_strict_auth_state_transitions_and_address_fallback() {
    // 1. Cleartext downgrade after SCRAM start MUST be refused.
    let (p_down, h_down) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_scram_offer(&mut s);
        let _ = read_client_frame(&mut s);
        send_frame(&mut s, b'R', &3i32.to_be_bytes());
    });
    let cfg_down = PgConfig::parse(&format!("postgres://u:pw@127.0.0.1:{p_down}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_down),
        Err(PgError::Auth(_))
    ));
    h_down.join().unwrap();

    // 2. Restarting AuthenticationSASL after SCRAM start MUST be refused.
    let (p_restart, h_restart) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_scram_offer(&mut s);
        let _ = read_client_frame(&mut s);
        send_scram_offer(&mut s);
    });
    let cfg_restart =
        PgConfig::parse(&format!("postgres://u:pw@127.0.0.1:{p_restart}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_restart),
        Err(PgError::Auth(_))
    ));
    h_restart.join().unwrap();

    // 3. Out-of-order SASLFinal before SASLContinue and duplicate AuthenticationOk.
    let (p_ooo, h_ooo) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_scram_offer(&mut s);
        let _ = read_client_frame(&mut s);
        send_frame(
            &mut s,
            b'R',
            &[&12i32.to_be_bytes()[..], b"v=AAAA"].concat(),
        );
    });
    let cfg_ooo = PgConfig::parse(&format!("postgres://u:pw@127.0.0.1:{p_ooo}/db")).unwrap();
    assert!(matches!(PgClient::connect(&cfg_ooo), Err(PgError::Auth(_))));
    h_ooo.join().unwrap();

    let (p_dup, h_dup) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_frame(&mut s, b'R', &0i32.to_be_bytes());
        send_frame(&mut s, b'R', &0i32.to_be_bytes());
    });
    let cfg_dup = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p_dup}/db")).unwrap();
    assert!(matches!(PgClient::connect(&cfg_dup), Err(PgError::Auth(_))));
    h_dup.join().unwrap();

    // 4. Non-UTF8 SASLContinue (in SaslStarted) and non-UTF8 SASLFinal (in SaslContinued).
    let (p_bad_c, h_bad_c) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_scram_offer(&mut s);
        let _ = read_client_frame(&mut s);
        send_frame(&mut s, b'R', &[&11i32.to_be_bytes()[..], &[0xff]].concat());
    });
    let cfg_bad_c = PgConfig::parse(&format!("postgres://u:pw@127.0.0.1:{p_bad_c}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_bad_c),
        Err(PgError::Auth(_))
    ));
    h_bad_c.join().unwrap();

    let (p_bad_f, h_bad_f) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_scram_offer(&mut s);
        let (_, body) = read_client_frame(&mut s).unwrap();
        let mech_end = body.iter().position(|&b| b == 0).unwrap();
        let init_str = std::str::from_utf8(&body[mech_end + 5..]).unwrap();
        let client_nonce = init_str.strip_prefix("n,,n=,r=").unwrap();
        let sf = format!("r={client_nonce}srv,s=AAAA,i=1");
        send_frame(
            &mut s,
            b'R',
            &[&11i32.to_be_bytes()[..], sf.as_bytes()].concat(),
        );
        let _ = read_client_frame(&mut s);
        send_frame(&mut s, b'R', &[&12i32.to_be_bytes()[..], &[0xff]].concat());
    });
    let cfg_bad_f = PgConfig::parse(&format!("postgres://u:pw@127.0.0.1:{p_bad_f}/db")).unwrap();
    assert!(matches!(
        PgClient::connect(&cfg_bad_f),
        Err(PgError::Auth(_))
    ));
    h_bad_f.join().unwrap();
}

#[test]
fn wire_address_fallback_and_nul_text_codec() {
    // 1. Address fallback: unreachable address followed by reachable listener succeeds.
    let closed_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let unreachable_addr = closed_listener.local_addr().unwrap();
    drop(closed_listener);
    let reachable_listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let reachable_addr = reachable_listener.local_addr().unwrap();
    let accept_handle = thread::spawn(move || {
        let (_stream, _) = reachable_listener.accept().expect("accept fallback");
    });
    let stream = connect_first_reachable(
        &[unreachable_addr, reachable_addr],
        "dual.local",
        Duration::from_millis(200),
    )
    .expect("fallback to reachable listener");
    assert_eq!(stream.peer_addr().unwrap(), reachable_addr);
    drop(stream);
    accept_handle.join().unwrap();

    // 6. Lossless NUL text codec round-trip and malformed-escape refusal.
    for sample in ["plain", "a\0b", "a\\b", "a\\0b", "a\\\0b", ""] {
        let encoded = encode_pg_text(sample);
        assert!(!encoded.as_bytes().contains(&0));
        assert_eq!(decode_pg_text(&encoded).unwrap(), sample);
    }
    for malformed in ["bad\\x", "trailing\\"] {
        assert!(matches!(
            decode_pg_text(malformed),
            Err(PgError::Protocol(_))
        ));
    }
}
