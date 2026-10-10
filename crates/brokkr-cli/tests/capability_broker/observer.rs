//! `serve`'s private observer (decision 0065 slice two U6c5b; MB3, MB4,
//! SC1): the sources admission observes afresh compared with what the plan
//! sealed, the handles the observer hands back being the very objects it
//! checked, and its supervision under the startup deadline, which a
//! blocked source operation, an admitted launcher held at its run (U6c5c)
//! or a cancelled attempt ends with nothing of admission left running.

use std::ffi::OsStr;
use std::io::{IoSlice, IoSliceMut, Read};
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rustix::net::{AddressFamily, SendAncillaryBuffer, SendAncillaryMessage, SendFlags};
use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags};
use rustix::net::{SocketFlags, SocketType};
use rustix::process::Pid;
use serde_json::{json, Value};

use super::{admitted, bootstrap, brokkr, chmod, euid, refused, setpriv, skip, unservable};
use super::{Refusal, Sealed, Store, HARNESS};

/// A sequenced-packet pair, both ends closed on exec unless handed on.
pub(super) fn pair() -> (OwnedFd, OwnedFd) {
    let (unix, packets) = (AddressFamily::UNIX, SocketType::SEQPACKET);
    rustix::net::socketpair(unix, packets, SocketFlags::CLOEXEC, None).unwrap()
}

/// The one record waiting on `socket`, parsed, and the handles riding it;
/// a null record where the observer handed none back.
pub(super) fn received(socket: &OwnedFd) -> (Value, Vec<OwnedFd>) {
    let mut bytes = vec![0; 128 << 10];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(64))];
    let mut control = RecvAncillaryBuffer::new(&mut space);
    let mut iov = [IoSliceMut::new(&mut bytes)];
    let flags = RecvFlags::DONTWAIT | RecvFlags::CMSG_CLOEXEC;
    let message = rustix::net::recvmsg(socket, &mut iov, &mut control, flags);
    let mut handles = Vec::new();
    for carried in control.drain() {
        if let RecvAncillaryMessage::ScmRights(fds) = carried {
            handles.extend(fds);
        }
    }
    let record = match message {
        Ok(message) if message.bytes > 0 => {
            serde_json::from_slice(&bytes[..message.bytes]).unwrap()
        }
        Ok(_) | Err(_) => Value::Null,
    };
    (record, handles)
}

/// The private tree of the generated identity an admitted observation hands
/// over with its entry (U6c6b), which the launch removes once its box is
/// settled; none where the record holds no entry.
pub(super) fn handed_over(record: &Value) -> Option<PathBuf> {
    serde_json::from_value(record["observed"]["entry"]["identity"].clone()).ok()
}

/// Remove the tree `record` hands over, where no broker launches its box.
pub(super) fn released(record: &Value) {
    if let Some(tree) = handed_over(record) {
        std::fs::remove_dir_all(tree).unwrap();
    }
}

/// Whether the shell standing for a broker's harness runs under
/// `no_new_privs`, as the broker it starts does either way.
#[derive(Debug, Clone, Copy)]
pub(super) enum Harness {
    Confined,
    Unconfined,
}

/// What a relaying run of this binary reads: the binary under test, the
/// plan's locator and its digest, a line each.
const RELAY: &str = "BROKKR_TEST_OBSERVER_RELAY";

/// [`relays_an_observation_its_confined_broker_and_harness_started`] by
/// its full name, which a relaying run is filtered to.
const RELAYING: &str =
    "capability_broker::observer::relays_an_observation_its_confined_broker_and_harness_started";

/// What `broker observe` hands back for the plan `sealed` holds at
/// `digest`, observed as `serve` observes it (U6c5c): started by a broker
/// under `no_new_privs`, itself started by a shell standing for its
/// harness. That broker is this test binary, rerun as one test, which
/// hands the record and its handles on over its standard input.
pub(super) fn relayed(sealed: &Sealed, digest: &str, harness: Harness) -> (Value, Vec<OwnedFd>) {
    let (ours, theirs) = pair();
    let this = std::env::current_exe().unwrap();
    let mut command = match harness {
        Harness::Confined => Command::new(setpriv()),
        Harness::Unconfined => Command::new("/bin/sh"),
    };
    match harness {
        Harness::Confined => command.args(["--no-new-privs", "/bin/sh", "-c", HARNESS]),
        Harness::Unconfined => command.args(["-c", HARNESS, "setpriv", "--no-new-privs"]),
    };
    command
        .arg(this)
        .args(["--exact", RELAYING, "--test-threads=1"]);
    let named = format!(
        "{}\n{}\n{digest}\n{}",
        brokkr().display(),
        sealed.locator.display(),
        std::process::id()
    );
    command.env(RELAY, named).env("HOME", &sealed.home);
    command
        .stdin(theirs)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    assert!(command.status().unwrap().success());
    drop(command);
    received(&ours)
}

/// Rerun by [`relayed`], the broker it stands for: `broker observe` of the
/// plan the relay names, its record and handles handed on over this run's
/// standard input; or by [`adopted`], the attempt it stands for. Run as
/// itself, the proof that the observer reads its harness's credentials
/// beside its broker's and its own: under a confined harness the writers
/// are this user's, under an unconfined one none are proved confined.
#[test]
fn relays_an_observation_its_confined_broker_and_harness_started() {
    if let Some(named) = std::env::var_os(RELAY) {
        return relay(named.to_str().unwrap());
    }
    if let Some(named) = std::env::var_os(ADOPTING) {
        return adopt(named.to_str().unwrap());
    }
    let Some((sealed, _, digest)) = sealed_as_observed() else {
        return;
    };
    let writers = |harness| {
        let (record, _) = relayed(&sealed, &digest, harness);
        released(&record);
        record["observed"]["writers"].clone()
    };
    assert_eq!(writers(Harness::Confined), json!([euid()]));
    assert_eq!(writers(Harness::Unconfined), Value::Null);
}

/// The record `record`, the tree it hands over (U6c6b) renamed to name
/// `owner`, the test process this relay stands in for, as its owner: a relay
/// ends at once, and a dead owner's tree is any reaper's to remove before
/// the test reads it.
fn kept_for(record: Value, owner: &str) -> Value {
    let Some(tree) = handed_over(&record) else {
        return record;
    };
    let name = tree.file_name().unwrap().to_str().unwrap();
    let relay = format!("-{}-", std::process::id());
    let renamed = tree.with_file_name(name.replacen(&relay, &format!("-{owner}-"), 1));
    std::fs::rename(&tree, &renamed).unwrap();
    let (from, to) = (tree.to_str().unwrap(), renamed.to_str().unwrap());
    serde_json::from_str(&record.to_string().replace(from, to)).unwrap()
}

/// Observe the plan `named` names with the binary it names, in a process
/// group of its own as a broker starts it, and hand the record and the
/// handles riding it on over standard input.
fn relay(named: &str) {
    let mut lines = named.lines();
    let [Some(binary), Some(locator), Some(digest), Some(owner)] =
        [lines.next(), lines.next(), lines.next(), lines.next()]
    else {
        panic!("relay {named:?}");
    };
    let (ours, theirs) = pair();
    let mut command = Command::new(binary);
    command.args([
        "broker",
        "observe",
        "--plan",
        locator,
        "--plan-digest",
        digest,
    ]);
    command
        .stdin(Stdio::null())
        .stdout(theirs)
        .stderr(Stdio::null())
        .process_group(0);
    command.status().unwrap();
    drop(command);
    let (record, handles) = received(&ours);
    let bytes = serde_json::to_vec(&kept_for(record, owner)).unwrap();
    let fds: Vec<BorrowedFd<'_>> = handles.iter().map(AsFd::as_fd).collect();
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(64))];
    let mut control = SendAncillaryBuffer::new(&mut space);
    assert!(fds.is_empty() || control.push(SendAncillaryMessage::ScmRights(&fds)));
    let iov = [IoSlice::new(&bytes)];
    rustix::net::sendmsg(std::io::stdin(), &iov, &mut control, SendFlags::empty()).unwrap();
}

/// What an adopting run of this binary reads: the program it starts and
/// that program's arguments, one a line.
const ADOPTING: &str = "BROKKR_TEST_OBSERVER_ADOPTING";

/// `inner` started by this test binary rerun as the attempt it stands for
/// ([`adopt`]), with `inner`'s environment and directory, its standard
/// input a pipe and its standard output closed.
fn adopted(inner: &Command) -> Command {
    let args = std::iter::once(inner.get_program()).chain(inner.get_args());
    let named: Vec<&str> = args.map(|arg| arg.to_str().unwrap()).collect();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", RELAYING, "--test-threads=1"]);
    command.env(ADOPTING, named.join("\n"));
    let set = inner
        .get_envs()
        .filter_map(|(key, value)| Some((key, value?)));
    command
        .envs(set)
        .current_dir(inner.get_current_dir().unwrap());
    command.stdin(Stdio::piped()).stdout(Stdio::null());
    command
}

/// Stand for the attempt: a child subreaper in this run's session, so what
/// an observer leaves when it alone is killed is adopted here, never
/// orphaned (the kernel hangs up an orphaned group with a stopped member,
/// which would end a held launcher whatever the broker killed). Start the
/// program `named` names and end as it does; what was adopted meanwhile
/// goes on to the next reaper.
fn adopt(named: &str) {
    let pid = rustix::process::getpid();
    rustix::process::set_child_subreaper(Some(pid)).unwrap();
    let mut lines = named.lines();
    let mut command = Command::new(lines.next().unwrap());
    let status = command.args(lines).status().unwrap();
    std::process::exit(status.code().unwrap_or(128));
}

/// The sealed fixture's broker args for the plan sealed at `digest`.
pub(super) fn serve_args<'a>(sealed: &'a Sealed, digest: &'a str) -> [&'a str; 6] {
    let locator = sealed.locator.to_str().unwrap();
    [
        "broker",
        "serve",
        "--plan",
        locator,
        "--plan-digest",
        digest,
    ]
}

/// A sealed fixture, its plan sealed with the sources observed for it, and
/// that plan's digest; none, declared skipped, where no box stands.
fn sealed_as_observed() -> Option<(Sealed, Value, String)> {
    if let Some(reason) = unservable() {
        skip(&reason);
        return None;
    }
    let sealed = Sealed::new();
    let plan = sealed.plan();
    let digest = sealed.seal_bytes(plan.to_string().as_bytes());
    Some((sealed, plan, digest))
}

#[test]
fn admission_compares_the_sealed_sources_with_a_fresh_observation() {
    let Some((sealed, plan, digest)) = sealed_as_observed() else {
        return;
    };
    // Unchanged, the sources observed afresh at admission are the sealed ones.
    assert_eq!(sealed.serve(&digest), admitted());
    // The server replaced at its path by a file of the same bytes, mode and
    // time, its directory's time put back: only its identity moved. The
    // plan is still bound, so the cause is the filesystem's (SC1).
    let server = sealed.path("opt/docs/bin/docs-mcp");
    let bin = sealed.path("opt/docs/bin");
    let (was, dir) = (
        std::fs::metadata(&server).unwrap(),
        std::fs::metadata(&bin).unwrap(),
    );
    let fresh = sealed.path("opt/docs/bin/.docs-mcp");
    std::fs::copy(&server, &fresh).unwrap();
    chmod(&fresh, 0o755);
    let file = std::fs::File::options().write(true).open(&fresh).unwrap();
    file.set_modified(was.modified().unwrap()).unwrap();
    std::fs::rename(&fresh, &server).unwrap();
    let held = std::fs::File::open(&bin).unwrap();
    held.set_modified(dir.modified().unwrap()).unwrap();
    assert_ne!(std::fs::metadata(&server).unwrap().ino(), was.ino());
    let store = Store::new(sealed.path("store/secrets.env"));
    let answer = sealed.serve(&digest);
    let ended = Instant::now();
    assert_eq!(answer, refused(Refusal::Identity));
    // Zero lookups and zero starts.
    assert_eq!(store.lookups(ended), 0);
    assert!(!sealed.path("started").exists());
    // Drift outranks every cause after the box: the store sealed into the
    // package is the box's cause only once the sources are the sealed ones.
    let mut boxed = plan.clone();
    let inside = sealed.root.store("opt/docs/secrets.env");
    boxed["box"]["excluded"]["store"] = json!(inside);
    assert_eq!(
        sealed.answer_bytes(boxed.to_string().as_bytes()),
        refused(Refusal::Identity)
    );
    let resealed = sealed.observing(boxed);
    assert_eq!(
        sealed.answer_bytes(resealed.to_string().as_bytes()),
        refused(Refusal::StoreInBox)
    );
    // That later cause travels by its name, with no handle beside it.
    let (record, handles) = sealed.observation(&sealed.seal_bytes(resealed.to_string().as_bytes()));
    let later = (&record["observed"]["refused"], handles.len());
    assert_eq!(later, (&json!("store-in-box"), 0));
    // Sealed afresh by the engine, the replaced file is admitted again,
    // its store the protected empty one again.
    std::fs::remove_file(sealed.path("store/secrets.env")).unwrap();
    sealed.root.store("store/secrets.env");
    let fresh = sealed.observing(plan);
    assert_eq!(
        sealed.answer_bytes(fresh.to_string().as_bytes()),
        admitted()
    );
}

/// The fields of the observer's record of a standing box, in order.
const OBSERVATION: [&str; 7] = [
    "digest", "entries", "entry", "handles", "mounts", "refused", "writers",
];

#[test]
fn the_observer_hands_back_the_very_handles_it_checked_in_a_closed_record() {
    let Some((sealed, plan, digest)) = sealed_as_observed() else {
        return;
    };
    let (record, handles) = sealed.observation(&digest);
    let observed = &record["observed"];
    let fields: Vec<&str> = observed
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    let closed = (record.as_object().unwrap().len(), fields);
    assert_eq!(closed, (1, OBSERVATION.to_vec()));
    let sources = &plan["box"]["sources"];
    let facts = ["entries", "mounts", "digest"].map(|fact| &observed[fact]);
    assert_eq!(
        facts,
        ["entries", "mounts", "digest"].map(|fact| &sources[fact])
    );
    let confined = (&observed["refused"], &observed["writers"]);
    assert_eq!(confined, (&Value::Null, &json!([euid()])));
    // One handle rides the record for each path it names, each the very
    // object its path named when it was checked, and the store's last,
    // a path handle that reads nothing, its path no source of the box's
    // (U6c5c). The generated identity's files outlive the observer, their
    // private tree handed over with the entry for the launch to remove
    // (U6c6b); no broker launches here, so this test removes it.
    let mut handles = handles;
    let store = handles.pop().unwrap();
    let paths: Vec<PathBuf> = serde_json::from_value(observed["handles"].clone()).unwrap();
    assert_eq!(paths.len(), handles.len());
    let path = sealed.path("store/secrets.env");
    assert!(!paths.contains(&path));
    let flags = rustix::fs::fcntl_getfl(&store).unwrap();
    assert!(flags.contains(rustix::fs::OFlags::PATH));
    let identity = |held: &std::fs::Metadata| (held.dev(), held.ino());
    let held = |fd: &OwnedFd| {
        std::fs::File::from(fd.try_clone().unwrap())
            .metadata()
            .unwrap()
    };
    let stored = identity(&std::fs::metadata(&path).unwrap());
    assert_eq!(identity(&held(&store)), stored);
    for (path, fd) in paths.iter().zip(&handles) {
        let named = std::fs::metadata(path).unwrap();
        assert_eq!((path, identity(&held(fd))), (path, identity(&named)));
    }
    let scratch = std::fs::canonicalize(std::env::temp_dir()).unwrap();
    let generated: Vec<&PathBuf> = paths
        .iter()
        .filter(|path| path.starts_with(&scratch))
        .collect();
    let names = generated
        .iter()
        .map(|path| path.file_name().unwrap().to_str().unwrap());
    assert_eq!(
        names.collect::<Vec<_>>(),
        ["passwd", "group", "hosts", "nsswitch.conf"]
    );
    let tree = generated[0].parent().unwrap().parent().unwrap();
    assert_eq!(handed_over(&record), Some(tree.to_path_buf()));
    std::fs::remove_dir_all(tree).unwrap();
    for named in [
        sealed.path("opt/docs"),
        bootstrap(),
        sealed.path("opt/docs/bin/docs-mcp"),
    ] {
        assert!(paths.contains(&named), "{named:?} in {paths:?}");
    }
    // The package moved away and another made at its path: the handle
    // still holds the package that was checked, never the name.
    let docs = sealed.path("opt/docs");
    let checked = identity(&std::fs::metadata(&docs).unwrap());
    std::fs::rename(&docs, sealed.path("opt/old")).unwrap();
    std::fs::create_dir_all(docs.join("bin")).unwrap();
    let at = paths.iter().position(|path| *path == docs).unwrap();
    assert_eq!(identity(&held(&handles[at])), checked);
    assert_ne!(identity(&std::fs::metadata(&docs).unwrap()), checked);
}

#[test]
fn an_observer_its_broker_did_not_start_hands_nothing_back() {
    let sealed = Sealed::new();
    let locator = sealed.locator.to_str().unwrap();
    let args = [
        "broker",
        "observe",
        "--plan",
        locator,
        "--plan-digest",
        super::DIGEST,
    ];
    let observe = |shell: Option<&str>| {
        let (ours, theirs) = pair();
        let mut command = match shell {
            // A process group of its own, as its broker starts it.
            None => {
                let mut command = sealed.root.command(&args);
                command.process_group(0);
                command
            }
            // Started by a shell, so the socket's maker is not its parent.
            Some(shell) => {
                let mut command = Command::new("sh");
                command.args(["-c", shell, "sh", "setpriv", "--no-new-privs"]);
                command.arg(brokkr()).args(args);
                command
            }
        };
        command
            .env("HOME", &sealed.home)
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        let status = command.stdout(theirs).status().unwrap();
        drop(command);
        (status.code(), received(&ours).0)
    };
    // Its broker's own, an unbound plan's record: its cause, by name.
    assert_eq!(observe(None), (Some(0), json!({"refused": "unbound"})));
    assert_eq!(observe(Some("\"$@\"; exit $?")), (Some(1), Value::Null));
    // A stdout that is no socket carries no record either.
    let mut piped = sealed.root.command(&args);
    let out = piped.env("HOME", &sealed.home).output().unwrap();
    assert_eq!((out.status.code(), out.stdout.len()), (Some(1), 0));
}

#[test]
fn an_observer_leading_no_process_group_of_its_own_refuses_before_its_tether() {
    let sealed = Sealed::new();
    let locator = sealed.locator.to_str().unwrap();
    let args = ["broker", "observe", "--plan", locator];
    // Started by this test, its socket's maker and its parent, but in this
    // test's own process group: the tether's watcher kills its group when
    // the socket's end goes, so the observer refuses before arming it. The
    // socket stays open until the observer has exited, so no watcher it
    // armed could fire: what this test sees is the refusal alone.
    let (ours, theirs) = pair();
    let mut command = Command::new(brokkr());
    command.args(args).args(["--plan-digest", super::DIGEST]);
    command.env("HOME", &sealed.home).stdin(Stdio::null());
    let out = command
        .stdout(theirs)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    drop(command);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let answer = ((out.status.code(), stderr), received(&ours).0);
    assert_eq!(answer, (refused(Refusal::Identity), Value::Null));
    // Its end gone, this test, and every process of its group, live on.
    drop(ours);
}

/// A FUSE mount at `point` whose connection is never initialised: any
/// lookup, attribute read or listing that reaches it waits in the kernel
/// until the connection is aborted, which dropping this value does before
/// it unmounts.
struct Stalled {
    point: PathBuf,
    device: Option<OwnedFd>,
}

impl Stalled {
    /// The stalled mount at the existing directory `point`, or none where
    /// this host mounts no FUSE file system for this user.
    fn at(point: &Path) -> Option<Stalled> {
        let (ours, theirs) = rustix::net::socketpair(
            AddressFamily::UNIX,
            SocketType::STREAM,
            SocketFlags::CLOEXEC,
            None,
        )
        .unwrap();
        // fusermount3 hands the device back over its stdin.
        let status = Command::new("fusermount3")
            .arg("--")
            .arg(point)
            .env("_FUSE_COMMFD", "0")
            .stdin(theirs)
            .stderr(Stdio::null())
            .status();
        if !status.is_ok_and(|status| status.success()) {
            return None;
        }
        let device = received_stream(&ours)?;
        Some(Stalled {
            point: point.to_path_buf(),
            device: Some(device),
        })
    }
}

impl Drop for Stalled {
    fn drop(&mut self) {
        drop(self.device.take());
        let unmount = Command::new("fusermount3")
            .arg("-uz")
            .arg(&self.point)
            .status();
        assert!(unmount.unwrap().success());
    }
}

/// The one descriptor waiting on a stream socket.
fn received_stream(socket: &OwnedFd) -> Option<OwnedFd> {
    let mut byte = [0; 1];
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut control = RecvAncillaryBuffer::new(&mut space);
    let mut iov = [IoSliceMut::new(&mut byte)];
    rustix::net::recvmsg(socket, &mut iov, &mut control, RecvFlags::CMSG_CLOEXEC).ok()?;
    let mut fds = control.drain().filter_map(|carried| match carried {
        RecvAncillaryMessage::ScmRights(mut fds) => fds.next(),
        _ => None,
    });
    fds.next()
}

/// Why a run skips the stalled-mount proofs: no FUSE mount stands, as under
/// `no_new_privs`, where the setuid fusermount3 cannot mount.
const NO_FUSE: &str = "this host mounts no FUSE file system for this user, so no source attribute read can be held blocked in the kernel";

/// What holds the observer blocked: a directory of the package under the
/// stalled mount, whose attribute read by the walk waits in the kernel
/// ('D'), or the version query of the launcher it admitted, its own child,
/// held by a tracer at the exec of the very handle admitted while the
/// observer waits on it (U6c5c). Only a launcher no managed writer could
/// write is admitted, so no script this run plants can stand for it: the
/// tracer holds the host's own.
enum Block {
    Mount { _held: Stalled },
    Launcher { tracer: PathBuf },
}

/// Which blocking operation a proof holds the observer in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Mount,
    Launcher,
}

/// The operations every blocking proof is run under.
const KINDS: [Kind; 2] = [Kind::Mount, Kind::Launcher];

/// Why a run skips the held-launcher proofs: a boundary proof's skip, which
/// fails where boundary evidence is required, as CI's bubblewrap setup
/// installs the tracer.
pub(super) const NO_TRACER: &str =
    "no strace on PATH traces a child here, so no admitted launcher can be held at its exec";

/// The tracer's arguments: every process of the run followed, stopped by
/// a seccomp filter at `execve` alone so the observation runs at its own
/// pace, nothing written, and each `execve` of `/proc/self/fd/0` (the
/// launcher's run through its kept handle, and no other exec of the run)
/// stopped at its entry by `SIGSTOP`, which only a kill ends: a stop the
/// tracer itself held would keep a killed child at its exit until it let
/// go, which no broker could hasten.
const TRACED: [&str; 11] = [
    "-f",
    "--seccomp-bpf",
    "-qqq",
    "-o",
    "/dev/null",
    "-e",
    "trace=execve",
    "-P",
    "/proc/self/fd/0",
    "-e",
    "inject=execve:signal=SIGSTOP",
];

/// The `strace` this process's `PATH` finds, where it traces a child here.
pub(super) fn tracer() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let mut found = std::env::split_paths(&path).map(|dir| dir.join("strace"));
    let tracer = found.find(|tracer| tracer.is_file())?;
    let mut probe = Command::new(&tracer);
    probe.args([
        "-f",
        "--seccomp-bpf",
        "-qqq",
        "-o",
        "/dev/null",
        "/bin/true",
    ]);
    let traced = probe.stdin(Stdio::null()).stderr(Stdio::null()).status();
    traced
        .is_ok_and(|status| status.success())
        .then_some(tracer)
}

/// Who starts the broker: a harness that waits on it, or the attempt,
/// which leaves it in the background and waits.
#[derive(Debug, Clone, Copy)]
pub(super) enum Starter {
    Harness,
    Attempt,
}

/// The broker serving the plan `sealed` holds at `digest`, started by
/// `starter`, a shell under `no_new_privs` as a confined harness is, with
/// the engine's HOME; the whole run traced where `block` holds the
/// launcher.
fn serving(sealed: &Sealed, digest: &str, block: &Block, starter: Starter) -> Command {
    let command = match block {
        Block::Mount { .. } => Command::new(setpriv()),
        Block::Launcher { tracer } => traced(tracer, TRACED),
    };
    started(command, sealed, digest, starter)
}

/// `tracer`, given `args`, running `setpriv`.
pub(super) fn traced(tracer: &Path, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Command {
    let mut command = Command::new(tracer);
    command.args(args).arg(setpriv());
    command
}

/// `command`, `setpriv` or a tracer running it, made the broker serving
/// the plan `sealed` holds at `digest`, started by `starter`.
pub(super) fn started(
    mut command: Command,
    sealed: &Sealed,
    digest: &str,
    starter: Starter,
) -> Command {
    let shell = match starter {
        Starter::Harness => ["/bin/sh", "-c", HARNESS],
        Starter::Attempt => ["/bin/sh", "-c", "\"$0\" \"$@\" & wait"],
    };
    command.arg("--no-new-privs").args(shell).arg(brokkr());
    // A pipe, which names no path the tracer could resolve and report.
    command
        .args(serve_args(sealed, digest))
        .stdin(Stdio::piped());
    command
        .env("HOME", &sealed.home)
        .current_dir(&sealed.root.path);
    command
}

/// Kill `pid` outright.
fn kill(pid: i32) {
    let signal = rustix::process::Signal::KILL;
    rustix::process::kill_process(Pid::from_raw(pid).unwrap(), signal).unwrap();
}

/// A process's state, parent and process group, from `/proc/<pid>/stat`;
/// none once it is gone.
pub(super) fn status(pid: i32) -> Option<(char, i32, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let mut fields = stat.rsplit_once(')')?.1.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let parent = fields.next()?.parse().ok()?;
    let group = fields.next()?.parse().ok()?;
    Some((state, parent, group))
}

/// Every process of the host.
pub(super) fn processes() -> impl Iterator<Item = i32> {
    let entries = std::fs::read_dir("/proc").unwrap();
    entries.filter_map(|entry| entry.ok()?.file_name().to_str()?.parse().ok())
}

/// Every process that is not a zombie and whose group is `group`.
fn members(group: i32) -> Vec<i32> {
    let member =
        |pid: &i32| status(*pid).is_some_and(|(state, _, of)| (of == group) & (state != 'Z'));
    processes().filter(member).collect()
}

/// The `broker observe` of `locator` running now, if one is.
fn observer(locator: &Path) -> Option<i32> {
    let wanted = locator.to_str().unwrap().as_bytes();
    processes().find(|pid| {
        let line = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let args: Vec<&[u8]> = line.split(|byte| *byte == 0).collect();
        args.contains(&&b"observe"[..]) & args.contains(&wanted)
    })
}

/// The observer of `locator` once `block` holds it: waiting in the kernel
/// ('D') on the stalled mount, or with a live child, the launcher it waits
/// on.
fn blocked(locator: &Path, block: &Block) -> i32 {
    let until = Instant::now() + Duration::from_secs(20);
    while Instant::now() < until {
        let waiting = |pid: &i32| match block {
            Block::Mount { .. } => status(*pid).is_some_and(|(state, _, _)| state == 'D'),
            Block::Launcher { .. } => processes().any(|child| {
                status(child).is_some_and(|(state, parent, _)| (parent == *pid) & (state != 'Z'))
            }),
        };
        if let Some(pid) = observer(locator).filter(waiting) {
            return pid;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("no observer of {} blocked", locator.display());
}

/// Whether `pid` and every member of its group are gone within a while:
/// a killed process closes its descriptors before it turns zombie, so one
/// look can catch it mid-exit.
fn ended(pid: i32) -> bool {
    gone(pid) & within(|| members(pid).is_empty())
}

/// A sealed fixture, its plan sealed with the sources observed for it, and
/// `kind`'s block laid on it after: the stalled mount over an empty
/// directory of the package, or the tracer that holds the launcher. None,
/// declared skipped, where no FUSE mount stands or no tracer traces.
fn stalled(kind: Kind) -> Option<(Sealed, String, Block)> {
    let sealed = Sealed::new();
    let share = sealed.path("opt/docs/share");
    std::fs::create_dir_all(&share).unwrap();
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let block = match kind {
        Kind::Mount => Stalled::at(&share).map(|held| Block::Mount { _held: held }),
        Kind::Launcher => tracer().map(|tracer| Block::Launcher { tracer }),
    };
    match (&block, kind) {
        (None, Kind::Mount) => skip(NO_FUSE),
        (None, Kind::Launcher) => skip(NO_TRACER),
        (Some(_), _) => {}
    }
    Some((sealed, digest, block?))
}

#[test]
fn a_launcher_a_managed_writer_owns_refuses_before_it_ever_runs() {
    let Some((sealed, _, digest)) = sealed_as_observed() else {
        return;
    };
    // First on the broker's `PATH`, a launcher that would note its run and
    // never answer; this user, a managed writer, owns it.
    let ran = sealed.path("ran");
    let script = format!("#!/bin/sh\n: > '{}'\nexec sleep 600\n", ran.display());
    chmod(&sealed.root.write("stall/bwrap", &script), 0o755);
    let path = std::env::var("PATH").unwrap();
    let path = format!("{}:{path}", sealed.path("stall").display());
    let started = Instant::now();
    let answer = sealed.serve_in(&sealed.locator, &digest, |command| {
        command.env("HOME", &sealed.home).env("PATH", &path);
    });
    assert_eq!(answer, refused(Refusal::Identity));
    assert!(started.elapsed() < Duration::from_secs(20));
    assert!(!ran.exists());
}

/// The tracer's arguments when it records rather than holds: every process
/// followed, stopped by a seccomp filter at `execve` alone, no signal
/// written, and each `execve` of `/proc/self/fd/0` written whole, its
/// environment included, to the file named next.
const RECORDED: [&str; 11] = [
    "-f",
    "--seccomp-bpf",
    "-qqq",
    "-v",
    "-e",
    "trace=execve",
    "-e",
    "signal=none",
    "-P",
    "/proc/self/fd/0",
    "-o",
];

#[test]
fn the_admitted_launcher_runs_through_its_handle_with_no_environment() {
    let Some((sealed, _, digest)) = sealed_as_observed() else {
        return;
    };
    let Some(tracer) = tracer() else {
        return skip(NO_TRACER);
    };
    let log = sealed.path("launched.log");
    let args = RECORDED.iter().map(OsStr::new).chain([log.as_os_str()]);
    let mut command = started(traced(&tracer, args), &sealed, &digest, Starter::Harness);
    let out = command.stderr(Stdio::piped()).output().unwrap();
    let answer = (out.status.code(), String::from_utf8(out.stderr).unwrap());
    assert_eq!(answer, admitted());
    // The one run the admission gave its launcher: the kept handle, by the
    // kernel's name for it, asked its version with an empty environment.
    let log = std::fs::read_to_string(&log).unwrap();
    let calls = log.lines().filter_map(|line| line.split_once(' '));
    // strace pads a short pid to its column, so the call starts after it.
    let calls: Vec<&str> = calls.map(|(_, call)| call.trim_start()).collect();
    let run = r#"execve("/proc/self/fd/0", ["/proc/self/fd/0", "--version"], []) = 0"#;
    assert_eq!(calls, [run]);
}

/// `proof` run under each block this run can lay, where a box stands, and
/// declared skipped where none does.
fn each_block(proof: impl Fn(Kind, Sealed, String, Block)) {
    if let Some(reason) = unservable() {
        return skip(&reason);
    }
    for kind in KINDS {
        if let Some((sealed, digest, block)) = stalled(kind) {
            proof(kind, sealed, digest, block);
        }
    }
}

#[test]
fn an_operation_blocked_to_the_deadline_refuses_with_nothing_left_running() {
    each_block(|kind, sealed, digest, block| {
        let store = Store::new(sealed.path("store/secrets.env"));
        let started = Instant::now();
        let mut command = adopted(&serving(&sealed, &digest, &block, Starter::Harness));
        let attempt = command.stderr(Stdio::piped()).spawn().unwrap();
        let pid = blocked(&sealed.locator, &block);
        let (_, broker, _) = status(pid).unwrap();
        // The broker ends once the full deadline, never extended, has
        // passed, and no more than a slice after; the observer, whatever it
        // started and its whole group are gone by then.
        let elapsed = awaited(broker) - started;
        let ended = swept(pid);
        let out = attempt.wait_with_output().unwrap();
        let ended_at = Instant::now();
        let answer = (out.status.code(), String::from_utf8(out.stderr).unwrap());
        assert_eq!((kind, answer), (kind, refused(Refusal::Identity)));
        let within = (elapsed >= Duration::from_secs(30)) & (elapsed < Duration::from_secs(32));
        assert!(within, "{kind:?}: {elapsed:?}");
        assert!(ended, "{kind:?}");
        // Nothing was looked up or started.
        assert_eq!((kind, store.lookups(ended_at)), (kind, 0));
        assert!(!sealed.path("started").exists());
    });
}

#[test]
fn a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running() {
    each_block(|kind, sealed, digest, block| {
        let store = Store::new(sealed.path("store/secrets.env"));
        // The shell that started the broker, standing for the attempt's own
        // start, ends while the observer is blocked.
        let mut command = adopted(&serving(&sealed, &digest, &block, Starter::Attempt));
        let mut attempt = command.stderr(Stdio::piped()).spawn().unwrap();
        let pid = blocked(&sealed.locator, &block);
        let (_, broker, _) = status(pid).unwrap();
        let taken = cancelled(broker);
        let ended = swept(pid);
        attempt.wait().unwrap();
        let mut stderr = String::new();
        let mut pipe = attempt.stderr.take().unwrap();
        pipe.read_to_string(&mut stderr).unwrap();
        let ended_at = Instant::now();
        // The broker answered with the identity cause soon after and
        // ended, its observer and the observer's group gone.
        assert_eq!((kind, stderr), (kind, refused(Refusal::Identity).1));
        assert!(taken < Duration::from_secs(5), "{kind:?}: {taken:?}");
        assert!(ended, "{kind:?}");
        assert!(gone(broker), "{kind:?}");
        assert_eq!((kind, store.lookups(ended_at)), (kind, 0));
        assert!(!sealed.path("started").exists());
    });
}

#[test]
fn a_broker_killed_outright_takes_its_blocked_observer_with_it() {
    each_block(|kind, sealed, digest, block| {
        let mut command = adopted(&serving(&sealed, &digest, &block, Starter::Harness));
        let mut attempt = command.stderr(Stdio::null()).spawn().unwrap();
        let pid = blocked(&sealed.locator, &block);
        let (_, broker, _) = status(pid).unwrap();
        kill(broker);
        // No broker is left to kill the group: the observer's watcher
        // kills it, the observer and whatever it started, the launcher's
        // version query included.
        let ended = swept(pid);
        attempt.wait().unwrap();
        assert!(ended, "{kind:?}");
        assert!(!sealed.path("started").exists());
    });
}

/// Cancel the attempt that started `broker`, killing the shell that stands
/// for its start, and how long the broker then took to end.
pub(super) fn cancelled(broker: i32) -> Duration {
    let (_, shell, _) = status(broker).unwrap();
    kill(shell);
    let cancelled = Instant::now();
    awaited(broker) - cancelled
}

/// When `pid` is gone, or only a zombie, waiting no longer than a minute.
pub(super) fn awaited(pid: i32) -> Instant {
    let until = Instant::now() + Duration::from_secs(60);
    let live = || status(pid).is_some_and(|(state, _, _)| state != 'Z');
    while live() & (Instant::now() < until) {
        std::thread::sleep(Duration::from_millis(20));
    }
    Instant::now()
}

/// Whether `pid` and every member of its group ended within a while
/// ([`ended`]), each survivor then killed, so a failure leaves nothing
/// running for the attempt to wait on.
fn swept(pid: i32) -> bool {
    let ended = ended(pid);
    for member in members(pid).into_iter().filter_map(Pid::from_raw) {
        rustix::process::kill_process(member, rustix::process::Signal::KILL).ok();
    }
    ended
}

/// Whether `pid` is gone, or only a zombie, within a while.
pub(super) fn gone(pid: i32) -> bool {
    within(|| status(pid).is_none_or(|(state, _, _)| state == 'Z'))
}

/// Whether `holds` comes true within five seconds.
pub(super) fn within(holds: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if holds() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}
