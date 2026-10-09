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

/// The bridge's command is decided on a fold, so it lands only on the head
/// that fold read (decision 0029). A peer at another terminal retries the
/// parked run after the bridge folded and before it journals the command:
/// the bridge's retry, accepted on that fold, would land on a running run
/// where `fold` refuses it. It writes no command, refuses as a stale
/// cursor, and the peer's retry stands.
#[test]
fn a_bridge_command_beaten_to_the_head_by_a_peer_is_refused_stale() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::tests::parked_store(&dir.path().join("beaten.db"), "beaten");
    let (seq, hash) = store.head_hash("beaten").unwrap();
    let wire = super::tests::looper("looper-command", "retry", seq, &hash);

    let peer = |store: &mut Store| {
        let retried = operator_command(store, "beaten", Retry, "peer", "again").unwrap();
        assert!(matches!(retried, FencedCommandOutcome::Accepted { .. }));
    };
    let refused = super::operator::apply_fenced_windows(&mut store, "beaten", &wire, peer, |_| {});

    let events = store.load("beaten").unwrap();
    let rejected = events.last().unwrap();
    assert_eq!(
        refused.unwrap(),
        FencedCommandOutcome::Rejected {
            reason: Refusal::StaleCursor.word().into(),
            head_seq: 9,
            head_hash: rejected.event_hash.clone(),
        }
    );
    let tail: Vec<_> = events[6..].iter().map(|event| event.event_type).collect();
    let (commanded, accepted) = (EventType::OperatorCommanded, EventType::OperatorAccepted);
    assert_eq!(tail, [commanded, accepted, EventType::OperatorRejected]);
    assert_eq!(rejected.payload["command_id"], "looper-command");
    assert_eq!(events[6].payload["operator"], "peer");
    assert_eq!(fold(&events).unwrap().status, Status::Running);

    // The refusal is the command's only record, so a redelivery of the
    // same wire answers with it and writes nothing.
    let redelivered = apply_fenced_operator_command(&mut store, "beaten", &wire).unwrap();
    let receipt = FencedCommandOutcome::Rejected {
        reason: Refusal::StaleCursor.word().into(),
        head_seq: 9,
        head_hash: rejected.event_hash.clone(),
    };
    assert_eq!(redelivered, receipt);
    assert_eq!(
        store.head_hash("beaten").unwrap(),
        (9, rejected.event_hash.clone())
    );
}

/// Two deliveries of one command race the same parked head: the first
/// journals the command and its acceptance after the second folded, so
/// the second's command loses the fence. What moved the head disposed of
/// the very command it carries, so it answers with that acceptance and
/// writes nothing — the receipt every redelivery reads — where a stale
/// refusal would be a receipt the journal contradicts.
#[test]
fn a_bridge_command_beaten_to_the_head_by_its_own_redelivery_answers_with_its_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::tests::parked_store(&dir.path().join("twice.db"), "twice");
    let (seq, hash) = store.head_hash("twice").unwrap();
    let wire = super::tests::looper("looper-command", "retry", seq, &hash);

    let mut first = None;
    let peer = |store: &mut Store| {
        first = Some(apply_fenced_operator_command(store, "twice", &wire).unwrap());
    };
    let second = super::operator::apply_fenced_windows(&mut store, "twice", &wire, peer, |_| {});

    let events = store.load("twice").unwrap();
    let receipt = FencedCommandOutcome::Accepted {
        head_seq: 8,
        head_hash: events.last().unwrap().event_hash.clone(),
    };
    assert_eq!(
        (first, second.unwrap()),
        (Some(receipt.clone()), receipt.clone())
    );
    let tail: Vec<_> = events[6..].iter().map(|event| event.event_type).collect();
    assert_eq!(
        tail,
        [EventType::OperatorCommanded, EventType::OperatorAccepted]
    );
    let redelivered = apply_fenced_operator_command(&mut store, "twice", &wire).unwrap();
    assert_eq!(redelivered, receipt);
}
