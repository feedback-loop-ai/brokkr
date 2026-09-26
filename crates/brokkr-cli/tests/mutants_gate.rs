//! `scripts/mutants.sh gate`, the required brokkr-core check the operator
//! ruled on #289: a pull request that adds a miss to brokkr-core fails.
//! Each case runs the real script against a stub `cargo` that plays
//! cargo-mutants, and a planted allow-list, so the verdict is pinned
//! without a mutation run. The last case holds the workflow's required job
//! to that gate.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A committed miss, and the same file and mutation at another line.
const KNOWN: &str = "crates/brokkr-core/src/fold.rs:384:41: replace == with != in apply";
const KNOWN_MOVED: &str = "crates/brokkr-core/src/fold.rs:390:41: replace == with != in apply";
const KNOWN_AGAIN: &str = "crates/brokkr-core/src/fold.rs:512:9: replace == with != in apply";
/// A miss no committed one accounts for.
const FRESH: &str = "crates/brokkr-core/src/policy.rs:700:5: replace < with <= in planted";

/// Stands in for `cargo mutants`. Each call appends its arguments as one
/// line to `STUB_ARGV`. `--list` prints `STUB_LISTED` (only if the
/// `--in-diff` file names `STUB_LIST_NAMING`, when that is set) and exits
/// `STUB_LIST_STATUS`; a run writes `STUB_MISSED` as its missed.txt and
/// one mutant as its mutants.json (unless `STUB_NO_OUTPUT`), and exits
/// `STUB_STATUS`.
const STUB: &str = r#"#!/usr/bin/env bash
printf '%s\n' "$*" >> "$STUB_ARGV"
out=""; list=""; diff=""
while [ $# -gt 0 ]; do
  case "$1" in --output) out="$2"; shift ;; --in-diff) diff="$2"; shift ;; --list) list=1 ;; esac
  shift
done
if [ -n "$list" ]; then
  [ -z "${STUB_LIST_NAMING:-}" ] || grep -qF -e "$STUB_LIST_NAMING" "$diff" || exit "${STUB_LIST_STATUS:-0}"
  printf '%s' "${STUB_LISTED:-}"; exit "${STUB_LIST_STATUS:-0}"
fi
if [ -z "${STUB_NO_OUTPUT:-}" ]; then
  mkdir -p "$out/mutants.out"
  printf '%s' "${STUB_MISSED:-}" > "$out/mutants.out/missed.txt"
  printf '[{"name":"planted"}]' > "$out/mutants.out/mutants.json"
fi
exit "${STUB_STATUS:-0}"
"#;

struct Gate {
    dir: tempfile::TempDir,
}

impl Gate {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::create_dir_all(dir.path().join("allow")).unwrap();
        let stub = dir.path().join("bin/cargo");
        std::fs::write(&stub, STUB).unwrap();
        let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
        std::fs::set_permissions(&stub, mode).unwrap();
        for crate_name in ["brokkr-core", "brokkr-protocol"] {
            let allow = dir.path().join(format!("allow/{crate_name}.missed.txt"));
            std::fs::write(
                allow,
                if crate_name == "brokkr-core" {
                    lines(&[KNOWN])
                } else {
                    String::new()
                },
            )
            .unwrap();
        }
        Self { dir }
    }

    /// Run `scripts/mutants.sh <args>` from the workspace with `env` set.
    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.run_in(&workspace(), args, env)
    }

    /// Run `scripts/mutants.sh <args>` from the repository at `cwd`.
    fn run_in(&self, cwd: &Path, args: &[&str], env: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.path("bin").display(),
            std::env::var("PATH").unwrap()
        );
        let mut command = Command::new("bash");
        command
            .arg(workspace().join("scripts/mutants.sh"))
            .args(args)
            .current_dir(cwd)
            .env("PATH", path)
            .env("MUTANTS_OUT", self.path("out"))
            .env("MUTANTS_ALLOW", self.path("allow"))
            .env("STUB_ARGV", self.path("argv"))
            .env_remove("MUTANTS_JOBS")
            .env_remove("GITHUB_STEP_SUMMARY");
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().unwrap()
    }

    /// The gate over a diff that lists one mutant, whose run misses `missed`.
    fn verdict(&self, missed: &[&str]) -> Output {
        let missed = lines(missed);
        self.run(
            &["gate", "HEAD", "brokkr-core"],
            &[
                ("STUB_LISTED", "a listed mutant\n"),
                ("STUB_MISSED", &missed),
                ("STUB_STATUS", "2"),
            ],
        )
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// A scratch repository of two commits, the second planting `file`.
    fn branch(&self, file: &str) -> PathBuf {
        let repo = self.path("repo");
        std::fs::create_dir_all(repo.join(Path::new(file).parent().unwrap())).unwrap();
        std::fs::write(repo.join("README.md"), "base\n").unwrap();
        git(&repo, &["init", "--quiet"]);
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "--quiet", "-m", "base"]);
        std::fs::write(repo.join(file), "pub fn planted() {}\n").unwrap();
        git(&repo, &["add", file]);
        git(&repo, &["commit", "--quiet", "-m", "branch"]);
        repo
    }
}

fn git(repo: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    for setting in [
        "user.name=stub",
        "user.email=stub@example.invalid",
        "commit.gpgsign=false",
        "core.hooksPath=/dev/null",
    ] {
        command.args(["-c", setting]);
    }
    let status = command.args(args).current_dir(repo).status().unwrap();
    assert!(status.success(), "git {args:?}");
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lines(lines: &[&str]) -> String {
    lines.iter().map(|line| format!("{line}\n")).collect()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn a_miss_no_committed_one_accounts_for_fails_the_gate_by_name() {
    let output = Gate::new().verdict(&[FRESH]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains(&format!("- {FRESH}")),
        "{}",
        text(&output.stdout)
    );
    assert!(text(&output.stderr).contains("brokkr-core miss(es); a test must catch each"));
}

#[test]
fn a_committed_miss_at_a_moved_line_passes_the_gate() {
    let output = Gate::new().verdict(&[KNOWN_MOVED]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("misses this diff adds: 0"));
}

#[test]
fn a_second_miss_of_a_committed_file_and_mutation_fails_the_gate() {
    let output = Gate::new().verdict(&[KNOWN_MOVED, KNOWN_AGAIN]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("misses this diff adds: 1"), "{stdout}");
    assert!(stdout.contains(&format!("- {KNOWN_AGAIN}")), "{stdout}");
    assert!(!stdout.contains(&format!("- {KNOWN_MOVED}")), "{stdout}");
}

/// The gate lists, then measures, the branch's diff over brokkr-core's
/// scope: whole-scope misses would equal the committed list and pass, and
/// another crate's would not be brokkr-core's.
#[test]
fn the_gate_lists_and_measures_brokkr_core_over_the_diff() {
    let gate = Gate::new();
    let output = gate.verdict(&[KNOWN_MOVED]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    let out = gate.path("out");
    let scope = format!(
        "--in-diff {}/branch.diff --package brokkr-core --file crates/brokkr-core/**",
        out.display()
    );
    assert_eq!(
        std::fs::read_to_string(gate.path("argv")).unwrap(),
        format!(
            "mutants --list {scope}\nmutants --no-shuffle --output {} {scope}\n",
            out.display()
        )
    );
}

/// The diff the gate hands cargo-mutants is the branch's own: the stub
/// lists a mutant only when that diff names the file the branch planted,
/// so a gate that wrote an empty diff would list nothing and pass.
#[test]
fn the_gate_measures_the_changes_the_branch_carries() {
    const PLANTED: &str = "crates/brokkr-core/src/planted.rs";
    let gate = Gate::new();
    let repo = gate.branch(PLANTED);
    let missed = lines(&[FRESH]);
    let output = gate.run_in(
        &repo,
        &["gate", "HEAD~1", "brokkr-core"],
        &[
            ("STUB_LIST_NAMING", PLANTED),
            ("STUB_LISTED", "a listed mutant\n"),
            ("STUB_MISSED", &missed),
            ("STUB_STATUS", "2"),
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains(&format!("- {FRESH}")),
        "{}",
        text(&output.stdout)
    );
}

#[test]
fn a_diff_with_no_brokkr_core_mutant_passes_without_a_run() {
    // A run would exit 99 and fail the gate: only the empty list may pass.
    let output = Gate::new().run(&["gate", "HEAD", "brokkr-core"], &[("STUB_STATUS", "99")]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("brokkr-core mutants in this diff: 0"));
}

/// One failure to measure: the stub's environment, the gate's exit code,
/// and the reason it prints.
type Failure<'a> = (&'a [(&'a str, &'a str)], i32, &'a str);

#[test]
fn every_failure_to_measure_fails_the_gate() {
    let gate = Gate::new();
    let listed = ("STUB_LISTED", "a listed mutant\n");
    let cases: [Failure; 4] = [
        (&[listed, ("STUB_STATUS", "4")], 4, "cargo mutants exited 4"),
        (
            &[listed, ("STUB_STATUS", "2"), ("STUB_NO_OUTPUT", "1")],
            1,
            "missed.txt is missing",
        ),
        (&[("STUB_LIST_STATUS", "5")], 5, ""),
        (
            &[listed, ("STUB_STATUS", "70")],
            70,
            "cargo mutants exited 70",
        ),
    ];
    for (env, code, reason) in cases {
        let output = gate.run(&["gate", "HEAD", "brokkr-core"], env);
        assert_eq!(
            output.status.code(),
            Some(code),
            "{env:?}: {}",
            text(&output.stderr)
        );
        assert!(
            text(&output.stderr).contains(reason),
            "{env:?}: {}",
            text(&output.stderr)
        );
    }
}

#[test]
fn the_gate_holds_brokkr_core_alone_and_needs_its_allow_list() {
    let gate = Gate::new();
    let output = gate.run(&["gate", "HEAD", "brokkr-protocol"], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("only brokkr-core is gated"));
    std::fs::remove_file(gate.path("allow/brokkr-core.missed.txt")).unwrap();
    let output = gate.run(&["gate", "HEAD", "brokkr-core"], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output.stderr).contains("brokkr-core.missed.txt is missing"));
}

/// Once every committed core miss is caught, the allow-list is empty. An
/// empty list must still fail a fresh miss: the #420 landing found an
/// `NR == FNR` awk idiom that read every fresh miss as committed here.
#[test]
fn an_empty_committed_list_still_fails_a_fresh_miss() {
    let gate = Gate::new();
    std::fs::write(gate.path("allow/brokkr-core.missed.txt"), "").unwrap();
    let output = gate.verdict(&[FRESH]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(
        text(&output.stdout).contains(&format!("- {FRESH}")),
        "{}",
        text(&output.stdout)
    );
}

/// A base the gate cannot diff against measures nothing, so it fails.
#[test]
fn a_base_git_cannot_resolve_fails_the_gate() {
    let output = Gate::new().run(&["gate", "no-such-base-ref", "brokkr-core"], &[]);
    assert!(!output.status.success(), "{}", text(&output.stdout));
}

/// The body of mutants.yml's job `id`: its lines indented under the id.
fn job(workflow: &str, id: &str) -> String {
    let (_, rest) = workflow
        .split_once(&format!("\n  {id}:\n"))
        .unwrap_or_else(|| panic!("mutants.yml has no {id} job"));
    rest.lines()
        .take_while(|line| line.is_empty() || line.starts_with("    "))
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The check branch protection names runs the gate, whole and unsoftened,
/// under the name the guide gives it; the protocol job only reports. The
/// #420 landing found that turning the run line to `in-diff` left every
/// test green while the required check stopped failing anything.
#[test]
fn the_required_job_runs_the_gate_and_protocol_only_reports() {
    let workflow = std::fs::read_to_string(workspace().join(".github/workflows/mutants.yml"))
        .expect("mutants.yml");
    let core = job(&workflow, "core-gate");
    for line in [
        "    name: 'mutants in the diff: brokkr-core'\n",
        "          BASE: ${{ github.event.pull_request.base.sha }}\n",
        "        run: bash scripts/mutants.sh gate \"$BASE\" brokkr-core\n",
    ] {
        assert!(core.contains(line), "core-gate lacks {line:?}:\n{core}");
    }
    assert!(!core.contains("continue-on-error"), "{core}");
    assert_eq!(workflow.matches("scripts/mutants.sh gate ").count(), 1);
    let protocol = job(&workflow, "in-diff");
    for line in [
        "    name: 'mutants in the diff: brokkr-protocol (report only)'\n",
        "        run: bash scripts/mutants.sh in-diff \"$BASE\" brokkr-protocol\n",
    ] {
        assert!(
            protocol.contains(line),
            "in-diff lacks {line:?}:\n{protocol}"
        );
    }
    let guide = std::fs::read_to_string(workspace().join("docs/guides/contributing-by-hand.md"))
        .expect("the contributing guide");
    for named in [
        "`mutants in the diff: brokkr-core`",
        "`bash scripts/mutants.sh gate origin/main brokkr-core`",
    ] {
        assert!(guide.contains(named), "the guide does not name {named}");
    }
}
