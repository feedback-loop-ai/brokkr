//! Agy's stream folded (#1200, G9). The stream is agy's own shape, not
//! Claude Code's (the record's Facts): one `init`, `step_update` rows, and
//! one `result` per turn. Each row is typed here, at the edge, as agy wrote
//! it and before any mask, and nothing of it is carried past this module
//! as JSON: the pin, the status, the denial, the response and the
//! signed-out words are each read from that one typed row (J2).
//!
//! - One launch is one turn (J3): a second `init`, a second `result`, a
//!   known row behind the `result` and a known row that does not type each
//!   fail the turn. An `event` the fold does not know is passed over, and
//!   so is a line that is no object naming one. Once a denial or a failure
//!   is seen, no later row changes the verdict.
//! - The served model is init's `model`, which repeats the slug as asked; a
//!   seat whose init names another is refused before any row counts.
//! - A turn agy stopped at a denial is a success by every outer sign: the
//!   only sign is the result's `denied_actions`. A denied step's own
//!   `state` is not reliable (`ERROR` in `denied-command-default`, `DONE`
//!   in `denied-command-accept-edits`), so it is never read for one.
//! - Exit 3 is agy's print timeout, whatever rows came before it
//!   (`print-timeout`, whose response step had finished).
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

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::super::refusal::refusal_reason;
use super::super::{add_usage, EFFORT_NOT_REPORTED};
use crate::PreSessionRefusal;

/// The words a result's `error` holds when agy is signed out (the record's
/// `signed-out` turn). Only matched, on the row as agy wrote it.
const SIGNED_OUT: &str = "authentication failed or timed out";

/// The most characters of a word of agy's the driver's words name.
const NAMED: usize = 40;

/// A row's `event`, read before the row is: one the fold knows, or any
/// other value, which is passed over.
#[derive(Deserialize)]
struct Envelope {
    event: Named,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Named {
    Known(Event),
    Other(IgnoredAny),
}

/// The three rows of a turn.
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
}

/// One known row of agy's stream, as far as the fold reads it.
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Row {
    Init { init: Init },
    StepUpdate { step_update: Step },
    Result { result: Ending },
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
/// agy's but a plain word it names, bounded.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Failure {
    #[error(
        "agy stopped the turn at a denial of {}: a turn agy stopped at a denial is not an answer \
         (#1200)",
        .0.join(", ")
    )]
    Denied(Vec<String>),
    #[error("agy's turn ended SUCCESS with an empty response, which is not an answer (#1200)")]
    Empty,
    #[error("agy's turn ended with status {0}, which is not an answer (#1200)")]
    Status(String),
    #[error("agy's turn ended with no result, so it gave no answer (#1200)")]
    Unended,
    #[error("agy wrote a second init row, and one launch is one turn (#1200)")]
    Reinitialized,
    #[error("agy wrote {} behind the turn's result, and one launch is one turn (#1200)", .0.row())]
    Behind(Event),
    #[error("agy wrote {} the driver cannot read, so the turn gave no answer (#1200)", .0.row())]
    Unread(Event),
}

/// How a turn ended, read from its stream and exit: the answer it gave, a
/// refusal, a failure, or an exit that says it all.
#[derive(Debug, PartialEq)]
pub(super) enum Verdict {
    Answer(String),
    Refused(PreSessionRefusal),
    Failed(Failure),
    Exited,
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

/// Agy's rows read for the model they served and the turn's end.
pub(super) struct AgyFold {
    pin: String,
    served: Served,
    /// The model responses folded as turns.
    turns: u64,
    /// Whether any row arrived.
    rows: bool,
    /// Whether a result's error said agy is signed out.
    signed_out: bool,
    ending: Option<Ending>,
    /// The first failure seen, which no later row changes.
    fault: Option<Failure>,
}

impl AgyFold {
    pub(super) fn new(pin: &str) -> AgyFold {
        AgyFold {
            pin: pin.to_string(),
            served: Served::Awaited,
            turns: 0,
            rows: false,
            signed_out: false,
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

    /// One row as agy wrote it, typed once. `Err` is the seat's refusal:
    /// init named another model than the pin, or a step arrived before
    /// init named one. A known row out of its turn's order, or one that
    /// does not type, is the turn's failure; once one is seen, rows are
    /// only read for the signed-out words.
    pub(super) fn read(
        &mut self,
        event: &Value,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) -> Result<(), PreSessionRefusal> {
        self.rows = true;
        let Ok(Envelope {
            event: Named::Known(known),
        }) = Envelope::deserialize(event)
        else {
            return Ok(());
        };
        let Ok(row) = Row::deserialize(event) else {
            self.fault.get_or_insert(Failure::Unread(known));
            return Ok(());
        };
        if let Row::Result { result } = &row {
            self.signed_out |= result.signed_out();
        }
        match (&self.fault, &self.ending) {
            (Some(_), _) => Ok(()),
            (None, Some(_)) => {
                self.fault = Some(Failure::Behind(known));
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
            Row::Init { .. } if self.served != Served::Awaited => {
                self.fault = Some(Failure::Reinitialized)
            }
            Row::Init { init } => return self.initialized(init, session_meta),
            Row::StepUpdate { .. } if !self.pinned() => return Err(unconfirmed(&self.pin)),
            Row::StepUpdate { step_update } => self.step(&step_update, emit),
            Row::Result { result } => self.ended(result, session_meta),
        }
        Ok(())
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
            Some(model) => return Err(substituted(&self.pin, &model)),
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

    /// The one result: its usage folded, and a denial the turn's failure.
    fn ended(&mut self, result: Ending, session_meta: &mut Map<String, Value>) {
        settled(&result, session_meta);
        if !result.denied_actions.is_empty() {
            let actions = result.denied_actions.iter();
            self.fault = Some(Failure::Denied(actions.map(|d| named(&d.action)).collect()));
        }
        self.ending = Some(result);
    }

    /// The turn's verdict once agy exited `exit_code`.
    pub(super) fn verdict(&self, exit_code: i32) -> Verdict {
        if let Some(failure) = &self.fault {
            return Verdict::Failed(failure.clone());
        }
        if exit_code == 3 {
            return Verdict::Refused(print_timeout());
        }
        let Some(ending) = &self.ending else {
            return self.unended(exit_code);
        };
        match &ending.status {
            Status::Success if !self.pinned() => Verdict::Refused(unconfirmed(&self.pin)),
            Status::Success if !ending.response.trim().is_empty() => {
                Verdict::Answer(ending.response.clone())
            }
            _ if self.signed_out => Verdict::Refused(signed_out()),
            _ if exit_code == 1 && !self.pinned() => Verdict::Refused(launch_refused(
                "agy exited 1 before any turn: it does not serve the model or the effort the seat \
                 named",
            )),
            // A non-zero exit is the CLI's own failure, and says so.
            _ if exit_code != 0 => Verdict::Exited,
            Status::Success => Verdict::Failed(Failure::Empty),
            Status::Other(status) => Verdict::Failed(Failure::Status(named(status))),
        }
    }

    /// The verdict of a turn whose stream held no result.
    fn unended(&self, exit_code: i32) -> Verdict {
        match exit_code {
            2 if !self.rows => Verdict::Refused(launch_refused(
                "agy exited 2 on an option it does not define",
            )),
            0 if !self.pinned() => Verdict::Refused(unconfirmed(&self.pin)),
            0 => Verdict::Failed(Failure::Unended),
            _ => Verdict::Exited,
        }
    }
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

/// `text` where it is a plain word of agy's, its first [`NAMED`]
/// characters, else a word of the driver's own: no other byte of agy's
/// reaches the driver's words.
fn named(text: &str) -> String {
    let plain = !text.is_empty()
        && (text.bytes()).all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte));
    match plain {
        true => text.chars().take(NAMED).collect(),
        false => "<unnamed>".to_string(),
    }
}

/// The refusal of a seat agy served another model than its pin.
fn substituted(pin: &str, served: &str) -> PreSessionRefusal {
    let words = format!("the seat pinned {pin:?} and agy served {served:?}");
    refusal_reason("model_substituted", None, Some(&words))
}

/// The refusal of a seat no init row confirmed the model of.
fn unconfirmed(pin: &str) -> PreSessionRefusal {
    let words = format!("the seat pinned {pin:?} and no init row named the model agy served");
    refusal_reason("model_unconfirmed", None, Some(&words))
}

fn signed_out() -> PreSessionRefusal {
    refusal_reason(
        "not_logged_in",
        None,
        Some(
            "Antigravity CLI is signed out: run `agy` once and sign in; the login is \
             ~/.gemini/antigravity-cli/antigravity-oauth-token",
        ),
    )
}

fn launch_refused(words: &str) -> PreSessionRefusal {
    refusal_reason("launch_refused", None, Some(words))
}

fn print_timeout() -> PreSessionRefusal {
    refusal_reason(
        "print_timeout",
        None,
        Some("agy exited 3 at its own print timeout, which this launch never asks for"),
    )
}
