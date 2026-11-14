//! The sweep (decision 0104 rulings 7 to 9, as R3 to R6 amend them): a
//! closed effect's tree goes with its bytes and the pass's generation, a
//! parked run's open effect keeps every tree, unproven cleanup and an
//! attempt no event names keep theirs, the path shape refuses a `..`, a
//! link, another root and another filesystem, a partial removal records
//! nothing, an absent path closes, and the hands half records what the
//! reaper removed.

#[path = "../../../../tests/support/env_guard.rs"]
mod env_guard;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_protocol::hands::Listed;
use brokkr_protocol::tree::{self, TreeError};
use brokkr_store::{Store, Swept as Closed};
use serde_json::{json, Value};
use uuid::Uuid;

use env_guard::EnvGuard;

use super::*;
use crate::envelope_builder::EnvelopeBuilder;

const RUN: &str = "run-sweep";
const RUN2: &str = "run-sweep-two";
const EFFECT: &str = "effect-sweep";
const ATTEMPT: &str = "11111111-1111-4111-8111-111111111111";
const ATTEMPT2: &str = "22222222-2222-4222-8222-222222222222";

/// A machine of the test's own: the environment held for the test's whole
/// body, with `TMPDIR` at a directory the test owns, so the hands half
/// reads and reaps nothing of the host's.
fn own_temp() -> (EnvGuard, PathBuf) {
    let mut env = EnvGuard::lock();
    // Kept, not deleted: another test's `tempfile::tempdir()` can stand
    // inside this one while the guard holds `TMPDIR`, and deleting it
    // would take that test's journal with it.
    let tmp = tempfile::tempdir().unwrap().keep();
    env.set("TMPDIR", &tmp);
    (env, tmp)
}

fn journal(dir: &Path) -> Store {
    Store::open(&dir.join("forge.db")).unwrap()
}

fn append(
    store: &mut Store,
    run: &str,
    event_type: EventType,
    payload: Value,
    attempt: Option<&str>,
) -> EventEnvelope {
    store
        .append_next(run, event_type, payload, None, attempt.map(str::to_string))
        .unwrap()
}

/// A run opened and its effect requested.
fn opened(store: &mut Store, run: &str) {
    store
        .create_run(run, "a feature", "self", &json!({}))
        .unwrap();
    append(
        store,
        run,
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
        None,
    );
    append(
        store,
        run,
        EventType::PhaseEntered,
        json!({"phase": "work"}),
        None,
    );
    append(
        store,
        run,
        EventType::EffectRequested,
        json!({"effect_id": EFFECT, "seat": "worker"}),
        None,
    );
}

fn started(store: &mut Store, run: &str, attempt: &str) {
    append(
        store,
        run,
        EventType::EffectStarted,
        json!({"effect_id": EFFECT, "attempt_id": attempt}),
        Some(attempt),
    );
}

fn succeeded(store: &mut Store, run: &str, attempt: &str) {
    append(
        store,
        run,
        EventType::EffectSucceeded,
        json!({"effect_id": EFFECT, "result": {"result": "ok"}}),
        Some(attempt),
    );
}

fn failed(store: &mut Store, run: &str, attempt: &str) {
    append(
        store,
        run,
        EventType::EffectFailed,
        json!({"effect_id": EFFECT}),
        Some(attempt),
    );
}

fn indeterminate(store: &mut Store, run: &str, attempt: &str, extra: Value) {
    let mut payload = json!({
        "effect_id": EFFECT,
        "attempt_id": attempt,
        "reason": "the driver stopped answering",
    });
    payload
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    append(
        store,
        run,
        EventType::EffectIndeterminate,
        payload,
        Some(attempt),
    );
}

/// The seat's evidence an unresolved cleanup carries.
fn seat_evidence() -> Value {
    json!({
        "received": {"status": "failed", "error": "the driver stopped"},
        "cleanup": {"state": "unresolved", "reason": "a child still runs"},
    })
}

/// The panel's evidence an unresolved member carries.
fn panel_evidence() -> Value {
    json!({
        "unresolved_members": {
            "member": {
                "received": {"status": "failed", "error": "the driver stopped"},
                "cleanup": {"state": "unresolved", "reason": "a child still runs"},
            }
        },
    })
}

/// Park a run after a failed attempt, so its effect stays open with a
/// terminal event on the attempt.
fn parked_after_failure(store: &mut Store, run: &str, attempt: &str) {
    started(store, run, attempt);
    failed(store, run, attempt);
    append(
        store,
        run,
        EventType::RunParked,
        json!({"reason": "the seat's attempts are exhausted"}),
        None,
    );
}

/// Stop a parked run through the operator's command, as a conclusion does.
fn stopped(store: &mut Store, run: &str) {
    append(
        store,
        run,
        EventType::OperatorCommanded,
        json!({"command_id": "stop-1", "command": "stop"}),
        None,
    );
    append(
        store,
        run,
        EventType::OperatorAccepted,
        json!({"command_id": "stop-1"}),
        None,
    );
    append(store, run, EventType::RunStopped, json!({}), None);
}

fn completed(store: &mut Store, run: &str) {
    append(store, run, EventType::RunCompleted, json!({}), None);
}

fn scratch(dir: &Path) -> PathBuf {
    let root = dir.join("scratch");
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn plant(root: &Path, attempt: &str) -> PathBuf {
    let tree = root.join(attempt);
    std::fs::create_dir_all(tree.join("home")).unwrap();
    std::fs::write(tree.join("home").join("cache"), b"x").unwrap();
    tree
}

fn seat(store: &mut Store, run: &str, effect: &str, attempt: &str, root: &Path) {
    store.record_seat_tree(run, effect, attempt, root).unwrap();
}

/// Take the journal as the pass's dispatcher, run one pass, and hand back
/// the generation the holder row recorded.
fn pass_one(store: &mut Store) -> (Swept, i64) {
    let held = crate::dispatch::take_dispatch(store).unwrap().unwrap();
    let generation = held.generation.0;
    (pass(store, &held).unwrap(), generation)
}

/// The bytes `tree` holds, counted before a pass removes it.
fn bytes(tree: &Path) -> u64 {
    tree::count_tree(tree).unwrap().unwrap().bytes
}

/// The one tree a test expected the pass to remove.
fn removed(swept: &Swept) -> &Removed {
    assert_eq!(swept.removed().len(), 1);
    &swept.removed()[0]
}

/// A closed effect's tree and a named copy of its path, held for a seam.
fn target(fixture: &mut Fixture) -> (PathBuf, PathBuf) {
    fixture.succeeded(RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);
    (tree.clone(), tree)
}

/// Take the journal as a dispatcher for a pass driven through a seam.
fn held(fixture: &mut Fixture) -> crate::dispatch::Dispatcher {
    crate::dispatch::take_dispatch(&mut fixture.store)
        .unwrap()
        .unwrap()
}

/// Run a pass through a seam under the journal's own dispatcher.
fn walk(
    fixture: &mut Fixture,
    device_of: impl Fn(&Path, u64) -> u64,
    remove: impl Fn(&Path) -> Result<Option<u64>, TreeError>,
) -> Swept {
    let held = held(fixture);
    pass_with(&mut fixture.store, &held, device_of, remove).unwrap()
}

/// The one reason a pass kept a tree.
fn kept_reason(swept: &Swept) -> &str {
    assert_eq!(swept.kept().len(), 1);
    &swept.kept()[0].reason
}

/// Run a pass and hand back the one reason it kept the tree for.
fn kept_after_pass(fixture: &mut Fixture) -> String {
    let (swept, _) = pass_one(&mut fixture.store);
    kept_reason(&swept).to_string()
}

/// One test's own machine and journal, with the scratch root a closed
/// effect's tree is planted under.
struct Fixture {
    tmp: PathBuf,
    store: Store,
    root: PathBuf,
    _env: EnvGuard,
}

impl Fixture {
    fn new() -> Fixture {
        let (env, tmp) = own_temp();
        let store = journal(&tmp);
        let root = scratch(&tmp.join("run"));
        Fixture {
            tmp,
            store,
            root,
            _env: env,
        }
    }

    fn opened(&mut self, run: &str) {
        opened(&mut self.store, run);
    }

    /// A run opened, its attempt started and succeeded.
    fn succeeded(&mut self, run: &str, attempt: &str) {
        self.opened(run);
        started(&mut self.store, run, attempt);
        succeeded(&mut self.store, run, attempt);
    }

    /// A tree planted at `root/attempt` and its row recorded.
    fn seated(&mut self, run: &str, attempt: &str, root: &Path) -> PathBuf {
        let tree = plant(root, attempt);
        seat(&mut self.store, run, EFFECT, attempt, root);
        tree
    }

    fn here(&mut self, run: &str, attempt: &str) -> PathBuf {
        let root = self.root.clone();
        self.seated(run, attempt, &root)
    }

    /// A closed effect's tree, with the bytes it holds.
    fn closed(&mut self, attempt: &str) -> (PathBuf, u64) {
        self.succeeded(RUN, attempt);
        let tree = self.here(RUN, attempt);
        let freed = bytes(&tree);
        (tree, freed)
    }
}

#[test]
fn a_closed_effects_tree_goes_with_its_bytes_and_the_generation() {
    let mut fixture = Fixture::new();
    let (tree, freed) = fixture.closed(ATTEMPT);

    let (swept, generation) = pass_one(&mut fixture.store);

    assert!(!tree.exists());
    let removal = removed(&swept);
    assert_eq!(removal.kind, SweepKind::Scratch);
    assert_eq!(removal.path, tree);
    assert_eq!(removal.bytes, Some(freed));
    assert!(swept.kept().is_empty());
    assert!(swept.failed().is_empty());
    assert_eq!(swept.freed(), freed);
    let seq = fixture.store.seat_trees(RUN).unwrap()[0].seq;
    let sweeps = fixture.store.tree_sweeps().unwrap();
    assert_eq!(sweeps.len(), 1);
    assert_eq!(sweeps[0].kind, Closed::Scratch(seq));
    assert_eq!(sweeps[0].path, tree);
    assert_eq!(sweeps[0].bytes, Some(freed as i64));
    assert_eq!(sweeps[0].generation, generation);
    assert_eq!(fixture.store.open_seat_trees().unwrap(), vec![]);
}

#[test]
fn a_parked_runs_open_effect_keeps_every_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    parked_after_failure(&mut fixture.store, RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);

    assert!(tree.exists());
    assert_eq!(kept_after_pass(&mut fixture), "its effect is still open");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn an_attempt_with_unproven_cleanup_keeps_its_tree() {
    for evidence in [seat_evidence(), panel_evidence()] {
        let mut fixture = Fixture::new();
        fixture.opened(RUN);
        started(&mut fixture.store, RUN, ATTEMPT);
        indeterminate(&mut fixture.store, RUN, ATTEMPT, evidence);
        stopped(&mut fixture.store, RUN);
        let tree = fixture.here(RUN, ATTEMPT);

        assert!(tree.exists());
        assert_eq!(
            kept_after_pass(&mut fixture),
            "its attempt carries unproven cleanup evidence"
        );
        assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    }
}

#[test]
fn an_attempt_with_no_terminal_event_keeps_its_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    append(
        &mut fixture.store,
        RUN,
        EventType::RunStopped,
        json!({}),
        None,
    );
    let tree = fixture.here(RUN, ATTEMPT);

    assert!(tree.exists());
    assert_eq!(
        kept_after_pass(&mut fixture),
        "no terminal event names its attempt"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_row_whose_attempt_no_event_names_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT2);

    assert!(tree.exists());
    assert_eq!(kept_after_pass(&mut fixture), "no event names its attempt");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_root_that_is_not_its_canonical_path_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let root = PathBuf::from("/x/../scratch");
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &root);

    assert_eq!(
        kept_after_pass(&mut fixture),
        "its root is not its own canonical path"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_root_through_a_symlinked_component_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let real = fixture.tmp.join("real");
    let root = real.join("scratch");
    let tree = plant(&root, ATTEMPT);
    let link = fixture.tmp.join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let named = link.join("scratch");
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &named);

    assert!(tree.exists());
    assert_eq!(
        kept_after_pass(&mut fixture),
        "its root is not its own canonical path"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn two_roots_in_one_run_keep_both_trees() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    failed(&mut fixture.store, RUN, ATTEMPT);
    started(&mut fixture.store, RUN, ATTEMPT2);
    succeeded(&mut fixture.store, RUN, ATTEMPT2);
    let one = scratch(&fixture.tmp.join("one"));
    let two = scratch(&fixture.tmp.join("two"));
    let first = fixture.seated(RUN, ATTEMPT, &one);
    let second = fixture.seated(RUN, ATTEMPT2, &two);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(first.exists());
    assert!(second.exists());
    assert_eq!(swept.kept().len(), 2);
    for kept in swept.kept() {
        assert_eq!(kept.reason, "another row of its run names a different root");
    }
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_link_at_the_attempt_path_is_kept_and_not_followed() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let target = fixture.tmp.join("target");
    std::fs::create_dir_all(target.join("kept")).unwrap();
    std::os::unix::fs::symlink(&target, fixture.root.join(ATTEMPT)).unwrap();
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);

    assert!(target.join("kept").is_dir());
    assert_eq!(
        kept_after_pass(&mut fixture),
        "its tree is a link, not a directory"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_tree_on_another_filesystem_is_kept() {
    let mut fixture = Fixture::new();
    let (named, tree) = target(&mut fixture);

    let swept = walk(
        &mut fixture,
        |path, device| match path == named.as_path() {
            true => device + 1,
            false => device,
        },
        tree::remove_tree,
    );

    assert!(tree.exists());
    assert_eq!(
        kept_reason(&swept),
        "its tree is on another filesystem than its root"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_partial_removal_records_nothing() {
    let mut fixture = Fixture::new();
    let (named, tree) = target(&mut fixture);

    let swept = walk(&mut fixture, super::same_device, |path| {
        match path == named.as_path() {
            true => Err(TreeError::Io {
                path: path.to_path_buf(),
                cause: std::io::Error::from_raw_os_error(13),
            }),
            false => tree::remove_tree(path),
        }
    });

    assert!(tree.exists());
    assert_eq!(swept.failed().len(), 1);
    assert_eq!(swept.failed()[0].path, tree);
    assert_eq!(swept.failed()[0].cause, "Permission denied (os error 13)");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

#[test]
fn an_absent_path_closes_with_bytes_null() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);

    let (swept, _) = pass_one(&mut fixture.store);

    assert_eq!(swept.removed()[0].bytes, None);
    assert_eq!(swept.freed(), 0);
    let seq = fixture.store.seat_trees(RUN).unwrap()[0].seq;
    assert_eq!(
        fixture.store.tree_sweeps().unwrap()[0].kind,
        Closed::Scratch(seq)
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap()[0].bytes, None);
    assert_eq!(fixture.store.open_seat_trees().unwrap(), vec![]);
}

#[test]
fn a_pass_over_a_completed_run_appends_no_event() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    let tree = fixture.here(RUN, ATTEMPT);
    let before = fixture.store.load(RUN).unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(!tree.exists());
    assert_eq!(swept.removed().len(), 1);
    let after = fixture.store.load(RUN).unwrap();
    assert_eq!(after.len(), before.len());
    assert_eq!(
        after.last().unwrap().event_hash,
        before.last().unwrap().event_hash
    );
}

#[test]
fn a_hands_tree_whose_owner_lives_is_kept() {
    let mut fixture = Fixture::new();
    let tree = hands_tree(&fixture.tmp, std::process::id(), "live");
    std::fs::write(tree.join(".owner.lock"), "").unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(tree.exists());
    assert!(swept.removed().is_empty());
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_dead_owners_free_hands_tree_is_recorded_as_hands() {
    let mut fixture = Fixture::new();
    let tree = hands_tree(&fixture.tmp, exited_pid(), "dead");
    std::fs::write(tree.join(".owner.lock"), "").unwrap();
    let freed = bytes(&tree);

    let (swept, generation) = pass_one(&mut fixture.store);

    assert!(!tree.exists());
    let removal = removed(&swept);
    assert_eq!(removal.kind, SweepKind::Hands);
    assert_eq!(removal.bytes, Some(freed));
    let sweeps = fixture.store.tree_sweeps().unwrap();
    assert_eq!(sweeps.len(), 1);
    assert_eq!(sweeps[0].kind, Closed::Hands);
    assert_eq!(sweeps[0].path, tree);
    assert_eq!(sweeps[0].bytes, Some(freed as i64));
    assert_eq!(sweeps[0].generation, generation);
}

#[test]
fn a_hands_tree_with_no_lock_is_kept_and_reported() {
    let mut fixture = Fixture::new();
    let tree = hands_tree(&fixture.tmp, exited_pid(), "lockless");

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(tree.exists());
    assert_eq!(swept.kept().len(), 1);
    assert!(swept.kept()[0].reason.contains("holds no lock file"));
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_dry_run_lists_what_a_pass_would_remove_without_removing_it() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    fixture.succeeded(RUN2, ATTEMPT2);
    let tree = fixture.here(RUN, ATTEMPT);
    let freed = bytes(&tree);
    seat(&mut fixture.store, RUN2, EFFECT, ATTEMPT2, &fixture.root);

    let plan = plan(&fixture.store).unwrap();

    assert!(tree.exists());
    assert_eq!(plan.removed().len(), 2);
    assert_eq!(plan.removed()[0].bytes, Some(freed));
    assert_eq!(plan.removed()[1].bytes, None);
    assert_eq!(plan.freed(), freed);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_dry_run_lists_a_dead_hands_tree_with_its_size() {
    let fixture = Fixture::new();
    let tree = hands_tree(&fixture.tmp, exited_pid(), "planned");
    std::fs::write(tree.join(".owner.lock"), "").unwrap();
    let freed = bytes(&tree);

    let plan = plan(&fixture.store).unwrap();

    assert!(tree.exists());
    assert_eq!(plan.removed()[0].kind, SweepKind::Hands);
    assert_eq!(plan.removed()[0].path, tree);
    assert_eq!(plan.removed()[0].bytes, Some(freed));
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_queue_that_does_not_record_sweeps_refuses_the_removal() {
    let mut fixture = Fixture::new();
    let (tree, _) = fixture.closed(ATTEMPT);
    let held = crate::dispatch::take_dispatch(&mut fixture.store)
        .unwrap()
        .unwrap();
    let path = fixture.store.path().to_path_buf();
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute_batch("UPDATE meta SET value = '5' WHERE key = 'queue_schema';")
        .unwrap();

    let refused = pass(&mut fixture.store, &held).unwrap_err();

    assert!(!tree.exists());
    assert_eq!(
        refused.to_string(),
        "a tree was removed but not recorded: the journal's queue is not one this binary writes \
         tree sweeps in"
    );
}

#[test]
fn the_lines_name_every_tree_and_the_total() {
    let swept = Swept {
        removed: vec![
            Removed {
                kind: SweepKind::Scratch,
                path: PathBuf::from("/run/scratch/one"),
                bytes: Some(10),
            },
            Removed {
                kind: SweepKind::Hands,
                path: PathBuf::from("/tmp/hands\x1b[2J"),
                bytes: None,
            },
        ],
        kept: vec![Kept {
            path: PathBuf::from("/run/scratch/two"),
            reason: "its effect is still open".into(),
        }],
        failed: vec![Failed {
            path: PathBuf::from("/run/scratch/three"),
            cause: "Permission denied".into(),
        }],
    };
    let done = swept.lines_with(safe, false);
    assert_eq!(
        done,
        "sweep: removed /run/scratch/one: 10 bytes\n\
         sweep: removed /tmp/hands[2J: it was already gone\n\
         sweep: kept /run/scratch/two: its effect is still open\n\
         sweep: could not remove /run/scratch/three: Permission denied\n\
         sweep: freed 10 bytes\n"
    );
    let would = swept.lines_with(safe, true);
    assert_eq!(
        would,
        "sweep: would remove /run/scratch/one: 10 bytes\n\
         sweep: would remove /tmp/hands[2J: it is already gone\n\
         sweep: kept /run/scratch/two: its effect is still open\n\
         sweep: could not remove /run/scratch/three: Permission denied\n\
         sweep: would free 10 bytes\n"
    );
}

#[test]
fn a_run_that_does_not_load_keeps_its_row() {
    let mut fixture = Fixture::new();
    let (tree, _) = fixture.closed(ATTEMPT);
    let db = fixture.store.path().to_path_buf();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch(
            "DROP TRIGGER events_append_only_update;
             UPDATE events SET envelope = replace(envelope, 'a feature', 'spoiled')
                 WHERE run_id = 'run-sweep' AND seq = 1;",
        )
        .unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(tree.exists());
    assert_eq!(swept.kept()[0].reason, "its run cannot be read");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
}

#[test]
fn a_run_that_does_not_fold_keeps_its_row() {
    let mut fixture = Fixture::new();
    let (tree, _) = fixture.closed(ATTEMPT);
    let last = fixture.store.load(RUN).unwrap().last().unwrap().clone();
    let out_of_place = EnvelopeBuilder::new(
        EventType::EffectSucceeded,
        json!({"effect_id": "other", "result": {"result": "ok"}}),
    )
    .run(RUN)
    .seq(last.seq + 1)
    .event_id("out-of-place")
    .previous(last.event_hash)
    .sealed();
    let db = fixture.store.path().to_path_buf();
    brokkr_store::test_support::plant_envelope(
        &rusqlite::Connection::open(&db).unwrap(),
        &out_of_place,
    )
    .unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(tree.exists());
    assert_eq!(swept.kept()[0].reason, "its run does not fold");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
}

#[test]
fn a_row_the_checks_should_refuse_is_kept_by_its_shape() {
    let mut fixture = Fixture::new();
    let unreadable = scratch(&fixture.tmp.join("sealed").join("scratch"));
    for run in ["bad-uuid", "bad-root", "bad-file", "unreadable"] {
        fixture
            .store
            .create_run(run, "a feature", "self", &json!({}))
            .unwrap();
        append(
            &mut fixture.store,
            run,
            EventType::RunStarted,
            json!({"feature": "a feature", "manifest": {}}),
            None,
        );
    }
    let bad_file = fixture.root.join(ATTEMPT2);
    std::fs::write(&bad_file, b"x").unwrap();
    std::fs::create_dir_all(unreadable.join(ATTEMPT)).unwrap();
    let conn = rusqlite::Connection::open(fixture.store.path()).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    for (run, attempt, root) in [
        ("bad-uuid", "not-a-uuid", fixture.root.as_path()),
        ("bad-root", ATTEMPT, Path::new("/x/not-scratch")),
        ("bad-file", ATTEMPT2, fixture.root.as_path()),
        ("unreadable", ATTEMPT, unreadable.as_path()),
    ] {
        conn.execute(
            "INSERT INTO seat_trees (run_id, effect_id, attempt_id, root, recorded_at)
             VALUES (?1, 'effect', ?2, ?3, 0)",
            rusqlite::params![run, attempt, root.to_str().unwrap()],
        )
        .unwrap();
    }
    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o000)).unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        swept
            .kept()
            .iter()
            .map(|kept| kept.reason.as_str())
            .collect::<Vec<_>>(),
        [
            "its attempt id is not the one the engine mints",
            "its root is not an absolute path ending in 'scratch'",
            "its tree is not a directory",
            "its tree cannot be read",
        ]
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_plan_count_that_refuses_is_reported_and_removes_nothing() {
    let mut fixture = Fixture::new();
    let (tree, _) = fixture.closed(ATTEMPT);
    let empty = tempfile::tempdir().unwrap();

    let plan = plan_with(&fixture.store, empty.path(), super::same_device, |_| {
        Err(TreeError::Io {
            path: PathBuf::from("/x"),
            cause: std::io::Error::from_raw_os_error(13),
        })
    })
    .unwrap();

    assert!(tree.exists());
    assert_eq!(plan.removed(), &[]);
    assert_eq!(plan.failed()[0].cause, "Permission denied (os error 13)");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_hands_listing_whose_count_refuses_is_reported() {
    let mut swept = Swept::default();
    plan_hands_with(
        [
            Listed {
                tree: PathBuf::from("/tmp/brokkr-hands-x"),
                count: Err(TreeError::Io {
                    path: PathBuf::from("/tmp/brokkr-hands-x"),
                    cause: std::io::Error::from_raw_os_error(13),
                }),
            },
            Listed {
                tree: PathBuf::from("/tmp/brokkr-hands-gone"),
                count: Ok(None),
            },
        ]
        .into_iter(),
        &mut swept,
    );
    assert_eq!(swept.removed()[0].bytes, None);
    assert_eq!(
        swept.removed()[0].path,
        PathBuf::from("/tmp/brokkr-hands-gone")
    );
    assert_eq!(swept.failed()[0].cause, "Permission denied (os error 13)");
}

#[test]
fn a_foreign_removal_cause_names_no_path() {
    let foreign = TreeError::Foreign {
        path: PathBuf::from("/tree/mounted"),
        device: 2,
        tree: 1,
    };
    assert_eq!(
        cause(&foreign),
        "is on another filesystem (device 2, not 1); a tree is removed on its own filesystem"
    );
}

/// A hands tree named the way the reaper parses it.
fn hands_tree(tmp: &Path, pid: u32, label: &str) -> PathBuf {
    let tree = tmp.join(format!("brokkr-hands-{label}-{pid}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(tree.join("session")).unwrap();
    std::fs::write(tree.join("session").join("data"), b"x").unwrap();
    tree
}

/// A pid the kernel has reaped: a child that ran to completion.
fn exited_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    assert!(child.wait().unwrap().success());
    pid
}

/// A safe renderer for the lines: the CLI's own strips controls and the
/// bidi marks, and this pins the same visible result for a control byte.
fn safe(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control() && *character != '\u{202e}')
        .collect()
}
