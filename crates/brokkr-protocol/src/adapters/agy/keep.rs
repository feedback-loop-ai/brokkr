//! Keeping an agy turn's `transcript.jsonl` past the engine HOME's removal
//! (#1227, decision 0075 level 2).
//!
//! The source lives inside the engine's temporary HOME, where an unboxed
//! seat's own commands could write. So every step fails closed:
//! - the conversation id must be an exact 36-byte lowercase UUID
//!   (`8-4-4-4-12`) before it ever names a path component;
//! - every component from the engine HOME down to `transcript.jsonl` is
//!   opened one step at a time with `O_NOFOLLOW`, and the leaf must be a
//!   regular file (opened with `O_NONBLOCK` so a planted FIFO cannot hang);
//! - the kept-conversations parent under `<operator HOME>/.local/state/brokkr/agy-conversations`
//!   is verified owner-only (directory owned by effective UID, no group or
//!   other bits, never a symlink) before anything is written;
//! - ancestors above `agy-conversations` (`~/.local/state/brokkr` and up)
//!   are traversed and, if missing, created with `create_dir_all`, asking
//!   nothing of an existing ancestor that the operating system does not ask to
//!   traverse it and create an entry in it (#1227 K3);
//! - no mode is changed by pathname; `agy-conversations` and each
//!   per-conversation directory that the keeper creates is created with
//!   `mkdirat` at mode 0700, opened with `O_NOFOLLOW`, verified by
//!   `(st_dev, st_ino)` against the identity recorded right after `mkdirat`,
//!   and tightened to mode 0700 through that verified handle with `fchmod`
//!   (existing directories' modes are never changed); when the umask makes a
//!   directory the keeper has just created impossible for its owner to open,
//!   no transcript is kept and removal of that directory through
//!   `unlink_if_same` is attempted (an unlink I/O error can leave the staged
//!   directory even as persistence is refused), and a replaced entry is left
//!   untouched with nothing
//!   kept (#1227 K2, Q2); the kept file is mode 0600 through `fchmod`,
//!   regardless of the 0644 mode agy writes;
//! - the copy is bounded at [`TRANSCRIPT_SOURCE_CAP`] (32 MiB, the one cap
//!   shared with `brokkr-view`); anything larger is not kept and any
//!   partial directory is removed;
//! - no seat-chosen id, path or content reaches a notice.
//!
//! **Kept-file rule (#1227 U4, W3, W4, Y3, Y4, Y9):** the driver and the
//! engine are two processes, so the driver can only keep nothing in the cases
//! it can see. Those are: a probe; a refusal to start; no `init` with a
//! conversation; an id that is not a UUID; a source that is not a regular file
//! reached without a symbolic link; a file over the bound; a reference the
//! reader's rule rejects (Y3); a row the turn's masking would change (Y4); and
//! an undeclared kind. What the driver cannot see is the engine refusing the
//! row after the driver has returned. Then a kept file with no record can
//! remain; it is readable only by the operator, and the reaper of issue #1323
//! removes kept files that no journal names.
//!
//! **Same-user directory limit (#1227 L1, operator rulings 2026-11-25 and
//! 2026-11-26):** the limit is the class of processes running as the operator's
//! own user that create, rename, move, replace, rewrite or remove entries under
//! the engine's HOME for the turn or under the operator's `agy-conversations`
//! directory while a keep is in progress. The keeper runs as that same user,
//! the engine HOME is a mode 0700 directory the driver made owned by the
//! operator, the turn's `agy` process has exited when keeping begins, and a
//! boxed seat's tools ran in the box and cannot reach it. No privilege boundary
//! is crossed and nothing of another user's is reachable.
//!
//! Against that class, the keeper promises no outcome — on the kept side, for
//! example, where kept state lands if the directory is moved, which entry is
//! removed if one is swapped between `statat` and `unlinkat`, or what the
//! record's home names if a parent is renamed; on the source side, a source
//! directory detached by a replaced parent, a database or subtree changed after
//! its own final check before publication, or a same-length in-place rewrite
//! that restores the recorded modification time. Promises are against
//! accidents (a turn still writing when killed, a partial tree, a link, a
//! non-regular entry, a tree over its bounds, a bound value in a file or name,
//! or an unconfirmed model pin). What
//! the code holds are four mechanisms, each checkable by reading it and held
//! by tests:
//! (m1) every open the keeper makes for the kept place is an `openat` relative
//! to a handle it holds, with `O_NOFOLLOW` on the final component, so it never
//! traverses a symbolic link placed at a name it opens;
//! (m2) no system call in the keeper changes a mode by pathname; a mode
//! changes only through `fchmod` on a handle;
//! (m3) the transcript file is created with `O_CREAT | O_EXCL | O_NOFOLLOW`,
//! relative to the conversation directory's handle, so an existing entry at
//! that name is never opened or truncated;
//! (m4) every `unlinkat` in the keeper is preceded by a `statat` through the
//! same parent handle whose `(st_dev, st_ino)` equals the identity the keeper
//! recorded, and is skipped when they differ.
//!
//! When no such same-user process acts on the tree during a keep — every
//! ordinary run — all guarantees above (the kept place and its modes, the
//! refusals, refusing publication on each listed failure, and attempting to
//! remove staged entries on rollback; an unlink I/O failure can leave a partial
//! directory even as persistence is refused) hold.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};

use brokkr_core::seat::{
    is_agy_uuid, is_native_transcript_home, is_valid_transcript_home, AGY_TRANSCRIPT_FILE,
    TRANSCRIPT_SOURCE_CAP,
};
use rustix::fs::{
    fchmod, fstat, mkdirat, openat, statat, unlinkat, AtFlags, FileType, Mode, OFlags, RawMode,
    Stat, CWD,
};
use rustix::io::Errno;
use thiserror::Error;

use crate::transcript::AGY_CONVERSATIONS;

/// Chunk size for bounded streaming copies.
const COPY_BUF_BYTES: usize = 65_536;

/// One kept agy conversation transcript, holding the exact [`crate::transcript::Transcript`]
/// address already verified by mask preflight so publication serializes the
/// same constructor and bytes (#1227 Y4).
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Kept {
    transcript: crate::transcript::Transcript,
}

impl Kept {
    pub(super) fn transcript_value(&self) -> serde_json::Value {
        self.transcript.value()
    }
}

#[cfg(test)]
impl Kept {
    pub(super) fn home(&self) -> &Path {
        self.transcript.home()
    }

    pub(super) fn locator(&self) -> &str {
        self.transcript.locator()
    }
}

/// Shared failures while validating or acquiring the operator's
/// `agy-conversations` parent directory, used by both transcript and rejoin
/// keeping (#1227, decision 0071 ruling 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(super) enum ParentError {
    #[error("the operator has no HOME to keep it under")]
    NoOperatorHome,
    #[error("the reference does not pass the run view's reference rule")]
    InvalidReference,
    #[error("the kept-conversations directory path is not UTF-8")]
    NonUtf8Home,
    #[error("the kept-conversations parent is not an owner-only directory of this user's")]
    UnownedParent,
    #[error("could not prepare the kept-conversations parent: {}", io::Error::from(*.0))]
    ParentIo(io::ErrorKind),
    #[error("a newly created kept directory was replaced before it could be opened")]
    ReplacedDirectory,
}

/// Shared classification of source open/stat errnos under the engine HOME
/// (decision 0071 ruling 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(super) enum SourceErrno {
    #[error("a required conversation file or directory is absent")]
    Absent,
    #[error(
        "a path component under the engine HOME is a symbolic link or not of the expected file type"
    )]
    Unsafe,
    #[error("could not read source: {}", io::Error::from(*.0))]
    Io(io::ErrorKind),
}

pub(super) fn classify_source_dir_errno(errno: Errno) -> SourceErrno {
    match errno {
        Errno::NOENT => SourceErrno::Absent,
        Errno::LOOP | Errno::NOTDIR => SourceErrno::Unsafe,
        other => SourceErrno::Io(io::Error::from(other).kind()),
    }
}

pub(super) fn classify_source_file_errno(errno: Errno) -> SourceErrno {
    match errno {
        Errno::NOENT => SourceErrno::Absent,
        Errno::LOOP | Errno::ISDIR | Errno::NOTDIR => SourceErrno::Unsafe,
        other => SourceErrno::Io(io::Error::from(other).kind()),
    }
}

/// Why an agy turn's transcript was not kept. Reported as a notice while
/// leaving the turn's own outcome unchanged, and never echoing any id,
/// path or content the seat chose (#1227).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum KeepNotice {
    #[error("agy transcript not kept: turn refused to start and recorded no transcript")]
    RefusedToStart,
    #[error("agy transcript not kept: init's conversation id is not a lowercase UUID")]
    InvalidId,
    #[error("agy transcript not kept: {0}")]
    Parent(#[from] ParentError),
    #[error(
        "agy transcript not kept: masking the turn's secret bindings would rewrite the transcript \
         row"
    )]
    ReferenceMasked,
    #[error("agy transcript not kept: transcript file is absent")]
    Absent,
    #[error(
        "agy transcript not kept: a path component under the engine HOME is a symbolic link or \
         not of the expected file type"
    )]
    UnsafeSource,
    #[error("agy transcript not kept: transcript exceeds the {0}-byte copy limit")]
    TooLarge(u64),
    #[error("agy transcript not kept: a kept directory for this conversation already exists")]
    AlreadyKept,
    #[error("agy transcript not kept: could not copy the transcript: {}", io::Error::from(*.0))]
    Io(io::ErrorKind),
}

/// Keep `conversation_id`'s `transcript.jsonl` from `engine_home` under
/// `operator_home`'s kept-conversations parent, bounded at
/// [`TRANSCRIPT_SOURCE_CAP`], refusing to publish if the run view's reference
/// rule rejects `(locator, home)` (#1227 Y3) or if passing the emitted
/// transcript row through [`crate::adapters::probe::seat_row`] with `bindings`
/// changes any byte (#1227 Y4).
pub(super) fn keep(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[crate::secret::BoundSecret],
) -> Result<Kept, KeepNotice> {
    keep_with_bindings_and_cap(
        conversation_id,
        engine_home,
        operator_home,
        bindings,
        TRANSCRIPT_SOURCE_CAP,
        false,
    )
}

/// [`keep`] with an explicit byte limit and no bindings, so unit tests
/// exercise the copy bound and path gates without writing 32 MiB.
#[cfg(test)]
pub(super) fn keep_with_cap(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    cap: u64,
) -> Result<Kept, KeepNotice> {
    keep_with_bindings_and_cap(conversation_id, engine_home, operator_home, &[], cap, false)
}

fn keep_with_bindings_and_cap(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[crate::secret::BoundSecret],
    cap: u64,
    replace: bool,
) -> Result<Kept, KeepNotice> {
    // Every UUID admitted by `is_agy_uuid` is lowercase ASCII hex and `-`, so
    // it also satisfies `brokkr_core::seat::is_clean_transcript_path`
    // (tested by `brokkr_core::seat::tests::transcript_source_cap_and_agy_uuid_rule_bind`).
    if !is_agy_uuid(conversation_id) {
        return Err(KeepNotice::InvalidId);
    }
    let parent_path = resolve_parent_path(operator_home)?;
    let transcript = crate::transcript::Transcript::agy(parent_path, conversation_id);
    if row_would_mask(&transcript.emitted(), bindings) {
        return Err(KeepNotice::ReferenceMasked);
    }
    let src_fd = open_source(engine_home, conversation_id, cap)?;
    let parent_fd = owned_parent_raw(transcript.home())?;
    if replace {
        super::restore::stage_replace_transcript(&parent_fd, conversation_id, src_fd, cap)?;
    } else {
        let mut guard = ConvGuard::create(&parent_fd, conversation_id)?;
        let dest_fd = guard.open_dest()?;
        copy_bounded(src_fd, dest_fd, cap)?;
        guard.defuse();
    }
    Ok(Kept { transcript })
}

/// Whether passing an exact emitted checkpoint row through the shared path's
/// [`crate::adapters::probe::seat_row`] with `bindings` differs from the
/// unmasked row in any byte (#1227 Y4).
pub(super) fn row_would_mask(
    emitted: &crate::adapters::Emitted,
    bindings: &[crate::secret::BoundSecret],
) -> bool {
    if bindings.is_empty() {
        return false;
    }
    let unmasked =
        crate::adapters::probe::seat_row(crate::adapters::AdapterKind::Agy, &[], &emitted.row);
    let masked =
        crate::adapters::probe::seat_row(crate::adapters::AdapterKind::Agy, bindings, &emitted.row);
    masked != unmasked
}

/// Validate `operator_home` (present, UTF-8, and admitted by both the shared
/// run-view lexical home rule [`is_valid_transcript_home`] and the native
/// discovery root rule [`is_native_transcript_home`]) and return
/// `<operator_home>/.local/state/brokkr/agy-conversations` without touching
/// disk (#1227 Y3).
pub(super) fn resolve_parent_path(operator_home: Option<&Path>) -> Result<PathBuf, ParentError> {
    let operator = operator_home
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or(ParentError::NoOperatorHome)?;
    let parent = operator.join(AGY_CONVERSATIONS.iter().collect::<PathBuf>());
    let parent_str = parent.to_str().ok_or(ParentError::NonUtf8Home)?;
    if !is_valid_transcript_home(parent_str) || !is_native_transcript_home(parent_str) {
        return Err(ParentError::InvalidReference);
    }
    Ok(parent)
}

pub(super) fn dir_flags() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

pub(super) fn source_file_flags() -> OFlags {
    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC
}

pub(super) fn dest_file_flags() -> OFlags {
    OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

pub(super) fn io_errno(errno: Errno) -> KeepNotice {
    KeepNotice::Io(io::Error::from(errno).kind())
}

fn io_err(err: io::Error) -> KeepNotice {
    KeepNotice::Io(err.kind())
}

pub(super) fn parent_err_errno(errno: Errno) -> ParentError {
    ParentError::ParentIo(io::Error::from(errno).kind())
}

pub(super) fn parent_leaf_errno(errno: Errno) -> ParentError {
    match errno {
        Errno::LOOP | Errno::NOTDIR => ParentError::UnownedParent,
        other => parent_err_errno(other),
    }
}

impl From<SourceErrno> for KeepNotice {
    fn from(err: SourceErrno) -> Self {
        match err {
            SourceErrno::Absent => Self::Absent,
            SourceErrno::Unsafe => Self::UnsafeSource,
            SourceErrno::Io(kind) => Self::Io(kind),
        }
    }
}

pub(super) fn classify_dir_errno(errno: Errno) -> KeepNotice {
    classify_source_dir_errno(errno).into()
}

pub(super) fn classify_file_errno(errno: Errno) -> KeepNotice {
    classify_source_file_errno(errno).into()
}

pub(super) fn is_owned_private_dir(st_mode: RawMode, st_uid: u32, uid: u32) -> bool {
    FileType::from_raw_mode(st_mode) == FileType::Directory
        && st_uid == uid
        && (Mode::from_raw_mode(st_mode).bits() & 0o077) == 0
}

/// Open `<engine_home>/.gemini/antigravity-cli/brain/<id>/.system_generated/logs/transcript.jsonl`
/// without following any symlink from `engine_home` down, checking that the
/// leaf is a regular file and rejecting an apparent size above `cap`.
fn open_source(engine_home: &Path, conversation_id: &str, cap: u64) -> Result<OwnedFd, KeepNotice> {
    let mut dir =
        openat(CWD, engine_home, dir_flags(), Mode::empty()).map_err(classify_dir_errno)?;
    let steps = [
        ".gemini",
        "antigravity-cli",
        "brain",
        conversation_id,
        ".system_generated",
        "logs",
    ];
    for step in steps {
        dir = openat(&dir, step, dir_flags(), Mode::empty()).map_err(classify_dir_errno)?;
    }
    let file = openat(&dir, "transcript.jsonl", source_file_flags(), Mode::empty())
        .map_err(classify_file_errno)?;
    let stat = fstat(&file).map_err(io_errno)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(KeepNotice::UnsafeSource);
    }
    if u64::try_from(stat.st_size)
        .ok()
        .is_some_and(|size| size > cap)
    {
        return Err(KeepNotice::TooLarge(cap));
    }
    Ok(file)
}

pub(super) fn same_entry(one: &Stat, other: &Stat) -> bool {
    (one.st_dev, one.st_ino) == (other.st_dev, other.st_ino)
}

pub(super) fn same_entry_at(dir_fd: BorrowedFd<'_>, name: &OsStr, expected: &Stat) -> bool {
    statat(dir_fd, name, AtFlags::SYMLINK_NOFOLLOW).is_ok_and(|entry| same_entry(expected, &entry))
}

/// Attempt to remove `(dir_fd, name)` when its `(st_dev, st_ino)` still match
/// `expected` (mechanism (m4)). Ignores `unlinkat` I/O errors, so cleanup is
/// best-effort and can leave a partial directory even as persistence is refused.
pub(super) fn unlink_if_same(
    dir_fd: BorrowedFd<'_>,
    name: &OsStr,
    expected: &Stat,
    flags: AtFlags,
) {
    if same_entry_at(dir_fd, name, expected) {
        let _ = unlinkat(dir_fd, name, flags);
    }
}

/// Open a directory this call just created at `(parent_fd, name)`, verify
/// through `fstat` that the opened handle has the same `(st_dev, st_ino)` as
/// `created_stat` (#1227 Q2), and attempt to remove the created entry through
/// `unlink_if_same` if opening fails (an unlink I/O error can leave the staged
/// directory even as persistence is refused, #1227 K2). If the entry was replaced
/// before the open, returns `replaced` (e.g. [`ParentError::ReplacedDirectory`]) without touching
/// the replacement.
pub(super) fn open_created_dir<E: Clone>(
    parent_fd: BorrowedFd<'_>,
    name: &OsStr,
    created_stat: &Stat,
    replaced: E,
    on_open_err: fn(Errno) -> E,
) -> Result<(OwnedFd, Stat), E> {
    let (fd, opened_stat) = openat(parent_fd, name, dir_flags(), Mode::empty())
        .and_then(|fd| fstat(&fd).map(|stat| (fd, stat)))
        .map_err(|errno| {
            unlink_if_same(parent_fd, name, created_stat, AtFlags::REMOVEDIR);
            match errno {
                Errno::LOOP | Errno::NOTDIR => replaced.clone(),
                other => on_open_err(other),
            }
        })?;
    if !same_entry(created_stat, &opened_stat) {
        return Err(replaced);
    }
    Ok((fd, opened_stat))
}

/// Ensure `dir` (`agy-conversations`) exists as a directory and return an
/// opened handle to it.
///
/// Ancestors above `dir` (`~/.local/state/brokkr` and up) are traversed and, if
/// missing, created with [`std::fs::create_dir_all`], asking nothing of an
/// existing ancestor that the operating system does not ask to traverse it and
/// create an entry in it (#1227 K3). `dir` itself is created with
/// `mkdirat(0o700)`, opened with `O_NOFOLLOW`, verified by `same_entry` against
/// the `statat` recorded right after `mkdirat` (#1227 Q2), and set to mode 0700
/// through the verified handle with `fchmod` (#1227 K2). When the umask makes a
/// newly created `dir` impossible for its owner to open, removal through
/// `unlink_if_same` is attempted (an unlink I/O error can leave the staged
/// directory even as persistence is refused). An existing `dir` is
/// opened without changing its mode (W8).
fn ensure_parent_dir(
    dir: &Path,
    before_open: impl FnOnce(),
    after_verify: impl FnOnce(),
) -> Result<OwnedFd, ParentError> {
    let Some(ancestor) = dir.parent() else {
        return openat(CWD, dir, dir_flags(), Mode::empty()).map_err(parent_leaf_errno);
    };
    std::fs::create_dir_all(ancestor).map_err(|err| ParentError::ParentIo(err.kind()))?;
    let mode = Mode::from_raw_mode(0o700);
    match mkdirat(CWD, dir, mode) {
        Ok(()) => {
            let created_stat =
                statat(CWD, dir, AtFlags::SYMLINK_NOFOLLOW).map_err(parent_err_errno)?;
            before_open();
            let (fd, _) = open_created_dir(
                CWD,
                dir.as_os_str(),
                &created_stat,
                ParentError::ReplacedDirectory,
                parent_err_errno,
            )?;
            after_verify();
            fchmod(&fd, mode).map_err(parent_err_errno)?;
            Ok(fd)
        }
        Err(Errno::EXIST) => {
            openat(CWD, dir, dir_flags(), Mode::empty()).map_err(parent_leaf_errno)
        }
        Err(errno) => Err(parent_err_errno(errno)),
    }
}

/// Prepare and open `<operator_home>/.local/state/brokkr/agy-conversations`,
/// traversing and creating missing ancestors above it with `create_dir_all`
/// (#1227 K3), creating `agy-conversations` with `mkdirat(0o700)` and setting
/// mode 0700 only through a verified handle via `fchmod` without changing any
/// existing directory's mode, and verifying through the opened handle that it
/// is a real directory owned by the effective UID with no group or other
/// permission bits (`mode & 0o077 == 0`). A non-writable owner-only parent
/// (for instance mode 0500) passes this check and fails at
/// [`ConvGuard::create`] with [`KeepNotice::Io`], leaving the turn unchanged
/// (#1227 K2, K3, Q2, W8).
#[cfg(test)]
pub(super) fn owned_parent(parent: &Path) -> Result<OwnedFd, KeepNotice> {
    owned_parent_raw(parent).map_err(Into::into)
}

pub(super) fn owned_parent_raw(parent: &Path) -> Result<OwnedFd, ParentError> {
    owned_parent_raw_with(parent, || {}, || {})
}

#[cfg(test)]
pub(super) fn owned_parent_with(
    parent: &Path,
    before_open: impl FnOnce(),
    after_verify: impl FnOnce(),
) -> Result<OwnedFd, KeepNotice> {
    owned_parent_raw_with(parent, before_open, after_verify).map_err(Into::into)
}

fn owned_parent_raw_with(
    parent: &Path,
    before_open: impl FnOnce(),
    after_verify: impl FnOnce(),
) -> Result<OwnedFd, ParentError> {
    let fd = ensure_parent_dir(parent, before_open, after_verify)?;
    let stat = fstat(&fd).map_err(parent_err_errno)?;
    let uid = rustix::process::geteuid().as_raw();
    if !is_owned_private_dir(stat.st_mode, stat.st_uid, uid) {
        return Err(ParentError::UnownedParent);
    }
    Ok(fd)
}

/// RAII guard for a newly created `<parent>/<uuid>` directory, bound to the
/// acquired parent and conversation directory handles and their `(st_dev,
/// st_ino)` identities so rollback checks entry identity through
/// `unlink_if_same` before unlinking (subject to the same-user directory limit
/// in this module's documentation, #1227 L1, S3).
pub(super) struct ConvGuard<'a> {
    parent_fd: Option<&'a OwnedFd>,
    conv_fd: OwnedFd,
    conv_stat: Stat,
    id: &'a str,
    dest_stat: Option<Stat>,
}

impl<'a> ConvGuard<'a> {
    pub(super) fn create(parent_fd: &'a OwnedFd, id: &'a str) -> Result<Self, KeepNotice> {
        Self::create_with(parent_fd, id, |_, _| {}, |_, _| {})
    }

    pub(super) fn create_with(
        parent_fd: &'a OwnedFd,
        id: &'a str,
        before_open: impl FnOnce(&OwnedFd, &str),
        after_verify: impl FnOnce(&OwnedFd, &str),
    ) -> Result<Self, KeepNotice> {
        let mode = Mode::from_raw_mode(0o700);
        mkdirat(parent_fd, id, mode).map_err(|errno| match errno {
            Errno::EXIST => KeepNotice::AlreadyKept,
            other => io_errno(other),
        })?;
        let created_stat = statat(parent_fd, id, AtFlags::SYMLINK_NOFOLLOW).map_err(io_errno)?;
        before_open(parent_fd, id);
        let (conv_fd, conv_stat) = open_created_dir(
            parent_fd.as_fd(),
            OsStr::new(id),
            &created_stat,
            KeepNotice::Parent(ParentError::ReplacedDirectory),
            io_errno,
        )?;
        let guard = Self {
            parent_fd: Some(parent_fd),
            conv_fd,
            conv_stat,
            id,
            dest_stat: None,
        };
        after_verify(parent_fd, id);
        fchmod(&guard.conv_fd, mode).map_err(io_errno)?;
        Ok(guard)
    }

    /// Create `<conv>/transcript.jsonl` inside the acquired conversation
    /// directory with mode 0600 (never following a symlink and never
    /// clobbering an existing file).
    pub(super) fn open_dest(&mut self) -> Result<OwnedFd, KeepNotice> {
        let mode = Mode::from_raw_mode(0o600);
        let file_fd = openat(&self.conv_fd, AGY_TRANSCRIPT_FILE, dest_file_flags(), mode)
            .map_err(io_errno)?;
        self.dest_stat = Some(fstat(&file_fd).map_err(io_errno)?);
        fchmod(&file_fd, mode).map_err(io_errno)?;
        Ok(file_fd)
    }

    fn defuse(&mut self) {
        self.parent_fd = None;
    }
}

impl Drop for ConvGuard<'_> {
    fn drop(&mut self) {
        if let Some(parent_fd) = self.parent_fd.take() {
            if let Some(dest_stat) = &self.dest_stat {
                unlink_if_same(
                    self.conv_fd.as_fd(),
                    OsStr::new(AGY_TRANSCRIPT_FILE),
                    dest_stat,
                    AtFlags::empty(),
                );
            }
            unlink_if_same(
                parent_fd.as_fd(),
                OsStr::new(self.id),
                &self.conv_stat,
                AtFlags::REMOVEDIR,
            );
        }
    }
}

/// Stream-copy at most `remaining` bytes from `src` to `dest`, passing each
/// chunk through `screen` before writing. Shared by transcript and rejoin
/// keeping (decision 0071 ruling 5).
pub(super) fn stream_copy_bounded<E>(
    src: &mut File,
    dest: &mut File,
    remaining: u64,
    mut screen: impl FnMut(&[u8]) -> Result<(), E>,
    on_over: impl FnOnce() -> E,
    on_io: fn(io::Error) -> E,
) -> Result<u64, E> {
    let mut buf = vec![0u8; COPY_BUF_BYTES];
    let mut copied: u64 = 0;
    loop {
        let read = src.read(&mut buf).map_err(on_io)?;
        if read == 0 {
            break;
        }
        copied = copied.saturating_add(read as u64);
        if copied > remaining {
            return Err(on_over());
        }
        screen(&buf[..read])?;
        dest.write_all(&buf[..read]).map_err(on_io)?;
    }
    dest.flush().map_err(on_io)?;
    Ok(copied)
}

/// Copy at most `cap` bytes from `src_fd` to `dest_fd`, failing with
/// [`KeepNotice::TooLarge`] if `src_fd` yields even one byte beyond `cap`.
pub(super) fn copy_bounded(src_fd: OwnedFd, dest_fd: OwnedFd, cap: u64) -> Result<(), KeepNotice> {
    let mut src = File::from(src_fd);
    let mut dest = File::from(dest_fd);
    stream_copy_bounded(
        &mut src,
        &mut dest,
        cap,
        |_| Ok(()),
        || KeepNotice::TooLarge(cap),
        io_err,
    )?;
    Ok(())
}

/// Inputs that decide whether a finished agy turn keeps its `transcript.jsonl`
/// before the engine HOME is removed (#1227 W3, W4, W6, W7).
pub(super) struct KeepContext<'a> {
    pub(super) declared: bool,
    pub(super) probe: bool,
    pub(super) refused_to_start: bool,
    pub(super) conversation: &'a super::fold::Conversation,
    pub(super) engine_home: &'a Path,
    pub(super) operator_home: Option<&'a Path>,
    pub(super) bindings: &'a [crate::secret::BoundSecret],
}

/// Typed outcome of publishing an agy turn's transcript before the engine HOME
/// is removed, carried directly to rejoin publication instead of reading back
/// string-keyed `session_meta` (decision 0071 rulings 3 and 7).
#[derive(Debug, PartialEq, Eq)]
pub(super) enum TranscriptOutcome {
    Kept(Kept),
    NotKept(Option<KeepNotice>),
}

/// Keep the conversation's `transcript.jsonl` before the engine HOME is
/// removed and publish the transcript row (#1227).
pub(super) fn publish_transcript(
    ctx: KeepContext<'_>,
    session_meta: &mut serde_json::Map<String, serde_json::Value>,
    emit: &mut impl FnMut(&crate::adapters::Emitted),
) -> TranscriptOutcome {
    publish_transcript_mode(ctx, false, session_meta, emit)
}

pub(super) fn publish_transcript_mode(
    ctx: KeepContext<'_>,
    replace: bool,
    session_meta: &mut serde_json::Map<String, serde_json::Value>,
    emit: &mut impl FnMut(&crate::adapters::Emitted),
) -> TranscriptOutcome {
    use super::fold::Conversation;
    // Probe turns, undeclared launches, and turns with no init each match main
    // to the byte: keep nothing, emit no notice, record `none` (#1227 W4).
    if !ctx.declared || ctx.probe || matches!(ctx.conversation, Conversation::None) {
        record_none(session_meta, emit);
        return TranscriptOutcome::NotKept(None);
    }
    if ctx.refused_to_start {
        record_none(session_meta, emit);
        return TranscriptOutcome::NotKept(Some(KeepNotice::RefusedToStart));
    }
    let Conversation::Named(id) = ctx.conversation else {
        record_none(session_meta, emit);
        return TranscriptOutcome::NotKept(Some(KeepNotice::InvalidId));
    };
    let outcome = match replace {
        true => keep_with_bindings_and_cap(
            id,
            ctx.engine_home,
            ctx.operator_home,
            ctx.bindings,
            TRANSCRIPT_SOURCE_CAP,
            true,
        ),
        false => keep(id, ctx.engine_home, ctx.operator_home, ctx.bindings),
    };
    match outcome {
        Ok(mut kept) => {
            kept.transcript.finish(session_meta, emit);
            TranscriptOutcome::Kept(kept)
        }
        Err(notice) => {
            record_none(session_meta, emit);
            TranscriptOutcome::NotKept(Some(notice))
        }
    }
}

fn record_none(
    session_meta: &mut serde_json::Map<String, serde_json::Value>,
    emit: &mut impl FnMut(&crate::adapters::Emitted),
) {
    let mut transcript = crate::transcript::Transcript::none();
    transcript.finish(session_meta, emit);
}

/// Report a keep notice without changing the turn's own outcome: join onto the
/// invocation's stderr on `Ok` (including beside `SIGN_IN_WITHHELD` on a
/// signed-out turn, #1227 U3), and write to driver stderr on `Err`.
pub(super) fn report_notice(
    outcome: &mut Result<crate::adapters::Invocation, super::Ended>,
    notice: &impl std::fmt::Display,
) {
    match outcome {
        Ok(invocation) => {
            crate::adapters::stderr::join_note(&mut invocation.stderr, &notice.to_string());
        }
        Err(_) => eprintln!("{notice}"),
    }
}
