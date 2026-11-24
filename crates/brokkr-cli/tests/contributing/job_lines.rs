//! The jobs behind the twelve checks, held line for line (#340): a skipped
//! step or job reports success, which satisfies a required check, so every
//! line of each required job, and of each job it needs, is held here, and a
//! changed one fails until the guide's row is re-read.

use super::{needs_of, workspace, MAIN_REQUIRES};
use crate::workflow;

/// Every line of each job behind the twelve checks, in `MAIN_REQUIRES`'s
/// order, each job a required job `needs:`, and each job that one needs,
/// held once, before the first job that needs it, as its
/// workflow writes it, but for comments, blank lines and the
/// value of a `run:` or `command:` key, whose lines `LEG_LINES` holds. A
/// skipped step or job reports success, which satisfies a required check,
/// and a runner, an action, an input or a variable changed alters what a
/// check proves while its commands stand, so each is held word for word:
/// a `continue-on-error`, `if:`, `needs:`, `runs-on:`, `uses:`, `with:` or
/// `env:` line added, dropped or changed fails the test. A line changed
/// here is a line to re-read in the guide's row and the sections it links.
/// A line that is a key of `SHARED_STEPS` stands for that step's lines.
const JOB_LINES: [(&str, &str); 16] = [
    (
        "delivered-by-brokkr",
        r#"
    name: delivered by brokkr
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    permissions:
      contents: read
      pull-requests: read
    steps:
<checkout>
          ref: ${{ github.event.pull_request.base.sha }}
          path: base-verifier
<checkout>
          repository: ${{ github.event.pull_request.head.repo.full_name }}
          ref: ${{ github.event.pull_request.head.sha }}
          fetch-depth: 0
          path: pr
      - name: the base branch, for the merge-base
        env:
          BASE_REF: ${{ github.event.pull_request.base.ref }}
          BASE_REPO: https://github.com/${{ github.repository }}.git
        run:
<stable toolchain>
      - name: build the base branch's offline verifier
        run:
      - name: the tier is cut by the delta since the judgment (decisions 0033, 0038)
        env:
          PR_BODY: ${{ github.event.pull_request.body }}
          PR_HEAD: ${{ github.event.pull_request.head.sha }}
          PR_BASE: refs/remotes/base/${{ github.event.pull_request.base.ref }}
          REPO: pr
          EVIDENCE: https://github.com/${{ github.event.pull_request.head.repo.full_name }}.git
          VERIFIER: base-verifier/target/debug/brokkr
          CLASSES: base-verifier/.github/delivery-classes.json
          LABELS: ${{ join(github.event.pull_request.labels.*.name, ',') }}
        run:
"#,
    ),
    (
        "ratchets",
        r#"
    name: baseline ratchets
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    permissions:
      contents: read
    steps:
<checkout>
          fetch-depth: 2
<stable toolchain>
      - uses: ./.github/actions/setup-jscpd
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-shear@1.14.0
          fallback: none
      - name: file-size ratchet (quality/file-lines.txt)
        run:
      - name: duplication ratchet (quality/jscpd-baseline-*.json)
        run:
      - name: no unused dependency (cargo-shear)
        run:
      - name: a raised baseline names its ruling (quality/)
        if: github.event_name == 'pull_request'
        env:
          PR_BODY: ${{ github.event.pull_request.body }}
        run:
"#,
    ),
    (
        "msrv",
        r#"
    name: MSRV (1.89)
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    steps:
<checkout>
      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master
        with:
          toolchain: 1.89.0 # the MSRV, Cargo.toml's rust-version: moved by hand, never by Renovate
      - run:
"#,
    ),
    (
        "quality",
        r#"
    name: format, clippy, contracts
<local build runner>
    timeout-minutes: 20
    steps:
<local checkout>
<stable toolchain>
          components: rustfmt, clippy
<rust-cache>
        if: runner.environment != 'self-hosted'
      - name: formatting is canonical
        run:
      - name: clippy is warning-free across every target
        run:
      - name: an added suppression names a ruling
        if: github.event_name == 'pull_request'
        env:
          BROKKR_SUPPRESSION_BASE: ${{ github.event.pull_request.base.sha }}
        run:
      - name: frozen and additive contracts compile
        run:
"#,
    ),
    (
        "classify",
        r#"
    name: docs lane or full (scripts/docs-lane.sh)
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    outputs:
      lane: ${{ steps.lane.outputs.lane }}
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        if: github.event_name == 'pull_request'
        with:
          fetch-depth: 0
          persist-credentials: false
      - name: a pull request's change, documentation alone or not
        id: lane
        env:
          EVENT: ${{ github.event_name }}
          BASE: ${{ github.event.pull_request.base.sha }}
          HEAD: ${{ github.event.pull_request.head.sha }}
        run:
"#,
    ),
    (
        "docs-lane",
        r#"
    name: docs lane (the tests that read documentation)
    needs: classify
    if: needs.classify.outputs.lane == 'docs'
    runs-on: ubuntu-24.04
    timeout-minutes: 30
    steps:
<checkout>
<stable toolchain>
<rust-cache>
      - name: bubblewrap for the boxed samples
        uses: ./.github/actions/setup-bubblewrap
        with:
          cache-build: 'true'
      - name: the tests that read documentation (scripts/docs-lane-tests.txt)
        run:
        env:
          BROKKR_REQUIRE_BOUNDARY_EVIDENCE: '1'
          BROKKR_REQUIRE_PIDNS: '1'
"#,
    ),
    (
        "engine-machines",
        r#"
    name: test (${{ matrix.os }}) on ${{ matrix.machine }}
    needs: classify
    if: github.event_name != 'push' && needs.classify.outputs.lane != 'docs'
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            machine: linux/1
          - os: ubuntu-latest
            machine: linux/2
          - os: ubuntu-latest
            machine: linux/3
          - os: ubuntu-latest
            machine: linux/4
          - os: ubuntu-latest
            machine: linux/5
          - os: ubuntu-latest
            machine: linux/6
          - os: ubuntu-latest
            machine: linux/7
          - os: ubuntu-latest
            machine: linux/8
          - os: macos-latest
            machine: macos/1
          - os: macos-latest
            machine: macos/2
          - os: macos-latest
            machine: macos/3
          - os: macos-latest
            machine: macos/4
          - os: macos-latest
            machine: macos/5
    runs-on: ${{ fromJSON(matrix.os == 'ubuntu-latest' && vars.BROKKR_LOCAL_CI == 'true' && github.event_name != 'merge_group' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository) && '["self-hosted", "Linux", "X64", "brokkr-engine"]' || format('["{0}"]', matrix.os == 'ubuntu-latest' && 'ubuntu-24.04' || matrix.os)) }}
    timeout-minutes: ${{ matrix.os == 'macos-latest' && 60 || 45 }}
    steps:
<local checkout>
<stable toolchain>
<rust-cache>
        if: runner.environment != 'self-hosted'
      - name: bubblewrap for the hands tests
        id: bubblewrap
        if: runner.os == 'Linux' && runner.environment != 'self-hosted'
        uses: ./.github/actions/setup-bubblewrap
        with:
          protect-system-tree: 'background'
          cache-build: 'true'
      - run:
        env:
          BROKKR_TEST_MACHINE: ${{ matrix.machine }}
          BROKKR_SYSTEM_SET: ${{ steps.bubblewrap.outputs.system-set }}
          BROKKR_REQUIRE_BOUNDARY_EVIDENCE: ${{ runner.os == 'Linux' && '1' || '' }}
          BROKKR_REQUIRE_PIDNS: ${{ runner.os == 'Linux' && '1' || '' }}
      - name: R3 native startup gate
        if: runner.os == 'macOS' && matrix.machine == 'macos/1'
        run:
      - name: R3 native diagnostics
        if: always() && runner.os == 'macOS' && matrix.machine == 'macos/1'
        uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        with:
          name: r3-native-diagnostics
          path: |
            target/r3-startup.log
            target/r3-startup-report.txt
          if-no-files-found: warn
      - name: self and verify bundles compile under the constitutional lint
        run:
"#,
    ),
    (
        "engine",
        r#"
    name: test (${{ matrix.os }})
    needs: [classify, docs-lane, engine-machines]
    if: always() && github.event_name != 'push'
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - name: the docs lane passed on a docs change, and every machine on any other
        env:
          CLASSIFIED: ${{ needs.classify.result }}
          LANE: ${{ needs.classify.outputs.lane }}
          DOCS: ${{ needs.docs-lane.result }}
          MACHINES: ${{ needs.engine-machines.result }}
        run:
"#,
    ),
    (
        "coverage-machines",
        r#"
    name: exact coverage on ${{ matrix.machine }}
    needs: classify
    if: needs.classify.outputs.lane != 'docs'
    strategy:
      fail-fast: false
      matrix:
        machine: [linux/1, linux/2, linux/3, linux/4, linux/5, linux/6, linux/7, linux/8]
    runs-on: ${{ fromJSON(vars.BROKKR_LOCAL_CI == 'true' && github.event_name != 'merge_group' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository) && '["self-hosted", "Linux", "X64", "brokkr-coverage"]' || '["ubuntu-24.04"]') }}
    timeout-minutes: 45
    steps:
<local checkout>
<coverage toolchain>
      - name: bubblewrap for the hands tests
        id: bubblewrap
        if: runner.environment != 'self-hosted'
        uses: ./.github/actions/setup-bubblewrap
        with:
          protect-system-tree: 'background'
          cache-build: 'true'
      - name: measure this machine's shards
        run:
        env:
          BROKKR_TEST_MACHINE: ${{ matrix.machine }}
          BROKKR_COVERAGE_PROFILES: target/coverage-machine
          BROKKR_SYSTEM_SET: ${{ steps.bubblewrap.outputs.system-set }}
          BROKKR_REQUIRE_BOUNDARY_EVIDENCE: '1'
          BROKKR_REQUIRE_PIDNS: '1'
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        with:
          name: coverage-machine-${{ strategy.job-index }}
          path: target/coverage-machine/
          if-no-files-found: error
          retention-days: 1
"#,
    ),
    (
        "coverage",
        r#"
    name: exact coverage report
    needs: coverage-machines
    runs-on: ${{ fromJSON(vars.BROKKR_LOCAL_CI == 'true' && github.event_name != 'merge_group' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository) && '["self-hosted", "Linux", "X64", "brokkr-coverage"]' || '["ubuntu-24.04"]') }}
    timeout-minutes: 30
    steps:
<local checkout>
<coverage toolchain>
      - uses: actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0
        with:
          pattern: coverage-machine-*
          path: target/coverage-machines
      - name: prove literal nonzero 100% source-line/branch/function coverage
        run:
        env:
          BROKKR_COVERAGE_PROFILES: target/coverage-machines
          BROKKR_REQUIRE_BOUNDARY_EVIDENCE: '1'
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-crap@0.5.0
          fallback: none
      - name: complexity ratchet (quality/crap-baseline.json)
        run:
      - name: cargo-public-api 0.52.0, pinned
        run:
      - name: public-API ratchet (quality/public-api/)
        run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        if: always()
        with:
          name: coverage-exact
          path: target/coverage/
"#,
    ),
    (
        "coverage-gate",
        r#"
    name: exact coverage gate
    needs: [classify, docs-lane, coverage]
    if: always()
    runs-on: ubuntu-24.04
    timeout-minutes: 5
    steps:
      - name: the docs lane passed on a docs change, and the report on any other
        env:
          CLASSIFIED: ${{ needs.classify.result }}
          LANE: ${{ needs.classify.outputs.lane }}
          DOCS: ${{ needs.docs-lane.result }}
          REPORT: ${{ needs.coverage.result }}
        run:
"#,
    ),
    (
        "license-compliance",
        r#"
    name: dependency licenses (cargo-deny)
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    permissions:
      contents: read
    steps:
<checkout>
      - uses: EmbarkStudios/cargo-deny-action@3c6349835b2b7b196a839186cb8b78e02f7b5f25 # v2.1.1
        with:
          command:
"#,
    ),
    (
        "lint-non-rust",
        r#"
    name: non-Rust lints
    runs-on: ubuntu-24.04
    timeout-minutes: 15
    steps:
<checkout>
      - name: actionlint, pinned by digest
        uses: ./.github/actions/setup-actionlint
      - name: lychee, pinned by digest
        uses: ./.github/actions/setup-lychee
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: shellcheck@0.11.0,typos@1.50.2,zizmor@1.30.1
          fallback: none
      - name: the offline lints (scripts/lint-non-rust.sh)
        run:
      - uses: actions/setup-node@49933ea5288caeca8642d1e84afbd3f7d6820020 # v4.4.0
        with:
          node-version: 22.23.3
      - name: the diagrams render (mermaid-cli, from .github/lint's lockfile)
        env:
          PUPPETEER_SKIP_DOWNLOAD: '1'
        run:
      - name: Renovate's configuration is valid (renovate-config-validator, the pinned image)
        run:
"#,
    ),
    (
        "dependency-audit",
        r#"
    name: RustSec dependency audit
    runs-on: ubuntu-24.04
    timeout-minutes: 20
    permissions:
      contents: read
    steps:
<checkout>
      - name: cargo-audit, pinned by digest
        uses: ./.github/actions/setup-cargo-audit
      - name: RustSec advisory audit
        run:
"#,
    ),
    (
        "release-binary",
        r#"
    name: release binary artifact
<local build runner>
    timeout-minutes: 20
    steps:
<local checkout>
<stable toolchain>
<rust-cache>
        if: runner.environment != 'self-hosted'
      - run:
      - run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        with:
          name: brokkr-linux-x86_64
          path: target/release/brokkr
"#,
    ),
    (
        "core-gate",
        r#"
    name: 'mutants in the diff: brokkr-core'
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-24.04
    timeout-minutes: 90
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          fetch-depth: 0
          persist-credentials: false
<stable toolchain>
<rust-cache>
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-mutants@27.1.0
          fallback: none
      - name: no miss this pull request adds to brokkr-core
        env:
          BASE: ${{ github.event.pull_request.base.sha }}
          MUTANTS_JOBS: '2'
        run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        if: always()
        with:
          name: mutants-in-diff-brokkr-core
          path: target/mutants/mutants.out/
"#,
    ),
];

/// The `key: value` pairs written at `indent` among `lines`, comments
/// and deeper lines skipped.
fn keys_at<'a>(lines: &[&'a str], indent: &str) -> Vec<(&'a str, &'a str)> {
    lines
        .iter()
        .filter_map(|line| line.strip_prefix(indent))
        .filter(|line| !line.starts_with([' ', '#']) && !line.is_empty())
        .map(|line| {
            let (key, value) = line.split_once(':').unwrap_or((line, ""));
            (key, value.trim())
        })
        .collect()
}

/// A workflow's top-level keys, in order: a workflow-wide `defaults:` or
/// `env:` would reach every step.
fn top_level_keys(workflow: &str) -> Vec<&str> {
    keys_at(&workflow.lines().collect::<Vec<_>>(), "")
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

/// The steps several required jobs take word for word, held once.
const SHARED_STEPS: [(&str, &str); 6] = [
    (
        "<local build runner>",
        r#"    runs-on: ${{ fromJSON(vars.BROKKR_LOCAL_CI == 'true' && github.event_name != 'merge_group' && (github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name == github.repository) && '["self-hosted", "Linux", "X64", "brokkr-build"]' || '["ubuntu-24.04"]') }}"#,
    ),
    (
        "<local checkout>",
        "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          persist-credentials: false
          clean: ${{ runner.environment != 'self-hosted' }}
      - uses: ./.github/actions/local-runner
        if: runner.environment == 'self-hosted'",
    ),
    (
        "<checkout>",
        "      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          persist-credentials: false",
    ),
    (
        "<stable toolchain>",
        "      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master
        with:
          toolchain: 1.98.0",
    ),
    (
        "<rust-cache>",
        "      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2",
    ),
    (
        "<coverage toolchain>",
        r#"      - name: the pinned coverage toolchain and its measuring tool
        id: nightly
        run:
      - uses: dtolnay/rust-toolchain@02cb101ec7c40f2c49e1d9714d64511d8e1b74de # master
        with:
          toolchain: ${{ steps.nightly.outputs.toolchain }}
          components: llvm-tools-preview, rustfmt, clippy
      - uses: taiki-e/install-action@9983c65e42da123ff25d1f78505eb6de315aa172 # v2.87.20
        with:
          tool: cargo-llvm-cov@${{ steps.nightly.outputs.cargo_llvm_cov }}
          fallback: none
      - uses: Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6 # v2.9.2
        if: runner.environment != 'self-hosted'
      - uses: actions/cache@0057852bfaa89a56745cba8c7296529d2fc39830 # v4.3.0
        if: runner.environment != 'self-hosted'
        with:
          path: ~/.cache/brokkr-coverage-cache
          key: brokkr-coverage-${{ runner.os }}-${{ hashFiles('rust-nightly-version.txt', 'cargo-llvm-cov-version.txt', 'Cargo.lock', 'Cargo.toml', 'crates/*/Cargo.toml') }}"#,
    ),
];

/// The lines `JOB_LINES` holds for one job, each shared step's name
/// replaced by its lines.
fn expected_lines(held: &str) -> Vec<&str> {
    held.lines()
        .skip(1)
        .flat_map(|line| {
            SHARED_STEPS
                .iter()
                .find(|(name, _)| *name == line)
                .map_or_else(|| vec![line], |(_, step)| step.lines().collect())
        })
        .collect()
}

/// A job's lines as `JOB_LINES` holds them: comments and blank lines
/// dropped, and a `run:` or `command:` key kept without its value, the
/// block under it included.
fn held_lines(body: &str) -> Vec<&str> {
    let indent = |line: &str| line.len() - line.trim_start().len();
    let mut held = Vec::new();
    let mut value_under = None;
    for line in body.lines() {
        if value_under.is_some_and(|key| line.trim().is_empty() || indent(line) > key) {
            continue;
        }
        value_under = None;
        let key = line.trim_start().trim_start_matches("- ");
        if key.is_empty() || key.starts_with('#') {
            continue;
        }
        let at = line.len() - key.len();
        match ["run:", "command:"]
            .into_iter()
            .find(|name| key.starts_with(name))
        {
            Some(name) => {
                held.push(&line[..at + name.len()]);
                value_under = Some(at);
            }
            None => held.push(line),
        }
    }
    held
}

/// Every line of the nightly CPU budgets' one job (#1295), held as
/// `JOB_LINES` holds a required job's: it is no required check, but its
/// red status is the only signal a regression raises, so a softened step,
/// a write permission or another runner fails until it is re-read.
const CPU_BUDGETS_LINES: &str = r#"
    name: CPU budgets (Callgrind, against the base)
    runs-on: ubuntu-24.04
    timeout-minutes: 90
    permissions:
      contents: read
      actions: read
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0
        with:
          ref: ${{ inputs.ref }}
          fetch-depth: 0
          persist-credentials: false
<stable toolchain>
<rust-cache>
      - name: valgrind, and the gungraun runner at the library's version
        run:
      - name: the base, the previous nightly's head
        env:
          GH_TOKEN: ${{ github.token }}
          REPO: ${{ github.repository }}
          REF: ${{ inputs.ref }}
          COUNTS: "the head's counts, 2% over the base at most"
        run:
      - name: the head's benches, built
        run:
      - name: the base's counts
        run:
      - name: the head's counts, 2% over the base at most
        run:
      - uses: actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4.6.2
        if: always()
        with:
          name: cpu-budgets
          path: |
            ${{ runner.temp }}/record/
            ${{ runner.temp }}/gungraun/
            target/gungraun/
            target/cargo-timings/
          if-no-files-found: ignore
"#;

/// The nightly CPU budgets (#1295) are held whole like the required jobs:
/// cpu-budgets.yml's top-level keys, its one job, and every line of that
/// job as `CPU_BUDGETS_LINES` holds it.
#[test]
fn the_nightly_cpu_budgets_run_every_step_unsoftened() {
    let text =
        std::fs::read_to_string(workspace().join(".github/workflows/cpu-budgets.yml")).unwrap();
    assert_eq!(
        top_level_keys(&text),
        ["name", "on", "permissions", "concurrency", "jobs"],
        "cpu-budgets.yml's top-level keys"
    );
    let jobs = workflow::jobs(&text);
    let ids: Vec<&str> = jobs.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, ["cpu-budgets"], "cpu-budgets.yml's jobs");
    assert_eq!(
        held_lines(&jobs[0].1),
        expected_lines(CPU_BUDGETS_LINES),
        "a line of cpu-budgets.yml's job changed: re-read what its red status signals"
    );
}

/// A skipped step or job reports success, which satisfies a required
/// check, so the jobs behind the twelve are held whole: each workflow's
/// top-level keys (a workflow-wide `defaults:` or `env:` would reach every
/// step), and every line of each required job and of each job it needs,
/// whose result it reports, as `JOB_LINES` holds it.
#[test]
fn the_required_jobs_run_every_step_unsoftened() {
    let root = workspace();
    let read =
        |file: &str| std::fs::read_to_string(root.join(".github/workflows").join(file)).unwrap();
    for (file, top) in [
        (
            "pr-gates.yml",
            ["name", "on", "concurrency", "permissions", "jobs"],
        ),
        (
            "ci.yml",
            ["name", "on", "concurrency", "permissions", "jobs"],
        ),
        (
            "mutants.yml",
            ["name", "on", "permissions", "concurrency", "jobs"],
        ),
    ] {
        assert_eq!(top_level_keys(&read(file)), top, "{file}'s top-level keys");
    }
    let body_of = |id: &str, file: &str| {
        workflow::jobs(&read(file))
            .into_iter()
            .find_map(|(job, body)| (job == id).then_some(body))
            .unwrap_or_else(|| panic!("{file} has no {id} job"))
    };
    // Each required job after every job it needs, and each of those after
    // its own, each once: a verdict judges what its needs ran (#1295).
    fn hold<'f>(
        id: &str,
        file: &'f str,
        body_of: &dyn Fn(&str, &str) -> String,
        required: &mut Vec<(String, &'f str)>,
    ) {
        if required.iter().any(|(held, _)| held == id) {
            return;
        }
        for needed in needs_of(&body_of(id, file), id) {
            hold(&needed, file, body_of, required);
        }
        required.push((id.to_string(), file));
    }
    let mut required: Vec<(String, &str)> = Vec::new();
    for &(_, id, file) in &MAIN_REQUIRES {
        hold(id, file, &body_of, &mut required);
    }
    let held: Vec<&str> = JOB_LINES.iter().map(|(id, _)| *id).collect();
    let ids: Vec<&str> = required.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        held, ids,
        "JOB_LINES holds each required job and each job it needs once, in order"
    );
    for ((id, file), (_, lines)) in required.into_iter().zip(JOB_LINES) {
        let body = body_of(&id, file);
        assert_eq!(
            held_lines(&body),
            expected_lines(lines),
            "a line of {file}'s {id} changed: re-read its row and the sections the row links"
        );
    }
}
