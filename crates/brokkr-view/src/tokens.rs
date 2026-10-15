//! A participant's tokens read apart for a console's seat row (#568): the
//! total, input and output, and the cache hit, the one place that share is
//! worked out (decision 0013).

use crate::{cell_of, fmt_count, fmt_tokens, usage_text, Cell, TokenUsage};
use serde::Serialize;

/// A participant's tokens as separate cells: the total, input and
/// output as counts (`1.24M tok`, `1.19M`, `48k`), and the share of input
/// served from cache, cache reads over input (`99.9%`). The one place the
/// share is worked out, so no surface divides for itself (decision 0013).
/// Each is absent, with its reason, where the harness reported nothing.
#[derive(Serialize)]
pub struct TokenCells {
    pub total: Cell,
    pub input: Cell,
    pub output: Cell,
    pub cache_hit: Cell,
}

/// Cache reads over input, to one decimal: `99.9%`. Cache reads are a
/// subset of input (the [`TokenUsage`] rule), so the share is at most a
/// whole; `None` when either is unreported or input is zero.
pub(crate) fn cache_hit_text(usage: &TokenUsage) -> Option<String> {
    match (usage.cache_read_tokens, usage.input_tokens) {
        (Some(read), Some(input)) if input > 0 => {
            Some(format!("{:.1}%", read as f64 * 100.0 / input as f64))
        }
        _ => None,
    }
}

/// A participant's usage cell and its token cells, from one reading.
pub(crate) fn usage_cells(usage: Option<&TokenUsage>, prefix: &str) -> (Cell, TokenCells) {
    let count = |read: fn(&TokenUsage) -> Option<u64>, show: fn(u64) -> String| {
        cell_of(usage.and_then(read).map(show), Some("not reported"))
    };
    let usage_cell = cell_of(
        usage.map(|usage| usage_text(usage, prefix)),
        Some("no token usage recorded"),
    );
    let tokens = TokenCells {
        total: count(|usage| usage.total_tokens, fmt_tokens),
        input: count(|usage| usage.input_tokens, fmt_count),
        output: count(|usage| usage.output_tokens, fmt_count),
        cache_hit: cell_of(
            usage.and_then(cache_hit_text),
            Some("no cache reads and input reported to compare"),
        ),
    };
    (usage_cell, tokens)
}
