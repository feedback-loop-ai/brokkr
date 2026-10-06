//! The engine's one consumer of a harness tool call (decision 0065 slice
//! two, U4e; CC1, CC3 and SC4): the private observation a driver hands
//! over under [`OBSERVATION_KEY`] is decoded into protocol's one type and
//! taken off the checkpoint before [`Checkpoints::offer`], and the
//! capability-call attribution group is derived here, from the outcome the
//! site's spawn was sealed with, and from nothing the driver said.
//!
//! A tool the selected candidate's native holdings admit, by its exact
//! name, is attributed to that holding under an attempt-owned call id. A
//! tool the candidate's adapter inventory knows but no holding admits
//! refuses the attempt. The id is `n-` and the SHA-256 of the canonical
//! [`NativeCall`]: the attempt, the site's two engine stamps, the
//! selected provider and the harness's own id, so it is bounded whatever
//! the harness id's length and owned by its structural site, never by a
//! display tag. Any other call — workspace hands, a local tool, an
//! MCP call whose evidence is its broker's ledger (CC2) — stays an
//! ordinary checkpoint. A checkpoint that carries no observation is a
//! legacy row and passes unchanged: shipped drivers emit no observation
//! until every consumer is installed (design D9, U4f2).
//!
//! One harness call is one observed call (U4f; CC1): the second
//! observation of a call id an attempt already attributed, its start and
//! completion, stays an ordinary row. At a site offered a root, a
//! session-scoped id that root's earlier attempts journaled is history
//! the resumed harness replays, never a new use.
//!
//! [`Checkpoints::offer`]: super::Checkpoints::offer

use std::collections::{BTreeMap, BTreeSet};

use brokkr_protocol::adapters::capability_calls::{Format, Observation, Tool, OBSERVATION_KEY};
use brokkr_protocol::native_controls::managed;
use serde::Serialize;
use serde_json::{json, Value};

use super::resume::{RootHistory, SiteContext};
use super::SiteSpawn;
use crate::bundle::SiteFacts;
use crate::capabilities::{Implementation, NativePlan, Outcome};

/// Why an observed call fails its attempt. Each text is CC1's.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum Refusal {
    /// A tool the candidate's inventory knows, which none of its holdings
    /// admits. It is never journaled as an ordinary call.
    #[error("observed capability tool '{tool}' is not held by this attempt")]
    Unheld { tool: String },
    /// A held tool's call without the harness's own id to own it by, at a
    /// site with no engine stamps, or an observation that cannot be read.
    /// No id is guessed.
    #[error("capability telemetry cannot be attributed")]
    Unattributable,
}

/// The holding one tool is attributed to.
#[derive(Debug)]
struct Holder {
    capability: String,
    dialect: String,
}

/// One site's call authority: each tool the selected candidate's native
/// holdings admit, by its exact name, every tool its adapter inventory
/// knows, and who owns the calls it makes.
#[derive(Debug, Default)]
pub(super) struct Calls {
    held: BTreeMap<String, Holder>,
    known: BTreeSet<String>,
    owner: Option<Owner>,
    /// The call ids this attempt has attributed so far.
    seen: BTreeSet<String>,
}

/// The engine-owned part of a native call's identity: the attempt, the
/// site's structural stamps and the selected candidate's provider, and
/// the offered root's history at the site.
#[derive(Debug)]
struct Owner {
    attempt: String,
    site: String,
    instance: String,
    provider: String,
    history: RootHistory,
}

/// The canonical tuple a native call id is the digest of: the owner and
/// the harness's own measured id for the call.
#[derive(Serialize)]
struct NativeCall<'a> {
    attempt: &'a str,
    site: &'a str,
    instance: &'a str,
    provider: &'a str,
    call: &'a str,
}

impl Owner {
    /// `n-` and the SHA-256 of the canonical tuple: 66 characters, so
    /// any harness id fits seat-record v6's call id.
    fn call_id(&self, call: &str) -> String {
        self.call_id_in(&self.attempt, call)
    }

    /// The id `call` took, or would have taken, under `attempt`.
    fn call_id_in(&self, attempt: &str, call: &str) -> String {
        let tuple = NativeCall {
            attempt,
            site: &self.site,
            instance: &self.instance,
            provider: &self.provider,
            call,
        };
        let tuple = serde_json::to_value(tuple).expect("a tuple of strings serializes");
        format!("n-{}", brokkr_core::canonical::sha256_hex(&tuple))
    }

    /// Whether `call` is one the offered root's earlier attempts
    /// journaled at this site. Only a session-scoped id can be: Claude's
    /// `toolu_*` ids name a call for the session's life, and a resumed
    /// session emitted only new ones (U0, C08). Codex's `item_N` restart
    /// at `item_0` on every invocation, resume included (U0, X04/X05), so
    /// a repeated one is a fresh call; dsh's resume is unmeasured, and
    /// its ids are judged fresh with nothing measured to call them
    /// replayed.
    fn replays(&self, format: Format, call: &str) -> bool {
        match format {
            Format::Claude => self.history.attempts.iter().any(|attempt| {
                let earlier = self.call_id_in(attempt, call);
                self.history.calls.contains(&earlier)
            }),
            Format::Codex | Format::Dsh => false,
        }
    }
}

/// The attribution group the engine writes on one observed call: SC4's
/// whole group, with the concrete tool unclamped and no response digest.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    tool: String,
    capability: String,
    dialect: String,
    call_id: String,
}

impl Stamp {
    /// Write the group onto `checkpoint`, the object it was read from,
    /// after the driver's own attribution fields were erased.
    pub(super) fn apply(self, checkpoint: &mut Value) {
        checkpoint["tool"] = Value::String(self.tool);
        checkpoint["capability"] = Value::String(self.capability);
        checkpoint["dialect"] = Value::String(self.dialect);
        checkpoint["call_id"] = Value::String(self.call_id);
        checkpoint["call_state"] = Value::String("observed".to_string());
    }
}

impl Calls {
    /// The authority of the outcome `spawn` was sealed with: the selected
    /// candidate's own, found by the identity its launch record names, so
    /// a fallback never borrows its primary's holdings nor a member its
    /// sibling's. A spawn no outcome serves holds and knows nothing.
    /// `attempt` and `site`, the site's engine stamps, own its calls.
    pub(super) fn of(
        facts: Option<&SiteFacts>,
        spawn: &SiteSpawn,
        attempt: &str,
        site: Option<&SiteContext>,
    ) -> Calls {
        let sealed = spawn
            .record
            .as_ref()
            .map(|record| &record.expected.identity);
        let Some(outcome) = facts
            .and_then(|facts| facts.capabilities.as_ref())
            .and_then(|site| {
                site.outcomes
                    .iter()
                    .find(|outcome| Some(&outcome.identity()) == sealed)
            })
        else {
            return Calls::default();
        };
        let owner = site.map(|site| Owner {
            attempt: attempt.to_string(),
            site: site.site_ref.clone(),
            instance: site.instance_ref.clone(),
            provider: outcome.provider.clone(),
            history: site.history.clone(),
        });
        Calls {
            owner,
            ..Calls::serving(outcome)
        }
    }

    fn serving(outcome: &Outcome) -> Calls {
        let mut held = BTreeMap::new();
        for (capability, holding) in &outcome.held {
            match holding.implementation {
                Implementation::Native { .. } => {}
                // CC1: an MCP tool is told apart by its `cap-` server.
                Implementation::Mcp { .. } => continue,
            }
            for tool in &holding.tools {
                let holder = Holder {
                    capability: capability.clone(),
                    dialect: holding.dialect.clone(),
                };
                held.insert(tool.clone(), holder);
            }
        }
        // The inventory names what it knows in the plan's guards, read
        // through the decoder the driver reads them with: it identifies a
        // tool and grants none.
        let known = match &outcome.native {
            NativePlan::Known { controls, .. } => managed(&json!({"native_controls": controls}))
                .expect("the engine's own known plan reads back")
                .map(|plan| plan.guards.into_iter().flat_map(|guard| guard.tools))
                .into_iter()
                .flatten()
                .collect(),
            NativePlan::Unmeasured { .. } => BTreeSet::new(),
        };
        Calls {
            held,
            known,
            ..Calls::default()
        }
    }

    /// Take the observation off `checkpoint`, whatever it says, and judge
    /// the call it describes. A held call is attributed once: replayed
    /// history and a call this attempt already attributed stay ordinary.
    pub(super) fn consume(&mut self, checkpoint: &mut Value) -> Result<Option<Stamp>, Refusal> {
        let Some(observed) = checkpoint
            .as_object_mut()
            .and_then(|object| object.remove(OBSERVATION_KEY))
        else {
            return Ok(None);
        };
        let observation: Observation =
            serde_json::from_value(observed).map_err(|_| Refusal::Unattributable)?;
        let tool = match observation.tool {
            Tool::Named { name } => name,
            // CC2: an MCP call's evidence is its broker's ledger, never a
            // second attributed call; a call that names nothing claims
            // nothing.
            Tool::Mcp { .. } | Tool::McpUnidentified { .. } | Tool::Missing => return Ok(None),
        };
        let Some(holder) = self.held.get(&tool) else {
            return match self.known.contains(&tool) {
                true => Err(Refusal::Unheld { tool }),
                false => Ok(None),
            };
        };
        let call = observation.call.filter(|call| !call.is_empty());
        let (call, owner) = call
            .zip(self.owner.as_ref())
            .ok_or(Refusal::Unattributable)?;
        let call_id = owner.call_id(&call);
        if owner.replays(observation.format, &call) || !self.seen.insert(call_id.clone()) {
            return Ok(None);
        }
        Ok(Some(Stamp {
            tool,
            capability: holder.capability.clone(),
            dialect: holder.dialect.clone(),
            call_id,
        }))
    }
}
