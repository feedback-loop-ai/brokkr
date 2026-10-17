//! Claude's result usage as the driver folds it into the session record:
//! the thinking count is a subset of the output count or it is absent
//! (seat-record v6; decision 0035 ruling 4).

use super::super::*;

#[test]
fn claudes_thinking_is_a_subset_of_its_output_or_it_is_left_out() {
    // Claude Code 2.1.300 reported a long session's thinking above its
    // output on the result; the store refuses such a record and the
    // seat failed at its last step. The count is kept when it is a
    // subset, left out when it cannot be one, and never zero.
    let fold = |output: u64, thinking: u64| {
        let mut turns = 1;
        let mut meta = Map::new();
        let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
        let event = json!({"type": "result", "num_turns": 92, "usage": {"input_tokens": 4,
            "output_tokens": output, "output_tokens_details": {"thinking_tokens": thinking}}});
        fold_stream_event(&event, &mut turns, &mut meta, &mut transcript, &mut |_| {});
        (
            meta.get("output_tokens").cloned(),
            meta.get("reasoning_output_tokens").cloned(),
        )
    };
    assert_eq!(fold(50, 40), (Some(json!(50)), Some(json!(40))));
    assert_eq!(fold(50, 50), (Some(json!(50)), Some(json!(50))));
    assert_eq!(fold(43, 900), (Some(json!(43)), None));
    assert_eq!(fold(43, 0), (Some(json!(43)), None));
}
