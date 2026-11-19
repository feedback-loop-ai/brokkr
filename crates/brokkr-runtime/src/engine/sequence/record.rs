//! A sequence step's engine-authored record: the step's stamped result
//! beside the engine's `sequence-step-finished` marker, journaled for
//! every step before the last and for a gate step that ends the sequence
//! (decision 0109 ruling 7, fix round 0109a1r5, V1).

use brokkr_core::envelope::EventType;
use brokkr_core::findings::EngineMarker;
use brokkr_core::realms::Boundary;
use brokkr_store::StoreError;
use serde_json::{json, Value};

use super::super::{stamp_boundary, Engine, EngineError};
use super::{SequenceRun, StepAt};
use crate::bundle::SeatClass;

impl Engine {
    /// A gate step that ends the sequence, at its end or early, journals
    /// its own step record before the attempt's success, as a step before
    /// the last does, so the observation its span accepted is on the
    /// record and decide never infers it from the terminal. A work step
    /// that ends it journals none: it took no observation. `Err` is the
    /// seat record's refusal of the record.
    pub(super) fn record_ending_gate_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: &Value,
    ) -> Result<Result<(), String>, EngineError> {
        if at.step.class != SeatClass::Gate {
            return Ok(Ok(()));
        }
        let result = self.stamp_step_covered(run.effect_id, result.clone());
        let boundary = self.site_boundary(&at.label);
        self.journal_step_record(run, at, &result, boundary)
    }

    /// Journal `result`, already stamped, as `at`'s engine-authored step
    /// record; `Err` is the seat record's refusal of it.
    pub(super) fn journal_step_record(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: &Value,
        boundary: Option<Boundary>,
    ) -> Result<Result<(), String>, EngineError> {
        let model = result
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("not reported")
            .to_string();
        let marker = stamp_boundary(
            EngineMarker::StepFinished.record(json!({
                "step_name": at.step.name,
                "member": at.step.name,
                "model": model,
                "result": result,
            })),
            boundary,
        );
        let payload = json!({
            "effect_id": run.effect_id,
            "attempt_id": run.attempt_id,
            "checkpoint": marker,
        });
        let attempt_id = Some(run.attempt_id.to_string());
        match self.append(EventType::EffectCheckpointed, payload, attempt_id) {
            Ok(_) => Ok(Ok(())),
            Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                Ok(Err(refusal.to_string()))
            }
            Err(error) => Err(error),
        }
    }
}
