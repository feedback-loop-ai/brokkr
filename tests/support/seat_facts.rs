//! The run view's participants, held to the facts the writer's seat rows
//! group into (#753; decision 0089 ruling 4 at the projection's level):
//! each participant's facts compared with what `run_view` shows of it,
//! field for field, a field that differs named with its run, and the live
//! block with what the rows say of the attempt in flight, each phase's
//! inner columns with the steps and tags they list, and each visit's
//! models with the ones they list (slice 2b). brokkr-view holds every committed and hand-built journal to it,
//! brokkr-runtime every engine scenario's carried account, and brokkr-cli
//! every fixture's rows read from the store.

use std::collections::BTreeMap;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::seat::row::{
    facts, inner, live, models, Member, Outcome, Provider, SeatFacts, SeatRow,
};
use brokkr_view::{fmt_dur, run_view, short_target, Cell, Participant, RunView};

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

/// Whether `participant`'s activity is what these facts show: a working
/// participant's last turn, its tool and its target; or a member's clock,
/// from its first checkpoint to its finish or, once its effect
/// `concluded`, its last checkpoint; and no tool otherwise. A seat's clock
/// and its result read the trail, and a carried member's its source.
fn activity_shown(facts: &SeatFacts, concluded: bool, participant: &Participant) -> bool {
    let activity = &participant.activity;
    let target = activity
        .target_full
        .as_ref()
        .or(activity.target_short.as_ref());
    match (&facts.tool, participant.working) {
        (Some(tool), true) => {
            let text = match facts.target.as_deref().map(short_target) {
                Some(short) => format!("{tool} · {short}"),
                None => tool.clone(),
            };
            return activity.tool.as_ref() == Some(tool)
                && target == facts.target.as_ref()
                && activity.text == text;
        }
        _ if activity.tool.is_some() => return false,
        _ => {}
    }
    if facts.member == Member::Seat || participant.carried_from.is_some() {
        return true;
    }
    let finished = facts.finish.as_ref().map(|finish| finish.at.as_str());
    let end = finished.or(facts.last_at.as_deref().filter(|_| concluded));
    let clock = facts.started_at.as_deref().zip(end);
    match clock.and_then(|(from, to)| fmt_dur(from, to)) {
        Some(duration) => activity.text == duration,
        None => activity.absent,
    }
}

/// Whether `participant`'s terminal line is a member's finish as these
/// facts keep it, where its outcome is no JSON null, and its effect's
/// terminal otherwise, which the trail states.
fn terminal_shown(facts: &SeatFacts, participant: &Participant) -> bool {
    let line = &participant.terminal_line.text;
    let finish = facts
        .finish
        .as_ref()
        .filter(|_| facts.member != Member::Seat);
    match finish.and_then(|finish| Some((finish.outcome.as_ref()?, &finish.at))) {
        Some((outcome, at)) => *line == format!("panel-member-finished · {outcome} · {at}"),
        None => !line.starts_with("panel-member-finished"),
    }
}

/// The session the view names: a Claude session's transcript locator, or
/// the one its last session record names.
fn session_id(facts: &SeatFacts) -> Option<&str> {
    let transcript = facts.transcript.as_ref();
    let claude = transcript.filter(|transcript| transcript.kind == "claude-session");
    let locator = claude.map(|transcript| transcript.locator.as_str());
    locator
        .filter(|locator| !locator.is_empty())
        .or(facts.session_id.as_deref())
}

/// The first field `facts` and `participant` disagree on, by name, its
/// effect `concluded` or not.
fn differs(
    facts: &SeatFacts,
    concluded: bool,
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
        (
            "last_at",
            facts.last_at.as_deref()
                == participant
                    .checkpoints
                    .last()
                    .map(|row| row.recorded_at.as_str()),
        ),
        ("activity", activity_shown(facts, concluded, participant)),
        ("terminal_line", terminal_shown(facts, participant)),
        (
            "session_id",
            session_id(facts) == participant.session_id.as_deref(),
        ),
    ];
    fields
        .into_iter()
        .find(|(_, same)| !same)
        .map(|(field, _)| field)
}

/// Whether `event` is a terminal of the effect and attempt `start` names.
fn concludes(event: &EventEnvelope, start: &EventEnvelope) -> bool {
    let terminal = matches!(
        event.event_type,
        EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
    );
    let names = |key: &str| event.payload.get(key) == start.payload.get(key);
    terminal && names("effect_id") && names("attempt_id")
}

/// The live block `rows` show, each line its label and its text, labelled
/// as the trail reads the attempt in flight: the run's last start, while
/// no terminal of its effect and attempt followed it, its seat the last
/// that effect was asked by that names one, else `?`; one bare line for
/// that seat while the rows hold no checkpoint of it yet.
fn live_lines(events: &[EventEnvelope], rows: &[SeatRow]) -> Vec<(String, String)> {
    let mut starts = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectStarted);
    let Some(start) = starts.next_back() else {
        return Vec::new();
    };
    let mut after = events.iter().filter(|event| event.seq > start.seq);
    if after.any(|event| concludes(event, start)) {
        return Vec::new();
    }
    let asked = events.iter().filter(|event| {
        event.event_type == EventType::EffectRequested
            && event.payload.get("effect_id") == start.payload.get("effect_id")
    });
    let named = asked.filter_map(|event| event.payload["seat"].as_str());
    let mut named = named.filter(|seat| !seat.is_empty());
    let seat = named.next_back().unwrap_or("?");
    let live = live(rows);
    if live.is_empty() {
        return vec![(seat.to_string(), format!("{seat} · working"))];
    }
    let line = |(member, turn): (Member, Option<(String, String)>)| {
        let label = match member {
            Member::Seat => seat.to_string(),
            Member::Named(member) => format!("{seat}:{member}"),
        };
        let text = match turn {
            Some((turn, tool)) => format!("{label} · turn {turn} · {tool}"),
            None => format!("{label} · working"),
        };
        (label, text)
    };
    live.into_iter().map(line).collect()
}

/// The rail's columns of a phase whose newest effect is `effect_id`, from
/// the steps and tags `rows` keep of it, each column as its label and its
/// nodes' keys: each finished step, then each step a tag names that none
/// finished, a step's tagged members under its label, and a step alone a
/// node of its own; the tags in one column where no step finished; else
/// the seat's node.
fn columns(rows: &[SeatRow], effect_id: &str) -> Vec<(Option<String>, Vec<String>)> {
    let (finished, tags) = inner(rows, effect_id);
    let key = |tag: &str| format!("{effect_id}:{tag}");
    if finished.is_empty() {
        let keys: Vec<String> = tags.iter().map(|tag| key(tag)).collect();
        let keys = if keys.is_empty() {
            vec![effect_id.to_string()]
        } else {
            keys
        };
        return vec![(None, keys)];
    }
    let mut steps: Vec<(String, Vec<String>)> = finished
        .into_iter()
        .map(|step| (step, Vec::new()))
        .collect();
    for tag in &tags {
        let step = tag.split_once(':').map_or(tag.as_str(), |(step, _)| step);
        if !steps.iter().any(|(name, _)| name == step) {
            steps.push((step.to_string(), Vec::new()));
        }
        if step != tag {
            let at = steps.iter().position(|(name, _)| name == step);
            steps[at.unwrap()].1.push(key(tag));
        }
    }
    let column = |(step, keys): (String, Vec<String>)| {
        if keys.is_empty() {
            (None, vec![key(&step)])
        } else {
            (Some(step), keys)
        }
    };
    steps.into_iter().map(column).collect()
}

/// Each phase of `view` whose inner columns or plainness are not what
/// `rows` keep of the effect `events` last asked for in it, as `run: phase
/// name`, and how many phases were compared.
fn phases_differ(
    run: &str,
    events: &[EventEnvelope],
    rows: &[SeatRow],
    view: &RunView,
) -> (Vec<String>, usize) {
    let asked = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested);
    let newest: BTreeMap<&str, &str> = asked
        .filter_map(|event| {
            Some((
                event.payload["phase"].as_str()?,
                event.payload["effect_id"].as_str()?,
            ))
        })
        .collect();
    let found = view.phases.iter().filter(|phase| {
        let kept = newest
            .get(phase.name.as_str())
            .map_or_else(Vec::new, |effect_id| columns(rows, effect_id));
        let shown: Vec<(Option<String>, Vec<String>)> = phase
            .columns
            .iter()
            .map(|column| {
                (
                    column.label.clone(),
                    column.nodes.iter().map(|node| node.key.clone()).collect(),
                )
            })
            .collect();
        let plain = kept.len() < 2 && kept.first().is_none_or(|(_, keys)| keys.len() == 1);
        kept != shown || plain != phase.plain
    });
    (
        found
            .map(|phase| format!("{run}: phase {}", phase.name))
            .collect(),
        view.phases.len(),
    )
}

/// Each visit of `view`'s path whose models are not the ones `rows` keep
/// of the requests asked in it, from its entry to the next, as `run:
/// visit n`, and how many visits were compared.
fn visits_differ(
    run: &str,
    events: &[EventEnvelope],
    rows: &[SeatRow],
    view: &RunView,
) -> (Vec<String>, usize) {
    let entered = events
        .iter()
        .filter(|event| event.event_type == EventType::PhaseEntered);
    let entered: Vec<u64> = entered.map(|event| event.seq).collect();
    let path = &view.dashboard.path;
    if entered.len() != path.len() {
        return (vec![format!("{run}: {} visits", path.len())], 0);
    }
    let ends = entered.iter().skip(1).copied().chain([u64::MAX]);
    let kept = entered.iter().zip(ends).map(|(from, to)| {
        let models: Vec<String> = models(rows, *from, to).into_iter().collect();
        (!models.is_empty()).then(|| models.join(", "))
    });
    let differ = kept
        .zip(path)
        .enumerate()
        .filter(|(_, (kept, visit))| *kept != visit.model);
    (
        differ.map(|(at, _)| format!("{run}: visit {at}")).collect(),
        path.len(),
    )
}

/// Every participant of `run`, whose journal is `events`, that the facts
/// `rows` group into state otherwise than its run view shows it, as `run:
/// key: field`, then the live block where `rows` show another, as `run:
/// live`, then each phase and each visit whose lists differ, and how many
/// participants, phases and visits were compared.
pub(crate) fn disagreements(
    run: &str,
    events: &[EventEnvelope],
    rows: &[SeatRow],
) -> (Vec<String>, usize) {
    let facts = facts(rows);
    let view = run_view(events, None);
    let (phases, phases_compared) = phases_differ(run, events, rows, &view);
    let (visits, visits_compared) = visits_differ(run, events, rows, &view);
    if facts.len() != view.participants.len() {
        let counts = (facts.len(), view.participants.len());
        return (vec![format!("{run}: {counts:?} participants")], 0);
    }
    let concluded = |effect_id: &str| {
        let mut seats = facts.iter().filter(|seat| seat.member == Member::Seat);
        let seat = seats.find(|seat| seat.effect_id == effect_id);
        seat.is_some_and(|seat| seat.outcome != Outcome::Working)
    };
    let pairs = facts.iter().zip(&view.participants);
    let mut found: Vec<String> = pairs
        .filter_map(|(facts, participant)| {
            let start = started_at(events, participant);
            let field = differs(facts, concluded(&facts.effect_id), participant, start)?;
            Some(format!("{run}: {}: {field}", participant.key))
        })
        .collect();
    let shown: Vec<(String, String)> = view
        .live
        .iter()
        .map(|line| (line.label.clone(), line.text.clone()))
        .collect();
    if live_lines(events, rows) != shown {
        found.push(format!("{run}: live"));
    }
    found.extend(phases.into_iter().chain(visits));
    (found, facts.len() + phases_compared + visits_compared)
}
