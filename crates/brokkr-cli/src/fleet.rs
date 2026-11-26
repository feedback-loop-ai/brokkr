//! The fleet read, written once (#377). Every surface that lists a
//! hearth's runs — `brokkr runs`, `brokkr tui`, `brokkr system ui` and Muninn's
//! dossier — reads it here, so one corrupt run reads one way everywhere.
//!
//! A run whose journal will not load, or loads and will not fold, is
//! QUARANTINED: it stays in the listing as a row carrying the refusal's
//! own words, and every healthy run beside it is still read. Nothing is
//! repaired (decision 0001) — the refusal is reported. Single-run verbs
//! (`inspect`, `watch`, `resume`) keep their bare `load(..)?` and
//! `fold(..)?`: a command aimed at one run must fail loudly on that run.

use std::path::Path;

use brokkr_core::ending::{closing, Closing};
use brokkr_core::fold::{fold, FoldError, RunState};
use brokkr_core::EventEnvelope;
use brokkr_runtime::realms::Hearth;
use brokkr_store::{Store, StoreError};
use brokkr_view::{Cooling, RunRow, WaitFact, WaitRefusal, WaitRound, Waits};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

mod kept;
#[cfg(test)]
pub(crate) mod kept_tests;
mod listing;
pub(crate) mod page;
mod world;
#[cfg(test)]
mod world_tests;

pub(crate) use brokkr_view::{AHEAD_ROWS, PAGE_ROWS};
pub(crate) use kept::{Journal, KeptFleet};
pub(crate) use listing::{cursor, list_hearth, Called, Filter, PageRefused};
pub(crate) use page::{one, refused_row, Ask, Pages};
pub(crate) use world::list_world;

/// The aggregate every readout paints: every cool-down in force at `now`
/// and, for a lane no row holds any more, the newest row that ended it, so
/// a probe's later round is painted after it passes and never reverts to
/// the event's earlier end. Read once by the caller and passed into
/// brokkr-view as data (decision 0103 slice 4f). The view is pure and
/// opens no store; its end is written by brokkr-core's one stamp writer,
/// never by the raw `Display` of a time: a row whose end cannot be written
/// is not passed on, and the event's own end stands.
pub(crate) fn cooling(
    store: &Store,
    now: &str,
) -> Result<Vec<brokkr_view::Cooling>, brokkr_store::StoreError> {
    let at = OffsetDateTime::parse(now, &Rfc3339).map_err(|_| brokkr_store::StoreError::Clock {
        now: now.to_string(),
    })?;
    Ok(store
        .cooldowns_newest(at)?
        .into_iter()
        .filter_map(|row| {
            let until = brokkr_core::stamp::write_millis(row.until_at).ok()?;
            Some(brokkr_view::Cooling {
                provider: row.provider,
                route: row.route,
                until,
            })
        })
        .collect())
}

/// Read the latest stored round and its held maximum for each recorded site.
fn read_rounds(store: &Store, run_id: &str, fact: WaitFact) -> WaitFact {
    let WaitFact::Waits {
        event_seq, sites, ..
    } = fact
    else {
        return fact;
    };
    let read = store.read_snapshot(|| {
        sites
            .iter()
            .filter_map(|site| {
                latest_round(store, run_id, event_seq, site.site.as_deref()).transpose()
            })
            .collect::<Result<_, StoreError>>()
    });
    match read {
        Ok(rounds) => WaitFact::Waits {
            event_seq,
            sites,
            rounds,
        },
        Err(error) => WaitFact::Unreadable(WaitRefusal::Store(error.to_string())),
    }
}

/// Read a site's ordered rounds once and look up the last round's maximum.
fn latest_round(
    store: &Store,
    run_id: &str,
    event_seq: u64,
    site: Option<&str>,
) -> Result<Option<(Option<String>, WaitRound)>, StoreError> {
    let key = site.unwrap_or_default();
    let rounds = store.cooldown_rounds(run_id, event_seq, key)?;
    let held = store.cooldown_held_rounds(run_id, event_seq, key)?;
    Ok(rounds.last().map(|row| {
        let round = WaitRound {
            held: held.get(&row.rounds).copied(),
        };
        (site.map(str::to_string), round)
    }))
}

/// The applied event's waits, kept beside the current readout so a console
/// refresh can see table-only changes at the same journal head.
pub(crate) struct RunWaits {
    recorded: WaitFact,
    pub(crate) painted: Waits,
}

impl RunWaits {
    pub(crate) fn read(
        store: &Store,
        run_id: &str,
        now: &str,
        events: &[EventEnvelope],
        state: Option<&RunState>,
    ) -> Result<Self, StoreError> {
        let mut waits = Self {
            recorded: WaitFact::applied(events, state),
            painted: Waits::default(),
        };
        waits.refresh(store, run_id, now)?;
        Ok(waits)
    }

    pub(crate) fn refresh(
        &mut self,
        store: &Store,
        run_id: &str,
        now: &str,
    ) -> Result<(), StoreError> {
        let paint = || -> Result<Waits, StoreError> {
            let cooling = cooling(store, now)?;
            let fact = read_rounds(store, run_id, self.recorded.clone());
            Ok(Waits::of(&fact, &cooling))
        };
        self.painted = match self.recorded {
            WaitFact::Waits { .. } => store.read_snapshot(paint),
            WaitFact::NoWait | WaitFact::Unreadable(_) => paint(),
        }?;
        Ok(())
    }
}

/// A kept behind row can gain a new kind at the same head and route end.
fn refresh_row(store: &Store, row: RunRow, seq: u64, cooling: &[Cooling]) -> RunRow {
    if row.waits == Waits::default() {
        return row;
    }
    let fact = match store.last_event(&row.run_id, seq) {
        Ok(event) => read_rounds(store, &row.run_id, WaitFact::of(event.as_ref())),
        Err(error) => WaitFact::Unreadable(WaitRefusal::Store(error.to_string())),
    };
    row.with_waits(Waits::of(&fact, cooling))
}

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
    pub waits: WaitFact,
}

/// A run's journal and its fold or refusal.
type Judged = (Vec<EventEnvelope>, Result<RunState, Quarantine>);

impl ListedRun {
    fn new(store: &Store, listing: (String, String, String), read: Judged) -> Self {
        let (run_id, feature, created_at) = listing;
        let (events, state) = read;
        let residuals = brokkr_view::residual_findings_at(&run_id, &events, state.as_ref().ok());
        let waits = read_rounds(
            store,
            &run_id,
            WaitFact::applied(&events, state.as_ref().ok()),
        );
        Self {
            run_id,
            feature,
            created_at,
            events,
            state,
            residuals,
            waits,
        }
    }

    /// The row every fleet surface builds this run's listing from.
    pub(crate) fn entry(&self) -> brokkr_view::RunEntry<'_> {
        let state = self.state.as_ref();
        entry(
            Listing {
                run_id: &self.run_id,
                feature: &self.feature,
                created_at: &self.created_at,
            },
            self.events.last().map(|event| event.recorded_at.as_str()),
            state,
            (&self.residuals, ended(&self.events, state)),
            &self.waits,
            brokkr_core::CauseRead::latest(&self.events).map(Box::new),
        )
    }
}

/// The ending of a run read as `events` folded to `state`, by the one
/// reading every summary row is written from (decision 0064); none for a
/// run that did not fold.
fn ended(events: &[EventEnvelope], state: Result<&RunState, &Quarantine>) -> Option<Closing> {
    state.ok().and_then(|state| closing(events, state))
}

/// What the journal's `runs` table says of a run, borrowed.
struct Listing<'a> {
    run_id: &'a str,
    feature: &'a str,
    created_at: &'a str,
}

/// A run's listing row, whichever read holds the run: this read's
/// [`ListedRun`] or the kept read's (#744), so the two cannot word a
/// quarantine differently. `read` is what each derived of its events:
/// the residual findings and the ending.
fn entry<'a>(
    listing: Listing<'a>,
    last_recorded_at: Option<&'a str>,
    state: Result<&'a RunState, &'a Quarantine>,
    (residuals, closing): (&'a [brokkr_view::ResidualFinding], Option<Closing>),
    waits: &brokkr_view::WaitFact,
    park_cause: Option<Box<brokkr_core::CauseRead>>,
) -> brokkr_view::RunEntry<'a> {
    brokkr_view::RunEntry {
        park_cause,
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
        closing,
        waits: waits.clone(),
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
        .map(|listing| {
            let read = read_run(store, &listing.0);
            ListedRun::new(store, listing, read)
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
