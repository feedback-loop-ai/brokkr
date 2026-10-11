//! The workspace suite as concurrent shards (#633): `scripts/test-shards.sh`
//! runs the tests `cargo test --workspace` runs, each test target exactly
//! once, the first phase's shards at once and capability_broker alone after
//! them. What it refuses is bound here: its shards are held to the real
//! tree's `cargo metadata`, a planted target no shard runs, one run twice,
//! a selector that names nothing or cannot be read is refused by name, and
//! with a `cargo` stub first on `PATH` the phases, the failures and a
//! filter that matches nothing are each shown.

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

use serde::{Deserialize, Serialize};

use crate::workspace_root::workspace;

const SCRIPT: &str = "scripts/test-shards.sh";

/// The part of `cargo metadata --format-version 1` the script reads, so a
/// fixture planted from it is written back with exactly those fields.
/// Cargo adds fields over time, so unknown ones are ignored.
#[derive(Clone, Deserialize, Serialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Package {
    name: String,
    targets: Vec<Target>,
}

/// One compiled root: its kinds as Cargo names them (`lib`, `test`, …),
/// an open vocabulary the script maps, and whether `cargo test` runs it
/// and its doctests.
#[derive(Clone, Deserialize, Serialize)]
struct Target {
    kind: Vec<String>,
    name: String,
    test: bool,
    doctest: bool,
}

/// The real tree's targets, as the script reads them.
fn metadata() -> Metadata {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(workspace())
        .output()
        .expect("cargo metadata runs");
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("cargo metadata is JSON")
}

/// A stand-in for `cargo`: `metadata` prints the planted metadata; every
/// other call is recorded, and a shard's call prints one libtest summary.
/// A first-phase shard waits until the whole first phase has started, so
/// a runner that ran them one at a time fails here; the second phase
/// fails unless the whole first phase has ended. `STUB_FAIL` names a word
/// whose shard fails, `STUB_EMPTY=2` makes the second phase run no test,
/// `STUB_PLANT` names a word whose shard prints `STUB_LINES` in place of
/// its summary and exits 0, and `STUB_KILL` one whose shard kills its
/// runner, the script's subshell, before the runner records its status,
/// and keeps running, not yet ended, until the scratch's `release` exists.
const STUB: &str = r#"#!/usr/bin/env bash
dir="$STUB_DIR"
if [ "$1" = metadata ]; then cat "$dir/metadata.json"; exit 0; fi
printf '%s\n' "$*" >> "$dir/calls"
case " $* " in *" --no-run "* | *" --list "*) exit 0 ;; esac
summary() {
  printf 'test result: %s. %s passed; %s failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n' "$1" "$2" "$3"
}
phase=1
case " $* " in *" --skip "*) ;; *" capability_broker:: "*) phase=2 ;; esac
if [ "$phase" = 1 ]; then
  : > "$dir/started/$$"
  tries=0
  while started=("$dir"/started/*); [ "${#started[@]}" -lt "$STUB_PHASE_ONE" ]; do
    tries=$((tries + 1))
    if [ "$tries" -ge 600 ]; then echo 'stub: the first phase did not start together'; summary FAILED 0 1; exit 101; fi
    sleep 0.05
  done
else
  ended=("$dir"/ended/*)
  if [ ! -e "${ended[0]}" ] || [ "${#ended[@]}" -ne "$STUB_PHASE_ONE" ]; then
    echo 'stub: capability_broker started beside the first phase'; summary FAILED 0 1; exit 101
  fi
fi
status=0
if [ -n "${STUB_FAIL:-}" ] && [[ " $* " == *" $STUB_FAIL "* ]]; then
  summary FAILED 0 1; status=101
elif [ -n "${STUB_PLANT:-}" ] && [[ " $* " == *" $STUB_PLANT "* ]]; then
  printf '%s' "$STUB_LINES"
elif [ "${STUB_EMPTY:-}" = "$phase" ]; then
  summary ok 0 0
else
  summary ok 1 0
fi
if [ -n "${STUB_KILL:-}" ] && [[ " $* " == *" $STUB_KILL "* ]]; then
  kill -KILL "$PPID"
  tries=0
  while [ ! -e "$dir/release" ] && [ "$tries" -lt 600 ]; do tries=$((tries + 1)); sleep 0.05; done
fi
if [ "$phase" = 1 ]; then : > "$dir/ended/$$"; fi
exit "$status"
"#;

/// A scratch directory with the stub as `cargo` in its `bin/`, `metadata`
/// as the stub's answer and `script` as the shard script.
struct Scratch {
    dir: tempfile::TempDir,
}

impl Scratch {
    fn new(metadata: &Metadata, script: &str) -> Self {
        let dir = tempfile::tempdir().expect("scratch");
        for sub in ["bin", "started", "ended", "tmp"] {
            std::fs::create_dir(dir.path().join(sub)).expect("scratch directory");
        }
        let cargo = dir.path().join("bin/cargo");
        std::fs::write(&cargo, STUB).expect("stub cargo");
        std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("mode");
        let metadata = serde_json::to_string(metadata).expect("metadata serializes");
        std::fs::write(dir.path().join("metadata.json"), metadata).expect("metadata");
        std::fs::write(dir.path().join("test-shards.sh"), script).expect("script");
        Self { dir }
    }

    /// The script run from the workspace with `args`, the stub first on
    /// `PATH` and the scratch `tmp/` as `TMPDIR`, expecting `phase_one`
    /// shards in its first phase.
    fn run(&self, args: &[&str], phase_one: usize, env: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.path().join("bin").display(),
            std::env::var("PATH").expect("PATH")
        );
        Command::new("bash")
            .arg(self.dir.path().join("test-shards.sh"))
            .args(args)
            .current_dir(workspace())
            .env("PATH", path)
            .env("STUB_DIR", self.dir.path())
            .env("TMPDIR", self.dir.path().join("tmp"))
            .env("STUB_PHASE_ONE", phase_one.to_string())
            .envs(env.iter().copied())
            .output()
            .expect("the script runs")
    }

    /// Every call the stub recorded but `metadata`, sorted.
    fn calls(&self) -> Vec<String> {
        let calls = std::fs::read_to_string(self.dir.path().join("calls")).unwrap_or_default();
        let mut calls: Vec<String> = calls.lines().map(str::to_string).collect();
        calls.sort();
        calls
    }
}

fn script() -> String {
    std::fs::read_to_string(workspace().join(SCRIPT)).expect("the shard script")
}

/// The test mode's shard commands after `cargo`, as the plan prints them.
const SHARDS: [&str; 6] = [
    "--lib",
    "--test it -- --skip capability_broker::",
    "--test heap_claude --test heap_codex --test heap_dsh",
    "--bin brokkr --test driver_conformance --test transcript_surfaces --test hands_exits \
     --test secret_drop --test capability_launch --test operated_repo --test queued_launch \
     --test seatbelt_lifetime_probe",
    "--doc",
    "--test it -- capability_broker::",
];

/// The real tree's shards are its whole suite, each target once: the
/// plan passes the partition check in both modes and prints each shard's
/// selection, the coverage mode without the doctests cargo-llvm-cov never
/// runs.
#[test]
fn the_real_trees_shards_are_its_whole_suite() {
    let names = [
        "phase 1, every library",
        "phase 1, the it binaries, capability_broker skipped",
        "phase 1, the heap measurements",
        "phase 1, the other own-process binaries",
        "phase 1, the doctests",
        "phase 2, capability_broker, alone",
    ];
    let plan: Vec<String> = names
        .iter()
        .zip(SHARDS)
        .map(|(name, shard)| format!("{name}: {shard}\n"))
        .collect();
    for (mode, expected) in [
        ("test", plan.concat()),
        ("coverage", [&plan[..4], &plan[5..]].concat().concat()),
    ] {
        let run = Command::new("bash")
            .args([SCRIPT, "plan", mode])
            .current_dir(workspace())
            .output()
            .expect("the plan runs");
        assert_eq!(run.status.code(), Some(0), "{mode}: {run:?}");
        assert_eq!(String::from_utf8_lossy(&run.stdout), expected, "{mode}");
    }
}

/// brokkr-cli's targets in `metadata`, to plant a target in or change.
fn cli_targets(metadata: &mut Metadata) -> &mut Vec<Target> {
    &mut metadata
        .packages
        .iter_mut()
        .find(|package| package.name == "brokkr-cli")
        .expect("brokkr-cli")
        .targets
}

/// The planted metadata or table, the plan's exact refusal and its
/// closing line.
fn assert_refused(metadata: &Metadata, script: &str, problems: &[&str]) {
    let run = Scratch::new(metadata, script).run(&["plan", "test"], 0, &[]);
    let expected: String = problems
        .iter()
        .map(|problem| format!("{problem}\n"))
        .chain([
            "test shards: the shards are not the whole suite, each test target \
                 exactly once\n"
                .to_string(),
        ])
        .collect();
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stderr), expected);
}

/// The partition check refuses, by name, a test target no shard runs (a
/// new `[[test]]`, and one a shard's selection lost), one two shards run,
/// a split whose skip and filter differ, a selector that names no target
/// or that the plan cannot read, and a shard that runs a target the suite
/// does not.
#[test]
fn a_target_no_shard_runs_or_two_shards_run_is_refused_by_name() {
    let real = metadata();
    let table = script();
    let mut planted = real.clone();
    cli_targets(&mut planted).push(Target {
        kind: vec!["test".to_string()],
        name: "a_new_binary".to_string(),
        test: true,
        doctest: false,
    });
    assert_refused(
        &planted,
        &table,
        &["no shard runs brokkr-cli test a_new_binary"],
    );
    let lost = table.replace("test:heap_codex test:heap_dsh", "test:heap_codex");
    assert_refused(&real, &lost, &["no shard runs brokkr-cli test heap_dsh"]);
    let twice = table.replace("bin:brokkr ", "bin:brokkr test:heap_dsh ");
    assert_refused(
        &real,
        &twice,
        &[
            "brokkr-cli test heap_dsh does not run exactly once: the heap measurements (none), \
           the other own-process binaries (none)",
        ],
    );
    let split = table.replace(
        "skip:capability_broker::",
        "skip:capability_broker::observer::",
    );
    let unsplit: Vec<String> = [
        "brokkr-core",
        "brokkr-store",
        "brokkr-runtime",
        "brokkr-view",
        "brokkr-cli",
    ]
    .iter()
    .map(|package| {
        format!(
            "{package} test it does not run exactly once: the it binaries, capability_broker \
                 skipped (skip:capability_broker::observer::), capability_broker, alone \
                 (only:capability_broker::)"
        )
    })
    .collect();
    assert_refused(
        &real,
        &split,
        &unsplit.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let unread = table.replace("test:heap_claude ", "tests:heap_claude ");
    assert_refused(
        &real,
        &unread,
        &[
            "the heap measurements: a selector the plan cannot read: tests:heap_claude",
            "no shard runs brokkr-cli test heap_claude",
        ],
    );
    let mut removed = real.clone();
    cli_targets(&mut removed).retain(|target| target.name != "heap_dsh");
    assert_refused(
        &removed,
        &table,
        &["the heap measurements: test:heap_dsh selects no target"],
    );
    let mut untested = real.clone();
    for target in cli_targets(&mut untested)
        .iter_mut()
        .filter(|target| target.name == "heap_dsh")
    {
        target.test = false;
    }
    assert_refused(
        &untested,
        &table,
        &["the heap measurements runs brokkr-cli test heap_dsh, which cargo test --workspace does not"],
    );
}

/// Each mode builds as it can and runs every shard once: the first
/// phase's shards at once (the stub fails a shard that starts alone) and
/// capability_broker only after all of them ended. Each mode builds every
/// binary first: the test mode with `--no-run`, the coverage mode, which
/// runs through cargo-llvm-cov with no doctests, by listing every binary.
#[test]
fn the_first_phase_runs_at_once_and_capability_broker_alone_after_it() {
    let real = metadata();
    let test = "test --workspace --all-features --locked --no-fail-fast";
    let coverage = "+nightly-2026-09-05 llvm-cov --no-report --workspace --all-features --locked \
                    --branch --no-fail-fast";
    let test_calls: Vec<String> = ["--no-run"]
        .iter()
        .chain(&SHARDS)
        .map(|shard| format!("{test} {shard}"))
        .collect();
    let coverage_calls: Vec<String> = ["-- --list"]
        .iter()
        .chain(&SHARDS[..4])
        .chain(&SHARDS[5..])
        .map(|shard| format!("{coverage} {shard}"))
        .collect();
    for (args, phase_one, mut expected) in [
        (&["test"][..], 5, test_calls),
        (&["coverage", "nightly-2026-09-05"][..], 4, coverage_calls),
    ] {
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(args, phase_one, &[]);
        assert_eq!(run.status.code(), Some(0), "{args:?}: {run:?}");
        expected.sort();
        assert_eq!(scratch.calls(), expected, "{args:?}");
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            stdout.contains("test shards: phase 2, capability_broker, alone: exit 0 after "),
            "{stdout}"
        );
        // A settled run removes its work directory: nothing is left in TMPDIR.
        let left: Vec<std::ffi::OsString> = std::fs::read_dir(scratch.dir.path().join("tmp"))
            .expect("scratch tmp")
            .map(|entry| entry.expect("scratch tmp entry").file_name())
            .collect();
        assert_eq!(left, Vec::<std::ffi::OsString>::new(), "{args:?}");
    }
}

/// The test mode run over `metadata` with `env` fails with exactly
/// `refusal`, only after every shard ran, the later phase included; its
/// scratch and its stdout.
fn refused_after_every_shard(
    metadata: &Metadata,
    env: &[(&str, &str)],
    refusal: &str,
) -> (Scratch, String) {
    let scratch = Scratch::new(metadata, &script());
    let run = scratch.run(&["test"], 5, env);
    assert_eq!(run.status.code(), Some(1), "{env:?}: {run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stderr), refusal, "{env:?}");
    assert_eq!(scratch.calls().len(), SHARDS.len() + 1, "{env:?}");
    (scratch, String::from_utf8_lossy(&run.stdout).into_owned())
}

/// A failing shard fails the run only after every shard has ended, the
/// later phase included, and is named with its exit; a filtered shard
/// whose run executes no test is refused, since its filter matches none.
#[test]
fn a_failed_shard_or_an_empty_filter_fails_the_run_after_every_shard() {
    let real = metadata();
    for (env, refusal) in [
        (
            ("STUB_FAIL", "--lib"),
            "test shards: every library failed with exit 101\n",
        ),
        (
            ("STUB_EMPTY", "2"),
            "test shards: capability_broker, alone executed no test, so its filter matches none\n",
        ),
    ] {
        refused_after_every_shard(&real, &[env], refusal);
    }
    let run = Scratch::new(&real, &script()).run(&["coverage", "nightly"], 0, &[]);
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        "test shards: the coverage toolchain 'nightly' is not nightly-YYYY-MM-DD\n"
    );
}

/// A readable summary showing a test executed.
const ONE_TEST: &str = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; \
                        0 filtered out; finished in 0.00s\n";

/// `test result:` lines in shapes the grammar cannot read.
const UNREAD: &str = "test result: everything is fine\n";
const UNKNOWN: &str = "test result: UNKNOWN. unreadable\n";

/// Every shard's summaries are ruled on as scripts/run-it-tests.sh rules on
/// a run's, though the shard exits 0: a filtered shard with a summary the
/// grammar cannot read is refused beside a readable executed one, before it
/// or after it, and an unfiltered shard is refused when it prints no summary
/// or only one the grammar cannot read.
#[test]
fn every_shards_summaries_are_read_and_an_unread_or_missing_one_refuses() {
    let real = metadata();
    let unread = "printed a test result: summary the guard cannot read, so no test is \
                  shown to have run";
    let skipped = "the it binaries, capability_broker skipped";
    for (word, lines, shard, refusal) in [
        ("--skip", [UNREAD, ONE_TEST].concat(), skipped, unread),
        ("--skip", [ONE_TEST, UNKNOWN].concat(), skipped, unread),
        (
            "--lib",
            "running the library\n".to_string(),
            "every library",
            "printed no test result: summary, so no test is shown to have run",
        ),
        ("--lib", UNREAD.to_string(), "every library", unread),
    ] {
        let env = [("STUB_PLANT", word), ("STUB_LINES", lines.as_str())];
        refused_after_every_shard(&real, &env, &format!("test shards: {shard} {refusal}\n"));
    }
}

/// The stub's calls and how many first-phase shards ended, once the
/// scratch's `release` has let an orphaned shard command end.
fn after_release(scratch: &Scratch) -> (Vec<String>, usize) {
    std::fs::write(scratch.dir.path().join("release"), "").expect("release");
    for _ in 0..600 {
        if ended(scratch) == SHARDS.len() - 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    (scratch.calls(), ended(scratch))
}

/// How many first-phase shard commands have ended.
fn ended(scratch: &Scratch) -> usize {
    std::fs::read_dir(scratch.dir.path().join("ended"))
        .expect("ended")
        .count()
}

/// A shard whose runner dies before it records the shard's exit status is
/// refused by name; its log is still printed, the other shards are still
/// collected, and the logs are kept, since the shard's command may outlive
/// its runner. For that reason the later phase is not started, even once
/// the orphaned command has ended: capability_broker could not be shown to
/// run alone.
#[test]
fn a_shard_whose_runner_dies_unrecorded_is_refused_and_its_logs_are_kept() {
    let scratch = Scratch::new(&metadata(), &script());
    let run = scratch.run(&["test"], 5, &[("STUB_KILL", "--lib")]);
    let running = ended(&scratch);
    let (calls, finally) = after_release(&scratch);
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        "test shards: every library recorded no exit status (its runner ended with exit 137), \
         so it is not shown to have passed\n\
         test shards: phase 2 was not started: an earlier runner recorded no exit status, so its \
         command may still be running and the phase cannot be shown to run alone\n"
    );
    assert_eq!((running, finally), (SHARDS.len() - 2, SHARDS.len() - 1));
    let mut first_phase: Vec<String> = ["--no-run"]
        .iter()
        .chain(&SHARDS[..5])
        .map(|shard| format!("test --workspace --all-features --locked --no-fail-fast {shard}"))
        .collect();
    first_phase.sort();
    assert_eq!(calls, first_phase);
    let stdout = String::from_utf8_lossy(&run.stdout);
    let work: Vec<_> = std::fs::read_dir(scratch.dir.path().join("tmp"))
        .expect("the scratch TMPDIR")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert_eq!(work.len(), 1, "{work:?}");
    let shown = format!(
        "\n==> every library (phase 1): no exit status recorded, its runner ended with exit 137; \
         the logs are kept in {}\n{ONE_TEST}",
        work[0].display()
    );
    for line in [
        shown.as_str(),
        "\n==> the doctests (phase 1): exit 0 after ",
        "test shards: phase 1, every library: no exit status recorded\n",
        "test shards: phase 2, capability_broker, alone: not started\n",
    ] {
        assert!(stdout.contains(line), "{line}\n{stdout}");
    }
    assert!(work[0].join("0.log").is_file(), "{work:?}");
}
