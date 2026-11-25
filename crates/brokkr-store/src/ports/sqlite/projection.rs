//! SQLite projection maintenance, decoupled prefix-hash verification, and
//! rebuild logic for [`super::SqlitePorts`] (decision 0126, Slice 1).

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use brokkr_core::summary::{Account, Summary};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::summary::Listed;
use crate::{event_index, schema, seat_summary, summary, StoreError};

use super::{
    checked_seq, divergent, has_local_runs_or_events, load_verified_local, query_strings,
    verify_chain_for_sqlite,
};

const INSERT_EVENT_INDEX: &str = "INSERT OR IGNORE INTO event_index
         (run_id, seq, type, attempt_id, member, recorded_at, event_id, stream_ordinal)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";
const INSERT_PROJECTION_HASH: &str = "INSERT OR IGNORE INTO projection_hashes
         (run_id, seq, event_hash)
     VALUES (?1, ?2, ?3)";
const PROJECTION_HASHES_DDL: &str = "CREATE TABLE IF NOT EXISTS projection_hashes (
    run_id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    event_hash TEXT NOT NULL,
    PRIMARY KEY (run_id, seq)
);";
const PROJECTION_TABLES: [&str; 6] = [
    "run_summary",
    "seat_summary",
    "seat_head",
    "seat_lag",
    "event_index",
    "projection_hashes",
];
pub(super) const EXISTING_PROJECTED_RUNS: &str = "SELECT run_id FROM run_summary
     UNION SELECT DISTINCT run_id FROM seat_summary
     UNION SELECT run_id FROM seat_head
     UNION SELECT run_id FROM seat_lag
     UNION SELECT DISTINCT run_id FROM event_index
     UNION SELECT DISTINCT run_id FROM projection_hashes";
const LOCAL_RUNS: &str = "SELECT run_id FROM runs
     UNION SELECT DISTINCT run_id FROM events
     ORDER BY run_id";
const DECOUPLED_PREFIX_HASHES: &str = "SELECT seq, event_hash FROM projection_hashes
     WHERE run_id = ?1 AND seq <= ?2 ORDER BY seq";
const PROJECTION_HASH_AT_SEQ: &str =
    "SELECT event_hash FROM projection_hashes WHERE run_id = ?1 AND seq = ?2";

pub(super) struct VerifiedRun {
    pub(super) run_id: String,
    pub(super) events: Vec<EventEnvelope>,
    pub(super) account: Account,
    pub(super) listed: Listed,
}

pub(super) enum ProjectionPlan {
    Skip,
    Colocated(Vec<EventEnvelope>),
    Decoupled(Vec<EventEnvelope>),
}

enum DecoupledAction {
    Skip,
    Project,
}

/// Does this journal carry the `projection_hashes` table? A pure read of
/// `sqlite_master` so steady-state opens take no write lock.
pub(crate) fn projection_hashes_intact(conn: &Connection) -> Result<bool, StoreError> {
    schema::all_present(conn, "table", &["projection_hashes"])
}

/// Create the `projection_hashes` table idempotently.
pub(crate) fn migrate_projection_hashes(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(PROJECTION_HASHES_DDL)?;
    Ok(())
}

type OrdinalStreams<'a> = BTreeMap<(&'a str, Option<&'a str>), (Option<&'a str>, i64)>;

fn checkpoint_ordinal<'a>(
    envelope: &'a EventEnvelope,
    indexed: Indexed<'a>,
    streams: &mut OrdinalStreams<'a>,
) -> Option<i64> {
    if indexed.event_type != EventType::EffectCheckpointed {
        return None;
    }
    let checkpoint_member = envelope
        .payload
        .get("checkpoint")
        .and_then(|value| value.get("member"))
        .and_then(Value::as_str);
    let Some(attempt_id) = indexed
        .attempt_id
        .filter(|_| checkpoint_member == indexed.member)
    else {
        return Some(0);
    };
    let effect_id = envelope.payload.get("effect_id").and_then(Value::as_str);
    let (last_effect, ordinal) = streams
        .entry((attempt_id, indexed.member))
        .or_insert((effect_id, 0));
    if *last_effect != effect_id {
        *last_effect = effect_id;
        *ordinal = 1;
    } else {
        *ordinal += 1;
    }
    Some(*ordinal)
}

fn insert_event_index_rows(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut insert = tx.prepare_cached(INSERT_EVENT_INDEX)?;
    let mut streams = BTreeMap::new();
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?;
        let indexed = Indexed::of(envelope);
        let ordinal = checkpoint_ordinal(envelope, indexed, &mut streams);
        insert.execute(params![
            run_id,
            seq,
            indexed.event_type.as_str(),
            indexed.attempt_id,
            indexed.member,
            indexed.recorded_at,
            envelope.event_id,
            ordinal,
        ])?;
    }
    Ok(())
}

fn insert_projection_hashes(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut insert = tx.prepare_cached(INSERT_PROJECTION_HASH)?;
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?;
        insert.execute(params![run_id, seq, envelope.event_hash])?;
    }
    Ok(())
}

fn listed_for_projection(
    tx: &Connection,
    run_id: &str,
    fallback: &Listed,
) -> Result<Listed, StoreError> {
    let row: Option<(String, String)> = tx
        .query_row(
            "SELECT feature, created_at FROM runs WHERE run_id = ?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if let Some((feature, created_at)) = row {
        return Ok(Listed::new(&feature, &created_at));
    }
    if fallback.listing().feature.is_empty() {
        let existing: Option<String> = tx
            .query_row(
                "SELECT feature FROM run_summary
                 WHERE run_id = ?1 AND feature IS NOT NULL AND feature != ''",
                params![run_id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(feature) = existing {
            return Ok(Listed::new(&feature, fallback.listing().created_at));
        }
    }
    Ok(fallback.clone())
}

pub(super) fn write_full_projection(
    tx: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
    account: &Account,
    fallback_listed: &Listed,
) -> Result<(), StoreError> {
    let listed = listed_for_projection(tx, run_id, fallback_listed)?;
    summary::upsert(tx, run_id, &Summary::of(account, listed.listing()))?;
    seat_summary::write(tx, account, None)?;
    insert_event_index_rows(tx, run_id, events)?;
    event_index::write_parks(tx, account)?;
    event_index::write_all_checkpoints(tx, account)?;
    Ok(())
}

pub(super) fn write_decoupled_projection(
    tx: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
    account: &Account,
    fallback_listed: &Listed,
) -> Result<(), StoreError> {
    write_full_projection(tx, run_id, events, account, fallback_listed)?;
    insert_projection_hashes(tx, run_id, events)?;
    Ok(())
}

fn clear_projections(tx: &Connection) -> Result<(), StoreError> {
    for table in PROJECTION_TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }
    Ok(())
}

fn clear_run_projections(tx: &Connection, run_id: &str) -> Result<(), StoreError> {
    for table in PROJECTION_TABLES {
        let sql = format!("DELETE FROM {table} WHERE run_id = ?1");
        tx.execute(&sql, params![run_id])?;
    }
    Ok(())
}

fn clear_stale_projections(
    tx: &Connection,
    pre_existing: &[String],
    keep: &BTreeSet<&str>,
) -> Result<(), StoreError> {
    for run_id in pre_existing {
        if !keep.contains(run_id.as_str()) {
            clear_run_projections(tx, run_id)?;
        }
    }
    Ok(())
}

fn summary_progress(
    tx: &Connection,
    run_id: &str,
    min_head: i64,
) -> Result<Option<(i64, Option<i64>, String)>, StoreError> {
    Ok(tx
        .query_row(
            "SELECT head_seq, folded_seq, head_hash
             FROM run_summary WHERE run_id = ?1 AND head_seq >= ?2",
            params![run_id, min_head],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?)
}

fn is_colocated_projection_current(
    tx: &Connection,
    run_id: &str,
    (expected_seq, expected_hash): (i64, &str),
) -> Result<bool, StoreError> {
    let progress = summary_progress(tx, run_id, 0)?;
    Ok(progress.is_some_and(|(head_seq, folded_seq, head_hash)| {
        folded_seq == Some(head_seq) && head_seq == expected_seq && head_hash == expected_hash
    }))
}

fn verify_decoupled_prefix(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let incoming_max = checked_seq(envelopes.len() as u64)?;
    let stored: Vec<(i64, String)> = tx
        .prepare(DECOUPLED_PREFIX_HASHES)?
        .query_map(params![run_id, incoming_max], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let incoming: Vec<(i64, String)> = envelopes
        .iter()
        .map(|envelope| (envelope.seq as i64, envelope.event_hash.clone()))
        .collect();
    if stored != incoming {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn verify_projection_hash(
    tx: &Connection,
    run_id: &str,
    seq: i64,
    expected_hash: &str,
) -> Result<(), StoreError> {
    let stored: Option<String> = tx
        .query_row(PROJECTION_HASH_AT_SEQ, params![run_id, seq], |row| {
            row.get(0)
        })
        .optional()?;
    if stored.as_deref() != Some(expected_hash) {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn plan_decoupled_apply(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<DecoupledAction, StoreError> {
    let Some((existing_head_seq, existing_folded_seq, existing_head_hash)) =
        summary_progress(tx, run_id, 1)?
    else {
        return Ok(DecoupledAction::Project);
    };
    let incoming_max = checked_seq(envelopes.len() as u64)?;
    if incoming_max < existing_head_seq {
        verify_decoupled_prefix(tx, run_id, envelopes)?;
        return Ok(DecoupledAction::Skip);
    }
    let at_head = &envelopes[(existing_head_seq - 1) as usize];
    if at_head.event_hash != existing_head_hash {
        return Err(divergent(run_id));
    }
    if incoming_max == existing_head_seq && existing_folded_seq == Some(existing_head_seq) {
        return Ok(DecoupledAction::Skip);
    }
    Ok(DecoupledAction::Project)
}

pub(super) fn resolve_projection_plan(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<ProjectionPlan, StoreError> {
    if has_local_runs_or_events(tx)? {
        let local = load_verified_local(tx, run_id)?;
        let matched: Vec<EventEnvelope> = local
            .iter()
            .skip_while(|event| event.seq < envelopes[0].seq)
            .take(envelopes.len())
            .cloned()
            .collect();
        if serde_json::to_vec(&matched)? != serde_json::to_vec(envelopes)? {
            return Err(divergent(run_id));
        }
        let head = local.last().expect("matched non-empty batch");
        let target = (checked_seq(head.seq)?, head.event_hash.as_str());
        if is_colocated_projection_current(tx, run_id, target)? {
            return Ok(ProjectionPlan::Skip);
        }
        Ok(ProjectionPlan::Colocated(local))
    } else {
        verify_chain_for_sqlite(run_id, (0, ZERO_HASH), envelopes)?;
        match plan_decoupled_apply(tx, run_id, envelopes)? {
            DecoupledAction::Skip => Ok(ProjectionPlan::Skip),
            DecoupledAction::Project => Ok(ProjectionPlan::Decoupled(envelopes.to_vec())),
        }
    }
}

pub(super) fn verify_local_events_match(
    tx: &Connection,
    verified: &[VerifiedRun],
) -> Result<bool, StoreError> {
    let local_runs = query_strings(tx, LOCAL_RUNS)?;
    if local_runs.is_empty() {
        return Ok(false);
    }
    let mut expected: BTreeMap<&str, &[EventEnvelope]> = verified
        .iter()
        .map(|run| (run.run_id.as_str(), run.events.as_slice()))
        .collect();
    for run_id in local_runs {
        let local = load_verified_local(tx, &run_id)?;
        let Some(external) = expected.remove(run_id.as_str()) else {
            return Err(StoreError::DivergentEventLog { run_id });
        };
        if serde_json::to_vec(&local)? != serde_json::to_vec(external)? {
            return Err(StoreError::DivergentEventLog { run_id });
        }
    }
    if let Some((extra, _)) = expected.into_iter().next() {
        return Err(divergent(extra));
    }
    Ok(true)
}

pub(super) fn rebuild_colocated(
    tx: &Connection,
    verified: &[VerifiedRun],
) -> Result<u64, StoreError> {
    clear_projections(tx)?;
    let mut total = 0u64;
    for run in verified {
        write_full_projection(tx, &run.run_id, &run.events, &run.account, &run.listed)?;
        total += run.events.len() as u64;
    }
    Ok(total)
}

pub(super) fn rebuild_decoupled(
    tx: &Connection,
    verified: &[VerifiedRun],
    pre_existing: &[String],
) -> Result<u64, StoreError> {
    let keep: BTreeSet<&str> = verified
        .iter()
        .filter(|run| !run.events.is_empty())
        .map(|run| run.run_id.as_str())
        .collect();
    clear_stale_projections(tx, pre_existing, &keep)?;
    let mut total = 0u64;
    for run in verified {
        let Some(last) = run.events.last() else {
            continue;
        };
        let scanned_head = checked_seq(last.seq)?;
        let current: Option<(i64, Option<i64>)> = tx
            .query_row(
                "SELECT head_seq, folded_seq FROM run_summary WHERE run_id = ?1",
                params![run.run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if current.is_some_and(|(head, folded)| head > scanned_head && folded == Some(head)) {
            verify_projection_hash(tx, &run.run_id, scanned_head, &last.event_hash)?;
            continue;
        }
        clear_run_projections(tx, &run.run_id)?;
        write_decoupled_projection(tx, &run.run_id, &run.events, &run.account, &run.listed)?;
        total += run.events.len() as u64;
    }
    Ok(total)
}
