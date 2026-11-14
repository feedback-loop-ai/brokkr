//! Decision 0108 ruling 3's release, and the proof it rests on: an
//! acknowledged (quarantined) run stops holding its worktree once its run
//! lock is free and every attempt its own reservation names is proven ended
//! by the broker's settlement records and the host — never by the run's
//! fold. A run that no longer folds is exactly the case #440 records, and
//! the fold is never read here.
//!
//! The records are the reservation's: each attempt's driver settlement
//! records (U8e2, written at the dispatch door before a driver spawns, so an
//! empty list is the fact that none did) and the process group the engine
//! recorded before the driver was told anything. The host answers the one
//! question a record cannot: is that group gone? Where the records cannot
//! prove an attempt, `brokkr operator settle` writes the journaled record
//! this module reads back by type, and the exclusion honours it as the
//! proof. Acknowledgement alone never releases.
//!
//! Moved out of `settlement.rs`, which this would take past its 800 lines
//! (decision 0071 ruling 4).

use std::collections::BTreeSet;
use std::path::Path;
use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_core::fold::SETTLE;
use brokkr_core::EventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::attestation::reserved_for;
use super::{opened, recorded_settled};
use crate::engine::broker::admission::{amend, Held, Reservation, Reserved, Unadmitted, Witnessed};
use crate::engine::broker::inventory::{Attempt, Budget, Standing, Writing};
use crate::engine::broker::root::Root;
use crate::engine::broker::startup;

#[cfg(test)]
mod tests;

/// How the proof proves one standing attempt ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(in crate::engine) enum Proof {
    /// Every driver it spawned sealed Settled in the reservation, or it
    /// spawned none: #403's settlement proof, read from the broker's own
    /// record and never from the run.
    Records,
    /// Its driver records do not prove it, and the process group the
    /// reservation names is gone from the host. Only the operator's settle
    /// record may rely on this alone.
    Group,
}

/// One attempt's proven end, as the operator's settle record names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::engine) struct Proven {
    pub(in crate::engine) attempt: String,
    pub(in crate::engine) proof: Proof,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(in crate::engine) group: Option<u32>,
}

/// Whether the run had a lock file for its driver to take, as the settle
/// record names it: `free` where the file stood and the lock was free,
/// `none` where a binary before decision 0108 left no file at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(in crate::engine) enum LockState {
    Free,
    None,
}

/// `operator/commanded`'s `args` for command `settle`
/// (contracts/operator-settle.v1.schema.json): the run, the lock the proof
/// found, each attempt proven and how, and the operator's reason. Closed at
/// every level, so a field no reader knows is refused rather than read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::engine) struct SettleArgs {
    pub(in crate::engine) run: String,
    pub(in crate::engine) lock: LockState,
    pub(in crate::engine) attempts: Vec<Proven>,
    pub(in crate::engine) reason: String,
}

/// The last settle record a run carries, by type, never by folding it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::engine) struct Settled {
    pub(in crate::engine) seq: u64,
    pub(in crate::engine) args: SettleArgs,
}

/// Why a standing attempt is not proven ended: its process group still has
/// a member on this host, or nothing proves it and no group was recorded to
/// ask about.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Live {
    #[error(
        "attempt '{attempt}' stands, and its process group {group} still has members on this host"
    )]
    Group { attempt: String, group: u32 },
    #[error("attempt '{attempt}' stands: {why}")]
    Unproven { attempt: String, why: Unproven },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(in crate::engine) enum Unproven {
    #[error(
        "its driver records are not sealed Settled, and it recorded no process group to ask the \
         host about"
    )]
    NoGroup,
    #[error("its cleanup was witnessed unresolved, and it recorded no process group")]
    Unresolved,
    /// The exclusion's proof alone: the broker's own records, never the
    /// host's word, which is the operator's to read with `brokkr operator
    /// settle`.
    #[error(
        "only the host's word proves it: its process group is gone, and no driver record is \
         sealed Settled; `brokkr operator settle` is the operator's route"
    )]
    NoRecords,
    /// A reservation read through the view of another build names no
    /// settled attempt and no process group: nothing here proves it.
    #[error("the record names no settled attempt, and carries no process group to ask about")]
    Unrecorded,
}

/// Whether the process group `group` still has a member on this host. A
/// read that cannot be taken counts as alive: a doubtful host never
/// releases a worktree.
pub(in crate::engine::broker) fn alive(group: u32) -> bool {
    let group = i32::try_from(group)
        .ok()
        .and_then(rustix::process::Pid::from_raw);
    // A group this host cannot name, or one whose signal fails for any
    // reason but "no such process", counts as alive: only the kernel's
    // `ESRCH` proves nothing of the group runs.
    group.is_none_or(|group| {
        rustix::process::test_kill_process_group(group) != Err(rustix::io::Errno::SRCH)
    })
}

/// The proof the broker's records and the host give for the attempt
/// `attempt` of `reserved`: `Group` where only the host proves it, which
/// the operator's settle record may rely on. A live group refuses first,
/// whatever the records say.
pub(in crate::engine::broker) fn proven(reserved: &Reserved, attempt: &str) -> Result<Proof, Live> {
    let records = recorded(reserved);
    let group = reserved.group;
    if let Some(group) = group.filter(|group| alive(*group)) {
        return Err(Live::Group {
            attempt: attempt.to_string(),
            group,
        });
    }
    let why = || match reserved.cleanup {
        Witnessed::Unresolved => Unproven::Unresolved,
        Witnessed::Settling | Witnessed::Settled => Unproven::NoGroup,
    };
    match (records, group) {
        (true, _) => Ok(Proof::Records),
        (false, Some(_)) => Ok(Proof::Group),
        (false, None) => Err(Live::Unproven {
            attempt: attempt.to_string(),
            why: why(),
        }),
    }
}

/// [`proven`] narrowed to what the exclusion may use: the broker's own
/// settlement records, never the host's word alone (decision 0108 ruling 3).
pub(in crate::engine::broker) fn proven_records(
    reserved: &Reserved,
    attempt: &str,
) -> Result<(), Live> {
    match proven(reserved, attempt)? {
        Proof::Records => Ok(()),
        Proof::Group => Err(Live::Unproven {
            attempt: attempt.to_string(),
            why: Unproven::NoRecords,
        }),
    }
}

/// Whether the broker's own records prove every driver of `reserved`
/// settled, or that it spawned none: the record is written at the dispatch
/// door before a driver spawns, so an empty list is a fact, never a gap.
fn recorded(reserved: &Reserved) -> bool {
    let spawned_none = reserved.drivers.is_empty() & (reserved.cleanup != Witnessed::Unresolved);
    spawned_none | recorded_settled(reserved)
}

/// [`proven_records`] for a record another build wrote, read through the
/// view: its attempt's own settlement fact, and no process group to ask the
/// host about. Where that fact is absent, only the operator's record can
/// prove it.
pub(in crate::engine::broker) fn view_records(attempt: &Attempt, id: &str) -> Result<(), Live> {
    let proved = attempt.proved();
    proved.then_some(()).ok_or_else(|| Live::Unproven {
        attempt: id.to_string(),
        why: Unproven::Unrecorded,
    })
}

/// Every attempt `reservation` names standing, proven as `brokkr operator
/// settle` proves it now: the records, or the host's answer that the group
/// it recorded is gone.
pub(in crate::engine) fn provenances(reservation: &Reservation) -> Result<Vec<Proven>, Live> {
    reservation
        .attempts
        .iter()
        .map(|(attempt, reserved)| {
            let proof = proven(reserved, attempt)?;
            Ok(Proven {
                attempt: attempt.clone(),
                proof,
                group: reserved.group,
            })
        })
        .collect()
}

/// [`proven_records`] over every attempt `reservation` names standing.
pub(in crate::engine::broker) fn reservation_records(
    reservation: &Reservation,
) -> Result<(), Live> {
    reservation
        .attempts
        .iter()
        .try_for_each(|(attempt, reserved)| proven_records(reserved, attempt))
}

/// The reservations the run `run` of the journal at `journal` still holds
/// standing, read whole under the admission lock and returned with it held:
/// `operator settle` reads the proof and writes under both locks, in the
/// order admission takes them — the admission lock first, then the run's —
/// so the two never wait on each other in opposite orders (decision 0108
/// ruling 3).
pub(in crate::engine) fn settle_locked(
    root: &Root,
    run: &str,
    journal: &Path,
) -> Result<(Vec<Reservation>, std::os::fd::OwnedFd), Unadmitted> {
    let deadline = startup(Instant::now(), Duration::MAX);
    let lock =
        crate::engine::broker::root::locked(root.dir(), deadline).map_err(Unadmitted::from)?;
    let mut budget = Budget::new(deadline);
    let reservations = reserved_for(root, run, journal, &mut budget)?;
    Ok((reservations, lock))
}

/// Record the process group `group` of the attempt in flight under the
/// lease `held`, under the admission lock, before its driver is told
/// anything. A seal that fails leaves the group unrecorded: a later release
/// falls back to the drivers' own settlement and never guesses (decision
/// 0108 ruling 3).
pub(in crate::engine) fn spawned(held: &Held, group: u32) {
    let deadline = startup(Instant::now(), Duration::MAX);
    let recorded = amend(held, deadline, |mut reservation| {
        let reserved = reservation.attempts.get_mut(&held.attempt).into_iter();
        reserved.for_each(|reserved| reserved.group = Some(group));
        Ok(reservation)
    });
    let _ = recorded;
}

/// The operator's word that a quarantined run has been looked at: the
/// acknowledgement standing beside the chain, and the repair record a
/// repair leaves in it. A repair moves the head past the acknowledgement
/// and so lapses it (#511), which is why the repair record stands on its
/// own here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::engine::broker) struct Looked {
    pub(in crate::engine::broker) acknowledged: bool,
    pub(in crate::engine::broker) repaired: bool,
}

impl Looked {
    /// Either is the operator's word; neither is no record at all.
    pub(in crate::engine::broker) fn known(&self) -> bool {
        self.acknowledged | self.repaired
    }
}

/// The guarded read the release's other proof needs: whether run `run` was
/// looked at, and the last settle record it carries. Read from the events
/// and the acknowledgement table beside the chain, by type, never by
/// folding the run. None where the journal cannot be read as the proof
/// requires: any participant's writer could reach it (R2), or it is not one
/// regular file (L5).
pub(in crate::engine::broker) fn quarantine(
    path: &Path,
    run: &str,
    writers: &[&Writing],
    budget: &Budget,
) -> Option<(Looked, Option<Settled>)> {
    let store = opened(path, writers, budget)?;
    let acknowledged = store.acknowledgement(run).unwrap_or_default().is_some();
    let events = store.load(run).ok();
    let repaired = events.as_deref().is_some_and(repaired);
    let settled = events.as_deref().and_then(record);
    let looked = Looked {
        acknowledged,
        repaired,
    };
    Some((looked, settled))
}

/// Whether `events` carry an operator's repair record after the run's first
/// terminal event: the record that made the run fold again (decision 0108
/// ruling 4), read by type and never by folding. A `repair` standing before
/// the terminal event is the pending command it has always been, no record.
pub(in crate::engine) fn repaired(events: &[EventEnvelope]) -> bool {
    let terminal = events
        .iter()
        .find(|event| brokkr_core::fold::terminal_event(event.event_type).is_some())
        .map(|event| event.seq);
    let Some(terminal) = terminal else {
        return false;
    };
    events.iter().any(|event| {
        let after = event.seq > terminal;
        let commanded = event.event_type == EventType::OperatorCommanded;
        let repair =
            event.payload.get("command").and_then(Value::as_str) == Some(brokkr_core::fold::REPAIR);
        after & commanded & repair
    })
}

/// The last settle record `events` carry, by type: an `operator/commanded`
/// whose command is `settle`, its args the published contract. A malformed
/// record is no record, never an earlier one, so a broken write cannot
/// release a worktree.
pub(in crate::engine) fn record(events: &[EventEnvelope]) -> Option<Settled> {
    events.iter().rev().find_map(|event| {
        let commanded = event.event_type == EventType::OperatorCommanded;
        let settled = event.payload.get("command").and_then(Value::as_str) == Some(SETTLE);
        (commanded & settled)
            .then(|| event.payload.get("args"))
            .flatten()
            .and_then(|args| serde_json::from_value(args.clone()).ok())
            .map(|args| Settled {
                seq: event.seq,
                args,
            })
    })
}

/// Whether the settle record `settled` releases the standing reservation
/// `standing`: it names this very run, covers every attempt the broker
/// still shows standing — never fewer — and names no process group the
/// host still shows alive. A record that a later attempt outran does not
/// release.
pub(in crate::engine::broker) fn honoured(standing: &Standing, settled: &Settled) -> bool {
    let names = &settled.args;
    let ours = names.run == standing.run;
    let recorded: BTreeSet<&str> = names
        .attempts
        .iter()
        .map(|proven| proven.attempt.as_str())
        .collect();
    let covered = standing
        .attempts
        .keys()
        .all(|attempt| recorded.contains(attempt.as_str()));
    let living = names
        .attempts
        .iter()
        .filter_map(|proven| proven.group)
        .any(alive);
    let supported = names.attempts.iter().all(|proven| match proven.proof {
        // Only a group the host shows gone may stand for an attempt the
        // records cannot prove.
        Proof::Group => match &standing.whole {
            // The broker's own record names the group that matters: a
            // record that names another group while the reservation shows
            // this one alive does not release.
            Some(reservation) => reservation
                .attempts
                .get(&proven.attempt)
                .is_none_or(|reserved| {
                    reserved
                        .group
                        .or(proven.group)
                        .is_some_and(|group| !alive(group))
                }),
            None => proven.group.is_some_and(|group| !alive(group)),
        },
        Proof::Records => match &standing.whole {
            Some(reservation) => reservation
                .attempts
                .get(&proven.attempt)
                .is_none_or(|reserved| proven_records(reserved, &proven.attempt).is_ok()),
            None => standing
                .attempts
                .get(&proven.attempt)
                .is_none_or(Attempt::proved),
        },
    });
    ours & covered & !living & supported
}
