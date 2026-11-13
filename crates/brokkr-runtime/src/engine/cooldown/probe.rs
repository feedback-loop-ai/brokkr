//! The one-turn cool-down probe (decision 0103 ruling 4(a)): a dedicated
//! invocation of the cooled site's own adapter and launch path, never the
//! retried effect, and what its report leaves. Its seam is the engine's;
//! this module owns the probe so `cooldown.rs` stays within ruling 4's
//! 800-line ceiling.

use std::time::{Duration, Instant};

use brokkr_core::envelope::EventType;
use brokkr_core::fold::RunState;
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderWait};
use brokkr_protocol::hands::HandsSpec;
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, RefusalClass};
use serde_json::{Map, Value};
use uuid::Uuid;

use super::{terminal, Engine, EngineError};
use crate::agents::Candidate;
use crate::bundle::{
    Aggregate, ExecutableBody, Invocation, PanelMember, Seat, SeatClass, SequenceStep, StepBody,
    ENGINE_VERSION,
};
use crate::engine::broker::{observed, settlement};
use crate::engine::carry::record::Tag;
use crate::engine::dispatch::Dispatch;
use crate::engine::sequence::{
    step_allowed_results, step_driver_input, StepComposition, StepInputKind,
};
use crate::engine::{chain_index, phase_of, spawn_site, Door, Selection, Site, SiteSpawn};

/// The seat a probe composes under, selected once: the body's form, the
/// case and site label, and the gate class. One selection serves both the
/// single seat's probe and a panel member's, so neither reads the seat's
/// body its own way.
#[derive(Clone)]
struct ProbeSeat {
    seat: Seat,
    case: Option<String>,
    site: String,
    gate: bool,
    form: ProbeForm,
}

/// One sited probe's common facts: the journal the refused attempt stands
/// in, the run state, the effect and seat, the wait, and the seat's
/// selected body.
struct ProbeSite<'a> {
    events: &'a [EventEnvelope],
    state: &'a RunState,
    effect_id: &'a str,
    seat: &'a str,
    wait: &'a ProviderWait,
    picked: &'a ProbeSeat,
}

/// The selected body in the shape the probe composes.
#[derive(Clone)]
enum ProbeForm {
    Single(Vec<Candidate>),
    Panel(Vec<PanelMember>, Aggregate),
    Sequence(Vec<SequenceStep>),
}

impl Engine {
    /// The selected site label (`<seat>` or `<seat>:<case>`) this run's
    /// strategy names, as every attempt and probe composes it: the exact
    /// identity a cleanup refusal matches a reserved driver by (decision
    /// 0103 ruling 4).
    pub(in crate::engine) fn selected_label(&self, seat: &str, strategy: Option<&str>) -> String {
        let case = self.bundle.seats[seat]
            .body
            .selected(strategy)
            .and_then(|(_, case)| case);
        site_name(seat, case)
    }

    /// The seat a probe composes under, selected once: the body's form, the
    /// case, and the gate class. Its label is [`Engine::selected_label`]'s,
    /// the one derivation a cleanup refusal matches a reserved driver by
    /// (decision 0103 ruling 4; ruling 5).
    fn probe_seat(&self, seat: &str, strategy: Option<&str>) -> ProbeSeat {
        let site = self.selected_label(seat, strategy);
        let seat_def = &self.bundle.seats[seat];
        let (body, case) = seat_def
            .body
            .selected(strategy)
            .expect("seat input resolved the selector");
        let case = case.map(str::to_string);
        let gate = seat_def
            .body
            .selected_is_gate(strategy, seat_def.has_gate());
        let form = match body {
            ExecutableBody::Invocation(Invocation::Single { candidates, .. }) => {
                ProbeForm::Single(candidates.clone())
            }
            ExecutableBody::Invocation(Invocation::Panel { members, aggregate }) => {
                ProbeForm::Panel(members.clone(), *aggregate)
            }
            ExecutableBody::Sequence { steps } => ProbeForm::Sequence(steps.to_vec()),
        };
        ProbeSeat {
            seat: seat_def.clone(),
            case,
            site,
            gate,
            form,
        }
    }
}

/// The selected body itself, borrowed from the seat the probe owns.
fn probe_body<'a>(picked: &'a ProbeSeat, strategy: Option<&str>) -> ExecutableBody<'a> {
    picked
        .seat
        .body
        .selected(strategy)
        .expect("seat input resolved the selector")
        .0
}

/// What an admitted probe holds before its driver composes: the attempt
/// name, the input it runs with, and the deadline its admission shared.
struct AdmittedProbe {
    attempt: String,
    input: Value,
    deadline: Duration,
}

impl Engine {
    /// Read the probe's input and admit it under its attempt, holding no
    /// effect attempt: `Ok` is the admitted probe, `Err` the park reason.
    fn admit_probe(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        wait: &ProviderWait,
        (picked, body): (&ProbeSeat, ExecutableBody<'_>),
        selection: &Selection,
    ) -> Result<Result<AdmittedProbe, Probe>, EngineError> {
        let phase = phase_of(state, "probe without a phase")?;
        let input = self.seat_input(state, &phase, effect_id)?;
        let attempt = Uuid::new_v4().to_string();
        let observed = observed();
        let began = Instant::now();
        let deadline = Duration::from_secs(picked.seat.limits.timeout_seconds);
        if let Err(refusal) = self.admit_for_probe(
            events,
            [&attempt, &picked.site],
            body,
            selection,
            (began, deadline),
            &observed,
        ) {
            return Ok(Err(Probe::Parked(format!(
                "the cool-down probe for {}/{} was not admitted: {refusal}",
                wait.provider, wait.route
            ))));
        }
        Ok(Ok(AdmittedProbe {
            attempt,
            input,
            deadline,
        }))
    }

    /// The dispatch one admitted probe composes under, at `site_name`: the
    /// seat's own for a single seat, and the member's derived
    /// `<seat>:<member>` for a panel member.
    fn probe_dispatch<'a>(
        effect_id: &'a str,
        attempt: &'a str,
        seat: &'a str,
        site_name: &'a str,
        picked: &'a ProbeSeat,
        body: ExecutableBody<'a>,
    ) -> Dispatch<'a> {
        Dispatch {
            effect_id,
            attempt_id: attempt,
            seat_name: seat,
            case: picked.case.as_deref(),
            site_name,
            body,
            gate: picked.gate,
        }
    }

    /// A dedicated one-turn invocation of the cooled site's own adapter and
    /// launch path (decision 0103 ruling 4(a)): never the retried effect, at
    /// the route the entry names, with the refused candidate's model pin and
    /// effort and the same boundary and credential facts. It is launched as
    /// its attempt is: admission seals the reach its box is built from and,
    /// where the root is protected, takes the writer lease its door
    /// rechecks, and settlement releases that lease once the report proved
    /// its processes settled.
    pub(super) fn probe(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
    ) -> Result<Probe, EngineError> {
        let picked = self.probe_seat(seat, state.strategy.as_deref());
        let candidates = match &picked.form {
            ProbeForm::Single(candidates) => candidates.clone(),
            _ => {
                return Ok(Probe::Parked(format!(
                    "no cool-down probe for {}/{}: a wait is recorded at a body that is not a \
                     single seat",
                    wait.provider, wait.route
                )))
            }
        };
        let mut selection = Selection::new();
        if !candidates.is_empty() {
            let index = refused_index(events, effect_id, &None, candidates.len());
            selection.insert(None, candidates[index].clone());
        }
        let hands = self.runtime_hands(&picked.site);
        let body = probe_body(&picked, state.strategy.as_deref());
        let admitted =
            match self.admit_probe(events, state, effect_id, wait, (&picked, body), &selection)? {
                Ok(admitted) => admitted,
                Err(parked) => return Ok(parked),
            };
        let at = Self::probe_dispatch(
            effect_id,
            &admitted.attempt,
            seat,
            &picked.site,
            &picked,
            body,
        );
        let probe = self.probe_run(
            &at,
            admitted.input,
            hands.as_ref(),
            &selection,
            admitted.deadline,
            wait,
        );
        self.settle_probe(probe)
    }

    /// A cooled site-tagged wait's own one-turn probe: a seat-level panel
    /// member's through [`Engine::member_runs`] (decision 0103 slice 4c2a),
    /// and a sequence step's or step member's through the step's own
    /// composition (slice 4c2b).
    pub(super) fn probe_member(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
        member: &str,
    ) -> Result<Probe, EngineError> {
        let picked = self.probe_seat(seat, state.strategy.as_deref());
        match &picked.form {
            ProbeForm::Panel(members, aggregate) => {
                let Some(member_def) = members.iter().find(|candidate| candidate.name == member)
                else {
                    return Ok(Probe::Parked(format!(
                        "no cool-down probe for {}/{}: the panel declares no member {member}",
                        wait.provider, wait.route
                    )));
                };
                let tag = Some(member_def.name.clone());
                let (selection, one) =
                    probe_member_invocation(events, effect_id, tag, member_def, *aggregate);
                let body = ExecutableBody::Invocation(&one);
                // A member's own site label is `<seat>:<member>`; the
                // one-member body makes the claim's writer walk produce
                // exactly that.
                let prepared = match self.admit_probe(
                    events,
                    state,
                    effect_id,
                    wait,
                    (&picked, body),
                    &selection,
                )? {
                    Ok(prepared) => prepared,
                    Err(parked) => return Ok(parked),
                };
                let runs = self.member_runs(
                    crate::engine::AttemptIds::new(effect_id, &prepared.attempt),
                    &picked.site,
                    std::slice::from_ref(member_def),
                    &prepared.input["members"],
                    &prepared.input,
                    &prepared.input["context"],
                    &selection,
                    "",
                    picked.gate,
                );
                let run = &runs[0];
                // `member_runs` derives this member's own driver seat, which
                // its door, `effect/started` and settlement record all name.
                let at = Self::probe_dispatch(
                    effect_id,
                    &prepared.attempt,
                    seat,
                    &run.driver_seat,
                    &picked,
                    body,
                );
                let probe =
                    self.spawn_probe(&at, &run.spawn, run.input.clone(), prepared.deadline, wait);
                self.settle_probe(probe)
            }
            ProbeForm::Sequence(steps) => {
                let at = ProbeSite {
                    events,
                    state,
                    effect_id,
                    seat,
                    wait,
                    picked: &picked,
                };
                self.probe_sequence_site(&at, steps, member)
            }
            ProbeForm::Single(_) => Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: member {member} stands at a body that is not a \
                 panel or a sequence",
                wait.provider, wait.route
            ))),
        }
    }

    /// A sequence site's own probe: the step a `<step>` wait names, or the
    /// member a `<step>:<member>` wait names, composed on the step's own
    /// body — never the single seat's composition (decision 0103 slice
    /// 4c2b).
    fn probe_sequence_site(
        &mut self,
        at: &ProbeSite<'_>,
        steps: &[SequenceStep],
        site: &str,
    ) -> Result<Probe, EngineError> {
        let Some((index, step, member)) = step_site(steps, site) else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: the sequence declares no step at {site}",
                at.wait.provider, at.wait.route
            )));
        };
        match member {
            None => self.probe_step_single(at, steps, index, step),
            Some(member) => self.probe_step_member(at, steps, index, step, member),
        }
    }

    /// Admit a sequence site's probe body under `at`: the one call both a
    /// step's probe and a step member's take.
    fn admit_site(
        &mut self,
        at: &ProbeSite<'_>,
        picked: &ProbeSeat,
        body: ExecutableBody<'_>,
        selection: &Selection,
    ) -> Result<Result<AdmittedProbe, Probe>, EngineError> {
        self.admit_probe(
            at.events,
            at.state,
            at.effect_id,
            at.wait,
            (picked, body),
            selection,
        )
    }

    /// Admit a sequence site's probe body and derive the facts both a step's
    /// probe and a step member's share: the admitted attempt, whether the
    /// step is final, and the step's attempt-time context.
    fn admit_step(
        &mut self,
        at: &ProbeSite<'_>,
        index: usize,
        steps: &[SequenceStep],
        picked: &ProbeSeat,
        body: ExecutableBody<'_>,
        selection: &Selection,
    ) -> Result<Result<(AdmittedProbe, bool, Value), Probe>, EngineError> {
        let prepared = match self.admit_site(at, picked, body, selection)? {
            Ok(prepared) => prepared,
            Err(parked) => return Ok(Err(parked)),
        };
        let last = index + 1 == steps.len();
        let context = step_context(at.events, at.effect_id, &prepared.input);
        Ok(Ok((prepared, last, context)))
    }

    /// A `<step>` wait's probe: the step's own command, the link the refusal
    /// named, and its model pin, effort, boundary and credential facts,
    /// through the same composition the step's attempt used.
    fn probe_step_single(
        &mut self,
        at: &ProbeSite<'_>,
        steps: &[SequenceStep],
        index: usize,
        step: &SequenceStep,
    ) -> Result<Probe, EngineError> {
        let StepBody::Invocation(Invocation::Single {
            command,
            candidates,
            ..
        }) = &step.body
        else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: step {} is not a single driver",
                at.wait.provider, at.wait.route, step.name
            )));
        };
        let driver_seat = format!("{}:{}", at.picked.site, step.name);
        let tag = Some(step.name.clone());
        let mut selection = Selection::new();
        if !candidates.is_empty() {
            let index = refused_index(at.events, at.effect_id, &tag, candidates.len());
            selection.insert(tag.clone(), candidates[index].clone());
        }
        let one = std::slice::from_ref(step);
        let body = ExecutableBody::Sequence { steps: one };
        let (prepared, last, context) =
            match self.admit_step(at, index, steps, at.picked, body, &selection)? {
                Ok(admitted) => admitted,
                Err(parked) => return Ok(parked),
            };
        let input = step_driver_input(
            &prepared.input,
            index,
            &driver_seat,
            last,
            &prepared.input["steps"][index]["role_path"],
            &context,
            StepInputKind::Model,
        );
        let composition = StepComposition {
            label: &driver_seat,
            effect_id: at.effect_id,
            attempt_id: &prepared.attempt,
            gate: step.class == SeatClass::Gate,
            command,
            site: &tag,
            selection: &selection,
        };
        let (spawn, input) = self.compose_step(composition, input);
        let dispatch = Self::probe_dispatch(
            at.effect_id,
            &prepared.attempt,
            at.seat,
            &driver_seat,
            at.picked,
            body,
        );
        let probe = self.spawn_probe(&dispatch, &spawn, input, prepared.deadline, at.wait);
        self.settle_probe(probe)
    }

    /// A `<step>:<member>` wait's probe: that member through
    /// [`Engine::member_runs`] with the step's members and its `<step>:`
    /// prefix, never the seat-level panel's composition.
    fn probe_step_member(
        &mut self,
        at: &ProbeSite<'_>,
        steps: &[SequenceStep],
        index: usize,
        step: &SequenceStep,
        member: &str,
    ) -> Result<Probe, EngineError> {
        let StepBody::Invocation(Invocation::Panel { members, aggregate }) = &step.body else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: step {} is not a panel",
                at.wait.provider, at.wait.route, step.name
            )));
        };
        let Some(member_def) = members.iter().find(|candidate| candidate.name == member) else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: step {} declares no member {member}",
                at.wait.provider, at.wait.route, step.name
            )));
        };
        let tag_prefix = format!("{}:", step.name);
        let tag = Some(Tag::prefixed(&tag_prefix, &member_def.name).0);
        let (selection, one) =
            probe_member_invocation(at.events, at.effect_id, tag, member_def, *aggregate);
        let body = ExecutableBody::Invocation(&one);
        // The one-member body is anchored at the step's own driver site, so
        // the claim's writer walk seals `work:<step>:<member>` exactly.
        let driver_seat = format!("{}:{}", at.picked.site, step.name);
        let mut picked = at.picked.clone();
        picked.site = driver_seat.clone();
        let (prepared, last, context) =
            match self.admit_step(at, index, steps, &picked, body, &selection)? {
                Ok(admitted) => admitted,
                Err(parked) => return Ok(parked),
            };
        let mut step_input = prepared.input.clone();
        step_input["allowed_results"] = step_allowed_results(&prepared.input, index, last);
        let runs = self.member_runs(
            crate::engine::AttemptIds::new(at.effect_id, &prepared.attempt),
            &driver_seat,
            std::slice::from_ref(member_def),
            &step_input["steps"][index]["members"],
            &step_input,
            &context,
            &selection,
            &tag_prefix,
            step.class == SeatClass::Gate,
        );
        let run = &runs[0];
        let dispatch = Self::probe_dispatch(
            at.effect_id,
            &prepared.attempt,
            at.seat,
            &run.driver_seat,
            at.picked,
            body,
        );
        let probe = self.spawn_probe(
            &dispatch,
            &run.spawn,
            run.input.clone(),
            prepared.deadline,
            at.wait,
        );
        self.settle_probe(probe)
    }

    /// The admitted probe's one driver, composed and marked with no broker:
    /// spawned under its attempt's lease and run as a probe, its cleanup
    /// witnessed on the lease. No `begun` is read and no broker call folded:
    /// the probe serves none, so `begun` is `None` by construction and the
    /// only settlement fact that can stand is its report's cleanup.
    fn probe_run(
        &mut self,
        at: &Dispatch<'_>,
        mut input: Value,
        hands: Option<&HandsSpec>,
        selection: &Selection,
        deadline: Duration,
        wait: &ProviderWait,
    ) -> Probe {
        let composed = self.composed_single(at, selection, &mut input, hands, &[]);
        let spawn = composed.expect("a single seat composed its command");
        self.spawn_probe(at, &spawn, input, deadline, wait)
    }

    /// Spawn one admitted probe's already-composed site and run it to its
    /// end: the single seat's composed spawn and the panel member's own.
    fn spawn_probe(
        &mut self,
        at: &Dispatch<'_>,
        spawn: &SiteSpawn,
        input: Value,
        deadline: Duration,
        wait: &ProviderWait,
    ) -> Probe {
        let door = self
            .held
            .as_ref()
            .map(|held| Door::of(held, at.site_name, None));
        let workdir = self.workdir();
        let (process, input) =
            match spawn_site(&self.bundle, spawn, &input, &workdir, deadline, door) {
                Ok(started) => started,
                Err(reason) => {
                    return Probe::Parked(format!(
                        "the cool-down probe for {}/{} did not spawn: {reason}",
                        wait.provider, wait.route
                    ));
                }
            };
        let report = process.probing().run_attempt_resuming(
            ENGINE_VERSION,
            at.effect_id,
            at.attempt_id,
            at.site_name,
            input,
            None,
            |_| {},
        );
        settlement::ended(&mut self.held, at.site_name, &report.cleanup);
        self.probe_end(wait, report)
    }

    /// What the probe's report leaves: an answer permits the retry, and
    /// every other end parks, cleanup's refusal first (decision 0103
    /// ruling 4(a)). A typed rate-limit refusal parks in 4c1 because the
    /// table that records its next round is 4d's.
    pub(super) fn probe_end(&self, wait: &ProviderWait, report: AttemptReport) -> Probe {
        if let Cleanup::Unresolved { reason } = &report.cleanup {
            return Probe::Parked(cleanup_failure(
                Some(&wait.provider),
                Some(&wait.route),
                &reason.to_string(),
            ));
        }
        if let Some(refusal) = &report.refusal {
            let said = match refusal.class() {
                RefusalClass::RateLimit => "was refused again by the provider",
                RefusalClass::Other => "was refused",
            };
            return Probe::Parked(format!(
                "the cool-down probe for {}/{} {said}: {}",
                wait.provider,
                wait.route,
                refusal.words()
            ));
        }
        match report.settled_outcome() {
            AttemptOutcome::Succeeded { .. } => match report.checkpoints.first() {
                Some(_) => Probe::Answered(ProbeAnswer {
                    site: wait.site.clone(),
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    answered_at: self.now(),
                }),
                None => Probe::Parked(format!(
                    "the cool-down probe for {}/{} reported success with no answer checkpoint",
                    wait.provider, wait.route
                )),
            },
            AttemptOutcome::Failed { error } => Probe::Parked(format!(
                "the cool-down probe for {}/{} ended without an answer: {error}",
                wait.provider, wait.route
            )),
            AttemptOutcome::Indeterminate { reason } => Probe::Parked(format!(
                "the cool-down probe for {}/{} ended without an answer: {reason}",
                wait.provider, wait.route
            )),
        }
    }
}

/// The sequence step and member a wait's site names. An exact step name is
/// a single step; otherwise a member is resolved by the declared
/// `(<step>, <member>)` pair, so a wait at `a:b:c` is `a:b`'s member `c`
/// even when `a` also declares a member `b:c`. The prefix walk that follows
/// is only for the diagnostic a site naming neither: exact first, so a step
/// whose own name contains a colon is not mistaken for a member of an
/// earlier one.
fn step_site<'a>(
    steps: &'a [SequenceStep],
    site: &'a str,
) -> Option<(usize, &'a SequenceStep, Option<&'a str>)> {
    if let Some((index, step)) = steps.iter().enumerate().find(|(_, step)| step.name == site) {
        return Some((index, step, None));
    }
    if let Some((index, step, member)) = declared_member(steps, site) {
        return Some((index, step, Some(member)));
    }
    steps.iter().enumerate().find_map(|(index, step)| {
        site.strip_prefix(&format!("{}:", step.name))
            .map(|member| (index, step, Some(member)))
    })
}

/// The step and member whose declared tag is exactly `site`, in declaration
/// order.
fn declared_member<'a>(
    steps: &'a [SequenceStep],
    site: &str,
) -> Option<(usize, &'a SequenceStep, &'a str)> {
    steps.iter().enumerate().find_map(|(index, step)| {
        let StepBody::Invocation(Invocation::Panel { members, .. }) = &step.body else {
            return None;
        };
        members
            .iter()
            .find(|member| Tag::new(Some(&step.name), &member.name).0 == site)
            .map(|member| (index, step, member.name.as_str()))
    })
}

/// The context a step's attempt had: the seat's context with the earlier
/// steps' checkpointed results of the failed attempt beside it, so the
/// probe's input derives exactly as the attempt's did (decision 0103 slice
/// 4c2b).
pub(super) fn step_context(events: &[EventEnvelope], effect_id: &str, seat_input: &Value) -> Value {
    let mut context = seat_input["context"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let attempt = terminal(events, effect_id)
        .and_then(|event| event.payload.get("attempt_id"))
        .and_then(Value::as_str);
    let mut prior = Map::new();
    for event in events {
        if event.event_type != EventType::EffectCheckpointed
            || event.payload.get("effect_id").and_then(Value::as_str) != Some(effect_id)
            || attempt.is_some_and(|attempt| {
                event.payload.get("attempt_id").and_then(Value::as_str) != Some(attempt)
            })
        {
            continue;
        }
        let Some(checkpoint) = event.payload.get("checkpoint") else {
            continue;
        };
        if checkpoint.get("step").and_then(Value::as_str) != Some("sequence-step-finished") {
            continue;
        }
        let (Some(name), Some(result)) = (
            checkpoint.get("step_name").and_then(Value::as_str),
            checkpoint.get("result"),
        ) else {
            continue;
        };
        prior.insert(name.to_string(), result.clone());
    }
    context.insert("prior_results".into(), Value::Object(prior));
    Value::Object(context)
}

/// The selection and one-member body a panel member's probe admits: its
/// own candidates, the link the refusal named, and the tag the journal
/// names it by. A seat-level member and a sequence step's member take this
/// one composition.
fn probe_member_invocation(
    events: &[EventEnvelope],
    effect_id: &str,
    tag: Option<String>,
    member: &PanelMember,
    aggregate: Aggregate,
) -> (Selection, Invocation) {
    let mut selection = Selection::new();
    if !member.candidates.is_empty() {
        let index = refused_index(events, effect_id, &tag, member.candidates.len());
        selection.insert(tag, member.candidates[index].clone());
    }
    let one = Invocation::Panel {
        members: vec![member.clone()],
        aggregate,
    };
    (selection, one)
}

/// The chain link the refused attempt used: the start failures recorded
/// before the effect's last terminal event, clamped as `chain_index` does.
fn refused_index(events: &[EventEnvelope], effect_id: &str, site: &Site, chain: usize) -> usize {
    let before = terminal(events, effect_id)
        .and_then(|last| events.iter().position(|event| event.seq == last.seq))
        .map_or(events, |at| &events[..at]);
    chain_index(before, effect_id, site, chain)
}

/// The label a seat's attempt and its probe are both composed under: the
/// seat, or `<seat>:<case>` where the selector chose a case.
fn site_name(seat: &str, case: Option<&str>) -> String {
    case.map(|case| format!("{seat}:{case}"))
        .unwrap_or_else(|| seat.to_string())
}

/// The opening of every cleanup refusal: the probe for this wait did not
/// prove its process tree gone, so it is ruling 4's cleanup failure. The
/// admission's recovery appends the reservation's facts to the same
/// sentence (decision 0071 ruling 5: one home for the rule). `None` is a
/// reservation whose own attempt names no wait: the sentence stays neutral
/// rather than borrowing another member's route.
pub(in crate::engine) fn cleanup_failure(
    provider: Option<&str>,
    route: Option<&str>,
    detail: &str,
) -> String {
    match (provider, route) {
        (Some(provider), Some(route)) => format!(
            "the cool-down probe for {provider}/{route} did not prove its process tree gone, so \
             it is a cleanup failure: {detail}"
        ),
        _ => format!(
            "a cool-down probe did not prove its process tree gone, so it is a cleanup failure: \
             {detail}"
        ),
    }
}

/// What a probe leaves.
pub(super) enum Probe {
    Answered(ProbeAnswer),
    Parked(String),
}
