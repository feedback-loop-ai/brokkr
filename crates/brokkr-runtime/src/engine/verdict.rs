//! A seat's result admitted as a verdict's record (decision 0109 slice
//! 0109a1): the findings a seat reports are stamped with the digest of
//! the compiled site that reported them, before the result is joined
//! into a panel's aggregate or journaled, and a result the admission
//! refuses fails its attempt with the refusal named, never repaired
//! (decision 0001). No word is refused here and no routing moves.

use brokkr_core::envelope::EventType;
use brokkr_core::findings::{CheckedHead, Covered, EngineWritten, Finding};
use brokkr_core::fold::{RunState, StepObservation, StepRecords};
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::resume::SiteContext;
use super::Engine;
use crate::bundle::Seat;
use crate::bundle::{ExecutableBody, SeatClass, SequenceStep};

/// Ruling 7's `dirty` is true unless the tree check proved the worktree
/// clean at commission time. That check is slice 0109b's, so every
/// record this slice writes says the worktree may be dirty.
const DIRTY_UNTIL_THE_TREE_CHECK: bool = true;

/// Which `covered` field set a gate-produced result records (decision
/// 0109 ruling 7). Whether one is recorded at all is the producing
/// span's: only a gate span's checked observation stamps one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Covering {
    /// A gate with its own head record: the commit its span's
    /// stable-head check accepted.
    Head,
    /// The protected phase: the SHA-256 of that head, pinned at decide to
    /// the `reviewed_heads` the same observation records.
    HeadDigest,
}

impl Covering {
    /// What an effect records in the bundle's protected phase or not.
    pub(super) fn of(protected: bool) -> Covering {
        if protected {
            Covering::HeadDigest
        } else {
            Covering::Head
        }
    }
}

impl Engine {
    /// The covered record of `effect_id`'s latest span, under `covering`:
    /// present only where that span was a gate span whose stable-head
    /// check accepted a head that reads.
    fn covered_of(&self, effect_id: &str, covering: Covering) -> Option<Value> {
        let head = self.observed_head_of(effect_id)?;
        let covered = match covering {
            Covering::Head => Covered::commit(head, DIRTY_UNTIL_THE_TREE_CHECK)?,
            Covering::HeadDigest => Covered::digest_of(head, DIRTY_UNTIL_THE_TREE_CHECK),
        };
        Some(serde_json::to_value(covered).expect("a covered record serializes"))
    }

    /// What the engine stamps on a result of `effect_id` produced by its
    /// latest span: the covered record under `covering`, and, in the
    /// protected phase, the effect's observation as `checked_head` (fix
    /// round 0109a1r3, H1), so the head decide records is durable before
    /// the success is. `step` stamps a sequence step's checkpoint, which
    /// carries the observation only when the step's own span accepted it.
    fn stamps(
        &self,
        effect_id: &str,
        covering: Option<Covering>,
        step: bool,
    ) -> Map<String, Value> {
        let mut stamps = Map::new();
        if let Some(covered) = covering.and_then(|covering| self.covered_of(effect_id, covering)) {
            stamps.insert("covered".into(), covered);
        }
        let checked = match step {
            true => self
                .observed_head_of(effect_id)
                .filter(|_| self.spans.protected),
            false => self.checked_head_of(effect_id),
        };
        if let Some(head) = checked {
            stamps.insert(CHECKED_HEAD.into(), Value::from(head));
        }
        stamps
    }

    /// A sequence step's result before its checkpoint is journaled, with
    /// the covered record of the step's own span when the step was a gate
    /// (fix round 0109a1r2, R2): each gate-produced result is stamped
    /// from the span that produced it, and a protected gate step's
    /// checkpoint journals the head its span accepted.
    pub(super) fn stamp_step_covered(&self, effect_id: &str, mut result: Value) -> Value {
        if let Some(result) = result.as_object_mut() {
            result.extend(self.stamps(effect_id, self.covering, true));
        }
        result
    }

    /// A terminal effect payload with its covered record and its checked
    /// head stamped on a success, before it is appended: each terminal
    /// event spends the covering its dispatch armed, so none outlives its
    /// effect. The head is the one the effect's observation accepted,
    /// never a fresh read; an effect with no such observation records
    /// neither.
    pub(super) fn stamp_covered(&mut self, event_type: EventType, mut payload: Value) -> Value {
        let covering = match event_type {
            EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate => self.covering.take(),
            _ => None,
        };
        if event_type != EventType::EffectSucceeded {
            return payload;
        }
        // The row's mark that its result is this engine's admitted
        // verdict, which a released engine's success never carries (fix
        // round 0109a1r6, W6).
        brokkr_core::findings::stamp_authored(&mut payload);
        let effect_id = payload.get("effect_id").and_then(Value::as_str);
        let stamps = effect_id.map(|effect_id| self.stamps(effect_id, covering, false));
        let result = payload.get_mut("result").and_then(Value::as_object_mut);
        if let (Some(stamps), Some(result)) = (stamps, result) {
            result.extend(stamps);
        }
        payload
    }
}

/// The engine-written field a protected effect's result journals its
/// observation in (seat-record v7).
const CHECKED_HEAD: &str = EngineWritten::CheckedHead.field();

/// Why the protected phase's result is refused at decide (fix round
/// 0109a1r3, H1): it never advances on a head no check of its own effect
/// accepted, and decide reads that head from the journaled result, never
/// from process memory, so a resumed engine decides alike.
#[derive(Debug, Error, PartialEq, Eq)]
pub(super) enum ObservationProblem {
    #[error(
        "the protected result carries no durable checked head of its own effect \
         (decision 0109 ruling 7)"
    )]
    Missing,
    #[error("the protected result's checked head is not a commit id (decision 0109 ruling 7)")]
    Unreadable,
    #[error(
        "the protected effect's durable observations disagree on the head decided on \
         (decision 0109 ruling 7)"
    )]
    Disagrees,
    #[error(
        "the protected effect's gate step record is partial: it journals one of \
         `checked_head` and `covered` without the other (decision 0109 ruling 7)"
    )]
    Partial,
    #[error(
        "the gate step '{step}' the compiled body holds journaled no whole record of its own \
         in this attempt (decision 0109 ruling 7)"
    )]
    Unrecorded { step: String },
    #[error(
        "the protected result names no step its compiled body holds as its producer \
         (decision 0109 ruling 7)"
    )]
    Unattributed,
}

/// The step that produced a protected effect's terminal, read from the
/// compiled body by the name its success journals (fix rounds 0109a1r6,
/// W1, and 0109a1r7, X1), and the records the attempt owes for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Producer<'a> {
    /// A single body, whose success names no step: its terminal is its
    /// own observation, and it owes no step record.
    Effect,
    /// A sequence step the compiled body holds, of `class`, and `owed`:
    /// every gate step at or before it in compiled order, which ran in
    /// this attempt and each journaled one whole record of its own. A
    /// gate step's terminal is its own record's result and equals it,
    /// presence included; a work step's carries no `covered`, and its
    /// head is the last owed record's.
    Step {
        name: &'a str,
        class: SeatClass,
        owed: Vec<&'a str>,
    },
}

impl<'a> Producer<'a> {
    /// The producer of `state`'s terminal in `seat`'s body. The identity
    /// is bound (X1): a sequence's success must name one of its steps,
    /// and a single body's must name none, or no producer stands.
    pub(super) fn of(seat: &'a Seat, state: &RunState) -> Result<Producer<'a>, ObservationProblem> {
        let steps = match seat.body.selected(state.strategy.as_deref()) {
            Some((ExecutableBody::Sequence { steps }, _)) => Some(steps),
            Some((ExecutableBody::Invocation(_), _)) | None => None,
        };
        match (steps, state.ending_step.as_deref()) {
            (None, None) => Ok(Producer::Effect),
            (Some(steps), Some(name)) => Producer::step(steps, name),
            (None, Some(_)) | (Some(_), None) => Err(ObservationProblem::Unattributed),
        }
    }

    /// `name`, one of `steps`, with the gate steps it owes.
    fn step(steps: &'a [SequenceStep], name: &str) -> Result<Producer<'a>, ObservationProblem> {
        let index = (steps.iter().position(|step| step.name == name))
            .ok_or(ObservationProblem::Unattributed)?;
        let owed = steps[..=index]
            .iter()
            .filter(|step| step.class == SeatClass::Gate)
            .map(|step| step.name.as_str())
            .collect();
        let step = &steps[index];
        Ok(Producer::Step {
            name: &step.name,
            class: step.class,
            owed,
        })
    }
}

/// The head a protected `raw_result` was checked on, read from every
/// durable observation of its attempt (fix rounds 0109a1r4, K2, and
/// 0109a1r5, V1): the result's journaled `checked_head`, its `covered`
/// where present, and `step`, the latest gate step record of the attempt
/// the fold carried, or why they do not stand: none recorded, one that is
/// not a commit id, or any two that disagree.
///
/// Every gate step journals its own record, the one that ends the
/// sequence included, so the span that produced each observation is on
/// the record and none is inferred from the terminal: the terminal agrees
/// with the latest record field by field, its `checked_head` always and
/// its `covered` where it carries one (a later work step spends the
/// span's own `covered`, never the effect's head). A legitimately newer
/// final gate is that latest record, so it agrees by construction. An
/// effect with no step record, a single body or a sequence of work steps
/// only, has its terminal as its one observation.
///
/// The terminal is matched to the step that produced it (fix round
/// 0109a1r6, W1): a gate step's terminal is its own record's result, so
/// it equals that record field by field, presence included; a work
/// step's terminal carries no `covered`, and its head is pinned to the
/// last owed gate record.
///
/// What the attempt owes is the compiled body's, never the journal's
/// (fix round 0109a1r7, X1): one whole record for each gate step at or
/// before the producer, so a record removed, blanked or re-tagged in the
/// journal refuses rather than removing its obligation. A malformed
/// record latched in the attempt refuses whatever record followed it, and
/// a single body owes none, so any step record beside it refuses.
pub(super) fn checked_head(
    raw_result: &Value,
    records: &StepRecords,
    producer: &Producer<'_>,
) -> Result<CheckedHead, ObservationProblem> {
    let head = raw_result
        .get(CHECKED_HEAD)
        .ok_or(ObservationProblem::Missing)?;
    let head = CheckedHead::deserialize(head).map_err(|_| ObservationProblem::Unreadable)?;
    let covered = match raw_result.get("covered").map(Covered::deserialize) {
        None => None,
        Some(Ok(covered)) if covered.projects(head.as_str()) => Some(covered),
        Some(_) => return Err(ObservationProblem::Disagrees),
    };
    match &records.malformed {
        Some(StepObservation::Partial) => return Err(ObservationProblem::Partial),
        Some(_) => return Err(ObservationProblem::Disagrees),
        None => {}
    }
    let agrees = matched(producer, records, &head, covered.as_ref())?;
    agrees.then_some(head).ok_or(ObservationProblem::Disagrees)
}

/// Whether the terminal's `head` and `covered` match the record of the
/// step that produced it under `producer`, or why there is none to match:
/// an owed gate record absent, or journaling no whole observation.
fn matched(
    producer: &Producer<'_>,
    records: &StepRecords,
    head: &CheckedHead,
    covered: Option<&Covered>,
) -> Result<bool, ObservationProblem> {
    let (class, owed) = match producer {
        Producer::Effect => return Ok(records.by_step.is_empty()),
        Producer::Step { class, owed, .. } => (*class, owed),
    };
    let mut last = None;
    for gate in owed {
        match records.by_step.get(*gate) {
            Some(StepObservation::Checked { head, covered }) => last = Some((head, covered)),
            _ => {
                let step = gate.to_string();
                return Err(ObservationProblem::Unrecorded { step });
            }
        }
    }
    // A gate producer is the last step it owes (`Producer::step`, bound
    // by the_terminals_producer_is_read_from_the_compiled_body), so `last`
    // is its own record; with no owed record the producer is work.
    Ok(match (class, last) {
        (SeatClass::Gate, Some((seen, stamped))) => seen == head && covered == Some(stamped),
        (SeatClass::Work, Some((seen, _))) => seen == head && covered.is_none(),
        (_, None) => covered.is_none(),
    })
}

/// `report` with its successful result admitted at `site`, the site whose
/// driver produced it, read as a gate or a work site by its compiled
/// class: its findings stamped and handed back typed, or the refusal put
/// in place of the outcome acted on, as a refused checkpoint's is
/// (decision 0034 ruling 6), so the attempt fails, is retried and parks
/// under decision 0006. A report already refused is left as it stands.
pub(super) fn admitted(mut report: AttemptReport, site: Option<&SiteContext>) -> Admitted {
    let unstamped = |report| Admitted {
        report,
        findings: None,
    };
    if report.refused.is_some() {
        return unstamped(report);
    }
    let AttemptOutcome::Succeeded { result } = &mut report.outcome else {
        return unstamped(report);
    };
    match brokkr_core::findings::admit(result, site.map(SiteContext::reporter)) {
        Ok(findings) => Admitted { report, findings },
        Err(refusal) => {
            report.refused = Some(AttemptOutcome::Failed {
                error: format!("the result was refused: {refusal}"),
            });
            unstamped(report)
        }
    }
}

/// An admitted attempt: its report, and beside it the findings its
/// successful result reported, stamped and typed (`None` where it
/// reported none), kept out of the result's `Value` until the journal's
/// boundary writes them (fix round 0109a1r3, L1).
#[derive(Debug)]
pub(crate) struct Admitted {
    pub(crate) report: AttemptReport,
    pub(crate) findings: Option<Vec<Finding>>,
}

impl Admitted {
    /// The report the journal records: its successful result carrying
    /// the stamped findings it was admitted with.
    pub(super) fn journaled(mut self) -> AttemptReport {
        if let (AttemptOutcome::Succeeded { result }, Some(findings)) =
            (&mut self.report.outcome, &self.findings)
        {
            brokkr_core::findings::journal(result, findings);
        }
        self.report
    }
}

#[cfg(test)]
pub(super) mod covered_tests;
#[cfg(test)]
mod inspect_tests;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod observation_tests;
#[cfg(test)]
mod span_tests;
#[cfg(test)]
mod tests;
