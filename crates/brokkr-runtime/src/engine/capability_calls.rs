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
//! refuses the attempt. Any other call — workspace hands, a local tool, an
//! MCP call whose evidence is its broker's ledger (CC2) — stays an
//! ordinary checkpoint. A checkpoint that carries no observation is a
//! legacy row and passes unchanged: shipped drivers emit no observation
//! until every consumer is installed (design D9, U4f2).
//!
//! [`Checkpoints::offer`]: super::Checkpoints::offer

use std::collections::{BTreeMap, BTreeSet};

use brokkr_protocol::adapters::capability_calls::{Observation, Tool, OBSERVATION_KEY};
use brokkr_protocol::native_controls::managed;
use serde_json::{json, Value};

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
    /// A held tool's call without the harness's own id to own it by, or
    /// an observation that cannot be read. No id is guessed.
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
/// holdings admit, by its exact name, and every tool its adapter inventory
/// knows.
#[derive(Debug, Default)]
pub(super) struct Calls {
    held: BTreeMap<String, Holder>,
    known: BTreeSet<String>,
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
    pub(super) fn of(facts: Option<&SiteFacts>, spawn: &SiteSpawn) -> Calls {
        let sealed = spawn
            .record
            .as_ref()
            .map(|record| &record.expected.identity);
        facts
            .and_then(|facts| facts.capabilities.as_ref())
            .and_then(|site| {
                site.outcomes
                    .iter()
                    .find(|outcome| Some(&outcome.identity()) == sealed)
            })
            .map_or_else(Calls::default, Calls::serving)
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
        Calls { held, known }
    }

    /// Take the observation off `checkpoint`, whatever it says, and judge
    /// the call it describes. `owner` is the attempt, and the member or
    /// step under it, that the call id is owned by.
    pub(super) fn consume(
        &self,
        checkpoint: &mut Value,
        owner: &str,
    ) -> Result<Option<Stamp>, Refusal> {
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
        let call_id = call.map(|call| format!("{owner}:{call}"));
        Ok(Some(Stamp {
            tool,
            capability: holder.capability.clone(),
            dialect: holder.dialect.clone(),
            call_id: call_id.ok_or(Refusal::Unattributable)?,
        }))
    }
}

/// Who owns the calls a site makes: its attempt, and the member or step
/// tag it runs under.
pub(super) fn owner(attempt_id: &str, tag: Option<&str>) -> String {
    match tag {
        None => attempt_id.to_string(),
        Some(tag) => format!("{attempt_id}:{tag}"),
    }
}
