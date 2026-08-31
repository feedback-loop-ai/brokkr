//! `forge ui` — the embedded read-only surface (decision 0003): a static
//! page compiled into the binary, served on loopback, reading
//! projections only. No commands, no writes, no Node, no external
//! assets; removing this module changes nothing about execution
//! semantics (first-release acceptance criteria).
//!
//! Routes: `/` (page) · `/api/runs` · `/api/view/<id>` · `/api/run/<id>` ·
//! `/api/session/<id>` · `/sse/<id>` (server-sent head changes,
//! poll-backed) · `/sse/session/<id>` (server-sent transcript growth,
//! poll-backed by the same clock).
//!
//! `/api/runs` and `/api/view/<id>` serve `forge-view`'s models: the page
//! paints them and derives nothing (decision 0013). `/api/run/<id>` keeps
//! its raw summary-and-events shape as the parity baseline.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use forge_core::fold::{fold, Status};
use forge_store::Store;
use serde_json::{json, Value};

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

fn not_found(what: &str) -> Response {
    Response {
        status: "404 Not Found",
        content_type: "application/json",
        body: json!({"error": format!("{what} not found")}).to_string(),
    }
}

fn status_str(status: &Status) -> &'static str {
    match status {
        Status::Running => "running",
        Status::AwaitingOperator => "awaiting_operator",
        Status::Completed => "completed",
        Status::Stopped => "stopped",
    }
}

/// DNS-rebinding guard: a remote page can make the victim's browser
/// resolve an attacker domain to 127.0.0.1 and read journals unless the
/// Host header is pinned to loopback names. Reject everything else.
pub fn request_allowed(method: &str, host: Option<&str>) -> bool {
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
    if let Some(session_id) = path.strip_prefix("/api/session/") {
        // Journal-independent: the lowest drill level is the seat's own
        // session transcript on the operator's machine, not the db.
        return session_transcript(session_id);
    }
    if !db.is_file() {
        // Reads never create: a missing database is a 404, not an
        // initialized empty store.
        return not_found("database");
    }
    let store = match Store::open(db) {
        Ok(store) => store,
        Err(e) => {
            return Response {
                status: "500 Internal Server Error",
                content_type: "application/json",
                body: json!({"error": e.to_string()}).to_string(),
            }
        }
    };
    if path == "/api/runs" {
        // The page receives `RunsView.runs` — already newest first,
        // because ordering is a derivation rule and not something each
        // surface reverses for itself.
        let mut folded = Vec::new();
        if let Ok(list) = store.list_runs() {
            for (run_id, feature, created_at) in list {
                // The same fleet grace the table gives: a run whose
                // journal does not fold is a quarantined row carrying
                // the fold error, never a missing row.
                let folded_run = store
                    .load(&run_id)
                    .ok()
                    .map(|events| crate::fold_or_quarantine(&events));
                folded.push((run_id, feature, created_at, folded_run));
            }
        }
        let entries: Vec<forge_view::RunEntry> = folded
            .iter()
            .map(
                |(run_id, feature, created_at, folded_run)| forge_view::RunEntry {
                    run_id,
                    feature,
                    created_at,
                    state: folded_run.as_ref().and_then(|folded| folded.as_ref().ok()),
                    detail: folded_run
                        .as_ref()
                        .and_then(|folded| folded.as_ref().err())
                        .map(String::as_str),
                },
            )
            .collect();
        let view = forge_view::run_rows(&entries);
        return ok(
            "application/json",
            serde_json::to_string(&view.runs).expect("run rows serialize"),
        );
    }
    if let Some(run_id) = path.strip_prefix("/api/view/") {
        let events = match store.load(run_id) {
            Ok(events) => events,
            Err(_) => return not_found(run_id),
        };
        let state = fold(&events).ok();
        let view = forge_view::run_view(&events, state.as_ref());
        return ok(
            "application/json",
            serde_json::to_string(&view).expect("the run view serializes"),
        );
    }
    if let Some(run_id) = path.strip_prefix("/api/run/") {
        let events = match store.load(run_id) {
            Ok(events) => events,
            Err(_) => return not_found(run_id),
        };
        let state = fold(&events).ok();
        let body = json!({
            "summary": state.map(|s| json!({
                "run_id": s.run_id,
                "status": status_str(&s.status),
                "phase": s.phase,
                "seq": s.seq,
                "park_reason": s.park_reason,
                "consecutive_failures": s.consecutive_failures,
                "last_decision": s.last_decision,
                "feature": s.feature,
            })),
            "events": events,
        });
        return ok("application/json", body.to_string());
    }
    not_found(path)
}

/// One block of a turn: prose, or a tool marker carrying a file target.
pub(crate) struct Block {
    pub kind: &'static str,
    pub text: String,
}

/// One transcript turn as the surfaces show it. Two plain structs: no
/// `serde` derive, so the TUI reaches the transcript without the console
/// gaining a dependency edge or the JSON body gaining a definition.
pub(crate) struct Turn {
    pub role: String,
    pub ts: String,
    pub blocks: Vec<Block>,
}

/// The id is joined into `~/.claude/projects/*/<id>.jsonl`, so validating
/// it is a path-traversal guard and belongs with the lookup, never in a
/// caller. Both surfaces call [`session_turns`], which calls this first.
///
/// It is also a DISPLAY guard, which is why it is public: the id is a
/// raw journal string, and every surface renders it inside a
/// `claude --resume <id>` line an operator is invited to paste. Control
/// characters alone are not enough — `;`, `&&`, `$(…)` and backticks
/// survive sanitizing, so a hostile seat could otherwise hand the
/// operator a pasteable shell command. Ids that fail this render as the
/// deliberate-absence mark instead.
pub fn valid_session_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
}

/// Validation, then location: where this session's transcript lives on
/// the operator's machine, or `None` for "there is no such file here".
/// The id is checked BEFORE any path is formed, which is what makes the
/// guard a traversal guard rather than a lookup with a comment beside
/// it. Both readers of a transcript — the parse below and the growth
/// watch — come through here, so neither can form a path of its own.
pub(crate) fn transcript_path(id: &str) -> Option<PathBuf> {
    if !valid_session_id(id) {
        return None;
    }
    let home = std::env::var_os("HOME")?;
    let projects = Path::new(&home).join(".claude").join("projects");
    let file_name = format!("{id}.jsonl");
    for dir in std::fs::read_dir(&projects).ok()?.flatten() {
        let candidate = dir.path().join(&file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The transcript's size on disk, and 0 when it cannot be read at all —
/// a transcript that vanished mid-watch has not grown.
pub(crate) fn transcript_len(file: &Path) -> u64 {
    std::fs::metadata(file).map(|meta| meta.len()).unwrap_or(0)
}

/// Growth as a predicate rather than a comparison buried in a loop, so
/// the rule has its own test: the FIRST observation is never growth —
/// the reader already holds the file as it stood — and a file that
/// shrank (rotated, replaced, gone) has not grown either. Prose is only
/// ever appended, so length is the whole signal.
pub(crate) fn transcript_grew(previous: Option<u64>, current: u64) -> bool {
    previous.is_some_and(|previous| current > previous)
}

/// Validation, location and parse, together: the seat session's local
/// transcript by id, plus whether the size cap truncated it. `None` is
/// "there is no such transcript on this machine" — never a guess.
pub(crate) fn session_turns(id: &str) -> Option<(Vec<Turn>, bool)> {
    let text = std::fs::read_to_string(transcript_path(id)?).ok()?;
    let mut turns = Vec::new();
    let mut budget: usize = 4_000_000;
    let mut truncated = false;
    for line in text.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
        if kind != "user" && kind != "assistant" {
            continue;
        }
        let msg = v.get("message").cloned().unwrap_or(Value::Null);
        let role = msg
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or(kind)
            .to_string();
        let ts = v
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let mut blocks = Vec::new();
        match msg.get("content") {
            Some(Value::String(t)) => blocks.push(Block {
                kind: "text",
                text: t.clone(),
            }),
            Some(Value::Array(parts)) => {
                for part in parts {
                    match part.get("type").and_then(Value::as_str) {
                        Some("text") => {
                            if let Some(t) = part.get("text").and_then(Value::as_str) {
                                if !t.trim().is_empty() {
                                    blocks.push(Block {
                                        kind: "text",
                                        text: t.to_string(),
                                    });
                                }
                            }
                        }
                        Some("tool_use") => {
                            let name = part.get("name").and_then(Value::as_str).unwrap_or("?");
                            let target = part
                                .pointer("/input/file_path")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            let label = if target.is_empty() {
                                name.to_string()
                            } else {
                                format!("{name} · {target}")
                            };
                            blocks.push(Block {
                                kind: "tool",
                                text: label,
                            });
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        if blocks.is_empty() {
            continue;
        }
        let cost: usize = blocks.iter().map(|block| block.text.len()).sum();
        if cost > budget {
            truncated = true;
            break;
        }
        budget -= cost;
        turns.push(Turn { role, ts, blocks });
    }
    Some((turns, truncated))
}

/// Lowest-level drilldown: the seat session's transcript, located in
/// the operator's local Claude projects directory by session id. The
/// id is strictly validated before any path is formed; the response
/// carries prose text and tool names (file targets only), size-capped.
/// This is a loopback-only, operator-local surface — the same trust as
/// `claude --resume <id>` in a terminal.
fn session_transcript(id: &str) -> Response {
    // The two misses read differently to the operator, and that is the
    // only reason the validity question is asked here as well: the guard
    // that matters lives inside `session_turns`.
    if !valid_session_id(id) {
        return not_found("session");
    }
    let Some((turns, truncated)) = session_turns(id) else {
        return not_found("transcript");
    };
    let turns: Vec<Value> = turns
        .iter()
        .map(|turn| {
            let blocks: Vec<Value> = turn
                .blocks
                .iter()
                .map(|block| json!({"kind": block.kind, "text": block.text}))
                .collect();
            json!({"role": turn.role, "ts": turn.ts, "blocks": blocks})
        })
        .collect();
    ok(
        "application/json",
        json!({"session_id": id, "turns": turns, "truncated": truncated}).to_string(),
    )
}

fn head_seq(db: &Path, run_id: &str) -> u64 {
    Store::open(db)
        .and_then(|s| s.head_hash(run_id))
        .map(|(seq, _)| seq)
        .unwrap_or(0)
}

fn read_request(reader: &mut impl BufRead) -> std::io::Result<(String, String, Option<String>)> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let method = line.split_whitespace().next().unwrap_or("").to_string();
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    let mut host: Option<String> = None;
    let mut header = String::new();
    loop {
        header.clear();
        match reader.read_line(&mut header)? {
            0 => break,
            _ if header.trim().is_empty() => break,
            _ => {
                if let Some(value) = header.to_ascii_lowercase().strip_prefix("host:") {
                    host = Some(value.trim().to_string());
                }
            }
        }
    }
    Ok((method, path, host))
}

fn serve_client(db: PathBuf, stream: TcpStream) {
    serve_client_with_limit(db, stream, None)
}

fn serve_client_with_limit(db: PathBuf, stream: TcpStream, sse_limit: Option<usize>) {
    let mut reader = BufReader::new(&stream);
    let mut writer = &stream;
    serve_io(&db, &mut reader, &mut writer, sse_limit);
}

/// One poll of every server-sent stream on this surface. The run's head
/// and a seat's transcript move on the same clock, which is why there is
/// one constant and no second cadence to reason about.
const SSE_POLL: std::time::Duration = std::time::Duration::from_millis(1000);

const SSE_HEADER: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\
                          Cache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n";

fn write_response(stream: &mut impl Write, response: Response) {
    let payload = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n\
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
    sse_limit: Option<usize>,
) {
    let request = read_request(reader);
    let Ok((method, path, host)) = request else {
        return;
    };
    if !request_allowed(&method, host.as_deref()) {
        let _ = stream
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }

    // Before the run's stream, because a run id never contains a slash:
    // the seat's own prose lands BETWEEN journal checkpoints, so the
    // transcript is watched on its own file rather than on the head.
    if let Some(session_id) = path.strip_prefix("/sse/session/") {
        // The id guard and the lookup both run before a single byte of
        // stream is written: an id that cannot name a transcript, or a
        // transcript that is not on this machine, is a clean 404 — never
        // an open connection waiting for a file to appear.
        let Some(file) = transcript_path(session_id) else {
            write_response(stream, not_found("transcript"));
            return;
        };
        if stream.write_all(SSE_HEADER.as_bytes()).is_err() {
            return;
        }
        let mut seen: Option<u64> = None;
        let mut sent = 0usize;
        loop {
            let size = transcript_len(&file);
            let message = if transcript_grew(seen, size) {
                format!("data: {}\n\n", json!({"size": size}))
            } else {
                // Heartbeat comment, exactly as the run's stream: the
                // write that fails is how a closed drilldown is reaped.
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

fn open_system_browser(url: &str) {
    let program = std::env::var("FORGE_BROWSER_BIN").unwrap_or("xdg-open".to_string());
    let _ = std::process::Command::new(program).arg(url).spawn();
}

fn serve_listener(
    db: PathBuf,
    listener: TcpListener,
    open_browser: bool,
    connection_limit: Option<usize>,
    mut opener: impl FnMut(&str),
) -> std::io::Result<()> {
    let bound = listener.local_addr()?.port();
    let url = format!("http://127.0.0.1:{bound}/");
    eprintln!("brokkr ui: {url} (read-only; Ctrl-C to stop)");
    if open_browser {
        opener(&url);
    }
    let limit = connection_limit.unwrap_or(usize::MAX);
    for stream in listener.incoming().flatten().take(limit) {
        let db = db.clone();
        std::thread::spawn(move || serve_client(db, stream));
    }
    Ok(())
}

/// Bind loopback and serve until killed. Returns the bound port (0 in
/// `port` picks an ephemeral one).
pub fn serve(db: PathBuf, port: u16, open_browser: bool) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    serve_listener(db, listener, open_browser, None, open_system_browser)
}

#[cfg(test)]
mod tests;
