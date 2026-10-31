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

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::canonical::ZERO_HASH;
use crate::envelope::EventEnvelope;
use crate::fold::{fold, fold_onto, FoldError, RunState, Status};
use crate::policy::Severity;
use crate::residual::{
    note_closing, open_severity, residuals, Claim, Mark, Residual, ResidualPhase,
};
use crate::seat::row::{SeatRow, Seats};
use crate::stamp::parse_millis;

mod aggregate;
mod park;

use aggregate::Reading;
pub use aggregate::{aggregate, Aggregate, Ending};
pub use park::{parks, Park};

/// The bound of a name a run's data states: a phase, a rule id, the phase
/// that ruled, a seat (decision 0089 ruling 2).
pub const NAME_BYTES: usize = 64;

/// The bound of a park's reason, which `ParkReason::Stated` leaves free.
pub const PARK_REASON_BYTES: usize = 256;

/// The bound of a refusal's words.
pub const REFUSAL_BYTES: usize = 512;

/// The bound of the feature's first line, the title's source.
pub const FEATURE_LINE_BYTES: usize = 1024;

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
    pub fn within(text: &str, bytes: usize) -> Bounded {
        let held = cut(text, bytes);
        Bounded {
            cut: held.to_string(),
            whole: (held.len() < text.len()).then(|| text.to_string()),
        }
    }
}

/// One residual finding of a run, owned so it outlives the events it was
/// read from, typed: the phase that ruled, the rule it names and its
/// claim, and the mark that closes it. The summary's `residuals` column
/// is written from it and read back into it, its phase and claim by name,
/// and a name no finding states refuses there (decision 0071 ruling 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stated {
    pub seq: u64,
    pub phase: ResidualPhase,
    pub rule_id: Option<String>,
    pub claim: Claim,
    pub superseded: Option<Mark>,
}

impl Stated {
    /// `residual` as a finding, closed by `superseded` where a mark
    /// closes it.
    pub fn of(residual: &Residual<'_>, superseded: Option<Mark>) -> Stated {
        Stated {
            seq: residual.ruling.seq,
            phase: residual.phase,
            rule_id: residual.rule_id().map(str::to_string),
            claim: residual.claim,
            superseded,
        }
    }
}

/// A finding as the `residuals` column words it: its phase, and its
/// claim's input and value, by name.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Words {
    seq: u64,
    phase: String,
    rule_id: Option<String>,
    input: String,
    value: String,
    superseded: Option<Mark>,
}

/// Why the `residuals` column states a finding no writer writes.
#[derive(Debug, Error, PartialEq, Eq)]
enum Unstated {
    #[error("phase '{0}' carries no residual finding")]
    Phase(String),
    #[error("'{0}: {1}' is no claim a finding states")]
    Claim(String, String),
}

impl Serialize for Stated {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Words {
            seq: self.seq,
            phase: self.phase.as_str().to_string(),
            rule_id: self.rule_id.clone(),
            input: self.claim.input().to_string(),
            value: self.claim.value().to_string(),
            superseded: self.superseded.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Stated {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Stated, D::Error> {
        let words = Words::deserialize(deserializer)?;
        let phase = ResidualPhase::named(&words.phase).ok_or(Unstated::Phase(words.phase));
        let claim = Claim::named(&words.input, &words.value)
            .ok_or(Unstated::Claim(words.input, words.value));
        let refused = |unstated: Unstated| serde::de::Error::custom(unstated);
        Ok(Stated {
            seq: words.seq,
            phase: phase.map_err(refused)?,
            rule_id: words.rule_id,
            claim: claim.map_err(refused)?,
            superseded: words.superseded,
        })
    }
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
    /// Each as its ruling states it, unmarked: the marks are kept beside.
    findings: Vec<Stated>,
    marks: BTreeMap<u64, Mark>,
    /// Its seats' rows (decision 0089 ruling 1).
    seats: Seats,
    /// What the aggregate readouts read of it (ruling 7).
    reading: Reading,
    /// Its parks (ruling 7).
    parks: park::Parking,
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
            seats: Seats::default(),
            reading: Reading::default(),
            parks: park::Parking::default(),
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
        self.findings.extend(
            residuals(suffix)
                .iter()
                .map(|residual| Stated::of(residual, None)),
        );
        for event in suffix {
            note_closing(&mut self.marks, event);
            self.seats.note(event);
            self.reading.note(event);
            self.parks.note(event);
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

    /// Its seats' rows, in the order they were opened, whatever the fold
    /// read: what [`facts`](crate::seat::row::facts) groups.
    pub fn seats(&self) -> &[SeatRow] {
        self.seats.rows()
    }

    /// Whether its seats' rows keep all the run view reads of them: not
    /// while the run's last start, unended, names no effect or one no
    /// request has asked for, whose live block no row holds. The writer
    /// keeps no seat head for such a run, so a reader reads it whole
    /// (decision 0089 ruling 8).
    pub fn seats_kept(&self) -> bool {
        self.seats.keep_live()
    }

    /// Its parks, in the order they opened, each with the operator's answer
    /// and the run's next move where they have happened (ruling 7).
    pub fn parks(&self) -> Vec<Park> {
        self.parks.parks()
    }

    /// The seq of every `run/parked` event of the run, in order, including
    /// one whose stamp did not parse and so opened no park: the writer
    /// records its row processed, with no dated facts.
    pub fn park_rows(&self) -> &[u64] {
        self.parks.rows()
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
    /// An operator stop accepted with no ending written, at the seq it
    /// was accepted: the run is stopping, not running. `None` otherwise.
    pub stop_accepted_seq: Option<u64>,
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
    /// What the aggregate readouts read of the run (ruling 7), whole.
    pub aggregate: Aggregate,
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
            stop_accepted_seq: state.and_then(RunState::stopping_seq),
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
            aggregate: account.reading.of(state),
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
    let findings: Vec<Stated> = account
        .findings
        .iter()
        .map(|finding| Stated {
            superseded: account.marks.get(&finding.seq).cloned(),
            ..finding.clone()
        })
        .collect();
    // Through a value, so its keys sort: the canonical form (`canonical`).
    serde_json::to_value(&findings)
        .expect("a list of findings serializes")
        .to_string()
}

#[cfg(test)]
mod tests;
