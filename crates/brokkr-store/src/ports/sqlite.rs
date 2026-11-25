//! SQLite [`SqlitePorts`] implementations of [`EventLog`],
//! [`ProjectionStore`], and [`CoordinationStore`] (decision 0126, Slice 1).
//!
//! Wrapped behind [`SqlitePorts`] rather than implemented directly on
//! [`Store`] so importing `brokkr_store::ports::*` never shadows inherent
//! [`Store`] methods (`create_run`, `list_runs`, `manifest`).

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, ChainError, EventEnvelope, EventType};
use brokkr_core::summary::Account;
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::damage;
use crate::start::Row;
use crate::summary::Listed;
use crate::{
    head_of, import, origin, patiently, queue, refuse_existing, start, terminal, Attribution,
    Cooldown, EntryId, NewEntry, QueueEntry, Store, StoreError,
};

use super::{CoordinationStore, EventLog, ProjectionStore};

mod projection;

pub(crate) use projection::{migrate_projection_hashes, projection_hashes_intact};
use projection::{
    rebuild_colocated, rebuild_decoupled, resolve_projection_plan, verify_local_events_match,
    write_decoupled_projection, write_full_projection, ProjectionPlan, VerifiedRun,
    EXISTING_PROJECTED_RUNS,
};

/// Thin adapter exposing a SQLite [`Store`] through the [`EventLog`],
/// [`ProjectionStore`], and [`CoordinationStore`] traits without shadowing
/// [`Store`]'s inherent methods.
pub struct SqlitePorts<'a>(&'a Store);

impl<'a> SqlitePorts<'a> {
    /// Wrap a [`Store`] reference as a port implementation.
    pub fn new(store: &'a Store) -> Self {
        Self(store)
    }

    fn projected_runs(&self) -> Result<Vec<String>, StoreError> {
        patiently("rebuild", self.0.patience, &self.0.path, || {
            query_strings(&self.0.conn, EXISTING_PROJECTED_RUNS)
        })
    }
}

const UPDATE_CREATED_AT: &str = "UPDATE runs SET created_at = ?2 WHERE run_id = ?1";
const LIST_RUNS_SUMMARY: &str =
    "SELECT run_id, COALESCE(feature, ''), COALESCE(created_at_whole, created_at, '')
     FROM run_summary
     ORDER BY COALESCE(created_at_whole, created_at, ''), run_id";
const HAS_LOCAL_RUNS_OR_EVENTS: &str =
    "SELECT EXISTS(SELECT 1 FROM runs) OR EXISTS(SELECT 1 FROM events)";
const HAS_RUNLESS_SUMMARY: &str =
    "SELECT EXISTS(SELECT 1 FROM run_summary WHERE run_id NOT IN (SELECT run_id FROM runs))";

fn manifest_meta(manifest: &Value) -> (&str, &str) {
    let feature = manifest
        .get("feature")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let bundle_name = manifest
        .get("bundle_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    (feature, bundle_name)
}

fn listing_from(manifest: Option<&Value>, events: &[EventEnvelope]) -> Listed {
    let started = events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted);
    let feature = started
        .and_then(|event| {
            event
                .payload
                .get("feature")
                .and_then(Value::as_str)
                .or_else(|| {
                    event
                        .payload
                        .pointer("/manifest/feature")
                        .and_then(Value::as_str)
                })
        })
        .or_else(|| manifest.and_then(|value| value.get("feature").and_then(Value::as_str)))
        .unwrap_or_default();
    let created_at = events
        .first()
        .map(|event| event.recorded_at.as_str())
        .unwrap_or_default();
    Listed::new(feature, created_at)
}

fn checked_seq(seq: u64) -> Result<i64, StoreError> {
    i64::try_from(seq).map_err(|_| {
        StoreError::Chain(ChainError::SeqOverflow {
            after: i64::MAX as u64,
        })
    })
}

fn verify_batch_envelopes(run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
    for envelope in envelopes {
        if envelope.run_id != run_id {
            return Err(StoreError::Chain(ChainError::ForeignRun {
                seq: envelope.seq,
            }));
        }
        let _ = checked_seq(envelope.seq)?;
    }
    Ok(())
}

fn verify_chain_for_sqlite(
    run_id: &str,
    expected_head: (u64, &str),
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        let _ = checked_seq(envelope.seq)?;
    }
    verify_chain_after(run_id, expected_head.0, expected_head.1, envelopes)?;
    Ok(())
}

fn parse_clock(now: &str) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::parse(now, &Rfc3339).map_err(|_| StoreError::Clock {
        now: now.to_string(),
    })
}

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

fn divergent(run_id: &str) -> StoreError {
    StoreError::DivergentEventLog {
        run_id: run_id.to_string(),
    }
}

fn query_strings(conn: &Connection, sql: &str) -> Result<Vec<String>, StoreError> {
    Ok(conn
        .prepare(sql)?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?)
}

fn has_local_runs_or_events(tx: &Connection) -> Result<bool, StoreError> {
    let found: i64 = tx.query_row(HAS_LOCAL_RUNS_OR_EVENTS, [], |row| row.get(0))?;
    Ok(found != 0)
}

/// Refuse authoritative `runs`/`events` writes into a SQLite database already
/// carrying decoupled projections (runs in `run_summary` absent from `runs`).
pub(crate) fn refuse_if_decoupled_projection_db(
    tx: &Connection,
    run_id: &str,
) -> Result<(), StoreError> {
    let has_runless: i64 = tx.query_row(HAS_RUNLESS_SUMMARY, [], |row| row.get(0))?;
    if has_runless != 0 {
        return Err(divergent(run_id));
    }
    Ok(())
}

fn ensure_run_exists(conn: &Connection, run_id: &str) -> Result<(), StoreError> {
    if has_run(conn, run_id)? {
        Ok(())
    } else {
        Err(StoreError::RunNotFound(run_id.to_string()))
    }
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

fn load_verified_local(tx: &Connection, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
    refuse_row_before_chain(tx, run_id)?;
    let rows = tx
        .prepare("SELECT seq, event_hash, envelope FROM events WHERE run_id = ?1 ORDER BY seq")?
        .query_map(params![run_id], read_local_event)?
        .collect::<Result<Vec<_>, _>>()?;
    let mut local = Vec::with_capacity(rows.len());
    for (sql_seq, sql_hash, raw) in rows {
        let envelope = damage::envelope(run_id, sql_seq, &raw)?;
        if sql_seq as u64 != envelope.seq || sql_hash != envelope.event_hash {
            return Err(divergent(run_id));
        }
        local.push(envelope);
    }
    verify_chain_after(run_id, 0, ZERO_HASH, &local)?;
    Ok(local)
}

fn insert_events(
    tx: &Connection,
    run_id: &str,
    head_seq: &mut u64,
    envelopes: &[EventEnvelope],
    check_terminal: bool,
) -> Result<(), StoreError> {
    // Plain `INSERT` (not `INSERT OR IGNORE`) so duplicate `(run_id, seq)` is
    // never silently ignored; `verify_chain_for_sqlite` rules out duplicates
    // (pinned by `sqlite_append_envelopes_enforces_chain_integrity`), so a
    // redundant `inserted == 0` guard would be unreachable under exact coverage.
    let mut insert = tx.prepare_cached(import::INSERT_EVENT)?;
    for envelope in envelopes {
        if check_terminal {
            terminal::refuse(tx, run_id, envelope.event_type, *head_seq)?;
        }
        let seq = checked_seq(envelope.seq)?;
        let serialized = serde_json::to_string(envelope)?;
        insert.execute(params![run_id, seq, envelope.event_hash, serialized])?;
        if envelope.seq == 1 {
            tx.execute(UPDATE_CREATED_AT, params![run_id, envelope.recorded_at])?;
        }
        *head_seq = envelope.seq;
    }
    Ok(())
}

impl EventLog for SqlitePorts<'_> {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        import::recordable(run_id)?;
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
            ensure_run_exists(&tx, run_id)?;
            refuse_row_before_chain(&tx, run_id)?;
            let (mut head_seq, found_hash) = head_of(&tx, run_id)?;
            if head_seq != expected_head.0 || found_hash != expected_head.1 {
                return Err(StoreError::HeadMoved {
                    expected_seq: expected_head.0,
                    found_seq: head_seq,
                });
            }
            verify_chain_for_sqlite(run_id, expected_head, envelopes)?;
            insert_events(&tx, run_id, &mut head_seq, envelopes, true)?;
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
        import::recordable(run_id)?;
        verify_chain_for_sqlite(run_id, (0, ZERO_HASH), envelopes)?;
        let listed = listing_from(Some(manifest), envelopes);
        let (_, bundle_name) = manifest_meta(manifest);
        let manifest_json = serde_json::to_string(manifest)?;
        let feature = listed.listing().feature;
        let created_at = listed.listing().created_at;
        patiently("import_run", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            if envelopes.is_empty() {
                let row = Row {
                    run_id,
                    feature,
                    bundle_name,
                    manifest: &manifest_json,
                    origin: None,
                };
                start::insert_run(&tx, &row)?;
            } else {
                refuse_if_decoupled_projection_db(&tx, run_id)?;
                refuse_existing(&tx, run_id)?;
                tx.execute(
                    "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![run_id, feature, bundle_name, manifest_json, created_at],
                )?;
                let mut head_seq = 0;
                insert_events(&tx, run_id, &mut head_seq, envelopes, false)?;
            }
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
                self.0.rows(run_id, None)?
            } else if let Ok(after) = i64::try_from(from_seq - 1) {
                self.0.rows(run_id, Some(after))?
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
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        verify_batch_envelopes(run_id, envelopes)?;
        patiently("apply_envelopes", self.0.patience, &self.0.path, || {
            if envelopes.is_empty() {
                if has_local_runs_or_events(&self.0.conn)? && !has_run(&self.0.conn, run_id)? {
                    return Err(divergent(run_id));
                }
                return Ok(());
            }
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            match resolve_projection_plan(&tx, run_id, envelopes)? {
                ProjectionPlan::Skip => Ok(()),
                ProjectionPlan::Colocated(stream) => {
                    let listed = listing_from(None, &stream);
                    let account = Account::whole(run_id, &stream);
                    write_full_projection(&tx, run_id, &stream, &account, &listed)?;
                    tx.commit()?;
                    Ok(())
                }
                ProjectionPlan::Decoupled(stream) => {
                    let listed = listing_from(None, &stream);
                    let account = Account::whole(run_id, &stream);
                    write_decoupled_projection(&tx, run_id, &stream, &account, &listed)?;
                    tx.commit()?;
                    Ok(())
                }
            }
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        let colocated = patiently("list_runs", self.0.patience, &self.0.path, || {
            has_local_runs_or_events(&self.0.conn)
        })?;
        if colocated {
            return self.0.list_runs();
        }
        patiently("list_runs", self.0.patience, &self.0.path, || {
            Ok(self
                .0
                .conn
                .prepare(LIST_RUNS_SUMMARY)?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let pre_existing = self.projected_runs()?;
        let run_ids = log.list_run_ids()?;
        let mut verified = Vec::with_capacity(run_ids.len());
        for run_id in run_ids {
            let manifest = log.manifest(&run_id)?;
            let events = log.read_stream(&run_id, 1)?;
            verify_chain_for_sqlite(&run_id, (0, ZERO_HASH), &events)?;
            let account = Account::whole(&run_id, &events);
            let listed = listing_from(Some(&manifest), &events);
            verified.push(VerifiedRun {
                run_id,
                events,
                account,
                listed,
            });
        }
        patiently("rebuild", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            let total = if verify_local_events_match(&tx, &verified)? {
                rebuild_colocated(&tx, &verified)?
            } else {
                rebuild_decoupled(&tx, &verified, &pre_existing)?
            };
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
