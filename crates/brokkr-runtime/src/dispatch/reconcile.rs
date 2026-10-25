//! The dispatcher's reconciliation (decision 0095 ruling 3), under its
//! lock and before its pass: what a dispatcher left mid-start, because it
//! died, was fenced out or saw a link or a spawn `Contended` past its
//! patience, is settled by the store's steps under this dispatcher's
//! generation, each fenced and journaled.
//!
//! An entry left launching whose claim a run's manifest carries is linked
//! to that run, adopted: never relaunched, never stopped and never
//! concluded (decision 0068 ruling 7). One whose claim no run carries goes
//! back to its place by an uncounted `reclaim`, no start having been
//! refused, and its next claim is a new one. An entry linked with no
//! `spawn` row, adopted now or before, started no driver, so it is to be
//! spawned; one with a `spawn` row is settled and left alone, its run
//! reading `Running` for the operator to resume should its driver have
//! died. Spawning is the `brokkr dispatch` loop's, never this.

use std::num::NonZeroU32;

use brokkr_store::{Claim, Store, StoreError};

use super::Dispatcher;

/// An entry reconciliation settled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// Back in its place: no run carried its claim, which the `reclaim`
    /// ended.
    Reclaimed(Claim),
    /// Linked to this run, which started no driver: adopted now, or linked
    /// before with no `spawn` row. Its driver is the dispatcher's to spawn,
    /// under the claim.
    ToSpawn(Claim, String),
}

/// The `link` row's reason for a run reconciliation adopts.
fn adopted(claim: Claim, run: &str) -> String {
    let (entry, seq) = claim.parts();
    format!(
        "reconciled: run '{run}' carries claim {seq} of entry {entry}, so it is adopted, never \
         relaunched, stopped or concluded (decision 0068 ruling 7)"
    )
}

/// The `reclaim` row's reason for a claim no run carries.
fn reclaimed(claim: Claim) -> String {
    let (entry, seq) = claim.parts();
    format!(
        "reconciled: no run carries claim {seq} of entry {entry}, so the claim ends and the \
         entry returns to its place"
    )
}

/// Reconcile what the journal's dispatchers left unsettled, as
/// `dispatcher`, before its pass: link each entry left launching to the run
/// that carries its claim, or reclaim it where none does, and say each
/// entry settled, in the order of the entries, those left launching first.
/// Refused, writing nothing, unless the dispatcher's generation is the
/// journal's latest holder; a refusal of one step leaves every step before
/// it written.
pub fn reconcile(store: &mut Store, dispatcher: &Dispatcher) -> Result<Vec<Settled>, StoreError> {
    let generation = dispatcher.generation;
    let (launching, linked) = store.queue_unsettled(generation)?;
    let mut settled = Vec::with_capacity(launching.len() + linked.len());
    for (claim, carrier) in launching {
        settled.push(match carrier {
            Some(run) => {
                store.queue_link(claim, generation, &run, &adopted(claim, &run))?;
                Settled::ToSpawn(claim, run)
            }
            // Uncounted, so no bound applies to it.
            None => {
                let reason = reclaimed(claim);
                store.queue_reclaim(claim, generation, None, NonZeroU32::MAX, &reason)?;
                Settled::Reclaimed(claim)
            }
        });
    }
    let to_spawn = linked.into_iter();
    settled.extend(to_spawn.map(|(claim, run)| Settled::ToSpawn(claim, run)));
    Ok(settled)
}

#[cfg(test)]
mod tests;
