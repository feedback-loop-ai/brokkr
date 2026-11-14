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
use super::opened;
use crate::engine::broker::admission::{
    amend, Held, Reservation, Reserved, Unadmitted, Unknown, Witnessed,
};
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
    /// Its driver records do not prove it, and every driver not sealed
    /// Settled has its own process group gone from the host. Only the
    /// operator's settle record may rely on this alone.
    Group,
}

/// One attempt's proven end, as the operator's settle record names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::engine) struct Proven {
    pub(in crate::engine) attempt: String,
    pub(in crate::engine) proof: Proof,
    /// Every process group that proved a driver the records did not: one
    /// per driver not sealed Settled. Empty where the proof is `Records`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(in crate::engine) groups: Vec<u32>,
}

/// `operator/commanded`'s `args` for command `settle`
/// (contracts/operator-settle.v1.schema.json): the run, each attempt proven
/// and how, and the operator's reason. Closed at every level, so a field no
/// reader knows is refused rather than read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::engine) struct SettleArgs {
    pub(in crate::engine) run: String,
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
         host about; repair the run, then `brokkr operator release` is the operator's route where \
         the attempt's journaled ending is indeterminate"
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
/// `attempt` of `reserved`: `Records` when every driver it spawned is
/// sealed Settled (or it spawned none), otherwise `Group` once the host
/// shows every driver not sealed Settled gone. A live group refused first,
/// and a driver with no recorded group refuses by name: the host is asked
/// about *each* driver, never about one group standing in for all of them
/// (decision 0108 ruling 3).
///
/// Only a driver the records do not prove is asked about. A group recorded
/// for a driver sealed Settled is not a writer: the seal is the settlement
/// proof, and the kernel may since have reused that group id for an
/// unrelated process, which no host question could tell apart (F9's
/// correction). Asking it would keep the worktree held with no route.
///
/// An empty driver list is the fact that none spawned only for a record
/// written by #822 or later. A main build that still reported version
/// 0.12.0 could write that shape between #745 and #822, when it spawned
/// without listing; its empty list does not prove none spawned. What keeps
/// that closed is not the release of a version: such a build's boxed
/// reservation has no `sealed.store` (before b8e2caff8, 2026-10-25), so it
/// reads only through the view and its unsettled attempt holds; an
/// unbounded writer's journal is reachable, so the guarded read refuses.
/// Both facts are pinned by
/// `an_old_boxed_reservation_without_store_is_read_through_the_view` and
/// `a_journal_a_writer_could_reach_proves_nothing`; the limit is stated
/// here rather than guessed away (decision 0089 ruling 9).
pub(in crate::engine::broker) fn proven(reserved: &Reserved, attempt: &str) -> Result<Proof, Live> {
    let groups = unsettled_groups(reserved);
    if let Some(group) = groups.iter().flatten().copied().find(|group| alive(*group)) {
        return Err(Live::Group {
            attempt: attempt.to_string(),
            group,
        });
    }
    if reserved.drivers.is_empty() {
        // No driver was listed. For a #822-or-later record that is none
        // spawned; an older record's view or reachable journal refuses
        // before this arm is reached (see above).
        return match reserved.cleanup {
            Witnessed::Unresolved => Err(unproven(attempt, Unproven::Unresolved)),
            Witnessed::Settling | Witnessed::Settled => Ok(Proof::Records),
        };
    }
    let Some(groups) = groups else {
        let why = match reserved.cleanup {
            Witnessed::Unresolved => Unproven::Unresolved,
            Witnessed::Settling | Witnessed::Settled => Unproven::NoGroup,
        };
        return Err(unproven(attempt, why));
    };
    match (groups.is_empty(), reserved.cleanup) {
        (_, Witnessed::Unresolved) => Err(unproven(attempt, Unproven::Unresolved)),
        (true, Witnessed::Settling | Witnessed::Settled) => Ok(Proof::Records),
        (false, Witnessed::Settling | Witnessed::Settled) => Ok(Proof::Group),
    }
}

/// The refusal of attempt `attempt` for `why`.
fn unproven(attempt: &str, why: Unproven) -> Live {
    Live::Unproven {
        attempt: attempt.to_string(),
        why,
    }
}

/// Whether `driver`'s own record seals it Settled.
fn settled(driver: &crate::engine::broker::settlement::Driver) -> bool {
    driver.cleanup == Witnessed::Settled
}

/// The process group recorded for `driver`: its own, or — for a record
/// written in 440d's one-slot shape — the reservation's single group, which
/// may prove only a single-driver attempt (its slot names the last driver,
/// never every one).
fn effective_group(
    reserved: &Reserved,
    driver: &crate::engine::broker::settlement::Driver,
) -> Option<u32> {
    let legacy = match reserved.drivers.as_slice() {
        [_] => reserved.group,
        _ => None,
    };
    driver.group.or(legacy)
}

/// The groups of every driver `reserved` does not seal Settled, in spawn
/// order; none where one of them recorded no group, for no host question
/// can prove that driver. A record with no driver list and 440d's single
/// slot names that slot, the only group it can ask about.
fn unsettled_groups(reserved: &Reserved) -> Option<Vec<u32>> {
    if reserved.drivers.is_empty() {
        return Some(reserved.group.into_iter().collect());
    }
    reserved
        .drivers
        .iter()
        .filter(|driver| !settled(driver))
        .map(|driver| effective_group(reserved, driver))
        .collect()
}

/// Every process group `reserved` records on any of its drivers, one set
/// for the exclusion's "the record names a group the broker recorded" check.
fn recorded_groups(reserved: &Reserved) -> BTreeSet<u32> {
    if reserved.drivers.is_empty() {
        return reserved.group.into_iter().collect();
    }
    reserved
        .drivers
        .iter()
        .filter_map(|driver| effective_group(reserved, driver))
        .collect()
}

/// [`proven`] narrowed to what the exclusion may use: the broker's own
/// settlement records, never the host's word alone (decision 0108 ruling 3).
pub(in crate::engine::broker) fn proven_records(
    reserved: &Reserved,
    attempt: &str,
) -> Result<(), Live> {
    match proven(reserved, attempt)? {
        Proof::Records => Ok(()),
        Proof::Group => Err(unproven(attempt, Unproven::NoRecords)),
    }
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
/// settle` proves it now: the records, or the host's answer about every
/// driver not sealed Settled.
pub(in crate::engine) fn provenances(reservation: &Reservation) -> Result<Vec<Proven>, Live> {
    reservation
        .attempts
        .iter()
        .map(|(attempt, reserved)| {
            let proof = proven(reserved, attempt)?;
            let groups = match proof {
                Proof::Group => unsettled_groups(reserved).unwrap_or_default(),
                Proof::Records => Vec::new(),
            };
            Ok(Proven {
                attempt: attempt.clone(),
                proof,
                groups,
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

/// Record the process group `group` the driver spawned for the site `site`
/// of the attempt in flight under the lease `held`, on that driver's own
/// record, under the admission lock, before the driver is told anything. A
/// seal that fails leaves that driver without a group, and a later release
/// refuses for it rather than standing an earlier driver's dead group in
/// its place (decision 0108 ruling 3).
pub(in crate::engine) fn spawned(held: &Held, site: &str, group: u32) {
    let deadline = startup(Instant::now(), Duration::MAX);
    let recorded = amend(held, deadline, |mut reservation| {
        let reserved = reservation.attempts.get_mut(&held.attempt);
        let driver = reserved
            .ok_or(Unknown::Lost)?
            .drivers
            .iter_mut()
            .rfind(|driver| driver.site == site)
            .ok_or(Unknown::Lost)?;
        driver.group = Some(group);
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

/// The sequence of `events`' first terminal event: the boundary after which
/// a record is an annotation and before which it is the pending command it
/// has always been.
fn terminal_seq(events: &[EventEnvelope]) -> Option<u64> {
    events
        .iter()
        .find(|event| brokkr_core::fold::terminal_event(event.event_type).is_some())
        .map(|event| event.seq)
}

/// Whether `events` carry an operator's repair record after the run's first
/// terminal event: the record that made the run fold again (decision 0108
/// ruling 4), read by type and never by folding. A `repair` standing before
/// the terminal event is the pending command it has always been, no record.
pub(in crate::engine) fn repaired(events: &[EventEnvelope]) -> bool {
    let Some(terminal) = terminal_seq(events) else {
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
/// whose command is `settle`, standing after the run's terminal event, its
/// args the published contract. A pre-terminal word is the pending command,
/// not an annotation, and a malformed record is no record — never an earlier
/// one — so neither can release a worktree (decision 0108, ruling 4's
/// erratum, read through the same [`terminal_seq`] as [`repaired`]).
pub(in crate::engine) fn record(events: &[EventEnvelope]) -> Option<Settled> {
    let terminal = terminal_seq(events)?;
    let last = events.iter().rev().find(|event| {
        let after = event.seq > terminal;
        let commanded = event.event_type == EventType::OperatorCommanded;
        let settled = event.payload.get("command").and_then(Value::as_str) == Some(SETTLE);
        after & commanded & settled
    })?;
    let args = last
        .payload
        .get("args")
        .and_then(|args| serde_json::from_value(args.clone()).ok())?;
    Some(Settled {
        seq: last.seq,
        args,
    })
}

/// Whether `attempt` is one `standing`'s reservation names, standing or
/// already settled: a record that names any other attempt releases nothing.
fn named(standing: &Standing, attempt: &str) -> bool {
    standing.attempts.contains_key(attempt) || standing.settled.contains(attempt)
}

/// Whether the settle record `settled` releases the standing reservation
/// `standing`: it names this very run, names no attempt the reservation does
/// not, covers every attempt the broker still shows standing — never fewer —
/// and names no process group the host still shows alive. A record that a
/// later attempt outran does not release.
pub(in crate::engine::broker) fn honoured(standing: &Standing, settled: &Settled) -> bool {
    let names = &settled.args;
    let ours = names.run == standing.run;
    let known = names
        .attempts
        .iter()
        .all(|proven| named(standing, &proven.attempt));
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
        .flat_map(|proven| &proven.groups)
        .any(|group| alive(*group));
    let supported = names
        .attempts
        .iter()
        .all(|proven| supports(standing, proven));
    ours & known & covered & !living & supported
}

/// Whether `proven`'s proof for its attempt is one `standing`'s record and
/// the host support.
fn supports(standing: &Standing, proven: &Proven) -> bool {
    match proven.proof {
        Proof::Group => group_supports(standing, proven),
        Proof::Records => records_supports(standing, proven),
    }
}

/// A records proof stands only where the broker's own settlement records
/// prove every driver of the attempt settled, or that it spawned none.
fn records_supports(standing: &Standing, proven: &Proven) -> bool {
    match &standing.whole {
        Some(reservation) => reservation
            .attempts
            .get(&proven.attempt)
            .is_none_or(|reserved| proven_records(reserved, &proven.attempt).is_ok()),
        None => standing
            .attempts
            .get(&proven.attempt)
            .is_none_or(Attempt::proved),
    }
}

/// A group proof stands only where the reservation records every group it
/// names, and it names every group of a driver the records do not prove
/// settled: a dead group cannot stand for a sibling the record left out.
fn group_supports(standing: &Standing, proven: &Proven) -> bool {
    let Some(reservation) = &standing.whole else {
        return !proven.groups.is_empty() && proven.groups.iter().all(|group| !alive(*group));
    };
    let Some(reserved) = reservation.attempts.get(&proven.attempt) else {
        return false;
    };
    let Some(expected) = unsettled_groups(reserved) else {
        return false;
    };
    let recorded = recorded_groups(reserved);
    let named: BTreeSet<u32> = proven.groups.iter().copied().collect();
    named.iter().all(|group| recorded.contains(group))
        && expected.iter().all(|group| named.contains(group))
}
