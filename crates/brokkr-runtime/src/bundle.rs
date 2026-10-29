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
use brokkr_core::policy::Machine;
// The test modules reach it through `use super::*`.
#[cfg(test)]
use brokkr_core::policy::is_declarable;
use brokkr_core::realms::Boundary;
use serde_json::{json, Map, Value};
use thiserror::Error;

mod aggregate;
mod assemble;
mod capability_record;
mod charters;
pub mod compose;
pub mod entry;
mod expand;
mod gate;
pub use gate::{GateAssessment, WorkSite};
mod invocation;
mod mcp;
mod model_policy;
mod opened;
mod origins;
mod parse;
mod phases;
mod pins;
mod reference;
mod route_policy;
mod sandbox;
mod sequence;
mod site_form;
mod site_key;
mod survey;
mod tier;
mod walk;

pub use aggregate::Aggregate;
use capability_record::{record_capabilities, CapabilityWalk};
use charters::parse_role;
use compose::{Ancestor, COMPOSE_PREFIX};
pub use expand::expand_command;
pub(crate) use expand::expand_lowering;
pub use invocation::Invocation;
pub(crate) use mcp::{McpContribution, McpIntent};
pub(crate) use model_policy::{built_in_model_driver, short_flag};
#[cfg(test)]
use model_policy::{command_pins_model, effort_exempt};
use model_policy::{enforce_model_pins, inline_route_pin, model_unpinned, route_pin};
use model_policy::{ModelPin, MODEL_FLAG};
use opened::Opened;
use parse::{SeatScope, SiteCompiler};
use reference::{agent_name, bind_charter, judge_mapped_hands, resolved_candidates, ReferenceAt};
use route_policy::{judge_link, RouteLink};
use sandbox::{inline_codex_fragment, judge_sandbox_link, refuse_authored_sandbox, SandboxAsk};
use sequence::{sequence_steps, step_body, step_class, step_name, step_results, StepAt};
use site_form::{classify, SiteRole, StepShape, STEP};
#[cfg(test)]
use site_form::{refuse_amendments, refuse_boundary_key, refuse_confine};
pub(crate) use site_key::SiteKey;
pub use survey::{survey, Survey};
pub use tier::{ProvisionalRefusal, RealmLaw};
use walk::{
    refuse_oversized, refuse_unlisted_consumed, skipped_path, unpinned_active_input, walk_digests,
    walk_keys, Walk,
};

#[cfg(test)]
use crate::agents::{resolve_route, Lowering, TrustTier};
use crate::agents::{
    Adapter, Adapters, Availability, Candidate, EgressClass, HandsNotice, Sandbox,
};
use crate::argv::{Dispatch, Executable};
use crate::dialect::{Dialect, DialectPhase};
use crate::phase_name;
use crate::places::Shelves;
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
    /// A composed bundle's `Policy` refusal and its chain note.
    #[error("bundle: bundle policy: {0} ({1})")]
    ComposedPolicy(Box<brokkr_core::PolicyError>, String),
    #[error("bundle: {0}")]
    Provisional(ProvisionalRefusal),
    /// GP2 (decision 0065 slice two, U3c): an inline site's verified
    /// charter names an ask it writes in no DATA paragraph.
    #[error("bundle: {0}")]
    Charter(crate::agents::charter_data::CharterRefusal),
    /// A layer naming what its origin may not compose from (#697).
    #[error("bundle: {0}")]
    Shelf(crate::places::ShelfRefusal),
    /// A seat that cannot be the folder's own (decision 0091 ruling 1).
    #[error("bundle: {0}")]
    Folder(compose::FolderSeatRefusal),
}
/// How [`CompileError::Capability`] renders.
fn capability_line(reason: &str) -> String {
    brokkr_protocol::native_controls::bounded_line(&format!("bundle: {reason}"))
}
/// The engine-owned inputs live with the policy that reads them, because
/// the loader's presence refusal (decision 0050, ruling 1) exempts them.
pub use brokkr_core::policy::{is_engine_owned, ENGINE_OWNED_INPUTS, REALM_FACTS};

/// The two results synthesized by a dialect exec for each phase. This is
/// also the vocabulary recorded on every compiled dialect step, so dispatch
/// and sequence-boundary reasoning cannot drift apart.
pub(crate) fn dialect_results(phase: &str) -> [&'static str; 2] {
    match DialectPhase::parse(phase) {
        Some(DialectPhase::Clarify) => ["clear", "ambiguous"],
        Some(DialectPhase::Analyze) => ["consistent", "drift"],
        None if phase == phase_name::VERIFY => ["pass", "fail"],
        Some(DialectPhase::Specify | DialectPhase::Design | DialectPhase::Tasks) | None => {
            ["drafted", "fail"]
        }
    }
}
#[derive(Debug, Clone)]
pub struct Seat {
    /// True when every invocation site in this seat is a gate. The runtime
    /// uses the compiled fact to enforce a stable repository HEAD around the
    /// whole effect — but only where the effect has no inner spans of its
    /// own. A sequence, all-gate or mixed, is watched per step instead: a
    /// step carries its own class, and that step is the gate.
    pub gate: GateAssessment,
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
/// boundary (see [`site_form::refuse_confine`]).
#[derive(Debug, Clone)]
pub enum SeatBody {
    /// One driver or a panel, the single carrying its own class.
    Invocation(Invocation),
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
    },
}
#[derive(Clone, Copy)]
pub enum ExecutableBody<'a> {
    Invocation(&'a Invocation),
    Sequence { steps: &'a [SequenceStep] },
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
            SeatBody::Invocation(invocation) => ExecutableBody::Invocation(invocation),
            SeatBody::Sequence { steps } => ExecutableBody::Sequence { steps },
            SeatBody::Select { .. } => return None,
        };
        Some((executable, case))
    }

    pub fn selected_is_gate(&self, strategy: Option<&str>, ordinary: bool) -> bool {
        match self {
            SeatBody::Select { cases, default } => strategy
                .and_then(|case| cases.get(case))
                .or(default.as_deref())
                .is_some_and(SeatBody::is_gate),
            _ => ordinary,
        }
    }

    pub(crate) fn is_gate(&self) -> bool {
        matches!(self.assess(), GateAssessment::AllGate)
    }
}

impl Seat {
    pub fn has_gate(&self) -> bool {
        matches!(self.gate, GateAssessment::AllGate)
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
    /// One driver or a panel; the step holds the single's class.
    Invocation(Invocation<()>),
    /// A realm-dialect validator or deterministic loop check.
    Dialect { execution: DialectExecution },
}

#[derive(Debug, Clone)]
pub struct PanelMember {
    pub name: String,
    pub role_path: PathBuf,
    pub command: Vec<String>,
    pub candidates: Vec<Candidate>,
    pub written: Vec<String>,
    pub class: SeatClass,
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

    /// The class of a whole that judges exactly when `gate` says it does.
    fn of(gate: bool) -> SeatClass {
        match gate {
            true => SeatClass::Gate,
            false => SeatClass::Work,
        }
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
    /// #430: the model ids each of those candidates is served on, as the
    /// compile read them, in candidate order; sealed beside them.
    pub(crate) pins: Vec<crate::capabilities::manifest::ModelPins>,
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
    /// #896 round 2: the office's declared hands access; `None` inline.
    pub access: Option<brokkr_protocol::native_controls::Access>,
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
    /// Every layer's origin, leaf first, as `roots` (#697 ruling 5).
    pub origins: Vec<crate::places::Origin>,
    /// The boundary this bundle was compiled under (decision 0046 ruling
    /// 1): the operated realm's word, `namespace` in no realm. Pinned per
    /// hands site in the manifest's `boundary` map, and the one word the
    /// engine composes every site with hands from (design DD3).
    pub boundary: Boundary,
    /// The engine executable every `{brokkr}` was expanded to (#349, ask
    /// 8), carried to the engine so its spawns name the same binary.
    /// Machine-local, so never digested.
    pub executable: Executable,
    /// The composition chain, nearest ancestor first (decision 0017).
    /// Empty unless the recipe extends another.
    pub chain: Vec<Ancestor>,
    /// The folder layer a seat was taken from, last in `roots` (0091).
    pub folder: Option<Ancestor>,
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

    /// Adapters no site opened, or an optional read that failed to load;
    /// adapters that loaded are read as `loaded` ([`Opened`]).
    fn unloaded(pinned: Option<&'a Result<Adapters, String>>) -> Self {
        CapabilityAdapters {
            adapters: None,
            unloaded: pinned
                .and_then(|loaded| loaded.as_ref().err())
                .map(String::as_str),
        }
    }
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
        let leaf = (dir, &Shelves::default());
        Self::compile_unmapped_on(leaf, library_root, adapters_root, law, operator_root)
    }

    /// [`Bundle::compile_unmapped`] of the bundle at `leaf`'s directory,
    /// composed across `leaf`'s shelves (#697).
    pub(crate) fn compile_unmapped_on(
        leaf: (&Path, &Shelves),
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
        Self::compile_as(
            &Executable::current(),
            leaf,
            (library_root, adapters_root),
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
        let (leaf, roots) = ((dir, &Shelves::default()), (library_root, adapters_root));
        let engine = Executable::current();
        Self::compile_as(&engine, leaf, roots, realm_name, dialect, law, capabilities)
    }

    /// [`Bundle::compile_with_capabilities`] of the bundle at `leaf`'s
    /// directory, composed across `leaf`'s shelves (#697 ruling 3), for the
    /// engine executable `engine`, the one value its every `{brokkr}`
    /// expands to and the compiled bundle carries to the engine (#349, ask
    /// 8).
    pub(crate) fn compile_as(
        engine: &Executable,
        (dir, shelves): (&Path, &Shelves),
        roots: (&Path, &Path),
        realm_name: Option<&str>,
        dialect: Option<&Dialect>,
        law: impl Into<RealmLaw>,
        capabilities: &crate::capabilities::CapabilityContext,
    ) -> Result<Bundle, CompileError> {
        let (dir, resolved) = survey::composed(dir, shelves)?;
        let note = resolved.chain_note();
        let realm = (realm_name, dialect, law.into());
        Bundle::assemble(&dir, resolved, roots, engine, realm, capabilities)
            // A composed bundle's failure is wrapped ONCE with the chain: a
            // capability keeps its raw words, a policy refusal its type. A
            // folder seat's refusal rises as the resolver's do, unwrapped.
            .map_err(|error| match (note, error) {
                (Some(note), CompileError::Capability(reason)) => {
                    CompileError::Capability(format!("bundle: {reason} ({note})"))
                }
                (Some(note), CompileError::Policy(error)) => {
                    CompileError::ComposedPolicy(Box::new(error), note)
                }
                (_, error @ CompileError::Folder(_)) | (None, error) => error,
                (Some(note), error) => CompileError::Invalid(format!("{error} ({note})")),
            })
    }

    /// The agent roots ride through: composition resolves the bundle,
    /// then agent references inside the RESOLVED seats resolve against
    /// the library and adapters (decisions 0016 and 0017 layered), and
    /// every `{brokkr}` expands to `engine`.
    fn assemble(
        dir: &Path,
        mut resolved: compose::Resolved,
        (library_root, adapters_root): (&Path, &Path),
        engine: &Executable,
        (realm_name, dialect, law): (Option<&str>, Option<&Dialect>, RealmLaw),
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
        let description = assemble::config_text(config, "description");
        let cost = assemble::config_text(config, "cost");
        let table = phases::table(&resolved, realm_name, dialect)?;
        let authority = authority.with_minimum(table.declared_minimum);
        let wrapped_verify = assemble::wrapped_verify(table.uses_dialect, dialect);
        let charters = Charters::default();
        let roots = (library_root, adapters_root);
        let at = (&authority, &resolved, engine, &charters);
        let mut seated = survey::seat(at, roots, (dialect, law), &table)?;
        let compile = &mut survey::Refusals::Compile;
        seated.settle(&table.machine, adapters_root, compile)?;
        seated.wrap(wrapped_verify, compile)?;
        let assemble::Seated {
            boundary,
            agents,
            sites,
            seats,
            ..
        } = seated;
        let phases::Table {
            machine,
            dialect_prompts,
            protected_phase,
            ..
        } = table;

        let capability_record = assemble::capability_record(&authority, &agents, &sites);
        let select_records = assemble::select_records(&seats);
        let projections = assemble::projections(&sites);
        let drivers = (!projections.drivers.is_empty()).then_some(&projections.drivers);
        assemble::seal_layers(&agents.reads, &mut resolved, charters)?;
        let mut manifest = manifest_for(
            dir,
            &name,
            &resolved.chain,
            Some(&projections.records),
            drivers,
            &projections.hands,
            &select_records,
            boundary,
            Some(capability_record),
            &resolved.leaf_digests(),
        )?;
        let folder = resolved.pin_folder(&mut manifest);
        resolved.check_leaf(manifest["files"].as_object().expect("manifest files"))?;
        let charters = assemble::charter_pins(&sites);
        Ok(Bundle {
            hands: projections.hands,
            inline_resume: projections.inline_resume,
            sites,
            charters,
            name,
            description,
            cost,
            dir: dir.to_path_buf(),
            roots: resolved.roots,
            origins: resolved.origins,
            boundary,
            executable: engine.clone(),
            chain: resolved.chain,
            folder,
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
        for (case, executable) in selectable(seat) {
            let sites =
                crate::engine::resume::structural_sites(executable, phase, case, seat.has_gate());
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

/// Each selectable body of `seat`, with the case that selects it: the
/// select default under the case `default`, and an unselected body under
/// none. Selection never nests, so every entry here resolves — the
/// `filter_map` carries that fact rather than a branch nothing can take.
fn selectable(seat: &Seat) -> Vec<(Option<&str>, ExecutableBody<'_>)> {
    let bodies: Vec<(Option<&str>, &SeatBody)> = match &seat.body {
        SeatBody::Select { cases, default, .. } => cases
            .iter()
            .map(|(case, body)| (Some(case.as_str()), body))
            .chain(default.as_deref().map(|body| (Some("default"), body)))
            .collect(),
        body => vec![(None, body)],
    };
    bodies
        .into_iter()
        .filter_map(|(case, body)| body.selected(None).map(|(body, _)| (case, body)))
        .collect()
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
fn owner_index(seats: &BTreeMap<String, Seat>) -> Result<BTreeMap<String, SiteKey>, CompileError> {
    let mut seen: BTreeMap<String, SiteKey> = BTreeMap::new();
    for (phase, seat) in seats {
        for (case, executable) in selectable(seat) {
            for owner in crate::engine::resume::owner_sites(executable, phase, case) {
                let label = owner.label();
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
                        first.describe(),
                        owner.describe()
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
    match body {
        SeatBody::Invocation(invocation) => invocation.manifest(),
        SeatBody::Sequence { steps } => json!({"sequence": steps.iter().map(|step| {
            let body = match &step.body {
                StepBody::Invocation(invocation) => invocation.manifest(),
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
fn resolve_reference(
    compiler: &mut SiteCompiler<'_>,
    dir: &Path,
    what: &str,
    raw: &Value,
    secrets: &[String],
    site: SiteRole,
) -> Result<ResolvedSeat, CompileError> {
    let (boundary, engine) = (compiler.boundary, compiler.engine);
    let SiteCompiler { agents, sites, .. } = compiler;
    let name = agent_name(what, raw)?;
    let (library, adapters) = agents.agent_roots()?;
    // Decision 0065 slice one (design D5.2): the site's own typed `tools`
    // narrows a PRIVATE clone of the office before composition. Shape is
    // judged here, narrowing inside the resolver, and the effective value
    // is what the chain below is composed from.
    let requested = decode_site_tools(what, raw)?;
    let report = crate::agents::report_narrowed(
        library,
        adapters,
        &Availability::unspecified(),
        name,
        boundary,
        &requested,
    )
    .map_err(|e| CompileError::Invalid(format!("seat '{what}': {e}")))?;
    let effective = report.agent.local();
    let access = report.agent.access;
    let at = ReferenceAt {
        engine,
        dir,
        what,
        raw,
        secrets,
        boundary,
        name,
    };
    judge_mapped_hands(at, &report, agents, sites)?;
    let adapters = agents.adapters()?;
    let resolution = crate::agents::resolve_report(report, adapters)
        .map_err(|e| CompileError::Invalid(format!("seat '{what}': {e}")))?;
    refuse_member_declarations(what, name, site, &resolution)?;
    let candidates = resolved_candidates(at, &resolution, adapters)?;
    site_facts(sites, what).record = Some(resolution.record.clone());
    let (pin, read) = bind_charter(what, &resolution)?;
    site_facts(sites, what).charter = Some(pin);
    agents.reads.push(read);
    // The capability pass judges exactly the chain this site will run
    // (decision 0065): one outcome per candidate, never their union.
    site_facts(sites, what).chain = candidates.clone();
    // The effective local declaration, beside the site's other facts
    // (design D5.2): a checked value even where nothing was declared.
    site_facts(sites, what).local = Some(effective);
    // #896 round 2: the office's access travels with its hands.
    site_facts(sites, what).access = Some(access);
    Ok(ResolvedSeat {
        role_path: resolution.charter.clone(),
        command: candidates[0].argv.clone(),
        candidates,
        limits: resolution.limits,
        inputs: resolution.inputs.clone(),
        hands: resolution.hands.clone(),
    })
}

/// A panel member or sequence step has no `limits` or `inputs` of its own,
/// so an agent it references may declare neither.
fn refuse_member_declarations(
    what: &str,
    name: &str,
    site: SiteRole,
    resolution: &crate::agents::Resolution,
) -> Result<(), CompileError> {
    if site != SiteRole::Member {
        return Ok(());
    }
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
    Ok(())
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
    expand: (&Executable, &Path),
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
    record_judged_tools(expand, what, local, lowered, sandboxed, sites);
    Ok(())
}

/// Record a site's judged local declaration and what it lowered to: the
/// half of [`record_inline_tools`] that refuses nothing. The generated
/// dialect validator declares no `tools`, so it records the unspecified
/// declaration here directly (operator ruling of 2026-09-30, unit 26c).
fn record_judged_tools(
    (engine, dir): (&Executable, &Path),
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
        argv: expand_command(engine, dir, &segment.argv),
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
        Some(declared) => {
            TemplateExpectation::Declared(expand_command(engine, dir, &declared.argv))
        }
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
    refuse_authored_sandbox(command, &refuse)?;
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
    let (part, fragment, door) = inline_codex_fragment(adapter, seat_class, &refuse)?;
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
    refuse_authored_control(&authored, authored_capability_control, "list", &refuse)?;
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

/// Operator ruling 1 of 2026-09-23: the first option of an inline site's
/// authored command that `control` names, beside the typed list or class
/// the engine composes, refuses the site; `typed` says which it is. The
/// one wording of [`lower_inline_allow`] and of an inline Codex sandbox's
/// reading (#288, slice I).
fn refuse_authored_control(
    authored: &brokkr_protocol::native_controls::grammar::Command,
    control: fn(&brokkr_protocol::native_controls::grammar::Node) -> Option<&'static str>,
    typed: &str,
    refuse: &impl Fn(String) -> CompileError,
) -> Result<(), CompileError> {
    if let Some((node, kind)) = authored
        .nodes
        .iter()
        .find_map(|node| control(node).map(|kind| (node, kind)))
    {
        return Err(refuse(format!(
            "while its authored command carries '{}' (argument {}), {kind}; the engine composes \
             the typed {typed} as its own contribution and a recipe authors no capability-bearing \
             option beside it, so the site is refused rather than reconciled (operator ruling 1 \
             of 2026-09-23; decision 0065 slice one, design D5.3)",
            node.name(),
            node.at + 1
        )));
    }
    Ok(())
}

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

/// The fragment the engine selects for one link on its serving path, named
/// for a refusal: `hands.workspace` under a box, the `hands.harness` member
/// the class selects under `harness`, an absent member read as empty. Under
/// `open` no engine fragment expresses a class, so there is none.
fn sandbox_fragment<'c>(
    boundary: Boundary,
    seat_class: SeatClass,
    adapter: &'c Adapter,
    candidate: &'c Candidate,
) -> Option<(&'static str, &'c [String])> {
    if boundary.is_boxed() {
        Some(("`hands.workspace` fragment", &candidate.hands_fragment))
    } else if boundary == Boundary::Harness {
        Some(match seat_class {
            SeatClass::Gate => (
                "`hands.harness.gate` fragment",
                adapter.harness.gate.as_deref().unwrap_or(&[]),
            ),
            SeatClass::Work => (
                "`hands.harness.work` fragment",
                adapter.harness.work.as_deref().unwrap_or(&[]),
            ),
        })
    } else {
        None
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
fn admit_local_sandbox(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    law: SiteLaw<'_>,
    agents: &mut Opened<'_>,
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
    let adapters = agents.adapters()?;
    let ask = SandboxAsk {
        what,
        requested,
        boundary,
        seat_class,
    };
    for (index, candidate) in candidates.iter().enumerate() {
        judge_sandbox_link(&ask, index + 1, candidate, adapters)?;
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
        // Compile has no run, so it hands no run's paths: every profile refuses (U9c2).
        grammar::judge_inline_codex_launch(lowered.class.intent(), &segments, capture, None)
            .map_err(|cause| {
                CompileError::Invalid(format!(
                    "seat {} declares 'tools.sandbox' '{}', but {} (operator ruling of \
                     2026-09-25; rebuild unit 5d-fix-b; design D5.3)",
                    bounded_site(what),
                    lowered.class.name(),
                    inline_codex_refusal(what, &cause)
                ))
            })?;
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
pub(crate) fn dispatch_driver(parts: &[String]) -> Option<String> {
    match Dispatch::of(parts) {
        Dispatch::Driver { kind, .. } => Some(kind.to_string()),
        Dispatch::Raw => None,
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
/// chain consulted. Returns the site's class, read where ruling 1 is.
fn enforce_model_policy(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    secrets: &[String],
    agents: &mut Opened<'_>,
    law: SiteLaw<'_>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<SeatClass, CompileError> {
    // The hands law is the FIRST statement (decision 0046 ruling 4;
    // design DD22): it judges every site that declares hands whatever
    // its class, so the work-class early return below cannot bypass it,
    // and it reads the adapters only on the chain path, where the
    // `agent` key opened them — a bundle whose only hands site is a
    // work-class exec seat compiles with no `adapters/` in sight.
    enforce_hands_boundary(what, raw, candidates, law, agents)?;
    let class = parse_class(what, raw)?;
    // A gate's tier and judges are read out of the adapters.
    if class == SeatClass::Gate {
        agents.adapters()?;
    }
    // The route is read off the model pin, so a site the pin walk refuses by
    // this `model_unpinned` over this argv has no route to judge here.
    if !model_unpinned(&command_parts(raw)) {
        enforce_route_policy(what, raw, candidates, secrets, agents, sites)?;
    }
    // Design D5.3, LAST: a typed sandbox class is admitted only where an
    // existing engine fragment expresses it exactly, after every standing
    // refusal above has had its say, so none of them loses precedence.
    admit_local_sandbox(what, raw, candidates, law, agents, sites)?;
    Ok(class)
}

/// Decision 0021's two prohibitions proper — the gate tier, the judges
/// list and the egress bar — at one site, after the hands law has spoken.
fn enforce_route_policy(
    what: &str,
    raw: &Value,
    candidates: &[Candidate],
    secrets: &[String],
    agents: &mut Opened<'_>,
    sites: &mut BTreeMap<String, SiteFacts>,
) -> Result<(), CompileError> {
    let class = tier::admitted(what, raw, candidates, agents)?;
    if class == SeatClass::Work && secrets.is_empty() {
        return Ok(());
    }
    // Read the adapter data without writing beside it: the inline
    // witness now lands in the one canonical site table (design D10 F1),
    // so no split borrow of the context is needed.
    let minimum = agents.egress_minimum;
    let adapters = agents.adapters()?;
    let mut authorised = Map::new();
    for (link, driver, abstract_model, pin) in route_links(raw, candidates, adapters) {
        let adapter = driver
            .as_deref()
            .and_then(|provider| adapters.adapter(provider));
        let named = match &driver {
            Some(provider) => format!("driver '{provider}'"),
            None => "an unnamed driver (the command is no driver dispatch)".to_string(),
        };
        let at = RouteLink {
            what,
            link,
            driver,
            adapter,
            abstract_model,
            pin,
            named,
        };
        judge_link(&at, class, raw, secrets, minimum)?;
        // Both prohibitions passed, so an adapter answered for this
        // driver. Only an inline site is recorded: `candidates` is empty
        // exactly there.
        if let (Some(provider), Some(adapter)) =
            (at.driver.filter(|_| candidates.is_empty()), at.adapter)
        {
            authorised.insert(provider, Value::String(adapter.digest.clone()));
        }
    }
    if !authorised.is_empty() {
        site_facts(sites, what).driver = Some(authorised);
    }
    Ok(())
}

/// Each link of a site as (link, driver, abstract model, concrete model
/// pin): the id is what carries the ROUTE (decision 0036 ruling 2), and
/// an agent chain's abstract name becomes concrete through the adapter
/// that maps it.
fn route_links(
    raw: &Value,
    candidates: &[Candidate],
    adapters: &Adapters,
) -> Vec<(usize, Option<String>, Option<String>, ModelPin)> {
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
        // driver, `enforce_route_policy`'s `(None, _)` arm lands the site
        // on `Uncontracted` whatever this returns.
        true => {
            let parts = command_parts(raw);
            let driver = dispatch_driver(&parts);
            let pin = inline_route_pin(
                &parts,
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
    }
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
    agents: &mut Opened<'_>,
) -> Result<(), CompileError> {
    let has_hands = law.agent_hands.is_some() || raw.get("hands").is_some();
    if !has_hands || law.boundary.is_boxed() {
        return Ok(());
    }
    let boundary = law.boundary;
    if candidates.is_empty() {
        return inline_hands_boundary(what, raw, law);
    }
    let adapters = agents.adapters()?;
    let class = parse_class(what, raw)?;
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

/// The hands law's inline arms, under `harness` and `open`: an exec site
/// names the bundle's own pinned script, and any other is refused.
fn inline_hands_boundary(what: &str, raw: &Value, law: SiteLaw<'_>) -> Result<(), CompileError> {
    let boundary = law.boundary;
    let parts = command_parts(raw);
    match dispatch_driver(&parts).as_deref() {
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
    }
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
    if let Some(skip) = crate::places::unpinned_reason(dir, Path::new(relative))
        .map_err(|refused| refused.to_string())?
    {
        let holds = match skip {
            crate::places::Skip::Workspace
            | crate::places::Skip::State
            | crate::places::Skip::Shelf => "workspace data, never bundle bytes",
            crate::places::Skip::GitMetadata => "git's own metadata, which no bundle pins",
            crate::places::Skip::GitIgnored => {
                "in a tree git ignores in the bundle's repository, whose bytes are not part of \
                 the bundle's identity"
            }
        };
        return Err(format!(
            "'{token}' names a path the manifest walk does not pin ({relative} is {holds})"
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
fn directory(path: &Path) -> std::io::Result<std::fs::File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NONBLOCK | OFlags::CLOEXEC;
    Ok(rustix::fs::open(path, flags, Mode::empty())?.into())
}

/// A handle on the directory `name` inside the directory `parent` holds,
/// never following a link it names.
fn directory_at(parent: &std::fs::File, name: &OsStr) -> std::io::Result<std::fs::File> {
    open_at(parent, name, rustix::fs::OFlags::DIRECTORY)
}

/// Open `name` inside the directory `parent` holds, non-blocking, never
/// following a link it names, with any further `flags`. Non-blocking: a FIFO
/// met on the way to an input is refused by the handle's kind, never waited
/// on. No-follow: each link on the way is seen, its text read, and followed
/// by this code.
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
    let refused =
        |error: crate::places::SkipError| Place::of(FaultKind::Unbound, error.to_string());
    if let Some(place) = unpinned_active_input(root, reference).map_err(refused)? {
        return Err(Place::of(FaultKind::Unbound, place));
    }
    let reference = PathBuf::from(reference);
    let observed = observe(root, open, &reference)?;
    let target = root.join(&observed.binding.target_key);
    if let Some(place) = skipped_path(root, &target).map_err(refused)? {
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
/// Ancestor and folder maps are the very maps their digests hash (0091).
/// Paths come from compile's canonical roots and expanded script token;
/// component comparisons never reinterpret a literal filename byte.
pub(crate) fn layer_drift(bundle: &Bundle, directory: &Path) -> Option<(String, String)> {
    let (layer, name, pinned) = compose::pinned_layer(bundle, directory)?;
    let current = match walk_files(layer, directory, &BTreeMap::new()) {
        Ok(files) => files,
        Err(error) => return Some((name.clone(), error.to_string())),
    };
    let relative = directory
        .strip_prefix(layer)
        .expect("directory under layer");
    let pinned_files = pinned.iter().filter(|(key, _)| {
        compose::reserved_prefix(key).is_none() && Path::new(key).starts_with(relative)
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

/// The capability walk's label for a sequence step: its `name`, or else its
/// one-based position (which the parse refuses), so it is the step's name.
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
/// source label — cannot overwrite a still-needed source. A survey's
/// per-site facts move by the same table (U9a2).
fn relocate_verify_facts<T>(sites: &mut BTreeMap<String, T>, moved: &[(String, String, SiteKey)]) {
    let staged: Vec<(String, T)> = moved
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
    census: &BTreeMap<String, SiteKey>,
    phase: &str,
    label: &str,
    second: &str,
) -> Result<(), CompileError> {
    match census.get(label) {
        Some(occupant) => Err(CompileError::Invalid(format!(
            "seat '{phase}' addresses two different sites as '{label}': {} and {second}. The \
             selection, the argv lookup, the hands map and the boundary map all key on that one \
             string, so one site would answer for the other; rename one of them",
            occupant.describe()
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

/// Parse a sequence body: named steps, each a single driver or a panel,
/// run serially inside one effect. At least two steps (a one-step
/// sequence is a single seat); names unique case-insensitively. Beside them,
/// `has_gate`: every step, a dialect one or an unsupplied check too, WROTE `gate`.
fn parse_sequence(
    compiler: &mut SiteCompiler<'_>,
    at: SeatScope<'_>,
    phase: &str,
    raw: &Value,
) -> Result<(Vec<SequenceStep>, GateAssessment), CompileError> {
    let steps_raw = sequence_steps(phase, raw)?;
    let mut steps: Vec<SequenceStep> = Vec::with_capacity(steps_raw.len());
    let mut whole_gate = GateAssessment::AllGate;
    for (index, step_raw) in steps_raw.iter().enumerate() {
        let name = step_name(phase, index, step_raw, &steps)?;
        let what = format!("{phase}:{name}");
        let shape = classify(&what, step_raw, &STEP)?;
        let step = StepAt {
            phase,
            name,
            what: &what,
            raw: step_raw,
        };
        let step_results = step_results(at, step, index + 1 == steps_raw.len())?;
        let written = (shape == StepShape::Dialect)
            .then(|| parse_class(&what, step_raw).unwrap_or(SeatClass::Work));
        let parsed = step_body(compiler, at, step, shape, step_results)?;
        if written == Some(SeatClass::Work) {
            whole_gate.first_work(WorkSite {
                path: vec![name.into()],
                compiled: parsed.is_some(),
            });
        }
        let Some(parsed) = parsed else { continue };
        let class = step_class(compiler, at, step, &parsed.body, parsed.hands)?;
        if written.is_none() {
            if let GateAssessment::ContainsWork(site) =
                gate::step_assessment(name, class, &parsed.body)
            {
                whole_gate.first_work(site);
            }
        }
        steps.push(SequenceStep {
            name: name.to_string(),
            class,
            results: parsed.results,
            body: parsed.body,
        });
    }
    Ok((steps, whole_gate))
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
    (engine, dir): (&Executable, &Path),
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
    Ok(expand_command(engine, dir, &parts))
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
fn walk_files(
    dir: &Path,
    scope: &Path,
    consumed: &BTreeMap<String, Supplied>,
) -> Result<BTreeMap<String, String>, CompileError> {
    let (paths, unpinned) = Walk::list(dir, scope, consumed)?;
    let walked = walk_keys(dir, paths)?;
    refuse_unlisted_consumed(consumed, &walked)?;
    refuse_oversized(dir, &walked)?;
    walk_digests(dir, walked, unpinned, consumed)
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
fn node(meta: &std::fs::Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    (meta.dev(), meta.ino())
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
