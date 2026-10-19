//! What a `brokkr doctor` report reads from disk, and its database line
//! (#374): the journal every other verb opens, or `none` where that
//! journal has no place (decision 0083 ruling 5).

use std::path::Path;

use brokkr_store::Store;

use super::Report;
use crate::folder::FolderError;

/// The paths a report reads, one parameter (decision 0071 ruling 10):
/// the journal, or why it has none, the agent library, the adapters and
/// the secrets store.
pub(super) struct Paths<'a> {
    pub(super) db: Result<&'a Path, &'a FolderError>,
    pub(super) library_root: &'a Path,
    pub(super) adapters_root: &'a Path,
    pub(super) secrets_store: &'a Path,
}

/// Deliberately read-write, unlike every reading verb (#375): the probe
/// asks whether a run could write here, so it opens the way `brokkr run`
/// opens, creating, enabling WAL and migrating. A journal with no place
/// is a warning naming the refusal a run would give, and nothing is
/// created: doctor reports and never refuses (decision 0046's Addendum).
pub(super) fn report_database(report: &mut Report, db: Result<&Path, &FolderError>) {
    match db.map(|db| (db, Store::open(db))) {
        Ok((db, Ok(store))) => {
            report.ok(
                "database",
                format!("{} opens (WAL, append-only triggers)", db.display()),
            );
            report_summaries(report, &store);
        }
        Ok((db, Err(e))) => report.missing("database", format!("{}: {e}", db.display())),
        Err(none) => report.warn("database", format!("none: {none}")),
    }
}

/// Every run whose summary row stands at its head and is not its fold's
/// (decision 0089 ruling 4), read one run at a time after the open has
/// caught up what fell behind: each a failure, since such a row is a bug
/// or an edit and never a lag. A summary the journal cannot be read for
/// at all is one too. A journal whose every row agrees adds no line.
fn report_summaries(report: &mut Report, store: &Store) {
    match store.summary_disagreements() {
        Ok(found) => {
            for disagreement in found {
                report.missing("summary", disagreement.to_string());
            }
        }
        Err(error) => report.missing("summary", format!("not read: {error}")),
    }
}

#[cfg(test)]
mod tests;
