//! The map of the world (decision 0023, phase 1): `forge.realms/v1`.
//!
//! A repository is a realm; the map that holds them is the connective
//! truth and is not itself a realm. This module is the PURE half — the
//! shape, the refusals, the content digest, and the per-realm fact
//! lookup. Reading a file, resolving a path against a workspace and
//! asking git anything all live in `forge-runtime::realms`, because this
//! crate performs no I/O (decision 0003, constitutional boundary 1).
//!
//! The v1 shape is minimal by ruling: the realms — each a name, a path
//! and a default branch — and the world's journal. Nothing else.
//! Decision 0021's per-realm driver and egress constraints are a later
//! amendment, deliberately not speculatively schema'd; unknown fields
//! are REFUSED here so that amendment must arrive as a version rather
//! than as drift in a file still calling itself v1.

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

/// The one shape this build reads. A map calling itself anything else is
/// refused by name, never read hopefully.
pub const SCHEMA_V1: &str = "forge.realms/v1";

/// The file an invocation defaults to when it names no map.
pub const DEFAULT_MAP_FILE: &str = "realms.json";

/// The key repository facts were recorded under before any map existed.
/// Every journal written that way keeps folding exactly as it did: the
/// per-realm lookup falls back to this key, so an unkeyed head is still
/// the head the ship gate compares against.
pub const LEGACY_REALM_KEY: &str = "repo";

#[derive(Debug, Error, PartialEq)]
pub enum RealmsError {
    #[error("{path} is not a readable realms map: {detail}")]
    Malformed { path: String, detail: String },
    #[error("{path} is not a usable realms map: {problem}")]
    Invalid { path: String, problem: String },
}

/// One repository in the world.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Realm {
    /// The realm's identity, and the key its facts are journaled under.
    pub name: String,
    /// Absolute, or relative to the directory the map file lives in.
    pub path: String,
    /// The branch this realm's work is measured against.
    pub default_branch: String,
}

/// The map as written: realms, and the journal the world writes.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealmMap {
    pub schema: String,
    pub realms: Vec<Realm>,
    pub journal: String,
}

/// A realm name is a journal key: lowercase, digits, and the three
/// separators a repository name already uses. Refused early, because a
/// name that cannot be read back out of evidence is not a name.
fn is_name(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c))
}

impl RealmMap {
    /// Parse and validate one map's text. `path` names the file only so
    /// a refusal can cite it; nothing is read from disk here.
    pub fn parse(path: &str, text: &str) -> Result<(RealmMap, Value), RealmsError> {
        let content: Value =
            serde_json::from_str(text).map_err(|error| RealmsError::Malformed {
                path: path.to_string(),
                detail: error.to_string(),
            })?;
        RealmMap::of(path, content)
    }

    /// Validate one map already parsed into a value — the shape a reader
    /// holds when the map arrives embedded in a run's manifest pin rather
    /// than as a file. Same refusals, same words: a world read back out
    /// of evidence is held to what it was held to going in.
    pub fn of(path: &str, content: Value) -> Result<(RealmMap, Value), RealmsError> {
        let map: RealmMap =
            serde_json::from_value(content.clone()).map_err(|error| RealmsError::Malformed {
                path: path.to_string(),
                detail: error.to_string(),
            })?;
        let invalid = |problem: String| RealmsError::Invalid {
            path: path.to_string(),
            problem,
        };
        if map.schema != SCHEMA_V1 {
            return Err(invalid(format!(
                "it calls itself '{}'; this build reads {SCHEMA_V1}",
                map.schema
            )));
        }
        if map.realms.is_empty() {
            return Err(invalid("it names no realms".to_string()));
        }
        if map.journal.trim().is_empty() {
            return Err(invalid("its journal is empty".to_string()));
        }
        for (index, realm) in map.realms.iter().enumerate() {
            if !is_name(&realm.name) {
                return Err(invalid(format!(
                    "realm {index} is named '{}'; a realm name is lowercase letters, \
                     digits, '.', '_' and '-', starting with a letter or digit",
                    realm.name
                )));
            }
            if realm.path.trim().is_empty() {
                return Err(invalid(format!("realm '{}' has no path", realm.name)));
            }
            if realm.default_branch.trim().is_empty() {
                return Err(invalid(format!(
                    "realm '{}' has no default branch",
                    realm.name
                )));
            }
            if map.realms[..index].iter().any(|e| e.name == realm.name) {
                return Err(invalid(format!("realm '{}' is named twice", realm.name)));
            }
        }
        Ok((map, content))
    }
}

/// The head recorded for one realm, accepting BOTH shapes.
///
/// A journal written before any map recorded one unkeyed head under
/// [`LEGACY_REALM_KEY`]; a mapped run records it under the realm's own
/// name. A reader may also arrive without the map that was in effect —
/// `brokkr resume` takes no map — so a single-entry record answers for
/// itself: with one realm recorded there is nothing to be ambiguous
/// about, and refusing to read it would lose the ship gate's drift check
/// for no gain.
pub fn recorded_head<'a>(recorded: &'a Value, realm: Option<&str>) -> Option<&'a str> {
    let heads = recorded.as_object()?;
    if let Some(head) = realm.and_then(|name| heads.get(name)) {
        return head.as_str();
    }
    if let Some(head) = heads.get(LEGACY_REALM_KEY) {
        return head.as_str();
    }
    match heads.len() {
        1 => heads.values().next()?.as_str(),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
