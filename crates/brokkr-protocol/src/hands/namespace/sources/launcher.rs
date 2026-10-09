//! The box launcher, checked before it could run (decision 0065 slice two,
//! U6c5c; MB3; operator rulings 2026-10-08 and 2026-10-09). It runs on the
//! host, outside any box, so whatever it executes runs with the observer's
//! host access. It is admitted only as an ELF executable of this host's
//! class and machine lying in the protected system set, that no managed
//! writer could write, never a script, whose program interpreter, every
//! `DT_RUNPATH` and `DT_RPATH` entry and every `DT_NEEDED` name holding a
//! slash lie in that set too, as the loader's own preload file and cache
//! do where they exist. Its headers are read through the handle it was
//! checked by, never mapped or run, and admitted only in a layout the
//! operator's closed list of 2026-10-09 names ([`headers`]), every count,
//! range and string within a fixed bound.
//!
//! The route it was found by is admitted beside its final inode: the
//! observation resolves the launcher by that route, its reach and its
//! second resolution included, and before the launcher runs every hop's
//! mount is proved, no other mount shows a hop in the seat's reach, and
//! the route resolves once more to the same hops and the same object. Only
//! then does the handle the observation kept run, under a cleared
//! environment.

use std::ffi::OsString;
use std::fmt;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use rustix::fs::FileType;

use super::super::{spellings, Observed, Profile, ServerProfile};
use super::host::store::{exposed, showing, spelled, table, top};
use super::host::{acl, excluded, opened, readable, resolve, route, Credentials, Hop, Host, Owned};
use super::{owned, stat_at, Kind, Presence, Role, Source};
use crate::broker::{Reach, Refusal};

/// The launcher as [`checked`] admitted it: the path it was found at, the
/// handle it was checked through, and every hop of the route it took.
pub(in crate::hands) struct Checked {
    found: PathBuf,
    fd: OwnedFd,
    chain: Vec<Hop>,
}

impl Checked {
    /// The source the observation takes the launcher as: support, by the
    /// path it was found at, so the route it was found by is observed.
    pub(in crate::hands) fn source(&self) -> Source<'_> {
        Source {
            path: &self.found,
            role: Role::Support,
            presence: Presence::Required,
        }
    }
}

/// What `observe` reads on `host` for `profile`, the box launcher checked
/// and never run (MB3): the writers are those observed and those sealed,
/// and the launcher `host.found` names is [`checked`] and handed to
/// `observe`, for the observation to run once its admission is complete
/// ([`ran`]). A launcher refused here reaches `observe` as none, its cause
/// winning where it precedes `observe`'s; one never found leaves the box
/// unavailable.
pub(in crate::hands) fn launched(
    host: &Host<'_>,
    profile: &ServerProfile<'_>,
    observe: impl FnOnce(&Host<'_>) -> Result<Observed, Refusal>,
) -> Result<Observed, Refusal> {
    let credentials = host.credentials.sealed(profile.writers);
    let host = Host {
        credentials,
        ..*host
    };
    let checked = (host.found)().map(|path| checked(path, &host));
    let launcher = checked.as_ref().and_then(|checked| checked.as_ref().ok());
    let observed = observe(&Host { launcher, ..host });
    match checked {
        Some(Err(cause)) => Err(observed.err().into_iter().fold(cause, Ord::min)),
        Some(Ok(_)) | None => observed,
    }
}

/// The launcher `found` names before it could run: opened as the object
/// its route resolves to, a regular file whose canonical place lies in the
/// server box's protected system set and, by its handle's facts, writable
/// by no managed writer ([`fit`]), a rule stricter than a support file's,
/// and an ELF executable the loader runs only protected code for
/// ([`elf`]). Identity's cause otherwise. A test alone may grant a place
/// outside the set its membership (`host.admitted`), so an image it plants
/// is decided by its headers; no other build has the grant.
fn checked(found: PathBuf, host: &Host<'_>) -> Result<Checked, Refusal> {
    let (resolved, fd, facts) = opened(&found, host)?;
    let owned = owned(&facts, acl(fd.as_fd()));
    #[cfg(test)]
    let granted = (host.admitted)(&resolved.place);
    #[cfg(not(test))]
    let granted = false;
    let ok = fit(&owned, &resolved.place, &host.credentials, granted);
    ok.then_some(()).ok_or(Refusal::Identity)?;
    let dir = resolved.place.parent().ok_or(Refusal::Identity)?;
    elf(fd.as_fd(), dir, host).map_err(|_| Refusal::Identity)?;
    let chain = resolved.chain;
    Ok(Checked { found, fd, chain })
}

/// The version of the launcher `host` checked, none checked reporting
/// none, asked once its source is admitted with the `handles` the
/// observation kept: its route [`routed`] against the seat's `reach`, the
/// route resolving again through the very hops it was checked by, and the
/// handle kept at its found path the object checked, which alone runs, so
/// whatever the path names now never does. Then, after `host.between`,
/// that handle runs (`host.run`).
pub(super) fn ran(
    host: &Host<'_>,
    handles: &[(PathBuf, OwnedFd)],
    reach: &Reach,
) -> Result<String, Refusal> {
    let Some(checked) = host.launcher else {
        return Ok(String::new());
    };
    routed(&checked.chain, reach, host)?;
    let identity = |fd: BorrowedFd<'_>| stat_at(fd, c"").ok().map(|facts| facts.identity());
    let object = identity(checked.fd.as_fd()).ok_or(Refusal::Identity)?;
    let again = resolve(&checked.found, host)?.ok_or(Refusal::Identity)?;
    let same = route(&again.chain) == route(&checked.chain);
    let held = handles.iter().find(|(path, _)| *path == checked.found);
    let held = held.map(|(_, fd)| fd.as_fd());
    let held = held.filter(|held| same & (identity(*held) == Some(object)));
    let held = held.ok_or(Refusal::Identity)?;
    (host.between)();
    Ok((host.run)(held))
}

/// Whether every hop of the launcher's route lies on a mount the table
/// proves (identity) and shows, at its own place or any other mount's
/// ([`showing`]), inside no root of `reach`, each as spelled and as it
/// resolves ([`exposed`], the reach cause), the first by MB3's order.
fn routed(chain: &[Hop], reach: &Reach, host: &Host<'_>) -> Result<(), Refusal> {
    let records = table(host)?;
    let declared = reach.writable.iter().chain(&reach.readable);
    let roots = spelled(declared.map(PathBuf::as_path));
    let causes = route(chain)
        .iter()
        .filter_map(|hop| match showing(&records, hop) {
            Err(unproved) => Some(unproved),
            Ok(places) => exposed(places.iter(), &roots).then_some(Refusal::BindOverlapsReach),
        });
    causes.min().map_or(Ok(()), Err)
}

/// The version the launcher `fd` holds reports: run through that very
/// handle, made its standard input and named `/proc/self/fd/0`, so no name
/// is looked up again, under a cleared environment; empty where it does
/// not run or report.
pub(in crate::hands) fn version(fd: BorrowedFd<'_>) -> String {
    let ran = fd.try_clone_to_owned().ok().and_then(|stdin| {
        let mut command = Command::new("/proc/self/fd/0");
        command.arg("--version").env_clear().stdin(stdin);
        command.stderr(Stdio::null()).output().ok()
    });
    let ran = ran.and_then(|out| String::from_utf8(out.stdout).ok());
    ran.map(|text| text.trim().to_string()).unwrap_or_default()
}

/// Why a launcher's headers do not admit it, each refusing identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(in crate::hands) enum Unfit {
    #[error("the box launcher cannot be read")]
    Unread,
    #[error("the box launcher is a script")]
    Script,
    #[error("the box launcher is not an ELF file")]
    NotElf,
    #[error("the box launcher is not an executable of this host's class and machine")]
    Foreign,
    #[error("the box launcher's headers are malformed")]
    Malformed,
    #[error("the box launcher's headers are truncated")]
    Truncated,
    #[error("the box launcher's headers pass a bound")]
    Over,
    #[error("the box launcher is static, naming no program interpreter or no dynamic segment")]
    Static,
    #[error("the box launcher names an audit library or a filter")]
    Audited,
    #[error("the box launcher's {0} is relative, empty or holds a token")]
    Relative(Input),
    #[error("the box launcher's {0} does not resolve")]
    Unresolved(Input),
    #[error("the box launcher's {0} is not in the protected system set")]
    Unprotected(Input),
}

/// What the loader reads for the launcher beside the launcher itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Input {
    Interpreter,
    SearchPath,
    Needed,
    Preload,
    Cache,
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Input::Interpreter => "program interpreter",
            Input::SearchPath => "library search path",
            Input::Needed => "needed library",
            Input::Preload => "loader preload file",
            Input::Cache => "loader cache",
        })
    }
}

/// The ELF identification a launcher must bear (the ruling's item 1):
/// ELFCLASS64, this host's data encoding and `EI_VERSION` 1; and this
/// host's machine, none where this observer knows no machine.
const IDENT: [u8; 3] = [2, if cfg!(target_endian = "little") { 1 } else { 2 }, 1];
const MACHINE: Option<u16> = if cfg!(target_arch = "x86_64") {
    Some(62)
} else if cfg!(target_arch = "aarch64") {
    Some(183)
} else {
    None
};

/// The bounds a launcher's headers are read within: program headers,
/// dynamic entries, and the bytes of one string, `PATH_MAX`.
const HEADERS: u16 = 64;
const DYNAMIC: u64 = 512;
const STRING: usize = 4096;

/// The program header types the ruling's list reads: a loaded segment, the
/// dynamic segment, the interpreter's name and the program headers' own.
const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_INTERP: u32 = 3;
const PT_PHDR: u32 = 6;

/// The dynamic tags the ruling's list reads: the end, a needed name, the
/// string table and its size, the two search paths, and the four that hand
/// the loader another library to run, each refused (`DT_DEPAUDIT`,
/// `DT_AUDIT`, `DT_AUXILIARY` and `DT_FILTER`).
const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_STRSZ: u64 = 10;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;
const DELEGATING: [u64; 4] = [0x6fff_fefb, 0x6fff_fefc, 0x7fff_fffd, 0x7fff_ffff];

/// What a launcher's headers name for the loader: its program interpreter,
/// each `DT_RUNPATH` and `DT_RPATH` string, and each `DT_NEEDED` name.
#[derive(Debug, Default, PartialEq, Eq)]
pub(in crate::hands) struct Loaded {
    pub(in crate::hands) interpreter: Vec<u8>,
    pub(in crate::hands) search: Vec<Vec<u8>>,
    pub(in crate::hands) needed: Vec<Vec<u8>>,
}

/// Whether the launcher `fd` holds, lying in `dir`, admits: its headers
/// read ([`headers`]), then every path the loader would read for it
/// [`protected`] (the ruling's items 5 and 8): its interpreter as named,
/// absolute; each search path entry, and each needed name holding a slash
/// or a `$`, with a leading `$ORIGIN` expanded against `dir` and no other
/// `$`; and each of `host.loader` where it exists.
pub(in crate::hands) fn elf(fd: BorrowedFd<'_>, dir: &Path, host: &Host<'_>) -> Result<(), Unfit> {
    let reader = readable(fd).ok().flatten().ok_or(Unfit::Unread)?;
    let loaded = headers(reader.as_fd())?;
    let absolute = loaded.interpreter.starts_with(b"/");
    absolute
        .then_some(())
        .ok_or(Unfit::Relative(Input::Interpreter))?;
    let path = PathBuf::from(OsString::from_vec(loaded.interpreter));
    protected(Input::Interpreter, &path, false, host)?;
    for entry in loaded
        .search
        .iter()
        .flat_map(|list| list.split(|byte| *byte == b':'))
    {
        let path = expanded(entry, dir, Input::SearchPath)?;
        protected(Input::SearchPath, &path, true, host)?;
    }
    let named = |name: &&Vec<u8>| name.contains(&b'/') | name.contains(&b'$');
    for name in loaded.needed.iter().filter(named) {
        let path = expanded(name, dir, Input::Needed)?;
        protected(Input::Needed, &path, false, host)?;
    }
    for (input, path) in [Input::Preload, Input::Cache].into_iter().zip(host.loader) {
        match resolve(path, host) {
            Ok(None) => {}
            Ok(Some(_)) => protected(input, path, false, host)?,
            Err(_) => return Err(Unfit::Unresolved(input)),
        }
    }
    Ok(())
}

/// `entry` as the loader takes it: a leading `$ORIGIN` or `${ORIGIN}`,
/// then nothing or a slash, replaced by `origin`; then an absolute path
/// holding no other `$`, so `$LIB` and `$PLATFORM` are never expanded.
/// Relative, empty or holding another token otherwise.
fn expanded(entry: &[u8], origin: &Path, input: Input) -> Result<PathBuf, Unfit> {
    let token = [&b"$ORIGIN"[..], b"${ORIGIN}"].into_iter();
    let mut rest = token.filter_map(|token| entry.strip_prefix(token));
    let rest = rest.find(|rest| rest.is_empty() | rest.starts_with(b"/"));
    let path = match rest {
        Some(rest) => [origin.as_os_str().as_bytes(), rest].concat(),
        None => entry.to_vec(),
    };
    let named = path.starts_with(b"/") & !path.contains(&b'$');
    let path = PathBuf::from(OsString::from_vec(path));
    named.then_some(path).ok_or(Unfit::Relative(input))
}

/// Whether a launcher whose handle reports `owned` at the canonical `place`
/// is read further, decided apart from observing it: a regular file in the
/// server box's system set ([`system`]), or `granted` a place there,
/// writable by none of `writers` ([`excluded`]).
pub(in crate::hands) fn fit(
    owned: &Owned,
    place: &Path,
    writers: &Credentials,
    granted: bool,
) -> bool {
    let file = FileType::from_raw_mode(owned.mode) == FileType::RegularFile;
    file & (system(place) | granted) & excluded(owned, writers)
}

/// Whether the canonical `place` lies in the server box's system set, each
/// root as spelled and as it resolves.
fn system(place: &Path) -> bool {
    let mut roots = Profile::Server.system().map(Path::new).flat_map(spellings);
    roots.any(|root| place.starts_with(root))
}

/// Whether the object at `path` is in the protected system set: resolved
/// through descriptors to a canonical place inside the server box's system
/// set ([`system`]), a directory where `dir` and a regular file otherwise,
/// and writable by no managed writer as [`protects`] judges its facts.
fn protected(input: Input, path: &Path, dir: bool, host: &Host<'_>) -> Result<(), Unfit> {
    let (resolved, fd, facts) = opened(path, host).map_err(|_| Unfit::Unresolved(input))?;
    let kind = matches!((facts.kind(), dir), (Kind::Dir, true) | (Kind::File, false));
    let object = owned(&facts, acl(fd.as_fd()));
    let hops = route(&resolved.chain).iter().map(|hop| {
        let mode = hop.link.as_ref().map_or(hop.owned.mode, |_| 0);
        Owned { mode, ..hop.owned }
    });
    let writers = &host.credentials;
    let ok = system(&resolved.place) & kind & protects(top(), &object, hops, writers);
    ok.then_some(()).ok_or(Unfit::Unprotected(input))
}

/// MB3's write-exclusion proof ([`excluded`]) over a loader input's facts,
/// decided apart from reading them: `top`, the `/` every resolution starts
/// from and records no hop for, read; it, the `object` itself and every
/// one of `hops`, its route (a link with its mode cleared, judged by its
/// owner alone as its mode means nothing), writable by none of `writers`.
pub(in crate::hands) fn protects(
    top: Option<Owned>,
    object: &Owned,
    hops: impl IntoIterator<Item = Owned>,
    writers: &Credentials,
) -> bool {
    let top = top.is_some_and(|top| excluded(&top, writers));
    let mut hops = hops.into_iter();
    top & excluded(object, writers) & hops.all(|hop| excluded(&hop, writers))
}

/// One program header as the loader reads it: its type, where its bytes
/// lie in the file, the address they load at, how many of them are
/// file-backed and how many loaded, and its alignment.
#[derive(Debug, Clone, Copy)]
struct Segment {
    kind: u32,
    offset: u64,
    address: u64,
    file: u64,
    memory: u64,
    align: u64,
}

impl Segment {
    fn of(bytes: &[u8]) -> Segment {
        Segment {
            kind: word(bytes, 0),
            offset: long(bytes, 8),
            address: long(bytes, 16),
            file: long(bytes, 32),
            memory: long(bytes, 40),
            align: long(bytes, 48),
        }
    }

    /// Where in the file the `size` bytes this loaded segment loads at
    /// `address` lie, by its own translation, where its file-backed bytes
    /// hold them all; none otherwise.
    fn holds(&self, address: u64, size: u64) -> Option<u64> {
        let inside = address.checked_sub(self.address)?;
        let end = inside.checked_add(size)?;
        (end <= self.file).then(|| self.offset.checked_add(inside))?
    }
}

/// What the ELF headers of the file `fd` reads name for the loader, read
/// by the operator's closed list of 2026-10-09 and refused by anything
/// else: the identity ([`header`], item 1, its [`HEADERS`] bound
/// included); the program-header table wholly within the file ([`within`],
/// 2); exactly
/// one `PT_PHDR`, before every `PT_LOAD`, that is the table and is mapped
/// by one load's own translation (3); the loads whole and [`ordered`] (4);
/// exactly one interpreter before every load ([`interpreter`], 5) and one
/// dynamic segment ([`dynamic`], 6 to 8), a launcher with either missing
/// static (9). Other segment types are not read. "At least one `PT_LOAD`"
/// (4) is refused here before the table's own mapping, which needs one.
pub(in crate::hands) fn headers(fd: BorrowedFd<'_>) -> Result<Loaded, Unfit> {
    let header = header(fd)?;
    let held = |held: bool| held.then_some(()).ok_or(Unfit::Malformed);
    held(half(&header, 54) == 56)?;
    let count = half(&header, 56);
    (count <= HEADERS).then_some(()).ok_or(Unfit::Over)?;
    held(count > 0)?;
    let stat = rustix::fs::fstat(fd).map_err(|_| Unfit::Unread)?;
    let length = u64::try_from(stat.st_size).map_err(|_| Unfit::Unread)?;
    let (at, span) = (long(&header, 32), u64::from(count) * 56);
    within(at, span, length)?;
    let table = read(fd, at, usize::from(count) * 56)?;
    let segments = table.as_chunks::<56>().0.iter().map(|at| Segment::of(at));
    let segments: Vec<Segment> = segments.collect();
    let loads = loaded(&segments, length)?;
    let first = segments.iter().position(|segment| segment.kind == PT_LOAD);
    let first = first.ok_or(Unfit::Malformed)?;
    let (place, own) = lone(&segments, PT_PHDR)?.ok_or(Unfit::Malformed)?;
    held((place < first) & (own.offset == at) & (own.file == span) & mapped(&own, &loads))?;
    let (interpreter, dynamic) = (lone(&segments, PT_INTERP)?, lone(&segments, PT_DYNAMIC)?);
    let (Some((place, interpreter)), Some((_, dynamic))) = (interpreter, dynamic) else {
        return Err(Unfit::Static);
    };
    held(place < first)?;
    let interpreter = self::interpreter(fd, &interpreter, length)?;
    let named = self::dynamic(fd, &dynamic, &loads)?;
    Ok(Loaded {
        interpreter,
        ..named
    })
}

/// The one segment of `kind` `segments` hold, with its place among them;
/// none where they hold none, and malformed where they hold two.
fn lone(segments: &[Segment], kind: u32) -> Result<Option<(usize, Segment)>, Unfit> {
    let held = segments.iter().copied().enumerate();
    let mut held = held.filter(|(_, segment)| segment.kind == kind);
    let first = held.next();
    held.next()
        .is_none()
        .then_some(first)
        .ok_or(Unfit::Malformed)
}

/// Whether `segment` lies wholly inside the file-backed bytes of one of
/// `loads`, at the very place that load maps its address from.
fn mapped(segment: &Segment, loads: &[Segment]) -> bool {
    let held = |load: &Segment| load.holds(segment.address, segment.file) == Some(segment.offset);
    loads.iter().any(held)
}

/// Whether `size` bytes from `offset` lie wholly within a file of `length`
/// bytes: truncated where they run past its end, malformed where their end
/// passes the offsets.
fn within(offset: u64, size: u64, length: u64) -> Result<(), Unfit> {
    let end = offset.checked_add(size).ok_or(Unfit::Malformed)?;
    (end <= length).then_some(()).ok_or(Unfit::Truncated)
}

/// The loaded segments of `segments` (item 4), each wholly within a file
/// of `length` bytes ([`within`]) and all [`ordered`] on this host's pages.
fn loaded(segments: &[Segment], length: u64) -> Result<Vec<Segment>, Unfit> {
    let loads = segments.iter().filter(|segment| segment.kind == PT_LOAD);
    let loads: Vec<Segment> = loads.copied().collect();
    for load in &loads {
        within(load.offset, load.file, length)?;
    }
    let page = page().ok_or(Unfit::Unread)?;
    ordered(&loads, page)
        .then_some(loads)
        .ok_or(Unfit::Malformed)
}

/// This host's page size, the unit the loader maps segments in, from the
/// kernel's auxiliary vector (`AT_PAGESZ`); none where it does not read.
fn page() -> Option<u64> {
    let vector = std::fs::read("/proc/self/auxv").ok()?;
    let mut entries = vector.as_chunks::<16>().0.iter();
    let size = entries.find(|entry| long(&entry[..], 0) == AT_PAGESZ);
    size.map(|entry| long(&entry[..], 8))
        .filter(|size| *size > 0)
}

/// The auxiliary vector's entry for the page size.
const AT_PAGESZ: u64 = 6;

/// Whether the loaded segments `loads`, each already within the file, are
/// whole and in order, so the bytes every one maps are its own: none
/// file-backed past its loaded size or ending past the address space; each
/// with `p_offset ≡ p_vaddr` modulo its alignment where that is above 1;
/// in ascending address order, each beginning on a later `page` than the
/// last its predecessor loads, as the loader maps whole pages and no
/// mapping then replaces a page another holds; and no two file ranges
/// sharing a byte.
fn ordered(loads: &[Segment], page: u64) -> bool {
    let span = |load: &Segment| {
        let end = load.address.checked_add(load.memory)?;
        let align = load.align;
        let aligned = (align < 2) || (load.offset % align == load.address % align);
        let whole = aligned & (load.file <= load.memory);
        whole.then(|| (load.address / page, end.div_ceil(page)))
    };
    let spans: Option<Vec<(u64, u64)>> = loads.iter().map(span).collect();
    let apart = |one: &Segment, other: &Segment| {
        let (start, end) = (one.offset.max(other.offset), one.offset + one.file);
        start >= end.min(other.offset + other.file)
    };
    let mut pairs = loads.iter().enumerate();
    let files = pairs.all(|(at, one)| loads[at + 1..].iter().all(|other| apart(one, other)));
    files & spans.is_some_and(|spans| spans.windows(2).all(|pair| pair[0].1 <= pair[1].0))
}

/// The program interpreter's name the interpreter `segment` holds (item
/// 5): its bytes within [`STRING`] and wholly within a file of `length`
/// bytes ([`within`]), one name ended by its one NUL, and not empty.
fn interpreter(fd: BorrowedFd<'_>, segment: &Segment, length: u64) -> Result<Vec<u8>, Unfit> {
    let size = usize::try_from(segment.file).map_err(|_| Unfit::Over)?;
    (size <= STRING).then_some(()).ok_or(Unfit::Over)?;
    within(segment.offset, segment.file, length)?;
    let text = read(fd, segment.offset, size)?;
    let named = text
        .split_last()
        .filter(|(end, name)| (**end == 0) & !name.contains(&0));
    let (_, name) = named
        .filter(|(_, name)| !name.is_empty())
        .ok_or(Unfit::Malformed)?;
    Ok(name.to_vec())
}

/// The ELF header of the file `fd` reads, its identification checked (item
/// 1): a script refuses as one, anything else without ELF's magic as no
/// ELF, a short header as truncated, and another class, encoding,
/// `EI_VERSION`, type than `ET_EXEC` or `ET_DYN`, or machine as foreign.
fn header(fd: BorrowedFd<'_>) -> Result<[u8; 64], Unfit> {
    let mut prefix = [0; 64];
    let mut at = 0;
    while at < prefix.len() {
        let read = rustix::io::pread(fd, &mut prefix[at..], at as u64);
        match read.map_err(|_| Unfit::Unread)? {
            0 => break,
            read => at += read,
        }
    }
    let prefix = &prefix[..at];
    (!prefix.starts_with(b"#!"))
        .then_some(())
        .ok_or(Unfit::Script)?;
    prefix
        .starts_with(b"\x7fELF")
        .then_some(())
        .ok_or(Unfit::NotElf)?;
    let header: [u8; 64] = prefix.try_into().map_err(|_| Unfit::Truncated)?;
    let kind = matches!(half(&header, 16), 2 | 3);
    let machine = MACHINE.is_some_and(|machine| half(&header, 18) == machine);
    let ident = header[4..7] == IDENT[..];
    (ident & kind & machine)
        .then_some(header)
        .ok_or(Unfit::Foreign)
}

/// The search path strings and needed names the dynamic `segment` names,
/// no interpreter among them (item 6): a whole number of entries,
/// [`DYNAMIC`] at most, lying wholly inside one of the `loads`'
/// file-backed bytes by its own translation
/// ([`mapped`]) and holding a `DT_NULL`, none after the first read. Its
/// string table is [`tabled`] (item 7) and each string read from it
/// ([`string`], item 8).
fn dynamic(fd: BorrowedFd<'_>, segment: &Segment, loads: &[Segment]) -> Result<Loaded, Unfit> {
    let size = segment.file;
    size.is_multiple_of(16)
        .then_some(())
        .ok_or(Unfit::Malformed)?;
    (size / 16 <= DYNAMIC).then_some(()).ok_or(Unfit::Over)?;
    mapped(segment, loads)
        .then_some(())
        .ok_or(Unfit::Malformed)?;
    let size = usize::try_from(size).map_err(|_| Unfit::Over)?;
    let entries = read(fd, segment.offset, size)?;
    let entries: Vec<(u64, u64)> = entries
        .as_chunks::<16>()
        .0
        .iter()
        .map(|entry| (long(entry, 0), long(entry, 8)))
        .collect();
    let end = entries.iter().position(|(tag, _)| *tag == DT_NULL);
    let entries = &entries[..end.ok_or(Unfit::Malformed)?];
    let (base, table) = tabled(entries, loads)?;
    let mut loaded = Loaded::default();
    let named = |(tag, _): &&(u64, u64)| [DT_NEEDED, DT_RPATH, DT_RUNPATH].contains(tag);
    for (tag, at) in entries.iter().filter(named) {
        let text = string(fd, base, table, *at)?;
        match *tag == DT_NEEDED {
            true => loaded.needed.push(text),
            false => loaded.search.push(text),
        }
    }
    Ok(loaded)
}

/// Where the string table the dynamic `entries` before `DT_NULL` name
/// lies in the file, and its size (item 7), whatever entries name it: no
/// entry [`DELEGATING`] the loader to another library (audited); exactly
/// one `DT_STRTAB` and one `DT_STRSZ`, a second refused even when equal,
/// and at most one `DT_RUNPATH` and one `DT_RPATH`; the table's whole
/// extent inside one of the `loads`' file-backed bytes, translated by it.
fn tabled(entries: &[(u64, u64)], loads: &[Segment]) -> Result<(u64, u64), Unfit> {
    let delegating = entries.iter().any(|(tag, _)| DELEGATING.contains(tag));
    (!delegating).then_some(()).ok_or(Unfit::Audited)?;
    let held = |wanted: u64| entries.iter().filter(move |(tag, _)| *tag == wanted);
    let once = [DT_RUNPATH, DT_RPATH].map(|tag| held(tag).count() < 2);
    (once == [true; 2]).then_some(()).ok_or(Unfit::Malformed)?;
    let only = |wanted: u64| {
        let mut held = held(wanted);
        match (held.next(), held.next()) {
            (Some((_, value)), None) => Ok(*value),
            _ => Err(Unfit::Malformed),
        }
    };
    let (address, size) = (only(DT_STRTAB)?, only(DT_STRSZ)?);
    let base = loads.iter().find_map(|load| load.holds(address, size));
    Ok((base.ok_or(Unfit::Malformed)?, size))
}

/// The string at `at` in the string table of `size` bytes at `base` (item
/// 8): the bytes before its NUL, which must come within [`STRING`] bytes
/// and the table, so one at or past the table's end has none.
fn string(fd: BorrowedFd<'_>, base: u64, size: u64, at: u64) -> Result<Vec<u8>, Unfit> {
    let left = size.saturating_sub(at);
    let room = usize::try_from(left).unwrap_or(usize::MAX).min(STRING + 1);
    let start = base.checked_add(at).ok_or(Unfit::Malformed)?;
    let bytes = read(fd, start, room)?;
    match bytes.iter().position(|byte| *byte == 0) {
        Some(end) => Ok(bytes[..end].to_vec()),
        None if room > STRING => Err(Unfit::Over),
        None => Err(Unfit::Malformed),
    }
}

/// `size` bytes of the file `fd` reads from `offset`, read in place,
/// never mapped: truncated where the file ends first.
fn read(fd: BorrowedFd<'_>, offset: u64, size: usize) -> Result<Vec<u8>, Unfit> {
    let mut bytes = vec![0; size];
    let mut at = 0;
    while at < size {
        let from = offset.checked_add(at as u64).ok_or(Unfit::Malformed)?;
        let read = rustix::io::pread(fd, &mut bytes[at..], from).map_err(|_| Unfit::Unread)?;
        at += (read > 0).then_some(read).ok_or(Unfit::Truncated)?;
    }
    Ok(bytes)
}

/// A field of `N` bytes at `at` of an ELF structure in this host's
/// encoding, which the header's identification matched.
fn field<const N: usize>(bytes: &[u8], at: usize) -> [u8; N] {
    let mut field = [0; N];
    field.copy_from_slice(&bytes[at..at + N]);
    field
}

fn half(bytes: &[u8], at: usize) -> u16 {
    u16::from_ne_bytes(field(bytes, at))
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(field(bytes, at))
}

fn long(bytes: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(field(bytes, at))
}
