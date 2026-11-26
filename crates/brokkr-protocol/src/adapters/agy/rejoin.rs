//! Keeping an agy work turn's rejoin state (`conversations/<id>.db` and
//! `brain/<id>/`) beside its kept `transcript.jsonl`, and publishing its
//! `root_session` checkpoint row (#1227, decision 0075 level 2).
//!
//! One kept place, one rule: every file and directory kept for rejoin lives
//! under `<operator HOME>/.local/state/brokkr/agy-conversations/<id>/rejoin/`,
//! obeying the same four mechanisms (m1)–(m4) and the same-user directory limit
//! documented in [`super::keep`] (operator ruling 2026-11-25).

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Stdio};

use brokkr_core::seat::{is_agy_uuid, AGY_TRANSCRIPT_KIND};
use rustix::fs::{fchmod, fstat, mkdirat, openat, statat, AtFlags, FileType, Mode, RawDir, Stat};
use rustix::io::Errno;
use serde_json::{Map, Value};
use thiserror::Error;

use super::fold::Conversation;
use super::keep::{self, ParentError, SourceErrno};
use crate::adapters::lost;
use crate::adapters::{Emitted, Invocation, RootSession, Step};
use crate::secret::{BoundSecret, SecretMatcher};

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

const RAW_DIR_BUF_BYTES: usize = 8_192;

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
    #[error("agy rejoin state not kept: a conversation file changed while it was being copied")]
    ChangedUnderfoot,
    #[error("agy rejoin state not kept: rejoin state for this conversation already exists")]
    AlreadyKept,
    #[error("agy rejoin state not kept: could not copy rejoin state: {}", io::Error::from(*.0))]
    Io(io::ErrorKind),
}

/// Ask the agy binary this turn launches for its version via `bin --version`,
/// under the engine's owned environment (`HOME`, `PWD`, and
/// `AGY_CLI_DISABLE_AUTO_UPDATE=true`, decision 0124) and with no bound
/// secrets. Parses stdout through the same rule as every other driver
/// ([`crate::adapters::parse_observed_version`]).
pub(super) fn observe_version(bin: &str, workdir: &str, owned: &[(&str, &str)]) -> Option<String> {
    let output = lost::observing(|| {
        let mut command = Command::new(bin);
        command
            .arg("--version")
            .current_dir(workdir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, value) in owned {
            command.env(name, value);
        }
        command.output()
    })
    .ok()?;
    if !output.status.success() {
        return None;
    }
    super::super::parse_observed_version(&output.stdout)
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
    pub(super) conversation: &'a Conversation,
    pub(super) harness_version: Option<&'a str>,
    pub(super) engine_home: &'a Path,
    pub(super) operator_home: Option<&'a Path>,
    pub(super) bindings: &'a [BoundSecret],
}

/// Publish the kept transcript and, for a declaring work turn, the kept rejoin
/// state and `root_session` before the engine HOME is removed (#1227).
pub(super) fn publish_turn(
    turn: TurnKeep<'_>,
    outcome: &mut Result<Invocation, super::Ended>,
    emit: &mut impl FnMut(&Emitted),
) {
    let mut fallback_meta = Map::new();
    let target_meta = match outcome {
        Ok(invocation) => &mut invocation.session_meta,
        Err(_) => &mut fallback_meta,
    };
    let transcript_notice = keep::publish_transcript(
        keep::KeepContext {
            declared: turn.declared.transcript,
            probe: turn.probe,
            refused_to_start: turn.refused_to_start,
            conversation: turn.conversation,
            engine_home: turn.engine_home,
            operator_home: turn.operator_home,
            bindings: turn.bindings,
        },
        target_meta,
        emit,
    );
    let kept_transcript = target_meta
        .get("transcript")
        .filter(|t| t.get("kind").and_then(Value::as_str) == Some(AGY_TRANSCRIPT_KIND))
        .cloned();
    if let Some(notice) = transcript_notice {
        keep::report_notice(outcome, &notice);
    }
    if let Some(notice) = publish_root_session(&turn, kept_transcript.as_ref(), emit) {
        keep::report_notice(outcome, &notice);
    }
}

fn publish_root_session(
    turn: &TurnKeep<'_>,
    kept_transcript: Option<&Value>,
    emit: &mut impl FnMut(&Emitted),
) -> Option<RejoinNotice> {
    if !turn.declared.root || turn.probe || turn.refused_to_start {
        return None;
    }
    let Conversation::Named(id) = turn.conversation else {
        return None;
    };
    let version = turn.harness_version?;
    let candidate = root_row(id, version, true, kept_transcript);
    if keep::row_would_mask(&candidate, turn.bindings) {
        return Some(RejoinNotice::ReferenceMasked);
    }
    let keep_result = keep_rejoin(
        id,
        turn.engine_home,
        turn.operator_home,
        turn.bindings,
        kept_transcript.is_some(),
        RejoinCaps::DEFAULT,
    );
    let (persistent, notice) = match keep_result {
        Ok(()) => (true, None),
        Err(notice) => (false, Some(notice)),
    };
    let address = persistent.then_some(kept_transcript).flatten();
    emit(&root_row(id, version, persistent, address));
    notice
}

fn root_row(id: &str, version: &str, persistent: bool, address: Option<&Value>) -> Emitted {
    let root = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        id.to_string(),
        version.to_string(),
        persistent,
    );
    let mut row = Map::new();
    row.insert("harness".into(), Value::String("agy".into()));
    row.insert("launch".into(), Value::String("cold".into()));
    row.insert("root_session".into(), root.value());
    if let Some(transcript) = address {
        row.insert("transcript".into(), transcript.clone());
    }
    Emitted::new(Step::HarnessStarted, Value::Object(row))
}

/// Copy `<engine_home>/.gemini/antigravity-cli/{conversations/<id>.db,brain/<id>/}`
/// into `<operator_home>/.local/state/brokkr/agy-conversations/<id>/rejoin/`
/// under `caps`, operator-only modes, no-symlink traversal, bound-value
/// screening, and all-or-nothing rollback.
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
        &mut |_| {},
    )
}

/// [`keep_rejoin`] with a per-file hook run after opening a source file and
/// before reading its bytes, so unit tests deterministically exercise the
/// changed-underfoot rollback path (decision 0071 ruling 2).
pub(super) fn keep_rejoin_with(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    conv_exists: bool,
    caps: RejoinCaps,
    before_copy: &mut dyn FnMut(&OsStr),
) -> Result<(), RejoinNotice> {
    if !is_agy_uuid(conversation_id) {
        return Err(RejoinNotice::InvalidId);
    }
    let parent_path = keep::resolve_parent_path(operator_home)?;
    let source = open_rejoin_source(engine_home, conversation_id)?;
    let parent_fd = keep::owned_parent_raw(&parent_path)?;
    let mut guard = RejoinGuard::new(parent_fd);
    let conv_idx = if conv_exists {
        guard.open_existing_dir(0, OsStr::new(conversation_id))?
    } else {
        guard.create_dir(0, OsStr::new(conversation_id))?
    };
    let rejoin_idx = guard.create_dir(conv_idx, OsStr::new(REJOIN_DIR))?;
    let mut state = CopyState {
        caps,
        used_bytes: 0,
        used_entries: 1,
        matcher: SecretMatcher::new(bindings),
        before_copy,
    };
    let db_name = format!("{conversation_id}.db");
    copy_regular_file(
        FileTarget {
            src_dir: &source.conversations_dir,
            src_name: OsStr::new(&db_name),
            dest_dir_idx: rejoin_idx,
            dest_name: OsStr::new(REJOIN_DB_FILE),
        },
        &mut guard,
        &mut state,
    )?;
    let brain_dest_idx = guard.create_dir(rejoin_idx, OsStr::new(REJOIN_BRAIN_DIR))?;
    copy_brain_dir(&source.brain_dir, &mut guard, brain_dest_idx, 0, &mut state)?;
    guard.defuse();
    Ok(())
}

struct RejoinSource {
    conversations_dir: OwnedFd,
    brain_dir: OwnedFd,
}

fn open_rejoin_source(
    engine_home: &Path,
    conversation_id: &str,
) -> Result<RejoinSource, RejoinNotice> {
    let home_dir = openat(
        rustix::fs::CWD,
        engine_home,
        keep::dir_flags(),
        Mode::empty(),
    )
    .map_err(classify_dir_errno)?;
    let gemini = openat(&home_dir, ".gemini", keep::dir_flags(), Mode::empty())
        .map_err(classify_dir_errno)?;
    let agy_dir = openat(&gemini, "antigravity-cli", keep::dir_flags(), Mode::empty())
        .map_err(classify_dir_errno)?;
    let conversations_dir = openat(&agy_dir, "conversations", keep::dir_flags(), Mode::empty())
        .map_err(classify_dir_errno)?;
    let db_name = format!("{conversation_id}.db");
    let db_stat = statat(&conversations_dir, &db_name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(classify_file_errno)?;
    if FileType::from_raw_mode(db_stat.st_mode) != FileType::RegularFile {
        return Err(RejoinNotice::Source(SourceErrno::Unsafe));
    }
    let brain_root =
        openat(&agy_dir, "brain", keep::dir_flags(), Mode::empty()).map_err(classify_dir_errno)?;
    let brain_dir = openat(
        &brain_root,
        conversation_id,
        keep::dir_flags(),
        Mode::empty(),
    )
    .map_err(classify_dir_errno)?;
    Ok(RejoinSource {
        conversations_dir,
        brain_dir,
    })
}

struct CopyState<'a> {
    caps: RejoinCaps,
    used_bytes: u64,
    used_entries: usize,
    matcher: SecretMatcher,
    before_copy: &'a mut dyn FnMut(&OsStr),
}

impl CopyState<'_> {
    fn charge_entry(&mut self) -> Result<(), RejoinNotice> {
        self.used_entries += 1;
        if self.used_entries > self.caps.entries {
            return Err(RejoinNotice::TooManyEntries(self.caps.entries));
        }
        Ok(())
    }
}

struct FileTarget<'a> {
    src_dir: &'a OwnedFd,
    src_name: &'a OsStr,
    dest_dir_idx: usize,
    dest_name: &'a OsStr,
}

fn copy_brain_dir(
    src_dir: &OwnedFd,
    guard: &mut RejoinGuard,
    dest_dir_idx: usize,
    depth: usize,
    state: &mut CopyState<'_>,
) -> Result<(), RejoinNotice> {
    let names = list_dir_sorted(src_dir)?;
    for name in names {
        state.charge_entry()?;
        let entry_stat =
            statat(src_dir, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(classify_file_errno)?;
        match FileType::from_raw_mode(entry_stat.st_mode) {
            FileType::Directory => {
                if depth + 1 > state.caps.depth {
                    return Err(RejoinNotice::TooDeep(state.caps.depth));
                }
                let child_src = openat(src_dir, &name, keep::dir_flags(), Mode::empty())
                    .map_err(classify_dir_errno)?;
                let child_dest_idx = guard.create_dir(dest_dir_idx, &name)?;
                copy_brain_dir(&child_src, guard, child_dest_idx, depth + 1, state)?;
            }
            FileType::RegularFile => {
                copy_regular_file(
                    FileTarget {
                        src_dir,
                        src_name: &name,
                        dest_dir_idx,
                        dest_name: &name,
                    },
                    guard,
                    state,
                )?;
            }
            _ => return Err(RejoinNotice::Source(SourceErrno::Unsafe)),
        }
    }
    Ok(())
}

fn list_dir_sorted(dir: &OwnedFd) -> Result<Vec<OsString>, RejoinNotice> {
    let mut names = Vec::new();
    let mut buf = [MaybeUninit::<u8>::uninit(); RAW_DIR_BUF_BYTES];
    let mut listing = RawDir::new(dir, &mut buf);
    while let Some(entry) = listing.next() {
        let entry = entry.map_err(io_errno)?;
        let bytes = entry.file_name().to_bytes();
        if bytes != b"." && bytes != b".." {
            names.push(OsStr::from_bytes(bytes).to_owned());
        }
    }
    names.sort_unstable();
    Ok(names)
}

fn copy_regular_file(
    target: FileTarget<'_>,
    guard: &mut RejoinGuard,
    state: &mut CopyState<'_>,
) -> Result<(), RejoinNotice> {
    let src_fd = openat(
        target.src_dir,
        target.src_name,
        keep::source_file_flags(),
        Mode::empty(),
    )
    .map_err(classify_file_errno)?;
    let before = fstat(&src_fd).map_err(io_errno)?;
    // `statat` already verified `FileType::RegularFile` before `openat` with
    // `O_NOFOLLOW`, and `verify_unchanged` checks `(st_dev, st_ino)` after EOF
    // (tested by `changed_underfoot_rolls_back_rejoin_and_spares_transcript`).
    let apparent = before.st_size.unsigned_abs();
    if state.used_bytes.saturating_add(apparent) > state.caps.bytes {
        return Err(RejoinNotice::TooLarge(state.caps.bytes));
    }
    let dest_fd = guard.create_file(target.dest_dir_idx, target.dest_name)?;
    (state.before_copy)(target.src_name);
    let mut src = File::from(src_fd);
    let mut dest = File::from(dest_fd);
    let copied = stream_copy_and_screen(&mut src, &mut dest, state)?;
    verify_unchanged(
        target.src_dir,
        target.src_name,
        &src,
        &before,
        apparent,
        copied,
    )
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
        io_err,
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
    let mut noop = |_: &OsStr| {};
    let mut state = CopyState {
        caps: RejoinCaps {
            bytes: cap,
            entries: 1,
            depth: 1,
        },
        used_bytes: 0,
        used_entries: 0,
        matcher: SecretMatcher::new(&[]),
        before_copy: &mut noop,
    };
    stream_copy_and_screen(src, dest, &mut state)
}

fn verify_unchanged(
    src_dir: &OwnedFd,
    src_name: &OsStr,
    src: &File,
    before: &Stat,
    apparent: u64,
    copied: u64,
) -> Result<(), RejoinNotice> {
    let after = fstat(src).map_err(io_errno)?;
    let entry_after = statat(src_dir, src_name, AtFlags::SYMLINK_NOFOLLOW)
        .map_err(|_| RejoinNotice::ChangedUnderfoot)?;
    let unchanged = keep::same_entry(before, &entry_after)
        && copied == apparent
        && after.st_size == before.st_size
        && after.st_mtime == before.st_mtime
        && after.st_mtime_nsec == before.st_mtime_nsec;
    match unchanged {
        true => Ok(()),
        false => Err(RejoinNotice::ChangedUnderfoot),
    }
}

/// RAII rollback guard for every directory and file created during one rejoin
/// keep, unlinking entries in reverse creation order through
/// [`keep::unlink_if_same`] (mechanism (m4)).
struct RejoinGuard {
    armed: bool,
    dirs: Vec<OwnedFd>,
    entries: Vec<(usize, OsString, Stat, AtFlags)>,
}

impl RejoinGuard {
    fn new(parent_fd: OwnedFd) -> Self {
        Self {
            armed: true,
            dirs: vec![parent_fd],
            entries: Vec::new(),
        }
    }

    fn open_existing_dir(
        &mut self,
        parent_idx: usize,
        name: &OsStr,
    ) -> Result<usize, RejoinNotice> {
        let fd = openat(
            &self.dirs[parent_idx],
            name,
            keep::dir_flags(),
            Mode::empty(),
        )
        .map_err(io_errno)?;
        let stat = fstat(&fd).map_err(io_errno)?;
        let uid = rustix::process::geteuid().as_raw();
        if !keep::is_owned_private_dir(stat.st_mode, stat.st_uid, uid) {
            return Err(RejoinNotice::Parent(ParentError::UnownedParent));
        }
        let idx = self.dirs.len();
        self.dirs.push(fd);
        Ok(idx)
    }

    fn create_dir(&mut self, parent_idx: usize, name: &OsStr) -> Result<usize, RejoinNotice> {
        let mode = Mode::from_raw_mode(0o700);
        mkdirat(&self.dirs[parent_idx], name, mode).map_err(|errno| match errno {
            Errno::EXIST => RejoinNotice::AlreadyKept,
            other => io_errno(other),
        })?;
        let created_stat =
            statat(&self.dirs[parent_idx], name, AtFlags::SYMLINK_NOFOLLOW).map_err(io_errno)?;
        let (fd, stat) = keep::open_created_dir(
            self.dirs[parent_idx].as_fd(),
            name,
            &created_stat,
            RejoinNotice::Parent(ParentError::ReplacedDirectory),
            io_errno,
        )?;
        let idx = self.dirs.len();
        self.dirs.push(fd);
        self.entries
            .push((parent_idx, name.to_os_string(), stat, AtFlags::REMOVEDIR));
        fchmod(&self.dirs[idx], mode).map_err(io_errno)?;
        Ok(idx)
    }

    fn create_file(&mut self, parent_idx: usize, name: &OsStr) -> Result<OwnedFd, RejoinNotice> {
        let mode = Mode::from_raw_mode(0o600);
        let fd = openat(&self.dirs[parent_idx], name, keep::dest_file_flags(), mode)
            .map_err(io_errno)?;
        let stat = fstat(&fd).map_err(io_errno)?;
        self.entries
            .push((parent_idx, name.to_os_string(), stat, AtFlags::empty()));
        fchmod(&fd, mode).map_err(io_errno)?;
        Ok(fd)
    }

    fn defuse(&mut self) {
        self.armed = false;
    }
}

impl Drop for RejoinGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for (parent_idx, name, stat, flags) in self.entries.iter().rev() {
            keep::unlink_if_same(self.dirs[*parent_idx].as_fd(), name, stat, *flags);
        }
    }
}

fn io_errno(errno: Errno) -> RejoinNotice {
    RejoinNotice::Io(io::Error::from(errno).kind())
}

fn io_err(err: io::Error) -> RejoinNotice {
    RejoinNotice::Io(err.kind())
}

fn classify_dir_errno(errno: Errno) -> RejoinNotice {
    keep::classify_source_dir_errno(errno).into()
}

fn classify_file_errno(errno: Errno) -> RejoinNotice {
    keep::classify_source_file_errno(errno).into()
}
