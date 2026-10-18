//! A participant's tokens read apart for a console's seat row (#568): the
//! total, input and output, and the cache hit, the one place that share is
//! worked out (decision 0013).

use crate::{cell_of, fmt_count, fmt_tokens, usage_text, Cell, TokenUsage};
use serde::Serialize;

/// A participant's tokens as separate cells: the total, input and
/// output as counts (`1.24M tok`, `1.19M`, `48k`), and the share of the
/// prompt served from cache, cache reads over input and cache writes
/// (`99.9%`). The one place the share is worked out, so no surface
/// divides for itself (decision 0013).
/// Each is absent, with its reason, where the harness reported nothing.
#[derive(Serialize, Debug, PartialEq)]
pub struct TokenCells {
    pub total: Cell,
    pub input: Cell,
    pub output: Cell,
    pub cache_hit: Cell,
}

impl TokenCells {
    /// The tokens as every surface reads them beside the cache hit:
    /// `Σ 1.24M tok ↑ 1.19M ↓ 48k`, the total's own words when none of
    /// the three was reported. The console's seat row and the TUI's say
    /// exactly this, so the arrows mean one thing everywhere.
    pub fn arrows(&self) -> String {
        if self.total.absent && self.input.absent && self.output.absent {
            return self.total.text.clone();
        }
        format!(
            "Σ {} ↑ {} ↓ {}",
            self.total.text, self.input.text, self.output.text
        )
    }

    /// [`TokenCells::arrows`] with the cache hit beside it, for a surface
    /// with one cell to give them: `Σ 1.24M tok ↑ 1.19M ↓ 48k · 99.9%
    /// cache`, the arrows alone when no cache hit was worked out.
    pub fn line(&self) -> String {
        match self.cache_hit.absent {
            true => self.arrows(),
            false => format!("{} · {} cache", self.arrows(), self.cache_hit.text),
        }
    }
}

/// Cache reads over the whole prompt, to one decimal: `99.9%`. Cache
/// reads are a subset of input (the [`TokenUsage`] rule); cache writes are
/// not, yet each is a prompt token the provider read uncached, a miss. Over
/// input alone a Claude seat, whose uncached input is a few tokens a call
/// and whose misses are its writes, read 100.0% whatever it missed: a
/// session of 20,213,446 reads, 242 uncached and 243,474 written is 98.8%.
/// `None` when reads or input are unreported or input is zero.
pub(crate) fn cache_hit_text(usage: &TokenUsage) -> Option<String> {
    match (usage.cache_read_tokens, usage.input_tokens) {
        (Some(read), Some(input)) if input > 0 => {
            let prompt = input.saturating_add(usage.cache_write_tokens.unwrap_or(0));
            Some(format!("{:.1}%", read as f64 * 100.0 / prompt as f64))
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

#[cfg(test)]
mod tests;
