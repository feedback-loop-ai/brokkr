//! Whether a seat may start (#372, #355): the refusals that come before any
//! launch, and so before an `accepted` could be sent.

use serde_json::Value;

use super::{render_prompt, AdapterKind, OfferState, StartRefusal};
use crate::overrides::OverrideError;

/// The prompt a seat starts on, or why it launches nothing: a start with
/// no correlation (#372), an override that cannot be read (#355), or a
/// model seat whose charter cannot be read.
pub(super) fn start_prompt(
    effect_id: &str,
    attempt_id: &str,
    gate: Result<(), OverrideError>,
    input: &Value,
    kind: AdapterKind,
) -> Result<String, StartRefusal> {
    if effect_id.is_empty() {
        return Err(StartRefusal::MissingCorrelation("effect_id"));
    }
    if attempt_id.is_empty() {
        return Err(StartRefusal::MissingCorrelation("attempt_id"));
    }
    gate.map_err(StartRefusal::Override)?;
    render_prompt(input, kind)
}

/// The session a start rejoins: the offer made for its own effect and
/// attempt, none when nothing was offered, and a refusal for a poisoned
/// exchange or an offer that named another start (decision 0030).
pub(super) fn session(
    offered: OfferState,
    effect_id: &str,
    attempt_id: &str,
) -> Result<Option<String>, &'static str> {
    match offered {
        OfferState::None => Ok(None),
        OfferState::Poisoned(reason) => Err(reason),
        OfferState::Pending(pending) => {
            if pending.effect_id == effect_id && pending.attempt_id == attempt_id {
                Ok(Some(pending.handle))
            } else {
                Err("resume offer named a different effect or attempt")
            }
        }
    }
}
