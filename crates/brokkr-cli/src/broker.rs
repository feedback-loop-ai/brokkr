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
//! then initializes that server and lists its filtered catalog (U6e), and
//! serves the harness on the broker's own standard input and output, every
//! byte it lets out masked and guarded at the output edge, the server's
//! standard error drained and masked beside it, and every call recorded
//! in the plan's ledger (U6f1a, U6f1b). Everything the broker writes to its
//! own standard error passes one display sanitizer, the shared masker and
//! the final guard, while the bindings that guarded the ledger are held: a
//! served session's server's masked tail before its ledger closes, Clean
//! only once it was taken; a failed session's tail and the broker's own
//! last line after, its first cause kept (`session::lifetime`, which holds
//! the one description of that order). A plan that retains its responses is
//! refused with SD3's retention cause before any MCP message is exchanged
//! with its server, until U8b builds retention; decision 0065's compile
//! fence still refuses every MCP grant.
//!
//! A harness may end `serve` by an interrupt or a termination signal, as
//! Claude Code ends every MCP server it started with SIGINT, rather than by
//! closing its standard input (U9b1e; CR3). Either signal is then its harness
//! ending the session: [`Signalled`] only counts what arrives, on a pipe the
//! session reads beside its own work, and the session ends as a closed input
//! ends it, its ledger closed by the one writer it has.

mod bootstrap;
mod rpc;
mod session;
mod survey;

use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Mutex, PoisonError};

use brokkr_core::canonical;
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use crate::cli_args::{BrokerCmd, BrokerServeArgs};

/// The signals by which a harness ends `serve`'s session.
const ENDING: [i32; 2] = [SIGINT, SIGTERM];

/// How many of [`ENDING`]'s signals have reached `serve`: each arrival
/// written to signal-hook's own pipe by its handler, which does nothing
/// else, and counted only when the session asks, on whichever of its
/// threads asks. Two arrivals of one signal before it asks are one. Where
/// the handlers cannot be installed, none is counted and a signal ends the
/// process as it did before, leaving its ledger unclosed, which CR3
/// refuses.
struct Signalled(Mutex<(Option<Signals>, usize)>);

impl Signalled {
    /// [`ENDING`]'s handlers installed, before anything of the plan is read.
    fn observed() -> Signalled {
        Signalled(Mutex::new((Signals::new(ENDING).ok(), 0)))
    }

    /// Whether at least `times` signals have arrived.
    fn at_least(&self, times: usize) -> bool {
        let mut held = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let (signals, seen) = &mut *held;
        *seen += signals.iter_mut().flat_map(Signals::pending).count();
        *seen >= times
    }
}

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

/// The engine's HOME as `home` states it, where it is absolute and
/// resolves: the one rule admission's binding and the provisioning survey
/// each read the engine's HOME by, each refusing with its own cause.
fn canonical_home(home: &Path) -> Option<PathBuf> {
    home.is_absolute().then_some(())?;
    std::fs::canonicalize(home).ok()
}

/// Serve the plan `command` names: refuse before looking up a secret or
/// starting anything where a check fails; otherwise launch its box, hand
/// its server its bindings, start and initialize it, and serve the harness
/// until it ends, the box ended then, or end with the session's first
/// cause, its line written by the session itself, masked and guarded,
/// without waiting on a standard error nobody reads. Or, as `serve`'s
/// private observer, observe it and hand back what was checked; or, as its
/// waiting bootstrap, report the box ready and exec the server, returning
/// only the cause of one that did not start. Or, as the engine's
/// provisioning observer, survey one box and reply ([`survey`], U7c0b).
pub(crate) fn run(command: BrokerCmd) -> anyhow::Result<ExitCode> {
    match command {
        BrokerCmd::Serve(BrokerServeArgs { plan, plan_digest }) => {
            Ok(session::serve(&plan, &plan_digest, &Signalled::observed()))
        }
        BrokerCmd::Observe(BrokerServeArgs { plan, plan_digest }) => {
            session::observe(&plan, &plan_digest)?;
            Ok(ExitCode::SUCCESS)
        }
        BrokerCmd::Bootstrap(args) => Err(bootstrap::run(&args).into()),
        BrokerCmd::Survey => survey::run(),
    }
}
