//! Agy's stream folded (#1200, G9). The stream is agy's own shape, not
//! Claude Code's (the record's Facts): one `init`, `step_update` rows, and
//! one `result` per turn. Each row is typed here, at the edge, and nothing
//! of it is carried past this module as JSON.
//!
//! - The served model is init's `model`, which repeats the slug as asked; a
//!   seat whose init names another is refused before any row counts.
//! - A turn agy stopped at a denial is a success by every outer sign: the
//!   only sign is the result's `denied_actions`. A denied step's own
//!   `state` is not reliable (`ERROR` in `denied-command-default`, `DONE`
//!   in `denied-command-accept-edits`), so it is never read for one.
//! - The answer is a `SUCCESS` result's non-empty `response`.
//! - The result's `usage` is the invocation's own, in tokens with no price
//!   and no cache-write count: one figure per invocation, from the result.
//!   A cache read is not part of `input_tokens` (`tools-turn`: the second
//!   step's 2,289 input beside 6,851 read), so the journal's inclusive
//!   input adds it back, as the Claude fold does; thinking rides as
//!   reasoning output.
//! - A `tool` step is passed over: the checkpoint's tool observation has no
//!   format for agy's telemetry without a contract change.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::super::refusal::refusal_reason;
use super::super::{add_usage, EFFORT_NOT_REPORTED};
use crate::PreSessionRefusal;

/// The words a result's `error` holds when agy is signed out (the record's
/// `signed-out` turn). Only matched, on the row as agy wrote it.
const SIGNED_OUT: &str = "authentication failed or timed out";

/// One row of agy's stream, as far as the fold reads it.
#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum Row {
    Init {
        init: Init,
    },
    StepUpdate {
        step_update: Step,
    },
    Result {
        result: Ending,
    },
    #[serde(other)]
    Other,
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

/// A turn's `result`.
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
}

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

/// How a turn ended, read from its stream and exit: an answer, a refusal,
/// a failure in the driver's own words, or an exit that says it all.
#[derive(Debug, PartialEq)]
pub(super) enum Verdict {
    Answer,
    Refused(PreSessionRefusal),
    Failed(String),
    Exited,
}

/// Agy's rows read for the model they served and the turn's end.
pub(super) struct AgyFold {
    pin: String,
    /// Whether init named the pin.
    served: bool,
    /// The model responses folded as turns.
    turns: u64,
    /// Whether any row arrived.
    rows: bool,
    /// Whether a result's error said agy is signed out, read unmasked.
    signed_out: bool,
    ending: Option<Ending>,
}

impl AgyFold {
    pub(super) fn new(pin: &str) -> AgyFold {
        AgyFold {
            pin: pin.to_string(),
            served: false,
            turns: 0,
            rows: false,
            signed_out: false,
            ending: None,
        }
    }

    /// Whether a model response has been folded as a turn of the seat's.
    pub(super) fn worked(&self) -> bool {
        self.turns > 0
    }

    /// `event` as agy wrote it, before the seat's bindings mask it, read
    /// for the signed-out words alone, so no binding can erase them (#1178
    /// U2). No byte of it is kept.
    pub(super) fn observe(&mut self, event: &Value) {
        let error = event.pointer("/result/error").and_then(Value::as_str);
        self.signed_out |= error.is_some_and(|error| error.contains(SIGNED_OUT));
    }

    /// One masked row. `Err` is the seat's refusal: init named another
    /// model than the pin, or a step arrived before init named one. A row
    /// that is not one the fold types is passed over.
    pub(super) fn read(
        &mut self,
        event: Value,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) -> Result<(), PreSessionRefusal> {
        self.rows = true;
        match serde_json::from_value(event).unwrap_or(Row::Other) {
            Row::Init { init } => match init.model {
                Some(model) if model == self.pin => {
                    self.served = true;
                    session_meta.insert("model".into(), Value::String(model));
                }
                Some(model) => return Err(substituted(&self.pin, &model)),
                None => {}
            },
            Row::StepUpdate { .. } if !self.served => return Err(unconfirmed(&self.pin)),
            Row::StepUpdate { step_update } => self.step(&step_update, emit),
            Row::Result { result } => {
                settled(&result, session_meta);
                self.ending = Some(result);
            }
            Row::Other => {}
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

    /// The turn's verdict once agy exited `exit_code`.
    pub(super) fn verdict(&self, exit_code: i32) -> Verdict {
        let Some(ending) = &self.ending else {
            return self.unended(exit_code);
        };
        if !ending.denied_actions.is_empty() {
            return Verdict::Failed(denied(&ending.denied_actions));
        }
        match &ending.status {
            Status::Success if !self.served => Verdict::Refused(unconfirmed(&self.pin)),
            Status::Success if !ending.response.trim().is_empty() => Verdict::Answer,
            _ if self.signed_out => Verdict::Refused(signed_out()),
            _ if exit_code == 1 && !self.served => Verdict::Refused(launch_refused(
                "agy exited 1 before any turn: it does not serve the model or the effort the seat \
                 named",
            )),
            _ if exit_code == 3 => Verdict::Refused(print_timeout()),
            // A non-zero exit is the CLI's own failure, and says so.
            _ if exit_code != 0 => Verdict::Exited,
            Status::Success => Verdict::Failed(
                "agy's turn ended SUCCESS with an empty response, which is not an answer (#1200)"
                    .to_string(),
            ),
            Status::Other(status) => Verdict::Failed(format!(
                "agy's turn ended with status {}, which is not an answer (#1200)",
                named(status)
            )),
        }
    }

    /// The verdict of a turn whose stream held no result.
    fn unended(&self, exit_code: i32) -> Verdict {
        match exit_code {
            2 if !self.rows => Verdict::Refused(launch_refused(
                "agy exited 2 on an option it does not define",
            )),
            3 => Verdict::Refused(print_timeout()),
            0 if !self.served => Verdict::Refused(unconfirmed(&self.pin)),
            0 => Verdict::Failed(
                "agy's turn ended with no result, so it gave no answer (#1200)".to_string(),
            ),
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

/// `text` where it is a plain identifier of agy's, else a word of the
/// driver's own: no other byte of agy's reaches the driver's words.
fn named(text: &str) -> &str {
    let plain = !text.is_empty()
        && text.len() <= 40
        && (text.bytes()).all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte));
    if plain {
        text
    } else {
        "<unnamed>"
    }
}

/// The failure of a turn agy stopped at a denial, naming each action.
fn denied(actions: &[Denied]) -> String {
    let named: Vec<&str> = actions.iter().map(|denied| named(&denied.action)).collect();
    format!(
        "agy stopped the turn at a denial of {}: a turn agy stopped at a denial is not an answer \
         (#1200)",
        named.join(", ")
    )
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
