//! Issue #1383 (operator ruling 2026-11-25): the macOS test shards leave the
//! pull-request path for nightly, manual dispatch and release runs, replaced
//! on pull requests and merge groups by `build (macos-latest)`.

use std::collections::{BTreeMap, BTreeSet};

use super::{events, needs_of, workspace};
use crate::workflow;

/// Missing workflow job lookup error (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
enum JobLookupError {
    Missing { file: String, id: String },
}

impl std::fmt::Display for JobLookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { file, id } => write!(f, "{file} has no {id} job"),
        }
    }
}

impl std::error::Error for JobLookupError {}

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

/// Check C4 on `macos-tests.yml`: schedule + dispatch share the same test jobs,
/// and `notify-failure` opens/updates one issue on a failing nightly only.
/// Top-level `concurrency:` block keys (`group`, `cancel-in-progress`).
fn workflow_concurrency(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some((_, rest)) = text.split_once("\nconcurrency:\n") else {
        return map;
    };
    for line in rest.lines() {
        if !line.starts_with("  ") {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

fn check_nightly_workflow(nightly: &str, problems: &mut Vec<String>) {
    if events(nightly) != ["schedule", "workflow_dispatch"] {
        problems.push(format!(
            "macos-tests.yml events are {:?}, not [schedule, workflow_dispatch]",
            events(nightly)
        ));
    }
    let conc = workflow_concurrency(nightly);
    match conc.get("group") {
        None => problems.push("macos-tests.yml has no workflow concurrency group".to_string()),
        Some(group)
            if group.contains("github.sha")
                || group.contains("github.ref")
                || group.contains("github.run_id") =>
        {
            problems.push(format!(
                "macos-tests.yml concurrency group `{group}` varies per commit/ref and lets two scheduled runs race"
            ));
        }
        Some(_) => {}
    }
    if conc.get("cancel-in-progress").map(String::as_str) != Some("false") {
        problems.push("macos-tests.yml concurrency must set cancel-in-progress: false".to_string());
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
            let lookup =
                body.find("existing=\"$(gh api \"repos/${REPO}/issues?labels=${LABEL}&state=open");
            let create = body.find("gh api -X POST \"repos/${REPO}/issues\"");
            if !matches!((lookup, create), (Some(l), Some(c)) if l < c) {
                problems.push("notify-failure does not look up the open issue immediately before creating one".to_string());
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
        "  group: macos-tests\n",
        "  group: macos-tests-${{ github.sha }}\n",
        1,
    );
    assert_ne!(mut_sha_group, nightly);
    assert!(
        !macos_shards_and_nightly_problems(&shards, &mut_sha_group, &ci, &release).is_empty(),
        "per-commit concurrency group on macos-tests.yml must fail"
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
                "name" => name = Some(v.trim().to_string()),
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

/// Extract the `with:` key-value pairs of every step in `job_body` that uses
/// `action_prefix@<ref>`.
fn step_action_with_blocks(job_body: &str, action_prefix: &str) -> Vec<BTreeMap<String, String>> {
    let Some((_, steps_section)) = job_body.split_once("    steps:\n") else {
        return Vec::new();
    };
    let mut steps: Vec<Vec<&str>> = Vec::new();
    for line in steps_section.lines() {
        if line.starts_with("      - ") {
            steps.push(vec![line]);
        } else if let Some(current) = steps.last_mut() {
            current.push(line);
        }
    }
    let needle = format!("uses: {action_prefix}@");
    steps
        .into_iter()
        .filter(|step| {
            step.iter().any(|line| {
                let trimmed = line.trim_start();
                trimmed
                    .strip_prefix("- ")
                    .unwrap_or(trimmed)
                    .starts_with(&needle)
            })
        })
        .map(|step| parse_with_map(&step))
        .collect()
}

/// Parse the top-level `key: value` pairs inside a step's `with:` block.
fn parse_with_map(step: &[&str]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let mut with_indent: Option<usize> = None;
    let mut entry_indent: Option<usize> = None;
    for &line in step {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let Some(wi) = with_indent else {
            if trimmed == "with:" {
                with_indent = Some(indent);
            }
            continue;
        };
        if indent <= wi {
            break;
        }
        if indent == *entry_indent.get_or_insert(indent) {
            if let Some((k, v)) = trimmed.split_once(':') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    map
}

/// Local reusable workflows called at job level (`    uses: ./.github/workflows/...`).
fn called_local_workflows(workflow_text: &str) -> Vec<String> {
    workflow_text
        .lines()
        .filter_map(|line| line.strip_prefix("    uses: ./"))
        .map(|rest| rest.split('#').next().unwrap_or(rest).trim().to_string())
        .filter(|path| path.starts_with(".github/workflows/"))
        .collect()
}

/// Check that every `actions/download-artifact` step in `job_id` selects an
/// exact literal artifact name (no bare "all artifacts", no `merge-multiple`,
/// no `pattern`, no wildcards) and matches `expected` when required.
fn check_job_downloads(
    job_id: &str,
    body: &str,
    expected: Option<&BTreeSet<String>>,
    allowed: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for with in step_action_with_blocks(body, "actions/download-artifact") {
        if with.contains_key("pattern") {
            problems.push(format!(
                "release.yml's {job_id} downloads artifacts by pattern instead of exact build name"
            ));
        }
        if with.get("merge-multiple").map(String::as_str) == Some("true") {
            problems.push(format!(
                "release.yml's {job_id} uses merge-multiple instead of exact build artifact names"
            ));
        }
        match with.get("name") {
            None => problems.push(format!(
                "release.yml's {job_id} downloads all artifacts of the run without an exact name"
            )),
            Some(name)
                if name.contains('*')
                    || name.contains('?')
                    || name.contains('[')
                    || name.contains("${{") =>
            {
                problems.push(format!(
                    "release.yml's {job_id} download name `{name}` is not an exact literal name"
                ));
            }
            Some(name) => {
                if !allowed.contains(name) {
                    problems.push(format!(
                        "release.yml's {job_id} downloads `{name}`, which is not a build matrix artifact"
                    ));
                }
                seen.insert(name.clone());
            }
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

/// Check that no `actions/upload-artifact` step in `body` collides with any
/// `build` artifact name downloaded by the release publisher or pages job.
fn check_non_build_uploads(
    source: &str,
    job_id: &str,
    body: &str,
    build_names: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    for with in step_action_with_blocks(body, "actions/upload-artifact") {
        match with.get("name") {
            None => problems.push(format!("{source}'s {job_id} uploads an unnamed artifact")),
            Some(name) if build_names.contains(name) => problems.push(format!(
                "{source}'s {job_id} uploads `{name}`, which collides with a build artifact"
            )),
            Some(_) => {}
        }
    }
}

/// H1 (release provenance): `release.yml`'s `publish` job downloads exactly
/// the artifacts `build` uploads by exact matrix name, `pages` downloads only
/// the Linux package build artifacts by exact name, no job downloads "all
/// artifacts of the run" or uses a wildcard pattern, and no upload in another
/// job of `release.yml` or any reusable workflow it calls can match the
/// publisher's selection.
fn release_artifact_provenance_problems(
    release: &str,
    called_workflows: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut problems = Vec::new();
    let build_body = match job_of(release, "release.yml", "build") {
        Ok(body) => body,
        Err(err) => return vec![err.to_string()],
    };
    let build_uploads: Vec<Option<String>> =
        step_action_with_blocks(&build_body, "actions/upload-artifact")
            .into_iter()
            .map(|w| w.get("name").cloned())
            .collect();
    if build_uploads != [Some("${{ matrix.name }}".to_string())] {
        problems.push(format!(
            "release.yml's build upload names are {build_uploads:?}, expected [Some(\"${{{{ matrix.name }}}}\")]"
        ));
    }
    let (build_names, linux_pkg_names) = build_matrix_artifact_names(&build_body);
    if build_names.is_empty() || linux_pkg_names.is_empty() {
        problems.push("release.yml's build matrix yielded no artifact names".to_string());
    }
    let (mut saw_publish, mut saw_pages) = (false, false);
    for (id, body) in workflow::jobs(release) {
        if id == "build" {
            continue;
        }
        check_non_build_uploads("release.yml", &id, &body, &build_names, &mut problems);
        match id.as_str() {
            "publish" => {
                saw_publish = true;
                check_job_downloads(
                    "publish",
                    &body,
                    Some(&build_names),
                    &build_names,
                    &mut problems,
                );
            }
            "pages" => {
                saw_pages = true;
                check_job_downloads(
                    "pages",
                    &body,
                    Some(&linux_pkg_names),
                    &build_names,
                    &mut problems,
                );
            }
            other => check_job_downloads(other, &body, None, &build_names, &mut problems),
        }
    }
    if !saw_publish {
        problems.push("release.yml has no publish job".to_string());
    }
    if !saw_pages {
        problems.push("release.yml has no pages job".to_string());
    }
    for path in called_local_workflows(release) {
        let Some(text) = called_workflows.get(&path) else {
            problems.push(format!("release.yml calls {path}, which was not checked"));
            continue;
        };
        for (job_id, body) in workflow::jobs(text) {
            check_non_build_uploads(&path, &job_id, &body, &build_names, &mut problems);
        }
    }
    problems
}

#[test]
fn release_publishes_only_build_matrix_artifacts() {
    let root = workspace();
    let release = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    let called: BTreeMap<String, String> = called_local_workflows(&release)
        .into_iter()
        .map(|path| {
            let text = std::fs::read_to_string(root.join(&path)).unwrap();
            (path, text)
        })
        .collect();
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

    let mut called_collide = called.clone();
    let shards_collide = called_collide
        .get_mut(".github/workflows/macos-test-shards.yml")
        .unwrap();
    *shards_collide = shards_collide.replacen(
        "          name: r3-native-diagnostics\n",
        "          name: brokkr-macos-arm64\n",
        1,
    );
    assert!(
        !release_artifact_provenance_problems(&release, &called_collide).is_empty(),
        "a called workflow uploading a build artifact name must fail"
    );
}
