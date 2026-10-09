use super::*;
use crate::tests::env_guard::EnvGuard;
use brokkr_core::{envelope::EventType, Status};
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

pub(super) fn exchange(db: PathBuf, request: &str, sse_limit: Option<usize>) -> String {
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

pub(super) struct BrokenWriter;

impl Write for BrokenWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken response"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct FailAfterOneWrite {
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
        assert_eq!(status.as_str(), text);
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
    // A journal that cannot list its runs is refused, never an empty
    // fleet (#377).
    let refused = handle(&malformed_schema, "/api/runs");
    assert_eq!(refused.status, "500 Internal Server Error");
    assert_eq!(
        refused.body,
        json!({"error": "sqlite: no such column: feature in SELECT run_id, feature, \
                         created_at FROM runs ORDER BY created_at at offset 15"})
        .to_string()
    );

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

    let mut env = EnvGuard::lock();
    env.set("BROKKR_BROWSER_BIN", "true");
    open_system_browser("http://127.0.0.1:9/");
}

/// A browser pinned by the retired spelling alone, or by a value that is
/// not UTF-8, opens nothing: neither the pin nor `xdg-open` in its place
/// (#355).
#[test]
fn a_browser_override_that_cannot_be_read_is_refused_by_name() {
    use std::os::unix::ffi::OsStrExt;
    let retired = concat!("FOR", "GE_BROWSER_BIN");
    let mut env = EnvGuard::lock();
    env.remove("BROKKR_BROWSER_BIN");
    env.set(retired, "true");
    let mut opened = Vec::new();
    let mut open = || open_browser_with("http://x/", |p, _| opened.push(p.to_string()));
    let refused = OverrideError::Retired {
        retired: retired.to_string(),
        current: "BROKKR_BROWSER_BIN",
    };
    assert_eq!(browser_program(), Err(refused));
    open();
    env.set(
        "BROKKR_BROWSER_BIN",
        std::ffi::OsStr::from_bytes(b"/opt/\xff"),
    );
    let variable = "BROKKR_BROWSER_BIN";
    assert_eq!(
        browser_program(),
        Err(OverrideError::NotUnicode { variable })
    );
    open();
    env.set("BROKKR_BROWSER_BIN", "true");
    open();
    env.remove("BROKKR_BROWSER_BIN");
    env.remove(retired);
    open();
    assert_eq!(opened, ["true", "xdg-open"]);
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

#[test]
fn the_page_strips_directional_formatting_at_the_display_boundary() {
    // L3: the browser is a display boundary too. Its text nodes are
    // inert, but bidi/directional formatting characters reorder visible
    // text, so the page removes them without touching machine JSON.
    assert!(
        PAGE.contains("\\u061C"),
        "the Arabic letter mark is named in the browser sanitizer"
    );
    assert!(
        PAGE.contains("function displayText"),
        "one browser display sanitizer exists"
    );
    assert!(
        PAGE.contains("node.textContent = displayText(text)"),
        "el() and svgEl() route their text through it"
    );
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

/// The participant's transcript route, with its key encoded once.
pub(super) fn transcript_route(db: &Path, key: &str) -> Response {
    handle(db, &format!("/api/transcript/r1/{}", percent_encode(key)))
}

/// A participant recording Claude session `abcd-1234` under the canonical
/// form of `projects`.
fn claude_participant_under(projects: &Path) -> (tempfile::TempDir, PathBuf, String) {
    let recorded = std::fs::canonicalize(projects).unwrap();
    let reference = claude_reference("abcd-1234", recorded.to_str().unwrap());
    participant_fixture(Some(reference), None, None)
}

/// The participant's presentation, which answers for any known key.
fn presentation_route(db: &Path, key: &str) -> Value {
    let response = handle(db, &format!("/api/presentation/r1/{}", percent_encode(key)));
    assert_eq!(response.status, "200 OK", "{}", response.body);
    serde_json::from_str(&response.body).unwrap()
}

/// One participant stream request line, keyed like the body route.
pub(super) fn watch_request(route: &str) -> String {
    format!("GET /sse/transcript/{route} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
}

/// The page's half of the same rule: the controller keys the watch to the
/// active participant and owns the exact handle it opened.
#[test]
fn the_page_watches_one_working_sessions_prose_and_closes_what_it_opens() {
    for kept in [
        "participantPath('/sse/transcript/', subject)",
        "working: part.working === true",
        "source.close(); handlers.close()",
    ] {
        assert!(PAGE.contains(kept), "the page streams prose with {kept}");
    }
    // No separate cache survives across intervals: the controller's own
    // body state is the only cache, and a key change clears it.
    assert!(!PAGE.contains("transcriptCache"));
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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
    assert_eq!(parsed["view_version"], 17);
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

// =========================================================================
// The browser participant controller (D9/D11). The exact marker-delimited
// controller bytes served from `PAGE` are extracted and evaluated with the
// pinned, default-feature-free dev-only Boa engine; deterministic promises,
// identity-bearing watches, timers and paint/clear effects drive the traces.

const CONTROLLER_START: &str = "/* transcript-controller:start */";
const CONTROLLER_END: &str = "/* transcript-controller:end */";

fn controller_block() -> String {
    let start = PAGE
        .find(CONTROLLER_START)
        .expect("the served page marks the controller start")
        + CONTROLLER_START.len();
    let end = PAGE[start..]
        .find(CONTROLLER_END)
        .expect("the served page marks the controller end")
        + start;
    PAGE[start..end].to_string()
}

const DRIVER: &str = r#"
var __trace = [];
var __pres = [];
var __body = [];
var __watches = [];
var __timers = [];
var __controller = null;
function __t(kind, detail) {
  __trace.push(detail === undefined ? kind : kind + ' ' + detail);
}
function __newController() {
  __trace = [];
  __pres = [];
  __body = [];
  __watches = [];
  __timers = [];
  __controller = createTranscriptController({
    requestPresentation: function (subject) {
      __t('presentation', subject.participantKey);
      return new Promise(function (resolve, reject) {
        __pres.push({ resolve: resolve, reject: reject });
      });
    },
    requestBody: function (id) {
      __t('body', id);
      return new Promise(function (resolve, reject) {
        __body.push({ resolve: resolve, reject: reject });
      });
    },
    openWatch: function (id, subject, handlers) {
      var handle = { id: id, token: __watches.length, closed: false };
      __watches.push({ handle: handle, handlers: handlers });
      __t('open', id + '#' + handle.token);
      return handle;
    },
    closeWatch: function (handle) {
      if (handle) handle.closed = true;
      __t('close', handle ? handle.id + '#' + handle.token : '');
    },
    schedule: function (callback) {
      var timer = { callback: callback, active: true };
      __timers.push(timer);
      __t('timer');
      return timer;
    },
    cancel: function (timer) {
      if (timer) timer.active = false;
      __t('cancel');
    },
    clear: function () { __t('clear'); },
    render: function (view) {
      __t('render',
        view.phase + ' ' + (view.sessionId === null ? '-' : view.sessionId)
        + ' ' + (view.reason === null ? '-' : view.reason));
    }
  });
  return true;
}
function __operatorSelect(subject) { __controller.operator_select(subject); }
function __sync(subject) { __controller.sync(subject); }
function __repaint() { __controller.repaint(); }
function __clearController() { __controller.clear(); }
function __resolvePresentation(presentation) { __pres.shift().resolve(presentation); }
function __rejectPresentation() { __pres.shift().reject(new Error('presentation')); }
function __resolveBody(body) { __body.shift().resolve(body); }
function __resolveBodyAt(index, body) { __body.splice(index, 1)[0].resolve(body); }
function __rejectBody() { __body.shift().reject(new Error('body')); }
function __tick() { __timers[__timers.length - 1].callback(); }
function __grow(index) { __watches[index].handlers.growth(); }
function __watchClose(index) { __watches[index].handlers.close(); }
function __traceText() { return JSON.stringify(__trace); }
function __snapshotText() { return JSON.stringify(__controller.snapshot()); }
"#;

struct Boa {
    context: boa_engine::Context,
}

impl Boa {
    fn boot() -> Boa {
        let mut context = boa_engine::Context::default();
        let block = controller_block();
        context
            .eval(boa_engine::Source::from_bytes(block.as_bytes()))
            .expect("the served controller block evaluates");
        context
            .eval(boa_engine::Source::from_bytes(DRIVER.as_bytes()))
            .expect("the deterministic driver evaluates");
        let mut harness = Boa { context };
        harness.call("__newController()");
        harness
    }

    fn call(&mut self, source: &str) -> String {
        let value = self
            .context
            .eval(boa_engine::Source::from_bytes(source.as_bytes()))
            .unwrap_or_else(|error| panic!("evaluating {source}: {error}"));
        self.context.run_jobs().expect("queued promise jobs drain");
        value
            .as_string()
            .map(|text| text.to_std_string_escaped())
            .unwrap_or_default()
    }

    fn select(&mut self, subject: &Value) {
        self.call(&format!("__operatorSelect({subject})"));
    }
    fn sync(&mut self, subject: &Value) {
        self.call(&format!("__sync({subject})"));
    }
    fn resolve_presentation(&mut self, presentation: &Value) {
        self.call(&format!("__resolvePresentation({presentation})"));
    }
    fn reject_presentation(&mut self) {
        self.call("__rejectPresentation()");
    }
    fn resolve_body(&mut self, body: &Value) {
        self.call(&format!("__resolveBody({body})"));
    }
    fn resolve_body_at(&mut self, index: usize, body: &Value) {
        self.call(&format!("__resolveBodyAt({index}, {body})"));
    }
    fn reject_body(&mut self) {
        self.call("__rejectBody()");
    }
    fn tick(&mut self) {
        self.call("__tick()");
    }
    fn grow(&mut self, index: usize) {
        self.call(&format!("__grow({index})"));
    }
    fn close_watch(&mut self, index: usize) {
        self.call(&format!("__watchClose({index})"));
    }
    fn trace(&mut self) -> Vec<String> {
        serde_json::from_str(&self.call("__traceText()")).expect("the effect trace parses")
    }
    fn state(&mut self) -> Value {
        serde_json::from_str(&self.call("__snapshotText()")).expect("the snapshot parses")
    }
}

fn subject(
    run_id: &str,
    participant_key: &str,
    reference: Option<Value>,
    session_id: Option<&str>,
    provider: Option<&str>,
    working: bool,
) -> Value {
    json!({
        "runId": run_id,
        "participantKey": participant_key,
        "reference": reference,
        "sessionId": session_id,
        "provider": provider,
        "working": working,
    })
}

fn claude_reference(id: &str, home: &str) -> Value {
    json!({"kind": "claude-session", "locator": id, "home": home})
}

fn presentation(
    reference: Value,
    admitted: bool,
    reason: Option<&str>,
    hint: Option<&str>,
    drill_eligible: bool,
) -> Value {
    json!({
        "reference": reference,
        "legacy": false,
        "admitted": admitted,
        "reason": reason,
        "explanation": reason.map(|_| "explained"),
        "path": if admitted { json!("/confirmed") } else { Value::Null },
        "hint": hint,
        "drill_eligible": drill_eligible,
    })
}

fn text_body(text: &str) -> Value {
    json!({"session_id": "abcd-1234", "turns": [
        {"role": "assistant", "ts": "", "blocks": [{"kind": "text", "text": text}]}
    ], "truncated": false})
}

#[test]
fn the_served_controller_block_evaluates_in_boa() {
    let mut boa = Boa::boot();
    assert_eq!(boa.call("'ok'"), "ok");
    let state = boa.state();
    assert_eq!(state["active"], Value::Null);
    assert_eq!(state["body"], "missing");
    assert_eq!(state["bodies"], 0);
}

/// The production adapter is thin and lives outside the block; the block
/// reaches no ambient browser global and is extracted from the exact bytes.
#[test]
fn the_controller_is_isolated_and_the_adapter_is_thin() {
    assert_eq!(
        PAGE.matches(CONTROLLER_START).count(),
        1,
        "exactly one served controller block"
    );
    assert_eq!(PAGE.matches(CONTROLLER_END).count(), 1);
    assert_eq!(
        PAGE.matches("= createTranscriptController(").count(),
        1,
        "the adapter constructs the controller exactly once"
    );
    assert!(PAGE.contains("encodeURIComponent(subject.runId)"));
    assert!(PAGE.contains("encodeURIComponent(subject.participantKey)"));
    assert!(PAGE.contains("fetchJson(participantPath('/api/transcript/', subject))"));
    assert!(PAGE.contains("fetchJson(participantPath('/api/presentation/', subject))"));
    assert_eq!(
        PAGE.matches("{ cache: 'no-store' }").count(),
        1,
        "presentation and body fetches share the one no-store fetch"
    );
    assert_eq!(
        PAGE.matches("new EventSource(").count(),
        2,
        "the run stream and the controller's watch each own one EventSource"
    );
    assert!(PAGE.contains("source.onerror = () => { source.close(); handlers.close(); };"));
    assert_eq!(
        PAGE.matches("setInterval(").count(),
        2,
        "the runs poll and the controller's re-check each own one timer"
    );
    assert!(PAGE.contains("clearInterval(handle)"));
    assert!(PAGE.contains("transcriptBox.replaceChildren()"));
    assert!(PAGE.contains("/^[0-9a-fA-F][0-9a-fA-F-]{0,63}$/"));
    for retired in [
        "transcriptCache",
        "closeSessionWatch",
        "held by ",
        "— resume the session for the rest",
        "— claude --resume carries the rest",
    ] {
        assert!(
            !PAGE.contains(retired),
            "the page still carries {retired:?}"
        );
    }
    let block = controller_block();
    assert!(block.contains("function createTranscriptController(effects)"));
    assert!(
        block.contains("function operator_select(subject)"),
        "the controller exposes the explicit edge-triggered selection"
    );
    assert!(
        PAGE.contains("transcriptController.operator_select(subject)"),
        "the adapter routes explicit selection through operator_select"
    );
    assert!(
        PAGE.contains("explicitSelectKey = part.key;"),
        "the seat row marks an explicit selection"
    );
    assert!(
        PAGE.contains("explicitSelectKey = n.key;"),
        "the graph node marks an explicit selection instead of background sync"
    );
    assert!(
        PAGE.contains("'local reference · '"),
        "the page paints the authoritative common reference as its own fact"
    );
    for banned in [
        "document",
        "window",
        "fetch",
        "EventSource",
        "setInterval",
        "setTimeout",
        "clearInterval",
        "localStorage",
        "XMLHttpRequest",
        "JSON",
    ] {
        assert!(
            !block.contains(banned),
            "the controller block reaches the ambient {banned}"
        );
    }
}

/// The pinned Boa engine is a dev-only edge: the release dependency tree
/// excludes it by construction.
#[test]
fn the_test_engine_is_pinned_and_dev_only() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let (production, dev) = manifest
        .split_once("[dev-dependencies]")
        .expect("the manifest separates dev-dependencies");
    assert!(
        !production.contains("boa_engine"),
        "Boa must never be a production dependency"
    );
    assert!(
        dev.contains("boa_engine = { version = \"=0.21.1\", default-features = false }"),
        "Boa is pinned exactly with default features disabled"
    );
}

/// Codex and DSH drill like Claude (#352): an admitted, eligible source of
/// either kind fetches its body and opens its watch, and an ineligible one
/// drills nothing.
#[test]
fn admitted_sources_of_every_kind_drill_only_when_eligible() {
    for (kind, locator) in [
        ("codex-thread", "0199mine"),
        ("dsh-session", "sessions/one"),
    ] {
        let reference = json!({"kind": kind, "locator": locator, "home": "/retained"});
        let seat = subject(
            "r1",
            "seat",
            Some(reference.clone()),
            None,
            Some("codex"),
            true,
        );
        let pres = presentation(reference.clone(), true, None, Some("shared hint"), false);
        let mut boa = Boa::boot();
        boa.select(&seat);
        boa.resolve_presentation(&pres);
        boa.tick();
        boa.resolve_presentation(&pres);
        boa.tick();
        boa.resolve_presentation(&pres);
        let trace = boa.trace();
        assert!(
            !trace.iter().any(|entry| entry.starts_with("body ")),
            "{kind} drove a body request: {trace:?}"
        );
        assert!(
            !trace.iter().any(|entry| entry.starts_with("open ")),
            "{kind} opened a watch: {trace:?}"
        );
        let state = boa.state();
        assert_eq!(state["admitted"], true);
        assert_eq!(state["drillEligible"], false);
        assert_eq!(state["bodies"], 0);
        assert_eq!(state["opens"], 0);

        let eligible = presentation(reference, true, None, Some("shared hint"), true);
        let mut boa = Boa::boot();
        boa.select(&seat);
        boa.resolve_presentation(&eligible);
        boa.resolve_body(&text_body("prose"));
        let trace = boa.trace();
        assert!(
            trace.contains(&format!("body {locator}")),
            "{kind}: {trace:?}"
        );
        assert!(
            trace.contains(&format!("open {locator}#0")),
            "{kind}: {trace:?}"
        );
        let state = boa.state();
        assert_eq!(
            (&state["body"], &state["sessionId"]),
            (&json!("succeeded"), &json!(locator))
        );
    }

    // The page drills on the server's word alone: an admitted presentation
    // the server does not call eligible drills nothing.
    let reference = claude_reference("abcd-1234", "/retained/claude");
    let pres = presentation(
        reference.clone(),
        true,
        None,
        Some("full session: claude --resume abcd-1234"),
        false,
    );
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres);
    boa.tick();
    boa.resolve_presentation(&pres);
    let trace = boa.trace();
    assert!(
        !trace.iter().any(|entry| entry.starts_with("body ")),
        "{trace:?}"
    );
    assert!(
        !trace.iter().any(|entry| entry.starts_with("open ")),
        "{trace:?}"
    );
}

#[test]
fn a_concluded_zero_turn_body_is_fetched_once() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(
        reference.clone(),
        true,
        None,
        Some("full session: claude --resume abcd-1234"),
        true,
    );
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        false,
    ));
    boa.resolve_presentation(&pres);
    boa.resolve_body(&json!({"session_id": "abcd-1234", "turns": [], "truncated": false}));
    boa.tick();
    boa.resolve_presentation(&pres);
    boa.tick();
    boa.resolve_presentation(&pres);
    let state = boa.state();
    assert_eq!(state["body"], "succeeded");
    assert_eq!(
        state["bodies"], 1,
        "an empty success is received, not re-requested"
    );
    assert_eq!(state["opens"], 0, "a concluded participant watches nothing");
    let trace = boa.trace();
    assert_eq!(
        trace
            .iter()
            .filter(|entry| entry.starts_with("body "))
            .count(),
        1
    );
}

#[test]
fn a_persistently_unreadable_body_costs_one_request_per_interval() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        false,
    ));
    boa.resolve_presentation(&pres);
    boa.reject_body();
    boa.resolve_presentation(&pres);
    assert_eq!(boa.state()["body"], "refused");
    for _ in 0..2 {
        boa.tick();
        boa.resolve_presentation(&pres);
        boa.reject_body();
        boa.resolve_presentation(&pres);
    }
    let state = boa.state();
    assert_eq!(state["bodies"], 3, "one refused request per interval");
    assert_eq!(state["opens"], 0);
}

#[test]
fn discovery_refusals_close_admission_everywhere() {
    for (kind, locator, home) in [
        ("dsh-session", "sessions/one", "/retained"),
        ("claude-session", "abcd-1234", "/retained/claude"),
        ("codex-thread", "0199mine", "/retained"),
    ] {
        let reference = json!({"kind": kind, "locator": locator, "home": home});
        let refused = presentation(reference.clone(), false, Some("unreadable"), None, false);
        let mut boa = Boa::boot();
        boa.select(&subject(
            "r1",
            "seat",
            Some(reference),
            None,
            Some("codex"),
            true,
        ));
        boa.resolve_presentation(&refused);
        boa.tick();
        boa.resolve_presentation(&refused);
        boa.tick();
        boa.resolve_presentation(&refused);
        let state = boa.state();
        assert_eq!(state["admitted"], false);
        assert_eq!(state["reason"], "unreadable");
        assert_eq!(state["explanation"], "explained");
        assert_eq!(state["path"], Value::Null);
        assert_eq!(
            state["hint"],
            Value::Null,
            "a discovery refusal has no path hint"
        );
        assert_eq!(state["sessionId"], Value::Null, "no session label to drill");
        assert_eq!(state["body"], "missing");
        assert_eq!(state["bodies"], 0);
        assert_eq!(state["opens"], 0);
        let trace = boa.trace();
        assert_eq!(
            trace
                .iter()
                .filter(|entry| entry.starts_with("presentation "))
                .count(),
            3,
            "each interval performs only fresh presentation discovery: {trace:?}"
        );
    }
}

#[test]
fn a_recheck_first_interval_opens_one_watch() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut subject = subject("r1", "seat", Some(reference), None, Some("claude"), false);
    let mut boa = Boa::boot();
    boa.select(&subject);
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["opens"], 0, "a concluded seat watches nothing");

    // The seat starts working without a key change; the next recurring
    // re-check spends its restored budget on the missing watch.
    subject["working"] = json!(true);
    boa.sync(&subject);
    boa.tick();
    boa.resolve_presentation(&pres);
    assert_eq!(boa.state()["opens"], 1);
    assert_eq!(boa.state()["watch"], true);

    boa.tick();
    boa.resolve_presentation(&pres);
    boa.tick();
    boa.resolve_presentation(&pres);
    let state = boa.state();
    assert_eq!(state["opens"], 1, "one automatic opening per interval");
    assert_eq!(state["watch"], true);
}

#[test]
fn a_closure_first_interval_and_a_second_immediate_closure() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["opens"], 1);

    // The watch closes at once; the interval's single budget is spent, so
    // the fresh presentation repaints from a fresh body without a watch.
    boa.close_watch(0);
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("second"));
    let state = boa.state();
    assert_eq!(
        state["opens"], 1,
        "no second watch before the next re-check"
    );
    assert_eq!(state["watch"], false);
    assert_eq!(state["body"], "succeeded");

    // The next re-check restores the budget and opens exactly one watch.
    boa.tick();
    boa.resolve_presentation(&pres);
    assert_eq!(boa.state()["opens"], 2);
    assert_eq!(boa.state()["watch"], true);

    // A second immediate closure again repaints from a fresh body and
    // leaves the watch closed until the following re-check.
    boa.close_watch(1);
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("third"));
    let state = boa.state();
    assert_eq!(state["opens"], 2);
    assert_eq!(state["watch"], false);
    assert_eq!(state["body"], "succeeded");
}

#[test]
fn admission_loss_rejects_late_callbacks_and_then_recovers() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let lost = presentation(
        reference.clone(),
        false,
        Some("not-found"),
        Some("hint"),
        false,
    );
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["watch"], true);

    boa.tick();
    boa.resolve_presentation(&lost);
    let state = boa.state();
    assert_eq!(state["admitted"], false);
    assert_eq!(state["reason"], "not-found");
    assert_eq!(state["watch"], false);
    assert_eq!(
        state["body"], "missing",
        "admission loss discards the prose"
    );

    // A late growth or close callback from the closed handle is inert.
    let before = boa.trace().len();
    boa.grow(0);
    boa.close_watch(0);
    assert_eq!(
        boa.trace().len(),
        before,
        "a stale watch callback repainted"
    );

    // The next re-check admits again and restores body and watch.
    boa.tick();
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("recovered"));
    let state = boa.state();
    assert_eq!(state["admitted"], true);
    assert_eq!(state["body"], "succeeded");
    assert_eq!(state["watch"], true);
    assert_eq!(state["opens"], 2);
}

#[test]
fn an_identity_change_resets_only_the_new_key() {
    let first = claude_reference("aaaa-1111", "/local/projects");
    let second = claude_reference("bbbb-2222", "/local/projects");
    let pres_first = presentation(first.clone(), true, None, Some("hint-a"), true);
    let pres_second = presentation(second.clone(), true, None, Some("hint-b"), true);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat-a",
        Some(first),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres_first);
    boa.resolve_body(&text_body("a"));
    assert_eq!(boa.state()["watch"], true);

    boa.select(&subject(
        "r1",
        "seat-b",
        Some(second),
        None,
        Some("claude"),
        true,
    ));
    let state = boa.state();
    assert_eq!(state["watch"], false, "the old watch is closed");
    assert_eq!(state["body"], "missing");
    assert_eq!(state["sessionId"], Value::Null);

    boa.resolve_presentation(&pres_second);
    boa.resolve_body(&text_body("b"));
    let state = boa.state();
    assert_eq!(state["sessionId"], "bbbb-2222");
    assert_eq!(state["body"], "succeeded");
    assert_eq!(state["watch"], true);
    assert_eq!(state["opens"], 2);

    // The first key's stale handle cannot touch the second's state.
    boa.grow(0);
    boa.close_watch(0);
    assert_eq!(boa.state()["watch"], true);
    assert_eq!(boa.state()["body"], "succeeded");
}

#[test]
fn identical_subject_reselection_starts_a_fresh_interval() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference.clone()),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["opens"], 1);
    // A closure spends the interval's opening; the fresh body repaints
    // without a second watch.
    boa.close_watch(0);
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("second"));
    assert_eq!(boa.state()["opens"], 1);
    // The next re-check opens one watch and the interval then refuses a
    // deferred body, exhausting the floor and the budget.
    boa.tick();
    boa.resolve_presentation(&pres);
    assert_eq!(boa.state()["opens"], 2);
    boa.close_watch(1);
    boa.resolve_presentation(&pres);
    boa.reject_body();
    boa.resolve_presentation(&pres);
    let exhausted = boa.state();
    assert_eq!(exhausted["body"], "refused");
    let generation = exhausted["generation"].as_u64().unwrap();

    // The operator reselects the identical subject: a fresh generation,
    // interval, presentation, body and eligible watch — never the
    // consumed refusal floor or watch budget.
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    let fresh = boa.state();
    assert!(
        fresh["generation"].as_u64().unwrap() > generation,
        "identical reselection is an event, not a no-op"
    );
    assert_eq!(fresh["body"], "missing");
    assert_eq!(fresh["openingsUsed"], 0);
    assert_eq!(fresh["watch"], false);

    // Late callbacks from the prior generation stay inert while the fresh
    // interval completes.
    boa.grow(0);
    boa.close_watch(0);
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("third"));
    let state = boa.state();
    assert_eq!(state["body"], "succeeded");
    assert_eq!(state["watch"], true);
    assert_eq!(state["opens"], 3, "one eligible opening per interval");
}

// =========================================================================
// HTTP: the shared Claude routes and the participant-presentation route.

pub(super) fn percent_encode(component: &str) -> String {
    let mut out = String::new();
    for byte in component.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub(super) fn participant_fixture(
    reference: Option<Value>,
    provider: Option<&str>,
    session_id: Option<&str>,
) -> (tempfile::TempDir, PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r1", "feat", "self", &json!({"files": {}}))
        .unwrap();
    {
        let mut append = |kind, payload| {
            store.append_next("r1", kind, payload, None, None).unwrap();
        };
        append(
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
        );
        append(EventType::PhaseEntered, json!({"phase": "intake"}));
        append(
            EventType::EffectRequested,
            json!({"effect_id": "e1:seat", "seat": "intake", "phase": "intake",
                   "idempotency_key": "k", "input_digest": "d"}),
        );
        let mut started = json!({"effect_id": "e1:seat", "attempt_id": "a1", "driver": "d"});
        if let Some(provider) = provider {
            started["provenance"] = json!([{"member": null, "agent": "intake",
                "model": "m", "provider": provider, "chain_index": 1}]);
        }
        append(EventType::EffectStarted, started);
        let mut checkpoint = json!({"step": "session-started"});
        if let Some(reference) = reference {
            checkpoint["transcript"] = reference;
        }
        if let Some(session_id) = session_id {
            checkpoint["session_id"] = json!(session_id);
        }
        append(
            EventType::EffectCheckpointed,
            json!({"effect_id": "e1:seat", "attempt_id": "a1", "checkpoint": checkpoint}),
        );
    }
    let view: Value = serde_json::from_str(&handle(&db, "/api/view/r1").body).unwrap();
    let key = view["participants"][0]["key"]
        .as_str()
        .expect("the fixture derives a participant")
        .to_string();
    (dir, db, key)
}

pub(super) fn claude_projects_home() -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    let projects = home.path().join(".claude").join("projects");
    std::fs::create_dir_all(projects.join("seat")).unwrap();
    (home, projects)
}

#[test]
fn the_presentation_route_decodes_each_component_exactly_once() {
    let (_dir, db, key) = participant_fixture(
        Some(claude_reference("abcd-1234", "/retained/claude")),
        None,
        None,
    );
    let once = percent_encode(&key);
    let twice = percent_encode(&once);

    let response = handle(&db, &format!("/api/presentation/r1/{once}"));
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert!(parsed.get("reference").is_some());
    assert!(parsed.get("legacy").is_some());
    assert!(parsed.get("admitted").is_some());
    assert!(parsed.get("reason").is_some());
    assert!(parsed.get("explanation").is_some());
    assert!(parsed.get("hint").is_some());
    assert!(parsed.get("drill_eligible").is_some());
    assert!(
        parsed.get("turns").is_none(),
        "no prose in the presentation"
    );
    assert!(parsed.get("blocks").is_none());
    assert!(parsed.get("truncated").is_none());

    // A double-encoded key decodes once to a non-peer, never twice.
    assert_eq!(
        handle(&db, &format!("/api/presentation/r1/{twice}")).status,
        "404 Not Found"
    );

    // Malformed escapes and extra or missing components are refused.
    for bad in [
        "/api/presentation/r1/%zz".to_string(),
        "/api/presentation/r1/%".to_string(),
        "/api/presentation/r1".to_string(),
        "/api/presentation/r1/".to_string(),
        "/api/presentation/".to_string(),
        format!("/api/presentation/r1/{once}/extra"),
    ] {
        assert_eq!(
            handle(&db, &bad).status,
            "404 Not Found",
            "route {bad} must refuse"
        );
    }
}

/// R25: the participant route carries an agent-controlled confirmed
/// path/home only as the shared reversible portable display literal, and
/// the served page paints that exact value through a text node without
/// reconstructing a fragment.
#[test]
fn a_hostile_recorded_home_reaches_the_page_only_as_portable_display_data() {
    let dir = tempfile::tempdir().unwrap();
    let hostile_home = dir.path().join("home $(x) `t` ;a&b|c<d>e%f!g \"q\" \\ é😀");
    std::fs::create_dir_all(hostile_home.join("sessions")).unwrap();
    std::fs::write(
        hostile_home.join("sessions/rollout-0199mine.jsonl"),
        "{\"type\":\"turn_context\"}\n",
    )
    .unwrap();
    let home_text = hostile_home.to_str().unwrap().to_string();
    let (_dir, db, key) = participant_fixture(
        Some(json!({"kind": "codex-thread", "locator": "0199mine", "home": home_text})),
        None,
        None,
    );

    let response = handle(
        &db,
        &format!("/api/presentation/r1/{}", percent_encode(&key)),
    );
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(parsed["admitted"], true, "{}", response.body);
    assert_eq!(parsed["drill_eligible"], true, "{}", response.body);
    let path = parsed["path"].as_str().expect("a confirmed rollout");
    let expected = format!(
        "full session: path {}, codex exec resume 0199mine, home {}",
        brokkr_view::transcript::portable_display_literal(path),
        brokkr_view::transcript::portable_display_literal(&home_text),
    );
    let hint = parsed["hint"].as_str().expect("the shared hint");
    assert_eq!(hint, expected);
    for raw in ["$(", "`", ";", "&", "|", "<", ">", "%", "!", "é", "😀"] {
        assert!(!hint.contains(raw), "{raw:?} survived raw in {hint:?}");
    }
    assert!(hint.contains("\\u0020"), "{hint:?}");
    assert!(hint.contains("\\ud83d\\ude00"), "{hint:?}");
    // The response's decoded JSON value equals the shared string; the raw
    // document carries the separate outer escaping layer only. The echo of
    // the recorded reference keeps its own raw bytes; only the completed
    // hint must be portable.
    assert!(response.body.contains("\\\\u0020"), "{}", response.body);

    // The served page consumes `view.hint` verbatim through `el`'s
    // textContent and never derives or reconstructs a path/home fragment.
    let page = handle(&db, "/");
    assert_eq!(page.status, "200 OK");
    assert!(
        page.body.contains("el('p', 'cause', view.hint)"),
        "the participant block paints the shared hint verbatim"
    );
    assert!(!page.body.contains("full session: <id>"));
}

#[test]
fn an_eligible_claude_participant_shares_the_hint_and_turns() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    std::fs::write(
        projects.join("seat/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"the words\"}}\n",
    )
    .unwrap();
    env.set("HOME", home.path());
    let (_dir, db, key) = claude_participant_under(&projects);

    let parsed = presentation_route(&db, &key);
    assert_eq!(parsed["admitted"], true, "{parsed}");
    assert_eq!(parsed["drill_eligible"], true, "{parsed}");
    assert_eq!(parsed["reason"], Value::Null);
    assert_eq!(parsed["hint"], "full session: claude --resume abcd-1234");
    assert!(parsed["path"]
        .as_str()
        .unwrap()
        .ends_with("abcd-1234.jsonl"));

    let api = transcript_route(&db, &key);
    assert_eq!(api.status, "200 OK");
    let body: Value = serde_json::from_str(&api.body).unwrap();
    assert_eq!(
        (&body["run_id"], &body["seat"]),
        (&json!("r1"), &json!(key))
    );
    assert_eq!(body["turns"][0]["blocks"][0]["text"], "the words");
    assert_eq!(body["truncated"], false);
}

/// A legacy Codex seat's flat id never synthesizes a Claude read (#352
/// retired the id-only route that once answered it anyway).
#[test]
fn a_legacy_codex_participant_is_ineligible_and_its_transcript_refuses() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    std::fs::write(
        projects.join("seat/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"hello\"}}\n",
    )
    .unwrap();
    env.set("HOME", home.path());
    let (_dir, db, key) = participant_fixture(None, Some("codex"), Some("abcd-1234"));

    let response = handle(
        &db,
        &format!("/api/presentation/r1/{}", percent_encode(&key)),
    );
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(parsed["admitted"], false, "{}", response.body);
    assert_eq!(parsed["reason"], "no-reference");
    assert_eq!(parsed["hint"], Value::Null);
    assert_eq!(parsed["drill_eligible"], false);

    let api = transcript_route(&db, &key);
    assert_eq!(api.status, "404 Not Found", "{}", api.body);
    let body: Value = serde_json::from_str(&api.body).unwrap();
    assert_eq!(body["unavailable"], "no-reference");
    assert_eq!(body["turns"], json!([]));
}

#[test]
fn a_common_reference_defeats_a_stale_flat_id() {
    for reference in [
        json!({"kind": "none", "locator": "", "home": ""}),
        json!({"kind": "claude-session", "locator": "", "home": "/retained"}),
    ] {
        let (_dir, db, key) =
            participant_fixture(Some(reference), Some("claude"), Some("abcd-1234"));
        let response = handle(
            &db,
            &format!("/api/presentation/r1/{}", percent_encode(&key)),
        );
        assert_eq!(response.status, "200 OK", "{}", response.body);
        let parsed: Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(parsed["legacy"], false);
        assert_eq!(parsed["admitted"], false, "{}", response.body);
        assert!(parsed["reason"] == "none" || parsed["reason"] == "unannounced");
        assert_eq!(parsed["hint"], Value::Null);
        assert_eq!(parsed["path"], Value::Null);
    }
}

#[test]
fn every_api_body_and_the_presentation_send_no_store() {
    let (_dir, db, key) = participant_fixture(
        Some(claude_reference("abcd-1234", "/retained/claude")),
        None,
        None,
    );
    let requests = [
        "GET /api/runs HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_string(),
        "GET /api/view/r1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_string(),
        "GET /api/run/r1 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_string(),
        format!("/api/presentation/r1/{}", percent_encode(&key)),
        format!("/api/transcript/r1/{}", percent_encode(&key)),
    ];
    for request in requests {
        let line = if request.starts_with("GET") {
            request
        } else {
            format!("GET {request} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        };
        let response = exchange(db.clone(), &line, None);
        assert!(
            response.contains("Cache-Control: no-store"),
            "{line}: {response}"
        );
    }

    // A refusal carries the header too.
    let missing = exchange(
        db,
        "GET /api/transcript/r1/nobody HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
        None,
    );
    assert!(missing.contains("Cache-Control: no-store"), "{missing}");
    assert!(missing.contains("{\"error\":\"participant not found\"}"));
}

/// Operator ruling 2026-09-27 (decision 0073 rulings 3 and 4): a Claude
/// reference recorded under a home that is not the local projects home is
/// drill-eligible like every other valid reference. The body route serves
/// it, its watch opens, and the page keeps no recorded-home refusal.
#[test]
fn a_foreign_claude_home_drills_over_http() {
    let mut env = EnvGuard::lock();
    let (local_home, _local_projects) = claude_projects_home();
    env.set("HOME", local_home.path());
    let custom = tempfile::tempdir().unwrap();
    let custom_projects = custom.path().join("claude-projects");
    std::fs::create_dir_all(custom_projects.join("seat")).unwrap();
    std::fs::write(
        custom_projects.join("seat/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"custom\"}}\n",
    )
    .unwrap();
    let (_dir, db, key) = claude_participant_under(&custom_projects);

    let parsed = presentation_route(&db, &key);
    assert_eq!(parsed["admitted"], true, "{parsed}");
    assert_eq!(parsed["drill_eligible"], true, "{parsed}");
    assert_eq!(parsed["hint"], "full session: claude --resume abcd-1234");
    assert_eq!(parsed["reason"], Value::Null);

    let body = transcript_route(&db, &key);
    assert_eq!(body.status, "200 OK", "{}", body.body);
    let document: Value = serde_json::from_str(&body.body).unwrap();
    assert_eq!(document["turns"][0]["blocks"][0]["text"], "custom");
    let route = format!("r1/{}", percent_encode(&key));
    let watch = exchange(db, &watch_request(&route), Some(1));
    assert!(watch.starts_with("HTTP/1.1 200 OK"), "{watch}");
    assert!(watch.contains("Content-Type: text/event-stream"), "{watch}");
    assert!(
        !PAGE.contains("homeUnavailable"),
        "the page mirrors no home rule"
    );
}

#[test]
fn the_client_id_guard_refuses_a_leading_hyphen_drill() {
    let reference = claude_reference("-abc", "/local/projects");
    let pres = presentation(
        reference.clone(),
        true,
        None,
        Some("full session: claude --resume -abc"),
        true,
    );
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres);
    let state = boa.state();
    assert_eq!(
        state["sessionId"],
        Value::Null,
        "the client guard is the leading-hexadecimal rule"
    );
    assert_eq!(state["bodies"], 0);
    assert_eq!(state["opens"], 0);
    let trace = boa.trace();
    assert!(
        !trace.iter().any(|entry| entry.starts_with("body ")),
        "{trace:?}"
    );
    assert!(
        !trace.iter().any(|entry| entry.starts_with("open ")),
        "{trace:?}"
    );
}

#[test]
fn an_eligibility_change_resets_that_key() {
    let reference = claude_reference("abcd-1234", "/retained/claude");
    let eligible = presentation(reference.clone(), true, None, Some("hint"), true);
    let ineligible = presentation(reference.clone(), true, None, Some("hint"), false);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&eligible);
    boa.resolve_body(&text_body("x"));
    assert_eq!(boa.state()["watch"], true);

    boa.tick();
    boa.resolve_presentation(&ineligible);
    let state = boa.state();
    assert_eq!(state["drillEligible"], false);
    assert_eq!(state["watch"], false, "eligibility loss closes the watch");
    assert_eq!(state["body"], "missing", "eligibility loss clears prose");
}

#[test]
fn a_failed_presentation_request_is_an_unreadable_refusal() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.reject_presentation();
    let state = boa.state();
    assert_eq!(state["admitted"], false);
    assert_eq!(state["reason"], "unreadable");
    assert_eq!(state["bodies"], 0);
    assert_eq!(state["opens"], 0);
    assert_eq!(state["sessionId"], Value::Null);
}

/// Syncing a participant from working to concluded closes the watch it
/// owns instead of leaving it open on a seat with no further prose.
#[test]
fn syncing_a_concluded_participant_closes_its_owned_watch() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut working = subject("r1", "seat", Some(reference), None, Some("claude"), true);
    let mut boa = Boa::boot();
    boa.select(&working);
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["watch"], true);

    working["working"] = json!(false);
    boa.sync(&working);
    let state = boa.state();
    assert_eq!(state["watch"], false, "a concluded seat keeps no watch");
    assert!(
        boa.trace().iter().any(|entry| entry.starts_with("close ")),
        "the transition closes the owned watch: {:?}",
        boa.trace()
    );
}

/// Two growth callbacks issue two body requests; when the newer response
/// resolves first, the older superseded response cannot repaint.
#[test]
fn overlapping_growth_reads_cannot_restore_older_prose() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let pres = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&pres);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["watch"], true);

    boa.grow(0);
    boa.grow(0);
    assert_eq!(
        boa.state()["bodies"],
        3,
        "each growth callback issues one body request"
    );

    // Resolve the newer request first, then the older one.
    boa.resolve_body_at(1, &text_body("newer"));
    boa.resolve_body_at(0, &text_body("older"));
    let renders = boa
        .trace()
        .iter()
        .filter(|entry| entry.starts_with("render body"))
        .count();
    assert_eq!(
        renders,
        2,
        "only the newest body request may repaint: {:?}",
        boa.trace()
    );
}

/// An eligibility-only presentation change resets the watch budget, as
/// the transcript-reading recovery rule exempts eligibility changes from
/// the previous automatic-recovery bound.
#[test]
fn an_eligibility_change_restores_the_watch_budget() {
    let reference = claude_reference("abcd-1234", "/local/projects");
    let ineligible = presentation(reference.clone(), true, None, Some("hint"), false);
    let eligible = presentation(reference.clone(), true, None, Some("hint"), true);
    let mut boa = Boa::boot();
    boa.select(&subject(
        "r1",
        "seat",
        Some(reference),
        None,
        Some("claude"),
        true,
    ));
    boa.resolve_presentation(&eligible);
    boa.resolve_body(&text_body("first"));
    assert_eq!(boa.state()["watch"], true);
    assert_eq!(boa.state()["openingsUsed"], 1);

    // The closure's fresh presentation turns the drill ineligible. The
    // spent opening must not survive that eligibility change.
    boa.close_watch(0);
    boa.resolve_presentation(&ineligible);
    assert_eq!(
        boa.state()["openingsUsed"],
        0,
        "an eligibility change restores the watch budget"
    );
    assert_eq!(boa.state()["drillEligible"], false);
    assert_eq!(
        boa.state()["opens"],
        1,
        "no watch opens while the drill is ineligible"
    );
}

#[test]
fn an_enumeration_error_closes_browser_admission() {
    use crate::local_transcript::fault::{FailAt, Plan};
    use crate::local_transcript::fault_tests::claude_fixture;

    let (dir, reference) = claude_fixture("one");
    let home = reference.home.clone();
    let (_db_dir, db, key) = participant_fixture(
        Some(json!({"kind": "claude-session", "locator": "abcd-1234", "home": home})),
        None,
        None,
    );
    let _guard = Plan::new().fail(FailAt::Entries, 1).install();
    let response = handle(
        &db,
        &format!("/api/presentation/r1/{}", percent_encode(&key)),
    );
    assert_eq!(response.status, "200 OK", "{}", response.body);
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(parsed["admitted"], false, "{}", response.body);
    assert_eq!(parsed["reason"], "unreadable", "{}", response.body);
    drop(dir);
}

#[test]
fn an_empty_presentation_run_component_is_refused() {
    let (_dir, db, key) = participant_fixture(
        Some(claude_reference("abcd-1234", "/retained/claude")),
        None,
        None,
    );
    let response = handle(&db, &format!("/api/presentation//{}", percent_encode(&key)));
    assert_eq!(response.status, "404 Not Found");
    assert_eq!(
        response.body,
        json!({"error": "participant not found"}).to_string()
    );
}

#[test]
fn an_unknown_presentation_run_is_not_found() {
    let (dir, db) = fixture();
    let response = handle(&db, "/api/presentation/unknown-run/whatever");
    assert_eq!(response.status, "404 Not Found");
    assert_eq!(
        response.body,
        json!({"error": "participant not found"}).to_string()
    );
    drop(dir);
}

/// A missing journal gains no file of any kind on any browser route: each
/// journal route answers 404, one poll of the run stream opens read-only and
/// reports head sequence zero, and the database path, its `-wal` and its
/// `-shm` still do not exist afterward.
#[test]
fn a_missing_journal_gains_no_file_on_every_browser_route() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("missing").join("forge.db");
    for path in [
        "/api/runs",
        "/api/run/r222",
        "/api/view/r222",
        "/api/presentation/r222/key",
        "/api/transcript/r222/key",
    ] {
        let response = handle(&db, path);
        assert_eq!(
            response.status, "404 Not Found",
            "{path}: {}",
            response.body
        );
    }

    let request = b"GET /sse/r222 HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n".to_vec();
    let mut reader = std::io::Cursor::new(request);
    let mut out: Vec<u8> = Vec::new();
    serve_io(&db, &mut reader, &mut out, Some(1));
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("\"seq\":0"), "{text}");

    assert!(!db.exists(), "no database was created");
    assert!(
        !PathBuf::from(format!("{}-wal", db.display())).exists(),
        "no -wal was created"
    );
    assert!(
        !PathBuf::from(format!("{}-shm", db.display())).exists(),
        "no -shm was created"
    );
}
