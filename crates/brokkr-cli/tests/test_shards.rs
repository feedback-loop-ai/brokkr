//! The workspace suite as shards (#633): `scripts/test-shards.sh` runs the
//! tests `cargo test --workspace` runs, each test target exactly once, one
//! shard after another and capability_broker last. What it refuses is
//! bound here: its shards are held to the real tree's `cargo metadata`, a
//! planted target no shard runs, one run twice, a selector that names
//! nothing or cannot be read is refused by name, and with a `cargo` stub
//! first on `PATH` the order, the failures and a filter that matches
//! nothing are each shown.

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
/// A shard's call also records `start` and `end` with its arguments in
/// `events`, and takes a moment between them, so two shards run at once
/// show as interleaved, and records in `machine` the BROKKR_TEST_MACHINE it
/// inherited, or `unset`. `STUB_FAIL` names a word whose shard fails,
/// `STUB_EMPTY` one whose shard runs no test, `STUB_PLANT` one whose shard
/// prints `STUB_LINES` in place of its summary and exits 0, and
/// `STUB_KILL` one whose shard kills its runner, the script's subshell,
/// before the runner records its status, and keeps running, not yet
/// ended, until the scratch's `release` exists. `STUB_RECORD` names a word
/// whose shard writes the scratch's `record` as its own status record, the
/// one whose log has none yet, and kills its runner before the runner
/// writes one. Each shard's call also
/// records in `signals` how a SIGINT it raises ends a child shell: 130
/// when SIGINT keeps its default, 0 when it was left ignored.
const STUB: &str = r#"#!/usr/bin/env bash
dir="$STUB_DIR"
if [ "$1" = metadata ]; then cat "$dir/metadata.json"; exit 0; fi
printf '%s\n' "$*" >> "$dir/calls"
case " $* " in *" --no-run "* | *" --list "*) exit 0 ;; esac
summary() {
  printf 'test result: %s. %s passed; %s failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n' "$1" "$2" "$3"
}
names() { local word="$1"; shift; [ -n "$word" ] && [[ " $* " == *" $word "* ]]; }
printf 'start %s\n' "$*" >> "$dir/events"
printf '%s\n' "${BROKKR_TEST_MACHINE-unset}" >> "$dir/machine"
sh -c 'kill -INT $$; exit 0'
printf '%s %s\n' "$?" "$*" >> "$dir/signals"
sleep 0.2
status=0
if names "${STUB_FAIL:-}" "$@"; then
  summary FAILED 0 1; status=101
elif names "${STUB_PLANT:-}" "$@"; then
  printf '%s' "$STUB_LINES"
elif names "${STUB_EMPTY:-}" "$@"; then
  summary ok 0 0
else
  summary ok 1 0
fi
if names "${STUB_KILL:-}" "$@"; then
  kill -KILL "$PPID"
  tries=0
  while [ ! -e "$dir/release" ] && [ "$tries" -lt 600 ]; do tries=$((tries + 1)); sleep 0.05; done
fi
if names "${STUB_RECORD:-}" "$@"; then
  for log in "$TMPDIR"/brokkr-test-shards.*/*.log; do
    [ -e "${log%.log}.status" ] || cp "$dir/record" "${log%.log}.status"
  done
  kill -KILL "$PPID"
fi
printf 'end %s\n' "$*" >> "$dir/events"
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
        for sub in ["bin", "tmp"] {
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
    /// `PATH` and the scratch `tmp/` as `TMPDIR`.
    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.command(args, env).output().expect("the script runs")
    }

    /// The command [`Scratch::run`] runs. A protected system set the job
    /// running these tests named is not this run's: only `env` names one.
    fn command(&self, args: &[&str], env: &[(&str, &str)]) -> Command {
        let path = format!(
            "{}:{}",
            self.dir.path().join("bin").display(),
            std::env::var("PATH").expect("PATH")
        );
        let mut command = Command::new("bash");
        command
            .arg(self.dir.path().join("test-shards.sh"))
            .args(args)
            .current_dir(workspace())
            .env("PATH", path)
            .env("STUB_DIR", self.dir.path())
            .env("TMPDIR", self.dir.path().join("tmp"))
            .env_remove("BROKKR_SYSTEM_SET")
            .env_remove("BROKKR_SYSTEM_SET_WAIT_SECONDS")
            .env_remove("BROKKR_TEST_MACHINE")
            .envs(env.iter().copied());
        command
    }

    /// The lines the stub wrote to `file`, in order.
    fn lines(&self, file: &str) -> Vec<String> {
        let lines = std::fs::read_to_string(self.dir.path().join(file)).unwrap_or_default();
        lines.lines().map(str::to_string).collect()
    }

    /// Every call the stub recorded but `metadata`, in order.
    fn calls(&self) -> Vec<String> {
        self.lines("calls")
    }

    /// Each shard call's `start` and `end`, in order.
    fn events(&self) -> Vec<String> {
        self.lines("events")
    }
}

/// The `start` and `end` of each of `shards` run after `prefix`, one
/// shard after another.
fn one_after_another(prefix: &str, shards: &[&str]) -> Vec<String> {
    shards
        .iter()
        .flat_map(|shard| {
            [
                format!("start {prefix} {shard}"),
                format!("end {prefix} {shard}"),
            ]
        })
        .collect()
}

fn script() -> String {
    std::fs::read_to_string(workspace().join(SCRIPT)).expect("the shard script")
}

/// The test mode's arguments to `cargo` before a shard's selection.
const TEST: &str = "test --workspace --all-features --locked --no-fail-fast";

/// The coverage mode's arguments to `cargo` before a shard's selection.
const COVERAGE: &str = "+nightly-2026-09-05 llvm-cov --no-report --workspace --all-features \
                        --locked --branch --no-fail-fast";

/// Each mode that runs shards: its arguments, its arguments to `cargo`
/// before a shard's selection, and its build's selection.
const MODES: [(&[&str], &str, &str); 2] = [
    (&["test"], TEST, "--no-run"),
    (&["coverage", "nightly-2026-09-05"], COVERAGE, "-- --list"),
];

/// The shards' names, in plan order.
const NAMES: [&str; 6] = [
    "every library",
    "the it binaries, capability_broker skipped",
    "the heap measurements",
    "the other own-process binaries",
    "the doctests",
    "capability_broker, alone",
];

/// The test mode's shard commands after `cargo`, as the plan prints them.
const SHARDS: [&str; 6] = [
    "--lib",
    "--test it -- --skip capability_broker::",
    "--test heap_claude --test heap_codex --test heap_dsh",
    "--bin brokkr --test driver_conformance --test transcript_surfaces --test hands_exits \
     --test secret_drop --test capability_launch --test operated_repo --test queued_launch \
     --test seatbelt_lifetime_probe --test summary_heap --test heap_fleet --test heap_open \
     --test heap_run_view --test heap_seat_view --test heap_costs",
    "--doc",
    "--test it -- capability_broker::",
];

/// The real tree's shards are its whole suite, each target once: the
/// plan passes the partition check in both modes and prints each shard's
/// selection, the coverage mode without the doctests cargo-llvm-cov never
/// runs.
#[test]
fn the_real_trees_shards_are_its_whole_suite() {
    let plan: Vec<String> = NAMES
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
            .env_remove("BROKKR_TEST_MACHINE")
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
    let run = Scratch::new(metadata, script).run(&["plan", "test"], &[]);
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
/// A metadata read that is not exactly one JSON object with a packages
/// array is refused before the partition is accepted: jq evaluates its
/// filter once per value it reads, so output with no value would pass every
/// check without reading a target, and two values would be read twice.
#[test]
fn metadata_that_is_not_one_json_object_is_refused() {
    let real = metadata();
    for planted in ["", " \n\t\n", "{}", "{\"packages\":[]}{\"packages\":[]}"] {
        let scratch = Scratch::new(&real, &script());
        std::fs::write(scratch.dir.path().join("metadata.json"), planted).expect("planted");
        for args in [
            &["plan", "test"][..],
            &["plan", "coverage"][..],
            &["test"][..],
        ] {
            let run = scratch.run(args, &[]);
            assert_eq!(run.status.code(), Some(1), "{planted:?} {args:?}: {run:?}");
            assert_eq!(
                String::from_utf8_lossy(&run.stderr),
                "test shards: cargo metadata could not be read\n",
                "{planted:?} {args:?}"
            );
        }
    }
}

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

/// Each mode builds every binary first, the test mode with `--no-run`,
/// the coverage mode, which runs through cargo-llvm-cov with no doctests,
/// by listing every binary, and then runs every shard once, in plan order,
/// one after another: no shard starts before the one before it ended, so
/// capability_broker, last, runs alone.
#[test]
fn every_shard_runs_alone_one_after_another_in_plan_order() {
    let real = metadata();
    let coverage_shards: Vec<&str> = SHARDS[..4].iter().chain(&SHARDS[5..]).copied().collect();
    for ((args, prefix, build), shards) in MODES.into_iter().zip([SHARDS.to_vec(), coverage_shards])
    {
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(args, &[]);
        assert_eq!(
            scratch.events(),
            one_after_another(prefix, &shards),
            "{args:?}"
        );
        assert_eq!(run.status.code(), Some(0), "{args:?}: {run:?}");
        let calls: Vec<String> = [build]
            .iter()
            .chain(&shards)
            .map(|shard| format!("{prefix} {shard}"))
            .collect();
        assert_eq!(scratch.calls(), calls, "{args:?}");
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            stdout.contains("test shards: capability_broker, alone: exit 0 after "),
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
/// `refusal`, only after every shard ran, each in its turn.
fn refused_after_every_shard(metadata: &Metadata, env: &[(&str, &str)], refusal: &str) {
    let scratch = Scratch::new(metadata, &script());
    let run = scratch.run(&["test"], env);
    assert_eq!(run.status.code(), Some(1), "{env:?}: {run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stderr), refusal, "{env:?}");
    assert_eq!(
        scratch.events(),
        one_after_another(TEST, &SHARDS),
        "{env:?}"
    );
}

/// A shard's command receives SIGINT as a terminal sends it, never left
/// ignored as a non-interactive shell leaves a command it starts in the
/// background: a test that raises SIGINT on a child, as run_view's
/// ctrl_c test does, sees the child end by the signal (128 plus 2).
#[test]
fn every_shard_receives_sigint_with_its_default_disposition() {
    let real = metadata();
    let scratch = Scratch::new(&real, &script());
    let run = scratch.run(&["test"], &[]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let signals =
        std::fs::read_to_string(scratch.dir.path().join("signals")).expect("signals recorded");
    let ended: Vec<&str> = signals
        .lines()
        .map(|line| line.split_once(' ').map_or(line, |(status, _)| status))
        .collect();
    assert_eq!(ended, vec!["130"; SHARDS.len()], "{signals}");
}

/// A failing shard, the first, fails the run only after every later shard
/// has run, and is named with its exit; a filtered shard whose run
/// executes no test is refused, since its filter matches none.
#[test]
fn a_failed_shard_or_an_empty_filter_fails_the_run_after_every_shard() {
    let real = metadata();
    for (env, refusal) in [
        (
            ("STUB_FAIL", "--lib"),
            "test shards: every library failed with exit 101\n",
        ),
        (
            ("STUB_EMPTY", "-- capability_broker::"),
            "test shards: capability_broker, alone executed no test, so its filter matches none\n",
        ),
    ] {
        refused_after_every_shard(&real, &[env], refusal);
    }
    let run = Scratch::new(&real, &script()).run(&["coverage", "nightly"], &[]);
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

/// The stub's events once the scratch's `release` has let an orphaned
/// shard command end, which it records last.
fn after_release(scratch: &Scratch, last: &str) -> Vec<String> {
    std::fs::write(scratch.dir.path().join("release"), "").expect("release");
    poll(|| scratch.events().last().map(String::as_str) == Some(last));
    scratch.events()
}

/// Returns once `done` holds, or after 30 seconds, looking every 50 ms.
fn poll(done: impl Fn() -> bool) {
    for _ in 0..600 {
        if done() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// A shard whose runner dies before it records the shard's exit status is
/// refused by name; its log is still printed, the shards before it are
/// still collected, and the logs are kept, since the shard's command may
/// outlive its runner. For that reason no later shard is started, even
/// once the orphaned command has ended: none could be shown to run alone.
#[test]
fn a_shard_whose_runner_dies_unrecorded_is_refused_and_its_logs_are_kept() {
    let scratch = Scratch::new(&metadata(), &script());
    let run = scratch.run(&["test"], &[("STUB_KILL", "heap_claude")]);
    let ran = one_after_another(TEST, &SHARDS[..3]);
    let running = scratch.events();
    let finally = after_release(&scratch, &ran[5]);
    assert_eq!((running.as_slice(), finally), (&ran[..5], ran.clone()));
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    let unstarted = not_started(&NAMES[3..]);
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!(
            "test shards: the heap measurements recorded no exit status (its runner ended with \
             exit 137), so it is not shown to have passed\n{unstarted}"
        )
    );
    let calls: Vec<String> = ["--no-run"]
        .iter()
        .chain(&SHARDS[..3])
        .map(|shard| format!("{TEST} {shard}"))
        .collect();
    assert_eq!(scratch.calls(), calls);
    let stdout = String::from_utf8_lossy(&run.stdout);
    let work: Vec<_> = std::fs::read_dir(scratch.dir.path().join("tmp"))
        .expect("the scratch TMPDIR")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert_eq!(work.len(), 1, "{work:?}");
    let shown = format!(
        "\n==> the heap measurements: no exit status recorded, its runner ended with exit 137; \
         the logs are kept in {}\n{ONE_TEST}",
        work[0].display()
    );
    for line in [
        shown.as_str(),
        "\n==> every library: exit 0 after ",
        "test shards: the heap measurements: no exit status recorded\n",
        "test shards: capability_broker, alone: not started\n",
    ] {
        assert!(stdout.contains(line), "{line}\n{stdout}");
    }
    assert!(work[0].join("2.log").is_file(), "{work:?}");
}

/// The refusal of each of `shards`, left unstarted after an earlier
/// runner recorded no exit status.
fn not_started(shards: &[&str]) -> String {
    shards
        .iter()
        .map(|shard| {
            format!(
                "test shards: {shard} was not started: an earlier runner recorded no exit \
                 status, so its command may still be running and the shard cannot be shown to \
                 run alone\n"
            )
        })
        .collect()
}

/// A shard's status record that is not exactly one record and its newline,
/// byte for byte, is no record (#647): one with a NUL bash would drop, a
/// second newline bash would strip, or a NUL in place of its newline is
/// refused in both modes as recorded by none, and no later shard starts.
/// The record written plainly, its runner killed alike, passes.
#[test]
fn a_shard_status_record_that_is_not_one_line_byte_for_byte_is_refused() {
    let real = metadata();
    for (args, prefix, build) in MODES {
        // The coverage mode runs no doctests.
        let later: Vec<&str> = NAMES[1..]
            .iter()
            .copied()
            .filter(|name| prefix == TEST || *name != "the doctests")
            .collect();
        for record in ["0\0 3\n", "0 3\n\n", "0 3\0", "0 3\n"] {
            let scratch = Scratch::new(&real, &script());
            std::fs::write(scratch.dir.path().join("record"), record).expect("record");
            let run = scratch.run(args, &[("STUB_RECORD", "--lib")]);
            let stderr = String::from_utf8_lossy(&run.stderr);
            if record == "0 3\n" {
                assert_eq!(
                    (run.status.code(), stderr.as_ref()),
                    (Some(0), ""),
                    "{args:?}"
                );
                continue;
            }
            assert_eq!(run.status.code(), Some(1), "{args:?} {record:?}: {run:?}");
            assert_eq!(
                stderr,
                format!(
                    "test shards: every library recorded no exit status (its runner ended with \
                     exit 137), so it is not shown to have passed\n{}",
                    not_started(&later)
                ),
                "{args:?} {record:?}"
            );
            assert_eq!(
                scratch.calls(),
                [format!("{prefix} {build}"), format!("{prefix} --lib")],
                "{args:?} {record:?}"
            );
        }
    }
}

/// A protected system set's preparation, as setup-bubblewrap's background
/// step leaves it in the scratch's `set/`: its `log`, and its `status`
/// renamed into place once written, as the step does.
fn prepared(scratch: &Scratch, log: &str, status: &str) {
    let set = scratch.dir.path().join("set");
    std::fs::write(set.join("log"), log).expect("log");
    std::fs::write(set.join("status.new"), status).expect("status");
    std::fs::rename(set.join("status.new"), set.join("status")).expect("status renamed");
}

/// The refusal of a protected system set's preparation that `failed` as
/// shown.
fn half_protected(failed: &str) -> String {
    format!(
        "test shards: the protected system set's preparation {failed}, so the boundary \
         proofs would read a set it may have left half protected\n"
    )
}

/// The run of `args` over the scratch's protected system set `set/`,
/// waiting at most `wait` seconds, refuses with exactly `refusal` after the
/// build `build` and before any shard, the set's log printed last.
fn refused_before_any_shard(
    scratch: &Scratch,
    args: &[&str],
    build: &str,
    wait: &str,
    refusal: &str,
) {
    let set = scratch.dir.path().join("set");
    let set_dir = set.to_str().expect("a UTF-8 scratch");
    let env = [
        ("BROKKR_SYSTEM_SET", set_dir),
        ("BROKKR_SYSTEM_SET_WAIT_SECONDS", wait),
    ];
    let run = scratch.run(args, &env);
    assert_eq!(run.status.code(), Some(1), "{args:?}: {run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stderr), refusal, "{args:?}");
    assert_eq!(scratch.calls(), [build], "{args:?}");
    assert_eq!(scratch.events(), Vec::<String>::new(), "{args:?}");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.ends_with("\nsetfacl: the set\n"), "{stdout}");
}

/// A protected system set whose preparation failed, never recorded its
/// status, recorded one the guard cannot read, or was never started is
/// refused in both modes after the build and before any shard runs, with
/// its log printed (#647); so is a wait limit that is not a number.
#[test]
fn an_unfinished_or_failed_system_set_preparation_refuses_before_any_shard() {
    let real = metadata();
    let failed = half_protected("failed with exit 1 after 3s");
    let unreadable = half_protected("recorded an exit status the guard cannot read");
    for (args, prefix, build) in MODES {
        let build = format!("{prefix} {build}");
        for (status, wait, refusal) in [
            (Some("1 3\n"), "600", failed.clone()),
            (
                None,
                "1",
                half_protected("recorded no exit status within 1s"),
            ),
            (Some("done\n"), "600", unreadable.clone()),
            // A status `$?` cannot be: past 64 bits a numeric test would
            // error and read as success, so each is refused as unreadable.
            (Some("9223372036854775808 3\n"), "600", unreadable.clone()),
            (Some("18446744073709551616 3\n"), "600", unreadable.clone()),
            // A second record after the first is no record: the whole file
            // is read, never its first line alone.
            (Some("0 3\n1 4\n"), "600", unreadable.clone()),
            (Some("00 3\n"), "600", unreadable.clone()),
            (Some("256 3\n"), "600", unreadable.clone()),
            // Bash drops a NUL and strips every trailing newline from what
            // it reads, so each would read as `0 3`: the file is compared
            // byte for byte with the one record and its newline.
            (Some("0\0 3\n"), "600", unreadable.clone()),
            (Some("0 3\n\n"), "600", unreadable.clone()),
            (Some("0 3\0"), "600", unreadable.clone()),
            (
                Some("0 3\n"),
                "soon",
                "test shards: BROKKR_SYSTEM_SET_WAIT_SECONDS 'soon' is not a number of seconds\n"
                    .to_string(),
            ),
        ] {
            let scratch = Scratch::new(&real, &script());
            let set = scratch.dir.path().join("set");
            std::fs::create_dir(&set).expect("set");
            match status {
                Some(status) => prepared(&scratch, "setfacl: the set\n", status),
                None => std::fs::write(set.join("log"), "setfacl: the set\n").expect("log"),
            }
            refused_before_any_shard(&scratch, args, &build, wait, &refusal);
        }
        // A status that cannot be read is refused as unreadable, its log
        // printed, never left to end the script at its redirection. Root
        // reads a mode-000 file, so where the tests run as root this case
        // is skipped.
        if !rustix::process::geteuid().is_root() {
            let scratch = Scratch::new(&real, &script());
            std::fs::create_dir(scratch.dir.path().join("set")).expect("set");
            prepared(&scratch, "setfacl: the set\n", "0 3\n");
            let status = scratch.dir.path().join("set/status");
            std::fs::set_permissions(&status, std::fs::Permissions::from_mode(0o000))
                .expect("mode");
            refused_before_any_shard(&scratch, args, &build, "600", &unreadable);
        }
    }
    let scratch = Scratch::new(&real, &script());
    let missing = scratch.dir.path().join("set");
    let run = scratch.run(
        &["test"],
        &[("BROKKR_SYSTEM_SET", missing.to_str().expect("UTF-8"))],
    );
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!(
            "test shards: the protected system set's directory {} is not there, so its \
             preparation was never started\n",
            missing.display()
        )
    );
    assert_eq!(scratch.calls(), [format!("{TEST} --no-run")]);
}

/// The shards wait for a protected system set's preparation still running
/// once the build has ended: no shard starts until its status records exit
/// 0, and the run then passes with its log printed (#647).
#[test]
fn the_shards_wait_after_the_build_for_the_system_sets_preparation() {
    let scratch = Scratch::new(&metadata(), &script());
    let set = scratch.dir.path().join("set");
    std::fs::create_dir(&set).expect("set");
    let set_dir = set.to_str().expect("a UTF-8 scratch");
    let child = scratch
        .command(&["test"], &[("BROKKR_SYSTEM_SET", set_dir)])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the script starts");
    let build = [format!("{TEST} --no-run")];
    poll(|| scratch.calls() == build);
    std::thread::sleep(std::time::Duration::from_millis(1500));
    let waiting = (scratch.calls(), scratch.events());
    prepared(&scratch, "server system set: 10 entries\n", "0 143\n");
    let run = child.wait_with_output().expect("the script ends");
    assert_eq!(waiting, (build.to_vec(), Vec::new()));
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    assert_eq!(scratch.events(), one_after_another(TEST, &SHARDS));
    let stdout = String::from_utf8_lossy(&run.stdout);
    let shown = stdout
        .split_once("\n==> the protected system set: exit 0 after 143s, awaited for ")
        .and_then(|(_, rest)| rest.split_once("s\n"))
        .map_or("", |(_, log)| log);
    assert!(
        shown.starts_with("server system set: 10 entries\ntest shards: runs every library:"),
        "{stdout}"
    );
}

/// The heap shard's real process launcher overlaps exactly its three
/// measurements, preserves coverage profiles, and joins on failure or cancel.
#[test]
fn parallel_heap_launcher_preserves_the_shard_barrier_and_measurements() {
    let output = Command::new("python3")
        .arg("scripts/tests/test_parallel_heaps.py")
        .current_dir(workspace())
        .output()
        .expect("the heap launcher's process tests run");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn parallel_heap_opt_in_replaces_only_the_heap_command_in_both_modes() {
    let real = metadata();
    for (args, prefix, build) in MODES {
        let scratch = Scratch::new(&real, &script());
        let python = scratch.dir.path().join("bin/python3");
        std::fs::write(&python, STUB).expect("stub heap launcher");
        std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
        let mut args = args.to_vec();
        args.push("--parallel-heaps");
        let run = scratch.run(&args, &[]);
        assert!(run.status.success(), "{run:?}");
        let mut commands = vec![format!("{prefix} {build}")];
        for (index, shard) in SHARDS.iter().enumerate() {
            if args[0] == "coverage" && *shard == "--doc" {
                continue;
            }
            commands.push(if index == 2 {
                if args[0] == "coverage" {
                    "scripts/parallel-heaps.py --nightly nightly-2026-09-05".to_string()
                } else {
                    "scripts/parallel-heaps.py".to_string()
                }
            } else {
                format!("{prefix} {shard}")
            });
        }
        assert_eq!(scratch.calls(), commands);
        let events: Vec<_> = commands[1..]
            .iter()
            .flat_map(|command| [format!("start {command}"), format!("end {command}")])
            .collect();
        assert_eq!(scratch.events(), events);
    }
}

/// Each machine of each system's layout, and the shards it runs, by their
/// place in [`NAMES`]: every shard sits on exactly one machine of each.
const MACHINES: [(&str, &[usize]); 5] = [
    ("linux/1", &[0]),
    ("linux/2", &[1, 2, 3, 4]),
    ("linux/3", &[5]),
    ("macos/1", &[0, 1, 2, 4, 5]),
    ("macos/2", &[3]),
];

/// A run whose BROKKR_TEST_MACHINE names a machine builds every binary and
/// runs that machine's shards alone, one after another in plan order, and
/// no shard's command inherits the variable.
#[test]
fn a_machine_runs_only_its_shards_one_after_another_in_plan_order() {
    let real = metadata();
    for (machine, indices) in MACHINES {
        let shards: Vec<&str> = indices.iter().map(|&at| SHARDS[at]).collect();
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(&["test"], &[("BROKKR_TEST_MACHINE", machine)]);
        assert_eq!(run.status.code(), Some(0), "{machine}: {run:?}");
        assert_eq!(
            scratch.events(),
            one_after_another(TEST, &shards),
            "{machine}"
        );
        let calls: Vec<String> = ["--no-run"]
            .iter()
            .chain(&shards)
            .map(|shard| format!("{TEST} {shard}"))
            .collect();
        assert_eq!(scratch.calls(), calls, "{machine}");
        assert_eq!(
            scratch.lines("machine"),
            vec!["unset"; shards.len()],
            "{machine}: a shard inherited the machine"
        );
        let stdout = String::from_utf8_lossy(&run.stdout);
        assert!(
            stdout.contains(&format!(
                "test shards: machine {machine} runs {} of the 6 shards\n",
                shards.len()
            )),
            "{stdout}"
        );
    }
    for system in ["linux", "macos"] {
        let mut placed: Vec<usize> = MACHINES
            .iter()
            .filter(|(machine, _)| machine.starts_with(system))
            .flat_map(|(_, indices)| indices.iter().copied())
            .collect();
        placed.sort_unstable();
        assert_eq!(placed, [0, 1, 2, 3, 4, 5], "{system} runs every shard once");
    }
}

/// A machine named in a form the layouts do not use, or one no shard runs
/// on, is refused before anything is built or run.
#[test]
fn a_machine_no_shard_runs_on_or_one_not_named_so_is_refused() {
    let real = metadata();
    for (machine, refusal) in [
        (
            "linux/4",
            "test shards: BROKKR_TEST_MACHINE names linux/4, which runs no shard\n",
        ),
        (
            "windows/1",
            "test shards: BROKKR_TEST_MACHINE 'windows/1' is not linux/<n> or macos/<n>\n",
        ),
        (
            "linux/0",
            "test shards: BROKKR_TEST_MACHINE 'linux/0' is not linux/<n> or macos/<n>\n",
        ),
        (
            "linux/1 ",
            "test shards: BROKKR_TEST_MACHINE 'linux/1 ' is not linux/<n> or macos/<n>\n",
        ),
    ] {
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(&["test"], &[("BROKKR_TEST_MACHINE", machine)]);
        assert_eq!(run.status.code(), Some(1), "{machine:?}: {run:?}");
        assert_eq!(String::from_utf8_lossy(&run.stderr), refusal, "{machine:?}");
        assert_eq!(scratch.calls(), Vec::<String>::new(), "{machine:?}");
    }
}

/// A layout that leaves a machine empty, or names a machine by anything
/// but a number from 1, is refused by name in every mode, before anything
/// is built or run.
#[test]
fn a_layout_with_an_empty_machine_or_an_unnumbered_one_is_refused() {
    let real = metadata();
    let table = script();
    let line = "shard 'capability_broker, alone' 3 1 test:it only:capability_broker::";
    assert_eq!(table.matches(line).count(), 1, "the broker's shard line");
    for (planted, refusal) in [
        (
            "shard 'capability_broker, alone' 4 1 test:it only:capability_broker::",
            "test shards: the linux layout leaves machine 3 with no shard\n",
        ),
        (
            "shard 'capability_broker, alone' 3 x test:it only:capability_broker::",
            "test shards: capability_broker, alone names the macos machine 'x', which is not \
             a number from 1\n",
        ),
    ] {
        let scratch = Scratch::new(&real, &table.replace(line, planted));
        for args in [
            &["test"][..],
            &["plan", "test"][..],
            &["machines", "test"][..],
        ] {
            let run = scratch.run(args, &[]);
            assert_eq!(run.status.code(), Some(1), "{planted} {args:?}: {run:?}");
            assert_eq!(
                String::from_utf8_lossy(&run.stderr),
                refusal,
                "{planted} {args:?}"
            );
        }
        assert_eq!(scratch.calls(), Vec::<String>::new(), "{planted}");
    }
}

/// The test job runs exactly the layouts' machines: ci.yml's
/// `engine-machines` matrix names each machine the script lays out, in
/// order, each on its own system's runner, and the script prints each
/// machine's shards as [`MACHINES`] holds them.
#[test]
fn the_test_jobs_machines_are_the_layouts() {
    let run = Command::new("bash")
        .args([SCRIPT, "machines", "test"])
        .current_dir(workspace())
        .env_remove("BROKKR_TEST_MACHINE")
        .output()
        .expect("the machines print");
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    let expected: String = MACHINES
        .iter()
        .map(|(machine, indices)| {
            let names: Vec<&str> = indices.iter().map(|&at| NAMES[at]).collect();
            format!("{machine}: {}\n", names.join(" | "))
        })
        .collect();
    assert_eq!(String::from_utf8_lossy(&run.stdout), expected);

    let workflow =
        std::fs::read_to_string(workspace().join(".github/workflows/ci.yml")).expect("ci.yml");
    let body = crate::workflow::jobs(&workflow)
        .into_iter()
        .find_map(|(id, body)| (id == "engine-machines").then_some(body))
        .expect("ci.yml's engine-machines job");
    let lines: Vec<&str> = body.lines().map(str::trim).collect();
    let listed: Vec<(&str, &str)> = lines
        .windows(2)
        .filter_map(|pair| {
            let os = pair[0].strip_prefix("- os: ")?;
            Some((os, pair[1].strip_prefix("machine: ")?))
        })
        .collect();
    let laid_out: Vec<(&str, &str)> = MACHINES
        .iter()
        .map(|(machine, _)| match machine.starts_with("linux/") {
            true => ("ubuntu-latest", *machine),
            false => ("macos-latest", *machine),
        })
        .collect();
    assert_eq!(listed, laid_out);
}
