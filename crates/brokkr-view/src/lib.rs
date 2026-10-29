//! `brokkr-view` — one derivation, two surfaces (decision 0013).
//!
//! Pure functions over a run's journal (plus the folded `RunState`)
//! producing serializable view models: run rows, the run summary and its
//! ruling, participants, live seat lines, the phase rail with each
//! phase's inner topology, and the decision trail. There is no I/O, no
//! rendering, and no terminal or DOM concept here — the manifest depends
//! on exactly `brokkr-core`, `serde`, `serde_json` and `unicode-width`,
//! so that property is a compile error rather than a review convention,
//! and the absence of a clock is what forces `now` to be a parameter.
//!
//! Every displayed scalar reaches a caller as a **(structured value,
//! rendered text) pair**. The console's renderer is JavaScript and
//! cannot call a Rust helper: a model carrying only `cost: Option<f64>`
//! would force the page to re-derive `'Σ $' + cost.toFixed(4)` — the
//! duplication this crate exists to end. The structured value stays so
//! `--json` is lossless.
//!
//! Derivation reads `serde_json::Value` with the same `typeof` guards
//! the JavaScript uses, never typed payload structs: a `#[serde(default)]`
//! would invent `""` where the console shows a deliberate absence with a
//! reason, and a wrong-typed field would make a whole event vanish where
//! the console renders `?` and keeps the row. Both are repair
//! (decision 0001).

#![forbid(unsafe_code)]

mod entry;
mod rail;
pub use entry::{entry_sentence, ContinuedView, EntryView};

mod analytics;
mod attempts;
pub mod capability_calls;
mod clarify;
mod commands;
mod compare;
mod dashboard;
mod fleet;
mod graph;
mod items;
pub mod js;
mod operator_trail;
mod participant;
mod run_summary;
mod scope;
mod tokens;
pub mod transcript;
use tokens::usage_cells;
pub use tokens::TokenCells;

use crate::operator_trail::{commands_of, operator_what};
pub use analytics::Range;
pub use analytics::{analytics, Analytics, AnalyticsError, AnalyticsHearth, AnalyticsRun, Query};
pub use brokkr_core::policy::RuleSeverity;
use brokkr_core::seat::meter;
use brokkr_core::seat::{
    add_token, boundary_entries, count_item, finish_usage, stamp_of, transcript_of,
};
pub use brokkr_core::seat::{reported_cost, seat_costs, SeatCost, TokenUsage, Transcript};
pub use clarify::Clarify;
use clarify::{clarify, worktree};
use commands::{commands, unavailable};
pub use commands::{member_retry_refusal, operator_commands, operator_refusal};
mod started;
mod told;
use told::{Finish, Told};
mod composed;
mod seat_view;
pub use commands::{Admission, Command, CommandRefusal, Disabled, Offered, Unavailable};
pub use compare::{
    first_divergence, resolution_divergence, resolutions, resolved, FirstDivergence, Resolution,
    Sides,
};
pub use composed::{composed_view, composed_whole, Parts};
use dashboard::dashboard;
pub use dashboard::{working_checkpoints, Concluded, Dashboard, Decision, Outcome, Ruled, Visit};
pub use fleet::{
    at_work, behind_line, fleet_rows, need, page, ranges, run_row, run_rows, section_of,
    section_of_rows, sections, stopping_detail, summary_row, title, title_within, window, wrap,
    Acknowledgement, Cursor, CursorError, HearthEntries, Hire, Need, Page, Paged, Quarantine,
    Refusal, RunRow, Section, Standing, Verdict, AHEAD_ROWS, PAGE_ROWS, TITLE_COLUMNS,
    VERDICT_COLUMNS,
};
pub use graph::{
    graph, GraphError, GraphQuery, GraphResolution, GraphTable, GraphView, Table,
    GRAPH_VIEW_VERSION,
};
pub use participant::Subject;
pub use run_summary::{summary, Summary};
pub use scope::{
    keeps_participant, keeps_phase, keeps_row, lens_for, Lens, Membership, Scope, ScopeError,
    Scopes,
};
pub use seat_view::page_rows;
use seat_view::{attempt_boundary, boundaries_of, row_of, Boundaries};
pub use started::CarriedFrom;

use std::collections::BTreeMap;

// The core's ruling record; the dashboard's `Decision` holds this name.
use brokkr_core::decision::Decision as Decided;
use brokkr_core::fold::RunState;
use brokkr_core::realms::Boundary;
use brokkr_core::residual::{closings, residuals, Mark};
use brokkr_core::summary::Stated;
use brokkr_core::{EventEnvelope, EventType};
use serde::Serialize;
use serde_json::Value;

/// Wire version of every model in this crate. `--json` consumers pin it.
/// Bumped to 2 by decision 0016: participants gained `provenance` and
/// the run view gained `notices`. Bumped to 3 when the phase rail gained
/// `returns` — the roads back decision 0022 made real. The precedent is
/// 0016's: an additive model field moves the wire version, because a
/// consumer pinning the old one is entitled to know the shape grew.
/// Bumped to 4 by decision 0031: participants gained served-model evidence.
/// Bumped to 5 by decision 0032: every participant gained the common
/// transcript shape and its rendered cell.
/// Bumped to 6 by decision 0034: seat accounting gained one structured
/// token record and one rendered cell, including cache writes.
/// Bumped to 7 by decision 0035: the hire gained its second half.
/// Participants and checkpoints carry the effort, the token record
/// carries the reasoning subset, and the pin the plan asked for stands
/// beside the effort the harness says it applied — two cells, because
/// ruling 6 keeps them two facts.
/// Bumped to 8 by decision 0041 ruling 6: the run summary and triage
/// phase expose the strategy class derived by the fold.
/// Bumped to 10 by two shape changes that met in one merge. Decision
/// 0046 ruling 3: every model cell gained the boundary cell beside it,
/// and the run view gained its run-level boundary fact — the plain word
/// in the data, the adjective *unboxed* only in rendered text. Decision
/// 0047 ruling 3: a residual finding, the run rows that carry it and
/// the ruling it was read from all state the operator's supersede
/// annotation when one names it.
/// Bumped to 11 by #376, pending the operator's ruling: a participant's
/// `cost` became every attempt's reported cost summed (the rule
/// [`reported_cost`] states), where it was the last attempt's alone, and
/// participants gained `last_attempt_cost` beside it.
/// Bumped to 12 by #491: a run row gained its `title` and its `verdict`.
/// Bumped to 13 by #503: a run row gained its `hire`.
/// Bumped to 14 by #503's second round: the run view gained its `dashboard`.
/// Bumped to 15 by #503's landing: a run row gained its `quarantine`.
/// Bumped to 16 by decision 0065: a checkpoint gained its capability call.
/// Bumped to 17 by #351: a participant gained its `working`.
/// Bumped to 18 by #351's third slice: the run view gained its `scopes`.
/// Bumped to 19 by #568: the run view gained `commands`, `unavailable`,
/// `clarify` and `worktree`, and the analytics readout began.
/// Bumped to 20 by #568's second slice: the run view gained its `recipe`.
/// Bumped to 21 by #511: a run row gained its `acknowledgement`.
/// Bumped to 22 by #568's console tab: a participant gained its
/// `tokens`, the total, input, output and cache-hit cells.
/// Bumped to 23 by #568's Library: the analytics readout gained `recipes`.
/// Bumped to 24 by #568's run view: a participant gained its `visit`.
/// Bumped to 25 by #753 (decision 0089 ruling 6): a page of runs gained
/// `next`, and a section lists its runs by last activity.
/// Bumped to 26 by #787: a participant gained its `carried_from`, and the
/// status word `carried` (decision 0093).
/// Bumped to 27 by decision 0094: the run view records its entry and continuation.
/// Bumped to 28 by #909: a run whose accepted stop has no ending stands
/// as `stopping`, with its detail, instead of `running`.
pub const VIEW_VERSION: u32 = 28;

/// The note every absent boundary cell carries (decision 0046 ruling 3;
/// design DD13): a journal written before the boundary was named, a
/// running seat whose finishing record has not landed, and an entry the
/// reader refuses all say this one thing — and never a default word.
pub const NO_BOUNDARY: &str = "no boundary recorded";

/// The deliberate-absence mark: a value the journal never carries reads
/// as a dim dash with its reason, never as an empty cell that looks like
/// broken data.
pub const ABSENT: &str = "—";

const KNOWN_STATUS: [&str; 4] = ["completed", "stopped", "awaiting_operator", "running"];

/// The events the decision trail hides: checkpoints are the transcript's
/// job and effect plumbing is forensics.
const TRAIL_SKIP: [EventType; 3] = [
    EventType::EffectCheckpointed,
    EventType::EffectRequested,
    EventType::EffectStarted,
];

// ---------------------------------------------------------------- models

/// One displayed scalar: its rendered text plus whether the journal
/// carries it at all, and why not when it does not.
#[derive(Serialize, Debug, PartialEq)]
pub struct Cell {
    pub text: String,
    pub absent: bool,
    pub note: Option<String>,
}

/// A run as the store lists it, with its folded state when the journal
/// reads back.
pub struct RunEntry<'a> {
    pub run_id: &'a str,
    pub feature: &'a str,
    pub created_at: &'a str,
    /// When the journal last moved: its last event's `recorded_at`, or
    /// none when it would not load. The fleet dates a finished run by it.
    pub last_recorded_at: Option<&'a str>,
    pub state: Option<&'a RunState>,
    /// Why the state is absent: a fleet read quarantines a run whose
    /// journal does not load or fold rather than losing the whole fleet
    /// with it, and the refusal is the row's whole account of itself.
    /// Nothing is repaired here (README law 2) — the refusal is reported.
    pub detail: Option<Refusal<'a>>,
    /// What [`residual_findings`] derived for this run, marks included.
    /// A caller that has not read the run's events — or read a journal
    /// that would not open at all — states none, which is exactly what
    /// a fleet listing said before decision 0047.
    pub residuals: &'a [ResidualFinding],
}

#[derive(Serialize)]
pub struct RunsView {
    pub view_version: u32,
    pub runs: Vec<RunRow>,
    pub count: usize,
    /// Where the next page starts, when one page was asked for (decision
    /// 0089 ruling 6); a listing of every run states none.
    #[serde(flatten)]
    pub page: Option<Paged>,
}

impl RunsView {
    /// Every run of `runs`, listed whole.
    pub fn of(runs: Vec<RunRow>) -> RunsView {
        RunsView {
            view_version: VIEW_VERSION,
            count: runs.len(),
            runs,
            page: None,
        }
    }
}

/// One hearth's runs, under the realm name they belong to (decision 0026
/// ruling 3). Journals never merge (ruling 5): this is a listing of one
/// journal, standing beside the others, never folded into them.
#[derive(Serialize)]
pub struct RealmRuns {
    /// The realms sharing this hearth, as the world names them.
    pub realm: String,
    pub journal: String,
    pub runs: Vec<RunRow>,
    pub count: usize,
    /// Why this hearth lists no runs, when the journal could not be
    /// read at all: the open's own words. A world does not lose its
    /// other hearths because one realm's journal is missing, and
    /// nothing is repaired — the refusal is reported (README law 2).
    pub detail: Option<String>,
}

/// The fleet of a many-hearth world: one section per distinct journal,
/// in map order. The one derivation both `brokkr runs` surfaces render
/// (decision 0013), so text and `--json` cannot disagree about which
/// realm a run belongs to.
#[derive(Serialize)]
pub struct FleetView {
    pub view_version: u32,
    pub realms: Vec<RealmRuns>,
    /// Every run the world holds, across every hearth — a count, never
    /// a merged listing.
    pub count: usize,
}

#[derive(Serialize)]
pub struct Ruling {
    pub rule_id: String,
    /// A closed-set key, never a class name from the journal.
    pub severity_class: String,
    pub from: String,
    pub next: String,
    pub result: Option<String>,
    pub inputs: Vec<(String, String)>,
    pub problem: Option<String>,
    /// The operator's supersede annotation for THIS ruling, when one
    /// names a finding read from it (decision 0047 ruling 3). The
    /// ruling itself is untouched — a run stopped on a security hold
    /// stays stopped on it (ruling 6); the mark says only that the
    /// finding is closed elsewhere.
    pub superseded: Option<Superseded>,
}

/// What a seat is doing, or did. Live work keeps the tool and target
/// split so the console's tooltip stays on the target span.
#[derive(Serialize)]
pub struct Activity {
    pub text: String,
    pub absent: bool,
    pub note: Option<String>,
    pub tool: Option<String>,
    pub target_short: Option<String>,
    /// Present only when the target was shortened.
    pub target_full: Option<String>,
}

/// A served model and the boundary its seat's hands stood behind, as one
/// unit (decision 0046 ruling 3; design DD12). Flattened onto every
/// carrier as `served`, so the wire keeps `model` and gains `boundary`
/// as siblings while a Rust renderer cannot take the model without the
/// boundary in reach. The boundary cell carries the plain word — one of
/// the five, or decision 0031's `not applicable` for a site without
/// hands — and never the adjective; a journal that recorded none renders
/// the absent mark with [`NO_BOUNDARY`].
#[derive(Serialize)]
pub struct ModelAtBoundary {
    /// The provider-reported model. No adapter default or abstract agent
    /// choice is substituted when the journal carries no such report.
    pub model: Cell,
    pub boundary: Cell,
}

/// The run-level boundary fact (decision 0046 ruling 3; design DD12):
/// the word every boxed site stood under, whether the run is rendered
/// *unboxed*, and the one rendered line every run header prints.
#[derive(Serialize)]
pub struct RunBoundary {
    /// The run's plain word — several, joined, if its boxed sites
    /// disagree. Absent with [`NO_BOUNDARY`] when the manifest declares
    /// hands and the journal holds no valid entry; absent with no note
    /// when the run declares no hands at all.
    pub word: Cell,
    /// True exactly when a valid `effect/started.boundary` entry has
    /// `gate` true and a word of `harness` or `open`.
    pub unboxed: bool,
    /// The rendered value: `harness · unboxed`, `namespace`, `not
    /// recorded`, or empty for a run that boxes nothing. The only
    /// place the adjective is spelled, so no surface composes its own.
    pub text: String,
}

#[derive(Serialize)]
pub struct CheckpointRow {
    pub turn: Cell,
    pub step: String,
    #[serde(flatten)]
    pub served: ModelAtBoundary,
    /// The effort this turn was configured with, as the harness echoed
    /// it. Per-turn because both codex and claude write it per turn, so
    /// a thread that changed effort mid-seat says so row by row rather
    /// than averaging into one seat-level answer.
    pub effort: Cell,
    pub usage: Cell,
    pub target: Cell,
    pub target_full: Option<String>,
    /// The capability call this checkpoint records (decision 0065).
    pub capability_call: capability_calls::CallEvidence,
    pub recorded_at: String,
}

/// Which agent resolution selected one invocation's adapter and pinned
/// model (decision 0016). This is plan provenance, deliberately distinct
/// from the provider-reported served model on [`Participant`]. `line` is the
/// rendered sentence every surface prints, so no surface can decide on
/// its own how to describe a fallback — or quietly stop mentioning one.
#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub agent: String,
    pub model: String,
    /// The effort PINNED with that model (decision 0035 ruling 5): what
    /// the plan asked for, never what a harness says it applied. `None`
    /// for a run journaled before this decision, and for a candidate an
    /// effortless provider serves — the two are told apart by the cell's
    /// note, never by substituting one for the other.
    pub effort: Option<String>,
    pub provider: String,
    pub chain_index: u64,
    /// Brokkr never claims the second choice equals the first.
    pub fallback: bool,
    pub line: String,
}

/// A run-level fact an operator must see rather than find: a
/// compile-time capability gap the agent marked optional, or a fallback
/// that actually happened. A notice that only lands in JSON nobody reads
/// is the ruling's "never nothing" defeated.
#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct RunNotice {
    /// A closed vocabulary: `capability-gap` or `fallback`.
    pub kind: String,
    pub text: String,
}

#[derive(Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the served `working` beside the three aggregation flags, by the operator's 2026-10-07 ruling on #351"
)]
pub struct Participant {
    pub carried_from: Option<CarriedFrom>,
    pub key: String,
    pub label: String,
    pub member: Option<String>,
    /// Precomputed scope membership: no surface implements the predicate.
    pub phase: Option<String>,
    /// Which visit to `phase` the seat was asked in, from 1: how many
    /// times the journal had entered that phase when the seat was
    /// requested, so a surface tells a return's second pass from the
    /// first without counting. `None` when the seat names no phase, or
    /// one the journal had not entered.
    pub visit: Option<u64>,
    pub status: String,
    pub status_class: String,
    /// Whether the participant is still at work, and so can still gain
    /// prose: served so the browser paints it rather than testing the
    /// status word itself (decision 0013; #351).
    pub working: bool,
    pub attempts: u64,
    pub turns: Option<u64>,
    pub turns_aggregated: bool,
    pub turns_cell: Cell,
    /// What the seat spent: every attempt's reported cost, summed (the
    /// rule [`reported_cost`] states), the rule `brokkr costs` sums by.
    pub cost: Option<f64>,
    /// The last attempt's finishing cost alone, beside the sum and never
    /// in its place.
    pub last_attempt_cost: Option<f64>,
    pub cost_aggregated: bool,
    pub cost_cell: Cell,
    pub usage: Option<TokenUsage>,
    pub usage_aggregated: bool,
    pub usage_cell: Cell,
    /// The figures a console's seat row reads apart: total, input,
    /// output and the cache hit (#568).
    pub tokens: TokenCells,
    /// The served model and its boundary, flattened: `model` and
    /// `boundary` are siblings on the wire.
    #[serde(flatten)]
    pub served: ModelAtBoundary,
    /// The effort the harness echoed as APPLIED, after every profile and
    /// plugin layer (decision 0035 ruling 3). CONFIGURATION, not a
    /// measurement of what the model did — `usage.reasoning_output_tokens`
    /// is the only meter here, and on at least one live lane the two
    /// disagree. Never filled from `effort_pin`: ruling 6 keeps the two
    /// apart, and a journal written before this decision stays visibly
    /// absent rather than borrowing the plan's answer.
    pub effort: Cell,
    /// The effort the PLAN pinned, read from this invocation's agent
    /// resolution. Also configuration, and the other half of the pair.
    pub effort_pin: Cell,
    pub activity: Activity,
    pub member_count: usize,
    pub transcript: Option<Transcript>,
    pub transcript_cell: Cell,
    /// Compatibility for the Claude transcript reader and journals written
    /// before decision 0032. New non-Claude locators never enter this field.
    pub session_id: Option<String>,
    pub terminal_line: Cell,
    pub checkpoints: Vec<CheckpointRow>,
    /// Absent for an inline seat: it has no agent-resolution provenance.
    pub provenance: Option<Provenance>,
}

#[derive(Serialize)]
pub struct LiveLine {
    pub label: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct Node {
    pub label: String,
    pub key: String,
    pub state: String,
    pub state_class: String,
    #[serde(flatten)]
    pub served: ModelAtBoundary,
}

#[derive(Serialize)]
pub struct Column {
    pub label: Option<String>,
    pub nodes: Vec<Node>,
}

#[derive(Serialize)]
pub struct Phase {
    pub name: String,
    /// The triage ruling, beside that phase only. Older and non-triage
    /// runs carry no class rather than an inferred one.
    pub strategy: Option<String>,
    pub visits: u64,
    pub current: bool,
    /// No inner structure was observed: the rail draws a single node.
    pub plain: bool,
    pub columns: Vec<Column>,
    /// The roads that sent the run BACK here — the road in, not the
    /// count of arrivals. One [`Road`] per distinct departure the journal
    /// recorded, in the order it first recorded them, so a surface that
    /// draws the return draws it once however many times it was taken,
    /// and says how many (decision 0022's reforging is the machine's only
    /// backward edge today; nothing here names it).
    ///
    /// Derived from the `transition/decided` that CAUSED the revisit,
    /// never inferred from `visits > 1`: the count says a phase was
    /// entered twice, and only the transition says where from. A ruling
    /// is backward only when this phase stands strictly before its
    /// `from` on the rail (first-entry order): the forward pass after a
    /// return, and a phase ruled onto itself, are arrivals, not roads.
    pub returns: Vec<Road>,
}

/// One road back into a phase (#568).
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Road {
    /// The phase whose ruling sent the run back.
    pub from: String,
    /// How many times the journal ruled this road.
    pub taken: u64,
    /// The rules that ruled it, each once, the most recently ruled last.
    /// A ruling that records no rule adds to `taken` alone.
    pub rules: Vec<String>,
}

/// The human answer to "what happened here", as parts a surface may
/// arrange and as the plain composition a surface may simply print.
#[derive(Serialize)]
pub struct What {
    pub text: String,
    pub badge: Option<String>,
    pub badge_class: Option<String>,
    pub arrow: Option<String>,
    pub problem: Option<String>,
}

#[derive(Serialize)]
pub struct JournalRow {
    pub seq: u64,
    pub causation_seq: Option<u64>,
    pub event_type: String,
    pub recorded_at: String,
    /// False for the plumbing the decision trail hides.
    pub in_trail: bool,
    /// Precomputed scope membership: no surface implements the predicate.
    pub phases: Vec<String>,
    pub what: What,
    pub label: Cell,
    #[serde(flatten)]
    pub served: ModelAtBoundary,
    pub payload_json: String,
}

#[derive(Serialize)]
pub struct RunView {
    pub entry: Option<EntryView>,
    pub view_version: u32,
    /// `None` when the journal does not fold — never a guessed state.
    pub summary: Option<Summary>,
    pub ruling: Option<Ruling>,
    pub participants: Vec<Participant>,
    pub live: Vec<LiveLine>,
    pub phases: Vec<Phase>,
    /// Run-level facts every surface shows (decision 0016): optional
    /// capability gaps recorded at compile time, and fallbacks that
    /// actually happened.
    pub notices: Vec<RunNotice>,
    /// The run-level boundary fact every run header prints (decision
    /// 0046 ruling 3), derived once.
    pub boundary: RunBoundary,
    pub journal: Vec<JournalRow>,
    /// Every event, unfiltered: the console's `full journal · N events`.
    pub event_count: usize,
    /// The path, the last ruling and the last seats' notes (#503).
    pub dashboard: Dashboard,
    /// What each phase scope and each seat scope keeps, decided by the
    /// terminal's rule (#351), so the browser paints it. Only a surface
    /// that serves the view decides it, by [`RunView::with_scopes`]: the
    /// terminal asks the predicates instead and never pays for it (#624).
    pub scopes: Option<Scopes>,
    /// The four commands, each admitted or refused (#568); empty when the
    /// journal does not fold.
    pub commands: Vec<Offered>,
    /// The actions a console shows disabled, with their reasons (#568).
    pub unavailable: Vec<Disabled>,
    /// The open questions of a run parked by clarify's ruling (#568).
    pub clarify: Option<Clarify>,
    /// The working tree the run was started in, when its pin names one.
    pub worktree: Option<String>,
    /// The recipe the run started under, as its start recorded it (#568).
    pub recipe: Option<String>,
}

// --------------------------------------------------------------- helpers

fn cell_of(text: Option<String>, note: Option<&str>) -> Cell {
    match text {
        Some(text) => Cell {
            text,
            absent: false,
            note: None,
        },
        None => Cell {
            text: ABSENT.to_string(),
            absent: true,
            note: note.map(str::to_string),
        },
    }
}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// JS truthiness, for the places the console tests a raw journal value.
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(Value::Number(number)) => number.as_f64() != Some(0.0),
        Some(Value::String(text)) => !text.is_empty(),
        _ => true,
    }
}

/// The second path segment of an event type, the console's last-resort
/// trail label. Written as trimming rather than splitting so there is no
/// unreachable "no slash" arm to leave the coverage gate red.
fn type_tail(type_str: &str) -> &str {
    type_str
        .trim_start_matches(|character| character != '/')
        .trim_start_matches('/')
}

/// The three terminal effect events and the (status, badge class) pair
/// each implies. Pairing them keeps the class total: a derived status
/// always has a class, so no surface needs an unreachable fallback.
fn terminal_status(event_type: EventType) -> Option<(&'static str, &'static str)> {
    match event_type {
        EventType::EffectSucceeded => Some(SUCCEEDED),
        EventType::EffectFailed => Some(FAILED),
        EventType::EffectIndeterminate => Some(INDETERMINATE),
        _ => None,
    }
}

const WORKING: (&str, &str) = ("working", "running");
const SUCCEEDED: (&str, &str) = ("succeeded", "completed");
const FAILED: (&str, &str) = ("failed", "stopped");
const INDETERMINATE: (&str, &str) = ("indeterminate", "awaiting_operator");
/// A member its effect's last start carries (decision 0093).
const CARRIED: (&str, &str) = ("carried", "completed");

/// The allowlist a panel member's own outcome must pass to override the
/// effect's terminal status. `working` is in it, so a member really can
/// be pinned to `working` after the effect concluded.
fn outcome_status(outcome: &str) -> Option<(&'static str, &'static str)> {
    match outcome {
        "working" => Some(WORKING),
        "succeeded" => Some(SUCCEEDED),
        "failed" => Some(FAILED),
        "indeterminate" => Some(INDETERMINATE),
        _ => None,
    }
}

/// A node's (state, class) pair for the inner topology.
fn outcome_state(token: &str) -> Option<(&'static str, &'static str)> {
    match token {
        "succeeded" => Some(("finished", "on-phosphor")),
        "failed" => Some(("failed", "on-halt")),
        "indeterminate" => Some(("indeterminate", "on-park")),
        _ => None,
    }
}

const NODE_FINISHED: (&str, &str) = ("finished", "on-phosphor");
const NODE_ACTIVE: (&str, &str) = ("active", "in-active");

/// The severity class of a payload's `severity`.
fn severity_class(payload: &Value) -> &'static str {
    class_of_severity(field(payload, "severity").and_then(RuleSeverity::named))
}

/// The fixed severity table. A severity outside the vocabulary, or none,
/// falls back to the empty key — journal data never names a class.
fn class_of_severity(severity: Option<RuleSeverity>) -> &'static str {
    match severity {
        Some(RuleSeverity::Flagged) => "awaiting_operator",
        Some(RuleSeverity::Hard) => "stopped",
        Some(RuleSeverity::Normal) | None => "",
    }
}

/// `(p.result && typeof p.result === 'object') ? p.result.result : p.result`,
/// kept only when the outcome is a string.
fn result_token(payload: &Value) -> Option<&str> {
    match payload.get("result") {
        Some(Value::Object(map)) => map.get("result").and_then(Value::as_str),
        other => other.and_then(Value::as_str),
    }
}

/// Wall-clock between two journal timestamps, humanized exactly as the
/// console humanizes it: seconds below a minute, zero-padded seconds
/// below an hour, and **seconds dropped entirely** at an hour and above.
pub fn fmt_dur(from_iso: &str, to_iso: &str) -> Option<String> {
    let to = brokkr_core::stamp::parse_millis(to_iso)?;
    let from = brokkr_core::stamp::parse_millis(from_iso)?;
    fmt_millis(to - from)
}

/// `millis` humanized as [`fmt_dur`] humanizes the time between two
/// stamps, and nothing for a negative span.
pub(crate) fn fmt_millis(millis: i64) -> Option<String> {
    if millis < 0 {
        return None;
    }
    let seconds = js::round_half_up(millis as f64 / 1000.0);
    if seconds < 60 {
        return Some(format!("{seconds}s"));
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return Some(format!("{minutes}m{:02}s", seconds % 60));
    }
    Some(format!("{}h{:02}m", minutes / 60, minutes % 60))
}

/// How long ago a run was created. `now` is a parameter: this crate has
/// no clock, which is also what makes the golden tests deterministic.
pub fn age(created_at: &str, now: &str) -> Option<String> {
    fmt_dur(created_at, now)
}

/// Targets are file paths (record-time ruling); the tail is the
/// informative end, so long ones display tail-first.
pub fn short_target(target: &str) -> String {
    if js::len(target) <= 44 {
        return target.to_string();
    }
    let mut parts: Vec<&str> = target.split('/').collect();
    let mut tail = parts.remove(parts.len() - 1).to_string();
    while parts.len() > 1 {
        let candidate = parts[parts.len() - 1];
        if js::len(&tail) + js::len(candidate) + 1 > 40 {
            break;
        }
        tail = format!("{candidate}/{tail}");
        parts.pop();
    }
    format!("…/{tail}")
}

/// Clamp to a column width on `char` boundaries — byte-slicing a UTF-8
/// feature panics, and a panic in `brokkr runs` is worse than any
/// misalignment.
pub fn clamp(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}

// ------------------------------------------------------------ residuals

/// The run and the ruling an operator's supersede names as having
/// closed a finding (decision 0047 ruling 1). Keyed by realm the way
/// decision 0026 ruling 3 keys every fleet fact — `None` in a one-hearth
/// world, where there is only ever one journal to have read it in.
#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct SupersededBy {
    pub realm: Option<String>,
    pub run_id: String,
    pub seq: u64,
}

/// The operator's word that a residual finding is closed elsewhere
/// (decision 0047 ruling 1), as the journal carries it.
#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct Superseded {
    /// The ANNOTATION's own sequence number, on the annotated run — the
    /// citation a reader follows to the operator's command, not the
    /// sequence of the ruling it closes.
    pub seq: u64,
    pub by: SupersededBy,
    pub reason: String,
    pub operator: String,
    pub recorded_at: String,
}

/// How a mark reads on a rendered line: `realm/run seq N` where the
/// annotation named a realm, the bare run id where it did not — the
/// same shape a fleet read cites every fact by (decision 0026 ruling 3).
/// One derivation, so no surface renders the citation its own way.
pub fn cited(by: &SupersededBy) -> String {
    match &by.realm {
        Some(realm) => format!("{realm}/{} seq {}", by.run_id, by.seq),
        None => format!("{} seq {}", by.run_id, by.seq),
    }
}

/// Every supersede a run's journal carries, by the sequence number of
/// the ruling each closes, as core reads them ([`closings`]): a finding
/// named by more than one annotation carries the LAST in journal order
/// (decision 0047 ruling 3), and what an annotation does not say reads
/// as the absence mark.
fn supersedes(events: &[EventEnvelope]) -> BTreeMap<u64, Superseded> {
    closings(events)
        .into_iter()
        .map(|(finding, mark)| (finding, superseded_of(mark)))
        .collect()
}

/// A mark as every surface shows it, the absence mark where it is silent.
fn superseded_of(mark: Mark) -> Superseded {
    Superseded {
        seq: mark.seq,
        by: SupersededBy {
            realm: mark.by.realm,
            run_id: mark.by.run_id,
            seq: mark.by.seq,
        },
        reason: mark.reason.unwrap_or_else(|| ABSENT.to_string()),
        operator: mark.operator.unwrap_or_else(|| ABSENT.to_string()),
        recorded_at: mark.recorded_at,
    }
}

/// One residual finding, with the journal fact it was read from.
#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct ResidualFinding {
    pub run_id: String,
    /// The sequence number of the ruling this was read from — the
    /// citation a reader can go and check.
    pub seq: u64,
    /// The phase that ruled: `verify` or `review`.
    pub phase: String,
    pub rule_id: String,
    /// The rule input the finding is derived from, by its exact name in
    /// the evaluator's closed vocabulary.
    pub input: String,
    pub value: String,
    /// The rendered sentence every surface prints, citations included —
    /// the supersede mark among them, when the finding carries one.
    pub line: String,
    /// The operator's word that this finding is closed, or `None`
    /// (decision 0047 ruling 3). The finding itself is unchanged: it
    /// stays in the journal and in every readout, marked.
    pub superseded: Option<Superseded>,
}

/// Residual findings as the journal actually carries them, read by the
/// one derivation the engine admits a supersede by
/// ([`brokkr_core::residual::residuals`]) and rendered here. Each finding
/// names the run and the sequence number it came from, which is decision
/// 0007's provenance discipline applied to a readout instead of to a
/// seat input.
///
/// A finding an operator has superseded (decision 0047) is still
/// derived, still listed and still cited — it carries the annotation as
/// its `superseded` mark and leaves only Muninn's queue. **The
/// annotation is verified when it is WRITTEN and never re-verified
/// here**: `brokkr operator supersede` refuses a citation that does not
/// resolve, and the journal it cites is append-only, so what was true at
/// write time stays true. A superseding run concluded, stopped or ruled
/// again afterwards changes nothing about a mark already written
/// (ruling 2).
pub fn residual_findings(run_id: &str, events: &[EventEnvelope]) -> Vec<ResidualFinding> {
    let marks: BTreeMap<u64, Mark> = closings(events).into_iter().collect();
    residuals(events)
        .iter()
        .map(|residual| {
            let superseded = marks.get(&residual.ruling.seq).cloned();
            stated_finding(run_id, Stated::of(residual, superseded))
        })
        .collect()
}

/// A residual finding of run `run_id` as every surface shows it, from the
/// finding as its summary's `residuals` column states it (decision 0089
/// ruling 2) or as [`residual_findings`] reads it from the journal: the
/// one rendering of its line.
pub(crate) fn stated_finding(run_id: &str, stated: Stated) -> ResidualFinding {
    let Stated {
        seq,
        phase,
        rule_id,
        claim,
        superseded,
    } = stated;
    let (phase, input, value) = (phase.as_str(), claim.input(), claim.value());
    let rule_id = rule_id.unwrap_or_else(|| "?".to_string());
    let superseded = superseded.map(superseded_of);
    let mark = match &superseded {
        Some(mark) => format!(" · superseded by {}", cited(&mark.by)),
        None => String::new(),
    };
    ResidualFinding {
        run_id: run_id.to_string(),
        seq,
        line: format!("{run_id} seq {seq} · {phase} · {rule_id} · {input}: {value}{mark}"),
        phase: phase.to_string(),
        rule_id,
        input: input.to_string(),
        value: value.to_string(),
        superseded,
    }
}

/// The finding a fleet read raises for a run whose journal does not
/// fold. A quarantined run is not a run with nothing to say about it:
/// an unfoldable journal is the loudest thing in a fleet, so it travels
/// as a finding the operator's aide can propose about, cited by the
/// exact sequence number the fold refused at. The claim is the fleet
/// read's own, not the evaluator's, so the two rule fields the
/// evaluator would fill carry the absence mark rather than a borrowed
/// name.
pub fn quarantine_finding(run_id: &str, seq: u64, error: &str) -> ResidualFinding {
    ResidualFinding {
        run_id: run_id.to_string(),
        seq,
        phase: ABSENT.to_string(),
        rule_id: ABSENT.to_string(),
        input: "journal_folds".to_string(),
        value: "false".to_string(),
        line: format!("{run_id} seq {seq} · journal does not fold · {error}"),
        // A journal that does not fold cannot be read for annotations
        // either, and an operator cannot supersede a finding this crate
        // derived rather than the evaluator: `--findings` names a
        // ruling's seq, and this one names the seq the fold refused at.
        superseded: None,
    }
}

// ------------------------------------------------- the participant scan

/// Per-effect facts. Identity is the **request occurrence**, not the
/// `effect_id`: the console's `effects.set` replaces the object, and Σ
/// partitions on object identity, so keying on the id would merge what
/// the console splits.
struct EffectFacts {
    seat: String,
    phase: Option<String>,
    attempts: u64,
    open: Option<String>,
    terminal: Option<(&'static str, &'static str)>,
    terminal_index: Option<usize>,
    started_at: Option<String>,
}

struct Build {
    carried: Option<started::CarriedEntry>,
    effect_index: usize,
    key: String,
    label: String,
    member: Option<String>,
    /// The LAST attempt's provenance for this invocation site: what
    /// actually ran, which is the only thing a fallback is visible in.
    provenance: Option<Value>,
    /// Last provider-reported model for this invocation site.
    model: Option<String>,
    /// The boundary this site's last attempt journaled in its
    /// `effect/started.boundary` entry (decision 0046 ruling 3): the
    /// attempt's fact, last attempt winning as provenance does. `None`
    /// for an attempt no site of which declares hands, and for an entry
    /// the reader refuses (design DD14).
    boundary: Option<String>,
    /// The boundary a record of this site's attempt carried beside a
    /// `model` (design DD19) — the fallback when no entry was journaled,
    /// which is how a site without hands reads `not applicable` from the
    /// engine's own stamp and never from the manifest (design DD13).
    stamp: Option<String>,
    /// Last harness-echoed effort for this invocation site, on exactly
    /// the terms of the model beside it: last write wins, because both
    /// harnesses that echo one write it per turn.
    effort: Option<String>,
    turns: Option<u64>,
    /// Completed items of the seat's work, folded from the
    /// `item-completed` checkpoints a codex driver already journals
    /// inside its one turn; `None` while none has arrived.
    items: Option<u64>,
    last_turn: Option<Value>,
    session: Option<Value>,
    transcript: Option<Transcript>,
    member_outcome: Option<Value>,
    member_finished_at: Option<String>,
    checkpoints: Vec<(Value, String, Option<String>)>,
    /// What its records told, typed once the scan has read them all.
    told: Told,
    status: &'static str,
    status_class: &'static str,
}

struct Scan {
    effects: Vec<EffectFacts>,
    parts: Vec<Build>,
    by_key: BTreeMap<String, usize>,
    boundaries: Boundaries,
}

fn ensure(scan: &mut Scan, slot: usize, effect_id: &str, member: Option<&str>) -> usize {
    let key = participant::key_of(effect_id, member);
    if let Some(index) = scan.by_key.get(&key) {
        return *index;
    }
    let seat = &scan.effects[slot].seat;
    let label = match member {
        Some(member) => format!("{seat}:{member}"),
        None => seat.clone(),
    };
    scan.parts.push(Build {
        carried: None,
        effect_index: slot,
        key: key.clone(),
        label,
        member: member.map(str::to_string),
        provenance: None,
        model: None,
        boundary: None,
        stamp: None,
        effort: None,
        turns: None,
        items: None,
        last_turn: None,
        session: None,
        transcript: None,
        member_outcome: None,
        member_finished_at: None,
        checkpoints: Vec::new(),
        told: Told::default(),
        status: WORKING.0,
        status_class: WORKING.1,
    });
    let index = scan.parts.len() - 1;
    scan.by_key.insert(key, index);
    index
}

#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn scan_participants(events: &[EventEnvelope]) -> Scan {
    let mut scan = Scan {
        effects: Vec::new(),
        parts: Vec::new(),
        by_key: BTreeMap::new(),
        boundaries: boundaries_of(events),
    };
    let mut by_effect_id: BTreeMap<String, usize> = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        let payload = &event.payload;
        if event.event_type == EventType::EffectRequested {
            if let Some(effect_id) = field(payload, "effect_id") {
                scan.effects.push(EffectFacts {
                    seat: field(payload, "seat").unwrap_or("?").to_string(),
                    phase: field(payload, "phase").map(str::to_string),
                    attempts: 0,
                    open: None,
                    terminal: None,
                    terminal_index: None,
                    started_at: None,
                });
                let slot = scan.effects.len() - 1;
                by_effect_id.insert(effect_id.to_string(), slot);
                ensure(&mut scan, slot, effect_id, None);
                continue;
            }
        }
        let Some((effect_id, slot)) =
            field(payload, "effect_id").and_then(|id| by_effect_id.get(id).map(|slot| (id, *slot)))
        else {
            continue;
        };
        match event.event_type {
            EventType::EffectStarted => {
                {
                    let effect = &mut scan.effects[slot];
                    effect.attempts += 1;
                    effect.open = field(payload, "attempt_id").map(str::to_string);
                    if effect.started_at.is_none() {
                        effect.started_at = Some(event.recorded_at.clone());
                    }
                }
                started::sites(&mut scan, slot, effect_id, payload);
            }
            EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate => {
                // Matched by attempt id against the OPEN attempt: a
                // terminal for a stale attempt is ignored entirely —
                // it does not clear `open`, set the status, or
                // contribute a duration or a result token.
                if field(payload, "attempt_id") == scan.effects[slot].open.as_deref() {
                    if let Some(model) = payload.pointer("/result/model").and_then(Value::as_str) {
                        let part = ensure(&mut scan, slot, effect_id, None);
                        scan.parts[part].model = Some(model.to_string());
                    }
                    if let Some(word) = payload.get("result").and_then(stamp_of) {
                        let part = ensure(&mut scan, slot, effect_id, None);
                        scan.parts[part].stamp = Some(word);
                    }
                    if let Some(effort) = payload.pointer("/result/effort").and_then(Value::as_str)
                    {
                        let part = ensure(&mut scan, slot, effect_id, None);
                        scan.parts[part].effort = Some(effort.to_string());
                    }
                    let effect = &mut scan.effects[slot];
                    effect.open = None;
                    effect.terminal = terminal_status(event.event_type);
                    effect.terminal_index = Some(index);
                }
            }
            EventType::EffectCheckpointed => {
                let checkpoint = match payload.get("checkpoint") {
                    Some(checkpoint) => checkpoint.clone(),
                    None => Value::Null,
                };
                let member = checkpoint
                    .get("member")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let part = ensure(&mut scan, slot, effect_id, member.as_deref());
                if let Some(model) = checkpoint.get("model").and_then(Value::as_str) {
                    scan.parts[part].model = Some(model.to_string());
                }
                if field(payload, "attempt_id") == scan.effects[slot].open.as_deref() {
                    if let Some(word) = stamp_of(&checkpoint) {
                        scan.parts[part].stamp = Some(word);
                    }
                }
                if let Some(effort) = checkpoint.get("effort").and_then(Value::as_str) {
                    scan.parts[part].effort = Some(effort.to_string());
                }
                if let Some(transcript) = checkpoint.get("transcript").and_then(transcript_of) {
                    // ALWAYS replace: a retry's live transcript follows the
                    // concluded attempt's, just as the old session id did.
                    scan.parts[part].transcript = Some(transcript);
                }
                let boundary = row_boundary(event, &scan);
                scan.parts[part].checkpoints.push((
                    checkpoint.clone(),
                    event.recorded_at.clone(),
                    boundary,
                ));
                let step = checkpoint.get("step").and_then(Value::as_str);
                // A turn is a turn whoever counts it. Claude numbers a
                // turn per tool use and journals `seat-turn`; codex
                // closes each turn with `turn-completed` and journals no
                // `seat-turn` at all. Folding only the first left every
                // codex seat showing `—` turns while its turn numbers
                // sat in the journal, unread.
                if matches!(step, Some("seat-turn" | "turn-completed")) {
                    if let Some(turn) = checkpoint.get("turn").and_then(Value::as_u64) {
                        let current = scan.parts[part].turns;
                        scan.parts[part].turns = Some(match current {
                            Some(previous) => previous.max(turn),
                            None => turn,
                        });
                    }
                }
                count_item(&mut scan.parts[part].items, step);
                if step == Some("seat-turn") {
                    scan.parts[part].last_turn = Some(checkpoint);
                } else if step.is_some_and(|step| step.ends_with("-session-finished")) {
                    scan.parts[part].session = Some(checkpoint);
                } else if step == Some("session-started") {
                    // The id arrives at init so a WORKING seat's
                    // transcript is locatable and streamable. It
                    // ALWAYS replaces: on a retry, attempt two's
                    // started arrives after attempt one's finished,
                    // and the live session is the one to stream — a
                    // kept guard here would pin the drill to the dead
                    // attempt's transcript. Each finished checkpoint
                    // replaces in turn, bringing that attempt's cost;
                    // the seat's spend sums them all (`spent`).
                    scan.parts[part].session = Some(checkpoint);
                } else if step == Some("panel-member-finished") && member.is_some() {
                    scan.parts[part].member_outcome = checkpoint.get("outcome").cloned();
                    scan.parts[part].member_finished_at = Some(event.recorded_at.clone());
                }
            }
            _ => {}
        }
    }
    for part in &mut scan.parts {
        let effect = &scan.effects[part.effect_index];
        (part.status, part.status_class) = started::status_of(part, effect);
        part.told = Told::scanned(part);
    }
    scan
}

// -------------------------------------------------------------- boundary

/// One reported value, or the sorted, deduplicated join of several: how
/// a panel parent reports what its members reported, for every cell that
/// aggregates that way (the model, the effort, the boundary).
fn joined(mut reported: Vec<&str>) -> Option<String> {
    reported.sort_unstable();
    reported.dedup();
    match reported.as_slice() {
        [] => None,
        [only] => Some((*only).to_string()),
        _ => Some(reported.join(", ")),
    }
}

/// A site's own boundary: its last attempt's entry, else the stamp a
/// record of its attempt carried beside a model (design DD13).
fn own_boundary(part: &Build) -> Option<&str> {
    part.boundary.as_deref().or(part.stamp.as_deref())
}

/// A participant's boundary: its own, or — for a panel or sequence seat
/// that has none of its own — what its members reported, aggregated
/// exactly as its model cell is. A member never aggregates.
fn site_boundary(scan: &Scan, part: &Build) -> Option<String> {
    match own_boundary(part) {
        Some(word) => Some(word.to_string()),
        None if part.member.is_some() => None,
        None => joined(
            scan.parts
                .iter()
                .filter(|other| other.effect_index == part.effect_index && other.member.is_some())
                .filter_map(own_boundary)
                .collect(),
        ),
    }
}

/// Rows read their own attempt's entry before the record's stamp. A
/// request has no attempt yet and may display its participant's summary;
/// an attempt that recorded no entry never borrows a later attempt's word.
fn row_boundary(event: &EventEnvelope, scan: &Scan) -> Option<String> {
    let payload = &event.payload;
    if let Some(boundary) = attempt_boundary(&scan.boundaries, payload) {
        return boundary;
    }
    let effect_id = field(payload, "effect_id")?;
    let record = payload.get("checkpoint").or_else(|| payload.get("result"));
    let member = record.and_then(|record| field(record, "member"));
    let key = participant::key_of(effect_id, member);
    site_boundary(scan, &scan.parts[*scan.by_key.get(&key)?])
}

/// The run-level fact, derived once (decision 0046 ruling 3; design
/// DD12, DD13). `unboxed` is true exactly when a valid entry has `gate`
/// true and a word of `harness` or `open`. Whether the run declares hands
/// at all is read from the journaled `run/started` manifest — the one
/// boolean read there, beside the agents roster the notices already
/// read — and only for this fact: a run whose manifest declares hands
/// and whose journal holds no valid entry is *not recorded*; a run that
/// declares none renders nothing. `not applicable` entries are valid but
/// carry no word, so a run whose only entries are the sentinel reads the
/// same as one with none.
fn run_boundary(events: &[EventEnvelope]) -> RunBoundary {
    let declares_hands = events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted)
        .and_then(|event| event.payload.pointer("/manifest/hands"))
        .is_some();
    let mut words: Vec<&'static str> = Vec::new();
    let mut unboxed = false;
    for event in events {
        if event.event_type != EventType::EffectStarted {
            continue;
        }
        for entry in boundary_entries(&event.payload) {
            // Validated by `boundary_entries`, so the sentinel is the
            // only word that parses to no boundary.
            let Some(boundary) = Boundary::from_recorded(&entry.word).flatten() else {
                continue;
            };
            words.push(boundary.word());
            unboxed |= entry.gate && !boundary.is_boxed();
        }
    }
    let word = joined(words);
    let text = match (&word, declares_hands) {
        (Some(word), _) if unboxed => format!("{word} · unboxed"),
        (Some(word), _) => word.clone(),
        (None, true) => "not recorded".to_string(),
        (None, false) => String::new(),
    };
    RunBoundary {
        word: cell_of(word, declares_hands.then_some(NO_BOUNDARY)),
        unboxed,
        text,
    }
}

// -------------------------------------------------------------- activity

fn transcript_cell(transcript: Option<&Transcript>) -> Cell {
    match transcript {
        Some(transcript) if transcript.kind == "none" => cell_of(Some("none".to_string()), None),
        Some(transcript) => {
            let locator = if transcript.locator.is_empty() {
                ABSENT
            } else {
                &transcript.locator
            };
            let home = if transcript.home.is_empty() {
                ABSENT
            } else {
                &transcript.home
            };
            cell_of(
                Some(format!("{} · {} · home {}", transcript.kind, locator, home)),
                None,
            )
        }
        None => cell_of(None, Some("no transcript reference recorded")),
    }
}

fn target_cells(target: Option<&str>) -> (Option<String>, Option<String>) {
    match target {
        Some(target) => {
            let short = short_target(target);
            let full = if short == target {
                None
            } else {
                Some(target.to_string())
            };
            (Some(short), full)
        }
        None => (None, None),
    }
}

fn activity_for(
    events: &[EventEnvelope],
    scan: &Scan,
    part: &Build,
    member_count: usize,
) -> Activity {
    let effect = &scan.effects[part.effect_index];
    // A member's clock starts at its own first checkpoint — which is
    // also what created it, so there is always one — and the seat's at
    // the effect's first attempt.
    let start_at: Option<&str> = if part.member.is_some() {
        part.told.first_at.as_deref()
    } else {
        effect.started_at.as_deref()
    };
    // A member with no panel-member-finished (a sequence's final or exec
    // step) ends at its last checkpoint once the effect is done.
    let end_at: Option<&str> = if part.member.is_some() {
        match &part.told.finish {
            Some(finish) => Some(finish.at.as_str()),
            None => match part.told.last_at.as_deref() {
                Some(at) if effect.open.is_none() && effect.terminal.is_some() => Some(at),
                _ => None,
            },
        }
    } else {
        match effect.terminal_index {
            Some(index) => Some(&events[index].recorded_at),
            None => None,
        }
    };
    let mut bits: Vec<String> = Vec::new();
    if part.member.is_none() {
        if let Some(index) = effect.terminal_index {
            if let Some(token) = result_token(&events[index].payload) {
                bits.push(token.to_string());
            }
        }
    }
    if let (Some(from), Some(to)) = (start_at, end_at) {
        if let Some(duration) = fmt_dur(from, to) {
            bits.push(duration);
        }
    }
    let live = if part.status == WORKING.0 {
        part.told.turn.as_ref()
    } else {
        None
    };
    if let Some(turn) = live {
        let (short, full) = target_cells(turn.target.as_deref());
        let text = match &short {
            Some(short) => format!("{} · {short}", turn.tool),
            None => turn.tool.clone(),
        };
        return Activity {
            text,
            absent: false,
            note: None,
            tool: Some(turn.tool.clone()),
            target_short: short,
            target_full: full,
        };
    }
    if !bits.is_empty() {
        return Activity {
            text: bits.join(" · "),
            absent: false,
            note: None,
            tool: None,
            target_short: None,
            target_full: None,
        };
    }
    if member_count > 0 {
        return Activity {
            text: format!("{member_count} members ↓"),
            absent: true,
            note: None,
            tool: None,
            target_short: None,
            target_full: None,
        };
    }
    Activity {
        text: ABSENT.to_string(),
        absent: true,
        note: Some("no activity recorded".to_string()),
        tool: None,
        target_short: None,
        target_full: None,
    }
}

fn checkpoint_rows(part: &Build) -> Vec<CheckpointRow> {
    let rows = part.checkpoints.iter();
    let row = |(checkpoint, at, boundary): &(Value, String, Option<String>)| {
        row_of(checkpoint, at, boundary.clone())
    };
    rows.map(row).collect()
}

/// Members conclude on their own `panel-member-finished` outcome; the
/// seat itself on the effect's terminal event. A JSON-null outcome falls
/// through to the effect (the console's `memberOutcome !== null`), while
/// an absent one prints the literal `undefined` — a quirk, ported.
fn terminal_line(events: &[EventEnvelope], scan: &Scan, part: &Build) -> Cell {
    let effect = &scan.effects[part.effect_index];
    if part.member.is_some() {
        if let Some(Finish {
            at,
            outcome: Some(outcome),
        }) = &part.told.finish
        {
            return cell_of(
                Some(format!("panel-member-finished · {outcome} · {at}")),
                None,
            );
        }
    }
    match effect.terminal_index {
        Some(index) => cell_of(
            Some(format!(
                "{} · {}",
                events[index].event_type.as_str(),
                events[index].recorded_at
            )),
            None,
        ),
        None => Cell {
            text: "no terminal event yet".to_string(),
            absent: true,
            note: None,
        },
    }
}

/// A seat's own figure, or — when it has none — the Σ of its members'
/// in participant insertion order: `f64` addition is not associative
/// and the console sums in map order. The flag says which it is.
fn own_or_members(own: Option<f64>, members: &[Option<f64>]) -> (Option<f64>, bool) {
    let reported: Vec<f64> = members.iter().flatten().copied().collect();
    if own.is_some() || reported.is_empty() {
        return (own, false);
    }
    let mut total = 0.0;
    for value in &reported {
        total += value;
    }
    (Some(total), true)
}

fn usage_text(usage: &TokenUsage, prefix: &str) -> String {
    let mut fields = Vec::new();
    for (label, count) in [
        ("in", usage.input_tokens),
        ("out", usage.output_tokens),
        ("cache read", usage.cache_read_tokens),
        ("cache write", usage.cache_write_tokens),
        // Last, and outside the total above it, so the cell reads as
        // what it is: a part of `out`, shown rather than summed.
        ("reasoning", usage.reasoning_output_tokens),
    ] {
        if let Some(count) = count {
            fields.push(format!("{label} {}", fmt_tokens(count)));
        }
    }
    match usage.total_tokens {
        Some(total) => format!("{prefix}{} · {}", fmt_tokens(total), fields.join(" · ")),
        None => format!("{prefix}{}", fields.join(" · ")),
    }
}

/// A token count as a table cell: `842 tok`, `312k tok`, `3.99M tok`.
///
/// Three tiers on powers of a thousand, rounded the way every other
/// number this view humanizes is. The unit is spelled out because the
/// column this lands in otherwise holds dollars, and a bare `312k` next
/// to a `$0.0313` invites reading it as money — which is the one thing
/// this cell must never say.
fn fmt_tokens(total: u64) -> String {
    format!("{} tok", fmt_count(total))
}

/// The count alone, for a cell whose label already says tokens: `842`,
/// `312k`, `3.99M`.
fn fmt_count(total: u64) -> String {
    if total < 1_000 {
        return total.to_string();
    }
    let thousands = js::round_half_up(total as f64 / 1_000.0);
    if thousands < 1_000 {
        return format!("{thousands}k");
    }
    // Hundredths of a million, placed with integer arithmetic: rounding
    // once here keeps the printed digits from rounding a second time.
    let hundredths = js::round_half_up(total as f64 / 10_000.0);
    format!("{}.{:02}M", hundredths / 100, hundredths % 100)
}

#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn participants(events: &[EventEnvelope], scan: &Scan) -> Vec<Participant> {
    let mut out = Vec::new();
    let visits = participant::visits(events);
    for part in &scan.parts {
        // A panel/sequence seat carries no telemetry of its own — its
        // members do, and only the members of this request occurrence.
        let members: Vec<&Build> = if part.member.is_some() {
            Vec::new()
        } else {
            scan.parts
                .iter()
                .filter(|other| other.effect_index == part.effect_index && other.member.is_some())
                .collect()
        };
        let (cost, cost_aggregated) = own_or_members(
            part.told.spent,
            &members
                .iter()
                .map(|other| other.told.spent)
                .collect::<Vec<_>>(),
        );
        let (last_cost, _) = own_or_members(
            part.told.last_cost,
            &members
                .iter()
                .map(|other| other.told.last_cost)
                .collect::<Vec<_>>(),
        );
        let attempts = scan.effects[part.effect_index].attempts;
        let mut usage = part.told.usage.clone();
        let mut turns = part.turns;
        let mut usage_aggregated = false;
        let mut turns_aggregated = false;
        if !members.is_empty() {
            let member_usage: Vec<TokenUsage> = members
                .iter()
                .filter_map(|other| other.told.usage.clone())
                .collect();
            let member_turns: Vec<u64> = members.iter().filter_map(|other| other.turns).collect();
            if usage.is_none() && !member_usage.is_empty() {
                let mut total = TokenUsage::default();
                for member in &member_usage {
                    add_token(&mut total.input_tokens, member.input_tokens);
                    add_token(&mut total.output_tokens, member.output_tokens);
                    add_token(&mut total.cache_read_tokens, member.cache_read_tokens);
                    add_token(&mut total.cache_write_tokens, member.cache_write_tokens);
                    add_token(
                        &mut total.reasoning_output_tokens,
                        member.reasoning_output_tokens,
                    );
                }
                usage = finish_usage(total);
                usage_aggregated = true;
            }
            if turns.is_none() && !member_turns.is_empty() {
                turns = Some(member_turns.iter().sum());
                turns_aggregated = true;
            }
        }
        let turns_prefix = if turns_aggregated { "Σ " } else { "" };
        let cost_prefix = if cost_aggregated { "Σ " } else { "" };
        let usage_prefix = if usage_aggregated { "Σ " } else { "" };
        // What this seat spent, in the only unit its harness reported.
        // A price when there is one; otherwise the tokens the turns
        // counted; otherwise the absence mark, as before. Never a
        // conversion between the two, and never both in one cell: a
        // subscription harness reports no marginal price, and a null
        // price stays null rather than becoming an invented number.
        //
        // Dollars win when a Σ's members disagree in kind — the Σ is a
        // dollar total over the members that reported dollars, and the
        // token-only members show their own tokens on their own rows
        // right beneath it. Folding tokens into a dollar figure, or
        // printing both in one cell, would be the unit mixing this
        // whole cell exists to refuse.
        //
        // A retried seat's dollars are every attempt's, and the cell says
        // so rather than leaving the sum to be read as one session's.
        let tokens = usage.as_ref().and_then(|usage| usage.total_tokens);
        let cost_text = match (cost, tokens) {
            (Some(cost), _) if attempts > 1 => Some(format!(
                "{cost_prefix}${} over {attempts} attempts",
                js::to_fixed_4(cost)
            )),
            (Some(cost), _) => Some(format!("{cost_prefix}${}", js::to_fixed_4(cost))),
            (None, Some(tokens)) => Some(format!("{usage_prefix}{}", fmt_tokens(tokens))),
            (None, None) => None,
        };
        // A member never aggregates: `members` is empty for one, and the
        // join of nothing is the absence mark.
        let model = match &part.model {
            Some(model) => Some(model.clone()),
            None => joined(
                members
                    .iter()
                    .filter_map(|member| member.model.as_deref())
                    .collect(),
            ),
        };
        // The applied effort, aggregated across a panel's members
        // exactly as the served model above it is: a panel whose members
        // ran at different levels says both rather than picking one.
        let effort = match &part.effort {
            Some(effort) => Some(effort.clone()),
            None => joined(
                members
                    .iter()
                    .filter_map(|member| member.effort.as_deref())
                    .collect(),
            ),
        };
        let provenance = part.provenance.as_ref().map(provenance_of);
        let transcript = part.transcript.clone();
        let session_id = transcript
            .as_ref()
            .filter(|transcript| transcript.kind == "claude-session")
            .map(|transcript| transcript.locator.clone())
            .filter(|locator| !locator.is_empty())
            .or_else(|| part.told.session_id.clone());
        let (usage_cell, tokens) = usage_cells(usage.as_ref(), usage_prefix);
        let carried_from = started::carried_from(events, scan, part);
        let activity = started::activity(events, scan, part, members.len(), carried_from.as_ref());
        out.push(Participant {
            carried_from,
            key: part.key.clone(),
            label: part.label.clone(),
            member: part.member.clone(),
            phase: scan.effects[part.effect_index].phase.clone(),
            visit: visits.get(part.effect_index).copied().flatten(),
            status: part.status.to_string(),
            status_class: part.status_class.to_string(),
            working: part.status == WORKING.0,
            attempts,
            turns,
            turns_aggregated,
            turns_cell: cell_of(
                items::turns_text(turns, part.items, turns_prefix),
                Some("no turn telemetry recorded"),
            ),
            cost,
            last_attempt_cost: last_cost,
            cost_aggregated,
            cost_cell: cell_of(cost_text, Some("no session cost or token usage recorded")),
            usage_cell,
            tokens,
            usage,
            usage_aggregated,
            served: ModelAtBoundary {
                model: cell_of(model, Some("no served model recorded")),
                boundary: cell_of(site_boundary(scan, part), Some(NO_BOUNDARY)),
            },
            // Two cells, never one filled from the other (decision 0035
            // ruling 6). A journal written before this decision carries
            // neither, and each says so in its own words rather than the
            // applied cell quietly showing the plan's pin.
            effort: cell_of(
                effort,
                Some("no applied effort recorded — configuration, not a measurement"),
            ),
            effort_pin: cell_of(
                provenance
                    .as_ref()
                    .and_then(|provenance| provenance.effort.clone()),
                Some("no effort pinned by this run's resolution"),
            ),
            activity,
            member_count: members.len(),
            transcript_cell: transcript_cell(transcript.as_ref()),
            transcript,
            session_id,
            terminal_line: terminal_line(events, scan, part),
            checkpoints: checkpoint_rows(part),
            provenance,
        });
    }
    out
}

/// The one place a provenance record becomes words. Every surface prints
/// `line`; none composes its own, which is what stops a readout from
/// quietly dropping the "not the first choice" half of the sentence.
fn provenance_of(entry: &Value) -> Provenance {
    let text = |key: &str| field(entry, key).unwrap_or("?").to_string();
    let chain_index = entry
        .get("chain_index")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let (agent, model, provider) = (text("agent"), text("model"), text("provider"));
    let line = match chain_index {
        0 => format!("{agent} · selected {model} via {provider}"),
        // The chain is a fallback chain, not a portability claim.
        _ => format!(
            "{agent} · selected {model} via {provider} (fallback #{chain_index}; \
             not the agent's first choice)"
        ),
    };
    Provenance {
        agent,
        model,
        // Read, never defaulted: an absent pin is a pre-0035 journal or
        // an effortless provider, and `None` is the honest answer to
        // both. The sentence above stays about the model and its
        // fallback — ruling 6 asks for a separate cell, not a reworded
        // line that an old journal would have to be excused from.
        effort: field(entry, "effort").map(str::to_string),
        provider,
        chain_index,
        fallback: chain_index > 0,
        line,
    }
}

/// Run-level notices, from two journaled sources and no third: the
/// compile-time record already carried inside `run/started`'s manifest,
/// and the fallbacks that actually happened. Deduplicated by text, in
/// first-seen order.
fn run_notices(events: &[EventEnvelope]) -> Vec<RunNotice> {
    let mut out: Vec<RunNotice> = Vec::new();
    let mut push = |kind: &str, text: String| {
        let notice = RunNotice {
            kind: kind.to_string(),
            text,
        };
        if !out.contains(&notice) {
            out.push(notice);
        }
    };
    let agents = events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted)
        .and_then(|event| event.payload.pointer("/manifest/agents"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for (site, record) in &agents {
        let notices = record
            .get("notices")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for notice in notices {
            let message = field(&notice, "message").unwrap_or("capability gap");
            push("capability-gap", format!("{site}: {message}"));
        }
    }
    for event in events {
        if event.event_type != EventType::EffectStarted {
            continue;
        }
        let entries = event
            .payload
            .get("provenance")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for entry in entries {
            let derived = provenance_of(&entry);
            if derived.fallback {
                let site = field(&entry, "member").unwrap_or("seat").to_string();
                push("fallback", format!("{site}: {}", derived.line));
            }
        }
    }
    out
}

// ------------------------------------------------------------ live lines

/// The live block is a **second scan**, not the activity column: `open`
/// tracks the last `effect/started` globally, labels come from
/// `p.seat` truthiness defaulting `?`, and one bare seat row is
/// synthesized when the scan found nothing. Folding it into the
/// participant model would delete an observable surface.
fn live_lines(events: &[EventEnvelope]) -> Vec<LiveLine> {
    let mut open: Option<(Option<String>, Option<String>)> = None;
    for event in events {
        let payload = &event.payload;
        if event.event_type == EventType::EffectStarted {
            open = Some((
                field(payload, "effect_id").map(str::to_string),
                field(payload, "attempt_id").map(str::to_string),
            ));
        } else if let Some((effect_id, attempt_id)) = &open {
            if terminal_status(event.event_type).is_some()
                && field(payload, "effect_id") == effect_id.as_deref()
                && field(payload, "attempt_id") == attempt_id.as_deref()
            {
                open = None;
            }
        }
    }
    let Some((open_effect, open_attempt)) = open else {
        return Vec::new();
    };
    let mut seat = "?".to_string();
    let mut rows: Vec<(String, Option<Value>)> = Vec::new();
    for event in events {
        let payload = &event.payload;
        if event.event_type == EventType::EffectRequested
            && field(payload, "effect_id") == open_effect.as_deref()
        {
            if let Some(named) = field(payload, "seat").filter(|seat| !seat.is_empty()) {
                seat = named.to_string();
            }
        }
        if event.event_type == EventType::EffectCheckpointed
            && field(payload, "effect_id") == open_effect.as_deref()
            && field(payload, "attempt_id") == open_attempt.as_deref()
        {
            let checkpoint = match payload.get("checkpoint") {
                Some(checkpoint) => checkpoint.clone(),
                None => Value::Null,
            };
            let label = match checkpoint.get("member").and_then(Value::as_str) {
                Some(member) => format!("{seat}:{member}"),
                None => seat.clone(),
            };
            let existing = rows.iter().position(|(name, _)| *name == label);
            if checkpoint.get("step").and_then(Value::as_str) == Some("seat-turn") {
                match existing {
                    Some(index) => rows[index].1 = Some(checkpoint),
                    None => rows.push((label, Some(checkpoint))),
                }
            } else if !matches!(existing, Some(index) if rows[index].1.is_some()) {
                match existing {
                    Some(index) => rows[index].1 = None,
                    None => rows.push((label, None)),
                }
            }
        }
    }
    if rows.is_empty() {
        rows.push((seat, None));
    }
    let shown = |checkpoint: Value| {
        let shown = |key: &str| js::to_display(checkpoint.get(key));
        (shown("turn"), shown("tool"))
    };
    rows.into_iter()
        .map(|(label, checkpoint)| live_line(label, checkpoint.map(shown)))
        .collect()
}

/// A live line as the trail paints it: `label`, and its last `seat-turn`'s
/// turn and tool as shown, or `working` while it has none.
fn live_line(label: String, turn: Option<(String, String)>) -> LiveLine {
    let text = match turn {
        Some((turn, tool)) => format!("{label} · turn {turn} · {tool}"),
        None => format!("{label} · working"),
    };
    LiveLine { label, text }
}

// ------------------------------------------------------- inner topology

/// Events bucketed by effect once, so `watch` does not pay an
/// O(phases × events) rescan on every redraw.
struct Buckets<'a> {
    /// Phase name -> the NEWEST effect requested in it, and its seat
    /// (last write wins). The seat travels with the id because it is the
    /// label a structureless phase draws, and reading it here keeps that
    /// lookup total — a participant always exists for a requested effect,
    /// so a "no participant" fallback would be an uncoverable branch.
    newest: BTreeMap<&'a str, (&'a str, &'a str)>,
    /// Effect id -> the sequence steps its checkpoints finished and the
    /// member tags they carry, each once, in first order.
    listed: BTreeMap<&'a str, (Vec<String>, Vec<String>)>,
    /// Effect id -> every phase it was requested in (scope membership).
    effect_phases: BTreeMap<&'a str, Vec<String>>,
}

/// A checkpoint's `payload` as its effect's lists read it: a finished
/// step's name, or else the member it tags, each listed once.
fn listed((finished, tags): &mut (Vec<String>, Vec<String>), payload: &Value) {
    let checkpoint = payload.get("checkpoint").unwrap_or(&Value::Null);
    let (list, name) = match field(checkpoint, "step") {
        Some("sequence-step-finished") => (finished, field(checkpoint, "step_name")),
        _ => (tags, field(checkpoint, "member")),
    };
    if let Some(name) = name.filter(|name| !list.iter().any(|known| known == name)) {
        list.push(name.to_string());
    }
}

fn bucket(events: &[EventEnvelope]) -> Buckets<'_> {
    let mut buckets = Buckets {
        newest: BTreeMap::new(),
        listed: BTreeMap::new(),
        effect_phases: BTreeMap::new(),
    };
    for event in events {
        let payload = &event.payload;
        match event.event_type {
            EventType::EffectRequested => {
                if let (Some(effect_id), Some(phase)) =
                    (field(payload, "effect_id"), field(payload, "phase"))
                {
                    let seat = field(payload, "seat").unwrap_or("?");
                    buckets.newest.insert(phase, (effect_id, seat));
                    let names = buckets.effect_phases.entry(effect_id).or_default();
                    if !names.iter().any(|name| name == phase) {
                        names.push(phase.to_string());
                    }
                }
            }
            EventType::EffectCheckpointed => {
                if let Some(effect_id) = field(payload, "effect_id") {
                    listed(buckets.listed.entry(effect_id).or_default(), payload);
                }
            }
            _ => {}
        }
    }
    buckets
}

/// A participant's own `panel-member-finished` outcome rules first; a
/// finished step is phosphor even though its participant inherits the
/// effect's terminal status; otherwise the effect's terminal decides,
/// else the node is active — checkpoints arriving with no terminal yet.
fn node_state(part: Option<&Build>, step_done: bool) -> (&'static str, &'static str) {
    let outcome = part.and_then(|part| part.told.word.as_deref());
    if let Some(state) = outcome.and_then(outcome_state) {
        return state;
    }
    if step_done {
        return NODE_FINISHED;
    }
    match part.and_then(|part| outcome_state(part.status)) {
        Some(state) => state,
        None => NODE_ACTIVE,
    }
}

fn make_node(scan: &Scan, effect_id: &str, label: &str, tag: Option<&str>, done: bool) -> Node {
    let key = participant::key_of(effect_id, tag);
    let part = scan.by_key.get(&key).map(|index| &scan.parts[*index]);
    let (state, state_class) = node_state(part, done);
    Node {
        label: label.to_string(),
        key,
        state: state.to_string(),
        state_class: state_class.to_string(),
        served: ModelAtBoundary {
            model: cell_of(
                part.and_then(|part| part.model.clone()),
                Some("no served model recorded"),
            ),
            // A node's boundary is its participant's (decision 0046
            // ruling 3).
            boundary: cell_of(
                part.and_then(|part| site_boundary(scan, part)),
                Some(NO_BOUNDARY),
            ),
        },
    }
}

/// Inner topology for one phase, from its NEWEST observed effect. Only
/// observed events count: declared-but-unstarted topology is never
/// invented. An empty result means the phase has no observed effect.
fn inner_columns(scan: &Scan, buckets: &Buckets, phase: &str) -> Vec<Column> {
    let Some((effect_id, seat)) = buckets.newest.get(phase).copied() else {
        return Vec::new();
    };
    let unlisted = (Vec::new(), Vec::new());
    let (finished, tags) = buckets.listed.get(effect_id).unwrap_or(&unlisted);
    let mut columns: Vec<Column> = Vec::new();
    if !finished.is_empty() {
        let mut steps: Vec<String> = finished.clone();
        let mut step_tags: Vec<(String, Vec<String>)> = Vec::new();
        for tag in tags {
            let (step, tagged) = match tag.find(':') {
                Some(cut) => (tag[..cut].to_string(), true),
                None => (tag.clone(), false),
            };
            if !steps.contains(&step) {
                steps.push(step.clone());
            }
            if tagged {
                match step_tags.iter_mut().find(|(name, _)| *name == step) {
                    Some((_, list)) => list.push(tag.clone()),
                    None => step_tags.push((step, vec![tag.clone()])),
                }
            }
        }
        for step in &steps {
            let done = finished.iter().any(|name| name == step);
            match step_tags.iter().find(|(name, _)| name == step) {
                Some((_, list)) => columns.push(Column {
                    label: Some(step.clone()),
                    nodes: list
                        .iter()
                        .map(|tag| {
                            make_node(scan, effect_id, &tag[step.len() + 1..], Some(tag), done)
                        })
                        .collect(),
                }),
                None => columns.push(Column {
                    label: None,
                    nodes: vec![make_node(scan, effect_id, step, Some(step), done)],
                }),
            }
        }
    } else if !tags.is_empty() {
        columns.push(Column {
            label: None,
            nodes: tags
                .iter()
                .map(|tag| make_node(scan, effect_id, tag, Some(tag), false))
                .collect(),
        });
    } else {
        columns.push(Column {
            label: None,
            nodes: vec![make_node(scan, effect_id, seat, None, false)],
        });
    }
    columns
}

// ----------------------------------------------------- trail and journal

fn what_of(
    scan: &Scan,
    event: &EventEnvelope,
    marks: &BTreeMap<u64, Superseded>,
    commands: &BTreeMap<String, (String, u64)>,
) -> What {
    let payload = &event.payload;
    match event.event_type {
        EventType::TransitionDecided => {
            let rule = display_or_mark(payload.get("rule_id"));
            let result = match field(payload, "result") {
                Some(result) => format!(" · {result}"),
                None => String::new(),
            };
            // The ruling line every trail prints — `inspect`'s and the
            // console's alike — carries the mark when a finding read
            // from this ruling is superseded (decision 0047 ruling 3).
            let mark = match marks.get(&event.seq) {
                Some(mark) => format!(" · superseded by {}", cited(&mark.by)),
                None => String::new(),
            };
            let arrow = format!(
                " {} → {}{result}{mark}",
                display_or_mark(payload.get("from")),
                display_or_mark(payload.get("next"))
            );
            What {
                text: format!("{rule}{arrow}"),
                badge: Some(rule),
                badge_class: Some(severity_class(payload).to_string()),
                arrow: Some(arrow),
                problem: match payload.get("problem") {
                    Some(problem) if truthy(Some(problem)) => Some(js::to_display(Some(problem))),
                    _ => None,
                },
            }
        }
        EventType::PhaseEntered => {
            let name = display_or_mark(payload.get("phase"));
            What {
                text: name.clone(),
                badge: Some(name),
                badge_class: None,
                arrow: None,
                problem: None,
            }
        }
        EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate => {
            let effect_id = field(payload, "effect_id");
            let label = match effect_id.and_then(|id| scan.by_key.get(id)) {
                Some(index) => scan.parts[*index].label.clone(),
                None => effect_id.unwrap_or("?").to_string(),
            };
            let token = match result_token(payload) {
                Some(token) => format!(" · {token}"),
                None => String::new(),
            };
            What {
                text: format!("{label}{token}"),
                badge: None,
                badge_class: None,
                arrow: None,
                problem: field(payload, "error")
                    .filter(|error| !error.is_empty())
                    .map(|error| js::slice(error, 0, 160)),
            }
        }
        EventType::OperatorCommanded
        | EventType::OperatorAccepted
        | EventType::OperatorRejected => operator_what(event, commands),
        EventType::RunStarted => {
            let text = match field(payload, "feature") {
                // The ellipsis is UNCONDITIONAL: a five-character
                // feature renders `hello…` on the console today, and
                // this is a port, not a correction.
                Some(feature) => format!("{}…", js::slice(feature, 0, 110)),
                None => "run started".to_string(),
            };
            What {
                text,
                badge: None,
                badge_class: None,
                arrow: None,
                problem: None,
            }
        }
        other => {
            let token = ["rule_id", "phase", "reason"]
                .iter()
                .find_map(|key| field(payload, key).filter(|value| !value.is_empty()));
            let text = match token {
                Some(token) => token.to_string(),
                None => type_tail(other.as_str()).to_string(),
            };
            What {
                text,
                badge: None,
                badge_class: None,
                arrow: None,
                problem: None,
            }
        }
    }
}

fn display_or_mark(value: Option<&Value>) -> String {
    match value {
        Some(Value::Null) | None => "?".to_string(),
        Some(value) => js::to_display(Some(value)),
    }
}

fn label_of(event: &EventEnvelope) -> Cell {
    let payload = &event.payload;
    if event.event_type == EventType::EffectCheckpointed {
        let checkpoint = match payload.get("checkpoint") {
            Some(checkpoint) => checkpoint,
            None => &Value::Null,
        };
        let mut label = match checkpoint.get("step").and_then(Value::as_str) {
            Some("seat-turn") => match checkpoint.get("tool").and_then(Value::as_str) {
                Some(tool) => tool.to_string(),
                None => "seat-turn".to_string(),
            },
            _ => match checkpoint.get("step").and_then(Value::as_str) {
                Some(step) => step.to_string(),
                None => String::new(),
            },
        };
        if let Some(member) = checkpoint.get("member").and_then(Value::as_str) {
            label = format!("{member} · {label}");
        }
        return cell_of(Some(label).filter(|label| !label.is_empty()), None);
    }
    let token = ["rule_id", "phase"]
        .iter()
        .find_map(|key| field(payload, key).filter(|value| !value.is_empty()))
        .or_else(|| result_token(payload).filter(|value| !value.is_empty()))
        .or_else(|| {
            ["command", "reason"]
                .iter()
                .find_map(|key| field(payload, key).filter(|value| !value.is_empty()))
        });
    cell_of(token.map(str::to_string), None)
}

/// Each event's seq by its id, the last where an id repeats: what names a
/// row's cause. `verify_chain` pins `seq == i + 1`, so seq 0 is
/// unrepresentable in a loaded journal and the console's truthiness check
/// on the looked-up seq has nothing to guard. A total map is the whole rule.
fn ids(events: &[EventEnvelope]) -> BTreeMap<&str, u64> {
    let mut by_id: BTreeMap<&str, u64> = BTreeMap::new();
    for event in events {
        by_id.insert(event.event_id.as_str(), event.seq);
    }
    by_id
}

/// The journal's rows for `events`, each caused by the seq `cause` names
/// for its causation id.
#[expect(clippy::excessive_nesting, reason = "baseline 2026-09, #288")]
fn journal_rows(
    events: &[EventEnvelope],
    scan: &Scan,
    buckets: &Buckets,
    marks: &BTreeMap<u64, Superseded>,
    commands: &BTreeMap<String, (String, u64)>,
    cause: impl Fn(&str) -> Option<u64>,
) -> Vec<JournalRow> {
    events
        .iter()
        .map(|event| {
            let payload = &event.payload;
            let recorded_model = payload
                .pointer("/checkpoint/model")
                .or_else(|| payload.pointer("/result/model"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    terminal_status(event.event_type)?;
                    let effect_id = field(payload, "effect_id")?;
                    let part = scan.by_key.get(effect_id)?;
                    if let Some(model) = &scan.parts[*part].model {
                        return Some(model.clone());
                    }
                    let effect_index = scan.parts[*part].effect_index;
                    joined(
                        scan.parts
                            .iter()
                            .filter(|candidate| candidate.effect_index == effect_index)
                            .filter_map(|candidate| candidate.model.as_deref())
                            .collect(),
                    )
                });
            let recorded_boundary = row_boundary(event, scan);
            let mut phases: Vec<String> = Vec::new();
            for key in ["phase", "from"] {
                if let Some(name) = field(payload, key) {
                    if !phases.iter().any(|known| known == name) {
                        phases.push(name.to_string());
                    }
                }
            }
            if let Some(effect_id) = field(payload, "effect_id") {
                if let Some(names) = buckets.effect_phases.get(effect_id) {
                    for name in names {
                        if !phases.iter().any(|known| known == name) {
                            phases.push(name.clone());
                        }
                    }
                }
            }
            JournalRow {
                seq: event.seq,
                causation_seq: event.causation_id.as_deref().and_then(&cause),
                event_type: event.event_type.as_str().to_string(),
                recorded_at: event.recorded_at.clone(),
                in_trail: !TRAIL_SKIP.contains(&event.event_type),
                phases,
                what: what_of(scan, event, marks, commands),
                label: label_of(event),
                served: ModelAtBoundary {
                    model: cell_of(recorded_model, Some("no served model recorded")),
                    boundary: cell_of(recorded_boundary, Some(NO_BOUNDARY)),
                },
                payload_json: match payload {
                    Value::Object(_) => payload.to_string(),
                    _ => "{}".to_string(),
                },
            }
        })
        .collect()
}

// ------------------------------------------------------ summary + ruling

/// The last decision read as a ruling, when it records a rule at all
/// (`null` included): the one test both the run view and the fleet read.
pub(crate) fn ruled(decision: Option<&Decided>) -> Option<&Decided> {
    decision.filter(|decision| !decision.rule_id.is_absent())
}

/// A recorded word as the console prints it, `?` where none is recorded.
fn word_or_mark(word: Option<&String>) -> String {
    word.map_or_else(|| "?".to_string(), String::clone)
}

/// A recorded word the console prints only when it is not empty.
fn nonempty(word: Option<&String>) -> Option<String> {
    word.filter(|word| !word.is_empty()).cloned()
}

fn ruling_of(decision: Option<&Decided>, superseded: Option<Superseded>) -> Option<Ruling> {
    let decision = ruled(decision)?;
    let inputs = match decision.inputs.present() {
        Some(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.to_string()))
            .collect(),
        None => Vec::new(),
    };
    Some(Ruling {
        rule_id: word_or_mark(decision.rule_id.present()),
        severity_class: class_of_severity(decision.severity.present().copied()).to_string(),
        from: decision.from.clone(),
        next: word_or_mark(decision.next.present()),
        result: nonempty(Some(&decision.result)),
        inputs,
        problem: nonempty(decision.problem.present()),
        superseded,
    })
}

// ------------------------------------------------------------ entrypoint

/// The whole readout for one run: one derivation, every surface.
pub fn run_view(events: &[EventEnvelope], state: Option<&RunState>) -> RunView {
    let scan = scan_participants(events);
    let buckets = bucket(events);
    let marks = supersedes(events);
    let commands = commands_of(events);
    let by_id = ids(events);
    let cause = |id: &str| by_id.get(id).copied();
    let read = Read {
        live: live_lines(events),
        journal: journal_rows(events, &scan, &buckets, &marks, &commands, cause),
        event_count: events.len(),
        dashboard: dashboard(events),
    };
    viewed(events, &scan, &buckets, &marks, read, state)
}

/// What a run view reads beside its trail: the live block, the journal's
/// rows, the event count and the dashboard, read whole or by a page.
pub(crate) struct Read {
    pub(crate) live: Vec<LiveLine>,
    pub(crate) journal: Vec<JournalRow>,
    pub(crate) event_count: usize,
    pub(crate) dashboard: Dashboard,
}

/// The run view of the run whose trail `events` holds: its participants
/// as `scan` read them, its effects as `buckets` holds them, its
/// supersedes as `marks`, and the rest as `read`.
fn viewed(
    events: &[EventEnvelope],
    scan: &Scan,
    buckets: &Buckets,
    marks: &BTreeMap<u64, Superseded>,
    read: Read,
    state: Option<&RunState>,
) -> RunView {
    // The ruling the summary carries is the LAST one ruled, so that is
    // the seq an annotation must name for the header line to be marked
    // (decision 0047 ruling 3). Every other ruling's mark reaches the
    // reader through the trail row it was ruled on.
    let last_ruled = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::TransitionDecided)
        .and_then(|event| marks.get(&event.seq))
        .cloned();
    let clarify = state.and_then(|state| clarify(events, state));
    RunView {
        view_version: VIEW_VERSION,
        summary: state.map(summary),
        ruling: ruling_of(
            state.and_then(|state| state.last_decision.as_ref()),
            last_ruled,
        ),
        participants: participants(events, scan),
        live: read.live,
        phases: rail::phase_rail(events, scan, buckets, state),
        notices: run_notices(events),
        boundary: run_boundary(events),
        journal: read.journal,
        event_count: read.event_count,
        dashboard: read.dashboard,
        scopes: None,
        commands: state.map_or_else(Vec::new, commands),
        unavailable: state.map_or_else(Vec::new, |state| {
            let residuals = residual_findings(&state.run_id, events);
            unavailable(state, &residuals, clarify.as_ref())
        }),
        clarify,
        worktree: worktree(events),
        recipe: brokkr_core::summary::aggregate(events, None).recipe,
        entry: entry::of_events(events),
    }
}

#[cfg(test)]
mod tests;
