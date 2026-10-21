//! U8e (CR3, CR4, MB5; D7): every route by which an attempt's drivers end
//! meets the settlement barrier before any result of it is admitted, and
//! evidence is read only once every process the attempt owned is proven
//! settled. A deterministic harness reports success whatever its broker
//! latched; the causes are those the broker's observed stages latch (MB3,
//! MB4), laid in its ledger by the protocol's own writer, as in the parent
//! suite. A real panel, a dispatched driver killed at its deadline, a
//! driver whose door refused it, and a restarted engine are each driven
//! here; the sibling-witness order is driven through the panel's own
//! settlement ([`Engine::settle_members`]) with reports a test can make
//! unresolved.

use std::time::Duration;

use brokkr_core::fold::Cursor;
use brokkr_protocol::broker::Refusal;
use brokkr_store::StoreError;
use rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER;
use rusqlite::Error::SqliteFailure;

use super::*;
use crate::engine::broker::admission::recheck;
use crate::engine::broker::admission::tests::last;
use crate::engine::broker::settlement::witnessed;
use crate::engine::checkpoints::{self, Checkpoints};
use crate::engine::panel::MemberRun;
use crate::engine::tests::{fail_event, state};

/// Why the restarted engine settles an attempt it finds in flight.
const RESTARTED: &str =
    "engine restarted while the attempt was in flight; completion cannot be established";

/// A cleanup a driver ended with, its tree not proven over.
fn unresolved() -> Cleanup {
    let reason = Unsettled::Stdout;
    Cleanup::Unresolved { reason }
}

/// The panel member `name`, run by `spawn` at the seat `work`.
fn member(name: &str, spawn: SiteSpawn) -> MemberRun {
    MemberRun {
        name: name.into(),
        driver_seat: "work".into(),
        boundary: None,
        offer: None,
        context: None,
        input: serving_input(&spawn),
        spawn,
    }
}

/// The ledger `serve` writes for the site's attempt beside the plan
/// `broker` was sealed with.
fn serve_beside(site: &Site, broker: &SealedBroker, serve: impl FnOnce(Ledger<'static>)) {
    let bytes = written((&site.attempt.owner(), &broker.plan_digest), serve);
    made(&PathBuf::from(format!("{}.ledger", broker.plan)), &bytes);
}

/// D7, CR4: a broker's half-written record is not read while any driver
/// of its attempt is not proven settled, a sibling's included, so no
/// premature corruption is found and nothing is appended; the same record
/// in an attempt whose every process settled refuses exactly, its verified
/// call kept and its begun one interrupted.
#[test]
fn a_live_half_record_is_not_read_while_any_driver_of_its_attempt_is_unresolved() {
    let mut live = Site::folding();
    live.laid(&Fault::Partial.ledger(&live));
    let before = fs::read(live.ledger()).unwrap();
    witnessed(&mut live.engine.held, &unresolved());
    let folded = live.fold(succeeded());
    assert_eq!((folded.refused, live.rows()), (None, vec![]));
    assert_eq!(fs::read(live.ledger()).unwrap(), before);

    let mut settled = Site::folding();
    settled.laid(&Fault::Partial.ledger(&settled));
    let folded = settled.fold(succeeded());
    let cause = "broker ledger ends in a partial record";
    assert_eq!(folded.settled_outcome(), failed(cause));
    let rows = [
        row(1, "lookup", "succeeded"),
        row(2, "lookup", "interrupted"),
    ];
    assert_eq!(settled.rows(), rows);
}

/// D7: a panel's members are all witnessed before any member's evidence
/// is read, so a member that settled is not read while a sibling declared
/// after it ended unresolved, whose broker may still write.
#[test]
fn every_member_is_witnessed_before_any_member_is_read() {
    let mut site = Site::folding();
    site.laid(&Fault::Partial.ledger(&site));
    let runs = [
        member("docs", site.spawn.clone()),
        member("sibling", brokered(Vec::new(), vec!["must-not-run".into()])),
    ];
    let begun = site.engine.begun(&site.spawn).unwrap();
    let sibling = AttemptReport {
        cleanup: unresolved(),
        ..report(succeeded(), "")
    };
    let reports = vec![
        ("docs".to_string(), report(succeeded(), ""), begun),
        ("sibling".to_string(), sibling, None),
    ];
    let engine = &mut site.engine;
    let run = engine.run_id.clone();
    let attempt = checkpoints::Attempt {
        run_id: &run,
        effect_id: EFFECT,
        attempt_id: ATTEMPT,
    };
    let settled = Checkpoints::new(&mut engine.store, &mut engine.current_cause, attempt).settle();
    let members = engine.settle_members([EFFECT, ATTEMPT], &runs, reports, settled, "");
    let refused = (members.unwrap().into_iter()).map(|(name, report)| (name, report.refused));
    let refused: Vec<_> = refused.collect();
    let unchanged = [("docs".to_string(), None), ("sibling".to_string(), None)];
    assert_eq!((refused, site.rows()), (unchanged.to_vec(), vec![]));
}

/// CR4, CR3: every panel member's report is settled by its own brokers'
/// calls once every member ended, before the panel admits it: a member
/// whose broker answered keeps its success and its call; one whose
/// harness reported success after its box failed fails with that exact
/// cause and no call; and one whose door refused it started no broker, so
/// its failed start stands and no ledger is missing for it.
#[test]
fn every_panel_member_is_settled_by_its_own_broker_calls() {
    let argv = checkpointing_command(EFFECT, ATTEMPT, &[], succeeded());
    let mut site = Site::new(false, argv.clone());
    site.serve(|mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let wiki = site.pinned("wiki");
    serve_beside(&site, &wiki, |mut ledger| {
        ledger.fail(Failure::Refused(Refusal::Establishment));
        ledger.close().unwrap();
    });
    let mut refused = brokered(vec![site.pinned("notes")], argv.clone());
    refused.refusal = Some("the door refused it".into());
    let runs = [
        member("docs", site.spawn.clone()),
        member("wiki", brokered(vec![wiki], argv)),
        member("notes", refused),
    ];
    let deadline = Duration::from_secs(10);
    let reports = site.engine.run_panel(EFFECT, ATTEMPT, &runs, deadline, "");
    let outcomes: Vec<_> = (reports.unwrap().into_iter())
        .map(|(name, report)| (name, report.settled_outcome()))
        .collect();
    let expected = [
        ("docs".to_string(), succeeded()),
        (
            "wiki".to_string(),
            failed("MCP server box could not be established"),
        ),
        (
            "notes".to_string(),
            failed("member driver did not spawn: the door refused it"),
        ),
    ];
    assert_eq!(outcomes, expected);
    assert_eq!(site.rows(), [row(1, "lookup", "succeeded")]);
}

/// CR3: a single driver whose door refused it started no broker: its
/// attempt fails as the failed start it is, with no call and no missing
/// ledger.
#[test]
fn a_driver_that_did_not_spawn_is_a_failed_start_without_a_missing_ledger() {
    let mut site = Site::folding();
    site.spawn.refusal = Some("the door refused it".into());
    let last = concluded(&mut site, Duration::from_secs(10)).unwrap();
    let error = json!({
        "effect_id": EFFECT, "attempt_id": ATTEMPT, "error": "driver did not spawn: the door refused it",
    });
    assert_eq!(
        (last.event_type, last.payload),
        (EventType::EffectFailed, error)
    );
    assert_eq!(site.rows(), Vec::<Value>::new());
}

/// CR4, MB3, MB4: a harness that reports success after its broker failed
/// at an observed stage fails with exactly that stage's cause and no call:
/// the box before the secret wait, its readiness, the secret handoff (a
/// handoff failure, or a status lost before the sender finished), the
/// boxed exec (an exec failure, or a status lost after it), and a status
/// pipe's end followed by no functioning child, which fails initialization
/// or times out and never closes clean. A closure that could not persist
/// keeps the verified call before it and the lifecycle's refusal.
#[test]
fn a_successful_harness_fails_with_its_brokers_observed_stage_cause() {
    let argv = checkpointing_command(EFFECT, ATTEMPT, &[], succeeded());
    let stages = [
        Failure::Refused(Refusal::Establishment),
        Failure::Refused(Refusal::Unavailable),
        Failure::Refused(Refusal::Handoff),
        Failure::Refused(Refusal::Exec),
        Failure::Protocol,
        Failure::Timeout,
    ];
    for failure in stages {
        let mut site = Site::new(false, argv.clone());
        let cause = format!("{failure}; stderr tail: ");
        site.serve(|mut ledger| {
            ledger.fail(failure);
            ledger.close().unwrap();
        });
        let last = concluded(&mut site, Duration::from_secs(10)).unwrap();
        let error = &last.payload["error"];
        assert_eq!(
            (last.event_type, error),
            (EventType::EffectFailed, &json!(cause))
        );
        assert_eq!(site.rows(), Vec::<Value>::new(), "{cause}");
    }
    let mut site = Site::new(false, argv);
    site.laid(&Fault::Unclosed.ledger(&site));
    let last = concluded(&mut site, Duration::from_secs(10)).unwrap();
    let cause = json!("broker ledger lifecycle is invalid; stderr tail: ");
    assert_eq!(
        (last.event_type, &last.payload["error"]),
        (EventType::EffectFailed, &cause)
    );
    assert_eq!(site.rows(), [row(1, "lookup", "succeeded")]);
}

/// MB5, CR4: a driver killed at its deadline, its broker cut off mid-call,
/// has its tree settled before its evidence is read: its answered call
/// and its begun one, interrupted, are each journaled once, and the
/// attempt is not admitted as anything but what the kill left.
#[test]
fn a_driver_killed_at_its_deadline_settles_its_interrupted_call() {
    let reason = String::new();
    let mut argv = checkpointing_command(
        EFFECT,
        ATTEMPT,
        &[],
        AttemptOutcome::Indeterminate { reason },
    );
    argv[2].push_str("; sleep 30");
    let mut site = Site::new(false, argv);
    site.serve(|mut ledger| {
        answered(&mut ledger, 1);
        let _cut = ledger.room().unwrap().begin("lookup").unwrap();
    });
    let last = concluded(&mut site, Duration::from_secs(1)).unwrap();
    let reason = "attempt exceeded its 1s deadline and was killed; its process tree is gone, and \
                  what it did is unknown; stderr tail: ";
    let killed = json!({"effect_id": EFFECT, "attempt_id": ATTEMPT, "reason": reason});
    assert_eq!(
        (last.event_type, last.payload),
        (EventType::EffectIndeterminate, killed)
    );
    let rows = [
        row(1, "lookup", "succeeded"),
        row(2, "lookup", "interrupted"),
    ];
    assert_eq!(site.rows(), rows);
}

/// The engine of `site` as a restart finds it, the attempt in flight and
/// no lease held, since only the durable reservation outlives the engine
/// that died: it settles the attempt as it finds it.
fn restarted(site: &mut Site) {
    let engine = &mut site.engine;
    engine.held = None;
    let run = engine.run_id.clone();
    let in_flight = Cursor::EffectInFlight {
        effect_id: EFFECT.into(),
        attempt_id: ATTEMPT.into(),
        seat: "work".into(),
        failed_attempts: 0,
        stop_rides: false,
    };
    let events = engine.store.load(&run).unwrap();
    let current = state(Some("work"), in_flight.clone());
    engine.advance_running(&events, in_flight, current).unwrap();
    let settled = json!({"effect_id": EFFECT, "attempt_id": ATTEMPT, "reason": RESTARTED});
    assert_eq!(last(engine), (EventType::EffectIndeterminate, settled));
}

/// Whether `ended` is the storage failure the journal's forced refusal of
/// a checkpoint hands up: SQLite's own trigger abort, with the message
/// `fail_event`'s trigger raises.
fn forced<T>(ended: &Result<T, EngineError>) -> bool {
    let Err(EngineError::Store(StoreError::Sqlite(SqliteFailure(error, message)))) = ended else {
        return false;
    };
    let trigger = error.extended_code == SQLITE_CONSTRAINT_TRIGGER;
    trigger & (message.as_deref() == Some("forced event failure"))
}

/// CR4, D6: an attempt its engine could not conclude, its calls journaled
/// before the engine died or a storage failure handed up before its
/// report, is settled by a restarted engine without a read: it holds no
/// lease and no proof that the attempt's processes settled, so the
/// attempt is indeterminate, no call is appended again or for the first
/// time, the ledger is as the broker left it, no driver or server runs
/// again, and the attempt's reservation still stands.
#[test]
fn an_attempt_its_engine_could_not_conclude_is_restarted_indeterminate_without_a_read() {
    let mut died = Site::folding();
    died.serve(|mut ledger| {
        answered(&mut ledger, 1);
        ledger.fail(Failure::Protocol);
        ledger.close().unwrap();
    });
    let folded = died.fold(succeeded());
    assert_eq!(
        folded.settled_outcome(),
        failed("MCP server protocol is invalid")
    );
    let (ledger, held) = (fs::read(died.ledger()).unwrap(), died.engine.held.clone());
    restarted(&mut died);
    assert_eq!(died.rows(), [row(1, "lookup", "succeeded")]);
    assert_eq!(fs::read(died.ledger()).unwrap(), ledger);
    assert_eq!(recheck(held.as_ref().unwrap()), Ok(()));

    let inner = json!({"step": "inner"});
    let argv = checkpointing_command(EFFECT, ATTEMPT, &[inner], succeeded());
    let mut failed_store = Site::new(false, argv);
    failed_store.serve(|mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let ledger = fs::read(failed_store.ledger()).unwrap();
    fail_event(failed_store.engine.store.path(), "effect/checkpointed");
    let handed = concluded(&mut failed_store, Duration::from_secs(10));
    assert!(forced(&handed), "{handed:?}");
    let held = failed_store.engine.held.clone();
    let settled = failed_store.engine.settle(ATTEMPT, handed.map(drop));
    assert_eq!(recheck(held.as_ref().unwrap()), Ok(()));
    assert!(forced(&settled), "{settled:?}");
    restarted(&mut failed_store);
    assert_eq!(failed_store.rows(), Vec::<Value>::new());
    assert_eq!(fs::read(failed_store.ledger()).unwrap(), ledger);
    assert_eq!(recheck(held.as_ref().unwrap()), Ok(()));
}
