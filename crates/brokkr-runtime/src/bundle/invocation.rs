//! One driver, or a parallel panel joined by a declared aggregate (#349):
//! the two invocation forms a seat, a selected case and a sequence step
//! share, written once with the walks every one of them takes.

use std::path::PathBuf;

use serde_json::{json, Map, Value};

use super::{Aggregate, PanelMember, SeatClass};
use crate::agents::Candidate;

/// One agent session, or a parallel panel joined by a declared
/// deterministic rule (decision 0002's sanctioned forms).
///
/// `C` is where a lone driver's class lives (decision 0021 ruling 1). A
/// seat's or a selected case's single carries its own [`SeatClass`]. A
/// sequence step's single carries `()`, because a step's class is
/// [`SequenceStep::class`](super::SequenceStep::class) and is never stored
/// twice. A panel carries none: each [`PanelMember::class`] does.
#[derive(Debug, Clone)]
pub enum Invocation<C = SeatClass> {
    Single {
        role_path: PathBuf,
        command: Vec<String>,
        /// The bounded fallback chain (decision 0016). EMPTY for an
        /// inline site, which is what keeps the execute path — and
        /// therefore inline behaviour — exactly as it was.
        candidates: Vec<Candidate>,
        /// The command as written, which 0031 reads; EMPTY for an agent.
        written: Vec<String>,
        /// Decision 0021 ruling 1's class of this driver-bearing site.
        class: C,
    },
    Panel {
        members: Vec<PanelMember>,
        aggregate: Aggregate,
    },
}

impl<C> Invocation<C> {
    /// How the manifest records this invocation, wherever it stands.
    pub(super) fn manifest(&self) -> Value {
        match self {
            Invocation::Single { candidates, .. } => driver_manifest(candidates),
            Invocation::Panel { members, aggregate } => json!({
                "panel": members
                    .iter()
                    .map(|member| (member.name.clone(), driver_manifest(&member.candidates)))
                    .collect::<Map<_, _>>(),
                "aggregate": aggregate.word(),
            }),
        }
    }

    /// How many drivers this invocation runs at once.
    pub(crate) fn parallel(&self) -> usize {
        match self {
            Invocation::Single { .. } => 1,
            Invocation::Panel { members, .. } => members.len(),
        }
    }

    /// Every driver-bearing site under the label `what`, with the command
    /// it wrote: a member is labelled `<what>:<member>`.
    pub(super) fn written_sites(&self, what: &str) -> Vec<(String, &[String])> {
        match self {
            Invocation::Single { written, .. } => vec![(what.to_string(), written)],
            Invocation::Panel { members, .. } => members
                .iter()
                .map(|member| (format!("{what}:{}", member.name), &member.written[..]))
                .collect(),
        }
    }

    /// Every invocation site with its fallback chain, tagged within its
    /// body: a single by the enclosing `step`'s name, if any, and a member
    /// by its name under that step.
    pub(crate) fn chains(&self, step: Option<&str>) -> Vec<(Option<String>, &[Candidate])> {
        match self {
            Invocation::Single { candidates, .. } => vec![(step.map(str::to_string), candidates)],
            Invocation::Panel { members, .. } => members
                .iter()
                .map(|member| {
                    let tag = match step {
                        None => member.name.clone(),
                        Some(step) => format!("{step}:{}", member.name),
                    };
                    (Some(tag), member.candidates.as_slice())
                })
                .collect(),
        }
    }
}

impl Invocation {
    /// This invocation as a sequence step's, whose class the step holds.
    pub(super) fn to_step(&self) -> Invocation<()> {
        match self {
            Invocation::Single {
                role_path,
                command,
                candidates,
                written,
                ..
            } => Invocation::Single {
                role_path: role_path.clone(),
                command: command.clone(),
                candidates: candidates.clone(),
                written: written.clone(),
                class: (),
            },
            Invocation::Panel { members, aggregate } => Invocation::Panel {
                members: members.clone(),
                aggregate: *aggregate,
            },
        }
    }
}

/// How the manifest records one driver: inline, or its agent's chain.
fn driver_manifest(candidates: &[Candidate]) -> Value {
    if candidates.is_empty() {
        json!({"driver": "inline"})
    } else {
        json!({"agent": candidates[0].agent, "candidates": candidates.iter().map(|candidate| json!({
            "provider": candidate.provider,
            "model": candidate.model,
            "effort": candidate.effort,
        })).collect::<Vec<_>>()})
    }
}

#[cfg(test)]
mod tests;
