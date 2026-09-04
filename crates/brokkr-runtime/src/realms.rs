//! The world an invocation opens (decision 0023, phase 1): loading the
//! map, resolving its paths against the workspace, and pinning it.
//!
//! The shape and its refusals are `brokkr_core::realms`; this is the half
//! that touches a filesystem. A map NAMED at invocation and missing, or
//! present and malformed, is a refusal here — before a store is opened
//! and long before any seat spawns. There is no silent fallback: a world
//! that never drew a map notices nothing, and a world that drew a broken
//! one is told.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;
use brokkr_core::realms::{Realm, RealmMap, RealmsError, DEFAULT_MAP_FILE};
use serde_json::{json, Value};
use thiserror::Error;

use crate::dialect::{library_path, Dialect};

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("no realms map at {0}")]
    Missing(String),
    #[error("reading realms map {path}: {source}")]
    Unreadable {
        path: String,
        source: std::io::Error,
    },
    #[error(transparent)]
    Map(#[from] RealmsError),
    #[error("this run's pinned realms map is unreadable: {0}")]
    Unpinned(String),
    #[error("realm '{realm}' names {kind} at {path}, but it is not a readable file: {detail}")]
    RealmText {
        realm: String,
        kind: &'static str,
        path: String,
        detail: String,
    },
    #[error("realm '{realm}' dialect is unusable: {detail}")]
    RealmDialect { realm: String, detail: String },
}

#[derive(Debug, Clone)]
struct TextPin {
    source: String,
    sha256: String,
    content: String,
}

#[derive(Debug, Clone)]
struct DialectPin {
    source: String,
    sha256: String,
    content: Value,
    dialect: Dialect,
}

#[derive(Debug, Clone)]
struct RealmTextFailure {
    realm: String,
    kind: &'static str,
    path: String,
    detail: String,
}

impl RealmTextFailure {
    fn error(&self) -> WorldError {
        WorldError::RealmText {
            realm: self.realm.clone(),
            kind: self.kind,
            path: self.path.clone(),
            detail: self.detail.clone(),
        }
    }
}

type RealmTexts = BTreeMap<
    String,
    (
        Result<Option<TextPin>, RealmTextFailure>,
        Result<Option<DialectPin>, String>,
    ),
>;

/// A loaded map, with everything a run needs to answer for it later: the
/// file it came from, the content verbatim, and the content's digest.
#[derive(Debug)]
pub struct World {
    /// The map file as it was named at invocation.
    pub source: PathBuf,
    pub map: RealmMap,
    /// The map's content as parsed — the bytes that are embedded and
    /// hashed. Canonical JSON, so re-indenting the file moves nothing.
    pub content: Value,
    pub sha256: String,
    texts: RealmTexts,
}

/// One hearth of a world (decision 0026 ruling 1): a journal, and the
/// realms whose runs live in it. A v1 map has exactly one — every realm
/// falls back to the world's journal — which is why every surface that
/// groups by hearth shows a v1 world exactly as it always did.
///
/// Journals never merge (ruling 5). A hearth is a place to READ from;
/// nothing here folds two of them together.
#[derive(Debug, PartialEq, Eq)]
pub struct Hearth {
    /// The realms sharing this journal, in map order. Never empty.
    pub realms: Vec<String>,
    pub journal: PathBuf,
}

impl Hearth {
    /// The hearth's name on a tab bar or a section header. Several
    /// realms sharing one journal share one heading, joined — a reader
    /// is told which realms a listing is of, not made to guess.
    pub fn label(&self) -> String {
        match self.realms.is_empty() {
            // Only reachable for a hearth built from a bare journal with
            // no map at all, which is never grouped or tabbed.
            true => "world".to_string(),
            false => self.realms.join("+"),
        }
    }
}

/// A path made comparable. An unresolvable path (a realm whose directory
/// does not exist yet) compares as written rather than failing the whole
/// lookup: the map is evidence about intent, not a mount check.
fn absolute(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

impl World {
    /// Load one named map, refusing a missing or malformed file.
    pub fn load(path: &Path) -> Result<World, WorldError> {
        let named = path.display().to_string();
        if !path.is_file() {
            return Err(WorldError::Missing(named));
        }
        let text = std::fs::read_to_string(path).map_err(|source| WorldError::Unreadable {
            path: named.clone(),
            source,
        })?;
        let (map, content) = RealmMap::parse(&named, &text)?;
        let source = path.to_path_buf();
        let texts = load_realm_texts(&source, &map);
        Ok(World {
            source,
            sha256: canonical::sha256_hex(&content),
            map,
            content,
            texts,
        })
    }

    /// The map an invocation reads: the one it named, else `realms.json`
    /// beside the workspace when there is one, else no map at all.
    pub fn discover(dir: &Path, named: Option<&Path>) -> Result<Option<World>, WorldError> {
        match named {
            Some(path) => World::load(path).map(Some),
            None => {
                let default = dir.join(DEFAULT_MAP_FILE);
                match default.is_file() {
                    true => World::load(&default).map(Some),
                    false => Ok(None),
                }
            }
        }
    }

    /// The world a run believed in, read back out of its own manifest —
    /// never off the disk, which may since have changed or gone. This is
    /// what ruling 4's embedding is FOR: `brokkr resume` names a journal
    /// and no map, and still keys its facts by realm, because the pin
    /// answers. A manifest with no pin is a run that had no world.
    ///
    /// The embedded content answers for itself: its digest is re-derived
    /// and must match the pin, and the map is re-validated by the same
    /// rules that admitted it. A pin that fails either is a refusal, not
    /// a quiet fall back to the unkeyed shape.
    pub fn from_manifest(manifest: &Value) -> Result<Option<World>, WorldError> {
        let Some(pin) = manifest.get("realms") else {
            return Ok(None);
        };
        let unpinned = |problem: &str| WorldError::Unpinned(problem.to_string());
        let source = pin
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| unpinned("it names no source"))?;
        let sha256 = pin
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or_else(|| unpinned("it carries no digest"))?;
        let content = pin
            .get("map")
            .cloned()
            .ok_or_else(|| unpinned("it embeds no map"))?;
        let derived = canonical::sha256_hex(&content);
        if derived != sha256 {
            return Err(WorldError::Unpinned(format!(
                "the embedded map hashes to {derived}, not the pinned {sha256}"
            )));
        }
        let (map, content) = RealmMap::of(source, content)?;
        let texts = pinned_texts(pin, &map)?;
        Ok(Some(World {
            source: PathBuf::from(source),
            map,
            content,
            sha256: sha256.to_string(),
            texts,
        }))
    }

    /// A map's relative paths are relative to the map file's own
    /// directory, so a world travels with the workspace it describes.
    fn resolve(&self, value: &str) -> PathBuf {
        let path = Path::new(value);
        match self.source.parent().filter(|_| path.is_relative()) {
            Some(base) => base.join(path),
            None => path.to_path_buf(),
        }
    }

    /// The journal this world writes. `--db` outranks it.
    pub fn journal(&self) -> PathBuf {
        self.resolve(&self.map.journal)
    }

    /// One realm's working tree.
    pub fn path_of(&self, realm: &Realm) -> PathBuf {
        self.resolve(&realm.path)
    }

    /// One realm's effective journal, resolved the way every other path
    /// in a map is: against the MAP FILE's own directory, never against
    /// the world journal's directory. A realm's hearth travels with the
    /// workspace the map describes, exactly as its working tree does.
    pub fn journal_of(&self, realm: &Realm) -> PathBuf {
        self.resolve(self.map.journal_of(realm))
    }

    /// The DISTINCT journals this world's realms carry, in map order —
    /// what a fleet reader opens (decision 0026 rulings 2 and 3).
    ///
    /// Realms sharing a journal share a hearth: most maps still name one
    /// journal for the whole world, and those must not pay for the
    /// many-hearth case by opening or listing it twice. A v1 map always
    /// yields exactly one hearth.
    pub fn hearths(&self) -> Vec<Hearth> {
        let mut hearths: Vec<Hearth> = Vec::new();
        for realm in &self.map.realms {
            let journal = self.journal_of(realm);
            let at = absolute(&journal);
            match hearths
                .iter_mut()
                .find(|hearth| absolute(&hearth.journal) == at)
            {
                Some(hearth) => hearth.realms.push(realm.name.clone()),
                None => hearths.push(Hearth {
                    realms: vec![realm.name.clone()],
                    journal,
                }),
            }
        }
        hearths
    }

    /// The realm a repository IS, when the world knows it. Facts about a
    /// repository the map does not name are recorded unkeyed, exactly as
    /// they were before any map existed — the engine never invents a
    /// realm name for a tree the operator did not map.
    pub fn realm_for(&self, repo: &Path) -> Option<&Realm> {
        let target = absolute(repo);
        self.map
            .realms
            .iter()
            .find(|realm| absolute(&self.path_of(realm)) == target)
    }

    /// The immutable house text selected for this repository's realm.
    pub fn house_for(&self, repo: &Path) -> Result<Option<&str>, WorldError> {
        let Some(realm) = self.realm_for(repo) else {
            return Ok(None);
        };
        self.house_for_realm(realm)
    }

    /// Check and read one declared realm's immutable house text. Doctor
    /// uses the realm directly so two declarations that currently point
    /// at the same absent checkout are still diagnosed independently.
    pub fn house_for_realm(&self, realm: &Realm) -> Result<Option<&str>, WorldError> {
        match self.texts.get(&realm.name).map(|(house, _)| house) {
            Some(Ok(house)) => Ok(house.as_ref().map(|pin| pin.content.as_str())),
            Some(Err(failure)) => Err(failure.error()),
            None => Ok(None),
        }
    }

    /// The checked dialect selected for a repository, resolved either from
    /// Brokkr's library or from the realm itself.
    pub fn dialect_for(&self, repo: &Path) -> Result<Option<&Dialect>, WorldError> {
        let Some(realm) = self.realm_for(repo) else {
            return Ok(None);
        };
        self.dialect_for_realm(realm)
    }

    pub fn dialect_for_realm(&self, realm: &Realm) -> Result<Option<&Dialect>, WorldError> {
        match self.texts.get(&realm.name).map(|(_, dialect)| dialect) {
            Some(Ok(dialect)) => Ok(dialect.as_ref().map(|pin| &pin.dialect)),
            Some(Err(detail)) => Err(WorldError::RealmDialect {
                realm: realm.name.clone(),
                detail: detail.clone(),
            }),
            None => Ok(None),
        }
    }

    /// The world as it goes into a run manifest: named, hashed, embedded.
    pub fn pin(&self, repo: Option<&Path>) -> Result<Value, WorldError> {
        let mut pin = json!({
            "source": self.source.display().to_string(),
            "sha256": self.sha256,
            "map": self.content,
        });
        if let Some(realm) = repo.and_then(|repo| self.realm_for(repo)) {
            pin["realm"] = json!(realm.name);
            if let Some((house, dialect)) = self.texts.get(&realm.name) {
                let house = house.as_ref().map_err(RealmTextFailure::error)?;
                if let Some(house) = house {
                    pin["house"] = json!({
                        "source": house.source,
                        "sha256": house.sha256,
                        "content": house.content
                    });
                }
                let dialect = dialect
                    .as_ref()
                    .map_err(|detail| WorldError::RealmDialect {
                        realm: realm.name.clone(),
                        detail: detail.clone(),
                    })?;
                if let Some(dialect) = dialect {
                    pin["dialect"] = json!({
                        "source": dialect.source,
                        "sha256": dialect.sha256,
                        "content": dialect.content
                    });
                    let instructions = json!(dialect.dialect.rendered);
                    pin["dialect"]["instructions_sha256"] =
                        json!(canonical::sha256_hex(&instructions));
                    pin["dialect"]["instructions"] = instructions;
                }
            }
        }
        Ok(pin)
    }

    /// A run manifest with this world pinned into it (run-manifest/v4, carried forward by v5).
    /// The bundle manifest is untouched — the map is workspace data, not
    /// bundle data, so adopting a map moves no bundle digest.
    pub fn pinned(&self, manifest: &Value, repo: Option<&Path>) -> Result<Value, WorldError> {
        let mut fields = manifest.as_object().cloned().unwrap_or_default();
        fields.insert("realms".to_string(), self.pin(repo)?);
        Ok(Value::Object(fields))
    }
}

fn read_text(
    realm: &Realm,
    kind: &'static str,
    path: PathBuf,
) -> Result<TextPin, RealmTextFailure> {
    let content = std::fs::read_to_string(&path).map_err(|error| RealmTextFailure {
        realm: realm.name.clone(),
        kind,
        path: path.display().to_string(),
        detail: error.to_string(),
    })?;
    Ok(TextPin {
        source: path.display().to_string(),
        sha256: canonical::sha256_bytes(content.as_bytes()),
        content,
    })
}

fn load_realm_texts(map_source: &Path, map: &RealmMap) -> RealmTexts {
    let base = map_source.parent().unwrap_or(Path::new(""));
    let mut texts = BTreeMap::new();
    for realm in &map.realms {
        let realm_root = {
            let path = Path::new(&realm.path);
            if path.is_relative() {
                base.join(path)
            } else {
                path.to_path_buf()
            }
        };
        let house = match &realm.house {
            Some(path) => read_text(realm, "house", realm_root.join(path)).map(Some),
            None => Ok(None),
        };
        let dialect = match &realm.dialect {
            Some(value) => {
                let path = library_path(base, value, &realm_root);
                Dialect::load(&path)
                    .map(|(dialect, content)| {
                        Some(DialectPin {
                            source: path.display().to_string(),
                            sha256: canonical::sha256_hex(&content),
                            content,
                            dialect,
                        })
                    })
                    .map_err(|error| error.to_string())
            }
            None => Ok(None),
        };
        texts.insert(realm.name.clone(), (house, dialect));
    }
    texts
}

fn pinned_text(pin: &Value, key: &str) -> Result<Option<TextPin>, WorldError> {
    let Some(value) = pin.get(key) else {
        return Ok(None);
    };
    let field = |name| {
        value
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| WorldError::Unpinned(format!("its {key} pin carries no {name}")))
    };
    let source = field("source")?.to_string();
    let sha256 = field("sha256")?.to_string();
    let content = field("content")?.to_string();
    let derived = canonical::sha256_bytes(content.as_bytes());
    if derived != sha256 {
        return Err(WorldError::Unpinned(format!(
            "the pinned {key} hashes to {derived}, not the pinned {sha256}"
        )));
    }
    Ok(Some(TextPin {
        source,
        sha256,
        content,
    }))
}

fn pinned_dialect(pin: &Value) -> Result<Option<DialectPin>, WorldError> {
    let Some(value) = pin.get("dialect") else {
        return Ok(None);
    };
    let source = value
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| WorldError::Unpinned("its dialect pin carries no source".into()))?;
    let sha256 = value
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| WorldError::Unpinned("its dialect pin carries no sha256".into()))?;
    let content = value
        .get("content")
        .cloned()
        .ok_or_else(|| WorldError::Unpinned("its dialect pin carries no content".into()))?;
    let derived = canonical::sha256_hex(&content);
    if derived != sha256 {
        return Err(WorldError::Unpinned(format!(
            "the pinned dialect hashes to {derived}, not the pinned {sha256}"
        )));
    }
    let text = serde_json::to_string(&content).expect("JSON serializes");
    let (mut dialect, _) =
        Dialect::parse(source, &text).map_err(|error| WorldError::Unpinned(error.to_string()))?;
    let instructions = value
        .get("instructions")
        .ok_or_else(|| WorldError::Unpinned("its dialect pin carries no instructions".into()))?;
    let expected = value
        .get("instructions_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| WorldError::Unpinned("its dialect instructions carry no sha256".into()))?;
    let actual = canonical::sha256_hex(instructions);
    if actual != expected {
        return Err(WorldError::Unpinned(format!(
            "the pinned dialect instructions hash to {actual}, not the pinned {expected}"
        )));
    }
    dialect.rendered = serde_json::from_value(instructions.clone()).map_err(|error| {
        WorldError::Unpinned(format!("its dialect instructions are malformed: {error}"))
    })?;
    Ok(Some(DialectPin {
        source: source.to_string(),
        sha256: sha256.to_string(),
        content,
        dialect,
    }))
}

fn pinned_texts(pin: &Value, map: &RealmMap) -> Result<RealmTexts, WorldError> {
    let house = pinned_text(pin, "house")?;
    let dialect = pinned_dialect(pin)?;
    let mut texts = BTreeMap::new();
    if let Some(realm) = pin
        .get("realm")
        .and_then(Value::as_str)
        .and_then(|name| map.realms.iter().find(|realm| realm.name == name))
    {
        if realm.house.is_some() && house.is_none() {
            return Err(WorldError::Unpinned(
                "its selected realm names a house but the manifest pins none".to_string(),
            ));
        }
        if realm.dialect.is_some() && dialect.is_none() {
            return Err(WorldError::Unpinned(
                "its selected realm names a dialect but the manifest pins none".to_string(),
            ));
        }
        texts.insert(realm.name.clone(), (Ok(house), Ok(dialect)));
    } else if map.realms.len() == 1 {
        texts.insert(map.realms[0].name.clone(), (Ok(house), Ok(dialect)));
    }
    Ok(texts)
}

#[cfg(test)]
mod tests;
