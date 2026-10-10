//! The waiting bootstrap's decisions (U6c6a), driven through its own
//! pipes with the box's facts stood in: no test here launches a box. The
//! real binary's refusals are `tests/capability_broker.rs`'s.

use std::io::{PipeReader, PipeWriter};
use std::os::fd::{AsRawFd, IntoRawFd};

use serde_json::{json, Value};

use super::*;

const PLAN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SOURCES: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

/// A box's mount table every check admits: bubblewrap's empty root, as a
/// live bubblewrap 0.11 namespace shows it, its own `/proc` with two covers
/// and its minimal `nodev` `/dev` with two nodes, terminals and queues, the
/// two private tmpfs, and the [`SEALED`] sources read-only: a system
/// source, a nested mount below it and a package at a path holding a space.
const MOUNTS: &str = "\
20 1 0:40 /newroot / rw,nosuid,nodev,relatime - tmpfs tmpfs rw,uid=1000,gid=1000,inode64
21 20 0:41 / /proc rw,nosuid,nodev,noexec - proc proc rw
22 21 0:41 /sys /proc/sys ro,nosuid,nodev,noexec - proc proc rw
23 20 0:42 / /dev rw,nosuid,nodev - tmpfs tmpfs rw,mode=755
24 23 0:5 /null /dev/null rw,nosuid - devtmpfs udev rw
25 23 0:43 / /dev/pts rw,nosuid,noexec - devpts devpts rw
26 20 8:1 /usr /usr ro,nosuid,nodev - ext4 /dev/sda1 rw
27 26 8:2 / /usr/local ro,nosuid,nodev - ext4 /dev/sda2 rw
28 20 0:44 / /runtime/home rw,nosuid,nodev - tmpfs tmpfs rw,mode=700
29 20 0:45 / /tmp rw,nosuid,nodev - tmpfs tmpfs rw,mode=700
30 20 8:1 /opt/docs\\040mcp /opt/docs\\040mcp ro,nosuid,nodev - ext4 /dev/sda1 rw
31 21 0:41 /bus /proc/bus ro,nosuid,nodev,noexec - proc proc rw
32 23 0:5 /zero /dev/zero rw,nosuid - devtmpfs udev rw
33 23 0:46 / /dev/mqueue rw,nosuid,nodev,noexec - mqueue mqueue rw
";

/// The mounts the intent seals, each by its path in the box and the
/// device and inode of its source.
const SEALED: [(&str, (u32, u32), u64); 3] = [
    ("/usr", (8, 1), 100),
    ("/usr/local", (8, 2), 2),
    ("/opt/docs mcp", (8, 1), 300),
];

/// The broker's namespaces, as an intent seals them.
const HOST: [u64; 5] = [1, 2, 3, 4, 5];

/// Namespaces by their five identities, in [`Spaces`]' order.
fn ids([mnt, pid, net, ipc, uts]: [u64; 5]) -> Spaces {
    Spaces {
        mnt,
        pid,
        net,
        ipc,
        uts,
    }
}

/// A fresh private directory at `path`: empty, `0700` and uid 1000's.
fn directory(path: &str) -> Directory {
    Directory {
        path: PathBuf::from(path),
        mode: PRIVATE_MODE,
        uid: 1000,
        empty: true,
    }
}

/// A box every check admits: its own namespaces, MB4's fixed environment
/// alone, fresh private directories, [`MOUNTS`] and each [`SEALED`]
/// source's identity at its path.
fn admitted() -> Observed {
    Observed {
        pid: 2,
        euid: 1000,
        spaces: Some(ids([11, 12, 13, 14, 15])),
        mounts: MOUNTS.to_string(),
        sources: SEALED
            .iter()
            .map(|(_, device, inode)| Some((*device, *inode)))
            .collect(),
        environment: server_environment().map(OsString::from).collect(),
        private: vec![Some(directory("/runtime/home")), Some(directory("/tmp"))],
    }
}

/// The admitted box with `line` of [`MOUNTS`] replaced by `with`.
fn remounted(line: usize, with: &str) -> Observed {
    let mut lines: Vec<&str> = MOUNTS.lines().collect();
    lines[line] = with;
    Observed {
        mounts: lines.join("\n"),
        ..admitted()
    }
}

/// The admitted box with `line` added to [`MOUNTS`].
fn added(line: &str) -> Observed {
    Observed {
        mounts: format!("{MOUNTS}{line}\n"),
        ..admitted()
    }
}

/// An intent for [`PLAN`] and [`SOURCES`] on `network` beside [`HOST`],
/// sealing the [`SEALED`] mounts.
fn intent(network: &str) -> Value {
    let [mnt, pid, net, ipc, uts] = HOST;
    let mounts: Vec<Value> = SEALED
        .iter()
        .map(|(path, device, inode)| json!({"path": path, "device": device, "inode": inode}))
        .collect();
    json!({
        "plan": PLAN,
        "sources": SOURCES,
        "network": network,
        "host": {"mnt": mnt, "pid": pid, "net": net, "ipc": ipc, "uts": uts},
        "mounts": mounts,
    })
}

/// `body` as one control frame.
fn frame(body: &[u8]) -> Vec<u8> {
    let length = u32::try_from(body.len()).unwrap().to_be_bytes();
    [&length[..], body].concat()
}

/// A fresh anonymous pipe.
fn anonymous() -> (PipeReader, PipeWriter) {
    std::io::pipe().unwrap()
}

/// The arguments naming `control` and `ready`, each given up to the
/// bootstrap.
fn handed(control: impl IntoRawFd, ready: impl IntoRawFd) -> BrokerBootstrapArgs {
    BrokerBootstrapArgs {
        control: Some(control.into_raw_fd().to_string()),
        ready: Some(ready.into_raw_fd().to_string()),
    }
}

/// Run the bootstrap with `control` written and closed before it starts,
/// standing in `observed`: its cause, and every byte its ready pipe
/// carried before it closed.
fn bootstrap(control: &[u8], observed: Observed) -> (Refusal, Vec<u8>) {
    let (control_rx, mut control_tx) = anonymous();
    let (mut ready_rx, ready_tx) = anonymous();
    control_tx.write_all(control).unwrap();
    drop(control_tx);
    let refusal = run_with(&handed(control_rx, ready_tx), |_| observed);
    let mut ready = Vec::new();
    ready_rx.read_to_end(&mut ready).unwrap();
    (refusal, ready)
}

/// The bootstrap's answer to the [`intent`] on `network` in `observed`.
fn answer(network: &str, observed: Observed) -> (Refusal, Vec<u8>) {
    bootstrap(&frame(intent(network).to_string().as_bytes()), observed)
}

/// What a box the bootstrap refuses leaves: the establishment cause and
/// no ready byte.
fn unestablished() -> (Refusal, Vec<u8>) {
    (Refusal::Establishment, Vec::new())
}

/// Check that each labelled box is refused, unestablished, on an
/// isolated network, naming every case that is not.
fn refused_each(cases: impl IntoIterator<Item = (&'static str, Observed)>) {
    let answered = cases
        .into_iter()
        .map(|(case, observed)| (case, answer("isolated", observed)))
        .filter(|(_, answer)| *answer != unestablished());
    assert_eq!(answered.collect::<Vec<_>>(), []);
}

/// The one ready message an admitted box with `net` writes, as parsed.
fn ready_message(net: u64) -> Value {
    json!({
        "plan": PLAN,
        "sources": SOURCES,
        "pid": 2,
        "namespaces": {"mnt": 11, "pid": 12, "net": net, "ipc": 14, "uts": 15},
    })
}

#[test]
fn an_admitted_box_writes_one_closed_ready_message_and_still_refuses_when_released() {
    let (refusal, ready) = answer("isolated", admitted());
    assert_eq!(refusal, Refusal::ServingIncomplete);
    let message: Value = serde_json::from_slice(&ready).unwrap();
    assert_eq!(message, ready_message(13));
    assert!(ready.len() <= FRAME_MAX, "{}", ready.len());
}

#[test]
fn a_ready_message_is_written_only_within_the_bound() {
    let wide = ids([u64::MAX; 5]);
    let ready = |plan| Ready {
        plan,
        sources: SOURCES,
        pid: u32::MAX,
        namespaces: &wide,
    };
    let size = |plan| message(&ready(plan)).map(|bytes| bytes.len());
    assert_eq!(size(PLAN), Ok(319));
    assert_eq!(size(&"a".repeat(FRAME_MAX)), Err(Refusal::Establishment));
}

#[test]
fn the_environment_is_the_one_this_process_was_started_with() {
    let started = observe(&sealing(json!([]))).environment;
    let cargo = OsString::from("CARGO_MANIFEST_DIR");
    assert!(started.contains(&cargo), "{started:?}");
    assert!(!fixed(&started));
    let environ = b"PATH=/usr/bin\0LD_PRELOAD\0HOME=a=b\0\0";
    assert_eq!(
        names(environ),
        ["PATH", "LD_PRELOAD", "HOME"].map(OsString::from)
    );
}

/// An isolated intent sealing `mounts`, as the bootstrap reads it.
fn sealing(mounts: Value) -> Intent {
    let mut intent = intent("isolated");
    intent["mounts"] = mounts;
    serde_json::from_value(intent).unwrap()
}

#[test]
fn a_sealed_source_is_seen_by_its_own_device_and_inode_where_it_stands() {
    let root = tempfile::tempdir().unwrap();
    let link = root.path().join("link");
    std::os::unix::fs::symlink(root.path(), &link).unwrap();
    let own = |path: &Path| {
        let facts = std::fs::symlink_metadata(path).unwrap();
        let device = (
            rustix::fs::major(facts.dev()),
            rustix::fs::minor(facts.dev()),
        );
        Some((device, facts.ino()))
    };
    let unsealed = json!({"device": [0, 0], "inode": 0});
    let at = |path: &Path| {
        let mut source = unsealed.clone();
        source["path"] = json!(path);
        source
    };
    let paths = [link.clone(), root.path().join("absent")];
    let seen = observe(&sealing(json!(paths.each_ref().map(|path| at(path))))).sources;
    assert_eq!(seen, [own(&link), None]);
    assert_ne!(own(&link), own(root.path()));
}

#[test]
fn a_shared_network_is_the_brokers_and_an_isolated_one_is_not() {
    let shared = |net| Observed {
        spaces: Some(ids([11, 12, net, 14, 15])),
        ..admitted()
    };
    let (refusal, ready) = answer("shared", shared(3));
    assert_eq!(refusal, Refusal::ServingIncomplete);
    let message: Value = serde_json::from_slice(&ready).unwrap();
    assert_eq!(message, ready_message(3));
    assert_eq!(answer("shared", shared(13)), unestablished());
    assert_eq!(answer("isolated", shared(3)), unestablished());
}

#[test]
fn each_namespace_the_box_shares_with_the_broker_refuses() {
    let shared = |at: usize| {
        let mut ours = [11, 12, 13, 14, 15];
        ours[at] = HOST[at];
        Some(ids(ours))
    };
    let cases = [
        ("mnt", shared(0)),
        ("pid", shared(1)),
        ("ipc", shared(3)),
        ("uts", shared(4)),
        ("unseen", None),
    ];
    refused_each(cases.map(|(case, spaces)| {
        let observed = Observed {
            spaces,
            ..admitted()
        };
        (case, observed)
    }));
}

#[test]
fn any_environment_beside_the_fixed_names_refuses() {
    let mut names: Vec<OsString> = server_environment().map(OsString::from).collect();
    let cases = [
        ("loader", [&names[..], &["LD_PRELOAD".into()]].concat()),
        ("binding", [&names[..], &["DOCS_TOKEN".into()]].concat()),
        ("twice", [&names[..], &["HOME".into()]].concat()),
        ("missing", names.split_off(1)),
    ];
    refused_each(cases.map(|(case, environment)| {
        let observed = Observed {
            environment,
            ..admitted()
        };
        (case, observed)
    }));
}

#[test]
fn each_private_directory_is_a_distinct_fresh_one_only_its_owner_enters() {
    let changed = |change: fn(&mut Directory)| {
        let mut observed = admitted();
        change(observed.private[1].as_mut().unwrap());
        observed
    };
    let cases = [
        ("open", changed(|dir| dir.mode = 0o040_755)),
        ("sticky", changed(|dir| dir.mode = 0o041_700)),
        ("link", changed(|dir| dir.mode = 0o120_700)),
        ("owner", changed(|dir| dir.uid = 0)),
        ("used", changed(|dir| dir.empty = false)),
        (
            "same",
            changed(|dir| dir.path = PathBuf::from("/runtime/home")),
        ),
    ];
    refused_each(cases);
    let mut unnamed = admitted();
    unnamed.private[0] = None;
    assert_eq!(answer("isolated", unnamed), unestablished());
}

#[test]
fn each_private_directory_is_its_own_writable_tmpfs_with_nothing_below() {
    let home = "28 20 0:44 / /runtime/home rw,nosuid,nodev - tmpfs tmpfs rw";
    let cases = [
        (
            "read-only",
            remounted(8, "28 20 0:44 / /runtime/home ro - tmpfs tmpfs rw"),
        ),
        (
            "bound",
            remounted(8, "28 20 8:1 /x /runtime/home rw - ext4 /dev/sda1 rw"),
        ),
        (
            "absent",
            remounted(8, "28 20 8:1 /opt /opt ro - ext4 /dev/sda1 rw"),
        ),
        ("twice", remounted(7, home)),
        (
            "below",
            remounted(7, "27 29 8:1 /x /tmp/x ro - ext4 /dev/sda1 rw"),
        ),
        (
            "part",
            remounted(8, "28 20 0:44 /x /runtime/home rw - tmpfs tmpfs rw"),
        ),
        (
            "seen",
            remounted(9, "29 20 0:44 / /tmp rw - tmpfs tmpfs rw"),
        ),
        (
            "rooted",
            remounted(9, "29 20 0:40 / /tmp rw - tmpfs tmpfs rw"),
        ),
    ];
    refused_each(cases);
}

#[test]
fn each_source_is_read_only_the_root_bubblewraps_tmpfs_and_proc_and_dev_the_kernels() {
    let cases = [
        (
            "source",
            remounted(6, "26 20 8:1 /usr /usr rw - ext4 /dev/sda1 rw"),
        ),
        (
            "nested",
            remounted(7, "27 26 8:2 / /usr/local rw - ext4 /dev/sda2 rw"),
        ),
        (
            "root",
            remounted(0, "20 1 8:1 /newroot / rw - ext4 /dev/sda1 rw"),
        ),
        (
            "whole root",
            remounted(0, "20 1 0:40 / / rw - tmpfs tmpfs rw"),
        ),
        (
            "root seen",
            remounted(6, "26 20 0:40 /usr /usr ro - tmpfs tmpfs rw"),
        ),
        (
            "proc",
            remounted(2, "22 21 8:1 /x /proc/sys rw - ext4 /dev/sda1 rw"),
        ),
        (
            "dev",
            remounted(4, "24 23 8:1 /x /dev/null rw - ext4 /dev/sda1 rw"),
        ),
        ("unread", remounted(6, "26 20 8:1 /usr /usr")),
        ("short", remounted(6, "26 20 8:1 - ext4 /dev/sda1 rw")),
    ];
    refused_each(cases);
    let kernels = [
        (
            "root part",
            remounted(0, "20 1 0:40 /x / rw - tmpfs tmpfs rw"),
        ),
        (
            "procfs part",
            remounted(1, "21 20 0:41 /1 /proc rw - proc proc rw"),
        ),
        (
            "proc below",
            remounted(2, "22 21 0:41 /sys /proc/sys rw - proc proc rw"),
        ),
        (
            "dev part",
            remounted(3, "23 20 0:42 /x /dev rw - tmpfs tmpfs rw"),
        ),
        (
            "devtmpfs",
            remounted(4, "24 23 0:5 / /dev/null rw - devtmpfs udev rw"),
        ),
        (
            "node",
            remounted(4, "24 23 0:5 /sda /dev/sda rw - devtmpfs udev rw"),
        ),
        (
            "renamed",
            remounted(4, "24 23 0:5 /sda /dev/null rw - devtmpfs udev rw"),
        ),
    ];
    refused_each(kernels);
}

#[test]
fn proc_and_dev_are_the_box_own_and_being_read_only_explains_no_mount() {
    let cases = [
        (
            "ext4 at proc/sys",
            remounted(
                2,
                "22 21 8:1 /sys /proc/sys ro,nosuid,nodev - ext4 /dev/sda1 rw",
            ),
        ),
        (
            "host proc at proc/sys",
            remounted(2, "22 21 0:9 /sys /proc/sys ro,nosuid,nodev - proc proc rw"),
        ),
        (
            "uncovered",
            added("34 21 0:41 /kcore /proc/kcore ro,nosuid,nodev - proc proc rw"),
        ),
        (
            "host devtmpfs at dev",
            remounted(3, "23 20 0:5 / /dev ro,nosuid,nodev - devtmpfs udev rw"),
        ),
        (
            "dev without nodev",
            remounted(3, "23 20 0:42 / /dev rw,nosuid - tmpfs tmpfs rw,mode=755"),
        ),
        (
            "node outside NODES",
            added("34 23 0:5 /sda /dev/sda ro,nosuid,nodev - devtmpfs udev rw"),
        ),
        (
            "node swapped",
            remounted(4, "24 23 0:5 /zero /dev/null rw,nosuid - devtmpfs udev rw"),
        ),
        (
            "terminals elsewhere",
            remounted(
                5,
                "25 23 0:43 / /dev/shm rw,nosuid,noexec - devpts devpts rw",
            ),
        ),
    ];
    refused_each(cases);
}

/// The admitted box with its private `HOME` tmpfs moved to `path`, on
/// parent mount `parent`: still one whole writable tmpfs alone on its
/// device, empty, 0700 and its owner's.
fn rehomed(path: &str, parent: u32) -> Observed {
    let line = format!("28 {parent} 0:44 / {path} rw,nosuid,nodev - tmpfs tmpfs rw");
    let mut observed = remounted(8, &line);
    observed.private[0].as_mut().unwrap().path = PathBuf::from(path);
    observed
}

#[test]
fn a_private_directory_stands_outside_the_box_own_trees_and_apart() {
    let mut whole = remounted(8, "28 21 0:41 /irq /proc/irq ro - proc proc rw");
    let mut lines: Vec<&str> = whole.mounts.lines().collect();
    lines[0] = "20 1 0:40 / / rw,nosuid,nodev,relatime - tmpfs tmpfs rw";
    whole.mounts = lines.join("\n");
    whole.private[0].as_mut().unwrap().path = PathBuf::from("/");
    refused_each([
        ("in dev", rehomed("/dev/unsealed", 23)),
        ("in proc", rehomed("/proc/x", 21)),
        ("at the root", whole),
        ("in a source", rehomed("/usr/x", 26)),
        ("over a source", rehomed("/opt", 20)),
        ("in the other", rehomed("/tmp/x", 29)),
    ]);
}

/// The [`SEALED`] mounts as the intent's sources.
fn sealed() -> Vec<Source> {
    let source = |(path, device, inode): (&str, (u32, u32), u64)| Source {
        path: PathBuf::from(path),
        device,
        inode,
    };
    SEALED.map(source).into()
}

/// `place`'s row, by its name.
fn row(place: Place) -> &'static str {
    match place {
        Place::Private => "private",
        Place::Below => "below",
        Place::Root => "root",
        Place::Procfs => "procfs",
        Place::Proc => "proc",
        Place::Devfs => "devfs",
        Place::Dev => "dev",
        Place::Source => "source",
        Place::Elsewhere => "elsewhere",
    }
}

#[test]
fn proc_dev_and_the_root_are_judged_before_a_private_or_source_exemption() {
    let points = ["/proc", "/proc/x", "/dev", "/dev/x", "/", "/usr", "/tmp"];
    let directories = points.map(directory);
    let private: Vec<&Directory> = directories.iter().collect();
    let sources = points.map(|point| Source {
        path: PathBuf::from(point),
        device: (0, 0),
        inode: 0,
    });
    let rows = points.map(|point| row(place(Path::new(point), &private, &sources)));
    let expected = ["procfs", "proc", "devfs", "dev", "root", "source", "source"];
    assert_eq!(rows, expected);
    let rows = points.map(|point| row(place(Path::new(point), &private, &[])));
    let expected = [
        "procfs", "proc", "devfs", "dev", "root", "private", "private",
    ];
    assert_eq!(rows, expected);
    let below = row(place(Path::new("/tmp/x"), &private, &[]));
    assert_eq!(below, "below");
}

#[test]
fn only_a_path_beyond_the_root_proc_and_dev_is_outside() {
    let paths = [
        "/", "/proc", "/proc/x", "/dev", "/dev/x", "/devices", "/tmp",
    ];
    let outside = paths.map(|path| outside(Path::new(path)));
    assert_eq!(outside, [false, false, false, false, false, true, true]);
}

#[test]
fn a_private_path_is_placed_only_outside_and_apart() {
    let sources = sealed();
    let placed_at = |path: &str| {
        let home = directory(path);
        let tmp = directory("/tmp");
        placed(&[&home, &tmp], &sources)
    };
    let paths = [
        "/runtime/home",
        "/dev/unsealed",
        "/proc/x",
        "/",
        "/usr/x",
        "/opt",
        "/tmp/x",
    ];
    let placed = paths.map(placed_at);
    assert_eq!(placed, [true, false, false, false, false, false, false]);
}

#[test]
fn every_mount_meets_its_own_row_of_the_closed_table() {
    refused_each([
        (
            "root not tmpfs",
            remounted(0, "20 1 8:9 /newroot / rw,nosuid - ext4 /dev/sdc rw"),
        ),
        (
            "proc not proc",
            remounted(1, "21 20 0:41 / /proc rw,nosuid - tmpfs tmpfs rw"),
        ),
        (
            "kernel fstype",
            remounted(5, "25 23 0:43 / /dev/pts rw,nosuid - tmpfs tmpfs rw"),
        ),
        (
            "second dev",
            added("34 20 0:47 / /dev rw,nosuid,nodev - tmpfs tmpfs rw"),
        ),
    ]);
    let mut moved = intent("isolated");
    moved["mounts"][2]["path"] = json!("/proc/irq");
    let observed = remounted(10, "30 21 0:41 /irq /proc/irq ro - proc proc rw");
    let answer = bootstrap(&frame(moved.to_string().as_bytes()), observed);
    assert_eq!(answer, unestablished());
}

#[test]
fn each_sealed_source_and_nothing_else_stands_where_the_intent_puts_it() {
    let seen = |change: fn(&mut Vec<Option<Identity>>)| {
        let mut observed = admitted();
        change(&mut observed.sources);
        observed
    };
    let cases = [
        ("inode", seen(|seen| seen[0] = Some(((8, 1), 101)))),
        ("device", seen(|seen| seen[1] = Some(((8, 9), 2)))),
        ("unseen", seen(|seen| seen[2] = None)),
        ("short", seen(|seen| seen.truncate(2))),
        (
            "missing",
            remounted(7, "27 21 0:41 /irq /proc/irq ro - proc proc rw"),
        ),
        (
            "stacked",
            added("31 26 8:1 /usr /usr ro - ext4 /dev/sda1 rw"),
        ),
        (
            "unexpected",
            added("31 20 8:3 / /srv ro - ext4 /dev/sdb1 rw"),
        ),
    ];
    refused_each(cases);
}

#[test]
fn the_intent_is_one_closed_frame_of_at_most_four_kibibytes() {
    let body = intent("isolated").to_string();
    let padded = |size: usize| format!("{body:<size$}").into_bytes();
    let (refusal, ready) = bootstrap(&frame(&padded(FRAME_MAX)), admitted());
    assert_eq!(refusal, Refusal::ServingIncomplete);
    assert!(!ready.is_empty());
    let mut unknown = intent("isolated");
    unknown["store"] = json!("/x");
    let mut unsealed = intent("isolated");
    unsealed["sources"] = json!("ab");
    let whole = frame(body.as_bytes());
    let cases = [
        ("over", frame(&padded(FRAME_MAX + 1))),
        ("unknown", frame(unknown.to_string().as_bytes())),
        ("unsealed", frame(unsealed.to_string().as_bytes())),
        ("network", frame(intent("open").to_string().as_bytes())),
        ("length", whole[..2].to_vec()),
        ("truncated", whole[..whole.len() - 1].to_vec()),
    ];
    for (case, control) in cases {
        assert_eq!(
            (case, bootstrap(&control, admitted())),
            (case, unestablished())
        );
    }
}

#[test]
fn a_byte_after_the_frame_is_no_release() {
    let trailing = [frame(intent("isolated").to_string().as_bytes()), vec![b'x']].concat();
    let (refusal, ready) = bootstrap(&trailing, admitted());
    assert_eq!(refusal, Refusal::Establishment);
    let message: Value = serde_json::from_slice(&ready).unwrap();
    assert_eq!(message, ready_message(13));
}

#[test]
fn readiness_leaves_on_a_distinct_private_pipe_and_no_other_descriptor() {
    let (control_rx, _control_tx) = anonymous();
    let (ready_rx, ready_tx) = anonymous();
    let link = link(ready_tx.as_raw_fd()).unwrap();
    let args = handed(control_rx, ready_tx);
    let numbers = (args.control.as_deref(), args.ready.as_deref());
    let stdout = descriptors(numbers, &[link]).err();
    assert_eq!(stdout, Some(Refusal::Establishment));
    drop(ready_rx);

    let (control_rx, control_tx) = anonymous();
    let same = descriptors((Some(&rx(control_rx)), Some(&tx(control_tx))), &[]).err();
    assert_eq!(same, Some(Refusal::Establishment));

    let marker = tempfile::tempfile().unwrap();
    let (control_rx, mut control_tx) = anonymous();
    control_tx
        .write_all(&frame(intent("isolated").to_string().as_bytes()))
        .unwrap();
    drop(control_tx);
    let marked = handed(control_rx, marker.try_clone().unwrap());
    assert_eq!(run_with(&marked, |_| admitted()), Refusal::Establishment);
    assert_eq!(marker.metadata().unwrap().len(), 0);
}

/// `pipe`'s read end given up, by number.
fn rx(pipe: PipeReader) -> String {
    pipe.into_raw_fd().to_string()
}

/// `pipe`'s write end given up, by number.
fn tx(pipe: PipeWriter) -> String {
    pipe.into_raw_fd().to_string()
}

#[test]
fn absent_or_unusable_control_context_refuses() {
    let (control_rx, _control_tx) = anonymous();
    let control = rx(control_rx);
    for ready in [None, Some("x"), Some("2"), Some("-1"), Some("999999")] {
        let refused = descriptors((Some(&control), ready), &[]).err();
        assert_eq!((ready, refused), (ready, Some(Refusal::Establishment)));
    }
    let refused = descriptors((None, Some(&control)), &[]).err();
    assert_eq!(refused, Some(Refusal::Establishment));

    let (control_rx, mut control_tx) = anonymous();
    let (ready_rx, _ready_tx) = anonymous();
    control_tx
        .write_all(&frame(intent("isolated").to_string().as_bytes()))
        .unwrap();
    drop(control_tx);
    let backwards = handed(control_rx, ready_rx);
    assert_eq!(run_with(&backwards, |_| admitted()), Refusal::Establishment);
}

#[test]
fn a_host_is_no_box() {
    let host = spaces().map(|host| [host.mnt, host.pid, host.net, host.ipc, host.uts]);
    let [mnt, pid, net, ipc, uts] = host.unwrap_or(HOST);
    let mut ours = intent("shared");
    ours["host"] = json!({"mnt": mnt, "pid": pid, "net": net, "ipc": ipc, "uts": uts});
    let (control_rx, mut control_tx) = anonymous();
    let (mut ready_rx, ready_tx) = anonymous();
    control_tx
        .write_all(&frame(ours.to_string().as_bytes()))
        .unwrap();
    drop(control_tx);
    assert_eq!(run(&handed(control_rx, ready_tx)), Refusal::Establishment);
    let mut ready = Vec::new();
    ready_rx.read_to_end(&mut ready).unwrap();
    assert_eq!(ready, Vec::<u8>::new());
}
