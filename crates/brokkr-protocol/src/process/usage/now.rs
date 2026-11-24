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
/// `None` for an absent root or a tree containing another requested root
/// (including a repeated pid); `None` whole when the table cannot be read.
pub fn trees_now(roots: &[u32]) -> Option<Vec<Option<TreeNow>>> {
    let entries = table::snapshot().ok()?;
    Some(from_table(&entries, roots))
}

fn from_table(entries: &[table::Entry], roots: &[u32]) -> Vec<Option<TreeNow>> {
    let roots: Vec<_> = roots.iter().map(|root| i32::try_from(*root).ok()).collect();
    let each = |(at, root): (usize, &Option<i32>)| {
        let (sample, pids) = tree(entries, (*root)?)?;
        let overlaps = roots
            .iter()
            .enumerate()
            .any(|(other, pid)| other != at && pid.is_some_and(|pid| pids.contains(&pid)));
        if overlaps {
            return None;
        }
        Some(TreeNow {
            memory_bytes: sample.rss,
            cpu: sample.cpu,
        })
    };
    roots.iter().enumerate().map(each).collect()
}

#[cfg(test)]
mod tests;
