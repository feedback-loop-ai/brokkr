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

#[test]
fn the_view_endpoint_serves_the_models_the_page_paints() {
    let (_dir, db) = fixture();
    let view = handle(&db, "/api/view/r1");
    assert_eq!(view.status, "200 OK");
    let parsed: Value = serde_json::from_str(&view.body).unwrap();
    // VIEW_VERSION moved to 2 with decision 0016: participants gained
    // `provenance` and the run view gained `notices`.
    assert_eq!(parsed["view_version"], forge_view::VIEW_VERSION);
    assert_eq!(parsed["summary"]["run_id"], "r1");
    assert_eq!(parsed["summary"]["status"], "running");
    assert_eq!(parsed["event_count"], 2);
    assert_eq!(parsed["phases"][0]["name"], "intake");
    assert_eq!(parsed["phases"][0]["current"], true);
    // Absent values serialize as null, never skipped: a consumer can
    // tell "the journal does not carry this" from "your version lacks
    // the field".
    assert_eq!(parsed["ruling"], Value::Null);
    assert!(parsed["journal"][0]["phases"].is_array());

    // Unknown run and missing database are both 404 — reads never create.
    assert_eq!(handle(&db, "/api/view/nope").status, "404 Not Found");
    let empty = tempfile::tempdir().unwrap();
    let absent = empty.path().join("absent.db");
    assert_eq!(handle(&absent, "/api/view/r1").status, "404 Not Found");
    assert!(!absent.exists());

    // The new route lives inside `handle`, so it inherits the
    // DNS-rebinding guard structurally rather than by repetition.
    assert!(!request_allowed("GET", Some("evil.example.com")));
    assert!(!request_allowed("POST", Some("127.0.0.1")));

    // /api/runs answers with the reserialized rows, newest first.
    let runs: Value = serde_json::from_str(&handle(&db, "/api/runs").body).unwrap();
    assert_eq!(runs[0]["run_id"], "r1");
    assert_eq!(runs[0]["status_known"], true);
    assert_eq!(runs[0]["feature"], "feat");
}

#[test]
fn the_page_paints_and_derives_nothing() {
    // Decision 0013's load-bearing clause: the JS derivation is DELETED,
    // not duplicated. The page may branch on a model field; it may not
    // compute one. Each of these tokens is a derivation the page used to
    // carry and `forge-view` now owns.
    for banned in [
        "buildParticipants",
        "innerColumns",
        "fmtDur",
        "shortTarget",
        "toFixed",
        "Date.parse",
        "JSON.stringify",
        "innerHTML",
    ] {
        assert!(
            !PAGE.contains(banned),
            "ui.html must not derive: {banned} belongs in forge-view"
        );
    }
    // What it DOES carry: model consumption and geometry.
    for kept in ["/api/view/", "renderLoops", "svgEl", "textContent"] {
        assert!(PAGE.contains(kept), "the page still paints with {kept}");
    }
}

/// The traversal guard lives with the lookup, not in the HTTP layer: a
/// second caller — decision 0014's TUI — reaches `session_turns`
/// directly, and a refactor that left validation behind would hand it a
/// path traversal.
#[test]
fn the_session_lookup_carries_its_own_id_validation() {
    for bad in ["", "../../etc/passwd", &"a".repeat(65), "/etc/passwd"] {
        assert!(
            session_turns(bad).is_none(),
            "session_turns itself refuses {bad:?}"
        );
    }
}

/// The transcript drill (`/api/session/<id>`) is journal-independent and
/// operator-local: it locates the seat's own Claude transcript by id.
/// The endpoint predates this change and its responses are untouched;
/// what it lacked was a test, and the exact-coverage gate is the reason
/// display truth is being landed in Rust at all.
#[test]
fn the_transcript_drill_reads_a_local_session_or_says_why_it_cannot() {
    let missing_db = PathBuf::from("missing.db");

    // The id is validated STRICTLY, before any path is formed: empty,
    // over-long, and anything outside hex-and-dash are all refused.
    for bad in ["", "../../etc/passwd", &"a".repeat(65)] {
        let response = handle(&missing_db, &format!("/api/session/{bad}"));
        assert_eq!(response.status, "404 Not Found", "id {bad:?}");
        assert!(response.body.contains("session not found"), "id {bad:?}");
    }

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let projects = home.join(".claude").join("projects");
    // Two project directories: the first does not hold the file, so the
    // scan must keep looking rather than conclude on the first miss.
    std::fs::create_dir_all(projects.join("empty-project")).unwrap();
    std::fs::create_dir_all(projects.join("real-project")).unwrap();

    let transcript = concat!(
        // Not JSON at all: skipped, never guessed at.
        "not json\n",
        // A record that is neither a user nor an assistant turn.
        "{\"type\":\"summary\"}\n",
        // A user turn with no message: no blocks, so no turn.
        "{\"type\":\"user\"}\n",
        // String content, with an explicit role and timestamp.
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",",
        "\"content\":\"the plain form\"},\"timestamp\":\"2026-01-01T00:00:00Z\"}\n",
        // Block content: prose, whitespace-only prose, a text block with
        // no text, a tool with a file target, a tool without one, and a
        // block kind this surface does not render.
        "{\"type\":\"user\",\"message\":{\"content\":[",
        "{\"type\":\"text\",\"text\":\"what happened\"},",
        "{\"type\":\"text\",\"text\":\"   \"},",
        "{\"type\":\"text\"},",
        "{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{\"file_path\":\"src/lib.rs\"}},",
        "{\"type\":\"tool_use\",\"name\":\"Bash\"},",
        "{\"type\":\"thinking\"}]}}\n",
    );
    std::fs::write(projects.join("real-project/abcd-1234.jsonl"), transcript).unwrap();

    // A file that is not valid UTF-8 is unreadable, not repaired.
    std::fs::write(projects.join("real-project/dead-beef.jsonl"), [0xff, 0xfe]).unwrap();

    // Past the size cap the response is truncated rather than unbounded.
    let mut oversized =
        String::from("{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"");
    oversized.push_str(&"x".repeat(4_000_001));
    oversized.push_str("\"}}\n");
    std::fs::write(projects.join("real-project/0000-1111.jsonl"), &oversized).unwrap();

    let previous_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let response = handle(&missing_db, "/api/session/abcd-1234");
    assert_eq!(response.status, "200 OK");
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(parsed["session_id"], "abcd-1234");
    assert_eq!(parsed["truncated"], false);
    let turns = parsed["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2, "two turns carried prose: {}", response.body);
    assert_eq!(turns[0]["role"], "assistant");
    assert_eq!(turns[0]["ts"], "2026-01-01T00:00:00Z");
    assert_eq!(turns[0]["blocks"][0]["text"], "the plain form");
    // The role falls back to the record type and the stamp to empty.
    assert_eq!(turns[1]["role"], "user");
    assert_eq!(turns[1]["ts"], "");
    let blocks = turns[1]["blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 3, "blank prose and unknown kinds drop out");
    assert_eq!(blocks[0]["kind"], "text");
    assert_eq!(blocks[0]["text"], "what happened");
    // Tool markers carry file targets only — never prose or commands.
    assert_eq!(blocks[1]["kind"], "tool");
    assert_eq!(blocks[1]["text"], "Read · src/lib.rs");
    assert_eq!(blocks[2]["text"], "Bash");

    let truncated = handle(&missing_db, "/api/session/0000-1111");
    let parsed: Value = serde_json::from_str(&truncated.body).unwrap();
    assert_eq!(parsed["truncated"], true, "the size cap holds");
    assert!(parsed["turns"].as_array().unwrap().is_empty());

    assert_eq!(
        handle(&missing_db, "/api/session/dead-beef").status,
        "404 Not Found",
        "unreadable bytes are not a transcript"
    );
    assert_eq!(
        handle(&missing_db, "/api/session/9999-9999").status,
        "404 Not Found",
        "no such session on this machine"
    );

    // No projects directory at all: the scan finds nothing and says so.
    std::env::set_var("HOME", dir.path().join("elsewhere"));
    assert_eq!(
        handle(&missing_db, "/api/session/abcd-1234").status,
        "404 Not Found"
    );

    // No HOME: there is nowhere to look, and nothing is invented.
    std::env::remove_var("HOME");
    assert_eq!(
        handle(&missing_db, "/api/session/abcd-1234").status,
        "404 Not Found"
    );
    if let Some(previous_home) = previous_home {
        std::env::set_var("HOME", previous_home);
    }
}

/// AC-8's fourth surface: the console's payload carries the same two
/// model fields the other three read, and the page paints them from the
/// model rather than composing anything of its own.
#[test]
fn the_console_serves_and_paints_agent_provenance() {
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
            json!({"feature": "feat", "manifest": {"agents": {"intake": {
                "notices": [{"message": "optional capability gap"}],
            }}}}),
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
    store
        .append_next(
            "r1",
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "intake", "phase": "intake",
                   "idempotency_key": "k", "input_digest": "d"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a1", "driver": "d",
                   "provenance": [{"member": null, "agent": "intake",
                                   "model": "opus", "provider": "claude",
                                   "chain_index": 1}]}),
            None,
            None,
        )
        .unwrap();

    let view = handle(&db, "/api/view/r1");
    let parsed: Value = serde_json::from_str(&view.body).unwrap();
    let provenance = &parsed["participants"][0]["provenance"];
    assert_eq!(provenance["model"], "opus");
    assert_eq!(provenance["fallback"], json!(true));
    assert!(provenance["line"]
        .as_str()
        .unwrap()
        .contains("intake · opus via claude"));
    let notices = parsed["notices"].as_array().unwrap();
    assert_eq!(notices.len(), 2);
    assert_eq!(notices[0]["kind"], "capability-gap");
    assert_eq!(notices[1]["kind"], "fallback");

    // The page reads both from the model and composes neither.
    let page = handle(&db, "/").body;
    assert!(page.contains("part.provenance.line"));
    assert!(page.contains("view.notices"));
}
