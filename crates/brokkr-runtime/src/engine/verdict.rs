//! A seat's result admitted as a verdict's record (decision 0109 slice
//! 0109a1): the findings a seat reports are stamped with the digest of
//! the compiled site that reported them, before the result is joined
//! into a panel's aggregate or journaled, and a result the admission
//! refuses fails its attempt with the refusal named, never repaired
//! (decision 0001). No word is refused here and no routing moves.

use brokkr_core::envelope::EventType;
use brokkr_core::findings::{CheckedHead, Covered, Reporter};
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::resume::SiteContext;
use super::Engine;
use crate::bundle::SeatClass;

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
const CHECKED_HEAD: &str = "checked_head";

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
        "the result's covered head does not pin the head decided on \
         (decision 0109 ruling 7)"
    )]
    Disagrees,
}

/// The head a protected `raw_result` was checked on, read from its
/// journaled `checked_head`, or why it does not stand: none recorded,
/// one that is not a commit id, or a `covered` record that does not read
/// or does not project it.
pub(super) fn checked_head(raw_result: &Value) -> Result<CheckedHead, ObservationProblem> {
    let head = raw_result
        .get(CHECKED_HEAD)
        .ok_or(ObservationProblem::Missing)?;
    let head = CheckedHead::deserialize(head).map_err(|_| ObservationProblem::Unreadable)?;
    match raw_result.get("covered").map(Covered::deserialize) {
        None => Ok(head),
        Some(Ok(covered)) if covered.projects(head.as_str()) => Ok(head),
        Some(_) => Err(ObservationProblem::Disagrees),
    }
}

/// `report` with its successful result admitted at `site`, the site whose
/// driver produced it, read as a gate or a work site by its compiled
/// class: its findings stamped in place, or the refusal put
/// in place of the outcome acted on, as a refused checkpoint's is
/// (decision 0034 ruling 6), so the attempt fails, is retried and parks
/// under decision 0006. A report already refused is left as it stands.
pub(super) fn admitted(mut report: AttemptReport, site: Option<&SiteContext>) -> AttemptReport {
    if report.refused.is_some() {
        return report;
    }
    let AttemptOutcome::Succeeded { result } = &mut report.outcome else {
        return report;
    };
    let reporter = site.map(|site| match site.class {
        SeatClass::Gate => Reporter::Gate(&site.site_ref),
        SeatClass::Work => Reporter::Work,
    });
    if let Err(refusal) = brokkr_core::findings::admit(result, reporter) {
        report.refused = Some(AttemptOutcome::Failed {
            error: format!("the result was refused: {refusal}"),
        });
    }
    report
}

#[cfg(test)]
pub(super) mod covered_tests;
#[cfg(test)]
mod observation_tests;
#[cfg(test)]
mod span_tests;
#[cfg(test)]
mod tests;
