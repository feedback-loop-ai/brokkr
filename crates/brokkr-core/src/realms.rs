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
//!
//! v4 (decision 0046 ruling 1) adds one more optional field on the same
//! terms: a realm may name the `boundary` its boxed hands stand behind,
//! one of a closed vocabulary that lives here as [`Boundary`] so every
//! crate that pins, records or renders the word reads the same five. A
//! realm that names none stands under `namespace`, which is what every
//! bundle meant before the word existed.
//!
//! v5 (decision 0057) adds the crossing, on exactly those terms again:
//! a realm may say what it `publishes` — a named, repository-relative
//! FILE it owns — and what it `consumes` — a crossing another realm
//! publishes, pinned by a sha256 over that file's raw bytes. Both are
//! optional, both are refused under an older label, and a v5 map that
//! names neither reads exactly as a v4 map does: a world that never
//! drew a crossing notices nothing. This module holds the whole of the
//! shape and its refusals; reading a published file, or checking a pin
//! against the bytes on disk, is a later slice's work in
//! `brokkr-runtime` — this crate performs no I/O.
//!
//! v6 (decision 0065 ruling 3) adds the grant, on those terms once more:
//! a realm may list the `capabilities` it grants, each naming the tool
//! dialect that serves it and optionally a subset of that dialect's
//! tools, the offices the grant reaches, and the dialect's own
//! restriction keys. It is refused under every older label even written
//! empty, and a realm that names none grants nothing — which is what
//! every realm under v1 through v5 does (ruling 4: no grandfathering).
//! This module judges the grant's SHAPE only; finding the dialect file,
//! and everything the grant means for a seat, is `brokkr-runtime`'s work.
//!
//! v7 (proposed decision 0075 ruling 5) adds one WORLD-level list, the
//! `provisional_offices`, judged in `realms/provisional.rs`.
//!
//! v8 (decision 0065 slice two, SC2) reserves one more grant key, `retain`,
//! whose only legal value is `false`: the retention veto, judged with the
//! rest of the grant in `realms/grants.rs`. Under v6 and v7 the same
//! spelling stays the dialect's restriction.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};
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

/// The boundary a realm's boxed hands stand behind (decision 0046 ruling
/// 1): v3 plus one optional per-realm `boundary`, and nothing else.
pub const SCHEMA_V4: &str = "forge.realms/v4";

/// The crossing (decision 0057): v4 plus two optional per-realm lists,
/// `publishes` and `consumes`, and nothing else.
pub const SCHEMA_V5: &str = "forge.realms/v5";

/// The grant (decision 0065 ruling 3): v5 plus one optional per-realm
/// `capabilities` map, and nothing else. Every older label keeps loading
/// and grants nothing — there is no grandfathering (ruling 4).
pub const SCHEMA_V6: &str = "forge.realms/v6";

/// The provisional tier (proposed decision 0075 ruling 5): v6 plus one
/// optional world-level `provisional_offices`, and nothing else.
pub const SCHEMA_V7: &str = "forge.realms/v7";

/// The retention veto (decision 0065 slice two, SC2): v7 plus one reserved
/// grant key, `retain`, and nothing else.
pub const SCHEMA_V8: &str = "forge.realms/v8";

/// Every label this build reads, oldest first — the one list a refusal
/// spells out and the version gates are written against.
pub const SCHEMAS: [&str; 8] = [
    SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4, SCHEMA_V5, SCHEMA_V6, SCHEMA_V7, SCHEMA_V8,
];

/// What stands between a box's hands and the machine (decision 0046
/// ruling 1). A closed vocabulary that names the MECHANISM and never the
/// operating system, frozen the way a contract is: a sixth word is a new
/// decision, not a new variant.
///
/// Deliberately no `Default`. Absence becomes `namespace` in exactly one
/// place — [`Realm::boundary`], the realm's own resolver — so a reader of
/// evidence (a journal, a manifest, a record) holds an `Option<Boundary>`
/// and cannot print `namespace` where nothing was recorded (decision 0031
/// ruling 3's pattern; design DD1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Boundary {
    /// Decision 0043's empty-root user namespace built by bubblewrap.
    Namespace,
    /// The same box built by the system sandbox (`sandbox-exec`).
    Seatbelt,
    /// A pinned image with the worktree mounted — decision 0008's
    /// `confine`, re-homed.
    Container,
    /// Nothing of Brokkr's; the harness's own sandbox as its adapter
    /// fragment addresses it.
    Harness,
    /// Nothing at all.
    Open,
}

/// The five words, in ruling order, for every refusal that spells the
/// closed set out.
pub const BOUNDARIES: [Boundary; 5] = [
    Boundary::Namespace,
    Boundary::Seatbelt,
    Boundary::Container,
    Boundary::Harness,
    Boundary::Open,
];

/// The word a record writes where a site has no hands at all: decision
/// 0031 ruling 1's sentinel, reused rather than reinvented. It is never a
/// boundary — the realms enum cannot admit it — which is why the record
/// carries an `Option<Boundary>` and this module spells the `None`.
pub const NOT_APPLICABLE: &str = "not applicable";

/// Serde's record vocabulary for `Option<Boundary>`. A missing field can
/// remain absent; a present sentinel means the site declared no hands.
pub mod recorded_boundary {
    use super::{Boundary, BoundaryError};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        boundary: &Option<Boundary>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(Boundary::recorded(*boundary))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Boundary>, D::Error> {
        let word = String::deserialize(deserializer)?;
        Boundary::from_recorded(&word).ok_or_else(|| serde::de::Error::custom(BoundaryError(word)))
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error(
    "'{0}' is not a boundary; the vocabulary is namespace, seatbelt, container, \
     harness and open, and a new boundary is a new decision (decision 0046 ruling 1)"
)]
pub struct BoundaryError(pub String);

impl Boundary {
    /// The word the operator writes and the record carries.
    pub fn word(self) -> &'static str {
        match self {
            Boundary::Namespace => "namespace",
            Boundary::Seatbelt => "seatbelt",
            Boundary::Container => "container",
            Boundary::Harness => "harness",
            Boundary::Open => "open",
        }
    }

    /// The word a seat record or an `effect/started` entry carries for a
    /// site: the boundary's own, or the sentinel for a site without
    /// hands. The one spelling of the sentinel (design DD1).
    pub fn recorded(site: Option<Boundary>) -> &'static str {
        match site {
            Some(boundary) => boundary.word(),
            None => NOT_APPLICABLE,
        }
    }

    /// Read a recorded word back: `Some(Some(word))` for a boundary,
    /// `Some(None)` for the sentinel, `None` for a word outside the six —
    /// which every reader renders as *not recorded*, never as a boundary
    /// (design DD14).
    pub fn from_recorded(word: &str) -> Option<Option<Boundary>> {
        if word == NOT_APPLICABLE {
            return Some(None);
        }
        word.parse().ok().map(Some)
    }

    /// Does Brokkr itself build this boundary's box? Under these three
    /// the workspace tool is served and the seat is told its hands are
    /// boxed; under `harness` and `open` nothing of Brokkr's stands.
    pub fn is_boxed(self) -> bool {
        matches!(
            self,
            Boundary::Namespace | Boundary::Seatbelt | Boundary::Container
        )
    }
}

impl fmt::Display for Boundary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

impl FromStr for Boundary {
    type Err = BoundaryError;

    fn from_str(word: &str) -> Result<Boundary, BoundaryError> {
        BOUNDARIES
            .into_iter()
            .find(|boundary| boundary.word() == word)
            .ok_or_else(|| BoundaryError(word.to_string()))
    }
}

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
    Invalid { path: String, problem: Unusable },
}

/// A crossing this realm publishes (decision 0057 ruling 1): a name, and
/// the repository-relative FILE the realm owns and offers. The bytes are
/// the contract — it is not a package and not a registry entry, and
/// nothing here fetches anything.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedCrossing {
    /// The crossing's name, in the realm-name grammar, unique within the
    /// realm that publishes it.
    pub name: String,
    /// Repository-relative path to the published file, named on exactly
    /// the terms `house` and `dialect` are named.
    pub path: String,
}

/// A crossing this realm consumes (decision 0057 ruling 2): the name, the
/// realm that publishes it, and the digest this realm is built against.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumedCrossing {
    /// The crossing's name, as the publishing realm publishes it.
    pub name: String,
    /// The realm that publishes it — never the consuming realm itself.
    pub realm: String,
    /// Lowercase hex sha256 over the published file's RAW bytes, never
    /// over a canonical form: a crossing may be a schema, a `.proto` or
    /// Markdown, and only the publisher's own format knows what
    /// canonicalising would mean. Judged here as a SHAPE only; comparing
    /// it against bytes on disk is a later slice's work, in the crate
    /// that may read a file.
    pub sha256: String,
}

/// A crossing list as the map WROTE it: never named, named as `null`, or
/// named as an array.
///
/// Serde reads a missing property and an explicit `null` alike into
/// `None`, which would make "this realm never said the word" and "this
/// realm said the word and named nothing" the same fact. They are not
/// the same fact. The first is every realm that ever loaded; the second
/// is a map saying something no version of the contract admits — v1
/// through v4 have no such property at all, and `realms.v5` types both
/// lists `array`. Held apart here, so the version gate sees a written
/// `null` as written (decision 0057 ruling 3.5) and the map cannot be
/// accepted where its own contract file would refuse it (ruling 3.7).
#[derive(Debug, Default, PartialEq, Eq)]
pub enum CrossingList<T> {
    /// The property was not written: a realm that drew no crossing,
    /// which is what every realm did before the word existed.
    #[default]
    Absent,
    /// The property was written as `null`, which is neither a list nor an
    /// absence. Carried this far only so the refusal can name it.
    Null,
    /// The property was written as an array, read back exactly as given.
    List(Vec<T>),
}

impl<T> CrossingList<T> {
    /// Whether the map wrote the word at all. A written `null` counts,
    /// which is the whole point: the version gate judges presence.
    pub fn is_written(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// Whether the map wrote the word as `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// The crossings written, or none at all — the one place absence is
    /// read, so every reader spells "no crossings" the same way.
    pub fn entries(&self) -> &[T] {
        match self {
            Self::List(entries) => entries,
            Self::Absent | Self::Null => &[],
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for CrossingList<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Reached only when the property IS written; `serde(default)`
        // answers for the absent case, and is the only way to reach
        // `Absent` at all.
        Ok(match Option::<Vec<T>>::deserialize(deserializer)? {
            Some(entries) => Self::List(entries),
            None => Self::Null,
        })
    }
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
    /// The boundary this realm's boxed hands stand behind — `forge.realms/v4`
    /// vocabulary (decision 0046 ruling 1), absent in every older map and
    /// refused in one. Absent means `namespace`, which is what every
    /// bundle meant until the word existed: see [`Realm::boundary`], the
    /// one place that absence is resolved.
    #[serde(default)]
    pub boundary: Option<Boundary>,
    /// The crossings this realm publishes — `forge.realms/v5` vocabulary
    /// (decision 0057 ruling 1), absent in every older map and refused in
    /// one. Absent means this realm publishes nothing, which is what
    /// every realm did before the word existed.
    #[serde(default)]
    pub publishes: CrossingList<PublishedCrossing>,
    /// The crossings this realm consumes, each pinned by digest —
    /// `forge.realms/v5` vocabulary (decision 0057 ruling 2), absent in
    /// every older map and refused in one.
    #[serde(default)]
    pub consumes: CrossingList<ConsumedCrossing>,
    /// The capabilities map exactly as the realm WROTE it —
    /// `forge.realms/v6` vocabulary (decision 0065 ruling 3), absent in
    /// every older map and refused in one, even written empty. `None` is
    /// a realm that never said the word; a written `null` arrives as
    /// `Some(Value::Null)` so the version gate and the shape refusal both
    /// see it. Read through [`Realm::grants`], never from here.
    #[serde(default, deserialize_with = "written")]
    pub capabilities: Option<Value>,
    /// The grants that map declares, judged by [`RealmMap::of`]. Empty for
    /// a realm that names none, which is every realm under v1 through v5:
    /// everything is off until the realm lists it (ruling 4).
    #[serde(skip)]
    pub grants: std::collections::BTreeMap<String, CapabilityGrant>,
}

/// Keep a written property apart from an absent one, `null` included:
/// serde reads both into `None` unless the property's own value is kept.
fn written<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

impl Realm {
    /// The boundary this realm runs under: the word it declares, else
    /// `namespace`. This is the whole of decision 0046 ruling 1's
    /// resolution, in one place, so no reader of evidence has to guess
    /// what an absent word meant.
    pub fn boundary(&self) -> Boundary {
        self.boundary.unwrap_or(Boundary::Namespace)
    }

    /// The crossings this realm publishes: its own list, or none at all.
    /// The one place that absence is read, so every reader spells "this
    /// realm publishes nothing" the same way.
    pub fn published(&self) -> &[PublishedCrossing] {
        self.publishes.entries()
    }

    /// The crossings this realm consumes: its own list, or none at all.
    pub fn consumed(&self) -> &[ConsumedCrossing] {
        self.consumes.entries()
    }
}

/// The map as written: realms, and the journal the world writes.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RealmMap {
    pub schema: String,
    pub realms: Vec<Realm>,
    pub journal: String,
    /// The offices, agents by name, a provisional model may hold: v7
    /// vocabulary, judged in `realms/provisional.rs`. Absent lists none.
    #[serde(default, deserialize_with = "provisional::null_as_none")]
    pub provisional_offices: Vec<String>,
}

/// A realm name is a journal key: lowercase, digits, and the three
/// separators a repository name already uses. Refused early, because a
/// name that cannot be read back out of evidence is not a name. A
/// capability and a tool dialect are named in the same grammar (decision
/// 0065), which is why the runtime reads it from here.
pub fn is_name(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c))
}

/// Is this label older than the version that introduced a word? A
/// version is a promise about what a file may say, so a word is refused
/// under every label that predates it and legal under its own and every
/// later one. One function, so the fifth version's gate reads exactly
/// like the second's.
fn older_than(schema: &str, introduced: &str) -> bool {
    SCHEMAS
        .iter()
        .take_while(|label| **label != introduced)
        .any(|label| *label == schema)
}

/// House and dialect files are facts inside their realm, never an escape
/// hatch to another tree. Treat the contract spelling as portable: reject
/// Unix roots, Windows roots and parent components on every host.
#[inline(never)]
fn is_repository_relative(value: &str) -> bool {
    let bytes = value.as_bytes();
    !matches!(bytes.first(), Some(b'/' | b'\\'))
        && !(bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
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
        // A v6 map carries grants, and a grant written twice would be
        // granted as whichever copy came second (decision 0065). The rule
        // arrives WITH the version, and v7 and v8 keep it: an older map
        // keeps the reading it has always had.
        if matches!(
            content.get("schema").and_then(Value::as_str),
            Some(SCHEMA_V6 | SCHEMA_V7 | SCHEMA_V8)
        ) {
            crate::canonical::parse_strict(text).map_err(|detail| RealmsError::Malformed {
                path: path.to_string(),
                detail,
            })?;
        }
        RealmMap::of(path, content)
    }

    /// Validate one map already parsed into a value — the shape a reader
    /// holds when the map arrives embedded in a run's manifest pin rather
    /// than as a file. Same refusals, same words: a world read back out
    /// of evidence is held to what it was held to going in.
    #[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
    pub fn of(path: &str, content: Value) -> Result<(RealmMap, Value), RealmsError> {
        let invalid = |problem: Unusable| RealmsError::Invalid {
            path: path.to_string(),
            problem,
        };
        // The boundary word is judged before the shape is read, so a
        // refusal can name the realm that wrote it and the five words
        // it may write (decision 0046 ruling 1) rather than serde's
        // anonymous "unknown variant". Only a string is judged here; any
        // other shape is the malformed-map refusal below, as for every
        // other field.
        for realm in content
            .get("realms")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(word) = realm.get("boundary").and_then(Value::as_str) {
                if let Err(error) = word.parse::<Boundary>() {
                    return Err(invalid(Unusable::Boundary {
                        realm: realm
                            .get("name")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        error,
                    }));
                }
            }
        }
        let mut map: RealmMap =
            serde_json::from_value(content.clone()).map_err(|error| RealmsError::Malformed {
                path: path.to_string(),
                detail: error.to_string(),
            })?;
        if !SCHEMAS.contains(&map.schema.as_str()) {
            return Err(invalid(Unusable::UnknownSchema(map.schema)));
        }
        // The v6 word, held to its version BEFORE its shape is read, and
        // held even when written empty (decision 0065 ruling 3): presence
        // is what a version gate judges, so `capabilities: {}` under an
        // older label is refused rather than shrugged at. Under v6 the
        // grants are judged here, once, and every reader takes them from
        // [`Realm::grants`].
        let schema = map.schema.clone();
        let unversioned = |realm: &Realm, word: Word| {
            invalid(Unusable::Unversioned {
                realm: realm.name.clone(),
                word,
                schema: schema.clone(),
            })
        };
        for realm in &mut map.realms {
            let Some(capabilities) = &realm.capabilities else {
                continue;
            };
            if older_than(&schema, Word::Capabilities.introduced()) {
                return Err(unversioned(realm, Word::Capabilities));
            }
            realm.grants = grants::parse_grants(&realm.name, &schema, capabilities)
                .map_err(|error| invalid(error.into()))?;
        }
        if map.realms.is_empty() {
            return Err(invalid(Unusable::NoRealms));
        }
        if map.journal.trim().is_empty() {
            return Err(invalid(Unusable::EmptyJournal));
        }
        for (index, realm) in map.realms.iter().enumerate() {
            let name = realm.name.clone();
            if !is_name(&realm.name) {
                return Err(invalid(Unusable::Name { index, name }));
            }
            if realm.path.trim().is_empty() {
                return Err(invalid(Unusable::NoPath(name)));
            }
            if realm.default_branch.trim().is_empty() {
                return Err(invalid(Unusable::NoBranch(name)));
            }
            if map.realms[..index].iter().any(|e| e.name == realm.name) {
                return Err(invalid(Unusable::NamedTwice(name)));
            }
            // The one new word, held to the version that introduced it.
            // `deny_unknown_fields` cannot do this job any more — the
            // field is known to the reader now — so the refusal is
            // written out, and it names the version that would admit it
            // rather than merely saying no.
            match &realm.journal {
                Some(_) if older_than(&map.schema, Word::Journal.introduced()) => {
                    return Err(unversioned(realm, Word::Journal))
                }
                Some(journal) if journal.trim().is_empty() => {
                    return Err(invalid(Unusable::Empty {
                        realm: name,
                        word: Word::Journal,
                    }))
                }
                _ => {}
            }
            // The v4 word, held to its version exactly as the v2 and v3
            // words are held to theirs (decision 0046 ruling 1).
            if realm.boundary.is_some() && older_than(&map.schema, Word::Boundary.introduced()) {
                return Err(unversioned(realm, Word::Boundary));
            }
            for (word, value) in [(Word::House, &realm.house), (Word::Dialect, &realm.dialect)] {
                match value {
                    Some(_) if older_than(&map.schema, word.introduced()) => {
                        return Err(unversioned(realm, word))
                    }
                    Some(value) if value.trim().is_empty() => {
                        return Err(invalid(Unusable::Empty { realm: name, word }))
                    }
                    Some(value) if !is_repository_relative(value) => {
                        return Err(invalid(Unusable::Outside { realm: name, word }))
                    }
                    _ => {}
                }
            }
            // The two v5 lists, held to their version exactly as every
            // word before them is held to its own (decision 0057 ruling
            // 3). Presence is what is judged: a realm that draws no
            // crossing is a realm as it was before the word existed —
            // and a realm that WROTE the word as `null` wrote it, which
            // is why [`CrossingList`] keeps the two apart. A written
            // `null` is refused on its own terms after the version gate
            // has had its say, because under an older label the deeper
            // fault is the word itself.
            for (word, written, null) in [
                (
                    Word::Publishes,
                    realm.publishes.is_written(),
                    realm.publishes.is_null(),
                ),
                (
                    Word::Consumes,
                    realm.consumes.is_written(),
                    realm.consumes.is_null(),
                ),
            ] {
                if written && older_than(&map.schema, word.introduced()) {
                    return Err(unversioned(realm, word));
                }
                if null {
                    return Err(invalid(Unusable::NullList { realm: name, word }));
                }
            }
            // What a realm publishes is judged on its own terms: a name
            // that can be read back out of evidence, a file inside the
            // realm that owns it, and no name used twice.
            let published = realm.published();
            for (index, entry) in published.iter().enumerate() {
                let (realm, crossing) = (name.clone(), entry.name.clone());
                if !is_name(&crossing) {
                    return Err(invalid(Unusable::CrossingName { realm, crossing }));
                }
                if entry.path.trim().is_empty() {
                    return Err(invalid(Unusable::NoCrossingPath { realm, crossing }));
                }
                if !is_repository_relative(&entry.path) {
                    return Err(invalid(Unusable::CrossingOutside { realm, crossing }));
                }
                if published[..index].iter().any(|e| e.name == crossing) {
                    return Err(invalid(Unusable::PublishedTwice { realm, crossing }));
                }
            }
            // What a realm consumes is judged on its own terms here —
            // the pin's shape, the crossing being between realms, and
            // one name meaning one file — and against the rest of the
            // world below, once every realm's own shape is known.
            let consumed = realm.consumed();
            for (index, entry) in consumed.iter().enumerate() {
                let (realm, crossing) = (name.clone(), entry.name.clone());
                if !crate::canonical::is_sha256_hex(&entry.sha256) {
                    let pin = entry.sha256.clone();
                    return Err(invalid(Unusable::Pin {
                        realm,
                        crossing,
                        pin,
                    }));
                }
                if entry.realm == realm {
                    return Err(invalid(Unusable::ConsumesItself { realm, crossing }));
                }
                if consumed[..index].iter().any(|e| e.name == crossing) {
                    return Err(invalid(Unusable::ConsumedTwice { realm, crossing }));
                }
            }
        }
        // Every crossing consumed is a crossing published, by a realm
        // this world holds. Judged after the loop above, so a name is
        // resolved only against realms whose own shape already stands.
        for realm in &map.realms {
            for crossing in realm.consumed() {
                let (realm, publisher, crossing) = (
                    realm.name.clone(),
                    crossing.realm.clone(),
                    crossing.name.clone(),
                );
                let Some(publishing) = map.realms.iter().find(|e| e.name == publisher) else {
                    return Err(invalid(Unusable::NoPublisher {
                        realm,
                        crossing,
                        publisher,
                    }));
                };
                if !publishing.published().iter().any(|e| e.name == crossing) {
                    return Err(invalid(Unusable::Unpublished {
                        realm,
                        crossing,
                        publisher,
                    }));
                }
            }
        }
        provisional::judge(&map, &content)
            .map(|()| (map, content))
            .map_err(invalid)
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

mod grants;
mod provisional;
mod unusable;

pub use grants::{CapabilityGrant, GrantError, GrantList, GrantRetention, GRANT_KEYS};
pub use unusable::{Unusable, Word};

#[cfg(test)]
mod tests;
