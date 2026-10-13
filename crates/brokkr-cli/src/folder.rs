//! Where an adopter's Brokkr lives (decision 0083): one `.brokkr/` folder
//! at its repository's root. A verb walks up from its working directory to
//! that root, refuses a folder it cannot read, tells a recipe library from
//! an adopter, refuses an old layout with the moves it lists, and takes
//! each fact it defaults from here, in ruling 2's order: its flag, then
//! for the journal the map's, then the folder's place, then a library's.
//!
//! The places a folder holds are derived once, by
//! [`brokkr_runtime::places::Places`], which the launch, the queue and the
//! engine re-derive from the root this hands them. The hidden verbs a
//! seat or a box runs (`driver`, `hands`, `broker`, `fake-driver` and the
//! dsh sandbox runner) never come here.

use std::fmt;
use std::path::{Path, PathBuf};

use brokkr_runtime::places::{library, Places, PlacesError, FOLDER, LIBRARY_STATE, SECRETS, STATE};

/// The old layout's signature at a repository root: `init`'s bundle.
const BUNDLE: &str = "bundle.json";

/// The journal's name in the folder's state.
pub(crate) const JOURNAL: &str = "forge.db";

/// Why a verb refused before it opened anything, in the operator's words.
#[derive(Debug)]
pub(crate) enum FolderError {
    /// A `.brokkr` entry the walk met below the root.
    Nested { path: PathBuf, root: PathBuf },
    /// A root `.brokkr` the runtime's one judgement refuses: not a
    /// directory, or not readable.
    Places(PlacesError),
    /// A state fact with no place: no folder, and no library.
    NoPlace { cwd: PathBuf, root: PathBuf },
    /// A run verb with no bundle named and no folder to supply one.
    NoBundle { cwd: PathBuf, root: PathBuf },
    /// An old layout, with the moves it lists.
    Old(OldLayout),
    /// An entry that cannot be read, refused rather than guessed at.
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for FolderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FolderError::Nested { path, root } => write!(
                f,
                "{}/{FOLDER} is not at this repository's root {} (decision 0083); Brokkr reads \
                 only {}/{FOLDER}: move or remove it",
                path.display(),
                root.display(),
                root.display()
            ),
            FolderError::Places(refused) => write!(f, "{refused}"),
            FolderError::NoPlace { cwd, root } => write!(
                f,
                "no {FOLDER}/ folder from {} up to {}, and {} is not a recipe library (decision \
                 0083); nothing was created: run `brokkr init .` from {}",
                cwd.display(),
                root.display(),
                root.display(),
                root.display()
            ),
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

/// What the root holds, as ruling 2 tells it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `<root>/.brokkr` is a directory, and the root is no library.
    Folder,
    /// A `recipes/<name>/bundle.json` at the root, which keeps its own
    /// layout whatever else stands there (the operator's ruling 1).
    Library,
    /// Neither.
    Bare,
}

/// A verb's discovered layout: its working directory, the repository
/// root the walk reached, and what that root holds.
#[derive(Debug)]
pub(crate) struct Layout {
    cwd: PathBuf,
    root: PathBuf,
    /// The workspace as the verb was handed it, which a library's facts
    /// still resolve against exactly as they always did.
    workspace: PathBuf,
    kind: Kind,
}

impl Layout {
    /// Walk up from `workspace` to the repository root, the first
    /// directory holding a `.git` entry, or `workspace` itself outside any
    /// repository; refuse a `.brokkr` below the root, and a root `.brokkr`
    /// that is not a directory, before anything is opened. The walk is
    /// over the absolute path as the process names it, as git's own is.
    pub(crate) fn discover(workspace: &Path) -> Result<Layout, FolderError> {
        let cwd = std::path::absolute(workspace).map_err(|source| FolderError::Unreadable {
            path: workspace.to_path_buf(),
            source,
        })?;
        let mut root = cwd.clone();
        for dir in cwd.ancestors() {
            if entry(&dir.join(".git"))?.is_some() {
                root = dir.to_path_buf();
                break;
            }
        }
        for dir in cwd.ancestors().take_while(|dir| *dir != root) {
            if entry(&dir.join(FOLDER))?.is_some() {
                let path = dir.to_path_buf();
                return Err(FolderError::Nested { path, root });
            }
        }
        Layout::at(cwd, root, workspace)
    }

    /// The layout of `root` itself, judged with no walk: what `init`
    /// asks of the directory it writes into.
    pub(crate) fn of_root(root: &Path) -> Result<Layout, FolderError> {
        let root = root.to_path_buf();
        Layout::at(root.clone(), root.clone(), &root)
    }

    fn at(cwd: PathBuf, root: PathBuf, workspace: &Path) -> Result<Layout, FolderError> {
        // The runtime's one judgement of the root's `.brokkr`, which the
        // launch, the queue and the engine re-derive at dispatch: a
        // library outranks a folder beside it, so its journal never moves
        // to a fresh one under the folder.
        let found = brokkr_runtime::places::folder(&root).map_err(FolderError::Places)?;
        let kind = match found {
            Some(_) => Kind::Folder,
            None if library(&root) => Kind::Library,
            None => Kind::Bare,
        };
        Ok(Layout {
            cwd,
            root,
            workspace: workspace.to_path_buf(),
            kind,
        })
    }

    /// The tree a verb's defaults resolve against: the root, where the
    /// folder is, else the workspace as handed.
    pub(crate) fn workspace(&self) -> PathBuf {
        match self.kind {
            Kind::Folder => self.root.clone(),
            Kind::Library | Kind::Bare => self.workspace.clone(),
        }
    }

    /// A path a flag names, read as typed: against the directory the verb
    /// was started in (ruling 2's step 1). Where the folder makes the root
    /// the workspace, a relative one is made absolute here, because the
    /// launch, the queue and the map read a relative path against the
    /// workspace; a library's stays as typed, byte for byte.
    pub(crate) fn typed(&self, path: PathBuf) -> PathBuf {
        match self.kind {
            Kind::Folder => self.cwd.join(path),
            Kind::Library | Kind::Bare => path,
        }
    }

    /// The repository a run operates when `--repo` names none: the root,
    /// from wherever in it the run is started, where the folder is.
    pub(crate) fn operated(&self) -> Option<PathBuf> {
        (self.kind == Kind::Folder).then(|| self.root.clone())
    }

    /// Ruling 4: an old layout is refused, with the moves to make. A
    /// recipe library keeps its layout and is never refused (the
    /// operator's ruling of 2026-10-13).
    pub(crate) fn refuse_old(&self) -> Result<(), FolderError> {
        self.old()?.map_or(Ok(()), |old| Err(FolderError::Old(old)))
    }

    /// The old shape the root holds, if any.
    pub(crate) fn old(&self) -> Result<Option<OldLayout>, FolderError> {
        let (bundle, forge) = (self.stands(BUNDLE)?, self.stands(LIBRARY_STATE)?);
        Ok(match self.kind {
            Kind::Library => None,
            Kind::Bare if bundle => Some(OldLayout::Root {
                moves: state_moves(&self.root, forge),
            }),
            Kind::Bare if forge => Some(OldLayout::StateOnly {
                moves: state_moves(&self.root, forge),
            }),
            Kind::Bare => None,
            Kind::Folder => self.half_moved(bundle, forge)?,
        })
    }

    /// The half-moved shape: each Brokkr entry left beside the folder.
    fn half_moved(&self, bundle: bool, forge: bool) -> Result<Option<OldLayout>, FolderError> {
        let left: Vec<PathBuf> = [(bundle, BUNDLE), (forge, LIBRARY_STATE)]
            .into_iter()
            .filter(|(stands, _)| *stands)
            .map(|(_, name)| self.root.join(name))
            .collect();
        let both_journals = self.stands(&format!("{LIBRARY_STATE}/{JOURNAL}"))?
            && self.stands(&format!("{FOLDER}/{STATE}/{JOURNAL}"))?;
        Ok((!left.is_empty()).then(|| OldLayout::HalfMoved {
            root: self.root.clone(),
            left,
            both_journals,
        }))
    }

    /// The run-local state directory: the folder's, `None` for a library's
    /// own `.forge/`, or no place at all (ruling 2's step 4). `escape` is
    /// `--db` given: in an old layout the state then takes the library
    /// column, which there is that layout's own `.forge/`.
    pub(crate) fn state(&self, escape: bool) -> Result<Option<PathBuf>, FolderError> {
        match self.kind {
            Kind::Folder => Ok(Places::in_folder(&self.root.join(FOLDER)).state),
            Kind::Library => Ok(None),
            Kind::Bare if escape && self.holds_old()? => Ok(None),
            Kind::Bare => Err(FolderError::NoPlace {
                cwd: self.cwd.clone(),
                root: self.root.clone(),
            }),
        }
    }

    /// Whether the root holds a root layout's or a state-only layout's
    /// entry: what the `--db` escape reads as that layout's own.
    fn holds_old(&self) -> Result<bool, FolderError> {
        Ok(self.stands(BUNDLE)? || self.stands(LIBRARY_STATE)?)
    }

    /// Whether anything stands at `name` under the root, link or not.
    fn stands(&self, name: &str) -> Result<bool, FolderError> {
        entry(&self.root.join(name)).map(|found| found.is_some())
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
        match self.kind {
            Kind::Folder => Ok(self.root.join(FOLDER)),
            Kind::Library | Kind::Bare => Err(FolderError::NoBundle {
                cwd: self.cwd.clone(),
                root: self.root.clone(),
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
        (self.kind == Kind::Folder).then(|| Places::in_folder(&self.root.join(FOLDER)))
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
            workspace: layout.map_or(cwd, |layout| match layout.kind {
                Kind::Folder => layout.root,
                Kind::Library | Kind::Bare => layout.workspace,
            }),
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
    brokkr_runtime::places::entry(path).map_err(|source| FolderError::Unreadable {
        path: path.to_path_buf(),
        source,
    })
}

/// The state's moves (ruling 4's third group): every `.forge/` entry to
/// `.brokkr/state/`, one line each, a journal's `-wal` and `-shm` beside
/// it whatever the journal is named, then the old ignore file out and the
/// folder's own in. Each entry git tracks moves by `git mv` and goes by
/// `git rm`, each untracked one by `mv` and `rm`, so a journal and its
/// logs move whole however git holds them. A `.forge` that is a link is
/// judged as the link (ruling 4): its entries move out through it, and
/// the link itself goes by `rm`, never `rmdir`, leaving the directory it
/// names. The compile-derived moves are not listed (the operator's ruling
/// of 2026-10-13).
fn state_moves(root: &Path, forge: bool) -> Vec<String> {
    let (names, tracked) = (forge_entries(root), tracked(root));
    let mut moves = vec![format!("mkdir -p {FOLDER}/{STATE}")];
    let bases = names
        .iter()
        .filter(|name| *name != IGNORE && !journal_log(name, &names));
    for name in bases {
        let members = ["", "-wal", "-shm"].map(|suffix| format!("{name}{suffix}"));
        for member in members.iter().filter(|member| names.contains(member)) {
            let verb = match tracked.contains(member) {
                true => "git mv",
                false => "mv",
            };
            let to = word(&format!("{FOLDER}/{STATE}/{member}"));
            moves.push(format!("{verb} {} {to}", word(&forged(member))));
        }
    }
    let rmdir = removal(root);
    let ignore = (IGNORE.to_string(), forged(IGNORE));
    match (tracked.contains(&ignore.0), names.contains(&ignore.0)) {
        (true, _) => moves.push(format!("git rm {}", ignore.1)),
        (false, true) => moves.extend([vec![format!("rm {}", ignore.1)], rmdir].concat()),
        (false, false) if forge => moves.extend(rmdir),
        (false, false) => {}
    }
    moves.push(format!("printf '{STATE}/\\n' > {FOLDER}/{IGNORE}"));
    moves
}

/// The old layout's ignore file, and the folder's.
const IGNORE: &str = ".gitignore";

/// The emptied `.forge`'s removal: `rmdir`, or for a link, a note that
/// the link goes and the directory it names stays, then `rm`.
fn removal(root: &Path) -> Vec<String> {
    let linked = std::fs::symlink_metadata(root.join(LIBRARY_STATE))
        .is_ok_and(|entry| entry.file_type().is_symlink());
    match linked {
        true => vec![
            format!(
                "# {LIBRARY_STATE} is a link: rm removes the link, and the directory it names \
                 stays"
            ),
            format!("rm {LIBRARY_STATE}"),
        ],
        false => vec![format!("rmdir {LIBRARY_STATE}")],
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

/// Whether `name` is a `-wal` or `-shm` whose base is among `names`, so
/// it moves beside that base, a journal of any name, and not on its own.
fn journal_log(name: &str, names: &[String]) -> bool {
    ["-wal", "-shm"]
        .iter()
        .filter_map(|suffix| name.strip_suffix(suffix))
        .any(|journal| names.iter().any(|known| known == journal))
}

/// The names directly under the root's `.forge/`, sorted. A name that is
/// not UTF-8 is listed lossily: the list is advice.
fn forge_entries(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join(LIBRARY_STATE))
        .map(|entries| {
            let names = entries.flatten().map(|entry| entry.file_name());
            names
                .map(|name| name.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The names directly under `.forge/` that git tracks. A root git cannot
/// read tracks nothing: the list is advice, and `mv` is still a move.
fn tracked(root: &Path) -> Vec<String> {
    let listed = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--", LIBRARY_STATE])
        .output();
    let stdout = listed.map(|out| out.stdout).unwrap_or_default();
    let prefix = forged("");
    String::from_utf8_lossy(&stdout)
        .split('\0')
        .filter_map(|path| path.strip_prefix(&prefix))
        .map(|rest| rest.split('/').next().unwrap_or(rest).to_string())
        .collect()
}

#[cfg(test)]
pub(crate) mod folder_tests;
