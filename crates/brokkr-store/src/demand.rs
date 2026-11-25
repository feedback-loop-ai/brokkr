//! What admission reads of a journal to size an entry by what its recipe
//! used (decision 0125): the usage its recipes' latest runs recorded under
//! `effect-usage/v1`, and the runs that have no terminal event. Another
//! journal the realms map lists is read through [`Store::read_untouched`]
//! (decision 0119 ruling 3), which writes nothing beside it and answers
//! nothing from a lock-free read the journal changed under.
//!
//! The history is one bounded statement per journal: the latest
//! `candidates` ended runs of each recipe by `created_at`, and of their events
//! only the `phase/entered` rows, the endings carrying a `usage` object
//! and `run/completed`, each prefiltered by its text and then read through
//! `json_valid`, so an envelope no reader parses is never extracted.

use brokkr_core::EventType;
use rusqlite::params;

use crate::{patiently, Store, StoreError};

/// One event of a recipe's recent run that sizes it: its run, the recipe
/// it started under and when, and the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRow {
    pub run: String,
    pub recipe: String,
    pub created_at: String,
    pub event: UsageEvent,
}

/// A phase entered, an attempt ending with its usage as JSON text, or
/// a run completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsageEvent {
    Completed,
    Entered(String),
    Ended(String),
}

/// A run with no terminal event, and the recipe it started under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unended {
    pub run: String,
    pub recipe: String,
}

const RUN_COMPLETED: &str = EventType::RunCompleted.as_str();

/// Whether the run named by the query's `r` alias has a terminal event.
fn ended_sql() -> String {
    format!(
        "EXISTS (SELECT 1 FROM events WHERE events.run_id = r.run_id AND {})",
        crate::terminal::terminal_predicate()
    )
}

/// The latest `?1` ended runs of each recipe, and the events of theirs that size
/// it, newest run first and each run's events in order.
fn history_sql() -> String {
    let kind = crate::summary::text_at("e.envelope", "$.type");
    let phase = crate::summary::text_at("e.envelope", "$.payload.phase");
    let ended = ended_sql();
    format!(
        "SELECT r.run_id, r.bundle_name, r.created_at, {kind}, {phase},
            CASE WHEN json_valid(e.envelope)
                  AND json_type(e.envelope, '$.payload.usage') = 'object'
                THEN json_extract(e.envelope, '$.payload.usage') END
         FROM (SELECT run_id, bundle_name, created_at, ROW_NUMBER() OVER (
                   PARTITION BY bundle_name ORDER BY created_at DESC, run_id DESC) AS latest
               FROM runs AS r WHERE {ended}) AS r
         JOIN events AS e ON e.run_id = r.run_id
         WHERE r.latest <= ?1
           AND (instr(e.envelope, 'phase/entered') > 0
                OR instr(e.envelope, '\"usage\"') > 0
                OR instr(e.envelope, '{RUN_COMPLETED}') > 0)
         ORDER BY r.created_at DESC, r.run_id DESC, e.seq"
    )
}

/// The runs no terminal event ends, in the journal's order.
fn unended_sql() -> String {
    format!(
        "SELECT r.run_id, r.bundle_name FROM runs AS r WHERE NOT {}
         ORDER BY r.created_at, r.run_id",
        ended_sql()
    )
}

/// The event a history row names, or `None` for one that sizes nothing:
/// an ending without `usage`, or a row whose text only looked like one.
fn event(kind: Option<String>, phase: Option<String>, usage: Option<String>) -> Option<UsageEvent> {
    match (kind.as_deref(), phase, usage) {
        (Some(RUN_COMPLETED), _, _) => Some(UsageEvent::Completed),
        (Some("phase/entered"), Some(phase), _) => Some(UsageEvent::Entered(phase)),
        (Some("effect/succeeded" | "effect/failed" | "effect/indeterminate"), _, Some(usage)) => {
            Some(UsageEvent::Ended(usage))
        }
        _ => None,
    }
}

impl Store {
    /// Every event of the latest `candidates` ended runs of each recipe that
    /// sizes it ([`UsageEvent`]), newest run first.
    pub fn usage_history(&self, candidates: u32) -> Result<Vec<UsageRow>, StoreError> {
        let sql = history_sql();
        patiently("usage_history", self.patience, &self.path, || {
            let mut statement = self.conn.prepare(&sql)?;
            let rows = statement.query_map(params![candidates], |row| {
                let (run, recipe, created_at) = (row.get(0)?, row.get(1)?, row.get(2)?);
                let event = event(row.get(3)?, row.get(4)?, row.get(5)?);
                Ok(event.map(|event| UsageRow {
                    run,
                    recipe,
                    created_at,
                    event,
                }))
            })?;
            let rows = rows.collect::<Result<Vec<_>, _>>()?;
            Ok(rows.into_iter().flatten().collect())
        })
    }

    /// Every run no terminal event ends, and its recipe: the runs that may
    /// still be running, which only their fold says.
    pub fn unended_runs(&self) -> Result<Vec<Unended>, StoreError> {
        let sql = unended_sql();
        patiently("unended_runs", self.patience, &self.path, || {
            let mut statement = self.conn.prepare(&sql)?;
            let rows = statement.query_map([], |row| {
                Ok(Unended {
                    run: row.get(0)?,
                    recipe: row.get(1)?,
                })
            })?;
            Ok(rows.collect::<Result<Vec<_>, _>>()?)
        })
    }
}

#[cfg(test)]
mod tests;
