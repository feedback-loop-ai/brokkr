//! Issue #1383 (operator ruling 2026-11-25): the macOS test shards leave the
//! pull-request path for nightly, manual dispatch and release runs, replaced
//! on pull requests and merge groups by `build (macos-latest)`.

use super::{events, needs_of, workspace};
use crate::workflow;

/// Owned body of job `id` in `text`, or a diagnostic naming `file`.
fn job_of(text: &str, file: &str, id: &str) -> Result<String, String> {
    workflow::jobs(text)
        .into_iter()
        .find_map(|(seen, body)| (seen == id).then_some(body))
        .ok_or_else(|| format!("{file} has no {id} job"))
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
        Err(err) => problems.push(err),
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
        Err(err) => problems.push(err),
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
        Err(err) => problems.push(err),
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
            Err(err) => problems.push(err),
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
        Err(err) => problems.push(err),
    }
}

/// Check that a caller reuses `macos-test-shards.yml` and keeps verdict `test (macos-latest)`.
fn check_shard_caller(file: &str, text: &str, wants_cache: bool, problems: &mut Vec<String>) {
    if text.contains("macos/1") {
        problems.push(format!("{file} copied the macOS shard list"));
    }
    match job_of(text, file, "macos-machines") {
        Ok(body) => {
            if !body.contains("    uses: ./.github/workflows/macos-test-shards.yml\n") {
                problems.push(format!(
                    "{file}'s macos-machines does not call macos-test-shards.yml"
                ));
            }
            let enables_cache = body.contains("use-cache: true");
            if enables_cache != wants_cache {
                problems.push(format!(
                    "{file}'s macos-machines use-cache={enables_cache}, expected {wants_cache}"
                ));
            }
        }
        Err(err) => problems.push(err),
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
        Err(err) => problems.push(err),
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
        }
        Err(err) => problems.push(err),
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
            Err(err) => problems.push(err),
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

#[test]
fn macos_test_shards_run_nightly_on_dispatch_and_in_release_with_failure_issue() {
    let root = workspace();
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let shards = read(".github/workflows/macos-test-shards.yml");
    let nightly = read(".github/workflows/macos-tests.yml");
    let ci = read(".github/workflows/ci.yml");
    let release = read(".github/workflows/release.yml");
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
}

#[test]
fn contributing_records_macos_build_handoff_and_shard_schedule() {
    let contributing = std::fs::read_to_string(workspace().join("CONTRIBUTING.md")).unwrap();
    for required in [
        "`build (macos-latest)`",
        "controller adds `build (macos-latest)`",
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
