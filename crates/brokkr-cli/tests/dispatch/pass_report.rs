//! Dispatch assertions use the report of the pass whose claim they exercise.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use brokkr_store::EntryId;

use super::{Machine, Standing};

pub(super) const CLAIMED: [&str; 4] = ["add", "claim", "link", "spawn"];

/// Re-execute the test alone and relay its skip line even on a passing run.
pub(super) fn in_its_own_dispatch_engine(test: &str) -> bool {
    const RERUN: &str = "BROKKR_TEST_DISPATCH_ENGINE";
    if std::env::var_os(RERUN).is_some() {
        return true;
    }
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", test, "--nocapture"])
        .env(RERUN, "1")
        .stdin(Stdio::null());
    let ended = Standing::spawned(&mut command).ended_within(Duration::from_secs(180));
    let stdout = String::from_utf8_lossy(&ended.stdout);
    let stderr = String::from_utf8_lossy(&ended.stderr);
    assert!(
        ended.status.success() && stdout.contains("test result: ok. 1 passed;"),
        "{test}: {}\n{stdout}\n{stderr}",
        ended.status
    );
    let skip = format!("{test}: ");
    for line in stderr.lines().filter(|line| line.starts_with(&skip)) {
        writeln!(std::io::stderr().lock(), "{line}").unwrap();
    }
    false
}

fn waiting_lines(stdout: &str) -> impl Iterator<Item = &str> {
    stdout.lines().filter(|line| line.starts_with("waiting:"))
}

/// Whether this pass had only machine waits and reported no claim.
pub(super) fn machine_only_wait(stdout: &str) -> bool {
    let claimed = [
        "started: ",
        "spawn failed: ",
        "left for reconciliation: ",
        "reclaimed: ",
        "held: ",
    ];
    let mut waits = waiting_lines(stdout).peekable();
    !stdout
        .lines()
        .any(|line| claimed.iter().any(|prefix| line.starts_with(prefix)))
        && waits.peek().is_some()
        && waits.all(machine_reasons)
}

fn machine_reasons(line: &str) -> bool {
    let Some((entry, reasons)) = line
        .strip_prefix("waiting: entry ")
        .and_then(|wait| wait.split_once(": "))
    else {
        return false;
    };
    let mut segments = reasons.split("; ");
    let first = segments.next().and_then(|segment| segment.split_once(": "));
    // A shortfall can contain a semicolon in its own words. A following
    // reason starts with another kind and colon.
    let rest = segments.filter_map(|segment| segment.split_once(": "));
    entry.parse::<EntryId>().is_ok()
        && first.is_some_and(|first| {
            std::iter::once(first).chain(rest).all(|(kind, words)| {
                !words.trim().is_empty()
                    && matches!(kind, "cpu_short" | "memory_short" | "memory_pressure")
            })
        })
}

pub(super) fn say_wait(test: &str, stdout: &str) {
    let waits = waiting_lines(stdout).collect::<Vec<_>>().join(" | ");
    eprintln!("{test}: the pass claimed nothing while waiting on the machine: {waits}");
}

pub(super) fn diagnostics(machine: &Machine, stdout: &str, stderr: &str) -> String {
    format!(
        "pass stdout:\n{stdout}\npass stderr:\n{stderr}\n{}",
        listed(machine)
    )
}

pub(super) fn assert_commands(machine: &Machine, stdout: &str, stderr: &str, expected: &[&str]) {
    assert_eq!(
        machine.commands(),
        expected,
        "{}",
        diagnostics(machine, stdout, stderr)
    );
}

/// A second measurement is useful only as a failure diagnostic.
fn listed(machine: &Machine) -> String {
    let listed = machine.brokkr(&["queue", "list", "--json", "--db", machine.db()]);
    format!(
        "queue list --json:\n{}",
        String::from_utf8_lossy(&listed.stdout)
    )
}

pub(super) fn check_start(test: &str, machine: &Machine, stdout: &str, stderr: &str) -> bool {
    if machine_only_wait(stdout) {
        assert_commands(machine, stdout, stderr, &["add"]);
        say_wait(test, stdout);
        return false;
    }
    assert_commands(machine, stdout, stderr, &CLAIMED);
    assert!(
        stdout.lines().any(|line| line.starts_with("started: ")),
        "{}",
        diagnostics(machine, stdout, stderr)
    );
    true
}

pub(super) fn first_report(dispatcher: &Standing) -> String {
    let mut stdout = dispatcher.said("queue: ");
    if let Some((facts, summary)) = stdout.split_once("queue: ") {
        let end = summary.find('\n').map_or(summary.len(), |i| i + 1);
        stdout.truncate(facts.len() + "queue: ".len() + end);
    }
    stdout
}

/// Start a standing dispatcher, and wait for its seat only if it claimed.
pub(super) fn standing_over_a_working_seat(
    test: &str,
    every: &str,
) -> Option<(Machine, Standing, String)> {
    let machine = Machine::new();
    machine.queued();
    let args = ["queue", "dispatch", "--every", every, "--db", machine.db()];
    let dispatcher = Standing::start(&machine, &args);
    let stdout = first_report(&dispatcher);
    let stderr = dispatcher.heard.lock().unwrap().clone();
    if !check_start(test, &machine, &stdout, &stderr) {
        return None;
    }
    machine.seat_runs();
    Some((machine, dispatcher, stdout))
}

/// A pass's first `started:` line: its words before the driver's log, and
/// that log's path, which it names last.
pub(super) fn started_line(stdout: &str) -> (&str, PathBuf) {
    let line = (stdout.lines())
        .find(|line| line.starts_with("started: "))
        .unwrap_or_else(|| panic!("no started line in {stdout}"));
    let (named, log) = (line.rsplit_once(", log "))
        .unwrap_or_else(|| panic!("the started line names no log: {line}"));
    (named, PathBuf::from(log))
}

/// The run, entry, claim and driver pid a pass's `started:` line names.
pub(super) fn started(stdout: &str) -> (String, String, String, i32) {
    let (line, _) = started_line(stdout);
    let words: Vec<&str> = line.split([' ', ',']).filter(|w| !w.is_empty()).collect();
    // started: run R under claim S of entry E driver pid P
    let pid = words[words.len() - 1].parse().unwrap();
    (
        words[2].to_string(),
        words[8].to_string(),
        words[5].to_string(),
        pid,
    )
}

const PRESSURE: &str = "waiting: entry 1: memory_pressure: kernel memory pressure is warn\n";
const CPU: &str = "waiting: entry 2: cpu_short: 0 millicores available\n";

#[test]
fn only_machine_waits_explain_no_claim() {
    let stdout = format!(
        "{PRESSURE}{CPU}waiting: entry 3: memory_short: memory is short; it needs more than the machine's 2 bytes\n\
         queue: 3 waiting, 0 held\n"
    );
    assert!(machine_only_wait(&stdout), "{stdout:?}");
}

#[test]
fn another_reason_beside_a_machine_wait_disallows_the_skip() {
    let stdout = format!(
        "{CPU}waiting: entry 1: memory_pressure: kernel memory pressure is warn; \
         not_started: waits on 3:completed\nqueue: 2 waiting, 0 held\n"
    );
    assert!(!machine_only_wait(&stdout), "{stdout:?}");
}

#[test]
fn a_claim_disallows_the_skip_even_with_a_machine_wait() {
    let stdout = format!(
        "started: run r under claim 2 of entry 2, driver pid 7, log /tmp/r.log\n{PRESSURE}\
         queue: 1 waiting, 0 held\n"
    );
    assert!(!machine_only_wait(&stdout), "{stdout:?}");
}

#[test]
fn no_waiting_line_gives_no_reason_to_skip() {
    let reports = [
        "queue: 0 waiting, 0 held\n",
        "waiting: entry 1: \nqueue: 1 waiting, 0 held\n",
    ];
    assert_eq!(reports.map(machine_only_wait), [false; 2], "{reports:?}");
}
