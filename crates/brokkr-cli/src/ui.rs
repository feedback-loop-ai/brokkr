//! `brokkr ui` — the embedded read-only surface (decision 0003): a static
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
//! `/api/runs` and `/api/view/<id>` serve `brokkr-view`'s models: the page
//! paints them and derives nothing (decision 0013). `/api/run/<id>` keeps
//! its raw summary-and-events shape as the parity baseline.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};

use brokkr_core::fold::{fold, Status};
use brokkr_store::Store;
use brokkr_view::transcript::{
    LegacyProvenance, Snapshot, TranscriptKind, TranscriptRead, Unavailable, ValidReference,
};
use serde_json::{json, Value};

const PAGE: &str = include_str!("ui.html");

mod safe_fs;

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
    let store = match Store::open_read_only(db) {
        Ok(store) => store,
        Err(e) => {
            return Response {
                status: "500 Internal Server Error",
                content_type: "application/json",
                body: json!({"error": e.to_string()}).to_string(),
            }
        }
    };
    if let Some(rest) = path.strip_prefix("/api/presentation/") {
        return participant_presentation(&store, rest);
    }
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
                let (folded_run, residuals) = crate::listed_run(&store, &run_id);
                folded.push((run_id, feature, created_at, folded_run, residuals));
            }
        }
        let entries: Vec<brokkr_view::RunEntry> = folded
            .iter()
            .map(
                |(run_id, feature, created_at, folded_run, residuals)| brokkr_view::RunEntry {
                    run_id,
                    feature,
                    created_at,
                    state: folded_run.as_ref().and_then(|folded| folded.as_ref().ok()),
                    detail: folded_run
                        .as_ref()
                        .and_then(|folded| folded.as_ref().err())
                        .map(String::as_str),
                    residuals,
                },
            )
            .collect();
        let view = brokkr_view::run_rows(&entries);
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
        let view = brokkr_view::run_view(&events, state.as_ref());
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

/// Lowest-level drilldown: the seat session's transcript, located in
/// the operator's local Claude projects directory by session id. The
/// id is strictly validated before any path is formed; the response
/// carries prose text and tool names (file targets only), size-capped.
/// This is a loopback-only, operator-local surface — the same trust as
/// `claude --resume <id>` in a terminal.
fn session_transcript(id: &str) -> Response {
    // The two misses read differently to the operator, and that is the
    // only reason the validity question is asked here as well: the guard
    // that matters lives inside the shared reader.
    if !brokkr_view::transcript::valid_claude_id(id) {
        return not_found("session");
    }
    let read = read_local(None, LegacyProvenance::Claude, Some(id));
    if !read.is_readable() {
        return not_found("transcript");
    }
    let turns: Vec<Value> = read
        .turns
        .iter()
        .map(|turn| {
            let blocks: Vec<Value> = turn
                .blocks
                .iter()
                .map(|block| json!({"kind": block.kind.as_str(), "text": block.text}))
                .collect();
            json!({"role": turn.role, "ts": turn.ts, "blocks": blocks})
        })
        .collect();
    ok(
        "application/json",
        json!({"session_id": id, "turns": turns, "truncated": read.truncated}).to_string(),
    )
}

/// The local Claude projects root a legacy flat id is synthesized
/// against, or `None` when there is no usable `HOME`.
pub(crate) fn local_projects_home() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    Path::new(&home)
        .join(".claude")
        .join("projects")
        .to_str()
        .map(str::to_string)
}

/// A safely opened candidate source: the lossless confirmed path and the
/// verified handle retained for the body read.
pub(crate) struct AdmittedSource {
    pub path: String,
    pub file: safe_fs::OpenedFile,
    /// The leaf's lossless identity, recorded at discovery and rechecked
    /// through the same held handle at the read boundary.
    pub identity: safe_fs::Identity,
}

/// The outcome of safe discovery, before any body byte is interpreted.
pub(crate) enum Discovery {
    Admitted(AdmittedSource),
    Refused(Unavailable, Option<String>),
}

/// One lookup's collected facts, resolved after the bounded scope is
/// exhausted so iterator order never chooses the explanation.
#[derive(Default)]
struct Lookup {
    candidates: Vec<AdmittedSource>,
    examined: usize,
    unsafe_seen: bool,
    io_seen: bool,
    limit_hit: bool,
    invalid_depth_seen: bool,
}

impl Lookup {
    /// Entries still chargeable to the one 10,000-entry discovery budget.
    fn remaining(&self) -> usize {
        brokkr_view::transcript::DISCOVERY_LIMIT.saturating_sub(self.examined)
    }

    /// Charge one directory entry that was actually examined.
    fn note_entry(&mut self) {
        self.examined += 1;
    }

    fn resolve(mut self, kind: TranscriptKind) -> Discovery {
        if self.limit_hit && self.candidates.len() < 2 {
            return Discovery::Refused(Unavailable::DiscoveryLimit, None);
        }
        if self.candidates.len() > 1 {
            return Discovery::Refused(Unavailable::AmbiguousSource, None);
        }
        // An unreadable sibling prevents proving that a sole candidate is
        // unique, so I/O failure outranks a provisional admission.
        if self.io_seen {
            return Discovery::Refused(Unavailable::Unreadable, None);
        }
        if let Some(candidate) = self.candidates.pop() {
            return Discovery::Admitted(candidate);
        }
        if self.unsafe_seen {
            return Discovery::Refused(Unavailable::UnsafePath, None);
        }
        if kind == TranscriptKind::DshSession && self.invalid_depth_seen {
            return Discovery::Refused(
                Unavailable::NotFound,
                Some("no valid depth-zero DSH session header".to_string()),
            );
        }
        Discovery::Refused(Unavailable::NotFound, None)
    }
}

/// Enumerate one directory under the remaining discovery budget. A
/// listing larger than the budget sets the limit fact without
/// materializing the extra names; an I/O failure is recorded and yields
/// `None`, ending this lookup.
fn bounded_entries(dir: &safe_fs::Dir, lookup: &mut Lookup) -> Option<Vec<std::ffi::OsString>> {
    match dir.entries_bounded(lookup.remaining()) {
        Ok((names, truncated)) => {
            if truncated {
                lookup.limit_hit = true;
            }
            Some(names)
        }
        Err(_) => {
            lookup.io_seen = true;
            None
        }
    }
}

fn root_error(error: &std::io::Error) -> Discovery {
    let reason = if error.kind() == std::io::ErrorKind::NotFound {
        Unavailable::NotFound
    } else {
        Unavailable::Unreadable
    };
    Discovery::Refused(reason, None)
}

/// Whether a recorded home uses the current target's native absolute
/// syntax, so that a foreign-platform spelling is refused before any
/// filesystem call rather than reinterpreted as a host-relative path.
#[cfg(unix)]
fn native_absolute_home(home: &str) -> bool {
    home.starts_with('/')
}

#[cfg(windows)]
fn native_absolute_home(home: &str) -> bool {
    let bytes = home.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/'))
        || ((home.starts_with("\\\\") || home.starts_with("//")) && home.len() > 2)
}

/// Locate one safely opened local source for a validated reference. No
/// transcript content is read for Claude or Codex; a DSH candidate's
/// bounded opening header is read to prove depth-zero ownership. The
/// recorded home is canonicalized once (it may itself be a symlink) and
/// only that canonical directory is opened as the traversal root.
fn discover(reference: &ValidReference) -> Discovery {
    // Lexical validity is portable; native I/O is not. A recorded home
    // spelled in another platform's absolute syntax is refused here,
    // before canonicalization, and its bytes stay echoed unchanged.
    if !native_absolute_home(&reference.home) {
        return Discovery::Refused(Unavailable::InvalidReference, None);
    }
    let canonical = match std::fs::canonicalize(&reference.home) {
        Ok(path) => path,
        Err(error) => return root_error(&error),
    };
    let Some(home) = canonical.to_str() else {
        // A non-Unicode root has no lossless output spelling.
        return Discovery::Refused(Unavailable::Unreadable, None);
    };
    let root = match safe_fs::Dir::open_root(home) {
        Ok(root) => root,
        Err(error) => return root_error(&error),
    };
    let mut lookup = Lookup::default();
    match reference.kind {
        TranscriptKind::ClaudeSession => discover_claude(&root, home, reference, &mut lookup),
        TranscriptKind::CodexThread => discover_codex(&root, home, reference, &mut lookup),
        TranscriptKind::DshSession => discover_dsh(&root, home, reference, &mut lookup),
        TranscriptKind::None => {}
    }
    lookup.resolve(reference.kind)
}

/// D3: an acquisition is current only when a fresh handle-relative walk
/// finds the same unique safe candidate at the same recorded path and
/// identity. A swapped leaf or ancestor name, a changed DSH opening
/// header or a newly ambiguous root fails closed before any byte of the
/// retained handle is read.
fn acquisition_is_current(reference: &ValidReference, admitted: &AdmittedSource) -> bool {
    // The point before the read-boundary acquisition re-walk. A test-owned
    // change is timed here so the real re-walk observes it; the seam never
    // replaces the re-walk's answer.
    #[cfg(test)]
    safe_fs::fault::point(safe_fs::fault::ChangeAt::BeforeRewalk);
    matches!(
        discover(reference),
        Discovery::Admitted(recheck)
            if recheck.identity == admitted.identity && recheck.path == admitted.path
    )
}

fn join_path(base: &str, name: &str) -> String {
    format!("{base}/{name}")
}

fn discover_claude(
    root: &safe_fs::Dir,
    home: &str,
    reference: &ValidReference,
    lookup: &mut Lookup,
) {
    let file_name = format!("{}.jsonl", reference.locator);
    let Some(entries) = bounded_entries(root, lookup) else {
        return;
    };
    for name in entries {
        lookup.note_entry();
        let Some(name) = name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        let project = match root.child(std::ffi::OsStr::new(name)) {
            Ok(safe_fs::Child::Dir(dir)) => dir,
            Ok(safe_fs::Child::Unsafe) => {
                lookup.unsafe_seen = true;
                continue;
            }
            Ok(_) => continue,
            Err(_) => {
                lookup.io_seen = true;
                continue;
            }
        };
        match project.child(std::ffi::OsStr::new(&file_name)) {
            Ok(safe_fs::Child::File(file)) => match file.identity() {
                Ok(identity) => lookup.candidates.push(AdmittedSource {
                    path: join_path(&join_path(home, name), &file_name),
                    identity,
                    file,
                }),
                Err(_) => lookup.io_seen = true,
            },
            Ok(safe_fs::Child::Unsafe) => lookup.unsafe_seen = true,
            Ok(_) => {}
            Err(_) => lookup.io_seen = true,
        }
    }
}

/// The whole-filename token predicate for a Codex rollout.
fn codex_filename_matches(filename: &str, id: &str) -> bool {
    if !filename.starts_with("rollout-") || !filename.ends_with(".jsonl") {
        return false;
    }
    let bytes = filename.as_bytes();
    let id = id.as_bytes();
    if id.is_empty() || id.len() > bytes.len() {
        return false;
    }
    for start in 0..=(bytes.len() - id.len()) {
        if &bytes[start..start + id.len()] != id {
            continue;
        }
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after = start + id.len();
        let after_ok = after == bytes.len() || !bytes[after].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn discover_codex(
    root: &safe_fs::Dir,
    home: &str,
    reference: &ValidReference,
    lookup: &mut Lookup,
) {
    lookup.note_entry();
    let sessions = match root.child(std::ffi::OsStr::new("sessions")) {
        Ok(safe_fs::Child::Dir(dir)) => dir,
        Ok(safe_fs::Child::Unsafe) => {
            lookup.unsafe_seen = true;
            return;
        }
        Ok(_) => return,
        Err(_) => {
            lookup.io_seen = true;
            return;
        }
    };
    walk_codex(
        &sessions,
        &join_path(home, "sessions"),
        0,
        reference,
        lookup,
    );
}

fn walk_codex(
    dir: &safe_fs::Dir,
    path: &str,
    depth: usize,
    reference: &ValidReference,
    lookup: &mut Lookup,
) {
    let Some(entries) = bounded_entries(dir, lookup) else {
        return;
    };
    let mut subdirs: Vec<(String, safe_fs::Dir)> = Vec::new();
    for name in entries {
        lookup.note_entry();
        let Some(name_str) = name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        // Open and classify the child before applying the rollout filename
        // predicate: a regular file may match, while an opened directory
        // remains traversable within depth six even when its own name
        // resembles `rollout-*.jsonl`.
        match dir.child(std::ffi::OsStr::new(name_str)) {
            Ok(safe_fs::Child::File(file)) => {
                if codex_filename_matches(name_str, &reference.locator) {
                    match file.identity() {
                        Ok(identity) => lookup.candidates.push(AdmittedSource {
                            path: join_path(path, name_str),
                            identity,
                            file,
                        }),
                        Err(_) => lookup.io_seen = true,
                    }
                }
            }
            Ok(safe_fs::Child::Dir(sub)) => {
                if depth < 6 {
                    subdirs.push((name_str.to_string(), sub));
                }
            }
            Ok(safe_fs::Child::Unsafe) => lookup.unsafe_seen = true,
            Ok(safe_fs::Child::Absent) => {}
            Err(_) => lookup.io_seen = true,
        }
    }
    for (name, sub) in subdirs {
        walk_codex(&sub, &join_path(path, &name), depth + 1, reference, lookup);
    }
}

/// The classification of a DSH candidate's bounded opening record. The
/// variants keep an invalid-depth session header distinct from an absent
/// or malformed opening row and from a header that outgrew the cap, so
/// discovery can resolve the exact specified outcome and explanation.
enum HeaderCheck {
    Valid,
    InvalidDepth,
    NotSessionHeader,
    TooLarge,
    Io,
}

/// Read at most the bounded first record from a DSH candidate and admit
/// it only as an opening `session` object with absent or unsigned-zero
/// depth. The version field neither qualifies nor vetoes ownership.
fn dsh_header(file: &safe_fs::OpenedFile) -> HeaderCheck {
    // The header budget is DSH_HEADER_CAP bytes; read one further byte so
    // a newline that immediately follows a header of exactly the cap is
    // seen instead of being discarded as the overflow probe.
    let (bytes, overflow, _eof) =
        match file.read_bounded(brokkr_view::transcript::DSH_HEADER_CAP as u64 + 1) {
            Ok(read) => read,
            Err(_) => return HeaderCheck::Io,
        };
    let header = match bytes.iter().position(|byte| *byte == b'\n') {
        Some(end) => &bytes[..end],
        None => {
            // No complete first record inside the cap: either the header
            // itself exceeds the bound, or the whole file was read.
            if overflow {
                return HeaderCheck::TooLarge;
            }
            &bytes[..]
        }
    };
    let Ok(text) = std::str::from_utf8(header) else {
        // Invalid UTF-8 prevents establishing a unique ownership answer.
        return HeaderCheck::Io;
    };
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return HeaderCheck::NotSessionHeader;
    };
    let Some(object) = value.as_object() else {
        return HeaderCheck::NotSessionHeader;
    };
    if object.get("type").and_then(Value::as_str) != Some("session") {
        return HeaderCheck::NotSessionHeader;
    }
    match object.get("delegationDepth") {
        // Omitted depth retains the legacy meaning of zero.
        None => {}
        Some(depth) if depth.as_u64() == Some(0) => {}
        // Null, booleans, strings, negatives and floats are invalid.
        Some(_) => return HeaderCheck::InvalidDepth,
    }
    HeaderCheck::Valid
}

/// Open one locator component from the previous directory, charging it to
/// the discovery budget and resolving the shared child arms. `None` ends
/// the lookup: an unsafe child, a non-directory child or an I/O failure.
fn dsh_step(parent: &safe_fs::Dir, component: &str, lookup: &mut Lookup) -> Option<safe_fs::Dir> {
    lookup.note_entry();
    match parent.child(std::ffi::OsStr::new(component)) {
        Ok(safe_fs::Child::Dir(dir)) => Some(dir),
        Ok(safe_fs::Child::Unsafe) => {
            lookup.unsafe_seen = true;
            None
        }
        Ok(_) => None,
        Err(_) => {
            lookup.io_seen = true;
            None
        }
    }
}

fn discover_dsh(root: &safe_fs::Dir, home: &str, reference: &ValidReference, lookup: &mut Lookup) {
    // The recorded locator is a relative component path. `split_once`
    // separates the first component, which opens from the root; each later
    // component opens from the previous step's directory. The base is
    // always the last directory a step opened, so nothing is left unset:
    // `split_once` yields at least one component for any string, and once
    // the loop starts it either returns or stores the next directory. The
    // removed post-walk `else` was unreachable for exactly that reason.
    let mut base_path = home.to_string();
    let (first, rest) = match reference.locator.split_once('/') {
        Some((first, rest)) => (first, Some(rest)),
        None => (reference.locator.as_str(), None),
    };
    let Some(mut base) = dsh_step(root, first, lookup) else {
        return;
    };
    base_path = join_path(&base_path, first);
    if let Some(rest) = rest {
        for component in rest.split('/') {
            let Some(next) = dsh_step(&base, component, lookup) else {
                return;
            };
            base_path = join_path(&base_path, component);
            base = next;
        }
    }
    let Some(projects) = bounded_entries(&base, lookup) else {
        return;
    };
    for project_name in projects {
        lookup.note_entry();
        let Some(project_name) = project_name.to_str() else {
            lookup.io_seen = true;
            continue;
        };
        let project = match base.child(std::ffi::OsStr::new(project_name)) {
            Ok(safe_fs::Child::Dir(dir)) => dir,
            Ok(safe_fs::Child::Unsafe) => {
                lookup.unsafe_seen = true;
                continue;
            }
            Ok(_) => continue,
            Err(_) => {
                lookup.io_seen = true;
                continue;
            }
        };
        let project_path = join_path(&base_path, project_name);
        let Some(sessions) = bounded_entries(&project, lookup) else {
            continue;
        };
        for session_name in sessions {
            lookup.note_entry();
            let Some(session_name) = session_name.to_str() else {
                lookup.io_seen = true;
                continue;
            };
            let session = match project.child(std::ffi::OsStr::new(session_name)) {
                Ok(safe_fs::Child::Dir(dir)) => dir,
                Ok(safe_fs::Child::Unsafe) => {
                    lookup.unsafe_seen = true;
                    continue;
                }
                Ok(_) => continue,
                Err(_) => {
                    lookup.io_seen = true;
                    continue;
                }
            };
            // The admitted names, newest first. A session directory yields
            // at most one candidate: the first admitted name carrying a
            // valid header, so a directory holding both admits the newer
            // and never becomes ambiguous with itself. Only an absent file
            // or a row that is not a session header moves on to the next
            // name; every other outcome is decisive for this directory.
            for session_file in DSH_SESSION_FILES {
                let mut decided = true;
                match session.child(std::ffi::OsStr::new(session_file)) {
                    Ok(safe_fs::Child::File(file)) => match dsh_header(&file) {
                        HeaderCheck::Valid => match file.identity() {
                            Ok(identity) => lookup.candidates.push(AdmittedSource {
                                path: join_path(
                                    &join_path(&project_path, session_name),
                                    session_file,
                                ),
                                identity,
                                file,
                            }),
                            Err(_) => lookup.io_seen = true,
                        },
                        HeaderCheck::InvalidDepth => lookup.invalid_depth_seen = true,
                        HeaderCheck::NotSessionHeader => decided = false,
                        HeaderCheck::TooLarge => lookup.limit_hit = true,
                        HeaderCheck::Io => lookup.io_seen = true,
                    },
                    Ok(safe_fs::Child::Unsafe) => lookup.unsafe_seen = true,
                    Ok(_) => decided = false,
                    Err(_) => lookup.io_seen = true,
                }
                if decided {
                    break;
                }
            }
        }
    }
}

/// The session filenames DSH discovery admits, newest first. The core
/// versions this file: 0.1.5-rc.1 writes `session.v3.jsonl` where earlier
/// cores wrote `session.jsonl`, and a reader that knows only one name loses
/// exactly the newest evidence. The set is closed rather than a glob so
/// that admitting a name stays a decision with evidence behind it, and
/// ordered so the choice between two present files is stated rather than
/// incidental.
const DSH_SESSION_FILES: [&str; 2] = ["session.v3.jsonl", "session.jsonl"];

/// Select, validate, safely discover, bound-read and project one local
/// transcript: the one read path every local surface consumes (D2).
pub fn read_local(
    common: Option<&brokkr_view::Transcript>,
    provenance: LegacyProvenance,
    legacy_id: Option<&str>,
) -> TranscriptRead {
    let local = local_projects_home();
    read_with_home(common, provenance, legacy_id, local.as_deref())
}

fn read_with_home(
    common: Option<&brokkr_view::Transcript>,
    provenance: LegacyProvenance,
    legacy_id: Option<&str>,
    local_projects: Option<&str>,
) -> TranscriptRead {
    let selection =
        brokkr_view::transcript::select_reference(common, provenance, legacy_id, local_projects);
    let valid = match &selection.outcome {
        Ok(valid) => valid.clone(),
        Err(reason) => {
            return TranscriptRead::refused(
                selection.reference.clone(),
                selection.legacy,
                *reason,
                brokkr_view::transcript::explanation_for(*reason),
                None,
                false,
                0,
                0,
                None,
            )
        }
    };
    match discover(&valid) {
        Discovery::Refused(reason, explanation) => {
            let hint = brokkr_view::transcript::full_session(&valid, None);
            TranscriptRead::refused(
                selection.reference.clone(),
                selection.legacy,
                reason,
                explanation.unwrap_or_else(|| brokkr_view::transcript::explanation_for(reason)),
                None,
                false,
                0,
                0,
                hint,
            )
        }
        Discovery::Admitted(source) => {
            let source_identity = brokkr_view::transcript::SourceIdentity {
                device: source.identity.device,
                inode: source.identity.inode,
            };
            let hint = brokkr_view::transcript::full_session(&valid, Some(&source.path));
            // Re-derive the unique safe candidate through the retained
            // root and require the same recorded path and identity: a
            // swapped name, changed opening header or new ambiguity fails
            // closed rather than reading the old handle's bytes.
            if !acquisition_is_current(&valid, &source) {
                return refused_source(&selection, source.path, hint);
            }
            // Recheck the held leaf's checked lossless identity at the read
            // boundary. The handle is already the verified source, so this
            // never reopens a display path; a widening failure or a
            // mismatch is a bounded fail-closed `unreadable`.
            if source.file.identity().ok() != Some(source.identity) {
                return refused_source(&selection, source.path, hint);
            }
            let (bytes, overflow, eof) = match source
                .file
                .read_bounded(brokkr_view::transcript::SOURCE_CAP)
            {
                Ok(read) => read,
                Err(_) => return refused_source(&selection, source.path, hint),
            };
            let snapshot = Snapshot {
                bytes: &bytes,
                overflow,
                eof,
            };
            // `project` re-admits the snapshot; a UTF-8 failure becomes an
            // `unreadable` projection with zero counts and source-only
            // truncation, exactly as the failure-stage matrix fixes.
            let projection = brokkr_view::transcript::project(valid.kind, &snapshot);
            if let Some(reason) = projection.unavailable {
                let mut read = TranscriptRead::refused(
                    selection.reference.clone(),
                    selection.legacy,
                    reason,
                    brokkr_view::transcript::explanation_for(reason),
                    Some(source.path),
                    projection.truncated,
                    projection.skipped_lines,
                    projection.unrecognized_records,
                    hint,
                );
                read.source_identity = Some(source_identity);
                return read;
            }
            let mut read = TranscriptRead::readable(
                selection.reference.clone(),
                selection.legacy,
                valid.kind,
                Some(source.path),
                projection.turns,
                projection.truncated,
                projection.skipped_lines,
                projection.unrecognized_records,
            );
            read.source_identity = Some(source_identity);
            read
        }
    }
}

/// The fail-closed read-boundary refusal: a fresh acquisition that no
/// longer matches the admitted source, a held-handle identity that cannot
/// be rechecked, or a bounded read that failed all present the same
/// `unreadable` result with the recorded path and hint.
fn refused_source(
    selection: &brokkr_view::transcript::Selection,
    path: String,
    hint: Option<String>,
) -> TranscriptRead {
    TranscriptRead::refused(
        selection.reference.clone(),
        selection.legacy,
        Unavailable::Unreadable,
        brokkr_view::transcript::explanation_for(Unavailable::Unreadable),
        Some(path),
        false,
        0,
        0,
        hint,
    )
}

/// One safely discovered Claude source by flat id: the journal-independent
/// lookup the API, SSE and browser routes share. Each call revalidates
/// unique safe discovery rather than trusting an earlier spelling.
#[allow(dead_code)] // consumed by the SSE/watch and browser routes as they land
pub(crate) fn claude_source(id: &str, home: Option<&str>) -> Discovery {
    // The projects home arrives as a parameter, the same split
    // `read_local`/`read_with_home` already uses, so the missing-home arm is
    // testable without mutating the process environment.
    let Some(home) = home else {
        return Discovery::Refused(Unavailable::MissingHome, None);
    };
    if !brokkr_view::transcript::valid_claude_id(id) {
        return Discovery::Refused(Unavailable::InvalidReference, None);
    }
    let valid = ValidReference {
        kind: TranscriptKind::ClaudeSession,
        locator: id.to_string(),
        home: home.to_string(),
    };
    discover(&valid)
}

/// The current size of the unique safely discovered Claude source for a
/// flat id, or `None` when the shared guard and discovery admit nothing.
/// The size event keeps its shipped `{"size": n}` shape; the size is
/// measured through the retained handle, never a reopened pathname.
fn claude_source_size(id: &str) -> Option<u64> {
    match claude_source(id, local_projects_home().as_deref()) {
        Discovery::Admitted(source) => Some(source.file.len()),
        Discovery::Refused(..) => None,
    }
}

/// The provenance bridge the presentation route shares with the command:
/// Claude, LaneTally and an inline pre-0032 seat may fall back to a local
/// Claude id, while explicit Codex/DSH provenance refuses synthesis.
fn presentation_provenance(participant: &brokkr_view::Participant) -> LegacyProvenance {
    match participant
        .provenance
        .as_ref()
        .map(|provenance| provenance.provider.as_str())
    {
        None => LegacyProvenance::Absent,
        Some("claude") => LegacyProvenance::Claude,
        Some("lanetally") => LegacyProvenance::LaneTally,
        Some(_) => LegacyProvenance::Other,
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

/// The server's local Claude projects root, canonicalized. Canonicalizing
/// is the only way "the same recorded home" is established: a recorded
/// home that cannot be canonicalized is not the local projects home.
fn canonical_local_projects() -> Option<PathBuf> {
    std::fs::canonicalize(local_projects_home()?).ok()
}

/// True only when the recorded home and the local projects home are the
/// same canonical directory.
fn same_canonical_home(recorded: &str, local: &Option<PathBuf>) -> bool {
    let Some(local) = local else { return false };
    std::fs::canonicalize(recorded)
        .map(|path| path == *local)
        .unwrap_or(false)
}

/// The CLI-private, prose-free participant presentation (D9): the
/// selected reference, its legacy flag, admission state, discovery-stage
/// unavailability, the shared hint and Claude drill eligibility. It is
/// constructed from selection, validation, bounded safe discovery and
/// hint helpers only — never from body bytes or a content projector.
fn presentation_payload(
    common: Option<&brokkr_view::Transcript>,
    provenance: LegacyProvenance,
    legacy_id: Option<&str>,
) -> Value {
    let selection = brokkr_view::transcript::select_reference(
        common,
        provenance,
        legacy_id,
        local_projects_home().as_deref(),
    );
    let (admitted, reason, explanation, path, valid) = match &selection.outcome {
        Ok(valid) => match discover(valid) {
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
    let local = canonical_local_projects();
    let drill_eligible = valid.as_ref().is_some_and(|valid| {
        valid.kind == TranscriptKind::ClaudeSession && same_canonical_home(&valid.home, &local)
    });
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

/// One GET participant-presentation route, keyed by a full run id and an
/// encoded participant key. Each path component is decoded exactly once;
/// a malformed escape or any extra component is refused before the
/// read-only journal is touched. No path or home override is accepted.
fn participant_presentation(store: &Store, rest: &str) -> Response {
    let mut components = rest.split('/');
    let (Some(run_id), Some(participant_key), None) =
        (components.next(), components.next(), components.next())
    else {
        return not_found("participant");
    };
    if run_id.is_empty() || participant_key.is_empty() {
        return not_found("participant");
    }
    let (Some(run_id), Some(participant_key)) =
        (decode_component(run_id), decode_component(participant_key))
    else {
        return not_found("participant");
    };
    // The post-decode emptiness check is removed as unreachable:
    // `decode_component` appends exactly one byte per input byte or `%XX`
    // triple, so a non-empty component decodes to a non-empty byte string;
    // `String::from_utf8` of a non-empty vector is non-empty or `None`; and
    // the pre-decode check above already refused an empty component.
    let events = match store.load(&run_id) {
        Ok(events) => events,
        Err(_) => return not_found("participant"),
    };
    let state = fold(&events).ok();
    let view = brokkr_view::run_view(&events, state.as_ref());
    let Some(participant) = view
        .participants
        .iter()
        .find(|participant| participant.key == participant_key)
    else {
        return not_found("participant");
    };
    let payload = presentation_payload(
        participant.transcript.as_ref(),
        presentation_provenance(participant),
        participant.session_id.as_deref(),
    );
    ok("application/json", payload.to_string())
}

fn head_seq(db: &Path, run_id: &str) -> u64 {
    Store::open_read_only(db)
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
        // The shared identifier guard and safe discovery both run before a
        // single byte of stream is written: an id that cannot name a
        // transcript, or a transcript that is not on this machine, is a
        // clean 404 — never an open connection waiting for a file to
        // appear. Every refusal, an invalid id included, is
        // `{"error":"transcript not found"}`.
        if claude_source_size(session_id).is_none() {
            write_response(stream, not_found("transcript"));
            return;
        }
        if stream.write_all(SSE_HEADER.as_bytes()).is_err() {
            return;
        }
        let mut seen: Option<u64> = None;
        let mut sent = 0usize;
        loop {
            // Unique safe discovery is revalidated on every poll, not
            // trusted from the admitted first look. Losing it closes the
            // stream without reporting another size.
            let Some(size) = claude_source_size(session_id) else {
                return;
            };
            let message = if seen.is_some_and(|previous| size > previous) {
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
    let program = brokkr_protocol::legacy::env("BROKKR_BROWSER_BIN", Some("FORGE_BROWSER_BIN"))
        .unwrap_or("xdg-open".to_string());
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
