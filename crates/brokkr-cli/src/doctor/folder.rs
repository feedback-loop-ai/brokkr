//! Doctor's folder line, the report's first (decision 0083 ruling 5):
//! the folder discovery found and from where, or why there is none, in
//! the report's status column. Doctor reports and never refuses: a
//! discovery refusal, an old layout and a folder whose compile refuses
//! are each a line that makes the report unhealthy. The old shape is the
//! runtime's one judgement ([`places::Layout::old`]), read without the
//! moves a refusal lists, and its words are the refusal's own openings.

use std::path::{Path, PathBuf};

use brokkr_runtime::bundle::Bundle;
use brokkr_runtime::launch::compile_for;
use brokkr_runtime::places::{self, Kind, Old, FOLDER};
use brokkr_runtime::realms::World;

use super::Report;
use crate::folder::{DoctorDefaults, FolderError, Found, Opening};

/// The line's name in the report.
const WHAT: &str = "folder";

/// The bundle the report reads, its compile, and the folder's compile
/// when that was the same one.
type Doctored = (
    Option<PathBuf>,
    Option<anyhow::Result<Bundle>>,
    Option<anyhow::Result<()>>,
);

/// The bundle doctor's report reads (R5 of 2026-10-14): `bundle` as
/// given, compiled under the map doctor inspected (`mapped`); else a
/// folder found with no old shape, compiled once as the folder line
/// compiles it, that one compile also the folder line's. A folder that
/// refuses is no bundle to the rest of the report, so its refusal is
/// named once, in the folder line.
pub(super) fn bundle(
    bundle: Option<&Path>,
    defaults: &DoctorDefaults,
    realms: Option<&Path>,
    mapped: Option<&World>,
) -> Doctored {
    let workspace = &defaults.workspace;
    if let Some(dir) = bundle {
        let compiled = compile_for(workspace, dir, mapped, workspace).map_err(Into::into);
        return (Some(dir.to_path_buf()), Some(compiled), None);
    }
    match default_folder(&defaults.found).map(|dir| (compile_folder(workspace, &dir, realms), dir))
    {
        Some((Ok(compiled), dir)) => (Some(dir), Some(Ok(compiled)), Some(Ok(()))),
        Some((Err(refused), _)) => (None, None, Some(Err(refused))),
        None => (None, None, None),
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
