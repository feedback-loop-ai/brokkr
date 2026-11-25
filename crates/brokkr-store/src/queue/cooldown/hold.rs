//! A probe's hold on its route (fix round 0106b1r4, F1; the operator's
//! ruling of 2026-11-18, amending decision 0103 ruling 2): one version 7
//! `probe_holds` row, written once the probe is reserved and before it is
//! dispatched, that the pass
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

use super::{aggregate, time_of, Cooldown, LAST_SECOND};
use crate::queue::storage::{cool_downs_held, cool_downs_writable};
use crate::queue::{QueueRefusal, UnsettledHold, UnwrittenHold};
use crate::{chain, patiently, CooldownAppend, Landed, Store, StoreError};

/// The hold one probe takes on its route before it is dispatched: the run
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
    /// The probe's deadline, which the hold's row shows as its end, kept
    /// to the first whole second after it: a readout only, which no gate
    /// reads to end the hold (fix round 0106b1r12, S1).
    pub deadline: OffsetDateTime,
}

/// A written hold, by its row: what the probe's answer, or its renewed
/// round, ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeHoldId(i64);

/// The first whole second strictly after `deadline`, never before it (fix
/// round 0106b1r6, D1). `None` where that second is past [`LAST_SECOND`]:
/// it is never clamped there, which would show the hold ending before its
/// probe's deadline (fix round 0106b1r9, E2).
fn after_second(deadline: OffsetDateTime) -> Option<i64> {
    let end = deadline.unix_timestamp() + 1;
    (end <= LAST_SECOND).then_some(end)
}

/// A hold that stands, as SQL: the one predicate every read and every end
/// of a hold selects by, at the storage edge, so a pass and a claim read
/// only the standing rows and never materialize the history (fix round
/// 0106b1r12, S1, and its return). Only the ended stamp decides, which its
/// answer, its renewed round's commit, or the settlement of its probe's
/// process tree writes, by the engine or by an operator; its stored end,
/// any clock and any lock state are never read.
macro_rules! standing_hold {
    () => {
        "ended_at IS NULL"
    };
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
            probe: true,
        })
    }
}

/// The standing holds' select: only the rows [`standing_hold`] names, read
/// through version 7's partial index over them.
pub(super) const STANDING: &str = concat!(
    "SELECT seq, run_id, attempt_id, site, provider, route, deadline_at, recorded_at
     FROM probe_holds WHERE ",
    standing_hold!(),
    " ORDER BY seq"
);

/// Every hold standing, as the rows the in-force read joins: each row
/// [`standing_hold`] selects, whatever the time, and no other. A queue that
/// holds no version 7 table holds no hold.
pub(super) fn standing(conn: &Connection) -> Result<Vec<Cooldown>, StoreError> {
    if !cool_downs_writable(conn)? {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(STANDING)?;
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
        })
    })?;
    let rows = raw.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(HoldRow::cooldown).collect()
}

/// End the hold `id` at `at`, if it still stands, inside the caller's
/// transaction: a hold ends once, and one already ended, or absent, is left
/// as it was. `None` names no hold: a NULL seq matches no row, so no branch
/// decides it. Whatever the update reports, the hold is read again in the
/// same transaction, and one still standing refuses
/// ([`QueueRefusal::ProbeHoldUnended`]), so an end the table did not write
/// is never reported as written, and the caller's transaction, a renewed
/// round's included, rolls back whole (fix round 0106b1r13, M2).
pub(super) fn ended(conn: &Connection, id: Option<ProbeHoldId>, at: i64) -> Result<(), StoreError> {
    const END_HOLD: &str = concat!(
        "UPDATE probe_holds SET ended_at = ?2 WHERE seq = ?1 AND ",
        standing_hold!()
    );
    let seq = id.map(|id| id.0);
    conn.execute(END_HOLD, params![seq, at])?;
    let standing: Option<(String, String)> = conn
        .query_row(
            concat!(
                "SELECT run_id, attempt_id FROM probe_holds WHERE seq = ?1 AND ",
                standing_hold!()
            ),
            [seq],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    standing.map_or(Ok(()), |(run_id, attempt_id)| {
        let unended = UnsettledHold { run_id, attempt_id };
        Err(QueueRefusal::ProbeHoldUnended(Box::new(unended)).into())
    })
}

/// Refused where a hold of the run `run_id` naming one of `attempts` still
/// stands after its settlement's updates: the first such attempt, by name.
fn unsettled(conn: &Connection, run_id: &str, attempts: &[String]) -> Result<(), StoreError> {
    const COUNT_STANDING: &str = concat!(
        "SELECT COUNT(*) FROM probe_holds WHERE run_id = ?1 AND attempt_id = ?2 AND ",
        standing_hold!()
    );
    for attempt in attempts {
        let standing: i64 = conn.query_row(COUNT_STANDING, [run_id, attempt], |row| row.get(0))?;
        if standing > 0 {
            let unsettled = UnsettledHold {
                run_id: run_id.to_string(),
                attempt_id: attempt.clone(),
            };
            return Err(QueueRefusal::ProbeHoldUnsettled(Box::new(unsettled)).into());
        }
    }
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
    /// ([`QueueRefusal::ProbeHoldUnending`]; fix round 0106b1r9, E2). A
    /// route another standing hold or an unexpired cool-down holds at `now`,
    /// as [`Store::cooldown`] reads it, refuses in the same transaction
    /// ([`QueueRefusal::ProbeHoldContended`]; fix round 0106b1r13, H2).
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
        let Some(ends) = after_second(hold.deadline) else {
            return Err(QueueRefusal::ProbeHoldUnending(Box::new(unwritten())).into());
        };
        patiently("hold_probe", self.patience, &self.path, || {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if !cool_downs_writable(&tx)? {
                return Err(QueueRefusal::ProbeHoldUnstorable(Box::new(unwritten())).into());
            }
            // The route as every reader judges it, read in this write's own
            // transaction: another standing hold or an unexpired row holds
            // it, so no second hold is written, and no reader can admit
            // between the read and the insert (fix round 0106b1r13, H2).
            let at = now.unix_timestamp();
            if aggregate(&tx, hold.provider, hold.route, at)?.is_some() {
                return Err(QueueRefusal::ProbeHoldContended(Box::new(unwritten())).into());
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

    /// Journal `event`, a retry's `effect/started`, only where none of
    /// `routes`, each a provider and route its waits cooled, stands at
    /// `now`: every route is read through the in-force read the gates and
    /// [`Store::hold_probe`] read, in the `Immediate` transaction that
    /// appends the event, so the start's one decision covers all its routes
    /// in one snapshot and is serialized with every hold's write. A hold
    /// that commits first refuses the start
    /// ([`QueueRefusal::StartHeld`]), writing nothing; a start that commits
    /// first was admitted before that hold existed, and its launch follows
    /// the event this wrote (fix round 0106b1r14, R1).
    pub fn append_clear(
        &mut self,
        event: CooldownAppend<'_>,
        routes: &[(&str, &str)],
        now: OffsetDateTime,
    ) -> Result<Landed, StoreError> {
        let at = now.unix_timestamp();
        let clear = |tx: &Connection| {
            if !cool_downs_held(tx)? {
                return Ok(());
            }
            for (provider, route) in routes {
                if aggregate(tx, provider, route, at)?.is_some() {
                    let held = UnwrittenHold {
                        run_id: event.run_id.to_string(),
                        provider: provider.to_string(),
                        route: route.to_string(),
                    };
                    return Err(QueueRefusal::StartHeld(Box::new(held)).into());
                }
            }
            Ok(())
        };
        let (conn, carried) = (&mut self.conn, self.carried.as_ref());
        let (landed, settled) = patiently("append", self.patience, &self.path, || {
            chain::append_once((conn, carried), &event, &[], &clear)
        })?;
        self.settle((event.run_id, 1), settled);
        Ok(landed)
    }

    /// End, at `now`, every hold standing for one of `attempts` of the run
    /// `run_id`: the attempts a settlement proved, whose probes' process
    /// trees are therefore over (fix round 0106b1r12, S2). The restarted
    /// engine's reaper and the operator's settle record are its callers; a
    /// hold naming any other attempt, of this run or another, is left as
    /// it stands. A queue that holds no version 7 table holds no hold. An
    /// end the table took without writing it, so that a hold of a settled
    /// attempt still stands once the updates ran, refuses
    /// ([`QueueRefusal::ProbeHoldUnsettled`]) and writes none of them: a
    /// settlement is never reported as applied while its hold stands.
    pub fn settle_probe_holds(
        &mut self,
        run_id: &str,
        attempts: &[String],
        now: OffsetDateTime,
    ) -> Result<(), StoreError> {
        patiently("settle_probe_holds", self.patience, &self.path, || {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if cool_downs_writable(&tx)? {
                for attempt in attempts {
                    tx.execute(
                        concat!(
                            "UPDATE probe_holds SET ended_at = ?3
                             WHERE run_id = ?1 AND attempt_id = ?2 AND ",
                            standing_hold!()
                        ),
                        params![run_id, attempt, now.unix_timestamp()],
                    )?;
                }
                unsettled(&tx, run_id, attempts)?;
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// End the hold `id` at `now`: the probe answered, so its route opens
    /// at once. Only the hold the probe wrote is touched, and `None`, a
    /// probe on a queue that took no hold, touches none. The end and its
    /// check are one `Immediate` transaction: a hold the update left
    /// standing refuses ([`QueueRefusal::ProbeHoldUnended`]) and stands
    /// (fix round 0106b1r13, M2).
    pub fn end_probe_hold(
        &mut self,
        id: Option<ProbeHoldId>,
        now: OffsetDateTime,
    ) -> Result<(), StoreError> {
        patiently("end_probe_hold", self.patience, &self.path, || {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            ended(&tx, id, now.unix_timestamp())?;
            tx.commit()?;
            Ok(())
        })
    }
}
