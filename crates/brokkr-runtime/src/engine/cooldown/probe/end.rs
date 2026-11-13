//! What a probe's report leaves (decision 0103 ruling 4(a)): an answer
//! permits the retry; a typed rate-limit refusal before any answer appends
//! the next round and extends the wait, spending no effect attempt; every
//! other end parks, cleanup's refusal first. The admission's recovery
//! shares the cleanup refusal's opening.

use brokkr_core::{ProbeAnswer, ProviderWait};
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, PreSessionRefusal, RefusalClass};
use brokkr_store::{CooldownRound, RoundAppend};

use super::super::sites;
use super::super::{episode, maximum_reason, past_maximum, terminal};
use super::{Engine, Probe};

impl Engine {
    /// What the probe's report leaves: an answer permits the retry; a typed
    /// rate-limit refusal appends the next round to the terminal event's
    /// episode and answers [`Probe::Extended`], so the caller waits the new
    /// aggregate and probes again (decision 0103 ruling 2(a)); every other
    /// end parks, cleanup's refusal first (ruling 4(a)). No end here spends
    /// an effect attempt; only a rate-limit refusal writes a row.
    pub(in crate::engine::cooldown) fn probe_end(
        &mut self,
        effect_id: &str,
        wait: &ProviderWait,
        report: AttemptReport,
    ) -> Probe {
        if let Cleanup::Unresolved { reason } = &report.cleanup {
            return Probe::Parked(cleanup_failure(
                Some(&wait.provider),
                Some(&wait.route),
                &reason.to_string(),
            ));
        }
        if let Some(refusal) = &report.refusal {
            return match refusal.class() {
                RefusalClass::RateLimit => self.extend_wait(effect_id, wait, refusal),
                RefusalClass::Other => Probe::Parked(format!(
                    "the cool-down probe for {}/{} was refused: {}",
                    wait.provider,
                    wait.route,
                    refusal.words()
                )),
            };
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

    /// One typed rate-limit refusal's next round: the plan read from the
    /// record, the maximum judged first over the episode through it — the
    /// probe rounds included — and, within the maximum, the row appended at
    /// the terminal's own `(run_id, event_seq)`. The caller waits the new
    /// aggregate and probes again. The effect's attempts are untouched: no
    /// attempt event is journaled here.
    fn extend_wait(
        &mut self,
        effect_id: &str,
        wait: &ProviderWait,
        refusal: &PreSessionRefusal,
    ) -> Probe {
        let plan = match self.extension_plan(effect_id, wait, refusal) {
            Ok(plan) => plan,
            Err(reason) => return Probe::Parked(reason),
        };
        if past_maximum(plan.seconds, plan.total, plan.max) {
            let next = ProviderWait {
                site: wait.site.clone(),
                provider: wait.provider.clone(),
                route: wait.route.clone(),
                words: refusal.words().to_string(),
                retry_after_seconds: refusal.retry_after_seconds(),
                seconds: plan.seconds,
                until: self.now(),
                rounds: plan.rounds,
            };
            return Probe::Parked(maximum_reason(&next, plan.total, plan.max));
        }
        // A round within the maximum has a length at most the declared
        // maximum, a `u32`, so its end is always representable.
        let until = sites::wait_until(self.now(), plan.seconds)
            .expect("a round within the maximum has a representable end");
        let round = CooldownRound {
            run_id: &self.run_id,
            event_seq: plan.event_seq,
            site: &plan.site,
            provider: &wait.provider,
            route: &wait.route,
            until_at: until,
            reason: refusal.words(),
            retry_after_seconds: refusal.retry_after_seconds(),
            rounds: plan.rounds,
            seconds: plan.seconds,
        };
        match self.store.append_cooldown_round(round) {
            Ok(RoundAppend::Written) => Probe::Extended,
            Ok(RoundAppend::NoTable) => Probe::Parked(format!(
                "the cool-down probe for {}/{} was refused again by the provider: {}",
                wait.provider,
                wait.route,
                refusal.words()
            )),
            Err(error) => Probe::Parked(format!(
                "the cool-down probe for {}/{} could not record its next round: {error}",
                wait.provider, wait.route
            )),
        }
    }

    /// The facts of a probe's next round before anything is written: its
    /// terminal event, site, round, length and the episode through it, or
    /// the park reason a bound, the journal, the rounds or the episode
    /// cannot be read. Nothing is appended here.
    fn extension_plan(
        &mut self,
        effect_id: &str,
        wait: &ProviderWait,
        refusal: &PreSessionRefusal,
    ) -> Result<ExtensionPlan, String> {
        let bounds = match self.cooldown_bounds(&wait.provider, &wait.route) {
            Ok(Some(bounds)) => bounds,
            Ok(None) => {
                return Err(format!(
                    "the cool-down probe for {}/{} was refused again by the provider, and no \
                     cool-down is declared for it: {}",
                    wait.provider,
                    wait.route,
                    refusal.words()
                ))
            }
            Err(error) => {
                return Err(format!(
                    "the cool-down probe for {}/{} was refused again, and its bounds cannot be \
                     read: {error}: {}",
                    wait.provider,
                    wait.route,
                    refusal.words()
                ))
            }
        };
        let events = self.store.load(&self.run_id).map_err(|error| {
            format!(
                "the cool-down probe for {}/{} was refused again, and its journal cannot be \
                 read: {error}: {}",
                wait.provider,
                wait.route,
                refusal.words()
            )
        })?;
        let Some(event) = terminal(&events, effect_id) else {
            return Err(format!(
                "the cool-down probe for {}/{} was refused again with no terminal event to \
                 extend: {}",
                wait.provider,
                wait.route,
                refusal.words()
            ));
        };
        let site = wait.site.clone().unwrap_or_default();
        let rounds = self
            .store
            .cooldown_rounds(&self.run_id, event.seq, &site)
            .map_err(|error| {
                format!(
                    "the cool-down probe for {}/{} was refused again, and its rounds cannot be \
                     read: {error}: {}",
                    wait.provider,
                    wait.route,
                    refusal.words()
                )
            })?
            .iter()
            .map(|row| row.rounds)
            .max()
            .unwrap_or(0)
            .max(wait.rounds)
            .saturating_add(1);
        let retry_after = refusal.retry_after_seconds();
        let seconds = sites::wait_seconds(retry_after, bounds.default_seconds.get(), rounds);
        let max = u64::from(bounds.max_seconds.get());
        let prior = episode(&self.store, &self.run_id, &events, effect_id).map_err(|error| {
            format!(
                "the cool-down probe for {}/{} was refused again, and its episode cannot be \
                 read: {error}: {}",
                wait.provider,
                wait.route,
                refusal.words()
            )
        })?;
        Ok(ExtensionPlan {
            event_seq: event.seq,
            site,
            rounds,
            seconds,
            total: prior.1.saturating_add(seconds),
            max,
        })
    }
}

/// One probe round's facts before its row is written.
struct ExtensionPlan {
    event_seq: u64,
    site: String,
    rounds: u64,
    seconds: u64,
    total: u64,
    max: u64,
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
