//! Keeping an agy work turn's rejoin state (`conversations/<id>.db` and
//! `brain/<id>/`) beside its kept `transcript.jsonl`, and publishing its
//! `root_session` checkpoint row (#1227, decision 0075 level 2).
//!
//! One kept place, one rule: every file and directory kept for rejoin lives
//! under `<operator HOME>/.local/state/brokkr/agy-conversations/<id>/rejoin/`,
//! obeying (m1)–(m4) and the same-user limit for both the engine HOME and kept
//! tree in [`super::keep`] (operator rulings 2026-11-25 and 2026-11-26): the
//! mechanisms here make accidents visible and are not claimed to defeat a
//! same-user process or prove kept state unchanged.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use brokkr_core::seat::is_agy_uuid;
use rustix::fs::{fchmod, fstat, mkdirat, openat, statat, AtFlags, Dir, FileType, Mode, Stat};
use rustix::io::Errno;
use thiserror::Error;

use super::fold::Conversation;
use super::keep::{self, ParentError, SourceErrno};
use super::restore;
use crate::secret::{BoundSecret, SecretMatcher};
pub(super) use restore::publish_turn;

/// Subdirectory inside `<agy-conversations>/<id>/` holding kept rejoin state.
pub(super) const REJOIN_DIR: &str = "rejoin";
/// Kept copy of `<engine HOME>/.gemini/antigravity-cli/conversations/<id>.db`.
pub(super) const REJOIN_DB_FILE: &str = "conversation.db";
/// Kept copy of `<engine HOME>/.gemini/antigravity-cli/brain/<id>/`.
pub(super) const REJOIN_BRAIN_DIR: &str = "brain";

/// Total byte bound across `conversations/<id>.db` and all files under
/// `brain/<id>/`: 128 MiB (4× `TRANSCRIPT_SOURCE_CAP`).
pub(super) const REJOIN_SOURCE_CAP: u64 = 128 * 1024 * 1024;
/// Maximum total files and directories copied for rejoin.
pub(super) const REJOIN_ENTRY_CAP: usize = 4_096;
/// Maximum directory nesting depth under `brain/<id>/` (lab depth is 4).
pub(super) const REJOIN_DEPTH_CAP: usize = 16;

/// Copy bounds for keeping rejoin state; overridable in unit tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RejoinCaps {
    pub(super) bytes: u64,
    pub(super) entries: usize,
    pub(super) depth: usize,
}

impl RejoinCaps {
    pub(super) const DEFAULT: Self = Self {
        bytes: REJOIN_SOURCE_CAP,
        entries: REJOIN_ENTRY_CAP,
        depth: REJOIN_DEPTH_CAP,
    };
}

/// Why an agy work turn's rejoin state was not kept. Never echoes any id,
/// path, or bound value (#1227).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum RejoinNotice {
    #[error("agy rejoin state not kept: init's conversation id is not a lowercase UUID")]
    InvalidId,
    #[error("agy rejoin state not kept: {0}")]
    Parent(#[from] ParentError),
    #[error(
        "agy rejoin state not kept: masking the turn's secret bindings would rewrite the root \
         session row"
    )]
    ReferenceMasked,
    #[error(
        "agy rejoin state not kept: the conversation's transcript address could not be published"
    )]
    AddressUnavailable,
    #[error("agy rejoin state not kept: a bound secret value appeared in the conversation")]
    BoundValueAppeared,
    #[error("agy rejoin state not kept: {0}")]
    Source(#[from] SourceErrno),
    #[error("agy rejoin state not kept: conversation state exceeds the {0}-byte copy limit")]
    TooLarge(u64),
    #[error("agy rejoin state not kept: conversation state exceeds the {0}-entry limit")]
    TooManyEntries(usize),
    #[error("agy rejoin state not kept: conversation brain exceeds the {0}-level depth limit")]
    TooDeep(usize),
    #[error("agy rejoin state not kept: conversation database is empty")]
    EmptyDatabase,
    #[error("agy rejoin state not kept: conversation brain directory is empty")]
    EmptyBrain,
    #[error("agy rejoin state not kept: a conversation file changed while it was being copied")]
    ChangedUnderfoot,
    #[error("agy rejoin state not kept: rejoin state for this conversation already exists")]
    AlreadyKept,
    #[error("agy rejoin state not kept: could not copy rejoin state: {}", io::Error::from(*.0))]
    Io(io::ErrorKind),
}

/// Adapter-declared `agy-conversation` kinds carried into post-turn keeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DeclaredKinds {
    pub(super) transcript: bool,
    pub(super) root: bool,
}

/// Post-turn keeping inputs shared by transcript and rejoin publication.
pub(super) struct TurnKeep<'a> {
    pub(super) declared: DeclaredKinds,
    pub(super) probe: bool,
    pub(super) refused_to_start: bool,
    pub(super) pin_confirmed: bool,
    pub(super) conversation: &'a Conversation,
    pub(super) engine_home: &'a Path,
    pub(super) operator_home: Option<&'a Path>,
    pub(super) bindings: &'a [BoundSecret],
}

/// Deterministic observation points inside [`keep_rejoin_with`] so tests can
/// exercise source races without sleeping (decision 0071 ruling 2).
#[derive(Debug)]
pub(super) enum CopyPoint<'a> {
    OpenEntry(&'a OsStr),
    ReadFile(&'a OsStr),
    FinalVerify,
    FinalStat(&'a OsStr, &'a mut Stat),
}

type Hook<'a> = &'a mut dyn FnMut(CopyPoint<'_>);

/// Copy `<id>.db` and `brain/<id>/` into `rejoin/` under `caps`, operator-only
/// modes, no-symlink traversal, bound-value screening, and a final source check.
/// On refusal, persistence is refused (`persistent: true` is never recorded) and
/// rollback attempts to remove staged entries; an unlink I/O error can leave a
/// partial directory.
pub(super) fn keep_rejoin(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    conv_exists: bool,
    caps: RejoinCaps,
) -> Result<(), RejoinNotice> {
    keep_rejoin_with(
        conversation_id,
        engine_home,
        operator_home,
        bindings,
        conv_exists,
        caps,
        &mut noop_hook,
    )
}

pub(super) fn noop_hook(point: CopyPoint<'_>) {
    let _ = match point {
        CopyPoint::OpenEntry(n) | CopyPoint::ReadFile(n) => (Some(n), None),
        CopyPoint::FinalStat(n, s) => (Some(n), Some(s)),
        CopyPoint::FinalVerify => (None, None),
    };
}

/// [`keep_rejoin`] with a deterministic hook for race tests (decision 0071 ruling 2).
pub(super) fn keep_rejoin_with(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    conv_exists: bool,
    caps: RejoinCaps,
    on_point: &mut dyn FnMut(CopyPoint<'_>),
) -> Result<(), RejoinNotice> {
    if !is_agy_uuid(conversation_id) {
        return Err(RejoinNotice::InvalidId);
    }
    let parent_path = keep::resolve_parent_path(operator_home)?;
    let db_name = format!("{conversation_id}.db");
    let (conv_name, db_os) = (OsStr::new(conversation_id), OsStr::new(&db_name));
    let src = open_rejoin_source(engine_home, conv_name, db_os, on_point)?;
    let mut guard = RejoinRollback::begin(keep::owned_parent_raw(&parent_path)?);
    let mut state = CopyState::new(caps, bindings, on_point);
    let tree = copy_rejoin_tree(
        (conv_name, db_os),
        conv_exists,
        (src.db_fd, &src.db_stat, src.brain_dir, src.brain_stat),
        &mut guard,
        &mut state,
    )?;
    (state.on_point)(CopyPoint::FinalVerify);
    let convs = src.conversations_dir.as_fd();
    if !same_stamp_at(convs, db_os, &src.db_stat, state.on_point) {
        return Err(RejoinNotice::ChangedUnderfoot);
    }
    verify_dir_tree(src.brain_root.as_fd(), conv_name, &tree, state.on_point)?;
    guard.armed = false;
    Ok(())
}

fn copy_rejoin_tree(
    (conv_name, db_name): (&OsStr, &OsStr),
    conv_exists: bool,
    (db_fd, db_stat, brain_dir, brain_stat): (OwnedFd, &Stat, OwnedFd, Stat),
    guard: &mut RejoinRollback,
    state: &mut CopyState<'_>,
) -> Result<RecordedDir, RejoinNotice> {
    let (conv_fd, conv_node) = match conv_exists {
        true => guard.open_existing_root_dir(conv_name)?,
        false => guard.create_root_dir_with(conv_name, || {})?,
    };
    copy_rejoin_into_dir(
        (conv_fd.as_fd(), conv_node),
        (OsStr::new(REJOIN_DIR), db_name),
        (db_fd, db_stat, brain_dir, brain_stat),
        guard,
        state,
    )
}

pub(super) fn copy_rejoin_into_dir(
    (conv_fd, conv_node): (BorrowedFd<'_>, usize),
    (dir_name, db_name): (&OsStr, &OsStr),
    (db_fd, db_stat, brain_dir, brain_stat): (OwnedFd, &Stat, OwnedFd, Stat),
    guard: &mut RejoinRollback,
    state: &mut CopyState<'_>,
) -> Result<RecordedDir, RejoinNotice> {
    let (rejoin_fd, rejoin_node) = guard.create_dir(conv_fd, conv_node, dir_name)?;
    let dest = |fd, node| DestDir { fd, node, depth: 0 };
    state.charge_entry()?;
    let (rdest, db_dst) = (
        dest(rejoin_fd.as_fd(), rejoin_node),
        OsStr::new(REJOIN_DB_FILE),
    );
    copy_opened_file((db_fd, db_name, db_stat), rdest, db_dst, guard, state)?;
    state.charge_entry()?;
    let (brain_fd, brain_node) =
        guard.create_dir(rejoin_fd.as_fd(), rejoin_node, OsStr::new(REJOIN_BRAIN_DIR))?;
    let bdest = dest(brain_fd.as_fd(), brain_node);
    copy_brain_dir(brain_dir, brain_stat, bdest, guard, state)
}

pub(super) struct RejoinSource {
    pub(super) conversations_dir: OwnedFd,
    pub(super) db_fd: OwnedFd,
    pub(super) db_stat: Stat,
    pub(super) brain_root: OwnedFd,
    pub(super) brain_dir: OwnedFd,
    pub(super) brain_stat: Stat,
}

pub(super) fn open_non_empty_db(
    dir: BorrowedFd<'_>,
    name: &OsStr,
) -> Result<(OwnedFd, Stat), RejoinNotice> {
    let fd =
        openat(dir, name, keep::source_file_flags(), Mode::empty()).map_err(classify_file_errno)?;
    let stat = fstat(&fd).map_err(io_errno)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(RejoinNotice::Source(SourceErrno::Unsafe));
    }
    if stat.st_size == 0 {
        return Err(RejoinNotice::EmptyDatabase);
    }
    Ok((fd, stat))
}

pub(super) fn open_rejoin_source(
    engine_home: &Path,
    conv_name: &OsStr,
    db_name: &OsStr,
    on_point: &mut dyn FnMut(CopyPoint<'_>),
) -> Result<RejoinSource, RejoinNotice> {
    let mut agy_dir = open_src_dir(rustix::fs::CWD, engine_home.as_os_str())?;
    for step in [".gemini", "antigravity-cli"] {
        agy_dir = open_src_dir(agy_dir.as_fd(), OsStr::new(step))?;
    }
    let conversations_dir = open_src_dir(agy_dir.as_fd(), OsStr::new("conversations"))?;
    on_point(CopyPoint::OpenEntry(db_name));
    let (db_fd, db_stat) = open_non_empty_db(conversations_dir.as_fd(), db_name)?;
    let brain_root = open_src_dir(agy_dir.as_fd(), OsStr::new("brain"))?;
    let brain_dir = open_src_dir(brain_root.as_fd(), conv_name)?;
    let brain_stat = fstat(&brain_dir).map_err(io_errno)?;
    Ok(RejoinSource {
        conversations_dir,
        db_fd,
        db_stat,
        brain_root,
        brain_dir,
        brain_stat,
    })
}

pub(super) struct CopyState<'a> {
    caps: RejoinCaps,
    used_bytes: u64,
    used_entries: usize,
    matcher: SecretMatcher,
    pub(super) on_point: &'a mut dyn FnMut(CopyPoint<'_>),
}

impl<'a> CopyState<'a> {
    pub(super) fn new(caps: RejoinCaps, bindings: &[BoundSecret], on_point: Hook<'a>) -> Self {
        Self {
            caps,
            used_bytes: 0,
            used_entries: 0,
            matcher: SecretMatcher::new(bindings),
            on_point,
        }
    }

    pub(super) fn charge_entry(&mut self) -> Result<(), RejoinNotice> {
        self.used_entries += 1;
        if self.used_entries > self.caps.entries {
            return Err(RejoinNotice::TooManyEntries(self.caps.entries));
        }
        Ok(())
    }

    fn screen_name(&mut self, name: &OsStr) -> Result<(), RejoinNotice> {
        self.matcher.reset();
        if self.matcher.feed(name.as_bytes()) {
            return Err(RejoinNotice::BoundValueAppeared);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(super) struct DestDir<'a> {
    pub(super) fd: BorrowedFd<'a>,
    pub(super) node: usize,
    pub(super) depth: usize,
}

pub(super) struct RecordedDir {
    stat: Stat,
    entries: Vec<(OsString, RecordedEntry)>,
}

enum RecordedEntry {
    File(Stat),
    Dir(RecordedDir),
}

pub(super) fn copy_brain_dir(
    src_fd: OwnedFd,
    src_stat: Stat,
    dest: DestDir<'_>,
    guard: &mut RejoinRollback,
    state: &mut CopyState<'_>,
) -> Result<RecordedDir, RejoinNotice> {
    let mut names = Vec::new();
    for_each_dir_entry(&src_fd, |name| {
        state.charge_entry()?;
        state.screen_name(&name)?;
        names.push(name);
        Ok(())
    })?;
    if dest.depth == 0 && names.is_empty() {
        return Err(RejoinNotice::EmptyBrain);
    }
    names.sort_unstable();
    let mut entries = Vec::with_capacity(names.len());
    for name in names {
        (state.on_point)(CopyPoint::OpenEntry(&name));
        let entry_fd = openat(&src_fd, &name, keep::source_file_flags(), Mode::empty())
            .map_err(classify_file_errno)?;
        let entry_stat = fstat(&entry_fd).map_err(io_errno)?;
        let recorded = match FileType::from_raw_mode(entry_stat.st_mode) {
            FileType::Directory => {
                if dest.depth + 1 > state.caps.depth {
                    return Err(RejoinNotice::TooDeep(state.caps.depth));
                }
                let (child_fd, child_node) = guard.create_dir(dest.fd, dest.node, &name)?;
                let next = DestDir {
                    fd: child_fd.as_fd(),
                    node: child_node,
                    depth: dest.depth + 1,
                };
                RecordedEntry::Dir(copy_brain_dir(entry_fd, entry_stat, next, guard, state)?)
            }
            FileType::RegularFile => {
                copy_opened_file((entry_fd, &name, &entry_stat), dest, &name, guard, state)?;
                RecordedEntry::File(entry_stat)
            }
            _ => return Err(RejoinNotice::Source(SourceErrno::Unsafe)),
        };
        entries.push((name, recorded));
    }
    Ok(RecordedDir {
        stat: src_stat,
        entries,
    })
}

pub(super) fn verify_dir_tree(
    parent: BorrowedFd<'_>,
    name: &OsStr,
    expected: &RecordedDir,
    on_point: Hook<'_>,
) -> Result<(), RejoinNotice> {
    let Ok(fd) = openat(parent, name, keep::dir_flags(), Mode::empty()) else {
        return Err(RejoinNotice::ChangedUnderfoot);
    };
    let after_names = read_sorted_names(&fd, expected.entries.len())?;
    let names_ok = after_names
        .iter()
        .zip(&expected.entries)
        .all(|(a, (e, _))| a == e);
    let stamp_ok = fstat(&fd)
        .ok()
        .is_some_and(|a| same_stamp(&expected.stat, &a));
    if !names_ok || !stamp_ok {
        return Err(RejoinNotice::ChangedUnderfoot);
    }
    for (entry_name, entry) in &expected.entries {
        match entry {
            RecordedEntry::File(stat) => {
                if !same_stamp_at(fd.as_fd(), entry_name, stat, on_point) {
                    return Err(RejoinNotice::ChangedUnderfoot);
                }
            }
            RecordedEntry::Dir(sub) => verify_dir_tree(fd.as_fd(), entry_name, sub, on_point)?,
        }
    }
    Ok(())
}

fn read_sorted_names(fd: &OwnedFd, expected_len: usize) -> Result<Vec<OsString>, RejoinNotice> {
    let mut after_names = Vec::with_capacity(expected_len);
    for_each_dir_entry(fd, |entry_name| {
        if after_names.len() == expected_len {
            return Err(RejoinNotice::ChangedUnderfoot);
        }
        after_names.push(entry_name);
        Ok(())
    })?;
    if after_names.len() != expected_len {
        return Err(RejoinNotice::ChangedUnderfoot);
    }
    after_names.sort_unstable();
    Ok(after_names)
}

pub(super) fn same_stamp(expected: &Stat, actual: &Stat) -> bool {
    FileType::from_raw_mode(expected.st_mode) == FileType::from_raw_mode(actual.st_mode)
        && keep::same_entry(expected, actual)
        && (expected.st_size, expected.st_mtime, expected.st_mtime_nsec)
            == (actual.st_size, actual.st_mtime, actual.st_mtime_nsec)
}

pub(super) fn same_stamp_at(
    dir: BorrowedFd<'_>,
    name: &OsStr,
    expected: &Stat,
    hook: Hook<'_>,
) -> bool {
    statat(dir, name, AtFlags::SYMLINK_NOFOLLOW).is_ok_and(|mut actual| {
        hook(CopyPoint::FinalStat(name, &mut actual));
        same_stamp(expected, &actual)
    })
}

pub(super) fn for_each_dir_entry(
    dir: &OwnedFd,
    visit: impl FnMut(OsString) -> Result<(), RejoinNotice>,
) -> Result<(), RejoinNotice> {
    let listing = Dir::read_from(dir).map_err(io_errno)?;
    for_each_listed_entry(
        listing.map(|e| e.map(|d| OsStr::from_bytes(d.file_name().to_bytes()).to_owned())),
        visit,
    )
}

pub(super) fn for_each_listed_entry(
    entries: impl IntoIterator<Item = Result<OsString, Errno>>,
    mut visit: impl FnMut(OsString) -> Result<(), RejoinNotice>,
) -> Result<(), RejoinNotice> {
    for entry in entries {
        let name = entry.map_err(io_errno)?;
        if name != OsStr::new(".") && name != OsStr::new("..") {
            visit(name)?;
        }
    }
    Ok(())
}

pub(super) fn copy_opened_file(
    (src_fd, src_name, src_stat): (OwnedFd, &OsStr, &Stat),
    dest: DestDir<'_>,
    dest_name: &OsStr,
    guard: &mut RejoinRollback,
    state: &mut CopyState<'_>,
) -> Result<(), RejoinNotice> {
    let apparent = src_stat.st_size.unsigned_abs();
    if state.used_bytes.saturating_add(apparent) > state.caps.bytes {
        return Err(RejoinNotice::TooLarge(state.caps.bytes));
    }
    let dest_fd = guard.create_file(dest.fd, dest.node, dest_name)?;
    (state.on_point)(CopyPoint::ReadFile(src_name));
    let mut src_file = File::from(src_fd);
    let mut dest_file = File::from(dest_fd);
    let copied = stream_copy_and_screen(&mut src_file, &mut dest_file, state)?;
    if copied != apparent {
        return Err(RejoinNotice::ChangedUnderfoot);
    }
    Ok(())
}

fn stream_copy_and_screen(
    src: &mut File,
    dest: &mut File,
    state: &mut CopyState<'_>,
) -> Result<u64, RejoinNotice> {
    state.matcher.reset();
    let remaining = state.caps.bytes.saturating_sub(state.used_bytes);
    let cap = state.caps.bytes;
    let copied = keep::stream_copy_bounded(
        src,
        dest,
        remaining,
        |chunk| match state.matcher.feed(chunk) {
            true => Err(RejoinNotice::BoundValueAppeared),
            false => Ok(()),
        },
        || RejoinNotice::TooLarge(cap),
        |e| RejoinNotice::Io(e.kind()),
    )?;
    state.used_bytes = state.used_bytes.saturating_add(copied);
    Ok(copied)
}

#[cfg(test)]
pub(super) fn stream_copy_for_test(
    src: &mut File,
    dest: &mut File,
    cap: u64,
) -> Result<u64, RejoinNotice> {
    let caps = RejoinCaps {
        bytes: cap,
        entries: 1,
        depth: 1,
    };
    let mut hook = noop_hook;
    let mut state = CopyState::new(caps, &[], &mut hook);
    stream_copy_and_screen(src, dest, &mut state)
}

struct RollbackDir {
    name: OsString,
    stat: Stat,
    created: bool,
    children: Vec<RollbackEntry>,
}

enum RollbackEntry {
    File(OsString, Stat),
    Dir(usize),
}

/// RAII rollback guard holding only `parent_fd` across the walk; `Drop`
/// attempts to re-open created directories on demand (verifying identity) and
/// remove created entries in reverse order via [`keep::unlink_if_same`] (m4).
/// Unlink I/O errors are ignored and can leave a partial directory even as
/// persistence is refused (`persistent: true` is never recorded).
pub(super) struct RejoinRollback {
    armed: bool,
    parent_fd: OwnedFd,
    root: Vec<RollbackEntry>,
    dirs: Vec<RollbackDir>,
}

impl RejoinRollback {
    pub(super) fn begin(parent_fd: OwnedFd) -> Self {
        Self {
            armed: true,
            parent_fd,
            root: Vec::new(),
            dirs: Vec::new(),
        }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }

    fn record_dir(
        &mut self,
        parent: Option<usize>,
        name: &OsStr,
        stat: Stat,
        created: bool,
    ) -> usize {
        let idx = self.dirs.len();
        self.dirs.push(RollbackDir {
            name: name.to_os_string(),
            stat,
            created,
            children: Vec::new(),
        });
        match parent {
            None => self.root.push(RollbackEntry::Dir(idx)),
            Some(p) => self.dirs[p].children.push(RollbackEntry::Dir(idx)),
        }
        idx
    }

    pub(super) fn open_existing_root_dir(
        &mut self,
        name: &OsStr,
    ) -> Result<(OwnedFd, usize), RejoinNotice> {
        let fd =
            openat(&self.parent_fd, name, keep::dir_flags(), Mode::empty()).map_err(io_errno)?;
        let stat = fstat(&fd).map_err(io_errno)?;
        if !keep::is_owned_private_dir(
            stat.st_mode,
            stat.st_uid,
            rustix::process::geteuid().as_raw(),
        ) {
            return Err(RejoinNotice::Parent(ParentError::UnownedParent));
        }
        let idx = self.record_dir(None, name, stat, false);
        Ok((fd, idx))
    }

    fn finish_created_dir(
        &mut self,
        parent: Option<usize>,
        name: &OsStr,
        (fd, stat): (OwnedFd, Stat),
    ) -> Result<(OwnedFd, usize), RejoinNotice> {
        let idx = self.record_dir(parent, name, stat, true);
        fchmod(&fd, Mode::from_raw_mode(0o700)).map_err(io_errno)?;
        Ok((fd, idx))
    }

    pub(super) fn create_root_dir_with(
        &mut self,
        name: &OsStr,
        before_open: impl FnOnce(),
    ) -> Result<(OwnedFd, usize), RejoinNotice> {
        let created = create_verified_dir(self.parent_fd.as_fd(), name, before_open)?;
        self.finish_created_dir(None, name, created)
    }

    pub(super) fn create_dir(
        &mut self,
        parent_fd: BorrowedFd<'_>,
        parent_node: usize,
        name: &OsStr,
    ) -> Result<(OwnedFd, usize), RejoinNotice> {
        let created = create_verified_dir(parent_fd, name, || {})?;
        self.finish_created_dir(Some(parent_node), name, created)
    }

    pub(super) fn create_file(
        &mut self,
        parent_fd: BorrowedFd<'_>,
        parent_node: usize,
        name: &OsStr,
    ) -> Result<OwnedFd, RejoinNotice> {
        let mode = Mode::from_raw_mode(0o600);
        let fd = openat(parent_fd, name, keep::dest_file_flags(), mode).map_err(io_errno)?;
        let stat = fstat(&fd).map_err(io_errno)?;
        self.dirs[parent_node]
            .children
            .push(RollbackEntry::File(name.to_os_string(), stat));
        fchmod(&fd, mode).map_err(io_errno)?;
        Ok(fd)
    }

    fn rollback_entry(&self, parent_fd: BorrowedFd<'_>, entry: &RollbackEntry) {
        match entry {
            RollbackEntry::File(name, stat) => {
                keep::unlink_if_same(parent_fd, name, stat, AtFlags::empty());
            }
            RollbackEntry::Dir(idx) => {
                let dir = &self.dirs[*idx];
                if let Some(fd) = (!dir.children.is_empty())
                    .then(|| open_same_dir(parent_fd, &dir.name, &dir.stat))
                    .flatten()
                {
                    for child in dir.children.iter().rev() {
                        self.rollback_entry(fd.as_fd(), child);
                    }
                }
                if dir.created {
                    keep::unlink_if_same(parent_fd, &dir.name, &dir.stat, AtFlags::REMOVEDIR);
                }
            }
        }
    }
}

impl Drop for RejoinRollback {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for entry in self.root.iter().rev() {
            self.rollback_entry(self.parent_fd.as_fd(), entry);
        }
    }
}

fn create_verified_dir(
    parent_fd: BorrowedFd<'_>,
    name: &OsStr,
    before_open: impl FnOnce(),
) -> Result<(OwnedFd, Stat), RejoinNotice> {
    let mode = Mode::from_raw_mode(0o700);
    mkdirat(parent_fd, name, mode).map_err(|errno| match errno {
        Errno::EXIST => RejoinNotice::AlreadyKept,
        other => io_errno(other),
    })?;
    let created_stat = statat(parent_fd, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io_errno)?;
    before_open();
    keep::open_created_dir(
        parent_fd,
        name,
        &created_stat,
        RejoinNotice::Parent(ParentError::ReplacedDirectory),
        io_errno,
    )
}

pub(super) fn open_same_dir(
    parent_fd: BorrowedFd<'_>,
    name: &OsStr,
    expected: &Stat,
) -> Option<OwnedFd> {
    openat(parent_fd, name, keep::dir_flags(), Mode::empty())
        .and_then(|fd| fstat(&fd).map(|stat| (fd, stat)))
        .ok()
        .and_then(|(fd, stat)| keep::same_entry(expected, &stat).then_some(fd))
}

pub(super) fn open_src_dir(parent: BorrowedFd<'_>, name: &OsStr) -> Result<OwnedFd, RejoinNotice> {
    openat(parent, name, keep::dir_flags(), Mode::empty())
        .map_err(|e| keep::classify_source_dir_errno(e).into())
}

pub(super) fn io_errno(errno: Errno) -> RejoinNotice {
    RejoinNotice::Io(io::Error::from(errno).kind())
}

pub(super) fn classify_file_errno(errno: Errno) -> RejoinNotice {
    keep::classify_source_file_errno(errno).into()
}
