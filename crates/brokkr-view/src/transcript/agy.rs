//! Antigravity CLI (`agy-conversation`) transcript validation and projection
//! (#1227, decision 0075 level 2).
//!
//! Each line of `transcript.jsonl` is one JSON step row. Rows are projected in
//! exact file order — `step_index` is neither trusted as unique nor used to
//! sort (`l2b-two-rejoins.transcript.ndjson` carries two rows with index 4,
//! one after index 6). `Turn` carries `{role, ts, blocks}` and no per-row
//! model field, so a `PLANNER_RESPONSE` row's `model` slug is absent from the
//! projection without loss.
//!
//! **Tool call arguments (#1227 W1).** This reader never parses and never
//! re-serializes what a row holds as a string. Each member of
//! `tool_calls[].args` is shown as `name=value` using the value's text exactly
//! as the row holds it after the row's own one JSON parse: for a string value,
//! the string's characters verbatim; for any other value, its compact JSON.
//! Agy's string values are already JSON text written once (`"sleep 90"`,
//! `{"command":"cat seed.txt"}`), so every character of a tool's argument
//! stands raw or escaped exactly once — the two forms `mask_secrets` covers —
//! and this is the same number of escapings the claude and codex readers give
//! a tool's input.
//!
//! **Field bounds inventory (#1227 W2).** Every field of [`Turn`] and
//! [`Projection`] filled from the file is bounded:
//! - `Turn.role`: static literal from the closed 4-variant [`RowKind`] enum
//!   (`"user"`, `"assistant"`, `"tool"`, `"system"`), never copied from file
//!   bytes.
//! - `Turn.ts`: admitted only when `created_at` matches the exact 20-byte
//!   UTC second format `YYYY-MM-DDTHH:MM:SSZ` with valid calendar date and
//!   clock; any other value becomes `""` and increments a counted notice
//!   without dropping or skipping the row.
//! - `Turn.blocks`: each block has a closed [`super::BlockKind`] and text from
//!   `content`, `thinking`, `error`, `tool_calls[].{name, args}`, or static
//!   [`UNFINISHED_NOTICE`], bounded by [`TurnCollector`] under
//!   [`super::DISPLAY_CAP`] and masked by `local_transcript::mask_secrets`
//!   before any surface prints it.
//! - `Projection.turns`: bounded by [`TurnCollector`] under
//!   [`super::DISPLAY_CAP`] (4,000,000 bytes, [`super::DISPLAY_EVENT_COST`] =
//!   512 bytes per turn).
//! - `Projection.skipped_lines` and `Projection.unrecognized_records`: `u64`
//!   counts bounded by lines in the [`super::SOURCE_CAP`] (32 MiB) snapshot.
//! - `Projection.truncated`: `bool` from source overflow or display cap.
//! - `Projection.unavailable`: closed [`super::Unavailable`] enum set only by
//!   snapshot admission (UTF-8 check).
//! - `Projection.notices`: at most one static-format count string
//!   ([`invalid_timestamp_notice`]), carrying no file bytes.

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::Value;

use super::{
    compose, parsed_rows, Admitted, Block, Projection, Role, RowOutcome, Tail, TurnCollector,
};

/// Words shown for a row whose `status` is `RUNNING` when the turn ended
/// (for instance a killed background command in `l2-killed.transcript.ndjson`).
pub(super) const UNFINISHED_NOTICE: &str = "unfinished (step was still running)";

/// Counted notice emitted when one or more recognized rows carried a
/// `created_at` value outside the exact 20-byte `YYYY-MM-DDTHH:MM:SSZ` form
/// (#1227 W2). Carries no bytes from the file.
pub(super) fn invalid_timestamp_notice(count: u64) -> String {
    format!("agy timestamps omitted (not YYYY-MM-DDTHH:MM:SSZ): {count}")
}

/// A row's `created_at` member, distinguishing an absent member from a
/// present value including JSON `null` (#1227 Y7).
#[derive(Default)]
enum CreatedAt {
    #[default]
    Absent,
    Text(String),
    Other,
}

impl<'de> Deserialize<'de> for CreatedAt {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Present {
            Text(String),
            Other(IgnoredAny),
        }
        Ok(match Present::deserialize(deserializer)? {
            Present::Text(text) => Self::Text(text),
            Present::Other(_) => Self::Other,
        })
    }
}

#[derive(Deserialize)]
struct AgyRow {
    source: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    status: Option<String>,
    #[serde(default)]
    created_at: CreatedAt,
    content: Option<String>,
    error: Option<String>,
    thinking: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Deserialize)]
struct ToolCall {
    name: String,
    args: Option<Value>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowKind {
    UserInput,
    PlannerResponse,
    ModelGeneric,
    SystemMessage,
}

impl RowKind {
    fn classify(source: Option<&str>, kind: Option<&str>) -> Option<Self> {
        match (source?, kind?) {
            ("USER_EXPLICIT", "USER_INPUT") => Some(Self::UserInput),
            ("MODEL", "PLANNER_RESPONSE") => Some(Self::PlannerResponse),
            ("MODEL", "GENERIC") => Some(Self::ModelGeneric),
            ("SYSTEM", "SYSTEM_MESSAGE") => Some(Self::SystemMessage),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowStatus {
    Done,
    Error,
    Running,
    Other,
}

impl RowStatus {
    fn parse(word: Option<&str>) -> Self {
        match word {
            None | Some("DONE") => Self::Done,
            Some("ERROR") => Self::Error,
            Some("RUNNING") => Self::Running,
            Some(_) => Self::Other,
        }
    }
}

/// Project an admitted `agy-conversation` snapshot into `projection` in file
/// order under the shared display budget. Because a kept `agy-conversation`
/// file was copied after `agy` exited, it is complete and a malformed
/// newline-less final line counts as skipped (#1227 U7).
pub(super) fn project(admitted: &Admitted<'_>, projection: &mut Projection) {
    let mut collector = TurnCollector::new(projection.truncated);
    let mut invalid_timestamps = 0u64;
    projection.skipped_lines = parsed_rows(admitted, Tail::Complete, |_, value, _| {
        let outcome = project_row(value, &mut invalid_timestamps);
        if outcome.unrecognized {
            projection.unrecognized_records += 1;
        }
        collector.admit(outcome.role, outcome.ts, outcome.blocks);
    });
    if invalid_timestamps > 0 {
        projection
            .notices
            .push(invalid_timestamp_notice(invalid_timestamps));
    }
    collector.finish(projection);
}

fn project_row(value: Value, invalid_timestamps: &mut u64) -> RowOutcome<Block> {
    if !value.is_object() {
        return RowOutcome::nothing(true);
    }
    let Ok(row) = serde_json::from_value::<AgyRow>(value) else {
        return RowOutcome::nothing(true);
    };
    let Some(kind) = RowKind::classify(row.source.as_deref(), row.kind.as_deref()) else {
        return RowOutcome::nothing(true);
    };
    let status = RowStatus::parse(row.status.as_deref());
    let ts = match row.created_at {
        CreatedAt::Absent => String::new(),
        CreatedAt::Text(ts) if is_agy_timestamp(&ts) => ts,
        CreatedAt::Text(_) | CreatedAt::Other => {
            *invalid_timestamps += 1;
            String::new()
        }
    };
    let (role, mut blocks) = match kind {
        RowKind::UserInput => (Role::User, user_blocks(row.content)),
        RowKind::PlannerResponse => (
            Role::Assistant,
            planner_blocks(row.thinking, row.content, row.tool_calls),
        ),
        RowKind::ModelGeneric => (Role::Tool, generic_blocks(status, row.content, row.error)),
        RowKind::SystemMessage => (Role::System, user_blocks(row.content)),
    };
    if status == RowStatus::Running {
        blocks.push(Block::text(UNFINISHED_NOTICE));
    }
    let mut outcome = RowOutcome::of(blocks, role);
    outcome.ts = ts;
    outcome
}

struct TimestampFields {
    year: u32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

/// Whether `ts` is an exact 20-byte ASCII UTC timestamp `YYYY-MM-DDTHH:MM:SSZ`
/// with a valid calendar date and clock (#1227 W2).
fn is_agy_timestamp(ts: &str) -> bool {
    let Some(bytes) = timestamp_bytes(ts) else {
        return false;
    };
    let Some(fields) = parse_timestamp_fields(bytes) else {
        return false;
    };
    timestamp_in_range(fields)
}

fn timestamp_bytes(ts: &str) -> Option<&[u8; 20]> {
    let b: &[u8; 20] = ts.as_bytes().try_into().ok()?;
    if b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    Some(b)
}

fn parse_ascii_digits(slice: &[u8]) -> Option<u32> {
    let mut value = 0u32;
    for &byte in slice {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value * 10 + u32::from(byte - b'0');
    }
    Some(value)
}

fn parse_timestamp_fields(b: &[u8; 20]) -> Option<TimestampFields> {
    Some(TimestampFields {
        year: parse_ascii_digits(&b[0..4])?,
        month: parse_ascii_digits(&b[5..7])?,
        day: parse_ascii_digits(&b[8..10])?,
        hour: parse_ascii_digits(&b[11..13])?,
        minute: parse_ascii_digits(&b[14..16])?,
        second: parse_ascii_digits(&b[17..19])?,
    })
}

fn days_in_month(year: u32, month: u32) -> Option<u32> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => {
            let leap =
                (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400);
            Some(if leap { 29 } else { 28 })
        }
        _ => None,
    }
}

fn timestamp_in_range(fields: TimestampFields) -> bool {
    let Some(max_day) = days_in_month(fields.year, fields.month) else {
        return false;
    };
    (1..=max_day).contains(&fields.day)
        && fields.hour <= 23
        && fields.minute <= 59
        && fields.second <= 60
}

fn user_blocks(content: Option<String>) -> Vec<Block> {
    content
        .filter(|text| !text.is_empty())
        .map(Block::text)
        .into_iter()
        .collect()
}

fn planner_blocks(
    thinking: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
) -> Vec<Block> {
    let mut blocks = Vec::new();
    if let Some(thought) = thinking.filter(|text| !text.is_empty()) {
        blocks.push(Block::reasoning(thought));
    }
    if let Some(text) = content.filter(|text| !text.is_empty()) {
        blocks.push(Block::text(text));
    }
    if let Some(calls) = tool_calls {
        for call in calls {
            let args = format_args(call.args.as_ref()).unwrap_or_default();
            let text = compose(&[&call.name, &args]);
            if !text.is_empty() {
                blocks.push(Block::tool(text));
            }
        }
    }
    blocks
}

/// Format `tool_calls[].args` without ever parsing or re-serializing any
/// string value (#1227 W1). Each object member renders as `name=value` with
/// string values written verbatim and non-string values as compact JSON.
fn format_args(args: Option<&Value>) -> Option<String> {
    match args {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Object(map)) if map.is_empty() => None,
        Some(Value::Object(map)) => {
            let parts: Vec<String> = map
                .iter()
                .map(|(name, value)| match value {
                    Value::String(text) => format!("{name}={text}"),
                    other => format!("{name}={other}"),
                })
                .collect();
            Some(parts.join(", "))
        }
        Some(other) => Some(other.to_string()),
    }
}

fn generic_blocks(status: RowStatus, content: Option<String>, error: Option<String>) -> Vec<Block> {
    let body = match status {
        RowStatus::Error => error
            .filter(|text| !text.is_empty())
            .or_else(|| content.filter(|text| !text.is_empty())),
        RowStatus::Done | RowStatus::Running | RowStatus::Other => {
            content.filter(|text| !text.is_empty())
        }
    };
    body.map(Block::tool_result).into_iter().collect()
}
