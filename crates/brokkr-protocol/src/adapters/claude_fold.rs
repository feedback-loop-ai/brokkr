//! What a claude stream folds beside the session record: the assistant
//! records counted so far, the usage its `result`s have settled, and the
//! cost a resumed process started from.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::capability_calls::Observation;
use super::row::{CapturedId, Ident, Row};
use super::step::{Emitted, Step};
use super::{
    add_usage, effort_in_json, fold_result_meta, model_in_json, plain_claude_session,
    EFFORT_NOT_REPORTED, MODEL_NOT_REPORTED, USAGE_FIELDS,
};
use crate::transcript::{Locator, Transcript};
use crate::PreSessionRefusal;

const REASONING: &str = "reasoning_output_tokens";

/// A claude stream's fold across its `result`s.
///
/// A `result` reports its own query's usage and turn count, not the
/// session's. A message queued into the session (a background task's
/// notice) runs another query, and that query ends in a result of its
/// own. Measured on Claude Code 2.1.302 (2026-10-18): a session whose
/// background task finished reported 33,845 cache reads over two turns,
/// then 22,923 over one, and `modelUsage` carried both. So the results
/// are summed. `modelUsage` is no substitute: on `--resume` it also
/// carries the earlier process's usage, as `total_cost_usd` does. Kept
/// last-result-only, a seat journaled 310,250 of its 26,882,705 cache
/// reads and one of its turns (swarmail, epic-120-batch-1-of-6-smtp-
/// and-p-2c675a1a).
#[derive(Default)]
pub(super) struct ClaudeFold {
    /// Assistant records counted: the live turn count until a result.
    pub(super) turns: u64,
    /// Every result's usage and turns so far, summed.
    settled: Map<String, Value>,
    /// The session cost a resumed process started from, read before it
    /// launched; `None` on a cold launch or with nothing to restore.
    restored: Option<f64>,
    /// The id of the message the last assistant line carried.
    message: Option<Ident>,
}

impl ClaudeFold {
    /// The fold for a launch that rejoins `session`, or a cold one.
    ///
    /// `--resume` restores the session's running cost from the newest
    /// `cost-state` record in its transcript, and each `result` after it
    /// reports that cost plus this process's own. Measured on Claude Code
    /// 2.1.302 (2026-10-18): a session whose record said $0.0553188 was
    /// resumed for one short query, and the result said $0.057847. The
    /// record is read before the process launches, so the figure is the
    /// one it restored, and an attempt journals its own spend rather than
    /// its predecessor's a second time.
    ///
    /// The transcript is a weaker source than the stream, as the dsh tail
    /// is: the one value taken is a number, nothing but a subtraction is
    /// derived from it, and at worst the record holds a cost a compromised
    /// seat chose.
    pub(super) fn rejoining(home: &Path, session: Option<&str>) -> ClaudeFold {
        ClaudeFold {
            restored: session.and_then(|session| restored_cost(home, session)),
            ..ClaudeFold::default()
        }
    }

    /// Whether an assistant line is the message the line before it began.
    /// Claude Code streams a message as a line per content block, each
    /// repeating the message's id and its usage, so a fold that counted
    /// every line read a seat's tokens two and a half times over (114
    /// Claude seats in the swarmail journal: 774.6M input against their
    /// transcripts' 315.1M). A line without an id is a message of its own.
    pub(super) fn repeats(&mut self, id: Option<Ident>) -> bool {
        let repeat = id.is_some() && id == self.message;
        self.message = id;
        repeat
    }

    /// One `result` folded: its usage, thinking count and turns added to
    /// what earlier results settled, and the sums written over the live
    /// estimate, which repeats a message's usage per content block and
    /// reads its output before it is final.
    pub(super) fn settle(&mut self, event: &Row<'_>, session_meta: &mut Map<String, Value>) {
        let mut reported = claude_usage(event.at("/usage"));
        for (key, pointer) in [
            ("num_turns", "/num_turns"),
            (REASONING, "/usage/output_tokens_details/thinking_tokens"),
        ] {
            if let Some(value) = event.unsigned_count(pointer).filter(|value| *value != 0) {
                reported.insert(key.into(), Value::from(value));
            }
        }
        for (key, value) in reported {
            let value = value.as_u64().expect("claude usage is a positive integer");
            add_usage(&mut self.settled, &key, value);
        }
        session_meta.extend(self.settled.clone());
        // Claude's thinking is a session figure, on the session record
        // only (decision 0035 ruling 4), and a SUBSET of output_tokens
        // (seat-record v6). Which of Claude's two figures is off when it
        // is not cannot be told from the stream, so it is then absent.
        let output = self.settled.get("output_tokens").and_then(Value::as_u64);
        let thought = self.settled.get(REASONING).and_then(Value::as_u64);
        if thought.is_some_and(|thought| output.is_none_or(|output| output < thought)) {
            session_meta.remove(REASONING);
        }
        // The process's cost so far, less what `--resume` restored. A
        // total below the restored figure cannot be split, so it is absent.
        if let Some(total) = event.number("/total_cost_usd").filter(|total| *total > 0.0) {
            let own = total - self.restored.unwrap_or_default();
            if own > 0.0 {
                session_meta.insert("total_cost_usd".into(), Value::from(own));
            } else {
                session_meta.remove("total_cost_usd");
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Event {
    System,
    Assistant,
    Result,
}

/// One Claude stream row folded into seat telemetry. Transcript records
/// become available as soon as init announces an id. Each tool-use block
/// becomes one seat-turn checkpoint, in order. Text-only turns emit one
/// checkpoint as well, and usage rides only the first checkpoint.
pub(super) fn fold_stream_event(
    event: &Row<'_>,
    identity: Option<CapturedId>,
    fold: &mut ClaudeFold,
    session_meta: &mut Map<String, Value>,
    transcript: &mut Transcript,
    emit: &mut impl FnMut(&Emitted),
) -> Option<PreSessionRefusal> {
    let kinds = &[
        ("system", Event::System),
        ("assistant", Event::Assistant),
        ("result", Event::Result),
    ];
    match event.kind("/type", kinds) {
        Some(Event::System) => {
            if let Some(identity) = identity {
                transcript.record(Locator::Session(identity), session_meta, emit);
            }
        }
        Some(Event::Assistant) => {
            let base = fold.turn(event, session_meta);
            emit_turn(event, base, emit);
        }
        Some(Event::Result) => {
            fold_result_meta(event, identity, session_meta, transcript, emit);
            fold.settle(event, session_meta);
        }
        None => {}
    }
    None
}

impl ClaudeFold {
    fn turn(&mut self, event: &Row<'_>, meta: &mut Map<String, Value>) -> Map<String, Value> {
        let repeat = self.repeats(event.ident("/message/id"));
        self.turns += u64::from(!repeat);
        meta.insert("num_turns".into(), Value::from(self.turns));

        let mut base = Map::new();
        base.insert("turn".into(), Value::from(self.turns));
        // Each record names the effort it applied (decision 0035 ruling
        // 3). An absent or invalid token leaves the last valid one in meta.
        for (key, label, missing) in [
            ("model", model_in_json(event), MODEL_NOT_REPORTED),
            ("effort", effort_in_json(event), EFFORT_NOT_REPORTED),
        ] {
            if let Some(label) = &label {
                meta.insert(key.into(), Value::String(label.clone()));
            }
            base.insert(
                key.into(),
                Value::String(label.unwrap_or_else(|| missing.into())),
            );
        }
        if !repeat {
            // Thinking tokens are reported only in the result (0035 ruling 4).
            for (key, value) in claude_usage(event.at("/message/usage")) {
                add_usage(
                    meta,
                    &key,
                    value.as_u64().expect("claude usage is a positive integer"),
                );
                base.insert(key, value);
            }
        }
        base
    }
}

fn emit_turn(event: &Row<'_>, mut base: Map<String, Value>, emit: &mut impl FnMut(&Emitted)) {
    let blocks = event
        .items("/message/content")
        .into_iter()
        .flatten()
        .filter(|block| block.is("/type", "tool_use"));
    let mut emitted = false;
    for block in blocks {
        let mut checkpoint = base.clone();
        // The journal keeps only file_path, never commands, URLs or full inputs.
        if let Some(target) = block.text("/input/file_path") {
            checkpoint.insert(
                "target".into(),
                Value::String(target.chars().take(80).collect()),
            );
        }
        emit(&Emitted::new(
            Step::SeatTurn,
            Observation::claude(&block).onto(Value::Object(checkpoint)),
        ));
        if !emitted {
            for key in USAGE_FIELDS {
                base.remove(key);
            }
        }
        emitted = true;
    }
    if !emitted {
        emit(&Emitted::new(Step::SeatTurn, Value::Object(base)));
    }
}

/// Claude reports cache reads and cache creations beside its uncached
/// input count. The seat record's input count is inclusive of reads,
/// while cache creations retain their own cache_write_tokens name.
fn claude_usage(usage: Option<Row<'_>>) -> Map<String, Value> {
    let Some(usage) = usage else {
        return Map::new();
    };
    let raw = |path| usage.unsigned_count(path).unwrap_or_default();
    let read = raw("/cache_read_input_tokens");
    [
        ("input_tokens", raw("/input_tokens").saturating_add(read)),
        ("output_tokens", raw("/output_tokens")),
        ("cache_read_tokens", read),
        ("cache_write_tokens", raw("/cache_creation_input_tokens")),
    ]
    .into_iter()
    .filter(|(_, count)| *count != 0)
    .map(|(key, count)| (key.to_owned(), Value::from(count)))
    .collect()
}

/// The newest `totalCostUSD` a `cost-state` record of `session` holds in
/// `<home>/<project>/<session>.jsonl`: found by the id the launch rejoins,
/// never by the newest file, so another seat's session lends it nothing.
fn restored_cost(home: &Path, session: &str) -> Option<f64> {
    if !plain_claude_session(session) {
        return None;
    }
    let name = format!("{session}.jsonl");
    let entries = std::fs::read_dir(home).ok()?.flatten();
    let mut projects: Vec<PathBuf> = entries.map(|entry| entry.path()).collect();
    projects.sort();
    let body = projects
        .iter()
        .find_map(|project| std::fs::read_to_string(project.join(&name)).ok())?;
    let mut named = body
        .lines()
        .rev()
        .filter(|line| line.contains("cost-state"));
    named.find_map(|line| {
        let record = serde_json::from_str::<Value>(line).ok()?;
        let ours = record["type"] == "cost-state" && record["sessionId"] == session;
        record["totalCostUSD"]
            .as_f64()
            .filter(|cost| ours && *cost >= 0.0)
    })
}
