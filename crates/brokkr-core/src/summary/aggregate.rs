//! What the aggregate readouts read of a run (decision 0089 ruling 7):
//! the realm and recipe it started under, when it started, when and how it
//! ended, how often it entered implement, and where its reviews sent it.
//! Read here once, from its events and its fold, so the writer keeps on
//! the run's summary row what `brokkr analytics` and the recipe's graph
//! derive from the run read whole (decision 0071 ruling 5).
//!
//! The writer's [`Account`](super::Account) carries a [`Reading`] over
//! every event it seals, as it carries the fold; [`aggregate`] reads a
//! run's events whole the same way.

use serde::Deserialize;
use serde_json::Value;

use crate::ending::{closing, Closing, Ending};
use crate::envelope::{EventEnvelope, EventType};
use crate::fold::{RunState, Status};
use crate::policy::audit::STOP;
use crate::residual::{text as field, ResidualPhase};
use crate::stamp::parse_millis;

/// The phase whose visits count a run's rework.
const IMPLEMENT_PHASE: &str = "implement";

/// When and how a finished run ended: its last terminal event's time, its
/// ending (decision 0064), and the policy table's hard stop where that is
/// what stopped it, which the `ending_rule` column keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finished {
    pub at: i64,
    pub ending: Ending,
    /// `None` for a run that completed and for one an operator stopped
    /// anywhere else, whatever a later `classify` names.
    pub hard_stop: Option<HardStop>,
}

/// The policy table's hard stop that stopped a run: the rule its last
/// ruling names, when it names one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardStop {
    pub rule: Option<String>,
}

/// What the aggregate readouts read of one run, its names whole: the
/// writer cuts them where it stores them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aggregate {
    /// The realm its start's world pin names.
    pub realm: Option<String>,
    /// The recipe its start's manifest names.
    pub recipe: Option<String>,
    /// Its first event's time.
    pub started_ms: Option<i64>,
    /// When and how it ended, once it has and that event's stamp parses.
    pub finished: Option<Finished>,
    pub implement_visits: u64,
    /// Whether its first ruling from review sent it back to a phase it
    /// had entered,
    pub review_first: bool,
    /// whether its second routed it on to one it had not,
    pub review_second: bool,
    /// and whether any sent it back.
    pub review_back: bool,
}

/// Where a ruling from review sent the run: back to a phase it had
/// entered, on to one it had not, or held by a park or the hard stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    Back,
    On,
    Held,
}

/// `run/started`'s manifest as far as its realm and recipe, read whole
/// or not at all: a world pin that does not parse leaves both unread.
#[derive(Deserialize)]
struct Started {
    manifest: Manifest,
}

#[derive(Deserialize)]
struct Manifest {
    realms: Option<Pin>,
    #[serde(default)]
    bundle_name: Option<String>,
}

/// The world pin, typed to its map's realms as every reader of it types
/// it, though only its realm is read here.
#[derive(Deserialize)]
struct Pin {
    #[serde(rename = "source")]
    _source: String,
    realm: Option<String>,
    #[serde(rename = "map")]
    _map: PinnedMap,
}

#[derive(Deserialize)]
struct PinnedMap {
    #[serde(rename = "realms")]
    _realms: Vec<PinnedRealm>,
}

#[derive(Deserialize)]
struct PinnedRealm {
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "path")]
    _path: String,
}

/// What a run's events say of it so far, carried event by event.
#[derive(Debug, Clone, Default)]
pub(crate) struct Reading {
    /// Whether its first event has been read.
    begun: bool,
    realm: Option<String>,
    recipe: Option<String>,
    started_ms: Option<i64>,
    /// Its last terminal event's time, `Some(None)` where that event's
    /// stamp does not parse.
    terminal: Option<Option<i64>>,
    entered: Vec<String>,
    routes: Vec<Route>,
}

impl Reading {
    /// The reading carried over `event`, the run's next.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        let payload = &event.payload;
        if !self.begun {
            self.begun = true;
            self.started_ms = parse_millis(&event.recorded_at);
            if event.event_type == EventType::RunStarted {
                if let Ok(started) = Started::deserialize(payload) {
                    self.realm = started.manifest.realms.and_then(|pin| pin.realm);
                    self.recipe = started.manifest.bundle_name;
                }
            }
        }
        match event.event_type {
            EventType::PhaseEntered => self.entered.extend(field(payload, "phase").map(Into::into)),
            EventType::TransitionDecided => self.ruled(payload),
            EventType::RunCompleted | EventType::RunStopped => {
                self.terminal = Some(parse_millis(&event.recorded_at));
            }
            _ => {}
        }
    }

    /// A ruling from review: where it sent the run.
    fn ruled(&mut self, payload: &Value) {
        if field(payload, "from") != Some(ResidualPhase::Review.as_str()) {
            return;
        }
        self.routes.push(match field(payload, "next") {
            None | Some(STOP) => Route::Held,
            Some(next) if self.entered.iter().any(|entered| entered == next) => Route::Back,
            Some(_) => Route::On,
        });
    }

    /// What the reading and `state`, the run's fold at the same head, say
    /// of the run, `closing` its ending as [`crate::ending::closing`]
    /// reads it at that head.
    pub(crate) fn of(&self, state: Option<&RunState>, closing: Option<&Closing>) -> Aggregate {
        let finished = self.terminal.flatten().zip(state.zip(closing));
        Aggregate {
            realm: self.realm.clone(),
            recipe: self.recipe.clone(),
            started_ms: self.started_ms,
            finished: finished.map(|(at, (state, closing))| Finished {
                at,
                ending: closing.ended.ending,
                hard_stop: hard_stop(state),
            }),
            implement_visits: state
                .and_then(|state| state.visits.get(IMPLEMENT_PHASE).copied())
                .unwrap_or(0),
            review_first: self.routes.first() == Some(&Route::Back),
            review_second: self.routes.get(1) == Some(&Route::On),
            review_back: self.routes.contains(&Route::Back),
        }
    }
}

/// The table's hard stop, while the fold states it stopped the run there:
/// stopped, at the `stop` phase.
fn hard_stop(state: &RunState) -> Option<HardStop> {
    let stopped = state.run.status() == Status::Stopped && state.phase.as_deref() == Some(STOP);
    stopped.then(|| HardStop {
        rule: state
            .last_decision
            .as_ref()
            .and_then(|decision| decision.rule_id.present().cloned()),
    })
}

/// What the aggregate readouts read of the run `events` hold, in order
/// from its first, folded to `state`, or `None` where it does not fold.
pub fn aggregate(events: &[EventEnvelope], state: Option<&RunState>) -> Aggregate {
    let mut reading = Reading::default();
    for event in events {
        reading.note(event);
    }
    let closing = state.and_then(|state| closing(events, state));
    reading.of(state, closing.as_ref())
}

#[cfg(test)]
mod tests;
