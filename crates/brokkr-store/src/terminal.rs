//! The append's terminal rule (#440): nothing but an annotation is ever
//! written after a run's terminal event.
//!
//! The rule itself has one home, `brokkr_core::fold::lawful_after_terminal`,
//! which reads the fold relation's terminal column. This module only
//! answers, inside the append's own write transaction, whether the run
//! has already reached a terminal event, so the store can refuse the
//! event before it writes anything — no event, no cool-down row, no
//! projection.
//!
//! The answer is bounded. The summary's fold is read by key, and while it
//! stands at the head it is the whole answer. A row that is absent or
//! behind falls back to the terminal index (`event_index_terminal`, a
//! partial index on `events`): the read asks whether the run has ANY
//! terminal event, so a pre-#440 journal whose terminal event sits under
//! later non-annotation events is still refused instead of read as live.
//! The terminal types are brokkr-core's one list, and a new event type is
//! refused until the relation admits it.

use std::sync::LazyLock;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{lawful_after_terminal, Status};
use rusqlite::{params, Connection, OptionalExtension};

use crate::StoreError;

/// The summary row's fold, by key: its status word and the seq it was
/// written at.
const SUMMARY: &str = "SELECT status, folded_seq FROM run_summary WHERE run_id = ?1";

/// The predicate that names a run's terminal event on `events`, guarded so
/// an envelope no reader parses is never extracted. The partial index and
/// the gate's query both read it, so the query takes the index instead of
/// walking the run (#440). Its types are brokkr-core's one terminal list,
/// never a copy here.
fn terminal_predicate() -> String {
    let names: Vec<String> = brokkr_core::fold::TERMINAL_EVENTS
        .into_iter()
        .map(|(event, _)| format!("'{}'", event.as_str()))
        .collect();
    format!(
        "{} IN ({})",
        crate::summary::text_at("envelope", "$.type"),
        names.join(", ")
    )
}

/// The index that keys a run's terminal event on `events`: the gate's
/// fallback reads the run's terminal rows alone, never its every event.
fn terminal_index() -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS event_index_terminal ON events (run_id, seq) WHERE {}",
        terminal_predicate()
    )
}

/// Does the journal's terminal index carry the definition this build
/// writes? A name alone would keep a copy from another build, whose
/// predicate the gate's query could not take.
pub(crate) fn intact(conn: &Connection) -> Result<bool, StoreError> {
    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = ?1",
            ["event_index_terminal"],
            |row| row.get(0),
        )
        .optional()?;
    Ok(sql
        .as_deref()
        .is_some_and(|sql| sql.contains(&terminal_predicate())))
}

/// Create the terminal index, `IF NOT EXISTS`; a copy another build wrote
/// is dropped first so this build's predicate is the one the query takes.
/// An additive repair, run only when [`intact`] says the journal lacks it.
pub(crate) fn migrate(conn: &Connection) -> Result<(), StoreError> {
    if !intact(conn)? {
        conn.execute_batch("DROP INDEX IF EXISTS event_index_terminal")?;
    }
    Ok(conn.execute_batch(&terminal_index())?)
}

/// The run's last terminal event, read through `event_index_terminal`:
/// the predicate is the index's own, so SQLite searches the run's terminal
/// rows and never walks its events.
static TERMINAL_AT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT {} FROM events WHERE run_id = ?1 AND {} ORDER BY seq DESC LIMIT 1",
        crate::summary::text_at("envelope", "$.type"),
        terminal_predicate()
    )
});

/// Refuse `event_type` when `run_id` has reached a terminal event and the
/// event is not an annotation. Called inside the append's transaction,
/// before the event is sealed, so a refusal leaves the journal exactly as
/// it found it.
pub(crate) fn refuse(
    tx: &Connection,
    run_id: &str,
    event_type: EventType,
    head: u64,
) -> Result<(), StoreError> {
    if lawful_after_terminal(event_type) {
        return Ok(());
    }
    let Some(status) = terminal_status(tx, run_id, head)? else {
        return Ok(());
    };
    Err(StoreError::AfterTerminal {
        run_id: run_id.to_string(),
        status,
        event_type,
    })
}

/// The status of the terminal event the run has reached, or `None` for a
/// run that is still live. The summary answers first while its fold
/// stands at `head`; a terminal status is absorbing, so it answers
/// wherever it stands. A row absent, behind or wordless falls back to the
/// run's terminal index, which answers "has this run ANY terminal event".
fn terminal_status(tx: &Connection, run_id: &str, head: u64) -> Result<Option<Status>, StoreError> {
    let mut statement = tx.prepare(SUMMARY)?;
    let summary: Option<(Option<String>, Option<i64>)> = statement
        .query_row(params![run_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    note(&statement);
    if let Some((word, folded)) = summary {
        if let Some(status) = word.as_deref().and_then(Status::named) {
            if matches!(status, Status::Completed | Status::Stopped) {
                return Ok(Some(status));
            }
            if folded == Some(head as i64) {
                return Ok(None);
            }
        }
    }
    any_terminal(tx, run_id)
}

/// The run's last terminal event, whichever later events a pre-#440
/// journal left above it: an index read of the terminal types alone, not
/// a walk of the tail. The row's `type` is the event's wire name, so it is
/// read back through brokkr-core's one terminal list.
fn any_terminal(tx: &Connection, run_id: &str) -> Result<Option<Status>, StoreError> {
    let mut statement = tx.prepare(TERMINAL_AT.as_str())?;
    let word: Option<String> = statement
        .query_row(params![run_id], |row| row.get(0))
        .optional()?;
    note(&statement);
    Ok(word.as_deref().and_then(|word| {
        brokkr_core::fold::TERMINAL_EVENTS
            .into_iter()
            .find(|(event, _)| event.as_str() == word)
            .map(|(_, status)| status)
    }))
}

#[cfg(test)]
thread_local! {
    /// The instructions the check's statements took on this test thread.
    static STEPS: std::cell::Cell<i64> = const { std::cell::Cell::new(0) };
}

/// Count `statement`'s VM steps against the check's bound test.
fn note(statement: &rusqlite::Statement<'_>) {
    #[cfg(test)]
    STEPS.with(|steps| {
        steps.set(
            steps.get() + i64::from(statement.reset_status(rusqlite::StatementStatus::VmStep)),
        );
    });
    #[cfg(not(test))]
    let _ = statement;
}

/// Take the instructions the check's statements have taken since the last
/// call, for the test that binds the check to one size of run.
#[cfg(test)]
pub(crate) fn take_steps() -> i64 {
    STEPS.with(std::cell::Cell::take)
}

#[cfg(test)]
mod tests;
