//! The cool-down probe's admission (decision 0103 slice 4c1, fix round
//! 430s4c5): the attempt's own claim, sealed reach and, where the root is
//! protected, lease, but with no `effect/failed` when it is refused. It
//! lives beside admission's core so `admission.rs` stays within ruling 4's
//! 800-line ceiling.

use std::sync::Arc;
use std::time::{Duration, Instant};

use brokkr_core::{EventEnvelope, ProviderWait};

use super::*;
use crate::bundle::ExecutableBody;
use crate::engine::broker::settlement::{recorded_settled, standing};
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
        events: &[EventEnvelope],
        [attempt, site]: [&str; 2],
        body: ExecutableBody<'_>,
        selection: &Selection,
        (began, bound): (Instant, Duration),
        observed: &Observed,
    ) -> Result<Admission, Unadmitted> {
        // A run's own standing attempt that no journal event names is a
        // crashed probe's, and may still be live: this admission never
        // passes over it (decision 0103 ruling 4, this fix round).
        if let Some(unknown) = self.orphan_at(observed, events) {
            return Err(unknown.into());
        }
        let (deadline, spelled, root) = observed_coordination(observed, (began, bound));
        self.admitted_in((spelled, &root), [attempt, site], body, selection, deadline)
    }

    /// The run's own standing attempt that `events` never names, if one
    /// stands: a crashed probe's reservation. None where no coordination
    /// root is protected (C1) or nothing stands.
    fn orphan_at(&self, observed: &Observed, events: &[EventEnvelope]) -> Option<Unknown> {
        let root = observed.home.as_deref().and_then(observed.open)?;
        let (_, repo) = keyed(&self.workdir()).ok()?;
        let deadline = startup(Instant::now(), Duration::MAX);
        let _lock = locked(root.dir(), deadline).ok()?;
        let reservation = standing(&root, [&repo, &self.run_id]).ok().flatten()?;
        let attempt = orphans(reservation.attempts.into_keys(), events)
            .into_iter()
            .next()?;
        Some(Unknown::Orphaned {
            run: self.run_id.clone(),
            attempt,
        })
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

impl Engine {
    /// Before any probe (decision 0103 ruling 4, this fix round): find the
    /// run's own reservation and release every standing attempt the journal
    /// never named — a crashed cool-down probe's — once every process it
    /// witnessed is proven settled, the probe's no-terminal case the
    /// recoverable one. `Err` is the cleanup refusal to park with, naming
    /// the reservation and its attempt; nothing may probe or retry then. An
    /// unprotected root holds no reservation (C1), so nothing is at stake
    /// and a resume there may probe again.
    pub(in crate::engine) fn recovered_probe(
        &self,
        events: &[EventEnvelope],
        wait: &ProviderWait,
        observed: &Observed,
    ) -> Result<(), String> {
        let Some(root) = observed.home.as_deref().and_then(observed.open) else {
            return Ok(());
        };
        let repo = match keyed(&self.workdir()) {
            Ok((_, repo)) => repo,
            Err(unknown) => return Err(refused(wait, None, None, &unknown.to_string())),
        };
        let deadline = startup(Instant::now(), Duration::MAX);
        settle_orphaned(&root, [&repo, &self.run_id], events, wait, deadline)
    }
}

/// Whether the journal names `attempt` at all: every effect attempt is
/// journaled before it is admitted; a cool-down probe's is not.
fn named(events: &[EventEnvelope], attempt: &str) -> bool {
    events
        .iter()
        .any(|event| event.attempt_id.as_deref() == Some(attempt))
}

/// Every one of `attempts` that the journal never names: the reservation's
/// crashed probes, found by the attempts no open journaled attempt names.
fn orphans(attempts: impl Iterator<Item = String>, events: &[EventEnvelope]) -> Vec<String> {
    attempts.filter(|attempt| !named(events, attempt)).collect()
}

/// Settle or refuse every standing attempt of the run `[repo, run]` that
/// the journal never named, under the admission lock taken before
/// `deadline`: each proven settled through attestation's own judging
/// ([`recorded_settled`]) is sealed Settled and released; the first not
/// proven parks, naming the reservation and the attempt.
pub(super) fn settle_orphaned(
    root: &Root,
    [repo, run]: [&str; 2],
    events: &[EventEnvelope],
    wait: &ProviderWait,
    deadline: Instant,
) -> Result<(), String> {
    let at = root.path().join(repo).join(run).join(RESERVATION);
    let _lock = locked(root.dir(), deadline)
        .map_err(|unknown| refused(wait, Some(&at), None, &unknown.to_string()))?;
    let Some(mut reservation) = standing(root, [repo, run])
        .map_err(|unknown| refused(wait, Some(&at), None, &unknown.to_string()))?
    else {
        return Ok(());
    };
    let orphans = orphans(reservation.attempts.keys().cloned(), events);
    let Some(first) = orphans.first() else {
        return Ok(());
    };
    for attempt in &orphans {
        let reserved = &reservation.attempts[attempt];
        let proven = recorded_settled(reserved);
        if !proven {
            return Err(refused(
                wait,
                Some(&at),
                Some(attempt),
                &witnessed(reserved),
            ));
        }
    }
    for attempt in &orphans {
        if let Some(reserved) = reservation.attempts.get_mut(attempt) {
            reserved.cleanup = Witnessed::Settled;
        }
    }
    for attempt in &orphans {
        reservation = settled(reservation, attempt)
            .expect("a proven probe's attempt holds its positive settlement fact");
    }
    sealed(root, &reservation, &Budget::new(deadline))
        .map(drop)
        .map_err(|unknown| refused(wait, Some(&at), Some(first), &unknown.to_string()))
}

/// Why a crashed probe's reservation is not proven settled: its cleanup
/// unresolved, no driver recorded, or a driver not sealed Settled. Its
/// state is named in words, never as a `Debug` rendering.
fn witnessed(reserved: &Reserved) -> String {
    if reserved.cleanup == Witnessed::Unresolved {
        return "its cleanup is unresolved".to_string();
    }
    let unsettled: Vec<String> = reserved
        .drivers
        .iter()
        .filter(|driver| driver.cleanup != Witnessed::Settled)
        .map(|driver| format!("'{}'", driver.site))
        .collect();
    match unsettled.is_empty() {
        true => "it recorded no driver, so no process is proven settled".to_string(),
        false => format!(
            "its driver at site {} is not sealed Settled",
            unsettled.join(", ")
        ),
    }
}

/// The cleanup refusal a crashed probe's reservation parks with: ruling
/// 4's wording first, then the reservation, its attempt where one is
/// named, and why it is not proven settled.
fn refused(
    wait: &ProviderWait,
    at: Option<&std::path::Path>,
    attempt: Option<&str>,
    why: &str,
) -> String {
    let held = match (at, attempt) {
        (Some(at), Some(attempt)) => {
            format!(
                "the reservation at {} names attempt '{attempt}'",
                at.display()
            )
        }
        (Some(at), None) => format!("the reservation at {} cannot be read", at.display()),
        (None, _) => "its reservation cannot be located".to_string(),
    };
    format!(
        "the cool-down probe for {}/{} did not prove its process tree gone, so it is a cleanup \
         failure: {held}, and {why}; no probe or retry may start until it is proven settled",
        wait.provider, wait.route
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::broker::settlement::Driver;

    fn reserved(cleanup: Witnessed, drivers: Vec<(&str, Witnessed)>) -> Reserved {
        Reserved {
            protects: Protects::Nothing,
            writers: Vec::new(),
            cleanup,
            serves: Vec::new(),
            drivers: drivers
                .into_iter()
                .map(|(site, cleanup)| Driver {
                    site: site.to_string(),
                    begun: None,
                    cleanup,
                })
                .collect(),
        }
    }

    /// Why a crashed probe is not proven settled is said in words: an
    /// unresolved cleanup, no recorded driver, or the site of each driver
    /// not sealed Settled, never a `Debug` rendering.
    #[test]
    fn a_crashed_probes_witness_names_its_state_in_words() {
        assert_eq!(
            witnessed(&reserved(Witnessed::Unresolved, Vec::new())),
            "its cleanup is unresolved"
        );
        assert_eq!(
            witnessed(&reserved(Witnessed::Settling, Vec::new())),
            "it recorded no driver, so no process is proven settled"
        );
        assert_eq!(
            witnessed(&reserved(
                Witnessed::Settling,
                vec![("work", Witnessed::Settling)]
            )),
            "its driver at site 'work' is not sealed Settled"
        );
        assert_eq!(
            witnessed(&reserved(
                Witnessed::Settling,
                vec![
                    ("work", Witnessed::Settled),
                    ("work:chore", Witnessed::Unresolved)
                ]
            )),
            "its driver at site 'work:chore' is not sealed Settled"
        );
    }
}
