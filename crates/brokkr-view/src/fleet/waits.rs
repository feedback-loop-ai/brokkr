//! A run's provider waits, derived once for every readout (decision 0103
//! ruling 2(a), slice 4f; decision 0071 ruling 7).
//!
//! The terminal event carries the attempt's waits; the store's aggregate
//! carries every later round a probe appended, and its `until` for that
//! provider and route is the end the readout paints. A later, shorter row
//! never shortens one: the aggregate is a MAX. A wait is not a park, so a
//! run standing in one reads `waiting`, and the
//! staleness bound waits for the latest end before it may call the run
//! dead. An entry the event's own reader refuses is named here, never read
//! as no wait.

use std::collections::BTreeMap;

use brokkr_core::fold::RunState;
use brokkr_core::stamp::parse_millis;
use brokkr_core::EventEnvelope;
use brokkr_core::{provider_waits, Held, ProviderWait, ProviderWaitError};
use serde::Serialize;

use super::{RunRow, Standing};

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
    /// The last event's waits and the store's last round per recorded site.
    Waits {
        event_seq: u64,
        sites: Vec<ProviderWait>,
        rounds: BTreeMap<Option<String>, WaitRound>,
    },
    /// A refusal reading the event's waits or its stored rounds.
    Unreadable(WaitRefusal),
}

impl WaitFact {
    /// The waits `event` carries. `None` is no event, so no wait.
    pub fn of(event: Option<&EventEnvelope>) -> WaitFact {
        let Some(event) = event else {
            return WaitFact::NoWait;
        };
        match provider_waits(event) {
            Ok(Some(sites)) => WaitFact::Waits {
                event_seq: event.seq,
                sites,
                rounds: BTreeMap::new(),
            },
            Ok(None) => WaitFact::NoWait,
            Err(error) => WaitFact::Unreadable(WaitRefusal::Event(error)),
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

/// A refusal on the event payload or the stored wait records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WaitRefusal {
    Event(ProviderWaitError),
    Store(String),
}

impl WaitRefusal {
    fn words(&self) -> String {
        match self {
            WaitRefusal::Event(error) => error.to_string(),
            WaitRefusal::Store(words) => words.clone(),
        }
    }
}

/// The store's last round for one recorded site. The route aggregate
/// supplies the painted end independently of this site's kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitRound {
    pub held: Option<Held>,
}

/// The kind of a site's wait and the maximum a held wait passed.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WaitKind {
    Ordinary,
    Held { max_seconds: u64 },
}

impl WaitKind {
    fn description(self) -> String {
        match self {
            WaitKind::Ordinary => "ordinary".to_owned(),
            WaitKind::Held { max_seconds } => format!("held (maximum {max_seconds}s)"),
        }
    }

    fn driver(self) -> &'static str {
        match self {
            WaitKind::Ordinary => "the engine",
            WaitKind::Held { .. } => "brokkr resume",
        }
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
    #[serde(flatten)]
    pub kind: WaitKind,
    driver: &'static str,
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
    /// end, kind, driver and the provider's words, with `(ended)` once the
    /// end has passed.
    fn line(&self, now: &str) -> String {
        let site = match &self.site {
            Some(site) => format!(" at {site}"),
            None => String::new(),
        };
        let ended = match self.ended(now) {
            true => " (ended)",
            false => "",
        };
        let kind = self.kind.description();
        format!(
            "wait      {}/{}{} until {}{} · {} · {}: {}",
            self.provider, self.route, site, self.until, ended, kind, self.driver, self.words
        )
    }
}

/// A run's waits as every readout paints them. `refused` carries the
/// reason a present wait or its stored rounds could not be read.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Waits {
    pub waits: Vec<Wait>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refused: Option<String>,
}

impl Waits {
    /// The standing of a running run without an accepted stop.
    pub(super) fn standing(&self) -> Standing {
        if self.waits.is_empty() {
            Standing::Running
        } else {
            Standing::Waiting
        }
    }

    /// Refresh the standing of an already derived row or summary.
    fn standing_for(&self, standing: Standing) -> Standing {
        match standing {
            Standing::Running | Standing::Waiting => self.standing(),
            Standing::Quarantined
            | Standing::Parked
            | Standing::Stopping
            | Standing::Shipped
            | Standing::Stopped
            | Standing::OperatorStopped => standing,
        }
    }

    /// The waits `fact` states, each ended at the aggregate's `until` for
    /// its provider and route when `cooling` holds a later one, else at
    /// the event's own `until`.
    pub fn of(fact: &WaitFact, cooling: &[Cooling]) -> Waits {
        match fact {
            WaitFact::NoWait => Waits::default(),
            WaitFact::Unreadable(refusal) => Waits::refused(refusal.words()),
            WaitFact::Waits { sites, rounds, .. } => match painted(sites, rounds, cooling) {
                Ok(waits) => Waits {
                    waits,
                    refused: None,
                },
                Err(error) => Waits::refused(error.to_string()),
            },
        }
    }

    /// A waits collection that paints one refusal and no wait: a present
    /// entry the view cannot read is never read as no wait.
    fn refused(error: String) -> Waits {
        Waits {
            waits: Vec::new(),
            refused: Some(error),
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
fn painted(
    waits: &[ProviderWait],
    rounds: &BTreeMap<Option<String>, WaitRound>,
    cooling: &[Cooling],
) -> Result<Vec<Wait>, ProviderWaitError> {
    waits
        .iter()
        .map(|wait| wait_of(wait, rounds.get(&wait.site), cooling))
        .collect()
}

/// One event wait, its end written by brokkr-core and taken from the
/// aggregate's latest `until` for its provider and route where that is
/// later than the event's own.
fn wait_of(
    wait: &ProviderWait,
    round: Option<&WaitRound>,
    cooling: &[Cooling],
) -> Result<Wait, ProviderWaitError> {
    let until = wait.until_stamp()?;
    let end = cooling
        .iter()
        .filter(|cooling| cooling.provider == wait.provider && cooling.route == wait.route)
        .filter_map(|cooling| parse_millis(&cooling.until).map(|at| (at, &cooling.until)))
        .max_by_key(|(at, _)| *at)
        .filter(|(at, _)| *at > parse_millis(&until).unwrap_or(i64::MIN))
        .map_or(until, |(_, until)| until.clone());
    // The terminal writer records the event kind on its own row as well.
    let held = round.map_or(wait.held, |round| round.held);
    let kind = held.map_or(WaitKind::Ordinary, |held| WaitKind::Held {
        max_seconds: held.max_seconds,
    });
    Ok(Wait {
        kind,
        driver: kind.driver(),
        site: wait.site.clone(),
        provider: wait.provider.clone(),
        route: wait.route.clone(),
        until: end,
        words: wait.words.clone(),
    })
}

#[cfg(test)]
mod tests;

impl RunRow {
    /// Replace waits from a fresh read at the same journal head.
    pub fn with_waits(mut self, waits: Waits) -> Self {
        self.verdict.standing = waits.standing_for(self.verdict.standing);
        self.status_word = self
            .verdict
            .standing
            .status_word(self.status.as_deref(), self.ending)
            .to_string();
        self.waits = waits;
        self
    }
}
