//! The one refusal a result that contradicts a deterministic dialect
//! check's failure meets. An early ending and the final step both pass
//! through it, so ending a sequence sooner cannot judge the failure away
//! (decision 0050, ruling 7).

use serde_json::Value;

use super::{Carried, Engine, EngineError, SequenceRun, StepAt, StepVerdict, Unproven};
use crate::bundle::dialect_results;

impl Engine {
    /// Refuse `result` when it contradicts a deterministic dialect check's
    /// failure: the same check and the same text for the early ending and
    /// the final step, so neither path can end without it.
    pub(super) fn refuse_contradiction(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: &Value,
        carried: &Carried,
    ) -> Result<StepVerdict<()>, EngineError> {
        let Some((check, failure)) = &carried.deterministic_failure else {
            return Ok(StepVerdict::Goes(()));
        };
        if result["result"] != dialect_results(run.phase())[0] {
            return Ok(StepVerdict::Goes(()));
        }
        self.end_indeterminate(
            run,
            at,
            &format!(
                "result '{}' contradicts deterministic step '{check}' result '{failure}'",
                result["result"]
            ),
            Unproven::Proven,
        )
    }
}
