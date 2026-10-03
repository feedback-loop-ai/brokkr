//! `forge.realms/v7`'s one word (proposed decision 0075 ruling 5): the
//! world-level `provisional_offices`, the agents a model its adapter
//! declares provisional may be seated in. Absent or empty, a provisional
//! model is seated nowhere, and a v7 map that does not name the list reads
//! exactly as a v6 map does. The word is refused under every older label,
//! and a written `null`, an empty name or a name listed twice is refused
//! here as `realms.v7` refuses it. What the list admits is the compile's
//! work, in `brokkr-runtime`.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::{older_than, RealmMap, RealmsError, SCHEMA_V7};

/// A written `null` is read as no office only so that [`judge`] refuses it
/// on the map as written, in its own words, rather than serde in anonymous
/// ones.
pub(super) fn null_as_none<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<String>, D::Error> {
    Option::<Vec<String>>::deserialize(deserializer).map(Option::unwrap_or_default)
}

/// The list, held to its version exactly as every word before it is held
/// to its own. Presence is judged on the map as WRITTEN, because serde
/// reads a written `null` as the absence it is not.
pub(super) fn judge(path: &str, map: &RealmMap, content: &Value) -> Result<(), RealmsError> {
    let invalid = |problem: String| {
        Err(RealmsError::Invalid {
            path: path.to_string(),
            problem,
        })
    };
    let Some(written) = content.get("provisional_offices") else {
        return Ok(());
    };
    if older_than(&map.schema, SCHEMA_V7) {
        return invalid(format!(
            "it names provisional offices, which is {SCHEMA_V7} vocabulary in a map calling \
             itself {}",
            map.schema
        ));
    }
    if written.is_null() {
        return invalid(
            "it writes provisional_offices as null; the list is an array, and a map that \
             lists no office leaves the word out"
                .to_string(),
        );
    }
    let offices = &map.provisional_offices;
    for (index, office) in offices.iter().enumerate() {
        if office.trim().is_empty() {
            return invalid(format!("provisional office {index} is empty"));
        }
        if offices[..index].contains(office) {
            return invalid(format!("provisional office '{office}' is listed twice"));
        }
    }
    Ok(())
}
