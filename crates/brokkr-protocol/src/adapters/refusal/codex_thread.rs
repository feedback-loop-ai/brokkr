//! The Codex thread-record reader: the one walk below `CODEX_HOME`, the
//! non-work row allow-list, and the header correlation every caller uses
//! (decision 0035). It reads only a bounded rollout codex itself wrote.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::super::Ident;
use super::{
    codex_class, codex_error, exhausted_reset, reset_windows, system_time_nanos, Now, Row,
};
use crate::secret::BoundSecret;
use crate::{PreSessionRefusal, RefusalClass};

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

#[derive(Clone, Copy)]
enum Envelope {
    Header,
    Context,
    Error,
    ResponseItem,
    EventMsg,
}

const ENVELOPES: &[(&str, Envelope)] = &[
    ("session_meta", Envelope::Header),
    ("world_state", Envelope::Context),
    ("turn_context", Envelope::Context),
    ("error", Envelope::Error),
    ("response_item", Envelope::ResponseItem),
    ("event_msg", Envelope::EventMsg),
];

#[derive(Clone, Copy)]
enum EventMsg {
    TaskStarted,
    TokenCount,
    Error,
    Complete,
    Item,
}

const EVENT_MSGS: &[(&str, EventMsg)] = &[
    ("task_started", EventMsg::TaskStarted),
    ("token_count", EventMsg::TokenCount),
    ("error", EventMsg::Error),
    ("task_complete", EventMsg::Complete),
    ("item_started", EventMsg::Item),
    ("item_completed", EventMsg::Item),
];

/// Classify one row by the envelope and payload Codex writes.
fn codex_row(row: &Row<'_>) -> CodexRow {
    match row.kind("/type", ENVELOPES) {
        Some(Envelope::Header) => CodexRow::SessionMeta,
        Some(Envelope::Context) => CodexRow::Context,
        Some(Envelope::Error) => CodexRow::Error,
        Some(Envelope::ResponseItem) => response_item_row(row),
        Some(Envelope::EventMsg) => row
            .at("/payload")
            .as_ref()
            .map_or(CodexRow::Work, event_msg_row),
        None => CodexRow::Work,
    }
}

fn response_item_row(row: &Row<'_>) -> CodexRow {
    let prompt = row.is("/payload/type", "message")
        && (row.is("/payload/role", "developer") || row.is("/payload/role", "user"));
    if prompt {
        CodexRow::PromptMessage
    } else {
        CodexRow::Work
    }
}

fn event_msg_row(payload: &Row<'_>) -> CodexRow {
    match payload.kind("/type", EVENT_MSGS) {
        Some(EventMsg::TaskStarted) => CodexRow::TaskStarted,
        Some(EventMsg::TokenCount) => CodexRow::TokenCount,
        Some(EventMsg::Error) => CodexRow::Error,
        Some(EventMsg::Complete) => complete_row(payload),
        Some(EventMsg::Item) => prompt_item_row(payload),
        None => CodexRow::Work,
    }
}

/// A completion with the error and no last message is the refusal; a
/// last message as well is work, as the H1 case pins.
fn complete_row(payload: &Row<'_>) -> CodexRow {
    if payload.present("/error") && !payload.present("/last_agent_message") {
        CodexRow::ErrorComplete
    } else {
        CodexRow::Work
    }
}

fn prompt_item_row(payload: &Row<'_>) -> CodexRow {
    if payload.is("/item/type", "UserMessage") {
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
pub(in crate::adapters) fn codex_thread_refusal(
    body: &str,
    since_nanos: Option<i128>,
    now: Option<&Now>,
    bindings: &[BoundSecret],
) -> Option<PreSessionRefusal> {
    let mut resets = Vec::new();
    let mut error_seen = false;
    let mut rate_limit_error = None;
    for line in body.lines() {
        let raw = serde_json::from_str::<Value>(line).ok()?;
        let row = Row::new(&raw, bindings);
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
            .at("/payload/rate_limits")
            .as_ref()
            .map(reset_windows)
            .filter(|windows| !windows.is_empty())
        {
            resets = windows;
        }
        if let Some(found) = thread_error(row) {
            let found = found.with_fallback("/error");
            let class = codex_class(&found);
            if class == RefusalClass::RateLimit {
                rate_limit_error = Some(codex_error(&found, class, now));
            }
            error_seen = true;
        }
    }
    let mut refusal = rate_limit_error?;
    // Windows can be reported after the error and take precedence over prose.
    refusal.retry_after_seconds = now
        .and_then(|now| exhausted_reset(now, &resets))
        .or(refusal.retry_after_seconds);
    Some(refusal)
}

/// Whether a row is at or after the start; an untimed row is read, never assumed history.
fn row_in_invocation(row: &Row<'_>, since_nanos: Option<i128>) -> bool {
    since_nanos
        .and_then(|since| row.since("/timestamp", since))
        .unwrap_or(true)
}

/// Return the error at its own nesting level, including an explicit null
/// on a completion. Such a row alone is not an error-only completion.
fn thread_error(row: Row<'_>) -> Option<Row<'_>> {
    match row.kind("/type", ENVELOPES)? {
        Envelope::Error => Some(row),
        Envelope::EventMsg => event_msg_error(row.at("/payload")?),
        Envelope::Header | Envelope::Context | Envelope::ResponseItem => None,
    }
}

fn event_msg_error(payload: Row<'_>) -> Option<Row<'_>> {
    match payload.kind("/type", EVENT_MSGS)? {
        EventMsg::Complete if payload.has("/error") => payload.at("/error"),
        EventMsg::Error => Some(payload),
        EventMsg::Complete | EventMsg::TaskStarted | EventMsg::TokenCount | EventMsg::Item => None,
    }
}

/// The same read where the CLI announced no thread, so the locator is
/// empty: every candidate is first correlated by cwd and session start,
/// singleton included, and exactly one correlated file is read.
pub(in crate::adapters) fn codex_thread_refusal_since(
    root: &Path,
    since_nanos: i128,
    workdir: &str,
    now: Option<&Now>,
    depth: usize,
    bindings: &[BoundSecret],
) -> Option<PreSessionRefusal> {
    let mut found = find_codex_threads_since(root, since_nanos, depth);
    found.retain(|path| thread_header(path, None, workdir, since_nanos, bindings).is_some());
    let [path] = found.as_slice() else {
        return None;
    };
    let body = std::fs::read_to_string(path).ok()?;
    codex_thread_refusal(&body, Some(since_nanos), now, bindings)
}

/// The thread record whose file name carries `thread_id`, anywhere in a
/// bounded walk below `root`. The id is codex's own announcement, never
/// "the newest file": a concurrent seat's thread must not lend this one
/// its effort (decision 0035).
pub(in crate::adapters) fn find_codex_thread(
    root: &Path,
    thread_id: &Ident,
    depth: usize,
) -> Option<PathBuf> {
    let mut found = None;
    walk_codex_threads(root, depth, &mut |path| {
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| thread_id.names_rollout(name));
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

/// A rollout belongs to this invocation only when its first row names
/// the workdir, a current start and the record's own id.
pub(in crate::adapters) fn thread_header(
    path: &Path,
    announced: Option<&Ident>,
    workdir: &str,
    since_nanos: i128,
    bindings: &[BoundSecret],
) -> Option<()> {
    let body = std::fs::read_to_string(path).ok()?;
    let raw = serde_json::from_str::<Value>(body.lines().next()?).ok()?;
    let row = Row::new(&raw, bindings);
    if !matches!(row.kind("/type", ENVELOPES), Some(Envelope::Header)) {
        return None;
    }
    let payload = row.at("/payload")?;
    let id = payload.ident("/id")?;
    let started = payload.since("/timestamp", since_nanos)?;
    (payload.equals("/cwd", workdir) && started && names_own_record(path, &id, announced))
        .then_some(())
}

/// Whether `id` is the record's own: equal to the announced thread when
/// one is known, else equal to the full id the file name carries. A
/// mismatch, including an empty id or a fragment of the real id,
/// correlates nothing.
fn names_own_record(path: &Path, id: &Ident, announced: Option<&Ident>) -> bool {
    if id.is_offered("") {
        return false;
    }
    announced.map_or_else(
        || {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(record_id_from_name)
                .is_some_and(|own| *id == Ident::from_offered(own))
        },
        |announced| announced == id,
    )
}

/// The length of the full id [`is_codex_thread_id`] accepts.
const CODEX_THREAD_ID_LEN: usize = 36;

/// The id a codex rollout's file name carries: the full-shape id at the
/// end of `rollout-<timestamp>-<id>.jsonl`. A name that does not end in
/// one carries no id, so a fixed word, a leading or middle fragment, or a
/// trailing group of the real id never stands in for it.
fn record_id_from_name(name: &str) -> Option<&str> {
    let stem = name.strip_suffix(".jsonl")?.strip_prefix("rollout-")?;
    let at = stem.len().checked_sub(CODEX_THREAD_ID_LEN)?;
    let id = stem.get(at..)?;
    is_codex_thread_id(id).then_some(id)
}

/// Codex's thread id as it writes it: 8-4-4-4-12 lowercase hex groups,
/// read at their fixed byte positions. The caller hands 36 bytes, so a
/// multibyte character cannot sit at a hyphen's position and pass.
fn is_codex_thread_id(id: &str) -> bool {
    id.char_indices().all(|(at, c)| match at {
        8 | 13 | 18 | 23 => c == '-',
        _ => c.is_ascii_digit() || matches!(c, 'a'..='f'),
    })
}

#[cfg(test)]
mod tests;
