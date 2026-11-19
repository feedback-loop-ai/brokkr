//! A run's residual findings as every surface shows them, read once, and
//! what each gate verdict the journal records holds beside its word, a
//! sequence step's included, each finding under the site that reported it
//! (decision 0109).

use std::collections::{BTreeMap, BTreeSet};

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
    /// The producer's label where this is a settled panel member's payload
    /// that carried findings or a covered head without the engine's mark
    /// for its attempt and member, as a released engine journaled it:
    /// unverified legacy evidence, shown as that and never as a verdict, so
    /// `findings` and `covered` stay empty (fix round 0109a1r5, V2).
    pub unverified: Option<String>,
}

/// Every successful result in `events` that recorded findings or a
/// covered head, in journal order. A result an older engine wrote, with
/// neither, records nothing and is not shown; a covered record that does
/// not read is not shown either, so no head is invented (decision 0109
/// ruling 6), and findings that do not read are said not to read, never
/// shown as none. Read with `state`, the run's fold, so a repaired run's
/// late tail records no verdict, through the same `RunState::applies`
/// range the frame and [`residual_findings_at`] read (decision 0108
/// ruling 4, fix round 0109a1r5, V5). A caller with no fold passes `None`.
pub fn gate_records(events: &[EventEnvelope], state: Option<&RunState>) -> Vec<GateRecord> {
    let kept = |event: &&EventEnvelope| state.is_none_or(|state| state.applies(event.seq));
    let events: Vec<&EventEnvelope> = events.iter().filter(kept).collect();
    let events = events.as_slice();
    let effects: BTreeMap<&str, (&str, &str)> = (events.iter().copied())
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
    let (ended, settled) = (ending_steps(events), settled_marks(events));
    (events.iter().copied())
        .filter(|event| !read_at_its_success(event, &ended))
        .flat_map(|event| verdicts_of(event, &settled))
        .filter_map(|verdict| reading.record(&verdict))
        .collect()
}

/// One verdict the journal records: its effect, its result, the seat
/// within the effect that produced it, a sequence step's, or none for
/// the effect's own body, and whether the engine's provenance marks it.
struct Verdict<'a> {
    effect_id: &'a str,
    result: &'a Value,
    seat: Option<&'a str>,
    marked: bool,
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
        if !verdict.marked {
            let carries = ["findings", "covered"]
                .iter()
                .any(|key| result.get(key).is_some());
            return carries.then(|| GateRecord {
                phase: phase.to_string(),
                key: participant::key_of(effect_id, verdict.seat),
                findings: Vec::new(),
                covered: None,
                unread: false,
                unverified: Some(participant::label_of(seat, verdict.seat)),
            });
        }
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
            unverified: None,
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
/// indeterminate attempt's panel evidence, under that member (K7). A
/// checkpoint that names the marker's word without the engine's
/// `authored` is a driver's, and is no verdict. The failure event says
/// who appended the evidence, not who supplied what is inside it: a
/// released engine copied a member's raw result there. So a settled
/// member's payload is marked only where `settled` holds the engine's
/// authored `panel-member-finished` marker of that attempt naming that
/// member as succeeded (fix round 0109a1r5, V2).
fn verdicts_of<'a>(event: &'a EventEnvelope, settled: &BTreeSet<StepOf<'_>>) -> Vec<Verdict<'a>> {
    let effect_id = text(event, "effect_id");
    let verdict = |result, seat, marked| Verdict {
        effect_id,
        result,
        seat,
        marked,
    };
    let payload = &event.payload;
    match event.event_type {
        EventType::EffectSucceeded => {
            vec![verdict(&payload["result"], payload["step"].as_str(), true)]
        }
        EventType::EffectCheckpointed => {
            let checkpoint = &payload["checkpoint"];
            let step = EngineMarker::authored(checkpoint) == Some(EngineMarker::StepFinished);
            let seat = checkpoint["step_name"].as_str();
            (step
                .then(|| verdict(&checkpoint["result"], seat, true))
                .into_iter())
            .collect()
        }
        EventType::EffectFailed | EventType::EffectIndeterminate => (payload
            .get("settled_members")
            .and_then(Value::as_object)
            .into_iter()
            .flatten())
        .map(|(member, result)| {
            let marked = settled.contains(&step_of(event, member));
            verdict(result, Some(member.as_str()), marked)
        })
        .collect(),
        _ => Vec::new(),
    }
}

/// Each `(effect, attempt, member)` the engine's own authored
/// `panel-member-finished` marker names as succeeded: the engine's mark
/// on that member's settled payload. A released engine's marker carries
/// no `authored`, which v6's closed checkpoint refuses and the live sink
/// refuses from a driver, so none of its rows is marked.
fn settled_marks<'a>(events: &[&'a EventEnvelope]) -> BTreeSet<StepOf<'a>> {
    member_markers(events)
        .filter(|(_, marker)| marker["outcome"] == "succeeded")
        .filter_map(|(event, marker)| Some(step_of(event, marker["member"].as_str()?)))
        .collect()
}

/// Each `panel-member-finished` checkpoint the engine authored in
/// `events`, beside its event; never a driver's record.
fn member_markers<'a, 'e>(
    events: &'e [&'a EventEnvelope],
) -> impl Iterator<Item = (&'a EventEnvelope, &'a Value)> + 'e {
    (events.iter().copied())
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| (event, &event.payload["checkpoint"]))
        .filter(|(_, marker)| EngineMarker::authored(marker) == Some(EngineMarker::MemberFinished))
}

/// Each `(effect, attempt, step)` whose success names the step that
/// produced it. A gate step that ends its sequence journals its own step
/// record before that success (fix round 0109a1r5, V1), carrying the same
/// verdict, so the success is read and the record is not read twice.
fn ending_steps<'a>(events: &[&'a EventEnvelope]) -> BTreeSet<StepOf<'a>> {
    (events.iter().copied())
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .filter_map(|event| Some(step_of(event, event.payload["step"].as_str()?)))
        .collect()
}

/// A step or member of one attempt of one effect.
type StepOf<'a> = (&'a str, &'a str, &'a str);

/// `step` of `event`'s effect and attempt.
fn step_of<'a>(event: &'a EventEnvelope, step: &'a str) -> StepOf<'a> {
    (text(event, "effect_id"), text(event, "attempt_id"), step)
}

/// Whether `event` is a step record whose step's success `ended` holds.
fn read_at_its_success(event: &EventEnvelope, ended: &BTreeSet<StepOf<'_>>) -> bool {
    let step = event.payload["checkpoint"]["step_name"].as_str();
    event.event_type == EventType::EffectCheckpointed
        && step.is_some_and(|step| ended.contains(&step_of(event, step)))
}

/// The panel member each `(effect, site)` belongs to, as the engine's own
/// `panel-member-finished` marker names it: the member's site beside its
/// name, whether or not the member journaled a checkpoint, and never a
/// driver's record.
fn site_members<'a>(events: &[&'a EventEnvelope]) -> BTreeMap<(&'a str, String), &'a str> {
    member_markers(events)
        .filter_map(|(event, marker)| {
            let site = marker.get("site_ref")?.as_str()?;
            let member = marker.get("member")?.as_str()?;
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
