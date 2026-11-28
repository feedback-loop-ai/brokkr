use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::gate_moved_head;
use serde::Serialize;

const PATH_LIMIT: usize = 50;

type Diff = BTreeMap<Vec<u8>, String>;
type Flagged = BTreeMap<Vec<u8>, (IndexFlags, FileReading)>;

/// Keep the three readings separate when a path appears in more than one.
#[derive(Debug, Default, Clone)]
pub(in crate::engine) struct Snapshot {
    worktree: Diff,
    index: Diff,
    flagged: Flagged,
}

#[cfg(test)]
impl From<Diff> for Snapshot {
    fn from(worktree: Diff) -> Self {
        Self {
            worktree,
            ..Self::default()
        }
    }
}

impl Snapshot {
    fn paths(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.worktree
            .keys()
            .chain(self.index.keys())
            .chain(self.flagged.keys())
    }

    fn differs(&self, end: &Self, path: &[u8]) -> bool {
        self.worktree.get(path) != end.worktree.get(path)
            || self.index.get(path) != end.index.get(path)
            || self.flagged.get(path) != end.flagged.get(path)
    }
}

pub(super) fn read(repo: &Path) -> Option<Snapshot> {
    Some(Snapshot {
        worktree: read_diff(repo, &[])?,
        index: read_diff(repo, &["--cached"])?,
        flagged: read_flags(repo)?,
    })
}

fn git(repo: &Path) -> Command {
    let mut command = Command::new("git");
    command.current_dir(repo).args([
        "-c",
        "core.fileMode=true",
        "-c",
        "diff.autoRefreshIndex=true",
        "-c",
        "core.fsmonitor=false",
        // Command-line config outranks the repository's, so Git before 2.42,
        // which lets `core.useReplaceRefs` undo the option below, still
        // reads the real objects.
        "-c",
        "core.useReplaceRefs=false",
        "--no-optional-locks",
        "--no-replace-objects",
    ]);
    command
}

fn stdout(command: &mut Command) -> Option<Vec<u8>> {
    let output = command.output().ok()?;
    output.status.success().then_some(output.stdout)
}

/// One diff supplies both the NUL-delimited paths and the corresponding
/// binary-capable patches. Hashing each patch separately lets us name changes
/// to an already dirty tree, including paths restored to HEAD.
fn read_diff(repo: &Path, options: &[&str]) -> Option<Diff> {
    let bytes = stdout(
        git(repo)
            .args([
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
            ])
            .args(options)
            .args(["HEAD", "--"]),
    )?;
    parse(&bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IndexFlags {
    AssumeUnchanged,
    SkipWorktree,
    Both,
}

impl IndexFlags {
    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            b'S' => Some(Self::SkipWorktree),
            b's' => Some(Self::Both),
            b'a'..=b'z' => Some(Self::AssumeUnchanged),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum FileReading {
    Regular { digest: String, executable: bool },
    Link(String),
    Missing,
}

fn read_flags(repo: &Path) -> Option<Flagged> {
    let bytes = stdout(git(repo).args(["ls-files", "-v", "-z", "--full-name", "--", ":/"]))?;
    let paths = flagged_paths(&bytes)?;
    if paths.is_empty() {
        return Some(Flagged::new());
    }

    // Git paths above are relative to the worktree even when the operated
    // directory is below it. Git names that worktree, skipping any invalid
    // .git on the way up.
    let output = stdout(git(repo).args(["rev-parse", "--show-toplevel"]));
    let root = worktree_root(repo, output.as_deref())?;
    paths
        .into_iter()
        .map(|(path, flags)| Some((path.to_vec(), (flags, read_flagged_file(&root, path)?))))
        .collect()
}

/// The root Git printed, without its one trailing newline, resolved as
/// the flagged-file reading compares parents to their resolved form.
fn worktree_root(repo: &Path, output: Option<&[u8]>) -> Option<PathBuf> {
    let root = OsStr::from_bytes(output?.strip_suffix(b"\n")?);
    repo.join(root).canonicalize().ok()
}

fn flagged_paths(output: &[u8]) -> Option<Vec<(&[u8], IndexFlags)>> {
    let mut flagged = Vec::new();
    for record in output.split_inclusive(|&byte| byte == 0) {
        let (&tag, rest) = record.split_first()?;
        let path = rest.strip_prefix(b" ")?.strip_suffix(b"\0")?;
        if let Some(flags) = IndexFlags::from_tag(tag) {
            flagged.push((path, flags));
        }
    }
    Some(flagged)
}

fn read_flagged_file(root: &Path, path: &[u8]) -> Option<FileReading> {
    let path = Path::new(OsStr::from_bytes(path));
    if !path
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let file = root.join(path);
    let parent = file.parent()?;
    match parent.canonicalize() {
        Ok(resolved) if resolved == parent => {}
        Ok(_) => return Some(FileReading::Missing),
        Err(error) => return missing(error),
    }
    let metadata = match file.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) => return missing(error),
    };
    if metadata.is_symlink() {
        let target = file.read_link().ok()?;
        Some(FileReading::Link(sha256_bytes(
            target.as_os_str().as_bytes(),
        )))
    } else if metadata.is_file() {
        Some(FileReading::Regular {
            digest: sha256_bytes(&fs::read(file).ok()?),
            executable: metadata.permissions().mode() & 0o100 != 0,
        })
    } else {
        Some(FileReading::Missing)
    }
}

fn missing(error: io::Error) -> Option<FileReading> {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Some(FileReading::Missing),
        _ => None,
    }
}

/// With renames disabled the raw records and patches are in the same
/// order. Git emits a deletion and an addition for a file type change.
fn parse(output: &[u8]) -> Option<Diff> {
    if output.is_empty() {
        return Some(Diff::new());
    }
    let separator = output.windows(2).position(|pair| pair == b"\0\0")?;
    let (raw, patch) = (&output[..separator], &output[separator + 2..]);
    let mut fields = raw.split(|&byte| byte == 0);
    let mut patches = patch_offsets(patch).peekable();
    let mut snapshot = Diff::new();
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
        (None, Some(_)) => Some(encode_tree_unreadable(Unreadable::Start)),
        (Some(_), None) => Some(encode_tree_unreadable(Unreadable::End)),
        (None, None) => Some(encode_tree_unreadable(Unreadable::Both)),
    }
}

fn changed(start: &Snapshot, end: &Snapshot) -> Option<String> {
    let distinct: BTreeSet<_> = start.paths().chain(end.paths()).collect();
    let mut changed = distinct.into_iter().filter(|path| start.differs(end, path));
    let paths: Vec<String> = changed
        .by_ref()
        .take(PATH_LIMIT)
        .map(|path| show(path))
        .collect();
    (!paths.is_empty()).then(|| {
        encode(&TreeEvidence::Changed {
            omitted: changed.count(),
            paths: &paths,
        })
    })
}

enum Unreadable {
    Start,
    End,
    Both,
}

#[derive(Serialize)]
#[serde(untagged)]
enum TreeEvidence<'a> {
    Changed {
        omitted: usize,
        paths: &'a [String],
    },
    Unreadable {
        end_unreadable: bool,
        start_unreadable: bool,
    },
}

fn encode_tree_unreadable(unreadable: Unreadable) -> String {
    encode(&TreeEvidence::Unreadable {
        end_unreadable: matches!(unreadable, Unreadable::End | Unreadable::Both),
        start_unreadable: matches!(unreadable, Unreadable::Start | Unreadable::Both),
    })
}

fn encode(evidence: &TreeEvidence<'_>) -> String {
    gate_moved_head::encode_tree(evidence)
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
        let dir = tempfile::tempdir().unwrap();
        let absent = dir.path().join("missing");
        assert_eq!(read(&absent).map(|_| ()), None);
        assert_eq!(read_flagged_file(dir.path(), b"../outside"), None);
        assert_eq!(missing(io::ErrorKind::PermissionDenied.into()), None);
    }

    #[test]
    fn the_root_is_the_one_line_git_printed() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = dir.path().canonicalize().unwrap();
        let mut line = resolved.as_os_str().as_bytes().to_vec();
        assert_eq!(worktree_root(dir.path(), Some(&line)), None);
        line.push(b'\n');
        assert_eq!(worktree_root(dir.path(), Some(&line)), Some(resolved));
        assert_eq!(worktree_root(dir.path(), Some(b"missing\n")), None);
        assert_eq!(worktree_root(dir.path(), None), None);
    }
}
