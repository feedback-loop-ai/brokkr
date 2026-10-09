//! What the source observer reads of its host, and MB3's proofs over it
//! (decision 0065 slice two, U6c5a): each source's whole resolution chain
//! through descriptors, a file's own facts read through its descriptor,
//! the mount table and its alias mapping, the managed writers' credentials
//! and the kernel write-exclusion predicate, and where the box launcher
//! lies, found without running it; [`super::launcher`] checks and runs it
//! (U6c5c). The observer's memory is bounded by its counts alone (MB3,
//! operator ruling 2026-10-07): entries, depth, links per resolution and
//! mount records.

use std::collections::{BTreeMap, VecDeque};
use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, PathBuf};

use rustix::fs::{Mode, OFlags};
use rustix::io::Errno;

use super::launcher::{version, Checked};
use super::{handle, owned, stat_at, Facts, Identity, Kind, Limits, LIMITS};
use crate::broker::Refusal;

/// MB4's store identity, beside the resolution and alias facts it reads
/// (U6c5c). It lies beside this file, a child of the observer's: declared
/// here, as sources.rs stands at its 800-line ceiling.
#[path = "store.rs"]
pub(in crate::hands) mod store;

/// How a regular file is opened to be read: never blocking on a writer
/// and never taking a terminal.
const READ: OFlags = OFlags::RDONLY
    .union(OFlags::NONBLOCK)
    .union(OFlags::NOCTTY)
    .union(OFlags::CLOEXEC);

/// The longest link target Linux makes, `PATH_MAX` less its NUL: a link
/// whose facts give a longer one is no link this observer knows.
const LONGEST: usize = 4095;

/// The target of the link `fd` holds, read into room for one byte more
/// than the `size` its facts gave it, which is first held to [`LONGEST`]:
/// one that reads at any other length is no longer the link observed.
pub(super) fn target(fd: BorrowedFd<'_>, size: u64) -> Result<Vec<u8>, Refusal> {
    let size = usize::try_from(size).ok().filter(|size| *size <= LONGEST);
    let size = size.ok_or(Refusal::Identity)?;
    let mut target = vec![0; size + 1];
    let read = rustix::fs::readlinkat_raw(fd, c"", &mut target[..]);
    let read = read.map_err(|_| Refusal::Identity)?;
    target.truncate(read);
    (read == size).then_some(target).ok_or(Refusal::Identity)
}

/// The kernel's own name for the object `fd` holds: opening it, or reading
/// an attribute through it, looks up no name the host could replace.
fn named(fd: BorrowedFd<'_>) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}

/// A readable handle on the regular file the path handle `held` holds,
/// reopened through that handle, so no name is looked up again and no
/// device a name might now give is ever opened: none where the file's
/// mode or ACL denies this process a read, refused on any other failure.
pub(super) fn readable(held: BorrowedFd<'_>) -> Result<Option<OwnedFd>, Refusal> {
    match rustix::fs::open(named(held), READ, Mode::empty()) {
        Ok(fd) => Ok(Some(fd)),
        Err(errno) => (errno == Errno::ACCESS)
            .then_some(None)
            .ok_or(Refusal::Identity),
    }
}

/// The managed writers' credentials as this process's user namespace maps
/// them: their uids, the overflow uid an unmapped owner reads as, and
/// whether their privilege is confined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::hands) struct Credentials {
    pub(in crate::hands) uids: Vec<u32>,
    pub(in crate::hands) overflow: u32,
    pub(in crate::hands) confinement: Confinement,
}

impl Credentials {
    /// These writers and the `sealed` ones the plan names, each uid once:
    /// confined as the observed writers are, the plan having sealed its
    /// own writers confined.
    pub(in crate::hands) fn sealed(&self, sealed: &[u32]) -> Credentials {
        let mut all = self.clone();
        all.uids.extend(sealed);
        all.uids.sort_unstable();
        all.uids.dedup();
        all
    }
}

/// Whether no writer is root or holds any capability, nor can gain either
/// at exec (`no_new_privs`); an unread fact leaves it unproved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Confinement {
    Proved,
    Unproved,
}

/// A file's extended access ACL, as its descriptor reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Acl {
    Absent,
    Present,
    Unknown,
}

/// The write facts of a support file that is multiply linked or that this
/// process cannot read, or of a hop on a route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) struct Owned {
    pub(in crate::hands) uid: u32,
    pub(in crate::hands) mode: u32,
    pub(in crate::hands) acl: Acl,
}

/// MB3's kernel write-exclusion proof for a multiply-linked support file:
/// every writer and the owner mapped, the owner none of the writers, no
/// group or other write bit, no extended access ACL, and the writers'
/// privilege confined. Ownership and modes belong to the inode, so the
/// proof holds for every name the file has, enumerated or not.
pub(in crate::hands) fn excluded(file: &Owned, writers: &Credentials) -> bool {
    let uids = &writers.uids;
    let mapped = !uids.is_empty() & !uids.contains(&writers.overflow);
    let owned = file.uid != writers.overflow;
    let owner = !uids.contains(&file.uid);
    let mode = file.mode & 0o022 == 0;
    let acl = file.acl == Acl::Absent;
    let privilege = writers.confinement == Confinement::Proved;
    mapped & owned & owner & mode & acl & privilege
}

/// MB3's proof for a support file this process cannot read, observed
/// without reading it: owned by root, and the write-exclusion proof whole,
/// so no managed writer can write it (operator ruling 2026-10-07).
pub(in crate::hands) fn untouchable(file: &Owned, writers: &Credentials) -> bool {
    (file.uid == 0) & excluded(file, writers)
}

/// Whether the file `fd` holds has an extended access ACL, read through
/// the kernel's name for it, which needs no read access to the file: a
/// path handle serves as well as an open file.
pub(in crate::hands) fn acl(fd: BorrowedFd<'_>) -> Acl {
    let mut value = [0u8; 0];
    match rustix::fs::getxattr(named(fd), "system.posix_acl_access", &mut value[..]) {
        Ok(_) | Err(Errno::RANGE) => Acl::Present,
        Err(Errno::NODATA) => Acl::Absent,
        Err(_) => Acl::Unknown,
    }
}

/// One record of the mount table, as MB3 reads it: its id and device,
/// where in its filesystem its root lies, where it lies in this namespace,
/// whether its own option is `rw`, and whether its filesystem is one of
/// [`LOCAL`], whose device and inode name one object here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::hands) struct Record {
    pub(in crate::hands) id: u64,
    pub(in crate::hands) dev: (u32, u32),
    pub(in crate::hands) root: PathBuf,
    pub(in crate::hands) point: PathBuf,
    pub(in crate::hands) writable: bool,
    pub(in crate::hands) local: bool,
}

/// The local filesystems whose identity the observer proves; overlay,
/// FUSE, network and anything else are not proved.
const LOCAL: [&str; 9] = [
    "ext2", "ext3", "ext4", "xfs", "btrfs", "f2fs", "tmpfs", "squashfs", "erofs",
];

/// The mount table `text`, strictly: each line its six fields, optional
/// fields, a lone `-` and exactly three more; `rw` or `ro` first among its
/// options; octal escapes only; no id twice; no more records than `bound`,
/// which is checked before any is kept.
pub(in crate::hands) fn records(text: &[u8], bound: usize) -> Result<Vec<Record>, Refusal> {
    let body = text.strip_suffix(b"\n").ok_or(Refusal::Identity)?;
    let lines = body.split(|byte| *byte == b'\n');
    let count = lines.clone().count();
    (count <= bound).then_some(()).ok_or(Refusal::Identity)?;
    let mut records = Vec::with_capacity(count);
    let mut ids = Vec::with_capacity(count);
    for line in lines {
        let record = record(line)?;
        ids.push(record.id);
        records.push(record);
    }
    ids.sort_unstable();
    let unique = ids.windows(2).all(|pair| pair[0] != pair[1]);
    unique.then_some(records).ok_or(Refusal::Identity)
}

/// One mount table line, refused where any field is malformed.
fn record(line: &[u8]) -> Result<Record, Refusal> {
    let mut fields = line.split(|byte| *byte == b' ');
    let mut six = [&b""[..]; 6];
    for field in &mut six {
        *field = fields.next().ok_or(Refusal::Identity)?;
    }
    let found = fields.by_ref().any(|field| field == b"-");
    let tail = [fields.next(), fields.next(), fields.next(), fields.next()];
    let [id, parent, dev, root, point, options] = six;
    let ([Some(filesystem), Some(_), Some(_), None], true, false) =
        (tail, found, six.contains(&&b"-"[..]))
    else {
        return Err(Refusal::Identity);
    };
    number(parent).ok_or(Refusal::Identity)?;
    let local = LOCAL.iter().any(|name| name.as_bytes() == filesystem);
    let (root, point) = (unescaped(root), unescaped(point));
    let (Some(root), Some(point)) = (root, point) else {
        return Err(Refusal::Identity);
    };
    // A pseudo filesystem's root names no path (nsfs: `mnt:[4026532867]`);
    // a local one's always does.
    let named = point.is_absolute() & (root.is_absolute() | !local);
    let parsed = (
        named.then_some(()),
        number(id),
        device(dev),
        writable(options),
    );
    let (Some(()), Some(id), Some(dev), Some(writable)) = parsed else {
        return Err(Refusal::Identity);
    };
    Ok(Record {
        id,
        dev,
        root,
        point,
        writable,
        local,
    })
}

/// A `major:minor` device, each part decimal.
fn device(dev: &[u8]) -> Option<(u32, u32)> {
    let (major, minor) = dev.split_at(dev.iter().position(|byte| *byte == b':')?);
    let major = number(major)?.try_into().ok()?;
    Some((major, number(&minor[1..])?.try_into().ok()?))
}

/// Whether a mount's own options, `rw` or `ro` first, let it be written.
fn writable(options: &[u8]) -> Option<bool> {
    match options.split(|byte| *byte == b',').next()? {
        b"rw" => Some(true),
        b"ro" => Some(false),
        _ => None,
    }
}

/// Decimal digits and nothing else.
fn number(text: &[u8]) -> Option<u64> {
    let digits = !text.is_empty() & text.iter().all(u8::is_ascii_digit);
    digits.then(|| std::str::from_utf8(text).ok()?.parse().ok())?
}

/// A mount table path with its `\ooo` escapes decoded.
fn unescaped(text: &[u8]) -> Option<PathBuf> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text;
    while let Some((first, tail)) = rest.split_first() {
        rest = tail;
        if *first != b'\\' {
            bytes.push(*first);
            continue;
        }
        let (digits, tail) = (rest.get(..3)?, &rest[3..]);
        let octal = digits.iter().all(|digit| (b'0'..=b'7').contains(digit));
        let value =
            octal.then(|| u8::from_str_radix(std::str::from_utf8(digits).ok()?, 8).ok())??;
        bytes.push(value);
        rest = tail;
    }
    Some(PathBuf::from(OsString::from_vec(bytes)))
}

/// Every place outside the mount `own` where the object at `inside` on
/// `dev` is mounted too, or part of it is, as a record's point and the
/// rest of the way below it: another record of that device whose root
/// holds `inside` shows it at that point plus the rest ([`holding`]), and
/// one whose root lies inside it shows that part at its point. Neither a
/// mount id nor a canonical spelling hides one, and none is built as a
/// path.
pub(in crate::hands) fn aliases<'r>(
    records: &'r [Record],
    own: u64,
    dev: (u32, u32),
    inside: &'r Path,
) -> impl Iterator<Item = (&'r Path, &'r Path)> + 'r {
    let below = move |record: &'r Record| {
        let part = record.root.starts_with(inside);
        part.then_some((record.point.as_path(), Path::new("")))
    };
    let alias = move |record: &'r Record| held(record, inside).or_else(|| below(record));
    others(records, own, dev).filter_map(alias)
}

/// The places of [`aliases`] another mount's root holding `inside` gives,
/// alone: all a route's hop needs. A mount whose root lies below a hop
/// shows only part of it, which the route passes through only where that
/// part is a later hop, whose own holding mounts show it.
pub(in crate::hands) fn holding<'r>(
    records: &'r [Record],
    own: u64,
    dev: (u32, u32),
    inside: &'r Path,
) -> impl Iterator<Item = (&'r Path, &'r Path)> + 'r {
    others(records, own, dev).filter_map(move |record| held(record, inside))
}

/// Every record of the device `dev` but the mount `own`.
fn others(records: &[Record], own: u64, dev: (u32, u32)) -> impl Iterator<Item = &Record> {
    records
        .iter()
        .filter(move |record| (record.id != own) & (record.dev == dev))
}

/// Where `record` shows the object at `inside`, where its root holds it.
fn held<'r>(record: &'r Record, inside: &'r Path) -> Option<(&'r Path, &'r Path)> {
    let rest = inside.strip_prefix(&record.root).ok()?;
    Some((record.point.as_path(), rest))
}

/// Whether the path `head` then `tail` lies within `root` or holds it:
/// one is the other's prefix, component by component.
fn overlapping(head: &Path, tail: &Path, root: &Path) -> bool {
    let path = head.components().chain(tail.components());
    path.zip(root.components()).all(|(one, other)| one == other)
}

/// Every mount the walk stood on, by its id and each device an object on
/// it showed, and whether a walk crossed into it from the mount its own
/// root lies on.
pub(in crate::hands) type Seen = BTreeMap<(u64, (u32, u32)), bool>;

/// Whether every mount the walk stood on is proved: in the table once,
/// each device the walk saw on it the record's, a local filesystem,
/// read-only where nested inside a source, whatever root lies on it too,
/// and with no alias of any walked object in `reach`. `placed` is each
/// source root's canonical place and mount.
pub(in crate::hands) fn mapped(
    records: &[Record],
    seen: &Seen,
    placed: &[(PathBuf, u64)],
    reach: &[PathBuf],
) -> bool {
    let find = |id: u64| records.iter().find(|record| record.id == id);
    let clear = |record: &Record, inside: &Path| {
        let mut aliases = aliases(records, record.id, record.dev, inside);
        !aliases.any(|(point, rest)| reach.iter().any(|root| overlapping(point, rest, root)))
    };
    let mounts = seen.iter().all(|(&(id, dev), &nested)| {
        find(id).is_some_and(|record| {
            let known = (record.dev == dev) & record.local;
            known & (!nested | (!record.writable & clear(record, &record.root)))
        })
    });
    let roots = placed.iter().all(|(place, id)| {
        let held =
            find(*id).and_then(|record| Some((record, place.strip_prefix(&record.point).ok()?)));
        held.is_some_and(|(record, rest)| clear(record, &record.root.join(rest)))
    });
    mounts & roots
}

/// What the observer reads beside its sources: its bounds, the writers'
/// credentials, the mount table, the box launcher as
/// [`super::launcher::launched`] checked it, where the launcher lies,
/// found without running it, the loader's preload file and cache, how the
/// checked launcher is run for its version, a step run between the two
/// resolutions of every source and between the launcher's admission and
/// its run, and one run with the directory and name of each object an
/// observation is about to open; in a test alone, the places a launcher
/// is granted the system set's membership at.
pub(in crate::hands) struct Host<'h> {
    pub(in crate::hands) limits: Limits,
    pub(in crate::hands) credentials: Credentials,
    pub(in crate::hands) mountinfo: &'h dyn Fn() -> Option<Box<dyn Read>>,
    pub(in crate::hands) launcher: Option<&'h Checked>,
    pub(in crate::hands) found: &'h dyn Fn() -> Option<PathBuf>,
    pub(in crate::hands) loader: [&'h Path; 2],
    pub(in crate::hands) run: &'h dyn Fn(BorrowedFd<'_>) -> String,
    pub(in crate::hands) between: &'h dyn Fn(),
    pub(in crate::hands) opening: &'h dyn Fn(&Path, &OsStr),
    #[cfg(test)]
    pub(in crate::hands) admitted: &'h dyn Fn(&Path) -> bool,
}

impl Host<'static> {
    /// This host: MB3's bounds, the managed writers this process observes
    /// ([`credentials`]), `/proc/self/mountinfo`, the bubblewrap `PATH`
    /// names, glibc's loader files, the launcher run through its handle
    /// ([`version`]), and no launcher until one is checked.
    pub(in crate::hands) fn live() -> Host<'static> {
        Host {
            limits: LIMITS,
            credentials: credentials(),
            mountinfo: &mount_table,
            launcher: None,
            found: &|| crate::hands::require_bwrap().ok(),
            loader: ["/etc/ld.so.preload", "/etc/ld.so.cache"].map(Path::new),
            run: &version,
            between: &|| {},
            opening: &|_, _| {},
            #[cfg(test)]
            admitted: &|_| false,
        }
    }
}

/// The mount table, whose records the observer counts against its bound.
fn mount_table() -> Option<Box<dyn Read>> {
    Some(Box::new(std::fs::File::open("/proc/self/mountinfo").ok()?))
}

/// `path` resolved on `host`, its object opened as a path handle that must
/// be the object resolved: the resolution, the handle and the facts it
/// reports; identity unprotected where it is absent or another object.
pub(super) fn opened(path: &Path, host: &Host<'_>) -> Result<(Resolved, OwnedFd, Facts), Refusal> {
    let resolved = resolve(path, host)?.ok_or(Refusal::Identity)?;
    let (dir, name) = (resolved.dir.as_fd(), &resolved.name);
    (host.opening)(resolved.place.parent().unwrap_or(&resolved.place), name);
    let held = handle(dir, name, OFlags::PATH, &resolved.facts);
    let (fd, facts) = held.ok_or(Refusal::Identity)?;
    Ok((resolved, fd, facts))
}

/// The managed writers this observer reads: itself, the broker that
/// started it and that broker's harness, each from its
/// `/proc/<pid>/status` beside the kernel's overflow uid. A parent counts
/// only where it was still the parent once its status was read: one that
/// ended may have left its pid to another process.
fn credentials() -> Credentials {
    let overflow = std::fs::read_to_string("/proc/sys/fs/overflowuid").ok();
    let own = status("self");
    let broker = parent("self", &status);
    let harness = broker.as_ref().and_then(|(pid, _)| parent(pid, &status));
    let statuses = [own, broker.map(|held| held.1), harness.map(|held| held.1)];
    let read = statuses.each_ref().map(Option::as_deref);
    credited(overflow.as_deref(), &read)
}

/// The `/proc/<pid>/status` text of `pid`.
fn status(pid: &str) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/status")).ok()
}

/// The pid of `pid`'s parent and its status, each read by `status`, while
/// it stayed that parent: `pid`'s own status names the same parent before
/// and after.
pub(in crate::hands) fn parent(
    pid: &str,
    status: &dyn Fn(&str) -> Option<String>,
) -> Option<(String, String)> {
    let ppid = || Some(field(&status(pid)?, "PPid:")?.to_string());
    let parent = ppid()?;
    let held = status(&parent)?;
    (ppid()? == parent).then_some((parent, held))
}

/// The credentials of the writers whose `statuses` were read, from the
/// kernel's `overflow` uid file: every uid each holds (real, effective,
/// saved and filesystem), any of which it may act as. A fact that does not
/// read leaves no writer known, so no linked support file passes. Their
/// privilege is confined only where each one's every path to more is
/// closed and observed: no uid is root, its permitted, effective and
/// ambient capability sets are all empty, and `no_new_privs` is set, so no
/// exec gains a setuid owner's identity or a file's capabilities.
/// `no_new_privs` removes no capability already held, and no held one is
/// proved unable to write a file (a module, raw I/O, another process's
/// memory, a device node, BPF), so any at all, like anything else unread or
/// not, leaves them unconfined.
pub(in crate::hands) fn credited(overflow: Option<&str>, statuses: &[Option<&str>]) -> Credentials {
    let overflow: Option<u32> = overflow.and_then(|text| text.strip_suffix('\n')?.parse().ok());
    let writers: Option<Vec<(Vec<u32>, bool)>> = statuses
        .iter()
        .map(|status| status.and_then(writer))
        .collect();
    let (Some(writers), Some(overflow)) = (writers.filter(|all| !all.is_empty()), overflow) else {
        return Credentials {
            uids: Vec::new(),
            overflow: u32::MAX,
            confinement: Confinement::Unproved,
        };
    };
    let confinement = match writers.iter().all(|(_, confined)| *confined) {
        true => Confinement::Proved,
        false => Confinement::Unproved,
    };
    let held = writers.into_iter().flat_map(|(ids, _)| ids);
    let uids: std::collections::BTreeSet<u32> = held.collect();
    Credentials {
        uids: uids.into_iter().collect(),
        overflow,
        confinement,
    }
}

/// One writer's uids from its `status`, and whether its privilege is
/// confined; none where its uids do not read.
fn writer(status: &str) -> Option<(Vec<u32>, bool)> {
    let ids = uids(status)?;
    let capless = capabilities(status) == Some(0);
    let fenced = field(status, "NoNewPrivs:") == Some("1");
    let confined = capless & !ids.contains(&0) & fenced;
    Some((ids, confined))
}

/// The value of the one line of a `/proc/<pid>/status` text that `key`
/// begins; none where no line or a second one does.
fn field<'s>(status: &'s str, key: &str) -> Option<&'s str> {
    let mut lines = status.lines().filter_map(|line| line.strip_prefix(key));
    let value = lines.next()?.trim();
    lines.next().is_none().then_some(value)
}

/// The distinct real, effective, saved and filesystem uids of a
/// `/proc/<pid>/status` text: exactly four decimal fields.
fn uids(status: &str) -> Option<Vec<u32>> {
    let fields: Vec<&str> = field(status, "Uid:")?.split_whitespace().collect();
    let parsed: Option<std::collections::BTreeSet<u32>> =
        fields.iter().map(|uid| uid.parse().ok()).collect();
    (fields.len() == 4).then(|| parsed.map(|ids| ids.into_iter().collect()))?
}

/// The permitted, effective and ambient capability sets of a
/// `/proc/<pid>/status` text, together, each read exactly once.
pub(in crate::hands) fn capabilities(status: &str) -> Option<u64> {
    ["CapPrm:", "CapEff:", "CapAmb:"]
        .iter()
        .try_fold(0, |held, key| {
            Some(held | u64::from_str_radix(field(status, key)?, 16).ok()?)
        })
}

/// One step a resolution took: a directory it entered, a link it read or
/// the object it ended on, where it lay and what it was, who may change it
/// (a directory's facts and ACL read through its own handle) and the
/// mount it lies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::hands) struct Hop {
    pub(in crate::hands) place: PathBuf,
    pub(in crate::hands) identity: Identity,
    pub(in crate::hands) link: Option<Vec<u8>>,
    pub(in crate::hands) owned: Owned,
    pub(in crate::hands) mount: u64,
}

/// The hops of `chain` before the object itself.
pub(super) fn route(chain: &[Hop]) -> &[Hop] {
    &chain[..chain.len().saturating_sub(1)]
}

/// A source resolved, its object not yet opened: the directory handle it
/// is opened from and its name there (`.` where the object is that
/// directory itself), its canonical place, every hop on the way, and the
/// object's facts. It follows every link and refuses a special file, so it
/// ends on a directory or a regular file.
pub(super) struct Resolved {
    pub(super) dir: OwnedFd,
    pub(super) name: OsString,
    pub(super) place: PathBuf,
    pub(super) chain: Vec<Hop>,
    pub(super) facts: Facts,
}

/// The host's root, which every resolution starts from: a path handle,
/// closed on exec as every handle the observer takes is.
pub(in crate::hands) fn root() -> rustix::io::Result<OwnedFd> {
    let flags = OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC;
    rustix::fs::open("/", flags, Mode::empty())
}

/// `path` resolved on `host` from `/` one component at a time, each link
/// read through its own handle and followed here rather than by the
/// kernel, `..` leaving the directory actually entered; none where a
/// component is absent; refused where it is not absolute, ends on a
/// special file, takes more than the bound's links, a link or directory
/// cannot be read, or what a component's handle opens is not what it
/// observed.
pub(super) fn resolve(path: &Path, host: &Host<'_>) -> Result<Option<Resolved>, Refusal> {
    path.is_absolute().then_some(()).ok_or(Refusal::Identity)?;
    let start = root().map_err(|_| Refusal::Identity)?;
    let mut walk = Walk {
        stack: vec![(start, PathBuf::from("/"))],
        queue: parts(path),
        chain: Vec::new(),
        links: 0,
        file: None,
    };
    while let Some(part) = walk.queue.pop_front() {
        if !walk.step(part, host)? {
            return Ok(None);
        }
    }
    walk.end().map(Some)
}

/// One resolution in progress: the directories entered, the components
/// still to take, every hop so far, the links read, and the regular file
/// it ended on, where it did.
struct Walk {
    stack: Vec<(OwnedFd, PathBuf)>,
    queue: VecDeque<OsString>,
    chain: Vec<Hop>,
    links: usize,
    file: Option<(OsString, Facts)>,
}

/// What one step took: a link to follow and its target, a directory it
/// entered and who may write it, as its handle reports, or the regular
/// file the resolution ends on.
enum Taken {
    Link(Vec<u8>),
    Dir(OwnedFd, Owned),
    File,
}

impl Walk {
    /// Take the component `part`: back out of a directory, follow a link
    /// or enter the object it names; false where it names nothing.
    fn step(&mut self, part: OsString, host: &Host<'_>) -> Result<bool, Refusal> {
        if part == ".." {
            self.stack.truncate((self.stack.len() - 1).max(1));
            return Ok(true);
        }
        let (dir, at) = &self.stack[self.stack.len() - 1];
        let Some(facts) = lookup(dir.as_fd(), &part)? else {
            return Ok(false);
        };
        let place = at.join(&part);
        let ending = (self.queue.is_empty(), self.links == host.limits.hops);
        let taken = take((dir.as_fd(), at, &part), &facts, ending, host)?;
        let Some(taken) = taken else {
            return Ok(false);
        };
        let (link, entered, owned) = match taken {
            Taken::Link(link) => (Some(link), None, owned(&facts, Acl::Absent)),
            Taken::Dir(fd, held) => (None, Some(fd), held),
            Taken::File => (None, None, owned(&facts, Acl::Absent)),
        };
        let target = link.clone();
        let hop = Hop {
            place,
            identity: facts.identity(),
            link,
            owned,
            mount: facts.mount,
        };
        self.chain.push(hop);
        match (entered, target) {
            (Some(fd), _) => {
                let place = self.chain[self.chain.len() - 1].place.clone();
                self.stack.push((fd, place));
            }
            (None, Some(target)) => self.follow(&target),
            (None, None) => self.file = Some((part, facts)),
        }
        Ok(true)
    }

    /// Go on along the last hop's link target, `link`: from `/` where it
    /// is absolute, from the directory the link lies in otherwise.
    fn follow(&mut self, link: &[u8]) {
        let target = Path::new(OsStr::from_bytes(link));
        if target.is_absolute() {
            self.stack.truncate(1);
        }
        let mut ahead = parts(target);
        ahead.extend(std::mem::take(&mut self.queue));
        self.queue = ahead;
        self.links += 1;
    }

    /// What the walk ended on: the regular file it took last, in the
    /// directory it lay in, or the directory it last entered, itself.
    fn end(mut self) -> Result<Resolved, Refusal> {
        let (dir, at) = self.stack.pop().ok_or(Refusal::Identity)?;
        let (name, place, facts) = match self.file {
            Some((name, facts)) => {
                let place = at.join(&name);
                (name, place, facts)
            }
            None => {
                let facts = stat_at(dir.as_fd(), c"").map_err(|_| Refusal::Identity)?;
                (OsString::from("."), at, facts)
            }
        };
        Ok(Resolved {
            dir,
            name,
            place,
            chain: self.chain,
            facts,
        })
    }
}

/// How a resolution takes the object `facts` observed at `at`: a link read
/// through its own handle (refused once `spent` the bound's links), a
/// directory entered through a path handle, or a regular file ended on
/// where it is `last`; none where a path goes on through a file; a special
/// file it would end on refuses. Each handle must be the object observed.
fn take(
    (dir, place, part): (BorrowedFd<'_>, &Path, &OsStr),
    facts: &Facts,
    (last, spent): (bool, bool),
    host: &Host<'_>,
) -> Result<Option<Taken>, Refusal> {
    let open = |flags| {
        (host.opening)(place, part);
        handle(dir, part, flags, facts).ok_or(Refusal::Identity)
    };
    match (facts.kind(), last) {
        (Kind::Link, _) if spent => Err(Refusal::Identity),
        (Kind::Link, _) => {
            let link = target(open(OFlags::PATH)?.0.as_fd(), facts.size)?;
            Ok(Some(Taken::Link(link)))
        }
        (Kind::Dir, _) => {
            let (fd, held) = open(OFlags::PATH | OFlags::DIRECTORY)?;
            let owned = owned(&held, acl(fd.as_fd()));
            Ok(Some(Taken::Dir(fd, owned)))
        }
        (Kind::File, true) => Ok(Some(Taken::File)),
        (Kind::Special, true) => Err(Refusal::Identity),
        (Kind::File | Kind::Special, false) => Ok(None),
    }
}

/// `part`'s facts in `dir`, none where it is absent or `dir` is no
/// directory; any other failure refuses.
fn lookup(dir: BorrowedFd<'_>, part: &OsStr) -> Result<Option<Facts>, Refusal> {
    match stat_at(dir, part) {
        Ok(facts) => Ok(Some(facts)),
        Err(Errno::NOENT | Errno::NOTDIR) => Ok(None),
        Err(_) => Err(Refusal::Identity),
    }
}

/// A path's components after its root, `..` kept as a step back.
fn parts(path: &Path) -> VecDeque<OsString> {
    let part = |part| match part {
        Component::Normal(name) => Some(name.to_os_string()),
        Component::ParentDir => Some(OsString::from("..")),
        Component::RootDir | Component::CurDir | Component::Prefix(_) => None,
    };
    path.components().filter_map(part).collect()
}
