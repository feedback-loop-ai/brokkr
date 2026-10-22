//! Fenced disposition, shared by the plain and member retry commands.
use super::*;

pub(super) struct Commanded<'a> {
    pub(super) command: OperatorCommand,
    pub(super) operator: &'a str,
    pub(super) reason: &'a str,
    pub(super) args: Value,
}

pub(super) enum Disposition {
    Accepted(FencedCommandOutcome),
    Refused(Refusal, FencedCommandOutcome),
}

impl Disposition {
    pub(super) fn outcome(self) -> FencedCommandOutcome {
        match self {
            Disposition::Accepted(outcome) | Disposition::Refused(_, outcome) => outcome,
        }
    }
}

pub(super) fn disposed(
    store: &mut Store,
    run_id: &str,
    request: Commanded<'_>,
    (asked, head): (RunState, Head),
    mut between: impl FnMut(&mut Store),
) -> Result<Disposition, EngineError> {
    let Commanded {
        command,
        operator,
        reason,
        args,
    } = request;
    let command_id = Uuid::new_v4().to_string();
    // A journal that does not fold cannot host a legal acceptance at
    // all, so refuse before writing anything rather than adding to it.
    // What the run already refused before the command was even journaled
    // is not a race, and is not reported as one.
    let illegal_when_asked = acceptance_refusal(&asked, command).is_some();
    let commanded = store.append_next(
        run_id,
        EventType::OperatorCommanded,
        json!({"command_id": command_id, "command": command.as_str(), "args": args, "operator": operator}),
        Some(head.event_id),
        None,
    )?;

    let mut lost = 0;
    // One line per refusal callsite: the exact-coverage gate reads a
    // multi-line call's `?` edges as their own regions, and a refusal
    // whose failure edge cannot be reached deterministically would read
    // as an uncovered line forever. Sharing the line makes the gate see
    // what is actually exercised.
    let refusal_of = |store: &mut Store, why: Refusal| {
        let (command_id, cause) = (&command_id, &commanded.event_id);
        let rejection = Rejection {
            command_id,
            operator,
            refusal: why,
            cause,
        };
        refuse(store, run_id, rejection).map(|outcome| Disposition::Refused(why, outcome))
    };
    let disposition = loop {
        let (state, head) = folded(store, run_id)?;
        between(store);
        if let Some(refusal) = standing_refusal(&state, command, &command_id, illegal_when_asked) {
            break refusal_of(store, refusal)?;
        }
        let payload = json!({"command_id": command_id, "operator": operator, "reason": reason});
        let cause = &commanded.event_id;
        match fenced_append(
            store,
            run_id,
            &head,
            cause,
            EventType::OperatorAccepted,
            payload,
            None,
        ) {
            Ok(_) => break Disposition::Accepted(accepted(store, run_id)?),
            // A peer appended between the fold and the write. The
            // acceptance was never written; decide again against what it
            // wrote, and refuse rather than spin forever.
            Err(StoreError::HeadMoved { .. }) if lost < FENCE_ATTEMPTS => lost += 1,
            Err(StoreError::HeadMoved { .. }) => break refusal_of(store, Refusal::LostFence)?,
            Err(error) => return Err(error.into()),
        }
    };
    // The pair that just landed must read back. Same proof the fenced
    // path takes before it acknowledges anything to Looper.
    read_back(store, run_id)?;
    Ok(disposition)
}
