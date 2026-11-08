//! The one-turn cool-down probe (decision 0103 ruling 4(a)): a dedicated
//! invocation of the cooled site's own adapter and launch path, never the
//! retried effect, and what its report leaves. Its seam is the engine's;
//! this module owns the probe so `cooldown.rs` stays within ruling 4's
//! 800-line ceiling.

use std::time::{Duration, Instant};

use brokkr_core::fold::RunState;
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderWait};
use brokkr_protocol::hands::HandsSpec;
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, RefusalClass};
use serde_json::Value;
use uuid::Uuid;

use super::{terminal, Engine, EngineError};
use crate::agents::Candidate;
use crate::bundle::{Aggregate, ExecutableBody, Invocation, PanelMember, Seat, ENGINE_VERSION};
use crate::engine::broker::{observed, settlement};
use crate::engine::dispatch::Dispatch;
use crate::engine::{chain_index, phase_of, spawn_site, Door, Selection, Site, SiteSpawn};

/// The seat a probe composes under, selected once: the body's form, the
/// case and site label, and the gate class. One selection serves both the
/// single seat's probe and a panel member's, so neither reads the seat's
/// body its own way.
struct ProbeSeat {
    seat: Seat,
    case: Option<String>,
    site: String,
    gate: bool,
    form: ProbeForm,
}

/// The selected body in the shape the probe composes.
enum ProbeForm {
    Single(Vec<Candidate>),
    Panel(Vec<PanelMember>, Aggregate),
    Other,
}

impl ProbeSeat {
    fn of(seat: &Seat, name: &str, strategy: Option<&str>) -> ProbeSeat {
        let (body, case) = seat
            .body
            .selected(strategy)
            .expect("seat input resolved the selector");
        let case = case.map(str::to_string);
        let site = site_name(name, case.as_deref());
        let gate = seat.body.selected_is_gate(strategy, seat.has_gate());
        let form = match body {
            ExecutableBody::Invocation(Invocation::Single { candidates, .. }) => {
                ProbeForm::Single(candidates.clone())
            }
            ExecutableBody::Invocation(Invocation::Panel { members, aggregate }) => {
                ProbeForm::Panel(members.clone(), *aggregate)
            }
            ExecutableBody::Sequence { .. } => ProbeForm::Other,
        };
        ProbeSeat {
            seat: seat.clone(),
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
        let picked = ProbeSeat::of(&self.bundle.seats[seat], seat, state.strategy.as_deref());
        let ProbeForm::Single(candidates) = &picked.form else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: a wait is recorded at a body that is not a \
                 single seat",
                wait.provider, wait.route
            )));
        };
        let candidates = candidates.clone();
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

    /// The cooled panel member's own one-turn probe (decision 0103 ruling
    /// 4(a), slice 4c2a): its own command and candidates, the link the
    /// refusal named, and its model pin, effort, boundary and credential
    /// facts, composed through [`Engine::member_runs`] for that one member —
    /// never the single seat's composition. It is admitted and settled
    /// without provisioning, as the single seat's probe is.
    pub(super) fn probe_member(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
        member: &str,
    ) -> Result<Probe, EngineError> {
        let picked = ProbeSeat::of(&self.bundle.seats[seat], seat, state.strategy.as_deref());
        let ProbeForm::Panel(members, aggregate) = &picked.form else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: member {member} stands at a body that is not a \
                 panel",
                wait.provider, wait.route
            )));
        };
        let Some(member_def) = members.iter().find(|candidate| candidate.name == member) else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: the panel declares no member {member}",
                wait.provider, wait.route
            )));
        };
        let tag = Some(member_def.name.clone());
        let mut selection = Selection::new();
        if !member_def.candidates.is_empty() {
            let index = refused_index(events, effect_id, &tag, member_def.candidates.len());
            selection.insert(tag, member_def.candidates[index].clone());
        }
        let one = Invocation::Panel {
            members: vec![member_def.clone()],
            aggregate: *aggregate,
        };
        let body = ExecutableBody::Invocation(&one);
        // A member's own site label is `<seat>:<member>`; the one-member
        // body makes the claim's writer walk produce exactly that.
        let prepared =
            match self.admit_probe(events, state, effect_id, wait, (&picked, body), &selection)? {
                Ok(prepared) => prepared,
                Err(parked) => return Ok(parked),
            };
        let runs = self.member_runs(
            &prepared.attempt,
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
        // `member_runs` derives this member's own driver seat, which its
        // door, `effect/started` and settlement record all name.
        let at = Self::probe_dispatch(
            effect_id,
            &prepared.attempt,
            seat,
            &run.driver_seat,
            &picked,
            body,
        );
        let probe = self.spawn_probe(&at, &run.spawn, run.input.clone(), prepared.deadline, wait);
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
                &wait.provider,
                &wait.route,
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
/// sentence (decision 0071 ruling 5: one home for the rule).
pub(in crate::engine) fn cleanup_failure(provider: &str, route: &str, detail: &str) -> String {
    format!(
        "the cool-down probe for {provider}/{route} did not prove its process tree gone, so it is \
         a cleanup failure: {detail}"
    )
}

/// What a probe leaves.
pub(super) enum Probe {
    Answered(ProbeAnswer),
    Parked(String),
}
