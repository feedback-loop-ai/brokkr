//! The world an invocation opens (decision 0023, phase 1): loading the
//! map, resolving its paths against the workspace, and pinning it.
//!
//! The shape and its refusals are `brokkr_core::realms`; this is the half
//! that touches a filesystem. A map NAMED at invocation and missing, or
//! present and malformed, is a refusal here — before a store is opened
//! and long before any seat spawns. There is no silent fallback: a world
//! that never drew a map notices nothing, and a world that drew a broken
//! one is told. A crossing (decision 0057) is judged here for the same
//! reason and at the same moment: `brokkr_core::realms` says what a
//! published and a consumed crossing may look like, and this half reads
//! the published file and holds the consumer's pin to its bytes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;
use brokkr_core::realms::{Boundary, Realm, RealmMap, RealmsError, DEFAULT_MAP_FILE};
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
    #[error(
        "realm '{realm}' publishes crossing '{crossing}' at {path}, \
         but it is not a readable file: {detail}"
    )]
    Crossing {
        realm: String,
        crossing: String,
        path: String,
        detail: String,
    },
    /// Boxed, and alone among these variants: six names are what it takes
    /// to say which contract moved, and a refusal nobody can act on is
    /// worth less than the word it saves. The box keeps that cost off
    /// every other `Result` in this module.
    #[error(
        "realm '{}' consumes crossing '{}' from realm '{}' pinned at {}, \
         but {} hashes to {}",
        .0.realm, .0.crossing, .0.publisher, .0.pinned, .0.path, .0.observed
    )]
    CrossingMoved(Box<MovedCrossing>),
}

/// A consumed crossing whose publisher's bytes are no longer the bytes it
/// was pinned against. Every name a reader needs to act is here — which
/// realm is refusing, which crossing, whose file, what was pinned and
/// what is there now — so the contract that moved can be identified
/// without opening either repository. `path` is the publisher's declared
/// repository-relative path, never the host location it resolved to: a
/// refusal reaches run journals and readouts that must not carry the
/// operator's filesystem layout.
#[derive(Debug)]
pub struct MovedCrossing {
    pub realm: String,
    pub crossing: String,
    pub publisher: String,
    pub path: String,
    pub pinned: String,
    pub observed: String,
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

/// A published crossing, resolved against the tree that publishes it
/// (decision 0057 rulings 1 and 2): where the bytes were found, and what
/// they hash to. Carried on the [`World`] beside the house and dialect
/// pins, so that whatever records a crossing later reads the digest that
/// was verified at load rather than hashing the file a second time — by
/// which point it could say something else.
#[derive(Debug)]
pub struct ResolvedCrossing {
    /// The path the bytes were read from, resolved against the publishing
    /// realm's own worktree.
    pub source: String,
    /// The repository-relative path the MAP declared, beside the resolved
    /// `source`. A refusal names the contract at this level — the
    /// publisher's realm and its own path — rather than the host location
    /// the bytes happened to be read from, so no readout or seat input
    /// carries the operator's filesystem layout (decision 0020 ruling 1).
    pub declared: String,
    /// sha256 over the file's RAW bytes, never a canonical form: a
    /// crossing may be a schema, a `.proto` or Markdown, and only the
    /// publisher's own format knows what canonicalising would mean.
    pub sha256: String,
}

/// Every crossing this world publishes, keyed by the realm that
/// publishes it and the name it publishes it under — the pair decision
/// 0057's consequences make unique, since a realm is named once and a
/// crossing name is one name inside one realm.
type Crossings = BTreeMap<(String, String), ResolvedCrossing>;

/// What is wrong with one crossing, held apart from the realm and the
/// name so the two [`WorldError`] variants a crossing can raise are
/// built from one place and worded once.
#[derive(Debug, Clone)]
enum CrossingFault {
    /// The publishing realm's own file could not be read.
    Unpublished { path: String, detail: String },
    /// The publisher's bytes are no longer the bytes a consumer pinned.
    Moved {
        publisher: String,
        path: String,
        pinned: String,
        observed: String,
    },
}

/// A crossing that is not what its realm's map says it is, carried as
/// DATA rather than returned as a `Result` — the same shape, and for the
/// same reason, as [`RealmTextFailure`] above.
///
/// One reading of the disk answers two questions with it. `World::load`
/// REFUSES on the first of these, so `run`, `rerun`, `compile` and
/// `resume` end before a seat spawns (decision 0046's Addendum, decision
/// 0021 ruling 2); `brokkr doctor` REPORTS them all and refuses nothing
/// (the same Addendum: a doctor line reports). Both read their wording
/// out of [`CrossingFailure::error`], so a moved crossing has exactly one
/// wording in this build.
#[derive(Debug, Clone)]
pub struct CrossingFailure {
    /// The realm this failure is named against: the PUBLISHER when its
    /// own file cannot be read, the CONSUMER when a pin no longer
    /// matches. Which is the whole point of the ordering in
    /// [`resolve_crossings`].
    realm: String,
    crossing: String,
    fault: CrossingFault,
}

impl CrossingFailure {
    pub fn realm(&self) -> &str {
        &self.realm
    }

    pub fn crossing(&self) -> &str {
        &self.crossing
    }

    /// The refusal a run would have been given for this crossing —
    /// `WorldError::Crossing` or `WorldError::CrossingMoved`, never a
    /// third wording composed at the reporting site.
    pub fn error(&self) -> WorldError {
        match &self.fault {
            CrossingFault::Unpublished { path, detail } => WorldError::Crossing {
                realm: self.realm.clone(),
                crossing: self.crossing.clone(),
                path: path.clone(),
                detail: detail.clone(),
            },
            CrossingFault::Moved {
                publisher,
                path,
                pinned,
                observed,
            } => WorldError::CrossingMoved(Box::new(MovedCrossing {
                realm: self.realm.clone(),
                crossing: self.crossing.clone(),
                publisher: publisher.clone(),
                path: path.clone(),
                pinned: pinned.clone(),
                observed: observed.clone(),
            })),
        }
    }

    fn is_unpublished(&self) -> bool {
        matches!(self.fault, CrossingFault::Unpublished { .. })
    }

    /// Whether this is a CONSUMER's pin that no longer matches, rather
    /// than a PUBLISHER's unreadable file. A readout that lists a realm's
    /// consumed crossings asks this rather than matching on the crossing's
    /// name alone: a realm may publish `x` and consume another realm's `x`
    /// (only consuming its OWN `x` is refused, ruling 3.6), so both faults
    /// can sit on one report under one name, and the name alone would say
    /// the wrong one moved.
    pub fn moved(&self) -> bool {
        matches!(self.fault, CrossingFault::Moved { .. })
    }

    /// The realm that published the crossing, for a MOVED fault: the
    /// second half of the pair a consumed entry is matched against, so
    /// two consumers of the same name from two publishers are not
    /// conflated. `None` for an unreadable publication, which is the
    /// publisher's own fault and is never a consumed pin's answer.
    pub fn publisher(&self) -> Option<&str> {
        match &self.fault {
            CrossingFault::Moved { publisher, .. } => Some(publisher),
            CrossingFault::Unpublished { .. } => None,
        }
    }
}

/// A consumed pin that was never compared to anything, because the realm
/// that publishes it could not have its file read. It is not a failure of
/// this realm — the publisher's own unreadable file is already a
/// [`CrossingFailure`] on the publisher's report, and it is the refusal
/// [`World::load`] gives — but it is not a pin that matched either, and a
/// readout that counted it among the matching ones would say a contract
/// was verified against bytes nobody read.
#[derive(Debug)]
pub struct UncheckedPin {
    /// The realm that publishes the crossing, whose line carries why.
    pub publisher: String,
    pub crossing: String,
}

/// Why the pin was never compared, worded ONCE for every surface that
/// reports one — `brokkr doctor`'s warn line and `brokkr realms`' pin
/// state — exactly as a moved pin is worded once by
/// [`CrossingFailure::error`]. Each surface frames it in its own words
/// ("pin not checked: …", "unchecked · …"); neither owns the reason.
impl std::fmt::Display for UncheckedPin {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "realm '{}' publishes it and its file could not be read",
            self.publisher
        )
    }
}

/// One realm's crossings, as `brokkr doctor` reports them: how many files
/// it publishes, how many pins it carries, whichever of those is not
/// currently true, and which of its pins could not be checked at all.
///
/// Built only for a realm that draws a crossing at all, so a world that
/// never drew one gets no line — exactly as it writes no manifest key and
/// prints nothing at compile.
#[derive(Debug)]
pub struct CrossingReport {
    pub realm: String,
    pub published: usize,
    pub consumed: usize,
    pub failures: Vec<CrossingFailure>,
    /// The consumed pins nothing could be compared to, so that
    /// `consumed` is never read as "this many pins matched".
    pub unchecked: Vec<UncheckedPin>,
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
    crossings: Crossings,
    /// Every crossing this world draws, per realm, whether or not it is
    /// currently true. Read as a refusal by [`World::load`] and as a
    /// readout by `brokkr doctor`; empty for a world that draws none and
    /// for a world replayed from a manifest, which resolves none.
    reports: Vec<CrossingReport>,
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
    /// Load one named map, refusing a missing or malformed file — and a
    /// crossing that is not what the map says it is.
    pub fn load(path: &Path) -> Result<World, WorldError> {
        let world = World::read(path)?;
        // Eagerly, and unlike the house and the dialect: those are
        // deferred per realm because only the realm actually running needs
        // its own, but a crossing is a fact about the WHOLE world's
        // integrity. A contract that moved is refused here, before a store
        // is opened and long before any seat spawns, whichever realm this
        // invocation happens to be standing in.
        match world.crossing_failure() {
            Some(failure) => Err(failure),
            None => Ok(world),
        }
    }

    /// Every step of [`World::load`] except its crossing refusal: the
    /// crossings are resolved and the pins compared exactly the same way,
    /// and whatever is not true is carried as data.
    fn read(path: &Path) -> Result<World, WorldError> {
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
        let (crossings, reports) = resolve_crossings(&source, &map);
        Ok(World {
            source,
            sha256: canonical::sha256_hex(&content),
            map,
            content,
            texts,
            crossings,
            reports,
        })
    }

    /// The map an invocation reads: the one it named, else `realms.json`
    /// beside the workspace when there is one, else no map at all.
    pub fn discover(dir: &Path, named: Option<&Path>) -> Result<Option<World>, WorldError> {
        World::found(dir, named, World::load)
    }

    /// The world the READ surfaces read — `brokkr doctor`, `brokkr realms`
    /// and `brokkr muninn run`, and nothing else: a crossing that has
    /// moved is a LINE, not the end of the readout.
    ///
    /// A doctor line reports and never refuses (decision 0046's Addendum),
    /// and folding a moved crossing into `World::discover`'s `Err` would
    /// throw away every house, dialect and boundary line under it — a
    /// broken world where one contract moved. The same argument reaches
    /// every surface that only looks: a readout that refuses to describe
    /// the world is at its least useful in exactly the world it was asked
    /// about, and it protects nothing, because it starts nothing. So the
    /// read surfaces load the world and read [`World::crossings_report`]
    /// beside it, while every verb that starts or continues a run keeps
    /// [`World::load`]'s refusal.
    ///
    /// Never used to pin a run: a world read this way may hold fewer
    /// resolved crossings than its map declares.
    pub fn inspect(dir: &Path, named: Option<&Path>) -> Result<Option<World>, WorldError> {
        World::found(dir, named, World::read)
    }

    fn found(
        dir: &Path,
        named: Option<&Path>,
        open: fn(&Path) -> Result<World, WorldError>,
    ) -> Result<Option<World>, WorldError> {
        match named {
            Some(path) => open(path).map(Some),
            None => {
                let default = dir.join(DEFAULT_MAP_FILE);
                match default.is_file() {
                    true => open(&default).map(Some),
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
            // A resumed world answers off its own manifest and never off
            // the disk, which may since have changed or gone. The manifest
            // DOES carry what the original run observed (`crossings`,
            // run-manifest/v10), and that pin is deliberately not read back
            // into here: it answers a READER's question about the past —
            // which contracts did that run stand on — not a live
            // accessor's question about the present. `World::crossing` on
            // a replayed world would otherwise report bytes nobody has
            // looked at since, so it reports none, and re-reading the
            // files the replayed run stood beside is exactly what this
            // path must not do.
            crossings: BTreeMap::new(),
            // And so it reports none either. What a resumed run owes the
            // present is a FENCE, not a readout, and that is
            // [`World::verify_crossings`], asked for by the verb.
            reports: Vec::new(),
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

    /// The boundary the operated repository's realm runs under (decision
    /// 0046 ruling 1): the realm's declared word, else `namespace` — and
    /// `namespace` too for a repository the map does not name, exactly as
    /// a run with no map at all. The one resolver the compiler, the verbs
    /// and the engine's entry fence all read, so they cannot disagree.
    pub fn boundary_for(&self, repo: &Path) -> Boundary {
        self.realm_for(repo)
            .map_or(Boundary::Namespace, Realm::boundary)
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

    /// One published crossing as this world resolved it: the path it was
    /// read from and the digest of the bytes that were there. Keyed by the
    /// realm that PUBLISHES it and the name it publishes it under, which
    /// is how a consuming realm names it too (`ConsumedCrossing`), so a
    /// consumer's resolved crossings are its own `consumed()` entries read
    /// through here.
    ///
    /// Present only for a world loaded off a disk. A world replayed from a
    /// manifest resolves none — the manifest's `crossings` pin is
    /// testimony about the run that wrote it, not a live reading — and
    /// this accessor never reaches for a file to make up for it.
    pub fn crossing(&self, realm: &str, name: &str) -> Option<&ResolvedCrossing> {
        self.crossings.get(&(realm.to_string(), name.to_string()))
    }

    /// Every crossing this world draws, one entry per realm that draws
    /// one, with whatever under it is not currently true — what `brokkr
    /// doctor` prints. Empty for a world that drew none, so doctor adds
    /// no line to a world that never heard the word.
    pub fn crossings_report(&self) -> &[CrossingReport] {
        &self.reports
    }

    /// The refusal [`World::load`] gives for this world's crossings, or
    /// `None` when every one of them is what its map says it is.
    fn crossing_failure(&self) -> Option<WorldError> {
        first_crossing_failure(&self.reports)
    }

    /// Re-read this world's crossings off the disk as it stands NOW, and
    /// refuse exactly as [`World::load`] would have.
    ///
    /// `brokkr resume` names a journal and no map: its world is rehydrated
    /// from the run manifest's pin ([`World::from_manifest`]), which
    /// resolves no crossings at all, deliberately, because that pin is
    /// testimony about the past. A resumed run is therefore the one whose
    /// world has never met the disk, and this is where it does — so that
    /// it cannot carry on over bytes its journal never saw.
    ///
    /// Paths resolve by the map's own rule, unchanged: a map's relative
    /// names are relative to the map FILE's own directory, and the pinned
    /// source is itself resolved against the workspace the operator is
    /// standing in. For the ordinary `./realms.json` that is that
    /// workspace — where a `--repo`-less verb already looks — and for a
    /// map pinned by absolute path it is that path's own directory.
    pub fn verify_crossings(&self, workspace: &Path) -> Result<(), WorldError> {
        let source = workspace.join(&self.source);
        let (_, reports) = resolve_crossings(&source, &self.map);
        match first_crossing_failure(&reports) {
            Some(failure) => Err(failure),
            None => Ok(()),
        }
    }

    /// The crossings this world STOOD ON, as they go into a run manifest
    /// (run-manifest/v10, on decision 0023 ruling 4's terms): per
    /// publishing realm, per crossing name, where the bytes were read and
    /// what they hashed to. Exactly the map [`resolve_crossings`] built at
    /// load, serialized verbatim — nothing is hashed a second time, by
    /// which point it could say something else.
    ///
    /// This is OBSERVATION and never a second copy of the DECLARATION: a
    /// consuming realm's pin already rides into the manifest inside the
    /// embedded map itself (`realms.map…consumes[].sha256`, unchanged
    /// since v4), and a reader who holds both sees what the world claimed
    /// beside what it stood on.
    ///
    /// `None` when this world publishes nothing, so a world that never
    /// drew a crossing writes the exact shape it always wrote. The key
    /// pair cannot collide: only a realm's own `publishes` list adds an
    /// entry, and a name used twice in one such list is already refused
    /// (decision 0057 ruling 3.3).
    fn crossings_pin(&self) -> Option<Value> {
        if self.crossings.is_empty() {
            return None;
        }
        let mut published = serde_json::Map::new();
        for ((realm, name), resolved) in &self.crossings {
            let publisher = published.entry(realm.clone()).or_insert_with(|| json!({}));
            publisher[name.as_str()] = json!({
                "source": resolved.source,
                "sha256": resolved.sha256,
            });
        }
        Some(Value::Object(published))
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

    /// A run manifest with this world pinned into it (run-manifest/v4,
    /// carried forward by v5, and the crossings beside it at v10).
    /// The bundle manifest is untouched — the map and the crossings are
    /// workspace data, not bundle data, so adopting either moves no
    /// bundle digest.
    pub fn pinned(&self, manifest: &Value, repo: Option<&Path>) -> Result<Value, WorldError> {
        let mut fields = manifest.as_object().cloned().unwrap_or_default();
        fields.insert("realms".to_string(), self.pin(repo)?);
        // A SIBLING of `realms`, never nested inside it: the map is what
        // this world declared and the crossings are what it observed, and
        // the contract keeps two answers to two questions apart. Omitted
        // entirely, never written empty, when there is nothing to say.
        if let Some(crossings) = self.crossings_pin() {
            fields.insert("crossings".to_string(), crossings);
        }
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

/// The directory a realm's own repository-relative names resolve
/// against: its `path`, itself resolved against the map file's directory,
/// so a house, a dialect and a crossing all travel with the workspace the
/// map describes.
fn realm_root(base: &Path, realm: &Realm) -> PathBuf {
    let path = Path::new(&realm.path);
    match path.is_relative() {
        true => base.join(path),
        false => path.to_path_buf(),
    }
}

fn load_realm_texts(map_source: &Path, map: &RealmMap) -> RealmTexts {
    let base = map_source.parent().unwrap_or(Path::new(""));
    let mut texts = BTreeMap::new();
    for realm in &map.realms {
        let realm_root = realm_root(base, realm);
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

/// Decision 0057's filesystem half: read every published crossing off the
/// tree that publishes it, then hold every pinned one to the bytes that
/// are actually there.
///
/// Publication is resolved for the whole world first, and only then are
/// the pins compared. That ordering is what a reader is told: a crossing
/// whose file is gone is the PUBLISHER's fault and is named as the
/// publisher's, never reported as the consumer having pinned the wrong
/// digest of a file that is not there at all. Its consumers' pins are then
/// neither matching nor moved, and come back as [`UncheckedPin`]s so that
/// a readout can say so rather than count them as verified.
///
/// Nothing here refuses. Every crossing is read and every pin compared,
/// and what is not true comes back as a [`CrossingReport`] per realm, so
/// that one reading serves both the verb that must end
/// ([`World::load`], through [`first_crossing_failure`]) and the doctor
/// line that must not.
///
/// Nothing is fetched and nothing is written. A crossing consumed from a
/// realm whose worktree this workspace does not hold is an ordinary
/// missing file: how bytes reach a consumer that is not co-located is
/// deliberately unsettled (0057's Context), and refusing is not choosing.
fn resolve_crossings(map_source: &Path, map: &RealmMap) -> (Crossings, Vec<CrossingReport>) {
    let base = map_source.parent().unwrap_or(Path::new(""));
    let mut crossings = Crossings::new();
    let mut reports: Vec<CrossingReport> = Vec::new();
    for realm in &map.realms {
        if realm.published().is_empty() && realm.consumed().is_empty() {
            continue;
        }
        let mut report = CrossingReport {
            realm: realm.name.clone(),
            published: realm.published().len(),
            consumed: realm.consumed().len(),
            failures: Vec::new(),
            unchecked: Vec::new(),
        };
        let root = realm_root(base, realm);
        for crossing in realm.published() {
            let path = root.join(&crossing.path);
            match std::fs::read(&path) {
                Ok(bytes) => {
                    crossings.insert(
                        (realm.name.clone(), crossing.name.clone()),
                        ResolvedCrossing {
                            source: path.display().to_string(),
                            declared: crossing.path.clone(),
                            sha256: canonical::sha256_bytes(&bytes),
                        },
                    );
                }
                Err(error) => report.failures.push(CrossingFailure {
                    realm: realm.name.clone(),
                    crossing: crossing.name.clone(),
                    fault: CrossingFault::Unpublished {
                        path: path.display().to_string(),
                        detail: error.to_string(),
                    },
                }),
            }
        }
        reports.push(report);
    }
    for realm in &map.realms {
        for crossing in realm.consumed() {
            let report = reports
                .iter_mut()
                .find(|report| report.realm == realm.name)
                .expect("a realm that consumes a crossing draws one");
            // `brokkr-core` admits a `consumes` entry only when this world
            // holds the realm it names AND that realm publishes that
            // crossing (0057 ruling 3.1 and 3.2), so the entry is one of
            // the publications resolved above — unless that publication's
            // own file could not be read, which is already the
            // PUBLISHER's line, and a pin has nothing to be compared to.
            // Nothing is charged to this realm for that, but the pin is
            // recorded as unchecked, so no reader is told it matched.
            let Some(published) = crossings.get(&(crossing.realm.clone(), crossing.name.clone()))
            else {
                report.unchecked.push(UncheckedPin {
                    publisher: crossing.realm.clone(),
                    crossing: crossing.name.clone(),
                });
                continue;
            };
            if published.sha256 != crossing.sha256 {
                report.failures.push(CrossingFailure {
                    realm: realm.name.clone(),
                    crossing: crossing.name.clone(),
                    fault: CrossingFault::Moved {
                        publisher: crossing.realm.clone(),
                        // The publisher's OWN declared path, never the
                        // resolved `source`: the refusal names the
                        // contract at the level the map declares it, and
                        // no run journal or seat input learns where the
                        // operator's checkout lives.
                        path: published.declared.clone(),
                        pinned: crossing.sha256.clone(),
                        observed: published.sha256.clone(),
                    },
                });
            }
        }
    }
    (crossings, reports)
}

/// The one refusal a world with several broken crossings gives, in the
/// order it has always given them: every publisher's own unreadable file
/// before any consumer's pin, so a crossing whose file is GONE is named
/// as the publisher's fault and never reported as the consumer having
/// pinned the wrong digest of a file that is not there at all.
fn first_crossing_failure(reports: &[CrossingReport]) -> Option<WorldError> {
    let failures = || reports.iter().flat_map(|report| &report.failures);
    failures()
        .find(|failure| failure.is_unpublished())
        .or_else(|| failures().next())
        .map(CrossingFailure::error)
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
    // A pin is read at the version it was written, not at the version
    // this build writes: the run pinned its world and a resume gets that
    // world back (decision 0042's dialect versions, `dialect::SCHEMAS`).
    let (mut dialect, _) = Dialect::parse_pinned(source, &text)
        .map_err(|error| WorldError::Unpinned(error.to_string()))?;
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

// `pub(crate)` so the two-repository fixture below can be REUSED by the
// engine's own tests rather than rebuilt there: a private `mod` is
// visible only to its own module and its descendants, so `pub(crate)`
// items inside it stay unreachable from `crate::engine::tests` until the
// module itself is widened. Test-only visibility, no production surface.
#[cfg(test)]
pub(crate) mod tests;
