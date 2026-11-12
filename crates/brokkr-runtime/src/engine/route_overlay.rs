//! One seat's route overlay, bound where the private start context is
//! built (decision 0056's 2026-09-14 note as amended by #1052 ask 2;
//! design D6 mechanism 1; AS3). A binding is EITHER a compiled leaf
//! layer's `files` member, with the manifest's recorded digest, OR the
//! overlay the seat's adapter route declares, with the digest that
//! declaration pins at load. The engine never hashes the file here: a
//! member edited since compilation or a declared file changed since load
//! still travels with its recorded digest, and the driver's one read and
//! required equality is what refuses the bytes.
//!
//! A value that does not bind yields no member, never a failed start —
//! the adapter's pre-work refusal is the single outcome.

use std::path::{Component, Path};

use super::resume;
use super::Bundle;
use crate::agents::DeclaredRouteOverlay;

/// The seat's single `--patch` value, or `None`. One exact `--patch` with
/// a non-empty, non-flag value is the only shape; a second `--patch`, a
/// bare `--patch`, or any other `--patch…` spelling (an `=`-joined value
/// or a longer token) is ambiguous and yields no value.
pub(super) fn single_patch_value(command: &[String]) -> Option<&str> {
    let mut found: Option<&str> = None;
    let mut index = 0;
    while index < command.len() {
        let part = command[index].as_str();
        if part == "--patch" {
            if found.is_some() {
                return None;
            }
            let value = command.get(index + 1)?;
            if value.is_empty() || value.starts_with("--") {
                return None;
            }
            found = Some(value);
            index += 2;
            continue;
        }
        if part.starts_with("--patch") {
            return None;
        }
        index += 1;
    }
    found
}

/// The seat's `--patch` value and its relative path, with the two
/// lexical refusals both binding forms share: an absolute value and a
/// `..` component bind nothing.
fn patch_value(command: &[String]) -> Option<(&str, &Path)> {
    let value = single_patch_value(command)?;
    let relative = Path::new(value);
    if relative.is_absolute() {
        return None;
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    Some((value, relative))
}

/// Bind one seat's single `--patch` value: the declared overlay when the
/// seat's route has one, otherwise the compiled leaf layer's member.
pub(super) fn route_overlay_binding(
    bundle: &Bundle,
    command: &[String],
    workdir: &Path,
    declared: Option<&DeclaredRouteOverlay>,
) -> Option<resume::RouteOverlay> {
    match declared {
        Some(declared) => declared_binding(declared, command, workdir),
        None => layer_binding(bundle, command, workdir),
    }
}

/// The route overlay its adapter's route declares (decision 0056's
/// 2026-09-14 note as amended): the value must be the one the composition
/// spelled, the file must canonicalize inside the run's working
/// directory, and it must be the declaration's own resolved file. A
/// library staged outside the working directory binds nothing — this is
/// the limit the guide states: the library must sit inside the run's
/// tree, and the driver then refuses the start with `PatchUnbound`.
fn declared_binding(
    declared: &DeclaredRouteOverlay,
    command: &[String],
    workdir: &Path,
) -> Option<resume::RouteOverlay> {
    let (value, relative) = patch_value(command)?;
    let base = std::fs::canonicalize(workdir).ok()?;
    let resolved = std::fs::canonicalize(base.join(relative)).ok()?;
    if !resolved.starts_with(&base) || !resolved.is_file() || resolved != declared.path {
        return None;
    }
    Some(resume::RouteOverlay {
        value: value.to_string(),
        digest: declared.sha256.clone(),
    })
}

/// Bind the value to the compiled leaf layer (design D6 mechanism 1;
/// AS3). The value resolves relative to the run's working directory,
/// without an absolute path or `..` component or symlink escape, to a
/// regular file inside the compiled bundle's own layer directory that is
/// a `files` member of the compiled manifest; the binding carries the
/// argv value and that member's recorded digest. An ancestor layer's
/// file, a same-shaped file outside the layer, a working-directory
/// shadow and the bundle-relative `./` expansion (already absolute) all
/// take this same outcome.
fn layer_binding(
    bundle: &Bundle,
    command: &[String],
    workdir: &Path,
) -> Option<resume::RouteOverlay> {
    let (value, relative) = patch_value(command)?;
    // Canonicalize both sides: an in-layer symlink whose target resolves
    // outside the layer is not a binding, and a working-directory shadow
    // canonicalizes outside the compiled layer too.
    let resolved = std::fs::canonicalize(workdir.join(relative)).ok()?;
    let layer = std::fs::canonicalize(&bundle.dir).ok()?;
    if !resolved.is_file() {
        return None;
    }
    let inside = resolved.strip_prefix(&layer).ok()?;
    // Manifest keys are component-joined with `/` on every platform
    // (bundle.rs's `walk_files`), so spell the looked-up key the same way.
    let key = inside
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()?
        .join("/");
    let digest = bundle.manifest.get("files")?.get(key.as_str())?.as_str()?;
    Some(resume::RouteOverlay {
        value: value.to_string(),
        digest: digest.to_string(),
    })
}

#[cfg(test)]
mod tests;
