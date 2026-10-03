//! dsh's typed reader (#484). Its headless profile prints the final
//! message as text, so its events are its session log's, which the
//! adapter's driver folds (`fold_dsh_event`): the `session` header and
//! `assistant/message`, decoded closed by `type`. No dsh turn that reached
//! a provider has been recorded (the operator ruled out keyed billing on
//! 2026-10-03), so the events are the ones the driver's fold reads, and
//! any other is unread until one is. Its one recorded line is commander's
//! refusal of `--model`. A key is consumed, or inert, as in `claude`.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::forms::{self, Form, Says, UNKNOWN_OPTION};
use super::read::{decode, Class, Counted, Decoded, Given, Harness, Said};
use super::Fault;

/// What dsh prints that the probe reads whole.
const FORMS: [Form; 2] = [
    UNKNOWN_OPTION,
    // Its refusal of a launch with no credential.
    Form {
        words: "error {credential} is not set",
        says: Says::Refuses(Class::Auth),
    },
];

#[derive(Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
enum Event {
    #[serde(rename = "session")]
    Session {
        id: String,
        /// Inert: the log's format.
        #[serde(rename = "version")]
        _version: u64,
        /// Inert: the scratch repository the turn runs in.
        #[serde(default, rename = "cwd")]
        _cwd: Given<String>,
        /// Inert: how deep a delegation the session is.
        #[serde(default, rename = "delegationDepth")]
        _delegation_depth: Given<u64>,
    },
    #[serde(rename = "assistant/message")]
    AssistantMessage {
        data: Step,
        /// Inert: the event's place in the log.
        #[serde(default, rename = "seq")]
        _seq: Given<u64>,
    },
}

/// One assembled assistant step. Every key may be absent, so the struct
/// defaults each one.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct Step {
    usage: Given<Usage>,
    /// Inert: dsh's own turn, which never leaves 1 for one task.
    #[serde(rename = "turn")]
    _turn: Given<u64>,
    /// Inert: the step's place in the turn.
    #[serde(rename = "step")]
    _step: Given<u64>,
    /// Inert: which model served the step.
    #[serde(rename = "message")]
    _message: Given<StepMessage>,
}

/// Inert, every key: which model served the step.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StepMessage {
    #[serde(rename = "source")]
    _source: Source,
}

/// Inert, every key: as `StepMessage`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    #[serde(rename = "model")]
    _model: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_tokens: Given<u64>,
    #[serde(default)]
    reasoning_tokens: Given<u64>,
    #[serde(default)]
    total_tokens: Given<u64>,
}

/// One dsh event, decoded, or why it is unread.
pub(super) fn event(fields: Map<String, Value>) -> Result<Decoded, Fault> {
    let (label, said) = match decode(Harness::Dsh, fields)? {
        Event::Session { id, .. } => ("session", vec![Said::Session { key: "id", id }]),
        Event::AssistantMessage { data, .. } => {
            let usage = data.usage.0.map(|usage| {
                Counted::of(
                    "/data/usage",
                    None,
                    [
                        ("inputTokens", Some(usage.input_tokens)),
                        ("outputTokens", Some(usage.output_tokens)),
                        ("cacheReadTokens", usage.cache_read_tokens.0),
                        ("reasoningTokens", usage.reasoning_tokens.0),
                        ("totalTokens", usage.total_tokens.0),
                    ],
                )
            });
            ("assistant/message", usage.into_iter().collect())
        }
    };
    Ok(Decoded {
        label: label.to_string(),
        said,
    })
}

/// One line of dsh's text, its stdout's reply among them, read whole
/// against its forms.
pub(super) fn text(line: &str) -> Option<Vec<Said>> {
    forms::read(line, &FORMS)
}
