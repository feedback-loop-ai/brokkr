//! The operator's verbs against the rule they now share with `fold`.

use super::*;

/// A selector with no strategy and no default parks a run at `Start`,
/// before it entered any phase, and `fold` refuses a retry's acceptance
/// there: there is no phase to return to. Both doors ask `fold`'s own
/// rule, so both refuse the retry by name and the journal still folds,
/// where a hand-kept mirror of the rule accepted it and left a journal
/// that stops folding for good.
#[test]
fn a_retry_with_no_phase_to_return_to_is_refused_at_both_doors() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("start.db")).unwrap();
    store
        .create_run("start", "feature", "test", &json!({}))
        .unwrap();
    for (event_type, payload) in [
        (EventType::RunStarted, json!({"feature": "feature"})),
        (EventType::RunParked, json!({"reason": "SELECT-NO-DEFAULT"})),
    ] {
        store
            .append_next("start", event_type, payload, None, None)
            .unwrap();
    }

    let unfenced = operator_command(&mut store, "start", Retry, "operator", "again").unwrap();
    let (seq, hash) = store.head_hash("start").unwrap();
    let again = FencedCommand {
        reason: "again",
        ..super::tests::looper("fenced", "retry", seq, &hash)
    };
    let fenced = apply_fenced_operator_command(&mut store, "start", &again).unwrap();
    for outcome in [unfenced, fenced] {
        assert!(
            matches!(&outcome, FencedCommandOutcome::Rejected { reason, .. }
                if reason == Refusal::NoPhaseToRetry.word()),
            "{outcome:?}"
        );
    }
    let state = fold(&store.load("start").unwrap()).unwrap();
    assert_eq!(
        (state.status, state.phase),
        (Status::AwaitingOperator, None)
    );
}
