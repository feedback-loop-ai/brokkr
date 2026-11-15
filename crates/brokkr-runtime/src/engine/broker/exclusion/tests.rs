//! Decision 0108 ruling 4's worktree release, as far as this unit owns it.
//!
//! Repair makes a broken run fold again as its terminal status. It does
//! NOT release the worktree, because the broker's exclusion counts the
//! run's standing attempts in its own reservation records, not its fold:
//! `leased` refuses any other run while an attempt stands, and an attempt
//! whose engine never sealed its positive settlement fact still stands
//! after the repair. That release is decision 0108 ruling 3 (440d).

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Run};
use brokkr_store::test_support::{lags, plant_chained};
use brokkr_store::{Attribution, Store};
use serde_json::json;
use std::path::Path;

use crate::engine::broker::admission::tests::{cr2, open, Host};

use crate::engine::broker::admission::Conflict;
use crate::engine::operator::{operator_repair, Repair};

/// Run `run`'s journal at `db`, stopped at seq 5.
fn concluded(db: &Path, run: &str) {
    let mut store = crate::engine::tests::in_flight_store(db, run);
    store
        .append_next(
            run,
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
}

/// Shape (B) planted behind the store: the stop command and acceptance
/// 0.12.0 left after the conclusion.
fn broken(db: &Path, run: &str) -> Store {
    let store = Store::open(db).unwrap();
    let head = store.load(run).unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(db).unwrap();
    plant_chained(&connection, run, &head, &crate::late::late_stop()).unwrap();
    drop(connection);
    Store::open(db).unwrap()
}

/// A worktree held by run A's standing attempt is still held after run A
/// is repaired: the run folds as stopped, and the broker's own record
/// keeps a1 standing because no positive settlement fact was ever sealed.
/// Releasing it is decision 0108 ruling 3, unit 3b (440d).
#[test]
fn a_repaired_run_still_holds_its_worktree_until_its_attempt_is_settled() {
    let host = Host::new();
    let work = host.worktree("work");
    let held = host
        .admit(&open(&work, ["run-a", "a1"]))
        .expect("run A's attempt is admitted");
    let second = || host.admit(&open(&work, ["run-b", "b1"]));
    assert_eq!(
        second(),
        cr2(Conflict::Held("run-a".into())),
        "run A holds the worktree while a1 stands"
    );

    let db = work.join("forge.db");
    concluded(&db, "run-a");
    let mut store = broken(&db, "run-a");
    assert_eq!(
        fold(&store.load("run-a").unwrap()).unwrap_err(),
        brokkr_core::FoldError::AfterTerminal { seq: 7 },
        "shape (B) does not fold"
    );
    // 0.12.0's operator acknowledged the quarantined run before anything
    // else: the acknowledgement stands beside the chain at the broken
    // head. The worktree is still held after it, because admission reads
    // the broker's reservation, never the journal's fold.
    let (seq, hash) = store.head_hash("run-a").unwrap();
    store
        .acknowledge_quarantine(
            "run-a",
            (seq, &hash),
            Attribution {
                operator: "operator",
                reason: "0.12.0 left a stop after the conclusion",
            },
        )
        .expect("the broken run is acknowledged where it stands");
    assert!(
        store.acknowledgement("run-a").unwrap().is_some(),
        "the acknowledgement stands at the broken head"
    );
    assert_eq!(
        second(),
        cr2(Conflict::Held("run-a".into())),
        "acknowledging the run does not release the worktree"
    );

    operator_repair(
        &mut store,
        "run-a",
        &Repair {
            operator: "repairer",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    let state = fold(&store.load("run-a").unwrap()).unwrap();
    assert_eq!(state.run, Run::Stopped, "the repaired run folds again");
    assert!(state.late_tail.is_some());
    assert_eq!(
        store.acknowledgement("run-a").unwrap(),
        None,
        "the repair moved the head past the acknowledgement; the fold is the record now"
    );
    lags(&mut store, &["run-a"]);

    let (standing, settled) = host.attempts(&work, "run-a");
    assert_eq!(standing, ["a1"], "the broker's record still names a1");
    assert!(
        settled.is_empty(),
        "no positive settlement fact was sealed for a1"
    );
    assert_eq!(
        second(),
        cr2(Conflict::Held("run-a".into())),
        "`operator repair` alone does NOT release the worktree a broken run held: admission \
         reads the broker's own reservation, and a1 still stands because no positive \
         settlement fact was sealed. Decision 0108 ruling 3 (440d) is what releases it"
    );
    let _ = held;
}
