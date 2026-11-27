//! PostgreSQL implementation of [`brokkr_store::ports::ProjectionStore`] (decision 0126 ruling 5).

use std::collections::BTreeSet;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::event_index::Indexed;
use brokkr_store::ports::{
    checked_seq, manifest_meta, validate_manifest, validate_run_id, validate_started_manifests,
    verify_chain_checked, SqliteProjectionStore, VerifiedRun,
};
use brokkr_store::StoreError;
use serde_json::Value;

use crate::error::PgError;
use crate::wire::PgClient;

const LOCK_PROJECTED_RUN_SQL: &str = "SELECT feature, bundle_name, created_at, head_seq, head_hash
     FROM brokkr_projected_runs WHERE run_id = $1 FOR UPDATE";
const SELECT_OUT_OF_RANGE_SQL: &str =
    "SELECT COUNT(*) FROM brokkr_projected_events WHERE run_id = $1 AND (seq < 1 OR seq > $2::bigint)";
const SELECT_EVENT_HASH_AT_SQL: &str =
    "SELECT event_hash FROM brokkr_projected_events WHERE run_id = $1 AND seq = $2::bigint";
const SELECT_PREFIX_ROWS_SQL: &str =
    "SELECT seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload::text
     FROM brokkr_projected_events
     WHERE run_id = $1 AND seq <= $2::bigint
     ORDER BY seq ASC";
const INSERT_PROJECTED_EVENT_SQL: &str = "INSERT INTO brokkr_projected_events
        (run_id, seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload)
     VALUES ($1, $2::bigint, $3, $4, $5, $6, $7, $8, $9::jsonb)";
const UPSERT_PROJECTED_RUN_SQL: &str =
    "INSERT INTO brokkr_projected_runs (run_id, feature, bundle_name, created_at, head_seq, head_hash)
     VALUES ($1, $2, $3, $4, $5::bigint, $6)
     ON CONFLICT (run_id) DO UPDATE SET
         feature = EXCLUDED.feature,
         bundle_name = EXCLUDED.bundle_name,
         created_at = EXCLUDED.created_at,
         head_seq = EXCLUDED.head_seq,
         head_hash = EXCLUDED.head_hash";
const LIST_PROJECTED_RUNS_SQL: &str =
    "SELECT run_id, feature, created_at FROM brokkr_projected_runs ORDER BY created_at ASC, run_id ASC";
const DELETE_PROJECTED_EVENTS_SQL: &str = "DELETE FROM brokkr_projected_events";
const DELETE_PROJECTED_RUNS_SQL: &str = "DELETE FROM brokkr_projected_runs";

struct StoredRun {
    feature: String,
    bundle_name: String,
    created_at: String,
    head_seq: u64,
    head_hash: String,
}

struct ProjectedRunMeta<'a> {
    feature: &'a str,
    bundle_name: &'a str,
    created_at: &'a str,
    head_seq: u64,
    head_hash: &'a str,
}

fn divergent(run_id: &str) -> StoreError {
    StoreError::DivergentEventLog {
        run_id: run_id.to_string(),
    }
}

pub(crate) fn apply_envelopes(
    client: &mut PgClient,
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
    client.in_transaction(
        |tx| apply_batch_tx(tx, run_id, manifest, envelopes),
        PgError::into_store_error,
    )
}

pub(crate) fn list_runs(
    client: &mut PgClient,
) -> Result<Vec<(String, String, String)>, StoreError> {
    let res = client
        .query(LIST_PROJECTED_RUNS_SQL, &[])
        .map_err(PgError::into_store_error)?;
    let mut out = Vec::with_capacity(res.rows.len());
    for row in &res.rows {
        let run_id = row
            .get_str(0)
            .map_err(PgError::into_store_error)?
            .to_string();
        let feature = row
            .get_str(1)
            .map_err(PgError::into_store_error)?
            .to_string();
        let created_at = row
            .get_str(2)
            .map_err(PgError::into_store_error)?
            .to_string();
        out.push((run_id, feature, created_at));
    }
    Ok(out)
}

pub(crate) fn rebuild_from_verified(
    client: &mut PgClient,
    mut verified: Vec<VerifiedRun>,
) -> Result<u64, StoreError> {
    verified.sort_by(|a, b| {
        let order_a = a.metadata.as_ref().and_then(|m| m.list_order);
        let order_b = b.metadata.as_ref().and_then(|m| m.list_order);
        order_a.cmp(&order_b).then_with(|| a.run_id.cmp(&b.run_id))
    });
    client.in_transaction(
        |tx| {
            tx.query(DELETE_PROJECTED_EVENTS_SQL, &[])
                .map_err(PgError::into_store_error)?;
            tx.query(DELETE_PROJECTED_RUNS_SQL, &[])
                .map_err(PgError::into_store_error)?;
            let mut total = 0u64;
            for run in &verified {
                let (feature, created_at) = SqliteProjectionStore::resolve_listing(
                    &run.events,
                    Some(&run.manifest),
                    run.metadata.as_ref(),
                );
                let (_, bundle_name) = manifest_meta(&run.manifest);
                let (head_seq, head_hash) = run
                    .events
                    .last()
                    .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
                let meta = ProjectedRunMeta {
                    feature,
                    bundle_name,
                    created_at,
                    head_seq,
                    head_hash,
                };
                upsert_projected_run(tx, &run.run_id, &meta)?;
                insert_projected_events(tx, &run.run_id, &run.events)?;
                total += run.events.len() as u64;
            }
            Ok(total)
        },
        PgError::into_store_error,
    )
}

fn apply_batch_tx(
    tx: &mut PgClient,
    run_id: &str,
    manifest: Option<&Value>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let existing = load_stored_run(tx, run_id)?;
    let stored_head_seq = existing.as_ref().map_or(0, |row| row.head_seq);
    refuse_out_of_range_events(tx, run_id, stored_head_seq)?;
    if let Some(row) = &existing {
        if row.head_seq == 0 && row.head_hash != ZERO_HASH {
            return Err(divergent(run_id));
        }
    }
    let (feature, created_at) = resolve_batch_listing(envelopes, manifest, existing.as_ref());
    let bundle_name = resolve_bundle_name(manifest, existing.as_ref());
    let Some(last) = envelopes.last() else {
        if let Some(row) = existing.as_ref().filter(|r| r.head_seq > 0) {
            verify_stored_head_event_hash(tx, run_id, row)?;
            maybe_fill_manifest_meta(tx, run_id, row, manifest)?;
            return Ok(());
        }
        let meta = ProjectedRunMeta {
            feature,
            bundle_name,
            created_at,
            head_seq: 0,
            head_hash: ZERO_HASH,
        };
        return upsert_projected_run(tx, run_id, &meta);
    };
    let append_from = match reconcile_existing_run(tx, run_id, existing.as_ref(), envelopes, last)?
    {
        Reconcile::AppendFrom(idx) => idx,
        Reconcile::NoAdvance(row) => {
            maybe_fill_manifest_meta(tx, run_id, row, manifest)?;
            return Ok(());
        }
    };
    insert_projected_events(tx, run_id, &envelopes[append_from..])?;
    let meta = ProjectedRunMeta {
        feature,
        bundle_name,
        created_at,
        head_seq: last.seq,
        head_hash: &last.event_hash,
    };
    upsert_projected_run(tx, run_id, &meta)
}

fn resolve_batch_listing<'a>(
    envelopes: &'a [EventEnvelope],
    manifest: Option<&'a Value>,
    existing: Option<&'a StoredRun>,
) -> (&'a str, &'a str) {
    let (mut feature, mut created_at) =
        SqliteProjectionStore::resolve_listing(envelopes, manifest, None);
    if let Some(row) = existing {
        if feature.is_empty() && (envelopes.is_empty() || row.head_seq > 0) {
            feature = row.feature.as_str();
        }
        if envelopes.is_empty() && created_at.is_empty() {
            created_at = row.created_at.as_str();
        }
    }
    (feature, created_at)
}

fn resolve_bundle_name<'a>(
    manifest: Option<&'a Value>,
    existing: Option<&'a StoredRun>,
) -> &'a str {
    let from_manifest = manifest
        .map(manifest_meta)
        .map(|(_, bundle)| bundle)
        .filter(|s| !s.is_empty());
    let from_existing = existing
        .map(|r| r.bundle_name.as_str())
        .filter(|s| !s.is_empty());
    from_manifest.or(from_existing).unwrap_or("")
}

fn load_stored_run(tx: &mut PgClient, run_id: &str) -> Result<Option<StoredRun>, StoreError> {
    let res = tx
        .query(LOCK_PROJECTED_RUN_SQL, &[Some(run_id)])
        .map_err(PgError::into_store_error)?;
    let Some(row) = res.rows.first() else {
        return Ok(None);
    };
    let head_seq_raw = row.get_i64(3).map_err(PgError::into_store_error)?;
    let head_seq = u64::try_from(head_seq_raw).map_err(|_| divergent(run_id))?;
    Ok(Some(StoredRun {
        feature: row
            .get_str(0)
            .map_err(PgError::into_store_error)?
            .to_string(),
        bundle_name: row
            .get_str(1)
            .map_err(PgError::into_store_error)?
            .to_string(),
        created_at: row
            .get_str(2)
            .map_err(PgError::into_store_error)?
            .to_string(),
        head_seq,
        head_hash: row
            .get_str(4)
            .map_err(PgError::into_store_error)?
            .to_string(),
    }))
}

fn refuse_out_of_range_events(
    tx: &mut PgClient,
    run_id: &str,
    head_seq: u64,
) -> Result<(), StoreError> {
    let limit = checked_seq(head_seq)?.to_string();
    let res = tx
        .query(SELECT_OUT_OF_RANGE_SQL, &[Some(run_id), Some(&limit)])
        .map_err(PgError::into_store_error)?;
    // SQL `SELECT COUNT(*)` always returns one row; verified by `ports_tests`.
    let count = res.rows[0].get_i64(0).map_err(PgError::into_store_error)?;
    if count > 0 {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_stored_head_event_hash(
    tx: &mut PgClient,
    run_id: &str,
    row: &StoredRun,
) -> Result<(), StoreError> {
    let seq = checked_seq(row.head_seq)?.to_string();
    let res = tx
        .query(SELECT_EVENT_HASH_AT_SQL, &[Some(run_id), Some(&seq)])
        .map_err(PgError::into_store_error)?;
    let stored = res
        .rows
        .first()
        .map(|r| r.get_str(0))
        .transpose()
        .map_err(PgError::into_store_error)?;
    if stored != Some(row.head_hash.as_str()) {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_stored_prefix(
    tx: &mut PgClient,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<BTreeSet<u64>, StoreError> {
    let limit = checked_seq(envelopes.len() as u64)?.to_string();
    let res = tx
        .query(SELECT_PREFIX_ROWS_SQL, &[Some(run_id), Some(&limit)])
        .map_err(PgError::into_store_error)?;
    let mut present = BTreeSet::new();
    for row in &res.rows {
        let seq = row.get_i64(0).map_err(PgError::into_store_error)?;
        // `refuse_out_of_range_events` already refused any `seq < 1` or `seq > head_seq`
        // (tested by `ports_tests::projection_monotonicity_repair_divergence_and_rebuild`).
        let idx = (seq - 1) as usize;
        let expected = &envelopes[idx];
        let indexed = Indexed::of(expected);
        let stored_payload: Value =
            serde_json::from_str(row.get_str(7).map_err(PgError::into_store_error)?)
                .map_err(|_| divergent(run_id))?;
        let stored = (
            row.get_str(1).map_err(PgError::into_store_error)?,
            row.get_str(2).map_err(PgError::into_store_error)?,
            row.get_opt_str(3).map_err(PgError::into_store_error)?,
            row.get_opt_str(4).map_err(PgError::into_store_error)?,
            row.get_str(5).map_err(PgError::into_store_error)?,
            row.get_str(6).map_err(PgError::into_store_error)?,
            &stored_payload,
        );
        let want = (
            expected.event_id.as_str(),
            expected.event_type.as_str(),
            indexed.attempt_id,
            indexed.member,
            expected.event_hash.as_str(),
            expected.recorded_at.as_str(),
            &expected.payload,
        );
        if stored != want {
            return Err(divergent(run_id));
        }
        present.insert(seq as u64);
    }
    Ok(present)
}

enum Reconcile<'a> {
    AppendFrom(usize),
    NoAdvance(&'a StoredRun),
}

fn reconcile_existing_run<'a>(
    tx: &mut PgClient,
    run_id: &str,
    existing: Option<&'a StoredRun>,
    envelopes: &[EventEnvelope],
    last: &EventEnvelope,
) -> Result<Reconcile<'a>, StoreError> {
    let Some(row) = existing else {
        return Ok(Reconcile::AppendFrom(0));
    };
    if row.head_seq == 0 {
        return Ok(Reconcile::AppendFrom(0));
    }
    if row.head_seq > last.seq {
        verify_stored_head_event_hash(tx, run_id, row)?;
        let present = verify_stored_prefix(tx, run_id, envelopes)?;
        if present.len() != envelopes.len() {
            return Err(divergent(run_id));
        }
        return Ok(Reconcile::NoAdvance(row));
    }
    if row.head_seq == last.seq {
        if row.head_hash != last.event_hash {
            return Err(divergent(run_id));
        }
        let present = verify_stored_prefix(tx, run_id, envelopes)?;
        repair_missing_events(tx, run_id, envelopes, &present)?;
        return Ok(Reconcile::NoAdvance(row));
    }
    let prefix = &envelopes[..row.head_seq as usize];
    if prefix[(row.head_seq - 1) as usize].event_hash != row.head_hash {
        return Err(divergent(run_id));
    }
    let present = verify_stored_prefix(tx, run_id, prefix)?;
    repair_missing_events(tx, run_id, prefix, &present)?;
    Ok(Reconcile::AppendFrom(row.head_seq as usize))
}

fn repair_missing_events(
    tx: &mut PgClient,
    run_id: &str,
    envelopes: &[EventEnvelope],
    present: &BTreeSet<u64>,
) -> Result<(), StoreError> {
    let missing: Vec<EventEnvelope> = envelopes
        .iter()
        .filter(|env| !present.contains(&env.seq))
        .cloned()
        .collect();
    if !missing.is_empty() {
        insert_projected_events(tx, run_id, &missing)?;
    }
    Ok(())
}

fn maybe_fill_manifest_meta(
    tx: &mut PgClient,
    run_id: &str,
    existing: &StoredRun,
    manifest: Option<&Value>,
) -> Result<(), StoreError> {
    let Some(m) = manifest else {
        return Ok(());
    };
    let (mf, mb) = manifest_meta(m);
    let feature = if existing.feature.is_empty() {
        mf
    } else {
        existing.feature.as_str()
    };
    let bundle_name = if existing.bundle_name.is_empty() {
        mb
    } else {
        existing.bundle_name.as_str()
    };
    if (feature, bundle_name) == (existing.feature.as_str(), existing.bundle_name.as_str()) {
        return Ok(());
    }
    let meta = ProjectedRunMeta {
        feature,
        bundle_name,
        created_at: &existing.created_at,
        head_seq: existing.head_seq,
        head_hash: &existing.head_hash,
    };
    upsert_projected_run(tx, run_id, &meta)
}

fn insert_projected_events(
    tx: &mut PgClient,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?.to_string();
        let indexed = Indexed::of(envelope);
        let payload = serde_json::to_string(&envelope.payload)?;
        tx.query(
            INSERT_PROJECTED_EVENT_SQL,
            &[
                Some(run_id),
                Some(seq.as_str()),
                Some(envelope.event_id.as_str()),
                Some(envelope.event_type.as_str()),
                indexed.attempt_id,
                indexed.member,
                Some(envelope.event_hash.as_str()),
                Some(envelope.recorded_at.as_str()),
                Some(payload.as_str()),
            ],
        )
        .map_err(PgError::into_store_error)?;
    }
    Ok(())
}

fn upsert_projected_run(
    tx: &mut PgClient,
    run_id: &str,
    meta: &ProjectedRunMeta<'_>,
) -> Result<(), StoreError> {
    let seq = checked_seq(meta.head_seq)?.to_string();
    tx.query(
        UPSERT_PROJECTED_RUN_SQL,
        &[
            Some(run_id),
            Some(meta.feature),
            Some(meta.bundle_name),
            Some(meta.created_at),
            Some(seq.as_str()),
            Some(meta.head_hash),
        ],
    )
    .map_err(PgError::into_store_error)?;
    Ok(())
}
