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

/// The file a scratch branch plants in brokkr-core, its source, and its
/// hunk as git renders it.
const PLANTED: &str = "crates/brokkr-core/src/planted.rs";
const SOURCE: &[u8] = b"pub fn planted() {}\n";
const PLANTED_HUNK: &str = "diff --git a/crates/brokkr-core/src/planted.rs \
b/crates/brokkr-core/src/planted.rs\nnew file mode 100644\n--- /dev/null\n\
+++ b/crates/brokkr-core/src/planted.rs\n@@ -0,0 +1 @@\n+pub fn planted() {}\n";

/// Stands in for `cargo mutants`. Each call appends its arguments as one
/// line to `STUB_ARGV`. `--list` prints `STUB_LISTED` (only if the
/// `--in-diff` file names `STUB_LIST_NAMING`, when that is set) and exits
/// `STUB_LIST_STATUS`; a run writes `STUB_MISSED` as its missed.txt and
/// `STUB_JSON` (when set; one mutant when not) as its mutants.json,
/// leaving out the file `STUB_OMIT` names, and exits `STUB_STATUS`.
const STUB: &str = r#"#!/usr/bin/env bash
printf '%s\n' "$*" >> "$STUB_ARGV"
out=""; list=""; diff=""
while [ $# -gt 0 ]; do
  case "$1" in --output) out="$2"; shift ;; --in-diff) diff="$2"; shift ;; --list) list=1 ;; esac
  shift
done
if [ -n "$list" ]; then
  [ -z "${STUB_LIST_NAMING:-}" ] || grep -qaF -e "$STUB_LIST_NAMING" "$diff" || exit "${STUB_LIST_STATUS:-0}"
  printf '%s' "${STUB_LISTED:-}"; exit "${STUB_LIST_STATUS:-0}"
fi
mkdir -p "$out/mutants.out"
[ "${STUB_OMIT:-}" = missed.txt ] || printf '%s' "${STUB_MISSED:-}" > "$out/mutants.out/missed.txt"
json='[{"name":"planted"}]'; [ -z "${STUB_JSON+set}" ] || json="$STUB_JSON"
[ "${STUB_OMIT:-}" = mutants.json ] || printf '%s' "$json" > "$out/mutants.out/mutants.json"
exit "${STUB_STATUS:-0}"
"#;

/// Stands in for `git`, passing every call to the host's git, except that
/// the rendered diff (the one call with `--src-prefix=a/`) prints
/// `STUB_RENDER` when that is set, empty included: a diff git might one day
/// render without a hunk, planted.
const RENDER_GIT: &str = r#"#!/usr/bin/env bash
if [ -n "${STUB_RENDER+set}" ]; then
  for arg in "$@"; do
    [ "$arg" != --src-prefix=a/ ] || { printf '%s' "$STUB_RENDER"; exit 0; }
  done
fi
PATH="$STUB_PATH" exec git "$@"
"#;

/// Stands in for `wc`, padding the host's count as macOS's `wc` does, so
/// every case sees the counts a macOS runner prints. `STUB_PATH` is the
/// host's own `PATH`.
const PADDING_WC: &str = r#"#!/usr/bin/env bash
count="$(PATH="$STUB_PATH" wc "$@")" || exit
printf '%8s\n' "$count"
"#;

struct Gate {
    dir: tempfile::TempDir,
}

impl Gate {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::create_dir_all(dir.path().join("allow")).unwrap();
        for (name, body) in [("cargo", STUB), ("wc", PADDING_WC), ("git", RENDER_GIT)] {
            let stub = dir.path().join("bin").join(name);
            std::fs::write(&stub, body).unwrap();
            let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
            std::fs::set_permissions(&stub, mode).unwrap();
        }
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
        let host = std::env::var("PATH").unwrap();
        let path = format!("{}:{host}", self.path("bin").display());
        let mut command = Command::new("bash");
        command
            .arg(workspace().join("scripts/mutants.sh"))
            .args(args)
            .current_dir(cwd)
            .env("PATH", path)
            .env("STUB_PATH", host)
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
        self.scratch(&Scratch {
            branch: &[(file, SOURCE)],
            ..Scratch::default()
        })
    }

    /// A scratch repository of two commits made as `scratch` says, with
    /// its settings written last, where only the gate's own git reads them.
    fn scratch(&self, scratch: &Scratch) -> PathBuf {
        let repo = self.path("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "--quiet"]);
        write(&repo, &[("README.md", b"base\n")]);
        write(&repo, scratch.base);
        git(&repo, &["add", "--all"]);
        git(&repo, &["commit", "--quiet", "-m", "base"]);
        write(&repo, scratch.branch);
        if let Some((from, to)) = scratch.rename {
            git(&repo, &["mv", from, to]);
        }
        if let Some(file) = scratch.chmod {
            let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
            std::fs::set_permissions(repo.join(file), mode).unwrap();
        }
        git(&repo, &["add", "--all"]);
        git(&repo, &["commit", "--quiet", "-m", "branch"]);
        for (key, value) in scratch.config {
            git(&repo, &["config", key, value]);
        }
        repo
    }
}

/// How a scratch repository's two commits are made: the files the base
/// holds, the files the branch writes, a path it renames and one it marks
/// executable, and the settings git then reads from the repository.
#[derive(Default)]
struct Scratch<'a> {
    base: &'a [(&'a str, &'a [u8])],
    branch: &'a [(&'a str, &'a [u8])],
    rename: Option<(&'a str, &'a str)>,
    chmod: Option<&'a str>,
    config: &'a [(&'a str, &'a str)],
}

fn write(repo: &Path, files: &[(&str, &[u8])]) {
    for (file, bytes) in files {
        let path = repo.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
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
    assert!(text(&output.stdout).contains("misses this diff adds: 0 (1 missed,"));
}

#[test]
fn a_second_miss_of_a_committed_file_and_mutation_fails_the_gate() {
    let output = Gate::new().verdict(&[KNOWN_MOVED, KNOWN_AGAIN]);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    let stdout = text(&output.stdout);
    assert!(stdout.contains("misses this diff adds: 1\n"), "{stdout}");
    let stderr = text(&output.stderr);
    assert!(stderr.contains("this diff adds 1 brokkr-core"), "{stderr}");
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

/// The gate over a scratch branch whose planted hunk the stub lists, so
/// the gate measures the change only when the diff carries it as a hunk.
fn measured(gate: &Gate, repo: &Path, env: &[(&str, &str)]) -> Output {
    let mut env = env.to_vec();
    env.extend([
        ("STUB_LIST_NAMING", "+pub fn planted() {}"),
        ("STUB_LISTED", "a listed mutant\n"),
    ]);
    gate.run_in(repo, &["gate", "HEAD~1", "brokkr-core"], &env)
}

/// The #420 landing's first hold: a NUL byte in a core comment, or a
/// `-diff` attribute, made git print `Binary files ... differ` in place of
/// a hunk, cargo-mutants listed nothing, and the gate passed. Every setting
/// that would render a change as anything but a hunk is overridden, so the
/// change is measured whatever git is configured to do.
#[test]
fn the_diff_is_a_hunk_whatever_git_is_configured_to_do() {
    const OLD: &str = "crates/brokkr-core/src/old.rs";
    let planted: &[(&str, &[u8])] = &[(PLANTED, SOURCE)];
    let cases: [(&str, Scratch); 7] = [
        (
            "a NUL byte in a comment",
            Scratch {
                branch: &[(PLANTED, b"pub fn planted() {} // \0\n")],
                ..Scratch::default()
            },
        ),
        (
            "a -diff attribute",
            Scratch {
                base: &[(".gitattributes", b"crates/brokkr-core/** -diff\n")],
                branch: planted,
                ..Scratch::default()
            },
        ),
        (
            "a textconv",
            Scratch {
                base: &[(".gitattributes", b"*.rs diff=convert\n")],
                branch: planted,
                config: &[("diff.convert.textconv", "sed s/planted/converted/")],
                ..Scratch::default()
            },
        ),
        (
            "an external diff",
            Scratch {
                branch: planted,
                config: &[("diff.external", "true")],
                ..Scratch::default()
            },
        ),
        (
            "colour",
            Scratch {
                branch: planted,
                config: &[("color.diff", "always")],
                ..Scratch::default()
            },
        ),
        (
            "no prefixes",
            Scratch {
                branch: planted,
                config: &[("diff.noprefix", "true")],
                ..Scratch::default()
            },
        ),
        (
            "a pure rename",
            Scratch {
                base: &[(OLD, SOURCE)],
                rename: Some((OLD, PLANTED)),
                ..Scratch::default()
            },
        ),
    ];
    for (case, scratch) in cases {
        let gate = Gate::new();
        let output = measured(&gate, &gate.scratch(&scratch), &[]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{case}: {}",
            text(&output.stderr)
        );
        let stdout = text(&output.stdout);
        assert!(
            stdout.contains("misses this diff adds: 0 (0 missed,"),
            "{case}: {stdout}"
        );
    }
}

/// What git still leaves out of the diff is refused before anything is
/// listed: a binary marker by its line, and a changed scope path no hunk
/// heads by its name (no diff, another file's hunk, or headers with no
/// hunk under them). Each planted render would otherwise list a mutant and
/// pass.
#[test]
fn a_change_the_diff_does_not_carry_as_a_hunk_is_refused_before_listing() {
    let header = "diff --git a/crates/brokkr-core/src/planted.rs \
        b/crates/brokkr-core/src/planted.rs\nnew file mode 100644\n";
    let marker = "Binary files /dev/null and b/crates/brokkr-core/src/planted.rs differ";
    let other = PLANTED_HUNK.replace("planted.rs", "other.rs");
    let headers_only = format!("{header}--- /dev/null\n+++ b/{PLANTED}\n");
    let uncarried = format!("carries no hunk for these changes in the scope:\n  {PLANTED}\n");
    let cases = [
        (
            format!("{header}index 0000000..1111111\n{marker}\n"),
            marker.to_string(),
        ),
        (
            format!("{header}GIT binary patch\nliteral 20\nbcmZ?wbhEHb00000\n\n"),
            "\n  GIT binary patch\n".to_string(),
        ),
        (String::new(), uncarried.clone()),
        (other, uncarried.clone()),
        (headers_only, uncarried),
    ];
    for (render, reason) in cases {
        let gate = Gate::new();
        let repo = gate.branch(PLANTED);
        let output = measured(&gate, &repo, &[("STUB_RENDER", &render)]);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{render:?}: {stderr}");
        assert!(stderr.contains(&reason), "{render:?}: {stderr}");
        assert!(!gate.path("argv").exists(), "{render:?} was listed");
    }
}

/// Only the scope's changes must head a hunk: a change the gate does not
/// measure is left to the diff as git renders it, so it never fails the
/// required check.
#[test]
fn a_change_outside_the_scope_needs_no_hunk() {
    let gate = Gate::new();
    let repo = gate.scratch(&Scratch {
        branch: &[("README.md", b"changed\n"), (PLANTED, SOURCE)],
        ..Scratch::default()
    });
    let output = measured(&gate, &repo, &[("STUB_RENDER", PLANTED_HUNK)]);
    assert_eq!(output.status.code(), Some(0), "{}", text(&output.stderr));
    assert!(text(&output.stdout).contains("misses this diff adds: 0 (0 missed,"));
}

/// A change with nothing to mutate needs no hunk: a file whose mode alone
/// changed, and an empty file added. Both pass with nothing listed.
#[test]
fn a_change_with_nothing_to_mutate_needs_no_hunk() {
    let cases: [(&str, Scratch); 2] = [
        (
            "a mode change",
            Scratch {
                base: &[(PLANTED, SOURCE)],
                chmod: Some(PLANTED),
                ..Scratch::default()
            },
        ),
        (
            "an empty file",
            Scratch {
                branch: &[("crates/brokkr-core/src/empty.rs", b"")],
                ..Scratch::default()
            },
        ),
    ];
    for (case, scratch) in cases {
        let gate = Gate::new();
        let repo = gate.scratch(&scratch);
        let output = gate.run_in(
            &repo,
            &["gate", "HEAD~1", "brokkr-core"],
            &[("STUB_STATUS", "99")],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{case}: {}",
            text(&output.stderr)
        );
        assert!(
            text(&output.stdout).contains("brokkr-core mutants in this diff: 0"),
            "{case}"
        );
    }
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
    let misses = ("STUB_STATUS", "2");
    let no_miss = "cargo mutants exited 2, but";
    let not_a_list = "mutants.out/mutants.json is not a list of mutants";
    // The #420 landing's second hold: `[{bad}]` passed a check that read
    // only the brackets. jq now parses the file whole.
    let cases: [Failure; 15] = [
        (&[listed, ("STUB_JSON", "[{bad}]")], 1, not_a_list),
        (
            &[listed, ("STUB_JSON", "[{\"name\":broken}]")],
            1,
            not_a_list,
        ),
        (&[listed, ("STUB_JSON", "[1]")], 1, not_a_list),
        (&[listed, ("STUB_JSON", "{\"a\":{}}")], 1, not_a_list),
        (&[listed, ("STUB_JSON", "[{}] [{}]")], 1, not_a_list),
        (&[listed, ("STUB_STATUS", "4")], 4, "cargo mutants exited 4"),
        (&[listed, misses], 1, no_miss),
        (&[listed, misses, ("STUB_MISSED", " \n\n")], 1, no_miss),
        (&[listed, ("STUB_JSON", "")], 1, not_a_list),
        (&[listed, ("STUB_JSON", "not json\n")], 1, not_a_list),
        (&[listed, ("STUB_JSON", "[{\"name\":")], 1, not_a_list),
        (
            &[listed, ("STUB_JSON", "[ ]\n")],
            1,
            "found no mutant to test",
        ),
        (
            &[listed, ("STUB_STATUS", "2"), ("STUB_OMIT", "missed.txt")],
            1,
            "mutants.out/missed.txt is missing",
        ),
        (
            &[listed, ("STUB_STATUS", "2"), ("STUB_OMIT", "mutants.json")],
            1,
            "mutants.out/mutants.json is missing",
        ),
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

/// A list that fails ends the gate with cargo-mutants' own code, before
/// any mutant is run: the stub's argument log holds the one `--list` call.
#[test]
fn a_failed_list_fails_the_gate_before_any_run() {
    let gate = Gate::new();
    let output = gate.run(
        &["gate", "HEAD", "brokkr-core"],
        &[
            ("STUB_LISTED", "a listed mutant\n"),
            ("STUB_LIST_STATUS", "5"),
        ],
    );
    assert_eq!(output.status.code(), Some(5), "{}", text(&output.stderr));
    let calls = std::fs::read_to_string(gate.path("argv")).unwrap();
    assert_eq!(calls.lines().count(), 1, "{calls}");
    assert!(calls.contains("--list"), "{calls}");
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

/// Without jq the gate cannot read mutants.json, so it fails before it
/// calls anything: here nothing but bash is reachable, and git's absence
/// would otherwise surface later as 127.
#[test]
fn a_host_without_jq_fails_before_anything_runs() {
    let gate = Gate::new();
    let bash = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|dir| dir.join("bash"))
        .find(|path| path.is_file())
        .expect("bash on PATH");
    let output = Command::new(bash)
        .arg(workspace().join("scripts/mutants.sh"))
        .args(["gate", "HEAD", "brokkr-core"])
        .current_dir(workspace())
        .env("PATH", gate.path("allow"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1), "{}", text(&output.stderr));
    assert!(text(&output.stderr).contains("jq is required"));
}

/// A base the gate cannot diff against measures nothing, so it fails with
/// git's own code, before cargo-mutants is called at all.
#[test]
fn a_base_git_cannot_resolve_fails_the_gate() {
    let gate = Gate::new();
    let output = gate.run(&["gate", "no-such-base-ref", "brokkr-core"], &[]);
    assert_eq!(output.status.code(), Some(128), "{}", text(&output.stderr));
    assert!(!gate.path("argv").exists());
}

/// The conditions in a job's body, each as `if: <value>`, read at every
/// depth: a step's leading `- ` is dropped, and a key is read without its
/// spacing or quotes, so `if :` and `'if':` count. A flow collection or a
/// merge key could carry a condition this reading does not see, so either
/// one fails.
fn conditions(job: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in job.lines() {
        let line = line.trim_start();
        let line = line.strip_prefix("- ").unwrap_or(line).trim_start();
        assert!(
            !line.starts_with(['{', '[']),
            "a flow collection in the job: {line}"
        );
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let (key, value) = (key.trim().trim_matches(['"', '\'']), value.trim());
        assert_ne!(key, "<<", "a merge key in the job: {line}");
        assert!(
            !value.starts_with(['{', '[']),
            "a flow collection in the job: {line}"
        );
        if key == "if" {
            found.push(format!("if: {value}"));
        }
    }
    found
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
    // A skipped job or step reports success, which satisfies a required
    // check: the #420 landing set the job's condition to `false`, then gave
    // the gate step an `if:` of its own and the job a `needs:` on the
    // weekly job, and every test stayed green each time. The job runs on
    // exactly the event it gates, and its only other condition is the
    // artifact upload's. Conditions are read at every depth, a step's
    // leading `- if:` included.
    assert_eq!(
        conditions(&core),
        ["if: github.event_name == 'pull_request'", "if: always()"],
        "core-gate's conditions must be the pull-request event and the upload's:\n{core}"
    );
    assert!(core.contains("\n    if: github.event_name == 'pull_request'\n"));
    let keys: Vec<&str> = core
        .lines()
        .filter_map(|line| line.strip_prefix("    "))
        .filter(|line| !line.starts_with([' ', '#']))
        .filter_map(|line| line.split_once(':').map(|(key, _)| key))
        .collect();
    assert_eq!(
        keys,
        ["name", "if", "runs-on", "timeout-minutes", "steps"],
        "core-gate gained or lost a job key, such as `needs:`:\n{core}"
    );
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
