//! Admission's cool-down leg (decision 0103 ruling 6, slice 4e): an entry
//! whose provider and route stand in a cool-down is not admitted and not
//! held, and the claim's fence refuses one recorded after the pass read.
//!
//! The rows are the store's own read of every unexpired
//! `provider_cooldowns` row and every standing probe hold, a probe in
//! flight holding its route until it ends or its deadline passes (fix round
//! 0106b1r4) ([`brokkr_store::Store::cooldowns_in_force`]),
//! aggregated per provider and route to the latest `until_at`, with the
//! words of the latest row at that maximum (ruling 2's aggregate). A pass
//! that cannot read a row refuses; it never reads one as no cool-down.
//! Nothing here writes, and an expired row is history the pass never
//! clears.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use brokkr_store::Cooldown;
use time::OffsetDateTime;

use super::capacity::{Capacity, Lane};

/// Every unexpired row's provider and route, aggregated to the latest end
/// across them and the words of the latest row at that end.
pub(super) struct Cooling {
    providers: BTreeMap<String, BTreeMap<String, Held>>,
}

/// One lane's aggregate: when it ends, and the words of the row that ends
/// last (the later row where two end together).
struct Held {
    until: OffsetDateTime,
    words: String,
}

impl Cooling {
    /// Aggregate `rows`, in the order the store read them (rising `seq`):
    /// a later, shorter row never shortens a wait, and a row at the same
    /// end replaces the earlier one's words.
    pub(super) fn of(rows: &[Cooldown]) -> Cooling {
        let mut cooling = Cooling {
            providers: BTreeMap::new(),
        };
        for row in rows {
            let routes = cooling.providers.entry(row.provider.clone()).or_default();
            let keep = routes
                .get(&row.route)
                .is_some_and(|held| held.until > row.until_at);
            if !keep {
                routes.insert(
                    row.route.clone(),
                    Held {
                        until: row.until_at,
                        words: row.reason.clone(),
                    },
                );
            }
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
                until: held.until,
                words: held.words.clone(),
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

/// The pass's and the claim's cool-down legs for one lane, `provider` on
/// `route`, over the rows `store` holds in force at `now`: the pass's one
/// aggregate ([`Leg::Held`], as `Measure::take` reads it) and the claim's
/// own rows ([`Leg::Rows`], the same statement its transaction reads). The
/// engine's tests ask both of a row the engine wrote (fix round 0106b1r3).
#[cfg(test)]
pub(crate) fn legs_at(
    store: &brokkr_store::Store,
    now: OffsetDateTime,
    provider: &str,
    route: &str,
) -> [Result<(), Capacity>; 2] {
    let rows = store
        .cooldowns_in_force(now)
        .expect("the rows in force read");
    let lanes = BTreeSet::from([Lane {
        provider: provider.to_string(),
        route: Some(route.to_string()),
    }]);
    let pass = Leg::Held(&Cooling::of(&rows)).room(&lanes);
    let claim = Leg::Rows(&rows).room(&lanes);
    [pass, claim]
}

/// The reason's text, naming the provider, route, end and the provider's
/// words. An end no RFC 3339 text can spell is named by its whole second
/// instead, so printing a refusal never panics.
pub(super) fn word(
    f: &mut fmt::Formatter<'_>,
    provider: &str,
    route: &str,
    until: OffsetDateTime,
    words: &str,
) -> fmt::Result {
    write!(
        f,
        "provider {provider} is cooling on route {route} until {}: {words}",
        spelled(until)
    )
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
