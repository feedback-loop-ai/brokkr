//! Unit tests for `config`, `scram`, `error`, and `wire` over an in-process
//! mock PostgreSQL v3 TCP wire server (`127.0.0.1:0`).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::config::{PgConfig, DEFAULT_CONNECT_TIMEOUT, DEFAULT_IO_TIMEOUT, DEFAULT_PORT};
use crate::error::PgError;
use crate::scram::{
    base64_decode, base64_encode, constant_time_eq, hmac_sha256, pbkdf2_hmac_sha256, ScramClient,
};
use crate::wire::{resolve_first_addr, PgClient};

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
    let hex_cases = PgConfig::parse("postgres://u:p%3A%3a@localhost/mydb").expect("hex cases");
    assert_eq!(hex_cases.password.as_deref(), Some("p::"));

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
    assert!(!constant_time_eq(b"abc", b"abd"));

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
}

#[test]
fn wire_scram_sha_256_handshake_and_query_round_trip() {
    let (port, handle) = spawn_one_shot(|mut s| {
        let startup = read_startup(&mut s);
        assert!(startup.windows(15).any(|w| w == b"brokkr-store-pg"));
        let mut sasl = Vec::new();
        sasl.extend_from_slice(&10i32.to_be_bytes());
        sasl.extend_from_slice(b"SCRAM-SHA-256\0\0");
        send_frame(&mut s, b'R', &sasl);

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
        send_frame(&mut s, b'T', &[0, 3]);
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

        for _ in 0..5 {
            let _ = read_client_frame(&mut s);
        }
        send_frame(&mut s, b'1', b"");
        send_frame(&mut s, b'2', b"");
        send_frame(&mut s, b'n', b"");
        send_frame(&mut s, b'C', b"BEGIN\0");
        send_frame(&mut s, b'Z', b"T");

        for _ in 0..5 {
            let _ = read_client_frame(&mut s);
        }
        send_frame(
            &mut s,
            b'E',
            &error_payload("23505", "dup key", Some("brokkr_runs_pkey")),
        );
        send_frame(&mut s, b'Z', b"E");

        for _ in 0..5 {
            let _ = read_client_frame(&mut s);
        }
        send_frame(&mut s, b'C', b"ROLLBACK\0");
        send_frame(&mut s, b'Z', b"I");

        let _ = read_client_frame(&mut s);
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
        let (p, h) = spawn_one_shot(move |mut s| {
            let _ = read_startup(&mut s);
            send_frame(&mut s, b'R', &0i32.to_be_bytes());
            send_frame(&mut s, b'Z', b"I");
            for _ in 0..5 {
                let _ = read_client_frame(&mut s);
            }
            send_frame(&mut s, bad_frame.0, &bad_frame.1);
            let _ = read_client_frame(&mut s);
        });
        let cfg = PgConfig::parse(&format!("postgres://u@127.0.0.1:{p}/db")).unwrap();
        let mut c = PgClient::connect(&cfg).expect("trust connect");
        assert!(matches!(
            c.query("SELECT 1", &[]),
            Err(PgError::Protocol(_))
        ));
        drop(c);
        h.join().unwrap();
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
        let mut sasl = Vec::new();
        sasl.extend_from_slice(&10i32.to_be_bytes());
        sasl.extend_from_slice(b"SCRAM-SHA-256\0\0");
        send_frame(&mut s, b'R', &sasl);
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
        resolve_first_addr(std::iter::empty(), "empty.invalid"),
        Err(PgError::Protocol(_))
    ));

    let (p_many, h_many) = spawn_one_shot(|mut s| {
        let _ = read_startup(&mut s);
        send_frame(&mut s, b'R', &0i32.to_be_bytes());
        send_frame(&mut s, b'Z', b"I");
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
