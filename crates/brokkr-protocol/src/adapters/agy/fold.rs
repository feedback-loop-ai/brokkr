//! Agy's stream folded (#1200, G9). The stream is agy's own shape, not
//! Claude Code's (the record's Facts): one `init`, `step_update` rows, and
//! one `result` per turn. Each line is read as agy wrote its bytes, and a
//! known row is typed once, straight from its text, before any mask; nothing
//! of it is carried past this module as JSON. The pin, the status, the
//! denial, the response and the signed-out words are each read from that
//! one typed row (J2, K1).
//!
//! - One launch is one turn (J3): a second `init`, a second `result`, a
//!   known row behind the `result`, a known row that does not type, one that
//!   repeats a key the fold reads, and a line that is not UTF-8 each fail
//!   the turn. A line that is not JSON, and one whose `event` the fold does
//!   not know, are passed over and change nothing the verdict reads (K6).
//!   Once a fault is seen, no later row changes the verdict.
//! - The served model is init's `model`, which repeats the slug as asked; a
//!   seat whose init names another is refused before any row counts.
//! - A turn agy stopped at a denial is a success by every outer sign: the
//!   only sign is the result's `denied_actions`. A denied step's own
//!   `state` is not reliable (`ERROR` in `denied-command-default`, `DONE`
//!   in `denied-command-accept-edits`), so it is never read for one.
//! - The verdict is read in one order ([`AgyFold::verdict`], K2), and every
//!   failure and refusal that follows a typed result names the first
//!   result's status before its own words, a fault or none before it (K5,
//!   L5).
//! - Every word of agy's the driver's words name is masked whole through
//!   the seat's bindings first, and only then escaped and bounded (K3).
//! - The answer is a `SUCCESS` result's non-empty `response`, carried in
//!   the accepting verdict (J7).
//! - The result's `usage` is the invocation's own, in tokens with no price
//!   and no cache-write count: one figure per invocation, from the result.
//!   A cache read is not part of `input_tokens` (`tools-turn`: the second
//!   step's 2,289 input beside 6,851 read), so the journal's inclusive
//!   input adds it back, as the Claude fold does; thinking rides as
//!   reasoning output.
//! - A `tool` step is passed over: the checkpoint's tool observation has no
//!   format for agy's telemetry without a contract change.

use std::fmt;

use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use thiserror::Error;

use super::super::refusal::refusal_reason;
use super::super::{add_usage, secret, EFFORT_NOT_REPORTED};
use crate::PreSessionRefusal;

/// The words agy writes when it is signed out, in a result's `error` and on
/// stderr (the record's `signed-out` turn). Only matched, on the text as agy
/// wrote it.
pub(super) const SIGNED_OUT: &str = "authentication failed or timed out";

/// The most characters of a word of agy's the driver's words name.
const NAMED: usize = 40;

/// The rows of a turn: the one list of the events the fold knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Event {
    Init,
    StepUpdate,
    Result,
}

impl Event {
    /// The row, as the driver's words name it.
    fn row(self) -> &'static str {
        match self {
            Event::Init => "an init row",
            Event::StepUpdate => "a step_update row",
            Event::Result => "a result row",
        }
    }

    /// The row `text` holds as this event's, typed straight from the text,
    /// or none where it does not type: a key the fold reads written twice,
    /// at any depth, is no row (K1).
    fn typed(self, text: &str) -> Option<Row> {
        match self {
            Event::Init => {
                (serde_json::from_str(text).ok()).map(|row: InitRow| Row::Init(row.init))
            }
            Event::StepUpdate => {
                (serde_json::from_str(text).ok()).map(|row: StepRow| Row::Step(row.step_update))
            }
            Event::Result => {
                (serde_json::from_str(text).ok()).map(|row: ResultRow| Row::Result(row.result))
            }
        }
    }
}

/// A line's top-level `event` value: one the fold knows, or any other.
#[derive(Deserialize)]
#[serde(untagged)]
enum Named {
    Known(Event),
    Other(IgnoredAny),
}

/// The `event`s a line's object names at its top level, read from its
/// text: the first the fold knows, and how many `event` keys it holds.
struct Tags {
    known: Option<Event>,
    named: usize,
}

impl<'de> Deserialize<'de> for Tags {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Tags, D::Error> {
        deserializer.deserialize_map(TagsVisitor)
    }
}

struct TagsVisitor;

impl<'de> Visitor<'de> for TagsVisitor {
    type Value = Tags;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object naming an event")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Tags, A::Error> {
        let mut tags = Tags {
            known: None,
            named: 0,
        };
        while let Some(key) = map.next_key::<String>()? {
            if key != "event" {
                map.next_value::<IgnoredAny>()?;
                continue;
            }
            tags.named += 1;
            if let Named::Known(event) = map.next_value()? {
                tags.known.get_or_insert(event);
            }
        }
        Ok(tags)
    }
}

/// One known row of agy's stream, as far as the fold reads it.
enum Row {
    Init(Init),
    Step(Step),
    Result(Ending),
}

#[derive(Deserialize)]
struct InitRow {
    init: Init,
}

#[derive(Deserialize)]
struct StepRow {
    step_update: Step,
}

#[derive(Deserialize)]
struct ResultRow {
    result: Ending,
}

#[derive(Deserialize)]
struct Init {
    model: Option<String>,
}

#[derive(Deserialize)]
struct Step {
    step_type: StepType,
    state: StepState,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StepType {
    AgentResponse,
    #[serde(other)]
    Other,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum StepState {
    Done,
    #[serde(other)]
    Other,
}

/// A turn's `result`. Its `status` is named; `denied_actions`, where
/// present, is a list, and where absent is no denial, as every recorded
/// success shows.
#[derive(Deserialize)]
struct Ending {
    status: Status,
    #[serde(default)]
    response: String,
    #[serde(default)]
    usage: Usage,
    num_turns: Option<u64>,
    #[serde(default)]
    denied_actions: Vec<Denied>,
    error: Option<String>,
}

impl Ending {
    fn signed_out(&self) -> bool {
        (self.error.as_deref()).is_some_and(|error| error.contains(SIGNED_OUT))
    }
}

/// A result's status: `SUCCESS`, or any other word, which is named.
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Status {
    Success,
    #[serde(untagged)]
    Other(String),
}

impl Status {
    /// The status as agy wrote it.
    fn word(&self) -> &str {
        match self {
            Status::Success => "SUCCESS",
            Status::Other(word) => word,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    thinking_tokens: u64,
    cache_read_tokens: u64,
}

#[derive(Deserialize)]
struct Denied {
    action: String,
}

/// A turn that ended with no answer, in the driver's own words: no byte of
/// agy's but a word [`named`] built.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Failure {
    #[error(
        "agy stopped the turn at a denial of {}: a turn agy stopped at a denial is not an answer \
         (#1200)",
        .0.join(", ")
    )]
    Denied(Vec<String>),
    /// A `SUCCESS` with an empty response, which the failure's [`Seen`]
    /// names (M4).
    #[error("agy's response was empty, which is not an answer (#1200)")]
    Empty,
    /// A status that is not `SUCCESS`, which the failure's [`Seen`] names.
    #[error("agy's turn ended with a status that is not SUCCESS, which is not an answer (#1200)")]
    Unsuccessful,
    #[error("agy's turn ended with no result, so it gave no answer (#1200)")]
    Unended,
    #[error("agy exited {0}, so the turn gave no answer (#1200)")]
    Exited(i32),
    #[error("agy wrote a second init row, and one launch is one turn (#1200)")]
    Reinitialized,
    #[error("agy wrote {} behind the turn's result, and one launch is one turn (#1200)", .0.row())]
    Behind(Event),
    #[error("agy wrote {} the driver cannot read, so the turn gave no answer (#1200)", .0.row())]
    Unread(Event),
    #[error("agy wrote a line that is not UTF-8, so the turn gave no answer (#1200)")]
    NotText,
    /// A stdout that failed to read before its end (L3).
    #[error("agy's stdout could not be read to its end, so the turn gave no answer (#1200)")]
    Unreadable,
}

/// What the turn's result and exit add to the words of a verdict that is
/// no answer: the first result's status where one came, named before the
/// verdict's own words (K5, L5), and the print timeout where a fault agy
/// ended its turn behind outranked it, named after them (K2, L4).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Seen {
    pub(super) status: Option<String>,
    pub(super) timed_out: bool,
}

impl Seen {
    /// `words` with what was seen around them: the status first, so no
    /// bound applied behind it cuts it, and the timeout last.
    fn around(&self, words: impl fmt::Display) -> String {
        let status = (self.status.as_ref())
            .map(|status| format!("agy's result had status {status}; "))
            .unwrap_or_default();
        let timed_out = match self.timed_out {
            true => "; agy then exited 3 at its own print timeout",
            false => "",
        };
        format!("{status}{words}{timed_out}")
    }
}

/// A failure, with what the turn's result and exit add to its words.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}", .seen.around(&self.failure))]
pub(super) struct Failed {
    pub(super) failure: Failure,
    pub(super) seen: Seen,
}

/// How a turn ended, read from its stream and exit: the answer it gave, a
/// refusal, or a failure.
#[derive(Debug, PartialEq)]
pub(super) enum Verdict {
    Answer(String),
    Refused(PreSessionRefusal),
    Failed(Failed),
}

/// What the turn's init said of the model agy served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Served {
    /// No init row has arrived.
    Awaited,
    /// The init named no model.
    Unnamed,
    /// The init named the pin.
    Pinned,
}

/// The first fault a turn showed, which no later row changes: a failure,
/// or the refusal the harness was stopped at.
enum Fault {
    Failed(Failure),
    Refused(PreSessionRefusal),
}

/// Agy's rows read for the model they served and the turn's end.
pub(super) struct AgyFold<'a> {
    pin: String,
    /// What every word of agy's the driver names is masked through first.
    bindings: &'a [secret::BoundSecret],
    served: Served,
    /// The model responses folded as turns.
    turns: u64,
    /// Whether any known row arrived.
    rows: bool,
    /// Whether the first typed result's error said agy is signed out.
    signed_out: bool,
    /// The first typed result's status, [`named`], fault or none (L5).
    status: Option<String>,
    ending: Option<Ending>,
    fault: Option<Fault>,
}

impl<'a> AgyFold<'a> {
    pub(super) fn new(pin: &str, bindings: &'a [secret::BoundSecret]) -> AgyFold<'a> {
        AgyFold {
            pin: pin.to_string(),
            bindings,
            served: Served::Awaited,
            turns: 0,
            rows: false,
            signed_out: false,
            status: None,
            ending: None,
            fault: None,
        }
    }

    /// Whether a model response has been folded as a turn of the seat's.
    pub(super) fn worked(&self) -> bool {
        self.turns > 0
    }

    /// Whether init named the pin.
    fn pinned(&self) -> bool {
        self.served == Served::Pinned
    }

    /// Whether a result said agy is signed out, read as agy wrote it.
    pub(super) fn signed_out(&self) -> bool {
        self.signed_out
    }

    /// `text`, a word of agy's, as the driver's words name it.
    fn named(&self, text: &str) -> String {
        named(text, self.bindings)
    }

    /// One line of agy's stdout, as its bytes. `Err` is the seat's refusal,
    /// which stops the harness: init named another model than the pin, or a
    /// step arrived before init named one. A line that is not UTF-8, a
    /// known row out of its turn's order and one that does not type are the
    /// turn's fault; once one is seen, the first typed result still gives
    /// its status and its signed-out words, and nothing else is read.
    pub(super) fn read(
        &mut self,
        line: &[u8],
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) -> Result<(), PreSessionRefusal> {
        let Ok(text) = std::str::from_utf8(line) else {
            self.fault.get_or_insert(Fault::Failed(Failure::NotText));
            return Ok(());
        };
        let Ok(Tags {
            known: Some(event),
            named,
        }) = serde_json::from_str::<Tags>(text)
        else {
            return Ok(());
        };
        self.rows = true;
        let Some(row) = event.typed(text).filter(|_| named == 1) else {
            self.fault
                .get_or_insert(Fault::Failed(Failure::Unread(event)));
            return Ok(());
        };
        if let (Row::Result(result), None) = (&row, &self.status) {
            self.signed_out = result.signed_out();
            self.status = Some(self.named(result.status.word()));
        }
        match (&self.fault, &self.ending) {
            (Some(_), _) => Ok(()),
            (None, Some(_)) => {
                self.fault = Some(Fault::Failed(Failure::Behind(event)));
                Ok(())
            }
            (None, None) => self.fold(row, session_meta, emit),
        }
    }

    /// A row in its turn's order.
    fn fold(
        &mut self,
        row: Row,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) -> Result<(), PreSessionRefusal> {
        match row {
            Row::Init(_) if self.served != Served::Awaited => {
                self.fault = Some(Fault::Failed(Failure::Reinitialized))
            }
            Row::Init(init) => return self.initialized(init, session_meta),
            Row::Step(_) if !self.pinned() => {
                let refused = unconfirmed(&self.pin, self.bindings, &Seen::default());
                return Err(self.stopped(refused));
            }
            Row::Step(step) => self.step(&step, emit),
            Row::Result(result) => self.ended(result, session_meta),
        }
        Ok(())
    }

    /// `refused` as the fault the harness is stopped at.
    fn stopped(&mut self, refused: PreSessionRefusal) -> PreSessionRefusal {
        self.fault = Some(Fault::Refused(refused.clone()));
        refused
    }

    /// The one init: the pin confirmed, another model refused, and an init
    /// that names none confirming nothing.
    fn initialized(
        &mut self,
        init: Init,
        session_meta: &mut Map<String, Value>,
    ) -> Result<(), PreSessionRefusal> {
        self.served = Served::Unnamed;
        match init.model {
            Some(model) if model == self.pin => {
                self.served = Served::Pinned;
                session_meta.insert("model".into(), Value::String(model));
            }
            Some(model) => {
                let refused = substituted(&self.pin, &self.named(&model), self.bindings);
                return Err(self.stopped(refused));
            }
            None => {}
        }
        Ok(())
    }

    /// A model response that is done is one turn, journaled on the pin.
    fn step(&mut self, step: &Step, emit: &mut impl FnMut(&Value)) {
        if (&step.step_type, &step.state) != (&StepType::AgentResponse, &StepState::Done) {
            return;
        }
        self.turns += 1;
        let mut row = Map::new();
        row.insert("step".into(), Value::String("seat-turn".into()));
        row.insert("turn".into(), Value::from(self.turns));
        row.insert("model".into(), Value::String(self.pin.clone()));
        row.insert("effort".into(), Value::String(EFFORT_NOT_REPORTED.into()));
        emit(&Value::Object(row));
    }

    /// The one result: its usage folded, and a denial the turn's fault.
    fn ended(&mut self, result: Ending, session_meta: &mut Map<String, Value>) {
        settled(&result, session_meta);
        if !result.denied_actions.is_empty() {
            let actions = result.denied_actions.iter();
            let named = actions.map(|denied| self.named(&denied.action)).collect();
            self.fault = Some(Fault::Failed(Failure::Denied(named)));
        }
        self.ending = Some(result);
    }

    /// A stdout read to its end where `unread` is false; otherwise the
    /// turn's fault, behind any seen earlier (L3).
    pub(super) fn unreadable(&mut self, unread: bool) {
        if unread {
            self.fault.get_or_insert(Fault::Failed(Failure::Unreadable));
        }
    }

    /// The turn's verdict once agy exited `exit_code`, `signed_out_on_stderr`
    /// where its stderr held the signed-out words: the first that applies
    /// (K2). A fault; agy signed out; its print timeout; any other non-zero
    /// exit; a status that is not `SUCCESS`; no result; a pin no init
    /// confirmed; an empty response; and only then the answer. A refusal
    /// the driver stopped agy at reads no exit of agy's (L4).
    pub(super) fn verdict(&self, exit_code: i32, signed_out_on_stderr: bool) -> Verdict {
        let status = self.status.clone();
        if let Some(fault) = &self.fault {
            let timed_out = exit_code == 3;
            return match fault {
                Fault::Failed(failure) => failed(failure.clone(), Seen { status, timed_out }),
                Fault::Refused(refused) => Verdict::Refused(refused.clone()),
            };
        }
        let seen = Seen {
            status,
            timed_out: false,
        };
        if self.signed_out || signed_out_on_stderr {
            return Verdict::Refused(signed_out(&seen));
        }
        match exit_code {
            0 => {}
            3 => return Verdict::Refused(print_timeout(&seen)),
            code => return self.exited(code, seen),
        }
        let Some(ending) = &self.ending else {
            return self.unended();
        };
        match &ending.status {
            Status::Other(_) => failed(Failure::Unsuccessful, seen),
            Status::Success if !self.pinned() => {
                Verdict::Refused(unconfirmed(&self.pin, self.bindings, &seen))
            }
            Status::Success if ending.response.trim().is_empty() => failed(Failure::Empty, seen),
            Status::Success => Verdict::Answer(ending.response.clone()),
        }
    }

    /// The verdict of a turn agy exited `code` from, neither 0 nor 3: before
    /// a confirmed init, exit 1 is a launch refusal, and exit 2 with no
    /// known row a refused option; otherwise a failure naming the exit.
    fn exited(&self, code: i32, seen: Seen) -> Verdict {
        match code {
            1 if !self.pinned() => Verdict::Refused(launch_refused(
                "agy exited 1 before any turn: it does not serve the model or the effort the seat \
                 named",
                &seen,
            )),
            2 if !self.rows => Verdict::Refused(launch_refused(
                "agy exited 2 on an option it does not define",
                &seen,
            )),
            _ => failed(Failure::Exited(code), seen),
        }
    }

    /// The verdict of a turn that exited 0 with no result.
    fn unended(&self) -> Verdict {
        match self.pinned() {
            false => Verdict::Refused(unconfirmed(&self.pin, self.bindings, &Seen::default())),
            true => failed(Failure::Unended, Seen::default()),
        }
    }
}

fn failed(failure: Failure, seen: Seen) -> Verdict {
    Verdict::Failed(Failed { failure, seen })
}

/// A result's usage and turns folded into the seat's record, each figure
/// the result reports above zero, the rest not reported.
fn settled(result: &Ending, session_meta: &mut Map<String, Value>) {
    let usage = &result.usage;
    let input = usage.input_tokens.saturating_add(usage.cache_read_tokens);
    let figures = [
        ("input_tokens", input),
        ("output_tokens", usage.output_tokens),
        ("reasoning_output_tokens", usage.thinking_tokens),
        ("cache_read_tokens", usage.cache_read_tokens),
    ];
    for (key, value) in figures.into_iter().filter(|(_, value)| *value > 0) {
        add_usage(session_meta, key, value);
    }
    if let Some(turns) = result.num_turns.filter(|turns| *turns > 0) {
        session_meta.insert("num_turns".into(), Value::from(turns));
    }
}

/// `text` masked whole through `bindings`, as written, and then escaped:
/// the order every value the driver's words hold is built in, before any
/// bound reads them, so no cut falls inside a value the mask has not
/// replaced (#1200 Z1). Only the first `limit` escaped characters are
/// built.
fn masked(text: &str, bindings: &[secret::BoundSecret], limit: usize) -> String {
    let masked = secret::mask_text(text.as_bytes(), bindings);
    masked.escape_debug().take(limit).collect()
}

/// `text`, a word of agy's, as the driver's words name it (K3): [`masked`],
/// and only then cut to its first [`NAMED`] characters. The one home of
/// such a word: every status, denied action and served model the driver
/// names is built here.
pub(super) fn named(text: &str, bindings: &[secret::BoundSecret]) -> String {
    masked(text, bindings, NAMED)
}

/// The refusal of a seat agy served another model than its `pin`, the pin
/// [`masked`] whole through `bindings` and the served model [`named`].
fn substituted(pin: &str, served: &str, bindings: &[secret::BoundSecret]) -> PreSessionRefusal {
    let pin = masked(pin, bindings, usize::MAX);
    let words = format!("the seat pinned \"{pin}\" and agy served \"{served}\"");
    refusal_reason("model_substituted", None, Some(&words))
}

/// A refusal under `token` in `words`, with what the turn's result added,
/// its status first, so the bound on a refusal's words never cuts it (L5).
fn refused(token: &str, words: &str, seen: &Seen) -> PreSessionRefusal {
    refusal_reason(token, None, Some(&seen.around(words)))
}

/// The refusal of a seat no init row confirmed the model of. This and the
/// three builders below are each a refusal built on what the result showed,
/// and the driver's tests call each with a long status (M5). The `pin` is
/// [`masked`] whole through `bindings` before the bound reads it (#1200 Z1).
pub(super) fn unconfirmed(
    pin: &str,
    bindings: &[secret::BoundSecret],
    seen: &Seen,
) -> PreSessionRefusal {
    let pin = masked(pin, bindings, usize::MAX);
    let words = format!("the seat pinned \"{pin}\" and no init row named the model agy served");
    refused("model_unconfirmed", &words, seen)
}

pub(super) fn signed_out(seen: &Seen) -> PreSessionRefusal {
    refused(
        "not_logged_in",
        "Antigravity CLI is signed out: run `agy` once and sign in; the login is \
         ~/.gemini/antigravity-cli/antigravity-oauth-token",
        seen,
    )
}

pub(super) fn launch_refused(words: &str, seen: &Seen) -> PreSessionRefusal {
    refused("launch_refused", words, seen)
}

pub(super) fn print_timeout(seen: &Seen) -> PreSessionRefusal {
    refused(
        "print_timeout",
        "agy exited 3 at its own print timeout, which this launch never asks for",
        seen,
    )
}
