//! Why `conclude` refused a run, and the words each refusal prints; and
//! the reason an accepted operator stop concludes a run with, moved out of
//! `operator.rs`, which `release` would take past its 800 lines (decision
//! 0065 slice two, U8e3; operator ruling of 2026-10-11).

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{concluded_text, OperatorCommand, Refusal, Status};
use brokkr_core::EventEnvelope;
use serde_json::Value;

use super::{command_id_of, EngineError, OPERATOR_STOP_RULE};

/// The conclusion an accepted operator stop is journaled with: the rule
/// id [`OPERATOR_STOP_RULE`], the command it names, and the operator who
/// gave it with the reason they recorded. `run/stopped`'s v1 payload is
/// closed at `{reason}` (`contracts/README.md`, additionalProperties
/// false), so the citation lives INSIDE the reason string — no new field,
/// no second vocabulary — exactly as `request_or_finish`'s policy-driven
/// hard stop cites its `rule_id` there. The cause is read back from the
/// journal that `operator_command` wrote: `fold` spends the pending
/// command when it accepts it, so the events are the only place it
/// survives.
pub(in crate::engine) fn operator_stop_reason(events: &[EventEnvelope]) -> String {
    let accepted = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::OperatorAccepted);
    let accepted_field = |field: &str| {
        accepted
            .and_then(|event| event.payload.get(field))
            .and_then(Value::as_str)
    };
    // The acceptance carries only the command's id; the command itself
    // is on the `operator/commanded` it disposes of.
    let command_id = accepted_field("command_id").unwrap_or("unrecorded");
    let command = events
        .iter()
        .find(|event| {
            event.event_type == EventType::OperatorCommanded
                && command_id_of(event) == Some(command_id)
        })
        .and_then(|event| event.payload.get("command"))
        .and_then(Value::as_str)
        .unwrap_or(OperatorCommand::Stop.as_str());
    format!(
        "{OPERATOR_STOP_RULE}: operator '{}' commanded {command} ({command_id}): {}",
        accepted_field("operator").unwrap_or("unrecorded"),
        accepted_field("reason").unwrap_or("no reason recorded"),
    )
}

/// Why `conclude` refused a run: each is the operator's to act on, and
/// none wrote a conclusion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConcludeRefusal {
    /// The run already has its conclusion. A second `run/stopped` would
    /// fail the fold as an event after terminal, so this comes before the
    /// first append, not after a half-written closure.
    AlreadyConcluded { status: Status },
    /// The stop `conclude` commanded was refused, with the fold's reason.
    StopRefused { refusal: Refusal },
    /// A fenced write met a head that moved beneath the conclusion.
    JournalMoved,
}

impl ConcludeRefusal {
    /// The words each refusal has always printed, the run named in them.
    /// The two that were once `engine:` errors keep that prefix.
    pub(in crate::engine) fn text(&self, run_id: &str) -> String {
        match self {
            ConcludeRefusal::AlreadyConcluded { status } => concluded_text(run_id, *status),
            ConcludeRefusal::StopRefused { refusal } => format!(
                "engine: conclude: run '{run_id}' refused the stop ({}); the journal \
                 moved beneath the conclusion, so something may still be driving this run — \
                 look with `brokkr runs` before closing",
                refusal.word()
            ),
            ConcludeRefusal::JournalMoved => format!(
                "engine: conclude: the journal moved beneath the conclusion of run '{run_id}', \
                 so something may still be driving it — a conclusion is for a run believed \
                 dead; look with `brokkr runs` before closing"
            ),
        }
    }

    pub(super) fn of(self, run_id: &str) -> EngineError {
        EngineError::ConcludeRefused {
            run_id: run_id.to_string(),
            why: self,
        }
    }
}
