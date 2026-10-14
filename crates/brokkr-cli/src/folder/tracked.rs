//! What git tracks under an old layout's `.forge/`, asked of git and read
//! typed (decision 0071 ruling 3), and the shapes no listed `git mv` or
//! `git rm` could complete, each refused by name before a move is listed
//! (decision 0083 ruling 4; round 634k).

use std::fmt;
use std::path::Path;

use brokkr_runtime::places::LIBRARY_STATE;

use super::{entry, forged, unreadable, FolderError, Forge};

/// The names directly under `.forge/` that git tracks, as git's bytes,
/// `forge` what stands at the root's `.forge`. Outside any repository git
/// tracks nothing; inside one, a git that cannot be run or will not list
/// refuses by name, because an entry it tracks moved by `mv` would leave
/// git a deletion and a stranger. So do the shapes no `git mv` or `git rm`
/// could complete (round 634k's third finding): a submodule git tracks
/// under `.forge`, a tracked path git does not read in the work tree
/// ([`held`]), a linked `.forge` git tracks itself or tracks entries under,
/// one whose target lies inside the repository, where git does not look
/// ([`super::links::outside`]), a sparse checkout beside tracked entries ([`sparse`]), and any tracked
/// path under it git holds a change to ([`committed`]).
pub(super) fn tracked(root: &Path, forge: Forge) -> Result<Vec<Vec<u8>>, FolderError> {
    let repository = brokkr_runtime::places::root(root).map_err(FolderError::Places)?;
    if entry(&repository.join(".git"))?.is_none() {
        return Ok(Vec::new());
    }
    let listed = ask_git(root, "ls-files", &["-s", "-v", "-z", "--", LIBRARY_STATE])?;
    let index = held(root, &listed)?;
    // A `.forge` the index holds itself, where a directory stands or none
    // does, is a change `committed` refuses by name (` D .forge`).
    if forge == Forge::Link && (index.itself || !index.names.is_empty()) {
        let path = root.join(LIBRARY_STATE);
        return Err(FolderError::LinkedTracked { path });
    }
    if forge == Forge::Link {
        super::links::outside(root, &repository)?;
    }
    if !index.names.is_empty() && sparse(root)? {
        let root = root.to_path_buf();
        return Err(FolderError::Sparse { root });
    }
    committed(root)?;
    Ok(index.names)
}

/// What git's index holds at a path: a file or a link ([`held`]), or a submodule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    Blob,
    Submodule,
}

/// One record of `git ls-files -s -v`, `TAG MODE OBJECT STAGE<TAB>PATH`:
/// its tag, what its mode says git holds, and its path. The tags `-s -v`
/// prints are `H`, `S` and `M`, each lowercase where the assume-unchanged
/// bit is set; any other tag or mode is no record this reads.
fn staged(record: &[u8]) -> Option<(u8, Held, &[u8])> {
    let [tag @ (b'H' | b'S' | b'M' | b'h' | b's' | b'm'), b' ', record @ ..] = record else {
        return None;
    };
    let tab = record.iter().position(|byte| *byte == b'\t')?;
    let held = match record[..tab].split(|byte| *byte == b' ').next() {
        Some(b"100644" | b"100755" | b"120000") => Held::Blob,
        Some(b"160000") => Held::Submodule,
        _ => return None,
    };
    Some((*tag, held, &record[tab + 1..]))
}

/// What git's index holds at and under `.forge` ([`held`]).
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Index {
    /// The names directly under `.forge/` git tracks, as git's bytes.
    pub(super) names: Vec<Vec<u8>>,
    /// Whether git tracks `.forge` itself, a link committed as one, which
    /// a listed `rm` would leave git a deletion of (the fourth landing's
    /// review).
    pub(super) itself: bool,
}

/// What `git ls-files -s -v -z` printed in `root` holds at and under
/// `.forge`, as git's bytes, read typed (decision 0071 ruling 3). A
/// submodule there refuses by name: `git mv` moves one only together with
/// `.gitmodules`, which a change outside `.forge` stops (round 634k's
/// review). So does every path git does not read in the work tree, tagged
/// `S` or in lowercase: `git status` states no change to one, and `git rm`
/// refuses one a sparse checkout leaves out (round 634k's chief). A record
/// this cannot read is refused with its bytes.
pub(super) fn held(root: &Path, listed: &[u8]) -> Result<Index, FolderError> {
    use std::os::unix::ffi::OsStrExt;
    let prefix = forged("");
    let (mut names, mut unwatched, mut itself) = (Vec::new(), Vec::new(), false);
    for record in listed.split(|byte| *byte == 0).filter(|r| !r.is_empty()) {
        let (tag, held, path) = staged(record).ok_or_else(|| unread(root, "ls-files", record))?;
        if held == Held::Submodule {
            let path = root.join(std::ffi::OsStr::from_bytes(path));
            return Err(FolderError::TrackedSubmodule { path });
        }
        if tag != b'H' && tag != b'M' {
            unwatched.push(Unwatched {
                tag: char::from(tag),
                path: one_line(path),
            });
        }
        itself |= path == LIBRARY_STATE.as_bytes();
        let under = path.strip_prefix(prefix.as_bytes());
        let name = under.and_then(|rest| rest.split(|byte| *byte == b'/').next());
        names.extend(name.map(<[u8]>::to_vec));
    }
    match unwatched.is_empty() {
        true => Ok(Index { names, itself }),
        false => Err(FolderError::Unwatched {
            root: root.to_path_buf(),
            paths: unwatched,
        }),
    }
}

/// The refusal of a record git's `verb` printed in `root`, with its bytes.
fn unread(root: &Path, verb: &'static str, record: &[u8]) -> FolderError {
    FolderError::Untracked {
        verb,
        root: root.to_path_buf(),
        said: format!(
            "it printed a record this cannot read: '{}'",
            String::from_utf8_lossy(record).escape_debug()
        ),
    }
}

/// Refuse every tracked path under `.forge` whose index or work tree
/// differs from the last commit, by git's own answer, asked once: an
/// ignore file staged and never committed, or committed and edited since,
/// which `git rm` refuses, and a tracked entry no longer the file it was,
/// which `git mv` refuses. A status git will not give, or that this cannot
/// read, refuses as an unlisted repository does.
fn committed(root: &Path) -> Result<(), FolderError> {
    let args = ["--porcelain=v1", "-z", "--untracked-files=no", "--"];
    let status = ask_git(root, "status", &[&args[..], &[LIBRARY_STATE]].concat())?;
    let changes = changes(root, &status)?;
    match changes.is_empty() {
        true => Ok(()),
        false => Err(FolderError::Uncommitted {
            root: root.to_path_buf(),
            changes,
        }),
    }
}

/// Whether `root`'s repository has a sparse checkout, by git's own typed
/// answer (`core.sparseCheckout`, unset read as false). A tracked entry
/// git reads in the work tree can still have a destination the sparse
/// definition leaves out, and `git mv` refuses that (the third landing's
/// review), so a sparse checkout beside tracked entries lists no moves.
fn sparse(root: &Path) -> Result<bool, FolderError> {
    let args = [
        "--type=bool",
        "--default=false",
        "--get",
        "core.sparseCheckout",
    ];
    Ok(ask_git(root, "config", &args)? == b"true\n")
}

/// What `git -C root <verb> <args>` prints. A git that cannot be run, or
/// that fails, refuses by name, with what it said.
fn ask_git(root: &Path, verb: &'static str, args: &[&str]) -> Result<Vec<u8>, FolderError> {
    let git = Path::new("git");
    let mut asked = std::process::Command::new(git);
    asked.arg("-C").arg(root).arg(verb).args(args);
    let asked = asked.output().map_err(unreadable(git))?;
    match asked.status.success() {
        true => Ok(asked.stdout),
        false => Err(FolderError::Untracked {
            verb,
            root: root.to_path_buf(),
            said: String::from_utf8_lossy(&asked.stderr)
                .trim_end()
                .to_string(),
        }),
    }
}

/// `path`, git's bytes, as a refusal names it on one line, a control
/// character escaped.
fn one_line(path: &[u8]) -> String {
    String::from_utf8_lossy(path)
        .chars()
        .map(|c| match c.is_control() {
            true => c.escape_default().to_string(),
            false => c.to_string(),
        })
        .collect()
}

/// A tracked path git reports changed (`git status --porcelain=v1`): its
/// two-letter state, index then work tree, and its path as the refusal
/// names it on one line ([`one_line`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    state: [char; 2],
    path: String,
}

impl Change {
    /// One record, `XY PATH`, each letter one git documents for a tracked
    /// path; anything else is no record this reads.
    fn read(record: &[u8]) -> Option<Change> {
        let known = |letter: &u8| b" MTADRCU".contains(letter);
        match record {
            [x, y, b' ', path @ ..] if known(x) && known(y) && !path.is_empty() => Some(Change {
                state: [char::from(*x), char::from(*y)],
                path: one_line(path),
            }),
            _ => None,
        }
    }

    /// Whether git follows this record with the path it was renamed or
    /// copied from.
    fn renamed(&self) -> bool {
        self.state.iter().any(|letter| matches!(letter, 'R' | 'C'))
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [index, tree] = self.state;
        write!(f, "{index}{tree} {}", self.path)
    }
}

/// A tracked path git does not read in the work tree (`git ls-files -v`):
/// its tag, `S` for skip-worktree, as a sparse checkout sets it, or a
/// lowercase one for assume-unchanged, and its path on one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Unwatched {
    tag: char,
    path: String,
}

impl fmt::Display for Unwatched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.tag, self.path)
    }
}

/// The changes `git status --porcelain=v1 -z` printed in `root`, read
/// typed (decision 0071 ruling 3): NUL-ended records, a rename's or a
/// copy's origin in the field after it. A record this cannot read is
/// refused with its bytes, never skipped.
pub(super) fn changes(root: &Path, status: &[u8]) -> Result<Vec<Change>, FolderError> {
    let unread = |record: &[u8]| unread(root, "status", record);
    let mut fields = status
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    let mut found = Vec::new();
    while let Some(record) = fields.next() {
        let change = Change::read(record).ok_or_else(|| unread(record))?;
        if change.renamed() && fields.next().is_none() {
            return Err(unread(record));
        }
        found.push(change);
    }
    Ok(found)
}

/// The lines under a refusal that names paths, each on its own line.
pub(super) fn listed_paths<T: fmt::Display>(paths: &[T]) -> String {
    paths.iter().map(|path| format!("\n  {path}")).collect()
}
