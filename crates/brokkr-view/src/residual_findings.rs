//! A run's residual findings as every surface shows them, read once, and
//! what each gate verdict recorded beside its word (decision 0109).

use std::collections::BTreeMap;

use brokkr_core::findings::{Covered, CoveredHead, Finding};
use brokkr_core::fold::RunState;
use brokkr_core::residual::{closings_of, residuals_of, Mark};
use brokkr_core::summary::Stated;
use brokkr_core::{EventEnvelope, EventType};
use serde::Deserialize;

/// One finding a verdict recorded, as it reads: its key, `site` and
/// `id`, and its record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateFinding {
    pub site: String,
    pub id: String,
    pub file: String,
    pub line: Option<u64>,
    pub summary: String,
    pub severity: &'static str,
    pub security: bool,
}

/// What one successful result recorded beside its word (decision 0109
/// slice 0109a1): the phase it answered, its findings in the order it
/// reported them, and its covered head as one line of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRecord {
    pub phase: String,
    pub findings: Vec<GateFinding>,
    pub covered: Option<String>,
}

/// Every successful result in `events` that recorded findings or a
/// covered head, in journal order. A result an older engine wrote, with
/// neither, records nothing and is not shown; a record that does not read
/// is not shown either, so no finding and no head is invented (decision
/// 0109 ruling 6).
pub fn gate_records(events: &[EventEnvelope]) -> Vec<GateRecord> {
    let phases: BTreeMap<&str, &str> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .map(|event| (text(event, "effect_id"), text(event, "phase")))
        .collect();
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .filter_map(|event| {
            let result = &event.payload["result"];
            let findings = result
                .get("findings")
                .and_then(|findings| Vec::<Finding>::deserialize(findings).ok())
                .unwrap_or_default();
            let covered = result
                .get("covered")
                .and_then(|covered| Covered::deserialize(covered).ok());
            (!findings.is_empty() || covered.is_some()).then(|| GateRecord {
                phase: phases
                    .get(text(event, "effect_id"))
                    .copied()
                    .unwrap_or("-")
                    .to_string(),
                findings: findings.iter().map(gate_finding).collect(),
                covered: covered.as_ref().map(covered_text),
            })
        })
        .collect()
}

/// The string `key` of `event`'s payload, `-` where it holds none.
fn text<'a>(event: &'a EventEnvelope, key: &str) -> &'a str {
    event.payload[key].as_str().unwrap_or("-")
}

fn gate_finding(finding: &Finding) -> GateFinding {
    let key = finding.key();
    GateFinding {
        site: key.site().to_string(),
        id: key.id().to_string(),
        file: finding.file().to_string(),
        line: finding.line().map(std::num::NonZeroU64::get),
        summary: finding.summary().to_string(),
        severity: finding.severity().name(),
        security: finding.security(),
    }
}

/// `head <commit>` or `head sha256 <digest>`, and whether the worktree
/// may have been dirty.
fn covered_text(covered: &Covered) -> String {
    let head = match covered.head() {
        CoveredHead::Commit(commit) => format!("head {commit}"),
        CoveredHead::Digest(digest) => format!("head sha256 {digest}"),
    };
    match covered.dirty() {
        true => format!("{head} · dirty"),
        false => format!("{head} · clean"),
    }
}

use crate::{stated_finding, ResidualFinding};

/// Every residual finding a run's journal carries, with its supersede
/// mark, read with `state`, the run's fold, so a repaired run's late tail
/// closes no finding and states none: the same rule the fold and the
/// summary keep (decision 0108 ruling 4). The range test is
/// `RunState::applies`. A caller with no fold passes `None`.
pub fn residual_findings_at(
    run_id: &str,
    events: &[EventEnvelope],
    state: Option<&RunState>,
) -> Vec<ResidualFinding> {
    let kept = |event: &&EventEnvelope| state.is_none_or(|state| state.applies(event.seq));
    let marks: BTreeMap<u64, Mark> = closings_of(events.iter().filter(kept))
        .into_iter()
        .collect();
    residuals_of(events.iter().filter(kept))
        .iter()
        .map(|residual| {
            let superseded = marks.get(&residual.ruling.seq).cloned();
            stated_finding(run_id, Stated::of(residual, superseded))
        })
        .collect()
}

#[cfg(test)]
mod tests;
