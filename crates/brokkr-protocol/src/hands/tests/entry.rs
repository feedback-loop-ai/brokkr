//! A server box's entry and its launch (decision 0065 slice two, U6c6b;
//! MB3, MB5): the entry names each mounted handle by its place among the
//! box's handles and hands the generated identity's tree over once, as one
//! closed shape; a launch that cannot stand refuses by its own cause, and
//! the entry removes that tree, never one not handed to it; and a scratch
//! that cannot hold the identity leaves the box not established, after
//! every cause the observer finds first.

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::json;

use super::super::*;
use super::server::{scratch_of, unservable, Host};
use crate::broker::{Network, Reach, Refusal};

/// A package server's box prepared under `host`, the test binary its
/// bootstrap, its writers confined.
fn boxed(host: &Host) -> Result<ServerBox, Refusal> {
    let entry = host.plant("opt/docs/bin/docs-mcp");
    let docs = ServerProgram::resolve(entry.to_str().unwrap(), &host.home).unwrap();
    let bootstrap = std::env::current_exe().unwrap();
    let seat = Reach {
        writable: vec![host.path("work")],
        readable: Vec::new(),
    };
    let profile = ServerProfile {
        reach: &seat,
        network: &Network::Isolated,
        bootstrap: &bootstrap,
        writers: &[],
    };
    ServerBox::prepare_with(&docs, &profile, &super::sources::confined())
}

#[test]
fn an_entry_names_each_mounted_handle_by_its_place_and_hands_its_identity_over_once() {
    if unservable() {
        return;
    }
    let host = Host::new();
    let mut server = boxed(&host).unwrap();
    let paths: Vec<PathBuf> = server.handles().map(|(path, _)| path.into()).collect();
    let numbers: Vec<String> = server
        .handles()
        .map(|(_, fd)| fd.as_raw_fd().to_string())
        .collect();
    // Each word is the argv's, but the number after each descriptor mount,
    // which only this process can read, is its handle's place instead.
    let mut words = Vec::new();
    let mut mounted = false;
    for word in server.argv() {
        words.push(match mounted {
            true => json!({"handle": numbers.iter().position(|number| number == word)}),
            false => json!({"text": word}),
        });
        mounted = word == "--ro-bind-fd";
    }
    let launcher = require_bwrap().unwrap();
    let launcher = paths.iter().position(|path| *path == launcher);
    // Handed to another live process, this test's own parent: the tree is
    // renamed to name it, where no reaper takes it while that one lives.
    let made = paths
        .iter()
        .find(|path| path.ends_with("etc/passwd"))
        .unwrap();
    let made = made.parent().unwrap().parent().unwrap().to_path_buf();
    let owner = rustix::process::Pid::as_raw(rustix::process::getppid()).unsigned_abs();
    let entry = server.entry(owner).unwrap();
    let scratch = scratch_of(&entry);
    let name = made.file_name().unwrap().to_str().unwrap();
    let own = format!("-{}-", std::process::id());
    let renamed = made.with_file_name(name.replacen(&own, &format!("-{owner}-"), 1));
    assert_eq!((made.exists(), scratch.clone()), (false, renamed));
    let expected = json!({
        "argv": words, "launcher": launcher, "bootstrap": std::env::current_exe().unwrap(),
        "identity": scratch,
    });
    assert_eq!(serde_json::to_value(&entry).unwrap(), expected);
    // Its handles name the tree where it now lies, each the object held.
    for (path, fd) in server.handles() {
        let named = std::fs::metadata(path).unwrap();
        let held = std::fs::File::from(fd.try_clone().unwrap())
            .metadata()
            .unwrap();
        assert_eq!((path, held.ino()), (path, named.ino()));
    }
    // Handed over once: the box no longer removes its identity's tree, and
    // a second entry has none to hand over.
    assert_eq!(server.entry(owner), Err(Refusal::Establishment));
    drop(server);
    assert!(scratch.join("etc/passwd").is_file());
    // Nor does a copy of the entry decoded in a process it was not handed
    // to; but the entry its maker drops, one it could not hand over, does.
    let copy: ServerEntry = serde_json::from_value(expected).unwrap();
    drop(copy);
    assert!(scratch.join("etc/passwd").is_file());
    drop(entry);
    assert!(!scratch.exists());
}

#[test]
fn a_launch_that_cannot_stand_refuses_by_its_own_cause_and_its_entry_removes_the_identity() {
    if unservable() {
        return;
    }
    let host = Host::new();
    let mut server = boxed(&host).unwrap();
    let entry = server.entry(std::process::id()).unwrap();
    let scratch = scratch_of(&entry);
    let handles: Vec<BorrowedFd<'_>> = server.handles().map(|(_, fd)| fd.as_fd()).collect();
    let launched = |entry: &ServerEntry, handles: &[BorrowedFd<'_>]| {
        ServerBox::launch(entry, handles, |_, _| Vec::new(), soon()).map(drop)
    };
    // An identity tree not handed to this process is never removed: one
    // naming it but outside the temporary directory, or one there naming
    // another owner. The launch refuses it, and the entry leaves it.
    let own = std::process::id();
    let outside = host.path(&format!("tree-{own}-outside"));
    let other = std::env::temp_dir().join(format!("u6c6b-owner-1-{own}"));
    for tree in [&outside, &other] {
        std::fs::create_dir_all(tree).unwrap();
        let mut named = serde_json::to_value(&entry).unwrap();
        named["identity"] = json!(tree);
        let named: ServerEntry = serde_json::from_value(named).unwrap();
        let answer = launched(&named, &handles);
        drop(named);
        assert_eq!(
            (tree, answer, tree.is_dir()),
            (tree, Err(Refusal::Identity), true)
        );
    }
    std::fs::remove_dir(other).unwrap();
    // An entry naming a handle the launch was not given.
    assert_eq!(launched(&entry, &handles[..1]), Err(Refusal::Identity));
    // A launcher that cannot start: its handle a directory's.
    let at = serde_json::to_value(&entry).unwrap()["launcher"].as_u64();
    let at = usize::try_from(at.unwrap()).unwrap();
    let root = std::fs::File::open("/").unwrap();
    let mut swapped = handles.clone();
    swapped[at] = root.as_fd();
    assert_eq!(launched(&entry, &swapped), Err(Refusal::Unavailable));
    // Whatever the launch answered, the tree is the entry's to remove.
    assert!(scratch.is_dir());
    drop(entry);
    assert!(!scratch.exists());
}

#[test]
fn an_entry_is_one_closed_shape() {
    let entry = json!({"argv": [{"text": "--ro-bind-fd"}, {"handle": 0}, {"text": "/usr"}],
        "launcher": 1, "bootstrap": "/brokkr", "identity": "/nowhere/u6c6b"});
    let decoded: ServerEntry = serde_json::from_value(entry.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), entry);
    // A key beside the four, or a word of another kind.
    let mut extra = entry.clone();
    extra["program"] = json!("/bin/sh");
    let mut word = entry;
    word["argv"][1] = json!({"fd": 3});
    for (case, shape) in [("extra", extra), ("word", word)] {
        let answer = serde_json::from_value::<ServerEntry>(shape).map_err(|error| error.classify());
        assert_eq!(
            (case, answer),
            (case, Err(serde_json::error::Category::Data))
        );
    }
}

#[test]
fn the_launch_hook_leaves_open_only_the_descriptors_it_names_and_reports_the_systems_error() {
    // Run in started shells, never here, so this process's own descriptors
    // stay as they are; a hook before it stands for a descriptor inherited
    // without close-on-exec. Each shell reports whether that stray, then
    // the carried one, is open: `0` where it is.
    let (carried, stray) = (File::open("/").unwrap(), File::open("/").unwrap());
    let [carried, stray] = [&carried, &stray].map(|file| file.as_raw_fd());
    let probe =
        format!("test -e /proc/self/fd/{stray}; s=$?; test -e /proc/self/fd/{carried}; echo $s $?");
    let started = |hook: Option<Vec<i32>>| {
        let mut shell = Command::new("/bin/sh");
        shell.args(["-c", &probe]).stdin(Stdio::null());
        // SAFETY: each hook only sets one flag of descriptors by number,
        // between the shell's fork and its exec.
        unsafe {
            shell.pre_exec(move || match libc::fcntl(stray, libc::F_SETFD, 0) {
                -1 => Err(std::io::Error::last_os_error()),
                _ => Ok(()),
            });
            if let Some(fds) = hook {
                shell.pre_exec(namespace::entry::inheritor(fds));
            }
        }
        let output = shell.output().map_err(|error| error.raw_os_error())?;
        Ok(String::from_utf8(output.stdout).unwrap())
    };
    // Unhooked, the stray enters the shell and the carried one does not;
    // hooked, the reverse.
    assert_eq!(started(None), Ok("0 1\n".to_string()));
    assert_eq!(started(Some(vec![carried])), Ok("1 0\n".to_string()));
    // A descriptor that is not open is the system's error, which the
    // launch reports as its child's failure to start.
    let closed = started(Some(vec![i32::MAX]));
    assert_eq!(closed, Err(Some(libc::EBADF)));
}

/// A launch's startup: its absolute deadline, and its cancellation.
type Startup<'a> = (Instant, &'a (dyn Fn() -> bool + Sync));

/// Never cancelled.
fn never() -> bool {
    false
}

/// A startup thirty seconds from its deadline, never cancelled.
pub(super) fn soon() -> Startup<'static> {
    (Instant::now() + Duration::from_secs(30), &never)
}

/// A started shell that touches `marker` once it runs, its hook `hook`,
/// started within `startup`: what the start answered, once the shell has
/// ended, and how long it took.
fn start(
    marker: &Path,
    hook: impl FnMut() -> std::io::Result<()> + Send + Sync + 'static,
    startup: Startup<'_>,
) -> (Result<(), Refusal>, Duration) {
    let mut shell = Command::new("/bin/sh");
    let touch = format!("touch {}", marker.display());
    shell.args(["-c", &touch]).stdin(Stdio::null());
    let begun = Instant::now();
    let started = namespace::entry::started(shell, hook, startup);
    let ended = started.map(|mut shell| assert!(shell.wait().unwrap().success()));
    (ended, begun.elapsed())
}

#[test]
fn a_launch_starts_within_the_startup_or_never_runs_and_is_settled() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("ran");
    // In time, the started process is released and runs.
    assert_eq!(start(&marker, || Ok(()), soon()).0, Ok(()));
    std::fs::remove_file(&marker).unwrap();
    // A startup already past its deadline, or cancelled, never releases
    // it: it fails its own start, its hook never runs, and nor does it.
    let gone = || true;
    let far = Instant::now() + Duration::from_secs(30);
    let cases: [(&str, Startup<'_>); 2] = [
        ("deadline", (Instant::now(), &never)),
        ("cancelled", (far, &gone)),
    ];
    for (case, startup) in cases {
        let (mut signs, signing) = std::io::pipe().unwrap();
        let fd = signing.as_raw_fd();
        let sign = move || {
            // SAFETY: a `write` of a static byte to a pipe this test holds
            // open, between fork and exec.
            unsafe { libc::write(fd, b"r".as_ptr().cast(), 1) };
            Ok(())
        };
        let (answer, _) = start(&marker, sign, startup);
        drop(signing);
        let mut signed = Vec::new();
        signs.read_to_end(&mut signed).unwrap();
        let ran = (signed, marker.exists());
        assert_eq!(
            (case, answer, ran),
            (case, Err(Refusal::Establishment), (vec![], false))
        );
    }
    // Released, a start that stalls before its exec, its hook sleeping
    // five seconds, is killed through its pidfd at the deadline, or once
    // the attempt is cancelled, and reaped: no process of it is left.
    for (case, cancelling) in [("deadline", false), ("cancelled", true)] {
        let begun = Instant::now();
        let late = move || cancelling & (begun.elapsed() >= Duration::from_millis(200));
        let deadline = match cancelling {
            true => far,
            false => begun + Duration::from_millis(200),
        };
        let startup: Startup<'_> = (deadline, &late);
        let (mut named, naming) = std::io::pipe().unwrap();
        let fd = naming.as_raw_fd();
        let stall = move || {
            // SAFETY: `getpid`, a `write` of a stack buffer to a pipe this
            // test holds open, and a `sleep`, between fork and exec.
            unsafe {
                let pid = libc::getpid().to_ne_bytes();
                libc::write(fd, pid.as_ptr().cast(), pid.len());
                libc::sleep(5);
            }
            Ok(())
        };
        let (answer, took) = start(&marker, stall, startup);
        drop(naming);
        let mut pid = [0; 4];
        named.read_exact(&mut pid).unwrap();
        let left = Path::new(&format!("/proc/{}", i32::from_ne_bytes(pid))).exists();
        let ran = marker.exists();
        let fields = (case, answer, ran, left);
        assert_eq!(fields, (case, Err(Refusal::Establishment), false, false));
        assert!(took < Duration::from_secs(2), "{case}: {took:?}");
    }
}

#[test]
fn a_launcher_that_dies_with_its_parent_outlives_its_own_start() {
    // A parent-death signal, as bubblewrap's `--die-with-parent` arms one,
    // follows the thread that started the process: the start is made on
    // the caller's, so the launcher lives on once its start has ended.
    let die = || {
        // SAFETY: one `prctl` of this process's own flag, between fork and
        // exec.
        match unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) } {
            0 => Ok(()),
            _ => Err(std::io::Error::last_os_error()),
        }
    };
    let mut shell = Command::new("/bin/sh");
    shell.args(["-c", "sleep 0.2"]).stdin(Stdio::null());
    let mut launcher = namespace::entry::started(shell, die, soon()).unwrap();
    let ended = launcher.wait().unwrap();
    assert_eq!((ended.code(), ended.signal()), (Some(0), None));
}

#[test]
fn a_started_launcher_names_itself_and_runs_on_only_its_release() {
    // The hook is run here, in this process, so it is measured: it names
    // this process, then reads one byte, which releases it only where it
    // is the release, and fails the start otherwise.
    for (answer, expected) in [(1, Ok(())), (0, Err(Some(libc::ECANCELED)))] {
        let (mut report, reporter) = std::io::pipe().unwrap();
        let (hold, mut releaser) = std::io::pipe().unwrap();
        releaser.write_all(&[answer]).unwrap();
        let ends = [reporter.as_raw_fd(), hold.as_raw_fd()];
        let mut hook = namespace::entry::reporter(ends);
        let hooked = hook().map_err(|error| error.raw_os_error());
        drop(reporter);
        let mut named = Vec::new();
        report.read_to_end(&mut named).unwrap();
        let pid = std::process::id().to_ne_bytes().to_vec();
        assert_eq!((answer, hooked, named), (answer, expected, pid));
    }
}

/// What a sealing run of this binary reads: that it is one.
const SEALING: &str = "BROKKR_TEST_SEALING";

/// [`the_launch_hook_run_in_a_process_of_its_own_seals_all_it_does_not_name`]
/// by its full name, which a sealing run is filtered to.
const SEALED: &str = "hands::tests::entry::\
    the_launch_hook_run_in_a_process_of_its_own_seals_all_it_does_not_name";

/// The close-on-exec flag of each of `files`.
fn sealed(files: [&File; 2]) -> [i32; 2] {
    // SAFETY: one `fcntl` read of a flag of a descriptor this test holds.
    files.map(|file| unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC)
}

/// Run as itself, this test reruns its binary, filtered to itself; so
/// rerun, it runs the launch hook in this process, which it seals, so the
/// hook is measured where it ran: the descriptor it names is left open
/// across an exec, a stray inherited without close-on-exec is not, and a
/// descriptor that is not open is the system's error.
#[test]
fn the_launch_hook_run_in_a_process_of_its_own_seals_all_it_does_not_name() {
    if std::env::var_os(SEALING).is_some() {
        let (carried, stray) = (File::open("/").unwrap(), File::open("/").unwrap());
        // SAFETY: one `fcntl` clearing a flag of a descriptor this test
        // holds, as one inherited without close-on-exec stands.
        unsafe { libc::fcntl(stray.as_raw_fd(), libc::F_SETFD, 0) };
        let mut hook = namespace::entry::inheritor(vec![carried.as_raw_fd()]);
        assert_eq!(hook().map_err(|error| error.raw_os_error()), Ok(()));
        assert_eq!(sealed([&carried, &stray]), [0, libc::FD_CLOEXEC]);
        let mut closed = namespace::entry::inheritor(vec![i32::MAX]);
        let failed = closed().map_err(|error| error.raw_os_error());
        assert_eq!(failed, Err(Some(libc::EBADF)));
        return;
    }
    let rerun = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", SEALED, "--test-threads=1"])
        .env(SEALING, "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    assert!(rerun.unwrap().success());
}

/// What a scratchless run of this binary reads: that it is one.
const SCRATCHLESS: &str = "BROKKR_TEST_SCRATCHLESS";

/// [`a_scratch_that_cannot_hold_the_identity_leaves_the_box_not_established`]
/// by its full name, which a scratchless run is filtered to.
const UNSCRATCHED: &str = "hands::tests::entry::\
    a_scratch_that_cannot_hold_the_identity_leaves_the_box_not_established";

/// Run as itself, this test reruns its binary, filtered to itself, with a
/// temporary directory no scratch can be made in; so rerun, it is the
/// proof: the box is not established, but a cause the observer finds,
/// a multiply-linked program file, still comes first (MB3's order).
#[test]
fn a_scratch_that_cannot_hold_the_identity_leaves_the_box_not_established() {
    if std::env::var_os(SCRATCHLESS).is_some() {
        let host = Host::new();
        assert_eq!(boxed(&host).map(drop), Err(Refusal::Establishment));
        let program = host.path("opt/docs/bin/docs-mcp");
        std::fs::hard_link(&program, host.path("opt/docs/bin/again")).unwrap();
        assert_eq!(boxed(&host).map(drop), Err(Refusal::Linked));
        return;
    }
    if unservable() {
        return;
    }
    let rerun = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", UNSCRATCHED, "--test-threads=1"])
        .env(SCRATCHLESS, "1")
        .env("TMPDIR", "/proc/u6c6b")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    assert!(rerun.unwrap().success());
}
