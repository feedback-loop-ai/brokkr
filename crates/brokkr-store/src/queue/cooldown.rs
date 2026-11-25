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

use std::collections::BTreeMap;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::{provider_waits, Held};
use rusqlite::{params, Connection, OptionalExtension};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::storage::{cool_downs_held, cool_downs_writable};
use super::QueueRefusal;
use crate::{patiently, Store, StoreError};

mod hold;
mod joined;
mod readout;
pub use hold::{ProbeHold, ProbeHoldId};
pub use joined::{CooldownAggregate, RowEnd};

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
    /// Version 7's kind (decision 0106 ruling 2): the held fact the writer
    /// parsed from the event's own `waits` entry when it derived this row,
    /// carried here so the payload is read once (fix round 0106b1r3).
    pub held: Option<Held>,
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
    /// Whether the row is a probe's hold on its route (fix round
    /// 0106b1r4, F1): no time ends it, only its answer, its renewed round
    /// or its tree's settlement, so `until_at` is its probe's deadline, a
    /// readout, never when the route opens (the return of fix round
    /// 0106b1r12). A `provider_cooldowns` row is never one.
    pub probe: bool,
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
    /// The probe's hold this round ends, in the round's own transaction
    /// (fix round 0106b1r4, F1): a round the store does not take ends none.
    pub ends: Option<ProbeHoldId>,
}

/// What [`Store::append_cooldown_round`] did. A queue this binary may not
/// write takes no row and the engine falls back to the 4c park; a row the
/// store refused is a [`StoreError`] instead, and nothing was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundAppend {
    Written,
    NoTable,
}

/// Write `rows` as the cool-downs of `event`, inside the transaction that
/// wrote that event. A row whose end has already passed is history and is
/// written not at all; a row that violates the table's CHECK constraints
/// or its unique index fails the whole transaction, so the event and every
/// row land together or not at all. No rows, or a queue that holds no
/// table this binary may write, leaves the table untouched and the event
/// alone lands. A row's kind is the one its writer derived with it from the
/// event's own `waits` entry for its site (decision 0106 ruling 2), so the
/// payload is parsed once, by that writer. What it returns is the store's
/// own evidence: the held rows it wrote, counting none it skipped (fix
/// round 0106b1r10, H4).
pub(crate) fn insert(
    tx: &Connection,
    event: &EventEnvelope,
    rows: &[Cooling],
) -> Result<usize, StoreError> {
    // Every append that carries no cool-downs — every non-terminal event,
    // and `append_next`/`append_next_if_head` — leaves the table alone, so
    // a journal whose table is gone or whose queue a newer binary stores
    // still takes its event.
    if rows.is_empty() {
        return Ok(0);
    }
    let recorded = OffsetDateTime::parse(&event.recorded_at, &Rfc3339)
        .expect("the journal's own recorded_at is RFC 3339")
        .unix_timestamp();
    // A row the table did not write rolls the event back with it: a
    // terminal never stands over a held or cooling row the store does not
    // hold (fix round 0106b1r6, D3).
    let unwritten = |row: &Cooling| {
        QueueRefusal::CooldownRowUnwritten(Box::new(super::UnwrittenRow {
            run_id: event.run_id.clone(),
            event_seq: event.seq,
            site: row.site.clone(),
            rounds: row.rounds,
        }))
    };
    // A row belongs to version 4's table. A version 1, 2 or 3 queue holds
    // no table, and a version this binary does not know is a newer
    // binary's, left as found: either way an ordinary wait's event alone
    // lands and its `waits` stay the attempt's record (decision 0103 ruling
    // 2(a)). A live held row is the only thing that holds its route, so
    // such a queue refuses its event as the table refuses an unwritten
    // row, and nothing is recorded as held that the store did not write
    // (fix round 0106b1r9, E3). An ordinary row it does not write is not
    // counted written either (fix round 0106b1r10, H4).
    if !cool_downs_writable(tx)? {
        let live_held =
            |row: &&Cooling| row.held.is_some() && until_second(row.until_at) > recorded;
        return rows
            .iter()
            .find(live_held)
            .map_or(Ok(0), |row| Err(unwritten(row).into()));
    }
    let mut held = 0;
    for row in rows {
        let until = until_second(row.until_at);
        // A row whose end passed before the event's stamp is history, and
        // is neither written nor counted.
        if until <= recorded {
            continue;
        }
        let unwritten = || unwritten(row);
        insert_row(
            tx,
            &StoredRow {
                run_id: &event.run_id,
                event_seq: event.seq,
                site: &row.site,
                provider: &row.provider,
                route: &row.route,
                until,
                reason: &row.reason,
                retry_after_seconds: row.retry_after_seconds,
                rounds: row.rounds,
                recorded,
                held: row.held,
            },
            unwritten,
        )?;
        // `insert_row` refused any row the table did not write, so this
        // one landed.
        held += usize::from(row.held.is_some());
    }
    Ok(held)
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
    /// Version 7's kind: `Some` for a held wait's row.
    held: Option<Held>,
}

/// Insert one row, refusing with `unwritten`'s words where the table wrote
/// none, as a trigger that ignores the insert leaves it: the one check
/// every cool-down writer takes, a terminal's rows, a resume's landing of
/// them and a probe's renewed round alike, so no row is ever believed
/// written that the store did not write (fix rounds 0106b1r5, H3, and
/// 0106b1r6, D3). The table's CHECK constraints and unique index judge it.
fn insert_row(
    conn: &Connection,
    row: &StoredRow<'_>,
    unwritten: impl FnOnce() -> QueueRefusal,
) -> Result<(), StoreError> {
    let written = conn.execute(
        "INSERT INTO provider_cooldowns
            (run_id, event_seq, site, provider, route, until_at, reason,
             retry_after_seconds, rounds, recorded_at, held, max_seconds)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
            row.held.map(|_| 1i64),
            row.held.map(|held| held.max_seconds as i64),
        ],
    )?;
    if written != 1 {
        return Err(unwritten().into());
    }
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

/// Every row in force at `now`, in the order it was written, then every
/// probe hold no settlement has ended ([`hold::standing`], by its one
/// predicate [`hold::standing_hold`], selected in SQL): the pass's and the claim's one
/// reading, so a probe in flight holds its route for both (fix round
/// 0106b1r4, F1), and no clock, margin or lock state ends a hold (fix round
/// 0106b1r12, S1). A row whose stored instant this reader cannot spell
/// refuses by name, carrying the row's `seq`.
///
/// Its two statements read one snapshot on every caller (fix round
/// 0106b1r5, H1): the claim's `Immediate` transaction, and
/// [`Store::cooldowns_in_force`]'s [`Store::read_snapshot`]. Read in two, a
/// refusal that commits its renewed row and ends its hold between them
/// leaves neither visible, and the route reads open though it was held in
/// every committed state.
pub(super) fn in_force(conn: &Connection, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    let sql = format!("SELECT {ROWS} FROM provider_cooldowns WHERE until_at > ?1 ORDER BY seq");
    let mut rows = read_rows(conn, &sql, now)?;
    rows.extend(hold::standing(conn)?);
    Ok(rows)
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
            probe: false,
        })
    }
}

/// The aggregate `provider` and `route` stand at, at `now`, or `None` when
/// nothing holds them: every unexpired row and every standing probe hold
/// holds the lane, and the lane ends at the latest `until_at`, the later
/// row's words where two end together. It is [`in_force`]'s reading, the
/// pass's and the claim's own, so an admitted run's wait, retry and probe
/// admission judge a route exactly as the gates do, a standing hold
/// included, whatever its stored end (fix round 0106b1r13, H1).
fn aggregate(
    conn: &Connection,
    provider: &str,
    route: &str,
    now: i64,
) -> Result<Option<CooldownAggregate>, StoreError> {
    let held = in_force(conn, now)?
        .iter()
        .filter(|row| row.provider == provider && row.route == route)
        .fold(None, |held, row| Some(CooldownAggregate::joined(held, row)));
    Ok(held)
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

/// One site's held rounds on an event, by round: version 7's kind, which
/// the table's CHECK (`held_names_its_maximum`) ties to its maximum, so a
/// present maximum is a held row. Only the version this binary writes holds
/// the kind's columns; an older one holds no held row.
fn held_rounds(
    conn: &Connection,
    run_id: &str,
    event_seq: u64,
    site: &str,
) -> Result<BTreeMap<u64, Held>, StoreError> {
    if !cool_downs_writable(conn)? {
        return Ok(BTreeMap::new());
    }
    const HELD_ROUNDS: &str = "SELECT rounds, max_seconds FROM provider_cooldowns
         WHERE run_id = ?1 AND event_seq = ?2 AND site = ?3 AND held = 1";
    let mut statement = conn.prepare(HELD_ROUNDS)?;
    let kinds = statement.query_map(params![run_id, event_seq as i64, site], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?;
    kinds
        .map(|kind| {
            let (rounds, max) = kind?;
            Ok((
                rounds as u64,
                Held {
                    max_seconds: max as u64,
                },
            ))
        })
        .collect()
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
    /// unexpired row and every standing probe hold is in force and the wait
    /// ends at the latest `until_at` across them (decision 0103 ruling
    /// 2(a)), or, while a hold stands, when a read finds it ended. An
    /// expired row is history and is never deleted; `None` is a route
    /// nothing holds. It reads [`Store::cooldowns_in_force`]'s rows, in one
    /// snapshot that joins a caller's own, so the engine's wait, retry and
    /// probe admission and the gates read one state (fix round 0106b1r13,
    /// H1). The queue's version is judged first: a version 1, 2 or 3
    /// journal holds no cool-down and answers `None`, and a version this
    /// binary does not know refuses.
    pub fn cooldown(
        &self,
        provider: &str,
        route: &str,
        now: OffsetDateTime,
    ) -> Result<Option<CooldownAggregate>, StoreError> {
        patiently("cooldown", self.patience, &self.path, || {
            self.read_snapshot(|| {
                if cool_downs_held(&self.conn)? {
                    aggregate(&self.conn, provider, route, now.unix_timestamp())
                } else {
                    Ok(None)
                }
            })
        })
    }

    /// Every cool-down row in force at `now`, in the order it was written:
    /// the rows the claim's judge reads inside its transaction. The
    /// queue's version is judged first, as [`Store::cooldown`] does. The
    /// version, the rows and the probe holds are read in one snapshot,
    /// which joins a caller's own (fix round 0106b1r5, H1).
    pub fn cooldowns_in_force(&self, now: OffsetDateTime) -> Result<Vec<Cooldown>, StoreError> {
        patiently("cooldowns_in_force", self.patience, &self.path, || {
            self.read_snapshot(|| {
                if cool_downs_held(&self.conn)? {
                    in_force(&self.conn, now.unix_timestamp())
                } else {
                    Ok(Vec::new())
                }
            })
        })
    }

    /// Every cool-down row in force at `now`, and the newest row of every
    /// provider and route no unexpired row holds any more, in the order
    /// they were written: what a readout paints, so a probe's later end
    /// survives the instant it passes and the event's earlier end never
    /// returns (decision 0103 slice 4f). Admission keeps reading
    /// [`Store::cooldown`] and [`Store::cooldowns_in_force`], which an
    /// expired row never holds. The queue's version is judged first, as
    /// [`Store::cooldown`] does. The read is one scan of the table's keys,
    /// providers, routes and ends in its primary key's order (#1112), one
    /// row per lookup by that key, so SQLite sorts nothing; the scan keeps
    /// each provider and route's newest `seq` so far as it goes, then reads
    /// each kept row's whole projection by its primary key, so the bound is
    /// `n` lookups in the table's rows, never the run or its events, plus a
    /// sort of the kept `seq`s and one lookup per kept row. SQLite holds one
    /// row at a time: the scanned row's provider and route, every row's
    /// read, discarded or kept, and then one kept row's projection. A
    /// discarded row's words are never read, and its provider and route are
    /// its group's, which the kept row of that group returns; no two rows'
    /// names are held at once. The read holds one provider and route per
    /// pair and the kept rows, the rows it returns. The history a
    /// journal keeps is the rate limits its runs met, so `n` is that count
    /// and not a run's size; the console pays the read twice per refresh,
    /// once to compare the kept frame's aggregate and once in the frame or
    /// whole read. The scan and every lookup read one snapshot, so a
    /// writer's commit at any point in the read changes nothing it returns,
    /// and its rows are the rows the journal held when the scan began. A
    /// row SQLite cannot hand over refuses the read: a key column's
    /// refusal (its key, provider, route or end, which the scan reads for
    /// every row) precedes a payload column's refusal of any kept row,
    /// however early that row was written. Its groups are the provider and
    /// route strings SQLite hands over, which are the stored keys in the
    /// UTF-8 journal the store makes, the only encoding it makes. A foreign
    /// UTF-16 journal can store two keys that decode to one string (an
    /// unpaired surrogate), which this read takes as one group; such a
    /// journal is outside its scope.
    pub fn cooldowns_newest(&self, now: OffsetDateTime) -> Result<Vec<Cooldown>, StoreError> {
        patiently("cooldowns_newest", self.patience, &self.path, || {
            if cool_downs_held(&self.conn)? {
                readout::newest(self, now.unix_timestamp())
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
        patiently("cooldown_rounds", self.patience, &self.path, || {
            if cool_downs_held(&self.conn)? {
                rounds(&self.conn, run_id, event_seq, site)
            } else {
                Ok(Vec::new())
            }
        })
    }

    /// One site's held rounds on a terminal event, by round, with the
    /// maximum each passed (decision 0106 ruling 2): the rounds whose
    /// interval the episode does not count and whose end a resume does not
    /// sleep to. The version is judged as [`Store::cooldown_rounds`] judges
    /// it; a queue older than version 7 holds no held row.
    pub fn cooldown_held_rounds(
        &self,
        run_id: &str,
        event_seq: u64,
        site: &str,
    ) -> Result<BTreeMap<u64, Held>, StoreError> {
        patiently("cooldown_held_rounds", self.patience, &self.path, || {
            if cool_downs_held(&self.conn)? {
                held_rounds(&self.conn, run_id, event_seq, site)
            } else {
                Ok(BTreeMap::new())
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
        self.append_round(&round, None)
    }

    /// Append ONE probe's held round (decision 0106 ruling 2): the provider
    /// stated a reset that alone passes the declared maximum `held` names.
    /// The round is judged and written as [`Store::append_cooldown_round`]
    /// writes an ordinary one, with version 7's kind on its row, so
    /// admission holds the route to its end and the episode does not count
    /// its interval.
    pub fn append_held_round(
        &mut self,
        round: CooldownRound<'_>,
        held: Held,
    ) -> Result<RoundAppend, StoreError> {
        self.append_round(&round, Some(held))
    }

    /// The one probe-round writer both appends share (0071 ruling 5).
    fn append_round(
        &mut self,
        round: &CooldownRound<'_>,
        held: Option<Held>,
    ) -> Result<RoundAppend, StoreError> {
        patiently("append_cooldown_round", self.patience, &self.path, || {
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
            // The hold ends only over the renewed row the table wrote (fix
            // round 0106b1r5, H3): a row it did not write refuses, and the
            // dropped transaction rolls back, leaving the hold standing.
            let unwritten = || {
                let mismatch = super::CooldownRoundMismatch {
                    run_id: round.run_id.to_string(),
                    event_seq: round.event_seq,
                    site: round.site.to_string(),
                    rounds: round.rounds,
                    standing,
                };
                QueueRefusal::CooldownRoundUnwritten(Box::new(mismatch))
            };
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
                    held,
                },
                unwritten,
            )?;
            hold::ended(&tx, round.ends, recorded)?;
            tx.commit()?;
            Ok(RoundAppend::Written)
        })
    }
}

#[cfg(test)]
mod tests;
