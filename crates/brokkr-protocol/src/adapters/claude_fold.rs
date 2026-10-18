//! What a claude stream folds beside the session record: the assistant
//! records counted so far, the usage its `result`s have settled, and the
//! cost a resumed process started from.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{add_usage, claude_usage, plain_claude_session, positive_cost, positive_count};

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

    /// One `result` folded: its usage, thinking count and turns added to
    /// what earlier results settled, and the sums written over the live
    /// estimate, which repeats a message's usage per content block and
    /// reads its output before it is final.
    pub(super) fn settle(&mut self, event: &Value, session_meta: &mut Map<String, Value>) {
        let mut reported = claude_usage(event.get("usage"));
        for (key, pointer) in [
            ("num_turns", "/num_turns"),
            (REASONING, "/usage/output_tokens_details/thinking_tokens"),
        ] {
            if let Some(value) = positive_count(event.pointer(pointer)) {
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
        if let Some(total) = positive_cost(event.get("total_cost_usd")) {
            let own = total - self.restored.unwrap_or_default();
            if own > 0.0 {
                session_meta.insert("total_cost_usd".into(), Value::from(own));
            } else {
                session_meta.remove("total_cost_usd");
            }
        }
    }
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
