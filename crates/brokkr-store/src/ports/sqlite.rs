//! SQLite [`Store`] implementations of [`EventLog`], [`ProjectionStore`],
//! and [`CoordinationStore`] (decision 0126, Slice 1).
//!
//! Delegates to the existing SQLite tables, triggers, and helpers without
//! altering any existing [`Store`] API or introducing new crate dependencies.

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::summary::{Account, Summary};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::start::Row;
use crate::summary::Listed;
use crate::{
    chain, event_index, head_of, import, origin, patiently, queue, read, seat_summary, start,
    summary, Attribution, Cooldown, EntryId, NewEntry, QueueEntry, Store, StoreError,
};

use super::{CoordinationStore, EventLog, ProjectionStore};

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

fn parse_clock(now: &str) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::parse(now, &Rfc3339).map_err(|_| StoreError::Clock {
        now: now.to_string(),
    })
}

fn ensure_run_exists(conn: &Connection, run_id: &str) -> Result<(), StoreError> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM runs WHERE run_id = ?1",
            params![run_id],
            |row| row.get(0),
        )
        .optional()?;
    found
        .map(|_| ())
        .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))
}

fn write_projections(
    tx: &Connection,
    run_id: &str,
    account: &Account,
    listed: &Listed,
) -> Result<(), StoreError> {
    summary::write(tx, run_id, &Summary::of(account, listed.listing()))?;
    seat_summary::write(tx, account, None)?;
    event_index::write_parks(tx, account)?;
    event_index::write_all_available(tx, account)?;
    Ok(())
}

impl EventLog for Store {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        import::recordable(run_id)?;
        let (feature, bundle_name) = manifest_meta(manifest);
        let manifest_json = serde_json::to_string(manifest)?;
        let host = origin::local_host();
        patiently("create_run", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
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
        patiently("append", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            ensure_run_exists(&tx, run_id)?;
            let (found_seq, found_hash) = head_of(&tx, run_id)?;
            if found_seq != expected_head.0 || found_hash != expected_head.1 {
                return Err(StoreError::HeadMoved {
                    expected_seq: expected_head.0,
                    found_seq,
                });
            }
            let mut insert = tx.prepare_cached(chain::INSERT_EVENT)?;
            for envelope in envelopes {
                let serialized = serde_json::to_string(envelope)?;
                let inserted = insert.execute(params![
                    run_id,
                    envelope.seq as i64,
                    envelope.event_hash,
                    serialized,
                ])?;
                if inserted == 0 {
                    return Err(StoreError::AppendConflict { seq: envelope.seq });
                }
            }
            drop(insert);
            tx.commit()?;
            Ok(())
        })
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        patiently("head", self.patience, &self.path, || {
            ensure_run_exists(&self.conn, run_id)?;
            head_of(&self.conn, run_id)
        })
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        let after = from_seq.saturating_sub(1).min(i64::MAX as u64) as i64;
        patiently("load", self.patience, &self.path, || {
            ensure_run_exists(&self.conn, run_id)?;
            self.rows(run_id, Some(after))
        })
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        Store::manifest(self, run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        patiently("list_runs", self.patience, &self.path, || {
            Ok(self
                .conn
                .prepare("SELECT run_id FROM runs ORDER BY created_at, run_id")?
                .query_map([], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?)
        })
    }
}

impl Store {
    fn rebuild_run(
        &self,
        run_id: &str,
        manifest: &Value,
        events: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        let manifest_json = serde_json::to_string(manifest)?;
        let (feature, bundle_name) = manifest_meta(manifest);
        let account = Account::whole(run_id, events);
        patiently("rebuild", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            let listed = match tx
                .query_row(
                    "SELECT feature, created_at FROM runs WHERE run_id = ?1",
                    params![run_id],
                    |row| {
                        Ok(Listed::new(
                            &row.get::<_, String>(0)?,
                            &row.get::<_, String>(1)?,
                        ))
                    },
                )
                .optional()?
            {
                Some(listed) => listed,
                None => {
                    let row = Row {
                        run_id,
                        feature,
                        bundle_name,
                        manifest: &manifest_json,
                        origin: None,
                    };
                    start::insert_run(&tx, &row)?
                }
            };
            let mut insert = tx.prepare_cached(chain::INSERT_EVENT)?;
            for event in events {
                let serialized = serde_json::to_string(event)?;
                insert.execute(params![
                    run_id,
                    event.seq as i64,
                    event.event_hash,
                    serialized,
                ])?;
            }
            drop(insert);
            write_projections(&tx, run_id, &account, &listed)?;
            tx.commit()?;
            Ok(())
        })
    }
}

impl ProjectionStore for Store {
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        let _ = envelopes;
        patiently("apply_envelopes", self.patience, &self.path, || {
            let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)?;
            ensure_run_exists(&tx, run_id)?;
            let listed = Listed::of(&tx, run_id)?;
            let events = read::rows_in(&tx, run_id, None)?;
            let account = Account::whole(run_id, &events);
            write_projections(&tx, run_id, &account, &listed)?;
            tx.commit()?;
            Ok(())
        })
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        Store::list_runs(self)
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        let run_ids = log.list_run_ids()?;
        let mut total = 0u64;
        for run_id in &run_ids {
            let manifest = log.manifest(run_id)?;
            let events = log.read_stream(run_id, 1)?;
            self.rebuild_run(run_id, &manifest, &events)?;
            total += events.len() as u64;
        }
        Ok(total)
    }
}

impl CoordinationStore for Store {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        let _ = parse_clock(now)?;
        let by = Attribution {
            operator: "port",
            reason: "enqueue",
        };
        patiently("queue_add", self.patience, &self.path, || {
            queue::add_at(&self.conn, *entry, by, now)
        })
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        Store::queue_list(self)
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        let parsed = parse_clock(now)?;
        self.cooldowns_in_force(parsed)
    }
}
