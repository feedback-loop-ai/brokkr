//! The Codex thread-record reader: the one walk below `CODEX_HOME`, the
//! non-work row allow-list, and the header correlation every caller uses
//! (decision 0035). It reads only a bounded rollout codex itself wrote.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{codex_error, codex_rate_limit, parse_nanos, reset_windows, system_time_nanos, Now};
use crate::PreSessionRefusal;

/// The closed list of non-work row kinds after this invocation's boundary
/// (#1037, hold c-6761090419). Prompt ingestion is the measured
/// `codex-nomodel.jsonl` shape; any other kind means work may have happened.
const CODEX_NON_WORK: [CodexRow; 8] = [
    CodexRow::SessionMeta,
    CodexRow::PromptMessage,
    CodexRow::PromptItem,
    CodexRow::Context,
    CodexRow::TaskStarted,
    CodexRow::TokenCount,
    CodexRow::Error,
    CodexRow::ErrorComplete,
];

/// One rollout row's kind; `Work` is every other shape, named or not.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CodexRow {
    SessionMeta,
    PromptMessage,
    PromptItem,
    Context,
    TaskStarted,
    TokenCount,
    Error,
    ErrorComplete,
    Work,
}

/// Classify one row by the envelope and payload codex writes.
fn codex_row(row: &Value) -> CodexRow {
    match row.get("type").and_then(Value::as_str) {
        Some("session_meta") => CodexRow::SessionMeta,
        Some("world_state" | "turn_context") => CodexRow::Context,
        Some("error") => CodexRow::Error,
        Some("response_item") => response_item_row(row),
        Some("event_msg") => event_msg_row(row),
        _ => CodexRow::Work,
    }
}

fn response_item_row(row: &Value) -> CodexRow {
    let payload = row.get("payload").unwrap_or(&Value::Null);
    let prompt = payload.get("type").and_then(Value::as_str) == Some("message")
        && matches!(
            payload.get("role").and_then(Value::as_str),
            Some("developer" | "user")
        );
    if prompt {
        CodexRow::PromptMessage
    } else {
        CodexRow::Work
    }
}

fn event_msg_row(row: &Value) -> CodexRow {
    match row.pointer("/payload/type").and_then(Value::as_str) {
        Some("task_started") => CodexRow::TaskStarted,
        Some("token_count") => CodexRow::TokenCount,
        Some("error") => CodexRow::Error,
        Some("task_complete") => complete_row(row),
        Some("item_started" | "item_completed") => prompt_item_row(row),
        _ => CodexRow::Work,
    }
}

/// A completion with the error and no last message is the refusal; a
/// last message as well is work, as the H1 case pins.
fn complete_row(row: &Value) -> CodexRow {
    let payload = row.get("payload").unwrap_or(&Value::Null);
    let error = payload.get("error").is_some_and(|error| !error.is_null());
    let message = payload
        .get("last_agent_message")
        .is_some_and(|message| !message.is_null());
    if error && !message {
        CodexRow::ErrorComplete
    } else {
        CodexRow::Work
    }
}

fn prompt_item_row(row: &Value) -> CodexRow {
    if row.pointer("/payload/item/type").and_then(Value::as_str) == Some("UserMessage") {
        CodexRow::PromptItem
    } else {
        CodexRow::Work
    }
}

/// Read a codex thread record's own error through the adapter's locator
/// (decision 0035): over the whole invocation, every row must be
/// [`CODEX_NON_WORK`], evidence is never reset at a start, a turn started
/// after the error is the H1 notice, and only a usage- or rate-limit error
/// refuses; any other error keeps the result lost. The two facts are kept
/// apart (H3): "an error of any kind was seen" feeds the notice rule, while
/// "a usage- or rate-limit error was seen" is the only one that may refuse.
pub(crate) fn codex_thread_refusal(
    body: &str,
    since_nanos: Option<i128>,
    now: Option<&Now>,
) -> Option<PreSessionRefusal> {
    let mut resets = Vec::new();
    let mut error_seen = false;
    let mut rate_limit_error = None;
    for line in body.lines() {
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            return None; // An unreadable line may be truncated work, never history.
        };
        if !row_in_invocation(&row, since_nanos) {
            continue;
        }
        let kind = codex_row(&row);
        if kind == CodexRow::TaskStarted {
            // A turn started after ANY error means the error was a
            // notice; the invocation continued, so the lost path stands.
            if error_seen {
                return None;
            }
            continue;
        }
        if !CODEX_NON_WORK.contains(&kind) {
            // Work anywhere after the boundary, before or after the
            // error: the seat may have acted and the attempt stays lost.
            return None;
        }
        if let Some(windows) = row
            .pointer("/payload/rate_limits")
            .map(reset_windows)
            .filter(|windows| !windows.is_empty())
        {
            resets = windows;
        }
        if let Some(found) = thread_error(&row) {
            if codex_rate_limit(&found) {
                rate_limit_error = Some(found);
            }
            error_seen = true;
        }
    }
    Some(codex_error(&rate_limit_error?, now, &resets))
}

/// Whether a row is at or after the start; an untimed row is read, never assumed history.
fn row_in_invocation(row: &Value, since_nanos: Option<i128>) -> bool {
    let Some(since_nanos) = since_nanos else {
        return true;
    };
    row.get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_nanos)
        .is_none_or(|at| at >= since_nanos)
}

/// The error a rollout row names, or nothing: the last one after the
/// current turn began is the refusal the early exit left behind.
fn thread_error(row: &Value) -> Option<Value> {
    let payload = match row.get("type").and_then(Value::as_str)? {
        "error" => return Some(row.clone()),
        "event_msg" => row.get("payload")?,
        _ => return None,
    };
    match payload.get("type").and_then(Value::as_str)? {
        "task_complete" => payload.get("error").cloned(),
        "error" => Some(payload.clone()),
        _ => None,
    }
}

/// The same read where the CLI announced no thread, so the locator is
/// empty: every candidate is first correlated by cwd and session start,
/// singleton included, and exactly one correlated file is read.
pub(crate) fn codex_thread_refusal_since(
    root: &Path,
    since_nanos: i128,
    workdir: &str,
    now: Option<&Now>,
    depth: usize,
) -> Option<PreSessionRefusal> {
    let mut found = find_codex_threads_since(root, since_nanos, depth);
    found.retain(|path| {
        thread_header(path, None)
            .is_some_and(|(cwd, started)| cwd == workdir && started >= since_nanos)
    });
    let [path] = found.as_slice() else {
        return None;
    };
    let body = std::fs::read_to_string(path).ok()?;
    codex_thread_refusal(&body, Some(since_nanos), now)
}

/// The thread record whose file name carries `thread_id`, anywhere in a
/// bounded walk below `root`. The id is codex's own announcement, never
/// "the newest file": a concurrent seat's thread must not lend this one
/// its effort (decision 0035).
pub(crate) fn find_codex_thread(root: &Path, thread_id: &str, depth: usize) -> Option<PathBuf> {
    let mut found = None;
    walk_codex_threads(root, depth, &mut |path| {
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| names_codex_thread(name, thread_id));
        if named {
            found = Some(path.to_path_buf());
        }
        named
    });
    found
}

/// Every codex rollout written at or after `since_nanos` in a bounded
/// walk below `root`: which files this invocation could have created.
fn find_codex_threads_since(root: &Path, since_nanos: i128, depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk_codex_threads(root, depth, &mut |path| {
        let written = path
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .and_then(system_time_nanos)
            .is_none_or(|at| at >= since_nanos);
        if written {
            found.push(path.to_path_buf());
        }
        false
    });
    found
}

/// Walk the rollouts below `root`, stopping when `visit` asks to. The
/// one walk both locators use; each judges a file its own way.
fn walk_codex_threads(root: &Path, depth: usize, visit: &mut impl FnMut(&Path) -> bool) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    let mut directories = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            directories.push(path);
            continue;
        }
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".jsonl"));
        if named && visit(&path) {
            return true;
        }
    }
    if depth == 0 {
        return false;
    }
    // Deterministic order: two seats under one home must not disagree.
    directories.sort();
    directories
        .into_iter()
        .any(|directory| walk_codex_threads(&directory, depth - 1, visit))
}

/// `name` carries `thread_id` as a WHOLE token, bounded at both ends by a
/// non-alphanumeric or by the ends of the name, never as a fragment: a
/// fragment is not an announcement, and a bare `contains` would lose the
/// "never by the newest file" rule by another route.
fn names_codex_thread(name: &str, thread_id: &str) -> bool {
    name.match_indices(thread_id).any(|(at, _)| {
        let before = name[..at].chars().next_back();
        let after = name[at + thread_id.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric())
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

/// The cwd and session start a rollout's header names, only when its first
/// row is a `session_meta` with string `cwd`, `timestamp` and `id` in its
/// payload, and the id is this record's own (F4): the announced thread when
/// one is known, else the id its file name carries.
pub(crate) fn thread_header(path: &Path, announced: Option<&str>) -> Option<(String, i128)> {
    let body = std::fs::read_to_string(path).ok()?;
    let first = body.lines().next()?;
    let row = serde_json::from_str::<Value>(first).ok()?;
    if row.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = row.get("payload")?;
    let cwd = payload.get("cwd").and_then(Value::as_str)?.to_string();
    let started = parse_nanos(payload.get("timestamp")?.as_str()?)?;
    let id = payload.get("id").and_then(Value::as_str)?;
    names_own_record(path, id, announced).then_some((cwd, started))
}

/// Whether `id` is the record's own: the announced thread when one is
/// known, else an id its file name carries as a whole token. A mismatch,
/// including an empty id, correlates nothing.
fn names_own_record(path: &Path, id: &str, announced: Option<&str>) -> bool {
    if id.is_empty() {
        return false;
    }
    announced.map_or_else(
        || {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| names_codex_thread(name, id))
        },
        |announced| announced == id,
    )
}
