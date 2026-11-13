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
//! behind falls back to a primary-key walk backwards from the head, which
//! stops at the first event that is not an annotation: by this rule, the
//! event before any trailing annotations is the terminal event itself, so
//! the walk reads those annotations and one more, never the run's events.
//! A new event type is refused until the relation admits it: the walk
//! asks [`lawful_after_terminal`], not a list kept here.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{lawful_after_terminal, Status};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

use crate::StoreError;

/// The summary row's fold, by key: its status word and the seq it was
/// written at.
const SUMMARY: &str = "SELECT status, folded_seq FROM run_summary WHERE run_id = ?1";

/// The event type at one seq, read from the envelope's text with the same
/// guarded last-key reading the store's triggers use.
const TYPE_AT: &str = "SELECT CASE WHEN json_valid(envelope) THEN
        (SELECT CASE type WHEN 'text' THEN value END FROM json_each(envelope)
         WHERE key = 'type' ORDER BY id DESC LIMIT 1)
    END FROM events WHERE run_id = ?1 AND seq = ?2";

/// Refuse `event_type` when `run_id` reached a terminal event at or
/// before `head` and the event is not an annotation. Called inside the
/// append's transaction, before the event is sealed, so a refusal leaves
/// the journal exactly as it found it.
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
        status: status.as_str(),
        event_type: event_type.as_str(),
    })
}

/// The status of the terminal event the run reached at or before `head`,
/// or `None` for a run that is still live. The summary answers first
/// while its fold stands at `head`; a terminal status is absorbing, so it
/// answers wherever it stands.
fn terminal_status(tx: &Connection, run_id: &str, head: u64) -> Result<Option<Status>, StoreError> {
    let mut statement = tx.prepare(SUMMARY)?;
    let summary: Option<(Option<String>, Option<i64>)> = statement
        .query_row(params![run_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    note(&statement);
    if let Some((word, folded)) = summary {
        if let Some(status) = word.as_deref().and_then(status_of) {
            if matches!(status, Status::Completed | Status::Stopped) {
                return Ok(Some(status));
            }
            if folded == Some(head as i64) {
                return Ok(None);
            }
        }
    }
    walk(tx, run_id, head)
}

/// The terminal event at or before `head`, found by walking the events
/// backwards past the annotations that follow it and stopping at the
/// first event that is not one.
fn walk(tx: &Connection, run_id: &str, head: u64) -> Result<Option<Status>, StoreError> {
    let mut statement = tx.prepare(TYPE_AT)?;
    let mut found = None;
    let mut seq = head;
    while seq > 0 {
        let word: Option<String> = statement
            .query_row(params![run_id, seq as i64], |row| row.get(0))
            .optional()?;
        let Some(word) = word else { break };
        let event_type = serde_json::from_value::<EventType>(Value::String(word))?;
        if !lawful_after_terminal(event_type) {
            found = terminal_event(event_type);
            break;
        }
        seq -= 1;
    }
    note(&statement);
    Ok(found)
}

/// The summary status a stored word names, read against the statuses'
/// own words so the vocabulary has one home ([`Status::as_str`]).
fn status_of(word: &str) -> Option<Status> {
    [
        Status::Running,
        Status::AwaitingOperator,
        Status::Completed,
        Status::Stopped,
    ]
    .into_iter()
    .find(|status| status.as_str() == word)
}

/// The status a run's final event states, or `None` for an event that
/// concluded nothing.
fn terminal_event(event: EventType) -> Option<Status> {
    if event == EventType::RunCompleted {
        Some(Status::Completed)
    } else if event == EventType::RunStopped {
        Some(Status::Stopped)
    } else {
        None
    }
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
