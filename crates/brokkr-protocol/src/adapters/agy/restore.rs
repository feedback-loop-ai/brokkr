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
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brokkr_core::seat::{is_agy_uuid, AGY_TRANSCRIPT_FILE, AGY_TRANSCRIPT_KIND};
use rustix::fs::{
    fchmod, flock, fstat, openat, renameat, statat, AtFlags, FileType, FlockOperation, Mode,
    OFlags, CWD,
};
use rustix::io::Errno;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use super::keep::{self, KeepNotice, ParentError, SourceErrno};
use super::rejoin::{
    self, CopyPoint, CopyState, DestDir, RejoinCaps, RejoinNotice, RejoinRollback,
    REJOIN_BRAIN_DIR, REJOIN_DB_FILE, REJOIN_DEPTH_CAP, REJOIN_DIR,
};
use super::{Prepared, StartMode};
use crate::adapters::{
    originating_harness_version, resume_gate, spawn, version_qualifies, LaunchPlan, ResumeGate,
    AGY_SHAPE,
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
    #[error("agy rejoin skipped: masking the turn's secret bindings would rewrite the launch row")]
    ReferenceMasked,
    #[error(
        "agy rejoin skipped: could not lock or open kept conversation: {}",
        io::Error::from(*.0)
    )]
    Io(io::ErrorKind),
    #[error("agy rejoin skipped ({0})")]
    Copy(#[from] RejoinNotice),
}

/// Exclusive advisory lock on one kept conversation plus ownership of the
/// set-aside `rejoin/` copy taken out of the live path before `agy` starts.
///
/// While the lock is still held on drop:
/// - If the rejoin was never confirmed, one `renameat` puts the set-aside copy
///   back at `rejoin/`.
/// - If the rejoin was confirmed, best-effort removal cleans the set-aside
///   non-live copy (leaving it for the reaper of `#1323` on failure) and never
///   touches the live path.
pub(super) struct RejoinLease {
    conv_fd: OwnedFd,
    aside: String,
    confirmed: bool,
    // Dropped last so the exclusive advisory lock stays held across put-back
    // and set-aside cleanup.
    _lock_fd: OwnedFd,
}

impl RejoinLease {
    pub(super) fn confirm(&mut self) {
        self.confirmed = true;
    }

    #[cfg(test)]
    pub(super) fn aside_name(&self) -> &str {
        &self.aside
    }
}

impl Drop for RejoinLease {
    fn drop(&mut self) {
        if self.confirmed {
            remove_entry_tree(
                self.conv_fd.as_fd(),
                OsStr::new(&self.aside),
                REJOIN_DEPTH_CAP + 4,
            );
        } else {
            let _ = renameat(&self.conv_fd, &self.aside, &self.conv_fd, REJOIN_DIR);
        }
    }
}

/// What [`plan_launch_with`] settled before spawning `agy`: the shared
/// [`LaunchPlan`], any held [`RejoinLease`], and any [`RestoreNotice`]
/// explaining why an offer fell back cold.
pub(super) struct PlannedLaunch {
    pub(super) plan: LaunchPlan,
    pub(super) lock: Option<RejoinLease>,
    pub(super) restore_notice: Option<RestoreNotice>,
}

fn probe_version(prepared: &Prepared, owned_env: &[(&str, &str)]) -> Option<String> {
    let probe_cmd = [prepared.launch.command[0].clone(), "--version".to_string()];
    spawn::observed_version_in(&probe_cmd, &prepared.workdir, owned_env)
}

fn declined_cold(
    prepared: &Prepared,
    owned_env: &[(&str, &str)],
    refusal: &'static str,
    observed: Option<String>,
    notice: Option<RestoreNotice>,
) -> PlannedLaunch {
    let version = match (observed, prepared.declared_root) {
        (Some(v), _) => Some(v),
        (None, true) => probe_version(prepared, owned_env),
        (None, false) => None,
    };
    PlannedLaunch {
        plan: cold_plan(prepared.launch.command.clone(), Some(refusal), version),
        lock: None,
        restore_notice: notice,
    }
}

fn pre_version_gate(
    prepared: &Prepared,
    offered: &str,
    input: &Value,
) -> Result<(String, PathBuf), &'static str> {
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
    let parent = check_owned_target(input, offered, prepared.operator_home.as_deref())?;
    Ok((applies_to, parent))
}

/// Settle whether this invocation runs cold or rejoins an offered
/// conversation, checking the shared `resume_gate`, MCP isolation, UUID
/// syntax, `owned_target`, installed version (via the shared
/// [`version_qualifies`] predicate), exclusive lock, whole-state restore, and
/// live-path take-out.
pub(super) fn plan_launch_with(
    prepared: &Prepared,
    mode: StartMode<'_>,
    input: &Value,
    owned_env: &[(&str, &str)],
    bindings: &[BoundSecret],
    on_restore: &mut dyn FnMut(CopyPoint<'_>),
) -> PlannedLaunch {
    let Some(offered) = mode.session.filter(|_| !mode.probe) else {
        let version = match prepared.declared_root && !mode.probe {
            true => probe_version(prepared, owned_env),
            false => None,
        };
        return PlannedLaunch {
            plan: cold_plan(prepared.launch.command.clone(), None, version),
            lock: None,
            restore_notice: None,
        };
    };
    let (applies_to, parent) = match pre_version_gate(prepared, offered, input) {
        Ok(checked) => checked,
        Err(reason) => return declined_cold(prepared, owned_env, reason, None, None),
    };
    let Some(version) = probe_version(prepared, owned_env) else {
        return PlannedLaunch {
            plan: cold_plan(
                prepared.launch.command.clone(),
                Some("unverified-harness"),
                None,
            ),
            lock: None,
            restore_notice: None,
        };
    };
    let qualifying = originating_harness_version(input)
        .filter(|v| !v.is_empty())
        .is_some_and(|originating| version_qualifies(&version, &applies_to, Some(originating)));
    if !qualifying {
        return declined_cold(
            prepared,
            owned_env,
            "unverified-harness",
            Some(version),
            None,
        );
    }
    if resumed_rows_would_mask(offered, &version, &parent, bindings) {
        return declined_cold(
            prepared,
            owned_env,
            "unverified-harness",
            Some(version),
            Some(RestoreNotice::ReferenceMasked),
        );
    }
    let lease = match lock_and_restore_with(
        offered,
        Path::new(prepared.home.path()),
        prepared.operator_home.as_deref(),
        RejoinCaps::DEFAULT,
        on_restore,
    ) {
        Ok(lease) => lease,
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
    PlannedLaunch {
        plan: rejoin_plan(command, offered.to_string(), version),
        lock: Some(lease),
        restore_notice: None,
    }
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

/// Fail-closed check of every coordinate the engine carries in
/// `resume_context.owned_target` (`provider_id`, `persistence_locator`,
/// `persistence_home`), matching `owned_dsh_root` in `adapters.rs`.
/// Closed `resume_context.owned_target` object the engine emits (#1227).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedTarget {
    provider_id: String,
    persistence_locator: String,
    persistence_home: String,
}

pub(super) fn check_owned_target(
    input: &Value,
    offered: &str,
    operator_home: Option<&Path>,
) -> Result<PathBuf, &'static str> {
    let raw = input
        .pointer("/resume_context/owned_target")
        .ok_or("unverified-harness")?;
    let target = OwnedTarget::deserialize(raw).map_err(|_| "unverified-harness")?;
    if target.provider_id != offered
        || target.persistence_locator != offered
        || target.persistence_home.is_empty()
    {
        return Err("unverified-harness");
    }
    let admitted = keep::resolve_parent_path(operator_home).map_err(|_| "unverified-harness")?;
    let (Ok(canon_admitted), Ok(canon_recorded)) = (
        std::fs::canonicalize(&admitted),
        std::fs::canonicalize(&target.persistence_home),
    ) else {
        return Err("unverified-harness");
    };
    if canon_admitted != canon_recorded {
        return Err("instance-changed");
    }
    Ok(admitted)
}

/// Take the exclusive lock for `conversation_id`, restore its kept
/// `conversation.db` and `brain/` whole into `engine_home` under `caps`, and
/// rename `rejoin/` aside out of the live path before `agy` starts.
#[cfg(test)]
pub(super) fn lock_and_restore(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    caps: RejoinCaps,
) -> Result<RejoinLease, RestoreNotice> {
    lock_and_restore_with(
        conversation_id,
        engine_home,
        operator_home,
        caps,
        &mut rejoin::noop_hook,
    )
}

pub(super) fn lock_and_restore_with(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    caps: RejoinCaps,
    on_point: &mut dyn FnMut(CopyPoint<'_>),
) -> Result<RejoinLease, RestoreNotice> {
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
    let lock_fd = acquire_lock(conv_fd.as_fd())?;
    let mut guard = restore_into_home(
        conv_fd.as_fd(),
        conversation_id,
        engine_home,
        caps,
        on_point,
    )?;
    let aside = next_staging_name("rejoin.aside");
    renameat(&conv_fd, REJOIN_DIR, &conv_fd, &aside).map_err(restore_io_errno)?;
    guard.disarm();
    Ok(RejoinLease {
        conv_fd,
        aside,
        confirmed: false,
        _lock_fd: lock_fd,
    })
}

fn acquire_lock(conv_fd: BorrowedFd<'_>) -> Result<OwnedFd, RestoreNotice> {
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
    // `fd` is an open regular file and `LOCK_EX | LOCK_NB` is non-blocking, so
    // the reachable error is contention (`EWOULDBLOCK`); any error declines
    // cold cleanly (removal-control: `l2_absent_missing_and_locked_kept_state_declines_run_cold`).
    flock(&fd, FlockOperation::NonBlockingLockExclusive).map_err(|_| RestoreNotice::LockHeld)?;
    Ok(fd)
}

struct KeptRejoinSource {
    db_fd: OwnedFd,
    db_stat: rustix::fs::Stat,
    brain_dir: OwnedFd,
    brain_stat: rustix::fs::Stat,
}

fn open_kept_rejoin_source(conv_fd: BorrowedFd<'_>) -> Result<KeptRejoinSource, RejoinNotice> {
    let rejoin_dir = rejoin::open_src_dir(conv_fd, OsStr::new(REJOIN_DIR))?;
    let (db_fd, db_stat) =
        rejoin::open_non_empty_db(rejoin_dir.as_fd(), OsStr::new(REJOIN_DB_FILE))?;
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
    on_point: &mut dyn FnMut(CopyPoint<'_>),
) -> Result<RejoinRollback, RejoinNotice> {
    let src = open_kept_rejoin_source(conv_fd)?;
    let mut agy_dir = rejoin::open_src_dir(CWD, engine_home.as_os_str())?;
    for step in [".gemini", "antigravity-cli"] {
        agy_dir = rejoin::open_src_dir(agy_dir.as_fd(), OsStr::new(step))?;
    }
    let mut guard = RejoinRollback::begin(agy_dir);
    let mut state = CopyState::new(caps, &[], on_point);
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
    let (root_fd, root_node) = guard.create_root_dir_with(OsStr::new("brain"), || {})?;
    state.charge_entry()?;
    let id_os = OsStr::new(conversation_id);
    let (dst_fd, dst_node) = guard.create_dir(root_fd.as_fd(), root_node, id_os)?;
    rejoin::copy_brain_dir(
        src.brain_dir,
        src.brain_stat,
        DestDir {
            fd: dst_fd.as_fd(),
            node: dst_node,
            depth: 0,
        },
        &mut guard,
        &mut state,
    )?;
    Ok(guard)
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
    fchmod(&dest_fd, mode).map_err(keep::io_errno)?;
    if let Err(err) = keep::copy_bounded(src_fd, dest_fd, cap).and_then(|()| {
        renameat(&conv_fd, &stage, &conv_fd, AGY_TRANSCRIPT_FILE).map_err(keep::io_errno)
    }) {
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

/// Stage the turn's end state into a non-live sibling directory under
/// `<agy-conversations>/<id>/`, verify source stamps under slice 1's rules,
/// and publish it to the already-vacated live `rejoin/` path with one
/// `renameat`. On any failure, the staging directory rolls back and the live
/// path stays empty.
pub(super) fn replace_rejoin(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    caps: RejoinCaps,
) -> Result<(), RejoinNotice> {
    replace_rejoin_with(
        conversation_id,
        engine_home,
        operator_home,
        bindings,
        caps,
        &mut rejoin::noop_hook,
    )
}

pub(super) fn replace_rejoin_with(
    conversation_id: &str,
    engine_home: &Path,
    operator_home: Option<&Path>,
    bindings: &[BoundSecret],
    caps: RejoinCaps,
    on_point: &mut dyn FnMut(CopyPoint<'_>),
) -> Result<(), RejoinNotice> {
    let parent_path = keep::resolve_parent_path(operator_home)?;
    let db_name = format!("{conversation_id}.db");
    let (conv_name, db_os) = (OsStr::new(conversation_id), OsStr::new(&db_name));
    let src = rejoin::open_rejoin_source(engine_home, conv_name, db_os, on_point)?;
    let mut guard = RejoinRollback::begin(keep::owned_parent_raw(&parent_path)?);
    let (conv_fd, conv_node) = guard.open_existing_root_dir(conv_name)?;
    let staging = next_staging_name("rejoin.next");
    let staging_os = OsStr::new(&staging);
    let mut state = CopyState::new(caps, bindings, on_point);
    let tree = rejoin::copy_rejoin_into_dir(
        (conv_fd.as_fd(), conv_node),
        (staging_os, db_os),
        (src.db_fd, &src.db_stat, src.brain_dir, src.brain_stat),
        &mut guard,
        &mut state,
    )?;
    (state.on_point)(CopyPoint::FinalVerify);
    if !rejoin::same_stamp_at(
        src.conversations_dir.as_fd(),
        db_os,
        &src.db_stat,
        state.on_point,
    ) {
        return Err(RejoinNotice::ChangedUnderfoot);
    }
    rejoin::verify_dir_tree(src.brain_root.as_fd(), conv_name, &tree, state.on_point)?;
    renameat(&conv_fd, staging_os, &conv_fd, REJOIN_DIR).map_err(rejoin::io_errno)?;
    guard.disarm();
    Ok(())
}

/// Best-effort removal of a set-aside non-live directory via `openat` +
/// `O_NOFOLLOW` + `unlink_if_same` (obeying (m1) and (m4)); any entry that
/// cannot be removed stays under its non-live name for the reaper of `#1323`.
pub(super) fn remove_entry_tree(parent_fd: BorrowedFd<'_>, name: &OsStr, budget: usize) {
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

mod publish;
pub(super) use publish::publish_turn;
use publish::resumed_rows_would_mask;

fn restore_io_errno(errno: Errno) -> RestoreNotice {
    RestoreNotice::Io(io::Error::from(errno).kind())
}
