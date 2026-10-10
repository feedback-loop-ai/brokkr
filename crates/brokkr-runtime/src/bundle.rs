//! Declarative bundles: policy + one seat per phase (decision 0005).
//! `compile` validates before anything runs and produces the pinned
//! content-addressed manifest. Rejections here are the executable slice
//! of the constitutional lint: a bundle that could reach ship around the
//! protected review phase, name a result no rule covers, or reference a
//! missing role never loads at all.

use brokkr_protocol::hands::HandsSpec;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::policy::{Machine, BOOLEAN_INPUTS, IDENTIFIER_INPUTS, SEVERITY_INPUTS};
use brokkr_core::realms::Boundary;
use serde_json::{json, Map, Value};
use thiserror::Error;

mod charters;
pub mod compose;
mod mcp;
mod tier;

use charters::parse_role;
use compose::{Ancestor, COMPOSE_PREFIX};
pub(crate) use mcp::McpIntent;
pub use tier::{ProvisionalRefusal, RealmLaw};

use crate::agents::{
    resolve_route, route_is_effortless, Adapter, Adapters, Availability, Candidate, Composition,
    EgressClass, HandsNotice, Library, Lowering, Sandbox, TrustTier,
};
use crate::dialect::{Dialect, DIALECT_PHASES};
use brokkr_protocol::native_controls::{Origin, Segment, TemplateExpectation};

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DRIVER_PROTOCOL: u32 = 1;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("bundle: {0}")]
    Invalid(String),
    /// A refusal by capability authority (decision 0065): a grant, an
    /// abstract definition, a tool dialect or a seat's resolution. It
    /// reads exactly as `Invalid` does; it is a variant of its own so that
    /// `brokkr resume` can tell "the pinned authority cannot be
    /// reproduced here" from any other compile failure and refuse through
    /// the manifest-mismatch door with capabilities named (design D7).
    /// The whole line, `bundle: ` and any composition-chain note included,
    /// renders through the protocol's one refusal sink: one line, at most
    /// 512 scalar values (rebuild unit 12-fix-e; design D6).
    #[error("{}", capability_line(.0))]
    Capability(String),
    #[error("bundle io: {0}")]
    Io(#[from] std::io::Error),
    #[error("bundle json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bundle policy: {0}")]
    Policy(#[from] brokkr_core::PolicyError),
    #[error("bundle: {0}")]
    Provisional(ProvisionalRefusal),
    /// GP2 (decision 0065 slice two, U3c): an inline site's verified
    /// charter names an ask it writes in no DATA paragraph.
    #[error("bundle: {0}")]
    Charter(crate::agents::charter_data::CharterRefusal),
}

/// How [`CompileError::Capability`] renders.
fn capability_line(reason: &str) -> String {
    brokkr_protocol::native_controls::bounded_line(&format!("bundle: {reason}"))
}

/// The engine-owned inputs live with the policy that reads them, because
/// the loader's presence refusal (decision 0050, ruling 1) exempts them.
pub use brokkr_core::policy::{is_engine_owned, ENGINE_OWNED_INPUTS, REALM_FACTS};

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

#[derive(Debug, Clone)]
pub struct Seat {
    /// True when every invocation site in this seat is a gate. The runtime
    /// uses the compiled fact to enforce a stable repository HEAD around the
    /// whole effect — but only where the effect has no inner spans of its
    /// own. A sequence, all-gate or mixed, is watched per step instead: a
    /// step carries its own class, and that step is the gate.
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

/// One agent session, a parallel panel joined by a declared
/// deterministic rule, or a serial sequence of named steps (decision
/// 0002's sanctioned forms: composition INSIDE the executor — one
/// effect, one typed result at the boundary; inner structure is
/// journaled as checkpoint evidence).
///
/// Decision 0008's `driver.confine` no longer rides here: decision 0046
/// ruling 5 retired it into the `container` boundary the realm declares,
/// and the field is refused by name until slice (iii) measures that
/// boundary (see [`refuse_confine`]).
#[derive(Debug, Clone)]
pub enum SeatBody {
    Single {
        role_path: PathBuf,
        command: Vec<String>,
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
                candidates,
            } => ExecutableBody::Single {
                role_path,
                command,
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

/// A sequence step's canonical class, read once for the step and for its
/// capability record (GP1): a dialect step runs the realm dialect's own
/// validator, which judges, so it is a gate though it writes no class.
fn step_class(step: &Value) -> SeatClass {
    match step.get("dialect").is_some() || is_gate_class(step) {
        true => SeatClass::Gate,
        false => SeatClass::Work,
    }
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

/// The confinement state of one execution site's hands (design D10 F1).
///
/// `Unknown` is not `none`: a site the compiler registered but never
/// resolved must never read as an affirmative no-hands fact, because the
/// adapter gate spends that absence as permission.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum HandsState {
    /// Registered, but no declaration established whether this site has
    /// hands. Never reads as `none`.
    #[default]
    Unknown,
    /// The site's own declaration was read successfully and establishes
    /// no hands.
    NoHands,
    /// The site's declaration (or resolved agent) carries these hands.
    Hands(HandsSpec),
}

/// One site's pinned driver digests, keyed by provider: the shape both
/// decision 0021's witness and decision 0035's effortless-listing witness
/// carry, merged into the manifest's `drivers` projection.
pub type DriverDigests = Map<String, Value>;

/// Every execution-site-owned fact of one compiled label (design D10 F1):
/// the resume assessment, the pinned driver digests, the hands state, the
/// agent resolution record and the inline driver evidence. One value, so
/// the whole family relocates at once when a dialect wrapper changes the
/// executing coordinate — a sixth field follows the value rather than
/// needing a fifth remembered move.
#[derive(Debug, Clone, Default)]
pub struct SiteFacts {
    pub inline_resume: Option<Value>,
    pub pin_drivers: Option<DriverDigests>,
    pub hands: HandsState,
    pub record: Option<Value>,
    pub driver: Option<DriverDigests>,
    /// The resolved chain of an agent-backed site, kept beside its record
    /// so the capability pass judges exactly the candidates the site will
    /// run (decision 0065). Empty for an inline site.
    pub chain: Vec<Candidate>,
    /// Decision 0065 ruling 5: what this site asks for and, per provider
    /// candidate, what it holds. `None` only until the capability pass has
    /// run; the engine refuses to launch a model site that still has none.
    pub capabilities: Option<crate::capabilities::SiteCapabilities>,
    /// Second council H6: the charter this site's AGENT was resolved with,
    /// bound to the pin its library record carries. An agent's charter
    /// stands outside every layer's file map, so nothing at the dispatch
    /// door used to compare it — `charter_drift` answered `None` and the
    /// launch went ahead on whatever the file said by then. The binding is
    /// carried outside the manifest, because the path is the host's and
    /// bundle identity is not.
    /// Rebuild unit 17: every site with a charter carries it, an inline
    /// site's bound to the layer that declared its role, so its owner is
    /// selected where the site is compiled and never guessed from a path.
    /// `None` at an exec site, which has no charter.
    pub charter: Option<CharterPin>,
    /// Decision 0065 slice one (design D5.2): the EFFECTIVE typed local
    /// declaration of this executable site — the office's narrowed by the
    /// site's, or the inline site's own. `None` is a site the local pass
    /// never visited; `Some` with both fields unspecified is a visited site
    /// that declared nothing. Containers never own one. Private compile
    /// data: not a manifest field and not a grant.
    pub local: Option<crate::agents::LocalTools>,
    /// Rebuild unit 5b (design D5.3, D5.7): an inline Claude or LaneTally
    /// site's typed allow, lowered by the engine onto its adapter's tool
    /// permissions. The engine appends it behind the authored command as
    /// its own `local` segment at dispatch; the authored command never
    /// carries it. `None` at every other site.
    pub inline_local: Option<crate::agents::LocalLowering>,
    /// Rebuild unit 5c (operator ruling of 2026-09-24): the permission
    /// template the site's adapter declares behind its driver verb, taken
    /// where `inline_local` is and nowhere else. The engine appends it as
    /// its own `template` segment between the authored command and the
    /// lowered list. `None` where the allow does not lower or the adapter
    /// declares no template.
    pub inline_template: Option<Segment>,
    /// Rebuild unit 5c-fix (operator ruling of 2026-09-24, item 2): the
    /// adapter's declaration of that template as a typed fact, separate
    /// from the segment above, which is what is emitted. The engine fills
    /// the expected state from this fact alone, so a template omitted or
    /// altered on its way into the command has something to contradict.
    /// Recorded exactly where `inline_local` is — `Declared` with the
    /// expanded argv, or `None` for an adapter that declares no template —
    /// and `None` (unrecorded) at every other site.
    pub declared_template: Option<TemplateExpectation>,
    /// Rebuild unit 5d (operator ruling of 2026-09-25, "narrow"): an inline
    /// Codex seat's typed sandbox class, lowered onto the fragment its
    /// adapter declares for the seat's class. The engine appends it behind
    /// the authored command as its own `local` segment at dispatch, as it
    /// appends `inline_local`. `None` at every other site.
    pub inline_sandbox: Option<InlineSandbox>,
    /// Rebuild unit 14a1: the adapter's declared dialect an inline site's
    /// command is composed from, recorded where its typed declaration was
    /// lowered — the permission flag its allow lowered onto, and the
    /// fragment its class lowered onto as declared, tokens unexpanded, and
    /// (operator ruling (B) of 2026-09-27) where the site has hands, the
    /// `hands.workspace` fragment its driver's adapter declares.
    /// `Some` at every inline site [`record_inline_tools`] visited, `None`
    /// at every other; read through [`SiteFacts::inline_serving`].
    pub inline_dialect: Option<crate::agents::DeclaredDialect>,
    /// Rebuild unit 14a4a (operator ruling (B) of 2026-09-27): an inline
    /// site's hands, served like an agent's — the `hands.workspace` fragment
    /// its dialect carries, through the compile's expansion as an agent's
    /// `hands` segment is. The engine appends it behind every other segment
    /// of the site's command at dispatch, and the box expands its tokens.
    /// `None` at a site whose dialect carries no hands fragment.
    pub inline_hands: Option<Segment>,
    /// The discovery notice the adapter an INLINE built-in model driver
    /// names declares (decision 0069). An agent-resolved site
    /// carries its notice on each `Candidate` instead, and the engine
    /// reads this only when no candidate serves the site. The adapter it
    /// was read from is witnessed through `pin_drivers`.
    pub inline_hands_notice: Option<HandsNotice>,
    /// Decision 0065 slice two, U1f (SI2, MB1): the MCP server set an INLINE
    /// site's serving intends, from its resolved hands and dispatched driver,
    /// never its bytes; a dialect step, served by exec, records NoModelSurface.
    /// Agent-backed candidates carry theirs in their own composition; `None`
    /// at those sites, at a panel, sequence or select, and an unsupported check.
    pub(crate) inline_mcp: Option<McpIntent>,
}

/// One inline Codex seat's lowered sandbox (rebuild unit 5d): the class
/// its typed declaration names, the engine's `local` segment expressing
/// it — the adapter's `hands.harness` fragment for the seat's class, whose
/// `{result_path}` the engine fills at dispatch — and the result door that
/// fragment opens: `last-message` at a gate, `file` at a work seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSandbox {
    pub class: Sandbox,
    pub segment: Segment,
    pub door: crate::agents::ResultDoor,
}

/// Every charter one compile bound, keyed by the path the seat will be told
/// from, with every binding a site selected for that path. Held on the
/// [`Bundle`] rather than only per site, so the pin survives every
/// projection of the site facts (second council H6; rebuild unit 17).
pub type CharterPins = BTreeMap<PathBuf, BTreeSet<CharterPin>>;

/// One charter as the compile bound it (second council H6; rebuild unit 17,
/// design D7): the owner selected with the site, the reference as written,
/// the path the seat is told, and the digest the owner already pins for
/// those bytes. Rebuild unit 18-fix-b (council F1, F2): with the [`Binding`]
/// the compile's bound read verified — both keys and the file it read — and
/// who the owner's directory was when that same read reached it. A pin is
/// built only from such a read ([`CharterPin::of`]), never from a path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CharterPin {
    pub owner: CharterOwner,
    pub reference: String,
    pub path: PathBuf,
    pub digest: String,
    pub(crate) binding: Binding,
    pub(crate) directory: OwnerIdentity,
}

/// Who pins a charter (rebuild unit 17): the layer that declared an inline
/// role, with the key its file map pins the role under, or the library an
/// agent was loaded from, with its own contained root.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharterOwner {
    Layer { dir: PathBuf, key: String },
    Library { agent: String, root: PathBuf },
}

impl CharterOwner {
    /// The owner's canonical directory: the declaring layer's, or the
    /// library's own.
    pub fn root(&self) -> &PathBuf {
        match self {
            CharterOwner::Layer { dir, .. } => dir,
            CharterOwner::Library { root, .. } => root,
        }
    }
}

impl CharterPin {
    /// The pin of the charter `bound` read from `owner`'s directory by
    /// [`owned_input`]: its digest, its binding and who that read found the
    /// owner's directory to be, all from the one read.
    pub(crate) fn of(
        owner: CharterOwner,
        reference: &str,
        path: PathBuf,
        bound: &BoundInput,
    ) -> CharterPin {
        CharterPin {
            owner,
            reference: reference.to_string(),
            path,
            digest: sha256_bytes(&bound.bytes),
            binding: bound.held.binding.clone(),
            directory: bound
                .held
                .owner
                .clone()
                .expect("a charter is read through its owner's directory"),
        }
    }

    /// The owner and the key a dispatch refusal names. A layer is found by
    /// its exact directory, never by the longest root a path starts with;
    /// `None` where no layer of this bundle is that directory.
    fn named(&self, bundle: &Bundle) -> Option<(String, String)> {
        match &self.owner {
            CharterOwner::Layer { dir, key } => {
                let name = match dir == &bundle.dir {
                    true => &bundle.name,
                    false => {
                        &bundle
                            .chain
                            .iter()
                            .find(|ancestor| &ancestor.dir == dir)?
                            .name
                    }
                };
                Some((format!("layer '{name}'"), key.clone()))
            }
            CharterOwner::Library { agent, .. } => Some((
                format!("agent '{agent}'"),
                self.path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            )),
        }
    }
}

impl SiteFacts {
    /// The hands this site resolved, when it has any. `Unknown` and
    /// `NoHands` both answer `None`; the two are told apart by the
    /// `hands` state itself, never by this answer.
    pub fn hands_spec(&self) -> Option<&HandsSpec> {
        match &self.hands {
            HandsState::Hands(spec) => Some(spec),
            _ => None,
        }
    }

    /// Rebuild unit 14a1: an inline site's typed serving inputs — its
    /// recorded dialect, no pins (the recipe writes its own model and
    /// effort) and the typed hands this site resolved. `None` at a site
    /// with no inline composition, whose candidates carry theirs.
    pub fn inline_serving(&self) -> Option<crate::agents::ServingInputs> {
        Some(crate::agents::ServingInputs {
            dialect: self.inline_dialect.clone()?,
            pins: Vec::new(),
            spec: self.hands_spec().cloned(),
        })
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
    /// composed nothing. The box binds the root that owns an exec site's
    /// script, and the unboxed dispatch checks its script directory at spawn
    /// (decision 0046 ruling 4), so an inherited seat's script is judged
    /// against the layer that wrote it.
    pub roots: Vec<PathBuf>,
    /// The boundary this bundle was compiled under (decision 0046 ruling
    /// 1): the operated realm's word, `namespace` in no realm. Pinned per
    /// hands site in the manifest's `boundary` map, and the one word the
    /// engine composes every site with hands from (design DD3).
    pub boundary: Boundary,
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
    /// The resume assessment each INLINE driver-bearing site's adapter
    /// declares, keyed as `hands` is. An agent-resolved site carries its
    /// assessment on the selected `Candidate`; a raw `driver.command`
    /// does not resolve through the library, so the compiler reads the
    /// adapter it names and carries the closed assessment here for the
    /// engine to place in the driver's private start context (proposed
    /// decision 0056 ruling 5). Absent for a site whose driver no adapter
    /// declares, which the driver reads as unmeasured — never implicit
    /// support.
    pub inline_resume: BTreeMap<String, Value>,
    /// The canonical execution-site family (design D10 F1). Every fact a
    /// dialect wrapper must carry to the executing coordinate lives in
    /// one `SiteFacts` per label, so relocation moves the whole family.
    /// The engine reads confinement and assessment facts from here;
    /// `hands` and `inline_resume` beside it are serialization
    /// projections, never separately mutable authorities.
    pub sites: BTreeMap<String, SiteFacts>,
    /// Second council H6: the library pin of every agent charter this
    /// compile resolved, consulted at the dispatch door for a charter no
    /// layer's file map keys.
    pub charters: CharterPins,
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
    /// Decision 0036 ruling 4: the egress class a seat's resolved route
    /// must MEET before that seat may declare secret bindings. The
    /// operator rules it into the bundle; absence is `Contracted`, which
    /// is exactly what `binding_grant: true` meant, so every bundle on
    /// disk keeps the behaviour it has.
    egress_minimum: EgressClass,
    /// Every library charter's bound read, held until the bundle is sealed
    /// (rebuild unit 18-fix-b return, council F2).
    reads: Vec<LibraryRead>,
    /// The offices a provisional model may hold ([`RealmLaw`]).
    provisional_offices: Vec<String>,
}

/// One library charter a site was bound to, as its read holds it: the site
/// and agent a refusal names, the reference as written, the digest of the
/// buffer read and what the read holds. No layer's walk pins a library's
/// file, so before the bundle is sealed the read itself is checked to still
/// stand — its owner, every entry on its way and its bytes — as a layer's
/// charter is by that layer's seal (rebuild unit 18-fix-b return, F2).
struct LibraryRead {
    site: String,
    agent: String,
    reference: String,
    digest: String,
    held: Held,
}

impl LibraryRead {
    /// Refused, where the read no longer stands as it was read.
    fn check(&self) -> Result<(), CompileError> {
        match self.held.intact(&self.digest) {
            true => Ok(()),
            false => Err(library_refusal(
                &self.site,
                &self.agent,
                &self.reference,
                "which the compile no longer holds as it was read: its library's directory, an \
                 entry on its way, or its bytes changed after the read that bound it",
            )),
        }
    }
}

/// The refusal of the charter `reference` the library's `agent` names for
/// the seat `site`, for the reason `clause`.
fn library_refusal(site: &str, agent: &str, reference: &str, clause: &str) -> CompileError {
    CompileError::Invalid(format!(
        "seat '{site}': agent '{agent}' names charter {}, {clause}. What a seat is told must be \
         what the bundle's identity names, so it is refused (decision 0065 slice one, design D7)",
        bounded_reference(reference)
    ))
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
/// agent at all (`recipes/verify` and `recipes/fast` are exactly that).
/// A bundle with none of the three has nothing to check and still
/// compiles with no `adapters/` directory in sight. A typed `tools`
/// declaration (decision 0065 slice one, design D5.2) is judged against
/// what an adapter can represent, so it opens the adapters too — through
/// this same fallible context, never a swallowed load.
fn needs_adapters(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            map.contains_key("agent")
                || map.contains_key("dialect")
                || map.contains_key("secrets")
                || map.contains_key("tools")
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
pub(crate) fn short_flag(flag: &str) -> bool {
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

/// The model pin as decision 0036 ruling 2 reads it: which route the
/// material goes to, or which kind of silence the argv holds — on one
/// of the two flags decision 0040 ruling 1 has it read. On `--model`,
/// the flag this engine composes for the four model-bearing built-ins,
/// it is also decision 0031 ruling 2's read, which asks only whether one
/// concrete id is stated; a neighbouring `--model…` flag is a different
/// flag to it, not an illegible spelling of this one, because `--model`
/// is long.
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
    matches!(route_pin(raw, MODEL_FLAG), ModelPin::Concrete(_))
}

/// Issue #373: the dsh driver admits one spelling of its model pin, the
/// separate `--model <id>`, and refuses every other word beginning with
/// `--model` before it spawns — the joined `--model=<id>` this compiler
/// reads as a pin (decision 0040 ruling 2) and a long neighbour such as
/// `--model-fallback` that it walks past alike. A seat compile admitted
/// would then park at spawn, after its run and journal exist, so compile
/// refuses what spawn will.
fn dsh_refuses_model_word(raw: &Value) -> bool {
    raw.pointer("/driver/command")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|part| part != MODEL_FLAG && part.starts_with(MODEL_FLAG))
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
    /// Issue #373: dsh sites carrying a `--model…` word the dsh driver
    /// refuses at spawn.
    dsh_model: Vec<String>,
    /// Decision 0035 addendum 2026-09-11: per site, the adapter digest
    /// whose effortless listing exempted it from the effort pin. The
    /// declaration that authorised the exemption rides the bundle's
    /// identity beside decision 0021's — un-listing a route moves the
    /// digest of every bundle it excused.
    witnessed: BTreeMap<String, DriverDigests>,
    /// Proposed decision 0056 ruling 5: per inline driver-bearing site,
    /// the resume assessment the adapter it names declares, carried to
    /// [`Bundle::inline_resume`] for the driver's private start context.
    resume: Map<String, Value>,
    /// The declaration each of those assessments was read from. Kept
    /// apart from `witnessed` because an exemption is a different fact
    /// from a consumed assessment, but merged into the manifest's
    /// `drivers` pin with it: an edit to the resume block must move the
    /// bundle identity `pinned_bundle_holds` compares, or a changed
    /// assessment would reuse a root the old one opened.
    resume_witness: BTreeMap<String, DriverDigests>,
    /// Decision 0069: per inline driver-bearing site, the
    /// discovery notice its adapter declares. The declaration is already
    /// witnessed in `resume_witness`, which pins every adapter an inline
    /// built-in consults, assessment or not.
    hands_notice: BTreeMap<String, HandsNotice>,
}

/// Adapter data for an inline model seat, loaded only where a bundle seats
/// one, and kept as the `Result` the load gave (decision 0066 ruling 1).
///
/// Readers with opposite needs. The effortless-route exemption (decision
/// 0035 addendum 2026-09-11), the inline resume assessment and the inline
/// discovery notice (decision 0069) take the `Ok` and read an absent or
/// unloadable root as none of them, so the strict rule stands. The
/// capability pass is MANDATORY: the same data says how a harness's native
/// search is switched off, and an error swallowed here used to reach it as
/// "nothing declared" and compile a Codex seat with no denial. It takes the
/// error too, and refuses the seat with the loader's own words. A PRESENT
/// root that does not load is refused after that pass in any case
/// (decision 0069): a malformed discovery notice must not pass itself off
/// as an adapter that declares none. `None` is a bundle that seats no
/// inline model driver and so asked for nothing.
fn load_pin_adapters(root: &Path, seats: &Map<String, Value>) -> Option<Result<Adapters, String>> {
    fn has_inline_model_driver(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                built_in_model_driver(value).is_some() || map.values().any(has_inline_model_driver)
            }
            Value::Array(items) => items.iter().any(has_inline_model_driver),
            _ => false,
        }
    }
    if !seats.values().any(has_inline_model_driver) {
        return None;
    }
    Some(Adapters::load(root).map_err(|error| error.to_string()))
}

/// The adapters a capability pass resolves against, and why there are none
/// where a load failed: what [`mcp::candidate_capabilities`] and
/// [`mcp::inline_capabilities`] are handed.
#[derive(Clone, Copy)]
struct CapabilityAdapters<'a> {
    adapters: Option<&'a Adapters>,
    unloaded: Option<&'a str>,
}

impl<'a> CapabilityAdapters<'a> {
    fn loaded(adapters: &'a Adapters) -> Self {
        CapabilityAdapters {
            adapters: Some(adapters),
            unloaded: None,
        }
    }

    /// An inline seat's adapters where no agent context holds them: never
    /// asked for, or failed to load. Inline adapters that loaded open an
    /// agent context (`tier::inline_context`) and are read as `loaded`.
    fn unloaded(pinned: Option<&'a Result<Adapters, String>>) -> Self {
        CapabilityAdapters {
            adapters: None,
            unloaded: pinned
                .and_then(|loaded| loaded.as_ref().err())
                .map(String::as_str),
        }
    }
}

/// Decision 0035 addendum 2026-09-11: a seat whose concrete lane
/// resolves through its adapter to an effortless route needs no effort
/// pin — and the adapter digest that says so is witnessed beside the
/// exemption. Only a readable concrete id with a route prefix can
/// claim it: a bare id keeps the adapter default's standing, and an
/// unreadable pin is refused on the model axis first.
fn effort_exempt(
    what: &str,
    raw: &Value,
    adapters: Option<&Adapters>,
    witnessed: &mut BTreeMap<String, DriverDigests>,
) -> bool {
    let adapters = match adapters {
        Some(adapters) => adapters,
        None => return false,
    };
    let kind = match built_in_model_driver(raw) {
        Some(kind) => kind,
        None => return false,
    };
    let adapter = match adapters.adapter(kind) {
        Some(adapter) => adapter,
        None => return false,
    };
    match inline_route_pin(raw, Some(adapter)) {
        ModelPin::Concrete(id) if route_is_effortless(adapter, &id) => {
            let mut authorised = Map::new();
            authorised.insert(kind.to_string(), Value::String(adapter.digest.clone()));
            witnessed.insert(what.to_string(), authorised);
            true
        }
        _ => false,
    }
}

/// Inspect every driver-bearing invocation site in the already composed
/// bundle. Agent references are pinned by their resolved candidate
/// chains — a chain that names no effort for an effort-bearing provider
/// is refused where the vocabulary is known, in the resolver — while
/// inline built-ins must state both concrete pins in their argv.
fn collect_unpinned(what: &str, raw: &Value, adapters: Option<&Adapters>, out: &mut Unpinned) {
    if raw.get("agent").is_some() {
        return;
    }
    // Both built-in lists are the same list: exec has neither a model to
    // pin nor an effort to pin, and a custom driver owns its own
    // contract. So a site is asked for both pins or for neither, and a
    // seat missing both is named in both halves of one refusal.
    if let Some(kind) = built_in_model_driver(raw) {
        if !command_pins_model(raw) {
            out.model.push(what.to_string());
        }
        if !command_pins_effort(raw) && !effort_exempt(what, raw, adapters, &mut out.witnessed) {
            out.effort.push(what.to_string());
        }
        if kind == "dsh" && dsh_refuses_model_word(raw) {
            out.dsh_model.push(what.to_string());
        }
        // The adapter a built-in model driver names answers for this
        // INLINE site as it does for an agent-resolved one: its measured
        // resume assessment travels to the engine so the driver's gate
        // can judge an offer at this site. Unmeasured stays unmeasured —
        // an adapter with no resume block contributes the null the gate
        // reads as `unsupported-resume` — and a missing or malformed
        // adapters root contributes nothing at all.
        if let Some(adapter) = adapters.and_then(|adapters| adapters.adapter(kind)) {
            out.resume.insert(what.to_string(), adapter.resume.value());
            // The DECLARATION the assessment was read from is pinned
            // beside every other adapter a site consulted: the engine
            // consumes this resume block to decide whether that site
            // rejoins, so an edit to it must move the bundle identity
            // that `pinned_bundle_holds` compares and the instance key
            // the offer is stamped with. Otherwise a changed
            // restrictions assessment would enable a different rejoin
            // under the old, still-eligible root. Recorded beside the
            // exemption map rather than inside it because the two are
            // different facts; both ride the manifest's `drivers` pin.
            let mut authorised = Map::new();
            authorised.insert(kind.to_string(), Value::String(adapter.digest.clone()));
            out.resume_witness.insert(what.to_string(), authorised);
            // Its discovery notice (decision 0069) is read from
            // that same witnessed declaration, and kept apart from the
            // assessment: reading a notice qualifies no resume.
            if let Some(notice) = &adapter.hands_notice {
                out.hands_notice.insert(what.to_string(), notice.clone());
            }
        }
        return;
    }
    if let Some(panel) = raw.get("panel").and_then(Value::as_object) {
        for (member, member_raw) in panel {
            collect_unpinned(&format!("{what}:{member}"), member_raw, adapters, out);
        }
    }
    if let Some(sequence) = raw.get("sequence").and_then(Value::as_array) {
        for (index, step) in sequence.iter().enumerate() {
            let name = step_label(index, step);
            collect_unpinned(&format!("{what}:{name}"), step, adapters, out);
        }
    }
    if let Some(select) = raw.get("select").and_then(Value::as_object) {
        if let Some(cases) = select.get("cases").and_then(Value::as_object) {
            for (case, body) in cases {
                collect_unpinned(&format!("{what}:{case}"), body, adapters, out);
            }
        }
        if let Some(body) = select.get("default") {
            collect_unpinned(&format!("{what}:default"), body, adapters, out);
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

/// What `enforce_model_pins` returns: the adapter digests whose effortless
/// listings exempted inline seats, the resume assessment each inline
/// built-in model driver's adapter declares for the engine, the
/// declaration each of those assessments was read from, and the discovery
/// notice each declares. Four maps rather than one because they answer
/// different questions from the same walk — the third rides the
/// manifest's `drivers` pin beside the first, while the second travels to
/// the driver's private start context and the fourth to the engine's
/// per-attempt notice.
type PinWitness = (
    BTreeMap<String, DriverDigests>,
    Map<String, Value>,
    BTreeMap<String, DriverDigests>,
    BTreeMap<String, HandsNotice>,
);

/// One refusal names the complete repair set, on BOTH axes. A model pin
/// without an effort pin is half a hire (decision 0035 ruling 5), so the
/// two clauses stand beside each other rather than the first hiding the
/// second behind a second compile. Returns the adapter digests whose
/// effortless listings exempted inline seats, for the manifest, beside
/// the resume assessment each inline built-in model driver's adapter
/// declares, for the engine's private start context, beside the digest of
/// the declaration each assessment was read from, for the manifest.
fn enforce_model_pins(
    seats: &Map<String, Value>,
    adapters: Option<&Adapters>,
) -> Result<PinWitness, CompileError> {
    let mut unpinned = Unpinned::default();
    for (phase, raw) in seats {
        collect_unpinned(phase, raw, adapters, &mut unpinned);
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
    if !unpinned.dsh_model.is_empty() {
        refusals.push(format!(
            "dsh seats {} carry a '--model…' word the dsh driver refuses at spawn; \
             write the pin as '--model <concrete-model-id>' and no other '--model…' \
             flag (issue #373)",
            labels(&unpinned.dsh_model)
        ));
    }
    if refusals.is_empty() {
        return Ok((
            unpinned.witnessed,
            unpinned.resume,
            unpinned.resume_witness,
            unpinned.hands_notice,
        ));
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

    /// Compile in no realm: the library's default dialect and the
    /// `namespace` boundary, which is what every bundle meant before
    /// decision 0046 named the word.
    pub fn compile_with(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
    ) -> Result<Bundle, CompileError> {
        Self::compile_under(dir, library_root, adapters_root, Boundary::Namespace)
    }

    /// Compile in no named realm but under a stated boundary — what
    /// `brokkr doctor` does for the realm it discovered, whose dialect it
    /// reports separately — or under a stated law, which also names the
    /// world's provisional offices.
    pub fn compile_under(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
        law: impl Into<RealmLaw>,
    ) -> Result<Bundle, CompileError> {
        Self::compile_unmapped(
            dir,
            library_root,
            adapters_root,
            law,
            library_root.parent().unwrap_or(Path::new("")),
        )
    }

    /// [`Bundle::compile_under`] with the operator's configuration
    /// directory named rather than inferred (decision 0065; design D2):
    /// without a map, the abstract definitions and tool dialects are read
    /// under the OPERATED repository, which a `--repo` makes a different
    /// directory from the one the library stands in. The context grants
    /// nothing either way; the root only locates what a seat's ask is
    /// checked against.
    pub fn compile_unmapped(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
        law: impl Into<RealmLaw>,
        operator_root: &Path,
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
        Self::compile_with_capabilities(
            dir,
            library_root,
            adapters_root,
            None,
            default.as_ref(),
            law,
            &crate::capabilities::CapabilityContext::no_grants(
                crate::capabilities::UNMAPPED,
                operator_root,
            ),
        )
    }

    /// Compile in a named realm. Supplying this context makes the dialect
    /// requirement enforceable before any journal is opened, and pins the
    /// realm's boundary into the bundle's identity (decision 0046 ruling
    /// 1). Compilation consults no machine: a realm declaring `seatbelt`
    /// compiles and pins the word, and the refusal that stops a run under
    /// it comes at start.
    pub fn compile_with_realm(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
        realm_name: Option<&str>,
        dialect: Option<&Dialect>,
        boundary: Boundary,
    ) -> Result<Bundle, CompileError> {
        // No grant context was supplied, so there is none: the explicit
        // no-grant context, never a skipped denial (decision 0065 ruling
        // 4). The operator's configuration directory is the one the
        // library and the default dialect already stand in.
        let capabilities = crate::capabilities::CapabilityContext::no_grants(
            realm_name.unwrap_or(crate::capabilities::UNMAPPED),
            library_root.parent().unwrap_or(Path::new("")),
        );
        Self::compile_with_capabilities(
            dir,
            library_root,
            adapters_root,
            realm_name,
            dialect,
            boundary,
            &capabilities,
        )
    }

    /// Compile under an explicit capability context (decision 0065; design
    /// D2): the operated realm, what it grants, and where the operator's
    /// definitions and tool dialects live. Every other entry point reaches
    /// here with the no-grant context; only a caller that read the
    /// operator's realm map can supply a grant, which is what makes the
    /// realm the one place a capability is granted.
    pub fn compile_with_capabilities(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
        realm_name: Option<&str>,
        dialect: Option<&Dialect>,
        law: impl Into<RealmLaw>,
        capabilities: &crate::capabilities::CapabilityContext,
    ) -> Result<Bundle, CompileError> {
        let dir = dir
            .canonicalize()
            .map_err(|e| CompileError::Invalid(format!("bundle dir {}: {e}", dir.display())))?;
        // Composition resolves FIRST, into one flat bundle; everything
        // below this line compiles a single bundle and never learns that
        // composition happened (decision 0017).
        let resolved = compose::resolve_unsealed(&dir)?;
        let note = resolved.chain_note();
        match Bundle::assemble(
            &dir,
            resolved,
            library_root,
            adapters_root,
            realm_name,
            dialect,
            law.into(),
            capabilities,
        ) {
            Ok(bundle) => Ok(bundle),
            // Every failure downstream of resolution on a composed
            // bundle is wrapped ONCE with the chain — one arm, rather
            // than teaching each lint about layers.
            // A capability refusal is wrapped in its raw words, as it has
            // always read, and bounded once, where the whole line renders.
            Err(error) => Err(match (note, error) {
                (Some(note), CompileError::Capability(reason)) => {
                    CompileError::Capability(format!("bundle: {reason} ({note})"))
                }
                (Some(note), error) => CompileError::Invalid(format!("{error} ({note})")),
                (None, error) => error,
            }),
        }
    }

    /// The agent roots ride through: composition resolves the bundle,
    /// then agent references inside the RESOLVED seats resolve against
    /// the library and adapters (decisions 0016 and 0017 layered).
    #[expect(
        clippy::excessive_nesting,
        clippy::too_many_lines,
        clippy::too_many_arguments,
        reason = "baseline 2026-09, #288; decision 0065 adds the capability context"
    )]
    fn assemble(
        dir: &Path,
        mut resolved: compose::Resolved,
        library_root: &Path,
        adapters_root: &Path,
        realm_name: Option<&str>,
        dialect: Option<&Dialect>,
        RealmLaw {
            boundary,
            provisional_offices,
        }: RealmLaw,
        capabilities: &crate::capabilities::CapabilityContext,
    ) -> Result<Bundle, CompileError> {
        // Decision 0065 (design D4 steps 1 and 2): the operated realm's
        // grants are judged BEFORE any seat is looked at — every
        // definition, every selected dialect, every restriction, and the
        // two realm-wide refusals. None of them can become an optional
        // drop, because no ask has been read yet.
        let authority = crate::capabilities::Authority::load(capabilities.clone())
            .map_err(CompileError::Capability)?;
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
        // a composition layer. The returned maps carry the adapter
        // digests whose effortless listings exempted inline seats, for
        // the manifest below, the resume assessment each inline
        // built-in model driver's adapter declares, for the engine, and
        // the digest of the declaration each assessment was read from,
        // also for the manifest. The resume witnesses ride the same
        // `drivers` pin the exemptions do: a site that already has an
        // exemption names the same provider and digest, so extending
        // the map leaves that fact unchanged rather than duplicating it.
        // Kept for the capability pass below (decision 0065): an inline
        // model seat in a bundle that seats no gate opens no agent
        // context, and its adapter's native declaration is still what
        // says how its search is switched off, as its model's tier is what
        // says whether it may be seated (proposed decision 0075 ruling 5).
        let pin_adapters = load_pin_adapters(adapters_root, &resolved.seats);
        let inline_adapters = pin_adapters.as_ref().and_then(|l| l.as_ref().ok());
        let (pin_drivers, inline_resume, resume_witness, inline_hands_notice) =
            enforce_model_pins(&resolved.seats, inline_adapters)?;
        // The one canonical family table (design D10 F1). Seeded with the
        // inline pins and assessments before any parse writes beside
        // them; every later fact is written into an entrant of this same
        // map, and a wrapper moves whole entrants.
        let mut sites: BTreeMap<String, SiteFacts> = BTreeMap::new();
        for (label, value) in pin_drivers.into_iter().chain(resume_witness) {
            site_facts(&mut sites, &label).pin_drivers = Some(value);
        }
        for (label, value) in inline_resume {
            site_facts(&mut sites, &label).inline_resume = Some(value);
        }
        for (label, notice) in inline_hands_notice {
            site_facts(&mut sites, &label).inline_hands_notice = Some(notice);
        }
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

        let authority = authority.with_minimum(parse_egress_minimum(config)?);
        let egress_minimum = authority.minimum();
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
            false => tier::inline_context(inline_adapters, egress_minimum),
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
                         agent, seats a gate, declares a secret binding or declares \
                         typed tools"
                    ))
                })?,
                egress_minimum,
                reads: Vec::new(),
                provisional_offices,
            }),
        };
        // Decision 0065 ruling 1 (CQ2; design D3): a library this compile
        // LOADED is linted whole, before any seat is resolved or subtracts
        // anything — every agent in it, seated or not, asks only for what
        // the operator defined. The capability walk below resolves seated
        // references alone, so without this a valid seated worker hides an
        // unseated office's undefined request. A library no seat opens is
        // not loaded to be linted. What the lint consulted is pinned: the
        // names ride into the manifest's definitions beside the seats' own.
        let library_asks: Vec<String> = match agents.as_ref().and_then(|a| a.library.as_ref()) {
            None => Vec::new(),
            Some(library) => {
                let problems = authority.definitions.lint(library);
                if !problems.is_empty() {
                    return Err(CompileError::Capability(problems.join("; ")));
                }
                library
                    .agents()
                    .flat_map(|agent| agent.capabilities.keys().cloned())
                    .collect()
            }
        };

        // The dialect's `verify` wrapper, applied only AFTER the authoring
        // census below (design D10 F2). Wrapping renames the wrapped seat's
        // whole subtree, so a collision it creates or disguises must be
        // decided against every authoring owner before any fact moves.
        let wrapped_verify = if uses_dialect {
            dialect.and_then(|dialect| match &dialect.verify {
                crate::dialect::CommandOrUnsupported::Command(command) => Some(command),
                crate::dialect::CommandOrUnsupported::Unsupported(_) => None,
            })
        } else {
            None
        };
        // The resolved agent hands the wrapped verify seat carried, kept for
        // the synthetic validator's hands law in the wrapper pass. Only
        // `verify` is ever wrapped, so one slot answers.
        let mut verify_agent_hands: Option<HandsSpec> = None;

        let mut seats = BTreeMap::new();
        let charters = Charters::default();
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
            refuse_boundary_key(phase, raw)?;
            refuse_crossing_keys(phase, raw)?;
            refuse_unknown_keys(phase, raw, SEAT_KEYS)?;
            refuse_confine(phase, raw)?;
            refuse_driver_keys(phase, raw)?;
            let law = SiteLaw {
                boundary,
                dir,
                agent_hands: None,
            };
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
            if has_panel || has_sequence || has_select {
                refuse_tools_on_container(phase, raw)?;
            }
            let secrets = parse_secrets(phase, raw)?;
            let agent_seat = match has_agent {
                false => None,
                true => Some(resolve_reference(
                    &mut agents,
                    &mut sites,
                    dir,
                    phase,
                    phase,
                    raw,
                    &secrets,
                    Site::Seat,
                    boundary,
                )?),
            };
            let law = SiteLaw {
                agent_hands: agent_seat.as_ref().and_then(|seat| seat.hands.as_ref()),
                ..law
            };
            let body = if let Some(agent_seat) = &agent_seat {
                SeatBody::Single {
                    role_path: agent_seat.role_path.clone(),
                    command: agent_seat.command.clone(),
                    candidates: agent_seat.candidates.clone(),
                }
            } else if has_panel {
                let (members, aggregate) = parse_panel(
                    dir,
                    phase,
                    raw,
                    &results,
                    &secrets,
                    &mut agents,
                    &mut sites,
                    boundary,
                    &charters,
                )?;
                SeatBody::Panel { members, aggregate }
            } else if has_sequence {
                SeatBody::Sequence {
                    steps: parse_sequence(
                        dir,
                        phase,
                        raw,
                        &mut agents,
                        &mut sites,
                        BodyCompile {
                            results: &results,
                            secrets: &secrets,
                            dialect,
                            boundary,
                            charters: &charters,
                        },
                    )?,
                }
            } else if has_select {
                parse_select(
                    dir,
                    phase,
                    raw,
                    &mut agents,
                    &mut sites,
                    SelectCompile {
                        results: &results,
                        secrets: &secrets,
                        roots: &resolved.roots,
                        case_origin: &resolved.case_origin,
                        dialect,
                        boundary,
                        charters: &charters,
                    },
                )?
            } else {
                record_inline_tools(
                    dir,
                    phase,
                    raw,
                    true,
                    &command_parts(raw),
                    agents.as_ref().map(|context| &context.adapters),
                    &mut sites,
                )?;
                SeatBody::Single {
                    role_path: parse_role(dir, phase, raw, &charters, &mut sites)?,
                    command: parse_command(dir, phase, raw, &secrets)?,
                    candidates: Vec::new(),
                }
            };
            // The dialect wrapper is applied after this loop, once every
            // authoring owner is registered (design D10 F2). Record the
            // agent hands the wrapped seat resolved so the synthetic
            // validator's hands law sees the same site it always did.
            if phase == "verify" && wrapped_verify.is_some() {
                verify_agent_hands = agent_seat.as_ref().and_then(resolved_hands);
            }
            // Decision 0021, at the seat's own driver-bearing site. A
            // panel or a sequence has none: its members and steps were
            // each checked where they were built.
            match &body {
                SeatBody::Single { candidates, .. } => {
                    enforce_model_policy(
                        phase,
                        raw,
                        candidates,
                        &secrets,
                        &mut agents,
                        law,
                        &mut sites,
                    )?;
                    record_hands(
                        phase,
                        raw,
                        agent_seat.as_ref().and_then(resolved_hands),
                        &secrets,
                        &mut sites,
                    )?;
                }
                SeatBody::Select { .. } => refuse_class_without_a_driver(phase, raw)?,
                // A to-be-wrapped seat's class belongs to its members or
                // its synthetic validator, exactly as it did when the
                // wrapper ran inline; the wrapper pass applies the same
                // refusal for a shape it cannot wrap.
                _ if phase == "verify" && wrapped_verify.is_some() => {}
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

        // Decision 0065 ruling 5, over the COMPOSED seats and before the
        // wrapper moves anything: every executable site — seat, member,
        // step, selected body, inherited or not — resolves office asks
        // minus seat subtractions against the realm's grants, once per
        // provider candidate, and the outcome lands in the one canonical
        // site family so relocation carries it like every other fact.
        {
            let (library, adapters) = match &agents {
                Some(context) => (
                    context.library.as_ref(),
                    CapabilityAdapters::loaded(&context.adapters),
                ),
                None => (None, CapabilityAdapters::unloaded(pin_adapters.as_ref())),
            };
            for (phase, raw) in &resolved.seats {
                // The layer that wrote the seat, as its agent's hands
                // segment is expanded against (review return F1).
                let dir = &resolved.roots[resolved.seat_origin[phase]];
                record_capabilities(
                    &authority,
                    library,
                    adapters,
                    boundary,
                    (dir, &resolved.roots, &resolved.case_origin),
                    phase,
                    raw,
                    &mut sites,
                )?;
            }
        }
        // Decision 0069: a PRESENT adapters root that does not load is
        // refused with the loader's own words, so a malformed discovery
        // notice never passes itself off as an adapter that declares none.
        // Judged after the capability pass, which already refuses a seat
        // the failed load leaves with no valid denial, naming that same
        // cause (decision 0066 ruling 1). An absent root reads as no notice.
        if let Some(Err(problem)) = pin_adapters.as_ref().filter(|_| adapters_root.exists()) {
            return Err(CompileError::Invalid(problem.clone()));
        }

        // The authoring census (design D10 F2): every structural owner is
        // registered, and a raw collision refused, BEFORE the wrapper
        // changes an address or any destination fact is written. The
        // within-body pass keeps its narrower diagnostic first, and the
        // census then catches what wrapping would disguise or create.
        refuse_aliasing_sites(&seats)?;
        let census = owner_index(&seats)?;

        if let Some(command) = wrapped_verify.filter(|_| seats.contains_key("verify")) {
            // The wrapped seat's exact authoring owners — its own single
            // label or its own panel members, never a prefix sweep that
            // would also drag an unrelated literal phase onto a wrapper
            // coordinate.
            let (prior_body, moved) =
                {
                    let body = &seats
                        .get("verify")
                        .expect("the wrapper is applied only to a parsed verify seat")
                        .body;
                    let executable =
                        match body.selected(None) {
                            Some((executable, _))
                                if !matches!(executable, ExecutableBody::Sequence { .. }) =>
                            {
                                executable
                            }
                            _ => return Err(CompileError::Invalid(
                                "dialect verify currently requires a single or panel verify seat"
                                    .into(),
                            )),
                        };
                    let moved: Vec<(String, String, crate::engine::resume::SiteKey)> =
                        crate::engine::resume::owner_sites(executable, "verify", None)
                            .into_iter()
                            .map(|(tag, owner)| {
                                let (source, destination) = match &tag {
                                    None => ("verify".to_string(), "verify:checks".to_string()),
                                    Some(tag) => {
                                        (format!("verify:{tag}"), format!("verify:checks:{tag}"))
                                    }
                                };
                                (source, destination, owner)
                            })
                            .collect();
                    let prior_body =
                        match body {
                            SeatBody::Single {
                                role_path,
                                command,
                                candidates,
                            } => StepBody::Single {
                                role_path: role_path.clone(),
                                command: command.clone(),
                                candidates: candidates.clone(),
                            },
                            SeatBody::Panel { members, aggregate } => StepBody::Panel {
                                members: members.clone(),
                                aggregate: *aggregate,
                            },
                            _ => return Err(CompileError::Invalid(
                                "dialect verify currently requires a single or panel verify seat"
                                    .into(),
                            )),
                        };
                    (prior_body, moved)
                };
            // The validator's address is claimed against the WHOLE census,
            // before any source is drained (operator ruling of 2026-09-30,
            // unit 26c): its facts are written before the wrapped body's
            // move, so a member named for it would take them along.
            claim_address(
                &census,
                "verify",
                "verify:dialect-verify",
                "the injected dialect validator",
            )?;
            // Drain every source reservation before claiming any
            // destination: a member `x` beside `checks:x` has the second
            // member's source as its destination, which stays legal.
            let mut remaining = census;
            for (source, _, _) in &moved {
                remaining.remove(source);
            }
            for (_, destination, owner) in &moved {
                claim_address(
                    &remaining,
                    "verify",
                    destination,
                    &crate::engine::resume::describe(owner),
                )?;
            }
            let dialect_site = "verify:dialect-verify";
            let synthetic = dialect_gate_site(dialect_site, boundary)?;
            let verify_raw = &resolved.seats["verify"];
            let secrets = parse_secrets("verify", verify_raw)?;
            let law = SiteLaw {
                boundary,
                dir: &resolved.roots[resolved.seat_origin["verify"]],
                agent_hands: verify_agent_hands.as_ref(),
            };
            enforce_model_policy(
                dialect_site,
                &synthetic,
                &[],
                &secrets,
                &mut agents,
                law,
                &mut sites,
            )?;
            record_hands(dialect_site, &synthetic, None, &secrets, &mut sites)?;
            // The one site the engine generated was written by no author
            // and asks for nothing, and it still gets an explicit outcome
            // through the same walk every authored site takes: "no
            // capability" is a recorded fact, never a missing one (design
            // D5). ONLY this site is given one here — an authored site the
            // walk above missed keeps none, and a site with no outcome is
            // refused at dispatch rather than launched on its defaults.
            // And it is refused like one: an `exec` harness declared with
            // a native power it cannot switch off does not seat the
            // validator either (ruling 4).
            {
                // A dialect phase is what opened the agent context above,
                // so the wrapper never runs without one.
                let context = agents
                    .as_ref()
                    .expect("a bundle that uses the dialect opened its adapters");
                let library = context.library.as_ref();
                let adapters = CapabilityAdapters::loaded(&context.adapters);
                record_capabilities(
                    &authority,
                    library,
                    adapters,
                    boundary,
                    (law.dir, &resolved.roots, &resolved.case_origin),
                    dialect_site,
                    &synthetic,
                    &mut sites,
                )?;
            }
            // The generated validator declares no `tools`, and like every
            // other visited executable it records that as a CHECKED
            // unspecified value rather than an unvisited one (design
            // D5.2; review return F3).
            let unspecified = crate::agents::LocalTools::unspecified();
            record_judged_tools(law.dir, dialect_site, unspecified, None, None, &mut sites);
            relocate_verify_facts(&mut sites, &moved);
            let prior = SequenceStep {
                name: "checks".into(),
                class: if is_gate_class(verify_raw) {
                    SeatClass::Gate
                } else {
                    SeatClass::Work
                },
                results: seats["verify"].results.clone(),
                body: prior_body,
            };
            seats.get_mut("verify").expect("parsed above").body = SeatBody::Sequence {
                steps: vec![
                    prior,
                    SequenceStep {
                        name: "dialect-verify".into(),
                        class: SeatClass::Gate,
                        results: dialect_results("verify")
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

        refuse_global_aliasing(&seats)?;

        let capability_sites: Map<String, Value> = sites
            .iter()
            .map(|(label, facts)| {
                let site = facts
                    .capabilities
                    .as_ref()
                    .expect("the capability walk gave every compiled site an outcome");
                (label.clone(), site.manifest())
            })
            .collect();
        let consulted: Vec<String> = sites
            .values()
            .filter_map(|facts| facts.capabilities.as_ref())
            .flat_map(|site| site.asks.asks.keys().chain(&site.asks.subtracted).cloned())
            .chain(library_asks)
            .collect();
        let mut capability_record = authority.manifest(&consulted);
        capability_record["sites"] = Value::Object(capability_sites);

        let select_records: Map<String, Value> = seats
            .iter()
            .filter_map(|(site, seat)| match &seat.body {
                SeatBody::Select { .. } => Some((site.clone(), body_manifest(&seat.body))),
                _ => None,
            })
            .collect();
        // Decision 0035 addendum 2026-09-11: the adapter digests whose
        // effortless listings exempted inline seats ride beside decision
        // 0021's — un-listing a route moves the digest of every bundle
        // it excused, and a bundle no listing excused keeps its shape.
        // The manifest fields are serialization projections of the one
        // canonical family table (design D10 F1), read in one place and
        // never separately mutated.
        let hands: BTreeMap<String, HandsSpec> = sites
            .iter()
            .filter_map(|(label, facts)| {
                facts.hands_spec().map(|spec| (label.clone(), spec.clone()))
            })
            .collect();
        let inline_resume: BTreeMap<String, Value> = sites
            .iter()
            .filter_map(|(label, facts)| {
                facts
                    .inline_resume
                    .clone()
                    .map(|value| (label.clone(), value))
            })
            .collect();
        let records: Map<String, Value> = sites
            .iter()
            .filter_map(|(label, facts)| facts.record.clone().map(|record| (label.clone(), record)))
            .collect();
        let mut drivers: Map<String, Value> = Map::new();
        fold_driver_facts(&mut drivers, &sites);
        let drivers = (!drivers.is_empty()).then_some(&drivers);
        // Design D7 (rebuild unit 16-fix-b, F3): every layer's identity is
        // sealed from the buffers it was composed from and the charters its
        // seats were bound to, ancestors first, so no consumed file is read
        // again by a walk. The leaf's walk takes its own the same way, and
        // each input must still stand as it was read or nothing seals.
        // Rebuild unit 18-fix-b (council F2): who each charter's owner is
        // was taken by the read that bound the charter, and the seal's check
        // compares it; no later walk records another. Its return (council
        // F2): a library charter, which no layer's walk pins, is checked by
        // the read that bound it, owner included, before any identity is
        // sealed.
        for read in agents.iter().flat_map(|context| &context.reads) {
            read.check()?;
        }
        resolved.seal(charters.into_inner())?;
        let manifest = manifest_for(
            dir,
            &name,
            &resolved.chain,
            Some(&records),
            drivers,
            &hands,
            &select_records,
            boundary,
            Some(capability_record),
            &resolved.leaf_digests(),
        )?;
        resolved.check_leaf(manifest["files"].as_object().expect("manifest files"))?;
        // Second council H6: every charter this compile bound, kept where a
        // projection of the site facts cannot lose it, with each binding a
        // site selected for its path (rebuild unit 17).
        let mut charters = CharterPins::new();
        for charter in sites.values().filter_map(|site| site.charter.clone()) {
            charters
                .entry(charter.path.clone())
                .or_default()
                .insert(charter);
        }
        Ok(Bundle {
            hands,
            inline_resume,
            sites,
            charters,
            name,
            description,
            cost,
            dir: dir.to_path_buf(),
            roots: resolved.roots,
            boundary,
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

/// Two structurally different sites of one selected body may not
/// flatten to one address (proposed decision 0056 ruling 1; design D2).
///
/// The check is scoped to the lookup scopes that actually key on the
/// flattened label: `select_candidates` inserts it into a `BTreeMap`,
/// `argv_for` selects by it, `site_boundary` and `mark_hands` consult
/// `bundle.hands` under it, and the per-site resume plan is keyed by it.
/// So step `a:b` with member `c` and step `a` with member `b:c` — both
/// spelled `a:b:c` — could select each other's candidate, hands identity
/// and boundary before any resume digest exists. An ambiguous bundle
/// fails at compile time, naming both sites.
///
/// Ordinary repeated member names under different steps are untouched:
/// `review`/`alpha` and `design`/`alpha` flatten to two distinct
/// strings, and nothing about chain progression or the historical
/// meaning of a display tag moves.
fn refuse_aliasing_sites(seats: &BTreeMap<String, Seat>) -> Result<(), CompileError> {
    for (phase, seat) in seats {
        // Each selectable body, with the case that selects it. Selection
        // never nests, so every entry here resolves — the `filter_map`
        // carries that fact rather than a branch nothing can take.
        let bodies: Vec<(Option<&str>, &SeatBody)> = match &seat.body {
            SeatBody::Select { cases, default, .. } => cases
                .iter()
                .map(|(case, body)| (Some(case.as_str()), body))
                .chain(default.as_deref().map(|body| (Some("default"), body)))
                .collect(),
            body => vec![(None, body)],
        };
        let executable = bodies
            .into_iter()
            .filter_map(|(case, body)| body.selected(None).map(|(body, _)| (case, body)));
        for (case, executable) in executable {
            let sites =
                crate::engine::resume::structural_sites(executable, phase, case, seat.has_gate);
            if let Some((label, both)) = crate::engine::resume::flat_address_collision(&sites) {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' addresses two different sites as '{label}': {both}. \
                     The selection, the argv lookup, the hands map and the boundary map all \
                     key on that one string, so one site would answer for the other; rename \
                     one of them"
                )));
            }
        }
    }
    Ok(())
}

/// Every flattened address of `seats` mapped to its unique structural
/// owner, refusing a label two different owners claim (design D10 F2;
/// proposal SR1). The within-body pass above cannot see a distinct phase,
/// a literal `verify:checks` beside the wrapped `verify`, the injected
/// validator, or a literal member-destination phase; engine `site_plans`,
/// `argv_for`, `mark_hands` and the boundary map all key on that one
/// string, so two owners sharing it would serve each other's assessment,
/// hands and boundary. The walk is structural: it registers factless and
/// deterministic owners, selected cases, defaults, panels and sequences,
/// so an empty map can never excuse an alias.
///
/// Called twice: on the authoring bodies before the wrapper changes an
/// address, and on the final bodies afterwards. The first call is the one
/// that catches a collision wrapping would disguise or create.
fn owner_index(
    seats: &BTreeMap<String, Seat>,
) -> Result<BTreeMap<String, crate::engine::resume::SiteKey>, CompileError> {
    let mut seen: BTreeMap<String, crate::engine::resume::SiteKey> = BTreeMap::new();
    for (phase, seat) in seats {
        // Each selectable body, with the case that selects it; selection
        // never nests, so every case body resolves.
        let bodies: Vec<(Option<&str>, &SeatBody)> = match &seat.body {
            SeatBody::Select { cases, default, .. } => cases
                .iter()
                .map(|(case, body)| (Some(case.as_str()), body))
                .chain(default.as_deref().map(|body| (Some("default"), body)))
                .collect(),
            body => vec![(None, body)],
        };
        let executable = bodies
            .into_iter()
            .filter_map(|(case, body)| body.selected(None).map(|(body, _)| (case, body)));
        for (case, executable) in executable {
            let site_name = match case {
                Some(case) => format!("{phase}:{case}"),
                None => phase.clone(),
            };
            for (tag, owner) in crate::engine::resume::owner_sites(executable, phase, case) {
                let label = match &tag {
                    None => site_name.clone(),
                    Some(tag) => format!("{site_name}:{tag}"),
                };
                // A flattened label this census has already registered is
                // a collision, whether or not the two structural keys
                // happen to name equal owners. The enumeration cannot
                // legitimately revisit an equal owner, so there is no
                // same-owner tolerance to keep: phase keys are unique map
                // keys, case keys come from a closed strategy vocabulary
                // that excludes `default`, selection does not nest, and
                // each body emits its single once or its members/steps
                // with distinct enumerated indices in `SiteKey`. Equal
                // keys therefore cannot be emitted twice; a repeated
                // label names different keys and must be refused. (Fact
                // merges for one owner are a separate operation in
                // `site_facts`, not an address registration.)
                if let Some(first) = seen.get(&label) {
                    return Err(CompileError::Invalid(format!(
                        "seat '{phase}' addresses two different sites as '{label}': {} and \
                         {}. The selection, the argv lookup, the hands map and the boundary \
                         map all key on that one string, so one site would answer for the \
                         other; rename one of them",
                        crate::engine::resume::describe(first),
                        crate::engine::resume::describe(&owner)
                    )));
                }
                seen.insert(label, owner);
            }
        }
    }
    Ok(seen)
}

/// The final ownership check: every flattened address of the compiled
/// bundle names exactly one structural owner. The wrapper pass claims the
/// addresses it introduces against the authoring census before writing,
/// so this second walk is an independent backstop rather than the only
/// guard (design D10 F2).
fn refuse_global_aliasing(seats: &BTreeMap<String, Seat>) -> Result<(), CompileError> {
    refuse_aliasing_sites(seats)?;
    owner_index(seats)?;
    Ok(())
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
/// it. `results`, `secrets` and `class` stay legal beside it, because
/// they are bindings the SEAT provides rather than statements about what
/// the agent is, and `brokkr agents show` never claims to show them.
/// `limits` is likewise a strategy bound on the invocation, not part of
/// the office. `class` in particular is the seat's authority, never the
/// agent's: one charter may sit in a work seat here and a gate seat
/// there, which is why decision 0021 ruling 1 puts the division in bundle
/// data. `driver.confine` was the one `driver` key legal here until
/// decision 0046 ruling 5 retired it; [`refuse_confine`] now refuses it
/// by name before this lint reads the driver at all.
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
        if let Some(key) = object.keys().next() {
            return refuse(&format!("driver.{key}"));
        }
    }
    Ok(())
}

/// Decision 0046 ruling 5: decision 0008's `driver.confine` retired into
/// the `container` boundary the realm declares, and until slice (iii)
/// measures that boundary the field is refused wherever it is written —
/// in a bundle and beside `agent:` alike — so a bundle that still carries
/// it fails loudly rather than running a wrapper nobody exercised. No
/// shipped bundle declares it, so nothing shipped moves for this reason.
fn refuse_confine(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.pointer("/driver/confine") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares driver.confine; decision 0008's container \
             confinement retired into the `container` boundary a realm declares \
             (realms.json, forge.realms/v4), and the field is refused until that \
             boundary is measured in decision 0046's slice (iii) (decision 0046 \
             ruling 5)"
        ))),
    }
}

/// The keys a site's `driver` object may write. Closed, like the site's
/// own vocabulary ([`refuse_unknown_keys`]): the compiler reads only
/// `command` there (and refuses `confine` by name first, in
/// [`refuse_confine`]), so before rebuild unit 5e a `tools` object placed
/// under `driver` compiled, delivered nothing and ran the seat at its
/// harness default.
const DRIVER_KEYS: &[&str] = &["command"];

/// Decision 0004's closed input semantics, applied to the `driver`
/// object (decision 0065 slice one, rebuild unit 5e): an unknown key is
/// refused where it is written, never ignored. The reason names the key
/// and the object, and never the value; a key that is not a short name is
/// described rather than echoed, so the reason stays bounded. A
/// capability key gets the place it belongs.
fn refuse_driver_keys(what: &str, raw: &Value) -> Result<(), CompileError> {
    let Some(driver) = raw.get("driver").and_then(Value::as_object) else {
        return Ok(());
    };
    let Some(key) = driver
        .keys()
        .find(|key| !DRIVER_KEYS.contains(&key.as_str()))
    else {
        return Ok(());
    };
    let named = if key.len() <= 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-.".contains(&byte))
    {
        format!("'{key}'")
    } else {
        "one that is not a short name and is not echoed".to_string()
    };
    let place = match key.as_str() {
        "tools" | "hands" | "capabilities" => {
            format!(" '{key}' is a site declaration, written on the seat beside its driver.")
        }
        "sandbox" => {
            " 'sandbox' is a typed tool field, written as 'tools.sandbox' on the seat.".to_string()
        }
        _ => String::new(),
    };
    Err(CompileError::Invalid(format!(
        "seat {} driver has an unknown key, {named}; known: {}.{place} A key the compiler \
         does not read is a declaration that was never made — a capability placed there would \
         compile, deliver nothing and run the seat at its harness default — so it is refused \
         rather than ignored (decision 0004; decision 0065 slice one, rebuild unit 5e)",
        bounded_site(what),
        DRIVER_KEYS.join(", ")
    )))
}

/// A site identity rendered for a refusal, bounded and safe (decision
/// 0065 slice one, rebuild unit 5e-fix). A site label is built from
/// author-written phase, member, step, case and recipe names, and nothing
/// bounds their length or their characters. A label of at most 64 bytes of
/// ASCII letters, digits, `_`, `-`, `.` and `:` (the site separator) is
/// quoted whole. Any other is named by its leading run of those
/// characters, at most 32, and its length in bytes, so a 100,000-character
/// member name cannot become a 100,000-byte reason and a newline cannot
/// forge a line. Rebuild unit 5e-fix-b renders a root key and a
/// composition chain's layer names the same way.
pub(crate) fn bounded_site(label: &str) -> String {
    if plain_label(label) {
        return format!("'{label}'");
    }
    let lead: String = label
        .bytes()
        .take_while(safe_label_byte)
        .take(32)
        .map(char::from)
        .collect();
    format!("'{lead}…' ({} bytes, not echoed in full)", label.len())
}

/// A label [`bounded_site`] quotes whole: at most 64 bytes, all of them
/// [`safe_label_byte`].
fn plain_label(label: &str) -> bool {
    label.len() <= 64 && label.bytes().all(|byte| safe_label_byte(&byte))
}

fn safe_label_byte(byte: &u8) -> bool {
    byte.is_ascii_alphanumeric() || b"_-.:".contains(byte)
}

/// Decision 0046 ruling 1: the boundary is the realm's fact, declared in
/// `realms.json` under `forge.realms/v4`, and a bundle never names it. A
/// site that writes the key is told where the word lives rather than
/// that the key is unknown.
fn refuse_boundary_key(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.get("boundary") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares boundary; {}",
            brokkr_protocol::hands::BOUNDARY_IS_THE_REALMS
        ))),
    }
}

/// Decision 0057 ruling 1: a crossing is the REALM's — a file one realm
/// publishes and another realm pins by its bytes — declared in
/// `realms.json` under `forge.realms/v5`, and a bundle never names one.
///
/// The same refusal `boundary` gets one function above, on the same terms
/// and for the same reason: a site that writes the word is told where the
/// word lives rather than that its key is unknown. Per seat, like
/// `boundary`, because a seat is the site that would claim the fact;
/// there is no root-level unknown-key check for either.
fn refuse_crossing_keys(what: &str, raw: &Value) -> Result<(), CompileError> {
    let Some(key) = CROSSING_KEYS.iter().find(|key| raw.get(*key).is_some()) else {
        return Ok(());
    };
    Err(CompileError::Invalid(format!(
        "seat '{what}' declares {key}; {CROSSING_IS_THE_REALMS}"
    )))
}

/// The two words `forge.realms/v5` adds, refused wherever a bundle writes
/// one.
const CROSSING_KEYS: [&str; 2] = ["publishes", "consumes"];

/// Where a crossing lives (decision 0057 ruling 1), said once for the
/// site that tries to write it into a bundle.
const CROSSING_IS_THE_REALMS: &str = "a crossing is declared by the realm \
    (realms.json, forge.realms/v5) and never by a bundle, because the file one realm \
    publishes and the digest another pins it at is the realm's fact and not a \
    recipe's (decision 0057 ruling 1)";

/// The box the compiler builds for every dialect `validate`/`check` step
/// (decision 0042 ruling 4). Public so `brokkr doctor` probes the
/// dialect's tool inside the SAME box its gate will run in, instead of
/// answering for the host PATH while the gate runs with `$HOME` unbound
/// (issue #218). One function, two readers: a spec that changes here
/// changes both the box that runs and the box doctor measures.
pub fn dialect_gate_hands() -> HandsSpec {
    HandsSpec::default()
}

/// The synthetic boxed exec gate the compiler builds for a dialect
/// validate or check step (decision 0042 ruling 4), shared by the two
/// sites that build one. Under `harness` and `open` the step is refused
/// here, before the gate law runs: its argv is the realm's dialect
/// declaration and not the bundle's own pinned script, which is the one
/// thing decision 0046 ruling 4 admits at an unboxed gate, and a design
/// note may not widen a ruling (design DD8; decision 0042's addendum,
/// ruling 1). A decision that admits the step deletes this arm.
fn dialect_gate_site(what: &str, boundary: Boundary) -> Result<Value, CompileError> {
    if !boundary.is_boxed() {
        return Err(CompileError::Invalid(format!(
            "dialect step '{what}' holds its gate boxed (decision 0042 ruling 4), and under \
             the `{boundary}` boundary no box stands: its command is the realm's dialect \
             declaration, not the bundle's own pinned script, which is all decision 0046 \
             ruling 4 admits at an unboxed gate. Run the realm under a boxed boundary \
             (namespace) until a decision admits the dialect step on the pinned-script terms"
        )));
    }
    Ok(json!({
        "class": "gate",
        "hands": dialect_gate_hands().to_value(),
        "driver": {"command": ["{brokkr}", "driver", "exec", "--"]}
    }))
}

/// Resolve `"agent": "<name>"` into an ordinary seat body, and record
/// the resolution under this invocation site.
#[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn resolve_reference(
    agents: &mut Option<AgentContext>,
    sites: &mut BTreeMap<String, SiteFacts>,
    dir: &Path,
    what: &str,
    site_key: &str,
    raw: &Value,
    secrets: &[String],
    site: Site,
    boundary: Boundary,
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
    // Decision 0065 slice one (design D5.2): the site's own typed `tools`
    // narrows a PRIVATE clone of the office before composition. Shape is
    // judged here, narrowing inside the resolver, and the effective value
    // is what the chain below is composed from.
    let requested = decode_site_tools(what, raw)?;
    let report = crate::agents::report_narrowed(
        library,
        &context.adapters,
        &Availability::unspecified(),
        name,
        boundary,
        &requested,
    )
    .map_err(|e| CompileError::Invalid(format!("seat '{what}': {e}")))?;
    let effective = report.agent.local();
    // D33: judge every mapped hands link before resolving capability gaps.
    // In particular dsh/LaneTally earn the tier refusal under namespace,
    // and the missing harness.gate refusal under harness. An unmapped
    // chain still receives the resolver's precise unmapped-model error.
    if report.agent.hands.is_some() && report.entries.iter().all(|entry| entry.provider.is_some()) {
        let candidates: Vec<Candidate> = report
            .entries
            .iter()
            .map(|entry| Candidate {
                agent: name.to_string(),
                model: entry.model.clone(),
                effort: entry.effort.clone(),
                provider: entry.provider.clone().expect("mapped above"),
                argv: entry.argv.clone(),
                hands_fragment: entry.hands_fragment.clone(),
                harness: entry.harness.clone(),
                // The hands law reads argv, class and boundary; the
                // resume assessment and the discovery notice are not its
                // terms, and this projection is discarded after that
                // judgment.
                resume: Default::default(),
                hands_notice: None,
                // The entry's own lowering, refused or composed, never a
                // valid empty one standing in for it (design D5.7).
                lowering: entry.lowering.clone(),
            })
            .collect();
        enforce_model_policy(
            what,
            raw,
            &candidates,
            secrets,
            agents,
            SiteLaw {
                boundary,
                dir,
                agent_hands: report.agent.hands.as_ref(),
            },
            sites,
        )?;
    }
    let context = agents.as_mut().expect("agent context opened above");
    let resolution = crate::agents::resolve_report(report, &context.adapters)
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
        refuse_permission_pins(what, candidate, &context.adapters)?;
        lint_secret_refs(what, &candidate.argv, secrets)?;
        candidates.push(Candidate {
            agent: candidate.agent.clone(),
            model: candidate.model.clone(),
            effort: candidate.effort.clone(),
            provider: candidate.provider.clone(),
            argv: expand_command(dir, &candidate.argv),
            hands_fragment: candidate.hands_fragment.clone(),
            harness: candidate.harness.clone(),
            resume: candidate.resume.clone(),
            hands_notice: candidate.hands_notice.clone(),
            lowering: expand_lowering(dir, &candidate.lowering),
        });
    }
    site_facts(sites, site_key).record = Some(resolution.record.clone());
    // Second council H6: the charter this site will be told, bound to the
    // digest its library record pins, so the dispatch door can compare the
    // bytes it is about to hand over against the bytes the compile read.
    // Rebuild unit 17: its owner is the library the office was loaded
    // from, with that library's own root, whether it stands outside the
    // recipe or inside it.
    // Rebuild unit 18-fix-b (council F1, F2): the pin is the binding of a
    // bound read from the library's directory, which must supply the bytes
    // the library record pins; a charter no such read binds is refused here
    // rather than compiled into a seat every dispatch refuses.
    let source = &resolution.charter_source;
    let refused =
        |clause: String| library_refusal(what, &resolution.agent, &source.reference, &clause);
    let bound = match owned_input(&source.library, &source.reference) {
        Ok(bound) if sha256_bytes(&bound.bytes) == source.digest => bound,
        Ok(_) => {
            return Err(refused(
                "whose bytes changed after its library was loaded".into(),
            ))
        }
        Err(InputFault::Missing(error)) => return Err(refused(missing_clause(&error))),
        Err(InputFault::Place(place)) => return Err(refused(place.to_string())),
    };
    let owner = CharterOwner::Library {
        agent: resolution.agent.clone(),
        root: source.library.clone(),
    };
    let pin = CharterPin::of(owner, &source.reference, resolution.charter.clone(), &bound);
    site_facts(sites, site_key).charter = Some(pin);
    // Rebuild unit 18-fix-b return (F2): the read is held until the seal,
    // which checks it still stands, its owner included.
    context.reads.push(LibraryRead {
        site: what.to_string(),
        agent: resolution.agent.clone(),
        reference: source.reference.clone(),
        digest: source.digest.clone(),
        held: bound.held,
    });
    // The capability pass judges exactly the chain this site will run
    // (decision 0065): one outcome per candidate, never their union.
    site_facts(sites, site_key).chain = candidates.clone();
    // The effective local declaration, beside the site's other facts
    // (design D5.2): a checked value even where nothing was declared.
    site_facts(sites, site_key).local = Some(effective);
    Ok(ResolvedSeat {
        role_path: resolution.charter.clone(),
        command: candidates[0].argv.clone(),
        candidates,
        limits: resolution.limits,
        inputs: resolution.inputs.clone(),
        hands: resolution.hands.clone(),
    })
}

/// Rebuild unit 5c-fix-b (chief R1; operator ruling 1 of 2026-09-23): a
/// model or effort pin never carries a permission control. An adapter
/// whose `model_flag` or `effort_flag` spells one is refused as a
/// declaration, whichever model or effort it would pin, and every later
/// `template` contribution of the candidate's composition must be a model
/// or effort pin ([`brokkr_protocol::native_controls::pin_fault`]), so a
/// model or effort value cannot smuggle one either. The refusal names the
/// field and the control's canonical spelling, or the contribution's
/// position and a fixed cause, and never a token.
fn refuse_permission_pins(
    what: &str,
    candidate: &crate::agents::Candidate,
    adapters: &crate::agents::Adapters,
) -> Result<(), CompileError> {
    use brokkr_protocol::native_controls::{permission_control, pin_fault};
    let provider = &candidate.provider;
    // A resolved candidate was composed from its provider's adapter, whose
    // driver template opens the composition (operator ruling of 2026-09-30,
    // unit 26c): the pins judged are the ones that adapter and that
    // composition carry.
    let pins = adapters.adapter(provider).into_iter().flat_map(|adapter| {
        [
            ("model_flag", &adapter.model_flag),
            ("effort_flag", &adapter.effort_flag),
        ]
    });
    for (field, flag) in pins {
        if let Some(control) = flag.as_deref().and_then(permission_control) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}': the '{provider}' adapter declares its {field} as the \
                 permission control '{control}'; a model or effort pin names a model or an \
                 effort and never carries a permission mode, so the declaration is refused \
                 rather than composed (operator ruling 1 of 2026-09-23; rebuild unit \
                 5c-fix-b)"
            )));
        }
    }
    let segments = crate::engine::composed(candidate)
        .map_or(&[][..], |composition| composition.segments.as_slice());
    for (at, pin) in segments.iter().enumerate().skip(1) {
        if pin.origin != brokkr_protocol::native_controls::Origin::Template {
            continue;
        }
        if let Some(fault) = pin_fault(&segments[0].argv, &pin.argv) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}': the '{provider}' adapter's composition carries a template \
                 contribution (segment {}) behind its driver template that {fault}; only a model \
                 or effort pin may follow the driver template, and its tokens are not echoed \
                 because they can carry a value (operator ruling 1 of 2026-09-23; rebuild unit \
                 5c-fix-b)",
                at + 1
            )));
        }
    }
    Ok(())
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
        return Ok(crate::capabilities::ABSENT_EGRESS_MINIMUM);
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
    "capabilities",
    "tools",
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
    "capabilities",
    "tools",
    "panel",
    "aggregate",
    "sequence",
];

/// The keys a PANEL MEMBER may write. Narrower than a seat's: a member
/// has no `results`, `limits`, `inputs` or `secrets` of its own — the
/// seat above it does — which is why an agent declaring them at a member
/// site is already refused rather than silently discarded.
const MEMBER_KEYS: &[&str] = &[
    "class",
    "agent",
    "role",
    "driver",
    "hands",
    "capabilities",
    "tools",
];

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
    "capabilities",
    "tools",
    "panel",
    "aggregate",
    "dialect",
];

/// Decode one site's typed `tools` (decision 0065 slice one, design D5.2)
/// through the agents' strict decoder, naming the site as every other
/// refusal of it does. A site that writes no `tools` requests nothing.
fn decode_site_tools(what: &str, raw: &Value) -> Result<crate::agents::LocalTools, CompileError> {
    match raw.as_object() {
        Some(site) => crate::agents::decode_local_tools(&format!("seat '{what}'"), site)
            .map_err(CompileError::Invalid),
        None => Ok(crate::agents::LocalTools::unspecified()),
    }
}

/// A `tools` declaration beside a panel, sequence or select is refused
/// (design D5.2): a local declaration belongs to the site that executes,
/// and a container that carried one could only share it as a grant or
/// drop it — so even an empty object is refused here.
fn refuse_tools_on_container(what: &str, raw: &Value) -> Result<(), CompileError> {
    match raw.get("tools") {
        None => Ok(()),
        Some(_) => Err(CompileError::Invalid(format!(
            "seat '{what}' declares 'tools' beside a panel, sequence or select; a local \
             declaration belongs to the site that executes — the member, step or case body — \
             and a container cannot own one, even an empty object, because it would either \
             become a shared grant or be ignored (decision 0065 slice one, design D5)"
        ))),
    }
}

/// Decode, judge and record the typed local declaration of a site whose
/// command no office composes — an inline driver site or a dialect-generated
/// check (design D5.3), whose command is `command`. Decoding is not runnable
/// admission. A typed allow at an inline Claude or LaneTally site is lowered
/// onto its adapter's tool permissions by unit 3's own lowering (rebuild
/// unit 5b), and recorded for the engine to append as its `local` segment;
/// every other nonempty field is kept exactly and refused rather than
/// recorded beside an unchanged command. An unspecified declaration is
/// recorded as a checked value, distinct from a site never visited.
///
/// Rebuild unit 5d (operator ruling of 2026-09-25): a typed sandbox at a
/// seat — `seat`, a site whose own `class` rules it, never a panel member,
/// sequence step or select case — whose command dispatches the codex
/// driver is lowered by [`lower_inline_sandbox`]; everywhere else it keeps
/// its refusal.
fn record_inline_tools(
    dir: &Path,
    what: &str,
    raw: &Value,
    seat: bool,
    command: &[String],
    adapters: Option<&crate::agents::Adapters>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let local = decode_site_tools(what, raw)?;
    let lowered = match &local.allow {
        Some(allow) => Some(lower_inline_allow(what, raw, command, allow, adapters)?),
        None => None,
    };
    let sandboxed = match local.sandbox {
        Some(class) if seat && dispatch_driver(command).as_deref() == Some("codex") => {
            Some(lower_inline_sandbox(what, raw, command, class, adapters)?)
        }
        Some(_) => {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' declares 'tools.sandbox' on a site whose command no office \
                 composes; the engine does not yet lower a typed local sandbox into an authored \
                 command, so the restriction would be recorded and not delivered — it is kept \
                 exactly and refused rather than run unrestricted, until decision 0065 slice \
                 one's lowering and origin transport prove its delivery (design D5.3); an \
                 authored flag cannot stand in for it"
            )))
        }
        None => None,
    };
    record_judged_tools(dir, what, local, lowered, sandboxed, sites);
    Ok(())
}

/// Record a site's judged local declaration and what it lowered to: the
/// half of [`record_inline_tools`] that refuses nothing. The generated
/// dialect validator declares no `tools`, so it records the unspecified
/// declaration here directly (operator ruling of 2026-09-30, unit 26c).
fn record_judged_tools(
    dir: &Path,
    what: &str,
    local: crate::agents::LocalTools,
    lowered: Option<InlineAllow>,
    sandboxed: Option<InlineClass>,
    sites: &mut BTreeMap<String, SiteFacts>,
) {
    let facts = site_facts(sites, what);
    facts.local = Some(local);
    // Expanded as an agent's composition is, segment by segment, so the
    // engine's contribution names this machine's paths as the command does.
    let expanded = |segment: Segment| Segment {
        origin: segment.origin,
        argv: expand_command(dir, &segment.argv),
    };
    // A seat's allow and its sandbox never both lower: the one lowers only
    // for claude and lanetally, the other only for codex. Whichever did
    // brings its adapter's template (rebuild unit 5d reuses 5c's).
    let (lowered, allow_template, permissions) = lowered
        .map_or((None, None, None), |(lowered, template, permissions)| {
            (Some(lowered), template, permissions)
        });
    let (sandboxed, sandbox_template, declared) = sandboxed.map_or(
        (None, None, Vec::new()),
        |(sandboxed, template, declared)| (Some(sandboxed), template, declared),
    );
    // Rebuild unit 14a1: the dialect the lowering read, recorded before
    // anything is expanded — the class's fragment as the adapter declares
    // it, carried beside and never read back from the segment emitted.
    facts.inline_dialect = Some(crate::agents::DeclaredDialect {
        permissions,
        sandbox: declared,
        ..Default::default()
    });
    let template = allow_template.or(sandbox_template);
    // Rebuild unit 5c-fix: the declaration is recorded as its own typed
    // fact, beside and never read back from the segment to be emitted.
    let lowers = lowered.is_some() || sandboxed.is_some();
    facts.declared_template = lowers.then(|| match &template {
        Some(declared) => TemplateExpectation::Declared(expand_command(dir, &declared.argv)),
        None => TemplateExpectation::None,
    });
    facts.inline_local = lowered.map(|lowered| crate::agents::LocalLowering {
        segment: expanded(lowered.segment),
        limits: lowered.limits,
    });
    facts.inline_sandbox = sandboxed.map(|sandboxed| InlineSandbox {
        segment: expanded(sandboxed.segment),
        ..sandboxed
    });
    facts.inline_template = template.map(expanded);
}

/// Rebuild unit 5d (operator ruling of 2026-09-25, "narrow"; design D5.3):
/// the one inline shape whose typed sandbox the engine delivers — a seat
/// whose command dispatches the codex driver, with no hands and no
/// capability-bearing option of its author's. A work seat is admitted
/// exactly `workspace-write` and a gate exactly `read-only`; the class is
/// then expressed by the fragment the adapter declares for that class of
/// seat (`hands.harness.work` or `hands.harness.gate`), which must express
/// exactly it, judged by the same reading [`admit_local_sandbox`] judges an
/// agent's fragment with. The gate fragment opens the adapter's declared
/// result door. Beside it comes the adapter's permission template, as unit
/// 5c places it, or `None` where the adapter declares none, and the
/// fragment as the adapter declares it (rebuild unit 14a1).
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn lower_inline_sandbox(
    what: &str,
    raw: &Value,
    command: &[String],
    class: Sandbox,
    adapters: Option<&crate::agents::Adapters>,
) -> Result<InlineClass, CompileError> {
    let requested = class.name();
    let refuse = |cause: String| {
        CompileError::Invalid(format!(
            "seat '{what}' declares 'tools.sandbox' '{requested}' {cause}"
        ))
    };
    if raw.get("hands").is_some() {
        return Err(refuse(
            "beside the site's own hands; hands replace the harness's tools, so an inline \
             sandbox would stand beside the box's restriction rather than express it — it is \
             kept exactly and refused (decision 0065 slice one, design D5.3)"
                .to_string(),
        ));
    }
    let seat_class = parse_class(what, raw)?;
    let (admitted, site_kind) = match seat_class {
        SeatClass::Gate => (Sandbox::ReadOnly, "an inline Codex gate"),
        SeatClass::Work => (Sandbox::WorkspaceWrite, "an inline Codex work seat"),
    };
    if class != admitted {
        let admitted = admitted.name();
        return Err(refuse(format!(
            "at {site_kind}, where only '{admitted}' is admitted: a gate changes no files, so it \
             runs read-only and delivers its result through the last-message door, a work seat \
             runs workspace-write, and danger-full-access is admitted nowhere (operator ruling of \
             2026-09-25, inline Codex sandbox classes are narrowed; design D5.3)"
        )));
    }
    // Operator ruling 1 of 2026-09-23: the engine composes the class as its
    // own contribution, and nothing it composes is merged with or ordered
    // against a control of the author's, read under the harness's grammar.
    let grammar = brokkr_protocol::native_controls::grammar::grammar("codex")
        .expect("the codex grammar is modelled");
    let authored = grammar
        .parse(brokkr_protocol::native_controls::harness_arguments(command))
        .map_err(|problem| {
            refuse(format!(
                "while its authored command cannot be read: the 'codex' command grammar cannot \
                 place argument {} ({}), whose token is not echoed because it can carry a value: \
                 it {}. A control nobody can read is a control nobody can rule on, so it is \
                 refused rather than passed through (decision 0066 ruling 6; operator ruling 1 \
                 of 2026-09-23)",
                problem.at + 1,
                unplaced_label(grammar, &problem.token),
                problem.cause
            ))
        })?;
    if let Some((node, kind)) = authored
        .nodes
        .iter()
        .find_map(|node| authored_sandbox_control(node).map(|kind| (node, kind)))
    {
        return Err(refuse(format!(
            "while its authored command carries '{}' (argument {}), {kind}; the engine composes \
             the typed class as its own contribution and a recipe authors no capability-bearing \
             option beside it, so the site is refused rather than reconciled (operator ruling 1 \
             of 2026-09-23; decision 0065 slice one, design D5.3)",
            node.name(),
            node.at + 1
        )));
    }
    let adapter = adapters
        .and_then(|adapters| adapters.adapter("codex"))
        .ok_or_else(|| {
            refuse(
                "for driver 'codex', which no loaded adapter declares; with no sandbox control \
                 the class cannot be expressed, so it is refused rather than run unrestricted \
                 (decision 0065 slice one, design D5.3)"
                    .to_string(),
            )
        })?;
    // Rebuild unit 5d-fix (chief F2): a gate delivers through the
    // last-message door alone, captured into the engine-owned result path.
    let (part, fragment, door) = match seat_class {
        SeatClass::Gate => {
            if adapter.harness.result != crate::agents::ResultDoor::LastMessage {
                return Err(refuse(
                    "at an inline Codex gate, but the codex adapter declares its \
                     `hands.harness.result` door as 'file'; a gate delivers only through the \
                     last-message door, the harness's capture of its final message into the \
                     engine-owned result path, so file delivery is refused (decision 0046 ruling \
                     4; operator ruling of 2026-09-25; rebuild unit 5d-fix)"
                        .to_string(),
                ));
            }
            (
                "`hands.harness.gate` fragment",
                adapter.harness.gate.as_deref(),
                crate::agents::ResultDoor::LastMessage,
            )
        }
        SeatClass::Work => (
            "`hands.harness.work` fragment",
            adapter.harness.work.as_deref(),
            crate::agents::ResultDoor::File,
        ),
    };
    let fragment = fragment.unwrap_or(&[]);
    let whom = format!("seat '{what}'");
    match expressed_sandbox(&whom, part, fragment, Contribution::Written)? {
        Some(found) if found == requested => {}
        _ => {
            return Err(refuse(format!(
                "at {site_kind}, but the codex adapter's {part} does not express exactly that \
                 class; a missing or different fragment is not a representation, and a fragment \
                 is neither called narrower nor clamped — refused (design D5.3)"
            )))
        }
    }
    let refused = |cause: String| CompileError::Invalid(format!("seat '{what}': {cause}"));
    let template = crate::agents::inline_template(adapter, "codex").map_err(refused)?;
    let sandboxed = InlineSandbox {
        class,
        segment: Segment::new(Origin::Local, fragment),
        door,
    };
    // The whole launch, native plan included, is judged once the plan is
    // resolved ([`admit_inline_launch`]; rebuild unit 5d-fix-b).
    Ok((sandboxed, template, fragment.to_vec()))
}

/// What [`lower_inline_sandbox`] lowered: the class, the adapter's template
/// and the fragment the class lowered onto, as the adapter declares it.
type InlineClass = (InlineSandbox, Option<Segment>, Vec<String>);

/// What an option an author wrote beside a typed inline Codex sandbox is
/// (rebuild unit 5d; operator ruling 1 of 2026-09-23), or `None` for one
/// that bears no capability. Judged on the parsed node under the codex
/// grammar, so every spelling of an option is judged at once, and its value
/// is never echoed: the grammar's own capability classification (a list,
/// a load, a catalogue control — `--sandbox` itself among them — or a
/// configuration assignment into a capability table or with no bounded
/// meaning), and beside it the two inert-typed options whose value the
/// engine's control owns: the root the class is measured from and the
/// file the gate's result door writes.
fn authored_sandbox_control(
    node: &brokkr_protocol::native_controls::grammar::Node,
) -> Option<&'static str> {
    Some(match node.name() {
        "--sandbox" => "a sandbox class, which the typed declaration alone supplies",
        "--cd" => "a root selector, which moves the root the sandbox class is measured from",
        "--output-last-message" => {
            "a result capture, which the engine's gate control owns as the last-message door"
        }
        _ => match node.bears_capability() {
            Ok(false) => return None,
            Ok(true) => "which bears a capability the realm grants and the engine composes",
            Err(_) => "a configuration assignment with no bounded meaning",
        },
    })
}

/// The placeholder an adapter's gate fragment captures into, which the
/// engine fills with the result path it owns at dispatch.
pub const RESULT_PATH: &str = "{result_path}";

/// A refusal of an inline Codex launch as admission and the dispatch door
/// both word it (rebuild unit 5d-fix-c1, chief F4 of run
/// `0065-rebuild-unit-5d-fix-b-see-t-8067eebc`): the seat in the one
/// bounded representation, [`bounded_site`], beside the value-free cause of
/// [`judge_inline_codex_launch`], the judgment both boundaries call.
///
/// [`judge_inline_codex_launch`]: brokkr_protocol::native_controls::grammar::judge_inline_codex_launch
pub(crate) fn inline_codex_refusal(site: &str, cause: &impl std::fmt::Display) -> String {
    format!(
        "the inline Codex launch of seat {} {cause}",
        bounded_site(site)
    )
}

/// Rebuild unit 5b (design D5.3, D5.7): the one inline shape whose typed
/// allow the engine delivers — a command that dispatches the claude or
/// lanetally driver, with no hands and no capability-bearing option of its
/// author's (rebuild unit 5b-fix), whose adapter maps every name. The list
/// is lowered by the same function that lowers an agent's, and every other
/// shape refuses with its own cause. Beside it comes the adapter's declared
/// permission template, which the engine emits here as it does for an
/// agent (rebuild unit 5c), or `None` where the adapter declares none, and
/// the permission flag the list lowered onto (rebuild unit 14a1).
fn lower_inline_allow(
    what: &str,
    raw: &Value,
    command: &[String],
    allow: &[String],
    adapters: Option<&crate::agents::Adapters>,
) -> Result<InlineAllow, CompileError> {
    let refuse = |cause: String| {
        CompileError::Invalid(format!("seat '{what}' declares 'tools.allow' {cause}"))
    };
    let driver = match dispatch_driver(command) {
        Some(kind) if kind == "claude" || kind == "lanetally" => kind,
        other => {
            let dispatches = match other {
                Some(kind) => format!("dispatches the '{kind}' driver"),
                None => "dispatches no built-in driver".to_string(),
            };
            return Err(refuse(format!(
                "on an inline site whose command {dispatches}; the engine lowers a typed local \
                 allow into an inline command only for the claude and lanetally drivers, whose \
                 adapters map it onto their tool permissions, so here the restriction would be \
                 recorded and not delivered — it is kept exactly and refused rather than run \
                 unrestricted (decision 0065 slice one, design D5.3); an authored flag cannot \
                 stand in for it"
            )));
        }
    };
    if raw.get("hands").is_some() {
        return Err(refuse(
            "beside the site's own hands; hands replace the harness's tools, so a direct local \
             list at an inline site would stand beside the box's restriction rather than express \
             it — it is kept exactly and refused (decision 0065 slice one, design D5.3)"
                .to_string(),
        ));
    }
    // Operator ruling 1 of 2026-09-23: the engine composes the typed list
    // as its own contribution, and nothing it composes is merged with or
    // ordered against a capability control of the author's, read under the
    // harness's own grammar.
    let grammar = brokkr_protocol::native_controls::grammar::grammar(&driver)
        .expect("the claude and lanetally grammars are modelled");
    let authored = grammar
        .parse(brokkr_protocol::native_controls::harness_arguments(command))
        .map_err(|problem| {
            // The problem's rendering quotes its token, and a joined or
            // misplaced token carries its value (rebuild unit 5b-fix2, S2):
            // only the position, a bounded label and the grammar's cause,
            // built from fixed text and canonical option names, are named.
            refuse(format!(
                "while its authored command cannot be read: the '{}' command grammar cannot \
                 place argument {} ({}), whose token is not echoed because it can carry a value: \
                 it {}. A control nobody can read is a control nobody can rule on, so it is \
                 refused rather than passed through (decision 0066 ruling 6; operator ruling 1 \
                 of 2026-09-23)",
                grammar.harness,
                problem.at + 1,
                unplaced_label(grammar, &problem.token),
                problem.cause
            ))
        })?;
    if let Some((node, kind)) = authored
        .nodes
        .iter()
        .find_map(|node| authored_capability_control(node).map(|kind| (node, kind)))
    {
        return Err(refuse(format!(
            "while its authored command carries '{}' (argument {}), {kind}; the engine composes \
             the typed list as its own contribution and a recipe authors no capability-bearing \
             option beside it, so the site is refused rather than reconciled (operator ruling 1 \
             of 2026-09-23; decision 0065 slice one, design D5.3)",
            node.name(),
            node.at + 1
        )));
    }
    let adapter = adapters
        .and_then(|adapters| adapters.adapter(&driver))
        .ok_or_else(|| {
            refuse(format!(
                "for driver '{driver}', which no loaded adapter declares; with no tool permission \
                 mapping the restriction cannot be expressed, so it is refused rather than run \
                 unrestricted (decision 0065 slice one, design D5.3)"
            ))
        })?;
    let refused = |cause: String| CompileError::Invalid(format!("seat '{what}': {cause}"));
    let lowered = crate::agents::lower_allow(adapter, allow, "site").map_err(refused)?;
    let template = crate::agents::inline_template(adapter, &driver).map_err(refused)?;
    Ok((
        lowered,
        template,
        crate::agents::declared_permissions(adapter),
    ))
}

/// What [`lower_inline_allow`] lowered: the list, the adapter's template
/// and the permission flag the list lowered onto.
type InlineAllow = (
    crate::agents::LocalLowering,
    Option<Segment>,
    Option<brokkr_protocol::native_controls::ListFlag>,
);

/// What a capability-bearing option an author wrote at an inline typed
/// site is (operator ruling 1 of 2026-09-23; rebuild units 5b-fix and
/// 5b-fix2), or `None` for an option that bears no capability. Judged on
/// the parsed node, so every spelling of one option — split, `=`-joined,
/// an alias, repeated or variadic — is judged at once, and its value is
/// never echoed and, but for an effort's, never read: a permission mode is
/// refused whichever mode it names. Web
/// and search reach Claude only as tool names, which ride a list; an
/// option the grammar does not model never parses. The whole sweep of the
/// Claude/LaneTally grammar, with the options judged inert and why, is
/// recorded in evidence.md ("Unit 5b-fix2").
///
/// An effort is the one option whose VALUE decides it (rebuild unit
/// 5b-fix3, R2): the CLI reference's `ultracode` requests `xhigh` with
/// workflows turned on, so only the reference's plain levels stand, by this
/// fixed classification and never by an adapter's declaration — adding a
/// name to adapter data does not make its meaning inert — and the refusal
/// names the fixed levels, never the value or the adapter's list (R1).
fn authored_capability_control(
    node: &brokkr_protocol::native_controls::grammar::Node,
) -> Option<&'static str> {
    use brokkr_protocol::native_controls::grammar::Effect;
    /// The effort values the CLI reference gives as a level and nothing
    /// more (https://code.claude.com/docs/en/cli-reference, `--effort`).
    const PLAIN_EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
    Some(match (node.spec.effect, node.name()) {
        (Effect::List(_), _) => "a tool list",
        (Effect::Load | Effect::Config, _) => {
            "which loads or configures a server, a plugin or a settings document"
        }
        (Effect::Session, _) => {
            "a session selector, and a rejoined session restores its saved working directory"
        }
        (_, "--permission-mode") => "a permission mode",
        (_, "--strict-mcp-config") => "an MCP configuration control",
        (_, "--add-dir") => "an additional directory, which grants file access",
        (_, "--bg") => {
            "a background session, which runs under a supervisor the engine does not launch"
        }
        (_, "--input-format") => {
            "an input format, whose streamed input can carry control messages the engine does not \
             compose"
        }
        (_, "--effort")
            if !node
                .values
                .iter()
                .all(|value| PLAIN_EFFORTS.iter().any(|plain| plain == value)) =>
        {
            "an effort other than the reference's plain levels (low, medium, high, xhigh, max), \
             which can turn on more than effort"
        }
        _ => return None,
    })
}

/// A bounded, value-free label for the token a grammar could not place
/// (rebuild unit 5b-fix2, S2): the modelled option it names, read before
/// any `=` and named by its canonical spelling whichever alias was written
/// (rebuild unit 5b-fix3, R3), or what kind of token it is. An unmodelled
/// name is authored text of any length, so it is never echoed either.
fn unplaced_label(
    grammar: &brokkr_protocol::native_controls::grammar::Grammar,
    token: &str,
) -> String {
    if !token.starts_with('-') || token == "-" {
        return "a bare word".to_string();
    }
    let name = token.split_once('=').map_or(token, |(name, _)| name);
    grammar
        .options
        .iter()
        .find(|spec| spec.canonical == name || spec.aliases.contains(&name))
        .map_or_else(
            || format!("an option the '{}' grammar does not model", grammar.harness),
            |spec| format!("'{}'", spec.canonical),
        )
}

/// Which contribution [`expressed_sandbox`] judges (design D5.6): bytes an
/// author or the selected hands fragment wrote, or the resolved native plan,
/// which alone may also write the exact key its measured denial uses. The
/// context selects that one allowance; it never skips a competing-control
/// check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Contribution {
    Written,
    Native,
}

/// The one `--sandbox` class an argv expresses under the codex grammar, or
/// `None` where it names none. Read through the public protocol grammar
/// rather than by token matching, so a joined, attached or aliased spelling
/// is the same option. Anything the grammar cannot place refuses — an
/// unreadable contribution is uncertainty, and uncertainty refuses typed
/// admission — naming its position and never its token. So does a COMPETING control beside the class (design D5.3,
/// made explicit by D5.5): a switch that lifts or replaces the sandbox
/// (`--full-auto`, `--dangerously-bypass-approvals-and-sandbox`), or a
/// configuration assignment into `sandbox_mode` or `sandbox_workspace_write`,
/// which is the same control through an opaque door, or `--add-dir`, which
/// adds a filesystem root the class would not reach (review return S1: a
/// control on the same reach, in the split or the `=` spelling, whatever
/// its value). So does an OPAQUE
/// contribution (review return F2): an option the grammar types as a load
/// (`--profile`) reads a whole configuration document the engine cannot
/// see into, and a configuration assignment outside the two tables the
/// shipped fragments are established to write — the hands transport under
/// `mcp_servers.brokkr` and the effort under `model_reasoning_effort` — is
/// unqualified; either could set the same control, so neither leaves a
/// class checkable. These are refused wherever they stand — the selected
/// fragment, the authored command or the resolved native plan — and never
/// reconciled by argument order or trusted for their provenance. So is the
/// root selector `--cd` in every spelling the grammar reads as it (`--cd
/// PATH`, `--cd=PATH`, `-C PATH`, `-CPATH`; unit 2-fix A1): whatever its
/// value — the current workspace included — it moves what the class is
/// measured from, and which of two selectors a harness honours is not
/// established, so it is refused rather than ordered. `whom` names the
/// requester: an agent's link (`seat 'x' link 1`), or an inline seat
/// (`seat 'x'`, rebuild unit 5d).
fn expressed_sandbox(
    whom: &str,
    part: &str,
    argv: &[String],
    contribution: Contribution,
) -> Result<Option<String>, CompileError> {
    use brokkr_protocol::native_controls::grammar;
    /// The two codex switches whose effect on the sandbox is not a class.
    const SANDBOX_SWITCHES: [&str; 2] =
        ["--full-auto", "--dangerously-bypass-approvals-and-sandbox"];
    /// The two configuration tables that reach the same control.
    const SANDBOX_TABLES: [&str; 2] = ["sandbox_mode", "sandbox_workspace_write"];
    /// The option that widens the sandbox's reach by a root, whose value
    /// is authored bytes and is never echoed.
    const ADDED_ROOT: &str = "--add-dir";
    /// The option that selects the root itself, canonical for `-C`.
    const ROOT_SELECTOR: &str = "--cd";
    /// The configuration keys an existing fragment is ESTABLISHED to write
    /// (design D5.3): the boxed hands transport, as a table, and the
    /// effort assignment, exactly. Every other assignment is unqualified.
    const ESTABLISHED_TABLES: [&str; 1] = ["mcp_servers.brokkr"];
    const ESTABLISHED_KEYS: [&str; 1] = ["model_reasoning_effort"];
    /// The one further key a RESOLVED NATIVE plan is established to write
    /// (design D5.6): the measured web-search denial `-c`,
    /// `web_search="disabled"`, exactly this key — not a table, not a
    /// descendant, and never in written bytes.
    const NATIVE_KEY: &str = "web_search";
    let command = match grammar::parse("codex", argv).expect("the codex grammar is modelled") {
        Ok(command) => command,
        // The problem's rendering quotes its token, and an attached or
        // joined token carries its value (unit 2-fix review return S2): only
        // the position and the grammar's cause, which is built from fixed
        // text and canonical option names alone, are named.
        Err(problem) => {
            let (argument, cause) = (problem.at + 1, problem.cause);
            return Err(CompileError::Invalid(format!(
                "{whom} requests a typed 'tools.sandbox', but the {part} it \
                 would be judged against cannot be read: the 'codex' command grammar cannot place \
                 argument {argument}, whose token is not echoed because it can carry a value: it \
                 {cause} — refused (design D5.3)"
            )));
        }
    };
    let mut expressed = None;
    for node in &command.nodes {
        if node.name() == "--sandbox" {
            expressed = node.values.first().cloned();
        }
        if let Some(switch) = SANDBOX_SWITCHES.iter().find(|name| node.name() == **name) {
            return Err(CompileError::Invalid(format!(
                "{whom} requests a typed 'tools.sandbox', but the {part} \
                 carries `{switch}`, a switch that lifts or replaces the sandbox a `--sandbox` \
                 class would express, so no typed class can be checked against it — refused \
                 (design D5.3)"
            )));
        }
        if node.name() == ADDED_ROOT {
            return Err(CompileError::Invalid(format!(
                "{whom} requests a typed 'tools.sandbox', but the {part} \
                 carries `{ADDED_ROOT}`, which adds a filesystem root the `--sandbox` class \
                 would not reach, a competing control on the same reach that no typed class can \
                 be checked against — refused (design D5.3)"
            )));
        }
        if node.name() == ROOT_SELECTOR {
            return Err(CompileError::Invalid(format!(
                "{whom} requests a typed 'tools.sandbox', but the {part} \
                 carries `{ROOT_SELECTOR}`, which selects the root the `--sandbox` class is \
                 measured from, a competing root control that no typed class can be checked \
                 against whatever its value or position — refused (design D5.3)"
            )));
        }
        if node.spec.effect == grammar::Effect::Load {
            let option = node.name();
            return Err(CompileError::Invalid(format!(
                "{whom} requests a typed 'tools.sandbox', but the {part} \
                 carries `{option}`, which loads an opaque configuration document the engine \
                 cannot see into and that can set the same control, so no typed class can be \
                 checked against it — refused (design D5.3)"
            )));
        }
        if node.spec.effect == grammar::Effect::Config {
            if let Some(table) = SANDBOX_TABLES.iter().find(|table| {
                node.values
                    .iter()
                    .any(|value| grammar::config_under(&grammar::config_key(value), table))
            }) {
                // Only written bytes reach here: a resolved native plan's
                // every declared assignment passed the bounded reader at
                // adapter load, which admits no sandbox table (rebuild units
                // 11 and 12).
                return Err(CompileError::Invalid(format!(
                    "{whom} requests a typed 'tools.sandbox', but the {part} \
                     assigns '{table}' through the harness's configuration, a second door to the \
                     same control that no typed class can be checked against — refused (design \
                     D5.3)"
                )));
            }
            // An assignment outside the established keys is not echoed:
            // its key is authored bytes, and only its position is named. A
            // resolved native plan's assignments are all established ones
            // (rebuild units 11 and 12), so only written bytes reach here.
            let established = node.values.iter().all(|value| {
                let key = grammar::config_key(value);
                ESTABLISHED_TABLES
                    .iter()
                    .any(|table| grammar::config_under(&key, table))
                    || ESTABLISHED_KEYS.contains(&key.as_str())
                    || (contribution == Contribution::Native && key == NATIVE_KEY)
            });
            if !established {
                let at = node.at;
                return Err(CompileError::Invalid(format!(
                    "{whom} requests a typed 'tools.sandbox', but the {part} \
                     assigns configuration at argument {at} outside the keys an existing fragment \
                     is established to write (the hands transport under 'mcp_servers.brokkr' and \
                     the effort 'model_reasoning_effort'); an unqualified assignment could reach \
                     the same control, so no typed class can be checked against it — refused \
                     (design D5.3)"
                )));
            }
        }
    }
    Ok(expressed)
}

/// The one class design D5.3's table admits on a serving path, keyed on
/// the path alone — the boundary and the seat's class — and never on
/// adapter data (review return F1): a box and a harness gate hold
/// read-only, harness work holds workspace-write. The selected fragment
/// is the path's REPRESENTATION and must express this class exactly; it
/// is not an authority, so an adapter whose fragment expresses a wider
/// class does not widen the table.
fn admitted_sandbox(boundary: Boundary, seat_class: SeatClass) -> (Sandbox, &'static str) {
    if boundary.is_boxed() {
        (Sandbox::ReadOnly, "a boxed site")
    } else {
        match seat_class {
            SeatClass::Gate => (Sandbox::ReadOnly, "a harness gate"),
            SeatClass::Work => (Sandbox::WorkspaceWrite, "a harness work seat"),
        }
    }
}

/// Design D5.3: a typed `tools.sandbox` is admitted only where an EXISTING
/// engine fragment already expresses exactly that class, and refused
/// everywhere else — never dropped, clamped or called narrower. Runs after
/// every standing refusal of the site (the hands law, the gate tier, the
/// judges list, the egress bar), so those keep their precedence. The
/// admitted shapes are actual codex dispatch with hands: `hands.workspace`
/// under a boxed boundary, `hands.harness.gate` for a gate and
/// `hands.harness.work` for a work seat under `harness`. Every link of the
/// chain is judged, so a later candidate cannot hide behind the primary.
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn admit_local_sandbox(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    law: SiteLaw<'_>,
    adapters: Option<&Adapters>,
    sites: &BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let Some(requested) = sites
        .get(what)
        .and_then(|facts| facts.local.as_ref())
        .and_then(|local| local.sandbox)
    else {
        return Ok(());
    };
    // Rebuild unit 5d: an inline Codex seat's class was judged and lowered
    // where it was recorded (`lower_inline_sandbox`), onto the engine's own
    // control; the standing refusals above have had their say. No inline
    // site records a class any other way: `record_inline_tools` lowers it
    // or refuses the site (operator ruling of 2026-09-30, unit 26c), so no
    // guard on the lowering stands here, where the exact-coverage gate would
    // count it unreachable. The removal control is
    // `an_inline_site_records_a_checked_empty_declaration_and_refuses_each_nonempty_field`,
    // and a class recorded without its lowering still refuses at dispatch
    // (`the_dispatch_door_admits_only_the_record_sealed_for_its_spawn`).
    if candidates.is_empty() {
        return Ok(());
    }
    let class = requested.name();
    let boundary = law.boundary;
    if law.agent_hands.is_none() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' requests 'tools.sandbox' '{class}' without hands; no engine path \
             expresses a sandbox class for a site without hands, so the class would be recorded \
             and not delivered — refused under the `{boundary}` boundary until decision 0065 \
             slice one's lowering proves it (design D5.3)"
        )));
    }
    let seat_class = parse_class(what, raw)?;
    let adapters = adapters.expect("an agent-resolved site opened the adapters (needs_adapters)");
    for (index, candidate) in candidates.iter().enumerate() {
        let link = index + 1;
        let provider = &candidate.provider;
        // The HARNESS is what the command dispatches, read off the command
        // itself: a provider label is not evidence of a sandbox class.
        let harness = dispatch_driver(&candidate.argv)
            .unwrap_or_else(|| crate::capabilities::OPAQUE_HARNESS.to_string());
        if harness != "codex" {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' link {link} requests 'tools.sandbox' '{class}' but dispatches the \
                 '{harness}' harness through provider '{provider}'; only the codex harness's own \
                 `--sandbox` fragments express a sandbox class today, and a provider label, a \
                 permission mode or an unmodelled driver is not evidence of one — refused under \
                 the `{boundary}` boundary (design D5.3)"
            )));
        }
        let adapter = adapters
            .adapter(provider)
            .expect("resolution mapped every link of the chain");
        let (part, fragment): (&str, &[String]) = if boundary.is_boxed() {
            ("`hands.workspace` fragment", &candidate.hands_fragment)
        } else if boundary == Boundary::Harness {
            match seat_class {
                SeatClass::Gate => (
                    "`hands.harness.gate` fragment",
                    adapter.harness.gate.as_deref().unwrap_or(&[]),
                ),
                SeatClass::Work => (
                    "`hands.harness.work` fragment",
                    adapter.harness.work.as_deref().unwrap_or(&[]),
                ),
            }
        } else {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' link {link} requests 'tools.sandbox' '{class}' under the `open` \
                 boundary, where a work seat runs at the harness's own default and no engine \
                 fragment expresses a class; a presumed provider default is not a representation \
                 — refused (design D5.3)"
            )));
        };
        if fragment.is_empty() {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' link {link} requests 'tools.sandbox' '{class}' under the \
                 `{boundary}` boundary, but provider '{provider}' supplies no {part} to express \
                 it; a missing fragment is not a representation — refused (design D5.3)"
            )));
        }
        let whom = format!("seat '{what}' link {link}");
        match expressed_sandbox(&whom, part, fragment, Contribution::Written)? {
            Some(found) if found == class => {
                // The fragment represents the class; the TABLE decides
                // whether this path may hold it at all (review return F1).
                let (admitted, site_kind) = admitted_sandbox(boundary, seat_class);
                if requested != admitted {
                    let admitted = admitted.name();
                    return Err(CompileError::Invalid(format!(
                        "seat '{what}' link {link} requests 'tools.sandbox' '{class}' at \
                         {site_kind} under the `{boundary}` boundary, where decision 0065 slice \
                         one admits only '{admitted}' (design D5.3: a box and a harness gate hold \
                         read-only, harness work holds workspace-write); the {part} of provider \
                         '{provider}' expresses '{class}' too, but adapter data is a \
                         representation and not an authority, so a fragment cannot widen that \
                         table — refused"
                    )));
                }
            }
            Some(found) => {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' link {link} requests 'tools.sandbox' '{class}', but the {part} \
                     the engine selects for provider '{provider}' under the `{boundary}` boundary \
                     expresses '{found}'; a fragment is neither called narrower nor clamped, the \
                     typed class must match it exactly — refused (design D5.3)"
                )))
            }
            None => {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' link {link} requests 'tools.sandbox' '{class}', but the {part} \
                     the engine selects for provider '{provider}' under the `{boundary}` boundary \
                     names no `--sandbox` class at all; a fragment that expresses nothing is not a \
                     representation — refused (design D5.3)"
                )))
            }
        }
        // The other contributions must not carry a competing control: the
        // authored part of the command is read after brokkr's own dispatch
        // tokens, under the same grammar.
        let (authored, _) = candidate.parts();
        let authored = brokkr_protocol::native_controls::harness_arguments(authored);
        if let Some(found) =
            expressed_sandbox(&whom, "authored command", authored, Contribution::Written)?
        {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' link {link} requests 'tools.sandbox' '{class}', but the authored \
                 command of provider '{provider}' already carries `--sandbox` '{found}', a \
                 competing control the selected {part} would stand beside; authored bytes cannot \
                 supply or contest a typed representation — refused under the `{boundary}` \
                 boundary (design D5.3)"
            )));
        }
    }
    Ok(())
}

/// Design D5.6 (unit 2-fix S1): the RESOLVED native plan of every link is
/// the third contribution a typed class is judged beside. It exists only
/// after resolution, so this runs where the plans are sealed and before
/// their notices and facts are published, after [`admit_local_sandbox`]
/// has already admitted the matching hands fragment. The whole argv each
/// plan resolved to — the selected ON or OFF and any substituted
/// restriction transport — is read once through the same typed decoder the
/// driver reads it with, and judged by the same guard: no disposition label
/// or engine provenance exempts it, and a valid denial beside a competing
/// control does not end the scan. Only the selected hands fragment
/// represents the class, so any native `--sandbox`, matching or not,
/// competes. An inline Codex seat's plan is judged with its whole launch by
/// [`admit_inline_launch`] instead.
fn admit_native_sandbox(
    what: &str,
    requested: Sandbox,
    site: &crate::capabilities::SiteCapabilities,
) -> Result<(), CompileError> {
    let class = requested.name();
    for (index, outcome) in site.outcomes.iter().enumerate() {
        let link = index + 1;
        let whom = format!("seat '{what}' link {link}");
        let provider = &outcome.provider;
        let plan = brokkr_protocol::native_controls::managed(
            &json!({"native_controls": outcome.controls()}),
        )
        .map_err(CompileError::Invalid)?
        .expect("the plan is read from under its own key");
        let part = "resolved native control argv";
        if expressed_sandbox(&whom, part, &plan.argv, Contribution::Native)?.is_some() {
            return Err(CompileError::Invalid(format!(
                "{whom} requests 'tools.sandbox' '{class}', but the {part} of provider \
                 '{provider}' carries `--sandbox`, a second sandbox control beside the selected \
                 hands fragment; only that fragment represents a typed class, so even a matching \
                 native class competes — refused (design D5.3)"
            )));
        }
    }
    Ok(())
}

/// Rebuild unit 5d-fix-b (chief F1 and F2; design D5.3): the admission of an
/// inline Codex seat's whole launch over its compiled plan — the authored
/// command, the recorded template, the engine's `local` fragment and each
/// resolved native plan, in the order dispatch composes them — by
/// [`grammar::judge_inline_codex_launch`], the same judgment the dispatch
/// door runs (rebuild unit 5d-fix-c1). The capture follows the admitted
/// class: a `read-only` class is a gate's and captures into the
/// engine-owned result path. The seat is named bounded ([`bounded_site`]).
fn admit_inline_launch(
    what: &str,
    parts: &[String],
    facts: &SiteFacts,
    site: &crate::capabilities::SiteCapabilities,
) -> Result<(), CompileError> {
    use brokkr_protocol::native_controls::grammar;
    let Some(lowered) = &facts.inline_sandbox else {
        return Ok(());
    };
    let capture = (lowered.class == Sandbox::ReadOnly).then_some(RESULT_PATH);
    for outcome in &site.outcomes {
        let plan = brokkr_protocol::native_controls::managed(
            &json!({"native_controls": outcome.controls()}),
        )
        .map_err(CompileError::Invalid)?
        .expect("the plan is read from under its own key");
        let segments: Vec<Segment> = std::iter::once(Segment::new(
            Origin::Authored,
            brokkr_protocol::native_controls::harness_arguments(parts),
        ))
        .chain(facts.inline_template.clone())
        .chain([
            lowered.segment.clone(),
            Segment::new(Origin::Native, &plan.argv),
        ])
        .collect();
        grammar::judge_inline_codex_launch(lowered.class.intent(), &segments, capture).map_err(
            |cause| {
                CompileError::Invalid(format!(
                    "seat {} declares 'tools.sandbox' '{}', but {} (operator ruling of \
                     2026-09-25; rebuild unit 5d-fix-b; design D5.3)",
                    bounded_site(what),
                    lowered.class.name(),
                    inline_codex_refusal(what, &cause)
                ))
            },
        )?;
    }
    Ok(())
}

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
/// matched on — a bundle may spell it `{brokkr}` or the absolute path
/// of the binary it means, and both are the same dispatch. `None` for any other shape: a raw process is a driver that
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
    law: SiteLaw<'_>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    // The hands law is the FIRST statement (decision 0046 ruling 4;
    // design DD22): it judges every site that declares hands whatever
    // its class, so the work-class early return below cannot bypass it,
    // and it reads the adapters only on the chain path, where the
    // `agent` key opened them — a bundle whose only hands site is a
    // work-class exec seat compiles with no `adapters/` in sight, as
    // `needs_adapters` says it may.
    enforce_hands_boundary(
        what,
        raw,
        candidates,
        law,
        agents.as_ref().map(|context| &context.adapters),
    )?;
    enforce_route_policy(what, raw, candidates, secrets, agents, sites)?;
    // Design D5.3, LAST: a typed sandbox class is admitted only where an
    // existing engine fragment expresses it exactly, after every standing
    // refusal above has had its say, so none of them loses precedence.
    admit_local_sandbox(
        what,
        raw,
        candidates,
        law,
        agents.as_ref().map(|context| &context.adapters),
        sites,
    )
}

/// Decision 0021's two prohibitions proper — the gate tier, the judges
/// list and the egress bar — at one site, after the hands law has spoken.
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn enforce_route_policy(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    secrets: &[String],
    agents: &mut Option<AgentContext>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let class = tier::admitted(what, raw, candidates, agents.as_ref())?;
    if class == SeatClass::Work && secrets.is_empty() {
        return Ok(());
    }
    // Read the adapter data without writing beside it: the inline
    // witness now lands in the one canonical site table (design D10 F1),
    // so no split borrow of the context is needed.
    let context = agents
        .as_ref()
        .expect("a gate-class or secret-binding seat opens the adapters");
    let adapters = &context.adapters;
    let minimum = context.egress_minimum;
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
        site_facts(sites, what).driver = Some(authorised);
    }
    Ok(())
}

/// What the hands law reads beside a site (decision 0046 ruling 4;
/// design DD22): the boundary the bundle compiles under, the directory
/// of the layer that DECLARED the site — the one its `./` resolves
/// against and the one the manifest walk digests — and the agent's hands
/// when the site names an agent.
#[derive(Clone, Copy)]
struct SiteLaw<'a> {
    boundary: Boundary,
    dir: &'a Path,
    agent_hands: Option<&'a HandsSpec>,
}

/// The hands law (decision 0046 ruling 4; design DD22): one function,
/// total over class, that rules what a site's hands MEAN under the
/// boundary the bundle compiles under. A site without hands returns at
/// once. Under a boundary Brokkr builds (`namespace`, `seatbelt`,
/// `container`) it returns at once too: what hands mean under a box is
/// decision 0043's law, unchanged. Under `harness` and `open` three
/// arms:
///
/// - an inline exec site is admitted only for the bundle's own pinned
///   script, judged by [`pinned_script`] against the declaring layer —
///   work or gate, the class unread (proposal D32);
/// - an inline model site is refused naming the repair: its argv is the
///   author's and carries the box's own tokens (D10);
/// - an agent-resolved site is judged by its chain against the adapters
///   the `agent` key opened: under `harness` every link must declare the
///   `hands.harness` fragment its class selects — `gate` for a gate,
///   `work` for a work seat (DD7) — and under `open` a gate is refused
///   while a work seat runs at the harness's default (D11). The class is
///   read here and nowhere else in the law, because here it selects a
///   fragment.
fn enforce_hands_boundary(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    law: SiteLaw<'_>,
    adapters: Option<&Adapters>,
) -> Result<(), CompileError> {
    let has_hands = law.agent_hands.is_some() || raw.get("hands").is_some();
    if !has_hands || law.boundary.is_boxed() {
        return Ok(());
    }
    let boundary = law.boundary;
    if candidates.is_empty() {
        let parts = command_parts(raw);
        return match dispatch_driver(&parts).as_deref() {
            Some("exec") => pinned_script(law.dir, &parts).map(drop).map_err(|problem| {
                CompileError::Invalid(format!(
                    "seat '{what}' declares hands under the `{boundary}` boundary, where no \
                     box stands, so its exec command must name the bundle's own pinned \
                     script: {problem} (decision 0046 ruling 4; decision 0021)"
                ))
            }),
            Some(driver) => Err(CompileError::Invalid(format!(
                "seat '{what}' is an inline `{driver}` site that declares hands under the \
                 `{boundary}` boundary; its argv is the author's own and carries the \
                 box's tokens, which no harness stands behind unboxed. Seat it through an \
                 agent, whose adapter declares how the harness stands under `harness`, or \
                 run the realm under a boxed boundary (decision 0046 ruling 4)"
            ))),
            None => Err(CompileError::Invalid(format!(
                "seat '{what}' declares hands under the `{boundary}` boundary, where no box \
                 stands, and its command is a bare program rather than a `{{brokkr}} driver \
                 exec` dispatch of the bundle's own pinned script (decision 0046 ruling 4; \
                 decision 0021)"
            ))),
        };
    }
    let class = parse_class(what, raw)?;
    let adapters = adapters.expect("an agent-resolved site opened the adapters (needs_adapters)");
    for (index, candidate) in candidates.iter().enumerate() {
        let link = index + 1;
        let provider = &candidate.provider;
        // The adapter is the one the resolution mapped; the Candidate
        // carries its `hands.harness` so the engine, which holds no
        // adapter at spawn, can compose from it too.
        let harness = &adapters
            .adapter(provider)
            .expect("resolution mapped every link of the chain")
            .harness;
        // Two words reach here — the boxed three returned above — and
        // the class selects the fragment, which is the one thing ruling
        // 1 gives class to do under `harness`.
        let under_harness = boundary == Boundary::Harness;
        match class {
            SeatClass::Gate if !under_harness => {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' is a gate with hands under the `open` boundary, where \
                     nothing at all stands between a model's hands and the machine; `open` \
                     never holds a model gate (decision 0046 ruling 4)"
                )));
            }
            SeatClass::Gate => {
                if harness.gate.is_none() {
                    return Err(CompileError::Invalid(format!(
                        "seat '{what}' gate link {link} resolves to provider '{provider}', \
                         which declares no `hands.harness.gate` fragment{}; under the \
                         `harness` boundary a model may judge only under its harness's own \
                         read-only sandbox as the adapter addresses it (decision 0046 ruling 4)",
                        measured(&harness.gate_gap)
                    )));
                }
            }
            SeatClass::Work if under_harness => {
                if harness.work.is_none() {
                    return Err(CompileError::Invalid(format!(
                        "seat '{what}' link {link} resolves to provider '{provider}', which \
                         declares no `hands.harness.work` fragment{}: a capability gap — \
                         under the `harness` boundary a work seat with hands writes the tree \
                         only under the harness's own writable sandbox as the adapter \
                         addresses it (decision 0046 rulings 1 and 4)",
                        measured(&harness.work_gap)
                    )));
                }
            }
            // A work seat under `open` runs at the harness's default;
            // whether that default writes is the harness's fact (D11).
            SeatClass::Work => {}
        }
    }
    Ok(())
}

/// The measured reason beside a missing `hands.harness` member, when the
/// operator recorded one.
fn measured(gap: &Option<String>) -> String {
    match gap {
        Some(reason) => format!(" ({reason})"),
        None => String::new(),
    }
}

/// The bundle's own pinned script, read off a RAW exec dispatch before
/// `expand_command` erases the `./` spelling (decision 0046 ruling 4;
/// design DD9): after the `--` that ends `{brokkr} driver exec`, zero or
/// more bare interpreter names, then exactly one `./`-relative script
/// token, then arguments nobody judges. The token's components are plain
/// names — no `..`, no `.`, no empty component, no `\`, no drive or UNC
/// prefix, and none of the measured startup metacharacters (0048) — and, joined
/// to the declaring layer's directory, a regular
/// file by `metadata` (following a symlink, as the manifest walk does)
/// that the walk pins. Nothing is canonicalised and no two spellings are
/// compared: a token spelled any other way is refused as not `./`-relative.
/// This checks bundle bytes: under `harness` and `open` the interpreter
/// is unpinned and resolved through inherited PATH, so admission defends
/// a careless bundle, not a hostile seat (decision 0049 ruling 3).
///
/// Returns the manifest key the script is pinned under.
fn pinned_script(dir: &Path, parts: &[String]) -> Result<String, String> {
    let after = parts
        .iter()
        .position(|part| part == "--")
        .map(|index| &parts[index + 1..])
        .ok_or_else(|| "the dispatch carries no `--` after `exec`".to_string())?;
    for token in after {
        if let Some(relative) = token.strip_prefix("./") {
            return pinned_key(dir, token, relative);
        }
        if token.starts_with('-') {
            return Err(format!(
                "'{token}' is an option token before the script; interpreters take no \
                 options here (`bash -c` would run text, not pinned bytes)"
            ));
        }
        if token.contains(['/', '\\']) {
            return Err(format!(
                "'{token}' is spelled as a path that is not `./`-relative to the bundle; \
                 only a `./` script the manifest walk pins is admitted unboxed"
            ));
        }
        // A bare interpreter name (`bash`, `python3`, `true`): walked
        // past, until a script token or the end of the argv.
    }
    Err(format!(
        "the command names no `./`-relative script after its interpreters ({})",
        after.join(" ")
    ))
}

/// The lookup half of [`pinned_script`]: the components of one `./`
/// token, and the file they name under the declaring layer.
fn pinned_key(dir: &Path, token: &str, relative: &str) -> Result<String, String> {
    let components: Vec<&str> = relative.split('/').collect();
    // Closed against the interpreters measured in decision 0048 only:
    // * ? [ ] select glob matches; { } expand brace alternatives; ( )
    // trigger MSYS globify too; ' " delimit quotes and \ escapes them;
    // ~ introduces tilde expansion; ` is removed by powershell.exe;
    // CR/LF split MSYS arguments but Rust does not autoquote them.
    // A different interpreter may reinterpret other bytes. The execution
    // guarantee rests on a boundary supplying the filesystem and PATH,
    // never on this set (0049 ruling 3; 0046 ruling 4). The backtick is
    // a small engineering refusal of the reviewed route, with no guarantee.
    // Refuse each even unmatched or in a
    // later component: admission reads bundle bytes, never the host OS,
    // interpreter, environment, or which matching siblings exist today.
    // This changes admission only; walk_files and the canonical pin keep
    // the exact filename bytes. See 0048 for evidence and scope.
    const STARTUP_METACHARACTERS: &str = "*?[]{}()'\"\\~`\r\n";
    for component in &components {
        if let Some(character) = component
            .chars()
            .find(|c| STARTUP_METACHARACTERS.contains(*c))
        {
            return Err(format!(
                "'{token}' has component {component:?} containing refused startup character \
                 {character:?}; rename the component to meet the pinned-script admission \
                 rule (decision 0048; decision 0046 ruling 4)"
            ));
        }
    }
    let plain = |component: &&str| {
        !component.is_empty() && *component != "." && *component != ".." && !component.contains(':')
    };
    if !components.iter().all(plain) {
        return Err(format!(
            "'{token}' is not a plain `./`-relative path: every component is a plain name, \
             never `..`, `.`, empty, a `\\` or a drive"
        ));
    }
    if unpinned_top_level(components[0]) {
        return Err(format!(
            "'{token}' names a path the manifest walk does not pin ({} is workspace data, \
             never bundle bytes)",
            components[0]
        ));
    }
    let mut path = dir.to_path_buf();
    for component in &components {
        path.push(component);
    }
    match std::fs::metadata(&path) {
        Ok(meta) if meta.is_file() => Ok(components.join("/")),
        _ => Err(format!(
            "'{token}' names no regular file under the declaring layer {}",
            dir.display()
        )),
    }
}

/// The two top-level names the manifest walk skips (decision 0042): a
/// scaffold may also be the workspace brokkr is invoked from, and its
/// realm map and dialect library are workspace declarations pinned into
/// the RUN manifest, never bundle files. One function, shared by the walk
/// and by the pinned-script lookup, so the two cannot drift.
///
/// Decision 0065 adds the operator's abstract capability definitions on
/// the same terms: the ones a compile CONSULTS are pinned by name and
/// digest in the manifest's `capabilities` section, so walking the whole
/// directory would make an unconsulted definition a second source of
/// bundle identity.
fn unpinned_top_level(name: &str) -> bool {
    name == "realms.json" || name == "dialects" || name == crate::capabilities::DEFINITIONS_DIR
}

/// The top-level name an ACTIVE input stands under, where that name is one
/// the walk skips (decision 0066 ruling 5, correcting decision 0065's
/// design D7). A seat's charter decides what the seat is told and a layer's
/// table decides how the run is ruled, so neither may sit where the
/// manifest's file map does not look: the skip is sound for operator
/// configuration only while nothing a recipe EXECUTES is read from under
/// it. One predicate, shared with the walk, so the two cannot drift.
///
/// `root` is the declaring layer's canonical directory and `reference` is
/// what that layer wrote. Judged twice: here, the reference as written, its
/// `.` and `..` folded without touching the disk, which catches a path
/// spelled into the skipped tree — a link there that points back out
/// included, since the link is itself bytes nobody pins; and in
/// [`bound_input`], its canonical target, which catches an allowed spelling
/// whose file, or whose parent, is a link into it. That second question is
/// asked of the one resolution the read is then bound to.
///
/// A reference written OUT of the layer altogether — `../shared/role.md`
/// — escapes the same map the same way, and no other route pins it: an
/// agent's charter is pinned by its library record, a dialect's
/// instructions by the dialect pin, and an inline role by the file map or
/// by nothing. It is refused on the same terms. The answer is WHERE the
/// input stands, as the clause both refusals carry.
///
/// Second council H5: A `..` STEP IS NOT A PATH THE WALK EVER TAKES. The
/// walk descends real directory entries — through a link, though a
/// consumed input whose link leaves the layer is refused by
/// [`bound_input`] under operator ruling 3 — and every key it writes is a
/// chain of such entries. A
/// reference that walks back UP cannot be one of those keys: with
/// `base/alias -> ../outside/child`, `alias/../charter.md` folds
/// lexically to `base/charter.md`, which the map does pin, while the file
/// a reader opens is `outside/charter.md`, which nothing pins. The chief
/// edited that file and every manifest digest stayed the same.
///
/// So the fold is no longer asked to stand in for the filesystem: a
/// reference that reaches its file through `..` is refused outright, and
/// what remains is exactly the set of paths the walk enumerates.
fn unpinned_active_input(root: &Path, reference: &str) -> Option<String> {
    let folded = folded(&root.join(reference));
    if !folded.starts_with(root) {
        return Some(
            "which stands outside the layer's own directory, where the bundle's file walk \
             never reaches"
                .to_string(),
        );
    }
    // The narrower question first, so a reference spelled into operator
    // configuration keeps naming the tree it reached.
    skipped_top_level(root, &folded).or_else(|| {
        Path::new(reference)
            .components()
            .any(|part| part == std::path::Component::ParentDir)
            .then(|| {
                "which reaches its file through a '..' step — never a path the bundle's file \
                 walk takes, so a link earlier in it can put the file a reader opens outside \
                 everything the walk pinned"
                    .to_string()
            })
    })
}

/// The clause for a path standing under a top-level name the walk skips,
/// judged relative to the declaring layer's canonical `root`.
fn skipped_top_level(root: &Path, path: &Path) -> Option<String> {
    let first = path.strip_prefix(root).ok()?.components().next()?;
    let name = first.as_os_str().to_str()?;
    unpinned_top_level(name).then(|| {
        format!(
            "which stands under '{name}' — a top-level name the bundle's file walk does not \
             pin, because it holds operator configuration"
        )
    })
}

/// One consumed input, resolved from its layer's directory handle a name at
/// a time and read through the handle that resolution ended at (operator
/// ruling 3; decision 0065 slice one, design D7).
pub(crate) struct BoundInput {
    /// The bytes the handle supplied: the buffer a caller hashes and parses,
    /// and whose digest the layer's walk takes for both of its keys.
    pub(crate) bytes: Vec<u8>,
    /// What the resolution observed, the names it bound the input by and
    /// the handles it holds, so the walk can verify the input without
    /// opening its path to read it.
    pub(crate) held: Held,
}

/// The names one observation bound its input by, with the file it read
/// (rebuild unit 16-fix-c, F1): built only by [`observe`], in one value,
/// from the handles that resolution holds, and compared whole whenever the
/// input is observed again. A key is never chosen by listing a path.
/// Rebuild unit 18-fix-b (council F1): a charter's pin keeps its binding
/// whole, and the dispatch door compares it whole. Its return (council
/// F1): whole includes every entry the resolution walked, so a
/// directory on the way replaced around the very file that was read is a
/// binding that no longer holds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Binding {
    /// The file-map key under which the declaring layer's walk pins what the
    /// reference names, as written. Where no step of the written path is a
    /// link it is `target_key`. Through a link it is the written spelling:
    /// the link's own entry where the last step is the link, and otherwise
    /// the path the walk lists through a linked directory, which names the
    /// target's own entry, never a second one (rebuild unit 16-fix-d).
    key: String,
    /// The file-map key the declaring layer's walk writes for the target:
    /// each name exactly as it was looked up in the held directory before
    /// it. A spelling the filesystem accepted for an entry its directory
    /// lists otherwise (a case or normalization alias) is bound as written,
    /// and the walk, listing no entry of that name, refuses it.
    target_key: String,
    /// The `(dev, ino)` of the handle that was read.
    id: (u64, u64),
    /// Every step the resolution took, in order, from the layer's
    /// directory to the file read: each directory it stood in is the one
    /// it held, and each link the text it followed.
    steps: Vec<Step>,
}

impl Binding {
    /// The reference's key and the target's key.
    pub(crate) fn keys(&self) -> [&str; 2] {
        [&self.key, &self.target_key]
    }

    /// What a layer's walk is told about `key`, one of these keys, whose
    /// buffer hashed to `digest`, read for `consumer`.
    pub(crate) fn supplied(&self, digest: &str, consumer: String) -> Supplied {
        Supplied {
            digest: digest.to_string(),
            id: self.id,
            target: self.target_key.clone(),
            consumer,
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
impl Binding {
    /// The binding a read under `root` names the file at `target` by,
    /// spelled `key`: a test's expected pin (rebuild unit 18-fix-b). Its
    /// steps are looked at by path, one written name at a time, a link's
    /// text followed from the link's own directory (its return).
    pub(crate) fn expected(root: &Path, key: &str, target: &str) -> Binding {
        use std::os::unix::fs::MetadataExt;
        let id = |path: &Path| {
            let meta = std::fs::metadata(path).unwrap();
            (meta.dev(), meta.ino())
        };
        let names = |path: &Path| -> Vec<OsString> {
            path.components()
                .filter(|part| *part != std::path::Component::CurDir)
                .rev()
                .map(|part| part.as_os_str().to_os_string())
                .collect()
        };
        let mut steps = vec![Step::Entry(OsString::new(), id(root))];
        let (mut at, mut pending) = (root.to_path_buf(), names(Path::new(key)));
        while let Some(name) = pending.pop() {
            let path = at.join(&name);
            match std::fs::read_link(&path) {
                Ok(text) => {
                    pending.extend(names(&text));
                    steps.push(Step::Link(name, text.into_os_string()));
                }
                Err(_) if name == ".." => {
                    at.pop();
                }
                Err(_) => {
                    steps.push(Step::Entry(name, id(&path)));
                    at = path;
                }
            }
        }
        Binding {
            key: key.to_string(),
            target_key: target.to_string(),
            id: id(&root.join(target)),
            steps,
        }
    }
}

/// A key a bound read supplied, as its layer's walk takes it (design D7):
/// the digest of the buffer that was read, and the file the read held,
/// named by its target's key, so no other name for that file is walked as
/// a file nothing consumed.
pub(crate) struct Supplied {
    digest: String,
    id: (u64, u64),
    target: String,
    /// Who consumed it, bounded, as its own refusal opens: the declaring
    /// file, and the document, the table or the seat and its role.
    consumer: String,
}

/// One step of an owner-rooted resolution, in the order it was taken: an
/// entry opened by the name it was looked up by, with the `(dev, ino)` its
/// handle holds, or a link, with the text it was followed by. The layer's
/// own directory is the first entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Step {
    Entry(OsString, (u64, u64)),
    Link(OsString, OsString),
}

/// A handle a resolution stands in, with the name it was looked up by and
/// the `(dev, ino)` it holds.
#[cfg(any(target_os = "linux", target_os = "macos"))]
type Hold = (std::fs::File, OsString, (u64, u64));

/// What one bound read observed, and the handles its resolution ended
/// holding: the chain from the layer's directory to the read file, the read
/// file last. A handle for a step the resolution left again — a directory a
/// `..` climbed out of, or a chain an absolute link's text restarted at the
/// layer's directory — was closed when it was left and is not among them.
/// The retained handles stay open while the input is consumed, so no file
/// they hold can give its number to another.
pub(crate) struct Held {
    root: PathBuf,
    reference: PathBuf,
    binding: Binding,
    /// Who the owner's directory was when this read reached it from `/`
    /// ([`owned_input`]); `None` for a read given its directory by its path.
    owner: Option<OwnerIdentity>,
    handles: Vec<std::fs::File>,
}

impl Held {
    /// The names the input was bound by, and the file that was read.
    pub(crate) fn binding(&self) -> &Binding {
        &self.binding
    }

    /// Whether `now`, the reference observed again, stands as it stood when
    /// it was read: the same binding — both keys, the file read and the same
    /// steps (every entry the same file, every link the same text) — and
    /// the same owner, it and every directory above it.
    fn stands(&self, now: &Observation) -> bool {
        (&now.binding, &now.owner) == (&self.binding, &self.owner)
    }

    /// Whether the input still stands as it was read: its reference, resolved
    /// again from the layer's directory — reached as the read reached it —
    /// [`Held::stands`], and the held file still holds exactly the bytes
    /// whose digest is `digest`. Those bytes are read back through the held
    /// handle, never by opening the path, and any failure to observe or read
    /// is a change: this fails closed.
    pub(crate) fn intact(&self, digest: &str) -> bool {
        use std::io::{Read, Seek};
        let mut file = self.handles.last().expect("a resolution ends at a handle");
        let mut again = Vec::new();
        let reread = file.rewind().and_then(|()| file.read_to_end(&mut again));
        let held = reread.map(|_| sha256_bytes(&again));
        let open = || match self.owner {
            Some(_) => owner_open(&self.root),
            None => by_path(&self.root),
        };
        let stands = observe(&self.root, &open, &self.reference).is_ok_and(|now| self.stands(&now));
        (held.ok().as_deref(), stands) == (Some(digest), true)
    }
}

/// What one owner-rooted resolution saw.
struct Observation {
    handles: Vec<std::fs::File>,
    binding: Binding,
    owner: Option<OwnerIdentity>,
}

/// Why an active input was not bound. `Missing` is said by
/// [`missing_clause`] in the caller's own sentence, because a missing
/// charter and a missing table are not said in the same words; `Place` is
/// the clause a refusal carries.
pub(crate) enum InputFault {
    Missing(std::io::Error),
    Place(Place),
}

/// Where a bound read refused, as the resolver refused it (rebuild unit
/// 18-fix, F4): the closed kind a dispatch refusal is named by, beside the
/// clause a compile refusal carries. The kind is set where the refusal is
/// made and never read back out of the clause's prose.
pub(crate) struct Place {
    kind: FaultKind,
    clause: String,
    /// What to do about it, where the caller's own remedy does not apply
    /// (rebuild unit 18-fix-b return, F3): moving a charter within its owner
    /// cannot make a directory above that owner readable.
    remedy: Option<&'static str>,
}

/// The kinds of place a bound read refuses, each the one word a dispatch
/// refusal names it by.
#[derive(Clone, Copy)]
enum FaultKind {
    /// A FIFO, device or directory where a regular file must be.
    Nonregular,
    /// A link, or a `..`, that leads out of the owner's directory.
    Outward,
    /// A file, or an owner's directory, that is no longer the one bound.
    Replaced,
    /// A file that is there but cannot be read.
    Unreadable,
    /// A reference no bound read can name a file by: a skipped tree, a
    /// `..` step, too many links, a host that cannot bind a read.
    Unbound,
}

impl Place {
    fn of(kind: FaultKind, clause: impl Into<String>) -> InputFault {
        InputFault::Place(Place {
            kind,
            clause: clause.into(),
            remedy: None,
        })
    }

    /// The remedy this place carries, if the caller's does not apply.
    pub(crate) fn remedy(&self) -> Option<&'static str> {
        self.remedy
    }
}

impl std::fmt::Display for Place {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.clause)
    }
}

/// The clause for an input whose target could not be resolved: absent, or
/// unresolvable with the io kind that says why.
pub(crate) fn missing_clause(error: &std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::NotFound => "which does not exist".to_string(),
        kind => format!("which cannot be resolved ({kind})"),
    }
}

/// An authored input reference rendered for a refusal, bounded (decision
/// 0065 slice one, rebuild unit 16): the same shape as [`bounded_site`],
/// over the printable ASCII a path is written in other than the quote. A
/// reference of at most 128 such bytes is quoted whole; any other is named
/// by its leading run of them, at most 64, and its length in bytes, so ten
/// thousand bytes of `./` cannot become a ten-thousand-byte reason.
pub(crate) fn bounded_reference(reference: &str) -> String {
    let printable = |byte: &u8| (b' '..=b'~').contains(byte) && *byte != b'\'';
    if reference.len() <= 128 && reference.bytes().all(|byte| printable(&byte)) {
        return format!("'{reference}'");
    }
    let lead: String = reference
        .bytes()
        .take_while(printable)
        .take(64)
        .map(char::from)
        .collect();
    format!("'{lead}…' ({} bytes, not echoed in full)", reference.len())
}

/// The file-map key the walk writes for `path` in the layer at `root`, or
/// `None` when `path` does not stand inside it.
fn walk_key(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    Some(
        relative
            .iter()
            .map(|part| part.to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// Where a bound read stands, for the tests' controlled replacements, in
/// the order a read reaches them; and where the walk stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadStage {
    /// The resolution holds a handle on the entry at this path, looked up
    /// in the directory handle before it.
    Entered,
    /// A handle is open on the contained target; nothing is checked yet.
    Opened,
    /// The handle's own kind was checked: it holds a regular file.
    Checked,
    /// The handle's bytes are in the buffer, and the binding is about to be
    /// verified again.
    Read,
    /// The binding was verified and both keys are fixed; the input is about
    /// to be handed to its caller.
    Verified,
    /// The walk is about to judge, then read and hash, a file under no
    /// consumed key.
    Walked,
    /// The walk is about to open and list a directory: one in a skipped tree
    /// is not yet opened through its parent's handle.
    Listing,
    /// A directory in a skipped tree was opened through its parent's handle
    /// and checked to be the directory its parent listed there, and is about
    /// to be listed through that handle.
    DirectoryChecked,
    /// The walk listed this entry in a tree it skips, and is about to ask
    /// what it is without following a link.
    Skipped,
    /// An owner's directory is about to be reached from `/`, and who it is
    /// taken (rebuild unit 18-fix-b, F2).
    Owning,
    /// An owner's directory was reached, and who it is was taken.
    Owned,
}

#[cfg(test)]
type ReadHook = Box<dyn FnMut(ReadStage, &Path)>;

#[cfg(test)]
thread_local! {
    /// A controlled replacement, run at each stage of a bound read on this
    /// thread. Tests only: nothing in production can reach between the
    /// stages.
    pub(crate) static READ_HOOK: std::cell::RefCell<Option<ReadHook>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn at_stage(stage: ReadStage, target: &Path) {
    READ_HOOK.with(|hook| {
        if let Some(hook) = hook.borrow_mut().as_mut() {
            hook(stage, target);
        }
    });
}

#[cfg(not(test))]
fn at_stage(_: ReadStage, _: &Path) {}

/// The most links one resolution follows, as Linux's own lookup does.
const MAX_LINKS: usize = 40;

/// The clause a consumed input whose resolved target leaves its layer
/// carries (operator ruling 3).
const OUTWARD: &str = "which resolves through a link to a file outside the layer's own \
                       directory; the walk pins such a link only by the bytes it reaches, so \
                       retargeting it to equal bytes moves nothing, and it is refused rather \
                       than pinned and admitted (operator ruling 3)";

/// The clause a FIFO, device or directory carries.
const NONREGULAR: &str = "which is not a regular file; only a regular file's bytes are read, \
                          hashed and pinned, and a FIFO, device or directory could supply bytes \
                          the walk never hashed";

/// The clause a read whose reference no longer resolves as it did carries.
const REPLACED: &str = "which was replaced while it was read: the file the read holds is no \
                        longer the contained target that was checked, so its bytes are not the \
                        ones verified";

/// Every name the directory `handle` holds lists, but `.` and `..`, read
/// through that handle and never by a path (rebuild unit 16-fix-e, second
/// return F2). A listing that cannot be opened or ends in error is an error:
/// the caller fails closed. The calls are rustix's safe ones (operator ruling
/// 2026-09-29: this crate forbids `unsafe`, decision 0071 ruling 1).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn names_in(handle: &std::fs::File) -> std::io::Result<Vec<OsString>> {
    use std::os::unix::ffi::OsStrExt;
    let mut names = Vec::new();
    for entry in rustix::fs::Dir::read_from(handle)? {
        let entry = entry?;
        let name = entry.file_name().to_bytes();
        if !matches!(name, b"." | b"..") {
            names.push(OsStr::from_bytes(name).to_os_string());
        }
    }
    Ok(names)
}

/// A handle on the directory at `path`, reached as a path is (a pinned
/// tree's contained linked directory included), which opens nothing but a
/// directory and never waits.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn directory(path: &Path) -> std::io::Result<std::fs::File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NONBLOCK | OFlags::CLOEXEC;
    Ok(rustix::fs::open(path, flags, Mode::empty())?.into())
}

/// A handle on the directory `name` inside the directory `parent` holds,
/// never following a link it names.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn directory_at(parent: &std::fs::File, name: &OsStr) -> std::io::Result<std::fs::File> {
    open_at(parent, name, rustix::fs::OFlags::DIRECTORY)
}

/// No supported host lacks a listing through a handle (decision 0063); any
/// other refuses rather than listing a layer by its path.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn directory(_: &Path) -> std::io::Result<std::fs::File> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// As [`directory`]: any other host lists nothing.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn directory_at(_: &std::fs::File, _: &OsStr) -> std::io::Result<std::fs::File> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// As [`directory`]: any other host lists nothing.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn names_in(_: &std::fs::File) -> std::io::Result<Vec<OsString>> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// Open `name` inside the directory `parent` holds, non-blocking, never
/// following a link it names, with any further `flags`. Non-blocking: a FIFO
/// met on the way to an input is refused by the handle's kind, never waited
/// on. No-follow: each link on the way is seen, its text read, and followed
/// by this code.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn open_at(
    parent: &std::fs::File,
    name: &OsStr,
    flags: rustix::fs::OFlags,
) -> std::io::Result<std::fs::File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::NOFOLLOW | OFlags::CLOEXEC | flags;
    Ok(rustix::fs::openat(parent, name, flags, Mode::empty())?.into())
}

/// The text of the link `name` inside the directory `parent` holds.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn link_at(parent: &std::fs::File, name: &OsStr) -> std::io::Result<OsString> {
    use std::os::unix::ffi::OsStringExt;
    let text = rustix::fs::readlinkat(parent, name, Vec::new())?;
    Ok(OsString::from_vec(text.into_bytes()))
}

/// Why an entry on the way to an input could not be opened or read: an
/// absent entry, or a parent that is not a directory, is a reference that
/// resolves to nothing, said in the caller's own words; anything else is a
/// file that cannot be read, with its cause.
fn unopened(error: std::io::Error) -> InputFault {
    match error.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => {
            InputFault::Missing(error)
        }
        kind => Place::of(
            FaultKind::Unreadable,
            format!("which cannot be read ({kind})"),
        ),
    }
}

/// Put `text`'s names on `pending`, the first on top, each marked `written`
/// or not. An absolute `text` restarts at the layer's directory, which it
/// must stand under; a relative one continues from the current handle.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn queue(
    root: &Path,
    text: &Path,
    written: bool,
    stack: &mut Vec<Hold>,
    pending: &mut Vec<(OsString, bool)>,
) -> Result<(), InputFault> {
    let relative = if text.is_absolute() {
        stack.truncate(1);
        text.strip_prefix(root)
            .map_err(|_| Place::of(FaultKind::Outward, OUTWARD))?
    } else {
        text
    };
    pending.extend(
        relative
            .components()
            .rev()
            .filter(|part| *part != std::path::Component::CurDir)
            .map(|part| (part.as_os_str().to_os_string(), written)),
    );
    Ok(())
}

/// Resolve `reference` from the layer's directory at `root` a name at a
/// time: each name is looked up inside the directory handle before it,
/// without following a link; a link's text is read and its names are looked
/// up in turn, from the link's own directory or, when absolute, from the
/// layer's. A `..` that would step above the layer, an absolute text outside
/// it, or more than [`MAX_LINKS`] links is refused. No path is canonicalized
/// and then opened: every handle is found inside the one before it. The
/// layer's directory itself is the handle `open` gives: by its path at
/// compile ([`by_path`]), and at dispatch through [`owner_read`]'s check
/// that it is still the directory the compile bound.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn observe(root: &Path, open: Opener<'_>, reference: &Path) -> Result<Observation, InputFault> {
    use std::os::unix::fs::MetadataExt;
    let identity = |file: &std::fs::File| file.metadata().map(|meta| (meta.dev(), meta.ino()));
    let (top, owner) = open()?;
    let id = identity(&top).map_err(InputFault::Missing)?;
    let mut steps = vec![Step::Entry(OsString::new(), id)];
    let mut stack: Vec<Hold> = vec![(top, OsString::new(), id)];
    let mut pending = Vec::new();
    queue(root, reference, true, &mut stack, &mut pending)?;
    let (mut links, mut through_link) = (0, false);
    while let Some((name, written)) = pending.pop() {
        if name == ".." {
            if stack.len() == 1 {
                return Err(Place::of(FaultKind::Outward, OUTWARD));
            }
            stack.pop();
            continue;
        }
        let parent = &stack.last().expect("the layer's directory stays").0;
        match open_at(parent, &name, rustix::fs::OFlags::empty()) {
            Ok(file) => {
                let id = identity(&file).map_err(InputFault::Missing)?;
                steps.push(Step::Entry(name.clone(), id));
                stack.push((file, name, id));
                let path = stack[1..]
                    .iter()
                    .fold(root.to_path_buf(), |path, (_, name, _)| path.join(name));
                at_stage(ReadStage::Entered, &path);
            }
            Err(error) => {
                let text = link_at(parent, &name).map_err(|_| unopened(error))?;
                links += 1;
                if links > MAX_LINKS {
                    return Err(Place::of(
                        FaultKind::Unbound,
                        format!(
                            "which resolves through more than {MAX_LINKS} links, so it names no \
                             file"
                        ),
                    ));
                }
                through_link |= written;
                steps.push(Step::Link(name, text.clone()));
                queue(root, Path::new(&text), false, &mut stack, &mut pending)?;
            }
        }
    }
    // Each name exactly as it was looked up in the held directory before it
    // (rebuild unit 16-fix-d): no listing, search or inode match ever puts
    // another entry in its place. The keys, and the file read, are bound
    // here and nowhere else.
    let target_key = stack[1..]
        .iter()
        .map(|(_, name, _)| name.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    // Through a link, the link's own entry as written: `unpinned_active_input`
    // has refused every spelling that leaves the layer or steps up. Without
    // one, the written path IS the target, and its key is the target's.
    let key = match through_link {
        true => walk_key(root, &folded(&root.join(reference))).unwrap_or_default(),
        false => target_key.clone(),
    };
    let id = stack.last().expect("the layer's directory stays").2;
    Ok(Observation {
        handles: stack.into_iter().map(|(file, ..)| file).collect(),
        binding: Binding {
            key,
            target_key,
            id,
            steps,
        },
        owner,
    })
}

/// No supported host lacks the resolution; any other refuses rather than
/// reading an input it cannot bind (design D7).
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn observe(_: &Path, _: Opener<'_>, _: &Path) -> Result<Observation, InputFault> {
    Err(Place::of(
        FaultKind::Unbound,
        "which this host cannot read through a handle bound to its contained target",
    ))
}

/// How a resolution is given its layer's directory: [`by_path`], or reached
/// as an owner's ([`owner_open`], and [`owner_read`]'s checked handle), with
/// who that owner is.
type Opener<'a> = &'a dyn Fn() -> Result<(std::fs::File, Option<OwnerIdentity>), InputFault>;

/// The layer's directory opened by its canonical path, as the compile
/// opens it: the compile has just canonicalized that path.
fn by_path(root: &Path) -> Result<(std::fs::File, Option<OwnerIdentity>), InputFault> {
    Ok((
        std::fs::File::open(root).map_err(InputFault::Missing)?,
        None,
    ))
}

/// Decision 0065 slice one, design D7, under operator ruling 3 ("It is not
/// pinned and admitted"): resolve what `reference` names against the
/// declaring layer's canonical `root`, require the target to stand inside
/// that layer and outside every tree the walk skips, and read it ONCE,
/// through one handle, into the one buffer a caller parses and hashes.
///
/// The target is found by [`observe`], from the layer's directory handle a
/// name at a time, never by canonicalizing a path and opening it: a link
/// inside the layer is followed, and one that leads out of it is refused
/// even where the walk would pin the bytes it reaches, because the walk
/// pins a link only by those bytes and a retarget to equal bytes moves
/// nothing. Every name is opened non-blocking, so a FIFO never blocks the
/// compile, and the handle the resolution ends at must hold a regular file
/// before a byte is read, so a FIFO, device or directory never supplies
/// any. Both file-map keys come from that one observation, bound with the
/// file it read ([`Binding`]): whether a written step was a link is what it
/// saw, and each name is exactly the one it looked up, never another entry
/// found by a later look at the directory or the path.
///
/// After the read, while the handles are still held, the reference is
/// resolved again from the layer's directory and must take exactly the same
/// steps to the same binding, keys included (rebuild unit 16-fix-c, F1).
/// What is guaranteed is exactly this: the bytes returned are the buffer
/// read from the file the observation held, and when the reference no
/// longer resolves to that file in the same steps under the same names, the
/// read is refused.
/// A file put in place BEFORE the resolution is the file resolved; one
/// swapped in and back again between the open and the check is not seen,
/// and the buffer is still the held file's. Bytes written in place into the
/// held file after the read are refused where the layer's walk verifies it
/// ([`Held::intact`]), before that layer's identity is sealed.
pub(crate) fn bound_input(root: &Path, reference: &str) -> Result<BoundInput, InputFault> {
    bound_through(root, &|| by_path(root), reference)
}

/// [`bound_input`] with the layer's directory given by `open`, for both of
/// the read's resolutions.
fn bound_through(root: &Path, open: Opener<'_>, reference: &str) -> Result<BoundInput, InputFault> {
    use std::io::Read;
    if let Some(place) = unpinned_active_input(root, reference) {
        return Err(Place::of(FaultKind::Unbound, place));
    }
    let reference = PathBuf::from(reference);
    let observed = observe(root, open, &reference)?;
    let target = root.join(&observed.binding.target_key);
    if let Some(place) = skipped_top_level(root, &target) {
        return Err(Place::of(FaultKind::Unbound, place));
    }
    let held = Held {
        root: root.to_path_buf(),
        reference,
        binding: observed.binding,
        owner: observed.owner,
        handles: observed.handles,
    };
    let mut file = held.handles.last().expect("a resolution ends at a handle");
    at_stage(ReadStage::Opened, &target);
    if !file.metadata().map_err(unopened)?.is_file() {
        return Err(Place::of(FaultKind::Nonregular, NONREGULAR));
    }
    at_stage(ReadStage::Checked, &target);
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(unopened)?;
    at_stage(ReadStage::Read, &target);
    if !observe(root, open, &held.reference).is_ok_and(|now| held.stands(&now)) {
        return Err(Place::of(FaultKind::Replaced, REPLACED));
    }
    at_stage(ReadStage::Verified, &target);
    Ok(BoundInput { bytes, held })
}

/// Who an owner's directory is (rebuild unit 18-fix, F1; design D7): the
/// `(dev, ino)` of every directory from `/` down to it, the owner's own
/// last. Rebuild unit 18-fix-b (council F2): taken by the very read that
/// bound a charter, when it reached the owner's directory, and compared by
/// that read's second resolution, by the seal's check and at the door — never
/// recorded by a walk of its own.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OwnerIdentity(Vec<(u64, u64)>);

/// The clause an owner's directory that is no longer the one bound carries.
const OWNER_REPLACED: &str = "whose owner's directory, or a directory above it, is no longer \
                              the one the compile bound: it was replaced, or reached through a \
                              link";

/// The owner's directory at the canonical `root`, found from `/` a name at a
/// time: each looked up inside the directory handle before it, WITHOUT
/// following a link, and opened as a directory. A link where a directory of
/// the compiled path stood is a replaced component and refuses; nothing is
/// opened by the stored path. The handle comes with the [`OwnerIdentity`] of
/// the directories it was reached through. A directory on the way that is
/// there but cannot be opened is named (rebuild unit 18-fix-b, F3).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn owner_directory(root: &Path) -> Result<(std::fs::File, OwnerIdentity), InputFault> {
    use std::os::unix::fs::MetadataExt;
    let identity = |file: &std::fs::File| {
        file.metadata()
            .map(|meta| (meta.dev(), meta.ino()))
            .map_err(unopened)
    };
    at_stage(ReadStage::Owning, root);
    let mut reached = PathBuf::from("/");
    let mut handle = directory(&reached).map_err(unreached_at(&reached))?;
    let mut ancestry = vec![identity(&handle)?];
    for name in root.strip_prefix("/").unwrap_or(root) {
        reached.push(name);
        handle = directory_at(&handle, name).map_err(|error| match link_at(&handle, name) {
            Ok(_) => Place::of(FaultKind::Replaced, OWNER_REPLACED),
            Err(_) => unreached_at(&reached)(error),
        })?;
        ancestry.push(identity(&handle)?);
    }
    at_stage(ReadStage::Owned, root);
    Ok((handle, OwnerIdentity(ancestry)))
}

/// Why the directory at `path`, on the way from `/` to an owner's, could not
/// be opened: gone, as [`unopened`] says it, or there but not observable —
/// an ancestor without read permission — named, bounded, with its cause
/// (rebuild unit 18-fix-b, F3), so a compile refuses it by name.
/// Its return (F3): the remedy is the directory's, not the charter's.
fn unreached(path: &Path, error: std::io::Error) -> InputFault {
    let kind = error.kind();
    match unopened(error) {
        InputFault::Place(_) => InputFault::Place(Place {
            kind: FaultKind::Unreadable,
            clause: format!(
                "whose owner's directory cannot be reached: {} cannot be opened ({kind}), so the \
                 directory the charter is read from cannot be bound",
                bounded_reference(&path.to_string_lossy())
            ),
            remedy: Some(UNREACHED_REMEDY),
        }),
        missing => missing,
    }
}

/// [`unreached`] at `path`, for `/` and for every directory below it alike
/// (unit 26c).
fn unreached_at(path: &Path) -> impl Fn(std::io::Error) -> InputFault + '_ {
    move |error| unreached(path, error)
}

/// What to do about an owner's directory the compile cannot reach.
const UNREACHED_REMEDY: &str = "The compile opens every directory from '/' down to the charter's \
                                owner, so that directory must be readable by the user who \
                                compiles; grant it, or compile from a realm under directories \
                                that user can read (decision 0065 slice one, design D7)";

/// No supported host lacks the lookup; any other binds no owner.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn owner_directory(_: &Path) -> Result<(std::fs::File, OwnerIdentity), InputFault> {
    Err(Place::of(
        FaultKind::Unbound,
        "which this host cannot read through a handle bound to its owner",
    ))
}

/// The owner's directory at `root`, reached by [`owner_directory`], with who
/// it is.
fn owner_open(root: &Path) -> Result<(std::fs::File, Option<OwnerIdentity>), InputFault> {
    let (handle, owner) = owner_directory(root)?;
    Ok((handle, Some(owner)))
}

/// Rebuild unit 18-fix-b (council F1, F2): unit 16's bound read of
/// `reference`, as the compile reads a charter — from its owner's directory
/// at `root`, reached from `/` by [`owner_directory`] in each of the read's
/// two resolutions. Who the owner is is taken by that read and nowhere else:
/// the second resolution must find the same owner, as the seal's check and
/// the dispatch door must, or the read refuses as replaced. An owner the
/// compile cannot observe refuses the read, by name (F3).
pub(crate) fn owned_input(root: &Path, reference: &str) -> Result<BoundInput, InputFault> {
    bound_through(root, &|| owner_open(root), reference)
}

/// Rebuild unit 18-fix (F1): the same read at the dispatch door, whose
/// owner's directory must be `owner`, the one the compile's read found, or
/// the read refuses as replaced before anything is read. One resolver.
fn owner_read(
    root: &Path,
    owner: &OwnerIdentity,
    reference: &str,
) -> Result<BoundInput, InputFault> {
    let open = || {
        let (handle, now) = owner_directory(root)?;
        if &now != owner {
            return Err(Place::of(FaultKind::Replaced, OWNER_REPLACED));
        }
        Ok((handle, Some(now)))
    };
    bound_through(root, &open, reference)
}

/// A path with its `.` and `..` folded away, without touching the disk:
/// the spelling a layer's file map keys a file under. `Path::components`
/// already drops every `.` but a leading one, and every path folded here is
/// absolute — a reference joined to its layer's canonical root — so only
/// `..` is left to fold.
fn folded(path: &Path) -> PathBuf {
    let mut folded = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                folded.pop();
            }
            other => folded.push(other),
        }
    }
    folded
}

/// Decision 0066 ruling 5 at the dispatch door: the charter a seat is
/// about to be told must still be the bytes its layer's file map pinned.
/// The driver reads a role when it renders the prompt, long after the
/// compile that hashed it; without this check an edit in between reaches
/// the seat as fresh instructions under the old identity. The role is read
/// through any link exactly as the walk read it, so a link retargeted since
/// the compile is a change like any other. `None` where the pin holds.
///
/// Rebuild unit 17: the owner is the one the compile selected with each
/// site that is told this path — the declaring layer of an inline role, the
/// library of an agent's charter — found by the exact path, never by the
/// longest layer root the path starts with, and never by folding a spelling
/// onto a neighbouring pin. Every binding of the path must hold.
///
/// Second council H6: A CHARTER NO LAYER KEYS IS NOT A CHARTER NOBODY
/// PINNED. An agent's charter stands in the library, outside every
/// layer's file map, and the first repair answered `None` for it — so a
/// bundle could be compiled once, `agents/charters/worker.md` edited, and
/// the seat dispatched with `render_prompt` consuming the changed text
/// under the old identity. The library record's own `charter_digest`
/// already existed; it is compared HERE, at the door, against the bytes
/// the driver is about to be handed. A role neither route pins is refused
/// rather than launched, because a charter nothing pins is a charter
/// nobody ruled on.
/// The verified charter TEXT, from the same read the pin was checked
/// against (second council H6; task 6.3). A driver that reopened
/// `role_path` would read whatever the file said by then — after the door
/// read what it said at the door — so the bytes that were compared are the
/// bytes the seat is told, and nothing reopens the path to render them.
/// Rebuild unit 18: each binding is checked by [`pinned_charter`], through
/// its owner's bound read; the dispatch door itself checks only the site's
/// own binding, through [`site_charter_text`].
///
/// `Err((owner, what))` is the pin's complaint, in the two pieces the
/// dispatch refusal is written from.
pub fn charter_text(bundle: &Bundle, role: &Path) -> Result<String, (String, String)> {
    let pins: Vec<&CharterPin> = bundle.charters.get(role).into_iter().flatten().collect();
    if pins.is_empty() {
        return unbound_charter(bundle, role);
    }
    let mut texts = pins
        .into_iter()
        .map(|pin| pinned_charter(bundle, pin))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(texts.swap_remove(0))
}

/// Rebuild unit 18 (design D7): the charter ONE site is told, from the
/// binding the compile selected with that site — never from every binding
/// of a path, and never from the path the input names. `role` is the path
/// the site's input carries; it must be the path its binding was compiled
/// for, because an input whose `role_path` was merged over names a charter
/// the site was never bound to (a neighbour's, pinned or not). A site with
/// no binding is [`charter_text`]'s unbound case exactly.
pub fn site_charter_text(
    bundle: &Bundle,
    pin: Option<&CharterPin>,
    role: &Path,
) -> Result<String, (String, String)> {
    let Some(pin) = pin else {
        return unbound_charter(bundle, role);
    };
    let (name, key) = owned(bundle, pin)?;
    if role != pin.path {
        return Err((name, format!("replaced: {key}")));
    }
    pinned_charter(bundle, pin)
}

/// Rebuild unit 19 (design D7; task 19.1): EVERY charter this bundle bound,
/// checked exactly as the dispatch door checks one site's — through its
/// owner's bound read, owner, target and bytes ([`pinned_charter`]) — so a
/// run is neither started nor resumed over a charter that moved since the
/// compile. `Err` is the first binding's complaint, in the two pieces a
/// refusal is written from. Nothing read here is kept: every dispatch still
/// reads its own site's charter at the door.
///
/// Its review return (F1): `Ok` is what a run records at its start, one
/// entry per binding — its owner, key, target and [`binding_digest`] — so
/// that a resume, whose bundle is compiled again, is held to the bindings
/// the run STARTED over ([`charters_as_started`]), not the recompile's.
pub fn charters_intact(bundle: &Bundle) -> Result<Value, (String, String)> {
    bound_charters(bundle).map(|bound| charter_record(&bound))
}

/// Rebuild unit 19, review return (F1): a pinned resume over the bindings
/// its run started over. A recompile reads each charter afresh, so an
/// equal-byte retarget inside the owner, or a file replaced by equal bytes,
/// compiles to the very manifest the run pinned and binds anew; checked
/// against `started` — the run's own record, from [`charters_intact`] at its
/// start — each binding must be the one the run began with. A binding the
/// run did not record (a run started before its record existed) refuses:
/// nothing vouches for it.
///
/// Its second review return (F1): a run whose start recorded no bindings
/// at all refuses as `unrecorded` even when the bundle binds none, since
/// its start vouched for nothing (operator ruling 2026-09-29, point 1).
pub fn charters_as_started(
    bundle: &Bundle,
    started: Option<&Value>,
) -> Result<(), (String, String)> {
    let now = bound_charters(bundle)?;
    let Some(started) = started.cloned() else {
        let first = now
            .first()
            .map(|(owner, _, key, _, _)| (owner.clone(), key.clone()));
        let (owner, key) = first.unwrap_or_else(|| {
            let owner = format!("bundle '{}'", bundle.name);
            (owner, "the run started with no charter record".to_string())
        });
        return Err((owner, format!("unrecorded: {key}")));
    };
    for (owner, reference, key, target, binding) in &now {
        // Matched by owner and reference as written: a library's key is its
        // charter's file name, which a retarget moves.
        let was =
            started.as_array().into_iter().flatten().find(|was| {
                was["owner"] == owner.as_str() && was["reference"] == reference.as_str()
            });
        let cause = match was {
            None => "unrecorded",
            Some(was) if was["target"] != target.as_str() => "retargeted",
            Some(was) if was["binding"] != binding.as_str() => "replaced",
            Some(_) => continue,
        };
        let key = was.and_then(|was| was["key"].as_str()).unwrap_or(key);
        return Err((owner.clone(), format!("{cause}: {key}")));
    }
    // Every binding matched one the run recorded; a recorded binding this
    // bundle no longer selects is the one thing left to differ.
    match started == charter_record(&now) {
        true => Ok(()),
        false => Err((
            format!("bundle '{}'", bundle.name),
            "unselected: a charter the run started over".to_string(),
        )),
    }
}

/// One binding as a run records it: owner, reference, key, target, binding
/// digest.
type BoundCharter = (String, String, String, String, String);

/// Every charter the bundle bound, each checked through [`pinned_charter`],
/// as a run records it.
fn bound_charters(bundle: &Bundle) -> Result<BTreeSet<BoundCharter>, (String, String)> {
    let mut bound = BTreeSet::new();
    for pin in bundle.charters.values().flatten() {
        pinned_charter(bundle, pin)?;
        let (owner, key) = owned(bundle, pin)?;
        let (reference, target) = (pin.reference.clone(), pin.binding.target_key.clone());
        bound.insert((owner, reference, key, target, binding_digest(pin)));
    }
    Ok(bound)
}

fn charter_record(bound: &BTreeSet<BoundCharter>) -> Value {
    bound
        .iter()
        .map(|(owner, reference, key, target, binding)| {
            json!({"owner": owner, "reference": reference, "key": key, "target": target,
                   "binding": binding})
        })
        .collect()
}

/// The digest of a pin's whole binding and the owner directory its read
/// reached: every key, step, link text and `(dev, ino)` [`pinned_charter`]
/// compares, each part length-prefixed so no two bindings encode alike.
fn binding_digest(pin: &CharterPin) -> String {
    let Binding {
        key,
        target_key,
        id,
        steps,
    } = &pin.binding;
    let identity = |(dev, ino): (u64, u64)| [dev.to_be_bytes(), ino.to_be_bytes()].concat();
    let mut bytes = Vec::new();
    let mut put = |part: &[u8]| {
        bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
        bytes.extend_from_slice(part);
    };
    put(key.as_bytes());
    put(target_key.as_bytes());
    put(&identity(*id));
    for step in steps {
        match step {
            Step::Entry(name, id) => {
                put(b"entry");
                put(name.as_encoded_bytes());
                put(&identity(*id));
            }
            Step::Link(name, text) => {
                put(b"link");
                put(name.as_encoded_bytes());
                put(text.as_encoded_bytes());
            }
        }
    }
    for directory in &pin.directory.0 {
        put(&identity(*directory));
    }
    sha256_bytes(&bytes)
}

/// The owner and key a pin's refusals name. A pin whose layer is not one of
/// this bundle's is a charter the bundle's identity does not answer for.
fn owned(bundle: &Bundle, pin: &CharterPin) -> Result<(String, String), (String, String)> {
    pin.named(bundle).ok_or_else(|| {
        (
            format!("bundle '{}'", bundle.name),
            format!("unpinned: {}", pin.path.display()),
        )
    })
}

/// A role no site was bound to. After a compile every role is bound — an
/// inline charter to the layer that declared it, an agent's to the library
/// it was loaded from — so reaching here means the bundle's identity does
/// not answer for what this seat is about to be told, and the launch stops.
///
/// A RELATIVE role was not produced by this engine at all: `parse_role`
/// joins its layer's absolute directory and the library resolves an
/// absolute charter, so an absolute path is the only shape a compile
/// writes. Such a role reads as it always did.
fn unbound_charter(bundle: &Bundle, role: &Path) -> Result<String, (String, String)> {
    if !role.is_absolute() {
        return Ok(std::fs::read_to_string(role).unwrap_or_default());
    }
    Err((
        format!("bundle '{}'", bundle.name),
        format!("unpinned: {}", role.display()),
    ))
}

/// Rebuild unit 18 (design D7, operator ruling 3): one binding checked at
/// consumption, and the text of the read that checked it. The reference is
/// resolved again from its OWNER's canonical root — the declaring layer's,
/// or the library's own, wherever that library stands — through unit 16's
/// handle-bound read ([`bound_input`]), so a link that now leaves the owner,
/// a FIFO, or a replacement mid-read refuses exactly as it does at compile.
/// The buffer read must hash to the pinned digest, and the file it was read
/// from must be the target the compile bound: a retarget to equal bytes is
/// a moved charter, not an unchanged one. Only then is the buffer the text.
///
/// Rebuild unit 18-fix (council F1): the owner's directory is not opened by
/// its stored path, which follows whatever now stands there — a link to an
/// equal-byte tree outside, or such a tree renamed into place. Both of the
/// read's resolutions reach it from `/` without following a link, and it
/// must be the directory the compile bound, it and every directory above it
/// ([`owner_read`]).
///
/// Rebuild unit 18-fix-b (council F1): the owner the door requires is the
/// one the compile's read found, carried on the pin, and the read's WHOLE
/// binding must be the pin's — both keys and the file read. Equal bytes at
/// the same path in another file, or in a replaced directory, are
/// `replaced`; a target reached under another key is `retargeted`.
/// Its return (council F1): the binding carries every directory
/// the compile's read walked, so the very file read, moved into a directory
/// that replaced its own, is `replaced` too.
fn pinned_charter(bundle: &Bundle, pin: &CharterPin) -> Result<String, (String, String)> {
    let (name, key) = owned(bundle, pin)?;
    let refused = |cause: &str| (name.clone(), format!("{cause}: {key}"));
    let root = pin.owner.root();
    let bound = owner_read(root, &pin.directory, &pin.reference)
        .map_err(|fault| refused(fault_kind(&fault)))?;
    if sha256_bytes(&bound.bytes) != pin.digest {
        return Err(refused("changed"));
    }
    let now = bound.held.binding();
    if now != &pin.binding {
        return Err(refused(match now.target_key == pin.binding.target_key {
            true => "replaced",
            false => "retargeted",
        }));
    }
    // The pin is over BYTES; what a seat is told is text. A charter whose
    // bytes are not text is refused rather than rendered with its
    // undecodable parts replaced, because what the seat would then read is
    // not what the digest names.
    String::from_utf8(bound.bytes).map_err(|_| refused("unreadable"))
}

/// The one word a dispatch refusal names a failed bound read by: bounded,
/// never the clause's path or the value that failed, and taken from the
/// kind the resolver refused with, never from its prose (council F4).
fn fault_kind(fault: &InputFault) -> &'static str {
    match fault {
        InputFault::Missing(_) => "missing",
        InputFault::Place(place) => match place.kind {
            FaultKind::Nonregular => "nonregular",
            FaultKind::Outward => "outward",
            FaultKind::Replaced => "replaced",
            FaultKind::Unreadable => "unreadable",
            FaultKind::Unbound => "unbound",
        },
    }
}

/// Re-walk the script's directory, including its helpers, against the
/// declaring layer's compiled file map (decision 0046 ruling 4; proposed
/// 0048 narrows DD9). A realm scaffolded by `init .` also holds mutable
/// source, results and its journal; those are outside this spawn check.
/// Ancestor maps are the very maps hashed into their compose digests.
/// Paths come from compile's canonical roots and expanded script token;
/// component comparisons never reinterpret a literal filename byte.
pub(crate) fn layer_drift(bundle: &Bundle, directory: &Path) -> Option<(String, String)> {
    let layer = bundle
        .roots
        .iter()
        .filter(|root| directory.starts_with(root))
        .max_by_key(|root| root.components().count())?;
    let (name, pinned) = if layer == &bundle.dir {
        (&bundle.name, bundle.manifest["files"].as_object()?)
    } else {
        let ancestor = bundle
            .chain
            .iter()
            .find(|ancestor| &ancestor.dir == layer)?;
        (&ancestor.name, &ancestor.files)
    };
    let current = match walk_files(layer, directory, &BTreeMap::new()) {
        Ok(files) => files,
        Err(error) => return Some((name.clone(), error.to_string())),
    };
    let relative = directory
        .strip_prefix(layer)
        .expect("directory under layer");
    let pinned_files = pinned.iter().filter(|(key, _)| {
        !key.starts_with(COMPOSE_PREFIX) && Path::new(key).starts_with(relative)
    });
    for (key, digest) in pinned_files {
        match current.get(key) {
            Some(now) if now == digest => {}
            Some(_) => return Some((name.clone(), format!("changed: {key}"))),
            None => return Some((name.clone(), format!("missing: {key}"))),
        }
    }
    current
        .keys()
        .find(|key| !pinned.contains_key(*key))
        .map(|key| (name.clone(), format!("added: {key}")))
}

/// Decision 0043: record one site's hands — the agent's when the site
/// names an agent, the site's own `hands` value when it is inline. A
/// site with hands and secret bindings is refused under every boundary: the
/// `namespace` box clears the environment, and a binding that cannot reach
/// its seat is a binding silently dropped.
fn record_hands(
    what: &str,
    raw: &Value,
    from_agent: Option<HandsSpec>,
    secrets: &[String],
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let spec = match (from_agent, raw.get("hands")) {
        (Some(spec), _) => Some(spec),
        (None, Some(declared)) => Some(HandsSpec::parse(declared).map_err(|problem| {
            CompileError::Invalid(format!("seat '{what}' hands: {problem} (decision 0043)"))
        })?),
        (None, None) => None,
    };
    let state = match spec {
        Some(spec) => {
            if !secrets.is_empty() {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' declares hands and secret bindings {secrets:?}; under \
                     every boundary a seat with hands receives no binding, as the \
                     `namespace` box clears the environment (decision 0043)"
                )));
            }
            HandsState::Hands(spec)
        }
        // The declaration was read successfully and establishes no
        // hands: an affirmative fact, distinct from an unregistered or
        // unresolved site (design D10 F1).
        None => HandsState::NoHands,
    };
    site_facts(sites, what).hands = state;
    Ok(())
}

/// Walk one composed seat exactly as [`collect_unpinned`] does — same
/// labels, so the facts land where the engine looks — and resolve every
/// executable site's capabilities. An agent-backed site's asks are its
/// agent's minus what the site subtracts; an inline site's map is its own
/// office's asks. A wanted capability a candidate lost is a notice in that
/// site's agent record, where a skipped model link already is. `dir` is
/// the directory an inline site's command was expanded against; a select
/// case a later layer wrote by `override.cases` is walked in that layer's
/// root, from `roots` by `case_origin`, as `parse_select` parses it.
#[expect(clippy::too_many_arguments, reason = "decision 0065, #288")]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn record_capabilities(
    authority: &crate::capabilities::Authority,
    library: Option<&Library>,
    adapters: CapabilityAdapters<'_>,
    boundary: Boundary,
    (dir, roots, case_origin): (&Path, &[PathBuf], &BTreeMap<String, usize>),
    what: &str,
    raw: &Value,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let written = raw.get("capabilities");
    if let Some(name) = raw.get("agent").and_then(Value::as_str) {
        let agent = library
            .and_then(|library| library.agent(name))
            .expect("the seat loop resolved this agent reference");
        let asks = site_asks(what, raw, Some((name, &agent.capabilities)))?;
        let chain = site_facts(sites, what).chain.clone();
        // Under the harness boundary a hands site's candidates are composed
        // with the `hands.harness.*` fragment of the seat's class.
        let class = match (boundary, &agent.hands) {
            (Boundary::Harness, Some(_)) => Some(asks.class),
            _ => None,
        };
        let site = mcp::candidate_capabilities(authority, adapters, asks, &chain, class)?;
        let facts = site_facts(sites, what);
        // The EFFECTIVE class, an inherited office class included, as the
        // local admission recorded it (design D5.6).
        if let Some(requested) = facts.local.as_ref().and_then(|local| local.sandbox) {
            admit_native_sandbox(what, requested, &site)?;
        }
        let notices = facts
            .record
            .as_mut()
            .and_then(|record| record.get_mut("notices"))
            .and_then(Value::as_array_mut)
            .expect("an agent-backed site carries its resolution record");
        for (candidate, outcome) in chain.iter().zip(&site.outcomes) {
            for (capability, message) in &outcome.notices {
                notices.push(
                    crate::agents::Notice {
                        agent: candidate.agent.clone(),
                        provider: candidate.provider.clone(),
                        model: candidate.model.clone(),
                        capability: "capability".to_string(),
                        item: capability.clone(),
                        message: message.clone(),
                    }
                    .value(),
                );
            }
        }
        facts.capabilities = Some(site);
        return Ok(());
    }
    // A single site, by the same two keys `has_single` reads.
    if ["driver", "role"].iter().any(|key| raw.get(key).is_some()) {
        let asks = site_asks(what, raw, None)?;
        let parts = command_parts(raw);
        let driver = dispatch_driver(&parts);
        let facts = site_facts(sites, what);
        // Operator ruling (B) of 2026-09-27: an inline site with hands is
        // served like an agent, so its dialect carries the `hands.workspace`
        // fragment its driver's adapter declares, as `agents::compose`
        // carries an agent's. Its hands are boxed: the hands law refuses an
        // inline model harness's hands unboxed (decision 0046 ruling 4).
        let declared = adapters
            .adapters
            .zip(driver.as_deref())
            .and_then(|(adapters, driver)| adapters.adapter(driver))
            .and_then(|adapter| adapter.hands.as_ref())
            .filter(|_| facts.hands_spec().is_some());
        if let (Some(dialect), Some(fragment)) = (facts.inline_dialect.as_mut(), declared) {
            dialect.hands = fragment.clone();
            // Rebuild unit 14a4a: the engine emits that fragment, expanded
            // as an agent's `hands` segment is, and the plan types it.
            facts.inline_hands = Some(Segment::new(Origin::Hands, &expand_command(dir, fragment)));
        }
        // U1f: the MCP set this serving intends, never read from its bytes.
        facts.inline_mcp = Some(McpIntent::inline(driver.as_deref(), facts.hands_spec()));
        let serving = mcp::InlineServing::of(facts, driver.as_deref(), &parts);
        let site = mcp::inline_capabilities(authority, adapters, asks, &serving)?;
        // Rebuild unit 5d-fix-b: an inline class the engine lowered is
        // judged with its whole launch, the resolved native plan included.
        admit_inline_launch(what, &parts, facts, &site)?;
        facts.capabilities = Some(site);
        return Ok(());
    }
    if written.is_some() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' declares 'capabilities' beside a panel, sequence or select; a request \
             belongs to the site that executes — the member, step or case body — because that \
             is the office the realm grants to (decision 0065 ruling 5)"
        )));
    }
    // A dialect step runs the realm dialect's own validator through the
    // exec driver: no author wrote its command and it asks for nothing,
    // and it still gets an explicit outcome — "no capability" is a
    // recorded fact, never a missing one (design D5). Only where the
    // dialect supplied a validator: an unsupported `check` compiles to no
    // site at all, and none is invented for it here.
    if raw.get("dialect").is_some() {
        if sites.contains_key(what) {
            let asks = crate::capabilities::SiteAsks::at(step_class(raw), what, None, None)
                .map_err(CompileError::Invalid)?;
            let facts = site_facts(sites, what);
            facts.inline_mcp = Some(McpIntent::inline(Some("exec"), facts.hands_spec()));
            let serving = mcp::InlineServing {
                driver: Some("exec"),
                argv: &[],
                local: &[],
                hands: &[],
                intent: facts.inline_mcp,
            };
            facts.capabilities = Some(mcp::inline_capabilities(
                authority, adapters, asks, &serving,
            )?);
        }
        return Ok(());
    }
    let mut nested: Vec<(String, &Value, &Path)> = Vec::new();
    if let Some(panel) = raw.get("panel").and_then(Value::as_object) {
        nested.extend(
            panel
                .iter()
                .map(|(member, raw)| (format!("{what}:{member}"), raw, dir)),
        );
    }
    if let Some(sequence) = raw.get("sequence").and_then(Value::as_array) {
        for (index, step) in sequence.iter().enumerate() {
            nested.push((format!("{what}:{}", step_label(index, step)), step, dir));
        }
    }
    if let Some(select) = raw.get("select").and_then(Value::as_object) {
        // Each case in the layer that wrote it (second review return C1);
        // the default stays with the seat's owner.
        nested.extend(
            select
                .get("cases")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(case, raw)| {
                    let label = format!("{what}:{case}");
                    let owner = case_origin
                        .get(&label)
                        .map_or(dir, |&index| roots[index].as_path());
                    (label, raw, owner)
                }),
        );
        if let Some(body) = select.get("default") {
            nested.push((format!("{what}:default"), body, dir));
        }
    }
    for (label, raw, dir) in nested {
        record_capabilities(
            authority,
            library,
            adapters,
            boundary,
            (dir, roots, case_origin),
            &label,
            raw,
            sites,
        )?;
    }
    Ok(())
}

/// One executable site's asks at its own canonical class (GP1), read once
/// for every candidate it resolves against: what the site itself declares,
/// never its container's class, a neighbour's or its execution label.
fn site_asks(
    what: &str,
    raw: &Value,
    agent: Option<(&str, &crate::capabilities::Requests)>,
) -> Result<crate::capabilities::SiteAsks, CompileError> {
    let class = parse_class(what, raw)?;
    crate::capabilities::SiteAsks::at(class, what, agent, raw.get("capabilities"))
        .map_err(CompileError::Invalid)
}

/// The label a sequence step is recorded under: its `name`, or its
/// one-based position where the step names itself nothing. One spelling,
/// shared by the pin walk and the capability walk, so the two record the
/// same step under the same label.
fn step_label(index: usize, step: &Value) -> String {
    step.get("name")
        .and_then(Value::as_str)
        .map_or_else(|| format!("step-{}", index + 1), str::to_string)
}

/// The one accessor into the canonical site family (design D10 F1):
/// entrant creation is the only way a fact reaches the table, so a
/// site's whole value is the unit collection and relocation operate on.
fn site_facts<'a>(sites: &'a mut BTreeMap<String, SiteFacts>, label: &str) -> &'a mut SiteFacts {
    sites.entry(label.to_string()).or_default()
}

/// Relocate the whole execution-site family of a dialect-wrapped `verify`
/// seat (design D10 F1/F2): each exact authoring label of the wrapped body
/// moves to its wrapper destination — `verify` to `verify:checks`,
/// `verify:<member>` to `verify:checks:<member>`. Only the wrapped body's
/// OWN owners move; a distinct literal phase `verify:bar` keeps its facts
/// instead of being dragged onto a wrapper coordinate by a prefix sweep.
/// Every source is drained before any destination is inserted, so a member
/// named `x` beside one named `checks:x` — whose destination is the other's
/// source label — cannot overwrite a still-needed source.
fn relocate_verify_facts(
    sites: &mut BTreeMap<String, SiteFacts>,
    moved: &[(String, String, crate::engine::resume::SiteKey)],
) {
    let staged: Vec<(String, SiteFacts)> = moved
        .iter()
        .filter_map(|(source, destination, _)| {
            sites
                .remove(source)
                .map(|facts| (destination.clone(), facts))
        })
        .collect();
    for (destination, facts) in staged {
        sites.insert(destination, facts);
    }
}

/// Claim one wrapper-introduced address against the authoring census
/// before any fact is written there (design D10 F2). `owner_index` already
/// guarantees the census holds no duplicate labels, so any occupant is a
/// distinct owner and the bundle is ambiguous. The refusal is worded
/// exactly like the final walk's, naming the seat and both owners.
fn claim_address(
    census: &BTreeMap<String, crate::engine::resume::SiteKey>,
    phase: &str,
    label: &str,
    second: &str,
) -> Result<(), CompileError> {
    match census.get(label) {
        Some(occupant) => Err(CompileError::Invalid(format!(
            "seat '{phase}' addresses two different sites as '{label}': {} and {second}. The \
             selection, the argv lookup, the hands map and the boundary map all key on that one \
             string, so one site would answer for the other; rename one of them",
            crate::engine::resume::describe(occupant)
        ))),
        None => Ok(()),
    }
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
    boundary: Boundary,
    charters: &'a Charters,
}

fn parse_selected_body(
    dir: &Path,
    what: &str,
    raw: &Value,
    agents: &mut Option<AgentContext>,
    sites: &mut BTreeMap<String, SiteFacts>,
    compile: BodyCompile<'_>,
) -> Result<SeatBody, CompileError> {
    let BodyCompile {
        results,
        secrets,
        boundary,
        charters,
        ..
    } = compile;
    refuse_boundary_key(what, raw)?;
    refuse_crossing_keys(what, raw)?;
    refuse_unknown_keys(what, raw, BODY_KEYS)?;
    refuse_confine(what, raw)?;
    refuse_driver_keys(what, raw)?;
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
    if has_panel || has_sequence {
        refuse_tools_on_container(what, raw)?;
    }
    if has_agent {
        let resolved = resolve_reference(
            agents,
            sites,
            dir,
            what,
            what,
            raw,
            secrets,
            Site::Seat,
            boundary,
        )?;
        let law = SiteLaw {
            boundary,
            dir,
            agent_hands: resolved.hands.as_ref(),
        };
        enforce_model_policy(what, raw, &resolved.candidates, secrets, agents, law, sites)?;
        record_hands(what, raw, resolved.hands, secrets, sites)?;
        let body = SeatBody::Single {
            role_path: resolved.role_path,
            command: resolved.command,
            candidates: resolved.candidates,
        };
        Ok(body)
    } else if has_panel {
        let (members, aggregate) = parse_panel(
            dir, what, raw, results, secrets, agents, sites, boundary, charters,
        )?;
        refuse_class_without_a_driver(what, raw)?;
        Ok(SeatBody::Panel { members, aggregate })
    } else if has_sequence {
        let steps = parse_sequence(dir, what, raw, agents, sites, compile)?;
        refuse_class_without_a_driver(what, raw)?;
        Ok(SeatBody::Sequence { steps })
    } else {
        record_inline_tools(
            dir,
            what,
            raw,
            false,
            &command_parts(raw),
            agents.as_ref().map(|context| &context.adapters),
            sites,
        )?;
        let law = SiteLaw {
            boundary,
            dir,
            agent_hands: None,
        };
        enforce_model_policy(what, raw, &[], secrets, agents, law, sites)?;
        record_hands(what, raw, None, secrets, sites)?;
        let body = SeatBody::Single {
            role_path: parse_role(dir, what, raw, charters, sites)?,
            command: parse_command(dir, what, raw, secrets)?,
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
    boundary: Boundary,
    charters: &'a Charters,
}

fn parse_select(
    dir: &Path,
    phase: &str,
    raw: &Value,
    agents: &mut Option<AgentContext>,
    sites: &mut BTreeMap<String, SiteFacts>,
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
                sites,
                BodyCompile {
                    results: compile.results,
                    secrets: compile.secrets,
                    dialect: compile.dialect,
                    boundary: compile.boundary,
                    charters: compile.charters,
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
                sites,
                BodyCompile {
                    results: compile.results,
                    secrets: compile.secrets,
                    dialect: compile.dialect,
                    boundary: compile.boundary,
                    charters: compile.charters,
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

#[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn parse_panel(
    dir: &Path,
    what: &str,
    raw: &Value,
    declared_results: &[String],
    secrets: &[String],
    agents: &mut Option<AgentContext>,
    sites: &mut BTreeMap<String, SiteFacts>,
    boundary: Boundary,
    charters: &Charters,
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
        refuse_boundary_key(&site, member_raw)?;
        refuse_crossing_keys(&site, member_raw)?;
        refuse_confine(&site, member_raw)?;
        refuse_driver_keys(&site, member_raw)?;
        if member_raw.get("agent").is_some() {
            refuse_amendments(&site, member_raw)?;
        }
        refuse_unknown_keys(&site, member_raw, MEMBER_KEYS)?;
        let (role_path, command, candidates, agent_hands) = match member_raw.get("agent") {
            None => {
                record_inline_tools(
                    dir,
                    &site,
                    member_raw,
                    false,
                    &command_parts(member_raw),
                    agents.as_ref().map(|context| &context.adapters),
                    sites,
                )?;
                (
                    parse_role(dir, &site, member_raw, charters, sites)?,
                    parse_command(dir, &site, member_raw, secrets)?,
                    Vec::new(),
                    None,
                )
            }
            Some(_) => {
                let resolved = resolve_reference(
                    agents,
                    sites,
                    dir,
                    &site,
                    &site,
                    member_raw,
                    secrets,
                    Site::Member,
                    boundary,
                )?;
                (
                    resolved.role_path,
                    resolved.command,
                    resolved.candidates,
                    resolved.hands,
                )
            }
        };
        let law = SiteLaw {
            boundary,
            dir,
            agent_hands: agent_hands.as_ref(),
        };
        enforce_model_policy(&site, member_raw, &candidates, secrets, agents, law, sites)?;
        record_hands(&site, member_raw, agent_hands, secrets, sites)?;
        members.push(PanelMember {
            name: name.clone(),
            role_path,
            command,
            candidates,
        });
    }
    Ok((members, aggregate))
}

/// Parse a sequence body: named steps, each a single driver or a panel,
/// run serially inside one effect. At least two steps (a one-step
/// sequence is a single seat); names unique case-insensitively.
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
#[expect(
    clippy::excessive_nesting,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn parse_sequence(
    dir: &Path,
    phase: &str,
    raw: &Value,
    agents: &mut Option<AgentContext>,
    sites: &mut BTreeMap<String, SiteFacts>,
    compile: BodyCompile<'_>,
) -> Result<Vec<SequenceStep>, CompileError> {
    let BodyCompile {
        results,
        secrets,
        dialect,
        boundary,
        charters,
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
        refuse_boundary_key(&what, step_raw)?;
        refuse_crossing_keys(&what, step_raw)?;
        refuse_confine(&what, step_raw)?;
        refuse_driver_keys(&what, step_raw)?;
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
        if has_panel {
            refuse_tools_on_container(&what, step_raw)?;
        }
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
                    // No executable site is compiled for an unsupported
                    // check, so a `tools` declaration here would have no
                    // owner and could only be discarded (design D5.2).
                    if step_raw.get("tools").is_some() {
                        return Err(CompileError::Invalid(format!(
                            "sequence step '{what}' declares 'tools' on a dialect step whose \
                             '{operation}' the dialect does not supply; no executable site \
                             exists to own the declaration, so it could only be discarded — \
                             refused (decision 0065 slice one, design D5)"
                        )));
                    }
                    continue;
                }
                return Err(CompileError::Invalid(format!(
                    "dialect '{}' declares phase '{phase}' {operation} unsupported",
                    dialect.name
                )));
            };
            let synthetic = dialect_gate_site(&what, boundary)?;
            // The dialect's validator is an exec-generated check: it can
            // own a checked empty declaration and represents no nonempty
            // local field (design D5.2), judged against the command the
            // validator actually runs.
            record_inline_tools(
                dir,
                &what,
                step_raw,
                false,
                &command_parts(&synthetic),
                agents.as_ref().map(|context| &context.adapters),
                sites,
            )?;
            let law = SiteLaw {
                boundary,
                dir,
                agent_hands: None,
            };
            enforce_model_policy(&what, &synthetic, &[], secrets, agents, law, sites)?;
            record_hands(&what, &synthetic, None, secrets, sites)?;
            StepBody::Dialect {
                execution: DialectExecution {
                    argv: command.argv.clone(),
                    state: command.state.clone(),
                },
            }
        } else if has_agent {
            let resolved = resolve_reference(
                agents,
                sites,
                dir,
                &what,
                &what,
                step_raw,
                secrets,
                Site::Member,
                boundary,
            )?;
            agent_hands = resolved.hands;
            StepBody::Single {
                role_path: resolved.role_path,
                command: resolved.command,
                candidates: resolved.candidates,
            }
        } else if has_panel {
            let (members, aggregate) = parse_panel(
                dir,
                &what,
                step_raw,
                &step_results,
                secrets,
                agents,
                sites,
                boundary,
                charters,
            )?;
            StepBody::Panel { members, aggregate }
        } else {
            record_inline_tools(
                dir,
                &what,
                step_raw,
                false,
                &command_parts(step_raw),
                agents.as_ref().map(|context| &context.adapters),
                sites,
            )?;
            StepBody::Single {
                role_path: parse_role(dir, &what, step_raw, charters, sites)?,
                command: parse_command(dir, &what, step_raw, secrets)?,
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
                let law = SiteLaw {
                    boundary,
                    dir,
                    agent_hands: agent_hands.as_ref(),
                };
                enforce_model_policy(&what, step_raw, candidates, secrets, agents, law, sites)?;
                record_hands(&what, step_raw, agent_hands, secrets, sites)?;
            }
            StepBody::Panel { .. } => refuse_class_without_a_driver(&what, step_raw)?,
            StepBody::Dialect { .. } => {}
        }
        steps.push(SequenceStep {
            name: name.to_string(),
            class: step_class(step_raw),
            results: step_results,
            body,
        });
    }
    Ok(steps)
}

/// The charters a compile bound, each kept with the layer that declared it
/// until that layer's identity is sealed from its buffer (design D7).
type Charters = std::cell::RefCell<Vec<compose::CharterRead>>;

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
pub fn expand_command(dir: &Path, parts: &[String]) -> Vec<String> {
    parts
        .iter()
        .map(|part| {
            if part == "{brokkr}" {
                return brokkr_executable(std::env::current_exe());
            }
            match part.strip_prefix("./") {
                Some(rel) => dir.join(rel).to_string_lossy().into_owned(),
                None => part.clone(),
            }
        })
        .collect()
}

/// [`expand_command`] over a composition, one segment at a time (design
/// D5.7): each token keeps the origin of the segment that supplied it, and
/// because the expansion is token for token, the expanded segments are
/// exactly the expanded argv, in order. A lowering that never composed
/// has nothing to expand.
pub(crate) fn expand_lowering(dir: &Path, lowering: &Lowering) -> Lowering {
    match lowering {
        Lowering::Composed(composition) => Lowering::Composed(Composition {
            segments: composition
                .segments
                .iter()
                .map(|segment| Segment {
                    origin: segment.origin,
                    argv: expand_command(dir, &segment.argv),
                })
                .collect(),
            // Rebuild unit 5c-fix2: the declared template is expanded as
            // its segment is, so the seal compares like with like.
            template: match &composition.template {
                TemplateExpectation::None => TemplateExpectation::None,
                TemplateExpectation::Declared(argv) => {
                    TemplateExpectation::Declared(expand_command(dir, argv))
                }
            },
            ..composition.clone()
        }),
        other => other.clone(),
    }
}

fn brokkr_executable(current: std::io::Result<PathBuf>) -> String {
    match current {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => "brokkr".to_string(),
    }
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

/// Fold every site's pinned driver digests into the manifest's `drivers`
/// projection, merging a site's exemption/resume witness with its inline
/// driver evidence under the one label the family table keys on (design
/// D10 F1). A site with neither fact contributes no entry, exactly as the
/// two formerly separate maps did.
fn fold_driver_facts(drivers: &mut Map<String, Value>, sites: &BTreeMap<String, SiteFacts>) {
    for (label, facts) in sites {
        let mut merged = Map::new();
        for extra in [&facts.pin_drivers, &facts.driver].into_iter().flatten() {
            for (provider, digest) in extra {
                merged.insert(provider.clone(), digest.clone());
            }
        }
        if !merged.is_empty() {
            drivers.insert(label.clone(), Value::Object(merged));
        }
    }
}

/// The pinned, content-addressed identity of a bundle. `chain` is the
/// resolved composition (decision 0017): each ancestor rides in `files`
/// under the reserved `@compose/` prefix, zero-padded so canonical key
/// sorting preserves chain order, so changing a base moves the digest of
/// everything derived from it — and so the chain survives the dispatch
/// manifest round-trip, which copies `files` verbatim. `agents` is the
/// resolution record (decision 0016), pinned for the same reason.
/// `consumed` carries the digests of the layer's bound buffers into its
/// walk ([`walk_files`]).
#[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
fn manifest_for(
    dir: &Path,
    bundle_name: &str,
    chain: &[Ancestor],
    agents: Option<&Map<String, Value>>,
    drivers: Option<&Map<String, Value>>,
    hands: &BTreeMap<String, HandsSpec>,
    select: &Map<String, Value>,
    boundary: Boundary,
    capabilities: Option<Value>,
    consumed: &BTreeMap<String, Supplied>,
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
    for (rel, digest) in walk_files(dir, dir, consumed)? {
        files.insert(rel, Value::String(digest));
    }
    let mut manifest = json!({
        "engine": ENGINE_VERSION,
        "event_schema": brokkr_core::envelope::EVENT_SCHEMA_VERSION,
        "database_schema": brokkr_store::DATABASE_SCHEMA,
        "driver_protocol": DRIVER_PROTOCOL,
        "bundle_name": bundle_name,
        "files": Value::Object(files),
    });
    // ALWAYS present in a compiled bundle, unlike every key below
    // (decision 0065 ruling 8; run-manifest v11): "this realm grants
    // nothing and this seat holds nothing" is a fact about the bundle, and
    // a manifest that merely lacked the key could not be told from one
    // written before the word existed. A changed grant, scope, tool
    // subset, restriction, definition or dialect byte moves the digest.
    // Absent only from an ANCESTOR layer's own digest, which is compiled
    // in no realm and holds nothing.
    if let Some(capabilities) = capabilities {
        manifest["capabilities"] = capabilities;
    }
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
    // boxes no hands keeps its v5 shape and identity byte for byte. The
    // resolved boundary rides beside it, one entry per hands site and
    // every entry the realm's word (decision 0046 ruling 1; run-manifest
    // v9): written in the same loop, so the two key sets are equal by
    // construction, and absent with it, so a plain bundle keeps its
    // pre-0046 identity (design DD4).
    if !hands.is_empty() {
        let mut specs = Map::new();
        let mut boundaries = Map::new();
        for (site, spec) in hands {
            specs.insert(site.clone(), spec.to_value());
            boundaries.insert(site.clone(), Value::String(boundary.word().to_string()));
        }
        manifest["hands"] = Value::Object(specs);
        manifest["boundary"] = Value::Object(boundaries);
    }
    // Absent for every non-selecting bundle: v6 identity is preserved.
    if !select.is_empty() {
        manifest["select"] = Value::Object(select.clone());
    }
    Ok(manifest)
}

/// The manifest walk over one layer's directory: every regular file
/// under `scope`, keyed relative to the layer `dir` and digested, in
/// sorted key order. Compile walks the layer; spawn walks the script's
/// directory with the same filename and byte identity rules.
///
/// A key in `consumed` is a file a bound read already supplied (design D7;
/// rebuild unit 16-fix-b, F3): its digest is the digest of the buffer that
/// was parsed, and the walk never opens its path to hash it again. The
/// caller verifies that each such input still stands as it was read before
/// the digest is sealed.
///
/// The walk is the one place that sees every name in the layer, so it holds
/// each consumed file to exactly one entry (rebuild unit 16-fix-d): a
/// consumed key it lists no entry for is refused, naming who consumed it,
/// a file under no consumed key that IS a consumed file under another entry
/// is refused as another name for it (16-fix-c, F1), and a consumed file
/// met under a second entry, consumed or not, is refused naming both. What
/// was consumed is known by its binding, not by the name the walk happens
/// to list it under. A link is its own entry, so a contained link to a
/// consumed file is one name whose target is the other; and a path through
/// a contained linked directory lists the target's own entry again, which
/// is that entry, never a second one ([`Entry`]).
///
/// A tree under a top-level name the walk skips is still searched for such a
/// second entry whenever the layer consumed anything (rebuild unit 16-fix-d,
/// return F1). It is never pinned, so an unconsulted definition there moves
/// no identity; but a hard link there to a consumed file is a second name for
/// it inside the layer all the same, and is refused on the same terms. That
/// search never follows a link (16-fix-e, F3): a linked directory there is
/// one entry, not a tree, so it cannot leave the layer or loop; a directory
/// there is opened through its parent's handle, never following a link, and
/// listed through that handle only while it is the one its parent listed
/// ([`listing`]; 16-fix-e, return F3 and second return F2); and an entry
/// there it cannot ask what it is is refused naming it (16-fix-e, second
/// return F1). A directory the walk cannot list, skipped or not, is refused
/// naming it, and naming who read a consumed entry it holds.
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn walk_files(
    dir: &Path,
    scope: &Path,
    consumed: &BTreeMap<String, Supplied>,
) -> Result<BTreeMap<String, String>, CompileError> {
    // Each directory to list, with how its parent found it when it stands
    // in a skipped tree.
    let mut stack: Vec<(PathBuf, Option<Found>)> = vec![(scope.to_path_buf(), None)];
    let (mut paths, mut unpinned) = (Vec::new(), Vec::new());
    while let Some((current, found)) = stack.pop() {
        let in_skipped = found.is_some();
        let (handle, names) = listing(dir, &current, found, consumed)?;
        let handle = std::rc::Rc::new(handle);
        for name in names {
            let path = current.join(&name);
            // A consumed entry is the one entry it was read by, never a tree
            // to descend (16-fix-e, return F1): whatever stands there now is
            // judged by its own check, which names who read it.
            let consumed_here = consumed.contains_key(&walk_key(dir, &path).unwrap_or_default());
            // A scaffold may also be the workspace from which Brokkr is
            // invoked. Its realm map and dialect library are workspace
            // declarations pinned into the RUN manifest, never bundle files:
            // changing either must not move the strategy's identity.
            let skipped =
                in_skipped || current == dir && name.to_str().is_some_and(unpinned_top_level);
            if skipped && consumed.is_empty() {
                continue;
            }
            if skipped {
                // Asked of the entry, never its target (16-fix-e, F3): a
                // linked directory here is one entry, not a tree. An entry
                // it cannot ask is refused as itself (second return F1).
                at_stage(ReadStage::Skipped, &path);
                let meta = std::fs::symlink_metadata(&path)
                    .map_err(|error| unobservable(dir, &path, &error))?;
                if meta.is_dir() {
                    let found = (std::rc::Rc::clone(&handle), name, node(&meta));
                    stack.push((path, Some(found)));
                } else {
                    unpinned.push(path);
                }
            } else if !consumed_here && path.is_dir() {
                stack.push((path, None));
            } else if consumed_here || path.is_file() {
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
    let mut walked = Vec::with_capacity(paths.len());
    for path in paths {
        // Join actual components, never replace bytes inside a name:
        // Unix `scripts\gate.sh` is a different file from `scripts/gate.sh`.
        // Refuse names JSON cannot represent instead of losing bytes.
        let rel = path
            .strip_prefix(dir)
            .expect("walked under dir")
            .iter()
            .map(|name| {
                name.to_str().ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "bundle filename '{}' is not UTF-8; manifest keys must preserve exact names",
                        path.display()
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        // The composition namespace is computed, never read from disk:
        // a real file under it could otherwise forge provenance.
        if rel.starts_with(COMPOSE_PREFIX) {
            return Err(CompileError::Invalid(format!(
                "bundle file '{rel}' uses the reserved '{COMPOSE_PREFIX}' namespace; \
                 composition provenance is computed by the resolver, never \
                 supplied by a bundle"
            )));
        }
        walked.push((path, rel));
    }
    if let Some((key, supplied)) = consumed
        .iter()
        .find(|(key, _)| !walked.iter().any(|(_, rel)| rel == *key))
    {
        return Err(CompileError::Invalid(format!(
            "{}, which the walk that pins the layer lists under no entry of the name {} it was \
             read by: the name reached the file through a case or normalization alias the \
             filesystem accepted, or the entry was removed after the read. A layer's identity \
             names a consumed file by the entry its directory lists, so it is refused; write the \
             reference as its directory lists it (decision 0065 slice one, design D7)",
            supplied.consumer,
            bounded_reference(key)
        )));
    }
    let mut named: BTreeMap<(u64, u64), (Entry, String)> = BTreeMap::new();
    let mut files = BTreeMap::new();
    for (path, rel) in walked {
        let supplied = consumed.get(&rel);
        if supplied.is_none() {
            at_stage(ReadStage::Walked, &path);
        }
        one_entry(dir, &path, &rel, supplied, consumed, &mut named)?;
        let digest = match supplied {
            Some(supplied) => supplied.digest.clone(),
            None => sha256_bytes(&std::fs::read(&path)?),
        };
        files.insert(rel, digest);
    }
    // Judged after every pinned entry, so a consumed file's first entry is
    // the one it was read by; nothing here is read or pinned.
    for path in unpinned {
        let rel = path
            .strip_prefix(dir)
            .expect("walked under dir")
            .to_string_lossy();
        one_entry(dir, &path, &rel, None, consumed, &mut named)?;
    }
    Ok(files)
}

/// How the walk found a directory in a skipped tree: the handle on the
/// directory its parent's listing named it in, its name there, and the
/// `(dev, ino)` it was, asked without following a link.
type Found = (std::rc::Rc<std::fs::File>, OsString, (u64, u64));

/// The names `current`, a directory under the layer `dir`, lists, and the
/// handle they were listed through: the listing reads that handle, never a
/// path (rebuild unit 16-fix-e, second return F2). A pinned directory is
/// opened as its path reaches it. One in a skipped tree was `found` by its
/// parent's listing (return F3): it is opened through its parent's handle,
/// never following a link, and listed only while that handle holds the
/// directory found and its entry still stands as it, checked when it is
/// opened and again after the listing. A link or anything else put in its
/// place is refused before a name listed there is walked, and the walk never
/// opens or lists a directory outside the layer.
fn listing(
    dir: &Path,
    current: &Path,
    found: Option<Found>,
    consumed: &BTreeMap<String, Supplied>,
) -> Result<(std::fs::File, Vec<OsString>), CompileError> {
    let refuse = |error| unlisted(dir, current, error, consumed);
    at_stage(ReadStage::Listing, current);
    let Some((parent, name, id)) = found else {
        let handle = directory(current).map_err(refuse)?;
        let names = names_in(&handle).map_err(refuse)?;
        return Ok((handle, names));
    };
    let stands = || std::fs::symlink_metadata(current).is_ok_and(|meta| node(&meta) == id);
    let opened = directory_at(&parent, &name)
        .and_then(|handle| handle.metadata().map(|meta| (handle, node(&meta))));
    let handle = match opened {
        Ok((handle, held)) if held == id => handle,
        Err(error) if stands() => return Err(refuse(error)),
        _ => return Err(replaced(dir, current, "before")),
    };
    at_stage(ReadStage::DirectoryChecked, current);
    let names = names_in(&handle).map_err(refuse)?;
    if !stands() {
        return Err(replaced(dir, current, "while"));
    }
    Ok((handle, names))
}

/// The refusal of `directory`, in a skipped tree under the layer `dir`,
/// which stood as another entry `moment` the walk listed it.
fn replaced(dir: &Path, directory: &Path, moment: &str) -> CompileError {
    let key = walk_key(dir, directory).unwrap_or_default();
    CompileError::Invalid(format!(
        "bundle directory {} was replaced {moment} the walk listed it: a tree the walk skips is \
         searched only to hold each consumed file to one name, entered as the directory its \
         parent listed there and never through a link, so a directory replaced there is refused \
         rather than followed (decision 0065 slice one, design D7)",
        bounded_reference(&format!("./{key}"))
    ))
}

/// The refusal of the entry at `path`, in a skipped tree under the layer
/// `dir`, which the walk listed but could not ask what it is (rebuild unit
/// 16-fix-e, second return F1 and F3): named by its own place in the layer,
/// never as its directory, with the kind of failure, never an unbounded io
/// message.
fn unobservable(dir: &Path, path: &Path, error: &std::io::Error) -> CompileError {
    let key = walk_key(dir, path).unwrap_or_default();
    CompileError::Invalid(format!(
        "bundle entry {}, in a tree the walk skips, cannot be observed ({}): the walk asks each \
         entry there what it is, without following a link, to hold each consumed file to one \
         name, so an entry it cannot observe is refused rather than passed over (decision 0065 \
         slice one, design D7)",
        bounded_reference(&format!("./{key}")),
        error.kind()
    ))
}

/// The refusal of `directory`, under the layer `dir`, which the walk could
/// not list (rebuild unit 16-fix-e, F3): named by its place in the layer,
/// with the kind of failure, never an unbounded io message. Where it holds a
/// consumed entry, the refusal opens with who read that entry, as the
/// input's own refusals do (16-fix-e, return F1).
fn unlisted(
    dir: &Path,
    directory: &Path,
    error: std::io::Error,
    consumed: &BTreeMap<String, Supplied>,
) -> CompileError {
    let key = walk_key(dir, directory).unwrap_or_default();
    let refusal = format!(
        "bundle directory {} cannot be listed ({}): the walk that pins a layer lists every \
         directory it enters, a skipped tree's included where it holds each consumed file to one \
         name, so a directory it cannot list is refused rather than passed over (decision 0065 \
         slice one, design D7)",
        bounded_reference(&format!("./{key}")),
        error.kind()
    );
    match consumed
        .iter()
        .find(|(held, _)| Path::new(held).starts_with(&key))
    {
        Some((held, supplied)) => CompileError::Invalid(format!(
            "{}, whose entry {} stands in {refusal}",
            supplied.consumer,
            bounded_reference(held)
        )),
        None => CompileError::Invalid(refusal),
    }
}

/// The `(dev, ino)` `meta` describes.
#[cfg(unix)]
fn node(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
}

/// As [`entry_of`]: any other host consumes nothing, and so never searches a
/// skipped tree.
#[cfg(not(unix))]
fn node(_: &std::fs::Metadata) -> (u64, u64) {
    (0, 0)
}

/// The refusal of a consumed input whose entry `key` the walk could not
/// observe (rebuild unit 16-fix-e, F1): removed, replaced or made
/// unobservable after the read. It names who consumed it, as the input's own
/// refusals do, and the kind of failure, never a bare io message.
fn unobserved(held: &Supplied, key: &str, error: &std::io::Error) -> CompileError {
    CompileError::Invalid(format!(
        "{}, whose entry {} the walk that pins the layer cannot observe ({}): it was removed, \
         replaced or made unobservable after the read. A consumed input the walk cannot observe \
         is refused rather than pinned unobserved (decision 0065 slice one, design D7)",
        held.consumer,
        bounded_reference(key),
        error.kind()
    ))
}

/// Hold the walked `path`, keyed `rel` and consumed as `supplied` or not, to
/// the one-entry rule of [`walk_files`], `named` holding each consumed
/// file's first entry.
fn one_entry(
    dir: &Path,
    path: &Path,
    rel: &str,
    supplied: Option<&Supplied>,
    consumed: &BTreeMap<String, Supplied>,
    named: &mut BTreeMap<(u64, u64), (Entry, String)>,
) -> Result<(), CompileError> {
    // A consumed key the walk cannot observe names who read it (16-fix-e,
    // F1); an entry under no consumed key is not known to be one.
    let found = consumed_entry(path, consumed);
    let found = match supplied {
        Some(supplied) => found.map_err(|error| unobserved(supplied, rel, &error))?,
        None => found?,
    };
    let Some((held, entry)) = found else {
        return Ok(());
    };
    // A path through a linked directory lists the target's own entry again:
    // the same file, under the same entry, is the consumed file and not
    // another name for it.
    let same = match supplied {
        Some(supplied) => supplied.id == held.id,
        None => {
            let target = entry_of(&dir.join(&held.target));
            entry == target.map_err(|error| unobserved(held, &held.target, &error))?
        }
    };
    if !same {
        return Err(CompileError::Invalid(format!(
            "bundle file {} is another name for {}, the file a bound read consumed: a layer's \
             identity names a consumed file by the entry it was read by, and a second name for \
             it inside the layer is refused rather than walked as a file nothing consumed \
             (decision 0065 slice one, design D7)",
            bounded_reference(rel),
            bounded_reference(&held.target)
        )));
    }
    let (first_entry, first) = named
        .entry(held.id)
        .or_insert_with(|| (entry.clone(), rel.to_string()));
    if *first_entry != entry {
        return Err(CompileError::Invalid(format!(
            "bundle files {} and {} are two names for one file a bound read consumed: a layer's \
             identity names a consumed file by exactly one entry, and a second name for it \
             inside the layer, consumed or not, is refused rather than bound twice (decision \
             0065 slice one, design D7)",
            bounded_reference(first),
            bounded_reference(rel)
        )));
    }
    Ok(())
}

/// One directory entry: the `(dev, ino)` of the directory that holds it,
/// and its name there. Two walked paths are one entry exactly when both
/// agree, however many linked directories either passed through; two hard
/// links are two entries.
type Entry = ((u64, u64), OsString);

/// The entry `path` names: its directory, following links on the way, and
/// its own name.
#[cfg(unix)]
fn entry_of(path: &Path) -> std::io::Result<Entry> {
    use std::os::unix::fs::MetadataExt;
    let parent = std::fs::metadata(path.parent().expect("a walked path has a parent"))?;
    let name = path.file_name().expect("a walked path has a name");
    Ok(((parent.dev(), parent.ino()), name.to_os_string()))
}

/// The consumed file the entry at `path` is, when it is one — the same
/// `(dev, ino)` as a file a bound read held — with the entry it is. The
/// entry itself is asked, never a link's target, so a link to a consumed
/// file is walked as the link it is.
#[cfg(unix)]
fn consumed_entry<'a>(
    path: &Path,
    consumed: &'a BTreeMap<String, Supplied>,
) -> std::io::Result<Option<(&'a Supplied, Entry)>> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::symlink_metadata(path)?;
    let id = (meta.dev(), meta.ino());
    match consumed.values().find(|supplied| supplied.id == id) {
        Some(supplied) => Ok(Some((supplied, entry_of(path)?))),
        None => Ok(None),
    }
}

/// No supported host lacks the identity; any other consumes nothing, since
/// its every bound read refuses (design D7).
#[cfg(not(unix))]
fn entry_of(_: &Path) -> std::io::Result<Entry> {
    Err(std::io::ErrorKind::Unsupported.into())
}

/// As [`entry_of`]: any other host consumes nothing.
#[cfg(not(unix))]
fn consumed_entry<'a>(
    _: &Path,
    _: &'a BTreeMap<String, Supplied>,
) -> std::io::Result<Option<(&'a Supplied, Entry)>> {
    Ok(None)
}

#[cfg(test)]
mod agent_tests;
#[cfg(test)]
mod compose_tests;

#[cfg(test)]
mod model_policy_tests;

#[cfg(test)]
pub(crate) mod secret_binding_tests;

#[cfg(test)]
mod tests;
