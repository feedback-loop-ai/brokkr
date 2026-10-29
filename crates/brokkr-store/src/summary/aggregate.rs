//! The run summary's aggregate columns (decision 0089 ruling 7): what
//! `brokkr analytics` and Muninn's dossier read of a run, brokkr-core's
//! [`Aggregate`], written by the writer with the folded columns in the
//! same statement, after every column a page selects, so no page reads
//! them. A name is a group key, cut where it is stored to its bound at a
//! character boundary, never in a query; a run is found by when it
//! finished through the `(finished_ms)` index.
//!
//! The columns carry their own stamp, `aggregated_seq`, the seq they were
//! folded at, which a writer sets with `folded_seq`. A binary from before
//! them writes the folded columns and not these, so a trigger sends a row
//! whose stamp is not its folded seq behind its head: it reads whole and
//! the next writable open refolds it (ruling 8), and no row stands at its
//! head with aggregates of an earlier one. A trigger can be gone, so the
//! open's repair sends such a row behind on the same test, whether or not
//! the trigger was there when it was written.

use brokkr_core::summary::{Aggregate, Bounded, Ending, NAME_BYTES};
use rusqlite::types::Value;
use rusqlite::Connection;

use super::BACKFILLED;
use crate::StoreError;

/// The columns, in the row's order, and their types.
const COLUMNS: [(&str, &str); 11] = [
    ("realm", "TEXT"),
    ("recipe", "TEXT"),
    ("started_ms", "INTEGER"),
    ("finished_ms", "INTEGER"),
    ("ending", "TEXT"),
    ("ending_rule", "TEXT"),
    ("implement_visits", "INTEGER"),
    ("review_first", "INTEGER"),
    ("review_second", "INTEGER"),
    ("review_back", "INTEGER"),
    ("aggregated_seq", "INTEGER"),
];

pub(super) const INDEX: &str = "run_summary_finished";
pub(super) const TRIGGER: &str = "run_summary_aggregated";

/// The columns as the table's definition lists them.
pub(super) fn defined() -> String {
    let typed: Vec<String> = COLUMNS
        .iter()
        .map(|(name, kind)| format!("{name} {kind}"))
        .collect();
    typed.join(", ")
}

/// The row's own test that its aggregates are not the ones its folded head
/// states: `aggregated_seq` is written in the same statement as
/// `folded_seq`, so they differ only for a row a writer from before them
/// folded while the trigger was absent. `prefix` names the row in the
/// statement that asks, `NEW.` in the trigger and nothing on the row the
/// lane and the repair read, so the ruling has one home in each.
pub(super) fn stale(prefix: &str) -> String {
    format!("{prefix}aggregated_seq IS NOT {prefix}folded_seq")
}

/// The index and the trigger, `IF NOT EXISTS` as the table's own are. The
/// trigger fires when an update names `folded_hash`, as every writer's
/// does, and its own update does not fire it again, as SQLite fires no
/// trigger recursively unless asked.
pub(super) fn migration() -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {INDEX} ON run_summary (finished_ms);
        CREATE TRIGGER IF NOT EXISTS {TRIGGER} AFTER UPDATE OF folded_hash ON run_summary
            WHEN {stale} BEGIN
            UPDATE run_summary SET folded_seq = NULL, folded_hash = NULL
                WHERE run_id = NEW.run_id;
        END;",
        stale = stale("NEW.")
    )
}

/// A table without them, which a build from before them made, is made
/// anew: every row is derived, so it is dropped, reseeded behind and
/// refolded by the open's backfill, which is marked unfinished.
pub(super) fn rebuild() -> String {
    format!(
        "DROP TABLE IF EXISTS run_summary;
        DELETE FROM meta WHERE key = '{BACKFILLED}';"
    )
}

/// Does the table hold every column? A pure read.
pub(super) fn shaped(conn: &Connection) -> Result<bool, StoreError> {
    let found = super::columns(conn)?;
    Ok(COLUMNS
        .iter()
        .all(|(column, _)| found.iter().any(|name| name == column)))
}

/// How a run ended, as the `ending` column words it.
fn word(ending: &Ending) -> &'static str {
    match ending {
        Ending::Shipped => "shipped",
        Ending::ByRule(_) => "by_rule",
        Ending::ByOperator => "by_operator",
    }
}

/// A name cut to its bound as a group key, NULL where there is none.
fn key(name: Option<&str>) -> Value {
    name.map_or(Value::Null, |name| {
        Value::Text(Bounded::within(name, NAME_BYTES).cut)
    })
}

fn flag(set: bool) -> Value {
    Value::Integer(i64::from(set))
}

/// The columns `aggregate`, folded at `folded_seq`, is written as, by name.
pub(super) fn written(aggregate: &Aggregate, folded_seq: u64) -> Vec<(&'static str, Value)> {
    let finished = aggregate.finished.as_ref();
    let rule = finished.and_then(|(_, ending)| match ending {
        Ending::ByRule(rule) => rule.as_deref(),
        Ending::Shipped | Ending::ByOperator => None,
    });
    let values = [
        key(aggregate.realm.as_deref()),
        key(aggregate.recipe.as_deref()),
        aggregate.started_ms.map_or(Value::Null, Value::Integer),
        finished.map_or(Value::Null, |(at, _)| Value::Integer(*at)),
        finished.map_or(Value::Null, |(_, ending)| Value::Text(word(ending).into())),
        key(rule),
        Value::Integer(aggregate.implement_visits as i64),
        flag(aggregate.review_first),
        flag(aggregate.review_second),
        flag(aggregate.review_back),
        Value::Integer(folded_seq as i64),
    ];
    COLUMNS.iter().map(|(name, _)| *name).zip(values).collect()
}

#[cfg(test)]
mod tests;
