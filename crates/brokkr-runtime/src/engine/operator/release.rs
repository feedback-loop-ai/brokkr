//! `release` (decision 0065 slice two, U8e3; operator ruling 2026-10-21,
//! "A"): the operator's release of the writer reservation of a run whose
//! attempts ended with no proof that their drivers settled, such as one
//! whose engine was killed while its driver ran. Issuing it is the
//! operator's attestation that every process those attempts owned has
//! ended; it is settlement's one exception to positive proof
//! ([`attested`]), and nothing else may use it.
//!
//! The command is journaled with its reason before anything is released:
//! one `operator/commanded` naming `release`, the reason in its `args`, and
//! on a live run the `operator/accepted` that disposes of it; a run already
//! completed or stopped takes no acceptance (`fold` refuses one there), so
//! the command alone is the record, as `supersede`'s is. Both are fenced on
//! the head the release was judged on, under the admission lock. A refusal
//! journals nothing and releases nothing.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{concluded, OperatorCommand, Refusal};
use brokkr_core::EventEnvelope;
use brokkr_store::{Store, StoreError};
use serde_json::json;
use uuid::Uuid;

use super::{accepted, decided, fenced_append, read_back, refuse, FencedCommandOutcome, Head};
use super::{EngineError, Rejection};
use crate::engine::broker::admission::Unknown;
use crate::engine::broker::observed;
use crate::engine::broker::settlement::{attested, Refused, Unreleased};

/// Release the run `run_id`'s standing attempts on `operator`'s
/// attestation, journaled with `reason`, in the coordination root the host
/// observes as dispatch does; refused where none is protected, for then
/// none can be read.
pub(super) fn released(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    released_racing(store, run_id, [operator, reason], |_| {})
}

/// [`released`], `between` run before each of its fenced appends: after
/// the release was judged, and after the command landed. Production passes
/// a no-op; tests pass a peer, for a fence is either always there or it is
/// not a fence.
pub(in crate::engine) fn released_racing(
    store: &mut Store,
    run_id: &str,
    [operator, reason]: [&str; 2],
    mut between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    let observed = observed();
    let root = observed.home.as_deref().and_then(observed.open);
    let root = root.ok_or(Unreleased::standing(Unknown::Root.into()))?;
    attested(&root, run_id, store, |store, events| {
        journaled(store, run_id, events, [operator, reason], &mut between)
    })?;
    read_back(store, run_id)?;
    accepted(store, run_id)
}

/// The operator's command journaled with its reason on the head of
/// `events`, the journal the release was judged on, and on a live run its
/// acceptance on the command; a refusal where a peer moved either head,
/// the command, where it landed, disposed of as a run that moved.
fn journaled(
    store: &mut Store,
    run_id: &str,
    events: &[EventEnvelope],
    [operator, reason]: [&str; 2],
    between: &mut impl FnMut(&mut Store),
) -> Result<(), EngineError> {
    let (state, head) = decided(store, run_id, events)?;
    let command_id = Uuid::new_v4().to_string();
    let release = OperatorCommand::Release.as_str();
    let args = json!({"reason": reason});
    let commanded =
        json!({"command_id": command_id, "command": release, "args": args, "operator": operator});
    let moved = || EngineError::from(Unreleased::refused(Refused::Moved));
    between(store);
    let kind = EventType::OperatorCommanded;
    let commanded = match fenced_append(store, run_id, &head, &head.event_id, kind, commanded, None)
    {
        Err(StoreError::HeadMoved { .. }) => return Err(moved()),
        written => written?,
    };
    if concluded(&state).is_some() {
        return Ok(());
    }
    let (head, cause) = (Head::of(&commanded), commanded.event_id.as_str());
    let payload = json!({"command_id": command_id, "operator": operator, "reason": reason});
    between(store);
    let kind = EventType::OperatorAccepted;
    match fenced_append(store, run_id, &head, cause, kind, payload, None) {
        Err(StoreError::HeadMoved { .. }) => {
            let (command_id, refusal) = (&command_id, Refusal::LostFence);
            let rejection = Rejection {
                command_id,
                operator,
                refusal,
                cause,
            };
            refuse(store, run_id, rejection)?;
            Err(moved())
        }
        written => written.map(drop).map_err(EngineError::from),
    }
}
