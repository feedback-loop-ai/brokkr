//! A run's residual findings as every surface shows them, read once, and
//! what each gate verdict the journal records holds beside its word, a
//! sequence step's included, each finding under the site that reported it
//! (decision 0109).

use std::collections::BTreeMap;

use brokkr_core::findings::{Covered, CoveredHead, EngineMarker, Finding};
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
    /// member the engine's marker gives its site, else the sequence step
    /// that produced its verdict, else the effect's own.
    pub participant: String,
    /// That seat's label, as a seat scope names it: the effect's seat,
    /// and `:` and its member or step (fix round 0109a1r4, K6).
    pub producer: String,
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
    let effects: BTreeMap<&str, (&str, &str)> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .map(|event| {
            (
                text(event, "effect_id"),
                (text(event, "phase"), text(event, "seat")),
            )
        })
        .collect();
    let reading = Reading {
        effects,
        members: site_members(events),
    };
    (events.iter())
        .flat_map(verdicts_of)
        .filter_map(|verdict| reading.record(&verdict))
        .collect()
}

/// One verdict the journal records: its effect, its result, and the seat
/// within the effect that produced it, a sequence step's, or none for
/// the effect's own body.
struct Verdict<'a> {
    effect_id: &'a str,
    result: &'a Value,
    seat: Option<&'a str>,
}

/// What a verdict is read with: each effect's phase and seat, and the
/// panel member each `(effect, site)` belongs to.
struct Reading<'a> {
    effects: BTreeMap<&'a str, (&'a str, &'a str)>,
    members: BTreeMap<(&'a str, String), &'a str>,
}

impl Reading<'_> {
    /// `verdict`'s record, if it recorded findings or a covered head. A
    /// finding is filed under the member whose site it carries, else the
    /// seat that produced the verdict (fix round 0109a1r4, K6), and names
    /// that producer.
    fn record(&self, verdict: &Verdict<'_>) -> Option<GateRecord> {
        let Verdict {
            effect_id, result, ..
        } = *verdict;
        let read = (result.get("findings")).map(Vec::<Finding>::deserialize);
        let unread = matches!(read, Some(Err(_)));
        let findings = read.and_then(Result::ok).unwrap_or_default();
        let covered = result
            .get("covered")
            .and_then(|covered| Covered::deserialize(covered).ok());
        let (phase, seat) = self.effects.get(effect_id).copied().unwrap_or(("-", "-"));
        let producer = |finding: &Finding| {
            let site = finding.key().site().to_string();
            let member = self.members.get(&(effect_id, site)).copied();
            let member = member.or(verdict.seat);
            Producer {
                key: participant::key_of(effect_id, member),
                label: participant::label_of(seat, member),
            }
        };
        (!findings.is_empty() || covered.is_some() || unread).then(|| GateRecord {
            phase: phase.to_string(),
            key: participant::key_of(effect_id, verdict.seat),
            findings: (findings.iter())
                .map(|finding| gate_finding(finding, producer(finding)))
                .collect(),
            covered: covered.as_ref().map(covered_text),
            unread,
        })
    }
}

/// The seat that produced a finding: its participant key and its label.
struct Producer {
    key: String,
    label: String,
}

/// The verdicts `event` journals, admitted only by provenance the engine
/// established (fix round 0109a1r4, K1): a success's result, which only
/// the engine appends, under the sequence step it names (K6); a sequence
/// step's result in the step's checkpoint the engine authored, under the
/// step's name; and each settled member's result in a failed or
/// indeterminate attempt's panel evidence, which only the engine writes,
/// under that member (K7). A checkpoint that names the marker's word
/// without the engine's `authored` is a driver's, and is no verdict.
fn verdicts_of(event: &EventEnvelope) -> Vec<Verdict<'_>> {
    let effect_id = text(event, "effect_id");
    let verdict = |result, seat| Verdict {
        effect_id,
        result,
        seat,
    };
    let payload = &event.payload;
    match event.event_type {
        EventType::EffectSucceeded => vec![verdict(&payload["result"], payload["step"].as_str())],
        EventType::EffectCheckpointed => {
            let checkpoint = &payload["checkpoint"];
            let step = EngineMarker::authored(checkpoint) == Some(EngineMarker::StepFinished);
            let seat = checkpoint["step_name"].as_str();
            (step
                .then(|| verdict(&checkpoint["result"], seat))
                .into_iter())
            .collect()
        }
        EventType::EffectFailed | EventType::EffectIndeterminate => (payload
            .get("settled_members")
            .and_then(Value::as_object)
            .into_iter()
            .flatten())
        .map(|(member, result)| verdict(result, Some(member.as_str())))
        .collect(),
        _ => Vec::new(),
    }
}

/// The panel member each `(effect, site)` belongs to, as the engine's own
/// `panel-member-finished` marker names it: the member's site beside its
/// name, whether or not the member journaled a checkpoint, and never a
/// driver's record.
fn site_members(events: &[EventEnvelope]) -> BTreeMap<(&str, String), &str> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter_map(|event| {
            let checkpoint = &event.payload["checkpoint"];
            let marker = EngineMarker::authored(checkpoint);
            (marker == Some(EngineMarker::MemberFinished)).then_some(())?;
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

fn gate_finding(finding: &Finding, producer: Producer) -> GateFinding {
    let key = finding.key();
    GateFinding {
        site: key.site().to_string(),
        id: key.id().to_string(),
        file: finding.file().to_string(),
        line: finding.line(),
        summary: finding.summary().to_string(),
        severity: finding.severity().name(),
        security: finding.security(),
        participant: producer.key,
        producer: producer.label,
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
