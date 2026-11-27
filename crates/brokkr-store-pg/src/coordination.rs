//! PostgreSQL implementation of [`brokkr_store::ports::CoordinationStore`],
//! `FOR UPDATE SKIP LOCKED` queue claims, and idempotent cooldown catch-up
//! (decision 0126 rulings 3 & 5).

use std::collections::{BTreeMap, BTreeSet};
use std::io;

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::ports::{
    checked_seq, cooldown_time_of, cooldown_until_second, extract_cooldowns, parse_clock,
    validate_cooldown_envelopes, validate_run_id, CooldownBatch, VerifiedRun,
};
use brokkr_store::{
    Cooldown, Cooling, EntryId, EntryState, NewEntry, QueueEntry, QueueRefusal, StoreError, Wait,
    WaitOn,
};

use crate::error::PgError;
use crate::wire::{PgClient, PgRow};

const CHECK_WAIT_EXISTS_SQL: &str = "SELECT 1 FROM brokkr_queue_entries WHERE id = $1::bigint";
const INSERT_QUEUE_ENTRY_SQL: &str =
    "INSERT INTO brokkr_queue_entries (payload, priority, status, added_at)
     VALUES ($1, $2::bigint, 'pending', $3)
     RETURNING id";
const INSERT_QUEUE_WAIT_SQL: &str = "INSERT INTO brokkr_queue_waits (entry_id, awaits, condition)
     VALUES ($1::bigint, $2::bigint, $3)";
const LIST_QUEUE_SQL: &str = "SELECT id, status, priority, payload, added_at, run_id
     FROM brokkr_queue_entries
     WHERE status <> 'dropped'
     ORDER BY CASE WHEN status IN ('pending', 'held', 'launching') OR run_id IS NULL THEN 0 ELSE 1 END,
              priority DESC, id ASC";
const LIST_ALL_WAITS_SQL: &str =
    "SELECT entry_id, awaits, condition FROM brokkr_queue_waits ORDER BY entry_id ASC, awaits ASC";
const LIST_ENTRY_WAITS_SQL: &str =
    "SELECT awaits, condition FROM brokkr_queue_waits WHERE entry_id = $1::bigint ORDER BY awaits ASC";
const CLAIM_NEXT_SQL: &str = "SELECT id, payload, priority, added_at
     FROM brokkr_queue_entries
     WHERE status = 'pending'
     ORDER BY priority DESC, id ASC
     FOR UPDATE SKIP LOCKED
     LIMIT 1";
const MARK_CLAIMED_SQL: &str = "UPDATE brokkr_queue_entries
     SET status = 'launching', claimed_by = $2, claimed_at = $3
     WHERE id = $1::bigint";
const SELECT_ACTIVE_COOLDOWNS_SQL: &str =
    "SELECT seq, site, provider, route, until_at, reason, retry_after_seconds, rounds, recorded_at
     FROM brokkr_provider_cooldowns
     WHERE until_at > $1::bigint
     ORDER BY seq ASC";
const INSERT_COOLDOWN_IDEMPOTENT_SQL: &str =
    "INSERT INTO brokkr_provider_cooldowns
        (run_id, event_seq, site, provider, route, until_at, reason, retry_after_seconds, rounds, recorded_at)
     VALUES ($1, $2::bigint, $3, $4, $5, $6::bigint, $7, $8::bigint, $9::bigint, $10::bigint)
     ON CONFLICT (run_id, event_seq, site, rounds) DO NOTHING";
const SELECT_COOLDOWN_CONFLICT_SQL: &str =
    "SELECT provider, route, until_at, reason, retry_after_seconds, recorded_at
     FROM brokkr_provider_cooldowns
     WHERE run_id = $1 AND event_seq = $2::bigint AND site = $3 AND rounds = $4::bigint";

/// A queue entry atomically claimed by a worker via `FOR UPDATE SKIP LOCKED`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedEntry {
    pub id: EntryId,
    pub payload: String,
    pub priority: i64,
    pub worker_id: String,
    pub claimed_at: String,
    pub added_at: String,
    pub waits: Vec<Wait>,
}

struct CooldownRow<'a> {
    run_id: &'a str,
    seq_str: String,
    rounds_str: String,
    row: &'a Cooling,
    until: i64,
    retry_i64: Option<i64>,
    recorded: i64,
}

pub(crate) fn enqueue(
    client: &mut PgClient,
    entry: &NewEntry<'_>,
    now: &str,
) -> Result<EntryId, StoreError> {
    let _ = parse_clock(now)?;
    validate_wait_list(entry.waits)?;
    client.in_transaction(
        |tx| {
            for wait in entry.waits {
                let awaited = wait.entry.0.to_string();
                let res = tx
                    .query(CHECK_WAIT_EXISTS_SQL, &[Some(&awaited)])
                    .map_err(PgError::into_store_error)?;
                if res.rows.is_empty() {
                    return Err(QueueRefusal::UnknownWait(wait.entry).into());
                }
            }
            let priority = entry.priority.to_string();
            let res = tx
                .query(
                    INSERT_QUEUE_ENTRY_SQL,
                    &[Some(entry.payload), Some(&priority), Some(now)],
                )
                .map_err(PgError::into_store_error)?;
            let id = res
                .rows
                .first()
                .ok_or_else(|| {
                    PgError::Protocol("INSERT RETURNING id returned no row".into())
                        .into_store_error()
                })?
                .get_i64(0)
                .map_err(PgError::into_store_error)?;
            let id_str = id.to_string();
            for wait in entry.waits {
                let awaits = wait.entry.0.to_string();
                tx.query(
                    INSERT_QUEUE_WAIT_SQL,
                    &[Some(&id_str), Some(&awaits), Some(wait.on.word())],
                )
                .map_err(PgError::into_store_error)?;
            }
            Ok(EntryId(id))
        },
        PgError::into_store_error,
    )
}

pub(crate) fn list_queue(client: &mut PgClient) -> Result<Vec<QueueEntry>, StoreError> {
    client.in_transaction(
        |tx| {
            let entries_res = tx
                .query(LIST_QUEUE_SQL, &[])
                .map_err(PgError::into_store_error)?;
            let waits_res = tx
                .query(LIST_ALL_WAITS_SQL, &[])
                .map_err(PgError::into_store_error)?;
            let mut waits_by_entry: BTreeMap<i64, Vec<Wait>> = BTreeMap::new();
            for row in &waits_res.rows {
                let entry_id = row.get_i64(0).map_err(PgError::into_store_error)?;
                let awaits = EntryId(row.get_i64(1).map_err(PgError::into_store_error)?);
                let cond = parse_wait_on(row.get_str(2).map_err(PgError::into_store_error)?)?;
                waits_by_entry.entry(entry_id).or_default().push(Wait {
                    entry: awaits,
                    on: cond,
                });
            }
            let mut out = Vec::with_capacity(entries_res.rows.len());
            let mut pos = 0u32;
            for row in &entries_res.rows {
                out.push(decode_queue_row(row, &mut waits_by_entry, &mut pos)?);
            }
            Ok(out)
        },
        PgError::into_store_error,
    )
}

pub(crate) fn claim_next_entry(
    client: &mut PgClient,
    worker_id: &str,
    now: &str,
) -> Result<Option<ClaimedEntry>, StoreError> {
    if worker_id.trim().is_empty() {
        return Err(PgError::EmptyWorkerId.into_store_error());
    }
    let _ = parse_clock(now)?;
    client.in_transaction(
        |tx| {
            let res = tx
                .query(CLAIM_NEXT_SQL, &[])
                .map_err(PgError::into_store_error)?;
            let Some(row) = res.rows.first() else {
                return Ok(None);
            };
            let id = row.get_i64(0).map_err(PgError::into_store_error)?;
            let payload = row
                .get_str(1)
                .map_err(PgError::into_store_error)?
                .to_string();
            let priority = row.get_i64(2).map_err(PgError::into_store_error)?;
            let added_at = row
                .get_str(3)
                .map_err(PgError::into_store_error)?
                .to_string();
            let id_str = id.to_string();
            tx.query(
                MARK_CLAIMED_SQL,
                &[Some(&id_str), Some(worker_id), Some(now)],
            )
            .map_err(PgError::into_store_error)?;
            let waits_res = tx
                .query(LIST_ENTRY_WAITS_SQL, &[Some(&id_str)])
                .map_err(PgError::into_store_error)?;
            let mut waits = Vec::with_capacity(waits_res.rows.len());
            for wrow in &waits_res.rows {
                let awaits = EntryId(wrow.get_i64(0).map_err(PgError::into_store_error)?);
                let cond = parse_wait_on(wrow.get_str(1).map_err(PgError::into_store_error)?)?;
                waits.push(Wait {
                    entry: awaits,
                    on: cond,
                });
            }
            Ok(Some(ClaimedEntry {
                id: EntryId(id),
                payload,
                priority,
                worker_id: worker_id.to_string(),
                claimed_at: now.to_string(),
                added_at,
                waits,
            }))
        },
        PgError::into_store_error,
    )
}

pub(crate) fn active_cooldowns(
    client: &mut PgClient,
    now: &str,
) -> Result<Vec<Cooldown>, StoreError> {
    let parsed = parse_clock(now)?.unix_timestamp().to_string();
    let res = client
        .query(SELECT_ACTIVE_COOLDOWNS_SQL, &[Some(&parsed)])
        .map_err(PgError::into_store_error)?;
    let mut out = Vec::with_capacity(res.rows.len());
    for row in &res.rows {
        out.push(decode_cooldown_row(row)?);
    }
    Ok(out)
}

pub(crate) fn apply_cooldowns(
    client: &mut PgClient,
    run_id: &str,
    envelopes: &[EventEnvelope],
) -> Result<u64, StoreError> {
    validate_run_id(run_id)?;
    validate_cooldown_envelopes(run_id, envelopes)?;
    let derived = extract_cooldowns(envelopes)?;
    client.in_transaction(
        |tx| apply_derived(tx, run_id, &derived),
        PgError::into_store_error,
    )
}

pub(crate) fn sync_verified_cooldowns(
    client: &mut PgClient,
    verified: &[VerifiedRun],
) -> Result<u64, StoreError> {
    client.in_transaction(
        |tx| {
            let mut inserted = 0u64;
            for run in verified {
                let derived = extract_cooldowns(&run.events)?;
                inserted += apply_derived(tx, &run.run_id, &derived)?;
            }
            Ok(inserted)
        },
        PgError::into_store_error,
    )
}

fn apply_derived(
    tx: &mut PgClient,
    run_id: &str,
    derived: &[CooldownBatch<'_>],
) -> Result<u64, StoreError> {
    let mut inserted = 0u64;
    for (event_seq, recorded_at, rows) in derived {
        // `provider_waits` refuses empty `waits` arrays (tested by
        // `coord_tests::coordination_idempotent_cooldowns_and_conflict_refusals`),
        // so `rows` is always non-empty here.
        let recorded = parse_clock(recorded_at)?.unix_timestamp();
        let mut seen = BTreeSet::new();
        for row in rows {
            if !seen.insert((row.site.as_str(), row.rounds)) {
                return Err(StoreError::DivergentEventLog {
                    run_id: run_id.to_string(),
                });
            }
            inserted += upsert_cooldown_row(tx, run_id, *event_seq, recorded, row)?;
        }
    }
    Ok(inserted)
}

fn upsert_cooldown_row(
    tx: &mut PgClient,
    run_id: &str,
    event_seq: u64,
    recorded: i64,
    row: &Cooling,
) -> Result<u64, StoreError> {
    let checked = CooldownRow {
        run_id,
        seq_str: checked_seq(event_seq)?.to_string(),
        rounds_str: checked_cooldown_i64("rounds", row.rounds)?.to_string(),
        row,
        until: cooldown_until_second(row.until_at),
        retry_i64: row
            .retry_after_seconds
            .map(|v| checked_cooldown_i64("retry_after_seconds", v))
            .transpose()?,
        recorded,
    };
    if checked.until <= checked.recorded {
        verify_existing_cooldown(tx, &checked, false)?;
        return Ok(0);
    }
    let until_str = checked.until.to_string();
    let retry_str = checked.retry_i64.map(|v| v.to_string());
    let recorded_str = checked.recorded.to_string();
    let res = tx
        .query(
            INSERT_COOLDOWN_IDEMPOTENT_SQL,
            &[
                Some(checked.run_id),
                Some(&checked.seq_str),
                Some(&checked.row.site),
                Some(&checked.row.provider),
                Some(&checked.row.route),
                Some(&until_str),
                Some(&checked.row.reason),
                retry_str.as_deref(),
                Some(&checked.rounds_str),
                Some(&recorded_str),
            ],
        )
        .map_err(PgError::into_store_error)?;
    if res.rows_affected > 0 {
        return Ok(1);
    }
    verify_existing_cooldown(tx, &checked, true)?;
    Ok(0)
}

fn verify_existing_cooldown(
    tx: &mut PgClient,
    checked: &CooldownRow<'_>,
    require_present: bool,
) -> Result<(), StoreError> {
    let res = tx
        .query(
            SELECT_COOLDOWN_CONFLICT_SQL,
            &[
                Some(checked.run_id),
                Some(&checked.seq_str),
                Some(&checked.row.site),
                Some(&checked.rounds_str),
            ],
        )
        .map_err(PgError::into_store_error)?;
    let Some(existing) = res.rows.first() else {
        if require_present {
            return Err(StoreError::DivergentEventLog {
                run_id: checked.run_id.to_string(),
            });
        }
        return Ok(());
    };
    let stored = (
        existing.get_str(0).map_err(PgError::into_store_error)?,
        existing.get_str(1).map_err(PgError::into_store_error)?,
        existing.get_i64(2).map_err(PgError::into_store_error)?,
        existing.get_str(3).map_err(PgError::into_store_error)?,
        existing.get_opt_i64(4).map_err(PgError::into_store_error)?,
        existing.get_i64(5).map_err(PgError::into_store_error)?,
    );
    let expected = (
        checked.row.provider.as_str(),
        checked.row.route.as_str(),
        checked.until,
        checked.row.reason.as_str(),
        checked.retry_i64,
        checked.recorded,
    );
    if stored != expected {
        return Err(StoreError::DivergentEventLog {
            run_id: checked.run_id.to_string(),
        });
    }
    Ok(())
}

fn checked_cooldown_i64(field: &'static str, value: u64) -> Result<i64, StoreError> {
    i64::try_from(value).map_err(|_| StoreError::CooldownFieldOutOfRange { field, value })
}

fn checked_pos_u64(field: &'static str, raw: i64, seq: i64) -> Result<u64, StoreError> {
    u64::try_from(raw).ok().filter(|&v| v >= 1).ok_or_else(|| {
        StoreError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid cooldown {field} {raw} at seq {seq}"),
        ))
    })
}

fn validate_wait_list(waits: &[Wait]) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    for &wait in waits {
        if !WaitOn::ALL.contains(&wait.on) {
            return Err(QueueRefusal::NotACondition(wait).into());
        }
        if !seen.insert(wait.entry) {
            return Err(QueueRefusal::DuplicateWait(wait.entry).into());
        }
    }
    Ok(())
}

fn parse_wait_on(word: &str) -> Result<WaitOn, StoreError> {
    WaitOn::ALL
        .into_iter()
        .find(|known| known.word() == word)
        .ok_or_else(|| {
            StoreError::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown queue wait condition '{word}'"),
            ))
        })
}

fn decode_queue_row(
    row: &PgRow,
    waits_by_entry: &mut BTreeMap<i64, Vec<Wait>>,
    pos: &mut u32,
) -> Result<QueueEntry, StoreError> {
    let id = row.get_i64(0).map_err(PgError::into_store_error)?;
    let status = row.get_str(1).map_err(PgError::into_store_error)?;
    let priority = row.get_i64(2).map_err(PgError::into_store_error)?;
    let payload = row
        .get_str(3)
        .map_err(PgError::into_store_error)?
        .to_string();
    let added_at = row
        .get_str(4)
        .map_err(PgError::into_store_error)?
        .to_string();
    let run_id = row.get_opt_str(5).map_err(PgError::into_store_error)?;
    let (state, position) = decode_entry_state(status, run_id, pos)?;
    let waits = waits_by_entry.remove(&id).unwrap_or_default();
    Ok(QueueEntry::new(
        EntryId(id),
        position,
        state,
        priority,
        waits,
        payload,
        added_at,
    ))
}

fn decode_cooldown_row(row: &PgRow) -> Result<Cooldown, StoreError> {
    let seq = row.get_i64(0).map_err(PgError::into_store_error)?;
    let until_raw = row.get_i64(4).map_err(PgError::into_store_error)?;
    let retry_raw = row.get_opt_i64(6).map_err(PgError::into_store_error)?;
    let rounds_raw = row.get_i64(7).map_err(PgError::into_store_error)?;
    let recorded_raw = row.get_i64(8).map_err(PgError::into_store_error)?;
    Ok(Cooldown {
        site: row
            .get_str(1)
            .map_err(PgError::into_store_error)?
            .to_string(),
        provider: row
            .get_str(2)
            .map_err(PgError::into_store_error)?
            .to_string(),
        route: row
            .get_str(3)
            .map_err(PgError::into_store_error)?
            .to_string(),
        until_at: cooldown_time_of(until_raw, seq)?,
        reason: row
            .get_str(5)
            .map_err(PgError::into_store_error)?
            .to_string(),
        retry_after_seconds: retry_raw
            .map(|r| checked_pos_u64("retry_after_seconds", r, seq))
            .transpose()?,
        rounds: checked_pos_u64("rounds", rounds_raw, seq)?,
        recorded_at: cooldown_time_of(recorded_raw, seq)?,
    })
}

fn decode_entry_state(
    status: &str,
    run_id: Option<&str>,
    pos: &mut u32,
) -> Result<(EntryState, Option<u32>), StoreError> {
    match status {
        "pending" => {
            *pos += 1;
            Ok((EntryState::Queued, Some(*pos)))
        }
        "held" => {
            *pos += 1;
            Ok((EntryState::Held, Some(*pos)))
        }
        "launching" => {
            *pos += 1;
            Ok((EntryState::Launching, Some(*pos)))
        }
        "claimed" => match run_id.filter(|r| !r.is_empty()) {
            Some(run) => Ok((
                EntryState::Claimed {
                    run: run.to_string(),
                    merged: false,
                },
                None,
            )),
            None => {
                *pos += 1;
                Ok((EntryState::Launching, Some(*pos)))
            }
        },
        other => Err(StoreError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unknown queue entry status '{other}'"),
        ))),
    }
}
