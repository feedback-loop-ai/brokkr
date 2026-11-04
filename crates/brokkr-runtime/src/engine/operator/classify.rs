//! `classify`: the operator annotation that names how a finished run
//! ended (decision 0064 ruling 5). It shares the fence and the read-back
//! with `supersede`, and the cause's rules with `stop` and `conclude`
//! ([`super::ending`]), and nothing else: it is no command with a
//! disposition, and no `operator/accepted` ever follows it. The fold
//! reads neither the command's `args` nor anything else about it; the
//! view reads the latest annotation, and a later one stands over an
//! earlier one.

use brokkr_core::ending::CLASSIFY;
use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use brokkr_core::summary::Account;
use brokkr_core::EventEnvelope;
use brokkr_store::Store;
use serde_json::json;
use uuid::Uuid;

use super::ending::{checked, Named, Shaped, Verb};
use super::{fenced_append, read_back, EngineError, Head};

/// Record how a finished run ended (decision 0064 ruling 5): one
/// `operator/commanded` event with `command: "classify"`, and nothing
/// else. No `operator/accepted` follows, because there is nothing to
/// execute and `fold` refuses an acceptance on a terminal run by design.
/// The event is the record, exactly as `supersede`'s is.
///
/// Every refusal comes before the first append, and a refusal writes
/// nothing at all — which is why this returns
/// [`EngineError::ClassifyRefused`] or [`EngineError::CauseRefused`]
/// rather than journaling a rejection the way `retry` and `stop` do.
/// There is no pending command here for a rejection to dispose of.
///
/// `by_journal` is the journal `by_realm` names, opened by the caller
/// (the workspace journal when the citation names no realm): a cited run
/// may live in another hearth, and a citation into a journal this process
/// never opened is one nobody can follow back.
///
/// The write is FENCED on the head the verification read (decision 0029),
/// as `supersede`'s is: the annotation is legal wherever it lands, but a
/// head that moved is a journal that grew under a verification, and
/// re-deciding beats writing against a reading that is no longer the
/// whole story.
pub fn operator_classify(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    ask: &Named,
) -> Result<EventEnvelope, EngineError> {
    operator_classify_racing(store, by_journal, run_id, ask, |_| {})
}

/// [`operator_classify`], with the window its fence exists for made
/// reachable. `between` runs after the verifying load, before the fenced
/// append. Production passes a no-op; the test passes a peer's append,
/// because a fence is either always there or it is not a fence.
pub(in crate::engine) fn operator_classify_racing(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    ask: &Named,
    mut between: impl FnMut(&mut Store),
) -> Result<EventEnvelope, EngineError> {
    // One voice for every refusal below, and every one of them comes
    // before the first append: a refused classify leaves the journal byte
    // for byte as it found it.
    // The verifying fold is `decided`'s, except that a journal whose
    // events do not fold is this verb's refusal, naming the run and the
    // fold's own words; a store that cannot load the run stays the
    // store's error.
    let events = store.load(run_id)?;
    let account = Account::whole(run_id, &events);
    let state = account
        .state()
        .map_err(|error| {
            refused(format!(
                "run '{run_id}' does not fold, so its ending cannot be named: {error}"
            ))
        })?
        .clone();
    store.carry(account);
    let head = Head::tip(&events);
    if state.run.status() != Status::Stopped {
        return Err(refused(format!(
            "run '{run_id}' is {}; classify names a stopped run's ending",
            state.run.status().as_str()
        )));
    }
    let shaped = checked(Verb::Classify, &ask.cause, by_journal)?;
    between(store);
    classified(store, run_id, &head, &shaped, ask)
}

/// The annotation itself, fenced on `head`: `shaped`'s `args` with the
/// operator's sentence beside them, and the read-back. `conclude` writes
/// it after `run/stopped` when the stop in force named another cause or
/// none (choice 4 (a)), exactly as the verb writes it.
pub(super) fn classified(
    store: &mut Store,
    run_id: &str,
    head: &Head,
    shaped: &Shaped,
    ask: &Named,
) -> Result<EventEnvelope, EngineError> {
    let mut args = shaped.args();
    args["reason"] = json!(ask.reason);
    let payload = json!({
        "command_id": Uuid::new_v4().to_string(),
        "command": CLASSIFY,
        "operator": ask.operator,
        "args": args,
    });
    // A head that moved comes back in the store's own words — `head
    // moved: expected seq N, found seq M` — rather than paraphrased into
    // a refusal of this verb's own. Nothing was written either way, and
    // the operator reads the journal again and asks again.
    let cause = &head.event_id;
    let written = fenced_append(
        store,
        run_id,
        head,
        cause,
        EventType::OperatorCommanded,
        payload,
        None,
    )?;
    // The annotation must read back, and reading back must change nothing
    // (ruling 5). Same proof the fenced command path takes.
    read_back(store, run_id)?;
    Ok(written)
}

/// The one voice of this verb's refusals of the run it names.
fn refused(text: impl Into<String>) -> EngineError {
    EngineError::ClassifyRefused(text.into())
}

#[cfg(test)]
#[path = "classify/tests.rs"]
mod tests;
