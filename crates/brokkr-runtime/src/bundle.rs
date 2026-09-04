//! Declarative bundles: policy + one seat per phase (decision 0005).
//! `compile` validates before anything runs and produces the pinned
//! content-addressed manifest. Rejections here are the executable slice
//! of the constitutional lint: a bundle that could reach ship around the
//! protected review phase, name a result no rule covers, or reference a
//! missing role never loads at all.

use brokkr_protocol::hands::HandsSpec;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::policy::{Machine, BOOLEAN_INPUTS, IDENTIFIER_INPUTS, SEVERITY_INPUTS};
use serde_json::{json, Map, Value};
use thiserror::Error;

pub mod compose;

use compose::{Ancestor, COMPOSE_PREFIX};

use crate::agents::{
    resolve_route, Adapter, Adapters, Availability, Candidate, EgressClass, Library, TrustTier,
};
use crate::dialect::{Dialect, DIALECT_PHASES};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const EVENT_SCHEMA: u32 = 1;
pub const DRIVER_PROTOCOL: u32 = 1;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("bundle: {0}")]
    Invalid(String),
    #[error("bundle io: {0}")]
    Io(#[from] std::io::Error),
    #[error("bundle json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bundle policy: {0}")]
    Policy(#[from] brokkr_core::PolicyError),
}

/// Inputs the engine owns. A seat may never supply or declare these:
/// journal-computed truth is never accepted from a caller (README law 2).
pub const ENGINE_OWNED_INPUTS: [&str; 7] = [
    "consecutive_failures",
    "drift_detected",
    "dirty_worktrees",
    "reviewed_heads",
    // The fold remembers the last successful triage result. A seat may
    // neither declare nor overwrite the class that governs its run.
    "strategy",
    // Read from the tree at the protected phase's ruling (decision
    // 0039): the review's own commits, classified by the repository's
    // declared docs class.
    "fixes_docs_only",
    // The same repository facts, keyed by realm (decision 0023). Read
    // from the tree by the engine, exactly like the two above it.
    REALM_FACTS,
];

/// The per-realm repository facts a decision records in a mapped world:
/// realm name -> observed HEAD, dirty worktree, drift.
pub const REALM_FACTS: &str = "realm_facts";

/// Closed, seat-declarable enum inputs. Their values are validated by the
/// pure policy evaluator whenever a ruling reads them.
pub const ENUM_INPUTS: [&str; 1] = ["drift_in"];

/// The two results synthesized by a dialect exec for each phase. This is
/// also the vocabulary recorded on every compiled dialect step, so dispatch
/// and sequence-boundary reasoning cannot drift apart.
pub(crate) fn dialect_results(phase: &str) -> [&'static str; 2] {
    match phase {
        "clarify" => ["clear", "ambiguous"],
        "analyze" => ["consistent", "drift"],
        "verify" => ["pass", "fail"],
        _ => ["drafted", "fail"],
    }
}

/// The same law over the phase-visit family (decision 0022): every
/// `visits_<phase>` is counted by the fold from `phase/entered` events,
/// so no seat may declare one and no seat may claim one.
pub fn is_engine_owned(name: &str) -> bool {
    ENGINE_OWNED_INPUTS.contains(&name) || name.starts_with(brokkr_core::policy::VISIT_PREFIX)
}

#[derive(Debug, Clone)]
pub struct Seat {
    /// True when every invocation site in this seat is a gate. The runtime
    /// uses the compiled fact to enforce a stable repository HEAD around the
    /// whole effect; mixed sequences carry the same fact per step instead.
    pub has_gate: bool,
    pub results: Vec<String>,
    pub limits: Limits,
    /// Typed facts this seat may supply (decision 0007). Anything else a
    /// seat sends is dropped before evaluation and never enters the
    /// journal record. Defaults to the non-engine-owned inputs the
    /// phase's own rules reference.
    pub inputs: Vec<String>,
    /// Secret NAMES this seat binds (decision 0012) — exactly parallel
    /// to the 0007 input declaration: declared or dropped. Bundles and
    /// digests carry names only; values live in the operator-side store
    /// and are resolved by the exec driver at spawn time, never here.
    pub secrets: Vec<String>,
    pub body: SeatBody,
}

/// Optional container confinement for a driver (the policy-confined
/// trust class): the command runs inside a pinned image with only the
/// declared mounts, network off unless granted. Absence = trusted
/// native process. Data, like everything else about a seat.
#[derive(Debug, Clone)]
pub struct Confine {
    pub image: String,
    pub network: bool,
    /// Extra read-only mounts beyond the workdir and bundle dir.
    pub mounts: Vec<String>,
}

/// One agent session, a parallel panel joined by a declared
/// deterministic rule, or a serial sequence of named steps (decision
/// 0002's sanctioned forms: composition INSIDE the executor — one
/// effect, one typed result at the boundary; inner structure is
/// journaled as checkpoint evidence).
#[derive(Debug, Clone)]
pub enum SeatBody {
    Single {
        role_path: PathBuf,
        command: Vec<String>,
        confine: Option<Confine>,
        /// The bounded fallback chain (decision 0016). EMPTY for an
        /// inline seat, which is what keeps the execute path — and
        /// therefore inline behaviour — exactly as it was.
        candidates: Vec<Candidate>,
    },
    Panel {
        members: Vec<PanelMember>,
        aggregate: Aggregate,
    },
    /// Named steps run one after another INSIDE one effect: later steps
    /// see earlier steps' result objects as context, and the FINAL
    /// step's result is the effect's single typed result.
    Sequence { steps: Vec<SequenceStep> },
    /// A strategy-selected body (decision 0041 ruling 7). Cases contain
    /// only the three executable body forms above; selection itself never
    /// nests, so one journal fact resolves this to one ordinary body.
    Select {
        cases: BTreeMap<String, SeatBody>,
        default: Option<Box<SeatBody>>,
        case_gates: BTreeMap<String, bool>,
        default_gate: bool,
    },
}

#[derive(Clone, Copy)]
pub enum ExecutableBody<'a> {
    Single {
        role_path: &'a Path,
        command: &'a [String],
        confine: Option<&'a Confine>,
        candidates: &'a [Candidate],
    },
    Panel {
        members: &'a [PanelMember],
        aggregate: Aggregate,
    },
    Sequence {
        steps: &'a [SequenceStep],
    },
}

/// The delivery classes which can reach a selected seat. `escalate` parks
/// in triage and therefore is not a selectable delivery route.
pub const SELECT_STRATEGIES: [&str; 4] = ["chore", "feature", "design", "engine"];

impl SeatBody {
    /// Resolve the body from the folded journal. `None` means the journal
    /// carries no triage result and this selector has no default.
    pub fn selected<'a>(
        &'a self,
        strategy: Option<&str>,
    ) -> Option<(ExecutableBody<'a>, Option<&'a str>)> {
        let (body, case) = match self {
            SeatBody::Select { cases, default, .. } => {
                match strategy.and_then(|key| cases.get_key_value(key)) {
                    Some((key, body)) => (body, Some(key.as_str())),
                    None => (default.as_deref()?, Some("default")),
                }
            }
            body => (body, None),
        };
        let executable = match body {
            SeatBody::Single {
                role_path,
                command,
                confine,
                candidates,
            } => ExecutableBody::Single {
                role_path,
                command,
                confine: confine.as_ref(),
                candidates,
            },
            SeatBody::Panel { members, aggregate } => ExecutableBody::Panel {
                members,
                aggregate: *aggregate,
            },
            SeatBody::Sequence { steps } => ExecutableBody::Sequence { steps },
            SeatBody::Select { .. } => return None,
        };
        Some((executable, case))
    }

    pub fn selected_is_gate(&self, strategy: Option<&str>, ordinary: bool) -> bool {
        match self {
            SeatBody::Select {
                case_gates,
                default_gate,
                ..
            } => strategy
                .and_then(|case| case_gates.get(case).copied())
                .unwrap_or(*default_gate),
            _ => ordinary,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SequenceStep {
    pub name: String,
    pub class: SeatClass,
    /// A non-final step's own closed vocabulary. The final step is the
    /// seat boundary and therefore receives the seat's vocabulary.
    pub results: Vec<String>,
    pub body: StepBody,
}

#[derive(Debug, Clone)]
pub struct DialectExecution {
    pub argv: Vec<String>,
    pub state: Option<Vec<String>>,
}

/// A sequence step's body: one driver, or a panel joined by a declared
/// aggregate — the same two forms a seat itself may take.
#[derive(Debug, Clone)]
pub enum StepBody {
    Single {
        role_path: PathBuf,
        command: Vec<String>,
        confine: Option<Confine>,
        candidates: Vec<Candidate>,
    },
    Panel {
        members: Vec<PanelMember>,
        aggregate: Aggregate,
    },
    /// A realm-dialect validator or deterministic loop check.
    Dialect { execution: DialectExecution },
}

#[derive(Debug, Clone)]
pub struct PanelMember {
    pub name: String,
    pub role_path: PathBuf,
    pub command: Vec<String>,
    pub confine: Option<Confine>,
    pub candidates: Vec<Candidate>,
}

/// Deterministic, order-independent aggregation rules — a closed
/// vocabulary, like conditions: named in data, implemented in the
/// engine, never arbitrary code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregate {
    /// "pass" only when every member reports "pass"; otherwise "fail".
    UnanimousPass,
    /// Worst-member-wins over clean < residual < security-hold; severity
    /// is the max, security and fixes flags are OR-ed.
    ReviewPanel,
}

impl Aggregate {
    fn parse(name: &str) -> Option<Aggregate> {
        match name {
            "unanimous-pass" => Some(Aggregate::UnanimousPass),
            "review-panel" => Some(Aggregate::ReviewPanel),
            _ => None,
        }
    }
    fn required_results(&self) -> &'static [&'static str] {
        match self {
            Aggregate::UnanimousPass => &["pass", "fail"],
            Aggregate::ReviewPanel => &["clean", "residual", "security-hold"],
        }
    }
}

/// Which side of decision 0021 ruling 1 a driver-bearing site sits on.
/// Work sites produce output the machine checks; gate sites ARE the
/// check, and nobody stands behind the judges. The division is bundle
/// data declared per site — the engine holds no roster of which phase
/// judges, exactly as it holds no roster of which vendor is trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatClass {
    Work,
    Gate,
}

impl SeatClass {
    fn parse(name: &str) -> Option<SeatClass> {
        match name {
            "work" => Some(SeatClass::Work),
            "gate" => Some(SeatClass::Gate),
            _ => None,
        }
    }
}

/// Compilation has already validated every `class` occurrence. Preserve the
/// whole-effect fact ruling 4 needs without mistaking a mixed sequence for a
/// gate: its work steps may move HEAD before its gate step judges them.
fn is_gate_class(value: &Value) -> bool {
    if let Some(select) = value.get("select").and_then(Value::as_object) {
        let cases = select.get("cases").and_then(Value::as_object);
        let defaults = select.get("default").into_iter();
        return cases.is_some_and(|cases| {
            let mut bodies = cases.values().chain(defaults);
            bodies
                .next()
                .is_some_and(|first| is_gate_class(first) && bodies.all(is_gate_class))
        });
    }
    if let Some(members) = value.get("panel").and_then(Value::as_object) {
        return !members.is_empty() && members.values().all(is_gate_class);
    }
    if let Some(steps) = value.get("sequence").and_then(Value::as_array) {
        return !steps.is_empty() && steps.iter().all(is_gate_class);
    }
    value.get("class").and_then(Value::as_str) == Some("gate")
}

/// Per-seat autonomy limits (decision 0006). Defaults keep the old
/// behavior: one attempt, one-hour deadline.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_attempts: u64,
    pub timeout_seconds: u64,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_attempts: 1,
            timeout_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bundle {
    pub name: String,
    /// Human-facing library metadata. Empty only for legacy or system
    /// bundles that predate the contributor recipe catalogue.
    pub description: String,
    /// A relative cost band, not a quote; provider rates remain outside
    /// the control-plane contract.
    pub cost: String,
    pub dir: PathBuf,
    /// Every layer's directory, leaf first — `[dir]` for a recipe that
    /// composed nothing. Confinement mounts all of them, so an inherited
    /// seat's role file is inside the container it runs in.
    pub roots: Vec<PathBuf>,
    /// The composition chain, nearest ancestor first (decision 0017).
    /// Empty unless the recipe extends another.
    pub chain: Vec<Ancestor>,
    pub machine: Machine,
    pub seats: BTreeMap<String, Seat>,
    pub manifest: Value,
    /// Decision 0043: the sites whose hands are one boxed tool, keyed as
    /// the manifest's `hands` key is — seat, `seat:member`, `seat:step`,
    /// `seat:step:member` — and as the engine labels the driver seat.
    pub hands: BTreeMap<String, HandsSpec>,
    /// The phase every path to a non-stop terminal must traverse.
    pub protected_phase: String,
    /// Dialect-owned prose, resolved once at compile time and keyed by the
    /// phase whose office receives it. Review is supplied only to the
    /// spec-compliance member by the engine.
    pub dialect_prompts: BTreeMap<String, String>,
}

/// The default library roots, resolved against the current working
/// directory exactly as `--recipes-dir` is. Read ONLY when a seat, panel
/// member or sequence step actually references an agent, which is what
/// makes a missing library a non-event for every bundle that inlines.
pub const DEFAULT_AGENTS_DIR: &str = "agents";
pub const DEFAULT_ADAPTERS_DIR: &str = "adapters";

/// The agent library and adapters for one compilation, plus the
/// per-invocation-site records that become the manifest's `agents` key.
///
/// The adapters are the wider need: an agent reference resolves through
/// them, AND decision 0021's two refusals read a driver's declarations
/// out of them. The LIBRARY is opened only when a seat actually names an
/// agent, which is what keeps a missing one a non-event for every bundle
/// that inlines.
struct AgentContext {
    library: Option<Library>,
    adapters: Adapters,
    records: Map<String, Value>,
    /// Per INLINE driver-bearing site, the adapter digests decision
    /// 0021's refusals consulted to let it stand. An agent site pins its
    /// adapters through the resolution record in `records`; a raw
    /// `driver.command` had nothing to pin, so the declaration that
    /// authorised a gate would otherwise sit outside the bundle's
    /// identity — a demotion could not be told from a re-run.
    drivers: Map<String, Value>,
    /// Decision 0036 ruling 4: the egress class a seat's resolved route
    /// must MEET before that seat may declare secret bindings. The
    /// operator rules it into the bundle; absence is `Contracted`, which
    /// is exactly what `binding_grant: true` meant, so every bundle on
    /// disk keeps the behaviour it has.
    egress_minimum: EgressClass,
}

/// One resolved agent reference, ready to become an ordinary seat body.
struct ResolvedSeat {
    role_path: PathBuf,
    command: Vec<String>,
    candidates: Vec<Candidate>,
    limits: Option<Limits>,
    inputs: Option<Vec<String>>,
    /// Decision 0043: the agent's hands, when it declared them.
    hands: Option<HandsSpec>,
}

fn resolved_hands(seat: &ResolvedSeat) -> Option<HandsSpec> {
    seat.hands.clone()
}

/// Does this bundle reference an agent anywhere? A bundle that does not
/// never opens the library, so a tree without one compiles exactly as it
/// did before decision 0016.
fn mentions_agent(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.contains_key("agent") || map.values().any(mentions_agent),
        Value::Array(items) => items.iter().any(mentions_agent),
        _ => false,
    }
}

/// Does this bundle need the ADAPTER data at compile time? An agent
/// reference resolves through it, and decision 0021's refusals read a
/// driver's tier and grant out of it — so a gate-class site or a
/// declared secret binding needs it too, even in a bundle that names no
/// agent at all (`bundles/verify` and `recipes/fast` are exactly that).
/// A bundle with none of the three has nothing to check and still
/// compiles with no `adapters/` directory in sight.
fn needs_adapters(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key("agent")
                || map.contains_key("dialect")
                || map.contains_key("secrets")
                || map.get("class").and_then(Value::as_str) == Some("gate")
                || map.values().any(needs_adapters)
        }
        Value::Array(items) => items.iter().any(needs_adapters),
        _ => false,
    }
}

/// The model-bearing built-in named by one raw driver command. Custom
/// drivers own their own contract; exec has no model to pin.
fn built_in_model_driver(raw: &Value) -> Option<&str> {
    let command = raw.pointer("/driver/command").and_then(Value::as_array)?;
    if command.get(1).and_then(Value::as_str) != Some("driver") {
        return None;
    }
    match command.get(2).and_then(Value::as_str) {
        Some(kind @ ("claude" | "lanetally" | "codex" | "dsh")) => Some(kind),
        _ => None,
    }
}

/// What one inline command's argv says about one pin — the model pin on
/// `--model`, the effort pin on `--effort` (decision 0035). THREE states,
/// because decision 0031 and decision 0036 ruling 2 read the same walk
/// for different facts and only 0031 can collapse the last two: 0031
/// ruling 2 asks whether there IS a readable pin, while 0036 asks WHERE
/// the material goes — and "this argv names no model" and "this argv
/// names a model I cannot read" are different answers to that second
/// question. Kept apart here so the two rulings cannot drift over what
/// "the pinned model" means, and so the route resolver never has to
/// guess which kind of silence it was handed. One walker for both axes,
/// because both are the same rule: named once, concretely, never as a
/// second flag that could outrank the first and never as something that
/// reads as a flag itself.
enum ModelPin {
    /// No such flag anywhere in the argv: `exec`, and every driver that
    /// takes its destination from its own profile.
    Absent,
    /// One readable concrete value; for a model, its `<route>/` prefix
    /// is the route.
    Concrete(String),
    /// The flag is present and this compiler cannot read it as one
    /// concrete value: flag-shaped, empty, over-long, outside the id
    /// alphabet, dangling at the end of the argv, or pinned twice. It
    /// carries the flags the read used to reach that answer, because
    /// decision 0040 ruling 3 says a refusal names them and no constant
    /// stands in for a flag the read did not use.
    Unreadable(Vec<String>),
}

/// Decision 0040 ruling 2: a flag is SHORT when it is one dash and one
/// character, which is the shape the getopt convention lets a value
/// attach itself to (`-mspark/x`). Every other flag — the long `--model`
/// this engine composes, and anything else an adapter may declare — has
/// exactly the two spellings `FLAG VALUE` and `FLAG=VALUE`, and a longer
/// word beginning with it is a different flag of the same family rather
/// than an illegible spelling of this one.
fn short_flag(flag: &str) -> bool {
    let mut characters = flag.chars();
    characters.next() == Some('-')
        && matches!(characters.next(), Some(c) if c != '-')
        && characters.next().is_none()
}

/// The flag every adapter's route pin is read on beside its own
/// declaration (decision 0040 ruling 1), and the flag this engine
/// composes for the four model-bearing built-ins (decisions 0031, 0016).
const MODEL_FLAG: &str = "--model";

/// One walker, one flag, and the spellings that flag's own SHAPE has
/// (decision 0040 ruling 2) — not the spellings its caller asked for.
/// The shape is a property of the string, so 0031's reader and 0036's
/// reader can no longer disagree about what a word means: a long
/// neighbour is walked past by both, and a short flag's attached value
/// is a pin to both.
fn command_pin(raw: &Value, flag: &str, limit: usize) -> ModelPin {
    let illegible = || ModelPin::Unreadable(vec![flag.to_string()]);
    let concrete = |value: &str| {
        !value.is_empty()
            && !value.starts_with('-')
            && value.chars().count() <= limit
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    };
    let attached = format!("{flag}=");
    let Some(command) = raw.pointer("/driver/command").and_then(Value::as_array) else {
        return ModelPin::Absent;
    };
    let mut pin = None;
    let mut parts = command.iter().filter_map(Value::as_str);
    while let Some(part) = parts.next() {
        if part == flag {
            match parts.next().filter(|value| concrete(value)) {
                Some(value) if pin.replace(value).is_none() => {}
                _ => return illegible(),
            }
        } else if let Some(value) = part.strip_prefix(&attached) {
            if !concrete(value) || pin.replace(value).is_some() {
                return illegible();
            }
        } else if let Some(value) = part.strip_prefix(flag) {
            // A word that BEGINS with the flag and is neither of the
            // two spellings above. Decision 0040 ruling 2 decides it by
            // the flag's shape rather than by who is asking:
            //
            // - a SHORT flag carries its value attached, by the getopt
            //   convention every CLI this fleet has met honours, so the
            //   remainder IS the pin. A remainder that is not one
            //   concrete model id is illegible, which costs
            //   `-march=native` under `-m` a refusal naming the flag
            //   and nothing else;
            // - a LONG flag has no such form, so a longer word is an
            //   unrelated flag of the same family and every reader
            //   walks past it. `--model-fallback` is not a way of
            //   writing `--model`, for 0036 as much as for 0031.
            if short_flag(flag) && (!concrete(value) || pin.replace(value).is_some()) {
                return illegible();
            }
        }
    }
    match pin {
        Some(model) => ModelPin::Concrete(model.to_string()),
        None => ModelPin::Absent,
    }
}

/// The model pin on the flag this engine composes for the four
/// model-bearing built-ins: decision 0031 ruling 2's read, which asks
/// only whether one concrete id is stated. A neighbouring `--model…`
/// flag is a different flag to it, not an illegible spelling of this
/// one — now because `--model` is long, not because this caller said so.
fn model_pin(raw: &Value) -> ModelPin {
    command_pin(raw, MODEL_FLAG, 80)
}

/// The model pin as decision 0036 ruling 2 reads it: which route the
/// material goes to, or which kind of silence the argv holds — on one
/// of the two flags decision 0040 ruling 1 has it read.
fn route_pin(raw: &Value, flag: &str) -> ModelPin {
    command_pin(raw, flag, 80)
}

/// Decision 0040 ruling 1's read, and the whole of it: an inline site's
/// route, taken from the flag the adapter DECLARES (`model_flag`, since
/// decision 0016) and from `--model`, the one flag every model CLI this
/// fleet has met accepts. Where the two are the same string that is one
/// read, as it has always been.
///
/// The join is the ruling: a concrete pin on either flag names the
/// route, because the material goes where the argv says on whichever
/// flag the provider honours; two concrete pins naming DIFFERENT ids are
/// illegible, refused naming both, because the material can only go one
/// place and the argv says two; and a read that could not be made out on
/// either flag stays illegible, since a seat binding a secret does not
/// get the benefit of a doubt the machine genuinely has.
///
/// An adapter declaring `model_flag: "unsupported"` still has `--model`
/// read. A provider that cannot be told a model has no route to name, so
/// a pin found there is a site telling a binary something the adapter
/// says it cannot hear: illegible, never the adapter's own class.
fn inline_route_pin(raw: &Value, adapter: Option<&Adapter>) -> ModelPin {
    let declared = adapter.and_then(|adapter| adapter.model_flag.as_deref());
    let untellable = matches!(adapter, Some(adapter) if adapter.model_flag.is_none());
    let mut readings = Vec::new();
    if let Some(flag) = declared.filter(|flag| *flag != MODEL_FLAG) {
        readings.push(route_pin(raw, flag));
    }
    readings.push(match (route_pin(raw, MODEL_FLAG), untellable) {
        (ModelPin::Concrete(_), true) => ModelPin::Unreadable(vec![MODEL_FLAG.to_string()]),
        (pin, _) => pin,
    });

    let illegible: Vec<String> = readings
        .iter()
        .filter_map(|pin| match pin {
            ModelPin::Unreadable(flags) => Some(flags.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    if !illegible.is_empty() {
        return ModelPin::Unreadable(illegible);
    }
    let mut named: Vec<&String> = readings
        .iter()
        .filter_map(|pin| match pin {
            ModelPin::Concrete(model) => Some(model),
            _ => None,
        })
        .collect();
    named.dedup();
    match named.as_slice() {
        [] => ModelPin::Absent,
        [model] => ModelPin::Concrete((*model).clone()),
        // Two flags, two destinations, one body of material: the argv
        // cannot be obeyed, so it is not read.
        _ => ModelPin::Unreadable(
            declared
                .into_iter()
                .chain(Some(MODEL_FLAG))
                .map(str::to_string)
                .collect(),
        ),
    }
}

fn command_pins_model(raw: &Value) -> bool {
    matches!(model_pin(raw), ModelPin::Concrete(_))
}

/// The effort pin bound, matching `seat-record/v2`'s own: a level is one
/// bounded word, never a path and never a sentence.
fn command_pins_effort(raw: &Value) -> bool {
    matches!(command_pin(raw, "--effort", 40), ModelPin::Concrete(_))
}

/// Every driver-bearing invocation site that states one of the two pins
/// and not the other, or neither.
#[derive(Default)]
struct Unpinned {
    model: Vec<String>,
    effort: Vec<String>,
}

/// Inspect every driver-bearing invocation site in the already composed
/// bundle. Agent references are pinned by their resolved candidate
/// chains — a chain that names no effort for an effort-bearing provider
/// is refused where the vocabulary is known, in the resolver — while
/// inline built-ins must state both concrete pins in their argv.
fn collect_unpinned(what: &str, raw: &Value, out: &mut Unpinned) {
    if raw.get("agent").is_some() {
        return;
    }
    // Both built-in lists are the same list: exec has neither a model to
    // pin nor an effort to pin, and a custom driver owns its own
    // contract. So a site is asked for both pins or for neither, and a
    // seat missing both is named in both halves of one refusal.
    if built_in_model_driver(raw).is_some() {
        if !command_pins_model(raw) {
            out.model.push(what.to_string());
        }
        if !command_pins_effort(raw) {
            out.effort.push(what.to_string());
        }
        return;
    }
    if let Some(panel) = raw.get("panel").and_then(Value::as_object) {
        for (member, member_raw) in panel {
            collect_unpinned(&format!("{what}:{member}"), member_raw, out);
        }
    }
    if let Some(sequence) = raw.get("sequence").and_then(Value::as_array) {
        for (index, step) in sequence.iter().enumerate() {
            let name = step
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("step-{}", index + 1));
            collect_unpinned(&format!("{what}:{name}"), step, out);
        }
    }
    if let Some(select) = raw.get("select").and_then(Value::as_object) {
        if let Some(cases) = select.get("cases").and_then(Value::as_object) {
            for (case, body) in cases {
                collect_unpinned(&format!("{what}:{case}"), body, out);
            }
        }
        if let Some(body) = select.get("default") {
            collect_unpinned(&format!("{what}:default"), body, out);
        }
    }
}

fn labels(sites: &[String]) -> String {
    sites
        .iter()
        .map(|site| format!("'{site}'"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// One refusal names the complete repair set, on BOTH axes. A model pin
/// without an effort pin is half a hire (decision 0035 ruling 5), so the
/// two clauses stand beside each other rather than the first hiding the
/// second behind a second compile.
fn enforce_model_pins(seats: &Map<String, Value>) -> Result<(), CompileError> {
    let mut unpinned = Unpinned::default();
    for (phase, raw) in seats {
        collect_unpinned(phase, raw, &mut unpinned);
    }
    let mut refusals = Vec::new();
    if !unpinned.model.is_empty() {
        refusals.push(format!(
            "seats {} do not pin a model; add '--model <concrete-model-id>' to each \
             driver.command (decision 0031 ruling 2)",
            labels(&unpinned.model)
        ));
    }
    if !unpinned.effort.is_empty() {
        refusals.push(format!(
            "seats {} do not pin an effort; add '--effort <level>' — one of the \
             levels that driver's adapter declares — to each driver.command \
             (decision 0035 ruling 5)",
            labels(&unpinned.effort)
        ));
    }
    if refusals.is_empty() {
        return Ok(());
    }
    Err(CompileError::Invalid(refusals.join("; ")))
}

impl Bundle {
    /// Compile against the default `agents/` and `adapters/` roots.
    pub fn compile(dir: &Path) -> Result<Bundle, CompileError> {
        Bundle::compile_with(
            dir,
            Path::new(DEFAULT_AGENTS_DIR),
            Path::new(DEFAULT_ADAPTERS_DIR),
        )
    }

    pub fn compile_with(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
    ) -> Result<Bundle, CompileError> {
        let default_path = library_root
            .parent()
            .unwrap_or(Path::new(""))
            .join("dialects/openspec.json");
        let default = if default_path.is_file() {
            Some(
                Dialect::load(&default_path)
                    .map_err(|error| CompileError::Invalid(error.to_string()))?
                    .0,
            )
        } else {
            None
        };
        Self::compile_with_realm(dir, library_root, adapters_root, None, default.as_ref())
    }

    /// Compile in a named realm. Supplying this context makes the dialect
    /// requirement enforceable before any journal is opened.
    pub fn compile_with_realm(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
        realm_name: Option<&str>,
        dialect: Option<&Dialect>,
    ) -> Result<Bundle, CompileError> {
        let dir = dir
            .canonicalize()
            .map_err(|e| CompileError::Invalid(format!("bundle dir {}: {e}", dir.display())))?;
        // Composition resolves FIRST, into one flat bundle; everything
        // below this line compiles a single bundle and never learns that
        // composition happened (decision 0017).
        let resolved = compose::resolve(&dir)?;
        let note = resolved.chain_note();
        match Bundle::assemble(
            &dir,
            resolved,
            library_root,
            adapters_root,
            realm_name,
            dialect,
        ) {
            Ok(bundle) => Ok(bundle),
            // Every failure downstream of resolution on a composed
            // bundle is wrapped ONCE with the chain — one arm, rather
            // than teaching each lint about layers.
            Err(error) => Err(match note {
                Some(note) => CompileError::Invalid(format!("{error} ({note})")),
                None => error,
            }),
        }
    }

    /// The agent roots ride through: composition resolves the bundle,
    /// then agent references inside the RESOLVED seats resolve against
    /// the library and adapters (decisions 0016 and 0017 layered).
    fn assemble(
        dir: &Path,
        resolved: compose::Resolved,
        library_root: &Path,
        adapters_root: &Path,
        realm_name: Option<&str>,
        dialect: Option<&Dialect>,
    ) -> Result<Bundle, CompileError> {
        let config = &resolved.document;
        let name = resolved.name.clone();
        let description = config
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let cost = config
            .get("cost")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let table = resolved.table.clone();
        // One refusal names the complete repair set. Running this on the
        // flattened seats also means inherited omissions cannot hide in
        // a composition layer.
        enforce_model_pins(&resolved.seats)?;
        let machine = Machine::from_table(&table)?;
        let uses_dialect = machine
            .phases
            .iter()
            .any(|phase| DIALECT_PHASES.contains(&phase.as_str()));

        let mut dialect_prompts = BTreeMap::new();
        if uses_dialect {
            if let Some(dialect) = dialect {
                for phase in DIALECT_PHASES.into_iter().chain(["implement", "review"]) {
                    let rendered = dialect.rendered.get(phase).cloned().ok_or_else(|| {
                        CompileError::Invalid(format!(
                            "dialect '{}' pin carries no rendered instructions for phase '{phase}'",
                            dialect.name
                        ))
                    })?;
                    dialect_prompts.insert(phase.to_string(), rendered);
                }
            }
        }

        if let Some(phase) = machine
            .phases
            .iter()
            .find(|phase| DIALECT_PHASES.contains(&phase.as_str()))
        {
            if realm_name.is_some() && dialect.is_none() {
                return Err(CompileError::Invalid(format!(
                    "realm '{}' declares no dialect, but phase '{phase}' needs one",
                    realm_name.unwrap_or("<unmapped>")
                )));
            }
        }

        let egress_minimum = parse_egress_minimum(config)?;

        let protected_phase = config
            .get("protected_phase")
            .and_then(Value::as_str)
            .unwrap_or("review")
            .to_string();
        if !machine.phases.contains(&protected_phase) {
            return Err(CompileError::Invalid(format!(
                "policy has no '{protected_phase}' phase; the protected review \
                 gate is non-removable (extension model, layer 1)"
            )));
        }
        assert_phase_unavoidable(&machine, &table, &protected_phase)?;

        // Compile-time resolution depends on exactly two digested inputs:
        // the library and the adapters. Availability is UNSPECIFIED here —
        // a compile that probed PATH would give one bundle two digests and
        // make an in-flight run unresumable after an `apt install`. The
        // COMPOSED seats are what is scanned: a base may be what carries
        // the agent reference.
        let mut agents = match uses_dialect || resolved.seats.values().any(needs_adapters) {
            false => None,
            true => Some(AgentContext {
                library: match resolved.seats.values().any(mentions_agent) {
                    false => None,
                    true => Some(
                        Library::load(library_root)
                            .map_err(|e| CompileError::Invalid(e.to_string()))?,
                    ),
                },
                adapters: Adapters::load(adapters_root).map_err(|e| {
                    CompileError::Invalid(format!(
                        "{e}; the adapter data is where a driver's model mapping \
                         (decision 0016) and its trust tier and binding grant \
                         (decision 0021) are declared, and this bundle names an \
                         agent, seats a gate, or declares a secret binding"
                    ))
                })?,
                records: Map::new(),
                drivers: Map::new(),
                egress_minimum,
            }),
        };

        let mut seats = BTreeMap::new();
        let mut hands: BTreeMap<String, HandsSpec> = BTreeMap::new();
        for (phase, raw) in &resolved.seats {
            // An inherited seat's `role` and `./`-prefixed argv resolve
            // against the layer that WROTE them, found by name — the
            // resolver never looked inside the seat to learn this.
            let dir = &resolved.roots[resolved.seat_origin[phase]];
            if !machine.phases.contains(phase) {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' names a phase the policy does not have"
                )));
            }
            refuse_unknown_keys(phase, raw, SEAT_KEYS)?;
            let results = raw
                .get("results")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|r| !r.is_empty())
                .ok_or_else(|| {
                    CompileError::Invalid(format!("seat '{phase}' needs non-empty 'results'"))
                })?;
            for result in &results {
                let covered = machine
                    .rules
                    .iter()
                    .any(|rule| rule.from == *phase && rule.result == *result);
                // Decision 0041 reserves the implementer's `oversized`
                // verdict before ruling 6 seats triage. Until that phase
                // exists there can be no edge to it; the unmatched verdict
                // parks fail-closed. Once triage exists, ordinary coverage
                // is mandatory and the roster test pins both bounded arms.
                let reserved_oversized = phase == "implement"
                    && result == "oversized"
                    && !machine.phases.iter().any(|known| known == "triage");
                if !covered && !reserved_oversized {
                    return Err(CompileError::Invalid(format!(
                        "seat '{phase}' may emit '{result}' but no rule covers it; \
                         result variants without an outer rule are rejected"
                    )));
                }
            }
            let has_agent = raw.get("agent").is_some();
            if has_agent {
                refuse_amendments(phase, raw)?;
            }
            let has_single =
                !has_agent && (raw.get("role").is_some() || raw.get("driver").is_some());
            let has_panel = raw.get("panel").is_some();
            let has_sequence = raw.get("sequence").is_some();
            let has_select = raw.get("select").is_some();
            if [has_single, has_panel, has_sequence, has_agent, has_select]
                .iter()
                .filter(|f| **f)
                .count()
                > 1
            {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' must be exactly one of role+driver, agent, \
                     panel, sequence, or select"
                )));
            }
            let secrets = parse_secrets(phase, raw)?;
            let agent_seat = match has_agent {
                false => None,
                true => Some(resolve_reference(
                    &mut agents,
                    dir,
                    phase,
                    phase,
                    raw,
                    &secrets,
                    Site::Seat,
                )?),
            };
            let mut body = if let Some(agent_seat) = &agent_seat {
                SeatBody::Single {
                    role_path: agent_seat.role_path.clone(),
                    command: agent_seat.command.clone(),
                    confine: parse_confine(phase, raw)?,
                    candidates: agent_seat.candidates.clone(),
                }
            } else if has_panel {
                let (members, aggregate) =
                    parse_panel(dir, phase, raw, &results, &secrets, &mut agents, &mut hands)?;
                SeatBody::Panel { members, aggregate }
            } else if has_sequence {
                SeatBody::Sequence {
                    steps: parse_sequence(
                        dir,
                        phase,
                        raw,
                        &mut agents,
                        &mut hands,
                        BodyCompile {
                            results: &results,
                            secrets: &secrets,
                            dialect,
                        },
                    )?,
                }
            } else if has_select {
                parse_select(
                    dir,
                    phase,
                    raw,
                    &mut agents,
                    &mut hands,
                    SelectCompile {
                        results: &results,
                        secrets: &secrets,
                        roots: &resolved.roots,
                        case_origin: &resolved.case_origin,
                        dialect,
                    },
                )?
            } else {
                SeatBody::Single {
                    role_path: parse_role(dir, phase, raw)?,
                    command: parse_command(dir, phase, raw, &secrets)?,
                    confine: parse_confine(phase, raw)?,
                    candidates: Vec::new(),
                }
            };
            if phase == "verify" && uses_dialect {
                if let Some(command) = dialect.and_then(|dialect| match &dialect.verify {
                    crate::dialect::CommandOrUnsupported::Command(command) => Some(command),
                    crate::dialect::CommandOrUnsupported::Unsupported(_) => None,
                }) {
                    if let SeatBody::Single { candidates, .. } = &body {
                        enforce_model_policy(phase, raw, candidates, &secrets, &mut agents)?;
                        record_hands(
                            phase,
                            raw,
                            agent_seat.as_ref().and_then(resolved_hands),
                            &secrets,
                            &mut hands,
                        )?;
                    }
                    let prior_body =
                        match body {
                            SeatBody::Single {
                                role_path,
                                command,
                                confine,
                                candidates,
                            } => StepBody::Single {
                                role_path,
                                command,
                                confine,
                                candidates,
                            },
                            SeatBody::Panel { members, aggregate } => {
                                StepBody::Panel { members, aggregate }
                            }
                            _ => return Err(CompileError::Invalid(
                                "dialect verify currently requires a single or panel verify seat"
                                    .into(),
                            )),
                        };
                    let prior = SequenceStep {
                        name: "checks".into(),
                        class: if is_gate_class(raw) {
                            SeatClass::Gate
                        } else {
                            SeatClass::Work
                        },
                        results: results.clone(),
                        body: prior_body,
                    };
                    if let Some(spec) = hands.remove(phase) {
                        hands.insert(format!("{phase}:checks"), spec);
                    }
                    let context = agents
                        .as_mut()
                        .expect("dialect compilation opens the adapter context");
                    for records in [&mut context.records, &mut context.drivers] {
                        if let Some(record) = records.remove(phase) {
                            records.insert(format!("{phase}:checks"), record);
                        }
                    }
                    let dialect_site = format!("{phase}:dialect-verify");
                    let synthetic = json!({"class":"gate", "hands":"workspace", "driver":{"command":["{brokkr}","driver","exec","--"]}});
                    enforce_model_policy(&dialect_site, &synthetic, &[], &secrets, &mut agents)?;
                    record_hands(&dialect_site, &synthetic, None, &secrets, &mut hands)?;
                    body = SeatBody::Sequence {
                        steps: vec![
                            prior,
                            SequenceStep {
                                name: "dialect-verify".into(),
                                class: SeatClass::Gate,
                                results: dialect_results(phase)
                                    .into_iter()
                                    .map(str::to_string)
                                    .collect(),
                                body: StepBody::Dialect {
                                    execution: DialectExecution {
                                        argv: command.argv.clone(),
                                        state: command.state.clone(),
                                    },
                                },
                            },
                        ],
                    };
                }
            }
            // Decision 0021, at the seat's own driver-bearing site. A
            // panel or a sequence has none: its members and steps were
            // each checked where they were built.
            match &body {
                SeatBody::Single { candidates, .. } => {
                    enforce_model_policy(phase, raw, candidates, &secrets, &mut agents)?;
                    record_hands(
                        phase,
                        raw,
                        agent_seat.as_ref().and_then(resolved_hands),
                        &secrets,
                        &mut hands,
                    )?;
                }
                SeatBody::Select { .. } => refuse_class_without_a_driver(phase, raw)?,
                SeatBody::Sequence { .. } if phase == "verify" && uses_dialect => {}
                _ => refuse_class_without_a_driver(phase, raw)?,
            }
            // Decision 0006 bounds belong to the strategy seat, not the
            // office it hires. A roster entry supplies the default, while
            // an explicit seat limit (night-shift's one-attempt gates) wins.
            let limits = if raw.get("limits").is_some() {
                parse_limits(phase, raw)?
            } else {
                match agent_seat.as_ref().and_then(|agent_seat| agent_seat.limits) {
                    Some(limits) => limits,
                    None => parse_limits(phase, raw)?,
                }
            };
            // Input provenance (decision 0007): every input the phase's
            // rules reference must be engine-computed or supplied by
            // this seat's declaration; a declaration may only name known,
            // non-engine-owned evaluator inputs. An agent-resolved seat
            // faces this lint exactly as an inline one does — its
            // declaration is the agent's, or the 0007 default.
            let referenced = referenced_seat_inputs(&table, phase);
            let raw_inputs = match agent_seat
                .as_ref()
                .and_then(|agent_seat| agent_seat.inputs.clone())
            {
                Some(declared) => Some(json!(declared)),
                None => raw.get("inputs").cloned(),
            };
            let inputs = match raw_inputs {
                None => referenced.clone(),
                Some(declared) => {
                    let declared = declared
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .ok_or_else(|| {
                            CompileError::Invalid(format!(
                                "seat '{phase}' inputs must be an array of strings"
                            ))
                        })?;
                    for name in &declared {
                        if is_engine_owned(name) {
                            return Err(CompileError::Invalid(format!(
                                "seat '{phase}' declares engine-owned input '{name}'; \
                                 journal-computed truth is never accepted from a seat"
                            )));
                        }
                        if !declarable_input(name) {
                            return Err(CompileError::Invalid(format!(
                                "seat '{phase}' declares unknown input '{name}'; known: \
                                 the evaluator's closed vocabulary minus engine-owned"
                            )));
                        }
                    }
                    for needed in &referenced {
                        if !declared.contains(needed) {
                            return Err(CompileError::Invalid(format!(
                                "phase '{phase}' rules reference input '{needed}' but \
                                 seat '{phase}' does not declare it; the rule could \
                                 never fire from seat data"
                            )));
                        }
                    }
                    declared
                }
            };
            seats.insert(
                phase.clone(),
                Seat {
                    has_gate: is_gate_class(raw),
                    results,
                    limits,
                    inputs,
                    secrets,
                    body,
                },
            );
        }

        for phase in &machine.phases {
            if machine.terminal.contains(phase) {
                continue;
            }
            if !seats.contains_key(phase) {
                return Err(CompileError::Invalid(format!(
                    "non-terminal phase '{phase}' has no seat (no executor can run it)"
                )));
            }
        }

        let select_records: Map<String, Value> = seats
            .iter()
            .filter_map(|(site, seat)| match &seat.body {
                SeatBody::Select { .. } => Some((site.clone(), body_manifest(&seat.body))),
                _ => None,
            })
            .collect();
        let manifest = manifest_for(
            dir,
            &name,
            &resolved.chain,
            agents.as_ref().map(|a| &a.records),
            agents.as_ref().map(|a| &a.drivers),
            &hands,
            &select_records,
        )?;
        Ok(Bundle {
            hands,
            name,
            description,
            cost,
            dir: dir.to_path_buf(),
            roots: resolved.roots,
            chain: resolved.chain,
            machine,
            seats,
            manifest,
            protected_phase,
            dialect_prompts,
        })
    }

    pub fn manifest_digest(&self) -> String {
        brokkr_core::canonical::sha256_hex(&self.manifest)
    }
}

fn body_manifest(body: &SeatBody) -> Value {
    let single = |candidates: &[Candidate]| {
        if candidates.is_empty() {
            json!({"driver": "inline"})
        } else {
            json!({"agent": candidates[0].agent, "candidates": candidates.iter().map(|candidate| json!({
                "provider": candidate.provider,
                "model": candidate.model,
                "effort": candidate.effort,
            })).collect::<Vec<_>>()})
        }
    };
    match body {
        SeatBody::Single { candidates, .. } => single(candidates),
        SeatBody::Panel { members, aggregate } => json!({
            "panel": members.iter().map(|member| (member.name.clone(), single(&member.candidates))).collect::<Map<_, _>>(),
            "aggregate": match aggregate { Aggregate::UnanimousPass => "unanimous-pass", Aggregate::ReviewPanel => "review-panel" },
        }),
        SeatBody::Sequence { steps } => json!({"sequence": steps.iter().map(|step| {
            let body = match &step.body {
                StepBody::Single { candidates, .. } => single(candidates),
                StepBody::Panel { members, aggregate } => json!({
                    "panel": members.iter().map(|member| (member.name.clone(), single(&member.candidates))).collect::<Map<_, _>>(),
                    "aggregate": match aggregate { Aggregate::UnanimousPass => "unanimous-pass", Aggregate::ReviewPanel => "review-panel" },
                }),
                StepBody::Dialect { execution } => json!({
                    "dialect": {"argv": execution.argv, "state": execution.state},
                }),
            };
            json!({"name": step.name, "results": step.results, "body": body})
        }).collect::<Vec<_>>() }),
        SeatBody::Select { cases, default, .. } => {
            let mut resolved: Map<String, Value> = cases
                .iter()
                .map(|(case, body)| (case.clone(), body_manifest(body)))
                .collect();
            if let Some(body) = default {
                resolved.insert("default".into(), body_manifest(body));
            }
            Value::Object(resolved)
        }
    }
}

/// Removing the protected phase must disconnect every non-stop terminal
/// from the initial phase — no table path ships around review.
fn assert_phase_unavoidable(
    machine: &Machine,
    table: &Value,
    protected: &str,
) -> Result<(), CompileError> {
    let rules = table["rules"]
        .as_array()
        .expect("validated by Machine::from_table");
    let mut reachable = vec![machine.initial.clone()];
    let mut frontier = vec![machine.initial.clone()];
    while let Some(node) = frontier.pop() {
        for rule in rules {
            let from = rule["from"].as_str().unwrap_or_default();
            // A rule that parks (decision 0022) draws no edge: it reaches
            // no phase, so it can carry no path around review.
            let Some(next) = rule["next"].as_str() else {
                continue;
            };
            // `frontier` never contains the protected phase: it is excluded
            // from every push below. Therefore `from != protected` follows
            // from `from == node` and need not be re-tested.
            if from == node && next != protected {
                let next = next.to_string();
                if !reachable.contains(&next) {
                    reachable.push(next.clone());
                    frontier.push(next);
                }
            }
        }
    }
    for terminal in &machine.terminal {
        if terminal != "stop" && reachable.contains(terminal) {
            return Err(CompileError::Invalid(format!(
                "policy reaches terminal '{terminal}' without passing '{protected}'; \
                 a path to shipping that bypasses the protected review gate is \
                 constitutionally rejected"
            )));
        }
    }
    Ok(())
}

/// Parse a panel body (`"panel": {…members…}, "aggregate": "…"`) at
/// `what` — a seat's phase or a sequence step's `<phase>:<step>` label.
/// `declared_results` is checked to cover the aggregate's vocabulary
/// when Some: always for a seat-level panel (its aggregate reaches
/// decide()), but for a panel STEP only when it is the FINAL step — a
/// non-final step's aggregate output never reaches decide(), it only
/// feeds later steps as context.
/// Where an agent reference sits. A seat OWNS its 0006 bounds and its
/// 0007 declaration; a panel member or sequence step has neither — the
/// seat above it does. So an agent carrying `limits` or `inputs` cannot
/// be referenced from a member or a step: silently discarding them would
/// make `brokkr agents show` a lie for that site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Site {
    Seat,
    Member,
}

/// An agent reference is TOTAL: a seat that could amend the agent it
/// names would make `agent: implementer` stop being a complete statement
/// about what ran — inlining with extra steps, and drift with a name on
/// it. `results`, `secrets`, `class` and `driver.confine` stay legal
/// beside it, because they are bindings the SEAT provides rather than
/// statements about what the agent is, and `brokkr agents show` never
/// claims to show them. `limits` is likewise a strategy bound on the
/// invocation, not part of the office. `class` in particular is the
/// seat's authority, never the agent's: one charter may sit in a work
/// seat here and a gate seat there, which is why decision 0021 ruling 1
/// puts the division in bundle data.
fn refuse_amendments(what: &str, raw: &Value) -> Result<(), CompileError> {
    let refuse = |key: &str| {
        Err(CompileError::Invalid(format!(
            "seat '{what}' combines 'agent' with '{key}'; an agent reference is \
             total — '{key}' states what the agent IS, and a seat that could \
             amend it would make `brokkr agents show` a lie for that seat"
        )))
    };
    for key in ["role", "inputs", "hands"] {
        if raw.get(key).is_some() {
            return refuse(key);
        }
    }
    if let Some(driver) = raw.get("driver") {
        let object = driver.as_object().ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' driver must be an object"))
        })?;
        for key in object.keys() {
            if key != "confine" {
                return refuse(&format!("driver.{key}"));
            }
        }
    }
    Ok(())
}

/// Resolve `"agent": "<name>"` into an ordinary seat body, and record
/// the resolution under this invocation site.
fn resolve_reference(
    agents: &mut Option<AgentContext>,
    dir: &Path,
    what: &str,
    site_key: &str,
    raw: &Value,
    secrets: &[String],
    site: Site,
) -> Result<ResolvedSeat, CompileError> {
    let name = raw
        .get("agent")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' agent must be a non-empty string"))
        })?;
    let context = agents
        .as_mut()
        .expect("a bundle mentioning an agent loads the library");
    let library = context
        .library
        .as_ref()
        .expect("a bundle mentioning an agent opens the library");
    let resolution = crate::agents::resolve(
        library,
        &context.adapters,
        &Availability::unspecified(),
        name,
    )
    .map_err(|e| CompileError::Invalid(format!("seat '{what}': {e}")))?;
    if site == Site::Member {
        for (key, present) in [
            ("limits", resolution.limits.is_some()),
            ("inputs", resolution.inputs.is_some()),
        ] {
            if present {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' references agent '{name}', which declares \
                     '{key}'; a panel member or sequence step has no {key} of its \
                     own — the seat above it does — so the declaration could only \
                     be discarded silently"
                )));
            }
        }
    }
    // The composed argv faces the SAME secret-reference lint as an inline
    // one (decision 0012), and `{brokkr}` is expanded by the same
    // function, so a resolved seat is an inline seat by construction.
    let mut candidates = Vec::with_capacity(resolution.candidates.len());
    for candidate in &resolution.candidates {
        lint_secret_refs(what, &candidate.argv, secrets)?;
        candidates.push(Candidate {
            agent: candidate.agent.clone(),
            model: candidate.model.clone(),
            effort: candidate.effort.clone(),
            provider: candidate.provider.clone(),
            argv: expand_command(dir, &candidate.argv),
        });
    }
    context
        .records
        .insert(site_key.to_string(), resolution.record.clone());
    Ok(ResolvedSeat {
        role_path: resolution.charter.clone(),
        command: candidates[0].argv.clone(),
        candidates,
        limits: resolution.limits,
        inputs: resolution.inputs.clone(),
        hands: resolution.hands.clone(),
    })
}

/// A site's decision-0021 class, as written. ABSENT is `Work`: the
/// division is a declaration, and a site that declares nothing claims no
/// judging authority, so nothing about it is being trusted. A class the
/// vocabulary does not name is a refusal, in the manner of an unknown
/// aggregate — the closed vocabularies of this engine all fail the same
/// way.
fn parse_class(what: &str, raw: &Value) -> Result<SeatClass, CompileError> {
    let Some(declared) = raw.get("class") else {
        return Ok(SeatClass::Work);
    };
    declared.as_str().and_then(SeatClass::parse).ok_or_else(|| {
        CompileError::Invalid(format!(
            "seat '{what}' has unknown class {declared}; known: work, gate \
             (decision 0021 ruling 1) — an undeclared site is work"
        ))
    })
}

/// The bundle's egress minimum (decision 0036 ruling 4): the class a
/// seat's resolved route must MEET before that seat may declare secret
/// bindings. It is the operator's bar, ruled per bundle, and an absent
/// one is `contracted` — exactly what the superseded `binding_grant:
/// true` meant — so every bundle on disk keeps its present behaviour and
/// its present digest. Read whether or not this bundle turns out to need
/// the adapters: a bar written in a vocabulary this engine does not
/// speak is a refusal wherever it is written, never a silent default.
fn parse_egress_minimum(config: &Value) -> Result<EgressClass, CompileError> {
    let Some(declared) = config.get("egress_minimum") else {
        return Ok(EgressClass::Contracted);
    };
    declared
        .as_str()
        .and_then(EgressClass::parse)
        .ok_or_else(|| {
            CompileError::Invalid(format!(
                "bundle 'egress_minimum' is {declared}; the egress vocabulary is \
                 closed — {} — and an absent minimum is \"contracted\" \
                 (decision 0036 ruling 4)",
                EgressClass::VOCABULARY
            ))
        })
}

/// The keys a SEAT may write. Closed, like the class vocabulary itself
/// and for the same reason — see [`refuse_unknown_keys`].
const SEAT_KEYS: &[&str] = &[
    "results",
    "inputs",
    "limits",
    "secrets",
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "panel",
    "aggregate",
    "sequence",
    "select",
];

const BODY_KEYS: &[&str] = &[
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "panel",
    "aggregate",
    "sequence",
];

/// The keys a PANEL MEMBER may write. Narrower than a seat's: a member
/// has no `results`, `limits`, `inputs` or `secrets` of its own — the
/// seat above it does — which is why an agent declaring them at a member
/// site is already refused rather than silently discarded.
const MEMBER_KEYS: &[&str] = &["class", "agent", "role", "driver", "hands"];

/// The keys a SEQUENCE STEP may write: a member's, plus its name, plus
/// the two a step needs to be a panel of its own.
const STEP_KEYS: &[&str] = &[
    "name",
    "results",
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "panel",
    "aggregate",
    "dialect",
];

/// The vocabulary of a site object is CLOSED, because since decision
/// 0021 a dropped key is a dropped refusal. `class` is read by absence —
/// an undeclared site is work — so `"clas": "gate"` would leave a
/// judging site classed work, and every gate refusal below it unarmed,
/// with nothing anywhere to say so. The same silence would swallow a
/// misspelled `secrets`. So a key this compiler does not read is refused
/// where it is written, in the manner of an unknown class or an unknown
/// aggregate: the fail-closed reading of an absent declaration is only
/// honest if an absence cannot be manufactured by a typo.
fn refuse_unknown_keys(what: &str, raw: &Value, known: &[&str]) -> Result<(), CompileError> {
    let Some(object) = raw.as_object() else {
        return Ok(());
    };
    for key in object.keys() {
        if !known.contains(&key.as_str()) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' has unknown key '{key}'; known: {}. The site \
                 vocabulary is closed because a declaration this compiler \
                 cannot see is a declaration that was never made — a \
                 misspelled 'class' would leave a gate reading as work \
                 (decision 0021 ruling 1)",
                known.join(", ")
            )));
        }
    }
    Ok(())
}

/// A panel or a sequence has no driver of its own, so it has no class of
/// its own: `recipes/triage`'s `design` seat is a panel of work positions,
/// a work chief and a gate check, and a single word on the seat could
/// only be an approximation of all three. Refused rather than averaged.
fn refuse_class_without_a_driver(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.get("class") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares a class but bears no driver of its own; \
             decision 0021 ruling 1 classes each driver-bearing site, so a \
             panel's members and a sequence's steps each carry their own"
        ))),
    }
}

/// The driver an INLINE site names, read structurally off its raw
/// (pre-expansion) command: decision 0009's dispatch convention is
/// `<engine> driver <name> -- …`, so the token after the literal
/// `driver` IS the driver, the same way `{brokkr}` is a protocol marker
/// this compiler already recognises. The engine token itself is not
/// matched on — a bundle may spell it `{brokkr}`, `{forge}`, or the
/// absolute path of the binary it means, and all three are the same
/// dispatch. `None` for any other shape: a raw process is a driver that
/// declares nothing, which decision 0021 reads as untrusted and
/// ungranted rather than as exempt.
fn dispatch_driver(parts: &[String]) -> Option<String> {
    match parts {
        [_, marker, name, ..] if marker == "driver" => Some(name.clone()),
        _ => None,
    }
}

/// A resolved route, as a refusal says it out loud. One phrase for the
/// unrouted case, written once: an adapter that named no route for this
/// id answers on its own declared destination, and a driver no adapter
/// declares has only that destination to be judged on either.
fn destination((route, egress): (Option<&str>, EgressClass)) -> (String, EgressClass) {
    match route {
        Some(route) => (format!("on route '{route}'"), egress),
        None => ("on its own declared destination".to_string(), egress),
    }
}

/// The same phrase for a site whose pin could not be read, naming the
/// flags decision 0040 ruling 1's read actually used — one where the
/// declared flag IS `--model`, both where they differ and either was
/// illegible or the two named different destinations.
fn unreadable_destination(flags: &[String]) -> String {
    let named: Vec<String> = flags.iter().map(|flag| format!("'{flag}'")).collect();
    let (pin, is) = match named.len() {
        1 => ("pin", "is"),
        _ => ("pins", "are"),
    };
    format!(
        "on a destination it does not name (its {} {pin} {is} not one readable \
         concrete model id, so no route can be read off it)",
        named.join(" and "),
    )
}

/// Decision 0021's two compile-time prohibitions, at one driver-bearing
/// site. Both are a lookup and a comparison — deterministic code with an
/// exit status (decision 0025 ruling 6), refusing before any prompt
/// exists to leak, in the manner of a digest mismatch:
///
/// - a GATE-class site whose driver lacks the trusted tier (ruling 2);
/// - a site under a seat that declares secret bindings, whose resolved
///   ROUTE does not meet the bundle's egress minimum (ruling 4, as
///   enacted by decision 0036 ruling 4).
///
/// The two axes stay separate all the way down: the gate refusal reads
/// `trust_tier` and nothing else, because decision 0036 ruling 3 holds
/// that local is structural and confers no standing to judge — a model
/// on the operator's own hardware may be the most private worker in the
/// fleet and remain the least qualified to be the check.
///
/// Both fail closed on ABSENCE — an undeclared tier is untrusted, an
/// undeclared class is uncontracted, and a driver no adapter declares
/// has neither. `candidates` is the resolved fallback chain of an agent
/// site and empty for an inline one; EVERY link is checked, because
/// ruling 5 says an unavailable driver parks rather than substitutes,
/// and a chain that could fall back to an untrusted judge at run time
/// would have defeated the gate at compile time.
///
/// A site that SURVIVES both is witnessed when it is inline: the adapter
/// whose declaration authorised it is pinned into the manifest, so the
/// bundle's identity carries what let it judge. An agent site needs no
/// entry here — its resolution record already pins every adapter its
/// chain consulted.
fn enforce_model_policy(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    secrets: &[String],
    agents: &mut Option<AgentContext>,
) -> Result<(), CompileError> {
    let class = parse_class(what, raw)?;
    if class == SeatClass::Work && secrets.is_empty() {
        return Ok(());
    }
    // Destructured rather than borrowed whole: the lookup reads the
    // adapters while the witness writes beside them, and they are
    // disjoint fields of the same context.
    let AgentContext {
        adapters,
        drivers: witnessed,
        egress_minimum,
        ..
    } = agents
        .as_mut()
        .expect("a gate-class or secret-binding seat opens the adapters");
    let minimum = *egress_minimum;
    let mut authorised = Map::new();
    // Each site as (driver, concrete model id): the id is what carries
    // the ROUTE (decision 0036 ruling 2), and an agent chain's abstract
    // name becomes concrete through the adapter that maps it.
    let drivers: Vec<(usize, Option<String>, Option<String>, ModelPin)> =
        match candidates.is_empty() {
            // An inline site's route is read off its own argv, on the flag
            // the ADAPTER names (`model_flag`, since decision 0016) AND on
            // `--model` (decision 0040 ruling 1). A provider the operator
            // adds that takes `-m` is read on `-m`, so the route named
            // there is the route this site reaches; and it is read on
            // `--model` too, because the same CLI commonly honours that as
            // well, and a pin the engine declined to read there went to an
            // unruled route on the adapter's own clearance. Both doors,
            // held shut by one read. Where no adapter answers for the
            // driver, the `(None, _)` arm below lands the site on
            // `Uncontracted` whatever this returns.
            true => {
                let driver = dispatch_driver(&command_parts(raw));
                let pin = inline_route_pin(
                    raw,
                    driver
                        .as_deref()
                        .and_then(|provider| adapters.adapter(provider)),
                );
                let abstract_model = driver
                    .as_deref()
                    .and_then(|provider| adapters.adapter(provider))
                    .and_then(|adapter| match &pin {
                        ModelPin::Concrete(concrete) => adapter
                            .models
                            .iter()
                            .find_map(|(name, id)| (id == concrete).then(|| name.clone())),
                        _ => None,
                    });
                vec![(1, driver, abstract_model, pin)]
            }
            false => candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| {
                    let (_, concrete) = adapters
                        .serving(&candidate.model)
                        .expect("resolution mapped every link of the chain");
                    (
                        index + 1,
                        Some(candidate.provider.clone()),
                        Some(candidate.model.clone()),
                        ModelPin::Concrete(concrete.to_string()),
                    )
                })
                .collect(),
        };
    for (link, driver, abstract_model, pin) in drivers {
        let adapter = driver
            .as_deref()
            .and_then(|provider| adapters.adapter(provider));
        let named = match &driver {
            Some(provider) => format!("driver '{provider}'"),
            None => "an unnamed driver (the command is no driver dispatch)".to_string(),
        };
        // Decision 0043 ruling 3: a deterministic `exec` command whose
        // hands are boxed has no stochastic axis to distrust, and its
        // blast radius is the box. It may hold a gate.
        let boxed_exec = driver.as_deref() == Some("exec") && raw.get("hands").is_some();
        if class == SeatClass::Gate
            && !boxed_exec
            && adapter.map(|a| a.trust_tier) != Some(TrustTier::Trusted)
        {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' is gate class but seats {named}, which does not \
                 hold the trusted tier; a gate seat IS the check, and nobody \
                 stands behind the judges (decision 0021 ruling 2 — an \
                 undeclared tier is untrusted)"
            )));
        }
        // Decision 0041 ruling 3: trust authorises a provider, while
        // `judges` authorises the particular abstract hire. Both must
        // hold on every fallback link. A missing declaration is the
        // empty set, just like an absent tier is untrusted.
        if class == SeatClass::Gate && !boxed_exec {
            let admitted = adapter
                .zip(abstract_model.as_deref())
                .is_some_and(|(adapter, model)| adapter.judges.iter().any(|judge| judge == model));
            if !admitted {
                let model = abstract_model.as_deref().unwrap_or(match pin {
                    ModelPin::Absent => "<unpinned>",
                    _ => "<unmapped>",
                });
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' gate link {link} names model '{model}', which {named} does not declare in 'judges' (decision 0041 ruling 3 — an absent declaration is empty)"
                )));
            }
        }
        // Where this site's material actually goes. Two kinds of
        // not-knowing land on the floor for the one ruling-1 reason —
        // an absent declaration is uncontracted — but a refusal has to
        // tell them apart, so each names itself:
        //
        // - a driver NO adapter declares: the operator has said nothing
        //   about this binary's endpoint at all;
        // - a model pin this compiler cannot READ as one concrete id.
        //   Ruling 2 gives an unprefixed id the adapter's own class
        //   because an unprefixed id genuinely arrives at the
        //   destination that class is the operator's word about. A site
        //   that writes a pin has declined that default for somewhere
        //   the machine cannot name, so the adapter's word no longer
        //   covers it — reading the adapter's class here would clear an
        //   unnameable route on the strength of a ruling about a
        //   different one, the same fail-open ruling 2's asymmetry
        //   exists to prevent. `enforce_model_pins` refuses this shape
        //   first for the four model-bearing built-ins, all of which
        //   take `--model`; an adapter the operator ADDS is not on that
        //   list, and this refusal is what stands between it and its
        //   adapter's clearance. It can stand there because the pin
        //   above is read on that adapter's OWN declared `model_flag`
        //   AND on `--model` (decision 0040 ruling 1): an adapter
        //   taking `-m` has no second door to walk an unruled route
        //   through, and no third one on the flag its real CLI also
        //   honours. Nor a fourth by spelling — a word carrying either
        //   flag in a form no reading covers arrives here as
        //   `Unreadable` and is refused, rather than as the silence of
        //   an argv that never named a destination at all.
        let (reached, egress) = match (adapter, &pin) {
            (Some(adapter), ModelPin::Concrete(model)) => {
                destination(resolve_route(adapter, model))
            }
            // An argv naming no model leaves the binary on whatever its
            // profile resolves, which carries no prefix — so it is
            // literally the unprefixed case, and `resolve_route` stays
            // the one place ruling 2's rule is written.
            (Some(adapter), ModelPin::Absent) => destination(resolve_route(adapter, "")),
            // Named on the flags the read actually used, and only
            // those (decision 0040 ruling 3): no constant stands in for
            // a flag this read did not touch.
            (Some(_), ModelPin::Unreadable(flags)) => {
                (unreadable_destination(flags), EgressClass::Uncontracted)
            }
            (None, _) => destination((None, EgressClass::Uncontracted)),
        };
        if !secrets.is_empty() && egress < minimum {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' declares secret bindings {secrets:?} but seats \
                 {named} {reached}, whose egress class is {}; this bundle binds \
                 no secret below {} (decision 0021 ruling 4 as enacted by 0036 \
                 ruling 4 — an undeclared class is uncontracted, and \
                 'egress_minimum' is where the operator rules the bar)",
                egress.name(),
                minimum.name(),
            )));
        }
        // Both prohibitions passed, so an adapter answered for this
        // driver. Only an inline site is recorded: `candidates` is empty
        // exactly there.
        if let (Some(provider), Some(adapter)) = (driver.filter(|_| candidates.is_empty()), adapter)
        {
            authorised.insert(provider, Value::String(adapter.digest.clone()));
        }
    }
    if !authorised.is_empty() {
        witnessed.insert(what.to_string(), Value::Object(authorised));
    }
    Ok(())
}

/// Decision 0043: record one site's hands — the agent's when the site
/// names an agent, the site's own `hands` value when it is inline. A
/// site with hands and secret bindings is refused: the box clears the
/// environment, and a binding that cannot reach its seat is a binding
/// silently dropped.
fn record_hands(
    what: &str,
    raw: &Value,
    from_agent: Option<HandsSpec>,
    secrets: &[String],
    hands: &mut BTreeMap<String, HandsSpec>,
) -> Result<(), CompileError> {
    let spec = match (from_agent, raw.get("hands")) {
        (Some(spec), _) => spec,
        (None, Some(declared)) => HandsSpec::parse(declared).map_err(|problem| {
            CompileError::Invalid(format!("seat '{what}' hands: {problem} (decision 0043)"))
        })?,
        (None, None) => return Ok(()),
    };
    if !secrets.is_empty() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' declares hands and secret bindings {secrets:?}; the box \
             clears the environment, so a boxed seat cannot receive a binding \
             (decision 0043)"
        )));
    }
    hands.insert(what.to_string(), spec);
    Ok(())
}

/// A seat's decision-0006 bounds, as written inline.
fn parse_limits(phase: &str, raw: &Value) -> Result<Limits, CompileError> {
    let Some(raw_limits) = raw.get("limits") else {
        return Ok(Limits::default());
    };
    let object = raw_limits
        .as_object()
        .ok_or_else(|| CompileError::Invalid(format!("seat '{phase}' limits must be an object")))?;
    let mut limits = Limits::default();
    for (key, value) in object {
        let number = value.as_u64().filter(|n| *n >= 1).ok_or_else(|| {
            CompileError::Invalid(format!(
                "seat '{phase}' limits.{key} must be an integer >= 1"
            ))
        })?;
        match key.as_str() {
            "max_attempts" => limits.max_attempts = number,
            "timeout_seconds" => limits.timeout_seconds = number,
            other => {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' has unknown limit '{other}'"
                )))
            }
        }
    }
    Ok(limits)
}

/// Parse one body inside a selector. The label includes the case name, so
/// every existing refusal identifies both the seat and the bad case.
#[derive(Clone, Copy)]
struct BodyCompile<'a> {
    results: &'a [String],
    secrets: &'a [String],
    dialect: Option<&'a Dialect>,
}

fn parse_selected_body(
    dir: &Path,
    what: &str,
    raw: &Value,
    agents: &mut Option<AgentContext>,
    hands: &mut BTreeMap<String, HandsSpec>,
    compile: BodyCompile<'_>,
) -> Result<SeatBody, CompileError> {
    let BodyCompile {
        results, secrets, ..
    } = compile;
    refuse_unknown_keys(what, raw, BODY_KEYS)?;
    let has_agent = raw.get("agent").is_some();
    if has_agent {
        refuse_amendments(what, raw)?;
    }
    let has_single = !has_agent && (raw.get("role").is_some() || raw.get("driver").is_some());
    let has_panel = raw.get("panel").is_some();
    let has_sequence = raw.get("sequence").is_some();
    if [has_single, has_panel, has_sequence, has_agent]
        .iter()
        .filter(|present| **present)
        .count()
        != 1
    {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' must be exactly one of role+driver, agent, panel, or sequence"
        )));
    }
    if has_agent {
        let resolved = resolve_reference(agents, dir, what, what, raw, secrets, Site::Seat)?;
        enforce_model_policy(what, raw, &resolved.candidates, secrets, agents)?;
        record_hands(what, raw, resolved.hands, secrets, hands)?;
        let body = SeatBody::Single {
            role_path: resolved.role_path,
            command: resolved.command,
            confine: parse_confine(what, raw)?,
            candidates: resolved.candidates,
        };
        Ok(body)
    } else if has_panel {
        let (members, aggregate) = parse_panel(dir, what, raw, results, secrets, agents, hands)?;
        refuse_class_without_a_driver(what, raw)?;
        Ok(SeatBody::Panel { members, aggregate })
    } else if has_sequence {
        let steps = parse_sequence(dir, what, raw, agents, hands, compile)?;
        refuse_class_without_a_driver(what, raw)?;
        Ok(SeatBody::Sequence { steps })
    } else {
        enforce_model_policy(what, raw, &[], secrets, agents)?;
        record_hands(what, raw, None, secrets, hands)?;
        let body = SeatBody::Single {
            role_path: parse_role(dir, what, raw)?,
            command: parse_command(dir, what, raw, secrets)?,
            confine: parse_confine(what, raw)?,
            candidates: Vec::new(),
        };
        Ok(body)
    }
}

struct SelectCompile<'a> {
    results: &'a [String],
    secrets: &'a [String],
    roots: &'a [PathBuf],
    case_origin: &'a BTreeMap<String, usize>,
    dialect: Option<&'a Dialect>,
}

fn parse_select(
    dir: &Path,
    phase: &str,
    raw: &Value,
    agents: &mut Option<AgentContext>,
    hands: &mut BTreeMap<String, HandsSpec>,
    compile: SelectCompile<'_>,
) -> Result<SeatBody, CompileError> {
    let select = raw
        .get("select")
        .and_then(Value::as_object)
        .ok_or_else(|| CompileError::Invalid(format!("seat '{phase}' select must be an object")))?;
    for key in select.keys() {
        if !["on", "cases", "default"].contains(&key.as_str()) {
            return Err(CompileError::Invalid(format!(
                "seat '{phase}' select has unknown key '{key}'; known: on, cases, default"
            )));
        }
    }
    match select.get("on").and_then(Value::as_str) {
        Some("strategy") => {}
        other => return Err(CompileError::Invalid(format!(
            "seat '{phase}' select has unknown 'on' {}; known: strategy — the selection vocabulary is closed",
            other.unwrap_or("<missing>")
        ))),
    }
    let cases_raw = select
        .get("cases")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{phase}' select.cases must be an object"))
        })?;
    for case in cases_raw.keys() {
        if !SELECT_STRATEGIES.contains(&case.as_str()) {
            return Err(CompileError::Invalid(format!(
                "seat '{phase}' select has unknown strategy case '{case}'; known: {}",
                SELECT_STRATEGIES.join(", ")
            )));
        }
    }
    let has_default = select.get("default").is_some();
    for strategy in SELECT_STRATEGIES {
        if !cases_raw.contains_key(strategy) && !has_default {
            return Err(CompileError::Invalid(format!(
                "strategy class '{strategy}' resolves to nothing in seat '{phase}'; add that case or a default"
            )));
        }
    }
    let mut cases = BTreeMap::new();
    let mut case_gates = BTreeMap::new();
    for (case, body) in cases_raw {
        let label = format!("{phase}:{case}");
        case_gates.insert(case.clone(), is_gate_class(body));
        let case_dir = &compile.roots[compile.case_origin.get(&label).copied().unwrap_or(0)];
        cases.insert(
            case.clone(),
            parse_selected_body(
                case_dir,
                &label,
                body,
                agents,
                hands,
                BodyCompile {
                    results: compile.results,
                    secrets: compile.secrets,
                    dialect: compile.dialect,
                },
            )?,
        );
    }
    let default = select
        .get("default")
        .map(|body| {
            parse_selected_body(
                dir,
                &format!("{phase}:default"),
                body,
                agents,
                hands,
                BodyCompile {
                    results: compile.results,
                    secrets: compile.secrets,
                    dialect: compile.dialect,
                },
            )
            .map(Box::new)
        })
        .transpose()?;
    let default_gate = select.get("default").is_some_and(is_gate_class);
    Ok(SeatBody::Select {
        cases,
        default,
        case_gates,
        default_gate,
    })
}

fn parse_panel(
    dir: &Path,
    what: &str,
    raw: &Value,
    declared_results: &[String],
    secrets: &[String],
    agents: &mut Option<AgentContext>,
    hands: &mut BTreeMap<String, HandsSpec>,
) -> Result<(Vec<PanelMember>, Aggregate), CompileError> {
    let members_raw = raw
        .get("panel")
        .and_then(Value::as_object)
        .ok_or_else(|| CompileError::Invalid(format!("seat '{what}' panel must be an object")))?;
    if members_raw.len() < 2 {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' panel needs at least two members; \
             a one-member panel is a single seat"
        )));
    }
    let gate_members: Vec<&str> = members_raw
        .iter()
        .filter(|(_, member)| member.get("class").and_then(Value::as_str) == Some("gate"))
        .map(|(name, _)| name.as_str())
        .collect();
    let work_members: Vec<&str> = members_raw
        .iter()
        .filter(|(_, member)| member.get("class").and_then(Value::as_str) != Some("gate"))
        .map(|(name, _)| name.as_str())
        .collect();
    if !gate_members.is_empty() && !work_members.is_empty() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' is a mixed panel: gate members [{}], work members [{}]; \
             a panel may judge or work, never do both",
            gate_members.join(", "),
            work_members.join(", ")
        )));
    }
    let aggregate_name = raw
        .get("aggregate")
        .and_then(Value::as_str)
        .ok_or_else(|| CompileError::Invalid(format!("seat '{what}' panel needs 'aggregate'")))?;
    let aggregate = Aggregate::parse(aggregate_name).ok_or_else(|| {
        CompileError::Invalid(format!(
            "seat '{what}' unknown aggregate '{aggregate_name}'; known: \
             unanimous-pass, review-panel"
        ))
    })?;
    for required in aggregate.required_results() {
        if !declared_results.iter().any(|r| r == required) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' aggregate '{aggregate_name}' can emit \
                 '{required}' but the seat does not declare it"
            )));
        }
    }
    let mut members = Vec::with_capacity(members_raw.len());
    for (name, member_raw) in members_raw {
        let site = format!("{what}:{name}");
        if member_raw.get("agent").is_some() {
            refuse_amendments(&site, member_raw)?;
        }
        refuse_unknown_keys(&site, member_raw, MEMBER_KEYS)?;
        let (role_path, command, candidates, agent_hands) = match member_raw.get("agent") {
            None => (
                parse_role(dir, &site, member_raw)?,
                parse_command(dir, &site, member_raw, secrets)?,
                Vec::new(),
                None,
            ),
            Some(_) => {
                let resolved = resolve_reference(
                    agents,
                    dir,
                    &site,
                    &site,
                    member_raw,
                    secrets,
                    Site::Member,
                )?;
                (
                    resolved.role_path,
                    resolved.command,
                    resolved.candidates,
                    resolved.hands,
                )
            }
        };
        enforce_model_policy(&site, member_raw, &candidates, secrets, agents)?;
        record_hands(&site, member_raw, agent_hands, secrets, hands)?;
        members.push(PanelMember {
            name: name.clone(),
            role_path,
            command,
            confine: parse_confine(&site, member_raw)?,
            candidates,
        });
    }
    Ok((members, aggregate))
}

/// Parse a sequence body: named steps, each a single driver or a panel,
/// run serially inside one effect. At least two steps (a one-step
/// sequence is a single seat); names unique case-insensitively.
fn parse_sequence(
    dir: &Path,
    phase: &str,
    raw: &Value,
    agents: &mut Option<AgentContext>,
    hands: &mut BTreeMap<String, HandsSpec>,
    compile: BodyCompile<'_>,
) -> Result<Vec<SequenceStep>, CompileError> {
    let BodyCompile {
        results,
        secrets,
        dialect,
    } = compile;
    let steps_raw = raw
        .get("sequence")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{phase}' sequence must be an array"))
        })?;
    if steps_raw.len() < 2 {
        return Err(CompileError::Invalid(format!(
            "seat '{phase}' sequence needs at least two steps; \
             a one-step sequence is a single seat"
        )));
    }
    let mut steps: Vec<SequenceStep> = Vec::with_capacity(steps_raw.len());
    for (index, step_raw) in steps_raw.iter().enumerate() {
        let name = step_raw
            .get("name")
            .and_then(Value::as_str)
            .filter(|n| !n.is_empty())
            .ok_or_else(|| {
                CompileError::Invalid(format!(
                    "seat '{phase}' sequence step {index} needs a non-empty 'name'"
                ))
            })?;
        if steps.iter().any(|s| s.name.eq_ignore_ascii_case(name)) {
            return Err(CompileError::Invalid(format!(
                "seat '{phase}' sequence has duplicate step name '{name}' \
                 (step names are case-insensitive)"
            )));
        }
        let what = format!("{phase}:{name}");
        let has_agent = step_raw.get("agent").is_some();
        if has_agent {
            refuse_amendments(&what, step_raw)?;
        }
        let has_single =
            !has_agent && (step_raw.get("role").is_some() || step_raw.get("driver").is_some());
        let has_panel = step_raw.get("panel").is_some();
        let has_dialect = step_raw.get("dialect").is_some();
        if [has_single, has_panel, has_agent, has_dialect]
            .iter()
            .filter(|f| **f)
            .count()
            > 1
        {
            return Err(CompileError::Invalid(format!(
                "sequence step '{what}' must be exactly one of role+driver, agent, \
                 panel, or dialect"
            )));
        }
        refuse_unknown_keys(&what, step_raw, STEP_KEYS)?;
        let final_step = index + 1 == steps_raw.len();
        if final_step && step_raw.get("results").is_some() {
            return Err(CompileError::Invalid(format!(
                "sequence step '{what}' is final and receives the seat's results; remove its ignored 'results'"
            )));
        }
        let mut step_results = if final_step {
            results.to_vec()
        } else if step_raw.get("results").is_some() {
            step_raw
                .get("results")
                .and_then(Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|values| !values.is_empty())
                .ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "sequence step '{what}' needs its own non-empty 'results' vocabulary"
                    ))
                })?
        } else if let Some(aggregate) = step_raw
            .get("aggregate")
            .and_then(Value::as_str)
            .and_then(Aggregate::parse)
        {
            aggregate
                .required_results()
                .iter()
                .map(|result| (*result).to_string())
                .collect()
        } else {
            return Err(CompileError::Invalid(format!(
                "sequence step '{what}' needs its own non-empty 'results' vocabulary"
            )));
        };
        let mut agent_hands = None;
        let body = if has_dialect {
            let operation = step_raw
                .get("dialect")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "sequence step '{what}' dialect must be 'validate' or 'check'"
                    ))
                })?;
            let expected = if ["clarify", "analyze"].contains(&phase) {
                "check"
            } else {
                "validate"
            };
            if operation != expected {
                return Err(CompileError::Invalid(format!(
                    "sequence step '{what}' uses dialect operation '{operation}'; phase '{phase}' needs '{expected}'"
                )));
            }
            let dialect = dialect.ok_or_else(|| {
                CompileError::Invalid(format!(
                    "phase '{phase}' needs a realm dialect to resolve step '{name}'"
                ))
            })?;
            let Some(command) = dialect.validation(phase) else {
                if operation == "check" {
                    continue;
                }
                return Err(CompileError::Invalid(format!(
                    "dialect '{}' declares phase '{phase}' {operation} unsupported",
                    dialect.name
                )));
            };
            let synthetic = json!({
                "class": "gate",
                "hands": "workspace",
                "driver": {"command": ["{brokkr}", "driver", "exec", "--"]}
            });
            enforce_model_policy(&what, &synthetic, &[], secrets, agents)?;
            record_hands(&what, &synthetic, None, secrets, hands)?;
            StepBody::Dialect {
                execution: DialectExecution {
                    argv: command.argv.clone(),
                    state: command.state.clone(),
                },
            }
        } else if has_agent {
            let resolved =
                resolve_reference(agents, dir, &what, &what, step_raw, secrets, Site::Member)?;
            agent_hands = resolved.hands;
            StepBody::Single {
                role_path: resolved.role_path,
                command: resolved.command,
                confine: parse_confine(&what, step_raw)?,
                candidates: resolved.candidates,
            }
        } else if has_panel {
            let (members, aggregate) =
                parse_panel(dir, &what, step_raw, &step_results, secrets, agents, hands)?;
            StepBody::Panel { members, aggregate }
        } else {
            StepBody::Single {
                role_path: parse_role(dir, &what, step_raw)?,
                command: parse_command(dir, &what, step_raw, secrets)?,
                confine: parse_confine(&what, step_raw)?,
                candidates: Vec::new(),
            }
        };
        // A final dialect step emits the deterministic adapter's two
        // outcomes, not every result the enclosing seat accepts. Keeping
        // that actual vocabulary on the compiled step lets the engine see
        // an earlier author result which no remaining step can produce
        // (notably `upstream`) and end the sequence at that boundary.
        if has_dialect {
            step_results = dialect_results(phase)
                .into_iter()
                .map(str::to_string)
                .collect();
        }
        match &body {
            StepBody::Single { candidates, .. } => {
                enforce_model_policy(&what, step_raw, candidates, secrets, agents)?;
                record_hands(&what, step_raw, agent_hands, secrets, hands)?;
            }
            StepBody::Panel { .. } => refuse_class_without_a_driver(&what, step_raw)?,
            StepBody::Dialect { .. } => {}
        }
        steps.push(SequenceStep {
            name: name.to_string(),
            class: if has_dialect || is_gate_class(step_raw) {
                SeatClass::Gate
            } else {
                SeatClass::Work
            },
            results: step_results,
            body,
        });
    }
    Ok(steps)
}

fn parse_role(dir: &Path, what: &str, raw: &Value) -> Result<PathBuf, CompileError> {
    let Some(role_rel) = raw.get("role").and_then(Value::as_str) else {
        if raw
            .pointer("/driver/command")
            .and_then(Value::as_array)
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .as_deref()
            .and_then(dispatch_driver)
            .as_deref()
            == Some("exec")
        {
            // A deterministic exec site is the script it names. It has no
            // model to instruct and therefore no charter to load into a
            // prompt; the prompt remains the typed run context and result
            // contract the script reads.
            return Ok(PathBuf::new());
        }
        return Err(CompileError::Invalid(format!(
            "seat '{what}' missing 'role'"
        )));
    };
    let role_path = dir.join(role_rel);
    if !role_path.is_file() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' role file '{role_rel}' does not exist"
        )));
    }
    Ok(role_path)
}

/// Parse a seat's declared secret bindings (decision 0012): NAMES only,
/// grammar plus denylist validated — exactly parallel to the 0007
/// `inputs` declaration. Values never appear anywhere a bundle can reach.
fn parse_secrets(phase: &str, raw: &Value) -> Result<Vec<String>, CompileError> {
    let Some(declared) = raw.get("secrets") else {
        return Ok(Vec::new());
    };
    let declared = declared.as_array().ok_or_else(|| {
        CompileError::Invalid(format!(
            "seat '{phase}' secrets must be an array of strings"
        ))
    })?;
    let mut names: Vec<String> = Vec::with_capacity(declared.len());
    for item in declared {
        let name = item.as_str().ok_or_else(|| {
            CompileError::Invalid(format!(
                "seat '{phase}' secrets must be an array of strings"
            ))
        })?;
        brokkr_protocol::secret::validate_name(name)
            .map_err(|e| CompileError::Invalid(format!("seat '{phase}': {e}")))?;
        if names.iter().any(|n| n == name) {
            return Err(CompileError::Invalid(format!(
                "seat '{phase}' declares secret '{name}' twice"
            )));
        }
        names.push(name.to_string());
    }
    Ok(names)
}

/// A site's command tokens as WRITTEN, before `{brokkr}` and `./` are
/// expanded. Two readers share it: the one that expands it into argv,
/// and decision 0021's refusals, which need the dispatch shape the
/// expansion erases (`{brokkr}` becomes a machine-local absolute path).
fn command_parts(raw: &Value) -> Vec<String> {
    raw.get("driver")
        .and_then(|d| d.get("command"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn parse_command(
    dir: &Path,
    what: &str,
    raw: &Value,
    secrets: &[String],
) -> Result<Vec<String>, CompileError> {
    let parts = command_parts(raw);
    if parts.is_empty() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' needs driver.command"
        )));
    }
    lint_secret_refs(what, &parts, secrets)?;
    Ok(expand_command(dir, &parts))
}

/// Constitutional lint (decision 0012): every `{{secret:` occurrence in
/// the raw template must be a well-formed, DECLARED reference —
/// referenced ⇒ declared, and typos fail closed here rather than riding
/// into argv as literal text. Declared-but-unreferenced is legal:
/// env-only consumers (gh reading GH_TOKEN) take no argv reference at
/// all. An adapter-composed argv faces exactly this lint, against the
/// REFERENCING seat's declared secrets.
fn lint_secret_refs(what: &str, parts: &[String], secrets: &[String]) -> Result<(), CompileError> {
    for part in parts {
        let refs = brokkr_protocol::secret::scan_secret_refs(part)
            .map_err(|e| CompileError::Invalid(format!("seat '{what}' command template: {e}")))?;
        for name in refs {
            if !secrets.contains(&name) {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' command template references undeclared secret \
                     '{name}'; declare it in the seat's 'secrets' list \
                     (undeclared names never resolve)"
                )));
            }
        }
    }
    Ok(())
}

/// `{brokkr}` is this engine's own executable (built-in drivers) and
/// `./`-prefixed entries are bundle-relative. Composed argv is expanded
/// by this same function, which is why a resolved seat's command is an
/// inline seat's command by construction — and why the manifest record
/// carries names, never argv: the expansion is machine-local.
///
/// Public because a seat composed OUTSIDE a bundle — Muninn's, under
/// decision 0020 — must expand the same tokens from the same code
/// rather than from a second copy of this rule.
///
/// `{forge}` is the same token under its old name (decision 0019): it
/// expands to the same path for one more release, and says so once.
pub fn expand_command(dir: &Path, parts: &[String]) -> Vec<String> {
    parts
        .iter()
        .map(|part| {
            if part == "{brokkr}" {
                return brokkr_executable(std::env::current_exe());
            }
            if part == "{forge}" {
                brokkr_protocol::legacy::say_once("{forge}", "{brokkr}");
                return brokkr_executable(std::env::current_exe());
            }
            match part.strip_prefix("./") {
                Some(rel) => dir.join(rel).to_string_lossy().into_owned(),
                None => part.clone(),
            }
        })
        .collect()
}

fn brokkr_executable(current: std::io::Result<PathBuf>) -> String {
    match current {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => "brokkr".to_string(),
    }
}

fn parse_confine(what: &str, raw: &Value) -> Result<Option<Confine>, CompileError> {
    let Some(raw_confine) = raw.get("driver").and_then(|d| d.get("confine")) else {
        return Ok(None);
    };
    let object = raw_confine.as_object().ok_or_else(|| {
        CompileError::Invalid(format!("seat '{what}' driver.confine must be an object"))
    })?;
    let image = object
        .get("image")
        .and_then(Value::as_str)
        .filter(|i| !i.is_empty())
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' confine needs a non-empty image"))
        })?
        .to_string();
    let network = object
        .get("network")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mounts = object
        .get("mounts")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    for key in object.keys() {
        if !["image", "network", "mounts"].contains(&key.as_str()) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' confine has unknown key '{key}'"
            )));
        }
    }
    Ok(Some(Confine {
        image,
        network,
        mounts,
    }))
}

fn declarable_input(name: &str) -> bool {
    !is_engine_owned(name)
        && (BOOLEAN_INPUTS.contains(&name)
            || SEVERITY_INPUTS.contains(&name)
            || ENUM_INPUTS.contains(&name)
            || IDENTIFIER_INPUTS.contains(&name))
}

/// The non-engine-owned inputs the phase's rules reference: the default
/// (and minimum) seat declaration.
fn referenced_seat_inputs(table: &Value, phase: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let Some(rules) = table.get("rules").and_then(Value::as_array) else {
        return names;
    };
    for rule in rules {
        if rule.get("from").and_then(Value::as_str) != Some(phase) {
            continue;
        }
        let Some(when) = rule.get("when").and_then(Value::as_object) else {
            continue;
        };
        for key in when.keys() {
            let name = if key == "strategy_in" {
                "strategy"
            } else if key == "drift_in" {
                "drift_in"
            } else {
                key.strip_suffix("_gte")
                    .or_else(|| key.strip_suffix("_above"))
                    .or_else(|| key.strip_suffix("_at_most"))
                    .or_else(|| key.strip_suffix("_in"))
                    .unwrap_or(key)
            }
            .to_string();
            if !is_engine_owned(&name) && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.sort();
    names
}

/// The pinned, content-addressed identity of a bundle. `chain` is the
/// resolved composition (decision 0017): each ancestor rides in `files`
/// under the reserved `@compose/` prefix, zero-padded so canonical key
/// sorting preserves chain order, so changing a base moves the digest of
/// everything derived from it — and so the chain survives the dispatch
/// manifest round-trip, which copies `files` verbatim. `agents` is the
/// resolution record (decision 0016), pinned for the same reason.
fn manifest_for(
    dir: &Path,
    bundle_name: &str,
    chain: &[Ancestor],
    agents: Option<&Map<String, Value>>,
    drivers: Option<&Map<String, Value>>,
    hands: &BTreeMap<String, HandsSpec>,
    select: &Map<String, Value>,
) -> Result<Value, CompileError> {
    let mut files = Map::new();
    for (index, ancestor) in chain.iter().enumerate() {
        // A base's directory may legitimately differ from its declared
        // name. Recording only one lets a directory answer to a name it
        // does not declare, in an append-only manifest; record both.
        let label = match &ancestor.reached_as {
            Some(reached) if *reached != ancestor.name => {
                format!("{}@{reached}", ancestor.name)
            }
            _ => ancestor.name.clone(),
        };
        files.insert(
            format!("{COMPOSE_PREFIX}{index:04}/{label}"),
            Value::String(ancestor.digest.clone()),
        );
    }
    let mut stack = vec![dir.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                // A secrets store inside the bundle would ride the
                // manifest digest: rotation would change the digest AND
                // the manifest would embed a SHA-256 of the secret file —
                // an offline-guessing oracle (decision 0012, layer 2).
                if path.file_name().and_then(|n| n.to_str()) == Some("secrets.env") {
                    return Err(CompileError::Invalid(format!(
                        "bundle contains a secrets store '{}'; the store must \
                         live outside the bundle dir (e.g. .forge/secrets.env) \
                         so digests carry names only",
                        path.display()
                    )));
                }
                paths.push(path);
            }
        }
    }
    paths.sort();
    for path in paths {
        // Manifest keys are bundle identity: platform-independent by
        // construction (windows separators would fork the digest).
        let rel = path
            .strip_prefix(dir)
            .expect("walked under dir")
            .to_string_lossy()
            .replace('\\', "/");
        // The composition namespace is computed, never read from disk:
        // a real file under it could otherwise forge provenance.
        if rel.starts_with(COMPOSE_PREFIX) {
            return Err(CompileError::Invalid(format!(
                "bundle file '{rel}' uses the reserved '{COMPOSE_PREFIX}' namespace; \
                 composition provenance is computed by the resolver, never \
                 supplied by a bundle"
            )));
        }
        files.insert(rel, Value::String(sha256_bytes(&std::fs::read(&path)?)));
    }
    let mut manifest = json!({
        "engine": ENGINE_VERSION,
        "event_schema": EVENT_SCHEMA,
        "database_schema": brokkr_store::DATABASE_SCHEMA,
        "driver_protocol": DRIVER_PROTOCOL,
        "bundle_name": bundle_name,
        "files": Value::Object(files),
    });
    // ABSENT when no seat references an agent (the decision-0012
    // `if !seat.secrets.is_empty()` precedent, applied verbatim): a
    // non-adopting bundle's manifest is byte-identical to what it was.
    // This key is also the pin that replaces the `manifest.files` entry a
    // charter loses when it leaves the recipe directory — it carries the
    // charter digest.
    if let Some(records) = agents.filter(|records| !records.is_empty()) {
        manifest["agents"] = Value::Object(records.clone());
    }
    // ABSENT on the same terms, and for the same reason: a bundle whose
    // inline seats neither judge nor bind never consults a tier or a
    // grant, so nothing authorised it and there is nothing to pin. Where
    // something did (decision 0021), the adapter digest that answered
    // rides the bundle's identity — a tier demoted in `adapters/` moves
    // the digest of every bundle whose gates it was standing behind.
    if let Some(records) = drivers.filter(|records| !records.is_empty()) {
        manifest["drivers"] = Value::Object(records.clone());
    }
    // ABSENT on the same terms once more (decision 0043): a bundle that
    // boxes no hands keeps its v5 shape and identity byte for byte.
    if !hands.is_empty() {
        manifest["hands"] = Value::Object(
            hands
                .iter()
                .map(|(site, spec)| (site.clone(), spec.to_value()))
                .collect(),
        );
    }
    // Absent for every non-selecting bundle: v6 identity is preserved.
    if !select.is_empty() {
        manifest["select"] = Value::Object(select.clone());
    }
    Ok(manifest)
}

#[cfg(test)]
mod agent_tests;
#[cfg(test)]
mod compose_tests;

#[cfg(test)]
mod model_policy_tests;

#[cfg(test)]
mod secret_binding_tests;

#[cfg(test)]
mod tests;
