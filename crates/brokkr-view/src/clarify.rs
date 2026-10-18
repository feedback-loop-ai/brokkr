//! Two facts a console needs to send the operator to the spec (#568,
//! design D7): the open questions a run parked on when clarify's rounds
//! ran out, and the working tree the run was started in. Both are read
//! from the journal's bytes alone, parsed once into a type here; the
//! disk is never read and a path is never guessed (decision 0071 ruling 1).

use std::collections::BTreeMap;

use brokkr_core::fold::{RunState, Status};
use brokkr_core::{EventEnvelope, EventType};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::fleet::{last_rule, ruling_parked};

/// The phase whose gate judges a specification's ambiguity: a run parked
/// by its ruling waits on the questions its clarifier left open.
const CLARIFY_PHASE: &str = "clarify";

/// The open questions of a run parked after clarify's rounds ran out.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Clarify {
    /// The `effect/succeeded` the clarifier's record was read from: the
    /// citation a reader can check.
    pub seq: u64,
    /// The questions, in the order the record lists them. Empty when the
    /// record's notes are absent or neither text nor a list of texts.
    pub questions: Vec<String>,
}

/// The part of a seat record the questions are read from. A record
/// carries more, so unknown fields are left unread rather than refused.
#[derive(Deserialize)]
struct ClarifierRecord {
    #[serde(default)]
    notes: Option<Notes>,
}

/// A record's notes, in the two shapes that list questions: one per
/// entry, or one per line.
#[derive(Deserialize)]
#[serde(untagged)]
enum Notes {
    Entries(Vec<String>),
    Text(String),
}

impl Notes {
    fn questions(self) -> Vec<String> {
        let lines = match self {
            Notes::Entries(entries) => entries,
            Notes::Text(text) => text.lines().map(str::to_string).collect(),
        };
        lines
            .into_iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }
}

/// Whether the run is parked by a ruling from clarify: its last ruling
/// came from that phase and routed nowhere, which is how the fold parks
/// on a rule (the triage recipe's `CLARIFY-AMBIGUOUS-EXHAUSTED`).
fn parked_by_clarify(state: &RunState) -> bool {
    state.run.status() == Status::AwaitingOperator
        && ruling_parked(state)
        && last_rule(state).and_then(|(_, from)| from) == Some(CLARIFY_PHASE)
}

/// The open questions of a run parked by clarify's ruling, read from the
/// clarifier's recorded result: the fold's `last_result`, which the last
/// `effect/succeeded` carries. `None` for every other run.
pub(crate) fn clarify(events: &[EventEnvelope], state: &RunState) -> Option<Clarify> {
    if !parked_by_clarify(state) {
        return None;
    }
    let seq = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::EffectSucceeded)?
        .seq;
    let questions = state
        .last_result
        .as_ref()
        .and_then(|record| ClarifierRecord::deserialize(record).ok())
        .and_then(|record| record.notes)
        .map_or_else(Vec::new, Notes::questions);
    Some(Clarify { seq, questions })
}

/// `run/started`'s world pin, as far as the working tree needs it.
#[derive(Deserialize)]
struct Started {
    manifest: Manifest,
}

#[derive(Deserialize)]
struct Manifest {
    realms: Option<Pin>,
    #[serde(default)]
    bundle_name: Option<String>,
}

#[derive(Deserialize)]
struct Pin {
    source: String,
    realm: Option<String>,
    map: PinnedMap,
}

#[derive(Deserialize)]
struct PinnedMap {
    realms: Vec<PinnedRealm>,
}

#[derive(Deserialize)]
struct PinnedRealm {
    name: String,
    path: String,
}

/// The world pin `run/started` carries, when it carries one whole.
fn manifest(events: &[EventEnvelope]) -> Option<Manifest> {
    let first = events
        .first()
        .filter(|event| event.event_type == EventType::RunStarted)?;
    Some(Started::deserialize(&first.payload).ok()?.manifest)
}

fn pin(events: &[EventEnvelope]) -> Option<Pin> {
    manifest(events)?.realms
}

/// The recipe the run started under: the `bundle_name` its `run/started`
/// manifest recorded (#568 slice two, design D1). `None` when the first
/// event is not a start or names none; never guessed from anything else.
pub(crate) fn recipe(events: &[EventEnvelope]) -> Option<String> {
    manifest(events)?.bundle_name
}

/// What a manifest pins that a recipe's graph reads (#568, the Library's
/// D2 and D4): its files' digests and the agents its seats hired. Parsed
/// apart from [`Manifest`], so a pin this reader cannot type leaves the
/// realm and the recipe readable, and states no pin rather than a guess.
#[derive(Deserialize, Default)]
pub(crate) struct Pins {
    #[serde(default)]
    pub(crate) files: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) agents: BTreeMap<String, Hired>,
}

/// One seat's agent resolution, as far as the graph names it.
#[derive(Deserialize)]
pub(crate) struct Hired {
    #[serde(default)]
    pub(crate) provider: Option<String>,
    #[serde(default)]
    pub(crate) model: Option<String>,
}

/// The pins of a manifest: a compiled bundle's, or a run's start's.
pub(crate) fn pins_of(manifest: &Value) -> Pins {
    Pins::deserialize(manifest).unwrap_or_default()
}

/// The pins `run/started` carries, none when the first event is no start.
pub(crate) fn pins(events: &[EventEnvelope]) -> Pins {
    let first = events
        .first()
        .filter(|event| event.event_type == EventType::RunStarted);
    first
        .and_then(|event| event.payload.get("manifest"))
        .map_or_else(Pins::default, pins_of)
}

/// The realm the run was started in, as its world pin names it.
pub(crate) fn pinned_realm(events: &[EventEnvelope]) -> Option<String> {
    pin(events)?.realm
}

/// The working tree the run was started in, from its `run/started`
/// manifest's world pin: the pinned realm's `path`, resolved against the
/// pinned map's own directory as the world resolves it. `None` when the
/// run pinned no world or no realm, or when the path does not resolve to
/// an absolute one: a relative path would be a guess at a directory.
pub(crate) fn worktree(events: &[EventEnvelope]) -> Option<String> {
    let pin = pin(events)?;
    let name = pin.realm?;
    let realm = pin
        .map
        .realms
        .into_iter()
        .find(|realm| realm.name == name)?;
    // Joined as text, on the `/` of the two supported hosts (decision
    // 0063): this crate names no `std::path` (decision 0071 ruling 1).
    let path = match realm.path.starts_with('/') {
        true => realm.path,
        false => {
            let (base, _) = pin.source.rsplit_once('/')?;
            format!("{base}/{}", realm.path)
        }
    };
    path.starts_with('/').then_some(path)
}

#[cfg(test)]
mod tests;
