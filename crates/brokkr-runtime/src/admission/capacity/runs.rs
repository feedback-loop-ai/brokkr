//! Decision 0110 ruling 7, amending decision 0068 ruling 3: the journal
//! drives fewer runs than the ceiling its host configuration declares.
//! The runs counted are the census's, every run of the journal that reads
//! `Running`, direct `brokkr run` runs among them and an unstarted run
//! counted against nothing (decision 0095 rulings 4 and 6), and a claim's
//! re-check counts them again by the runs that moved (ruling 7). A host
//! file that declares no ceiling, every v1, v2 and v3 file, is a
//! declaration check, judged beside the providers' and routes'; the
//! ceiling itself is the first of the ceilings. Neither latches.

use std::num::NonZeroU32;
use std::path::Path;

use super::{Capacity, HostConfig};

/// The run ceiling `config`, read from `file`, declares, or why it
/// declares none.
pub(super) fn ceiling(config: &HostConfig, file: &Path) -> Result<NonZeroU32, Capacity> {
    config
        .runs
        .ok_or_else(|| Capacity::RunsUndeclared(file.to_path_buf()))
}

/// `running` runs below `ceiling`, so that one more would not pass it.
pub(super) fn below(running: usize, ceiling: NonZeroU32) -> Result<(), Capacity> {
    let ceiling = ceiling.get();
    match running >= ceiling as usize {
        true => Err(Capacity::RunsFull { running, ceiling }),
        false => Ok(()),
    }
}

#[cfg(test)]
mod tests;
