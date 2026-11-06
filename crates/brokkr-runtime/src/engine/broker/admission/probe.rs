//! The cool-down probe's admission (decision 0103 slice 4c1, fix round
//! 430s4c5): the attempt's own claim, sealed reach and, where the root is
//! protected, lease, but with no `effect/failed` when it is refused. It
//! lives beside admission's core so `admission.rs` stays within ruling 4's
//! 800-line ceiling.

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::*;
use crate::bundle::ExecutableBody;
use crate::engine::Selection;

/// What an attempt's admission came to: the lease it holds for the door
/// and the settlement, or a claim that proceeds holding none (C1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) enum Admission {
    Leased,
    Unreserved,
}

impl Engine {
    /// The cool-down probe's admission: [`Engine::admitted_in`]'s own core
    /// on the host `observed`, with the attempt it names, but with no
    /// failed event when it is refused (decision 0103 ruling 4: a probe
    /// spends no effect attempt). The caller parks naming the refusal
    /// instead.
    pub(in crate::engine) fn admit_for_probe(
        &mut self,
        [_, attempt, site]: [&str; 3],
        body: ExecutableBody<'_>,
        selection: &Selection,
        (began, bound): (Instant, Duration),
        observed: &Observed,
    ) -> Result<Admission, Unadmitted> {
        let (deadline, spelled, root) = observed_coordination(observed, (began, bound));
        self.admitted_in((spelled, &root), [attempt, site], body, selection, deadline)
    }

    /// The attempt's admission before `deadline`, through the claim both
    /// dispatch and the probe take: the reach it seals for every boxed
    /// writer, and, where the root is protected, the lease. Refused with
    /// the cause, holding no lease and sealing no reach.
    pub(super) fn admitted_in(
        &mut self,
        (home, root): Coordination<'_>,
        [attempt, site]: [&str; 2],
        body: ExecutableBody<'_>,
        selection: &Selection,
        deadline: Instant,
    ) -> Result<Admission, Unadmitted> {
        self.reach = Authored::default();
        let claim = self.claim(&home, [attempt, site], body, selection)?;
        self.reach = claim.authored.clone();
        let admitted = match coordinated(root(), &claim) {
            Ok(Some(root)) => admit(&Arc::new(root), &claim, deadline),
            Ok(None) => match records_unopened(&claim, &home, deadline) {
                Ok(()) => return Ok(self.unreserved()),
                Err(unknown) => Err(unknown.into()),
            },
            Err(unknown) => Err(unknown.into()),
        };
        let held = admitted?;
        self.held = Some(held);
        Ok(Admission::Leased)
    }

    /// Proceed holding no writer lease (C1): the root is not protected, nothing
    /// is at stake and every boxed writer is kept from the root's path, so the
    /// door rechecks nothing and settlement releases nothing. The absence of a
    /// reservation tells a later broker nothing: this writer may be live. No
    /// journal marker either: the seat-record fence (decision 0034) refuses a
    /// checkpoint before the driver, which would lose the attempt.
    fn unreserved(&mut self) -> Admission {
        self.held = None;
        Admission::Unreserved
    }
}
