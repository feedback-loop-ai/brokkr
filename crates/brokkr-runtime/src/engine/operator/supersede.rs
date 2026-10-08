//! `supersede`: the operator annotation that closes a terminal run's
//! residual findings (decision 0047). It shares the fence and the
//! read-back with the other verbs, and nothing else: it is no command
//! with a disposition, and no `operator/accepted` ever follows it.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Status};
use brokkr_core::residual::{residuals, SUPERSEDE};
use brokkr_core::EventEnvelope;
use brokkr_store::Store;
use serde_json::json;
use uuid::Uuid;

use super::{fenced_append, read_back, EngineError, Head};

/// What `brokkr operator supersede` says, as the operator typed it
/// (decision 0047 ruling 1): which of this run's residual findings are
/// closed, the run and ruling that closed them, and why.
pub struct Supersede<'a> {
    /// The sequence numbers of the rulings whose residuals are closed.
    pub findings: &'a [u64],
    /// The realm the superseding run was read in, or `None` for the
    /// workspace journal — decision 0026 ruling 3's key, on a citation.
    pub by_realm: Option<&'a str>,
    pub by_run: &'a str,
    pub by_seq: u64,
    pub reason: &'a str,
    pub operator: &'a str,
}

/// Record that a terminal run's residual findings are closed by another
/// run (decision 0047 ruling 1). One `operator/commanded` event with
/// `command: "supersede"`, and nothing else: no `operator/accepted`
/// follows, because there is nothing to execute and `fold` refuses an
/// acceptance on a terminal run by design. The event is the record.
///
/// Every citation is verified BEFORE anything is written (ruling 2), and
/// a refusal writes nothing at all — which is why this returns
/// [`EngineError::SupersedeRefused`] rather than journaling a rejection
/// the way `retry` and `stop` do. There is no pending command here for a
/// rejection to dispose of.
///
/// `by_journal` is the journal `by_realm` names, opened by the caller
/// (the workspace journal when the citation names no realm): a
/// superseding run may live in another hearth, and a citation into a
/// journal this process never opened is one nobody can follow back.
///
/// The write is FENCED on the head the verification read (decision
/// 0029). The event is legal wherever it lands — a terminal status is
/// absorbing, so `fold` exempts this annotation at any later seq — but
/// a head that moved is a journal that grew under a verification, and
/// re-deciding beats writing against a reading that is no longer the
/// whole story.
pub fn operator_supersede(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    ask: &Supersede,
) -> Result<EventEnvelope, EngineError> {
    operator_supersede_racing(store, by_journal, run_id, ask, |_| {})
}

/// [`operator_supersede`], with the window its fence exists for made
/// reachable. `between` runs after the verifying load, before the
/// fenced append. Production passes a no-op; the test passes a peer's
/// append, because a fence is either always there or it is not a fence.
pub(in crate::engine) fn operator_supersede_racing(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    ask: &Supersede,
    mut between: impl FnMut(&mut Store),
) -> Result<EventEnvelope, EngineError> {
    // One voice for every refusal below, and every one of them comes
    // before the first append: a refused supersede leaves the journal
    // byte for byte as it found it.
    let refused = EngineError::SupersedeRefused;
    let events = store.load(run_id)?;
    let state = fold(&events)?;
    if !matches!(state.status, Status::Completed | Status::Stopped) {
        return Err(refused(format!(
            "run '{run_id}' is {}, not completed or stopped; on a run that is still \
             going the fold would hold this as a PENDING command and the \
             operator/accepted that disposes of it would be refused as unknown",
            state.status.as_str()
        )));
    }
    if ask.by_run == run_id {
        return Err(refused(format!(
            "run '{run_id}' cannot supersede its own findings; name the run that \
             closed them"
        )));
    }
    if ask.findings.is_empty() {
        return Err(refused(format!(
            "a supersede on run '{run_id}' that names no finding closes nothing; \
             name the seq of every ruling whose residual is closed"
        )));
    }
    let derived = residuals(&events);
    for finding in ask.findings {
        if !derived.iter().any(|known| known.ruling.seq == *finding) {
            return Err(refused(format!(
                "seq {finding} is not a residual finding of run '{run_id}'; \
                 brokkr inspect --run {run_id} lists the rulings that carry one"
            )));
        }
    }
    let realm = match ask.by_realm {
        Some(realm) => format!("realm '{realm}'"),
        None => "the workspace journal".to_string(),
    };
    let cited = by_journal
        .load(ask.by_run)
        .map_err(|error| refused(format!("run '{}' in {realm}: {error}", ask.by_run)))?;
    if !cited
        .iter()
        .any(|event| event.seq == ask.by_seq && event.event_type == EventType::TransitionDecided)
    {
        return Err(refused(format!(
            "seq {} of run '{}' in {realm} is not a transition/decided; a supersede \
             cites the RULING that closed the finding",
            ask.by_seq, ask.by_run
        )));
    }
    let head = Head::tip(&events);
    let payload = json!({
        "command_id": Uuid::new_v4().to_string(),
        "command": SUPERSEDE,
        "operator": ask.operator,
        "args": {
            "findings": ask.findings,
            "by": {"realm": ask.by_realm, "run_id": ask.by_run, "seq": ask.by_seq},
            "reason": ask.reason,
        },
    });
    between(store);
    // A head that moved comes back in the store's own words — `head
    // moved: expected seq N, found seq M` — rather than paraphrased
    // into a refusal of this verb's own. Nothing was written either
    // way, and the operator reads the journal again and asks again.
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
    // The annotation must read back, and reading back must change
    // nothing (ruling 3). Same proof the fenced command path takes.
    read_back(store, run_id)?;
    Ok(written)
}
