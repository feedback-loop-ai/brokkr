//! How a probe's hold is written (fix round 0106b1r4, F1, and the return of
//! fix round 0106b1r12; the operator's ruling of 2026-11-18, amending
//! decision 0103 ruling 2), and what a hold the store refuses leaves: a
//! route another hold or a cool-down already holds sends the run back to
//! its wait (fix round 0106b1r13, H2); any other refusal parks the probe
//! unadmitted. Its end and its reaper are [`super::reap`]'s.

use brokkr_core::park::Park;
use brokkr_core::ProviderWait;
use brokkr_store::{ProbeHold, QueueRefusal, StoreError};
use time::OffsetDateTime;

use super::Probe;
use crate::engine::Engine;

impl Engine {
    /// Hold the probe's route once it is reserved and before it is
    /// dispatched: one hold row, written at `now`, carrying the provider,
    /// the route and the probe's `deadline`, a readout the store keeps to
    /// the first whole second strictly after it (fix round 0106b1r6, D1).
    /// Every reader, the pass, the claim and an admitted run's wait, retry
    /// and probe, reads it as cooling down until its answer or its renewed
    /// round commits, or its probe's process tree is settled; no clock or
    /// lock state ends it (fix rounds 0106b1r12, S1, and 0106b1r13, H1).
    /// The store writes none where another hold or a cool-down already holds
    /// the route, read in the write's own transaction (H2), nor one the
    /// table did not write, one whose readout cannot be stored after its
    /// deadline, or one on a queue this binary does not write, older than
    /// version 7 or a newer binary's (fix round 0106b1r9, E3). An engine's
    /// own open moves a version 4 to 6 queue to version 7 before any probe.
    /// The hold names `attempt`, the probe's own, so no later drive of the
    /// run ends or adopts it as its own (fix round 0106b1r12, S3).
    pub(super) fn hold_the_route(
        &mut self,
        (wait, attempt): (&ProviderWait, &str),
        now: OffsetDateTime,
        deadline: OffsetDateTime,
    ) -> Result<(), StoreError> {
        let hold = ProbeHold {
            run_id: &self.run_id,
            attempt_id: attempt,
            site: wait.site.as_deref().unwrap_or(""),
            provider: &wait.provider,
            route: &wait.route,
            deadline,
        };
        // Only a hold this insert returned is ever ended: one the store
        // refuses, or did not write, leaves none (fix round 0106b1r6, D2).
        self.cooldown.hold = None;
        #[cfg(test)]
        before_hold();
        self.cooldown.hold = Some(self.store.hold_probe(&hold, now)?);
        Ok(())
    }
}

/// What a hold the store refused leaves `wait`'s probe, which spawned
/// nothing: a route another probe's hold or a cool-down holds is waited on
/// again (fix round 0106b1r13, H2), and every other refusal parks it
/// unadmitted, in the store's words, so no probe runs on a route it could
/// not hold.
pub(super) fn unheld(wait: &ProviderWait, refused: &StoreError) -> Probe {
    if let StoreError::Queue(QueueRefusal::ProbeHoldContended(_)) = refused {
        return Probe::Held;
    }
    Probe::Parked(Park::ruled(format!(
        "the cool-down probe for {}/{} was not dispatched: its route's hold was not written: \
         {refused}",
        wait.provider, wait.route
    )))
}

#[cfg(test)]
thread_local! {
    /// What runs on this thread as a probe's hold is about to be written,
    /// after the wait's last read of its route: the contention test lands
    /// another run's hold there.
    static BEFORE_HOLD: std::cell::RefCell<Option<Box<dyn FnMut()>>> =
        const { std::cell::RefCell::new(None) };
}

/// Run `act` on this thread as each probe's hold is about to be written,
/// after the wait's last read of its route; `None` lifts it. Another
/// thread's probes are unmoved.
#[cfg(test)]
pub(in crate::engine::cooldown) fn before_hold_here(act: Option<Box<dyn FnMut()>>) {
    BEFORE_HOLD.with(|before| *before.borrow_mut() = act);
}

/// The act [`before_hold_here`] set on this thread, if one is set.
#[cfg(test)]
fn before_hold() {
    BEFORE_HOLD.with(|before| {
        if let Some(act) = before.borrow_mut().as_mut() {
            act();
        }
    });
}
