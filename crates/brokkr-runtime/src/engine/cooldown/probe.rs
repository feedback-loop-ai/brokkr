//! The one-turn cool-down probe (decision 0103 ruling 4(a)): a dedicated
//! invocation of the cooled site's own adapter and launch path, never the
//! retried effect, and what its report leaves. Its seam is the engine's;
//! this module owns the probe so `cooldown.rs` stays within ruling 4's
//! 800-line ceiling.

use std::time::{Duration, Instant};

use brokkr_core::fold::RunState;
use brokkr_core::park::Park;
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderWait};
use brokkr_protocol::hands::HandsSpec;
use brokkr_protocol::Cleanup;
use brokkr_store::ProbeHold;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{Engine, EngineError};
use crate::admission::spelled;
use crate::agents::Candidate;
use crate::bundle::{
    Aggregate, ExecutableBody, Invocation, PanelMember, Seat, SeatClass, SequenceStep, StepBody,
    ENGINE_VERSION,
};
use crate::engine::broker::{observed, settlement};
use crate::engine::carry::record::Tag;
use crate::engine::dispatch::{spawn_site_by, Dispatch, ProbeDeadline, SpawnDeadline, Unspawned};
use crate::engine::sequence::{
    step_allowed_results, step_driver_input, StepComposition, StepInputKind,
};
use crate::engine::{phase_of, Door, Selection, SiteSpawn};

mod end;
mod sequence;
pub(in crate::engine) use end::cleanup_failure;
pub(super) use sequence::step_context;
use sequence::{probe_member_invocation, refused_index, site_name, step_site};

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
/// name, the input it runs with, and the one deadline its hold and its
/// driver both end by (fix rounds 0106b1r5, H2, and 0106b1r9, E1).
struct AdmittedProbe {
    attempt: String,
    input: Value,
    deadline: ProbeDeadline,
}

/// A probe whose deadline passed before its driver could be spawned (fix
/// round 0106b1r5, H2): it is not spawned, and its route's hold stands to
/// its stored end, past the deadline by the shutdown grace and the hold's
/// margin (fix round 0106b1r10, H2). Its words are the park's reason.
#[derive(Debug, thiserror::Error)]
#[error(
    "the cool-down probe for {provider}/{route} was not spawned: its deadline {} passed before \
     its driver could start, and its route's hold stands to its stored end, past the deadline \
     by the shutdown grace and the hold's margin",
    spelled(*ends)
)]
struct Late {
    provider: String,
    route: String,
    ends: OffsetDateTime,
}

impl Late {
    /// The park of `wait`'s probe, unspawned at its deadline `ends`.
    fn park(wait: &ProviderWait, ends: OffsetDateTime) -> Probe {
        let late = Late {
            provider: wait.provider.clone(),
            route: wait.route.clone(),
            ends,
        };
        Probe::Parked(Park::ruled(late.to_string()))
    }
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
        // The probe's one deadline, taken once as it begins, on the
        // monotonic clock read above and then the engine's: the fork reads
        // it, its watchdog kills at it, and its hold's stored end, the crash
        // fallback, lies past it by the shutdown grace and the margin.
        let now = self.now();
        let ends = ProbeDeadline::begun(began, now, deadline);
        if let Err(unheld) = self.hold_the_route(wait, now, ends.fallback) {
            return Ok(Err(Probe::Parked(Park::ruled(format!(
                "the cool-down probe for {}/{} was not admitted: its route's hold was not \
                 written: {unheld}",
                wait.provider, wait.route
            )))));
        }
        if let Err(refusal) = self.admit_for_probe(
            events,
            [&attempt, &picked.site],
            body,
            selection,
            (began, deadline),
            &observed,
        ) {
            return Ok(Err(Probe::Parked(Park::ruled(format!(
                "the cool-down probe for {}/{} was not admitted: {refusal}",
                wait.provider, wait.route
            )))));
        }
        Ok(Ok(AdmittedProbe {
            attempt,
            input,
            deadline: ends,
        }))
    }

    /// Hold the probe's route before it is reserved and dispatched (fix
    /// round 0106b1r4, F1; the operator's ruling of 2026-11-18, amending
    /// decision 0103 ruling 2): one hold row, written at `now`, carrying the
    /// provider, the route and its crash fallback `fallback` (fix round
    /// 0106b1r10, H2), which the store keeps to the first whole second
    /// strictly after it (fix round 0106b1r6, D1). The pass and the claim
    /// read it as cooling down until the engine ends it on evidence (H1),
    /// or, where nothing ended it, that second comes.
    /// A hold the store refuses parks the probe unadmitted, so no probe
    /// runs on a route it could not hold: one the table did not write, one
    /// whose end cannot be stored after its fallback, and one on a queue
    /// this binary does not write, older than version 7 or a newer
    /// binary's, which takes no hold (fix round 0106b1r9, E3). An engine's
    /// own open moves a version 4 to 6 queue to version 7 before any probe.
    fn hold_the_route(
        &mut self,
        wait: &ProviderWait,
        now: OffsetDateTime,
        fallback: OffsetDateTime,
    ) -> Result<(), brokkr_store::StoreError> {
        let hold = ProbeHold {
            run_id: &self.run_id,
            site: wait.site.as_deref().unwrap_or(""),
            provider: &wait.provider,
            route: &wait.route,
            fallback,
        };
        // Only a hold this insert returned is ever ended: one the store
        // refuses, or did not write, leaves none (fix round 0106b1r6, D2).
        self.cooldown.hold = None;
        self.cooldown.hold = Some(self.store.hold_probe(&hold, now)?);
        Ok(())
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
    /// rechecks. The lease is returned standing: the wait's `probe_each`
    /// settles it once the probe's end is recorded,
    /// so a renewed refusal's round is committed before the release (fix
    /// round 0106b1r3).
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
                return Ok(Probe::Parked(Park::ruled(format!(
                    "no cool-down probe for {}/{}: a wait is recorded at a body that is not a \
                     single seat",
                    wait.provider, wait.route
                ))))
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
        Ok(probe)
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
                    return Ok(Probe::Parked(Park::ruled(format!(
                        "no cool-down probe for {}/{}: the panel declares no member {member}",
                        wait.provider, wait.route
                    ))));
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
                Ok(probe)
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
            ProbeForm::Single(_) => Ok(Probe::Parked(Park::ruled(format!(
                "no cool-down probe for {}/{}: member {member} stands at a body that is not a \
                 panel or a sequence",
                wait.provider, wait.route
            )))),
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
            return Ok(Probe::Parked(Park::ruled(format!(
                "no cool-down probe for {}/{}: the sequence declares no step at {site}",
                at.wait.provider, at.wait.route
            ))));
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
            return Ok(Probe::Parked(Park::ruled(format!(
                "no cool-down probe for {}/{}: step {} is not a single driver",
                at.wait.provider, at.wait.route, step.name
            ))));
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
        Ok(probe)
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
            return Ok(Probe::Parked(Park::ruled(format!(
                "no cool-down probe for {}/{}: step {} is not a panel",
                at.wait.provider, at.wait.route, step.name
            ))));
        };
        let Some(member_def) = members.iter().find(|candidate| candidate.name == member) else {
            return Ok(Probe::Parked(Park::ruled(format!(
                "no cool-down probe for {}/{}: step {} declares no member {member}",
                at.wait.provider, at.wait.route, step.name
            ))));
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
        Ok(probe)
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
        deadline: ProbeDeadline,
        wait: &ProviderWait,
    ) -> Probe {
        let composed = self.composed_single(at, selection, &mut input, hands, &[]);
        let spawn = composed.expect("a single seat composed its command");
        self.spawn_probe(at, &spawn, input, deadline, wait)
    }

    /// Spawn one admitted probe's already-composed site and run it to its
    /// end: the single seat's composed spawn and the panel member's own.
    /// `deadline`, the probe's one deadline, travels to the fork as it was
    /// taken: a probe already past it composes no spawn, the fork reads it
    /// again on the engine's clock under the registry's lock as the last
    /// step before the spawn, and the watchdog kills at its own instant,
    /// never at a length sampled earlier. A probe not spawned leaves its
    /// hold standing past it (fix rounds 0106b1r5, H2, 0106b1r6, D1, and
    /// 0106b1r9, E1).
    fn spawn_probe(
        &mut self,
        at: &Dispatch<'_>,
        spawn: &SiteSpawn,
        input: Value,
        deadline: ProbeDeadline,
        wait: &ProviderWait,
    ) -> Probe {
        let ends = deadline.ends;
        if ends <= self.now() {
            return Late::park(wait, ends);
        }
        let door = self
            .held
            .as_ref()
            .map(|held| Door::of(held, at.site_name, None));
        let workdir = self.workdir();
        let now = &*self.cooldown.clock.now;
        let by = SpawnDeadline::By { deadline, now };
        let (process, input) = match spawn_site_by(&self.bundle, spawn, &input, &workdir, by, door)
        {
            Ok(started) => started,
            Err(Unspawned::Late) => return Late::park(wait, ends),
            Err(Unspawned::Refused(reason)) => {
                return Probe::Parked(Park::ruled(format!(
                    "the cool-down probe for {}/{} did not spawn: {reason}",
                    wait.provider, wait.route
                )));
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
        // The hold ends on evidence, never on a clock (fix round 0106b1r10,
        // H1): right after the settlement above proved the driver's whole
        // tree ended, for a probe that reported no refusal. A refusal's hold
        // ends with its renewed round's commit, or stands to its stored end
        // where that round is not written; a tree not proven ended, like a
        // probe never spawned, leaves it standing to that end too.
        let proven = matches!(report.cleanup, Cleanup::Settled) && report.refusal.is_none();
        let probe = self.probe_end(at.effect_id, wait, report);
        match proven {
            true => self.end_the_hold(wait, probe),
            false => probe,
        }
    }

    /// End the probe's own hold once its tree is proven ended, and go on
    /// with what the probe left. A hold the store cannot end parks the
    /// probe by the store's words: its route stays held to the stored end.
    fn end_the_hold(&mut self, wait: &ProviderWait, probe: Probe) -> Probe {
        let hold = self.cooldown.hold.take();
        match self.store.end_probe_hold(hold, self.now()) {
            Ok(()) => probe,
            Err(error) => Probe::Parked(Park::ruled(format!(
                "the cool-down probe for {}/{} ended, and its route's hold could not be ended: \
                 {error}",
                wait.provider, wait.route
            ))),
        }
    }
}

/// A rate-limited probe's next round before anything is written: the
/// terminal event it extends, the site and round it names, its length and
/// the episode through it. The wait judges the maximum with its own
/// remainders included and appends the row (decision 0103 ruling 2(a),
/// choice 3), so a round past the maximum writes nothing.
pub(super) struct NextRound {
    pub(super) event_seq: u64,
    pub(super) site: Option<String>,
    pub(super) provider: String,
    pub(super) route: String,
    pub(super) words: String,
    pub(super) retry_after_seconds: Option<u64>,
    /// The row's round: the store's next one, which the append requires.
    pub(super) rounds: u64,
    /// The site's episode round this wait is: the count its length doubles
    /// from and the round a park names. A fill at round one can be the
    /// site's second wait, so this need not equal `rounds` (finding 3).
    pub(super) round: u64,
    pub(super) seconds: u64,
    /// The episode's seconds through the round's terminal, before it.
    pub(super) episode: u64,
}

/// What a probe leaves: an answer, a rate-limit refusal's next round the
/// caller judges and writes before waiting and probing again, or the
/// park's reason.
pub(super) enum Probe {
    Answered(ProbeAnswer),
    Extended(NextRound),
    Parked(Park),
}
