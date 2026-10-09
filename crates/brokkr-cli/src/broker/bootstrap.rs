//! `brokkr broker bootstrap` (decision 0077, design D5; MB3, MB4, MB5;
//! slice two U6c6a): the trusted helper a server box runs first. It reads
//! the broker's sealed, nonsecret intent from one inherited anonymous pipe,
//! checks the box it actually stands in against it — its namespaces, every
//! mount, the private tmpfs `HOME` and `TMPDIR`, the network and its own
//! fixed environment — and writes one closed ready message of at most
//! 4 KiB on a second inherited pipe, never stdout. Then it waits on the
//! first until the broker lets it go.
//!
//! It takes no plan, store or grant locator and runs nothing: the binding
//! channel and the server's exec are U6c8's, so a released bootstrap still
//! refuses with SD3's incomplete-serving cause. Absent or invalid control
//! context, or a box it cannot show, is MB3's establishment cause, and no
//! ready byte is written. A marker or an environment bit is no evidence:
//! every fact is read from `/proc` and the private directories themselves.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;
use brokkr_protocol::broker::{Network, Refusal};
use brokkr_protocol::hands::server_environment;
use serde::{Deserialize, Serialize};

use crate::cli_args::BrokerBootstrapArgs;

// Linux's: elsewhere no `/proc` names a descriptor, so every bootstrap
// refuses before its pipes are taken.
#[cfg(all(test, target_os = "linux"))]
mod tests;

/// The most bytes one control message holds: the intent frame's body, and
/// the ready message, which [`message`] measures before a byte is written
/// (MB3).
const FRAME_MAX: usize = 4096;

/// MB4's private directories, by the fixed names that locate them. The
/// names only say where to look: each must be shown a fresh private tmpfs.
const PRIVATE: [&str; 2] = ["HOME", "TMPDIR"];

/// A private directory's mode: a directory only its owner may enter, with
/// no special bit.
const PRIVATE_MODE: u32 = 0o040_700;

/// Where the box's own proc file system stands (bubblewrap's `--proc`).
const PROC: &str = "/proc";

/// Where the box's minimal `/dev` stands: its own `nodev` tmpfs
/// (bubblewrap's `--dev`).
const DEV: &str = "/dev";

/// The parts of the box's own proc that bubblewrap covers, each with a
/// read-only bind of itself at its own path, and nothing else of it.
const COVERS: [&str; 4] = ["/sys", "/sysrq-trigger", "/irq", "/bus"];

/// The kernel file systems a box's `/dev` may hold whole, each only at its
/// own place: its terminals and its queues.
const KERNEL: [(&str, &str); 2] = [("/dev/pts", "devpts"), ("/dev/mqueue", "mqueue")];

/// The host device nodes a box's `/dev` may bind, each alone at its own
/// name (bubblewrap's `--dev`): never the host's whole `devtmpfs`, nor
/// another node of it.
const NODES: [&str; 6] = ["/null", "/zero", "/full", "/random", "/urandom", "/tty"];

/// The directory of its tmpfs that bubblewrap pivots into as the box's
/// root, which mountinfo shows as the root mount's root: an empty root the
/// launcher made, not a host file system's.
const BUBBLEWRAP_ROOT: &str = "/newroot";

/// A file's identity: its device, `major:minor`, and its inode.
type Identity = ((u32, u32), u64);

/// The broker's sealed intent for this box: the plan and source-set
/// digests readiness binds, the network the dialect's egress projects to,
/// the broker's own namespaces, which the box must not share, and every
/// mount it binds into the box. Names and numbers only: no locator, value
/// or authority.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    plan: String,
    sources: String,
    network: Network,
    host: Spaces,
    mounts: Vec<Source>,
}

/// One mount the broker binds into the box, nested ones each their own
/// (MB3): where it stands in the box, and the device and inode of the
/// checked source it shows there.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: PathBuf,
    device: (u32, u32),
    inode: u64,
}

/// A process's namespaces, by the inode `/proc` gives each.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Spaces {
    mnt: u64,
    pid: u64,
    net: u64,
    ipc: u64,
    uts: u64,
}

/// The one ready message (MB3): the plan and source-set digests the
/// intent sealed, this child by its process id in the box, and the
/// namespaces it observed itself in.
#[derive(Serialize)]
struct Ready<'a> {
    plan: &'a str,
    sources: &'a str,
    pid: u32,
    namespaces: &'a Spaces,
}

/// What the bootstrap observes of the box it stands in.
struct Observed {
    pid: u32,
    euid: u32,
    spaces: Option<Spaces>,
    /// `/proc/self/mountinfo`, as read.
    mounts: String,
    /// The identity each of the intent's mounts shows at its path in the
    /// box, in the intent's order, where it shows one.
    sources: Vec<Option<Identity>>,
    /// Every entry's name in the environment this process was started
    /// with, which its loader read.
    environment: Vec<OsString>,
    /// The directory each [`PRIVATE`] name locates, in order, where it
    /// names one that can be read.
    private: Vec<Option<Directory>>,
}

/// A private directory's own facts, read without following a link.
struct Directory {
    path: PathBuf,
    mode: u32,
    uid: u32,
    empty: bool,
}

/// One mount, as `/proc/self/mountinfo` records it.
struct Record<'a> {
    /// The file system's device, `major:minor`.
    device: &'a str,
    /// The directory of that file system the mount shows: `/` for all of it.
    root: &'a str,
    /// Where it stands in the box, its escapes decoded.
    point: PathBuf,
    read_only: bool,
    /// Whether its own options refuse device nodes.
    nodev: bool,
    fstype: &'a str,
}

/// Where a mount stands in the box, which is what it may be.
enum Place {
    /// At a private directory: checked with that directory.
    Private,
    /// Below a private directory, which must start empty.
    Below,
    /// The box's own root.
    Root,
    /// `/proc` itself.
    Procfs,
    /// Below `/proc`.
    Proc,
    /// `/dev` itself.
    Devfs,
    /// Below `/dev`.
    Dev,
    /// A mount the intent seals.
    Source,
    /// Anywhere else: a mount nothing explains.
    Elsewhere,
}

/// Run the bootstrap `args` names in the box it stands in, and end with
/// the cause it refuses by: nothing it does ends in serving.
pub(super) fn run(args: &BrokerBootstrapArgs) -> Refusal {
    run_with(args, observe)
}

/// [`run`], with `observe` standing for the box's facts.
fn run_with(args: &BrokerBootstrapArgs, observe: impl FnOnce(&Intent) -> Observed) -> Refusal {
    match ready(args, observe) {
        Ok(control) => released(control),
        Err(refusal) => refusal,
    }
}

/// Take the control descriptors, read the intent, check the box, and write
/// the one ready message, closing its pipe; the control pipe, still held,
/// where all of it held.
fn ready(
    args: &BrokerBootstrapArgs,
    observe: impl FnOnce(&Intent) -> Observed,
) -> Result<File, Refusal> {
    let numbers = (args.control.as_deref(), args.ready.as_deref());
    let (mut control, ready) = descriptors(numbers, &standard())?;
    let intent = intent(&mut control)?;
    let observed = observe(&intent);
    let bytes = message(&decide(&intent, &observed)?)?;
    File::from(ready)
        .write_all(&bytes)
        .or(Err(Refusal::Establishment))?;
    Ok(control)
}

/// The bytes of the one ready message, where they fit [`FRAME_MAX`].
fn message(ready: &Ready<'_>) -> Result<Vec<u8>, Refusal> {
    let bytes = serde_json::to_vec(ready).or(Err(Refusal::Establishment))?;
    (bytes.len() <= FRAME_MAX)
        .then_some(bytes)
        .ok_or(Refusal::Establishment)
}

/// Wait on the control pipe until the broker lets the bootstrap go. No
/// binding channel stands yet (U6c8), so a release is still SD3's
/// incomplete-serving cause; a byte after the intent frame is no release.
fn released(control: File) -> Refusal {
    let mut rest = Vec::new();
    match control.take(1).read_to_end(&mut rest) {
        Ok(0) => Refusal::ServingIncomplete,
        Ok(_) | Err(_) => Refusal::Establishment,
    }
}

/// What this process's standard streams are open on, where they are.
fn standard() -> Vec<String> {
    (0..3).filter_map(link).collect()
}

/// The control and ready descriptors `numbers` names: two distinct
/// inherited anonymous pipes, neither one a `stdio` stream's, so neither
/// stdout nor a marker file can carry readiness. Each is owned only once
/// every check held.
fn descriptors(
    (control, ready): (Option<&str>, Option<&str>),
    stdio: &[String],
) -> Result<(File, OwnedFd), Refusal> {
    let (control, control_link) = pipe(control)?;
    let (ready, ready_link) = pipe(ready)?;
    let links = [&control_link, &ready_link];
    let shared = stdio.iter().any(|link| links.contains(&link));
    if (control_link == ready_link) | shared {
        return Err(Refusal::Establishment);
    }
    // SAFETY: `/proc` shows each number open on an anonymous pipe in this
    // process. The broker made both for this process to inherit; neither
    // is a standard stream nor the other, and nothing else here takes it.
    let (control, ready) = unsafe { (OwnedFd::from_raw_fd(control), OwnedFd::from_raw_fd(ready)) };
    Ok((File::from(control), ready))
}

/// The inherited descriptor `text` numbers, and the anonymous pipe it
/// holds as `/proc` links it.
fn pipe(text: Option<&str>) -> Result<(RawFd, String), Refusal> {
    let fd = text
        .and_then(|text| text.parse::<RawFd>().ok())
        .filter(|fd| *fd > 2)
        .ok_or(Refusal::Establishment)?;
    let link = link(fd)
        .filter(|link| link.starts_with("pipe:["))
        .ok_or(Refusal::Establishment)?;
    Ok((fd, link))
}

/// What this process's descriptor `fd` is open on, where it is open.
fn link(fd: RawFd) -> Option<String> {
    let link = std::fs::read_link(format!("/proc/self/fd/{fd}")).ok()?;
    link.into_os_string().into_string().ok()
}

/// The one intent frame: a four-byte big-endian length, then that many
/// bytes, at most [`FRAME_MAX`], of one closed intent whose digests are
/// sha256s.
fn intent(control: &mut File) -> Result<Intent, Refusal> {
    let mut length = [0; 4];
    control
        .read_exact(&mut length)
        .or(Err(Refusal::Establishment))?;
    let length = usize::try_from(u32::from_be_bytes(length)).unwrap_or(usize::MAX);
    if length > FRAME_MAX {
        return Err(Refusal::Establishment);
    }
    let mut body = vec![0; length];
    control
        .read_exact(&mut body)
        .or(Err(Refusal::Establishment))?;
    let intent: Intent = serde_json::from_slice(&body).or(Err(Refusal::Establishment))?;
    let sealed = canonical::is_sha256_hex(&intent.plan) & canonical::is_sha256_hex(&intent.sources);
    sealed.then_some(intent).ok_or(Refusal::Establishment)
}

/// What this process observes of the box it stands in, the identity at
/// each of `intent`'s mounts included.
fn observe(intent: &Intent) -> Observed {
    Observed {
        pid: std::process::id(),
        euid: rustix::process::geteuid().as_raw(),
        spaces: spaces(),
        mounts: std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default(),
        sources: intent
            .mounts
            .iter()
            .map(|source| identity(&source.path))
            .collect(),
        environment: names(&std::fs::read("/proc/self/environ").unwrap_or_default()),
        private: PRIVATE
            .iter()
            .map(|name| std::env::var_os(name).and_then(directory))
            .collect(),
    }
}

/// Each entry's name in `environ`, the block `/proc` keeps of the
/// environment a process was started with: an entry without `=` is all
/// name, so it matches no fixed one.
fn names(environ: &[u8]) -> Vec<OsString> {
    let entries = environ
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty());
    entries
        .map(|entry| entry.split(|byte| *byte == b'=').next().unwrap_or_default())
        .map(|name| OsString::from_vec(name.to_vec()))
        .collect()
}

/// This process's namespaces, where `/proc` shows each.
fn spaces() -> Option<Spaces> {
    let space = |name: &str| {
        let space = std::fs::metadata(format!("/proc/self/ns/{name}"));
        space.ok().map(|space| space.ino())
    };
    Some(Spaces {
        mnt: space("mnt")?,
        pid: space("pid")?,
        net: space("net")?,
        ipc: space("ipc")?,
        uts: space("uts")?,
    })
}

/// The identity of what `path` names in the box, itself and not a link's
/// target.
#[cfg(target_os = "linux")]
fn identity(path: &Path) -> Option<Identity> {
    let facts = std::fs::symlink_metadata(path).ok()?;
    let device = (
        rustix::fs::major(facts.dev()),
        rustix::fs::minor(facts.dev()),
    );
    Some((device, facts.ino()))
}

/// Off Linux no box stands, so nothing the bootstrap names has a box's
/// identity, and it refuses as not established.
#[cfg(not(target_os = "linux"))]
fn identity(_: &Path) -> Option<Identity> {
    None
}

/// The directory `path` names, itself and not a link's target, and
/// whether it holds anything.
fn directory(path: OsString) -> Option<Directory> {
    let path = PathBuf::from(path);
    let facts = std::fs::symlink_metadata(&path).ok()?;
    let empty = std::fs::read_dir(&path).ok()?.next().is_none();
    Some(Directory {
        mode: facts.mode(),
        uid: facts.uid(),
        empty,
        path,
    })
}

/// The ready message for `observed`, where it shows the box `intent`
/// seals: the fixed environment alone, namespaces apart from the broker's
/// with the network the intent projects, fresh private directories, each
/// sealed source where the intent puts it, and every mount where the box
/// may hold it.
fn decide<'a>(intent: &'a Intent, observed: &'a Observed) -> Result<Ready<'a>, Refusal> {
    let spaces = observed.spaces.as_ref().ok_or(Refusal::Establishment)?;
    let private = private(observed, &intent.mounts).ok_or(Refusal::Establishment)?;
    let held = fixed(&observed.environment)
        & separate(&intent.host, spaces, &intent.network)
        & bound(&intent.mounts, &observed.sources)
        & mounted(&observed.mounts, &private, &intent.mounts);
    let ready = Ready {
        plan: &intent.plan,
        sources: &intent.sources,
        pid: observed.pid,
        namespaces: spaces,
    };
    held.then_some(ready).ok_or(Refusal::Establishment)
}

/// Whether the environment holds MB4's fixed names, each once, and
/// nothing else: no binding, loader or inherited entry reached this
/// loader (the names are the builder's own table).
fn fixed(environment: &[OsString]) -> bool {
    let mut names: Vec<&OsStr> = environment.iter().map(OsString::as_os_str).collect();
    let mut expected: Vec<&OsStr> = server_environment().map(OsStr::new).collect();
    names.sort();
    expected.sort();
    names == expected
}

/// Whether `ours` stands apart from the broker's `host` namespaces: its
/// own mount, PID, IPC and UTS namespaces always, and its own network
/// namespace exactly where the network is isolated (MB4).
fn separate(host: &Spaces, ours: &Spaces, network: &Network) -> bool {
    let pairs = [
        (host.mnt, ours.mnt),
        (host.pid, ours.pid),
        (host.ipc, ours.ipc),
        (host.uts, ours.uts),
    ];
    let apart = pairs.iter().all(|(host, ours)| host != ours);
    let network = match network {
        Network::Isolated => host.net != ours.net,
        Network::Shared => host.net == ours.net,
    };
    apart & network
}

/// Whether each of `sources` shows, at its path in the box, the device and
/// inode the broker checked (MB3's mount identity): a changed source, or
/// one not there, is no box the intent seals.
fn bound(sources: &[Source], seen: &[Option<Identity>]) -> bool {
    let each = sources
        .iter()
        .zip(seen)
        .all(|(source, seen)| *seen == Some((source.device, source.inode)));
    (sources.len() == seen.len()) & each
}

/// Each private directory, where every one was observed, [`placed`] apart
/// from the `sources` and the other, a directory only this user may enter,
/// and empty.
fn private<'o>(observed: &'o Observed, sources: &[Source]) -> Option<Vec<&'o Directory>> {
    let private: Vec<&Directory> = observed.private.iter().flatten().collect();
    let fresh = private
        .iter()
        .all(|dir| (dir.mode == PRIVATE_MODE) & (dir.uid == observed.euid) & dir.empty);
    let whole = private.len() == PRIVATE.len();
    (whole & fresh & placed(&private, sources)).then_some(private)
}

/// Whether each private directory stands where one may (row 5 of the
/// commission's closed mount table): [`outside`] the box's own trees, and
/// neither at, above nor below a sealed source or the other private
/// directory. The environment chose these paths, so none is trusted until
/// this holds.
fn placed(private: &[&Directory], sources: &[Source]) -> bool {
    let apart = |one: &Path, other: &Path| !one.starts_with(other) & !other.starts_with(one);
    private.iter().enumerate().all(|(at, dir)| {
        let others = private.iter().enumerate().filter(|(other, _)| *other != at);
        let others = others.map(|(_, other)| other.path.as_path());
        let sealed = sources.iter().map(|source| source.path.as_path());
        let alone = others.chain(sealed).all(|other| apart(&dir.path, other));
        outside(&dir.path) & alone
    })
}

/// Whether `path` lies outside the trees the box itself holds: not its
/// root, and neither at nor below `/proc` or `/dev`, which only their own
/// rows judge. No private directory or sealed source stands in them.
fn outside(path: &Path) -> bool {
    let reserved = [PROC, DEV].iter().any(|tree| path.starts_with(tree));
    !reserved & (path != Path::new("/"))
}

/// Whether the mounts in `mounts` are the box `sources` seals, each by the
/// one row of the commission's closed table its place selects: `/proc` the
/// box's own proc, its covers each a read-only bind of itself; `/dev` the
/// box's own whole `nodev` tmpfs, shown nowhere else, holding only its
/// terminals, its queues and the [`NODES`], each at its own name; the root
/// bubblewrap's tmpfs, shown nowhere else; each source, outside those,
/// exactly one read-only mount at its path; each private directory exactly
/// one whole writable tmpfs, shown nowhere else, with nothing below it; and
/// no other mount, nested and read-only ones included.
fn mounted(mounts: &str, private: &[&Directory], sources: &[Source]) -> bool {
    let records: Option<Vec<Record<'_>>> = mounts.lines().map(Record::parse).collect();
    let Some(records) = records else {
        return false;
    };
    let root = alone(&records, Path::new("/")).is_some_and(|root| root.tmpfs(BUBBLEWRAP_ROOT));
    let fresh = private.iter().all(|dir| {
        let tmpfs = alone(&records, &dir.path);
        tmpfs.is_some_and(|tmpfs| tmpfs.tmpfs("/") & !tmpfs.read_only)
    });
    let sealed = sources
        .iter()
        .all(|source| outside(&source.path) & only(&records, &source.path).is_some());
    let each = records
        .iter()
        .all(|record| record.admitted(place(&record.point, private, sources)));
    proc(&records) & dev(&records) & root & sealed & fresh & each
}

/// Whether `records` hold the box's own proc: one whole proc file system
/// at `/proc`, and every mount below it a part of that same one, by its
/// device. A host's proc, or any other file system, bound there is none of
/// the box's.
fn proc(records: &[Record<'_>]) -> bool {
    let own = |procfs: &Record<'_>| {
        let mut below = records
            .iter()
            .filter(|record| record.point.starts_with(PROC));
        let whole = (procfs.fstype == "proc") & (procfs.root == "/");
        whole & below.all(|record| record.device == procfs.device)
    };
    only(records, Path::new(PROC)).is_some_and(own)
}

/// Whether `records` hold the box's own `/dev`: exactly one whole tmpfs
/// there, on a device no other mount shows, refusing device nodes.
fn dev(records: &[Record<'_>]) -> bool {
    alone(records, Path::new(DEV)).is_some_and(|dev| dev.tmpfs("/") & dev.nodev)
}

/// The one mount `records` hold at `path`, where exactly one stands there.
fn only<'r>(records: &'r [Record<'r>], path: &Path) -> Option<&'r Record<'r>> {
    let mut at = records.iter().filter(|record| record.point == path);
    at.next().filter(|_| at.next().is_none())
}

/// The one mount `records` hold at `path`, where its device no other mount
/// in the box shows. That a tmpfs is new, not a host tmpfs mounted whole,
/// the box cannot show from inside: the launcher that made it does
/// (U6c6b).
fn alone<'r>(records: &'r [Record<'r>], path: &Path) -> Option<&'r Record<'r>> {
    let record = only(records, path)?;
    let shared = records
        .iter()
        .filter(|other| other.device == record.device)
        .count();
    (shared == 1).then_some(record)
}

/// Where `point` stands in the box, whose private directories are
/// `private` and whose sealed mounts are `sources`, by the first row of the
/// commission's closed table it meets: the box's own `/proc` and `/dev`
/// trees before any source or private directory is consulted, so neither
/// exemption reaches into them.
fn place(point: &Path, private: &[&Directory], sources: &[Source]) -> Place {
    if point == Path::new(PROC) {
        Place::Procfs
    } else if point.starts_with(PROC) {
        Place::Proc
    } else if point == Path::new(DEV) {
        Place::Devfs
    } else if point.starts_with(DEV) {
        Place::Dev
    } else if point == Path::new("/") {
        Place::Root
    } else if sources.iter().any(|source| point == source.path) {
        Place::Source
    } else if private.iter().any(|dir| point == dir.path) {
        Place::Private
    } else if private.iter().any(|dir| point.starts_with(&dir.path)) {
        Place::Below
    } else {
        Place::Elsewhere
    }
}

/// A mountinfo path with its octal escapes (`\040` for a space, and the
/// tab, newline and backslash likewise) decoded, where each is whole.
fn unescaped(text: &str) -> Option<PathBuf> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        let code = std::str::from_utf8(rest.get(..3)?).ok()?;
        bytes.push(u8::from_str_radix(code, 8).ok()?);
        rest = &rest[3..];
    }
    Some(PathBuf::from(OsString::from_vec(bytes)))
}

impl<'a> Record<'a> {
    /// One mountinfo line: its device and root (fields three and four, as
    /// escaped), its mount point decoded, whether its own options make it
    /// read-only and refuse device nodes, and the file-system type after
    /// the separator.
    fn parse(line: &'a str) -> Option<Record<'a>> {
        let (fields, rest) = line.split_once(" - ")?;
        let mut fields = fields.split(' ').skip(2);
        let device = fields.next()?;
        let root = fields.next()?;
        let point = unescaped(fields.next()?)?;
        let options = fields.next()?;
        let option = |name| options.split(',').any(|option| option == name);
        let fstype = rest.split(' ').next()?;
        Some(Record {
            device,
            root,
            point,
            read_only: option("ro"),
            nodev: option("nodev"),
            fstype,
        })
    }

    /// Whether this mount is a tmpfs showing its directory `root`: `/`
    /// for all of it.
    fn tmpfs(&self, root: &str) -> bool {
        (self.fstype == "tmpfs") & (self.root == root)
    }

    /// Whether this mount may stand at `place`. The root, `/dev` and the
    /// private directories are [`alone`]'s to check, and `/proc` and what
    /// stands below it are [`proc`]'s; below `/proc` only a read-only
    /// cover, below `/dev` only a whole kernel file system at its own place
    /// or one of the [`NODES`] at its own name, and nothing the intent does
    /// not seal. Being read-only explains no other mount.
    fn admitted(&self, place: Place) -> bool {
        let whole = self.root == "/";
        let kernel = KERNEL
            .iter()
            .any(|(at, fstype)| (self.point == Path::new(at)) & (self.fstype == *fstype));
        let node = (self.fstype == "devtmpfs") & NODES.iter().any(|node| self.shows(node, DEV));
        match place {
            Place::Private | Place::Root | Place::Procfs | Place::Devfs => true,
            Place::Below | Place::Elsewhere => false,
            Place::Proc => self.read_only & COVERS.iter().any(|cover| self.shows(cover, PROC)),
            Place::Dev => (whole & kernel) | node,
            Place::Source => self.read_only,
        }
    }

    /// Whether this mount shows `part` of its file system at that same
    /// `part` of `base`: a cover or a node bound at its own name, not
    /// renamed.
    fn shows(&self, part: &str, base: &str) -> bool {
        (self.root == part) & (self.point == Path::new(&format!("{base}{part}")))
    }
}
