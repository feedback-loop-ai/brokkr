//! U8e2 (CR4, MB5; D6; operator ruling 2026-10-21): an engine restarted
//! with an attempt in flight recovers it only where every driver the
//! attempt spawned has its own settlement record sealed Settled, folding
//! each from the journal head that record keeps; otherwise it keeps the
//! attempt indeterminate, reads nothing, and leaves its reservation
//! refusing a conflicting run. The engine that died is driven through the
//! real door and barrier, a single site, a sequence's steps or a panel's
//! members, until a forced journal failure stops its fold, or its records
//! are written as its door and barrier write them. The restart is
//! simulated in process: through the engine's own turn
//! ([`Engine::drive_once`]) where the journal shows the attempt in flight,
//! or as [`restart`] does.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use brokkr_core::fold::fold as folds;
use brokkr_protocol::broker::Owner;

use super::routes::{forced, restart, RESTARTED};
use super::*;
use crate::engine::broker::admission::tests::{conflicting, last, soon, Coordinated};
use crate::engine::broker::admission::{
    amend, recheck, Conflict, Reservation, Unadmitted, Unknown, Witnessed, RESERVATION,
};
use crate::engine::broker::settlement::{ended, spawning, Driver, Unreleased};
use crate::engine::fence::{tests::after_the_fold, tip_of};
use crate::engine::operator::conclude;
use crate::engine::panel::MemberRun;
use crate::engine::tests::fail_event;
use crate::engine::{spawn_site, Bundle, Door, DriverRun};

/// The trigger [`fail_event`] plants, which refuses every settled call.
const SETTLED: &str = "capability-call-settled";

impl Site {
    /// Where the run's reservation stands.
    fn reservation_at(&self) -> PathBuf {
        self.attempt.dir().parent().unwrap().join(RESERVATION)
    }

    /// The run's reservation as it stands.
    fn reservation(&self) -> Reservation {
        serde_json::from_slice(&fs::read(self.reservation_at()).unwrap()).unwrap()
    }

    /// The journal's path, its run's id, and what restarts it in a fresh
    /// process: the bundle it pins and its worktree.
    fn journal(&self) -> (PathBuf, String, Bundle, PathBuf) {
        let engine = &self.engine;
        let db = engine.store.path().to_path_buf();
        (
            db,
            engine.run_id.clone(),
            engine.bundle.clone(),
            engine.repo.clone(),
        )
    }

    /// The settlement records the attempt's reservation keeps.
    fn drivers(&self) -> Vec<Driver> {
        self.reservation().attempts[ATTEMPT].drivers.clone()
    }

    /// Whether the run's reservation settled its attempt, so none of it
    /// stands any longer.
    fn released(&self) -> bool {
        let reservation = self.reservation();
        reservation.attempts.is_empty() & reservation.settled.contains(ATTEMPT)
    }

    /// The record of the driver at `site` as the door writes it, Settling,
    /// beginning where the site's attempt began where `begun`.
    fn spawning(&self, site: &str, begun: bool) {
        let held = self.engine.held.as_ref().unwrap();
        let begun = begun.then_some(&self.begun);
        assert_eq!(spawning(held, site, begun), Ok(()));
    }

    /// The site's driver spawned at `work` and ended settled, its record
    /// sealed as the barrier seals it, before its engine died.
    fn settled_at_work(&mut self) {
        self.spawning("work", true);
        ended(&mut self.engine.held, "work", &Cleanup::Settled);
    }

    /// The journal at the head now: what a driver spawned now begins at.
    fn head(&self) -> (u64, String) {
        self.engine.store.head_hash(&self.engine.run_id).unwrap()
    }

    /// The run driven to its effect's attempt `a1` opened, as dispatch
    /// opens it, so the journal shows it in flight: the effect's id.
    fn in_flight(&mut self) -> String {
        let engine = &mut self.engine;
        engine.drive_once().unwrap();
        engine.drive_once().unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();
        let effect = events.last().unwrap().payload["effect_id"].clone();
        let started = json!({"effect_id": effect, "attempt_id": ATTEMPT, "driver": "none"});
        let attempt = Some(ATTEMPT.to_string());
        engine
            .append(EventType::EffectStarted, started, attempt)
            .unwrap();
        effect.as_str().unwrap().to_string()
    }

    /// The journal made to refuse every settled call from now on, as an
    /// engine killed before its fold leaves it.
    fn dying(&self) {
        fail_event(self.engine.store.path(), SETTLED);
    }

    /// The journal healed of [`Site::dying`]'s refusal.
    fn healed(&self) {
        let journal = rusqlite::Connection::open(self.engine.store.path()).unwrap();
        journal
            .execute_batch("DROP TRIGGER forced_event_failure")
            .unwrap();
    }

    /// The site's driver run through the door and the barrier, its fold
    /// stopped by a journal that refuses every settled call, as an engine
    /// killed before its fold leaves it; then the journal healed.
    fn died_before_its_fold(&mut self) {
        self.dying();
        let died = concluded(self, Duration::from_secs(10));
        assert!(forced(&died), "the fold was stopped: {died:?}");
        self.healed();
    }

    /// One turn of the engine a fresh process restarts the run with, as
    /// the run's turn takes it: no lease held, the journal folded, its
    /// fence armed at that fold's head, in the coordination root the lease
    /// stood in.
    fn turn(&mut self) -> Result<Option<crate::engine::DriveEnd>, EngineError> {
        let _stood = Coordinated::at(&self.home());
        self.engine.held = None;
        self.engine.drive_once()
    }

    /// The event types the journal holds past its first `from`.
    fn after(&self, from: usize) -> Vec<EventType> {
        let events = self.engine.store.load(&self.engine.run_id).unwrap();
        (events[from..].iter().map(|event| event.event_type)).collect()
    }

    /// How many events the journal holds.
    fn length(&self) -> usize {
        self.engine.store.load(&self.engine.run_id).unwrap().len()
    }

    /// One more plan beside the first, serving `capability` and owned by
    /// the site labelled `site`, as provisioning seals a step's or a
    /// member's own, its ledger laid as `serve` writes it: its broker.
    fn owned(
        &self,
        capability: &str,
        site: &str,
        serve: impl FnOnce(Ledger<'static>),
    ) -> SealedBroker {
        let broker = self.pinned(capability);
        let at = PathBuf::from(&broker.plan);
        let mut plan: Value = serde_json::from_slice(&fs::read(&at).unwrap()).unwrap();
        plan["owner"]["site"] = json!(site);
        let bytes = serde_json::to_vec(&plan).unwrap();
        fs::write(&at, &bytes).unwrap();
        let digest = sha(&bytes);
        let inventory = self.attempt.dir().join("inventory.json");
        let mut pins: Inventory = serde_json::from_slice(&fs::read(&inventory).unwrap()).unwrap();
        let pin = pins.plans.iter_mut().find(|pin| pin.locator == at).unwrap();
        pin.digest = digest.clone();
        fs::write(&inventory, serde_json::to_vec(&pins).unwrap()).unwrap();
        let owner: Owner = serde_json::from_value(plan["owner"].clone()).unwrap();
        made(
            &PathBuf::from(format!("{}.ledger", broker.plan)),
            &written((&owner, &digest), serve),
        );
        SealedBroker {
            plan_digest: digest,
            ..broker
        }
    }
}

/// The settled checkpoint of call `call` to `lookup` of the plan serving
/// `capability` owned by the site labelled `site`, succeeded.
fn owned_row(site: &str, capability: &str, call: u64) -> Value {
    let server = format!("cap-{capability}");
    let tuple = json!({
        "attempt": ATTEMPT, "site": site, "instance": "work#0", "server": server, "call": call,
    });
    let call_id = format!("m-{}", canonical::sha256_hex(&tuple));
    let mut row = served(capability, call, "lookup", "succeeded");
    row["call_id"] = json!(call_id);
    row
}

/// The attempt's end as a restarted engine journals it for the effect
/// `effect`: `event`, saying `said` under `key`.
fn ending_of(effect: &str, event: EventType, key: &str, said: &str) -> (EventType, Value) {
    let ended = json!({"effect_id": effect, "attempt_id": ATTEMPT, key: said});
    (event, ended)
}

/// [`ending_of`] the fixtures' own effect.
fn ending(event: EventType, key: &str, said: &str) -> (EventType, Value) {
    ending_of(EFFECT, event, key, said)
}

/// The restarted engine's indeterminate end.
fn indeterminate() -> (EventType, Value) {
    ending(EventType::EffectIndeterminate, "reason", RESTARTED)
}

/// The refusal of a conflicting run on the worktree of the run `run`.
fn held_by(run: &str) -> Result<(), Unadmitted> {
    Err(Unadmitted::Evidence(Conflict::Held(run.into())))
}

/// The settlement record of the driver at `site`, beginning at `begun`,
/// sealed Settled.
fn settled_at(site: &str, begun: Option<(u64, String)>) -> Driver {
    let (site, cleanup) = (site.to_string(), Witnessed::Settled);
    Driver {
        site,
        begun,
        cleanup,
    }
}

/// A site whose spawn runs `argv`, its plan's ledger holding `calls`
/// answered calls and closed clean.
fn answering(argv: Vec<String>, calls: u64) -> Site {
    let site = Site::new(false, argv);
    site.serve(|mut ledger| {
        answered(&mut ledger, calls);
        ledger.close().unwrap();
    });
    site
}

/// [`answering`], its spawn a driver that checkpoints nothing and
/// succeeds.
fn driven(calls: u64) -> Site {
    answering(
        checkpointing_command(EFFECT, ATTEMPT, &[], succeeded()),
        calls,
    )
}

/// [`answering`], its spawn never run.
fn unspawned(calls: u64) -> Site {
    answering(vec!["must-not-run".into()], calls)
}

/// MB5, CR4, M1 of U8e2's first run: an engine killed after its driver
/// settled and before its fold left the driver's record sealed Settled,
/// beginning at the head taken before it spawned. The engine a fresh
/// process restarts the run with, through its own turn and on the fence
/// that turn's fold armed, appends every accepted call exactly once,
/// spawns and replays nothing, ends the attempt indeterminate as before on
/// its own appends, and releases its reservation.
#[test]
fn a_restart_after_its_drivers_settled_appends_every_call_once_and_releases() {
    let mut site = driven(2);
    let effect = site.in_flight();
    let (held, ledger, head) = (
        site.engine.held.clone().unwrap(),
        fs::read(site.ledger()).unwrap(),
        site.head(),
    );
    site.died_before_its_fold();
    let settled = settled_at("work", Some(head));
    assert_eq!((site.drivers(), site.rows()), (vec![settled], vec![]));
    let from = site.length();
    assert!(site.turn().unwrap().is_none(), "the run drives on");
    let ended = ending_of(&effect, EventType::EffectIndeterminate, "reason", RESTARTED);
    assert_eq!(last(&site.engine), ended);
    let rows = [row(1, "lookup", "succeeded"), row(2, "lookup", "succeeded")];
    assert_eq!(site.rows(), rows);
    use EventType::{EffectCheckpointed, EffectIndeterminate};
    let journaled = [EffectCheckpointed, EffectCheckpointed, EffectIndeterminate];
    assert_eq!(site.after(from), journaled);
    assert_eq!(fs::read(site.ledger()).unwrap(), ledger);
    let lost = Err(Unadmitted::Identity(Unknown::Lost));
    assert_eq!((recheck(&held), site.released()), (lost, true));
}

/// M1 of U8e2's first run, J1 of its second, decision 0029: the restarted
/// engine's fence moves only over the calls it appended itself onto the
/// head its fold took. A peer that appends after its calls takes that head
/// away from the attempt's end; one that appends before them, an event
/// that is none of its calls, refuses the calls themselves, for the fold
/// they were decided on no longer stands. Either way the drift is named,
/// the peer's event stands, the journal still folds, and the reservation
/// stands.
#[test]
fn a_peer_that_moves_the_journal_refuses_the_restarts_end() {
    let peer_checkpoint = |db: PathBuf, run: String, effect: String| {
        move || {
            let checkpoint = json!({"effect_id": effect, "attempt_id": ATTEMPT});
            let attempt = Some(ATTEMPT.to_string());
            let mut store = Store::open(&db).unwrap();
            let event = EventType::EffectCheckpointed;
            store
                .append_next(&run, event, checkpoint, None, attempt)
                .unwrap();
        }
    };
    for after_its_calls in [true, false] {
        let mut site = unspawned(2);
        let effect = site.in_flight();
        site.begin();
        site.settled_at_work();
        let held = site.engine.held.clone().unwrap();
        let (db, run) = (
            site.engine.store.path().to_path_buf(),
            site.engine.run_id.clone(),
        );
        // The peer writes as a store no fold was handed to: its run is left
        // behind its summary's head on purpose.
        brokkr_store::test_support::lags(&mut site.engine.store, &[&run]);
        let peer = peer_checkpoint(db, run, effect.clone());
        let folded = site.length() as u64;
        let refused = if after_its_calls {
            after_the_fold(Box::new(peer));
            site.turn().map(drop)
        } else {
            let _stood = Coordinated::at(&site.home());
            let engine = &mut site.engine;
            engine.held = None;
            let events = engine.store.load(&engine.run_id).unwrap();
            (engine.current_cause, engine.fence) = tip_of(&events);
            peer();
            engine.restarted(&effect, ATTEMPT)
        };
        let (expected, found) = match after_its_calls {
            true => (folded + 2, folded + 3),
            false => (folded, folded + 1),
        };
        let Err(EngineError::JournalMoved {
            expected_seq,
            found_seq,
            ..
        }) = refused
        else {
            panic!("{after_its_calls}: {refused:?}");
        };
        assert_eq!(
            (expected_seq, found_seq),
            (expected, found),
            "{after_its_calls}"
        );
        let events = site.engine.store.load(&site.engine.run_id).unwrap();
        assert_eq!(events.len() as u64, found, "{after_its_calls}");
        folds(&events).unwrap();
        let calls = [row(1, "lookup", "succeeded"), row(2, "lookup", "succeeded")];
        let calls = if after_its_calls { &calls[..] } else { &[] };
        let rows = [calls, &[Value::Null]].concat();
        assert_eq!(
            site.rows(),
            rows,
            "{after_its_calls}: the peer's after the calls, or alone"
        );
        assert_eq!(recheck(&held), Ok(()), "{after_its_calls}");
    }
}

/// CR4: an engine killed mid-fold, after a commit whose acknowledgement
/// was lost, is recovered from the head its driver's record keeps, never
/// one taken at the restart: the committed call is found and the rest
/// appended once. A verified Closed that failed fails the attempt with its
/// exact cause, that session's calls and none alike, and the reservation
/// is released.
#[test]
fn a_restart_finds_a_lost_commit_and_fails_with_the_sessions_exact_cause() {
    let cases = [
        (2, Failure::Protocol, "MCP server protocol is invalid"),
        (
            0,
            Failure::Refused(Refusal::Establishment),
            "MCP server box could not be established",
        ),
    ];
    for (calls, failure, cause) in cases {
        let mut site = Site::folding();
        site.serve(|mut ledger| {
            answered(&mut ledger, calls);
            ledger.fail(failure);
            ledger.close().unwrap();
        });
        let held = site.engine.held.clone().unwrap();
        site.settled_at_work();
        let rows: Vec<Value> = (1..=calls)
            .map(|call| row(call, "lookup", "succeeded"))
            .collect();
        let run = site.engine.run_id.clone();
        committed(&mut site.engine.store, &run, &rows[..rows.len().min(1)]);
        let failed = ending(EventType::EffectFailed, "error", cause);
        assert_eq!(restart(&mut site), failed, "{cause}");
        assert_eq!(site.rows(), rows, "{cause}");
        let lost = Err(Unadmitted::Identity(Unknown::Lost));
        assert_eq!((recheck(&held), site.released()), (lost, true), "{cause}");
    }
}

/// CR4, M2 and L1 of U8e2's first run: a restart names the exact cause its
/// drivers' calls leave. A driver owning two brokers, the first's ledger
/// missing and the second's verified Closed failed, fails with the
/// failure's own cause, not the missing ledger met before it; a ledger
/// that does not verify leaves the attempt indeterminate with its cause
/// beside the restart's, its verified prefix journaled. Either way the
/// reservation is released.
#[test]
fn a_restart_names_the_exact_cause_its_calls_leave() {
    let mut two = Site::folding();
    let digest = two.second();
    let wiki = written((&two.attempt.owner(), &digest), |mut ledger| {
        answered(&mut ledger, 1);
        ledger.fail(Failure::Protocol);
        ledger.close().unwrap();
    });
    made(&two.attempt.dir().join("plan-1.json.ledger"), &wiki);
    let failed = ending(
        EventType::EffectFailed,
        "error",
        "MCP server protocol is invalid",
    );

    let partial = Site::folding();
    partial.laid(&Fault::Partial.ledger(&partial));
    let reason = format!("{RESTARTED}: broker ledger ends in a partial record");
    let refused = ending(EventType::EffectIndeterminate, "reason", &reason);

    let cases = [
        (two, failed, vec![served("wiki", 1, "lookup", "succeeded")]),
        (
            partial,
            refused,
            vec![
                row(1, "lookup", "succeeded"),
                row(2, "lookup", "interrupted"),
            ],
        ),
    ];
    for (mut site, ended, rows) in cases {
        site.settled_at_work();
        assert_eq!(restart(&mut site), ended);
        assert_eq!((site.rows(), site.released()), (rows, true));
    }
}

/// CR4: calls a lock left unjournaled at a restart are named first, as a
/// running engine names them, with the cause the fold met beside them; the
/// attempt is indeterminate, never failed by that cause.
#[test]
fn a_restart_whose_calls_a_lock_stranded_is_indeterminate_naming_them() {
    let tally = Tally {
        refusal: Some(Unfolded::Missing),
        failed: None,
        stranded: 2,
    };
    let reason = "2 checkpoint(s) were not journaled: a peer still held the journal's write \
                  lock 3 patiences after the seat stopped; broker ledger is missing for this \
                  attempt";
    let ended = (EventType::EffectIndeterminate, "reason", reason.to_string());
    assert_eq!(super::super::restarted(&[tally]), ended);
}

/// MB5, D6, CR4: an engine killed while its driver may still run, its
/// record Settling as the door wrote it, or before any record was written,
/// is restarted indeterminate: nothing is read, so the broker's live
/// half-record is no corruption and its ledger is as it left it, nothing
/// is appended, and the reservation stands, refusing a conflicting run.
#[test]
fn a_restart_while_a_driver_may_run_reads_nothing_and_keeps_the_reservation() {
    for recorded in [true, false] {
        let mut site = Site::folding();
        site.laid(&Fault::Partial.ledger(&site));
        let ledger = fs::read(site.ledger()).unwrap();
        if recorded {
            site.spawning("work", true);
        }
        let held = site.engine.held.clone().unwrap();
        assert_eq!(restart(&mut site), indeterminate(), "{recorded}");
        assert_eq!(site.rows(), Vec::<Value>::new(), "{recorded}");
        assert_eq!(fs::read(site.ledger()).unwrap(), ledger, "{recorded}");
        assert_eq!(recheck(&held), Ok(()), "{recorded}");
        let refused = conflicting(&site.home(), &site.engine).map(drop);
        assert_eq!(refused, held_by(&site.engine.run_id), "{recorded}");
    }
}

/// H1 of U8e2's first run (MB5, D6): a restart adopts the attempt under
/// the admission lock, and from then until its release no driver of it
/// spawns and no record of it is removed unseen. The live engine's own
/// dispatch door ([`spawn_site`]), passed after the adoption under a lease
/// that still stands, refuses as no longer standing and spawns nothing,
/// and the release goes on; a record written after the adoption by any
/// other hand keeps the reservation standing, refusing a conflicting run,
/// and fails the restart with its settlement's refusal.
#[test]
fn a_door_passed_after_the_adoption_refuses_and_the_release_holds_to_the_records() {
    for door in [true, false] {
        let mut site = unspawned(1);
        site.in_flight();
        site.begin();
        site.settled_at_work();
        let live = site.engine.held.clone().unwrap();
        let seen = Rc::new(RefCell::new(None));
        let (peer, saw) = (live.clone(), Rc::clone(&seen));
        let (bundle, work) = (site.engine.bundle.clone(), site.engine.repo.clone());
        let ran = site.home().join("ran-past-the-adoption");
        let argv = vec!["touch".into(), ran.to_string_lossy().into_owned()];
        after_the_fold(Box::new(move || {
            let landed = match door {
                true => {
                    let (spawn, deadline) = (SiteSpawn::inherit(argv), Duration::from_secs(5));
                    let door = Some(Door::of(&peer, "work:check", None));
                    let spawned = spawn_site(&bundle, &spawn, &json!({}), &work, deadline, door);
                    spawned.map(|(process, _)| drop(process))
                }
                false => amend(&peer, soon(), |mut reservation| {
                    let reserved = reservation.attempts.get_mut(ATTEMPT).unwrap();
                    let late = Driver {
                        cleanup: Witnessed::Settling,
                        ..settled_at("work:check", None)
                    };
                    reserved.drivers.push(late);
                    Ok(reservation)
                })
                .map(drop)
                .map_err(|refusal| refusal.to_string()),
            };
            *saw.borrow_mut() = Some(landed);
        }));
        let turned = site.turn().map(drop);
        let landed = seen.borrow_mut().take().unwrap();
        if door {
            let lost = Unadmitted::Identity(Unknown::Lost);
            assert_eq!(
                (landed, ran.exists(), turned.is_ok(), site.released()),
                (Err(lost.to_string()), false, true, true)
            );
            assert_eq!(recheck(&live), Err(lost));
        } else {
            assert_eq!(landed, Ok(()));
            let unproven = Unadmitted::Identity(Unknown::Unproven);
            let Err(EngineError::Unreleased(unreleased)) = turned else {
                panic!("{turned:?}");
            };
            assert_eq!(unreleased, Unreleased::standing(unproven));
            assert_eq!((recheck(&live), site.released()), (Ok(()), false));
            let refused = conflicting(&site.home(), &site.engine).map(drop);
            assert_eq!(refused, held_by(&site.engine.run_id));
        }
    }
}

/// J1 of U8e2's second run (CR4; decision 0029, rulings 1–3): two engines
/// restarted with one attempt in flight both read the journal before
/// either appends. The second appends every call and ends the attempt; the
/// first's append then finds the head moved and writes nothing, and its
/// read again finds the calls and the second's end, so it refuses with the
/// drift named. Each call stands once, the journal folds, and the second's
/// settlement released the reservation.
#[test]
fn two_restarts_that_both_read_before_either_appends_journal_each_call_once() {
    let (site, folded, moved) = moved_after_the_read(|db, run, bundle, work| {
        let store = Store::open(&db).unwrap();
        let mut second = Engine::resume(store, bundle, &run, Some(work)).unwrap();
        let ended = second.drive_once().unwrap();
        assert!(ended.is_none(), "the second drives on");
    });
    let seq = folded as u64;
    assert_eq!(moved, (seq, seq + 3));
    use EventType::{EffectCheckpointed, EffectIndeterminate};
    let journaled = [EffectCheckpointed, EffectCheckpointed, EffectIndeterminate];
    assert_eq!(site.after(folded), journaled);
    let rows = [row(1, "lookup", "succeeded"), row(2, "lookup", "succeeded")];
    assert_eq!((site.rows(), site.released()), (rows.to_vec(), true));
    folds(&site.engine.store.load(&site.engine.run_id).unwrap()).unwrap();
}

/// J1 of U8e2's second run (MB5, CR4; decision 0029): an operator's
/// conclude that lands between a restarted engine's read and its append
/// takes the head away. The append writes nothing, the journal read again
/// holds the run's end, and the restart refuses with the drift named, no
/// call appended after that end: the journal folds as conclude left it,
/// stopped, and the reservation stands.
#[test]
fn a_conclude_between_a_restarts_read_and_its_append_leaves_no_call_after_it() {
    let (site, folded, moved) = moved_after_the_read(|db, run, _, _| {
        let mut store = Store::open(&db).unwrap();
        conclude(&mut store, &run, "operator", "it died").unwrap();
    });
    let seq = folded as u64;
    assert_eq!(moved, (seq, seq + 4));
    use EventType::{EffectIndeterminate, OperatorAccepted, OperatorCommanded, RunStopped};
    let ended = [
        OperatorCommanded,
        OperatorAccepted,
        EffectIndeterminate,
        RunStopped,
    ];
    assert_eq!((site.after(folded), site.rows()), (ended.to_vec(), vec![]));
    let state = folds(&site.engine.store.load(&site.engine.run_id).unwrap()).unwrap();
    let stopped = brokkr_core::fold::Run::Stopped;
    assert_eq!((state.run, site.released()), (stopped, false));
}

/// H1 of U8e2's third run (CR4; decision 0029, rulings 1–3; 0077 ruling
/// 4): a running engine's fold reads the journal and finds its calls
/// unjournaled; a restarted engine, adopting the attempt whose every record
/// is sealed Settled, then appends them and ends the attempt; the running
/// engine's commit finds the head moved and writes nothing, and its read
/// again finds the calls and the attempt's end, so it refuses with the
/// drift named. Each call stands once, nothing follows the end, and the
/// journal folds.
#[test]
fn a_running_fold_and_a_restart_that_both_read_journal_each_call_once() {
    let mut site = unspawned(2);
    let effect = site.in_flight();
    site.begin();
    site.settled_at_work();
    let (db, run, bundle, work) = site.journal();
    let from = site.length();
    after_the_read(Box::new(move || {
        let store = Store::open(&db).unwrap();
        let mut restarted = Engine::resume(store, bundle, &run, Some(work)).unwrap();
        let ended = restarted.drive_once().unwrap();
        assert!(ended.is_none(), "the restart drives on");
    }));
    let _stood = Coordinated::at(&site.home());
    let reported = report(succeeded(), "");
    let (engine, spawn, begun) = (&mut site.engine, &site.spawn, &site.begun);
    let folded = engine.fold_with([&effect, ATTEMPT], spawn, begun, reported, |_| {});
    let rows = [row(1, "lookup", "succeeded"), row(2, "lookup", "succeeded")];
    assert_eq!((site.rows(), site.released()), (rows.to_vec(), true));
    use EventType::{EffectCheckpointed, EffectIndeterminate};
    let journaled = [EffectCheckpointed, EffectCheckpointed, EffectIndeterminate];
    assert_eq!(site.after(from), journaled);
    let Err(EngineError::JournalMoved {
        expected_seq,
        found_seq,
        ..
    }) = folded
    else {
        panic!("{folded:?}");
    };
    let seq = site.begun.0 .0;
    assert_eq!((expected_seq, found_seq), (seq, seq + 3));
    folds(&site.engine.store.load(&site.engine.run_id).unwrap()).unwrap();
}

/// A site whose attempt of two answered calls is in flight, its driver's
/// record sealed Settled, turned by a restarted engine with `peer` put
/// between its read of the journal and its append, a peer whose store
/// leaves the run's summary behind its head on purpose: the site, the
/// journal's length before the turn, and the seqs the turn's refusal
/// names, the head its fold left and the head it found.
fn moved_after_the_read(
    peer: impl FnOnce(PathBuf, String, Bundle, PathBuf) + 'static,
) -> (Site, usize, (u64, u64)) {
    let mut site = unspawned(2);
    site.in_flight();
    site.begin();
    site.settled_at_work();
    let (db, run, bundle, work) = site.journal();
    brokkr_store::test_support::lags(&mut site.engine.store, &[&run]);
    after_the_read(Box::new(move || peer(db, run, bundle, work)));
    let folded = site.length();
    let refused = site.turn().map(drop);
    let Err(EngineError::JournalMoved {
        expected_seq,
        found_seq,
        ..
    }) = refused
    else {
        panic!("{refused:?}");
    };
    (site, folded, (expected_seq, found_seq))
}

/// L1 of U8e2's second run (CR4, fail-closed): an adoption that fails is
/// handed up with its typed refusal, never read as proof that is missing.
/// A reservation that does not read refuses the restart, nothing is
/// journaled, and the attempt stays in flight for a later restart, which
/// recovers it once the record reads again. Proof that is absent is no
/// failure: a run that reserved nothing in the root, or a root that is not
/// protected, where none can have been taken (C1), restarts the attempt
/// indeterminate with nothing read, as before U8e2.
#[test]
fn an_adoption_that_fails_journals_nothing_and_absent_proof_is_indeterminate() {
    let mut site = unspawned(1);
    site.in_flight();
    site.settled_at_work();
    let at = site.reservation_at();
    let kept = fs::read(&at).unwrap();
    fs::write(&at, b"{").unwrap();
    let from = site.length();
    let Err(EngineError::Unreleased(unreleased)) = site.turn() else {
        panic!("the adoption failed");
    };
    let unread = Unadmitted::Identity(Unknown::Participant(at.clone()));
    assert_eq!(
        (unreleased, site.length()),
        (Unreleased::standing(unread), from)
    );
    fs::write(&at, kept).unwrap();
    assert!(site.turn().unwrap().is_none(), "the run drives on");
    let rows = vec![row(1, "lookup", "succeeded")];
    assert_eq!((site.rows(), site.released()), (rows, true));

    for rooted in [true, false] {
        let mut site = unspawned(1);
        let effect = site.in_flight();
        site.settled_at_work();
        let held = site.engine.held.take().unwrap();
        let elsewhere = site.home().with_file_name("elsewhere");
        let _stood = match rooted {
            true => Coordinated::at(&elsewhere),
            false => Coordinated::unrooted(&elsewhere),
        };
        assert!(site.engine.drive_once().unwrap().is_none(), "{rooted}");
        let ended = ending_of(&effect, EventType::EffectIndeterminate, "reason", RESTARTED);
        assert_eq!(
            (last(&site.engine), site.rows()),
            (ended, vec![]),
            "{rooted}"
        );
        assert_eq!(
            (recheck(&held), site.released()),
            (Ok(()), false),
            "{rooted}"
        );
    }
}

/// MB5: a settled earlier driver never stands for a later one. A sequence
/// whose first step settled through the door and the barrier, its later
/// step's record Settling as its door wrote it, and a panel whose engine
/// died after sealing one member's record and before the other's, each
/// restart indeterminate, nothing more appended and the reservation
/// refusing a conflicting run.
#[test]
fn a_later_driver_still_settling_keeps_the_attempt_unrecovered() {
    let mut sequence = driven(1);
    let (spawn, deadline) = (sequence.spawn.clone(), Duration::from_secs(10));
    let input = serving_input(&spawn);
    let engine = &mut sequence.engine;
    let step = engine.run_driver(EFFECT, ATTEMPT, "work", &spawn, input, deadline, None, None);
    assert!(matches!(step, Ok(DriverRun::Ran(_))));
    sequence.spawning("work:check", false);

    let mut panel = unspawned(1);
    panel.spawning("work", true);
    panel.spawning("work:wiki", false);
    ended(&mut panel.engine.held, "work", &Cleanup::Settled);

    let folded = [vec![row(1, "lookup", "succeeded")], vec![]];
    for (mut site, rows) in [sequence, panel].into_iter().zip(folded) {
        let settling = |driver: &Driver| driver.cleanup;
        let records: Vec<Witnessed> = site.drivers().iter().map(settling).collect();
        assert_eq!(records, [Witnessed::Settled, Witnessed::Settling]);
        let held = site.engine.held.clone().unwrap();
        assert_eq!(restart(&mut site), indeterminate());
        assert_eq!(site.rows(), rows);
        assert_eq!(recheck(&held), Ok(()));
        let refused = conflicting(&site.home(), &site.engine).map(drop);
        assert_eq!(refused, held_by(&site.engine.run_id));
    }
}

/// L2 of U8e2's first run (MB5, CR4): a real panel's every member writes
/// its own record at its door, keeping the head the panel took before any
/// member spawned, none for a member whose site serves no broker, and has
/// it sealed by the panel's own settlement before any member is read. An
/// engine killed before the members' fold is recovered from those
/// records: each member's calls through the plans its own site owns, once.
#[test]
fn a_real_panels_members_are_recorded_and_recovered_from_their_heads() {
    let argv = checkpointing_command(EFFECT, ATTEMPT, &[], succeeded());
    let site = Site::new(false, argv.clone());
    let docs = site.owned("notes", "work:docs", |mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let member = |name: &str, spawn: SiteSpawn| MemberRun {
        name: name.into(),
        driver_seat: format!("work:{name}"),
        boundary: None,
        offer: None,
        context: None,
        input: serving_input(&spawn),
        spawn,
    };
    let runs = [
        member("docs", brokered(vec![docs], argv.clone())),
        member("bare", brokered(Vec::new(), argv)),
    ];
    let mut site = site;
    let head = site.head();
    site.dying();
    let died = site
        .engine
        .run_panel(EFFECT, ATTEMPT, &runs, Duration::from_secs(10), "");
    assert!(forced(&died), "the members' fold was stopped: {died:?}");
    site.healed();
    let mut records = site.drivers();
    records.sort_by(|one, other| one.site.cmp(&other.site));
    let expected = [
        settled_at("work:bare", None),
        settled_at("work:docs", Some(head)),
    ];
    assert_eq!((records, site.rows()), (expected.to_vec(), vec![]));
    assert_eq!(restart(&mut site), indeterminate());
    assert_eq!(site.rows(), [owned_row("work:docs", "notes", 1)]);
    assert!(site.released());
}

/// MB5, CR4: a sequence's later step is folded from its own record's head
/// through the plans its own site owns: the earlier step's calls,
/// journaled before the later step began, are found from the earlier
/// step's head and never appended again from the later one's.
#[test]
fn a_sequences_later_step_is_folded_through_its_own_plans_from_its_own_head() {
    let argv = checkpointing_command(EFFECT, ATTEMPT, &[], succeeded());
    let mut site = answering(argv.clone(), 1);
    let first = site.head();
    let spawn = site.spawn.clone();
    let step = concluded_step(&mut site, "work", spawn);
    assert!(matches!(step, Ok(DriverRun::Ran(_))));
    let check = site.owned("notes", "work:check", |mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let second = site.head();
    site.dying();
    let died = concluded_step(&mut site, "work:check", brokered(vec![check], argv));
    assert!(forced(&died), "the later step's fold was stopped");
    site.healed();
    let records = [
        settled_at("work", Some(first)),
        settled_at("work:check", Some(second)),
    ];
    assert_eq!(site.drivers(), records);
    assert_eq!(restart(&mut site), indeterminate());
    let rows = [
        row(1, "lookup", "succeeded"),
        owned_row("work:check", "notes", 1),
    ];
    assert_eq!((site.rows(), site.released()), (rows.to_vec(), true));
}

/// The step of `site`'s attempt at the site labelled `label`, run by
/// `spawn` through the door and the barrier.
fn concluded_step(
    site: &mut Site,
    label: &str,
    spawn: SiteSpawn,
) -> Result<DriverRun, EngineError> {
    let (input, deadline) = (serving_input(&spawn), Duration::from_secs(10));
    let engine = &mut site.engine;
    engine.run_driver(EFFECT, ATTEMPT, label, &spawn, input, deadline, None, None)
}

/// MB5: operator conclude keeps its own judgment of an attempt in flight
/// whose every driver settled: it closes it indeterminate, folds and
/// replays nothing, and its reservation stands.
#[test]
fn operator_conclude_folds_nothing_of_a_recoverable_attempt() {
    let mut site = unspawned(1);
    site.in_flight();
    site.settled_at_work();
    let held = site.engine.held.take().unwrap();
    let run = site.engine.run_id.clone();
    let from = site.length();
    let state = conclude(&mut site.engine.store, &run, "operator", "it died").unwrap();
    assert_eq!(state.run, brokkr_core::fold::Run::Stopped);
    use EventType::{EffectIndeterminate, OperatorAccepted, OperatorCommanded, RunStopped};
    let concluded = [
        OperatorCommanded,
        OperatorAccepted,
        EffectIndeterminate,
        RunStopped,
    ];
    assert_eq!(
        (site.after(from), site.rows()),
        (concluded.to_vec(), vec![])
    );
    assert_eq!(recheck(&held), Ok(()));
}
