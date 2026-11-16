//! One entry's start and the disposal of a refused start (decision 0110
//! ruling 2; decision 0095 rulings 1 and 7): the classes of every
//! `LaunchError` variant, the uncounted and counted reclaims, the hold at
//! the bound, a `Contended` link and `spawn` left for reconciliation, and a
//! `Contended` disposal retried until it lands.

use std::cell::RefCell;
use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

use brokkr_core::realms::Boundary;
use brokkr_store::{Claim, Generation, QueueRefusal, StartRefusal, Store, StoreError};

use super::*;
use crate::boundary::Unboxable;
use crate::dispatch::{Passed, Seams, Stage};
use crate::launch::tests::claimed::claimed;
use crate::launch::QueuedLaunch;

use crate::dispatch::pass::tests::support::{
    commands, count, holding, host, refusals, spawned_one, stands_ok,
};

/// A claim, its generation and the launch that rebuilt it, for a disposal
/// that needs no start.
fn disposed(root: &Path) -> (Claim, Generation, QueuedLaunch) {
    let claimed = claimed(root, None);
    let queued = QueuedLaunch::of(&claimed.request, &claimed.run).unwrap();
    let claim = claimed.claim;
    // Keep the dispatcher alive for the journal's generation, then let it
    // go: the disposal is a step under the same store, not the lock.
    drop(claimed._dispatcher);
    (claim, claimed.generation, queued)
}

/// Dispose of `claim` as a bundle-moved refusal, uncounted.
fn dispose_bundle_moved(
    store: &mut Store,
    claim: Claim,
    generation: Generation,
    queued: &QueuedLaunch,
) -> Outcome {
    dispose(
        store,
        claim,
        generation,
        NonZeroU32::MIN,
        Class::BundleMoved("measured".into()),
        queued,
    )
    .unwrap()
}

/// The `reclaim` row's reason in `store`.
fn reclaim_reason(store: &Store) -> String {
    commands(store)
        .into_iter()
        .find(|(_, command, ..)| command == "reclaim")
        .expect("a reclaim")
        .3
}

/// One claimed entry whose run has started, its dispatcher still holding
/// the journal, and a store on the same journal.
struct Started {
    claimed: crate::launch::tests::claimed::Claimed,
    run: String,
    store: Store,
}

fn started(root: &Path) -> Started {
    let crate::launch::tests::claimed::Claimed {
        _dispatcher,
        generation,
        entry,
        claim,
        request,
        run,
    } = claimed(root, None);
    let engine = crate::launch::start(request.clone(), run, &mut |_| {}).unwrap();
    let run = engine.run_id.clone();
    drop(engine);
    let store = Store::open(&root.join("forge.db")).unwrap();
    Started {
        claimed: crate::launch::tests::claimed::Claimed {
            _dispatcher,
            generation,
            entry,
            claim,
            request,
            run: crate::launch::NewRun::default(),
        },
        run,
        store,
    }
}

/// Link and spawn under `claim`, with the two seams given.
fn linked(
    store: &mut Store,
    dispatcher: &crate::dispatch::Dispatcher,
    claim: Claim,
    run: &str,
    stands: &mut dyn FnMut() -> Result<(), Ending>,
    spawn: &mut dyn FnMut(&Spawn) -> Result<Spawned, SpawnFailed>,
) -> Outcome {
    link_and_spawn(store, dispatcher, claim, run, stands, spawn).unwrap()
}

/// Link and spawn with a stand that always stands and a spawn of one run.
fn linked_quiet(
    store: &mut Store,
    dispatcher: &crate::dispatch::Dispatcher,
    claim: Claim,
    run: &str,
) -> Outcome {
    let mut stands = stands_ok;
    let mut spawn = spawned_one;
    linked(store, dispatcher, claim, run, &mut stands, &mut spawn)
}

/// [`started`] with the store's patience short enough for a held lock to
/// contend within the test.
fn started_contended(root: &Path) -> Started {
    let mut started = started(root);
    started
        .store
        .set_patience(Duration::from_millis(20))
        .unwrap();
    started
}

#[test]
fn a_class_maps_every_launch_refusal_to_its_disposal() {
    assert!(matches!(
        class(LaunchError::ClaimWithDispatch),
        Class::Start(LaunchError::ClaimWithDispatch)
    ));
    assert!(matches!(
        class(LaunchError::World(WorldError::Missing("no map".into()))),
        Class::Counted(StartRefusal::Compile)
    ));
    assert!(matches!(
        class(LaunchError::World(WorldError::Crossing {
            realm: "b".into(),
            crossing: "c".into(),
            path: "p".into(),
            detail: "d".into(),
        })),
        Class::Counted(StartRefusal::Crossing)
    ));
    assert!(matches!(
        class(LaunchError::Unboxable(Unboxable::MissingTool {
            boundary: Boundary::Namespace,
            tool: "bwrap",
            seats: vec![],
        })),
        Class::Counted(StartRefusal::Unboxable)
    ));
    assert!(matches!(
        class(LaunchError::Store(StoreError::RunNotFound("gone".into()))),
        Class::Store(StoreError::RunNotFound(_))
    ));
    assert!(matches!(
        class(LaunchError::Store(StoreError::Contended {
            operation: "start",
            waited_ms: 1,
        })),
        Class::Contended
    ));
    assert!(matches!(
        class(LaunchError::Store(StoreError::Queue(
            QueueRefusal::Superseded(Generation(3))
        ))),
        Class::Fenced(Generation(3))
    ));
    assert!(matches!(
        class(LaunchError::Store(StoreError::Queue(
            QueueRefusal::SchemaMismatch { found: 9 }
        ))),
        Class::QueueStorage(QueueRefusal::SchemaMismatch { found: 9 })
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::Invariant("x", "y".into()))),
        Class::Start(LaunchError::Engine(EngineError::Invariant(..)))
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::UnbuiltBoundary {
            boundary: Boundary::Namespace,
            slice: "ii",
            seats: vec![],
        })),
        Class::Counted(StartRefusal::Compile)
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::World(
            WorldError::Missing("no map".into())
        ))),
        Class::Counted(StartRefusal::Compile)
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::Store(
            StoreError::Contended {
                operation: "start",
                waited_ms: 1,
            }
        ))),
        Class::Contended
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::Store(StoreError::Queue(
            QueueRefusal::Superseded(Generation(3))
        )))),
        Class::Fenced(Generation(3))
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::Store(StoreError::Queue(
            QueueRefusal::SchemaMismatch { found: 9 }
        )))),
        Class::QueueStorage(QueueRefusal::SchemaMismatch { found: 9 })
    ));
    assert!(matches!(
        class(LaunchError::Engine(EngineError::Store(
            StoreError::RunNotFound("gone".into())
        ))),
        Class::Store(StoreError::RunNotFound(_))
    ));
}

#[test]
fn a_run_lock_ends_and_an_uninstalled_recipe_counts() {
    // These bind the arms a wildcard would otherwise swallow: a run lock
    // ends the dispatcher rather than counting, and a recipe that is not
    // installed counts as a compile.
    assert!(matches!(
        class(LaunchError::Engine(EngineError::RunLock(
            crate::run_lock::RunLockError::UnsafeId("run".into())
        ))),
        Class::Start(LaunchError::Engine(EngineError::RunLock(_)))
    ));
    assert!(matches!(
        class(LaunchError::RecipeNotFound {
            name: "recipe".into(),
            recipes_dir: "recipes".into(),
        }),
        Class::Counted(StartRefusal::Compile)
    ));
}

#[test]
fn a_store_refusal_splits_contention_from_an_ending() {
    assert!(matches!(
        refused(StoreError::Contended {
            operation: "x",
            waited_ms: 1,
        }),
        Refused::Contended(_)
    ));
    assert!(matches!(
        refused(StoreError::Queue(QueueRefusal::Superseded(Generation(2)))),
        Refused::Fenced(Generation(2))
    ));
    assert!(matches!(
        refused(StoreError::Queue(QueueRefusal::SchemaMismatch { found: 8 })),
        Refused::QueueStorage(QueueRefusal::SchemaMismatch { found: 8 })
    ));
    assert!(matches!(
        refused(StoreError::Queue(QueueRefusal::UnknownEntry(
            brokkr_store::EntryId(1)
        ))),
        Refused::Store(StoreError::Queue(QueueRefusal::UnknownEntry(_)))
    ));
    assert!(matches!(
        refused(StoreError::RunNotFound("gone".into())),
        Refused::Store(StoreError::RunNotFound(_))
    ));
}

#[test]
fn an_ended_store_refusal_keeps_its_class() {
    assert!(matches!(
        ended(StoreError::Contended {
            operation: "x",
            waited_ms: 1,
        }),
        Ending::Store(StoreError::Contended { .. })
    ));
    assert!(matches!(
        ended(StoreError::Queue(QueueRefusal::Superseded(Generation(2)))),
        Ending::Fenced(Generation(2))
    ));
    assert!(matches!(
        ended(StoreError::Queue(QueueRefusal::SchemaMismatch { found: 8 })),
        Ending::QueueStorage(QueueRefusal::SchemaMismatch { found: 8 })
    ));
    assert!(matches!(
        ended(StoreError::RunNotFound("gone".into())),
        Ending::Store(StoreError::RunNotFound(_))
    ));
}

#[test]
fn a_disposal_that_is_any_other_refusal_writes_nothing_and_ends() {
    for class in [
        Class::Fenced(Generation(7)),
        Class::QueueStorage(QueueRefusal::SchemaMismatch { found: 9 }),
        Class::Store(StoreError::RunNotFound("gone".into())),
        Class::Start(LaunchError::ClaimWithDispatch),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (claim, generation, queued) = disposed(dir.path());
        let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
        let refused = dispose(
            &mut store,
            claim,
            generation,
            NonZeroU32::MIN,
            class,
            &queued,
        )
        .unwrap_err();
        match refused {
            Ending::Fenced(g) => assert_eq!(g, Generation(7)),
            Ending::QueueStorage(QueueRefusal::SchemaMismatch { found }) => assert_eq!(found, 9),
            Ending::Store(StoreError::RunNotFound(_)) | Ending::Start(_) => {}
            other => panic!("{other:?}"),
        }
        assert_eq!(count(&store, "reclaim"), 0);
        assert_eq!(count(&store, "hold"), 0);
    }
}

#[test]
fn a_contended_start_is_reclaimed_uncounted() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let class = class(LaunchError::Store(StoreError::Contended {
        operation: "start",
        waited_ms: 1,
    }));
    let outcome = dispose(
        &mut store,
        claim,
        generation,
        NonZeroU32::MIN,
        class,
        &queued,
    )
    .unwrap();
    assert!(matches!(outcome, Outcome::Reclaimed(ref reclaimed) if reclaimed.refused.is_none()));
    assert_eq!(count(&store, "reclaim"), 1);
    assert_eq!(refusals(&store), Vec::<String>::new());
}

#[test]
fn a_bundle_moved_names_both_digests_or_an_unreadable_remeasure() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(dir.path());
    let actual = queued.compiled().unwrap().manifest_digest();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let outcome = dispose_bundle_moved(&mut store, claim, generation, &queued);
    assert!(matches!(outcome, Outcome::Reclaimed(ref reclaimed) if reclaimed.refused.is_none()));
    let reason = reclaim_reason(&store);
    assert!(reason.contains("measured"), "{reason}");
    assert!(reason.contains(&actual), "{reason}");
    assert_eq!(refusals(&store), Vec::<String>::new());

    let unreadable = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(unreadable.path());
    std::fs::remove_dir_all(unreadable.path().join("bundle")).unwrap();
    let mut store = Store::open(&unreadable.path().join("forge.db")).unwrap();
    let outcome = dispose_bundle_moved(&mut store, claim, generation, &queued);
    assert!(matches!(outcome, Outcome::Reclaimed(_)));
    assert!(reclaim_reason(&store).contains("unreadable"));
}

#[test]
fn a_bundle_moved_carried_by_the_launch_store_is_uncounted() {
    let dir = tempfile::tempdir().unwrap();
    let claimed = claimed(dir.path(), None);
    let moved = QueueRefusal::BundleMoved(claimed.claim, "measured".into());
    assert!(matches!(
        class(LaunchError::Store(StoreError::Queue(moved))),
        Class::BundleMoved(ref measured) if measured == "measured"
    ));
}

#[test]
fn a_counted_disposal_holds_at_a_bound_of_one_with_its_variant() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let outcome = dispose(
        &mut store,
        claim,
        generation,
        NonZeroU32::MIN,
        Class::Counted(StartRefusal::Compile),
        &queued,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Outcome::Held(ref held)
            if held.refusal == StartRefusal::Compile && held.count == 1
    ));
    assert_eq!(count(&store, "hold"), 1);
    assert_eq!(refusals(&store), vec!["compile".to_string()]);
    let (_, _, _, reason) = commands(&store)
        .into_iter()
        .find(|(_, command, ..)| command == "hold")
        .unwrap();
    assert_eq!(reason, "compile");
}

#[test]
fn a_counted_disposal_below_the_bound_reclaims_and_counts() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let bound = NonZeroU32::new(3).unwrap();
    let outcome = dispose(
        &mut store,
        claim,
        generation,
        bound,
        Class::Counted(StartRefusal::Crossing),
        &queued,
    )
    .unwrap();
    assert!(matches!(
        outcome,
        Outcome::Reclaimed(ref reclaimed)
            if reclaimed.refused == Some(StartRefusal::Crossing) && reclaimed.count == 1
    ));
    assert_eq!(count(&store, "reclaim"), 1);
    assert_eq!(refusals(&store), vec!["crossing".to_string()]);
}

#[test]
fn a_contended_disposal_is_retried_until_it_lands_with_no_other_claim() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store.set_patience(Duration::from_millis(20)).unwrap();
    let holder = holding(&store);
    let released = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        holder.execute_batch("ROLLBACK").unwrap();
    });
    let outcome = dispose(
        &mut store,
        claim,
        generation,
        NonZeroU32::MIN,
        Class::Counted(StartRefusal::Compile),
        &queued,
    )
    .unwrap();
    released.join().unwrap();
    assert!(matches!(
        outcome,
        Outcome::Held(ref held) if held.refusal == StartRefusal::Compile
    ));
    assert_eq!(count(&store, "hold"), 1);
    assert_eq!(refusals(&store), vec!["compile".to_string()]);
}

#[test]
fn a_contended_link_leaves_the_entry_for_reconciliation_and_is_never_reclaimed() {
    let dir = tempfile::tempdir().unwrap();
    let Started {
        claimed,
        run,
        mut store,
    } = started_contended(dir.path());
    let holder = holding(&store);
    let outcome = linked_quiet(&mut store, &claimed._dispatcher, claimed.claim, &run);
    assert!(matches!(
        outcome,
        Outcome::Contended(ref contended) if contended.stage == Stage::Link
    ));
    assert_eq!(count(&store, "reclaim"), 0);
    holder.execute_batch("ROLLBACK").unwrap();

    let settled = crate::dispatch::reconcile(&mut store, &claimed._dispatcher)
        .unwrap()
        .unwrap();
    assert_eq!(
        settled,
        vec![crate::dispatch::Settled::ToSpawn(
            claimed.claim,
            run.clone()
        )]
    );
    let seen = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&seen);
    let passed: Passed = crate::dispatch::pass(
        &mut store,
        &claimed._dispatcher,
        &host(&dir.path().join("absent-host.json")),
        OsString::new(),
        Seams {
            stands: || Ok(()),
            spawn: move |spawn: &Spawn| {
                record.borrow_mut().push(spawn.run.clone());
                Ok(Spawned { pid: 3 })
            },
        },
    )
    .unwrap();
    assert_eq!(passed.spawned.len(), 1);
    assert_eq!(seen.borrow().as_slice(), std::slice::from_ref(&run));
}

#[test]
fn a_contended_spawn_row_is_left_for_reconciliation_and_never_reclaimed() {
    let dir = tempfile::tempdir().unwrap();
    let Started {
        claimed,
        run,
        mut store,
    } = started_contended(dir.path());
    let path = store.path().to_path_buf();
    let held = Rc::new(RefCell::new(None::<rusqlite::Connection>));
    let holder = Rc::clone(&held);
    let mut stands = move || {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("BEGIN IMMEDIATE").unwrap();
        *holder.borrow_mut() = Some(conn);
        Ok(())
    };
    let mut spawn = spawned_one;
    let outcome = linked(
        &mut store,
        &claimed._dispatcher,
        claimed.claim,
        &run,
        &mut stands,
        &mut spawn,
    );
    assert!(matches!(
        outcome,
        Outcome::Contended(ref contended) if contended.stage == Stage::Spawn
    ));
    assert_eq!(count(&store, "reclaim"), 0);
    assert_eq!(count(&store, "spawn"), 0);
    let conn = held.borrow_mut().take();
    if let Some(conn) = conn {
        conn.execute_batch("ROLLBACK").unwrap();
    }
}

#[test]
fn a_stand_that_ends_before_a_spawn_row_leaves_the_link_alone() {
    let dir = tempfile::tempdir().unwrap();
    let Started {
        claimed,
        run,
        mut store,
    } = started(dir.path());
    let mut stands = || Err(Ending::Stood);
    let mut spawn = spawned_one;
    let refused = link_and_spawn(
        &mut store,
        &claimed._dispatcher,
        claimed.claim,
        &run,
        &mut stands,
        &mut spawn,
    )
    .unwrap_err();
    assert!(matches!(refused, Ending::Stood));
    assert_eq!(count(&store, "spawn"), 0);
    assert_eq!(
        store.queue_entry(claimed.entry).unwrap().state,
        brokkr_store::EntryState::Claimed { run }
    );
}

#[test]
fn contention_is_reported_and_any_other_step_refusal_ends() {
    let reported = contend(
        StoreError::Contended {
            operation: "link",
            waited_ms: 1,
        },
        Subject::Entry(brokkr_store::EntryId(1)),
        Stage::Link,
    )
    .unwrap();
    assert!(matches!(
        reported,
        Outcome::Contended(ref contended) if contended.stage == Stage::Link
    ));
    let ended = contend(
        StoreError::RunNotFound("gone".into()),
        Subject::Entry(brokkr_store::EntryId(1)),
        Stage::Link,
    )
    .unwrap_err();
    assert!(matches!(ended, Ending::Store(StoreError::RunNotFound(_))));
}

#[test]
fn a_reclaim_that_is_fenced_out_ends_the_disposal() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation, queued) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    conn.execute_batch(
        "INSERT INTO dispatch_holders (generation, host, pid, started_at) \
         VALUES (99, NULL, 1, 'then')",
    )
    .unwrap();
    let refused = dispose(
        &mut store,
        claim,
        generation,
        NonZeroU32::MIN,
        Class::Counted(StartRefusal::Compile),
        &queued,
    )
    .unwrap_err();
    assert!(matches!(refused, Ending::Fenced(Generation(1))));
    assert_eq!(count(&store, "hold"), 0);
    assert_eq!(count(&store, "reclaim"), 0);
}

#[test]
fn every_counted_word_and_queue_storage_class_is_named() {
    assert_eq!(StartRefusal::Crossing.word(), "crossing");
    assert_eq!(StartRefusal::Compile.word(), "compile");
    assert_eq!(StartRefusal::Unboxable.word(), "unboxable");

    let ended = contend(
        StoreError::Queue(QueueRefusal::SchemaMismatch { found: 9 }),
        Subject::Entry(brokkr_store::EntryId(1)),
        Stage::Claim,
    )
    .unwrap_err();
    assert!(matches!(
        ended,
        Ending::QueueStorage(QueueRefusal::SchemaMismatch { found: 9 })
    ));

    // The notifier `launch::start` takes, kept as a value so it needs no
    // closure allocation when a start says nothing.
    silent("nothing to say");
}
