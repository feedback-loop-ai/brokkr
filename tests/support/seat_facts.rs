//! The run view's participants, held to the facts the writer's seat rows
//! group into (#753; decision 0089 ruling 4 at the projection's level):
//! each participant's facts compared with what `run_view` shows of it,
//! field for field, a field that differs named with its run. brokkr-view
//! holds every committed and hand-built journal to it, and brokkr-runtime
//! every engine scenario's carried account.

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::seat::row::{Member, Outcome, Provider, SeatFacts};
use brokkr_view::{run_view, Cell, Participant};

/// A cell's text, or `None` where the view marks it absent.
fn shown(cell: &Cell) -> Option<String> {
    (!cell.absent).then(|| cell.text.clone())
}

/// The turns cell the view paints from these facts: the turns, `Σ`-marked
/// when summed, and the items beside them.
fn turns_cell(facts: &SeatFacts) -> Option<String> {
    let sum = if facts.turns_aggregated { "Σ " } else { "" };
    let items = facts.items.map(|items| format!("{items} items"));
    match (facts.turns, items) {
        (None, items) => items,
        (Some(turns), None) => Some(format!("{sum}{turns}")),
        (Some(turns), Some(items)) => Some(format!("{sum}{turns} · {items}")),
    }
}

/// The provider the view's provenance names from these facts: `?` where
/// the participant's last provenance entry names none.
fn provider(facts: &SeatFacts) -> Option<&str> {
    facts.provider.as_ref().map(|provider| match provider {
        Provider::Unnamed => "?",
        Provider::Named(name) => name.as_str(),
    })
}

fn status(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Working => "working",
        Outcome::Succeeded => "succeeded",
        Outcome::Failed => "failed",
        Outcome::Indeterminate => "indeterminate",
        Outcome::Carried => "carried",
    }
}
/// When the view's clock starts for `participant`: a seat's at its
/// effect's first start after its first request, as the view's scan
/// passes over a start before it; a member's at its first checkpoint.
fn started_at<'a>(events: &'a [EventEnvelope], participant: &'a Participant) -> Option<&'a str> {
    if participant.member.is_some() {
        return participant
            .checkpoints
            .first()
            .map(|row| row.recorded_at.as_str());
    }
    let effect_id = participant.key.as_str();
    let names = |event: &EventEnvelope, event_type| {
        event.event_type == event_type && event.payload["effect_id"].as_str() == Some(effect_id)
    };
    let mut asked = events
        .iter()
        .skip_while(|event| !names(event, EventType::EffectRequested));
    let started = asked.find(|event| names(event, EventType::EffectStarted));
    started.map(|event| event.recorded_at.as_str())
}

/// The first field `facts` and `participant` disagree on, by name.
fn differs(
    facts: &SeatFacts,
    participant: &Participant,
    start: Option<&str>,
) -> Option<&'static str> {
    let key = match &facts.member {
        Member::Seat => facts.effect_id.clone(),
        Member::Named(member) => format!("{}:{member}", facts.effect_id),
    };
    let provenance = participant.provenance.as_ref();
    let bits = |cost: Option<f64>| cost.map(f64::to_bits);
    let carried = facts.carried_from.as_ref();
    let carried = carried.map(|from| (from.attempt_id.as_str(), from.ordinal));
    let shown_carried = participant.carried_from.as_ref();
    let shown_carried = shown_carried.map(|from| (from.attempt_id.as_str(), from.ordinal));
    let fields = [
        ("key", key == participant.key),
        ("model", facts.model == shown(&participant.served.model)),
        ("effort", facts.effort == shown(&participant.effort)),
        (
            "boundary",
            facts.boundary == shown(&participant.served.boundary),
        ),
        (
            "provider",
            provider(facts) == provenance.map(|provenance| provenance.provider.as_str()),
        ),
        ("usage", facts.usage == participant.usage),
        (
            "usage_aggregated",
            facts.usage_aggregated == participant.usage_aggregated,
        ),
        ("cost", bits(facts.cost) == bits(participant.cost)),
        (
            "cost_aggregated",
            facts.cost_aggregated == participant.cost_aggregated,
        ),
        (
            "last_attempt_cost",
            bits(facts.last_attempt_cost) == bits(participant.last_attempt_cost),
        ),
        ("turns", facts.turns == participant.turns),
        (
            "turns_aggregated",
            facts.turns_aggregated == participant.turns_aggregated,
        ),
        ("items", turns_cell(facts) == shown(&participant.turns_cell)),
        ("transcript", facts.transcript == participant.transcript),
        ("outcome", status(facts.outcome) == participant.status),
        ("carried_from", carried == shown_carried),
        ("started_at", facts.started_at.as_deref() == start),
    ];
    fields
        .into_iter()
        .find(|(_, same)| !same)
        .map(|(field, _)| field)
}

/// Every participant of `run`, whose journal is `events`, that `facts`
/// state otherwise than its run view shows it, as `run: key: field`, and
/// how many were compared.
pub(crate) fn disagreements(
    run: &str,
    events: &[EventEnvelope],
    facts: &[SeatFacts],
) -> (Vec<String>, usize) {
    let view = run_view(events, None);
    if facts.len() != view.participants.len() {
        let counts = (facts.len(), view.participants.len());
        return (vec![format!("{run}: {counts:?} participants")], 0);
    }
    let pairs = facts.iter().zip(&view.participants);
    let found = pairs.filter_map(|(facts, participant)| {
        let field = differs(facts, participant, started_at(events, participant))?;
        Some(format!("{run}: {}: {field}", participant.key))
    });
    (found.collect(), facts.len())
}
