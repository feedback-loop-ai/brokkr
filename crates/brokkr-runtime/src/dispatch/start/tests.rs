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

use crate::dispatch::pass::tests::support::{
    commands, count, holding, host, refusals, spawned_one, stands_ok,
};

/// A claim and its generation, for a disposal that needs no start.
fn disposed(root: &Path) -> (Claim, Generation) {
    let claimed = claimed(root, None);
    let claim = claimed.claim;
    // Keep the dispatcher alive for the journal's generation, then let it
    // go: the disposal is a step under the same store, not the lock.
    drop(claimed._dispatcher);
    (claim, claimed.generation)
}

/// A counted class of `refusal` with the start's words `words`.
fn counted_as(refusal: StartRefusal, words: &str) -> Class {
    Class::Counted(refusal, words.into())
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

/// Link and spawn under `claim`, with the stand given and a spawn of one
/// run: the outcome, and every run the steps left.
fn linked(
    store: &mut Store,
    dispatcher: &crate::dispatch::Dispatcher,
    claim: Claim,
    run: &str,
    stands: &mut dyn FnMut() -> Result<(), Ending>,
) -> (Result<Outcome, Ending>, Vec<Left>) {
    let mut spawn = spawned_one;
    let mut left = Vec::new();
    let steps: Steps<'_> = (stands, &mut spawn);
    let outcome = link_and_spawn(store, dispatcher, claim, run, steps, &mut left);
    (outcome, left)
}

/// Link and spawn with a stand that always stands and a spawn of one run.
fn linked_quiet(
    store: &mut Store,
    dispatcher: &crate::dispatch::Dispatcher,
    claim: Claim,
    run: &str,
) -> (Outcome, Vec<Left>) {
    let mut stands = stands_ok;
    let (outcome, left) = linked(store, dispatcher, claim, run, &mut stands);
    (outcome.unwrap(), left)
}

/// The one run `left` names, as the report names it.
fn left_one(claim: Claim, run: &str) -> Vec<Left> {
    vec![Left {
        claim,
        run: run.into(),
    }]
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
    let missing = WorldError::Missing("no map".into());
    let words = missing.to_string();
    assert!(matches!(
        class(LaunchError::World(missing)),
        Class::Counted(StartRefusal::Compile, ref said) if *said == words
    ));
    let crossing = WorldError::Crossing {
        realm: "b".into(),
        crossing: "c".into(),
        path: "p".into(),
        detail: "d".into(),
    };
    let words = crossing.to_string();
    assert!(matches!(
        class(LaunchError::World(crossing)),
        Class::Counted(StartRefusal::Crossing, ref said) if *said == words
    ));
    let unboxable = LaunchError::Unboxable(Unboxable::MissingTool {
        boundary: Boundary::Namespace,
        tool: "bwrap",
        seats: vec![],
    });
    let words = unboxable.to_string();
    assert!(matches!(
        class(unboxable),
        Class::Counted(StartRefusal::Unboxable, ref said) if *said == words
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
        class(LaunchError::Engine(EngineError::World(
            WorldError::Missing("no map".into())
        ))),
        Class::Counted(StartRefusal::Compile, _)
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
    let unbuilt = EngineError::UnbuiltBoundary {
        boundary: Boundary::Namespace,
        slice: "ii",
        seats: vec![],
    };
    let words = unbuilt.to_string();
    assert!(matches!(
        class(LaunchError::Engine(unbuilt)),
        Class::Counted(StartRefusal::Compile, ref said) if *said == words
    ));
    let recipe = LaunchError::RecipeNotFound {
        name: "recipe".into(),
        recipes_dir: "recipes".into(),
    };
    let words = recipe.to_string();
    assert!(matches!(
        class(recipe),
        Class::Counted(StartRefusal::Compile, ref said) if *said == words
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

/// Dispose of a fresh claim as `class` under a bound of 1: the ending,
/// with no `reclaim` or `hold` written.
fn ended_by(class: Class) -> Ending {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let refused = dispose(&mut store, claim, generation, NonZeroU32::MIN, class).unwrap_err();
    assert_eq!(count(&store, "reclaim"), 0);
    assert_eq!(count(&store, "hold"), 0);
    refused
}

#[test]
fn a_disposal_that_is_any_other_refusal_writes_nothing_and_ends() {
    assert!(matches!(
        ended_by(Class::Fenced(Generation(7))),
        Ending::Fenced(Generation(7))
    ));
    assert!(matches!(
        ended_by(Class::QueueStorage(QueueRefusal::SchemaMismatch {
            found: 9
        })),
        Ending::QueueStorage(QueueRefusal::SchemaMismatch { found: 9 })
    ));
    assert!(matches!(
        ended_by(Class::Store(StoreError::RunNotFound("gone".into()))),
        Ending::Store(StoreError::RunNotFound(ref run)) if run == "gone"
    ));
    assert!(matches!(
        ended_by(Class::Start(LaunchError::ClaimWithDispatch)),
        Ending::Start(LaunchError::ClaimWithDispatch)
    ));
}

/// Decision 0110 ruling 2: a bundle moved under the claim is reclaimed
/// uncounted, naming the digest the claim recorded and the one the start
/// compiled, as the start's refusal carried them, with nothing compiled
/// again.
#[test]
fn a_bundle_moved_names_both_digests_the_start_carried() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation) = disposed(dir.path());
    // The bundle is gone, so a disposal that compiled again could not name
    // the start's digest.
    std::fs::remove_dir_all(dir.path().join("bundle")).unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let moved = Class::BundleMoved(digests());
    let outcome = dispose(&mut store, claim, generation, NonZeroU32::MIN, moved).unwrap();
    let (entry, seq) = claim.parts();
    let said = format!(
        "start refused: claim {seq} of entry {entry} measured bundle measured, and this start \
         compiled compiled"
    );
    assert!(matches!(
        outcome,
        Outcome::Reclaimed(ref reclaimed)
            if reclaimed.refused.is_none() && reclaimed.reason == said
    ));
    assert_eq!(reclaim_reason(&store), said);
    assert_eq!(refusals(&store), Vec::<String>::new());
}

#[test]
fn a_bundle_moved_carried_by_the_launch_store_is_uncounted() {
    let dir = tempfile::tempdir().unwrap();
    let claimed = claimed(dir.path(), None);
    let moved = QueueRefusal::BundleMoved(claimed.claim, digests());
    assert!(matches!(
        class(LaunchError::Store(StoreError::Queue(moved))),
        Class::BundleMoved(ref carried) if *carried == digests()
    ));
    let moved = QueueRefusal::BundleMoved(claimed.claim, digests());
    let engine = EngineError::Store(StoreError::Queue(moved));
    assert!(matches!(
        class(LaunchError::Engine(engine)),
        Class::BundleMoved(ref carried) if *carried == digests()
    ));
}

/// The digest a claim recorded and the one a start compiled.
fn digests() -> Box<(String, String)> {
    Box::new(("measured".into(), "compiled".into()))
}

#[test]
fn a_counted_disposal_holds_at_a_bound_of_one_with_its_variant_and_words() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let class = counted_as(StartRefusal::Compile, "the bundle does not parse");
    let outcome = dispose(&mut store, claim, generation, NonZeroU32::MIN, class).unwrap();
    let said = "compile: the bundle does not parse";
    assert!(matches!(
        outcome,
        Outcome::Held(ref held)
            if held.refusal == StartRefusal::Compile && held.count == 1 && held.reason == said
    ));
    assert_eq!(count(&store, "hold"), 1);
    assert_eq!(refusals(&store), vec!["compile".to_string()]);
    let (_, _, _, reason) = commands(&store)
        .into_iter()
        .find(|(_, command, ..)| command == "hold")
        .unwrap();
    assert_eq!(reason, said);
}

#[test]
fn a_counted_disposal_below_the_bound_reclaims_and_counts() {
    let dir = tempfile::tempdir().unwrap();
    let (claim, generation) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let bound = NonZeroU32::new(3).unwrap();
    let class = counted_as(StartRefusal::Crossing, "the crossing moved");
    let outcome = dispose(&mut store, claim, generation, bound, class).unwrap();
    assert!(matches!(
        outcome,
        Outcome::Reclaimed(ref reclaimed)
            if reclaimed.refused == Some(StartRefusal::Crossing)
                && reclaimed.count == 1
                && reclaimed.reason == "crossing: the crossing moved"
    ));
    assert_eq!(count(&store, "reclaim"), 1);
    assert_eq!(refusals(&store), vec!["crossing".to_string()]);
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
    let (outcome, left) = linked_quiet(&mut store, &claimed._dispatcher, claimed.claim, &run);
    assert!(matches!(
        outcome,
        Outcome::Contended(ref contended) if contended.stage == Stage::Link
    ));
    assert_eq!(
        left,
        left_one(claimed.claim, &run),
        "the report names the run"
    );
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
    let (outcome, left) = linked(
        &mut store,
        &claimed._dispatcher,
        claimed.claim,
        &run,
        &mut stands,
    );
    assert!(matches!(
        outcome,
        Ok(Outcome::Contended(ref contended)) if contended.stage == Stage::Spawn
    ));
    assert_eq!(
        left,
        left_one(claimed.claim, &run),
        "the report names the run"
    );
    assert_eq!(count(&store, "reclaim"), 0);
    assert_eq!(count(&store, "spawn"), 0);
    let conn = held.borrow_mut().take().unwrap();
    conn.execute_batch("ROLLBACK").unwrap();

    // The next pass's reconciliation spawns the run, once, and reclaims
    // nothing.
    let passed = crate::dispatch::pass(
        &mut store,
        &claimed._dispatcher,
        &host(&dir.path().join("absent-host.json")),
        OsString::new(),
        crate::dispatch::pass::tests::support::quiet(),
    )
    .unwrap();
    assert_eq!(
        passed.reconciled,
        vec![crate::dispatch::Settled::ToSpawn(
            claimed.claim,
            run.clone()
        )]
    );
    assert_eq!(passed.spawned.len(), 1);
    assert_eq!(passed.spawned[0].run, run);
    assert_eq!(count(&store, "spawn"), 1);
    assert_eq!(count(&store, "reclaim"), 0);
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
    let (refused, left) = linked(
        &mut store,
        &claimed._dispatcher,
        claimed.claim,
        &run,
        &mut stands,
    );
    assert!(matches!(refused, Err(Ending::Stood)));
    assert_eq!(
        left,
        left_one(claimed.claim, &run),
        "the report names the run"
    );
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
    let (claim, generation) = disposed(dir.path());
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    conn.execute_batch(
        "INSERT INTO dispatch_holders (generation, host, pid, started_at) \
         VALUES (99, NULL, 1, 'then')",
    )
    .unwrap();
    let class = counted_as(StartRefusal::Compile, "c");
    let refused = dispose(&mut store, claim, generation, NonZeroU32::MIN, class).unwrap_err();
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

/// A clock the store cannot read is a store error like any other the
/// pass does not report: it ends the dispatcher (decision 0110 ruling 2's
/// closed disposal; `StoreError::Clock` arrived with #430 slice 4f).
#[test]
fn a_store_clock_refusal_ends_the_dispatcher() {
    let error = StoreError::Clock {
        now: "not a stamp".to_string(),
    };
    assert!(matches!(
        refused(error),
        Refused::Store(StoreError::Clock { .. })
    ));
}
