//! Staging the single patch dsh reads at launch.

use std::{io::Write, path::Path};

use crate::native_controls::dsh::Web;

use super::{
    dsh_effort, dsh_effort_settings_in, dsh_hands::Selection, dsh_settings_row,
    dsh_storage_context, dsh_transcript_row, DshArgv, DshSeatOverlay,
};

/// The seat overlay with Brokkr's sandbox rows: a linked worktree's scoped
/// runner (decision 0054) or a boxed seat's hands rows (#896). It is one
/// patch file because `--patch` is the launcher's only override channel.
#[cfg(test)]
pub(super) fn dsh_seat_overlay_with(
    model: Option<&str>,
    effort: Option<&str>,
    root: &std::path::Path,
    route: Option<&[u8]>,
    sandbox: Option<&str>,
) -> Result<DshSeatOverlay, String> {
    dsh_seat_overlay_in(model, effort, root, route, sandbox, create_patch)
}

fn create_patch() -> std::io::Result<tempfile::NamedTempFile> {
    tempfile::Builder::new()
        .prefix("brokkr-dsh-seat-")
        .suffix(".yml")
        .tempfile()
}

// Test-only observation that `dsh_seat_overlay_in` was entered, so a
// refusing planner path can prove that staging never happened: an error
// or an absent retained directory does not establish that fact (task
// 8.10; answer U's R3). The planner is synchronous and the counter is
// thread-local, so parallel tests never observe one another's staging;
// a process-global counter would.
#[cfg(test)]
thread_local! {
    static DSH_STAGING_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(super) fn dsh_staging_calls() -> usize {
    DSH_STAGING_CALLS.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(super) fn reset_dsh_staging_calls() {
    DSH_STAGING_CALLS.with(|calls| calls.set(0));
}

/// The seat overlay over an injected file, so the ways staging can fail
/// are reachable from a test without a full disk. The injected creator
/// stages the patch; the settings document, when one is needed, is
/// staged beside it under its own prefix. Every row after the model's
/// is composed first and written once, so the one way the write can
/// fail is the one way a test can make it fail.
///
/// The validated route rows, when the seat carries one, are folded AHEAD
/// of the transcript, model and settings rows, so the launcher receives
/// exactly one `--patch` and Brokkr's own rows apply last (AS3; design D6
/// mechanism 1). The route bytes are already grammar-checked and their
/// provenance is the bound manifest digest the bundle digest covers.
#[cfg(test)]
pub(super) fn dsh_seat_overlay_in(
    model: Option<&str>,
    effort: Option<&str>,
    root: &std::path::Path,
    route: Option<&[u8]>,
    sandbox: Option<&str>,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<DshSeatOverlay, String> {
    stage(
        SeatPatch {
            model,
            effort,
            root,
            route,
            sandbox,
            web: None,
        },
        create,
    )
}

struct SeatPatch<'a> {
    model: Option<&'a str>,
    effort: Option<&'a str>,
    root: &'a Path,
    route: Option<&'a [u8]>,
    sandbox: Option<&'a str>,
    web: Option<Web>,
}

/// The hands composition already disables the entire tool-web row.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum StageRefusal {
    #[error(
        "dsh driver: boxed hands disable the tool-web row, which a native web holding would enable"
    )]
    Boxed,
    #[error("{0}")]
    Write(String),
}

fn web_for(web: Web, sandbox: &Selection) -> Result<Option<Web>, StageRefusal> {
    match sandbox {
        Selection::Hands { .. } if web.any_enabled() => Err(StageRefusal::Boxed),
        Selection::Hands { .. } => Ok(None),
        Selection::Runner(_) => Ok(Some(web)),
    }
}

pub(super) fn stage_launch(
    argv: &DshArgv,
    root: &Path,
    route: Option<&[u8]>,
    sandbox: &Selection,
) -> Result<DshSeatOverlay, StageRefusal> {
    let web = web_for(argv.web, sandbox)?;
    stage(
        SeatPatch {
            model: argv.model.as_deref(),
            effort: argv.effort.as_deref(),
            root,
            route,
            sandbox: sandbox.rows(),
            web,
        },
        create_patch,
    )
    .map_err(StageRefusal::Write)
}

fn stage(
    patch: SeatPatch<'_>,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<DshSeatOverlay, String> {
    let SeatPatch {
        model,
        effort,
        root,
        route,
        sandbox,
        web,
    } = patch;
    #[cfg(test)]
    DSH_STAGING_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut rows = dsh_transcript_row(root)?;
    let settings = match (model, effort) {
        (Some(model), Some(effort)) => {
            let settings = dsh_effort_settings_in(model, effort, || {
                tempfile::Builder::new()
                    .prefix("brokkr-dsh-seat-settings-")
                    .suffix(".yaml")
                    .tempfile()
            })?;
            rows.push_str(&dsh_settings_row(settings.path())?);
            Some(settings)
        }
        (None, Some(_)) => {
            return Err(
                "dsh driver: `--effort` needs a `--model` beside it: the level rides the \
                 seat's default-model selection, which names its provider and model, and this \
                 driver does not read the profile's default back to restate it"
                    .to_string(),
            );
        }
        _ => None,
    };
    // Brokkr's sandbox rows, a linked worktree's scoped runner (decision
    // 0054) or a boxed seat's hands rows (#896), ride with its own rows,
    // after the route's: the route may not reach what they name.
    if let Some(sandbox) = sandbox {
        rows.push_str(sandbox);
    }
    let mut body = route_prefix(route)
        .map_err(|_| "dsh driver: validated route overlay is not UTF-8".to_string())?;
    if let Some(model) = model {
        body.push_str(&dsh_effort::model_row(model, effort)?);
    }
    body.push_str(&rows);
    if let Some(web) = web {
        body.push_str(&web.row());
    }
    let mut patch = dsh_storage_context(create(), "could not stage the dsh seat overlay")?;
    // The write carries a path too: `tempfile` wraps a failed write on its
    // own handle with the file it holds, so this half is as path-bearing
    // as the allocation above.
    dsh_storage_context(
        patch.write_all(body.as_bytes()),
        "could not write the dsh seat overlay",
    )?;
    Ok(DshSeatOverlay { patch, settings })
}

fn route_prefix(route: Option<&[u8]>) -> Result<String, std::str::Utf8Error> {
    let mut body = String::new();
    if let Some(route) = route {
        let text = std::str::from_utf8(route)?;
        body.push_str(text);
        if !text.ends_with('\n') {
            body.push('\n');
        }
    }
    Ok(body)
}

#[cfg(test)]
mod tests;
