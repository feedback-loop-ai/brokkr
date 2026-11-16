//! Doctor's lines for the trees under a recorded root (decision 0104
//! rulings 6 and 7): a directory no open `seat_trees` row names is listed
//! with its size, and every tree the sweep would keep is listed with the
//! sweep's own reason. Both are warnings, never refusals, and neither
//! removes anything: the row is the sweep's only work list.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use brokkr_protocol::tree::count_tree;
use brokkr_runtime::sweep;
use brokkr_store::{SeatTree, Store, StoreError};

use crate::doctor::Report;
use crate::render::Safe;

/// Report each directory under a root the seat-tree rows name that no open
/// row names, then each tree the sweep would keep with its reason. A
/// journal with no rows adds no line.
pub(super) fn report(report: &mut Report, store: &Store) {
    let (rows, open) = match rows(store) {
        Ok(rows) => rows,
        Err(error) => {
            report.warn("tree", format!("not read: {error}"));
            return;
        }
    };
    for (root, named) in by_root(&rows, &open) {
        list(report, &root, &named);
    }
    report_kept(report, store);
}

/// Each tree the sweep would keep, by the sweep's own derivation, with its
/// reason and the bytes the count gives. Doctor only paints the verdict.
fn report_kept(report: &mut Report, store: &Store) {
    match sweep::kept(store) {
        Ok(kept) => {
            for kept in kept {
                kept_line(report, kept.path(), &kept.reason());
            }
        }
        Err(error) => report.warn("tree", format!("kept trees not read: {error}")),
    }
}

/// Every seat-tree row, across every run, in run-list order, and the seqs
/// of the rows still open: a row a `tree_sweeps` row closed is not one of
/// them, so a tree recreated at its path is visible.
fn rows(store: &Store) -> Result<(Vec<SeatTree>, BTreeSet<i64>), StoreError> {
    let open: BTreeSet<i64> = store
        .open_seat_trees()?
        .into_iter()
        .map(|row| row.seq)
        .collect();
    let mut rows = Vec::new();
    for (run, _, _) in store.list_runs()? {
        rows.extend(store.seat_trees(&run)?);
    }
    Ok((rows, open))
}

/// Each recorded root, and the attempt names its rows give, the open ones
/// apart from those a sweep already closed.
#[derive(Default)]
struct Named {
    open: BTreeSet<OsString>,
    closed: BTreeSet<OsString>,
}

/// Each recorded root, and the attempt names its rows give under it.
fn by_root(rows: &[SeatTree], open: &BTreeSet<i64>) -> BTreeMap<PathBuf, Named> {
    let mut roots: BTreeMap<PathBuf, Named> = BTreeMap::new();
    for row in rows {
        let named = roots.entry(row.root.clone()).or_default();
        let names = match open.contains(&row.seq) {
            true => &mut named.open,
            false => &mut named.closed,
        };
        names.insert(OsString::from(&row.attempt_id));
    }
    roots
}

/// Each directory under `root` no open row names, with the bytes the walk
/// counts. A directory only a closed row names is named as such. A root
/// that is not its own canonical path is named as the sweep sees it,
/// because every row under it is kept; a root that is gone adds no line. A
/// file, a link, an entry that vanished and an unreadable root add no
/// line; nothing is removed.
fn list(report: &mut Report, root: &Path, named: &Named) {
    list_with(report, root, named, |path| std::fs::symlink_metadata(path));
}

/// [`list`] with the stat a test drives, so an entry that vanishes between
/// the directory read and its own stat is provable on one host.
fn list_with(
    report: &mut Report,
    root: &Path,
    named: &Named,
    stat: impl Fn(&Path) -> std::io::Result<std::fs::Metadata>,
) {
    match std::fs::canonicalize(root) {
        Ok(canonical) if canonical.as_os_str() == root.as_os_str() => {}
        Ok(_) => {
            report.warn(
                "tree",
                format!(
                    "{}: not its own canonical path, so the sweep keeps its trees",
                    safe(&root.display().to_string())
                ),
            );
            return;
        }
        Err(_) => return,
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if named.open.contains(&entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = stat(&path) else {
            continue;
        };
        if !meta.file_type().is_dir() {
            continue;
        }
        match named.closed.contains(&entry.file_name()) {
            true => line_closed(report, &path),
            false => line(report, &path),
        }
    }
}

/// One unrecorded tree's warning: the path through `safe`, then its size.
/// A count that refuses is named by its cause and removes nothing.
pub(super) fn line(report: &mut Report, path: &Path) {
    line_saying(report, path, "named by no seat tree");
}

/// One directory only a closed row names: a tree recreated at a path the
/// sweep already closed, which no pass will remove.
fn line_closed(report: &mut Report, path: &Path) {
    line_saying(report, path, "named only by a closed seat tree");
}

/// One named tree's warning with its size and the words that say which row
/// named it, or none.
fn line_saying(report: &mut Report, path: &Path, saying: &str) {
    let named = safe(&path.display().to_string());
    match count_tree(path) {
        Ok(Some(counted)) => match counted.unreadable {
            Some(_) => report.warn(
                "tree",
                format!("{named}: at least {} bytes, {saying}", counted.bytes),
            ),
            None => report.warn(
                "tree",
                format!("{named}: {} bytes, {saying}", counted.bytes),
            ),
        },
        Ok(None) => report.warn("tree", format!("{named}: gone before the count, {saying}")),
        Err(error) => report.warn(
            "tree",
            format!("the count failed: {}", safe(&error.to_string())),
        ),
    }
}

/// One kept tree's warning: the path and the sweep's reason through
/// `safe`, then the size. A count that refuses, finds the tree gone or
/// reads only a lower bound still names the tree and its reason.
pub(super) fn kept_line(report: &mut Report, path: &Path, reason: &str) {
    let named = safe(&path.display().to_string());
    let reason = safe(reason);
    match count_tree(path) {
        Ok(Some(counted)) => match counted.unreadable {
            Some(_) => report.warn(
                "tree",
                format!("{named}: at least {} bytes, kept: {reason}", counted.bytes),
            ),
            None => report.warn(
                "tree",
                format!("{named}: {} bytes, kept: {reason}", counted.bytes),
            ),
        },
        Ok(None) => report.warn(
            "tree",
            format!("{named}: gone before the count, kept: {reason}"),
        ),
        Err(error) => report.warn(
            "tree",
            format!(
                "{named}: the count failed: {}, kept: {reason}",
                safe(&error.to_string())
            ),
        ),
    }
}

fn safe(text: &str) -> String {
    Safe::new(text).as_str().to_string()
}

#[cfg(test)]
mod tests;
