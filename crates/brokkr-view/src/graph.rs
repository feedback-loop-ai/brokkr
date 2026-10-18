//! A recipe's graph (#568, the Library's slice one): its state machine,
//! the resolved policy table, with the journal's traffic over a range on
//! every rule and state. One pure derivation, `now` passed in; `brokkr
//! graph --json` emits it and the console draws it, so a count is shown
//! one way (decision 0013). Nothing new is recorded: the table resolves as
//! the recipe does now, and each run's pins say whether it ran under it.

use std::fmt;

use brokkr_core::policy::Machine;
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::analytics::{listed, CostFigure, RuleCount, Span};
use crate::{AnalyticsHearth, Range, RuleSeverity};

mod facts;
mod figures;

pub use figures::graph;

/// Wire version of [`GraphView`], its own: the console pins it beside the
/// run, fleet and analytics views' shared version. 2: a recipe that does
/// not compile is a [`Table::Refused`], and `does_not_compile` names the
/// realm's recipes that do not.
pub const GRAPH_VIEW_VERSION: u32 = 2;

/// The recipe's table as it resolves in the realm now: its machine, and
/// the compiled manifest whose pins each run's are compared with.
#[derive(Clone, Copy)]
pub struct GraphTable<'a> {
    pub machine: &'a Machine,
    pub manifest: &'a Value,
}

/// Where the recipe's table came from, or why there is none to draw.
#[derive(Clone, Copy)]
pub enum GraphResolution<'a> {
    /// The recipe resolves in the realm and compiled.
    Compiled(GraphTable<'a>),
    /// No recipe of that name resolves in the realm.
    NotFound,
    /// The recipe resolves and does not compile against this Brokkr.
    Refused {
        /// The compiler's refusal, in its own words.
        reason: &'a str,
        /// The bundle directory it resolves to, as the realm names it.
        bundle: &'a str,
    },
}

impl<'a> GraphResolution<'a> {
    /// The table, when the recipe compiled.
    pub(crate) fn compiled(self) -> Option<GraphTable<'a>> {
        match self {
            GraphResolution::Compiled(table) => Some(table),
            GraphResolution::NotFound | GraphResolution::Refused { .. } => None,
        }
    }
}

/// What the graph is asked for (decision 0071 ruling 10's parameter
/// object).
pub struct GraphQuery<'a> {
    pub hearths: &'a [AnalyticsHearth<'a>],
    pub recipe: &'a str,
    pub realm: &'a str,
    pub table: GraphResolution<'a>,
    /// The recipes the realm resolves by name: its shelf and its bundle
    /// directories.
    pub shelved: &'a [String],
    /// Those of them that do not compile against this Brokkr.
    pub does_not_compile: &'a [String],
    pub range: Range,
    pub now: &'a str,
}

/// Why no graph was derived. Its `Display` is the operator's text
/// (decision 0071 ruling 8), by hand as [`crate::AnalyticsError`]'s is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// No recipe of that name resolves in the realm and no run carries it.
    UnknownRecipe {
        recipe: String,
        realm: String,
        known: Vec<String>,
    },
    /// A `now` that is not an RFC 3339 time.
    Clock(String),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&match self {
            GraphError::UnknownRecipe {
                recipe,
                realm,
                known,
            } => match known.is_empty() {
                true => format!("no recipe '{recipe}' in {realm}: it knows no recipe"),
                false => format!(
                    "no recipe '{recipe}' in {realm}: the recipes it knows are {}",
                    listed(known)
                ),
            },
            GraphError::Clock(now) => {
                format!("the clock read '{now}', which is not an RFC 3339 time")
            }
        })
    }
}

// ---------------------------------------------------------------- model

#[derive(Serialize, Debug, PartialEq)]
pub struct GraphView {
    pub view_version: u32,
    pub realm: String,
    pub recipe: String,
    pub range: Range,
    pub from: String,
    pub to: String,
    /// The realm's recipes: those it resolves by name and those its runs
    /// carry, sorted.
    pub recipes: Vec<String>,
    /// Those of the realm's recipes that resolve and do not compile against
    /// this Brokkr, sorted: each is still drawn, with its refusal.
    pub does_not_compile: Vec<String>,
    pub table: Table,
    /// The runs the range keeps: those that ended in it, and those open.
    pub runs: usize,
    /// The table's initial state first, then the rest in its `phases` order.
    pub states: Vec<State>,
    /// Every rule of the table, in its order, or only the fired ones when
    /// the table was not found.
    pub rules: Vec<Edge>,
    /// The rules joining each pair of states, one link per pair in the
    /// order its first rule holds: what the graph draws. A park has none.
    pub links: Vec<Link>,
    pub never_fired: NeverFired,
    /// Rules that fired in the range and are not in the table now.
    pub retired: Vec<RuleCount>,
    /// Runs whose pinned table is not the one resolved now.
    pub drifted_runs: usize,
    pub drift_text: Option<String>,
    pub ends: Ends,
    pub live: Vec<LiveRun>,
}

/// Whether the table was found where the recipe resolves, and compiled.
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Table {
    Resolved,
    NotFound {
        text: String,
    },
    /// The recipe resolves and its bundle does not compile against this
    /// Brokkr: only the rules its runs fired are drawn.
    Refused {
        text: String,
        /// The compiler's refusal, in its own words.
        reason: String,
        /// The bundle directory the recipe resolves to.
        bundle: String,
    },
}

/// Where a state sits (design D3): in the row, initial first then in `phases` order, at the
/// row's end (the last terminal that completes a run), or beneath it
/// (`stop`, and every other terminal).
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    Row,
    End,
    Beneath,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct State {
    pub name: String,
    pub place: Place,
    /// The distinct runs that entered it in the range.
    pub entered: usize,
    /// From an entry to the run's next entry or terminal event.
    pub median: Span,
    pub p90: Span,
    pub cost: CostFigure,
    /// Its seats' tokens, summed and read apart as every console surface
    /// reads them: total, input, output and the cache hit.
    pub tokens: crate::TokenCells,
    /// The seats the latest run in the range pinned for it.
    pub seats: Vec<SeatPin>,
}

/// One seat as the run's manifest pinned it: its key and the provider and
/// model its agent resolution chose (decision 0016).
#[derive(Serialize, Debug, PartialEq)]
pub struct SeatPin {
    pub key: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub text: String,
}

/// How a rule moves the run (design D3), placed by the client by it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    /// To a later state of the row.
    Forward,
    /// Back to its own state, or a park that holds the run there.
    SelfLoop,
    /// To an earlier state, drawn above the row.
    Back,
    /// To a terminal.
    Terminal,
}

impl RuleKind {
    /// The design's word for it, which the wire carries.
    pub const fn word(self) -> &'static str {
        match self {
            RuleKind::Forward => "forward",
            RuleKind::SelfLoop => "self",
            RuleKind::Back => "back",
            RuleKind::Terminal => "terminal",
        }
    }
}

impl Serialize for RuleKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

#[derive(Serialize, Debug, PartialEq)]
pub struct Edge {
    pub id: String,
    pub from: String,
    /// `None` for a rule that parks the run (decision 0022).
    pub next: Option<String>,
    pub result: String,
    /// Each condition in Brokkr's words.
    pub when: Vec<String>,
    pub severity: Option<RuleSeverity>,
    /// Its `transition/decided` events in the range.
    pub count: usize,
    pub kind: RuleKind,
    pub text: String,
    /// The runs that fired it in the range, the latest firing of each, the
    /// newest first and at most [`LATEST_RUNS`]: what a picked transition
    /// shows, so a failure leads to the runs that failed.
    pub latest: Vec<RuleRun>,
}

/// How many of a rule's runs a picked transition lists.
pub(crate) const LATEST_RUNS: usize = 5;

/// One run that fired a rule: its id, its fleet label, and when.
#[derive(Serialize, Debug, PartialEq)]
pub struct RuleRun {
    pub run_id: String,
    pub label: String,
    pub at: String,
}

/// Every rule from one state to one next state, as one link: the runs it
/// moved in the range and the outcomes that moved them (design D3).
#[derive(Serialize, Debug, PartialEq)]
pub struct Link {
    pub from: String,
    pub next: String,
    pub kind: RuleKind,
    /// The rules' counts, summed.
    pub count: usize,
    /// How many of the table's rules the link joins.
    pub rules: usize,
    /// One per result that fired, its rules' counts summed, the most runs
    /// first: `pass 21`, `residual 12`. Empty when none fired.
    pub outcomes: Vec<Outcome>,
    /// The outcomes' words joined, or empty when none fired.
    pub text: String,
}

/// A result's runs over one link: what the seat said, and how often.
#[derive(Serialize, Debug, PartialEq)]
pub struct Outcome {
    pub result: String,
    pub count: usize,
    pub text: String,
}

/// The table's rules that never fired in the range, of all of them.
#[derive(Serialize, Debug, PartialEq)]
pub struct NeverFired {
    pub count: usize,
    pub of: usize,
    pub text: String,
}

/// How the runs the range keeps ended (design D5).
#[derive(Serialize, Debug, PartialEq)]
pub struct Ends {
    pub done: usize,
    /// Stopped by a rule, by each rule, the most frequent first.
    pub stop: Vec<RuleCount>,
    pub stopped: usize,
    /// Stopped by the operator.
    pub operator: usize,
    pub open: usize,
    pub text: String,
}

/// One open run: its fleet title and the state it is in.
#[derive(Serialize, Debug, PartialEq)]
pub struct LiveRun {
    pub run_id: String,
    pub label: String,
    pub state: Option<String>,
}

#[cfg(test)]
mod tests;
