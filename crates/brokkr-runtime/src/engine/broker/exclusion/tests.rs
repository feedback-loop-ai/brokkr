//! Decision 0108 ruling 3's worktree release: a quarantined run stops
//! holding its worktree once its run lock is free and every attempt its own
//! reservation names is proven ended by the broker's records and the host,
//! never by the run's fold. The proof's own unit tests live beside the
//! verb; these are the admission's, over the comparison that refuses.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Run};
use serde_json::json;

use crate::engine::broker::admission::tests::cr2;
use crate::engine::broker::admission::tests::{
    acknowledge, boxed_claim, child_lock, dead_group, isolated, quarantined, unacknowledged,
    Fixture, LiveGroup,
};
use crate::engine::broker::admission::{Conflict, Unadmitted, Witnessed};
use crate::engine::operator::{operator_repair, operator_settle, Repair, Settle};

/// A claim of the fixture's worktree that is not run A's: what a later
/// admission compares.
fn next(fixture: &Fixture) -> Result<crate::engine::broker::admission::Held, Unadmitted> {
    fixture
        .host
        .admit(&boxed_claim(&fixture.work, ["run-b", "b1"]))
}

/// The binding: an acknowledged, unfoldable run whose driver ended settled
/// releases its worktree, and a new claim in that worktree is admitted. The
/// proof reads the broker's record, not the run's fold, which still fails.
#[test]
fn a_quarantined_run_whose_writers_are_settled_releases_its_worktree() {
    let mut fixture = unacknowledged(&[Witnessed::Settled], None);
    assert_eq!(
        fold(&fixture.store.load("run-a").unwrap()).unwrap_err(),
        brokkr_core::FoldError::AfterTerminal { seq: 7 },
        "shape (B) does not fold"
    );
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "an unacknowledged run holds the worktree, its writers proven or not"
    );
    acknowledge(&mut fixture.store, "run-a");
    assert!(
        next(&fixture).is_ok(),
        "the acknowledged run's settled attempt releases the worktree"
    );
}

/// The binding's other half: one attempt's process group still alive keeps
/// the worktree held, with today's words. The record says every driver
/// settled; the host says a member of the group still runs.
#[test]
fn a_quarantined_run_with_a_live_process_group_still_holds_its_worktree() {
    if !isolated("engine::broker::exclusion::tests::a_quarantined_run_with_a_live_process_group_still_holds_its_worktree") {
        return;
    }
    let live = LiveGroup::new();
    let fixture = quarantined(&[Witnessed::Settled], Some(live.group()));
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the host still shows the attempt's process group, so it holds"
    );
    drop(live);
}

/// A child process that holds the run's lock keeps the worktree held, and
/// the refusal names the child (decision 0108 ruling 1): the kernel excludes
/// across processes, not only across open descriptions.
#[test]
fn a_quarantined_run_whose_lock_a_child_holds_is_refused_naming_it() {
    let fixture = quarantined(&[Witnessed::Settled], None);
    let held = child_lock(&fixture.store, "run-a");
    let refused = next(&fixture).unwrap_err();
    let words = refused.to_string();
    assert!(
        words.contains(&format!("run 'run-a' is driven by pid {}", held.holder())),
        "{words}"
    );
    drop(held);
    assert!(
        next(&fixture).is_ok(),
        "once the child lets go, the settled run releases"
    );
}

/// Acknowledgement alone never releases: a run with an attempt no record
/// proves keeps its worktree however it is looked at.
#[test]
fn an_acknowledged_run_whose_record_proves_nothing_still_holds() {
    let fixture = quarantined(&[Witnessed::Settling], None);
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "an unsettled driver with no group record keeps the worktree held"
    );
}

/// The same run repaired (440c): the repair record stands for the
/// acknowledgement a repair lapses (#511), so the settled attempt releases
/// the worktree and the run folds again.
#[test]
fn a_repaired_run_whose_writers_are_settled_releases_its_worktree() {
    let mut fixture = unacknowledged(&[Witnessed::Settled], None);
    operator_repair(
        &mut fixture.store,
        "run-a",
        &Repair {
            operator: "repairer",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    let state = fold(&fixture.store.load("run-a").unwrap()).unwrap();
    assert_eq!(state.run, Run::Stopped, "the repaired run folds again");
    assert_eq!(
        fixture.store.acknowledgement("run-a").unwrap(),
        None,
        "the repair moved the head past the acknowledgement"
    );
    assert!(
        next(&fixture).is_ok(),
        "the repaired run's settled attempt releases the worktree"
    );
}

/// Where the records prove nothing, the operator's settle record is the
/// proof the exclusion honours: a run whose only driver is not sealed
/// Settled, and whose recorded process group is gone from the host, is
/// released once `brokkr operator settle` names it. This is 440c's F5 test
/// rewritten to the new truth.
#[test]
fn a_settle_record_releases_what_the_records_cannot_prove() {
    let mut fixture = quarantined(&[Witnessed::Settling], Some(dead_group()));
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the broker's own records do not prove a Settling driver ended"
    );
    let written = operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason: "the group is gone",
            operator: "operator",
        },
    )
    .expect("the operator proves the group gone");
    assert_eq!(written.payload["command"], "settle");
    assert!(
        next(&fixture).is_ok(),
        "the settle record is the proof the exclusion honours"
    );
}

/// A settle record that names an attempt the host still shows alive does
/// not release, even with the lock free.
#[test]
fn a_settle_record_naming_a_live_group_does_not_release() {
    use crate::engine::broker::settlement::release::{LockState, Proof, Proven, SettleArgs};
    if !isolated(
        "engine::broker::exclusion::tests::a_settle_record_naming_a_live_group_does_not_release",
    ) {
        return;
    }
    let live = LiveGroup::new();
    let mut fixture = quarantined(&[Witnessed::Settling], Some(live.group()));
    // A record written when the group was still live: it names the attempt
    // and its group, and the exclusion asks the host again.
    let args = SettleArgs {
        run: "run-a".into(),
        lock: LockState::Free,
        attempts: vec![Proven {
            attempt: "a1".into(),
            proof: Proof::Records,
            group: Some(live.group()),
        }],
        reason: "the group was gone when I looked".into(),
    };
    let payload = json!({
        "command_id": "c-settle",
        "command": brokkr_core::fold::SETTLE,
        "operator": "operator",
        "args": args,
    });
    let (seq, hash) = fixture.store.head_hash("run-a").unwrap();
    fixture
        .store
        .append_next_if_head(
            "run-a",
            seq,
            &hash,
            EventType::OperatorCommanded,
            payload,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the record names a group the host still shows alive"
    );
    drop(live);
}

/// A journal any participant's writer could reach proves nothing, whatever
/// it says: an unkeyed chain a writer can forge is no proof (R2). The
/// release refuses rather than trust it.
#[test]
fn a_journal_a_writer_could_reach_proves_nothing() {
    let fixture = quarantined(&[Witnessed::Settled], None);
    let inside = fixture.work.join("forge.db");
    std::fs::copy(&fixture.db, &inside).unwrap();
    let at = crate::engine::broker::admission::tests::reservation_path(
        fixture.host.home(),
        &fixture.work,
        "run-a",
    );
    let bytes = std::fs::read(&at).unwrap();
    let mut reserved: crate::engine::broker::admission::Reservation =
        serde_json::from_slice(&bytes).unwrap();
    reserved.journal = inside;
    std::fs::write(&at, serde_json::to_vec(&reserved).unwrap()).unwrap();
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "a reachable journal is no proof"
    );
}

/// A released run's later commands still refuse: the release frees the
/// worktree, not the run. `retry` and `stop` meet the terminal gate (or the
/// unfoldable journal) before they write, and a `resume` meets the fold.
#[test]
fn a_released_run_still_refuses_retry_and_stop() {
    use crate::engine::operator::{operator_stop, Cause, Named};
    let mut fixture = unacknowledged(&[Witnessed::Settled], None);
    acknowledge(&mut fixture.store, "run-a");
    assert!(next(&fixture).is_ok(), "the worktree is released");
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = crate::engine::operator_command(
        &mut fixture.store,
        "run-a",
        brokkr_core::fold::OperatorCommand::Retry,
        "operator",
        "again",
    );
    assert!(
        matches!(
            refused,
            Err(crate::engine::EngineError::Fold(_)
                | crate::engine::EngineError::AlreadyConcluded { .. })
        ),
        "retry: {refused:?}"
    );
    let cited = brokkr_store::Store::open_read_only(fixture.store.path()).unwrap();
    let named = Named {
        cause: Cause {
            ending: brokkr_core::ending::Ending::Withdrawn,
            by_realm: None,
            by_run: None,
            by_ref: None,
        },
        reason: "again",
        operator: "operator",
    };
    let refused = operator_stop(&mut fixture.store, &cited, "run-a", &named);
    assert!(
        matches!(
            refused,
            Err(crate::engine::EngineError::Fold(_)
                | crate::engine::EngineError::AlreadyConcluded { .. })
        ),
        "stop: {refused:?}"
    );
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "neither command wrote anything"
    );
}
