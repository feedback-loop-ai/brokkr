//! The steps of [`read_layers`](super::read_layers) (#288, slice M): one
//! layer's declaring document read, named and appended, and the base its
//! `extends` reaches in the library its origin reads (#697), each refusing
//! as it did inline. `read_layers` reads as the cycle check, these two and
//! the depth bound. The last step of a layer, its seal, is here too.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{invalid, Ancestor, Consumed, Layer, Pinned};
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
        crate::bundle::InputFault::Place(place) => refused(LayerRefusal::Place {
            file: &file,
            place: place.to_string(),
        }),
    })?;
    let (read, bytes) = Pinned::of(bound);
    let text = String::from_utf8(bytes).map_err(std::io::Error::other)?;
    let parsed = brokkr_core::canonical::parse_strict(&text)
        .map_err(|problem| LayerRefusal::Parse {
            file: &file,
            problem,
        })
        .map_err(refused)?;
    let document: Map<String, Value> = serde_json::from_value(parsed)?;
    let name = name_of(&document, &file, layers)
        .map_err(refused)?
        .to_string();
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
    let base = base_of(file, extends).map_err(refused)?;
    // The library is the leaf directory's parent, what a recipe library already is and what
    // `brokkr library recipes add` installs into before it compile-verifies the copy, or a `.brokkr`
    // folder's shelf (#696): `places::base_library`, the one judgement of a layer's library.
    // `dir` is already canonical, so the library is asked for directly, never through `..`.
    // A personal layer reads its own shelf or the project's (#697 ruling 3).
    let library = match origin {
        Origin::Personal => shelves.personal_library(dir, base)?,
        Origin::Project | Origin::Folder | Origin::Library => crate::places::base_library(dir)?,
    };
    let candidate = library.join(base);
    // A missing base asks the shelf first, which may refuse a personal one in its own words.
    let resolved = if candidate.is_dir() {
        Some(candidate.canonicalize()?)
    } else {
        shelves.refuse_personal(origin, dir, base)?;
        None
    };
    contained((file, base), &library, resolved)
        .map(|resolved| (base, resolved))
        .map_err(refused)
}

/// The base `base`, named by `file`, as the directory it resolved to in `library`, or refused:
/// `None` (no directory stands there) is no recipe, and one resolving anywhere but directly in
/// `library` stands outside it. A base must BE in the library, not merely be reachable from
/// it: `brokkr library recipes add` already refuses symlinks, and a composed base is bind-mounted
/// read-only into every confined seat, so a link pointing outside would widen that mount.
fn contained<'a>(
    (file, base): (&'a Path, &'a str),
    library: &'a Path,
    resolved: Option<PathBuf>,
) -> Result<PathBuf, LayerRefusal<'a>> {
    let Some(resolved) = resolved else {
        return Err(LayerRefusal::NotARecipe {
            file,
            base,
            library,
        });
    };
    if resolved.parent() != Some(library) {
        return Err(LayerRefusal::Outside {
            file,
            base,
            resolved,
            library,
        });
    }
    Ok(resolved)
}

/// The name the layer `file` declares in `document`, refused when it declares none or one an
/// ancestor in `layers` already declares.
fn name_of<'a>(
    document: &'a Map<String, Value>,
    file: &'a Path,
    layers: &'a [Layer],
) -> Result<&'a str, LayerRefusal<'a>> {
    let name = document
        .get("name")
        .and_then(Value::as_str)
        .ok_or(LayerRefusal::Nameless { file })?;
    match layers.iter().find(|layer| layer.name == name) {
        Some(prior) => Err(LayerRefusal::NameTaken {
            file,
            name,
            prior: &prior.file,
        }),
        None => Ok(name),
    }
}

/// The pin of the layer at `dir`, `(name, reached_as)`, over `chain`, its
/// own ancestors sealed so far, from what `read` composed it from: an
/// ancestor's in [`Resolved::seal`](super::Resolved::seal), and the folder
/// layer's a seat was taken from, over no chain (decision 0091).
pub(super) fn sealed(
    dir: &Path,
    (name, reached_as): (&str, Option<String>),
    chain: &[Ancestor],
    read: &Consumed,
) -> Result<Ancestor, CompileError> {
    // An ancestor's digest covers its own files and its own
    // ancestors — never the leaf's agent resolution or the adapter
    // declarations that authorised its gates, both of which belong
    // to the composed bundle rather than to any layer.
    // An ancestor boxes nothing at this level, so the boundary it is
    // handed writes no key and moves no digest; `namespace` is the
    // word a layer meant before decision 0046 named one.
    let manifest = crate::bundle::manifest_for(
        dir,
        name,
        chain,
        None,
        None,
        &BTreeMap::new(),
        &Map::new(),
        brokkr_core::realms::Boundary::Namespace,
        // Nor does an ancestor hold or grant a capability (decision
        // 0065): authority belongs to the composed bundle compiled in
        // a realm, so a layer's digest carries none and does not move.
        None,
        &read.digests(),
    )?;
    let files = manifest["files"]
        .as_object()
        .expect("manifest files")
        .clone();
    read.check(&files)?;
    Ok(Ancestor {
        name: name.to_string(),
        reached_as,
        dir: dir.to_path_buf(),
        digest: brokkr_core::canonical::sha256_hex(&manifest),
        files,
    })
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
fn base_of<'f, 'v: 'f>(file: &'f Path, extends: &'v Value) -> Result<&'v str, LayerRefusal<'f>> {
    let base = extends.as_str().ok_or(LayerRefusal::NotAString { file })?;
    if !valid_recipe_name(base) {
        return Err(LayerRefusal::NotARecipeName { file, base });
    }
    Ok(base)
}

/// Why a layer's read or its `extends` was refused, one variant per check, each field
/// borrowed from the layer or its library where it can be. Its `Display` is the operator's
/// text byte for byte; [`refused`] folds it into the compile refusal, the one place it
/// becomes text (#353).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum LayerRefusal<'a> {
    /// The declaring document's bound read refused its place (decision 0065, design D7).
    #[error(
        "{}: the layer's declaring document, {place}. What a run is composed from must be what \
         its identity names, so it is refused (decision 0065 slice one, design D7)",
        .file.display()
    )]
    Place { file: &'a Path, place: String },
    /// The declaring document is no strict JSON (design D3).
    #[error("{}: {problem}", .file.display())]
    Parse { file: &'a Path, problem: String },
    /// The document declares no string `name`.
    #[error("{} missing 'name'", .file.display())]
    Nameless { file: &'a Path },
    /// An ancestor, `prior`, already declares the layer's name.
    #[error(
        "{} declares name '{name}', which its ancestor {} already declares; a derived recipe \
         states its own name so runs, manifests and dispatch pins never report a base's name \
         under a derived digest",
        .file.display(),
        .prior.display()
    )]
    NameTaken {
        file: &'a Path,
        name: &'a str,
        prior: &'a Path,
    },
    /// No directory stands at the base's name in the library.
    #[error(
        "{}: extends '{base}', which is not a recipe in {}",
        .file.display(),
        .library.display()
    )]
    NotARecipe {
        file: &'a Path,
        base: &'a str,
        library: &'a Path,
    },
    /// The base resolves somewhere other than directly in the library.
    #[error(
        "{}: extends '{base}', which resolves outside the library to {}; a base is composed \
         from and mounted into confined seats, so it must be a real directory in {}",
        .file.display(),
        .resolved.display(),
        .library.display()
    )]
    Outside {
        file: &'a Path,
        base: &'a str,
        resolved: PathBuf,
        library: &'a Path,
    },
    /// `extends` is not a string.
    #[error("{}: 'extends' must be the name of a recipe in the library", .file.display())]
    NotAString { file: &'a Path },
    /// `extends` is a string that is no recipe name.
    #[error(
        "{}: 'extends' value '{base}' is not a recipe name; names match ^[a-z0-9][a-z0-9-]*$ \
         and are refused before any path is built",
        .file.display()
    )]
    NotARecipeName { file: &'a Path, base: &'a str },
}

/// A layer's refusal as the compile refusal it has always been.
fn refused(refusal: LayerRefusal<'_>) -> CompileError {
    invalid(refusal.to_string())
}

#[cfg(test)]
mod tests;
