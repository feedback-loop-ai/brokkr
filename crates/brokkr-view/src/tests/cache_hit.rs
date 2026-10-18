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

/// One Claude message's turn rows as a driver journaled each streamed
/// line: the probe's usage of 2026-10-18, with an untokened tool use.
pub(crate) fn streamed_turns() -> Vec<Value> {
    let first = json!({"step": "seat-turn", "input_tokens": 11_230, "cache_read_tokens": 11_228,
                       "cache_write_tokens": 11_389, "output_tokens": 16});
    let second = json!({"step": "seat-turn", "input_tokens": 22_619, "cache_read_tokens": 22_617,
                        "cache_write_tokens": 306, "output_tokens": 3});
    let untokened = json!({"step": "seat-turn", "tool": "Read"});
    vec![first.clone(), first.clone(), untokened, first, second]
}

#[test]
fn a_claude_message_streamed_line_by_line_is_metered_once() {
    // Claude Code streams a message as one line per content block, each
    // repeating its usage, and the driver journaled a turn for each. The
    // first message's three repeats, an untokened tool use between them,
    // are one message beside the second: 33,849 input, not 56,309.
    let mut events = codex_journal(vec![]);
    let session = events.pop().unwrap();
    for (seq, mut checkpoint) in (3..).zip(streamed_turns()) {
        checkpoint["turn"] = json!(seq - 2);
        let payload = json!({"effect_id": "eff1", "attempt_id": "att1", "checkpoint": checkpoint});
        events.insert(
            seq as usize - 1,
            ev(seq, EventType::EffectCheckpointed, payload, T0),
        );
    }
    events.push(session);
    let view = run_view(&events, None);
    let cells = crate::analytics::tests::cells(&view.participants[0].tokens);
    assert_eq!(cells, ["34k tok", "34k", "19", "74.3%"]);
}
