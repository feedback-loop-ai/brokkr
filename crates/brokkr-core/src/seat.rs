//! A seat's readings: what its checkpoints and seat record report, read
//! from the journal one way, so the writer's projection and every reader
//! derive a seat's facts alike (decision 0089 ruling 3; decision 0071
//! rulings 5 and 7). What a surface paints of them stays in `brokkr-view`.

pub mod meter;
pub mod row;

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
/// `brokkr costs` sums through this same function, so the ledger and
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

#[cfg(test)]
mod tests;
