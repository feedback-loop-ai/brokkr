//! The cross-run analytics readout (#568, `operator-console-analytics`):
//! one pure aggregation over every hearth's runs, for a range and an
//! optional realm, with `now` passed in. `brokkr analytics --json` emits
//! it and the console paints it, so one figure is shown one way (decision
//! 0013). What no journal records is listed in [`NOT_RECORDED`], never
//! invented.

use std::fmt;
use std::str::FromStr;

use brokkr_core::fold::RunState;
use brokkr_core::EventEnvelope;
use serde::{Serialize, Serializer};

mod facts;
mod figures;
mod recipes;
mod words;

pub(crate) use facts::{finished, in_range, is_open, Ending, Window};
pub use figures::analytics;
pub(crate) use figures::{cost_figure, middle, span, total};
pub(crate) use words::{listed, stamp};

const HOUR_MILLIS: i64 = 60 * 60 * 1000;
const DAY_MILLIS: i64 = 24 * HOUR_MILLIS;

/// What the journal does not record, so the readout states it instead of
/// a figure: the console's "not recorded yet" line.
pub(crate) const NOT_RECORDED: [&str; 4] = [
    "machine capacity over time",
    "cost limits",
    "costs for Codex and dsh",
    "reasons findings were dismissed",
];

/// How far back the readout looks from `now`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Range {
    Day,
    Week,
    Month,
}

impl Range {
    /// Every range, in the order a console offers them.
    pub(crate) const ALL: [Range; 3] = [Range::Day, Range::Week, Range::Month];

    /// The word the flag takes and the wire carries.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Range::Day => "24h",
            Range::Week => "7d",
            Range::Month => "30d",
        }
    }

    pub(crate) const fn millis(self) -> i64 {
        match self {
            Range::Day => DAY_MILLIS,
            Range::Week => 7 * DAY_MILLIS,
            Range::Month => 30 * DAY_MILLIS,
        }
    }
}

impl FromStr for Range {
    type Err = AnalyticsError;

    fn from_str(word: &str) -> Result<Range, AnalyticsError> {
        Range::ALL
            .into_iter()
            .find(|range| range.word() == word)
            .ok_or_else(|| AnalyticsError::UnknownRange(word.to_string()))
    }
}

impl Serialize for Range {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// Why the readout could not be derived. Its `Display` is the operator's
/// text (decision 0071 ruling 8), written by hand because this crate's
/// dependencies hold no `thiserror` (see [`crate::ScopeError`]); the shell
/// wraps the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyticsError {
    /// A range word outside [`Range::ALL`].
    UnknownRange(String),
    /// A `now` that is not an RFC 3339 time.
    Clock(String),
}

impl fmt::Display for AnalyticsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            AnalyticsError::UnknownRange(word) => {
                format!("unknown range '{word}': one of 24h, 7d or 30d")
            }
            AnalyticsError::Clock(now) => {
                format!("the clock read '{now}', which is not an RFC 3339 time")
            }
        })
    }
}

/// One run as a fleet reader loaded it: its journal and, when it folds,
/// its state.
pub struct AnalyticsRun<'a> {
    pub events: &'a [EventEnvelope],
    pub state: Option<&'a RunState>,
}

/// One hearth as a fleet reader hands it over.
pub struct AnalyticsHearth<'a> {
    /// The hearth's name, a run's realm when its pin names none.
    pub label: &'a str,
    /// The realms the map gives this journal, in map order.
    pub realms: &'a [String],
    pub journal: &'a str,
    pub runs: &'a [AnalyticsRun<'a>],
    /// The refusal that stopped the hearth being listed, if one did.
    pub refusal: Option<&'a str>,
}

/// What the readout is asked for (decision 0071 ruling 10's parameter
/// object).
pub struct Query<'a> {
    pub hearths: &'a [AnalyticsHearth<'a>],
    pub range: Range,
    pub realm: Option<&'a str>,
    pub now: &'a str,
}

// ---------------------------------------------------------------- model

#[derive(Serialize, Debug, PartialEq)]
pub struct Analytics {
    pub view_version: u32,
    pub range: Range,
    pub realm: Option<String>,
    pub from: String,
    pub to: String,
    pub headline: Headline,
    pub per_day: Vec<Day>,
    pub waiting_by_step: Vec<StepWait>,
    pub rework: Rework,
    pub models: Vec<ModelShare>,
    pub realms: Vec<RealmRow>,
    /// One row per realm and recipe with runs in the range.
    pub recipes: Vec<RecipeRow>,
    /// Hearths whose journal did not load, each with its refusal.
    pub quarantined: Vec<QuarantinedHearth>,
    pub not_recorded: [&'static str; 4],
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Headline {
    pub shipped: Shipped,
    /// Start to end of the runs that finished, parked time included.
    pub median: Span,
    pub waiting: Waiting,
    pub stopped_by_rule: StoppedByRule,
    pub cost: CostFigure,
}

/// Runs shipped in the range, against the equal range before it.
#[derive(Serialize, Debug, PartialEq)]
pub struct Shipped {
    pub count: usize,
    pub previous: usize,
    pub change: i64,
    pub change_text: String,
}

/// A median duration, `None` with the absence mark when nothing finished.
#[derive(Serialize, Debug, PartialEq)]
pub struct Span {
    pub millis: Option<i64>,
    pub text: String,
}

/// Time parked waiting on the operator: from a park to the operator's
/// next command, against all parked time, from a park until the run
/// moves again.
#[derive(Serialize, Debug, PartialEq)]
pub struct Waiting {
    pub waiting_millis: i64,
    pub parked_millis: i64,
    pub text: String,
    pub share_percent: Option<i64>,
    pub share_text: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct StoppedByRule {
    pub count: usize,
    /// The most frequent rule id, the lesser id on a tie.
    pub top: Option<RuleCount>,
    pub text: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct RuleCount {
    /// The rule, or `None` when the stop recorded none (decision 0071
    /// ruling 3: an absence is a variant, not a word).
    pub rule_id: Option<String>,
    pub count: usize,
}

/// Reported cost in USD, or the statement that nothing reported one.
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReportedCost {
    Reported { usd: f64 },
    NotReported,
}

impl ReportedCost {
    fn of(total: Option<f64>) -> ReportedCost {
        match total {
            Some(usd) => ReportedCost::Reported { usd },
            None => ReportedCost::NotReported,
        }
    }

    fn text(&self) -> String {
        match self {
            ReportedCost::Reported { usd } => format!("${usd:.2}"),
            ReportedCost::NotReported => "not reported".to_string(),
        }
    }
}

/// The reported cost and the harnesses behind it: those that reported a
/// cost, and those whose seats ran in the range and reported none.
#[derive(Serialize, Debug, PartialEq)]
pub struct CostFigure {
    pub cost: ReportedCost,
    pub text: String,
    pub reporting: Vec<String>,
    pub silent: Vec<String>,
    pub note: Option<String>,
}

/// One UTC day: the runs that shipped and that stopped on it.
#[derive(Serialize, Debug, PartialEq)]
pub struct Day {
    pub date: String,
    pub shipped: usize,
    pub stopped: usize,
}

/// Waiting on the operator in one step: the hours and the runs.
#[derive(Serialize, Debug, PartialEq)]
pub struct StepWait {
    pub phase: Option<String>,
    pub waiting_millis: i64,
    pub runs: usize,
    pub text: String,
}

/// How often work was sent back, over the runs that finished.
#[derive(Serialize, Debug, PartialEq)]
pub struct Rework {
    pub runs: usize,
    pub implement_visits: Option<f64>,
    pub implement_visits_text: String,
    /// Runs a review ruling sent back to a phase they had been in.
    pub sent_back: usize,
    /// Of the runs their first review sent back, those whose second
    /// review let them on.
    pub second_try_passed: usize,
    pub second_try_of: usize,
    pub second_try_percent: Option<i64>,
    pub second_try_text: String,
}

/// The share of step attempts one served model took.
#[derive(Serialize, Debug, PartialEq)]
pub struct ModelShare {
    /// The served model, or `None` when its seat reported none.
    pub model: Option<String>,
    pub attempts: usize,
    pub percent: i64,
    pub text: String,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct RealmRow {
    pub realm: String,
    pub shipped: usize,
    pub stopped: usize,
    pub median: Span,
    pub waiting_millis: i64,
    pub waiting_text: String,
    pub cost: ReportedCost,
    pub cost_text: String,
    /// Runs whose journal does not load or fold, so no figure reads them.
    pub unreadable_runs: usize,
}

/// One recipe of one realm over the runs the range keeps: those that ended
/// in it and those still open (design D6).
#[derive(Serialize, Debug, PartialEq)]
pub struct RecipeRow {
    pub realm: String,
    pub recipe: String,
    pub runs: usize,
    /// The runs that reached `done`.
    pub done: usize,
    /// `done` as a percent of `runs`.
    pub done_rate: f64,
    pub done_text: String,
    /// The reported cost over `runs`, with the harnesses behind it.
    pub cost_per_run: CostFigure,
    /// The tokens a run spent, on average over `runs`, read apart as the
    /// run view's seats are: total, input, output and the cache hit.
    pub tokens_per_run: crate::TokenCells,
    /// Whether no other priced recipe of the realm reaches `done` at least
    /// as often for at most as much, one strictly; absent when this recipe
    /// reported no cost.
    pub on_frontier: Option<bool>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct QuarantinedHearth {
    pub realm: String,
    pub journal: String,
    pub refusal: String,
}

#[cfg(test)]
pub(crate) mod tests;
