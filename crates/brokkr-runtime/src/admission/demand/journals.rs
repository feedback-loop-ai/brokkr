//! The journals a measured pass reads (decision 0125 rulings 3 and 5,
//! ruling G): its own store, and every other journal the realms map lists,
//! each read untouched ([`Store::read_untouched`]) through one
//! bounded history statement and one statement for the runs no terminal
//! event ends, each of which is folded to tell whether it runs.
//!
//! A journal the map lists that is not there holds no runs. One that is
//! there and cannot be read, or holds a run that does not fold,
//! contributes nothing, and is named; a failure of the pass's own journal
//! fails the pass, as every other read of it does.

use std::path::{Path, PathBuf};
use std::time::Duration;

use brokkr_core::fold::{fold, Status};
use brokkr_store::{Store, UsageRow};

use super::{Observation, Short};
use crate::admission::capacity::Shared;
use crate::admission::AdmissionError;

/// How long a pass waits on another journal's rollback-mode writer
/// before it names that journal unreadable. A WAL journal is read
/// without its lock, so this bounds only a rollback-mode one.
const PATIENCE: Duration = Duration::from_millis(500);

/// A run that reads `running`: the journal it is in, its id, its recipe
/// and the phase it stands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Running {
    pub(crate) journal: PathBuf,
    pub(crate) run: String,
    pub(crate) recipe: String,
    pub(crate) phase: Option<String>,
}

/// What the journals said: every history row, every running run, and
/// each journal that could not be read.
#[derive(Debug, Default)]
pub(crate) struct Read {
    pub(crate) rows: Vec<UsageRow>,
    pub(crate) running: Vec<Running>,
    pub(crate) unreadable: Vec<Short>,
}

/// Read `own` and each of `others` that is not `own`, each once: the
/// latest `candidates` ended runs of each recipe, and the running runs.
pub(crate) fn read(
    own: &Store,
    others: &[PathBuf],
    candidates: u32,
) -> Result<Read, AdmissionError> {
    let (rows, running) = one(own, candidates)?;
    let mut read = Read {
        rows,
        running,
        unreadable: Vec::new(),
    };
    let mut seen = vec![canonical(own.path())];
    for journal in others {
        let at = canonical(journal);
        if seen.contains(&at) || absent(journal) {
            continue;
        }
        seen.push(at);
        match Store::read_untouched(journal, PATIENCE, |store| one(store, candidates)) {
            Ok((rows, running)) => {
                read.rows.extend(rows);
                read.running.extend(running);
            }
            Err(error) => read
                .unreadable
                .push(Short::Observation(Observation::Unreadable {
                    journal: journal.clone(),
                    error: Shared::new(error),
                })),
        }
    }
    Ok(read)
}

/// The path `journal` resolves to, or as written where it does not.
fn canonical(journal: &Path) -> PathBuf {
    std::fs::canonicalize(journal).unwrap_or_else(|_| journal.to_path_buf())
}

/// Nothing at `journal`: a realm that has not run yet.
fn absent(journal: &Path) -> bool {
    matches!(std::fs::symlink_metadata(journal), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
}

/// One journal's history rows and running runs.
fn one(store: &Store, candidates: u32) -> Result<(Vec<UsageRow>, Vec<Running>), AdmissionError> {
    let rows = store.usage_history(candidates)?;
    let mut running = Vec::new();
    for unended in store.unended_runs()? {
        let events = store.load(&unended.run)?;
        if events.is_empty() {
            continue;
        }
        let state = fold(&events).map_err(|source| {
            let run = unended.run.clone();
            AdmissionError::Fold { run, source }
        })?;
        if state.run.status() == Status::Running {
            running.push(Running {
                journal: store.path().to_path_buf(),
                run: unended.run,
                recipe: unended.recipe,
                phase: state.phase,
            });
        }
    }
    Ok((rows, running))
}
