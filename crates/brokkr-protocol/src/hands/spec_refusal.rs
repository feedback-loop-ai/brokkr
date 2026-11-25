//! Why a site's `hands` spec was refused (decision 0043; decision 0046
//! ruling 1).
//!
//! `HandsSpec::checked` refuses in this one cause, one variant per check,
//! so a test can name the check that fired. A bind entry's own refusal
//! travels inside it typed. Its `Display` is the operator's text byte for
//! byte; `HandsSpec::parse` is the one place it becomes text (#353).

use super::bind_refusal::BindRefusal;
use super::BOUNDARY_IS_THE_REALMS;

/// Why a `hands` value was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpecRefusal {
    /// The value is a string other than `workspace`.
    #[error("hands '{kind}' is not a known kind; the vocabulary is: workspace")]
    UnknownKind { kind: String },
    /// The value is neither a string nor an object.
    #[error("hands must be \"workspace\" or an object, got {got}")]
    NotKindOrObject { got: String },
    /// The object names `boundary`, which only a realm declares.
    #[error("hands names 'boundary'; {BOUNDARY_IS_THE_REALMS}")]
    Boundary,
    /// The object names a key outside `kind`, `network` and `binds`.
    #[error("hands has unknown key '{key}'; known: kind, network, binds")]
    UnknownKey { key: String },
    /// The object's `kind` is absent, not a string, or not `workspace`.
    #[error("hands.kind must be \"workspace\"")]
    Kind,
    /// The object's `network` is present and not a boolean.
    #[error("hands.network must be a boolean, got {got}")]
    Network { got: String },
    /// The object's `binds` is present and not an array.
    #[error("hands.binds must be an array")]
    BindsNotArray,
    /// A `binds` entry refused in its own cause.
    #[error(transparent)]
    Bind(BindRefusal),
}

#[cfg(test)]
mod tests;
