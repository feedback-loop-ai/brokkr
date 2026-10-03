//! An adapter's `models` map, with the tier a model may declare
//! (proposed decision 0075 ruling 5). An abstract name maps to its
//! concrete id, as it always has, or to `{"id", "tier"}` where the model
//! is provisional. A model that declares no tier, bare id or object, is
//! promoted, which is every model shipped before the tier existed, so
//! every adapter on disk reads exactly as it did. Promotion is data only:
//! removing the tier leaves the model mapped to the same id. The tier
//! judges every pin a seat composes, so an adapter's own argv may carry
//! none.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Map, Value};

use super::{invalid, name_map, only_keys, string, Adapter, LibraryError};
use crate::bundle::short_flag;

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

/// An adapter file as every load refusal names it; its provider is its
/// file name, so the two read the same.
pub(super) fn described(name: &str, path: &Path) -> String {
    format!("adapter '{name}' ({})", path.display())
}

/// The flags every harness this fleet has met takes a model on, beside the
/// adapter's own `model_flag`: the primary pin, and the model a run falls
/// to. The tier reads both on a seat's own argv.
const PIN_FLAGS: [&str; 2] = ["--model", "--fallback-model"];

/// The adapter, once no argv it supplies names a model. A pin is the
/// engine's to compose from a seat's chain, where the tier judges it; one
/// the adapter carried in its driver tail or a hands fragment would ride
/// behind every seat it serves, a gate's included, and no tier check reads
/// it. So a fragment word that is a pin flag, in either spelling, or a
/// short `model_flag` with its value attached, is refused here, at load.
pub(super) fn pinless(adapter: Adapter, path: &Path) -> Result<Adapter, LibraryError> {
    let what = described(&adapter.provider, path);
    let flags: Vec<&str> = PIN_FLAGS
        .into_iter()
        .chain(adapter.model_flag.as_deref())
        .collect();
    let fragments = [
        ("driver", Some(&adapter.driver)),
        ("hands.workspace", adapter.hands.as_ref()),
        ("hands.harness.gate", adapter.harness.gate.as_ref()),
        ("hands.harness.work", adapter.harness.work.as_ref()),
    ];
    for (key, words) in fragments {
        let named = words.into_iter().flatten().find_map(|word| {
            flags.iter().find(|flag| {
                *word == **flag
                    || word.starts_with(&format!("{flag}="))
                    || (short_flag(flag) && word.starts_with(**flag))
            })
        });
        if let Some(flag) = named {
            return invalid(format!(
                "{what} '{key}' names the model flag '{flag}'; a model pin is the engine's to \
                 compose from a seat's chain, where the provisional tier judges it, and one an \
                 adapter carries would seat a model no tier check reads (proposed decision 0075 \
                 ruling 5)"
            ));
        }
    }
    Ok(adapter)
}
