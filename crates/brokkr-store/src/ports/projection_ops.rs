//! Shared projection listing, ordering, and batch-reconciliation helpers used by
//! both [`super::SqliteProjectionStore`] and external [`super::ProjectionStore`]
//! backends such as `brokkr-store-pg` (decision 0071 ruling 5).

use std::collections::BTreeSet;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use serde_json::Value;

use crate::StoreError;

use super::{
    divergent, non_empty_str, started_feature_of, validate_manifest, validate_run_id,
    validate_started_manifests, verify_chain_checked, RunMetadata, VerifiedRun,
};

/// Snapshot of a projected run row loaded inside a projection transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredProjectionRun {
    pub feature: String,
    pub created_at: String,
    pub head_seq: u64,
    pub head_hash: String,
}

/// Outcome of reconciling an incoming envelope batch against the stored
/// projection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionPlan<'a> {
    /// Run has no events yet (`head_seq == 0`); upsert empty run metadata.
    UpsertEmpty {
        feature: &'a str,
        created_at: &'a str,
    },
    /// Stored projection is already at or ahead of the batch; only fill missing
    /// manifest metadata when present.
    NoAdvance,
    /// Batch advanced the projection to `(head_seq, head_hash)`; upsert the
    /// projected run header after events have been inserted.
    Advance {
        feature: &'a str,
        created_at: &'a str,
        head_seq: u64,
        head_hash: &'a str,
    },
}

/// Sort verified runs in deterministic projection-rebuild order (`list_order`,
/// then `run_id`).
/// Compute `(feature, created_at, head_seq, head_hash)` for a verified run
/// during a projection rebuild.
pub fn rebuild_run_header(run: &VerifiedRun) -> (&str, &str, u64, &str) {
    let (feature, created_at) = resolve_projection_listing(
        &run.events,
        Some(&run.manifest),
        None,
        run.metadata.as_ref(),
    );
    let (head_seq, head_hash) = run
        .events
        .last()
        .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
    (feature, created_at, head_seq, head_hash)
}

pub fn sort_verified_runs(runs: &mut [VerifiedRun]) {
    runs.sort_by(|a, b| {
        let order_a = a.metadata.as_ref().and_then(|m| m.list_order);
        let order_b = b.metadata.as_ref().and_then(|m| m.list_order);
        order_a.cmp(&order_b).then_with(|| a.run_id.cmp(&b.run_id))
    });
}

/// Return the subset of `envelopes` whose sequence numbers are absent from
/// `present`, preserving sequence order.
pub fn missing_projected_events(
    envelopes: &[EventEnvelope],
    present: &BTreeSet<u64>,
) -> Vec<EventEnvelope> {
    envelopes
        .iter()
        .filter(|env| !present.contains(&env.seq))
        .cloned()
        .collect()
}

/// Validate `run_id`, optional `manifest`, hash-chain continuity from seq 1,
/// and any embedded `RunStarted` manifests before opening a projection write
/// transaction.
pub fn validate_projection_apply(
    run_id: &str,
    manifest: Option<&Value>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    validate_run_id(run_id)?;
    if let Some(m) = manifest {
        validate_manifest(run_id, m)?;
    }
    verify_chain_checked(run_id, (0, ZERO_HASH), envelopes)?;
    validate_started_manifests(run_id, envelopes)?;
    Ok(())
}

/// Resolve a run's `(feature, created_at)` listing metadata using the single
/// projection precedence rule shared across SQLite and PostgreSQL backends:
/// once `envelopes` is non-empty, `RunStarted` > `manifest` > existing started
/// `feature` and `envelopes[0].recorded_at` always win; `fallback` and existing
/// unstarted `created_at` are only used when `envelopes` is empty.
pub fn resolve_projection_listing<'a>(
    envelopes: &'a [EventEnvelope],
    manifest: Option<&'a Value>,
    existing: Option<&'a StoredProjectionRun>,
    fallback: Option<&'a RunMetadata>,
) -> (&'a str, &'a str) {
    let manifest_feature = manifest.and_then(|m| non_empty_str(m.get("feature")));
    if let Some(first) = envelopes.first() {
        let existing_started_feature = existing
            .filter(|r| r.head_seq > 0)
            .map(|r| r.feature.as_str())
            .filter(|s| !s.is_empty());
        let feature = started_feature_of(envelopes)
            .or(manifest_feature)
            .or(existing_started_feature)
            .unwrap_or("");
        return (feature, first.recorded_at.as_str());
    }
    let existing_feature = existing
        .map(|r| r.feature.as_str())
        .filter(|s| !s.is_empty());
    let existing_created_at = existing
        .map(|r| r.created_at.as_str())
        .filter(|s| !s.is_empty());
    let feature = fallback
        .map(|m| m.feature.as_str())
        .or(existing_feature)
        .or(manifest_feature)
        .unwrap_or("");
    let created_at = fallback
        .map(|m| m.created_at.as_str())
        .or(existing_created_at)
        .or_else(|| manifest.and_then(|m| non_empty_str(m.get("created_at"))))
        .unwrap_or("");
    (feature, created_at)
}

/// Reconcile a validated `envelopes` batch against `existing`, verifying stored
/// heads and prefixes, repairing gaps on equal or advancing batches, inserting
/// new suffix events, and returning the [`ProjectionPlan`] for the caller's run
/// header update.
/// Callback verifying the stored head row inside a projection transaction.
pub type HeadChecker<Ctx> = fn(&mut Ctx, &str, &StoredProjectionRun) -> Result<(), StoreError>;
/// Callback verifying the stored prefix rows and returning present sequence numbers.
pub type PrefixVerifier<Ctx> =
    fn(&mut Ctx, &str, &[EventEnvelope]) -> Result<BTreeSet<u64>, StoreError>;
/// Callback inserting missing or new projected event rows.
pub type EventsInserter<Ctx> = fn(&mut Ctx, &str, &[EventEnvelope]) -> Result<(), StoreError>;

/// Backend callbacks invoked inside a projection transaction by
/// [`reconcile_projection_batch`].
pub struct ProjectionBatchOps<Ctx> {
    pub check_head: HeadChecker<Ctx>,
    pub verify_prefix: PrefixVerifier<Ctx>,
    pub insert_events: EventsInserter<Ctx>,
}

pub fn reconcile_projection_batch<'a, Ctx>(
    ctx: &mut Ctx,
    run_id: &str,
    manifest: Option<&'a Value>,
    envelopes: &'a [EventEnvelope],
    existing: Option<&'a StoredProjectionRun>,
    ops: ProjectionBatchOps<Ctx>,
) -> Result<ProjectionPlan<'a>, StoreError> {
    if let Some(row) = existing {
        if row.head_seq == 0 && row.head_hash != ZERO_HASH {
            return Err(divergent(run_id));
        }
    }
    let (feature, created_at) = resolve_projection_listing(envelopes, manifest, existing, None);
    let Some(last) = envelopes.last() else {
        if let Some(row) = existing.filter(|r| r.head_seq > 0) {
            (ops.check_head)(ctx, run_id, row)?;
            return Ok(ProjectionPlan::NoAdvance);
        }
        return Ok(ProjectionPlan::UpsertEmpty {
            feature,
            created_at,
        });
    };
    reconcile_non_empty(
        ctx,
        run_id,
        envelopes,
        last,
        existing,
        (feature, created_at),
        ops,
    )
}

fn verify_and_repair_prefix<Ctx>(
    ctx: &mut Ctx,
    run_id: &str,
    prefix: &[EventEnvelope],
    ops: &ProjectionBatchOps<Ctx>,
) -> Result<(), StoreError> {
    let present = (ops.verify_prefix)(ctx, run_id, prefix)?;
    let missing = missing_projected_events(prefix, &present);
    if !missing.is_empty() {
        (ops.insert_events)(ctx, run_id, &missing)?;
    }
    Ok(())
}

fn reconcile_non_empty<'a, Ctx>(
    ctx: &mut Ctx,
    run_id: &str,
    envelopes: &'a [EventEnvelope],
    last: &'a EventEnvelope,
    existing: Option<&'a StoredProjectionRun>,
    listing: (&'a str, &'a str),
    ops: ProjectionBatchOps<Ctx>,
) -> Result<ProjectionPlan<'a>, StoreError> {
    let (feature, created_at) = listing;
    let Some(row) = existing.filter(|r| r.head_seq > 0) else {
        (ops.insert_events)(ctx, run_id, envelopes)?;
        return Ok(ProjectionPlan::Advance {
            feature,
            created_at,
            head_seq: last.seq,
            head_hash: &last.event_hash,
        });
    };
    if row.head_seq > last.seq {
        (ops.check_head)(ctx, run_id, row)?;
        let present = (ops.verify_prefix)(ctx, run_id, envelopes)?;
        if present.len() != envelopes.len() {
            return Err(divergent(run_id));
        }
        return Ok(ProjectionPlan::NoAdvance);
    }
    if row.head_seq == last.seq {
        if row.head_hash != last.event_hash {
            return Err(divergent(run_id));
        }
        verify_and_repair_prefix(ctx, run_id, envelopes, &ops)?;
        return Ok(ProjectionPlan::NoAdvance);
    }
    let split = row.head_seq as usize;
    let prefix = &envelopes[..split];
    if prefix[split - 1].event_hash != row.head_hash {
        return Err(divergent(run_id));
    }
    verify_and_repair_prefix(ctx, run_id, prefix, &ops)?;
    (ops.insert_events)(ctx, run_id, &envelopes[split..])?;
    Ok(ProjectionPlan::Advance {
        feature,
        created_at,
        head_seq: last.seq,
        head_hash: &last.event_hash,
    })
}
