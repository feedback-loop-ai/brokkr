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
use serde_json::Value;
use thiserror::Error;

use crate::decision::Decision;
use crate::envelope::{EventEnvelope, EventType};
use crate::fold::{OperatorCommand, RunState, Status};
use crate::policy::audit::STOP;
use crate::policy::RuleSeverity;
use crate::residual::ResidualPhase;

/// The phase whose dirty-worktree stops read `blocked` (addendum choice
/// 1, row 4).
const SHIP: &str = "ship";

/// The operator command that names a finished run's ending after the fact
/// (decision 0064 ruling 5).
pub const CLASSIFY: &str = "classify";

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

    /// The six operator-owned words, in ruling 2's order, as a refusal
    /// lists them.
    pub fn operator_words() -> String {
        Ending::ALL
            .into_iter()
            .filter(|ending| ending.operator_owned())
            .map(Ending::word)
            .collect::<Vec<_>>()
            .join(", ")
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

/// The `args` of one `operator/commanded` with `command: "classify"`
/// (decision 0064 ruling 5) or `command: "stop"` (ruling 4), under
/// contract `operator-ending.v1`, read as a closed value. A field outside
/// the contract is refused here, as the schema refuses it at the edge.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperatorEnding {
    cause: String,
    #[serde(default, deserialize_with = "present")]
    by: Option<By>,
    #[serde(default, rename = "ref", deserialize_with = "present")]
    reference: Option<String>,
    #[serde(default, rename = "reason", deserialize_with = "present")]
    _reason: Option<String>,
}

/// A field the contract writes as a value: a present `null` is refused,
/// because the contract says `by` is an object, `ref` a string and
/// `reason` a string, while serde's `Option` would read `null` as absent.
/// Absent stays absent through [`Default`], which is why the callers also
/// carry `default`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// The run a `superseded` ending cites, keyed as decision 0026 ruling 3
/// keys every fleet fact. `realm` is required and is `null` for a
/// one-hearth world: absence is not a workspace citation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct By {
    realm: Realm,
    run_id: String,
}

/// A present realm key: a string, or `null` for the workspace journal.
/// The newtype keeps `null` distinct from a missing field, which the
/// contract refuses. It reads by `deserialize_any`, not
/// `deserialize_option`: serde's missing-field path answers a field that
/// asks for an option with `None`, which would turn absence into the
/// workspace citation this type exists to refuse.
#[derive(Debug)]
struct Realm(Option<String>);

impl<'de> Deserialize<'de> for Realm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct RealmVisitor;

        impl<'de> serde::de::Visitor<'de> for RealmVisitor {
            type Value = Realm;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a realm name or null")
            }

            fn visit_unit<E>(self) -> Result<Realm, E> {
                Ok(Realm(None))
            }

            fn visit_str<E>(self, realm: &str) -> Result<Realm, E> {
                Ok(Realm(Some(realm.to_string())))
            }
        }

        deserializer.deserialize_any(RealmVisitor)
    }
}

/// The ending a `classify` annotation or a stop names, with the citation
/// its cause requires, or `None` when its `args` are not ones this build
/// reads.
/// The contract is `operator-ending.v1`: one of the six operator-owned
/// endings, `by` with `superseded`, `ref` with `delivered-elsewhere`, and
/// neither elsewhere. A malformed or unrecognised annotation is not read
/// as a nearest guess (decision 0064 ruling 5).
pub(crate) fn operator_ending(args: &Value) -> Option<Ended> {
    let OperatorEnding {
        cause,
        by,
        reference,
        _reason,
    } = serde_json::from_value(args.clone()).ok()?;
    let ending = Ending::named(&cause).ok()?;
    let cited = match ending {
        Ending::Superseded => match (by, reference) {
            (
                Some(By {
                    realm: Realm(realm),
                    run_id,
                }),
                None,
            ) => Some(Citation::Run { realm, run_id }),
            _ => return None,
        },
        Ending::DeliveredElsewhere => match (by, reference) {
            (None, Some(reference)) => Some(Citation::Ref(reference)),
            _ => return None,
        },
        Ending::Infrastructure
        | Ending::Exhausted
        | Ending::Withdrawn
        | Ending::OperatorStopped => {
            if by.is_some() || reference.is_some() {
                return None;
            }
            None
        }
        Ending::Shipped
        | Ending::ShippedFlagged
        | Ending::Refused
        | Ending::Failed
        | Ending::Blocked
        | Ending::Unclassified => return None,
    };
    Some(Ended { ending, cited })
}

/// What a run's events say of its ending so far, carried event by event
/// (addendum choice 5): the severity of its last ruling from review, the
/// cause the accepted stop names in its `args` (ruling 4), and the latest
/// `classify` annotation (ruling 5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Reading {
    last_review: Option<RuleSeverity>,
    /// The latest stop commanded, by its command id, with the ending its
    /// `args` name, if they name one: what an acceptance of that id adopts.
    commanded_stop: Option<(String, Option<Ended>)>,
    /// The ending the accepted stop's `args` name; `None` for a stop that
    /// names none, as every stop before decision 0064 slice E3 did.
    stopped: Option<Ended>,
    classified: Option<Ended>,
}

impl Reading {
    /// The reading carried over `event`, the run's next.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        let payload = &event.payload;
        let command_id = payload.get("command_id").and_then(Value::as_str);
        if event.event_type == EventType::OperatorCommanded {
            let command = payload.get("command").and_then(Value::as_str);
            let named = payload.get("args").and_then(operator_ending);
            if command == Some(CLASSIFY) {
                if let Some(classified) = named {
                    self.classified = Some(classified);
                }
            } else if command == Some(OperatorCommand::Stop.as_str()) {
                self.commanded_stop = command_id.map(|id| (id.to_string(), named));
            }
            return;
        }
        if event.event_type == EventType::OperatorAccepted {
            let accepted = |(stop, _): &mut (String, _)| Some(stop.as_str()) == command_id;
            if let Some((_, named)) = self.commanded_stop.take_if(accepted) {
                self.stopped = named;
            }
            return;
        }
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
        match state.run.status() {
            Status::Running | Status::AwaitingOperator => None,
            // Ruling 6: `shipped`/`shipped-flagged` imply `completed`, so
            // no operator cause can stand here; `classify` refuses a
            // completed run rather than journaling an unread annotation.
            Status::Completed => Some(Ended {
                ending: Ending::completed(self.last_review),
                cited: None,
            }),
            // A later `classify` stands over an earlier one (ruling 5),
            // over the cause the closing stop named, and over the closure
            // the operator did not type.
            Status::Stopped if self.classified.is_some() => self.classified.clone(),
            Status::Stopped if self.stopped.is_some() => self.stopped.clone(),
            Status::Stopped if state.phase.as_deref() == Some(STOP) => {
                let ruling = state.last_decision.as_ref();
                Some(Ended {
                    ending: ruling.map_or(Ending::Unclassified, |it| {
                        Ending::ruled(&it.from, &it.result)
                    }),
                    cited: None,
                })
            }
            Status::Stopped => Some(Ended {
                ending: Ending::Unclassified,
                cited: None,
            }),
        }
    }
}

/// The ending of the run `events` hold, in order from its first, folded
/// to `state`; `None` while it is live, parked or stopping.
///
/// A completed run splits on its last ruling from review, and a run the
/// policy table stopped reads [`Ending::ruled`] of the ruling that
/// stopped it. A run an operator stopped reads the latest `classify`
/// annotation that names one of ruling 4's six (ruling 5); else the
/// cause, with its citation, that its accepted stop names in its `args`,
/// as `stop` and `conclude` write it under `--cause` (ruling 4, decision
/// 0064 slice E3); and `unclassified` when it carries neither, as every
/// stop journaled before E3 does.
pub fn ended(events: &[EventEnvelope], state: &RunState) -> Option<Ended> {
    let mut reading = Reading::default();
    for event in events {
        reading.note(event);
    }
    reading.of(state)
}

#[cfg(test)]
mod tests;
