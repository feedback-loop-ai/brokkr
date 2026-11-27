//! Issue #1383 (operator ruling 2026-11-25) and #1396 (operator ruling
//! 2026-11-26): the macOS test shards leave the pull-request path for nightly,
//! manual dispatch and release runs, replaced on pull requests and merge groups
//! by `build (macos-latest)`. Every structural workflow question goes through
//! `crate::workflow::Workflow` (backed by `yaml-rust2`).

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use super::workspace;

use crate::workflow::{
    ActionMatch, ClosedCondition, CondVal, ConditionContext, EventExpectation, EventTrigger, Job,
    Permissions, RefusalKind, RunsOn, Scalar, Step, Workflow, WorkflowError,
};

const SHARDS_WORKFLOW: &str = "./.github/workflows/macos-test-shards.yml";
const MACOS_SHARDS: [&str; 5] = ["macos/1", "macos/2", "macos/3", "macos/4", "macos/5"];
const CI_PR_TYPES: [&str; 3] = ["opened", "synchronize", "reopened"];
const CI_MG_TYPES: [&str; 1] = ["checks_requested"];
const RELEASE_TAGS: [&str; 1] = ["v*"];
const VERDICT_SCRIPT: &str = "test \"$MACHINES\" = success";
const GATE_A_SCRIPT: &str = "set -euo pipefail\nsw_vers\nuname -m\ntest -x /usr/bin/sandbox-exec\ncargo test --locked -p brokkr-seatbelt-probe --test seatbelt_lifetime_probe \\\n  -- --ignored --exact --nocapture \\\n  seatbelt_probe::native::native_startup_feasibility_probe \\\n  2>&1 | tee target/r3-startup.log\ngrep -Eq 'test result: ok\\. 1 passed' target/r3-startup.log\ngrep -q 'Gate A startup verdict' target/r3-startup-report.txt\n";
const BUNDLES_SCRIPT: &str = "cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\ncargo run --locked -p brokkr-cli -- library compile --bundle recipes/verify\n";

fn parse_or_problem(file: &str, text: &str, problems: &mut Vec<String>) -> Option<Workflow> {
    match Workflow::parse_closed_form(file, text) {
        Ok(wf) => Some(wf),
        Err(err) => {
            problems.push(err.to_string());
            None
        }
    }
}

fn job_mentions_macos(job: &Job) -> bool {
    job.all_scalars
        .iter()
        .any(|scalar| scalar.to_ascii_lowercase().contains("macos"))
}

fn shard_ctx(machine: &str, use_cache: bool) -> ConditionContext {
    ConditionContext::default_ok()
        .with_matrix("machine", CondVal::Str(machine.to_string()))
        .with_input("use-cache", CondVal::Bool(use_cache))
}

fn step_enforces_all_shards(file: &str, job: &Job, step: &Step, step_idx: usize) -> bool {
    MACOS_SHARDS.iter().all(|machine| {
        [false, true].into_iter().all(|use_cache| {
            let ctx = shard_ctx(machine, use_cache);
            job.step_enforces(file, step, step_idx, &ctx, &ctx) == Ok(true)
        })
    })
}

/// Check C1 and C2 on `ci.yml`: no macOS test shard runs on `pull_request` or
/// `merge_group`, `engine` judges Linux only, and `build-macos` (`build
/// (macos-latest)`) compiles the whole workspace and test targets on macOS
/// without running tests.
fn ci_macos_split_problems(ci: &str) -> Vec<String> {
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
            problems.push(format!(
                "ci.yml does not run unfiltered on {} with expected types",
                spec.event_name()
            ));
        }
    }
    match wf.require_job("ci.yml", "engine-machines") {
        Ok(job) if job_mentions_macos(job) => {
            problems.push("engine-machines still mentions macOS".to_string());
        }
        Ok(_) => {}
        Err(err) => problems.push(err.to_string()),
    }
    match wf.require_job("ci.yml", "engine") {
        Ok(job) => {
            if job.name.as_deref() != Some("test (ubuntu-latest)") {
                problems.push("engine's name is not `test (ubuntu-latest)`".to_string());
            }
            if job_mentions_macos(job) {
                problems.push("engine still mentions macOS".to_string());
            }
        }
        Err(err) => problems.push(err.to_string()),
    }
    check_ci_build_macos(&wf, &mut problems);
    check_ci_dispatch_only_macos(&wf, &mut problems);
    if wf.grants_permission("issues", "write") {
        problems.push("ci.yml gained `issues: write` on the pull-request path".to_string());
    }
    problems
}

fn check_ci_dispatch_only_macos(wf: &Workflow, problems: &mut Vec<String>) {
    for (id, run_on_prior_failure) in [("macos-machines", false), ("macos-tests", true)] {
        let job = match wf.require_job("ci.yml", id) {
            Ok(job) => job,
            Err(err) => {
                problems.push(err.to_string());
                continue;
            }
        };
        if job.allows_failure() {
            problems.push(format!(
                "ci.yml's {id} allows failure via continue-on-error"
            ));
        }
        let dispatch_ok = wf.event_reaches_job(
            "ci.yml",
            job,
            &EventExpectation::Plain("workflow_dispatch"),
            &ConditionContext::default_ok().with_prior_succeeded(!run_on_prior_failure),
        ) == Ok(true);
        let skipped_elsewhere = ["pull_request", "merge_group", "push"]
            .into_iter()
            .all(|ev| {
                job.runs_under(
                    "ci.yml",
                    &ConditionContext::default_ok()
                        .with_event(ev)
                        .with_prior_succeeded(!run_on_prior_failure),
                ) == Ok(false)
            });
        if !dispatch_ok || !skipped_elsewhere {
            problems.push(format!(
                "ci.yml's {id} does not gate on workflow_dispatch alone"
            ));
        }
    }
}

fn check_ci_build_macos(wf: &Workflow, problems: &mut Vec<String>) {
    let job = match wf.require_job("ci.yml", "build-macos") {
        Ok(job) => job,
        Err(err) => {
            problems.push(err.to_string());
            return;
        }
    };
    if job.name.as_deref() != Some("build (macos-latest)") {
        problems.push("build-macos name is not `build (macos-latest)`".to_string());
    }
    if job.allows_failure() {
        problems.push("build-macos allows failure via continue-on-error".to_string());
    }
    let pr_spec = EventExpectation::WithTypes {
        name: "pull_request",
        types: &CI_PR_TYPES,
    };
    let mg_spec = EventExpectation::WithTypes {
        name: "merge_group",
        types: &CI_MG_TYPES,
    };
    let ok_ctx = ConditionContext::default_ok();
    let pr_ctx = ok_ctx.clone().with_event("pull_request");
    let mg_ctx = ok_ctx.clone().with_event("merge_group");
    let push_ctx = ok_ctx.clone().with_event("push");
    if wf.event_reaches_job("ci.yml", job, &pr_spec, &ok_ctx) != Ok(true)
        || wf.event_reaches_job("ci.yml", job, &mg_spec, &ok_ctx) != Ok(true)
        || job.runs_under("ci.yml", &push_ctx) != Ok(false)
    {
        problems.push("build-macos does not run on pull_request and merge_group".to_string());
    }
    if job.runs_on != Some(RunsOn::Single("macos-latest".to_string())) {
        problems.push("build-macos does not run on macos-latest".to_string());
    }
    let step_runs = |idx: usize, s: &Step| {
        job.step_enforces("ci.yml", s, idx, &pr_ctx, &pr_ctx) == Ok(true)
            && job.step_enforces("ci.yml", s, idx, &mg_ctx, &mg_ctx) == Ok(true)
    };
    if !job.steps.iter().enumerate().any(|(idx, s)| {
        matches!(
            s.action_match("Swatinem/rust-cache"),
            Some(ActionMatch::Exact)
        ) && step_runs(idx, s)
    }) {
        problems.push("build-macos does not reuse rust-cache".to_string());
    }
    if !job.steps.iter().enumerate().any(|(idx, s)| {
        s.run.as_deref() == Some("cargo test --workspace --all-features --locked --no-run")
            && step_runs(idx, s)
    }) {
        problems.push(
            "build-macos does not compile all workspace test targets without running tests"
                .to_string(),
        );
    }
    if job.steps.iter().any(|s| {
        s.run
            .as_deref()
            .is_some_and(|r| r.contains("test-shards.sh"))
    }) {
        problems.push("build-macos runs test-shards.sh".to_string());
    }
}

/// Check the single reusable shard workflow `macos-test-shards.yml` (C3).
fn check_reusable_shards(shards: &str, problems: &mut Vec<String>) {
    let Some(wf) = parse_or_problem("macos-test-shards.yml", shards, problems) else {
        return;
    };
    if wf.event_names() != ["workflow_call"] || wf.unfiltered_event("workflow_call").is_none() {
        problems.push(format!(
            "macos-test-shards.yml events are {:?}, not unfiltered [workflow_call]",
            wf.event_names()
        ));
    }
    match wf.workflow_call_input("use-cache") {
        Some(input) => {
            if input.input_type.as_ref().and_then(Scalar::as_literal) != Some("boolean") {
                problems.push("macos-test-shards.yml use-cache input is not typed boolean".into());
            }
            if input.default.as_ref().and_then(Scalar::as_bool) != Some(false) {
                problems
                    .push("macos-test-shards.yml use-cache input does not default to false".into());
            }
        }
        None => problems.push("macos-test-shards.yml has no use-cache input".to_string()),
    }
    match wf.require_job("macos-test-shards.yml", "macos-machines") {
        Ok(job) => check_reusable_shard_job(job, problems),
        Err(err) => problems.push(err.to_string()),
    }
}

fn is_cache_step(step: &Step) -> bool {
    matches!(
        step.action_match("Swatinem/rust-cache"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    ) || matches!(
        step.action_match("actions/cache"),
        Some(ActionMatch::Exact | ActionMatch::SubAction(_))
    )
}

fn check_shard_cache_steps(job: &Job, problems: &mut Vec<String>) {
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
        let gated = MACOS_SHARDS.iter().all(|m| {
            let (on, off) = (shard_ctx(m, true), shard_ctx(m, false));
            job.runs_under("macos-test-shards.yml", &on) == Ok(true)
                && step.runs_under("macos-test-shards.yml", &job.id, idx, &on) == Ok(true)
                && step.runs_under("macos-test-shards.yml", &job.id, idx, &off) == Ok(false)
        });
        if !gated {
            problems.push(format!(
                "macos-test-shards.yml step {} cache is not gated on inputs.use-cache",
                idx + 1
            ));
        }
    }
    if !saw_rust_cache {
        problems.push("macos-test-shards.yml does not gate rust-cache on inputs.use-cache".into());
    }
}

fn check_reusable_shard_job(job: &Job, problems: &mut Vec<String>) {
    const FILE: &str = "macos-test-shards.yml";
    if job.runs_on != Some(RunsOn::Single("macos-latest".to_string())) {
        problems.push("macos-test-shards.yml macos-machines does not run on macos-latest".into());
    }
    match &job.matrix {
        None => problems.push("macos-test-shards.yml lost the five macOS shards".to_string()),
        Some(matrix) => {
            if let Err(refusal) = matrix.require_no_include(FILE, &job.id) {
                problems.push(refusal.to_string());
            }
            if let Err(refusal) = matrix.require_no_exclude(FILE, &job.id) {
                problems.push(refusal.to_string());
            }
            if matrix.axes.len() != 1
                || matrix.axis_strings("machine").as_deref() != Some(MACOS_SHARDS.as_slice())
            {
                problems.push("macos-test-shards.yml lost the five macOS shards".to_string());
            }
        }
    }
    if job.allows_failure() {
        problems.push("macos-test-shards.yml macos-machines sets continue-on-error".to_string());
    }
    let has_shard_step = job.steps.iter().enumerate().any(|(idx, s)| {
        s.run.as_deref() == Some("bash scripts/test-shards.sh test --parallel-heaps")
            && s.env_value("BROKKR_TEST_MACHINE") == Some("${{ matrix.machine }}")
            && step_enforces_all_shards(FILE, job, s, idx)
    });
    if !has_shard_step {
        problems.push(
            "macos-test-shards.yml shard step lost scripts/test-shards.sh or BROKKR_TEST_MACHINE: ${{ matrix.machine }}"
                .to_string(),
        );
    }
    let has_gate_a = job.steps.iter().enumerate().any(|(idx, s)| {
        s.run.as_deref() == Some(GATE_A_SCRIPT)
            && [false, true].into_iter().all(|c| {
                let ctx = shard_ctx("macos/1", c);
                job.step_enforces(FILE, s, idx, &ctx, &ctx) == Ok(true)
            })
            && MACOS_SHARDS[1..]
                .iter()
                .all(|m| s.runs_under(FILE, &job.id, idx, &shard_ctx(m, false)) == Ok(false))
    });
    if !has_gate_a {
        problems.push("macos-test-shards.yml lost Gate A scoped to macos/1".to_string());
    }
    check_shard_cache_steps(job, problems);
    let has_bundles = job.steps.iter().enumerate().any(|(idx, s)| {
        s.run.as_deref() == Some(BUNDLES_SCRIPT) && step_enforces_all_shards(FILE, job, s, idx)
    });
    if !has_bundles {
        problems.push("macos-test-shards.yml lost recipes/self and recipes/verify compile".into());
    }
}

fn check_shard_caller(
    file: &str,
    text: &str,
    wants_cache: bool,
    events: &[EventExpectation<'_>],
    problems: &mut Vec<String>,
) {
    let Some(wf) = parse_or_problem(file, text, problems) else {
        return;
    };
    if wf
        .jobs
        .iter()
        .any(|j| j.all_scalars.iter().any(|sc| sc.contains("macos/1")))
    {
        problems.push(format!("{file} copied the macOS shard list"));
    }
    match wf.require_job(file, "macos-machines") {
        Ok(job) => check_caller_machines(file, &wf, job, wants_cache, events, problems),
        Err(err) => problems.push(err.to_string()),
    }
    match wf.require_job(file, "macos-tests") {
        Ok(job) => check_caller_verdict(file, &wf, job, events, problems),
        Err(err) => problems.push(err.to_string()),
    }
}

fn check_caller_machines(
    file: &str,
    wf: &Workflow,
    job: &Job,
    wants_cache: bool,
    events: &[EventExpectation<'_>],
    problems: &mut Vec<String>,
) {
    if job.uses.as_deref() != Some(SHARDS_WORKFLOW) {
        problems.push(format!(
            "{file}'s macos-machines does not call macos-test-shards.yml"
        ));
    }
    if job.allows_failure() {
        problems.push(format!("{file}'s macos-machines sets continue-on-error"));
    }
    let ok = ConditionContext::default_ok();
    if !events
        .iter()
        .all(|ev| wf.event_reaches_job(file, job, ev, &ok) == Ok(true))
    {
        problems.push(format!(
            "{file}'s macos-machines does not run on {events:?}"
        ));
    }
    let cache_setting = match job.with_value("use-cache") {
        None => Ok(false),
        Some(val) => val
            .as_bool()
            .ok_or_else(|| format!("{val:?} is not a YAML boolean")),
    };
    match cache_setting {
        Ok(enables_cache) if enables_cache != wants_cache => problems.push(format!(
            "{file}'s macos-machines use-cache={enables_cache}, expected {wants_cache}"
        )),
        Ok(_) => {}
        Err(reason) => problems.push(format!(
            "{file}'s macos-machines use-cache input is invalid: {reason}"
        )),
    }
}

fn check_caller_verdict(
    file: &str,
    wf: &Workflow,
    job: &Job,
    events: &[EventExpectation<'_>],
    problems: &mut Vec<String>,
) {
    if job.name.as_deref() != Some("test (macos-latest)") {
        problems.push("macos-tests is not named `test (macos-latest)`".to_string());
    }
    if !job.needs.iter().any(|n| n == "macos-machines") {
        problems.push(format!("{file}'s macos-tests does not need macos-machines"));
    }
    if job.allows_failure() {
        problems.push(format!("{file}'s macos-tests sets continue-on-error"));
    }
    let runs_verdict = job.steps.iter().enumerate().any(|(idx, s)| {
        s.run.as_deref() == Some(VERDICT_SCRIPT)
            && s.env_value("MACHINES") == Some("${{ needs.macos-machines.result }}")
            && events.iter().all(|ev| {
                let job_ctx = ConditionContext::default_ok().with_prior_succeeded(false);
                let step_ctx = ConditionContext::default_ok().with_event(ev.event_name());
                !s.allows_failure()
                    && wf.event_reaches_job(file, job, ev, &job_ctx) == Ok(true)
                    && s.runs_under(file, &job.id, idx, &step_ctx) == Ok(true)
            })
    });
    if !runs_verdict {
        problems.push("macos-tests does not check MACHINES=success".to_string());
    }
}

fn extract_gha_expressions(text: &str) -> Result<Vec<&str>, &'static str> {
    let mut exprs = Vec::new();
    let mut rest = text;
    while let Some((_, after_open)) = rest.split_once("${{") {
        let Some((inner, after_close)) = after_open.split_once("}}") else {
            return Err("unclosed `${{` expression");
        };
        exprs.push(inner.trim());
        rest = after_close;
    }
    if rest.contains("}}") {
        return Err("stray `}}` outside expression");
    }
    Ok(exprs)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConcurrencyProblem {
    MissingBlock,
    UnreadableGroup,
    MultilineOrEmptyGroup(String),
    InvalidExpression { group: String, reason: &'static str },
    SharedSlot(String),
    DisallowedExpression { group: String, expr: String },
    CancelInProgressNotFalse,
}

fn nightly_concurrency_problems(wf: &Workflow) -> Vec<ConcurrencyProblem> {
    let mut problems = Vec::new();
    let Some(conc) = &wf.concurrency else {
        problems.push(ConcurrencyProblem::MissingBlock);
        return problems;
    };
    match conc.group.as_ref().and_then(Scalar::as_str) {
        None => problems.push(ConcurrencyProblem::UnreadableGroup),
        Some(raw_group) => {
            let group = raw_group.trim_end_matches('\n');
            if group.is_empty() || group.contains(['\n', '\r']) {
                problems.push(ConcurrencyProblem::MultilineOrEmptyGroup(raw_group.into()));
            }
            match extract_gha_expressions(group) {
                Err(reason) => problems.push(ConcurrencyProblem::InvalidExpression {
                    group: group.to_string(),
                    reason,
                }),
                Ok(exprs) if exprs.is_empty() => {
                    problems.push(ConcurrencyProblem::SharedSlot(group.to_string()));
                }
                Ok(exprs) => {
                    for expr in exprs {
                        if expr != "github.event_name" {
                            problems.push(ConcurrencyProblem::DisallowedExpression {
                                group: group.to_string(),
                                expr: expr.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }
    if conc.cancel_in_progress.as_ref().and_then(Scalar::as_bool) != Some(false) {
        problems.push(ConcurrencyProblem::CancelInProgressNotFalse);
    }
    problems
}

const STUB_GH_SCRIPT: &str = r#"#!/usr/bin/env bash
set -euo pipefail
[ -n "${GH_TOKEN:-}" ] || { echo "GH_TOKEN is not set" >&2; exit 1; }
stub_dir="$(cd "$(dirname "$0")" && pwd)"
log_file="${stub_dir}/calls.jsonl"
issues_file="${stub_dir}/issues.json"
[ "${1:-}" = "api" ] || { echo "unexpected gh subcommand: ${1:-}" >&2; exit 2; }
shift
m=""; p="false"; j=""; ep=""; has_fields=0
fields=()
while [ "$#" -gt 0 ]; do
  case "$1" in
    -X|--method) m="$2"; shift 2 ;;
    --method=*) m="${1#--method=}"; shift ;;
    -X*) m="${1#-X}"; shift ;;
    --paginate) p="true"; shift ;;
    --jq|-q) j="$2"; shift 2 ;;
    --jq=*) j="${1#--jq=}"; shift ;;
    -f|--raw-field|-F|--field) fields+=("$2"); has_fields=1; shift 2 ;;
    --raw-field=*) fields+=("${1#--raw-field=}"); has_fields=1; shift ;;
    --field=*) fields+=("${1#--field=}"); has_fields=1; shift ;;
    -f*) fields+=("${1#-f}"); has_fields=1; shift ;;
    -F*) fields+=("${1#-F}"); has_fields=1; shift ;;
    --input) has_fields=1; shift 2 ;;
    --input=*) has_fields=1; shift ;;
    -H|--header) shift 2 ;;
    --header=*) shift ;;
    -*) echo "unexpected gh flag: $1" >&2; exit 2 ;;
    *) [ -z "$ep" ] && ep="$1" || { echo "unexpected extra arg: $1" >&2; exit 2; }; shift ;;
  esac
done
if [ -z "$m" ]; then
  if [ "$has_fields" -eq 1 ]; then m="POST"; else m="GET"; fi
fi
m="$(printf '%s' "$m" | tr '[:lower:]' '[:upper:]')"
jq -nc --arg method "$m" --argjson paginate "$p" --arg endpoint "$ep" \
  '{method:$method,paginate:$paginate,endpoint:$endpoint,fields:$ARGS.positional}' \
  --args -- ${fields[@]+"${fields[@]}"} >> "$log_file"
out() { if [ -n "$j" ]; then printf '%s\n' "$1" | jq -r "$j"; else printf '%s\n' "$1"; fi; }
case "$m:$p:$ep" in
  GET:false:repos/owner/repo/labels/*|PATCH:false:repos/owner/repo/issues/*|POST:false:repos/owner/repo/issues|POST:false:repos/owner/repo/labels) exit 0 ;;
  GET:true:repos/owner/repo/actions/runs/*/jobs*) out '{"jobs":[{"name":"test (macos-latest) on macos/2","conclusion":"failure","html_url":"https://example.invalid/jobs/2"}]}' ;;
  GET:true:repos/owner/repo/issues?*) out "$(cat "$issues_file")" ;;
  *) echo "unexpected gh call: $m paginate=$p $ep" >&2; exit 2 ;;
esac
"#;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GhCall {
    method: String,
    paginate: bool,
    endpoint: String,
    fields: Vec<String>,
}

struct NotifyScenario {
    label: &'static str,
    issues_json: &'static str,
    expected_patch: Option<&'static str>,
    expected_posts: usize,
}

struct RunnableStep<'a> {
    script: &'a str,
    env: BTreeMap<String, String>,
}

fn eval_notify_env_expr(expr: &str, declared_needs: &[String]) -> Result<&'static str, String> {
    match expr {
        "github.token" => Ok("stub-github-token"),
        "github.repository" => Ok("owner/repo"),
        "github.run_id" => Ok("777"),
        "github.server_url" => Ok("https://example.invalid"),
        "github.sha" => Ok("deadbeef"),
        "github.event_name" => Ok("schedule"),
        other => {
            if let Some(job_id) = other
                .strip_prefix("needs.")
                .and_then(|r| r.strip_suffix(".result"))
            {
                if declared_needs.iter().any(|n| n == job_id) {
                    return Ok("failure");
                }
                return Err(format!(
                    "references `{other}` without declaring `{job_id}` in needs"
                ));
            }
            Err(format!("uses unsupported env expression `{other}`"))
        }
    }
}

fn expand_notify_env_value(raw: &str, declared_needs: &[String]) -> Result<String, String> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some((before, after_open)) = rest.split_once("${{") {
        let Some((inner, after_close)) = after_open.split_once("}}") else {
            return Err(format!("unclosed `${{{{` in env value `{raw}`"));
        };
        out.push_str(before);
        out.push_str(eval_notify_env_expr(inner.trim(), declared_needs)?);
        rest = after_close;
    }
    if rest.contains("}}") {
        return Err(format!("stray `}}}}` in env value `{raw}`"));
    }
    out.push_str(rest);
    Ok(out)
}

fn collect_notify_steps<'a>(job: &'a Job, problems: &mut Vec<String>) -> Vec<RunnableStep<'a>> {
    const FILE: &str = "macos-tests.yml";
    let mut job_ctx = ConditionContext::default_ok()
        .with_event("schedule")
        .with_prior_succeeded(false);
    for dep in &job.needs {
        job_ctx = job_ctx.with_needs_result(dep, "failure");
    }
    if job.allows_failure() {
        problems.push("notify-failure job sets continue-on-error".to_string());
        return Vec::new();
    }
    match job.runs_under(FILE, &job_ctx) {
        Ok(true) => {}
        Ok(false) => {
            problems.push("notify-failure job does not run on a failing nightly".to_string());
            return Vec::new();
        }
        Err(err) => {
            problems.push(err.to_string());
            return Vec::new();
        }
    }
    let mut step_ctx = ConditionContext::default_ok().with_event("schedule");
    for dep in &job.needs {
        step_ctx = step_ctx.with_needs_result(dep, "failure");
    }
    let mut runnable = Vec::new();
    for (idx, step) in job.steps.iter().enumerate() {
        let Some(script) = step.run.as_deref() else {
            continue;
        };
        match step.runs_under(FILE, &job.id, idx, &step_ctx) {
            Ok(false) => continue,
            Err(err) => {
                problems.push(err.to_string());
                return Vec::new();
            }
            Ok(true) => {}
        }
        if step.allows_failure() {
            problems.push("notify-failure step sets continue-on-error".to_string());
            return Vec::new();
        }
        let mut env = BTreeMap::new();
        for layer in [&job.env, &step.env] {
            for (k, v) in layer {
                match expand_notify_env_value(v, &job.needs) {
                    Ok(expanded) => {
                        env.insert(k.clone(), expanded);
                    }
                    Err(reason) => {
                        problems.push(format!("notify-failure env `{k}` {reason}"));
                        return Vec::new();
                    }
                }
            }
        }
        runnable.push(RunnableStep { script, env });
    }
    if runnable.is_empty() {
        problems.push("notify-failure has no runnable script step".to_string());
    }
    runnable
}

fn verify_notify_scenario(
    dir: &Path,
    path_env: &str,
    steps: &[RunnableStep<'_>],
    scenario: &NotifyScenario,
    problems: &mut Vec<String>,
) {
    let log_path = dir.join("calls.jsonl");
    let issues_path = dir.join("issues.json");
    std::fs::write(&log_path, "").expect("init stub log");
    std::fs::write(&issues_path, scenario.issues_json).expect("write stub issues");
    // Runner-provided environment is strictly `PATH` (with the stub `gh` dir
    // first); every other binding comes from workflow -> job -> step `env:`.
    for step in steps {
        let output = Command::new("bash")
            .args(["-e", "-c", step.script])
            .env_clear()
            .env("PATH", path_env)
            .envs(&step.env)
            .output()
            .expect("bash");
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            problems.push(format!(
                "notify-failure script failed in `{}`: {}",
                scenario.label,
                stderr.trim()
            ));
            return;
        }
    }
    let log = std::fs::read_to_string(&log_path).expect("read stub log");
    let calls: Vec<GhCall> = log
        .lines()
        .map(|l| serde_json::from_str(l).expect("parse stub gh jsonl"))
        .collect();
    if !calls.iter().any(|c| {
        c.method == "GET"
            && c.paginate
            && c.endpoint
                .starts_with("repos/owner/repo/actions/runs/777/jobs")
    }) {
        problems.push(format!(
            "notify-failure ({}) did not paginate failed jobs",
            scenario.label
        ));
    }
    if !calls.iter().any(|c| {
        c.method == "GET"
            && c.paginate
            && c.endpoint
                .starts_with("repos/owner/repo/issues?labels=macos-nightly&state=open")
    }) {
        problems.push(format!(
            "notify-failure ({}) did not paginate open issue lookup",
            scenario.label
        ));
    }
    check_notify_mutations(&calls, scenario, problems);
}

fn check_notify_mutations(calls: &[GhCall], scenario: &NotifyScenario, problems: &mut Vec<String>) {
    let has_run_body = |fields: &[String]| {
        fields.iter().any(|f| {
            f.strip_prefix("body=").is_some_and(|b| {
                b.contains("https://example.invalid/owner/repo/actions/runs/777")
                    && b.contains("deadbeef")
            })
        })
    };
    for c in calls {
        if !c.endpoint.starts_with("repos/owner/repo/") {
            problems.push(format!(
                "notify-failure ({}) targeted unexpected repository endpoint `{}`",
                scenario.label, c.endpoint
            ));
        }
        if matches!(c.method.as_str(), "POST" | "PATCH") && c.paginate {
            problems.push(format!(
                "notify-failure ({}) paginated write call {} {}",
                scenario.label, c.method, c.endpoint
            ));
        }
    }
    let patches: Vec<&GhCall> = calls.iter().filter(|c| c.method == "PATCH").collect();
    let patch_endpoints: Vec<&str> = patches.iter().map(|c| c.endpoint.as_str()).collect();
    match scenario.expected_patch {
        Some(expected) if patch_endpoints != [expected] => problems.push(format!(
            "notify-failure ({}) patches were {patch_endpoints:?}, expected [{expected:?}]",
            scenario.label
        )),
        Some(_) if !has_run_body(&patches[0].fields) => problems.push(format!(
            "notify-failure ({}) patch missing expected body fields",
            scenario.label
        )),
        None if !patches.is_empty() => problems.push(format!(
            "notify-failure ({}) unexpectedly patched {patch_endpoints:?}",
            scenario.label
        )),
        _ => {}
    }
    let posts: Vec<&GhCall> = calls
        .iter()
        .filter(|c| c.method == "POST" && c.endpoint != "repos/owner/repo/labels")
        .collect();
    if posts.len() != scenario.expected_posts {
        problems.push(format!(
            "notify-failure ({}) created {} issue(s), expected {}",
            scenario.label,
            posts.len(),
            scenario.expected_posts
        ));
    }
    for post in posts {
        if !post.fields.iter().any(|f| f == "labels[]=macos-nightly") {
            problems.push(format!(
                "notify-failure ({}) created issue without labels[]=macos-nightly: {:?}",
                scenario.label, post.fields
            ));
        }
        if !post
            .fields
            .iter()
            .any(|f| f.strip_prefix("title=").is_some_and(|t| !t.is_empty()))
        {
            problems.push(format!(
                "notify-failure ({}) created issue without title",
                scenario.label
            ));
        }
        if !has_run_body(&post.fields) {
            problems.push(format!(
                "notify-failure ({}) created issue without expected body",
                scenario.label
            ));
        }
    }
}

fn run_notify_failure_behavior(job: &Job, problems: &mut Vec<String>) {
    let steps = collect_notify_steps(job, problems);
    if steps.is_empty() {
        return;
    }
    let dir = tempfile::tempdir().expect("tempdir for notify-failure stub");
    let gh_path = dir.path().join("gh");
    std::fs::write(&gh_path, STUB_GH_SCRIPT).expect("write stub gh");
    std::fs::set_permissions(&gh_path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod stub gh");
    let path_env = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let scenarios = [
        NotifyScenario {
            label: "existing-open-issue",
            issues_json: r#"[{"number":10,"pull_request":{}},{"number":99},{"number":42}]"#,
            expected_patch: Some("repos/owner/repo/issues/42"),
            expected_posts: 0,
        },
        NotifyScenario {
            label: "no-open-issue",
            issues_json: "[]",
            expected_patch: None,
            expected_posts: 1,
        },
    ];
    for scenario in &scenarios {
        verify_notify_scenario(dir.path(), &path_env, &steps, scenario, problems);
    }
}

fn check_nightly_workflow(nightly: &str, problems: &mut Vec<String>) {
    let Some(wf) = parse_or_problem("macos-tests.yml", nightly, problems) else {
        return;
    };
    if wf.event_names() != ["schedule", "workflow_dispatch"]
        || !wf
            .event("schedule")
            .is_some_and(EventTrigger::has_active_schedule)
        || wf.unfiltered_event("workflow_dispatch").is_none()
    {
        problems.push(format!(
            "macos-tests.yml events/schedule invalid: {:?}",
            wf.events
        ));
    }
    for p in nightly_concurrency_problems(&wf) {
        problems.push(format!("{p:?}"));
    }
    let job = match wf.require_job("macos-tests.yml", "notify-failure") {
        Ok(job) => job,
        Err(err) => {
            problems.push(err.to_string());
            return;
        }
    };
    check_notify_job_gate(&wf, job, problems);
    let expected_perms = Permissions::Scopes(BTreeMap::from([
        ("actions".to_string(), "read".to_string()),
        ("issues".to_string(), "write".to_string()),
    ]));
    if job.permissions.as_ref() != Some(&expected_perms) {
        problems.push(
            "notify-failure permissions are not minimal (actions: read, issues: write)".to_string(),
        );
    }
    run_notify_failure_behavior(job, problems);
}

fn check_notify_job_gate(wf: &Workflow, job: &Job, problems: &mut Vec<String>) {
    const FILE: &str = "macos-tests.yml";
    let ctx_for = |verdict: &str, prior_ok: bool| {
        let mut ctx = ConditionContext::default_ok().with_prior_succeeded(prior_ok);
        for dep in &job.needs {
            ctx = ctx.with_needs_result(dep, verdict);
        }
        ctx
    };
    let gate_ok = job.needs.iter().any(|n| n == "macos-tests")
        && wf.event_reaches_job(
            FILE,
            job,
            &EventExpectation::ActiveSchedule,
            &ctx_for("failure", false),
        ) == Ok(true)
        && wf.event_reaches_job(
            FILE,
            job,
            &EventExpectation::ActiveSchedule,
            &ctx_for("cancelled", false),
        ) == Ok(true)
        && wf.event_reaches_job(
            FILE,
            job,
            &EventExpectation::ActiveSchedule,
            &ctx_for("success", true),
        ) == Ok(false)
        && wf.event_reaches_job(
            FILE,
            job,
            &EventExpectation::Plain("workflow_dispatch"),
            &ctx_for("failure", false),
        ) == Ok(false);
    if !gate_ok {
        problems.push(
            "notify-failure condition does not match failing nightly schedule only".to_string(),
        );
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

    fn problems(&self) -> Vec<String> {
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
                    Ok(job) if !job.needs.iter().any(|n| n == "macos-tests") => {
                        problems.push(format!("release.yml's {id} does not wait on macos-tests"));
                    }
                    Ok(_) => {}
                    Err(err) => problems.push(err.to_string()),
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

#[test]
fn pull_request_path_replaces_macos_shards_with_macos_build() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();
    assert_eq!(ci_macos_split_problems(&ci), Vec::<String>::new());

    let mut_no_run = ci.replacen("--locked --no-run", "--locked", 1);
    assert!(!ci_macos_split_problems(&mut_no_run).is_empty());
    let mut_perm = ci.replacen(
        "permissions:\n  contents: read\n",
        "permissions:\n  contents: read\n  issues: write\n",
        1,
    );
    assert_ne!(mut_perm, ci);
    assert!(!ci_macos_split_problems(&mut_perm).is_empty());

    for (filter, expected) in [
        ("    paths: [crates/**]\n", "not a form this guard reads"),
        ("    branches: [other]\n", "unfiltered on pull_request"),
    ] {
        let filtered = ci.replacen(
            "  pull_request:\n    types: [opened, synchronize, reopened]\n",
            &format!("  pull_request:\n    types: [opened, synchronize, reopened]\n{filter}"),
            1,
        );
        assert_ne!(filtered, ci);
        let probs = ci_macos_split_problems(&filtered);
        assert!(
            probs.iter().any(|p| p.contains(expected)),
            "filter `{filter}` expected `{expected}`: {probs:?}"
        );
    }

    let key_macos = ci.replacen(
        "  engine:\n    name: test (ubuntu-latest)\n",
        "  engine:\n    name: test (ubuntu-latest)\n    env:\n      MACOS_HOST: '1'\n",
        1,
    );
    assert_ne!(key_macos, ci);
    assert!(ci_macos_split_problems(&key_macos)
        .iter()
        .any(|p| p.contains("engine still mentions macOS")));

    for (bypass, expected) in [
        ("        if: false\n", "not a form this guard reads"),
        (
            "        continue-on-error: true\n",
            "compile all workspace test targets",
        ),
        (
            "        if: vars.ENABLE_BUILD\n",
            "not a form this guard reads",
        ),
    ] {
        let bypassed = ci.replacen(
            "      - name: workspace and test targets compile on macOS\n",
            &format!("      - name: workspace and test targets compile on macOS\n{bypass}"),
            1,
        );
        assert_ne!(bypassed, ci);
        let probs = ci_macos_split_problems(&bypassed);
        assert!(
            probs.iter().any(|p| p.contains(expected)),
            "bypass `{bypass}` expected `{expected}`: {probs:?}"
        );
    }
}

#[test]
fn macos_test_shards_run_nightly_on_dispatch_and_in_release_with_failure_issue() {
    let wfs = ShardWorkflows::load();
    assert_eq!(wfs.problems(), Vec::<String>::new());

    let mut_release_pub = wfs
        .release
        .replacen("needs: [build, macos-tests]", "needs: build", 1);
    assert!(!wfs.with_release(mut_release_pub).problems().is_empty());

    let mut_nightly_cond = wfs.nightly.replacen(
        "github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "github.event_name == 'schedule'",
        1,
    );
    assert_ne!(mut_nightly_cond, wfs.nightly);
    assert!(!wfs.with_nightly(mut_nightly_cond).problems().is_empty());

    for empty_sched in ["  schedule: []\n", "  schedule:\n"] {
        let no_cron =
            wfs.nightly
                .replacen("  schedule:\n    - cron: '17 4 * * *'\n", empty_sched, 1);
        assert_ne!(no_cron, wfs.nightly);
        assert!(wfs
            .with_nightly(no_cron)
            .problems()
            .iter()
            .any(|p| p.contains("events/schedule invalid")));
    }

    let mut_no_machine = wfs.shards.replacen(
        "        env:\n          BROKKR_TEST_MACHINE: ${{ matrix.machine }}\n",
        "",
        1,
    );
    assert_ne!(mut_no_machine, wfs.shards);
    assert!(wfs
        .with_shards(mut_no_machine)
        .problems()
        .iter()
        .any(|p| p.contains("BROKKR_TEST_MACHINE")));

    let closed = "not a form this guard reads";
    for (bypass, shard_exp, bundle_exp) in [
        ("        if: false\n", closed, closed),
        (
            "        if: matrix.machine == 'macos/1'\n",
            "shard step lost",
            "recipes/self and recipes/verify compile",
        ),
        ("        if: vars.RUN_SHARDS\n", closed, closed),
        (
            "        continue-on-error: true\n",
            "shard step lost",
            "recipes/self and recipes/verify compile",
        ),
    ] {
        let shard_mut = wfs.shards.replacen(
            "      - run: bash scripts/test-shards.sh test --parallel-heaps\n",
            &format!("      - run: bash scripts/test-shards.sh test --parallel-heaps\n{bypass}"),
            1,
        );
        let shard_probs = wfs.with_shards(shard_mut).problems();
        assert!(
            shard_probs.iter().any(|p| p.contains(shard_exp)),
            "shard bypass `{bypass}`: {shard_probs:?}"
        );
        let bundle_mut = wfs.shards.replacen(
            "      - name: self and verify bundles compile under the constitutional lint\n",
            &format!(
                "      - name: self and verify bundles compile under the constitutional lint\n{bypass}"
            ),
            1,
        );
        let bundle_probs = wfs.with_shards(bundle_mut).problems();
        assert!(
            bundle_probs.iter().any(|p| p.contains(bundle_exp)),
            "bundle bypass `{bypass}`: {bundle_probs:?}"
        );
    }

    let mut_unpaginated = wfs.nightly.replacen(
        "existing=\"$(gh api --paginate \"repos/${REPO}/issues?labels=${LABEL}&state=open",
        "existing=\"$(gh api \"repos/${REPO}/issues?labels=${LABEL}&state=open",
        1,
    );
    assert_ne!(mut_unpaginated, wfs.nightly);
    assert!(!wfs.with_nightly(mut_unpaginated).problems().is_empty());

    let wf = Workflow::parse(&wfs.ci).expect("ci.yml parses");
    let missing = wf.require_job("ci.yml", "no-such-job").unwrap_err();
    assert_eq!(
        missing,
        WorkflowError::MissingJob {
            file: "ci.yml".to_string(),
            id: "no-such-job".to_string()
        }
    );
    assert_eq!(missing.to_string(), "ci.yml has no no-such-job job");
}

#[test]
fn nightly_concurrency_and_notify_script_hold_under_yaml_variants() {
    let wfs = ShardWorkflows::load();

    let shared = wfs.nightly.replacen(
        "  group: macos-tests-${{ github.event_name }}\n",
        "  group: macos-tests\n",
        1,
    );
    assert_eq!(
        nightly_concurrency_problems(&Workflow::parse(&shared).unwrap()),
        vec![ConcurrencyProblem::SharedSlot("macos-tests".to_string())]
    );

    for (bad_group, group, expr) in [
        (
            "  group: macos-tests-${{ github.sha }}\n",
            "macos-tests-${{ github.sha }}",
            "github.sha",
        ),
        (
            "  group: macos-tests-${{ github.event_name }}-${{ github.run_number }}\n",
            "macos-tests-${{ github.event_name }}-${{ github.run_number }}",
            "github.run_number",
        ),
        (
            "  group: macos-tests-${{ github.event_name }}-${{ github.run_attempt }}\n",
            "macos-tests-${{ github.event_name }}-${{ github.run_attempt }}",
            "github.run_attempt",
        ),
        (
            "  group: macos-tests-${{ github.event_name }}-${{ github.sha }}\n",
            "macos-tests-${{ github.event_name }}-${{ github.sha }}",
            "github.sha",
        ),
        (
            "  group: macos-tests-${{ github.event_name || github.run_number }}\n",
            "macos-tests-${{ github.event_name || github.run_number }}",
            "github.event_name || github.run_number",
        ),
        (
            "  group: >-\n    macos-tests-${{ github.event_name }}-${{ github.sha }}\n",
            "macos-tests-${{ github.event_name }}-${{ github.sha }}",
            "github.sha",
        ),
        (
            "  group: >\n    macos-tests-${{ github.event_name }}-${{ github.sha }}\n",
            "macos-tests-${{ github.event_name }}-${{ github.sha }}",
            "github.sha",
        ),
        (
            "  group: macos-tests-${{ github.event_name }}\n    -${{ github.sha }}\n",
            "macos-tests-${{ github.event_name }} -${{ github.sha }}",
            "github.sha",
        ),
    ] {
        let mutated = wfs.nightly.replacen(
            "  group: macos-tests-${{ github.event_name }}\n",
            bad_group,
            1,
        );
        assert_ne!(mutated, wfs.nightly);
        assert_eq!(
            nightly_concurrency_problems(&Workflow::parse(&mutated).unwrap()),
            vec![ConcurrencyProblem::DisallowedExpression {
                group: group.to_string(),
                expr: expr.to_string(),
            }]
        );
    }

    let mut_comment_conc = wfs
        .nightly
        .replacen("concurrency:\n", "concurrency: # serialize\n", 1);
    assert_eq!(
        wfs.with_nightly(mut_comment_conc).problems(),
        Vec::<String>::new()
    );
    let mut_cancel_true = wfs.nightly.replacen(
        "  cancel-in-progress: false\n",
        "  cancel-in-progress: true\n",
        1,
    );
    assert_eq!(
        nightly_concurrency_problems(&Workflow::parse(&mut_cancel_true).unwrap()),
        vec![ConcurrencyProblem::CancelInProgressNotFalse]
    );
}

#[test]
fn notify_failure_harness_enforces_gh_fields_methods_conditions_and_workflow_env() {
    let wfs = ShardWorkflows::load();

    // Unconditional issue create after update branch, both with `-X POST` and
    // with fields-only (implicit POST in `gh api`).
    for create_call in [
        "            gh api -X POST \"repos/${REPO}/issues\" \\\n",
        "            gh api \"repos/${REPO}/issues\" \\\n",
    ] {
        let mut_uncond = wfs
            .nightly
            .replacen(
                "          if [ -n \"$existing\" ]; then\n            gh api -X PATCH \"repos/${REPO}/issues/${existing}\" -f body=\"$body\" >/dev/null\n          else\n            gh api -X POST \"repos/${REPO}/issues\" \\\n",
                &format!("          if [ -n \"$existing\" ]; then\n            gh api -X PATCH \"repos/${{REPO}}/issues/${{existing}}\" -f body=\"$body\" >/dev/null\n          fi\n{create_call}"),
                1,
            )
            .replacen(
                "              -f \"labels[]=${LABEL}\" >/dev/null\n          fi\n",
                "              -f \"labels[]=${LABEL}\" >/dev/null\n",
                1,
            );
        assert_ne!(mut_uncond, wfs.nightly);
        let problems = wfs.with_nightly(mut_uncond).problems();
        assert!(
            problems
                .iter()
                .any(|p| p.contains("created 1 issue(s), expected 0")),
            "duplicate create `{create_call}` must fail: {problems:?}"
        );
    }

    let no_label = wfs.nightly.replacen(
        "              -f body=\"$body\" \\\n              -f \"labels[]=${LABEL}\" >/dev/null\n",
        "              -f body=\"$body\" >/dev/null\n",
        1,
    );
    assert_ne!(no_label, wfs.nightly);
    assert!(wfs
        .with_nightly(no_label)
        .problems()
        .iter()
        .any(|p| p.contains("without labels[]=macos-nightly")));

    for dropped in [
        "          REPO: ${{ github.repository }}\n",
        "          GH_TOKEN: ${{ github.token }}\n",
        "          VERDICT: ${{ needs.macos-tests.result }}\n",
    ] {
        let mutated = wfs.nightly.replacen(dropped, "", 1);
        assert_ne!(mutated, wfs.nightly);
        assert!(
            !wfs.with_nightly(mutated).problems().is_empty(),
            "dropping `{dropped}` must fail"
        );
    }
    let dropped_needs = wfs.nightly.replacen(
        "    needs: [macos-machines, macos-tests]\n",
        "    needs: [macos-machines]\n",
        1,
    );
    assert!(!wfs.with_nightly(dropped_needs).problems().is_empty());

    for bypass in ["        if: false\n", "        continue-on-error: true\n"] {
        let mutated = wfs.nightly.replacen(
            "      - name: open or update the nightly macOS failure issue\n",
            &format!("      - name: open or update the nightly macOS failure issue\n{bypass}"),
            1,
        );
        assert!(!wfs.with_nightly(mutated).problems().is_empty());
    }

    let lifted_env = wfs
        .nightly
        .replacen(
            "permissions:\n",
            "env:\n  LABEL: macos-nightly\n\npermissions:\n",
            1,
        )
        .replacen("          LABEL: macos-nightly\n", "", 1);
    let lifted_probs = wfs.with_nightly(lifted_env).problems();
    assert!(
        lifted_probs
            .iter()
            .any(|p| p.contains("macos-tests.yml (top-level env)")
                && p.contains("not a form this guard reads")),
        "workflow-level env must be refused: {lifted_probs:?}"
    );
}

#[test]
fn cache_input_is_parsed_structurally_across_callers() {
    let wfs = ShardWorkflows::load();

    let mut_ci_comment = wfs.ci.replacen(
        "    with:\n      use-cache: true\n",
        "    # use-cache: true\n",
        1,
    );
    assert!(!wfs.with_ci(mut_ci_comment).problems().is_empty());

    let mut_ci_spaced = wfs.ci.replacen(
        "      use-cache: true\n",
        "      use-cache:  true # warm\n",
        1,
    );
    assert_eq!(wfs.with_ci(mut_ci_spaced).problems(), Vec::<String>::new());

    let mut_release_comment = wfs.release.replacen(
        "    uses: ./.github/workflows/macos-test-shards.yml\n",
        "    uses: ./.github/workflows/macos-test-shards.yml\n    # use-cache: true\n",
        1,
    );
    assert_eq!(
        wfs.with_release(mut_release_comment).problems(),
        Vec::<String>::new()
    );

    for bad_release_with in [
        "    with:\n      use-cache:  true\n",
        "    with:\n      Use-Cache: true\n",
        "    with:\n      use-cache: true # warm\n",
        "    with:\n      use-cache: 'true'\n",
        "    with: { use-cache: true }\n",
    ] {
        let mut_release = wfs.release.replacen(
            "    uses: ./.github/workflows/macos-test-shards.yml\n",
            &format!("    uses: ./.github/workflows/macos-test-shards.yml\n{bad_release_with}"),
            1,
        );
        assert!(!wfs.with_release(mut_release).problems().is_empty());
    }

    let mut_ci_quoted = wfs
        .ci
        .replacen("      use-cache: true\n", "      use-cache: 'true'\n", 1);
    assert!(!wfs.with_ci(mut_ci_quoted).problems().is_empty());
    let mut_default_true =
        wfs.shards
            .replacen("        default: false\n", "        default: true\n", 1);
    assert!(!wfs.with_shards(mut_default_true).problems().is_empty());
}

#[test]
fn contributing_records_macos_build_handoff_and_shard_schedule() {
    let contributing = std::fs::read_to_string(workspace().join("CONTRIBUTING.md")).unwrap();
    for required in [
        "requires 12 checks:",
        "`build (macos-latest)`",
        "once #1383 has landed and the operator's controller has added `build (macos-latest)`",
        "without running tests",
        "`test (macos-latest)`",
        "nightly against `main`",
        "`workflow_dispatch`",
        "`release.yml`",
        "## 7. Workflow closed form",
        "not a form this guard reads; allowed forms:",
        "strategy.job-index",
        "no workflow-level `env:`",
    ] {
        assert!(
            contributing.contains(required),
            "CONTRIBUTING.md does not record {required}"
        );
    }
}

#[test]
fn workflow_reader_checks_mapping_keys_write_implies_read_and_refuses_unknown_conditions() {
    let wf = Workflow::parse(
        "on: push\npermissions:\n  contents: write\njobs:\n  BROKKR_FLAKE_PR_TOKEN:\n    runs-on: ubuntu-24.04\n    env:\n      SECRET_KEY_NAME: safe\n",
    )
    .expect("parses");
    let job = wf.job("BROKKR_FLAKE_PR_TOKEN").expect("job");
    assert!(job.all_scalars.iter().any(|s| s == "BROKKR_FLAKE_PR_TOKEN"));
    assert!(job.all_scalars.iter().any(|s| s == "SECRET_KEY_NAME"));
    assert!(wf.grants_permission("contents", "write"));
    assert!(wf.grants_permission("contents", "read"));
    assert!(!wf.grants_permission("issues", "read"));

    let ctx = ConditionContext::default_ok()
        .with_event("pull_request")
        .with_runner_environment("self-hosted")
        .with_input("Use-Cache", CondVal::Bool(true))
        .with_matrix("nfpm_arch", CondVal::Str("amd64".into()))
        .with_needs_result("macos-tests", "failure")
        .with_needs_output("classify", "lane", CondVal::Str("full".into()))
        .with_step_output(
            "flake-pr",
            "pull-request-operation",
            CondVal::Str("UPDATED".into()),
        );

    let eval = |expr: Option<&str>, c: &ConditionContext| {
        ClosedCondition::parse(expr, "t.yml", "job a if")
            .and_then(|cond| cond.evaluate(c, "t.yml", "job a if"))
    };

    assert_eq!(eval(Some("inputs.use-cache"), &ctx), Ok(true));
    assert_eq!(eval(Some("matrix.nfpm_arch"), &ctx), Ok(true));
    assert_eq!(
        eval(
            Some("runner.environment == 'SELF-HOSTED' && needs.classify.outputs.lane != 'docs'"),
            &ctx
        ),
        Ok(true)
    );
    assert_eq!(
        eval(
            Some("steps.flake-pr.outputs.pull-request-operation == 'created' || steps.flake-pr.outputs.pull-request-operation == 'updated'"),
            &ctx
        ),
        Ok(true)
    );
    let failed_ctx = ctx.clone().with_prior_succeeded(false);
    assert_eq!(eval(None, &failed_ctx), Ok(false));
    assert_eq!(eval(Some("always()"), &failed_ctx), Ok(true));
    assert_eq!(
        eval(Some("github.event_name == 'pull_request'"), &failed_ctx),
        Ok(false)
    );
    assert_eq!(
        eval(
            Some("always() && github.event_name == 'pull_request'"),
            &failed_ctx
        ),
        Ok(true)
    );
    let absent_ctx = ctx
        .clone()
        .with_step_output("cache", "cache-hit", CondVal::Absent);
    assert_eq!(
        eval(Some("steps.cache.outputs.cache-hit != 'true'"), &absent_ctx),
        Ok(true)
    );
    assert_eq!(
        eval(Some("steps.cache.outputs.cache-hit == ''"), &absent_ctx)
            .unwrap_err()
            .kind,
        RefusalKind::ConditionTypeMismatch
    );
    assert_eq!(
        eval(Some("inputs.use-cache == 'true'"), &ctx)
            .unwrap_err()
            .kind,
        RefusalKind::ConditionTypeMismatch
    );
    for (bad, expected_kind) in [
        ("${{ inputs.use-cache }}", RefusalKind::ConditionSyntax),
        ("!inputs.use-cache", RefusalKind::ConditionSyntax),
        ("(matrix.nfpm_arch || false) == true", RefusalKind::ConditionSyntax),
        (
            "github.event_name == 'push' || github.event_name == 'pull_request' && runner.environment == 'self-hosted'",
            RefusalKind::ConditionSyntax,
        ),
        ("contains(github.ref, 'main')", RefusalKind::ConditionSyntax),
        ("hashFiles('Cargo.lock')", RefusalKind::ConditionSyntax),
        ("failure()", RefusalKind::ConditionSyntax),
        ("github.event_name", RefusalKind::ConditionBarePath),
        ("needs.missing.result == 'success'", RefusalKind::ConditionUnbound),
    ] {
        let err = eval(Some(bad), &ctx).unwrap_err();
        assert_eq!(err.kind, expected_kind, "bad condition `{bad}`: {err}");
        assert!(err.to_string().contains("not a form this guard reads"), "{err}");
    }
}

#[test]
fn closed_form_closes_nightly_always_and_expression_emulation_findings() {
    let wfs = ShardWorkflows::load();

    // Finding 2 (#1417): dropping `always() &&` from nightly notify-failure fails
    // because implicit `success()` skips the notifier when macos-tests failed.
    let dropped_always = wfs.nightly.replacen(
        "if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "if: github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        1,
    );
    assert_ne!(dropped_always, wfs.nightly);
    let probs = wfs.with_nightly(dropped_always).problems();
    assert!(
        probs
            .iter()
            .any(|p| p
                .contains("notify-failure condition does not match failing nightly schedule only")),
        "dropping always() && must fail: {probs:?}"
    );

    // Finding 3 (#1417): parentheses or mixed && / || are refused.
    let parenthesized = wfs.nightly.replacen(
        "if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "if: always() && (github.event_name == 'schedule' && needs.macos-tests.result != 'success')",
        1,
    );
    let paren_probs = wfs.with_nightly(parenthesized).problems();
    assert!(
        paren_probs
            .iter()
            .any(|p| p.contains("macos-tests.yml (job notify-failure if)")
                && p.contains("not a form this guard reads")),
        "parenthesized condition must be refused: {paren_probs:?}"
    );

    let mixed_ops = wfs.ci.replacen(
        "    if: github.event_name != 'push'\n    runs-on: macos-latest\n",
        "    if: github.event_name == 'pull_request' || github.event_name == 'merge_group' && github.event_name != 'push'\n    runs-on: macos-latest\n",
        1,
    );
    assert_ne!(mixed_ops, wfs.ci);
    let mixed_probs = ci_macos_split_problems(&mixed_ops);
    assert!(
        mixed_probs
            .iter()
            .any(|p| p.contains("ci.yml (job build-macos if)")
                && p.contains("not a form this guard reads")),
        "mixed && / || must be refused: {mixed_probs:?}"
    );

    let coerce_expr = wfs.shards.replacen(
        "        if: matrix.machine == 'macos/1'\n",
        "        if: (matrix.machine || false) == true\n",
        1,
    );
    assert_ne!(coerce_expr, wfs.shards);
    let coerce_probs = wfs.with_shards(coerce_expr).problems();
    assert!(
        coerce_probs
            .iter()
            .any(|p| p.contains("macos-test-shards.yml")
                && p.contains("not a form this guard reads")),
        "coercing || expression must be refused: {coerce_probs:?}"
    );
}

#[test]
fn closed_form_closes_event_types_and_workflow_env_findings() {
    let wfs = ShardWorkflows::load();

    // Finding 4 (#1417): changing an event's `types` so the job no longer runs where required.
    let bad_pr_types = wfs.ci.replacen(
        "  pull_request:\n    types: [opened, synchronize, reopened]\n",
        "  pull_request:\n    types: [labeled]\n",
        1,
    );
    assert_ne!(bad_pr_types, wfs.ci);
    let pr_probs = ci_macos_split_problems(&bad_pr_types);
    assert!(
        pr_probs
            .iter()
            .any(|p| p.contains("build-macos does not run on pull_request")),
        "changed pull_request types must fail: {pr_probs:?}"
    );

    let bad_mg_types = wfs.ci.replacen(
        "  merge_group:\n    types: [checks_requested]\n",
        "  merge_group:\n    types: [destroyed]\n",
        1,
    );
    assert_ne!(bad_mg_types, wfs.ci);
    let mg_probs = ci_macos_split_problems(&bad_mg_types);
    assert!(
        mg_probs
            .iter()
            .any(|p| p.contains("build-macos does not run on pull_request and merge_group")),
        "changed merge_group types must fail: {mg_probs:?}"
    );

    let bad_dispatch_types = wfs.ci.replacen(
        "  workflow_dispatch:\n",
        "  workflow_dispatch:\n    types: [manual]\n",
        1,
    );
    assert_ne!(bad_dispatch_types, wfs.ci);
    let disp_probs = wfs.with_ci(bad_dispatch_types).problems();
    assert!(
        disp_probs
            .iter()
            .any(|p| p.contains("ci.yml's macos-machines does not run on")),
        "workflow_dispatch types must fail shard caller: {disp_probs:?}"
    );

    let bad_release_types = wfs.release.replacen(
        "  push:\n    tags: [\"v*\"]\n",
        "  push:\n    tags: [\"v*\"]\n    types: [created]\n",
        1,
    );
    assert_ne!(bad_release_types, wfs.release);
    let rel_probs = wfs.with_release(bad_release_types).problems();
    assert!(
        rel_probs
            .iter()
            .any(|p| p.contains("release.yml's macos-machines does not run on")),
        "release push types must fail shard caller: {rel_probs:?}"
    );

    // Finding 5 (#1417): workflow-level `env` added to a covered file is refused.
    let ci_top_env = wfs.ci.replacen(
        "permissions:\n",
        "env:\n  MACOS_HOST: '1'\n\npermissions:\n",
        1,
    );
    assert_ne!(ci_top_env, wfs.ci);
    let env_probs = ci_macos_split_problems(&ci_top_env);
    assert!(
        env_probs
            .iter()
            .any(|p| p.contains("ci.yml (top-level env)")
                && p.contains("not a form this guard reads")),
        "workflow-level env in ci.yml must be refused: {env_probs:?}"
    );
}

#[test]
fn closed_form_closes_shard_runner_matrix_cache_and_script_findings() {
    let wfs = ShardWorkflows::load();

    // Finding 6a: matrix `exclude` added to shard job is refused.
    let with_exclude = wfs.shards.replacen(
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n",
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n        exclude:\n          - machine: macos/2\n",
        1,
    );
    let excl_probs = wfs.with_shards(with_exclude).problems();
    assert!(
        excl_probs.iter().any(|p| p
            .contains("macos-test-shards.yml (job macos-machines strategy.matrix)")
            && p.contains("not a form this guard reads")),
        "matrix exclude must be refused: {excl_probs:?}"
    );

    // Finding 6b: shard runner changed off macos-latest.
    let bad_runner = wfs.shards.replacen(
        "    runs-on: macos-latest\n",
        "    runs-on: ubuntu-24.04\n",
        1,
    );
    let runner_probs = wfs.with_shards(bad_runner).problems();
    assert!(
        runner_probs
            .iter()
            .any(|p| p.contains("macos-machines does not run on macos-latest")),
        "changed shard runner must fail: {runner_probs:?}"
    );

    // Finding 6c: second cache step left ungated while the original remains gated.
    let second_cache = wfs.shards.replacen(
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n        if: inputs.use-cache\n        with:\n          shared-key: build-macos\n",
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n        if: inputs.use-cache\n        with:\n          shared-key: build-macos\n      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n        if: matrix.machine == 'macos/1'\n",
        1,
    );
    assert_ne!(second_cache, wfs.shards);
    let cache_probs = wfs.with_shards(second_cache).problems();
    assert!(
        cache_probs
            .iter()
            .any(|p| p.contains("cache is not gated on inputs.use-cache")),
        "second cache step with changed condition must fail: {cache_probs:?}"
    );

    // Finding 6d: checked scripts changed only inside a shell comment and only outside one.
    let gate_a_comment = wfs.shards.replacen(
        "          set -euo pipefail\n",
        "          # added comment only\n          set -euo pipefail\n",
        1,
    );
    assert!(wfs
        .with_shards(gate_a_comment)
        .problems()
        .iter()
        .any(|p| p.contains("lost Gate A scoped to macos/1")));
    let gate_a_code = wfs.shards.replacen(
        "          grep -q 'Gate A startup verdict' target/r3-startup-report.txt\n",
        "          true\n",
        1,
    );
    assert!(wfs
        .with_shards(gate_a_code)
        .problems()
        .iter()
        .any(|p| p.contains("lost Gate A scoped to macos/1")));
    let bundle_comment = wfs.shards.replacen(
        "          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
        "          # bundle comment\n          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
        1,
    );
    assert!(wfs
        .with_shards(bundle_comment)
        .problems()
        .iter()
        .any(|p| p.contains("lost recipes/self and recipes/verify compile")));
    let bundle_code = wfs.shards.replacen(
        "          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
        "          # cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
        1,
    );
    assert!(wfs
        .with_shards(bundle_code)
        .problems()
        .iter()
        .any(|p| p.contains("lost recipes/self and recipes/verify compile")));
}

#[test]
fn closed_form_closes_gh_stub_paginated_create_and_foreign_repo_findings() {
    let wfs = ShardWorkflows::load();

    // Finding 7 (#1417): gh stub rejects paginated create and foreign repository slugs.
    let paginated_create = wfs.nightly.replacen(
        "gh api -X POST \"repos/${REPO}/issues\"",
        "gh api --paginate -X POST \"repos/${REPO}/issues\"",
        1,
    );
    assert_ne!(paginated_create, wfs.nightly);
    let pag_probs = wfs.with_nightly(paginated_create).problems();
    assert!(
        pag_probs
            .iter()
            .any(|p| p.contains("unexpected gh call: POST paginate=true repos/owner/repo/issues")),
        "paginated create must fail: {pag_probs:?}"
    );

    let other_repo = wfs.nightly.replacen(
        "gh api -X POST \"repos/${REPO}/issues\"",
        "gh api -X POST \"repos/other/repo/issues\"",
        1,
    );
    assert_ne!(other_repo, wfs.nightly);
    let repo_probs = wfs.with_nightly(other_repo).problems();
    assert!(
        repo_probs
            .iter()
            .any(|p| p.contains("unexpected gh call: POST paginate=false repos/other/repo/issues")),
        "foreign repository slug must fail: {repo_probs:?}"
    );
}
