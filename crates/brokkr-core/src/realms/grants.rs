//! The grant (decision 0065 ruling 3), read by the version of the map that
//! wrote it. Under `forge.realms/v6` and `v7` the engine owns three keys of a
//! grant and every other key is the selected dialect's restriction, `retain`
//! included. `forge.realms/v8` reserves a fourth, `retain`, whose one legal
//! value is `false`: the realm may veto a dialect's retention, never require
//! it (slice two, SC2 and CR1). Which keys are reserved follows the version,
//! so an older map's restriction is never read as the newer veto by spelling
//! alone.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use thiserror::Error;

use super::{is_name, older_than, RealmsError, SCHEMA_V8};

/// The keys of a grant the engine owns under `forge.realms/v6` and `v7`;
/// every other key is the dialect's.
pub const GRANT_KEYS: [&str; 3] = ["dialect", "tools", "offices"];

/// The keys of a grant the engine owns from `forge.realms/v8`.
const VETO_GRANT_KEYS: [&str; 4] = ["dialect", "tools", "offices", "retain"];

/// What a grant says about retaining the responses of the capability it
/// grants, which is a fact of the map version that wrote it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantRetention {
    /// A `forge.realms/v6` or `v7` grant: the version reserves no `retain`,
    /// so a written one is a restriction and the dialect's declaration
    /// stands.
    Unreserved,
    /// A `forge.realms/v8` grant that leaves `retain` out: the dialect's
    /// declaration stands.
    Inherit,
    /// A `forge.realms/v8` grant writing `retain: false`.
    Veto,
}

impl GrantRetention {
    /// The keys of the grant the engine owns; every other key is a
    /// restriction the selected dialect's schema defines.
    pub fn reserved_keys(self) -> &'static [&'static str] {
        match self {
            GrantRetention::Unreserved => &GRANT_KEYS,
            GrantRetention::Inherit | GrantRetention::Veto => &VETO_GRANT_KEYS,
        }
    }
}

/// One capability a realm grants (decision 0065 ruling 3): the tool
/// dialect that serves it here, and how the grant is narrowed. The two
/// lists keep absence apart from emptiness, because they mean opposite
/// things: no `offices` reaches every requesting office and `[]` reaches
/// none; no `tools` admits the dialect's whole set and `[]` admits none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant {
    /// A tool dialect's name under `dialects/tools/`.
    pub dialect: String,
    pub tools: Option<Vec<String>>,
    pub offices: Option<Vec<String>>,
    /// The retention veto, or that this map version cannot write one.
    pub retention: GrantRetention,
    /// Every other key of the grant, exactly as written. The selected
    /// dialect's schema defines them; this crate interprets none.
    pub restrictions: Map<String, Value>,
}

impl CapabilityGrant {
    /// The grant as the realm wrote it: what a manifest pins and what a
    /// restriction schema is asked about.
    pub fn value(&self) -> Value {
        let mut grant = self.restrictions.clone();
        grant.insert("dialect".into(), Value::String(self.dialect.clone()));
        for (key, list) in [("tools", &self.tools), ("offices", &self.offices)] {
            if let Some(list) = list {
                grant.insert(key.into(), serde_json::json!(list));
            }
        }
        if self.retention == GrantRetention::Veto {
            grant.insert("retain".into(), Value::Bool(false));
        }
        Value::Object(grant)
    }

    /// Does this grant reach the office? Absent scope reaches every
    /// office that asks.
    pub fn reaches(&self, office: &str) -> bool {
        self.offices
            .as_ref()
            .is_none_or(|offices| offices.iter().any(|named| named == office))
    }
}

/// Why a realm's written `capabilities` map is refused, in the map's own
/// voice: each names the realm, the capability and the field.
#[derive(Debug, Error, PartialEq, Eq)]
pub(super) enum GrantError {
    #[error(
        "realm '{realm}' writes capabilities as {written}; a capabilities map is an object \
         from capability name to grant, and a realm that grants nothing leaves the word out"
    )]
    NotAMap { realm: String, written: Value },
    #[error(
        "realm '{realm}' grants a capability named '{capability}'; a capability name is \
         lowercase letters, digits, '.', '_' and '-', starting with a letter or digit"
    )]
    Name { realm: String, capability: String },
    #[error(
        "realm '{realm}' grants capability '{capability}' as {grant}; a grant is an object \
         naming the tool dialect that serves the capability"
    )]
    NotAGrant {
        realm: String,
        capability: String,
        grant: Value,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' without a tool dialect name; \
         'dialect' names a file under dialects/tools/ in the realm-name grammar"
    )]
    Dialect { realm: String, capability: String },
    #[error(
        "realm '{realm}' grants capability '{capability}' with a malformed '{field}'; it \
         is a list of distinct non-empty strings, and leaving it out is how a \
         grant says all"
    )]
    List {
        realm: String,
        capability: String,
        field: &'static str,
    },
    #[error(
        "realm '{realm}' capability '{capability}': retain must be false when present; the \
         realm may veto retention, never require it"
    )]
    Retain { realm: String, capability: String },
}

/// A list of distinct non-empty strings, or no list at all.
fn distinct_names(value: Option<&Value>) -> Option<Option<Vec<String>>> {
    let Some(value) = value else {
        return Some(None);
    };
    let mut names: Vec<String> = Vec::new();
    for item in value.as_array()? {
        let name = item.as_str().filter(|name| !name.trim().is_empty())?;
        if names.iter().any(|known| known == name) {
            return None;
        }
        names.push(name.to_string());
    }
    Some(Some(names))
}

/// Judge one realm's written `capabilities` map under the map's `schema`,
/// refusing as the map at `path`.
pub(super) fn parse_grants(
    path: &str,
    realm: &str,
    schema: &str,
    written: &Value,
) -> Result<BTreeMap<String, CapabilityGrant>, RealmsError> {
    grants_of(realm, schema, written).map_err(|error| RealmsError::Invalid {
        path: path.to_string(),
        problem: error.to_string(),
    })
}

fn grants_of(
    realm: &str,
    schema: &str,
    written: &Value,
) -> Result<BTreeMap<String, CapabilityGrant>, GrantError> {
    let Some(map) = written.as_object() else {
        return Err(GrantError::NotAMap {
            realm: realm.to_string(),
            written: written.clone(),
        });
    };
    let reserving = !older_than(schema, SCHEMA_V8);
    map.iter()
        .map(|(name, grant)| {
            parse_grant(realm, name, grant, reserving).map(|grant| (name.clone(), grant))
        })
        .collect()
}

/// Judge one grant. `reserving` is whether the map's version reserves
/// `retain`.
fn parse_grant(
    realm: &str,
    name: &str,
    grant: &Value,
    reserving: bool,
) -> Result<CapabilityGrant, GrantError> {
    let (realm, capability) = (realm.to_string(), name.to_string());
    if !is_name(name) {
        return Err(GrantError::Name { realm, capability });
    }
    let Some(fields) = grant.as_object() else {
        return Err(GrantError::NotAGrant {
            realm,
            capability,
            grant: grant.clone(),
        });
    };
    let Some(dialect) = fields
        .get("dialect")
        .and_then(Value::as_str)
        .filter(|dialect| is_name(dialect))
    else {
        return Err(GrantError::Dialect { realm, capability });
    };
    let mut lists = [None, None];
    for (slot, field) in lists.iter_mut().zip(["tools", "offices"]) {
        let Some(list) = distinct_names(fields.get(field)) else {
            return Err(GrantError::List {
                realm,
                capability,
                field,
            });
        };
        *slot = list;
    }
    let [tools, offices] = lists;
    let retention = match (reserving, fields.get("retain")) {
        (false, _) => GrantRetention::Unreserved,
        (true, None) => GrantRetention::Inherit,
        (true, Some(Value::Bool(false))) => GrantRetention::Veto,
        (true, Some(_)) => return Err(GrantError::Retain { realm, capability }),
    };
    let reserved = retention.reserved_keys();
    Ok(CapabilityGrant {
        dialect: dialect.to_string(),
        tools,
        offices,
        retention,
        restrictions: fields
            .iter()
            .filter(|(key, _)| !reserved.contains(&key.as_str()))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    })
}
