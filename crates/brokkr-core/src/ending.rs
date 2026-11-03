//! How a run ended (decision 0064): one word from a closed vocabulary,
//! derived from the journal by the read side and never a status. The
//! fold keeps its four statuses (ruling 1); a terminal run's ending says
//! what `completed` and `stopped` do not, and what it asks of the
//! operator.
//!
//! This is the vocabulary's one home (ruling 2). Beside it are ruling
//! 6's tone, gutter mark and wants, each a function of the ending alone;
//! the 2026-11-03 addendum's ruling 1, which derives a policy ending from
//! the ruling that ended the run; and the reading of a run's events its
//! ruling 5 carries, which gives a terminal run its ending.

use serde::Deserialize;
use thiserror::Error;

use crate::decision::Decision;
use crate::envelope::{EventEnvelope, EventType};
use crate::fold::{RunState, Status};
use crate::policy::audit::STOP;
use crate::policy::RuleSeverity;
use crate::residual::ResidualPhase;

/// The phase whose dirty-worktree stops read `blocked` (addendum choice
/// 1, row 4).
const SHIP: &str = "ship";

/// How a terminal run ended: decision 0064 ruling 2's twelve words. A
/// new ending is a new decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ending {
    /// Completed, its last review clean.
    Shipped,
    /// Completed, its last review passing residuals through flagged.
    ShippedFlagged,
    /// A review rule ruled a hard stop on the work.
    Refused,
    /// Verification or a twice-broken smith ended it.
    Failed,
    /// The smith reported it could not proceed.
    Blocked,
    /// A bounded loop ran out and the operator closed its park.
    Exhausted,
    /// Another run replaced this one.
    Superseded,
    /// The host, a provider or the engine failed; the work was not judged.
    Infrastructure,
    /// The work landed by hand or by another road.
    DeliveredElsewhere,
    /// The commission was wrong or no longer wanted.
    Withdrawn,
    /// An operator stopped a run still working, and named no other cause.
    OperatorStopped,
    /// The journal does not say.
    Unclassified,
}

/// What colour a terminal row is painted (ruling 6): what it waits on,
/// never whether it was good. `running`'s bold is a status's, not an
/// ending's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Done.
    Green,
    /// Waiting on a ruling from the operator.
    Yellow,
    /// Waiting on an action; the work was not judged.
    Cyan,
    /// Judged, and did not pass.
    Red,
    /// Settled, or nothing known.
    Dim,
}

/// What an ending asks the operator for, when it asks for anything
/// (ruling 6): the two that the header counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wants {
    Ruling,
    Action,
}

/// A word outside the twelve, refused rather than read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("'{word}' is not an ending; an ending is one of {words}", words = vocabulary())]
pub struct UnknownEnding {
    pub word: String,
}

/// The twelve words, in ruling 2's order, as a refusal lists them.
fn vocabulary() -> String {
    Ending::ALL.map(Ending::word).join(", ")
}

impl Ending {
    /// Every ending, in ruling 2's order.
    pub const ALL: [Ending; 12] = [
        Ending::Shipped,
        Ending::ShippedFlagged,
        Ending::Refused,
        Ending::Failed,
        Ending::Blocked,
        Ending::Exhausted,
        Ending::Superseded,
        Ending::Infrastructure,
        Ending::DeliveredElsewhere,
        Ending::Withdrawn,
        Ending::OperatorStopped,
        Ending::Unclassified,
    ];

    /// The word the journal, the wire and every surface write for it.
    pub const fn word(self) -> &'static str {
        match self {
            Ending::Shipped => "shipped",
            Ending::ShippedFlagged => "shipped-flagged",
            Ending::Refused => "refused",
            Ending::Failed => "failed",
            Ending::Blocked => "blocked",
            Ending::Exhausted => "exhausted",
            Ending::Superseded => "superseded",
            Ending::Infrastructure => "infrastructure",
            Ending::DeliveredElsewhere => "delivered-elsewhere",
            Ending::Withdrawn => "withdrawn",
            Ending::OperatorStopped => "operator-stopped",
            Ending::Unclassified => "unclassified",
        }
    }

    /// The ending [`Ending::word`] writes as `word`; any other word is
    /// refused.
    pub fn named(word: &str) -> Result<Ending, UnknownEnding> {
        let named = Ending::ALL.into_iter().find(|ending| ending.word() == word);
        named.ok_or_else(|| UnknownEnding { word: word.into() })
    }

    /// Its tone class (ruling 6's table).
    pub const fn tone(self) -> Tone {
        match self {
            Ending::Shipped | Ending::ShippedFlagged => Tone::Green,
            Ending::Refused | Ending::Exhausted => Tone::Yellow,
            Ending::Blocked | Ending::Infrastructure => Tone::Cyan,
            Ending::Failed => Tone::Red,
            Ending::Superseded
            | Ending::DeliveredElsewhere
            | Ending::Withdrawn
            | Ending::OperatorStopped
            | Ending::Unclassified => Tone::Dim,
        }
    }

    /// Its one-character gutter mark, which carries the tone's distinction
    /// without colour: `!` a ruling is wanted, `>` an action is wanted,
    /// `~` shipped with residuals to read, `?` unclassified, a space
    /// otherwise.
    pub const fn gutter(self) -> char {
        match self {
            Ending::Refused | Ending::Exhausted => '!',
            Ending::Blocked | Ending::Infrastructure => '>',
            Ending::ShippedFlagged => '~',
            Ending::Unclassified => '?',
            Ending::Shipped
            | Ending::Failed
            | Ending::Superseded
            | Ending::DeliveredElsewhere
            | Ending::Withdrawn
            | Ending::OperatorStopped => ' ',
        }
    }

    /// What it asks the operator for, if anything.
    pub const fn wants(self) -> Option<Wants> {
        match self {
            Ending::Refused | Ending::Exhausted => Some(Wants::Ruling),
            Ending::Blocked | Ending::Infrastructure => Some(Wants::Action),
            Ending::Shipped
            | Ending::ShippedFlagged
            | Ending::Failed
            | Ending::Superseded
            | Ending::DeliveredElsewhere
            | Ending::Withdrawn
            | Ending::OperatorStopped
            | Ending::Unclassified => None,
        }
    }

    /// Whether an operator who ends a run may name it as the cause
    /// (ruling 4's six).
    pub const fn operator_owned(self) -> bool {
        match self {
            Ending::Superseded
            | Ending::Infrastructure
            | Ending::DeliveredElsewhere
            | Ending::Withdrawn
            | Ending::Exhausted
            | Ending::OperatorStopped => true,
            Ending::Shipped
            | Ending::ShippedFlagged
            | Ending::Refused
            | Ending::Failed
            | Ending::Blocked
            | Ending::Unclassified => false,
        }
    }

    /// The ending of a run the policy table stopped, from the ruling that
    /// stopped it: the first of the addendum's ruling 1 rows its `from`
    /// and `result` match, else `unclassified`. The rule id is never read,
    /// and no policy stop is `exhausted`, which is the operator's word.
    pub fn ruled(from: &str, result: &str) -> Ending {
        let row = rows().into_iter().find(|row| row.matches(from, result));
        row.map_or(Ending::Unclassified, |Row(_, _, ending)| ending)
    }

    /// The ending of a completed run whose last ruling from review carried
    /// `last_review`: `shipped-flagged` when it was flagged. A flagged
    /// ruling from any other phase marks a route, not residuals, and is
    /// not read.
    pub fn completed(last_review: Option<RuleSeverity>) -> Ending {
        match last_review {
            Some(RuleSeverity::Flagged) => Ending::ShippedFlagged,
            Some(RuleSeverity::Normal | RuleSeverity::Hard) | None => Ending::Shipped,
        }
    }
}

/// One row of the addendum's ruling 1: a ruling from the phase it names,
/// any phase when `None`, whose result is one of those it names, any
/// result when `None`, ends its run as its ending.
struct Row(
    Option<&'static str>,
    Option<&'static [&'static str]>,
    Ending,
);

impl Row {
    fn matches(&self, from: &str, result: &str) -> bool {
        let Row(phase, results, _) = self;
        phase.is_none_or(|phase| phase == from)
            && results.is_none_or(|results| results.contains(&result))
    }
}

/// The addendum's ruling 1, in its order; the first row that matches
/// rules. The two `verify` rows are for journals the v1 table ruled.
fn rows() -> [Row; 6] {
    let review = Some(ResidualPhase::Review.as_str());
    let verify = Some(ResidualPhase::Verify.as_str());
    [
        Row(None, Some(&["blocked"]), Ending::Blocked),
        Row(None, Some(&["broken", "fail", "failed"]), Ending::Failed),
        Row(review, None, Ending::Refused),
        Row(Some(SHIP), Some(&["ready", "shipped"]), Ending::Blocked),
        Row(verify, Some(&["stack-conflict"]), Ending::Blocked),
        Row(verify, Some(&["env-degraded"]), Ending::Infrastructure),
    ]
}

/// What a terminal run's ending cites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Citation {
    /// The run that replaced a `superseded` one, in the realm named when
    /// one is.
    Run {
        realm: Option<String>,
        run_id: String,
    },
    /// The commit or pull request a `delivered-elsewhere` run landed by,
    /// as written.
    Ref(String),
}

/// A terminal run's ending, and its citation where it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ended {
    pub ending: Ending,
    pub cited: Option<Citation>,
}

/// What a run's events say of its ending so far, carried event by event
/// (addendum choice 5): the severity of its last ruling from review.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Reading {
    last_review: Option<RuleSeverity>,
}

impl Reading {
    /// The reading carried over `event`, the run's next.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        if event.event_type != EventType::TransitionDecided {
            return;
        }
        let Ok(decision) = Decision::deserialize(&event.payload) else {
            return;
        };
        if ResidualPhase::named(&decision.from) == Some(ResidualPhase::Review) {
            self.last_review = decision.severity.present().copied();
        }
    }

    /// The ending the reading and `state`, the run's fold at the same
    /// head, give the run, once its fold states it is over.
    pub(crate) fn of(&self, state: &RunState) -> Option<Ended> {
        let ending = match state.run.status() {
            Status::Running | Status::AwaitingOperator => return None,
            Status::Completed => Ending::completed(self.last_review),
            Status::Stopped if state.phase.as_deref() == Some(STOP) => {
                let ruling = state.last_decision.as_ref();
                ruling.map_or(Ending::Unclassified, |it| {
                    Ending::ruled(&it.from, &it.result)
                })
            }
            Status::Stopped => Ending::Unclassified,
        };
        Some(Ended {
            ending,
            cited: None,
        })
    }
}

/// The ending of the run `events` hold, in order from its first, folded
/// to `state`; `None` while it is live, parked or stopping.
///
/// A completed run splits on its last ruling from review, and a run the
/// policy table stopped reads [`Ending::ruled`] of the ruling that
/// stopped it. A run an operator stopped reads `unclassified` today, and
/// nothing cites: the two facts that would name its cause cannot exist
/// in a journal yet, so both are read as absent. The cause an accepted
/// stop carries in its `args` is written once `stop` and `conclude` take
/// `--cause` (decision 0064 slice E3), and the latest `classify`, which
/// stands over it and over an earlier `classify`, once that command
/// exists (slice E2).
pub fn ended(events: &[EventEnvelope], state: &RunState) -> Option<Ended> {
    let mut reading = Reading::default();
    for event in events {
        reading.note(event);
    }
    reading.of(state)
}

#[cfg(test)]
mod tests;
