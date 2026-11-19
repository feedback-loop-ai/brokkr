//! A rerun's deadline: one that ends is read whole; one that never does
//! fails the test, killed and reaped first.

use rustix::io::Errno;
use rustix::process::{test_kill_process, Pid};

use super::*;

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

#[test]
fn a_rerun_whose_child_never_exits_fails_within_patience_and_leaves_no_child() {
    let dir = tempfile::tempdir().unwrap();
    let pid = dir.path().join("pid");
    let stuck = shell(&pid, "echo stuck; exec sleep 600");
    let begun = Instant::now();
    let bound = Duration::from_secs(2);
    let rerun = std::panic::AssertUnwindSafe(|| ended(stuck, bound));
    let failed = std::panic::catch_unwind(rerun).unwrap_err();
    assert!(begun.elapsed() < PATIENCE, "{:?}", begun.elapsed());
    let failed = failed.downcast::<String>().unwrap();
    let killed = "the rerun ends within 2s; killed, reaped Some(ExitStatus(unix_wait_status(9))), \
        it printed:\nstuck\n";
    assert_eq!(*failed, killed);
    let pid = std::fs::read_to_string(&pid).unwrap();
    let pid = Pid::from_raw(pid.trim().parse().unwrap()).unwrap();
    assert_eq!(test_kill_process(pid), Err(Errno::SRCH));
}
