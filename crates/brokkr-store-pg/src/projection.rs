//! PostgreSQL implementation of [`brokkr_store::ports::ProjectionStore`] (decision 0126 ruling 5).

use std::collections::BTreeSet;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::EventEnvelope;
use brokkr_core::event_index::Indexed;
use brokkr_store::ports::{
    checked_seq, manifest_meta, missing_manifest_feature, read_verified_log, rebuild_run_header,
    reconcile_projection_batch, sort_verified_runs, validate_projection_apply, EventLog,
    ProjectionBatchOps, ProjectionPlan, StoredProjectionRun,
};
use brokkr_store::StoreError;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::PgError;
use crate::schema::{decode_pg_text, encode_pg_text, jsonb_param};
use crate::wire::PgClient;

const LOCK_PROJECTED_RUN_ADVISORY_SQL: &str = "SELECT pg_advisory_xact_lock($1::bigint)";
const LOCK_PROJECTED_RUN_SQL: &str = "SELECT feature, bundle_name, created_at, head_seq, head_hash
     FROM brokkr_projected_runs WHERE run_id = $1 FOR UPDATE";
const SELECT_OUT_OF_RANGE_SQL: &str =
    "SELECT COUNT(*) FROM brokkr_projected_events WHERE run_id = $1 AND (seq < 1 OR seq > $2::bigint)";
const SELECT_IN_RANGE_COUNT_SQL: &str =
    "SELECT COUNT(*) FROM brokkr_projected_events WHERE run_id = $1 AND seq >= 1 AND seq <= $2::bigint";
const SELECT_EVENT_HASH_AT_SQL: &str =
    "SELECT event_hash FROM brokkr_projected_events WHERE run_id = $1 AND seq = $2::bigint";
const SELECT_PREFIX_ROWS_SQL: &str =
    "SELECT seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload_json
     FROM brokkr_projected_events
     WHERE run_id = $1 AND seq <= $2::bigint
     ORDER BY seq ASC";
const INSERT_PROJECTED_EVENT_SQL: &str = "INSERT INTO brokkr_projected_events
        (run_id, seq, event_id, event_type, attempt_id, member, event_hash, recorded_at, payload, payload_json)
     VALUES ($1, $2::bigint, $3, $4, $5, $6, $7, $8, $9::jsonb, $10)";
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
    "SELECT run_id, feature, created_at FROM brokkr_projected_runs";
const LOCK_PROJECTION_TABLES_SQL: &str =
    "LOCK TABLE brokkr_projected_runs, brokkr_projected_events IN EXCLUSIVE MODE";
const DELETE_PROJECTED_EVENTS_SQL: &str = "DELETE FROM brokkr_projected_events";
const DELETE_PROJECTED_RUNS_SQL: &str = "DELETE FROM brokkr_projected_runs";

struct StoredRun {
    head: StoredProjectionRun,
    bundle_name: String,
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
    validate_projection_apply(run_id, manifest, envelopes)?;
    client.in_transaction(
        |tx| apply_batch_tx(tx, run_id, manifest, envelopes),
        PgError::into_store_error,
    )
}

/// List projected runs in deterministic `(created_at, run_id)` UTF-8 byte order
/// after decoding escaped NUL/backslash text. Equal `created_at` timestamps
/// tie-break by `run_id` byte order so incremental `apply_envelopes` and
/// `rebuild_from` produce identical ordering regardless of insertion sequence.
pub(crate) fn list_runs(
    client: &mut PgClient,
) -> Result<Vec<(String, String, String)>, StoreError> {
    let res = client
        .query(LIST_PROJECTED_RUNS_SQL, &[])
        .map_err(PgError::into_store_error)?;
    let mut rows = Vec::with_capacity(res.rows.len());
    for row in &res.rows {
        let run_id = row
            .get_str(0)
            .map_err(PgError::into_store_error)?
            .to_string();
        let feature = decode_pg_text(row.get_str(1).map_err(PgError::into_store_error)?)
            .map_err(PgError::into_store_error)?;
        let created_at = decode_pg_text(row.get_str(2).map_err(PgError::into_store_error)?)
            .map_err(PgError::into_store_error)?;
        rows.push((created_at, run_id, feature));
    }
    rows.sort();
    Ok(rows
        .into_iter()
        .map(|(created_at, run_id, feature)| (run_id, feature, created_at))
        .collect())
}

pub(crate) fn rebuild_from(client: &mut PgClient, log: &dyn EventLog) -> Result<u64, StoreError> {
    client.in_transaction(
        |tx| {
            tx.query(LOCK_PROJECTION_TABLES_SQL, &[])
                .map_err(PgError::into_store_error)?;
            let mut verified = read_verified_log(log)?;
            sort_verified_runs(&mut verified);
            tx.query(DELETE_PROJECTED_EVENTS_SQL, &[])
                .map_err(PgError::into_store_error)?;
            tx.query(DELETE_PROJECTED_RUNS_SQL, &[])
                .map_err(PgError::into_store_error)?;
            let mut total = 0u64;
            for run in &verified {
                let (feature, created_at, head_seq, head_hash) = rebuild_run_header(run);
                let (_, bundle_name) = manifest_meta(&run.manifest);
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

fn projection_run_lock_key(run_id: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(b"brokkr-proj-run:");
    hasher.update(run_id.as_bytes());
    let digest = hasher.finalize();
    i64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

fn apply_batch_tx(
    tx: &mut PgClient,
    run_id: &str,
    manifest: Option<&Value>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let lock_key = projection_run_lock_key(run_id).to_string();
    tx.query(LOCK_PROJECTED_RUN_ADVISORY_SQL, &[Some(&lock_key)])
        .map_err(PgError::into_store_error)?;
    let existing = load_stored_run(tx, run_id)?;
    let stored_head_seq = existing.as_ref().map_or(0, |row| row.head.head_seq);
    refuse_out_of_range_events(tx, run_id, stored_head_seq)?;
    let bundle_name = resolve_bundle_name(manifest, existing.as_ref());
    let plan = reconcile_projection_batch(
        tx,
        run_id,
        manifest,
        envelopes,
        existing.as_ref().map(|row| &row.head),
        ProjectionBatchOps {
            check_head: |c, id, row| {
                verify_stored_event_count(c, id, row.head_seq)?;
                verify_stored_head_event_hash(c, id, row)
            },
            verify_prefix: verify_stored_prefix,
            insert_events: insert_projected_events,
        },
    )?;
    match plan {
        ProjectionPlan::UpsertEmpty {
            feature,
            created_at,
        } => {
            let meta = ProjectedRunMeta {
                feature,
                bundle_name,
                created_at,
                head_seq: 0,
                head_hash: ZERO_HASH,
            };
            upsert_projected_run(tx, run_id, &meta)
        }
        ProjectionPlan::NoAdvance => {
            // `reconcile_projection_batch` only returns `NoAdvance` when `existing`
            // is `Some` with `head_seq > 0` (verified by
            // `ports_tests::projection_monotonicity_repair_divergence_and_rebuild`).
            let row = existing.as_ref().expect("NoAdvance requires stored run");
            maybe_fill_manifest_meta(tx, run_id, row, manifest)
        }
        ProjectionPlan::Advance {
            feature,
            created_at,
            head_seq,
            head_hash,
        } => {
            let meta = ProjectedRunMeta {
                feature,
                bundle_name,
                created_at,
                head_seq,
                head_hash,
            };
            upsert_projected_run(tx, run_id, &meta)
        }
    }
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
        .map(|row| row.bundle_name.as_str())
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
        head: StoredProjectionRun {
            feature: decode_pg_text(row.get_str(0).map_err(PgError::into_store_error)?)
                .map_err(PgError::into_store_error)?,
            created_at: decode_pg_text(row.get_str(2).map_err(PgError::into_store_error)?)
                .map_err(PgError::into_store_error)?,
            head_seq,
            head_hash: row
                .get_str(4)
                .map_err(PgError::into_store_error)?
                .to_string(),
        },
        bundle_name: decode_pg_text(row.get_str(1).map_err(PgError::into_store_error)?)
            .map_err(PgError::into_store_error)?,
    }))
}

fn query_event_count(
    tx: &mut PgClient,
    sql: &str,
    run_id: &str,
    limit: i64,
) -> Result<i64, StoreError> {
    let limit_str = limit.to_string();
    let res = tx
        .query(sql, &[Some(run_id), Some(&limit_str)])
        .map_err(PgError::into_store_error)?;
    // SQL `SELECT COUNT(*)` always returns one row; verified by `ports_tests`.
    res.rows[0].get_i64(0).map_err(PgError::into_store_error)
}

fn refuse_out_of_range_events(
    tx: &mut PgClient,
    run_id: &str,
    head_seq: u64,
) -> Result<(), StoreError> {
    let limit = checked_seq(head_seq)?;
    if query_event_count(tx, SELECT_OUT_OF_RANGE_SQL, run_id, limit)? > 0 {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_stored_event_count(
    tx: &mut PgClient,
    run_id: &str,
    head_seq: u64,
) -> Result<(), StoreError> {
    let expected = checked_seq(head_seq)?;
    if query_event_count(tx, SELECT_IN_RANGE_COUNT_SQL, run_id, expected)? != expected {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_stored_head_event_hash(
    tx: &mut PgClient,
    run_id: &str,
    row: &StoredProjectionRun,
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
        let expected_payload = serde_json::to_string(&expected.payload)?;
        let stored = (
            row.get_str(1).map_err(PgError::into_store_error)?,
            row.get_str(2).map_err(PgError::into_store_error)?,
            row.get_opt_str(3).map_err(PgError::into_store_error)?,
            row.get_opt_str(4).map_err(PgError::into_store_error)?,
            row.get_str(5).map_err(PgError::into_store_error)?,
            row.get_str(6).map_err(PgError::into_store_error)?,
            row.get_str(7).map_err(PgError::into_store_error)?,
        );
        let exp_event_id = encode_pg_text(&expected.event_id);
        let exp_attempt = indexed.attempt_id.map(encode_pg_text);
        let exp_member = indexed.member.map(encode_pg_text);
        let exp_recorded = encode_pg_text(&expected.recorded_at);
        let want = (
            exp_event_id.as_ref(),
            expected.event_type.as_str(),
            exp_attempt.as_deref(),
            exp_member.as_deref(),
            expected.event_hash.as_str(),
            exp_recorded.as_ref(),
            expected_payload.as_str(),
        );
        if stored != want {
            return Err(divergent(run_id));
        }
        present.insert(seq as u64);
    }
    Ok(present)
}

fn maybe_fill_manifest_meta(
    tx: &mut PgClient,
    run_id: &str,
    existing: &StoredRun,
    manifest: Option<&Value>,
) -> Result<(), StoreError> {
    let feature = missing_manifest_feature(&existing.head, manifest)
        .unwrap_or(existing.head.feature.as_str());
    let bundle_name = if existing.bundle_name.is_empty() {
        manifest.map(manifest_meta).map_or("", |(_, b)| b)
    } else {
        existing.bundle_name.as_str()
    };
    if (feature, bundle_name)
        == (
            existing.head.feature.as_str(),
            existing.bundle_name.as_str(),
        )
    {
        return Ok(());
    }
    let meta = ProjectedRunMeta {
        feature,
        bundle_name,
        created_at: &existing.head.created_at,
        head_seq: existing.head.head_seq,
        head_hash: &existing.head.head_hash,
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
        let payload_json = serde_json::to_string(&envelope.payload)?;
        let payload_jsonb = jsonb_param(&envelope.payload, &payload_json);
        let event_id = encode_pg_text(&envelope.event_id);
        let attempt_id = indexed.attempt_id.map(encode_pg_text);
        let member = indexed.member.map(encode_pg_text);
        let recorded_at = encode_pg_text(&envelope.recorded_at);
        tx.query(
            INSERT_PROJECTED_EVENT_SQL,
            &[
                Some(run_id),
                Some(seq.as_str()),
                Some(event_id.as_ref()),
                Some(envelope.event_type.as_str()),
                attempt_id.as_deref(),
                member.as_deref(),
                Some(envelope.event_hash.as_str()),
                Some(recorded_at.as_ref()),
                payload_jsonb,
                Some(payload_json.as_str()),
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
    let feature = encode_pg_text(meta.feature);
    let bundle_name = encode_pg_text(meta.bundle_name);
    let created_at = encode_pg_text(meta.created_at);
    tx.query(
        UPSERT_PROJECTED_RUN_SQL,
        &[
            Some(run_id),
            Some(feature.as_ref()),
            Some(bundle_name.as_ref()),
            Some(created_at.as_ref()),
            Some(seq.as_str()),
            Some(meta.head_hash),
        ],
    )
    .map_err(PgError::into_store_error)?;
    Ok(())
}
