//! A probe's hold on its route (fix round 0106b1r4, F1; the operator's
//! ruling of 2026-11-18, amending decision 0103 ruling 2): one version 7
//! `probe_holds` row, written before the probe is reserved, that the pass
//! and the claim read as a cool-down through the one in-force read
//! ([`super::in_force`]) while it is unended and before its deadline. A
//! refusal's renewed round ends it in the transaction that writes the
//! round; an answer ends it at once; any other end leaves it standing
//! until its deadline, after which it holds nothing.

use rusqlite::{params, Connection, OptionalExtension};
use time::OffsetDateTime;

use super::{time_of, Cooldown, LAST_SECOND};
use crate::queue::storage::cool_downs_writable;
use crate::queue::{QueueRefusal, UnwrittenHold};
use crate::{patiently, Store, StoreError};

/// The hold one probe takes on its route before it is reserved: the run
/// and site it probes for, the provider and route it asks, and the
/// deadline its own spawn runs under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeHold<'a> {
    pub run_id: &'a str,
    /// The invocation-site tag, `''` for an inline seat.
    pub site: &'a str,
    pub provider: &'a str,
    pub route: &'a str,
    /// The instant the probe's driver is killed by. The hold stops
    /// counting at the first whole second after it, whatever became of the
    /// probe.
    pub deadline: OffsetDateTime,
}

/// A written hold, by its row: what the probe's answer, or its renewed
/// round, ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeHoldId(i64);

/// The first whole second strictly after `deadline`, never past
/// [`LAST_SECOND`]: a hold stored here ends later than the probe's driver
/// is killed, even on a deadline that falls on a whole second, where a
/// round-up would end both at once (fix round 0106b1r6, D1).
fn after_second(deadline: OffsetDateTime) -> i64 {
    deadline.unix_timestamp().saturating_add(1).min(LAST_SECOND)
}

/// The words a hold is read with, naming the run whose probe is in flight.
fn words(run_id: &str) -> String {
    format!("a cool-down probe of run {run_id} holds the route while it is in flight")
}

/// Every hold standing at `now`, unended and before its deadline, as the
/// rows the in-force read joins: its deadline is its end and its words name
/// the probing run. A queue that holds no version 7 table holds no hold.
pub(super) fn standing(conn: &Connection, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    if !cool_downs_writable(conn)? {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(
        "SELECT seq, run_id, site, provider, route, deadline_at, recorded_at FROM probe_holds
         WHERE ended_at IS NULL AND deadline_at > ?1 ORDER BY seq",
    )?;
    let raw = statement.query_map([now], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
        ))
    })?;
    let rows = raw.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(seq, run_id, site, provider, route, deadline, recorded)| {
            Ok(Cooldown {
                site,
                provider,
                route,
                until_at: time_of(deadline, seq)?,
                reason: words(&run_id),
                retry_after_seconds: None,
                // A hold is no round of any episode.
                rounds: 0,
                recorded_at: time_of(recorded, seq)?,
            })
        })
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
    /// `Immediate` transaction: `None` where the queue holds no table this
    /// binary may write, which the engine parks on rather than probing an
    /// unheld route. An insert the table took without writing its row, as
    /// a trigger that ignores it leaves it, refuses by name
    /// ([`QueueRefusal::ProbeHoldUnwritten`]), so no probe runs on a hold
    /// believed written (fix round 0106b1r6, D2).
    pub fn hold_probe(
        &mut self,
        hold: &ProbeHold<'_>,
        now: OffsetDateTime,
    ) -> Result<Option<ProbeHoldId>, StoreError> {
        patiently("hold_probe", self.patience, || {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if !cool_downs_writable(&tx)? {
                return Ok(None);
            }
            // The hold's id is the row this INSERT returns, never the
            // connection's last rowid, which an ignored insert leaves at an
            // earlier row's (fix round 0106b1r6, D2).
            let written = tx
                .query_row(
                    "INSERT INTO probe_holds
                        (run_id, site, provider, route, recorded_at, deadline_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     RETURNING seq",
                    params![
                        hold.run_id,
                        hold.site,
                        hold.provider,
                        hold.route,
                        now.unix_timestamp(),
                        after_second(hold.deadline),
                    ],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(seq) = written else {
                let unwritten = UnwrittenHold {
                    run_id: hold.run_id.to_string(),
                    provider: hold.provider.to_string(),
                    route: hold.route.to_string(),
                };
                return Err(QueueRefusal::ProbeHoldUnwritten(Box::new(unwritten)).into());
            };
            tx.commit()?;
            Ok(Some(ProbeHoldId(seq)))
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
