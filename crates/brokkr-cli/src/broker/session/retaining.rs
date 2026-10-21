//! A retaining plan's staging, read before its ledger opens (decision 0065
//! slice two, U8b; CR2, CR3, operator ruling 2026-10-15): how many of the
//! attempt's sealed plans retain their responses, which its share of the
//! attempt's budget is divided by, as the engine divided it when it sealed
//! them, read from the same sealed inventory and nothing else, permitted
//! fallback slots included; and its own handle of the attempt directory,
//! where its responses are staged beside its ledger.

use std::ffi::OsStr;
use std::os::fd::OwnedFd;
use std::path::Path;

use brokkr_core::canonical;
use brokkr_protocol::broker::ledger::Unrecorded;
use brokkr_protocol::broker::{Inventory, Plan, Refusal};

use super::{ensure, protected, Ended, INVENTORY};

/// What a retaining plan's ledger stages by: the number of retaining plans
/// its share divides the budget by, and a handle of its attempt directory,
/// closed on exec, so no server inherits it.
pub(super) type Retention = (usize, OwnedFd);

/// The retention of the bound `plan`, held in the attempt directory `dir`
/// at `attempt`: none where it retains nothing; else every plan the
/// attempt's inventory pins read beside it as the bound one was, each of
/// them retaining or not, and a handle of `dir` of its own, or, where
/// none can be made, the record that cannot be. A pin outside the attempt
/// directory, or one whose plan does not verify, leaves the bound plan
/// unbound: its share could not be the one the engine sealed.
pub(super) fn retention(
    plan: &Plan,
    dir: &OwnedFd,
    attempt: &Path,
) -> Result<Option<Retention>, Ended> {
    if !plan.retained {
        return Ok(None);
    }
    let inventory = protected(dir, OsStr::new(INVENTORY)).ok_or(Refusal::Unbound)?;
    let inventory: Inventory = serde_json::from_slice(&inventory).or(Err(Refusal::Unbound))?;
    let retaining = inventory.plans.iter().try_fold(0, |retaining, pin| {
        let beside = (pin.locator.parent() == Some(attempt)).then_some(());
        let name = beside
            .and(pin.locator.file_name())
            .ok_or(Refusal::Unbound)?;
        let bytes = protected(dir, name).ok_or(Refusal::Unbound)?;
        ensure(
            canonical::sha256_bytes(&bytes) == pin.digest,
            Refusal::Unbound,
        )?;
        let pinned: Plan = serde_json::from_slice(&bytes).or(Err(Refusal::Unbound))?;
        Ok::<_, Refusal>(retaining + usize::from(pinned.retained))
    })?;
    let held = dir.try_clone().or(Err(Unrecorded::Record))?;
    Ok(Some((retaining, held)))
}

#[cfg(test)]
mod tests;
