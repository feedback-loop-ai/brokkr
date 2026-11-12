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
use std::path::{Component, Path};

use brokkr_core::canonical::sha256_bytes;
use serde_json::{Map, Value};

use super::{
    egress_class, invalid, object, only_keys, route_name, string, EgressClass, LibraryError,
    ROUTE_GRAMMAR,
};
use crate::agents::DeclaredRouteOverlay;

/// The one provider whose driver admits a route overlay today (decision
/// 0056's 2026-09-14 note). An adapter of another provider may not
/// declare one: its driver would receive a `--patch` it does not read.
const OVERLAY_PROVIDER: &str = "dsh";

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
pub(super) struct Declared {
    pub(super) routes: BTreeMap<String, EgressClass>,
    pub(super) overlays: BTreeMap<String, DeclaredRouteOverlay>,
}

/// Read an adapter's `routes`, or refuse its first bad entry.
pub(super) fn declarations(
    map: &Map<String, Value>,
    what: &str,
    path: &Path,
    provider: &str,
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
                if provider != OVERLAY_PROVIDER {
                    return invalid(format!(
                        "{what} 'routes.{route}.overlay' declares a route overlay on adapter \
                         '{provider}'; only the '{OVERLAY_PROVIDER}' driver admits a route \
                         overlay, and a provider that does not read one refuses it at load \
                         rather than composing a flag it ignores"
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
fn directory(path: &Path, what: &str) -> Result<std::path::PathBuf, LibraryError> {
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
    let prefix = format!("{what} 'routes.{route}.overlay");
    let at = format!("{prefix}'");
    let declared = object(raw, &at)?;
    only_keys(declared, &OVERLAY_KEYS, &at)?;
    let file = string(declared, "file", &at)?;
    let relative = Path::new(&file);
    if relative.is_absolute() {
        return invalid(format!(
            "{prefix}.file' must be relative to the adapter file's own directory"
        ));
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return invalid(format!(
            "{prefix}.file' must not traverse out of the adapter file's own directory"
        ));
    }
    let resolved = directory.join(relative).canonicalize().map_err(|_| {
        LibraryError::Invalid(format!(
            "{prefix}.file' does not resolve to a regular file inside the adapter file's \
             own directory"
        ))
    })?;
    if !resolved.starts_with(directory) {
        return invalid(format!(
            "{prefix}.file' resolves outside the adapter file's own directory"
        ));
    }
    let meta = std::fs::metadata(&resolved).map_err(|_| {
        LibraryError::Invalid(format!(
            "{prefix}.file' does not resolve to a regular file inside the adapter file's \
             own directory"
        ))
    })?;
    if !meta.is_file() {
        return invalid(format!(
            "{prefix}.file' does not resolve to a regular file inside the adapter file's \
             own directory"
        ));
    }
    if meta.len() > MAX_BYTES {
        return invalid(format!(
            "{prefix}.file' is past the route overlay's byte bound"
        ));
    }
    let sha256 = string(declared, "sha256", &at)?;
    if !is_digest(&sha256) {
        return invalid(format!(
            "{prefix}.sha256' must be a 64-character lowercase hexadecimal digest"
        ));
    }
    let bytes = std::fs::read(&resolved).map_err(|_| {
        LibraryError::Invalid(format!(
            "{prefix}.file' does not resolve to a readable regular file inside the adapter \
             file's own directory"
        ))
    })?;
    if bytes.len() as u64 > MAX_BYTES {
        return invalid(format!(
            "{prefix}.file' is past the route overlay's byte bound"
        ));
    }
    if sha256_bytes(&bytes) != sha256 {
        return invalid(format!(
            "{prefix}.sha256' does not equal the digest of the file this route declares"
        ));
    }
    Ok(DeclaredRouteOverlay {
        file,
        path: resolved,
        sha256,
    })
}

/// `true` for 64 lowercase hexadecimal characters, the one digest spelling
/// the driver's comparison can produce.
fn is_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests;
