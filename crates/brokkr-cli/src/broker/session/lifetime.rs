//! A bound plan's one lifetime and its ledger (decision 0065 slice two,
//! U6d, U6e; CR3, MB3, MB4): the ledger is created beside the plan before
//! anything of the plan is checked further or started, handed to the
//! session for its calls' records, and closed once the session has ended,
//! Failed with its first cause: admission's refusal, or the fatal ending
//! of a server it started. An admitted plan whose server initialized has
//! SD3's incomplete serving for its cause, with which `serve` then
//! refuses, so no session yet closes Clean.

use std::ffi::OsStr;
use std::os::fd::OwnedFd;

use brokkr_protocol::broker::ledger::{Failure, Ledger, Unrecorded};
use brokkr_protocol::broker::{Owner, Refusal};

use crate::broker::rpc::Fatal;

/// Why `serve` ends before serving: its plan refused, its ledger not
/// recorded, or its server's session ended, in the protocol's own words.
#[derive(Debug, PartialEq)]
pub(in crate::broker) enum Ended {
    Refused(Refusal),
    Unrecorded(Unrecorded),
    Failed(Fatal),
}

impl std::fmt::Display for Ended {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ended::Refused(refusal) => refusal.fmt(formatter),
            Ended::Unrecorded(unrecorded) => unrecorded.fmt(formatter),
            Ended::Failed(fatal) => fatal.fmt(formatter),
        }
    }
}

impl std::error::Error for Ended {}

impl From<Refusal> for Ended {
    fn from(refusal: Refusal) -> Ended {
        Ended::Refused(refusal)
    }
}

impl Ended {
    /// The cause a session that ended so latches: its refusal, or its
    /// fatal ending's; none where a record could not be made, which latches
    /// nothing here: the ledger latched its own limit where that was why,
    /// and after an append that failed it records nothing, its closure
    /// included. A record the ledger refused before appending it leaves
    /// the closure to what is latched; the call path's own order makes
    /// none so.
    fn failure(&self) -> Option<Failure> {
        match self {
            Ended::Refused(refusal) => Some(Failure::Refused(refusal.clone())),
            Ended::Failed(fatal) => Some(fatal.failure()),
            Ended::Unrecorded(_) => None,
        }
    }
}

/// `session` run inside the ledger of the plan named `plan` in the held
/// `attempt` directory, the plan sealed at `digest` for `owner`, and handed
/// that ledger: a ledger that cannot be opened ends the broker before
/// anything starts, a second lifetime's included. The session's ending is
/// then [`settled`] in the ledger, and ends the broker whether or not the
/// closure is recorded.
pub(super) fn lived<T>(
    (attempt, plan): (&OwnedFd, &OsStr),
    (owner, digest): (&Owner, &str),
    session: impl FnOnce(&mut Ledger) -> Result<T, Ended>,
) -> Result<T, Ended> {
    let opened = Ledger::open(attempt, plan, owner, digest);
    let mut ledger = opened.map_err(Ended::Unrecorded)?;
    let ended = session(&mut ledger);
    settled(ledger, &ended);
    ended
}

/// Close `ledger` once its session has `ended`: Failed with the session's
/// first cause, SD3's where it ended admitted. Where the closure is not
/// recorded, recovery finds only the verified prefix, and never a success.
pub(in crate::broker) fn settled<T>(mut ledger: Ledger, ended: &Result<T, Ended>) {
    let cause = match ended {
        Ok(_) => Some(Failure::Refused(Refusal::ServingIncomplete)),
        Err(ended) => ended.failure(),
    };
    if let Some(cause) = cause {
        ledger.fail(cause);
    }
    ledger.close().ok();
}
