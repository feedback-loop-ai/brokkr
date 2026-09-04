//! The map of the world (decision 0023, phase 1): `forge.realms/v1`, and
//! its many-hearth amendment `forge.realms/v2` (decision 0026 ruling 1),
//! and the realm-owned house and dialect declaration in `forge.realms/v3`.
//!
//! A repository is a realm; the map that holds them is the connective
//! truth and is not itself a realm. This module is the PURE half — the
//! shape, the refusals, the content digest, and the per-realm fact
//! lookup. Reading a file, resolving a path against a workspace and
//! asking git anything all live in `brokkr-runtime::realms`, because this
//! crate performs no I/O (decision 0003, constitutional boundary 1).
//!
//! The v1 shape is minimal by ruling: the realms — each a name, a path
//! and a default branch — and the world's journal. Nothing else.
//! Decision 0021's per-realm driver and egress constraints are a later
//! amendment, deliberately not speculatively schema'd; unknown fields
//! are REFUSED here so that amendment must arrive as a version rather
//! than as drift in a file still calling itself v1.
//!
//! v2 adds exactly ONE optional field, and lands beside v1 rather than
//! inside it: a realm may name its own `journal`, and a realm that names
//! none falls back to the world's — which is what every v1 realm does,
//! which is why a v1 map keeps loading exactly as it always has. The
//! vocabulary stays closed at both levels in both versions, and the one
//! new word is refused in a map that still calls itself v1: a version is
//! a promise about what a file may say, and a loader that shrugged at
//! v2 vocabulary under a v1 label would have made the promise a hint.

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

/// The original shape. A map calling itself anything this build does not
/// read is refused by name, never read hopefully.
pub const SCHEMA_V1: &str = "forge.realms/v1";

/// Many hearths (decision 0026 ruling 1): v1 plus the optional per-realm
/// `journal`, and nothing else.
pub const SCHEMA_V2: &str = "forge.realms/v2";

/// The realm's prompt constitution and specification dialect. Both are
/// declarations here; only the house is acted on by this slice.
pub const SCHEMA_V3: &str = "forge.realms/v3";

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
    /// This realm's own hearth, when it has one — `forge.realms/v2`
    /// vocabulary, absent in every v1 map and refused in one. Absent
    /// means the world's journal, so the fallback is not a special case
    /// anywhere: see [`RealmMap::journal_of`].
    #[serde(default)]
    pub journal: Option<String>,
    /// Repository-relative Markdown rendered between charter and run context.
    #[serde(default)]
    pub house: Option<String>,
    /// A library dialect name or repository-relative dialect path. Resolution
    /// belongs to decision 0042's later slice; this version only records it.
    #[serde(default)]
    pub dialect: Option<String>,
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

/// House and dialect files are facts inside their realm, never an escape
/// hatch to another tree. Treat the contract spelling as portable: reject
/// Unix roots, Windows roots and parent components on every host.
#[inline(never)]
fn is_repository_relative(value: &str) -> bool {
    let bytes = value.as_bytes();
    !matches!(bytes.first(), Some(b'/' | b'\\'))
        && !matches!(bytes, [_, b':', b'/' | b'\\', ..])
        && !value.split(['/', '\\']).any(|component| component == "..")
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
        if map.schema != SCHEMA_V1 && map.schema != SCHEMA_V2 && map.schema != SCHEMA_V3 {
            return Err(invalid(format!(
                "it calls itself '{}'; this build reads {SCHEMA_V1}, {SCHEMA_V2} and {SCHEMA_V3}",
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
            // The one new word, held to the version that introduced it.
            // `deny_unknown_fields` cannot do this job any more — the
            // field is known to the reader now — so the refusal is
            // written out, and it names the version that would admit it
            // rather than merely saying no.
            match &realm.journal {
                Some(_) if map.schema == SCHEMA_V1 => {
                    return Err(invalid(format!(
                        "realm '{}' names its own journal, which is {SCHEMA_V2} vocabulary \
                         in a map calling itself {SCHEMA_V1}",
                        realm.name
                    )))
                }
                Some(journal) if journal.trim().is_empty() => {
                    return Err(invalid(format!(
                        "realm '{}' has an empty journal",
                        realm.name
                    )))
                }
                _ => {}
            }
            for (field, value) in [("house", &realm.house), ("dialect", &realm.dialect)] {
                match value {
                    Some(_) if map.schema != SCHEMA_V3 => {
                        return Err(invalid(format!(
                            "realm '{}' names its {field}, which is {SCHEMA_V3} vocabulary in a map calling itself {}",
                            realm.name, map.schema
                        )))
                    }
                    Some(value) if value.trim().is_empty() => {
                        return Err(invalid(format!(
                            "realm '{}' has an empty {field}", realm.name
                        )))
                    }
                    Some(value) if !is_repository_relative(value) => {
                        return Err(invalid(format!(
                            "realm '{}' has a non-repository-relative {field}",
                            realm.name
                        )))
                    }
                    _ => {}
                }
            }
        }
        Ok((map, content))
    }

    /// The journal one realm's runs live in: its own when it names one,
    /// else the world's. This is the whole of ruling 1's resolution, in
    /// one place, so that every fleet reader answers "which hearth?" the
    /// same way — and so that a v1 map, where no realm names a journal,
    /// resolves every realm to the single journal it always had.
    pub fn journal_of<'a>(&'a self, realm: &'a Realm) -> &'a str {
        realm.journal.as_deref().unwrap_or(&self.journal)
    }
}

/// The head recorded for one realm, in the TWO shapes the ruling names
/// and no third one.
///
/// A journal written before any map recorded one unkeyed head under
/// [`LEGACY_REALM_KEY`], and it is still read: the per-realm lookup falls
/// back to that key. A mapped run records the head under the realm's own
/// name, and it answers to that name alone — a reader that cannot name
/// the realm is told nothing rather than handed whichever head happened
/// to be recorded, because a head from another realm would be compared
/// against this realm's tree. No reader has to guess: `brokkr resume`
/// takes no map but rehydrates the world from the run's own manifest pin.
pub fn recorded_head<'a>(recorded: &'a Value, realm: Option<&str>) -> Option<&'a str> {
    let heads = recorded.as_object()?;
    realm
        .and_then(|name| heads.get(name))
        .or_else(|| heads.get(LEGACY_REALM_KEY))?
        .as_str()
}

#[cfg(test)]
mod tests;
