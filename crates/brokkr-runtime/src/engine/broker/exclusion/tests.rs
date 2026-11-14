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
    acknowledge, boxed_claim, child_lock, dead_group, isolated, quarantined, quarantined_drivers,
    unacknowledged, Fixture, LiveGroup,
};
use crate::engine::broker::admission::{Conflict, Unadmitted, Witnessed};
use crate::engine::broker::settlement::release;
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

/// The same run acknowledged and then repaired (440c): the repair record
/// stands for the acknowledgement a repair lapses (#511), so the settled
/// attempt releases the worktree and the run folds again.
#[test]
fn a_repaired_run_whose_writers_are_settled_releases_its_worktree() {
    let mut fixture = quarantined(&[Witnessed::Settled], None);
    assert!(
        fixture.store.acknowledgement("run-a").unwrap().is_some(),
        "the run is acknowledged before the repair lapses it"
    );
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
    use crate::engine::broker::settlement::release::{Proof, Proven, SettleArgs};
    if !isolated(
        "engine::broker::exclusion::tests::a_settle_record_naming_a_live_group_does_not_release",
    ) {
        return;
    }
    let live = LiveGroup::new();
    let mut fixture = quarantined(&[Witnessed::Settling], Some(live.group()));
    // A record written when the group was still live: it names the attempt
    // and its group, and the exclusion asks the host again. Its proof is
    // `group`, so only `living` refuses it.
    let args = SettleArgs {
        run: "run-a".into(),
        attempts: vec![Proven {
            attempt: "a1".into(),
            proof: Proof::Group,
            groups: vec![live.group()],
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
/// worktree, not the run. `retry` and `stop` meet the terminal gate before
/// they write, exactly, and `resume` meets the run's fold.
#[test]
fn a_released_run_still_refuses_retry_and_stop() {
    use crate::engine::operator::{operator_stop, Cause, Named};
    use crate::engine::EngineError;
    use brokkr_core::fold::OperatorCommand;
    let mut fixture = quarantined(&[Witnessed::Settled], None);
    operator_repair(
        &mut fixture.store,
        "run-a",
        &Repair {
            operator: "repairer",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    // The fixture's planted tail leaves its summary behind its head; this
    // test reads the terminal gate, not the summary.
    brokkr_store::test_support::lags(&mut fixture.store, &["run-a"]);
    assert_eq!(
        fixture.store.acknowledgement("run-a").unwrap(),
        None,
        "the repair lapsed the acknowledgement it followed"
    );
    assert!(next(&fixture).is_ok(), "the worktree is released");
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = crate::engine::operator_command(
        &mut fixture.store,
        "run-a",
        OperatorCommand::Retry,
        "operator",
        "again",
    );
    assert!(
        matches!(
            refused,
            Err(EngineError::AlreadyConcluded {
                verb: OperatorCommand::Retry,
                ..
            })
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
            Err(EngineError::AlreadyConcluded {
                verb: OperatorCommand::Stop,
                ..
            })
        ),
        "stop: {refused:?}"
    );
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "neither command wrote anything"
    );
}

/// A panel attempt has one driver per member, each leading its own process
/// group. A settle record must prove *every* one: with only one member's
/// group gone, the other member's own group keeps the worktree held, and
/// once both are gone the same settle record releases it.
#[test]
fn a_panel_attempt_with_one_live_group_still_holds_its_worktree() {
    if !isolated(
        "engine::broker::exclusion::tests::a_panel_attempt_with_one_live_group_still_holds_its_worktree",
    ) {
        return;
    }
    let mut g1 = LiveGroup::new();
    let mut g2 = LiveGroup::new();
    let (mut fixture, held) =
        quarantined_drivers(&[("m1", Witnessed::Settling), ("m2", Witnessed::Settling)]);
    release::spawned(&held, "m1", g1.group());
    release::spawned(&held, "m2", g2.group());
    // The second member's group only is gone.
    g2.end();
    let head = fixture.store.head_hash("run-a").unwrap();
    let refused = operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason: "both are done",
            operator: "operator",
        },
    )
    .unwrap_err();
    let said = refused.to_string();
    assert!(
        said.contains(&format!(
            "attempt 'a1' stands, and its process group {} still has members on this host",
            g1.group()
        )),
        "{said}"
    );
    assert_eq!(
        fixture.store.head_hash("run-a").unwrap(),
        head,
        "the refusal wrote nothing"
    );
    assert_eq!(
        next(&fixture),
        cr2(Conflict::Held("run-a".into())),
        "the live member keeps the worktree held"
    );
    // Both are gone now: the same proof releases.
    g1.end();
    operator_settle(
        &mut fixture.store,
        "run-a",
        &Settle {
            reason: "both are done",
            operator: "operator",
        },
    )
    .expect("every driver's own group is gone");
    assert!(
        next(&fixture).is_ok(),
        "both groups gone admits the next claim"
    );
}

/// The route for a reservation written before this build (v0.13.0's shape:
/// drivers, no process group): `operator settle` cannot prove it, and the
/// operator repairs the run and then uses the existing
/// `brokkr operator release` attestation. The 0.12.0-era journal is the
/// shape (B) one; the reservation is the first released shape that records
/// drivers, because 0.12.0 wrote no reservation at all. The next claim is
/// admitted only after that release.
#[test]
fn an_old_shape_b_reservation_is_released_by_repair_then_operator_release() {
    use crate::engine::broker::admission::tests::{Coordinated, Host};
    use crate::engine::broker::settlement::Driver;
    use brokkr_core::fold::OperatorCommand;

    let host = Host::new();
    let work = host.worktree("work");
    let db = host.journal("run-a.db");
    let mut store = crate::engine::tests::in_flight_store(&db, "run-a");
    store
        .append_next(
            "run-a",
            EventType::EffectIndeterminate,
            json!({"effect_id": "effect", "attempt_id": "attempt"}),
            None,
            Some("attempt".into()),
        )
        .unwrap();
    store
        .append_next(
            "run-a",
            EventType::RunStopped,
            json!({"reason": "concluded"}),
            None,
            None,
        )
        .unwrap();
    let head = store.load("run-a").unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_chained(
        &connection,
        "run-a",
        &head,
        &crate::late::late_stop(),
    )
    .unwrap();
    drop(connection);
    let mut store = brokkr_store::Store::open(&db).unwrap();
    // v0.13.0 wrote the drivers' own records and no process group: 0.12.0
    // wrote no reservation at all (see the commit message's tag facts).
    let mut claim = boxed_claim(&work, ["run-a", "attempt"]);
    claim.journal = db.clone();
    claim.reserved.drivers = vec![Driver {
        site: "work".into(),
        begun: None,
        cleanup: Witnessed::Settling,
        group: None,
    }];
    host.admit(&claim).expect("the old reservation is admitted");
    acknowledge(&mut store, "run-a");
    let _stood = Coordinated::at(host.home());
    assert_eq!(
        host.admit(&boxed_claim(&work, ["run-b", "b1"])),
        cr2(Conflict::Held("run-a".into())),
        "the old reservation holds the worktree"
    );
    let refused = operator_settle(
        &mut store,
        "run-a",
        &Settle {
            reason: "the driver is gone",
            operator: "operator",
        },
    )
    .unwrap_err();
    assert!(
        refused.to_string().contains("recorded no process group"),
        "settle cannot prove a reservation with no group: {refused}"
    );
    operator_repair(
        &mut store,
        "run-a",
        &Repair {
            operator: "operator",
            reason: "0.12.0 left a stop after the conclusion",
        },
    )
    .expect("the repair names the late tail");
    crate::engine::operator_command(
        &mut store,
        "run-a",
        OperatorCommand::Release,
        "operator",
        "every process the attempt owned has ended",
    )
    .expect("the operator's release attests the old attempt ended");
    assert!(
        host.admit(&boxed_claim(&work, ["run-b", "b1"])).is_ok(),
        "the released old run no longer holds the worktree"
    );
}

/// A repaired run still meets its fold on `resume`: the run reads as the
/// status its terminal event states, the resume is not a second driver, and
/// the run is the same stopped run. This is the command the release does not
/// make lawful; it is asserted here because the released run's later
/// commands were only ever checked for `retry` and `stop`.
#[test]
fn a_repaired_run_resumes_to_its_fold() {
    use crate::engine::operator::{Cause, Named};
    use brokkr_core::fold::Run;

    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let db = dir.path().join("forge.db");
    let bundle = crate::engine::tests::bundle(
        dir.path(),
        crate::engine::tests::single_body(vec!["missing-driver".into()]),
    );
    let mut engine = crate::engine::Engine::start(
        brokkr_store::Store::open(&db).unwrap(),
        bundle.clone(),
        "resume",
        Some(work.clone()),
    )
    .unwrap();
    let run = engine.run_id.clone();
    let _ = engine.drive();
    drop(engine);
    let mut store = brokkr_store::Store::open(&db).unwrap();
    let cited = brokkr_store::Store::open_read_only(&db).unwrap();
    let named = Named {
        cause: Cause {
            ending: brokkr_core::ending::Ending::Withdrawn,
            by_realm: None,
            by_run: None,
            by_ref: None,
        },
        reason: "concluded",
        operator: "operator",
    };
    let state = crate::engine::conclude(&mut store, &cited, &run, &named).unwrap();
    assert_eq!(state.run, Run::Stopped);
    let head = store.load(&run).unwrap().pop().unwrap();
    drop(store);
    let connection = rusqlite::Connection::open(&db).unwrap();
    brokkr_store::test_support::plant_chained(&connection, &run, &head, &crate::late::late_stop())
        .unwrap();
    drop(connection);
    let mut store = brokkr_store::Store::open(&db).unwrap();
    crate::engine::operator_repair(
        &mut store,
        &run,
        &Repair {
            operator: "operator",
            reason: "kept as evidence",
        },
    )
    .expect("the repair names the late tail");
    let mut resumed = crate::engine::Engine::resume(
        brokkr_store::Store::open(&db).unwrap(),
        bundle,
        &run,
        Some(work),
    )
    .expect("the repaired run resumes");
    let end = resumed.drive().unwrap();
    assert_eq!(end.state.run, Run::Stopped, "resume meets the run's fold");
}
