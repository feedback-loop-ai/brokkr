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
//!   file whose fences mermaid-cli does not all render is refused.

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
