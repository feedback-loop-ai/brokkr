//! A seat's result admitted as a verdict's record (decision 0109 slice
//! 0109a1): the findings a seat reports are stamped with the digest of
//! the compiled site that reported them, before the result is joined
//! into a panel's aggregate or journaled, and a result the admission
//! refuses fails its attempt with the refusal named, never repaired
//! (decision 0001). No word is refused here and no routing moves.

use brokkr_core::envelope::EventType;
use brokkr_core::findings::Covered;
use brokkr_core::fold::RunState;
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde::Deserialize;
use serde_json::Value;

use super::resume::SiteContext;
use super::Engine;

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

    /// A sequence step's result before its checkpoint is journaled, with
    /// the covered record of the step's own span when the step was a gate
    /// (fix round 0109a1r2, R2): each gate-produced result is stamped
    /// from the span that produced it.
    pub(super) fn stamp_step_covered(&self, effect_id: &str, mut result: Value) -> Value {
        let covered = self
            .covering
            .and_then(|covering| self.covered_of(effect_id, covering));
        if let (Some(covered), Some(result)) = (covered, result.as_object_mut()) {
            result.insert("covered".into(), covered);
        }
        result
    }

    /// A terminal effect payload with its covered record stamped on a
    /// success, before it is appended: each terminal event spends the
    /// covering its dispatch armed, so none outlives its effect. The head
    /// is the one the effect's stable-head check accepted, never a fresh
    /// read; an effect with no such observation records nothing.
    pub(super) fn stamp_covered(&mut self, event_type: EventType, mut payload: Value) -> Value {
        let covering = match event_type {
            EventType::EffectSucceeded
            | EventType::EffectFailed
            | EventType::EffectIndeterminate => self.covering.take(),
            _ => None,
        };
        let (Some(covering), EventType::EffectSucceeded) = (covering, event_type) else {
            return payload;
        };
        let effect_id = payload.get("effect_id").and_then(Value::as_str);
        let covered = effect_id.and_then(|effect_id| self.covered_of(effect_id, covering));
        let result = payload.get_mut("result").and_then(Value::as_object_mut);
        if let (Some(covered), Some(result)) = (covered, result) {
            result.insert("covered".into(), covered);
        }
        payload
    }

    /// Why the protected phase's result is refused at decide: its covered
    /// record does not read, or its digest does not pin the head the
    /// effect's stable-head check accepted, which this decision records as
    /// `reviewed_heads` (ruling 7), or there is no such observation, as
    /// after a restart between the effect and its decision. A result with
    /// no covered record is not judged here and is not trusted as covered.
    pub(super) fn covered_problem(
        &self,
        state: &RunState,
        phase: &str,
        raw_result: &Value,
    ) -> Option<String> {
        if phase != self.bundle.protected_phase {
            return None;
        }
        let covered = raw_result.get("covered")?;
        match (Covered::deserialize(covered).ok(), self.decided_head(state)) {
            (Some(covered), Some(head)) if covered.projects(head) => None,
            _ => Some(
                "the result's covered head does not pin the head decided on \
                 (decision 0109 ruling 7)"
                    .into(),
            ),
        }
    }
}

/// `report` with its successful result admitted at `site`, the site whose
/// driver produced it: its findings stamped in place, or the refusal put
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
    let site = site.map(|site| site.site_ref.as_str());
    if let Err(refusal) = brokkr_core::findings::admit(result, site) {
        report.refused = Some(AttemptOutcome::Failed {
            error: format!("the result was refused: {refusal}"),
        });
    }
    report
}

#[cfg(test)]
pub(super) mod covered_tests;
#[cfg(test)]
mod span_tests;
#[cfg(test)]
mod tests;
