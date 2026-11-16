//! The dispatcher's pass (decision 0110 ruling 1): one pass under the
//! journal's lock reconciles, sweeps, reads the bound, judges and starts
//! each admitted entry in the queue's order, and reports what it did.

use std::cell::RefCell;
use std::ffi::OsString;
use std::path::Path;
use std::rc::Rc;

use brokkr_store::{EntryId, EntryState, Store};
use serde_json::json;

use super::*;
use crate::admission::capacity::tests::{ceilings, Fixture};
use crate::dispatch::{take_dispatch, Dispatcher};

pub(crate) mod support;

use support::{claims, commands, host, quiet, run_pass, runs, spawned_one, stands_ok};

/// A spawn seam's recording answer.
fn recording(
    seen: &Rc<RefCell<Vec<crate::dispatch::Spawn>>>,
    pid: u32,
) -> impl FnMut(
    &crate::dispatch::Spawn,
) -> Result<crate::dispatch::Spawned, crate::dispatch::SpawnFailed>
       + '_ {
    let seen = Rc::clone(seen);
    move |spawn: &crate::dispatch::Spawn| {
        seen.borrow_mut().push(spawn.clone());
        Ok(crate::dispatch::Spawned { pid })
    }
}

/// Run one pass with a seam that always stands and a spawn that records
/// every run it is handed.
fn one_pass(
    store: &mut Store,
    dispatcher: &Dispatcher,
    file: &Path,
) -> (Passed, Vec<crate::dispatch::Spawn>) {
    one_pass_pid(store, dispatcher, file, 7)
}

/// [`one_pass`], with the pid the spawn seam answers.
fn one_pass_pid(
    store: &mut Store,
    dispatcher: &Dispatcher,
    file: &Path,
    pid: u32,
) -> (Passed, Vec<crate::dispatch::Spawn>) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let seams = Seams {
        stands: stands_ok,
        spawn: recording(&seen, pid),
    };
    let passed = run_pass(store, dispatcher, file, seams).unwrap();
    let seen = seen.borrow().clone();
    (passed, seen)
}

/// Run `script` on a connection to `store`'s journal.
fn sql(store: &Store, script: &str) {
    rusqlite::Connection::open(store.path())
        .unwrap()
        .execute_batch(script)
        .unwrap();
}

/// The state `entry` stands in.
fn state(store: &Store, entry: EntryId) -> EntryState {
    store.queue_entry(entry).unwrap().state
}

/// A fixture whose one entry is queued and whose store's patience is short
/// enough for a held lock to contend within the test.
fn setup_contended() -> (Fixture, std::path::PathBuf, Dispatcher, EntryId) {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    fixture
        .store
        .set_patience(std::time::Duration::from_millis(20))
        .unwrap();
    (fixture, file, dispatcher, entries[0])
}

/// Assert a pass ended on a store refusal and claimed nothing for `entry`.
fn assert_store_refused(store: &Store, entry: EntryId, refused: Ending) {
    assert!(matches!(refused, Ending::Store(StoreError::Sqlite(_))));
    assert!(claims(store).is_empty());
    assert_eq!(state(store, entry), EntryState::Queued);
}

/// A claimed entry whose run exists and is linked with no `spawn` row: what
/// a pass reconciles.
fn linked_claimed(root: &Path) -> (crate::launch::tests::claimed::Claimed, String, Store) {
    use crate::launch::start;
    use crate::launch::tests::claimed::claimed;
    let crate::launch::tests::claimed::Claimed {
        _dispatcher,
        generation,
        entry,
        claim,
        request,
        run,
    } = claimed(root, None);
    let engine = start(request.clone(), run, &mut |_| {}).unwrap();
    let run = engine.run_id.clone();
    drop(engine);
    let mut store = Store::open(&root.join("forge.db")).unwrap();
    store
        .queue_link(claim, generation, &run, "started")
        .unwrap();
    let claimed = crate::launch::tests::claimed::Claimed {
        _dispatcher,
        generation,
        entry,
        claim,
        request,
        run: crate::launch::NewRun::default(),
    };
    (claimed, run, store)
}

/// A stand that stands once and ends on every call after.
fn stands_once() -> impl FnMut() -> Result<(), Ending> {
    let mut calls = 0;
    move || {
        calls += 1;
        match calls {
            1 => Ok(()),
            _ => Err(Ending::Stood),
        }
    }
}

/// A stand that stands once, then records a newer holder row at `path`.
fn fences_on_second(path: std::path::PathBuf) -> impl FnMut() -> Result<(), Ending> {
    let mut calls = 0;
    move || {
        calls += 1;
        if calls == 2 {
            sql(
                &Store::open(&path).unwrap(),
                "INSERT INTO dispatch_holders (generation, host, pid, started_at) \
                 VALUES (99, NULL, 1, 'then')",
            );
        }
        Ok(())
    }
}

/// A fixture with `entries` queued, the host it declared, its dispatcher
/// taken, and each entry id.
fn setup(entries: usize) -> (Fixture, std::path::PathBuf, Dispatcher, Vec<EntryId>) {
    let mut fixture = Fixture::new();
    let file = fixture.declare(&ceilings(1, 1, json!({})));
    let ids = (0..entries)
        .map(|_| fixture.queue_bundle("bundle"))
        .collect();
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    (fixture, file, dispatcher, ids)
}

#[test]
fn one_pass_starts_one_admitted_entry_and_a_second_starts_nothing() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];

    let (passed, seen) = one_pass(&mut fixture.store, &dispatcher, &file);
    assert_eq!(seen.len(), 1, "one spawn seam call");
    assert_eq!(passed.started.len(), 1);
    let started = &passed.started[0];
    assert_eq!(started.entry, entry);
    assert_eq!(started.spawned.as_ref().unwrap().pid, 7);
    assert_eq!(
        commands(&fixture.store)
            .into_iter()
            .map(|(_, command, operator, _)| (command, operator))
            .collect::<Vec<_>>(),
        vec![
            ("add".into(), "vy".into()),
            ("claim".into(), "dispatch".into()),
            ("link".into(), "dispatch".into()),
            ("spawn".into(), "dispatch".into()),
        ]
    );
    assert_eq!(runs(&fixture.store).len(), 1);
    assert_eq!(seen[0].run, started.run);
    assert_eq!(seen[0].entry, entry);
    assert_eq!(seen[0].claim, started.claim);

    let (again, seen) = one_pass(&mut fixture.store, &dispatcher, &file);
    assert!(
        again.started.is_empty(),
        "a started entry is not started again"
    );
    assert!(seen.is_empty(), "no second spawn");
    assert_eq!(runs(&fixture.store).len(), 1);
}

#[test]
fn the_pass_runs_reconcile_sweep_bound_judge_and_start_in_that_order() {
    let (mut fixture, file, dispatcher, _) = setup(1);
    let steps = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&steps);
    let seams = quiet();
    let passed = pass_with(
        &mut fixture.store,
        &dispatcher,
        &host(&file),
        OsString::new(),
        seams,
        |step| record.borrow_mut().push(step),
    )
    .unwrap();
    assert_eq!(passed.started.len(), 1);
    assert_eq!(
        steps.borrow().as_slice(),
        &[
            Stage::Reconcile,
            Stage::Sweep,
            Stage::Bound,
            Stage::Judge,
            Stage::Start
        ]
    );
}

#[test]
fn an_unreadable_bound_claims_nothing_and_reports_it() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    sql(
        &fixture.store,
        "PRAGMA ignore_check_constraints = ON;
         INSERT INTO dispatch_settings (bound, operator, reason, recorded_at)
         VALUES ('not a bound', 'vy', 'test', 'then');",
    );
    let (passed, seen) = one_pass(&mut fixture.store, &dispatcher, &file);
    assert!(passed.bound.is_none());
    assert!(passed.bound_refused.is_some());
    assert!(passed.started.is_empty());
    assert!(seen.is_empty());
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
    assert_eq!(claims(&fixture.store), vec![]);
}

#[test]
fn two_admitted_entries_under_one_provider_ceiling_start_only_the_first() {
    let (mut fixture, file, dispatcher, entries) = setup(2);
    let first = entries[0];
    let second = entries[1];
    let (passed, seen) = one_pass(&mut fixture.store, &dispatcher, &file);
    assert_eq!(passed.started.len(), 1);
    assert_eq!(passed.started[0].entry, first);
    assert_eq!(
        passed.claims_refused.len(),
        1,
        "{:?}",
        passed.claims_refused
    );
    assert_eq!(passed.claims_refused[0].entry, second);
    assert!(matches!(
        passed.claims_refused[0].reason,
        Reason::Capacity(_)
    ));
    assert_eq!(seen.len(), 1);
    assert_eq!(runs(&fixture.store).len(), 1);
    assert_eq!(state(&fixture.store, second), EntryState::Queued);
}

#[test]
fn reconciliation_spawns_a_linked_run_with_no_spawn_row_once() {
    use crate::dispatch::reconcile::Settled;

    let dir = tempfile::tempdir().unwrap();
    let (claimed, run, mut store) = linked_claimed(dir.path());
    let (passed, seen) = one_pass_pid(
        &mut store,
        &claimed._dispatcher,
        &dir.path().join("absent-host.json"),
        9,
    );
    assert_eq!(
        passed.reconciled,
        vec![Settled::ToSpawn(claimed.claim, run.clone())]
    );
    assert_eq!(passed.spawned.len(), 1);
    assert_eq!(passed.spawned[0].run, run);
    assert_eq!(passed.spawned[0].spawned.as_ref().unwrap().pid, 9);
    assert_eq!(seen.len(), 1);
    assert_eq!(
        commands(&store)
            .into_iter()
            .filter(|(_, command, ..)| command == "spawn")
            .count(),
        1
    );
}

#[test]
fn a_stand_that_ends_before_a_claim_claims_nothing() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    let seams = Seams {
        stands: || Err(Ending::Stood),
        spawn: spawned_one,
    };
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused, Ending::Stood));
    assert_eq!(claims(&fixture.store), vec![]);
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}

#[test]
fn a_stand_that_ends_before_a_spawn_row_leaves_the_entry_for_reconciliation() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    let seams = Seams {
        stands: stands_once(),
        spawn: spawned_one,
    };
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused, Ending::Stood));
    let read = fixture.store.queue_entry(entry).unwrap();
    assert!(matches!(read.state, EntryState::Claimed { .. }));
    assert_eq!(
        commands(&fixture.store)
            .into_iter()
            .filter(|(_, command, ..)| command == "spawn")
            .count(),
        0,
        "the spawn row waits for the next pass"
    );
    drop(dispatcher);

    let next = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let (passed, seen) = one_pass_pid(&mut fixture.store, &next, &file, 2);
    assert_eq!(passed.spawned.len(), 1);
    assert_eq!(seen.len(), 1);
    assert_eq!(
        commands(&fixture.store)
            .into_iter()
            .filter(|(_, command, ..)| command == "spawn")
            .count(),
        1
    );
}

#[test]
fn a_newer_holder_row_fences_the_pass_out_with_nothing_written_after() {
    let (mut fixture, file, dispatcher, _) = setup(1);
    let generation = dispatcher.generation;
    let path = fixture.store.path().to_path_buf();
    let seams = Seams {
        stands: fences_on_second(path),
        spawn: spawned_one,
    };
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused, Ending::Fenced(g) if g == generation));
    let words: Vec<String> = commands(&fixture.store)
        .into_iter()
        .map(|(_, command, ..)| command)
        .collect();
    assert_eq!(words, vec!["add", "claim", "link"]);
}

#[test]
fn a_planted_store_error_ends_the_pass_and_reconciliation_reclaims_the_entry() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    sql(
        &fixture.store,
        "CREATE TRIGGER planted BEFORE INSERT ON runs \
         BEGIN SELECT RAISE(ABORT, 'planted store failure'); END;",
    );
    let seams = quiet();
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused, Ending::Store(StoreError::Sqlite(_))));
    assert_eq!(state(&fixture.store, entry), EntryState::Launching);
    drop(dispatcher);
    sql(&fixture.store, "DROP TRIGGER planted");
    let next = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let settled = crate::dispatch::reconcile(&mut fixture.store, &next)
        .unwrap()
        .unwrap();
    assert_eq!(settled.len(), 1);
    assert!(matches!(
        settled[0],
        crate::dispatch::Settled::Reclaimed(claim) if claim.parts().0 == entry
    ));
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}

#[test]
fn a_bundle_moved_between_the_pass_and_the_start_is_reclaimed_uncounted() {
    let mut fixture = Fixture::new();
    let file = fixture.declare(&ceilings(1, 1, json!({})));
    fixture.queue_bundle("bundle");
    let before = fixture
        .launch_in("bundle")
        .compiled()
        .unwrap()
        .manifest_digest();
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let role = fixture.root.join("bundle/roles/seat.md");
    let mut moved = false;
    let seams = Seams {
        stands: move || {
            if !moved {
                std::fs::write(&role, "# moved\n").unwrap();
                moved = true;
            }
            Ok(())
        },
        spawn: spawned_one,
    };
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    let after = fixture
        .launch_in("bundle")
        .compiled()
        .unwrap()
        .manifest_digest();
    assert_ne!(before, after, "the edit moves the manifest digest");
    assert_eq!(passed.reclaimed.len(), 1);
    assert!(passed.reclaimed[0].refused.is_none(), "uncounted");
    let (_, command, _, reason) = commands(&fixture.store)
        .into_iter()
        .find(|(_, command, ..)| command == "reclaim")
        .expect("a reclaim");
    assert_eq!(command, "reclaim");
    assert!(reason.contains(&before), "{reason}");
    assert!(reason.contains(&after), "{reason}");
}

#[test]
fn a_compile_refusal_under_a_bound_of_one_holds_the_entry_with_its_variant() {
    use std::num::NonZeroU32;
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    fixture
        .store
        .set_dispatch_bound(
            NonZeroU32::MIN,
            brokkr_store::Attribution {
                operator: "vy",
                reason: "bound",
            },
        )
        .unwrap();
    let bundle = fixture.root.join("bundle/bundle.json");
    let mut broken = false;
    let seams = Seams {
        stands: move || {
            if !broken {
                std::fs::write(&bundle, "{not json").unwrap();
                broken = true;
            }
            Ok(())
        },
        spawn: spawned_one,
    };
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    assert_eq!(passed.held.len(), 1, "{:?}", passed.held);
    assert!(matches!(passed.held[0].refusal, StartRefusal::Compile));
    assert_eq!(state(&fixture.store, entry), EntryState::Held);
    let (_, command, _, reason) = commands(&fixture.store)
        .into_iter()
        .find(|(_, command, ..)| command == "hold")
        .expect("a hold");
    assert_eq!(reason, "compile");
    assert_eq!(command, "hold");
    let conn = rusqlite::Connection::open(fixture.store.path()).unwrap();
    let stored: String = conn
        .query_row("SELECT refusal FROM queue_refusals", [], |row| row.get(0))
        .unwrap();
    assert_eq!(stored, "compile");
}

#[test]
fn a_sweep_record_refusal_is_reported_beside_the_removals_it_made() {
    let mut passed = Passed::default();
    record_sweep(
        &mut passed,
        Err(SweepError::Record {
            report: Box::default(),
            cause: "forced record failure".into(),
        }),
    )
    .unwrap();
    assert!(passed.swept.is_none());
    let refused = passed.sweep_refused.expect("a record refusal");
    assert_eq!(refused.cause, "forced record failure");
    assert!(refused.report.removed().is_empty());
}

#[test]
fn a_sweep_store_refusal_contended_is_reported_and_any_other_ends() {
    let mut passed = Passed::default();
    record_sweep(
        &mut passed,
        Err(SweepError::Store(StoreError::Contended {
            operation: "sweep",
            waited_ms: 1,
        })),
    )
    .unwrap();
    assert_eq!(passed.contended[0].stage, Stage::Sweep);

    let mut passed = Passed::default();
    let ended = record_sweep(
        &mut passed,
        Err(SweepError::Store(StoreError::RunNotFound("gone".into()))),
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Store(StoreError::RunNotFound(_))));
}

#[test]
fn a_sweep_dispatcher_refusal_ends_the_pass() {
    let mut passed = Passed::default();
    let ended = record_sweep(
        &mut passed,
        Err(SweepError::Dispatcher(super::super::DispatcherError::Lock(
            Path::new("lock").to_path_buf(),
        ))),
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Dispatcher(_)));
}

#[test]
fn an_unreadable_host_is_reported_and_claims_nothing() {
    let mut fixture = Fixture::new();
    let file = fixture.root.join("host.json");
    std::fs::write(&file, "{not json").unwrap();
    let entry = fixture.queue_bundle("bundle");
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let seams = quiet();
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    assert!(matches!(
        passed.admission_refused,
        Some(crate::admission::AdmissionError::Host(_))
    ));
    assert_eq!(claims(&fixture.store), vec![]);
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}

#[test]
fn a_store_refusal_that_is_not_contention_ends_the_pass() {
    let mut passed = Passed::default();
    let ended = store_refused(
        &mut passed,
        StoreError::RunNotFound("gone".into()),
        Stage::Sweep,
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Store(StoreError::RunNotFound(_))));

    let mut passed = Passed::default();
    let ended = store_refused(
        &mut passed,
        StoreError::Queue(brokkr_store::QueueRefusal::Superseded(Generation(4))),
        Stage::Judge,
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Fenced(Generation(4))));

    let mut passed = Passed::default();
    let ended = store_refused(
        &mut passed,
        StoreError::Queue(brokkr_store::QueueRefusal::SchemaMismatch { found: 9 }),
        Stage::Reconcile,
    )
    .unwrap_err();
    assert!(matches!(
        ended,
        Ending::QueueStorage(brokkr_store::QueueRefusal::SchemaMismatch { found: 9 })
    ));

    let mut passed = Passed::default();
    let stop = store_refused(
        &mut passed,
        StoreError::Contended {
            operation: "sweep",
            waited_ms: 1,
        },
        Stage::Sweep,
    )
    .unwrap();
    assert!(stop);
    assert_eq!(passed.contended.len(), 1);
    assert_eq!(passed.contended[0].stage, Stage::Sweep);
}

#[test]
fn a_pass_over_a_store_that_is_not_its_journal_ends_by_dispatcher() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let mut first_store = Store::open(&first.path().join("forge.db")).unwrap();
    let mut second_store = Store::open(&second.path().join("forge.db")).unwrap();
    let dispatcher = take_dispatch(&mut second_store).unwrap().unwrap();
    let seams = quiet();
    let refused = run_pass(
        &mut first_store,
        &dispatcher,
        &first.path().join("absent-host.json"),
        seams,
    )
    .unwrap_err();
    assert!(matches!(refused, Ending::Dispatcher(_)));
}

#[test]
fn a_pass_over_queue_storage_this_binary_does_not_write_ends_naming_it() {
    let mut fixture = Fixture::new();
    fixture.queue_bundle("bundle");
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let conn = rusqlite::Connection::open(fixture.store.path()).unwrap();
    conn.execute(
        "UPDATE meta SET value = '999' WHERE key = 'queue_schema'",
        [],
    )
    .unwrap();
    let seams = quiet();
    let refused = run_pass(
        &mut fixture.store,
        &dispatcher,
        &fixture.root.join("absent-host.json"),
        seams,
    )
    .unwrap_err();
    assert!(matches!(
        refused,
        Ending::QueueStorage(brokkr_store::QueueRefusal::SchemaMismatch { found: 999 })
    ));
}

#[test]
fn a_spawn_failure_is_reported_with_the_run_and_never_retried() {
    let (mut fixture, file, dispatcher, _) = setup(1);
    let seams = Seams {
        stands: || Ok(()),
        spawn: |_: &crate::dispatch::Spawn| {
            Err(crate::dispatch::SpawnFailed::Refused(
                "no descriptor".into(),
            ))
        },
    };
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    assert_eq!(passed.started.len(), 1);
    let started = &passed.started[0];
    assert!(matches!(
        started.spawned,
        Err(crate::dispatch::SpawnFailed::Refused(ref why)) if why == "no descriptor"
    ));
    assert!(!started.run.is_empty());
    assert_eq!(
        started.spawned.as_ref().unwrap_err().resume(&started.run),
        format!("brokkr resume {}", started.run)
    );
    let second = run_pass(&mut fixture.store, &dispatcher, &file, quiet()).unwrap();
    assert!(second.started.is_empty());
    assert!(second.spawned.is_empty());
}

#[test]
fn every_start_outcome_is_reported_and_only_two_stop_the_claims() {
    use crate::dispatch::{ClaimRefused, Contended, Held, Reclaimed};
    use crate::launch::tests::claimed::claimed;

    let dir = tempfile::tempdir().unwrap();
    let fixture = claimed(dir.path(), None);
    let claim = fixture.claim;
    let mut passed = Passed::default();
    let started = Started {
        entry: EntryId(1),
        claim,
        run: "r".into(),
        spawned: Ok(crate::dispatch::Spawned { pid: 1 }),
    };
    assert!(record_start(
        &mut passed,
        crate::dispatch::start::Outcome::Started(started)
    ));
    assert_eq!(passed.started.len(), 1);

    assert!(record_start(
        &mut passed,
        crate::dispatch::start::Outcome::Reclaimed(Reclaimed {
            claim,
            refused: None,
            count: 0,
        })
    ));
    assert!(record_start(
        &mut passed,
        crate::dispatch::start::Outcome::Held(Held {
            claim,
            refusal: StartRefusal::Compile,
            count: 1,
        })
    ));
    assert!(record_start(
        &mut passed,
        crate::dispatch::start::Outcome::ClaimRefused(ClaimRefused {
            entry: EntryId(1),
            reason: crate::admission::Reason::OperatorHold,
        })
    ));
    assert_eq!(passed.reclaimed.len(), 1);
    assert_eq!(passed.held.len(), 1);
    assert_eq!(passed.claims_refused.len(), 1);

    let stop = record_start(
        &mut passed,
        crate::dispatch::start::Outcome::Contended(Contended {
            entry: Some(EntryId(1)),
            claim: Some(claim),
            stage: Stage::Link,
            error: StoreError::Contended {
                operation: "link",
                waited_ms: 1,
            },
        }),
    );
    assert!(!stop);
    assert_eq!(passed.contended.len(), 1);

    let stop = record_start(
        &mut passed,
        crate::dispatch::start::Outcome::Admission(crate::admission::AdmissionError::Unmeasured(
            EntryId(1),
        )),
    );
    assert!(!stop);
    assert!(passed.admission_refused.is_some());
}

#[test]
fn reconciliation_spawn_stops_on_a_stand_that_ends_and_on_a_store_refusal() {
    use crate::dispatch::reconcile::Settled;
    use crate::launch::tests::claimed::claimed;

    let dir = tempfile::tempdir().unwrap();
    let first = claimed(dir.path(), None);
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let run = "run".to_string();
    let mut passed = Passed {
        reconciled: vec![Settled::ToSpawn(first.claim, run.clone())],
        ..Passed::default()
    };
    let mut seams = Seams {
        stands: || Err(Ending::Stood),
        spawn: spawned_one,
    };
    let refused = spawned_after_reconcile(&mut store, &first._dispatcher, &mut passed, &mut seams)
        .unwrap_err();
    assert!(matches!(refused, Ending::Stood));

    let second_dir = tempfile::tempdir().unwrap();
    let second = claimed(second_dir.path(), None);
    let mut store = Store::open(&second_dir.path().join("forge.db")).unwrap();
    let engine = crate::launch::start(second.request.clone(), second.run, &mut |_| {}).unwrap();
    let run = engine.run_id.clone();
    drop(engine);
    store
        .queue_link(second.claim, second.generation, &run, "started")
        .unwrap();
    let mut passed = Passed {
        reconciled: vec![Settled::ToSpawn(second.claim, run.clone())],
        ..Passed::default()
    };
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER no_spawn BEFORE INSERT ON queue_commands \
         WHEN NEW.command = 'spawn' BEGIN SELECT RAISE(ABORT, 'no spawn'); END;",
    )
    .unwrap();
    let mut seams = Seams {
        stands: stands_ok,
        spawn: spawned_one,
    };
    let refused = spawned_after_reconcile(&mut store, &second._dispatcher, &mut passed, &mut seams)
        .unwrap_err();
    assert!(matches!(refused, Ending::Store(StoreError::Sqlite(_))));

    let mut passed = Passed {
        reconciled: vec![Settled::Reclaimed(second.claim)],
        ..Passed::default()
    };
    let mut seams = Seams {
        stands: stands_ok,
        spawn: spawned_one,
    };
    assert!(
        spawned_after_reconcile(&mut store, &second._dispatcher, &mut passed, &mut seams,).unwrap()
    );
    assert_eq!(passed.reconciled.len(), 1);
}

#[test]
fn a_contended_reconciliation_is_reported_and_claims_nothing() {
    let (mut fixture, file, dispatcher, entry) = setup_contended();
    let holder = lock_journal(fixture.store.path());
    let seams = quiet();
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    holder.execute_batch("ROLLBACK").unwrap();
    assert_eq!(passed.contended.len(), 1);
    assert_eq!(passed.contended[0].stage, Stage::Reconcile);
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
    assert_eq!(claims(&fixture.store), vec![]);
}

#[test]
fn a_store_refusal_at_the_claim_ends_the_pass_and_writes_no_claim() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    sql(
        &fixture.store,
        "CREATE TRIGGER no_claim BEFORE INSERT ON queue_claims \
         BEGIN SELECT RAISE(ABORT, 'no claim'); END;",
    );
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, quiet()).unwrap_err();
    assert_store_refused(&fixture.store, entry, refused);
}

/// A stand that takes a write lock on `path` when the pass first calls it,
/// and the held connection a test releases.
type LockingSeams = Seams<
    Box<dyn FnMut() -> Result<(), Ending>>,
    fn(&crate::dispatch::Spawn) -> Result<crate::dispatch::Spawned, crate::dispatch::SpawnFailed>,
>;

fn lock_seams(
    path: std::path::PathBuf,
) -> (Rc<RefCell<Option<rusqlite::Connection>>>, LockingSeams) {
    let held = Rc::new(RefCell::new(None));
    let stands: Box<dyn FnMut() -> Result<(), Ending>> = {
        let holder = Rc::clone(&held);
        Box::new(move || {
            *holder.borrow_mut() = Some(lock_journal(&path));
            Ok(())
        })
    };
    (
        held,
        Seams {
            stands,
            spawn: spawned_one,
        },
    )
}

/// A write lock on `path`'s journal, held until its connection is dropped.
fn lock_journal(path: &Path) -> rusqlite::Connection {
    let holder = rusqlite::Connection::open(path).unwrap();
    holder
        .busy_timeout(std::time::Duration::from_millis(20))
        .unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    holder
}

#[test]
fn a_contended_claim_is_reported_and_no_further_entry_is_claimed() {
    let (mut fixture, file, dispatcher, entry) = setup_contended();
    let (held, seams) = lock_seams(fixture.store.path().to_owned());
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    let conn = held.borrow_mut().take();
    drop(conn);
    assert_eq!(passed.contended.len(), 1);
    assert_eq!(passed.contended[0].stage, Stage::Claim);
    assert!(claims(&fixture.store).is_empty());
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}

#[test]
fn a_contended_reconciliation_spawn_row_is_reported_and_stops_the_pass() {
    use crate::dispatch::reconcile::Settled;

    let dir = tempfile::tempdir().unwrap();
    let (claimed, _run, mut store) = linked_claimed(dir.path());
    store
        .set_patience(std::time::Duration::from_millis(20))
        .unwrap();
    let (held, seams) = lock_seams(store.path().to_owned());
    let passed = run_pass(
        &mut store,
        &claimed._dispatcher,
        &dir.path().join("absent-host.json"),
        seams,
    )
    .unwrap();
    let conn = held.borrow_mut().take();
    drop(conn);
    assert_eq!(passed.contended.len(), 1);
    assert_eq!(passed.contended[0].stage, Stage::Spawn);
    assert!(matches!(
        passed.reconciled.as_slice(),
        [Settled::ToSpawn(..)]
    ));
    assert_eq!(
        commands(&store)
            .into_iter()
            .filter(|(_, command, ..)| command == "spawn")
            .count(),
        0
    );
}

#[test]
fn a_store_refusal_while_judging_ends_the_pass_and_claims_nothing() {
    use crate::launch::{BundleSource, LaunchRequest, NewRun, QueuedLaunch, RunMap};
    use crate::realms::World;
    use brokkr_store::{Attribution, NewEntry};

    let mut fixture = Fixture::new();
    let map = fixture.root.join("realms.json");
    let realms = json!({
        "schema": "forge.realms/v7",
        "journal": "forge.db",
        "realms": [{"name": "b", "path": ".", "default_branch": "main", "house": "HOUSE.md"}]
    });
    std::fs::write(&map, realms.to_string()).unwrap();
    std::fs::write(fixture.root.join("HOUSE.md"), "First rule.\n").unwrap();
    let request = LaunchRequest {
        workspace: fixture.root.clone(),
        bundle: BundleSource::Dir("bundle".into()),
        shelf: None,
        journal: fixture.root.join("forge.db"),
        repo: None,
        secrets: None,
        host_path: OsString::new(),
    };
    let run = NewRun {
        entry: None,
        feature: "f".into(),
        map: RunMap::Named(World::load(&map).unwrap()),
        dispatch: None,
        claim: None,
    };
    let payload = QueuedLaunch::of(&request, &run).unwrap().encode().unwrap();
    let entry = fixture
        .store
        .queue_add(
            NewEntry {
                payload: &payload,
                priority: 0,
                waits: &[],
            },
            Attribution {
                operator: "vy",
                reason: "queued",
            },
        )
        .unwrap();
    std::fs::write(fixture.root.join("HOUSE.md"), "Second rule.\n").unwrap();
    sql(
        &fixture.store,
        "CREATE TRIGGER no_latch BEFORE INSERT ON queue_latches \
         BEGIN SELECT RAISE(ABORT, 'no latch'); END;",
    );
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let refused = run_pass(
        &mut fixture.store,
        &dispatcher,
        &fixture.root.join("absent-host.json"),
        quiet(),
    )
    .unwrap_err();
    assert_store_refused(&fixture.store, entry, refused);
}

#[test]
fn a_claim_whose_moved_run_does_not_fold_is_reported_and_stops_claims() {
    use brokkr_core::EventType;

    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    let path = fixture.store.path().to_path_buf();
    let seams = Seams {
        stands: move || {
            let mut side = Store::open(&path).unwrap();
            side.create_run("does-not-fold", "f", "b", &json!({}))
                .unwrap();
            side.append_next(
                "does-not-fold",
                EventType::PhaseEntered,
                json!({"phase": "work"}),
                None,
                None,
            )
            .unwrap();
            Ok(())
        },
        spawn: spawned_one,
    };
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    assert!(matches!(
        passed.admission_refused,
        Some(crate::admission::AdmissionError::Fold { .. })
    ));
    assert!(claims(&fixture.store).is_empty());
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}
