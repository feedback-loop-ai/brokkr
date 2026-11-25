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

use support::{
    always_claims, claims, commands, host, quiet, run_pass, runs, spawned_one, stands_ok,
};

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
        claims: || true,
        stands: stands_ok,
        spawn: recording(&seen, pid),
    };
    let passed = run_pass(store, dispatcher, file, seams).unwrap();
    let seen = seen.borrow().clone();
    (passed, seen)
}

/// The private seams with `observe` and the real launch.
fn watching<O: FnMut(Stage)>(observe: O) -> Watch<O> {
    Watch {
        observe,
        launch: crate::launch::start,
    }
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
fn assert_store_refused(store: &Store, entry: EntryId, refused: Box<Ended>) {
    assert!(matches!(
        refused.ending,
        Ending::Store(StoreError::Sqlite(_))
    ));
    assert!(claims(store).is_empty());
    assert_eq!(state(store, entry), EntryState::Queued);
    assert!(refused.passed.started.is_empty());
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

/// Queue one launch under the named map `realms` writes at `root`, and
/// answer its entry.
fn queue_named(store: &mut Store, root: &Path, realms: serde_json::Value) -> EntryId {
    use crate::launch::{BundleSource, LaunchRequest, NewRun, QueuedLaunch, RunMap};
    use crate::realms::World;
    use brokkr_store::{Attribution, NewEntry};

    let map = root.join("realms.json");
    std::fs::write(&map, realms.to_string()).unwrap();
    let request = LaunchRequest {
        workspace: root.to_path_buf(),
        bundle: BundleSource::Dir("bundle".into()),
        shelf: None,
        journal: root.join("forge.db"),
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
    store
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
        .unwrap()
}

/// A fixture whose one entry is queued under a named map whose house text
/// changes before the pass judges it, and the host file the tests read.
fn a_named_map_whose_house_moved() -> (Fixture, EntryId, Dispatcher, std::path::PathBuf) {
    let mut fixture = Fixture::new();
    std::fs::write(fixture.root.join("HOUSE.md"), "First rule.\n").unwrap();
    let entry = queue_named(
        &mut fixture.store,
        &fixture.root,
        json!({
            "schema": "forge.realms/v7",
            "journal": "forge.db",
            "realms": [{"name": "b", "path": ".", "default_branch": "main", "house": "HOUSE.md"}]
        }),
    );
    std::fs::write(fixture.root.join("HOUSE.md"), "Second rule.\n").unwrap();
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let host = fixture.root.join("absent-host.json");
    (fixture, entry, dispatcher, host)
}

/// Assert `left` names the one run `run`, under the one claim the journal
/// holds, by its entry and seq.
fn assert_left(store: &Store, left: &[Left], run: &str) {
    let (seq, entry, ..) = claims(store)[0];
    let [left] = left else {
        panic!("{left:?}");
    };
    assert_eq!(
        (left.claim.parts(), left.run.as_str()),
        ((EntryId(entry), seq), run)
    );
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
    // The run carries the claim it was started under (decision 0095
    // ruling 5), which reconciliation reads.
    let (entry_id, seq) = started.claim.parts();
    assert_eq!(
        fixture.store.manifest(&started.run).unwrap()["queue_entry"],
        json!({"entry": entry_id.0, "claim": seq})
    );

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
        watching(|step| record.borrow_mut().push(step)),
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
    assert!(matches!(
        passed.bound,
        Some(BoundOutcome::Unreadable(StoreError::Sqlite(_)))
    ));
    assert!(passed.started.is_empty());
    assert!(seen.is_empty());
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
    assert_eq!(claims(&fixture.store), vec![]);
}

#[test]
fn a_bound_read_refusal_is_split_by_its_class() {
    let mut passed = Passed::default();
    assert!(bound_refused(
        &mut passed,
        StoreError::Contended {
            operation: "dispatch_bound",
            waited_ms: 1,
        },
    )
    .unwrap()
    .is_none());
    assert_eq!(passed.contended[0].subject, Subject::Pass);
    assert_eq!(passed.contended[0].stage, Stage::Bound);

    let mut passed = Passed::default();
    let ended = bound_refused(
        &mut passed,
        StoreError::Queue(brokkr_store::QueueRefusal::Superseded(Generation(4))),
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Fenced(Generation(4))));

    let mut passed = Passed::default();
    let ended = bound_refused(
        &mut passed,
        StoreError::Queue(brokkr_store::QueueRefusal::SchemaMismatch { found: 9 }),
    )
    .unwrap_err();
    assert!(matches!(
        ended,
        Ending::QueueStorage(brokkr_store::QueueRefusal::SchemaMismatch { found: 9 })
    ));

    let mut passed = Passed::default();
    assert!(bound_refused(
        &mut passed,
        StoreError::Json(serde_json::from_str::<serde_json::Value>("{").unwrap_err()),
    )
    .unwrap()
    .is_none());
    assert!(matches!(
        passed.bound,
        Some(BoundOutcome::Unreadable(StoreError::Json(_)))
    ));
}

#[test]
fn two_admitted_entries_under_one_provider_ceiling_start_only_the_first() {
    let (mut fixture, file, dispatcher, entries) = setup(3);
    let first = entries[0];
    let second = entries[1];
    let by = brokkr_store::Attribution {
        operator: "vy",
        reason: "held",
    };
    let hold = brokkr_store::QueueCommand::Hold;
    fixture.store.queue_command(entries[2], hold, by).unwrap();
    let (passed, seen) = one_pass(&mut fixture.store, &dispatcher, &file);
    assert_eq!((passed.waiting, passed.held_entries), (0, 1));
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

    // The next pass judges the second entry waiting on the full ceiling.
    let (again, _) = one_pass(&mut fixture.store, &dispatcher, &file);
    assert_eq!((again.waiting, again.held_entries), (1, 1));
}

/// "Queue it before the rest" is a priority (decision 0068 ruling 2,
/// #1191): A queued at the default priority, then B at 10, with room for
/// one run. The pass starts B, and A waits on the room B took.
#[test]
fn a_higher_priority_queued_later_starts_first_and_the_earlier_waits_on_capacity() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let a = entries[0];
    let payload = fixture.launch_in("bundle").encode().unwrap();
    let b = brokkr_store::NewEntry {
        payload: &payload,
        priority: 10,
        waits: &[],
    };
    let b = fixture
        .store
        .queue_add(b, crate::admission::capacity::tests::BY);
    let b = b.unwrap();
    let full = Reason::Capacity(crate::admission::capacity::Capacity::ProviderFull {
        provider: "alpha".into(),
        running: 1,
        ceiling: 1,
    });

    let (passed, seen) = one_pass(&mut fixture.store, &dispatcher, &file);
    let started: Vec<EntryId> = passed.started.iter().map(|s| s.entry).collect();
    let refused: Vec<(EntryId, &Reason)> = (passed.claims_refused.iter())
        .map(|refused| (refused.entry, &refused.reason))
        .collect();
    assert_eq!(
        (started, refused, seen.len()),
        (vec![b], vec![(a, &full)], 1)
    );
    assert_eq!(state(&fixture.store, a), EntryState::Queued);

    let judged = admission::pass_within(&fixture.store, &host(&file))
        .unwrap()
        .0;
    let verdicts: Vec<(EntryId, Option<admission::Verdict>)> = (judged.into_iter())
        .map(|judged| (judged.entry.id, judged.verdict))
        .collect();
    let waits = admission::Verdict {
        reasons: vec![full],
    };
    assert_eq!(verdicts, vec![(a, Some(waits)), (b, None)]);
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
        claims: || true,
        stands: || Err(Ending::Stood),
        spawn: spawned_one,
    };
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused.ending, Ending::Stood));
    assert_eq!(claims(&fixture.store), vec![]);
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}

/// Decision 0110 ruling 5: the stop is read inside the claim's own
/// transaction, once it holds the journal's write lock, so a stop asked
/// while the claim waited on another writer still claims nothing.
#[test]
fn a_stop_is_read_with_the_claims_write_lock_held_and_claims_nothing() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let path = fixture.store.path().to_path_buf();
    let mut asked = Vec::new();
    let seams = Seams {
        claims: || {
            let other = rusqlite::Connection::open(&path).unwrap();
            other.busy_timeout(std::time::Duration::ZERO).unwrap();
            let begun = other.execute_batch("BEGIN IMMEDIATE");
            asked.push(begun.err().and_then(|error| error.sqlite_error_code()));
            false
        },
        stands: stands_ok,
        spawn: spawned_one,
    };
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    assert_eq!(asked, [Some(rusqlite::ErrorCode::DatabaseBusy)]);
    assert!(passed.started.is_empty());
    assert_eq!(claims(&fixture.store), vec![]);
    assert_eq!(state(&fixture.store, entries[0]), EntryState::Queued);
}

#[test]
fn a_stand_that_ends_before_a_spawn_row_leaves_the_entry_for_reconciliation() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    let seams = Seams {
        claims: || true,
        stands: stands_once(),
        spawn: spawned_one,
    };
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused.ending, Ending::Stood));
    let read = fixture.store.queue_entry(entry).unwrap();
    let EntryState::Claimed { run, .. } = read.state else {
        panic!("{:?}", read.state);
    };
    // The ending's report names the run and its claim (6a F2).
    assert_left(&fixture.store, &refused.passed.left, &run);
    assert!(refused.passed.started.is_empty());
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
        claims: || true,
        stands: fences_on_second(path),
        spawn: spawned_one,
    };
    let refused = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(refused.ending, Ending::Fenced(g) if g == generation));
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
    assert!(matches!(
        refused.ending,
        Ending::Store(StoreError::Sqlite(_))
    ));
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
        claims: || true,
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
        claims: || true,
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
    // The class word, then the start's own words (#430 c-6928014415, F3).
    assert!(reason.starts_with("compile: "), "{reason}");
    assert!(reason.len() > "compile: ".len(), "{reason}");
    assert_eq!(passed.held[0].reason, reason);
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
    let Some(SweepOutcome::Refused(refused)) = passed.sweep else {
        panic!("a record refusal");
    };
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
    assert_eq!(passed.contended[0].subject, Subject::Pass);

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
        Subject::Pass,
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Store(StoreError::RunNotFound(_))));

    let mut passed = Passed::default();
    let ended = store_refused(
        &mut passed,
        StoreError::Queue(brokkr_store::QueueRefusal::Superseded(Generation(4))),
        Stage::Judge,
        Subject::Pass,
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Fenced(Generation(4))));

    let mut passed = Passed::default();
    let ended = store_refused(
        &mut passed,
        StoreError::Queue(brokkr_store::QueueRefusal::SchemaMismatch { found: 9 }),
        Stage::Reconcile,
        Subject::Pass,
    )
    .unwrap_err();
    assert!(matches!(
        ended,
        Ending::QueueStorage(brokkr_store::QueueRefusal::SchemaMismatch { found: 9 })
    ));

    let mut passed = Passed::default();
    store_refused(
        &mut passed,
        StoreError::Contended {
            operation: "sweep",
            waited_ms: 1,
        },
        Stage::Sweep,
        Subject::Pass,
    )
    .unwrap();
    assert_eq!(passed.contended.len(), 1);
    assert_eq!(passed.contended[0].stage, Stage::Sweep);
    assert_eq!(passed.contended[0].subject, Subject::Pass);

    let mut passed = Passed::default();
    store_refused(
        &mut passed,
        StoreError::Contended {
            operation: "judge",
            waited_ms: 1,
        },
        Stage::Judge,
        Subject::Pass,
    )
    .unwrap();
    assert_eq!(passed.contended[0].stage, Stage::Judge);
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
    assert!(matches!(refused.ending, Ending::Dispatcher(_)));
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
        refused.ending,
        Ending::QueueStorage(brokkr_store::QueueRefusal::SchemaMismatch { found: 999 })
    ));
}

#[test]
fn a_spawn_failure_is_reported_with_the_run_and_never_retried() {
    let (mut fixture, file, dispatcher, _) = setup(1);
    let seams = Seams {
        claims: || true,
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
    // The report names the run the journal holds beside the seam's
    // refusal; 6b's view words the command that resumes it (decision 0071
    // ruling 7).
    assert_eq!(runs(&fixture.store), vec![started.run.clone()]);
    let second = run_pass(&mut fixture.store, &dispatcher, &file, quiet()).unwrap();
    assert!(second.started.is_empty());
    assert!(second.spawned.is_empty());
}

#[test]
fn an_ending_after_a_spawn_failure_keeps_the_report_of_what_the_pass_did() {
    let (mut fixture, file, dispatcher, entries) = setup(2);
    let first = entries[0];
    let mut stands = {
        let mut calls = 0;
        move || {
            calls += 1;
            match calls {
                3 => Err(Ending::Stood),
                _ => Ok(()),
            }
        }
    };
    // The stand ends before the second entry's claim, after the first
    // entry's failed spawn has been reported.
    let seams = Seams {
        claims: || true,
        stands: move || stands(),
        spawn: |_: &crate::dispatch::Spawn| {
            Err(crate::dispatch::SpawnFailed::Refused(
                "no descriptor".into(),
            ))
        },
    };
    let ended = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap_err();
    assert!(matches!(ended.ending, Ending::Stood));
    assert_eq!(ended.passed.started.len(), 1);
    let started = &ended.passed.started[0];
    assert_eq!(started.entry, first);
    assert!(!started.run.is_empty());
    assert!(matches!(
        started.spawned,
        Err(crate::dispatch::SpawnFailed::Refused(ref why)) if why == "no descriptor"
    ));
}

#[test]
fn every_start_outcome_is_reported_and_only_three_stop_the_claims() {
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
            reason: "r".into(),
        })
    ));
    assert!(record_start(
        &mut passed,
        crate::dispatch::start::Outcome::Held(Held {
            claim,
            refusal: StartRefusal::Compile,
            count: 1,
            reason: "compile: c".into(),
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
            subject: Subject::Claim(EntryId(1), claim),
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

    let stop = record_start(&mut passed, crate::dispatch::start::Outcome::Stopped);
    assert!(!stop);
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
        claims: always_claims(),
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
        claims: always_claims(),
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
        claims: always_claims(),
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
    assert_eq!(passed.contended[0].subject, Subject::Pass);
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
type LockingSeams = PassSeams<
    'static,
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
            claims: always_claims(),
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
    assert_eq!(passed.contended[0].subject, Subject::Entry(entry));
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
    assert_eq!(
        passed.contended[0].subject,
        Subject::Claim(claimed.entry, claimed.claim)
    );
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
fn a_contended_reconciliation_spawn_keeps_every_settled_item_for_the_next_pass() {
    use crate::dispatch::reconcile::Settled;

    let dir = tempfile::tempdir().unwrap();
    let (claimed, run, mut store) = linked_claimed(dir.path());
    store
        .set_patience(std::time::Duration::from_millis(20))
        .unwrap();
    let mut passed = Passed {
        reconciled: vec![
            Settled::ToSpawn(claimed.claim, run.clone()),
            Settled::Reclaimed(claimed.claim),
        ],
        ..Passed::default()
    };
    let (held, mut seams) = lock_seams(store.path().to_owned());
    assert!(
        !spawned_after_reconcile(&mut store, &claimed._dispatcher, &mut passed, &mut seams,)
            .unwrap()
    );
    let conn = held.borrow_mut().take();
    drop(conn);
    assert_eq!(
        passed.reconciled,
        vec![
            Settled::ToSpawn(claimed.claim, run),
            Settled::Reclaimed(claimed.claim),
        ]
    );
}

#[test]
fn a_contended_judge_is_reported_and_claims_nothing() {
    let (mut fixture, entry, dispatcher, host_file) = a_named_map_whose_house_moved();
    fixture
        .store
        .set_patience(std::time::Duration::from_millis(20))
        .unwrap();
    let path = fixture.store.path().to_path_buf();
    let held = Rc::new(RefCell::new(None::<rusqlite::Connection>));
    let lock = Rc::clone(&held);
    let passed = pass_with(
        &mut fixture.store,
        &dispatcher,
        &host(&host_file),
        OsString::new(),
        quiet(),
        watching(move |stage| {
            if stage != Stage::Judge {
                return;
            }
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.busy_timeout(std::time::Duration::from_millis(20))
                .unwrap();
            conn.execute_batch("BEGIN IMMEDIATE").unwrap();
            *lock.borrow_mut() = Some(conn);
        }),
    )
    .unwrap();
    let conn = held.borrow_mut().take();
    drop(conn);
    assert_eq!(passed.contended.len(), 1, "{:?}", passed.contended);
    assert_eq!(passed.contended[0].stage, Stage::Judge);
    assert_eq!(passed.contended[0].subject, Subject::Pass);
    assert!(claims(&fixture.store).is_empty());
    assert_eq!(state(&fixture.store, entry), EntryState::Queued);
}

#[test]
fn a_store_refusal_while_judging_ends_the_pass_and_claims_nothing() {
    let (mut fixture, entry, dispatcher, host_file) = a_named_map_whose_house_moved();
    sql(
        &fixture.store,
        "CREATE TRIGGER no_latch BEFORE INSERT ON queue_latches \
         BEGIN SELECT RAISE(ABORT, 'no latch'); END;",
    );
    let refused = run_pass(&mut fixture.store, &dispatcher, &host_file, quiet()).unwrap_err();
    assert_store_refused(&fixture.store, entry, refused);
}

#[test]
fn a_claim_whose_moved_run_does_not_fold_is_reported_and_stops_claims() {
    use brokkr_core::EventType;

    let (mut fixture, file, dispatcher, entries) = setup(1);
    let entry = entries[0];
    let path = fixture.store.path().to_path_buf();
    let seams = Seams {
        claims: || true,
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

#[test]
fn a_crossing_that_moves_between_the_pass_and_the_start_counts_a_reclaim() {
    let mut fixture = Fixture::new();
    let file = fixture.declare(&ceilings(1, 1, json!({})));
    let crossing = fixture.root.join("contracts/orders.v1.schema.json");
    std::fs::create_dir_all(crossing.parent().unwrap()).unwrap();
    std::fs::write(&crossing, "{\"title\": \"orders\"}\n").unwrap();
    queue_named(
        &mut fixture.store,
        &fixture.root,
        json!({
            "schema": "forge.realms/v7",
            "journal": "forge.db",
            "realms": [{
                "name": "b", "path": ".", "default_branch": "main",
                "publishes": [{"name": "orders.api", "path": "contracts/orders.v1.schema.json"}]
            }]
        }),
    );
    let dispatcher = take_dispatch(&mut fixture.store).unwrap().unwrap();
    let seams = Seams {
        claims: || true,
        stands: move || {
            std::fs::remove_file(&crossing).unwrap();
            Ok(())
        },
        spawn: spawned_one,
    };
    let passed = run_pass(&mut fixture.store, &dispatcher, &file, seams).unwrap();
    assert_eq!(passed.reclaimed.len(), 1, "{passed:?}");
    assert_eq!(passed.reclaimed[0].refused, Some(StartRefusal::Crossing));
    assert_eq!(passed.reclaimed[0].count, 1);
}

/// 6a F1: a stand that ends in the reconciliation's spawn keeps the run it
/// stood on, and every later item, in `Ended.passed.reconciled`.
#[test]
fn a_stand_that_ends_in_the_reconciliation_spawn_keeps_every_settled_item() {
    use crate::dispatch::reconcile::Settled;

    let dir = tempfile::tempdir().unwrap();
    let (claimed, run, mut store) = linked_claimed(dir.path());
    let seams = Seams {
        claims: || true,
        stands: || Err(Ending::Stood),
        spawn: spawned_one,
    };
    let host_file = dir.path().join("absent-host.json");
    let ended = run_pass(&mut store, &claimed._dispatcher, &host_file, seams).unwrap_err();
    assert!(matches!(ended.ending, Ending::Stood));
    assert_eq!(
        ended.passed.reconciled,
        vec![Settled::ToSpawn(claimed.claim, run)]
    );
    assert!(ended.passed.spawned.is_empty());
}

/// 6a F2: a `spawn` row the store refuses ends the pass, and the ending's
/// report names the run it linked and its claim.
#[test]
fn a_refused_spawn_row_ends_the_pass_naming_the_run_it_linked() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    sql(
        &fixture.store,
        "CREATE TRIGGER no_spawn BEFORE INSERT ON queue_commands \
         WHEN NEW.command = 'spawn' BEGIN SELECT RAISE(ABORT, 'no spawn'); END;",
    );
    let ended = run_pass(&mut fixture.store, &dispatcher, &file, quiet()).unwrap_err();
    assert!(matches!(ended.ending, Ending::Store(StoreError::Sqlite(_))));
    let EntryState::Claimed { run, .. } = state(&fixture.store, entries[0]) else {
        panic!("linked");
    };
    assert_left(&fixture.store, &ended.passed.left, &run);
    assert!(ended.passed.started.is_empty());
}

/// A launch whose start is `Contended`, writing no run: the pass-level
/// fault a `Contended` start is stood by.
fn contended_start(
    _: crate::launch::LaunchRequest,
    _: crate::launch::NewRun,
    _: &mut dyn FnMut(&str),
) -> Result<crate::Engine, LaunchError> {
    Err(LaunchError::Store(StoreError::Contended {
        operation: "start_run",
        waited_ms: 1,
    }))
}

/// A launch refused as a compile that first takes the journal's write lock
/// and lets it go 150ms later, so the disposal after it is `Contended`
/// until it lands.
fn refused_under_a_held_lock(
    request: crate::launch::LaunchRequest,
    _: crate::launch::NewRun,
    _: &mut dyn FnMut(&str),
) -> Result<crate::Engine, LaunchError> {
    let holder = lock_journal(&request.journal);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        holder.execute_batch("ROLLBACK").unwrap();
    });
    Err(LaunchError::RecipeNotFound {
        name: "recipe".into(),
        recipes_dir: "recipes".into(),
    })
}

/// One pass over `fixture`'s journal whose every start is `launch`.
fn faulted(
    fixture: &mut Fixture,
    dispatcher: &Dispatcher,
    file: &Path,
    launch: crate::dispatch::start::Launch,
) -> Passed {
    let watch = Watch {
        observe: |_| {},
        launch,
    };
    let host = host(file);
    pass_with(
        &mut fixture.store,
        dispatcher,
        &host,
        OsString::new(),
        quiet(),
        watch,
    )
    .unwrap()
}

/// Decision 0110 ruling 2, through the pass: a `Contended` start returns
/// its entry uncounted, and the pass goes on.
#[test]
fn a_contended_start_is_reclaimed_uncounted() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let passed = faulted(&mut fixture, &dispatcher, &file, contended_start);
    assert_eq!(passed.reclaimed.len(), 1, "{passed:?}");
    let reclaimed = &passed.reclaimed[0];
    assert_eq!((reclaimed.refused, reclaimed.count), (None, 0));
    let (entry, seq) = reclaimed.claim.parts();
    let said = format!("start contended: claim {seq} of entry {entry} wrote no run");
    assert_eq!(reclaimed.reason, said);
    assert_eq!(state(&fixture.store, entries[0]), EntryState::Queued);
    assert_eq!(runs(&fixture.store), Vec::<String>::new());
    let refusals = crate::dispatch::pass::tests::support::refusals(&fixture.store);
    assert_eq!(refusals, Vec::<String>::new());
}

/// Decision 0095 ruling 7, through the pass: a `Contended` disposal is
/// retried until its hold lands, and no further claim lands before it.
#[test]
fn a_contended_disposal_is_retried_until_it_lands_with_no_other_claim() {
    let (mut fixture, file, dispatcher, entries) = setup(2);
    fixture
        .store
        .set_patience(std::time::Duration::from_millis(20))
        .unwrap();
    let by = brokkr_store::Attribution {
        operator: "vy",
        reason: "bound",
    };
    let one = std::num::NonZeroU32::MIN;
    fixture.store.set_dispatch_bound(one, by).unwrap();
    let passed = faulted(&mut fixture, &dispatcher, &file, refused_under_a_held_lock);
    assert_eq!(passed.held.len(), 2, "{passed:?}");
    assert!(passed.contended.is_empty(), "{:?}", passed.contended);
    let conn = rusqlite::Connection::open(fixture.store.path()).unwrap();
    let mut rows = conn
        .prepare("SELECT command, entry_id FROM queue_commands WHERE command != 'add' ORDER BY seq")
        .unwrap();
    let read = |row: &rusqlite::Row<'_>| Ok((row.get(0)?, EntryId(row.get(1)?)));
    let steps: Vec<(String, EntryId)> = rows
        .query_map([], read)
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let [first, second] = [entries[0], entries[1]];
    assert_eq!(
        steps,
        [
            ("claim".to_string(), first),
            ("hold".into(), first),
            ("claim".into(), second),
            ("hold".into(), second),
        ]
    );
}

/// What admission finds of `entry` now, `None` once it is no longer
/// judged.
fn reasons_of(fixture: &Fixture, file: &Path, entry: EntryId) -> Option<Vec<Reason>> {
    let judged = admission::pass_within(&fixture.store, &host(file))
        .unwrap()
        .0;
    let judged = judged.into_iter().find(|judged| judged.entry.id == entry);
    judged
        .and_then(|judged| judged.verdict)
        .map(|verdict| verdict.reasons)
}

/// #1192's acceptance, through the pass: B waits on `A:shipped`. It waits
/// while A's run runs, starts once A ships clean, and is held, saying the
/// ending A came to, once A ships with its review's residuals flagged.
#[test]
fn an_entry_waiting_on_shipped_starts_when_it_ships_and_is_held_when_it_ships_flagged() {
    use brokkr_core::ending::Ending as Came;
    for (severity, ships) in [("normal", true), ("flagged", false)] {
        let (mut fixture, file, dispatcher, entries) = setup(1);
        let a = entries[0];
        let run = one_pass(&mut fixture.store, &dispatcher, &file).0.started[0]
            .run
            .clone();
        let wait = brokkr_store::Wait {
            entry: a,
            on: brokkr_store::WaitOn::Named(Came::Shipped),
        };
        let payload = fixture.launch_in("bundle").encode().unwrap();
        let b = brokkr_store::NewEntry {
            payload: &payload,
            priority: 0,
            waits: &[wait],
        };
        let by = crate::admission::capacity::tests::BY;
        let b = fixture.store.queue_add(b, by).unwrap();
        let unended = Reason::Unended {
            wait,
            run: run.clone(),
        };
        let waiting = one_pass(&mut fixture.store, &dispatcher, &file).0;
        assert_eq!(waiting.started.len(), 0);
        assert_eq!(waiting.waiting, waiting.waits.len());
        assert_eq!(
            waiting.waits,
            [Waiting {
                entry: b,
                reasons: vec![unended],
            }]
        );

        reviewed(&mut fixture.store, &run, severity);
        let passed = one_pass(&mut fixture.store, &dispatcher, &file).0;
        let started: Vec<EntryId> = passed.started.iter().map(|s| s.entry).collect();
        let otherwise = Reason::EndedOtherwise {
            wait,
            run,
            ending: Came::ShippedFlagged,
        };
        let expected = match ships {
            true => (vec![b], None),
            false => (vec![], Some(vec![otherwise])),
        };
        assert_eq!((started, reasons_of(&fixture, &file, b)), expected);
    }
}

/// End `run` through one effect of `phase` that came to `result`, the
/// policy table's ruling `ruled`, then `tail`.
fn ruled_end(
    store: &mut Store,
    run: &str,
    (phase, result): (&str, &str),
    ruled: serde_json::Value,
    tail: &[(brokkr_core::EventType, serde_json::Value)],
) {
    use brokkr_core::EventType;
    let effect = json!({"effect_id": "e", "phase": phase, "seat": phase});
    let attempt = json!({"effect_id": "e", "attempt_id": "a"});
    let succeeded = json!({"effect_id": "e", "attempt_id": "a", "result": {"result": result}});
    let ends = [
        (EventType::PhaseEntered, json!({ "phase": phase })),
        (EventType::EffectRequested, effect),
        (EventType::EffectStarted, attempt),
        (EventType::EffectSucceeded, succeeded),
        (EventType::TransitionDecided, ruled),
    ];
    for (event, payload) in ends.into_iter().chain(tail.iter().cloned()) {
        (store.append_next(run, event, payload, None, None)).unwrap();
    }
}

/// Complete `run` after review's `residual`, ruled with `severity`: it
/// ships clean when `normal`, flagged when `flagged`.
fn reviewed(store: &mut Store, run: &str, severity: &str) {
    let ruled = json!({"from": "review", "result": "residual", "next": "done",
                       "rule_id": "R", "severity": severity});
    let completed = (brokkr_core::EventType::RunCompleted, json!({}));
    ruled_end(store, run, ("review", "residual"), ruled, &[completed]);
}

/// Queue an entry of the fixture's bundle that waits on `waits`.
fn waiting_on(fixture: &mut Fixture, waits: &[brokkr_store::Wait]) -> EntryId {
    let payload = fixture.launch_in("bundle").encode().unwrap();
    let new = brokkr_store::NewEntry {
        payload: &payload,
        priority: 0,
        waits,
    };
    let by = crate::admission::capacity::tests::BY;
    fixture.store.queue_add(new, by).unwrap()
}

/// Record entry `a`'s work merged, as `brokkr queue merged` does.
fn merged(store: &mut Store, a: EntryId) {
    let by = crate::admission::capacity::tests::BY;
    let merged = brokkr_store::QueueCommand::Merged;
    assert_eq!(store.queue_command(a, merged, by).unwrap(), None);
}

/// The run the pass starts for the fixture's one entry.
fn first_run(fixture: &mut Fixture, dispatcher: &Dispatcher, file: &Path) -> String {
    let passed = one_pass(&mut fixture.store, dispatcher, file).0;
    passed.started[0].run.clone()
}

/// The entries a pass starts.
fn started_by(fixture: &mut Fixture, dispatcher: &Dispatcher, file: &Path) -> Vec<EntryId> {
    let passed = one_pass(&mut fixture.store, dispatcher, file).0;
    passed.started.iter().map(|s| s.entry).collect()
}

/// #1192's merge, through the pass and its claim: B waits on `A:merged`.
/// It waits while A's run runs, is held once A ships with no merge
/// recorded, and starts once the operator records A's merge, the claim's
/// recheck reading that record afresh in its own transaction.
#[test]
fn an_entry_waiting_on_merged_is_held_once_its_run_ships_and_starts_once_a_merge_is_recorded() {
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let a = entries[0];
    let run = first_run(&mut fixture, &dispatcher, &file);
    let wait = brokkr_store::Wait {
        entry: a,
        on: brokkr_store::WaitOn::Merged,
    };
    let b = waiting_on(&mut fixture, &[wait]);
    let unended = Reason::Unended {
        wait,
        run: run.clone(),
    };
    let started = started_by(&mut fixture, &dispatcher, &file);
    let reasons = reasons_of(&fixture, &file, b);
    assert_eq!((started, reasons), (vec![], Some(vec![unended])));

    reviewed(&mut fixture.store, &run, "normal");
    let started = started_by(&mut fixture, &dispatcher, &file);
    let unmerged = Reason::Unmerged { wait, run };
    let reasons = reasons_of(&fixture, &file, b);
    assert_eq!((started, reasons), (vec![], Some(vec![unmerged])));

    merged(&mut fixture.store, a);
    let started = started_by(&mut fixture, &dispatcher, &file);
    assert_eq!((started, reasons_of(&fixture, &file, b)), (vec![b], None));
}

/// The merge record is the fact a `merged` wait reads, and only it reads
/// it (#1192): a merge recorded on A whose run stopped `failed` starts B,
/// waiting on `A:merged`; one recorded on A that shipped flagged leaves an
/// entry waiting on `A:shipped` held as before, and one waiting on
/// `A:completed` starts as before.
#[test]
fn a_merge_record_meets_a_merged_wait_whatever_the_run_ended_and_moves_no_other_wait() {
    use brokkr_core::ending::Ending as Came;
    use brokkr_core::EventType;
    use brokkr_store::WaitOn;
    let on = |entry, on| brokkr_store::Wait { entry, on };
    let (mut fixture, file, dispatcher, entries) = setup(1);
    let a = entries[0];
    let run = first_run(&mut fixture, &dispatcher, &file);
    let ruled = json!({"from": "verify", "result": "fail", "next": "stop", "rule_id": "R"});
    let stopped = [
        (EventType::PhaseEntered, json!({"phase": "stop"})),
        (
            EventType::RunStopped,
            json!({"reason": "verification failed"}),
        ),
    ];
    ruled_end(
        &mut fixture.store,
        &run,
        ("verify", "fail"),
        ruled,
        &stopped,
    );
    merged(&mut fixture.store, a);
    let b = waiting_on(&mut fixture, &[on(a, WaitOn::Merged)]);
    assert_eq!(started_by(&mut fixture, &dispatcher, &file), vec![b]);

    let (mut fixture, file, dispatcher, entries) = setup(1);
    let a = entries[0];
    let run = first_run(&mut fixture, &dispatcher, &file);
    reviewed(&mut fixture.store, &run, "flagged");
    merged(&mut fixture.store, a);
    let shipped = on(a, WaitOn::Named(Came::Shipped));
    let b = waiting_on(&mut fixture, &[shipped]);
    let c = waiting_on(&mut fixture, &[on(a, WaitOn::Completed)]);
    let started = started_by(&mut fixture, &dispatcher, &file);
    let otherwise = Reason::EndedOtherwise {
        wait: shipped,
        run,
        ending: Came::ShippedFlagged,
    };
    let held = reasons_of(&fixture, &file, b);
    assert_eq!((started, held), (vec![c], Some(vec![otherwise])));
}
