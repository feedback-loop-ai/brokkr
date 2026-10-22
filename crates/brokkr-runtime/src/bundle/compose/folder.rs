//! A seat that is the folder's own (decision 0091 ruling 1): a recipe's
//! seat that says `"from": "folder"` takes its `driver` and `hands` from
//! the folder bundle's own declaration of the same seat, read as written
//! from `<root>/.brokkr/bundle.json` and never composed, while its `class`,
//! `results` and `limits` stay the recipe's. The folder's layer is then
//! pinned as an ancestor's is, under a `@folder/<name>` key in the leaf's
//! manifest, and its root joins the bundle's roots after every composed
//! layer, so 0048's spawn walk checks `.brokkr/scripts/` against the map
//! the compile kept. Where no folder stands, the recipe's own declaration
//! is what compiles.

use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};
use thiserror::Error;

use super::{layers, Ancestor, Consumed, Layer, Merged, COMPOSE_PREFIX};
use crate::bundle::{Bundle, CompileError};
use crate::places::{Origin, Shelves};

/// The reserved manifest namespace a folder layer's digest rides in.
pub(crate) const FOLDER_PREFIX: &str = "@folder/";

/// The seat key this module reads, and the one word it takes.
const FROM: &str = "from";

/// The keys a seat the folder supplies is read for, on the recipe's side
/// and on the folder's, and nothing else: any other key would compile and
/// deliver nothing, so it is refused rather than ignored (decision 0004).
const SUPPLIED_KEYS: [&str; 6] = ["class", "results", "limits", "driver", "hands", FROM];

/// Where a seat's driver and hands come from: the closed vocabulary of one
/// word `from` is read into (decision 0071 ruling 2).
enum SeatFrom {
    Folder,
}

/// Which declaration of a seat the folder supplies wrote a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Recipe,
    Folder,
}

impl fmt::Display for Side {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Side::Recipe => "recipe's",
            Side::Folder => "folder's",
        })
    }
}

/// A seat's `class` and `results` as written, in JSON, which the folder's
/// and the recipe's declarations must agree on; absent is `null`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub class: String,
    pub results: String,
}

impl Shape {
    fn of(seat: &Value) -> Shape {
        let written = |key| seat.get(key).unwrap_or(&Value::Null).to_string();
        Shape {
            class: written("class"),
            results: written("results"),
        }
    }
}

impl fmt::Display for Shape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} with results {}", self.class, self.results)
    }
}

/// A seat that cannot be the folder's own (decision 0091 ruling 1). Each
/// refuses before anything is written, and a caller matches the variant,
/// never the text (decision 0071 ruling 8).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FolderSeatRefusal {
    /// `from` says anything but `"folder"`.
    #[error(
        "seat '{seat}' says \"from\": {word}, and \"folder\" is the one word it takes (decision \
         0091): any other is refused rather than read as no source at all (decision 0071 ruling 2)"
    )]
    Word { seat: String, word: String },
    /// The recipe's seat names no driver to run where no folder stands.
    #[error(
        "seat '{seat}' says \"from\": \"folder\" and declares no driver of its own, which a \
         compile with no folder runs (decision 0091)"
    )]
    NoOwnDriver { seat: String },
    /// The folder bundle would take the seat from itself through its base.
    #[error(
        "{}: seat '{seat}' is the folder's own by its base '{base}' (decision 0091), and a folder \
         declares its own: declare seat '{seat}' in {}",
        .file.display(),
        .file.display()
    )]
    ThroughItsBase {
        file: PathBuf,
        seat: String,
        base: String,
    },
    /// The folder bundle declares no such seat.
    #[error(
        "seat '{seat}' is the folder's own (decision 0091), and {} declares no seat '{seat}'",
        .file.display()
    )]
    Undeclared { seat: String, file: PathBuf },
    /// Either declaration writes a key the seat is not read for.
    #[error(
        "seat '{seat}' is the folder's own (decision 0091), and the {side} declaration writes \
         '{key}', which a seat the folder supplies is never read for: it takes class, results, \
         limits, driver and hands alone, so any other key is refused rather than ignored \
         (decision 0004)"
    )]
    Unread {
        seat: String,
        side: Side,
        key: String,
    },
    /// The folder's declaration runs anything but its own pinned script.
    #[error(
        "seat '{seat}' is the folder's own (decision 0091), and the folder's is not a `{{brokkr}} \
         driver exec` dispatch of its own pinned script"
    )]
    NotItsOwnScript { seat: String },
    /// The two declarations disagree on what the seat is. Each shape is
    /// boxed, so a compile error stays small.
    #[error(
        "seat '{seat}' is the folder's own (decision 0091), and the folder's is {folder} where \
         the recipe's is {recipe}"
    )]
    Differs {
        seat: String,
        folder: Box<Shape>,
        recipe: Box<Shape>,
    },
    /// The folder's declaration writes no `hands`, so the seat would spawn
    /// with no hands and its script with no walk at its spawn.
    #[error(
        "seat '{seat}' is the folder's own (decision 0091), and the folder's declares no hands, \
         so its script would spawn without the walk that checks it: declare its hands in {}",
        .file.display()
    )]
    NoHands { seat: String, file: PathBuf },
}

fn refused(refusal: FolderSeatRefusal) -> CompileError {
    CompileError::Folder(refusal)
}

/// A seat the folder supplied, as the recipe's layer declared it before the
/// folder's driver and hands replaced them, and the index of that layer:
/// what a compile with no folder reads, so refusal 4 is held to it.
pub(crate) struct OwnSeat {
    pub(crate) raw: Value,
    pub(crate) origin: usize,
}

/// The folder layer a compile took a seat from, its declaring document's
/// read, and once sealed, its pin.
pub(super) struct Folder {
    name: String,
    dir: PathBuf,
    read: Consumed,
    sealed: Option<Ancestor>,
}

impl Folder {
    /// The folder `layer`, whose root joins `roots` and `origins` after
    /// every composed layer, which is the index [`take`] gave its seats.
    pub(super) fn of(layer: Layer, roots: &mut Vec<PathBuf>, origins: &mut Vec<Origin>) -> Folder {
        roots.push(layer.dir.clone());
        origins.push(Origin::Folder);
        let read = Consumed {
            file: layer.file,
            document: layer.read,
            table: None,
            charters: Vec::new(),
        };
        Folder {
            name: layer.name,
            dir: layer.dir,
            read,
            sealed: None,
        }
    }

    /// Seal the folder's own walk as an ancestor's is, composing nothing.
    pub(super) fn seal(&mut self) -> Result<(), CompileError> {
        let layer = (self.name.as_str(), None);
        self.sealed = Some(layers::sealed(&self.dir, layer, &[], &self.read)?);
        Ok(())
    }

    /// Write the folder's digest into the leaf's `manifest` under its
    /// `@folder/<name>` key, and hand back its pin for the bundle.
    pub(super) fn pin(&self, manifest: &mut Value) -> Ancestor {
        let sealed = self
            .sealed
            .clone()
            .expect("the folder is sealed before it is pinned");
        let digest = Value::String(sealed.digest.clone());
        manifest["files"][format!("{FOLDER_PREFIX}{}", self.name)] = digest;
        sealed
    }
}

/// The reserved prefix a manifest key is computed under, if any: the
/// composition chain's or a folder layer's. A walk refuses a real file
/// there, and a spawn's walk skips it.
pub(crate) fn reserved_prefix(key: &str) -> Option<&'static str> {
    [COMPOSE_PREFIX, FOLDER_PREFIX]
        .into_iter()
        .find(|prefix| key.starts_with(prefix))
}

/// The layer of `bundle` a spawn's script `directory` stands in, its
/// longest root, with the name and file map its compile pinned: the leaf's
/// own manifest, an ancestor's, or the folder's. `None` where no root
/// holds it, or no layer owns that root.
pub(crate) fn pinned_layer<'a>(
    bundle: &'a Bundle,
    directory: &Path,
) -> Option<(&'a PathBuf, &'a String, &'a Map<String, Value>)> {
    let layer = bundle
        .roots
        .iter()
        .filter(|root| directory.starts_with(root))
        .max_by_key(|root| root.components().count())?;
    if layer == &bundle.dir {
        return Some((layer, &bundle.name, bundle.manifest["files"].as_object()?));
    }
    let ancestor = bundle
        .chain
        .iter()
        .chain(&bundle.folder)
        .find(|ancestor| &ancestor.dir == layer)?;
    Some((layer, &ancestor.name, &ancestor.files))
}

/// Give every merged seat that says `"from": "folder"` the folder's driver
/// and hands. The folder is the chain's own folder layer where the recipe
/// composes one, else the folder `shelves` hold, else none, and then the
/// recipe's declaration stands. Returns the folder's layer where it was
/// read here, for [`Folder::of`]. Each seat refuses in the order the word,
/// its own driver, its own keys, its base, the folder's declaration, its
/// word, its keys, its script, its shape and its hands. That order holds
/// refusal 4 at its ruled place for the decision's own case, a seat that
/// declares no driver. A seat whose driver is present but which a compile
/// with no folder would not run is refused as refusal 4 after the
/// resolver's refusals (5, 1, 3, 2, 7), once the seats are compiled.
pub(super) fn take(
    merged: &mut Merged,
    layers: &[Layer],
    shelves: &Shelves,
) -> Result<Option<Layer>, CompileError> {
    let chained = layers
        .iter()
        .position(|layer| layer.origin == Origin::Folder);
    let mut taken = Vec::new();
    for (seat, value) in &merged.seats {
        let Some(SeatFrom::Folder) = from(seat, value)? else {
            continue;
        };
        if let Some(at) = chained {
            through_base(merged, layers, at, seat)?;
        }
        taken.push(seat.clone());
    }
    if taken.is_empty() {
        return Ok(None);
    }
    let read = match chained {
        Some(_) => None,
        None => shelved(shelves)?,
    };
    let (folder, index) = match (chained, &read) {
        (Some(at), _) => (&layers[at], at),
        (None, Some(layer)) => (layer, layers.len()),
        (None, None) => return Ok(None),
    };
    for seat in &taken {
        supply(merged, folder, index, seat)?;
    }
    Ok(read)
}

/// The folder `shelves` hold, read as a layer, canonical as every layer's
/// directory is; none where the compile stands in no folder.
fn shelved(shelves: &Shelves) -> Result<Option<Layer>, CompileError> {
    let Some(dir) = shelves.standing_folder() else {
        return Ok(None);
    };
    let (dir, mut read) = (dir.canonicalize()?, Vec::new());
    layers::read_layer(&mut read, (&dir, Origin::Folder), &None)?;
    Ok(read.pop())
}

/// What a declaration of `seat` says in `from`, read once into the closed
/// vocabulary, on either side: nothing, or the folder. Any other word
/// refuses.
fn word(seat: &str, value: &Value) -> Result<Option<SeatFrom>, CompileError> {
    let Some(word) = value.get(FROM) else {
        return Ok(None);
    };
    if word.as_str() != Some("folder") {
        let (seat, word) = (seat.to_string(), word.to_string());
        return Err(refused(FolderSeatRefusal::Word { seat, word }));
    }
    Ok(Some(SeatFrom::Folder))
}

/// What the recipe's seat's `from` says, by [`word`]. A seat that is the
/// folder's own must still declare a driver of its own, and no key it is
/// not read for. Where a folder supplies the seat, the compile holds that
/// driver to what a compile with no folder runs once the table is read
/// ([`OwnSeat`]), because the folder's replaces it before any parser
/// reads it; where none stands, the compile reads it itself.
fn from(seat: &str, value: &Value) -> Result<Option<SeatFrom>, CompileError> {
    let Some(SeatFrom::Folder) = word(seat, value)? else {
        return Ok(None);
    };
    if value.get("driver").is_none() {
        let seat = seat.to_string();
        return Err(refused(FolderSeatRefusal::NoOwnDriver { seat }));
    }
    admit(seat, Side::Recipe, value)?;
    Ok(Some(SeatFrom::Folder))
}

/// Refuse a key the `side` declaration of `seat` writes beyond
/// [`SUPPLIED_KEYS`].
fn admit(seat: &str, side: Side, value: &Value) -> Result<(), CompileError> {
    let unread = value
        .as_object()
        .into_iter()
        .flat_map(Map::keys)
        .find(|key| !SUPPLIED_KEYS.contains(&key.as_str()));
    match unread {
        None => Ok(()),
        Some(key) => Err(refused(FolderSeatRefusal::Unread {
            seat: seat.to_string(),
            side,
            key: key.clone(),
        })),
    }
}

/// Refuse a seat the chain's folder layer, at `at`, would take from itself
/// through its base: one a layer beneath it supplied, which the folder
/// does not declare as its own.
fn through_base(
    merged: &Merged,
    layers: &[Layer],
    at: usize,
    seat: &str,
) -> Result<(), CompileError> {
    if merged.seat_from[seat] <= at {
        return Ok(());
    }
    let folder = &layers[at];
    let base = folder.document["extends"]
        .as_str()
        .expect("a layer with a base names it");
    Err(refused(FolderSeatRefusal::ThroughItsBase {
        file: folder.file.clone(),
        seat: seat.to_string(),
        base: base.to_string(),
    }))
}

/// Take `seat`'s driver and hands from `folder`'s own declaration, which
/// must exist, say nothing in `from` but what [`word`] reads, write
/// nothing the seat is not read for, dispatch the folder's own pinned
/// script, agree with the recipe's class and results, and declare hands,
/// read last, so the spawn walks the folder's map; the seat is then the
/// folder layer's, at `index`. A declaration that says `"from": "folder"`
/// is the folder's own seat, which takes itself.
fn supply(
    merged: &mut Merged,
    folder: &Layer,
    index: usize,
    seat: &str,
) -> Result<(), CompileError> {
    let declared = folder
        .document
        .get("seats")
        .and_then(|seats| seats.get(seat));
    let declared = declared.ok_or_else(|| {
        let file = folder.file.clone();
        refused(FolderSeatRefusal::Undeclared {
            seat: seat.to_string(),
            file,
        })
    })?;
    let (None | Some(SeatFrom::Folder)) = word(seat, declared)?;
    admit(seat, Side::Folder, declared)?;
    let parts = crate::bundle::command_parts(declared);
    let exec = crate::bundle::dispatch_driver(&parts).as_deref() == Some("exec");
    if !exec || crate::bundle::pinned_script(&folder.dir, &parts).is_err() {
        let seat = seat.to_string();
        return Err(refused(FolderSeatRefusal::NotItsOwnScript { seat }));
    }
    let (theirs, ours) = (Shape::of(declared), Shape::of(&merged.seats[seat]));
    if theirs != ours {
        let seat = seat.to_string();
        let (folder, recipe) = (Box::new(theirs), Box::new(ours));
        return Err(refused(FolderSeatRefusal::Differs {
            seat,
            folder,
            recipe,
        }));
    }
    if declared.get("hands").is_none() {
        let (seat, file) = (seat.to_string(), folder.file.clone());
        return Err(refused(FolderSeatRefusal::NoHands { seat, file }));
    }
    let recipes = OwnSeat {
        raw: merged.seats[seat].clone(),
        origin: merged.seat_from[seat],
    };
    merged.own_seats.insert(seat.to_string(), recipes);
    let own = merged.seats[seat]
        .as_object_mut()
        .expect("a seat with a `from` is an object");
    own.insert("driver".into(), declared["driver"].clone());
    own.remove("hands");
    own.extend(
        declared
            .get("hands")
            .map(|hands| ("hands".into(), hands.clone())),
    );
    merged.seat_from.insert(seat.to_string(), index);
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
