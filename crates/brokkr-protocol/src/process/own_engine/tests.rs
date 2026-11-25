//! A rerun's deadline: one that ends is read whole; one that never does
//! fails the test, killed and reaped first.

use std::sync::mpsc::{channel, RecvTimeoutError};

use rustix::io::Errno;
use rustix::process::{kill_process, test_kill_process, Pid, Signal};

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

/// The pid a shell wrote to `path`.
fn written(path: &std::path::Path) -> Pid {
    let pid = std::fs::read_to_string(path).unwrap();
    Pid::from_raw(pid.trim().parse().unwrap()).unwrap()
}

/// `stuck` run by [`ended`] to `bound` on a thread of its own: the panic
/// it fails with, which must come within [`PATIENCE`] however [`ended`]
/// behaves.
fn overrun(stuck: Command, bound: Duration) -> String {
    let (done, finished) = channel::<()>();
    let rerun = std::thread::spawn(move || {
        let _done = done;
        ended(stuck, bound)
    });
    let waited = finished.recv_timeout(PATIENCE);
    assert_eq!(waited, Err(RecvTimeoutError::Disconnected));
    *rerun.join().unwrap_err().downcast::<String>().unwrap()
}

/// How a rerun stuck past `bound` fails, before what it printed.
fn killed(bound: Duration) -> String {
    format!("the rerun ends within {bound:?}; killed, reaped by signal 9, it printed:\n")
}

/// How long a stuck rerun runs before it is killed.
const STUCK: Duration = Duration::from_secs(2);

/// How long a rerun that writes without pause runs before it is killed:
/// long enough that [`ended`] drains more than [`KEPT`] from it on a loaded
/// host (#1310). [`ended`] reads a [`CHUNK`] a poll, every 20 ms, so the
/// tail fills only after 16 full polls; one loaded lane read 827,392 bytes
/// in 2 s, and ten seconds is five times that margin.
const WRITING: Duration = Duration::from_secs(10);

#[test]
fn a_rerun_whose_child_never_exits_fails_within_patience_and_leaves_no_child() {
    let dir = tempfile::tempdir().unwrap();
    let pid = dir.path().join("pid");
    let stuck = shell(&pid, "echo stuck; exec sleep 600");
    let failed = overrun(stuck, STUCK);
    assert_eq!(failed, format!("{}stuck\n", killed(STUCK)));
    assert_eq!(test_kill_process(written(&pid)), Err(Errno::SRCH));
}

/// A rerun that writes without pause still fails at its deadline, showing
/// only the tail of what it wrote.
#[test]
fn a_rerun_that_never_stops_writing_fails_within_patience_keeping_its_tail() {
    let dir = tempfile::tempdir().unwrap();
    let pid = dir.path().join("pid");
    let failed = overrun(shell(&pid, "exec yes"), WRITING);
    let tail = failed.strip_prefix(&killed(WRITING)).unwrap();
    assert_eq!(tail.len(), KEPT);
    assert!(tail.bytes().all(|byte| b"y\n".contains(&byte)));
    assert_eq!(test_kill_process(written(&pid)), Err(Errno::SRCH));
}

/// Whether the killed `pid` is dead. On Linux an orphan's reaper is
/// whichever subreaper above the rerun adopted it, so dead is gone or that
/// reaper's zombie.
#[cfg(target_os = "linux")]
fn dead(pid: Pid) -> bool {
    let row = crate::process::table::stat_of(pid.as_raw_pid()).unwrap();
    row.is_none_or(|row| row.zombie)
}

/// Whether the killed `pid` is dead: on macOS, `launchd` reaps an orphan,
/// so dead is gone.
#[cfg(not(target_os = "linux"))]
fn dead(pid: Pid) -> bool {
    test_kill_process(pid) == Err(Errno::SRCH)
}

/// Kill the descendant whose pid a shell wrote to `held`, and see it dead.
fn kill_holder(held: &std::path::Path) {
    let holder = written(held);
    kill_process(holder, Signal::KILL).unwrap();
    let ended = until(Instant::now() + PATIENCE, || dead(holder).then_some(()));
    assert_eq!(ended, Some(()));
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
    let failed = overrun(stuck, STUCK);
    kill_holder(&held);
    assert_eq!(failed, format!("{}stuck\n", killed(STUCK)));
    assert_eq!(test_kill_process(written(&pid)), Err(Errno::SRCH));
}

/// A rerun that ends long before its bound while a descendant holds its
/// output fails [`GRACE`] after its end, not its bound, showing what it
/// printed.
#[test]
fn a_rerun_that_ends_with_its_output_held_fails_within_grace_showing_it() {
    let dir = tempfile::tempdir().unwrap();
    let held = dir.path().join("held");
    let mut early = shell(
        &dir.path().join("pid"),
        "echo early; sleep 600 & echo $! > \"$1\"",
    );
    early.arg(&held);
    let failed = overrun(early, PATIENCE);
    kill_holder(&held);
    let shown = "the rerun's output ends within 5s of its end; it printed:\nearly\n";
    assert_eq!(failed, shown);
}

/// Every process-wide PATH write must happen in a rerun so sibling tests
/// and the children they spawn keep the parent test binary's PATH (#1368).
#[test]
fn every_lib_test_that_writes_path_runs_in_an_engine_of_its_own() {
    use std::{fs, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut directories = vec![root.clone()];
    let mut read_own_engine = false;
    let mut writes = Vec::new();
    while let Some(dir) = directories.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                directories.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
                let relative = path.strip_prefix(&root).unwrap();
                read_own_engine |= relative == Path::new("process/own_engine.rs");
                writes.extend(
                    path_writes(&source).into_iter().map(|(line, isolated)| {
                        (format!("{}:{line}", relative.display()), isolated)
                    }),
                );
            }
        }
    }

    assert!(read_own_engine, "process/own_engine.rs unread");
    writes.sort_unstable();
    assert_eq!(
        writes
            .iter()
            .map(|(path, _)| path.as_str())
            .collect::<Vec<_>>(),
        [
            "adapters/agy/tests.rs:1002",
            "adapters/dsh_hands/tests.rs:1445",
            "probe/tests.rs:951",
            "probe/tests.rs:954",
        ],
        "the walk must name every PATH write"
    );
    let offenders: Vec<_> = writes
        .iter()
        .filter_map(|(path, isolated)| (!isolated).then_some(path.as_str()))
        .collect();
    assert!(
        offenders.is_empty(),
        "PATH writes without an engine of their own: {offenders:?}"
    );
}

/// Find guard calls with PATH on the same line or the next non-blank line,
/// and record whether the enclosing function has already called the rerun.
fn path_writes(source: &str) -> Vec<(usize, bool)> {
    let mut lines = source
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty())
        .peekable();
    let mut writes = Vec::new();
    let mut isolated = false;
    while let Some((number, line)) = lines.next() {
        if line.starts_with("//") {
            continue;
        }
        if line.split_whitespace().any(|word| word == "fn") {
            isolated = false;
        }
        let next = lines.peek().map_or("", |(_, line)| *line);
        let path = "\"PATH\"";
        if [".set(", ".remove("].iter().any(|method| {
            line.match_indices(method).any(|(index, _)| {
                let argument = &line[index + method.len()..];
                argument.starts_with(path) || (argument.is_empty() && next.starts_with(path))
            })
        }) {
            writes.push((number, isolated));
        }
        isolated |= line.contains("in_its_own_engine(");
    }
    writes
}
