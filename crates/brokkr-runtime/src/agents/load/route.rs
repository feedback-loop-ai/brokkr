//! An adapter's `routes`, read once (decision 0036 ruling 2; decision
//! 0056's 2026-09-14 note as amended by issue #1052 ask 2). A route entry
//! is either today's string egress class or an object carrying that class
//! beside the route's overlay declaration. The class and its readers stay
//! exactly where they were; the overlay is a separate map on the adapter,
//! so no egress reader moves.
//!
//! The overlay is the route's own fact and lives with the route: its file
//! is relative to the adapter file's directory, resolved through symlinks
//! and held inside that directory, a regular file within the driver
//! reader's bound, and it must carry the declared SHA-256. Every refusal
//! names the adapter, the route and the field, never the value.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use brokkr_core::canonical::{is_sha256_hex, sha256_bytes};
use serde_json::{Map, Value};

use super::{
    egress_class, invalid, object, only_keys, route_name, string, EgressClass, LibraryError,
    ROUTE_GRAMMAR,
};
use crate::agents::DeclaredRouteOverlay;

/// The one driver that admits a route overlay today (decision 0056's
/// 2026-09-14 note). An adapter dispatching another driver may not
/// declare one: that driver would receive a `--patch` it does not read.
const OVERLAY_DRIVER: &str = "dsh";

/// The driver reader's bound, mirrored here so the loader refuses an
/// over-bound file before any run reaches the driver's own check. The
/// authoritative constant is `brokkr_protocol::adapters::route_overlay`'s
/// `MAX_BYTES`, which is private to the protocol crate; #1037 holds
/// `adapters.rs`, so this loader states the same number rather than
/// widening that crate from here.
const MAX_BYTES: u64 = 64 * 1024;

/// The keys of an object route entry: the class every route has, and the
/// overlay this one may declare.
const ROUTE_KEYS: [&str; 2] = ["egress", "overlay"];
/// The keys of an overlay declaration.
const OVERLAY_KEYS: [&str; 2] = ["file", "sha256"];

/// Both maps one adapter's `routes` declaration produces: the classes its
/// existing readers take, and the route overlays the engine composes.
#[derive(Debug)]
pub(super) struct Declared {
    pub(super) routes: BTreeMap<String, EgressClass>,
    pub(super) overlays: BTreeMap<String, DeclaredRouteOverlay>,
}

/// Read an adapter's `routes`, or refuse its first bad entry.
pub(super) fn declarations(
    map: &Map<String, Value>,
    what: &str,
    path: &Path,
    driver: &[String],
) -> Result<Declared, LibraryError> {
    let mut routes = BTreeMap::new();
    let mut overlays = BTreeMap::new();
    let Some(declared) = map.get("routes") else {
        return Ok(Declared { routes, overlays });
    };
    let Some(entries) = declared.as_object() else {
        return invalid(format!(
            "{what} 'routes' must be an object of route name → egress class"
        ));
    };
    let directory = directory(path, what)?;
    for (route, value) in entries {
        if !route_name(route) {
            return invalid(format!(
                "{what} 'routes' names '{route}', which does not match {ROUTE_GRAMMAR}"
            ));
        }
        match value {
            Value::String(_) => {
                routes.insert(
                    route.clone(),
                    egress_class(value, &format!("routes.{route}"), what)?,
                );
            }
            Value::Object(entry) => {
                let at = format!("{what} 'routes.{route}'");
                only_keys(entry, &ROUTE_KEYS, &at)?;
                let egress = entry.get("egress").ok_or_else(|| {
                    LibraryError::Invalid(format!(
                        "{at} needs 'egress': the route's class is declared in both forms"
                    ))
                })?;
                routes.insert(route.clone(), route_egress(egress, route, what)?);
                let overlay = entry.get("overlay").ok_or_else(|| {
                    LibraryError::Invalid(format!(
                        "{at} needs 'overlay': the object form is how a route declares one"
                    ))
                })?;
                let kind = crate::capabilities::harness_of(driver);
                if kind != OVERLAY_DRIVER {
                    return invalid(format!(
                        "{what} 'routes.{route}.overlay' declares a route overlay on an \
                         adapter that dispatches '{kind}'; only the '{OVERLAY_DRIVER}' \
                         driver reads a route overlay, so a driver that does not is \
                         refused at load rather than handed a flag it ignores"
                    ));
                }
                overlays.insert(
                    route.clone(),
                    overlay_declaration(overlay, what, route, &directory)?,
                );
            }
            _ => {
                return invalid(format!(
                    "{what} 'routes.{route}' must be a string egress class or an object \
                     with 'egress' and 'overlay'"
                ))
            }
        }
    }
    Ok(Declared { routes, overlays })
}

/// The class an object entry's `egress` states, refused in the closed
/// vocabulary without echoing the value.
fn route_egress(declared: &Value, route: &str, what: &str) -> Result<EgressClass, LibraryError> {
    match declared.as_str().and_then(EgressClass::parse) {
        Some(class) => Ok(class),
        None => invalid(format!(
            "{what} 'routes.{route}.egress' is not one of {}; a route's class is the same \
             closed vocabulary in both forms (decision 0036 ruling 1)",
            EgressClass::VOCABULARY
        )),
    }
}

/// The adapter file's own directory, canonical, where its overlay paths
/// resolve.
fn directory(path: &Path, what: &str) -> Result<PathBuf, LibraryError> {
    path.parent()
        .unwrap_or_else(|| Path::new(""))
        .canonicalize()
        .map_err(|_| {
            LibraryError::Invalid(format!(
                "{what} 'routes' cannot resolve the adapter file's own directory"
            ))
        })
}

/// One overlay declaration: `file` and `sha256`, checked against the file
/// the first names. Every refusal names the adapter, the route and the
/// field, never the value.
fn overlay_declaration(
    raw: &Value,
    what: &str,
    route: &str,
    directory: &Path,
) -> Result<DeclaredRouteOverlay, LibraryError> {
    overlay_declaration_with(
        raw,
        what,
        route,
        directory,
        |path| std::fs::metadata(path),
        |path| std::fs::read(path),
    )
    .map_err(|fault| match fault {
        OverlayFault::Library(error) => error,
        OverlayFault::Refused(refusal) => LibraryError::Invalid(refusal.to_string()),
    })
}

/// The text the canonicalize failure and the not-a-file check share.
const NOT_REGULAR: &str =
    "does not resolve to a regular file inside the adapter file's own directory";
/// The text the metadata fault and the read fault share.
const UNREADABLE: &str =
    "does not resolve to a readable regular file inside the adapter file's own directory";
/// The text the metadata bound and the read bound share.
const OVER_BOUND: &str = "is past the route overlay's byte bound";

/// Why one overlay declaration was refused: one variant per check, in the
/// order the checks run, so a test can tell which fired even where two
/// share their words. Each names the adapter, the route and the field,
/// never the value or a path. It becomes text only in
/// [`overlay_declaration`].
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum OverlayRefusal<'a> {
    #[error(
        "{what} 'routes.{route}.overlay.file' must be relative to the adapter file's own directory"
    )]
    Absolute { what: &'a str, route: &'a str },
    #[error(
        "{what} 'routes.{route}.overlay.file' must not traverse out of the adapter file's own \
         directory"
    )]
    Traverses { what: &'a str, route: &'a str },
    #[error("{what} 'routes.{route}.overlay.file' {text}", text = NOT_REGULAR)]
    Unresolved { what: &'a str, route: &'a str },
    #[error(
        "{what} 'routes.{route}.overlay.file' resolves outside the adapter file's own directory"
    )]
    Outside { what: &'a str, route: &'a str },
    #[error("{what} 'routes.{route}.overlay.file' {text}", text = UNREADABLE)]
    MetadataUnreadable { what: &'a str, route: &'a str },
    #[error("{what} 'routes.{route}.overlay.file' {text}", text = NOT_REGULAR)]
    NotRegular { what: &'a str, route: &'a str },
    #[error("{what} 'routes.{route}.overlay.file' {text}", text = OVER_BOUND)]
    MetadataOverBound { what: &'a str, route: &'a str },
    #[error(
        "{what} 'routes.{route}.overlay.sha256' must be a 64-character lowercase hexadecimal \
         digest"
    )]
    Malformed { what: &'a str, route: &'a str },
    #[error("{what} 'routes.{route}.overlay.file' {text}", text = UNREADABLE)]
    ReadUnreadable { what: &'a str, route: &'a str },
    #[error("{what} 'routes.{route}.overlay.file' {text}", text = OVER_BOUND)]
    ReadOverBound { what: &'a str, route: &'a str },
    #[error(
        "{what} 'routes.{route}.overlay.sha256' does not equal the digest of the file this \
         route declares"
    )]
    Mismatch { what: &'a str, route: &'a str },
}

/// What [`overlay_declaration_with`] refuses with: the shared readers'
/// own [`LibraryError`], carried unchanged, or this file's typed refusal.
#[derive(Debug)]
enum OverlayFault<'a> {
    Library(LibraryError),
    Refused(OverlayRefusal<'a>),
}

impl From<LibraryError> for OverlayFault<'_> {
    fn from(error: LibraryError) -> Self {
        Self::Library(error)
    }
}

impl<'a> From<OverlayRefusal<'a>> for OverlayFault<'a> {
    fn from(refusal: OverlayRefusal<'a>) -> Self {
        Self::Refused(refusal)
    }
}

/// [`overlay_declaration`] over an injected metadata read and file read,
/// the closure seams the fault tests use. Production's one path is the
/// call above with `std::fs::metadata` and `std::fs::read`.
fn overlay_declaration_with<'a, M, R>(
    raw: &Value,
    what: &'a str,
    route: &'a str,
    directory: &Path,
    metadata: M,
    read: R,
) -> Result<DeclaredRouteOverlay, OverlayFault<'a>>
where
    M: FnOnce(&Path) -> std::io::Result<std::fs::Metadata>,
    R: FnOnce(&Path) -> std::io::Result<Vec<u8>>,
{
    let at = format!("{what} 'routes.{route}.overlay'");
    let declared = object(raw, &at)?;
    only_keys(declared, &OVERLAY_KEYS, &at)?;
    let file = string(declared, "file", &at)?;
    let resolved = overlay_path(&file, what, route, directory)?;
    let meta =
        metadata(&resolved).map_err(|_| OverlayRefusal::MetadataUnreadable { what, route })?;
    if !meta.is_file() {
        return Err(OverlayRefusal::NotRegular { what, route }.into());
    }
    if meta.len() > MAX_BYTES {
        return Err(OverlayRefusal::MetadataOverBound { what, route }.into());
    }
    let sha256 = string(declared, "sha256", &at)?;
    if !is_sha256_hex(&sha256) {
        return Err(OverlayRefusal::Malformed { what, route }.into());
    }
    let bytes = read(&resolved).map_err(|_| OverlayRefusal::ReadUnreadable { what, route })?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(OverlayRefusal::ReadOverBound { what, route }.into());
    }
    if sha256_bytes(&bytes) != sha256 {
        return Err(OverlayRefusal::Mismatch { what, route }.into());
    }
    Ok(DeclaredRouteOverlay {
        file,
        path: resolved,
        sha256,
    })
}

/// The declared file's canonical path, held inside the adapter's own
/// directory through symlinks. The two lexical refusals come first, so an
/// absolute path or a `..` component is named as itself rather than as an
/// escape.
fn overlay_path<'a>(
    file: &str,
    what: &'a str,
    route: &'a str,
    directory: &Path,
) -> Result<PathBuf, OverlayRefusal<'a>> {
    let relative = Path::new(file);
    if relative.is_absolute() {
        return Err(OverlayRefusal::Absolute { what, route });
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(OverlayRefusal::Traverses { what, route });
    }
    let resolved = directory
        .join(relative)
        .canonicalize()
        .map_err(|_| OverlayRefusal::Unresolved { what, route })?;
    if !resolved.starts_with(directory) {
        return Err(OverlayRefusal::Outside { what, route });
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests;
