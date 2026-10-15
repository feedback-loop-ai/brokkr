//! #568's console tab: a participant's tokens are read apart once, here —
//! the total, input, output, and the cache hit as cache reads over input.
//! A child of the owning `tests.rs`, which sits at its line baseline.

use super::*;
use crate::tokens::cache_hit_text;

#[test]
fn the_cache_hit_is_cache_reads_over_input_and_absent_without_both() {
    // A real review seat (3,002,733 of 3,004,371) first; nothing to compare is no figure, never a zero.
    let hits = [
        (3_002_733, 3_004_371, "99.9%"),
        (0, 80, "0.0%"),
        (1, 3, "33.3%"),
    ];
    for (read, input, hit) in hits
        .map(|(r, i, h)| (Some(r), Some(i), Some(h)))
        .into_iter()
        .chain([
            (None, Some(80), None),
            (Some(5), None, None),
            (Some(0), Some(0), None),
        ])
    {
        let usage = TokenUsage {
            cache_read_tokens: read,
            input_tokens: input,
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
