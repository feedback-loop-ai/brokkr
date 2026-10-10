//! The box launcher (decision 0065 slice two, U6c5c; MB3; operator rulings
//! 2026-10-08 and 2026-10-09): admitted only as a protected ELF executable
//! in the system set whose loader inputs are protected too, read from its
//! headers exactly as the loader maps them without running it, by the
//! route it was found by beside its final inode, and run once admitted
//! through the handle the observation kept. A stand-in counts every run
//! the launcher is given, so each refusal shows no run. Every launcher a
//! positive control runs is the host's own bubblewrap, the one protected
//! launcher a test can name. An image planted here lies outside the system
//! set; a table test grants its fixture the set's membership (the
//! test-only `admitted`), so the stand-in counts a run for exactly the
//! images whose headers admit, and none is ever executed.

use std::cell::{Cell, RefCell};
use std::ffi::OsStr;
use std::io::Read;
use std::os::fd::{AsFd, BorrowedFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;

use super::super::namespace::sources::host::*;
use super::super::namespace::sources::launcher::{elf, fit, headers, protects, version};
use super::super::namespace::sources::launcher::{Input, Unfit};
use super::super::*;
use super::checked::{aliased, docs, mount_of};
use super::server::{unservable, Host as Fixture};
use super::sources::table;
use super::sources::{chmod, edited, moved, named, overlay, reach, serving, strangers};
use crate::broker::{Network, Reach, Refusal};

/// The ELF identification bytes (class, encoding, version) and machine of
/// this host, as the test binary's own header bears them.
fn native() -> ([u8; 3], [u8; 2]) {
    let mut header = [0; 20];
    let mut exe = std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
    exe.read_exact(&mut header).unwrap();
    ([header[4], header[5], header[6]], [header[18], header[19]])
}

/// The program interpreter this host's own programs name.
fn interpreter() -> Vec<u8> {
    let exe = std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
    headers(exe.as_fd()).unwrap().interpreter
}

/// The host's own bubblewrap, canonical, where it lies in the server box's
/// system set: the launcher every positive control runs. None, the proof
/// skipped as boundary evidence, where this host has none there.
pub(super) fn bubblewrap() -> Option<PathBuf> {
    let found = crate::hands::require_bwrap().ok();
    let found = found.and_then(|path| path.canonicalize().ok());
    let inside = |path: &PathBuf| {
        HOST_TOOLCHAIN_BINDS
            .iter()
            .any(|root| path.starts_with(root))
    };
    let found = found.filter(inside);
    if found.is_none() {
        let reason = "no bubblewrap lies in this host's system set";
        skip_boundary_proof(boundary_evidence_required(), reason);
    }
    found
}

/// Another protected ELF file than bubblewrap: this host's program
/// interpreter, canonical.
fn elsewhere() -> PathBuf {
    let named = PathBuf::from(OsStr::from_bytes(&interpreter()));
    named.canonicalize().unwrap()
}

/// What an ELF image built here names for the loader: its interpreter,
/// its search path strings by tag (`DT_RPATH` 15, `DT_RUNPATH` 29), its
/// needed names, dynamic entries after the string table's two, and program
/// headers after its own four, each its type, and its offset, address,
/// file-backed and loaded size. It is never run.
#[derive(Clone, Default)]
struct Image {
    interpreter: Vec<u8>,
    search: Vec<(u64, Vec<u8>)>,
    needed: Vec<Vec<u8>>,
    extra: Vec<(u64, u64)>,
    others: Vec<(u32, [u64; 4])>,
}

/// The places of an image's own four program headers.
const PHDR: usize = 64;
const INTERP: usize = 64 + 56;
const LOAD: usize = 64 + 2 * 56;
const DYNAMIC: usize = 64 + 3 * 56;

/// One program header: its type, and its offset, address, file-backed and
/// loaded size.
fn segment(kind: u32, fields: [u64; 4]) -> Vec<u8> {
    let mut bytes = vec![0; 56];
    bytes[..4].copy_from_slice(&kind.to_ne_bytes());
    for (at, value) in [8, 16, 32, 40].into_iter().zip(fields) {
        bytes[at..at + 8].copy_from_slice(&value.to_ne_bytes());
    }
    bytes
}

/// A segment's fields for `size` bytes at `at`, loaded at an address equal
/// to their place.
fn placed(at: usize, size: usize) -> [u64; 4] {
    let (at, size) = (at as u64, size as u64);
    [at, at, size, size]
}

impl Image {
    /// The image this host's programs have: its interpreter, and libc
    /// needed by name alone.
    fn host() -> Image {
        Image {
            interpreter: interpreter(),
            needed: vec![b"libc.so.6".to_vec()],
            ..Image::default()
        }
    }

    /// The host's image with a note after its own program headers, which
    /// a test may make another header of.
    fn noted() -> Image {
        Image {
            others: vec![(4, [0; 4])],
            ..Image::host()
        }
    }

    /// The image's bytes: the header, its own program headers (the table's,
    /// the interpreter's, one loaded segment over the whole image, and the
    /// dynamic segment), its other program headers, the interpreter's
    /// name, the string table and the dynamic entries, in that order.
    fn bytes(&self) -> Vec<u8> {
        let count = 4 + self.others.len();
        let interp_at = 64 + 56 * count;
        let interp = [&self.interpreter[..], &[0]].concat();
        let strings_at = interp_at + interp.len();
        let (mut strings, mut entries) = (vec![0], Vec::new());
        let named = self.needed.iter().map(|name| (1, name));
        for (tag, text) in named.chain(self.search.iter().map(|(tag, text)| (*tag, text))) {
            entries.push((tag, strings.len() as u64));
            strings.extend([&text[..], &[0]].concat());
        }
        let dynamic_at = strings_at + strings.len();
        entries.extend([(5, strings_at as u64), (10, strings.len() as u64)]);
        entries.extend(&self.extra);
        entries.push((0, 0));
        let end = dynamic_at + entries.len() * 16;
        let (ident, machine) = native();
        let mut bytes = b"\x7fELF".to_vec();
        bytes.extend(ident);
        bytes.resize(16, 0);
        bytes.extend(3u16.to_ne_bytes());
        bytes.extend(machine);
        bytes.extend(1u32.to_ne_bytes());
        bytes.extend([0u64, 64, 0].map(u64::to_ne_bytes).concat());
        bytes.extend(0u32.to_ne_bytes());
        bytes.extend(
            [64u16, 56, count as u16, 0, 0, 0]
                .map(u16::to_ne_bytes)
                .concat(),
        );
        bytes.extend(segment(6, placed(64, 56 * count)));
        bytes.extend(segment(3, placed(interp_at, interp.len())));
        bytes.extend(segment(1, placed(0, end)));
        bytes.extend(segment(2, placed(dynamic_at, entries.len() * 16)));
        for (kind, fields) in &self.others {
            bytes.extend(segment(*kind, *fields));
        }
        bytes.extend(interp);
        bytes.extend(strings);
        let words = entries.iter().flat_map(|(tag, value)| [*tag, *value]);
        bytes.extend(words.flat_map(u64::to_ne_bytes));
        bytes
    }
}

/// What the stand-in for a launcher's run reports: a version that mounts
/// descriptors.
const STAND_IN: &str = "bubblewrap 0.5.0";

/// A launcher image at `relative` under `fixture`, this host's, executable
/// and its owner's alone to write: whole, and outside the system set.
pub(super) fn launcher(fixture: &Fixture, relative: &str) -> PathBuf {
    planted(fixture, relative, Image::host().bytes())
}

/// `bytes` planted at `relative` under `fixture` as `0o755`.
fn planted(fixture: &Fixture, relative: &str, bytes: Vec<u8>) -> PathBuf {
    let path = fixture.plant(relative);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// A launcher test's ground: a fixture, the docs program planted in it,
/// and a stand-in for running a launcher, reporting [`STAND_IN`], that
/// counts each run it is given in `runs`.
struct Ground {
    fixture: Fixture,
    program: ServerProgram,
    runs: Rc<Cell<usize>>,
    stand: Box<dyn Fn(BorrowedFd<'_>) -> String>,
}

impl Ground {
    fn new() -> Ground {
        let fixture = Fixture::new();
        let program = docs(&fixture);
        let runs = Rc::new(Cell::new(0));
        let counted = Rc::clone(&runs);
        let stand = Box::new(move |_: BorrowedFd<'_>| {
            counted.set(counted.get() + 1);
            STAND_IN.to_string()
        });
        Ground {
            fixture,
            program,
            runs,
            stand,
        }
    }

    /// A ground where a box can be prepared: none in a seat's hands box,
    /// which maps the system's root-owned files to the overflow uid.
    fn served() -> Option<Ground> {
        (!unservable()).then(Ground::new)
    }

    /// A ground where a box can be prepared, and the host's own bubblewrap
    /// ([`bubblewrap`]) to launch it with.
    fn launched() -> Option<(Ground, PathBuf)> {
        let ground = Ground::served()?;
        Some((ground, bubblewrap()?))
    }

    /// This host, finding no launcher and running one with the stand-in,
    /// its writers strangers.
    fn host(&self) -> Host<'_> {
        Host {
            found: &|| None,
            run: &*self.stand,
            credentials: strangers(),
            ..Host::live()
        }
    }

    /// What preparing the box on `host` for the seat `seat` and the
    /// `sealed` writers answers, and how many more runs the stand-in
    /// counted meanwhile.
    fn refusing(&self, host: &Host<'_>, seat: &Reach, sealed: &[u32]) -> Ran {
        let before = self.runs.get();
        let answer = launching(&self.program, host, seat, sealed);
        (answer, self.runs.get() - before)
    }
}

/// Whether the box `program` is prepared in on `host` for the seat `seat`
/// and the `sealed` writers, its bootstrap this test binary.
fn launching(
    program: &ServerProgram,
    host: &Host<'_>,
    seat: &Reach,
    sealed: &[u32],
) -> Result<(), Refusal> {
    let bootstrap = std::env::current_exe().unwrap();
    let profile = ServerProfile {
        reach: seat,
        network: &Network::Isolated,
        bootstrap: &bootstrap,
        writers: sealed,
    };
    ServerBox::prepare_with(program, &profile, host).map(|_| ())
}

/// Counting each time it runs, run `step` on its `at`th time.
fn on_call(calls: &Cell<usize>, at: usize, step: impl Fn()) {
    calls.set(calls.get() + 1);
    if calls.get() == at {
        step();
    }
}

/// What preparing a box answered, and how many times its launcher ran.
type Ran = (Result<(), Refusal>, usize);

/// The cases of `answered`, each expected to refuse `cause` with no run.
fn unrun<'c>(answered: &[(&'c str, Ran)], cause: impl Fn(&str) -> Refusal) -> Vec<(&'c str, Ran)> {
    let expected = answered
        .iter()
        .map(|(case, _)| (*case, (Err(cause(case)), 0)));
    expected.collect()
}

#[test]
fn the_launcher_runs_once_admitted_through_the_handle_kept() {
    let Some((ground, bwrap)) = Ground::launched() else {
        return;
    };
    let (fixture, program) = (&ground.fixture, &ground.program);
    // The host's own bubblewrap, found through a link beside the seat's
    // reach and run as the live host runs it.
    let link = fixture.link("launcher/bwrap", &bwrap);
    let reports = RefCell::new(Vec::new());
    let run = |fd: BorrowedFd<'_>| {
        let report = version(fd);
        reports.borrow_mut().push(report.clone());
        report
    };
    let (seat, found) = (reach(&[], &[]), named(&link));
    let live = Host {
        found: &found,
        run: &run,
        ..ground.host()
    };
    let admitted = launching(program, &live, &seat, &[]);
    // Its link renamed over by one to another protected ELF file once its
    // admission is complete, just before it runs (the second step between,
    // the first lying between the observation's two resolutions): the
    // object admitted runs, through the handle kept, and reports as
    // bubblewrap.
    let other = fixture.link("other/bwrap", &elsewhere());
    let calls = Cell::new(0);
    let swap = || on_call(&calls, 2, || std::fs::rename(&other, &link).unwrap());
    let swapping = Host {
        between: &swap,
        found: &found,
        run: &run,
        ..ground.host()
    };
    let again = launching(program, &swapping, &seat, &[]);
    let reports = reports.take();
    let bubblewrap = reports
        .iter()
        .all(|report| report.starts_with("bubblewrap "));
    assert_eq!(
        (admitted, again, reports.len(), bubblewrap, calls.get()),
        (Ok(()), Ok(()), 2, true, 2)
    );
}

#[test]
fn a_launcher_outside_the_system_set_never_runs() {
    let Some(ground) = Ground::served() else {
        return;
    };
    // A whole image of this host's, every input it names protected and no
    // managed writer able to write it, under the build's own directory.
    let bwrap = launcher(&ground.fixture, "launcher/bwrap");
    let file = std::fs::File::open(&bwrap).unwrap();
    let typed = elf(file.as_fd(), bwrap.parent().unwrap(), &ground.host());
    let found = named(&bwrap);
    let host = Host {
        found: &found,
        ..ground.host()
    };
    let answer = ground.refusing(&host, &reach(&[], &[]), &[]);
    assert_eq!((typed, answer), (Ok(()), (Err(Refusal::Identity), 0)));
}

/// The first script `/usr/bin` holds by name, root's and writable by no
/// one else: a launcher in the system set whose headers alone refuse it.
/// None, the proof skipped as boundary evidence, where it holds none.
fn script() -> Option<PathBuf> {
    let mut names: Vec<PathBuf> = std::fs::read_dir("/usr/bin")
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    names.sort();
    let shut = |path: &PathBuf| {
        let held = std::fs::symlink_metadata(path).unwrap();
        held.is_file() & (held.uid() == 0) & (held.mode() & 0o022 == 0)
    };
    let scripted = |path: &PathBuf| {
        let mut start = [0; 2];
        let read = std::fs::File::open(path).and_then(|mut file| file.read_exact(&mut start));
        read.is_ok() & (start == *b"#!")
    };
    let script = names.into_iter().filter(shut).find(scripted);
    if script.is_none() {
        let reason = "/usr/bin holds no root-owned script";
        skip_boundary_proof(boundary_evidence_required(), reason);
    }
    script
}

#[test]
fn a_script_in_the_system_set_never_runs() {
    let Some(ground) = Ground::served() else {
        return;
    };
    let Some(script) = script() else {
        return;
    };
    let file = std::fs::File::open(&script).unwrap();
    let typed = elf(file.as_fd(), script.parent().unwrap(), &ground.host());
    let found = named(&script);
    let host = Host {
        found: &found,
        ..ground.host()
    };
    let answer = ground.refusing(&host, &reach(&[], &[]), &[]);
    let refused = (Err(Unfit::Script), (Err(Refusal::Identity), 0));
    assert_eq!((typed, answer), refused, "{script:?}");
}

#[test]
fn a_launcher_is_fit_only_as_a_regular_file_in_the_system_set_no_writer_could_write() {
    let (bin, opt) = (Path::new("/usr/bin/bwrap"), Path::new("/opt/bwrap"));
    let root = Owned {
        uid: 0,
        mode: 0o100_755,
        acl: Acl::Absent,
    };
    let judge =
        |owned: Owned, place: &Path, writers: Credentials| fit(&owned, place, &writers, false);
    let (directory, grouped) = (
        Owned {
            mode: 0o040_755,
            ..root
        },
        Owned {
            mode: 0o100_775,
            ..root
        },
    );
    let rooted = Credentials {
        uids: vec![0],
        ..strangers()
    };
    let unconfined = Credentials {
        confinement: Confinement::Unproved,
        ..strangers()
    };
    let cases = [
        ("fit", judge(root, bin, strangers())),
        ("directory", judge(directory, bin, strangers())),
        ("outside", judge(root, opt, strangers())),
        ("grouped", judge(grouped, bin, strangers())),
        ("observed", judge(root, bin, rooted)),
        ("unconfined", judge(root, bin, unconfined)),
    ];
    let refused = ["directory", "outside", "grouped", "observed", "unconfined"];
    let expected = cases.map(|(case, _)| (case, !refused.contains(&case)));
    assert_eq!(cases, expected);
}

#[test]
fn a_launcher_its_own_check_refuses_never_runs() {
    let Some((ground, bwrap)) = Ground::launched() else {
        return;
    };
    let fixture = &ground.fixture;
    let (seat, found) = (reach(&[], &[]), named(&bwrap));
    let refused = |host: &Host<'_>, sealed: &[u32]| ground.refusing(host, &seat, sealed);
    let with = |credentials| Host {
        credentials,
        found: &found,
        ..ground.host()
    };
    // Writable by a managed writer: root among those observed or those
    // sealed, or its writers unconfined.
    let rooted = Credentials {
        uids: vec![0],
        ..strangers()
    };
    let mut cases = vec![
        ("observed", refused(&with(rooted), &[])),
        ("sealed", refused(&with(strangers()), &[0])),
    ];
    let unconfined = Credentials {
        confinement: Confinement::Unproved,
        ..strangers()
    };
    cases.push(("unconfined", refused(&with(unconfined), &[])));
    // No regular file at all, though no writer could write it.
    let directory = named(bwrap.parent().unwrap());
    let dir_host = Host {
        found: &directory,
        ..ground.host()
    };
    cases.push(("directory", refused(&dir_host, &[])));
    // Found through a link replaced as its handle opens; what the new link
    // names never runs either.
    let link = fixture.link("launcher/bwrap", &bwrap);
    let other = fixture.link("other/bwrap", &elsewhere());
    let replacing = |dir: &Path, name: &OsStr| {
        if (dir == fixture.path("launcher")) & (name == "bwrap") {
            std::fs::rename(&other, &link).ok();
        }
    };
    let linked = named(&link);
    let replaced = Host {
        opening: &replacing,
        found: &linked,
        ..ground.host()
    };
    cases.push(("replaced", refused(&replaced, &[])));
    assert_eq!(cases, unrun(&cases, |_| Refusal::Identity));
}

#[test]
fn a_launcher_its_observation_refuses_never_runs() {
    let Some((ground, bwrap)) = Ground::launched() else {
        return;
    };
    let fixture = &ground.fixture;
    let link = fixture.link("launcher/bwrap", &bwrap);
    let found = named(&link);
    let refused =
        |text: &[u8], seat: &Reach, between: &dyn Fn(), opening: &dyn Fn(&Path, &OsStr)| {
            let mountinfo = || serving(text);
            let host = Host {
                mountinfo: &mountinfo,
                between,
                opening,
                found: &found,
                ..ground.host()
            };
            ground.refusing(&host, seat, &[])
        };
    let (text, none, alone) = (table(), reach(&[], &[]), |_: &Path, _: &OsStr| {});
    // On a route through the seat's reach, read-only reach included.
    let held = reach(&[], &[fixture.path("launcher")]);
    let mut cases = vec![("reach", refused(&text, &held, &|| {}, &alone))];
    // Its route's mount unproved: no local filesystem, or another device.
    let id = mount_of(&fixture.path("launcher"));
    for (case, edit) in [("overlay", overlay as fn(&mut [String])), ("device", moved)] {
        let unproved = edited(&text, id, |fields| edit(fields));
        cases.push((case, refused(&unproved, &none, &|| {}, &alone)));
    }
    // The directory holding its link mounted again inside writable reach.
    let work = fixture.path("work");
    let writable = reach(std::slice::from_ref(&work), &[]);
    let into_reach = aliased(&fixture.path("launcher"), &work.join("alias"), "ro");
    cases.push(("alias", refused(&into_reach, &writable, &|| {}, &alone)));
    // Its link swapped for one naming another protected ELF file once it is
    // checked, before the observation resolves it, and put back before the
    // launcher's last resolution (the fourth open of its directory): the
    // observation holds the other file by the very route checked, so only
    // the kept handle's identity tells them apart.
    let (aside, later) = (
        fixture.path("aside"),
        fixture.link("later/bwrap", &elsewhere()),
    );
    let calls = Cell::new(0);
    let swapping = |dir: &Path, name: &OsStr| {
        if (dir == fixture.root) & (name == "launcher") {
            on_call(&calls, 2, || {
                std::fs::rename(&link, &aside).unwrap();
                std::fs::rename(&later, &link).unwrap();
            });
            if calls.get() == 4 {
                std::fs::rename(&link, &later).unwrap();
                std::fs::rename(&aside, &link).unwrap();
            }
        }
    };
    cases.push(("swapped", refused(&text, &none, &|| {}, &swapping)));
    // Its directory moved away between the two resolutions and the same
    // link put back at its place.
    let dir = fixture.path("launcher");
    let moves = Cell::new(0);
    let rebuilt = || {
        on_call(&moves, 1, || {
            std::fs::rename(&dir, fixture.path("moved")).unwrap();
            fixture.link("launcher/bwrap", &bwrap);
        })
    };
    cases.push(("ancestry", refused(&text, &none, &rebuilt, &alone)));
    let expected = unrun(&cases, |case| match case {
        "reach" | "alias" => Refusal::BindOverlapsReach,
        _ => Refusal::Identity,
    });
    assert_eq!(cases, expected);
    assert_eq!((calls.get(), moves.get()), (4, 1));
}

#[test]
fn the_route_a_launcher_was_found_by_is_admitted_before_it_runs() {
    let Some((ground, bwrap)) = Ground::launched() else {
        return;
    };
    let fixture = &ground.fixture;
    fixture.link("launcher/bwrap", &bwrap);
    let (work, links) = (fixture.path("work"), fixture.path("links"));
    std::fs::create_dir_all(&work).unwrap();
    let writable = reach(std::slice::from_ref(&work), &[]);
    let answer = |found: &Path, text: &[u8], seat: &Reach| {
        let (found, mountinfo) = (named(found), || serving(text));
        let host = Host {
            mountinfo: &mountinfo,
            found: &found,
            ..ground.host()
        };
        ground.refusing(&host, seat, &[])
    };
    // Found through links beside reach, that reach admits and runs.
    let linked = fixture.link("links/bwrap", Path::new("../launcher/bwrap"));
    assert_eq!(answer(&linked, &table(), &writable), (Ok(()), 1));
    // Found through a link inside writable reach to the same launcher,
    // whose final place lies outside it.
    let through = fixture.link("work/bwrap", Path::new("../launcher/bwrap"));
    let mut cases = vec![("reach", answer(&through, &table(), &writable))];
    // The directory holding the link it was found by mounted again inside
    // reach, though the launcher's own directory is mounted nowhere else.
    let into_reach = aliased(&links, &work.join("links"), "ro");
    cases.push(("alias", answer(&linked, &into_reach, &writable)));
    // Found through a link past `/dev`, which no local filesystem holds: a
    // hop's mount unproved, though the launcher's own is proved.
    let held = tempfile::tempdir_in("/dev/shm").unwrap();
    let shm = held.path().canonicalize().unwrap();
    std::os::unix::fs::symlink(fixture.path("launcher"), shm.join("launcher")).unwrap();
    let past = shm.join("launcher/bwrap");
    cases.push(("unproved", answer(&past, &table(), &reach(&[], &[]))));
    let expected = unrun(&cases, |case| match case {
        "unproved" => Refusal::Identity,
        _ => Refusal::BindOverlapsReach,
    });
    assert_eq!(cases, expected);
}

#[test]
fn a_route_that_changes_once_admitted_refuses_before_the_launcher_runs() {
    let Some((ground, bwrap)) = Ground::launched() else {
        return;
    };
    let fixture = &ground.fixture;
    let link = fixture.link("launcher/bwrap", &bwrap);
    let (dir, away) = (fixture.path("launcher"), fixture.path("away"));
    // Once the observation is admitted, as the launcher's route is proved
    // (the mount table's second read), its directory is moved away and a
    // link to it put in its place: the same launcher, by another route.
    let reads = Cell::new(0);
    let mountinfo = || {
        on_call(&reads, 2, || {
            std::fs::rename(&dir, &away).unwrap();
            std::os::unix::fs::symlink(&away, &dir).unwrap();
        });
        serving(&table())
    };
    let found = named(&link);
    let host = Host {
        mountinfo: &mountinfo,
        found: &found,
        ..ground.host()
    };
    let answer = ground.refusing(&host, &reach(&[], &[]), &[]);
    assert_eq!((answer, reads.get()), ((Err(Refusal::Identity), 0), 2));
}

/// Each case planted as a launcher on `ground` and judged on `host`: what
/// [`elf`] answers for it read directly, and what preparing the ground's
/// box with it found answers, with the runs the stand-in counted. The
/// fixture is granted the system set's membership (`admitted`), so its
/// headers alone decide whether the stand-in runs.
fn judged(
    ground: &Ground,
    cases: &[Case],
    host: &Host<'_>,
) -> Vec<(&'static str, Result<(), Unfit>, Ran)> {
    let granted = |place: &Path| place.starts_with(&ground.fixture.root);
    let judge = |(case, bytes, _): &Case| {
        let path = planted(&ground.fixture, &format!("{case}/bwrap"), bytes.clone());
        let file = std::fs::File::open(&path).unwrap();
        let typed = elf(file.as_fd(), path.parent().unwrap(), host);
        let found = named(&path);
        let host = Host {
            found: &found,
            credentials: host.credentials.clone(),
            admitted: &granted,
            ..*host
        };
        (*case, typed, ground.refusing(&host, &reach(&[], &[]), &[]))
    };
    cases.iter().map(judge).collect()
}

/// What each case of `cases` is expected to answer: its typed reading;
/// and, prepared, a run where it admits and a refusal with none where it
/// does not.
fn expected(cases: &[Case]) -> Vec<(&'static str, Result<(), Unfit>, Ran)> {
    let prepared = |typed: &Result<(), Unfit>| match typed {
        Ok(()) => (Ok(()), 1),
        Err(_) => (Err(Refusal::Identity), 0),
    };
    let expect = |(case, _, typed): &Case| (*case, *typed, prepared(typed));
    cases.iter().map(expect).collect()
}

/// Each case [`judged`] on `host` as [`expected`], every row that differs
/// printed beside the row expected.
fn agreed(ground: &Ground, cases: &[Case], host: &Host<'_>) {
    let (got, want) = (judged(ground, cases, host), expected(cases));
    let pairs = got.iter().zip(&want);
    let wrong: Vec<_> = pairs.filter(|(got, want)| got != want).collect();
    assert_eq!((got.len(), wrong), (want.len(), Vec::new()));
}

/// `value`'s bytes written into `bytes` at `at`.
fn put(bytes: &mut [u8], at: usize, value: &[u8]) {
    bytes[at..at + value.len()].copy_from_slice(value);
}

/// The 64-bit field at `at` of `bytes`.
fn long(bytes: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(bytes[at..at + 8].try_into().unwrap())
}

/// The place of the `nth` program header of an image.
fn header(nth: usize) -> usize {
    64 + 56 * nth
}

#[test]
fn only_a_whole_elf_executable_of_this_host_is_read_and_never_run() {
    let Some(ground) = Ground::served() else {
        return;
    };
    let good = Image::noted().bytes();
    let cases = [identification(&good), own(&good)].concat();
    agreed(&ground, &cases, &ground.host());
}

/// One image a table test plants, and what [`elf`] answers for it.
type Case = (&'static str, Vec<u8>, Result<(), Unfit>);

/// `good` with `value` written at `at`.
fn patched(good: &[u8], at: usize, value: &[u8]) -> Vec<u8> {
    let mut bytes = good.to_vec();
    put(&mut bytes, at, value);
    bytes
}

/// `good` with its program header at `from` copied over the one at `to`.
fn copied(good: &[u8], from: usize, to: usize) -> Vec<u8> {
    patched(good, to, &good[from..from + 56])
}

/// `good` with its program headers at `one` and `other` swapped.
fn swapped(good: &[u8], one: usize, other: usize) -> Vec<u8> {
    patched(&copied(good, one, other), one, &good[other..other + 56])
}

/// The image `good`, then images whose identification or program header
/// table is wrong (the ruling's items 1 and 2): a script, no ELF, a short
/// header, another class, encoding, version, machine or type, another
/// entry size, none, too many, cut short, past the file or wrapping; and
/// an executable, which admits as a shared object does.
fn identification(good: &[u8]) -> Vec<Case> {
    let edit = |at, value: &[u8]| patched(good, at, value);
    let (foreign, malformed) = (Err(Unfit::Foreign), Err(Unfit::Malformed));
    let half = |value: u16| value.to_ne_bytes();
    vec![
        ("elf", good.to_vec(), Ok(())),
        (
            "script",
            b"#!/bin/sh\necho bubblewrap 0.5.0\n".to_vec(),
            Err(Unfit::Script),
        ),
        ("magic", edit(0, b"\x7fELG"), Err(Unfit::NotElf)),
        ("short", good[..20].to_vec(), Err(Unfit::Truncated)),
        ("class", edit(4, &[1]), foreign),
        ("encoding", edit(5, &[3 - good[5]]), foreign),
        ("version", edit(6, &[2]), foreign),
        ("machine", edit(18, &[0xfe, 0xca]), foreign),
        ("type", edit(16, &half(1)), foreign),
        ("executable", edit(16, &half(2)), Ok(())),
        ("entry-size", edit(54, &half(55)), malformed),
        ("no-headers", edit(56, &half(0)), malformed),
        ("headers-over", edit(56, &half(65)), Err(Unfit::Over)),
        (
            "headers-cut",
            good[..header(2)].to_vec(),
            Err(Unfit::Truncated),
        ),
        (
            "headers-past-file",
            edit(32, &(good.len() as u64).to_ne_bytes()),
            Err(Unfit::Truncated),
        ),
        (
            "headers-wrapping",
            edit(32, &(u64::MAX - 8).to_ne_bytes()),
            malformed,
        ),
    ]
}

/// Images of `good` whose `PT_PHDR` is not the one mapped table before
/// every load (item 3): none, two, after the load, at another offset than
/// the table's, shorter than it, unmapped, mapped by another translation,
/// or before its load's bytes; or no load to map it at all.
fn own(good: &[u8]) -> Vec<Case> {
    let edit = |at, value: u64| patched(good, at, &value.to_ne_bytes());
    let mut elsewhere = edit(PHDR + 8, 72);
    put(&mut elsewhere, PHDR + 16, &72u64.to_ne_bytes());
    let size = long(good, PHDR + 32);
    // The load begun after the table, at the interpreter's name: all else
    // it maps, the table nothing does.
    let (start, end) = (long(good, INTERP + 8), long(good, LOAD + 32));
    let mut unloaded = good.to_vec();
    for (at, value) in [
        (8, start),
        (16, start),
        (32, end - start),
        (40, end - start),
    ] {
        put(&mut unloaded, LOAD + at, &value.to_ne_bytes());
    }
    let malformed = Err(Unfit::Malformed);
    vec![
        (
            "phdr-missing",
            patched(good, PHDR, &4u32.to_ne_bytes()),
            malformed,
        ),
        ("phdr-twice", copied(good, PHDR, header(4)), malformed),
        ("phdr-after-load", swapped(good, PHDR, header(4)), malformed),
        ("phdr-elsewhere", elsewhere, malformed),
        ("phdr-short", edit(PHDR + 32, size - 56), malformed),
        ("phdr-unmapped", edit(PHDR + 16, 1 << 20), malformed),
        ("phdr-shifted", edit(PHDR + 16, 72), malformed),
        ("phdr-unloaded", unloaded, malformed),
        (
            "no-loads",
            patched(good, LOAD, &4u32.to_ne_bytes()),
            malformed,
        ),
    ]
}

#[test]
fn a_launcher_is_read_exactly_as_the_loader_maps_it() {
    let Some(ground) = Ground::served() else {
        return;
    };
    let good = Image::host().bytes();
    let cases = [loads(&good), mapped(&good)].concat();
    agreed(&ground, &cases, &ground.host());
}

/// Images of `good` given further loaded segments (item 4), apart and
/// aligned, misaligned, out of order, overlapping, on its own page,
/// sharing file bytes with it next to it or past another, or past the
/// address space or the file's offsets; and with its own segment past the
/// file, or file-backed past its loaded size.
fn loads(good: &[u8]) -> Vec<Case> {
    let with = |loads: &[[u64; 4]]| {
        let image = Image {
            others: loads.iter().map(|load| (1, *load)).collect(),
            ..Image::host()
        };
        image.bytes()
    };
    let aligned = |load, align: u64| patched(&with(&[load]), header(4) + 48, &align.to_ne_bytes());
    let at = |address: u64| [0, address, 0, 16];
    let file = long(good, LOAD + 32);
    let mut past = patched(good, LOAD + 32, &(file + 8).to_ne_bytes());
    put(&mut past, LOAD + 40, &(file + 8).to_ne_bytes());
    let malformed = Err(Unfit::Malformed);
    vec![
        ("loads-apart", with(&[at(1 << 20)]), Ok(())),
        ("loads-aligned", aligned(at(1 << 20), 4096), Ok(())),
        (
            "load-unaligned",
            aligned(at((1 << 20) + 16), 4096),
            malformed,
        ),
        (
            "loads-unordered",
            with(&[at(2 << 20), at(1 << 20)]),
            malformed,
        ),
        ("loads-overlapping", with(&[at(16)]), malformed),
        // Past the image's own segment, on the page it ends on.
        ("loads-sharing-a-page", with(&[at(0xf00)]), malformed),
        (
            "loads-sharing-bytes",
            with(&[[0, 1 << 20, 16, 16]]),
            malformed,
        ),
        // Its bytes shared with the image's own segment, a segment with no
        // file-backed bytes between them.
        (
            "loads-sharing-bytes-apart",
            with(&[at(1 << 20), [8, 2 << 20, 8, 16]]),
            malformed,
        ),
        ("load-wrapping", with(&[at(u64::MAX - 8)]), malformed),
        (
            "load-offset-wrapping",
            with(&[[u64::MAX, 1 << 20, 1, 16]]),
            malformed,
        ),
        ("load-past-file", past, Err(Unfit::Truncated)),
        (
            "load-file-over-memory",
            patched(good, LOAD + 40, &(file - 1).to_ne_bytes()),
            malformed,
        ),
    ]
}

/// Images of `good` whose dynamic segment or string table the loader maps
/// from other bytes than those read (items 6 and 7), or with a name the
/// table does not end or that passes its bound (item 8).
fn mapped(good: &[u8]) -> Vec<Case> {
    let edit = |at, value: u64| patched(good, at, &value.to_ne_bytes());
    let (address, file) = (long(good, DYNAMIC + 16), long(good, LOAD + 32));
    let entries = long(good, DYNAMIC + 8) as usize;
    let size_at = entries + 2 * 16 + 8;
    let table = long(good, entries + 16 + 8);
    // The table's size run one byte past the file-backed bytes, which the
    // loaded size goes a page beyond.
    let mut beyond = edit(LOAD + 40, file + 4096);
    put(&mut beyond, size_at, &(file - table + 1).to_ne_bytes());
    // The table run past every load into bytes the file holds after them,
    // each name it holds still read whole.
    let mut unloaded = [good, &[0; 64]].concat();
    put(&mut unloaded, size_at, &(file + 64 - table).to_ne_bytes());
    let named_long = Image {
        needed: vec![vec![b'a'; 5000]],
        ..Image::host()
    };
    let malformed = Err(Unfit::Malformed);
    vec![
        ("dynamic-unmapped", edit(DYNAMIC + 16, 1 << 20), malformed),
        (
            "dynamic-shifted",
            edit(DYNAMIC + 16, address - 16),
            malformed,
        ),
        ("dynamic-past-file", edit(LOAD + 32, address + 8), malformed),
        ("table-crossing", edit(size_at, file - table + 1), malformed),
        ("table-past-file", beyond, malformed),
        ("table-unloaded", unloaded, malformed),
        ("name-unended-in-table", edit(size_at, 4), malformed),
        // The string table one byte long: libc's name lies past its end.
        ("name-past-table", edit(size_at, 1), malformed),
        ("name-over", named_long.bytes(), Err(Unfit::Over)),
    ]
}

#[test]
fn a_launcher_names_one_interpreter_one_dynamic_segment_and_one_string_table() {
    let Some(ground) = Ground::served() else {
        return;
    };
    let good = Image::noted().bytes();
    let cases = [segments(&good), tags(&good)].concat();
    agreed(&ground, &cases, &ground.host());
}

/// Images of `good`, its fifth program header a note, with no interpreter
/// or no dynamic segment (item 9), the note far past the file and unread,
/// or whose interpreter (item 5) or dynamic segment (item 6) is wrong:
/// two, after the load, unended, holding a NUL, empty, over its bound or
/// cut; or uneven, over its bound or unended.
fn segments(good: &[u8]) -> Vec<Case> {
    let edit = |at, value: &[u8]| patched(good, at, value);
    let interp_end = (long(good, INTERP + 8) + long(good, INTERP + 32) - 1) as usize;
    let entries = long(good, DYNAMIC + 8) as usize;
    let null = entries + long(good, DYNAMIC + 32) as usize - 16;
    // The dynamic segment eight bytes longer, as the loaded one and the
    // image are, so only its size's evenness is wrong.
    let mut uneven = [good, &[0; 8]].concat();
    for at in [DYNAMIC + 32, LOAD + 32, LOAD + 40] {
        put(&mut uneven, at, &(long(good, at) + 8).to_ne_bytes());
    }
    // The interpreter's name a lone NUL, the header's padding.
    let mut empty = edit(INTERP + 8, &9u64.to_ne_bytes());
    put(&mut empty, INTERP + 32, &1u64.to_ne_bytes());
    let mut unread = edit(header(4) + 8, &(u64::MAX - 1).to_ne_bytes());
    put(&mut unread, header(4) + 32, &16u64.to_ne_bytes());
    let note = 4u32.to_ne_bytes();
    let (malformed, over) = (Err(Unfit::Malformed), Err(Unfit::Over));
    vec![
        (
            "static-uninterpreted",
            edit(INTERP, &note),
            Err(Unfit::Static),
        ),
        ("static-undynamic", edit(DYNAMIC, &note), Err(Unfit::Static)),
        ("other-unread", unread, Ok(())),
        (
            "two-interpreters",
            copied(good, INTERP, header(4)),
            malformed,
        ),
        (
            "interpreter-after-load",
            swapped(good, INTERP, header(4)),
            malformed,
        ),
        ("interpreter-unended", edit(interp_end, b"x"), malformed),
        (
            "interpreter-holding-nul",
            edit(interp_end - 2, &[0]),
            malformed,
        ),
        ("interpreter-empty", empty, malformed),
        (
            "interpreter-over",
            edit(INTERP + 32, &4097u64.to_ne_bytes()),
            over,
        ),
        (
            "interpreter-cut",
            edit(INTERP + 8, &(1u64 << 40).to_ne_bytes()),
            Err(Unfit::Truncated),
        ),
        (
            "interpreter-wrapping",
            edit(INTERP + 8, &(u64::MAX - 8).to_ne_bytes()),
            malformed,
        ),
        ("two-dynamics", copied(good, DYNAMIC, header(4)), malformed),
        ("dynamic-uneven", uneven, malformed),
        (
            "dynamic-over",
            edit(DYNAMIC + 32, &(513u64 * 16).to_ne_bytes()),
            over,
        ),
        (
            "dynamic-unended",
            edit(null, &21u64.to_ne_bytes()),
            malformed,
        ),
    ]
}

/// Images of `good` whose dynamic entries before `DT_NULL` are wrong (item
/// 7), and two right: an audit entry after it, unread, and an image naming
/// no library whose table is whole. The string table or its size twice,
/// equal or not, or missing, with names or without; a table naming
/// nothing unmapped; and each entry delegating to another library.
fn tags(good: &[u8]) -> Vec<Case> {
    let entries = long(good, DYNAMIC + 8) as usize;
    let (table, size) = (long(good, entries + 16 + 8), long(good, entries + 32 + 8));
    let with = |extra: &[(u64, u64)]| {
        let image = Image {
            extra: extra.to_vec(),
            ..Image::noted()
        };
        image.bytes()
    };
    let bare = Image {
        needed: Vec::new(),
        ..Image::noted()
    };
    let bare = bare.bytes();
    let at = long(&bare, DYNAMIC + 8) as usize;
    let debug = 21u64.to_ne_bytes();
    let (malformed, audited) = (Err(Unfit::Malformed), Err(Unfit::Audited));
    vec![
        ("after-null", with(&[(0, 0), (0x6fff_fefc, 0)]), Ok(())),
        ("table-twice", with(&[(5, table)]), malformed),
        ("table-conflicting", with(&[(5, table + 1)]), malformed),
        ("size-twice", with(&[(10, size)]), malformed),
        ("size-conflicting", with(&[(10, size + 1)]), malformed),
        (
            "table-missing",
            patched(good, entries + 16, &debug),
            malformed,
        ),
        (
            "size-missing",
            patched(good, entries + 32, &debug),
            malformed,
        ),
        ("bare", bare.clone(), Ok(())),
        ("bare-table-missing", patched(&bare, at, &debug), malformed),
        (
            "bare-table-unmapped",
            patched(&bare, at + 8, &(1u64 << 20).to_ne_bytes()),
            malformed,
        ),
        ("depaudit", with(&[(0x6fff_fefb, 0)]), audited),
        ("audit", with(&[(0x6fff_fefc, 0)]), audited),
        ("auxiliary", with(&[(0x7fff_fffd, 0)]), audited),
        ("filter", with(&[(0x7fff_ffff, 0)]), audited),
    ]
}

#[test]
fn a_launcher_that_cannot_be_read_is_never_run() {
    if rustix::process::geteuid().is_root() {
        let reason = "euid 0 reads every file, so no launcher can be made unreadable";
        return skip_boundary_proof(boundary_evidence_required(), reason);
    }
    let ground = Ground::new();
    let bwrap = launcher(&ground.fixture, "launcher/bwrap");
    chmod(&bwrap, 0o311);
    let found = named(&bwrap);
    let host = Host {
        found: &found,
        ..ground.host()
    };
    let held = rustix::fs::open(&bwrap, rustix::fs::OFlags::PATH, rustix::fs::Mode::empty());
    let typed = elf(held.unwrap().as_fd(), bwrap.parent().unwrap(), &host);
    let answer = ground.refusing(&host, &reach(&[], &[]), &[]);
    assert_eq!(
        (typed, answer),
        (Err(Unfit::Unread), (Err(Refusal::Identity), 0))
    );
}

#[test]
fn every_path_the_loader_reads_for_the_launcher_must_be_protected() {
    let Some(ground) = Ground::served() else {
        return;
    };
    ground.fixture.plant("origin/lib.so");
    let cases = [interpreters(), searched()].concat();
    agreed(&ground, &cases, &ground.host());
}

/// An image of this host's naming `interpreter`, the `search` strings by
/// tag (29 `DT_RUNPATH`, 15 `DT_RPATH`), and the `needed` names.
fn naming(interpreter: &[u8], search: &[(u64, &[u8])], needed: &[&[u8]]) -> Vec<u8> {
    let image = Image {
        interpreter: interpreter.to_vec(),
        search: search
            .iter()
            .map(|(tag, text)| (*tag, text.to_vec()))
            .collect(),
        needed: needed.iter().map(|name| name.to_vec()).collect(),
        ..Image::default()
    };
    image.bytes()
}

/// libc by name alone, which the loader finds through protected places.
const LIBC: &[u8] = b"libc.so.6";

/// The host's own loader inputs, then interpreters relative, absent,
/// outside the set though root's on a route only root can change, and a
/// directory.
fn interpreters() -> Vec<Case> {
    let system = interpreter();
    let unprotected = Err(Unfit::Unprotected(Input::Interpreter));
    let host_inputs = naming(
        &system,
        &[(29, b"/usr/lib"), (15, b"/usr/lib")],
        &[LIBC, &system],
    );
    vec![
        ("system", host_inputs, Ok(())),
        (
            "interpreter-relative",
            naming(b"lib/ld.so", &[], &[LIBC]),
            Err(Unfit::Relative(Input::Interpreter)),
        ),
        (
            "interpreter-absent",
            naming(b"/nonexistent-u6c5c/ld.so", &[], &[LIBC]),
            Err(Unfit::Unresolved(Input::Interpreter)),
        ),
        (
            "interpreter-outside",
            naming(b"/etc/passwd", &[], &[LIBC]),
            unprotected,
        ),
        (
            "interpreter-directory",
            naming(b"/usr/lib", &[], &[LIBC]),
            unprotected,
        ),
    ]
}

/// Search path entries and slash-named needed libraries the loader would
/// read outside the protected set, or that name no place; a search path
/// named twice; and a token in a search path or in a needed name with no
/// slash, which the loader would expand.
fn searched() -> Vec<Case> {
    let system = interpreter();
    let search = |entry: &[u8], tag| naming(&system, &[(tag, entry)], &[LIBC]);
    let usr: &[u8] = b"/usr/lib";
    let twice = |tag| naming(&system, &[(tag, usr), (tag, usr)], &[LIBC]);
    let needing = |name: &[u8]| naming(&system, &[], &[name]);
    let relative = Err(Unfit::Relative(Input::SearchPath));
    vec![
        ("runpath-twice", twice(29), Err(Unfit::Malformed)),
        ("rpath-twice", twice(15), Err(Unfit::Malformed)),
        ("runpath-relative", search(b"lib", 29), relative),
        ("rpath-relative", search(b"lib", 15), relative),
        ("runpath-empty", search(b"/usr/lib:", 29), relative),
        ("runpath-token", search(b"/usr/$LIB", 29), relative),
        ("rpath-token", search(b"$ORIGIN/$PLATFORM", 15), relative),
        ("runpath-joined", search(b"$ORIGINX", 29), relative),
        (
            "needed-token",
            needing(b"lib$PLATFORM.so"),
            Err(Unfit::Relative(Input::Needed)),
        ),
        (
            "runpath-origin",
            search(b"$ORIGIN/..", 29),
            Err(Unfit::Unprotected(Input::SearchPath)),
        ),
        (
            "runpath-absent",
            search(b"/nonexistent-u6c5c", 29),
            Err(Unfit::Unresolved(Input::SearchPath)),
        ),
        (
            "runpath-file",
            search(b"/etc/ld.so.cache", 29),
            Err(Unfit::Unprotected(Input::SearchPath)),
        ),
        (
            "needed-relative",
            needing(b"lib/x.so"),
            Err(Unfit::Relative(Input::Needed)),
        ),
        (
            "needed-origin",
            needing(b"${ORIGIN}/../origin/lib.so"),
            Err(Unfit::Unprotected(Input::Needed)),
        ),
        (
            "needed-absent",
            needing(b"/nonexistent-u6c5c/x.so"),
            Err(Unfit::Unresolved(Input::Needed)),
        ),
    ]
}

#[test]
fn a_loader_input_a_managed_writer_could_change_is_unprotected() {
    let Some(ground) = Ground::served() else {
        return;
    };
    let system = interpreter();
    let unprotected = Err(Unfit::Unprotected(Input::Interpreter));
    // Root among the sealed writers: every system file the loader reads is
    // then writable by one, though the launcher, this user's, is not.
    let rooted = Host {
        credentials: strangers().sealed(&[0]),
        ..ground.host()
    };
    let cases = [("rooted", Image::host().bytes(), unprotected)];
    agreed(&ground, &cases, &rooted);
    // The system interpreter named through a directory its group can
    // write: the interpreter is protected, its route is not.
    let link = ground
        .fixture
        .link("links/ld.so", Path::new(OsStr::from_bytes(&system)));
    chmod(&ground.fixture.path("links"), 0o775);
    let image = Image {
        interpreter: link.as_os_str().as_bytes().to_vec(),
        ..Image::host()
    };
    let cases = [("linked", image.bytes(), unprotected)];
    agreed(&ground, &cases, &ground.host());
}

#[test]
fn a_loader_input_is_protected_only_where_root_object_and_every_hop_are() {
    // Facts as a resolution reads them, judged apart from any host: a
    // root-owned directory and file no one else can write, and each made
    // writable by its group.
    let shut = Owned {
        uid: 0,
        mode: 0o040_755,
        acl: Acl::Absent,
    };
    let open = Owned {
        mode: 0o040_775,
        ..shut
    };
    let file = Owned {
        mode: 0o100_644,
        ..shut
    };
    let writable = Owned {
        mode: 0o100_664,
        ..shut
    };
    let writers = strangers();
    let judge =
        |top, object: &Owned, hops: &[Owned]| protects(top, object, hops.iter().copied(), &writers);
    let cases = [
        ("protected", judge(Some(shut), &file, &[shut, shut])),
        // The object alone writable, `/` and its route shut.
        ("object", judge(Some(shut), &writable, &[shut, shut])),
        ("hop", judge(Some(shut), &file, &[shut, open])),
        // `/` alone writable, or unread, the object and its route shut.
        ("root", judge(Some(open), &file, &[shut, shut])),
        ("root-unread", judge(None, &file, &[shut, shut])),
    ];
    let expected = cases.map(|(case, _)| (case, case == "protected"));
    assert_eq!(cases, expected);
}

#[test]
fn the_loaders_preload_file_and_cache_are_read_only_where_they_exist() {
    let Some(ground) = Ground::served() else {
        return;
    };
    let fixture = &ground.fixture;
    let (preload, cache) = (
        fixture.path("etc/ld.so.preload"),
        fixture.path("etc/ld.so.cache"),
    );
    let answer = |loader: [&Path; 2]| {
        let host = Host {
            loader,
            ..ground.host()
        };
        let cases = [("elf", Image::host().bytes(), Ok(()))];
        judged(&ground, &cases, &host).remove(0)
    };
    // Absent, both; then each present outside the system set; the host's
    // cache, in the set; a preload file that does not resolve.
    let mut cases = vec![("absent", answer([&preload, &cache]))];
    let planted = fixture.file("etc/ld.so.preload", 0o644);
    cases.push(("preload", answer([&planted, &cache])));
    std::fs::rename(&planted, &cache).unwrap();
    cases.push(("cache", answer([&preload, &cache])));
    cases.push(("system", answer([&preload, Path::new("/etc/ld.so.cache")])));
    cases.push(("relative", answer([Path::new("etc/ld.so.preload"), &cache])));
    let cases: Vec<_> = cases
        .into_iter()
        .map(|(case, (_, typed, ran))| (case, typed, ran))
        .collect();
    let want = expected(&[
        ("absent", Vec::new(), Ok(())),
        (
            "preload",
            Vec::new(),
            Err(Unfit::Unprotected(Input::Preload)),
        ),
        ("cache", Vec::new(), Err(Unfit::Unprotected(Input::Cache))),
        ("system", Vec::new(), Ok(())),
        (
            "relative",
            Vec::new(),
            Err(Unfit::Unresolved(Input::Preload)),
        ),
    ]);
    assert_eq!(cases, want);
}

#[test]
fn every_launcher_refusal_reads_as_the_operators_text() {
    let causes = [
        (Unfit::Script, "the box launcher is a script"),
        (
            Unfit::Static,
            "the box launcher is static, naming no program interpreter or no dynamic segment",
        ),
        (
            Unfit::Audited,
            "the box launcher names an audit library or a filter",
        ),
        (
            Unfit::Unprotected(Input::Cache),
            "the box launcher's loader cache is not in the protected system set",
        ),
        (
            Unfit::Relative(Input::SearchPath),
            "the box launcher's library search path is relative, empty or holds a token",
        ),
    ];
    for (cause, text) in causes {
        assert_eq!(cause.to_string(), text);
    }
}

#[test]
fn each_loader_input_names_itself_in_its_refusal() {
    use Input::{Cache, Interpreter, Needed, Preload, SearchPath};
    let causes = [Interpreter, SearchPath, Needed, Preload, Cache]
        .map(|input| Unfit::Unprotected(input).to_string());
    let named = [
        "program interpreter",
        "library search path",
        "needed library",
        "loader preload file",
        "loader cache",
    ]
    .map(|input| format!("the box launcher's {input} is not in the protected system set"));
    assert_eq!(causes, named);
}
