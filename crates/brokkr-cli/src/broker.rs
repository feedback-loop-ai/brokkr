//! `brokkr broker serve` (decision 0077, design D5): the broker the
//! harness starts beside `brokkr hands`, one per held MCP capability, from
//! the configuration the engine writes. Its authority is the engine's plan
//! bound to the attempt and nothing a caller names: the command line holds
//! a plan locator and its digest, never a server argv, a grant or a secret
//! value.
//!
//! The plan itself, its binding and the serving protections are later
//! units' (slice two U6c–U6f), and decision 0065's compile fence still
//! refuses every MCP grant, so no plan is bound to any attempt yet and
//! every invocation refuses before anything is read or started.

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use crate::cli_args::{BrokerCmd, BrokerServeArgs};

/// The longest plan locator taken, in bytes: macOS's `PATH_MAX`, the
/// smaller of the two supported hosts' (decision 0063).
const PLAN_LOCATOR_MAX: usize = 1024;

/// Why the broker takes no plan: a locator or digest refused as the
/// command line is parsed, or a well-formed one that is not bound.
#[derive(Debug)]
pub(crate) enum BrokerError {
    Relative,
    Parent,
    TooLong,
    Digest,
    PlanUnbound,
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Relative => formatter.write_str("a plan locator is an absolute path"),
            BrokerError::Parent => formatter.write_str("a plan locator names no parent directory"),
            BrokerError::TooLong => {
                write!(
                    formatter,
                    "a plan locator is at most {PLAN_LOCATOR_MAX} bytes"
                )
            }
            BrokerError::Digest => {
                formatter.write_str("a plan digest is 64 lowercase hex characters")
            }
            BrokerError::PlanUnbound => {
                formatter.write_str("broker plan is not bound to this attempt")
            }
        }
    }
}

impl std::error::Error for BrokerError {}

/// `--plan`: an absolute, lexically normal path of bounded length. Its
/// shape is all this checks; whether the engine wrote it for this attempt
/// is the binding's to decide.
pub(crate) fn plan_locator(text: &str) -> Result<PathBuf, BrokerError> {
    let path = Path::new(text);
    if text.len() > PLAN_LOCATOR_MAX {
        Err(BrokerError::TooLong)
    } else if !path.is_absolute() {
        Err(BrokerError::Relative)
    } else if path.components().any(|part| part == Component::ParentDir) {
        Err(BrokerError::Parent)
    } else {
        Ok(path.to_path_buf())
    }
}

/// `--plan-digest`: a sha256 in the one spelling every reader takes.
pub(crate) fn plan_digest(text: &str) -> Result<String, BrokerError> {
    match brokkr_core::canonical::is_sha256_hex(text) {
        true => Ok(text.to_string()),
        false => Err(BrokerError::Digest),
    }
}

/// Serve the plan `command` names, or refuse before reading it or
/// starting anything.
pub(crate) fn run(command: BrokerCmd) -> anyhow::Result<ExitCode> {
    match command {
        // No engine plan inventory exists before U6c, so no locator and
        // digest a caller supplies can be bound to an attempt.
        BrokerCmd::Serve(BrokerServeArgs {
            plan: _,
            plan_digest: _,
        }) => Err(BrokerError::PlanUnbound.into()),
    }
}
