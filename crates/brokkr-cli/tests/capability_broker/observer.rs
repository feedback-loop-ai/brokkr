//! `serve`'s private observer (decision 0065 slice two U6c5b; MB3, MB4,
//! SC1): the sources admission observes afresh compared with what the plan
//! sealed, the handles the observer hands back being the very objects it
//! checked, and its supervision under the startup deadline, which a
//! blocked source operation or a cancelled attempt ends with nothing of
//! admission left running.

use std::io::{IoSliceMut, Read};
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rustix::net::{AddressFamily, SocketFlags, SocketType};
use rustix::net::{RecvAncillaryBuffer, RecvAncillaryMessage, RecvFlags};
use rustix::process::Pid;
use serde_json::{json, Value};

use super::{admitted, bootstrap, brokkr, chmod, euid, refused, skip, unservable};
use super::{Refusal, Sealed, Store};

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

/// The sealed fixture's broker args for the plan sealed at `digest`.
fn serve_args<'a>(sealed: &'a Sealed, digest: &'a str) -> [&'a str; 6] {
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
    boxed["box"]["excluded"]["store"] = json!(sealed.path("opt/docs/secrets.env"));
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
    // Sealed afresh by the engine, the replaced file is admitted again.
    let fresh = sealed.observing(plan);
    assert_eq!(
        sealed.answer_bytes(fresh.to_string().as_bytes()),
        admitted()
    );
}

/// The fields of the observer's record of a standing box, in order.
const OBSERVATION: [&str; 6] = [
    "digest", "entries", "handles", "mounts", "refused", "writers",
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
    // object its path named when it was checked. The generated identity's
    // files went with the observer's private scratch; their handles still
    // hold them, under no name.
    let paths: Vec<PathBuf> = serde_json::from_value(observed["handles"].clone()).unwrap();
    assert_eq!(paths.len(), handles.len());
    let identity = |held: &std::fs::Metadata| (held.dev(), held.ino());
    let held = |fd: &OwnedFd| {
        std::fs::File::from(fd.try_clone().unwrap())
            .metadata()
            .unwrap()
    };
    let mut unnamed = Vec::new();
    for (path, fd) in paths.iter().zip(&handles) {
        let Ok(named) = std::fs::metadata(path) else {
            assert_eq!(held(fd).nlink(), 0);
            unnamed.push(path.file_name().unwrap().to_str().unwrap());
            continue;
        };
        assert_eq!((path, identity(&held(fd))), (path, identity(&named)));
    }
    assert_eq!(unnamed, ["passwd", "group", "hosts", "nsswitch.conf"]);
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
            None => sealed.root.command(&args),
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

/// What holds the observer blocked in the kernel.
enum Block {
    /// A directory of the package lies under the stalled mount: the walk's
    /// attribute read of it waits ('D').
    Mount { _held: Stalled },
    /// The launcher the observer asks for its version never answers: the
    /// observer waits on its child, a member of its group. The `PATH` that
    /// finds that launcher first.
    Launcher(String),
}

/// Which blocking operation a proof holds the observer in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Mount,
    Launcher,
}

/// The two operations every blocking proof is run under.
const KINDS: [Kind; 2] = [Kind::Mount, Kind::Launcher];

/// A process's state, parent and process group, from `/proc/<pid>/stat`;
/// none once it is gone.
fn status(pid: i32) -> Option<(char, i32, i32)> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let mut fields = stat.rsplit_once(')')?.1.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let parent = fields.next()?.parse().ok()?;
    let group = fields.next()?.parse().ok()?;
    Some((state, parent, group))
}

/// Every process of the host.
fn processes() -> impl Iterator<Item = i32> {
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
            Block::Launcher(_) => processes().any(|child| {
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

/// Whether `pid` and every member of its group are gone within a while.
fn ended(pid: i32) -> bool {
    gone(pid) & members(pid).is_empty()
}

/// A sealed fixture, its plan sealed with the sources observed for it, and
/// `kind`'s block laid on it after: the stalled mount over an empty
/// directory of the package, or a launcher that never answers first on
/// the broker's `PATH`. None, declared skipped, where no FUSE mount stands.
fn stalled(kind: Kind) -> Option<(Sealed, String, Block)> {
    let sealed = Sealed::new();
    let share = sealed.path("opt/docs/share");
    std::fs::create_dir_all(&share).unwrap();
    let digest = sealed.seal_bytes(sealed.plan().to_string().as_bytes());
    let block = match kind {
        Kind::Mount => Stalled::at(&share).map(|held| Block::Mount { _held: held }),
        Kind::Launcher => {
            let launcher = sealed
                .root
                .write("stall/bwrap", "#!/bin/sh\nexec sleep 600\n");
            chmod(&launcher, 0o755);
            let path = std::env::var("PATH").unwrap();
            Some(Block::Launcher(format!(
                "{}:{path}",
                sealed.path("stall").display()
            )))
        }
    };
    if block.is_none() {
        skip(NO_FUSE);
    }
    Some((sealed, digest, block?))
}

/// `command` given what `block` needs of the broker's environment, and the
/// engine's HOME.
fn blocking<'c>(command: &'c mut Command, sealed: &Sealed, block: &Block) -> &'c mut Command {
    if let Block::Launcher(path) = block {
        command.env("PATH", path);
    }
    command.env("HOME", &sealed.home)
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
        let mut command = sealed.root.command(&serve_args(&sealed, &digest));
        let command = blocking(&mut command, &sealed, &block).stderr(Stdio::piped());
        let broker = command.spawn().unwrap();
        let pid = blocked(&sealed.locator, &block);
        let out = broker.wait_with_output().unwrap();
        let (elapsed, ended_at) = (started.elapsed(), Instant::now());
        let answer = (out.status.code(), String::from_utf8(out.stderr).unwrap());
        assert_eq!((kind, answer), (kind, refused(Refusal::Identity)));
        // The full deadline, never extended, and no more than a slice past.
        let within = (elapsed >= Duration::from_secs(30)) & (elapsed < Duration::from_secs(32));
        assert!(within, "{kind:?}: {elapsed:?}");
        // The observer, whatever it started and its whole group are gone;
        // nothing was looked up or started.
        assert!(ended(pid), "{kind:?}");
        assert_eq!((kind, store.lookups(ended_at)), (kind, 0));
        assert!(!sealed.path("started").exists());
    });
}

#[test]
fn a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running() {
    each_block(|kind, sealed, digest, block| {
        let store = Store::new(sealed.path("store/secrets.env"));
        // The broker's starter, standing for the attempt, ends while the
        // observer is blocked.
        let mut starter = Command::new("sh");
        starter.args(["-c", "\"$@\" & wait", "sh", "setpriv", "--no-new-privs"]);
        starter.arg(brokkr()).args(serve_args(&sealed, &digest));
        let starter = blocking(&mut starter, &sealed, &block).stderr(Stdio::piped());
        let mut starter = starter.spawn().unwrap();
        let pid = blocked(&sealed.locator, &block);
        let (_, broker, _) = status(pid).unwrap();
        starter.kill().unwrap();
        let cancelled = Instant::now();
        starter.wait().unwrap();
        let mut stderr = String::new();
        let mut pipe = starter.stderr.take().unwrap();
        pipe.read_to_string(&mut stderr).unwrap();
        let (taken, ended_at) = (cancelled.elapsed(), Instant::now());
        // The broker answered with the identity cause soon after, its
        // observer and the observer's group gone, and it ended too.
        assert_eq!((kind, stderr), (kind, refused(Refusal::Identity).1));
        assert!(taken < Duration::from_secs(5), "{kind:?}: {taken:?}");
        assert!(ended(pid), "{kind:?}");
        assert!(status(broker).is_none_or(|(state, _, _)| state == 'Z'));
        assert_eq!((kind, store.lookups(ended_at)), (kind, 0));
        assert!(!sealed.path("started").exists());
    });
}

#[test]
fn a_broker_killed_outright_takes_its_blocked_observer_with_it() {
    each_block(|kind, sealed, digest, block| {
        let mut command = sealed.root.command(&serve_args(&sealed, &digest));
        let command = blocking(&mut command, &sealed, &block).stderr(Stdio::null());
        let mut broker = command.spawn().unwrap();
        let pid = blocked(&sealed.locator, &block);
        broker.kill().unwrap();
        broker.wait().unwrap();
        // No broker is left to kill the group: the observer's watcher
        // kills it, the observer and whatever it started, the launcher's
        // version query included. Any survivor is killed before the
        // verdict, so a failure leaves nothing running.
        let ended = ended(pid);
        for member in members(pid).into_iter().filter_map(Pid::from_raw) {
            rustix::process::kill_process(member, rustix::process::Signal::KILL).ok();
        }
        assert!(ended, "{kind:?}");
        assert!(!sealed.path("started").exists());
    });
}

/// Whether `pid` is gone, or only a zombie, within a while.
fn gone(pid: i32) -> bool {
    let until = Instant::now() + Duration::from_secs(5);
    while Instant::now() < until {
        if status(pid).is_none_or(|(state, _, _)| state == 'Z') {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}
