//! Issue #1383 (operator ruling 2026-11-25), #1396 (operator ruling 2026-11-26)
//! and #1417 (operator ruling 2026-11-27): the macOS test shards leave the
//! pull-request path for nightly, manual dispatch and release runs, replaced on
//! pull requests and merge groups by `build (macos-latest)`.
//!
//! Every covered workflow is admitted once into `ClosedWorkflow` and evaluated
//! through `Verdict`, so refusals are total and final (decision 0071 rulings
//! 2, 3 and 8).

use super::workspace;
use std::collections::BTreeMap;

use crate::workflow::{
    ActionMatch, ClosedFormRefusal, ClosedJob, ClosedStep, ClosedWorkflow, ClosedWorkflowError,
    CondVal, ConditionContext, EventExpectation, RunsOn, Scalar, Verdict, WorkflowError,
    WorkflowInputType,
};

#[path = "macos_split/matrix_and_structured.rs"]
mod matrix_and_structured;
#[path = "macos_split/mutations.rs"]
mod mutations;
#[path = "macos_split/notify.rs"]
mod notify;

use notify::{check_nightly_workflow, ConcurrencyProblem, NotifyBehaviorProblem, NotifyEnvReason};

const SHARDS_WORKFLOW: &str = "./.github/workflows/macos-test-shards.yml";
const MACOS_SHARDS: [&str; 5] = ["macos/1", "macos/2", "macos/3", "macos/4", "macos/5"];
const CI_PR_TYPES: [&str; 3] = ["opened", "synchronize", "reopened"];
const CI_MG_TYPES: [&str; 1] = ["checks_requested"];
const RELEASE_TAGS: [&str; 1] = ["v*"];
const BUILD_MACOS_SCRIPT: &str = "cargo test --workspace --all-features --locked --no-run";
const VERDICT_SCRIPT: &str = "test \"$MACHINES\" = success";
const SHARD_RUN_SCRIPT: &str = "bash scripts/test-shards.sh test --parallel-heaps";
const GATE_A_SCRIPT: &str = "set -euo pipefail\nsw_vers\nuname -m\ntest -x /usr/bin/sandbox-exec\ncargo test --locked -p brokkr-seatbelt-probe --test seatbelt_lifetime_probe \\\n  -- --ignored --exact --nocapture \\\n  seatbelt_probe::native::native_startup_feasibility_probe \\\n  2>&1 | tee target/r3-startup.log\ngrep -Eq 'test result: ok\\. 1 passed' target/r3-startup.log\ngrep -q 'Gate A startup verdict' target/r3-startup-report.txt\n";
const BUNDLES_SCRIPT: &str = "cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\ncargo run --locked -p brokkr-cli -- library compile --bundle recipes/verify\n";

/// Typed guard problem over the four covered macOS-split workflows (decision 0071 rulings 2 and 8).
#[derive(Debug, Clone, PartialEq, Eq)]
enum SplitProblem {
    UnreadableWorkflow {
        file: String,
        err: WorkflowError,
    },
    ClosedForm(ClosedFormRefusal),
    MissingJob(WorkflowError),
    CiMissingTrigger {
        event: String,
    },
    CiJobMentionsMacos {
        job_id: String,
    },
    CiEngineWrongName,
    CiIssuesWriteGranted,
    CiJobAllowsFailure {
        job_id: String,
    },
    CiDispatchJobMissingDispatch {
        job_id: String,
    },
    CiDispatchJobRunsOnEvent {
        job_id: String,
        event: String,
    },
    BuildMacosWrongName,
    BuildMacosWrongEventGate,
    BuildMacosWrongRunner,
    BuildMacosMissingCheckout,
    BuildMacosMissingToolchain,
    BuildMacosMissingCache,
    BuildMacosMissingCompile,
    BuildMacosUnexpectedRunStep {
        step_index: usize,
    },
    ShardsInvalidEvents,
    ShardsMissingUseCacheInput,
    ShardsUseCacheWrongType,
    ShardsUseCacheWrongDefault,
    ShardsWrongRunner,
    ShardsWrongMatrix,
    ShardsJobAllowsFailure,
    ShardsMissingCheckout,
    ShardsMissingToolchain,
    ShardsSetupOrderInvalid,
    ShardsMissingShardStep,
    ShardsMissingGateA,
    ShardsCacheStepNotGated {
        step_index: usize,
    },
    ShardsMissingRustCache,
    ShardsMissingBundles,
    CallerCopiedShardList {
        file: String,
    },
    CallerWrongUses {
        file: String,
    },
    CallerJobAllowsFailure {
        file: String,
        job_id: String,
    },
    CallerMachinesEventMismatch {
        file: String,
        event: String,
    },
    CallerUseCacheMismatch {
        file: String,
        actual: bool,
        expected: bool,
    },
    CallerUseCacheNonBool {
        file: String,
        raw: Scalar,
    },
    CallerVerdictWrongName {
        file: String,
    },
    CallerVerdictMissingNeeds {
        file: String,
    },
    CallerVerdictMissingCheck {
        file: String,
    },
    ReleaseJobMissingMacosTestsNeeds {
        job_id: String,
    },
    NightlyInvalidEvents,
    NightlyConcurrency(ConcurrencyProblem),
    NotifyMissingMacosTestsNeeds,
    NotifyTooManyNeeds(usize),
    NotifyGateMismatch {
        event: String,
        verdict_result: String,
    },
    NotifyWrongPermissions,
    NotifyJobAllowsFailure,
    NotifyJobSkippedOnFailure,
    NotifyStepAllowsFailure {
        step_index: usize,
    },
    NotifyEnvError {
        key: String,
        reason: NotifyEnvReason,
    },
    NotifyMissingScriptStep,
    NotifyExtraScriptStep {
        step_index: usize,
    },
    NotifyScriptChanged,
    NotifyBehavior(NotifyBehaviorProblem),
}

impl From<ClosedFormRefusal> for SplitProblem {
    fn from(refusal: ClosedFormRefusal) -> Self {
        Self::ClosedForm(refusal)
    }
}

fn parse_or_problem(
    file: &str,
    text: &str,
    problems: &mut Vec<SplitProblem>,
) -> Option<ClosedWorkflow> {
    match ClosedWorkflow::parse(file, text) {
        Ok(wf) => Some(wf),
        Err(ClosedWorkflowError::Parse { file, err }) => {
            problems.push(SplitProblem::UnreadableWorkflow { file, err });
            None
        }
        Err(ClosedWorkflowError::Refusal(refusal)) => {
            problems.push(SplitProblem::ClosedForm(refusal));
            None
        }
    }
}

fn is_macos_runner_label(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    lower == "macos" || lower.starts_with("macos-")
}

fn job_runs_on_macos(job: &ClosedJob) -> bool {
    job.runs_on().is_some_and(|runs_on| {
        runs_on
            .possible_labels(job.matrix())
            .iter()
            .any(|label| is_macos_runner_label(label))
    })
}

fn is_macos_shard_value(val: &Scalar) -> bool {
    val.as_str().is_some_and(|s| {
        MACOS_SHARDS
            .iter()
            .any(|shard| s.eq_ignore_ascii_case(shard))
    })
}

fn caller_copied_shard_list(wf: &ClosedWorkflow) -> bool {
    wf.jobs.iter().any(|job| {
        let in_matrix = job.matrix().is_some_and(|matrix| {
            matrix.axes.values().flatten().any(is_macos_shard_value)
                || matrix
                    .include
                    .iter()
                    .flat_map(BTreeMap::values)
                    .any(is_macos_shard_value)
        });
        let in_reusable_with = job.uses().is_some()
            && job
                .with()
                .is_some_and(|with| with.values().any(is_macos_shard_value));
        in_matrix || in_reusable_with
    })
}

fn shard_ctx(machine: &str, use_cache: bool) -> ConditionContext {
    ConditionContext::default_ok()
        .with_matrix("machine", CondVal::Str(machine.to_string()))
        .with_input("use-cache", CondVal::Bool(use_cache))
}

fn step_enforces_all_shards(job: &ClosedJob, step: &ClosedStep) -> Verdict {
    Verdict::all(MACOS_SHARDS.iter().flat_map(|machine| {
        [false, true].into_iter().map(move |use_cache| {
            let ctx = shard_ctx(machine, use_cache);
            job.step_enforces(step, &ctx, &ctx)
        })
    }))
}

fn first_matching_index(steps: &[ClosedStep], pred: impl Fn(&ClosedStep) -> bool) -> Option<usize> {
    steps.iter().position(pred)
}

/// Check C1 and C2 on `ci.yml`.
fn ci_macos_split_problems(ci: &str) -> Vec<SplitProblem> {
    let mut problems = Vec::new();
    let Some(wf) = parse_or_problem("ci.yml", ci, &mut problems) else {
        return problems;
    };
    for spec in [
        EventExpectation::WithTypes {
            name: "pull_request",
            types: &CI_PR_TYPES,
        },
        EventExpectation::WithTypes {
            name: "merge_group",
            types: &CI_MG_TYPES,
        },
    ] {
        if !wf
            .event(spec.event_name())
            .is_some_and(|ev| spec.matches_trigger(ev))
        {
            problems.push(SplitProblem::CiMissingTrigger {
                event: spec.event_name().to_string(),
            });
        }
    }
    match wf.require_job("ci.yml", "engine-machines") {
        Ok(job) if job_runs_on_macos(job) => {
            problems.push(SplitProblem::CiJobMentionsMacos {
                job_id: "engine-machines".to_string(),
            });
        }
        Ok(_) => {}
        Err(err) => problems.push(SplitProblem::MissingJob(err)),
    }
    match wf.require_job("ci.yml", "engine") {
        Ok(job) => {
            if job.name() != Some("test (ubuntu-latest)") {
                problems.push(SplitProblem::CiEngineWrongName);
            }
            if job_runs_on_macos(job) {
                problems.push(SplitProblem::CiJobMentionsMacos {
                    job_id: "engine".to_string(),
                });
            }
        }
        Err(err) => problems.push(SplitProblem::MissingJob(err)),
    }
    check_ci_build_macos(&wf, &mut problems);
    check_ci_dispatch_only_macos(&wf, &mut problems);
    if wf.grants_permission("issues", "write") {
        problems.push(SplitProblem::CiIssuesWriteGranted);
    }
    problems
}

fn check_ci_dispatch_only_macos(wf: &ClosedWorkflow, problems: &mut Vec<SplitProblem>) {
    for (id, run_on_prior_failure) in [("macos-machines", false), ("macos-tests", true)] {
        let job = match wf.require_job("ci.yml", id) {
            Ok(job) => job,
            Err(err) => {
                problems.push(SplitProblem::MissingJob(err));
                continue;
            }
        };
        if job.allows_failure() {
            problems.push(SplitProblem::CiJobAllowsFailure {
                job_id: id.to_string(),
            });
        }
        let base_ctx = ConditionContext::default_ok().with_prior_succeeded(!run_on_prior_failure);
        if let Some(p) = wf
            .event_reaches_job(
                job,
                &EventExpectation::Plain("workflow_dispatch"),
                &base_ctx,
            )
            .expect_true(|| SplitProblem::CiDispatchJobMissingDispatch {
                job_id: id.to_string(),
            })
        {
            problems.push(p);
        }
        for ev in wf.events() {
            if ev.name == "workflow_dispatch" {
                continue;
            }
            let ctx = base_ctx.clone().with_event(&ev.name);
            if let Some(p) =
                job.runs_under(&ctx)
                    .expect_false(|| SplitProblem::CiDispatchJobRunsOnEvent {
                        job_id: id.to_string(),
                        event: ev.name.clone(),
                    })
            {
                problems.push(p);
            }
        }
    }
}

fn check_ci_build_macos(wf: &ClosedWorkflow, problems: &mut Vec<SplitProblem>) {
    let job = match wf.require_job("ci.yml", "build-macos") {
        Ok(job) => job,
        Err(err) => {
            problems.push(SplitProblem::MissingJob(err));
            return;
        }
    };
    if job.name() != Some("build (macos-latest)") {
        problems.push(SplitProblem::BuildMacosWrongName);
    }
    if job.allows_failure() {
        problems.push(SplitProblem::CiJobAllowsFailure {
            job_id: "build-macos".to_string(),
        });
    }
    check_ci_build_macos_events(wf, job, problems);
    if job.runs_on() != Some(&RunsOn::Single("macos-latest".to_string())) {
        problems.push(SplitProblem::BuildMacosWrongRunner);
    }
    check_ci_build_macos_steps(job, problems);
}

fn check_ci_build_macos_events(
    wf: &ClosedWorkflow,
    job: &ClosedJob,
    problems: &mut Vec<SplitProblem>,
) {
    let pr_spec = EventExpectation::WithTypes {
        name: "pull_request",
        types: &CI_PR_TYPES,
    };
    let mg_spec = EventExpectation::WithTypes {
        name: "merge_group",
        types: &CI_MG_TYPES,
    };
    let ok_ctx = ConditionContext::default_ok();
    let push_ctx = ok_ctx.clone().with_event("push");
    let event_gate = wf
        .event_reaches_job(job, &pr_spec, &ok_ctx)
        .and(wf.event_reaches_job(job, &mg_spec, &ok_ctx))
        .and(job.runs_under(&push_ctx).not());
    if let Some(p) = event_gate.expect_true(|| SplitProblem::BuildMacosWrongEventGate) {
        problems.push(p);
    }
}

fn check_ci_build_macos_steps(job: &ClosedJob, problems: &mut Vec<SplitProblem>) {
    let pr_ctx = ConditionContext::default_ok().with_event("pull_request");
    let mg_ctx = ConditionContext::default_ok().with_event("merge_group");
    let step_enforces_pr_mg = |s: &ClosedStep| {
        job.step_enforces(s, &pr_ctx, &pr_ctx)
            .and(job.step_enforces(s, &mg_ctx, &mg_ctx))
    };

    let checkout_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| matches!(s.action_match("actions/checkout"), Some(ActionMatch::Exact)))
            .map(step_enforces_pr_mg),
    );
    if let Some(p) = checkout_v.expect_true(|| SplitProblem::BuildMacosMissingCheckout) {
        problems.push(p);
    }

    let toolchain_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| {
                matches!(
                    s.action_match("dtolnay/rust-toolchain"),
                    Some(ActionMatch::Exact)
                ) && s.with_literal("toolchain").is_some()
            })
            .map(step_enforces_pr_mg),
    );
    if let Some(p) = toolchain_v.expect_true(|| SplitProblem::BuildMacosMissingToolchain) {
        problems.push(p);
    }

    let cache_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| {
                matches!(
                    s.action_match("Swatinem/rust-cache"),
                    Some(ActionMatch::Exact)
                )
            })
            .map(step_enforces_pr_mg),
    );
    if let Some(p) = cache_v.expect_true(|| SplitProblem::BuildMacosMissingCache) {
        problems.push(p);
    }

    let compile_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| s.run() == Some(BUILD_MACOS_SCRIPT))
            .map(step_enforces_pr_mg),
    );
    if let Some(p) = compile_v.expect_true(|| SplitProblem::BuildMacosMissingCompile) {
        problems.push(p);
    }

    for (idx, step) in job.steps.iter().enumerate() {
        if let Some(script) = step.run() {
            if script != BUILD_MACOS_SCRIPT {
                problems.push(SplitProblem::BuildMacosUnexpectedRunStep {
                    step_index: idx + 1,
                });
            }
        }
    }
}

/// Check the single reusable shard workflow `macos-test-shards.yml` (C3).
fn check_reusable_shards(shards: &str, problems: &mut Vec<SplitProblem>) {
    let Some(wf) = parse_or_problem("macos-test-shards.yml", shards, problems) else {
        return;
    };
    if wf.event_names() != ["workflow_call"] || wf.unfiltered_event("workflow_call").is_none() {
        problems.push(SplitProblem::ShardsInvalidEvents);
    }
    match wf.workflow_call_input("use-cache") {
        Some(input) => {
            if input.input_type != WorkflowInputType::Boolean {
                problems.push(SplitProblem::ShardsUseCacheWrongType);
            }
            if input.default.as_ref().and_then(Scalar::as_bool) != Some(false) {
                problems.push(SplitProblem::ShardsUseCacheWrongDefault);
            }
        }
        None => problems.push(SplitProblem::ShardsMissingUseCacheInput),
    }
    match wf.require_job("macos-test-shards.yml", "macos-machines") {
        Ok(job) => check_reusable_shard_job(job, problems),
        Err(err) => problems.push(SplitProblem::MissingJob(err)),
    }
}

fn is_cache_step(step: &ClosedStep) -> bool {
    matches!(
        step.action_match("Swatinem/rust-cache"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    ) || matches!(
        step.action_match("actions/cache"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    )
}

fn check_shard_cache_steps(job: &ClosedJob, problems: &mut Vec<SplitProblem>) {
    let mut saw_rust_cache = false;
    for (idx, step) in job.steps.iter().enumerate() {
        if !is_cache_step(step) {
            continue;
        }
        if matches!(
            step.action_match("Swatinem/rust-cache"),
            Some(ActionMatch::Exact)
        ) {
            saw_rust_cache = true;
        }
        let gated = Verdict::all(MACOS_SHARDS.iter().map(|m| {
            let (on, off) = (shard_ctx(m, true), shard_ctx(m, false));
            job.runs_under(&on)
                .and(step.runs_under(&on))
                .and(step.runs_under(&off).not())
        }));
        if let Some(p) = gated.expect_true(|| SplitProblem::ShardsCacheStepNotGated {
            step_index: idx + 1,
        }) {
            problems.push(p);
        }
    }
    if !saw_rust_cache {
        problems.push(SplitProblem::ShardsMissingRustCache);
    }
}

fn check_shard_setup_steps(job: &ClosedJob, problems: &mut Vec<SplitProblem>) {
    let is_checkout =
        |s: &ClosedStep| matches!(s.action_match("actions/checkout"), Some(ActionMatch::Exact));
    let is_toolchain = |s: &ClosedStep| {
        matches!(
            s.action_match("dtolnay/rust-toolchain"),
            Some(ActionMatch::Exact)
        ) && s.with_literal("toolchain").is_some()
    };

    let checkout_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| is_checkout(s))
            .map(|s| step_enforces_all_shards(job, s)),
    );
    if let Some(p) = checkout_v.expect_true(|| SplitProblem::ShardsMissingCheckout) {
        problems.push(p);
    }

    let toolchain_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| is_toolchain(s))
            .map(|s| step_enforces_all_shards(job, s)),
    );
    if let Some(p) = toolchain_v.expect_true(|| SplitProblem::ShardsMissingToolchain) {
        problems.push(p);
    }

    let checkout_idx = first_matching_index(&job.steps, is_checkout);
    let toolchain_idx = first_matching_index(&job.steps, is_toolchain);
    let first_work_idx = job.steps.iter().position(|s| {
        is_cache_step(s)
            || matches!(
                s.run(),
                Some(SHARD_RUN_SCRIPT | GATE_A_SCRIPT | BUNDLES_SCRIPT)
            )
    });
    if let (Some(co), Some(tc)) = (checkout_idx, toolchain_idx) {
        if co >= tc || first_work_idx.is_some_and(|w| tc >= w) {
            problems.push(SplitProblem::ShardsSetupOrderInvalid);
        }
    }
}

fn check_reusable_shard_job(job: &ClosedJob, problems: &mut Vec<SplitProblem>) {
    if job.runs_on() != Some(&RunsOn::Single("macos-latest".to_string())) {
        problems.push(SplitProblem::ShardsWrongRunner);
    }
    match job.matrix() {
        None => problems.push(SplitProblem::ShardsWrongMatrix),
        Some(matrix) => {
            if matrix.axes.len() != 1
                || matrix.axis_strings("machine").as_deref() != Some(MACOS_SHARDS.as_slice())
            {
                problems.push(SplitProblem::ShardsWrongMatrix);
            }
        }
    }
    if job.allows_failure() {
        problems.push(SplitProblem::ShardsJobAllowsFailure);
    }
    check_shard_setup_steps(job, problems);
    let shard_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| {
                s.run() == Some(SHARD_RUN_SCRIPT)
                    && s.env_value("BROKKR_TEST_MACHINE") == Some("${{ matrix.machine }}")
            })
            .map(|s| step_enforces_all_shards(job, s)),
    );
    if let Some(p) = shard_v.expect_true(|| SplitProblem::ShardsMissingShardStep) {
        problems.push(p);
    }
    let gate_a_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| s.run() == Some(GATE_A_SCRIPT))
            .map(|s| {
                let runs_on_one = Verdict::all([false, true].into_iter().map(|c| {
                    let ctx = shard_ctx("macos/1", c);
                    job.step_enforces(s, &ctx, &ctx)
                }));
                let skipped_others = Verdict::all(
                    MACOS_SHARDS[1..]
                        .iter()
                        .map(|m| s.runs_under(&shard_ctx(m, false)).not()),
                );
                runs_on_one.and(skipped_others)
            }),
    );
    if let Some(p) = gate_a_v.expect_true(|| SplitProblem::ShardsMissingGateA) {
        problems.push(p);
    }
    check_shard_cache_steps(job, problems);
    let bundles_v = Verdict::any(
        job.steps
            .iter()
            .filter(|s| s.run() == Some(BUNDLES_SCRIPT))
            .map(|s| step_enforces_all_shards(job, s)),
    );
    if let Some(p) = bundles_v.expect_true(|| SplitProblem::ShardsMissingBundles) {
        problems.push(p);
    }
}

fn check_shard_caller(
    file: &str,
    text: &str,
    wants_cache: bool,
    events: &[EventExpectation<'_>],
    problems: &mut Vec<SplitProblem>,
) {
    let Some(wf) = parse_or_problem(file, text, problems) else {
        return;
    };
    if caller_copied_shard_list(&wf) {
        problems.push(SplitProblem::CallerCopiedShardList {
            file: file.to_string(),
        });
    }
    match wf.require_job(file, "macos-machines") {
        Ok(job) => check_caller_machines(file, &wf, job, wants_cache, events, problems),
        Err(err) => problems.push(SplitProblem::MissingJob(err)),
    }
    match wf.require_job(file, "macos-tests") {
        Ok(job) => check_caller_verdict(file, &wf, job, events, problems),
        Err(err) => problems.push(SplitProblem::MissingJob(err)),
    }
}

fn check_caller_machines(
    file: &str,
    wf: &ClosedWorkflow,
    job: &ClosedJob,
    wants_cache: bool,
    events: &[EventExpectation<'_>],
    problems: &mut Vec<SplitProblem>,
) {
    if job.uses() != Some(SHARDS_WORKFLOW) {
        problems.push(SplitProblem::CallerWrongUses {
            file: file.to_string(),
        });
    }
    if job.allows_failure() {
        problems.push(SplitProblem::CallerJobAllowsFailure {
            file: file.to_string(),
            job_id: "macos-machines".to_string(),
        });
    }
    let ok = ConditionContext::default_ok();
    for ev in events {
        if let Some(p) = wf.event_reaches_job(job, ev, &ok).expect_true(|| {
            SplitProblem::CallerMachinesEventMismatch {
                file: file.to_string(),
                event: ev.event_name().to_string(),
            }
        }) {
            problems.push(p);
        }
    }
    match job.with_value("use-cache") {
        None => {
            if wants_cache {
                problems.push(SplitProblem::CallerUseCacheMismatch {
                    file: file.to_string(),
                    actual: false,
                    expected: wants_cache,
                });
            }
        }
        Some(val) => match val.as_bool() {
            Some(enables_cache) if enables_cache != wants_cache => {
                problems.push(SplitProblem::CallerUseCacheMismatch {
                    file: file.to_string(),
                    actual: enables_cache,
                    expected: wants_cache,
                });
            }
            Some(_) => {}
            None => problems.push(SplitProblem::CallerUseCacheNonBool {
                file: file.to_string(),
                raw: val.clone(),
            }),
        },
    }
}

fn check_caller_verdict(
    file: &str,
    wf: &ClosedWorkflow,
    job: &ClosedJob,
    events: &[EventExpectation<'_>],
    problems: &mut Vec<SplitProblem>,
) {
    if job.name() != Some("test (macos-latest)") {
        problems.push(SplitProblem::CallerVerdictWrongName {
            file: file.to_string(),
        });
    }
    if !job.needs().iter().any(|n| n == "macos-machines") {
        problems.push(SplitProblem::CallerVerdictMissingNeeds {
            file: file.to_string(),
        });
    }
    if job.allows_failure() {
        problems.push(SplitProblem::CallerJobAllowsFailure {
            file: file.to_string(),
            job_id: "macos-tests".to_string(),
        });
    }
    let runs_verdict = Verdict::any(
        job.steps
            .iter()
            .filter(|s| {
                s.run() == Some(VERDICT_SCRIPT)
                    && s.env_value("MACHINES") == Some("${{ needs.macos-machines.result }}")
            })
            .map(|s| {
                Verdict::all(events.iter().map(|ev| {
                    let job_ctx = ConditionContext::default_ok().with_prior_succeeded(false);
                    let step_ctx = ConditionContext::default_ok().with_event(ev.event_name());
                    Verdict::from_bool(!s.allows_failure())
                        .and(wf.event_reaches_job(job, ev, &job_ctx))
                        .and(s.runs_under(&step_ctx))
                }))
            }),
    );
    if let Some(p) = runs_verdict.expect_true(|| SplitProblem::CallerVerdictMissingCheck {
        file: file.to_string(),
    }) {
        problems.push(p);
    }
}

#[derive(Clone)]
struct ShardWorkflows {
    shards: String,
    nightly: String,
    ci: String,
    release: String,
}

impl ShardWorkflows {
    fn load() -> Self {
        let root = workspace();
        let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
        Self {
            shards: read(".github/workflows/macos-test-shards.yml"),
            nightly: read(".github/workflows/macos-tests.yml"),
            ci: read(".github/workflows/ci.yml"),
            release: read(".github/workflows/release.yml"),
        }
    }

    fn problems(&self) -> Vec<SplitProblem> {
        let mut problems = Vec::new();
        check_reusable_shards(&self.shards, &mut problems);
        check_nightly_workflow(&self.nightly, &mut problems);
        let nightly_events = [
            EventExpectation::ActiveSchedule,
            EventExpectation::Plain("workflow_dispatch"),
        ];
        let ci_events = [EventExpectation::Plain("workflow_dispatch")];
        let release_events = [EventExpectation::PushTags(&RELEASE_TAGS)];
        for (file, text, wants_cache, events) in [
            (
                "macos-tests.yml",
                self.nightly.as_str(),
                true,
                &nightly_events[..],
            ),
            ("ci.yml", self.ci.as_str(), true, &ci_events[..]),
            (
                "release.yml",
                self.release.as_str(),
                false,
                &release_events[..],
            ),
        ] {
            check_shard_caller(file, text, wants_cache, events, &mut problems);
        }
        if let Some(release_wf) = parse_or_problem("release.yml", &self.release, &mut problems) {
            for id in ["build", "publish"] {
                match release_wf.require_job("release.yml", id) {
                    Ok(job) if !job.needs().iter().any(|n| n == "macos-tests") => {
                        problems.push(SplitProblem::ReleaseJobMissingMacosTestsNeeds {
                            job_id: id.to_string(),
                        });
                    }
                    Ok(_) => {}
                    Err(err) => problems.push(SplitProblem::MissingJob(err)),
                }
            }
        }
        problems
    }

    fn with_nightly(&self, nightly: String) -> Self {
        Self {
            nightly,
            ..self.clone()
        }
    }
    fn with_shards(&self, shards: String) -> Self {
        Self {
            shards,
            ..self.clone()
        }
    }
    fn with_ci(&self, ci: String) -> Self {
        Self { ci, ..self.clone() }
    }
    fn with_release(&self, release: String) -> Self {
        Self {
            release,
            ..self.clone()
        }
    }
}
