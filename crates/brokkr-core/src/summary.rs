//! A run's summary, as the writer keeps it (decision 0089 rulings 2 and
//! 3): the fleet's whole account of one run, projected from its fold.
//!
//! The writer holds an [`Account`] of the run it appends to: the fold at
//! the run's head, or the fold's refusal there, and the residual findings
//! the journal carries with the operator's closing marks. Each append
//! carries it over what it sealed ([`Account::onto`]), and the summary
//! row is written from [`Summary::of`] the result. Both are pure, so the
//! writer, the backfill and every check of a row derive it one way: the
//! account carried over a journal's events is the account read whole.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::canonical::ZERO_HASH;
use crate::envelope::EventEnvelope;
use crate::fold::{fold, fold_onto, FoldError, RunState, Status};
use crate::policy::Severity;
use crate::residual::{note_closing, open_severity, residuals, Claim, Mark, ResidualPhase};
use crate::stamp::parse_millis;

/// The bound of a name a run's data states: a phase, a rule id, the phase
/// that ruled, a seat (decision 0089 ruling 2).
pub(crate) const NAME_BYTES: usize = 64;

/// The bound of a park's reason, which `ParkReason::Stated` leaves free.
pub(crate) const PARK_REASON_BYTES: usize = 256;

/// The bound of a refusal's words.
pub(crate) const REFUSAL_BYTES: usize = 512;

/// The bound of the feature's first line, the title's source.
pub(crate) const FEATURE_LINE_BYTES: usize = 1024;

/// The bound of a stamp: `created_at` and `last_recorded_at`.
pub const STAMP_BYTES: usize = 40;

/// The acknowledgement's copy on the row is cut by characters, as
/// SQLite's `substr` counts them in text: its operator to this many,
pub const ACK_OPERATOR_CHARS: usize = 64;

/// its reason to this many,
pub const ACK_REASON_CHARS: usize = 256;

/// and its time to this many.
pub const ACK_AT_CHARS: usize = 40;

/// `text` cut at a character boundary to at most `bytes` bytes.
pub(crate) fn cut(text: &str, bytes: usize) -> &str {
    if text.len() <= bytes {
        return text;
    }
    let mut end = bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// The first line of `feature` that says anything, cut to at most 1,024
/// bytes (decision 0089 ruling 2): what the fleet's title is derived from, which
/// a title of at most 256 columns reads the same from as from the whole
/// feature, save a line made mostly of zero-width marks.
pub fn feature_line(feature: &str) -> &str {
    let first = feature
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    cut(first, FEATURE_LINE_BYTES)
}

/// A value held within its column's bound, and the whole value when the
/// bound cut it: the column a page selects, and the one last in the row
/// that keeps the value lossless.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounded {
    pub cut: String,
    pub whole: Option<String>,
}

impl Bounded {
    /// `text` within `bytes`.
    pub(crate) fn within(text: &str, bytes: usize) -> Bounded {
        let held = cut(text, bytes);
        Bounded {
            cut: held.to_string(),
            whole: (held.len() < text.len()).then(|| text.to_string()),
        }
    }
}

/// One residual claim of the run, owned so the account outlives the
/// events it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    seq: u64,
    phase: ResidualPhase,
    rule_id: Option<String>,
    claim: Claim,
}

/// A finding as the summary's `residuals` column states it: its claim's
/// input and value by name, and the mark that closes it.
#[derive(Serialize)]
struct Stated<'a> {
    seq: u64,
    phase: &'static str,
    rule_id: Option<&'a str>,
    input: &'static str,
    value: &'static str,
    superseded: Option<&'a Mark>,
}

/// What a writer holds of a run: the run's head as it last judged it, the
/// fold there or the fold's refusal, and the residual findings and
/// closing marks the journal carries up to it.
#[derive(Debug, Clone)]
pub struct Account {
    run_id: String,
    head: (u64, String),
    last_recorded_at: Option<String>,
    read: Result<RunState, FoldError>,
    findings: Vec<Finding>,
    marks: BTreeMap<u64, Mark>,
}

impl Account {
    /// A run with no event: its head is seq 0 and the zero hash, as the
    /// store reads such a run, and its fold refuses as `fold` refuses an
    /// empty journal.
    pub fn empty(run_id: &str) -> Account {
        Account {
            run_id: run_id.to_string(),
            head: (0, ZERO_HASH.to_string()),
            last_recorded_at: None,
            read: Err(FoldError::Empty),
            findings: Vec::new(),
            marks: BTreeMap::new(),
        }
    }

    /// The account of a run's whole journal, `events` in order from seq 1.
    pub fn whole(run_id: &str, events: &[EventEnvelope]) -> Account {
        Account::empty(run_id).onto(events)
    }

    /// The account carried over `suffix`, the events after this head in
    /// order. A run with no event folds from its first; a fold carries on
    /// by [`fold_onto`]; a refusal stands, as a whole fold stops at the
    /// event it refuses. The caller verifies the suffix chains onto this
    /// head first; like [`fold`], this checks protocol shape only.
    pub fn onto(mut self, suffix: &[EventEnvelope]) -> Account {
        let Some(last) = suffix.last() else {
            return self;
        };
        self.read = match self.read {
            Err(FoldError::Empty) => fold(suffix),
            Err(refused) => Err(refused),
            Ok(state) => fold_onto(state, suffix),
        };
        self.findings
            .extend(residuals(suffix).into_iter().map(|residual| {
                Finding {
                    seq: residual.ruling.seq,
                    phase: residual.phase,
                    rule_id: residual
                        .ruling
                        .payload
                        .get("rule_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    claim: residual.claim,
                }
            }));
        for event in suffix {
            note_closing(&mut self.marks, event);
        }
        self.head = (last.seq, last.event_hash.clone());
        self.last_recorded_at = Some(last.recorded_at.clone());
        self
    }

    /// The run this account is of.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// The head it was judged at: the last event's seq and hash.
    pub fn head(&self) -> (u64, &str) {
        (self.head.0, &self.head.1)
    }

    /// The fold at the head, or the fold's refusal there.
    pub fn state(&self) -> Result<&RunState, &FoldError> {
        self.read.as_ref()
    }
}

/// What the run's `runs` row states: its feature and when it was created.
#[derive(Debug, Clone, Copy)]
pub struct Listing<'a> {
    pub feature: &'a str,
    pub created_at: &'a str,
}

/// The last ruling as the fleet reads it: the rule that settled it, the
/// phase that ruled, and whether it routed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ruled {
    pub rule_id: Option<Bounded>,
    pub from: Bounded,
    pub routes: bool,
}

/// The seat the fold's cursor has hired for the current effect, and how
/// many of its attempts have failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hired {
    pub seat: Bounded,
    pub failed_attempts: u64,
}

/// Why the fold refused, and the seq it refused at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub seq: u64,
    pub words: Bounded,
}

/// A run's summary row, every column the writer and the triggers keep
/// (decision 0089 ruling 2), as a fold states it. The acknowledgement's
/// copy is not here: it lives beside the chain and no fold reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub feature_line: String,
    pub created_at: Bounded,
    pub feature: String,
    pub status: Option<Status>,
    pub phase: Option<Bounded>,
    pub park_reason: Option<Bounded>,
    pub ruled: Option<Ruled>,
    pub hired: Option<Hired>,
    pub residual_open: Option<Severity>,
    pub residual_count: u64,
    /// The residual findings with their closing marks, as JSON.
    pub residuals: String,
    pub refused: Option<Refused>,
    pub folded_seq: u64,
    pub folded_hash: String,
    /// The last event's stamp in milliseconds, or `created_at`'s while
    /// the run has none, by the one parse; `None` when it does not parse.
    pub last_at_ms: Option<i64>,
    pub head_seq: u64,
    pub head_hash: String,
    pub last_recorded_at: Option<Bounded>,
}

impl Summary {
    /// The summary of `account`, for the run `listing` lists.
    pub fn of(account: &Account, listing: Listing<'_>) -> Summary {
        let state = account.read.as_ref().ok();
        let stamp = account.last_recorded_at.as_deref();
        Summary {
            feature_line: feature_line(listing.feature).to_string(),
            created_at: Bounded::within(listing.created_at, STAMP_BYTES),
            feature: listing.feature.to_string(),
            status: state.map(|state| state.run.status()),
            phase: state
                .and_then(|state| state.phase.as_deref())
                .map(|phase| Bounded::within(phase, NAME_BYTES)),
            park_reason: state
                .and_then(|state| state.run.park_reason())
                .map(|reason| Bounded::within(reason.as_str(), PARK_REASON_BYTES)),
            ruled: state.and_then(ruled),
            hired: state.and_then(hired),
            residual_open: open_severity(account.findings.iter().map(|finding| {
                (
                    finding.claim.severity(),
                    account.marks.contains_key(&finding.seq),
                )
            })),
            residual_count: account
                .findings
                .iter()
                .filter(|finding| !account.marks.contains_key(&finding.seq))
                .count() as u64,
            residuals: stated(account),
            refused: account.read.as_ref().err().map(|error| Refused {
                seq: error.seq(),
                words: Bounded::within(&error.to_string(), REFUSAL_BYTES),
            }),
            folded_seq: account.head.0,
            folded_hash: account.head.1.clone(),
            last_at_ms: parse_millis(stamp.unwrap_or(listing.created_at)),
            head_seq: account.head.0,
            head_hash: account.head.1.clone(),
            last_recorded_at: stamp.map(|stamp| Bounded::within(stamp, STAMP_BYTES)),
        }
    }
}

fn ruled(state: &RunState) -> Option<Ruled> {
    let decision = state.last_decision.as_ref()?;
    Some(Ruled {
        rule_id: decision
            .rule_id
            .present()
            .map(|rule| Bounded::within(rule, NAME_BYTES)),
        from: Bounded::within(&decision.from, NAME_BYTES),
        routes: decision.next.present().is_some(),
    })
}

fn hired(state: &RunState) -> Option<Hired> {
    let (seat, failed_attempts) = state.run.cursor()?.hire()?;
    Some(Hired {
        seat: Bounded::within(seat, NAME_BYTES),
        failed_attempts,
    })
}

/// The account's findings as the `residuals` column states them, as
/// canonical JSON.
fn stated(account: &Account) -> String {
    let findings: Vec<Stated<'_>> = account
        .findings
        .iter()
        .map(|finding| Stated {
            seq: finding.seq,
            phase: finding.phase.as_str(),
            rule_id: finding.rule_id.as_deref(),
            input: finding.claim.input(),
            value: finding.claim.value(),
            superseded: account.marks.get(&finding.seq),
        })
        .collect();
    // Through a value, so its keys sort: the canonical form (`canonical`).
    serde_json::to_value(&findings)
        .expect("a list of findings serializes")
        .to_string()
}

#[cfg(test)]
mod tests;
