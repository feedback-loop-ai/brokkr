use super::*;
use brokkr_core::envelope::EventType;
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
    assert!(page.body.contains("<title>brokkr</title>"));

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

    // Both spellings reach the same opener: the new name is what this
    // release documents, the old one answers for one release more
    // (decision 0019).
    std::env::set_var("BROKKR_BROWSER_BIN", "true");
    open_system_browser("http://127.0.0.1:9/");
    std::env::remove_var("BROKKR_BROWSER_BIN");
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
    assert_eq!(parsed["view_version"], brokkr_view::VIEW_VERSION);
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
    // carry and `brokkr-view` now owns.
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
            "ui.html must not derive: {banned} belongs in brokkr-view"
        );
    }
    // What it DOES carry: model consumption and geometry.
    for kept in ["/api/view/", "renderLoops", "svgEl", "textContent"] {
        assert!(PAGE.contains(kept), "the page still paints with {kept}");
    }
}

// -------------------------------------------------------- the road back
//
// A reforging is a road, and roads are drawn — the TUI's rail has drawn
// one since decision 0022, and the console drew none. There is no JS
// runtime in this Rust-only workspace, so what is proved here is the
// pair the surfaces share: the MODEL the console is served (asserted on
// committed journal fixtures, through the console's own route) and the
// rendering rules the page states about it (asserted on the served
// page's own source). Geometry beyond that is the browser's.

/// The reforging journal: `REVIEW-REFORGE` twice, then the exhausted
/// ruling ships. `implement` is entered three times and the road in is
/// `review`'s.
const REFORGED: &str = "reforging-the-road-back-hand-built";

/// The linear self-run: intake, implement, verify — no revisit anywhere.
const LINEAR: &str = "tui-graph-the-selection-box-gets-80f98deb";

/// A committed fixture replayed into a store the way the fleet reaches
/// any run — the export's own `(type, payload)` pairs re-appended — so
/// a console read meets the shape the engine recorded. The fixture file
/// is opened read-only and never edited.
fn replayed(name: &str) -> (tempfile::TempDir, PathBuf) {
    let ndjson = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(format!("fixtures/journals/{name}.ndjson")),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run(name, "fixture", "self", &json!({"files": {}}))
        .unwrap();
    for line in ndjson.lines().filter(|line| !line.trim().is_empty()) {
        let event: brokkr_core::envelope::EventEnvelope = serde_json::from_str(line).unwrap();
        store
            .append_next(name, event.event_type, event.payload.clone(), None, None)
            .unwrap();
    }
    (dir, db)
}

fn phases(db: &Path, run_id: &str) -> Vec<Value> {
    let view: Value =
        serde_json::from_str(&handle(db, &format!("/api/view/{run_id}")).body).unwrap();
    view["phases"].as_array().unwrap().clone()
}

#[test]
fn the_console_is_served_the_road_back_and_the_page_draws_it() {
    let (_dir, db) = replayed(REFORGED);
    let rail = phases(&db, REFORGED);
    let named = |name: &str| {
        rail.iter()
            .find(|phase| phase["name"] == name)
            .unwrap_or_else(|| panic!("the rail carries {name}"))
            .clone()
    };

    // The counts the ×N marker paints — three arrivals each on the
    // phases the loop crossed, one on the phase it never re-entered.
    assert_eq!(named("implement")["visits"], 3);
    assert_eq!(named("verify")["visits"], 3);
    assert_eq!(named("review")["visits"], 3);
    assert_eq!(named("intake")["visits"], 1);

    // And spelled the way every other surface spells it: `×N`, the
    // terminal's and `inspect`'s glyph, only past the first visit. The
    // operator's finding asked for the counts as the other surfaces
    // show them, so the spelling is part of the parity, not decoration.
    let page = handle(&db, "/").body;
    assert!(
        page.contains("svgEl('text', 'revisit', '×' + seg.visits)"),
        "the revisit marker wears the same × the TUI and inspect wear"
    );
    assert!(
        !page.contains("'x' + seg.visits"),
        "and never the ASCII x the console used to draw"
    );

    // And the road itself, already on the wire: the phase whose ruling
    // sent the run back, named on the phase it landed in. Two reforgings
    // were taken and ONE name is carried — the model deduped it, so a
    // surface that draws a road per name draws one road.
    assert_eq!(named("implement")["returns"], json!(["review"]));

    // The loop's other two legs are recorded as returns too — entering
    // `verify` from `implement` a second time IS a transition into a
    // phase already entered — and neither is a road the rail can draw:
    // a road is drawn LEFTWARD, and both of those departures lie left
    // of where they land. The page drops them on geometry, exactly as
    // the TUI's rail does, and one road is left.
    assert_eq!(named("verify")["returns"], json!(["implement"]));
    assert_eq!(named("review")["returns"], json!(["verify"]));
    let order: Vec<&str> = rail
        .iter()
        .map(|phase| phase["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        order,
        vec!["intake", "implement", "verify", "review", "ship", "done"]
    );
    let index = |name: &str| order.iter().position(|other| *other == name).unwrap();
    let leftward: Vec<(usize, usize)> = rail
        .iter()
        .enumerate()
        .flat_map(|(to, phase)| {
            phase["returns"]
                .as_array()
                .unwrap()
                .iter()
                .map(move |source| (to, index(source.as_str().unwrap())))
        })
        .filter(|(to, from)| from > to)
        .collect();
    assert_eq!(
        leftward,
        vec![(index("implement"), index("review"))],
        "one road: from beneath the phase that ruled to beneath the phase it landed in"
    );
    assert!(page.contains("function returnPairs(rail)"));
    assert!(page.contains("for (const source of phase.returns)"));
    assert!(page.contains("if (from > to) roads.push([to, from]);"));
    assert!(
        page.contains("const roads = returnPairs(rail);"),
        "the loops view asks the model for its roads"
    );
    // Solid, and headed at the LANDING only — the mirror of the rail's
    // own arrow, which marks arrival and never departure.
    assert!(page.contains("svgEl('path', 'ret')"));
    assert!(page.contains("svgEl('path', 'ret-tip')"));
    // Solid: the road was TAKEN, and a dashed road reads as one that
    // was not.
    assert!(page.contains(".loops .ret { fill: none; stroke: var(--line); stroke-width: 2; }"));
}

#[test]
fn a_run_that_never_went_back_renders_the_graph_it_did_before() {
    // The regression that matters most: the rendering is ADDITIVE and
    // conditional. On a real linear journal there is no road on the
    // wire, and every line of the page that could add one — the SVG's
    // own height, the arc elements, the legend — is gated on the same
    // empty list, so the graph is the graph it was before roads existed.
    let (_dir, db) = replayed(LINEAR);
    let rail = phases(&db, LINEAR);
    assert!(!rail.is_empty());
    assert!(
        rail.iter()
            .all(|phase| phase["returns"].as_array().unwrap().is_empty()),
        "the linear fixture recorded no backward transition: {rail:?}"
    );

    let page = handle(&db, "/").body;
    assert!(
        page.contains("svg.setAttribute('height', roads.length ? retY + 8 : nameY + 24);"),
        "no road, no rows: the height is the height it was"
    );
    assert!(
        page.contains("+ (roads.length ? ' · the arc under the rail is a reforging"),
        "and the legend gains its clause only when a road is drawn"
    );
    // Exactly one place appends arc elements, and it iterates the pairs:
    // an empty list appends nothing at all.
    assert_eq!(
        page.matches("roads.forEach").count(),
        1,
        "one drawing site, iterating the roads"
    );
    for arc in ["svgEl('path', 'ret')", "svgEl('path', 'ret-tip')"] {
        assert_eq!(page.matches(arc).count(), 1, "{arc} is drawn in one place");
    }
}

#[test]
fn the_road_back_is_never_inferred_from_a_repeated_visit() {
    // `visits` says a phase was entered twice; only the transition says
    // where from. The page's own non-inference, mirroring the TUI's: no
    // line that reads a visit count also reaches for a road, and the
    // pairing drops what has no geometry — a departure naming no phase
    // on this rail, and a landing that does not lie left of it.
    for line in PAGE.lines().filter(|line| line.contains("visits")) {
        assert!(
            !line.contains("roads") && !line.contains("returnPairs"),
            "a road drawn from a visit count: {line}"
        );
    }
    for line in PAGE.lines().filter(|line| line.contains("roads.push")) {
        assert!(
            !line.contains("visits"),
            "a road pushed from a visit count: {line}"
        );
    }
    // `findIndex` returns -1 for a departure the rail never drew, which
    // the same `from > to` test drops — one rule, both arms.
    assert!(PAGE.contains("const from = rail.findIndex((other) => other.name === source);"));
}

/// Decision 0019: the console wears the new name and keeps the motto.
/// The wordmark is BROKKR with no dimmed prefix, the title matches, and
/// the tagline — the product's motto, never the old product name —
/// survives the rename untouched. Law 4 of ruling 6 holds: the console
/// gains no myth text beyond the wordmark.
#[test]
fn the_console_wears_the_brokkr_wordmark_and_keeps_the_motto() {
    assert!(
        PAGE.contains("<title>brokkr</title>"),
        "the tab says brokkr"
    );
    assert!(
        PAGE.contains(r#"<span class="word">BROKKR</span>"#),
        "the wordmark is BROKKR, with no prefix element"
    );
    assert!(
        PAGE.contains(r#"<span class="tag">&gt; the machine is the outer loop_</span>"#),
        "the motto survives the rename"
    );
    for retired in ["the_", "FORGE", "the-forge", "class=\"the\""] {
        assert!(
            !PAGE.contains(retired),
            "no user-facing surface still says {retired}"
        );
    }
    for myth in ["Mjölnir", "Sindri", "Loki", "Muninn", "Edda"] {
        assert!(!PAGE.contains(myth), "law 4: the lore stays off the page");
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
    let _home = crate::tests::HOME
        .lock()
        .unwrap_or_else(|error| error.into_inner());
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

/// The whole liveness rule of the transcript watch, as a predicate with
/// its own test rather than a comparison buried in a poll loop. Prose is
/// only ever appended, so length is the signal — and the FIRST look is
/// never growth, because the reader already holds the file as it stood.
#[test]
fn transcript_growth_is_length_and_the_first_look_is_never_growth() {
    assert!(!transcript_grew(None, 0), "the first look at an empty file");
    assert!(!transcript_grew(None, 8_192), "nor at a long one");
    assert!(transcript_grew(Some(10), 11), "one more byte is new prose");
    assert!(!transcript_grew(Some(10), 10), "an unchanged file is quiet");
    assert!(
        !transcript_grew(Some(10), 3),
        "a file that shrank was replaced, not appended to"
    );

    // The length itself: what is there, and 0 for what is not — a
    // transcript that vanished mid-watch has not grown.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("t.jsonl");
    std::fs::write(&file, "1234567890").unwrap();
    assert_eq!(transcript_len(&file), 10);
    assert_eq!(transcript_len(&dir.path().join("gone.jsonl")), 0);
}

/// A drilled seat's prose lands BETWEEN journal checkpoints, so the
/// transcript gets a stream of its own: an event when the file grew, a
/// heartbeat when it did not, and — before a single byte of stream — a
/// clean 404 for an id that cannot name a transcript, never a
/// connection left open waiting for a file to appear.
#[test]
fn the_session_stream_fires_on_growth_and_says_nothing_otherwise() {
    let _home = crate::tests::HOME
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let previous_home = std::env::var_os("HOME");
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let projects = home.join(".claude").join("projects").join("live-project");
    std::fs::create_dir_all(&projects).unwrap();
    let file = projects.join("abcd-1234.jsonl");
    let turn = "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\
                \"content\":\"a word\"}}\n";
    std::fs::write(&file, turn).unwrap();
    std::env::set_var("HOME", &home);

    // The same guard the drill applies, on the same id, before any path
    // is formed — and a valid id with no transcript behind it reads the
    // same way to the operator: nothing to watch.
    let missing_db = PathBuf::from("missing.db");
    for bad in ["", "../../etc/passwd", &"a".repeat(65), "9999-9999"] {
        let response = exchange(
            missing_db.clone(),
            &format!("GET /sse/session/{bad} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert!(
            response.starts_with("HTTP/1.1 404 Not Found"),
            "id {bad:?}: {response}"
        );
        assert!(response.contains("transcript not found"), "id {bad:?}");
    }

    // Three polls of a real stream, with the file mutated between them
    // by the writer itself so the timing is the test's, not the clock's.
    let mut request = std::io::Cursor::new(
        b"GET /sse/session/abcd-1234 HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec(),
    );
    let mut watcher = GrowsThenVanishes {
        file: file.clone(),
        writes: 0,
        out: String::new(),
    };
    serve_io(&missing_db, &mut request, &mut watcher, Some(3));
    let stream = watcher.out;
    assert!(stream.starts_with("HTTP/1.1 200 OK"), "{stream}");
    assert!(
        stream.contains("Content-Type: text/event-stream"),
        "{stream}"
    );
    assert_eq!(
        stream.matches("data: ").count(),
        1,
        "one append, one event — and none for the first look or the \
         poll that found the file gone: {stream}"
    );
    assert_eq!(
        stream.matches(": ping").count(),
        2,
        "the polls that saw no new prose are heartbeats: {stream}"
    );

    // A client that has already gone gets no stream: the header write
    // failing and the first message failing are both a clean return,
    // never a thread left polling a socket nobody is reading.
    std::fs::write(&file, turn).unwrap();
    let mut request = std::io::Cursor::new(
        b"GET /sse/session/abcd-1234 HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec(),
    );
    serve_io(&missing_db, &mut request, &mut BrokenWriter, Some(1));
    let mut request = std::io::Cursor::new(
        b"GET /sse/session/abcd-1234 HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec(),
    );
    serve_io(
        &missing_db,
        &mut request,
        &mut FailAfterOneWrite::default(),
        Some(1),
    );

    if let Some(previous_home) = previous_home {
        std::env::set_var("HOME", previous_home);
    } else {
        std::env::remove_var("HOME");
    }
}

/// The stream's clock, driven from the writer: the transcript gains a
/// turn while the first heartbeat is on the wire, and vanishes while the
/// growth event is. Both are things a live seat's file actually does.
struct GrowsThenVanishes {
    file: PathBuf,
    writes: usize,
    out: String,
}

impl Write for GrowsThenVanishes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.out.push_str(&String::from_utf8_lossy(bytes));
        self.writes += 1;
        match self.writes {
            // 1 is the header. 2 is the first poll's heartbeat — new
            // prose lands before the next one.
            2 => {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&self.file)
                    .unwrap();
                file.write_all(b"{\"type\":\"user\",\"message\":{\"content\":\"more\"}}\n")
                    .unwrap();
            }
            // 3 is the growth event. Then the file goes away.
            3 => std::fs::remove_file(&self.file).unwrap(),
            _ => {}
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The page's half of the same rule: it subscribes to that one session,
/// drops that one cache entry, and closes what it opened.
#[test]
fn the_page_watches_one_working_sessions_prose_and_closes_what_it_opens() {
    for kept in [
        "/sse/session/",
        "transcriptCache.delete",
        "closeSessionWatch",
        "part.status === 'working'",
    ] {
        assert!(PAGE.contains(kept), "the page streams prose with {kept}");
    }
    // One cache entry at a time: a full clear would refetch every
    // transcript the operator has already read.
    assert!(!PAGE.contains("transcriptCache.clear"));
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
        .contains("intake · selected opus via claude"));
    let notices = parsed["notices"].as_array().unwrap();
    assert_eq!(notices.len(), 2);
    assert_eq!(notices[0]["kind"], "capability-gap");
    assert_eq!(notices[1]["kind"], "fallback");

    // The page reads both from the model and composes neither.
    let page = handle(&db, "/").body;
    assert!(page.contains("part.provenance.line"));
    assert!(page.contains("view.notices"));
}

/// Decision 0046 ruling 3's fourth surface: the console is served the
/// boundary cell beside every model cell and the run-level text, and
/// the page paints both cells into one row through its one pair helper
/// — computing nothing of its own (design DD12).
#[test]
fn the_console_serves_the_boundary_and_paints_the_pair() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run(
            "r1",
            "feat",
            "self",
            &json!({"files": {}, "engine": "0.9.0"}),
        )
        .unwrap();
    let mut append = |kind, payload| {
        store.append_next("r1", kind, payload, None, None).unwrap();
    };
    append(
        EventType::RunStarted,
        json!({"feature": "feat", "manifest": {"engine": "0.9.0",
               "hands": {"verify": {"binds": []}}}}),
    );
    append(EventType::PhaseEntered, json!({"phase": "verify"}));
    append(
        EventType::EffectRequested,
        json!({"effect_id": "e1", "seat": "verify", "phase": "verify",
               "idempotency_key": "k", "input_digest": "d"}),
    );
    append(
        EventType::EffectStarted,
        json!({"effect_id": "e1", "attempt_id": "a1", "driver": "d",
               "boundary": [{"member": null, "boundary": "harness", "gate": true}]}),
    );
    append(
        EventType::EffectCheckpointed,
        json!({"effect_id": "e1", "attempt_id": "a1", "checkpoint": {
            "step": "seat-turn", "turn": 1, "model": "claude-fable-5-1",
            "boundary": "harness"}}),
    );
    append(
        EventType::EffectSucceeded,
        json!({"effect_id": "e1", "attempt_id": "a1", "result": {
            "result": "pass", "model": "claude-fable-5-1", "boundary": "harness"}}),
    );

    let view = handle(&db, "/api/view/r1");
    let parsed: Value = serde_json::from_str(&view.body).unwrap();
    assert_eq!(parsed["view_version"], 9);
    let seat = &parsed["participants"][0];
    assert_eq!(seat["model"]["text"], "claude-fable-5-1");
    assert_eq!(seat["boundary"]["text"], "harness");
    assert_eq!(seat["checkpoints"][0]["boundary"]["text"], "harness");
    assert_eq!(
        parsed["phases"][0]["columns"][0]["nodes"][0]["boundary"]["text"],
        "harness"
    );
    assert_eq!(parsed["journal"][4]["model"]["text"], "claude-fable-5-1");
    assert_eq!(parsed["journal"][4]["boundary"]["text"], "harness");
    assert_eq!(parsed["journal"][0]["boundary"]["absent"], json!(true));
    assert_eq!(parsed["boundary"]["word"]["text"], "harness");
    assert_eq!(parsed["boundary"]["unboxed"], json!(true));
    assert_eq!(parsed["boundary"]["text"], "harness · unboxed");

    for (id, hands, entry, expected) in [
        (
            "boxed",
            true,
            json!([{"member":null,"boundary":"namespace","gate":true}]),
            "namespace",
        ),
        (
            "work",
            true,
            json!([{"member":null,"boundary":"harness","gate":false}]),
            "harness",
        ),
        ("old", true, Value::Null, "not recorded"),
        ("plain", false, Value::Null, ""),
    ] {
        let mut manifest = json!({"engine":"0.9.0","files":{}});
        if hands {
            manifest["hands"] = json!({"verify":{"binds":[]}});
        }
        store.create_run(id, "feat", "self", &manifest).unwrap();
        store
            .append_next(
                id,
                EventType::RunStarted,
                json!({"feature":"feat","manifest":manifest}),
                None,
                None,
            )
            .unwrap();
        store.append_next(id, EventType::EffectRequested, json!({"effect_id":"e","seat":"verify","phase":"verify","idempotency_key":"k","input_digest":"d"}), None, None).unwrap();
        let mut payload = json!({"effect_id":"e","attempt_id":"a","driver":"d"});
        if !entry.is_null() {
            payload["boundary"] = entry;
        }
        store
            .append_next(id, EventType::EffectStarted, payload, None, None)
            .unwrap();
        let response = handle(&db, &format!("/api/view/{id}"));
        let served: Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(served["boundary"]["text"], expected, "{id}: {served}");
    }

    // The page: one pair helper, the only `.model` read; the run header
    // prints the served text; and each of the four surfaces places the
    // two cells in one row.
    let page = handle(&db, "/").body;
    assert!(page.contains("function served(carrier)"), "the pair helper");
    assert!(page.contains("return [carrier.model, carrier.boundary];"));
    assert_eq!(
        page.lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .filter(|line| line.contains(".model"))
            .count(),
        1,
        "the helper is the only place the page names .model"
    );
    assert!(page.contains("view.boundary.text"), "the run header");
    assert!(!page.contains("unboxed"), "the adjective is the model's");
    assert_eq!(
        page.matches("cell(model), cell(boundary)").count(),
        4,
        "participants, checkpoints, trail and raw journal rows"
    );
    assert!(page.contains("'model ' + model.text + ' · boundary ' + boundary.text"));
    assert_eq!(page.matches("el('th', null, 'boundary')").count(), 4);
}
