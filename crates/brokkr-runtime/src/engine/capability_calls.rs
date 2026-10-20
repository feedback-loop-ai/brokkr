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
//! A broker call is not observed here: its evidence is its broker's ledger,
//! which the engine folds once the attempt's processes settle (U8d; CC2,
//! CR4). Each accepted call the verified ledger holds becomes one settled
//! checkpoint ([`settled`]), under an id owned by the plan's sealed owner,
//! its server and its call sequence; and the committed journal, by that id
//! and the whole payload, says which are journaled already ([`unjournaled`]).
//!
//! [`Checkpoints::offer`]: super::Checkpoints::offer

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_protocol::adapters::capability_calls::{Format, Observation, Tool, OBSERVATION_KEY};
use brokkr_protocol::broker::ledger::{Invalid, Outcome as Ended};
use brokkr_protocol::broker::Plan;
use brokkr_protocol::native_controls::managed;
use serde::{Deserialize, Serialize};
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

/// The step every settled broker checkpoint names: the engine's own row,
/// as `panel-member-finished` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Step {
    #[serde(rename = "capability-call-settled")]
    Settled,
}

/// The canonical tuple a broker call id is the digest of (D7): the plan's
/// sealed attempt, site and instance, its server and the call's sequence in
/// its ledger.
#[derive(Serialize)]
struct BrokerCall<'a> {
    attempt: &'a str,
    site: &'a str,
    instance: &'a str,
    server: &'a str,
    call: u64,
}

/// How a settled broker call ended (CC2): seat-record v6's four settled
/// states. A broker's private Started is never one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Settled {
    Succeeded,
    Failed,
    Refused,
    Interrupted,
}

impl Settled {
    /// The state of a call its verified Terminal ended with `ended`, or
    /// interrupted where it has none.
    fn of(ended: Option<Ended>) -> Settled {
        match ended {
            Some(Ended::Succeeded) => Settled::Succeeded,
            Some(Ended::Failed) => Settled::Failed,
            Some(Ended::Refused(_)) => Settled::Refused,
            None => Settled::Interrupted,
        }
    }
}

/// One settled broker call's checkpoint (CC2, seat-record v6): the plan's
/// capability and dialect, the tool its ledger names, the engine's id, the
/// state, and the digest its Terminal cites, which only an answered call
/// has. A refused call carries no cause: seat-record v6 has no field for
/// one, and its typed denial stays in the private ledger (operator ruling
/// of 2026-10-20, for a later seat-record version).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BrokerCheckpoint {
    step: Step,
    tool: String,
    capability: String,
    dialect: String,
    call_id: String,
    call_state: Settled,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    response_sha256: Option<String>,
}

/// A settled broker checkpoint's whole event payload: the effect and the
/// attempt it settles, and the checkpoint. It is typed until the store's
/// edge, where [`SettledEvent::payload`] writes it, and read back whole
/// from the journal, so two are compared field by field, never by a
/// part of either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SettledEvent {
    effect_id: String,
    attempt_id: String,
    checkpoint: BrokerCheckpoint,
}

impl SettledEvent {
    /// `checkpoint`, settling the attempt `attempt_id` of the effect
    /// `effect_id`.
    pub(super) fn new(effect_id: &str, attempt_id: &str, checkpoint: BrokerCheckpoint) -> Self {
        SettledEvent {
            effect_id: effect_id.to_string(),
            attempt_id: attempt_id.to_string(),
            checkpoint,
        }
    }

    /// The event's payload, as the store journals it.
    pub(super) fn payload(&self) -> Value {
        serde_json::to_value(self).expect("a settled checkpoint of strings serializes")
    }
}

/// The settled checkpoint of call `call` of `plan`'s ledger, to `tool`,
/// which ended `ended`, or none where it was interrupted, citing `cited`
/// (CC2, CR4): `m-` and the SHA-256 of [`BrokerCall`] its id, 66
/// characters, so it is bounded and owned by the sealed plan, never by
/// anything the harness or the model said.
pub(super) fn settled(
    plan: &Plan,
    call: u64,
    tool: &str,
    ended: Option<Ended>,
    cited: Option<&str>,
) -> BrokerCheckpoint {
    let owner = &plan.owner;
    let tuple = BrokerCall {
        attempt: &owner.attempt,
        site: &owner.site,
        instance: &owner.instance,
        server: &plan.server,
        call,
    };
    let tuple = serde_json::to_value(tuple).expect("a tuple of strings serializes");
    BrokerCheckpoint {
        step: Step::Settled,
        tool: tool.to_string(),
        capability: plan.capability.clone(),
        dialect: plan.dialect.name.clone(),
        call_id: format!("m-{}", brokkr_core::canonical::sha256_hex(&tuple)),
        call_state: Settled::of(ended),
        response_sha256: cited.map(str::to_string),
    }
}

/// A checkpoint event's payload as far as its call id: enough to tell
/// whether it names a call the engine derived, never to compare it.
#[derive(Deserialize)]
struct Checkpointed {
    checkpoint: Identified,
}

#[derive(Deserialize)]
struct Identified {
    call_id: Option<String>,
}

/// Of `rows`, the settled checkpoints the engine derived for the attempt
/// `attempt`, those the committed `journal` does not yet hold (CR4). The
/// journal is the one authority, and no payload either side holds for a
/// call is passed over unseen: a second row the engine derived for one
/// call id, any committed row under a derived id that is not that row
/// whole, and a second committed row for one, are each CR3's conflicting
/// duplicate, refused before anything is appended. A derived row the
/// journal holds whole, once, is journaled already.
pub(super) fn unjournaled<'r>(
    journal: &[EventEnvelope],
    attempt: &str,
    rows: &'r [SettledEvent],
) -> Result<Vec<&'r SettledEvent>, Invalid> {
    let mut derived = BTreeMap::new();
    for row in rows {
        if derived
            .insert(row.checkpoint.call_id.as_str(), row)
            .is_some()
        {
            return Err(Invalid::Conflict);
        }
    }
    let call_id = |payload: &Value| Checkpointed::deserialize(payload).ok()?.checkpoint.call_id;
    let checkpointed = (journal.iter())
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| event.attempt_id.as_deref() == Some(attempt));
    let mut journaled = BTreeSet::new();
    for event in checkpointed {
        let id = call_id(&event.payload);
        let Some((&id, &row)) = id.and_then(|id| derived.get_key_value(id.as_str())) else {
            continue;
        };
        let committed = SettledEvent::deserialize(&event.payload).ok();
        if (committed.as_ref() != Some(row)) | !journaled.insert(id) {
            return Err(Invalid::Conflict);
        }
    }
    let pending = rows.iter();
    Ok(pending
        .filter(|row| !journaled.contains(row.checkpoint.call_id.as_str()))
        .collect())
}
