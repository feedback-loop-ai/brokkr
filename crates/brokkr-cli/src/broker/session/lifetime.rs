//! A bound plan's one lifetime and its ledger (decision 0065 slice two,
//! U6d; CR3, MB3, MB4): the ledger is created beside the plan before
//! anything of the plan is checked further or started, and closed once
//! admission has answered, Failed with the first cause. An admitted plan's
//! cause is SD3's incomplete serving, with which `serve` then refuses, so
//! no session yet closes Clean, and none has a call.

use std::ffi::OsStr;
use std::os::fd::OwnedFd;

use brokkr_protocol::broker::ledger::{Failure, Ledger, Unrecorded};
use brokkr_protocol::broker::{Owner, Refusal};

/// Why `serve` ends before serving: its plan refused, or its ledger not
/// recorded, in the protocol's own words.
#[derive(Debug)]
pub(in crate::broker) enum Ended {
    Refused(Refusal),
    Unrecorded(Unrecorded),
}

impl std::fmt::Display for Ended {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ended::Refused(refusal) => refusal.fmt(formatter),
            Ended::Unrecorded(unrecorded) => unrecorded.fmt(formatter),
        }
    }
}

impl std::error::Error for Ended {}

impl From<Refusal> for Ended {
    fn from(refusal: Refusal) -> Ended {
        Ended::Refused(refusal)
    }
}

/// `admission` run inside the ledger of the plan named `plan` in the held
/// `attempt` directory, the plan sealed at `digest` for `owner`: a ledger
/// that cannot be opened ends the broker before anything starts, a second
/// lifetime's included. The closure carries admission's refusal, or SD3's
/// where it admitted the plan. That first cause ends the broker whether or
/// not the closure is recorded: where it is not, recovery finds only the
/// verified prefix, and never a success.
pub(super) fn lived<T>(
    (attempt, plan): (&OwnedFd, &OsStr),
    (owner, digest): (&Owner, &str),
    admission: impl FnOnce() -> Result<T, Refusal>,
) -> Result<T, Ended> {
    let opened = Ledger::open(attempt, plan, owner, digest);
    let mut ledger = opened.map_err(Ended::Unrecorded)?;
    let admitted = admission();
    let cause = admitted.as_ref().err().cloned();
    ledger.fail(Failure::Refused(
        cause.unwrap_or(Refusal::ServingIncomplete),
    ));
    ledger.close().ok();
    Ok(admitted?)
}
