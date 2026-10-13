//! `brokkr analytics`: the cross-run readout over every hearth the world
//! names (#568). The figures are derived once, in
//! [`brokkr_view::analytics`]; this verb reads the hearths, hands them over
//! with the clock, and prints what comes back. It writes nothing
//! (decision 0014).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_runtime::realms::Hearth;
use brokkr_view::{Analytics, AnalyticsError, AnalyticsHearth, AnalyticsRun, Query, Range};

use crate::{fleet, hearths_of, now_rfc3339, Exit};

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct AnalyticsArgs {
    /// The world's map — every hearth it names is read (default
    /// ./realms.json when present).
    #[arg(long)]
    pub(crate) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without
    /// either, .forge/forge.db as always.
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
fn range(word: &str) -> Result<Range, RangeRefused> {
    word.parse().map_err(RangeRefused)
}

/// `brokkr analytics`: the readout for the range ending now.
pub(crate) fn analytics(workspace: &Path, args: AnalyticsArgs) -> Result<ExitCode> {
    let hearths = hearths_of(workspace, args.realms, args.db)?;
    let view = derive(&hearths, args.range, args.realm.as_deref(), &now_rfc3339())?;
    match args.json {
        true => println!("{}", serde_json::to_string_pretty(&view)?),
        false => print!("{}", paint(&view)),
    }
    Ok(Exit::Completed.into())
}

/// Every hearth read, quarantining what does not load, and the view
/// derived over them at `now`.
fn derive(hearths: &[Hearth], range: Range, realm: Option<&str>, now: &str) -> Result<Analytics> {
    let read: Vec<fleet::HearthRead> = hearths
        .iter()
        .map(|hearth| fleet::read_journal(&hearth.journal))
        .collect();
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
    let query = Query {
        hearths: &grouped,
        range,
        realm,
        now,
    };
    brokkr_view::analytics(&query).map_err(|error| anyhow::Error::msg(error.to_string()))
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
