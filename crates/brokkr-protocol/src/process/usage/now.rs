//! What a running run's driver tree uses now, for admission's
//! reservations (decision 0125 ruling 1): one read of the process table,
//! each root's tree summed as the meter sums one tick's (decision 0112),
//! the root and its descendants by parent.

use std::time::Duration;

use super::super::table;
use super::tree;

/// One tree's resident memory now, and the CPU time it has spent, the
/// time of the children its processes waited for included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeNow {
    pub memory_bytes: u64,
    pub cpu: Duration,
}

/// Each of `roots`' trees as one read of the process table shows it,
/// `None` for a root the table does not list; `None` whole when the table
/// cannot be read.
pub fn trees_now(roots: &[u32]) -> Option<Vec<Option<TreeNow>>> {
    let entries = table::snapshot().ok()?;
    let each = |root: &u32| {
        let sample = tree(&entries, i32::try_from(*root).ok()?)?;
        Some(TreeNow {
            memory_bytes: sample.rss,
            cpu: sample.cpu,
        })
    };
    Some(roots.iter().map(each).collect())
}

#[cfg(test)]
mod tests;
