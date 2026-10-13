//! A ruling as the fold keeps it: the `transition/decided` record, the
//! verdict each ruled phase hands forward, and the heads the protected
//! phase recorded. Each is read from the journal once, here, and a
//! record serializes back to the bytes it was read from, `null` and
//! absent kept apart (#345).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::policy::{RuleSeverity, REVIEWED_HEADS, RULING_SEVERITIES};
use crate::realms::LEGACY_REALM_KEY;

/// A field the contract writes as a value or `null`, and which a
/// hand-built journal may leave out. The three stay apart, so a record
/// serializes to the bytes it was read from.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Recorded<T> {
    #[default]
    Absent,
    Null,
    Present(T),
}

impl<T> Recorded<T> {
    /// Whether the record left the field out. Every field holding one
    /// skips serializing it by this, so `Absent` is never written.
    pub fn is_absent(&self) -> bool {
        matches!(self, Recorded::Absent)
    }

    /// The value, when the record holds one.
    pub fn present(&self) -> Option<&T> {
        match self {
            Recorded::Present(value) => Some(value),
            Recorded::Absent | Recorded::Null => None,
        }
    }
}

impl<T: Serialize> Serialize for Recorded<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.present().serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Recorded<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<T>::deserialize(deserializer)?.map_or(Recorded::Null, Recorded::Present))
    }
}

/// By hand, not derived: serde's enum derive also reads a variant's
/// one-key map form (`{"normal": null}`), which would fold a record
/// that does not serialize back to its own bytes (decision 0084
/// rulings 1 and 2). The word alone reads.
impl<'de> Deserialize<'de> for RuleSeverity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(RuleSeverityWord)
    }
}

struct RuleSeverityWord;

impl serde::de::Visitor<'_> for RuleSeverityWord {
    type Value = RuleSeverity;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a ruling severity string")
    }

    fn visit_str<E: serde::de::Error>(self, word: &str) -> Result<RuleSeverity, E> {
        RuleSeverity::named(word).ok_or_else(|| E::unknown_variant(word, &RULING_SEVERITIES))
    }
}

/// A `transition/decided` payload: the policy's ruling on one result.
/// The v1 contract lists its seven fields, and no extension schema adds
/// one, so a field outside them is refused rather than dropped
/// (decision 0084; a published extension lands here as an
/// optional field the fold never reads). Fields are
/// declared in key order, so the record prints as the map it replaced.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub from: String,
    #[serde(default, skip_serializing_if = "Recorded::is_absent")]
    pub inputs: Recorded<Map<String, Value>>,
    #[serde(default, skip_serializing_if = "Recorded::is_absent")]
    pub next: Recorded<String>,
    #[serde(default, skip_serializing_if = "Recorded::is_absent")]
    pub problem: Recorded<String>,
    pub result: String,
    #[serde(default, skip_serializing_if = "Recorded::is_absent")]
    pub rule_id: Recorded<String>,
    #[serde(default, skip_serializing_if = "Recorded::is_absent")]
    pub severity: Recorded<RuleSeverity>,
}

impl Decision {
    /// The value the decision recorded for one input, `null` included.
    pub(crate) fn input(&self, name: &str) -> Option<&Value> {
        self.inputs.present()?.get(name)
    }
}

/// The verdict a ruled phase hands forward to the seats after it: its
/// result, and the inputs its ruling recorded. A decision that recorded
/// no inputs hands forward `{}`, and one that recorded `null` hands
/// forward `null`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PhaseResult {
    pub inputs: Option<Map<String, Value>>,
    pub result: String,
}

impl PhaseResult {
    /// The verdict `decision` hands forward.
    pub fn of(decision: &Decision) -> PhaseResult {
        PhaseResult {
            inputs: match &decision.inputs {
                Recorded::Absent => Some(Map::new()),
                Recorded::Null => None,
                Recorded::Present(inputs) => Some(inputs.clone()),
            },
            result: decision.result.clone(),
        }
    }
}

/// The heads the protected phase recorded, by realm name. An entry
/// whose value is not a string is kept as a realm with no readable
/// head, so a reader counts it and never falls back past it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReviewedHeads(BTreeMap<String, Option<String>>);

impl ReviewedHeads {
    /// A recorded `reviewed_heads` value. One that is not an object
    /// records no head.
    pub fn read(recorded: &Value) -> ReviewedHeads {
        ReviewedHeads(
            recorded
                .as_object()
                .into_iter()
                .flatten()
                .map(|(realm, head)| (realm.clone(), head.as_str().map(str::to_string)))
                .collect(),
        )
    }

    /// The heads a decision payload's inputs record, when they record
    /// any: the reader for journals too damaged to fold.
    pub fn recorded_in(payload: &Value) -> Option<ReviewedHeads> {
        let inputs = payload.get("inputs")?;
        inputs.get(REVIEWED_HEADS).map(ReviewedHeads::read)
    }

    /// The head recorded for one realm, in the TWO shapes the ruling
    /// names and no third one.
    ///
    /// A journal written before any map recorded one unkeyed head under
    /// [`LEGACY_REALM_KEY`], and it is still read: the per-realm lookup
    /// falls back to that key. A mapped run records the head under the
    /// realm's own name, and it answers to that name alone — a reader
    /// that cannot name the realm is told nothing rather than handed
    /// whichever head happened to be recorded, because a head from
    /// another realm would be compared against this realm's tree. No
    /// reader has to guess: `brokkr resume` takes no map but rehydrates
    /// the world from the run's own manifest pin.
    pub fn head_for(&self, realm: Option<&str>) -> Option<&str> {
        realm
            .and_then(|name| self.0.get(name))
            .or_else(|| self.0.get(LEGACY_REALM_KEY))?
            .as_deref()
    }

    /// Every readable head, in realm order.
    pub(crate) fn heads(&self) -> impl Iterator<Item = &str> {
        self.0.values().filter_map(Option::as_deref)
    }

    /// The head, when exactly one realm is recorded and its head reads.
    /// More than one realm is ambiguous and answers nothing.
    pub fn sole(&self) -> Option<&str> {
        self.0
            .values()
            .next()
            .filter(|_| self.0.len() == 1)?
            .as_deref()
    }
}

#[cfg(test)]
mod tests;
