//! The readiness receiver's reading and deciding (decision 0065 slice two,
//! U6c6b; MB3, MB5): real anonymous pipes stand for the box's, the host's
//! view of the box is stood in, and launchers that stand no box are real
//! processes. The live box is the CLI suite's.

use std::io::{PipeReader, PipeWriter, Read, Write};
use std::os::fd::OwnedFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use brokkr_protocol::broker::{BoxIntent, Refusal};
use serde_json::json;

use super::super::helper::{handed, received};
use super::linux::{
    decided, first, framed, message, read, sent, Egress, Intent, Ready, Seen, Source, Spaces,
    FRAME_MAX,
};
use super::{compared, ready, Admitted, Record, Startup};

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
        _store: store.into(),
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
    serde_json::from_value(json!({
        "reach": {"writable": [], "readable": []}, "executable": "/bin/true",
        "tree": {"kind": "system"},
        "sources": {"entries": 1, "mounts": 1, "digest": SOURCES},
        "writers": {"uids": [1000], "privilege": "confined"}, "network": "isolated",
        "environment": [], "bootstrap": {"path": "/", "digest": PLAN},
        "excluded": {"store": "/store", "control": []}
    }))
    .unwrap()
}

#[test]
fn a_launcher_that_cannot_start_is_unavailable_and_one_that_stands_no_box_is_not_established() {
    let intent = intent_sealed();
    // A launcher that cannot start at all: its handle a directory's.
    let directory = admitted(std::fs::File::open("/").unwrap().into());
    let answer = ready(&directory, (&intent, PLAN), startup(&live));
    assert_eq!(answer, Err(Refusal::Unavailable));
    // One that starts and fails naming no first process, as bubblewrap
    // denied its namespaces does.
    let failing = admitted(std::fs::File::open("/bin/false").unwrap().into());
    let answer = ready(&failing, (&intent, PLAN), startup(&live));
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

/// An intent for the sealed digests whose one mount stands at `path`.
fn intent(path: &str) -> Intent<'static> {
    Intent {
        plan: PLAN,
        sources: SOURCES,
        network: Egress::Isolated,
        host: spaces(),
        mounts: vec![Source {
            path: path.to_string(),
            device: (8, 1),
            inode: 77,
        }],
    }
}

#[test]
fn the_intent_is_one_frame_of_at_most_four_kibibytes() {
    let frame = framed(&intent("/usr/bin")).unwrap();
    let body = json!({
        "plan": PLAN, "sources": SOURCES, "network": "isolated",
        "host": {"mnt": 11, "pid": 12, "net": 13, "ipc": 14, "uts": 15},
        "mounts": [{"path": "/usr/bin", "device": [8, 1], "inode": 77}],
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
