//! `brokkr analytics`: the cross-run readout over every hearth the world
//! names (#568). The figures are derived once, in
//! [`brokkr_view::analytics`]; this verb reads the hearths, hands them over
//! with the clock, and prints what comes back. It writes nothing
//! (decision 0014).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_core::summary::{cut, STAMP_BYTES};
use brokkr_runtime::realms::Hearth;
use brokkr_store::Store;
use brokkr_view::{
    Analytics, AnalyticsError, AnalyticsHearth, AnalyticsRun, FactSource, Query, Range,
};

use crate::{fleet, hearths_of, now_rfc3339, Exit};

mod rows;

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct AnalyticsArgs {
    /// The world's map — every hearth it names is read (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(crate) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(crate) db: Option<PathBuf>,
    /// How far back from now: 24h, 7d or 30d.
    #[arg(long, default_value = "7d", value_parser = range)]
    pub(crate) range: Range,
    /// Only this realm's runs.
    #[arg(long)]
    pub(crate) realm: Option<String>,
    /// Emit the view model verbatim — this is what the console reads.
    #[arg(long)]
    pub(crate) json: bool,
}

/// The view's refusal of a word as a range, as clap carries an error: the
/// view's crate names no `std::error` (decision 0071 ruling 1).
#[derive(Debug)]
pub(crate) struct RangeRefused(pub(crate) AnalyticsError);

impl std::fmt::Display for RangeRefused {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for RangeRefused {}

/// `--range`, read by the view's one vocabulary.
pub(crate) fn range(word: &str) -> Result<Range, RangeRefused> {
    word.parse().map_err(RangeRefused)
}

/// The cross-run readouts, flattened into the top-level verbs so the
/// command line reads as it always has, and dispatched as one group.
#[derive(clap::Subcommand)]
pub(crate) enum CrossRunCmd {
    /// What the runs did across every hearth, for a range and an optional
    /// realm: shipped, start to end, waiting on you, rework, cost and
    /// served models. `--json` emits the view model.
    Analytics(AnalyticsArgs),
    /// One recipe's state machine and its traffic over a range. `--json` emits the view model.
    Graph(super::graph::GraphArgs),
}

/// Each cross-run readout to its verb.
pub(crate) fn cross_run(workspace: &Path, command: CrossRunCmd) -> Result<ExitCode> {
    match command {
        CrossRunCmd::Analytics(args) => analytics(workspace, args),
        CrossRunCmd::Graph(args) => super::graph::graph(workspace, args),
    }
}

/// `brokkr analytics`: the readout for the range ending now.
pub(crate) fn analytics(workspace: &Path, args: AnalyticsArgs) -> Result<ExitCode> {
    let hearths = hearths_of(workspace, args.realms, args.db)?;
    let view = derive(&hearths, args.range, args.realm.as_deref(), &now_rfc3339())?;
    printed(&view, args.json, paint)
}

/// A cross-run readout's view printed as `--json` asks: verbatim, or as
/// the verb paints it. `analytics` and `graph` alike.
pub(crate) fn printed<T: serde::Serialize>(
    view: &T,
    json: bool,
    paint: impl FnOnce(&T) -> String,
) -> Result<ExitCode> {
    match json {
        true => println!("{}", serde_json::to_string_pretty(view)?),
        false => print!("{}", paint(view)),
    }
    Ok(Exit::Completed.into())
}

/// Every hearth read, quarantining what does not load, and the view
/// derived over them at `now` (decision 0089 ruling 8).
///
/// `derive` decides each hearth before the first fleet read: one whose
/// store opens read-only and whose aggregates are ready takes its figures,
/// realms and unreadable counts from the rows, and is never loaded run by
/// run (U3c2's R1). A hearth that is not migrated, not ready or not
/// openable keeps today's whole read, so the fleet read is still needed
/// for it — and for a migrated hearth only when its row path refuses a
/// read, which falls back to the whole read exactly as U4 built it.
fn derive(hearths: &[Hearth], range: Range, realm: Option<&str>, now: &str) -> Result<Analytics> {
    // The decision is made for every hearth before any fleet read.
    let stores: Vec<Option<Store>> = hearths
        .iter()
        .map(|hearth| Store::open_read_only(&hearth.journal).ok())
        .collect();
    let row_stores: Vec<Option<&Store>> = stores
        .iter()
        .map(|store| {
            store
                .as_ref()
                .filter(|store| matches!(store.aggregates_ready(), Ok(true)))
        })
        .collect();
    let (labels, journals) = fleet::names(hearths);
    // The window and the view read the range, realm and clock; the facts
    // are the sources', so the query carries no hearths.
    let query = Query {
        hearths: &[],
        range,
        realm,
        now,
    };
    let (window, before) = brokkr_view::windows(&query).map_err(refused)?;
    let mut sources: Vec<Box<dyn FactSource>> = Vec::with_capacity(hearths.len());
    for index in 0..hearths.len() {
        let reading = Reading {
            hearth: &hearths[index],
            label: &labels[index],
            journal: &journals[index],
            range,
            realm,
            now,
            window,
            before,
        };
        sources.push(Box::new(source_of(&reading, row_stores[index])));
    }
    brokkr_view::analytics_with(&brokkr_view::FactUnion::new(sources), &query).map_err(refused)
}

/// One hearth's read request: the hearth, its label and journal, the
/// query's range, realm and clock, and the windows a figure folds
/// (decision 0071 ruling 10's parameter object).
struct Reading<'a> {
    hearth: &'a Hearth,
    label: &'a str,
    journal: &'a str,
    range: Range,
    realm: Option<&'a str>,
    now: &'a str,
    window: brokkr_view::Window,
    before: brokkr_view::Window,
}

/// One hearth's facts: the rows when its store is ready, the whole read
/// when it is not, and — for a ready hearth whose row path refuses a read —
/// the whole read of the fleet read taken then, so a refusal still states
/// every figure (decision 0089 ruling 8's boundary, U3c2).
fn source_of(reading: &Reading<'_>, store: Option<&Store>) -> rows::HearthSource {
    if let Some(store) = store {
        let empty: [AnalyticsRun<'_>; 0] = [];
        let grouped = AnalyticsHearth {
            label: reading.label,
            realms: &reading.hearth.realms,
            journal: reading.journal,
            runs: &empty,
            refusal: None,
        };
        if let Ok(source) = rows::HearthSource::from_store(
            &grouped,
            reading.range,
            reading.realm,
            reading.now,
            reading.window,
            reading.before,
            store,
        ) {
            return source;
        }
    }
    whole_source(reading, fleet_read(&reading.hearth.journal))
}

/// One hearth read whole: the fleet read's runs in the whole read's tie
/// order (decision 0089 ruling A4), quarantined when the fleet refused the
/// hearth, and handed to [`rows::HearthSource::read`] with no store, so
/// every figure is the whole read's.
fn whole_source(reading: &Reading<'_>, mut read: fleet::HearthRead) -> rows::HearthSource {
    order(&mut read.runs);
    let runs: Vec<AnalyticsRun<'_>> = read
        .runs
        .iter()
        .map(|run| AnalyticsRun {
            events: &run.events,
            state: run.state.as_ref().ok(),
        })
        .collect();
    let grouped = AnalyticsHearth {
        label: reading.label,
        realms: &reading.hearth.realms,
        journal: reading.journal,
        runs: &runs,
        refusal: read.detail.as_deref(),
    };
    rows::HearthSource::read(
        &grouped,
        reading.range,
        reading.realm,
        reading.now,
        reading.window,
        reading.before,
        None,
    )
}

/// The fleet read, counted in tests so a migrated, ready hearth's no-run
/// load can be asserted (decision 0089 ruling 8's boundary, U3c2).
fn fleet_read(journal: &Path) -> fleet::HearthRead {
    #[cfg(test)]
    FLEET_READS.with(|reads| reads.set(reads.get() + 1));
    fleet::read_journal(journal)
}

#[cfg(test)]
std::thread_local! {
    /// The fleet reads `derive` took on this test's thread.
    static FLEET_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many fleet reads `derive` has taken on this thread: a migrated,
/// ready hearth takes none.
#[cfg(test)]
fn fleet_reads() -> usize {
    FLEET_READS.with(std::cell::Cell::get)
}

/// Start a test's fleet-read count at zero.
#[cfg(test)]
fn reset_fleet_reads() {
    FLEET_READS.with(|reads| reads.set(0));
}

/// Put a fleet read's runs in the whole read's tie order (decision 0089
/// ruling A4): the cut `created_at`, then the run's id.
fn order(runs: &mut [fleet::ListedRun]) {
    runs.sort_by(|left, right| {
        place(&left.created_at, &left.run_id).cmp(&place(&right.created_at, &right.run_id))
    });
}

/// The view's refusal in its own words. Both of `derive`'s reads refuse
/// only on the query's windows, so the clock the first one refuses is the
/// one that names it.
fn refused(error: AnalyticsError) -> anyhow::Error {
    anyhow::Error::msg(error.to_string())
}

/// The key a run's realm is placed by (decision 0089 ruling A4's tie):
/// `created_at` cut where the store keeps it, then the run's id. The
/// store's covering realm index states this key, and the whole read sorts
/// by it too, so one key decides both lists.
pub(super) fn place<'a>(created_at: &'a str, run_id: &'a str) -> (&'a str, &'a str) {
    (cut(created_at, STAMP_BYTES), run_id)
}

/// Every hearth read, quarantining what does not load, handed to `view`
/// as the cross-run readouts take them: `analytics` and `graph` alike.
pub(crate) fn read_hearths<T>(hearths: &[Hearth], view: impl FnOnce(&[AnalyticsHearth]) -> T) -> T {
    let mut read: Vec<fleet::HearthRead> = hearths
        .iter()
        .map(|hearth| fleet::read_journal(&hearth.journal))
        .collect();
    // The whole read's tie rule (decision 0089 ruling A4, slice 4's unit
    // U3c): two runs listed at one cut `created_at` stand by their id, the
    // order the store's covering realm index states too.
    for hearth in &mut read {
        order(&mut hearth.runs);
    }
    let runs: Vec<Vec<AnalyticsRun>> = read
        .iter()
        .map(|hearth| {
            let runs = hearth.runs.iter();
            runs.map(|run| AnalyticsRun {
                events: &run.events,
                state: run.state.as_ref().ok(),
            })
            .collect()
        })
        .collect();
    let (labels, journals) = fleet::names(hearths);
    let grouped: Vec<AnalyticsHearth> = (0..hearths.len())
        .map(|index| AnalyticsHearth {
            label: &labels[index],
            realms: &hearths[index].realms,
            journal: &journals[index],
            runs: &runs[index],
            refusal: read[index].detail.as_deref(),
        })
        .collect();
    view(&grouped)
}

/// The headline as a few lines, painted from the view's own text.
fn paint(view: &Analytics) -> String {
    let headline = &view.headline;
    let mut out = format!(
        "{} → {} · {}\n",
        view.from,
        view.to,
        view.realm.as_deref().unwrap_or("all realms")
    );
    out += &format!(
        "shipped        {} ({})\n",
        headline.shipped.count, headline.shipped.change_text
    );
    out += &format!("start → end    {}\n", headline.median.text);
    out += &format!(
        "waiting on you {} ({})\n",
        headline.waiting.text, headline.waiting.share_text
    );
    out += &format!("stopped by rule {}\n", headline.stopped_by_rule.count);
    out += &format!("ledger         {}\n", headline.ledger.text);
    out += &format!("reported cost  {}\n", headline.cost.text);
    for quarantined in &view.quarantined {
        out += &format!(
            "quarantined    {}: {}\n",
            quarantined.realm, quarantined.refusal
        );
    }
    out += &format!("not recorded yet: {}\n", view.not_recorded.join(" · "));
    out
}

#[cfg(test)]
mod tests;
