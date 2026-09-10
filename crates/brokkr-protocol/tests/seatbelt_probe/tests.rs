//! Host-independent tests for the bounded R3 probe model. They exercise the
//! shared decision logic on Linux; they are not native macOS evidence and
//! close no SEATBELT residual.

use super::*;
use brokkr_protocol::hands::HOST_TOOLCHAIN_BINDS;

/// A scripted host: returns one precondition, one startup observation per
/// cell and one lifetime result per case, recording what it was asked to run.
struct ScriptedHost {
    precondition: Option<Precondition>,
    results: Vec<(Case, CaseResult)>,
    startup: Vec<(StartupCell, StartupObservation)>,
    runs: Vec<Case>,
    startup_runs: Vec<StartupCell>,
}

impl ScriptedHost {
    fn new(precondition: Option<Precondition>) -> ScriptedHost {
        ScriptedHost {
            precondition,
            results: Vec::new(),
            startup: Vec::new(),
            runs: Vec::new(),
            startup_runs: Vec::new(),
        }
    }

    fn with(mut self, result: CaseResult) -> ScriptedHost {
        self.results.push((result.case, result));
        self
    }

    fn with_startup(mut self, observation: StartupObservation) -> ScriptedHost {
        self.startup.push((observation.cell, observation));
        self
    }

    fn all_passing() -> ScriptedHost {
        let mut host = ScriptedHost::new(None);
        for cell in StartupCell::ALL {
            host = host.with_startup(passing_startup(cell));
        }
        for case in Case::ALL {
            host = host.with(passing_result(case));
        }
        host
    }
}

impl ProbeHost for ScriptedHost {
    fn precondition(&mut self) -> Option<Precondition> {
        self.precondition.clone()
    }

    fn run_startup_cell(&mut self, cell: StartupCell) -> StartupObservation {
        self.startup_runs.push(cell);
        self.startup
            .iter()
            .find(|(scripted, _)| *scripted == cell)
            .map(|(_, observation)| observation.clone())
            .unwrap_or_else(|| {
                StartupObservation::skipped(cell, "scripted host has no observation")
            })
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
fn lifecycle_events_must_be_exact_not_a_prefix() {
    // An intended prefix is not a pass.
    let mut prefix = passing_result(Case::Timeout);
    prefix.events.truncate(4);
    let verdict = evaluate(&[Case::Timeout], &[prefix], None);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("not the exact required"));

    // A reordered event is refused.
    let mut reordered = passing_result(Case::Timeout);
    reordered.events = vec![
        Event::Prepared,
        Event::PayloadStarted,
        Event::GuardRegistered,
    ];
    let verdict = evaluate(&[Case::Timeout], &[reordered], None);
    assert!(!verdict.is_pass());

    // A duplicate is refused by the ordered check.
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

    // The negative control has its own guard-free lifecycle.
    assert_eq!(
        Case::GroupKillNegativeControl.expected_lifecycle(),
        vec![
            Event::Prepared,
            Event::PayloadStarted,
            Event::Terminating,
            Event::PayloadQuiescent,
            Event::PrivateStateRemoved,
        ]
    );

    // A full, ordered lifecycle is accepted.
    assert!(events_are_ordered(&LIFECYCLE));
}

#[test]
fn every_measurement_repair_forgery_fails() {
    type Mutation = fn(&mut CaseResult);
    let generic: [(&str, Mutation); 6] = [
        ("trigger", |r: &mut CaseResult| {
            r.repairs.trigger = TriggerKind::Cancellation
        }),
        ("forged_marker", |r: &mut CaseResult| {
            r.repairs.forged_marker = true
        }),
        ("harness_cleanup", |r: &mut CaseResult| {
            r.repairs.harness_cleanup_counted = true
        }),
        ("start_time", |r: &mut CaseResult| {
            r.repairs.identities_have_start_time = false
        }),
        ("reaped", |r: &mut CaseResult| {
            r.repairs.reaped_all_children = false
        }),
        ("failure_report", |r: &mut CaseResult| {
            r.repairs.failure_report_preserved = false
        }),
    ];
    for (label, mutate) in generic {
        let mut result = passing_result(Case::Timeout);
        mutate(&mut result);
        let verdict = evaluate(&[Case::Timeout], &[result], None);
        assert!(!verdict.is_pass(), "{label} must fail");
    }

    // Late guard sampling fails for any guard-observing case.
    let mut result = passing_result(Case::Timeout);
    result.repairs.guard_observed_registered = false;
    let verdict = evaluate(&[Case::Timeout], &[result], None);
    assert!(!verdict.is_pass(), "late guard sampling must fail");
    assert!(verdict
        .reasons()
        .iter()
        .any(|r| r.contains("after unregister")));

    // A denial case with no recorded attack before the denial fails.
    let mut result = passing_result(Case::GuardInterference);
    result.repairs.attack_recorded_before_observation = false;
    let verdict = evaluate(&[Case::GuardInterference], &[result], None);
    assert!(!verdict.is_pass(), "missing attack record must fail");
    assert!(verdict
        .reasons()
        .iter()
        .any(|r| r.contains("prior recorded attempt")));

    // An early peer attack (peer not synchronized first) fails.
    let mut result = passing_result(Case::PeerBootout);
    result.repairs.peer_synchronized_before_attack = false;
    let verdict = evaluate(&[Case::PeerBootout], &[result], None);
    assert!(!verdict.is_pass(), "early peer attack must fail");
    assert!(verdict.reasons().iter().any(|r| r.contains("synchronized")));

    // The negative control owns the real-group-kill fact.
    let mut control = passing_result(Case::GroupKillNegativeControl);
    control.repairs.real_original_group_kill = false;
    let verdict = evaluate(&[Case::GroupKillNegativeControl], &[control], None);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|r| r.contains("real original-process-group")));

    // The negative control does not require a guard observation.
    let mut control = passing_result(Case::GroupKillNegativeControl);
    control.repairs.guard_observed_registered = false;
    assert!(evaluate(&[Case::GroupKillNegativeControl], &[control], None).is_pass());
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

// ---------------------------------------------------------------------------
// Gate A: the startup matrix
// ---------------------------------------------------------------------------

#[test]
fn every_startup_cell_passes_a_complete_matrix() {
    let cells: Vec<StartupObservation> = StartupCell::ALL
        .iter()
        .copied()
        .map(passing_startup)
        .collect();
    let verdict = evaluate_startup(&StartupCell::ALL, &cells);
    assert!(verdict.is_pass(), "{}", verdict.render());
}

#[test]
fn an_empty_startup_selection_fails() {
    let verdict = evaluate_startup(&[], &[]);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("zero startup cells"));
}

#[test]
fn every_startup_cell_failure_is_named_separately() {
    type Mutation = fn(&mut StartupObservation);
    let mutations: [(&str, Mutation); 6] = [
        ("not_run", |c: &mut StartupObservation| {
            c.ran = false;
            c.skip = Some("sandbox-exec rejected the profile".to_string());
        }),
        ("no_ready", |c: &mut StartupObservation| c.ready = false),
        ("no_child", |c: &mut StartupObservation| {
            c.ordinary_child = false
        }),
        ("signal", |c: &mut StartupObservation| {
            c.exit = HelperExit::Signal(6)
        }),
        ("no_digest", |c: &mut StartupObservation| {
            c.helper_digest.clear()
        }),
        ("crash", |c: &mut StartupObservation| {
            c.launchd_successive_crashes = Some(1)
        }),
    ];
    for cell in StartupCell::ALL {
        for (label, mutate) in mutations {
            if label == "crash" && !cell.launchd() {
                continue;
            }
            let mut observation = passing_startup(cell);
            mutate(&mut observation);
            let cells = vec![observation];
            let verdict = evaluate_startup(&[cell], &cells);
            assert!(!verdict.is_pass(), "{} {label} must fail", cell.name());
            assert!(
                verdict
                    .reasons()
                    .iter()
                    .any(|reason| reason.contains(cell.name())),
                "{} {label}: {}",
                cell.name(),
                verdict.render()
            );
        }
    }
}

#[test]
fn a_seatbelt_cell_without_a_profile_digest_fails() {
    let mut observation = passing_startup(StartupCell::S1DirectSeatbelt);
    observation.profile_digest = None;
    let verdict = evaluate_startup(&[StartupCell::S1DirectSeatbelt], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|r| r.contains("profile digest")));
}

#[test]
fn startup_cells_must_share_helper_bytes_and_argv() {
    let mut cells = vec![
        passing_startup(StartupCell::S0DirectUnboxed),
        passing_startup(StartupCell::S1DirectSeatbelt),
    ];
    cells[1].helper_argv.push("--extra".to_string());
    let verdict = evaluate_startup(
        &[StartupCell::S0DirectUnboxed, StartupCell::S1DirectSeatbelt],
        &cells,
    );
    assert!(!verdict.is_pass());
    assert!(verdict.reasons().iter().any(|r| r.contains("argv differs")));

    let mut cells = vec![
        passing_startup(StartupCell::S0DirectUnboxed),
        passing_startup(StartupCell::S1DirectSeatbelt),
    ];
    cells[1].helper_digest = "ffff".to_string();
    let verdict = evaluate_startup(
        &[StartupCell::S0DirectUnboxed, StartupCell::S1DirectSeatbelt],
        &cells,
    );
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|r| r.contains("helper bytes differ")));
}

#[test]
fn a_labelled_default_allow_diagnostic_can_never_pass() {
    let cell = StartupCell::S1DirectSeatbelt;
    let observation = passing_startup(cell).labelled_diagnostic("allow-default profile at S1");
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict.reasons()[0].contains("labelled non-passing diagnostic"));
}

#[test]
fn a_seatbelt_cell_that_reached_ready_must_prove_the_root_read_was_load_bearing() {
    let cell = StartupCell::S1DirectSeatbelt;

    // A passing cell with no removal evidence cannot show the root-inode rule
    // did any work.
    let mut observation = passing_startup(cell);
    observation.negative_controls.clear();
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("load-bearing negative control")));

    // A removal that still started the payload names a non-load-bearing rule.
    let mut observation = passing_startup(cell);
    observation.negative_controls[0].status = RemovalStatus::ObservedNotBlocking;
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("load-bearing negative control")));

    // A removal that never ran proves nothing either.
    let mut observation = passing_startup(cell);
    observation.negative_controls[0].status = RemovalStatus::Missing;
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("load-bearing negative control")));

    // The satisfied control is the only shape that passes.
    let observation = passing_startup(cell);
    assert!(observation.negative_controls[0].satisfied());
}

#[test]
fn an_unboxed_cell_does_not_require_the_root_read_negative_control() {
    let cell = StartupCell::S0DirectUnboxed;
    let observation = passing_startup(cell);
    assert!(observation.negative_controls.is_empty());
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(verdict.is_pass(), "{}", verdict.render());
}

#[test]
fn a_ready_seatbelt_cell_without_a_clean_denial_record_can_never_pass() {
    // The removal control is an additional obligation, never a replacement:
    // denial controls are still required alongside it.
    let cell = StartupCell::S3LaunchdSeatbelt;
    let mut observation = passing_startup(cell);
    observation.denials.clear();
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("denial control credential-read")));
}

#[test]
fn startup_cells_may_use_isolated_roots_but_not_drift() {
    let mut cells: Vec<StartupObservation> = StartupCell::ALL
        .iter()
        .copied()
        .map(passing_startup)
        .collect();
    for (index, cell) in cells.iter_mut().enumerate() {
        // The controller-generated root is a typed variable: distinct,
        // isolated roots are accepted.
        cell.helper_root = format!("/private/tmp/isolated-cell-{index}");
    }
    let verdict = evaluate_startup(&StartupCell::ALL, &cells);
    assert!(verdict.is_pass(), "{}", verdict.render());

    // Any other immutable launch input still drifts into failure.
    let mut executable = cells.clone();
    executable[2].helper_executable = "/tmp/lookalike-helper".to_string();
    let verdict = evaluate_startup(&StartupCell::ALL, &executable);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("executable differs")));

    let mut mode = cells.clone();
    mode[2].helper_mode = 0o644;
    let verdict = evaluate_startup(&StartupCell::ALL, &mode);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("permission mode differs")));

    let mut argv = cells;
    argv[2].helper_argv.push("--extra".to_string());
    let verdict = evaluate_startup(&StartupCell::ALL, &argv);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("argv differs")));
}

#[test]
fn startup_argv_must_carry_the_typed_root_and_a_nonce() {
    let mut missing_root = passing_startup(StartupCell::S0DirectUnboxed);
    missing_root.helper_argv.retain(|arg| arg != ROOT_TOKEN);
    let verdict = evaluate_startup(&[StartupCell::S0DirectUnboxed], &[missing_root]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("typed cell root")));

    let mut empty_nonce = passing_startup(StartupCell::S0DirectUnboxed);
    let index = empty_nonce
        .helper_argv
        .iter()
        .position(|arg| arg == "--nonce")
        .expect("passing argv has a nonce");
    empty_nonce.helper_argv[index + 1].clear();
    let verdict = evaluate_startup(&[StartupCell::S0DirectUnboxed], &[empty_nonce]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("nonce")));
}

#[test]
fn launchd_cells_require_a_parsed_terminal_state() {
    for cell in [
        StartupCell::S2LaunchdUnboxed,
        StartupCell::S3LaunchdSeatbelt,
    ] {
        let mut observation = passing_startup(cell);
        observation.launchd_state = None;
        let verdict = evaluate_startup(&[cell], &[observation]);
        assert!(!verdict.is_pass());
        assert!(
            verdict
                .reasons()
                .iter()
                .any(|reason| reason.contains("terminal state")),
            "{}: {}",
            cell.name(),
            verdict.render()
        );
    }
}

#[test]
fn startup_cells_require_the_exact_bounded_stage_sequence() {
    let mut missing = passing_startup(StartupCell::S0DirectUnboxed);
    missing.stages.pop();
    let verdict = evaluate_startup(&[StartupCell::S0DirectUnboxed], &[missing]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("startup stages")));

    let mut reordered = passing_startup(StartupCell::S0DirectUnboxed);
    reordered.stages.swap(0, 1);
    let verdict = evaluate_startup(&[StartupCell::S0DirectUnboxed], &[reordered]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("startup stages")));

    // The exact sequence passes.
    let complete = passing_startup(StartupCell::S0DirectUnboxed);
    assert_eq!(complete.stages, STARTUP_STAGES);
    assert!(evaluate_startup(&[StartupCell::S0DirectUnboxed], &[complete]).is_pass());
}

#[test]
fn launchd_print_parsing_never_synthesizes_missing_fields() {
    let complete = "gui/501/label = {\n\tstate = not running\n\truns = 1\n\tlast exit code = 0\n\tlast exit reason = exited normally\n}\n";
    let facts = parse_launchd_print(complete).expect("complete launchd state");
    assert_eq!(facts.state.as_deref(), Some("not running"));
    assert_eq!(facts.runs, Some(1));
    assert_eq!(facts.last_exit_code, Some(0));
    assert_eq!(
        facts.successive_crashes, None,
        "an omitted crash counter stays unknown"
    );
    assert_eq!(classify_launchd_exit(&facts), HelperExit::Clean);

    // An unknown field does not erase another fact; the required terminal
    // facts still fail closed.
    let partial = "gui/501/label = {\n\tstate = not running\n\truns = 1\n}\n";
    let facts = parse_launchd_print(partial).expect("parseable");
    assert_eq!(facts.state.as_deref(), Some("not running"));
    assert_eq!(facts.runs, Some(1));
    assert_eq!(facts.last_exit_code, None);
    assert_eq!(classify_launchd_exit(&facts), HelperExit::NotRun);

    // The running sample is non-terminal, even with a run counter.
    let running =
        "gui/501/label = {\n\tstate = running\n\truns = 0\n\tlast exit code = (never exited)\n}\n";
    let facts = parse_launchd_print(running).expect("parseable running state");
    assert_eq!(facts.last_exit_code, None);
    assert_eq!(classify_launchd_exit(&facts), HelperExit::NotRun);

    // A present nonzero crash counter or terminating signal fails closed.
    let crash = "gui/501/label = {\n\tstate = not running\n\truns = 2\n\tlast exit code = 0\n\tsuccessive crashes = 1\n}\n";
    let facts = parse_launchd_print(crash).expect("parseable");
    assert_eq!(facts.successive_crashes, Some(1));
    let signal = "gui/501/label = {\n\tstate = not running\n\truns = 1\n\tlast exit code = 0\n\tterminating signal = 9\n}\n";
    let facts = parse_launchd_print(signal).expect("parseable");
    assert_eq!(facts.terminating_signal, Some(9));

    // Nested and duplicated keys never manufacture a fact.
    let nested = "gui/501/label = {\n\tstate = not running\n\tstate = running\n\truns = 1\n\tlast exit code = 0\n\tsome block = {\n\t\truns = 99\n\t}\n}\n";
    let facts = parse_launchd_print(nested).expect("parseable");
    assert_eq!(facts.state, None, "a duplicated top-level key is unknown");
    assert_eq!(facts.runs, Some(1), "a nested key is not a top-level fact");
    assert!(facts
        .nested
        .iter()
        .any(|(path, line)| path.contains("some block") && line.contains("runs = 99")));
}

#[test]
fn the_fa7_launchd_samples_parse_as_the_measured_top_level_dictionary() {
    use super::fa7_launchd_samples::*;
    let running = parse_launchd_print(FA7_S2_RUNNING).expect("the running fa7 sample parses");
    assert_eq!(running.state.as_deref(), Some("running"));
    assert_eq!(running.active_count, Some(1));
    assert_eq!(
        running.last_exit_code_raw.as_deref(),
        Some("(never exited)"),
        "the raw value stays typed"
    );
    assert_eq!(running.last_exit_code, None);
    assert_eq!(
        classify_launchd_exit(&running),
        HelperExit::NotRun,
        "the running fa7 sample is non-terminal"
    );
    assert!(
        running.nested.iter().any(|(path, _)| path == "arguments"),
        "nested entries are kept under their block path"
    );

    // The measured S2 not-running sample carries `runs = 1`, `last exit
    // code = 0` and no `successive crashes` field; the new parser must read it
    // as one clean run rather than refuse the absent counter.
    let terminal = parse_launchd_print(FA7_S2_NOT_RUNNING).expect("the terminal fa7 sample parses");
    assert_eq!(terminal.state.as_deref(), Some("not running"));
    assert_eq!(terminal.runs, Some(1));
    assert_eq!(terminal.last_exit_code, Some(0));
    assert_eq!(terminal.successive_crashes, None);
    assert_eq!(classify_launchd_exit(&terminal), HelperExit::Clean);
    assert_eq!(terminal.active_count, Some(0));

    let s3 = parse_launchd_print(FA7_S3_RUNNING).expect("the S3 fa7 sample parses");
    assert!(s3.state.is_some());
}

#[test]
fn a_non_starting_seatbelt_cell_owes_no_removal_verdict() {
    let cell = StartupCell::S1DirectSeatbelt;
    let mut observation = passing_startup(cell);
    observation.ready = false;
    observation.negative_controls = vec![StartupNegativeControl {
        name: "root-inode-read".to_string(),
        removed_rule: "(allow file-read* (literal \"/\"))".to_string(),
        consumer: "root inode".to_string(),
        status: RemovalStatus::NotDue,
        exit: HelperExit::NotRun,
        stdout: String::new(),
        stderr: String::new(),
    }];
    // The cell fails on its own startup facts, and the not-due removal is not
    // read as blocking.
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(!verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("negative control")));
}

#[test]
fn a_stripped_replay_that_reaches_ready_is_not_blocking() {
    let cell = StartupCell::S1DirectSeatbelt;
    let mut observation = passing_startup(cell);
    observation.negative_controls[0].status = RemovalStatus::ObservedNotBlocking;
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("load-bearing negative control")));
}

#[test]
fn a_measured_nonzero_crash_counter_fails_the_cell() {
    for cell in [
        StartupCell::S2LaunchdUnboxed,
        StartupCell::S3LaunchdSeatbelt,
    ] {
        let mut observation = passing_startup(cell);
        observation.launchd_successive_crashes = Some(2);
        let verdict = evaluate_startup(&[cell], &[observation]);
        assert!(!verdict.is_pass(), "{}", cell.name());
        assert!(verdict
            .reasons()
            .iter()
            .any(|reason| reason.contains("successive crash")));
    }
}

#[test]
fn a_measured_terminating_signal_fails_the_cell() {
    for cell in [
        StartupCell::S2LaunchdUnboxed,
        StartupCell::S3LaunchdSeatbelt,
    ] {
        let mut observation = passing_startup(cell);
        observation.launchd_terminating_signal = Some(9);
        let verdict = evaluate_startup(&[cell], &[observation]);
        assert!(!verdict.is_pass(), "{}", cell.name());
        assert!(verdict
            .reasons()
            .iter()
            .any(|reason| reason.contains("terminating signal")));
    }
}

#[test]
fn a_ready_diagnostic_can_never_pass_its_cell() {
    let mut observation = passing_startup(StartupCell::S1DirectSeatbelt);
    observation.ready = false;
    observation.ordinary_child = false;
    observation.diagnostics = vec![ProfileDiagnostic {
        name: "helper-parent-read".to_string(),
        operation: "file-read*".to_string(),
        target: "/Users".to_string(),
        consumer: "helper executable".to_string(),
        allowance: "(allow file-read* (subpath \"/Users\"))".to_string(),
        ready: true,
        ordinary_child: true,
        exit: HelperExit::Clean,
        stdout: String::new(),
        stderr: String::new(),
    }];
    assert!(observation.diagnostics[0].reached_ready());
    let verdict = evaluate_startup(&[StartupCell::S1DirectSeatbelt], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("never reached")));
}

#[test]
fn a_passing_seatbelt_cell_requires_its_denial_controls() {
    let complete = passing_startup(StartupCell::S1DirectSeatbelt);
    assert_eq!(
        complete
            .denials
            .iter()
            .filter(|control| control.satisfied())
            .count(),
        STARTUP_DENIAL_CONTROLS.len()
    );
    assert!(evaluate_startup(
        &[StartupCell::S1DirectSeatbelt],
        std::slice::from_ref(&complete)
    )
    .is_pass());

    // A missing control fails.
    let mut missing = complete.clone();
    missing.denials.pop();
    let verdict = evaluate_startup(&[StartupCell::S1DirectSeatbelt], &[missing]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("denial control")));

    // An observed-but-allowed control fails.
    let mut allowed = complete.clone();
    allowed.denials[0].denied = false;
    let verdict = evaluate_startup(&[StartupCell::S1DirectSeatbelt], &[allowed]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("not observed and denied")));

    // Unboxed cells carry and require no Seatbelt denial controls.
    let unboxed = passing_startup(StartupCell::S0DirectUnboxed);
    assert!(unboxed.denials.is_empty());
    assert!(evaluate_startup(&[StartupCell::S0DirectUnboxed], &[unboxed]).is_pass());
}

// ---------------------------------------------------------------------------
// The two-gate order
// ---------------------------------------------------------------------------

#[test]
fn a_startup_failure_marks_the_lifetime_matrix_not_run() {
    let mut host = ScriptedHost::all_passing();
    // Replace S1 with a failed startup observation.
    host.startup
        .retain(|(cell, _)| *cell != StartupCell::S1DirectSeatbelt);
    host = host.with_startup(StartupObservation::skipped(
        StartupCell::S1DirectSeatbelt,
        "SIGABRT",
    ));
    let report = run_gated_probe(&mut host, &StartupCell::ALL, &Case::ALL);
    assert!(!report.verdict.is_pass());
    assert!(report.lifetime.is_none(), "lifetime must not run");
    assert!(
        host.runs.is_empty(),
        "no lifetime case may run after a startup failure"
    );
    assert!(report.verdict.reasons()[0].contains("lifetime matrix not run"));
}

#[test]
fn a_passing_startup_gate_runs_the_lifetime_matrix() {
    let mut host = ScriptedHost::all_passing();
    let report = run_gated_probe(&mut host, &StartupCell::ALL, &Case::ALL);
    assert!(report.verdict.is_pass(), "{}", report.verdict.render());
    assert!(report.lifetime.is_some());
    assert_eq!(host.runs.len(), Case::ALL.len());
    assert_eq!(host.startup_runs.len(), StartupCell::ALL.len());
}

// ---------------------------------------------------------------------------
// The audited startup-rule ledger and the pure check (tasks 1.18-1.21)
// ---------------------------------------------------------------------------

/// Fixed concrete inputs for host-independent check tests. They are a legal
/// canonical private layout: a fresh cell root under `/private/var/folders`
/// (its resolved spelling), a helper staged outside every cell root, and the
/// fixed payload/inputs layout.
pub(super) fn fixed_inputs() -> CheckInputs {
    CheckInputs {
        cell_root: "/private/var/folders/xy/T/brokkr-seatbelt-probe-1/cell-1".to_string(),
        payload_root: "/private/var/folders/xy/T/brokkr-seatbelt-probe-1/cell-1/payload"
            .to_string(),
        inputs_dir: "/private/var/folders/xy/T/brokkr-seatbelt-probe-1/cell-1/inputs".to_string(),
        helper: "/private/var/folders/xy/T/brokkr-seatbelt-probe-1/bin/seatbelt-probe-helper"
            .to_string(),
    }
}

fn fixed_profile(inputs: &CheckInputs) -> String {
    render_candidate_profile(&STARTUP_RULE_LEDGER, inputs)
}

#[test]
fn the_candidate_profile_equals_the_ledgers_disjoint_union() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    let result = check_startup_candidate(
        &profile,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    );
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn the_candidate_frame_is_exactly_version_then_deny_default() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    assert!(profile.starts_with("(version 1)\n(deny default)\n"));
    let units = super::ledger::parse_template(&profile).expect("parses");
    assert_eq!(
        units.units.len(),
        STARTUP_RULE_LEDGER.len(),
        "one unit per ledger entry"
    );
}

#[test]
fn every_ledger_entry_carries_its_typed_justification() {
    for entry in &*STARTUP_RULE_LEDGER {
        match &entry.class {
            LedgerClass::Baseline(baseline) => {
                assert!(!baseline.justification.trim().is_empty());
                assert!(matches!(
                    baseline.kind,
                    BaselineKind::HandsElement(_)
                        | BaselineKind::ExecutionInput
                        | BaselineKind::ProbeHarnessNeed
                ));
            }
            LedgerClass::DiagnosisAdmitted(admission) => {
                assert!(!admission.process.trim().is_empty());
                assert!(!admission.consumer.trim().is_empty());
                assert!(!admission.evidence.trim().is_empty());
                assert!(!admission.removal.trim().is_empty());
            }
        }
    }
}

#[test]
fn the_ledger_has_the_named_baseline_entries_and_one_diagnosis() {
    let baseline = STARTUP_RULE_LEDGER
        .iter()
        .filter(|entry| matches!(entry.class, LedgerClass::Baseline(_)))
        .count();
    let diagnosis = STARTUP_RULE_LEDGER
        .iter()
        .filter(|entry| matches!(entry.class, LedgerClass::DiagnosisAdmitted(_)))
        .count();
    assert_eq!(baseline, 23, "twenty-three baseline entries");
    assert_eq!(diagnosis, 1, "one diagnosis-admitted entry");
}

#[test]
fn the_program_binds_are_host_toolchain_sources() {
    for bind in PROGRAM_BINDS {
        assert!(
            HOST_TOOLCHAIN_BINDS.contains(&bind),
            "{bind} is not a host-toolchain source"
        );
    }
}

#[test]
fn the_fa7_template_keeps_a_disposition_for_every_unit() {
    check_fa7_dispositions().expect("every fa7 unit has a disposition");
}

#[test]
fn a_multi_operation_form_cannot_hide_an_operation() {
    let inputs = fixed_inputs();
    let mut profile = fixed_profile(&inputs);
    let needle = format!("(allow file-write* (subpath \"{}\"))", inputs.payload_root);
    profile = profile.replace(
        &needle,
        &format!(
            "(allow file-write* process-exec (subpath \"{}\"))",
            inputs.payload_root
        ),
    );
    let error = check_startup_candidate(
        &profile,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect_err("a hidden operation must fail");
    assert!(
        error.reason.contains("more than one operation"),
        "{}",
        error.reason
    );
    assert!(error.reason.contains("process-exec"), "{}", error.reason);
}

#[test]
fn a_top_level_form_other_than_the_frame_and_allow_fails() {
    let inputs = fixed_inputs();
    let base = fixed_profile(&inputs);
    for extra in [
        "(trace \"x\")\n",
        "(define \"x\" \"y\")\n",
        "(if true 1 2)\n",
        "(debug)\n",
        "(import \"x\")\n",
        "(param \"x\")\n",
        "(deny file-read*)\n",
        "(allow default)\n",
        "(version 1)\n",
        "bare-atom\n",
    ] {
        let profile = format!("{base}{extra}");
        let error = check_startup_candidate(
            &profile,
            &STARTUP_RULE_LEDGER,
            &STARTUP_NEGATIVE_ALLOWANCES,
            &inputs,
        )
        .expect_err("a non-allow top-level form must fail");
        assert!(!error.reason.is_empty());
    }
}

#[test]
fn modifiers_compound_filters_and_unparseable_text_fail() {
    let inputs = fixed_inputs();
    let unit = STARTUP_RULE_LEDGER[0].unit.render();
    assert_eq!(unit, "(allow process-fork)");
    let base = fixed_profile(&inputs);
    for (label, replacement) in [
        ("modifier", "(allow process-fork (with report))"),
        (
            "compound",
            "(allow process-fork (require-any (subpath \"/bin\")))",
        ),
        ("no-arg", "(allow process-fork (subpath))"),
        (
            "two-arg",
            "(allow process-fork (subpath \"/bin\" \"/sbin\"))",
        ),
    ] {
        let profile = base.replace("(allow process-fork)", replacement);
        let error = check_startup_candidate(
            &profile,
            &STARTUP_RULE_LEDGER,
            &STARTUP_NEGATIVE_ALLOWANCES,
            &inputs,
        )
        .expect_err(label);
        assert!(!error.reason.is_empty());
    }
    // A comment, an unbalanced parenthesis and a duplicated unit.
    for profile in [
        format!(";; comment\n{base}"),
        format!("{base}(allow process-fork\n"),
        format!("{base}(allow process-fork)\n"),
    ] {
        assert!(check_startup_candidate(
            &profile,
            &STARTUP_RULE_LEDGER,
            &STARTUP_NEGATIVE_ALLOWANCES,
            &inputs
        )
        .is_err());
    }
}

#[test]
fn a_duplicate_unit_fails() {
    let inputs = fixed_inputs();
    let base = fixed_profile(&inputs);
    let profile = format!("{base}(allow process-fork)\n");
    let error = check_startup_candidate(
        &profile,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect_err("a duplicate unit must fail");
    assert!(error.reason.contains("twice"), "{}", error.reason);
}

#[test]
fn an_unlisted_or_missing_unit_fails() {
    let inputs = fixed_inputs();
    let base = fixed_profile(&inputs);
    let extra = format!("{base}(allow sysctl-read)\n");
    let error = check_startup_candidate(
        &extra,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect_err("an unlisted unit must fail");
    assert!(error.reason.contains("neither half"), "{}", error.reason);

    let needle = STARTUP_RULE_LEDGER[0].unit.render();
    let missing = base.replace(&format!("{needle}\n"), "");
    let error = check_startup_candidate(
        &missing,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect_err("a missing unit must fail");
    assert!(error.reason.contains("lacks"), "{}", error.reason);
}

#[test]
fn the_removal_set_is_exactly_the_diagnosis_admitted_unit() {
    // The root-inode removal names exactly the diagnosis-admitted entry.
    let diagnosis: Vec<&LedgerEntry> = STARTUP_RULE_LEDGER
        .iter()
        .filter(|entry| matches!(entry.class, LedgerClass::DiagnosisAdmitted(_)))
        .collect();
    assert_eq!(diagnosis.len(), 1);
    let admission = match &diagnosis[0].class {
        LedgerClass::DiagnosisAdmitted(admission) => admission,
        _ => unreachable!(),
    };
    assert_eq!(admission.removal, STARTUP_NEGATIVE_ALLOWANCES[0].name);
    assert_eq!(
        diagnosis[0].unit.render(),
        STARTUP_NEGATIVE_ALLOWANCES[0].removed_rule
    );
}

#[test]
fn a_varied_ledger_that_adds_a_baseline_unit_is_judged() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    let mut ledger = STARTUP_RULE_LEDGER.clone();
    ledger.push(entry(
        unit("file-read*", "subpath", "/etc/ssh"),
        HandsElement::Toolchain,
        "smuggled",
    ));
    let error = check_startup_candidate(&profile, &ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs)
        .expect_err("a smuggled ledger unit fails its anchor");
    assert!(error.reason.contains("toolchain"), "{}", error.reason);
}

#[test]
fn the_concrete_inputs_are_validated_before_normalization() {
    let cell = "/private/var/folders/xy/T/brokkr-seatbelt-probe-1/cell-1";
    let helper = "/private/var/folders/xy/T/brokkr-seatbelt-probe-1/bin/seatbelt-probe-helper";
    let cases: [(&str, CheckInputs); 7] = [
        (
            "payload layout",
            CheckInputs {
                cell_root: cell.to_string(),
                payload_root: format!("{cell}/work"),
                inputs_dir: format!("{cell}/inputs"),
                helper: helper.to_string(),
            },
        ),
        (
            "inputs layout",
            CheckInputs {
                cell_root: cell.to_string(),
                payload_root: format!("{cell}/payload"),
                inputs_dir: "/tmp/inputs".to_string(),
                helper: helper.to_string(),
            },
        ),
        (
            "root cell",
            CheckInputs {
                cell_root: "/".to_string(),
                payload_root: "/payload".to_string(),
                inputs_dir: "/inputs".to_string(),
                helper: helper.to_string(),
            },
        ),
        (
            "usr parent",
            CheckInputs {
                cell_root: "/usr".to_string(),
                payload_root: "/usr/payload".to_string(),
                inputs_dir: "/usr/inputs".to_string(),
                helper: "/opt/helper".to_string(),
            },
        ),
        (
            "credential parent",
            CheckInputs {
                cell_root: "/private/etc".to_string(),
                payload_root: "/private/etc/payload".to_string(),
                inputs_dir: "/private/etc/inputs".to_string(),
                helper: "/opt/helper".to_string(),
            },
        ),
        (
            "host tmp parent",
            CheckInputs {
                cell_root: "/private/tmp".to_string(),
                payload_root: "/private/tmp/payload".to_string(),
                inputs_dir: "/private/tmp/inputs".to_string(),
                helper: "/opt/helper".to_string(),
            },
        ),
        (
            "data volume",
            CheckInputs {
                cell_root: "/System/Volumes/Data/private/var/folders/xy/T/cell".to_string(),
                payload_root: "/System/Volumes/Data/private/var/folders/xy/T/cell/payload"
                    .to_string(),
                inputs_dir: "/System/Volumes/Data/private/var/folders/xy/T/cell/inputs".to_string(),
                helper: "/System/Volumes/Data/opt/helper".to_string(),
            },
        ),
    ];
    for (label, inputs) in cases {
        let profile = fixed_profile(&inputs);
        let error = check_startup_candidate(
            &profile,
            &STARTUP_RULE_LEDGER,
            &STARTUP_NEGATIVE_ALLOWANCES,
            &inputs,
        )
        .expect_err(label);
        assert!(!error.reason.is_empty(), "{label}");
    }
}

#[test]
fn a_non_canonical_cell_root_or_helper_fails() {
    let good = fixed_inputs();
    for (label, cell, helper) in [
        ("relative", "cell", good.helper.as_str()),
        (
            "dotdot",
            "/private/var/folders/xy/T/../cell",
            good.helper.as_str(),
        ),
        (
            "trailing",
            "/private/var/folders/xy/T/cell/",
            good.helper.as_str(),
        ),
        (
            "var symlink",
            "/var/folders/xy/T/cell",
            good.helper.as_str(),
        ),
        ("tmp symlink", "/tmp/cell", good.helper.as_str()),
        (
            "helper under cell",
            good.cell_root.as_str(),
            good.payload_root.as_str(),
        ),
        ("helper root", good.cell_root.as_str(), "/"),
    ] {
        let inputs = CheckInputs {
            cell_root: cell.to_string(),
            payload_root: format!("{cell}/payload"),
            inputs_dir: format!("{cell}/inputs"),
            helper: helper.to_string(),
        };
        assert!(
            check_startup_candidate(
                &fixed_profile(&inputs),
                &STARTUP_RULE_LEDGER,
                &STARTUP_NEGATIVE_ALLOWANCES,
                &inputs,
            )
            .is_err(),
            "{label} must fail"
        );
    }
}

#[test]
fn a_placeholder_does_not_hide_its_concrete_target() {
    for (label, helper) in [
        ("program bind", "/usr/bin"),
        ("credential", "/private/etc/passwd"),
        (
            "data volume",
            "/System/Volumes/Data/Users/runner/seatbelt-probe-helper",
        ),
    ] {
        let mut inputs = fixed_inputs();
        inputs.helper = helper.to_string();
        let profile = fixed_profile(&inputs);
        let error = check_startup_candidate(
            &profile,
            &STARTUP_RULE_LEDGER,
            &STARTUP_NEGATIVE_ALLOWANCES,
            &inputs,
        )
        .expect_err(label);
        assert!(!error.reason.is_empty(), "{label}");
    }
}

#[test]
fn a_unit_that_covers_a_toolchain_spelling_or_control_target_fails() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    // A ledger unit relabelled as a probe-harness need but targeting `/usr`.
    let mut ledger = STARTUP_RULE_LEDGER.clone();
    if let Some(last) = ledger.iter_mut().find(|entry| {
        matches!(
            &entry.class,
            LedgerClass::Baseline(Baseline {
                kind: BaselineKind::ProbeHarnessNeed,
                ..
            })
        )
    }) {
        last.unit = unit("file-read*", "subpath", "/usr");
    }
    let error = check_startup_candidate(&profile, &ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs)
        .expect_err("a relabelled bind must fail");
    assert!(
        error.reason.contains("host-toolchain") || error.reason.contains("probe-harness"),
        "{}",
        error.reason
    );
}

#[test]
fn the_check_verdict_does_not_depend_on_hands_spec_home_or_git_facts() {
    use brokkr_protocol::hands::{box_argv, Bind, BindMode, GitFacts, HandsSpec};
    use std::path::{Path, PathBuf};

    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    let verdict = |ledger: &[LedgerEntry]| {
        check_startup_candidate(&profile, ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs).is_ok()
    };
    assert!(verdict(&STARTUP_RULE_LEDGER));

    let scratch = std::env::temp_dir().join(format!("brokkr-hands-test-{}", std::process::id()));
    let session = scratch.join("session");
    let varied: [(HandsSpec, PathBuf, GitFacts); 3] = [
        (
            HandsSpec::default(),
            PathBuf::from("/Users/runner"),
            GitFacts {
                common_dir: None,
                ..Default::default()
            },
        ),
        (
            HandsSpec {
                network: false,
                binds: vec![Bind {
                    path: "~/.rustup".to_string(),
                    mode: BindMode::Ro,
                    mask: Vec::new(),
                }],
            },
            PathBuf::from("/home/other"),
            GitFacts {
                common_dir: Some(PathBuf::from("/Users/runner/work/.git")),
                ..Default::default()
            },
        ),
        (
            HandsSpec {
                network: true,
                binds: vec![Bind {
                    path: "/opt/cache".to_string(),
                    mode: BindMode::Overlay,
                    mask: Vec::new(),
                }],
            },
            PathBuf::from("/Users/runner"),
            GitFacts {
                common_dir: Some(inputs.payload_root.clone().into()),
                ..Default::default()
            },
        ),
    ];
    for (spec, home, git) in varied {
        let argv = box_argv(
            &spec,
            Path::new("/work"),
            &home,
            &scratch,
            &session,
            &git,
            None,
            &["true".to_string()],
        )
        .expect("box argv");
        let mut sources: Vec<String> = argv
            .windows(2)
            .filter(|pair| pair[0] == "--ro-bind-try")
            .map(|pair| pair[1].clone())
            .collect();
        let mut expected: Vec<String> = HOST_TOOLCHAIN_BINDS
            .iter()
            .map(|source| source.to_string())
            .collect();
        if let Some(shared) = &git.common_dir {
            expected.push(shared.join("config").to_string_lossy().into_owned());
        }
        for bind in &spec.binds {
            if bind.mode == BindMode::Ro {
                let expanded = match bind.path.strip_prefix("~/") {
                    Some(rest) => home.join(rest),
                    None => PathBuf::from(&bind.path),
                };
                expected.push(expanded.to_string_lossy().into_owned());
            }
        }
        sources.sort();
        expected.sort();
        assert_eq!(
            sources, expected,
            "the --ro-bind-try sources must be exactly the host-toolchain set, the \
             home-expanded declared ro binds and <common>/config"
        );
        assert!(
            verdict(&STARTUP_RULE_LEDGER),
            "the check is independent of hands facts"
        );
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_declared_read_only_bind_is_not_a_toolchain_bind() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    let mut ledger = STARTUP_RULE_LEDGER.clone();
    let toolchain_slot = ledger
        .iter_mut()
        .find(|entry| {
            matches!(
                (&entry.class, &entry.unit.filter),
                (
                    LedgerClass::Baseline(Baseline {
                        kind: BaselineKind::HandsElement(HandsElement::Toolchain),
                        ..
                    }),
                    Some(_)
                )
            )
        })
        .expect("the ledger has a toolchain unit");
    toolchain_slot.unit = unit("file-read*", "subpath", "/Users/runner/.rustup");
    let error = check_startup_candidate(&profile, &ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs)
        .expect_err("a declared ro bind is not a toolchain bind");
    assert!(error.reason.contains("toolchain"), "{}", error.reason);
}

#[test]
fn the_git_common_config_is_not_a_toolchain_bind() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    let mut ledger = STARTUP_RULE_LEDGER.clone();
    let toolchain_slot = ledger
        .iter_mut()
        .find(|entry| {
            matches!(
                (&entry.class, &entry.unit.filter),
                (
                    LedgerClass::Baseline(Baseline {
                        kind: BaselineKind::HandsElement(HandsElement::Toolchain),
                        ..
                    }),
                    Some(_)
                )
            )
        })
        .expect("the ledger has a toolchain unit");
    toolchain_slot.unit = unit(
        "file-read*",
        "literal",
        "/Users/runner/work/brokkr/.git/config",
    );
    let error = check_startup_candidate(&profile, &ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs)
        .expect_err("the git common config is not a toolchain bind");
    assert!(error.reason.contains("toolchain"), "{}", error.reason);
}

#[test]
fn a_respelled_toolchain_target_fails_even_with_a_record() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    let mut ledger = STARTUP_RULE_LEDGER.clone();
    let toolchain_slot = ledger
        .iter_mut()
        .find(|entry| {
            matches!(
                (&entry.class, &entry.unit.filter),
                (
                    LedgerClass::Baseline(Baseline {
                        kind: BaselineKind::HandsElement(HandsElement::Toolchain),
                        ..
                    }),
                    Some(_)
                )
            )
        })
        .expect("the ledger has a toolchain unit");
    toolchain_slot.unit = unit("file-read*", "subpath", "/private/usr/bin");
    let error = check_startup_candidate(&profile, &ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs)
        .expect_err("a respelled toolchain target fails");
    assert!(error.reason.contains("toolchain"), "{}", error.reason);
}

#[test]
fn a_git_common_directory_under_the_payload_root_leaves_the_worktree_unit_valid() {
    // The check takes no GitFacts; the scenario is that a common dir under the
    // payload root never becomes a bind source the write unit covers.
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    check_startup_candidate(
        &profile,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect("a common dir under <payload-root> does not invalidate the write unit");
}

#[test]
fn the_data_volume_spelling_of_a_credential_control_stays_denied() {
    // The candidate carries no unit for the data-volume control target.
    let targets = super::ledger::path_denial_control_targets();
    assert!(targets.contains(&"/System/Volumes/Data/private/etc/passwd".to_string()));
    assert!(targets.contains(&"/etc/passwd".to_string()));
    assert!(!targets
        .iter()
        .any(|target| target == "/System/Volumes/Data/private/etc/hosts"));
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    check_startup_candidate(
        &profile,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect("the data-volume credential control is denied");
}

// ---------------------------------------------------------------------------
// The toolchain-respelling residual (task 1.25)
// ---------------------------------------------------------------------------

#[test]
fn a_respelled_toolchain_denial_event_is_detected_and_admits_nothing() {
    let events = vec![
        DenialEvent {
            operation: "file-read*".to_string(),
            path: "/usr/local/bin/cargo".to_string(),
        },
        DenialEvent {
            operation: "file-read*".to_string(),
            path: "/System/Volumes/Data/usr/local/bin/cargo".to_string(),
        },
        DenialEvent {
            operation: "process-exec".to_string(),
            path: "/private/etc/ssl/certs/ca.pem".to_string(),
        },
    ];
    let detected = detect_toolchain_respelling(&events);
    assert_eq!(detected.len(), 2);
    assert!(detected
        .iter()
        .any(|event| event.path == "/System/Volumes/Data/usr/local/bin/cargo"));
    assert!(detected
        .iter()
        .any(|event| event.path == "/private/etc/ssl/certs/ca.pem"));
    assert!(
        !detected
            .iter()
            .any(|event| event.path == "/usr/local/bin/cargo"),
        "the direct spelling is not a respelling"
    );

    // The residual names the exact finding and fails a cell that records it.
    let residual = StartupResidual::ToolchainRespelling { events: detected };
    assert_eq!(residual.name(), "SEATBELT-R3-STARTUP-toolchain-respelling");
    assert_eq!(residual.events().len(), 2);
    let cell = StartupCell::S1DirectSeatbelt;
    let mut observation = passing_startup(cell);
    observation.residual = Some(residual);
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("SEATBELT-R3-STARTUP-toolchain-respelling")));
}

#[test]
fn the_candidate_carries_no_dev_null_write_unit() {
    let inputs = fixed_inputs();
    let profile = fixed_profile(&inputs);
    assert!(
        !profile.contains("(allow file-write-data (literal \"/dev/null\"))"),
        "a /dev/null write enters only through a measured attribution"
    );
    check_startup_candidate(
        &profile,
        &STARTUP_RULE_LEDGER,
        &STARTUP_NEGATIVE_ALLOWANCES,
        &inputs,
    )
    .expect("the candidate carries no /dev/null write");
    // If it entered the baseline as a device-set unit, the operation anchor
    // refuses it.
    let mut ledger = STARTUP_RULE_LEDGER.clone();
    ledger[0].unit = unit("file-write-data", "literal", "/dev/null");
    ledger[0].class = LedgerClass::Baseline(Baseline {
        kind: BaselineKind::HandsElement(HandsElement::DeviceSet),
        justification: "smuggled".to_string(),
        correction: None,
    });
    let error = check_startup_candidate(&profile, &ledger, &STARTUP_NEGATIVE_ALLOWANCES, &inputs)
        .expect_err("a baseline /dev/null write fails the device-set anchor");
    assert!(error.reason.contains("device set"), "{}", error.reason);
}

#[test]
fn the_startup_stage_sequence_localizes_the_child_spawn() {
    assert_eq!(
        STARTUP_STAGES,
        [
            "entry",
            "payload-dir",
            "executable",
            "child-spawn",
            "child-observed",
            "ready",
            "return-clean",
        ]
    );
    // A cell that reached `executable` but not `child-spawn` is refused.
    let mut observation = passing_startup(StartupCell::S1DirectSeatbelt);
    observation.stages = vec![
        "entry".to_string(),
        "payload-dir".to_string(),
        "executable".to_string(),
    ];
    observation.ready = false;
    let verdict = evaluate_startup(&[StartupCell::S1DirectSeatbelt], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("startup stages")));
}

#[test]
fn a_respelled_helper_denial_event_records_its_own_residual() {
    let events = vec![DenialEvent {
        operation: "process-exec".to_string(),
        path: "/System/Volumes/Data/private/var/folders/xy/T/probe/bin/seatbelt-probe-helper"
            .to_string(),
    }];
    let residual = StartupResidual::HelperRespelling {
        events: events.clone(),
    };
    assert_eq!(residual.name(), "SEATBELT-R3-STARTUP-helper-respelling");
    assert_eq!(residual.events(), events.as_slice());
    let cell = StartupCell::S3LaunchdSeatbelt;
    let mut observation = passing_startup(cell);
    observation.residual = Some(residual);
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("SEATBELT-R3-STARTUP-helper-respelling")));
}
