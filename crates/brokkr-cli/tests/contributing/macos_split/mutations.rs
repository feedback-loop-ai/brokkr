//! Guard tests and workflow-text mutation proofs for the macOS split and
//! closed-form admission (#1383, #1396, #1417 round 2).
//!
//! Every mutation test asserts exact typed `SplitProblem`, `ClosedFormRefusal`,
//! `RefusalKind`, `ConcurrencyProblem`, or `NotifyBehaviorProblem` values; the
//! human-readable refusal sentence format is pinned once in its own test
//! (decision 0071 rulings 8 and 9).

use super::notify::{
    default_notify_env, nightly_concurrency_problems, run_notify_script_scenarios,
    ConcurrencyProblem, NotifyBehaviorProblem, NotifyEnvReason, RunnableStep,
    NOTIFY_FAILURE_SCRIPT,
};
use super::{ci_macos_split_problems, ShardWorkflows, SplitProblem};
use crate::contributing::workspace;
use crate::workflow::{
    ArtifactPattern, ClosedCondition, ClosedFormRefusal, ClosedWorkflow, CondVal, ConditionContext,
    RefusalKind, Scalar, Workflow, WorkflowError,
};

#[derive(Debug, PartialEq, Eq)]
enum EvalCheck {
    Mismatch,
    Refused(ClosedFormRefusal),
}

impl From<ClosedFormRefusal> for EvalCheck {
    fn from(r: ClosedFormRefusal) -> Self {
        Self::Refused(r)
    }
}

fn has_refusal(problems: &[SplitProblem], file: &str, place: &str, kind: RefusalKind) -> bool {
    problems.iter().any(|p| {
        matches!(
            p,
            SplitProblem::ClosedForm(r) if r.file == file && r.place == place && r.kind == kind
        )
    })
}

#[test]
fn pull_request_path_replaces_macos_shards_with_macos_build() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();
    assert_eq!(ci_macos_split_problems(&ci), Vec::<SplitProblem>::new());

    let mut_no_run = ci.replacen("--locked --no-run", "--locked", 1);
    let no_run_probs = ci_macos_split_problems(&mut_no_run);
    assert!(no_run_probs.contains(&SplitProblem::BuildMacosMissingCompile));
    assert!(no_run_probs.contains(&SplitProblem::BuildMacosUnexpectedRunStep { step_index: 4 }));

    let mut_perm = ci.replacen(
        "permissions:\n  contents: read\n",
        "permissions:\n  contents: read\n  issues: write\n",
        1,
    );
    assert_ne!(mut_perm, ci);
    assert_eq!(
        ci_macos_split_problems(&mut_perm),
        vec![SplitProblem::CiIssuesWriteGranted]
    );

    let filtered_paths = ci.replacen(
        "  pull_request:\n    types: [opened, synchronize, reopened]\n",
        "  pull_request:\n    types: [opened, synchronize, reopened]\n    paths: [crates/**]\n",
        1,
    );
    assert_ne!(filtered_paths, ci);
    let paths_probs = ci_macos_split_problems(&filtered_paths);
    assert!(
        has_refusal(
            &paths_probs,
            "ci.yml",
            "on.pull_request",
            RefusalKind::EventFilter
        ),
        "paths filter must be refused: {paths_probs:?}"
    );

    let filtered_branches = ci.replacen(
        "  pull_request:\n    types: [opened, synchronize, reopened]\n",
        "  pull_request:\n    types: [opened, synchronize, reopened]\n    branches: [other]\n",
        1,
    );
    assert_ne!(filtered_branches, ci);
    assert!(ci_macos_split_problems(&filtered_branches).contains(
        &SplitProblem::CiMissingTrigger {
            event: "pull_request".to_string(),
        }
    ));

    let key_macos = ci.replacen(
        "  engine:\n    name: test (ubuntu-latest)\n",
        "  engine:\n    name: test (ubuntu-latest)\n    env:\n      MACOS_HOST: '1'\n",
        1,
    );
    assert_ne!(key_macos, ci);
    assert_eq!(
        ci_macos_split_problems(&key_macos),
        vec![SplitProblem::CiJobMentionsMacos {
            job_id: "engine".to_string(),
        }]
    );

    let bypass_false = ci.replacen(
        "      - name: workspace and test targets compile on macOS\n",
        "      - name: workspace and test targets compile on macOS\n        if: false\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&bypass_false),
        "ci.yml",
        "job build-macos step 4 if",
        RefusalKind::ConditionSyntax,
    ));

    let bypass_coe = ci.replacen(
        "      - name: workspace and test targets compile on macOS\n",
        "      - name: workspace and test targets compile on macOS\n        continue-on-error: true\n",
        1,
    );
    assert_eq!(
        ci_macos_split_problems(&bypass_coe),
        vec![SplitProblem::BuildMacosMissingCompile]
    );

    let bypass_vars = ci.replacen(
        "      - name: workspace and test targets compile on macOS\n",
        "      - name: workspace and test targets compile on macOS\n        if: vars.ENABLE_BUILD\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&bypass_vars),
        "ci.yml",
        "job build-macos step 4 if",
        RefusalKind::ConditionSyntax,
    ));
}

#[test]
fn macos_test_shards_run_nightly_on_dispatch_and_in_release_with_failure_issue() {
    let wfs = ShardWorkflows::load();
    assert_eq!(wfs.problems(), Vec::<SplitProblem>::new());

    let mut_release_pub = wfs
        .release
        .replacen("needs: [build, macos-tests]", "needs: build", 1);
    assert_eq!(
        wfs.with_release(mut_release_pub).problems(),
        vec![SplitProblem::ReleaseJobMissingMacosTestsNeeds {
            job_id: "publish".to_string(),
        }]
    );

    let mut_nightly_cond = wfs.nightly.replacen(
        "github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "github.event_name == 'schedule'",
        1,
    );
    assert_ne!(mut_nightly_cond, wfs.nightly);
    assert!(wfs.with_nightly(mut_nightly_cond).problems().contains(
        &SplitProblem::NotifyGateMismatch {
            event: "schedule".to_string(),
            verdict_result: "success".to_string(),
        }
    ));

    let no_schedule = wfs
        .nightly
        .replacen("  schedule:\n    - cron: '17 4 * * *'\n", "", 1);
    assert_ne!(no_schedule, wfs.nightly);
    assert!(wfs
        .with_nightly(no_schedule)
        .problems()
        .contains(&SplitProblem::NightlyInvalidEvents));

    let mut_no_machine = wfs.shards.replacen(
        "        env:\n          BROKKR_TEST_MACHINE: ${{ matrix.machine }}\n",
        "",
        1,
    );
    assert_ne!(mut_no_machine, wfs.shards);
    assert_eq!(
        wfs.with_shards(mut_no_machine).problems(),
        vec![SplitProblem::ShardsMissingShardStep]
    );

    for (bypass, shard_kind) in [
        ("        if: false\n", Some(RefusalKind::ConditionSyntax)),
        ("        if: matrix.machine == 'macos/1'\n", None),
        (
            "        if: vars.RUN_SHARDS\n",
            Some(RefusalKind::ConditionSyntax),
        ),
        ("        continue-on-error: true\n", None),
    ] {
        let shard_mut = wfs.shards.replacen(
            "      - run: bash scripts/test-shards.sh test --parallel-heaps\n",
            &format!("      - run: bash scripts/test-shards.sh test --parallel-heaps\n{bypass}"),
            1,
        );
        let shard_probs = wfs.with_shards(shard_mut).problems();
        match shard_kind {
            Some(kind) => assert!(has_refusal(
                &shard_probs,
                "macos-test-shards.yml",
                "job macos-machines step 4 if",
                kind,
            )),
            None => assert_eq!(shard_probs, vec![SplitProblem::ShardsMissingShardStep]),
        }

        let bundle_mut = wfs.shards.replacen(
            "      - name: self and verify bundles compile under the constitutional lint\n",
            &format!(
                "      - name: self and verify bundles compile under the constitutional lint\n{bypass}"
            ),
            1,
        );
        let bundle_probs = wfs.with_shards(bundle_mut).problems();
        match shard_kind {
            Some(kind) => assert!(has_refusal(
                &bundle_probs,
                "macos-test-shards.yml",
                "job macos-machines step 7 if",
                kind,
            )),
            None => assert_eq!(bundle_probs, vec![SplitProblem::ShardsMissingBundles]),
        }
    }

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
        nightly_concurrency_problems(&ClosedWorkflow::parse("macos-tests.yml", &shared).unwrap()),
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
            nightly_concurrency_problems(
                &ClosedWorkflow::parse("macos-tests.yml", &mutated).unwrap()
            ),
            vec![ConcurrencyProblem::DisallowedExpression {
                group: group.to_string(),
                expr: expr.to_string(),
            }],
            "bad group `{bad_group}` must fail"
        );
    }

    let ok_folded = wfs.nightly.replacen(
        "  group: macos-tests-${{ github.event_name }}\n",
        "  group: >-\n    macos-tests-${{ github.event_name }}\n",
        1,
    );
    assert_eq!(
        wfs.with_nightly(ok_folded).problems(),
        Vec::<SplitProblem>::new()
    );
    let mut_cancel_true = wfs.nightly.replacen(
        "  cancel-in-progress: false\n",
        "  cancel-in-progress: true\n",
        1,
    );
    assert_eq!(
        nightly_concurrency_problems(
            &ClosedWorkflow::parse("macos-tests.yml", &mut_cancel_true).unwrap()
        ),
        vec![ConcurrencyProblem::CancelInProgressNotFalse]
    );
}

#[test]
fn notify_failure_harness_enforces_gh_fields_methods_conditions_and_workflow_env() {
    // Direct harness proofs (typed NotifyBehaviorProblem, never stderr substrings; G10).
    let base_env = default_notify_env();
    assert_eq!(
        run_notify_script_scenarios(&RunnableStep {
            script: NOTIFY_FAILURE_SCRIPT,
            env: base_env.clone(),
        }),
        vec![]
    );

    // Unconditional issue create after update branch, both explicit and implicit POST.
    for create_call in [
        "  gh api -X POST \"repos/${REPO}/issues\" \\\n",
        "  gh api \"repos/${REPO}/issues\" \\\n",
    ] {
        let mut_script = NOTIFY_FAILURE_SCRIPT
            .replacen(
                "if [ -n \"$existing\" ]; then\n  gh api -X PATCH \"repos/${REPO}/issues/${existing}\" -f body=\"$body\" >/dev/null\nelse\n  gh api -X POST \"repos/${REPO}/issues\" \\\n",
                &format!("if [ -n \"$existing\" ]; then\n  gh api -X PATCH \"repos/${{REPO}}/issues/${{existing}}\" -f body=\"$body\" >/dev/null\nfi\n{create_call}"),
                1,
            )
            .replacen(
                "    -f \"labels[]=${LABEL}\" >/dev/null\nfi\n",
                "    -f \"labels[]=${LABEL}\" >/dev/null\n",
                1,
            );
        assert_ne!(mut_script, NOTIFY_FAILURE_SCRIPT);
        let probs = run_notify_script_scenarios(&RunnableStep {
            script: &mut_script,
            env: base_env.clone(),
        });
        assert_eq!(
            probs,
            vec![NotifyBehaviorProblem::UnexpectedPostCount {
                scenario: "existing-open-issue",
                actual: 1,
                expected: 0,
            }]
        );
    }

    let no_label = NOTIFY_FAILURE_SCRIPT.replacen(
        "    -f body=\"$body\" \\\n    -f \"labels[]=${LABEL}\" >/dev/null\n",
        "    -f body=\"$body\" >/dev/null\n",
        1,
    );
    assert_eq!(
        run_notify_script_scenarios(&RunnableStep {
            script: &no_label,
            env: base_env.clone(),
        }),
        vec![NotifyBehaviorProblem::PostMissingLabel {
            scenario: "no-open-issue",
        }]
    );

    let unpaginated = NOTIFY_FAILURE_SCRIPT.replacen(
        "existing=\"$(gh api --paginate \"repos/${REPO}/issues?labels=${LABEL}&state=open",
        "existing=\"$(gh api \"repos/${REPO}/issues?labels=${LABEL}&state=open",
        1,
    );
    assert!(
        run_notify_script_scenarios(&RunnableStep {
            script: &unpaginated,
            env: base_env.clone(),
        })
        .contains(&NotifyBehaviorProblem::UnexpectedGhCall {
            scenario: "existing-open-issue",
            detail: "GET paginate=false repos/owner/repo/issues?labels=macos-nightly&state=open&per_page=100"
                .to_string(),
        })
    );

    let paginated_create = NOTIFY_FAILURE_SCRIPT.replacen(
        "gh api -X POST \"repos/${REPO}/issues\"",
        "gh api --paginate -X POST \"repos/${REPO}/issues\"",
        1,
    );
    assert_eq!(
        run_notify_script_scenarios(&RunnableStep {
            script: &paginated_create,
            env: base_env.clone(),
        }),
        vec![NotifyBehaviorProblem::PaginatedWriteCall {
            scenario: "no-open-issue",
            method: "POST".to_string(),
            endpoint: "repos/owner/repo/issues".to_string(),
        }]
    );

    let other_repo = NOTIFY_FAILURE_SCRIPT.replacen(
        "gh api -X POST \"repos/${REPO}/issues\"",
        "gh api -X POST \"repos/other/repo/issues\"",
        1,
    );
    assert_eq!(
        run_notify_script_scenarios(&RunnableStep {
            script: &other_repo,
            env: base_env,
        }),
        vec![NotifyBehaviorProblem::UnexpectedRepoEndpoint {
            scenario: "no-open-issue",
            endpoint: "repos/other/repo/issues".to_string(),
        }]
    );
}

#[test]
fn notify_failure_workflow_enforces_env_needs_bypass_and_workflow_env() {
    let wfs = ShardWorkflows::load();
    // Workflow-level env / needs / bypass mutations on macos-tests.yml.
    let dropped_token = wfs
        .nightly
        .replacen("          GH_TOKEN: ${{ github.token }}\n", "", 1);
    assert!(wfs
        .with_nightly(dropped_token)
        .problems()
        .contains(&SplitProblem::NotifyBehavior(
            NotifyBehaviorProblem::MissingGhToken {
                scenario: "existing-open-issue",
            }
        )));
    for dropped in [
        "          REPO: ${{ github.repository }}\n",
        "          VERDICT: ${{ needs.macos-tests.result }}\n",
    ] {
        let mutated = wfs.nightly.replacen(dropped, "", 1);
        assert_ne!(mutated, wfs.nightly);
        assert!(wfs
            .with_nightly(mutated)
            .problems()
            .contains(&SplitProblem::NotifyBehavior(
                NotifyBehaviorProblem::ScriptExitedNonZero {
                    scenario: "existing-open-issue",
                    code: Some(1),
                }
            )));
    }
    let dropped_needs = wfs.nightly.replacen(
        "    needs: [macos-machines, macos-tests]\n",
        "    needs: [macos-machines]\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_nightly(dropped_needs).problems(),
        "macos-tests.yml",
        "job notify-failure if",
        RefusalKind::ConditionUnbound,
    ));

    let step_false = wfs.nightly.replacen(
        "      - name: open or update the nightly macOS failure issue\n",
        "      - name: open or update the nightly macOS failure issue\n        if: false\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_nightly(step_false).problems(),
        "macos-tests.yml",
        "job notify-failure step 1 if",
        RefusalKind::ConditionSyntax,
    ));
    let step_coe = wfs.nightly.replacen(
        "      - name: open or update the nightly macOS failure issue\n",
        "      - name: open or update the nightly macOS failure issue\n        continue-on-error: true\n",
        1,
    );
    assert_eq!(
        wfs.with_nightly(step_coe).problems(),
        vec![SplitProblem::NotifyStepAllowsFailure { step_index: 1 }]
    );

    let lifted_env = wfs
        .nightly
        .replacen(
            "permissions:\n",
            "env:\n  LABEL: macos-nightly\n\npermissions:\n",
            1,
        )
        .replacen("          LABEL: macos-nightly\n", "", 1);
    assert!(has_refusal(
        &wfs.with_nightly(lifted_env).problems(),
        "macos-tests.yml",
        "top-level env",
        RefusalKind::WorkflowEnv,
    ));
}

#[test]
fn cache_input_is_parsed_structurally_across_callers() {
    let wfs = ShardWorkflows::load();

    let mut_ci_comment = wfs.ci.replacen(
        "    with:\n      use-cache: true\n",
        "    # use-cache: true\n",
        1,
    );
    assert_eq!(
        wfs.with_ci(mut_ci_comment).problems(),
        vec![SplitProblem::CallerUseCacheMismatch {
            file: "ci.yml".to_string(),
            actual: false,
            expected: true,
        }]
    );

    let mut_ci_spaced = wfs.ci.replacen(
        "      use-cache: true\n",
        "      use-cache:  true # warm\n",
        1,
    );
    assert_eq!(
        wfs.with_ci(mut_ci_spaced).problems(),
        Vec::<SplitProblem>::new()
    );

    let mut_release_comment = wfs.release.replacen(
        "    uses: ./.github/workflows/macos-test-shards.yml\n",
        "    uses: ./.github/workflows/macos-test-shards.yml\n    # use-cache: true\n",
        1,
    );
    assert_eq!(
        wfs.with_release(mut_release_comment).problems(),
        Vec::<SplitProblem>::new()
    );

    for (bad_release_with, expected) in [
        (
            "    with:\n      use-cache:  true\n",
            SplitProblem::CallerUseCacheMismatch {
                file: "release.yml".to_string(),
                actual: true,
                expected: false,
            },
        ),
        (
            "    with:\n      Use-Cache: true\n",
            SplitProblem::CallerUseCacheMismatch {
                file: "release.yml".to_string(),
                actual: true,
                expected: false,
            },
        ),
        (
            "    with:\n      use-cache: true # warm\n",
            SplitProblem::CallerUseCacheMismatch {
                file: "release.yml".to_string(),
                actual: true,
                expected: false,
            },
        ),
        (
            "    with:\n      use-cache: 'true'\n",
            SplitProblem::CallerUseCacheNonBool {
                file: "release.yml".to_string(),
                raw: Scalar::String("true".to_string()),
            },
        ),
        (
            "    with: { use-cache: true }\n",
            SplitProblem::CallerUseCacheMismatch {
                file: "release.yml".to_string(),
                actual: true,
                expected: false,
            },
        ),
    ] {
        let mut_release = wfs.release.replacen(
            "    uses: ./.github/workflows/macos-test-shards.yml\n",
            &format!("    uses: ./.github/workflows/macos-test-shards.yml\n{bad_release_with}"),
            1,
        );
        assert_eq!(wfs.with_release(mut_release).problems(), vec![expected]);
    }

    let mut_ci_quoted = wfs
        .ci
        .replacen("      use-cache: true\n", "      use-cache: 'true'\n", 1);
    assert_eq!(
        wfs.with_ci(mut_ci_quoted).problems(),
        vec![SplitProblem::CallerUseCacheNonBool {
            file: "ci.yml".to_string(),
            raw: Scalar::String("true".to_string()),
        }]
    );
    let mut_default_true =
        wfs.shards
            .replacen("        default: false\n", "        default: true\n", 1);
    assert_eq!(
        wfs.with_shards(mut_default_true).problems(),
        vec![SplitProblem::ShardsUseCacheWrongDefault]
    );
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
        "Total admission:",
        "Final refusals:",
        "not a form this guard reads; allowed forms:",
        "strategy.job-index",
        "no workflow-level `env:`",
        "`ci.yml`'s `pr-caches` and `release.yml`'s `build`",
        "`.github/workflow-digests.txt`",
        "`bash scripts/refresh-workflow-digests.sh`",
        "What the guards do not read:",
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

    let expect_allowed = |expr: &str, c: &ConditionContext| {
        let cond = ClosedCondition::parse_str(expr, "t.yml", "job a if").unwrap();
        assert_eq!(
            cond.evaluate(c).expect_true(|| EvalCheck::Mismatch),
            None,
            "expected allowed for `{expr}`"
        );
    };
    let expect_skipped = |expr: &str, c: &ConditionContext| {
        let cond = ClosedCondition::parse_str(expr, "t.yml", "job a if").unwrap();
        assert_eq!(
            cond.evaluate(c).expect_false(|| EvalCheck::Mismatch),
            None,
            "expected skipped for `{expr}`"
        );
    };

    expect_allowed("inputs.use-cache", &ctx);
    expect_allowed("matrix.nfpm_arch", &ctx);
    expect_allowed(
        "runner.environment == 'SELF-HOSTED' && needs.classify.outputs.lane != 'docs'",
        &ctx,
    );
    expect_allowed(
        "steps.flake-pr.outputs.pull-request-operation == 'created' || steps.flake-pr.outputs.pull-request-operation == 'updated'",
        &ctx,
    );

    let failed_ctx = ctx.clone().with_prior_succeeded(false);
    expect_allowed("always()", &failed_ctx);
    expect_skipped("github.event_name == 'pull_request'", &failed_ctx);
    expect_allowed(
        "always() && github.event_name == 'pull_request'",
        &failed_ctx,
    );

    let absent_ctx = ctx
        .clone()
        .with_step_output("cache", "cache-hit", CondVal::Absent);
    expect_allowed("steps.cache.outputs.cache-hit != 'true'", &absent_ctx);

    // Runtime refusal on boolean compared with string (when evaluated directly).
    let bool_cmp = ClosedCondition::parse_str("inputs.use-cache == 'true'", "t.yml", "job a if")
        .unwrap()
        .evaluate(&ctx)
        .expect_true(|| EvalCheck::Mismatch);
    assert!(matches!(
        bool_cmp,
        Some(EvalCheck::Refused(r)) if r.kind == RefusalKind::ConditionTypeMismatch
    ));
}

#[test]
fn closed_condition_parser_pins_refusal_sentence_and_rejects_bad_syntax_and_literals() {
    let ctx = ConditionContext::default_ok().with_event("pull_request");
    // Pin the single human-readable refusal sentence format once (decision 0071 ruling 8).
    let sample_err = ClosedCondition::parse_str("failure()", "t.yml", "job a if").unwrap_err();
    assert_eq!(sample_err.kind, RefusalKind::ConditionSyntax);
    assert_eq!(
        sample_err.to_string(),
        format!(
            "t.yml (job a if): {} is not a form this guard reads; allowed forms: {}.",
            sample_err.detail, sample_err.allowed
        )
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
        ("steps.cache.outputs.cache-hit == ''", RefusalKind::ConditionLiteral),
        ("github.event_name == '0'", RefusalKind::ConditionLiteral),
        ("github.event_name == '0x10'", RefusalKind::ConditionLiteral),
        ("github.event_name == 'café'", RefusalKind::ConditionLiteral),
    ] {
        let err = ClosedCondition::parse_str(bad, "t.yml", "job a if").unwrap_err();
        assert_eq!(err.kind, expected_kind, "bad condition `{bad}`: {err}");
    }

    let unbound =
        ClosedCondition::parse_str("needs.missing.result == 'success'", "t.yml", "job a if")
            .unwrap()
            .evaluate(&ctx)
            .expect_true(|| EvalCheck::Mismatch);
    assert!(matches!(
        unbound,
        Some(EvalCheck::Refused(r)) if r.kind == RefusalKind::ConditionUnbound
    ));
}

#[test]
fn closed_form_closes_round_1_expression_event_env_and_shard_findings() {
    let wfs = ShardWorkflows::load();

    // Dropping `always() &&` from nightly notify-failure fails because implicit success() skips it.
    let dropped_always = wfs.nightly.replacen(
        "if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "if: github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        1,
    );
    assert!(wfs.with_nightly(dropped_always).problems().contains(
        &SplitProblem::NotifyGateMismatch {
            event: "schedule".to_string(),
            verdict_result: "failure".to_string(),
        }
    ));

    // Parentheses, mixed && / ||, and coercing || are refused with typed place + kind.
    let parenthesized = wfs.nightly.replacen(
        "if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "if: always() && (github.event_name == 'schedule' && needs.macos-tests.result != 'success')",
        1,
    );
    assert!(has_refusal(
        &wfs.with_nightly(parenthesized).problems(),
        "macos-tests.yml",
        "job notify-failure if",
        RefusalKind::ConditionSyntax,
    ));

    let mixed_ops = wfs.ci.replacen(
        "    if: github.event_name != 'push'\n    runs-on: macos-latest\n",
        "    if: github.event_name == 'pull_request' || github.event_name == 'merge_group' && github.event_name != 'push'\n    runs-on: macos-latest\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&mixed_ops),
        "ci.yml",
        "job build-macos if",
        RefusalKind::ConditionSyntax,
    ));

    let coerce_expr = wfs.shards.replacen(
        "        if: matrix.machine == 'macos/1'\n",
        "        if: (matrix.machine || false) == true\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_shards(coerce_expr).problems(),
        "macos-test-shards.yml",
        "job macos-machines step 5 if",
        RefusalKind::ConditionSyntax,
    ));
}

#[test]
fn closed_form_closes_round_1_event_env_and_shard_findings() {
    let wfs = ShardWorkflows::load();
    // Event types mutations.
    let bad_pr_types = wfs.ci.replacen(
        "  pull_request:\n    types: [opened, synchronize, reopened]\n",
        "  pull_request:\n    types: [labeled]\n",
        1,
    );
    assert!(
        ci_macos_split_problems(&bad_pr_types).contains(&SplitProblem::BuildMacosWrongEventGate)
    );

    let bad_mg_types = wfs.ci.replacen(
        "  merge_group:\n    types: [checks_requested]\n",
        "  merge_group:\n    types: [destroyed]\n",
        1,
    );
    assert!(
        ci_macos_split_problems(&bad_mg_types).contains(&SplitProblem::BuildMacosWrongEventGate)
    );

    let bad_dispatch_types = wfs.ci.replacen(
        "  workflow_dispatch:\n",
        "  workflow_dispatch:\n    types: [manual]\n",
        1,
    );
    assert!(wfs.with_ci(bad_dispatch_types).problems().contains(
        &SplitProblem::CallerMachinesEventMismatch {
            file: "ci.yml".to_string(),
            event: "workflow_dispatch".to_string(),
        }
    ));

    let bad_release_types = wfs.release.replacen(
        "  push:\n    tags: [\"v*\"]\n",
        "  push:\n    tags: [\"v*\"]\n    types: [created]\n",
        1,
    );
    assert!(wfs.with_release(bad_release_types).problems().contains(
        &SplitProblem::CallerMachinesEventMismatch {
            file: "release.yml".to_string(),
            event: "push".to_string(),
        }
    ));

    // Workflow-level env refused.
    let ci_top_env = wfs.ci.replacen(
        "permissions:\n",
        "env:\n  MACOS_HOST: '1'\n\npermissions:\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&ci_top_env),
        "ci.yml",
        "top-level env",
        RefusalKind::WorkflowEnv,
    ));
}

#[test]
fn closed_form_closes_round_1_shard_runner_matrix_cache_and_script_findings() {
    let wfs = ShardWorkflows::load();
    // Shard matrix exclude, runner change, second cache step, and script mutations.
    let with_exclude = wfs.shards.replacen(
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n",
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n        exclude:\n          - machine: macos/2\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_shards(with_exclude).problems(),
        "macos-test-shards.yml",
        "job macos-machines strategy.matrix",
        RefusalKind::MatrixClause,
    ));

    let bad_runner = wfs.shards.replacen(
        "    runs-on: macos-latest\n",
        "    runs-on: ubuntu-24.04\n",
        1,
    );
    assert_eq!(
        wfs.with_shards(bad_runner).problems(),
        vec![SplitProblem::ShardsWrongRunner]
    );

    let second_cache = wfs.shards.replacen(
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n        if: inputs.use-cache\n        with:\n          shared-key: build-macos\n",
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n        if: inputs.use-cache\n        with:\n          shared-key: build-macos\n      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n        if: matrix.machine == 'macos/1'\n",
        1,
    );
    assert_eq!(
        wfs.with_shards(second_cache).problems(),
        vec![SplitProblem::ShardsCacheStepNotGated { step_index: 4 }]
    );

    for (needle, replacement, expected) in [
        (
            "          set -euo pipefail\n",
            "          # added comment only\n          set -euo pipefail\n",
            SplitProblem::ShardsMissingGateA,
        ),
        (
            "          grep -q 'Gate A startup verdict' target/r3-startup-report.txt\n",
            "          true\n",
            SplitProblem::ShardsMissingGateA,
        ),
        (
            "          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
            "          # bundle comment\n          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
            SplitProblem::ShardsMissingBundles,
        ),
        (
            "          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
            "          # cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n",
            SplitProblem::ShardsMissingBundles,
        ),
    ] {
        let mutated = wfs.shards.replacen(needle, replacement, 1);
        assert_eq!(wfs.with_shards(mutated).problems(), vec![expected]);
    }
}

#[test]
fn closed_form_closes_g1_comparison_and_g2_final_refusal_findings() {
    let wfs = ShardWorkflows::load();

    // G1: boolean input compared against string in R3 diagnostics upload is refused at admission.
    let g1_bool_input = wfs.shards.replacen(
        "        if: always() && matrix.machine == 'macos/1'\n",
        "        if: always() && inputs.use-cache == 'true'\n",
        1,
    );
    assert_ne!(g1_bool_input, wfs.shards);
    assert!(has_refusal(
        &wfs.with_shards(g1_bool_input).problems(),
        "macos-test-shards.yml",
        "job macos-machines step 6 if",
        RefusalKind::ConditionTypeMismatch,
    ));

    // G1: empty comparison literal on steps.cache.outputs.cache-hit in ci.yml is refused at admission.
    let g1_empty_lit = wfs.ci.replacen(
        "steps.cache.outputs.cache-hit != 'true'",
        "steps.cache.outputs.cache-hit == ''",
        1,
    );
    assert_ne!(g1_empty_lit, wfs.ci);
    assert!(has_refusal(
        &ci_macos_split_problems(&g1_empty_lit),
        "ci.yml",
        "job pr-caches step 4 if",
        RefusalKind::ConditionLiteral,
    ));

    // G1: steps.* path placed on a job if: is refused at admission.
    let g1_steps_on_job = wfs.ci.replacen(
        "  pr-caches:\n    name: pull-request cache (${{ matrix.key }}, ${{ matrix.os }})\n    if: github.event_name == 'push'\n",
        "  pr-caches:\n    name: pull-request cache (${{ matrix.key }}, ${{ matrix.os }})\n    if: steps.cache.outputs.cache-hit != 'true'\n",
        1,
    );
    assert_ne!(g1_steps_on_job, wfs.ci);
    assert!(has_refusal(
        &ci_macos_split_problems(&g1_steps_on_job),
        "ci.yml",
        "job pr-caches if",
        RefusalKind::ConditionUnbound,
    ));

    // G1: undeclared needs output in ci.yml is refused at admission.
    let g1_undeclared_out = wfs.ci.replacen(
        "needs.classify.outputs.lane == 'docs'",
        "needs.classify.outputs.missing == 'docs'",
        1,
    );
    assert_ne!(g1_undeclared_out, wfs.ci);
    assert!(has_refusal(
        &ci_macos_split_problems(&g1_undeclared_out),
        "ci.yml",
        "job docs-lane if",
        RefusalKind::ConditionUnbound,
    ));

    // G2: duplicate bundle step with unbound runner.environment before the valid step fails with refusal.
    let g2_dup_bundle = wfs.shards.replacen(
        "      - name: self and verify bundles compile under the constitutional lint\n",
        "      - name: shadowed bundles\n        if: runner.environment == 'self-hosted'\n        run: |\n          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self\n          cargo run --locked -p brokkr-cli -- library compile --bundle recipes/verify\n      - name: self and verify bundles compile under the constitutional lint\n",
        1,
    );
    assert_ne!(g2_dup_bundle, wfs.shards);
    assert!(has_refusal(
        &wfs.with_shards(g2_dup_bundle).problems(),
        "macos-test-shards.yml",
        "job macos-machines step 7 if",
        RefusalKind::ConditionUnbound,
    ));

    // G2: lone cache step with unbound runner.environment preserves its refusal (not CacheStepNotGated).
    let g2_cache_unbound = wfs.shards.replacen(
        "        if: inputs.use-cache\n",
        "        if: runner.environment == 'self-hosted'\n",
        1,
    );
    assert_ne!(g2_cache_unbound, wfs.shards);
    let g2_cache_probs = wfs.with_shards(g2_cache_unbound).problems();
    assert!(has_refusal(
        &g2_cache_probs,
        "macos-test-shards.yml",
        "job macos-machines step 3 if",
        RefusalKind::ConditionUnbound,
    ));
    assert!(!g2_cache_probs
        .iter()
        .any(|p| matches!(p, SplitProblem::ShardsCacheStepNotGated { .. })));

    // G2: unbound runner.environment on build-macos compile step preserves refusal.
    let g2_build_unbound = wfs.ci.replacen(
        "      - name: workspace and test targets compile on macOS\n",
        "      - name: workspace and test targets compile on macOS\n        if: runner.environment == 'self-hosted'\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&g2_build_unbound),
        "ci.yml",
        "job build-macos step 4 if",
        RefusalKind::ConditionUnbound,
    ));
}

#[test]
fn closed_form_closes_g3_matrix_g4_bare_axis_and_top_level_key_findings() {
    let wfs = ShardWorkflows::load();
    // G3: unread matrix exclude on ci.yml build-macos and macos-tests.yml notify-failure are refused.
    let g3_ci_exclude = wfs.ci.replacen(
        "  build-macos:\n    name: build (macos-latest)\n",
        "  build-macos:\n    name: build (macos-latest)\n    strategy:\n      matrix:\n        x: [1]\n        exclude:\n          - x: 1\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&g3_ci_exclude),
        "ci.yml",
        "job build-macos strategy.matrix",
        RefusalKind::MatrixClause,
    ));

    let g3_ci_include = wfs.ci.replacen(
        "  build-macos:\n    name: build (macos-latest)\n",
        "  build-macos:\n    name: build (macos-latest)\n    strategy:\n      matrix:\n        include:\n          - x: 1\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&g3_ci_include),
        "ci.yml",
        "job build-macos strategy.matrix",
        RefusalKind::MatrixClause,
    ));

    let g3_notify_exclude = wfs.nightly.replacen(
        "  notify-failure:\n    name: report nightly macOS test failure\n",
        "  notify-failure:\n    name: report nightly macOS test failure\n    strategy:\n      matrix:\n        x: [1]\n        exclude:\n          - x: 1\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_nightly(g3_notify_exclude).problems(),
        "macos-tests.yml",
        "job notify-failure strategy.matrix",
        RefusalKind::MatrixClause,
    ));

    // G4: misspelled bare matrix axis in release.yml fails ShardWorkflows via total admission.
    let g4_misspelled = wfs.release.replacen(
        "        if: matrix.nfpm_arch\n",
        "        if: matrix.nfpm_arh\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_release(g4_misspelled).problems(),
        "release.yml",
        "job build step 5 if",
        RefusalKind::ConditionBarePath,
    ));

    // Top-level unknown key (total admission).
    let top_defaults = wfs.ci.replacen(
        "permissions:\n",
        "defaults:\n  run:\n    shell: bash\n\npermissions:\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&top_defaults),
        "ci.yml",
        "top-level defaults",
        RefusalKind::TopLevelKey,
    ));
}

#[test]
fn closed_form_closes_g5_g6_g7_g8_trigger_gate_and_setup_findings() {
    let wfs = ShardWorkflows::load();

    // G5: adding schedule to ci.yml and allowing macos-machines on it fails dispatch-only guard.
    let g5_sched = wfs
        .ci
        .replacen(
            "  workflow_dispatch:\n",
            "  workflow_dispatch:\n  schedule:\n    - cron: '0 0 * * *'\n",
            1,
        )
        .replacen(
            "  macos-machines:\n    name: test (macos-latest) shards\n    if: github.event_name == 'workflow_dispatch'\n",
            "  macos-machines:\n    name: test (macos-latest) shards\n    if: github.event_name == 'workflow_dispatch' || github.event_name == 'schedule'\n",
            1,
        );
    assert_ne!(g5_sched, wfs.ci);
    assert!(
        ci_macos_split_problems(&g5_sched).contains(&SplitProblem::CiDispatchJobRunsOnEvent {
            job_id: "macos-machines".to_string(),
            event: "schedule".to_string(),
        })
    );

    // G6: sibling field on schedule entry is refused, never dropped.
    let g6_sched_extra = wfs.nightly.replacen(
        "  schedule:\n    - cron: '17 4 * * *'\n",
        "  schedule:\n    - cron: '17 4 * * *'\n      timezone: UTC\n",
        1,
    );
    assert_ne!(g6_sched_extra, wfs.nightly);
    assert!(has_refusal(
        &wfs.with_nightly(g6_sched_extra).problems(),
        "macos-tests.yml",
        "on.schedule",
        RefusalKind::EventFilter,
    ));

    // G7: extra shard-failure requirement on notify-failure fails when shards succeed & verdict fails.
    let g7_extra_dep_req = wfs.nightly.replacen(
        "if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success' && needs.macos-machines.result != 'success'",
        1,
    );
    assert_ne!(g7_extra_dep_req, wfs.nightly);
    assert!(wfs.with_nightly(g7_extra_dep_req).problems().contains(
        &SplitProblem::NotifyGateMismatch {
            event: "schedule".to_string(),
            verdict_result: "failure".to_string(),
        }
    ));

    // G8: gating checkout or toolchain setup on a single shard (or continue-on-error) fails.
    for (needle, expected) in [
        (
            "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n",
            SplitProblem::ShardsMissingCheckout,
        ),
        (
            "      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master\n",
            SplitProblem::ShardsMissingToolchain,
        ),
    ] {
        for bypass in [
            "        if: matrix.machine == 'macos/1'\n",
            "        continue-on-error: true\n",
        ] {
            let mutated = wfs.shards.replacen(needle, &format!("{needle}{bypass}"), 1);
            assert_ne!(mutated, wfs.shards);
            assert_eq!(wfs.with_shards(mutated).problems(), vec![expected.clone()]);
        }
    }
}

#[test]
fn closed_form_closes_g9_whole_script_and_g11_non_scalar_if_findings() {
    let wfs = ShardWorkflows::load();
    // G9: mutating notify-failure script text fails BEFORE executing bash (even if gh calls match).
    let marker_dir = tempfile::tempdir().expect("tempdir for G9 marker");
    let marker_path = marker_dir.path().join("must-not-exist");
    let g9_mutated = wfs.nightly.replacen(
        "          set -euo pipefail\n",
        &format!(
            "          set -euo pipefail\n          touch '{}'\n",
            marker_path.display()
        ),
        1,
    );
    assert_ne!(g9_mutated, wfs.nightly);
    assert_eq!(
        wfs.with_nightly(g9_mutated).problems(),
        vec![SplitProblem::NotifyScriptChanged]
    );
    assert!(
        !marker_path.exists(),
        "unverified workflow script must never execute before whole-script identity check"
    );

    // G11: YAML sequence or mapping in `if:` produces ClosedFormRefusal naming the job or step.
    let g11_seq = wfs.ci.replacen(
        "    if: github.event_name != 'push'\n    runs-on: macos-latest\n",
        "    if: [true]\n    runs-on: macos-latest\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&g11_seq),
        "ci.yml",
        "job build-macos if",
        RefusalKind::ConditionSyntax,
    ));

    let g11_map = wfs.shards.replacen(
        "        if: matrix.machine == 'macos/1'\n",
        "        if: { machine: macos/1 }\n",
        1,
    );
    assert!(has_refusal(
        &wfs.with_shards(g11_map).problems(),
        "macos-test-shards.yml",
        "job macos-machines step 5 if",
        RefusalKind::ConditionSyntax,
    ));
}

#[test]
fn closed_form_closes_g13_pattern_g15_whole_script_and_g16_ascii_literal_findings() {
    let wfs = ShardWorkflows::load();
    // G13: download pattern admits plain literal as well as `<literal>*` and carries the typed value (G12).
    let ci_admitted = ClosedWorkflow::parse("ci.yml", &wfs.ci).expect("ci.yml admitted");
    let cov_dl_pat = ci_admitted
        .require_job("ci.yml", "coverage")
        .unwrap()
        .steps
        .iter()
        .find_map(|s| s.artifact_pattern.clone());
    assert_eq!(
        cov_dl_pat,
        Some(ArtifactPattern::PrefixWildcard(
            "coverage-machine-".to_string()
        ))
    );
    let g13_plain_pat = wfs.ci.replacen(
        "pattern: coverage-machine-*",
        "pattern: coverage-machine-0",
        1,
    );
    assert_ne!(g13_plain_pat, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&g13_plain_pat),
        Vec::<SplitProblem>::new()
    );
    let plain_admitted =
        ClosedWorkflow::parse("ci.yml", &g13_plain_pat).expect("plain pattern admitted");
    let plain_pat = plain_admitted
        .require_job("ci.yml", "coverage")
        .unwrap()
        .steps
        .iter()
        .find_map(|s| s.artifact_pattern.clone());
    assert_eq!(
        plain_pat,
        Some(ArtifactPattern::Literal("coverage-machine-0".to_string()))
    );
    let g13_bad_pat = wfs
        .ci
        .replacen("pattern: coverage-machine-*", "pattern: '*coverage*'", 1);
    assert!(has_refusal(
        &ci_macos_split_problems(&g13_bad_pat),
        "ci.yml",
        "job coverage step 8 with.pattern",
        RefusalKind::ArtifactName,
    ));

    // G15: indirect invocation and even a shell comment inside build-macos's
    // `run:` script both fail the whole-script check.
    let g15_indirect = wfs.ci.replacen(
        "        run: cargo test --workspace --all-features --locked --no-run\n",
        "        run: cargo test --workspace --all-features --locked --no-run\n      - name: indirect\n        run: s=test-shards; bash \"scripts/${s}.sh\" test --parallel-heaps\n",
        1,
    );
    assert_eq!(
        ci_macos_split_problems(&g15_indirect),
        vec![SplitProblem::BuildMacosUnexpectedRunStep { step_index: 5 }]
    );
    let g15_shell_comment = wfs.ci.replacen(
        "        run: cargo test --workspace --all-features --locked --no-run\n",
        "        run: |\n          # test-shards.sh lives in macos-test-shards.yml, not here\n          cargo test --workspace --all-features --locked --no-run\n",
        1,
    );
    assert_ne!(g15_shell_comment, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&g15_shell_comment),
        vec![
            SplitProblem::BuildMacosMissingCompile,
            SplitProblem::BuildMacosUnexpectedRunStep { step_index: 4 },
        ]
    );

    // G16: non-ASCII comparison literal is refused at admission.
    let g16_non_ascii = wfs.ci.replacen(
        "    if: github.event_name != 'push'\n    runs-on: macos-latest\n",
        "    if: github.event_name != 'café'\n    runs-on: macos-latest\n",
        1,
    );
    assert!(has_refusal(
        &ci_macos_split_problems(&g16_non_ascii),
        "ci.yml",
        "job build-macos if",
        RefusalKind::ConditionLiteral,
    ));
}

#[test]
fn closed_form_refuses_empty_matrix_clauses_malformed_schedule_and_workflow_call_filters() {
    let wfs = ShardWorkflows::load();

    // L1a: empty `include: []` and empty `exclude: []` are refused like non-empty ones.
    let empty_exclude = wfs.shards.replacen(
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n",
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n        exclude: []\n",
        1,
    );
    assert_ne!(empty_exclude, wfs.shards);
    assert!(has_refusal(
        &wfs.with_shards(empty_exclude).problems(),
        "macos-test-shards.yml",
        "job macos-machines strategy.matrix",
        RefusalKind::MatrixClause,
    ));

    let empty_include = wfs.shards.replacen(
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n",
        "        machine: [macos/1, macos/2, macos/3, macos/4, macos/5]\n        include: []\n",
        1,
    );
    assert_ne!(empty_include, wfs.shards);
    assert!(has_refusal(
        &wfs.with_shards(empty_include).problems(),
        "macos-test-shards.yml",
        "job macos-machines strategy.matrix",
        RefusalKind::MatrixClause,
    ));

    // L1b: `on.schedule` empty list, null, or mapping instead of list yields ClosedFormRefusal on `on.schedule`.
    for bad_sched in ["  schedule: []\n", "  schedule:\n"] {
        let empty_schedule =
            wfs.nightly
                .replacen("  schedule:\n    - cron: '17 4 * * *'\n", bad_sched, 1);
        assert_ne!(empty_schedule, wfs.nightly);
        assert!(has_refusal(
            &wfs.with_nightly(empty_schedule).problems(),
            "macos-tests.yml",
            "on.schedule",
            RefusalKind::EventFilter,
        ));
    }

    let map_schedule = wfs.nightly.replacen(
        "  schedule:\n    - cron: '17 4 * * *'\n",
        "  schedule:\n    cron: '17 4 * * *'\n",
        1,
    );
    assert_ne!(map_schedule, wfs.nightly);
    assert!(has_refusal(
        &wfs.with_nightly(map_schedule).problems(),
        "macos-tests.yml",
        "on.schedule",
        RefusalKind::EventFilter,
    ));

    // L1c: `workflow_call` carrying `types` or any key other than `inputs` is refused.
    for (extra_line, label) in [
        ("    types: [opened]\n", "types"),
        ("    branches: [main]\n", "branches"),
        (
            "    secrets:\n      TOKEN:\n        required: false\n",
            "secrets",
        ),
    ] {
        let mutated = wfs.shards.replacen(
            "  workflow_call:\n",
            &format!("  workflow_call:\n{extra_line}"),
            1,
        );
        assert_ne!(mutated, wfs.shards, "{label}");
        assert!(
            has_refusal(
                &wfs.with_shards(mutated).problems(),
                "macos-test-shards.yml",
                "on.workflow_call",
                RefusalKind::EventFilter,
            ),
            "workflow_call with `{label}` must be refused"
        );
    }
}

#[test]
fn macos_split_variants_each_bind_to_a_planted_mutation() {
    let wfs = ShardWorkflows::load();

    // BuildMacosMissingCheckout
    let no_checkout = wfs.ci.replacen(
        "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n        with:\n          persist-credentials: false\n      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master\n        with:\n          toolchain: 1.98.0\n      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n      - name: workspace and test targets compile on macOS\n",
        "      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master\n        with:\n          toolchain: 1.98.0\n      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n      - name: workspace and test targets compile on macOS\n",
        1,
    );
    assert_ne!(no_checkout, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&no_checkout),
        vec![SplitProblem::BuildMacosMissingCheckout]
    );

    // BuildMacosMissingToolchain
    let no_toolchain = wfs.ci.replacen(
        "      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master\n        with:\n          toolchain: 1.98.0\n      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n      - name: workspace and test targets compile on macOS\n",
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2\n      - name: workspace and test targets compile on macOS\n",
        1,
    );
    assert_ne!(no_toolchain, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&no_toolchain),
        vec![SplitProblem::BuildMacosMissingToolchain]
    );

    // ShardsSetupOrderInvalid (toolchain before checkout)
    let swapped_setup = wfs.shards.replacen(
        "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n        with:\n          persist-credentials: false\n      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master\n        with:\n          toolchain: 1.98.0\n",
        "      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master\n        with:\n          toolchain: 1.98.0\n      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n        with:\n          persist-credentials: false\n",
        1,
    );
    assert_ne!(swapped_setup, wfs.shards);
    assert_eq!(
        wfs.with_shards(swapped_setup).problems(),
        vec![SplitProblem::ShardsSetupOrderInvalid]
    );

    // NotifyExtraScriptStep
    let extra_step = format!(
        "{}\n      - name: extra notify step\n        run: echo extra\n",
        wfs.nightly.trim_end_matches('\n')
    );
    assert_eq!(
        wfs.with_nightly(extra_step).problems(),
        vec![SplitProblem::NotifyExtraScriptStep { step_index: 2 }]
    );

    // NotifyEnvError: typed UndeclaredNeedsResult vs UnsupportedExpression
    let undeclared_needs = wfs.nightly.replacen(
        "          VERDICT: ${{ needs.macos-tests.result }}\n",
        "          VERDICT: ${{ needs.unlisted-job.result }}\n",
        1,
    );
    assert_ne!(undeclared_needs, wfs.nightly);
    assert_eq!(
        wfs.with_nightly(undeclared_needs).problems(),
        vec![SplitProblem::NotifyEnvError {
            key: "VERDICT".to_string(),
            reason: NotifyEnvReason::UndeclaredNeedsResult {
                expr: "needs.unlisted-job.result".to_string(),
                job_id: "unlisted-job".to_string(),
            },
        }]
    );

    let unsupported_expr = wfs.nightly.replacen(
        "          HEAD_SHA: ${{ github.sha }}\n",
        "          HEAD_SHA: ${{ github.actor }}\n",
        1,
    );
    assert_ne!(unsupported_expr, wfs.nightly);
    assert_eq!(
        wfs.with_nightly(unsupported_expr).problems(),
        vec![SplitProblem::NotifyEnvError {
            key: "HEAD_SHA".to_string(),
            reason: NotifyEnvReason::UnsupportedExpression {
                expr: "github.actor".to_string(),
            },
        }]
    );
}

const OUT_OF_FORM_TRIGGER_CASES: &[(&str, &str, &str)] = &[
    ("scalar on", "on: 42\n", "on"),
    ("null on", "on: null\n", "on"),
    ("non-string on list item", "on: [push, 42]\n", "on[1]"),
    ("non-string on map key", "on:\n  42: null\n", "on"),
    ("null schedule", "on:\n  schedule: null\n", "on.schedule"),
    (
        "non-list schedule",
        "on:\n  schedule: '17 4 * * *'\n",
        "on.schedule",
    ),
    (
        "empty schedule list",
        "on:\n  schedule: []\n",
        "on.schedule",
    ),
    (
        "scalar schedule row",
        "on:\n  schedule: [nope]\n",
        "on.schedule[0]",
    ),
    (
        "empty schedule row",
        "on:\n  schedule: [{}]\n",
        "on.schedule[0]",
    ),
    (
        "non-string schedule cron",
        "on:\n  schedule:\n    - cron: 42\n",
        "on.schedule[0]",
    ),
    (
        "null schedule cron",
        "on:\n  schedule:\n    - cron: null\n",
        "on.schedule[0]",
    ),
    (
        "workflow_call null types",
        "on:\n  workflow_call:\n    types: null\n",
        "on.workflow_call.types",
    ),
    (
        "scalar string branches filter",
        "on:\n  push:\n    branches: main\n",
        "on.push.branches",
    ),
    (
        "scalar string tags filter",
        "on:\n  push:\n    tags: v1\n",
        "on.push.tags",
    ),
    (
        "scalar string types filter",
        "on:\n  pull_request:\n    types: opened\n",
        "on.pull_request.types",
    ),
    (
        "non-list filter value",
        "on:\n  push:\n    branches: 42\n",
        "on.push.branches",
    ),
    (
        "null filter value",
        "on:\n  push:\n    tags: null\n",
        "on.push.tags",
    ),
    (
        "non-string filter list item",
        "on:\n  pull_request:\n    types: [opened, 42]\n",
        "on.pull_request.types",
    ),
    (
        "null workflow_call inputs",
        "on:\n  workflow_call:\n    inputs: null\n",
        "on.workflow_call.inputs",
    ),
    (
        "null workflow_call input definition",
        "on:\n  workflow_call:\n    inputs:\n      use-cache: null\n",
        "on.workflow_call.inputs.use-cache",
    ),
    (
        "non-mapping event config",
        "on:\n  push: [main]\n",
        "on.push",
    ),
    (
        "key with null value on event",
        "on:\n  push:\n    paths: null\n",
        "on.push",
    ),
];

#[test]
fn out_of_form_trigger_shapes_yield_typed_closed_form_refusal_never_parse_error() {
    for &(label, on_block, expected_place) in OUT_OF_FORM_TRIGGER_CASES {
        let yaml = format!(
            "name: sample\n{on_block}jobs:\n  ok:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: echo ok\n"
        );
        match ClosedWorkflow::parse("sample.yml", &yaml) {
            Err(crate::workflow::ClosedWorkflowError::Refusal(r)) => {
                assert_eq!(
                    (r.kind, r.place.as_str()),
                    (RefusalKind::EventFilter, expected_place),
                    "case `{label}` gave unexpected refusal: {r}"
                );
            }
            other => panic!(
                "case `{label}` must return ClosedWorkflowError::Refusal at `{expected_place}`, got {other:?}"
            ),
        }
    }
}
