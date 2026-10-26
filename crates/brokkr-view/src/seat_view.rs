//! A participant's checkpoint rows, made one way whichever way the run was
//! read (#753; decision 0089 ruling 7, slice 2b): by the participant scan
//! of the run read whole, or from the seat view's page, where each
//! checkpoint is made into its row as the store reads it and dropped, by
//! the words its rows read of the start each attempt on the page last
//! names, read once a page, which a row's boundary reads.

use std::collections::BTreeMap;

use brokkr_core::seat::{boundary_entries, stamp_of, token_usage, BoundaryEntry};
use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use crate::{
    capability_calls, cell_of, field, joined, js, target_cells, usage_text, CheckpointRow,
    ModelAtBoundary, NO_BOUNDARY,
};

/// An attempt, by its effect and its attempt.
type Attempt = (String, String);

/// Each attempt's boundary entries, as the last start naming it records
/// them.
pub(crate) type Boundaries = BTreeMap<Attempt, Vec<BoundaryEntry>>;

/// The boundary entries every start among `events` records.
pub(crate) fn boundaries_of(events: &[EventEnvelope]) -> Boundaries {
    events.iter().filter_map(boundary_of).collect()
}

/// The boundary entries `event` records, by its effect and its attempt,
/// where it is a start naming both.
fn boundary_of(event: &EventEnvelope) -> Option<(Attempt, Vec<BoundaryEntry>)> {
    if event.event_type != EventType::EffectStarted {
        return None;
    }
    let effect_id = field(&event.payload, "effect_id")?;
    let attempt_id = field(&event.payload, "attempt_id")?;
    let key = (effect_id.to_string(), attempt_id.to_string());
    Some((key, boundary_entries(&event.payload)))
}

/// What a row of one participant reads of its attempt's last start: its
/// member's entry's word, and, for the seat's own, every entry's word
/// joined; each within the six words, whatever the start holds.
#[derive(Debug, Default)]
struct Worded {
    word: Option<String>,
    joined: Option<String>,
}

/// What a row reads of an attempt no start names.
const UNSTARTED: Worded = Worded {
    word: None,
    joined: None,
};

impl Worded {
    /// What a row of `member`, `None` for the seat's own, reads of
    /// `entries`.
    fn of(entries: &[BoundaryEntry], member: Option<&str>) -> Worded {
        let entry = entries
            .iter()
            .find(|entry| entry.member.as_deref() == member);
        let words = || joined(entries.iter().map(|entry| entry.word.as_str()).collect());
        Worded {
            word: entry.map(|entry| entry.word.clone()),
            joined: member.is_none().then(words).flatten(),
        }
    }

    /// The boundary a row of `record` shows: its member's entry, else the
    /// stamp the record carried beside a model, else, for the seat's own,
    /// its members' entries joined.
    fn boundary(&self, record: Option<&Value>) -> Option<String> {
        let stamp = || record.and_then(stamp_of);
        let joined = || self.joined.clone();
        self.word.clone().or_else(stamp).or_else(joined)
    }
}

/// The attempt a record carrying `payload` names, by its effect and its
/// attempt, with the record and the member it names: `None` where it
/// names either not.
fn named(payload: &Value) -> Option<(Attempt, Option<&Value>, Option<&str>)> {
    let effect_id = field(payload, "effect_id")?;
    let attempt = field(payload, "attempt_id")?;
    let record = payload.get("checkpoint").or_else(|| payload.get("result"));
    let member = record.and_then(|record| field(record, "member"));
    let key = (effect_id.to_string(), attempt.to_string());
    Some((key, record, member))
}

/// The boundary the row of a record carrying `payload` shows, where it
/// names its effect and its attempt: its member's entry of that attempt's
/// last start, else the stamp the record carried beside a model, else, for
/// the seat's own record, its members' entries joined. `None` where it
/// names either not, whose boundary is its participant's own.
pub(crate) fn attempt_boundary(boundaries: &Boundaries, payload: &Value) -> Option<Option<String>> {
    let (key, record, member) = named(payload)?;
    let entries = boundaries.get(&key).map_or(&[][..], Vec::as_slice);
    Some(Worded::of(entries, member).boundary(record))
}

/// Let go of each member label among `payload`'s boundary entries that no
/// row of `member` reads: where `member` names one, every other member's
/// entry, as its rows read their own entry alone; for the seat's own, each
/// label's text, its entry kept with an empty label, as its rows read an
/// entry naming no member, else every entry's word joined.
fn unlabelled(payload: &mut Value, member: Option<&str>) {
    let Some(entries) = payload.get_mut("boundary").and_then(Value::as_array_mut) else {
        return;
    };
    match member {
        Some(member) => entries.retain(|entry| field(entry, "member") == Some(member)),
        None => entries
            .iter_mut()
            .filter_map(|entry| entry.get_mut("member"))
            .for_each(|label| {
                if let Value::String(label) = label {
                    *label = String::new();
                }
            }),
    }
}

/// What a row of `member` reads of `start`, by its effect and its
/// attempt, where it is a start naming both: the labels no such row reads
/// let go of first ([`unlabelled`]), so what a page keeps of a start is
/// the six words alone, whatever its entries hold.
fn worded(mut start: EventEnvelope, member: Option<&str>) -> Option<(Attempt, Worded)> {
    unlabelled(&mut start.payload, member);
    let (key, entries) = boundary_of(&start)?;
    Some((key, Worded::of(&entries, member)))
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

/// What makes the rows the seat view shows of a page's checkpoints, the
/// participant `member`'s, `None` for the seat's own, its read handing
/// over `starts`, the start each attempt on the page last names, one at a
/// time: of each start the words its rows read are kept, its labels let
/// go of first, and
/// the start dropped before the next is read, so a page keeps the six
/// words of each attempt, never a start's entries; and each checkpoint is
/// made into the row the participant scan makes of it. `None` where a
/// checkpoint names no effect or no attempt, or another member than
/// `member`, which no page holds, so the reader reads the run whole.
pub fn page_rows(
    member: Option<&str>,
    starts: &mut dyn Iterator<Item = EventEnvelope>,
) -> impl Fn(&EventEnvelope) -> Option<CheckpointRow> {
    let worded: BTreeMap<Attempt, Worded> =
        starts.filter_map(|start| worded(start, member)).collect();
    let member = member.map(str::to_string);
    move |event| {
        let (key, record, named) = named(&event.payload)?;
        if named != member.as_deref() {
            return None;
        }
        let boundary = worded.get(&key).unwrap_or(&UNSTARTED).boundary(record);
        let checkpoint = event.payload.get("checkpoint").unwrap_or(&Value::Null);
        Some(row_of(checkpoint, &event.recorded_at, boundary))
    }
}

#[cfg(test)]
mod tests;
