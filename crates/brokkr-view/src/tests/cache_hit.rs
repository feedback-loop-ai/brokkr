//! #568's console tab: a participant's cache hit is worked out once, here,
//! from cache reads over input. A child of the owning `tests.rs`, which sits
//! at its line baseline.

use super::*;

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
