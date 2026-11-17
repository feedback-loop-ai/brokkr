//! A provider's cool-downs in the queue (decision 0103 ruling 2(a), slice
//! 4d1): one append-only `provider_cooldowns` row per refused site and
//! round of an attempt's episode, written in the terminal event's own
//! `Immediate` transaction, read as the aggregate that holds a provider
//! and route, and read again inside a claim's transaction.
//!
//! The table's two times are Unix seconds (UTC), never text: a text time
//! compared as a string is wrong across precisions and offsets, and a
//! `unixepoch(text)` CHECK passes on malformed text because the function
//! returns NULL and a NULL CHECK passes. Only the readers here convert
//! them, into the `time` type the engine already carries.

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::provider_waits;
use rusqlite::{params, Connection, OptionalExtension};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::storage::{cool_downs_held, cool_downs_writable};
use super::QueueRefusal;
use crate::{patiently, Store, StoreError};

/// One cool-down a terminal event records, as its writer derives it. The
/// store fills `event_seq` and `recorded_at` from the event it writes the
/// rows with — the seq the event takes inside that transaction and the
/// instant the journal recorded it — so a caller never guesses either, and
/// a row whose end has already passed writes no row at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cooling {
    /// The invocation-site tag, `''` for an inline seat.
    pub site: String,
    pub provider: String,
    pub route: String,
    /// When the wait ends; a row is in force until this instant passes.
    pub until_at: OffsetDateTime,
    /// The provider's bounded words, as the terminal event recorded them.
    pub reason: String,
    /// The provider's own retry-after, when it gave one.
    pub retry_after_seconds: Option<u64>,
    /// Which round of this site's waits within the effect's episode.
    pub rounds: u64,
}

/// One `provider_cooldowns` row as a reader holds it: the cool-down the
/// writer recorded, with the instant the journal recorded it. The run and
/// the event seq are the row's own identity and are not carried here; a
/// caller that needs them reads the table by its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cooldown {
    pub site: String,
    pub provider: String,
    pub route: String,
    pub until_at: OffsetDateTime,
    pub reason: String,
    pub retry_after_seconds: Option<u64>,
    pub rounds: u64,
    pub recorded_at: OffsetDateTime,
}

/// The aggregate one provider and route stands at, at an instant: the
/// latest `until_at` across every unexpired row — a monotonic MAX, so a
/// later, shorter refusal never shortens a wait — and the words of the row
/// that ends last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownAggregate {
    pub until: OffsetDateTime,
    pub reason: String,
}

/// One probe round appended to a terminal event's episode (decision 0103
/// ruling 2(a), slice 4d2b). The event is the terminal whose `waits` entry
/// names `site`; the store fills the row's `recorded_at` from `until_at`
/// and `seconds`, so the two stored times differ by exactly the length the
/// engine computed, and the episode reads the round back from the table
/// rather than recomputing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooldownRound<'a> {
    pub run_id: &'a str,
    /// The terminal event's seq: the probe's row shares it with its attempt.
    pub event_seq: u64,
    /// The invocation-site tag, `''` for an inline seat.
    pub site: &'a str,
    pub provider: &'a str,
    pub route: &'a str,
    /// When this round's wait ends.
    pub until_at: OffsetDateTime,
    /// The provider's bounded words from the probe's refusal.
    pub reason: &'a str,
    pub retry_after_seconds: Option<u64>,
    /// The site's next round: the highest stored round plus one.
    pub rounds: u64,
    /// The length this round's wait was computed at.
    pub seconds: u64,
}

/// What [`Store::append_cooldown_round`] did. A queue this binary may not
/// write takes no row and the engine falls back to the 4c park; a row the
/// store refused is a [`StoreError`] instead, and nothing was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundAppend {
    Written,
    NoTable,
}

/// Write `rows` as the cool-downs of `run_id`'s event `event_seq`, inside
/// the transaction that wrote that event, which the journal recorded at
/// `recorded_at`. A row whose end has already passed is history and is
/// written not at all; a row that violates the table's CHECK constraints
/// or its unique index fails the whole transaction, so the event and every
/// row land together or not at all. No rows, or a queue that holds no
/// table this binary may write, leaves the table untouched and the event
/// alone lands.
pub(crate) fn insert(
    tx: &Connection,
    run_id: &str,
    event_seq: u64,
    recorded_at: &str,
    rows: &[Cooling],
) -> Result<(), StoreError> {
    // Every append that carries no cool-downs — every non-terminal event,
    // and `append_next`/`append_next_if_head` — leaves the table alone, so
    // a journal whose table is gone or whose queue a newer binary stores
    // still takes its event.
    if rows.is_empty() {
        return Ok(());
    }
    // A row belongs to version 4's table. A version 1, 2 or 3 queue holds
    // no table, and a version this binary does not know is a newer
    // binary's, left as found: either way the event alone lands and its
    // `waits` stay the attempt's record (decision 0103 ruling 2(a)).
    if !cool_downs_writable(tx)? {
        return Ok(());
    }
    let recorded = OffsetDateTime::parse(recorded_at, &Rfc3339)
        .expect("the journal's own recorded_at is RFC 3339")
        .unix_timestamp();
    for row in rows {
        let until = until_second(row.until_at);
        if until <= recorded {
            continue;
        }
        insert_row(
            tx,
            &StoredRow {
                run_id,
                event_seq,
                site: &row.site,
                provider: &row.provider,
                route: &row.route,
                until,
                reason: &row.reason,
                retry_after_seconds: row.retry_after_seconds,
                rounds: row.rounds,
                recorded,
            },
        )?;
    }
    Ok(())
}

/// One `provider_cooldowns` row as the table's one writer spells it:
/// shared by an attempt's rows and a probe's later round, so the INSERT
/// lives once (decision 0071 ruling 5).
struct StoredRow<'a> {
    run_id: &'a str,
    event_seq: u64,
    site: &'a str,
    provider: &'a str,
    route: &'a str,
    until: i64,
    reason: &'a str,
    retry_after_seconds: Option<u64>,
    rounds: u64,
    recorded: i64,
}

/// Insert one row. The table's CHECK constraints and unique index judge it.
fn insert_row(conn: &Connection, row: &StoredRow<'_>) -> Result<(), StoreError> {
    conn.execute(
        "INSERT INTO provider_cooldowns
            (run_id, event_seq, site, provider, route, until_at, reason,
             retry_after_seconds, rounds, recorded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            row.run_id,
            row.event_seq as i64,
            row.site,
            row.provider,
            row.route,
            row.until,
            row.reason,
            row.retry_after_seconds.map(|retry| retry as i64),
            row.rounds as i64,
            row.recorded,
        ],
    )?;
    Ok(())
}

/// The last whole second an RFC 3339 instant can name:
/// `9999-12-31T23:59:59Z`. A stored end is bounded here, not by whatever
/// range `time` happens to be built with: `large-dates` moves that range
/// from build to build, and a row this reader cannot spell refuses.
const LAST_SECOND: i64 = 253_402_300_799;

/// `at`'s whole second, rounded UP, and never past [`LAST_SECOND`]: a
/// stored end never falls before the instant it names, so a reader that
/// floors `now` never sees a wait end early. A wait is lengthened by under
/// a second, never shortened. An end in the last second's fraction rounds
/// to that second, the latest this binary can spell, rather than past it
/// into a value no reader can name.
fn until_second(at: OffsetDateTime) -> i64 {
    let seconds = at.unix_timestamp();
    let rounded = if at.nanosecond() == 0 {
        seconds
    } else {
        seconds + 1
    };
    rounded.min(LAST_SECOND)
}

/// The columns every cool-down reader selects, in the order [`Raw`] reads
/// them: one home for the projection, so the in-force read and the
/// readout's newest read cannot drift apart.
const ROWS: &str = "seq, site, provider, route, until_at, reason, retry_after_seconds, rounds,
                recorded_at";

/// Every row in force at `now`, in the order it was written: the claim's
/// judge and the aggregate share this reading. A row whose stored instant
/// this reader cannot spell refuses by name, carrying the row's `seq`.
pub(super) fn in_force(conn: &Connection, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    let sql = format!("SELECT {ROWS} FROM provider_cooldowns WHERE until_at > ?1 ORDER BY seq");
    read_rows(conn, &sql, now)
}

/// Every row in force at `now`, and, for a provider and route no row holds
/// any more, the newest row that ended it: what a readout paints, so a
/// probe's later end survives the instant it passes instead of reverting
/// to the event's earlier one (decision 0103 slice 4f). Admission reads
/// [`in_force`], which the expired row never joins. A row whose stored
/// instant this reader cannot spell refuses by name, carrying the row's
/// `seq`.
fn newest(conn: &Connection, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    newest_with(conn, now, |_| {})
}

/// [`newest`]'s one statement (#1112): one scan of the table in `seq`
/// order, its primary key, so SQLite sorts nothing and holds no row past
/// the one it hands over. [`Newest`] decides which rows are kept. The
/// statement's VM steps go to `stepped`, so the test that binds its cost
/// measures this statement.
fn newest_with(
    conn: &Connection,
    now: i64,
    stepped: impl FnOnce(i64),
) -> Result<Vec<Cooldown>, StoreError> {
    let sql = format!("SELECT {ROWS} FROM provider_cooldowns ORDER BY seq");
    let mut statement = conn.prepare(&sql)?;
    let mut newest = Newest::default();
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        newest.take(Raw::read(row)?, now);
    }
    drop(rows);
    stepped(i64::from(
        statement.get_status(rusqlite::StatementStatus::VmStep),
    ));
    newest.kept().into_iter().map(Raw::typed).collect()
}

/// Where one provider and route stands in [`Newest`]'s scan so far.
enum Standing {
    /// A row in force holds it: no expired row of it is kept.
    Held,
    /// Every row of it read so far has ended, and this is the newest.
    Ended(Raw),
}

/// The readout's rows, decided over a scan in `seq` order: every row in
/// force at `now`, and, for a provider and route no such row holds, the
/// row with the latest end, the later-written on a tie. A row is kept once
/// at most, and the rows a scan has passed over are not held.
#[derive(Default)]
struct Newest {
    in_force: Vec<Raw>,
    routes: HashMap<(String, String), Standing>,
}

impl Newest {
    /// Take the scan's next row, which is later-written than every row
    /// taken before it.
    fn take(&mut self, row: Raw, now: i64) {
        let key = (row.provider.clone(), row.route.clone());
        if row.until > now {
            self.routes.insert(key, Standing::Held);
            self.in_force.push(row);
            return;
        }
        match self.routes.entry(key) {
            Entry::Vacant(first) => {
                first.insert(Standing::Ended(row));
            }
            Entry::Occupied(mut seen) => {
                let later =
                    matches!(seen.get(), Standing::Ended(newest) if row.until >= newest.until);
                if later {
                    seen.insert(Standing::Ended(row));
                }
            }
        }
    }

    /// The kept rows in the order they were written.
    fn kept(self) -> Vec<Raw> {
        let mut kept = self.in_force;
        kept.extend(
            self.routes
                .into_values()
                .filter_map(|standing| match standing {
                    Standing::Ended(row) => Some(row),
                    Standing::Held => None,
                }),
        );
        kept.sort_by_key(|row| row.seq);
        kept
    }
}

/// The VM steps [`newest`]'s one statement takes at `now`, for the test
/// that holds it to one scan.
#[cfg(test)]
fn newest_steps(conn: &Connection, now: i64) -> Result<i64, StoreError> {
    let mut steps = 0;
    newest_with(conn, now, |taken| steps = taken)?;
    Ok(steps)
}

/// The rows `sql` selects at `now`, each read and typed once: the one
/// home of the prepare, the query and the refusal-by-row the in-force
/// readers share.
fn read_rows(conn: &Connection, sql: &str, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    let mut statement = conn.prepare(sql)?;
    let raw = statement.query_map([now], Raw::read)?;
    let rows = raw.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(Raw::typed).collect()
}

/// A row's columns as SQLite hands them over, before its instants are
/// turned into the `time` type: kept apart so a row this reader cannot
/// name refuses by its `seq` ([`time_of`]) instead of being guessed at.
struct Raw {
    seq: i64,
    site: String,
    provider: String,
    route: String,
    until: i64,
    reason: String,
    retry_after: Option<i64>,
    rounds: i64,
    recorded: i64,
}

impl Raw {
    fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Raw> {
        Ok(Raw {
            seq: row.get(0)?,
            site: row.get(1)?,
            provider: row.get(2)?,
            route: row.get(3)?,
            until: row.get(4)?,
            reason: row.get(5)?,
            retry_after: row.get(6)?,
            rounds: row.get(7)?,
            recorded: row.get(8)?,
        })
    }

    fn typed(self) -> Result<Cooldown, StoreError> {
        Ok(Cooldown {
            site: self.site,
            provider: self.provider,
            route: self.route,
            until_at: time_of(self.until, self.seq)?,
            reason: self.reason,
            retry_after_seconds: self.retry_after.map(|retry| retry as u64),
            rounds: self.rounds as u64,
            recorded_at: time_of(self.recorded, self.seq)?,
        })
    }
}

/// The aggregate `provider` and `route` stand at, at `now`, or `None` when
/// no row is in force: every unexpired row holds the lane, and the lane
/// ends at the latest `until_at`.
fn aggregate(
    conn: &Connection,
    provider: &str,
    route: &str,
    now: i64,
) -> Result<Option<CooldownAggregate>, StoreError> {
    let found: Option<(i64, i64, String)> = conn
        .query_row(
            "SELECT until_at, seq, reason FROM provider_cooldowns
             WHERE provider = ?1 AND route = ?2 AND until_at > ?3
             ORDER BY until_at DESC, seq DESC LIMIT 1",
            params![provider, route, now],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((until, seq, reason)) = found else {
        return Ok(None);
    };
    Ok(Some(CooldownAggregate {
        until: time_of(until, seq)?,
        reason,
    }))
}

/// A stored instant this reader can name, or the row's refusal: a whole
/// number of seconds outside `time`'s range is never guessed at.
fn time_of(seconds: i64, seq: i64) -> Result<OffsetDateTime, StoreError> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| QueueRefusal::CooldownTime { seq }.into())
}

/// One site's rounds on an event, in round order: the attempt's own row and
/// every probe round appended after it. The version is judged before this
/// runs, as the aggregate's read is.
fn rounds(
    conn: &Connection,
    run_id: &str,
    event_seq: u64,
    site: &str,
) -> Result<Vec<Cooldown>, StoreError> {
    let mut statement = conn.prepare(
        "SELECT seq, site, provider, route, until_at, reason, retry_after_seconds, rounds,
                recorded_at
         FROM provider_cooldowns
         WHERE run_id = ?1 AND event_seq = ?2 AND site = ?3
         ORDER BY rounds",
    )?;
    let raw = statement.query_map(params![run_id, event_seq as i64, site], Raw::read)?;
    let rows = raw.collect::<Result<Vec<_>, _>>()?;
    rows.into_iter().map(Raw::typed).collect()
}

/// The event at `(run_id, event_seq)`, parsed, or `None`: a probe round is
/// appended to an event that already landed, so the check reads it inside
/// the transaction that writes the row.
fn event_at(
    conn: &Connection,
    run_id: &str,
    event_seq: u64,
) -> Result<Option<EventEnvelope>, StoreError> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT envelope FROM events WHERE run_id = ?1 AND seq = ?2",
            params![run_id, event_seq as i64],
            |row| row.get(0),
        )
        .optional()?;
    raw.map(|raw| serde_json::from_str(&raw))
        .transpose()
        .map_err(StoreError::from)
}

impl Store {
    /// The aggregate `provider` and `route` stand at, at `now`: every
    /// unexpired row is in force and the wait ends at the latest
    /// `until_at` across them (decision 0103 ruling 2(a)). An expired row
    /// is history and is never deleted; `None` is a route no row holds.
    /// The queue's version is judged first: a version 1, 2 or 3 journal
    /// holds no cool-down and answers `None`, and a version this binary
    /// does not know refuses.
    pub fn cooldown(
        &self,
        provider: &str,
        route: &str,
        now: OffsetDateTime,
    ) -> Result<Option<CooldownAggregate>, StoreError> {
        patiently("cooldown", self.patience, || {
            if cool_downs_held(&self.conn)? {
                aggregate(&self.conn, provider, route, now.unix_timestamp())
            } else {
                Ok(None)
            }
        })
    }

    /// Every cool-down row in force at `now`, in the order it was written:
    /// the rows the claim's judge reads inside its transaction. The
    /// queue's version is judged first, as [`Store::cooldown`] does.
    pub fn cooldowns_in_force(&self, now: OffsetDateTime) -> Result<Vec<Cooldown>, StoreError> {
        patiently("cooldowns_in_force", self.patience, || {
            if cool_downs_held(&self.conn)? {
                in_force(&self.conn, now.unix_timestamp())
            } else {
                Ok(Vec::new())
            }
        })
    }

    /// Every cool-down row in force at `now`, and the newest row of every
    /// provider and route no unexpired row holds any more, in the order
    /// they were written: what a readout paints, so a probe's later end
    /// survives the instant it passes and the event's earlier end never
    /// returns (decision 0103 slice 4f). newest's one statement; admission
    /// keeps reading [`Store::cooldown`] and [`Store::cooldowns_in_force`],
    /// which an expired row never holds. The queue's version is judged
    /// first, as [`Store::cooldown`] does. The statement is one scan of the
    /// table in its primary key's order (#1112), so SQLite sorts nothing and
    /// its memory does not follow the table; the read keeps each provider
    /// and route's newest row so far as it goes, so the bound is `n` steps
    /// in the table's rows, never the run or its events, plus a sort of the
    /// kept rows by `seq`. What the read holds is the rows in force and one
    /// row per provider and route, the rows it returns. The history a
    /// journal keeps is the rate limits its runs met, so `n` is that count
    /// and not a run's size; the console pays the read twice per refresh,
    /// once to compare the kept frame's aggregate and once in the frame or
    /// whole read.
    pub fn cooldowns_newest(&self, now: OffsetDateTime) -> Result<Vec<Cooldown>, StoreError> {
        patiently("cooldowns_newest", self.patience, || {
            if cool_downs_held(&self.conn)? {
                newest(&self.conn, now.unix_timestamp())
            } else {
                Ok(Vec::new())
            }
        })
    }

    /// One site's rounds on a terminal event, in round order: the attempt's
    /// own row and every probe round appended after it. The episode reads
    /// its probe rounds here instead of recomputing them under today's
    /// bounds (decision 0103 ruling 2(a), slice 4d2b). The queue's version
    /// is judged first, as [`Store::cooldown`] does: a version 1, 2 or 3
    /// journal holds no cool-down and answers none.
    pub fn cooldown_rounds(
        &self,
        run_id: &str,
        event_seq: u64,
        site: &str,
    ) -> Result<Vec<Cooldown>, StoreError> {
        patiently("cooldown_rounds", self.patience, || {
            if cool_downs_held(&self.conn)? {
                rounds(&self.conn, run_id, event_seq, site)
            } else {
                Ok(Vec::new())
            }
        })
    }

    /// Append ONE probe round to its terminal event, in its own `Immediate`
    /// transaction (decision 0103 ruling 2(a), slice 4d2b). The queue must
    /// be version 4 (`NoTable` otherwise, and the engine parks as 4c does);
    /// the event must be a terminal effect event carrying a readable `waits`
    /// entry for the round's site; and the round must be exactly one more
    /// than the site's highest stored round for that `(run_id, event_seq)`,
    /// so a lost race writes neither a gap nor a duplicate. The table's
    /// CHECK constraints and unique index judge the row as they judge an attempt's
    /// rows.
    pub fn append_cooldown_round(
        &mut self,
        round: CooldownRound<'_>,
    ) -> Result<RoundAppend, StoreError> {
        patiently("append_cooldown_round", self.patience, || {
            let tx = self
                .conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            if !cool_downs_writable(&tx)? {
                return Ok(RoundAppend::NoTable);
            }
            let absent = || QueueRefusal::CooldownRoundEvent {
                run_id: round.run_id.to_string(),
                event_seq: round.event_seq,
            };
            let Some(event) = event_at(&tx, round.run_id, round.event_seq)? else {
                return Err(absent().into());
            };
            if !matches!(
                event.event_type,
                EventType::EffectFailed | EventType::EffectIndeterminate
            ) {
                return Err(absent().into());
            }
            let waits = provider_waits(&event).ok().flatten().unwrap_or_default();
            if !waits
                .iter()
                .any(|wait| wait.site.as_deref().unwrap_or("") == round.site)
            {
                return Err(QueueRefusal::CooldownRoundWait {
                    run_id: round.run_id.to_string(),
                    event_seq: round.event_seq,
                    site: round.site.to_string(),
                }
                .into());
            }
            let highest: i64 = tx.query_row(
                "SELECT COALESCE(MAX(rounds), 0) FROM provider_cooldowns
                 WHERE run_id = ?1 AND event_seq = ?2 AND site = ?3",
                params![round.run_id, round.event_seq as i64, round.site],
                |row| row.get(0),
            )?;
            let standing = u64::try_from(highest).unwrap_or(0);
            if round.rounds != standing.saturating_add(1) {
                return Err(QueueRefusal::CooldownRoundNotNext(Box::new(
                    super::CooldownRoundMismatch {
                        run_id: round.run_id.to_string(),
                        event_seq: round.event_seq,
                        site: round.site.to_string(),
                        rounds: round.rounds,
                        standing,
                    },
                ))
                .into());
            }
            // The row's two times differ by exactly the round's computed
            // length: `recorded_at` is the round's start, so the episode
            // sums the row back without re-deriving it under today's
            // bounds (decision 0071 ruling 5, one home for the length).
            let until = until_second(round.until_at);
            let recorded = until.saturating_sub(i64::try_from(round.seconds).unwrap_or(i64::MAX));
            insert_row(
                &tx,
                &StoredRow {
                    run_id: round.run_id,
                    event_seq: round.event_seq,
                    site: round.site,
                    provider: round.provider,
                    route: round.route,
                    until,
                    reason: round.reason,
                    retry_after_seconds: round.retry_after_seconds,
                    rounds: round.rounds,
                    recorded,
                },
            )?;
            tx.commit()?;
            Ok(RoundAppend::Written)
        })
    }
}

#[cfg(test)]
mod tests;
