//! Claude's result usage as the driver folds it into the session record:
//! the thinking count is a subset of the output count or it is absent
//! (seat-record v6; decision 0035 ruling 4).

use super::super::*;

/// The session record after Claude's `result` events, each as
/// `(output_tokens, thinking_tokens)`, folded in order into one session.
fn folded(results: &[(u64, u64)]) -> (Option<Value>, Option<Value>) {
    let mut turns = 1;
    let mut meta = Map::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    for (output, thinking) in results {
        let event = json!({"type": "result", "num_turns": 92, "usage": {"input_tokens": 4,
            "output_tokens": output, "output_tokens_details": {"thinking_tokens": thinking}}});
        fold_stream_event(&event, &mut turns, &mut meta, &mut transcript, &mut |_| {});
    }
    (
        meta.get("output_tokens").cloned(),
        meta.get("reasoning_output_tokens").cloned(),
    )
}

#[test]
fn claudes_thinking_is_a_subset_of_its_output_or_it_is_left_out() {
    // Claude Code 2.1.300 reported a long session's thinking above its
    // output on the result; the store refuses such a record and the
    // seat failed at its last step. The count is kept when it is a
    // subset, left out when it cannot be one, and never zero.
    assert_eq!(folded(&[(50, 40)]), (Some(json!(50)), Some(json!(40))));
    assert_eq!(folded(&[(50, 50)]), (Some(json!(50)), Some(json!(50))));
    assert_eq!(folded(&[(43, 900)]), (Some(json!(43)), None));
    assert_eq!(folded(&[(43, 0)]), (Some(json!(43)), None));
}

#[test]
fn a_later_result_replaces_the_thinking_an_earlier_one_reported() {
    // A message queued into the session (a background task's notice)
    // runs another turn, and that turn ends with a result of its own.
    // Its output replaces the earlier result's, so the earlier thinking
    // count goes with it: a stale one above the new output failed a
    // review seat at its last step (landing-the-console-library-slic-
    // b5217290), and a later result's own subset replaces it.
    assert_eq!(folded(&[(50, 40), (5, 0)]), (Some(json!(5)), None));
    assert_eq!(folded(&[(50, 40), (30, 900)]), (Some(json!(30)), None));
    assert_eq!(
        folded(&[(50, 40), (30, 20)]),
        (Some(json!(30)), Some(json!(20)))
    );
}
