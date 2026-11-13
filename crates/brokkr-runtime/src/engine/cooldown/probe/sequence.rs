//! The sequence step a wait's site names, and the facts a step's probe
//! composes from (decision 0103 slice 4c2b): the step and member a site
//! resolves to, the context the failed attempt ran with, the one-member
//! panel composition, and the chain link the refusal named.

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use serde_json::{Map, Value};

use super::super::terminal;
use crate::bundle::{Aggregate, Invocation, PanelMember, SequenceStep, StepBody};
use crate::engine::carry::record::Tag;
use crate::engine::{chain_index, Selection, Site};

/// The sequence step and member a wait's site names. An exact step name is
/// a single step; otherwise a member is resolved by the declared
/// `(<step>, <member>)` pair, so a wait at `a:b:c` is `a:b`'s member `c`
/// even when `a` also declares a member `b:c`. The prefix walk that follows
/// is only for the diagnostic a site naming neither: exact first, so a step
/// whose own name contains a colon is not mistaken for a member of an
/// earlier one.
pub(super) fn step_site<'a>(
    steps: &'a [SequenceStep],
    site: &'a str,
) -> Option<(usize, &'a SequenceStep, Option<&'a str>)> {
    if let Some((index, step)) = steps.iter().enumerate().find(|(_, step)| step.name == site) {
        return Some((index, step, None));
    }
    if let Some((index, step, member)) = declared_member(steps, site) {
        return Some((index, step, Some(member)));
    }
    steps.iter().enumerate().find_map(|(index, step)| {
        site.strip_prefix(&format!("{}:", step.name))
            .map(|member| (index, step, Some(member)))
    })
}

/// The step and member whose declared tag is exactly `site`, in declaration
/// order.
fn declared_member<'a>(
    steps: &'a [SequenceStep],
    site: &str,
) -> Option<(usize, &'a SequenceStep, &'a str)> {
    steps.iter().enumerate().find_map(|(index, step)| {
        let StepBody::Invocation(Invocation::Panel { members, .. }) = &step.body else {
            return None;
        };
        members
            .iter()
            .find(|member| Tag::new(Some(&step.name), &member.name).0 == site)
            .map(|member| (index, step, member.name.as_str()))
    })
}

/// The context a step's attempt had: the seat's context with the earlier
/// steps' checkpointed results of the failed attempt beside it, so the
/// probe's input derives exactly as the attempt's did (decision 0103 slice
/// 4c2b).
pub(in crate::engine::cooldown) fn step_context(
    events: &[EventEnvelope],
    effect_id: &str,
    seat_input: &Value,
) -> Value {
    let mut context = seat_input["context"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let attempt = terminal(events, effect_id)
        .and_then(|event| event.payload.get("attempt_id"))
        .and_then(Value::as_str);
    let mut prior = Map::new();
    for event in events {
        if event.event_type != EventType::EffectCheckpointed
            || event.payload.get("effect_id").and_then(Value::as_str) != Some(effect_id)
            || attempt.is_some_and(|attempt| {
                event.payload.get("attempt_id").and_then(Value::as_str) != Some(attempt)
            })
        {
            continue;
        }
        let Some(checkpoint) = event.payload.get("checkpoint") else {
            continue;
        };
        if checkpoint.get("step").and_then(Value::as_str) != Some("sequence-step-finished") {
            continue;
        }
        let (Some(name), Some(result)) = (
            checkpoint.get("step_name").and_then(Value::as_str),
            checkpoint.get("result"),
        ) else {
            continue;
        };
        prior.insert(name.to_string(), result.clone());
    }
    context.insert("prior_results".into(), Value::Object(prior));
    Value::Object(context)
}

/// The selection and one-member body a panel member's probe admits: its
/// own candidates, the link the refusal named, and the tag the journal
/// names it by. A seat-level member and a sequence step's member take this
/// one composition.
pub(super) fn probe_member_invocation(
    events: &[EventEnvelope],
    effect_id: &str,
    tag: Option<String>,
    member: &PanelMember,
    aggregate: Aggregate,
) -> (Selection, Invocation) {
    let mut selection = Selection::new();
    if !member.candidates.is_empty() {
        let index = refused_index(events, effect_id, &tag, member.candidates.len());
        selection.insert(tag, member.candidates[index].clone());
    }
    let one = Invocation::Panel {
        members: vec![member.clone()],
        aggregate,
    };
    (selection, one)
}

/// The chain link the refused attempt used: the start failures recorded
/// before the effect's last terminal event, clamped as `chain_index` does.
pub(super) fn refused_index(
    events: &[EventEnvelope],
    effect_id: &str,
    site: &Site,
    chain: usize,
) -> usize {
    let before = terminal(events, effect_id)
        .and_then(|last| events.iter().position(|event| event.seq == last.seq))
        .map_or(events, |at| &events[..at]);
    chain_index(before, effect_id, site, chain)
}

/// The label a seat's attempt and its probe are both composed under: the
/// seat, or `<seat>:<case>` where the selector chose a case.
pub(super) fn site_name(seat: &str, case: Option<&str>) -> String {
    case.map(|case| format!("{seat}:{case}"))
        .unwrap_or_else(|| seat.to_string())
}
