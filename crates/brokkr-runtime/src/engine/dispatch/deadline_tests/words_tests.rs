//! Decision 0104 ruling 3(a) for panels and sequences: a raised deadline,
//! or a host file that could not be read or was invalid, is named once in
//! the attempt's terminal words — at the panel's joined outcome, or in the
//! sequence words that end the attempt, whether a step's outcome or an
//! ending built without one. Each test drives the attempt end to end, so
//! the words it pins are the ones the journal carries.

use super::*;
use crate::bundle::{Aggregate, DialectExecution, PanelMember, SequenceStep, StepBody};
use crate::engine::carry::Panelled;
use std::os::unix::fs::PermissionsExt;

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

/// Drive one sequence step `step`, pinned to `candidate`, under `dir`'s
/// unreadable host, and return its `effect/failed` words.
fn step_error(dir: &Path, step: SequenceStep, candidate: &Candidate) -> String {
    let name = step.name.clone();
    let mut engine = sequence_engine(dir, vec![step], 1, Some(dir.to_path_buf()));
    pin_step(&mut engine, &name, candidate);
    engine.drive().unwrap();
    terminal(&engine, EventType::EffectFailed, "error")
}

/// The one `work` seat running one dialect step `validate` on `argv`,
/// under `timeout` and the host file `host`.
fn dialect_engine(dir: &Path, argv: Vec<String>, timeout: u64, host: Option<PathBuf>) -> Engine {
    sequence_engine(dir, vec![dialect_step("validate", argv)], timeout, host)
}

/// Drive one dialect step under `dir`'s unreadable host, with `engine_bin`
/// as the engine, and return the `field` of its first `event`.
fn dialect_words(
    dir: &Path,
    engine_bin: crate::argv::Executable,
    event: EventType,
    field: &str,
) -> String {
    let mut engine = dialect_engine(dir, vec!["true".into()], 1, Some(dir.to_path_buf()));
    engine.bundle.executable = engine_bin;
    engine.drive().unwrap();
    terminal(&engine, event, field)
}

/// A driver that accepts the attempt and then exits without a result: an
/// indeterminate outcome the watchdog did NOT end.
fn exited_driver(dir: &Path) -> Vec<String> {
    driver(dir, "exited-driver.sh", "exit 0\n")
}

/// A driver that accepts and then succeeds with `result`, a complete JSON
/// object literal such as `{"result":"complete"}`.
fn succeeded_driver(dir: &Path, result: &str) -> Vec<String> {
    let tail = format!(
        r#"printf '%s,"type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{result}}}\n' '{{"proto":"forge-driver/v1","msg_id":"r"' "$eid" "$aid"
"#
    );
    driver(dir, "succeeded-driver.sh", &tail)
}

/// A realm-dialect step `name` running `argv`.
fn dialect_step(name: &str, argv: Vec<String>) -> SequenceStep {
    step(
        name,
        StepBody::Dialect {
            execution: DialectExecution { argv, state: None },
        },
    )
}

/// A two-step sequence whose first step is `first`: the second is never
/// reached, but only a step before the last is checked against its own
/// vocabulary or a compiled ending.
fn with_unreachable_step(first: SequenceStep) -> Vec<SequenceStep> {
    vec![first, dialect_step("unreachable", vec!["true".into()])]
}

/// Drive a two-step sequence whose first step is `step` (pinned to
/// `candidate`) and return the `effect/failed` words it ended with.
fn two_step_error(
    dir: &Path,
    step: SequenceStep,
    candidate: &Candidate,
    host: Option<PathBuf>,
) -> String {
    let name = step.name.clone();
    let mut engine = sequence_engine(dir, with_unreachable_step(step), 1, host);
    pin_step(&mut engine, &name, candidate);
    engine.drive().unwrap();
    terminal(&engine, EventType::EffectFailed, "error")
}

/// A single step `draft` whose succeeded result the seat record refuses:
/// its model is outside the contract's vocabulary.
fn refused_step(dir: &Path) -> (SequenceStep, Candidate) {
    single_step(
        "draft",
        succeeded_driver(dir, r#"{"result":"complete","model":"not a model"}"#),
    )
}

/// The refused results' words, checked to be the fence's and not an
/// engine error.
fn refused_step_error(dir: &Path, step: SequenceStep, candidate: &Candidate) -> String {
    let error = two_step_error(dir, step, candidate, Some(dir.to_path_buf()));
    assert!(
        error.starts_with("sequence step 'draft': seat record at journal seq "),
        "{error}"
    );
    error
}

/// A one-panel-step sequence `check`, its members pinned at
/// `work:check:<member>`, under `timeout` and the host file `host`.
fn panel_step_engine(
    dir: &Path,
    members: Vec<PanelMember>,
    timeout: u64,
    host: Option<PathBuf>,
) -> Engine {
    let mut engine = sequence_engine(
        dir,
        vec![panel_step("check", members.clone())],
        timeout,
        host,
    );
    for member in &members {
        pin_member(&mut engine, &format!("work:check:{}", member.name), member);
    }
    engine
}

/// Assert `words` names the host file at `dir` exactly once.
fn assert_names_host_once(words: &str, dir: &Path) {
    let named = format!("the host configuration at {}", dir.display());
    assert_eq!(words.matches(&named).count(), 1, "{words}");
}

/// A checkpoint the seat-record fence refuses: its `turn` is a word.
const REFUSED_CHECKPOINT: &str = r#"printf '%s,"type":"checkpoint","effect_id":"%s","attempt_id":"%s","data":{"step":"seat-turn","turn":"one"}}\n' '{"proto":"forge-driver/v1","msg_id":"c1"' "$eid" "$aid"
"#;

/// A succeeded result the refused checkpoint leaves as the journal's
/// failure instead.
const SUCCEEDED_RESULT: &str = r#"printf '%s,"type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete"}}\n' '{"proto":"forge-driver/v1","msg_id":"r"' "$eid" "$aid"
"#;

/// A driver that reports its own failure.
const FAILED_RESULT: &str = r#"printf '%s,"type":"result","effect_id":"%s","attempt_id":"%s","status":"failed","result":null,"error":"driver failed"}\n' '{"proto":"forge-driver/v1","msg_id":"r"' "$eid" "$aid"
"#;

/// A stand-in engine binary `dir`/`name` that speaks the driver protocol
/// and then runs `tail`: a dialect step's own process, driven end to end.
fn fake_engine(dir: &Path, name: &str, tail: &str) -> crate::argv::Executable {
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\n{DRIVER_PRELUDE}{tail}")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    crate::argv::Executable(Some(path))
}

/// The `SubjectMoved` refusal's words for `attempt`.
fn subject_moved(attempt: &str) -> String {
    crate::engine::carry::Refusal::SubjectMoved {
        attempt: attempt.into(),
        observations: "the reviewed tree moved".into(),
    }
    .to_string()
}

/// A report the watchdog did (`killed`) or did not end.
fn report_of(outcome: AttemptOutcome, killed: bool) -> AttemptReport {
    let mut report = crate::engine::tests::report(outcome, "");
    report.deadline_killed = killed;
    report
}

/// The `dsh/spark-glm` raised deadline used by the namer's unit tests.
fn raised(seconds: u64, authored: u64) -> Deadline {
    Deadline {
        seconds,
        cause: Some(Cause::Raised {
            provider: "dsh".into(),
            route: "spark-glm".into(),
            authored,
        }),
    }
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
    let mut engine = panel_step_engine(dir.path(), members, 1, Some(host(dir.path(), Some(2))));
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
    assert_names_host_once(&error, dir.path());
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
    let error = step_error(dir.path(), step, &candidate);
    assert!(
        error.starts_with("sequence step 'step': driver failed; "),
        "{error}"
    );
    assert_names_host_once(&error, dir.path());
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
    assert_names_host_once(&error, &file);
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
    // member was killed.
    let deadline = Deadline {
        seconds: 2,
        cause: Some(Cause::Raised {
            provider: "dsh".into(),
            route: "spark-glm".into(),
            authored: 1,
        }),
    };
    let mut killed = crate::engine::tests::report(
        AttemptOutcome::Indeterminate {
            reason: "killed".into(),
        },
        "",
    );
    killed.deadline_killed = true;
    let reports = [("gone".to_string(), killed)];
    let name = deadline.panel_namer(&reports);
    let mut failed = Panelled::Joined(AttemptOutcome::Failed {
        error: "panel members failed — gone: driver failed".into(),
    });
    name(&mut failed);
    assert_eq!(
        failed.outcome(),
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

#[test]
fn a_dialect_steps_unreadable_host_is_named_once_in_its_words() {
    let dir = tempfile::tempdir().unwrap();
    // Nothing of the dialect's own argv runs: the engine's executable is
    // absent, so the step does not spawn.
    let error = dialect_words(
        dir.path(),
        crate::argv::Executable(Some(dir.path().join("no-such-engine"))),
        EventType::EffectFailed,
        "error",
    );
    assert!(
        error.starts_with("sequence step 'validate': driver did not spawn: "),
        "{error}"
    );
    assert_names_host_once(&error, dir.path());
    assert!(error.contains("cannot be read"), "{error}");
}

#[test]
fn a_sequence_panel_steps_unraised_member_keeps_todays_words() {
    let dir = tempfile::tempdir().unwrap();
    // The floor raises the step to two seconds, but the member exits
    // before the watchdog: the joined reason is untouched.
    let mut engine = panel_step_engine(
        dir.path(),
        vec![panel_member("gone", exited_driver(dir.path()))],
        1,
        Some(host(dir.path(), Some(2))),
    );
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert_eq!(
        reason,
        "sequence step 'check': panel members [\"gone\"] could not establish completion"
    );
}

#[test]
fn a_sequences_missing_change_names_an_unreadable_host_once() {
    let dir = tempfile::tempdir().unwrap();
    let step = dialect_step("validate", vec!["validator".into(), "{change}".into()]);
    let mut engine = sequence_engine(dir.path(), vec![step], 1, Some(dir.path().to_path_buf()));
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert!(
        reason.starts_with(
            "sequence step 'validate': cannot expand {change} because no preceding successful \
             result carries it; "
        ),
        "{reason}"
    );
    assert_names_host_once(&reason, dir.path());
}

#[test]
fn a_sequences_unraised_early_ending_keeps_todays_words() {
    let dir = tempfile::tempdir().unwrap();
    // The floor raises the attempt to two seconds, but the step returns an
    // out-of-vocabulary word before the watchdog: the deadline is not named.
    let (step, candidate) = single_step(
        "draft",
        succeeded_driver(dir.path(), r#"{"result":"halt"}"#),
    );
    let error = two_step_error(
        dir.path(),
        step,
        &candidate,
        Some(host(dir.path(), Some(2))),
    );
    assert_eq!(
        error,
        "sequence step 'draft': reported 'halt', outside its declared results [\"complete\"]"
    );
}

#[test]
fn a_sequences_out_of_vocabulary_result_names_an_unreadable_host_once() {
    let dir = tempfile::tempdir().unwrap();
    let (step, candidate) = single_step(
        "draft",
        succeeded_driver(dir.path(), r#"{"result":"halt"}"#),
    );
    let error = two_step_error(dir.path(), step, &candidate, Some(dir.path().to_path_buf()));
    assert!(
        error.starts_with(
            "sequence step 'draft': reported 'halt', outside its declared results \
             [\"complete\"]; "
        ),
        "{error}"
    );
    assert_names_host_once(&error, dir.path());
}

#[test]
fn a_sequences_refused_checkpoint_names_an_unreadable_host_once() {
    let dir = tempfile::tempdir().unwrap();
    // A step result the checkpoint's seat record refuses: the step is not
    // the last, so the refusal lands at its checkpoint.
    let (step, candidate) = refused_step(dir.path());
    let error = refused_step_error(dir.path(), step, &candidate);
    assert_names_host_once(&error, dir.path());
}

#[test]
fn a_sequences_refused_early_ending_names_an_unreadable_host_once() {
    let dir = tempfile::tempdir().unwrap();
    // The step's result names a compiled ending, so the sequence ends on
    // it; the seat record refuses that early result, and its words name
    // the host once.
    let (mut step, candidate) = refused_step(dir.path());
    step.ends_on = vec!["complete".into()];
    let error = refused_step_error(dir.path(), step, &candidate);
    assert_names_host_once(&error, dir.path());
}

#[test]
fn a_raised_deadline_names_nothing_on_a_panels_refused_outcome() {
    // The one namer a seat-level panel and a sequence's panel step share.
    // The watchdog killed a member, but the carry refusal stopped the join:
    // the refusal's words are the whole outcome, with no deadline words.
    let refusal = subject_moved("a");
    let reports = [(
        "slow".to_string(),
        report_of(
            AttemptOutcome::Indeterminate {
                reason: "killed".into(),
            },
            true,
        ),
    )];
    let mut panelled = Panelled::Refused(AttemptOutcome::Indeterminate {
        reason: refusal.clone(),
    });
    raised(2, 1).panel_namer(&reports)(&mut panelled);
    assert_eq!(
        panelled.outcome(),
        AttemptOutcome::Indeterminate {
            reason: refusal.clone()
        }
    );
    assert!(!refusal.contains("deadline"), "{refusal}");
}

#[test]
fn a_panels_refused_outcome_under_an_unreadable_host_names_the_host_once() {
    // A host that could not be read is named on a refusal too, once: the
    // refusal before the join is not the members' outcome, but the host is
    // still why the seat's own limit governed.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("host.json");
    let deadline = Deadline {
        seconds: 1,
        cause: Some(Cause::Unreadable {
            path: file.clone(),
            kind: std::io::ErrorKind::NotFound,
        }),
    };
    let refusal = subject_moved("a");
    let reports = [(
        "slow".to_string(),
        report_of(
            AttemptOutcome::Indeterminate {
                reason: "killed".into(),
            },
            false,
        ),
    )];
    let mut panelled = Panelled::Refused(AttemptOutcome::Indeterminate {
        reason: refusal.clone(),
    });
    deadline.panel_namer(&reports)(&mut panelled);
    let Panelled::Refused(AttemptOutcome::Indeterminate { reason }) = panelled else {
        panic!("a refusal stays a refusal");
    };
    assert!(reason.starts_with(&refusal), "{reason}");
    assert_names_host_once(&reason, &file);
}

#[test]
fn an_unreadable_host_is_named_in_the_refused_outcome_a_single_attempt_settles_on() {
    // The checkpoint's seat record is refused, so the journal's failure is
    // the outcome the attempt settles on; the host is named there, not on
    // the success the driver went on to report.
    let dir = tempfile::tempdir().unwrap();
    let script = driver(
        dir.path(),
        "refused-checkpoint.sh",
        &format!("{REFUSED_CHECKPOINT}{SUCCEEDED_RESULT}"),
    );
    let mut engine = attempt(dir.path(), Some(dir.path().to_path_buf()), script, 1);
    engine.drive().unwrap();
    let error = terminal(&engine, EventType::EffectFailed, "error");
    assert!(error.starts_with("seat record at journal seq "), "{error}");
    assert_names_host_once(&error, dir.path());
}

#[test]
fn an_unreadable_host_is_named_in_the_refused_outcome_a_sequences_step_settles_on() {
    let dir = tempfile::tempdir().unwrap();
    let (step, candidate) = single_step(
        "step",
        driver(
            dir.path(),
            "refused-checkpoint-step.sh",
            &format!("{REFUSED_CHECKPOINT}{SUCCEEDED_RESULT}"),
        ),
    );
    let error = step_error(dir.path(), step, &candidate);
    assert!(
        error.starts_with("sequence step 'step': seat record at journal seq "),
        "{error}"
    );
    assert_names_host_once(&error, dir.path());
}

#[test]
fn an_unreadable_host_is_named_in_the_refused_outcome_a_dialect_step_settles_on() {
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_engine(
        dir.path(),
        "refused-dialect.sh",
        &format!("{REFUSED_CHECKPOINT}{FAILED_RESULT}"),
    );
    let error = dialect_words(dir.path(), fake, EventType::EffectFailed, "error");
    assert!(
        error.starts_with("sequence step 'validate': seat record at journal seq "),
        "{error}"
    );
    assert_names_host_once(&error, dir.path());
}

#[test]
fn a_raised_deadline_leaves_a_refused_outcome_exactly_as_the_journal_set_it() {
    // The raised words stay on the outcome the driver received; the
    // journal's refusal is the outcome the attempt settles on, unchanged.
    let mut report = report_of(
        AttemptOutcome::Indeterminate {
            reason: "attempt exceeded its 2s deadline and was killed".into(),
        },
        true,
    );
    let refused = AttemptOutcome::Failed {
        error: "the journal's refusal".into(),
    };
    report.refused = Some(refused.clone());
    let mut run = DriverRun::Ran(report);
    raised(2, 1).name(&mut run);
    let DriverRun::Ran(report) = run else {
        panic!("the attempt ran");
    };
    assert_eq!(
        report.outcome,
        AttemptOutcome::Indeterminate {
            reason: "the seat's 2-second deadline: the shared-local floor host.json declares for \
                     dsh/spark-glm; the seat's own limit is 1 seconds; its process tree is gone, \
                     and what it did is unknown"
                .into()
        }
    );
    assert_eq!(report.settled_outcome(), refused);
}

#[test]
fn a_raised_deadline_names_both_numbers_and_the_source_on_a_killed_dialect_step() {
    // The sequence launches a candidate on the shared-local route, so its
    // attempt is raised; the dialect step then runs to the watchdog.
    let dir = tempfile::tempdir().unwrap();
    let (first, candidate) = single_step(
        "first",
        succeeded_driver(dir.path(), r#"{"result":"complete"}"#),
    );
    let fake = fake_engine(dir.path(), "slow-dialect.sh", "sleep 30\n");
    let mut engine = sequence_engine(
        dir.path(),
        vec![first, dialect_step("validate", vec!["true".into()])],
        1,
        Some(host(dir.path(), Some(2))),
    );
    pin_step(&mut engine, "first", &candidate);
    engine.bundle.executable = fake;
    engine.drive().unwrap();
    let reason = terminal(&engine, EventType::EffectIndeterminate, "reason");
    assert!(
        reason.starts_with("sequence step 'validate': the seat's 2-second deadline: "),
        "{reason}"
    );
    assert!(
        reason.contains(
            "the shared-local floor host.json declares for dsh/spark-glm; the seat's own limit is \
             1 seconds"
        ),
        "{reason}"
    );
    assert!(reason.contains("its process tree is gone"), "{reason}");
}

#[test]
fn a_dialect_step_that_ran_under_an_unreadable_host_names_it_once() {
    // The step's own process accepts and fails: not a spawn failure, so the
    // host is named in the outcome the step settles on.
    let dir = tempfile::tempdir().unwrap();
    let fake = fake_engine(dir.path(), "failed-dialect.sh", FAILED_RESULT);
    let error = dialect_words(dir.path(), fake, EventType::EffectFailed, "error");
    assert!(
        error.starts_with("sequence step 'validate': driver failed; "),
        "{error}"
    );
    assert_names_host_once(&error, dir.path());
}
