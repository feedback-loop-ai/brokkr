//! The fleet read, written once (#377). Every surface that lists a
//! hearth's runs — `brokkr runs`, `brokkr tui`, `brokkr ui` and Muninn's
//! dossier — reads it here, so one corrupt run reads one way everywhere.
//!
//! A run whose journal will not load, or loads and will not fold, is
//! QUARANTINED: it stays in the listing as a row carrying the refusal's
//! own words, and every healthy run beside it is still read. Nothing is
//! repaired (decision 0001) — the refusal is reported. Single-run verbs
//! (`inspect`, `watch`, `resume`) keep their bare `load(..)?` and
//! `fold(..)?`: a command aimed at one run must fail loudly on that run.

use std::path::Path;

use brokkr_core::fold::{fold, RunState};
use brokkr_core::EventEnvelope;
use brokkr_store::Store;

/// Why a listed run carries no state, and where its citation points.
pub(crate) struct Quarantine {
    /// The refusal's own words: the store's when the journal would not
    /// load, the fold's when it loaded and would not fold.
    pub detail: String,
    /// The sequence the fold refused at. A journal that would not load
    /// has no event this read can cite, so — like an empty journal — it
    /// cites the position before the first one.
    pub seq: u64,
}

/// One run of a fleet listing, read ONCE: its journal, what the fold says
/// about it, and the residual findings the journal carries with the
/// operator's supersede marks on them (decision 0047 ruling 3).
pub(crate) struct ListedRun {
    pub run_id: String,
    pub feature: String,
    pub created_at: String,
    /// The journal as it loaded; empty when it would not load.
    pub events: Vec<EventEnvelope>,
    pub state: Result<RunState, Quarantine>,
    pub residuals: Vec<brokkr_view::ResidualFinding>,
}

impl ListedRun {
    /// The row every fleet surface builds this run's listing from.
    pub(crate) fn entry(&self) -> brokkr_view::RunEntry<'_> {
        brokkr_view::RunEntry {
            run_id: &self.run_id,
            feature: &self.feature,
            created_at: &self.created_at,
            state: self.state.as_ref().ok(),
            detail: self
                .state
                .as_ref()
                .err()
                .map(|quarantine| quarantine.detail.as_str()),
            residuals: &self.residuals,
        }
    }
}

/// One hearth, read: every run it lists, and — when the hearth itself
/// could not be listed — the words of that refusal instead.
pub(crate) struct HearthRead {
    pub runs: Vec<ListedRun>,
    pub detail: Option<String>,
}

impl HearthRead {
    /// The runs, for a surface that reads exactly one hearth: there is no
    /// other hearth for a listing refusal to sit beside, so it is the
    /// surface's own refusal.
    pub(crate) fn listed(self) -> Result<Vec<ListedRun>, String> {
        match self.detail {
            Some(detail) => Err(detail),
            None => Ok(self.runs),
        }
    }

    fn refused(error: brokkr_store::StoreError) -> Self {
        HearthRead {
            runs: Vec::new(),
            detail: Some(error.to_string()),
        }
    }
}

/// Read every run of one hearth, quarantining per run.
pub(crate) fn read_hearth(store: &Store) -> HearthRead {
    let listed = match store.list_runs() {
        Ok(listed) => listed,
        Err(error) => return HearthRead::refused(error),
    };
    let runs = listed
        .into_iter()
        .map(|(run_id, feature, created_at)| {
            let (events, state) = match store.load(&run_id) {
                Ok(events) => {
                    let state = fold(&events).map_err(|error| Quarantine {
                        detail: error.to_string(),
                        seq: error.seq(),
                    });
                    (events, state)
                }
                Err(error) => (
                    Vec::new(),
                    Err(Quarantine {
                        detail: error.to_string(),
                        seq: 0,
                    }),
                ),
            };
            let residuals = brokkr_view::residual_findings(&run_id, &events);
            ListedRun {
                run_id,
                feature,
                created_at,
                events,
                state,
                residuals,
            }
        })
        .collect();
    HearthRead { runs, detail: None }
}

/// Read one hearth of a many-hearth world by its journal. A journal that
/// will not open at all is the hearth's own refusal, in its own words: a
/// many-hearth listing survives a realm whose journal is not there yet,
/// the same way it survives one corrupt run.
///
/// Opened READ-ONLY, and deliberately: a reading surface that creates the journal it came to read
/// has written to a world it was only asked to look at (decision 0026
/// ruling 5).
pub(crate) fn read_journal(journal: &Path) -> HearthRead {
    match Store::open_read_only(journal) {
        Ok(store) => read_hearth(&store),
        Err(error) => HearthRead::refused(error),
    }
}
