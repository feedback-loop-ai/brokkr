//! Restoring kept Antigravity CLI rejoin state into the engine's fresh HOME,
//! holding the per-conversation exclusive lock across restore, turn and keep,
//! planning a cold or `--conversation` launch through the shared resume
//! machinery, and replacing or removing kept state after a confirmed rejoin
//! (#1227, decision 0075 level 2 slice 2).
//!
//! Operates under the same stated limit as [`super::keep`] and
//! [`super::rejoin`] (operator rulings of 2026-11-25 and 2026-11-26): the
//! lock is advisory and the mechanisms guard against accidents, not against a
//! same-user process modifying the kept tree or engine HOME during a turn.

use std::ffi::OsStr;
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use brokkr_core::seat::{is_agy_uuid, AGY_TRANSCRIPT_FILE, AGY_TRANSCRIPT_KIND};
use rustix::fs::{
    fchmod, flock, fstat, openat, renameat, statat, AtFlags, FileType, FlockOperation, Mode,
    OFlags, CWD,
};
use rustix::io::Errno;
use serde_json::{Map, Value};
use thiserror::Error;

use super::fold::Conversation;
use super::keep::{self, KeepNotice, ParentError, SourceErrno};
use super::rejoin::{
    self, CopyState, DestDir, RejoinCaps, RejoinNotice, RejoinRollback, TurnKeep, REJOIN_BRAIN_DIR,
    REJOIN_DB_FILE, REJOIN_DEPTH_CAP, REJOIN_DIR,
};
use super::{home, LaunchError, Prepared, StartMode};
use crate::adapters::{
    originating_harness_version, resume_gate, spawn, Emitted, Invocation, LaunchHold, LaunchPlan,
    LaunchTerminal, ResumeGate, RootSession, Step, AGY_SHAPE,
};
use crate::secret::BoundSecret;

/// Exclusive advisory lock file inside `<agy-conversations>/<id>/`.
pub(super) const LOCK_FILE: &str = ".rejoin.lock";

static STAGING_SEQ: AtomicU64 = AtomicU64::new(0);

pub(super) fn next_staging_name(prefix: &str) -> String {
    let seq = STAGING_SEQ.fetch_add(1, Ordering::Relaxed);
    format!(".{prefix}.{}.{seq}", std::process::id())
}

/// Why an offered conversation was not restored into the engine's HOME.
/// Reported as a notice without echoing any id, path or bound value (#1227).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum RestoreNotice {
    #[error("agy rejoin skipped: {0}")]
    Parent(#[from] ParentError),
    #[error("agy rejoin skipped: kept conversation state is absent")]
    Absent,
    #[error("agy rejoin skipped: another turn holds the conversation lock")]
    LockHeld,
    #[error(
        "agy rejoin skipped: could not lock or open kept conversation: {}",
        io::Error::from(*.0)
    )]
    Io(io::ErrorKind),
    #[error("agy rejoin skipped ({0})")]
    Copy(#[from] RejoinNotice),
}

/// Exclusive advisory lock on one kept conversation, held from before restore
/// until after post-turn keeping completes and released automatically on drop.
pub(super) struct ConversationLock {
    _fd: OwnedFd,
}

/// What [`plan_launch`] settled before spawning `agy`: the shared
/// [`LaunchPlan`], any held [`ConversationLock`], and any [`RestoreNotice`]
/// explaining why an offer fell back cold.
pub(super) struct PlannedLaunch {
    pub(super) plan: LaunchPlan,
    pub(super) lock: Option<ConversationLock>,
    pub(super) restore_notice: Option<RestoreNotice>,
}

/// Settle whether this invocation runs cold or rejoins an offered
/// conversation, checking the shared `resume_gate`, MCP isolation, UUID
/// syntax, `owned_target`, installed version (probed once under the engine
/// HOME), exclusive lock, and whole-state restore into the engine HOME.
fn probe_version(
    prepared: &Prepared,
    owned_env: &[(&str, &str)],
) -> Result<Option<String>, LaunchError> {
    let probe_cmd = [prepared.launch.command[0].clone(), "--version".to_string()];
    let version = spawn::observed_version_in(&probe_cmd, &prepared.workdir, owned_env);
    home::unconfigured(&prepared.workdir)?;
    Ok(version)
}

fn declined_cold(
    prepared: &Prepared,
    owned_env: &[(&str, &str)],
    refusal: &'static str,
    observed: Option<String>,
    notice: Option<RestoreNotice>,
) -> Result<PlannedLaunch, LaunchError> {
    let version = match (observed, prepared.declared_root) {
        (Some(v), _) => Some(v),
        (None, true) => probe_version(prepared, owned_env)?,
        (None, false) => None,
    };
    Ok(PlannedLaunch {
        plan: cold_plan(prepared.launch.command.clone(), Some(refusal), version),
        lock: None,
        restore_notice: notice,
    })
}

fn pre_version_gate(
    prepared: &Prepared,
    offered: &str,
    input: &Value,
) -> Result<String, &'static str> {
    let applies_to = match resume_gate(input, AGY_SHAPE) {
        ResumeGate::Disabled(reason) => return Err(reason),
        ResumeGate::Enabled { applies_to } => applies_to,
    };
    if !prepared.declared_root {
        return Err("unsupported-resume");
    }
    if !prepared.isolated_resumes {
        return Err("restrictions-unavailable");
    }
    if !is_agy_uuid(offered) {
        return Err("invalid-session-id");
    }
    if let Some(reason) = check_owned_target(input, offered, prepared.operator_home.as_deref()) {
        return Err(reason);
    }
    Ok(applies_to)
}

pub(super) fn plan_launch(
    prepared: &Prepared,
    mode: StartMode<'_>,
    input: &Value,
    owned_env: &[(&str, &str)],
) -> Result<PlannedLaunch, LaunchError> {
    let Some(offered) = mode.session.filter(|_| !mode.probe) else {
        let version = match prepared.declared_root && !mode.probe {
            true => probe_version(prepared, owned_env)?,
            false => None,
        };
        return Ok(PlannedLaunch {
            plan: cold_plan(prepared.launch.command.clone(), None, version),
            lock: None,
            restore_notice: None,
        });
    };
    let applies_to = match pre_version_gate(prepared, offered, input) {
        Ok(version) => version,
        Err(reason) => return declined_cold(prepared, owned_env, reason, None, None),
    };
    let Some(version) = probe_version(prepared, owned_env)? else {
        return Ok(PlannedLaunch {
            plan: cold_plan(
                prepared.launch.command.clone(),
                Some("unverified-harness"),
                None,
            ),
            lock: None,
            restore_notice: None,
        });
    };
    let drifted = version != applies_to
        || originating_harness_version(input).is_some_and(|orig| orig != version);
    if drifted {
        return declined_cold(
            prepared,
            owned_env,
            "unverified-harness",
            Some(version),
            None,
        );
    }
    let lock = match lock_and_restore(
        offered,
        Path::new(prepared.home.path()),
        prepared.operator_home.as_deref(),
        RejoinCaps::DEFAULT,
    ) {
        Ok(lock) => lock,
        Err(notice) => {
            return declined_cold(
                prepared,
                owned_env,
                "unverified-harness",
                Some(version),
                Some(notice),
            );
        }
    };
    let mut command = prepared.launch.command.clone();
    command.extend(["--conversation".to_string(), offered.to_string()]);
    Ok(PlannedLaunch {
        plan: rejoin_plan(command, offered.to_string(), version),
        lock: Some(lock),
        restore_notice: None,
    })
}

fn cold_plan(
    command: Vec<String>,
    refusal: Option<&'static str>,
    harness_version: Option<String>,
) -> LaunchPlan {
    LaunchPlan {
        command,
        rejoining: None,
        refusal,
        sandbox: None,
        kind: AGY_TRANSCRIPT_KIND,
        harness_version,
        wrapper_digest: None,
        persistent: true,
        confirms_from_locator: true,
        effort: None,
    }
}

fn rejoin_plan(command: Vec<String>, offered: String, harness_version: String) -> LaunchPlan {
    LaunchPlan {
        command,
        rejoining: Some(offered),
        refusal: None,
        sandbox: None,
        kind: AGY_TRANSCRIPT_KIND,
        harness_version: Some(harness_version),
        wrapper_digest: None,
        persistent: true,
        confirms_from_locator: true,
        effort: None,
    }
}

fn check_owned_target(
    input: &Value,
    offered: &str,
    operator_home: Option<&Path>,
) -> Option<&'static str> {
    let target = input.pointer("/resume_context/owned_target")?;
    if target
        .get("provider_id")
        .and_then(Value::as_str)
        .is_some_and(|pid| pid != offered)
    {
        return Some("unverified-harness");
    }
    let recorded_home = target
        .get("persistence_home")
        .and_then(Value::as_str)
        .filter(|home| !home.is_empty())?;
    let Ok(admitted) = keep::resolve_parent_path(operator_home) else {
        return Some("unverified-harness");
    };
    let (Ok(canon_admitted), Ok(canon_recorded)) = (
        std::fs::canonicalize(&admitted),
        std::fs::canonicalize(recorded_home),
    ) else {
        return Some("unverified-harness");
    };
    (canon_admitted != canon_recorded).then_some("instance-changed")
}

/// Take the exclusive lock for `conversation_id` and restore its kept
/// `conversation.db` and `brain/` whole into `engine_home` under `caps`.
/// On any failure, removes anything partly written to `engine_home` and drops
/// the lock before returning the typed [`RestoreNotice`].
pub(super) fn lock_and_restore(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    caps: RejoinCaps,
) -> Result<ConversationLock, RestoreNotice> {
    let parent_path = keep::resolve_parent_path(operator_home)?;
    let Some(parent_fd) = open_existing_owned_parent(&parent_path)? else {
        return Err(RestoreNotice::Absent);
    };
    let conv_fd = match openat(
        &parent_fd,
        conversation_id,
        keep::dir_flags(),
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Err(RestoreNotice::Absent),
        Err(errno) => {
            return Err(RejoinNotice::from(keep::classify_source_dir_errno(errno)).into());
        }
    };
    let conv_stat = fstat(&conv_fd).map_err(restore_io_errno)?;
    if !keep::is_owned_private_dir(
        conv_stat.st_mode,
        conv_stat.st_uid,
        rustix::process::geteuid().as_raw(),
    ) {
        return Err(RestoreNotice::Parent(ParentError::UnownedParent));
    }
    let lock = acquire_lock(conv_fd.as_fd())?;
    restore_into_home(conv_fd.as_fd(), conversation_id, engine_home, caps)?;
    Ok(lock)
}

fn acquire_lock(conv_fd: BorrowedFd<'_>) -> Result<ConversationLock, RestoreNotice> {
    let (fd, created) = match openat(
        conv_fd,
        LOCK_FILE,
        OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => (fd, false),
        Err(Errno::NOENT) => {
            if statat(conv_fd, REJOIN_DIR, AtFlags::SYMLINK_NOFOLLOW).is_err() {
                return Err(RestoreNotice::Absent);
            }
            let mode = Mode::from_raw_mode(0o600);
            let fd = openat(
                conv_fd,
                LOCK_FILE,
                OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                mode,
            )
            .map_err(restore_io_errno)?;
            (fd, true)
        }
        Err(errno) => {
            return Err(RejoinNotice::from(keep::classify_source_file_errno(errno)).into());
        }
    };
    let stat = fstat(&fd).map_err(restore_io_errno)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
        return Err(RejoinNotice::Source(SourceErrno::Unsafe).into());
    }
    if created {
        fchmod(&fd, Mode::from_raw_mode(0o600)).map_err(restore_io_errno)?;
    }
    match flock(&fd, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => Ok(ConversationLock { _fd: fd }),
        Err(Errno::WOULDBLOCK) => Err(RestoreNotice::LockHeld),
        Err(errno) => Err(restore_io_errno(errno)),
    }
}

struct KeptRejoinSource {
    db_fd: OwnedFd,
    db_stat: rustix::fs::Stat,
    brain_dir: OwnedFd,
    brain_stat: rustix::fs::Stat,
}

fn open_kept_rejoin_source(conv_fd: BorrowedFd<'_>) -> Result<KeptRejoinSource, RejoinNotice> {
    let rejoin_dir = rejoin::open_src_dir(conv_fd, OsStr::new(REJOIN_DIR))?;
    let db_fd = openat(
        &rejoin_dir,
        REJOIN_DB_FILE,
        keep::source_file_flags(),
        Mode::empty(),
    )
    .map_err(rejoin::classify_file_errno)?;
    let db_stat = fstat(&db_fd).map_err(rejoin::io_errno)?;
    if FileType::from_raw_mode(db_stat.st_mode) != FileType::RegularFile {
        return Err(RejoinNotice::Source(SourceErrno::Unsafe));
    }
    let brain_dir = rejoin::open_src_dir(rejoin_dir.as_fd(), OsStr::new(REJOIN_BRAIN_DIR))?;
    let brain_stat = fstat(&brain_dir).map_err(rejoin::io_errno)?;
    Ok(KeptRejoinSource {
        db_fd,
        db_stat,
        brain_dir,
        brain_stat,
    })
}

fn restore_into_home(
    conv_fd: BorrowedFd<'_>,
    conversation_id: &str,
    engine_home: &Path,
    caps: RejoinCaps,
) -> Result<(), RejoinNotice> {
    let src = open_kept_rejoin_source(conv_fd)?;
    let mut agy_dir = rejoin::open_src_dir(CWD, engine_home.as_os_str())?;
    for step in [".gemini", "antigravity-cli"] {
        agy_dir = rejoin::open_src_dir(agy_dir.as_fd(), OsStr::new(step))?;
    }
    let mut guard = RejoinRollback::begin(agy_dir);
    let mut hook = rejoin::noop_hook;
    let mut state = CopyState::new(caps, &[], &mut hook);
    let (convs_fd, convs_node) = guard.create_root_dir_with(OsStr::new("conversations"), || {})?;
    state.charge_entry()?;
    let db_dst_name = format!("{conversation_id}.db");
    rejoin::copy_opened_file(
        (src.db_fd, OsStr::new(REJOIN_DB_FILE), &src.db_stat),
        DestDir {
            fd: convs_fd.as_fd(),
            node: convs_node,
            depth: 0,
        },
        OsStr::new(&db_dst_name),
        &mut guard,
        &mut state,
    )?;
    let (brain_root_fd, brain_root_node) =
        guard.create_root_dir_with(OsStr::new("brain"), || {})?;
    state.charge_entry()?;
    let (brain_dst_fd, brain_dst_node) = guard.create_dir(
        brain_root_fd.as_fd(),
        brain_root_node,
        OsStr::new(conversation_id),
    )?;
    rejoin::copy_brain_dir(
        src.brain_dir,
        src.brain_stat,
        DestDir {
            fd: brain_dst_fd.as_fd(),
            node: brain_dst_node,
            depth: 0,
        },
        &mut guard,
        &mut state,
    )?;
    guard.disarm();
    Ok(())
}

/// Open an already-existing `agy-conversations` parent without creating any
/// directory, returning `Ok(None)` when absent and verifying owner-only
/// directory ownership when present (#1227).
pub(super) fn open_existing_owned_parent(parent: &Path) -> Result<Option<OwnedFd>, ParentError> {
    let fd = match openat(CWD, parent, keep::dir_flags(), Mode::empty()) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Ok(None),
        Err(errno) => return Err(keep::parent_leaf_errno(errno)),
    };
    let stat = fstat(&fd).map_err(keep::parent_err_errno)?;
    let uid = rustix::process::geteuid().as_raw();
    if !keep::is_owned_private_dir(stat.st_mode, stat.st_uid, uid) {
        return Err(ParentError::UnownedParent);
    }
    Ok(Some(fd))
}

/// Replace `transcript.jsonl` inside an already-kept conversation directory
/// after a confirmed rejoin, staging with `O_CREAT | O_EXCL | O_NOFOLLOW` at
/// mode 0600 and renaming over the existing file.
pub(super) fn stage_replace_transcript(
    parent_fd: &OwnedFd,
    conversation_id: &str,
    src_fd: OwnedFd,
    cap: u64,
) -> Result<(), KeepNotice> {
    let conv_fd = openat(parent_fd, conversation_id, keep::dir_flags(), Mode::empty())
        .map_err(keep::io_errno)?;
    let conv_stat = fstat(&conv_fd).map_err(keep::io_errno)?;
    if !keep::is_owned_private_dir(
        conv_stat.st_mode,
        conv_stat.st_uid,
        rustix::process::geteuid().as_raw(),
    ) {
        return Err(KeepNotice::Parent(ParentError::UnownedParent));
    }
    let stage = next_staging_name("transcript.next");
    let mode = Mode::from_raw_mode(0o600);
    let dest_fd =
        openat(&conv_fd, &stage, keep::dest_file_flags(), mode).map_err(keep::io_errno)?;
    let dest_stat = fstat(&dest_fd).map_err(keep::io_errno)?;
    if let Err(err) = fchmod(&dest_fd, mode)
        .map_err(keep::io_errno)
        .and_then(|()| keep::copy_bounded(src_fd, dest_fd, cap))
        .and_then(|()| {
            renameat(&conv_fd, &stage, &conv_fd, AGY_TRANSCRIPT_FILE).map_err(keep::io_errno)
        })
    {
        keep::unlink_if_same(
            conv_fd.as_fd(),
            OsStr::new(&stage),
            &dest_stat,
            AtFlags::empty(),
        );
        return Err(err);
    }
    Ok(())
}

/// Replace kept rejoin state after a confirmed rejoin under slice 1's rules.
/// On any failure, removes the previous `rejoin/` directory too so no stale
/// older copy survives.
pub(super) fn replace_rejoin(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    caps: RejoinCaps,
) -> Result<(), RejoinNotice> {
    let result = stage_and_swap_rejoin(conversation_id, engine_home, operator_home, bindings, caps);
    if result.is_err() {
        remove_kept_rejoin(conversation_id, operator_home);
    }
    result
}

fn stage_and_swap_rejoin(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    caps: RejoinCaps,
) -> Result<(), RejoinNotice> {
    let parent_path = keep::resolve_parent_path(operator_home)?;
    let db_name = format!("{conversation_id}.db");
    let (conv_name, db_os) = (OsStr::new(conversation_id), OsStr::new(&db_name));
    let mut hook = rejoin::noop_hook;
    let src = rejoin::open_rejoin_source(engine_home, conv_name, db_os, &mut hook)?;
    let mut guard = RejoinRollback::begin(keep::owned_parent_raw(&parent_path)?);
    let (conv_fd, conv_node) = guard.open_existing_root_dir(conv_name)?;
    let staging = next_staging_name("rejoin.next");
    let staging_os = OsStr::new(&staging);
    let mut state = CopyState::new(caps, bindings, &mut hook);
    let tree = rejoin::copy_rejoin_into_dir(
        (conv_fd.as_fd(), conv_node),
        (staging_os, db_os),
        (src.db_fd, &src.db_stat, src.brain_dir, src.brain_stat),
        &mut guard,
        &mut state,
    )?;
    if !rejoin::same_stamp_at(
        src.conversations_dir.as_fd(),
        db_os,
        &src.db_stat,
        state.on_point,
    ) {
        return Err(RejoinNotice::ChangedUnderfoot);
    }
    rejoin::verify_dir_tree(src.brain_root.as_fd(), conv_name, &tree, state.on_point)?;
    remove_entry_tree(
        conv_fd.as_fd(),
        OsStr::new(REJOIN_DIR),
        REJOIN_DEPTH_CAP + 4,
    );
    renameat(&conv_fd, staging_os, &conv_fd, REJOIN_DIR).map_err(rejoin::io_errno)?;
    guard.disarm();
    Ok(())
}

/// Attempt to remove `<agy-conversations>/<id>/rejoin/` via `openat` +
/// `O_NOFOLLOW` + `unlink_if_same` (obeying (m1) and (m4)).
pub(super) fn remove_kept_rejoin(conversation_id: &str, operator_home: Option<&Path>) {
    let Ok(parent_path) = keep::resolve_parent_path(operator_home) else {
        return;
    };
    let Ok(Some(parent_fd)) = open_existing_owned_parent(&parent_path) else {
        return;
    };
    let Ok(conv_fd) = openat(
        &parent_fd,
        conversation_id,
        keep::dir_flags(),
        Mode::empty(),
    ) else {
        return;
    };
    let Ok(conv_stat) = fstat(&conv_fd) else {
        return;
    };
    if !keep::is_owned_private_dir(
        conv_stat.st_mode,
        conv_stat.st_uid,
        rustix::process::geteuid().as_raw(),
    ) {
        return;
    }
    remove_entry_tree(
        conv_fd.as_fd(),
        OsStr::new(REJOIN_DIR),
        REJOIN_DEPTH_CAP + 4,
    );
}

fn remove_entry_tree(parent_fd: BorrowedFd<'_>, name: &OsStr, budget: usize) {
    let Ok(stat) = statat(parent_fd, name, AtFlags::SYMLINK_NOFOLLOW) else {
        return;
    };
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
        keep::unlink_if_same(parent_fd, name, &stat, AtFlags::empty());
        return;
    }
    if budget > 0 {
        if let Some(dir_fd) = rejoin::open_same_dir(parent_fd, name, &stat) {
            let mut names = Vec::new();
            let _ = rejoin::for_each_dir_entry(&dir_fd, |entry| {
                names.push(entry);
                Ok(())
            });
            for entry in names {
                remove_entry_tree(dir_fd.as_fd(), &entry, budget - 1);
            }
        }
    }
    keep::unlink_if_same(parent_fd, name, &stat, AtFlags::REMOVEDIR);
}

/// Publish the kept transcript and, for a declaring work turn (or a declined
/// offer), the kept rejoin state and `harness-started` checkpoint before the
/// engine HOME is removed and while any conversation lock is still held (#1227).
pub(super) fn publish_turn(
    turn: TurnKeep<'_>,
    hold: &mut LaunchHold,
    outcome: &mut Result<Invocation, super::Ended>,
    emit: &mut impl FnMut(&Emitted),
) {
    let terminal = hold.terminal();
    // Unsettled rejoin (mismatch or unconfirmed): keep nothing and leave the
    // offered conversation's kept state untouched.
    if terminal.is_unsettled() {
        return;
    }
    let resumed = terminal == LaunchTerminal::Resumed;
    let mut fallback_meta = Map::new();
    let target_meta = match outcome {
        Ok(invocation) => &mut invocation.session_meta,
        Err(_) => &mut fallback_meta,
    };
    let ctx = keep::KeepContext {
        declared: turn.declared.transcript,
        probe: turn.probe,
        refused_to_start: turn.refused_to_start,
        conversation: turn.conversation,
        engine_home: turn.engine_home,
        operator_home: turn.operator_home,
        bindings: turn.bindings,
    };
    let transcript_outcome = if resumed && !turn.refused_to_start {
        keep::publish_transcript_mode(ctx, true, target_meta, emit)
    } else {
        keep::publish_transcript(ctx, target_meta, emit)
    };
    let kept_transcript = match &transcript_outcome {
        keep::TranscriptOutcome::Kept(kept) => Some(kept),
        keep::TranscriptOutcome::NotKept(notice) => {
            if let Some(notice) = notice {
                keep::report_notice(outcome, notice);
            }
            None
        }
    };
    if let Some(notice) = publish_root_session(&turn, hold, resumed, kept_transcript, emit) {
        keep::report_notice(outcome, &notice);
    }
}

fn publish_root_session(
    turn: &TurnKeep<'_>,
    hold: &mut LaunchHold,
    resumed: bool,
    kept_transcript: Option<&keep::Kept>,
    emit: &mut impl FnMut(&Emitted),
) -> Option<RejoinNotice> {
    if turn.probe || turn.refused_to_start {
        return None;
    }
    let declined_cold = !resumed && hold.has_refusal();
    let eligible = turn.declared.root && turn.pin_confirmed;
    let (Conversation::Named(id), Some(version)) = (turn.conversation, hold.harness_version())
    else {
        if declined_cold {
            hold.publish_settled(None, None, emit);
        }
        return None;
    };
    if !eligible {
        if declined_cold {
            hold.publish_settled(None, None, emit);
        }
        return None;
    }
    let transcript_address = kept_transcript.map(keep::Kept::transcript_value);
    if row_would_mask_launch(
        id,
        version,
        resumed,
        transcript_address.as_ref(),
        turn.bindings,
    ) {
        if resumed {
            remove_kept_rejoin(id, turn.operator_home);
        } else if declined_cold {
            hold.publish_settled(None, None, emit);
        }
        return Some(RejoinNotice::ReferenceMasked);
    }
    let keep_result = match resumed {
        true => replace_rejoin(
            id,
            turn.engine_home,
            turn.operator_home,
            turn.bindings,
            RejoinCaps::DEFAULT,
        ),
        false => rejoin::keep_rejoin(
            id,
            turn.engine_home,
            turn.operator_home,
            turn.bindings,
            kept_transcript.is_some(),
            RejoinCaps::DEFAULT,
        ),
    };
    let (persistent, notice) = match keep_result {
        Ok(()) => (true, None),
        Err(notice) => (false, Some(notice)),
    };
    let address = persistent.then_some(transcript_address).flatten();
    let root = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        id.clone(),
        version.to_string(),
        persistent,
    );
    hold.publish_settled(Some(root), address, emit);
    notice
}

fn row_would_mask_launch(
    id: &str,
    version: &str,
    resumed: bool,
    address: Option<&Value>,
    bindings: &[BoundSecret],
) -> bool {
    let root = RootSession::new(
        AGY_TRANSCRIPT_KIND,
        id.to_string(),
        version.to_string(),
        true,
    );
    let mut row = Map::new();
    row.insert("harness".into(), Value::String("agy".into()));
    row.insert(
        "launch".into(),
        Value::String(if resumed { "resumed" } else { "cold" }.into()),
    );
    row.insert("root_session".into(), root.value());
    if let Some(transcript) = address {
        row.insert("transcript".into(), transcript.clone());
    }
    let emitted = Emitted::new(Step::HarnessStarted, Value::Object(row));
    keep::row_would_mask(&emitted, bindings)
}

fn restore_io_errno(errno: Errno) -> RestoreNotice {
    RestoreNotice::Io(io::Error::from(errno).kind())
}
