//! A seat's readings: what its checkpoints and seat record report, read
//! from the journal one way, so the writer's projection and every reader
//! derive a seat's facts alike (decision 0089 ruling 3; decision 0071
//! rulings 5 and 7). What a surface paints of them stays in `brokkr-view`.

mod costs;
pub mod meter;
pub mod row;

pub use costs::{seat_costs, SeatCost};
use serde::Serialize;
use serde_json::Value;

use crate::realms::Boundary;
use crate::residual::text as field;
use meter::sum_usage;

// ----------------------------------------------------------------- tokens

/// Harness-reported token accounting under the seat-record vocabulary.
/// `total_tokens` is exactly input + output. Cache reads are already a
/// subset of input, reasoning output is already a subset of output, and
/// cache writes stay separately visible, so none of the three is
/// silently added a second time.
///
/// `reasoning_output_tokens` is decision 0035 ruling 4's field and the
/// only figure in this record that METERS what a model did — the effort
/// beside it is what was configured, and the model name is a claim. It
/// is absent, never zero, wherever the harness reported none: claude
/// meters its thinking only in its result, codex meters it per turn,
/// and this record admits both without inventing the missing one.
#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub reasoning_output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// The tokens a seat actually spent, summed across its own turns.
///
/// Per-turn checkpoints are authoritative. The finishing checkpoint is
/// only the fallback for old or externally-authored records that do not
/// carry turn accounting; reading both would count the same work twice.
///
/// `cache_read_tokens` is not an addend: a cache read IS an input token,
/// billed differently, and it arrives inside `input_tokens` already
/// (3,830,272 of the wager's 3,975,322). Adding it would double-count.
/// That is the journal's convention, not any one harness's — dsh counts
/// the two disjointly and its adapter folds the cache read back into
/// `input_tokens` before journaling, so this sum holds whichever driver
/// wrote the checkpoint. See "Checkpoints" in
/// `docs/guides/driver-authoring.md`.
pub fn add_token(total: &mut Option<u64>, count: Option<u64>) {
    if let Some(count) = count {
        *total = Some(total.unwrap_or_default().saturating_add(count));
    }
}

pub fn finish_usage(mut usage: TokenUsage) -> Option<TokenUsage> {
    usage.total_tokens = match (usage.input_tokens, usage.output_tokens) {
        (None, None) => None,
        (input, output) => Some(
            input
                .unwrap_or_default()
                .saturating_add(output.unwrap_or_default()),
        ),
    };
    // Reasoning never enters `total_tokens` — it is already inside
    // `output_tokens` (decision 0035 ruling 4), exactly as a cache read
    // is already inside `input_tokens`. It counts only towards whether
    // this record has anything to say at all.
    if usage.total_tokens.is_some()
        || usage.cache_read_tokens.is_some()
        || usage.cache_write_tokens.is_some()
        || usage.reasoning_output_tokens.is_some()
    {
        Some(usage)
    } else {
        None
    }
}

pub fn token_usage<'a>(records: impl Iterator<Item = &'a Value>) -> Option<TokenUsage> {
    let count = |record: &Value, key: &str| record.get(key).and_then(Value::as_u64);
    sum_usage(records.map(|record| TokenUsage {
        input_tokens: count(record, "input_tokens"),
        output_tokens: count(record, "output_tokens"),
        cache_read_tokens: count(record, "cache_read_tokens"),
        cache_write_tokens: count(record, "cache_write_tokens"),
        reasoning_output_tokens: count(record, "reasoning_output_tokens"),
        total_tokens: None,
    }))
}

/// One completed item of the seat's work: the step name decides, and a
/// checkpoint of any other step is ignored rather than guessed at.
pub fn count_item(items: &mut Option<u64>, step: Option<&str>) {
    if step == Some("item-completed") {
        *items.get_or_insert(0) += 1;
    }
}

// ------------------------------------------------------------- transcript

/// The common driver transcript reference (decision 0032). It contains
/// paths or ids only; transcript prose never enters the journal or view.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Transcript {
    pub kind: String,
    pub locator: String,
    pub home: String,
}

/// The latest recorded common transcript reference, preserved verbatim.
/// Selection happens before validation: an unrecognized kind is still a
/// recorded reference the reader must echo as `unsupported-kind`, so it
/// must not be filtered out here and leave a stale earlier reference in
/// its place.
pub fn transcript_of(value: &Value) -> Option<Transcript> {
    let kind = value.get("kind").and_then(Value::as_str)?;
    let locator = value.get("locator").and_then(Value::as_str)?;
    let home = value.get("home").and_then(Value::as_str)?;
    Some(Transcript {
        kind: kind.to_string(),
        locator: locator.to_string(),
        home: home.to_string(),
    })
}

// ------------------------------------------------------------------- cost

/// The spend one record reports: its `total_cost_usd`, nothing else.
///
/// The attempt rule is stated here, once (#376): a seat's spend is this
/// figure summed over every record of every attempt — a retry's
/// finishing checkpoint beside the concluded attempt's, never the last
/// alone. Every view surface reads the sum as `Participant::cost`, and
/// `brokkr stats costs` sums through this same function, so the ledger and
/// the views report one number for one run.
pub fn reported_cost(record: &Value) -> Option<f64> {
    record.get("total_cost_usd").and_then(Value::as_f64)
}

// --------------------------------------------------------------- boundary

/// One well-formed `effect/started.boundary` entry (decision 0046 ruling
/// 3): its site tag, its plain word and whether the site held a gate.
pub struct BoundaryEntry {
    pub member: Option<String>,
    pub word: String,
    pub gate: bool,
}

/// The entries of one `effect/started` this reader accepts. An entry
/// whose word is outside the six — the five boundaries and decision
/// 0031's sentinel — or which lacks its `member` key is *not recorded*
/// for that site, never boxed and never unboxed (design DD14); it is
/// dropped here, before any surface sees it.
pub fn boundary_entries(payload: &Value) -> Vec<BoundaryEntry> {
    payload
        .get("boundary")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    entry.get("member")?;
                    let word = field(entry, "boundary")?;
                    Boundary::from_recorded(word)?;
                    Some(BoundaryEntry {
                        member: entry
                            .get("member")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        word: word.to_string(),
                        gate: entry.get("gate").and_then(Value::as_bool).unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The word a seat record carries beside its model (design DD19), read
/// under the same vocabulary rule as an entry: a word outside the six is
/// no stamp at all.
pub fn stamp_of(record: &Value) -> Option<String> {
    let word = field(record, "boundary")?;
    Boundary::from_recorded(word)?;
    Some(word.to_string())
}

// ------------------------------------------------------------- transcript

/// Maximum source bytes admitted for a local transcript snapshot (32 MiB):
/// the one shared cap used by `brokkr-view`'s bounded reader and `brokkr-protocol`'s
/// `agy` transcript keeper (#1227, decision 0071 ruling 5).
pub const TRANSCRIPT_SOURCE_CAP: u64 = 33_554_432;

/// Wire kind label for an Antigravity CLI retained conversation transcript:
/// one home shared by `brokkr-protocol` and `brokkr-view` (#1227, decision
/// 0071 ruling 5).
pub const AGY_TRANSCRIPT_KIND: &str = "agy-conversation";

/// Basename of a kept Antigravity CLI conversation transcript file under
/// `<home>/<uuid>/`: one home shared by `brokkr-protocol`'s keeper and
/// `brokkr-cli`'s local-transcript discovery (#1227, decision 0071 ruling 5).
pub const AGY_TRANSCRIPT_FILE: &str = "transcript.jsonl";

/// Maximum byte length of a seat-record v8 kind word.
const RECORD_KIND_LIMIT: usize = 40;

/// Whether `text` is one kind of seat-record v8's grammar,
/// `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`, of at most 40 bytes. One home shared by
/// the runtime, the adapter loader and the declared-transcript carrier
/// (#1227 Y2, decision 0071 ruling 5); held to the contract's pattern by
/// `brokkr_runtime::agents::kinds::tests::the_kind_grammar_is_the_contracts`.
pub fn is_record_kind_word(text: &str) -> bool {
    let lower = |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit();
    text.len() <= RECORD_KIND_LIMIT
        && text.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && text
            .split('-')
            .all(|part| !part.is_empty() && part.bytes().all(lower))
}

/// Whether `text` is a valid declared `record_kinds` kind: one word of
/// [`is_record_kind_word`] and never the reserved word `"none"` (#1227 Y2).
pub fn is_declared_record_kind(text: &str) -> bool {
    is_record_kind_word(text) && text != "none"
}

/// True when every character is legal in a transcript path input: no ASCII
/// control character and no NUL. One home shared by `brokkr-view` and
/// `brokkr-protocol` (#1227 Y3, decision 0071 ruling 5).
pub fn is_clean_transcript_path(text: &str) -> bool {
    !text.chars().any(char::is_control)
}

/// Whether `home` is a valid recorded transcript home: non-empty, free of
/// control characters and NUL, and absolute. Unix roots begin with `/`;
/// drive-absolute (`C:\` or `C:/`) and UNC (`\\server\share`) spellings
/// are also recognized so recorded homes remain readable without widening
/// relative paths. One home shared by `brokkr-view` and `brokkr-protocol`
/// (#1227 Y3, decision 0071 ruling 5).
pub fn is_valid_transcript_home(home: &str) -> bool {
    if home.is_empty() || !is_clean_transcript_path(home) {
        return false;
    }
    if home.starts_with('/') {
        return true;
    }
    let bytes = home.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    home.starts_with("\\\\") && home.len() > 2
}

/// Whether `home` uses the supported host's native absolute syntax (leading
/// `/`, Linux and macOS per decision 0063), so a foreign-platform drive or UNC
/// spelling is refused before any filesystem call rather than reinterpreted as
/// a host-relative path. One home shared by `brokkr-cli`'s transcript discovery
/// and `brokkr-protocol`'s `agy` keeper (#1227 Y3, decision 0071 ruling 5).
pub fn is_native_transcript_home(home: &str) -> bool {
    home.starts_with('/')
}

/// Whether `id` is an exact lowercase UUID in Antigravity CLI's 8-4-4-4-12
/// form (36 ASCII bytes: lowercase hex digits with hyphens at 8, 13, 18 and
/// 23). One home shared by the driver's path gate and the view's locator
/// validator (#1227, decision 0071 ruling 5).
pub fn is_agy_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.len() != 36 {
        return false;
    }
    bytes.iter().enumerate().all(|(index, &byte)| match index {
        8 | 13 | 18 | 23 => byte == b'-',
        _ => byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'),
    })
}

#[cfg(test)]
mod tests;
