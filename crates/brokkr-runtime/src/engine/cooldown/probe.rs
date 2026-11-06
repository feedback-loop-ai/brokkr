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
use crate::bundle::{ExecutableBody, Invocation, ENGINE_VERSION};
use crate::engine::broker::{observed, settlement};
use crate::engine::dispatch::Dispatch;
use crate::engine::{chain_index, phase_of, spawn_site, Door, Selection};

impl Engine {
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
        let seat_def = self.bundle.seats[seat].clone();
        let strategy = state.strategy.as_deref();
        let (body, case) = seat_def
            .body
            .selected(strategy)
            .expect("seat input resolved the selector");
        let ExecutableBody::Invocation(Invocation::Single { candidates, .. }) = body else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: a wait is recorded at a body that is not a \
                 single seat",
                wait.provider, wait.route
            )));
        };
        let (candidates, timeout) = (candidates.clone(), seat_def.limits.timeout_seconds);
        let gate = seat_def
            .body
            .selected_is_gate(strategy, seat_def.has_gate());
        let site_name = case
            .map(|case| format!("{seat}:{case}"))
            .unwrap_or_else(|| seat.to_string());
        let mut selection = Selection::new();
        if !candidates.is_empty() {
            let index = refused_index(events, effect_id, candidates.len());
            selection.insert(None, candidates[index].clone());
        }
        let phase = phase_of(state, "probe without a phase")?;
        let input = self.seat_input(state, &phase, effect_id)?;
        let hands = self.runtime_hands(&site_name);
        let probe_attempt = Uuid::new_v4().to_string();
        let at = Dispatch {
            effect_id,
            attempt_id: &probe_attempt,
            seat_name: seat,
            case,
            site_name: &site_name,
            body,
            gate,
        };
        let observed = observed();
        let began = Instant::now();
        let deadline = Duration::from_secs(timeout);
        if let Err(refusal) = self.admit_for_probe(
            events,
            [&probe_attempt, &site_name],
            body,
            &selection,
            (began, deadline),
            &observed,
        ) {
            return Ok(Probe::Parked(format!(
                "the cool-down probe for {}/{} was not admitted: {refusal}",
                wait.provider, wait.route
            )));
        }
        let probe = self.probe_run(&at, input, hands.as_ref(), &selection, deadline, wait);
        self.settle_probe(probe)
    }

    /// The admitted probe's one driver: composed and marked with no broker,
    /// spawned under its attempt's lease and run as a probe, its cleanup
    /// witnessed on the lease. No `begun` is read and no broker call folded:
    /// the probe serves none, so `begun` is `None` by construction and the
    /// only settlement fact that can stand is its report's cleanup.
    fn probe_run(
        &mut self,
        at: &Dispatch<'_>,
        input: Value,
        hands: Option<&HandsSpec>,
        selection: &Selection,
        deadline: Duration,
        wait: &ProviderWait,
    ) -> Probe {
        let mut input = input;
        let composed = self.composed_single(at, selection, &mut input, hands, &[]);
        let spawn = composed.expect("a single seat composed its command");
        let door = self
            .held
            .as_ref()
            .map(|held| Door::of(held, at.site_name, None));
        let workdir = self.workdir();
        let (process, input) =
            match spawn_site(&self.bundle, &spawn, &input, &workdir, deadline, door) {
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
            return Probe::Parked(format!(
                "the cool-down probe for {}/{} did not prove its process tree gone, so it is a \
                 cleanup failure: {reason}",
                wait.provider, wait.route
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
                    site: None,
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
fn refused_index(events: &[EventEnvelope], effect_id: &str, chain: usize) -> usize {
    let before = terminal(events, effect_id)
        .and_then(|last| events.iter().position(|event| event.seq == last.seq))
        .map_or(events, |at| &events[..at]);
    chain_index(before, effect_id, &None, chain)
}

/// What a probe leaves.
pub(super) enum Probe {
    Answered(ProbeAnswer),
    Parked(String),
}
