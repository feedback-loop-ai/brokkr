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

#[cfg(test)]
mod tests {
    use super::*;

    fn cells_of(input: Option<u64>, output: Option<u64>, read: Option<u64>) -> TokenCells {
        let usage = crate::finish_usage(TokenUsage {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: read,
            ..TokenUsage::default()
        });
        usage_cells(usage.as_ref(), "").1
    }

    #[test]
    fn tokens_read_with_arrows_and_the_cache_hit_beside_them() {
        let cells = cells_of(Some(1_190_000), Some(48_000), Some(1_188_810));
        assert_eq!(cells.arrows(), "Σ 1.24M tok ↑ 1.19M ↓ 48k");
        assert_eq!(cells.line(), "Σ 1.24M tok ↑ 1.19M ↓ 48k · 99.9% cache");
    }

    #[test]
    fn no_cache_read_reported_leaves_the_arrows_alone() {
        let cells = cells_of(Some(5_000), Some(1_000), None);
        assert_eq!(cells.line(), "Σ 6k tok ↑ 5k ↓ 1k");
    }

    #[test]
    fn nothing_reported_reads_as_the_totals_own_words() {
        let cells = usage_cells(None, "").1;
        assert_eq!(
            (cells.arrows(), cells.line()),
            (crate::ABSENT.to_string(), crate::ABSENT.to_string())
        );
    }
}
