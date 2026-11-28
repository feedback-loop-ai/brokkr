//! Table tests for closed-form workflow-root key and `strategy.matrix`
//! admission across YAML kinds, and structured-read tests for caller
//! shard-list and macOS runner checks (#1417, decision 0071 rulings 3, 5, 8 and 9).

use std::collections::BTreeMap;

use super::{ci_macos_split_problems, ShardWorkflows, SplitProblem};
use crate::workflow::{ClosedWorkflow, ClosedWorkflowError, RefusalKind, Scalar};

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
        YamlKind::String => "machine",
        YamlKind::List => "[a]",
        YamlKind::Mapping => "{ a: 1 }",
    }
}

fn kind_label(kind: YamlKind) -> &'static str {
    match kind {
        YamlKind::Null => "null",
        YamlKind::Bool => "boolean",
        YamlKind::Number => "integer",
        YamlKind::String => "string",
        YamlKind::List | YamlKind::Mapping => "non-scalar YAML sequence or mapping",
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

fn matrix_expected_detail(pos: MatrixPosition, kind: YamlKind) -> Option<String> {
    let label = kind_label(kind);
    match pos {
        MatrixPosition::MatrixValue => {
            (kind != YamlKind::Mapping).then(|| format!("{label} `strategy.matrix`"))
        }
        MatrixPosition::AxisKey => {
            (kind != YamlKind::String).then(|| format!("{label} key in `strategy.matrix`"))
        }
        MatrixPosition::AxisValue => {
            (kind != YamlKind::List).then(|| format!("{label} value for matrix axis `machine`"))
        }
        MatrixPosition::AxisItem | MatrixPosition::IncludeRowValue => None,
        MatrixPosition::IncludeValueAdmittedJob => {
            (kind != YamlKind::List).then(|| "non-row matrix `include`".to_string())
        }
        MatrixPosition::IncludeValueDisallowedJob => Some("matrix `include`".to_string()),
        MatrixPosition::ExcludeValue => Some("matrix `exclude`".to_string()),
        MatrixPosition::IncludeRow => {
            (kind != YamlKind::Mapping).then(|| "non-row matrix `include`".to_string())
        }
        MatrixPosition::IncludeRowKey => {
            (kind != YamlKind::String).then(|| "non-row matrix `include`".to_string())
        }
    }
}

fn matrix_cell_case(
    pos: MatrixPosition,
    kind: YamlKind,
) -> (&'static str, &'static str, String, Option<String>) {
    let (file, job, matrix_expr) = match pos {
        MatrixPosition::MatrixValue => (
            "sample.yml",
            "other",
            value_yaml(kind, "[linux]", "{ os: [linux] }").to_string(),
        ),
        MatrixPosition::AxisKey => (
            "sample.yml",
            "other",
            format!("{{ os: [linux], {}: [linux] }}", key_yaml(kind)),
        ),
        MatrixPosition::AxisValue => (
            "sample.yml",
            "other",
            format!(
                "{{ os: [linux], machine: {} }}",
                value_yaml(kind, "[linux]", "{ a: 1 }")
            ),
        ),
        MatrixPosition::AxisItem => (
            "sample.yml",
            "other",
            format!(
                "{{ os: [linux], machine: [{}] }}",
                value_yaml(kind, "[linux]", "{ a: 1 }")
            ),
        ),
        MatrixPosition::IncludeValueAdmittedJob => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: {} }}",
                value_yaml(kind, "[{ os: ubuntu-24.04 }]", "{ os: ubuntu-24.04 }")
            ),
        ),
        MatrixPosition::IncludeValueDisallowedJob => (
            "sample.yml",
            "other",
            format!(
                "{{ os: [linux], include: {} }}",
                value_yaml(kind, "[{ os: linux }]", "{ os: linux }")
            ),
        ),
        MatrixPosition::ExcludeValue => (
            "sample.yml",
            "other",
            format!(
                "{{ os: [linux], exclude: {} }}",
                value_yaml(kind, "[{ os: linux }]", "{ os: linux }")
            ),
        ),
        MatrixPosition::IncludeRow => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: [{{ os: ubuntu-24.04 }}, {}] }}",
                value_yaml(kind, "[ubuntu-24.04]", "{ os: ubuntu-24.04 }")
            ),
        ),
        MatrixPosition::IncludeRowKey => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: [{{ os: ubuntu-24.04 }}, {{ os: ubuntu-24.04, {}: ubuntu-24.04 }}] }}",
                key_yaml(kind)
            ),
        ),
        MatrixPosition::IncludeRowValue => (
            "ci.yml",
            "pr-caches",
            format!(
                "{{ include: [{{ os: ubuntu-24.04 }}, {{ os: {} }}] }}",
                value_yaml(kind, "[ubuntu-24.04]", "{ a: 1 }")
            ),
        ),
    };
    let yaml = format!(
        "on: workflow_dispatch\njobs:\n  {job}:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {matrix_expr}\n    steps:\n      - run: echo ok\n"
    );
    (file, job, yaml, matrix_expected_detail(pos, kind))
}

fn assert_refusal(
    file: &str,
    yaml: &str,
    expected_kind: RefusalKind,
    expected_place: &str,
    expected_detail: &str,
    ctx: &str,
) {
    match ClosedWorkflow::parse(file, yaml) {
        Err(ClosedWorkflowError::Refusal(r)) => {
            assert_eq!(
                (r.kind, r.file.as_str(), r.place.as_str(), r.detail.as_str()),
                (expected_kind, file, expected_place, expected_detail),
                "{ctx} gave unexpected refusal: {r}"
            );
        }
        other => panic!(
            "{ctx} must fail with {expected_kind:?} refusal naming `{expected_place}`, got {other:?}\n{yaml}"
        ),
    }
}

#[test]
fn non_string_workflow_root_keys_yield_top_level_refusal_across_yaml_kinds() {
    let base = "on: workflow_dispatch\njobs:\n  ok:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: echo ok\n";
    ClosedWorkflow::parse("sample.yml", base)
        .expect("base workflow without extra root key must be admitted");

    let cases = [
        ("null", "null top-level key"),
        ("false", "boolean top-level key"),
        ("42", "integer top-level key"),
        ("1.5", "number top-level key"),
        ("[a]", "non-scalar YAML sequence or mapping top-level key"),
        (
            "{ a: 1 }",
            "non-scalar YAML sequence or mapping top-level key",
        ),
    ];
    for (root_key, expected_detail) in cases {
        let yaml = format!("{root_key}: null\n{base}");
        assert_refusal(
            "sample.yml",
            &yaml,
            RefusalKind::TopLevelKey,
            "workflow root",
            expected_detail,
            &format!("root key `{root_key}`"),
        );
    }
}

fn expected_scalar(kind: YamlKind) -> Scalar {
    match kind {
        YamlKind::Null => Scalar::Null,
        YamlKind::Bool => Scalar::Bool(false),
        YamlKind::Number => Scalar::Int(1),
        YamlKind::String => Scalar::String("linux".to_string()),
        YamlKind::List | YamlKind::Mapping => Scalar::Complex,
    }
}

fn s(val: &str) -> Scalar {
    Scalar::String(val.to_string())
}

fn assert_admitted_matrix(
    pos: MatrixPosition,
    kind: YamlKind,
    wf: &ClosedWorkflow,
    file: &str,
    job: &str,
) {
    let matrix = wf
        .require_job(file, job)
        .expect("admitted workflow must keep job")
        .matrix()
        .expect("admitted cell must keep parsed matrix");
    type MatrixAxes = BTreeMap<String, Vec<Scalar>>;
    type MatrixRows = Vec<BTreeMap<String, Scalar>>;
    let (expected_axes, expected_include): (MatrixAxes, MatrixRows) = match pos {
        MatrixPosition::MatrixValue => (BTreeMap::from([("os".into(), vec![s("linux")])]), vec![]),
        MatrixPosition::AxisKey | MatrixPosition::AxisValue => (
            BTreeMap::from([
                ("os".into(), vec![s("linux")]),
                ("machine".into(), vec![s("linux")]),
            ]),
            vec![],
        ),
        MatrixPosition::AxisItem => (
            BTreeMap::from([
                ("os".into(), vec![s("linux")]),
                ("machine".into(), vec![expected_scalar(kind)]),
            ]),
            vec![],
        ),
        MatrixPosition::IncludeValueAdmittedJob => (
            BTreeMap::new(),
            vec![BTreeMap::from([("os".into(), s("ubuntu-24.04"))])],
        ),
        MatrixPosition::IncludeRow => (
            BTreeMap::new(),
            vec![
                BTreeMap::from([("os".into(), s("ubuntu-24.04"))]),
                BTreeMap::from([("os".into(), s("ubuntu-24.04"))]),
            ],
        ),
        MatrixPosition::IncludeRowKey => (
            BTreeMap::new(),
            vec![
                BTreeMap::from([("os".into(), s("ubuntu-24.04"))]),
                BTreeMap::from([
                    ("os".into(), s("ubuntu-24.04")),
                    ("machine".into(), s("ubuntu-24.04")),
                ]),
            ],
        ),
        MatrixPosition::IncludeRowValue => (
            BTreeMap::new(),
            vec![
                BTreeMap::from([("os".into(), s("ubuntu-24.04"))]),
                BTreeMap::from([("os".into(), expected_scalar(kind))]),
            ],
        ),
        MatrixPosition::IncludeValueDisallowedJob | MatrixPosition::ExcludeValue => {
            unreachable!("cell ({pos:?}, {kind:?}) must never be admitted")
        }
    };
    assert_eq!(
        (&matrix.axes, &matrix.include),
        (&expected_axes, &expected_include),
        "admitted cell ({pos:?}, {kind:?}) did not retain expected parsed matrix values"
    );
}

#[test]
fn matrix_position_by_yaml_kind_table_admits_or_refuses_without_parse_error() {
    for pos in ALL_POSITIONS {
        for kind in ALL_KINDS {
            let (file, job, yaml, expected_detail) = matrix_cell_case(pos, kind);
            match expected_detail {
                None => {
                    let wf = ClosedWorkflow::parse(file, &yaml).unwrap_or_else(|err| {
                        panic!("cell ({pos:?}, {kind:?}) expected admitted, got {err:?}\n{yaml}")
                    });
                    assert_admitted_matrix(pos, kind, &wf, file, job);
                }
                Some(detail) => assert_refusal(
                    file,
                    &yaml,
                    RefusalKind::MatrixClause,
                    &format!("job {job} strategy.matrix"),
                    &detail,
                    &format!("cell ({pos:?}, {kind:?})"),
                ),
            }
        }
    }

    // Observable effect: a bare step `if: matrix.os` admits String include rows and refuses every
    // non-string include-row value (dropping e.g. `os: false` would wrongly treat row 2 as omitted).
    for kind in ALL_KINDS {
        let yaml = format!(
            "on: workflow_dispatch\njobs:\n  pr-caches:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {{ include: [{{ os: ubuntu-24.04 }}, {{ os: {} }}] }}\n    steps:\n      - if: matrix.os\n        run: echo ok\n",
            value_yaml(kind, "[ubuntu-24.04]", "{ a: 1 }")
        );
        if kind == YamlKind::String {
            let wf = ClosedWorkflow::parse("ci.yml", &yaml)
                .expect("bare matrix.os over all-string include rows must admit");
            assert_admitted_matrix(
                MatrixPosition::IncludeRowValue,
                kind,
                &wf,
                "ci.yml",
                "pr-caches",
            );
        } else {
            assert_refusal(
                "ci.yml",
                &yaml,
                RefusalKind::ConditionBarePath,
                "job pr-caches step 1 if",
                "bare matrix axis `matrix.os` does not name a non-empty string row with all other rows non-empty strings or omitted",
                &format!("bare matrix.os with IncludeRowValue {kind:?}"),
            );
        }
    }

    // Also verify every row-key kind under `exclude` refuses with MatrixClause, never Parse.
    for kind in ALL_KINDS {
        let yaml = format!(
            "on: workflow_dispatch\njobs:\n  other:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {{ os: [linux], exclude: [{{ os: linux, {}: linux }}] }}\n    steps:\n      - run: echo ok\n",
            key_yaml(kind)
        );
        assert_refusal(
            "sample.yml",
            &yaml,
            RefusalKind::MatrixClause,
            "job other strategy.matrix",
            "matrix `exclude`",
            &format!("exclude row key {kind:?}"),
        );
    }
}

#[test]
fn job_level_if_refuses_step_only_contexts_and_preserves_step_if() {
    let job_if_cases = [
        (
            "matrix.os == 'linux'",
            "comparison on `matrix.os` on a job `if` (available only on step `if`)",
        ),
        (
            "matrix.os",
            "bare matrix axis `matrix.os` on a job `if` (available only on step `if`)",
        ),
        (
            "runner.environment == 'self-hosted'",
            "comparison on `runner.environment` on a job `if` (available only on step `if`)",
        ),
        (
            "always() && matrix.os == 'linux'",
            "comparison on `matrix.os` on a job `if` (available only on step `if`)",
        ),
        (
            "github.event_name == 'workflow_dispatch' || runner.environment == 'self-hosted'",
            "comparison on `runner.environment` on a job `if` (available only on step `if`)",
        ),
    ];
    for (expr, expected_detail) in job_if_cases {
        let yaml = format!(
            "on: workflow_dispatch\njobs:\n  sample:\n    if: {expr}\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {{ os: [linux] }}\n    steps:\n      - run: echo ok\n"
        );
        assert_refusal(
            "sample.yml",
            &yaml,
            RefusalKind::ConditionUnbound,
            "job sample if",
            expected_detail,
            &format!("job if `{expr}`"),
        );
    }

    // The same step-only contexts remain admitted on step `if:`, and job-scoped contexts stay valid on job `if:`.
    for expr in [
        "matrix.os == 'linux'",
        "matrix.os",
        "runner.environment == 'self-hosted'",
    ] {
        let yaml = format!(
            "on: workflow_dispatch\njobs:\n  sample:\n    if: github.event_name == 'workflow_dispatch'\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: {{ os: [linux] }}\n    steps:\n      - if: {expr}\n        run: echo ok\n"
        );
        ClosedWorkflow::parse("sample.yml", &yaml)
            .unwrap_or_else(|err| panic!("step if `{expr}` must be admitted, got {err:?}\n{yaml}"));
    }

    // Admitted `include`-matrix job also refuses bare `matrix.*` on job `if:` while admitting it on step `if:`.
    let include_job_if = "on: workflow_dispatch\njobs:\n  pr-caches:\n    if: matrix.os\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: { include: [{ os: ubuntu-24.04 }] }\n    steps:\n      - run: echo ok\n";
    assert_refusal(
        "ci.yml",
        include_job_if,
        RefusalKind::ConditionUnbound,
        "job pr-caches if",
        "bare matrix axis `matrix.os` on a job `if` (available only on step `if`)",
        "include job if bare matrix.os",
    );
    let include_step_if = "on: workflow_dispatch\njobs:\n  pr-caches:\n    runs-on: ubuntu-24.04\n    strategy:\n      matrix: { include: [{ os: ubuntu-24.04 }] }\n    steps:\n      - if: matrix.os\n        run: echo ok\n";
    ClosedWorkflow::parse("ci.yml", include_step_if)
        .expect("include job step if bare matrix.os must admit");
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
