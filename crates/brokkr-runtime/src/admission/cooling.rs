//! Admission's cool-down leg (decision 0103 ruling 6, slice 4e): an entry
//! whose provider and route stand in a cool-down is not admitted and not
//! held, and the claim's fence refuses one recorded after the pass read.
//!
//! The rows are the store's own read of every unexpired
//! `provider_cooldowns` row and every standing probe hold, a probe holding
//! its route until its answer or renewed round commits or its process tree
//! is settled, never by a clock or a lock (fix rounds 0106b1r4 and
//! 0106b1r12) ([`brokkr_store::Store::cooldowns_in_force`]),
//! aggregated per provider and route to the latest `until_at`, with the
//! words of the latest row at that maximum (ruling 2's aggregate). A pass
//! that cannot read a row refuses; it never reads one as no cool-down.
//! Nothing here writes, and an expired row is history the pass never
//! clears.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use brokkr_store::{Cooldown, CooldownAggregate, RowEnd};
use time::OffsetDateTime;

use super::capacity::{Capacity, Lane};

/// Every unexpired row's provider and route, aggregated to the latest end
/// across them and the words of the latest row at that end.
pub(super) struct Cooling {
    providers: BTreeMap<String, BTreeMap<String, CooldownAggregate>>,
}

impl Cooling {
    /// Aggregate `rows`, in the order the store read them (rising `seq`),
    /// by the store's one fold of a route's latest end
    /// ([`CooldownAggregate::joined`]), so the gates and the engine never
    /// read one route two ways (fix round 0106b1r14, L1).
    pub(super) fn of(rows: &[Cooldown]) -> Cooling {
        let mut cooling = Cooling {
            providers: BTreeMap::new(),
        };
        for row in rows {
            let routes = cooling.providers.entry(row.provider.clone()).or_default();
            let held = routes.remove(&row.route);
            routes.insert(row.route.clone(), CooldownAggregate::joined(held, row));
        }
        cooling
    }

    /// The first of `lanes` a row holds now, or none: a lane with no route
    /// has no cool-down, and a provider-wide hold is never invented.
    pub(super) fn room(&self, lanes: &BTreeSet<Lane>) -> Result<(), Capacity> {
        for lane in lanes {
            let Some(route) = &lane.route else { continue };
            let Some(held) = self
                .providers
                .get(&lane.provider)
                .and_then(|routes| routes.get(route))
            else {
                continue;
            };
            return Err(Capacity::CoolingDown {
                provider: lane.provider.clone(),
                route: route.clone(),
                standing: Box::new(held.clone()),
            });
        }
        Ok(())
    }
}

/// The cool-down rows a pass or a claim judges: the pass's one aggregate,
/// or the rows a claim's own transaction read.
pub(super) enum Leg<'a> {
    Rows(&'a [Cooldown]),
    Held(&'a Cooling),
}

impl Leg<'_> {
    /// The first lane `lanes` holds.
    pub(super) fn room(&self, lanes: &BTreeSet<Lane>) -> Result<(), Capacity> {
        match self {
            Leg::Rows(rows) => Cooling::of(rows).room(lanes),
            Leg::Held(cooling) => cooling.room(lanes),
        }
    }
}

/// The reason's text, naming the provider, route, each end and the words
/// of the row that end came from. An end no RFC 3339 text can spell is
/// named by its whole second instead, so printing a refusal never panics.
/// A probe's hold says no time ends it, for only its answer, renewed round
/// or settlement does (the return of fix round 0106b1r12), and names its
/// own time as the second its probe's deadline falls before: the store
/// keeps the first whole second strictly after the deadline, so a deadline
/// at an exact second is never named as a second later (fix round
/// 0106b1r13, L1). An ordinary row beside the hold is named after it with
/// its own end and words, so a probe's words never stand for an ordinary
/// row's end, nor the reverse (fix round 0106b1r15, L1).
pub(super) fn word(
    f: &mut fmt::Formatter<'_>,
    (provider, route): (&str, &str),
    standing: &CooldownAggregate,
) -> fmt::Result {
    match standing {
        CooldownAggregate::Ordinary(RowEnd { until, reason }) => write!(
            f,
            "provider {provider} is cooling on route {route} until {}: {reason}",
            spelled(*until)
        ),
        CooldownAggregate::Held { probe, ordinary } => {
            write!(
                f,
                "provider {provider} is held on route {route} by a cool-down probe whose \
                 deadline falls before {}, and no time ends that hold: {}",
                spelled(probe.until),
                probe.reason
            )?;
            match ordinary {
                Some(RowEnd { until, reason }) => write!(
                    f,
                    "; a cool-down beside it holds the route until {}: {reason}",
                    spelled(*until)
                ),
                None => Ok(()),
            }
        }
    }
}

/// `at` as RFC 3339, written by core's one stamp writer (the readouts'
/// too, decision 0103 slice 4f), or words naming an end that text cannot
/// spell: a stored second outside RFC 3339's range (year 10000, under
/// `time`'s `large-dates`) is printed by its second, never by an `expect`
/// that would panic while a reason is shown. A held wait's refusal names
/// its end with the same words (decision 0106 ruling 2).
pub(crate) fn spelled(at: OffsetDateTime) -> String {
    brokkr_core::stamp::write_millis(at).unwrap_or_else(|_| {
        format!(
            "a time RFC 3339 cannot spell (second {})",
            at.unix_timestamp()
        )
    })
}

#[cfg(test)]
mod tests;
