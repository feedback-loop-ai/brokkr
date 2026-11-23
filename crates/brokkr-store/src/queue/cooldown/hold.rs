//! A probe's hold on its route (fix round 0106b1r4, F1; the operator's
//! ruling of 2026-11-18, amending decision 0103 ruling 2): one version 7
//! `probe_holds` row, written before the probe is reserved, that the pass
//! and the claim read as a cool-down through the one in-force read
//! ([`super::in_force`]) while it is unended. A refusal's renewed round
//! ends it in the transaction that writes the round; an answer, or the
//! settlement of the probe's process tree, ends it by its row (fix round
//! 0106b1r10, H1). Nothing else does (fix round 0106b1r12, S1): no stored
//! end, margin, clock or run lock is read to end it, so a hold no
//! settlement ended stands through any journal name, a crash, and a lock
//! missing, free or replaced. Its stored end is kept as a readout only.

use rusqlite::{params, Connection, OptionalExtension};
use time::OffsetDateTime;

use super::{time_of, Cooldown, LAST_SECOND};
use crate::queue::storage::cool_downs_writable;
use crate::queue::{QueueRefusal, UnwrittenHold};
use crate::{patiently, Store, StoreError};

/// The hold one probe takes on its route before it is reserved: the run
/// and site it probes for, the probe's own attempt, the provider and route
/// it asks, and the deadline its own spawn runs under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeHold<'a> {
    pub run_id: &'a str,
    /// The probe's own attempt: the driver instance that holds the route,
    /// so a later drive of the same run never ends or adopts an earlier
    /// instance's hold (fix round 0106b1r12, S3).
    pub attempt_id: &'a str,
    /// The invocation-site tag, `''` for an inline seat.
    pub site: &'a str,
    pub provider: &'a str,
    pub route: &'a str,
    /// The end the hold's row shows, kept to the first whole second after
    /// it: a readout only, which no gate reads to end the hold (fix round
    /// 0106b1r12, S1).
    pub fallback: OffsetDateTime,
}

/// A written hold, by its row: what the probe's answer, or its renewed
/// round, ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeHoldId(i64);

/// The first whole second strictly after `fallback`, never before it (fix
/// round 0106b1r6, D1). `None` where that second is past [`LAST_SECOND`]:
/// it is never clamped there, which would end the hold before its
/// fallback (fix round 0106b1r9, E2).
fn after_second(fallback: OffsetDateTime) -> Option<i64> {
    let end = fallback.unix_timestamp() + 1;
    (end <= LAST_SECOND).then_some(end)
}

/// Whether a hold has ended: the one predicate both gates read a hold by,
/// through the in-force read (fix round 0106b1r12, S1). Only its ended
/// stamp decides, which its answer, its renewed round's commit, or the
/// settlement of its probe's process tree writes, by the engine or by an
/// operator; its stored end, any clock and any lock state are never read.
pub(super) fn hold_ended(ended_at: Option<i64>) -> bool {
    ended_at.is_some()
}

/// One `probe_holds` row as the in-force read selects it.
struct HoldRow {
    seq: i64,
    run_id: String,
    attempt_id: String,
    site: String,
    provider: String,
    route: String,
    deadline: i64,
    recorded: i64,
    ended: Option<i64>,
}

impl HoldRow {
    /// The row as the in-force read joins it: its stored end is its end, a
    /// readout, and its words name the probing run, the attempt holding it
    /// and the evidence that alone ends it.
    fn cooldown(self) -> Result<Cooldown, StoreError> {
        Ok(Cooldown {
            until_at: time_of(self.deadline, self.seq)?,
            reason: format!(
                "a cool-down probe of run {} (attempt {}) holds the route until it answers, its \
                 renewed cool-down commits, or its process tree is settled",
                self.run_id, self.attempt_id
            ),
            site: self.site,
            provider: self.provider,
            route: self.route,
            retry_after_seconds: None,
            // A hold is no round of any episode.
            rounds: 0,
            recorded_at: time_of(self.recorded, self.seq)?,
        })
    }
}

/// Every hold standing, as the rows the in-force read joins: each hold
/// [`hold_ended`] does not read as ended, whatever the time. A queue that
/// holds no version 7 table holds no hold.
pub(super) fn standing(conn: &Connection) -> Result<Vec<Cooldown>, StoreError> {
    if !cool_downs_writable(conn)? {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(
        "SELECT seq, run_id, attempt_id, site, provider, route, deadline_at, recorded_at,
                ended_at
         FROM probe_holds ORDER BY seq",
    )?;
    let raw = statement.query_map([], |row| {
        Ok(HoldRow {
            seq: row.get(0)?,
            run_id: row.get(1)?,
            attempt_id: row.get(2)?,
            site: row.get(3)?,
            provider: row.get(4)?,
            route: row.get(5)?,
            deadline: row.get(6)?,
            recorded: row.get(7)?,
            ended: row.get(8)?,
        })
    })?;
    let rows = raw.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .filter(|row| !hold_ended(row.ended))
        .map(HoldRow::cooldown)
        .collect()
}

/// End the hold `id` at `at`, if it still stands: a hold ends once, and
/// one already ended is left as it was. `None` names no hold: a NULL seq
/// matches no row, so no branch decides it.
pub(super) fn ended(conn: &Connection, id: Option<ProbeHoldId>, at: i64) -> Result<(), StoreError> {
    conn.execute(
        "UPDATE probe_holds SET ended_at = ?2 WHERE seq = ?1 AND ended_at IS NULL",
        params![id.map(|id| id.0), at],
    )?;
    Ok(())
}

impl Store {
    /// Write `hold` at `now`, before its probe is reserved, in its own
    /// `Immediate` transaction: the id is the evidence of the write, and
    /// every other end refuses by name, so the engine parks rather than
    /// probing an unheld route. A queue that holds no table this binary
    /// may write refuses ([`QueueRefusal::ProbeHoldUnstorable`]; fix round
    /// 0106b1r9, E3). An insert the table took without writing its row, as
    /// a trigger that ignores it leaves it, refuses
    /// ([`QueueRefusal::ProbeHoldUnwritten`]), so no probe runs on a hold
    /// believed written (fix round 0106b1r6, D2). A hold whose end cannot
    /// be stored strictly after its deadline refuses before any write
    /// ([`QueueRefusal::ProbeHoldUnending`]; fix round 0106b1r9, E2).
    pub fn hold_probe(
        &mut self,
        hold: &ProbeHold<'_>,
        now: OffsetDateTime,
    ) -> Result<ProbeHoldId, StoreError> {
        let unwritten = || UnwrittenHold {
            run_id: hold.run_id.to_string(),
            provider: hold.provider.to_string(),
            route: hold.route.to_string(),
        };
        let Some(ends) = after_second(hold.fallback) else {
            return Err(QueueRefusal::ProbeHoldUnending(Box::new(unwritten())).into());
        };
        patiently("hold_probe", self.patience, || {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if !cool_downs_writable(&tx)? {
                return Err(QueueRefusal::ProbeHoldUnstorable(Box::new(unwritten())).into());
            }
            // The hold's id is the row this INSERT returns, never the
            // connection's last rowid, which an ignored insert leaves at an
            // earlier row's (fix round 0106b1r6, D2).
            let written = tx
                .query_row(
                    "INSERT INTO probe_holds
                        (run_id, attempt_id, site, provider, route, recorded_at, deadline_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     RETURNING seq",
                    params![
                        hold.run_id,
                        hold.attempt_id,
                        hold.site,
                        hold.provider,
                        hold.route,
                        now.unix_timestamp(),
                        ends,
                    ],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(seq) = written else {
                return Err(QueueRefusal::ProbeHoldUnwritten(Box::new(unwritten())).into());
            };
            tx.commit()?;
            Ok(ProbeHoldId(seq))
        })
    }

    /// End the hold `id` at `now`: the probe answered, so its route opens
    /// at once. Only the hold the probe wrote is touched, and `None`, a
    /// probe on a queue that took no hold, touches none.
    pub fn end_probe_hold(
        &mut self,
        id: Option<ProbeHoldId>,
        now: OffsetDateTime,
    ) -> Result<(), StoreError> {
        patiently("end_probe_hold", self.patience, || {
            ended(&self.conn, id, now.unix_timestamp())
        })
    }
}
