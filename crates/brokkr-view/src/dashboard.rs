//! The run dashboard (#503): what the fleet's dashboard and its live or
//! findings column say about the selected run — the path it took, the
//! ruling it last earned, and what its last seat and its last review
//! reported — derived here once (decision 0013) from the journal its run
//! view is folded from. A renderer lays these out and derives none of
//! them; a journal that does not say a thing leaves it absent, never
//! invented (decision 0001).

use std::collections::BTreeMap;

use brokkr_core::policy::Severity;
use brokkr_core::{EventEnvelope, EventType};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::{
    display_or_mark, field, fmt_dur, joined, js, result_token, truthy, CheckpointRow, RunView,
};

/// The phase whose seat reports a run's findings.
const REVIEW: &str = "review";

/// The dashboard's facts about one run, beside its run view.
#[derive(Serialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct Dashboard {
    /// The run the journal names, so a surface holding one run's view
    /// never paints it as another's.
    pub run_id: Option<String>,
    /// Every phase visit, in the order the journal entered them.
    pub path: Vec<Visit>,
    /// The last ruling the journal records.
    pub decision: Option<Decision>,
    /// The last seat to conclude, and what it reported.
    pub last_seat: Option<Concluded>,
    /// The last seat to conclude in the review phase: the run's findings.
    pub last_review: Option<Concluded>,
}

/// One visit to a phase: how it was ruled, its review's worst residual,
/// the models that served it and how long it ran.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Visit {
    pub phase: String,
    pub ruled: Ruled,
    /// The rule that ruled the visit, when one did.
    pub rule: Option<String>,
    /// The worst residual its ruling read, when above `none`.
    pub residual: Option<Severity>,
    /// Every model a seat of the visit reported serving it, joined.
    pub model: Option<String>,
    /// From the visit's entry to its last event.
    pub duration: Option<String>,
}

/// How a visit was ruled.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Ruled {
    /// No ruling yet.
    Open,
    /// Ruled on to a phase not yet visited.
    Passed,
    /// Ruled on with a flagged severity.
    Flagged,
    /// Ruled back to a phase already visited.
    Returned,
    /// Ruled to no next phase: the run waits on the operator.
    Parked,
    /// Ruled with a hard severity.
    Stopped,
}

/// The ruling severity a `transition/decided` names: phase-event/v1's
/// closed vocabulary.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    Normal,
    Flagged,
    Hard,
}

impl RuleSeverity {
    fn named(name: &str) -> Option<RuleSeverity> {
        match name {
            "normal" => Some(RuleSeverity::Normal),
            "flagged" => Some(RuleSeverity::Flagged),
            "hard" => Some(RuleSeverity::Hard),
            _ => None,
        }
    }

    /// The word the journal writes for it.
    pub fn label(self) -> &'static str {
        match self {
            RuleSeverity::Normal => "normal",
            RuleSeverity::Flagged => "flagged",
            RuleSeverity::Hard => "hard",
        }
    }
}

/// A `transition/decided`, read once.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Decision {
    /// The rule id, or `?` where the ruling names none.
    pub rule: String,
    pub severity: Option<RuleSeverity>,
    pub from: Option<String>,
    pub next: Option<String>,
    /// The seat result word the ruling read.
    pub result: Option<String>,
    pub residual: Option<Severity>,
    /// Why, in the ruling's own words.
    pub problem: Option<String>,
}

/// How an effect's open attempt concluded.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Succeeded,
    Failed,
    Indeterminate,
}

impl Outcome {
    fn of(event_type: EventType) -> Option<Outcome> {
        match event_type {
            EventType::EffectSucceeded => Some(Outcome::Succeeded),
            EventType::EffectFailed => Some(Outcome::Failed),
            EventType::EffectIndeterminate => Some(Outcome::Indeterminate),
            EventType::RunStarted
            | EventType::PhaseEntered
            | EventType::EffectRequested
            | EventType::EffectStarted
            | EventType::EffectCheckpointed
            | EventType::TransitionDecided
            | EventType::OperatorCommanded
            | EventType::OperatorAccepted
            | EventType::OperatorRejected
            | EventType::RunParked
            | EventType::RunCompleted
            | EventType::RunStopped => None,
        }
    }

    /// The word every surface prints for it.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::Succeeded => "succeeded",
            Outcome::Failed => "failed",
            Outcome::Indeterminate => "indeterminate",
        }
    }
}

/// A seat's concluded attempt: its result word and its notes, whole.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Concluded {
    pub seat: String,
    pub phase: Option<String>,
    pub outcome: Outcome,
    pub result: Option<String>,
    /// The seat's notes: a panel's as one line per member, a failed
    /// attempt's error where it wrote no notes.
    pub notes: Option<String>,
}

/// A visit under construction.
struct Entered {
    phase: String,
    entered_at: String,
    last_at: String,
    decision: Option<Decision>,
    ruled: Ruled,
    models: Vec<String>,
}

/// The one walk over the journal.
#[derive(Default)]
struct Reading<'a> {
    visits: Vec<Entered>,
    /// Effect id -> its seat, its phase and the visit it was requested in.
    effects: BTreeMap<&'a str, (&'a str, Option<&'a str>, Option<usize>)>,
    /// Effect id -> its open attempt.
    open: BTreeMap<&'a str, &'a str>,
    dashboard: Dashboard,
}

/// The dashboard of the run `events` journal: [`RunView`]'s, built with it.
pub(crate) fn dashboard(events: &[EventEnvelope]) -> Dashboard {
    let mut reading = Reading::default();
    for event in events {
        reading.read(event);
    }
    let path = reading.visits.into_iter().map(|visit| Visit {
        residual: visit.decision.as_ref().and_then(|ruling| ruling.residual),
        rule: visit.decision.map(|ruling| ruling.rule),
        model: joined(visit.models.iter().map(String::as_str).collect()),
        duration: fmt_dur(&visit.entered_at, &visit.last_at),
        phase: visit.phase,
        ruled: visit.ruled,
    });
    Dashboard {
        run_id: events.first().map(|event| event.run_id.clone()),
        path: path.collect(),
        ..reading.dashboard
    }
}

impl<'a> Reading<'a> {
    fn read(&mut self, event: &'a EventEnvelope) {
        if let Some(visit) = self.visits.last_mut() {
            visit.last_at.clone_from(&event.recorded_at);
        }
        let payload = &event.payload;
        let effect = field(payload, "effect_id");
        match (event.event_type, effect) {
            (EventType::PhaseEntered, _) => self.enter(event),
            (EventType::TransitionDecided, _) => self.decide(payload),
            (EventType::EffectRequested, Some(effect)) => {
                let seat = field(payload, "seat").unwrap_or("?");
                let visit = self.visits.len().checked_sub(1);
                let at = (seat, field(payload, "phase"), visit);
                self.effects.insert(effect, at);
            }
            (EventType::EffectStarted, Some(effect)) => {
                if let Some(attempt) = field(payload, "attempt_id") {
                    self.open.insert(effect, attempt);
                }
            }
            (EventType::EffectCheckpointed, Some(effect)) => {
                let model = payload.pointer("/checkpoint/model");
                self.served(effect, model.and_then(Value::as_str));
            }
            (kind, Some(effect)) => {
                if let Some(outcome) = Outcome::of(kind) {
                    self.conclude(effect, outcome, payload);
                }
            }
            (_, None) => {}
        }
    }

    fn enter(&mut self, event: &EventEnvelope) {
        let phase = display_or_mark(event.payload.get("phase"));
        self.visits.push(Entered {
            phase,
            entered_at: event.recorded_at.clone(),
            last_at: event.recorded_at.clone(),
            decision: None,
            ruled: Ruled::Open,
            models: Vec::new(),
        });
    }

    fn decide(&mut self, payload: &Value) {
        let ruling = decision_of(payload);
        let visited = |next: &str| self.visits.iter().any(|visit| visit.phase == next);
        let ruled = match (&ruling.severity, ruling.next.as_deref()) {
            (Some(RuleSeverity::Hard), _) => Ruled::Stopped,
            (_, None) => Ruled::Parked,
            (Some(RuleSeverity::Flagged), _) => Ruled::Flagged,
            (_, Some(next)) if visited(next) => Ruled::Returned,
            (Some(RuleSeverity::Normal) | None, Some(_)) => Ruled::Passed,
        };
        if let Some(visit) = self.visits.last_mut() {
            visit.decision = Some(ruling.clone());
            visit.ruled = ruled;
        }
        self.dashboard.decision = Some(ruling);
    }

    /// A model a seat reported serving `effect`, on the visit it was
    /// requested in.
    fn served(&mut self, effect: &str, model: Option<&str>) {
        let visit = self.effects.get(effect).and_then(|(_, _, visit)| *visit);
        let (Some(visit), Some(model)) = (visit, model) else {
            return;
        };
        let models = &mut self.visits[visit].models;
        if !models.iter().any(|known| known == model) {
            models.push(model.to_string());
        }
    }

    /// An effect's terminal event. One for an attempt that is not the
    /// open one is stale, and says nothing, as in the participant scan.
    fn conclude(&mut self, effect: &'a str, outcome: Outcome, payload: &Value) {
        if self.open.get(effect).copied() != field(payload, "attempt_id") {
            return;
        }
        self.open.remove(effect);
        self.served(
            effect,
            payload.pointer("/result/model").and_then(Value::as_str),
        );
        let Some((seat, phase, _)) = self.effects.get(effect).copied() else {
            return;
        };
        let concluded = Concluded {
            seat: seat.to_string(),
            phase: phase.map(str::to_string),
            outcome,
            result: result_token(payload).map(str::to_string),
            notes: notes_of(payload),
        };
        if phase == Some(REVIEW) {
            self.dashboard.last_review = Some(concluded.clone());
        }
        self.dashboard.last_seat = Some(concluded);
    }
}

fn decision_of(payload: &Value) -> Decision {
    let text = |key: &str| field(payload, key).map(str::to_string);
    let residual = payload
        .pointer("/inputs/max_residual_severity")
        .and_then(Value::as_str)
        .and_then(Severity::named)
        .filter(|severity| *severity > Severity::None);
    Decision {
        rule: display_or_mark(payload.get("rule_id")),
        severity: field(payload, "severity").and_then(RuleSeverity::named),
        from: text("from"),
        next: text("next"),
        result: text("result"),
        residual,
        problem: payload
            .get("problem")
            .filter(|problem| truthy(Some(problem)))
            .map(|problem| js::to_display(Some(problem))),
    }
}

/// A concluded attempt's notes: its result's `notes` — a panel's one line
/// per member, with each member's verdict — or else the error a failed
/// attempt wrote. An empty text is no notes.
fn notes_of(payload: &Value) -> Option<String> {
    let notes = match payload.pointer("/result/notes") {
        Some(Value::Object(map)) => Some(panel_notes(map)),
        Some(other) => Some(js::to_display(Some(other))),
        None => field(payload, "error").map(str::to_string),
    };
    notes.filter(|text| !text.trim().is_empty())
}

/// A panel's notes, one line per member: `name (verdict): note`. Notes of
/// any other shape print as the journal wrote them.
fn panel_notes(map: &Map<String, Value>) -> String {
    let Some(members) = map.get("members").and_then(Value::as_object) else {
        return Value::Object(map.clone()).to_string();
    };
    let verdicts = map.get("verdicts");
    let lines = members.iter().map(|(name, note)| {
        let verdict = verdicts.and_then(|verdicts| field(verdicts, name));
        let named = match verdict {
            Some(verdict) => format!("{name} ({verdict})"),
            None => name.clone(),
        };
        format!("{named}: {}", js::to_display(Some(note)))
    });
    lines.collect::<Vec<_>>().join("\n")
}

/// The checkpoints of the seat the fold names as at work, newest first,
/// each with the label of the participant that recorded it: the seat's
/// own and every member's, while they work.
pub fn working_checkpoints<'a>(view: &'a RunView, seat: &str) -> Vec<(&'a str, &'a CheckpointRow)> {
    let working = view.participants.iter().filter(|part| {
        let member = part.label.strip_prefix(seat);
        part.working() && member.is_some_and(|rest| rest.is_empty() || rest.starts_with(':'))
    });
    let mut rows: Vec<(&str, &CheckpointRow)> = working
        .flat_map(|part| {
            part.checkpoints
                .iter()
                .map(|row| (part.label.as_str(), row))
        })
        .collect();
    rows.sort_by(|(_, a), (_, b)| a.recorded_at.cmp(&b.recorded_at));
    rows.reverse();
    rows
}

#[cfg(test)]
mod tests;
