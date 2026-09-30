//! A sequence seat's steps, run one after another INSIDE one attempt
//! (decision 0002's serial form), one function per step form (#288).
//!
//! A step runs in its form — a single driver, a panel, or a realm-dialect
//! check — and is then settled in stages, each of which either lets the
//! attempt go on or journals its terminal event and ends it:
//! [`StepVerdict`] is that answer, matched at every stage.

use std::time::Duration;

use brokkr_core::envelope::EventType;
use brokkr_core::realms::Boundary;
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use brokkr_store::StoreError;
use serde_json::{json, Map, Value};

use super::{
    argv_for, copy_secret_binding_facts, dialect_attempt_outcome, expand_dialect_argv,
    failed_to_start, git_head, nearest_change, panel_outcome, stamp_boundary, start_failure_fields,
    start_failure_sites, stderr_tail, DriverRun, Engine, EngineError, Indeterminate, Selection,
    Site, Unproven,
};
use crate::bundle::{
    dialect_results, Aggregate, DialectExecution, PanelMember, SeatClass, SequenceStep, StepBody,
};

/// Where a step leaves its attempt, at each stage of settling it.
enum StepVerdict<T> {
    /// The attempt goes on, carrying `T` to the next stage or step.
    Goes(T),
    /// The attempt is over: its terminal event is journaled, so no later
    /// stage or step runs.
    Ended,
}

/// One sequence attempt's facts, the same for every step.
struct SequenceRun<'a> {
    effect_id: &'a str,
    attempt_id: &'a str,
    seat_name: &'a str,
    steps: &'a [SequenceStep],
    /// The requested-time seat input every step's driver input derives from.
    input: &'a Value,
    deadline: Duration,
    selection: &'a Selection,
    /// Whether the seat declares the `change` input, which a step's
    /// result then names for the steps after it.
    declares_change: bool,
}

/// What one step carries to the steps after it.
struct Carried {
    /// Earlier steps' result objects, by step name: `context.prior_results`.
    prior_results: Map<String, Value>,
    /// The change a dialect step's `{change}` expands to.
    nearest_change: Option<String>,
    /// A deterministic dialect check's failing step and result, which the
    /// final step may not contradict.
    deterministic_failure: Option<(String, String)>,
}

/// One step, placed in its sequence.
struct StepAt<'a> {
    index: usize,
    step: &'a SequenceStep,
    /// `<seat>:<step>`: the driver seat, and the label every site table
    /// keys the step on.
    label: String,
    /// The step's requested-time metadata: role, result and member paths.
    meta: &'a Value,
    /// The seat's context with the earlier steps' results beside it.
    context: Value,
}

/// A step form's outcome, and which of its sites failed to start, should
/// the step be the one that fails the attempt.
struct StepRun {
    outcome: AttemptOutcome,
    start_failures: Vec<Site>,
    /// What the step's terminal event carries when its tree is not
    /// proven over (#403).
    unproven: Unproven,
}

impl SequenceRun<'_> {
    fn is_last(&self, at: &StepAt) -> bool {
        at.index + 1 == self.steps.len()
    }

    fn phase(&self) -> &str {
        self.input["phase"].as_str().unwrap_or("")
    }

    /// The results a step may report: the seat's own for the final step,
    /// which is the seat's boundary, and the step's own for any other.
    fn allowed_results<'s>(&'s self, at: &StepAt<'s>) -> &'s Value {
        if self.is_last(at) {
            &self.input["allowed_results"]
        } else {
            &at.meta["allowed_results"]
        }
    }

    fn step_at<'s>(
        &'s self,
        index: usize,
        step: &'s SequenceStep,
        carried: &Carried,
    ) -> StepAt<'s> {
        let mut context = self.input["context"]
            .as_object()
            .cloned()
            .unwrap_or_default();
        context.insert(
            "prior_results".into(),
            Value::Object(carried.prior_results.clone()),
        );
        StepAt {
            index,
            step,
            label: format!("{}:{}", self.seat_name, step.name),
            meta: &self.input["steps"][index],
            context: Value::Object(context),
        }
    }

    /// A step driver's input: the fields a single step and a dialect step
    /// share, with the step's own role path.
    fn driver_input(&self, at: &StepAt, role_path: &Value) -> Value {
        json!({
            "feature": self.input["feature"],
            "phase": self.input["phase"],
            "seat": at.label,
            "role_path": role_path,
            "workdir": self.input["workdir"],
            "result_path": at.meta["result_path"],
            "allowed_results": self.allowed_results(at),
            "house_rules": self.input["house_rules"],
            "context": at.context,
        })
    }

    /// The `effect/failed` payload for a step: its problem, and the sites
    /// that failed to start.
    fn failed_payload(&self, at: &StepAt, problem: &str, start_failures: Vec<Site>) -> Value {
        let mut payload = json!({
            "effect_id": self.effect_id,
            "attempt_id": self.attempt_id,
            "error": format!("sequence step '{}': {problem}", at.step.name),
        });
        start_failure_fields(&mut payload, self.selection, start_failures);
        payload
    }

    /// Whether `result`, from a step before the last, names a result the
    /// seat declares and no remaining step can emit.
    fn ends_sequence(&self, at: &StepAt, result: &Value) -> bool {
        result
            .get("result")
            .and_then(Value::as_str)
            .is_some_and(|word| {
                let seat_declares = self.input["allowed_results"]
                    .as_array()
                    .is_some_and(|allowed| allowed.iter().any(|value| value == word));
                let later_can_emit = self.steps[at.index + 1..]
                    .iter()
                    .any(|later| later.results.iter().any(|allowed| allowed == word));
                seat_declares && !later_can_emit
            })
    }
}

impl Engine {
    /// Run named steps one after another INSIDE one attempt (decision
    /// 0002's serial form). Per-step driver inputs are derived
    /// deterministically from the requested-time seat input; earlier
    /// steps' result objects reach later steps as
    /// `context.prior_results`. Any step failing fails the WHOLE attempt
    /// (0006-retryable — a retry restarts from step 1); an indeterminate
    /// step parks it. The FINAL step's result object is the effect's
    /// single typed result — decide() validates it exactly as today.
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    pub(super) fn execute_sequence(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        seat_name: &str,
        steps: &[SequenceStep],
        seq_input: &Value,
        deadline: Duration,
        selection: &Selection,
    ) -> Result<(), EngineError> {
        let run = SequenceRun {
            effect_id,
            attempt_id,
            seat_name,
            steps,
            input: seq_input,
            deadline,
            selection,
            declares_change: self
                .bundle
                .seats
                .get(seat_name)
                .is_some_and(|seat| seat.inputs.iter().any(|input| input == "change")),
        };
        let mut carried = Carried {
            prior_results: Map::new(),
            nearest_change: nearest_change(&seq_input["context"]),
            deterministic_failure: None,
        };
        for (index, step) in steps.iter().enumerate() {
            let at = run.step_at(index, step, &carried);
            match self.sequence_step(&run, &at, &mut carried)? {
                StepVerdict::Goes(()) => {}
                StepVerdict::Ended => break,
            }
        }
        Ok(())
    }

    /// Run one step in its form, close its gate span, and settle it.
    fn sequence_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        carried: &mut Carried,
    ) -> Result<StepVerdict<()>, EngineError> {
        // The gate span inside a sequence is THIS step: armed here,
        // compared and cleared at this step's own end below, before
        // any later step gets to move the tree lawfully (decision
        // 0042 reads an author as a work step). Nothing outer arms
        // for a sequence, so this is the only observation taken.
        if at.step.class == SeatClass::Gate {
            self.active_gate_head = Some(git_head(&self.repo));
        }
        let ran = match &at.step.body {
            StepBody::Single { command, .. } => self.single_step(run, at, command)?,
            StepBody::Panel { members, aggregate } => {
                self.panel_step(run, at, members, *aggregate)?
            }
            StepBody::Dialect { execution } => {
                match self.dialect_step(run, at, execution, carried.nearest_change.as_deref())? {
                    StepVerdict::Goes(ran) => ran,
                    StepVerdict::Ended => return Ok(StepVerdict::Ended),
                }
            }
        };
        if self
            .finish_gate_head_check(run.effect_id, Some(run.attempt_id.to_string()))?
            .is_some()
        {
            return Ok(StepVerdict::Ended);
        }
        self.settle_step(run, at, ran, carried)
    }

    /// A single-driver step: the step's own driver, under its own site.
    fn single_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        command: &[String],
    ) -> Result<StepRun, EngineError> {
        let site = Some(at.step.name.clone());
        let mut input = run.driver_input(at, &at.meta["role_path"]);
        if !run.input["spec_dialect"].is_null() {
            input["spec_dialect"] = run.input["spec_dialect"].clone();
        }
        copy_secret_binding_facts(&mut input, run.input);
        let gate = at.step.class == SeatClass::Gate;
        let link = run.selection.get(&site);
        self.marks().site(&at.label, gate, link, &mut input);
        let hands = self.hands_for(&at.label);
        let mut spawn = self.compose_at(
            Some(&at.label),
            run.attempt_id,
            gate,
            argv_for(run.selection, &site, command).to_vec(),
            hands.as_ref(),
            link,
            input["result_path"].as_str().unwrap_or_default(),
        );
        self.mark_capabilities(&at.label, link, Some(&mut spawn), &mut input);
        // A sequence step now HAS such an identity: proposed
        // decision 0056 ruling 1 gives it a structural site
        // key, so a work-class step rejoins its own session
        // and a gate-class step never does. Under decision
        // 0030 alone there was nothing to match a step's
        // session by, and every step started cold.
        let driven = self.run_driver(
            run.effect_id,
            run.attempt_id,
            &at.label,
            &spawn,
            input,
            run.deadline,
            Some(&at.step.name),
            run.selection.plan(&site),
        )?;
        Ok(match driven {
            DriverRun::SpawnFailed(error) => StepRun {
                outcome: AttemptOutcome::Failed { error },
                start_failures: vec![site],
                unproven: Unproven::Proven,
            },
            DriverRun::Ran(report) => StepRun {
                start_failures: if failed_to_start(&report) {
                    vec![site]
                } else {
                    Vec::new()
                },
                unproven: Unproven::seat(&report),
                outcome: tailed_outcome(&report),
            },
        })
    }

    /// A panel step: its members run as a seat-level panel's do, tagged
    /// `<step>:<member>`, and joined by the step's declared aggregate.
    fn panel_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        members: &[PanelMember],
        aggregate: Aggregate,
    ) -> Result<StepRun, EngineError> {
        let tag_prefix = format!("{}:", at.step.name);
        let mut step_input = run.input.clone();
        step_input["allowed_results"] = run.allowed_results(at).clone();
        let runs = self.member_runs(
            run.attempt_id,
            &at.label,
            members,
            &at.meta["members"],
            &step_input,
            &at.context,
            run.selection,
            &tag_prefix,
            at.step.class == SeatClass::Gate,
        );
        let (effect_id, attempt_id) = (run.effect_id, run.attempt_id);
        let reports = self.run_panel(effect_id, attempt_id, &runs, run.deadline, &tag_prefix)?;
        self.journal_panel_members(effect_id, attempt_id, &reports, &runs, &tag_prefix)?;
        Ok(StepRun {
            start_failures: start_failure_sites(&reports, &tag_prefix),
            unproven: Unproven::panel(&reports),
            outcome: panel_outcome(aggregate, reports),
        })
    }

    /// A realm-dialect step: the engine's own `driver exec` runs the
    /// dialect's argv, with `{change}` expanded to the nearest change. A
    /// step that needs a change no earlier result carries parks.
    fn dialect_step(
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
        let command = std::iter::once(
            std::env::current_exe()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        )
        .chain(["driver", "exec", "--"].into_iter().map(str::to_string))
        .chain(expand_dialect_argv(&execution.argv, change))
        .collect::<Vec<_>>();
        let site = Some(at.step.name.clone());
        let mut input = run.driver_input(at, &json!(""));
        input["dialect_exec"] = json!({
            "success_result": dialect_results(run.phase())[0],
            "failure_result": dialect_results(run.phase())[1],
            "state": execution.state.as_ref().map(|state| expand_dialect_argv(state, change)),
            "change": if change.is_empty() { Value::Null } else { Value::String(change.to_string()) },
        });
        copy_secret_binding_facts(&mut input, run.input);
        self.mark_hands(&at.label, &mut input);
        // A dialect step composes under a boxed boundary only:
        // the compiler refuses it under `harness` and `open`
        // (design DD8), so no unboxed arm is reached here.
        let hands = self.hands_for(&at.label);
        let mut spawn = self.compose_at(
            Some(&at.label),
            run.attempt_id,
            true,
            argv_for(run.selection, &site, &command).to_vec(),
            hands.as_ref(),
            run.selection.get(&site),
            input["result_path"].as_str().unwrap_or_default(),
        );
        // The generated validator holds nothing, and says so
        // (decision 0065; design D5).
        self.mark_capabilities(&at.label, None, Some(&mut spawn), &mut input);
        let driven = self.run_driver(
            run.effect_id,
            run.attempt_id,
            &at.label,
            &spawn,
            input,
            run.deadline,
            Some(&at.step.name),
            None,
        )?;
        let mut unproven = Unproven::Proven;
        Ok(StepVerdict::Goes(StepRun {
            outcome: dialect_attempt_outcome(driven, &mut unproven),
            start_failures: Vec::new(),
            unproven,
        }))
    }

    /// Settle a step that ran: its outcome, the checks a step before the
    /// last answers, the change it names, and then the attempt's success
    /// (the last step) or the step's checkpoint (any other).
    fn settle_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        ran: StepRun,
        carried: &mut Carried,
    ) -> Result<StepVerdict<()>, EngineError> {
        // A step's result that names a model carries the step's
        // boundary beside it (design DD19); a panel step's aggregate
        // names none and carries none.
        let boundary = self.site_boundary(&at.label);
        let StepVerdict::Goes(result) = self.step_result(
            run,
            at,
            ran.outcome,
            ran.unproven,
            &ran.start_failures,
            boundary,
        )?
        else {
            return Ok(StepVerdict::Ended);
        };
        if !run.is_last(at) {
            let StepVerdict::Goes(()) = self.between_steps(run, at, &result, carried)? else {
                return Ok(StepVerdict::Ended);
            };
        }
        let StepVerdict::Goes(()) = self.carry_change(run, at, &result, carried)? else {
            return Ok(StepVerdict::Ended);
        };
        // A step's result the fence refuses (decision 0034, ruling
        // 6) fails the attempt at that step, with the same facts a
        // step that failed on its own would carry.
        let refused =
            |refusal: String| run.failed_payload(at, &refusal, ran.start_failures.clone());
        if run.is_last(at) {
            self.finish_sequence(run, at, result, carried, refused)
        } else {
            self.checkpoint_step(run, at, result, boundary, carried, refused)
        }
    }

    /// A step's outcome: a success's result, stamped with the step's
    /// boundary, goes on; a failure or an indeterminate ends the attempt,
    /// an indeterminate with what of the step is not proven over.
    fn step_result(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        outcome: AttemptOutcome,
        unproven: Unproven,
        start_failures: &[Site],
        boundary: Option<Boundary>,
    ) -> Result<StepVerdict<Value>, EngineError> {
        match outcome {
            AttemptOutcome::Succeeded { result } => {
                Ok(StepVerdict::Goes(stamp_boundary(result, boundary)))
            }
            AttemptOutcome::Failed { error } => {
                self.end_failed(run, run.failed_payload(at, &error, start_failures.to_vec()))
            }
            AttemptOutcome::Indeterminate { reason } => {
                self.end_indeterminate(run, at, &reason, unproven)
            }
        }
    }

    /// The checks a step before the last answers: a result in its own
    /// vocabulary, which may end the sequence, and which a deterministic
    /// dialect check's failure is remembered by.
    fn between_steps(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: &Value,
        carried: &mut Carried,
    ) -> Result<StepVerdict<()>, EngineError> {
        if let Some(problem) = vocabulary_problem(at.step, result) {
            return self.end_failed(run, run.failed_payload(at, &problem, Vec::new()));
        }
        // A seat result which no remaining compiled step can emit is
        // a declared sequence-ending boundary. The comparison is
        // against the remaining steps' actual vocabularies, not the
        // enclosing seat vocabulary inherited by the old final-step
        // representation.
        if run.ends_sequence(at, result) {
            return self
                .append(
                    EventType::EffectSucceeded,
                    json!({"effect_id": run.effect_id, "attempt_id": run.attempt_id, "result": result}),
                    Some(run.attempt_id.to_string()),
                )
                .map(|_| StepVerdict::Ended);
        }
        let phase = run.phase();
        if matches!(at.step.body, StepBody::Dialect { .. })
            && matches!(phase, "clarify" | "analyze")
            && result["result"] == dialect_results(phase)[1]
        {
            carried.deterministic_failure = Some((
                at.step.name.clone(),
                result["result"].as_str().unwrap().to_string(),
            ));
        }
        Ok(StepVerdict::Goes(()))
    }

    /// The change a step's result names, when the seat declares one: an
    /// identifier becomes the nearest change, and anything else parks.
    fn carry_change(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: &Value,
        carried: &mut Carried,
    ) -> Result<StepVerdict<()>, EngineError> {
        if !run.declares_change {
            return Ok(StepVerdict::Goes(()));
        }
        let Some(value) = result.pointer("/inputs/change") else {
            return Ok(StepVerdict::Goes(()));
        };
        let change = value.as_str();
        if !change.is_some_and(brokkr_core::policy::is_identifier) {
            return self.end_indeterminate(
                run,
                at,
                &format!("declared input 'change' must match ^[a-z0-9][a-z0-9._-]*$, got {value}"),
                Unproven::Proven,
            );
        }
        carried.nearest_change = change.map(str::to_string);
        Ok(StepVerdict::Goes(()))
    }

    /// The last step's result is the attempt's, unless it contradicts a
    /// deterministic dialect check's failure, which parks.
    fn finish_sequence(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: Value,
        carried: &Carried,
        refused: impl FnOnce(String) -> Value,
    ) -> Result<StepVerdict<()>, EngineError> {
        if let Some((check, failure)) = &carried.deterministic_failure {
            if result["result"] == dialect_results(run.phase())[0] {
                return self.end_indeterminate(
                    run,
                    at,
                    &format!(
                        "result '{}' contradicts deterministic step '{check}' result '{failure}'",
                        result["result"]
                    ),
                    Unproven::Proven,
                );
            }
        }
        self.append_succeeded(run.effect_id, run.attempt_id, result, refused)?;
        Ok(StepVerdict::Ended)
    }

    /// A step before the last journals its result as a checkpoint, which
    /// the later steps then read; a checkpoint the seat record refuses
    /// fails the attempt.
    fn checkpoint_step(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: Value,
        boundary: Option<Boundary>,
        carried: &mut Carried,
        refused: impl FnOnce(String) -> Value,
    ) -> Result<StepVerdict<()>, EngineError> {
        let model = result
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("not reported")
            .to_string();
        let marker = stamp_boundary(
            json!({
                "step": "sequence-step-finished",
                "step_name": at.step.name,
                "model": model,
                "result": result,
            }),
            boundary,
        );
        match self.append(
            EventType::EffectCheckpointed,
            json!({
                "effect_id": run.effect_id,
                "attempt_id": run.attempt_id,
                "checkpoint": marker,
            }),
            Some(run.attempt_id.to_string()),
        ) {
            Ok(_) => {}
            Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                return self.end_failed(run, refused(refusal.to_string()));
            }
            Err(error) => return Err(error),
        }
        carried.prior_results.insert(at.step.name.clone(), result);
        Ok(StepVerdict::Goes(()))
    }

    /// Journal the attempt's `effect/failed` and end it.
    fn end_failed<T>(
        &mut self,
        run: &SequenceRun,
        payload: Value,
    ) -> Result<StepVerdict<T>, EngineError> {
        self.append(
            EventType::EffectFailed,
            payload,
            Some(run.attempt_id.to_string()),
        )
        .map(|_| StepVerdict::Ended)
    }

    /// Journal the attempt's `effect/indeterminate`, naming the step and
    /// carrying what of it is not proven over (#403), and end it.
    fn end_indeterminate<T>(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        reason: &str,
        unproven: Unproven,
    ) -> Result<StepVerdict<T>, EngineError> {
        self.append(
            EventType::EffectIndeterminate,
            json!(Indeterminate {
                effect_id: run.effect_id,
                attempt_id: run.attempt_id,
                reason: format!("sequence step '{}': {reason}", at.step.name),
                unproven,
            }),
            Some(run.attempt_id.to_string()),
        )
        .map(|_| StepVerdict::Ended)
    }
}

/// A step driver's outcome, its failure or indeterminate reason with the
/// driver's stderr tail beside it: the tail rides on the attempt's
/// terminal event, as a single seat's does. It reads the settled outcome
/// (#403): a step whose tree is not proven over is indeterminate.
fn tailed_outcome(report: &AttemptReport) -> AttemptOutcome {
    let tail = stderr_tail(&report.stderr);
    match report.settled_outcome() {
        AttemptOutcome::Succeeded { result } => AttemptOutcome::Succeeded { result },
        AttemptOutcome::Failed { error } => AttemptOutcome::Failed {
            error: format!("{error}; stderr tail: {tail}"),
        },
        AttemptOutcome::Indeterminate { reason } => AttemptOutcome::Indeterminate {
            reason: format!("{reason}; stderr tail: {tail}"),
        },
    }
}

/// Whether a dialect step's argv or state names `{change}`.
fn needs_change(execution: &DialectExecution) -> bool {
    execution
        .argv
        .iter()
        .chain(execution.state.iter().flatten())
        .any(|token| token.contains("{change}"))
}

/// What is wrong with a step's result against its own vocabulary, if
/// anything.
fn vocabulary_problem(step: &SequenceStep, result: &Value) -> Option<String> {
    match result.get("result").and_then(Value::as_str) {
        None => Some("has no 'result' string".to_string()),
        Some(word) if !step.results.iter().any(|allowed| allowed == word) => Some(format!(
            "reported '{word}', outside its declared results {:?}",
            step.results
        )),
        Some(_) => None,
    }
}
