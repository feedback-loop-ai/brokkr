//! SQLite [`SqlitePorts`] implementations of [`EventLog`],
//! [`ProjectionStore`], and [`CoordinationStore`] (decision 0126, Slice 1).
//!
//! Wrapped behind [`SqlitePorts`] rather than implemented directly on
//! [`Store`] so importing `brokkr_store::ports::*` never shadows inherent
//! [`Store`] methods (`create_run`, `list_runs`, `manifest`).

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, ChainError, EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use brokkr_core::summary::{Account, Summary};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::read;
use crate::start::Row;
use crate::summary::Listed;
use crate::{
    event_index, head_of, import, now_rfc3339, origin, patiently, queue, refuse_existing,
    seat_summary, start, summary, terminal, Attribution, Cooldown, EntryId, NewEntry, QueueEntry,
    Store, StoreError,
};

use super::{CoordinationStore, EventLog, ProjectionStore};

/// Thin adapter exposing a SQLite [`Store`] through the [`EventLog`],
/// [`ProjectionStore`], and [`CoordinationStore`] traits without shadowing
/// [`Store`]'s inherent methods.
pub struct SqlitePorts<'a>(&'a Store);

impl<'a> SqlitePorts<'a> {
    /// Wrap a [`Store`] reference as a port implementation.
    pub fn new(store: &'a Store) -> Self {
        Self(store)
    }
}

const INSERT_EVENT_INDEX: &str = "INSERT OR IGNORE INTO event_index
         (run_id, seq, type, attempt_id, member, recorded_at, event_id, stream_ordinal)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

// Plain `INSERT` (not `INSERT OR IGNORE`) so a duplicate `(run_id, seq)` can
// never be silently ignored; `verify_chain_for_sqlite` inside the immediate
// transaction already rules out duplicate or non-contiguous sequence numbers
// (pinned by `sqlite_append_envelopes_enforces_chain_integrity`), so a
// redundant `inserted == 0` guard would be unreachable under exact coverage.
const INSERT_VERIFIED_EVENT: &str =
    "INSERT INTO events (run_id, seq, event_hash, envelope) VALUES (?1, ?2, ?3, ?4)";
const UPDATE_CREATED_AT: &str = "UPDATE runs SET created_at = ?2 WHERE run_id = ?1";
const LIST_RUNS_SUMMARY: &str =
    "SELECT run_id, COALESCE(feature, ''), COALESCE(created_at_whole, created_at, '')
     FROM run_summary
     ORDER BY COALESCE(created_at_whole, created_at, ''), run_id";
const PROJECTION_TABLES: [&str; 5] = [
    "run_summary",
    "seat_summary",
    "seat_head",
    "seat_lag",
    "event_index",
];
const HAS_LOCAL_RUN_OR_EVENTS: &str = "SELECT EXISTS(SELECT 1 FROM runs WHERE run_id = ?1)
         OR EXISTS(SELECT 1 FROM events WHERE run_id = ?1)";
const EXISTING_PROJECTED_RUNS: &str = "SELECT run_id FROM run_summary
     UNION SELECT DISTINCT run_id FROM seat_summary
     UNION SELECT run_id FROM seat_head
     UNION SELECT run_id FROM seat_lag
     UNION SELECT DISTINCT run_id FROM event_index";
const LOCAL_RUNS: &str = "SELECT run_id FROM runs
     UNION SELECT DISTINCT run_id FROM events
     ORDER BY run_id";

struct VerifiedRun {
    run_id: String,
    events: Vec<EventEnvelope>,
    account: Account,
    listed: Listed,
}

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

fn has_local_run_or_events(tx: &Connection, run_id: &str) -> Result<bool, StoreError> {
    let found: i64 = tx.query_row(HAS_LOCAL_RUN_OR_EVENTS, params![run_id], |row| row.get(0))?;
    Ok(found != 0)
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

fn load_verified_local(tx: &Connection, run_id: &str) -> Result<Vec<EventEnvelope>, StoreError> {
    refuse_row_before_chain(tx, run_id)?;
    let local = read::rows_in(tx, run_id, None)?;
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
    let mut insert = tx.prepare_cached(INSERT_VERIFIED_EVENT)?;
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
    Ok(match row {
        Some((feature, created_at)) => Listed::new(&feature, &created_at),
        None => fallback.clone(),
    })
}

fn write_full_projection(
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

fn clear_stale_projections(tx: &Connection, keep: &BTreeSet<&str>) -> Result<(), StoreError> {
    let existing: Vec<String> = tx
        .prepare(EXISTING_PROJECTED_RUNS)?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for run_id in existing {
        if !keep.contains(run_id.as_str()) {
            clear_run_projections(tx, &run_id)?;
        }
    }
    Ok(())
}

fn is_projection_current(
    tx: &Connection,
    run_id: &str,
    incoming_max: i64,
) -> Result<bool, StoreError> {
    let progress: Option<(i64, Option<i64>)> = tx
        .query_row(
            "SELECT head_seq, folded_seq FROM run_summary WHERE run_id = ?1",
            params![run_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(match progress {
        Some((head_seq, folded_seq)) => {
            head_seq > incoming_max
                || (head_seq == incoming_max && folded_seq == Some(incoming_max))
        }
        None => false,
    })
}

fn resolve_projection_stream(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<Vec<EventEnvelope>, StoreError> {
    if has_local_run_or_events(tx, run_id)? {
        let local = load_verified_local(tx, run_id)?;
        let matched: Vec<EventEnvelope> = local
            .iter()
            .skip_while(|event| event.seq < envelopes[0].seq)
            .take(envelopes.len())
            .cloned()
            .collect();
        if serde_json::to_vec(&matched)? != serde_json::to_vec(envelopes)? {
            return Err(StoreError::DivergentEventLog {
                run_id: run_id.to_string(),
            });
        }
        Ok(local)
    } else {
        verify_chain_for_sqlite(run_id, (0, ZERO_HASH), envelopes)?;
        Ok(envelopes.to_vec())
    }
}

fn verify_local_events_match(
    tx: &Connection,
    verified: &[VerifiedRun],
) -> Result<bool, StoreError> {
    let local_runs: Vec<String> = tx
        .prepare(LOCAL_RUNS)?
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
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
        return Err(StoreError::DivergentEventLog {
            run_id: extra.to_string(),
        });
    }
    Ok(true)
}

fn rebuild_colocated(tx: &Connection, verified: &[VerifiedRun]) -> Result<u64, StoreError> {
    clear_projections(tx)?;
    let mut total = 0u64;
    for run in verified {
        write_full_projection(tx, &run.run_id, &run.events, &run.account, &run.listed)?;
        total += run.events.len() as u64;
    }
    Ok(total)
}

fn rebuild_decoupled(tx: &Connection, verified: &[VerifiedRun]) -> Result<u64, StoreError> {
    let keep: BTreeSet<&str> = verified.iter().map(|run| run.run_id.as_str()).collect();
    clear_stale_projections(tx, &keep)?;
    let mut total = 0u64;
    for run in verified {
        let scanned_head = checked_seq(run.events.last().map_or(0, |event| event.seq))?;
        let current_head: Option<i64> = tx
            .query_row(
                "SELECT head_seq FROM run_summary WHERE run_id = ?1",
                params![run.run_id],
                |row| row.get(0),
            )
            .optional()?;
        if current_head.is_some_and(|head| head > scanned_head) || run.events.is_empty() {
            continue;
        }
        clear_run_projections(tx, &run.run_id)?;
        write_full_projection(tx, &run.run_id, &run.events, &run.account, &run.listed)?;
        total += run.events.len() as u64;
    }
    Ok(total)
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
        let created_at = if listed.listing().created_at.is_empty() {
            now_rfc3339()
        } else {
            listed.listing().created_at.to_string()
        };
        let feature = listed.listing().feature;
        patiently("import_run", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            refuse_existing(&tx, run_id)?;
            tx.execute(
                "INSERT INTO runs (run_id, feature, bundle_name, manifest, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![run_id, feature, bundle_name, manifest_json, created_at],
            )?;
            let mut head_seq = 0;
            insert_events(&tx, run_id, &mut head_seq, envelopes, false)?;
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
            Ok(self
                .0
                .conn
                .prepare("SELECT run_id FROM runs ORDER BY created_at, run_id")?
                .query_map([], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }
}

impl ProjectionStore for SqlitePorts<'_> {
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        verify_batch_envelopes(run_id, envelopes)?;
        if envelopes.is_empty() {
            return Ok(());
        }
        let max_seq = envelopes
            .iter()
            .map(|envelope| envelope.seq)
            .max()
            .expect("envelopes is non-empty");
        let incoming_max = checked_seq(max_seq)?;
        patiently("apply_envelopes", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            let stream = resolve_projection_stream(&tx, run_id, envelopes)?;
            if is_projection_current(&tx, run_id, incoming_max)? {
                return Ok(());
            }
            let listed = listing_from(None, &stream);
            let account = Account::whole(run_id, &stream);
            write_full_projection(&tx, run_id, &stream, &account, &listed)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
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
                rebuild_decoupled(&tx, &verified)?
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
