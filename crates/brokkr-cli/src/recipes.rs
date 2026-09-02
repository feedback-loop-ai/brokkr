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
//! working recipes as broken — and, in `add`, delete them for it.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use brokkr_runtime::{Bundle, SeatBody};

use crate::compile_in;

/// The two bundles shipped in-repo, always listed alongside `--dir`.
/// Paths are as written, relative to the workspace.
const BUILTINS: [&str; 2] = ["bundles/self", "bundles/verify"];

/// Resolve the run/resume bundle directory from the exactly-one-of
/// `--bundle` / `--recipe` pair (clap's arg group enforces the arity).
pub fn resolve(
    bundle: Option<PathBuf>,
    recipe: Option<String>,
    recipes_dir: &Path,
) -> Result<PathBuf> {
    match (bundle, recipe) {
        (Some(path), None) => Ok(path),
        (None, Some(name)) => {
            let path = recipes_dir.join(&name);
            anyhow::ensure!(
                path.is_dir(),
                "recipe '{name}' not found under {}; install it with \
                 `brokkr recipes add <source> --name {name}`",
                recipes_dir.display()
            );
            Ok(path)
        }
        _ => unreachable!("clap group requires exactly one of --bundle/--recipe"),
    }
}

/// Seat names in declared (sorted) order, panels rendered as
/// `review[correctness+security]`.
fn seat_summary(bundle: &Bundle) -> String {
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
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// One line per recipe that compiles; a warning line per one that does
/// not. Nothing aborts the listing: a broken recipe is information.
pub fn list(workspace: &Path, dir: &Path) -> Result<()> {
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
        Err(e) => println!(
            "warning: recipes dir {}: {e}; listing built-ins only",
            dir.display()
        ),
    }
    for builtin in BUILTINS {
        let path = workspace.join(builtin);
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        candidates.push((name, path));
    }
    for (name, path) in candidates {
        match compile_in(workspace, &path) {
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

/// Copy into `dest`, removing the partial copy if anything fails —
/// otherwise an aborted install would squat on the name and make every
/// retry fail with "already exists".
fn copy_into(from: &Path, dest: &Path) -> Result<()> {
    copy_dir(from, dest).inspect_err(|_| {
        let _ = std::fs::remove_dir_all(dest);
    })
}

/// Install a recipe: clone or copy into `<dir>/<name>`, then
/// compile-verify the copy. A copy that fails to compile is removed —
/// the library only ever holds recipes the compiler accepted or nothing.
pub fn add(workspace: &Path, source: &str, name: &str, dir: &Path) -> Result<()> {
    let dest = dir.join(name);
    if dest.exists() {
        bail!(
            "recipe '{name}' already exists at {}; remove it first",
            dest.display()
        );
    }
    std::fs::create_dir_all(dir)?;

    if is_git_source(source) {
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
        let root = bundle_root(&clone)?;
        copy_into(&root, &dest)?;
    } else {
        let src = Path::new(source);
        anyhow::ensure!(src.is_dir(), "source {source} is not a directory");
        copy_into(src, &dest)?;
    }

    match compile_in(workspace, &dest) {
        Ok(bundle) => {
            eprintln!(
                "added recipe '{name}' ({}) at {}",
                &bundle.manifest_digest()[..12],
                dest.display()
            );
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dest);
            bail!("recipe '{name}' does not compile (removed): {e}");
        }
    }
}

#[cfg(test)]
mod tests;
