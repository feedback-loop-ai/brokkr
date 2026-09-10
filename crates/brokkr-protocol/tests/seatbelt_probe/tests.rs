//! Host-independent tests for the bounded R3 probe model. They exercise the
//! shared decision logic on Linux; they are not native macOS evidence and
//! close no SEATBELT residual.

use super::*;

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
            c.launchd_crashes = Some(1)
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
    observation.negative_controls[0].blocked = false;
    let verdict = evaluate_startup(&[cell], &[observation]);
    assert!(!verdict.is_pass());
    assert!(verdict
        .reasons()
        .iter()
        .any(|reason| reason.contains("load-bearing negative control")));

    // A removal that never ran proves nothing either.
    let mut observation = passing_startup(cell);
    observation.negative_controls[0].observed = false;
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
    let complete = "state = not running\nruns = 1\nsuccessive crashes = 0\n";
    let facts = parse_launchd_print(complete).expect("complete launchd state");
    assert_eq!(facts.state, "not running");
    assert_eq!(facts.runs, 1);
    assert_eq!(facts.crashes, 0);
    assert_eq!(classify_launchd_exit(&facts), HelperExit::Clean);

    // A missing or unparsable field is an error, not a default.
    assert!(parse_launchd_print("state = not running\nsuccessive crashes = 0\n").is_err());
    assert!(parse_launchd_print("runs = 1\nsuccessive crashes = 0\n").is_err());
    assert!(
        parse_launchd_print("state = not running\nruns = one\nsuccessive crashes = 0\n").is_err()
    );

    // No terminal observation never becomes a synthetic nonzero exit.
    let running = "state = running\nruns = 0\nsuccessive crashes = 0\n";
    let facts = parse_launchd_print(running).expect("parseable running state");
    assert_eq!(classify_launchd_exit(&facts), HelperExit::NotRun);
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
