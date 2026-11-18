//! The gone-tree closes (decision 0104 ruling 8, as the operator's
//! 2026-11-15 ruling amends it): a tree that is absent at the verdict is
//! never walked, so whatever stands at its path when a later row's walk
//! runs is left alone. A gone row closes only once its root exists and R5
//! holds, its effect is closed, its attempt has a terminal event, and its
//! run has ended. A present tree is re-checked immediately before its walk.

use brokkr_core::envelope::EventType;
use serde_json::{json, Value};

use super::*;
use crate::envelope_builder::EnvelopeBuilder;

const ATTEMPT3: &str = "33333333-3333-4333-8333-333333333333";
const RUN3: &str = "run-sweep-three";

/// Plant `events` after `run`'s head, chained and sealed as the store seals
/// them, so a late tail an older engine wrote can be read. The store now
/// refuses these appends (#1078), so they are planted behind it.
fn plant_after_terminal(
    store: &Store,
    run: &str,
    events: &[(EventType, Value, Option<&str>, &str)],
) {
    let head = store.load(run).unwrap().pop().unwrap();
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    let mut previous = head.event_hash;
    let mut seq = head.seq;
    for (event_type, payload, attempt, event_id) in events {
        seq += 1;
        let envelope = EnvelopeBuilder::new(*event_type, payload.clone())
            .run(run)
            .seq(seq)
            .event_id(*event_id)
            .attempt(*attempt)
            .previous(previous)
            .sealed();
        brokkr_store::test_support::plant_envelope(&connection, &envelope).unwrap();
        previous = envelope.event_hash;
    }
}

/// The repair record that names the late tail after `terminal`, one seq
/// per type in `types`.
fn repair(terminal: u64, types: &[&str]) -> Value {
    let last = terminal + types.len() as u64;
    json!({
        "command_id": "c-repair",
        "command": "repair",
        "operator": "operator",
        "args": {
            "first_seq": terminal + 1,
            "last_seq": last,
            "terminal_seq": terminal,
            "event_types": types,
            "reason": "kept",
        },
    })
}

/// A fixture with a succeeded attempt's present tree, and the tree's path.
fn first_present() -> (Fixture, PathBuf) {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);
    (fixture, tree)
}

/// Walk `first`, planting `second`'s tree under `root` when the walk reaches
/// the first row, then hand back the report.
fn walk_planting(fixture: &mut Fixture, first: &Path, second: &str, root: &Path) -> Swept {
    let first = first.to_path_buf();
    let root = root.to_path_buf();
    walk(fixture, super::same_device, |path| {
        if path == first.as_path() {
            plant(&root, second);
        }
        tree::remove_tree(path)
    })
}

/// The chief's case (a): a failed attempt on a completed run whose tree is
/// gone at the verdict. A first row's walk plants the gone row's path; the
/// gone row closes with no walk, so the planted tree and its bytes stand.
#[test]
fn a_gone_tree_planted_after_its_verdict_is_never_walked() {
    let (mut fixture, first) = first_present();
    failed_on_completed(&mut fixture.store, RUN2, ATTEMPT2);
    seat(&mut fixture.store, RUN2, EFFECT, ATTEMPT2, &fixture.root);
    let second = fixture.root.join(ATTEMPT2);
    assert!(!second.exists());
    let root = fixture.root.clone();

    let swept = walk_planting(&mut fixture, &first, ATTEMPT2, &root);

    assert!(second.exists(), "the planted tree stands");
    let bytes = tree::count_tree(&second).unwrap().unwrap().bytes;
    assert!(bytes > 0);
    assert_eq!(swept.removed().len(), 2);
    assert_eq!(swept.removed()[1].path, second);
    assert_eq!(swept.removed()[1].bytes, None);
    let sweeps = fixture.store.tree_sweeps().unwrap();
    assert_eq!(sweeps.len(), 2);
    assert_eq!(sweeps[1].path, second);
    assert_eq!(sweeps[1].bytes, None);
    assert_eq!(fixture.store.open_seat_trees().unwrap(), vec![]);
}

/// The chief's case (b): the same gone tree on a live run whose effect is
/// open. The row is kept as "its effect is still open", no row closes, and
/// the tree a first walk plants stands.
#[test]
fn a_gone_tree_of_an_open_effect_is_kept_and_the_planted_tree_stands() {
    let (mut fixture, first) = first_present();
    fixture.opened(RUN2);
    started(&mut fixture.store, RUN2, ATTEMPT2);
    failed(&mut fixture.store, RUN2, ATTEMPT2);
    seat(&mut fixture.store, RUN2, EFFECT, ATTEMPT2, &fixture.root);
    let second = fixture.root.join(ATTEMPT2);
    let root = fixture.root.clone();

    let swept = walk_planting(&mut fixture, &first, ATTEMPT2, &root);

    assert!(second.exists(), "the planted tree stands");
    assert_eq!(swept.removed().len(), 1);
    assert_eq!(kept_reason(&swept), &KeptReason::EffectOpen);
    assert_eq!(swept.kept()[0].reason(), "its effect is still open");
    assert_eq!(fixture.store.tree_sweeps().unwrap().len(), 1);
    let open = fixture.store.open_seat_trees().unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].attempt_id, ATTEMPT2);
}

/// The chief's case (c): a row whose root is under a parent gone at the
/// verdict. A first walk plants a link at that parent to a tree elsewhere;
/// the gone root is kept with its own reason, nothing outside the root is
/// touched, and no row closes.
#[test]
fn a_gone_root_is_kept_and_a_link_planted_at_its_parent_is_never_followed() {
    let (mut fixture, first) = first_present();
    failed_on_completed(&mut fixture.store, RUN2, ATTEMPT2);
    let parent = fixture.tmp.join("gone");
    let root = parent.join("scratch");
    seat(&mut fixture.store, RUN2, EFFECT, ATTEMPT2, &root);
    let elsewhere = fixture.tmp.join("elsewhere");
    let target = plant(&scratch(&elsewhere), ATTEMPT2);
    let target_bytes = tree::count_tree(&target).unwrap().unwrap().bytes;

    let swept = walk(&mut fixture, super::same_device, |path| {
        if path == first.as_path() {
            std::os::unix::fs::symlink(&elsewhere, &parent).unwrap();
        }
        tree::remove_tree(path)
    });

    assert!(target.exists(), "nothing outside the root is touched");
    assert_eq!(
        tree::count_tree(&target).unwrap().unwrap().bytes,
        target_bytes
    );
    assert_eq!(swept.removed().len(), 1);
    kept_root_gone(&swept);
    assert_eq!(fixture.store.tree_sweeps().unwrap().len(), 1);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

/// A succeeded attempt's gone tree on a live run whose effect succeeded is
/// kept: a gone row closes only once its run has ended.
#[test]
fn a_gone_tree_of_a_run_that_has_not_ended_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);

    let (swept, _) = pass_one(&mut fixture.store);

    assert_eq!(
        kept_reason(&swept),
        &KeptReason::Held(HeldReason::RunNotEnded)
    );
    assert_eq!(swept.kept()[0].reason(), "its run has not ended");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

/// A succeeded attempt's gone tree on a completed run closes as gone with
/// no bytes, and the walk is never called.
#[test]
fn a_gone_tree_of_an_ended_run_closes_without_a_walk() {
    let mut fixture = Fixture::new();
    gone_succeeded_on_completed(&mut fixture);

    let swept = walk(&mut fixture, super::same_device, |_| {
        panic!("a gone tree is never walked")
    });

    assert_eq!(swept.removed().len(), 1);
    assert_eq!(swept.removed()[0].bytes, None);
    closed_gone(&fixture, &swept);
}

/// A present tree that goes between its verdict and its walk is kept, not
/// walked: the re-check narrows the window to one stat.
#[test]
fn a_tree_that_goes_between_the_verdict_and_the_walk_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    fixture.succeeded(RUN2, ATTEMPT2);
    let first = fixture.here(RUN, ATTEMPT);
    let second = fixture.here(RUN2, ATTEMPT2);

    let swept = walk(&mut fixture, super::same_device, |path| {
        if path == first.as_path() {
            std::fs::remove_dir_all(&second).unwrap();
            tree::remove_tree(path)
        } else {
            panic!("a tree that went is never walked");
        }
    });

    assert!(!second.exists());
    assert_eq!(swept.removed().len(), 1);
    assert_eq!(kept_reason(&swept), &KeptReason::Held(HeldReason::Vanished));
    assert_eq!(swept.kept()[0].reason(), "its tree went before the walk");
    assert_eq!(fixture.store.tree_sweeps().unwrap().len(), 1);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

/// A present tree that becomes a link between its verdict and its walk is
/// kept with the shape's own reason; the link's target is never touched.
#[test]
fn a_tree_that_becomes_a_link_before_the_walk_is_kept() {
    let (mut fixture, first) = first_present();
    fixture.succeeded(RUN2, ATTEMPT2);
    let second = fixture.here(RUN2, ATTEMPT2);
    let elsewhere = fixture.tmp.join("elsewhere");
    let target = scratch(&elsewhere).join("target");
    std::fs::create_dir_all(&target).unwrap();
    let second_path = second.clone();

    let swept = walk(&mut fixture, super::same_device, |path| {
        if path == first.as_path() {
            std::fs::remove_dir_all(&second_path).unwrap();
            std::os::unix::fs::symlink(&target, &second_path).unwrap();
            tree::remove_tree(path)
        } else {
            panic!("a tree that changed shape is never walked")
        }
    });

    assert!(target.exists(), "the link's target is never touched");
    assert_eq!(
        kept_reason(&swept),
        &KeptReason::Shape(ShapeReason::TreeIsLink)
    );
    assert_eq!(
        swept.kept()[0].reason(),
        "its tree is a link, not a directory"
    );
    assert_eq!(swept.removed().len(), 1);
    assert_eq!(fixture.store.tree_sweeps().unwrap().len(), 1);
}

/// After a record refusal, every later verdict is still reported: a kept
/// row keeps its own reason, and a ready row is reported as not reached.
/// Neither is removed or closed.
#[test]
fn a_record_refusal_reports_every_later_verdict() {
    let mut fixture = Fixture::new();
    let (first, _) = fixture.closed(ATTEMPT);
    failed_on_completed(&mut fixture.store, RUN2, ATTEMPT2);
    let kept = fixture.here(RUN2, ATTEMPT2);
    fixture.succeeded(RUN3, ATTEMPT3);
    let ready = fixture.here(RUN3, ATTEMPT3);
    refuse_records(&fixture.store);

    let refused = pass_refusal(&mut fixture);

    assert!(!first.exists());
    assert!(kept.exists());
    assert!(ready.exists());
    let SweepError::Record { report, .. } = &refused else {
        panic!("a record refusal carries its report: {refused:?}");
    };
    assert_eq!(report.removed().len(), 1);
    assert_eq!(report.failed().len(), 1);
    assert_eq!(
        kept_reasons(report),
        [KeptReason::Failed, KeptReason::Held(HeldReason::NotReached),]
    );
    assert_eq!(
        report.kept()[1].reason(),
        "not reached: a record was refused"
    );
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 3);
}

/// A late event names an attempt that otherwise ended failed. The fold
/// never applies it, so it neither ends the attempt nor closes the effect,
/// and the present tree is kept as the late reason (R-1, #1085). A second
/// attempt named only by a late event is kept too: reading all events would
/// see it succeeded and remove its tree, which the control records.
#[test]
fn a_late_event_naming_an_attempt_keeps_its_tree() {
    let mut fixture = Fixture::new();
    opened_failed(&mut fixture.store, RUN, ATTEMPT);
    started(&mut fixture.store, RUN, ATTEMPT2);
    completed(&mut fixture.store, RUN);
    let terminal = fixture.store.load(RUN).unwrap().last().unwrap().seq;
    let failed_attempt = fixture.here(RUN, ATTEMPT);
    let late_only = fixture.here(RUN, ATTEMPT2);
    let late = |attempt: &str| {
        json!({
            "effect_id": EFFECT,
            "attempt_id": attempt,
            "result": {"result": "ok"},
        })
    };
    plant_after_terminal(
        &fixture.store,
        RUN,
        &[
            (
                EventType::EffectSucceeded,
                late(ATTEMPT),
                Some(ATTEMPT),
                "late-succeeded-failed",
            ),
            (
                EventType::EffectSucceeded,
                late(ATTEMPT2),
                Some(ATTEMPT2),
                "late-succeeded-late-only",
            ),
            (
                EventType::OperatorCommanded,
                repair(terminal, &["effect/succeeded"; 2]),
                None,
                "repair",
            ),
        ],
    );

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(failed_attempt.exists());
    assert!(late_only.exists());
    assert_eq!(
        kept_reasons(&swept),
        [
            KeptReason::Held(HeldReason::LateEvent),
            KeptReason::Held(HeldReason::LateEvent),
        ]
    );
    assert_eq!(swept.kept()[0].reason(), "a late event names its attempt");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
}

/// A late event names a failed attempt whose tree is gone, on a completed
/// run. Nothing stands at its path to remove, so the row closes as gone with
/// no walk and no bytes, like any other gone row (the operator's 2026-11-16
/// ruling): the late check is a standing tree's.
#[test]
fn a_gone_tree_a_late_event_names_closes_as_gone() {
    let mut fixture = Fixture::new();
    opened_failed(&mut fixture.store, RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT, &fixture.root);
    let terminal = fixture.store.load(RUN).unwrap().last().unwrap().seq;
    let late = json!({
        "effect_id": EFFECT,
        "attempt_id": ATTEMPT,
        "result": {"result": "ok"},
    });
    plant_after_terminal(
        &fixture.store,
        RUN,
        &[
            (
                EventType::EffectSucceeded,
                late,
                Some(ATTEMPT),
                "late-succeeded-gone",
            ),
            (
                EventType::OperatorCommanded,
                repair(terminal, &["effect/succeeded"]),
                None,
                "repair",
            ),
        ],
    );
    let tree = fixture.root.join(ATTEMPT);
    assert!(!tree.exists());

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(swept.kept().is_empty());
    assert_eq!(removed(&swept).path, tree);
    assert_eq!(removed(&swept).bytes, None);
    let sweeps = fixture.store.tree_sweeps().unwrap();
    assert_eq!(sweeps.len(), 1);
    assert_eq!(sweeps[0].bytes, None);
    assert_eq!(fixture.store.open_seat_trees().unwrap(), vec![]);
    brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
}

/// A dry run lists a gone row as the pass would close it, with no bytes,
/// and never counts it: a tree planted at its path after the verdict is
/// never reported with bytes for a row a pass would never walk (ruling 9).
#[test]
fn a_dry_run_never_counts_a_gone_row() {
    let mut fixture = Fixture::new();
    gone_succeeded_on_completed(&mut fixture);
    let empty = tempfile::tempdir().unwrap();

    let plan = plan_with(
        &fixture.store,
        Ok(empty.path().to_path_buf()),
        super::same_device,
        |_| panic!("a gone row is never counted"),
    )
    .unwrap();

    assert_eq!(plan.removed().len(), 1);
    assert_eq!(plan.removed()[0].bytes, None);
    assert!(plan.kept().is_empty());
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

/// A dry run lists a kept tree with the reason a pass would give, counts
/// nothing for it, and leaves it standing.
#[test]
fn a_dry_run_lists_a_kept_tree_with_its_reason() {
    let mut fixture = Fixture::new();
    failed_on_completed(&mut fixture.store, RUN, ATTEMPT);
    let tree = fixture.here(RUN, ATTEMPT);

    let plan = plan(&fixture.store).unwrap();

    assert!(tree.exists());
    assert!(plan.removed().is_empty());
    assert_eq!(kept_reason(&plan), &KeptReason::Failed);
    assert_eq!(plan.kept()[0].reason(), "its attempt ended failed");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
}

/// A gone row whose attempt no event names is kept on an ended run: only
/// an attempt the journal saw end can close as gone.
#[test]
fn a_gone_row_whose_attempt_no_event_names_is_kept() {
    let mut fixture = Fixture::new();
    fixture.succeeded(RUN, ATTEMPT);
    completed(&mut fixture.store, RUN);
    seat(&mut fixture.store, RUN, EFFECT, ATTEMPT2, &fixture.root);
    assert!(!fixture.root.join(ATTEMPT2).exists());

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(swept.removed().is_empty());
    assert_eq!(kept_reason(&swept), &KeptReason::NoEventNamesAttempt);
    assert_eq!(swept.kept()[0].reason(), "no event names its attempt");
    assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
    assert_eq!(fixture.store.open_seat_trees().unwrap().len(), 1);
}

/// F5 of #430's 5c2 reviews: an operator payload the sweep reads is
/// decoded where it enters, and one that does not decode keeps the run's
/// rows, reported and never a panic. A known command word after the
/// terminal event is admitted as an annotation the fold does not read, so
/// only the sweep's decode meets its numeric `command_id`; a read by
/// string key skipped it and removed the succeeded tree. An applied
/// acceptance's numeric `command_id` is the fold's own refusal, met before
/// the sweep's decode: the invariant that leaves that decode's refusal a
/// value no test enters, and its tree is kept the same. The 5c2 landing
/// review's finding: a late `operator/commanded` whose payload is an array
/// is no late word, so the fold admits it unread, and serde would read it
/// into the struct by position; only an object decodes.
#[test]
fn an_operator_payload_that_does_not_decode_keeps_the_runs_tree() {
    let positional = (EventType::OperatorCommanded, json!(["c1", "stop"]));
    for (event_type, payload) in malformed_operator_payloads()
        .into_iter()
        .chain([positional])
    {
        let (mut fixture, tree) = first_present();
        completed(&mut fixture.store, RUN);
        plant_after_terminal(
            &fixture.store,
            RUN,
            &[(event_type, payload, None, "operator-unread")],
        );

        let (swept, _) = pass_one(&mut fixture.store);

        assert!(tree.exists(), "{event_type:?}");
        assert_eq!(kept_reasons(&swept), [KeptReason::RunDoesNotFold]);
        assert_eq!(swept.kept()[0].reason(), "its run does not fold");
        assert_eq!(fixture.store.tree_sweeps().unwrap(), vec![]);
        brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
    }
}

/// An `operator/commanded` stop and an `operator/accepted` whose numeric
/// `command_id` the sweep's decode refuses.
fn malformed_operator_payloads() -> [(EventType, Value); 2] {
    [
        (
            EventType::OperatorCommanded,
            json!({"command_id": 7, "command": "stop", "operator": "operator", "args": {}}),
        ),
        (EventType::OperatorAccepted, json!({"command_id": 7})),
    ]
}

/// The 5c2 landing review's finding: the stop reading is the applied
/// events' only, as the rest of the derivation is. A repaired late tail
/// carrying operator payloads the sweep cannot decode is never applied, so
/// the gone row of a completed run still closes as gone (ruling 8); a
/// decode of the whole journal kept it as a run that does not fold.
#[test]
fn a_repaired_tail_of_undecodable_operator_payloads_still_closes_a_gone_row() {
    let mut fixture = Fixture::new();
    gone_succeeded_on_completed(&mut fixture);
    let terminal = fixture.store.load(RUN).unwrap().last().unwrap().seq;
    let [commanded, accepted] = malformed_operator_payloads();
    plant_after_terminal(
        &fixture.store,
        RUN,
        &[
            (commanded.0, commanded.1, None, "late-commanded"),
            (accepted.0, accepted.1, None, "late-accepted"),
            (
                EventType::OperatorCommanded,
                repair(terminal, &["operator/commanded", "operator/accepted"]),
                None,
                "repair",
            ),
        ],
    );

    let (swept, _) = pass_one(&mut fixture.store);

    assert!(swept.kept().is_empty());
    assert_eq!(removed(&swept).path, fixture.root.join(ATTEMPT));
    closed_gone(&fixture, &swept);
    brokkr_store::test_support::lags(&mut fixture.store, &[RUN]);
}
