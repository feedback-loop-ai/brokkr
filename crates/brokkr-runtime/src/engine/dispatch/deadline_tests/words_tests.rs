//! Decision 0104 ruling 3(a) for panels and sequences: a raised deadline,
//! or a host file that could not be read or was invalid, is named once in
//! the attempt's terminal words — at the panel's joined outcome, or at the
//! sequence step that ends the attempt. Each test drives the attempt end to
//! end, so the words it pins are the ones the journal carries.

use super::*;
use crate::bundle::{Aggregate, PanelMember, SequenceStep, StepBody};

/// The one fixture candidate `script` launches, built through the parent's
/// `link` so another lane may add a field to `Candidate` without moving
/// this file.
fn driven(script: Vec<String>) -> Candidate {
    let mut candidate = link("dsh", "glm-flash");
    candidate.argv = script;
    templated(candidate)
}

/// A work panel member `name` running `script`.
fn panel_member(name: &str, script: Vec<String>) -> PanelMember {
    PanelMember {
        name: name.into(),
        role_path: "role.md".into(),
        command: script.clone(),
        candidates: vec![driven(script)],
        written: Vec::new(),
        class: SeatClass::Work,
    }
}

/// Pin the member's candidate to the shared-local Spark route at `label`.
fn pin_member(engine: &mut Engine, label: &str, member: &PanelMember) {
    let candidate = member.candidates[0].clone();
    engine.bundle.sites.insert(
        label.into(),
        facts(vec![candidate], vec![served(Some("spark-glm"))]),
    );
}

/// The one `work` seat running a panel of `members`, under `timeout` and
/// the host file `host`.
fn panel_engine(
    dir: &Path,
    members: Vec<PanelMember>,
    timeout: u64,
    host: Option<PathBuf>,
) -> Engine {
    let body = SeatBody::Invocation(Invocation::Panel {
        members: members.clone(),
        aggregate: Aggregate::UnanimousPass,
    });
    let mut engine = started(
        dir,
        body,
        Limits {
            max_attempts: 1,
            timeout_seconds: timeout,
        },
    );
    engine.cooldown.host = host;
    for member in &members {
        pin_member(&mut engine, &format!("work:{}", member.name), member);
    }
    engine
}

/// The one `work` seat running a one-member panel under `timeout` and the
/// host file `host`.
fn solo_panel(dir: &Path, member: PanelMember, timeout: u64, host: Option<PathBuf>) -> Engine {
    panel_engine(dir, vec![member], timeout, host)
}

/// A one-member panel raised to two seconds by the host floor, with the
/// seat's own limit one.
fn raised_solo_panel(dir: &Path, member: PanelMember) -> Engine {
    solo_panel(dir, member, 1, Some(host(dir, Some(2))))
}

/// One sequence step `name` around `body`.
fn step(name: &str, body: StepBody) -> SequenceStep {
    uncompiled_sequence_step! {
        name: name.into(),
        class: SeatClass::Work,
        results: vec!["complete".into()],
        body,
    }
}

/// A single-driver step `name` running `script`, with the candidate its
/// site is pinned by.
fn single_step(name: &str, script: Vec<String>) -> (SequenceStep, Candidate) {
    let candidate = driven(script.clone());
    let body = StepBody::Invocation(Invocation::Single {
        role_path: "role.md".into(),
        command: script,
        candidates: vec![candidate.clone()],
        written: Vec::new(),
        class: (),
    });
    (step(name, body), candidate)
}

/// A panel step `name` whose members join unanimously.
fn panel_step(name: &str, members: Vec<PanelMember>) -> SequenceStep {
    step(
        name,
        StepBody::Invocation(Invocation::Panel {
            members,
            aggregate: Aggregate::UnanimousPass,
        }),
    )
}

/// The one `work` seat running `steps`, under `timeout` and the host file
/// `host`.
fn sequence_engine(
    dir: &Path,
    steps: Vec<SequenceStep>,
    timeout: u64,
    host: Option<PathBuf>,
) -> Engine {
    let mut engine = started(
        dir,
        SeatBody::Sequence { steps },
        Limits {
            max_attempts: 1,
            timeout_seconds: timeout,
        },
    );
    engine.cooldown.host = host;
    engine
}

/// Pin a step's own candidate at `work:<step>`.
fn pin_step(engine: &mut Engine, step: &str, candidate: &Candidate) {
    engine.bundle.sites.insert(
        format!("work:{step}"),
        facts(vec![candidate.clone()], vec![served(Some("spark-glm"))]),
    );
}

/// A driver that accepts the attempt and then exits without a result: an
/// indeterminate outcome the watchdog did NOT end.
fn exited_driver(dir: &Path) -> Vec<String> {
    driver(dir, "exited-driver.sh", "exit 0\n")
}

#[test]
fn a_panels_raised_deadline_names_the_floor_and_keeps_its_member_list() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = raised_solo_panel(dir.path(), panel_member("slow", silent_driver(dir.path())));
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "panel members [\"slow\"] could not establish completion: the seat's 2-second deadline: \
         the shared-local floor host.json declares for dsh/spark-glm; the seat's own limit is 1 \
         seconds"
    );
}

#[test]
fn a_sequences_raised_deadline_names_the_floor_on_its_ending_step() {
    let dir = tempfile::tempdir().unwrap();
    let (step, candidate) = single_step("step", silent_driver(dir.path()));
    let mut engine = sequence_engine(dir.path(), vec![step], 1, Some(host(dir.path(), Some(2))));
    pin_step(&mut engine, "step", &candidate);
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "sequence step 'step': the seat's 2-second deadline: the shared-local floor host.json \
         declares for dsh/spark-glm; the seat's own limit is 1 seconds; its process tree is gone, \
         and what it did is unknown; stderr tail: "
    );
}

#[test]
fn a_sequence_panel_steps_raised_deadline_names_the_source_once() {
    let dir = tempfile::tempdir().unwrap();
    let members = vec![panel_member("slow", silent_driver(dir.path()))];
    let mut engine = sequence_engine(
        dir.path(),
        vec![panel_step("check", members.clone())],
        1,
        Some(host(dir.path(), Some(2))),
    );
    for member in &members {
        pin_member(&mut engine, &format!("work:check:{}", member.name), member);
    }
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "sequence step 'check': panel members [\"slow\"] could not establish completion: the \
         seat's 2-second deadline: the shared-local floor host.json declares for dsh/spark-glm; \
         the seat's own limit is 1 seconds"
    );
    assert_eq!(reason.matches("shared-local floor").count(), 1, "{reason}");
}

#[test]
fn a_panels_unreadable_host_is_named_once_in_a_failing_panels_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = panel_engine(
        dir.path(),
        vec![
            panel_member("a", failed_driver(dir.path())),
            panel_member("b", failed_driver(dir.path())),
        ],
        1,
        Some(dir.path().to_path_buf()),
    );
    engine.drive().unwrap();
    let error = terminal(&engine, EventType::EffectFailed, "error");
    assert!(
        error.starts_with("panel members failed — a: driver failed; b: driver failed; "),
        "{error}"
    );
    let named = format!("the host configuration at {}", dir.path().display());
    assert_eq!(error.matches(&named).count(), 1, "{error}");
    assert!(error.contains("cannot be read"), "{error}");
}

#[test]
fn a_panels_unreadable_host_is_named_once_in_an_indeterminate_panels_words() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = solo_panel(
        dir.path(),
        panel_member("slow", silent_driver(dir.path())),
        1,
        Some(dir.path().to_path_buf()),
    );
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    let named = format!(
        "panel members [\"slow\"] could not establish completion; the host configuration at {} \
         cannot be read",
        dir.path().display()
    );
    assert!(reason.starts_with(&named), "{reason}");
    assert_eq!(
        reason
            .matches(&format!(
                "the host configuration at {}",
                dir.path().display()
            ))
            .count(),
        1,
        "{reason}"
    );
}

#[test]
fn a_sequences_unreadable_host_is_named_once_in_the_failing_steps_words() {
    let dir = tempfile::tempdir().unwrap();
    let (step, candidate) = single_step("step", failed_driver(dir.path()));
    let mut engine = sequence_engine(dir.path(), vec![step], 1, Some(dir.path().to_path_buf()));
    pin_step(&mut engine, "step", &candidate);
    engine.drive().unwrap();
    let error = terminal(&engine, EventType::EffectFailed, "error");
    assert!(
        error.starts_with("sequence step 'step': driver failed; "),
        "{error}"
    );
    let named = format!("the host configuration at {}", dir.path().display());
    assert_eq!(error.matches(&named).count(), 1, "{error}");
}

#[test]
fn an_unspawned_panel_member_under_an_invalid_host_names_the_host_once() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    std::fs::write(&file, "not json").unwrap();
    let mut engine = solo_panel(
        dir.path(),
        panel_member("doomed", vec!["brokkr-no-such-driver-430".into()]),
        1,
        Some(file.clone()),
    );
    engine.drive().unwrap();
    let error = terminal(&engine, EventType::EffectFailed, "error");
    assert!(
        error.starts_with("panel members failed — doomed: member driver did not spawn: "),
        "{error}"
    );
    assert_eq!(
        error
            .matches(&format!("the host configuration at {}", file.display()))
            .count(),
        1,
        "{error}"
    );
    assert!(
        error.contains("is not a valid host configuration file"),
        "{error}"
    );
}

#[test]
fn a_panel_that_was_not_deadline_killed_keeps_todays_words() {
    let dir = tempfile::tempdir().unwrap();
    // The floor raises the attempt to two seconds, but the member exits
    // before the watchdog: the joined reason is untouched.
    let mut engine = raised_solo_panel(dir.path(), panel_member("gone", exited_driver(dir.path())));
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "panel members [\"gone\"] could not establish completion"
    );

    // The joined wording names only an indeterminate outcome, even when a
    // caller says a member was killed.
    let deadline = Deadline {
        seconds: 2,
        cause: Some(Cause::Raised {
            provider: "dsh".into(),
            route: "spark-glm".into(),
            authored: 1,
        }),
    };
    let mut failed = AttemptOutcome::Failed {
        error: "panel members failed — gone: driver failed".into(),
    };
    deadline.name_joined_outcome(&mut failed, true);
    assert_eq!(
        failed,
        AttemptOutcome::Failed {
            error: "panel members failed — gone: driver failed".into()
        }
    );
}

#[test]
fn a_panel_with_the_seat_limit_governing_keeps_todays_words() {
    let dir = tempfile::tempdir().unwrap();
    // The floor is one second and the seat authored two: the seat's own
    // limit governs, so the protocol's member list is the whole reason.
    let mut engine = solo_panel(
        dir.path(),
        panel_member("slow", silent_driver(dir.path())),
        2,
        Some(host(dir.path(), Some(1))),
    );
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "panel members [\"slow\"] could not establish completion"
    );
}

#[test]
fn a_sequence_with_the_seat_limit_governing_keeps_todays_words() {
    let dir = tempfile::tempdir().unwrap();
    let (step, candidate) = single_step("step", silent_driver(dir.path()));
    let mut engine = sequence_engine(dir.path(), vec![step], 2, Some(host(dir.path(), Some(1))));
    pin_step(&mut engine, "step", &candidate);
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "sequence step 'step': attempt exceeded its 2s deadline and was killed; its process tree \
         is gone, and what it did is unknown; stderr tail: "
    );
}
