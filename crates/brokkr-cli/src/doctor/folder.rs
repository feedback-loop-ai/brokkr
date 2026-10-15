//! Doctor's folder line, the report's first (decision 0083 ruling 5):
//! the folder discovery found and from where, or why there is none, in
//! the report's status column. Doctor reports and never refuses: a
//! discovery refusal, an old layout and a folder whose compile refuses
//! are each a line that makes the report unhealthy. The old shape is the
//! runtime's one judgement ([`places::Layout::old`]), read without the
//! moves a refusal lists, and its words are the refusal's own openings.

use std::path::Path;

use brokkr_runtime::places::{self, Kind, Old, FOLDER};
use brokkr_runtime::realms::World;

use super::Report;
use crate::folder::{DoctorDefaults, FolderError, Found, Opening};

/// The line's name in the report.
const WHAT: &str = "folder";

/// Put the folder line first in `report`: the found folder compiled as a
/// run would compile it, under the map a run loads (`realms`, else the
/// folder's), so a map that will not load is the compile's refusal, as
/// `brokkr compile` prints it, never a folder compiled under no map.
pub(super) fn report_folder(report: &mut Report, defaults: &DoctorDefaults, realms: Option<&Path>) {
    let workspace = &defaults.workspace;
    let compile = |dir: &Path| -> anyhow::Result<()> {
        let world = World::discover(workspace, realms)?;
        brokkr_runtime::launch::compile_for(workspace, dir, world.as_ref(), workspace)?;
        Ok(())
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
