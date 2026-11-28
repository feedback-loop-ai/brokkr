//! Table tests for closed-form `strategy.matrix` admission across every
//! position and YAML kind, and structured-read tests for caller shard-list and
//! macOS runner checks (#1417, decision 0071 rulings 3, 5, 8 and 9).

use super::{ci_macos_split_problems, ShardWorkflows, SplitProblem};
use crate::workflow::{ClosedWorkflow, ClosedWorkflowError, RefusalKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum YamlKind {
    Null,
    Bool,
    Number,
    String,
    List,
    Mapping,
}

const ALL_KINDS: [YamlKind; 6] = [
    YamlKind::Null,
    YamlKind::Bool,
    YamlKind::Number,
    YamlKind::String,
    YamlKind::List,
    YamlKind::Mapping,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatrixPosition {
    MatrixValue,
    AxisKey,
    AxisValue,
    AxisItem,
    IncludeValueAdmittedJob,
    IncludeValueDisallowedJob,
    ExcludeValue,
    IncludeRow,
    IncludeRowKey,
    IncludeRowValue,
}

const ALL_POSITIONS: [MatrixPosition; 10] = [
    MatrixPosition::MatrixValue,
    MatrixPosition::AxisKey,
    MatrixPosition::AxisValue,
    MatrixPosition::AxisItem,
    MatrixPosition::IncludeValueAdmittedJob,
    MatrixPosition::IncludeValueDisallowedJob,
    MatrixPosition::ExcludeValue,
    MatrixPosition::IncludeRow,
    MatrixPosition::IncludeRowKey,
    MatrixPosition::IncludeRowValue,
];

fn key_yaml(kind: YamlKind) -> &'static str {
    match kind {
        YamlKind::Null => "null",
        YamlKind::Bool => "false",
        YamlKind::Number => "1",
        YamlKind::String => "os",
        YamlKind::List => "[a]",
        YamlKind::Mapping => "{ a: 1 }",
    }
}

fn value_yaml(kind: YamlKind, list_val: &'static str, map_val: &'static str) -> &'static str {
    match kind {
        YamlKind::Null => "null",
        YamlKind::Bool => "false",
        YamlKind::Number => "1",
        YamlKind::String => "'linux'",
        YamlKind::List => list_val,
        YamlKind::Mapping => map_val,
    }
}

fn matrix_cell_case(
    pos: MatrixPosition,
    kind: YamlKind,
) -> (&'static str, &'static str, String, bool) {
    let (file, job, matrix_expr, admitted) = match pos {
        MatrixPosition::MatrixValue => (
            "sample.yml",
            "other",
            value_yaml(kind, "[linux]", "{ os: [linux] }").to_string(),
            kind == YamlKind::Mapping,
        ),
        MatrixPosition::AxisKey => (
            "sample.yml",
            "other",
            format!("{{ {}: [linux] }}", key_yaml(kind)),
            kind == YamlKind::String,
        ),
        MatrixPosition::AxisValue => (
            "sample.yml",
            "other",
            format!("{{ os: {} }}", value_yaml(kind, "[linux]", "{ a: 1 }")),
            kind == YamlKind::List,
        ),
        MatrixPosition::AxisItem => (
            "sample.yml",
            "other",
            format!("{{ os: [{}] }}", value_yaml(kind, "[linux]", "{ a: 1 }")),
            true,
        ),
        MatrixPosition::IncludeValueAdmittedJob => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: {} }}",
                value_yaml(kind, "[{ os: ubuntu-24.04 }]", "{ os: ubuntu-24.04 }")
            ),
            kind == YamlKind::List,
        ),
        MatrixPosition::IncludeValueDisallowedJob => (
            "sample.yml",
            "other",
            format!(
                "{{ include: {} }}",
                value_yaml(kind, "[{ os: linux }]", "{ os: linux }")
            ),
            false,
        ),
        MatrixPosition::ExcludeValue => (
            "sample.yml",
            "other",
            format!(
                "{{ exclude: {} }}",
                value_yaml(kind, "[{ os: linux }]", "{ os: linux }")
            ),
            false,
        ),
        MatrixPosition::IncludeRow => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: [{}] }}",
                value_yaml(kind, "[ubuntu-24.04]", "{ os: ubuntu-24.04 }")
            ),
            kind == YamlKind::Mapping,
        ),
        MatrixPosition::IncludeRowKey => (
            "ci.yml",
            "pr-caches",
            format!("{{ include: [{{ {}: ubuntu-24.04 }}] }}", key_yaml(kind)),
            kind == YamlKind::String,
        ),
        MatrixPosition::IncludeRowValue => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: [{{ os: {} }}] }}",
                value_yaml(kind, "[ubuntu-24.04]", "{ a: 1 }")
            ),
            true,
        ),
    };
    let yaml = format!(
        "on: workflow_dispatch\njobs:\n  {job}:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {matrix_expr}\n    steps:\n      - run: echo ok\n"
    );
    (file, job, yaml, admitted)
}

#[test]
fn matrix_position_by_yaml_kind_table_admits_or_refuses_without_parse_error() {
    for pos in ALL_POSITIONS {
        for kind in ALL_KINDS {
            let (file, job, yaml, expect_admitted) = matrix_cell_case(pos, kind);
            let res = ClosedWorkflow::parse(file, &yaml);
            if expect_admitted {
                assert!(
                    res.is_ok(),
                    "cell ({pos:?}, {kind:?}) expected admitted, got {res:?}\n{yaml}"
                );
            } else {
                let expected_place = format!("job {job} strategy.matrix");
                match res {
                    Err(ClosedWorkflowError::Refusal(r)) => {
                        assert_eq!(
                            (r.kind, r.file.as_str(), r.place.as_str()),
                            (RefusalKind::MatrixClause, file, expected_place.as_str()),
                            "cell ({pos:?}, {kind:?}) gave unexpected refusal: {r}"
                        );
                    }
                    other => panic!(
                        "cell ({pos:?}, {kind:?}) must fail with MatrixClause refusal naming `{expected_place}`, got {other:?}\n{yaml}"
                    ),
                }
            }
        }
    }

    // Also verify every row-key kind under `exclude` refuses with MatrixClause, never Parse.
    for kind in ALL_KINDS {
        let yaml = format!(
            "on: workflow_dispatch\njobs:\n  other:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {{ exclude: [{{ {}: linux }}] }}\n    steps:\n      - run: echo ok\n",
            key_yaml(kind)
        );
        match ClosedWorkflow::parse("sample.yml", &yaml) {
            Err(ClosedWorkflowError::Refusal(r)) => {
                assert_eq!(
                    (r.kind, r.place.as_str()),
                    (RefusalKind::MatrixClause, "job other strategy.matrix")
                );
            }
            other => panic!("exclude row key {kind:?} must fail with MatrixClause, got {other:?}"),
        }
    }
}

#[test]
fn caller_shard_list_check_uses_structured_fields_not_scalars() {
    let wfs = ShardWorkflows::load();

    // Shell comment mentioning `macos/1` in a release step gives no problem.
    let release_step_comment = wfs.release.replacen(
        "      - name: tag matches the workspace release version\n        shell: bash\n        run: |\n",
        "      - name: tag matches the workspace release version\n        shell: bash\n        run: |\n          # release documentation mentions macos/1\n",
        1,
    );
    assert_ne!(release_step_comment, wfs.release);
    assert_eq!(
        wfs.with_release(release_step_comment).problems(),
        Vec::<SplitProblem>::new()
    );

    // Step name, env, and shell comment mentioning `macos` or `macos/1` on Linux jobs give no problem.
    let ci_mentions_only = wfs
        .ci
        .replacen(
            "  engine:\n    name: test (ubuntu-latest)\n",
            "  engine:\n    name: test (ubuntu-latest)\n    env:\n      MACOS_NOTE: macos/1\n",
            1,
        )
        .replacen(
            "      - name: the docs lane passed on a docs change, and every machine on any other\n",
            "      - name: docs lane and linux verdict (not macos/1)\n",
            1,
        );
    assert_ne!(ci_mentions_only, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&ci_mentions_only),
        Vec::<SplitProblem>::new()
    );
    assert_eq!(
        wfs.with_ci(ci_mentions_only).problems(),
        Vec::<SplitProblem>::new()
    );

    // Caller whose matrix axis holds a shard value is reported as CallerCopiedShardList.
    let copied_matrix = format!(
        "{}\n  copied-shards:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix:\n        machine: [macos/1, macos/2]\n    steps:\n      - run: echo ok\n",
        wfs.release.trim_end_matches('\n')
    );
    assert_eq!(
        wfs.with_release(copied_matrix).problems(),
        vec![SplitProblem::CallerCopiedShardList {
            file: "release.yml".to_string(),
        }]
    );

    // Caller whose admitted `include` row holds a shard value is reported as CallerCopiedShardList.
    let copied_include = wfs
        .release
        .replacen("cross: true }", "cross: true, shard: macos/2 }", 1);
    assert_ne!(copied_include, wfs.release);
    assert_eq!(
        wfs.with_release(copied_include).problems(),
        vec![SplitProblem::CallerCopiedShardList {
            file: "release.yml".to_string(),
        }]
    );

    // Caller passing a shard value in a reusable-workflow call's `with:` input is reported.
    let copied_with = wfs.nightly.replacen(
        "    with:\n      use-cache: true\n",
        "    with:\n      use-cache: true\n      machine: macos/1\n",
        1,
    );
    assert_ne!(copied_with, wfs.nightly);
    assert_eq!(
        wfs.with_nightly(copied_with).problems(),
        vec![SplitProblem::CallerCopiedShardList {
            file: "macos-tests.yml".to_string(),
        }]
    );
}

#[test]
fn macos_runner_check_uses_runs_on_and_expanded_matrix_not_scalars() {
    let wfs = ShardWorkflows::load();

    // Job whose `runs-on` is a macOS label list, fromJSON macOS array, or matrix expansion is seen as macOS.
    let engine_label_list = wfs.ci.replacen(
        "  engine:\n    name: test (ubuntu-latest)\n    needs: [classify, docs-lane, engine-machines]\n    if: always() && github.event_name != 'push'\n    runs-on: ubuntu-24.04\n",
        "  engine:\n    name: test (ubuntu-latest)\n    needs: [classify, docs-lane, engine-machines]\n    if: always() && github.event_name != 'push'\n    runs-on: [self-hosted, macOS]\n",
        1,
    );
    assert_ne!(engine_label_list, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&engine_label_list),
        vec![SplitProblem::CiJobMentionsMacos {
            job_id: "engine".to_string(),
        }]
    );

    let from_json_macos = wfs.ci.replacen(
        "'[\"self-hosted\", \"Linux\", \"X64\", \"brokkr-engine\"]'",
        "'[\"self-hosted\", \"macOS\", \"ARM64\", \"brokkr-engine\"]'",
        1,
    );
    assert_ne!(from_json_macos, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&from_json_macos),
        vec![SplitProblem::CiJobMentionsMacos {
            job_id: "engine-machines".to_string(),
        }]
    );

    let matrix_runs_on = wfs.ci.replacen(
        "    strategy:\n      fail-fast: false\n      matrix:\n        machine: [linux/1, linux/2, linux/3, linux/4, linux/5, linux/6, linux/7, linux/8]\n    runs-on: ${{ fromJSON(vars.BROKKR_LOCAL_CI == 'true' && github.event_name != 'merge_group' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository) && '[\"self-hosted\", \"Linux\", \"X64\", \"brokkr-engine\"]' || '[\"ubuntu-24.04\"]') }}\n",
        "    strategy:\n      fail-fast: false\n      matrix:\n        machine: [linux/1, linux/2, linux/3, linux/4, linux/5, linux/6, linux/7, linux/8]\n        os: [ubuntu-24.04, macos-latest]\n    runs-on: ${{ matrix.os }}\n",
        1,
    );
    assert_ne!(matrix_runs_on, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&matrix_runs_on),
        vec![SplitProblem::CiJobMentionsMacos {
            job_id: "engine-machines".to_string(),
        }]
    );

    // An unexpanded matrix axis naming macOS does not mark a Linux `runs-on` job as macOS.
    let unexpanded_axis = wfs.ci.replacen(
        "    strategy:\n      fail-fast: false\n      matrix:\n        machine: [linux/1, linux/2, linux/3, linux/4, linux/5, linux/6, linux/7, linux/8]\n",
        "    strategy:\n      fail-fast: false\n      matrix:\n        machine: [linux/1, linux/2, linux/3, linux/4, linux/5, linux/6, linux/7, linux/8]\n        note: [macos-latest]\n",
        1,
    );
    assert_ne!(unexpanded_axis, wfs.ci);
    assert_eq!(
        ci_macos_split_problems(&unexpanded_axis),
        Vec::<SplitProblem>::new()
    );
}
