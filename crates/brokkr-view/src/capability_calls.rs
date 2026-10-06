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

use serde::Serialize;
use serde_json::Value;

use crate::{cell_of, Cell};

/// The note on a checkpoint that carries no complete attribution group:
/// an ordinary local tool, a non-tool step, and every record written
/// before seat-record v6 all say this one thing.
pub const UNRECORDED: &str = "capability attribution unrecorded";

/// Seat-record v6's closed `call_state` vocabulary. A broker's private
/// `started` record is never a public checkpoint and is not a member.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
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
    fn of(word: &str) -> Option<CallState> {
        match word {
            "observed" => Some(CallState::Observed),
            "succeeded" => Some(CallState::Succeeded),
            "failed" => Some(CallState::Failed),
            "refused" => Some(CallState::Refused),
            "interrupted" => Some(CallState::Interrupted),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            CallState::Observed => "observed",
            CallState::Succeeded => "succeeded",
            CallState::Failed => "failed",
            CallState::Refused => "refused",
            CallState::Interrupted => "interrupted",
        }
    }
}

/// One recorded capability call: the abstract capability, the dialect
/// that served it, the concrete tool, the engine's call identity and the
/// state the journal settled it in.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct CapabilityCall {
    pub capability: String,
    pub dialect: String,
    pub tool: String,
    pub call_id: String,
    pub state: CallState,
    /// The retained response's digest, which the journal carries beside
    /// succeeded or failed only; absent where none is recorded.
    pub response_sha256: Option<String>,
}

/// A checkpoint's call evidence as a (structured value, rendered text)
/// pair: `call` is absent exactly when `cell` is, with [`UNRECORDED`].
#[derive(Serialize)]
pub struct CallEvidence {
    pub call: Option<CapabilityCall>,
    pub cell: Cell,
}

/// The call `checkpoint` records, when it carries the whole group with a
/// state in the vocabulary; otherwise none, and never a partial one.
fn capability_call(checkpoint: &Value) -> Option<CapabilityCall> {
    let text = |key: &str| match checkpoint.get(key) {
        Some(Value::String(value)) if !value.is_empty() => Some(value.clone()),
        _ => None,
    };
    Some(CapabilityCall {
        capability: text("capability")?,
        dialect: text("dialect")?,
        tool: text("tool")?,
        call_id: text("call_id")?,
        state: CallState::of(checkpoint.get("call_state")?.as_str()?)?,
        response_sha256: text("response_sha256"),
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

/// `checkpoint`'s evidence, as the view's checkpoint row carries it.
pub(crate) fn call_evidence(checkpoint: &Value) -> CallEvidence {
    let call = capability_call(checkpoint);
    let cell = cell_of(call.as_ref().map(line), Some(UNRECORDED));
    CallEvidence { call, cell }
}
