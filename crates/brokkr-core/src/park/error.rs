//! Why a present `cause` object does not read: typed, so a reader and a test
//! match the fault and never its words (decision 0071 ruling 8). Each
//! `Display` keeps the operator's text, which opens with the same sentence
//! so a readout names the refusal.

use thiserror::Error;

/// Why a present `cause` object does not read.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParkCauseError {
    /// The value is not an object.
    #[error("the park cause does not read: it is not an object")]
    NotAnObject,
    /// The `kind` is absent, not a string, or names no cause; the text is
    /// what was there.
    #[error("the park cause does not read: {}", describe_kind(.0))]
    UnknownKind(Option<String>),
    /// A field the kind requires is absent.
    #[error("the park cause does not read: a {kind} cause requires `{field}`")]
    Missing {
        kind: &'static str,
        field: &'static str,
    },
    /// A field the kind forbids is present.
    #[error("the park cause does not read: a {kind} cause forbids `{field}`")]
    Forbidden { kind: &'static str, field: String },
    /// A field is present and is not the type the schema states.
    #[error("the park cause does not read: `{field}` of a {kind} cause is not {expected}")]
    WrongType {
        kind: &'static str,
        field: &'static str,
        expected: &'static str,
    },
    /// A `failure_class` outside the closed four.
    #[error("the park cause does not read: failure class '{0}' is none of the four")]
    UnknownFailureClass(String),
    /// A refusal `kind` that is absent, not a string, or names no 0103 cause.
    #[error("the park cause does not read: refusal {}", describe_kind(.0))]
    UnknownRefusal(Option<String>),
    /// An `until` that is not an RFC 3339 instant.
    #[error("the park cause does not read: `until` '{0}' is not an RFC 3339 time")]
    Until(String),
    /// A field the schema's forms refuse.
    #[error("the park cause does not read: {0}")]
    Form(#[from] FormError),
}

fn describe_kind(kind: &Option<String>) -> String {
    match kind {
        Some(kind) => format!("kind '{kind}' names none of the closed words"),
        None => "kind is absent or not a string".to_string(),
    }
}

/// A form `contracts/park-cause.v1.schema.json` states and a value breaks.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FormError {
    #[error("detail is empty")]
    EmptyDetail,
    #[error("rule_id is {0} characters, not 1 to {max}", max = super::RULE_ID_MAX)]
    RuleIdLength(usize),
    #[error("provider '{0}' is not a provider name")]
    Provider(String),
    #[error("route '{0}' is not a route name")]
    Route(String),
    #[error("{0} is 0, not at least 1")]
    Seconds(&'static str),
}
