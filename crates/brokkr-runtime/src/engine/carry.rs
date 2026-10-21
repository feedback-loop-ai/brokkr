//! One carry decision, shared by operator admission and dispatch (0093 rulings 1–4).
use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::fold::RunState;
use brokkr_core::EventEnvelope;
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde_json::Value;
use thiserror::Error;

use super::panel::{outcome_over, panel_outcome};
use super::worktree::TreeReading;
use super::{Engine, EngineError};
use crate::bundle::{Aggregate, ExecutableBody, Invocation, SeatBody, StepBody};

pub(crate) mod facts;
pub(crate) mod record;
use facts::{CarryFacts, PanelAttempt, PanelShape};
use record::{CarriedEntry, Head, PanelSpan, Tag};

/// A member retry's refusal, before anything is carried. Its display is also the drive's park reason.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct CarryRefusal(pub(crate) Refusal);

#[derive(Debug, Clone, Error)]
pub(crate) enum RecordId {
    #[error("attempt '{0}'")]
    Attempt(String),
    #[error("command at sequence {0}")]
    Command(u64),
}

#[derive(Debug, Error)]
pub(crate) enum Shape {
    #[error("phase '{phase}' visit {visit} holds no attempt its panel ended")]
    NoAttemptEnded { phase: String, visit: u64 },
    #[error("attempt '{attempt}' succeeded; {park_reason}")]
    Succeeded {
        attempt: String,
        park_reason: String,
    },
    #[error("attempt '{attempt}' ended at step '{step}' after the panel")]
    EndedAtLaterStep { attempt: String, step: String },
    #[error("attempt '{attempt}' records no verdict to carry")]
    NoRecord { attempt: String },
}

#[derive(Debug, Error)]
pub(crate) enum Refusal {
    #[error("phase '{phase}' runs no panel; use plain operator retry")]
    NoPanel { phase: String },
    #[error("panel step '{step}' follows first step '{first}'; use plain operator retry")]
    PanelStepFollows { step: String, first: String },
    #[error("'{name}' is no panel member; the panel's tags are {tags}")]
    UnknownMember { name: String, tags: String },
    #[error("member '{tag}' was named twice")]
    MemberTwice { tag: String },
    #[error("{record} field '{field}' does not read: {error}")]
    RecordUnread {
        record: RecordId,
        field: String,
        error: String,
    },
    #[error("{why}; use plain operator retry")]
    Shape { why: Shape },
    #[error(
        "members {members} have no settled verdict to carry; name them or use plain operator retry"
    )]
    Unsettled { members: String },
    #[error("repository '{}' has no readable reviewed head; use plain operator retry", .repo.display())]
    NoHead { repo: std::path::PathBuf },
    #[error("reviewed head moved: {observations}; use plain operator retry")]
    HeadMoved { observations: String },
    #[error("working tree is not clean: {observations}; use plain operator retry")]
    TreeNotClean { observations: String },
    #[error("inputs moved for effects {effects}; use plain operator retry")]
    InputMoved { effects: String },
    #[error("carrying attempt '{attempt}' changed its reviewed subject: {observations}")]
    SubjectMoved {
        attempt: String,
        observations: String,
    },
}

impl From<Refusal> for CarryRefusal {
    fn from(refusal: Refusal) -> Self {
        Self(refusal)
    }
}

#[derive(Debug, Clone)]
enum Contribution {
    Fresh(Tag),
    Carried {
        tag: Tag,
        result: Value,
        source: String,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct CarryPlan {
    entries: Vec<Contribution>,
    step: Option<String>,
    subject: Option<Head>,
}

impl Contribution {
    fn tag(&self) -> &Tag {
        match self {
            Self::Fresh(tag) | Self::Carried { tag, .. } => tag,
        }
    }
}

impl CarryPlan {
    fn new(
        tags: &[Tag],
        entries: Vec<Contribution>,
        step: Option<String>,
        subject: Option<Head>,
    ) -> Result<Self, EngineError> {
        let actual: Vec<_> = entries.iter().map(Contribution::tag).collect();
        if actual != tags.iter().collect::<Vec<_>>() {
            return Err(join_defect("contributions differ from declared members"));
        }
        let unique: BTreeSet<_> = actual.iter().copied().collect();
        if unique.len() != actual.len() {
            return Err(join_defect("duplicate contribution"));
        }
        Ok(Self {
            entries,
            step,
            subject,
        })
    }

    fn all_fresh(tags: &[Tag], step: Option<String>) -> Self {
        Self::new(
            tags,
            tags.iter().cloned().map(Contribution::Fresh).collect(),
            step,
            None,
        )
        .expect("fresh contributions cover declared members")
    }

    pub(crate) fn fresh(&self) -> Vec<Tag> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                Contribution::Fresh(tag) => Some(tag.clone()),
                Contribution::Carried { .. } => None,
            })
            .collect()
    }

    pub(crate) fn carried(&self) -> Vec<CarriedEntry> {
        self.entries
            .iter()
            .filter_map(|entry| match entry {
                Contribution::Carried { tag, source, .. } => Some(CarriedEntry {
                    member: tag.clone(),
                    attempt_id: source.clone(),
                }),
                Contribution::Fresh(_) => None,
            })
            .collect()
    }

    pub(crate) fn narrow(&self, body: &mut SeatBody, strategy: Option<&str>) {
        match body {
            SeatBody::Invocation(invocation) => self.narrow_invocation(invocation),
            SeatBody::Sequence { steps } => {
                if let Some(step) = steps.first_mut() {
                    if let StepBody::Invocation(invocation) = &mut step.body {
                        self.narrow_invocation(invocation);
                    }
                }
            }
            SeatBody::Select { cases, default } => {
                if let Some(body) = strategy
                    .and_then(|key| cases.get_mut(key))
                    .or(default.as_deref_mut())
                {
                    self.narrow(body, None);
                }
            }
        }
    }

    fn narrow_invocation<T>(&self, invocation: &mut Invocation<T>) {
        if let Invocation::Panel { members, .. } = invocation {
            let fresh = self.fresh();
            members.retain(|member| fresh.contains(&Tag::new(self.step.as_deref(), &member.name)));
        }
    }

    fn join(
        &self,
        reports: Vec<(String, AttemptReport)>,
    ) -> Result<Vec<(String, AttemptOutcome)>, EngineError> {
        let mut fresh = BTreeMap::new();
        for (name, report) in reports {
            if fresh.insert(name, report.settled_outcome()).is_some() {
                return Err(join_defect("duplicate fresh report"));
            }
        }
        let mut joined = Vec::new();
        for entry in &self.entries {
            let (tag, outcome) = match entry {
                Contribution::Fresh(tag) => (
                    tag,
                    fresh
                        .remove(tag.bare(self.step.as_deref()))
                        .ok_or_else(|| join_defect("missing fresh report"))?,
                ),
                Contribution::Carried { tag, result, .. } => (
                    tag,
                    AttemptOutcome::Succeeded {
                        result: result.clone(),
                    },
                ),
            };
            joined.push((tag.bare(self.step.as_deref()).to_owned(), outcome));
        }
        if !fresh.is_empty() {
            return Err(join_defect("foreign fresh report"));
        }
        Ok(joined)
    }
}

fn join_defect(detail: &str) -> EngineError {
    EngineError::Invariant("complete carry plan", detail.to_owned())
}

pub(crate) fn outcome_under(
    aggregate: Aggregate,
    plan: Option<&CarryPlan>,
    reports: Vec<(String, AttemptReport)>,
    span: &PanelSpan,
    attempt: &str,
) -> Result<AttemptOutcome, EngineError> {
    let Some(plan) = plan else {
        return Ok(panel_outcome(aggregate, reports));
    };
    if let Some(subject) = &plan.subject {
        if !span.holds(subject) {
            let observations =
                serde_json::to_string(&(span, subject)).expect("typed subject serializes");
            let refusal = Refusal::SubjectMoved {
                attempt: attempt.to_owned(),
                observations,
            };
            return Ok(AttemptOutcome::Indeterminate {
                reason: refusal.to_string(),
            });
        }
    }
    Ok(outcome_over(aggregate, plan.join(reports)?))
}

pub(crate) fn carry_plan(facts: &CarryFacts, names: &[Tag]) -> Result<CarryPlan, CarryRefusal> {
    let (step, tags) = match &facts.shape {
        PanelShape::Panel { step, tags } => (step, tags),
        PanelShape::NoPanel { phase } => {
            return Err(Refusal::NoPanel {
                phase: phase.clone(),
            }
            .into())
        }
        PanelShape::StepFollows { step, first } => {
            return Err(Refusal::PanelStepFollows {
                step: step.clone(),
                first: first.clone(),
            }
            .into())
        }
    };
    let mut selected = BTreeSet::new();
    for name in names {
        if !tags.contains(name) {
            return Err(Refusal::UnknownMember {
                name: name.0.clone(),
                tags: words(tags),
            }
            .into());
        }
        if !selected.insert(name) {
            return Err(Refusal::MemberTwice {
                tag: name.0.clone(),
            }
            .into());
        }
    }
    if selected.len() == tags.len() {
        return Ok(CarryPlan::all_fresh(tags, step.clone()));
    }
    let parked = parked_attempt(facts, step.as_deref())?;
    let (entries, sources) = contributions(tags, names, parked, &facts.attempts)?;
    identity(facts, parked, &sources)?;
    // The rule walks the declared tags once; plan_construction_and_join_refuse_incomplete_sets
    // controls that invariant without adding a recoverable operator refusal for an engine defect.
    Ok(
        CarryPlan::new(tags, entries, step.clone(), facts.head.clone())
            .expect("contributions cover declared members"),
    )
}

fn words(tags: &[Tag]) -> String {
    tags.iter()
        .map(|tag| tag.0.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parked_attempt<'a>(
    facts: &'a CarryFacts,
    step: Option<&str>,
) -> Result<&'a PanelAttempt, CarryRefusal> {
    let refusal = |why| CarryRefusal(Refusal::Shape { why });
    let parked = facts
        .attempts
        .iter()
        .rev()
        .find(|at| {
            at.succeeded
                || at.record.is_some()
                || step.is_some_and(|step| at.steps.iter().any(|finished| finished == step))
        })
        .ok_or_else(|| {
            refusal(Shape::NoAttemptEnded {
                phase: facts.phase.clone(),
                visit: facts.visit,
            })
        })?;
    if parked.succeeded {
        return Err(refusal(Shape::Succeeded {
            attempt: parked.id.clone(),
            park_reason: parked
                .park_reason
                .clone()
                .unwrap_or_else(|| "no park follows it".into()),
        }));
    }
    if step.is_some_and(|step| parked.steps.iter().any(|finished| finished == step)) {
        return Err(refusal(Shape::EndedAtLaterStep {
            attempt: parked.id.clone(),
            step: facts.next_step(&parked.steps).to_owned(),
        }));
    }
    if parked
        .record
        .as_ref()
        .is_none_or(|record| record.settled_members.is_empty())
        && parked.carried.is_empty()
    {
        return Err(refusal(Shape::NoRecord {
            attempt: parked.id.clone(),
        }));
    }
    Ok(parked)
}

fn contributions<'a>(
    tags: &[Tag],
    names: &[Tag],
    parked: &'a PanelAttempt,
    attempts: &'a [PanelAttempt],
) -> Result<(Vec<Contribution>, Vec<&'a PanelAttempt>), CarryRefusal> {
    let mut entries = Vec::new();
    let mut sources = Vec::new();
    let mut unsettled = Vec::new();
    for tag in tags {
        if names.contains(tag) {
            entries.push(Contribution::Fresh(tag.clone()));
            continue;
        }
        let source = parked
            .record
            .as_ref()
            .filter(|record| record.settled_members.contains_key(tag))
            .map(|_| parked)
            .or_else(|| {
                parked
                    .carried
                    .iter()
                    .find(|entry| entry.member == *tag)
                    .and_then(|entry| {
                        attempts
                            .iter()
                            .find(|at| at.id == entry.attempt_id && at.seq < parked.seq)
                    })
            });
        let verdict = source.and_then(|source| {
            source
                .record
                .as_ref()
                .and_then(|record| record.settled_members.get(tag))
                .map(|result| (source, result))
        });
        match verdict {
            Some((source, result)) => {
                entries.push(Contribution::Carried {
                    tag: tag.clone(),
                    result: result.clone(),
                    source: source.id.clone(),
                });
                if !sources.iter().any(|at: &&PanelAttempt| at.id == source.id) {
                    sources.push(source);
                }
            }
            None => unsettled.push(tag.clone()),
        }
    }
    if !unsettled.is_empty() {
        return Err(Refusal::Unsettled {
            members: words(&unsettled),
        }
        .into());
    }
    Ok((entries, sources))
}

fn identity(
    facts: &CarryFacts,
    parked: &PanelAttempt,
    sources: &[&PanelAttempt],
) -> Result<(), CarryRefusal> {
    let now = facts.head.as_ref().ok_or_else(|| Refusal::NoHead {
        repo: facts.repo.clone(),
    })?;
    let mut observed: Vec<_> = sources.iter().copied().chain([parked]).collect();
    observed.sort_by_key(|at| at.seq);
    observed.dedup_by_key(|at| at.id.clone());
    let moved: Vec<_> = observed
        .iter()
        .filter(|at| {
            at.record.as_ref().is_none_or(|record| {
                record.span.head.at_start.as_ref() != Some(now)
                    || record.span.head.at_end.as_ref() != Some(now)
            })
        })
        .map(|at| {
            format!(
                "{}: {}",
                at.id,
                serde_json::to_string(&at.record.as_ref().map(|record| &record.span.head))
                    .expect("head serializes")
            )
        })
        .collect();
    if !moved.is_empty() {
        return Err(Refusal::HeadMoved {
            observations: format!(
                "{}; now {}",
                moved.join("; "),
                serde_json::to_string(now).expect("head serializes")
            ),
        }
        .into());
    }
    let mut dirty: Vec<_> = observed
        .iter()
        .filter(|at| {
            at.record.as_ref().is_none_or(|record| {
                record.span.tree.at_start != TreeReading::Clean
                    || record.span.tree.at_end != TreeReading::Clean
            })
        })
        .map(|at| {
            format!(
                "{}: {}",
                at.id,
                serde_json::to_string(&at.record.as_ref().map(|record| &record.span.tree))
                    .expect("tree serializes")
            )
        })
        .collect();
    if facts.tree != TreeReading::Clean {
        dirty.push(format!(
            "now {}",
            serde_json::to_string(&facts.tree).expect("tree serializes")
        ));
    }
    if !dirty.is_empty() {
        return Err(Refusal::TreeNotClean {
            observations: dirty.join("; "),
        }
        .into());
    }
    let mut effects = Vec::new();
    for at in sources {
        if !at.input_holds && !effects.contains(&at.effect) {
            effects.push(at.effect.clone());
        }
    }
    if !effects.is_empty() {
        return Err(Refusal::InputMoved {
            effects: effects.join(", "),
        }
        .into());
    }
    Ok(())
}

impl Engine {
    pub(crate) fn carry_selection(
        &self,
        events: &[EventEnvelope],
        state: &RunState,
        effect: &str,
    ) -> Result<Option<CarryPlan>, CarryRefusal> {
        let Some((cutoff, names)) = facts::scope(events, effect)? else {
            return Ok(None);
        };
        let attempts = facts::read_visit(events, state, cutoff)?;
        Ok(Some(carry_plan(
            &self.carry_facts(events, state, attempts),
            &names,
        )?))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod drive_tests;
#[cfg(test)]
pub(crate) mod fixture_tests;
