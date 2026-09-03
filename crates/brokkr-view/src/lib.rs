//! `brokkr-view` — one derivation, two surfaces (decision 0013).
//!
//! Pure functions over a run's journal (plus the folded `RunState`)
//! producing serializable view models: run rows, the run summary and its
//! ruling, participants, live seat lines, the phase rail with each
//! phase's inner topology, and the decision trail. There is no I/O, no
//! rendering, and no terminal or DOM concept here — the manifest depends
//! on exactly `brokkr-core`, `serde` and `serde_json`, so that property is
//! a compile error rather than a review convention, and the absence of a
//! clock is what forces `now` to be a parameter.
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

pub mod js;

use std::collections::{BTreeMap, HashMap};

use brokkr_core::fold::{RunState, Status};
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
pub const VIEW_VERSION: u32 = 6;

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
#[derive(Serialize)]
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
    pub state: Option<&'a RunState>,
    /// Why the state is absent: a fleet read quarantines a run whose
    /// journal does not fold rather than losing the whole fleet with it,
    /// and the error text is the row's whole account of itself. Nothing
    /// is repaired here (README law 2) — the refusal is reported.
    pub detail: Option<&'a str>,
}

#[derive(Serialize)]
pub struct RunRow {
    pub run_id: String,
    pub status: Option<String>,
    /// The status is one of the four the surfaces have a colour for.
    pub status_known: bool,
    pub phase: Option<String>,
    pub seq: Option<u64>,
    pub created_at: String,
    /// The **full** feature: the model stays terminal-agnostic and
    /// `--json` stays lossless. Clamping is the renderer's job.
    pub feature: String,
    /// Why this row carries no status, when it carries none: the fold
    /// error, verbatim. A quarantined run reads as `?` plus this line
    /// on every surface instead of vanishing from the fleet.
    pub detail: Option<String>,
}

#[derive(Serialize)]
pub struct RunsView {
    pub view_version: u32,
    pub runs: Vec<RunRow>,
    pub count: usize,
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

/// The nine `summarize()` keys, verbatim and in the same order the
/// existing `brokkr inspect` prints them.
#[derive(Serialize)]
pub struct Summary {
    pub consecutive_failures: BTreeMap<String, u64>,
    pub cursor: String,
    pub feature: Option<String>,
    pub last_decision: Option<Value>,
    pub park_reason: Option<String>,
    pub phase: Option<String>,
    pub run_id: String,
    pub seq: u64,
    pub status: String,
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

#[derive(Serialize)]
pub struct CheckpointRow {
    pub turn: Cell,
    pub step: String,
    pub model: Cell,
    pub usage: Cell,
    pub target: Cell,
    pub target_full: Option<String>,
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
    pub provider: String,
    pub chain_index: u64,
    /// Brokkr never claims the second choice equals the first.
    pub fallback: bool,
    pub line: String,
}

/// The common driver transcript reference (decision 0032). It contains
/// paths or ids only; transcript prose never enters the journal or view.
#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct Transcript {
    pub kind: String,
    pub locator: String,
    pub home: String,
}

/// Harness-reported token accounting under the seat-record vocabulary.
/// `total_tokens` is exactly input + output. Cache reads are already a
/// subset of input and cache writes stay separately visible, so neither
/// is silently added a second time.
#[derive(Serialize, Clone, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
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
pub struct Participant {
    pub key: String,
    pub label: String,
    pub member: Option<String>,
    /// Precomputed scope membership: no surface implements the predicate.
    pub phase: Option<String>,
    pub status: String,
    pub status_class: String,
    pub attempts: u64,
    pub turns: Option<u64>,
    pub turns_aggregated: bool,
    pub turns_cell: Cell,
    pub cost: Option<f64>,
    pub cost_aggregated: bool,
    pub cost_cell: Cell,
    pub usage: Option<TokenUsage>,
    pub usage_aggregated: bool,
    pub usage_cell: Cell,
    /// The provider-reported model. No adapter default or abstract agent
    /// choice is substituted when the journal carries no such report.
    pub model: Cell,
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
    pub model: Cell,
}

#[derive(Serialize)]
pub struct Column {
    pub label: Option<String>,
    pub nodes: Vec<Node>,
}

#[derive(Serialize)]
pub struct Phase {
    pub name: String,
    pub visits: u64,
    pub current: bool,
    /// No inner structure was observed: the rail draws a single node.
    pub plain: bool,
    pub columns: Vec<Column>,
    /// The phases whose ruling sent the run BACK here — the road in,
    /// not the count of arrivals. One name per distinct backward
    /// transition the journal recorded, in the order it recorded them,
    /// so a surface that draws the return draws it once however many
    /// times it was taken (decision 0022's reforging is the machine's
    /// only backward edge today; nothing here names it).
    ///
    /// Derived from the `transition/decided` that CAUSED the revisit,
    /// never inferred from `visits > 1`: the count says a phase was
    /// entered twice, and only the transition says where from.
    pub returns: Vec<String>,
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
    pub model: Cell,
    pub payload_json: String,
}

#[derive(Serialize)]
pub struct RunView {
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
    pub journal: Vec<JournalRow>,
    /// Every event, unfiltered: the console's `full journal · N events`.
    pub event_count: usize,
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

pub fn status_str(status: &Status) -> &'static str {
    match status {
        Status::Running => "running",
        Status::AwaitingOperator => "awaiting_operator",
        Status::Completed => "completed",
        Status::Stopped => "stopped",
    }
}

fn type_str(event_type: EventType) -> &'static str {
    match event_type {
        EventType::RunStarted => "run/started",
        EventType::PhaseEntered => "phase/entered",
        EventType::EffectRequested => "effect/requested",
        EventType::EffectStarted => "effect/started",
        EventType::EffectCheckpointed => "effect/checkpointed",
        EventType::EffectSucceeded => "effect/succeeded",
        EventType::EffectFailed => "effect/failed",
        EventType::EffectIndeterminate => "effect/indeterminate",
        EventType::TransitionDecided => "transition/decided",
        EventType::OperatorCommanded => "operator/commanded",
        EventType::OperatorAccepted => "operator/accepted",
        EventType::OperatorRejected => "operator/rejected",
        EventType::RunParked => "run/parked",
        EventType::RunCompleted => "run/completed",
        EventType::RunStopped => "run/stopped",
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
        EventType::EffectSucceeded => Some(("succeeded", "completed")),
        EventType::EffectFailed => Some(("failed", "stopped")),
        EventType::EffectIndeterminate => Some(("indeterminate", "awaiting_operator")),
        _ => None,
    }
}

const WORKING: (&str, &str) = ("working", "running");

/// The allowlist a panel member's own outcome must pass to override the
/// effect's terminal status. `working` is in it, so a member really can
/// be pinned to `working` after the effect concluded.
fn outcome_status(outcome: &str) -> Option<(&'static str, &'static str)> {
    match outcome {
        "working" => Some(WORKING),
        "succeeded" => Some(("succeeded", "completed")),
        "failed" => Some(("failed", "stopped")),
        "indeterminate" => Some(("indeterminate", "awaiting_operator")),
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

/// The fixed severity table. An unlisted severity falls back to the
/// empty key — journal data never names a class.
fn severity_class(payload: &Value) -> &'static str {
    match field(payload, "severity") {
        Some("flagged") => "awaiting_operator",
        Some("hard") => "stopped",
        _ => "",
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
    let to = js::parse_millis(to_iso)?;
    let from = js::parse_millis(from_iso)?;
    let millis = to - from;
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

// -------------------------------------- operator commands and residuals

/// The two commands `brokkr operator` accepts, in the order it names
/// them.
pub const OPERATOR_COMMANDS: [&str; 2] = ["retry", "stop"];

/// The phases whose rulings carry residual findings.
pub const RESIDUAL_PHASES: [&str; 2] = ["verify", "review"];

/// The operator commands a run in this status admits. Only a parked run
/// admits any: `retry` re-runs its phase, `stop` ends it, and every
/// other status admits neither. Derived here, once, so a surface that
/// suggests a command suggests one the engine will actually accept
/// rather than one it invented.
pub fn operator_commands(status: &str) -> Vec<String> {
    match status {
        "awaiting_operator" => OPERATOR_COMMANDS.iter().map(|c| c.to_string()).collect(),
        _ => Vec::new(),
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
    /// The rendered sentence every surface prints, citations included.
    pub line: String,
}

/// The one severity input and the two boolean inputs that carry a
/// residual claim, read through the evaluator's own closed vocabulary.
/// A severity of `none`, a false flag, an unranked severity name and any
/// key outside the vocabulary all carry no finding.
fn residual_value(key: &str, value: &Value) -> Option<String> {
    match key {
        "max_residual_severity" => {
            let name = value.as_str()?;
            let rank = brokkr_core::policy::SEVERITY_ORDER
                .iter()
                .position(|known| *known == name)?;
            match rank {
                0 => None,
                _ => Some(name.to_string()),
            }
        }
        "has_security_residual" | "high_risk_uncovered" => match value.as_bool() {
            Some(true) => Some("true".to_string()),
            _ => None,
        },
        _ => None,
    }
}

/// Residual findings as the journal actually carries them: the
/// STRUCTURED rule inputs of every `transition/decided` ruled from
/// `verify` or `review`. Reviewer prose lives in free-text notes and is
/// never re-read here as a typed finding — deriving structure from prose
/// is repair (decision 0001). Each finding names the run and the
/// sequence number it came from, which is decision 0007's provenance
/// discipline applied to a readout instead of to a seat input.
pub fn residual_findings(run_id: &str, events: &[EventEnvelope]) -> Vec<ResidualFinding> {
    let mut out = Vec::new();
    for event in events {
        if event.event_type != EventType::TransitionDecided {
            continue;
        }
        let payload = &event.payload;
        let Some(phase) = field(payload, "from").filter(|from| RESIDUAL_PHASES.contains(from))
        else {
            continue;
        };
        let Some(inputs) = payload.get("inputs").and_then(Value::as_object) else {
            continue;
        };
        let rule_id = display_or_mark(payload.get("rule_id"));
        for (input, raw) in inputs {
            let Some(value) = residual_value(input, raw) else {
                continue;
            };
            out.push(ResidualFinding {
                run_id: run_id.to_string(),
                seq: event.seq,
                phase: phase.to_string(),
                rule_id: rule_id.clone(),
                input: input.clone(),
                value: value.clone(),
                line: format!(
                    "{run_id} seq {} · {phase} · {rule_id} · {input}: {value}",
                    event.seq
                ),
            });
        }
    }
    out
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
    }
}

// ------------------------------------------------------- run rows

fn run_row(entry: &RunEntry) -> RunRow {
    let status = entry
        .state
        .map(|state| status_str(&state.status).to_string());
    let status_known = match &status {
        Some(status) => KNOWN_STATUS.contains(&status.as_str()),
        None => false,
    };
    RunRow {
        run_id: entry.run_id.to_string(),
        status,
        status_known,
        phase: entry.state.and_then(|state| state.phase.clone()),
        seq: entry.state.map(|state| state.seq),
        created_at: entry.created_at.to_string(),
        feature: entry.feature.to_string(),
        detail: entry.detail.map(str::to_string),
    }
}

/// Run rows, newest first. Ordering is a derivation rule, not something
/// each surface reverses for itself.
pub fn run_rows(entries: &[RunEntry]) -> RunsView {
    let mut runs: Vec<RunRow> = entries.iter().map(run_row).collect();
    runs.reverse();
    let count = runs.len();
    RunsView {
        view_version: VIEW_VERSION,
        runs,
        count,
    }
}

/// One hearth as a fleet reader hands it over: the realm it belongs to,
/// the journal it was read from, and either that journal's entries or
/// the words of the refusal that stopped it being read.
pub struct HearthEntries<'a> {
    pub realm: &'a str,
    pub journal: &'a str,
    pub entries: &'a [RunEntry<'a>],
    pub detail: Option<&'a str>,
}

/// The world's fleet, grouped by realm. Each hearth's rows are derived
/// by exactly the same [`run_rows`] a one-journal world uses — the
/// grouping is an arrangement of that derivation, never a second one,
/// and no fold ever crosses a journal boundary (decision 0026 ruling 5).
pub fn fleet_rows(hearths: &[HearthEntries]) -> FleetView {
    let realms: Vec<RealmRuns> = hearths
        .iter()
        .map(|hearth| {
            let view = run_rows(hearth.entries);
            RealmRuns {
                realm: hearth.realm.to_string(),
                journal: hearth.journal.to_string(),
                runs: view.runs,
                count: view.count,
                detail: hearth.detail.map(str::to_string),
            }
        })
        .collect();
    let count = realms.iter().map(|realm| realm.count).sum();
    FleetView {
        view_version: VIEW_VERSION,
        realms,
        count,
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
    effect_index: usize,
    key: String,
    label: String,
    member: Option<String>,
    /// The LAST attempt's provenance for this invocation site: what
    /// actually ran, which is the only thing a fallback is visible in.
    provenance: Option<Value>,
    /// Last provider-reported model for this invocation site.
    model: Option<String>,
    turns: Option<u64>,
    last_turn: Option<Value>,
    session: Option<Value>,
    transcript: Option<Transcript>,
    member_outcome: Option<Value>,
    member_finished_at: Option<String>,
    checkpoints: Vec<(Value, String)>,
    status: &'static str,
    status_class: &'static str,
}

struct Scan {
    effects: Vec<EffectFacts>,
    parts: Vec<Build>,
    by_key: HashMap<String, usize>,
}

fn ensure(scan: &mut Scan, slot: usize, effect_id: &str, member: Option<&str>) -> usize {
    let key = match member {
        Some(member) => format!("{effect_id}:{member}"),
        None => effect_id.to_string(),
    };
    if let Some(index) = scan.by_key.get(&key) {
        return *index;
    }
    let seat = &scan.effects[slot].seat;
    let label = match member {
        Some(member) => format!("{seat}:{member}"),
        None => seat.clone(),
    };
    scan.parts.push(Build {
        effect_index: slot,
        key: key.clone(),
        label,
        member: member.map(str::to_string),
        provenance: None,
        model: None,
        turns: None,
        last_turn: None,
        session: None,
        transcript: None,
        member_outcome: None,
        member_finished_at: None,
        checkpoints: Vec::new(),
        status: WORKING.0,
        status_class: WORKING.1,
    });
    let index = scan.parts.len() - 1;
    scan.by_key.insert(key, index);
    index
}

fn scan_participants(events: &[EventEnvelope]) -> Scan {
    let mut scan = Scan {
        effects: Vec::new(),
        parts: Vec::new(),
        by_key: HashMap::new(),
    };
    let mut by_effect_id: HashMap<String, usize> = HashMap::new();
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
                // Decision 0016's single derivation point. A site named
                // here gets a participant even if it never checkpoints:
                // "which model served this member" is a fact about the
                // attempt, not about whether the member said anything.
                let entries = payload
                    .get("provenance")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for entry in entries {
                    let member = entry
                        .get("member")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let part = ensure(&mut scan, slot, effect_id, member.as_deref());
                    scan.parts[part].provenance = Some(entry);
                }
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
                if let Some(transcript) = checkpoint.get("transcript").and_then(transcript_of) {
                    // ALWAYS replace: a retry's live transcript follows the
                    // concluded attempt's, just as the old session id did.
                    scan.parts[part].transcript = Some(transcript);
                }
                scan.parts[part]
                    .checkpoints
                    .push((checkpoint.clone(), event.recorded_at.clone()));
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
                    // replaces in turn, bringing the cost.
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
        let mut derived = if effect.open.is_some() {
            WORKING
        } else {
            match effect.terminal {
                Some(terminal) => terminal,
                None => WORKING,
            }
        };
        if part.member.is_some() {
            if let Some(outcome) = part.member_outcome.as_ref().and_then(Value::as_str) {
                if let Some(pinned) = outcome_status(outcome) {
                    derived = pinned;
                }
            }
        }
        part.status = derived.0;
        part.status_class = derived.1;
    }
    scan
}

// -------------------------------------------------------------- activity

fn transcript_of(value: &Value) -> Option<Transcript> {
    let kind = value.get("kind").and_then(Value::as_str)?;
    if !matches!(
        kind,
        "claude-session" | "codex-thread" | "dsh-session" | "none"
    ) {
        return None;
    }
    let locator = value.get("locator").and_then(Value::as_str)?;
    let home = value.get("home").and_then(Value::as_str)?;
    Some(Transcript {
        kind: kind.to_string(),
        locator: locator.to_string(),
        home: home.to_string(),
    })
}

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
        part.checkpoints.first().map(|(_, at)| at.as_str())
    } else {
        effect.started_at.as_deref()
    };
    // A member with no panel-member-finished (a sequence's final or exec
    // step) ends at its last checkpoint once the effect is done.
    let end_at: Option<&str> = if part.member.is_some() {
        match part.member_finished_at.as_deref() {
            Some(at) => Some(at),
            None => match part.checkpoints.last() {
                Some((_, at)) if effect.open.is_none() && effect.terminal.is_some() => Some(at),
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
    let live = if part.status == "working" {
        part.last_turn.as_ref()
    } else {
        None
    };
    if let Some(checkpoint) = live {
        let tool = js::to_display(checkpoint.get("tool"));
        let target = match checkpoint.get("target") {
            Some(Value::String(target)) if !target.is_empty() => Some(target.as_str()),
            _ => None,
        };
        let (short, full) = target_cells(target);
        let text = match &short {
            Some(short) => format!("{tool} · {short}"),
            None => tool.clone(),
        };
        return Activity {
            text,
            absent: false,
            note: None,
            tool: Some(tool),
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
    let mut rows = Vec::new();
    for (checkpoint, recorded_at) in &part.checkpoints {
        let target = match checkpoint.get("target") {
            Some(Value::String(target)) if !target.is_empty() => Some(target.as_str()),
            _ => None,
        };
        let (short, full) = target_cells(target);
        let step = match checkpoint.get("step").and_then(Value::as_str) {
            Some("seat-turn") => js::to_display(checkpoint.get("tool")),
            _ => js::to_display(checkpoint.get("step")),
        };
        rows.push(CheckpointRow {
            turn: cell_of(
                checkpoint
                    .get("turn")
                    .and_then(Value::as_u64)
                    .map(|turn| turn.to_string()),
                Some("not a numbered turn"),
            ),
            step,
            model: cell_of(
                checkpoint
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                Some("no served model recorded for this checkpoint"),
            ),
            usage: cell_of(
                token_usage(std::iter::once(checkpoint))
                    .as_ref()
                    .map(|usage| usage_text(usage, "")),
                Some("no token usage recorded for this checkpoint"),
            ),
            target: cell_of(
                short,
                Some("no target recorded — the journal carries file targets only"),
            ),
            target_full: full,
            recorded_at: recorded_at.clone(),
        });
    }
    rows
}

/// Members conclude on their own `panel-member-finished` outcome; the
/// seat itself on the effect's terminal event. A JSON-null outcome falls
/// through to the effect (the console's `memberOutcome !== null`), while
/// an absent one prints the literal `undefined` — a quirk, ported.
fn terminal_line(events: &[EventEnvelope], scan: &Scan, part: &Build) -> Cell {
    let effect = &scan.effects[part.effect_index];
    if part.member.is_some() {
        if let Some(finished_at) = &part.member_finished_at {
            if !matches!(part.member_outcome, Some(Value::Null)) {
                return cell_of(
                    Some(format!(
                        "panel-member-finished · {} · {finished_at}",
                        js::to_display(part.member_outcome.as_ref())
                    )),
                    None,
                );
            }
        }
    }
    match effect.terminal_index {
        Some(index) => cell_of(
            Some(format!(
                "{} · {}",
                type_str(events[index].event_type),
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

fn session_cost(part: &Build) -> Option<f64> {
    part.session
        .as_ref()
        .and_then(|session| session.get("total_cost_usd"))
        .and_then(Value::as_f64)
}

/// The tokens a seat actually spent, summed across its own turns.
///
/// Per-turn checkpoints are authoritative. The finishing checkpoint is
/// only the fallback for old or externally-authored records that do not
/// carry turn accounting; reading both would count the same work twice.
///
/// `cache_read_tokens` is not an addend: a cache read IS an input token,
/// billed differently, and it arrives inside `input_tokens` already
/// (3,830,272 of the wager's 3,975,322). Adding it would double-count.
/// That is the journal's convention, not any one harness's — dsh counts
/// the two disjointly and its adapter folds the cache read back into
/// `input_tokens` before journaling, so this sum holds whichever driver
/// wrote the checkpoint. See "Checkpoints" in
/// `docs/guides/driver-authoring.md`.
fn add_token(total: &mut Option<u64>, count: Option<u64>) {
    if let Some(count) = count {
        *total = Some(total.unwrap_or_default().saturating_add(count));
    }
}

fn finish_usage(mut usage: TokenUsage) -> Option<TokenUsage> {
    usage.total_tokens = match (usage.input_tokens, usage.output_tokens) {
        (None, None) => None,
        (input, output) => Some(
            input
                .unwrap_or_default()
                .saturating_add(output.unwrap_or_default()),
        ),
    };
    if usage.total_tokens.is_some()
        || usage.cache_read_tokens.is_some()
        || usage.cache_write_tokens.is_some()
    {
        Some(usage)
    } else {
        None
    }
}

fn token_usage<'a>(records: impl Iterator<Item = &'a Value>) -> Option<TokenUsage> {
    let mut usage = TokenUsage::default();
    for record in records {
        add_token(
            &mut usage.input_tokens,
            record.get("input_tokens").and_then(Value::as_u64),
        );
        add_token(
            &mut usage.output_tokens,
            record.get("output_tokens").and_then(Value::as_u64),
        );
        add_token(
            &mut usage.cache_read_tokens,
            record.get("cache_read_tokens").and_then(Value::as_u64),
        );
        add_token(
            &mut usage.cache_write_tokens,
            record.get("cache_write_tokens").and_then(Value::as_u64),
        );
    }
    finish_usage(usage)
}

fn session_usage(part: &Build) -> Option<TokenUsage> {
    token_usage(
        part.checkpoints
            .iter()
            .map(|(checkpoint, _)| checkpoint)
            .filter(|checkpoint| {
                matches!(
                    checkpoint.get("step").and_then(Value::as_str),
                    Some("seat-turn" | "turn-completed")
                ) && [
                    "input_tokens",
                    "output_tokens",
                    "cache_read_tokens",
                    "cache_write_tokens",
                ]
                .iter()
                .any(|key| checkpoint.get(*key).and_then(Value::as_u64).is_some())
            }),
    )
    .or_else(|| token_usage(part.session.iter()))
}

fn usage_text(usage: &TokenUsage, prefix: &str) -> String {
    let mut fields = Vec::new();
    for (label, count) in [
        ("in", usage.input_tokens),
        ("out", usage.output_tokens),
        ("cache read", usage.cache_read_tokens),
        ("cache write", usage.cache_write_tokens),
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
    if total < 1_000 {
        return format!("{total} tok");
    }
    let thousands = js::round_half_up(total as f64 / 1_000.0);
    if thousands < 1_000 {
        return format!("{thousands}k tok");
    }
    // Hundredths of a million, placed with integer arithmetic: rounding
    // once here keeps the printed digits from rounding a second time.
    let hundredths = js::round_half_up(total as f64 / 10_000.0);
    format!("{}.{:02}M tok", hundredths / 100, hundredths % 100)
}

fn participants(events: &[EventEnvelope], scan: &Scan) -> Vec<Participant> {
    let mut out = Vec::new();
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
        let mut cost = session_cost(part);
        let mut usage = session_usage(part);
        let mut turns = part.turns;
        let mut cost_aggregated = false;
        let mut usage_aggregated = false;
        let mut turns_aggregated = false;
        if !members.is_empty() {
            let costs: Vec<f64> = members
                .iter()
                .filter_map(|other| session_cost(other))
                .collect();
            let member_usage: Vec<TokenUsage> = members
                .iter()
                .filter_map(|other| session_usage(other))
                .collect();
            let member_turns: Vec<u64> = members.iter().filter_map(|other| other.turns).collect();
            if cost.is_none() && !costs.is_empty() {
                // Summed in participant insertion order: `f64` addition
                // is not associative and the console sums in map order.
                let mut total = 0.0;
                for value in &costs {
                    total += value;
                }
                cost = Some(total);
                cost_aggregated = true;
            }
            if usage.is_none() && !member_usage.is_empty() {
                let mut total = TokenUsage::default();
                for member in &member_usage {
                    add_token(&mut total.input_tokens, member.input_tokens);
                    add_token(&mut total.output_tokens, member.output_tokens);
                    add_token(&mut total.cache_read_tokens, member.cache_read_tokens);
                    add_token(&mut total.cache_write_tokens, member.cache_write_tokens);
                }
                usage = finish_usage(total);
                usage_aggregated = true;
            }
            if turns.is_none() && !member_turns.is_empty() {
                let mut total = 0u64;
                for value in &member_turns {
                    total += value;
                }
                turns = Some(total);
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
        let tokens = usage.as_ref().and_then(|usage| usage.total_tokens);
        let cost_text = match (cost, tokens) {
            (Some(cost), _) => Some(format!("{cost_prefix}${}", js::to_fixed_4(cost))),
            (None, Some(tokens)) => Some(format!("{usage_prefix}{}", fmt_tokens(tokens))),
            (None, None) => None,
        };
        let model = match &part.model {
            Some(model) => Some(model.clone()),
            None if !members.is_empty() => {
                let mut reported: Vec<&str> = members
                    .iter()
                    .filter_map(|member| member.model.as_deref())
                    .collect();
                reported.sort_unstable();
                reported.dedup();
                match reported.as_slice() {
                    [] => None,
                    [only] => Some((*only).to_string()),
                    _ => Some(reported.join(", ")),
                }
            }
            None => None,
        };
        let transcript = part.transcript.clone();
        let session_id = transcript
            .as_ref()
            .filter(|transcript| transcript.kind == "claude-session")
            .map(|transcript| transcript.locator.clone())
            .filter(|locator| !locator.is_empty())
            .or_else(|| {
                part.session
                    .as_ref()
                    .and_then(|session| session.get("session_id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
        out.push(Participant {
            key: part.key.clone(),
            label: part.label.clone(),
            member: part.member.clone(),
            phase: scan.effects[part.effect_index].phase.clone(),
            status: part.status.to_string(),
            status_class: part.status_class.to_string(),
            attempts: scan.effects[part.effect_index].attempts,
            turns,
            turns_aggregated,
            turns_cell: cell_of(
                turns.map(|turns| format!("{turns_prefix}{turns}")),
                Some("no turn telemetry recorded"),
            ),
            cost,
            cost_aggregated,
            cost_cell: cell_of(cost_text, Some("no session cost or token usage recorded")),
            usage_cell: cell_of(
                usage.as_ref().map(|usage| usage_text(usage, usage_prefix)),
                Some("no token usage recorded"),
            ),
            usage,
            usage_aggregated,
            model: cell_of(model, Some("no served model recorded")),
            activity: activity_for(events, scan, part, members.len()),
            member_count: members.len(),
            transcript_cell: transcript_cell(transcript.as_ref()),
            transcript,
            session_id,
            terminal_line: terminal_line(events, scan, part),
            checkpoints: checkpoint_rows(part),
            provenance: part.provenance.as_ref().map(provenance_of),
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
    rows.iter()
        .map(|(label, checkpoint)| {
            let text = match checkpoint {
                Some(checkpoint) => format!(
                    "{label} · turn {} · {}",
                    js::to_display(checkpoint.get("turn")),
                    js::to_display(checkpoint.get("tool"))
                ),
                None => format!("{label} · working"),
            };
            LiveLine {
                label: label.clone(),
                text,
            }
        })
        .collect()
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
    newest: HashMap<&'a str, (&'a str, &'a str)>,
    /// Effect id -> its `effect/checkpointed` events, in journal order.
    checkpoints: HashMap<&'a str, Vec<&'a Value>>,
    /// Effect id -> every phase it was requested in (scope membership).
    effect_phases: HashMap<&'a str, Vec<String>>,
}

fn bucket(events: &[EventEnvelope]) -> Buckets<'_> {
    let mut buckets = Buckets {
        newest: HashMap::new(),
        checkpoints: HashMap::new(),
        effect_phases: HashMap::new(),
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
                    buckets
                        .checkpoints
                        .entry(effect_id)
                        .or_default()
                        .push(payload);
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
    let outcome = part
        .and_then(|part| part.member_outcome.as_ref())
        .and_then(Value::as_str);
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
    let key = match tag {
        Some(tag) => format!("{effect_id}:{tag}"),
        None => effect_id.to_string(),
    };
    let part = scan.by_key.get(&key).map(|index| &scan.parts[*index]);
    let (state, state_class) = node_state(part, done);
    Node {
        label: label.to_string(),
        key,
        state: state.to_string(),
        state_class: state_class.to_string(),
        model: cell_of(
            part.and_then(|part| part.model.clone()),
            Some("no served model recorded"),
        ),
    }
}

/// Inner topology for one phase, from its NEWEST observed effect. Only
/// observed events count: declared-but-unstarted topology is never
/// invented. An empty result means the phase has no observed effect.
fn inner_columns(scan: &Scan, buckets: &Buckets, phase: &str) -> Vec<Column> {
    let Some((effect_id, seat)) = buckets.newest.get(phase).copied() else {
        return Vec::new();
    };
    let mut finished: Vec<String> = Vec::new();
    let mut tags: Vec<String> = Vec::new();
    if let Some(payloads) = buckets.checkpoints.get(effect_id) {
        for payload in payloads {
            let checkpoint = match payload.get("checkpoint") {
                Some(checkpoint) => checkpoint,
                None => &Value::Null,
            };
            if checkpoint.get("step").and_then(Value::as_str) == Some("sequence-step-finished") {
                if let Some(name) = checkpoint.get("step_name").and_then(Value::as_str) {
                    if !finished.iter().any(|step| step == name) {
                        finished.push(name.to_string());
                    }
                }
            } else if let Some(member) = checkpoint.get("member").and_then(Value::as_str) {
                if !tags.iter().any(|tag| tag == member) {
                    tags.push(member.to_string());
                }
            }
        }
    }
    let mut columns: Vec<Column> = Vec::new();
    if !finished.is_empty() {
        let mut steps: Vec<String> = finished.clone();
        let mut step_tags: Vec<(String, Vec<String>)> = Vec::new();
        for tag in &tags {
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

fn phase_rail(
    events: &[EventEnvelope],
    scan: &Scan,
    buckets: &Buckets,
    state: Option<&RunState>,
) -> Vec<Phase> {
    let mut visits: Vec<(String, u64)> = Vec::new();
    // The roads back, as `(landing, departure)` pairs deduped in the
    // order the journal recorded them. A transition is BACKWARD when
    // the phase it names as `next` has already been entered in this
    // run — the same fact `visits` counts, read here from the ruling
    // that caused the revisit rather than from the count it produced.
    let mut returns: Vec<(String, String)> = Vec::new();
    for event in events {
        match event.event_type {
            EventType::PhaseEntered => {
                let Some(name) = field(&event.payload, "phase") else {
                    continue;
                };
                match visits.iter_mut().find(|(known, _)| known == name) {
                    Some((_, count)) => *count += 1,
                    None => visits.push((name.to_string(), 1)),
                }
            }
            EventType::TransitionDecided => {
                // A ruling that parks carries no `next` at all, and a
                // ruling this crate cannot read is never repaired into
                // one (README law 2).
                let (Some(from), Some(next)) =
                    (field(&event.payload, "from"), field(&event.payload, "next"))
                else {
                    continue;
                };
                let backward = visits.iter().any(|(known, _)| known == next);
                let known = returns
                    .iter()
                    .any(|(to, source)| to == next && source == from);
                if backward && !known {
                    returns.push((next.to_string(), from.to_string()));
                }
            }
            _ => continue,
        }
    }
    let current = match state.and_then(|state| state.phase.as_deref()) {
        Some(phase) if visits.iter().any(|(name, _)| name == phase) => Some(phase.to_string()),
        _ => visits.last().map(|(name, _)| name.clone()),
    };
    visits
        .iter()
        .map(|(name, count)| {
            let columns = inner_columns(scan, buckets, name);
            let plain = match columns.first() {
                Some(column) => columns.len() == 1 && column.nodes.len() == 1,
                None => true,
            };
            Phase {
                name: name.clone(),
                visits: *count,
                current: current.as_deref() == Some(name.as_str()),
                plain,
                columns,
                returns: returns
                    .iter()
                    .filter(|(to, _)| to == name)
                    .map(|(_, from)| from.clone())
                    .collect(),
            }
        })
        .collect()
}

// ----------------------------------------------------- trail and journal

fn what_of(scan: &Scan, event: &EventEnvelope) -> What {
    let payload = &event.payload;
    match event.event_type {
        EventType::TransitionDecided => {
            let rule = display_or_mark(payload.get("rule_id"));
            let result = match field(payload, "result") {
                Some(result) => format!(" · {result}"),
                None => String::new(),
            };
            let arrow = format!(
                " {} → {}{result}",
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
                None => type_tail(type_str(other)).to_string(),
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

fn journal_rows(events: &[EventEnvelope], scan: &Scan, buckets: &Buckets) -> Vec<JournalRow> {
    // `verify_chain` pins `seq == i + 1`, so seq 0 is unrepresentable in
    // a loaded journal and the console's truthiness check on the looked-up
    // seq has nothing to guard. A total map is the whole rule.
    let mut by_id: HashMap<&str, u64> = HashMap::new();
    for event in events {
        by_id.insert(event.event_id.as_str(), event.seq);
    }
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
                    let mut models: Vec<&str> = scan
                        .parts
                        .iter()
                        .filter(|candidate| candidate.effect_index == effect_index)
                        .filter_map(|candidate| candidate.model.as_deref())
                        .collect();
                    models.sort_unstable();
                    models.dedup();
                    match models.as_slice() {
                        [] => None,
                        [only] => Some((*only).to_string()),
                        _ => Some(models.join(", ")),
                    }
                });
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
                causation_seq: event
                    .causation_id
                    .as_deref()
                    .and_then(|id| by_id.get(id).copied()),
                event_type: type_str(event.event_type).to_string(),
                recorded_at: event.recorded_at.clone(),
                in_trail: !TRAIL_SKIP.contains(&event.event_type),
                phases,
                what: what_of(scan, event),
                label: label_of(event),
                model: cell_of(recorded_model, Some("no served model recorded")),
                payload_json: match payload {
                    Value::Object(_) => payload.to_string(),
                    _ => "{}".to_string(),
                },
            }
        })
        .collect()
}

// ------------------------------------------------------ summary + ruling

fn summary_of(state: &RunState) -> Summary {
    Summary {
        consecutive_failures: state.consecutive_failures.clone(),
        cursor: format!("{:?}", state.cursor),
        feature: state.feature.clone(),
        last_decision: state.last_decision.clone(),
        park_reason: state.park_reason.clone(),
        phase: state.phase.clone(),
        run_id: state.run_id.clone(),
        seq: state.seq,
        status: status_str(&state.status).to_string(),
    }
}

fn ruling_of(decision: Option<&Value>) -> Option<Ruling> {
    let decision = decision?;
    let object = decision.as_object()?;
    let rule = object.get("rule_id")?;
    let inputs = match object.get("inputs").and_then(Value::as_object) {
        Some(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.to_string()))
            .collect(),
        None => Vec::new(),
    };
    Some(Ruling {
        rule_id: display_or_mark(Some(rule)),
        severity_class: severity_class(decision).to_string(),
        from: display_or_mark(object.get("from")),
        next: display_or_mark(object.get("next")),
        result: match object.get("result") {
            Some(result) if truthy(Some(result)) => Some(js::to_display(Some(result))),
            _ => None,
        },
        inputs,
        problem: match object.get("problem") {
            Some(problem) if truthy(Some(problem)) => Some(js::to_display(Some(problem))),
            _ => None,
        },
    })
}

// ------------------------------------------------------------ entrypoint

/// The whole readout for one run: one derivation, every surface.
pub fn run_view(events: &[EventEnvelope], state: Option<&RunState>) -> RunView {
    let scan = scan_participants(events);
    let buckets = bucket(events);
    RunView {
        view_version: VIEW_VERSION,
        summary: state.map(summary_of),
        ruling: ruling_of(state.and_then(|state| state.last_decision.as_ref())),
        participants: participants(events, &scan),
        live: live_lines(events),
        phases: phase_rail(events, &scan, &buckets, state),
        notices: run_notices(events),
        journal: journal_rows(events, &scan, &buckets),
        event_count: events.len(),
    }
}

#[cfg(test)]
mod tests;
