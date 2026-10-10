//! Composition (decision 0017): resolving a recipe's `extends` chain
//! into ONE flat bundle, before anything is parsed and long before
//! anything runs.
//!
//! This module is the only place in the engine that knows the word
//! `extends`. Its input is a leaf bundle directory; its output is a
//! [`Resolved`] — the flat `bundle.json` document, the flat policy
//! table, a NAME-level origin map, the ordered ancestor chain with
//! digests, and the layer directories. `Bundle::compile` consumes that
//! and parses exactly as it did before composition existed: there is no
//! inheritance at run time, no dynamic lookup, no surprise.
//!
//! Seat values are OPAQUE. The resolver decides only WHICH value wins
//! for a given name and never opens one — which is possible only
//! because the `override` and `remove` markers are resolver-owned
//! TOP-LEVEL keys BESIDE the values rather than keys inside them:
//! stripping an in-value marker before merging would itself be
//! rewriting an opaque value.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::CompileError;

mod forced_crew;

/// The deepest chain the resolver walks, leaf included. A chain that
/// long is a modelling mistake, not a composition.
const MAX_LAYERS: usize = 8;

/// Top-level `bundle.json` keys the resolver owns. They are consumed by
/// composition and never handed to the parser.
const RESOLVER_KEYS: [&str; 4] = ["extends", "override", "remove", "policy"];

/// Member kinds `override` may be keyed by.
const OVERRIDE_KINDS: [&str; 6] = ["seats", "cases", "limits", "rules", "table", "bundle"];

/// Member kinds `remove` may be keyed by.
const REMOVE_KINDS: [&str; 3] = ["seats", "rules", "phases"];

/// Policy-table members that are arrays of NAMES. They merge by union,
/// so a derived recipe re-declaring an inherited phase says nothing new
/// rather than colliding — that is what keeps a derived recipe small.
const TABLE_NAME_ARRAYS: [&str; 3] = ["phases", "terminal", "shippable_from"];

/// The reserved manifest namespace the composition chain rides in.
pub const COMPOSE_PREFIX: &str = "@compose/";

const NO_RULES: &[Value] = &[];

/// One ancestor of a composed recipe: its name, the directory it was
/// resolved from, and the digest of its own manifest — which
/// transitively covers every byte of that ancestor AND of its own
/// ancestors, so changing a base moves the digest of everything derived
/// from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ancestor {
    pub name: String,
    /// The library directory the base was extended by, when it differs
    /// from its declared name.
    pub reached_as: Option<String>,
    pub dir: PathBuf,
    pub digest: String,
    /// The exact file map hashed into `digest`, retained for checking a
    /// script directory at spawn without re-hashing unrelated realm files.
    pub files: Map<String, Value>,
}

/// The single flat bundle the rest of the engine sees.
pub struct Resolved {
    /// The leaf recipe's name — the composed strategy's identity.
    pub name: String,
    /// The flat `bundle.json` document. Seat values in it are
    /// byte-identical to the layer that supplied them.
    pub document: Value,
    /// The same seats, by name, for the parser.
    pub seats: Map<String, Value>,
    /// The flat policy table.
    pub table: Value,
    /// Seat name -> index into `roots`. NAME-level only: never derived
    /// from anything inside a seat value.
    pub seat_origin: BTreeMap<String, usize>,
    /// `<seat>:<case>` -> layer which wrote that case. Unlike ordinary
    /// opaque seats, marked case overrides may have mixed provenance.
    pub case_origin: BTreeMap<String, usize>,
    /// Ancestors, nearest first. Empty for a recipe that composed
    /// nothing, and empty until [`Resolved::seal`] has sealed them.
    pub chain: Vec<Ancestor>,
    /// Every layer's directory, leaf first.
    pub roots: Vec<PathBuf>,
    /// Every layer's declared name and the library directory it was reached
    /// as, leaf first.
    layers: Vec<(String, Option<String>)>,
    /// What each layer was composed from, leaf first: its declaring document
    /// and its own table, and then the charters its seats were bound to.
    /// Each ancestor's are compared with its walk when it is sealed; after
    /// that only the leaf's remain, for its own manifest.
    reads: Vec<Consumed>,
}

impl Resolved {
    /// The chain as one line, leaf first — appended ONCE to any compile
    /// error raised downstream of resolution, so a composed bundle's
    /// failures say what they were composed from without teaching every
    /// lint about layers. `None` when nothing was composed. Each name is
    /// the one its layer declares, which nothing bounds, so a plain name
    /// reads as written and any other is rendered bounded and safe
    /// (rebuild unit 5e-fix-b).
    pub fn chain_note(&self) -> Option<String> {
        if self.layers.len() == 1 {
            return None;
        }
        let shown = |(name, _): &(String, Option<String>)| match super::plain_label(name) {
            true => name.to_string(),
            false => super::bounded_site(name),
        };
        let names: Vec<String> = self.layers.iter().map(shown).collect();
        Some(format!("composed: {}", names.join(" -> ")))
    }

    /// Seal every ancestor's identity, deepest first (design D7; rebuild
    /// unit 16-fix-b, F3). Each ancestor's walk takes the digest of every
    /// file a bound read supplied from that read's buffer — its declaring
    /// document, its own table, and each charter in `charters` its seats
    /// named — and never reads those paths again; each input must still
    /// stand as it was read before the digest exists. An overridden
    /// ancestor's reads are compared too. `charters` carries every charter
    /// the compile bound, each with the layer that declared it.
    pub(crate) fn seal(&mut self, charters: Vec<CharterRead>) -> Result<(), CompileError> {
        for charter in charters {
            let index = self
                .roots
                .iter()
                .position(|root| *root == charter.dir)
                .expect("a charter is declared by a layer");
            self.reads[index].charters.push(charter);
        }
        let mut chain: Vec<Ancestor> = Vec::new();
        for index in (1..self.roots.len()).rev() {
            let (name, reached_as) = &self.layers[index];
            let read = &self.reads[index];
            // An ancestor's digest covers its own files and its own
            // ancestors — never the leaf's agent resolution or the adapter
            // declarations that authorised its gates, both of which belong
            // to the composed bundle rather than to any layer.
            // An ancestor boxes nothing at this level, so the boundary it is
            // handed writes no key and moves no digest; `namespace` is the
            // word a layer meant before decision 0046 named one.
            let manifest = super::manifest_for(
                &self.roots[index],
                name,
                &chain,
                None,
                None,
                &BTreeMap::new(),
                &Map::new(),
                brokkr_core::realms::Boundary::Namespace,
                // Nor does an ancestor hold or grant a capability (decision
                // 0065): authority belongs to the composed bundle compiled in
                // a realm, so a layer's digest carries none and does not move.
                None,
                &read.digests(),
            )?;
            let files = manifest["files"]
                .as_object()
                .expect("manifest files")
                .clone();
            read.check(&files)?;
            chain.insert(
                0,
                Ancestor {
                    name: name.clone(),
                    reached_as: reached_as.clone(),
                    dir: self.roots[index].clone(),
                    digest: brokkr_core::canonical::sha256_hex(&manifest),
                    files,
                },
            );
        }
        self.reads.truncate(1);
        self.chain = chain;
        Ok(())
    }

    /// The digests the leaf's own walk takes from its bound buffers.
    pub(crate) fn leaf_digests(&self) -> BTreeMap<String, super::Supplied> {
        self.reads[0].digests()
    }

    /// Refuse a leaf whose walk does not hold what it was composed from and
    /// the charters its own seats were bound to, as they were read.
    pub(crate) fn check_leaf(&self, files: &Map<String, Value>) -> Result<(), CompileError> {
        self.reads[0].check(files)
    }
}

fn invalid(message: String) -> CompileError {
    CompileError::Invalid(message)
}

/// One recipe source document in the chain.
struct Layer {
    name: String,
    /// The library directory this layer was reached by. It may differ
    /// from `name` — `brokkr recipes add --name` installs under a chosen
    /// directory — so provenance records both rather than trusting one.
    reached_as: Option<String>,
    dir: PathBuf,
    file: PathBuf,
    document: Map<String, Value>,
    /// Where the layer's walk must pin the bytes `document` was parsed from.
    read: Pinned,
}

/// `^[a-z0-9][a-z0-9-]*$`, checked BEFORE any path is built so `../x`,
/// `a/b`, `SDD` and `.` are refused as names and never become paths.
fn valid_recipe_name(name: &str) -> bool {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {}
        _ => return false,
    }
    characters.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Walk the chain leaf-first, each layer's own `extends` read from its
/// own `bundle.json`. A repeated directory is a cycle and names the
/// whole loop in order; a chain longer than [`MAX_LAYERS`] names the
/// chain so far.
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn read_layers(leaf: &Path) -> Result<Vec<Layer>, CompileError> {
    let mut layers: Vec<Layer> = Vec::new();
    // Canonical from the leaf down, so every recorded dir is comparable
    // on every platform: macOS resolves /var to /private/var, and a
    // chain that mixed the two would record two names for one place.
    let mut dir = leaf.canonicalize()?;
    // The name this layer was extended BY, for the layer after the leaf.
    let mut reached_as: Option<String> = None;
    loop {
        if let Some(at) = layers.iter().position(|layer| layer.dir == dir) {
            let mut loop_names: Vec<&str> = layers[at..].iter().map(|l| l.name.as_str()).collect();
            loop_names.push(&layers[at].name);
            return Err(invalid(format!(
                "composition cycle: {}; a recipe may not extend itself, \
                 directly or around its chain",
                loop_names.join(" -> ")
            )));
        }
        let file = dir.join("bundle.json");
        // Read STRICTLY from the layer's own bytes (decision 0065, design
        // D3): a seat's capability requests are written here, and an
        // ordinary JSON map keeps the last copy of a repeated key — so a
        // `"requires"` followed by a `"wants"` under one name, or a second
        // `capabilities` field, would weaken a requirement before any
        // validation saw it. Every layer is read this way, so a leaf that
        // replaces a seat cannot hide what its ancestor wrote twice.
        // The document is consumed like any active input (design D7): ONE
        // bound read, whose buffer is parsed here and whose digest the
        // layer's walk must pin before its identity is sealed.
        let bound = super::bound_input(&dir, "bundle.json").map_err(|fault| match fault {
            super::InputFault::Missing(error) => CompileError::Io(error),
            super::InputFault::Place(place) => invalid(format!(
                "{}: the layer's declaring document, {place}. What a run is composed from must \
                 be what its identity names, so it is refused (decision 0065 slice one, design \
                 D7)",
                file.display()
            )),
        })?;
        let (read, bytes) = Pinned::of(bound);
        let text = String::from_utf8(bytes).map_err(std::io::Error::other)?;
        let parsed = brokkr_core::canonical::parse_strict(&text)
            .map_err(|problem| invalid(format!("{}: {problem}", file.display())))?;
        let document: Map<String, Value> = serde_json::from_value(parsed)?;
        let name = document
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(format!("{} missing 'name'", file.display())))?
            .to_string();
        if let Some(prior) = layers.iter().find(|layer| layer.name == name) {
            return Err(invalid(format!(
                "{} declares name '{name}', which its ancestor {} already declares; \
                 a derived recipe states its own name so runs, manifests and \
                 dispatch pins never report a base's name under a derived digest",
                file.display(),
                prior.file.display()
            )));
        }
        let extends = document.get("extends").cloned();
        layers.push(Layer {
            name,
            reached_as: reached_as.clone(),
            dir: dir.clone(),
            file: file.clone(),
            document,
            read,
        });
        if layers.len() > MAX_LAYERS {
            return Err(invalid(format!(
                "composition chain is deeper than {MAX_LAYERS} layers: {}",
                layers
                    .iter()
                    .map(|l| l.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" -> ")
            )));
        }
        let Some(extends) = extends else {
            return Ok(layers);
        };
        let base = extends.as_str().ok_or_else(|| {
            invalid(format!(
                "{}: 'extends' must be the name of a recipe in the library",
                file.display()
            ))
        })?;
        if !valid_recipe_name(base) {
            return Err(invalid(format!(
                "{}: 'extends' value '{base}' is not a recipe name; names match \
                 ^[a-z0-9][a-z0-9-]*$ and are refused before any path is built",
                file.display()
            )));
        }
        // The library is the leaf directory's parent — which is what a
        // recipe library already is, and what `brokkr recipes add`
        // installs into before it compile-verifies the copy.
        // `dir` is already canonical, so its parent IS the library — and
        // asking for it directly avoids `..`, which Windows does NOT
        // resolve inside the `\\?\` verbatim paths canonicalize returns.
        // `pop` is total: a directory holding a bundle.json is never the
        // filesystem root, and if it somehow were, `library` stays put
        // and the "not a recipe in {library}" error below fires anyway.
        // An `ok_or_else` here would be an arm no test can reach.
        let mut library = dir.clone();
        library.pop();
        let candidate = library.join(base);
        if !candidate.is_dir() {
            return Err(invalid(format!(
                "{}: extends '{base}', which is not a recipe in {}",
                file.display(),
                library.display()
            )));
        }
        // A base must BE in the library, not merely be reachable from
        // it: `brokkr recipes add` already refuses symlinks, and a
        // composed base is bind-mounted read-only into every confined
        // seat, so a link pointing outside would widen that mount.
        let resolved = candidate.canonicalize()?;
        if resolved.parent() != Some(library.as_path()) {
            return Err(invalid(format!(
                "{}: extends '{base}', which resolves outside the library to {}; \
                 a base is composed from and mounted into confined seats, so it \
                 must be a real directory in {}",
                file.display(),
                resolved.display(),
                library.display()
            )));
        }
        reached_as = Some(base.to_string());
        dir = resolved;
    }
}

/// A layer's resolver-owned markers, validated on read: an unknown kind
/// or a non-string entry is refused here, naming the file and the key.
struct Markers {
    overrides: BTreeMap<String, Vec<String>>,
    removals: BTreeMap<String, Vec<String>>,
}

impl Markers {
    fn read(layer: &Layer) -> Result<Markers, CompileError> {
        Ok(Markers {
            overrides: marker_map(layer, "override", &OVERRIDE_KINDS)?,
            removals: marker_map(layer, "remove", &REMOVE_KINDS)?,
        })
    }

    fn overriding(&self, kind: &str) -> &[String] {
        self.overrides.get(kind).map_or(&[], Vec::as_slice)
    }

    fn removing(&self, kind: &str) -> &[String] {
        self.removals.get(kind).map_or(&[], Vec::as_slice)
    }

    fn overrides_name(&self, kind: &str, name: &str) -> bool {
        self.overriding(kind).iter().any(|listed| listed == name)
    }
}

fn marker_map(
    layer: &Layer,
    key: &str,
    kinds: &[&str],
) -> Result<BTreeMap<String, Vec<String>>, CompileError> {
    let mut out = BTreeMap::new();
    let Some(raw) = layer.document.get(key) else {
        return Ok(out);
    };
    let shape = || {
        invalid(format!(
            "{}: '{key}' must be an object of name arrays keyed by member kind ({})",
            layer.file.display(),
            kinds.join(", ")
        ))
    };
    for (kind, listed) in raw.as_object().ok_or_else(shape)? {
        if !kinds.contains(&kind.as_str()) {
            return Err(invalid(format!(
                "{}: '{key}.{kind}' is not a member kind; known: {}",
                layer.file.display(),
                kinds.join(", ")
            )));
        }
        let mut names = Vec::new();
        for item in listed.as_array().ok_or_else(shape)? {
            names.push(item.as_str().ok_or_else(shape)?.to_string());
        }
        out.insert(kind.clone(), names);
    }
    Ok(out)
}

/// What the fold has resolved so far, base-first.
#[derive(Default)]
struct Merged {
    document: Map<String, Value>,
    document_from: BTreeMap<String, usize>,
    seats: Map<String, Value>,
    seat_from: BTreeMap<String, usize>,
    case_from: BTreeMap<String, usize>,
    seats_declared: bool,
    table: Map<String, Value>,
    table_declared: bool,
    table_from: BTreeMap<String, PathBuf>,
    rule_from: BTreeMap<String, PathBuf>,
    /// Layer index -> the policy bytes that layer's table was parsed from.
    consumed: BTreeMap<usize, TableRead>,
}

fn rule_id(rule: &Value) -> Option<&str> {
    rule.get("id").and_then(Value::as_str)
}

fn rules_of(table: &Map<String, Value>) -> &[Value] {
    table
        .get("rules")
        .and_then(Value::as_array)
        .map_or(NO_RULES, Vec::as_slice)
}

fn redefined(file: &Path, kind: &str, key: &str, prior: &Path, marker: &str) -> CompileError {
    invalid(format!(
        "{}: redefines {kind} '{key}', which {} already defines; mark it \
         '{marker}: [\"{key}\"]' to replace it deliberately, or give it \
         another name to add one",
        file.display(),
        prior.display()
    ))
}

fn stale(layer: &Layer, marker: &str, name: &str, why: &str) -> CompileError {
    invalid(format!(
        "{}: '{marker}' names '{name}' but {why}; a marker that describes \
         nothing is a lie about the composition",
        layer.file.display()
    ))
}

/// Remove every rule carrying `id` from the resolved table.
fn drop_rule(table: &mut Map<String, Value>, id: &str) {
    let kept: Vec<Value> = rules_of(table)
        .iter()
        .filter(|rule| rule_id(rule) != Some(id))
        .cloned()
        .collect();
    table.insert("rules".into(), Value::Array(kept));
}

/// Remove a name from every table name array that lists it; `false`
/// when no array did — which makes the removal a stale marker.
fn drop_phase_name(table: &mut Map<String, Value>, name: &str) -> bool {
    let mut removed = false;
    for field in TABLE_NAME_ARRAYS {
        if let Some(list) = table.get_mut(field).and_then(Value::as_array_mut) {
            let before = list.len();
            list.retain(|item| item.as_str() != Some(name));
            removed |= list.len() != before;
        }
    }
    removed
}

/// Base order first, derived-only names appended. Re-declaring a name
/// the base already lists is a no-op: there is no value to collide
/// with, only a name.
fn union_names(base: Option<&Value>, own: &[Value]) -> Value {
    let mut out: Vec<Value> = base.and_then(Value::as_array).cloned().unwrap_or_default();
    for item in own {
        if !out.contains(item) {
            out.push(item.clone());
        }
    }
    Value::Array(out)
}

/// A layer's own policy table, the file it was read from — the file every
/// table-level refusal names — and the bytes it was parsed from.
type LayerTable = (Map<String, Value>, PathBuf, TableRead);

/// The layer's own policy table, read relative to THAT layer's
/// directory. A layer that declares no `policy` contributes no table.
fn own_table(layer: &Layer) -> Result<Option<LayerTable>, CompileError> {
    let Some(raw) = layer.document.get("policy") else {
        return Ok(None);
    };
    let relative = raw.as_str().ok_or_else(|| {
        invalid(format!(
            "{}: 'policy' must be a path to this layer's table",
            layer.file.display()
        ))
    })?;
    // Decision 0066 ruling 5, asked of EVERY layer before its table is
    // read or merged: a leaf that declares its own table cannot hide what
    // an ancestor read from under a name the file walk skips. Second
    // council H5: the table is resolved to the file that will be READ —
    // canonically, through every link — and operator ruling 3 (design D7)
    // binds that read to the contained, regular target by handle. Every
    // refusal names the declaring layer's file and the reference, bounded.
    let reference = super::bounded_reference(relative);
    let bound = super::bound_input(&layer.dir, relative).map_err(|fault| match fault {
        super::InputFault::Missing(error) => invalid(format!(
            "{}: 'policy' names {reference}, {}",
            layer.file.display(),
            super::missing_clause(&error)
        )),
        super::InputFault::Place(place) => invalid(format!(
            "{}: 'policy' names {reference}, {place}. A table there could change how a run is \
             ruled without moving the bundle's identity, so it is refused; move it to a path \
             the bundle pins, such as 'policy.json' (decision 0066 ruling 5)",
            layer.file.display()
        )),
    })?;
    // One read: the buffer this parses is the buffer that was hashed, and
    // its digest is what the layer's walk takes for the table's keys.
    let (pinned, bytes) = Pinned::of(bound);
    let table: Map<String, Value> = serde_json::from_slice(&bytes)?;
    let read = TableRead { reference, pinned };
    Ok(Some((table, layer.dir.join(relative), read)))
}

/// Where one consumed buffer must stand in its layer's file map: the
/// walk's key for the entry the reference names and for the target that
/// was read, as `held`'s binding names them, and the digest of the buffer
/// that was parsed. The walk takes that digest for both keys and never
/// reads either path; `held` is what the read observed, verified before the
/// layer is sealed.
struct Pinned {
    digest: String,
    held: super::Held,
}

/// How a layer's walk holds one consumed buffer.
enum Standing {
    /// Under both keys, with the buffer's digest, and the input still
    /// stands as it was read.
    Held,
    /// The input no longer stands as it was read: an entry on its way was
    /// replaced, removed or retargeted, a name it was bound by changed, or
    /// the held file's bytes changed. (A key the walk lists no entry for
    /// never reaches here: the walk refuses it, rebuild unit 16-fix-d.)
    Changed,
}

impl Pinned {
    /// Where `bound`'s buffer must stand, and the buffer, for its one parse.
    fn of(bound: super::BoundInput) -> (Pinned, Vec<u8>) {
        let pinned = Pinned {
            digest: brokkr_core::canonical::sha256_bytes(&bound.bytes),
            held: bound.held,
        };
        (pinned, bound.bytes)
    }

    fn standing(&self, files: &Map<String, Value>) -> Standing {
        let pinned: Vec<Option<&str>> = self
            .held
            .binding()
            .keys()
            .iter()
            .map(|key| files.get(*key).and_then(Value::as_str))
            .collect();
        // Not short-circuited: the held input is verified whatever the map
        // says, and either disagreement is a change.
        let digests = pinned.iter().all(|at| *at == Some(self.digest.as_str()));
        match digests & self.held.intact(&self.digest) {
            true => Standing::Held,
            false => Standing::Changed,
        }
    }
}

/// One charter a seat was bound to (design D7): the layer that declared it,
/// the seat and the reference, bounded for a refusal, and where its buffer
/// must stand in that layer's file map.
pub(crate) struct CharterRead {
    dir: PathBuf,
    site: String,
    reference: String,
    pinned: Pinned,
}

impl CharterRead {
    /// The charter `bound` read for the seat `site` from the layer at `dir`;
    /// its bytes are not otherwise needed at compile, so only their digest
    /// and what the read holds are kept.
    pub(crate) fn of(
        dir: &Path,
        site: String,
        reference: String,
        bound: super::BoundInput,
    ) -> CharterRead {
        CharterRead {
            dir: dir.to_path_buf(),
            site,
            reference,
            pinned: Pinned::of(bound).0,
        }
    }
}

/// A layer's table read: the authored reference, already bounded for a
/// refusal, and where its buffer must stand.
struct TableRead {
    reference: String,
    pinned: Pinned,
}

/// The bytes one layer was composed from — its declaring document, its own
/// table when it declares one, and the charters its seats were bound to —
/// as they must stand in that layer's file map (design D7). The walk takes
/// each one's digest from its buffer, and each input is verified to stand
/// as it was read before the identity is sealed rather than assumed to.
struct Consumed {
    file: PathBuf,
    document: Pinned,
    table: Option<TableRead>,
    charters: Vec<CharterRead>,
}

impl Consumed {
    /// The digest the layer's walk takes for each consumed key, from the
    /// buffer that was read (rebuild unit 16-fix-b, F3), with the file that
    /// read held (16-fix-c, F1). Where two reads name one key the first
    /// stands, and the other is judged against it.
    /// Each carries who consumed it, as that input's own refusal names it,
    /// so a key the walk lists no entry for is refused with its source,
    /// kind, site and reference (16-fix-d).
    fn digests(&self) -> BTreeMap<String, super::Supplied> {
        let file = self.file.display();
        let document = (
            &self.document,
            format!("{file}: the layer's declaring document"),
        );
        let tables = self.table.iter().map(|table| {
            let consumer = format!("{file}: 'policy' names {}", table.reference);
            (&table.pinned, consumer)
        });
        let charters = self.charters.iter().map(|charter| {
            let consumer = format!(
                "{file}: seat {} names role {}",
                charter.site, charter.reference
            );
            (&charter.pinned, consumer)
        });
        let mut digests = BTreeMap::new();
        for (pinned, consumer) in std::iter::once(document).chain(tables).chain(charters) {
            let binding = pinned.held.binding();
            for key in binding.keys() {
                digests
                    .entry(key.to_string())
                    .or_insert_with(|| binding.supplied(&pinned.digest, consumer.clone()));
            }
        }
        digests
    }

    /// Refuse a layer whose walk does not hold its document, its table or a
    /// charter as it was read: pinned under other bytes, or no longer
    /// standing as the read observed it. (Under no entry of the name it was
    /// read by, the walk has already refused it.) A link retargeted or
    /// replaced after the read moves the
    /// reference's own entry while the file that was read keeps the
    /// target's, so both are asked, and the whole resolution is observed
    /// again.
    fn check(&self, files: &Map<String, Value>) -> Result<(), CompileError> {
        let file = self.file.display();
        if !matches!(self.document.standing(files), Standing::Held) {
            return Err(invalid(format!(
                "{file}: the layer's declaring document changed between the read that composed \
                 the layer and the walk that pinned it. What a run is composed from must be what \
                 its identity names, so it is refused (decision 0065 slice one, design D7)"
            )));
        }
        if let Some(table) = &self.table {
            table.check(&file, files)?;
        }
        for charter in &self.charters {
            if !matches!(charter.pinned.standing(files), Standing::Held) {
                return Err(invalid(format!(
                    "{file}: seat {} names role {}, which the walk that pinned the layer does \
                     not hold as it was read: its entry was replaced, retargeted or removed, or \
                     its bytes changed, after the read that bound it. What a seat is told must \
                     be what its identity names, so it is refused (decision 0065 slice one, \
                     design D7)",
                    charter.site, charter.reference
                )));
            }
        }
        Ok(())
    }
}

impl TableRead {
    /// Refuse a table its layer's walk does not hold as it was read.
    fn check(
        &self,
        file: &impl std::fmt::Display,
        files: &Map<String, Value>,
    ) -> Result<(), CompileError> {
        let reference = &self.reference;
        match self.pinned.standing(files) {
            Standing::Held => Ok(()),
            Standing::Changed => Err(invalid(format!(
                "{file}: 'policy' names {reference}, whose entry, target or bytes changed between \
                 the read that parsed it and the walk that pinned it: an entry on its way was \
                 replaced, removed or retargeted, a name it was read by changed, or the bytes \
                 read changed. The table a run is ruled by must be the table its identity names, \
                 so it is refused (decision 0065 slice one, design D7)"
            ))),
        }
    }
}

/// Every key a layer's root may carry, and nothing else (decision 0004's
/// closed inputs; decision 0065 slice one, rebuild unit 5e-fix-b). Each is
/// read at the root by exactly one owner: `name` and `extends` by
/// [`read_layers`], `override` and `remove` by [`Markers::read`], `policy`
/// by [`own_table`], `seats` by [`merge_layer`], `forced_crew` by
/// [`forced_crew::admit`], and `description`, `cost`, `protected_phase` and
/// `egress_minimum` by `Bundle::assemble` and `parse_egress_minimum` in
/// `bundle.rs`. It is the table of `docs/guides/recipe-authoring.md`
/// ("`bundle.json` anatomy"), in its order. Any other key at a root, a
/// capability word included, was once retained and ignored, compiling and
/// confining nothing, so the root is closed rather than a list of refused words.
const ROOT_KEYS: [&str; 11] = [
    "name",
    "description",
    "cost",
    "forced_crew",
    "policy",
    "protected_phase",
    "egress_minimum",
    "seats",
    "extends",
    "override",
    "remove",
];

/// Refuse any key outside [`ROOT_KEYS`] at the root of one layer, and a
/// `forced_crew` that states no reason. The reason names the layer and the
/// key, both rendered bounded and safe, and never the value.
fn admit_root(layer: &Layer) -> Result<(), CompileError> {
    let Some(key) = layer
        .document
        .keys()
        .find(|key| !ROOT_KEYS.contains(&key.as_str()))
    else {
        return forced_crew::admit(&layer.name, &layer.document);
    };
    let (rest, last) = ROOT_KEYS.split_at(ROOT_KEYS.len() - 1);
    Err(invalid(format!(
        "recipe {} declares {} at the bundle root, which admits only {} and {}. A \
         capability or confinement is written on each seat it governs (a sandbox as \
         'tools.sandbox'), and a boundary is the realm's, declared in realms.json. A key the \
         compiler does not read would compile, deliver nothing and leave every seat at its \
         harness default, so it is refused rather than ignored (decision 0004; decision 0065 \
         slice one, rebuild unit 5e-fix-b)",
        super::bounded_site(&layer.name),
        super::bounded_site(key),
        rest.join(", "),
        last[0]
    )))
}

/// Merge one layer over everything resolved beneath it.
#[expect(
    clippy::excessive_nesting,
    clippy::too_many_lines,
    reason = "baseline 2026-09, #288"
)]
fn merge_layer(merged: &mut Merged, layers: &[Layer], index: usize) -> Result<(), CompileError> {
    let layer = &layers[index];
    admit_root(layer)?;
    let markers = Markers::read(layer)?;
    let table = own_table(layer)?;

    // Removals are checked against the base and applied before this
    // layer's own contributions, so removing a name and then declaring
    // it again is a plain addition rather than a collision.
    for name in markers.removing("seats") {
        if merged.seat_from.remove(name).is_none() {
            return Err(stale(layer, "remove.seats", name, "no ancestor defines it"));
        }
        merged.seats.remove(name);
    }
    for id in markers.removing("rules") {
        if merged.rule_from.remove(id).is_none() {
            return Err(stale(
                layer,
                "remove.rules",
                id,
                "no ancestor's table has it",
            ));
        }
        drop_rule(&mut merged.table, id);
    }
    for name in markers.removing("phases") {
        if !drop_phase_name(&mut merged.table, name) {
            return Err(stale(
                layer,
                "remove.phases",
                name,
                "no ancestor's table names it",
            ));
        }
    }

    let own_seats = layer.document.get("seats");
    for named in markers.overriding("cases") {
        let Some((seat, case)) = named.split_once(':') else {
            return Err(invalid(format!(
                "{}: 'override.cases' entry '{named}' must be '<seat>:<case>'",
                layer.file.display()
            )));
        };
        let inherited = merged
            .seats
            .get(seat)
            .and_then(|value| value.pointer("/select/cases"))
            .and_then(Value::as_object);
        if !inherited.is_some_and(|cases| cases.contains_key(case)) {
            return Err(stale(
                layer,
                "override.cases",
                named,
                "no ancestor defines it",
            ));
        }
        let own = own_seats
            .and_then(Value::as_object)
            .and_then(|seats| seats.get(seat))
            .and_then(|value| value.pointer("/select/cases"))
            .and_then(Value::as_object);
        if !own.is_some_and(|cases| cases.contains_key(case)) {
            return Err(stale(
                layer,
                "override.cases",
                named,
                "this recipe does not redefine it",
            ));
        }
    }
    for seat in markers.overriding("limits") {
        if !merged
            .seats
            .get(seat)
            .is_some_and(|value| value.get("limits").is_some())
        {
            return Err(stale(
                layer,
                "override.limits",
                seat,
                "no ancestor defines it",
            ));
        }
        if !own_seats
            .and_then(Value::as_object)
            .and_then(|seats| seats.get(seat))
            .is_some_and(|value| value.get("limits").is_some())
        {
            return Err(stale(
                layer,
                "override.limits",
                seat,
                "this recipe does not redefine it",
            ));
        }
    }
    for name in markers.overriding("seats") {
        if !merged.seats.contains_key(name) {
            return Err(stale(
                layer,
                "override.seats",
                name,
                "no ancestor defines it",
            ));
        }
        if !own_seats
            .and_then(Value::as_object)
            .is_some_and(|seats| seats.contains_key(name))
        {
            return Err(stale(
                layer,
                "override.seats",
                name,
                "this recipe does not redefine it",
            ));
        }
    }
    for key in markers.overriding("bundle") {
        if !merged.document.contains_key(key) {
            return Err(stale(layer, "override.bundle", key, "no ancestor sets it"));
        }
        if !layer.document.contains_key(key) {
            return Err(stale(
                layer,
                "override.bundle",
                key,
                "this recipe does not set it",
            ));
        }
    }
    for field in markers.overriding("table") {
        if !merged.table.contains_key(field) {
            return Err(stale(
                layer,
                "override.table",
                field,
                "no ancestor's table sets it",
            ));
        }
        if !table
            .as_ref()
            .is_some_and(|(own, _, _)| own.contains_key(field))
        {
            return Err(stale(
                layer,
                "override.table",
                field,
                "this recipe's table does not set it",
            ));
        }
    }
    for id in markers.overriding("rules") {
        if !merged.rule_from.contains_key(id) {
            return Err(stale(
                layer,
                "override.rules",
                id,
                "no ancestor's table has it",
            ));
        }
        if !table
            .as_ref()
            .is_some_and(|(own, _, _)| rules_of(own).iter().any(|r| rule_id(r) == Some(id)))
        {
            return Err(stale(
                layer,
                "override.rules",
                id,
                "this recipe's table does not have it",
            ));
        }
    }

    // Top-level bundle.json members other than the seats and the
    // resolver's own keys: `name` is always the layer's own (a derived
    // recipe never inherits an identity), everything else needs the
    // marker to replace an inherited value.
    for (key, value) in &layer.document {
        if RESOLVER_KEYS.contains(&key.as_str()) || key == "seats" {
            continue;
        }
        if key != "name"
            && merged.document.contains_key(key)
            && !markers.overrides_name("bundle", key)
        {
            return Err(redefined(
                &layer.file,
                "bundle member",
                key,
                &layers[merged.document_from[key]].file,
                "override.bundle",
            ));
        }
        merged.document.insert(key.clone(), value.clone());
        merged.document_from.insert(key.clone(), index);
    }

    if let Some(raw) = own_seats {
        let seats = raw.as_object().ok_or_else(|| {
            invalid(format!(
                "{}: 'seats' must be an object keyed by phase",
                layer.file.display()
            ))
        })?;
        merged.seats_declared = true;
        for (name, value) in seats {
            let case_overrides: Vec<&str> = markers
                .overriding("cases")
                .iter()
                .filter_map(|entry| entry.split_once(':'))
                .filter_map(|(seat, case)| (seat == name).then_some(case))
                .collect();
            let limits_override = markers.overrides_name("limits", name);
            if merged.seats.contains_key(name)
                && !markers.overrides_name("seats", name)
                && case_overrides.is_empty()
                && !limits_override
            {
                return Err(redefined(
                    &layer.file,
                    "seat",
                    name,
                    &layers[merged.seat_from[name]].file,
                    "override.seats",
                ));
            }
            if (!case_overrides.is_empty() || limits_override)
                && !markers.overrides_name("seats", name)
            {
                let own = value
                    .as_object()
                    .expect("validated partial override has an object seat");
                for key in own.keys() {
                    let covered = (key == "select" && !case_overrides.is_empty())
                        || (key == "limits" && limits_override);
                    if !covered {
                        return Err(invalid(format!(
                            "{}: seat '{name}' member '{key}' is not covered by its partial override; mark override.seats to replace the whole seat",
                            layer.file.display()
                        )));
                    }
                }
                if !case_overrides.is_empty() {
                    let own_select = own["select"]
                        .as_object()
                        .expect("validated case override has a select object");
                    if own_select.keys().any(|key| key != "cases") {
                        return Err(invalid(format!(
                            "{}: seat '{name}' partial case override may contain only select.cases; mark override.seats to replace the whole seat",
                            layer.file.display()
                        )));
                    }
                    let own_cases = own_select["cases"]
                        .as_object()
                        .expect("validated case override has own cases");
                    if let Some(case) = own_cases
                        .keys()
                        .find(|case| !case_overrides.contains(&case.as_str()))
                    {
                        return Err(invalid(format!(
                            "{}: seat '{name}' case '{case}' is not named by override.cases; mark it or replace the whole seat",
                            layer.file.display()
                        )));
                    }
                }
                let mut inherited = merged.seats[name].clone();
                if !case_overrides.is_empty() {
                    let inherited_cases = inherited
                        .pointer_mut("/select/cases")
                        .and_then(Value::as_object_mut)
                        .expect("validated case override has inherited cases");
                    let own_cases = value
                        .pointer("/select/cases")
                        .and_then(Value::as_object)
                        .expect("validated case override has own cases");
                    for case in case_overrides {
                        inherited_cases.insert(case.to_string(), own_cases[case].clone());
                        merged.case_from.insert(format!("{name}:{case}"), index);
                    }
                }
                if limits_override {
                    inherited["limits"] = value["limits"].clone();
                }
                merged.seats.insert(name.clone(), inherited);
            } else {
                // Ordinary seat values remain opaque. Only the explicitly
                // marked named cases above are opened by the resolver.
                merged.seats.insert(name.clone(), value.clone());
                if let Some(cases) = value.pointer("/select/cases").and_then(Value::as_object) {
                    for case in cases.keys() {
                        merged.case_from.insert(format!("{name}:{case}"), index);
                    }
                }
                merged.seat_from.insert(name.clone(), index);
            }
        }
    }

    let Some((own, own_path, consumed)) = table else {
        return Ok(());
    };
    merged.consumed.insert(index, consumed);
    merged.table_declared = true;
    for (key, value) in &own {
        if key == "rules" {
            continue;
        }
        if key == "schema" {
            if let Some(base) = merged.table.get("schema") {
                if base != value {
                    return Err(invalid(format!(
                        "{} declares policy schema {base} but {} declares {value}; \
                         the layers of one composition share one table schema",
                        merged.table_from["schema"].display(),
                        own_path.display()
                    )));
                }
            }
        } else if TABLE_NAME_ARRAYS.contains(&key.as_str()) && !markers.overrides_name("table", key)
        {
            let list = value.as_array().ok_or_else(|| {
                invalid(format!(
                    "{}: table '{key}' must be an array of names",
                    own_path.display()
                ))
            })?;
            let united = union_names(merged.table.get(key), list);
            merged.table.insert(key.clone(), united);
            merged.table_from.insert(key.clone(), own_path.clone());
            continue;
        } else if merged.table.contains_key(key) && !markers.overrides_name("table", key) {
            return Err(redefined(
                &own_path,
                "table member",
                key,
                &merged.table_from[key],
                "override.table",
            ));
        }
        merged.table.insert(key.clone(), value.clone());
        merged.table_from.insert(key.clone(), own_path.clone());
    }

    let own_rules: Vec<Value> = match own.get("rules") {
        None => Vec::new(),
        Some(raw) => raw
            .as_array()
            .ok_or_else(|| invalid(format!("{}: 'rules' must be an array", own_path.display())))?
            .clone(),
    };
    let mut own_ids: Vec<String> = Vec::new();
    for rule in &own_rules {
        let id = rule_id(rule).ok_or_else(|| {
            invalid(format!(
                "{}: every policy rule needs a string 'id'",
                own_path.display()
            ))
        })?;
        if let Some(prior) = merged.rule_from.get(id) {
            if !markers.overrides_name("rules", id) {
                return Err(redefined(
                    &own_path,
                    "policy rule",
                    id,
                    prior,
                    "override.rules",
                ));
            }
        }
        own_ids.push(id.to_string());
    }
    // Derived rules PRECEDE base rules — the engine's existing
    // first-match-wins order. Overriding by id is remove-then-prepend,
    // not naive prepending: a base twin left behind would be unreachable
    // and `Machine::from_table` would reject the whole table, making the
    // feature's headline use case structurally impossible.
    let mut rules: Vec<Value> = own_rules.clone();
    for rule in rules_of(&merged.table) {
        if !own_ids.iter().any(|id| rule_id(rule) == Some(id.as_str())) {
            rules.push(rule.clone());
        }
    }
    for id in own_ids {
        merged.rule_from.insert(id, own_path.clone());
    }
    merged.table.insert("rules".into(), Value::Array(rules));
    Ok(())
}

/// Resolve a leaf recipe directory into one flat bundle. A PURE
/// function over the recipe sources: named files read in a
/// name-determined order, `BTreeMap` throughout, no clock, no
/// environment, no `read_dir` order. Every ancestor is sealed from what it
/// was composed from; a compile, which binds charters too, seals them
/// itself ([`resolve_unsealed`]).
pub fn resolve(leaf: &Path) -> Result<Resolved, CompileError> {
    let mut resolved = resolve_unsealed(leaf)?;
    resolved.seal(Vec::new())?;
    Ok(resolved)
}

/// [`resolve`] up to the ancestors' identities, which [`Resolved::seal`]
/// seals once the compile has bound every charter a layer's seats name, so
/// that no walk reads a consumed file the compile read (design D7).
pub(crate) fn resolve_unsealed(leaf: &Path) -> Result<Resolved, CompileError> {
    let layers = read_layers(leaf)?;
    let mut merged = Merged::default();
    for index in (0..layers.len()).rev() {
        merge_layer(&mut merged, &layers, index)?;
    }
    if !merged.table_declared {
        return Err(invalid("bundle.json missing 'policy'".into()));
    }
    if !merged.seats_declared {
        return Err(invalid("bundle.json missing 'seats'".into()));
    }

    // Every layer's document and table are kept for its seal, an overridden
    // ancestor's too: its bytes were read and parsed whatever the leaf later
    // replaced. Ancestor digests are sealed deepest first by
    // `Resolved::seal`: each covers its own bytes AND its own ancestors'
    // digests, so a change at any depth moves every digest derived from it.
    let mut tables = merged.consumed;
    let name = layers[0].name.clone();
    let (mut roots, mut names, mut reads) = (Vec::new(), Vec::new(), Vec::new());
    for (index, layer) in layers.into_iter().enumerate() {
        reads.push(Consumed {
            file: layer.file,
            document: layer.read,
            table: tables.remove(&index),
            charters: Vec::new(),
        });
        names.push((layer.name, layer.reached_as));
        roots.push(layer.dir);
    }
    let mut document = merged.document;
    document.insert("seats".into(), Value::Object(merged.seats.clone()));
    Ok(Resolved {
        name,
        document: Value::Object(document),
        seats: merged.seats,
        table: Value::Object(merged.table),
        seat_origin: merged.seat_from,
        case_origin: merged.case_from,
        chain: Vec::new(),
        roots,
        layers: names,
        reads,
    })
}
