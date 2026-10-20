//! The steps of [`read_layers`](super::read_layers) (#288, slice M): one
//! layer's declaring document read, named and appended, and the base its
//! `extends` reaches in the library its origin reads (#697), each refusing
//! as it did inline. `read_layers` reads as the cycle check, these two and
//! the depth bound.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{invalid, Layer, Pinned};
use crate::bundle::CompileError;
use crate::places::{Origin, Shelves, FOLDER};

/// Read the layer at `dir`, of `origin`, from its own `bundle.json`, refuse
/// a name an ancestor already declares, and append it to `layers`. Returns
/// the layer's `extends`, if it declares one.
pub(super) fn read_layer(
    layers: &mut Vec<Layer>,
    (dir, origin): (&Path, Origin),
    reached_as: &Option<String>,
) -> Result<Option<Value>, CompileError> {
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
        origin,
        file,
        document,
        read,
    });
    Ok(extends)
}

/// The base the last of `layers` names in its `extends`, as the canonical
/// directory it is in the library its origin reads (#697), or refused: a
/// value that is no recipe name, a name that is no recipe there, or one
/// that resolves outside the library. A personal layer may name the
/// folder's own bundle as `.brokkr`.
pub(super) fn reach_base<'a>(
    layers: &[Layer],
    extends: &'a Value,
    shelves: &Shelves,
) -> Result<(&'a str, PathBuf), CompileError> {
    let layer = layers.last().expect("a layer was read");
    let (dir, file, origin) = (&layer.dir, &layer.file, layer.origin);
    if origin == Origin::Personal && extends.as_str() == Some(FOLDER) {
        return Ok((FOLDER, shelves.folder(dir)?));
    }
    let base = base_of(file, extends)?;
    // The library is the leaf directory's parent, what a recipe library already is and what
    // `brokkr recipes add` installs into before it compile-verifies the copy, or a `.brokkr`
    // folder's shelf (#696): `places::base_library`, the one judgement of a layer's library.
    // `dir` is already canonical, so the library is asked for directly, never through `..`.
    // A personal layer reads its own shelf or the project's (#697 ruling 3).
    let library = match origin {
        Origin::Personal => shelves.personal_library(dir, base)?,
        Origin::Project | Origin::Folder | Origin::Library => crate::places::base_library(dir)?,
    };
    let candidate = library.join(base);
    if !candidate.is_dir() {
        shelves.refuse_personal(origin, dir, base)?;
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

/// `^[a-z0-9][a-z0-9-]*$`, checked BEFORE any path is built so `../x`,
/// `a/b`, `SDD` and `.` are refused as names and never become paths.
fn valid_recipe_name(name: &str) -> bool {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The recipe a layer's `extends` names, refused before any path is built: a non-string, or a
/// name that is no recipe name.
fn base_of<'a>(file: &Path, extends: &'a Value) -> Result<&'a str, CompileError> {
    let file = file.display();
    let base = extends.as_str().ok_or_else(|| {
        invalid(format!(
            "{file}: 'extends' must be the name of a recipe in the library"
        ))
    })?;
    if !valid_recipe_name(base) {
        return Err(invalid(format!(
            "{file}: 'extends' value '{base}' is not a recipe name; names match \
             ^[a-z0-9][a-z0-9-]*$ and are refused before any path is built"
        )));
    }
    Ok(base)
}
