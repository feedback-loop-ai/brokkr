//! Additional routes discovered by the retry fence have their own wait
//! bounds. They contribute no rows or probes to the effect's episode.

use std::collections::BTreeMap;

use brokkr_core::park::{Park, ParkCause, ProviderLimitRefusal};
use brokkr_core::ProviderWait;
use brokkr_store::{CooldownAggregate, RowEnd};
use time::{Duration as TimeDuration, OffsetDateTime};

use super::super::held::{bound, Bound};
use super::super::{park_of, CooldownError, Engine, SLICE};
use super::{whole, Charge};

type Route<'a> = (&'a str, &'a str);

#[derive(Debug)]
pub(in crate::engine::cooldown) struct WaitEnds {
    pub(super) recorded: OffsetDateTime,
    fenced: Vec<((String, String), RouteEnd)>,
}

impl WaitEnds {
    pub(super) fn clear_at(&self, now: OffsetDateTime) -> bool {
        self.recorded <= now && self.fenced.is_empty()
    }
}

/// The wait target and the row that gave it. A standing probe's next poll
/// is computed from the clock, while its stored deadline and words stay
/// together for the park.
#[derive(Debug)]
struct RouteEnd {
    target: OffsetDateTime,
    row: RowEnd,
}

impl RouteEnd {
    fn ordinary(row: RowEnd) -> Self {
        Self {
            target: row.until,
            row,
        }
    }

    fn of(
        (provider, route): Route<'_>,
        aggregate: CooldownAggregate,
        now: OffsetDateTime,
    ) -> Result<Self, CooldownError> {
        match aggregate {
            CooldownAggregate::Ordinary(row) => Ok(Self::ordinary(row)),
            CooldownAggregate::Held { probe, ordinary } => {
                let target =
                    now.checked_add(slice())
                        .ok_or_else(|| CooldownError::HeldPastTheClock {
                            provider: provider.to_string(),
                            route: route.to_string(),
                        })?;
                Ok(ordinary
                    .filter(|row| row.until >= target)
                    .map_or(Self { target, row: probe }, Self::ordinary))
            }
        }
    }
}

fn slice() -> TimeDuration {
    TimeDuration::seconds(SLICE.as_secs() as i64)
}

/// Observed time per additional route, retained until the retry is admitted.
/// A route absent from a sample stops accruing time at that sample.
#[derive(Default)]
pub(crate) struct FencedCharges(BTreeMap<(String, String), Charge>);

impl FencedCharges {
    pub(super) fn sample(&mut self, now: OffsetDateTime) {
        for charge in self.0.values_mut() {
            *charge = charge.to_uncovered(now);
            charge.last = None;
        }
    }
}

impl Engine {
    pub(in crate::engine::cooldown) fn cool_down_ends(
        &self,
        waits: &[ProviderWait],
        now: OffsetDateTime,
    ) -> Result<WaitEnds, CooldownError> {
        let mut ends = WaitEnds {
            recorded: waits
                .iter()
                .map(|wait| wait.until)
                .max()
                .expect("a wait to take"),
            fenced: Vec::new(),
        };
        for (key @ (provider, route), aggregate) in self.cool_down_aggregates(waits, now)? {
            let end = RouteEnd::of(key, aggregate, now)?;
            if waits
                .iter()
                .any(|wait| wait.provider == provider && wait.route == route)
            {
                ends.recorded = ends.recorded.max(end.target);
            } else {
                ends.fenced
                    .push(((provider.to_string(), route.to_string()), end));
            }
        }
        Ok(ends)
    }

    /// Judge each extra route against its own maximum. Poll in slices while
    /// there are extra routes so the end of one never postpones another's
    /// next budget check.
    pub(super) fn fenced_wait_end(
        &mut self,
        ends: &WaitEnds,
        now: OffsetDateTime,
    ) -> Result<OffsetDateTime, Box<Park>> {
        self.cooldown.fenced_charge.sample(now);
        let mut target = ends.recorded;
        for (key, end) in &ends.fenced {
            let charge = self
                .cooldown
                .fenced_charge
                .0
                .entry(key.clone())
                .or_default();
            charge.last = Some(now);
            let seconds = whole(charge.unrecorded.saturating_add(end.target - now));
            self.judge_fenced_route(key, end, seconds)?;
            target = target.max(end.target).min(now.saturating_add(slice()));
        }
        Ok(target)
    }

    fn judge_fenced_route(
        &self,
        (provider, route): &(String, String),
        end: &RouteEnd,
        seconds: u64,
    ) -> Result<(), Box<Park>> {
        let bounds = self.cooldown_bounds(provider, route).map_err(|source| {
            Box::new(park_of(&CooldownError::WaitBoundsUnreadable {
                provider: provider.clone(),
                route: route.clone(),
                source: Box::new(source),
            }))
        })?;
        let max_seconds = bounds.map_or(u64::MAX, |bounds| u64::from(bounds.max_seconds.get()));
        if let Bound::Past(_) = bound(None, seconds, 0, max_seconds) {
            return Err(Box::new(Park::new(
                format!(
                    "cool-down for {provider}/{route} would wait {seconds}s, past the declared \
                     {max_seconds}s maximum: {}",
                    end.row.reason
                ),
                ParkCause::refusal(
                    provider,
                    ProviderLimitRefusal::PastMaximum {
                        route: route.clone(),
                        until: end.row.until,
                        seconds,
                        max_seconds,
                    },
                ),
            )));
        }
        Ok(())
    }

    /// Every aggregate the cooled sites stand at, at `now`, through the
    /// gates' own in-force read: every unexpired row and every standing
    /// probe hold, another run's or an earlier drive's of this run alike.
    /// The routes a refused start of this effect found held are read with
    /// the waits' own, so the wait waits on them too, charged as any other
    /// (fix round 0106b1r15, R1). This drive's own probe never stands while
    /// its wait reads: its answer or settled end ends its hold, its renewed
    /// round's commit ends it, and any other end parks. A store refusal
    /// gives the park reason.
    pub(super) fn cool_down_aggregates<'w>(
        &'w self,
        waits: &'w [ProviderWait],
        now: OffsetDateTime,
    ) -> Result<Vec<(Route<'w>, CooldownAggregate)>, CooldownError> {
        let waited = (waits.iter()).map(|wait| (wait.provider.as_str(), wait.route.as_str()));
        let fenced = (self.cooldown.fenced.iter()).map(|(p, r)| (p.as_str(), r.as_str()));
        let mut aggregates = Vec::new();
        for (provider, route) in waited.chain(fenced) {
            #[cfg(test)]
            super::super::tests::held_start::before_route(provider, route);
            match self.store.cooldown(provider, route, now) {
                Ok(Some(aggregate)) => aggregates.push(((provider, route), aggregate)),
                Ok(None) => {}
                Err(source) => {
                    return Err(CooldownError::AggregateUnreadable {
                        provider: provider.to_string(),
                        route: route.to_string(),
                        source: Box::new(source),
                    })
                }
            }
        }
        Ok(aggregates)
    }
}
