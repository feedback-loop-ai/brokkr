use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::gate_moved_head::{encode_tree_change, encode_tree_unreadable};

const PATH_LIMIT: usize = 50;

/// Per-path digests of the tracked worktree's differences from HEAD.
pub(super) type Snapshot = BTreeMap<Vec<u8>, String>;

/// One Git diff supplies both the NUL-delimited paths and the corresponding
/// binary-capable patches. Hashing each patch separately lets us name changes
/// to an already dirty tree, including paths restored to HEAD.
pub(super) fn read(repo: &Path) -> Option<Snapshot> {
    let output = Command::new("git")
        .current_dir(repo)
        .args([
            "-c",
            "core.fileMode=true",
            "--no-optional-locks",
            "diff",
            "--raw",
            "--binary",
            "--full-index",
            "-z",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--no-relative",
            "--ignore-submodules=dirty",
            "--submodule=short",
            "HEAD",
            "--",
        ])
        .output()
        .ok()?;
    output.status.success().then_some(())?;
    parse(&output.stdout)
}

/// With renames disabled the raw records and patches are in the same
/// order. Git emits a deletion and an addition for a file type change.
fn parse(output: &[u8]) -> Option<Snapshot> {
    if output.is_empty() {
        return Some(Snapshot::new());
    }
    let separator = output.windows(2).position(|pair| pair == b"\0\0")?;
    let (raw, patch) = (&output[..separator], &output[separator + 2..]);
    let mut fields = raw.split(|&byte| byte == 0);
    let mut patches = patch_offsets(patch).peekable();
    let mut snapshot = Snapshot::new();
    while let Some(header) = fields.next() {
        let path = fields.next()?;
        let start = patches.next()?;
        if header.ends_with(b" T") {
            patches.next()?;
        }
        let end = patches.peek().copied().unwrap_or(patch.len());
        snapshot.insert(path.to_vec(), sha256_bytes(&patch[start..end]));
    }
    patches.peek().is_none().then_some(snapshot)
}

fn patch_offsets(patch: &[u8]) -> impl Iterator<Item = usize> + '_ {
    patch
        .split_inclusive(|&byte| byte == b'\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line))
        })
        .filter_map(|(offset, line)| line.starts_with(b"diff --git ").then_some(offset))
}

pub(super) fn defect(start: Option<Snapshot>, end: Option<Snapshot>) -> Option<String> {
    match (start, end) {
        (Some(start), Some(end)) => changed(&start, &end),
        (start, end) => Some(encode_tree_unreadable(start.is_none(), end.is_none())),
    }
}

fn changed(start: &Snapshot, end: &Snapshot) -> Option<String> {
    let distinct: BTreeSet<_> = start.keys().chain(end.keys()).collect();
    let mut changed = distinct
        .into_iter()
        .filter(|&path| start.get(path) != end.get(path));
    let paths: Vec<String> = changed
        .by_ref()
        .take(PATH_LIMIT)
        .map(|path| show(path))
        .collect();
    (!paths.is_empty()).then(|| encode_tree_change(&paths, changed.count()))
}

fn show(path: &[u8]) -> String {
    match std::str::from_utf8(path) {
        Ok(path) => path.to_owned(),
        Err(_) => path.escape_ascii().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_incomplete_or_inconsistent_diff_is_unreadable() {
        for bytes in [
            &b"missing separator"[..],
            b":header M\0\0diff --git a/file b/file\n",
            b":header M\0file\0\0missing patch",
            b":header T\0file\0\0diff --git a/file b/file\n",
            b":header M\0file\0\0diff --git a/file b/file\ndiff --git a/extra b/extra\n",
        ] {
            assert_eq!(parse(bytes), None, "{bytes:?}");
        }
        let missing = tempfile::tempdir().unwrap().path().join("missing");
        assert_eq!(read(&missing), None);
    }
}
