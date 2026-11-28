//! The scripts the non-Rust lints and Renovate run (issue #339), each run
//! against a scratch repository with stubs first on `PATH`, so what each one
//! refuses is bound by a test:
//!
//! - `refresh-workflow-digests.sh` (#1417): rewrites or checks
//!   `.github/workflow-digests.txt` over `.github/actions/`,
//!   `.github/workflows/`, `.github/renovate.json5`, and
//!   `.github/renovate-global.json5`, agrees with `workflow_digests::check_workflow_digests`
//!   including nested action directories, refuses invalid path names, symlinks,
//!   and non-regular digest targets (including FIFOs without blocking), writes
//!   atomically via a same-directory temp file, supports `shasum -a 256`
//!   fallback without host hashers, and fails closed when neither hasher exists;
//! - `refresh-pin-checksums.sh`, Renovate's post-upgrade task: a digest
//!   moves only with its own version, an unchanged version whose release no
//!   longer matches is refused, and nothing is written unless every release
//!   was measured and at least one version moved;
//! - `shellcheck-actions.sh`: every composite action's `run: |` script is
//!   extracted and checked, and a `run:` (or `run :`) it cannot read is
//!   refused;
//! - `lint-diagrams.sh`: every mermaid fence Markdown allows is found, and a
//!   file whose fences mermaid-cli does not all render is refused;
//! - `lint-non-rust.sh` (#427): the offline lints run cheapest first, each
//!   tool at its CI pin; CI refuses a tool it cannot run, a verify seat
//!   names it; and `recipes/brokkr-fast`'s verify seat, the one a landing
//!   inherits, fails on a lint or on clippy with the command named.

use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// A scratch git repository with a `stub-bin/` put first on `PATH`.
struct Repo {
    root: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let repo = Self {
            root: tempfile::tempdir().expect("scratch repository"),
        };
        repo.git(&["init", "-q"]);
        repo
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn put(&self, relative: &str, text: &str) {
        let file = self.path().join(relative);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("mkdir");
        std::fs::write(file, text).expect(relative);
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.path().join(relative)).expect(relative)
    }

    /// An executable at `relative`, a stub standing in for a tool.
    fn executable(&self, relative: &str, text: &str) {
        self.put(relative, text);
        let file = self.path().join(relative);
        let mut mode = std::fs::metadata(&file).expect("stub").permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(file, mode).expect("stub mode");
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(self.path())
            .args([
                "-c",
                "user.name=scripts",
                "-c",
                "user.email=scripts@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?}");
    }

    /// The workspace's `script` run here, `stub-bin/` first on `PATH`.
    fn run(&self, script: &str, args: &[&str], env: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.path().join("stub-bin").display(),
            std::env::var("PATH").expect("PATH")
        );
        self.run_on(&path, script, args, env)
    }

    /// Links each named tool from the host `PATH` into `sys-bin/`, skipping
    /// any tool already linked there.
    fn link_host_tools(&self, tools: &[&str]) {
        let system = self.path().join("sys-bin");
        std::fs::create_dir_all(&system).expect("sys-bin");
        let host = std::env::var_os("PATH").expect("PATH");
        for tool in tools {
            let target = system.join(tool);
            if target.symlink_metadata().is_ok() {
                continue;
            }
            let found = std::env::split_paths(&host)
                .map(|directory| directory.join(tool))
                .find(|candidate| candidate.is_file())
                .unwrap_or_else(|| panic!("{tool} is not on the host PATH"));
            std::os::unix::fs::symlink(found, target).expect(tool);
        }
    }

    /// Links each of the [`SYSTEM_TOOLS`] from the host `PATH` into
    /// `sys-bin/`.
    fn link_system_tools(&self) {
        self.link_host_tools(&SYSTEM_TOOLS);
    }

    /// The workspace's `script` run here with only `stub-bin/` and
    /// `sys-bin/` on `PATH`, so a lint tool the host has installed cannot
    /// stand in for a stub, and one left unstubbed is missing.
    fn run_hermetic(&self, script: &str, args: &[&str], env: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.path().join("stub-bin").display(),
            self.path().join("sys-bin").display()
        );
        self.run_on(&path, script, args, env)
    }

    fn run_on(&self, path: &str, script: &str, args: &[&str], env: &[(&str, &str)]) -> Output {
        Command::new("bash")
            .arg(workspace().join(script))
            .args(args)
            .current_dir(self.path())
            .env("PATH", path)
            .env_remove("STUB_SWAP")
            .env_remove("STUB_FAIL")
            .envs(env.iter().copied())
            .output()
            .expect("bash")
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The stub writes `release <url>` to the `-o` file, `swapped` appended when
/// the URL contains `$STUB_SWAP`, and fails as curl does when it contains
/// `$STUB_FAIL`.
const STUB_CURL: &str = r#"#!/usr/bin/env bash
out="" url=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
if [ -n "${STUB_FAIL:-}" ] && [[ "$url" == *"$STUB_FAIL"* ]]; then exit 22; fi
body="release $url"
if [ -n "${STUB_SWAP:-}" ] && [[ "$url" == *"$STUB_SWAP"* ]]; then body="$body swapped"; fi
printf '%s' "$body" > "$out"
"#;

fn url(tool: &str, version: &str) -> String {
    format!("https://example.invalid/{tool}/v{version}/{tool}.tar.gz")
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest(tool: &str, version: &str) -> String {
    sha256_hex(&format!("release {}", url(tool, version)))
}

/// An action in the shape the refresh script reads, recording `sha256`.
fn action(tool: &str, version: &str, sha256: &str) -> String {
    let key = tool.to_ascii_uppercase();
    format!(
        "runs:
  using: composite
  steps:
    - name: {tool}, pinned by version and digest
      shell: bash
      env:
        {key}_VERSION: {version}
        {key}_SHA256: {sha256}
      run: |
        set -euo pipefail
        tarball=\"${{RUNNER_TEMP}}/{tool}.tar.gz\"
        curl -sSfL --connect-timeout 20 --max-time 60 --retry 6 --retry-delay 10 --retry-all-errors -o \"$tarball\" \\
          \"https://example.invalid/{tool}/v${{{key}_VERSION}}/{tool}.tar.gz\"
        echo \"${{{key}_SHA256}}  ${{tarball}}\" | sha256sum -c -
"
    )
}

/// A repository whose HEAD pins alpha 1.0.0 and beta 2.0.0, each at the
/// digest its release really has, with the stub curl.
struct Scratch {
    repo: Repo,
}

impl Scratch {
    fn new() -> Self {
        let scratch = Self { repo: Repo::new() };
        scratch.write("alpha", "1.0.0", &digest("alpha", "1.0.0"));
        scratch.write("beta", "2.0.0", &digest("beta", "2.0.0"));
        scratch.repo.executable("stub-bin/curl", STUB_CURL);
        scratch.repo.git(&["add", ".github"]);
        scratch.repo.git(&["commit", "-q", "-m", "pins"]);
        scratch
    }

    fn file(tool: &str) -> String {
        format!(".github/actions/setup-{tool}/action.yml")
    }

    fn write(&self, tool: &str, version: &str, sha256: &str) {
        self.repo
            .put(&Self::file(tool), &action(tool, version, sha256));
    }

    fn read(&self, tool: &str) -> String {
        self.repo.read(&Self::file(tool))
    }

    fn refresh(&self, stub: &[(&str, &str)]) -> Output {
        self.repo.run("scripts/refresh-pin-checksums.sh", &[], stub)
    }
}

#[test]
fn a_moved_version_rewrites_its_own_digest_and_no_other() {
    let scratch = Scratch::new();
    let beta = scratch.read("beta");
    scratch.write("alpha", "1.1.0", &digest("alpha", "1.0.0"));

    let output = scratch.refresh(&[]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        scratch.read("alpha"),
        action("alpha", "1.1.0", &digest("alpha", "1.1.0"))
    );
    assert_eq!(
        scratch.read("beta"),
        beta,
        "an unmoved digest was rewritten"
    );
    assert_eq!(
        stdout(&output),
        format!(
            "refresh-pin-checksums: .github/actions/setup-alpha/action.yml {}\n",
            digest("alpha", "1.1.0")
        )
    );
}

#[test]
fn an_unmoved_release_that_no_longer_matches_is_refused_and_nothing_is_written() {
    let scratch = Scratch::new();
    scratch.write("alpha", "1.1.0", &digest("alpha", "1.0.0"));
    let alpha = scratch.read("alpha");

    let output = scratch.refresh(&[("STUB_SWAP", "/beta/")]);

    assert_eq!(output.status.code(), Some(1));
    let swapped = sha256_hex(&format!("release {} swapped", url("beta", "2.0.0")));
    assert_eq!(
        stderr(&output),
        format!(
            "refresh-pin-checksums: .github/actions/setup-beta/action.yml: BETA_VERSION 2.0.0 did not move, yet its release now hashes to {swapped}, not the recorded {}; refusing to re-pin it\n",
            digest("beta", "2.0.0")
        )
    );
    // alpha was measured first, and still nothing was written.
    assert_eq!(scratch.read("alpha"), alpha);
}

#[test]
fn nothing_moved_or_a_failed_download_writes_nothing() {
    let scratch = Scratch::new();
    let output = scratch.refresh(&[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "refresh-pin-checksums: no pinned version moved against HEAD, so there is no digest to refresh\n"
    );

    scratch.write("alpha", "1.1.0", &digest("alpha", "1.0.0"));
    let alpha = scratch.read("alpha");
    let output = scratch.refresh(&[("STUB_FAIL", "/beta/")]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        format!(
            "refresh-pin-checksums: .github/actions/setup-beta/action.yml: could not download {}\n",
            url("beta", "2.0.0")
        )
    );
    assert_eq!(scratch.read("alpha"), alpha);
}

/// Stands in for shellcheck: appends each script it is handed, by file
/// name, to `$STUB_OUT`, and passes.
const STUB_SHELLCHECK: &str = r#"#!/usr/bin/env bash
for arg in "$@"; do
  case "$arg" in *.sh) { printf '== %s\n' "${arg##*/}"; cat "$arg"; } >> "$STUB_OUT" ;; esac
done
"#;

/// A repository holding `actions` as `.github/actions/<name>/action.yml`,
/// staged, with the stub shellcheck.
fn action_tree(actions: &[(&str, &str)]) -> Repo {
    let repo = Repo::new();
    for (name, text) in actions {
        repo.put(&format!(".github/actions/{name}/action.yml"), text);
    }
    repo.executable("stub-bin/shellcheck", STUB_SHELLCHECK);
    repo.git(&["add", ".github"]);
    repo
}

fn shellcheck_actions(repo: &Repo) -> Output {
    let checked = repo.path().join("checked.txt");
    let checked = checked.to_str().expect("a UTF-8 path");
    repo.run(
        "scripts/shellcheck-actions.sh",
        &[],
        &[("STUB_OUT", checked)],
    )
}

#[test]
fn every_run_block_of_every_action_is_extracted_and_checked() {
    let repo = action_tree(&[
        (
            "one",
            "runs:
  using: composite
  steps:
    - name: first
      shell: bash
      run: |
        echo one
    - shell: bash
      run: |
        if true; then
          echo two
        fi
",
        ),
        (
            "two",
            "runs:
  using: composite
  steps:
    - uses: actions/checkout@v4
    - run: |
        echo three
      shell: bash
    - shell: bash
      run: |
       echo four
",
        ),
    ]);

    let output = shellcheck_actions(&repo);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "shellcheck-actions: 4 action scripts clean\n"
    );
    assert_eq!(
        repo.read("checked.txt"),
        "\
== 0-1.sh
#!/usr/bin/env bash
echo one
== 0-2.sh
#!/usr/bin/env bash
if true; then
  echo two
fi
== 1-1.sh
#!/usr/bin/env bash
echo three
== 1-2.sh
#!/usr/bin/env bash
echo four
"
    );
}

#[test]
fn a_run_that_is_not_a_literal_block_or_no_script_at_all_is_refused() {
    for header in [
        "run: >-",
        "run: |-",
        "run: |2",
        "run: echo hi",
        "run : |",
        "run : echo hi",
    ] {
        let repo = action_tree(&[(
            "folded",
            &format!(
                "runs:
  using: composite
  steps:
    - shell: bash
      {header}
        echo folded
"
            ),
        )]);
        let output = shellcheck_actions(&repo);
        assert_eq!(output.status.code(), Some(1), "{header}");
        assert_eq!(
            stderr(&output),
            format!(
                "shellcheck-actions: .github/actions/folded/action.yml:5: not a `run: |` block, so it is not read: {header}\n"
            )
        );
    }

    let repo = action_tree(&[(
        "none",
        "runs:
  using: composite
  steps:
    - uses: actions/checkout@v4
",
    )]);
    let output = shellcheck_actions(&repo);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "shellcheck-actions: no run: | script found in .github/actions\n"
    );
}

/// Stands in for mermaid-cli 12.0.0 as measured on the host: it renders a
/// ```mermaid fence, indented or not, one numbered SVG beside the output,
/// and silently skips a ~~~mermaid fence.
const STUB_MMDC: &str = r#"#!/usr/bin/env bash
in="" out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -i) in="$2"; shift 2 ;;
    -o) out="$2"; shift 2 ;;
    *) shift ;;
  esac
done
pattern='^ {0,3}```mermaid'
n=0
while IFS= read -r line; do
  if [[ "$line" =~ $pattern ]]; then n=$((n + 1)); : > "${out%.md}-$n.svg"; fi
done < "$in"
cp "$in" "$out"
"#;

/// A repository holding `documents`, staged, with the stub renderer where
/// the lint job installs mermaid-cli.
fn documents(documents: &[(&str, &str)]) -> Repo {
    let repo = Repo::new();
    for (name, text) in documents {
        repo.put(name, text);
        repo.git(&["add", name]);
    }
    repo.executable(".github/lint/node_modules/.bin/mmdc", STUB_MMDC);
    repo
}

const DIAGRAM: &str = "flowchart LR\n  a --> b\n";

#[test]
fn every_mermaid_fence_is_found_and_each_file_renders_all_of_its_diagrams() {
    let repo = documents(&[
        (
            "README.md",
            &format!("# r\n\n```mermaid\n{DIAGRAM}```\n\n   ```mermaid\n{DIAGRAM}   ```\n"),
        ),
        (
            "indented.md",
            &format!("# i\n\n  ```mermaid\n{DIAGRAM}  ```\n"),
        ),
        ("plain.md", "# no diagram\n"),
    ]);
    let output = repo.run("scripts/lint-diagrams.sh", &[], &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "lint-diagrams: README.md renders\nlint-diagrams: indented.md renders\n"
    );
}

#[test]
fn a_fence_the_renderer_skips_or_no_diagram_at_all_is_refused() {
    let repo = documents(&[(
        "tilde.md",
        &format!("# t\n\n```mermaid\n{DIAGRAM}```\n\n~~~mermaid\n{DIAGRAM}~~~\n"),
    )]);
    let output = repo.run("scripts/lint-diagrams.sh", &[], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "lint-diagrams: tilde.md renders 1 of its 2 mermaid diagrams; open each with ```mermaid\n"
    );

    let repo = documents(&[("plain.md", "# no diagram\n")]);
    let output = repo.run("scripts/lint-diagrams.sh", &[], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "lint-diagrams: no mermaid diagram found in the tracked Markdown\n"
    );
}

/// What `lint-non-rust.sh` and fast's verify seat need on `PATH` beside
/// the stubs.
const SYSTEM_TOOLS: [&str; 14] = [
    "awk", "bash", "dirname", "git", "grep", "head", "mkdir", "rm", "sed", "sort", "tail", "tr",
    "wc", "xargs",
];

/// The version a lint fixture's CI installs each tool at: actionlint and
/// lychee in their setup actions, the others on the install-action line.
const LINT_PINS: [(&str, &str); 5] = [
    ("typos", "2.0.0"),
    ("shellcheck", "1.0.0"),
    ("actionlint", "4.0.0"),
    ("zizmor", "3.0.0"),
    ("lychee", "5.0.0"),
];

/// Stands in for a lint tool at `version`: reports it, appends
/// `<tool>|<arguments>|<SHELLCHECK_OPTS>` to `$STUB_OUT`, and fails as
/// typos does when `$STUB_FAIL` names it.
fn stub_lint(tool: &str, version: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
if [ "$1" = --version ]; then echo '{tool} {version}'; exit 0; fi
printf '%s|%s|%s\n' '{tool}' "$*" "${{SHELLCHECK_OPTS-}}" >> "$STUB_OUT"
if [ "${{STUB_FAIL-}}" = '{tool}' ]; then echo 'error: README.md:1: a misspelled word'; exit 2; fi
"#
    )
}

/// A repository whose CI pins the [`LINT_PINS`], with a stub at its pin
/// for every tool but `missing`, and `scripts/lint-non-rust.sh` copied
/// in, untracked, where the verify seat runs it.
fn lint_tree(missing: &[&str]) -> Repo {
    let repo = Repo::new();
    let installed: Vec<String> = LINT_PINS
        .iter()
        .filter(|(tool, _)| !["actionlint", "lychee"].contains(tool))
        .map(|(tool, version)| format!("{tool}@{version}"))
        .collect();
    repo.put(
        ".github/workflows/ci.yml",
        &format!(
            "jobs:\n  lint:\n    steps:\n      - with:\n          tool: {}\n",
            installed.join(",")
        ),
    );
    for (tool, version) in LINT_PINS {
        if ["actionlint", "lychee"].contains(&tool) {
            let key = tool.to_ascii_uppercase();
            repo.put(
                &format!(".github/actions/setup-{tool}/action.yml"),
                &format!("runs:\n  steps:\n    - env:\n        {key}_VERSION: {version}\n"),
            );
        }
        if !missing.contains(&tool) {
            repo.executable(&format!("stub-bin/{tool}"), &stub_lint(tool, version));
        }
    }
    repo.put(
        "scripts/shellcheck-actions.sh",
        "shellcheck -s bash action-scripts\n",
    );
    repo.put("README.md", "# lints\n");
    repo.git(&["add", ".github", "scripts", "README.md"]);
    repo.put(
        "scripts/lint-non-rust.sh",
        &std::fs::read_to_string(workspace().join("scripts/lint-non-rust.sh")).expect("the list"),
    );
    repo.link_system_tools();
    repo
}

/// `script` run hermetically in `repo` with `STUB_FAIL` set to `fail`,
/// and what the stubs recorded running, from a fresh record.
fn run_lints(repo: &Repo, script: &str, args: &[&str], fail: &str) -> (Output, String) {
    let ran = repo.path().join("ran.txt");
    std::fs::write(&ran, "").expect("a fresh record");
    let ran_path = ran.to_str().expect("a UTF-8 path");
    let output = repo.run_hermetic(script, args, &[("STUB_OUT", ran_path), ("STUB_FAIL", fail)]);
    (output, repo.read("ran.txt"))
}

const ALL_LINTS_RAN: &str = "\
typos|--hidden|
shellcheck|-S warning scripts/shellcheck-actions.sh|
shellcheck|-s bash action-scripts|
actionlint||-S warning
zizmor|--offline .github/actions/setup-actionlint/action.yml .github/actions/setup-lychee/action.yml .github/workflows/ci.yml|
lychee|--offline --include-fragments --no-progress README.md|
";

#[test]
fn every_offline_lint_runs_at_its_pin_cheapest_first() {
    let repo = lint_tree(&[]);
    let (output, ran) = run_lints(&repo, "scripts/lint-non-rust.sh", &[], "");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "lint-non-rust: 6 of 6 lints ran clean\n");
    assert_eq!(ran, ALL_LINTS_RAN);
}

#[test]
fn ci_refuses_a_lint_tool_missing_or_off_its_pin() {
    let repo = lint_tree(&["actionlint"]);
    let (output, ran) = run_lints(&repo, "scripts/lint-non-rust.sh", &[], "");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "lint-non-rust: SHELLCHECK_OPTS='-S warning' actionlint cannot run: actionlint is not on PATH\n"
    );
    assert_eq!(
        ran,
        "typos|--hidden|\nshellcheck|-S warning scripts/shellcheck-actions.sh|\nshellcheck|-s bash action-scripts|\n"
    );

    let repo = lint_tree(&[]);
    repo.executable("stub-bin/typos", &stub_lint("typos", "2.0.1"));
    let (output, ran) = run_lints(&repo, "scripts/lint-non-rust.sh", &[], "");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "lint-non-rust: typos --hidden cannot run: typos on PATH reports typos 2.0.1, not the pinned 2.0.0\n"
    );
    assert_eq!(ran, "");
}

#[test]
fn a_failing_lint_stops_the_list_by_name_and_an_unpinned_tool_is_refused() {
    let repo = lint_tree(&[]);
    let (output, ran) = run_lints(&repo, "scripts/lint-non-rust.sh", &["--seat"], "shellcheck");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "error: README.md:1: a misspelled word\n");
    assert_eq!(
        stderr(&output),
        "lint-non-rust: git ls-files -z '*.sh' | xargs -0 shellcheck -S warning failed\n"
    );
    assert_eq!(
        ran,
        "typos|--hidden|\nshellcheck|-S warning scripts/shellcheck-actions.sh|\n"
    );

    for tools in [
        "shellcheck@1.0.0,typos@2.0.0",
        "shellcheck@1.0.0,typos@2.0.0,zizmor@3.0.0,zizmor@3.0.1",
    ] {
        repo.put(
            ".github/workflows/ci.yml",
            &format!("          tool: {tools}\n"),
        );
        let (output, _) = run_lints(&repo, "scripts/lint-non-rust.sh", &["--seat"], "");
        assert_eq!(output.status.code(), Some(1), "{tools}");
        assert_eq!(
            stderr(&output),
            "lint-non-rust: CI does not pin exactly one version of zizmor\n",
            "{tools}"
        );
    }

    let (output, ran) = run_lints(&repo, "scripts/lint-non-rust.sh", &["--strict"], "");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        "lint-non-rust: unknown argument --strict; the only one is --seat\n"
    );
    assert_eq!(ran, "");
}

/// Stands in for cargo: appends `cargo <arguments>` to `$STUB_OUT`, fails
/// as a compile error does when `$STUB_FAIL` is `cargo <subcommand>`, and
/// reports one clean test suite.
const STUB_CARGO: &str = r#"#!/usr/bin/env bash
printf 'cargo %s\n' "$*" >> "$STUB_OUT"
if [ "${STUB_FAIL-}" = "cargo $1" ]; then echo 'error: could not compile `brokkr-cli` due to 1 previous error'; exit 101; fi
if [ "$1" = test ]; then echo 'test result: ok. 3 passed; 0 failed'; fi
"#;

/// brokkr-fast's verify seat, the one a landing inherits, run in `repo` with
/// `STUB_FAIL` set to `fail`: the result it wrote, and what ran.
fn fast_verify(repo: &Repo, fail: &str) -> (String, String) {
    repo.executable("stub-bin/cargo", STUB_CARGO);
    let result = repo.path().join(".forge/results/verify.json");
    repo.put(
        "prompt.md",
        &format!("Write the result to\n{}\n", result.display()),
    );
    let (output, ran) = run_lints(
        repo,
        "recipes/brokkr-fast/scripts/verify-seat.sh",
        &["prompt.md"],
        fail,
    );
    assert!(output.status.success(), "{}", stderr(&output));
    (repo.read(".forge/results/verify.json"), ran)
}

#[test]
fn a_branch_failing_typos_or_clippy_fails_the_landing_verify_by_name() {
    let repo = lint_tree(&[]);
    let (result, ran) = fast_verify(&repo, "typos");
    assert_eq!(
        result,
        r#"{"result": "fail", "notes": "typos --hidden failed; decisive output follows verbatim:\nerror: README.md:1: a misspelled word\nlint-non-rust: typos --hidden failed"}
"#
    );
    assert_eq!(ran, "cargo fmt --all -- --check\ntypos|--hidden|\n");

    let (result, ran) = fast_verify(&repo, "cargo clippy");
    assert_eq!(
        result,
        r#"{"result": "fail", "notes": "cargo clippy --workspace --all-targets --all-features --locked -- -D warnings failed; decisive output follows verbatim:\nerror: could not compile `brokkr-cli` due to 1 previous error"}
"#
    );
    assert_eq!(
        ran,
        format!(
            "cargo fmt --all -- --check\n{ALL_LINTS_RAN}cargo clippy --workspace --all-targets --all-features --locked -- -D warnings\n"
        )
    );
}

/// Stands in for shellcheck at its [`LINT_PINS`] pin, finding SC2086 in
/// its default format: file, line and code, and no word the verify seat's
/// error grep knows.
const STUB_SHELLCHECK_SC2086: &str = r#"#!/usr/bin/env bash
if [ "$1" = --version ]; then echo 'shellcheck 1.0.0'; exit 0; fi
printf '%s\n' '' 'In scripts/shellcheck-actions.sh line 1:' 'shellcheck -s bash $1' \
  '                   ^-- SC2086 (info): Double quote to prevent globbing and word splitting.' \
  '' 'Did you mean:' 'shellcheck -s bash "$1"' '' 'For more information:' \
  '  https://www.shellcheck.net/wiki/SC2086 -- Double quote to prevent globbing ...'
exit 1
"#;

#[test]
fn a_lint_finding_without_an_error_word_reaches_the_landing_verify_notes() {
    let repo = lint_tree(&[]);
    repo.executable("stub-bin/shellcheck", STUB_SHELLCHECK_SC2086);
    let (result, _) = fast_verify(&repo, "");
    assert_eq!(
        result,
        r#"{"result": "fail", "notes": "git ls-files -z '*.sh' | xargs -0 shellcheck -S warning failed; decisive output follows verbatim:\n\nIn scripts/shellcheck-actions.sh line 1:\nshellcheck -s bash $1\n                   ^-- SC2086 (info): Double quote to prevent globbing and word splitting.\n\nDid you mean:\nshellcheck -s bash \"$1\"\n\nFor more information:\n  https://www.shellcheck.net/wiki/SC2086 -- Double quote to prevent globbing ...\nlint-non-rust: git ls-files -z '*.sh' | xargs -0 shellcheck -S warning failed"}
"#
    );
}

#[test]
fn a_lint_tool_the_box_cannot_reach_is_named_and_the_rest_decide() {
    let repo = lint_tree(&["lychee"]);
    repo.executable("stub-bin/zizmor", &stub_lint("zizmor", "0.1.0"));
    let (result, _) = fast_verify(&repo, "");
    assert_eq!(
        result,
        r#"{"result": "pass", "notes": "cargo fmt --all -- --check: clean; cargo clippy --workspace --all-targets --all-features --locked -- -D warnings: clean\nlint-non-rust: not run: git ls-files -z .github/workflows .github/actions | xargs -0 zizmor --offline (zizmor on PATH reports zizmor 0.1.0, not the pinned 3.0.0)\nlint-non-rust: not run: git ls-files -z '*.md' | xargs -0 lychee --offline --include-fragments --no-progress (lychee is not on PATH)\nlint-non-rust: 4 of 6 lints ran clean\ncargo test --workspace: 1 successful test-suite summaries, 0 failed; cargo run -p brokkr-cli -- library compile --bundle recipes/self: 1 bundle compiled, 0 failed (offline from the bound Cargo registry cache)"}
"#
    );
}

/// A tree for `docs-lane.sh classify` (#1295), committed and tagged `base`:
/// pages under docs/ and at the top, one named by a file the docs lane
/// runs, one compiled into a crate, a script and a workflow.
fn lane_tree() -> Repo {
    let repo = Repo::new();
    for (path, text) in [
        ("docs/guide.md", "# guide\n"),
        ("docs/listed.md", "# listed\n"),
        ("docs/compiled.md", "# compiled\n"),
        ("README.md", "# front page\n"),
        (
            "scripts/docs-lane-tests.txt",
            "# the lane\nbrokkr-cli it doc_truth:: crates/cli/tests/doc_truth.rs\n",
        ),
        (
            "crates/cli/tests/doc_truth.rs",
            "read(\"docs/listed.md\");\n",
        ),
        (
            "crates/cli/src/lib.rs",
            "include_str!(\"../../../docs/compiled.md\");\n",
        ),
        ("scripts/old.sh", "true\n"),
        (".github/workflows/ci.yml", "jobs: {}\n"),
        ("docs/proposed.md", "# proposed\n"),
        ("openspec/plan.md", "See docs/proposed.md.\n"),
        ("docs/roled.md", "# roled\n"),
        ("recipes/r/roles/reviewer.md", "Read docs/roled.md first.\n"),
        ("docs/tested.md", "# tested\n"),
        (
            "crates/cli/tests/unlisted.rs",
            "read(\"docs/tested.md\");\n",
        ),
    ] {
        repo.put(path, text);
    }
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "base"]);
    repo.git(&["tag", "base"]);
    repo
}

/// What `docs-lane.sh classify` prints for `base...head` in `repo`, and the
/// reason it gives on stderr.
fn lane_of(repo: &Repo, base: &str, head: &str) -> (String, String) {
    let output = repo.run("scripts/docs-lane.sh", &["classify", base, head], &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    (stdout(&output), stderr(&output))
}

/// One path of a change, and its new text, `None` for a deletion.
type Edit<'a> = (&'a str, Option<&'a str>);

/// `repo`'s change, committed: each path written, or deleted when its text
/// is `None`.
fn change(repo: &Repo, edits: &[Edit]) {
    for (path, text) in edits {
        match text {
            Some(text) => repo.put(path, text),
            None => std::fs::remove_file(repo.path().join(path)).expect(path),
        }
    }
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "change"]);
}

/// The classifier, case by case: only a page under docs/ or at the top,
/// named by nothing but prose outside the code's trees and the docs lane's
/// own files, is documentation, and every other change, or one it cannot
/// read, is full, with its reason.
#[test]
fn the_docs_lane_classifier_calls_only_unnamed_pages_docs() {
    let edit = "# changed\n";
    let full = |reason: &str| ("full\n".to_string(), format!("docs lane: full: {reason}\n"));
    let docs = || ("docs\n".to_string(), String::new());
    let cases: [(&[Edit], (String, String)); 15] = [
        (&[("docs/guide.md", Some(edit))], docs()),
        (&[("README.md", Some(edit))], docs()),
        (&[("docs/listed.md", Some(edit))], docs()),
        (&[("docs/proposed.md", Some(edit))], docs()),
        (
            &[("docs/compiled.md", Some(edit))],
            full("the file name of docs/compiled.md is named in code's input, outside the docs lane's tests"),
        ),
        (
            &[("docs/roled.md", Some(edit))],
            full("the file name of docs/roled.md is named in code's input, outside the docs lane's tests"),
        ),
        (
            // A test the docs lane does not run names its page.
            &[("docs/tested.md", Some(edit))],
            full("the file name of docs/tested.md is named in code's input, outside the docs lane's tests"),
        ),
        (
            &[("docs/guide.md", Some(edit)), ("crates/cli/src/lib.rs", Some("fn f() {}\n"))],
            full("crates/cli/src/lib.rs is not a .md file"),
        ),
        (
            &[(".github/workflows/ci.yml", Some("jobs: {a: {}}\n"))],
            full(".github/workflows/ci.yml is not a .md file"),
        ),
        (
            &[(".github/pull_request_template.md", Some(edit))],
            full(".github/pull_request_template.md is neither under docs/ nor a top-level .md"),
        ),
        (
            &[("docs/guide.md", None), ("notes/guide.md", Some("# guide\n"))],
            full("notes/guide.md is neither under docs/ nor a top-level .md"),
        ),
        (&[("scripts/old.sh", None)], full("scripts/old.sh is not a .md file")),
        // A page deleted (D, 100644 to 000000): alone it is documentation;
        // one that code still names is not, since the base names it.
        (&[("docs/guide.md", None)], docs()),
        (
            &[("docs/compiled.md", None)],
            full("the file name of docs/compiled.md is named in code's input, outside the docs lane's tests"),
        ),
        (
            &[("docs/blob.md", Some("a\0binary\0page\n"))],
            full("docs/blob.md is a binary file"),
        ),
    ];
    for (edits, expected) in cases {
        let repo = lane_tree();
        change(&repo, edits);
        assert_eq!(lane_of(&repo, "base", "HEAD"), expected, "{edits:?}");
    }

    let repo = lane_tree();
    assert_eq!(lane_of(&repo, "base", "HEAD"), full("the change is empty"));
    let missing = "0123456789abcdef0123456789abcdef01234567";
    assert_eq!(
        lane_of(&repo, missing, "HEAD"),
        full(&format!("the base {missing} is not a commit here"))
    );

    // The base gained a reader of docs/guide.md since the head forked: the
    // head alone names nothing, the base does.
    let repo = lane_tree();
    change(
        &repo,
        &[(
            "crates/cli/src/guide.rs",
            Some("include_str!(\"guide.md\");\n"),
        )],
    );
    repo.git(&["tag", "tip"]);
    repo.git(&["checkout", "-q", "--detach", "base"]);
    change(&repo, &[("docs/guide.md", Some(edit))]);
    assert_eq!(
        lane_of(&repo, "tip", "HEAD"),
        full("the file name of docs/guide.md is named in code's input, outside the docs lane's tests")
    );

    let repo = lane_tree();
    std::os::unix::fs::symlink("guide.md", repo.path().join("docs/link.md")).unwrap();
    change(&repo, &[]);
    let (lane, reason) = lane_of(&repo, "base", "HEAD");
    assert_eq!(lane, "full\n");
    assert!(
        reason.starts_with("docs lane: full: docs/link.md changed as 000000 120000 "),
        "{reason}"
    );

    let repo = lane_tree();
    let output = repo.run("scripts/docs-lane.sh", &["classify", "base"], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        "usage: docs-lane.sh classify <base> <head>\n"
    );
}

/// `docs-lane.sh run` runs each package and target's filters as one test
/// command, through the guard that refuses a run of no test, in the order
/// the list first names each; one that fails fails the lane, and the rest
/// still run.
#[test]
fn the_docs_lane_runs_its_list_one_command_a_target() {
    let repo = lane_tree();
    repo.put(
        "scripts/docs-lane-tests.txt",
        "# the lane\n\
         a test:it one:: x.rs\n\
         b lib two:: y.rs\n\
         a test:it three:: z.rs\n",
    );
    for script in ["scripts/run-it-tests.sh", "scripts/test-summary.sh"] {
        repo.put(
            script,
            &std::fs::read_to_string(workspace().join(script)).unwrap(),
        );
    }
    repo.executable(
        "scripts/run-it-tests.sh",
        &repo.read("scripts/run-it-tests.sh"),
    );
    repo.executable(
        "stub-bin/cargo",
        &STUB_CARGO.replace(
            "0 failed'",
            "0 failed; 0 ignored; 0 measured; 0 filtered out'",
        ),
    );
    let ran = repo.path().join("ran.txt");
    let record = ran.to_str().unwrap();
    let output = repo.run("scripts/docs-lane.sh", &["run"], &[("STUB_OUT", record)]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        repo.read("ran.txt"),
        "cargo test --locked -p a --test it -- one:: three::\n\
         cargo test --locked -p b --lib -- two::\n"
    );
    std::fs::write(&ran, "").unwrap();
    let output = repo.run(
        "scripts/docs-lane.sh",
        &["run"],
        &[("STUB_OUT", record), ("STUB_FAIL", "cargo test")],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        repo.read("ran.txt").lines().count(),
        2,
        "the second target still ran"
    );

    repo.put("scripts/docs-lane-tests.txt", "a bench one:: x.rs\n");
    let output = repo.run("scripts/docs-lane.sh", &["run"], &[("STUB_OUT", record)]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "docs lane: scripts/docs-lane-tests.txt names bench, neither lib nor test:<name>\n"
    );
}

/// ci.yml's classifier step, run as a runner runs it: every event but a
/// pull request's is full without a classifier run; a pull request is
/// classified by its base's copy of the script, and a base without one is
/// full; the lane is the step's output.
#[test]
fn the_classifier_step_is_full_off_a_pull_request_and_runs_the_bases_copy() {
    let ci = std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).unwrap();
    let wf = crate::workflow::Workflow::parse(&ci).expect("ci.yml parses");
    let classify = wf.require_job("ci.yml", "classify").unwrap();
    let block = classify
        .steps
        .iter()
        .find_map(|s| s.run.as_deref())
        .expect("classify job run block")
        .to_string();
    let step = |repo: &Repo, event: &str| {
        let output_file = repo.path().join("output.txt");
        std::fs::write(&output_file, "").unwrap();
        let status = Command::new("bash")
            .args(["-e", "-c", &block])
            .current_dir(repo.path())
            .env("EVENT", event)
            .env("BASE", "base")
            .env("HEAD", "HEAD")
            .env("RUNNER_TEMP", repo.path())
            .env("GITHUB_OUTPUT", &output_file)
            .status()
            .expect("bash");
        (
            status.success(),
            std::fs::read_to_string(&output_file).unwrap(),
        )
    };

    let repo = lane_tree();
    change(&repo, &[("docs/guide.md", Some("# changed\n"))]);
    for event in ["merge_group", "push", "workflow_dispatch", "pull_request"] {
        assert_eq!(
            step(&repo, event),
            (true, "lane=full\n".to_string()),
            "{event}"
        );
    }

    let repo = lane_tree();
    repo.put(
        "scripts/docs-lane.sh",
        &std::fs::read_to_string(workspace().join("scripts/docs-lane.sh")).unwrap(),
    );
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "the classifier"]);
    repo.git(&["tag", "-f", "base"]);
    change(&repo, &[("docs/guide.md", Some("# changed\n"))]);
    assert_eq!(
        step(&repo, "pull_request"),
        (true, "lane=docs\n".to_string())
    );
    assert_eq!(
        step(&repo, "merge_group"),
        (true, "lane=full\n".to_string())
    );

    // The base's copy and the head's differ: the base's is the one run,
    // never the copy the pull request brings.
    let repo = lane_tree();
    repo.put("scripts/docs-lane.sh", "echo docs\n");
    repo.git(&["add", "-A"]);
    repo.git(&["commit", "-q", "-m", "the base's classifier"]);
    repo.git(&["tag", "-f", "base"]);
    change(&repo, &[("scripts/docs-lane.sh", Some("echo full\n"))]);
    assert_eq!(
        step(&repo, "pull_request"),
        (true, "lane=docs\n".to_string())
    );
}

const DIGEST_SCRIPT_TOOLS: [&str; 7] = ["cat", "chmod", "cmp", "find", "mktemp", "mv", "od"];

/// Link extra POSIX tools needed by `refresh-workflow-digests.sh` into
/// `sys-bin/` without exposing host `sha256sum` or `shasum`.
fn link_digest_script_tools(repo: &Repo) {
    repo.link_system_tools();
    repo.link_host_tools(&DIGEST_SCRIPT_TOOLS);
}

/// Build a stub `shasum` or `sha256sum` script backed solely by Rust `sha2`
/// digests so the hermetic hasher test needs neither tool on the host.
fn stub_hasher_from_sha2(is_shasum: bool, entries: &[(&str, &str)]) -> String {
    let mut script = String::from("#!/usr/bin/env bash\n");
    if is_shasum {
        script.push_str(
            "[ \"$1\" = -a ] && [ \"$2\" = 256 ] && [ \"$3\" = -- ] && [ \"$#\" -eq 4 ] || exit 99\n",
        );
        script.push_str("file=\"$4\"\n");
    } else {
        script.push_str("[ \"$1\" = -- ] && [ \"$#\" -eq 2 ] || exit 99\n");
        script.push_str("file=\"$2\"\n");
    }
    script.push_str("case \"$file\" in\n");
    for (rel, body) in entries {
        let digest = sha256_hex(body);
        script.push_str(&format!(
            "  '{rel}') printf '%s  %s\\n' '{digest}' \"$file\" ;;\n"
        ));
    }
    script.push_str("  *) exit 98 ;;\nesac\n");
    script
}

const SAMPLE_RENOVATE_GLOBAL: &str = "{ allowedCommands: [] }\n";
const SAMPLE_RENOVATE: &str = "{}\n";

fn plant_renovate_configs(repo: &Repo) {
    repo.put(".github/renovate-global.json5", SAMPLE_RENOVATE_GLOBAL);
    repo.put(".github/renovate.json5", SAMPLE_RENOVATE);
}

/// Run a workspace script hermetically (`stub-bin:sys-bin`) under a wall-clock
/// deadline so a blocked FIFO fails fast instead of hanging the suite.
fn run_script_hermetic_with_timeout(
    repo: &Repo,
    script: &str,
    args: &[&str],
    limit: std::time::Duration,
) -> Output {
    let path = format!(
        "{}:{}",
        repo.path().join("stub-bin").display(),
        repo.path().join("sys-bin").display()
    );
    let mut child = Command::new("bash")
        .arg(workspace().join(script))
        .args(args)
        .current_dir(repo.path())
        .env("PATH", path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn bash");
    let deadline = std::time::Instant::now() + limit;
    loop {
        if child.try_wait().expect("try_wait").is_some() {
            return child.wait_with_output().expect("wait_with_output");
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("{script} {args:?} blocked past {limit:?}");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn assert_digest_check_fails_with(repo: &Repo, expected: &str, absent: &str) {
    let out = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &["--check"], &[]);
    assert!(!out.status.success());
    let err = stderr(&out);
    assert!(err.contains(expected), "{err}");
    assert!(!err.contains(absent), "{err}");
}

#[test]
fn refresh_workflow_digests_agrees_with_guard_handles_check_symlinks_and_hashers() {
    let repo = Repo::new();
    link_digest_script_tools(&repo);
    let wf = "name: ci\non: push\njobs:\n  ok:\n    runs-on: ubuntu-24.04\n    steps:\n      - uses: ./.github/actions/outer/inner\n";
    let action =
        "name: inner\nruns:\n  using: composite\n  steps:\n    - run: echo hi\n      shell: bash\n";
    let helper = "echo helper\n";
    repo.put(".github/workflows/ci.yml", wf);
    repo.put(".github/actions/outer/inner/action.yml", action);
    repo.put(".github/actions/outer/helper.sh", helper);
    plant_renovate_configs(&repo);

    // Fails closed when neither sha256sum nor shasum is on PATH.
    let neither = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &["--check"], &[]);
    assert!(!neither.status.success());
    assert!(
        stderr(&neither).contains("neither sha256sum nor shasum"),
        "{}",
        stderr(&neither)
    );

    // Every subsequent run uses only the test's own PATH (stub hasher + linked POSIX tools).
    let initial_entries = [
        (".github/actions/outer/helper.sh", helper),
        (".github/actions/outer/inner/action.yml", action),
        (".github/renovate-global.json5", SAMPLE_RENOVATE_GLOBAL),
        (".github/renovate.json5", SAMPLE_RENOVATE),
        (".github/workflows/ci.yml", wf),
    ];
    repo.executable(
        "stub-bin/sha256sum",
        &stub_hasher_from_sha2(false, &initial_entries),
    );

    // --check before digest file exists fails with its missing-file sentence and writes nothing.
    assert_digest_check_fails_with(
        &repo,
        ".github/workflow-digests.txt is missing; run bash scripts/refresh-workflow-digests.sh",
        "is out of date",
    );
    assert!(!repo
        .path()
        .join(crate::workflow_digests::DIGEST_FILE)
        .exists());

    // Refresh writes the digest file and the Rust guard passes on the nested tree.
    let write = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(write.status.success(), "{}", stderr(&write));
    assert_eq!(
        crate::workflow_digests::check_workflow_digests(repo.path()),
        Vec::new()
    );
    let recorded = repo.read(crate::workflow_digests::DIGEST_FILE);
    let expected = format!(
        "{}  .github/actions/outer/helper.sh\n{}  .github/actions/outer/inner/action.yml\n{}  .github/renovate-global.json5\n{}  .github/renovate.json5\n{}  .github/workflows/ci.yml\n",
        sha256_hex(helper),
        sha256_hex(action),
        sha256_hex(SAMPLE_RENOVATE_GLOBAL),
        sha256_hex(SAMPLE_RENOVATE),
        sha256_hex(wf),
    );
    assert_eq!(recorded, expected);

    // --check succeeds when up to date, and rewriting unchanged tree changes nothing.
    let check_ok = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &["--check"], &[]);
    assert!(check_ok.status.success(), "{}", stderr(&check_ok));
    let rewrite = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(rewrite.status.success(), "{}", stderr(&rewrite));
    assert_eq!(repo.read(crate::workflow_digests::DIGEST_FILE), expected);

    // After a workflow edit, --check exits non-zero and writes nothing.
    let edited = format!("{wf}# changed\n");
    let edited_entries = [
        (".github/actions/outer/helper.sh", helper),
        (".github/actions/outer/inner/action.yml", action),
        (".github/renovate-global.json5", SAMPLE_RENOVATE_GLOBAL),
        (".github/renovate.json5", SAMPLE_RENOVATE),
        (".github/workflows/ci.yml", edited.as_str()),
    ];
    repo.put(".github/workflows/ci.yml", &edited);
    repo.executable(
        "stub-bin/sha256sum",
        &stub_hasher_from_sha2(false, &edited_entries),
    );
    assert_digest_check_fails_with(
        &repo,
        ".github/workflow-digests.txt is out of date; run bash scripts/refresh-workflow-digests.sh",
        "is missing",
    );
    assert_eq!(repo.read(crate::workflow_digests::DIGEST_FILE), expected);

    // Refresh updates it and guard passes again.
    let refresh_again = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(refresh_again.status.success(), "{}", stderr(&refresh_again));
    assert_eq!(
        crate::workflow_digests::check_workflow_digests(repo.path()),
        Vec::new()
    );

    // Succeeds with a `shasum -a 256` fallback when `sha256sum` is absent.
    std::fs::remove_file(repo.path().join("stub-bin/sha256sum")).expect("rm sha256sum stub");
    repo.executable(
        "stub-bin/shasum",
        &stub_hasher_from_sha2(true, &edited_entries),
    );
    let shasum_ok = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &["--check"], &[]);
    assert!(shasum_ok.status.success(), "{}", stderr(&shasum_ok));
    std::fs::remove_file(repo.path().join(crate::workflow_digests::DIGEST_FILE)).expect("rm");
    let shasum_write = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(shasum_write.status.success(), "{}", stderr(&shasum_write));
    assert_eq!(
        crate::workflow_digests::check_workflow_digests(repo.path()),
        Vec::new()
    );
}

#[test]
fn refresh_workflow_digests_refuses_symlinks_and_missing_renovate_files() {
    let repo = Repo::new();
    link_digest_script_tools(&repo);
    repo.put(".github/workflows/ci.yml", "name: ci\n");
    std::fs::create_dir_all(repo.path().join(".github/actions")).expect("actions dir");
    plant_renovate_configs(&repo);
    repo.executable("stub-bin/sha256sum", "#!/usr/bin/env bash\nexit 99\n");

    repo.put("outside.yml", "name: x\n");
    std::os::unix::fs::symlink(
        repo.path().join("outside.yml"),
        repo.path().join(".github/actions/link.yml"),
    )
    .expect("symlink");
    let sym = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(!sym.status.success());
    assert!(
        stderr(&sym).contains("refusing non-regular entry"),
        "{}",
        stderr(&sym)
    );
    std::fs::remove_file(repo.path().join(".github/actions/link.yml")).expect("rm symlink");

    std::fs::remove_file(repo.path().join(".github/renovate.json5")).expect("rm renovate");
    let missing = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(!missing.status.success());
    assert!(
        stderr(&missing).contains("missing pinned file: .github/renovate.json5"),
        "{}",
        stderr(&missing)
    );

    std::os::unix::fs::symlink(
        repo.path().join("outside.yml"),
        repo.path().join(".github/renovate.json5"),
    )
    .expect("symlink renovate");
    let sym_ren = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(!sym_ren.status.success());
    assert!(
        stderr(&sym_ren).contains("refusing symbolic link: .github/renovate.json5"),
        "{}",
        stderr(&sym_ren)
    );

    // Overflowing the OS pipe buffer with many symlinks must still report the
    // named non-regular-entry refusal (no SIGPIPE / exit 141 under pipefail).
    std::fs::remove_file(repo.path().join(".github/renovate.json5")).expect("rm symlink renovate");
    plant_renovate_configs(&repo);
    let outside = repo.path().join("outside.yml");
    let wf_dir = repo.path().join(".github/workflows");
    for i in 0..4096 {
        std::os::unix::fs::symlink(&outside, wf_dir.join(format!("s{i:04}.yml")))
            .expect("plant symlink");
    }
    let many_sym = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &["--check"], &[]);
    assert_eq!(
        many_sym.status.code(),
        Some(1),
        "expected exit 1, got {:?}; stderr={}",
        many_sym.status,
        stderr(&many_sym)
    );
    assert!(
        stderr(&many_sym).contains("refusing non-regular entry: .github/workflows/s0000.yml"),
        "{}",
        stderr(&many_sym)
    );
}

#[test]
fn refresh_workflow_digests_refuses_invalid_path_names_and_writes_nothing() {
    let wf = "name: ci\non: push\njobs:\n  ok:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: echo ok\n";
    let new_repo = || {
        let repo = Repo::new();
        link_digest_script_tools(&repo);
        repo.put(".github/workflows/ci.yml", wf);
        std::fs::create_dir_all(repo.path().join(".github/actions")).expect("actions dir");
        plant_renovate_configs(&repo);
        repo.executable("stub-bin/sha256sum", "#!/usr/bin/env bash\nexit 99\n");
        repo
    };
    let assert_refused = |repo: &Repo, expected_escaped: &[&str], sentinel: Option<&str>| {
        for args in [&[][..], &["--check"][..]] {
            let out = repo.run_hermetic("scripts/refresh-workflow-digests.sh", args, &[]);
            assert!(
                !out.status.success(),
                "args={args:?} must fail; stderr={}",
                stderr(&out)
            );
            let err = stderr(&out);
            for needle in expected_escaped {
                assert!(
                    err.contains(&format!("refusing invalid path name: {needle}")),
                    "args={args:?} missing escaped name `{needle}` in stderr: {err}"
                );
            }
            let target = repo.path().join(crate::workflow_digests::DIGEST_FILE);
            match sentinel {
                Some(prev) => assert_eq!(repo.read(crate::workflow_digests::DIGEST_FILE), prev),
                None => assert!(!target.exists(), "must not create digest file on refusal"),
            }
        }
    };

    let space_repo = new_repo();
    space_repo.put(".github/actions/bad name.yml", "name: bad\n");
    assert_refused(&space_repo, &[r".github/actions/bad\x20name.yml"], None);

    let non_ascii_repo = new_repo();
    non_ascii_repo.put(".github/actions/ok/action.yml", "name: ok\n");
    non_ascii_repo.put(".github/workflows/caf\u{00e9}.yml", wf);
    assert_refused(
        &non_ascii_repo,
        &[r".github/workflows/caf\xc3\xa9.yml"],
        None,
    );

    // 48 identical bytes plus one invalid byte: `od -v` must preserve every byte.
    let repeat_repo = new_repo();
    let repeated_prefix = "a".repeat(48);
    repeat_repo.put(
        &format!(".github/actions/{repeated_prefix} "),
        "name: repeat\n",
    );
    let expected_repeated = format!(r".github/actions/{repeated_prefix}\x20");
    assert_refused(&repeat_repo, &[&expected_repeated], None);

    // Non-UTF-8 filenames (skip only on host FS EILSEQ, e.g. macOS APFS).
    let raw_repo = new_repo();
    if crate::workflow_digests::plant_raw_name_file(
        &raw_repo.path().join(".github/actions"),
        b"bad-\xff.txt",
        "raw\n",
    ) {
        assert_refused(&raw_repo, &[r".github/actions/bad-\xff.txt"], None);
    }

    let twin_repo = new_repo();
    twin_repo.put(".github/actions/helper-\u{FFFD}.txt", "twin\n");
    let sentinel = "sentinel-unchanged\n";
    twin_repo.put(crate::workflow_digests::DIGEST_FILE, sentinel);
    if crate::workflow_digests::plant_raw_name_file(
        &twin_repo.path().join(".github/actions"),
        b"helper-\xff.txt",
        "sibling\n",
    ) {
        assert_refused(
            &twin_repo,
            &[
                r".github/actions/helper-\xef\xbf\xbd.txt",
                r".github/actions/helper-\xff.txt",
            ],
            Some(sentinel),
        );
    }
}

#[test]
fn refresh_workflow_digests_refuses_fifo_target_under_timeout_and_writes_atomically() {
    use std::io::Read as _;
    use std::os::unix::fs::{FileTypeExt, MetadataExt};

    let repo = Repo::new();
    link_digest_script_tools(&repo);
    let wf = "name: ci\non: push\njobs:\n  ok:\n    runs-on: ubuntu-24.04\n    steps:\n      - run: echo ok\n";
    repo.put(".github/workflows/ci.yml", wf);
    std::fs::create_dir_all(repo.path().join(".github/actions")).expect("actions dir");
    plant_renovate_configs(&repo);
    let initial_entries = [
        (".github/renovate-global.json5", SAMPLE_RENOVATE_GLOBAL),
        (".github/renovate.json5", SAMPLE_RENOVATE),
        (".github/workflows/ci.yml", wf),
    ];
    repo.executable(
        "stub-bin/sha256sum",
        &stub_hasher_from_sha2(false, &initial_entries),
    );

    let target = repo.path().join(crate::workflow_digests::DIGEST_FILE);
    assert!(
        Command::new("mkfifo")
            .arg(&target)
            .status()
            .is_ok_and(|s| s.success()),
        "mkfifo {}",
        target.display()
    );

    let limit = std::time::Duration::from_secs(5);
    for args in [&[][..], &["--check"][..]] {
        let out = run_script_hermetic_with_timeout(
            &repo,
            "scripts/refresh-workflow-digests.sh",
            args,
            limit,
        );
        assert!(
            !out.status.success(),
            "FIFO target must be refused for args={args:?}"
        );
        assert!(
            stderr(&out).contains("refusing non-regular target: .github/workflow-digests.txt"),
            "args={args:?}: {}",
            stderr(&out)
        );
        assert!(
            std::fs::symlink_metadata(&target)
                .expect("fifo metadata")
                .file_type()
                .is_fifo(),
            "FIFO target must remain untouched"
        );
    }

    std::fs::remove_file(&target).expect("rm fifo");
    let written = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(written.status.success(), "{}", stderr(&written));
    let stray: Vec<_> = std::fs::read_dir(repo.path().join(".github"))
        .expect("read .github")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("workflow-digests.txt."))
        .collect();
    assert_eq!(stray, Vec::<String>::new());

    // Unchanged tree rewrites nothing (preserves inode).
    let before_ino = std::fs::metadata(&target).expect("meta before").ino();
    let noop = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(noop.status.success(), "{}", stderr(&noop));
    assert_eq!(
        before_ino,
        std::fs::metadata(&target).expect("meta after").ino()
    );

    // Changed content atomically renames a new inode; open fd still reads old content whole.
    let old_content = repo.read(crate::workflow_digests::DIGEST_FILE);
    let mut old_fd = std::fs::File::open(&target).expect("open old digest file");
    let edited_wf = format!("{wf}# updated\n");
    repo.put(".github/workflows/ci.yml", &edited_wf);
    let updated_entries = [
        (".github/renovate-global.json5", SAMPLE_RENOVATE_GLOBAL),
        (".github/renovate.json5", SAMPLE_RENOVATE),
        (".github/workflows/ci.yml", edited_wf.as_str()),
    ];
    repo.executable(
        "stub-bin/sha256sum",
        &stub_hasher_from_sha2(false, &updated_entries),
    );
    let updated = repo.run_hermetic("scripts/refresh-workflow-digests.sh", &[], &[]);
    assert!(updated.status.success(), "{}", stderr(&updated));
    assert_ne!(
        before_ino,
        std::fs::metadata(&target).expect("meta updated").ino()
    );
    let mut from_old_fd = String::new();
    old_fd
        .read_to_string(&mut from_old_fd)
        .expect("read old fd");
    assert_eq!(from_old_fd, old_content);
    assert_ne!(repo.read(crate::workflow_digests::DIGEST_FILE), old_content);
    assert_eq!(
        crate::workflow_digests::check_workflow_digests(repo.path()),
        Vec::new()
    );
}

#[test]
fn docs_lane_keeps_workflow_action_and_digest_changes_on_the_full_lane() {
    for (path, body) in [
        (".github/workflows/ci.yml", "name: ci\n"),
        (".github/actions/local/action.yml", "name: local\n"),
        (".github/renovate.json5", "{}\n"),
        (".github/renovate-global.json5", "{}\n"),
        (
            ".github/workflow-digests.txt",
            "00  .github/workflows/ci.yml\n",
        ),
    ] {
        let repo = lane_tree();
        change(&repo, &[(path, Some(body))]);
        assert_eq!(
            lane_of(&repo, "base", "HEAD"),
            (
                "full\n".to_string(),
                format!("docs lane: full: {path} is not a .md file\n")
            ),
            "{path} must take the full lane so workflow_digests runs"
        );
    }
}
