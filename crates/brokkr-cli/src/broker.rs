//! `brokkr broker serve` (decision 0077, design D5): the broker the
//! harness starts beside `brokkr hands`, one per held MCP capability, from
//! the configuration the engine writes. Its authority is the engine's plan
//! bound to the attempt and nothing a caller names: the command line holds
//! a plan locator and its digest, never a server argv, a grant or a secret
//! value.
//!
//! The [`session`] binds the plan to the attempt and checks every plan
//! field and the box intent in MB3's refusal order, before any secret is
//! looked up or anything is started; its private `observe` verb is the
//! observer `serve` runs to prepare the box (U6c5b), and holds nothing
//! `serve` does not. Its private `bootstrap` verb is the waiting helper the
//! server box runs (U6c6a, U6c8): it checks the box it stands in, reports
//! ready on a private pipe, receives the declared bindings, which the
//! broker looks up only then, and execs the server. The [`rpc`] protocol
//! then initializes that server and lists its filtered catalog (U6e). The
//! serving protections are a later unit's (slice two U6f), so the box of
//! an admitted plan is ended once its server has initialized, before any
//! message reaches the harness, and the plan still refused with SD3's
//! incomplete-serving cause; decision 0065's compile fence still refuses
//! every MCP grant.

mod bootstrap;
mod rpc;
mod session;

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use brokkr_core::canonical;
use brokkr_protocol::broker::Refusal;

use crate::cli_args::{BrokerCmd, BrokerServeArgs};

/// The longest plan locator taken, in bytes: macOS's `PATH_MAX`, the
/// smaller of the two supported hosts' (decision 0063).
const PLAN_LOCATOR_MAX: usize = 1024;

/// Why the broker takes no plan locator or digest, as the command line is
/// parsed. A well-formed one the broker does not serve is a [`Refusal`].
#[derive(Debug)]
pub(crate) enum BrokerError {
    Relative,
    Parent,
    TooLong,
    Digest,
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
    match canonical::is_sha256_hex(text) {
        true => Ok(text.to_string()),
        false => Err(BrokerError::Digest),
    }
}

/// Serve the plan `command` names: refuse before looking up a secret or
/// starting anything where a check fails, and otherwise once its box has
/// been launched, its server handed its bindings, started and initialized,
/// and the box ended, before any message reaches the harness. Or, as
/// `serve`'s private observer,
/// observe it and hand back what was checked; or, as its waiting
/// bootstrap, report the box ready and exec the server, returning only the
/// cause of one that did not start.
pub(crate) fn run(command: BrokerCmd) -> anyhow::Result<ExitCode> {
    match command {
        BrokerCmd::Serve(BrokerServeArgs { plan, plan_digest }) => {
            // The checked handles stay held until the refusal ends the
            // broker: the launch mounted these very objects.
            let _admitted = session::admit(&plan, &plan_digest)?;
            Err(Refusal::ServingIncomplete.into())
        }
        BrokerCmd::Observe(BrokerServeArgs { plan, plan_digest }) => {
            session::observe(&plan, &plan_digest)?;
            Ok(ExitCode::SUCCESS)
        }
        BrokerCmd::Bootstrap(args) => Err(bootstrap::run(&args).into()),
    }
}
