//! The compiler's last lints over a table's phases, once its seats are read.

use std::collections::BTreeMap;

use brokkr_core::policy::Machine;

use super::{CompileError, Seat};

/// Every non-terminal phase has a seat to run it, and then, after the seat
/// lints, decision 0050 ruling 4: every valuation of the table is ruled.
pub(super) fn seated_and_ruled(
    machine: &Machine,
    seats: &BTreeMap<String, Seat>,
) -> Result<(), CompileError> {
    for phase in &machine.phases {
        if !machine.terminal.contains(phase) && !seats.contains_key(phase) {
            return Err(CompileError::Invalid(format!(
                "non-terminal phase '{phase}' has no seat (no executor can run it)"
            )));
        }
    }
    machine.refuse_unruled()?;
    Ok(())
}
