//! The operator's verbs against the rule they now share with `fold`.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;

/// A peer that writes to the store it is handed.
type Peer = Box<dyn FnMut(&mut Store)>;

thread_local! {
    /// The peer a test puts between a bridge refusal's receipt lookup and
    /// the write that lookup decided.
    static BEFORE_REFUSAL: RefCell<Option<Peer>> = RefCell::new(None);
    /// The peer a test puts between a bridge acceptance's commit and the
    /// receipt it reports.
    static AFTER_ACCEPTANCE: RefCell<Option<Peer>> = RefCell::new(None);
}

/// Run the peer a test put in `held`, each time the window opens. It is
/// taken out while it runs, so a delivery the peer makes itself passes
/// the window unheld.
fn run_held(held: &'static std::thread::LocalKey<RefCell<Option<Peer>>>, store: &mut Store) {
    let Some(mut peer) = held.with(|slot| slot.borrow_mut().take()) else {
        return;
    };
    peer(store);
    held.with(|slot| *slot.borrow_mut() = Some(peer));
}

/// The window on every turn a bridge refusal makes.
pub(super) fn before_refusal(store: &mut Store) {
    run_held(&BEFORE_REFUSAL, store);
}

/// The window after a bridge acceptance commits.
pub(super) fn after_acceptance(store: &mut Store) {
    run_held(&AFTER_ACCEPTANCE, store);
}

/// Put `peer` between each bridge refusal's receipt lookup and its write.
fn between_lookup_and_refusal(peer: impl FnMut(&mut Store) + 'static) {
    BEFORE_REFUSAL.with(|slot| *slot.borrow_mut() = Some(Box::new(peer)));
}

/// One event a peer journals on whatever head it finds.
fn journal(store: &mut Store, run_id: &str, event_type: EventType, payload: Value) {
    store
        .append_next(run_id, event_type, payload, None, None)
        .unwrap();
}

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

    let receipt = accepted_once(&mut store, "twice", &wire);
    assert_eq!((first, second.unwrap()), (Some(receipt.clone()), receipt));
}

/// The receipt of a parked run's one journaled command and its acceptance,
/// the only two events after the park; a redelivery answers with it.
fn accepted_once(store: &mut Store, run_id: &str, wire: &FencedCommand) -> FencedCommandOutcome {
    let events = store.load(run_id).unwrap();
    let receipt = FencedCommandOutcome::Accepted {
        head_seq: 8,
        head_hash: events.last().unwrap().event_hash.clone(),
    };
    let tail: Vec<_> = events[6..].iter().map(|event| event.event_type).collect();
    assert_eq!(
        tail,
        [EventType::OperatorCommanded, EventType::OperatorAccepted]
    );
    let redelivered = apply_fenced_operator_command(store, run_id, wire).unwrap();
    assert_eq!(redelivered, receipt);
    receipt
}

/// The window the receipt lookup left (decision 0029): the first delivery
/// journals its command after the second folded, and its acceptance after
/// the second looked the command up and found it undisposed. The second's
/// stale refusal was decided on that lookup's head, so it lands nowhere;
/// the lookup is made again and answers with the acceptance, where an
/// unfenced refusal is a second disposition every redelivery contradicts.
#[test]
fn a_bridge_refusal_beaten_to_the_head_by_its_own_redelivery_answers_with_its_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::tests::parked_store(&dir.path().join("window.db"), "window");
    let (seq, hash) = store.head_hash("window").unwrap();
    let wire = super::tests::looper("looper-command", "retry", seq, &hash);

    // The first delivery's two writes, each where its fence lands it.
    let first_command = |store: &mut Store| {
        let command = json!({"command_id": "looper-command", "command": "retry", "args": {}, "operator": "operator"});
        journal(store, "window", EventType::OperatorCommanded, command);
    };
    between_lookup_and_refusal(|store| {
        let acceptance =
            json!({"command_id": "looper-command", "operator": "operator", "reason": "reason"});
        journal(store, "window", EventType::OperatorAccepted, acceptance);
    });
    let second =
        super::operator::apply_fenced_windows(&mut store, "window", &wire, first_command, |_| {});

    assert_eq!(second.unwrap(), accepted_once(&mut store, "window", &wire));
}

/// A peer journals between the bridge's acceptance committing and its
/// receipt being reported. The receipt is the acceptance's own seq and
/// hash, the one every redelivery reads, where a head read after the
/// commit reports the peer's event as this command's.
#[test]
fn a_bridge_acceptance_reports_the_event_it_committed_whatever_lands_after() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::tests::parked_store(&dir.path().join("after.db"), "after");
    let (seq, hash) = store.head_hash("after").unwrap();
    let wire = super::tests::looper("looper-command", "retry", seq, &hash);

    AFTER_ACCEPTANCE.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(|store: &mut Store| {
            let command =
                json!({"command_id": "peer", "command": "stop", "args": {}, "operator": "peer"});
            journal(store, "after", EventType::OperatorCommanded, command);
        }));
    });
    let first = apply_fenced_operator_command(&mut store, "after", &wire).unwrap();

    let events = store.load("after").unwrap();
    let acceptance = &events[7];
    assert_eq!(acceptance.event_type, EventType::OperatorAccepted);
    let receipt = FencedCommandOutcome::Accepted {
        head_seq: 8,
        head_hash: acceptance.event_hash.clone(),
    };
    assert_eq!(first, receipt);
    assert_eq!(store.head_hash("after").unwrap().0, 9);
    let redelivered = apply_fenced_operator_command(&mut store, "after", &wire).unwrap();
    assert_eq!(redelivered, receipt);
}

/// Three deliveries of one stale command, each inside the last one's
/// window: the first journals the command, the second finds it undisposed
/// and refuses the incomplete replay, and the third does the same between
/// the second's lookup and its write. The third's refusal is the
/// command's one disposition, and the second and the first, whose own
/// refusals were decided on heads the third took away, answer with it.
#[test]
fn every_delivery_of_one_command_reads_the_refusal_that_landed_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::tests::parked_store(&dir.path().join("thrice.db"), "thrice");
    let (seq, hash) = store.head_hash("thrice").unwrap();
    let stale = super::tests::looper("looper-command", "retry", seq - 1, &hash);

    let answers = Rc::new(RefCell::new(Vec::new()));
    let (third_hash, third_answers) = (hash.clone(), Rc::clone(&answers));
    between_lookup_and_refusal(move |store| {
        let wire = super::tests::looper("looper-command", "retry", seq - 1, &third_hash);
        let third = apply_fenced_operator_command(store, "thrice", &wire).unwrap();
        third_answers.borrow_mut().push(third);
    });
    let second = |store: &mut Store| {
        let second = apply_fenced_operator_command(store, "thrice", &stale).unwrap();
        answers.borrow_mut().push(second);
    };
    let first = super::operator::apply_fenced_racing(&mut store, "thrice", &stale, second);

    let events = store.load("thrice").unwrap();
    let receipt = FencedCommandOutcome::Rejected {
        reason: Refusal::IncompleteCommandReplay.word().into(),
        head_seq: 8,
        head_hash: events.last().unwrap().event_hash.clone(),
    };
    let mut answered = answers.borrow().clone();
    answered.push(first.unwrap());
    assert_eq!(answered, [receipt.clone(), receipt.clone(), receipt]);
    let tail: Vec<_> = events[6..].iter().map(|event| event.event_type).collect();
    assert_eq!(
        tail,
        [EventType::OperatorCommanded, EventType::OperatorRejected]
    );
}

/// A bridge refusal whose head moves on every turn is not re-decided
/// forever: after `FENCE_ATTEMPTS` lost turns it refuses with the drift
/// named, and nothing of the command is written.
#[test]
fn a_bridge_refusal_whose_head_never_holds_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::tests::parked_store(&dir.path().join("moving.db"), "moving");
    let (seq, hash) = store.head_hash("moving").unwrap();
    let wire = super::tests::looper("looper-command", "retry", seq, &hash);

    let peer_command = |store: &mut Store, id: String| {
        let command = json!({"command_id": id, "command": "stop", "args": {}, "operator": "peer"});
        journal(store, "moving", EventType::OperatorCommanded, command);
    };
    let mut peers = 0;
    between_lookup_and_refusal(move |store| {
        peers += 1;
        peer_command(store, format!("peer-{peers}"));
    });
    let moved = super::operator::apply_fenced_windows(
        &mut store,
        "moving",
        &wire,
        |store| peer_command(store, "peer-0".into()),
        |_| {},
    );

    assert!(
        matches!(
            &moved,
            Err(EngineError::JournalMoved { run_id, expected_seq: 11, found_seq: 12 })
                if run_id == "moving"
        ),
        "{moved:?}"
    );
    let events = store.load("moving").unwrap();
    assert_eq!(events.len(), 12);
    assert!(events
        .iter()
        .all(|event| event.payload["command_id"] != "looper-command"));
}
