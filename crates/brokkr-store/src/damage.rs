//! A damaged journal (#1000): one refusal, naming the file and the way
//! back, for every way a journal's own bytes can say it is damaged.
//! Brokkr reports damage and never repairs it (decision 0001).
//!
//! What is damage is closed here, in [`Damage`], and decided in one place,
//! [`classified`], which every store operation's error passes through on
//! its way out of [`crate::patience::patiently`] and of both opens. What
//! is not damage keeps its own words: a peer's lock is
//! [`StoreError::Contended`], a schema this build does not read is
//! [`StoreError::SchemaMismatch`], a chain that does not verify is
//! [`StoreError::Chain`] (tamper evidence), and a missing file is SQLite's
//! own refusal to open it.

use std::path::Path;

use brokkr_core::envelope::EventEnvelope;
use serde::de::IgnoredAny;
use serde_json::error::Category;

use crate::{Store, StoreError};

/// How a journal's own bytes say it is damaged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Damage {
    /// SQLite read a page it cannot make sense of: `SQLITE_CORRUPT`.
    Malformed,
    /// SQLite reads the file as no database at all: `SQLITE_NOTADB`.
    NotADatabase,
    /// The file is a journal whose `meta` table records no schema: what a
    /// damaged page read as before SQLite 3.51.3, and what 0.12.0 printed
    /// as "Query returned no rows".
    NoSchema,
    /// A stored envelope that is not JSON, at the run and the row's seq,
    /// both read from the row since the envelope cannot be.
    Envelope { run_id: String, seq: i64 },
}

impl std::fmt::Display for Damage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Damage::Malformed => f.write_str("SQLite reads its disk image as malformed"),
            Damage::NotADatabase => f.write_str("SQLite reads it as no database"),
            Damage::NoSchema => f.write_str("it records no database schema"),
            Damage::Envelope { run_id, seq } => write!(
                f,
                "run '{}' holds an envelope at seq {seq} that is not JSON",
                run_id.escape_debug()
            ),
        }
    }
}

/// `error`, named [`StoreError::Damaged`] in `journal` where it says the
/// journal is damaged, and untouched otherwise. Damage already named
/// passes as it is.
pub(crate) fn classified(journal: &Path, error: StoreError) -> StoreError {
    let damage = match &error {
        StoreError::Sqlite(sqlite) => match sqlite.sqlite_error_code() {
            Some(rusqlite::ErrorCode::DatabaseCorrupt) => Damage::Malformed,
            Some(rusqlite::ErrorCode::NotADatabase) => Damage::NotADatabase,
            _ => return error,
        },
        StoreError::Unparsed { run_id, seq } => Damage::Envelope {
            run_id: run_id.clone(),
            seq: *seq,
        },
        // Not damage: each keeps its own words (a peer's lock, tamper
        // evidence, a schema this build does not read, JSON that is no
        // envelope, and every refusal about content or a name).
        StoreError::Contended { .. }
        | StoreError::Damaged { .. }
        | StoreError::Chain(_)
        | StoreError::RowBeforeChain { .. }
        | StoreError::SchemaMismatch { .. }
        | StoreError::Json(_)
        | StoreError::SeatRecord(_)
        | StoreError::Clock { .. }
        | StoreError::UnknownHead { .. }
        | StoreError::RunNotFound(_)
        | StoreError::RunExists(_)
        | StoreError::RunIdTooLong { .. }
        | StoreError::RunIdUnnameable(_)
        | StoreError::AppendConflict { .. }
        | StoreError::HeadMoved { .. }
        | StoreError::AfterTerminal { .. }
        | StoreError::TokenCountOutOfRange(..)
        | StoreError::AlreadyAcknowledged { .. }
        | StoreError::Queue(_)
        | StoreError::SeatTree(_) => return error,
    };
    StoreError::Damaged {
        journal: journal.to_path_buf(),
        damage,
    }
}

/// The envelope `text` the row of `run_id` at `seq` holds. Text that is not
/// JSON is [`StoreError::Unparsed`], which [`classified`] names damage;
/// JSON that is not an envelope keeps serde's words. serde stops at the
/// first field that does not fit an envelope, before it reads the rest, and
/// names its own limits (nesting past its depth, a number past `f64`)
/// syntax, so any refusal is damage only when the whole text does not
/// parse as some value either: skipping a value has neither limit.
pub(crate) fn envelope(run_id: &str, seq: i64, text: &str) -> Result<EventEnvelope, StoreError> {
    serde_json::from_str(text).map_err(|error| match error.classify() {
        Category::Syntax | Category::Eof | Category::Data
            if serde_json::from_str::<IgnoredAny>(text).is_err() =>
        {
            StoreError::Unparsed {
                run_id: run_id.to_string(),
                seq,
            }
        }
        Category::Io | Category::Syntax | Category::Eof | Category::Data => StoreError::Json(error),
    })
}

impl Store {
    /// `read`, its error passed through [`classified`] in this store's
    /// journal: for a read that does not wait under
    /// [`crate::patience::patiently`], as a page of the summary inside one
    /// snapshot does not.
    pub(crate) fn named<T>(&self, read: Result<T, StoreError>) -> Result<T, StoreError> {
        read.map_err(|error| classified(&self.path, error))
    }
}

#[cfg(test)]
mod tests;
