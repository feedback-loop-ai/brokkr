//! `brokkr recipes` — the recipe library. A recipe is a bundle directory
//! (policy.json + bundle.json + roles/) treated as a named, swappable
//! delivery strategy: list what is installed, install one from a local
//! path or a git URL. Validation is the ordinary compile, unmodified — a
//! recipe that fails the constitutional lints is warned or rejected,
//! never repaired (decision 0001).
//!
//! Every compile here runs against the WORKSPACE's roots (decision 0023,
//! as `run`, `resume` and `recipes show` already do): since decision 0021
//! a compile reads the adapter data even for a recipe that names no
//! agent, and a listing that resolved one tree while compiling against
//! whichever directory the operator happened to stand in would report
//! working recipes as broken — and, in `add`, delete them for it. For the
//! same reason each reads the workspace map's provisional offices
//! (decision 0075 ruling 5).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use brokkr_runtime::launch::BundleSource;
use brokkr_runtime::places::{self, Kind, PlacesError, FOLDER};
use brokkr_runtime::realms::World;
use brokkr_runtime::{Bundle, SeatBody};

use crate::compile_in;

mod agents;
mod sources;

/// The run/resume bundle from the at-most-one-of `--bundle` / `--recipe`
/// pair (clap's arg group enforces the arity), else the `implicit` one a
/// discovered folder supplies (decision 0083 ruling 3). The launch
/// resolves it, where each verb always did ([`BundleSource::resolve`]).
pub(crate) fn source<E>(
    bundle: Option<PathBuf>,
    recipe: Option<String>,
    recipes_dir: PathBuf,
    implicit: impl FnOnce() -> Result<PathBuf, E>,
) -> Result<BundleSource, E> {
    match (bundle, recipe) {
        (Some(path), _) => Ok(BundleSource::Dir(path)),
        (None, Some(name)) => Ok(BundleSource::Recipe { name, recipes_dir }),
        (None, None) => implicit().map(BundleSource::Dir),
    }
}

/// Seat names in declared (sorted) order, panels rendered as
/// `review[correctness+security]`.
fn seat_summary(bundle: &Bundle) -> String {
    fn hire(body: &SeatBody) -> String {
        match body {
            SeatBody::Single {
                candidates,
                command,
                ..
            } => candidates
                .first()
                .map(|candidate| candidate.agent.clone())
                .unwrap_or_else(|| command.first().cloned().unwrap_or_else(|| "inline".into())),
            SeatBody::Panel { members, .. } => members
                .iter()
                .map(|member| {
                    member
                        .candidates
                        .first()
                        .map(|candidate| candidate.agent.as_str())
                        .unwrap_or(member.name.as_str())
                })
                .collect::<Vec<_>>()
                .join("+"),
            SeatBody::Sequence { steps } => steps
                .iter()
                .map(|step| step.name.as_str())
                .collect::<Vec<_>>()
                .join(">"),
            SeatBody::Select { cases, default, .. } => default
                .as_deref()
                .or_else(|| cases.values().next())
                .map(hire)
                .unwrap_or_else(|| "unresolved".into()),
        }
    }
    bundle
        .seats
        .iter()
        .map(|(name, seat)| match &seat.body {
            SeatBody::Single { .. } => name.clone(),
            SeatBody::Panel { members, .. } => format!(
                "{name}[{}]",
                members
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join("+")
            ),
            SeatBody::Sequence { steps } => format!(
                "{name}[{}]",
                steps
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(">")
            ),
            SeatBody::Select { cases, default, .. } => {
                let mut names = cases
                    .iter()
                    .map(|(case, body)| format!("{case}={}", hire(body)))
                    .collect::<Vec<_>>();
                if let Some(body) = default {
                    names.push(format!("default={}", hire(body)));
                }
                format!("{name}{{{}}}", names.join(";"))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// One line per recipe that compiles; a warning line per one that does
/// not. Nothing aborts the listing: a broken recipe is information. A
/// `dir` that cannot be read warns, unless it is the folder's default
/// `shelf` (#696) and nothing stands there ([`unshelved`]): no recipe is
/// on it yet.
pub(crate) fn list(workspace: &Path, dir: &Path, shelf: bool) -> Result<()> {
    let world = World::discover(workspace, None)?;
    let candidates = match listed(dir) {
        Ok(candidates) => candidates,
        Err(_) if shelf && unshelved(dir) => Vec::new(),
        Err(e) => {
            println!("warning: recipes dir {}: {e}", dir.display());
            Vec::new()
        }
    };
    for (name, path) in candidates {
        match compile_in(workspace, &path, world.as_ref()) {
            Ok(bundle) => println!(
                "{name}\t{}\t{} phases\t{}\t{}\t{}\t{}",
                &bundle.manifest_digest()[..12],
                bundle.machine.phases.len(),
                seat_summary(&bundle),
                bundle.cost,
                bundle.description,
                path.display()
            ),
            Err(e) => println!("warning: {name} ({}): {e}", path.display()),
        }
    }
    Ok(())
}

/// The recipes in the library `dir`, each directory in it with its name,
/// sorted: what `list` and doctor's shelf lines both read.
pub(crate) fn listed(dir: &Path) -> std::io::Result<Vec<(String, PathBuf)>> {
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();
    let named = subdirs.into_iter().map(|sub| {
        let name = sub
            .file_name()
            .expect("read_dir child path has a final component");
        (name.to_string_lossy().into_owned(), sub)
    });
    Ok(named.collect())
}

/// Whether nothing stands at the shelf `dir`, judged as the entry itself:
/// a shelf that is a link to nothing stands, so it warns as an unreadable
/// one does rather than reading as no shelf.
pub(crate) fn unshelved(dir: &Path) -> bool {
    matches!(places::entry(dir), Ok(None))
}

/// A source is a git URL by prefix (`http://`, `https://`, `git@`, and —
/// implementer's ruling on the framed ambiguity — `file://`, which can
/// never be a plain local path) or by the `.git` suffix; anything else
/// is a local path.
fn is_git_source(source: &str) -> bool {
    ["http://", "https://", "git@", "file://"]
        .iter()
        .any(|p| source.starts_with(p))
        || source.ends_with(".git")
}

/// The bundle root inside a clone: the clone root if it carries
/// bundle.json, else the single subdirectory that does.
fn bundle_root(clone: &Path) -> Result<PathBuf> {
    if clone.join("bundle.json").is_file() {
        return Ok(clone.to_path_buf());
    }
    let mut roots: Vec<PathBuf> = std::fs::read_dir(clone)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("bundle.json").is_file())
        .collect();
    roots.sort();
    match roots.len() {
        1 => Ok(roots.remove(0)),
        0 => bail!("clone has no bundle.json at its root or in any subdirectory"),
        n => bail!("clone has {n} subdirectories with a bundle.json; a recipe source must have exactly one"),
    }
}

/// Recursive copy, skipping `.git` so a cloned recipe lands as plain
/// reviewable files, not a nested repository. Symlinks are refused:
/// following one would copy whatever it points at (possibly outside the
/// source) into the committable library.
fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let source = entry.path();
        let dest = to.join(entry.file_name());
        if kind.is_symlink() {
            bail!(
                "refusing to copy symlink {}: a recipe must be plain files",
                source.display()
            );
        }
        if kind.is_dir() {
            if entry.file_name() == ".git" {
                continue;
            }
            copy_dir(&source, &dest)?;
        } else {
            std::fs::copy(&source, &dest)?;
        }
    }
    Ok(())
}

/// The recipes one `add` copied into the library, by name, and every other
/// entry it made, the agents and charters it copied (#696) and the
/// directories they needed, removed together when dropped unless kept — so
/// an install refused at any step leaves no partial copy squatting on a
/// name and making every retry fail with "already exists".
struct Installing(Vec<(String, PathBuf)>, Vec<PathBuf>);

impl Installing {
    /// The destination is created here, before it is registered, so an
    /// entry this install did not make — a dangling link included — fails
    /// the copy and is never removed with the copies that are.
    fn copy(&mut self, from: &Path, name: &str, dest: PathBuf) -> Result<()> {
        std::fs::create_dir(&dest)?;
        self.0.push((name.to_string(), dest.clone()));
        copy_dir(from, &dest)
    }

    fn keep(mut self) -> Vec<(String, PathBuf)> {
        self.1.clear();
        std::mem::take(&mut self.0)
    }
}

impl Drop for Installing {
    fn drop(&mut self) {
        for (_, dest) in &self.0 {
            let _ = std::fs::remove_dir_all(dest);
        }
        // Made in order, each directory before what it holds.
        for made in self.1.iter().rev() {
            let _ = std::fs::remove_file(made).or_else(|_| std::fs::remove_dir(made));
        }
    }
}

/// The library directory, made here when nothing stands there yet, and
/// removed when dropped if it still holds nothing, so an install refused
/// before any copy leaves the workspace as it found it. Its parent, the
/// root or its folder, stands, and is held ([`hold`]) from before the
/// library is made until after it is removed: its fields drop after it.
struct Shelf {
    made: Option<PathBuf>,
    _held: rustix::fd::OwnedFd,
}

impl Shelf {
    /// Hold the parent of `place`, where the library `dir` resolves, then
    /// make `dir` if nothing stands there.
    fn make(dir: &Path, place: &Path) -> std::io::Result<Shelf> {
        let _held = hold(place.parent().unwrap_or(place))?;
        let made = match std::fs::create_dir(dir) {
            Ok(()) => Some(dir.to_path_buf()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => None,
            Err(e) => return Err(e),
        };
        Ok(Shelf { made, _held })
    }
}

impl Drop for Shelf {
    fn drop(&mut self) {
        self.made
            .iter()
            .for_each(|dir| drop(std::fs::remove_dir(dir)));
    }
}

/// The base a recipe directory's `bundle.json` names in `extends`, when it
/// names a single plain path component, so no base is read from or copied
/// to anywhere but its library. This only plans the copies: the compile
/// that follows reads the chain strictly (decision 0017) and names why a
/// layer this skips does not compose.
fn base_of(layer: &Path) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Layer {
        extends: Option<String>,
    }
    let text = std::fs::read_to_string(layer.join("bundle.json")).ok()?;
    let base = serde_json::from_str::<Layer>(&text).ok()?.extends?;
    plain(Path::new(&base)).then_some(base)
}

/// Whether `part` is a single plain path component: no separator, `..`,
/// `.`, root or prefix, and not empty. A name joined under a directory
/// lands in it only when this holds. The spelling itself is judged, since
/// [`Path::components`] drops a trailing separator and a trailing `.`.
fn plain(part: &Path) -> bool {
    let spelling = part.as_os_str().as_encoded_bytes();
    let separated = spelling
        .iter()
        .any(|&byte| std::path::is_separator(byte.into()));
    !separated && !matches!(spelling, b"" | b"." | b"..")
}

/// The recipes an install of `root` as `<dir>/<name>` copies, each by name
/// with the directory it is copied from: the recipe, and each base its
/// `extends` chain reaches (decision 0017) that the library does not
/// already hold, under the name it is extended by: a base resolves from
/// the library the leaf sits in, so a derived recipe copied alone would
/// never compile there. The chain is walked a layer at a time against the
/// target, so it stops at the first base the library holds, whose own
/// chain is the library's, wherever the source's chain would end. A base
/// that neither the library nor the source's directory holds as a plain
/// directory ends the walk too, and the compile that follows names why
/// the install does not compose. The library holds a base it has any
/// entry for, a dangling link included, read as the source side is, and
/// every name planned before it. A base is found beside its layer, which
/// is what the compiler's authority, `places::base_library`, names for a
/// layer that is no folder bundle; a plan holding a folder bundle is
/// refused before any copy ([`sources::refuse_folder_bundles`]).
fn planned(root: &Path, name: &str, dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut planned = vec![(name.to_string(), root.to_path_buf())];
    let mut layer = root.canonicalize()?;
    while let Some(base) = base_of(&layer) {
        let source = layer.with_file_name(&base);
        let plain = std::fs::symlink_metadata(&source).is_ok_and(|meta| meta.is_dir());
        let held = std::fs::symlink_metadata(dir.join(&base)).is_ok()
            || planned.iter().any(|(known, _)| *known == base);
        if held || !plain {
            break;
        }
        planned.push((base, source.clone()));
        layer = source;
    }
    Ok(planned)
}

/// Copy each `planned` recipe into `<dir>/<name>`, in order.
fn install(planned: &[(String, PathBuf)], dir: &Path) -> Result<Installing> {
    let mut installing = Installing(Vec::new(), Vec::new());
    for (name, source) in planned {
        installing.copy(source, name, dir.join(name))?;
    }
    Ok(installing)
}

/// Hold a library exclusively, by a lock on its parent `dir`, which stands
/// while an `add` makes and removes the library, until the handle drops; a
/// second holder waits. An install exposes each base it copies at its
/// library name before its compile is accepted, and removes it when
/// refused, so another `add` that ran meanwhile could stop its walk at that
/// copy and keep a leaf whose base is then removed. One `add` at a time
/// keeps "every copy kept or none" true across invocations.
fn hold(dir: &Path) -> std::io::Result<rustix::fd::OwnedFd> {
    use rustix::fs::{flock, open, FlockOperation, Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let library = open(dir, flags, Mode::empty())?;
    flock(&library, FlockOperation::LockExclusive)?;
    Ok(library)
}

/// A copy `add` made that the compiler refused, after every copy was
/// removed: the recipe and the compile refusal itself, so a caller matches
/// those and never the text (decision 0071 ruling 8).
#[derive(Debug)]
struct Refused {
    recipe: String,
    cause: anyhow::Error,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Refused { recipe, cause } = self;
        write!(
            formatter,
            "recipe '{recipe}' does not compile (removed): {cause}"
        )
    }
}

impl std::error::Error for Refused {}

/// An `add` refused because its library resolves to `place`, inside the
/// folder repository at `root` but not its shelf (decision 0083; #675,
/// #696): there a `recipes/<name>/bundle.json` would be a recipe library's
/// mark, which outranks a folder beside it (the operator's ruling 1 of
/// 2026-10-13 on #634) and moves every default out of the folder
/// unannounced. A caller matches the type, never the text (decision 0071
/// ruling 8).
#[derive(Debug, PartialEq, Eq)]
struct InFolder {
    root: PathBuf,
    place: PathBuf,
}

impl std::fmt::Display for InFolder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (root, place) = (self.root.display(), self.place.display());
        write!(
            formatter,
            "recipes add would install into {place}, inside the folder repository {root} but \
             not its shelf {root}/{FOLDER}/recipes/ (decision 0083; #675, #696): a folder \
             repository keeps its named recipes on its shelf, so nothing was written"
        )
    }
}

impl std::error::Error for InFolder {}

/// An `add` refused because the library resolves to `place`, inside the
/// recipe at `source` it would copy (#696): a folder's shelf lies inside
/// the folder's own bundle, so the copy would hold itself. A caller
/// matches the type, never the text (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
struct Within {
    source: PathBuf,
    place: PathBuf,
}

impl std::fmt::Display for Within {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "recipes add would copy {} into {}, which lies inside it (#696): install a bundle \
             directory that does not hold the shelf; nothing was written",
            self.source.display(),
            self.place.display()
        )
    }
}

impl std::error::Error for Within {}

/// An `add` refused because a path it would write is not one it can judge
/// before writing it (#675): the place judged must be the place written.
/// A caller matches the variant, never the text (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
enum Unplain {
    /// `name` is not one plain path component ([`plain`]), so
    /// `<dir>/<name>` could land outside `dir`.
    Name { name: String, dir: PathBuf },
    /// `path` is not a relative path of plain components, so
    /// `<clone>/<path>` could lead outside the clone (#695).
    Path { path: String },
}

impl std::fmt::Display for Unplain {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unplain::Name { name, dir } => write!(
                formatter,
                "--name '{name}' is not one plain path component, so the recipe would not be one \
                 directory of {} (#675); nothing was written",
                dir.display()
            ),
            Unplain::Path { path } => write!(
                formatter,
                "--path '{path}' is not a relative path of plain names, so recipes add cannot \
                 judge where in the clone it leads (#695); nothing was cloned or written"
            ),
        }
    }
}

impl std::error::Error for Unplain {}

/// An `add --path` refused because the path does not lead, within the
/// clone, to a bundle (#695). Every path is the one `--path` spells, read
/// below the clone's root. A caller matches the variant, never the text
/// (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
enum AtPath {
    /// The source is a local directory, which names its bundle itself.
    Local { source: String, path: String },
    /// `link`, a part of `path`, is a symbolic link, which can name
    /// anything outside the clone.
    Link { path: String, link: PathBuf },
    /// The clone holds no directory at `missing`, a part of `path`.
    Missing { path: String, missing: PathBuf },
    /// The directory at `path` holds no `bundle.json` file.
    Unbundled { path: String },
}

impl std::fmt::Display for AtPath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtPath::Local { source, path } => write!(
                formatter,
                "--path names a directory inside a git source's clone, and {source} is a local \
                 directory: name {} as the source itself (#695); nothing was written",
                Path::new(source).join(path).display()
            ),
            AtPath::Link { path, link } => write!(
                formatter,
                "--path {path} passes through {}, a symbolic link in the clone, which can name \
                 anything outside it (#695); nothing was written",
                link.display()
            ),
            AtPath::Missing { path, missing } => write!(
                formatter,
                "--path {path}: the clone holds no directory at {} (#695); nothing was written",
                missing.display()
            ),
            AtPath::Unbundled { path } => write!(
                formatter,
                "--path {path}: the clone's {path} holds no bundle.json (#695); nothing was \
                 written"
            ),
        }
    }
}

impl std::error::Error for AtPath {}

/// Where `add` takes a recipe from: a local bundle directory, or a git URL
/// whose clone holds the bundle at `path` ([`reach`]) or, without one, where
/// [`bundle_root`] discovers it.
enum Source<'a> {
    Local(&'a Path),
    Git { url: &'a str, path: Option<&'a str> },
}

impl<'a> Source<'a> {
    /// Judge `source` and `path` before anything is cloned or written: a
    /// path must be relative, of plain names only, each judged by its
    /// spelling as `--name` is ([`plain`], so an empty part refuses a
    /// trailing or doubled separator), and only a git source takes one.
    fn judge(source: &'a str, path: Option<&'a str>) -> Result<Source<'a>> {
        let git = is_git_source(source);
        match path {
            Some(path)
                if !path
                    .split(std::path::is_separator)
                    .all(|p| plain(Path::new(p))) =>
            {
                Err(Unplain::Path { path: path.into() }.into())
            }
            Some(path) if !git => {
                let (source, path) = (source.into(), path.into());
                Err(AtPath::Local { source, path }.into())
            }
            _ if git => Ok(Source::Git { url: source, path }),
            _ => Ok(Source::Local(Path::new(source))),
        }
    }

    /// The bundle root, the source's agent library, if it has one: a recipe
    /// library's or a shelf's, local or cloned ([`agents::library_of`]),
    /// else a clone's `agents/` (#696), and the clone they sit in.
    fn fetch(self) -> Result<Fetched> {
        match self {
            Source::Local(src) => {
                let shown = src.display();
                anyhow::ensure!(src.is_dir(), "source {shown} is not a directory");
                let agents = agents::library_of(src)?;
                let root = src.to_path_buf();
                Ok(Fetched(None, root, agents))
            }
            Source::Git { url, path } => {
                let tmp = tempfile::tempdir().context("creating temp dir for clone")?;
                let clone_dir = tmp.path().join("clone");
                let root = clone(url, path, &clone_dir)?;
                let agents = agents::library_of(&root)?
                    .unwrap_or_else(|| clone_dir.join(brokkr_runtime::bundle::DEFAULT_AGENTS_DIR));
                Ok(Fetched(Some(tmp), root, Some(agents)))
            }
        }
    }
}

/// What [`Source::fetch`] found: the clone, which must outlive the copies
/// made from it, the bundle root, and the source's agent library.
struct Fetched(Option<tempfile::TempDir>, PathBuf, Option<PathBuf>);

/// The directory at `path`, already judged plain ([`Source::judge`]),
/// inside `clone`, walked a part at a time by each part's own entry, never
/// through a link: a link can name anything outside the clone, and
/// [`copy_dir`] refuses links only below the bundle it is given. An entry
/// that cannot be read refuses as one the clone does not hold.
fn reach(clone: &Path, path: &str) -> Result<PathBuf, AtPath> {
    let (mut at, mut walked) = (clone.to_path_buf(), PathBuf::new());
    for part in path.split(std::path::is_separator) {
        at.push(part);
        walked.push(part);
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if meta.is_symlink() => {
                let (path, link) = (path.into(), walked);
                return Err(AtPath::Link { path, link });
            }
            Ok(meta) if meta.is_dir() => {}
            _ => {
                let (path, missing) = (path.into(), walked);
                return Err(AtPath::Missing { path, missing });
            }
        }
    }
    let bundle = std::fs::symlink_metadata(at.join("bundle.json"));
    if !bundle.is_ok_and(|meta| meta.is_file()) {
        return Err(AtPath::Unbundled { path: path.into() });
    }
    Ok(at)
}

/// Refuse an `add` whose library `dir`, an absolute path Brokkr built from
/// the repository's root, would put a recipe anywhere in a folder
/// repository but its own shelf (#675, #696 ruling 6), before anything is
/// cloned, copied or created. `dir` is judged where it resolves, since a
/// shelf may be a link: by the one walk ([`places::Layout::discover`])
/// from the nearest entry standing at `dir` or above it, since `add`
/// creates the rest. The walk resolves links and stops at the nearest
/// `.git`, so it is taken from every ancestor of that entry as `dir`
/// spells it, and from every ancestor of the real directory it resolves
/// to: a library or a shelf linked into a folder repository's `recipes/`,
/// and a nested repository below a folder, refuse. A layout or an entry
/// that cannot be read refuses in its own words (fail closed). The place
/// `dir` resolves to is returned.
fn refuse_folder(dir: &Path) -> Result<PathBuf> {
    let at = standing(dir)?;
    let rest = dir.strip_prefix(at).expect("an ancestor prefixes its path");
    let real = places::Layout::discover(at)?.cwd().to_path_buf();
    // Collected from its parts, so an empty `rest` leaves no trailing `/`.
    let place: PathBuf = real.join(rest).components().collect();
    let resolved = real
        .ancestors()
        .filter(|real| !at.ancestors().any(|s| s == *real));
    for start in at.ancestors().chain(resolved) {
        let layout = places::Layout::discover(start)?;
        if layout.kind() == Kind::Folder && place != layout.places().recipes {
            let root = layout.root().to_path_buf();
            return Err(InFolder { root, place }.into());
        }
    }
    Ok(place)
}

/// Refuse when any planned recipe in `sources`, the leaf or a base its
/// chain reaches, holds `place`, where the library resolves to
/// ([`Within`]): a base beside a leaf in the folder can be the shelf itself.
fn refuse_within(sources: &[&Path], place: &Path) -> Result<()> {
    for bundle in sources {
        let source = bundle.canonicalize()?;
        if place.starts_with(&source) {
            let place = place.to_path_buf();
            return Err(Within { source, place }.into());
        }
    }
    Ok(())
}

/// The nearest of `path`, an absolute path, and its ancestors at which an
/// entry stands, judged as the entry itself ([`places::entry`]); the root
/// directory always stands. One that cannot be read refuses.
fn standing(path: &Path) -> Result<&Path, PlacesError> {
    let found = path.ancestors().find_map(|at| match places::entry(at) {
        Ok(entry) => entry.map(|_| Ok(at)),
        Err(source) => Some(Err(PlacesError::Unreadable {
            path: at.to_path_buf(),
            source,
        })),
    });
    found.unwrap_or(Ok(path))
}

/// Clone the git `source` into `clone`, and the bundle root inside it: at
/// `path` ([`reach`]), else where [`bundle_root`] discovers it.
fn clone(source: &str, path: Option<&str>, clone: &Path) -> Result<PathBuf> {
    // `--` stops option injection; `protocol.ext.allow=never` stops
    // the ext transport, which would otherwise execute an arbitrary
    // command from a source like `ext::sh -c ... x.git`.
    let out = Command::new("git")
        .args(["-c", "protocol.ext.allow=never"])
        .args(["clone", "--depth", "1", "--"])
        .arg(source)
        .arg(clone)
        .output()
        .context("running git clone")?;
    if !out.status.success() {
        bail!(
            "git clone {source} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    match path {
        Some(path) => Ok(reach(clone, path)?),
        None => bundle_root(clone),
    }
}

/// Install a recipe into the library `dir`, the shelf of the repository at
/// `root` that `add` runs in (#696 ruling 6): clone or copy it into
/// `<dir>/<name>`, with the bases it extends that the library lacks and
/// the agents and charters they name that `root`'s agent library lacks
/// ([`agents::plan`]), then compile-verify every copy against `root`'s
/// agents and adapters. If any fails to compile all are removed — the
/// library only ever holds recipes the compiler accepted or nothing. The
/// library is held from before it is made to the last compile ([`Shelf`]); a clone
/// happens before, unheld. A name that is not one plain component
/// ([`Unplain::Name`]), a `path` the source cannot take ([`Source::judge`])
/// and a library in a folder repository but its shelf ([`refuse_folder`])
/// are refused first, and a planned recipe holding the library
/// ([`refuse_within`]) or a folder bundle before anything is copied.
pub(crate) fn add(
    root: &Path,
    dir: &Path,
    source: &str,
    name: &str,
    path: Option<&str>,
) -> Result<()> {
    if !plain(Path::new(name)) {
        let (name, dir) = (name.to_string(), dir.to_path_buf());
        return Err(Unplain::Name { name, dir }.into());
    }
    let source = Source::judge(source, path)?;
    let place = refuse_folder(dir)?;
    let world = World::discover(root, None)?;
    let dest = dir.join(name);
    if std::fs::symlink_metadata(&dest).is_ok() {
        bail!(
            "recipe '{name}' already exists at {}; remove it first",
            dest.display()
        );
    }
    let fetched = source.fetch()?;
    install_held(root, dir, &place, name, fetched, world.as_ref())
}

/// [`add`]'s install, once judged: make and hold the library `dir`, which
/// resolves to `place`, plan the recipes and the agents they name, refuse
/// a planned recipe holding `place` or a folder bundle, copy them, and
/// keep them only if every recipe compiles against `root`. The clone in
/// `fetched` outlives the copies.
fn install_held(
    root: &Path,
    dir: &Path,
    place: &Path,
    name: &str,
    Fetched(_clone, bundle, source_agents): Fetched,
    world: Option<&World>,
) -> Result<()> {
    // Declared before what it outlives: the copies are kept or removed while
    // the library is still held, and an empty one made here goes last.
    let _shelf = Shelf::make(dir, place)?;
    let planned = planned(&bundle, name, dir)?;
    let sources: Vec<&Path> = planned.iter().map(|(_, source)| source.as_path()).collect();
    refuse_within(&sources, place)?;
    sources::refuse_folder_bundles(&sources)?;
    let library = places::Places::of(root)?.agents;
    let hires = agents::plan(&sources, source_agents.as_deref(), &library)?;
    let mut installing = install(&planned, dir)?;
    agents::copy(&hires, &mut installing.1)?;
    let digests = verify(root, &installing, world)?;
    for ((recipe, path), digest) in installing.keep().iter().zip(digests) {
        eprintln!("added recipe '{recipe}' ({digest}) at {}", path.display());
    }
    for hire in &hires {
        eprintln!("added {} at {}", hire.what, hire.to.display());
    }
    Ok(())
}

/// The digest of each recipe `installing` copied, compiled against `root`,
/// or the first refusal ([`Refused`]).
fn verify(root: &Path, installing: &Installing, world: Option<&World>) -> Result<Vec<String>> {
    let compiled = installing.0.iter().map(|(recipe, path)| -> Result<String> {
        let bundle = compile_in(root, path, world).map_err(|cause| Refused {
            recipe: recipe.clone(),
            cause,
        })?;
        Ok(bundle.manifest_digest()[..12].to_string())
    });
    compiled.collect()
}

#[cfg(test)]
mod tests;
