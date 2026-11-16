//! A run's provider waits, derived once for every readout (decision 0103
//! ruling 2(a), slice 4f; decision 0071 ruling 7).
//!
//! The terminal event carries the attempt's waits; the store's aggregate
//! carries every later round a probe appended, and its `until` for that
//! provider and route is the end the readout paints. A later, shorter row
//! never shortens one: the aggregate is a MAX. A wait is not a park, so a
//! run standing in one reads `running`, never "needs you", and the
//! staleness bound waits for the latest end before it may call the run
//! dead. An entry the event's own reader refuses is named here, never read
//! as no wait.

use brokkr_core::fold::RunState;
use brokkr_core::stamp::parse_millis;
use brokkr_core::EventEnvelope;
use brokkr_core::{provider_waits, ProviderWait, ProviderWaitError};
use serde::Serialize;

use crate::RunView;

/// One cool-down the store's aggregate holds for a provider and route, as
/// a reader hands it in: the instant the row that ends last writes.
/// `brokkr-view` is pure and opens no store; its caller reads the store's
/// `cooldowns_newest` and passes the rows here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cooling {
    pub provider: String,
    pub route: String,
    /// RFC 3339, written by brokkr-core's one stamp writer.
    pub until: String,
}

/// The last event's optional `waits`, typed by brokkr-core's one reader,
/// or the refusal that reader gave. The fleet's entry and the run view
/// both carry this, so one derivation parses and paints it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum WaitFact {
    /// The last event carries no wait.
    #[default]
    NoWait,
    /// The last event's waits, as the extension read them.
    Waits(Vec<ProviderWait>),
    /// A present `waits` entry that did not read: the field it named.
    Unreadable(ProviderWaitError),
}

impl WaitFact {
    /// The waits `event` carries. `None` is no event, so no wait.
    pub fn of(event: Option<&EventEnvelope>) -> WaitFact {
        let Some(event) = event else {
            return WaitFact::NoWait;
        };
        match provider_waits(event) {
            Ok(Some(waits)) => WaitFact::Waits(waits),
            Ok(None) => WaitFact::NoWait,
            Err(error) => WaitFact::Unreadable(error),
        }
    }

    /// The waits the applied trail's last event states, or no wait where
    /// the fold applied none. The one home every surface reads: the trail's
    /// last applied event is `crate::live::last_applied`'s to name, so a
    /// repaired tail's kept event stands no wait (decision 0108 ruling 4),
    /// and a fold that refused stands none either, because it applied no
    /// trail to claim a wait from.
    pub fn applied(events: &[EventEnvelope], state: Option<&RunState>) -> WaitFact {
        WaitFact::of(crate::live::last_applied(events, state))
    }
}

/// One wait a run stands in: its provider, route and site, the end the
/// aggregate gives it, and the provider's own words.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Wait {
    /// The invocation-site tag; `None` for an inline seat.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    pub provider: String,
    pub route: String,
    /// The end the readout paints, RFC 3339.
    pub until: String,
    /// The provider's bounded words.
    pub words: String,
}

impl Wait {
    /// Whether the wait's end has passed at `now`. A clock or an end that
    /// does not read never ends a wait.
    fn ended(&self, now: &str) -> bool {
        match (parse_millis(now), parse_millis(&self.until)) {
            (Some(now), Some(until)) => until <= now,
            _ => false,
        }
    }

    /// The line every surface prints for this wait: provider, route, site,
    /// end and the provider's words, and `(ended)` once the end has passed.
    fn line(&self, now: &str) -> String {
        let site = match &self.site {
            Some(site) => format!(" at {site}"),
            None => String::new(),
        };
        let ended = match self.ended(now) {
            true => " (ended)",
            false => "",
        };
        format!(
            "wait      {}/{}{} until {}{}: {}",
            self.provider, self.route, site, self.until, ended, self.words
        )
    }
}

/// A run's waits as every readout paints them. `refused` is a `waits`
/// entry the view could not read, named by the field core refused; it is
/// never read as no wait.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Waits {
    pub waits: Vec<Wait>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

impl Waits {
    /// The waits `fact` states, each ended at the aggregate's `until` for
    /// its provider and route when `cooling` holds a later one, else at
    /// the event's own `until`.
    pub fn of(fact: &WaitFact, cooling: &[Cooling]) -> Waits {
        match fact {
            WaitFact::NoWait => Waits::default(),
            WaitFact::Unreadable(error) => Waits::refused(error),
            WaitFact::Waits(waits) => match painted(waits, cooling) {
                Ok(waits) => Waits {
                    waits,
                    refused: None,
                },
                Err(error) => Waits::refused(&error),
            },
        }
    }

    /// A waits collection that paints one refusal and no wait: a present
    /// entry the view cannot read is never read as no wait.
    fn refused(error: &ProviderWaitError) -> Waits {
        Waits {
            waits: Vec::new(),
            refused: Some(error.to_string()),
        }
    }

    /// The lines every surface prints, in the event's site order, with a
    /// refused entry's own line last.
    pub fn lines(&self, now: &str) -> Vec<String> {
        let refused = self
            .refused
            .iter()
            .map(|refused| format!("wait      refused: {refused}"));
        self.waits
            .iter()
            .map(|wait| wait.line(now))
            .chain(refused)
            .collect()
    }

    /// Whether any wait's end has not passed at `now`: a run standing in
    /// one is not called dead by the staleness bound.
    pub fn live(&self, now: &str) -> bool {
        self.waits.iter().any(|wait| !wait.ended(now))
    }
}

/// Every event wait painted, or the first whose end core cannot write.
fn painted(waits: &[ProviderWait], cooling: &[Cooling]) -> Result<Vec<Wait>, ProviderWaitError> {
    waits.iter().map(|wait| wait_of(wait, cooling)).collect()
}

/// One event wait, its end written by brokkr-core and taken from the
/// aggregate's latest `until` for its provider and route where that is
/// later than the event's own.
fn wait_of(wait: &ProviderWait, cooling: &[Cooling]) -> Result<Wait, ProviderWaitError> {
    let until = wait.until_stamp()?;
    let end = cooling
        .iter()
        .filter(|cooling| cooling.provider == wait.provider && cooling.route == wait.route)
        .filter_map(|cooling| parse_millis(&cooling.until).map(|at| (at, &cooling.until)))
        .max_by_key(|(at, _)| *at)
        .filter(|(at, _)| *at > parse_millis(&until).unwrap_or(i64::MIN))
        .map_or(until, |(_, until)| until.clone());
    Ok(Wait {
        site: wait.site.clone(),
        provider: wait.provider.clone(),
        route: wait.route.clone(),
        until: end,
        words: wait.words.clone(),
    })
}

#[cfg(test)]
mod tests;

/// [`run_view`](crate::run_view), its waits given the aggregate `cooling`
/// the caller read: a probe's later round paints the later end (decision
/// 0103 slice 4f).
pub fn run_view_with(
    events: &[EventEnvelope],
    state: Option<&RunState>,
    cooling: &[Cooling],
) -> RunView {
    let mut view = crate::run_view(events, state);
    view.waits = Waits::of(&WaitFact::applied(events, state), cooling);
    view
}
