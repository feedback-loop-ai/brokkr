//! Why a `tools/call`'s params were refused (decision 0043).
//!
//! `checked_call` refuses in this one cause, one variant per check, so a
//! test can name the check that fired. Its `Display` is the operator's
//! text byte for byte; `call_arguments` is the one place it becomes text
//! (#353).

use super::{MAX_TIMEOUT_MS, TOOL_NAME};

/// Why a call's params were refused, each field borrowed from the params.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum CallRefusal<'a> {
    /// The call's `name` is absent, not a string, or not the tool's.
    #[error("unknown tool; this server serves only '{TOOL_NAME}'")]
    UnknownTool,
    /// The call's arguments name a key outside `command` and `timeoutMs`.
    #[error("{TOOL_NAME}: unknown argument '{key}'")]
    UnknownArgument { key: &'a str },
    /// The `command` is absent, not a string, blank, or over 16,384 bytes.
    #[error("{TOOL_NAME}: command must contain 1-16384 characters")]
    Command,
    /// The `timeoutMs` is present and not an integer in its range.
    #[error("{TOOL_NAME}: timeoutMs must be an integer from 100 through {MAX_TIMEOUT_MS}")]
    Timeout,
}

#[cfg(test)]
mod tests;
