//! The run summary table's columns, read by name once: the definition a
//! part writes and the shape check that holds it share this read, so a
//! column's name exists in one place (decision 0071 ruling 5). The head's
//! columns and the trigger that keeps them at its event live here too, so
//! the millisecond reading it writes exists once.

use rusqlite::Connection;

use super::stamp_of;
use crate::StoreError;

/// The table's columns, by name.
pub(super) fn read(conn: &Connection) -> Result<Vec<String>, StoreError> {
    let mut statement = conn.prepare("SELECT name FROM pragma_table_info('run_summary')")?;
    let found: Vec<String> = statement
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(found)
}

/// The columns as the table's definition lists them, each name and its
/// type, in the order the definition states them.
pub(super) fn typed(columns: &[(&str, &str)]) -> String {
    let typed: Vec<String> = columns
        .iter()
        .map(|(name, kind)| format!("{name} {kind}"))
        .collect();
    typed.join(", ")
}

/// The columns an event moves when it is the run's new head, `event`
/// naming its row: the head, the whole stamp until the bounding trigger
/// cuts it, the stamp's millisecond reading where the fold reads one, and
/// an acknowledgement that no longer stands.
pub(super) fn moved_to(event: &str) -> String {
    let stamp = stamp_of(&format!("{event}.envelope"));
    format!(
        "head_seq = {event}.seq, head_hash = {event}.event_hash,
         last_recorded_at = {stamp}, last_recorded_at_whole = NULL,
         last_at_ms = {millis},
         acknowledged = 0, ack_operator = NULL, ack_reason = NULL, ack_at = NULL",
        millis = millis_of(&stamp)
    )
}

/// The journal stamp's shapes [`brokkr_core::stamp::parse_millis`] reads,
/// `stamp` naming the guarded `recorded_at` expression:
/// `YYYY-MM-DDTHH:MM:SS`, an optional `.` and digits, then `Z` or
/// `±HH:MM`. SQLite alone would also read a space separator, a missing
/// zone or text past the time, and would stop its own text reads at an
/// embedded NUL, so the whole shape is checked here before any date
/// function sees it (decision 0071 ruling 3).
fn accepted(stamp: &str) -> String {
    format!(
        "instr(CAST({stamp} AS BLOB), X'00') = 0
         AND length({stamp}) >= 20
         AND substr({stamp}, 1, 19) GLOB
             '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]T[0-9][0-9]:[0-9][0-9]:[0-9][0-9]'
         AND (
             (substr({stamp}, -1, 1) = 'Z'
              AND (length({stamp}) = 20
                   OR (substr({stamp}, 20, 1) = '.'
                       AND substr({stamp}, 21, length({stamp}) - 21)
                           NOT GLOB '*[^0-9]*')))
             OR (length({stamp}) >= 25
                 AND substr({stamp}, -6, 1) IN ('+', '-')
                 AND substr({stamp}, -5, 2) GLOB '[0-9][0-9]'
                 AND substr({stamp}, -3, 1) = ':'
                 AND substr({stamp}, -2, 2) GLOB '[0-9][0-9]'
                 AND (length({stamp}) = 25
                      OR (substr({stamp}, 20, 1) = '.'
                          AND substr({stamp}, 21, length({stamp}) - 26)
                              NOT GLOB '*[^0-9]*')))
         )"
    )
}

/// The stamp's zone offset in milliseconds, `stamp` already accepted:
/// `Z` is zero, and `±HH:MM` moves the UTC the date functions read by
/// the offset, as [`brokkr_core::stamp::parse_millis`] moves it.
fn zone_offset(stamp: &str) -> String {
    format!(
        "(CASE WHEN substr({stamp}, -1, 1) = 'Z' THEN 0
              WHEN substr({stamp}, -6, 1) = '+'
              THEN -(CAST(substr({stamp}, -5, 2) AS INTEGER) * 3600
                     + CAST(substr({stamp}, -2, 2) AS INTEGER) * 60)
              ELSE CAST(substr({stamp}, -5, 2) AS INTEGER) * 3600
                   + CAST(substr({stamp}, -2, 2) AS INTEGER) * 60
         END) * 1000"
    )
}

/// The stamp in milliseconds since the epoch, `stamp` naming the guarded
/// `recorded_at` expression, cut as [`brokkr_core::stamp::parse_millis`]
/// cuts it: the first three fractional digits of the fraction itself, not
/// rounded, a zone offset applied, not stripped, and the hour, minute and
/// second added as the fold adds them, so a leap second and an over-range
/// time field read the value the fold reads instead of `NULL`. The date
/// is SQLite's own reading; the shape is checked whole and an embedded
/// NUL refused, so a stamp the fold refuses (a space separator, a missing
/// zone, text past the zone) reads `NULL`. A month or day SQLite refuses
/// while the shape still matches reads `NULL`, where `parse_millis` has
/// no such limit. The trigger reads no application function, so an older
/// engine's appends keep the row's own last activity too (decision 0089
/// rulings 5 and 6).
fn millis_of(stamp: &str) -> String {
    let fraction = format!(
        "CASE WHEN substr({stamp}, -1, 1) = 'Z'
              THEN length({stamp}) - 21 ELSE length({stamp}) - 26 END"
    );
    format!(
        "CASE WHEN {accepted} THEN
             CAST(strftime('%s', substr({stamp}, 1, 10) || 'T00:00:00Z') AS INTEGER) * 1000
             + CAST(substr({stamp}, 12, 2) AS INTEGER) * 3600000
             + CAST(substr({stamp}, 15, 2) AS INTEGER) * 60000
             + CAST(substr({stamp}, 18, 2) AS INTEGER) * 1000
             + CASE WHEN substr({stamp}, 20, 1) = '.'
                 THEN CAST(substr({stamp}, 21, 1) AS INTEGER) * 100
                    + CASE WHEN {fraction} >= 2
                        THEN CAST(substr({stamp}, 22, 1) AS INTEGER) * 10 ELSE 0 END
                    + CASE WHEN {fraction} >= 3
                        THEN CAST(substr({stamp}, 23, 1) AS INTEGER) ELSE 0 END
                 ELSE 0 END
             + {offset}
         END",
        accepted = accepted(stamp),
        fraction = fraction,
        offset = zone_offset(stamp)
    )
}

/// The head trigger's name: the one definition this build keeps.
pub(super) const HEAD_TRIGGER: &str = "run_summary_of_an_event";

/// The head trigger, made exactly as [`super::migration`] creates it: the
/// one definition this build keeps, which [`triggers_intact`] reads back
/// from `sqlite_master`. SQLite strips `IF NOT EXISTS` from the text it
/// stores, so the statement is written without it, after the drop that
/// replaces an older build's copy.
pub(super) fn head_trigger() -> String {
    format!(
        "CREATE TRIGGER {HEAD_TRIGGER} AFTER INSERT ON events BEGIN
            UPDATE run_summary SET {moved}
                WHERE run_id = NEW.run_id AND NEW.seq > head_seq;
        END",
        moved = moved_to("NEW")
    )
}

/// Does the journal carry every trigger `names` lists, the head trigger
/// with this build's definition? One pure read of `sqlite_master`, the
/// statement the presence check already made, so a read that holds the
/// summary to its schema makes no statement more than before (decision
/// 0089 ruling 5). [`super::summary_intact`] asks it once, and a writable
/// open replaces an older build's copy before reading anything by a value
/// its appends never kept.
pub(super) fn triggers_intact(conn: &Connection, names: &[&str]) -> Result<bool, StoreError> {
    let mut statement =
        conn.prepare("SELECT name, sql FROM sqlite_master WHERE type = 'trigger'")?;
    let found: Vec<(String, Option<String>)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let present = |name: &str| found.iter().any(|(found, _)| found == name);
    let head = head_trigger();
    Ok(names.iter().all(|name| present(name))
        && found
            .iter()
            .any(|(name, sql)| name == HEAD_TRIGGER && sql.as_deref() == Some(head.as_str())))
}

/// A part of the row added after it (#909; decision 0064), with a stamp of
/// its own: its columns in the row's order, the one of them a writer sets
/// with `folded_seq`, the one of them a writer from before the part writes
/// too, in its own words, if any, and the trigger that sends behind a row
/// a writer from before the part folded, whose stamp it left off.
pub(super) struct Stamped {
    pub(super) columns: &'static [(&'static str, &'static str)],
    pub(super) stamp: &'static str,
    pub(super) shared: Option<&'static str>,
    pub(super) trigger: &'static str,
}

impl Stamped {
    /// The columns as the table's definition lists them.
    pub(super) fn defined(&self) -> String {
        typed(self.columns)
    }

    /// The row's own test that the part is not the one its folded head
    /// states: the stamp is written in the same statement as `folded_seq`,
    /// so they differ only for a row a writer from before the part folded.
    /// `prefix` names the row in the statement that asks.
    pub(super) fn stale(&self, prefix: &str) -> String {
        format!(
            "{prefix}{stamp} IS NOT {prefix}folded_seq",
            stamp = self.stamp
        )
    }

    /// The trigger, `IF NOT EXISTS` as the table's own are; its own update
    /// does not fire it again, as SQLite fires no trigger recursively
    /// unless asked. It fires on a stale stamp, and on the shared column
    /// rewritten under a stamp left as it was, which only a writer from
    /// before the part does at the head the stamp names, since a fold of
    /// one head writes one value. The row it sends behind keeps no stamp,
    /// so no later write at that head vouches for the part but its own.
    pub(super) fn migration(&self) -> String {
        let rewritten = self.shared.map_or(String::new(), |shared| {
            format!(
                " OR (NEW.{shared} IS NOT OLD.{shared} AND NEW.{stamp} IS OLD.{stamp})",
                stamp = self.stamp
            )
        });
        format!(
            "CREATE TRIGGER IF NOT EXISTS {trigger} AFTER UPDATE OF folded_hash ON run_summary
            WHEN {stale}{rewritten} BEGIN
            UPDATE run_summary SET folded_seq = NULL, folded_hash = NULL, {stamp} = NULL
                WHERE run_id = NEW.run_id;
        END;",
            trigger = self.trigger,
            stale = self.stale("NEW."),
            stamp = self.stamp
        )
    }

    /// Does the table carry every column of the part, which a journal from
    /// before it does not? A pure read.
    pub(super) fn shaped(&self, conn: &Connection) -> Result<bool, StoreError> {
        has(conn, self.columns)
    }
}

/// Does the table carry every one of `columns`, by name? One pure read of
/// the table's columns, however many parts `columns` chains.
pub(super) fn has<'a, 'b: 'a>(
    conn: &Connection,
    columns: impl IntoIterator<Item = &'a (&'b str, &'b str)>,
) -> Result<bool, StoreError> {
    let found = read(conn)?;
    Ok(columns
        .into_iter()
        .all(|(column, _)| found.iter().any(|name| name == column)))
}

/// The covering index the realm list and refused counts read (decision 0089
/// ruling A4, slice 4's unit U3c): the two names in their order and every
/// column [`not_behind`] tests, so the read visits the index alone.
pub(super) const REALM_INDEX: &str = "run_summary_realm";

/// The row is behind its folded head: its fold or a stamped part is not the
/// head's. `prefix` names the row, so one rule serves the generated `lane`
/// and each read that must state the test without reading that column.
pub(super) fn behind(prefix: &str) -> String {
    format!("NOT ({})", not_behind(prefix))
}

/// The rows that are not behind their folded head, stated on the stamped
/// columns they carry: `folded_seq` and `head_seq` at the same place, and
/// no part's stamp off it. A read that covers these columns with its index
/// asks this predicate instead of `lane`, so the index alone answers it.
pub(crate) fn not_behind(prefix: &str) -> String {
    format!(
        "{prefix}folded_seq IS NOT NULL
         AND {prefix}folded_seq = {prefix}head_seq
         AND NOT ({stale})",
        stale = super::stale(prefix)
    )
}
