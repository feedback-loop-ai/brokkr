//! The one-turn cool-down probe (decision 0103 ruling 4(a)): a dedicated
//! invocation of the cooled site's own adapter and launch path, never the
//! retried effect, and what its report leaves. Its seam is the engine's;
//! this module owns the probe so `cooldown.rs` stays within ruling 4's
//! 800-line ceiling.

use std::time::Duration;

use brokkr_core::fold::RunState;
use brokkr_core::{EventEnvelope, ProbeAnswer, ProviderWait};
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, RefusalClass};
use uuid::Uuid;

use super::{terminal, Engine, EngineError};
use crate::bundle::{ExecutableBody, Invocation, ENGINE_VERSION};
use crate::engine::{argv_for, chain_index, phase_of, spawn_site, Door, Selection};

impl Engine {
    /// A dedicated one-turn invocation of the cooled site's own adapter and
    /// launch path (decision 0103 ruling 4(a)): never the retried effect, at
    /// the route the entry names, with the refused candidate's model pin and
    /// effort and the same boundary and credential facts.
    pub(super) fn probe(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat: &str,
        wait: &ProviderWait,
    ) -> Result<Probe, EngineError> {
        let seat_def = &self.bundle.seats[seat];
        let (body, case) = seat_def
            .body
            .selected(state.strategy.as_deref())
            .expect("seat input resolved the selector");
        let ExecutableBody::Invocation(Invocation::Single {
            command,
            candidates,
            ..
        }) = body
        else {
            return Ok(Probe::Parked(format!(
                "no cool-down probe for {}/{}: a wait is recorded at a body that is not a \
                 single seat",
                wait.provider, wait.route
            )));
        };
        let (command, candidates, timeout) = (
            command.clone(),
            candidates.clone(),
            seat_def.limits.timeout_seconds,
        );
        let gate = seat_def
            .body
            .selected_is_gate(state.strategy.as_deref(), seat_def.has_gate());
        let site_name = case
            .map(|case| format!("{seat}:{case}"))
            .unwrap_or_else(|| seat.to_string());
        let mut selection = Selection::new();
        if !candidates.is_empty() {
            let index = refused_index(events, effect_id, candidates.len());
            selection.insert(None, candidates[index].clone());
        }
        let phase = phase_of(state, "probe without a phase")?;
        let mut input = self.seat_input(state, &phase, effect_id)?;
        let hands = self.runtime_hands(&site_name);
        let probe_attempt = Uuid::new_v4().to_string();
        let mut spawn = self.compose_at(
            Some(&site_name),
            &probe_attempt,
            gate,
            argv_for(&selection, &None, &command).to_vec(),
            hands.as_ref(),
            selection.get(&None),
            input["result_path"].as_str().unwrap_or_default(),
        );
        self.mark_serving(
            &site_name,
            selection.get(&None),
            (&[], hands.as_ref()),
            Some(&mut spawn),
            &mut input,
        );
        let begun = self.begun(&spawn)?;
        let door = self
            .held
            .as_ref()
            .map(|held| Door::of(held, &site_name, begun.as_ref()));
        let deadline = Duration::from_secs(timeout);
        let workdir = self.workdir();
        let (process, input) =
            match spawn_site(&self.bundle, &spawn, &input, &workdir, deadline, door) {
                Ok(started) => started,
                Err(reason) => {
                    return Ok(Probe::Parked(format!(
                        "the cool-down probe for {}/{} did not spawn: {reason}",
                        wait.provider, wait.route
                    )));
                }
            };
        let report = process.probing().run_attempt_resuming(
            ENGINE_VERSION,
            effect_id,
            &probe_attempt,
            &site_name,
            input,
            None,
            |_| {},
        );
        let report = self.folded(
            [effect_id, &probe_attempt, &site_name],
            &spawn,
            begun,
            report,
        )?;
        Ok(self.probe_end(wait, report))
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
