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
/// cuts it, the stamp's millisecond reading where SQLite reads one, and an
/// acknowledgement that no longer stands.
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

/// The stamp in milliseconds since the epoch, `stamp` naming the guarded
/// `recorded_at` expression: SQLite's own date functions read the journal's
/// `YYYY-MM-DDTHH:MM:SS[.fff…]Z` shape, its first three fractional digits
/// cut rather than rounded as [`brokkr_core::stamp::parse_millis`] cuts
/// them, and `NULL` where SQLite cannot read the date. The trigger reads
/// no application function, so an older engine's appends keep the row's
/// own last activity too (decision 0089 rulings 5 and 6). A shape SQLite
/// reads and the fold does not (a zone offset, say) takes the trigger's
/// reading; the fold's value stays authoritative where the row is folded.
fn millis_of(stamp: &str) -> String {
    format!(
        "CAST(strftime('%s', substr({stamp}, 1, 19) || 'Z') AS INTEGER) * 1000
         + CASE WHEN substr({stamp}, 20, 1) = '.'
             THEN CAST(substr({stamp}, 21, 1) AS INTEGER) * 100
                + CAST(substr({stamp}, 22, 1) AS INTEGER) * 10
                + CAST(substr({stamp}, 23, 1) AS INTEGER)
             ELSE 0 END"
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
