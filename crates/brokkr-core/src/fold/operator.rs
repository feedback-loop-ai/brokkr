//! How the fold reads an operator's command and its acceptance, moved out
//! of `fold.rs`, which `release` would take past its 800 lines (decision
//! 0065 slice two, U8e3; operator ruling of 2026-10-11).

use super::{
    acceptance_refusal, out_of_place, Cursor, EventEnvelope, FoldError, OperatorCommand, Run,
    RunState,
};
use crate::payload;

pub(super) fn command(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let commanded = payload::OperatorCommanded::read(event)?;
    let (command_id, command) = (commanded.command_id, commanded.command);
    state.pending_command = Some((command_id.to_string(), command.to_string()));
    Ok(())
}

pub(super) fn accept(state: &mut RunState, event: &EventEnvelope) -> Result<(), FoldError> {
    let command_id = payload::OperatorAccepted::read(event)?.command_id;
    let Some((pending_id, command)) = state.pending_command.take() else {
        return Err(FoldError::NoMatchingCommand { seq: event.seq });
    };
    if pending_id != command_id {
        return Err(FoldError::NoMatchingCommand { seq: event.seq });
    }
    let Some(command) = OperatorCommand::parse(&command) else {
        return Err(FoldError::UnknownCommand {
            seq: event.seq,
            command,
        });
    };
    // `acceptance_refusal`'s terminal answer never reaches here: the
    // relation refuses every event after a terminal first. What is left
    // is a retry with nothing to return to.
    if acceptance_refusal(state, command).is_some() {
        return Err(out_of_place(state, event));
    }
    match command {
        OperatorCommand::Retry => {
            state.stop_accepted_seq = None;
            state.run = Run::Running(Cursor::RequestEffect);
        }
        // `brokkr operator` is a live kill switch: it journals the
        // command AND its acceptance without reading the run's cursor
        // first, so a stop legitimately lands wherever the run stands.
        // It always reaches a conclusion; only WHEN differs. Mid-flight
        // it rides — the attempt is untouched, its checkpoints keep
        // applying, and the effect's own boundary (result, failure, or
        // indeterminate close) concludes the run per it, so decision
        // 0006's attempt bounds are never truncated. Anywhere else there
        // is no attempt to wait for, so the run concludes where it
        // stands: parked (the operator answering a park with "stop") and
        // running between effects are the same sentence, finished at the
        // first position the journal offers.
        OperatorCommand::Stop => {
            state.stop_accepted_seq = Some(event.seq);
            if let Run::Running(Cursor::EffectInFlight { stop_rides, .. }) = &mut state.run {
                *stop_rides = true;
            } else {
                state.run = Run::Running(Cursor::Stop);
            }
        }
        // The reservation's release is the engine's to effect; the run
        // and its ended attempts stand as they were.
        OperatorCommand::Release => {}
    }
    Ok(())
}
