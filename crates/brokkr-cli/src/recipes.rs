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
//! (proposed decision 0075 ruling 5).

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use brokkr_runtime::launch::BundleSource;
use brokkr_runtime::realms::World;
use brokkr_runtime::{Bundle, SeatBody};

use crate::compile_in;

/// The run/resume bundle from the exactly-one-of `--bundle` / `--recipe`
/// pair (clap's arg group enforces the arity). The launch resolves it,
/// where each verb always did ([`BundleSource::resolve`]).
pub(crate) fn source(
    bundle: Option<PathBuf>,
    recipe: Option<String>,
    recipes_dir: PathBuf,
) -> BundleSource {
    match (bundle, recipe) {
        (Some(path), None) => BundleSource::Dir(path),
        (None, Some(name)) => BundleSource::Recipe { name, recipes_dir },
        _ => unreachable!("clap group requires exactly one of --bundle/--recipe"),
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
    fn copy(&mut self, from: &Path, name: &str, dest: PathBuf) -> Result<()> {
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

/// Copy the recipe at `root` into `<dir>/<name>`, and with it each base
/// its `extends` chain reaches (decision 0017) that the library does not
/// already hold, under the name it is extended by: a base resolves from
/// the library the leaf sits in, so a derived recipe copied alone would
/// never compile there. The walk stops at the first base the library
/// holds, whose own chain is the library's. A chain that does not
/// resolve where the source stands copies the leaf alone, and the compile
/// that follows names why it does not compose.
fn install(root: &Path, name: &str, dir: &Path) -> Result<Installing> {
    let mut installing = Installing(Vec::new());
    installing.copy(root, name, dir.join(name))?;
    let Ok(resolved) = brokkr_runtime::bundle::compose::resolve(root) else {
        return Ok(installing);
    };
    for ancestor in &resolved.chain {
        let base = ancestor.reached_as.as_deref().unwrap_or(&ancestor.name);
        let dest = dir.join(base);
        if dest.exists() {
            break;
        }
        installing.copy(&ancestor.dir, base, dest)?;
    }
    Ok(installing)
}

/// Install a recipe: clone or copy into `<dir>/<name>`, with the bases it
/// extends that the library lacks, then compile-verify every copy. If any
/// fails to compile all are removed — the library only ever holds recipes
/// the compiler accepted or nothing.
pub(crate) fn add(workspace: &Path, source: &str, name: &str, dir: &Path) -> Result<()> {
    let world = World::discover(workspace, None)?;
    let dest = dir.join(name);
    if dest.exists() {
        bail!(
            "recipe '{name}' already exists at {}; remove it first",
            dest.display()
        );
    }
    std::fs::create_dir_all(dir)?;

    let installing = if is_git_source(source) {
        let tmp = tempfile::tempdir().context("creating temp dir for clone")?;
        let clone = tmp.path().join("clone");
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
        install(&bundle_root(&clone)?, name, dir)?
    } else {
        let src = Path::new(source);
        anyhow::ensure!(src.is_dir(), "source {source} is not a directory");
        install(src, name, dir)?
    };

    let mut digests = Vec::new();
    for (recipe, path) in &installing.0 {
        match compile_in(workspace, path, world.as_ref()) {
            Ok(bundle) => digests.push(bundle.manifest_digest()[..12].to_string()),
            Err(e) => bail!("recipe '{recipe}' does not compile (removed): {e}"),
        }
    }
    for ((recipe, path), digest) in installing.keep().iter().zip(digests) {
        eprintln!("added recipe '{recipe}' ({digest}) at {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
