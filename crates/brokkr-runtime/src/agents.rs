//! The agent library and its provider adapters (decision 0016): one
//! definition per agent, one data file per provider, and one pure
//! function that turns them into a resolved seat body plus the record
//! that pins the resolution.
//!
//! **Purity is a property of this file, not of review discipline.**
//! `resolve` takes availability as an ARGUMENT, and nothing here reads
//! PATH, the filesystem, the clock or the network, or spawns anything.
//! The I/O that turns two directories into a `Library` and an `Adapters`
//! lives on the other side of a named boundary in `agents/load.rs`, so
//! "unit-tested without spawning anything" is guaranteed by the
//! signature rather than asserted afterwards. `Bundle::compile` passes
//! `Availability::unspecified()`: a compile that probed this machine
//! would give one bundle two digests and make an in-flight run
//! unresumable after an `apt install`.
//!
//! The honesty rules of decision 0016 are mechanised here, not
//! documented here:
//!
//! - A **restriction** the resolved provider cannot express (a tool
//!   permission narrowing) is always a hard failure: the agent would run
//!   with MORE power than it declares. `optional` is structurally
//!   unrepresentable on a restriction — `tools.allow` is a plain array,
//!   there is no key to set.
//! - A **grant** the provider cannot serve (an MCP server) is a hard
//!   failure too, unless the agent marked that server `optional`, in
//!   which case it becomes a notice that lands in the run manifest.
//!   Never nothing.
//! - Both checks run over **every** entry in the chain, not just the
//!   chosen one: a chain whose second link cannot express the agent's
//!   restrictions would silently widen its blast radius the moment it
//!   fell back.
//! - Matching is per NAMED item. "The provider supports MCP" does not
//!   satisfy "the agent needs the `github` server"; otherwise the agent
//!   runs, finds no tools, and reports a content failure for a
//!   configuration cause — the machine diagnosing itself wrong, which
//!   decision 0001 exists to prevent.

use std::collections::BTreeMap;
use std::path::PathBuf;

use brokkr_core::canonical::sha256_hex;
use serde_json::{json, Value};
use thiserror::Error;

use crate::bundle::Limits;

mod load;

pub use load::{resolve_route, Adapters, Library, LibraryError};

/// The grammar every agent, model, provider and MCP server name obeys.
/// Quoted verbatim in rejection messages so a reader can act on them.
pub const NAME_GRAMMAR: &str = "^[a-z][a-z0-9-]*$";

/// `true` when `name` matches [`NAME_GRAMMAR`].
pub fn valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_lowercase())
        && characters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// What this machine knows about a provider. Absent entries are
/// `Unknown`, which is the only arm compile ever sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    Available,
    Unavailable,
    Unknown,
}

/// Provider → presence. `unspecified()` carries no facts, so resolution
/// performs no availability filtering at all.
#[derive(Debug, Clone)]
pub struct Availability(BTreeMap<String, Presence>);

impl Availability {
    /// What `Bundle::compile` passes: no facts about any machine.
    pub fn unspecified() -> Availability {
        Availability(BTreeMap::new())
    }

    /// Record one probed fact. `brokkr doctor` is the real consumer of
    /// the non-`Unknown` arms.
    pub fn record(&mut self, provider: &str, presence: Presence) {
        self.0.insert(provider.to_string(), presence);
    }

    pub fn presence(&self, provider: &str) -> Presence {
        self.0.get(provider).copied().unwrap_or(Presence::Unknown)
    }
}

/// One MCP server an agent needs. `optional` exists ONLY here — which is
/// what makes "optional on a restriction" unrepresentable rather than
/// merely forbidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpNeed {
    pub server: String,
    pub optional: bool,
}

/// One agent definition, as written plus the digests that pin it.
#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub description: String,
    /// Absolute, canonicalised, proven contained within the library root.
    pub charter: PathBuf,
    pub charter_digest: String,
    /// Ordered preference chain of abstract model names.
    pub models: Vec<String>,
    /// The effort hired with each candidate (decision 0035 ruling 5),
    /// keyed by the candidate's abstract model name. A candidate served
    /// by an effortless provider needs no entry; every other candidate
    /// without one is refused at resolution, where the provider — and
    /// therefore the vocabulary — is known.
    pub efforts: BTreeMap<String, String>,
    /// `None` declares NO tool restriction; `Some` is ordered, and that
    /// order is the provider flag's order.
    pub allow: Option<Vec<String>>,
    pub mcp: Vec<McpNeed>,
    /// Decision 0043: the agent's hands are one boxed tool. When set, the
    /// tool allow-list is not consulted — the box bounds what running
    /// anything can touch — and the adapter must say how it replaces the
    /// harness's own tools with that one.
    pub hands: Option<brokkr_protocol::hands::HandsSpec>,
    pub limits: Option<Limits>,
    pub inputs: Option<Vec<String>>,
    /// sha256 over the canonical definition JSON.
    pub digest: String,
    /// The definition as written, for `brokkr agents show`.
    pub source: Value,
}

/// How a provider expresses a tool-permission narrowing on its command
/// line. Absent from the adapter as the explicit string `"unsupported"`,
/// never inferred from an empty map.
#[derive(Debug, Clone)]
pub struct ToolPermissions {
    pub flag: String,
    pub separator: String,
    pub names: BTreeMap<String, String>,
}

/// How a provider names MCP servers on its command line.
#[derive(Debug, Clone)]
pub struct McpSupport {
    pub flag: String,
    pub servers: BTreeMap<String, String>,
}

/// An operator-granted trust tier (decision 0021 ruling 2). A closed
/// vocabulary rather than a boolean, because ruling 3 makes tiers
/// earnable in BOTH directions and a third tier must not need a breaking
/// rename. No vendor sits in an arm here: a tier is data an operator
/// rules into an adapter file, cited to the scorecard, and an ABSENT
/// declaration is `Untrusted` — the `at_most` lesson, applied to trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustTier {
    Untrusted,
    Trusted,
}

impl TrustTier {
    pub fn parse(name: &str) -> Option<TrustTier> {
        match name {
            "trusted" => Some(TrustTier::Trusted),
            "untrusted" => Some(TrustTier::Untrusted),
            _ => None,
        }
    }
}

/// Where a route's endpoint stands (decision 0036 ruling 1). A closed,
/// ORDERED vocabulary: the order is how much of the material's journey
/// the operator owns, so "meets a minimum" is a comparison rather than a
/// table of pairs. `Local` means the endpoint runs on hardware the
/// operator controls and the serialized material crosses no network
/// boundary they do not own; `Contracted` means a third party they have
/// ruled acceptable in a recorded ruling; `Uncontracted` is everything
/// else, and is the value of an absent declaration. No vendor and no
/// route name sits in an arm here: the vocabulary is the engine's, and
/// the assignment of a route to a class is operator data, exactly as
/// ruling 2 of decision 0021 holds for tiers.
///
/// Ordering is NOT a judging axis (ruling 3): `Local` outranks
/// `Contracted` for what may be SENT to it and confers nothing at all on
/// what may be believed from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EgressClass {
    Uncontracted,
    Contracted,
    Local,
}

impl EgressClass {
    pub fn parse(name: &str) -> Option<EgressClass> {
        match name {
            "local" => Some(EgressClass::Local),
            "contracted" => Some(EgressClass::Contracted),
            "uncontracted" => Some(EgressClass::Uncontracted),
            _ => None,
        }
    }

    /// The word the operator writes, quoted back in every refusal so a
    /// reader can act on the message without opening this file.
    pub fn name(&self) -> &'static str {
        match self {
            EgressClass::Local => "local",
            EgressClass::Contracted => "contracted",
            EgressClass::Uncontracted => "uncontracted",
        }
    }

    /// The vocabulary, in message order — one source for every refusal
    /// that has to spell the closed set out.
    pub const VOCABULARY: &'static str = "\"local\", \"contracted\" or \"uncontracted\"";
}

/// One provider adapter: data, never a Rust match arm. Adding a provider
/// or a model is a file, not a release.
#[derive(Debug, Clone)]
pub struct Adapter {
    pub provider: String,
    /// Decision 0021 ruling 2: what this driver is trusted to be. Gate
    /// seats require `Trusted`; absence is `Untrusted`.
    pub trust_tier: TrustTier,
    /// Decision 0036 ruling 2: the class of this adapter's OWN
    /// destination, and what an unprefixed model id resolves to. It
    /// answers for that destination ONLY — a route this file does not
    /// name falls to `Uncontracted`, not to this value, because ruling 1
    /// makes an absent declaration uncontracted (see `resolve_route`).
    /// It is also where the superseded `binding_grant` lands: a `true`
    /// grant reads as `Contracted`, a `false` or absent grant as
    /// `Uncontracted`.
    pub egress: EgressClass,
    /// Decision 0036 ruling 2: route name → declared class, where a
    /// route is the prefix of a concrete model id. An adapter fronting a
    /// single destination declares one class at the adapter and no
    /// routes.
    pub routes: BTreeMap<String, EgressClass>,
    /// Decision 0036 ruling 5: route name → the credential variable that
    /// route needs. Declared so `brokkr doctor` can say when the value
    /// comes from the process environment rather than the bindings
    /// store; the engine never reads the value, here or anywhere.
    pub credentials: BTreeMap<String, String>,
    /// The binary `brokkr doctor` probes for on this machine.
    pub binary: String,
    /// Optional operator-written advice `brokkr doctor` prints when the
    /// binary is absent — where it installs from, which env override
    /// points at it. Advice belongs in the data beside the binary name,
    /// not in a Rust constant that needs a release to correct.
    pub hint: Option<String>,
    /// The driver invocation prefix, `{brokkr}` left unexpanded.
    pub driver: Vec<String>,
    /// Abstract model name → concrete provider model id.
    pub models: BTreeMap<String, String>,
    pub model_flag: Option<String>,
    /// The effort levels this provider's harness names, as measured
    /// against its CLI. Empty for a provider with no effort control.
    pub efforts: Vec<String>,
    /// How this provider is TOLD an effort, or `None` where it declares
    /// `effort_flag` unsupported — the same shape, and the same meaning,
    /// as `model_flag`.
    pub effort_flag: Option<String>,
    pub tool_permissions: Option<ToolPermissions>,
    /// Decision 0043: how this provider is told to put its hands in the
    /// box — the argv fragment that disables its own tools and reaches
    /// the `brokkr hands serve` MCP server, with `{hands_mcp_json}` and
    /// `{hands_args_toml}` expanded by the engine at spawn. `None` where
    /// the provider declares `hands` unsupported; the reason, if measured,
    /// is `hands_gap`.
    pub hands: Option<Vec<String>>,
    pub hands_gap: Option<String>,
    /// Why `tool_permissions` is absent, when the operator MEASURED the
    /// provider's CLI and found no per-tool allow-list to map onto.
    /// Never a capability: a declared gap refuses exactly as a bare
    /// `"unsupported"` does. It exists so the refusal can name the
    /// provider's real restriction axis (codex's sandbox classes) rather
    /// than leaving a reader to wonder whether anyone ever looked.
    pub tool_permissions_gap: Option<String>,
    pub mcp: Option<McpSupport>,
    pub digest: String,
}

/// One resolved invocation: the agent it serves, a model, the provider
/// serving it, and the composed argv with `{brokkr}` still a literal
/// token. The agent name rides on every candidate because per-attempt
/// provenance names all three, and a site's chain is walked one link at
/// a time — carrying the name here is what lets the journal say WHICH
/// agent fell back without re-opening the library at run time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub agent: String,
    pub model: String,
    /// The effort hired with this model, `None` only where the provider
    /// declares no effort control at all. It is the PIN — what the plan
    /// asked for — and never what a harness reports it applied; the two
    /// stay separate facts (decision 0035 ruling 6).
    pub effort: Option<String>,
    pub provider: String,
    pub argv: Vec<String>,
}

/// An optional-capability gap: a WARNING that lands in the run manifest.
/// A warning that only reaches stderr is "nothing" by the ruling's own
/// words, so this is a value, not a print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub agent: String,
    pub provider: String,
    pub model: String,
    pub capability: String,
    pub item: String,
    pub message: String,
}

impl Notice {
    pub fn value(&self) -> Value {
        json!({
            "agent": self.agent,
            "provider": self.provider,
            "model": self.model,
            "capability": self.capability,
            "item": self.item,
            "message": self.message,
        })
    }
}

/// Every way resolution refuses. Each variant carries the names its
/// message must print — the message IS the contract (AC-2 asserts on
/// content, not on kind).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ResolveError {
    #[error("agent '{name}' is not in the library; known agents: {known}")]
    UnknownAgent { name: String, known: String },
    #[error(
        "agent '{agent}' names model '{model}', which no adapter maps; \
         adapters consulted: {files}"
    )]
    UnmappedModel {
        agent: String,
        model: String,
        files: String,
    },
    #[error(
        "agent '{agent}' cannot be served by provider '{provider}' on model \
         '{model}': {capability}. A capability the provider cannot express \
         fails compilation here rather than degrading silently at run time"
    )]
    Capability {
        agent: String,
        provider: String,
        model: String,
        capability: String,
    },
    #[error(
        "agent '{agent}' has no available candidate: every model in the chain \
         [{chain}] resolves to a provider this machine reports as unavailable"
    )]
    NoneAvailable { agent: String, chain: String },
}

/// One chain entry as the resolver sees it: the shared derivation behind
/// `resolve` (which the compiler calls) and `report` (which
/// `brokkr agents show` and `brokkr doctor` call). They are the same walk
/// — `resolve` is `report` plus the refusals.
#[derive(Debug, Clone)]
pub struct ChainEntry {
    pub model: String,
    /// The effort pinned with this candidate, `None` when the entry is
    /// unmapped, blocked, or served by a provider with no effort control.
    pub effort: Option<String>,
    /// `None` when no adapter maps this name.
    pub provider: Option<String>,
    pub presence: Presence,
    /// Empty when the entry is unmapped or blocked by a capability gap.
    pub argv: Vec<String>,
    pub gap: Option<ResolveError>,
    pub notices: Vec<Notice>,
}

/// The per-entry resolution picture, which never fails for a known agent
/// — an unmapped or blocked entry is reported, not thrown, so a readout
/// can print the whole chain.
#[derive(Debug, Clone)]
pub struct Report {
    pub agent: Agent,
    pub entries: Vec<ChainEntry>,
    /// The entry `resolve` would choose: the first mapped entry this
    /// machine does not report as unavailable.
    pub chosen: Option<usize>,
}

/// A resolved agent reference, ready to become an ordinary seat body.
#[derive(Debug, Clone)]
pub struct Resolution {
    pub agent: String,
    pub charter: PathBuf,
    pub limits: Option<Limits>,
    pub inputs: Option<Vec<String>>,
    /// Ordered; `[0]` is the choice and the rest are the bounded
    /// fallback order.
    pub candidates: Vec<Candidate>,
    /// The manifest record (spec Q1): names and digests only, never
    /// resolved argv — `{brokkr}` expands to a machine-local absolute
    /// path, and pinning it would make `manifest_digest()` machine
    /// dependent.
    pub record: Value,
    pub notices: Vec<Notice>,
    /// Decision 0043: the agent's hands, carried to the site that hires it.
    pub hands: Option<brokkr_protocol::hands::HandsSpec>,
}

fn capability_gap(
    agent: &Agent,
    adapter: &Adapter,
    model: &str,
    capability: String,
) -> ResolveError {
    ResolveError::Capability {
        agent: agent.name.clone(),
        provider: adapter.provider.clone(),
        model: model.to_string(),
        capability,
    }
}

/// Compose one candidate's argv, or refuse. A lookup and a join: there
/// is no template language, so there is no substitution function whose
/// branches could drift from the data.
fn compose(
    agent: &Agent,
    adapter: &Adapter,
    model: &str,
    concrete: &str,
    notices: &mut Vec<Notice>,
) -> Result<(Vec<String>, Option<String>), ResolveError> {
    let mut argv = adapter.driver.clone();
    // A provider that serves the model but cannot be TOLD which model is
    // the silent-substitution case in its purest form: it would run its
    // own default and the run would claim the pinned one.
    let flag = adapter.model_flag.as_ref().ok_or_else(|| {
        capability_gap(
            agent,
            adapter,
            model,
            "the provider declares model_flag unsupported, so the chosen model \
             could not be pinned and the provider's own default would run"
                .to_string(),
        )
    })?;
    argv.push(flag.clone());
    argv.push(concrete.to_string());

    // The other half of the hire (decision 0035 ruling 5). A model pin
    // without an effort pin is half a hire, and the half it withholds is
    // the half that moves the bill — so an effort-bearing provider that
    // this candidate names no effort for is refused here, exactly as a
    // provider that cannot be told its model is.
    let effort = match (&adapter.effort_flag, agent.efforts.get(model)) {
        (None, None) => None,
        (None, Some(effort)) => {
            return Err(capability_gap(
                agent,
                adapter,
                model,
                format!(
                    "the provider declares effort_flag unsupported, so the agent's \
                     effort '{effort}' could not be pinned and the provider's own \
                     default would run"
                ),
            ))
        }
        (Some(_), None) => {
            return Err(capability_gap(
                agent,
                adapter,
                model,
                format!(
                    "the provider takes an effort and this candidate pins none; add \
                     \"efforts\": {{\"{model}\": \"<one of: {}>\"}} to the agent \
                     (decision 0035 ruling 5)",
                    adapter.efforts.join(", ")
                ),
            ))
        }
        (Some(effort_flag), Some(effort)) => {
            if !adapter.efforts.iter().any(|known| known == effort) {
                return Err(capability_gap(
                    agent,
                    adapter,
                    model,
                    format!(
                        "the provider declares no effort '{effort}'; its vocabulary \
                         is {}",
                        adapter.efforts.join(", ")
                    ),
                ));
            }
            argv.push(effort_flag.clone());
            argv.push(effort.clone());
            Some(effort.clone())
        }
    };

    if agent.hands.is_some() {
        // Decision 0043 ruling 2: the box expresses the restriction. The
        // tool list is not consulted; what the provider must be able to
        // say is how its own tools are replaced by the one boxed tool.
        let fragment = adapter.hands.as_ref().ok_or_else(|| {
            let declared = match &adapter.hands_gap {
                Some(reason) => format!("the provider declares hands unsupported ({reason})"),
                None => "the provider declares hands unsupported".to_string(),
            };
            capability_gap(
                agent,
                adapter,
                model,
                format!(
                    "{declared}, so the agent's hands cannot be put in the box and \
                     the agent would run with the harness's own tools"
                ),
            )
        })?;
        argv.extend(fragment.iter().cloned());
    } else if let Some(allow) = &agent.allow {
        let permissions = adapter.tool_permissions.as_ref().ok_or_else(|| {
            // A measured gap names the axis the provider DOES have; a
            // bare `"unsupported"` names nothing, because nothing was
            // recorded. Either way the attempt refuses here.
            let declared = match &adapter.tool_permissions_gap {
                Some(reason) => {
                    format!("the provider declares tool_permissions unsupported ({reason})")
                }
                None => "the provider declares tool_permissions unsupported".to_string(),
            };
            capability_gap(
                agent,
                adapter,
                model,
                format!(
                    "{declared}, so the agent's restriction to {allow:?} cannot be \
                     expressed and the agent would run with MORE power than it declares"
                ),
            )
        })?;
        let mut expressed = Vec::with_capacity(allow.len());
        for tool in allow {
            let name = permissions.names.get(tool).ok_or_else(|| {
                capability_gap(
                    agent,
                    adapter,
                    model,
                    format!("the provider maps no tool permission named '{tool}'"),
                )
            })?;
            expressed.push(name.clone());
        }
        argv.push(permissions.flag.clone());
        argv.push(expressed.join(&permissions.separator));
    }

    for need in &agent.mcp {
        let served = adapter
            .mcp
            .as_ref()
            .and_then(|mcp| mcp.servers.get(&need.server).map(|value| (mcp, value)));
        match served {
            Some((mcp, value)) => {
                argv.push(mcp.flag.clone());
                argv.push(value.clone());
            }
            None => {
                let capability = match adapter.mcp.as_ref() {
                    Some(_) => format!(
                        "the provider declares no MCP server named '{}'",
                        need.server
                    ),
                    None => format!(
                        "the provider declares mcp unsupported, so the MCP server \
                         '{}' cannot be provided",
                        need.server
                    ),
                };
                if !need.optional {
                    return Err(capability_gap(agent, adapter, model, capability));
                }
                notices.push(Notice {
                    agent: agent.name.clone(),
                    provider: adapter.provider.clone(),
                    model: model.to_string(),
                    capability: "mcp".to_string(),
                    item: need.server.clone(),
                    message: format!(
                        "optional capability gap: {capability}; the agent runs with \
                         less power than it declares"
                    ),
                });
            }
        }
    }
    Ok((argv, effort))
}

fn entry_for(
    agent: &Agent,
    adapters: &Adapters,
    availability: &Availability,
    model: &str,
) -> ChainEntry {
    let Some((adapter, concrete)) = adapters.serving(model) else {
        return ChainEntry {
            model: model.to_string(),
            effort: None,
            provider: None,
            presence: Presence::Unknown,
            argv: Vec::new(),
            gap: None,
            notices: Vec::new(),
        };
    };
    let presence = availability.presence(&adapter.provider);
    let mut notices = Vec::new();
    let (argv, effort, gap) = match compose(agent, adapter, model, concrete, &mut notices) {
        Ok((argv, effort)) => (argv, effort, None),
        // A blocked entry contributes no notices: it contributes an
        // error, and reporting both would double-count one gap.
        Err(gap) => (Vec::new(), None, Some(gap)),
    };
    ChainEntry {
        model: model.to_string(),
        effort,
        provider: Some(adapter.provider.clone()),
        presence,
        argv,
        gap,
        notices,
    }
}

/// Walk an agent's whole chain without refusing anything but an unknown
/// name: the derivation `brokkr agents show` and `brokkr doctor` print.
pub fn report(
    library: &Library,
    adapters: &Adapters,
    availability: &Availability,
    name: &str,
) -> Result<Report, ResolveError> {
    let agent = library
        .agent(name)
        .ok_or_else(|| ResolveError::UnknownAgent {
            name: name.to_string(),
            known: library.names().join(", "),
        })?
        .clone();
    let entries: Vec<ChainEntry> = agent
        .models
        .iter()
        .map(|model| entry_for(&agent, adapters, availability, model))
        .collect();
    let chosen = entries
        .iter()
        .position(|entry| entry.provider.is_some() && entry.presence != Presence::Unavailable);
    Ok(Report {
        agent,
        entries,
        chosen,
    })
}

/// The compiler's function: `report`, plus every refusal decision 0016
/// rules. Pure — same inputs, byte-identical output, on any machine.
pub fn resolve(
    library: &Library,
    adapters: &Adapters,
    availability: &Availability,
    name: &str,
) -> Result<Resolution, ResolveError> {
    let report = report(library, adapters, availability, name)?;
    let agent = &report.agent;
    // Every entry, not just the chosen one.
    for entry in &report.entries {
        if let Some(gap) = &entry.gap {
            return Err(gap.clone());
        }
        if entry.provider.is_none() {
            return Err(ResolveError::UnmappedModel {
                agent: agent.name.clone(),
                model: entry.model.clone(),
                files: adapters.files().join(", "),
            });
        }
    }
    let chosen = report.chosen.ok_or_else(|| ResolveError::NoneAvailable {
        agent: agent.name.clone(),
        chain: agent.models.join(", "),
    })?;

    let candidates: Vec<Candidate> = report.entries[chosen..]
        .iter()
        .filter(|entry| entry.presence != Presence::Unavailable)
        .map(|entry| Candidate {
            agent: agent.name.clone(),
            model: entry.model.clone(),
            effort: entry.effort.clone(),
            provider: entry.provider.clone().expect("mapped above"),
            argv: entry.argv.clone(),
        })
        .collect();
    let skipped: Vec<Value> = report.entries[..chosen]
        .iter()
        .map(|entry| json!({"model": entry.model, "reason": "unavailable"}))
        .collect();
    let notices: Vec<Notice> = report
        .entries
        .iter()
        .flat_map(|entry| entry.notices.iter().cloned())
        .collect();
    // Every adapter the chain consulted, so an edit to a FALLBACK
    // provider's file moves the pin too: the capability checks that
    // admitted this chain depended on all of them.
    let mut consulted = BTreeMap::new();
    for entry in &report.entries {
        let provider = entry.provider.clone().expect("mapped above");
        let digest = adapters
            .digest(&provider)
            .expect("mapped above")
            .to_string();
        consulted.insert(provider, digest);
    }
    let record = json!({
        "agent": agent.name,
        "agent_digest": agent.digest,
        "charter_digest": agent.charter_digest,
        "adapter_digest": sha256_hex(&json!(consulted)),
        "chain": agent.models,
        "chosen_index": chosen,
        "model": candidates[0].model,
        "provider": candidates[0].provider,
        "skipped": skipped,
        "notices": notices.iter().map(Notice::value).collect::<Vec<_>>(),
    });
    Ok(Resolution {
        hands: agent.hands.clone(),
        agent: agent.name.clone(),
        charter: agent.charter.clone(),
        limits: agent.limits,
        inputs: agent.inputs.clone(),
        candidates,
        record,
        notices,
    })
}

#[cfg(test)]
mod tests;
