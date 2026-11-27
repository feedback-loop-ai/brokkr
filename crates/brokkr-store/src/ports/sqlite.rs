//! Co-located SQLite [`SqlitePorts`] implementations of [`EventLog`],
//! [`ProjectionStore`], and [`CoordinationStore`] over [`Store`] (decision
//! 0126, Slice 1).
//!
//! Wrapped behind [`SqlitePorts`] rather than implemented directly on
//! [`Store`] so importing `brokkr_store::ports::*` never shadows inherent
//! [`Store`] methods (`create_run`, `list_runs`, `manifest`).
//!
//! `SqlitePorts` is strictly co-located with its wrapped [`Store`]: its
//! projection methods verify that inputs match the local `runs`/`events`
//! tables and refresh [`Store`]'s own read-model tables cleanly. Dedicated
//! decoupled SQLite projections (`index.db`) belong to Slice 2 (decision 0126
//! ruling 4).

use std::collections::BTreeMap;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, ChainError, EventEnvelope, EventType};
use brokkr_core::summary::{Account, Summary};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;

use crate::event_index::ordinal::OrdinalState;
use crate::event_index::{IndexFault, IndexReading};
use crate::start::Row;
use crate::summary::Listed;
use crate::{
    damage, event_index, head_of, import, origin, patiently, queue, refuse_existing, seat_summary,
    start, summary, Attribution, Cooldown, EntryId, NewEntry, QueueEntry, Store, StoreError,
};

use super::{
    checked_seq, divergent, manifest_meta, parse_clock, read_verified_log, started_feature,
    started_feature_of, validate_append_batch, validate_import_stream, validate_manifest,
    CoordinationStore, EventLog, ProjectionStore, VerifiedRun,
};

/// Thin co-located adapter exposing a SQLite [`Store`] through the
/// [`EventLog`], [`ProjectionStore`], and [`CoordinationStore`] traits without
/// shadowing [`Store`]'s inherent methods.
pub struct SqlitePorts<'a>(&'a Store);

impl<'a> SqlitePorts<'a> {
    /// Wrap a [`Store`] reference as a co-located port implementation.
    pub fn new(store: &'a Store) -> Self {
        Self(store)
    }
}

const UPDATE_STARTED_META: &str = "UPDATE runs
     SET created_at = ?2,
         feature = CASE WHEN ?3 != '' THEN ?3 ELSE feature END
     WHERE run_id = ?1";
const LOCAL_RUNS: &str = "SELECT run_id FROM runs
     UNION SELECT DISTINCT run_id FROM events
     ORDER BY run_id";
const PROJECTION_TABLES: [&str; 5] = [
    "run_summary",
    "seat_summary",
    "seat_head",
    "seat_lag",
    "event_index",
];

fn has_run(conn: &Connection, run_id: &str) -> Result<bool, StoreError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM runs WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

fn ensure_run_exists(conn: &Connection, run_id: &str) -> Result<(), StoreError> {
    if has_run(conn, run_id)? {
        Ok(())
    } else {
        Err(StoreError::RunNotFound(run_id.to_string()))
    }
}

fn query_strings(conn: &Connection, sql: &str) -> Result<Vec<String>, StoreError> {
    Ok(conn
        .prepare(sql)?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?)
}

fn refuse_row_before_chain(conn: &Connection, run_id: &str) -> Result<(), StoreError> {
    let lowest: Option<i64> = conn
        .query_row(
            "SELECT MIN(seq) FROM events WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    if let Some(seq) = lowest.filter(|seq| *seq < 1) {
        return Err(StoreError::RowBeforeChain {
            run_id: run_id.to_string(),
            seq,
        });
    }
    Ok(())
}

fn read_local_event(r: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, String, String)> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
}

fn read_verified_rows(
    conn: &Connection,
    run_id: &str,
    after: Option<i64>,
    on_mismatch: &dyn Fn(ChainError) -> StoreError,
) -> Result<Vec<EventEnvelope>, StoreError> {
    const ALL: &str = "SELECT seq, event_hash, envelope FROM events WHERE run_id = ?1 ORDER BY seq";
    const AFTER: &str =
        "SELECT seq, event_hash, envelope FROM events WHERE run_id = ?1 AND seq > ?2 ORDER BY seq";
    let rows = match after {
        None => conn
            .prepare(ALL)?
            .query_map(params![run_id], read_local_event)?
            .collect::<Result<Vec<_>, _>>()?,
        Some(seq) => conn
            .prepare(AFTER)?
            .query_map(params![run_id, seq], read_local_event)?
            .collect::<Result<Vec<_>, _>>()?,
    };
    let mut events = Vec::with_capacity(rows.len());
    for (sql_seq, sql_hash, raw) in rows {
        let envelope = damage::envelope(run_id, sql_seq, &raw)?;
        if sql_seq < 1 {
            return Err(StoreError::RowBeforeChain {
                run_id: run_id.to_string(),
                seq: sql_seq,
            });
        }
        if sql_seq as u64 != envelope.seq {
            return Err(on_mismatch(ChainError::SeqGap {
                seq: envelope.seq,
                expected: sql_seq as u64,
            }));
        }
        if sql_hash != envelope.event_hash {
            return Err(on_mismatch(ChainError::BadHash { seq: envelope.seq }));
        }
        events.push(envelope);
    }
    Ok(events)
}

fn load_verified_local(
    tx: &Connection,
    run_id: &str,
    on_mismatch: &dyn Fn(ChainError) -> StoreError,
) -> Result<Vec<EventEnvelope>, StoreError> {
    refuse_row_before_chain(tx, run_id)?;
    let local = read_verified_rows(tx, run_id, None, on_mismatch)?;
    verify_chain_after(run_id, 0, ZERO_HASH, &local)?;
    Ok(local)
}

fn insert_events(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    // Plain `INSERT` (not `INSERT OR IGNORE`) so duplicate `(run_id, seq)` is
    // never silently ignored; `verify_chain_for_sqlite` rules out duplicates
    // (pinned by `sqlite_append_envelopes_enforces_chain_integrity`), so a
    // redundant `inserted == 0` guard would be unreachable under exact coverage.
    let mut insert = tx.prepare_cached(import::INSERT_EVENT)?;
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?;
        let serialized = serde_json::to_string(envelope)?;
        insert.execute(params![run_id, seq, envelope.event_hash, serialized])?;
        if envelope.seq == 1 {
            let feature = (envelope.event_type == EventType::RunStarted)
                .then(|| started_feature(&envelope.payload))
                .flatten()
                .unwrap_or_default();
            let args = params![run_id, envelope.recorded_at, feature];
            tx.execute(UPDATE_STARTED_META, args)?;
        }
    }
    Ok(())
}

fn max_indexed_seq(tx: &Connection, run_id: &str) -> Result<Option<i64>, StoreError> {
    tx.query_row(
        "SELECT MAX(seq) FROM event_index WHERE run_id = ?1",
        params![run_id],
        |row| row.get(0),
    )
    .map_err(StoreError::from)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum IndexSyncPlan {
    Current,
    Refresh,
    Incremental,
    Rebuild,
}

fn plan_index_sync(
    tx: &Connection,
    run_id: &str,
    reading: IndexReading,
    ordinals: OrdinalState,
) -> Result<IndexSyncPlan, StoreError> {
    if ordinals == OrdinalState::Corrupt {
        return Ok(IndexSyncPlan::Rebuild);
    }
    match reading {
        IndexReading::Agrees if ordinals == OrdinalState::Agrees => Ok(IndexSyncPlan::Current),
        IndexReading::Agrees | IndexReading::Behind => Ok(IndexSyncPlan::Refresh),
        IndexReading::Parted(first_missing, IndexFault::Missing)
            if max_indexed_seq(tx, run_id)?.is_none_or(|max| max < first_missing as i64) =>
        {
            Ok(IndexSyncPlan::Incremental)
        }
        IndexReading::Parted(_, _) => Ok(IndexSyncPlan::Rebuild),
    }
}

fn sync_event_index(
    tx: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
    account: &Account,
) -> Result<(), StoreError> {
    let reading = event_index::first_unindexed(tx, run_id, events)?;
    let ordinals = event_index::ordinal::check(tx, run_id, events)?;
    let plan = plan_index_sync(tx, run_id, reading, ordinals)?;
    match plan {
        IndexSyncPlan::Current => return Ok(()),
        IndexSyncPlan::Refresh => {}
        IndexSyncPlan::Incremental => event_index::index_run(tx, run_id)?,
        IndexSyncPlan::Rebuild => {
            tx.execute("DELETE FROM event_index WHERE run_id = ?1", params![run_id])?;
            event_index::index_run(tx, run_id)?;
        }
    }
    if plan != IndexSyncPlan::Rebuild && ordinals == OrdinalState::Behind {
        event_index::ordinal::renumber(tx, run_id)?;
    }
    event_index::write_parks(tx, account)?;
    event_index::checkpoint::write_behind(tx, account)?;
    Ok(())
}

fn write_colocated_projection(
    tx: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
) -> Result<(), StoreError> {
    let listed = Listed::of(tx, run_id)?;
    let account = Account::whole(run_id, events);
    summary::upsert(tx, run_id, &Summary::of(&account, listed.listing()))?;
    seat_summary::write(tx, &account, None)?;
    sync_event_index(tx, run_id, events, &account)?;
    Ok(())
}

fn clear_projections(tx: &Connection) -> Result<(), StoreError> {
    for table in PROJECTION_TABLES {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }
    Ok(())
}

fn load_local_manifest(tx: &Connection, run_id: &str) -> Result<Option<Value>, StoreError> {
    let raw: Option<String> = tx
        .query_row(
            "SELECT manifest FROM runs WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_str(&raw)?))
}

fn require_local_manifest(tx: &Connection, run_id: &str) -> Result<Value, StoreError> {
    load_local_manifest(tx, run_id)?.ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))
}

fn verify_local_events_match(tx: &Connection, verified: &[VerifiedRun]) -> Result<(), StoreError> {
    let local_runs = query_strings(tx, LOCAL_RUNS)?;
    let mut expected: BTreeMap<&str, &VerifiedRun> = verified
        .iter()
        .map(|run| (run.run_id.as_str(), run))
        .collect();
    for run_id in local_runs {
        let Some(local_manifest) = load_local_manifest(tx, &run_id)? else {
            return Err(StoreError::DivergentEventLog { run_id });
        };
        let local = load_verified_local(tx, &run_id, &|_| divergent(&run_id))?;
        let Some(external) = expected.remove(run_id.as_str()) else {
            return Err(StoreError::DivergentEventLog { run_id });
        };
        if local_manifest != external.manifest
            || serde_json::to_vec(&local)? != serde_json::to_vec(&external.events)?
        {
            return Err(StoreError::DivergentEventLog { run_id });
        }
    }
    if let Some((extra, _)) = expected.into_iter().next() {
        return Err(divergent(extra));
    }
    Ok(())
}

impl EventLog for SqlitePorts<'_> {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        import::recordable(run_id)?;
        validate_manifest(run_id, manifest)?;
        let (feature, bundle_name) = manifest_meta(manifest);
        let manifest_json = serde_json::to_string(manifest)?;
        let host = origin::local_host();
        patiently("create_run", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            let row = Row {
                run_id,
                feature,
                bundle_name,
                manifest: &manifest_json,
                origin: host.as_deref(),
            };
            start::insert_run(&tx, &row)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        patiently("append", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            let manifest = require_local_manifest(&tx, run_id)?;
            let local = load_verified_local(&tx, run_id, &StoreError::Chain)?;
            validate_append_batch(run_id, &manifest, &local, expected_head, envelopes)?;
            insert_events(&tx, run_id, envelopes)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        validate_import_stream(run_id, manifest, envelopes)?;
        let (manifest_feature, bundle_name) = manifest_meta(manifest);
        let feature = started_feature_of(envelopes).unwrap_or(manifest_feature);
        let manifest_json = serde_json::to_string(manifest)?;
        // `brokkr_core::fold` above refuses empty streams with `FoldError::Empty`,
        // proved by `sqlite_import_run_stream_enforces_adoptable_id_fold_and_post_terminal`.
        let created_at = envelopes
            .first()
            .map_or("", |first| first.recorded_at.as_str());
        patiently("import_run", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            refuse_existing(&tx, run_id)?;
            tx.execute(
                "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![run_id, feature, bundle_name, manifest_json, created_at],
            )?;
            insert_events(&tx, run_id, envelopes)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        patiently("head", self.0.patience, &self.0.path, || {
            ensure_run_exists(&self.0.conn, run_id)?;
            refuse_row_before_chain(&self.0.conn, run_id)?;
            head_of(&self.0.conn, run_id)
        })
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        patiently("load", self.0.patience, &self.0.path, || {
            ensure_run_exists(&self.0.conn, run_id)?;
            let rows = if from_seq <= 1 {
                read_verified_rows(&self.0.conn, run_id, None, &StoreError::Chain)?
            } else if let Ok(after) = i64::try_from(from_seq - 1) {
                read_verified_rows(&self.0.conn, run_id, Some(after), &StoreError::Chain)?
            } else {
                Vec::new()
            };
            refuse_row_before_chain(&self.0.conn, run_id)?;
            Ok(rows)
        })
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        Store::manifest(self.0, run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        patiently("list_runs", self.0.patience, &self.0.path, || {
            query_strings(
                &self.0.conn,
                "SELECT run_id FROM runs ORDER BY created_at, run_id",
            )
        })
    }
}

impl ProjectionStore for SqlitePorts<'_> {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        if envelopes.iter().any(|envelope| envelope.run_id != run_id) {
            return Err(divergent(run_id));
        }
        patiently("apply_envelopes", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            let local_manifest = require_local_manifest(&tx, run_id)?;
            if let Some(expected) = manifest {
                if &local_manifest != expected {
                    return Err(divergent(run_id));
                }
            }
            let local = load_verified_local(&tx, run_id, &|_| divergent(run_id))?;
            if !envelopes.is_empty() {
                let matched: Vec<EventEnvelope> = local
                    .iter()
                    .skip_while(|event| event.seq < envelopes[0].seq)
                    .take(envelopes.len())
                    .cloned()
                    .collect();
                if serde_json::to_vec(&matched)? != serde_json::to_vec(envelopes)? {
                    return Err(divergent(run_id));
                }
            }
            write_colocated_projection(&tx, run_id, &local)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        self.0.list_runs()
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        patiently("rebuild", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            let verified = read_verified_log(log)?;
            verify_local_events_match(&tx, &verified)?;
            clear_projections(&tx)?;
            let mut total = 0u64;
            for run in &verified {
                write_colocated_projection(&tx, &run.run_id, &run.events)?;
                total += run.events.len() as u64;
            }
            tx.commit()?;
            Ok(total)
        })
    }
}

impl CoordinationStore for SqlitePorts<'_> {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        let _ = parse_clock(now)?;
        let by = Attribution {
            operator: "port",
            reason: "enqueue",
        };
        patiently("queue_add", self.0.patience, &self.0.path, || {
            queue::add_at(&self.0.conn, *entry, by, now)
        })
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        Store::queue_list(self.0)
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        let parsed = parse_clock(now)?;
        self.0.cooldowns_in_force(parsed)
    }
}
