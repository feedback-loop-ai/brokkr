//! One test played in an engine of its own (#403, #484).

use std::io::{ErrorKind, PipeReader, Read};
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Set in the re-executed test binary: the one test it runs is already in
/// an engine of its own.
const OWN_ENGINE: &str = "BROKKR_TEST_OWN_ENGINE";

/// How long a re-executed test may run before its parent fails it.
const RERUN: Duration = Duration::from_secs(180);

/// How long a rerun killed at its deadline may take to be reaped, and one
/// that ended may take, from its end, to close its output.
const GRACE: Duration = Duration::from_secs(5);

/// The most of a rerun's output read in one poll: the pipe's capacity, so
/// a rerun that writes without pause cannot hold the poll past its deadline.
const CHUNK: usize = 64 * 1024;

/// The most of a rerun's output kept: its tail, where libtest's summary is.
const KEPT: usize = 1024 * 1024;

/// Whether this test runs in an engine of its own. An in-process launch is
/// one of the test binary's engine's attempts, and every test in the binary
/// shares that engine as its subreaper. An orphan another test hands it (a
/// hands box's `bwrap` namespace init, which outlives the `bwrap` that
/// started it) can read as a stray of this test's launches and refuse them,
/// so such a test re-executes this binary on `test`, its full path, alone.
/// True in the re-executed child, which runs the body; false in the parent,
/// once it has asserted that exactly one test passed there. The child's
/// own output is shown when it did not. A child that has not ended within
/// [`RERUN`] fails the test rather than hold it forever ([`ended`]).
pub fn in_its_own_engine(test: &str) -> bool {
    if std::env::var_os(OWN_ENGINE).is_some() {
        return true;
    }
    let mut rerun = Command::new(std::env::current_exe().unwrap());
    rerun.args(["--exact", test]).env(OWN_ENGINE, "1");
    let (status, shown) = ended(rerun, RERUN);
    let passed = shown.contains("test result: ok. 1 passed;");
    assert_eq!((status.code(), passed), (Some(0), true), "{shown}");
    false
}

/// `command`'s status and what it printed, its standard input closed and
/// its output and error one pipe, read without blocking as it waits, a
/// [`CHUNK`] a poll and its last [`KEPT`] bytes kept. One that has not
/// ended within `bound` is killed and reaped within [`GRACE`] before the
/// test fails, showing what it printed so far: it leaves no child behind,
/// and no reader that a descendant holding the pipe blocks. One whose
/// output is still open [`GRACE`] after it ended fails the test the same
/// way.
fn ended(mut command: Command, bound: Duration) -> (ExitStatus, String) {
    let (mut reader, writer) = std::io::pipe().unwrap();
    rustix::io::ioctl_fionbio(&reader, true).unwrap();
    command
        .stdin(Stdio::null())
        .stdout(writer.try_clone().unwrap())
        .stderr(writer);
    let mut child = command.spawn().unwrap();
    // The command holds the pipe's write ends: dropped, the reader ends
    // with the child's output.
    drop(command);
    let mut shown = Vec::new();
    let deadline = Instant::now() + bound;
    let status = until(deadline, || {
        drain(&mut reader, &mut shown);
        child.try_wait().unwrap()
    });
    let Some(status) = status else {
        child.kill().unwrap();
        let reaped = until(Instant::now() + GRACE, || child.try_wait().unwrap());
        drain(&mut reader, &mut shown);
        let signal = reaped.and_then(|status| status.signal());
        let reaped = signal.map_or("not by a signal".to_owned(), |signal| {
            format!("by signal {signal}")
        });
        let shown = String::from_utf8_lossy(&shown);
        panic!("the rerun ends within {bound:?}; killed, reaped {reaped}, it printed:\n{shown}");
    };
    let read = until(Instant::now() + GRACE, || drain(&mut reader, &mut shown));
    let shown = String::from_utf8_lossy(&shown).into_owned();
    let Some(()) = read else {
        panic!("the rerun's output ends within {GRACE:?} of its end; it printed:\n{shown}");
    };
    (status, shown)
}

/// Read at most a [`CHUNK`] of what `reader` holds now into `shown`, which
/// keeps its last [`KEPT`] bytes: `Some` once its writers are all gone,
/// `None` while they are not.
fn drain(reader: &mut PipeReader, shown: &mut Vec<u8>) -> Option<()> {
    let mut chunk = vec![0; CHUNK];
    let read = reader.read(&mut chunk);
    if read
        .as_ref()
        .is_err_and(|error| error.kind() == ErrorKind::WouldBlock)
    {
        return None;
    }
    let read = read.expect("the rerun's output reads");
    shown.extend_from_slice(&chunk[..read]);
    shown.drain(..shown.len().saturating_sub(KEPT));
    (read == 0).then_some(())
}

/// `probe`'s first answer before `deadline`, asked every 20 ms.
fn until<T>(deadline: Instant, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    loop {
        let found = probe();
        if found.is_some() || Instant::now() >= deadline {
            return found;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests;
