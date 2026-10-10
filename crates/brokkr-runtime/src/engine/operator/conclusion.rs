//! Why `conclude` refused a run, and the words each refusal prints.

use brokkr_core::fold::{Refusal, Status};

use super::EngineError;

/// Why `conclude` refused a run: each is the operator's to act on, and
/// none wrote a conclusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcludeRefusal {
    /// The run already has its conclusion. A second `run/stopped` would
    /// fail the fold as an event after terminal, so this comes before the
    /// first append, not after a half-written closure.
    AlreadyConcluded { status: Status },
    /// The stop `conclude` commanded was refused, with the fold's reason.
    StopRefused { refusal: Refusal },
    /// A fenced write met a head that moved beneath the conclusion.
    JournalMoved,
}

impl ConcludeRefusal {
    /// The words each refusal has always printed, the run named in them.
    /// The two that were once `engine:` errors keep that prefix.
    pub(in crate::engine) fn text(&self, run_id: &str) -> String {
        match self {
            ConcludeRefusal::AlreadyConcluded { status } => format!(
                "run '{run_id}' is already concluded ({}); conclude appends nothing",
                status.as_str()
            ),
            ConcludeRefusal::StopRefused { refusal } => format!(
                "engine: conclude: run '{run_id}' refused the stop ({}); the journal \
                 moved beneath the conclusion, so something may still be driving this run — \
                 look with `brokkr runs` before closing",
                refusal.word()
            ),
            ConcludeRefusal::JournalMoved => format!(
                "engine: conclude: the journal moved beneath the conclusion of run '{run_id}', \
                 so something may still be driving it — a conclusion is for a run believed \
                 dead; look with `brokkr runs` before closing"
            ),
        }
    }

    pub(super) fn of(self, run_id: &str) -> EngineError {
        EngineError::ConcludeRefused {
            run_id: run_id.to_string(),
            why: self,
        }
    }
}
