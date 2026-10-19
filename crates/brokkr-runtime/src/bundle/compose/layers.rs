//! The steps of [`read_layers`](super::read_layers) (#288, slice M): one
//! layer's declaring document read, named and appended, and the base its
//! `extends` reaches in the library, each refusing as it did inline.
//! `read_layers` reads as the cycle check, these two and the depth bound.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{base_of, invalid, Layer, Pinned};
use crate::bundle::CompileError;

/// Read the layer at `dir` from its own `bundle.json`, refuse a name an
/// ancestor already declares, and append it to `layers`. Returns the
/// layer's file and its `extends`, if it declares one.
pub(super) fn read_layer(
    layers: &mut Vec<Layer>,
    dir: &Path,
    reached_as: &Option<String>,
) -> Result<(PathBuf, Option<Value>), CompileError> {
    let file = dir.join("bundle.json");
    // Read STRICTLY from the layer's own bytes (decision 0065, design
    // D3): a seat's capability requests are written here, and an
    // ordinary JSON map keeps the last copy of a repeated key — so a
    // `"requires"` followed by a `"wants"` under one name, or a second
    // `capabilities` field, would weaken a requirement before any
    // validation saw it. Every layer is read this way, so a leaf that
    // replaces a seat cannot hide what its ancestor wrote twice.
    // The document is consumed like any active input (design D7): ONE
    // bound read, whose buffer is parsed here and whose digest the
    // layer's walk must pin before its identity is sealed.
    let bound = crate::bundle::bound_input(dir, "bundle.json").map_err(|fault| match fault {
        crate::bundle::InputFault::Missing(error) => CompileError::Io(error),
        crate::bundle::InputFault::Place(place) => invalid(format!(
            "{}: the layer's declaring document, {place}. What a run is composed from must \
             be what its identity names, so it is refused (decision 0065 slice one, design \
             D7)",
            file.display()
        )),
    })?;
    let (read, bytes) = Pinned::of(bound);
    let text = String::from_utf8(bytes).map_err(std::io::Error::other)?;
    let parsed = brokkr_core::canonical::parse_strict(&text)
        .map_err(|problem| invalid(format!("{}: {problem}", file.display())))?;
    let document: Map<String, Value> = serde_json::from_value(parsed)?;
    let name = document
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("{} missing 'name'", file.display())))?
        .to_string();
    if let Some(prior) = layers.iter().find(|layer| layer.name == name) {
        return Err(invalid(format!(
            "{} declares name '{name}', which its ancestor {} already declares; \
             a derived recipe states its own name so runs, manifests and \
             dispatch pins never report a base's name under a derived digest",
            file.display(),
            prior.file.display()
        )));
    }
    let extends = document.get("extends").cloned();
    layers.push(Layer {
        name,
        reached_as: reached_as.clone(),
        dir: dir.to_path_buf(),
        file: file.clone(),
        document,
        read,
    });
    Ok((file, extends))
}

/// The base a layer's `extends` names, as the canonical directory it is in
/// the layer's library, or refused: a value that is no recipe name, a name
/// that is no recipe there, or one that resolves outside the library.
pub(super) fn reach_base<'a>(
    dir: &Path,
    file: &Path,
    extends: &'a Value,
) -> Result<(&'a str, PathBuf), CompileError> {
    let base = base_of(file, extends)?;
    // The library is the leaf directory's parent, what a recipe library already is and what
    // `brokkr recipes add` installs into before it compile-verifies the copy, or a `.brokkr`
    // folder's shelf (#696): `places::base_library`, the one judgement of a layer's library.
    // `dir` is already canonical, so the library is asked for directly, never through `..`.
    let library = crate::places::base_library(dir)?;
    let candidate = library.join(base);
    if !candidate.is_dir() {
        return Err(invalid(format!(
            "{}: extends '{base}', which is not a recipe in {}",
            file.display(),
            library.display()
        )));
    }
    // A base must BE in the library, not merely be reachable from
    // it: `brokkr recipes add` already refuses symlinks, and a
    // composed base is bind-mounted read-only into every confined
    // seat, so a link pointing outside would widen that mount.
    let resolved = candidate.canonicalize()?;
    if resolved.parent() != Some(library.as_path()) {
        return Err(invalid(format!(
            "{}: extends '{base}', which resolves outside the library to {}; \
             a base is composed from and mounted into confined seats, so it \
             must be a real directory in {}",
            file.display(),
            resolved.display(),
            library.display()
        )));
    }
    Ok((base, resolved))
}
