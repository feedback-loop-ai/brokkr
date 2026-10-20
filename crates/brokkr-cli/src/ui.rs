//! `brokkr ui` — the embedded read-only surface (decision 0003): a static
//! page compiled into the binary, served on loopback, reading
//! projections only. No commands, no writes, no Node, no external
//! assets; removing this module changes nothing about execution
//! semantics (first-release acceptance criteria).
//!
//! Routes: `/` (page) · `/api/runs` · `/api/view/<id>` ·
//! `/api/presentation/<run>/<key>` · `/api/transcript/<run>/<key>` ·
//! `/sse/<id>` (server-sent head changes, poll-backed) ·
//! `/sse/transcript/<run>/<key>` (server-sent transcript growth,
//! poll-backed by the same clock).
//!
//! `/api/runs` and `/api/view/<id>` serve `brokkr-view`'s models: the page
//! paints them and derives nothing (decision 0013).
//!
//! What a client may hold of the surface, its request's bytes and time, a
//! write's wait and a share of the connections, is bounded in [`limits`].

mod limits;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use brokkr_core::fold::fold;
use brokkr_core::EventEnvelope;
use brokkr_protocol::overrides::{self, Override, OverrideError};
use brokkr_store::{Store, StoreError};
use brokkr_view::transcript::{Selection, ValidReference};
use serde_json::{json, Value};

use crate::fleet::PageRefused;
use crate::local_transcript::{self, Discovery};
use limits::{Deadline, Exceeded, Limits, Slots, Unread};

const PAGE: &str = include_str!("ui.html");

pub struct Response {
    pub status: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

fn ok(content_type: &'static str, body: String) -> Response {
    Response {
        status: "200 OK",
        content_type,
        body,
    }
}

/// A read this surface could not make, in the refusal's own words.
fn server_error(detail: String) -> Response {
    Response {
        status: "500 Internal Server Error",
        content_type: "application/json",
        body: json!({"error": detail}).to_string(),
    }
}

fn bad_request(detail: String) -> Response {
    Response {
        status: "400 Bad Request",
        content_type: "application/json",
        body: json!({"error": detail}).to_string(),
    }
}

/// `/api/runs`: every run, the fleet's sections in order and each by last
/// activity, as `RunsView.runs` — ordering is a derivation rule and not
/// something each surface reverses for itself. Asked `limit=N` or
/// `after=CURSOR`, one page as `brokkr runs --json --limit` prints it,
/// with its `next` (decision 0089 ruling 6).
///
/// The same fleet grace the table gives: a run whose journal does not
/// load or fold is a quarantined row carrying the refusal, never a
/// missing row. A journal that cannot list its runs at all is refused the
/// way one that cannot open is, never an empty fleet.
fn runs(db: &Path, store: &Store, query: Option<&str>) -> Response {
    let asked = match query.map(page_asked).transpose() {
        Ok(asked) => asked,
        Err(refused) => return bad_request(refused.to_string()),
    };
    // A refresh reads at most a page and one ahead of the runs behind the
    // summary and counts the rest, which every run keys and lists at
    // their places, all of it one read (decision 0089 ruling 6).
    let now = crate::now_rfc3339();
    let journal = (crate::fleet::Journal::at(db), now.as_str());
    let body = crate::fleet::list_hearth(store, journal, asked.is_none(), |lister| match &asked {
        None => lister.rows().map(|rows| serde_json::to_string(&rows)),
        Some((after, limit)) => lister
            .paged(after.as_ref(), *limit)
            .map(|view| serde_json::to_string(&view)),
    });
    match body {
        Ok(body) => ok("application/json", body.expect("run rows serialize")),
        Err(error) => server_error(error.to_string()),
    }
}

/// The page a query asks for: `limit=N`, at least one, and `after=CURSOR`,
/// in any order, the last of a repeated one standing, or the words of
/// what it is not.
fn page_asked(query: &str) -> Result<(Option<brokkr_view::Cursor>, usize), PageRefused> {
    let mut asked = (None, crate::fleet::PAGE_ROWS);
    for pair in query.split('&') {
        match pair.split_once('=') {
            Some(("limit", limit)) => {
                asked.1 = limit
                    .parse::<usize>()
                    .ok()
                    .filter(|limit| *limit > 0)
                    .ok_or_else(|| PageRefused::Limit(limit.to_string()))?;
            }
            Some(("after", after)) => asked.0 = Some(crate::fleet::cursor(after)?),
            _ => return Err(PageRefused::Query(pair.to_string())),
        }
    }
    Ok(asked)
}

fn not_found(what: &str) -> Response {
    Response {
        status: "404 Not Found",
        content_type: "application/json",
        body: json!({"error": format!("{what} not found")}).to_string(),
    }
}

/// DNS-rebinding guard: a remote page can make the victim's browser
/// resolve an attacker domain to 127.0.0.1 and read journals unless the
/// Host header is pinned to loopback names. Reject everything else.
pub(crate) fn request_allowed(method: &str, host: Option<&str>) -> bool {
    if method != "GET" {
        return false;
    }
    let Some(host) = host else { return false };
    let name = host.rsplit_once(':').map(|(n, _)| n).unwrap_or(host);
    matches!(name, "127.0.0.1" | "localhost" | "[::1]")
}

/// Pure request handling: path in, response out. The TCP loop below is a
/// thin shell around this, which is what the tests exercise.
pub fn handle(db: &Path, path: &str) -> Response {
    if path == "/" {
        return ok("text/html; charset=utf-8", PAGE.to_string());
    }
    if !db.is_file() {
        // Reads never create: a missing database is a 404, not an
        // initialized empty store.
        return not_found("database");
    }
    let store = match Store::open_read_only(db) {
        Ok(store) => store,
        Err(e) => return server_error(e.to_string()),
    };
    if let Some(rest) = path.strip_prefix("/api/presentation/") {
        return participant_presentation(&store, rest);
    }
    if let Some(rest) = path.strip_prefix("/api/transcript/") {
        return participant_transcript(db, &store, rest);
    }
    if let Some(query) = path
        .strip_prefix("/api/runs")
        .filter(|rest| rest.is_empty() || rest.starts_with('?'))
    {
        return runs(db, &store, query.strip_prefix('?'));
    }
    if let Some(run_id) = path.strip_prefix("/api/view/") {
        let events = match opened(&store, run_id) {
            Ok(events) => events,
            Err(refusal) => return refusal.response(run_id),
        };
        let state = fold(&events).ok();
        let view = brokkr_view::run_view(&events, state.as_ref()).with_scopes();
        return ok(
            "application/json",
            serde_json::to_string(&view).expect("the run view serializes"),
        );
    }
    not_found(path)
}

/// The lowest drill level (#352): one participant's transcript, read by
/// the one local read path `brokkr transcript` uses and masked against the
/// same secrets store beside the journal, served as that command's
/// `--json` document byte for byte, for every kind. This is a
/// loopback-only, operator-local surface — the same trust as the command
/// in a terminal. A refused read keeps its document under a 404, so the
/// page falls back to the checkpoint stream.
fn participant_transcript(db: &Path, store: &Store, rest: &str) -> Response {
    let (run_id, participant) = match route_participant(store, rest) {
        Ok(found) => found,
        Err(refusal) => return refusal.response("participant"),
    };
    // The browser reads one journal, so its subject is hearth zero's.
    let subject = brokkr_view::Subject::of(0, None, run_id.clone(), &participant);
    let read = local_transcript::read_local(&subject, crate::local_projects_home().as_deref());
    let read = local_transcript::mask_secrets(read, &local_transcript::store_beside(db));
    Response {
        status: if read.is_readable() {
            "200 OK"
        } else {
            "404 Not Found"
        },
        content_type: "application/json",
        body: crate::transcript_document(&run_id, &participant.key, &read, None),
    }
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Percent-decode one path component exactly once. `+` is a literal plus
/// in a path, and a `%` that does not begin a two-hex-digit escape
/// refuses the component rather than being repaired.
fn decode_component(component: &str) -> Option<String> {
    let bytes = component.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).copied().and_then(hex_nibble);
            let low = bytes.get(index + 2).copied().and_then(hex_nibble);
            let (Some(high), Some(low)) = (high, low) else {
                return None;
            };
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

/// The one drill-eligibility rule the presentation reports, the page obeys
/// and the watch opens by (decision 0073 rulings 3 and 4): every reference
/// that validates is eligible, of every kind and under any recorded home.
fn drillable(selection: &Selection) -> Option<&ValidReference> {
    selection.outcome.as_ref().ok()
}

/// The CLI-private, prose-free participant presentation (D9): the
/// selected reference, its legacy flag, admission state, discovery-stage
/// unavailability, the shared hint and drill eligibility ([`drillable`]).
/// It is constructed from selection, validation, bounded safe discovery and
/// hint helpers only — never from body bytes or a content projector.
fn presentation_payload(participant: &brokkr_view::Participant) -> Value {
    let selection = participant_selection(participant);
    let (admitted, reason, explanation, path, valid) = match &selection.outcome {
        Ok(valid) => match local_transcript::discover(valid) {
            Discovery::Admitted(source) => (
                true,
                None,
                None,
                Some(source.path.clone()),
                Some(valid.clone()),
            ),
            Discovery::Refused(reason, explanation) => (
                false,
                Some(reason.as_str()),
                Some(
                    explanation.unwrap_or_else(|| brokkr_view::transcript::explanation_for(reason)),
                ),
                None,
                Some(valid.clone()),
            ),
        },
        Err(reason) => (
            false,
            Some(reason.as_str()),
            Some(brokkr_view::transcript::explanation_for(*reason)),
            None,
            None,
        ),
    };
    let hint = valid
        .as_ref()
        .and_then(|valid| brokkr_view::transcript::full_session(valid, path.as_deref()));
    let drill_eligible = drillable(&selection).is_some();
    json!({
        "reference": selection.reference,
        "legacy": selection.legacy,
        "admitted": admitted,
        "reason": reason,
        "explanation": explanation,
        "path": path,
        "hint": hint,
        "drill_eligible": drill_eligible,
    })
}

/// Why a route that opens a run answers for no run or participant.
enum RouteRefusal {
    /// The route, its run or its participant is not there.
    Missing,
    /// The run is there, and the store refuses to open it: its chain does
    /// not verify, or a row of it does not read. Every route that opens a
    /// run refuses it in the store's words and reads nothing, as each of
    /// the run's verbs does (decision 0089 ruling 5).
    Unopened(StoreError),
    /// The run's journal loads and does not fold. That is fatal to the
    /// run's own verbs (`fleet.rs`), so every participant route refuses it
    /// in the fold's words, as `brokkr transcript` does, and reads nothing.
    Unfoldable(brokkr_core::fold::FoldError),
}

impl RouteRefusal {
    /// The response a route gives, naming `missing` when nothing is there.
    fn response(self, missing: &str) -> Response {
        match self {
            RouteRefusal::Missing => not_found(missing),
            RouteRefusal::Unopened(error) => server_error(error.to_string()),
            RouteRefusal::Unfoldable(error) => server_error(error.to_string()),
        }
    }
}

/// Run `run_id`'s events, its chain read whole and verified: missing when
/// the journal holds no such run, and the store's refusal otherwise.
fn opened(store: &Store, run_id: &str) -> Result<Vec<EventEnvelope>, RouteRefusal> {
    store.load(run_id).map_err(|error| match error {
        StoreError::RunNotFound(_) => RouteRefusal::Missing,
        refused => RouteRefusal::Unopened(refused),
    })
}

/// The participant a `<run>/<key>` route names, the three participant
/// routes' one lookup. No path or home override is accepted.
fn route_participant(
    store: &Store,
    rest: &str,
) -> Result<(String, brokkr_view::Participant), RouteRefusal> {
    let (run_id, participant_key) = route_components(rest).ok_or(RouteRefusal::Missing)?;
    let events = opened(store, &run_id)?;
    let state = fold(&events).map_err(RouteRefusal::Unfoldable)?;
    let view = brokkr_view::run_view(&events, Some(&state));
    let participant = view
        .participants
        .into_iter()
        .find(|participant| participant.key == participant_key)
        .ok_or(RouteRefusal::Missing)?;
    Ok((run_id, participant))
}

/// A route's full run id and encoded participant key. Each path component
/// is decoded exactly once; a malformed escape or any extra component is
/// refused before the read-only journal is touched.
fn route_components(rest: &str) -> Option<(String, String)> {
    let mut components = rest.split('/');
    let (Some(run_id), Some(participant_key), None) =
        (components.next(), components.next(), components.next())
    else {
        return None;
    };
    if run_id.is_empty() || participant_key.is_empty() {
        return None;
    }
    let run_id = decode_component(run_id)?;
    let participant_key = decode_component(participant_key)?;
    // The post-decode emptiness check is removed as unreachable:
    // `decode_component` appends exactly one byte per input byte or `%XX`
    // triple, so a non-empty component decodes to a non-empty byte string;
    // `String::from_utf8` of a non-empty vector is non-empty or `None`; and
    // the pre-decode check above already refused an empty component.
    Some((run_id, participant_key))
}

/// One GET participant-presentation route.
fn participant_presentation(store: &Store, rest: &str) -> Response {
    match route_participant(store, rest) {
        Ok((_, participant)) => ok(
            "application/json",
            presentation_payload(&participant).to_string(),
        ),
        Err(refusal) => refusal.response("participant"),
    }
}

/// The reference a participant selects against the local projects home,
/// or why none is selected.
fn participant_selection(participant: &brokkr_view::Participant) -> Selection {
    brokkr_view::transcript::select_reference(
        participant.transcript.as_ref(),
        participant.legacy_provenance(),
        participant.session_id.as_deref(),
        crate::local_projects_home().as_deref(),
    )
}

/// The drill-eligible reference a watched participant selects, resolved
/// once from the read-only journal when the watch opens, with a source on
/// this machine; otherwise the response that refuses the watch.
fn watched_reference(db: &Path, rest: &str) -> Result<ValidReference, Response> {
    let missing = || not_found("transcript");
    let store = Store::open_read_only(db).map_err(|_| missing())?;
    let (_, participant) =
        route_participant(&store, rest).map_err(|refusal| refusal.response("transcript"))?;
    drillable(&participant_selection(&participant))
        .filter(|valid| local_transcript::source_size(valid).is_some())
        .cloned()
        .ok_or_else(missing)
}

fn head_seq(db: &Path, run_id: &str) -> u64 {
    Store::open_read_only(db)
        .and_then(|s| s.head_hash(run_id))
        .map(|(seq, _)| seq)
        .unwrap_or(0)
}

/// The request line and the headers, within `limits`' bytes: a line that
/// runs past its room is refused before it is held whole. The section ends
/// at its blank line or at EOF.
fn read_request(
    reader: &mut impl BufRead,
    limits: &Limits,
) -> Result<(String, String, Option<String>), Unread> {
    let over_line = Exceeded::RequestLine {
        limit: limits.request_line_bytes,
    };
    let line = limits.line(reader, limits.request_line_bytes, over_line)?;
    let method = line.split_whitespace().next().unwrap_or("").to_string();
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let mut host: Option<String> = None;
    let over_headers = Exceeded::Headers {
        limit: limits.header_bytes,
    };
    let mut room = limits.header_bytes;
    loop {
        let header = limits.line(reader, room, over_headers)?;
        room -= header.len();
        if header.trim().is_empty() {
            break;
        }
        if let Some(value) = header.to_ascii_lowercase().strip_prefix("host:") {
            host = Some(value.trim().to_string());
        }
    }
    Ok((method, path, host))
}

fn serve_client(db: PathBuf, stream: TcpStream, limits: Limits) {
    serve_client_with_limit(db, stream, limits, None)
}

fn serve_client_with_limit(
    db: PathBuf,
    stream: TcpStream,
    limits: Limits,
    sse_limit: Option<usize>,
) {
    // The platform refuses a zero write timeout: no write is admitted, so
    // the connection closes unanswered.
    if stream
        .set_write_timeout(Some(limits.write_timeout))
        .is_err()
    {
        return;
    }
    let mut reader = BufReader::new(Deadline::new(&stream, limits.read_timeout));
    let mut writer = &stream;
    serve_io(&db, &mut reader, &mut writer, &limits, sse_limit);
}

/// One poll of every server-sent stream on this surface. The run's head
/// and a seat's transcript move on the same clock, which is why there is
/// one constant and no second cadence to reason about.
const SSE_POLL: std::time::Duration = std::time::Duration::from_millis(1000);

const SSE_HEADER: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                          Cache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";

fn write_response(stream: &mut impl Write, response: Response) {
    let payload = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nCache-Control: no-store\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{}",
        response.status,
        response.content_type,
        response.body.len(),
        response.body
    );
    let _ = stream.write_all(payload.as_bytes());
}

fn serve_io(
    db: &Path,
    reader: &mut impl BufRead,
    stream: &mut impl Write,
    limits: &Limits,
    sse_limit: Option<usize>,
) {
    let (method, path, host) = match read_request(reader, limits) {
        Ok(request) => request,
        Err(Unread::Exceeded(exceeded)) => return write_response(stream, exceeded.response()),
        Err(Unread::Unreadable) => return,
    };
    if !request_allowed(&method, host.as_deref()) {
        let _ = stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }

    // Before the run's stream, because a run id never contains a slash:
    // the seat's own prose lands BETWEEN journal checkpoints, so the
    // transcript is watched on its own file rather than on the head.
    if let Some(rest) = path.strip_prefix("/sse/transcript/") {
        watch_transcript(db, rest, stream, sse_limit);
        return;
    }

    if let Some(run_id) = path.strip_prefix("/sse/") {
        let run_id = run_id.to_string();
        if stream.write_all(SSE_HEADER.as_bytes()).is_err() {
            return;
        }
        let mut last = u64::MAX; // always push one initial event
        let mut sent = 0usize;
        loop {
            let seq = head_seq(db, &run_id);
            let message = if seq != last {
                last = seq;
                format!("data: {}\n\n", json!({"seq": seq}))
            } else {
                // Heartbeat comment: reaps disconnected clients within a
                // poll interval instead of leaking a thread per viewer.
                ": ping\n\n".to_string()
            };
            if stream.write_all(message.as_bytes()).is_err() {
                return; // client went away
            }
            sent += 1;
            if sse_limit.is_some_and(|limit| sent >= limit) {
                return;
            }
            std::thread::sleep(SSE_POLL);
        }
    }

    write_response(stream, handle(db, &path));
}

/// One participant's transcript growth (#352), keyed like its body route
/// and for every kind. The route, the journal, the reference and safe
/// discovery are all settled before a single byte of stream is written: a
/// participant with no transcript on this machine is a clean 404 — never
/// an open connection waiting for a file to appear. Every refusal is
/// `{"error":"transcript not found"}`, except an unfoldable journal's,
/// which carries the fold's words.
fn watch_transcript(db: &Path, rest: &str, stream: &mut impl Write, sse_limit: Option<usize>) {
    let reference = match watched_reference(db, rest) {
        Ok(reference) => reference,
        Err(refusal) => return write_response(stream, refusal),
    };
    if stream.write_all(SSE_HEADER.as_bytes()).is_err() {
        return;
    }
    let mut seen: Option<u64> = None;
    let mut sent = 0usize;
    loop {
        // Unique safe discovery is revalidated on every poll, not trusted
        // from the admitted first look. Losing it closes the stream
        // without reporting another size.
        let Some(size) = local_transcript::source_size(&reference) else {
            return;
        };
        let message = if seen.is_some_and(|previous| size > previous) {
            format!("data: {}\n\n", json!({"size": size}))
        } else {
            // Heartbeat comment, exactly as the run's stream: the write
            // that fails is how a closed drilldown is reaped.
            ": ping\n\n".to_string()
        };
        seen = Some(size);
        if stream.write_all(message.as_bytes()).is_err() {
            return; // the operator left the drilldown
        }
        sent += 1;
        if sse_limit.is_some_and(|limit| sent >= limit) {
            return;
        }
        std::thread::sleep(SSE_POLL);
    }
}

/// The browser `open_system_browser` runs, apart from the spawn so a test
/// can hold the refusal without racing a child.
fn browser_program() -> Result<String, OverrideError> {
    overrides::read(Override::BrowserBin)
}

fn open_system_browser(url: &str) {
    open_browser_with(url, |program, url| {
        drop(std::process::Command::new(program).arg(url).spawn());
    });
}

/// `open_system_browser` over an injected spawn, so a test observes that
/// an override that cannot be read runs nothing in its place.
fn open_browser_with(url: &str, spawn: impl FnOnce(&str, &str)) {
    match browser_program() {
        Ok(program) => spawn(&program, url),
        Err(refused) => eprintln!("brokkr ui: no browser opened: {refused}"),
    }
}

/// Accept and serve, each connection on a thread of its own while
/// `limits.connections` are served; one past them is refused by [`refuse`].
/// `connection_limit` ends the loop after that many accepts.
fn serve_listener(
    db: PathBuf,
    listener: TcpListener,
    open_browser: bool,
    connection_limit: Option<usize>,
    limits: Limits,
    mut opener: impl FnMut(&str),
) -> std::io::Result<()> {
    let bound = listener.local_addr()?.port();
    let url = format!("http://127.0.0.1:{bound}/");
    eprintln!("brokkr ui: {url} (read-only; Ctrl-C to stop)");
    if open_browser {
        opener(&url);
    }
    let slots = Slots::new(limits.connections);
    let limit = connection_limit.unwrap_or(usize::MAX);
    for stream in listener.incoming().flatten().take(limit) {
        let Some(slot) = slots.take() else {
            refuse(stream, limits.connections);
            continue;
        };
        let db = db.clone();
        std::thread::spawn(move || {
            serve_client(db, stream, limits);
            drop(slot);
        });
    }
    Ok(())
}

/// A connection past the cap, answered from the acceptor and closed: it is
/// never given a thread, and a client that does not read cannot hold the
/// acceptor, because the write does not block.
fn refuse(mut stream: TcpStream, limit: usize) {
    let _ = stream.set_nonblocking(true);
    write_response(&mut stream, Exceeded::Connections { limit }.response());
}

/// Bind loopback and serve until killed. Returns the bound port (0 in
/// `port` picks an ephemeral one).
pub(crate) fn serve(db: PathBuf, port: u16, open_browser: bool) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    serve_listener(
        db,
        listener,
        open_browser,
        None,
        Limits::DEFAULT,
        open_system_browser,
    )
}

#[cfg(test)]
mod limit_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod transcript_route_tests;
