//! Where an adopter's Brokkr lives (decision 0083): one `.brokkr/` folder
//! at its repository's root. A verb walks up from its working directory to
//! that root, refuses a folder it cannot read, tells a recipe library from
//! an adopter, refuses an old layout with the moves it lists, and takes
//! each fact it defaults from here, in ruling 2's order: its flag, then
//! for the journal the map's, then the folder's place, then a library's.
//!
//! Every fact of a layout is the runtime's one judgement,
//! [`brokkr_runtime::places::Layout`], which the launch, the queue and the
//! engine take again from the same workspace; this module holds only the
//! CLI's own: the flags' defaults, the refusals' words and the moves an
//! old layout lists. The hidden verbs a seat or a box runs (`driver`,
//! `hands`, `broker`, `fake-driver` and the dsh sandbox runner) never
//! come here.

use std::fmt;
use std::path::{Path, PathBuf};

use brokkr_runtime::places::{self, Kind, Old, Places, PlacesError, BUNDLE, FOLDER, JOURNAL};
use brokkr_runtime::places::{LIBRARY_STATE, SECRETS, STATE};
use tracked::{listed_paths, tracked, Change, Unwatched};

/// Why a verb refused before it opened anything, in the operator's words.
#[derive(Debug)]
pub(crate) enum FolderError {
    /// What the runtime's one judgement of a layout refuses: a `.brokkr`
    /// below the root or one that is not a directory, a workspace, root
    /// or library mark that cannot be read, or a state fact with no place.
    Places(PlacesError),
    /// A run verb with no bundle named and no folder to supply one.
    NoBundle { cwd: PathBuf, root: PathBuf },
    /// An old layout, with the moves it lists.
    Old(OldLayout),
    /// An entry that cannot be read, refused rather than guessed at.
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// An old layout's `.forge` that is not a directory, whose entries no
    /// move could take.
    ForgeNotADirectory { path: PathBuf },
    /// A symbolic link at or under the old layout's `.forge`, the first in
    /// sorted walk order: the moves list nothing through or for one (the
    /// operator's ruling of 2026-10-14).
    Link { path: PathBuf },
    /// A repository whose tracked `.forge/` entries git would not list,
    /// or whose changes to them git would not state, so no move could say
    /// which go by `git mv`. `verb` is what git was asked.
    Untracked {
        verb: &'static str,
        root: PathBuf,
        said: String,
    },
    /// Tracked paths under `.forge` whose index or work tree differs from
    /// the last commit, which `git mv` or `git rm` would refuse or leave
    /// half-moved, each with its two-letter state.
    Uncommitted { root: PathBuf, changes: Vec<Change> },
    /// Tracked paths under `.forge` git does not read in the work tree, a
    /// sparse checkout's or a skip-worktree or assume-unchanged bit's,
    /// whose changes `git status` never states, each with its tag.
    Unwatched {
        root: PathBuf,
        paths: Vec<Unwatched>,
    },
    /// A sparse checkout beside entries git tracks under `.forge`: a listed
    /// `git mv` into `.brokkr/state` could fall outside its definition.
    Sparse { root: PathBuf },
    /// A submodule git tracks under `.forge`: `git mv` moves it only with `.gitmodules`.
    TrackedSubmodule { path: PathBuf },
    /// A `.forge/` entry whose name is not UTF-8, which no listed move
    /// could name.
    NotUtf8 { path: PathBuf },
    /// A `.forge/` entry whose name holds a control character, which no
    /// listed move could name on one line.
    ControlCharacter { path: PathBuf },
    /// The old layout's ignore entry that is not a file, which no listed
    /// `rm` could remove; a link there refuses as [`FolderError::Link`].
    IgnoreNotAFile { path: PathBuf },
}

impl From<PlacesError> for FolderError {
    fn from(refused: PlacesError) -> FolderError {
        FolderError::Places(refused)
    }
}

/// The refusal of the entry at `path`, which cannot be read.
fn unreadable(path: &Path) -> impl FnOnce(std::io::Error) -> FolderError + '_ {
    move |source| FolderError::Unreadable {
        path: path.to_path_buf(),
        source,
    }
}

impl fmt::Display for FolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FolderError::Places(refused) => write!(f, "{refused}"),
            FolderError::NoBundle { cwd, root } => write!(
                f,
                "no {FOLDER}/ folder from {} up to {} (decision 0083), so there is no default \
                 bundle: name one with --bundle or --recipe, or run `brokkr init .` from {}",
                cwd.display(),
                root.display(),
                root.display()
            ),
            FolderError::Old(old) => write!(f, "{old}"),
            FolderError::Unreadable { path, source } => {
                write!(f, "reading {}: {source}", path.display())
            }
            FolderError::ForgeNotADirectory { path } => write!(
                f,
                "{} is a file, not Brokkr's state directory: move it aside",
                path.display()
            ),
            FolderError::Link { path } => write!(
                f,
                "{} is a symbolic link, and Brokkr lists no moves through or for one (decision \
                 0083; the operator's ruling of 2026-10-14); nothing was opened: move the state \
                 into {FOLDER}/{STATE} yourself, then run again",
                path.display()
            ),
            FolderError::Untracked { verb, root, said } => write!(
                f,
                "git {verb} in {} failed, so the moves cannot tell what git tracks (decision \
                 0083); nothing was opened: {said}",
                root.display()
            ),
            FolderError::Uncommitted { root, changes } => write!(
                f,
                "{}/{LIBRARY_STATE} holds paths git tracks whose index or work tree differs from \
                 the last commit, which `git mv` or `git rm` would refuse or leave half-moved, so \
                 no moves are listed (decision 0083); nothing was opened: commit or restore these, \
                 then run again{}",
                root.display(),
                listed_paths(changes)
            ),
            FolderError::Unwatched { root, paths } => write!(
                f,
                "{}/{LIBRARY_STATE} holds paths git tracks but does not read in the work tree, so \
                 `git status` cannot say whether `git mv` or `git rm` would complete, and `git rm` \
                 refuses a path a sparse checkout leaves out; no moves are listed (decision 0083); \
                 nothing was opened: run `git sparse-checkout disable`, or `git update-index \
                 --no-skip-worktree --no-assume-unchanged` on each, then run again{}",
                root.display(),
                listed_paths(paths)
            ),
            FolderError::Sparse { root } => write!(
                f,
                "{} has a sparse checkout and git tracks entries under {LIBRARY_STATE}, so a listed \
                 `git mv` into {FOLDER}/{STATE} could fall outside its definition and fail; no \
                 moves are listed (decision 0083); nothing was opened: run `git sparse-checkout \
                 disable`, or move the tracked entries yourself, then run again",
                root.display()
            ),
            FolderError::TrackedSubmodule { path } => write!(
                f,
                "{} is a submodule git tracks, which `git mv` moves only together with \
                 .gitmodules, so no moves are listed (decision 0083); nothing was opened: move it \
                 out of {LIBRARY_STATE} and commit, then run again",
                path.display()
            ),
            FolderError::NotUtf8 { path } => write!(
                f,
                "{} is not named in UTF-8, so no listed move could name it (decision 0083); \
                 nothing was opened: rename it",
                path.display()
            ),
            FolderError::ControlCharacter { path } => write!(
                f,
                "{} holds a control character, so no listed move could name it on one line \
                 (decision 0083); nothing was opened: rename it",
                path.display().to_string().escape_debug()
            ),
            FolderError::IgnoreNotAFile { path } => write!(
                f,
                "{} is neither a file nor a link, so no listed move could remove it (decision \
                 0083); nothing was opened: move it aside",
                path.display()
            ),
        }
    }
}

impl std::error::Error for FolderError {}

/// The three old shapes of ruling 4, each with what it lists.
#[derive(Debug)]
pub(crate) enum OldLayout {
    /// No folder, and a root `bundle.json`: `init`'s old output.
    Root { moves: Vec<String> },
    /// No folder, no root `bundle.json`, and a root `.forge` entry.
    StateOnly { moves: Vec<String> },
    /// A folder, and Brokkr entries still at its root.
    HalfMoved {
        root: PathBuf,
        left: Vec<PathBuf>,
        both_journals: bool,
    },
}

/// The words an old shape opens with (ruling 4), in the one place both
/// its refusal and doctor's folder line read them (ruling 5).
#[derive(Debug, Clone, Copy)]
pub(crate) enum Opening<'a> {
    Root,
    StateOnly,
    /// An entry left at the root beside the folder: the clause that
    /// follows the words naming the folder.
    Left(&'a Path),
}

impl fmt::Display for Opening<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (has, moves) = (
            "this repository has Brokkr",
            "nothing was opened, and Brokkr moves",
        );
        match self {
            Opening::Root => write!(
                f,
                "{has}'s old root layout and no {FOLDER}/ folder (decision 0083); {moves} nothing"
            ),
            Opening::StateOnly => write!(
                f,
                "{has} state in {LIBRARY_STATE}/ and no {FOLDER}/ folder (decision 0083); {moves} \
                 nothing"
            ),
            Opening::Left(path) => write!(
                f,
                "{} still stands at the root (decision 0083); move or remove it",
                path.display()
            ),
        }
    }
}

impl fmt::Display for OldLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OldLayout::Root { moves } => write!(
                f,
                "{}{}\nthen move {BUNDLE} and every file it reads into {FOLDER}/ at the same \
                 relative paths, and in {FOLDER}/realms.json name the journal \
                 \"{STATE}/{JOURNAL}\" and the realm path \"..\"\nBrokkr reads no script or \
                 charter: edit any path one names under {LIBRARY_STATE}/ to name the same file \
                 after the move",
                Opening::Root,
                listed(moves)
            ),
            OldLayout::StateOnly { moves } => write!(
                f,
                "{}{}\nthen brokkr init . writes the scaffold beside the moved state",
                Opening::StateOnly,
                listed(moves)
            ),
            OldLayout::HalfMoved {
                root,
                left,
                both_journals,
            } => {
                let lines = left.iter().map(|path| {
                    format!(
                        "{}/{FOLDER}/ is this repository's folder, but {}",
                        root.display(),
                        Opening::Left(path)
                    )
                });
                write!(f, "{}", lines.collect::<Vec<String>>().join("\n"))?;
                match both_journals {
                    true => write!(
                        f,
                        "\nboth {}/{LIBRARY_STATE}/{JOURNAL} and {}/{FOLDER}/{STATE}/{JOURNAL} hold \
                         a journal, and journals never merge (decision 0026 ruling 5): keep one",
                        root.display(),
                        root.display()
                    ),
                    false => Ok(()),
                }
            }
        }
    }
}

/// The list under a refusal's first line: advice, never a contract.
fn listed(moves: &[String]) -> String {
    let head = "\nmake these moves, then run `brokkr doctor` to check the result:";
    moves
        .iter()
        .fold(head.to_string(), |list, line| list + "\n  " + line)
}

/// A verb's discovered layout: the runtime's one judgement of it
/// ([`places::Layout`]), which every dispatch from the same workspace
/// takes again, and the defaults the CLI's flags take from it.
#[derive(Debug)]
pub(crate) struct Layout(places::Layout);

impl Layout {
    /// The runtime's walk from `workspace` to its repository's root,
    /// which refuses a nested or unreadable folder before anything is
    /// opened ([`places::Layout::discover`]).
    pub(crate) fn discover(workspace: &Path) -> Result<Layout, FolderError> {
        Ok(Layout(places::Layout::discover(workspace)?))
    }

    /// The tree a verb's defaults resolve against: the root, where the
    /// folder is, else the workspace as handed.
    pub(crate) fn workspace(&self) -> PathBuf {
        self.0.workspace()
    }

    /// A path a flag names, read as typed: against the directory the verb
    /// was started in (ruling 2's step 1). Where the folder makes the root
    /// the workspace, a relative one is made absolute here, because the
    /// launch, the queue and the map read a relative path against the
    /// workspace; a library's stays as typed, byte for byte.
    pub(crate) fn typed(&self, path: PathBuf) -> PathBuf {
        match self.0.kind() {
            Kind::Folder => self.0.cwd().join(path),
            Kind::Library | Kind::Bare => path,
        }
    }

    /// The repository a run operates when `--repo` names none: the root,
    /// from wherever in it the run is started, where the folder is.
    pub(crate) fn operated(&self) -> Option<PathBuf> {
        (self.0.kind() == Kind::Folder).then(|| self.0.root().to_path_buf())
    }

    /// Ruling 4: an old layout is refused, with the moves to make. A
    /// recipe library keeps its layout and is never refused (the
    /// operator's ruling of 2026-10-13).
    pub(crate) fn refuse_old(&self) -> Result<(), FolderError> {
        self.old()?.map_or(Ok(()), |old| Err(FolderError::Old(old)))
    }

    /// The old shape the root holds, if any, by the runtime's judgement
    /// ([`places::Layout::old`]), with the moves it lists.
    pub(crate) fn old(&self) -> Result<Option<OldLayout>, FolderError> {
        let root = self.0.root();
        Ok(match self.0.old()? {
            None => None,
            Some(Old::Root) => Some(OldLayout::Root {
                moves: state_moves(root)?,
            }),
            Some(Old::StateOnly) => Some(OldLayout::StateOnly {
                moves: state_moves(root)?,
            }),
            Some(Old::HalfMoved {
                left,
                both_journals,
            }) => Some(OldLayout::HalfMoved {
                root: root.to_path_buf(),
                left,
                both_journals,
            }),
        })
    }

    /// The run-local state directory: the folder's, `None` for a library's
    /// own `.forge/`, or no place at all (ruling 2's step 4), by the
    /// runtime's one rule ([`places::Layout::state`]), which a dispatch
    /// asks again. `escape` is `--db` given: in an old layout the state
    /// then takes the library column, which there is that layout's own
    /// `.forge/`.
    pub(crate) fn state(&self, escape: bool) -> Result<Option<PathBuf>, FolderError> {
        Ok(self.0.state(escape)?)
    }

    /// A state file's default: `name` in the folder's state, else in the
    /// library's own `.forge/`, as it always was.
    pub(crate) fn state_file(&self, escape: bool, name: &str) -> Result<PathBuf, FolderError> {
        Ok(in_state(self.state(escape)?, name))
    }

    /// The implicit bundle (ruling 3): the folder itself. With none there
    /// is no default bundle; a verb asks [`Layout::state`] first, so
    /// outside a library the no-place text answers before this one.
    pub(crate) fn bundle(&self) -> Result<PathBuf, FolderError> {
        match self.0.kind() {
            Kind::Folder => Ok(self.0.root().join(FOLDER)),
            Kind::Library | Kind::Bare => Err(FolderError::NoBundle {
                cwd: self.0.cwd().to_path_buf(),
                root: self.0.root().to_path_buf(),
            }),
        }
    }

    /// The agent library: the flag, else the folder's, else `agents`.
    pub(crate) fn agents(&self, flag: Option<PathBuf>) -> PathBuf {
        flag.or_else(|| self.folder_places().map(|places| places.agents))
            .unwrap_or_else(|| brokkr_runtime::bundle::DEFAULT_AGENTS_DIR.into())
    }

    /// The adapters: the flag, else the folder's, else `adapters`.
    pub(crate) fn adapters(&self, flag: Option<PathBuf>) -> PathBuf {
        flag.or_else(|| self.folder_places().map(|places| places.adapters))
            .unwrap_or_else(|| brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR.into())
    }

    /// The recipe library a verb reads named recipes from: the flag, else
    /// the folder's shelf ([`Layout::shelf`]), else `recipes`.
    pub(crate) fn recipes(&self, flag: Option<PathBuf>) -> PathBuf {
        flag.or_else(|| self.shelf())
            .unwrap_or_else(|| "recipes".into())
    }

    /// The folder's shelf of named recipes, `.brokkr/recipes` (#696), where
    /// the folder is.
    pub(crate) fn shelf(&self) -> Option<PathBuf> {
        self.folder_places().map(|places| places.recipes)
    }

    fn folder_places(&self) -> Option<Places> {
        (self.0.kind() == Kind::Folder).then(|| self.0.places())
    }
}

/// The defaults `brokkr doctor` reads. Doctor reports and never refuses
/// (decision 0046's Addendum): a journal with no place is reported as
/// `none`, with the refusal a run would give, and nothing is created
/// (ruling 5). The secrets store, which doctor only reads, and the
/// read-only facts take the library column where discovery fails, and
/// the folder line, the report's first, names why from `found`.
pub(crate) struct DoctorDefaults {
    pub(crate) workspace: PathBuf,
    /// The layout discovery found, else its refusal.
    pub(crate) found: Result<Found, FolderError>,
    pub(crate) agents: PathBuf,
    pub(crate) adapters: PathBuf,
    pub(crate) secrets: PathBuf,
}

/// A layout discovery found, as the runtime judged it, and its journal's
/// place, or the refusal a run would give for having none.
pub(crate) struct Found {
    pub(crate) layout: places::Layout,
    journal: Result<PathBuf, FolderError>,
}

impl DoctorDefaults {
    /// The defaults for a doctor run in `cwd`, `secrets` its flag.
    pub(crate) fn of(cwd: PathBuf, secrets: Option<PathBuf>) -> DoctorDefaults {
        let found = Layout::discover(&cwd);
        let layout = found.as_ref().ok();
        let state = layout.and_then(|layout| layout.state(false).ok().flatten());
        DoctorDefaults {
            secrets: secrets.unwrap_or_else(|| in_state(state, SECRETS)),
            agents: layout.map_or_else(
                || brokkr_runtime::bundle::DEFAULT_AGENTS_DIR.into(),
                |layout| layout.agents(None),
            ),
            adapters: layout.map_or_else(
                || brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR.into(),
                |layout| layout.adapters(None),
            ),
            workspace: layout.map_or(cwd, Layout::workspace),
            found: found.map(|layout| Found {
                journal: layout.state_file(false, JOURNAL),
                layout: layout.0,
            }),
        }
    }

    /// The journal every other verb would open, or why it has none:
    /// discovery's refusal, or the found layout's.
    pub(crate) fn journal(&self) -> Result<PathBuf, &FolderError> {
        match &self.found {
            Ok(found) => found.journal.as_ref().cloned(),
            Err(refused) => Err(refused),
        }
    }
}

/// `name` in the folder's `state`, else in a library's `.forge/`.
fn in_state(state: Option<PathBuf>, name: &str) -> PathBuf {
    state.unwrap_or_else(|| LIBRARY_STATE.into()).join(name)
}

/// The run-local directory a verb writes beside the repository `repo`
/// names (ruling 3): the folder's `state/` of the repository `repo`
/// stands in, from wherever in it, else a library's own `.forge/` under
/// `repo`, as the engine places a library run's. A layout that cannot be
/// read refuses, and a repository with neither a folder, a library mark
/// nor an old layout has no place for it (ruling 2's step 4).
///
/// Its one caller, `brokkr ledger --repo`, reads an old layout's root
/// only by ruling 4's `--db` escape, as a run's ship seat does before the
/// move: in the workspace's own repository the verb's check refuses an
/// old layout without `--db` before this is reached. So the escape is
/// taken here, and an old layout's `.forge/` is its own place.
pub(crate) fn run_local(repo: &Path) -> Result<PathBuf, FolderError> {
    let state = Layout::discover(repo)?.state(true)?;
    Ok(brokkr_runtime::places::run_local(state.as_deref(), repo))
}

/// What stands at `path`, by the runtime's one rule
/// ([`brokkr_runtime::places::entry`]). Anything that cannot be read
/// refuses.
fn entry(path: &Path) -> Result<Option<std::fs::Metadata>, FolderError> {
    brokkr_runtime::places::entry(path).map_err(unreadable(path))
}

/// The state's moves (ruling 4's third group): every `.forge/` entry to
/// `.brokkr/state/`, exactly one line each, a journal's `-wal` and `-shm`
/// beside it whatever the journal is named, then the old ignore file out
/// and the folder's own in. Each entry git tracks moves by `git mv` and goes by
/// `git rm`, each untracked one by `mv` and `rm`, so a journal and its
/// logs move whole however git holds them. The compile-derived moves are
/// not listed (the operator's ruling of 2026-10-13). The first thing
/// decided is the one link rule ([`no_link`]): a `.forge` that is a link,
/// or any link under it, lists no moves at all (the operator's ruling of
/// 2026-10-14). A `.forge` that is not a directory, entries that cannot
/// be listed or named, an ignore entry no `rm` could remove, a repository
/// git will not list or state, a submodule git tracks, and a tracked path
/// git holds a change to or does not read in the work tree refuse by
/// name, with no list at all, so every list printed completes when
/// executed (round 634k).
fn state_moves(root: &Path) -> Result<Vec<String>, FolderError> {
    no_link(&root.join(LIBRARY_STATE))?;
    let forge = Forge::at(root)?;
    let (names, tracked) = (forge_entries(root, forge)?, tracked(root)?);
    let tracks = |name: &str| tracked.iter().any(|known| known == name.as_bytes());
    let mut moves = vec![Move::Mkdir];
    let bases = names
        .iter()
        .filter(|name| *name != IGNORE && !journal_log(name, &names));
    for name in bases {
        let members = ["", "-wal", "-shm"].map(|suffix| format!("{name}{suffix}"));
        for member in members.into_iter().filter(|member| names.contains(member)) {
            moves.push(match tracks(&member) {
                true => Move::GitMv(member),
                false => Move::Mv(member),
            });
        }
    }
    // A tracked ignore file is clean ([`tracked`]), and `git rm` takes the
    // directory it empties with it.
    match (tracks(IGNORE), ignore_stands(root, &names)?) {
        (true, _) => moves.push(Move::GitRm),
        (false, true) => moves.extend([Move::Rm].into_iter().chain(forge.removal())),
        (false, false) => moves.extend(forge.removal()),
    }
    moves.push(Move::Printf);
    Ok(moves.iter().flat_map(Move::lines).collect())
}

/// Each arm the state moves' list has, by the line it lists:
/// `brokkr_folder::the_listed_moves_empty_every_forge` reads this enum
/// and executes a shape that reaches every arm (round 634k).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Move {
    Mkdir,
    GitMv(String),
    Mv(String),
    GitRm,
    Rm,
    Rmdir,
    Printf,
}

impl Move {
    /// The lines this move lists, each one shell command or a note.
    fn lines(&self) -> Vec<String> {
        let moved = |verb: &str, member: &str| {
            let to = word(&format!("{FOLDER}/{STATE}/{member}"));
            format!("{verb} {} {to}", word(&forged(member)))
        };
        match self {
            Move::Mkdir => vec![format!("mkdir -p {FOLDER}/{STATE}")],
            Move::GitMv(member) => vec![moved("git mv", member)],
            Move::Mv(member) => vec![moved("mv", member)],
            Move::GitRm => vec![format!("git rm {}", forged(IGNORE))],
            Move::Rm => vec![format!("rm {}", forged(IGNORE))],
            Move::Rmdir => vec![format!("rmdir {LIBRARY_STATE}")],
            Move::Printf => vec![format!("printf '{STATE}/\\n' > {FOLDER}/{IGNORE}")],
        }
    }
}

/// The old layout's ignore file, and the folder's, whose one line `init`
/// writes.
pub(crate) const IGNORE: &str = ".gitignore";

/// Whether the old ignore file stands among `names`, the root's `.forge/`
/// entries, judged as the entry itself, as the root's `.forge` is
/// (ruling 4): a file goes by `rm` or `git rm`; a link has refused already
/// ([`no_link`]); anything else refuses by name, with no moves, because no
/// listed `rm` could remove it.
fn ignore_stands(root: &Path, names: &[String]) -> Result<bool, FolderError> {
    if !names.iter().any(|name| name == IGNORE) {
        return Ok(false);
    }
    let path = root.join(forged(IGNORE));
    let found = std::fs::symlink_metadata(&path).map_err(unreadable(&path))?;
    match found.is_file() {
        true => Ok(true),
        false => Err(FolderError::IgnoreNotAFile { path }),
    }
}

/// The one link rule (the operator's ruling of 2026-10-14): `forge`, the
/// root's `.forge`, and every entry under it at any depth, each read as
/// the entry itself and never through a link. A link anywhere refuses by
/// name, the first in sorted walk order, so the moves never carry one or
/// look through one. An entry or a directory that cannot be read refuses,
/// never skipped.
fn no_link(forge: &Path) -> Result<(), FolderError> {
    let (mut links, mut left) = (Vec::new(), vec![forge.to_path_buf()]);
    while let Some(path) = left.pop() {
        let Some(found) = entry(&path)? else {
            continue;
        };
        if found.is_symlink() {
            links.push(path);
        } else if found.is_dir() {
            for listed in std::fs::read_dir(&path).map_err(unreadable(&path))? {
                left.push(listed.map_err(unreadable(&path))?.path());
            }
        }
    }
    links
        .into_iter()
        .min()
        .map_or(Ok(()), |path| Err(FolderError::Link { path }))
}

/// What stands at the root's `.forge`, judged as the entry itself and
/// never through a link (ruling 4); a link has refused already
/// ([`no_link`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Forge {
    Absent,
    Directory,
}

impl Forge {
    /// The root's `.forge`; anything but a directory refuses by name,
    /// because no listed move could empty or remove it.
    fn at(root: &Path) -> Result<Forge, FolderError> {
        let path = root.join(LIBRARY_STATE);
        match entry(&path)? {
            None => Ok(Forge::Absent),
            Some(found) if found.is_dir() => Ok(Forge::Directory),
            Some(_) => Err(FolderError::ForgeNotADirectory { path }),
        }
    }

    /// The emptied `.forge`'s removal: nothing where none stands, else
    /// `rmdir`.
    fn removal(self) -> Option<Move> {
        match self {
            Forge::Absent => None,
            Forge::Directory => Some(Move::Rmdir),
        }
    }
}

/// `path` as one shell word, so a listed move can be pasted as it reads:
/// bare where every character is plain, else single-quoted.
fn word(path: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "._-/+=:,@%".contains(c);
    match path.chars().all(plain) {
        true => path.to_string(),
        false => format!("'{}'", path.replace('\'', r"'\''")),
    }
}

/// `name` as it stands under the root's `.forge/`.
fn forged(name: &str) -> String {
    format!("{LIBRARY_STATE}/{name}")
}

/// Whether `name` is a `-wal` or `-shm` that moves beside its journal, a
/// journal of any name, and not on its own: its base is among `names`,
/// is not the ignore file, and is no such log itself. So a log's own log,
/// `j-wal-wal`, moves on its own, and every entry is listed exactly once.
fn journal_log(name: &str, names: &[String]) -> bool {
    let listed = |base: &str| base != IGNORE && names.iter().any(|known| known == base);
    ["-wal", "-shm"]
        .iter()
        .filter_map(|suffix| name.strip_suffix(suffix))
        .any(|base| listed(base) && !journal_log(base, names))
}

/// The names directly under the root's `.forge/`, `forge` what stands
/// there, sorted. A `.forge` whose entries cannot be listed refuses by
/// name, never read as empty, and so does an entry no listed move could
/// name on one line ([`nameable`]).
fn forge_entries(root: &Path, forge: Forge) -> Result<Vec<String>, FolderError> {
    let path = root.join(LIBRARY_STATE);
    let mut names = Vec::new();
    if forge == Forge::Absent {
        return Ok(names);
    }
    for listed in std::fs::read_dir(&path).map_err(unreadable(&path))? {
        let name = listed.map_err(unreadable(&path))?.file_name();
        names.push(nameable(&path, name)?);
    }
    names.sort();
    Ok(names)
}

/// `name`, an entry of the `.forge` at `forge`, as a listed move names
/// it on one line: a name that is not UTF-8, or that holds a control
/// character such as a newline, a tab or a carriage return, refuses by
/// name.
fn nameable(forge: &Path, name: std::ffi::OsString) -> Result<String, FolderError> {
    let path = forge.join(&name);
    match name.into_string() {
        Err(_) => Err(FolderError::NotUtf8 { path }),
        Ok(name) if name.chars().any(char::is_control) => {
            Err(FolderError::ControlCharacter { path })
        }
        Ok(name) => Ok(name),
    }
}

pub(crate) mod admit;
mod tracked;

#[cfg(test)]
pub(crate) mod folder_tests;
