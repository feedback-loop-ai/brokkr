//! The bounded source observer one MCP server's box is built from
//! (decision 0065 slice two, U6c5a; D5, MB3, MB4, SC1). Linux only: it
//! reads the kernel's own file and mount identities, and a host without
//! them has no server box (decision 0063 keeps macOS's refusal).
//!
//! Every source resolves from `/` one component at a time through
//! descriptors, the observer reading each link itself, so every directory
//! entered and every link read is a hop with its own identity: the whole
//! chain, not its final name. A directory source is then walked with
//! no-follow lookups, holding only its root handle and the active ancestor
//! stack. Every object the walk reaches, a source's own or one below it,
//! takes the one observation [`Observer::object`]: whatever it is and
//! however many links it has, it is opened and must be the object
//! observed. Every bound is a count, fixed, and the counts are the
//! observer's memory bound (operator ruling 2026-10-07). Every special,
//! cyclic or unknown fact refuses, and so does every unreadable one but a
//! support file proved root-owned and unwritable by every managed writer:
//! nothing is skipped and nothing an earlier preparation saw is reused.
//! Each directory's entries are taken in byte order, so the digest follows
//! the tree and never the order a filesystem lists it in. Once every
//! source is walked, each resolves again: the same route, ending on the
//! object its handle holds. The checked handles, not their names, are what
//! bubblewrap binds.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};

use rustix::fs::{AtFlags, FileType, Mode, OFlags, RawDir, StatxFlags};
use rustix::io::Errno;
use sha2::{Digest, Sha256};

use super::{Namespace, Observed, Profile, ServerProfile, ServerProgram, RESOLV_CONF};
use crate::broker::{Reach, Refusal, Sources, Tree};

pub(in crate::hands) mod host;

pub(in crate::hands) use host::Host;
use host::{acl, excluded, mapped, readable, records, resolve, route, target, untouchable};
use host::{Hop, Owned, Record, Seen};

/// MB3's observation bounds for one preparation: objects observed (each
/// walked source's root and every entry beneath it), directories below a
/// walked root, links read resolving one source, and mount table records.
#[derive(Debug, Clone, Copy)]
pub(in crate::hands) struct Limits {
    pub(in crate::hands) entries: u64,
    pub(in crate::hands) depth: usize,
    pub(in crate::hands) hops: usize,
    pub(in crate::hands) mounts: usize,
}

/// MB3's fixed bounds: 1,000,000 entries, depth 64, 40 hops per
/// resolution and 65,536 mount records.
pub(in crate::hands) const LIMITS: Limits = Limits {
    entries: 1_000_000,
    depth: 64,
    hops: 40,
    mounts: 65_536,
};

/// The buffer a directory source's listings are read through, once for
/// its whole walk.
const LISTING: usize = 32 << 10;

/// Which of MB3's rules a source's files take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::hands) enum Role {
    /// The launch name's own resolution: a program file whose route must
    /// lie outside writable reach.
    Launch,
    /// The program tree and the bootstrap: every regular file singly
    /// linked, whoever owns it.
    Program,
    /// The generated identity: program files the engine writes afresh
    /// under a new random name for each box, so their route, place,
    /// device, inode and times are checked but never digested, and the
    /// digest takes each file's name and its other facts (SC1).
    Generated,
    /// The fixed system set, the resolver and the launcher: a regular file
    /// with a second link needs the kernel write-exclusion proof.
    Support,
}

/// Whether a source may be absent, as an optional system path may, or
/// must be there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Presence {
    Required,
    Optional,
}

/// One host path the box mounts or runs, and the rules it takes.
#[derive(Debug, Clone, Copy)]
pub(in crate::hands) struct Source<'s> {
    pub(in crate::hands) path: &'s Path,
    pub(in crate::hands) role: Role,
    pub(in crate::hands) presence: Presence,
}

/// What one preparation observed: each source's handle and canonical
/// place, absent where an optional source is not there, and the facts the
/// plan seals of the set.
pub(in crate::hands) struct Observation {
    pub(in crate::hands) held: Vec<Option<(OwnedFd, PathBuf)>>,
    pub(in crate::hands) sources: Sources,
}

/// Observe `sources` against the seat's `reach` on `host`, or MB3's first
/// cause in [`Refusal`]'s order: a launch route in writable reach, then
/// any route, link target or source in reach, then a program file with a
/// second link, then any unprotected identity, an exceeded bound included.
pub(in crate::hands) fn observe(
    sources: &[Source<'_>],
    reach: &Reach,
    host: &Host<'_>,
) -> Result<Observation, Refusal> {
    let mut observer = Observer::new(host, reach);
    let mut first = Vec::with_capacity(sources.len());
    for source in sources {
        let held = observer.source(source);
        first.push(held);
    }
    observer.mounts();
    (host.between)();
    for (source, first) in sources.iter().zip(&first) {
        observer.again(source, first.as_ref());
    }
    let sources = observer.verdict()?;
    let held = first
        .into_iter()
        .map(|one| one.map(|one| (one.fd, one.place)));
    Ok(Observation {
        held: held.collect(),
        sources,
    })
}

/// A source as its first resolution left it: the handle its observation
/// opened, its canonical place and every hop on its way.
struct Held {
    fd: OwnedFd,
    place: PathBuf,
    chain: Vec<Hop>,
}

/// What names one object: its device (major, minor) and inode.
type Identity = ((u32, u32), u64);

/// An object's observed facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Facts {
    dev: (u32, u32),
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    nlink: u32,
    size: u64,
    mtime: (i64, u32),
    mount: u64,
}

/// The kind of object a mode names, as the observer treats it.
enum Kind {
    File,
    Dir,
    Link,
    Special,
}

impl Facts {
    /// The facts `held` reports, none where the kernel names no mount.
    fn of(held: &rustix::fs::Statx) -> Option<Facts> {
        let mount = held.stx_mask & StatxFlags::MNT_ID.bits() != 0;
        Some(Facts {
            dev: (held.stx_dev_major, held.stx_dev_minor),
            ino: held.stx_ino,
            mode: u32::from(held.stx_mode),
            uid: held.stx_uid,
            gid: held.stx_gid,
            nlink: held.stx_nlink,
            size: held.stx_size,
            mtime: (held.stx_mtime.tv_sec, held.stx_mtime.tv_nsec),
            mount: mount.then_some(held.stx_mnt_id)?,
        })
    }

    fn identity(&self) -> Identity {
        (self.dev, self.ino)
    }

    fn kind(&self) -> Kind {
        match FileType::from_raw_mode(self.mode) {
            FileType::RegularFile => Kind::File,
            FileType::Directory => Kind::Dir,
            FileType::Symlink => Kind::Link,
            _ => Kind::Special,
        }
    }

    /// The digested facts, in a fixed order; never a mount id or a
    /// descriptor number.
    fn bytes(&self) -> [u8; 80] {
        let ((major, minor), (sec, nsec)) = (self.dev, self.mtime);
        let [major, minor, mode, uid, gid, nlink, nsec] = [
            major, minor, self.mode, self.uid, self.gid, self.nlink, nsec,
        ]
        .map(u64::from);
        let (ino, size, sec) = (self.ino, self.size, sec.cast_unsigned());
        words(&[major, minor, mode, uid, gid, nlink, nsec, ino, size, sec])
    }

    /// The digested facts of a generated file, which every preparation
    /// writes alike: never its device, inode or times.
    fn made(&self) -> [u8; 40] {
        let [mode, uid, gid, nlink] = [self.mode, self.uid, self.gid, self.nlink].map(u64::from);
        words(&[mode, uid, gid, nlink, self.size])
    }
}

/// Words as their little-endian bytes, `B` being eight for each word.
fn words<const B: usize>(words: &[u64]) -> [u8; B] {
    let mut bytes = [0; B];
    for (at, word) in words.iter().enumerate() {
        bytes[at * 8..at * 8 + 8].copy_from_slice(&word.to_le_bytes());
    }
    bytes
}

/// An identity's digested bytes.
fn identity(((major, minor), ino): Identity) -> [u8; 24] {
    words(&[u64::from(major), u64::from(minor), ino])
}

/// Digest one object: its place (`dir`, and `name` below it where there is
/// one), its facts and its link's target, each closed by a zero byte. The
/// place is digested as joined, never built.
pub(in crate::hands) fn digest(
    hasher: &mut Sha256,
    (dir, name): (&Path, &OsStr),
    facts: &[&[u8]],
    link: &[u8],
) {
    let dir = dir.as_os_str().as_bytes();
    hasher.update(dir);
    if !name.is_empty() {
        if !dir.ends_with(b"/") {
            hasher.update(b"/");
        }
        hasher.update(name.as_bytes());
    }
    hasher.update([0]);
    for part in facts {
        hasher.update(part);
    }
    hasher.update([0]);
    hasher.update(link);
    hasher.update([0]);
}

/// `name` in `dir`, never followed; `dir`'s own object where `name` is
/// empty. A kernel that names no mount leaves the object unreadable.
fn stat_at(dir: BorrowedFd<'_>, name: impl rustix::path::Arg) -> rustix::io::Result<Facts> {
    let mask = StatxFlags::BASIC_STATS | StatxFlags::MNT_ID;
    let flags = AtFlags::SYMLINK_NOFOLLOW | AtFlags::EMPTY_PATH;
    let held = rustix::fs::statx(dir, name, flags, mask)?;
    Facts::of(&held).ok_or(Errno::NOTSUP)
}

/// A handle on `name` in `dir` that is the object `facts` observed, of the
/// kind observed, as the handle's own facts show.
fn handle(dir: BorrowedFd<'_>, name: &OsStr, flags: OFlags, facts: &Facts) -> Option<OwnedFd> {
    let flags = flags | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let fd = rustix::fs::openat(dir, name, flags, Mode::empty()).ok()?;
    let held = stat_at(fd.as_fd(), c"").ok()?;
    let kind = |facts: &Facts| FileType::from_raw_mode(facts.mode);
    let same = (held.identity(), kind(&held)) == (facts.identity(), kind(facts));
    same.then_some(fd)
}

/// A directory found to descend into: its name, facts and place.
struct Found {
    name: OsString,
    facts: Facts,
    place: PathBuf,
}

/// One directory on the walk's active ancestor stack: its handle, its
/// place, and the directories found in it that are still to be entered.
struct Frame {
    fd: OwnedFd,
    place: PathBuf,
    pending: Vec<Found>,
}

/// Where the observation opens an object: the directory's handle and
/// place, and the object's name in it (`.` for that directory itself).
type At<'a> = (BorrowedFd<'a>, &'a Path, &'a OsStr);

/// One preparation's observation in progress: the seat's reach roots as
/// spelled and as they resolve, MB3's first cause met, the objects counted
/// and whether a bound was passed, the digest, the mount records read,
/// each mount stood on, each source root's place and mount, each object
/// walked in a role from the mount its walk began on, and the mount of the
/// walk under way.
struct Observer<'o> {
    host: &'o Host<'o>,
    writable: Vec<PathBuf>,
    readable: Vec<PathBuf>,
    reach: Vec<PathBuf>,
    fault: Option<Refusal>,
    entries: u64,
    over: bool,
    hasher: Sha256,
    records: u64,
    seen: Seen,
    placed: Vec<(PathBuf, u64)>,
    walked: BTreeSet<(u64, Identity, Role)>,
    view: u64,
}

impl<'o> Observer<'o> {
    fn new(host: &'o Host<'o>, reach: &Reach) -> Observer<'o> {
        let spelled = |roots: &[PathBuf]| -> Vec<PathBuf> {
            roots
                .iter()
                .flat_map(|root| super::spellings(root))
                .collect()
        };
        let (writable, readable) = (spelled(&reach.writable), spelled(&reach.readable));
        Observer {
            host,
            reach: [&writable[..], &readable[..]].concat(),
            writable,
            readable,
            fault: None,
            entries: 0,
            over: false,
            hasher: Sha256::new(),
            records: 0,
            seen: Seen::new(),
            placed: Vec::new(),
            walked: BTreeSet::new(),
            view: 0,
        }
    }

    /// Hold `cause` where it precedes the cause already held.
    fn fault(&mut self, cause: Refusal) {
        self.fault = self.fault.take().into_iter().chain([cause]).min();
    }

    /// What `result` holds, or none with its cause held.
    fn charged<T>(&mut self, result: Result<T, Refusal>) -> Option<T> {
        result.map_err(|cause| self.fault(cause)).ok()
    }

    /// Count one more object, or refuse past the entry bound, after which
    /// no walk goes on.
    fn count(&mut self) -> Result<(), Refusal> {
        self.entries += 1;
        self.over |= self.entries > self.host.limits.entries;
        (!self.over).then_some(()).ok_or(Refusal::Identity)
    }

    /// Resolve `source`, check its route, place its root on its mount for
    /// the alias proof, observe its object, and walk it once per mount
    /// view, object and role: the same directory seen through another
    /// mount may hold other mounts beneath it. Its first resolution, none
    /// where it is absent or failed.
    fn source(&mut self, source: &Source<'_>) -> Option<Held> {
        let resolved = match resolve(source.path, self.host) {
            Ok(Some(resolved)) => resolved,
            Ok(None) if source.presence == Presence::Optional => return None,
            Ok(None) | Err(_) => {
                self.fault(Refusal::Identity);
                return None;
            }
        };
        let (facts, role, place) = (resolved.facts, source.role, &resolved.place);
        self.route(&resolved.chain, role);
        self.view = facts.mount;
        self.stood(&facts, true);
        self.placed.push((place.clone(), facts.mount));
        let fresh = self.walked.insert((facts.mount, facts.identity(), role));
        if fresh {
            let counted = self.root(place, &facts, role);
            self.charged(counted)?;
        }
        let at = match (resolved.name == ".", place.parent()) {
            (false, Some(dir)) => dir,
            (true, _) | (false, None) => place,
        };
        let mut fd = self.object((resolved.dir.as_fd(), at, &resolved.name), &facts, role)?;
        if fresh & matches!(facts.kind(), Kind::Dir) {
            fd = self.directory(fd, place, role);
        }
        Some(Held {
            fd,
            place: resolved.place,
            chain: resolved.chain,
        })
    }

    /// A source root walked for the first time in its role: counted, and
    /// digested, a generated file by its name and the facts every
    /// preparation repeats.
    fn root(&mut self, place: &Path, facts: &Facts, role: Role) -> Result<(), Refusal> {
        self.count()?;
        let (made, all) = (facts.made(), facts.bytes());
        let (named, bytes): (&Path, &[u8]) = match role {
            Role::Generated => (Path::new(place.file_name().unwrap_or_default()), &made),
            Role::Launch | Role::Program | Role::Support => (place, &all),
        };
        digest(&mut self.hasher, (named, OsStr::new("")), &[bytes], b"");
        Ok(())
    }

    /// Every hop of a route: digested, but for a generated file's, and
    /// outside the seat's reach; a launch's hop in writable reach is the
    /// launch's own cause.
    fn route(&mut self, chain: &[Hop], role: Role) {
        for hop in chain {
            if role != Role::Generated {
                let link = hop.link.as_deref().unwrap_or_default();
                let place = (hop.place.as_path(), OsStr::new(""));
                digest(&mut self.hasher, place, &[&identity(hop.identity)], link);
            }
            let inside = |roots: &[PathBuf]| roots.iter().any(|root| hop.place.starts_with(root));
            match (inside(&self.writable), inside(&self.readable), role) {
                (true, _, Role::Launch) => self.fault(Refusal::LaunchInReach),
                (true, _, Role::Program | Role::Generated | Role::Support) | (false, true, _) => {
                    self.fault(Refusal::BindOverlapsReach)
                }
                (false, false, _) => {}
            }
        }
    }

    /// The one observation every object the walk reaches takes, a source's
    /// own or one found beneath it, whatever it is and however many links
    /// it has. MB3's link rule is read from its facts first, so a linked
    /// program file keeps that cause whether it opens or not. Then `at`
    /// must open, a regular file and a link as a path handle and a
    /// directory for listing, and be the object observed, of the kind
    /// observed, or identity is not protected; a special file never is. A
    /// regular file is then reopened for reading through that handle
    /// ([`readable`]), and the handle returned is that reader, where one
    /// opened. One that cannot be read is a program or bootstrap file's
    /// identity cause; a support file, linked or unreadable, is held to the
    /// kernel write-exclusion proof, its ACL read through its handle, and
    /// unreadable it must be root's as well ([`untouchable`]).
    fn object(&mut self, at: At<'_>, facts: &Facts, role: Role) -> Option<OwnedFd> {
        let (dir, place, name) = at;
        let linked = facts.nlink > 1;
        let program = match role {
            Role::Launch | Role::Program | Role::Generated => true,
            Role::Support => false,
        };
        let flags = match facts.kind() {
            Kind::File if linked & program => {
                self.fault(Refusal::Linked);
                OFlags::PATH
            }
            Kind::File | Kind::Link => OFlags::PATH,
            Kind::Dir => OFlags::RDONLY | OFlags::DIRECTORY,
            Kind::Special => {
                self.fault(Refusal::Identity);
                return None;
            }
        };
        (self.host.opening)(place, name);
        let Some(fd) = handle(dir, name, flags, facts) else {
            self.fault(Refusal::Identity);
            return None;
        };
        if !matches!(facts.kind(), Kind::File) {
            return Some(fd);
        }
        let reader = self.charged(readable(fd.as_fd()))?;
        let unread = reader.is_none();
        if program & unread {
            self.fault(Refusal::Identity);
        }
        if !program & (linked | unread) {
            let owned = Owned {
                uid: facts.uid,
                mode: facts.mode,
                acl: acl(fd.as_fd()),
            };
            let writers = &self.host.credentials;
            let proved = match unread {
                true => untouchable(&owned, writers),
                false => excluded(&owned, writers),
            };
            if !proved {
                self.fault(Refusal::Identity);
            }
        }
        Some(reader.unwrap_or(fd))
    }

    /// The mount and device an object lies on, kept the first time it is
    /// seen, and whether the walk crossed into it from the mount its root
    /// lies on (`root` is whether the object is a source's root): a mount
    /// seen with a second device is proved against the table, where its
    /// record names one.
    fn stood(&mut self, facts: &Facts, root: bool) {
        let nested = !root & (facts.mount != self.view);
        *self.seen.entry((facts.mount, facts.dev)).or_insert(nested) |= nested;
    }

    /// Walk the directory source `root` holds at `place` through one
    /// listing buffer, holding open only the active ancestor stack, whose
    /// first frame is the root's own and is never left: each directory is
    /// listed whole when it is entered, every entry observed, and its
    /// subdirectories kept until they are. A listing or an entry that
    /// cannot be read refuses and ends its directory's listing. The root's
    /// handle, back from its frame.
    fn directory(&mut self, root: OwnedFd, place: &Path, role: Role) -> OwnedFd {
        let mut buffer = vec![MaybeUninit::uninit(); LISTING];
        let mut stack = vec![self.frame(root, place.to_path_buf(), role, &mut buffer)];
        while !self.over {
            let depth = stack.len();
            let Some(found) = stack[depth - 1].pending.pop() else {
                if depth == 1 {
                    break;
                }
                stack.pop();
                continue;
            };
            if depth > self.host.limits.depth {
                self.over = true;
                self.fault(Refusal::Identity);
                break;
            }
            let top = &stack[depth - 1];
            let at = (top.fd.as_fd(), top.place.as_path(), found.name.as_os_str());
            let Some(fd) = self.object(at, &found.facts, role) else {
                continue;
            };
            let child = self.frame(fd, found.place, role, &mut buffer);
            stack.push(child);
        }
        stack.swap_remove(0).fd
    }

    /// The directory `fd` holds at `place`, listed whole through `buffer`
    /// and its entries observed in byte order: each directory among them
    /// kept to be entered.
    fn frame(
        &mut self,
        fd: OwnedFd,
        place: PathBuf,
        role: Role,
        buffer: &mut [MaybeUninit<u8>],
    ) -> Frame {
        let mut pending = Vec::new();
        for name in self.listed(fd.as_fd(), buffer) {
            let found = self.entry((fd.as_fd(), &place, &name), role);
            let Some(found) = self.charged(found) else {
                break;
            };
            pending.extend(found);
        }
        Frame { fd, place, pending }
    }

    /// The names the directory `fd` holds lists, but `.` and `..`, each
    /// counted as it is read, and then sorted: none past an entry that
    /// cannot be read or the entry bound.
    fn listed(&mut self, fd: BorrowedFd<'_>, buffer: &mut [MaybeUninit<u8>]) -> Vec<OsString> {
        let mut names = Vec::new();
        let mut listing = RawDir::new(fd, buffer);
        while let Some(listed) = listing.next() {
            let named = listed.map_err(|_| Refusal::Identity).and_then(|entry| {
                let name = entry.file_name().to_bytes();
                match (name == b".") | (name == b"..") {
                    true => Ok(None),
                    false => self
                        .count()
                        .map(|()| Some(OsStr::from_bytes(name).to_owned())),
                }
            });
            let Some(named) = self.charged(named) else {
                break;
            };
            names.extend(named);
        }
        names.sort_unstable();
        names
    }

    /// One entry `at` of a listed directory: its mount stood on, observed,
    /// and digested with its link's target, which is read through the
    /// link's own handle; found where it is a directory, which is observed
    /// when it is entered.
    fn entry(&mut self, at: At<'_>, role: Role) -> Result<Option<Found>, Refusal> {
        let (dir, place, name) = at;
        let facts = stat_at(dir, name).map_err(|_| Refusal::Identity)?;
        self.stood(&facts, false);
        let link = match facts.kind() {
            Kind::Dir => {
                digest(&mut self.hasher, (place, name), &[&facts.bytes()], b"");
                let found = Found {
                    name: name.to_owned(),
                    facts,
                    place: place.join(name),
                };
                return Ok(Some(found));
            }
            Kind::File | Kind::Special => {
                self.object(at, &facts, role);
                None
            }
            Kind::Link => match self.object(at, &facts, role) {
                Some(held) => Some(target(held.as_fd(), facts.size)?),
                None => None,
            },
        };
        if let Some(link) = &link {
            self.aimed(place, link);
        }
        let target = link.as_deref().unwrap_or_default();
        digest(&mut self.hasher, (place, name), &[&facts.bytes()], target);
        Ok(None)
    }

    /// A link inside a source adds no bind, but one whose target lies in
    /// the seat's reach refuses even where nothing mounts it there.
    fn aimed(&mut self, dir: &Path, link: &[u8]) {
        let target = lexical(dir, Path::new(OsStr::from_bytes(link)));
        if self.reach.iter().any(|root| target.starts_with(root)) {
            self.fault(Refusal::BindOverlapsReach);
        }
    }

    /// The mount table, parsed within its bound, and every mount the walk
    /// stood on proved against it and digested in the order of its place,
    /// root and device, never the table's own.
    fn mounts(&mut self) {
        let Some(mut table) = (self.host.mountinfo)() else {
            return self.fault(Refusal::Identity);
        };
        let mut text = Vec::new();
        let read = table.read_to_end(&mut text).map_err(|_| Refusal::Identity);
        let parsed = read.and_then(|_| records(&text, self.host.limits.mounts));
        let Some(records) = self.charged(parsed) else {
            return;
        };
        self.records = records.len() as u64;
        if !mapped(&records, &self.seen, &self.placed, &self.reach) {
            self.fault(Refusal::Identity);
        }
        let mut used: Vec<&Record> = records
            .iter()
            .filter(|record| self.seen.keys().any(|(id, _)| *id == record.id))
            .collect();
        used.sort_unstable_by_key(|record| (&record.point, &record.root, record.dev));
        for record in used {
            let dev = words::<16>(&[record.dev.0, record.dev.1].map(u64::from));
            let facts = [record.root.as_os_str().as_bytes(), &dev];
            digest(
                &mut self.hasher,
                (&record.point, OsStr::new("")),
                &facts,
                b"",
            );
        }
    }

    /// The second resolution of `source`: absent where it was absent, and
    /// otherwise the same route, ending on the object its handle holds.
    fn again(&mut self, source: &Source<'_>, first: Option<&Held>) {
        let second = resolve(source.path, self.host);
        // Only a resolution that names nothing again proves the source
        // still absent: one that now fails is no longer known to be.
        let same = match (first, second) {
            (None | Some(_), Err(_)) | (None, Ok(Some(_))) | (Some(_), Ok(None)) => false,
            (None, Ok(None)) => true,
            (Some(first), Ok(Some(second))) => {
                let held = stat_at(first.fd.as_fd(), c"").map(|facts| facts.identity());
                let routed = route(&first.chain) == route(&second.chain);
                routed & (held == Ok(second.facts.identity()))
            }
        };
        if !same {
            self.fault(Refusal::Identity);
        }
    }

    /// The observation's verdict, or MB3's first cause.
    fn verdict(self) -> Result<Sources, Refusal> {
        let digest = hex::encode(self.hasher.finalize());
        let sources = Sources {
            entries: self.entries,
            mounts: self.records,
            digest,
        };
        self.fault.map_or(Ok(sources), Err)
    }
}

/// The first bubblewrap with `--ro-bind-fd` and a tmpfs's `--perms`.
const DESCRIPTOR_MOUNTS: (u32, u32, u32) = (0, 5, 0);

/// Observe on `host` what one server box mounts and runs (U6c5a): the
/// launch name and the program tree as program files, every source
/// `namespace` mounts (the system set and resolver as optional support,
/// the identity generated in `made`, where it could be, as generated files,
/// the rest required program files) and the launcher as support; the
/// launch must still name the executable, and the launcher mount
/// descriptors.
pub(super) fn served(
    namespace: &Namespace,
    made: Option<&Path>,
    program: &ServerProgram,
    profile: &ServerProfile<'_>,
    host: &Host<'_>,
) -> Result<Observed, Refusal> {
    let source = |path, role, presence| Source {
        path,
        role,
        presence,
    };
    let launcher = (host.launcher)();
    let support: Vec<&Path> = Profile::Server
        .system()
        .chain([RESOLV_CONF])
        .map(Path::new)
        .collect();
    let mut list = vec![source(&program.launch, Role::Launch, Presence::Required)];
    if let Tree::Package { root } = &program.tree {
        list.push(source(root, Role::Program, Presence::Required));
    }
    let bound = namespace.binds.iter().map(|bind| {
        let path = bind.host.as_path();
        let generated = made.is_some_and(|made| path.starts_with(made));
        match (support.contains(&path), generated) {
            (true, _) => source(path, Role::Support, Presence::Optional),
            (false, true) => source(path, Role::Generated, Presence::Required),
            (false, false) => source(path, Role::Program, Presence::Required),
        }
    });
    let launched = launcher
        .iter()
        .map(|(bwrap, _)| source(bwrap, Role::Support, Presence::Required));
    for next in bound.chain(launched) {
        if !list.iter().any(|held| held.path == next.path) {
            list.push(next);
        }
    }
    let observation = observe(&list, profile.reach, host)?;
    let launch = observation.held.first().and_then(Option::as_ref);
    let same = launch.is_some_and(|(_, place)| *place == program.executable);
    same.then_some(()).ok_or(Refusal::Identity)?;
    let version = launcher
        .as_ref()
        .and_then(|(_, reported)| crate::hands::parse_version(reported));
    let mounts = version.is_some_and(|version| version >= DESCRIPTOR_MOUNTS);
    mounts.then_some(()).ok_or(Refusal::Unavailable)?;
    let held = list.iter().zip(observation.held);
    let handles = held.filter_map(|(source, held)| Some((source.path.to_path_buf(), held?.0)));
    Ok((handles.collect(), observation.sources))
}

/// `target` read from a link in `dir`, as a path, `..` taken lexically.
fn lexical(dir: &Path, target: &Path) -> PathBuf {
    let mut place = match target.is_absolute() {
        true => PathBuf::from("/"),
        false => dir.to_path_buf(),
    };
    for part in target.components() {
        match part {
            Component::Normal(name) => place.push(name),
            Component::ParentDir => {
                place.pop();
            }
            Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
        }
    }
    place
}
