//! The one append-and-refusal fence a successful effect result passes
//! through (decision 0034, ruling 6), for a final result and an early
//! sequence ending alike.

use brokkr_core::envelope::EventType;
use brokkr_store::StoreError;
use serde_json::Value;

use super::super::{Engine, EngineError};

impl Engine {
    /// Append a successful result under the seat-record fence (decision
    /// 0034, ruling 6): a refused result becomes the attempt's
    /// `effect/failed`, never an engine error. The caller builds the
    /// payload, so an early ending can carry the step and why a final
    /// result does not.
    pub(crate) fn append_succeeded_under_fence(
        &mut self,
        payload: Value,
        attempt_id: &str,
        failed: impl FnOnce(String) -> Value,
    ) -> Result<(), EngineError> {
        match self.append(
            EventType::EffectSucceeded,
            payload,
            Some(attempt_id.to_string()),
        ) {
            Ok(_) => Ok(()),
            Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                self.append(
                    EventType::EffectFailed,
                    failed(refusal.to_string()),
                    Some(attempt_id.to_string()),
                )?;
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
}
