//! Nightly `macos-tests.yml` concurrency, gate, whole-script identity, and
//! typed `gh` behavior guards (#1383, #1396, #1417).

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use super::{parse_or_problem, SplitProblem};
use crate::workflow::{
    ClosedJob, ClosedStep, ClosedWorkflow, ConditionContext, EventExpectation, EventTrigger,
    Permissions, Scalar,
};

const JOB_RESULTS: [&str; 4] = ["success", "failure", "cancelled", "skipped"];
const MAX_NOTIFY_NEEDS: usize = 4;

pub(super) const NOTIFY_FAILURE_SCRIPT: &str = "set -euo pipefail\ngh api \"repos/${REPO}/labels/${LABEL}\" >/dev/null 2>&1 || \\\n  gh api -X POST \"repos/${REPO}/labels\" \\\n    -f name=\"$LABEL\" \\\n    -f color=\"b60205\" \\\n    -f description=\"Nightly macOS test suite failure\" >/dev/null\nfailed_jobs=\"$(gh api --paginate \"repos/${REPO}/actions/runs/${RUN_ID}/jobs?per_page=100\" \\\n  --jq '.jobs[] | select(.conclusion != null and .conclusion != \"success\" and .conclusion != \"skipped\") | \"- `\\(.name)` (\\(.conclusion)): \\(.html_url)\"')\"\nif [ -z \"$failed_jobs\" ]; then\n  failed_jobs=\"- \\`test (macos-latest)\\` (${VERDICT})\"\nfi\ntoday=\"$(date -u +%Y-%m-%d)\"\nbody=\"$(printf 'Nightly macOS test run failed on %s (`%s`).\\n\\n- Run: %s\\n- Verdict: `%s`\\n\\n## Failed jobs\\n\\n%s\\n\\nA passing nightly leaves this issue open; close it once investigated.' \"$today\" \"$HEAD_SHA\" \"$RUN_URL\" \"$VERDICT\" \"$failed_jobs\")\"\nexisting=\"$(gh api --paginate \"repos/${REPO}/issues?labels=${LABEL}&state=open&per_page=100\" \\\n  --jq '.[] | select(.pull_request == null) | .number' \\\n  | awk '{ n = $1 + 0; if (!seen || n < min) { min = n; seen = 1 } } END { if (seen) print min }')\"\nif [ -n \"$existing\" ]; then\n  gh api -X PATCH \"repos/${REPO}/issues/${existing}\" -f body=\"$body\" >/dev/null\nelse\n  gh api -X POST \"repos/${REPO}/issues\" \\\n    -f title=\"Nightly macOS tests failed\" \\\n    -f body=\"$body\" \\\n    -f \"labels[]=${LABEL}\" >/dev/null\nfi\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NotifyEnvReason {
    UndeclaredNeedsResult { expr: String, job_id: String },
    UnsupportedExpression { expr: String },
    UnclosedExpression { raw: String },
    StrayClosingBraces { raw: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ConcurrencyProblem {
    MissingBlock,
    UnreadableGroup,
    MultilineOrEmptyGroup(String),
    InvalidExpression { group: String, reason: &'static str },
    SharedSlot(String),
    DisallowedExpression { group: String, expr: String },
    CancelInProgressNotFalse,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum NotifyBehaviorProblem {
    MissingGhToken {
        scenario: &'static str,
    },
    UnexpectedGhSubcommand {
        scenario: &'static str,
        detail: String,
    },
    UnexpectedGhFlag {
        scenario: &'static str,
        detail: String,
    },
    UnexpectedGhExtraArg {
        scenario: &'static str,
        detail: String,
    },
    UnexpectedGhCall {
        scenario: &'static str,
        detail: String,
    },
    ScriptExitedNonZero {
        scenario: &'static str,
        code: Option<i32>,
    },
    MissingPaginatedJobsLookup {
        scenario: &'static str,
    },
    MissingPaginatedIssuesLookup {
        scenario: &'static str,
    },
    UnexpectedRepoEndpoint {
        scenario: &'static str,
        endpoint: String,
    },
    PaginatedWriteCall {
        scenario: &'static str,
        method: String,
        endpoint: String,
    },
    UnexpectedPatches {
        scenario: &'static str,
        actual: Vec<String>,
        expected: Option<String>,
    },
    PatchMissingRunBody {
        scenario: &'static str,
    },
    UnexpectedPostCount {
        scenario: &'static str,
        actual: usize,
        expected: usize,
    },
    PostMissingLabel {
        scenario: &'static str,
    },
    PostMissingTitle {
        scenario: &'static str,
    },
    PostMissingRunBody {
        scenario: &'static str,
    },
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

pub(super) fn nightly_concurrency_problems(wf: &ClosedWorkflow) -> Vec<ConcurrencyProblem> {
    let mut problems = Vec::new();
    let Some(conc) = wf.concurrency() else {
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
stub_dir="$(cd "$(dirname "$0")" && pwd)"
log_file="${stub_dir}/calls.jsonl"
err_file="${stub_dir}/stub_error.json"
issues_file="${stub_dir}/issues.json"
fail_stub() {
  jq -nc --arg kind "$1" --arg detail "$2" '{kind:$kind,detail:$detail}' > "$err_file"
  exit 2
}
[ -n "${GH_TOKEN:-}" ] || fail_stub "missing_gh_token" ""
[ "${1:-}" = "api" ] || fail_stub "unexpected_subcommand" "${1:-}"
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
    -*) fail_stub "unexpected_flag" "$1" ;;
    *) [ -z "$ep" ] && ep="$1" || fail_stub "unexpected_extra_arg" "$1"; shift ;;
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
  *) fail_stub "unexpected_call" "$m paginate=$p $ep" ;;
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StubErrorKind {
    MissingGhToken,
    UnexpectedSubcommand,
    UnexpectedFlag,
    UnexpectedExtraArg,
    UnexpectedCall,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StubGhError {
    kind: StubErrorKind,
    detail: String,
}

struct NotifyScenario {
    label: &'static str,
    issues_json: &'static str,
    expected_patch: Option<&'static str>,
    expected_posts: usize,
}

pub(super) struct RunnableStep<'a> {
    pub(super) script: &'a str,
    pub(super) env: BTreeMap<String, String>,
}

fn eval_notify_env_expr(
    expr: &str,
    declared_needs: &[String],
) -> Result<&'static str, NotifyEnvReason> {
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
                return Err(NotifyEnvReason::UndeclaredNeedsResult {
                    expr: other.to_string(),
                    job_id: job_id.to_string(),
                });
            }
            Err(NotifyEnvReason::UnsupportedExpression {
                expr: other.to_string(),
            })
        }
    }
}

fn expand_notify_env_value(
    raw: &str,
    declared_needs: &[String],
) -> Result<String, NotifyEnvReason> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some((before, after_open)) = rest.split_once("${{") {
        let Some((inner, after_close)) = after_open.split_once("}}") else {
            return Err(NotifyEnvReason::UnclosedExpression {
                raw: raw.to_string(),
            });
        };
        out.push_str(before);
        out.push_str(eval_notify_env_expr(inner.trim(), declared_needs)?);
        rest = after_close;
    }
    if rest.contains("}}") {
        return Err(NotifyEnvReason::StrayClosingBraces {
            raw: raw.to_string(),
        });
    }
    out.push_str(rest);
    Ok(out)
}

fn collect_notify_steps<'a>(
    job: &'a ClosedJob,
    problems: &mut Vec<SplitProblem>,
) -> Vec<RunnableStep<'a>> {
    let mut job_ctx = ConditionContext::default_ok()
        .with_event("schedule")
        .with_prior_succeeded(false);
    for dep in job.needs() {
        job_ctx = job_ctx.with_needs_result(dep, "failure");
    }
    if job.allows_failure() {
        problems.push(SplitProblem::NotifyJobAllowsFailure);
        return Vec::new();
    }
    if let Some(p) = job
        .runs_under(&job_ctx)
        .expect_true(|| SplitProblem::NotifyJobSkippedOnFailure)
    {
        problems.push(p);
        return Vec::new();
    }
    let mut step_ctx = ConditionContext::default_ok().with_event("schedule");
    for dep in job.needs() {
        step_ctx = step_ctx.with_needs_result(dep, "failure");
    }
    let mut runnable = Vec::new();
    for (idx, step) in job.steps.iter().enumerate() {
        let Some(script) = step.run() else {
            continue;
        };
        let mut allowed = false;
        if let Some(p) = step.runs_under(&step_ctx).visit_when_allowed(|| {
            allowed = true;
        }) {
            problems.push(p);
            return Vec::new();
        }
        if !allowed {
            continue;
        }
        if step.allows_failure() {
            problems.push(SplitProblem::NotifyStepAllowsFailure {
                step_index: idx + 1,
            });
            return Vec::new();
        }
        let Some(env) = expand_step_env(job, step, problems) else {
            return Vec::new();
        };
        runnable.push(RunnableStep { script, env });
    }
    if runnable.is_empty() {
        problems.push(SplitProblem::NotifyMissingScriptStep);
    }
    runnable
}

fn expand_step_env(
    job: &ClosedJob,
    step: &ClosedStep,
    problems: &mut Vec<SplitProblem>,
) -> Option<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for layer in [job.env(), step.env()] {
        for (k, v) in layer {
            match expand_notify_env_value(v, job.needs()) {
                Ok(expanded) => {
                    env.insert(k.clone(), expanded);
                }
                Err(reason) => {
                    problems.push(SplitProblem::NotifyEnvError {
                        key: k.clone(),
                        reason,
                    });
                    return None;
                }
            }
        }
    }
    Some(env)
}

fn verify_notify_scenario(
    dir: &Path,
    path_env: &str,
    step: &RunnableStep<'_>,
    scenario: &NotifyScenario,
    problems: &mut Vec<NotifyBehaviorProblem>,
) {
    let log_path = dir.join("calls.jsonl");
    let err_path = dir.join("stub_error.json");
    let issues_path = dir.join("issues.json");
    std::fs::write(&log_path, "").expect("init stub log");
    let _ = std::fs::remove_file(&err_path);
    std::fs::write(&issues_path, scenario.issues_json).expect("write stub issues");
    let output = Command::new("bash")
        .args(["-e", "-c", step.script])
        .env_clear()
        .env("PATH", path_env)
        .envs(&step.env)
        .output()
        .expect("bash");
    let log = std::fs::read_to_string(&log_path).expect("read stub log");
    let calls: Vec<GhCall> = log
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("parse stub gh jsonl"))
        .collect();
    let call_problems_before = problems.len();
    check_notify_call_safety(&calls, scenario, problems);
    if !output.status.success() {
        record_script_failure(
            &err_path,
            output.status.code(),
            scenario.label,
            call_problems_before,
            problems,
        );
        return;
    }
    check_notify_call_outcomes(&calls, scenario, problems);
}

fn record_script_failure(
    err_path: &Path,
    exit_code: Option<i32>,
    scenario: &'static str,
    call_problems_before: usize,
    problems: &mut Vec<NotifyBehaviorProblem>,
) {
    if let Ok(raw) = std::fs::read_to_string(err_path) {
        let stub_err: StubGhError = serde_json::from_str(&raw).expect("parse stub_error.json");
        let p = match stub_err.kind {
            StubErrorKind::MissingGhToken => NotifyBehaviorProblem::MissingGhToken { scenario },
            StubErrorKind::UnexpectedSubcommand => NotifyBehaviorProblem::UnexpectedGhSubcommand {
                scenario,
                detail: stub_err.detail,
            },
            StubErrorKind::UnexpectedFlag => NotifyBehaviorProblem::UnexpectedGhFlag {
                scenario,
                detail: stub_err.detail,
            },
            StubErrorKind::UnexpectedExtraArg => NotifyBehaviorProblem::UnexpectedGhExtraArg {
                scenario,
                detail: stub_err.detail,
            },
            StubErrorKind::UnexpectedCall => {
                if problems.len() > call_problems_before {
                    return;
                }
                NotifyBehaviorProblem::UnexpectedGhCall {
                    scenario,
                    detail: stub_err.detail,
                }
            }
        };
        problems.push(p);
    } else {
        problems.push(NotifyBehaviorProblem::ScriptExitedNonZero {
            scenario,
            code: exit_code,
        });
    }
}

fn check_notify_call_safety(
    calls: &[GhCall],
    scenario: &NotifyScenario,
    problems: &mut Vec<NotifyBehaviorProblem>,
) {
    for c in calls {
        if !c.endpoint.starts_with("repos/owner/repo/") {
            problems.push(NotifyBehaviorProblem::UnexpectedRepoEndpoint {
                scenario: scenario.label,
                endpoint: c.endpoint.clone(),
            });
        }
        if matches!(c.method.as_str(), "POST" | "PATCH") && c.paginate {
            problems.push(NotifyBehaviorProblem::PaginatedWriteCall {
                scenario: scenario.label,
                method: c.method.clone(),
                endpoint: c.endpoint.clone(),
            });
        }
    }
}

fn has_run_body(fields: &[String]) -> bool {
    fields.iter().any(|f| {
        f.strip_prefix("body=").is_some_and(|b| {
            b.contains("https://example.invalid/owner/repo/actions/runs/777")
                && b.contains("deadbeef")
        })
    })
}

fn check_notify_call_outcomes(
    calls: &[GhCall],
    scenario: &NotifyScenario,
    problems: &mut Vec<NotifyBehaviorProblem>,
) {
    if !calls.iter().any(|c| {
        c.method == "GET"
            && c.paginate
            && c.endpoint
                .starts_with("repos/owner/repo/actions/runs/777/jobs")
    }) {
        problems.push(NotifyBehaviorProblem::MissingPaginatedJobsLookup {
            scenario: scenario.label,
        });
    }
    if !calls.iter().any(|c| {
        c.method == "GET"
            && c.paginate
            && c.endpoint
                .starts_with("repos/owner/repo/issues?labels=macos-nightly&state=open")
    }) {
        problems.push(NotifyBehaviorProblem::MissingPaginatedIssuesLookup {
            scenario: scenario.label,
        });
    }
    let patches: Vec<&GhCall> = calls.iter().filter(|c| c.method == "PATCH").collect();
    let patch_endpoints: Vec<String> = patches.iter().map(|c| c.endpoint.clone()).collect();
    match scenario.expected_patch {
        Some(expected) if patch_endpoints.as_slice() != [expected] => {
            problems.push(NotifyBehaviorProblem::UnexpectedPatches {
                scenario: scenario.label,
                actual: patch_endpoints,
                expected: Some(expected.to_string()),
            });
        }
        Some(_) if !has_run_body(&patches[0].fields) => {
            problems.push(NotifyBehaviorProblem::PatchMissingRunBody {
                scenario: scenario.label,
            });
        }
        None if !patches.is_empty() => {
            problems.push(NotifyBehaviorProblem::UnexpectedPatches {
                scenario: scenario.label,
                actual: patch_endpoints,
                expected: None,
            });
        }
        _ => {}
    }
    let posts: Vec<&GhCall> = calls
        .iter()
        .filter(|c| c.method == "POST" && c.endpoint != "repos/owner/repo/labels")
        .collect();
    if posts.len() != scenario.expected_posts {
        problems.push(NotifyBehaviorProblem::UnexpectedPostCount {
            scenario: scenario.label,
            actual: posts.len(),
            expected: scenario.expected_posts,
        });
    }
    for post in posts {
        if !post.fields.iter().any(|f| f == "labels[]=macos-nightly") {
            problems.push(NotifyBehaviorProblem::PostMissingLabel {
                scenario: scenario.label,
            });
        }
        if !post
            .fields
            .iter()
            .any(|f| f.strip_prefix("title=").is_some_and(|t| !t.is_empty()))
        {
            problems.push(NotifyBehaviorProblem::PostMissingTitle {
                scenario: scenario.label,
            });
        }
        if !has_run_body(&post.fields) {
            problems.push(NotifyBehaviorProblem::PostMissingRunBody {
                scenario: scenario.label,
            });
        }
    }
}

pub(super) fn run_notify_script_scenarios(step: &RunnableStep<'_>) -> Vec<NotifyBehaviorProblem> {
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
    let mut problems = Vec::new();
    for scenario in &scenarios {
        verify_notify_scenario(dir.path(), &path_env, step, scenario, &mut problems);
    }
    problems
}

pub(super) fn default_notify_env() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("GH_TOKEN".into(), "stub-github-token".into()),
        ("REPO".into(), "owner/repo".into()),
        ("RUN_ID".into(), "777".into()),
        (
            "RUN_URL".into(),
            "https://example.invalid/owner/repo/actions/runs/777".into(),
        ),
        ("HEAD_SHA".into(), "deadbeef".into()),
        ("VERDICT".into(), "failure".into()),
        ("LABEL".into(), "macos-nightly".into()),
    ])
}

fn run_notify_failure_behavior(job: &ClosedJob, problems: &mut Vec<SplitProblem>) {
    let steps = collect_notify_steps(job, problems);
    if steps.is_empty() {
        return;
    }
    if steps.len() > 1 {
        for idx in 1..steps.len() {
            problems.push(SplitProblem::NotifyExtraScriptStep {
                step_index: idx + 1,
            });
        }
        return;
    }
    // Whole-script identity MUST be established BEFORE spawning bash (#1417 G9).
    if steps[0].script != NOTIFY_FAILURE_SCRIPT {
        problems.push(SplitProblem::NotifyScriptChanged);
        return;
    }
    for p in run_notify_script_scenarios(&steps[0]) {
        problems.push(SplitProblem::NotifyBehavior(p));
    }
}

pub(super) fn check_nightly_workflow(nightly: &str, problems: &mut Vec<SplitProblem>) {
    let Some(wf) = parse_or_problem("macos-tests.yml", nightly, problems) else {
        return;
    };
    if wf.event_names() != ["schedule", "workflow_dispatch"]
        || !wf
            .event("schedule")
            .is_some_and(EventTrigger::has_active_schedule)
        || wf.unfiltered_event("workflow_dispatch").is_none()
    {
        problems.push(SplitProblem::NightlyInvalidEvents);
    }
    for p in nightly_concurrency_problems(&wf) {
        problems.push(SplitProblem::NightlyConcurrency(p));
    }
    let job = match wf.require_job("macos-tests.yml", "notify-failure") {
        Ok(job) => job,
        Err(err) => {
            problems.push(SplitProblem::MissingJob(err));
            return;
        }
    };
    check_notify_job_gate(&wf, job, problems);
    let expected_perms = Permissions::Scopes(BTreeMap::from([
        ("actions".to_string(), "read".to_string()),
        ("issues".to_string(), "write".to_string()),
    ]));
    if job.permissions() != Some(&expected_perms) {
        problems.push(SplitProblem::NotifyWrongPermissions);
    }
    run_notify_failure_behavior(job, problems);
}

fn enumerate_need_assignments(deps: &[String]) -> Vec<Vec<(&str, &'static str)>> {
    let mut rows: Vec<Vec<(&str, &'static str)>> = vec![Vec::new()];
    for dep in deps {
        let mut next = Vec::with_capacity(rows.len() * JOB_RESULTS.len());
        for row in &rows {
            for &res in &JOB_RESULTS {
                let mut extended = row.clone();
                extended.push((dep.as_str(), res));
                next.push(extended);
            }
        }
        rows = next;
    }
    rows
}

fn check_notify_job_gate(wf: &ClosedWorkflow, job: &ClosedJob, problems: &mut Vec<SplitProblem>) {
    if !job.needs().iter().any(|n| n == "macos-tests") {
        problems.push(SplitProblem::NotifyMissingMacosTestsNeeds);
        return;
    }
    if job.needs().len() > MAX_NOTIFY_NEEDS {
        problems.push(SplitProblem::NotifyTooManyNeeds(job.needs().len()));
        return;
    }
    let assignments = enumerate_need_assignments(job.needs());
    for row in &assignments {
        let prior_ok = row.iter().all(|&(_, res)| res == "success");
        let verdict_res = row
            .iter()
            .find_map(|&(dep, res)| (dep == "macos-tests").then_some(res))
            .expect("macos-tests is in needs");
        let mut ctx = ConditionContext::default_ok().with_prior_succeeded(prior_ok);
        for &(dep, res) in row {
            ctx = ctx.with_needs_result(dep, res);
        }
        let sched_verdict = wf.event_reaches_job(job, &EventExpectation::ActiveSchedule, &ctx);
        let sched_problem = if verdict_res == "success" {
            sched_verdict.expect_false(|| SplitProblem::NotifyGateMismatch {
                event: "schedule".to_string(),
                verdict_result: verdict_res.to_string(),
            })
        } else {
            sched_verdict.expect_true(|| SplitProblem::NotifyGateMismatch {
                event: "schedule".to_string(),
                verdict_result: verdict_res.to_string(),
            })
        };
        if let Some(p) = sched_problem {
            problems.push(p);
            return;
        }
        for ev in wf.events() {
            if ev.name == "schedule" {
                continue;
            }
            let non_sched_ctx = ctx.clone().with_event(&ev.name);
            if let Some(p) =
                job.runs_under(&non_sched_ctx)
                    .expect_false(|| SplitProblem::NotifyGateMismatch {
                        event: ev.name.clone(),
                        verdict_result: verdict_res.to_string(),
                    })
            {
                problems.push(p);
                return;
            }
        }
    }
}
