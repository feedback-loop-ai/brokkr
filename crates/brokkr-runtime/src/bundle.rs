//! Declarative bundles: policy + one seat per phase (decision 0005).
//! `compile` validates before anything runs and produces the pinned
//! content-addressed manifest. Rejections here are the executable slice
//! of the constitutional lint: a bundle that could reach ship around the
//! protected review phase, name a result no rule covers, or reference a
//! missing role never loads at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::policy::{Machine, BOOLEAN_INPUTS, SEVERITY_INPUTS};
use serde_json::{json, Map, Value};
use thiserror::Error;

pub mod compose;

use compose::{Ancestor, COMPOSE_PREFIX};

use crate::agents::{Adapters, Availability, Candidate, Library, TrustTier};

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
/// journal-computed truth is never accepted from a caller (determinism
/// law 2, `docs/guides/overview.md`).
pub const ENGINE_OWNED_INPUTS: [&str; 5] = [
    "consecutive_failures",
    "drift_detected",
    "dirty_worktrees",
    "reviewed_heads",
    // The same repository facts, keyed by realm (decision 0023). Read
    // from the tree by the engine, exactly like the two above it.
    REALM_FACTS,
];

/// The per-realm repository facts a decision records in a mapped world:
/// realm name -> observed HEAD, dirty worktree, drift.
pub const REALM_FACTS: &str = "realm_facts";

/// The same law over the phase-visit family (decision 0022): every
/// `visits_<phase>` is counted by the fold from `phase/entered` events,
/// so no seat may declare one and no seat may claim one.
pub fn is_engine_owned(name: &str) -> bool {
    ENGINE_OWNED_INPUTS.contains(&name) || name.starts_with(brokkr_core::policy::VISIT_PREFIX)
}

#[derive(Debug, Clone)]
pub struct Seat {
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
}

#[derive(Debug, Clone)]
pub struct SequenceStep {
    pub name: String,
    pub body: StepBody,
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
    /// The phase every path to a non-stop terminal must traverse.
    pub protected_phase: String,
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
}

/// One resolved agent reference, ready to become an ordinary seat body.
struct ResolvedSeat {
    role_path: PathBuf,
    command: Vec<String>,
    candidates: Vec<Candidate>,
    limits: Option<Limits>,
    inputs: Option<Vec<String>>,
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
                || map.contains_key("secrets")
                || map.get("class").and_then(Value::as_str) == Some("gate")
                || map.values().any(needs_adapters)
        }
        Value::Array(items) => items.iter().any(needs_adapters),
        _ => false,
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

    pub fn compile_with(
        dir: &Path,
        library_root: &Path,
        adapters_root: &Path,
    ) -> Result<Bundle, CompileError> {
        let dir = dir
            .canonicalize()
            .map_err(|e| CompileError::Invalid(format!("bundle dir {}: {e}", dir.display())))?;
        // Composition resolves FIRST, into one flat bundle; everything
        // below this line compiles a single bundle and never learns that
        // composition happened (decision 0017).
        let resolved = compose::resolve(&dir)?;
        let note = resolved.chain_note();
        match Bundle::assemble(&dir, resolved, library_root, adapters_root) {
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
    ) -> Result<Bundle, CompileError> {
        let config = &resolved.document;
        let name = resolved.name.clone();
        let table = resolved.table.clone();
        let machine = Machine::from_table(&table)?;

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
        let mut agents = match resolved.seats.values().any(needs_adapters) {
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
            }),
        };

        let mut seats = BTreeMap::new();
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
                if !covered {
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
            if [has_single, has_panel, has_sequence, has_agent]
                .iter()
                .filter(|f| **f)
                .count()
                > 1
            {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' must be exactly one of role+driver, agent, \
                     panel, or sequence"
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
            let body = if let Some(agent_seat) = &agent_seat {
                SeatBody::Single {
                    role_path: agent_seat.role_path.clone(),
                    command: agent_seat.command.clone(),
                    confine: parse_confine(phase, raw)?,
                    candidates: agent_seat.candidates.clone(),
                }
            } else if has_panel {
                let (members, aggregate) =
                    parse_panel(dir, phase, raw, Some(&results), &secrets, &mut agents)?;
                SeatBody::Panel { members, aggregate }
            } else if has_sequence {
                SeatBody::Sequence {
                    steps: parse_sequence(dir, phase, raw, &results, &secrets, &mut agents)?,
                }
            } else {
                SeatBody::Single {
                    role_path: parse_role(dir, phase, raw)?,
                    command: parse_command(dir, phase, raw, &secrets)?,
                    confine: parse_confine(phase, raw)?,
                    candidates: Vec::new(),
                }
            };
            // Decision 0021, at the seat's own driver-bearing site. A
            // panel or a sequence has none: its members and steps were
            // each checked where they were built.
            match &body {
                SeatBody::Single { candidates, .. } => {
                    enforce_model_policy(phase, raw, candidates, &secrets, &mut agents)?
                }
                _ => refuse_class_without_a_driver(phase, raw)?,
            }
            let limits = match agent_seat.as_ref().and_then(|agent_seat| agent_seat.limits) {
                // An agent's 0006 bounds ARE the seat's bounds: `limits`
                // is forbidden beside `agent:`, so there is nothing to
                // reconcile and nothing silently overridden.
                Some(limits) => limits,
                None => parse_limits(phase, raw)?,
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

        let manifest = manifest_for(
            dir,
            &name,
            &resolved.chain,
            agents.as_ref().map(|a| &a.records),
            agents.as_ref().map(|a| &a.drivers),
        )?;
        Ok(Bundle {
            name,
            dir: dir.to_path_buf(),
            roots: resolved.roots,
            chain: resolved.chain,
            machine,
            seats,
            manifest,
            protected_phase,
        })
    }

    pub fn manifest_digest(&self) -> String {
        brokkr_core::canonical::sha256_hex(&self.manifest)
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
/// claims to show them. `class` in particular is the seat's authority,
/// never the agent's: one charter may sit in a work seat here and a gate
/// seat there, which is why decision 0021 ruling 1 puts the division in
/// bundle data.
fn refuse_amendments(what: &str, raw: &Value) -> Result<(), CompileError> {
    let refuse = |key: &str| {
        Err(CompileError::Invalid(format!(
            "seat '{what}' combines 'agent' with '{key}'; an agent reference is \
             total — '{key}' states what the agent IS, and a seat that could \
             amend it would make `brokkr agents show` a lie for that seat"
        )))
    };
    for key in ["role", "limits", "inputs"] {
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
    "panel",
    "aggregate",
    "sequence",
];

/// The keys a PANEL MEMBER may write. Narrower than a seat's: a member
/// has no `results`, `limits`, `inputs` or `secrets` of its own — the
/// seat above it does — which is why an agent declaring them at a member
/// site is already refused rather than silently discarded.
const MEMBER_KEYS: &[&str] = &["class", "agent", "role", "driver"];

/// The keys a SEQUENCE STEP may write: a member's, plus its name, plus
/// the two a step needs to be a panel of its own.
const STEP_KEYS: &[&str] = &[
    "name",
    "class",
    "agent",
    "role",
    "driver",
    "panel",
    "aggregate",
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
/// its own: `recipes/sdd`'s `design` seat is a panel of work positions,
/// a gate chief and a work check, and a single word on the seat could
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

/// Decision 0021's two compile-time prohibitions, at one driver-bearing
/// site. Both are a lookup and a comparison — deterministic code with an
/// exit status (decision 0025 ruling 6), refusing before any prompt
/// exists to leak, in the manner of a digest mismatch:
///
/// - a GATE-class site whose driver lacks the trusted tier (ruling 2);
/// - a site under a seat that declares secret bindings, whose driver
///   holds no binding grant (ruling 4).
///
/// Both fail closed on ABSENCE — an undeclared tier is untrusted, an
/// undeclared grant is none, and a driver no adapter declares has
/// neither. `candidates` is the resolved fallback chain of an agent site
/// and empty for an inline one; EVERY link is checked, because ruling 5
/// says an unavailable driver parks rather than substitutes, and a chain
/// that could fall back to an untrusted judge at run time would have
/// defeated the gate at compile time.
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
        ..
    } = agents
        .as_mut()
        .expect("a gate-class or secret-binding seat opens the adapters");
    let mut authorised = Map::new();
    let drivers: Vec<Option<String>> = match candidates.is_empty() {
        true => vec![dispatch_driver(&command_parts(raw))],
        false => candidates
            .iter()
            .map(|candidate| Some(candidate.provider.clone()))
            .collect(),
    };
    for driver in drivers {
        let adapter = driver
            .as_deref()
            .and_then(|provider| adapters.adapter(provider));
        let named = match &driver {
            Some(provider) => format!("driver '{provider}'"),
            None => "an unnamed driver (the command is no driver dispatch)".to_string(),
        };
        if class == SeatClass::Gate && adapter.map(|a| a.trust_tier) != Some(TrustTier::Trusted) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' is gate class but seats {named}, which does not \
                 hold the trusted tier; a gate seat IS the check, and nobody \
                 stands behind the judges (decision 0021 ruling 2 — an \
                 undeclared tier is untrusted)"
            )));
        }
        if !secrets.is_empty() && adapter.map(|a| a.binding_grant) != Some(true) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' declares secret bindings {secrets:?} but seats \
                 {named}, which holds no binding grant; trust to judge and \
                 clearance to receive are different grants (decision 0021 \
                 ruling 4 — an undeclared grant is none)"
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

fn parse_panel(
    dir: &Path,
    what: &str,
    raw: &Value,
    declared_results: Option<&[String]>,
    secrets: &[String],
    agents: &mut Option<AgentContext>,
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
    if let Some(results) = declared_results {
        for required in aggregate.required_results() {
            if !results.iter().any(|r| r == required) {
                return Err(CompileError::Invalid(format!(
                    "seat '{what}' aggregate '{aggregate_name}' can emit \
                     '{required}' but the seat does not declare it"
                )));
            }
        }
    }
    let mut members = Vec::with_capacity(members_raw.len());
    for (name, member_raw) in members_raw {
        let site = format!("{what}:{name}");
        if member_raw.get("agent").is_some() {
            refuse_amendments(&site, member_raw)?;
        }
        refuse_unknown_keys(&site, member_raw, MEMBER_KEYS)?;
        let (role_path, command, candidates) = match member_raw.get("agent") {
            None => (
                parse_role(dir, &site, member_raw)?,
                parse_command(dir, &site, member_raw, secrets)?,
                Vec::new(),
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
                (resolved.role_path, resolved.command, resolved.candidates)
            }
        };
        enforce_model_policy(&site, member_raw, &candidates, secrets, agents)?;
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
    results: &[String],
    secrets: &[String],
    agents: &mut Option<AgentContext>,
) -> Result<Vec<SequenceStep>, CompileError> {
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
        if [has_single, has_panel, has_agent]
            .iter()
            .filter(|f| **f)
            .count()
            > 1
        {
            return Err(CompileError::Invalid(format!(
                "sequence step '{what}' must be exactly one of role+driver, agent, \
                 or panel"
            )));
        }
        refuse_unknown_keys(&what, step_raw, STEP_KEYS)?;
        let body = if has_agent {
            let resolved =
                resolve_reference(agents, dir, &what, &what, step_raw, secrets, Site::Member)?;
            StepBody::Single {
                role_path: resolved.role_path,
                command: resolved.command,
                confine: parse_confine(&what, step_raw)?,
                candidates: resolved.candidates,
            }
        } else if has_panel {
            let final_step = index + 1 == steps_raw.len();
            let (members, aggregate) = parse_panel(
                dir,
                &what,
                step_raw,
                final_step.then_some(results),
                secrets,
                agents,
            )?;
            StepBody::Panel { members, aggregate }
        } else {
            StepBody::Single {
                role_path: parse_role(dir, &what, step_raw)?,
                command: parse_command(dir, &what, step_raw, secrets)?,
                confine: parse_confine(&what, step_raw)?,
                candidates: Vec::new(),
            }
        };
        match &body {
            StepBody::Single { candidates, .. } => {
                enforce_model_policy(&what, step_raw, candidates, secrets, agents)?
            }
            StepBody::Panel { .. } => refuse_class_without_a_driver(&what, step_raw)?,
        }
        steps.push(SequenceStep {
            name: name.to_string(),
            body,
        });
    }
    Ok(steps)
}

fn parse_role(dir: &Path, what: &str, raw: &Value) -> Result<PathBuf, CompileError> {
    let role_rel = raw
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| CompileError::Invalid(format!("seat '{what}' missing 'role'")))?;
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
    !is_engine_owned(name) && (BOOLEAN_INPUTS.contains(&name) || SEVERITY_INPUTS.contains(&name))
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
            let name = key
                .strip_suffix("_gte")
                .or_else(|| key.strip_suffix("_above"))
                .or_else(|| key.strip_suffix("_at_most"))
                .unwrap_or(key)
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
