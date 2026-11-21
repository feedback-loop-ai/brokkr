//! A seat's result admitted as a verdict's record (decision 0109 slice
//! 0109a1): the findings a seat reports are stamped with the digest of
//! the compiled site that reported them, before the result is joined
//! into a panel's aggregate or journaled, and a result the admission
//! refuses fails its attempt with the refusal named, never repaired
//! (decision 0001). No word is refused here and no routing moves.

use brokkr_core::envelope::EventType;
use brokkr_core::findings::{CheckedHead, Covered, CoveredHead, EngineWritten, Finding};
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
    #[error(
        "the protected result names the step '{step}' as its producer, and its compiled body \
         does not end on that step with that result (decision 0109 ruling 7)"
    )]
    Unlawful { step: String },
    #[error(
        "the protected effect's attempt journaled a record of the step '{step}', which its \
         compiled body does not run before its result's producer (decision 0109 ruling 7)"
    )]
    Beyond { step: String },
    #[error(
        "the protected seat's compiled body resolves no case under the run's strategy \
         (decision 0109 ruling 7)"
    )]
    Unselected,
    #[error(
        "the protected gate's result carries no `covered` head digest, the form its \
         dispatch armed (decision 0109 ruling 7)"
    )]
    Uncovered,
    #[error(
        "the protected effect's step records are journaled out of their compiled order \
         (decision 0109 ruling 7)"
    )]
    Reordered,
}

/// The step that produced a protected effect's terminal, read from the
/// compiled body by the name its success journals (fix rounds 0109a1r6,
/// W1, and 0109a1r7, X1), at a position the body may end on (fix round
/// 0109a1r8, M1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Producer<'a> {
    /// A single body, whose success names no step: its terminal is its
    /// own observation, and it owes no step record. `gate` is its compiled
    /// class as dispatch reads it to arm covering (fix round 0109a1r8,
    /// M2): a gate body owes the `covered` digest its dispatch armed, and
    /// a work body carries none (fix round 0109a1r4, K3).
    Effect { gate: bool },
    /// A sequence the compiled body holds, every step of it by name and
    /// class in compiled order, ended at `at`: its last step, or a step
    /// whose compiled `ends_on` holds the terminal's word. The tag names
    /// the step; whether the body may end there, what it owes and what it
    /// decides on are the compiled body's (M1).
    Step {
        steps: Vec<(&'a str, SeatClass)>,
        at: usize,
    },
    /// A sequence ended at `at`, a work step before every gate step the
    /// body holds, on a word in that step's compiled `ends_on` that is a
    /// compiled hard word: every table arm for it is hard and stops the run
    /// (decision 0050 rulings 5 and 7). A stop ships no head, so it owes
    /// no gate record and decides on none (#1221, #1212).
    Stop {
        steps: Vec<(&'a str, SeatClass)>,
        at: usize,
    },
}

impl<'a> Producer<'a> {
    /// The producer of `state`'s terminal `word` in `seat`'s body. The
    /// identity is bound (X1): a sequence's success must name one of its
    /// steps, at a position its compiled endings allow (M1), and a single
    /// body's must name none, or no producer stands. A body that resolves
    /// no case has none either (M1).
    pub(super) fn of(
        seat: &'a Seat,
        state: &RunState,
        word: Option<&str>,
    ) -> Result<Producer<'a>, ObservationProblem> {
        let strategy = state.strategy.as_deref();
        let (body, _) = (seat.body.selected(strategy)).ok_or(ObservationProblem::Unselected)?;
        match (body, state.ending_step.as_deref()) {
            (ExecutableBody::Invocation(_), None) => Ok(Producer::Effect {
                gate: super::arms_effect_gate_head(&body, seat, strategy),
            }),
            (ExecutableBody::Sequence { steps }, Some(name)) => Producer::step(steps, name, word),
            (ExecutableBody::Invocation(_), Some(_)) | (ExecutableBody::Sequence { .. }, None) => {
                Err(ObservationProblem::Unattributed)
            }
        }
    }

    /// `name`, one of `steps`, where the body may end with `word`: the
    /// last step, or one whose compiled `ends_on` holds it. A work step
    /// before every gate step, ended on a compiled hard word, is a stop.
    fn step(
        steps: &'a [SequenceStep],
        name: &str,
        word: Option<&str>,
    ) -> Result<Producer<'a>, ObservationProblem> {
        let at = (steps.iter().position(|step| step.name == name))
            .ok_or(ObservationProblem::Unattributed)?;
        let ends_on = |word: &str| steps[at].ends_on.iter().any(|ending| ending == word);
        if at + 1 != steps.len() && !word.is_some_and(ends_on) {
            let step = name.to_string();
            return Err(ObservationProblem::Unlawful { step });
        }
        // A step before a gate is not the last, so its word is in `ends_on`.
        let hard = |word: &str| steps[at].hard_results.iter().any(|hard| hard == word);
        let gated = steps.iter().any(|step| step.class == SeatClass::Gate);
        let before_gates = steps[..=at]
            .iter()
            .all(|step| step.class == SeatClass::Work);
        let stops = gated && before_gates && word.is_some_and(hard);
        let steps = (steps.iter())
            .map(|step| (step.name.as_str(), step.class))
            .collect();
        Ok(match stops {
            true => Producer::Stop { steps, at },
            false => Producer::Step { steps, at },
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
///
/// No row's tag sets the obligation (fix round 0109a1r8, M1): the
/// producer stands only at a position the compiled body may end on, every
/// record of the attempt is reconciled against that position, a record
/// past it included, and a body that holds a gate step decides on its
/// latest whole gate record's head, which the terminal's own is compared
/// to and never replaces. A work-only body has no gate record, so its
/// terminal is its one observation (fix round 0109a1r4, K3).
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
    unlatched(records)?;
    let decided = match producer {
        Producer::Effect { gate } => effect_head(*gate, records, head, covered.as_ref())?,
        // A stop owes a gate record it never has, so asked for a head it
        // refuses as before; `decided_head` never asks.
        Producer::Step { steps, at } | Producer::Stop { steps, at } => {
            reconciled(steps, *at, records)?;
            step_head(steps, *at, records, head, covered.as_ref())?
        }
    };
    decided.ok_or(ObservationProblem::Disagrees)
}

/// The head decide records for a protected terminal: none for a stop
/// (#1221, #1212), whose table arms all stop the run so no ship reads a
/// head, once every record of its attempt is reconciled against its
/// position; otherwise the head [`checked_head`] reads.
pub(super) fn decided_head(
    raw_result: &Value,
    records: &StepRecords,
    producer: &Producer<'_>,
) -> Result<Option<CheckedHead>, ObservationProblem> {
    match producer {
        Producer::Stop { steps, at } => {
            unlatched(records)?;
            reconciled(steps, *at, records).map(|()| None)
        }
        Producer::Effect { .. } | Producer::Step { .. } => {
            checked_head(raw_result, records, producer).map(Some)
        }
    }
}

/// Refuse an attempt whose fold latched a malformed step record.
fn unlatched(records: &StepRecords) -> Result<(), ObservationProblem> {
    match &records.malformed {
        Some(StepObservation::Partial) => Err(ObservationProblem::Partial),
        Some(_) => Err(ObservationProblem::Disagrees),
        None => Ok(()),
    }
}

/// The head a single body decides on, its terminal's own, if no step
/// record stands beside it and its `covered` is the form its dispatch
/// armed (fix round 0109a1r8, M2): a gate body's digest, which an absent
/// or commit-form record is not, and none for a work body.
fn effect_head(
    gate: bool,
    records: &StepRecords,
    head: CheckedHead,
    covered: Option<&Covered>,
) -> Result<Option<CheckedHead>, ObservationProblem> {
    let digest = covered.map(|covered| matches!(covered.head(), CoveredHead::Digest(_)));
    match (gate, digest) {
        (true, Some(true)) | (false, None) => Ok(records.by_step.is_empty().then_some(head)),
        (true, Some(false) | None) => Err(ObservationProblem::Uncovered),
        (false, Some(_)) => Ok(None),
    }
}

/// The head a sequence ended at `at` decides on, if its terminal `head`
/// and `covered` match the record of the step that produced it: the
/// latest whole gate record's head where the body holds a gate step, or
/// the terminal's own in a work-only body. Its error is why there is no
/// gate record to decide on: one owed absent or journaling no whole
/// observation.
fn step_head(
    steps: &[(&str, SeatClass)],
    at: usize,
    records: &StepRecords,
    head: CheckedHead,
    covered: Option<&Covered>,
) -> Result<Option<CheckedHead>, ObservationProblem> {
    let mut last = None;
    for gate in gates(&steps[..=at]) {
        match records.by_step.get(gate) {
            // Dispatch arms a protected gate step's covering in the digest
            // form, so a whole record owes it (fix round 0109a1r8, M2).
            Some(StepObservation::Checked { covered, .. })
                if !matches!(covered.head(), CoveredHead::Digest(_)) =>
            {
                return Err(ObservationProblem::Uncovered);
            }
            Some(StepObservation::Checked { head, covered }) => last = Some((head, covered)),
            _ => {
                let step = gate.to_string();
                return Err(ObservationProblem::Unrecorded { step });
            }
        }
    }
    // A gate producer is the last gate step at or before `at`, so `last`
    // is its own record; with none, the producer is a work step before
    // every gate the body holds, or the body holds none.
    Ok(match (steps[at].1, last, gates(steps).first()) {
        (SeatClass::Gate, Some((seen, stamped)), _) => {
            (*seen == head && covered == Some(stamped)).then(|| seen.clone())
        }
        (SeatClass::Work, Some((seen, _)), _) => {
            (*seen == head && covered.is_none()).then(|| seen.clone())
        }
        (_, None, Some(first)) => {
            let step = first.to_string();
            return Err(ObservationProblem::Unrecorded { step });
        }
        (_, None, None) => covered.is_none().then_some(head),
    })
}

/// The gate steps of `steps`, in compiled order.
fn gates<'a>(steps: &[(&'a str, SeatClass)]) -> Vec<&'a str> {
    (steps.iter())
        .filter(|(_, class)| *class == SeatClass::Gate)
        .map(|(name, _)| *name)
        .collect()
}

/// Every record of the attempt against the sequence ended at `at` (fix
/// round 0109a1r8, M1): each names a step the compiled body runs before
/// `at`, or `at` itself when it is a gate step, which journals its own
/// record; a work step's record journals no observation. The records were
/// journaled in the compiled order of their steps (fix round 0109a1r8,
/// M3), as a sequence runs its steps one after another.
fn reconciled(
    steps: &[(&str, SeatClass)],
    at: usize,
    records: &StepRecords,
) -> Result<(), ObservationProblem> {
    let position = |name: &str| steps.iter().position(|(step, _)| *step == name);
    for (name, seen) in &records.by_step {
        let gate = steps[at].1 == SeatClass::Gate;
        let ran = position(name).filter(|index| *index < at || (*index == at && gate));
        let Some(index) = ran else {
            let step = name.clone();
            return Err(ObservationProblem::Beyond { step });
        };
        if steps[index].1 == SeatClass::Work && *seen != StepObservation::Unobserved {
            return Err(ObservationProblem::Disagrees);
        }
    }
    let journaled: Vec<Option<usize>> = (records.order.iter()).map(|name| position(name)).collect();
    match journaled.windows(2).all(|pair| pair[0] < pair[1]) {
        true => Ok(()),
        false => Err(ObservationProblem::Reordered),
    }
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
mod stop_tests;
#[cfg(test)]
mod tests;
