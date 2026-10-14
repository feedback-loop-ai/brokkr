//! The readiness receiver's reading and deciding (decision 0065 slice two,
//! U6c6b; MB3, MB5): real anonymous pipes stand for the box's, the host's
//! view of the box is stood in, and launchers that stand no box are real
//! processes. The live box is the CLI suite's.

use std::io::{PipeReader, PipeWriter, Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{recover, Disposition, Failure, Invalid, Outcome};
use brokkr_protocol::broker::{BoxIntent, Network, Plan, Refusal};
use brokkr_protocol::secret::BoundSecret;
use serde_json::json;

use super::super::helper::{handed, received};
use super::super::lifetime::{lived, Binder, Ended, Held};
use super::linux::{
    decided, delivered, exec_status, first, framed, handed as supervised, message, read, sent,
    Intent, Published, Ready, Seen, Source, Spaces, FRAME_MAX,
};
use super::{compared, ready, Admitted, Handed, Link, Record, Startup};
use crate::broker::rpc::pieces::Put;
use crate::broker::rpc::Fatal;
use crate::broker::session::ensure;

/// The sealed plan's and source set's digests.
const PLAN: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SOURCES: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// The namespaces a box reports, and the host shows it in.
fn spaces() -> Spaces {
    Spaces {
        mnt: 11,
        pid: 12,
        net: 13,
        ipc: 14,
        uts: 15,
    }
}

/// A ready message for the sealed digests, its bootstrap pid 2.
fn ready_bytes() -> Vec<u8> {
    let namespaces = json!({"mnt": 11, "pid": 12, "net": 13, "ipc": 14, "uts": 15});
    let ready = json!({"plan": PLAN, "sources": SOURCES, "pid": 2, "namespaces": namespaces});
    ready.to_string().into_bytes()
}

/// Never cancelled.
fn live() -> bool {
    false
}

/// A startup five seconds from its deadline, cancelled as `cancelled` says.
fn startup(cancelled: &(dyn Fn() -> bool + Sync)) -> Startup<'_> {
    (Instant::now() + Duration::from_secs(5), cancelled)
}

/// A pipe `bytes` were written to, its writer then closed.
fn written(bytes: &[u8]) -> PipeReader {
    let (reader, mut writer) = std::io::pipe().unwrap();
    writer.write_all(bytes).unwrap();
    reader
}

#[test]
fn readiness_is_every_byte_until_the_pipe_closes_and_at_most_four_kibibytes() {
    // Exactly the bound, and one byte more.
    let most = vec![b' '; FRAME_MAX];
    let read_most = read(&mut written(&most), FRAME_MAX, startup(&live));
    assert_eq!(read_most.map(|bytes| bytes.len()), Ok(FRAME_MAX));
    let over = vec![b' '; FRAME_MAX + 1];
    let read_over = read(&mut written(&over), FRAME_MAX, startup(&live));
    assert_eq!(read_over, Err(Refusal::Establishment));
    // A message written in two parts is read whole: the reader waits for
    // the writer's close, not for the first part.
    let (mut reader, mut writer) = std::io::pipe().unwrap();
    let bytes = ready_bytes();
    let (head, tail) = bytes.split_at(bytes.len() / 2);
    writer.write_all(head).unwrap();
    let tail = tail.to_vec();
    let late = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        writer.write_all(&tail).unwrap();
    });
    let whole = read(&mut reader, FRAME_MAX, startup(&live));
    late.join().unwrap();
    assert_eq!(whole, Ok(bytes));
}

#[test]
fn a_read_past_the_absolute_deadline_refuses_whatever_the_box_wrote() {
    // The deadline the startup was given, already passed, is never
    // extended: a whole message waiting on the pipe is not read.
    let passed: Startup<'_> = (Instant::now() - Duration::from_millis(1), &live);
    let answer = read(&mut written(&ready_bytes()), FRAME_MAX, passed);
    assert_eq!(answer, Err(Refusal::Establishment));
    // A box that never closes its pipe is refused at the deadline.
    let (mut reader, _writer) = std::io::pipe().unwrap();
    let started = Instant::now();
    let soon: Startup<'_> = (started + Duration::from_millis(200), &live);
    assert_eq!(
        read(&mut reader, FRAME_MAX, soon),
        Err(Refusal::Establishment)
    );
    let waited = started.elapsed();
    let bounded = (waited >= Duration::from_millis(200)) & (waited < Duration::from_secs(2));
    assert!(bounded, "{waited:?}");
}

#[test]
fn a_cancelled_attempt_refuses_whatever_the_box_wrote() {
    let cancelled = || true;
    let answer = read(&mut written(&ready_bytes()), FRAME_MAX, startup(&cancelled));
    assert_eq!(answer, Err(Refusal::Establishment));
}

#[test]
fn the_ready_message_is_one_closed_message() {
    let bytes = ready_bytes();
    let ready = message(&bytes).unwrap();
    let fields = (ready.plan.as_str(), ready.sources.as_str(), ready.pid);
    assert_eq!((fields, ready.namespaces), ((PLAN, SOURCES, 2), spaces()));
    // Padded to the bound with whitespace, it is still the one message.
    let mut padded = bytes.clone();
    padded.resize(FRAME_MAX, b' ');
    assert_eq!(message(&padded).map(|ready| ready.pid), Ok(2));
    let duplicate = [&bytes[..], &bytes[..]].concat();
    let excess = [&bytes[..], b"x"].concat();
    let truncated = &bytes[..bytes.len() - 1];
    let mut unknown: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    unknown["ready"] = json!(true);
    let unknown = unknown.to_string().into_bytes();
    for (case, bytes) in [
        ("duplicate", duplicate.as_slice()),
        ("excess", &excess),
        ("truncated", truncated),
        ("unknown", &unknown),
        ("empty", b""),
    ] {
        let answer = message(bytes).map(|ready| ready.pid);
        assert_eq!((case, answer), (case, Err(Refusal::Establishment)));
    }
}

/// No box established.
const REFUSED: Result<(), Refusal> = Err(Refusal::Establishment);

/// Moves one namespace of a box elsewhere.
type Shift = fn(&mut Spaces);

/// What the host shows of an owned box: its first process the launcher
/// `100`'s child, and the bootstrap it names in the namespaces it reports.
fn owned() -> Seen {
    Seen {
        parent: Some(100),
        bootstrap: Some(spaces()),
    }
}

/// What deciding the sealed ready message answers against `seen`.
fn decide(ready: &Ready, seen: &Seen) -> Result<(), Refusal> {
    decided(ready, (PLAN, SOURCES), 100, seen)
}

#[test]
fn only_the_owned_bootstrap_for_these_digests_in_its_own_namespaces_is_ready() {
    let ready = message(&ready_bytes()).unwrap();
    assert_eq!(decide(&ready, &owned()), Ok(()));
    // A wrong plan or source-set digest.
    let other = "3".repeat(64);
    for (plan, sources) in [(other.as_str(), SOURCES), (PLAN, other.as_str())] {
        let answer = decided(&ready, (plan, sources), 100, &owned());
        assert_eq!(((plan, sources), answer), ((plan, sources), REFUSED));
    }
    // Another child: the first process not the launcher's own, or none of
    // its children the bootstrap the message names.
    let stranger = Seen {
        parent: Some(101),
        ..owned()
    };
    assert_eq!(decide(&ready, &stranger), REFUSED);
    let unnamed = Seen {
        bootstrap: None,
        ..owned()
    };
    assert_eq!(decide(&ready, &unnamed), REFUSED);
    // Each namespace the host shows apart from the one reported.
    let shifts: [(&str, Shift); 5] = [
        ("mnt", |spaces| spaces.mnt += 1),
        ("pid", |spaces| spaces.pid += 1),
        ("net", |spaces| spaces.net += 1),
        ("ipc", |spaces| spaces.ipc += 1),
        ("uts", |spaces| spaces.uts += 1),
    ];
    for (name, shift) in shifts {
        let mut shown = spaces();
        shift(&mut shown);
        let elsewhere = Seen {
            bootstrap: Some(shown),
            ..owned()
        };
        assert_eq!((name, decide(&ready, &elsewhere)), (name, REFUSED));
    }
}

#[test]
fn bubblewrap_info_must_name_the_first_process_of_the_box() {
    let info = b"{\n    \"child-pid\": 4242,\n    \"mnt-namespace\": 4026532924\n}\n";
    assert_eq!(first(info), Ok(4242));
    // A launcher that started names none where it could not set the box
    // up: the box is not established, never unavailable.
    for info in [&b""[..], b"{}", b"{\"child-pid\": -1}"] {
        let case = String::from_utf8_lossy(info);
        assert_eq!((&case, first(info)), (&case, Err(Refusal::Establishment)));
    }
}

/// An identity tree handed to this process, as an observer hands its
/// broker one: directly in the temporary directory, naming this process.
fn handed_tree() -> PathBuf {
    let owned = format!("u6c6b-{}-", std::process::id());
    let tree = tempfile::Builder::new().prefix(&owned).tempdir().unwrap();
    tree.keep()
}

/// An entry naming no mount, its launcher the first handle, and `tree`
/// its identity's.
fn entry(tree: &Path) -> serde_json::Value {
    json!({"argv": [], "launcher": 0, "bootstrap": "/", "identity": tree})
}

/// What admission admitted, its launcher's handle `launcher`, its entry
/// naming no mount and an identity tree handed to this process.
fn admitted(launcher: OwnedFd) -> Admitted {
    let store = std::fs::File::open("/dev/null").unwrap();
    Admitted {
        handles: vec![(PathBuf::from("/launcher"), launcher)],
        store: store.into(),
        entry: serde_json::from_value(entry(&handed_tree())).unwrap(),
    }
}

/// The observer's record of `sources`' digest, its one handle the
/// launcher's, its entry `entry` as it travels.
fn record(sources: &str, entry: serde_json::Value) -> serde_json::Value {
    json!({"observed": {
        "entries": 1, "mounts": 1, "digest": sources, "writers": [1000], "refused": null,
        "handles": ["/launcher"], "entry": entry,
    }})
}

#[test]
fn the_entry_crosses_closed_and_every_refused_comparison_removes_its_identity_tree() {
    let intent = intent_sealed();
    let handles = || {
        let held = ["/bin/true", "/dev/null"].map(|path| std::fs::File::open(path).unwrap());
        Vec::from(held.map(OwnedFd::from))
    };
    // The entry is decoded with the record, closed: never as its text.
    let tree = handed_tree();
    let text = record(SOURCES, json!(entry(&tree).to_string()));
    let decoded = serde_json::from_value::<Record>(text).map(drop);
    assert_eq!(
        decoded.map_err(|error| error.classify()),
        Err(serde_json::error::Category::Data)
    );
    std::fs::remove_dir(&tree).unwrap();
    // Each comparison that refuses removes the tree the record handed over:
    // drifted sources, and handles that are not the ones it names.
    let drift = "3".repeat(64);
    let cases = [("drift", drift.as_str(), 2), ("handles", SOURCES, 1)];
    for (case, sources, count) in cases {
        let tree = handed_tree();
        let observed = serde_json::from_value(record(sources, entry(&tree))).unwrap();
        let mut held = handles();
        held.truncate(count);
        let answer = compared(&intent, observed, held).map(drop);
        let left = tree.exists();
        assert_eq!((case, answer, left), (case, Err(Refusal::Identity), false));
    }
    // One admitted keeps it until it is dropped, the box then settled.
    let tree = handed_tree();
    let observed = serde_json::from_value(record(SOURCES, entry(&tree))).unwrap();
    let admitted = compared(&intent, observed, handles()).unwrap();
    assert!(tree.is_dir());
    drop(admitted);
    assert!(!tree.exists());
}

#[test]
fn a_record_its_end_does_not_follow_removes_the_identity_tree_it_carried() {
    use rustix::net::{send, socketpair, AddressFamily, SendFlags, SocketFlags, SocketType};
    let pair = || {
        let (unix, packets) = (AddressFamily::UNIX, SocketType::SEQPACKET);
        socketpair(unix, packets, SocketFlags::CLOEXEC, None).unwrap()
    };
    // The record decoded as it arrives owns the tree from then on: it
    // stands while the record does, once the observer's end has followed.
    let (ours, theirs) = pair();
    let tree = handed_tree();
    let bytes = record(SOURCES, entry(&tree)).to_string().into_bytes();
    send(&theirs, &bytes, SendFlags::empty()).unwrap();
    drop(theirs);
    let soon = Instant::now() + Duration::from_secs(5);
    let (observed, _) = received(&ours, soon, &live).unwrap();
    assert!(tree.is_dir());
    drop(observed);
    assert!(!tree.exists());
    // Refused where no end follows by the deadline, the attempt is
    // cancelled after the record, or a message more follows: each removes
    // the tree.
    for case in ["deadline", "cancelled", "more"] {
        let (ours, theirs) = pair();
        let tree = handed_tree();
        let bytes = record(SOURCES, entry(&tree)).to_string().into_bytes();
        send(&theirs, &bytes, SendFlags::empty()).unwrap();
        if case == "more" {
            send(&theirs, b"{}", SendFlags::empty()).unwrap();
        }
        let asked = std::cell::Cell::new(0);
        let cancelled = || {
            asked.set(asked.get() + 1);
            (case == "cancelled") & (asked.get() > 1)
        };
        let deadline = Instant::now() + Duration::from_millis(200);
        let answer = received(&ours, deadline, &cancelled).map(drop);
        let left = tree.exists();
        assert_eq!((case, answer, left), (case, Err(Refusal::Identity), false));
    }
}

#[test]
fn a_record_the_observer_cannot_hand_back_removes_the_identity_tree_it_carried() {
    // A record beyond the socket's bound is never sent: dropped, not
    // forgotten, it removes the tree its entry would have handed over.
    let tree = handed_tree();
    let mut oversized = record(SOURCES, entry(&tree));
    oversized["observed"]["handles"] = json!([format!("/{}", "a".repeat(128 << 10))]);
    let oversized: Record = serde_json::from_value(oversized).unwrap();
    let answer = handed(oversized, None);
    assert_eq!((answer, tree.exists()), (Err(Refusal::Identity), false));
}

/// The box intent sealed for [`SOURCES`], its one writer confined.
fn intent_sealed() -> BoxIntent {
    serde_json::from_value(box_sealed()).unwrap()
}

/// [`intent_sealed`], as it travels.
fn box_sealed() -> serde_json::Value {
    json!({
        "reach": {"writable": [], "readable": []}, "executable": "/bin/true",
        "tree": {"kind": "system"},
        "sources": {"entries": 1, "mounts": 1, "digest": SOURCES},
        "writers": {"uids": [1000], "privilege": "confined"}, "network": "isolated",
        "environment": [], "bootstrap": {"path": "/", "digest": PLAN},
        "excluded": {"store": "/store", "control": []}
    })
}

/// A plan of [`intent_sealed`]'s box declaring the binding `names`.
fn plan_sealed(names: &[&str]) -> Plan {
    let digest = |byte: &str| byte.repeat(64);
    serde_json::from_value(json!({
        "owner": {"repo": digest("a"), "run": "r", "effect": "e", "attempt": "a", "site": "s",
                  "instance": "s#0"},
        "server": "cap-c", "capability": "c",
        "dialect": {"name": "d", "digest": digest("b"), "definition": digest("c"), "version": "1"},
        "connection": {"argv": ["/bin/true"]}, "tools": ["t"], "restrictions": {},
        "retained": false, "secrets": names,
        "clearance": {"dialect": digest("b"), "policy": digest("d")}, "box": box_sealed(),
    }))
    .unwrap()
}

#[test]
fn a_launcher_that_cannot_start_is_unavailable_and_one_that_stands_no_box_is_not_established() {
    let plan = plan_sealed(&[]);
    // Nothing stands to be served.
    let served = || |_: Link<'_>, _: &mut PipeReader, ()| Ok::<(), Refusal>(());
    // A launcher that cannot start at all: its handle a directory's.
    let directory = admitted(std::fs::File::open("/").unwrap().into());
    let answer = ready(&directory, (&plan, PLAN), startup(&live), unbound, served());
    assert_eq!(answer, Err(Refusal::Unavailable));
    // One that starts and fails naming no first process, as bubblewrap
    // denied its namespaces does.
    let failing = admitted(std::fs::File::open("/bin/false").unwrap().into());
    let answer = ready(&failing, (&plan, PLAN), startup(&live), unbound, served());
    assert_eq!(answer, Err(Refusal::Establishment));
}

/// A pipe whose reader has read nothing and whose every byte is written.
fn full() -> (PipeReader, PipeWriter) {
    let (reader, mut writer) = std::io::pipe().unwrap();
    rustix::io::ioctl_fionbio(&writer, true).unwrap();
    for size in [4096, 1] {
        while writer.write(&vec![0; size]).is_ok() {}
    }
    (reader, writer)
}

/// What sending `frame` on `writer` within `startup` answers, and how long
/// it took, or none where it did not answer within five seconds.
fn sending(
    mut writer: PipeWriter,
    frame: Vec<u8>,
    startup: impl FnOnce() -> Startup<'static> + Send + 'static,
) -> Option<(Result<(), Refusal>, Duration)> {
    let (answered, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let begun = Instant::now();
        let sent = sent(&mut writer, &frame, startup());
        answered.send((sent, begun.elapsed())).ok();
    });
    answer.recv_timeout(Duration::from_secs(5)).ok()
}

#[test]
fn the_intent_is_sent_whole_or_refused_within_the_deadline_and_cancellation() {
    let frame = framed(&intent(&"a".repeat(FRAME_MAX - 400))).unwrap();
    // A bootstrap that drains a full pipe late takes the whole frame.
    let (mut reader, writer) = full();
    let drained = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let answer = sending(writer, frame.clone(), || startup(&live));
    assert_eq!(answer.map(|(sent, _)| sent), Some(Ok(())));
    assert!(drained.join().unwrap().ends_with(&frame));
    // One that never reads is refused at the absolute deadline.
    let (_reader, writer) = full();
    let soon = Instant::now() + Duration::from_millis(200);
    let answer = sending(writer, frame.clone(), move || (soon, &live));
    let (sent, took) = answer.unwrap();
    assert_eq!(sent, Err(Refusal::Establishment));
    assert!(took < Duration::from_secs(2), "{took:?}");
    // And at once when the attempt is cancelled.
    let (_reader, writer) = full();
    let answer = sending(writer, frame, || startup(&gone));
    assert_eq!(
        answer.map(|(sent, _)| sent),
        Some(Err(Refusal::Establishment))
    );
}

/// Always cancelled: the attempt's starter has ended.
fn gone() -> bool {
    true
}

/// An intent for the sealed digests whose one mount stands at `path`, its
/// server `/bin/true` declaring `DOCS_TOKEN`.
fn intent(path: &str) -> Intent {
    Intent {
        plan: PLAN.to_string(),
        sources: SOURCES.to_string(),
        network: Network::Isolated,
        host: spaces(),
        mounts: vec![Source {
            path: path.into(),
            device: (8, 1),
            inode: 77,
        }],
        executable: "/bin/true".into(),
        argv: vec!["true".to_string(), "--stdio".to_string()],
        secrets: vec!["DOCS_TOKEN".to_string()],
    }
}

#[test]
fn the_intent_is_one_frame_of_at_most_four_kibibytes() {
    let frame = framed(&intent("/usr/bin")).unwrap();
    let body = json!({
        "plan": PLAN, "sources": SOURCES, "network": "isolated",
        "host": {"mnt": 11, "pid": 12, "net": 13, "ipc": 14, "uts": 15},
        "mounts": [{"path": "/usr/bin", "device": [8, 1], "inode": 77}],
        "executable": "/bin/true", "argv": ["true", "--stdio"], "secrets": ["DOCS_TOKEN"],
    });
    let length = u32::from_be_bytes(frame[..4].try_into().unwrap());
    let parsed: serde_json::Value = serde_json::from_slice(&frame[4..]).unwrap();
    assert_eq!((length as usize, parsed), (frame.len() - 4, body));
    // The body at the bound, and one byte over it.
    let fixed = framed(&intent("")).unwrap().len() - 4;
    let at = framed(&intent(&"a".repeat(FRAME_MAX - fixed)));
    assert_eq!(at.map(|frame| frame.len()), Ok(FRAME_MAX + 4));
    let over = framed(&intent(&"a".repeat(FRAME_MAX - fixed + 1)));
    assert_eq!(over, Err(Refusal::Establishment));
}

/// A status pipe `bytes` were written to, its writer then closed.
fn status_of(bytes: &[u8]) -> Result<(), Refusal> {
    exec_status(&mut written(bytes), startup(&live))
}

#[test]
fn the_status_pipe_permits_initialization_only_at_its_end_and_reports_each_tag() {
    // Its end alone, as the server's exec closes it, permits the server's
    // initialization; each tag its own cause; anything else, the sender
    // having completed, the boxed-exec cause.
    let cases: [(&[u8], Result<(), Refusal>); 6] = [
        (b"", Ok(())),
        (b"H", Err(Refusal::Handoff)),
        (b"E", Err(Refusal::Exec)),
        (b"x", Err(Refusal::Exec)),
        (b"HH", Err(Refusal::Exec)),
        (b"EEEE", Err(Refusal::Exec)),
    ];
    for (bytes, cause) in cases {
        assert_eq!((bytes, status_of(bytes)), (bytes, cause));
    }
    // A status pipe still open at the deadline, or on cancellation, is the
    // handoff cause: the bootstrap never reached its exec.
    let (mut open, _writer) = std::io::pipe().unwrap();
    let soon = (Instant::now() + Duration::from_millis(100), &live as _);
    assert_eq!(exec_status(&mut open, soon), Err(Refusal::Handoff));
    assert_eq!(
        exec_status(&mut open, startup(&gone)),
        Err(Refusal::Handoff)
    );
}

/// An admission whose store, in `dir`, holds `store`, owner-only.
fn admitted_store(dir: &Path, store: &str) -> Admitted {
    let path = dir.join("store");
    std::fs::write(&path, store).unwrap();
    std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600)).unwrap();
    let mut admitted = admitted(std::fs::File::open("/bin/true").unwrap().into());
    admitted.store = std::fs::File::open(&path).unwrap().into();
    admitted
}

/// A session's binder that keeps nothing and refuses nothing.
fn unbound(_: Handed<Vec<BoundSecret>>) -> Result<(), Refusal> {
    Ok(())
}

/// The names of the bindings `handed`, none where nothing was looked up.
fn named(handed: Handed<Vec<BoundSecret>>) -> Vec<String> {
    let bindings = match handed {
        Handed::Published(bindings) => bindings,
        Handed::Nothing => Vec::new(),
        Handed::Uncertain => panic!("a sender that answered left its bindings uncertain"),
    };
    bindings
        .iter()
        .map(|bound| bound.name().to_owned())
        .collect()
}

/// A delivery's answer, whether it closed its control pipe, and what that
/// pipe carried.
type Delivery = (Result<(), Refusal>, bool, Vec<u8>);

/// What [`delivered`] answers for a plan declaring `names`, its store
/// holding `store`, on a control pipe whose reader is `kept` or gone and a
/// status pipe that holds `status`, its binder answering `bound`: its
/// answer, whether it closed the control pipe, and every byte that pipe
/// carried, where its reader was kept; and the names its binder was given.
fn delivering(
    names: &[&str],
    store: &str,
    (kept, status): (bool, &[u8]),
    bound: Result<(), Refusal>,
) -> (Delivery, Option<Vec<String>>) {
    let dir = tempfile::tempdir().unwrap();
    let admitted = admitted_store(dir.path(), store);
    let (reader, writer) = std::io::pipe().unwrap();
    let mut reader = kept.then_some(reader);
    let mut control = Some(writer);
    let given = std::cell::RefCell::new(None);
    let bind = |handed| {
        given.replace(Some(named(handed)));
        bound
    };
    let answer = delivered(
        (&mut control, &mut written(status)),
        (&admitted, &plan_sealed(names)),
        startup(&live),
        (
            || panic!("a sender that answered in time ended its box"),
            bind,
        ),
    );
    let closed = control.is_none();
    drop(control);
    let mut carried = Vec::new();
    if let Some(reader) = reader.as_mut() {
        reader.read_to_end(&mut carried).unwrap();
    }
    ((answer, closed, carried), given.into_inner())
}

#[test]
fn the_bindings_are_looked_up_after_readiness_sent_once_and_their_pipe_closed() {
    // The declared value, sent as the one frame, and the control pipe
    // closed after it: the reader meets its end.
    let stored = "DOCS_TOKEN=value\n";
    let body = br#"{"DOCS_TOKEN":"value"}"#;
    let length = u32::try_from(body.len()).unwrap().to_be_bytes();
    let frame = [&length[..], body].concat();
    // The binder is given each binding looked up, whatever follows.
    let token = Some(vec!["DOCS_TOKEN".to_string()]);
    let answer = delivering(&["DOCS_TOKEN"], stored, (true, b""), Ok(()));
    assert_eq!(answer, ((Ok(()), true, frame.clone()), token.clone()));
    // No name, no lookup: an empty frame, whatever the store holds.
    let answer = delivering(&[], "not an assignment", (true, b""), Ok(()));
    let empty = [0, 0, 0, 2, b'{', b'}'].to_vec();
    assert_eq!(answer, ((Ok(()), true, empty), Some(vec![])));
    // A name the store lacks is decision 0012's own cause, and nothing is
    // sent or bound; a bootstrap gone before the frame, the handoff's, its
    // bindings given to the binder first (operator ruling 2026-10-13).
    let missing = Refusal::Name(
        "secret 'DOCS_TOKEN' is not in the store at /store (brokkr secrets set DOCS_TOKEN)"
            .to_string(),
    );
    let answer = delivering(&["DOCS_TOKEN"], "", (true, b""), Ok(()));
    assert_eq!(answer, ((Err(missing), false, Vec::new()), Some(vec![])));
    let answer = delivering(&["DOCS_TOKEN"], stored, (false, b""), Ok(()));
    let gone = (Err(Refusal::Handoff), false, Vec::new());
    assert_eq!(answer, (gone, token.clone()));
    // The bootstrap's report after the whole frame keeps its own cause.
    let answer = delivering(&["DOCS_TOKEN"], stored, (true, b"H"), Ok(()));
    assert_eq!(
        answer,
        ((Err(Refusal::Handoff), true, frame), token.clone())
    );
    // The binder's refusal is the answer, before the send's own cause or
    // the bootstrap's is read, and the control pipe is not closed for it.
    let refused = delivering(&["DOCS_TOKEN"], stored, (false, b""), Err(Refusal::Masking));
    assert_eq!(refused, ((Err(Refusal::Masking), false, Vec::new()), token));
}

#[test]
fn a_control_pipe_already_closed_carries_no_frame() {
    let mut admitted = admitted(std::fs::File::open("/bin/true").unwrap().into());
    admitted.store = std::fs::File::open("/dev/null").unwrap().into();
    let answer = delivered(
        (&mut None, &mut written(b"")),
        (&admitted, &plan_sealed(&[])),
        startup(&live),
        (|| panic!("no sender ran"), unbound),
    );
    assert_eq!(answer, Err(Refusal::Handoff));
}

/// A store a peer holds: a FIFO whose one writer, which the test keeps,
/// writes nothing, so that a read of it that waited would wait until
/// [`HeldStore::watched`] closes that writer.
struct HeldStore {
    path: PathBuf,
    writer: std::sync::Mutex<Option<OwnedFd>>,
}

impl HeldStore {
    fn new(dir: &Path) -> HeldStore {
        let path = dir.canonicalize().unwrap().join("store");
        let fifo = rustix::fs::FileType::Fifo;
        let mode = rustix::fs::Mode::from_raw_mode(0o600);
        rustix::fs::mknodat(rustix::fs::CWD, &path, fifo, mode, 0).unwrap();
        // Opened to read and write, a FIFO's open waits for no peer.
        let flags = rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CLOEXEC;
        let writer = rustix::fs::open(&path, flags, rustix::fs::Mode::empty()).unwrap();
        HeldStore {
            path,
            writer: std::sync::Mutex::new(Some(writer)),
        }
    }

    /// The admitted path handle on the store, as admission holds one.
    fn handle(&self) -> OwnedFd {
        let flags = rustix::fs::OFlags::PATH | rustix::fs::OFlags::CLOEXEC;
        rustix::fs::open(&self.path, flags, rustix::fs::Mode::empty()).unwrap()
    }

    /// Let a reader still waiting go after five seconds, so that a sender
    /// that waits on the peer fails here rather than hangs: its writer
    /// closed, the reader meets the store's end.
    fn watched(&self, answered: &std::sync::atomic::AtomicBool) {
        let until = Instant::now() + Duration::from_secs(5);
        while !answered.load(std::sync::atomic::Ordering::SeqCst) & (Instant::now() < until) {
            std::thread::sleep(Duration::from_millis(20));
        }
        self.writer.lock().unwrap().take();
    }
}

#[test]
fn a_store_its_peer_holds_answers_at_once_with_its_read_cause() {
    // The store a FIFO whose writer is open and silent: a read that waited
    // would wait on that peer, so it answers at once, long before the
    // deadline, with the store's read cause; the box is not ended by the
    // sender's supervisor and no frame is sent.
    let dir = tempfile::tempdir().unwrap();
    let store = HeldStore::new(dir.path());
    let mut admitted = admitted(std::fs::File::open("/bin/true").unwrap().into());
    admitted.store = store.handle();
    let (mut reader, writer) = std::io::pipe().unwrap();
    let mut control = Some(writer);
    let answered = std::sync::atomic::AtomicBool::new(false);
    let begun = Instant::now();
    let answer = std::thread::scope(|scope| {
        scope.spawn(|| store.watched(&answered));
        let pipes = (&mut control, &mut written(b""));
        let plan = plan_sealed(&["DOCS_TOKEN"]);
        let far = (begun + Duration::from_secs(30), &live as _);
        let end = || panic!("a sender that answered in time ended its box");
        let answer = delivered(pipes, (&admitted, &plan), far, (end, unbound));
        answered.store(true, std::sync::atomic::Ordering::SeqCst);
        answer
    });
    let taken = begun.elapsed();
    drop(control);
    let mut carried = Vec::new();
    reader.read_to_end(&mut carried).unwrap();
    let cause = "cannot read secrets store /store: Resource temporarily unavailable (os error 11)";
    let refused = Err(Refusal::Name(cause.to_string()));
    assert_eq!((answer, carried), (refused, Vec::new()));
    assert!(taken < Duration::from_secs(2), "{taken:?}");
}

/// What [`delivered`] answers within `startup` for a plan declaring
/// `DOCS_TOKEN` from a store that holds it, on a control pipe that is full
/// and that no one reads, and when its box was ended.
fn untaken_delivery(startup: Startup<'_>) -> (Result<(), Refusal>, Option<Instant>) {
    let dir = tempfile::tempdir().unwrap();
    let admitted = admitted_store(dir.path(), "DOCS_TOKEN=value\n");
    let (_reader, writer) = full();
    let ended = std::cell::Cell::new(None);
    let pipes = (&mut Some(writer), &mut written(b""));
    let plan = plan_sealed(&["DOCS_TOKEN"]);
    let end = || ended.set(Some(Instant::now()));
    let answer = delivered(pipes, (&admitted, &plan), startup, (end, unbound));
    (answer, ended.get())
}

#[test]
fn a_frame_not_taken_by_the_deadline_ends_the_box_at_the_deadline() {
    // The sender's frame is not taken: at the absolute deadline, and no
    // more than a slice after, the box is ended and the handoff refused.
    let deadline = Instant::now() + Duration::from_millis(300);
    let (answer, ended) = untaken_delivery((deadline, &live));
    assert_eq!(answer, Err(Refusal::Handoff));
    let after = ended.and_then(|at| at.checked_duration_since(deadline));
    let bounded = after.is_some_and(|after| after < Duration::from_secs(2));
    assert!(bounded, "{after:?}");
}

#[test]
fn a_frame_not_taken_when_the_attempt_is_cancelled_ends_the_box_at_once() {
    let begun = Instant::now();
    let (answer, ended) = untaken_delivery((begun + Duration::from_secs(30), &gone));
    assert_eq!(answer, Err(Refusal::Handoff));
    let taken = ended.map(|at| at - begun);
    let bounded = taken.is_some_and(|taken| taken < Duration::from_secs(2));
    assert!(bounded, "{taken:?}");
}

/// What the supervisor answers within `startup` for a sender held, as a
/// store read the kernel holds is, until it is let go or two seconds pass:
/// its answer; how long after `from` it returned and ended the box; whether
/// the sender was still held when it returned; and whether the sender,
/// once let go, saw itself given up on.
fn held_sender(startup: Startup<'_>, from: Instant) -> HeldSender {
    let (release, held) = std::sync::mpsc::channel::<()>();
    let (report, reported) = std::sync::mpsc::channel();
    let ended = std::cell::Cell::new(None);
    let (_, answer) = supervised(
        std::thread::Builder::new(),
        startup,
        || ended.set(Some(from.elapsed())),
        move |given_up, _: &Published<()>| {
            held.recv_timeout(Duration::from_secs(2)).ok();
            report
                .send(given_up.load(std::sync::atomic::Ordering::SeqCst))
                .ok();
            Ok(())
        },
    );
    let returned = from.elapsed();
    let still_held = reported.try_recv().is_err();
    drop(release);
    let given_up = reported.recv_timeout(Duration::from_secs(5)).ok();
    (answer, (returned, ended.get()), still_held, given_up)
}

/// [`held_sender`]'s answer.
type HeldSender = (
    Result<(), Refusal>,
    (Duration, Option<Duration>),
    bool,
    Option<bool>,
);

#[test]
fn a_sender_held_past_the_deadline_is_refused_at_the_deadline_and_not_waited_for() {
    // At a 40 ms deadline, and no more than a few slices after, the box is
    // ended and the handoff refused, the sender still held and told it was
    // given up on (operator ruling 2026-10-11).
    let begun = Instant::now();
    let deadline = begun + Duration::from_millis(40);
    let (answer, (returned, ended), still_held, given_up) = held_sender((deadline, &live), begun);
    assert_eq!(
        (answer, still_held, given_up),
        (Err(Refusal::Handoff), true, Some(true))
    );
    let bounded =
        |at: Duration| (at >= Duration::from_millis(40)) & (at < Duration::from_millis(200));
    assert!(
        ended.is_some_and(bounded) & bounded(returned),
        "{ended:?} {returned:?}"
    );
}

#[test]
fn a_sender_held_when_the_attempt_is_cancelled_is_refused_at_once_and_not_waited_for() {
    let begun = Instant::now();
    let far = (begun + Duration::from_secs(30), &gone as _);
    let (answer, (returned, ended), still_held, given_up) = held_sender(far, begun);
    assert_eq!(
        (answer, still_held, given_up),
        (Err(Refusal::Handoff), true, Some(true))
    );
    let bounded = |at: Duration| at < Duration::from_millis(200);
    assert!(
        ended.is_some_and(bounded) & bounded(returned),
        "{ended:?} {returned:?}"
    );
}

#[test]
fn a_sender_thread_that_cannot_be_made_is_the_handoff_cause() {
    // No stack this large can be mapped, so no thread is made: the handoff
    // cause, with no sender run and no box ended by the supervisor.
    let unmade = std::thread::Builder::new().stack_size(usize::MAX);
    let answer: (Handed<()>, Result<(), Refusal>) = supervised(
        unmade,
        startup(&live),
        || panic!("a sender never made ended its box"),
        |_, _| panic!("a sender never made ran"),
    );
    assert_eq!(answer, (Handed::Nothing, Err(Refusal::Handoff)));
}

/// What a bound plan's ledger proves: each call and its outcome, its
/// closure and why verification stopped, if it did.
type Proved = (
    Vec<(String, Option<Outcome>)>,
    Option<Disposition>,
    Option<Invalid>,
);

/// A bound plan's lifetime whose session delivers `DOCS_TOKEN`, stored as
/// `value`, on a control pipe that is full and that no one reads, within
/// `startup`, its bindings given to the lifetime's binder: how the
/// lifetime ended, and what its ledger proves.
fn given_up(value: &str, startup: Startup<'_>) -> (Result<(), Ended>, Proved) {
    let dir = tempfile::tempdir().unwrap();
    let admitted = admitted_store(dir.path(), &format!("DOCS_TOKEN={value}\n"));
    let attempt: OwnedFd = std::fs::File::open(dir.path()).unwrap().into();
    let plan = plan_sealed(&["DOCS_TOKEN"]);
    let (_reader, writer) = full();
    let ended = plan_lived(&attempt, &plan, |binder| {
        let pipes = (&mut Some(writer), &mut written(b""));
        let bind = |found| binder.bind(found);
        delivered(pipes, (&admitted, &plan), startup, (|| (), bind)).map(drop)
    });
    (ended, proved(dir.path(), &plan))
}

/// `session` run in the lifetime of the ledger `plan` opens in `attempt`,
/// as `serve` runs it, its standard error taking whatever it is told.
fn plan_lived(
    attempt: &OwnedFd,
    plan: &Plan,
    session: impl FnOnce(Binder<'_, '_>) -> Result<(), Ended>,
) -> Result<(), Ended> {
    let ledger = (attempt, std::ffi::OsStr::new("plan"));
    let held = Held::default();
    lived(&held, ledger, (&plan.owner, PLAN), &|_| Put::Taken, session)
}

/// What the ledger `plan`'s lifetime left in `dir` proves.
fn proved(dir: &Path, plan: &Plan) -> Proved {
    let bytes = std::fs::read(dir.join("plan.ledger")).unwrap();
    let (lifecycle, invalid) = recover(&bytes, &plan.owner, PLAN);
    (
        lifecycle.calls().to_vec(),
        lifecycle.closed().cloned(),
        invalid,
    )
}

#[test]
fn bindings_looked_up_before_a_stalled_write_still_guard_the_closure_given_up_on() {
    // The frame is never taken, so the startup gives up on its write, at
    // its deadline or on the attempt's cancellation, half a second in,
    // long after the store was read. The binding looked up is still given
    // to the ledger (operator ruling 2026-10-13): DOCS_TOKEN's 0002 is the
    // Closed's sequence, 2, and `closed` its record's name, so the Closed
    // the handoff cause would close with is refused and never appended,
    // and the lifetime ends with the masking cause.
    let soon = || Instant::now() + Duration::from_millis(500);
    let unclosed = || (vec![], None, Some(Invalid::Lifecycle));
    let masking = || Err(Ended::Failed(Fatal::Failed(Failure::Masking)));
    for value in ["0002", "closed"] {
        let ran = given_up(value, (soon(), &live));
        let deadline = (value, "deadline", ran);
        assert_eq!(deadline, (value, "deadline", (masking(), unclosed())));
        let at = soon();
        let late = move || Instant::now() >= at;
        let ran = given_up(value, (at + Duration::from_secs(30), &late));
        let cancelled = (value, "cancelled", ran);
        assert_eq!(cancelled, (value, "cancelled", (masking(), unclosed())));
    }
    // The control: an unrelated binding leaves the handoff cause the
    // session's, and its closure.
    let handoff = Failure::Refused(Refusal::Handoff);
    let closed = (vec![], Some(Disposition::Failed(handoff)), None);
    let deadline = Instant::now() + Duration::from_millis(300);
    let ran = given_up("unrelated", (deadline, &live));
    assert_eq!(ran, (Err(Ended::Refused(Refusal::Handoff)), closed));
}

#[test]
fn what_a_sender_published_before_it_failed_or_was_given_up_on_is_taken() {
    // A sender that fails once it has published.
    let failing = |_: &AtomicBool, published: &Published<u8>| {
        published.publish(7);
        panic!("the sender failed after publishing");
    };
    let ended = || panic!("a sender that answered ended its box");
    let answer = supervised(std::thread::Builder::new(), startup(&live), ended, failing);
    assert_eq!(
        answer,
        (Handed::Published(7), Err::<(), _>(Refusal::Handoff))
    );
    // One held past the deadline once it has published, and one held when
    // the attempt is cancelled.
    let held = |startup: Startup<'_>| {
        let (release, held) = std::sync::mpsc::channel::<()>();
        let answer = supervised(
            std::thread::Builder::new(),
            startup,
            || (),
            move |_, published: &Published<u8>| {
                published.publish(7);
                held.recv_timeout(Duration::from_secs(2)).ok();
                Ok(())
            },
        );
        drop(release);
        answer
    };
    let soon = || Instant::now() + Duration::from_millis(200);
    let given_up = (Handed::Published(7), Err(Refusal::Handoff));
    assert_eq!(held((soon(), &live)), given_up);
    let at = soon();
    let late = move || Instant::now() >= at;
    assert_eq!(held((at + Duration::from_secs(30), &late)), given_up);
    // The control: one that answers in time having published nothing.
    let quiet = |_: &AtomicBool, _: &Published<u8>| Ok(());
    let ended = || panic!("a sender that answered ended its box");
    let answer = supervised(std::thread::Builder::new(), startup(&live), ended, quiet);
    assert_eq!(answer, (Handed::Nothing, Ok(())));
}

/// A bound plan's lifetime whose sender, if it `begins` its lookup, marks
/// it begun, and is then held past a 200 ms deadline before it publishes,
/// as one whose lookup resolved and whose publication had not yet come is:
/// how the lifetime ended, and what its ledger proves.
fn interleaved(begins: bool) -> (Result<(), Ended>, Proved) {
    let dir = tempfile::tempdir().unwrap();
    let attempt: OwnedFd = std::fs::File::open(dir.path()).unwrap().into();
    let plan = plan_sealed(&["DOCS_TOKEN"]);
    let (release, held) = std::sync::mpsc::channel::<()>();
    let ended = plan_lived(&attempt, &plan, |binder| {
        let deadline = Instant::now() + Duration::from_millis(200);
        let sender = move |_: &AtomicBool, published: &Published<Vec<BoundSecret>>| {
            let begun = begins && published.begin();
            held.recv_timeout(Duration::from_secs(2)).ok();
            published.publish(Vec::new());
            ensure(begun, Refusal::Handoff)
        };
        let thread = std::thread::Builder::new();
        let (handed, sent) = supervised(thread, (deadline, &live), || (), sender);
        binder.bind(handed)?;
        sent.map_err(Ended::from)
    });
    drop(release);
    (ended, proved(dir.path(), &plan))
}

#[test]
fn a_sender_given_up_on_between_its_lookup_and_its_publication_leaves_the_ledger_unclosed() {
    // Its lookup begun and nothing published when the startup gives up on
    // it: what it holds is unknown, so no guard can hold, and no closure is
    // appended, however its bindings are spelled; the lifetime ends with
    // the handoff cause, and recovery finds the Opened alone (operator
    // ruling 2026-10-13). Its later publication is never taken.
    let handoff = || Err(Ended::Refused(Refusal::Handoff));
    let unclosed = (vec![], None, Some(Invalid::Lifecycle));
    assert_eq!(interleaved(true), (handoff(), unclosed));
    // The control: given up on before its lookup began, it held nothing,
    // and the closure is appended with the handoff cause.
    let closed = Some(Disposition::Failed(Failure::Refused(Refusal::Handoff)));
    assert_eq!(interleaved(false), (handoff(), (vec![], closed, None)));
}

#[test]
fn a_lookup_begun_and_never_published_is_uncertain_unless_its_sender_returned() {
    type Sender = Box<dyn FnOnce(&AtomicBool, &Published<u8>) -> Result<(), Refusal> + Send>;
    let take = |sender: Sender| {
        let deadline = Instant::now() + Duration::from_millis(200);
        supervised(
            std::thread::Builder::new(),
            (deadline, &live),
            || (),
            sender,
        )
    };
    let handoff = || Err(Refusal::Handoff);
    // One that failed once its lookup began holds what it held unknown.
    let failed: Sender = Box::new(|_, published| {
        published.begin();
        panic!("the sender failed during its lookup");
    });
    assert_eq!(take(failed), (Handed::Uncertain, handoff()));
    // One that returned from a lookup that refused publishes nothing, and
    // holds nothing.
    let refused: Sender = Box::new(|_, published| {
        published.begin();
        Err(Refusal::Handoff)
    });
    assert_eq!(take(refused), (Handed::Nothing, handoff()));
    // One given up on before it began may not begin its lookup.
    let (report, reported) = std::sync::mpsc::channel();
    let late: Sender = Box::new(move |given_up, published| {
        let until = Instant::now() + Duration::from_secs(2);
        while !given_up.load(std::sync::atomic::Ordering::SeqCst) & (Instant::now() < until) {
            std::thread::sleep(Duration::from_millis(5));
        }
        report.send(published.begin()).ok();
        Ok(())
    });
    assert_eq!(take(late), (Handed::Nothing, handoff()));
    let begun = reported.recv_timeout(Duration::from_secs(5));
    assert_eq!(begun, Ok(false));
}
