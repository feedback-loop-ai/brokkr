//! `classify`: the operator annotation that names how a finished run
//! ended (decision 0064 ruling 5). It shares the fence and the read-back
//! with `supersede`, and nothing else: it is no command with a
//! disposition, and no `operator/accepted` ever follows it. The fold
//! reads neither the command's `args` nor anything else about it; the
//! view reads the latest annotation, and a later one stands over an
//! earlier one.

use brokkr_core::ending::{Ending, CLASSIFY};
use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use brokkr_core::summary::Account;
use brokkr_core::EventEnvelope;
use brokkr_store::Store;
use serde_json::json;
use uuid::Uuid;

use super::{fenced_append, read_back, EngineError, Head};

/// What `brokkr operator classify` says (decision 0064 ruling 5): the
/// ending one finished run takes, the run or ref a cause cites, why, and
/// who named it.
pub struct Classify<'a> {
    /// The ending the operator names, parsed where it was typed by
    /// [`classify_cause`] (decision 0071 ruling 3). Only ruling 4's six
    /// operator-owned endings are accepted; any other is refused.
    pub cause: Ending,
    /// The realm the cited run was read in, or `None` for the workspace
    /// journal — decision 0026 ruling 3's key, on a citation.
    pub by_realm: Option<&'a str>,
    /// The run that replaced this one; `superseded` only.
    pub by_run: Option<&'a str>,
    /// The commit or pull request the work landed by; `delivered-elsewhere`
    /// only, recorded as written and not resolved.
    pub by_ref: Option<&'a str>,
    pub reason: &'a str,
    pub operator: &'a str,
}

/// The cause an operator typed, parsed once at the edge into an
/// [`Ending`] (decision 0071 ruling 3): a word outside the twelve is this
/// verb's refusal, before any journal is opened. Whether the ending is one
/// an operator may name is [`operator_classify`]'s refusal.
pub fn classify_cause(word: &str) -> Result<Ending, EngineError> {
    Ending::named(word).map_err(|error| refused(error.to_string()))
}

/// Record how a finished run ended (decision 0064 ruling 5): one
/// `operator/commanded` event with `command: "classify"`, and nothing
/// else. No `operator/accepted` follows, because there is nothing to
/// execute and `fold` refuses an acceptance on a terminal run by design.
/// The event is the record, exactly as `supersede`'s is.
///
/// Every refusal comes before the first append, and a refusal writes
/// nothing at all — which is why this returns
/// [`EngineError::ClassifyRefused`] rather than journaling a rejection
/// the way `retry` and `stop` do. There is no pending command here for a
/// rejection to dispose of.
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
    ask: &Classify,
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
    ask: &Classify,
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
    let ending = ask.cause;
    let (by_run, by_ref) = shape(ending, ask)?;
    if let Some(by_run) = by_run {
        cited(by_journal, by_run, ask.by_realm)?;
    }
    let mut args = json!({"cause": ending.word(), "reason": ask.reason});
    if let Some(by_run) = by_run {
        args["by"] = json!({"realm": ask.by_realm, "run_id": by_run});
    }
    if let Some(by_ref) = by_ref {
        args["ref"] = json!(by_ref);
    }
    let payload = json!({
        "command_id": Uuid::new_v4().to_string(),
        "command": CLASSIFY,
        "operator": ask.operator,
        "args": args,
    });
    between(store);
    // A head that moved comes back in the store's own words — `head
    // moved: expected seq N, found seq M` — rather than paraphrased into
    // a refusal of this verb's own. Nothing was written either way, and
    // the operator reads the journal again and asks again.
    let cause = &head.event_id;
    let written = fenced_append(
        store,
        run_id,
        &head,
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

/// The one voice of this verb's refusals.
fn refused(text: impl Into<String>) -> EngineError {
    EngineError::ClassifyRefused(text.into())
}

/// The citation a cause requires: `--by-run` for `superseded`, `--by-ref`
/// for `delivered-elsewhere`, and neither for the other four; a citation
/// that belongs to another cause is refused by name.
fn shape<'a>(
    ending: Ending,
    ask: &'a Classify<'_>,
) -> Result<(Option<&'a str>, Option<&'a str>), EngineError> {
    match ending {
        Ending::Superseded => {
            if ask.by_ref.is_some() {
                return Err(refused(
                    "classify --cause superseded takes no --by-ref; name the run that \
                     replaced this one with --by-run"
                        .to_string(),
                ));
            }
            let Some(by_run) = ask.by_run else {
                return Err(refused(
                    "classify --cause superseded needs --by-run: the run that replaced this one"
                        .to_string(),
                ));
            };
            Ok((Some(by_run), None))
        }
        Ending::DeliveredElsewhere => {
            for (typed, name, only) in [
                (ask.by_run.is_some(), "--by-run", "superseded"),
                (
                    ask.by_realm.is_some(),
                    "--by-realm",
                    "the realm --by-run is read in",
                ),
            ] {
                if typed {
                    return Err(refused(format!(
                        "classify --cause delivered-elsewhere takes no {name}; {name} belongs \
                         to '{only}'"
                    )));
                }
            }
            let Some(by_ref) = ask.by_ref else {
                return Err(refused(
                    "classify --cause delivered-elsewhere needs --by-ref: the commit or \
                     pull request it landed by"
                        .to_string(),
                ));
            };
            Ok((None, Some(by_ref)))
        }
        Ending::Infrastructure
        | Ending::Exhausted
        | Ending::Withdrawn
        | Ending::OperatorStopped => {
            for (typed, name, only) in [
                (ask.by_run.is_some(), "--by-run", "superseded"),
                (ask.by_realm.is_some(), "--by-realm", "superseded"),
                (ask.by_ref.is_some(), "--by-ref", "delivered-elsewhere"),
            ] {
                if typed {
                    return Err(refused(format!(
                        "classify --cause {} takes no {name}; {name} belongs to '{only}'",
                        ending.word()
                    )));
                }
            }
            Ok((None, None))
        }
        Ending::Shipped
        | Ending::ShippedFlagged
        | Ending::Refused
        | Ending::Failed
        | Ending::Blocked
        | Ending::Unclassified => Err(refused(format!(
            "'{}' is not an operator-owned ending; an operator names one of {}",
            ending.word(),
            Ending::operator_words()
        ))),
    }
}

/// A `superseded` citation must resolve in the journal it names; it need
/// not be finished (ruling 4). A journal that cannot show it is refused,
/// never guessed at.
fn cited(by_journal: &Store, by_run: &str, by_realm: Option<&str>) -> Result<(), EngineError> {
    let realm = match by_realm {
        Some(realm) => format!("realm '{realm}'"),
        None => "the workspace journal".to_string(),
    };
    by_journal
        .load(by_run)
        .map(drop)
        .map_err(|error| refused(format!("run '{by_run}' in {realm}: {error}")))
}

#[cfg(test)]
#[path = "classify/tests.rs"]
mod tests;
