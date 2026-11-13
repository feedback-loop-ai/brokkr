//! What a probe's report leaves (decision 0103 ruling 4(a)): an answer
//! permits the retry, and every other end parks, cleanup's refusal first.
//! The admission's recovery shares the cleanup refusal's opening.

use brokkr_core::{ProbeAnswer, ProviderWait};
use brokkr_protocol::{AttemptOutcome, AttemptReport, Cleanup, RefusalClass};

use super::{Engine, Probe};

impl Engine {
    /// What the probe's report leaves: an answer permits the retry, and
    /// every other end parks, cleanup's refusal first (decision 0103
    /// ruling 4(a)). A typed rate-limit refusal parks in 4c1 because the
    /// table that records its next round is 4d's.
    pub(in crate::engine::cooldown) fn probe_end(
        &self,
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
