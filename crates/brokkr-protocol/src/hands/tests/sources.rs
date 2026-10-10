//! The server box's source observer (decision 0065 slice two, U6c5a; MB3,
//! MB4, SC1): every cause it refuses with, each bound at and over its
//! limit, the kernel write-exclusion proof condition by condition, mount
//! aliases read from the table, and the handle the box mounts.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use super::super::namespace::sources::host::*;
use super::super::namespace::sources::*;
use super::super::*;
use super::launcher::{bubblewrap, launcher};
use super::server::{build_dir, installed, server_system, unservable, Host as Fixture};
use crate::broker::{Network, Reach, Refusal, Tree};

pub(super) fn table() -> Vec<u8> {
    std::fs::read("/proc/self/mountinfo").unwrap()
}

/// A mount table that reads as `text`.
pub(super) fn serving(text: &[u8]) -> Option<Box<dyn Read>> {
    Some(Box::new(std::io::Cursor::new(text.to_vec())))
}

/// This process's mount table as it reads now.
fn live() -> Option<Box<dyn Read>> {
    serving(&table())
}

/// `text` parsed as a mount table of at most `mounts` records.
fn parse(text: &[u8], mounts: usize) -> Result<Vec<Record>, Refusal> {
    records(text, mounts)
}

/// Whether a run can make a file it owns unreadable to itself: euid 0
/// reads every file, so there the proof is skipped, and declared.
fn unprivileged() -> bool {
    let root = rustix::process::geteuid().is_root();
    if root {
        let reason = "euid 0 reads every file, so no file can be made unreadable";
        skip_boundary_proof(boundary_evidence_required(), reason);
    }
    !root
}

fn nothing() {}

fn untouched(_: &Path, _: &OsStr) {}

/// A uid that is neither this process's nor the overflow uid.
fn stranger() -> u32 {
    let euid = rustix::process::geteuid().as_raw();
    [4242, 4243].into_iter().find(|uid| *uid != euid).unwrap()
}

/// Credentials of one confined writer that owns nothing here.
pub(super) fn strangers() -> Credentials {
    Credentials {
        uids: vec![stranger()],
        overflow: 65534,
        confinement: Confinement::Proved,
    }
}

/// Credentials of this process alone, confined.
pub(super) fn ours() -> Credentials {
    Credentials {
        uids: vec![rustix::process::geteuid().as_raw()],
        ..strangers()
    }
}

/// This host with its writers confined: this process's own uids and
/// capabilities, with `no_new_privs` set as a confined managed writer
/// holds it, so the host's multiply-linked system files can pass. A test
/// process need not hold it itself, and setting it would be irreversible
/// for every test that shares the process.
pub(super) fn confined() -> Host<'static> {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let fenced: String = status
        .lines()
        .map(|line| match line.starts_with("NoNewPrivs:") {
            true => "NoNewPrivs:\t1",
            false => line,
        })
        .flat_map(|line| [line, "\n"])
        .collect();
    let overflow = std::fs::read_to_string("/proc/sys/fs/overflowuid").unwrap();
    let credentials = credited(Some(&overflow), &[Some(&fenced)]);
    assert_eq!(credentials.confinement, Confinement::Proved);
    Host {
        credentials,
        ..Host::live()
    }
}

/// A host for the observer: `credentials`, the live mount table and
/// nothing between the resolutions, unless a test gives its own.
pub(super) fn host<'h>(credentials: Credentials, mountinfo: Table<'h>) -> Host<'h> {
    sources_host(credentials, mountinfo, &nothing)
}

/// What a host reads its mount table from.
pub(super) type Table<'h> = &'h dyn Fn() -> Option<Box<dyn Read>>;

fn sources_host<'h>(
    credentials: Credentials,
    mountinfo: Table<'h>,
    between: &'h dyn Fn(),
) -> Host<'h> {
    Host {
        credentials,
        mountinfo,
        found: &|| None,
        between,
        opening: &untouched,
        ..Host::live()
    }
}

pub(super) fn source(path: &Path, role: Role) -> Source<'_> {
    Source {
        path,
        role,
        presence: Presence::Required,
    }
}

pub(super) fn reach(writable: &[PathBuf], readable: &[PathBuf]) -> Reach {
    Reach {
        writable: writable.to_vec(),
        readable: readable.to_vec(),
    }
}

/// What observing `sources` against `seat` on `host` answers.
fn answer(sources: &[Source<'_>], seat: &Reach, host: &Host<'_>) -> Result<(), Refusal> {
    observe(sources, seat, host).map(|_| ())
}

#[test]
fn the_write_exclusion_proof_needs_every_condition() {
    let file = Owned {
        uid: 0,
        mode: 0o100755,
        acl: Acl::Absent,
    };
    let writers = Credentials {
        uids: vec![1000],
        overflow: 65534,
        confinement: Confinement::Proved,
    };
    assert!(excluded(&file, &writers));
    let unproved = |file: Owned, writers: Credentials| !excluded(&file, &writers);
    // The owner is a writer; group or other may write; an access ACL is
    // there or cannot be read.
    assert!(unproved(Owned { uid: 1000, ..file }, writers.clone()));
    assert!(unproved(
        Owned {
            mode: 0o100775,
            ..file
        },
        writers.clone()
    ));
    assert!(unproved(
        Owned {
            mode: 0o100757,
            ..file
        },
        writers.clone()
    ));
    assert!(unproved(
        Owned {
            acl: Acl::Present,
            ..file
        },
        writers.clone()
    ));
    assert!(unproved(
        Owned {
            acl: Acl::Unknown,
            ..file
        },
        writers.clone()
    ));
    // The owner or a writer reads as the overflow uid, or no writer is
    // known at all; the writers' privilege is not proved confined.
    assert!(unproved(Owned { uid: 65534, ..file }, writers.clone()));
    let unmapped = Credentials {
        uids: vec![1000, 65534],
        ..writers.clone()
    };
    assert!(unproved(file, unmapped));
    let none = Credentials {
        uids: Vec::new(),
        ..writers.clone()
    };
    assert!(unproved(file, none));
    let privileged = Credentials {
        confinement: Confinement::Unproved,
        ..writers
    };
    assert!(unproved(file, privileged));
    // A descriptor whose filesystem keeps no ACL proves none absent.
    let (pipe, _writer) = std::io::pipe().unwrap();
    assert_eq!(acl(pipe.as_fd()), Acl::Unknown);
}

#[test]
fn the_writers_capabilities_are_each_set_read_once() {
    let status = "Name:\tx\nCapInh:\t0000000000000000\nCapPrm:\t0000000000000001\n\
                  CapEff:\t0000000000200000\nCapBnd:\t000001ffffffffff\nCapAmb:\t0000000000000000\n";
    assert_eq!(capabilities(status), Some(1 | 1 << 21));
    let twice = format!("{status}CapEff:\t0000000000000000\n");
    assert_eq!(capabilities(&twice), None);
    assert_eq!(capabilities(&status.replace("CapAmb", "CapXyz")), None);
    assert_eq!(
        capabilities(&status.replace("0000000000200000", "zz")),
        None
    );
}

/// A confined writer's status: two distinct uids, none root, no
/// capability, and `no_new_privs` set.
const CONFINED: &str =
    "Uid:\t1000\t1001\t1000\t1000\nCapPrm:\t0\nCapEff:\t0\nCapAmb:\t0\nNoNewPrivs:\t1\n";

#[test]
fn a_writer_is_confined_only_where_no_exec_or_uid_can_gain_privilege() {
    // Every uid the writer holds is one it may act as.
    let proved = Credentials {
        uids: vec![1000, 1001],
        overflow: 65534,
        confinement: Confinement::Proved,
    };
    assert_eq!(credited(Some("65534\n"), &[Some(CONFINED)]), proved);
    // A root uid beside the others, an ownership capability, `no_new_privs` clear,
    // unread or read twice, no uid line, and no status at all: each alone
    // leaves the writer unconfined.
    let unconfined = [
        CONFINED.replace("1001", "0"),
        CONFINED.replace("CapEff:\t0", "CapEff:\t200000"),
        CONFINED.replace("NoNewPrivs:\t1", "NoNewPrivs:\t0"),
        CONFINED.replace("NoNewPrivs:\t1\n", ""),
        format!("{CONFINED}NoNewPrivs:\t1\n"),
        CONFINED.replace("Uid:", "Gid:"),
    ];
    for status in unconfined {
        let confinement = credited(Some("65534\n"), &[Some(&status)]).confinement;
        assert_eq!((&status, confinement), (&status, Confinement::Unproved));
    }
    let unread = credited(Some("65534\n"), &[None]);
    assert_eq!(unread.confinement, Confinement::Unproved);
    // An unread or unterminated overflow uid, or a uid line of other than
    // four uids, leaves no writer known at all.
    let three = CONFINED.replace("\t1001", "");
    for (overflow, status) in [(None, CONFINED), (Some("65534"), CONFINED)]
        .into_iter()
        .chain([(Some("65534\n"), three.as_str())])
    {
        let unknown = credited(overflow, &[Some(status)]);
        let facts = (unknown.uids, unknown.overflow);
        assert_eq!((status, facts), (status, (Vec::new(), u32::MAX)));
    }
}

#[test]
fn a_writer_holding_any_capability_is_unconfined_whatever_no_new_privs_says() {
    // `no_new_privs` removes no capability already held: a module, raw
    // I/O, another process's memory, a device node, BPF or ownership each
    // leaves the writer unconfined, in any of the three sets.
    let held = [
        ("CAP_SYS_MODULE", 1u64 << 16),
        ("CAP_SYS_RAWIO", 1 << 17),
        ("CAP_SYS_PTRACE", 1 << 19),
        ("CAP_MKNOD", 1 << 27),
        ("CAP_BPF", 1 << 39),
        ("CAP_CHOWN", 1 << 0),
    ];
    for (name, bit) in held {
        for set in ["CapPrm", "CapEff", "CapAmb"] {
            let status = CONFINED.replace(&format!("{set}:\t0\n"), &format!("{set}:\t{bit:x}\n"));
            assert_ne!(status, CONFINED);
            let confinement = credited(Some("65534\n"), &[Some(&status)]).confinement;
            assert_eq!((name, set, confinement), (name, set, Confinement::Unproved));
        }
    }
}

const LINE: &str =
    "36 35 98:0 /mnt1 /mnt/parent rw,noatime master:1 - ext3 /dev/root rw,errors=continue";

#[test]
fn the_mount_table_is_read_strictly_and_within_its_bound() {
    let text = format!(
        "{LINE}\n37 35 0:4 mnt:[4026532867] /run/snapd/ns/x.mnt rw - nsfs nsfs rw\n\
         38 36 98:0 /a\\040b /mnt/w\\134x ro shared:2 master:3 - overlay o rw\n"
    );
    let parsed = parse(text.as_bytes(), 3).unwrap();
    assert_eq!(
        parsed[0],
        Record {
            id: 36,
            dev: (98, 0),
            root: PathBuf::from("/mnt1"),
            point: PathBuf::from("/mnt/parent"),
            writable: true,
            local: true,
        }
    );
    assert!(!parsed[1].local);
    let escaped = &parsed[2];
    let named = (&escaped.root, &escaped.point, escaped.writable);
    assert_eq!(
        named,
        (&PathBuf::from("/a b"), &PathBuf::from("/mnt/w\\x"), false)
    );
    assert!(!escaped.local);
    // The bound: three records at three, not at two.
    assert_eq!(parse(text.as_bytes(), 2), Err(Refusal::Identity));
    // A table with no final newline, nothing, a missing separator, a short
    // tail, a bad device, an unknown option, a bad escape, a relative point
    // or a local relative root, and an id twice.
    let malformed = [
        LINE.to_string(),
        String::new(),
        LINE.replace(" - ", " "),
        LINE.replace(" rw,errors=continue", ""),
        LINE.replace("98:0", "98-0"),
        LINE.replace("rw,noatime", "noatime"),
        LINE.replace("/mnt1", "/mnt\\9"),
        LINE.replace("/mnt/parent", "mnt/parent"),
        LINE.replace("/mnt1", "mnt1"),
        format!("{LINE}\n{LINE}"),
    ];
    for text in malformed {
        let answer = parse(format!("{text}\n").as_bytes(), 8).map(|_| ());
        let answer = match text.is_empty() | (text == LINE) {
            true => parse(text.as_bytes(), 8).map(|_| ()),
            false => answer,
        };
        assert_eq!((&text, answer), (&text, Err(Refusal::Identity)));
    }
}

/// A local record of device `dev` with `root` at `point`.
fn mounted(id: u64, dev: (u32, u32), root: &str, point: &str, writable: bool) -> Record {
    Record {
        id,
        dev,
        root: PathBuf::from(root),
        point: PathBuf::from(point),
        writable,
        local: true,
    }
}

#[test]
fn an_alias_is_every_other_place_an_object_is_mounted() {
    let rw = true;
    let records = [
        mounted(1, (8, 1), "/", "/", rw),
        // The device's `/opt` again, under the workspace.
        mounted(2, (8, 1), "/opt", "/work/opt", rw),
        // Part of the package elsewhere.
        mounted(3, (8, 1), "/opt/docs/lib", "/cache/lib", rw),
        // Another device's `/opt`, and an unrelated subtree.
        mounted(4, (8, 2), "/opt", "/srv/opt", rw),
        mounted(5, (8, 1), "/var", "/work/var", rw),
    ];
    let found = |inside: &str| -> Vec<(PathBuf, PathBuf)> {
        let found = aliases(&records, 1, (8, 1), Path::new(inside));
        found
            .map(|(point, rest)| (point.into(), rest.into()))
            .collect()
    };
    let pair = |point: &str, rest: &str| (PathBuf::from(point), PathBuf::from(rest));
    assert_eq!(
        found("/opt/docs"),
        [pair("/work/opt", "docs"), pair("/cache/lib", "")]
    );
    // The object itself is the root of a mount.
    assert_eq!(
        found("/opt"),
        [pair("/work/opt", ""), pair("/cache/lib", "")]
    );
}

#[test]
fn every_mount_under_a_source_is_proved_against_the_table() {
    let (ro, rw) = (false, true);
    let base = vec![
        mounted(1, (8, 1), "/", "/", rw),
        mounted(2, (0, 40), "/", "/opt/docs/data", ro),
    ];
    let seen = |dev: (u32, u32)| -> Seen { [((1, (8, 1)), false), ((2, dev), true)].into() };
    let placed = [(PathBuf::from("/opt/docs"), 1)];
    let reach = [PathBuf::from("/work")];
    let check = |records: &[Record], nested| mapped(records, &seen(nested), &placed, &reach);
    assert!(check(&base, (0, 40)));
    // A writable nested mount, one of another device than the walk saw, one
    // the table does not hold, and one on an unproved filesystem.
    let writable = [
        base[0].clone(),
        mounted(2, (0, 40), "/", "/opt/docs/data", rw),
    ];
    assert!(!check(&writable, (0, 40)));
    assert!(!check(&base, (0, 41)));
    assert!(!check(&base[..1], (0, 40)));
    // One mount that shows a second device beneath it, as a subvolume does.
    let mut twice = seen((0, 40));
    twice.insert((1, (8, 9)), false);
    assert!(!mapped(&base, &twice, &placed, &reach));
    let mut unproved = base.clone();
    unproved[1].local = false;
    assert!(!check(&unproved, (0, 40)));
    // The nested mount's filesystem, or the source's, mounted in reach too.
    let mut aliased = base.clone();
    aliased.push(mounted(3, (0, 40), "/", "/work/data", ro));
    assert!(!check(&aliased, (0, 40)));
    let mut aliased = base.clone();
    aliased.push(mounted(3, (8, 1), "/opt", "/work/opt", ro));
    assert!(!check(&aliased, (0, 40)));
    // A source root its mount's point does not hold.
    let elsewhere = [(PathBuf::from("/opt/docs"), 2)];
    assert!(!mapped(&base, &seen((0, 40)), &elsewhere, &reach));
}

#[test]
fn a_nested_mount_that_also_holds_a_source_root_is_proved_as_both() {
    // `/usr/local` is a source and a package below its nested mount at
    // `/usr/local/lib` is another, so that mount holds a root and lies
    // nested beneath one.
    let (ro, rw) = (false, true);
    let base = vec![
        mounted(1, (8, 1), "/", "/", rw),
        mounted(2, (8, 2), "/lib", "/usr/local/lib", ro),
    ];
    let seen: Seen = [((1, (8, 1)), false), ((2, (8, 2)), true)].into();
    let placed = [
        (PathBuf::from("/usr/local"), 1),
        (PathBuf::from("/usr/local/lib/pkg"), 2),
    ];
    let reach = [PathBuf::from("/work")];
    assert!(mapped(&base, &seen, &placed, &reach));
    // A sibling of the package, aliased into reach: the package root's
    // own proof does not see it, the nested mount's whole proof does.
    let mut sibling = base.clone();
    sibling.push(mounted(3, (8, 2), "/lib/other", "/work/other", rw));
    assert!(!mapped(&sibling, &seen, &placed, &reach));
    // The nested mount writable.
    let writable = [
        base[0].clone(),
        mounted(2, (8, 2), "/lib", "/usr/local/lib", rw),
    ];
    assert!(!mapped(&writable, &seen, &placed, &reach));
}

/// Set for the copy of this test binary a mount namespace runs: the
/// fixture root it observes from inside.
const VIEWS: &str = "BROKKR_TEST_MOUNT_VIEWS";

#[test]
fn every_mount_view_is_walked_and_every_nested_mount_proved() {
    if let Some(root) = std::env::var_os(VIEWS) {
        return views(Path::new(&root));
    }
    let required = boundary_evidence_required();
    if !super::can_create_namespace() {
        return skip_boundary_proof(required, "no namespace can be built here");
    }
    let fixture = Fixture::new();
    fixture.file("lib/sub/kept", 0o644);
    fixture.file("pkgs/pkg/a", 0o644);
    fixture.file("pkgs/other/b", 0o644);
    for dir in ["view", "local/lib", "work/other"] {
        std::fs::create_dir_all(fixture.path(dir)).unwrap();
    }
    // `view` is `lib` again with a tmpfs over its `sub` that `lib` lacks;
    // `local/lib` is `pkgs` read-only, and `pkgs/other` is in reach too.
    let mut command = Command::new(require_bwrap().unwrap());
    command.args(["--dev-bind", "/", "/"]);
    for (flag, from, to) in [
        ("--bind", "lib", "view"),
        ("--tmpfs", "", "view/sub"),
        ("--ro-bind", "pkgs", "local/lib"),
        ("--bind", "pkgs/other", "work/other"),
    ] {
        let from = (!from.is_empty()).then(|| fixture.path(from));
        command.arg(flag).args(from).arg(fixture.path(to));
    }
    let name = "hands::tests::sources::every_mount_view_is_walked_and_every_nested_mount_proved";
    rerun(command, name, VIEWS, &fixture);
}

/// Run this binary's test `name` again in the box `command` builds,
/// `variable` naming the fixture's root there: it must pass, and leave
/// `observed` in that root to show its body ran.
fn rerun(mut command: Command, name: &str, variable: &str, fixture: &Fixture) {
    let exe = std::env::current_exe().unwrap();
    command.arg("--").arg(exe).args(["--exact", name]);
    let ran = command.env(variable, &fixture.root).output().unwrap();
    let out = String::from_utf8_lossy(&ran.stdout);
    assert_eq!(ran.status.code(), Some(0), "{out}");
    assert!(fixture.path("observed").exists(), "{out}");
}

/// Inside the namespace: each view and each nested mount, observed.
fn views(root: &Path) {
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let path = |relative: &str| root.join(relative);
    let (lib, view) = (path("lib"), path("view"));
    let seat = reach(&[], &[]);
    let program = |paths: &[&PathBuf]| -> Vec<(PathBuf, Role)> {
        paths
            .iter()
            .map(|path| ((*path).clone(), Role::Program))
            .collect()
    };
    let at = |sources: Vec<(PathBuf, Role)>, seat: &Reach| {
        let sources: Vec<Source<'_>> = sources
            .iter()
            .map(|(path, role)| source(path, *role))
            .collect();
        (sources.len(), answer(&sources, seat, &observing))
    };
    // One directory through two mounts, the writable tmpfs only beneath
    // the later: that view is walked too, though its object was.
    assert_eq!(at(program(&[&lib]), &seat), (1, Ok(())));
    assert_eq!(at(program(&[&view]), &seat), (1, Err(Refusal::Identity)));
    let both = at(program(&[&lib, &view]), &seat);
    assert_eq!(both, (2, Err(Refusal::Identity)));
    // A package root on a read-only mount nested in a support source,
    // whose sibling is mounted in reach: the package's own alias proof
    // passes alone; nested beneath the source, the whole mount is proved.
    let work = reach(&[path("work")], &[]);
    let package = (path("local/lib/pkg"), Role::Program);
    assert_eq!(at(vec![package.clone()], &work), (1, Ok(())));
    let local = (path("local"), Role::Support);
    let nested = at(vec![local, package], &work);
    assert_eq!(nested, (2, Err(Refusal::Identity)));
    std::fs::write(path("observed"), "").unwrap();
}

/// A package at `opt/docs` with its entry and one module, `lib/a.py`: the
/// fixture, the package root and the module.
fn package() -> (Fixture, PathBuf, PathBuf) {
    let fixture = Fixture::new();
    fixture.plant("opt/docs/bin/docs-mcp");
    let module = fixture.file("opt/docs/lib/a.py", 0o644);
    let docs = fixture.path("opt/docs");
    (fixture, docs, module)
}

#[test]
fn a_program_or_bootstrap_file_with_a_second_link_refuses_whoever_owns_it() {
    let (fixture, docs, module) = package();
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let seat = reach(&[], &[]);
    // Directory link counts never decide: the tree's directories have many.
    let tree = [source(&docs, Role::Program)];
    assert!(std::fs::metadata(&docs).unwrap().nlink() > 2);
    assert_eq!(answer(&tree, &seat, &observing), Ok(()));
    // A second link to a module, with credentials under which a support
    // file would pass: a program file still refuses.
    std::fs::hard_link(&module, fixture.path("opt/docs/lib/b.py")).unwrap();
    assert_eq!(answer(&tree, &seat, &observing), Err(Refusal::Linked));
    let support = [source(&docs, Role::Support)];
    assert_eq!(answer(&support, &seat, &observing), Ok(()));
    // A bootstrap, generated or launch file linked from outside its tree.
    let boot = fixture.plant("boot/brokkr");
    std::fs::hard_link(&boot, fixture.path("elsewhere")).unwrap();
    for role in [Role::Program, Role::Generated, Role::Launch] {
        let one = [source(&boot, role)];
        assert_eq!(
            (role, answer(&one, &seat, &observing)),
            (role, Err(Refusal::Linked))
        );
    }
}

#[test]
fn a_bootstrap_that_cannot_be_read_keeps_its_link_and_reach_causes() {
    if !unprivileged() {
        return;
    }
    let fixture = Fixture::new();
    let boot = fixture.plant("boot/brokkr");
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let (seat, held) = (reach(&[], &[]), reach(&[], &[fixture.path("boot")]));
    let one = [source(&boot, Role::Program)];
    // Singly linked and unreadable, it leaves identity unprotected.
    chmod(&boot, 0o111);
    assert!(std::fs::File::open(&boot).is_err());
    assert_eq!(answer(&one, &seat, &observing), Err(Refusal::Identity));
    // Linked, it is the link's cause however its mode reads; in reach,
    // reach's.
    std::fs::hard_link(&boot, fixture.path("elsewhere")).unwrap();
    for mode in [0o755, 0o111] {
        chmod(&boot, mode);
        let answers = (
            answer(&one, &seat, &observing),
            answer(&one, &held, &observing),
        );
        let expected = (Err(Refusal::Linked), Err(Refusal::BindOverlapsReach));
        assert_eq!((mode, answers), (mode, expected));
    }
}

#[test]
fn a_file_inside_a_tree_that_cannot_be_read_refuses_as_the_same_file_as_a_source_does() {
    if !unprivileged() {
        return;
    }
    let (fixture, tree, module) = package();
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let seat = reach(&[], &[]);
    let roles = |path: &Path| {
        let observed = |role| answer(&[source(path, role)], &seat, &observing);
        (observed(Role::Program), observed(Role::Support))
    };
    assert_eq!(roles(&tree), (Ok(()), Ok(())));
    // Singly linked and unreadable, in a program tree or a support one,
    // inside it or as the source itself: identity is not protected.
    chmod(&module, 0o200);
    assert!(std::fs::File::open(&module).is_err());
    let unread = || (Err(Refusal::Identity), Err(Refusal::Identity));
    assert_eq!((roles(&tree), roles(&module)), (unread(), unread()));
    // Linked too: a program file's link cause comes first whether it
    // opens or not, and an unreadable support file must still be root's.
    std::fs::hard_link(&module, fixture.path("opt/a.py")).unwrap();
    assert_eq!(roles(&tree), (Err(Refusal::Linked), Err(Refusal::Identity)));
    chmod(&module, 0o644);
    assert_eq!(roles(&tree), (Err(Refusal::Linked), Ok(())));
}

/// Set for the copy of this test binary a user namespace runs as root
/// holding no capability: the fixture root it observes from inside.
const ROOTED: &str = "BROKKR_TEST_ROOT_ONLY";

#[test]
fn an_unreadable_file_passes_only_as_support_root_owns_and_no_writer_can_write() {
    if let Some(root) = std::env::var_os(ROOTED) {
        return rooted(Path::new(&root));
    }
    let required = boundary_evidence_required();
    if !super::can_create_namespace() {
        return skip_boundary_proof(required, "no namespace can be built here");
    }
    // Each file unreadable to its owner: support files with no write bit,
    // a group or an other write bit, or a writer's ACL entry; a module of
    // a program tree; a bootstrap.
    let fixture = Fixture::new();
    for (relative, mode) in [
        ("sys/sealed/f", 0o000),
        ("sys/group/f", 0o020),
        ("sys/other/f", 0o002),
        ("sys/granted/f", 0o000),
        ("opt/docs/lib/a.py", 0o000),
        ("boot/brokkr", 0o000),
    ] {
        let file = fixture.file(relative, 0o644);
        if relative.starts_with("sys/granted") {
            grant(&file, (6, 4));
        }
        chmod(&file, mode);
    }
    // Inside, every file this process made reads as root's.
    let mut command = Command::new(require_bwrap().unwrap());
    command.args([
        "--unshare-user",
        "--uid",
        "0",
        "--gid",
        "0",
        "--cap-drop",
        "ALL",
    ]);
    command.args(["--dev-bind", "/", "/"]);
    let name = "hands::tests::sources::an_unreadable_file_passes_only_as_support_root_owns_and_no_writer_can_write";
    rerun(command, name, ROOTED, &fixture);
}

/// Inside the user namespace: root owns every fixture file and, holding no
/// capability, reads none of them; each is observed in its role.
fn rooted(root: &Path) {
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let seat = reach(&[], &[]);
    let path = |relative: &str| root.join(relative);
    let sealed = path("sys/sealed/f");
    let denied = std::fs::File::open(&sealed).map_err(|error| error.kind());
    let owner = std::fs::metadata(&sealed).unwrap().uid();
    let refused = Some(std::io::ErrorKind::PermissionDenied);
    assert_eq!((owner, denied.err()), (0, refused));
    const IDENTITY: Result<(), Refusal> = Err(Refusal::Identity);
    for (relative, role, expected) in [
        // Root's, with no write bit and no ACL: as a source and within one.
        ("sys/sealed/f", Role::Support, Ok(())),
        ("sys/sealed", Role::Support, Ok(())),
        ("opt/docs", Role::Support, Ok(())),
        // A group or other write bit, or a writer's ACL entry.
        ("sys/group", Role::Support, IDENTITY),
        ("sys/other", Role::Support, IDENTITY),
        ("sys/granted", Role::Support, IDENTITY),
        // A program tree's file or a bootstrap must be read.
        ("opt/docs", Role::Program, IDENTITY),
        ("boot/brokkr", Role::Program, IDENTITY),
    ] {
        let answer = answer(&[source(&path(relative), role)], &seat, &observing);
        assert_eq!((relative, role, answer), (relative, role, expected));
    }
    std::fs::write(path("observed"), "").unwrap();
}

#[test]
fn a_file_replaced_before_it_opens_is_never_opened_to_be_read() {
    // The file the listing observed is replaced by a FIFO before the
    // observation opens it: its path handle shows another object, so
    // nothing, the FIFO least of all, is ever opened to be read.
    let fixture = Fixture::new();
    let tree = fixture.path("opt/docs");
    let file = fixture.file("opt/docs/a.py", 0o644);
    let swapped = std::cell::Cell::new(false);
    let swap = |dir: &Path, opened: &OsStr| {
        if dir.join(opened) == file && !swapped.replace(true) {
            std::fs::remove_file(&file).unwrap();
            let mode = rustix::fs::Mode::from_raw_mode(0o600);
            let fifo = rustix::fs::FileType::Fifo;
            rustix::fs::mknodat(rustix::fs::CWD, &file, fifo, mode, 0).unwrap();
        }
    };
    let mountinfo = live;
    let opening = Host {
        opening: &swap,
        ..host(strangers(), &mountinfo)
    };
    use rustix::fs::inotify;
    let flags = inotify::CreateFlags::NONBLOCK | inotify::CreateFlags::CLOEXEC;
    let watch = inotify::init(flags).unwrap();
    inotify::add_watch(&watch, &tree, inotify::WatchFlags::OPEN).unwrap();
    let answered = answer(&[source(&tree, Role::Program)], &reach(&[], &[]), &opening);
    assert_eq!((answered, swapped.get()), (Err(Refusal::Identity), true));
    let mut buffer = [std::mem::MaybeUninit::uninit(); 4096];
    let mut events = inotify::Reader::new(&watch, &mut buffer);
    let mut opened = Vec::new();
    while let Ok(event) = events.next() {
        opened.extend(event.file_name().map(|name| name.to_owned()));
    }
    assert_eq!(opened, Vec::<std::ffi::CString>::new());
}

#[test]
fn an_object_inside_a_tree_replaced_before_it_opens_refuses() {
    let mountinfo = live;
    let seat = reach(&[], &[]);
    // A file, a directory and a link, each replaced by a new one of its
    // kind between the listing that observed it and the observation that
    // opens it: only the open's own comparison can see it.
    for (name, kind) in [("lib/a.py", "file"), ("lib", "dir"), ("share", "link")] {
        let fixture = Fixture::new();
        let tree = fixture.path("opt/docs");
        fixture.file("opt/docs/lib/a.py", 0o644);
        fixture.link("opt/docs/share", Path::new("lib"));
        let target = tree.join(name);
        let swapped = std::cell::Cell::new(false);
        let swap = |dir: &Path, opened: &OsStr| {
            if dir.join(opened) == target && !swapped.replace(true) {
                std::fs::rename(&target, fixture.path("old")).unwrap();
                match kind {
                    "file" => drop(fixture.file("opt/docs/lib/a.py", 0o644)),
                    "dir" => std::fs::create_dir(&target).unwrap(),
                    _ => drop(fixture.link("opt/docs/share", Path::new("lib"))),
                }
            }
        };
        let opening = Host {
            opening: &swap,
            ..host(strangers(), &mountinfo)
        };
        let answered = answer(&[source(&tree, Role::Program)], &seat, &opening);
        assert_eq!((kind, answered), (kind, Err(Refusal::Identity)));
        assert!(swapped.get(), "{kind}");
    }
}

/// Set `file`'s mode.
pub(super) fn chmod(file: &Path, mode: u32) {
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// An extended access ACL naming this process's own uid beside the
/// owner's, group's and other's own entries, the owner's permissions
/// `owner` and every other entry's, the mask's included, `rest`, which
/// grants no write, so the mode's bits stay what they were: only the ACL
/// itself differs. The uid is mapped in any user namespace this runs in,
/// where an unmapped one such as `stranger()` is no valid ACL entry
/// (EINVAL).
pub(super) fn grant(file: &Path, (owner, rest): (u16, u16)) {
    let entry = |tag: u16, perm: u16, id: u32| {
        [
            &tag.to_le_bytes()[..],
            &perm.to_le_bytes(),
            &id.to_le_bytes(),
        ]
        .concat()
    };
    let acl = [
        2u32.to_le_bytes().to_vec(),
        entry(0x01, owner, u32::MAX),
        entry(0x02, rest, rustix::process::geteuid().as_raw()),
        entry(0x04, rest, u32::MAX),
        entry(0x10, rest, u32::MAX),
        entry(0x20, rest, u32::MAX),
    ]
    .concat();
    let fd = std::fs::File::open(file).unwrap();
    let name = "system.posix_acl_access";
    rustix::fs::fsetxattr(fd.as_fd(), name, &acl, rustix::fs::XattrFlags::empty()).unwrap();
}

/// A fresh fixture with a support library, `sys/lib/libx.so`, linked again
/// beside its directory: the fixture, the directory and the file.
pub(super) fn linked_library() -> (Fixture, PathBuf, PathBuf) {
    let fixture = Fixture::new();
    let lib = fixture.path("sys/lib");
    let file = fixture.file("sys/lib/libx.so", 0o644);
    std::fs::hard_link(&file, fixture.path("sys/other")).unwrap();
    (fixture, lib, file)
}

#[test]
fn a_linked_support_file_passes_only_with_the_whole_write_exclusion_proof() {
    let (fixture, lib, file) = linked_library();
    let mountinfo = live;
    let seat = reach(&[], &[]);
    let support = [source(&lib, Role::Support)];
    let with = |credentials: Credentials| answer(&support, &seat, &host(credentials, &mountinfo));
    assert_eq!(with(strangers()), Ok(()));
    // Owner equality, an unmapped owner, an unmapped writer, no writer and
    // unconfined privilege, each alone.
    let own = rustix::process::geteuid().as_raw();
    let overflowing = Credentials {
        overflow: own,
        ..strangers()
    };
    let unmapped = Credentials {
        uids: vec![stranger(), 65534],
        ..strangers()
    };
    let none = Credentials {
        uids: Vec::new(),
        ..strangers()
    };
    let unconfined = Credentials {
        confinement: Confinement::Unproved,
        ..strangers()
    };
    for credentials in [ours(), overflowing, unmapped, none, unconfined] {
        let case = format!("{credentials:?}");
        assert_eq!((&case, with(credentials)), (&case, Err(Refusal::Identity)));
    }
    // Group or other write, each alone, and then an access ACL.
    for mode in [0o664, 0o646] {
        chmod(&file, mode);
        assert_eq!((mode, with(strangers())), (mode, Err(Refusal::Identity)));
    }
    chmod(&file, 0o644);
    assert_eq!(with(strangers()), Ok(()));
    grant(&file, (6, 4));
    let mode = std::fs::metadata(&file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o644);
    assert_eq!(with(strangers()), Err(Refusal::Identity));
    // A singly linked support file needs no proof, even owned by a writer.
    let single = fixture.file("sys/single/libz.so", 0o664);
    let one = [source(single.parent().unwrap(), Role::Support)];
    assert_eq!(answer(&one, &seat, &host(ours(), &mountinfo)), Ok(()));
}

#[test]
fn special_unreadable_cyclic_and_absent_sources_refuse() {
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let seat = reach(&[], &[]);
    const IDENTITY: Result<(), Refusal> = Err(Refusal::Identity);
    // A FIFO and a socket inside a tree.
    for kind in ["fifo", "socket"] {
        let fixture = Fixture::new();
        let tree = fixture.path("opt/docs");
        fixture.plant("opt/docs/bin/docs-mcp");
        let endpoint = tree.join("endpoint");
        let _listener = match kind {
            "fifo" => {
                let mode = rustix::fs::Mode::from_raw_mode(0o600);
                let fifo = rustix::fs::FileType::Fifo;
                rustix::fs::mknodat(rustix::fs::CWD, &endpoint, fifo, mode, 0).unwrap();
                None
            }
            _ => {
                // Bound through the tree's descriptor: a deep build
                // directory would put its own path past SUN_LEN.
                let dir = std::fs::File::open(&tree).unwrap();
                let short = format!("/proc/self/fd/{}/endpoint", dir.as_raw_fd());
                Some(std::os::unix::net::UnixListener::bind(short).unwrap())
            }
        };
        assert!(endpoint.symlink_metadata().is_ok(), "{kind}");
        let one = [source(&tree, Role::Program)];
        assert_eq!((kind, answer(&one, &seat, &observing)), (kind, IDENTITY));
    }
    // A directory this process cannot list.
    let fixture = Fixture::new();
    let tree = fixture.path("opt/docs");
    let closed = tree.join("closed");
    std::fs::create_dir_all(&closed).unwrap();
    chmod(&closed, 0o000);
    assert!(
        std::fs::read_dir(&closed).is_err(),
        "run as an unprivileged user"
    );
    let one = [source(&tree, Role::Program)];
    let unreadable = answer(&one, &seat, &observing);
    chmod(&closed, 0o755);
    assert_eq!(unreadable, IDENTITY);
    // Links that resolve to each other; a required source that is absent;
    // an optional one that is absent has no handle and refuses nothing.
    fixture.link("a", &fixture.path("b"));
    fixture.link("b", &fixture.path("a"));
    let cyclic = fixture.path("a");
    assert_eq!(
        answer(&[source(&cyclic, Role::Support)], &seat, &observing),
        IDENTITY
    );
    let absent = fixture.path("none");
    assert_eq!(
        answer(&[source(&absent, Role::Support)], &seat, &observing),
        IDENTITY
    );
    let optional = Source {
        presence: Presence::Optional,
        ..source(&absent, Role::Support)
    };
    let observed = observe(&[optional], &seat, &observing).unwrap();
    assert!(observed.held[0].is_none());
}

#[test]
fn a_source_that_cannot_be_resolved_or_listed_refuses() {
    let fixture = Fixture::new();
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let seat = reach(&[], &[]);
    let optional = |path| Source {
        presence: Presence::Optional,
        ..source(path, Role::Support)
    };
    let held = |sources: &[Source<'_>]| {
        let observed = observe(sources, &seat, &observing);
        observed.map(|observed| {
            observed
                .held
                .iter()
                .map(Option::is_some)
                .collect::<Vec<_>>()
        })
    };
    // A FIFO as the source itself refuses, even an optional one; a path on
    // through a FIFO or a file names nothing, which only an optional source
    // may.
    let fifo = fixture.path("fifo");
    let mode = rustix::fs::Mode::from_raw_mode(0o600);
    rustix::fs::mknodat(rustix::fs::CWD, &fifo, rustix::fs::FileType::Fifo, mode, 0).unwrap();
    let file = fixture.file("file", 0o644);
    assert_eq!(held(&[optional(&fifo)]), Err(Refusal::Identity));
    let (through_fifo, through_file) = (fifo.join("x"), file.join("x"));
    let through = [optional(&through_fifo), optional(&through_file)];
    assert_eq!(held(&through), Ok(vec![false, false]));
    let required = [source(&through_file, Role::Support)];
    assert_eq!(held(&required), Err(Refusal::Identity));
    // A directory on the way that cannot be searched; a source directory
    // that cannot be listed; an entry of one that lists but cannot be
    // searched.
    let closed = fixture.path("closed");
    std::fs::create_dir_all(closed.join("x")).unwrap();
    let listed = fixture.file("tree/listed/f", 0o644);
    let listed = listed.parent().unwrap().to_path_buf();
    let cases = [
        (&closed, closed.join("x"), 0o000),
        (&closed, closed.clone(), 0o000),
        (&listed, fixture.path("tree"), 0o444),
    ];
    for (dir, path, mode) in cases {
        chmod(dir, mode);
        let answer = held(&[source(&path, Role::Support)]);
        chmod(dir, 0o755);
        assert_eq!((&path, answer), (&path, Err(Refusal::Identity)));
    }
}

/// A host bounded by `limits`, reading `table`.
fn bounded<'h>(limits: Limits, mountinfo: Table<'h>) -> Host<'h> {
    Host {
        limits,
        ..host(strangers(), mountinfo)
    }
}

/// A fixture whose package tree `pkg` holds one file.
fn one_file_tree() -> (Fixture, PathBuf) {
    let fixture = Fixture::new();
    fixture.file("pkg/a", 0o644);
    let tree = fixture.path("pkg");
    (fixture, tree)
}

#[test]
fn an_observation_with_no_room_or_no_mount_table_refuses() {
    let (_fixture, tree) = one_file_tree();
    let seat = reach(&[], &[]);
    let one = [source(&tree, Role::Program)];
    let mountinfo = live;
    assert_eq!(answer(&one, &seat, &host(strangers(), &mountinfo)), Ok(()));
    // Not even the source's own root fits.
    let none = Limits {
        entries: 0,
        ..LIMITS
    };
    let full = bounded(none, &mountinfo);
    assert_eq!(answer(&one, &seat, &full), Err(Refusal::Identity));
    // A mount table that cannot be read proves no mount.
    let unread = || -> Option<Box<dyn Read>> { None };
    let blind = host(strangers(), &unread);
    assert_eq!(answer(&one, &seat, &blind), Err(Refusal::Identity));
}

#[test]
fn each_bound_refuses_one_over_and_passes_at_its_limit() {
    let fixture = Fixture::new();
    let tree = fixture.path("pkg");
    fixture.file("pkg/a", 0o644);
    fixture.file("pkg/bb", 0o644);
    let text = table();
    let mountinfo = || serving(&text);
    let seat = reach(&[], &[]);
    let one = [source(&tree, Role::Program)];
    let at = |limits: Limits, sources: &[Source<'_>]| {
        answer(sources, &seat, &bounded(limits, &mountinfo))
    };
    // Entries: the root and its two files, on the one mount it stood on;
    // the bound is on every record the table holds.
    let observed = observe(&one, &seat, &host(strangers(), &mountinfo)).unwrap();
    let count = parse(&text, usize::MAX).unwrap().len();
    assert_eq!((observed.sources.entries, observed.sources.mounts), (3, 1));
    assert_eq!(
        at(
            Limits {
                entries: 3,
                ..LIMITS
            },
            &one
        ),
        Ok(())
    );
    assert_eq!(
        at(
            Limits {
                entries: 2,
                ..LIMITS
            },
            &one
        ),
        Err(Refusal::Identity)
    );
    // Mount records.
    assert_eq!(
        at(
            Limits {
                mounts: count,
                ..LIMITS
            },
            &one
        ),
        Ok(())
    );
    let fewer = count - 1;
    assert_eq!(
        at(
            Limits {
                mounts: fewer,
                ..LIMITS
            },
            &one
        ),
        Err(Refusal::Identity)
    );
    // Depth: the deepest directory lies three below the root.
    std::fs::create_dir_all(fixture.path("deep/a/b/c")).unwrap();
    let deep = fixture.path("deep");
    let deep = [source(&deep, Role::Program)];
    assert_eq!(at(Limits { depth: 3, ..LIMITS }, &deep), Ok(()));
    assert_eq!(
        at(Limits { depth: 2, ..LIMITS }, &deep),
        Err(Refusal::Identity)
    );
    // Hops: three links to the tree.
    fixture.link("l3", &tree);
    fixture.link("l2", &fixture.path("l3"));
    let first = fixture.link("l1", &fixture.path("l2"));
    let linked = [source(&first, Role::Program)];
    assert_eq!(at(Limits { hops: 3, ..LIMITS }, &linked), Ok(()));
    assert_eq!(
        at(Limits { hops: 2, ..LIMITS }, &linked),
        Err(Refusal::Identity)
    );
}

/// MB3's bounds are the one fixed set every live observation takes.
#[test]
fn the_observers_bounds_are_mb3s_fixed_counts() {
    let Limits {
        entries,
        depth,
        hops,
        mounts,
    } = LIMITS;
    assert_eq!((entries, depth, hops, mounts), (1_000_000, 64, 40, 65_536));
    let live = Host::live().limits;
    let taken = (live.entries, live.depth, live.hops, live.mounts);
    assert_eq!(taken, (entries, depth, hops, mounts));
}

#[test]
fn only_the_mount_records_the_walk_stood_on_are_counted_or_digested() {
    let (_fixture, tree) = one_file_tree();
    let seat = reach(&[], &[]);
    let one = [source(&tree, Role::Program)];
    let observed = |text: Vec<u8>| {
        let mountinfo = move || serving(&text);
        let observing = host(strangers(), &mountinfo);
        let sources = observe(&one, &seat, &observing).map(|observed| observed.sources);
        sources.map(|sources| (sources.entries, sources.mounts, sources.digest))
    };
    let text = table();
    let parsed = parse(&text, usize::MAX).unwrap();
    let first = observed(text.clone()).unwrap();
    assert_eq!((first.0, first.1), (2, 1));
    // A mount made elsewhere on the host, or one taken away, moves no
    // sealed fact.
    let next = parsed.iter().map(|record| record.id).max().unwrap() + 1;
    let elsewhere = format!("{next} 1 0:4242 / /u6c5b/elsewhere rw - tmpfs none rw\n");
    let proc = parsed
        .iter()
        .find(|record| record.point == Path::new("/proc"));
    let proc = proc.unwrap().id;
    for churned in [
        [text.clone(), elsewhere.into_bytes()].concat(),
        edited(&text, proc, Vec::clear),
    ] {
        assert_eq!(observed(churned), Ok(first.clone()));
    }
    // A change to the mount the tree stands on still moves the digest.
    let flags = rustix::fs::AtFlags::empty();
    let mask = rustix::fs::StatxFlags::MNT_ID;
    let own = rustix::fs::statx(rustix::fs::CWD, &tree, flags, mask);
    let own = own.unwrap().stx_mnt_id;
    let rooted = edited(&text, own, |fields| fields[3] = "/u6c5b".to_string());
    let moved = observed(rooted).unwrap();
    assert_eq!((moved.0, moved.1), (first.0, first.1));
    assert_ne!(moved.2, first.2);
}

#[test]
fn the_box_keeps_its_writers_uids_only_where_their_privilege_is_confined() {
    assert_eq!(confined_writers(&strangers()), Some(vec![stranger()]));
    let unproved = Credentials {
        confinement: Confinement::Unproved,
        ..strangers()
    };
    assert_eq!(confined_writers(&unproved), None);
}

#[test]
fn a_route_or_link_target_in_reach_refuses() {
    let fixture = Fixture::new();
    let docs = fixture.path("opt/docs");
    fixture.plant("opt/docs/bin/docs-mcp");
    let (work, cache) = (fixture.path("work"), fixture.path("cache"));
    let mountinfo = live;
    let observing = host(strangers(), &mountinfo);
    let both = reach(std::slice::from_ref(&work), std::slice::from_ref(&cache));
    let tree = [source(&docs, Role::Program)];
    assert_eq!(answer(&tree, &both, &observing), Ok(()));
    // A link inside the tree to a target in either root, absolute or
    // relative, refuses though nothing mounts it; one elsewhere does not.
    let link = docs.join("share");
    for (target, expected) in [
        (work.join("data"), Err(Refusal::BindOverlapsReach)),
        (
            PathBuf::from("../../cache/x"),
            Err(Refusal::BindOverlapsReach),
        ),
        (PathBuf::from("/usr/share/zoneinfo"), Ok(())),
    ] {
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let got = answer(&tree, &both, &observing);
        std::fs::remove_file(&link).unwrap();
        assert_eq!((&target, got), (&target, expected));
    }
    // A route through either root: the launch's own cause in writable
    // reach, the overlap cause otherwise.
    let via_work = fixture.link("work/docs-mcp", &docs.join("bin/docs-mcp"));
    let via_cache = fixture.link("cache/docs", &docs);
    let launched = [source(&via_work, Role::Launch)];
    assert_eq!(
        answer(&launched, &both, &observing),
        Err(Refusal::LaunchInReach)
    );
    let program = [source(&via_work, Role::Program)];
    assert_eq!(
        answer(&program, &both, &observing),
        Err(Refusal::BindOverlapsReach)
    );
    let cached = [source(&via_cache, Role::Launch)];
    assert_eq!(
        answer(&cached, &both, &observing),
        Err(Refusal::BindOverlapsReach)
    );
    // Reach first, then links: a linked program file beside the target.
    let module = fixture.file("opt/docs/lib/a.py", 0o644);
    std::fs::hard_link(&module, fixture.path("opt/docs/lib/b.py")).unwrap();
    assert_eq!(answer(&tree, &both, &observing), Err(Refusal::Linked));
    std::os::unix::fs::symlink(work.join("data"), &link).unwrap();
    assert_eq!(
        answer(&tree, &both, &observing),
        Err(Refusal::BindOverlapsReach)
    );
    let mixed = [
        source(&docs, Role::Program),
        source(&via_work, Role::Launch),
    ];
    assert_eq!(
        answer(&mixed, &both, &observing),
        Err(Refusal::LaunchInReach)
    );
}

#[test]
fn replacing_a_hop_or_the_source_between_resolutions_refuses() {
    let fixture = Fixture::new();
    let docs = fixture.path("opt/docs");
    fixture.plant("opt/docs/bin/docs-mcp");
    fixture.link("b", &docs);
    let a = fixture.link("a", &fixture.path("b"));
    let mountinfo = live;
    let seat = reach(&[], &[]);
    let quiet = sources_host(strangers(), &mountinfo, &nothing);
    assert_eq!(answer(&[source(&a, Role::Program)], &seat, &quiet), Ok(()));
    // An intermediate link replaced by a new one to the same target, made
    // beside it and renamed over it: the object is the same, the route is
    // not.
    let relink = || {
        std::os::unix::fs::symlink(&docs, fixture.path("b.new")).unwrap();
        std::fs::rename(fixture.path("b.new"), fixture.path("b")).unwrap();
    };
    let hop = sources_host(strangers(), &mountinfo, &relink);
    assert_eq!(
        answer(&[source(&a, Role::Program)], &seat, &hop),
        Err(Refusal::Identity)
    );
    // The source itself replaced: the route is the same, the object its
    // name now gives is not the one the handle holds.
    let replace = || {
        std::fs::rename(&docs, fixture.path("opt/old")).unwrap();
        std::fs::create_dir(&docs).unwrap();
    };
    let swapped = sources_host(strangers(), &mountinfo, &replace);
    let direct = [source(&docs, Role::Program)];
    assert_eq!(answer(&direct, &seat, &swapped), Err(Refusal::Identity));
    // An optional source that appears between the two.
    let late = fixture.path("late");
    let appear = || std::fs::create_dir(&late).unwrap();
    let appearing = sources_host(strangers(), &mountinfo, &appear);
    let optional = Source {
        presence: Presence::Optional,
        ..source(&late, Role::Support)
    };
    assert_eq!(
        answer(&[optional], &seat, &appearing),
        Err(Refusal::Identity)
    );
}

#[test]
fn an_absent_optional_source_that_no_longer_resolves_refuses() {
    let fixture = Fixture::new();
    let mountinfo = live;
    let seat = reach(&[], &[]);
    let (late, closed) = (fixture.path("late"), fixture.path("closed"));
    let below = closed.join("x");
    let optional = |path| Source {
        presence: Presence::Optional,
        ..source(path, Role::Support)
    };
    // Absent at both resolutions: no handle, and nothing refused.
    let quiet = sources_host(strangers(), &mountinfo, &nothing);
    for path in [&late, &below] {
        let observed = observe(&[optional(path)], &seat, &quiet).unwrap();
        assert!(observed.held[0].is_none());
    }
    // Absent, and then a link to itself, or below a directory that cannot
    // be searched: only a second absence proves it still absent.
    let cycle = || {
        fixture.link("late", &fixture.path("late"));
    };
    let cycling = sources_host(strangers(), &mountinfo, &cycle);
    let cyclic = answer(&[optional(&late)], &seat, &cycling);
    assert_eq!(cyclic, Err(Refusal::Identity));
    let close = || {
        std::fs::create_dir(&closed).unwrap();
        chmod(&closed, 0o000);
    };
    let closing = sources_host(strangers(), &mountinfo, &close);
    let unreadable = answer(&[optional(&below)], &seat, &closing);
    chmod(&closed, 0o755);
    assert_eq!(unreadable, Err(Refusal::Identity));
}

#[test]
fn a_handle_that_opens_another_object_than_the_one_observed_refuses() {
    let fixture = Fixture::new();
    let docs = fixture.path("opt/docs");
    fixture.plant("opt/docs/bin/docs-mcp");
    let mountinfo = live;
    let seat = reach(&[], &[]);
    let tree = [source(&docs, Role::Program)];
    assert_eq!(answer(&tree, &seat, &host(strangers(), &mountinfo)), Ok(()));
    // The source replaced once, after its facts were read and before its
    // handle opens: every later resolution agrees with that handle, so
    // only the open's own comparison can refuse it.
    let swapped = std::cell::Cell::new(false);
    let swap = |dir: &Path, name: &OsStr| {
        if dir.join(name) == docs && !swapped.replace(true) {
            std::fs::rename(&docs, fixture.path("opt/old")).unwrap();
            std::fs::create_dir(&docs).unwrap();
        }
    };
    let opening = Host {
        opening: &swap,
        ..host(strangers(), &mountinfo)
    };
    assert_eq!(answer(&tree, &seat, &opening), Err(Refusal::Identity));
    assert!(swapped.get());
}

#[test]
fn the_root_every_resolution_starts_from_is_closed_on_exec() {
    let held = root().unwrap();
    let flags = rustix::io::fcntl_getfd(&held).unwrap();
    assert_eq!(flags, rustix::io::FdFlags::CLOEXEC);
}

/// `text` with the line of mount `id` rewritten by `edit`, or dropped
/// where `edit` empties it.
pub(super) fn edited(text: &[u8], id: u64, edit: impl Fn(&mut Vec<String>)) -> Vec<u8> {
    let text = String::from_utf8(text.to_vec()).unwrap();
    let lines: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let mut fields: Vec<String> = line.split(' ').map(String::from).collect();
            if fields[0] == id.to_string() {
                edit(&mut fields);
            }
            (!fields.is_empty()).then(|| fields.join(" "))
        })
        .collect();
    format!("{}\n", lines.join("\n")).into_bytes()
}

/// A mount table path, escaped as the kernel writes it.
pub(super) fn escaped(path: &Path) -> String {
    path.display()
        .to_string()
        .replace('\\', "\\134")
        .replace(' ', "\\040")
}

#[test]
fn a_mount_alias_or_an_unproved_filesystem_refuses() {
    let fixture = Fixture::new();
    let docs = fixture.path("opt/docs");
    fixture.plant("opt/docs/bin/docs-mcp");
    let work = fixture.path("work");
    let seat = reach(std::slice::from_ref(&work), &[]);
    let text = table();
    let parsed = parse(&text, usize::MAX).unwrap();
    let flags = rustix::fs::AtFlags::empty();
    let mask = rustix::fs::StatxFlags::MNT_ID;
    let id = rustix::fs::statx(rustix::fs::CWD, &docs, flags, mask)
        .unwrap()
        .stx_mnt_id;
    let own = parsed.iter().find(|record| record.id == id).unwrap();
    let inside = own.root.join(docs.strip_prefix(&own.point).unwrap());
    let next = parsed.iter().map(|record| record.id).max().unwrap() + 1;
    let (major, minor) = own.dev;
    let alias = |root: &Path, point: &Path| {
        let line = format!(
            "{next} {id} {major}:{minor} {} {} rw - ext4 /dev/x rw\n",
            escaped(root),
            escaped(point)
        );
        [text.clone(), line.into_bytes()].concat()
    };
    let tree = [source(&docs, Role::Program)];
    let with = |table: Vec<u8>| {
        let mountinfo = move || serving(&table);
        answer(&tree, &seat, &host(strangers(), &mountinfo))
    };
    assert_eq!(with(text.clone()), Ok(()));
    // The package's parent mounted again inside writable reach: every file
    // there has one link and another canonical path, and it still refuses;
    // the same alias outside reach does not.
    let parent = inside.parent().unwrap();
    assert_eq!(
        with(alias(parent, &work.join("opt"))),
        Err(Refusal::Identity)
    );
    assert_eq!(with(alias(parent, &fixture.path("srv/opt"))), Ok(()));
    // Part of the package mounted into reach.
    assert_eq!(
        with(alias(&inside.join("bin"), &work.join("bin"))),
        Err(Refusal::Identity)
    );
    // Its own mount on an unproved filesystem, of another device, or gone.
    for edit in [overlay, moved] {
        let table = edited(&text, id, |fields| edit(fields));
        assert_eq!(with(table), Err(Refusal::Identity));
    }
    assert_eq!(with(edited(&text, id, Vec::clear)), Err(Refusal::Identity));
}

/// A mount line's `fields` made an overlay's, a filesystem never proved.
pub(super) fn overlay(fields: &mut [String]) {
    let at = fields.iter().position(|field| field == "-").unwrap();
    fields[at + 1] = "overlay".to_string();
}

/// A mount line's `fields` given another device.
pub(super) fn moved(fields: &mut [String]) {
    fields[2] = "4095:4095".to_string();
}

/// The box `program` is prepared in on `host`, its bootstrap `bootstrap`.
pub(super) fn prepared_on(
    program: &ServerProgram,
    bootstrap: &Path,
    host: &Host<'_>,
) -> Result<ServerBox, Refusal> {
    let seat = reach(&[], &[]);
    let profile = ServerProfile {
        reach: &seat,
        network: &Network::Isolated,
        bootstrap,
        writers: &[],
    };
    ServerBox::prepare_with(program, &profile, host)
}

#[test]
fn a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable() {
    let fixture = Fixture::new();
    let entry = fixture.plant("opt/docs/bin/docs-mcp");
    let docs = ServerProgram::resolve(entry.to_str().unwrap(), &fixture.home).unwrap();
    let bootstrap = std::env::current_exe().unwrap();
    // A stand-in for the launcher's run reports each version; the writers
    // here are strangers, or this process's own.
    let image = launcher(&fixture, "image/bwrap");
    let answer = |found: &dyn Fn() -> Option<PathBuf>, credentials: Credentials, report: &str| {
        let run = move |_: std::os::fd::BorrowedFd<'_>| report.to_string();
        let host = Host {
            found,
            credentials,
            run: &run,
            ..Host::live()
        };
        prepared_on(&docs, &bootstrap, &host).map(|_| ())
    };
    // Filesystem causes come first: a linked package file on a launcher
    // refused before it runs, outside the system set, whoever the writers
    // are.
    let module = fixture.file("opt/docs/lib/a.py", 0o644);
    std::fs::hard_link(&module, fixture.path("opt/docs/lib/b.py")).unwrap();
    let floor = "bubblewrap 0.5.0";
    assert_eq!(
        answer(&named(&image), strangers(), floor),
        Err(Refusal::Linked)
    );
    assert_eq!(answer(&named(&image), ours(), floor), Err(Refusal::Linked));
    std::fs::remove_file(fixture.path("opt/docs/lib/b.py")).unwrap();
    if unservable() {
        return;
    }
    let Some(bwrap) = bubblewrap() else {
        return;
    };
    // The host's own bubblewrap: a version that mounts descriptors admits.
    let found = named(&bwrap);
    assert_eq!(answer(&found, strangers(), floor), Ok(()));
    for report in ["bubblewrap 0.4.1", "bubblewrap"] {
        let unable = answer(&found, strangers(), report);
        assert_eq!((report, unable), (report, Err(Refusal::Unavailable)));
    }
    assert_eq!(
        answer(&|| None, strangers(), floor),
        Err(Refusal::Unavailable)
    );
    // A launcher outside the system set, or one a writer could write.
    assert_eq!(
        answer(&named(&image), strangers(), floor),
        Err(Refusal::Identity)
    );
    let rooted = Credentials {
        uids: vec![0],
        ..strangers()
    };
    assert_eq!(answer(&found, rooted, floor), Err(Refusal::Identity));
}

/// What finds `path`, without running it.
pub(super) fn named(path: &Path) -> impl Fn() -> Option<PathBuf> {
    let path = path.to_path_buf();
    move || Some(path.clone())
}

#[test]
fn program_and_bootstrap_links_refuse_and_the_launch_must_still_name_the_executable() {
    let fixture = Fixture::new();
    let bootstrap = std::env::current_exe().unwrap();
    let strange = Host {
        credentials: strangers(),
        ..Host::live()
    };
    let linked = |program: &ServerProgram, bootstrap: &Path| {
        prepared_on(program, bootstrap, &strange).map(|_| ())
    };
    // A package with a second link to one of its files, whose owner is no
    // writer: the program-tree cause, unconditionally.
    let entry = fixture.plant("opt/docs/bin/docs-mcp");
    let docs = ServerProgram::resolve(entry.to_str().unwrap(), &fixture.home).unwrap();
    let module = fixture.file("opt/docs/lib/a.py", 0o644);
    std::fs::hard_link(&module, fixture.path("opt/docs/lib/b.py")).unwrap();
    assert_eq!(linked(&docs, &bootstrap), Err(Refusal::Linked));
    std::fs::remove_file(fixture.path("opt/docs/lib/b.py")).unwrap();
    // A bootstrap with a second link, and one inside a private directory,
    // which must start empty (MB4): linked there as well, its link is the
    // earlier cause (MB3's order).
    let boot = fixture.plant("boot/brokkr");
    std::fs::hard_link(&boot, fixture.path("boot/again")).unwrap();
    assert_eq!(linked(&docs, &boot), Err(Refusal::Linked));
    let private = tempfile::tempdir_in("/tmp").unwrap();
    let hidden = private.path().join("brokkr");
    std::fs::copy(&boot, &hidden).unwrap();
    std::fs::hard_link(&hidden, private.path().join("again")).unwrap();
    assert_eq!(linked(&docs, &hidden), Err(Refusal::Linked));
    std::fs::remove_file(private.path().join("again")).unwrap();
    if unservable() {
        return;
    }
    let live = confined();
    let answer = |program: &ServerProgram, bootstrap: &Path| {
        prepared_on(program, bootstrap, &live).map(|_| ())
    };
    // Singly linked in the private directory, identity is unprotected.
    assert_eq!(answer(&docs, &hidden), Err(Refusal::Identity));
    assert_eq!(answer(&docs, &bootstrap), Ok(()));
    // The launch name repointed after it was resolved.
    let other = fixture.plant("opt/other/bin/docs-mcp");
    let link = fixture.link("links/docs-mcp", &entry);
    let named = ServerProgram::resolve(link.to_str().unwrap(), &fixture.home).unwrap();
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&other, &link).unwrap();
    assert_eq!(answer(&named, &bootstrap), Err(Refusal::Identity));
}

#[test]
fn the_box_mounts_the_observed_object_and_carries_its_facts() {
    if unservable() {
        return;
    }
    let fixture = Fixture::new();
    let entry = fixture.plant("opt/docs/bin/docs-mcp");
    let docs = fixture.path("opt/docs");
    let module = fixture.file("opt/docs/lib/a.py", 0o644);
    let program = ServerProgram::resolve(entry.to_str().unwrap(), &fixture.home).unwrap();
    let bootstrap = std::env::current_exe().unwrap();
    // Only the records the walk stood on are counted, so the count is a
    // fact of the tree, whatever mounts the host makes meanwhile.
    let sealed = |server: &ServerBox| {
        let sources = server.sources();
        (sources.entries, sources.mounts, sources.digest.clone())
    };
    let facts = || sealed(&prepared_on(&program, &bootstrap, &confined()).unwrap());
    let server = prepared_on(&program, &bootstrap, &confined()).unwrap();
    // The box carries its observation's facts for admission to compare.
    // An unchanged tree, its identity generated afresh under another name,
    // is the same set; a file rewritten in place is the same entries and
    // another digest; a directory and a file added are two more entries.
    let first = sealed(&server);
    assert_eq!(facts(), first);
    let writers = confined().credentials.uids;
    assert_eq!(server.writers(), Some(&writers[..]));
    std::fs::write(&module, "rewritten in place\n").unwrap();
    let rewritten = facts();
    assert_eq!(rewritten.0, first.0);
    assert_ne!(rewritten.2, first.2);
    fixture.file("opt/docs/share/b.txt", 0o644);
    let grown = facts();
    assert_eq!(grown.0, first.0 + 2);
    assert_ne!(grown.2, rewritten.2);
    let identity = |held: &std::fs::Metadata| (held.dev(), held.ino());
    let observed = identity(&std::fs::metadata(&docs).unwrap());
    std::fs::rename(&docs, fixture.path("opt/old")).unwrap();
    std::fs::create_dir_all(docs.join("bin")).unwrap();
    let (_, fd) = server.handles().find(|(path, _)| *path == docs).unwrap();
    let held = std::fs::File::from(fd.try_clone().unwrap())
        .metadata()
        .unwrap();
    assert_eq!(identity(&held), observed);
    assert_ne!(identity(&std::fs::metadata(&docs).unwrap()), observed);
    // The argv names that descriptor, never the path.
    let target = docs.display().to_string();
    let number = fd.as_raw_fd().to_string();
    let mount = server
        .argv()
        .windows(3)
        .find(|mount| mount[2] == target)
        .unwrap();
    assert_eq!(mount, ["--ro-bind-fd", number.as_str(), target.as_str()]);
}

#[test]
fn the_source_set_digest_binds_the_observed_facts_and_no_descriptor_number() {
    let fixture = Fixture::new();
    let docs = fixture.path("opt/docs");
    let module = fixture.file("opt/docs/lib/a.py", 0o644);
    let text = table();
    let mountinfo = || serving(&text);
    let observing = host(strangers(), &mountinfo);
    let seat = reach(&[], &[]);
    let tree = [source(&docs, Role::Program)];
    let digest = || observe(&tree, &seat, &observing).unwrap().sources.digest;
    let first = digest();
    assert_eq!(first.len(), 64);
    assert!(first
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() & !byte.is_ascii_uppercase()));
    // Other descriptors held open move every handle's number, not the
    // digest.
    let _held: Vec<std::fs::File> = (0..5)
        .map(|_| std::fs::File::open(build_dir()).unwrap())
        .collect();
    assert_eq!(digest(), first);
    // A mode, a new entry or a link's target each change it.
    chmod(&module, 0o600);
    let moded = digest();
    assert_ne!(moded, first);
    fixture.link("opt/docs/lib/c.py", Path::new("a.py"));
    let linked = digest();
    assert_ne!(linked, moded);
    std::fs::remove_file(docs.join("lib/c.py")).unwrap();
    fixture.link("opt/docs/lib/c.py", Path::new("b.py"));
    assert_ne!(digest(), linked);
    // A generated file is digested by its name and the facts every
    // preparation repeats, never its place or inode: two files written
    // alike under different routes, places and inodes are one digest as
    // generated files, two as program files.
    let one = fixture.file("one/etc/passwd", 0o644);
    let two = fixture.file("two/etc/passwd", 0o644);
    let digest = |path: &Path, role: Role| {
        let sources = observe(&[source(path, role)], &seat, &observing).unwrap();
        sources.sources.digest
    };
    assert_eq!(digest(&one, Role::Generated), digest(&two, Role::Generated));
    assert_ne!(digest(&one, Role::Program), digest(&two, Role::Program));
    // Its name, its mode and its size each count.
    let group = fixture.file("two/etc/group", 0o644);
    assert_ne!(
        digest(&group, Role::Generated),
        digest(&one, Role::Generated)
    );
    chmod(&two, 0o600);
    assert_ne!(digest(&two, Role::Generated), digest(&one, Role::Generated));
    chmod(&two, 0o644);
    std::fs::write(&two, "runner\n").unwrap();
    assert_ne!(digest(&two, Role::Generated), digest(&one, Role::Generated));
}

#[test]
fn the_digest_follows_the_tree_never_the_order_it_is_listed_or_mounted_in() {
    // A tmpfs lists a directory in the order its entries were made, and a
    // rename makes one anew: renamed away and back, with the directory's
    // time put back, it lists in another order with every fact the same.
    let shm = tempfile::tempdir_in("/dev/shm").unwrap();
    let tree = shm.path().canonicalize().unwrap().join("tree");
    for name in ["a", "b", "c"] {
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::write(tree.join(name), name).unwrap();
    }
    // A second source on another mount, so two records are digested.
    let fixture = Fixture::new();
    let docs = fixture.path("opt/docs");
    fixture.file("opt/docs/a.py", 0o644);
    let sources = [source(&tree, Role::Program), source(&docs, Role::Program)];
    let digest = |text: &[u8]| {
        let mountinfo = || serving(text);
        let observing = host(strangers(), &mountinfo);
        observe(&sources, &reach(&[], &[]), &observing)
            .unwrap()
            .sources
    };
    let listed = || -> Vec<OsString> {
        let entries = std::fs::read_dir(&tree).unwrap();
        entries.map(|entry| entry.unwrap().file_name()).collect()
    };
    let text = table();
    let (first, order) = (digest(&text), listed());
    let modified = std::fs::metadata(&tree).unwrap().modified().unwrap();
    std::fs::rename(tree.join("a"), tree.join("z")).unwrap();
    std::fs::rename(tree.join("z"), tree.join("a")).unwrap();
    let dir = std::fs::File::open(&tree).unwrap();
    dir.set_modified(modified).unwrap();
    assert_ne!(listed(), order);
    assert_eq!(digest(&text).digest, first.digest);
    // The same mount table read in reverse.
    let lines = text
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    let reversed: Vec<u8> = lines
        .rev()
        .flat_map(|line| [line, b"\n"])
        .flatten()
        .copied()
        .collect();
    assert_ne!(reversed, text);
    assert_eq!((first.mounts, first.entries), (digest(&reversed).mounts, 6));
    assert_eq!(digest(&reversed).digest, first.digest);
}

/// The regular files of the server's system set this process cannot
/// open, found apart from the observer with plain metadata, each once
/// however many system roots reach it.
fn unreadable_system_files() -> Vec<PathBuf> {
    let roots = server_system().into_iter();
    let mut pending: Vec<PathBuf> = roots
        .filter_map(|root| Path::new(root).canonicalize().ok())
        .collect();
    let mut unreadable = Vec::new();
    while let Some(path) = pending.pop() {
        let kind = path.symlink_metadata().unwrap().file_type();
        if kind.is_dir() {
            let listed = std::fs::read_dir(&path).unwrap();
            pending.extend(listed.map(|entry| entry.unwrap().path()));
        } else if kind.is_file() && std::fs::File::open(&path).is_err() {
            unreadable.push(path);
        }
    }
    unreadable.sort();
    unreadable.dedup();
    unreadable
}

#[test]
fn a_server_entry_root_owned_and_singly_linked_passes_with_the_systems_links_intact() {
    // The host's own system set, hard links and root-only files and all,
    // under this process's credentials confined: a singleton system entry
    // and a package it holds.
    if unservable() {
        return;
    }
    let fixture = Fixture::new();
    let bootstrap = std::env::current_exe().unwrap();
    let sh = ServerProgram::resolve("sh", &fixture.home).unwrap();
    assert!(matches!(sh.tree(), Tree::System {}));
    let (executable, _) = installed();
    let package = ServerProgram::resolve(executable.to_str().unwrap(), &fixture.home).unwrap();
    let unreadable = unreadable_system_files();
    eprintln!(
        "{} root-only system files: {unreadable:?}",
        unreadable.len()
    );
    for program in [&sh, &package] {
        let answer = prepared_on(program, &bootstrap, &confined());
        let counts = answer.as_ref().map(|server| {
            let sources = server.sources();
            (sources.entries, sources.mounts)
        });
        eprintln!(
            "{}: entries and mount records {counts:?}",
            program.executable().display()
        );
        assert_eq!(
            (program.executable(), counts.map(|_| ())),
            (program.executable(), Ok(()))
        );
    }
}

/// The digest names an object by its joined place, built from its
/// directory and name: an entry directly under `/` takes no second
/// separator, so it digests as its own path does, and `//usr` stays a
/// different place.
#[test]
fn an_entry_under_the_root_digests_as_its_joined_path() {
    use sha2::Digest as _;
    let of = |dir: &str, name: &str| {
        let mut hasher = sha2::Sha256::new();
        digest(
            &mut hasher,
            (Path::new(dir), OsStr::new(name)),
            &[b"facts"],
            b"",
        );
        hasher.finalize()
    };
    assert_eq!(of("/", "usr"), of("/usr", ""));
    assert_eq!(of("/usr", "lib"), of("/usr/lib", ""));
    assert_ne!(of("/", "usr"), of("//usr", ""));
}
