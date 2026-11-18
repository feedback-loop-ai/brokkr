//! The sweep (decision 0104 rulings 7 to 9, as R3 to R6 and the operator's
//! succeeded-only ruling amend them): a closed effect's tree goes only when
//! its attempt ended `effect/succeeded`, with its bytes and the pass's
//! generation; a parked run's open effect keeps every tree, and a failed,
//! indeterminate, concluded or unended attempt keeps its own, with its
//! reason. The path shape refuses a `..`, a link, another root and another
//! filesystem, a partial removal records nothing, an absent path closes, and
//! the hands half records what the reaper removed and names what it kept.

#[path = "../../../../tests/support/env_guard.rs"]
mod env_guard;
mod gone;

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::ending::Ending;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_protocol::hands::Listed;
use brokkr_protocol::tree::{self, TreeError};
use brokkr_store::{Store, Swept as Closed};
use serde_json::{json, Value};
use uuid::Uuid;

use env_guard::EnvGuard;

use super::*;
use crate::engine::{Cause, Named};
use crate::envelope_builder::EnvelopeBuilder;
use crate::reaped::exited_pid;

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
    let tmp = std::fs::canonicalize(&tmp).unwrap();
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
    append_caused(store, run, event_type, payload, attempt, None)
}

fn append_caused(
    store: &mut Store,
    run: &str,
    event_type: EventType,
    payload: Value,
    attempt: Option<&str>,
    cause: Option<&str>,
) -> EventEnvelope {
    store
        .append_next(
            run,
            event_type,
            payload,
            cause.map(str::to_string),
            attempt.map(str::to_string),
        )
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

fn started(store: &mut Store, run: &str, attempt: &str) -> EventEnvelope {
    append(
        store,
        run,
        EventType::EffectStarted,
        json!({"effect_id": EFFECT, "attempt_id": attempt}),
        Some(attempt),
    )
}

fn succeeded(store: &mut Store, run: &str, attempt: &str) -> EventEnvelope {
    append(
        store,
        run,
        EventType::EffectSucceeded,
        json!({"effect_id": EFFECT, "result": {"result": "ok"}}),
        Some(attempt),
    )
}

fn failed(store: &mut Store, run: &str, attempt: &str) -> EventEnvelope {
    append(
        store,
        run,
        EventType::EffectFailed,
        json!({"effect_id": EFFECT}),
        Some(attempt),
    )
}

/// A checkpoint of the attempt, as a driver's sink or a restarted
/// engine's broker tallies journal one.
fn checkpoint(store: &mut Store, run: &str, attempt: &str) -> EventEnvelope {
    append(
        store,
        run,
        EventType::EffectCheckpointed,
        json!({"effect_id": EFFECT, "attempt_id": attempt, "checkpoint": {"step": "seat-turn"}}),
        Some(attempt),
    )
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

/// A run opened, and `attempt` started and failed.
fn opened_failed(store: &mut Store, run: &str, attempt: &str) {
    opened(store, run);
    started(store, run, attempt);
    failed(store, run, attempt);
}

/// Assert the pass kept the row for a gone root, and closed it for nothing
/// else.
fn kept_root_gone(swept: &Swept) {
    assert_eq!(
        kept_reason(swept),
        &KeptReason::Shape(ShapeReason::RootGone)
    );
    assert_eq!(swept.kept()[0].reason(), "its root is gone");
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

/// A succeeded attempt on a completed run, its row recorded with no tree:
/// the row a pass closes as gone.
fn gone_succeeded_on_completed(fixture: &mut Fixture) {
    fixture.succeeded(RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);
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
fn kept_reason(swept: &Swept) -> &KeptReason {
    assert_eq!(swept.kept().len(), 1);
    &swept.kept()[0].reason
}

/// Every reason a pass kept its trees for, in the order it kept them.
fn kept_reasons(swept: &Swept) -> Vec<KeptReason> {
    swept
        .kept()
        .iter()
        .map(|kept| kept.reason.clone())
        .collect()
}

/// Run a pass and hand back the one reason it kept the tree for.
fn kept_after_pass(fixture: &mut Fixture) -> KeptReason {
    let (swept, _) = pass_one(&mut fixture.store);
    kept_reason(&swept).clone()
}

/// Assert the pass kept `tree` for `reason` and recorded no sweep.
fn kept(fixture: &mut Fixture, tree: &Path, reason: KeptReason) {
    assert!(tree.exists());
    assert_eq!(kept_after_pass(fixture), reason);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// Assert the pass kept `tree` for `reason`, said exactly `words`, and
/// recorded no sweep.
fn kept_saying(fixture: &mut Fixture, tree: &Path, reason: KeptReason, words: &str) {
    assert!(tree.exists());
    let (swept, _) = pass_one(&mut fixture.store);
    assert_eq!(kept_reason(&swept), &reason);
    assert_eq!(swept.kept()[0].reason(), words);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// Run a pass under the journal's own dispatcher and hand back its refusal.
fn pass_refusal(fixture: &mut Fixture) -> SweepError {
    let held = held(fixture);
    pass(&mut fixture.store, &held).unwrap_err()
}

/// Make every `tree_sweeps` insert fail, so a complete removal's record
/// refusal is provable without a queue this binary cannot write.
fn refuse_records(store: &Store) {
    rusqlite::Connection::open(store.path())
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER forced_sweep_record BEFORE INSERT ON tree_sweeps
             BEGIN SELECT RAISE(ABORT, 'forced record failure'); END;",
        )
        .unwrap();
}

/// Assert `pass` returned a record refusal whose report names one removal
/// and one record failure, and hand the report back.
fn record_refusal(refused: &SweepError) -> &Swept {
    let SweepError::Record { report, .. } = refused else {
        panic!("a record refusal carries its report: {refused:?}");
    };
    assert_eq!(report.removed().len(), 1);
    assert!(matches!(report.failed()[0].cause, FailedCause::Record(_)));
    report
}

/// A run whose first attempt failed and whose second succeeded: its effect
/// is closed and both attempts have a terminal event.
fn failed_then_succeeded(fixture: &mut Fixture) {
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    failed(&mut fixture.store, RUN, ATTEMPT);
    started(&mut fixture.store, RUN, ATTEMPT2);
    succeeded(&mut fixture.store, RUN, ATTEMPT2);
}

/// A run whose one attempt failed, then completed: its effect is closed and
/// the attempt has a terminal event.
fn failed_on_completed(store: &mut Store, run: &str, attempt: &str) {
    opened(store, run);
    started(store, run, attempt);
    failed(store, run, attempt);
    completed(store, run);
}

/// Assert the pass closed one row as gone, with no bytes and no open row.
fn closed_gone(fixture: &Fixture, swept: &Swept) {
    assert_eq!(swept.removed()[0].bytes, None);
    assert_eq!(fixture.store.tree_sweeps().unwrap()[0].bytes, None);
    assert_eq!(fixture.store.open_seat_trees().unwrap(), vec![]);
}

/// Stop the run and plant its attempt's tree and row, handing the tree
/// back.
fn stopped_with_tree(fixture: &mut Fixture) -> PathBuf {
    append(
        &mut fixture.store,
        RUN,
        EventType::RunStopped,
        json!({}),
        None,
    );
    fixture.here(RUN, ATTEMPT)
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
    assert_eq!(kept_after_pass(&mut fixture), KeptReason::EffectOpen);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn an_attempt_that_ended_indeterminate_keeps_its_tree_whatever_evidence() {
    for evidence in [seat_evidence(), panel_evidence()] {
        let mut fixture = Fixture::new();
        fixture.opened(RUN);
        started(&mut fixture.store, RUN, ATTEMPT);
        indeterminate(&mut fixture.store, RUN, ATTEMPT, evidence);
        stopped(&mut fixture.store, RUN);
        let tree = fixture.here(RUN, ATTEMPT);

        kept(&mut fixture, &tree, KeptReason::Indeterminate);
    }
}

#[test]
fn an_attempt_with_no_terminal_event_keeps_its_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    let tree = stopped_with_tree(&mut fixture);

    kept(&mut fixture, &tree, KeptReason::NoTerminalEvent);
}

#[test]
fn a_row_whose_attempt_no_event_names_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT2);

    assert!(tree.exists());
    assert_eq!(
        kept_after_pass(&mut fixture),
        KeptReason::NoEventNamesAttempt
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A terminal event names an attempt no earlier event names: the attempt's
/// one typed ending is the event's, and the tree is kept by it.
#[test]
fn a_terminal_event_naming_an_unstarted_attempt_keeps_its_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    failed(&mut fixture.store, RUN, ATTEMPT2);
    completed(&mut fixture.store, RUN);
    let tree = fixture.here(RUN, ATTEMPT2);

    kept_saying(
        &mut fixture,
        &tree,
        KeptReason::Failed,
        "its attempt ended failed",
    );
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
        KeptReason::Shape(ShapeReason::RootNotCanonical)
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn two_roots_in_one_run_keep_both_trees() {
    let mut fixture = Fixture::new();
    failed_then_succeeded(&mut fixture);
    let one = scratch(&fixture.tmp.join("one"));
    let two = scratch(&fixture.tmp.join("two"));
    let first = fixture.seated(RUN, ATTEMPT, &one);
    let second = fixture.seated(RUN, ATTEMPT2, &two);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(first.exists());
    assert!(second.exists());
    assert_eq!(swept.kept().len(), 2);
    for kept in swept.kept() {
        assert_eq!(kept.reason, KeptReason::Shape(ShapeReason::AnotherRoot));
    }
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_later_row_naming_another_root_keeps_the_runs_first_tree() {
    let mut fixture = Fixture::new();
    failed_then_succeeded(&mut fixture);
    let one = scratch(&fixture.tmp.join("one"));
    let first = fixture.seated(RUN, ATTEMPT2, &one);
    let (swept, _) = pass_one(&mut fixture.store);
    assert!(!first.exists(), "the succeeded attempt's first row goes");
    assert_eq!(swept.removed().len(), 1);

    let two = scratch(&fixture.tmp.join("two"));
    let second = fixture.seated(RUN, ATTEMPT, &two);
    let (swept, _) = pass_one(&mut fixture.store);

    assert!(second.exists());
    assert_eq!(
        kept_reason(&swept),
        &KeptReason::Shape(ShapeReason::AnotherRoot)
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap().len(), 1);
}

#[test]
fn a_failed_attempt_on_a_completed_run_keeps_its_tree() {
    let mut fixture = Fixture::new();
    failed_on_completed(&mut fixture.store, RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);

    kept(&mut fixture, &tree, KeptReason::Failed);
}

#[test]
fn a_reused_attempt_keeps_its_tree_when_any_terminal_event_ended_indeterminate() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    indeterminate(&mut fixture.store, RUN, ATTEMPT, seat_evidence());
    append(
        &mut fixture.store,
        RUN,
        EventType::RunParked,
        json!({"reason": "the attempt's cleanup is unresolved"}),
        None,
    );
    append(
        &mut fixture.store,
        RUN,
        EventType::OperatorCommanded,
        json!({"command_id": "retry-1", "command": "retry"}),
        None,
    );
    append(
        &mut fixture.store,
        RUN,
        EventType::OperatorAccepted,
        json!({"command_id": "retry-1"}),
        None,
    );
    append(
        &mut fixture.store,
        RUN,
        EventType::EffectRequested,
        json!({"effect_id": EFFECT, "seat": "worker"}),
        None,
    );
    started(&mut fixture.store, RUN, ATTEMPT);
    failed(&mut fixture.store, RUN, ATTEMPT);
    let tree = stopped_with_tree(&mut fixture);

    kept(&mut fixture, &tree, KeptReason::Indeterminate);
}

/// A reused attempt id whose first ending failed and whose last succeeded.
/// The tree stays: one terminal event naming the attempt is not
/// `effect/succeeded`, and a later success does not undo an earlier
/// failure.
#[test]
fn a_reused_attempt_whose_last_ending_succeeded_keeps_its_tree() {
    let mut fixture = Fixture::new();
    opened_failed(&mut fixture.store, RUN, ATTEMPT);
    started(&mut fixture.store, RUN, ATTEMPT);
    succeeded(&mut fixture.store, RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    let tree = fixture.here(RUN, ATTEMPT);

    kept_saying(
        &mut fixture,
        &tree,
        KeptReason::Failed,
        "its attempt ended failed",
    );
}

#[test]
fn a_root_spelled_with_a_dot_or_dot_dot_component_is_kept() {
    for spelling in ["dot", "dot-dot"] {
        let mut fixture = Fixture::new();
        fixture.succeeded(RUN, ATTEMPT);
        let root = match spelling {
            "dot" => fixture.tmp.join("run").join(".").join("scratch"),
            _ => fixture
                .tmp
                .join("run")
                .join("..")
                .join("run")
                .join("scratch"),
        };
        let tree = plant(&root, ATTEMPT);
        seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &root);

        assert_eq!(
            kept_after_pass(&mut fixture),
            KeptReason::Shape(ShapeReason::RootNotCanonical),
            "{spelling}"
        );
        assert!(tree.exists(), "{spelling}");
        assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![], "{spelling}");
    }
}

#[test]
fn a_pass_refuses_a_dispatcher_that_took_another_journal() {
    let one = tempfile::tempdir().unwrap();
    let two = tempfile::tempdir().unwrap();
    let mut first = journal(one.path());
    let mut second = journal(two.path());
    let held = crate::dispatch::take_dispatch(&mut first).unwrap().unwrap();
    opened(&mut second, RUN);
    started(&mut second, RUN, ATTEMPT);
    succeeded(&mut second, RUN, ATTEMPT);
    let root = scratch(&two.path().join("run"));
    let tree = plant(&root, ATTEMPT);
    seat(&mut second, RUN, EFFECT, ATTEMPT, &root);

    let refused = pass(&mut second, &held).unwrap_err();

    assert!(matches!(
        refused,
        SweepError::Dispatcher(crate::dispatch::DispatcherError::OtherJournal(_, _))
    ));
    assert!(tree.exists());
    assert_eq!(second.tree_sweeps().unwrap(), vec![]);
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
        KeptReason::Shape(ShapeReason::TreeIsLink)
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
        &KeptReason::Shape(ShapeReason::ForeignFilesystem)
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
    assert_eq!(
        swept.failed()[0].cause.to_string(),
        "Permission denied (os error 13)"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

#[test]
fn an_absent_path_closes_with_bytes_null() {
    let mut fixture = Fixture::new();
    gone_succeeded_on_completed(&mut fixture);

    let (swept, _) = pass_one(&mut fixture.store);

    assert_eq!(swept.freed(), 0);
    let seq = fixture.store.seat_trees(RUN).unwrap()[0].seq;
    assert_eq!(
        fixture.store.tree_sweeps().unwrap()[0].kind,
        Closed::Scratch(seq)
    );
    closed_gone(&fixture, &swept);
}

/// A failed attempt whose tree the operator already removed closes its row
/// as gone, with no bytes: the accepted cost keeps the tree while it
/// stands, not a row forever afterwards.
#[test]
fn a_failed_attempt_whose_tree_is_gone_closes_its_row_as_gone() {
    let mut fixture = Fixture::new();
    failed_on_completed(&mut fixture.store, RUN, ATTEMPT);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(swept.kept().is_empty());
    closed_gone(&fixture, &swept);
}

/// An absent tree whose attempt has no terminal event may still be a spawn
/// in progress: it is kept, not closed as gone.
#[test]
fn an_absent_tree_of_an_unended_attempt_is_kept() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(swept.removed().is_empty());
    assert_eq!(kept_reason(&swept), &KeptReason::NoTerminalEvent);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    let open = fixture.store.open_seat_trees().unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].attempt_id, ATTEMPT);
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
    assert_eq!(
        swept.kept()[0].reason,
        KeptReason::Hands(
            "its lock cannot be probed: it holds no lock file; trees from before #415 are no \
             longer reaped (0.13.0)"
                .to_string()
        )
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// F3 of #430's 5c2 reviews: a dry run names the lockless hands tree a
/// pass keeps, with the pass's own words, and neither removes it.
#[test]
fn a_dry_run_and_a_pass_name_a_kept_lockless_hands_tree_alike() {
    let mut fixture = Fixture::new();
    let tree = hands_tree(&fixture.tmp, exited_pid(), "lockless");
    let line = format!(
        "sweep: kept {}: its lock cannot be probed: it holds no lock file; trees from before \
         #415 are no longer reaped (0.13.0)\n",
        tree.display()
    );

    let planned = plan(&fixture.store).unwrap();

    assert!(tree.exists());
    assert_eq!(
        planned.lines_with(safe, true),
        format!("{line}sweep: would free 0 bytes\n")
    );
    let (swept, _) = pass_one(&mut fixture.store);
    assert!(tree.exists());
    assert_eq!(swept.kept(), planned.kept());
    assert_eq!(
        swept.lines_with(safe, false),
        format!("{line}sweep: freed 0 bytes\n")
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_dry_run_lists_what_a_pass_would_remove_without_removing_it() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    fixture.succeeded(RUN2, ATTEMPT2);
    completed(&mut fixture.store, RUN2);
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
    assert_eq!(record_refusal(&refused).removed()[0].path, tree);
    let SweepError::Record { cause, .. } = &refused else {
        unreachable!("record_refusal checked the variant");
    };
    assert_eq!(cause, UNRECORDED);
    assert_eq!(
        refused.to_string(),
        "a tree was removed but its sweep could not be recorded: the journal's queue is not one \
         this binary writes tree sweeps in"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn the_lines_name_every_tree_and_the_total() {
    let swept = Swept {
        removed: vec![
            Removed {
                path: PathBuf::from("/run/scratch/one"),
                bytes: Some(10),
                lower_bound: false,
            },
            Removed {
                path: PathBuf::from("/tmp/hands\x1b[2J"),
                bytes: None,
                lower_bound: false,
            },
        ],
        kept: vec![Kept {
            path: PathBuf::from("/run/scratch/two"),
            reason: KeptReason::EffectOpen,
        }],
        failed: vec![Failed {
            path: PathBuf::from("/run/scratch/three"),
            cause: FailedCause::Walk("Permission denied".into()),
        }],
        hands: None,
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
    assert_eq!(swept.kept()[0].reason, KeptReason::RunCannotBeRead);
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
    assert_eq!(swept.kept()[0].reason, KeptReason::RunDoesNotFold);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
}

#[test]
fn a_row_the_checks_should_refuse_is_kept_by_its_shape() {
    if rustix::process::geteuid().is_root() {
        return;
    }
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
        kept_reasons(&swept),
        [
            KeptReason::Shape(ShapeReason::AttemptNotEngineUuid),
            KeptReason::Shape(ShapeReason::RootNotAbsolute),
            KeptReason::Shape(ShapeReason::TreeNotDirectory),
            KeptReason::Shape(ShapeReason::TreeUnreadable),
        ]
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_plan_count_that_refuses_is_reported_and_removes_nothing() {
    let mut fixture = Fixture::new();
    let (tree, _) = fixture.closed(ATTEMPT);
    let empty = tempfile::tempdir().unwrap();

    let plan = plan_with(
        &fixture.store,
        Ok(empty.path().to_path_buf()),
        super::same_device,
        |_| {
            Err(TreeError::Io {
                path: PathBuf::from("/x"),
                cause: std::io::Error::from_raw_os_error(13),
            })
        },
    )
    .unwrap();

    assert!(tree.exists());
    assert_eq!(plan.removed(), &[]);
    assert_eq!(
        plan.failed()[0].cause.to_string(),
        "Permission denied (os error 13)"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

#[test]
fn a_plan_that_cannot_read_every_name_says_at_least() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);

    let plan = plan_with(
        &fixture.store,
        Ok(fixture.tmp.clone()),
        super::same_device,
        |_| {
            Ok(Some(tree::Counted {
                bytes: 7,
                unreadable: Some(PathBuf::from("/x/sealed")),
            }))
        },
    )
    .unwrap();

    assert_eq!(plan.removed()[0].bytes, Some(7));
    assert!(plan.removed()[0].lower_bound);
    assert_eq!(
        plan.lines_with(safe, true),
        format!(
            "sweep: would remove {}: at least 7 bytes\nsweep: would free at least 7 bytes\n",
            tree.display()
        )
    );
    assert!(tree.exists());
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
            Listed {
                tree: PathBuf::from("/tmp/brokkr-hands-part"),
                count: Ok(Some(tree::Counted {
                    bytes: 3,
                    unreadable: Some(PathBuf::from("/tmp/brokkr-hands-part/sealed")),
                })),
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
    assert!(!swept.removed()[0].lower_bound);
    assert_eq!(swept.removed()[1].bytes, Some(3));
    assert!(swept.removed()[1].lower_bound);
    assert_eq!(
        swept.failed()[0].cause.to_string(),
        "Permission denied (os error 13)"
    );
}

/// F4 of #430's 5c2 reviews: a walk refusal's line carries
/// `TreeError::cause`'s own words, the one home brokkr-protocol keeps,
/// and names no path the refusal carries. The line is compared with
/// `error.cause()` before the words are pinned, so a restatement in sweep
/// fails here as soon as the one home's words move.
#[test]
fn a_walk_refusals_line_is_the_trees_own_cause() {
    let io = TreeError::Io {
        path: PathBuf::from("/tree/sealed"),
        cause: std::io::Error::from_raw_os_error(13),
    };
    let foreign = TreeError::Foreign {
        path: PathBuf::from("/tree/mounted"),
        device: 2,
        tree: 1,
    };
    let words = (io.cause(), foreign.cause());
    let mut swept = Swept::default();
    let listed = |tree: &str, error| Listed {
        tree: PathBuf::from(tree),
        count: Err(error),
    };
    plan_hands_with(
        [
            listed("/tmp/brokkr-hands-io", io),
            listed("/tmp/brokkr-hands-foreign", foreign),
        ]
        .into_iter(),
        &mut swept,
    );

    assert_eq!(
        swept.lines_with(safe, true),
        format!(
            "sweep: could not remove /tmp/brokkr-hands-io: {}\nsweep: could not remove \
             /tmp/brokkr-hands-foreign: {}\nsweep: would free 0 bytes\n",
            words.0, words.1
        )
    );
    assert_eq!(words.0, "Permission denied (os error 13)");
    assert_eq!(
        words.1,
        "is on another filesystem (device 2, not 1); a tree is removed on its own filesystem"
    );
}

/// The chief's case: a running run whose attempt has a live child in its
/// tree, closed by `conclude`. The terminal is a conclude's, so the tree
/// stands, the report names the conclude, and no sweep row is written.
#[test]
fn a_conclude_closing_a_live_attempt_keeps_its_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);
    let mut child = Command::new("sleep")
        .arg("30")
        .current_dir(&tree)
        .spawn()
        .unwrap();
    let by = Store::open_read_only(fixture.store.path()).unwrap();
    let named = Named {
        cause: Cause {
            ending: Ending::OperatorStopped,
            by_realm: None,
            by_run: None,
            by_ref: None,
        },
        reason: "its engine died",
        operator: "operator",
    };
    crate::engine::conclude(&mut fixture.store, &by, RUN, &named).unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    let killed = child.kill();
    let reaped = child.wait();
    assert!(killed.is_ok(), "{killed:?}");
    assert!(reaped.is_ok(), "{reaped:?}");
    assert!(tree.exists());
    assert_eq!(kept_reason(&swept), &KeptReason::Concluded);
    assert_eq!(
        swept.kept()[0].reason(),
        "its attempt was closed under an accepted stop"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A restarted engine's broker tallies append the attempt's checkpoints
/// before `ended_restarted` ends it as failed; a checkpointless one is
/// caused by `effect/started`. Neither typed fact makes it go: its
/// terminal event is `effect/failed`, so the tree stands and is named.
#[test]
fn a_restarted_engines_failed_attempt_keeps_its_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    let checkpoint = checkpoint(&mut fixture.store, RUN, ATTEMPT);
    append_caused(
        &mut fixture.store,
        RUN,
        EventType::EffectFailed,
        json!({"effect_id": EFFECT, "error": "a broker call failed"}),
        Some(ATTEMPT),
        Some(&checkpoint.event_id),
    );
    completed(&mut fixture.store, RUN);
    let tree = fixture.here(RUN, ATTEMPT);

    kept(&mut fixture, &tree, KeptReason::Failed);
}

/// The adversarial case: a driver's own `effect/failed`, caused by its
/// `effect/started`, on a completed run. It is not a conclude and not a
/// checkpoint, and it is not a succeeded attempt: the tree stands, named
/// by its failure, and no sweep row is written.
#[test]
fn an_effect_failed_caused_by_effect_started_keeps_its_tree() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    let begun = started(&mut fixture.store, RUN, ATTEMPT);
    append_caused(
        &mut fixture.store,
        RUN,
        EventType::EffectFailed,
        json!({"effect_id": EFFECT, "error": "the driver failed"}),
        Some(ATTEMPT),
        Some(&begun.event_id),
    );
    completed(&mut fixture.store, RUN);
    let tree = fixture.here(RUN, ATTEMPT);

    kept_saying(
        &mut fixture,
        &tree,
        KeptReason::Failed,
        "its attempt ended failed",
    );
}

/// Two attempts of one effect: A1 ended failed, A2 succeeded. A2's tree
/// goes and A1's stays, with its reason.
#[test]
fn two_attempts_of_one_effect_sweep_only_the_succeeded_one() {
    let mut fixture = Fixture::new();
    failed_then_succeeded(&mut fixture);
    let root = fixture.root.clone();
    let first = fixture.seated(RUN, ATTEMPT, &root);
    let second = fixture.seated(RUN, ATTEMPT2, &root);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(first.exists());
    assert!(!second.exists());
    assert_eq!(swept.removed().len(), 1);
    assert_eq!(swept.removed()[0].path, second);
    assert_eq!(kept_reason(&swept), &KeptReason::Failed);
    assert_eq!(swept.kept()[0].path, first);
}

/// A driver's own `effect/succeeded` in a closed effect goes with its
/// bytes, even after a checkpoint: no typed fact makes a succeeded
/// attempt one a restart wrote.
#[test]
fn a_driver_written_succeeded_in_a_closed_effect_goes_with_its_bytes() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    checkpoint(&mut fixture.store, RUN, ATTEMPT);
    succeeded(&mut fixture.store, RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);
    let freed = bytes(&tree);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(!tree.exists());
    assert_eq!(swept.removed().len(), 1);
    assert_eq!(swept.removed()[0].bytes, Some(freed));
    assert!(swept.kept().is_empty());
    assert_eq!(fixture.store.tree_sweeps().unwrap().len(), 1);
}

/// Every indeterminate terminal keeps its tree, with or without the
/// cleanup evidence ruling 7 used to read.
#[test]
fn an_indeterminate_keeps_its_tree_with_and_without_evidence() {
    let mut fixture = Fixture::new();
    let mut trees = Vec::new();
    for (run, attempt, evidence) in [(RUN, ATTEMPT, json!({})), (RUN2, ATTEMPT2, seat_evidence())] {
        fixture.opened(run);
        started(&mut fixture.store, run, attempt);
        indeterminate(&mut fixture.store, run, attempt, evidence);
        stopped(&mut fixture.store, run);
        trees.push(fixture.here(run, attempt));
    }

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(trees.iter().all(|tree| tree.exists()));
    assert_eq!(swept.kept().len(), 2);
    for kept in swept.kept() {
        assert_eq!(kept.reason, KeptReason::Indeterminate);
    }
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A succeeded event the fold read names its effect, so an effect id the
/// sweep could not read never closes a row: the invariant is the fold's.
#[test]
fn a_succeeded_event_names_the_effect_the_fold_read() {
    let mut fixture = Fixture::new();
    fixture.opened(RUN);
    started(&mut fixture.store, RUN, ATTEMPT);
    succeeded(&mut fixture.store, RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(!tree.exists());
    assert_eq!(swept.removed().len(), 1);
}

/// A relative root is refused before the canonical check: the first half
/// of R5's absolute-and-scratch spelling.
#[test]
fn a_relative_root_ending_in_scratch_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    fixture
        .store
        .create_run("relative", "a feature", "self", &json!({}))
        .unwrap();
    append(
        &mut fixture.store,
        "relative",
        EventType::RunStarted,
        json!({"feature": "a feature", "manifest": {}}),
        None,
    );
    let conn = rusqlite::Connection::open(fixture.store.path()).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    conn.execute(
        "INSERT INTO seat_trees (run_id, effect_id, attempt_id, root, recorded_at)
         VALUES ('relative', 'effect', ?1, 'relative/scratch', 0)",
        rusqlite::params![ATTEMPT],
    )
    .unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    assert_eq!(
        kept_reasons(&swept),
        [KeptReason::Shape(ShapeReason::RootNotAbsolute)]
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A root that is itself gone is kept and reported, not read as an absent
/// tree: R5 checks a root that exists. Recreating the empty root lets the
/// next pass close the row as gone.
#[test]
fn a_gone_root_keeps_its_row_until_the_root_is_recreated() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    let tree = fixture.here(RUN, ATTEMPT);
    std::fs::remove_dir_all(&fixture.root).unwrap();

    let (swept, _) = pass_one(&mut fixture.store);

    kept_root_gone(&swept);
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);

    std::fs::create_dir_all(&fixture.root).unwrap();
    let (swept, _) = pass_one(&mut fixture.store);

    assert!(!tree.exists());
    assert_eq!(swept.removed()[0].bytes, None);
    assert_eq!(swept.freed(), 0);
    let rows = fixture.store.tree_sweeps().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].bytes, None);
    assert_eq!(fixture.store.open_seat_trees().unwrap(), vec![]);
}

/// A `TMPDIR` that is relative or not UTF-8 refuses the hands half before
/// it reads a tree, and says why. A good `TMPDIR` is tested by every hands
/// case above.
#[test]
fn a_relative_or_non_utf8_tmpdir_refuses_the_hands_half() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    let cases: [(OsString, KeptReason); 2] = [
        (OsString::from("relative/tmp"), KeptReason::TmpdirRelative),
        (
            OsString::from_vec(b"/tmp/brokkr-not-utf8-\xff".to_vec()),
            KeptReason::TmpdirNotUtf8,
        ),
    ];
    for (value, reason) in cases {
        let mut env = EnvGuard::lock();
        env.set("TMPDIR", &value);
        let plan = plan(&store).unwrap();
        assert_eq!(plan.hands, Some(reason.clone()));
        assert!(plan.removed().is_empty());
        assert!(plan.kept().is_empty());
        let said = plan.lines_with(safe, true);
        assert!(said.contains("sweep: hands trees kept: "), "{said}");
        if matches!(reason, KeptReason::TmpdirRelative) {
            let held = crate::dispatch::take_dispatch(&mut store).unwrap().unwrap();
            let swept = pass(&mut store, &held).unwrap();
            assert_eq!(swept.hands, Some(KeptReason::TmpdirRelative));
            assert!(swept.removed().is_empty());
        }
        drop(env);
    }
    let mut env = EnvGuard::lock();
    env.remove("TMPDIR");
    let plan = plan(&store).unwrap();
    assert_eq!(plan.hands, None);
    drop(env);
}

/// A record refusal after a complete removal carries the report of every
/// removal made, names the tree's record failure beside it, and is an
/// error so the verb exits non-zero.
#[test]
fn a_record_refusal_carries_every_removal_it_made() {
    let mut fixture = Fixture::new();
    let (tree, freed) = fixture.closed(ATTEMPT);
    refuse_records(&fixture.store);
    let refused = pass_refusal(&mut fixture);

    assert!(!tree.exists());
    assert_eq!(record_refusal(&refused).removed()[0].bytes, Some(freed));
    let SweepError::Record { cause, .. } = &refused else {
        unreachable!("record_refusal checked the variant");
    };
    assert!(cause.contains("forced record failure"), "{cause}");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A refused record stops the removals still ahead: a tree may only go when
/// a row can record it. The removal already made stays in the report, and
/// the hands reaper does not start, because it removes every dead tree
/// before the pass can record one.
#[test]
fn a_record_refusal_stops_later_removals_and_the_reaper() {
    let mut fixture = Fixture::new();
    let (first, freed) = fixture.closed(ATTEMPT);
    fixture.succeeded(RUN2, ATTEMPT2);
    let second = fixture.here(RUN2, ATTEMPT2);
    let hands = hands_tree(&fixture.tmp, exited_pid(), "after-refusal");
    std::fs::write(hands.join(".owner.lock"), "").unwrap();
    refuse_records(&fixture.store);

    let refused = pass_refusal(&mut fixture);

    assert!(!first.exists());
    assert!(second.exists(), "a refusal stops a later scratch removal");
    assert!(
        hands.exists(),
        "a refusal keeps the reaper from removing hands trees"
    );
    let SweepError::Record { report, .. } = &refused else {
        panic!("a record refusal carries its report: {refused:?}");
    };
    assert_eq!(report.removed().len(), 1);
    assert_eq!(report.removed()[0].bytes, Some(freed));
    assert_eq!(report.failed().len(), 1);
    assert!(matches!(report.failed()[0].cause, FailedCause::Record(_)));
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// Every kept reason has words, so the report's match arms all run and
/// none of the vocabulary is silent.
#[test]
fn every_kept_reason_has_words() {
    for reason in [
        KeptReason::Indeterminate,
        KeptReason::Failed,
        KeptReason::Concluded,
        KeptReason::EffectOpen,
        KeptReason::NoEventNamesAttempt,
        KeptReason::NoTerminalEvent,
        KeptReason::Shape(ShapeReason::AnotherRoot),
        KeptReason::Shape(ShapeReason::RootNotAbsolute),
        KeptReason::Shape(ShapeReason::AttemptNotEngineUuid),
        KeptReason::Shape(ShapeReason::RootNotCanonical),
        KeptReason::Shape(ShapeReason::RootGone),
        KeptReason::Shape(ShapeReason::RootUnreadable),
        KeptReason::Shape(ShapeReason::TreeIsLink),
        KeptReason::Shape(ShapeReason::TreeNotDirectory),
        KeptReason::Shape(ShapeReason::TreeUnreadable),
        KeptReason::Shape(ShapeReason::ForeignFilesystem),
        KeptReason::RunCannotBeRead,
        KeptReason::RunDoesNotFold,
        KeptReason::TmpdirRelative,
        KeptReason::TmpdirNotUtf8,
        KeptReason::Held(HeldReason::RunNotEnded),
        KeptReason::Held(HeldReason::LateEvent),
        KeptReason::Held(HeldReason::Vanished),
        KeptReason::Held(HeldReason::NotReached),
        KeptReason::Hands("the reaper's words".to_string()),
    ] {
        assert!(!reason.to_string().is_empty(), "{reason:?}");
    }
}

/// A hands removal whose sweep row is refused keeps its removal in the
/// report beside the record refusal, and the report says so.
#[test]
fn a_hands_record_refusal_carries_its_removal() {
    let mut fixture = Fixture::new();
    let tree = hands_tree(&fixture.tmp, exited_pid(), "record");
    std::fs::write(tree.join(".owner.lock"), "").unwrap();
    refuse_records(&fixture.store);

    let refused = pass_refusal(&mut fixture);

    assert!(!tree.exists());
    let report = record_refusal(&refused);
    assert_eq!(report.removed()[0].path, tree);
    let said = report.lines_with(safe, false);
    assert!(said.contains("sweep: could not record"), "{said}");
}

/// Two dead hands trees whose sweep rows both refuse: both removals stay
/// in the report beside their record refusals, and the pass is an error.
/// A refusal never drops a later removal.
#[test]
fn a_hands_record_refusal_reports_every_removal() {
    let mut fixture = Fixture::new();
    let first = hands_tree(&fixture.tmp, exited_pid(), "record-a");
    let second = hands_tree(&fixture.tmp, exited_pid(), "record-b");
    std::fs::write(first.join(".owner.lock"), "").unwrap();
    std::fs::write(second.join(".owner.lock"), "").unwrap();
    refuse_records(&fixture.store);

    let refused = pass_refusal(&mut fixture);

    assert!(!first.exists());
    assert!(!second.exists());
    let SweepError::Record { report, cause } = &refused else {
        panic!("a record refusal carries its report: {refused:?}");
    };
    assert_eq!(report.removed().len(), 2);
    assert_eq!(report.failed().len(), 2);
    let paths: Vec<&Path> = report
        .removed()
        .iter()
        .map(|removed| removed.path.as_path())
        .collect();
    assert!(paths.contains(&first.as_path()));
    assert!(paths.contains(&second.as_path()));
    for failed in report.failed() {
        assert!(matches!(failed.cause, FailedCause::Record(_)));
        assert!(
            failed.cause.to_string().contains("forced record failure"),
            "{failed:?}"
        );
    }
    assert!(cause.contains("forced record failure"), "{cause}");
}

/// A root that cannot be resolved for a reason other than absence keeps
/// its row: a symlink loop is not a gone directory.
#[test]
fn a_root_that_cannot_be_canonicalized_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let looped = fixture.tmp.join("loop");
    std::os::unix::fs::symlink(&looped, &looped).unwrap();
    let root = looped.join("scratch");
    let conn = rusqlite::Connection::open(fixture.store.path()).unwrap();
    conn.execute_batch("PRAGMA ignore_check_constraints = ON")
        .unwrap();
    conn.execute(
        "INSERT INTO seat_trees (run_id, effect_id, attempt_id, root, recorded_at)
         VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![RUN, EFFECT, ATTEMPT, root.to_str().unwrap()],
    )
    .unwrap();

    let reason = kept_after_pass(&mut fixture);

    assert_eq!(reason, KeptReason::Shape(ShapeReason::RootNotCanonical));
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A hands tree named the way the reaper parses it.
fn hands_tree(tmp: &Path, pid: u32, label: &str) -> PathBuf {
    let tree = tmp.join(format!("brokkr-hands-{label}-{pid}-{}", Uuid::new_v4()));
    std::fs::create_dir_all(tree.join("session")).unwrap();
    std::fs::write(tree.join("session").join("data"), b"x").unwrap();
    tree
}

/// The reaped-pid fixture is bounded: a child that never exits is killed,
/// and the fixture fails saying so within PATIENCE rather than hang.
#[test]
fn the_reaped_pid_fixture_fails_within_its_patience_on_a_child_that_never_exits() {
    let started = std::time::Instant::now();
    let failed = std::panic::catch_unwind(|| {
        crate::reaped::reaped_within(
            Command::new("sleep").arg("600"),
            std::time::Duration::from_millis(200),
        )
    })
    .unwrap_err();

    assert!(started.elapsed() < crate::reaped::PATIENCE);
    let words = failed.downcast_ref::<String>().unwrap();
    assert!(
        words.ends_with("did not exit within 200ms; it was killed"),
        "{words}"
    );
}

/// A safe renderer for the lines: the CLI's own strips controls and the
/// bidi marks, and this pins the same visible result for a control byte.
fn safe(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control() && *character != '\u{202e}')
        .collect()
}
