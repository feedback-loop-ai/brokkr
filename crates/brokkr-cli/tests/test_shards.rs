//! The workspace suite as shards (#633): `scripts/test-shards.sh` runs the
//! tests `cargo test --workspace` runs, each test target exactly once, one
//! shard after another and capability_broker last. What it refuses is
//! bound here: its shards are held to the real tree's `cargo metadata`, a
//! planted target no shard runs, one run twice, a selector that names
//! nothing or cannot be read is refused by name, and with a `cargo` stub
//! first on `PATH` the order, the failures and a filter that matches
//! nothing are each shown. The per-binary bound is enforced by the
//! foreground `shard-runner.py` that owns each shard's output and group;
//! its own process tests are run from here, and a runner that dies, a group
//! that cannot be shown ended and a binary that passes after its bound are
//! each refused by name.

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::workspace_root::workspace;

mod machines;

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

/// A stand-in for `cargo`: `metadata` prints the planted metadata; every other
/// call is recorded, and a shard's call prints one libtest summary, records
/// `start`/`end` for interleaving, records the machine and bound it inherited,
/// and records how a raised SIGINT ends a child shell. `STUB_FAIL` fails the
/// shard, `STUB_EMPTY` runs no test, `STUB_PLANT` prints `STUB_LINES` instead
/// of its summary, `STUB_KILL` kills its runner and waits for `release`, and
/// `STUB_RECORD` writes a status record then kills its runner.
///
/// `dep` copies a real executable into a directory named `deps`, so the kernel
/// reports the copy (never a script's interpreter) as the executable and the
/// runner finds it by `/proc` or `ps`. Each copy's script ends in `:`, since a
/// shell that execs its last command (macOS's `/bin/sh`) would leave `deps`
/// unseen. `STUB_HANG` runs `hang-0123abcd` past the bound, `STUB_QUIET_HANG`
/// with no start line, `STUB_DOCTEST_HANG` runs `rustdoc --test`,
/// `STUB_SLOW`/`STUB_PAIR` stay under it, `STUB_TAIL` prints a result then
/// lives past the bound, `STUB_LEAK` leaves a `deps` binary alive past the
/// bound after argv itself exits, `STUB_CARGO_TAIL` sleeps after every deps
/// binary is gone, `STUB_STOPPED` leaves a stopped `/bin/sleep` helper outside
/// `deps`, `STUB_LATE`/`STUB_LATE_PROGRESS` print a result at or after it,
/// `STUB_CHATTY` keeps printing, `STUB_PARENS` has a deps path with
/// parentheses, `STUB_UNWRITABLE` makes the bound record path a directory, and
/// `STUB_LEAVE` leaves a sleeping member in the group.
const STUB: &str = r#"#!/usr/bin/env bash
dir="$STUB_DIR"
deps_dir="$dir/target/debug/deps"
dep() {
  local name="$1" source="${2:-/bin/sh}" folder="${3:-$deps_dir}" path
  mkdir -p "$folder"
  path="$folder/$name"
  cp "$source" "$path"
  chmod +x "$path"
  printf '%s' "$path"
}
shell_dep() {
  local name="$1" body="$2"; shift 2
  local path
  path="$(dep "$name" /bin/sh)"
  "$path" -c "$body" "$name" "$@" &
}
run_dep() {
  local name="$1" seconds="$2" pidfile="${3:-}" folder="${4:-$deps_dir}" path
  path="$(dep "$name" /bin/sh "$folder")"
  "$path" -c "sleep $seconds; :" "$name" &
  [ -z "$pidfile" ] || printf '%s\n' "$!" > "$pidfile"
  wait
}
if [ "$1" = metadata ]; then cat "$dir/metadata.json"; exit 0; fi
if [[ " $* " == *" --message-format=json "* ]]; then
  for heap in heap_claude heap_codex heap_dsh; do
    printf '{"reason":"compiler-artifact","profile":{"test":true},"target":{"name":"%s"},"executable":"%s/%s-0123abcd"}\n' \
      "$heap" "$dir" "$heap"
  done
  exit 0
fi
printf '%s\n' "$*" >> "$dir/calls"
case " $* " in *" --no-run "* | *" --list "*) exit 0 ;; esac
summary() {
  printf 'test result: %s. %s passed; %s failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n' "$1" "$2" "$3"
}
names() { local word="$1"; shift; [ -n "$word" ] && [[ " $* " == *" $word "* ]]; }
printf 'start %s\n' "$*" >> "$dir/events"
printf '%s\n' "${BROKKR_TEST_MACHINE-unset}" >> "$dir/machine"
printf '%s\n' "${BROKKR_TEST_SHARD_BOUND_SECONDS-unset}" >> "$dir/bound"
sh -c 'kill -INT $$; exit 0'
printf '%s %s\n' "$?" "$*" >> "$dir/signals"
if names "${STUB_HANG:-}" "$@"; then run_dep hang-0123abcd 600 "$dir/hang.pid"; fi
if names "${STUB_QUIET_HANG:-}" "$@"; then run_dep quiet-0123abcd 600 "$dir/quiet.pid"; fi
if names "${STUB_DOCTEST_HANG:-}" "$@"; then
  cp /bin/sh "$dir/rustdoc"; chmod +x "$dir/rustdoc"
  "$dir/rustdoc" -c 'sleep 600; :' rustdoc --crate-name brokkr_core --test &
  printf '%s\n' "$!" > "$dir/doctest.pid"; wait
fi
if names "${STUB_STOPPED:-}" "$@"; then
  dep_path="$(dep stopped-0456 /bin/sh)"
  "$dep_path" -c '/bin/sleep 600 &
stopped=$!
printf "%s\n" "$stopped" > "$1"
sleep 0.2
kill -STOP "$stopped"
sleep 600; :' stopped "$dir/stopped.pid" &
  wait
fi
if names "${STUB_LATE:-}" "$@"; then
  shell_dep late-9999 'sleep 1.15
printf "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s\n"
sleep 600; :'
  wait
fi
if names "${STUB_LATE_PROGRESS:-}" "$@"; then
  shell_dep late-progress-1234 'sleep 0.6
printf "test still running...\n"
sleep 0.55
printf "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s\n"
sleep 600; :'
  wait
fi
if names "${STUB_CHATTY:-}" "$@"; then
  shell_dep chatty-5678 'count=0
while [ "$count" -lt 10 ]; do
  sleep 0.3
  printf "test still running...\n"
  count=$((count + 1))
done
sleep 600; :'
  wait
fi
if names "${STUB_TAIL:-}" "$@"; then
  shell_dep tail-9abc 'sleep 0.2
printf "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s\n"
sleep 2; :'
  wait
fi
if names "${STUB_LEAK:-}" "$@"; then
  dep_path="$(dep leak-0123abcd /bin/sh)"
  "$dep_path" -c 'sleep 1.8; :' leak-0123abcd & printf '%s\n' "$!" > "$dir/leak.pid"
  sleep 0.4; summary ok 1 0; exit 0
fi
if names "${STUB_CARGO_TAIL:-}" "$@"; then run_dep short-0123abcd 0.2; sleep 3; fi
if names "${STUB_PARENS:-}" "$@"; then
  run_dep hang-0123abcd 5 "$dir/parens.pid" "$dir/target/ctd (y)/debug/deps"
fi
if names "${STUB_SLOW:-}" "$@"; then run_dep slow-0123abcd 1.5; fi
if names "${STUB_PAIR:-}" "$@"; then run_dep first-aaaa 2; run_dep second-bbbb 2; fi
if names "${STUB_UNWRITABLE:-}" "$@"; then
  for work in "$TMPDIR"/brokkr-test-shards.*; do
    [ -d "$work" ] && mkdir -p "$work/0.bound"
  done
  run_dep unwritable-0abc 600 "$dir/unwritable.pid"
fi
if names "${STUB_LEAVE:-}" "$@"; then
  sleep 60 &
  printf '%s %s\n' "$$" "$!" > "$dir/leave.pid"
  summary ok 1 0
fi
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
  # Let the runner copy the summary the test expects in the kept log before
  # the stub kills it; the runner still dies before it writes its status.
  sleep 0.5
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

/// A stand-in for `uname`: the host system `STUB_UNAME` names, Linux unless
/// it names another, so a run reads the same on every host these tests run
/// on.
const UNAME: &str = "#!/usr/bin/env bash\nprintf '%s\\n' \"${STUB_UNAME:-Linux}\"\n";

/// A `tail` that fails on the first shard's log: nothing reads a shard's log
/// beside the runner any more, so this must change no shard's verdict.
const FAILING_TAIL: &str = r#"#!/usr/bin/env bash
for last; do :; done
case "$last" in
  */0.log) printf 'tail: cannot open the log\n' >&2; exit 1 ;;
esac
exec /usr/bin/tail "$@"
"#;

/// A `tail` that fails after a delay: nothing reads a shard's log beside the
/// runner that owns its output, so a failing tail must change no verdict.
const BLIND_TAIL: &str = r#"#!/usr/bin/env bash
for last; do :; done
case "$last" in
  */0.log) sleep 3; printf 'tail: cannot open the log\n' >&2; exit 1 ;;
esac
exec /usr/bin/tail "$@"
"#;

/// A `python3` that answers the first shard's runner with a `bound` status
/// and a record the shell cannot read, so the unreadable-record refusal is
/// exercised without a runner that would have to write a bad record itself.
const BAD_RECORD_RUNNER: &str = r#"#!/usr/bin/env bash
record=''
status=''
previous=''
for argument in "$@"; do
  case "$previous" in
    --bound-record) record="$argument" ;;
    --status) status="$argument" ;;
  esac
  previous="$argument"
done
case "$record" in
  */0.bound)
    printf 'not a record\n' > "$record"
    printf 'bound\n' > "$status"
    exit 1
    ;;
esac
exec "$REAL_PYTHON3" "$@"
"#;

/// A `rustc` for the real heap launcher: a sysroot and a reported host.
const RUSTC: &str = r#"#!/usr/bin/env bash
case "$*" in
  *--print*sysroot*) printf '%s\n' "$STUB_DIR/toolchain" ;;
  *) printf 'host: fixture-host\n' ;;
esac
"#;

/// A heap child the real launcher runs: it records its pid and sleeps past
/// the bound the shard passes.
const SLEEPING_HEAP: &str =
    "#!/usr/bin/env bash\nprintf '%s\\n' \"$$\" >\"$STUB_DIR/$(basename \"$0\").pid\"\nsleep 5\n";

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
        for (name, stub) in [("cargo", STUB), ("uname", UNAME)] {
            let path = dir.path().join("bin").join(name);
            std::fs::write(&path, stub).expect("stub");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("mode");
        }
        let metadata = serde_json::to_string(metadata).expect("metadata serializes");
        std::fs::write(dir.path().join("metadata.json"), metadata).expect("metadata");
        std::fs::write(dir.path().join("test-shards.sh"), script).expect("script");
        Self { dir }
    }

    /// Writes `body` as the executable `name` in the scratch's `bin/`, so a
    /// run finds it first on `PATH`.
    fn executable(&self, name: &str, body: &str) {
        let path = self.dir.path().join("bin").join(name);
        std::fs::write(&path, body).expect("a stub");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
    }

    /// The script run from the workspace with `args`, the stub first on
    /// `PATH` and the scratch `tmp/` as `TMPDIR`.
    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        self.command(args, env).output().expect("the script runs")
    }

    /// The script run as [`Scratch::run`] does, but killed and refused when it
    /// outlasts `limit`: a runner that stops reading its output must not hang
    /// the suite. The pipes are drained beside the wait, since a full stdout
    /// pipe would block the script the test is waiting on.
    fn run_within(&self, args: &[&str], env: &[(&str, &str)], limit: Duration) -> Output {
        let mut child = self
            .command(args, env)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("the script starts");
        let mut stdout = child.stdout.take().expect("the script's stdout");
        let mut stderr = child.stderr.take().expect("the script's stderr");
        let stdout = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut stdout, &mut bytes).expect("the script's stdout");
            bytes
        });
        let stderr = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            std::io::Read::read_to_end(&mut stderr, &mut bytes).expect("the script's stderr");
            bytes
        });
        let deadline = Instant::now() + limit;
        loop {
            let status = child.try_wait().expect("the script is polled");
            if let Some(status) = status {
                return Output {
                    status,
                    stdout: stdout.join().expect("the stdout reader"),
                    stderr: stderr.join().expect("the stderr reader"),
                };
            }
            if Instant::now() >= deadline {
                // The script's descendants may hold the pipes after it ends;
                // the readers are left to drain them rather than joined, so a
                // hung shard fails this test instead of hanging the suite.
                let _ = child.kill();
                let status = child.wait().expect("the script ends");
                panic!("the script outlasted {limit:?} and ended {status:?}");
            }
            std::thread::sleep(Duration::from_millis(20));
        }
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
            .env_remove("BROKKR_TEST_SHARD_BOUND_SECONDS")
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

/// `call` with the scratch's random heap bound-record path replaced by a
/// placeholder, so a command's shape is compared exactly.
fn without_bound_path(call: &str) -> String {
    match call.split_once(" --bound-record ") {
        Some((head, _)) => format!("{head} --bound-record <path>"),
        None => call.to_string(),
    }
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
const NAMES: [&str; 10] = [
    "every library, run_pages skipped",
    "the libraries' run_pages tests",
    "the it binaries, capability_broker skipped",
    "the heap measurements",
    "the other own-process binaries",
    "the fleet heap view",
    "the run and seat heap views",
    "the doctests",
    "capability_broker's enabled suite, alone",
    "capability_broker, its enabled suite skipped, alone",
];

/// The test mode's shard commands after `cargo`, as the plan prints them.
const SHARDS: [&str; 10] = [
    "--lib -- --skip run_pages::",
    "--lib -- run_pages::",
    "--test it -- --skip capability_broker::",
    "--test heap_claude --test heap_codex --test heap_dsh",
    "--bin brokkr --test driver_conformance --test transcript_surfaces --test hands_exits \
     --test secret_drop --test capability_launch --test operated_repo --test queued_launch \
     --test seatbelt_lifetime_probe --test summary_heap --test heap_open --test heap_costs",
    "--test heap_fleet",
    "--test heap_run_view --test heap_seat_view",
    "--doc",
    "--test it -- capability_broker::enabled::",
    "--test it -- capability_broker:: --skip capability_broker::enabled::",
];

/// Where [`NAMES`] holds the doctests, which the coverage mode never runs.
const DOCTESTS: usize = 7;

/// Where [`NAMES`] holds the heap measurements, the one shard
/// `--parallel-heaps` runs through its own launcher.
const HEAPS: usize = 3;

/// The word the cargo stub knows the first shard by: its `run_pages::`
/// skip, which no other shard's command holds, though the second runs the
/// libraries too.
const FIRST: &str = "--skip run_pages::";

/// Where [`NAMES`] holds the broker's enabled suite, which only Linux
/// compiles, so a macOS host never runs it.
const LINUX_ONLY: usize = 8;

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
        (
            "coverage",
            [&plan[..DOCTESTS], &plan[DOCTESTS + 1..]].concat().concat(),
        ),
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
        "2 3 test:it skip:capability_broker::\n",
        "2 3 test:it skip:capability_broker::observer::\n",
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
                 skipped (skip:capability_broker::observer::), capability_broker's enabled \
                 suite, alone (only:capability_broker::enabled::), capability_broker, its \
                 enabled suite skipped, alone (only:capability_broker:: \
                 skip:capability_broker::enabled::)"
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
    let coverage_shards: Vec<&str> = SHARDS[..DOCTESTS]
        .iter()
        .chain(&SHARDS[DOCTESTS + 1..])
        .copied()
        .collect();
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
            stdout.contains(
                "test shards: capability_broker, its enabled suite skipped, alone: exit 0 after "
            ),
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
            ("STUB_FAIL", FIRST),
            "test shards: every library, run_pages skipped failed with exit 101\n",
        ),
        (
            ("STUB_EMPTY", "-- capability_broker::"),
            "test shards: capability_broker, its enabled suite skipped, alone executed no test, \
             so its filter matches none\n",
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
    // The broker's narrower shard skips too; this names only `it`'s first.
    const SKIPPED: &str = "--skip capability_broker::";
    let real = metadata();
    let unread = "printed a test result: summary the guard cannot read, so no test is \
                  shown to have run";
    let skipped = "the it binaries, capability_broker skipped";
    for (word, lines, shard, refusal) in [
        (SKIPPED, [UNREAD, ONE_TEST].concat(), skipped, unread),
        (SKIPPED, [ONE_TEST, UNKNOWN].concat(), skipped, unread),
        (
            FIRST,
            "running the library\n".to_string(),
            "every library, run_pages skipped",
            "printed no test result: summary, so no test is shown to have run",
        ),
        (
            FIRST,
            UNREAD.to_string(),
            "every library, run_pages skipped",
            unread,
        ),
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

/// The one work directory a lost run kept under the scratch's TMPDIR.
fn kept_work(scratch: &Scratch) -> std::path::PathBuf {
    let work: Vec<_> = std::fs::read_dir(scratch.dir.path().join("tmp"))
        .expect("the scratch TMPDIR")
        .map(|entry| entry.expect("an entry").path())
        .collect();
    assert_eq!(work.len(), 1, "{work:?}");
    work.into_iter().next().expect("one kept work directory")
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
    let ran = one_after_another(TEST, &SHARDS[..=HEAPS]);
    let running = scratch.events();
    let finally = after_release(&scratch, &ran[2 * HEAPS + 1]);
    assert_eq!(
        (running.as_slice(), finally),
        (&ran[..=2 * HEAPS], ran.clone())
    );
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    let unstarted = not_started(&NAMES[HEAPS + 1..]);
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!(
            "test shards: the heap measurements recorded no exit status (its runner ended with \
             exit 137), so it is not shown to have passed\n{unstarted}"
        )
    );
    let calls: Vec<String> = ["--no-run"]
        .iter()
        .chain(&SHARDS[..=HEAPS])
        .map(|shard| format!("{TEST} {shard}"))
        .collect();
    assert_eq!(scratch.calls(), calls);
    let stdout = String::from_utf8_lossy(&run.stdout);
    let work = kept_work(&scratch);
    let shown = format!(
        "\n==> the heap measurements: no exit status recorded, its runner ended with exit 137; \
         the logs are kept in {}\n{ONE_TEST}",
        work.display()
    );
    for line in [
        shown.as_str(),
        "\n==> every library, run_pages skipped: exit 0 after ",
        "test shards: the heap measurements: no exit status recorded\n",
        "test shards: capability_broker, its enabled suite skipped, alone: not started\n",
    ] {
        assert!(stdout.contains(line), "{line}\n{stdout}");
    }
    assert!(work.join("2.log").is_file(), "{work:?}");
}

/// A `bound` status whose record is not the two readable lines is refused
/// by name as an unreadable bound record, not as a group that was not shown
/// ended: the run fails, keeps the log and starts no later shard.
#[test]
fn a_bound_status_with_an_unreadable_record_is_refused_by_name() {
    let scratch = Scratch::new(&metadata(), &script());
    let real_python = real_python3();
    scratch.executable("python3", BAD_RECORD_RUNNER);
    let run = scratch.run(&["test"], &[("REAL_PYTHON3", &real_python)]);
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    let unstarted: String = NAMES[1..]
        .iter()
        .map(|shard| {
            format!(
                "test shards: {shard} was not started: an earlier shard's bound record could \
                 not be read, so its command may still be running and the shard cannot be shown \
                 to run alone\n"
            )
        })
        .collect();
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!(
            "test shards: the bound record of every library, run_pages skipped could not be read, so it is not \
             shown to have passed\n{unstarted}"
        )
    );
    assert_eq!(scratch.calls(), [format!("{TEST} --no-run")]);
    assert_eq!(scratch.events(), Vec::<String>::new());
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains(
            "test shards: the bound record of every library, run_pages skipped could not be read\n"
        ),
        "{stdout}"
    );
}

/// A record path that is a directory makes the runner's write fail. The
/// runner still ends the group it owns and writes no status, so the shell
/// loses the shard as one whose runner recorded no exit status, keeps its
/// log and starts no later shard; the sleeper is gone.
#[test]
fn a_record_that_cannot_be_written_still_ends_the_group_and_loses_the_shard() {
    let scratch = Scratch::new(&metadata(), &script());
    let run = scratch.run_within(
        &["test"],
        &[
            ("STUB_UNWRITABLE", FIRST),
            ("BROKKR_TEST_SHARD_BOUND_SECONDS", "1"),
        ],
        Duration::from_secs(25),
    );
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    let pid = recorded_pid(&scratch, "unwritable.pid");
    let alive = process_alive(pid);
    if alive {
        rustix::process::kill_process_group(
            rustix::process::Pid::from_raw(pid).expect("a pid"),
            rustix::process::Signal::KILL,
        )
        .ok();
    }
    poll(|| !process_alive(pid));
    assert!(!alive, "the sleeper {pid} still runs");
    let unstarted = not_started(&NAMES[1..]);
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!(
            "test shards: every library, run_pages skipped recorded no exit status (its runner ended with exit 1), \
             so it is not shown to have passed\n{unstarted}"
        )
    );
    assert_eq!(
        scratch.events(),
        vec![format!("start {TEST} {}", SHARDS[0])],
        "the bounded shard recorded an end after its record failed"
    );
    let work = kept_work(&scratch);
    assert!(work.join("0.log").is_file(), "{work:?}");
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
        for record in ["exit 0\0\n", "exit 0\n\n", "exit 0\0", "exit 0\n"] {
            let scratch = Scratch::new(&real, &script());
            std::fs::write(scratch.dir.path().join("record"), record).expect("record");
            let run = scratch.run(args, &[("STUB_RECORD", FIRST)]);
            let stderr = String::from_utf8_lossy(&run.stderr);
            if record == "exit 0\n" {
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
                    "test shards: every library, run_pages skipped recorded no exit status (its runner ended with \
                     exit 137), so it is not shown to have passed\n{}",
                    not_started(&later)
                ),
                "{args:?} {record:?}"
            );
            assert_eq!(
                scratch.calls(),
                [
                    format!("{prefix} {build}"),
                    format!("{prefix} {}", SHARDS[0])
                ],
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
        shown.starts_with(
            "server system set: 10 entries\ntest shards: runs every library, run_pages skipped:"
        ),
        "{stdout}"
    );
}

/// The heap shard's real process launcher overlaps exactly its three
/// measurements, preserves coverage profiles, and joins on failure or cancel.
/// The foreground runner's own process tests: the bound, the statuses, the
/// group proof, the forwarded signals and the byte-for-byte output.
#[test]
fn the_shard_runner_owns_the_bound_and_the_group_proof() {
    let output = Command::new("python3")
        .arg("scripts/tests/test_shard_runner.py")
        .current_dir(workspace())
        .output()
        .expect("the shard runner's process tests run");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

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
        let real_python = real_python3();
        // Only the heap shard's `python3` is the stub: every other shard's
        // is the real interpreter running the real foreground runner.
        let wrapper = "#!/usr/bin/env bash\nif [ \"$1\" = scripts/parallel-heaps.py ]; then \
                       exec \"$STUB_DIR/bin/cargo\" \"$@\"; fi\nexec \"$REAL_PYTHON3\" \"$@\"\n";
        scratch.executable("python3", wrapper);
        let mut args = args.to_vec();
        args.push("--parallel-heaps");
        let run = scratch.run(&args, &[("REAL_PYTHON3", &real_python)]);
        assert!(run.status.success(), "{run:?}");
        let mut commands = vec![format!("{prefix} {build}")];
        for (index, shard) in SHARDS.iter().enumerate() {
            if args[0] == "coverage" && *shard == "--doc" {
                continue;
            }
            commands.push(if index == HEAPS {
                if args[0] == "coverage" {
                    "scripts/parallel-heaps.py --nightly nightly-2026-09-05 --binary-bound 2400 \
                     --bound-record <path>"
                        .to_string()
                } else {
                    "scripts/parallel-heaps.py --binary-bound 2400 --bound-record <path>"
                        .to_string()
                }
            } else {
                format!("{prefix} {shard}")
            });
        }
        let calls: Vec<String> = scratch
            .calls()
            .iter()
            .map(|call| without_bound_path(call))
            .collect();
        assert_eq!(calls, commands);
        let events: Vec<String> = scratch
            .events()
            .iter()
            .map(|line| without_bound_path(line))
            .collect();
        let expected: Vec<_> = commands[1..]
            .iter()
            .flat_map(|command| [format!("start {command}"), format!("end {command}")])
            .collect();
        assert_eq!(events, expected);
    }
}

/// Each package's `it` binary, as the partition check names it.
const IT_PACKAGES: [&str; 5] = [
    "brokkr-core",
    "brokkr-store",
    "brokkr-runtime",
    "brokkr-view",
    "brokkr-cli",
];

/// The broker's tests split three ways, `skip:F`, `only:F skip:G` and
/// `only:G`, only where G extends F and the narrower shard filters G: a
/// skip that does not extend its shard's filter is refused as more than one
/// filter, and a narrower shard that filters another part leaves G's tests
/// to no shard, which is refused for every `it` binary.
#[test]
fn a_narrower_skip_must_extend_its_filter_and_its_part_keep_a_shard() {
    let real = metadata();
    let table = script();
    let skip = "only:capability_broker:: skip:capability_broker::enabled::";
    let narrower = "test:it only:capability_broker::enabled::";
    assert_eq!(table.matches(skip).count(), 1, "the broker's narrower skip");
    assert_eq!(
        table.matches(narrower).count(),
        1,
        "the broker's narrower shard"
    );
    let unread = |package: &str, rest: &str, filters: &str| {
        format!(
            "{package} test it does not run exactly once: the it binaries, capability_broker \
             skipped (skip:capability_broker::), capability_broker's enabled suite, alone \
             ({rest}), capability_broker, its enabled suite skipped, alone ({filters})"
        )
    };
    let elsewhere = "only:capability_broker:: skip:hands::";
    let mut problems = vec![format!(
        "capability_broker, its enabled suite skipped, alone: more than one filter: {elsewhere}"
    )];
    problems.extend(
        IT_PACKAGES
            .iter()
            .map(|package| unread(package, "only:capability_broker::enabled::", elsewhere)),
    );
    assert_refused(
        &real,
        &table.replace(skip, elsewhere),
        &problems.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let another: Vec<String> = IT_PACKAGES
        .iter()
        .map(|package| unread(package, "only:capability_broker::serving::", skip))
        .collect();
    assert_refused(
        &real,
        &table.replace(narrower, "test:it only:capability_broker::serving::"),
        &another.iter().map(String::as_str).collect::<Vec<_>>(),
    );
}

/// Whether process `pid` is still there, read by the null signal.
fn process_alive(pid: i32) -> bool {
    rustix::process::test_kill_process(rustix::process::Pid::from_raw(pid).expect("a pid")).is_ok()
}

/// The pid the stub wrote to `file`, once it is there.
fn recorded_pid(scratch: &Scratch, file: &str) -> i32 {
    std::fs::read_to_string(scratch.dir.path().join(file))
        .expect("the stub recorded its pid")
        .trim()
        .parse()
        .expect("a pid")
}

/// A fresh scratch's run of the stub's word under `bound` seconds, refused
/// rather than left running when the bound is never enforced.
fn bounded(stub: &str, word: &str, bound: &str) -> (Scratch, Output) {
    let scratch = Scratch::new(&metadata(), &script());
    let run = scratch.run_within(
        &["test"],
        &[(stub, word), ("BROKKR_TEST_SHARD_BOUND_SECONDS", bound)],
        Duration::from_secs(25),
    );
    (scratch, run)
}

/// A run that passed untouched: exit 0, nothing on stderr, every shard in
/// its turn.
fn assert_whole_run_passes(scratch: &Scratch, run: &Output) {
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    assert_eq!(String::from_utf8_lossy(&run.stderr), "");
    assert_eq!(scratch.events(), one_after_another(TEST, &SHARDS));
}

/// The stub's word runs under `bound` seconds and passes untouched, every
/// shard in its turn.
fn assert_bounded_pass(stub: &str, word: &str, bound: &str) {
    let (scratch, run) = bounded(stub, word, bound);
    assert_whole_run_passes(&scratch, &run);
}

/// The stub's word starts `binary` and runs past the 2 s bound: the shard
/// fails by that name and no process of the group remains.
fn assert_bounded_hang(stub: &str, word: &str, pid_file: &str, binary: &str) -> (Scratch, Output) {
    let (scratch, run) = bounded(stub, word, "2");
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!("test shards: {binary} ran past 2 s\n")
    );
    let pid = recorded_pid(&scratch, pid_file);
    poll(|| !process_alive(pid));
    assert!(!process_alive(pid), "{binary} {pid} still runs");
    (scratch, run)
}

/// The events of a run whose first `--lib` shard started but never ended,
/// followed by every later shard in its turn.
fn bounded_first_shard_events() -> Vec<String> {
    let mut expected = vec![format!("start {TEST} {}", SHARDS[0])];
    expected.extend(one_after_another(TEST, &SHARDS[1..]));
    expected
}

/// `scratch`'s first `--lib` shard started and every later shard ran.
fn assert_bounded_first_shard_ran(scratch: &Scratch) {
    assert_eq!(
        scratch.events(),
        bounded_first_shard_events(),
        "a later shard did not run"
    );
}

/// `scratch`'s run is under 10 s and only its first shard started: every
/// later shard still ran in its turn.
fn assert_bounded_first_shard(scratch: &Scratch, started: Instant) {
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the bound took {:?}",
        started.elapsed()
    );
    assert_bounded_first_shard_ran(scratch);
}

/// `scratch`'s first shard run as `stub` under `bound` seconds, refused by
/// name for `binary`; the whole run is held to 25 s.
fn bounded_by_name(scratch: &Scratch, stub: &str, bound: &str, binary: &str) -> Output {
    let run = scratch.run_within(
        &["test"],
        &[(stub, FIRST), ("BROKKR_TEST_SHARD_BOUND_SECONDS", bound)],
        Duration::from_secs(25),
    );
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    assert_eq!(
        String::from_utf8_lossy(&run.stderr),
        format!("test shards: {binary} ran past {bound} s\n")
    );
    run
}

/// The interpreter path the `python3` stubs exec, found once.
fn real_python3() -> String {
    let output = Command::new("python3")
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .expect("the real python3");
    String::from_utf8(output.stdout)
        .expect("a UTF-8 interpreter path")
        .trim()
        .to_string()
}

/// A `deps` binary that sleeps past a 2 s bound fails its shard by name in
/// under 10 s, nothing of its process group remains, and the next shard
/// still runs and passes. The binary writes nothing at all: its clock is
/// its process, so a silent binary cannot be unbounded.
#[test]
fn a_binary_past_the_bound_fails_by_name_and_the_next_shard_runs() {
    let started = Instant::now();
    let (scratch, run) = assert_bounded_hang("STUB_HANG", FIRST, "hang.pid", "hang-0123abcd");
    assert_bounded_first_shard(&scratch, started);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains(
            "test shards: every library, run_pages skipped: hang-0123abcd ran past 2 s\n"
        ),
        "{stdout}"
    );
}

/// A `deps` binary prints a passing summary after a 1 s bound and keeps
/// running. It is named by the bound, never passed by the summary, and
/// every later shard runs. The summary may be killed before it is copied:
/// the clock is the process, not the output.
#[test]
fn a_binary_that_passes_after_the_bound_is_still_named() {
    let scratch = Scratch::new(&metadata(), &script());
    scratch.executable("tail", BLIND_TAIL);
    bounded_by_name(&scratch, "STUB_LATE", "1", "late-9999");
    assert_bounded_first_shard_ran(&scratch);
}

/// A `deps` binary that prints a progress line after 0.6 s and a passing
/// summary after 1.15 s is bounded by its process: neither line moves the
/// clock, so the shard fails by name and every later shard runs.
#[test]
fn a_binary_whose_progress_line_delays_the_result_is_still_bound() {
    let scratch = Scratch::new(&metadata(), &script());
    bounded_by_name(&scratch, "STUB_LATE_PROGRESS", "1", "late-progress-1234");
    assert_bounded_first_shard_ran(&scratch);
}

/// A `deps` binary that prints a line every 0.3 s for 3 s is bounded near
/// its 1 s clock, not at the end of its output: the shard fails by name and
/// every later shard runs.
#[test]
fn a_binary_that_keeps_printing_is_still_bounded() {
    let started = Instant::now();
    let scratch = Scratch::new(&metadata(), &script());
    bounded_by_name(&scratch, "STUB_CHATTY", "1", "chatty-5678");
    assert_bounded_first_shard(&scratch, started);
}

/// A `deps` path with parentheses still names its binary from the kernel's
/// executable path: the bound fires at 2 s, the group ends and every later
/// shard runs.
#[test]
fn a_deps_path_with_parentheses_is_named_and_bounded() {
    let (scratch, _) = assert_bounded_hang("STUB_PARENS", FIRST, "parens.pid", "hang-0123abcd");
    assert_bounded_first_shard_ran(&scratch);
}

/// A `deps` binary that prints no start line at all is still found in the
/// process table and bounded at 2 s, named by its executable; every later
/// shard runs.
#[test]
fn a_test_binary_that_prints_no_start_line_is_still_bounded() {
    let (scratch, _) = assert_bounded_hang("STUB_QUIET_HANG", FIRST, "quiet.pid", "quiet-0123abcd");
    assert_bounded_first_shard_ran(&scratch);
}

/// A binary that ends under the bound passes untouched, and every shard
/// still runs. The bound is 3 s against a 1.5 s binary, a margin the poll
/// interval and a loaded host cannot eat.
#[test]
fn a_binary_under_the_bound_passes_untouched() {
    assert_bounded_pass("STUB_SLOW", FIRST, "3");
}

/// Two binaries in one shard, each under the bound, whose sum passes it:
/// they pass, because the bound is per binary, not per shard. Each runs
/// 2 s under a 3 s bound, and their sum passes it.
#[test]
fn two_binaries_each_under_the_bound_pass_because_the_bound_is_per_binary() {
    assert_bounded_pass("STUB_PAIR", FIRST, "3");
}

/// A hang in cargo's own tail, after every `deps` binary is gone, is not a
/// test binary and is not bounded: nothing is named and the shard passes.
#[test]
fn a_hang_in_cargos_own_tail_is_not_bounded() {
    assert_bounded_pass("STUB_CARGO_TAIL", FIRST, "2");
}

/// A `deps` binary that prints a passing result at 0.2 s and then lives
/// 2 s is bounded at 1 s: a result line cannot clear its clock. The result
/// is copied before the bound, so it is in the log, and every later shard
/// runs.
#[test]
fn a_passing_result_line_at_the_tail_does_not_clear_the_clock() {
    let scratch = Scratch::new(&metadata(), &script());
    let run = bounded_by_name(&scratch, "STUB_TAIL", "1", "tail-9abc");
    assert_bounded_first_shard_ran(&scratch);
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains("test result: ok. 1 passed;"), "{stdout}");
}

/// A `deps` binary that outlives argv itself is still bounded: a fake cargo
/// prints a passing summary at 0.4 s and exits 0 while `leak-0123abcd` lives
/// 1.8 s. The 1 s clock is judged after argv's exit, so the shard fails by
/// the binary's name, every later shard runs, and nothing of the group
/// remains.
#[test]
fn a_binary_alive_after_argv_exits_is_still_bounded() {
    let scratch = Scratch::new(&metadata(), &script());
    bounded_by_name(&scratch, "STUB_LEAK", "1", "leak-0123abcd");
    assert_bounded_first_shard_ran(&scratch);
    let pid = recorded_pid(&scratch, "leak.pid");
    poll(|| !process_alive(pid));
    assert!(!process_alive(pid), "leak-0123abcd {pid} still runs");
}

/// A doctest run past the bound is named by its crate in the failure and in
/// the final summary, with the name's space kept whole, and later shards
/// still run.
#[test]
fn a_doctest_past_the_bound_is_named_as_the_crate() {
    let (scratch, run) = assert_bounded_hang(
        "STUB_DOCTEST_HANG",
        "--doc",
        "doctest.pid",
        "Doc-tests brokkr_core",
    );
    let mut expected = one_after_another(TEST, &SHARDS[..DOCTESTS]);
    expected.push(format!("start {TEST} --doc"));
    expected.extend(one_after_another(TEST, &SHARDS[DOCTESTS + 1..]));
    assert_eq!(scratch.events(), expected, "a later shard did not run");
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("test shards: the doctests: Doc-tests brokkr_core ran past 2 s\n"),
        "{stdout}"
    );
}

/// A bound that is not a whole number above 0 refuses by name before
/// anything is built or run, and a good one is not inherited by a shard.
#[test]
fn a_bound_that_is_not_a_positive_whole_number_refuses_and_no_shard_inherits_it() {
    let real = metadata();
    for value in ["0", "soon", "-1", "1.5"] {
        let scratch = Scratch::new(&real, &script());
        let run = scratch.run(&["test"], &[("BROKKR_TEST_SHARD_BOUND_SECONDS", value)]);
        assert_eq!(run.status.code(), Some(1), "{value:?}: {run:?}");
        assert_eq!(
            String::from_utf8_lossy(&run.stderr),
            format!(
                "test shards: BROKKR_TEST_SHARD_BOUND_SECONDS '{value}' is not a whole number of \
                 seconds above 0\n"
            ),
            "{value:?}"
        );
        assert_eq!(scratch.calls(), Vec::<String>::new(), "{value:?}");
        assert_eq!(scratch.events(), Vec::<String>::new(), "{value:?}");
    }
    let scratch = Scratch::new(&real, &script());
    let run = scratch.run(&["test"], &[("BROKKR_TEST_SHARD_BOUND_SECONDS", "2")]);
    assert_eq!(run.status.code(), Some(0), "{run:?}");
    assert_eq!(scratch.lines("bound"), vec!["unset"; SHARDS.len()]);
}

/// A `deps` binary that leaves a stopped `/bin/sleep` helper outside `deps`
/// is still ended by the KILL after 10 s, so nothing of the group remains
/// and every later shard runs. The parent `stopped-0456` is the clocked test
/// binary; the `/bin/sleep` helper gets no clock, but is in its parent's
/// group. The recorded pid is the helper, and the bound names the parent.
#[test]
fn a_stopped_group_member_ended_by_the_kills_kill_is_not_a_survivor() {
    let (scratch, _) = assert_bounded_hang("STUB_STOPPED", FIRST, "stopped.pid", "stopped-0456");
    assert_bounded_first_shard_ran(&scratch);
}

/// The Bash 4 constructs macOS's stock Bash 3.2 does not have, as they would
/// be written: a named file descriptor, the case operators, associative
/// arrays, the bulk readers, a coprocess, `wait -n` and `test -v`. The
/// script is run by the Bash 3.2 macOS ships, so a construct it cannot parse
/// is not a syntax preference: the script would stop before any shard.
#[test]
fn the_shard_script_carries_no_construct_bash_32_cannot_run() {
    let mut found = Vec::new();
    for (number, line) in script().lines().enumerate() {
        let mut constructs = Vec::new();
        if line.contains("exec {") {
            constructs.push("a {fd} named redirection");
        }
        if line.contains("${") && line.contains(",,") {
            constructs.push("a ${var,,} lower-case expansion");
        }
        if line.contains("${") && line.contains("^^") {
            constructs.push("a ${var^^} upper-case expansion");
        }
        for construct in [
            "declare -A",
            "mapfile",
            "readarray",
            "coproc",
            "wait -n",
            "[[ -v",
        ] {
            if line.contains(construct) {
                constructs.push(construct);
            }
        }
        for construct in constructs {
            found.push(format!("{SCRIPT}:{}: {construct}: {line}", number + 1));
        }
    }
    assert_eq!(
        found,
        Vec::<String>::new(),
        "Bash 4 or later, which stock macOS does not have:\n{}",
        found.join("\n")
    );
}

/// Nothing reads a shard's log but the runner that owns its output: a `tail`
/// that fails on the first shard's log changes no verdict, and every shard
/// still runs and passes.
#[test]
fn a_failing_tail_cannot_fail_a_shard_that_reads_its_own_output() {
    let scratch = Scratch::new(&metadata(), &script());
    scratch.executable("tail", FAILING_TAIL);
    let run = scratch.run(&["test"], &[]);
    assert_whole_run_passes(&scratch, &run);
}

/// The pid pair the stub wrote to `file`: the group's leader, then a member.
fn recorded_pair(scratch: &Scratch, file: &str) -> (i32, i32) {
    let text = std::fs::read_to_string(scratch.dir.path().join(file))
        .expect("the stub recorded its group and member");
    let mut words = text.split_whitespace();
    let leader = words.next().expect("a leader").parse().expect("a pid");
    let member = words.next().expect("a member").parse().expect("a pid");
    (leader, member)
}

/// A cargo stub that ends 0 but leaves a sleeping member in its own group
/// cannot be shown to have ended: the shard is lost, names the group and,
/// where the process table names pids (Linux), the member, keeps its log,
/// and starts no later shard. macOS's `ps -axww -o pid=,pgid=,lstart=,comm=`
/// names the group alone in the survivor line, matching the runner's own
/// `platform.system()` branch.
#[test]
fn a_group_that_is_not_shown_ended_is_lost_and_names_its_members() {
    let scratch = Scratch::new(&metadata(), &script());
    let run = scratch.run(&["test"], &[("STUB_LEAVE", FIRST)]);
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    let (leader, member) = recorded_pair(&scratch, "leave.pid");
    let later: String = NAMES[1..]
        .iter()
        .map(|shard| {
            format!(
                "test shards: {shard} was not started: an earlier shard could not be shown to \
                 have ended, so its command may still be running and the shard cannot be shown \
                 to run alone\n"
            )
        })
        .collect();
    let named = if cfg!(target_os = "linux") {
        format!(" {member}")
    } else {
        String::new()
    };
    let expected = format!(
        "test shards: could not show every library, run_pages skipped's process group ended: process group \
         {leader} still has members{named}\n{later}"
    );
    assert_eq!(String::from_utf8_lossy(&run.stderr), expected);
    assert_eq!(
        scratch.events(),
        one_after_another(TEST, &SHARDS[..1]),
        "a later shard ran while the group could not be shown ended"
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("test shards: every library, run_pages skipped: could not show its process group ended\n"),
        "{stdout}"
    );
    let work = kept_work(&scratch);
    assert!(work.join("0.log").is_file(), "{work:?}");
    rustix::process::kill_process(
        rustix::process::Pid::from_raw(member).expect("a pid"),
        rustix::process::Signal::KILL,
    )
    .expect("the leftover member is ended");
    poll(|| !process_alive(member));
    assert!(
        !process_alive(member),
        "leftover member {member} still runs"
    );
}

/// A heap child past the bound the shard passes writes the shard's bound
/// record through the real launcher: the run fails by the child's binary
/// name, the final summary says the same, the launcher's own exit is not
/// also refused as a failed shard, and every later shard runs.
#[test]
fn a_heap_child_past_the_bound_reaches_the_shards_record_and_summary() {
    let scratch = Scratch::new(&metadata(), &script());
    scratch.executable("rustc", RUSTC);
    for heap in ["heap_claude", "heap_codex", "heap_dsh"] {
        let path = scratch.dir.path().join(format!("{heap}-0123abcd"));
        std::fs::write(&path, SLEEPING_HEAP).expect("a heap child");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
    }
    let run = scratch.run(
        &["test", "--parallel-heaps"],
        &[("BROKKR_TEST_SHARD_BOUND_SECONDS", "1")],
    );
    assert_eq!(run.status.code(), Some(1), "{run:?}");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert_eq!(
        stderr, "test shards: heap_claude-0123abcd ran past 1 s\n",
        "the launcher's exit or missing summary was refused too"
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        stdout.contains("test shards: the heap measurements: heap_claude-0123abcd ran past 1 s\n"),
        "{stdout}"
    );
    let mut expected = one_after_another(TEST, &SHARDS[..HEAPS]);
    expected.extend(one_after_another(TEST, &SHARDS[HEAPS + 1..]));
    assert_eq!(scratch.events(), expected, "a later shard did not run");
    for heap in ["heap_claude", "heap_codex", "heap_dsh"] {
        let pid = recorded_pid(&scratch, &format!("{heap}-0123abcd.pid"));
        poll(|| !process_alive(pid));
        assert!(!process_alive(pid), "{heap}-0123abcd {pid} still runs");
    }
}

/// The launcher takes the shard's bound with no default of its own: a direct
/// invocation without `--binary-bound` refuses by name before it reads
/// Cargo, so the 2,400 has one home.
#[test]
fn the_heap_launcher_refuses_without_the_bound_the_script_owns() {
    let scratch = tempfile::tempdir().expect("scratch");
    let cargo = scratch.path().join("cargo");
    std::fs::write(&cargo, "#!/usr/bin/env bash\nexit 1\n").expect("cargo stub");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let path = format!(
        "{}:{}",
        scratch.path().display(),
        std::env::var("PATH").expect("PATH")
    );
    let run = Command::new("python3")
        .arg("scripts/parallel-heaps.py")
        .current_dir(workspace())
        .env("PATH", path)
        .output()
        .expect("the heap launcher runs");
    assert_eq!(run.status.code(), Some(2), "{run:?}");
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        stderr.contains("--binary-bound") && stderr.contains("required"),
        "{stderr}"
    );
}
