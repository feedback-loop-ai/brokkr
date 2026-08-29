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
//!   configuration cause — the forge diagnosing itself wrong, which
//!   decision 0001 exists to prevent.

use std::collections::BTreeMap;
use std::path::PathBuf;

use forge_core::canonical::sha256_hex;
use serde_json::{json, Value};
use thiserror::Error;

use crate::bundle::Limits;

mod load;

pub use load::{Adapters, Library, LibraryError};

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

    /// Record one probed fact. `forge doctor` is the real consumer of
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
    /// `None` declares NO tool restriction; `Some` is ordered, and that
    /// order is the provider flag's order.
    pub allow: Option<Vec<String>>,
    pub mcp: Vec<McpNeed>,
    pub limits: Option<Limits>,
    pub inputs: Option<Vec<String>>,
    /// sha256 over the canonical definition JSON.
    pub digest: String,
    /// The definition as written, for `forge agents show`.
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

/// One provider adapter: data, never a Rust match arm. Adding a provider
/// or a model is a file, not a release.
#[derive(Debug, Clone)]
pub struct Adapter {
    pub provider: String,
    /// The binary `forge doctor` probes for on this machine.
    pub binary: String,
    /// The driver invocation prefix, `{forge}` left unexpanded.
    pub driver: Vec<String>,
    /// Abstract model name → concrete provider model id.
    pub models: BTreeMap<String, String>,
    pub model_flag: Option<String>,
    pub tool_permissions: Option<ToolPermissions>,
    pub mcp: Option<McpSupport>,
    pub digest: String,
}

/// One resolved invocation: a model, the provider serving it, and the
/// composed argv with `{forge}` still a literal token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub model: String,
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
/// `forge agents show` and `forge doctor` call). They are the same walk
/// — `resolve` is `report` plus the refusals.
#[derive(Debug, Clone)]
pub struct ChainEntry {
    pub model: String,
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
    /// resolved argv — `{forge}` expands to a machine-local absolute
    /// path, and pinning it would make `manifest_digest()` machine
    /// dependent.
    pub record: Value,
    pub notices: Vec<Notice>,
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
) -> Result<Vec<String>, ResolveError> {
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

    if let Some(allow) = &agent.allow {
        let permissions = adapter.tool_permissions.as_ref().ok_or_else(|| {
            capability_gap(
                agent,
                adapter,
                model,
                format!(
                    "the provider declares tool_permissions unsupported, so the \
                     agent's restriction to {allow:?} cannot be expressed and the \
                     agent would run with MORE power than it declares"
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
    Ok(argv)
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
            provider: None,
            presence: Presence::Unknown,
            argv: Vec::new(),
            gap: None,
            notices: Vec::new(),
        };
    };
    let presence = availability.presence(&adapter.provider);
    let mut notices = Vec::new();
    let (argv, gap) = match compose(agent, adapter, model, concrete, &mut notices) {
        Ok(argv) => (argv, None),
        // A blocked entry contributes no notices: it contributes an
        // error, and reporting both would double-count one gap.
        Err(gap) => (Vec::new(), Some(gap)),
    };
    ChainEntry {
        model: model.to_string(),
        provider: Some(adapter.provider.clone()),
        presence,
        argv,
        gap,
        notices,
    }
}

/// Walk an agent's whole chain without refusing anything but an unknown
/// name: the derivation `forge agents show` and `forge doctor` print.
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
            model: entry.model.clone(),
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
