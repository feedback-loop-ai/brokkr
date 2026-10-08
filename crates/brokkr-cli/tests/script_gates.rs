//! The scripts the non-Rust lints and Renovate run (issue #339), each run
//! against a scratch repository with stubs first on `PATH`, so what each one
//! refuses is bound by a test:
//!
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
//!   names it; and `recipes/fast`'s verify seat, the one a landing
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

    /// Links each of the [`SYSTEM_TOOLS`] from the host `PATH` into
    /// `sys-bin/`.
    fn link_system_tools(&self) {
        let system = self.path().join("sys-bin");
        std::fs::create_dir_all(&system).expect("sys-bin");
        let host = std::env::var_os("PATH").expect("PATH");
        for tool in SYSTEM_TOOLS {
            let found = std::env::split_paths(&host)
                .map(|directory| directory.join(tool))
                .find(|candidate| candidate.is_file())
                .unwrap_or_else(|| panic!("{tool} is not on the host PATH"));
            std::os::unix::fs::symlink(found, system.join(tool)).expect(tool);
        }
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
        curl -sSfL --retry 3 -o \"$tarball\" \\
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

/// fast's verify seat, the one a landing inherits, run in `repo` with
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
        "recipes/fast/scripts/verify-seat.sh",
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
        r#"{"result": "pass", "notes": "cargo fmt --all -- --check: clean; cargo clippy --workspace --all-targets --all-features --locked -- -D warnings: clean\nlint-non-rust: not run: git ls-files -z .github/workflows .github/actions | xargs -0 zizmor --offline (zizmor on PATH reports zizmor 0.1.0, not the pinned 3.0.0)\nlint-non-rust: not run: git ls-files -z '*.md' | xargs -0 lychee --offline --include-fragments --no-progress (lychee is not on PATH)\nlint-non-rust: 4 of 6 lints ran clean\ncargo test --workspace: 1 successful test-suite summaries, 0 failed; cargo run -p brokkr-cli -- compile --bundle recipes/self: 1 bundle compiled, 0 failed (offline from the bound Cargo registry cache)"}
"#
    );
}
