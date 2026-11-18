//! The engine's reads of the repository's HEAD behind one seam, and the
//! one observation a gate span's stable-head check accepted (decision
//! 0109 ruling 7, fix round 0109a1r2).
//!
//! A gate span reads HEAD as it begins and as it ends, and the check
//! compares the two. The end read the check accepted is kept as the
//! span's [`Observed`] head: the `covered` stamp, decide's covered check
//! and its `reviewed_heads` all read that observation, so the head
//! recorded as reviewed is the head the check compared, "equal by
//! construction". None of them reads HEAD again, and a span that armed
//! no check leaves no observation, so its result carries no `covered`
//! and decide records no reviewed head for it. A head that moves after
//! the span ends is the drift ship already refuses.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{Cursor, Run, RunState};
use brokkr_core::gate_moved_head::{self, MovedHead};
use brokkr_core::EventEnvelope;
use serde_json::json;

use super::{git_head, Engine, EngineError};

/// One read of the HEAD of a repository.
type ReadHead = dyn Fn(&Path) -> Option<String> + Send + Sync;

/// Where the engine reads HEAD: git, or a planned sequence in a test. A
/// closure seam (decision 0071 ruling 2): one substitutable function.
pub(super) struct HeadReads(Box<ReadHead>);

impl Default for HeadReads {
    fn default() -> HeadReads {
        HeadReads(Box::new(git_head))
    }
}

impl HeadReads {
    /// The repository's HEAD at `repo`, as this seam reads it now.
    pub(super) fn read(&self, repo: &Path) -> Option<String> {
        (self.0)(repo)
    }

    /// A seam that answers `plan` in order, one head per read, and then
    /// the last head for every later read: a head that moves at a chosen
    /// read with no sleep and no racing commit.
    #[cfg(test)]
    pub(super) fn scripted(plan: &[&str]) -> HeadReads {
        let plan: Vec<String> = plan.iter().map(|head| head.to_string()).collect();
        let next = std::sync::Mutex::new(0usize);
        HeadReads(Box::new(move |_| {
            let mut next = next.lock().unwrap();
            let head = plan.get(*next).or(plan.last()).cloned();
            *next += 1;
            head
        }))
    }
}

/// The end head the stable-head check of `effect_id`'s latest gate span
/// compared with its start and accepted; `None` where HEAD did not read
/// at either end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Observed {
    pub(super) effect_id: String,
    pub(super) head: Option<String>,
}

impl Engine {
    /// Begin a span, an effect or a sequence step: the previous span's
    /// observation is spent, and a `gate` span reads its start head.
    pub(super) fn begin_span(&mut self, gate: bool) {
        self.observed_head = None;
        if gate {
            self.active_gate_head = Some(self.heads.read(&self.repo));
        }
    }

    /// The observed head of `effect_id`'s latest gate span, if that span
    /// was checked and HEAD read.
    pub(super) fn observed_head_of(&self, effect_id: &str) -> Option<&str> {
        self.observed_head
            .as_ref()
            .filter(|observed| observed.effect_id == effect_id)
            .and_then(|observed| observed.head.as_deref())
    }

    /// The observed head of the effect `state` is deciding.
    pub(super) fn decided_head(&self, state: &RunState) -> Option<&str> {
        match &state.run {
            Run::Running(Cursor::Decide { effect_id, .. }) => self.observed_head_of(effect_id),
            _ => None,
        }
    }

    /// Close the armed gate span of `effect_id`: its end head is read
    /// once and compared with its start. A match is kept as the span's
    /// observation; a move is journaled as the indeterminate end whose
    /// reason carries both heads.
    pub(super) fn finish_gate_head_check(
        &mut self,
        effect_id: &str,
        attempt_id: Option<String>,
    ) -> Result<Option<EventEnvelope>, EngineError> {
        let Some(start) = self.active_gate_head.take() else {
            return Ok(None);
        };
        let end = self.heads.read(&self.repo);
        if start == end {
            self.observed_head = Some(Observed {
                effect_id: effect_id.to_string(),
                head: end,
            });
            return Ok(None);
        }
        let moved = MovedHead {
            head_at_end: end,
            head_at_start: start,
        };
        // EffectIndeterminate has only a reason string in the frozen event
        // contract. Keep the evidence packed there and attach its structured
        // copy to run/parked rather than widening that contract in this slice.
        self.append_raw(
            EventType::EffectIndeterminate,
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "reason": gate_moved_head::encode(&moved),
            }),
            attempt_id,
        )
        .map(Some)
    }
}
