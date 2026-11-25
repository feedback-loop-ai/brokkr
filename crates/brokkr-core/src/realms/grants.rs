//! The grant (decision 0065 ruling 3), read by the version of the map that
//! wrote it. Under `forge.realms/v6` and `v7` the engine owns three keys of a
//! grant and every other key is the selected dialect's restriction, `retain`
//! included. `forge.realms/v8` reserves a fourth, `retain`, whose one legal
//! value is `false`: the realm may veto a dialect's retention, never require
//! it (slice two, SC2 and CR1). Which keys are reserved follows the version,
//! so an older map's restriction is never read as the newer veto by spelling
//! alone. Under v9, per-provider dialects share only offices and retention.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use thiserror::Error;

use super::{is_name, older_than, SCHEMA_V8, SCHEMA_V9};

/// The keys of a grant the engine owns under `forge.realms/v6` and `v7`;
/// every other key is the dialect's.
pub const GRANT_KEYS: [&str; 3] = ["dialect", "tools", "offices"];

/// The keys of a single-dialect grant the engine owns from `forge.realms/v8`.
const VETO_GRANT_KEYS: [&str; 4] = ["dialect", "tools", "offices", "retain"];

/// The entire vocabulary of a per-provider grant under v9.
const PROVIDER_GRANT_KEYS: [&str; 3] = ["dialects", "offices", "retain"];

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
    fn reserved_keys(self) -> &'static [&'static str] {
        match self {
            GrantRetention::Unreserved => &GRANT_KEYS,
            GrantRetention::Inherit | GrantRetention::Veto => &VETO_GRANT_KEYS,
        }
    }
}

/// The tool dialects under `dialects/tools/` a grant names (decision 0126).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantDialect {
    /// One dialect to judge for every provider.
    Single(String),
    /// Each provider may use only the dialect named under its own key.
    PerProvider(BTreeMap<String, String>),
}

impl From<&str> for GrantDialect {
    fn from(dialect: &str) -> Self {
        Self::Single(dialect.into())
    }
}

/// One capability a realm grants (decision 0065 ruling 3): the tool
/// dialect that serves it here, and how the grant is narrowed. The two
/// lists keep absence apart from emptiness, because they mean opposite
/// things: no `offices` reaches every requesting office and `[]` reaches
/// none; no `tools` admits the dialect's whole set and `[]` admits none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub dialect: GrantDialect,
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
        let (key, dialect) = match &self.dialect {
            GrantDialect::Single(name) => ("dialect", Value::String(name.clone())),
            GrantDialect::PerProvider(names) => ("dialects", serde_json::json!(names)),
        };
        grant.insert(key.into(), dialect);
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

/// A list a grant may write to narrow itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantList {
    Tools,
    Offices,
}

impl GrantList {
    /// The field as a grant spells it.
    pub(crate) fn field(self) -> &'static str {
        match self {
            GrantList::Tools => "tools",
            GrantList::Offices => "offices",
        }
    }
}

/// Why a realm's written `capabilities` map is refused, in the map's own
/// voice: each names the realm, the capability and the field. A value the
/// realm wrote is carried as its JSON text.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GrantError {
    #[error(
        "realm '{realm}' writes capabilities as {written}; a capabilities map is an object \
         from capability name to grant, and a realm that grants nothing leaves the word out"
    )]
    NotAMap { realm: String, written: String },
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
        grant: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' without a tool dialect name; \
         'dialect' names a file under dialects/tools/ in the realm-name grammar"
    )]
    Dialect { realm: String, capability: String },
    #[error(
        "realm '{realm}' grants capability '{capability}' with both 'dialect' and 'dialects'; \
         write exactly one"
    )]
    BothDialects { realm: String, capability: String },
    #[error(
        "realm '{realm}' grants capability '{capability}' with neither 'dialect' nor 'dialects'; \
         write exactly one"
    )]
    NoDialect { realm: String, capability: String },
    #[error(
        "realm '{realm}' grants capability '{capability}' with malformed 'dialects'; \
         write a non-empty object from provider name to tool dialect name"
    )]
    Dialects { realm: String, capability: String },
    #[error(
        "realm '{realm}' grants capability '{capability}' with invalid provider name \
         '{provider}' in 'dialects'; use the realm-name grammar"
    )]
    DialectProvider {
        realm: String,
        capability: String,
        provider: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' with an invalid tool dialect name \
         for provider '{provider}'; use the realm-name grammar"
    )]
    ProviderDialect {
        realm: String,
        capability: String,
        provider: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' with '{key}' beside 'dialects'; \
         only 'offices' and 'retain' may accompany 'dialects'"
    )]
    ProviderGrantKey {
        realm: String,
        capability: String,
        key: String,
    },
    #[error(
        "realm '{realm}' grants capability '{capability}' with a malformed '{}'; it \
         is a list of distinct non-empty strings, and leaving it out is how a \
         grant says all",
        .list.field()
    )]
    List {
        realm: String,
        capability: String,
        list: GrantList,
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

/// Judge one realm's written `capabilities` map under the map's `schema`.
pub(super) fn parse_grants(
    realm: &str,
    schema: &str,
    written: &Value,
) -> Result<BTreeMap<String, CapabilityGrant>, GrantError> {
    let Some(map) = written.as_object() else {
        return Err(GrantError::NotAMap {
            realm: realm.to_string(),
            written: written.to_string(),
        });
    };
    map.iter()
        .map(|(name, grant)| {
            parse_grant(realm, name, grant, schema).map(|grant| (name.clone(), grant))
        })
        .collect()
}

fn parse_dialect(
    realm: &str,
    capability: &str,
    fields: &Map<String, Value>,
    schema: &str,
) -> Result<GrantDialect, GrantError> {
    let single = fields.get("dialect");
    let names = if older_than(schema, SCHEMA_V9) {
        None
    } else {
        fields.get("dialects")
    };
    if let Some(names) = names {
        if single.is_some() {
            return Err(GrantError::BothDialects {
                realm: realm.into(),
                capability: capability.into(),
            });
        }
        return parse_provider_dialects(realm, capability, fields, names);
    }
    if schema == SCHEMA_V9 && single.is_none() {
        return Err(GrantError::NoDialect {
            realm: realm.into(),
            capability: capability.into(),
        });
    }
    single
        .and_then(Value::as_str)
        .filter(|dialect| is_name(dialect))
        .map(GrantDialect::from)
        .ok_or_else(|| GrantError::Dialect {
            realm: realm.into(),
            capability: capability.into(),
        })
}

fn parse_provider_dialects(
    realm: &str,
    capability: &str,
    fields: &Map<String, Value>,
    names: &Value,
) -> Result<GrantDialect, GrantError> {
    if let Some(key) = fields
        .keys()
        .find(|key| !PROVIDER_GRANT_KEYS.contains(&key.as_str()))
    {
        return Err(GrantError::ProviderGrantKey {
            realm: realm.into(),
            capability: capability.into(),
            key: key.clone(),
        });
    }
    let Some(names) = names.as_object().filter(|names| !names.is_empty()) else {
        return Err(GrantError::Dialects {
            realm: realm.into(),
            capability: capability.into(),
        });
    };
    let mut dialects = BTreeMap::new();
    for (provider, dialect) in names {
        if !is_name(provider) {
            return Err(GrantError::DialectProvider {
                realm: realm.into(),
                capability: capability.into(),
                provider: provider.clone(),
            });
        }
        let name = dialect
            .as_str()
            .filter(|name| is_name(name))
            .ok_or_else(|| GrantError::ProviderDialect {
                realm: realm.into(),
                capability: capability.into(),
                provider: provider.clone(),
            })?;
        dialects.insert(provider.clone(), name.into());
    }
    Ok(GrantDialect::PerProvider(dialects))
}

/// Judge one grant by the version that wrote it.
fn parse_grant(
    realm: &str,
    name: &str,
    grant: &Value,
    schema: &str,
) -> Result<CapabilityGrant, GrantError> {
    let (realm, capability) = (realm.to_string(), name.to_string());
    if !is_name(name) {
        return Err(GrantError::Name { realm, capability });
    }
    let Some(fields) = grant.as_object() else {
        return Err(GrantError::NotAGrant {
            realm,
            capability,
            grant: grant.to_string(),
        });
    };
    let dialect = parse_dialect(&realm, &capability, fields, schema)?;
    let mut lists = [None, None];
    for (slot, list) in lists.iter_mut().zip([GrantList::Tools, GrantList::Offices]) {
        let Some(names) = distinct_names(fields.get(list.field())) else {
            return Err(GrantError::List {
                realm,
                capability,
                list,
            });
        };
        *slot = names;
    }
    let [tools, offices] = lists;
    let retention = match (!older_than(schema, SCHEMA_V8), fields.get("retain")) {
        (false, _) => GrantRetention::Unreserved,
        (true, None) => GrantRetention::Inherit,
        (true, Some(Value::Bool(false))) => GrantRetention::Veto,
        (true, Some(_)) => return Err(GrantError::Retain { realm, capability }),
    };
    let reserved = match &dialect {
        GrantDialect::Single(_) => retention.reserved_keys(),
        GrantDialect::PerProvider(_) => &PROVIDER_GRANT_KEYS,
    };
    Ok(CapabilityGrant {
        dialect,
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
