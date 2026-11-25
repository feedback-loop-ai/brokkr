//! Issue #1383 (operator ruling 2026-11-25): the macOS test shards leave the
//! pull-request path for nightly, manual dispatch and release runs, replaced
//! on pull requests and merge groups by `build (macos-latest)`.

use std::collections::{BTreeMap, BTreeSet};

use super::{events, needs_of, workspace};
use crate::workflow::{self, ActionMatch, ParsedJob, ParsedStep, ParsedSteps, YamlMap};

/// Missing workflow job lookup error (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum JobLookupError {
    #[error("{file} has no {id} job")]
    Missing { file: String, id: String },
}

/// Owned body of job `id` in `text`, or a typed missing-job error naming `file`.
fn job_of(text: &str, file: &str, id: &str) -> Result<String, JobLookupError> {
    workflow::jobs(text)
        .into_iter()
        .find_map(|(seen, body)| (seen == id).then_some(body))
        .ok_or_else(|| JobLookupError::Missing {
            file: file.to_string(),
            id: id.to_string(),
        })
}

/// Push each message whose `needle` is missing from `body`.
fn require_needles(body: &str, needles: &[(&str, &str)], problems: &mut Vec<String>) {
    for &(needle, msg) in needles {
        if !body.contains(needle) {
            problems.push(msg.to_string());
        }
    }
}

/// Check C1 and C2 on `ci.yml`: no macOS test shard runs on `pull_request` or
/// `merge_group`, `engine` judges Linux only, and `build-macos` (`build
/// (macos-latest)`) compiles the whole workspace and test targets on macOS
/// without running tests.
fn ci_macos_split_problems(ci: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let non_comment = |body: &str| -> String {
        body.lines()
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
    };
    match job_of(ci, "ci.yml", "engine-machines") {
        Ok(body) if non_comment(&body).to_lowercase().contains("macos") => {
            problems.push("engine-machines still mentions macOS".to_string());
        }
        Ok(_) => {}
        Err(err) => problems.push(err.to_string()),
    }
    match job_of(ci, "ci.yml", "engine") {
        Ok(body) => {
            require_needles(
                &body,
                &[(
                    "    name: test (ubuntu-latest)\n",
                    "engine's name is not `test (ubuntu-latest)`",
                )],
                &mut problems,
            );
            if non_comment(&body).to_lowercase().contains("macos") {
                problems.push("engine still mentions macOS".to_string());
            }
        }
        Err(err) => problems.push(err.to_string()),
    }
    match job_of(ci, "ci.yml", "build-macos") {
        Ok(body) => {
            require_needles(
                &body,
                &[
                    (
                        "    name: build (macos-latest)\n",
                        "build-macos name is not `build (macos-latest)`",
                    ),
                    (
                        "    if: github.event_name != 'push'\n",
                        "build-macos does not run on pull_request and merge_group",
                    ),
                    (
                        "    runs-on: macos-latest\n",
                        "build-macos does not run on macos-latest",
                    ),
                    (
                        "Swatinem/rust-cache@",
                        "build-macos does not reuse rust-cache",
                    ),
                    (
                        "run: cargo test --workspace --all-features --locked --no-run\n",
                        "build-macos does not compile all workspace test targets without running tests",
                    ),
                ],
                &mut problems,
            );
            if body.contains("test-shards.sh") {
                problems.push("build-macos runs test-shards.sh".to_string());
            }
        }
        Err(err) => problems.push(err.to_string()),
    }
    for (id, cond) in [
        (
            "macos-machines",
            "    if: github.event_name == 'workflow_dispatch'\n",
        ),
        (
            "macos-tests",
            "    if: always() && github.event_name == 'workflow_dispatch'\n",
        ),
    ] {
        match job_of(ci, "ci.yml", id) {
            Ok(body) if !body.contains(cond) => {
                problems.push(format!(
                    "ci.yml's {id} does not gate on workflow_dispatch alone"
                ));
            }
            Ok(_) => {}
            Err(err) => problems.push(err.to_string()),
        }
    }
    if ci.contains("issues: write") {
        problems.push("ci.yml gained `issues: write` on the pull-request path".to_string());
    }
    problems
}

/// Check the single reusable shard workflow `macos-test-shards.yml` (C3).
fn check_reusable_shards(shards: &str, problems: &mut Vec<String>) {
    if events(shards) != ["workflow_call"] {
        problems.push(format!(
            "macos-test-shards.yml events are {:?}, not [workflow_call]",
            events(shards)
        ));
    }
    match workflow::nested_map(shards, &["on", "workflow_call", "inputs", "use-cache"]) {
        Some(YamlMap::Entries(input)) => {
            let is_bool = input
                .get("type")
                .and_then(|v| workflow::parse_yaml_literal(v))
                .as_deref()
                == Some("boolean");
            if !is_bool {
                problems
                    .push("macos-test-shards.yml use-cache input is not typed boolean".to_string());
            }
            let default_false = input
                .get("default")
                .is_some_and(|v| workflow::parse_yaml_bool(v) == Ok(false));
            if !default_false {
                problems.push(
                    "macos-test-shards.yml use-cache input does not default to false".to_string(),
                );
            }
        }
        Some(YamlMap::Unreadable(raw)) => problems.push(format!(
            "macos-test-shards.yml use-cache input is unreadable: `{raw}`"
        )),
        None => problems.push("macos-test-shards.yml has no use-cache input".to_string()),
    }
    match job_of(shards, "macos-test-shards.yml", "macos-machines") {
        Ok(body) => require_needles(
            &body,
            &[
                (
                    "machine: [macos/1, macos/2, macos/3, macos/4, macos/5]",
                    "macos-test-shards.yml lost the five macOS shards",
                ),
                (
                    "run: bash scripts/test-shards.sh test --parallel-heaps",
                    "macos-test-shards.yml lost scripts/test-shards.sh",
                ),
                (
                    "      - run: bash scripts/test-shards.sh test --parallel-heaps\n        env:\n          BROKKR_TEST_MACHINE: ${{ matrix.machine }}\n",
                    "macos-test-shards.yml shard step lost BROKKR_TEST_MACHINE: ${{ matrix.machine }}",
                ),
                (
                    "seatbelt_probe::native::native_startup_feasibility_probe",
                    "macos-test-shards.yml lost Gate A (startup probe)",
                ),
                (
                    "if: matrix.machine == 'macos/1'",
                    "Gate A is not scoped to macos/1",
                ),
                (
                    "if: inputs.use-cache",
                    "macos-test-shards.yml does not gate rust-cache on inputs.use-cache",
                ),
                (
                    "cargo run --locked -p brokkr-cli -- library compile --bundle recipes/self",
                    "macos-test-shards.yml lost recipes/self compile",
                ),
                (
                    "cargo run --locked -p brokkr-cli -- library compile --bundle recipes/verify",
                    "macos-test-shards.yml lost recipes/verify compile",
                ),
            ],
            problems,
        ),
        Err(err) => problems.push(err.to_string()),
    }
}

/// Check that a caller reuses `macos-test-shards.yml` and keeps verdict `test (macos-latest)`.
fn check_shard_caller(file: &str, text: &str, wants_cache: bool, problems: &mut Vec<String>) {
    if text.contains("macos/1") {
        problems.push(format!("{file} copied the macOS shard list"));
    }
    match job_of(text, file, "macos-machines") {
        Ok(body) => {
            let job = workflow::parse_job(&body);
            if let Some(reason) = &job.unreadable {
                problems.push(format!("{file}'s macos-machines is unreadable: {reason}"));
            }
            let calls_shards = job
                .uses
                .as_deref()
                .and_then(workflow::parse_yaml_literal)
                .as_deref()
                == Some("./.github/workflows/macos-test-shards.yml");
            if !calls_shards {
                problems.push(format!(
                    "{file}'s macos-machines does not call macos-test-shards.yml"
                ));
            }
            let cache_setting = match &job.with {
                None => Ok(false),
                Some(YamlMap::Unreadable(raw)) => Err(format!("unreadable with: `{raw}`")),
                Some(YamlMap::Entries(with)) => match with.get("use-cache") {
                    None => Ok(false),
                    Some(raw) => workflow::parse_yaml_bool(raw),
                },
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
        Err(err) => problems.push(err.to_string()),
    }
    match job_of(text, file, "macos-tests") {
        Ok(body) => {
            require_needles(
                &body,
                &[
                    (
                        "    name: test (macos-latest)\n",
                        "macos-tests is not named `test (macos-latest)`",
                    ),
                    (
                        "test \"$MACHINES\" = success",
                        "macos-tests does not check MACHINES=success",
                    ),
                ],
                problems,
            );
            if !needs_of(&body, file).iter().any(|n| n == "macos-machines") {
                problems.push(format!("{file}'s macos-tests does not need macos-machines"));
            }
        }
        Err(err) => problems.push(err.to_string()),
    }
}

/// Extract every `${{ ... }}` GitHub Actions expression inside `text`, failing
/// closed on unclosed `${{` delimiters.
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

/// Check that `macos-tests.yml`'s top-level `concurrency:` block serializes
/// scheduled runs across commits without cancelling in-progress runs, while
/// keeping `schedule` and `workflow_dispatch` in separate slots and disallowing
/// any per-run or per-commit expression suffix (e.g. `github.run_number`).
fn check_nightly_concurrency(nightly: &str, problems: &mut Vec<String>) {
    let conc = match workflow::nested_map(nightly, &["concurrency"]) {
        Some(YamlMap::Entries(map)) => map,
        Some(YamlMap::Unreadable(raw)) => {
            problems.push(format!(
                "macos-tests.yml concurrency is unreadable: `{raw}`"
            ));
            return;
        }
        None => {
            problems.push("macos-tests.yml has no workflow concurrency group".to_string());
            return;
        }
    };
    match conc.get("group").and_then(|v| workflow::parse_yaml_scalar(v)) {
        None => problems.push("macos-tests.yml has no readable workflow concurrency group".to_string()),
        Some(group) => match extract_gha_expressions(&group) {
            Err(err) => problems.push(format!(
                "macos-tests.yml concurrency group `{group}` is invalid: {err}"
            )),
            Ok(exprs) if exprs.is_empty() => problems.push(format!(
                "macos-tests.yml concurrency group `{group}` shares pending slots between schedule and workflow_dispatch"
            )),
            Ok(exprs) => {
                for expr in exprs {
                    if expr != "github.event_name" {
                        problems.push(format!(
                            "macos-tests.yml concurrency group `{group}` uses disallowed expression `{expr}` that can let scheduled runs race"
                        ));
                    }
                }
            }
        },
    }
    let cancels_false = conc
        .get("cancel-in-progress")
        .is_some_and(|v| workflow::parse_yaml_bool(v) == Ok(false));
    if !cancels_false {
        problems.push("macos-tests.yml concurrency must set cancel-in-progress: false".to_string());
    }
}

/// Check C4 on `macos-tests.yml`: schedule + dispatch share the same test jobs,
/// and `notify-failure` opens/updates one issue on a failing nightly only.
fn check_nightly_workflow(nightly: &str, problems: &mut Vec<String>) {
    if events(nightly) != ["schedule", "workflow_dispatch"] {
        problems.push(format!(
            "macos-tests.yml events are {:?}, not [schedule, workflow_dispatch]",
            events(nightly)
        ));
    }
    check_nightly_concurrency(nightly, problems);
    match job_of(nightly, "macos-tests.yml", "notify-failure") {
        Ok(body) => {
            require_needles(
                &body,
                &[
                    (
                        "    if: always() && github.event_name == 'schedule' && needs.macos-tests.result != 'success'\n",
                        "notify-failure condition does not match failing nightly schedule only",
                    ),
                    (
                        "      actions: read\n      issues: write\n",
                        "notify-failure permissions are not minimal (actions: read, issues: write)",
                    ),
                    (
                        "LABEL: macos-nightly\n",
                        "notify-failure does not declare its own label",
                    ),
                    (
                        "RUN_URL: ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}\n",
                        "notify-failure does not pass RUN_URL via env",
                    ),
                    (
                        "actions/runs/${RUN_ID}/jobs",
                        "notify-failure does not query failed jobs",
                    ),
                    (
                        "issues?labels=${LABEL}&state=open",
                        "notify-failure does not look up an existing open issue by label",
                    ),
                    (
                        "gh api -X PATCH \"repos/${REPO}/issues/${existing}\"",
                        "notify-failure does not update an existing open issue",
                    ),
                    (
                        "gh api -X POST \"repos/${REPO}/issues\"",
                        "notify-failure does not open a new issue when none exists",
                    ),
                ],
                problems,
            );
            if body.contains("contents: write") {
                problems.push("notify-failure asks for contents: write".to_string());
            }
            let lookup = body.find(
                "existing=\"$(gh api --paginate \"repos/${REPO}/issues?labels=${LABEL}&state=open",
            );
            let create = body.find("gh api -X POST \"repos/${REPO}/issues\"");
            if !matches!((lookup, create), (Some(l), Some(c)) if l < c) {
                problems.push(
                    "notify-failure does not paginate open issues immediately before creating one"
                        .to_string(),
                );
            }
        }
        Err(err) => problems.push(err.to_string()),
    }
}

/// Check C3 and C4 across `macos-test-shards.yml`, `macos-tests.yml`, `ci.yml`
/// and `release.yml`.
fn macos_shards_and_nightly_problems(
    shards: &str,
    nightly: &str,
    ci: &str,
    release: &str,
) -> Vec<String> {
    let mut problems = Vec::new();
    check_reusable_shards(shards, &mut problems);
    check_nightly_workflow(nightly, &mut problems);
    for (file, text, wants_cache) in [
        ("macos-tests.yml", nightly, true),
        ("ci.yml", ci, true),
        ("release.yml", release, false),
    ] {
        check_shard_caller(file, text, wants_cache, &mut problems);
    }
    for id in ["build", "publish"] {
        match job_of(release, "release.yml", id) {
            Ok(body)
                if !needs_of(&body, "release.yml")
                    .iter()
                    .any(|n| n == "macos-tests") =>
            {
                problems.push(format!("release.yml's {id} does not wait on macos-tests"));
            }
            Ok(_) => {}
            Err(err) => problems.push(err.to_string()),
        }
    }
    problems
}

#[test]
fn pull_request_path_replaces_macos_shards_with_macos_build() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();
    assert_eq!(ci_macos_split_problems(&ci), Vec::<String>::new());

    let mut_no_run = ci.replacen("--locked --no-run", "--locked", 1);
    assert!(
        !ci_macos_split_problems(&mut_no_run).is_empty(),
        "dropping --no-run from build-macos must fail"
    );
    let mut_perm = format!("{ci}\n# issues: write\n");
    assert!(
        !ci_macos_split_problems(&mut_perm).is_empty(),
        "adding issues: write to ci.yml must fail"
    );
}

fn shard_workflows() -> (String, String, String, String) {
    let root = workspace();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    (
        read(".github/workflows/macos-test-shards.yml"),
        read(".github/workflows/macos-tests.yml"),
        read(".github/workflows/ci.yml"),
        read(".github/workflows/release.yml"),
    )
}

#[test]
fn macos_test_shards_run_nightly_on_dispatch_and_in_release_with_failure_issue() {
    let (shards, nightly, ci, release) = shard_workflows();
    assert_eq!(
        macos_shards_and_nightly_problems(&shards, &nightly, &ci, &release),
        Vec::<String>::new()
    );

    let mut_release_pub = release.replacen("needs: [build, macos-tests]", "needs: build", 1);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &nightly, &ci, &mut_release_pub).is_empty(),
        "release publish not waiting on macos-tests must fail"
    );
    let mut_nightly_cond = nightly.replacen(
        "github.event_name == 'schedule' && needs.macos-tests.result != 'success'",
        "github.event_name == 'schedule'",
        1,
    );
    assert!(
        !macos_shards_and_nightly_problems(&shards, &mut_nightly_cond, &ci, &release).is_empty(),
        "notify-failure running on a passing nightly must fail"
    );

    let mut_no_machine = shards.replacen(
        "        env:\n          BROKKR_TEST_MACHINE: ${{ matrix.machine }}\n",
        "",
        1,
    );
    assert_ne!(mut_no_machine, shards);
    let machine_problems =
        macos_shards_and_nightly_problems(&mut_no_machine, &nightly, &ci, &release);
    assert!(
        machine_problems
            .iter()
            .any(|p| p.contains("BROKKR_TEST_MACHINE")),
        "removing BROKKR_TEST_MACHINE must fail naming it: {machine_problems:?}"
    );

    let mut_sha_group = nightly.replacen(
        "  group: macos-tests-${{ github.event_name }}\n",
        "  group: macos-tests-${{ github.sha }}\n",
        1,
    );
    assert_ne!(mut_sha_group, nightly);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &mut_sha_group, &ci, &release).is_empty(),
        "per-commit concurrency group on macos-tests.yml must fail"
    );

    let mut_shared_group = nightly.replacen(
        "  group: macos-tests-${{ github.event_name }}\n",
        "  group: macos-tests\n",
        1,
    );
    assert_ne!(mut_shared_group, nightly);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &mut_shared_group, &ci, &release).is_empty(),
        "shared concurrency group across schedule and dispatch must fail"
    );

    let mut_unpaginated = nightly.replacen(
        "existing=\"$(gh api --paginate \"repos/${REPO}/issues?labels=${LABEL}&state=open",
        "existing=\"$(gh api \"repos/${REPO}/issues?labels=${LABEL}&state=open",
        1,
    );
    assert_ne!(mut_unpaginated, nightly);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &mut_unpaginated, &ci, &release).is_empty(),
        "unpaginated nightly issue lookup must fail"
    );

    let missing = job_of(&ci, "ci.yml", "no-such-job").unwrap_err();
    assert_eq!(
        missing,
        JobLookupError::Missing {
            file: "ci.yml".to_string(),
            id: "no-such-job".to_string(),
        }
    );
    assert_eq!(missing.to_string(), "ci.yml has no no-such-job job");
}

#[test]
fn nightly_concurrency_rejects_per_run_suffixes_and_cache_input_is_parsed_structurally() {
    let (shards, nightly, ci, release) = shard_workflows();

    for bad_group in [
        "macos-tests-${{ github.event_name }}-${{ github.run_number }}",
        "macos-tests-${{ github.event_name }}-${{ github.run_attempt }}",
        "macos-tests-${{ github.event_name }}-${{ github.sha }}",
        "macos-tests-${{ github.event_name || github.run_number }}",
    ] {
        let mut_group = nightly.replacen(
            "  group: macos-tests-${{ github.event_name }}\n",
            &format!("  group: {bad_group}\n"),
            1,
        );
        assert_ne!(mut_group, nightly);
        assert!(
            !macos_shards_and_nightly_problems(&shards, &mut_group, &ci, &release).is_empty(),
            "concurrency group `{bad_group}` must fail"
        );
    }

    let mut_comment_conc = nightly.replacen("concurrency:\n", "concurrency: # serialize\n", 1);
    assert_eq!(
        macos_shards_and_nightly_problems(&shards, &mut_comment_conc, &ci, &release),
        Vec::<String>::new()
    );

    let mut_cancel_true = nightly.replacen(
        "  cancel-in-progress: false\n",
        "  cancel-in-progress: true\n",
        1,
    );
    assert!(
        !macos_shards_and_nightly_problems(&shards, &mut_cancel_true, &ci, &release).is_empty(),
        "cancel-in-progress: true must fail"
    );

    let mut_ci_comment_only = ci.replacen(
        "    with:\n      use-cache: true\n",
        "    # use-cache: true\n",
        1,
    );
    assert_ne!(mut_ci_comment_only, ci);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &nightly, &mut_ci_comment_only, &release)
            .is_empty(),
        "a comment `# use-cache: true` must not satisfy ci.yml's warm cache requirement"
    );

    let mut_ci_spaced = ci.replacen(
        "      use-cache: true\n",
        "      use-cache:  true # warm\n",
        1,
    );
    assert_eq!(
        macos_shards_and_nightly_problems(&shards, &nightly, &mut_ci_spaced, &release),
        Vec::<String>::new(),
        "valid extra spacing on use-cache: true in ci.yml must pass"
    );

    let mut_release_comment = release.replacen(
        "    uses: ./.github/workflows/macos-test-shards.yml\n",
        "    uses: ./.github/workflows/macos-test-shards.yml\n    # use-cache: true\n",
        1,
    );
    assert_eq!(
        macos_shards_and_nightly_problems(&shards, &nightly, &ci, &mut_release_comment),
        Vec::<String>::new(),
        "a comment mentioning use-cache: true in release.yml does not enable caching"
    );

    for bad_release_with in [
        "    with:\n      use-cache:  true\n",
        "    with:\n      use-cache: true # warm\n",
        "    with:\n      use-cache: 'true'\n",
        "    with: { use-cache: true }\n",
    ] {
        let mut_release = release.replacen(
            "    uses: ./.github/workflows/macos-test-shards.yml\n",
            &format!("    uses: ./.github/workflows/macos-test-shards.yml\n{bad_release_with}"),
            1,
        );
        assert!(
            !macos_shards_and_nightly_problems(&shards, &nightly, &ci, &mut_release).is_empty(),
            "release.yml with `{}` must fail",
            bad_release_with.trim()
        );
    }

    let mut_ci_quoted = ci.replacen("      use-cache: true\n", "      use-cache: 'true'\n", 1);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &nightly, &mut_ci_quoted, &release).is_empty(),
        "quoted use-cache: 'true' in ci.yml must fail"
    );

    let mut_default_true =
        shards.replacen("        default: false\n", "        default: true\n", 1);
    assert!(
        !macos_shards_and_nightly_problems(&mut_default_true, &nightly, &ci, &release).is_empty(),
        "macos-test-shards.yml defaulting use-cache to true must fail"
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
    ] {
        assert!(
            contributing.contains(required),
            "CONTRIBUTING.md does not record {required}"
        );
    }
}

/// Parse the `build` matrix in `release.yml`, returning `(all_names, linux_pkg_names)`.
fn build_matrix_artifact_names(build_body: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut all = BTreeSet::new();
    let mut linux_pkg = BTreeSet::new();
    for line in build_body.lines().map(str::trim) {
        let Some(entry) = line.strip_prefix("- {").and_then(|l| l.strip_suffix('}')) else {
            continue;
        };
        let mut name = None;
        let mut has_nfpm = false;
        for pair in entry.split(',') {
            let Some((k, v)) = pair.split_once(':') else {
                continue;
            };
            match k.trim() {
                "name" => name = workflow::parse_yaml_literal(v),
                "nfpm_arch" => has_nfpm = true,
                _ => {}
            }
        }
        if let Some(name) = name {
            if has_nfpm {
                linux_pkg.insert(name.clone());
            }
            all.insert(name);
        }
    }
    (all, linux_pkg)
}

/// Extract readable steps from a parsed job, pushing problems for any
/// unreadable job, `steps:` block, or individual step.
fn readable_steps<'a>(
    source: &str,
    job_id: &str,
    job: &'a ParsedJob,
    problems: &mut Vec<String>,
) -> &'a [ParsedStep] {
    if let Some(reason) = &job.unreadable {
        problems.push(format!("{source}'s {job_id} is unreadable: {reason}"));
    }
    match &job.steps {
        None => &[],
        Some(ParsedSteps::Unreadable(reason)) => {
            problems.push(format!(
                "{source}'s {job_id} has unreadable steps: {reason}"
            ));
            &[]
        }
        Some(ParsedSteps::Items(steps)) => {
            for step in steps {
                if let Some(reason) = &step.unreadable {
                    problems.push(format!("{source}'s {job_id} has unreadable step: {reason}"));
                }
            }
            steps.as_slice()
        }
    }
}

/// Extract local reusable workflows called at job level, recording problems for
/// any unparseable or external reusable workflow `uses:`.
fn called_workflows_in(
    source: &str,
    workflow_text: &str,
    problems: &mut Vec<String>,
) -> Vec<String> {
    let mut called = Vec::new();
    for (job_id, body) in workflow::jobs(workflow_text) {
        let job = workflow::parse_job(&body);
        let Some(raw_uses) = job.uses else {
            continue;
        };
        match workflow::parse_yaml_literal(&raw_uses) {
            None => problems.push(format!(
                "{source}'s {job_id} has unparseable reusable workflow uses `{raw_uses}`"
            )),
            Some(target) => match target
                .strip_prefix("./")
                .filter(|path| path.starts_with(".github/workflows/"))
            {
                Some(path) => called.push(path.to_string()),
                None => problems.push(format!(
                    "{source}'s {job_id} calls external workflow `{target}`, whose uploads are not checked"
                )),
            },
        }
    }
    called
}

/// Check that a `download-artifact` step's `with:` keys stay within
/// `{name, path}`, rejecting cross-run selectors (`run-id`, `repository`,
/// `github-token`), `pattern`, `merge-multiple`, and any unknown input.
fn check_download_with_keys(
    job_id: &str,
    with: &BTreeMap<String, String>,
    problems: &mut Vec<String>,
) {
    for key in with.keys() {
        match key.as_str() {
            "name" | "path" => {}
            "pattern" => problems.push(format!(
                "release.yml's {job_id} downloads artifacts by pattern instead of exact build name"
            )),
            "merge-multiple" => problems.push(format!(
                "release.yml's {job_id} uses merge-multiple instead of exact build artifact names"
            )),
            "run-id" | "repository" | "github-token" => problems.push(format!(
                "release.yml's {job_id} sets `{key}` on download-artifact, which selects artifacts outside the current run"
            )),
            other => problems.push(format!(
                "release.yml's {job_id} sets unexpected download-artifact input `{other}`"
            )),
        }
    }
}

/// Check that `release.yml`'s `build` job unconditionally uploads exactly
/// `${{ matrix.name }}` via `actions/upload-artifact` for every matrix row.
fn check_build_uploads(build_job: &ParsedJob, problems: &mut Vec<String>) {
    if let Some(cond) = &build_job.if_cond {
        problems.push(format!(
            "release.yml's build job is conditional (`{cond}`) and may skip matrix artifacts"
        ));
    }
    let steps = readable_steps("release.yml", "build", build_job, problems);
    let mut upload_names = Vec::new();
    for step in steps {
        let Some(raw_uses) = step.uses.as_deref() else {
            continue;
        };
        match workflow::action_repo_match(raw_uses, "actions/upload-artifact") {
            None => problems.push(format!(
                "release.yml's build step has unparseable uses `{raw_uses}`"
            )),
            Some(ActionMatch::Other) => continue,
            Some(ActionMatch::SubAction(sub)) => {
                problems.push(format!(
                    "release.yml's build uses `{sub}` instead of `actions/upload-artifact`"
                ));
                upload_names.push(None);
                continue;
            }
            Some(ActionMatch::Exact) => {}
        }
        if let Some(cond) = &step.if_cond {
            problems.push(format!(
                "release.yml's build upload step is conditional (`{cond}`) and may skip matrix artifacts"
            ));
        }
        match &step.with {
            Some(YamlMap::Entries(with)) => {
                let name = with
                    .get("name")
                    .and_then(|raw| workflow::parse_yaml_scalar(raw));
                upload_names.push(name);
            }
            Some(YamlMap::Unreadable(raw)) => {
                problems.push(format!(
                    "release.yml's build upload-artifact has unreadable with: `{raw}`"
                ));
                upload_names.push(None);
            }
            None => upload_names.push(None),
        }
    }
    if upload_names != [Some("${{ matrix.name }}".to_string())] {
        problems.push(format!(
            "release.yml's build upload names are {upload_names:?}, expected [Some(\"${{{{ matrix.name }}}}\")]"
        ));
    }
}

/// Check that every `actions/download-artifact` step in `job_id` stays bound to
/// the current workflow run, selects an exact literal artifact name from
/// `allowed`, is unconditional when `expected` is required, and matches
/// `expected`.
fn check_download_step(
    job_id: &str,
    step: &ParsedStep,
    require_unconditional: bool,
    allowed: &BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    if require_unconditional {
        if let Some(cond) = &step.if_cond {
            problems.push(format!(
                "release.yml's {job_id} download step is conditional (`{cond}`) and may skip a required artifact"
            ));
        }
    }
    let with = match &step.with {
        Some(YamlMap::Entries(map)) => map,
        Some(YamlMap::Unreadable(raw)) => {
            problems.push(format!(
                "release.yml's {job_id} download-artifact has unreadable with: `{raw}`"
            ));
            return;
        }
        None => {
            problems.push(format!(
                "release.yml's {job_id} downloads all artifacts of the run without an exact name"
            ));
            return;
        }
    };
    check_download_with_keys(job_id, with, problems);
    match with.get("name") {
        None => problems.push(format!(
            "release.yml's {job_id} downloads all artifacts of the run without an exact name"
        )),
        Some(raw) => match workflow::parse_yaml_literal(raw) {
            None => problems.push(format!(
                "release.yml's {job_id} download name `{raw}` is not an exact literal name"
            )),
            Some(name) if name.contains(['*', '?', '[']) => problems.push(format!(
                "release.yml's {job_id} download name `{name}` contains wildcard characters"
            )),
            Some(name) => {
                if !allowed.contains(&name) {
                    problems.push(format!(
                        "release.yml's {job_id} downloads `{name}`, which is not an allowed build artifact"
                    ));
                }
                seen.insert(name);
            }
        },
    }
}

fn check_job_downloads(
    job_id: &str,
    job: &ParsedJob,
    expected: Option<&BTreeSet<String>>,
    allowed: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    if expected.is_some() {
        if let Some(cond) = &job.if_cond {
            problems.push(format!(
                "release.yml's {job_id} job is conditional (`{cond}`) and may skip required downloads"
            ));
        }
    }
    let steps = readable_steps("release.yml", job_id, job, problems);
    let mut seen = BTreeSet::new();
    for step in steps {
        let Some(raw_uses) = step.uses.as_deref() else {
            continue;
        };
        match workflow::action_repo_match(raw_uses, "actions/download-artifact") {
            None => problems.push(format!(
                "release.yml's {job_id} step has unparseable uses `{raw_uses}`"
            )),
            Some(ActionMatch::Other) => {}
            Some(ActionMatch::Exact | ActionMatch::SubAction(_)) => check_download_step(
                job_id,
                step,
                expected.is_some(),
                allowed,
                &mut seen,
                problems,
            ),
        }
    }
    if let Some(expected) = expected {
        if &seen != expected {
            problems.push(format!(
                "release.yml's {job_id} downloads {seen:?}, expected {expected:?}"
            ));
        }
    }
}

/// Check that every `actions/upload-artifact` (including sub-actions such as
/// `actions/upload-artifact/merge`) step in `job` uses a resolved literal name
/// that cannot collide with any `build` artifact name.
fn check_non_build_uploads(
    source: &str,
    job_id: &str,
    job: &ParsedJob,
    build_names: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    let steps = readable_steps(source, job_id, job, problems);
    for step in steps {
        let Some(raw_uses) = step.uses.as_deref() else {
            continue;
        };
        match workflow::action_repo_match(raw_uses, "actions/upload-artifact") {
            None => {
                problems.push(format!(
                    "{source}'s {job_id} step has unparseable uses `{raw_uses}`"
                ));
                continue;
            }
            Some(ActionMatch::Other) => continue,
            Some(ActionMatch::Exact | ActionMatch::SubAction(_)) => {}
        }
        let with = match &step.with {
            Some(YamlMap::Entries(map)) => map,
            Some(YamlMap::Unreadable(raw)) => {
                problems.push(format!(
                    "{source}'s {job_id} upload-artifact has unreadable with: `{raw}`"
                ));
                continue;
            }
            None => {
                problems.push(format!("{source}'s {job_id} uploads an unnamed artifact"));
                continue;
            }
        };
        match with.get("name") {
            None => problems.push(format!("{source}'s {job_id} uploads an unnamed artifact")),
            Some(raw) => match workflow::parse_yaml_literal(raw) {
                None => problems.push(format!(
                    "{source}'s {job_id} uploads unresolved artifact name `{raw}`"
                )),
                Some(name) if build_names.contains(&name) => problems.push(format!(
                    "{source}'s {job_id} uploads `{name}`, which collides with a build artifact"
                )),
                Some(_) => {}
            },
        }
    }
}

/// H1 (release provenance): `release.yml`'s `publish` job downloads exactly
/// the artifacts `build` unconditionally uploads by exact matrix name, `pages`
/// downloads only the Linux package build artifacts by exact name, no job
/// downloads "all artifacts of the run" or uses a wildcard pattern, and no
/// upload in another job of `release.yml` or any reusable workflow it calls
/// (including `actions/upload-artifact/merge` or quoted local calls) can match
/// the publisher's selection.
fn release_artifact_provenance_problems(
    release: &str,
    called_workflows: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let build_body = match job_of(release, "release.yml", "build") {
        Ok(body) => body,
        Err(err) => return vec![err.to_string()],
    };
    let build_job = workflow::parse_job(&build_body);
    check_build_uploads(&build_job, &mut problems);
    let (build_names, linux_pkg_names) = build_matrix_artifact_names(&build_body);
    if build_names.is_empty() || linux_pkg_names.is_empty() {
        problems.push("release.yml's build matrix yielded no artifact names".to_string());
    }
    let no_artifacts = BTreeSet::new();
    let (mut saw_publish, mut saw_pages) = (false, false);
    for (id, body) in workflow::jobs(release) {
        let job = workflow::parse_job(&body);
        if id == "build" {
            check_job_downloads(
                "build",
                &job,
                Some(&no_artifacts),
                &no_artifacts,
                &mut problems,
            );
            continue;
        }
        check_non_build_uploads("release.yml", &id, &job, &build_names, &mut problems);
        match id.as_str() {
            "publish" => {
                saw_publish = true;
                check_job_downloads(
                    "publish",
                    &job,
                    Some(&build_names),
                    &build_names,
                    &mut problems,
                );
            }
            "pages" => {
                saw_pages = true;
                check_job_downloads(
                    "pages",
                    &job,
                    Some(&linux_pkg_names),
                    &build_names,
                    &mut problems,
                );
            }
            other => check_job_downloads(other, &job, None, &build_names, &mut problems),
        }
    }
    if !saw_publish {
        problems.push("release.yml has no publish job".to_string());
    }
    if !saw_pages {
        problems.push("release.yml has no pages job".to_string());
    }
    check_called_workflows(release, called_workflows, &build_names, &mut problems);
    problems
}

fn check_called_workflows(
    release: &str,
    called_workflows: &BTreeMap<String, String>,
    build_names: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    let mut pending = called_workflows_in("release.yml", release, problems);
    let mut visited = BTreeSet::new();
    while let Some(path) = pending.pop() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let Some(text) = called_workflows.get(&path) else {
            problems.push(format!("release.yml calls {path}, which was not checked"));
            continue;
        };
        for (job_id, body) in workflow::jobs(text) {
            let job = workflow::parse_job(&body);
            check_non_build_uploads(&path, &job_id, &job, build_names, problems);
        }
        pending.extend(called_workflows_in(&path, text, problems));
    }
}

fn release_and_called() -> (String, BTreeMap<String, String>) {
    let root = workspace();
    let release = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let mut problems = Vec::new();
    let mut pending = called_workflows_in("release.yml", &release, &mut problems);
    assert_eq!(problems, Vec::<String>::new());
    let mut called = BTreeMap::new();
    while let Some(path) = pending.pop() {
        if called.contains_key(&path) {
            continue;
        }
        let text = std::fs::read_to_string(root.join(&path)).unwrap();
        pending.extend(called_workflows_in(&path, &text, &mut problems));
        assert_eq!(problems, Vec::<String>::new());
        called.insert(path, text);
    }
    (release, called)
}

fn with_shard_upload_name(
    called: &BTreeMap<String, String>,
    replacement: &str,
) -> BTreeMap<String, String> {
    let mut mutated = called.clone();
    let shards = mutated
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap();
    *shards = shards.replacen(
        "          name: r3-native-diagnostics\n",
        &format!("          name: {replacement}\n"),
        1,
    );
    mutated
}

#[test]
fn release_publishes_only_build_matrix_artifacts() {
    let (release, called) = release_and_called();
    assert!(
        called.contains_key(".github/workflows/macos-test-shards.yml"),
        "release.yml must call macos-test-shards.yml"
    );
    assert_eq!(
        release_artifact_provenance_problems(&release, &called),
        Vec::<String>::new()
    );

    let dl = "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n";
    let widened_publish = release.replacen(
        &format!("{dl}          name: brokkr-linux-x86_64\n          path: dist\n"),
        &format!("{dl}          path: dist\n          merge-multiple: true\n"),
        1,
    );
    assert_ne!(widened_publish, release);
    assert!(
        !release_artifact_provenance_problems(&widened_publish, &called).is_empty(),
        "widening publish to all artifacts must fail"
    );

    let pattern_publish = release.replacen(
        "          name: brokkr-linux-x86_64\n",
        "          pattern: brokkr-*\n",
        1,
    );
    assert_ne!(pattern_publish, release);
    assert!(
        !release_artifact_provenance_problems(&pattern_publish, &called).is_empty(),
        "using a wildcard pattern in publish must fail"
    );

    let widened_pages = release.replacen(
        &format!("{dl}          name: brokkr-linux-aarch64\n          path: dist\n      - name: repository tooling"),
        &format!("{dl}          path: dist\n          merge-multiple: true\n      - name: repository tooling"),
        1,
    );
    assert_ne!(widened_pages, release);
    assert!(
        !release_artifact_provenance_problems(&widened_pages, &called).is_empty(),
        "widening pages to all artifacts must fail"
    );

    let mut called_extra = called.clone();
    called_extra
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap()
        .push_str(
            "      - name: extra diagnostics\n        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n          name: extra-macos-diagnostics\n          path: target/extra.log\n",
        );
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_extra),
        Vec::<String>::new(),
        "an extra non-build upload in macos-test-shards.yml cannot reach publish"
    );

    let called_collide = with_shard_upload_name(&called, "brokkr-macos-arm64");
    assert!(
        !release_artifact_provenance_problems(&release, &called_collide).is_empty(),
        "a called workflow uploading a build artifact name must fail"
    );
}

#[test]
fn release_provenance_rejects_build_downloads_cross_run_inputs_and_disguised_uploads() {
    let (release, called) = release_and_called();

    let attest = "      - name: attest packaged binary and checksum\n";
    let dl = "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n";
    for (label, step) in [
        (
            "download-all in build",
            format!("{dl}          path: dist\n          merge-multiple: true\n"),
        ),
        (
            "named download in build",
            format!("{dl}          name: brokkr-linux-x86_64\n          path: dist\n"),
        ),
    ] {
        let mutated = release.replacen(attest, &format!("{step}{attest}"), 1);
        assert_ne!(mutated, release);
        assert!(
            !release_artifact_provenance_problems(&mutated, &called).is_empty(),
            "{label} must fail"
        );
    }

    let target_dl = "          name: brokkr-linux-x86_64\n          path: dist\n";
    for extra_input in [
        "          run-id: 12345\n",
        "          repository: other/repo\n",
        "          github-token: ${{ secrets.GITHUB_TOKEN }}\n",
        "          artifact-ids: 99\n",
    ] {
        let mutated = release.replacen(target_dl, &format!("{target_dl}{extra_input}"), 1);
        assert_ne!(mutated, release);
        assert!(
            !release_artifact_provenance_problems(&mutated, &called).is_empty(),
            "download-artifact with `{}` must fail",
            extra_input.trim()
        );
    }

    for bad_name in [
        "\"brokkr-macos-arm64\"",
        "'brokkr-macos-arm64'",
        "brokkr-macos-arm64 # diagnostic",
        "'brokkr-macos-arm64' # diagnostic",
        "${{ matrix.name }}",
        "brokkr-${{ matrix.machine }}",
        "|",
        "\"brokkr-\\x6dacos-arm64\"",
    ] {
        let called_mut = with_shard_upload_name(&called, bad_name);
        assert!(
            !release_artifact_provenance_problems(&release, &called_mut).is_empty(),
            "non-build upload name `{bad_name}` must fail"
        );
    }

    let called_ok = with_shard_upload_name(&called, "'r3-native-diagnostics' # human-readable");
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_ok),
        Vec::<String>::new()
    );
}

#[test]
fn release_provenance_scans_commented_steps_merge_subactions_and_quoted_calls() {
    let (release, called) = release_and_called();

    let mut called_commented_steps = with_shard_upload_name(&called, "brokkr-macos-arm64");
    let shards = called_commented_steps
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap();
    *shards = shards
        .replacen("    steps:\n", "    steps: # shard steps\n", 1)
        .replacen("        with:\n", "        with: # upload inputs\n", 1);
    assert!(
        !release_artifact_provenance_problems(&release, &called_commented_steps).is_empty(),
        "colliding upload under `steps: # comment` and `with: # comment` must fail"
    );

    let mut called_flush_steps = called.clone();
    let shards_flush = called_flush_steps
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap();
    *shards_flush = shards_flush.replacen(
        "    steps:\n",
        "    steps:\n    - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n      with:\n        name: brokkr-macos-arm64\n        path: target/r3-startup.log\n",
        1,
    );
    assert!(
        !release_artifact_provenance_problems(&release, &called_flush_steps).is_empty(),
        "colliding upload at 4-space step indentation must fail"
    );

    let mut called_merge_collide = called.clone();
    called_merge_collide
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap()
        .push_str(
            "      - name: merge diagnostics\n        uses: actions/upload-artifact/merge@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n          name: brokkr-macos-arm64\n",
        );
    assert!(
        !release_artifact_provenance_problems(&release, &called_merge_collide).is_empty(),
        "actions/upload-artifact/merge colliding with a build name must fail"
    );

    let mut called_merge_unnamed = called.clone();
    called_merge_unnamed
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap()
        .push_str(
            "      - uses: actions/upload-artifact/merge@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n",
        );
    assert!(
        !release_artifact_provenance_problems(&release, &called_merge_unnamed).is_empty(),
        "unnamed actions/upload-artifact/merge must fail"
    );

    let mut called_merge_safe = called.clone();
    called_merge_safe
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap()
        .push_str(
            "      - uses: actions/upload-artifact/merge@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n          name: merged-r3-diagnostics\n",
        );
    assert_eq!(
        release_artifact_provenance_problems(&release, &called_merge_safe),
        Vec::<String>::new(),
        "actions/upload-artifact/merge with a disjoint literal name cannot reach publish"
    );

    for quoted_call in [
        "'./.github/workflows/extra.yml'",
        "\"./.github/workflows/extra.yml\"",
        "'./.github/workflows/extra.yml' # extra",
    ] {
        let release_with_extra = format!("{release}\n  extra-caller:\n    uses: {quoted_call}\n");
        assert!(
            !release_artifact_provenance_problems(&release_with_extra, &called).is_empty(),
            "unchecked quoted local call `{quoted_call}` must fail"
        );
        let mut called_with_extra = called.clone();
        called_with_extra.insert(
            ".github/workflows/extra.yml".to_string(),
            "on:\n  workflow_call:\njobs:\n  sneaky:\n    runs-on: ubuntu-24.04\n    steps: # hidden\n      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02\n        with:\n          name: brokkr-linux-x86_64\n          path: dist/*\n"
                .to_string(),
        );
        assert!(
            !release_artifact_provenance_problems(&release_with_extra, &called_with_extra)
                .is_empty(),
            "colliding upload in quoted local call `{quoted_call}` must fail"
        );
    }

    let release_remote_call = format!(
        "{release}\n  remote-caller:\n    uses: other/repo/.github/workflows/x.yml@11d5960a326750d5838078e36cf38b85af677262\n"
    );
    assert!(
        !release_artifact_provenance_problems(&release_remote_call, &called).is_empty(),
        "external reusable workflow call in release.yml must fail"
    );

    let mut called_flow_with = called.clone();
    let shards_flow = called_flow_with
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap();
    *shards_flow = shards_flow.replacen(
        "        with:\n          name: r3-native-diagnostics\n",
        "        with: { name: brokkr-macos-arm64 }\n",
        1,
    );
    assert!(
        !release_artifact_provenance_problems(&release, &called_flow_with).is_empty(),
        "flow-map `with:` on upload-artifact must fail closed"
    );
}

#[test]
fn release_provenance_rejects_conditional_build_uploads_and_downloads() {
    let (release, called) = release_and_called();

    let conditional_upload = release.replacen(
        "      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        with:\n",
        "      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2\n        if: matrix.nfpm_arch\n        with:\n",
        1,
    );
    assert_ne!(conditional_upload, release);
    let cond_problems = release_artifact_provenance_problems(&conditional_upload, &called);
    assert!(
        cond_problems.iter().any(|p| p.contains("conditional")),
        "conditional build upload skipping macOS artifacts must fail: {cond_problems:?}"
    );

    let conditional_build_job = release.replacen(
        "  build:\n    name: build (${{ matrix.name }})\n",
        "  build:\n    name: build (${{ matrix.name }})\n    if: matrix.nfpm_arch\n",
        1,
    );
    assert!(
        !release_artifact_provenance_problems(&conditional_build_job, &called).is_empty(),
        "conditional build job must fail"
    );

    let conditional_publish_dl = release.replacen(
        "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        with:\n          name: brokkr-macos-arm64\n",
        "      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0\n        if: false\n        with:\n          name: brokkr-macos-arm64\n",
        1,
    );
    assert!(
        !release_artifact_provenance_problems(&conditional_publish_dl, &called).is_empty(),
        "conditional download in publish must fail"
    );
}
