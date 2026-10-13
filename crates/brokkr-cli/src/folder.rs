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
    /// An old layout's `.forge` that is neither a directory nor a link,
    /// whose entries no move could take.
    ForgeNotADirectory { path: PathBuf },
    /// A repository whose tracked `.forge/` entries git would not list,
    /// so no move could say which go by `git mv`.
    Untracked { root: PathBuf, said: String },
    /// A `.forge/` entry whose name is not UTF-8, which no listed move
    /// could name.
    NotUtf8 { path: PathBuf },
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
            FolderError::Untracked { root, said } => write!(
                f,
                "git ls-files in {} failed, so the moves cannot tell what git tracks (decision \
                 0083); nothing was opened: {said}",
                root.display()
            ),
            FolderError::NotUtf8 { path } => write!(
                f,
                "{} is not named in UTF-8, so no listed move could name it (decision 0083); \
                 nothing was opened: rename it",
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

impl fmt::Display for OldLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OldLayout::Root { moves } => write!(
                f,
                "this repository has Brokkr's old root layout and no {FOLDER}/ folder \
                 (decision 0083); nothing was opened, and Brokkr moves nothing{}\nthen move \
                 {BUNDLE} and every file it reads into {FOLDER}/ at the same relative paths, \
                 and in {FOLDER}/realms.json name the journal \"{STATE}/{JOURNAL}\" and the \
                 realm path \"..\"\nBrokkr reads no script or charter: edit any path one names \
                 under {LIBRARY_STATE}/ to name the same file after the move",
                listed(moves)
            ),
            OldLayout::StateOnly { moves } => write!(
                f,
                "this repository has Brokkr state in {LIBRARY_STATE}/ and no {FOLDER}/ folder \
                 (decision 0083); nothing was opened, and Brokkr moves nothing{}\nthen brokkr \
                 init . writes the scaffold beside the moved state",
                listed(moves)
            ),
            OldLayout::HalfMoved {
                root,
                left,
                both_journals,
            } => {
                let lines = left.iter().map(|path| {
                    format!(
                        "{}/{FOLDER}/ is this repository's folder, but {} still stands at the \
                         root (decision 0083); move or remove it",
                        root.display(),
                        path.display()
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

    /// The layout of `root` itself, judged with no walk: what `init`
    /// asks of the directory it writes into.
    pub(crate) fn of_root(root: &Path) -> Result<Layout, FolderError> {
        Ok(Layout(places::Layout::of_root(root)?))
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

    fn folder_places(&self) -> Option<Places> {
        (self.0.kind() == Kind::Folder).then(|| self.0.places())
    }
}

/// The defaults `brokkr doctor` reads. Doctor reports and never refuses
/// (decision 0046's Addendum): a journal with no place is reported as
/// `none`, with the refusal a run would give, and nothing is created
/// (ruling 5). The secrets store, which doctor only reads, and the
/// read-only facts take the library column where discovery fails; the
/// folder line that names why is slice B's (#634).
pub(crate) struct DoctorDefaults {
    pub(crate) workspace: PathBuf,
    pub(crate) journal: Result<PathBuf, FolderError>,
    pub(crate) agents: PathBuf,
    pub(crate) adapters: PathBuf,
    pub(crate) secrets: PathBuf,
}

impl DoctorDefaults {
    /// The defaults for a doctor run in `cwd`, `secrets` its flag.
    pub(crate) fn of(cwd: PathBuf, secrets: Option<PathBuf>) -> DoctorDefaults {
        let (journal, layout) = match Layout::discover(&cwd) {
            Ok(layout) => (layout.state_file(false, JOURNAL), Some(layout)),
            Err(refused) => (Err(refused), None),
        };
        let state = layout
            .as_ref()
            .and_then(|layout| layout.state(false).ok().flatten());
        DoctorDefaults {
            journal,
            secrets: secrets.unwrap_or_else(|| in_state(state, SECRETS)),
            agents: layout.as_ref().map_or_else(
                || brokkr_runtime::bundle::DEFAULT_AGENTS_DIR.into(),
                |layout| layout.agents(None),
            ),
            adapters: layout.as_ref().map_or_else(
                || brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR.into(),
                |layout| layout.adapters(None),
            ),
            workspace: layout.map_or(cwd, |layout| layout.workspace()),
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
/// logs move whole however git holds them. A `.forge` that is a link is
/// judged as the link (ruling 4): its entries move out through it, and
/// the link itself goes by `rm`, never `rmdir`, leaving the directory it
/// names. The compile-derived moves are not listed (the operator's ruling
/// of 2026-10-13). A `.forge` that is neither, entries that cannot be
/// listed or named, or a repository git will not list refuse by name,
/// with no list at all.
fn state_moves(root: &Path) -> Result<Vec<String>, FolderError> {
    let forge = Forge::at(root)?;
    let (names, tracked) = (forge_entries(root, forge)?, tracked(root)?);
    let tracks = |name: &str| tracked.iter().any(|known| known == name.as_bytes());
    let mut moves = vec![format!("mkdir -p {FOLDER}/{STATE}")];
    let bases = names
        .iter()
        .filter(|name| *name != IGNORE && !journal_log(name, &names));
    for name in bases {
        let members = ["", "-wal", "-shm"].map(|suffix| format!("{name}{suffix}"));
        for member in members.iter().filter(|member| names.contains(member)) {
            let verb = match tracks(member) {
                true => "git mv",
                false => "mv",
            };
            let to = word(&format!("{FOLDER}/{STATE}/{member}"));
            moves.push(format!("{verb} {} {to}", word(&forged(member))));
        }
    }
    let ignore = (IGNORE.to_string(), forged(IGNORE));
    match (tracks(IGNORE), names.contains(&ignore.0)) {
        (true, _) => {
            moves.extend([vec![format!("git rm {}", ignore.1)], forge.unlinked()].concat())
        }
        (false, true) => moves.extend([vec![format!("rm {}", ignore.1)], forge.removal()].concat()),
        (false, false) => moves.extend(forge.removal()),
    }
    moves.push(format!("printf '{STATE}/\\n' > {FOLDER}/{IGNORE}"));
    Ok(moves)
}

/// The old layout's ignore file, and the folder's.
const IGNORE: &str = ".gitignore";

/// What stands at the root's `.forge`, judged as the entry itself and
/// never through a link (ruling 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Forge {
    Absent,
    Directory,
    Link,
}

impl Forge {
    /// The root's `.forge`; anything but a directory or a link refuses by
    /// name, because no listed move could empty or remove it.
    fn at(root: &Path) -> Result<Forge, FolderError> {
        let path = root.join(LIBRARY_STATE);
        match entry(&path)? {
            None => Ok(Forge::Absent),
            Some(found) if found.is_dir() => Ok(Forge::Directory),
            Some(found) if found.is_symlink() => Ok(Forge::Link),
            Some(_) => Err(FolderError::ForgeNotADirectory { path }),
        }
    }

    /// The emptied `.forge`'s removal: nothing where none stands, `rmdir`,
    /// or for a link, a note that the link goes and the directory it names
    /// stays, then `rm`.
    fn removal(self) -> Vec<String> {
        match self {
            Forge::Absent => Vec::new(),
            Forge::Directory => vec![format!("rmdir {LIBRARY_STATE}")],
            Forge::Link => vec![
                format!(
                    "# {LIBRARY_STATE} is a link: rm removes the link, and the directory it \
                     names stays"
                ),
                format!("rm {LIBRARY_STATE}"),
            ],
        }
    }

    /// The removal after `git rm` takes the last entry: a directory git
    /// emptied is gone with that entry, and a link still stands.
    fn unlinked(self) -> Vec<String> {
        match self {
            Forge::Link => self.removal(),
            Forge::Absent | Forge::Directory => Vec::new(),
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
/// name, never read as empty, and so does an entry whose name is not
/// UTF-8, which no listed move could name.
fn forge_entries(root: &Path, forge: Forge) -> Result<Vec<String>, FolderError> {
    let path = root.join(LIBRARY_STATE);
    let mut names = Vec::new();
    if forge == Forge::Absent {
        return Ok(names);
    }
    for listed in std::fs::read_dir(&path).map_err(unreadable(&path))? {
        let name = listed.map_err(unreadable(&path))?.file_name();
        let not_utf8 = |name| FolderError::NotUtf8 {
            path: path.join(name),
        };
        names.push(name.into_string().map_err(not_utf8)?);
    }
    names.sort();
    Ok(names)
}

/// The names directly under `.forge/` that git tracks, as git's bytes.
/// Outside any repository git tracks nothing; inside one, a git that
/// cannot be run or will not list refuses by name, because an entry it
/// tracks moved by `mv` would leave git a deletion and a stranger.
fn tracked(root: &Path) -> Result<Vec<Vec<u8>>, FolderError> {
    let repository = brokkr_runtime::places::root(root).map_err(FolderError::Places)?;
    if entry(&repository.join(".git"))?.is_none() {
        return Ok(Vec::new());
    }
    let git = Path::new("git");
    let mut list = std::process::Command::new(git);
    list.arg("-C").arg(root);
    let listed = list.args(["ls-files", "-z", "--", LIBRARY_STATE]).output();
    let listed = listed.map_err(unreadable(git))?;
    if !listed.status.success() {
        let said = String::from_utf8_lossy(&listed.stderr)
            .trim_end()
            .to_string();
        let root = root.to_path_buf();
        return Err(FolderError::Untracked { root, said });
    }
    let prefix = forged("");
    Ok(listed
        .stdout
        .split(|byte| *byte == 0)
        .filter_map(|path| path.strip_prefix(prefix.as_bytes()))
        .map(|rest| {
            rest.split(|byte| *byte == b'/')
                .next()
                .unwrap_or(rest)
                .to_vec()
        })
        .collect())
}

#[cfg(test)]
pub(crate) mod folder_tests;
