//! The engine's reads of the repository's HEAD behind one seam, and the
//! observation a checked span's stable-head check accepted (decision
//! 0109 ruling 7, fix rounds 0109a1r2 and r3).
//!
//! A checked span compares HEAD at its start and end, then compares
//! tracked tree content if HEAD held. The accepted head is kept as the
//! span's [`Observed`] head, and the `covered` stamp reads it. A
//! protected effect journals its observation as its result's
//! `checked_head`, durable before the success is, and decide's covered
//! check and its `reviewed_heads` read that record from the fold, never
//! process memory, so a resumed engine decides on the head the check
//! compared, "equal by construction", with no HEAD read of its own. A
//! head that moves after the observation is the drift ship refuses.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::gate_moved_head::{self, MovedHead};
use brokkr_core::EventEnvelope;
use serde_json::json;

use super::{git_head, Engine, EngineError};

mod tree;

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
        HeadReads(Box::new(scripted_reads(
            plan.iter().map(|head| Some(head.to_string())).collect(),
        )))
    }
}

type ReadTree = dyn Fn(&Path) -> Option<tree::Snapshot> + Send + Sync;

/// Where the engine reads the tracked tree for an armed span (0071 ruling 2).
pub(super) struct TreeReads(Box<ReadTree>);

impl Default for TreeReads {
    fn default() -> Self {
        Self(Box::new(tree::read))
    }
}

impl TreeReads {
    fn read(&self, repo: &Path) -> Option<tree::Snapshot> {
        (self.0)(repo)
    }

    #[cfg(test)]
    pub(super) fn scripted(plan: Vec<Option<tree::Snapshot>>) -> Self {
        Self(Box::new(scripted_reads(plan)))
    }
}

#[cfg(test)]
fn scripted_reads<T: Clone + Send + Sync + 'static>(
    plan: Vec<Option<T>>,
) -> impl Fn(&Path) -> Option<T> + Send + Sync {
    let next = std::sync::Mutex::new(0usize);
    move |_| {
        let mut next = next.lock().unwrap();
        let reading = plan.get(*next).or(plan.last()).cloned().flatten();
        *next += 1;
        reading
    }
}

/// The end head the stable-head check of `effect_id`'s span compared
/// with its start and accepted; `None` where HEAD did not read at either
/// end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Observed {
    pub(super) effect_id: String,
    pub(super) head: Option<String>,
}

/// The engine's head spans (fix round 0109a1r3, H1). An effect's span and
/// a sequence gate step's span are armed apart, and two observations are
/// kept apart: the latest span's own accepted head, which stamps that
/// span's `covered`, and the effect's observation, which a protected
/// effect journals as its `checked_head`. A gate step sets both; a later
/// work step spends only the first, so a sequence's observation stays
/// its latest gate step's accepted head.
#[derive(Debug, Default)]
pub(super) struct Spans {
    /// Where the effect span stands.
    effect: SpanStart,
    /// Where the sequence gate step's span stands.
    step: SpanStart,
    /// The latest span's accepted end head.
    pub(super) observed: Option<Observed>,
    /// The effect's one observation source.
    pub(super) checked: Option<Observed>,
    /// Whether the effect is the protected phase's, whose results
    /// journal the effect's observation.
    pub(super) protected: bool,
}

/// Which span a head check closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Span {
    Effect,
    Step,
}

/// An armed span retains both start readings, including unreadable ones.
#[derive(Debug, Default)]
enum SpanStart {
    #[default]
    Unarmed,
    Armed {
        head: Option<String>,
        tree: Option<tree::Snapshot>,
    },
}

impl Spans {
    /// Abandon every armed span: an attempt that left through an `Err`
    /// appends no terminal event, so no start outlives it.
    pub(super) fn abandon(&mut self) {
        self.effect = SpanStart::Unarmed;
        self.step = SpanStart::Unarmed;
    }
}

/// `observed`'s head, when it is `effect_id`'s and HEAD read.
fn head_of<'a>(observed: Option<&'a Observed>, effect_id: &str) -> Option<&'a str> {
    observed
        .filter(|observed| observed.effect_id == effect_id)
        .and_then(|observed| observed.head.as_deref())
}

impl Engine {
    fn read_gate_start(&self) -> SpanStart {
        SpanStart::Armed {
            head: self.heads.read(&self.repo),
            tree: self.trees.read(&self.repo),
        }
    }

    /// Begin an effect: every earlier observation is spent, the effect's
    /// span reads its start head and tree when `armed`, and `protected` says
    /// whether its results journal the effect's observation.
    pub(super) fn begin_effect_span(&mut self, armed: bool, protected: bool) {
        self.spans = Spans {
            effect: match armed {
                true => self.read_gate_start(),
                false => SpanStart::Unarmed,
            },
            protected,
            ..Spans::default()
        };
    }

    /// Begin a sequence step: the previous span's own observation is
    /// spent, the effect's is not, and a `gate` step reads its start head and tree.
    pub(super) fn begin_step_span(&mut self, gate: bool) {
        self.spans.observed = None;
        if gate {
            self.spans.step = self.read_gate_start();
        }
    }

    /// The accepted head of `effect_id`'s latest span, if that span was
    /// checked and HEAD read: what stamps that span's own `covered`.
    pub(super) fn observed_head_of(&self, effect_id: &str) -> Option<&str> {
        head_of(self.spans.observed.as_ref(), effect_id)
    }

    /// The observation a protected effect journals as its `checked_head`:
    /// none outside the protected phase, and none where no span of the
    /// effect accepted a head that reads.
    pub(super) fn checked_head_of(&self, effect_id: &str) -> Option<&str> {
        let checked = head_of(self.spans.checked.as_ref(), effect_id);
        checked.filter(|_| self.spans.protected)
    }

    /// Close an armed span, checking HEAD first. Only a span that passes
    /// both checks supplies the head observation; either defect settles
    /// the effect as indeterminate with the corresponding evidence.
    pub(super) fn finish_gate_head_check(
        &mut self,
        span: Span,
        effect_id: &str,
        attempt_id: Option<String>,
    ) -> Result<Option<EventEnvelope>, EngineError> {
        let armed = match span {
            Span::Effect => std::mem::take(&mut self.spans.effect),
            Span::Step => std::mem::take(&mut self.spans.step),
        };
        let (start, tree) = match armed {
            SpanStart::Unarmed => return Ok(None),
            SpanStart::Armed { head, tree } => (head, tree),
        };
        let end = self.heads.read(&self.repo);
        let reason = if start != end {
            Some(gate_moved_head::encode(&MovedHead {
                head_at_start: start,
                head_at_end: end.clone(),
            }))
        } else {
            tree::defect(tree, self.trees.read(&self.repo))
        };
        let Some(reason) = reason else {
            let observed = Observed {
                effect_id: effect_id.to_string(),
                head: end,
            };
            self.spans.checked = Some(observed.clone());
            self.spans.observed = Some(observed);
            return Ok(None);
        };
        // EffectIndeterminate has only a reason string in the frozen event
        // contract. Keep the evidence packed there and attach its structured
        // copy to run/parked rather than widening that contract in this slice.
        let mut payload = json!({
            "effect_id": effect_id,
            "attempt_id": attempt_id,
            "reason": reason,
        });
        self.stderr
            .name(EventType::EffectIndeterminate, &mut payload);
        self.append_raw(EventType::EffectIndeterminate, payload, attempt_id)
            .map(Some)
    }
}
