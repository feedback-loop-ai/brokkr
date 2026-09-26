//! `scripts/refresh-pin-checksums.sh`, Renovate's post-upgrade task (issue
//! #339), run against a scratch repository with a stub `curl` first on
//! `PATH`: a digest moves only with its own version, an unchanged version
//! whose release no longer matches is refused, and nothing is written
//! unless every release was measured and at least one version moved.

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

fn digest(tool: &str, version: &str) -> String {
    let hash = Sha256::digest(format!("release {}", url(tool, version)));
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// An action in the shape the script reads, recording `sha256`.
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

struct Scratch {
    root: tempfile::TempDir,
}

impl Scratch {
    /// A repository whose HEAD pins alpha 1.0.0 and beta 2.0.0, each at the
    /// digest its release really has.
    fn new() -> Self {
        let root = tempfile::tempdir().expect("scratch repository");
        let scratch = Self { root };
        scratch.write("alpha", "1.0.0", &digest("alpha", "1.0.0"));
        scratch.write("beta", "2.0.0", &digest("beta", "2.0.0"));
        let bin = scratch.path().join("stub-bin");
        std::fs::create_dir_all(&bin).expect("stub bin");
        std::fs::write(bin.join("curl"), STUB_CURL).expect("stub curl");
        let mut mode = std::fs::metadata(bin.join("curl"))
            .expect("stub")
            .permissions();
        mode.set_mode(0o755);
        std::fs::set_permissions(bin.join("curl"), mode).expect("stub mode");
        scratch.git(&["init", "-q"]);
        scratch.git(&["add", ".github"]);
        scratch.git(&["commit", "-q", "-m", "pins"]);
        scratch
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn file(&self, tool: &str) -> PathBuf {
        self.path()
            .join(format!(".github/actions/setup-{tool}/action.yml"))
    }

    fn write(&self, tool: &str, version: &str, sha256: &str) {
        let file = self.file(tool);
        std::fs::create_dir_all(file.parent().expect("action directory")).expect("mkdir");
        std::fs::write(file, action(tool, version, sha256)).expect("action");
    }

    fn read(&self, tool: &str) -> String {
        std::fs::read_to_string(self.file(tool)).expect("action")
    }

    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(self.path())
            .args([
                "-c",
                "user.name=pins",
                "-c",
                "user.email=pins@example.invalid",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .expect("git");
        assert!(status.success(), "git {args:?}");
    }

    /// The script, run here, with the stub first on `PATH`.
    fn refresh(&self, stub: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.path().join("stub-bin").display(),
            std::env::var("PATH").expect("PATH")
        );
        Command::new("bash")
            .arg(workspace().join("scripts/refresh-pin-checksums.sh"))
            .current_dir(self.path())
            .env("PATH", path)
            .env_remove("STUB_SWAP")
            .env_remove("STUB_FAIL")
            .envs(stub.iter().copied())
            .output()
            .expect("bash")
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
        String::from_utf8_lossy(&output.stdout),
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
    let swapped: String = Sha256::digest(format!("release {} swapped", url("beta", "2.0.0")))
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
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
