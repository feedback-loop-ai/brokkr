//! Whether a seat may start (#372, #355): the refusals that come before any
//! launch, and so before an `accepted` could be sent.

use serde_json::Value;

use super::{render_prompt, AdapterKind, OfferState, StartRefusal};
use crate::native_controls::{SealedServers, SealedServing, SERVING_INPUTS};
use crate::overrides::OverrideError;

/// What the driver's process takes on itself before it spawns a harness the
/// engine brokers MCP for: `no_new_privs` on Linux (U9b1b's ruling F1), set
/// by the verb, as this crate's rustix does not carry the call.
pub(super) type Confine = fn() -> std::io::Result<()>;

/// Why a start launches nothing, past its correlation.
#[derive(Debug, thiserror::Error)]
pub(super) enum Refused {
    #[error("resume exchange refused: {0}")]
    Resume(&'static str),
    #[error(
        "seat refused to start: its driver could not set no_new_privs before a harness the engine \
         brokers MCP for: {0}"
    )]
    Confinement(std::io::Error),
}

/// The session the start `message` rejoins ([`session`]), with its driver
/// confined first where the engine sealed brokers for the launch (MB3 reads
/// the harness and its parent as managed writers). Sealed inputs that do not
/// decode are no brokered launch here, as every launch reading them refuses
/// before any spawn. A launch with no brokered MCP leaves the driver as it was.
pub(super) fn admitted(
    message: &Value,
    confine: Confine,
    offered: OfferState,
    effect_id: &str,
    attempt_id: &str,
) -> Result<Option<String>, Refused> {
    let rejoined = session(offered, effect_id, attempt_id).map_err(Refused::Resume)?;
    let inputs = message
        .get("input")
        .and_then(|input| input.get(SERVING_INPUTS));
    let sealed = inputs.map(|inputs| SealedServing::decode(Some(inputs)));
    if let Some(Ok(sealed)) = sealed {
        if matches!(sealed.isolation.servers, SealedServers::Brokered(_)) {
            confine().map_err(Refused::Confinement)?;
        }
    }
    Ok(rejoined)
}

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

/// [`super::serve_with`] for the suites' starts that seal no broker.
#[cfg(test)]
pub(super) fn serve_io(
    kind: AdapterKind,
    extra: &[String],
    input: impl std::io::BufRead,
    output: impl std::io::Write,
) -> std::io::Result<()> {
    super::serve_with(kind, extra, input, output, super::tests::unconfined)
}

/// The session a start rejoins: the offer made for its own effect and
/// attempt, none when nothing was offered, and a refusal for a poisoned
/// exchange or an offer that named another start (decision 0030).
fn session(
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
