use super::*;
use forge_core::envelope::EventType;
use serde_json::json;
use std::io::{Read, Write};

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    (dir, db)
}

#[test]
fn page_runs_and_run_detail_serve_read_only() {
    let (_dir, db) = fixture();
    let page = handle(&db, "/");
    assert_eq!(page.status, "200 OK");
    assert!(page.body.contains("<title>forge</title>"));

    let runs = handle(&db, "/api/runs");
    let parsed: Value = serde_json::from_str(&runs.body).unwrap();
    assert_eq!(parsed[0]["run_id"], "r1");
    assert_eq!(parsed[0]["phase"], "intake");

    let detail = handle(&db, "/api/run/r1");
    let parsed: Value = serde_json::from_str(&detail.body).unwrap();
    assert_eq!(parsed["summary"]["status"], "running");
    assert_eq!(parsed["events"].as_array().unwrap().len(), 2);

    assert_eq!(handle(&db, "/api/run/nope").status, "404 Not Found");
    assert_eq!(handle(&db, "/definitely-not").status, "404 Not Found");
}

#[test]
fn rebinding_and_methods_are_rejected_and_reads_never_create() {
    // DNS-rebinding guard: only loopback Host names, only GET.
    assert!(request_allowed("GET", Some("127.0.0.1:8383")));
    assert!(request_allowed("GET", Some("localhost")));
    assert!(request_allowed("GET", Some("[::1]:9000")));
    assert!(!request_allowed("GET", Some("evil.example.com")));
    assert!(!request_allowed("GET", Some("evil.example.com:8383")));
    assert!(!request_allowed("GET", None));
    assert!(!request_allowed("POST", Some("127.0.0.1")));

    // A read of a missing database is a 404, not a created store.
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("absent.db");
    assert_eq!(handle(&db, "/api/runs").status, "404 Not Found");
    assert!(!db.exists(), "reads must never create the database");
}

fn exchange(db: PathBuf, request: &str, sse_limit: Option<usize>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        serve_client_with_limit(db, stream, sse_limit);
    });
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(request.as_bytes()).unwrap();
    client.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    server.join().unwrap();
    response
}

#[test]
fn tcp_shell_serves_forbidden_json_and_bounded_sse_heartbeats() {
    let (_dir, db) = fixture();
    let response = exchange(
        db.clone(),
        "GET /api/run/r1 HTTP/1.1\r\nHost: LOCALHOST:8383\r\n\r\n",
        None,
    );
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains("\"run_id\":\"r1\""));

    let forbidden = exchange(
        db.clone(),
        "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        None,
    );
    assert!(forbidden.starts_with("HTTP/1.1 403 Forbidden"));

    let sse = exchange(
        db,
        "GET /sse/r1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        Some(2),
    );
    assert!(sse.contains("Content-Type: text/event-stream"));
    assert!(sse.contains("data: {\"seq\":2}"));
    assert!(sse.contains(": ping"));
}

struct BrokenReader;

impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken request"))
    }
}

impl BufRead for BrokenReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Err(std::io::Error::other("broken request"))
    }

    fn consume(&mut self, _: usize) {}
}

struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken response"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct FailAfterOneWrite {
    writes: usize,
}

impl Write for FailAfterOneWrite {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.writes += 1;
        if self.writes == 1 {
            Ok(bytes.len())
        } else {
            Err(std::io::Error::other("client disconnected"))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn request_parser_store_errors_and_all_statuses_are_explicit() {
    assert!(read_request(&mut BrokenReader).is_err());
    let mut request = std::io::Cursor::new(b"GET\r\nHost: localhost\r\n".to_vec());
    let (method, path, host) = read_request(&mut request).unwrap();
    assert_eq!(method, "GET");
    assert_eq!(path, "/");
    assert_eq!(host.as_deref(), Some("localhost"));
    let mut without_host = std::io::Cursor::new(b"GET / HTTP/1.1\r\nX-Test: yes\r\n\r\n".to_vec());
    assert_eq!(read_request(&mut without_host).unwrap().2, None);

    for (status, text) in [
        (Status::Running, "running"),
        (Status::AwaitingOperator, "awaiting_operator"),
        (Status::Completed, "completed"),
        (Status::Stopped, "stopped"),
    ] {
        assert_eq!(status_str(&status), text);
    }

    let dir = tempfile::tempdir().unwrap();
    let corrupt = dir.path().join("corrupt.db");
    std::fs::write(&corrupt, "not sqlite").unwrap();
    assert_eq!(
        handle(&corrupt, "/api/runs").status,
        "500 Internal Server Error"
    );
    assert_eq!(head_seq(&corrupt, "run"), 0);

    let malformed_schema = dir.path().join("malformed-schema.db");
    let connection = rusqlite::Connection::open(&malformed_schema).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta VALUES ('database_schema', '1');
             CREATE TABLE runs (run_id TEXT PRIMARY KEY);
             CREATE TABLE events (
               run_id TEXT NOT NULL, seq INTEGER NOT NULL, event_hash TEXT NOT NULL,
               envelope TEXT NOT NULL, PRIMARY KEY (run_id, seq)
             );",
        )
        .unwrap();
    drop(connection);
    assert_eq!(handle(&malformed_schema, "/api/runs").body, "[]");

    let mut output = Vec::new();
    serve_io(&db_for_missing(), &mut BrokenReader, &mut output, None);

    let mut get = std::io::Cursor::new(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec());
    serve_io(Path::new("missing.db"), &mut get, &mut BrokenWriter, None);
    let mut sse =
        std::io::Cursor::new(b"GET /sse/run HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec());
    serve_io(
        Path::new("missing.db"),
        &mut sse,
        &mut BrokenWriter,
        Some(1),
    );
    let mut sse =
        std::io::Cursor::new(b"GET /sse/run HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec());
    serve_io(
        &dir.path().join("missing.db"),
        &mut sse,
        &mut FailAfterOneWrite::default(),
        Some(1),
    );
}

fn db_for_missing() -> PathBuf {
    PathBuf::from("missing.db")
}

#[test]
fn listener_open_hook_is_testable_and_public_bind_errors_return() {
    let (_dir, db) = fixture();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let client = std::thread::spawn(move || {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
    });
    let mut opened = None;
    serve_listener(db.clone(), listener, true, Some(1), |url| {
        opened = Some(url.to_string())
    })
    .unwrap();
    client.join().unwrap();
    assert!(opened.unwrap().starts_with("http://127.0.0.1:"));

    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = occupied.local_addr().unwrap().port();
    assert!(serve(db, port, false).is_err());

    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let server_db = fixture().1;
    std::thread::spawn(move || {
        let _ = serve(server_db, port, false);
    });
    for _ in 0..100 {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            stream
                .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .unwrap();
            break;
        }
        std::thread::yield_now();
    }

    std::env::set_var("FORGE_BROWSER_BIN", "true");
    open_system_browser("http://127.0.0.1:9/");
    std::env::remove_var("FORGE_BROWSER_BIN");
}
