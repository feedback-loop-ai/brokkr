//! A sequence seat's steps, run one after another INSIDE one attempt
//! (decision 0002's serial form), one function per step form (#288).
//!
//! A step runs in its form — a single driver, a panel, or a realm-dialect
//! check — and is then settled in stages, each of which either lets the
//! attempt go on or journals its terminal event and ends it:
//! [`StepVerdict`] is that answer, matched at every stage.

use brokkr_core::envelope::EventType;
use brokkr_core::realms::Boundary;
use brokkr_protocol::AttemptOutcome;
use brokkr_store::StoreError;
use serde_json::{json, Map, Value};

use super::carry::{
    outcome_under,
    record::{settled_members, PanelRecord, PanelSpan},
};
use super::cooldown::{refused_members, RefusedMember};
use super::dispatch::deadline::AttemptBound;
use super::{
    copy_secret_binding_facts, failed_to_start, git_head, stamp_boundary, start_failure_fields,
    start_failure_sites, DriverRun, Engine, EngineError, Indeterminate, Selection, Site, Unproven,
};
use crate::bundle::{
    dialect_results, Aggregate, Invocation, PanelMember, SeatClass, SequenceStep, StepBody,
};
use crate::dialect::DialectPhase;

mod compose;
mod contradiction;
mod dialect;
mod step_facts;
mod success;

pub(in crate::engine) use compose::StepComposition;
pub(crate) use step_facts::needs_change;
pub(super) use step_facts::{dialect_attempt_outcome, expand_dialect_argv, nearest_change};
use step_facts::{tailed_outcome, vocabulary_problem};

/// Where a step leaves its attempt, at each stage of settling it.
enum StepVerdict<T> {
    /// The attempt goes on, carrying `T` to the next stage or step.
    Goes(T),
    /// The attempt is over: its terminal event is journaled, so no later
    /// stage or step runs.
    Ended,
}

/// One sequence attempt's identity: the effect, the attempt and the seat
/// its steps run for, passed as a parameter object so the entry point
/// stays within ruling 4's seven parameters.
pub(in crate::engine) struct SequenceAt<'a> {
    effect_id: &'a str,
    attempt_id: &'a str,
    seat_name: &'a str,
}

impl<'a> SequenceAt<'a> {
    pub(in crate::engine) fn new(
        effect_id: &'a str,
        attempt_id: &'a str,
        seat_name: &'a str,
    ) -> Self {
        Self {
            effect_id,
            attempt_id,
            seat_name,
        }
    }
}

/// One sequence attempt's facts, the same for every step.
struct SequenceRun<'a> {
    effect_id: &'a str,
    attempt_id: &'a str,
    seat_name: &'a str,
    steps: &'a [SequenceStep],
    /// The requested-time seat input every step's driver input derives from.
    input: &'a Value,
    /// The attempt's deadline for its words, and what provisioning left of
    /// it (`bound.remaining`) for every step's bound.
    bound: AttemptBound<'a>,
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
    /// A deterministic dialect check's failing step and result, which
    /// neither an early ending nor the final step may contradict.
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

/// A step form's outcome, the sites that failed to start should the step
/// be the one that fails the attempt, and the sites whose reports carried
/// a typed rate-limit refusal.
struct StepRun {
    record: Option<PanelRecord>,
    outcome: AttemptOutcome,
    /// The chain-advance fact, unchanged from today: a site failed to
    /// start, in declared order.
    start_failures: Vec<Site>,
    /// The wait fact: every site whose report carried a typed rate-limit
    /// refusal, in declared order, whether or not its cleanup made the
    /// step indeterminate (decision 0103 ruling 2(a)).
    refused: Vec<RefusedMember>,
    /// What the step's terminal event carries when its tree is not
    /// proven over (#403).
    unproven: Unproven,
}

/// The results a step may report: the seat's own for the final step, which
/// is the seat's boundary, and the step's own for any other (decision 0002:
/// the one rule the attempt and the cool-down probe both read).
pub(super) fn step_allowed_results(seat_input: &Value, index: usize, last: bool) -> Value {
    if last {
        seat_input["allowed_results"].clone()
    } else {
        seat_input["steps"][index]["allowed_results"].clone()
    }
}

/// Which step a driver input is built for: a model step's attempt and its
/// cool-down probe carry the seat's spec dialect, while a dialect step's own
/// invocation predates that prompt and keeps the secret-binding facts alone
/// (decision 0103 slice 4c2b).
#[derive(Clone, Copy)]
pub(super) enum StepInputKind {
    Model,
    Dialect,
}

/// A step driver's input: the fields a single step and a dialect step
/// share, with the step's own metadata and the context its attempt (or its
/// probe) derived. The step's attempt and its cool-down probe both build
/// this one shape, and the tail it carries is decided once by `kind`
/// (decision 0103 slice 4c2b).
pub(super) fn step_driver_input(
    seat_input: &Value,
    index: usize,
    label: &str,
    last: bool,
    role_path: &Value,
    context: &Value,
    kind: StepInputKind,
) -> Value {
    let mut input = json!({
        "feature": seat_input["feature"],
        "phase": seat_input["phase"],
        "seat": label,
        "role_path": role_path,
        "workdir": seat_input["workdir"],
        "result_path": seat_input["steps"][index]["result_path"],
        "allowed_results": step_allowed_results(seat_input, index, last),
        "house_rules": seat_input["house_rules"],
        "context": context,
    });
    // Exhaustive on purpose: a new step kind must decide its own tail.
    let carries_spec_dialect = match kind {
        StepInputKind::Model => true,
        StepInputKind::Dialect => false,
    };
    if carries_spec_dialect && !seat_input["spec_dialect"].is_null() {
        input["spec_dialect"] = seat_input["spec_dialect"].clone();
    }
    copy_secret_binding_facts(&mut input, seat_input);
    input
}

impl SequenceRun<'_> {
    fn is_last(&self, at: &StepAt) -> bool {
        at.index + 1 == self.steps.len()
    }

    fn phase(&self) -> &str {
        self.input["phase"].as_str().unwrap_or("")
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

    /// A step driver's input, through the one derivation the step's attempt
    /// and its cool-down probe share, with the tail its own step form carries.
    fn driver_input(&self, at: &StepAt, role_path: &Value, kind: StepInputKind) -> Value {
        step_driver_input(
            self.input,
            at.index,
            &at.label,
            self.is_last(at),
            role_path,
            &at.context,
            kind,
        )
    }

    /// The `effect/failed` payload for a step: its problem, and the sites
    /// that failed to start.
    fn failed_payload(&self, at: &StepAt, problem: &str, start_failures: &[Site]) -> Value {
        let mut payload = json!({
            "effect_id": self.effect_id,
            "attempt_id": self.attempt_id,
            "error": format!("sequence step '{}': {problem}", at.step.name),
        });
        start_failure_fields(&mut payload, self.selection, start_failures.to_vec());
        payload
    }

    /// Why `result`, from a step before the last, ends the sequence, when
    /// it does: `hard-word` for the phase's hard vocabulary, `ends-on` for
    /// a compiled word no later step can emit. The sets are compiled on
    /// the step; nothing here compares vocabularies at run time.
    fn ends_sequence(&self, at: &StepAt, result: &Value) -> Option<&'static str> {
        let word = result.get("result").and_then(Value::as_str)?;
        if !at.step.ends_on.iter().any(|ending| ending == word) {
            return None;
        }
        Some(
            if at.step.hard_results.iter().any(|ending| ending == word) {
                "hard-word"
            } else {
                "ends-on"
            },
        )
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
    /// single typed result — decide() validates it exactly as today. Each
    /// step's outcome names the attempt's deadline where it is built, so
    /// only the step that ends the attempt reaches the terminal words.
    pub(super) fn execute_sequence(
        &mut self,
        at: &SequenceAt<'_>,
        steps: &[SequenceStep],
        seq_input: &Value,
        bound: AttemptBound<'_>,
        selection: &Selection,
    ) -> Result<(), EngineError> {
        let run = SequenceRun {
            effect_id: at.effect_id,
            attempt_id: at.attempt_id,
            seat_name: at.seat_name,
            steps,
            input: seq_input,
            bound,
            selection,
            declares_change: self
                .bundle
                .seats
                .get(at.seat_name)
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
            StepBody::Invocation(Invocation::Single { command, .. }) => {
                self.single_step(run, at, command)?
            }
            StepBody::Invocation(Invocation::Panel { members, aggregate }) => {
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
        let input = run.driver_input(at, &at.meta["role_path"], StepInputKind::Model);
        let gate = at.step.class == SeatClass::Gate;
        let composition = StepComposition {
            label: &at.label,
            effect_id: run.effect_id,
            attempt_id: run.attempt_id,
            gate,
            command,
            site: &site,
            selection: run.selection,
        };
        let (spawn, input) = self.compose_step(composition, input);
        // A sequence step now HAS such an identity: proposed
        // decision 0056 ruling 1 gives it a structural site
        // key, so a work-class step rejoins its own session
        // and a gate-class step never does. Under decision
        // 0030 alone there was nothing to match a step's
        // session by, and every step started cold.
        let mut driven = self.run_driver(
            run.effect_id,
            run.attempt_id,
            &at.label,
            &spawn,
            input,
            run.bound.remaining,
            Some(&at.step.name),
            run.selection.plan(&site),
        )?;
        run.bound.deadline.name(&mut driven);
        Ok(match driven {
            DriverRun::SpawnFailed(error) => StepRun {
                record: None,
                outcome: AttemptOutcome::Failed { error },
                start_failures: vec![Some(at.step.name.clone())],
                refused: Vec::new(),
                unproven: Unproven::Proven,
            },
            DriverRun::Ran(report) => {
                let start_failures = if failed_to_start(&report) {
                    vec![Some(at.step.name.clone())]
                } else {
                    Vec::new()
                };
                let refused = refused_members(
                    std::slice::from_ref(&(at.step.name.clone(), report.clone())),
                    "",
                );
                StepRun {
                    record: None,
                    start_failures,
                    refused,
                    unproven: Unproven::seat(&report),
                    outcome: tailed_outcome(&report),
                }
            }
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
        let mut span = PanelSpan::open(&self.repo);
        let tag_prefix = format!("{}:", at.step.name);
        let mut step_input = run.input.clone();
        step_input["allowed_results"] = step_allowed_results(run.input, at.index, run.is_last(at));
        let runs = self.member_runs(
            crate::engine::AttemptIds::new(run.effect_id, run.attempt_id),
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
        // One line, as panel.rs's callsites: the exact-coverage gate reads a
        // multi-line call's `?` edge as its own region.
        let remaining = run.bound.remaining;
        let reports = self.run_panel(effect_id, attempt_id, &runs, remaining, &tag_prefix)?;
        self.journal_panel_members(effect_id, attempt_id, &reports, &runs, &tag_prefix)?;
        span.close(&self.repo);
        let record = PanelRecord {
            span,
            settled_members: settled_members(&reports, Some(&at.step.name)),
        };
        let plan = (at.index == 0)
            .then_some(run.selection.carry.as_ref())
            .flatten();
        let start_failures = start_failure_sites(&reports, &tag_prefix);
        let refused = refused_members(&reports, &tag_prefix);
        let unproven = Unproven::panel(&reports);
        let name = run.bound.deadline.panel_namer(&reports);
        let mut outcome = outcome_under(aggregate, plan, reports, &record.span, run.attempt_id)?;
        name(&mut outcome);
        Ok(StepRun {
            record: Some(record),
            start_failures,
            refused,
            unproven,
            outcome: outcome.outcome(),
        })
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
        let start_failures = ran.start_failures.clone();
        let StepVerdict::Goes(result) = self.step_result(run, at, ran, boundary)? else {
            return Ok(StepVerdict::Ended);
        };
        if !run.is_last(at) {
            let StepVerdict::Goes(()) =
                self.between_steps(run, at, &result, carried, &start_failures)?
            else {
                return Ok(StepVerdict::Ended);
            };
        }
        let StepVerdict::Goes(()) = self.carry_change(run, at, &result, carried)? else {
            return Ok(StepVerdict::Ended);
        };
        // A step's result the fence refuses (decision 0034, ruling
        // 6) fails the attempt at that step, with the same facts a
        // step that failed on its own would carry.
        let refused = |mut refusal: String| {
            run.bound.deadline.name_words(&mut refusal);
            run.failed_payload(at, &refusal, &start_failures)
        };
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
        ran: StepRun,
        boundary: Option<Boundary>,
    ) -> Result<StepVerdict<Value>, EngineError> {
        match ran.outcome {
            AttemptOutcome::Succeeded { result } => {
                Ok(StepVerdict::Goes(stamp_boundary(result, boundary)))
            }
            AttemptOutcome::Failed { error } => {
                let mut payload = run.failed_payload(at, &error, &ran.start_failures);
                if let Some(record) = ran.record {
                    record.stamp(&mut payload);
                }
                let terminal = self.step_terminal(run.effect_id, payload, &ran.refused, true)?;
                let _ = self.end_failed::<()>(run, terminal.payload)?;
                if let Some(reason) = terminal.park {
                    self.park_with_reason(reason)?;
                }
                Ok(StepVerdict::Ended)
            }
            AttemptOutcome::Indeterminate { reason } => {
                let mut payload = json!(Indeterminate {
                    effect_id: run.effect_id,
                    attempt_id: run.attempt_id,
                    reason: format!("sequence step '{}': {reason}", at.step.name),
                    unproven: ran.unproven
                });
                if let Some(record) = ran.record {
                    record.stamp(&mut payload);
                }
                let terminal = self.step_terminal(run.effect_id, payload, &ran.refused, false)?;
                self.append(
                    EventType::EffectIndeterminate,
                    terminal.payload,
                    Some(run.attempt_id.to_owned()),
                )
                .map(|_| StepVerdict::Ended)
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
        start_failures: &[Site],
    ) -> Result<StepVerdict<()>, EngineError> {
        if let Some(mut problem) = vocabulary_problem(at.step, result) {
            run.bound.deadline.name_words(&mut problem);
            return self.end_failed(run, run.failed_payload(at, &problem, &[]));
        }
        // A deterministic dialect check's failure is recorded before any
        // ending is decided, so an early ending passes the same
        // contradiction check the final step does.
        let phase = run.phase();
        if matches!(at.step.body, StepBody::Dialect { .. })
            && matches!(
                DialectPhase::parse(phase).map(DialectPhase::is_loop),
                Some(true)
            )
            && result["result"] == dialect_results(phase)[1]
        {
            carried.deterministic_failure = Some((
                at.step.name.clone(),
                result["result"].as_str().unwrap().to_string(),
            ));
        }
        // The compiled endings settle a non-final step: an `ends_on` word
        // the step may report, or a hard word from the phase's table. The
        // journal names the step and why it ended.
        if let Some(why) = run.ends_sequence(at, result) {
            return self.end_sequence_on(run, at, result, why, start_failures, carried);
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
        if let StepVerdict::Ended = self.refuse_contradiction(run, at, &result, carried)? {
            return Ok(StepVerdict::Ended);
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

    /// End the sequence on a compiled word, journaling the step and why
    /// beside the result under the same seat-record fence as a final
    /// step's result (decision 0034). A result that contradicts a
    /// deterministic dialect check's failure parks here first, exactly as
    /// it would at the final step.
    fn end_sequence_on(
        &mut self,
        run: &SequenceRun,
        at: &StepAt,
        result: &Value,
        why: &'static str,
        start_failures: &[Site],
        carried: &Carried,
    ) -> Result<StepVerdict<()>, EngineError> {
        if let StepVerdict::Ended = self.refuse_contradiction(run, at, result, carried)? {
            return Ok(StepVerdict::Ended);
        }
        self.append_succeeded_under_fence(
            json!({
                "effect_id": run.effect_id,
                "attempt_id": run.attempt_id,
                "result": result,
                "step": at.step.name,
                "why": why,
            }),
            run.attempt_id,
            |mut refusal| {
                run.bound.deadline.name_words(&mut refusal);
                run.failed_payload(at, &refusal, start_failures)
            },
        )?;
        Ok(StepVerdict::Ended)
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
        let mut reason = format!("sequence step '{}': {reason}", at.step.name);
        run.bound.deadline.name_words(&mut reason);
        self.append(
            EventType::EffectIndeterminate,
            json!(Indeterminate {
                effect_id: run.effect_id,
                attempt_id: run.attempt_id,
                reason,
                unproven,
            }),
            Some(run.attempt_id.to_string()),
        )
        .map(|_| StepVerdict::Ended)
    }
}
