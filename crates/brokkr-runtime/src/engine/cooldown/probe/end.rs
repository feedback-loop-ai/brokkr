//! What a probe's report leaves (decision 0103 ruling 4(a)): an answer
//! permits the retry; a typed rate-limit refusal before any answer answers
//! the next round the wait judges and appends, extending the wait without
//! spending an effect attempt; every other end parks, cleanup's refusal
//! first. The admission's recovery shares the cleanup refusal's opening.

use brokkr_core::{ProbeAnswer, ProviderWait};
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, PreSessionRefusal, RefusalClass};

use super::super::sites;
use super::super::{episode_by_site, terminal, CooldownError};
use super::{Engine, NextRound, Probe};

impl Engine {
    /// What the probe's report leaves: an answer permits the retry; a typed
    /// rate-limit refusal before any answer answers the next round
    /// [`Probe::Extended`] carries, and the caller judges the maximum with
    /// its own remainders, appends the row and probes again (decision 0103
    /// ruling 2(a)); every other end parks, cleanup's refusal first (ruling
    /// 4(a)). No end here spends an effect attempt; only a rate-limit
    /// refusal can lead to a row.
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

    /// One typed rate-limit refusal's next round, read from the record but
    /// not written: the wait judges the maximum with the remainders it has
    /// already waited included, appends the row at the terminal's own
    /// `(run_id, event_seq)` and probes again (decision 0103 ruling 2(a),
    /// choice 3). The effect's attempts are untouched: no attempt event is
    /// journaled, here or by the round.
    fn extend_wait(
        &mut self,
        effect_id: &str,
        wait: &ProviderWait,
        refusal: &PreSessionRefusal,
    ) -> Probe {
        match self.extension_plan(effect_id, wait, refusal) {
            Ok(round) => Probe::Extended(round),
            Err(error) => Probe::Parked(error.to_string()),
        }
    }

    /// The facts of a probe's next round before anything is written: its
    /// terminal event, site, rounds, length and the episode through it, or
    /// the typed reason a bound, the journal, the rounds or the episode
    /// cannot be read. The row keeps the site's highest stored round plus
    /// one, the store's own requirement, so a terminal whose own row never
    /// landed is filled at round one rather than asked for round two. The
    /// length, though, comes from the episode's per-site count: a fill at
    /// round one is still the site's second wait, and the default doubles
    /// for each earlier wait of that site (decision 0103 ruling 3, finding
    /// 3). Nothing is appended here.
    fn extension_plan(
        &mut self,
        effect_id: &str,
        wait: &ProviderWait,
        refusal: &PreSessionRefusal,
    ) -> Result<NextRound, CooldownError> {
        let bounds = match self.cooldown_bounds(&wait.provider, &wait.route) {
            Ok(Some(bounds)) => bounds,
            Ok(None) => {
                return Err(CooldownError::ProbeUndeclared {
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    words: refusal.words().to_string(),
                })
            }
            Err(source) => {
                return Err(CooldownError::ProbeBoundsUnreadable {
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    source: Box::new(source),
                    words: refusal.words().to_string(),
                })
            }
        };
        let events = self.store.load(&self.run_id).map_err(|source| {
            CooldownError::ProbeJournalUnreadable {
                provider: wait.provider.clone(),
                route: wait.route.clone(),
                source: Box::new(source),
                words: refusal.words().to_string(),
            }
        })?;
        let Some(event) = terminal(&events, effect_id) else {
            return Err(CooldownError::ProbeNoTerminal {
                provider: wait.provider.clone(),
                route: wait.route.clone(),
                words: refusal.words().to_string(),
            });
        };
        let site = wait.site.clone().unwrap_or_default();
        let rounds = self
            .store
            .cooldown_rounds(&self.run_id, event.seq, &site)
            .map_err(|source| CooldownError::ProbeRoundsUnreadable {
                provider: wait.provider.clone(),
                route: wait.route.clone(),
                source: Box::new(source),
                words: refusal.words().to_string(),
            })?
            .iter()
            .map(|row| row.rounds)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let prior =
            episode_by_site(&self.store, &self.run_id, &events, effect_id).map_err(|source| {
                CooldownError::ProbeEpisodeUnreadable {
                    provider: wait.provider.clone(),
                    route: wait.route.clone(),
                    source: Box::new(source),
                    words: refusal.words().to_string(),
                }
            })?;
        let round = prior
            .rounds
            .get(&wait.site)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        let retry_after = refusal.retry_after_seconds();
        let seconds = sites::wait_seconds(retry_after, bounds.default_seconds.get(), round);
        Ok(NextRound {
            event_seq: event.seq,
            site: wait.site.clone(),
            provider: wait.provider.clone(),
            route: wait.route.clone(),
            words: refusal.words().to_string(),
            retry_after_seconds: retry_after,
            rounds,
            round,
            seconds,
            episode: prior.seconds,
        })
    }
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
