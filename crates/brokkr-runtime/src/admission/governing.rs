//! The governing facts of a realm admission compares (the operator's
//! realm-drift ruling, 2026-10-04), and what differs between two of them.

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::realms::{Boundary, CapabilityGrant, Realm};
use serde_json::Value;

use super::Difference;
use crate::realms::{World, WorldError};

/// The facts of the operated repository's realm that govern a run in it
/// (the operator's realm-drift ruling, 2026-10-04): which realm it is, its
/// boundary, its grants, its house rules and its dialect. Each is held
/// WHOLE, so a fact a later map version adds to any of them, a grant key
/// above all, is compared without this code naming it. A repository the
/// map does not name, or one under no map at all, is governed as a run
/// with no realm is: `namespace`, no grant, no house, no dialect.
#[derive(Debug)]
pub(super) struct Governing {
    pub(super) realm: Option<String>,
    boundary: Boundary,
    grants: BTreeMap<String, CapabilityGrant>,
    house: Value,
    dialect: Value,
}

/// The facts that govern a run in `realm` of `world`, `None` where the
/// world names the repository no realm, or under no map at all.
pub(super) fn governing(
    selected: Option<(&World, Option<&Realm>)>,
) -> Result<Governing, WorldError> {
    let Some((world, realm)) = selected else {
        return Ok(Governing {
            realm: None,
            boundary: Boundary::Namespace,
            grants: BTreeMap::new(),
            house: Value::Null,
            dialect: Value::Null,
        });
    };
    let pin = world.pin_of(realm)?;
    Ok(Governing {
        realm: realm.map(|realm| realm.name.clone()),
        boundary: realm.map_or(Boundary::Namespace, Realm::boundary),
        grants: realm.map(|realm| realm.grants.clone()).unwrap_or_default(),
        house: unplaced(&pin, "house"),
        dialect: unplaced(&pin, "dialect"),
    })
}

/// A text a world pins, whole but for the path it was read from: the
/// same text read from the same map is the same fact wherever the map is
/// opened from. `null` when the realm names none.
fn unplaced(pin: &Value, key: &str) -> Value {
    let mut text = pin[key].clone();
    if let Value::Object(fields) = &mut text {
        fields.remove("source");
    }
    text
}

/// Every governing fact that is not the same, in the order the ruling
/// names them. Grants compare as whole grants.
pub(super) fn differences(held: &Governing, now: &Governing) -> Vec<Difference> {
    let mut found = Vec::new();
    if held.realm != now.realm {
        found.push(Difference::Realm {
            was: held.realm.clone(),
            now: now.realm.clone(),
        });
    }
    let names: BTreeSet<&String> = held.grants.keys().chain(now.grants.keys()).collect();
    for name in names {
        match (held.grants.get(name), now.grants.get(name)) {
            (Some(_), None) => found.push(Difference::GrantRemoved(name.clone())),
            (None, Some(_)) => found.push(Difference::GrantAdded(name.clone())),
            (Some(was), Some(is)) if was != is => {
                found.push(Difference::GrantChanged(name.clone()))
            }
            (Some(_), Some(_)) | (None, None) => {}
        }
    }
    if held.boundary != now.boundary {
        found.push(Difference::Boundary {
            was: held.boundary,
            now: now.boundary,
        });
    }
    if held.house != now.house {
        found.push(Difference::House);
    }
    if held.dialect != now.dialect {
        found.push(Difference::Dialect);
    }
    found
}
