//! Host-independent tests for the bounded R3 probe model. They exercise the
//! shared decision logic on Linux; they are not native macOS evidence and
//! close no SEATBELT residual.

use super::*;

/// A scripted host: returns one precondition and one result per case, and
/// records which cases it was actually asked to run.
struct ScriptedHost {
    precondition: Option<Precondition>,
    results: Vec<(Case, CaseResult)>,
    runs: Vec<Case>,
}

impl ScriptedHost {
    fn new(precondition: Option<Precondition>) -> ScriptedHost {
        ScriptedHost {
            precondition,
            results: Vec::new(),
            runs: Vec::new(),
        }
    }

    fn with(mut self, result: CaseResult) -> ScriptedHost {
        self.results.push((result.case, result));
        self
    }
}

impl ProbeHost for ScriptedHost {
    fn precondition(&mut self) -> Option<Precondition> {
        self.precondition.clone()
    }

    fn run_case(&mut self, case: Case) -> CaseResult {
        self.runs.push(case);
        self.results
            .iter()
            .find(|(scripted, _)| *scripted == case)
            .map(|(_, result)| result.clone())
            .unwrap_or_else(|| CaseResult::skipped(case, "scripted host has no result"))
    }
}

fn all_passing() -> Vec<CaseResult> {
    Case::ALL.iter().copied().map(passing_result).collect()
}

#[test]
fn every_case_in_the_obligation_matrix_passes_a_complete_run() {
    let results = all_passing();
    let verdict = evaluate(&Case::ALL, &results, None);
    assert!(verdict.is_pass(), "{}", verdict.render());
    assert!(verdict.reasons().is_empty());
}

#[test]
fn an_empty_selection_fails() {
    let verdict = evaluate(&[], &[], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("zero cases selected"));
}

#[test]
fn every_precondition_fails_and_runs_no_case() {
    for precondition in [
        Precondition::LauncherMissing,
        Precondition::LaunchctlMissing,
        Precondition::OuterBox,
        Precondition::FacilityUnsupported("launchd refuses transient jobs".to_string()),
    ] {
        let mut host = ScriptedHost::new(Some(precondition.clone()));
        let report = run_probe(&mut host, &Case::ALL);
        assert!(!report.verdict.is_pass(), "{:?}", precondition);
        assert!(host.runs.is_empty(), "a precondition must run no case");
        assert!(report.verdict.reasons()[0].contains("precondition failed"));
    }
}

#[test]
fn a_skipped_case_is_a_failure_not_a_pass() {
    let mut results = all_passing();
    results[4] = CaseResult::skipped(Case::DoubleFork, "sandbox-exec disappeared");
    let verdict = evaluate(&Case::ALL, &results, None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons().iter().any(
        |reason| reason.contains("double-fork") && reason.contains("sandbox-exec disappeared")
    ));
}

#[test]
fn a_selected_case_with_no_result_fails() {
    let mut results = all_passing();
    results.pop();
    let verdict = evaluate(&Case::ALL, &results, None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons().iter().any(
        |reason| reason.contains("group-kill-negative-control") && reason.contains("no result")
    ));
}

#[test]
fn an_ordinary_child_without_its_positive_control_fails() {
    let mut result = passing_result(Case::OrdinaryChild);
    result.positive_control = false;
    let verdict = evaluate(&[Case::OrdinaryChild], &[result], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("positive control"));
}

#[test]
fn an_ordinary_child_with_a_surviving_label_fails() {
    let mut result = passing_result(Case::OrdinaryChild);
    result.labels_gone = false;
    let verdict = evaluate(&[Case::OrdinaryChild], &[result], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("transient job label"));
}

#[test]
fn every_no_survivor_obligation_fails_when_its_fact_is_wrong() {
    type Mutation = fn(&mut CaseResult);
    let base = passing_result(Case::Timeout);

    let cases: [(&str, Mutation); 5] = [
        ("heartbeat", |r: &mut CaseResult| r.heartbeat_before = false),
        ("survivors", |r: &mut CaseResult| r.survivors_after = 1),
        ("heartbeat_moved", |r: &mut CaseResult| {
            r.heartbeat_moved_during_quiet = true
        }),
        ("labels", |r: &mut CaseResult| r.labels_gone = false),
        ("cleanup", |r: &mut CaseResult| {
            r.cleanup_after_quiescence = false
        }),
    ];
    for (label, mutate) in cases {
        let mut result = base.clone();
        mutate(&mut result);
        let verdict = evaluate(&[Case::Timeout], &[result], None);
        assert!(!verdict.is_pass(), "{label} must fail");
        assert!(
            verdict.reasons()[0].contains("timeout"),
            "{label}: {}",
            verdict.render()
        );
    }
}

#[test]
fn a_missing_adversarial_positive_control_fails() {
    let mut result = passing_result(Case::Timeout);
    result.positive_control = false;
    result.heartbeat_before = false;
    let verdict = evaluate(&[Case::Timeout], &[result], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("heartbeat never advanced"));
}

#[test]
fn a_negative_control_that_contained_the_escape_fails() {
    let mut result = passing_result(Case::GroupKillNegativeControl);
    result.survivors_after = 0;
    let verdict = evaluate(&[Case::GroupKillNegativeControl], &[result], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("did not survive"));
}

#[test]
fn every_payload_denial_obligation_fails_when_its_fact_is_wrong() {
    for case in [Case::GuardInterference, Case::PeerBootout, Case::EscapeJob] {
        let mut denied = passing_result(case);
        denied.payload_attempt_denied = false;
        let verdict = evaluate(&[case], &[denied], None);
        assert!(!verdict.is_pass());
        assert!(verdict.reasons()[0].contains("not denied"));

        let mut guard = passing_result(case);
        guard.guard_survived_payload = false;
        let verdict = evaluate(&[case], &[guard], None);
        assert!(!verdict.is_pass());
        assert!(verdict.reasons()[0].contains("guard did not survive"));
    }
}

#[test]
fn lifecycle_events_must_be_ordered() {
    // A reordered event is refused.
    let mut reordered = passing_result(Case::Timeout);
    reordered.events = vec![
        Event::Prepared,
        Event::PayloadStarted,
        Event::GuardRegistered,
    ];
    let verdict = evaluate(&[Case::Timeout], &[reordered], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("out of order"));

    // A duplicate is refused.
    let mut duplicate = passing_result(Case::Timeout);
    duplicate.events = vec![Event::Prepared, Event::Prepared, Event::GuardRegistered];
    assert!(!events_are_ordered(&duplicate.events));

    // Cleanup before quiescence is refused.
    let mut early = passing_result(Case::Timeout);
    early.events = vec![
        Event::Prepared,
        Event::GuardRegistered,
        Event::PayloadStarted,
        Event::Terminating,
        Event::PrivateStateRemoved,
        Event::PayloadQuiescent,
    ];
    assert!(!events_are_ordered(&early.events));

    // A full, ordered lifecycle is accepted.
    assert!(events_are_ordered(&LIFECYCLE));
}

#[test]
fn verdicts_render_every_reason() {
    let pass = Verdict::Pass;
    assert!(pass.is_pass());
    assert!(pass.reasons().is_empty());
    assert!(pass.render().contains("PASS"));

    let fail = Verdict::Fail(vec!["one".to_string(), "two".to_string()]);
    assert!(!fail.is_pass());
    let rendered = fail.render();
    assert!(rendered.contains("FAIL"));
    assert!(rendered.contains("one"));
    assert!(rendered.contains("two"));
}

#[test]
fn a_full_run_reports_every_selected_case() {
    let mut host = ScriptedHost::new(None);
    for result in all_passing() {
        host = host.with(result);
    }
    let report = run_probe(&mut host, &Case::ALL);
    assert_eq!(report.selected.len(), Case::ALL.len());
    assert_eq!(report.results.len(), Case::ALL.len());
    assert_eq!(host.runs.len(), Case::ALL.len());
    assert!(report.verdict.is_pass());
}

#[test]
fn case_names_and_expectations_are_total() {
    assert_eq!(Case::ALL.len(), 12);
    for case in Case::ALL {
        assert!(!case.name().is_empty());
        assert_eq!(case.to_string(), case.name());
    }
    assert_eq!(
        Case::OrdinaryChild.expectation(),
        Expectation::OrdinaryCompletion
    );
    assert_eq!(Case::Timeout.expectation(), Expectation::NoSurvivor);
    assert_eq!(
        Case::GroupKillNegativeControl.expectation(),
        Expectation::GroupKillLeavesSurvivor
    );
    assert_eq!(Case::EscapeJob.expectation(), Expectation::PayloadDenied);
}
