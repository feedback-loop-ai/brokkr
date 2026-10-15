//! Doctor's folder line, the report's first (decision 0083 ruling 5):
//! the folder discovery found and from where, or why there is none, in
//! the report's status column. Doctor reports and never refuses: a
//! discovery refusal, an old layout and a folder whose compile refuses
//! are each a line that makes the report unhealthy. The old shape is the
//! runtime's one judgement ([`places::Layout::old`]), read without the
//! moves a refusal lists, and its words are the refusal's own openings.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use brokkr_runtime::bundle::Bundle;
use brokkr_runtime::launch::compile_for;
use brokkr_runtime::places::{self, Kind, Old, FOLDER};
use brokkr_runtime::realms::World;

use super::{Report, Seats};
use crate::folder::{DoctorDefaults, FolderError, Found, Opening};

/// The line's name in the report.
const WHAT: &str = "folder";

/// The bundle doctor's report reads (R5 of 2026-10-14), and which of the
/// three ways it came to have it, each with its directory and compile.
pub(super) enum Inspected {
    /// `--bundle`, compiled under the map doctor inspected.
    Given(PathBuf, anyhow::Result<Bundle>),
    /// The folder found with no old shape and no `--bundle`, compiled
    /// once, as `--bundle` compiles it, and the map a run loads (`map`):
    /// the folder line's outcome is the map's refusal, else that compile's.
    Folder {
        dir: PathBuf,
        compiled: anyhow::Result<Bundle>,
        map: anyhow::Result<()>,
    },
    /// No `--bundle`, and no folder found that a run would deliver under.
    Neither,
}

impl Inspected {
    /// The bundle the report reads, when it compiled.
    pub(super) fn compiled(&self) -> Option<&Bundle> {
        match self {
            Inspected::Given(_, compiled) | Inspected::Folder { compiled, .. } => {
                compiled.as_ref().ok()
            }
            Inspected::Neither => None,
        }
    }

    /// What the report learns about the seats binding a credential
    /// (decision 0040 ruling 4), `declared` the names the compiled
    /// bundle's seats declare. A folder whose compile refuses is a bundle
    /// that did not compile, never no bundle given.
    pub(super) fn seats<'a>(&self, declared: Option<&'a BTreeSet<String>>) -> Seats<'a> {
        match (self, declared) {
            (_, Some(declared)) => Seats::Declared(declared),
            (Inspected::Given(..) | Inspected::Folder { .. }, None) => Seats::BundleDidNotCompile,
            (Inspected::Neither, None) => Seats::NoBundleGiven,
        }
    }

    /// The bundle line's directory and compile: a given bundle's either
    /// way, the folder's unless its refusal is the folder line's, which it
    /// is where the map a run loads loads ([`Inspected::folder`]): a
    /// refusal is named once, and never nowhere.
    pub(super) fn line(&self) -> Option<(&Path, &anyhow::Result<Bundle>)> {
        match self {
            Inspected::Given(dir, compiled)
            | Inspected::Folder {
                dir,
                compiled: compiled @ Ok(_),
                ..
            }
            | Inspected::Folder {
                dir,
                compiled,
                map: Err(_),
            } => Some((dir, compiled)),
            Inspected::Folder {
                compiled: Err(_),
                map: Ok(()),
                ..
            }
            | Inspected::Neither => None,
        }
    }

    /// The folder line's outcome when the folder was the report's bundle,
    /// for [`report_folder`]: the map's refusal, else the compile's; else
    /// none, and the folder line compiles its own.
    pub(super) fn folder(self) -> Option<anyhow::Result<()>> {
        match self {
            Inspected::Folder { compiled, map, .. } => Some(map.and(compiled.map(drop))),
            Inspected::Given(..) | Inspected::Neither => None,
        }
    }
}

/// The bundle doctor's report reads (R5 of 2026-10-14): `bundle` as
/// given, else a folder found with no old shape, either compiled once
/// under the map doctor inspected (`mapped`), so the folder is read as
/// `doctor --bundle .brokkr` reads it. The folder also carries whether
/// the map a run loads (`realms`, else the folder's) loads: where it
/// does, it is the map inspected, and the compile is the one a run makes.
pub(super) fn bundle(
    bundle: Option<&Path>,
    defaults: &DoctorDefaults,
    realms: Option<&Path>,
    mapped: Option<&World>,
) -> Inspected {
    let workspace = &defaults.workspace;
    let compile = |dir: &Path| -> anyhow::Result<Bundle> {
        Ok(compile_for(workspace, dir, mapped, workspace)?)
    };
    if let Some(dir) = bundle {
        return Inspected::Given(dir.to_path_buf(), compile(dir));
    }
    match default_folder(&defaults.found) {
        Some(dir) => Inspected::Folder {
            compiled: compile(&dir),
            map: World::discover(workspace, realms)
                .map(drop)
                .map_err(Into::into),
            dir,
        },
        None => Inspected::Neither,
    }
}

/// The folder a run with no `--bundle` delivers under: the found folder,
/// where no old shape stands beside it.
fn default_folder(found: &Result<Found, FolderError>) -> Option<PathBuf> {
    let layout = &found.as_ref().ok()?.layout;
    let clean = layout.kind() == Kind::Folder && matches!(layout.old(), Ok(None));
    clean.then(|| layout.root().join(FOLDER))
}

/// `dir` compiled as a run would compile it, under the map a run loads
/// (`realms`, else the folder's), so a map that will not load is the
/// compile's refusal, as `brokkr compile` prints it, never a folder
/// compiled under no map.
fn compile_folder(workspace: &Path, dir: &Path, realms: Option<&Path>) -> anyhow::Result<Bundle> {
    let world = World::discover(workspace, realms)?;
    Ok(compile_for(workspace, dir, world.as_ref(), workspace)?)
}

/// Put the folder line first in `report`: the found folder compiled by
/// [`compile_folder`], unless `compiled` already holds that compile's
/// outcome, because the folder was the report's bundle ([`bundle`]).
pub(super) fn report_folder(
    report: &mut Report,
    defaults: &DoctorDefaults,
    realms: Option<&Path>,
    compiled: Option<anyhow::Result<()>>,
) {
    let compile = |dir: &Path| match compiled {
        Some(outcome) => outcome,
        None => compile_folder(&defaults.workspace, dir, realms).map(drop),
    };
    report_folder_with(report, &defaults.found, compile);
}

/// [`report_folder`], `compile` the folder's compile. The lines go ahead
/// of every other, and a missing one leaves the report unhealthy.
fn report_folder_with(
    report: &mut Report,
    found: &Result<Found, FolderError>,
    compile: impl FnOnce(&Path) -> anyhow::Result<()>,
) {
    let mut folder = Report {
        healthy: true,
        lines: Vec::new(),
    };
    match found {
        Ok(found) => observe(&mut folder, &found.layout, compile),
        Err(refused) => folder.missing(WHAT, refused.to_string()),
    }
    report.healthy &= folder.healthy;
    report.lines.splice(0..0, folder.lines);
}

/// The lines `layout` earns: a library's own layout, an old shape's
/// opening, each entry a half-moved folder left at the root, no folder,
/// or the folder found and whether its compile refuses.
fn observe(
    report: &mut Report,
    layout: &places::Layout,
    compile: impl FnOnce(&Path) -> anyhow::Result<()>,
) {
    let (cwd, root) = (layout.cwd().display(), layout.root().display());
    let folder = layout.root().join(FOLDER);
    let found = format!("{} (found from {cwd})", folder.display());
    match (layout.kind(), layout.old()) {
        (_, Err(refused)) => report.missing(WHAT, refused.to_string()),
        (Kind::Library, _) => report.ok(
            WHAT,
            format!("none: {root} is a recipe library, which keeps its own layout (decision 0083)"),
        ),
        (_, Ok(Some(Old::Root))) => report.missing(WHAT, Opening::Root.to_string()),
        (_, Ok(Some(Old::StateOnly))) => report.missing(WHAT, Opening::StateOnly.to_string()),
        (_, Ok(Some(Old::HalfMoved { left, .. }))) => {
            for path in &left {
                report.missing(WHAT, format!("{found}, but {}", Opening::Left(path)));
            }
        }
        (Kind::Bare, Ok(None)) => report.warn(
            WHAT,
            format!("none from {cwd} up to {root} — brokkr init . writes one"),
        ),
        (Kind::Folder, Ok(None)) => match compile(&folder) {
            Ok(()) => report.ok(WHAT, found),
            Err(refused) => report.missing(
                WHAT,
                format!(
                    "{found}, but its compile refuses (decision 0083): {}",
                    crate::safe_lines(&format!("{refused:#}"))
                ),
            ),
        },
    }
}

#[cfg(test)]
mod folder_tests;
