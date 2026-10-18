//! What a claude stream folds beside the session record: the assistant
//! records counted so far, and the usage its `result`s have settled.

use serde_json::{Map, Value};

use super::{add_usage, claude_usage, positive_count};

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
}

impl ClaudeFold {
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
    }
}
