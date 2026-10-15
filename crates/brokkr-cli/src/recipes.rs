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
use brokkr_runtime::{Bundle, Invocation, SeatBody};

use crate::compile_in;

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
            SeatBody::Invocation(Invocation::Single {
                candidates,
                command,
                ..
            }) => candidates
                .first()
                .map(|candidate| candidate.agent.clone())
                .unwrap_or_else(|| command.first().cloned().unwrap_or_else(|| "inline".into())),
            SeatBody::Invocation(Invocation::Panel { members, .. }) => members
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
            SeatBody::Invocation(Invocation::Single { .. }) => name.clone(),
            SeatBody::Invocation(Invocation::Panel { members, .. }) => format!(
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
/// not. Nothing aborts the listing: a broken recipe is information.
pub(crate) fn list(workspace: &Path, dir: &Path) -> Result<()> {
    let world = World::discover(workspace, None)?;
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            let mut subdirs: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect();
            subdirs.sort();
            for sub in subdirs {
                let name = sub
                    .file_name()
                    .expect("read_dir child path has a final component")
                    .to_string_lossy()
                    .into_owned();
                candidates.push((name, sub));
            }
        }
        Err(e) => println!("warning: recipes dir {}: {e}", dir.display()),
    }
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

/// The recipes one `add` copied into the library, by name, removed
/// together when dropped unless kept — so an install refused at any step
/// leaves no partial copy squatting on a name and making every retry fail
/// with "already exists".
struct Installing(Vec<(String, PathBuf)>);

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
        std::mem::take(&mut self.0)
    }
}

impl Drop for Installing {
    fn drop(&mut self) {
        for (_, dest) in &self.0 {
            let _ = std::fs::remove_dir_all(dest);
        }
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

/// Copy the recipe at `root` into `<dir>/<name>`, and with it each base
/// its `extends` chain reaches (decision 0017) that the library does not
/// already hold, under the name it is extended by: a base resolves from
/// the library the leaf sits in, so a derived recipe copied alone would
/// never compile there. The chain is walked a layer at a time against the
/// target, so it stops at the first base the library holds, whose own
/// chain is the library's, wherever the source's chain would end. A base
/// that neither the library nor the source's directory holds as a plain
/// directory ends the walk too, and the compile that follows names why
/// the install does not compose. The library holds a base it has any
/// entry for, a dangling link included, read as the source side is.
fn install(root: &Path, name: &str, dir: &Path) -> Result<Installing> {
    let mut installing = Installing(Vec::new());
    installing.copy(root, name, dir.join(name))?;
    let mut layer = root.canonicalize()?;
    while let Some(base) = base_of(&layer) {
        let dest = dir.join(&base);
        let source = layer.with_file_name(&base);
        let plain = std::fs::symlink_metadata(&source).is_ok_and(|meta| meta.is_dir());
        if std::fs::symlink_metadata(&dest).is_ok() || !plain {
            break;
        }
        installing.copy(&source, &base, dest)?;
        layer = source;
    }
    Ok(installing)
}

/// Hold the library exclusively, by a lock on its own directory, until the
/// handle drops; a second holder waits. An install exposes each base it
/// copies at its library name before its compile is accepted, and removes
/// it when refused, so another `add` that ran meanwhile could stop its walk
/// at that copy and keep a leaf whose base is then removed. One `add` at a
/// time keeps "every copy kept or none" true across invocations.
fn hold(dir: &Path) -> Result<rustix::fd::OwnedFd> {
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

/// An `add` refused in a folder repository (decision 0083; #675): a
/// `recipes/<name>/bundle.json` is a recipe library's mark, and a library
/// outranks a folder beside it (the operator's ruling 1 of 2026-10-13 on
/// #634), so the install would move every default out of the folder
/// unannounced. `root` is the folder repository's. A caller matches the
/// type, never the text (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
struct InFolder {
    root: PathBuf,
}

impl std::fmt::Display for InFolder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let root = self.root.display();
        write!(
            formatter,
            "{root} keeps its Brokkr in {root}/{FOLDER} (decision 0083): recipes add would \
             install a recipe library there, which outranks the folder and moves every default \
             out of it (#675); a folder repository's bundle is {FOLDER}/ itself, so nothing was \
             written"
        )
    }
}

impl std::error::Error for InFolder {}

/// An `add` refused because a path it would write is not one it can judge
/// before writing it (#675): the place judged must be the place written.
/// A caller matches the variant, never the text (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
enum Unplain {
    /// `dir` holds a part that is not a plain name below `standing`, the
    /// nearest entry on it that stands: a `..` there is resolved by the
    /// directories `add` would make, never by the walk that judges it.
    Dir { dir: PathBuf, standing: PathBuf },
    /// `name` is not one plain path component ([`plain`]), so
    /// `<dir>/<name>` could land outside `dir`.
    Name { name: String, dir: PathBuf },
}

impl std::fmt::Display for Unplain {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unplain::Dir { dir, standing } => write!(
                formatter,
                "--dir {} holds a part that is not a plain name below {}, the nearest entry on \
                 it that stands, so recipes add cannot judge where it would write (#675); \
                 nothing was written",
                dir.display(),
                standing.display()
            ),
            Unplain::Name { name, dir } => write!(
                formatter,
                "--name '{name}' is not one plain path component, so the recipe would not be one \
                 directory of {} (#675); nothing was written",
                dir.display()
            ),
        }
    }
}

impl std::error::Error for Unplain {}

/// Refuse an `add` that would make a folder repository a recipe library
/// (#675, the operator's option A), before anything is cloned, copied or
/// created: one run from a workspace whose layout is the folder's, or one
/// into a `dir` inside a folder repository, judged by the one walk
/// ([`places::Layout::discover`]) from the nearest entry standing at `dir`
/// or above it, since `add` creates the rest. That rest is made as named,
/// so it must hold plain names only ([`Unplain::Dir`]). The walk resolves
/// links and stops at the nearest `.git`, so it is taken from every
/// ancestor of that entry as `dir` spells it, and from every ancestor of
/// the real directory it resolves to: a folder repository a link or a
/// nested repository stands below still refuses, and so does one a link
/// from outside it resolves into. A layout or an entry that cannot be read
/// refuses in its own words (fail closed).
fn refuse_folder(workspace: &Path, dir: &Path) -> Result<()> {
    let target = std::path::absolute(dir).map_err(|source| PlacesError::Unreadable {
        path: dir.to_path_buf(),
        source,
    })?;
    let at = standing(&target)?;
    let rest = target
        .strip_prefix(at)
        .expect("an ancestor prefixes its path");
    if !rest.iter().all(|part| plain(Path::new(part))) {
        let (dir, standing) = (dir.to_path_buf(), at.to_path_buf());
        return Err(Unplain::Dir { dir, standing }.into());
    }
    refuse_any(std::iter::once(workspace).chain(at.ancestors()))?;
    // The walk from `at` above has resolved it already, so this one stands.
    let resolved = places::Layout::discover(at)?;
    let real = resolved.cwd().ancestors();
    refuse_any(real.filter(|real| !at.ancestors().any(|spelled| spelled == *real)))
}

/// Refuse [`InFolder`] when the layout discovered from any of `starts`
/// ([`places::Layout::discover`], the one judgement) is a folder; a walk
/// that refuses refuses in its own words.
fn refuse_any<'a>(starts: impl IntoIterator<Item = &'a Path>) -> Result<()> {
    for start in starts {
        let layout = places::Layout::discover(start)?;
        if layout.kind() == Kind::Folder {
            let root = layout.root().to_path_buf();
            return Err(InFolder { root }.into());
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

/// Clone the git `source` into `<tmp>/clone`, and the bundle root inside
/// it ([`bundle_root`]).
fn clone(source: &str, tmp: &Path) -> Result<PathBuf> {
    let clone = tmp.join("clone");
    // `--` stops option injection; `protocol.ext.allow=never` stops
    // the ext transport, which would otherwise execute an arbitrary
    // command from a source like `ext::sh -c ... x.git`.
    let out = Command::new("git")
        .args(["-c", "protocol.ext.allow=never"])
        .args(["clone", "--depth", "1", "--"])
        .arg(source)
        .arg(&clone)
        .output()
        .context("running git clone")?;
    if !out.status.success() {
        bail!(
            "git clone {source} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    bundle_root(&clone)
}

/// Install a recipe: clone or copy into `<dir>/<name>`, with the bases it
/// extends that the library lacks, then compile-verify every copy. If any
/// fails to compile all are removed — the library only ever holds recipes
/// the compiler accepted or nothing. The library is held from the first
/// copy to the last compile ([`hold`]); a clone happens before, unheld. A
/// name that is not one plain component ([`Unplain::Name`]) and a folder
/// repository ([`refuse_folder`]) are refused first.
pub(crate) fn add(workspace: &Path, source: &str, name: &str, dir: &Path) -> Result<()> {
    if !plain(Path::new(name)) {
        let (name, dir) = (name.to_string(), dir.to_path_buf());
        return Err(Unplain::Name { name, dir }.into());
    }
    refuse_folder(workspace, dir)?;
    let world = World::discover(workspace, None)?;
    let dest = dir.join(name);
    if std::fs::symlink_metadata(&dest).is_ok() {
        bail!(
            "recipe '{name}' already exists at {}; remove it first",
            dest.display()
        );
    }
    std::fs::create_dir_all(dir)?;

    let tmp;
    let root = if is_git_source(source) {
        tmp = tempfile::tempdir().context("creating temp dir for clone")?;
        clone(source, tmp.path())?
    } else {
        let src = Path::new(source);
        anyhow::ensure!(src.is_dir(), "source {source} is not a directory");
        src.to_path_buf()
    };
    // Declared before the copies, so they are kept or removed while it is
    // still held.
    let _library = hold(dir)?;
    let installing = install(&root, name, dir)?;

    let mut digests = Vec::new();
    for (recipe, path) in &installing.0 {
        match compile_in(workspace, path, world.as_ref()) {
            Ok(bundle) => digests.push(bundle.manifest_digest()[..12].to_string()),
            Err(cause) => {
                let recipe = recipe.clone();
                return Err(Refused { recipe, cause }.into());
            }
        }
    }
    for ((recipe, path), digest) in installing.keep().iter().zip(digests) {
        eprintln!("added recipe '{recipe}' ({digest}) at {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
