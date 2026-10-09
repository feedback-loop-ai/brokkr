//! What U6c5c checks beside the sources and the launcher (decision 0065
//! slice two; MB3, MB4): the store's identity, its route's protection and
//! every place it or a hop of its route shows at; and the managed writers,
//! observed and sealed, in the write-exclusion proof. The launcher's own
//! proofs moved to `launcher.rs`.

use std::cell::Cell;
use std::ffi::OsStr;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;

use super::super::namespace::sources::host::store::{admitted, guarded, shut, single};
use super::super::namespace::sources::host::*;
use super::super::namespace::sources::{observe, Role};
use super::super::*;
use super::server::{unservable, Host as Fixture};
use super::sources::{chmod, edited, escaped, grant, host, moved, overlay};
use super::sources::{confined, linked_library, prepared_on, reach, serving, source};
use super::sources::{strangers, table};
use crate::broker::{Reach, Refusal};

/// The docs package's program under `fixture`.
pub(super) fn docs(fixture: &Fixture) -> ServerProgram {
    let entry = fixture.plant("opt/docs/bin/docs-mcp");
    ServerProgram::resolve(entry.to_str().unwrap(), &fixture.home).unwrap()
}

/// The mount id of the object at `path`.
pub(super) fn mount_of(path: &Path) -> u64 {
    let flags = rustix::fs::AtFlags::empty();
    let mask = rustix::fs::StatxFlags::MNT_ID;
    let held = rustix::fs::statx(rustix::fs::CWD, path, flags, mask).unwrap();
    held.stx_mnt_id
}

/// Why a run skips a store proof: MB4's route guard admits a store only on
/// a route no one but its owner or root can change.
const NO_STORE_PLACE: &str = "neither TMPDIR nor /var/tmp is a directory outside /tmp whose every \
    ancestor only its owner or root can change, so no store fixture can pass MB4's route guard";

/// A store fixture, made owner-only under the first of `TMPDIR` and
/// `/var/tmp` that is a directory outside `/tmp` (which a server box keeps
/// private) whose every ancestor is [`shut`] to all but this user and
/// root; none skips, failing where boundary evidence is required.
fn stores() -> Option<Fixture> {
    let owner = rustix::process::geteuid().as_raw();
    let shut_to = |dir: &Path| {
        std::fs::metadata(dir).is_ok_and(|held| {
            let (uid, mode, acl) = (held.uid(), held.mode(), Acl::Absent);
            held.is_dir() & shut(Owned { uid, mode, acl }, owner)
        })
    };
    let qualifies = |place: &PathBuf| !place.starts_with("/tmp") & place.ancestors().all(shut_to);
    let named = std::env::var_os("TMPDIR").map(PathBuf::from);
    let places = named.into_iter().chain([PathBuf::from("/var/tmp")]);
    let place = places
        .filter_map(|place| place.canonicalize().ok())
        .find(qualifies);
    let Some(place) = place else {
        skip_boundary_proof(boundary_evidence_required(), NO_STORE_PLACE);
        return None;
    };
    let fixture = Fixture::under(&place);
    chmod(&fixture.root, 0o700);
    Some(fixture)
}

/// A protected empty store at `relative` under `fixture`, every directory
/// it lies in below the root writable by its owner alone.
fn stored(fixture: &Fixture, relative: &str) -> PathBuf {
    let path = fixture.file(relative, 0o600);
    std::fs::write(&path, "").unwrap();
    let dirs = path.ancestors().skip(1);
    for dir in dirs.take_while(|dir| *dir != fixture.root) {
        chmod(dir, 0o755);
    }
    path
}

/// MB4's answer for the store at `store`, the seat's `seat` and the box's
/// `sources`, on a host whose mount table reads as `text`.
fn store_on(
    store: &Path,
    seat: &Reach,
    sources: &[PathBuf],
    text: &[u8],
) -> Result<OwnedFd, Refusal> {
    let mountinfo = || serving(text);
    let held = sources.iter().map(PathBuf::as_path);
    admitted(store, seat, held, &host(strangers(), &mountinfo))
}

/// A path's device and inode.
fn identity(path: &Path) -> (u64, u64) {
    let held = std::fs::metadata(path).unwrap();
    (held.dev(), held.ino())
}

/// The device and inode the handle `fd` holds.
fn held(fd: &OwnedFd) -> (u64, u64) {
    let held = std::fs::File::from(fd.try_clone().unwrap());
    let held = held.metadata().unwrap();
    (held.dev(), held.ino())
}

#[test]
fn the_store_is_admitted_by_its_identity_alone_through_a_handle_that_reads_nothing() {
    let Some(fixture) = stores() else {
        return;
    };
    let store = stored(&fixture, "store/secrets.env");
    let (seat, text) = (reach(&[], &[]), table());
    let answer = |store: &Path| store_on(store, &seat, &[], &text).map(|_| ());
    let fd = store_on(&store, &seat, &[], &text).unwrap();
    assert_eq!(held(&fd), identity(&store));
    let flags = rustix::fs::fcntl_getfl(&fd).unwrap();
    assert!(flags.contains(rustix::fs::OFlags::PATH));
    let read = rustix::io::read(&fd, &mut [0u8; 1]);
    assert_eq!(read, Err(rustix::io::Errno::BADF));
    // Absent, and not made; a directory; a FIFO, which is never opened;
    // a second link.
    let absent = fixture.path("store/absent.env");
    assert_eq!(answer(&absent), Err(Refusal::Identity));
    assert!(!absent.exists());
    assert_eq!(answer(&fixture.path("store")), Err(Refusal::Identity));
    let fifo = fixture.path("store/fifo.env");
    let (mode, kind) = (
        rustix::fs::Mode::from_raw_mode(0o600),
        rustix::fs::FileType::Fifo,
    );
    rustix::fs::mknodat(rustix::fs::CWD, &fifo, kind, mode, 0).unwrap();
    assert_eq!(answer(&fifo), Err(Refusal::Identity));
    let linked = stored(&fixture, "store/linked.env");
    std::fs::hard_link(&linked, fixture.path("store/again.env")).unwrap();
    assert_eq!(answer(&linked), Err(Refusal::Identity));
    // Another user's file, where this run observes its owner as another.
    let passwd = Path::new("/etc/passwd");
    let euid = rustix::process::geteuid().as_raw();
    if std::fs::metadata(passwd).unwrap().uid() != euid {
        assert_eq!(answer(passwd), Err(Refusal::Identity));
    }
}

#[test]
fn only_a_regular_file_of_one_link_its_owner_owns_is_a_store() {
    let owner = rustix::process::geteuid().as_raw();
    let other = owner.wrapping_add(1);
    assert!(single(0o100600, 1, owner, owner));
    // A directory of one link, as btrfs gives every directory; a second
    // link; another owner.
    assert!(!single(0o040700, 1, owner, owner));
    assert!(!single(0o100600, 2, owner, owner));
    assert!(!single(0o100600, 1, other, owner));
}

#[test]
fn a_store_whose_route_or_resolution_changes_refuses() {
    let Some(fixture) = stores() else {
        return;
    };
    let (seat, text) = (reach(&[], &[]), table());
    let mountinfo = || serving(&text);
    let answer = |store: &Path, host: &Host<'_>| {
        let held = admitted(store, &seat, std::iter::empty(), host);
        held.map(|_| ())
    };
    let plain = host(strangers(), &mountinfo);
    // Its directory moved away once the route reached the store, before
    // the store opens, and an empty one, or a link to it, put in its place:
    // the store opened is the one resolved, but a second resolution finds
    // nothing there, or finds it by another route.
    for (dir, linked) in [("emptied", false), ("relinked", true)] {
        let store = stored(&fixture, &format!("{dir}/secrets.env"));
        assert_eq!(answer(&store, &plain), Ok(()));
        let (at, away) = (fixture.path(dir), fixture.path(&format!("{dir}.old")));
        let moving = |_: &Path, name: &OsStr| {
            if name == "secrets.env" {
                std::fs::rename(&at, &away).unwrap();
                match linked {
                    true => std::os::unix::fs::symlink(&away, &at).unwrap(),
                    false => std::fs::create_dir(&at).unwrap(),
                }
            }
        };
        let host = Host {
            opening: &moving,
            ..host(strangers(), &mountinfo)
        };
        assert_eq!((dir, answer(&store, &host)), (dir, Err(Refusal::Identity)));
    }
    // Replaced at its name between the two resolutions: the same route,
    // another object.
    let store = stored(&fixture, "replaced/secrets.env");
    let replacing = || {
        std::fs::remove_file(&store).unwrap();
        std::fs::write(&store, "").unwrap();
    };
    let host = Host {
        between: &replacing,
        ..host(strangers(), &mountinfo)
    };
    assert_eq!(answer(&store, &host), Err(Refusal::Identity));
}

#[test]
fn a_store_whose_route_another_could_change_refuses() {
    let Some(fixture) = stores() else {
        return;
    };
    let (seat, text) = (reach(&[], &[]), table());
    let answer = |store: &Path| store_on(store, &seat, &[], &text).map(|_| ());
    let store = stored(&fixture, "store/secrets.env");
    assert_eq!(answer(&store), Ok(()));
    // Its directory writable by its group, by others, or sticky but this
    // user's; then an extended access ACL alone.
    let dir = fixture.path("store");
    for mode in [0o775, 0o757, 0o1777] {
        chmod(&dir, mode);
        assert_eq!((mode, answer(&store)), (mode, Err(Refusal::Identity)));
    }
    chmod(&dir, 0o755);
    assert_eq!(answer(&store), Ok(()));
    // Made writable by its group after its lookup, as its handle opens, and
    // closed again before the second resolution: its handle's facts judge.
    let mountinfo = || serving(&text);
    let opening = |_: &Path, name: &OsStr| {
        if name == "store" {
            chmod(&dir, 0o775);
        }
    };
    let closing = || chmod(&dir, 0o755);
    let host = Host {
        opening: &opening,
        between: &closing,
        ..host(strangers(), &mountinfo)
    };
    let opened = admitted(&store, &seat, std::iter::empty(), &host).map(|_| ());
    assert_eq!(opened, Err(Refusal::Identity));
    grant(&dir, (7, 5));
    let mode = std::fs::metadata(&dir).unwrap().mode();
    assert_eq!(
        (mode & 0o7777, answer(&store)),
        (0o755, Err(Refusal::Identity))
    );
    // A link on its route, whose mode means nothing.
    stored(&fixture, "plain/secrets.env");
    let via = fixture.link("via", &fixture.path("plain"));
    assert_eq!(answer(&via.join("secrets.env")), Ok(()));
}

#[test]
fn a_route_is_guarded_from_the_root_it_starts_from() {
    let owner = rustix::process::geteuid().as_raw();
    let owned = |uid, mode| Owned {
        uid,
        mode,
        acl: Acl::Absent,
    };
    let hop = |owned| Hop {
        place: PathBuf::from("/var"),
        identity: ((0, 0), 0),
        link: None,
        owned,
        mount: 0,
    };
    let (root, dir) = (owned(0, 0o040755), owned(owner, 0o040755));
    assert!(guarded(Some(root), &[hop(dir)], owner));
    // The root unread, or another's to change; a hop another's to change.
    assert!(!guarded(None, &[hop(dir)], owner));
    let other = owned(owner.wrapping_add(1), 0o040755);
    assert!(!guarded(Some(other), &[hop(dir)], owner));
    assert!(!guarded(Some(root), &[hop(owned(owner, 0o040777))], owner));
}

#[test]
fn a_hop_is_shut_only_where_none_but_its_owner_or_root_can_change_it() {
    let owner = rustix::process::geteuid().as_raw();
    let hop = |uid, mode, acl| shut(Owned { uid, mode, acl }, owner);
    let other = owner.wrapping_add(1);
    assert!(hop(owner, 0o040755, Acl::Absent));
    assert!(hop(0, 0o040755, Acl::Absent));
    // Root's sticky directory, as `/tmp` is; the same this user's or
    // another's, which its owner could still rename.
    assert!(hop(0, 0o041777, Acl::Absent));
    assert!(!hop(other, 0o041777, Acl::Absent));
    assert!(!hop(other, 0o040755, Acl::Absent));
    // Group or other write; an ACL present or unread.
    assert!(!hop(owner, 0o040775, Acl::Absent));
    assert!(!hop(owner, 0o040757, Acl::Absent));
    assert!(!hop(owner, 0o040755, Acl::Present));
    assert!(!hop(owner, 0o040755, Acl::Unknown));
    // A link's mode means nothing; its owner still counts.
    assert!(hop(owner, 0o120777, Acl::Absent));
    assert!(!hop(other, 0o120777, Acl::Absent));
}

/// The live mount table with an alias line added: the directory `dir`
/// mounted again at `point`, with `option`.
pub(super) fn aliased(dir: &Path, point: &Path, option: &str) -> Vec<u8> {
    let text = table();
    let parsed = records(&text, usize::MAX).unwrap();
    let id = mount_of(dir);
    let own = parsed.iter().find(|record| record.id == id).unwrap();
    let inside = own.root.join(dir.strip_prefix(&own.point).unwrap());
    let next = parsed.iter().map(|record| record.id).max().unwrap() + 1;
    let (major, minor) = own.dev;
    let line = format!(
        "{next} {id} {major}:{minor} {} {} {option} - ext4 /dev/x rw\n",
        escaped(&inside),
        escaped(point)
    );
    [text, line.into_bytes()].concat()
}

#[test]
fn the_store_is_exposed_by_its_route_and_every_place_the_mount_table_shows_it_at() {
    let Some(fixture) = stores() else {
        return;
    };
    let store = stored(&fixture, "store/secrets.env");
    let (dir, work, text) = (fixture.path("store"), fixture.path("work"), table());
    let none = reach(&[], &[]);
    let answer = |store: &Path, seat: &Reach, sources: &[PathBuf], text: &[u8]| {
        store_on(store, seat, sources, text).map(|_| ())
    };
    assert_eq!(answer(&store, &none, &[], &text), Ok(()));
    // Reach and sources beside it, below a directory on its route.
    let beside = reach(std::slice::from_ref(&work), &[fixture.path("cache")]);
    let sources = [fixture.path("opt/docs")];
    assert_eq!(answer(&store, &beside, &sources, &text), Ok(()));
    // In reach, read-only reach included, or under a source of the box.
    let (dirs, stores) = (std::slice::from_ref(&dir), std::slice::from_ref(&store));
    let readable = reach(&[], dirs);
    let reachable = Err(Refusal::StoreReachable);
    assert_eq!(answer(&store, &readable, &[], &text), reachable);
    let in_box = Err(Refusal::StoreInBox);
    assert_eq!(answer(&store, &none, dirs, &text), in_box);
    assert_eq!(answer(&store, &none, stores, &text), in_box);
    // Hands exposure comes first.
    assert_eq!(answer(&store, &readable, dirs, &text), reachable);
    // A route through writable reach, though it ends outside it.
    let link = fixture.link("work/store", Path::new("../store"));
    chmod(&work, 0o755);
    let routed = link.join("secrets.env");
    let writable = reach(std::slice::from_ref(&work), &[]);
    assert_eq!(answer(&routed, &writable, &[], &text), reachable);
    assert_eq!(answer(&routed, &none, &[], &text), Ok(()));
    // Its directory mounted again inside reach, read-only too, or inside a
    // source: each file there has one link and another canonical path.
    let into_reach = aliased(&dir, &work.join("alias"), "ro");
    assert_eq!(answer(&store, &writable, &[], &into_reach), reachable);
    let source = fixture.path("opt/docs");
    let into_box = aliased(&dir, &source.join("alias"), "rw");
    assert_eq!(answer(&store, &none, &sources, &into_box), in_box);
    assert_eq!(answer(&store, &none, &[], &into_box), Ok(()));
    // Its own mount unproved: not in the table, or no local filesystem;
    // uncertainty outranks a known exposure.
    let id = mount_of(&store);
    let gone = edited(&text, id, Vec::clear);
    let identity = Err(Refusal::Identity);
    assert_eq!(answer(&store, &none, &[], &gone), identity);
    assert_eq!(answer(&store, &readable, &[], &gone), identity);
    for edit in [overlay, moved] {
        let table = edited(&text, id, |fields| edit(fields));
        assert_eq!(answer(&store, &none, &[], &table), identity);
    }
}

#[test]
fn a_hop_of_the_route_or_the_hands_system_set_exposes_the_store() {
    let Some(fixture) = stores() else {
        return;
    };
    stored(&fixture, "store/secrets.env");
    let links = fixture.path("links");
    std::fs::create_dir(&links).unwrap();
    chmod(&links, 0o755);
    let routed = fixture.link("links/store", Path::new("../store"));
    let routed = routed.join("secrets.env");
    let (work, text) = (fixture.path("work"), table());
    let (none, writable) = (reach(&[], &[]), reach(std::slice::from_ref(&work), &[]));
    let answer = |seat: &Reach, text: &[u8]| store_on(&routed, seat, &[], text).map(|_| ());
    assert_eq!(answer(&writable, &text), Ok(()));
    // The directory holding the link on its route mounted again in reach:
    // the link shows there, and the store it names with it, though neither
    // the store nor its own directory is mounted anywhere else.
    let into_reach = aliased(&links, &work.join("links"), "ro");
    let reachable = Err(Refusal::StoreReachable);
    assert_eq!(answer(&writable, &into_reach), reachable);
    assert_eq!(answer(&none, &into_reach), Ok(()));
    // Its directory mounted again under `/etc/ssl` beside the `certs` a
    // server box keeps: the workspace hands bind all of `/etc/ssl`.
    let dir = fixture.path("store");
    let ssl = aliased(&dir, Path::new("/etc/ssl/brokkr-u6c5c"), "ro");
    assert_eq!(answer(&none, &ssl), reachable);
    let certs = aliased(&dir, Path::new("/etc/ssl/certs/brokkr-u6c5c"), "ro");
    assert_eq!(answer(&none, &certs), reachable);
    // Reached by a link that lies past `/dev`, which no local filesystem
    // holds: a hop's mount unproved, though the store's own is proved.
    let held = tempfile::tempdir_in("/dev/shm").unwrap();
    let shm = held.path().canonicalize().unwrap();
    chmod(&shm, 0o700);
    std::os::unix::fs::symlink(fixture.path("store"), shm.join("store")).unwrap();
    let past = shm.join("store/secrets.env");
    let unproved = store_on(&past, &none, &[], &text).map(|_| ());
    assert_eq!(unproved, Err(Refusal::Identity));
}

#[test]
fn a_prepared_box_holds_no_handle_of_the_store_it_admits() {
    if unservable() {
        return;
    }
    let Some(fixture) = stores() else {
        return;
    };
    let program = docs(&fixture);
    let bootstrap = std::env::current_exe().unwrap();
    let store = stored(&fixture, "store/secrets.env");
    let seat = reach(&[], &[]);
    let server = prepared_on(&program, &bootstrap, &confined()).unwrap();
    let fd = server.store(&store, &seat).unwrap();
    assert_eq!(held(&fd), identity(&store));
    let mounted: Vec<(u64, u64)> = server.handles().map(|(_, fd)| held(fd)).collect();
    assert!(!mounted.contains(&identity(&store)));
    let named = store.display().to_string();
    assert!(!server.argv().contains(&named));
    assert!(!server.argv().contains(&fd.as_fd().as_raw_fd().to_string()));
    // A store inside the package would be in the box.
    let inside = stored(&fixture, "opt/docs/secrets.env");
    let server = prepared_on(&program, &bootstrap, &confined()).unwrap();
    let answer = server.store(&inside, &seat).map(|_| ());
    assert_eq!(answer, Err(Refusal::StoreInBox));
}

#[test]
fn a_support_file_is_judged_by_its_handle_s_own_facts() {
    let (_fixture, lib, file) = linked_library();
    let mountinfo = || serving(&table());
    let (support, seat) = ([source(&lib, Role::Support)], reach(&[], &[]));
    let answer = |host: &Host<'_>| observe(&support, &seat, host).map(|_| ());
    assert_eq!(answer(&host(strangers(), &mountinfo)), Ok(()));
    // Made writable by its group after it was looked up, before it opens:
    // the lookup's mode would pass, the handle's does not.
    let opening = |_: &Path, name: &OsStr| {
        if name == "libx.so" {
            chmod(&file, 0o664);
        }
    };
    let host = Host {
        opening: &opening,
        ..host(strangers(), &mountinfo)
    };
    assert_eq!(answer(&host), Err(Refusal::Identity));
}

#[test]
fn a_parent_counts_only_while_it_stays_the_parent() {
    // `pid` 7's status names 3 as its parent, and 3's reads; then a
    // reader under which 7's parent has become 1 by the second read.
    let status = |pid: &str| match pid {
        "7" => Some("Name:\tseat\nPPid:\t3\n".to_string()),
        "3" => Some("Name:\tbroker\nPPid:\t1\n".to_string()),
        _ => None,
    };
    let held = parent("7", &status);
    assert_eq!(
        held,
        Some(("3".to_string(), "Name:\tbroker\nPPid:\t1\n".to_string()))
    );
    let reads = Cell::new(0);
    let orphaned = |pid: &str| {
        reads.set(reads.get() + usize::from(pid == "7"));
        match (pid, reads.get()) {
            ("7", 1) => Some("PPid:\t3\n".to_string()),
            ("7", _) => Some("PPid:\t1\n".to_string()),
            _ => status(pid),
        }
    };
    assert_eq!(parent("7", &orphaned), None);
    assert_eq!(reads.get(), 2);
    // No parent named, or its status unread.
    assert_eq!(parent("9", &status), None);
    let unread = |pid: &str| (pid == "7").then(|| "PPid:\t3\n".to_string());
    assert_eq!(parent("7", &unread), None);
}

#[test]
fn the_write_exclusion_proof_holds_against_every_writer_observed_and_sealed() {
    // The broker's status, and its harness's under another uid.
    let broker =
        "Uid:\t1000\t1000\t1000\t1000\nCapPrm:\t0\nCapEff:\t0\nCapAmb:\t0\nNoNewPrivs:\t1\n";
    let harness = broker.replace("1000", "1002");
    let observed = credited(Some("65534\n"), &[Some(broker), Some(&harness)]);
    let proved = Credentials {
        uids: vec![1000, 1002],
        overflow: 65534,
        confinement: Confinement::Proved,
    };
    assert_eq!(observed, proved);
    let file = |uid| Owned {
        uid,
        mode: 0o100755,
        acl: Acl::Absent,
    };
    assert!(excluded(&file(0), &observed));
    assert!(!excluded(&file(1002), &observed));
    // The sealed writers join them: what a sealed writer owns is no longer
    // excluded.
    let sealed = observed.sealed(&[1003, 1000]);
    assert_eq!(sealed.uids, [1000, 1002, 1003]);
    assert!(excluded(&file(1003), &observed));
    assert!(!excluded(&file(1003), &sealed));
    // A harness unconfined leaves every writer unconfined; one unread, or
    // none read at all, leaves no writer known, whoever is sealed.
    let unconfined = harness.replace("NoNewPrivs:\t1", "NoNewPrivs:\t0");
    let loose = credited(Some("65534\n"), &[Some(broker), Some(&unconfined)]);
    assert_eq!(loose.confinement, Confinement::Unproved);
    let unread = credited(Some("65534\n"), &[Some(broker), None]);
    let facts = (unread.uids.clone(), unread.overflow, unread.confinement);
    assert_eq!(facts, (Vec::new(), u32::MAX, Confinement::Unproved));
    assert!(!excluded(&file(0), &unread.sealed(&[1000])));
    let none = credited(Some("65534\n"), &[]);
    assert_eq!(none.confinement, Confinement::Unproved);
}
