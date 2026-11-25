//! SQLite [`SqlitePorts`] implementations of [`EventLog`],
//! [`ProjectionStore`], and [`CoordinationStore`] (decision 0126, Slice 1).
//!
//! Wrapped behind [`SqlitePorts`] rather than implemented directly on
//! [`Store`] so importing `brokkr_store::ports::*` never shadows inherent
//! [`Store`] methods (`create_run`, `list_runs`, `manifest`).

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, ChainError, EventEnvelope, EventType};
use brokkr_core::event_index::Indexed;
use brokkr_core::fold::terminal_event;
use brokkr_core::summary::{
    entered_phase, Account, Bounded, Checkpoint, Summary, NAME_BYTES, STAMP_BYTES,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::damage::envelope as parse_envelope;
use crate::start::Row;
use crate::summary::Listed;
use crate::{
    chain, event_index, head_of, import, now_rfc3339, origin, patiently, queue, refuse_existing,
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
         (run_id, seq, type, attempt_id, member, recorded_at, event_id)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

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
    let created_at = started
        .or_else(|| events.first())
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

fn insert_events(
    tx: &Connection,
    run_id: &str,
    head_seq: &mut u64,
    envelopes: &[EventEnvelope],
    check_terminal: bool,
) -> Result<(), StoreError> {
    let mut insert = tx.prepare_cached(chain::INSERT_EVENT)?;
    for envelope in envelopes {
        if check_terminal {
            terminal::refuse(tx, run_id, envelope.event_type, *head_seq)?;
        }
        let seq = checked_seq(envelope.seq)?;
        let serialized = serde_json::to_string(envelope)?;
        let inserted = insert.execute(params![run_id, seq, envelope.event_hash, serialized])?;
        if inserted == 0 {
            return Err(StoreError::AppendConflict { seq: envelope.seq });
        }
        *head_seq = envelope.seq;
    }
    Ok(())
}

fn insert_event_index_rows(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    let mut insert = tx.prepare_cached(INSERT_EVENT_INDEX)?;
    for envelope in envelopes {
        let seq = checked_seq(envelope.seq)?;
        let indexed = Indexed::of(envelope);
        insert.execute(params![
            run_id,
            seq,
            indexed.event_type.as_str(),
            indexed.attempt_id,
            indexed.member,
            indexed.recorded_at,
            envelope.event_id,
        ])?;
    }
    Ok(())
}

fn write_full_projection(
    tx: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
    account: &Account,
    listed: &Listed,
) -> Result<(), StoreError> {
    summary::upsert(tx, run_id, &Summary::of(account, listed.listing()))?;
    seat_summary::write(tx, account, None)?;
    insert_event_index_rows(tx, run_id, events)?;
    event_index::write_parks(tx, account)?;
    event_index::write_all_checkpoints(tx, account)?;
    Ok(())
}

fn clear_projections(tx: &Connection) -> Result<(), StoreError> {
    tx.execute("DELETE FROM run_summary", [])?;
    tx.execute("DELETE FROM seat_summary", [])?;
    tx.execute("DELETE FROM seat_head", [])?;
    tx.execute("DELETE FROM seat_lag", [])?;
    tx.execute("DELETE FROM event_index", [])?;
    Ok(())
}

fn combined_with_prior(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<Option<Vec<EventEnvelope>>, StoreError> {
    let first_seq = checked_seq(envelopes[0].seq)?;
    let texts: Vec<(String, i64)> = tx
        .prepare_cached(
            "SELECT envelope, seq FROM events WHERE run_id = ?1 AND seq < ?2 ORDER BY seq",
        )?
        .query_map(params![run_id, first_seq], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if texts.is_empty() {
        return Ok(None);
    }
    let mut combined = Vec::with_capacity(texts.len() + envelopes.len());
    for (raw, seq) in &texts {
        combined.push(parse_envelope(run_id, *seq, raw)?);
    }
    combined.extend(envelopes.iter().cloned());
    Ok(Some(combined))
}

fn apply_incremental_projection(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    insert_event_index_rows(tx, run_id, envelopes)?;
    let mut phase: Option<String> = tx
        .query_row(
            "SELECT COALESCE(phase_whole, phase) FROM run_summary WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let mut updated_phase = None;
    let mut terminal = None;
    for event in envelopes {
        if let Some(entered) = entered_phase(event) {
            let owned = entered.to_string();
            phase = Some(owned.clone());
            updated_phase = Some(owned);
        }
        if let Some(status) = terminal_event(event.event_type) {
            terminal = Some(status);
        }
        if event.event_type == EventType::EffectCheckpointed {
            let checkpoint = Checkpoint::read(event, phase.as_deref());
            event_index::write_checkpoint(tx, run_id, event.seq, checkpoint.as_ref())?;
        }
    }
    tx.execute(
        "INSERT OR IGNORE INTO run_summary (run_id, head_seq, head_hash)
         VALUES (?1, 0, ?2)",
        params![run_id, ZERO_HASH],
    )?;
    let last = envelopes.last().expect("envelopes is non-empty");
    let last_seq = checked_seq(last.seq)?;
    let stamped = Bounded::within(&last.recorded_at, STAMP_BYTES);
    tx.execute(
        "UPDATE run_summary
         SET head_seq = ?2, head_hash = ?3,
             last_recorded_at = ?4, last_recorded_at_whole = ?5
         WHERE run_id = ?1 AND ?2 > head_seq",
        params![
            run_id,
            last_seq,
            last.event_hash,
            stamped.cut,
            stamped.whole,
        ],
    )?;
    if let Some(phase_text) = updated_phase {
        let bounded = Bounded::within(&phase_text, NAME_BYTES);
        tx.execute(
            "UPDATE run_summary SET phase = ?2, phase_whole = ?3 WHERE run_id = ?1",
            params![run_id, bounded.cut, bounded.whole],
        )?;
    }
    if let Some(status) = terminal {
        tx.execute(
            "UPDATE run_summary SET status = ?2 WHERE run_id = ?1",
            params![run_id, status.as_str()],
        )?;
    }
    Ok(())
}

fn listed_for_apply(
    tx: &Connection,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<Listed, StoreError> {
    if has_run(tx, run_id)? {
        Listed::of(tx, run_id)
    } else {
        Ok(listing_from(None, envelopes))
    }
}

fn listed_for_rebuild(
    tx: &Connection,
    run_id: &str,
    events: &[EventEnvelope],
    fallback: &Listed,
) -> Result<Listed, StoreError> {
    let own_head = events
        .last()
        .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
    if has_run(tx, run_id)? {
        let (seq, hash) = head_of(tx, run_id)?;
        if (seq, hash.as_str()) == own_head {
            return Listed::of(tx, run_id);
        }
    }
    Ok(fallback.clone())
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
        verify_batch_envelopes(run_id, envelopes)?;
        patiently("append", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            ensure_run_exists(&tx, run_id)?;
            let (mut head_seq, found_hash) = head_of(&tx, run_id)?;
            if head_seq != expected_head.0 || found_hash != expected_head.1 {
                return Err(StoreError::HeadMoved {
                    expected_seq: expected_head.0,
                    found_seq: head_seq,
                });
            }
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
        verify_batch_envelopes(run_id, envelopes)?;
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
        patiently("apply_envelopes", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            if envelopes[0].seq == 1 {
                let listed = listed_for_apply(&tx, run_id, envelopes)?;
                let account = Account::whole(run_id, envelopes);
                write_full_projection(&tx, run_id, envelopes, &account, &listed)?;
            } else if let Some(combined) = combined_with_prior(&tx, run_id, envelopes)? {
                let listed = listed_for_apply(&tx, run_id, &combined)?;
                let account = Account::whole(run_id, &combined);
                write_full_projection(&tx, run_id, &combined, &account, &listed)?;
            } else {
                apply_incremental_projection(&tx, run_id, envelopes)?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        patiently("list_runs", self.0.patience, &self.0.path, || {
            Ok(self
                .0
                .conn
                .prepare(
                    "SELECT run_id, COALESCE(feature, ''), COALESCE(created_at_whole, created_at, '')
                     FROM run_summary
                     ORDER BY COALESCE(created_at_whole, created_at, ''), run_id",
                )?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let run_ids = log.list_run_ids()?;
        let mut verified = Vec::with_capacity(run_ids.len());
        let mut total = 0u64;
        for run_id in run_ids {
            let manifest = log.manifest(&run_id)?;
            let events = log.read_stream(&run_id, 1)?;
            verify_chain_after(&run_id, 0, ZERO_HASH, &events)?;
            let account = Account::whole(&run_id, &events);
            let fallback = listing_from(Some(&manifest), &events);
            total += events.len() as u64;
            verified.push((run_id, events, account, fallback));
        }
        patiently("rebuild", self.0.patience, &self.0.path, || {
            let tx = Transaction::new_unchecked(&self.0.conn, TransactionBehavior::Immediate)?;
            clear_projections(&tx)?;
            for (run_id, events, account, fallback) in &verified {
                let listed = listed_for_rebuild(&tx, run_id, events, fallback)?;
                write_full_projection(&tx, run_id, events, account, &listed)?;
            }
            tx.commit()?;
            Ok(())
        })?;
        Ok(total)
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
