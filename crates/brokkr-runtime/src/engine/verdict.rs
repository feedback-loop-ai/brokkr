//! A seat's result admitted as a verdict's record (decision 0109 slice
//! 0109a1): the findings a seat reports are stamped with the digest of
//! the compiled site that reported them, before the result is joined
//! into a panel's aggregate or journaled, and a result the admission
//! refuses fails its attempt with the refusal named, never repaired
//! (decision 0001). No word is refused here and no routing moves.

use brokkr_core::envelope::EventType;
use brokkr_core::findings::Covered;
use brokkr_protocol::{AttemptOutcome, AttemptReport};
use serde_json::Value;

use super::resume::SiteContext;
use super::{git_head, Engine};

/// Ruling 7's `dirty` is true unless the tree check proved the worktree
/// clean at commission time. That check is slice 0109b's, so every
/// record this slice writes says the worktree may be dirty.
const DIRTY_UNTIL_THE_TREE_CHECK: bool = true;

/// Which `covered` field set a gate effect's successful result records
/// (decision 0109 ruling 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Covering {
    /// A gate with its own head record: the commit it read, taken at the
    /// end of the effect, where the stable-head check already holds.
    Head,
    /// The protected phase: the SHA-256 of its head, pinned at decide to
    /// the decide-time `reviewed_heads`.
    HeadDigest,
}

impl Covering {
    /// What an effect of a seat whose selected body is `gate` class
    /// records, in the bundle's protected phase or not: nothing for work.
    pub(super) fn of(gate: bool, protected: bool) -> Option<Covering> {
        match (gate, protected) {
            (false, _) => None,
            (true, true) => Some(Covering::HeadDigest),
            (true, false) => Some(Covering::Head),
        }
    }
}

impl Engine {
    /// A terminal effect payload with its covered record stamped on a
    /// success, before it is appended: each terminal event spends the
    /// covering its dispatch armed, so none outlives its effect. A head
    /// that does not read records nothing rather than a guess.
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
        let covered = git_head(&self.repo).and_then(|head| match covering {
            Covering::Head => Covered::commit(&head, DIRTY_UNTIL_THE_TREE_CHECK),
            Covering::HeadDigest => Some(Covered::digest_of(&head, DIRTY_UNTIL_THE_TREE_CHECK)),
        });
        let result = payload.get_mut("result").and_then(Value::as_object_mut);
        if let (Some(covered), Some(result)) = (covered, result) {
            let record = serde_json::to_value(covered).expect("a covered record serializes");
            result.insert("covered".into(), record);
        }
        payload
    }

    /// Why the protected phase's result is refused at decide: its covered
    /// record does not read, or its digest does not pin the head this
    /// decision records as `reviewed_heads` (ruling 7). A result with no
    /// covered record, from an older engine, is not judged here.
    pub(super) fn covered_problem(&self, phase: &str, raw_result: &Value) -> Option<String> {
        if phase != self.bundle.protected_phase {
            return None;
        }
        let covered = raw_result.get("covered")?;
        match (Covered::read(covered), git_head(&self.repo)) {
            (Some(covered), Some(head)) if covered.projects(&head) => None,
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
mod covered_tests;
#[cfg(test)]
mod tests;
