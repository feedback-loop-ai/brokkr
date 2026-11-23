//! The reaper of a probe's hold (fix round 0106b1r12, S2): a hold no live
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

use brokkr_store::StoreError;

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
