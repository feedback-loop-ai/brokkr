//! An adapter's `models` map, with the tier a model may declare
//! (proposed decision 0075 ruling 5). An abstract name maps to its
//! concrete id, as it always has, or to `{"id", "tier"}` where the model
//! is provisional. A model that declares no tier, bare id or object, is
//! promoted, which is every model shipped before the tier existed, so
//! every adapter on disk reads exactly as it did. Promotion is data only:
//! removing the tier leaves the model mapped to the same id.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::{invalid, name_map, only_keys, string, LibraryError};

/// The one tier a model declares. Absence is the other one, promoted.
const PROVISIONAL: &str = "provisional";

/// The abstract name → concrete id map every reader already reads, and
/// the names the adapter marks provisional.
pub(super) fn models(
    map: &Map<String, Value>,
    what: &str,
) -> Result<(BTreeMap<String, String>, BTreeSet<String>), LibraryError> {
    let mut ids = Map::new();
    let mut provisional = BTreeSet::new();
    for (name, value) in map
        .get("models")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let Some(declared) = value.as_object() else {
            ids.insert(name.clone(), value.clone());
            continue;
        };
        let at = format!("{what} 'models.{name}'");
        only_keys(declared, &["id", "tier"], &at)?;
        let id = string(declared, "id", &at)?;
        // A written `tier` is read, and one that is not the word is
        // refused; only an entry that leaves the key out is promoted.
        if declared.contains_key("tier") {
            let tier = string(declared, "tier", &at)?;
            if tier != PROVISIONAL {
                return invalid(format!(
                    "{at} declares tier '{tier}'; the only tier a model declares is \
                     \"{PROVISIONAL}\", and a model that declares none is promoted \
                     (proposed decision 0075 ruling 5)"
                ));
            }
            provisional.insert(name.clone());
        }
        ids.insert(name.clone(), Value::String(id));
    }
    // The flattened map is judged by the reader every string map shares,
    // so a name, an empty id and a `models` that is missing or no object
    // at all are refused in the words they always were.
    let mut flattened = map.clone();
    if let Some(slot) = flattened.get_mut("models").filter(|slot| slot.is_object()) {
        *slot = Value::Object(ids);
    }
    Ok((name_map(&flattened, "models", what)?, provisional))
}
