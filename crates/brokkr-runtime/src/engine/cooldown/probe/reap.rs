//! How a probe's hold ends, and its reaper (fix round 0106b1r12). A live
//! engine ends its own probe's hold once the probe's tree is proven over
//! ([`Engine::end_the_hold`], [`Engine::end_own_hold`]), and a renewed
//! round ends it in its own transaction.
//!
//! The reaper (S2): a hold no live
//! engine ended, its engine crashed or its probe's tree left unproven,
//! stands until a settlement proves that tree over. A restarted engine's
//! reaper is the recovery that runs before any probe of the run
//! ([`Engine::recovered_probe`]): it seals a crashed probe's attempt
//! Settled and releases it only where every driver it recorded is sealed
//! Settled, the record writer admission reads. Each attempt the run's own
//! reservation then names settled is proven over, by that recovery, by its
//! own engine or by the operator's attestation, and every hold of the run
//! naming one of them ends here. A hold whose attempt no record proves
//! stands, its route cooled, until a later reaper or an operator's settle
//! record settles it: the trade decision 0106's slice text states.

use brokkr_core::park::Park;
use brokkr_core::ProviderWait;
use brokkr_store::StoreError;

use super::Probe;
use crate::engine::broker::Observed;
use crate::engine::Engine;

/// Why the reaper ended no hold: the store refused, in its own words. The
/// holds stand, and the caller parks with these words.
#[derive(Debug, thiserror::Error)]
#[error(
    "the cool-down probe holds of run {run} whose attempts are settled could not be ended: \
     {source}; their routes stay held, and no probe or retry may start"
)]
pub(in crate::engine) struct Unreaped {
    run: String,
    #[source]
    source: StoreError,
}

impl Engine {
    /// End the probe's own hold once its tree is proven ended, and go on
    /// with what the probe left. A hold the store cannot end parks the
    /// probe by the store's words: its route stays held until a settlement
    /// ends it.
    pub(in crate::engine::cooldown) fn end_the_hold(
        &mut self,
        wait: &ProviderWait,
        probe: Probe,
    ) -> Probe {
        let unended = self.end_own_hold(&wait.provider, &wait.route);
        unended.map_or(probe, Probe::Parked)
    }

    /// End the probe's own hold, if it still holds one, on `provider` and
    /// `route`: the one end both a probe's settled end and a renewal its
    /// wait parks before take (fix round 0106b1r12, S5). `Some` is the
    /// park a hold the store cannot end gives, its route held until a
    /// settlement ends it.
    pub(in crate::engine::cooldown) fn end_own_hold(
        &mut self,
        provider: &str,
        route: &str,
    ) -> Option<Park> {
        let hold = self.cooldown.hold.take();
        let ended = self.store.end_probe_hold(hold, self.now());
        ended.err().map(|error| {
            Park::ruled(format!(
                "the cool-down probe for {provider}/{route} ended, and its route's hold could not \
                 be ended: {error}"
            ))
        })
    }

    /// End every probe hold of this run whose attempt the run's own
    /// reservation names settled, as `observed` resolves its coordination
    /// root. A store that cannot end them refuses in its own words.
    pub(in crate::engine) fn reap_probe_holds(
        &mut self,
        observed: &Observed,
    ) -> Result<(), Unreaped> {
        let settled = self.settled_attempts(observed);
        let now = self.now();
        let reaped = self.store.settle_probe_holds(&self.run_id, &settled, now);
        reaped.map_err(|source| Unreaped {
            run: self.run_id.clone(),
            source,
        })
    }
}
