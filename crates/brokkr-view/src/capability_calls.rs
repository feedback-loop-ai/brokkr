//! A checkpoint's capability-call evidence (decision 0065 ruling 8),
//! derived once here for every read surface (decision 0071 ruling 7).
//!
//! The journal's settled checkpoint IS the call evidence: this reads the
//! seat-record v6 attribution group off one checkpoint and nothing else.
//! It groups no private started or terminal stage, merges no two rows
//! that share a call id, and looks up no grant — a view admits nothing.
//! A historical checkpoint whose tool is `WebSearch` and which carries no
//! capability reads unrecorded: neither the tool's name nor today's realm
//! supplies attribution the journal never held (CC3). A missing
//! `response_sha256` says no retained response is recorded, never that
//! retention was vetoed (CR5).

use serde::de::{Deserializer, Error as _};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

use crate::{cell_of, Cell};

/// The note on a checkpoint that carries no complete attribution group:
/// an ordinary local tool, a non-tool step, and every record written
/// before seat-record v6 all say this one thing.
pub const UNRECORDED: &str = "capability attribution unrecorded";

/// The note on an attributed call whose checkpoint cites no retained
/// response (CR5). It says no digest is recorded and nothing more: not
/// that the realm vetoed retention, not that the call was granted, and
/// not that any response reached the model.
pub const NO_RETAINED: &str = "no retained response is recorded for this call";

/// Seat-record v6's closed `call_state` vocabulary. A broker's private
/// `started` record is never a public checkpoint and is not a member.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CallState {
    /// A native call the harness reported; it claims no completion.
    Observed,
    Succeeded,
    Failed,
    /// Denied before forwarding: evidence of a refusal, never of a grant.
    Refused,
    Interrupted,
}

impl CallState {
    const ALL: [CallState; 5] = [
        CallState::Observed,
        CallState::Succeeded,
        CallState::Failed,
        CallState::Refused,
        CallState::Interrupted,
    ];

    /// Each state's one spelling: the journal's word, what the view
    /// serializes, and what every surface prints.
    fn word(self) -> &'static str {
        match self {
            CallState::Observed => "observed",
            CallState::Succeeded => "succeeded",
            CallState::Failed => "failed",
            CallState::Refused => "refused",
            CallState::Interrupted => "interrupted",
        }
    }

    fn of(word: &str) -> Option<CallState> {
        CallState::ALL
            .into_iter()
            .find(|state| state.word() == word)
    }
}

impl Serialize for CallState {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// One recorded capability call: the abstract capability, the dialect
/// that served it, the concrete tool, the engine's call identity and the
/// state the journal settled it in. It is parsed once off a checkpoint,
/// whose other fields it leaves alone, so it reads the journal's
/// `call_state` and serializes the view's `state`.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct CapabilityCall {
    #[serde(deserialize_with = "word")]
    pub capability: String,
    #[serde(deserialize_with = "word")]
    pub dialect: String,
    #[serde(deserialize_with = "word")]
    pub tool: String,
    #[serde(deserialize_with = "word")]
    pub call_id: String,
    #[serde(rename(deserialize = "call_state"), deserialize_with = "state")]
    pub state: CallState,
    /// The retained response's digest, which the journal carries beside
    /// succeeded or failed only; absent where none is recorded.
    #[serde(default, deserialize_with = "digest")]
    pub response_sha256: Option<String>,
}

/// A checkpoint's call evidence as a structured value and the rendered
/// text of the call and of its retained response: `call` is absent
/// exactly when `cell` is, with [`UNRECORDED`]; `response` is present
/// exactly when the call cites a digest, and otherwise says which of
/// the two absences the journal holds.
#[derive(Serialize)]
pub struct CallEvidence {
    pub call: Option<CapabilityCall>,
    pub cell: Cell,
    pub response: Cell,
}

/// A group field: a nonempty string, or the whole group is unrecorded.
fn word<'de, D: Deserializer<'de>>(field: D) -> Result<String, D::Error> {
    String::deserialize(field).and_then(|word| {
        Some(word)
            .filter(|word| !word.is_empty())
            .ok_or_else(|| D::Error::custom("an empty attribution field"))
    })
}

/// The group's state, inside the vocabulary or the group is unrecorded.
fn state<'de, D: Deserializer<'de>>(field: D) -> Result<CallState, D::Error> {
    word(field).and_then(|word| {
        CallState::of(&word).ok_or_else(|| D::Error::custom("a call state outside v6"))
    })
}

/// A digest that is not a nonempty string is no digest; it never costs
/// the call its attribution.
fn digest<'de, D: Deserializer<'de>>(field: D) -> Result<Option<String>, D::Error> {
    Value::deserialize(field).map(|value| match value {
        Value::String(digest) if !digest.is_empty() => Some(digest),
        _ => None,
    })
}

/// The line every surface prints for `call`: `web-search via
/// claude-native-search · WebSearch observed`, with `· response
/// retained` after a state that kept one.
fn line(call: &CapabilityCall) -> String {
    let retained = match call.response_sha256 {
        Some(_) => " · response retained",
        None => "",
    };
    format!(
        "{} via {} · {} {}{retained}",
        call.capability,
        call.dialect,
        call.tool,
        call.state.word()
    )
}

/// The retained response `call` cites: its digest, or absent with why.
fn response(call: Option<&CapabilityCall>) -> Cell {
    match call {
        None => cell_of(None, Some(UNRECORDED)),
        Some(call) => cell_of(call.response_sha256.clone(), Some(NO_RETAINED)),
    }
}

/// `checkpoint`'s evidence, as the view's checkpoint row carries it: the
/// whole group with a state in the vocabulary, or none and never a
/// partial one.
pub(crate) fn call_evidence(checkpoint: &Value) -> CallEvidence {
    let call = CapabilityCall::deserialize(checkpoint).ok();
    let cell = cell_of(call.as_ref().map(line), Some(UNRECORDED));
    let response = response(call.as_ref());
    CallEvidence {
        call,
        cell,
        response,
    }
}
