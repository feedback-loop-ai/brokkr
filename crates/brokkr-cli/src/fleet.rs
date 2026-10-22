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

use brokkr_core::fold::{fold, FoldError, RunState};
use brokkr_core::EventEnvelope;
use brokkr_runtime::realms::Hearth;
use brokkr_store::Store;

mod kept;
#[cfg(test)]
pub(crate) mod kept_tests;
mod listing;
mod page;
mod world;
#[cfg(test)]
mod world_tests;

pub(crate) use brokkr_view::{AHEAD_ROWS, PAGE_ROWS};
pub(crate) use kept::{Journal, KeptFleet};
pub(crate) use listing::{cursor, list_hearth, Called, PageRefused};
pub(crate) use page::{refused_row, Ask, Pages};
pub(crate) use world::list_world;

/// Why a listed run carries no state, and where its citation points.
pub(crate) struct Quarantine {
    /// Which refused it: the store, or the fold (#503).
    pub kind: brokkr_view::Quarantine,
    /// The refusal's own words: the store's when the journal would not
    /// load, the fold's when it loaded and would not fold.
    pub detail: String,
    /// The sequence the fold refused at. A journal that would not load
    /// has no event this read can cite, so — like an empty journal — it
    /// cites the position before the first one.
    pub seq: u64,
    /// The operator's acknowledgement standing at the journal's last
    /// event (#511), when one does.
    pub acknowledgement: Option<brokkr_view::Acknowledgement>,
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
        entry(
            Listing {
                run_id: &self.run_id,
                feature: &self.feature,
                created_at: &self.created_at,
            },
            self.events.last().map(|event| event.recorded_at.as_str()),
            self.state.as_ref(),
            &self.residuals,
        )
    }
}

/// What the journal's `runs` table says of a run, borrowed.
struct Listing<'a> {
    run_id: &'a str,
    feature: &'a str,
    created_at: &'a str,
}

/// A run's listing row, whichever read holds the run: this read's
/// [`ListedRun`] or the kept read's (#744), so the two cannot word a
/// quarantine differently.
fn entry<'a>(
    listing: Listing<'a>,
    last_recorded_at: Option<&'a str>,
    state: Result<&'a RunState, &'a Quarantine>,
    residuals: &'a [brokkr_view::ResidualFinding],
) -> brokkr_view::RunEntry<'a> {
    brokkr_view::RunEntry {
        run_id: listing.run_id,
        feature: listing.feature,
        created_at: listing.created_at,
        last_recorded_at,
        state: state.ok(),
        detail: state.err().map(|quarantine| brokkr_view::Refusal {
            acknowledgement: quarantine.acknowledgement.as_ref(),
            ..quarantine.kind.in_words(&quarantine.detail)
        }),
        residuals,
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
            let (events, state) = read_run(store, &run_id);
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

/// One run's journal read whole and folded, quarantined when it will not
/// load or will not fold.
fn read_run(store: &Store, run_id: &str) -> (Vec<EventEnvelope>, Result<RunState, Quarantine>) {
    judged(store, run_id, store.load(run_id))
}

/// Run `run_id` as `loaded` left it: folded, or quarantined when it did
/// not load or does not fold.
fn judged(
    store: &Store,
    run_id: &str,
    loaded: Result<Vec<EventEnvelope>, brokkr_store::StoreError>,
) -> (Vec<EventEnvelope>, Result<RunState, Quarantine>) {
    match loaded {
        Ok(events) => {
            let state = fold(&events).map_err(|error| did_not_fold(store, run_id, &error));
            (events, state)
        }
        Err(error) => (
            Vec::new(),
            Err(Quarantine {
                kind: brokkr_view::Quarantine::DoesNotLoad,
                detail: error.to_string(),
                seq: 0,
                acknowledgement: acknowledged(store, run_id),
            }),
        ),
    }
}

/// The quarantine of a run whose journal loaded and would not fold,
/// cited at the sequence the fold refused at — whether the fold was of
/// the whole journal or carried onto what landed after it (#744).
fn did_not_fold(store: &Store, run_id: &str, error: &FoldError) -> Quarantine {
    Quarantine {
        kind: brokkr_view::Quarantine::DoesNotFold,
        detail: error.to_string(),
        seq: error.seq(),
        acknowledgement: acknowledged(store, run_id),
    }
}

/// The acknowledgement standing on quarantined run `run_id` (#511). A
/// read that fails counts as none, so the run stays in "needs you": a
/// quarantine is answered only on evidence that it was.
fn acknowledged(store: &Store, run_id: &str) -> Option<brokkr_view::Acknowledgement> {
    let standing = store.acknowledgement(run_id).ok().flatten()?;
    Some(brokkr_view::Acknowledgement {
        seq: standing.seq,
        event_hash: standing.event_hash,
        operator: standing.operator,
        reason: standing.reason,
        recorded_at: standing.recorded_at,
    })
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

/// Each hearth's realm label and journal path, in the hearths' order: the
/// names every fleet readout gives its journals.
pub(crate) fn names(hearths: &[Hearth]) -> (Vec<String>, Vec<String>) {
    let labels = hearths.iter().map(Hearth::label).collect();
    let journals = hearths
        .iter()
        .map(|hearth| hearth.journal.display().to_string())
        .collect();
    (labels, journals)
}
