//! PostgreSQL implementation of [`brokkr_store::ports::EventLog`] (decision 0126 ruling 5).

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope};
use brokkr_store::ports::{
    checked_seq, manifest_meta, started_feature_of, validate_append_batch, validate_import_stream,
    validate_manifest, validate_run_id, verify_chain_checked, RunMetadata,
};
use brokkr_store::StoreError;
use serde_json::Value;

use crate::error::{PgError, SQLSTATE_UNIQUE_VIOLATION};
use crate::schema::{decode_pg_text, encode_pg_text, jsonb_param};
use crate::wire::{PgClient, PgRow};

const INSERT_RUN_SQL: &str =
    "INSERT INTO brokkr_runs (run_id, manifest, manifest_json, feature, created_at)
     VALUES ($1, $2::jsonb, $3, $4, $5)";
const LOCK_RUN_MANIFEST_SQL: &str =
    "SELECT manifest_json FROM brokkr_runs WHERE run_id = $1 FOR UPDATE";
const SELECT_RUN_MANIFEST_SQL: &str = "SELECT manifest_json FROM brokkr_runs WHERE run_id = $1";
const SELECT_RUN_META_SQL: &str =
    "SELECT feature, created_at, list_order FROM brokkr_runs WHERE run_id = $1";
const SELECT_RUN_EXISTS_SQL: &str = "SELECT 1 FROM brokkr_runs WHERE run_id = $1";
const SELECT_EVENTS_SQL: &str =
    "SELECT seq, event_hash, envelope_json FROM brokkr_events WHERE run_id = $1 ORDER BY seq ASC";
const INSERT_EVENT_SQL: &str = "INSERT INTO brokkr_events
        (run_id, seq, event_type, event_hash, prev_hash, recorded_at, envelope, envelope_json)
     VALUES ($1, $2::bigint, $3, $4, $5, $6, $7::jsonb, $8)";
const UPDATE_STARTED_META_SQL: &str = "UPDATE brokkr_runs
     SET created_at = $2,
         feature = CASE WHEN $3 <> '' THEN $3 ELSE feature END
     WHERE run_id = $1";
const LIST_RUN_IDS_SQL: &str = "SELECT run_id FROM brokkr_runs ORDER BY run_id ASC";

pub(crate) fn create_run(
    client: &mut PgClient,
    run_id: &str,
    manifest: &Value,
) -> Result<(), StoreError> {
    validate_run_id(run_id)?;
    validate_manifest(run_id, manifest)?;
    let (feature, _) = manifest_meta(manifest);
    let created_at = manifest
        .get("created_at")
        .and_then(Value::as_str)
        .unwrap_or("");
    insert_run_row(client, run_id, manifest, feature, created_at)
}

pub(crate) fn append_envelopes(
    client: &mut PgClient,
    run_id: &str,
    expected_head: (u64, &str),
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    client.in_transaction(
        |tx| {
            let manifest = read_manifest_query(tx, LOCK_RUN_MANIFEST_SQL, run_id)?;
            let existing = load_verified_events(tx, run_id)?;
            validate_append_batch(run_id, &manifest, &existing, expected_head, envelopes)?;
            insert_events(tx, run_id, envelopes)?;
            if envelopes.first().is_some_and(|e| e.seq == 1) {
                let feature = encode_pg_text(started_feature_of(envelopes).unwrap_or(""));
                let created_at = encode_pg_text(&envelopes[0].recorded_at);
                tx.query(
                    UPDATE_STARTED_META_SQL,
                    &[
                        Some(run_id),
                        Some(created_at.as_ref()),
                        Some(feature.as_ref()),
                    ],
                )
                .map_err(PgError::into_store_error)?;
            }
            Ok(())
        },
        PgError::into_store_error,
    )
}

pub(crate) fn import_run_stream(
    client: &mut PgClient,
    run_id: &str,
    manifest: &Value,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    validate_import_stream(run_id, manifest, envelopes)?;
    let (manifest_feature, _) = manifest_meta(manifest);
    let feature = started_feature_of(envelopes).unwrap_or(manifest_feature);
    let created_at = envelopes[0].recorded_at.as_str();
    client.in_transaction(
        |tx| {
            insert_run_row(tx, run_id, manifest, feature, created_at)?;
            insert_events(tx, run_id, envelopes)
        },
        PgError::into_store_error,
    )
}

pub(crate) fn head(client: &mut PgClient, run_id: &str) -> Result<(u64, String), StoreError> {
    client.in_transaction(
        |tx| {
            ensure_run_exists(tx, run_id)?;
            let events = load_verified_events(tx, run_id)?;
            Ok(events.last().map_or((0, ZERO_HASH.to_string()), |last| {
                (last.seq, last.event_hash.clone())
            }))
        },
        PgError::into_store_error,
    )
}

pub(crate) fn read_stream(
    client: &mut PgClient,
    run_id: &str,
    from_seq: u64,
) -> Result<Vec<EventEnvelope>, StoreError> {
    client.in_transaction(
        |tx| {
            ensure_run_exists(tx, run_id)?;
            let events = load_verified_events(tx, run_id)?;
            Ok(events
                .into_iter()
                .filter(|event| event.seq >= from_seq)
                .collect())
        },
        PgError::into_store_error,
    )
}

pub(crate) fn manifest(client: &mut PgClient, run_id: &str) -> Result<Value, StoreError> {
    read_manifest_query(client, SELECT_RUN_MANIFEST_SQL, run_id)
}

pub(crate) fn run_metadata(
    client: &mut PgClient,
    run_id: &str,
) -> Result<Option<RunMetadata>, StoreError> {
    let row = query_run_row(client, SELECT_RUN_META_SQL, run_id)?;
    let feature = decode_pg_text(row.get_str(0).map_err(PgError::into_store_error)?)
        .map_err(PgError::into_store_error)?;
    let created_at = decode_pg_text(row.get_str(1).map_err(PgError::into_store_error)?)
        .map_err(PgError::into_store_error)?;
    let list_order = u64::try_from(row.get_i64(2).map_err(PgError::into_store_error)?).ok();
    Ok(Some(RunMetadata {
        feature,
        created_at,
        list_order,
    }))
}

pub(crate) fn list_run_ids(client: &mut PgClient) -> Result<Vec<String>, StoreError> {
    let res = client
        .query(LIST_RUN_IDS_SQL, &[])
        .map_err(PgError::into_store_error)?;
    let mut ids = Vec::with_capacity(res.rows.len());
    for row in &res.rows {
        ids.push(
            row.get_str(0)
                .map_err(PgError::into_store_error)?
                .to_string(),
        );
    }
    Ok(ids)
}

fn map_unique_run_error(run_id: &str, err: PgError) -> StoreError {
    if err.sqlstate() == Some(SQLSTATE_UNIQUE_VIOLATION) {
        StoreError::RunExists(run_id.to_string())
    } else {
        err.into_store_error()
    }
}

fn ensure_run_exists(tx: &mut PgClient, run_id: &str) -> Result<(), StoreError> {
    let res = tx
        .query(SELECT_RUN_EXISTS_SQL, &[Some(run_id)])
        .map_err(PgError::into_store_error)?;
    if res.rows.is_empty() {
        return Err(StoreError::RunNotFound(run_id.to_string()));
    }
    Ok(())
}

fn insert_run_row(
    client: &mut PgClient,
    run_id: &str,
    manifest: &Value,
    feature: &str,
    created_at: &str,
) -> Result<(), StoreError> {
    let manifest_json = serde_json::to_string(manifest)?;
    let manifest_jsonb = jsonb_param(manifest, &manifest_json);
    let feature_enc = encode_pg_text(feature);
    let created_enc = encode_pg_text(created_at);
    client
        .query(
            INSERT_RUN_SQL,
            &[
                Some(run_id),
                manifest_jsonb,
                Some(manifest_json.as_str()),
                Some(feature_enc.as_ref()),
                Some(created_enc.as_ref()),
            ],
        )
        .map_err(|err| map_unique_run_error(run_id, err))?;
    Ok(())
}

fn query_run_row(client: &mut PgClient, sql: &str, run_id: &str) -> Result<PgRow, StoreError> {
    let res = client
        .query(sql, &[Some(run_id)])
        .map_err(PgError::into_store_error)?;
    res.rows
        .into_iter()
        .next()
        .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))
}

fn read_manifest_query(
    client: &mut PgClient,
    sql: &str,
    run_id: &str,
) -> Result<Value, StoreError> {
    let row = query_run_row(client, sql, run_id)?;
    let raw = row.get_str(0).map_err(PgError::into_store_error)?;
    let manifest: Value = serde_json::from_str(raw)?;
    validate_manifest(run_id, &manifest)?;
    Ok(manifest)
}

fn load_verified_events(tx: &mut PgClient, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
    let res = tx
        .query(SELECT_EVENTS_SQL, &[Some(run_id)])
        .map_err(PgError::into_store_error)?;
    let mut events = Vec::with_capacity(res.rows.len());
    for row in &res.rows {
        let sql_seq = row.get_i64(0).map_err(PgError::into_store_error)?;
        if sql_seq < 1 {
            return Err(StoreError::RowBeforeChain {
                run_id: run_id.to_string(),
                seq: sql_seq,
            });
        }
        let sql_hash = row.get_str(1).map_err(PgError::into_store_error)?;
        let raw_env = row.get_str(2).map_err(PgError::into_store_error)?;
        let envelope: EventEnvelope =
            serde_json::from_str(raw_env).map_err(|_| StoreError::Unparsed {
                run_id: run_id.to_string(),
                seq: sql_seq,
            })?;
        if sql_seq as u64 != envelope.seq {
            return Err(StoreError::Chain(ChainError::SeqGap {
                seq: envelope.seq,
                expected: sql_seq as u64,
            }));
        }
        if sql_hash != envelope.event_hash {
            return Err(StoreError::Chain(ChainError::BadHash { seq: envelope.seq }));
        }
        events.push(envelope);
    }
    verify_chain_checked(run_id, (0, ZERO_HASH), &events)?;
    Ok(events)
}

fn insert_events(
    tx: &mut PgClient,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?;
        let seq_str = seq.to_string();
        let envelope_json = serde_json::to_string(envelope)?;
        let envelope_value = serde_json::to_value(envelope)?;
        let envelope_jsonb = jsonb_param(&envelope_value, &envelope_json);
        let recorded_enc = encode_pg_text(&envelope.recorded_at);
        tx.query(
            INSERT_EVENT_SQL,
            &[
                Some(run_id),
                Some(seq_str.as_str()),
                Some(envelope.event_type.as_str()),
                Some(envelope.event_hash.as_str()),
                Some(envelope.previous_hash.as_str()),
                Some(recorded_enc.as_ref()),
                envelope_jsonb,
                Some(envelope_json.as_str()),
            ],
        )
        .map_err(|err| {
            if err.sqlstate() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                StoreError::AppendConflict { seq: envelope.seq }
            } else {
                err.into_store_error()
            }
        })?;
    }
    Ok(())
}
