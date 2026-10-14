//! `brokkr operator acknowledge` (#511; decision 0076 item 11): the
//! operator's word that a quarantined run, one whose journal does not
//! load or loads and does not fold, has been looked at.
//!
//! Nothing is appended to the run's chain, which would only make a second
//! journal that does not fold, and nothing is repaired or folded past
//! (decision 0001). The store keeps the acknowledgement beside the chain,
//! at the head read here before the journal was judged, and refuses the
//! write when that head moved (decision 0029). A run that folds has
//! `conclude`, and is refused here.

use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::fold::fold;
use brokkr_core::EventEnvelope;
use brokkr_store::{Attribution, StoreError};

use super::delivery::operator_name;
use crate::cli_args::JournalArgs;
use crate::{open_journal, selector, Access, Exit};

/// Acknowledge quarantined run `run` in the journal `journal` names, for
/// `reason`.
pub(super) fn acknowledge(
    workspace: &Path,
    run: &str,
    reason: &str,
    journal: JournalArgs,
) -> Result<ExitCode> {
    let mut store = open_journal(&journal.journal(workspace)?, Access::Append)?;
    let run = selector::resolve_run(&store, run)?;
    // The head before the journal is judged: one that moves after this
    // read refuses the write below rather than acknowledging a journal
    // nobody judged.
    let (seq, hash) = store.head_hash(&run)?;
    let Some(refusal) = quarantine(store.load(&run))? else {
        anyhow::bail!(
            "run {run} folds, so it is not quarantined and there is nothing to acknowledge; \
             nothing was recorded. A run that folds is closed with brokkr conclude --run {run} \
             --reason <why>"
        );
    };
    let operator = operator_name();
    let by = Attribution {
        operator: &operator,
        reason,
    };
    let acknowledged = store
        .acknowledge_quarantine(&run, (seq, &hash), by)
        .map_err(|error| moved(error, &run))?;
    eprintln!(
        "acknowledged quarantined run {run} at event {}; it lists under older until its \
         journal moves",
        acknowledged.seq
    );
    eprintln!("its refusal stands: {refusal}");
    Ok(Exit::Completed.into())
}

/// What refused the journal `loaded` came from, in the refusal's own
/// words: the store's when it would not load, the fold's when it loaded
/// and would not fold, as the fleet read quarantines it. `None` when it
/// folds. Contention is no refusal of the journal, and passes on.
fn quarantine(
    loaded: Result<Vec<EventEnvelope>, StoreError>,
) -> Result<Option<String>, StoreError> {
    match loaded {
        Ok(events) => Ok(fold(&events).err().map(|error| error.to_string())),
        Err(error) if error.is_contention() => Err(error),
        Err(error) => Ok(Some(error.to_string())),
    }
}

/// The store's refusal of the write, with what to do when it is a head
/// that moved while the run was read.
fn moved(error: StoreError, run: &str) -> anyhow::Error {
    let head_moved = matches!(error, StoreError::HeadMoved { .. });
    let error = anyhow::Error::from(error);
    match head_moved {
        true => error.context(format!(
            "the journal of {run} moved while it was read; nothing was recorded; look again \
             with brokkr runs"
        )),
        false => error,
    }
}

#[cfg(test)]
mod tests;
