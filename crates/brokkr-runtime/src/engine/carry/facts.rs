//! Read evidence at the journal edge, then observe the repository without deciding.
use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::findings::EngineMarker;
use brokkr_core::fold::RunState;
use brokkr_core::{EventEnvelope, EventType};
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::super::worktree::{read_tree, TreeReading};
use super::super::{site_kinds, site_label};
use super::record::{CarriedEntry, Head, PanelRecord, RetryArgs, Tag};
use super::{CarryRefusal, Engine, ExecutableBody, Invocation, RecordId, Refusal, StepBody};
use crate::agents::kinds::{SeatKinds, UndeclaredKind, UNDECLARED};

#[derive(Debug)]
pub(crate) enum PanelShape {
    Panel {
        step: Option<String>,
        tags: Vec<Tag>,
        /// Every member's compiled law, by bare name (decision 0109).
        laws: crate::bundle::MemberLaws,
    },
    NoPanel {
        phase: String,
    },
    StepFollows {
        step: String,
        first: String,
    },
}

pub(crate) struct PanelAttempt {
    pub(crate) id: String,
    pub(crate) effect: String,
    pub(crate) seq: u64,
    pub(crate) succeeded: bool,
    pub(crate) record: Option<PanelRecord>,
    pub(crate) carried: Vec<CarriedEntry>,
    pub(crate) steps: Vec<String>,
    /// The members the engine's own marker settled in this attempt: the
    /// mark a settled payload is carried as a verdict by (W2), each with
    /// the site that marker names, which a retry carrying the member
    /// journals as its own (X2).
    pub(crate) marked: BTreeMap<Tag, Option<String>>,
    pub(crate) park_reason: Option<String>,
    pub(crate) input_holds: bool,
    ended_at: Option<u64>,
    /// Who wrote each member's settled result here, by tag ([`Writer`]).
    writers: BTreeMap<Tag, Writer>,
}

/// The seat that wrote a member's settled result, as the current bundle
/// resolves it for the selection its attempt began under: the label its
/// site ran under, and the harness it answers to (the controller's ruling
/// R18 of 2026-11-22).
struct Writer {
    seat: String,
    kinds: SeatKinds,
}

impl PanelAttempt {
    /// Whether `result`, `member`'s settled result, may carry: a carried
    /// result contributes to a new verdict, so one whose kind place names
    /// an undeclared or out-of-grammar kind of the seat that wrote it (v7's
    /// vocabulary for a member the body no longer declares) is not carried,
    /// while one that is not an object has no kind place by the grammar and
    /// carries as on main for the join to poison (the controller's ruling
    /// R23 of 2026-11-22).
    pub(crate) fn admits(&self, member: &Tag, result: &Value) -> bool {
        let writer = self.writers.get(member);
        let seat = writer.map_or(member.0.as_str(), |writer| writer.seat.as_str());
        let kinds = writer.map_or(&UNDECLARED, |writer| &writer.kinds);
        match kinds.admit(result, seat) {
            Ok(()) | Err(UndeclaredKind::Unread { .. }) => true,
            Err(UndeclaredKind::Named { .. }) => false,
        }
    }
}

pub(crate) struct CarryFacts {
    pub(crate) repo: std::path::PathBuf,
    pub(crate) shape: PanelShape,
    pub(crate) attempts: Vec<PanelAttempt>,
    pub(crate) head: Option<Head>,
    pub(crate) tree: TreeReading,
    pub(crate) phase: String,
    pub(crate) visit: u64,
    steps: Vec<String>,
}

impl CarryFacts {
    pub(crate) fn next_step(&self, finished: &[String]) -> &str {
        self.steps
            .iter()
            .find(|step| !finished.contains(step))
            .map(String::as_str)
            .unwrap_or("after the final step")
    }
}

fn unread(record: &RecordId, field: &str, error: impl ToString) -> CarryRefusal {
    Refusal::RecordUnread {
        record: record.clone(),
        field: field.to_owned(),
        error: error.to_string(),
    }
    .into()
}

fn parse<T: DeserializeOwned>(
    value: Value,
    record: &RecordId,
    field: &str,
) -> Result<T, CarryRefusal> {
    serde_json::from_value(value).map_err(|error| unread(record, field, error))
}

pub(crate) fn read_visit(
    events: &[EventEnvelope],
    state: &RunState,
    cutoff: u64,
) -> Result<Vec<PanelAttempt>, CarryRefusal> {
    let phase = state.phase.as_deref();
    let start = events
        .iter()
        .rposition(|event| {
            event.seq < cutoff
                && event.event_type == EventType::PhaseEntered
                && event.payload["phase"].as_str() == phase
        })
        .unwrap_or(0);
    let mut attempts: Vec<PanelAttempt> = Vec::new();
    let mut effects = BTreeSet::new();
    for event in events[start..]
        .iter()
        .take_while(|event| event.seq < cutoff)
    {
        let payload = &event.payload;
        match event.event_type {
            EventType::EffectRequested => {
                if payload["phase"].as_str() == phase {
                    effects.insert(payload["effect_id"].as_str().unwrap_or_default().to_owned());
                }
            }
            EventType::EffectStarted => record_start(&mut attempts, &effects, event)?,
            EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate => record_end(&mut attempts, event)?,
            EventType::EffectCheckpointed => record_step(&mut attempts, payload),
            EventType::RunParked => record_park(&mut attempts, payload),
            _ => {}
        }
    }
    Ok(attempts)
}

/// A start of one of this visit's effects opens an attempt; any other start is passed over.
fn record_start(
    attempts: &mut Vec<PanelAttempt>,
    effects: &BTreeSet<String>,
    event: &EventEnvelope,
) -> Result<(), CarryRefusal> {
    let payload = &event.payload;
    let effect = payload["effect_id"].as_str().unwrap_or_default();
    if !effects.contains(effect) {
        return Ok(());
    }
    let id = payload["attempt_id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let carried = match payload.get("carried") {
        Some(value) => parse(value.clone(), &RecordId::Attempt(id.clone()), "carried")?,
        None => Vec::new(),
    };
    attempts.push(PanelAttempt {
        id,
        effect: effect.to_owned(),
        seq: event.seq,
        succeeded: false,
        record: None,
        carried,
        steps: Vec::new(),
        marked: BTreeMap::new(),
        park_reason: None,
        input_holds: false,
        ended_at: None,
        writers: BTreeMap::new(),
    });
    Ok(())
}

/// A terminal names the attempt it ends; one naming no attempt of this visit is passed over.
fn record_end(attempts: &mut [PanelAttempt], event: &EventEnvelope) -> Result<(), CarryRefusal> {
    let payload = &event.payload;
    if let Some(at) = attempts
        .iter_mut()
        .find(|at| payload["attempt_id"].as_str() == Some(&at.id))
    {
        at.succeeded = event.event_type == EventType::EffectSucceeded;
        at.ended_at = Some(event.seq);
        at.record = terminal_record(payload, &at.id)?;
    }
    Ok(())
}

/// The first park after an attempt ended is the park that attempt caused.
fn record_park(attempts: &mut [PanelAttempt], payload: &Value) {
    for at in attempts {
        if at.ended_at.is_some() && at.park_reason.is_none() {
            at.park_reason = payload["reason"].as_str().map(str::to_owned);
        }
    }
}

/// A checkpoint of one of this visit's attempts: a finished step's name,
/// and each member the engine's own marker settles (fix round 0109a1r6,
/// W2), each read by the one check inspect reads it by, the engine's
/// authorship (fix round 0109a1r4, K1; #1212): a driver's checkpoint that
/// names a marker's word is no step record.
fn record_step(attempts: &mut [PanelAttempt], payload: &Value) {
    let Some(at) = attempts
        .iter_mut()
        .find(|at| payload["attempt_id"].as_str() == Some(&at.id))
    else {
        return;
    };
    let checkpoint = &payload["checkpoint"];
    if let Some(member) = EngineMarker::settled_member(checkpoint) {
        let site = checkpoint["site_ref"].as_str().map(str::to_owned);
        at.marked.insert(Tag(member.to_owned()), site);
    }
    if EngineMarker::authored(checkpoint) != Some(EngineMarker::StepFinished) {
        return;
    }
    if let Some(step) = checkpoint["step_name"].as_str() {
        at.steps.push(step.to_owned());
    }
}

fn terminal_record(payload: &Value, id: &str) -> Result<Option<PanelRecord>, CarryRefusal> {
    let record = RecordId::Attempt(id.to_owned());
    let fields = ["tree", "head", "settled_members"];
    if !fields.iter().any(|key| payload.get(key).is_some()) {
        return Ok(None);
    }
    if payload.get("tree").is_none() {
        return Err(unread(&record, "tree", "missing beside panel evidence"));
    }
    // Parse each field by the contract type before assembling the record, so the error names its field.
    let tree = parse(payload["tree"].clone(), &record, "tree")?;
    let head = match payload.get("head") {
        Some(value) => super::record::parse_heads(value.clone())
            .map_err(|error| unread(&record, "head", error))?,
        None => Default::default(),
    };
    let settled_members = match payload.get("settled_members") {
        Some(value) => parse(value.clone(), &record, "settled_members")?,
        None => BTreeMap::new(),
    };
    Ok(Some(PanelRecord {
        span: super::record::PanelSpan { tree, head },
        settled_members,
    }))
}

pub(crate) fn scope(
    events: &[EventEnvelope],
    effect: &str,
) -> Result<Option<(u64, Vec<Tag>)>, CarryRefusal> {
    let Some(request) = events.iter().rposition(|event| {
        event.event_type == EventType::EffectRequested && event.payload["effect_id"] == effect
    }) else {
        return Ok(None);
    };
    for event in events[..request].iter().rev() {
        if event.event_type == EventType::EffectRequested {
            break;
        }
        if event.event_type != EventType::OperatorAccepted {
            continue;
        }
        if let Some(command) = super::super::operator::command_for(events, event) {
            if command.payload["command"] != "retry" {
                continue;
            }
            let args: RetryArgs = parse(
                command
                    .payload
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
                &RecordId::Command(command.seq),
                "args",
            )?;
            return Ok(args.members.map(|members| (events[request].seq, members)));
        }
    }
    Ok(None)
}

impl Engine {
    pub(crate) fn carry_facts(
        &self,
        events: &[EventEnvelope],
        state: &RunState,
        mut attempts: Vec<PanelAttempt>,
    ) -> CarryFacts {
        let phase = state.phase.clone().unwrap_or_default();
        let selected = (self.bundle.seats.get(&phase))
            .and_then(|seat| seat.body.selected(state.strategy.as_deref()));
        for at in &mut attempts {
            if let Some((body, case)) = selected {
                at.writers = self.result_writers(events, at, body, (&phase, case));
            }
            let digest = events
                .iter()
                .find(|event| {
                    event.event_type == EventType::EffectRequested
                        && event.payload["effect_id"] == at.effect
                })
                .and_then(|event| event.payload["input_digest"].as_str());
            at.input_holds = self
                .seat_input(state, &phase, &at.effect)
                .ok()
                .is_some_and(|input| {
                    Some(brokkr_core::canonical::sha256_hex(&input).as_str()) == digest
                });
        }
        let body = selected.map(|(body, _)| body);
        let shape = panel_shape(body, &phase);
        let steps = match body {
            Some(ExecutableBody::Sequence { steps }) => {
                steps.iter().map(|step| step.name.clone()).collect()
            }
            _ => Vec::new(),
        };
        CarryFacts {
            repo: self.repo.clone(),
            shape,
            attempts,
            head: Head::observe(&self.repo),
            tree: read_tree(&self.repo),
            visit: state.visits.get(&phase).copied().unwrap_or_default(),
            phase,
            steps,
        }
    }

    /// Who wrote each member's result in `at`, a panel attempt of the seat
    /// `seat` running `body` (its case `case`): every member site's label
    /// and harness, as this bundle resolves them for the candidates the
    /// journal selected before `at` began, which is the selection it ran.
    fn result_writers(
        &self,
        events: &[EventEnvelope],
        at: &PanelAttempt,
        body: ExecutableBody<'_>,
        (seat, case): (&str, Option<&str>),
    ) -> BTreeMap<Tag, Writer> {
        let began = &events[..events.partition_point(|event| event.seq < at.seq)];
        let (selection, _, _) = super::super::select_candidates(began, &at.effect, body);
        let site_name = super::super::dispatch::site_name(seat, case);
        // The class a site is read as names no harness, so `gate` is moot.
        let sites = super::super::resume::structural_sites(body, seat, case, false);
        let writer = |entry: super::super::resume::StructuralSite| {
            let kinds = site_kinds(&self.bundle, &selection, &entry.site, &entry.command);
            let seat = site_label(&site_name, &entry.site);
            Some((Tag(entry.site?), Writer { seat, kinds }))
        };
        sites.into_iter().filter_map(writer).collect()
    }
}

fn panel_shape(body: Option<ExecutableBody<'_>>, phase: &str) -> PanelShape {
    match body {
        Some(ExecutableBody::Invocation(Invocation::Panel { members, .. })) => PanelShape::Panel {
            step: None,
            tags: members
                .iter()
                .map(|member| Tag::new(None, &member.name))
                .collect(),
            laws: crate::bundle::laws_of(members),
        },
        Some(ExecutableBody::Sequence { steps }) => {
            for (index, step) in steps.iter().enumerate() {
                if let StepBody::Invocation(Invocation::Panel { members, .. }) = &step.body {
                    if index != 0 {
                        return PanelShape::StepFollows {
                            step: step.name.clone(),
                            first: steps[0].name.clone(),
                        };
                    }
                    return PanelShape::Panel {
                        step: Some(step.name.clone()),
                        tags: members
                            .iter()
                            .map(|member| Tag::new(Some(&step.name), &member.name))
                            .collect(),
                        laws: crate::bundle::laws_of(members),
                    };
                }
            }
            PanelShape::NoPanel {
                phase: phase.to_owned(),
            }
        }
        Some(ExecutableBody::Invocation(Invocation::Single { .. })) | None => PanelShape::NoPanel {
            phase: phase.to_owned(),
        },
    }
}

#[cfg(test)]
mod contract_lifecycle_tests;
