//! The run's ending's columns (decision 0064, addendum choice 5): the
//! ending in decision 0064's twelve words, what it cites, whether a later
//! `classify` stood over an earlier ending, and the stamp that proves the
//! columns were written at the folded head, as the accepted stop's carry
//! theirs (#909). A writer from before them writes the `ending` column in
//! its three words and leaves the stamp off, so a trigger sends its row
//! behind, at a new head or at the one the stamp names, and the open's
//! backfill refolds it; where the trigger was gone, the open's repair
//! sends every row behind; a reader of a journal that lacks them reads
//! every run whole.
//!
//! Each part of the citation is held within [`CITATION_BYTES`] where a
//! page selects it, and kept whole last in the row where the bound cut it
//! (decision 0089 ruling 2), so a page reads no citation past its bound
//! and `Store::whole` reads it back lossless.

use brokkr_core::ending::Citation;
use brokkr_core::summary::{Bounded, Summary, CITATION_BYTES};
use rusqlite::types::Value;
use rusqlite::Connection;

use super::columns::{has, typed, Stamped};
use crate::schema::all_present;
use crate::StoreError;

/// The columns in the row's order, their stamp and their trigger, which
/// the summary's intact check lists.
pub(super) const PART: Stamped = Stamped {
    columns: &[
        ("ending", "TEXT"),
        ("ending_realm", "TEXT"),
        ("ending_run", "TEXT"),
        ("ending_ref", "TEXT"),
        ("ending_stood_over", "INTEGER"),
        ("ending_folded_seq", "INTEGER"),
    ],
    stamp: "ending_folded_seq",
    shared: Some("ending"),
    trigger: "run_summary_ended",
};

/// The open's repair where the trigger was gone: an older writer may have
/// rewritten `ending` at the head the stamp names while nothing watched,
/// which no stamp shows, so every row's stamp is cleared and the repair
/// sends each behind; nothing where the trigger stood. A pure read.
pub(super) fn unwatched(conn: &Connection) -> Result<String, StoreError> {
    let watched = all_present(conn, "trigger", &[PART.trigger])?;
    Ok(if watched {
        String::new()
    } else {
        format!("UPDATE run_summary SET {} = NULL", PART.stamp)
    })
}

/// The citation's parts whole where their bound cut them, last in the row
/// with the other whole values, which no page selects.
pub(super) const WHOLE: [(&str, &str); 3] = [
    ("ending_realm_whole", "TEXT"),
    ("ending_run_whole", "TEXT"),
    ("ending_ref_whole", "TEXT"),
];

/// The whole columns as the table's definition lists them.
pub(super) fn defined_whole() -> String {
    typed(&WHOLE)
}

/// Does the table carry every column of the ending, the bounded and the
/// whole? One pure read of the table's columns checks both.
pub(super) fn shaped(conn: &Connection) -> Result<bool, StoreError> {
    has(conn, PART.columns.iter().chain(&WHOLE))
}

/// A citation's part as its two columns hold it: within its bound, and
/// whole where the bound cut it.
fn part(text: Option<&str>) -> [Value; 2] {
    let Some(text) = text else {
        return [Value::Null, Value::Null];
    };
    let Bounded { cut, whole } = Bounded::within(text, CITATION_BYTES);
    [Value::Text(cut), whole.map_or(Value::Null, Value::Text)]
}

/// The columns `summary`'s ending, read at its folded seq, is written as,
/// by name: every one NULL but the stamp while the run is not over.
pub(super) fn written(summary: &Summary) -> Vec<(&'static str, Value)> {
    let closing = summary.closing.as_ref();
    let cited = closing.and_then(|closing| closing.ended.cited.as_ref());
    let (realm, run, reference) = match cited {
        Some(Citation::Run { realm, run_id }) => (realm.as_deref(), Some(run_id.as_str()), None),
        Some(Citation::Ref(reference)) => (None, None, Some(reference.as_str())),
        None => (None, None, None),
    };
    let [realm, realm_whole] = part(realm);
    let [run, run_whole] = part(run);
    let [reference, reference_whole] = part(reference);
    let values = [
        closing.map_or(Value::Null, |closing| {
            Value::Text(closing.ended.ending.word().to_string())
        }),
        realm,
        run,
        reference,
        closing.map_or(Value::Null, |closing| {
            Value::Integer(i64::from(closing.stood_over))
        }),
        Value::Integer(summary.folded_seq as i64),
    ];
    PART.columns
        .iter()
        .chain(&WHOLE)
        .map(|(name, _)| *name)
        .zip(
            values
                .into_iter()
                .chain([realm_whole, run_whole, reference_whole]),
        )
        .collect()
}

#[cfg(test)]
mod tests;
