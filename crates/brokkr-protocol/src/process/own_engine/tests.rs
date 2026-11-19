//! A rerun's deadline: one that ends is read whole; one that never does
//! fails the test, killed and reaped first.

use rustix::io::Errno;
use rustix::process::{kill_process, test_kill_process, Pid, Signal};

use super::*;
use crate::process::table::stat_of;

/// How long any wait here may take before the test fails.
const PATIENCE: Duration = Duration::from_secs(30);

/// A shell that writes its pid to the file its `$0` names, then runs
/// `rest`.
fn shell(pid: &std::path::Path, rest: &str) -> Command {
    let mut shell = Command::new("sh");
    let script = format!("echo $$ > \"$0\"; {rest}");
    shell.args(["-c", &script]).arg(pid);
    shell
}

#[test]
fn a_rerun_that_ends_is_read_whole_on_both_its_streams() {
    let dir = tempfile::tempdir().unwrap();
    let shell = shell(&dir.path().join("pid"), "echo out; echo err >&2; exit 3");
    let (status, shown) = ended(shell, PATIENCE);
    assert_eq!((status.code(), &shown[..]), (Some(3), "out\nerr\n"));
}

/// The pid a shell wrote to `path`.
fn written(path: &std::path::Path) -> Pid {
    let pid = std::fs::read_to_string(path).unwrap();
    Pid::from_raw(pid.trim().parse().unwrap()).unwrap()
}

/// `stuck` run by [`ended`] to a 2 s bound: the panic it fails with,
/// within [`PATIENCE`].
fn overrun(stuck: Command) -> String {
    let begun = Instant::now();
    let rerun = std::panic::AssertUnwindSafe(|| ended(stuck, Duration::from_secs(2)));
    let failed = std::panic::catch_unwind(rerun).unwrap_err();
    assert!(begun.elapsed() < PATIENCE, "{:?}", begun.elapsed());
    *failed.downcast::<String>().unwrap()
}

/// How a stuck rerun that printed `stuck` fails.
const KILLED: &str = "the rerun ends within 2s; killed, reaped by signal 9, it printed:\nstuck\n";

#[test]
fn a_rerun_whose_child_never_exits_fails_within_patience_and_leaves_no_child() {
    let dir = tempfile::tempdir().unwrap();
    let pid = dir.path().join("pid");
    let failed = overrun(shell(&pid, "echo stuck; exec sleep 600"));
    assert_eq!(failed, KILLED);
    assert_eq!(test_kill_process(written(&pid)), Err(Errno::SRCH));
}

/// A descendant the kill does not reach keeps the output's pipe open: the
/// rerun still fails in time, showing what it printed before.
#[test]
fn a_rerun_whose_descendant_holds_its_output_fails_within_patience_showing_it() {
    let dir = tempfile::tempdir().unwrap();
    let pid = dir.path().join("pid");
    let held = dir.path().join("held");
    let mut stuck = shell(&pid, "echo stuck; sleep 600 & echo $! > \"$1\"; wait");
    stuck.arg(&held);
    let failed = overrun(stuck);
    let holder = written(&held);
    kill_process(holder, Signal::KILL).unwrap();
    assert_eq!(failed, KILLED);
    assert_eq!(test_kill_process(written(&pid)), Err(Errno::SRCH));
    // The orphan's reaper is whichever subreaper above the rerun adopted
    // it: dead, it is gone or that reaper's zombie.
    let holder = holder.as_raw_pid();
    let dead = || stat_of(holder).unwrap().is_none_or(|row| row.zombie);
    assert_eq!(
        until(Instant::now() + PATIENCE, || dead().then_some(())),
        Some(())
    );
}
