//! What a verdict is derived from (decision 0089 ruling 2): the facts a
//! run's fold states, or its summary row states as the fold does.

use brokkr_core::fold::{RunState, Status};
use brokkr_core::park::CauseRead;

use super::{Standing, Waits, STOP_PHASE};
use crate::ruled;

/// What a verdict is derived from, whichever read holds the run: its fold
/// read whole, or its summary row, which states these as the fold does
/// (decision 0089 ruling 2), so the two derive one verdict.
#[derive(Clone, Copy)]
pub(crate) struct Facts<'a> {
    pub(crate) status: Status,
    pub(crate) phase: Option<&'a str>,
    /// The last ruling that names its rule, present or null: the rule id
    /// where present, the phase that ruled, and whether it routed on.
    pub(crate) ruling: Option<(Option<&'a str>, &'a str, bool)>,
    pub(crate) park_reason: Option<&'a str>,
    /// The park's typed cause, or `None` for a row that read none.
    pub(crate) park_cause: Option<&'a CauseRead>,
    /// The seq an operator stop was accepted at, while no ending closed it.
    pub(crate) stop_accepted_seq: Option<u64>,
}

impl<'a> Facts<'a> {
    pub(crate) fn of(state: &'a RunState) -> Facts<'a> {
        Facts::of_cause(state, None)
    }

    pub(crate) fn of_cause(state: &'a RunState, park_cause: Option<&'a CauseRead>) -> Facts<'a> {
        Facts {
            status: state.run.status(),
            phase: state.phase.as_deref(),
            ruling: ruled(state.last_decision.as_ref()).map(|decision| {
                let routes = decision.next.present().is_some();
                let rule = decision.rule_id.present().map(String::as_str);
                (rule, decision.from.as_str(), routes)
            }),
            park_reason: state.run.park_reason().map(|reason| reason.as_str()),
            park_cause: state.run.park_reason().and(park_cause),
            stop_accepted_seq: state.stopping_seq(),
        }
    }

    pub(crate) fn standing(self, waits: &Waits) -> Standing {
        match self.status {
            Status::Running if self.stop_accepted_seq.is_some() => Standing::Stopping,
            Status::Running => waits.standing(),
            Status::AwaitingOperator => Standing::Parked,
            Status::Completed => Standing::Shipped,
            Status::Stopped if self.phase == Some(STOP_PHASE) => Standing::Stopped,
            Status::Stopped => Standing::OperatorStopped,
        }
    }

    /// The last ruling's rule id and the phase that ruled, when it names a
    /// rule at all.
    pub(super) fn last_rule(self) -> Option<(&'a str, Option<&'a str>)> {
        let (rule, from, _) = self.ruling?;
        Some((rule?, Some(from)))
    }

    /// The last ruling routed nowhere, so it is what parked the run — the
    /// fold's own test for a rule-driven park. A park on an exhausted
    /// attempt limit or a lost attempt follows a ruling that routed on.
    pub(super) fn ruling_parked(self) -> bool {
        self.ruling.is_some_and(|(_, _, routes)| !routes)
    }
}
