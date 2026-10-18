//! #568's console tab: a participant's tokens are read apart once, here —
//! the total, input, output, and the cache hit as cache reads over input
//! and cache writes.
//! A child of the owning `tests.rs`, which sits at its line baseline.

use super::*;
use crate::tokens::cache_hit_text;

#[test]
fn the_cache_hit_is_cache_reads_over_input_and_writes_and_absent_without_both() {
    // A real review seat (3,002,733 of 3,004,371) first, then a Claude
    // seat that wrote 243,474 tokens to its cache, its misses, and read
    // 100.0% over input alone; nothing to compare is no figure, never a zero.
    let hits = [
        (3_002_733, 3_004_371, None, "99.9%"),
        (20_213_446, 20_213_688, Some(243_474), "98.8%"),
        (0, 80, None, "0.0%"),
        (1, 3, Some(1), "25.0%"),
    ];
    for (read, input, write, hit) in hits
        .map(|(r, i, w, h)| (Some(r), Some(i), w, Some(h)))
        .into_iter()
        .chain([
            (None, Some(80), Some(20), None),
            (Some(5), None, Some(5), None),
            (Some(0), Some(0), None, None),
        ])
    {
        let usage = TokenUsage {
            cache_read_tokens: read,
            input_tokens: input,
            cache_write_tokens: write,
            ..TokenUsage::default()
        };
        assert_eq!(cache_hit_text(&usage).as_deref(), hit);
    }
}

#[test]
fn a_seats_token_cells_are_its_usage_read_apart() {
    // The wager's crew-B seat: 3,830,272 of 3,975,322 input tokens were cache reads.
    let events = codex_journal(vec![json!({
        "input_tokens": 3_975_322, "cache_read_tokens": 3_830_272, "output_tokens": 14_051
    })]);
    let tokens = &run_view(&events, None).participants[0].tokens;
    let cells = [
        &tokens.total,
        &tokens.input,
        &tokens.output,
        &tokens.cache_hit,
    ];
    assert_eq!(
        cells.map(|cell| (cell.text.as_str(), cell.absent)),
        [
            ("3.99M tok", false),
            ("3.98M", false),
            ("14k", false),
            ("96.4%", false)
        ]
    );
}
