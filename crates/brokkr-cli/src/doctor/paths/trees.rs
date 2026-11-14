//! Doctor's line for the trees under a recorded root that no `seat_trees`
//! row names (decision 0104 ruling 6): trees an older engine or a person
//! made. It is a warning, never a refusal, and it removes nothing: the
//! row is the sweep's only work list.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use brokkr_protocol::tree::count_tree;
use brokkr_store::{SeatTree, Store, StoreError};

use crate::doctor::Report;
use crate::render::Safe;

/// Report each directory under a root the seat-tree rows name that no row
/// names. A journal with no rows adds no line.
pub(super) fn report(report: &mut Report, store: &Store) {
    let rows = match rows(store) {
        Ok(rows) => rows,
        Err(error) => {
            report.warn("tree", format!("not read: {error}"));
            return;
        }
    };
    for (root, named) in by_root(&rows) {
        list(report, &root, &named);
    }
}

/// Every seat-tree row, across every run, in run-list order. The row keeps
/// its own root, so a row that names a tree under another root is still
/// that root's named attempt.
fn rows(store: &Store) -> Result<Vec<SeatTree>, StoreError> {
    let mut rows = Vec::new();
    for (run, _, _) in store.list_runs()? {
        rows.extend(store.seat_trees(&run)?);
    }
    Ok(rows)
}

/// Each recorded root, and the attempt names its rows give.
fn by_root(rows: &[SeatTree]) -> BTreeMap<PathBuf, BTreeSet<OsString>> {
    let mut roots: BTreeMap<PathBuf, BTreeSet<OsString>> = BTreeMap::new();
    for row in rows {
        roots
            .entry(row.root.clone())
            .or_default()
            .insert(OsString::from(&row.attempt_id));
    }
    roots
}

/// Each directory under `root` no row names, with the bytes the walk
/// counts. A root that is not its own canonical path is named as the
/// sweep sees it, because every row under it is kept; a root that is gone
/// adds no line. A file, a link, an entry that vanished and an unreadable
/// root add no line; nothing is removed.
fn list(report: &mut Report, root: &Path, named: &BTreeSet<OsString>) {
    list_with(report, root, named, |path| std::fs::symlink_metadata(path));
}

/// [`list`] with the stat a test drives, so an entry that vanishes between
/// the directory read and its own stat is provable on one host.
fn list_with(
    report: &mut Report,
    root: &Path,
    named: &BTreeSet<OsString>,
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
        if named.contains(&entry.file_name()) {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = stat(&path) else {
            continue;
        };
        if !meta.file_type().is_dir() {
            continue;
        }
        line(report, &path);
    }
}

/// One unrecorded tree's warning: the path through `safe`, then its size.
/// A count that refuses is named by its cause and removes nothing.
pub(super) fn line(report: &mut Report, path: &Path) {
    let named = safe(&path.display().to_string());
    match count_tree(path) {
        Ok(Some(counted)) => match counted.unreadable {
            Some(_) => report.warn(
                "tree",
                format!(
                    "{named}: at least {} bytes, named by no seat tree",
                    counted.bytes
                ),
            ),
            None => report.warn(
                "tree",
                format!("{named}: {} bytes, named by no seat tree", counted.bytes),
            ),
        },
        Ok(None) => report.warn(
            "tree",
            format!("{named}: gone before the count, named by no seat tree"),
        ),
        Err(error) => report.warn(
            "tree",
            format!("the count failed: {}", safe(&error.to_string())),
        ),
    }
}

fn safe(text: &str) -> String {
    Safe::new(text).as_str().to_string()
}

#[cfg(test)]
mod tests;
