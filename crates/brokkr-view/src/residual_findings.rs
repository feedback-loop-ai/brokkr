//! A run's residual findings as every surface shows them, read once, and
//! what each gate verdict the journal records holds beside its word, a
//! sequence step's included, each finding under the site that reported it
//! (decision 0109).

use std::collections::BTreeMap;

use brokkr_core::findings::{Covered, CoveredHead, Finding};
use brokkr_core::fold::RunState;
use brokkr_core::residual::{closings_of, residuals_of, Mark};
use brokkr_core::summary::Stated;
use brokkr_core::{EventEnvelope, EventType};
use serde::Deserialize;
use serde_json::Value;

use crate::participant;

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
    /// The participant key of the seat that reported it: the panel
    /// member whose checkpoints carry its site, else the effect's own.
    pub participant: String,
}

/// What one successful result recorded beside its word (decision 0109
/// slice 0109a1): the phase it answered, the effect's own participant
/// key, its findings in the order it reported them, and its covered head
/// as one line of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateRecord {
    pub phase: String,
    pub key: String,
    pub findings: Vec<GateFinding>,
    pub covered: Option<String>,
    /// The result carries findings that do not read as finding records,
    /// which a surface says rather than printing them as absent (fix
    /// round 0109a1r3, M1).
    pub unread: bool,
}

/// Every successful result in `events` that recorded findings or a
/// covered head, in journal order. A result an older engine wrote, with
/// neither, records nothing and is not shown; a covered record that does
/// not read is not shown either, so no head is invented (decision 0109
/// ruling 6), and findings that do not read are said not to read, never
/// shown as none.
pub fn gate_records(events: &[EventEnvelope]) -> Vec<GateRecord> {
    let phases: BTreeMap<&str, &str> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .map(|event| (text(event, "effect_id"), text(event, "phase")))
        .collect();
    let members = site_members(events);
    events
        .iter()
        .filter_map(|event| {
            let (result, step) = verdict_of(event)?;
            let effect_id = text(event, "effect_id");
            let read =
                (result.get("findings")).map(|findings| Vec::<Finding>::deserialize(findings));
            let unread = matches!(read, Some(Err(_)));
            let findings = read.and_then(Result::ok).unwrap_or_default();
            let covered = result
                .get("covered")
                .and_then(|covered| Covered::deserialize(covered).ok());
            // A finding is its member's where a member's record carries
            // its site, else the step's that journaled it, else the
            // effect's own.
            let reporter = |finding: &Finding| {
                let site = finding.key().site().to_string();
                let member = members.get(&(effect_id, site)).copied();
                participant::key_of(effect_id, member.or(step))
            };
            (!findings.is_empty() || covered.is_some() || unread).then(|| GateRecord {
                phase: phases.get(effect_id).copied().unwrap_or("-").to_string(),
                key: participant::key_of(effect_id, step),
                findings: findings
                    .iter()
                    .map(|finding| gate_finding(finding, reporter(finding)))
                    .collect(),
                covered: covered.as_ref().map(covered_text),
                unread,
            })
        })
        .collect()
}

/// The verdict `event` journals, if it journals one (fix round 0109a1r3,
/// M3): a success's result, the effect's own, or a sequence step's result
/// in its step's checkpoint, under the step's name.
fn verdict_of(event: &EventEnvelope) -> Option<(&Value, Option<&str>)> {
    match event.event_type {
        EventType::EffectSucceeded => Some((&event.payload["result"], None)),
        EventType::EffectCheckpointed => {
            let checkpoint = &event.payload["checkpoint"];
            let step = checkpoint.get("step").and_then(Value::as_str);
            (step == Some(STEP_FINISHED))
                .then(|| (&checkpoint["result"], checkpoint["step_name"].as_str()))
        }
        _ => None,
    }
}

/// The engine's marker of a sequence step that finished before the last.
const STEP_FINISHED: &str = "sequence-step-finished";

/// The panel member each `(effect, site)` belongs to, as the engine's
/// structural stamp on a member's records names it: its checkpoints, and
/// the engine's own `panel-member-finished` marker, which carries the
/// member's site whether or not the member journaled a checkpoint.
fn site_members(events: &[EventEnvelope]) -> BTreeMap<(&str, String), &str> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter_map(|event| {
            let checkpoint = &event.payload["checkpoint"];
            let site = checkpoint.get("site_ref")?.as_str()?;
            let member = checkpoint.get("member")?.as_str()?;
            Some(((text(event, "effect_id"), site.to_string()), member))
        })
        .collect()
}

/// The string `key` of `event`'s payload, `-` where it holds none.
fn text<'a>(event: &'a EventEnvelope, key: &str) -> &'a str {
    event.payload[key].as_str().unwrap_or("-")
}

fn gate_finding(finding: &Finding, participant: String) -> GateFinding {
    let key = finding.key();
    GateFinding {
        site: key.site().to_string(),
        id: key.id().to_string(),
        file: finding.file().to_string(),
        line: finding.line(),
        summary: finding.summary().to_string(),
        severity: finding.severity().name(),
        security: finding.security(),
        participant,
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
