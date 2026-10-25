//! A participant's checkpoint rows, made one way whichever way the run was
//! read (#753; decision 0089 ruling 7, slice 2b): by the participant scan
//! of the run read whole, or from the seat view's page, where each
//! checkpoint is made into its row as the store reads it and dropped, by
//! the boundary entries of the start each attempt on the page last names,
//! read once a page, which a row's boundary reads.

use std::collections::BTreeMap;

use brokkr_core::seat::{boundary_entries, stamp_of, token_usage, BoundaryEntry};
use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use crate::{
    capability_calls, cell_of, field, joined, js, target_cells, usage_text, CheckpointRow,
    ModelAtBoundary, NO_BOUNDARY,
};

/// Each attempt's boundary entries, by its effect and its attempt, as the
/// last start naming both records them.
pub(crate) type Boundaries = BTreeMap<(String, String), Vec<BoundaryEntry>>;

/// The boundary entries every start among `events` records.
pub(crate) fn boundaries_of(events: &[EventEnvelope]) -> Boundaries {
    let started = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectStarted);
    let entries = started.filter_map(|event| {
        let effect_id = field(&event.payload, "effect_id")?;
        let attempt_id = field(&event.payload, "attempt_id")?;
        let key = (effect_id.to_string(), attempt_id.to_string());
        Some((key, boundary_entries(&event.payload)))
    });
    entries.collect()
}

/// The boundary the row of a record carrying `payload` shows, where it
/// names its effect and its attempt: its member's entry of that attempt's
/// last start, else the stamp the record carried beside a model, else, for
/// the seat's own record, its members' entries joined. `None` where it
/// names either not, whose boundary is its participant's own.
pub(crate) fn attempt_boundary(boundaries: &Boundaries, payload: &Value) -> Option<Option<String>> {
    let effect_id = field(payload, "effect_id")?;
    let attempt = field(payload, "attempt_id")?;
    let record = payload.get("checkpoint").or_else(|| payload.get("result"));
    let member = record.and_then(|record| field(record, "member"));
    let entries = boundaries.get(&(effect_id.to_string(), attempt.to_string()));
    let entry = entries.and_then(|entries| {
        entries
            .iter()
            .find(|entry| entry.member.as_deref() == member)
    });
    let boundary = entry
        .map(|entry| entry.word.clone())
        .or_else(|| record.and_then(stamp_of))
        .or_else(|| {
            if member.is_some() {
                return None;
            }
            joined(entries?.iter().map(|entry| entry.word.as_str()).collect())
        });
    Some(boundary)
}

/// The row of `checkpoint`, recorded at `recorded_at` within `boundary`.
pub(crate) fn row_of(
    checkpoint: &Value,
    recorded_at: &str,
    boundary: Option<String>,
) -> CheckpointRow {
    let target = match checkpoint.get("target") {
        Some(Value::String(target)) if !target.is_empty() => Some(target.as_str()),
        _ => None,
    };
    let (short, full) = target_cells(target);
    let step = match checkpoint.get("step").and_then(Value::as_str) {
        Some("seat-turn") => js::to_display(checkpoint.get("tool")),
        _ => js::to_display(checkpoint.get("step")),
    };
    let text = |key: &str| {
        checkpoint
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    CheckpointRow {
        turn: cell_of(
            checkpoint
                .get("turn")
                .and_then(Value::as_u64)
                .map(|turn| turn.to_string()),
            Some("not a numbered turn"),
        ),
        step,
        served: ModelAtBoundary {
            model: cell_of(
                text("model"),
                Some("no served model recorded for this checkpoint"),
            ),
            boundary: cell_of(boundary, Some(NO_BOUNDARY)),
        },
        effort: cell_of(
            text("effort"),
            Some("no applied effort recorded for this checkpoint"),
        ),
        usage: cell_of(
            token_usage(std::iter::once(checkpoint))
                .as_ref()
                .map(|usage| usage_text(usage, "")),
            Some("no token usage recorded for this checkpoint"),
        ),
        target: cell_of(
            short,
            Some("no target recorded — the journal carries file targets only"),
        ),
        target_full: full,
        capability_call: capability_calls::call_evidence(checkpoint),
        recorded_at: recorded_at.to_string(),
    }
}

/// What makes the rows the seat view shows of a page's checkpoints, its
/// read having handed over `starts`, the start each attempt on the page
/// last names: their boundary entries are read once, and each checkpoint
/// is made into the row the participant scan makes of it. `None` where a
/// checkpoint names no effect or no attempt, which no page holds, so the
/// reader reads the run whole.
pub fn page_rows(starts: &[EventEnvelope]) -> impl Fn(&EventEnvelope) -> Option<CheckpointRow> {
    let boundaries = boundaries_of(starts);
    move |event| {
        let boundary = attempt_boundary(&boundaries, &event.payload)?;
        let checkpoint = event.payload.get("checkpoint").unwrap_or(&Value::Null);
        Some(row_of(checkpoint, &event.recorded_at, boundary))
    }
}

#[cfg(test)]
mod tests;
