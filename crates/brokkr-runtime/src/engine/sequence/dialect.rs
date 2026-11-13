//! A realm-dialect step: the engine's own `driver exec` runs the dialect's
//! argv, with `{change}` expanded to the nearest change. Moved here from
//! `sequence.rs` to keep that file within ruling 4's 800-line ceiling; it
//! now also reads the step's bound from `run.bound` and names the
//! deadline in its outcome, as every step form does (decision 0104 slice
//! 5a2).

use serde_json::{json, Value};

use super::super::{argv_for, Engine, EngineError};
use super::{
    dialect_attempt_outcome, expand_dialect_argv, needs_change, SequenceRun, StepAt, StepInputKind,
    StepRun, StepVerdict, Unproven,
};
use crate::bundle::{dialect_results, DialectExecution};

impl Engine {
    /// A realm-dialect step: the engine's own `driver exec` runs the
    /// dialect's argv, with `{change}` expanded to the nearest change. A
    /// step that needs a change no earlier result carries parks.
    pub(super) fn dialect_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        execution: &DialectExecution,
        nearest_change: Option<&str>,
    ) -> Result<StepVerdict<StepRun>, EngineError> {
        if needs_change(execution) && nearest_change.is_none() {
            return self.end_indeterminate(
                run,
                at,
                "cannot expand {change} because no preceding successful result carries it",
                Unproven::Proven,
            );
        }
        let change = nearest_change.unwrap_or("");
        let command = std::iter::once(self.bundle.executable.lossy())
            .chain(["driver", "exec", "--"].into_iter().map(str::to_string))
            .chain(expand_dialect_argv(&execution.argv, change))
            .collect::<Vec<_>>();
        let site = Some(at.step.name.clone());
        let mut input = run.driver_input(at, &json!(""), StepInputKind::Dialect);
        input["dialect_exec"] = json!({
            "success_result": dialect_results(run.phase())[0],
            "failure_result": dialect_results(run.phase())[1],
            "state": execution.state.as_ref().map(|state| expand_dialect_argv(state, change)),
            "change": if change.is_empty() { Value::Null } else { Value::String(change.to_string()) },
        });
        self.mark_hands(&at.label, &mut input);
        // A dialect step composes under a boxed boundary only:
        // the compiler refuses it under `harness` and `open`
        // (design DD8), so no unboxed arm is reached here.
        let hands = self.hands_for(&at.label);
        let mut spawn = self.compose_at(
            Some(&at.label),
            crate::engine::AttemptIds::new(run.effect_id, run.attempt_id),
            true,
            argv_for(run.selection, &site, &command).to_vec(),
            hands.as_ref(),
            run.selection.get(&site),
            input["result_path"].as_str().unwrap_or_default(),
        );
        // The generated validator holds nothing, and says so
        // (decision 0065; design D5).
        let brokers = run.selection.brokers(&at.label);
        let served = (brokers, hands.as_ref());
        self.mark_serving(&at.label, None, served, Some(&mut spawn), &mut input);
        let mut driven = self.run_driver(
            run.effect_id,
            run.attempt_id,
            &at.label,
            &spawn,
            input,
            run.bound.remaining,
            Some(&at.step.name),
            None,
        )?;
        run.bound.deadline.name(&mut driven);
        let mut unproven = Unproven::Proven;
        Ok(StepVerdict::Goes(StepRun {
            record: None,
            outcome: dialect_attempt_outcome(driven, &mut unproven),
            start_failures: Vec::new(),
            refused: Vec::new(),
            unproven,
        }))
    }
}
