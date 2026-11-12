//! The cool-down probe's admission (decision 0103 slice 4c1, fix round
//! 430s4c5): the attempt's own claim, sealed reach and, where the root is
//! protected, lease, but with no `effect/failed` when it is refused. It
//! lives beside admission's core so `admission.rs` stays within ruling 4's
//! 800-line ceiling.

use std::sync::Arc;
use std::time::{Duration, Instant};

use brokkr_core::{EventEnvelope, ProviderWait};
use thiserror::Error;

use super::*;
use crate::bundle::ExecutableBody;
use crate::engine::broker::settlement::{recorded_settled, standing};
use crate::engine::cooldown::cleanup_failure;
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
    /// the reservation, its attempt and the live attempt's own route;
    /// nothing may probe or retry then. An unprotected root holds no
    /// reservation (C1), so nothing is at stake and a resume there may probe
    /// again.
    pub(in crate::engine) fn recovered_probe(
        &self,
        label: &str,
        events: &[EventEnvelope],
        waits: &[ProviderWait],
        observed: &Observed,
    ) -> Result<(), Recovery> {
        let Some(root) = observed.home.as_deref().and_then(observed.open) else {
            return Ok(());
        };
        let (_, repo) =
            keyed(&self.workdir()).map_err(|source| Recovery::at(None, Held::Unlocated(source)))?;
        let deadline = startup(Instant::now(), Duration::MAX);
        settle_orphaned(&root, [&repo, &self.run_id], label, events, waits, deadline)
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

/// Whether the reserved driver `driver` is exactly the one the wait at
/// `site` named under `label`: `<label>` itself for a site-less wait, and
/// `<label>:<site>` for a tagged one. A suffix match is not identity: the
/// members `b` and `x:b` both name `work:x:b` by suffix, and the cleanup
/// refusal would then name the wrong member's provider and route. The
/// label is the selected site label (`<seat>` or `<seat>:<case>`), so a
/// driver that merely shares the seat's prefix is not a match either. An
/// absent match leaves the refusal neutral (decision 0103 ruling 4).
fn names_site(site: Option<&str>, driver: &str, label: &str) -> bool {
    match site {
        None => driver == label,
        Some(site) => driver == format!("{label}:{site}"),
    }
}

/// The wait the standing attempt's own driver names: a driver at
/// `<label>:<site>` names the wait at `<site>`, and a driver at `<label>`
/// names the site-less wait (`recorded_waits` refuses one beside another).
/// The label is the selected site label (`<seat>` or `<seat>:<case>`), so
/// an unrelated driver leaves the refusal neutral rather than borrowing
/// another member's route (decision 0103 ruling 4).
fn attempt_wait<'a>(
    waits: &'a [ProviderWait],
    reserved: &Reserved,
    label: &str,
) -> Option<&'a ProviderWait> {
    reserved.drivers.iter().find_map(|driver| {
        waits
            .iter()
            .find(|wait| names_site(wait.site.as_deref(), &driver.site, label))
    })
}

/// Settle or refuse every standing attempt of the run `[repo, run]` that
/// the journal never named, under the admission lock taken before
/// `deadline`: each proven settled through attestation's own judging
/// ([`recorded_settled`]) is sealed Settled and released; the first not
/// proven parks, naming the reservation, the attempt and that attempt's own
/// route.
pub(super) fn settle_orphaned(
    root: &Root,
    [repo, run]: [&str; 2],
    label: &str,
    events: &[EventEnvelope],
    waits: &[ProviderWait],
    deadline: Instant,
) -> Result<(), Recovery> {
    let at = root.path().join(repo).join(run).join(RESERVATION);
    let _lock = locked(root.dir(), deadline).map_err(|source| {
        Recovery::at(
            None,
            Held::Unreadable {
                at: at.clone(),
                source,
            },
        )
    })?;
    let Some(mut reservation) = standing(root, [repo, run]).map_err(|source| {
        Recovery::at(
            None,
            Held::Unreadable {
                at: at.clone(),
                source,
            },
        )
    })?
    else {
        return Ok(());
    };
    let orphans = orphans(reservation.attempts.keys().cloned(), events);
    let Some(first) = orphans.first() else {
        return Ok(());
    };
    for attempt in &orphans {
        let reserved = &reservation.attempts[attempt];
        if !recorded_settled(reserved) {
            return Err(Recovery::at(
                attempt_wait(waits, reserved, label),
                Held::Unsettled {
                    at: at.clone(),
                    attempt: attempt.clone(),
                    why: witnessed(reserved),
                },
            ));
        }
    }
    // Every orphan is one of the reservation's own attempts, so mark it
    // through a pass over them: a key lookup would have a miss nothing
    // can reach, the branch the exact gate counted (decision 0071
    // ruling 9).
    let wait = attempt_wait(waits, &reservation.attempts[first.as_str()], label);
    let held = reservation
        .attempts
        .iter_mut()
        .filter(|(attempt, _)| orphans.contains(attempt));
    for (_, reserved) in held {
        reserved.cleanup = Witnessed::Settled;
    }
    for attempt in &orphans {
        reservation = settled(reservation, attempt)
            .expect("a proven probe's attempt holds its positive settlement fact");
    }
    sealed(root, &reservation, &Budget::new(deadline))
        .map(drop)
        .map_err(|source| {
            Recovery::at(
                wait,
                Held::Unsealed {
                    at: at.clone(),
                    attempt: first.clone(),
                    source,
                },
            )
        })
}

/// Ruling 4's cleanup refusal a crashed probe's reservation parks with:
/// the probe it names when the reservation's own attempt resolves one, and
/// what the reservation holds — where it could not be found, read or
/// sealed, or the attempt whose process tree is not proven settled. The
/// admission boundary renders it to the park reason; nothing may probe or
/// retry while it stands (decision 0071 ruling 8).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error(
    "{}; no probe or retry may start until it is proven settled",
    cleanup_failure(self.provider.as_deref(), self.route.as_deref(), &self.held.to_string())
)]
pub(in crate::engine) struct Recovery {
    pub(in crate::engine) provider: Option<String>,
    pub(in crate::engine) route: Option<String>,
    pub(in crate::engine) held: Box<Held>,
}

impl Recovery {
    fn at(wait: Option<&ProviderWait>, held: Held) -> Recovery {
        Recovery {
            provider: wait.map(|wait| wait.provider.clone()),
            route: wait.map(|wait| wait.route.clone()),
            held: Box::new(held),
        }
    }
}

/// What a cleanup refusal holds: the reservation's path where it could not
/// be read or sealed, the attempt where one is named, and the cause.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Held {
    #[error("its reservation cannot be located, and {0}")]
    Unlocated(#[source] Unknown),
    #[error("the reservation at {} cannot be read, and {source}", .at.display())]
    Unreadable {
        at: std::path::PathBuf,
        #[source]
        source: Unknown,
    },
    #[error("the reservation at {} names attempt '{attempt}', and {why}", .at.display())]
    Unsettled {
        at: std::path::PathBuf,
        attempt: String,
        why: Unproven,
    },
    #[error("the reservation at {} names attempt '{attempt}', and {source}", .at.display())]
    Unsealed {
        at: std::path::PathBuf,
        attempt: String,
        #[source]
        source: Unknown,
    },
}

/// Why a crashed probe's standing attempt is not proven settled: its
/// cleanup unresolved, no driver recorded, or each driver not sealed
/// Settled, named by its site. Never a `Debug` rendering.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(in crate::engine) enum Unproven {
    #[error("its cleanup is unresolved")]
    Cleanup,
    #[error("it recorded no driver, so no process is proven settled")]
    NoDriver,
    #[error("its driver at site {0} is not sealed Settled")]
    Driver(String),
}

/// Why a crashed probe's reservation is not proven settled: its cleanup
/// unresolved, no driver recorded, or a driver not sealed Settled.
fn witnessed(reserved: &Reserved) -> Unproven {
    if reserved.cleanup == Witnessed::Unresolved {
        return Unproven::Cleanup;
    }
    let unsettled: Vec<String> = reserved
        .drivers
        .iter()
        .filter(|driver| driver.cleanup != Witnessed::Settled)
        .map(|driver| format!("'{}'", driver.site))
        .collect();
    if unsettled.is_empty() {
        Unproven::NoDriver
    } else {
        Unproven::Driver(unsettled.join(", "))
    }
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

    #[test]
    fn names_site_matches_a_seat_case_and_site_exactly() {
        assert!(names_site(None, "work", "work"));
        assert!(!names_site(None, "work:fast", "work"));
        assert!(!names_site(None, "work:fast:draft", "work"));
        assert!(!names_site(Some("draft"), "work", "work"));
        assert!(names_site(Some("draft"), "work:draft", "work"));
        assert!(!names_site(Some("draft"), "work:fast:draft", "work"));
        assert!(!names_site(Some("draft"), "other:draft", "work"));
        assert!(!names_site(Some("draft"), "work:other", "work"));
        assert!(!names_site(Some("draft"), "work:a:b:draft", "work"));
        // The label is exact: a segment between the seat and the site is
        // not a selector case.
        assert!(!names_site(Some("draft"), "work:review:draft", "work"));
        assert!(!names_site(None, "work:b", "work"));
        // A cased seat's label carries its case.
        assert!(names_site(None, "work:fast", "work:fast"));
        assert!(names_site(Some("draft"), "work:fast:draft", "work:fast"));
        assert!(!names_site(Some("draft"), "work:draft", "work:fast"));
    }

    /// Why a crashed probe is not proven settled is said in words: an
    /// unresolved cleanup, no recorded driver, or the site of each driver
    /// not sealed Settled, never a `Debug` rendering.
    #[test]
    fn a_crashed_probes_witness_names_its_state_in_words() {
        let cleanup = witnessed(&reserved(Witnessed::Unresolved, Vec::new()));
        assert_eq!(cleanup, Unproven::Cleanup);
        assert_eq!(cleanup.to_string(), "its cleanup is unresolved");

        let no_driver = witnessed(&reserved(Witnessed::Settling, Vec::new()));
        assert_eq!(no_driver, Unproven::NoDriver);
        assert_eq!(
            no_driver.to_string(),
            "it recorded no driver, so no process is proven settled"
        );

        let one = witnessed(&reserved(
            Witnessed::Settling,
            vec![("work", Witnessed::Settling)],
        ));
        assert_eq!(one, Unproven::Driver("'work'".into()));
        assert_eq!(
            one.to_string(),
            "its driver at site 'work' is not sealed Settled"
        );

        let two = witnessed(&reserved(
            Witnessed::Settling,
            vec![
                ("work", Witnessed::Settled),
                ("work:chore", Witnessed::Unresolved),
            ],
        ));
        assert_eq!(two, Unproven::Driver("'work:chore'".into()));
        assert_eq!(
            two.to_string(),
            "its driver at site 'work:chore' is not sealed Settled"
        );
    }
}
