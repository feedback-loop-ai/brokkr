//! Member retry admission over the deciding fold (0093 rulings 1 and 2).
use super::{
    acceptance_refusal, decided, disposed, Commanded, EngineError, EventEnvelope,
    FencedCommandOutcome, Head, OperatorCommand, RunState, Store,
};
use crate::engine::carry::{
    carry_plan,
    facts::{read_visit, PanelAttempt},
    record::{RetryArgs, Tag},
    CarryRefusal,
};
use crate::engine::Engine;

/// A refusal writes nothing; a disposition was journaled under the operator fence.
#[derive(Debug)]
pub enum MemberRetryOutcome {
    FoldRefused(brokkr_core::fold::Refusal),
    CarryRefused(CarryRefusal),
    Disposed(FencedCommandOutcome),
}

pub(crate) struct RetryRead {
    deciding: (RunState, Head),
    events: Vec<EventEnvelope>,
    attempts: Vec<PanelAttempt>,
}

impl Engine {
    pub(crate) fn read_member_retry(
        store: &mut Store,
        run: &str,
    ) -> Result<Result<RetryRead, MemberRetryOutcome>, EngineError> {
        let events = store.load(run)?;
        let deciding = decided(store, run, &events)?;
        if let Some(refusal) = acceptance_refusal(&deciding.0, OperatorCommand::Retry) {
            return Ok(Err(MemberRetryOutcome::FoldRefused(refusal)));
        }
        let attempts = match read_visit(&events, &deciding.0, u64::MAX) {
            Ok(attempts) => attempts,
            Err(refusal) => return Ok(Err(MemberRetryOutcome::CarryRefused(refusal))),
        };
        Ok(Ok(RetryRead {
            deciding,
            events,
            attempts,
        }))
    }

    pub(crate) fn retry_members(
        &mut self,
        read: RetryRead,
        names: &[String],
        operator: &str,
        reason: &str,
    ) -> Result<MemberRetryOutcome, EngineError> {
        self.retry_members_racing(read, names, operator, reason, |_| {})
    }

    pub(crate) fn retry_members_racing(
        &mut self,
        read: RetryRead,
        names: &[String],
        operator: &str,
        reason: &str,
        mut before_command: impl FnMut(&mut Store),
    ) -> Result<MemberRetryOutcome, EngineError> {
        let facts = self.carry_facts(&read.events, &read.deciding.0, read.attempts);
        let names: Vec<_> = names.iter().cloned().map(Tag).collect();
        let plan = match carry_plan(&facts, &names) {
            Ok(plan) => plan,
            Err(refusal) => return Ok(MemberRetryOutcome::CarryRefused(refusal)),
        };
        let args = serde_json::to_value(RetryArgs {
            members: Some(plan.fresh()),
        })
        .expect("retry args serialize");
        let request = Commanded {
            command: OperatorCommand::Retry,
            operator,
            reason,
            args,
        };
        before_command(&mut self.store);
        // One line, as `disposed`'s own refusal callsites: the exact-coverage
        // gate reads a multi-line call's `?` edge as its own region.
        let (store, run_id) = (&mut self.store, &self.run_id);
        let disposition = disposed(store, run_id, request, read.deciding, |_| {})?;
        Ok(MemberRetryOutcome::Disposed(disposition.outcome()))
    }
}

#[cfg(test)]
mod tests;
