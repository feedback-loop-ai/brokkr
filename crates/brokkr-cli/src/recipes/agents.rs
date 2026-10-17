//! The agents a recipe names, installed with it (#696 ruling 5): `recipes
//! add` copies each agent definition a copied recipe's sites name, and the
//! charter that definition names, from the source's agent library into the
//! workspace's, so the compile still reads one agents directory. An
//! agent's own `extends` base is not copied: it resolves in the workspace's
//! library, where `init` wrote `implementer`, `intake` and `reviewer`.
//! Each capability a copied definition requests is copied too (#703), its
//! abstract definition from the directory the source's compile reads them
//! from into the one the workspace's reads them from ([`definitions_of`]).
//! Every file is planned, and every conflict refused, before anything is
//! written; the copies are written from the bytes that were compared.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use brokkr_core::realms::is_name;
use brokkr_runtime::agents::{valid_name, NAME_GRAMMAR};
use brokkr_runtime::bundle::DEFAULT_AGENTS_DIR;
use brokkr_runtime::capabilities::DEFINITIONS_DIR;
use brokkr_runtime::places::Layout;
use brokkr_runtime::realms::World;
use serde::de::IgnoredAny;
use serde::Deserialize;

use super::plain;

/// One recipe layer's sites, as far as they name agents, read as typed
/// data from its own `bundle.json`. Every other key is the compile's.
#[derive(Deserialize)]
struct Layer {
    #[serde(default)]
    seats: BTreeMap<String, Site>,
}

/// One site by the forms that hold an agent name: an `agent` reference,
/// a `panel` of members, a `sequence` of steps, and a `select`'s cases and
/// default.
#[derive(Deserialize)]
struct Site {
    agent: Option<String>,
    #[serde(default)]
    panel: BTreeMap<String, Site>,
    #[serde(default)]
    sequence: Vec<Site>,
    select: Option<Select>,
}

#[derive(Deserialize)]
struct Select {
    #[serde(default)]
    cases: BTreeMap<String, Site>,
    default: Option<Box<Site>>,
}

impl Site {
    /// The agent names this site and every site inside it hold.
    fn names<'a>(&'a self, into: &mut BTreeSet<&'a str>) {
        into.extend(self.agent.as_deref());
        let select = self.select.iter();
        let chosen =
            select.flat_map(|select| select.cases.values().chain(select.default.as_deref()));
        for inner in self.panel.values().chain(&self.sequence).chain(chosen) {
            inner.names(into);
        }
    }
}

/// An agent definition, as far as it names a charter and the capabilities
/// it requests; how strongly it requests each is the compile's to read.
#[derive(Deserialize)]
struct Definition {
    charter: Option<String>,
    #[serde(default)]
    capabilities: BTreeMap<String, IgnoredAny>,
}

/// One file an install copies into the workspace's agent library: what it
/// is, in the line `add` prints, where it goes, and the bytes read from
/// the source's library, which were compared and are what is written.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Hire {
    pub(super) what: String,
    pub(super) to: PathBuf,
    bytes: Vec<u8>,
}

/// An `add` refused before anything was written, because an agent the
/// recipe names could not be planned (#696). A caller matches the variant,
/// never the text (decision 0071 ruling 8).
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Hiring {
    /// A file `add` must read cannot be read or parsed: `cause` says why.
    Unread { path: PathBuf, cause: String },
    /// The recipe at `path` names an agent outside the name grammar.
    Unnamed { path: PathBuf, name: String },
    /// The definition at `path` names a charter that is not a relative
    /// path of plain names inside its library.
    Charter { path: PathBuf, charter: String },
    /// The definition at `path` names a charter its `library`, the
    /// source's agent library, does not hold.
    Uncharted {
        path: PathBuf,
        charter: String,
        library: PathBuf,
    },
    /// The definition at `path` requests a `capability` whose abstract
    /// definition `definitions`, the source's, does not hold (#703).
    Undefined {
        path: PathBuf,
        capability: String,
        definitions: PathBuf,
    },
    /// An entry of the source's agent library or capability definitions
    /// that is a link, or not the file or directory it must be.
    Unplain { path: PathBuf },
    /// Files the workspace already holds that differ from the source's, or
    /// cannot be compared with them.
    Differs { paths: Vec<PathBuf> },
}

impl std::fmt::Display for Hiring {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hiring::Unread { path, cause } => write!(
                formatter,
                "recipes add cannot read {}: {cause} (#696); nothing was written",
                path.display()
            ),
            Hiring::Unnamed { path, name } => write!(
                formatter,
                "{} names agent '{name}', which is not an agent name ({NAME_GRAMMAR}) (#696); \
                 nothing was written",
                path.display()
            ),
            Hiring::Charter { path, charter } => write!(
                formatter,
                "{} names charter '{charter}', which is not a relative path of plain names in \
                 its library (#696); nothing was written",
                path.display()
            ),
            Hiring::Uncharted {
                path,
                charter,
                library,
            } => write!(
                formatter,
                "{} names charter '{charter}', which its library {} does not hold, so the copy \
                 would bind to whatever this workspace holds there (#696); nothing was written",
                path.display(),
                library.display()
            ),
            Hiring::Undefined {
                path,
                capability,
                definitions,
            } => write!(
                formatter,
                "{} requests capability '{capability}', whose definition the source's {} does \
                 not hold, so the copy would bind to whatever this workspace defines there \
                 (#703); nothing was written",
                path.display(),
                definitions.display()
            ),
            Hiring::Unplain { path } => write!(
                formatter,
                "{} is a link, or not the file or directory recipes add copies from the source's \
                 agent library or capability definitions (#696); nothing was written",
                path.display()
            ),
            Hiring::Differs { paths } => {
                let paths: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                write!(
                    formatter,
                    "recipes add replaces no agent definition, charter or capability definition, \
                     and the source's differ from what stands at {} (#696): reconcile or remove \
                     each; nothing was written",
                    paths.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for Hiring {}

/// The agent library beside a source's recipe library, local or cloned:
/// the bundle directory's parent, when it is named `recipes` (a library, or
/// a folder's shelf), has its agents beside it; any other source has none
/// here, and a clone falls back to its own `agents/`.
pub(super) fn library_of(bundle: &Path) -> std::io::Result<Option<PathBuf>> {
    let bundle = bundle.canonicalize()?;
    let library = bundle
        .parent()
        .filter(|at| at.ends_with(brokkr_runtime::places::RECIPES));
    Ok(library
        .and_then(Path::parent)
        .map(|home| home.join(DEFAULT_AGENTS_DIR)))
}

/// Where the workspace at `root` keeps the capability definitions its
/// compile reads in `world`, by the compile's own rule
/// ([`capability_context`](brokkr_runtime::launch::capability_context)):
/// beside the map, else at `root`.
pub(super) fn definitions_of(root: &Path, world: Option<&World>) -> PathBuf {
    let context = brokkr_runtime::launch::capability_context(root, world, None, root);
    context.root.join(DEFINITIONS_DIR)
}

/// The files to copy from `source`, the source's agent library, into
/// `library`, the workspace's: each agent `recipes` name that `source`
/// holds, the charter its definition names, and the definition of each
/// capability it requests, from where the source's compile reads them
/// ([`define`]) into `definitions` (#703), but never one the workspace
/// already holds byte for byte. One it holds that differs refuses
/// ([`Hiring::Differs`], naming each), as does a name or a file that
/// cannot be read, and a charter or a definition the source lacks, which
/// the copy would bind to whatever the workspace holds there
/// ([`Hiring::Uncharted`], [`Hiring::Undefined`]). An agent name `source`
/// lacks, and a capability name outside the grammar, are the compile's to
/// judge.
pub(super) fn plan(
    recipes: &[&Path],
    source: Option<&Path>,
    library: &Path,
    definitions: &Path,
) -> Result<Vec<Hire>, Hiring> {
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    let (mut hires, mut asked) = (BTreeMap::new(), Vec::new());
    for name in named(recipes)? {
        let definition = format!("{name}.json");
        let Some(bytes) = read(source, &definition)? else {
            continue;
        };
        let path = source.join(&definition);
        let parsed = serde_json::from_slice::<Definition>(&bytes);
        let Definition {
            charter,
            capabilities,
        } = parsed.map_err(|e| unread(&path, &e))?;
        hire(
            &mut hires,
            library,
            &definition,
            format!("agent '{name}'"),
            bytes,
        );
        let requested: Vec<String> = capabilities.into_keys().filter(|c| is_name(c)).collect();
        if !requested.is_empty() {
            let (name, path) = (name.clone(), path.clone());
            asked.push(Asking {
                name,
                path,
                requested,
            });
        }
        let Some(charter) = charter else {
            continue;
        };
        if !charter
            .split(std::path::is_separator)
            .all(|part| plain(Path::new(part)))
        {
            return Err(Hiring::Charter { path, charter });
        }
        let Some(bytes) = read(source, &charter)? else {
            return Err(Hiring::Uncharted {
                path,
                charter,
                library: source.to_path_buf(),
            });
        };
        let what = format!("charter {charter} of agent '{name}'");
        hire(&mut hires, library, &charter, what, bytes);
    }
    define(&mut hires, asked, source, definitions)?;
    held(hires.into_values().collect())
}

/// The capabilities a planned agent requests: its name, where its
/// definition stands, and each capability name in the grammar it asks for.
struct Asking {
    name: String,
    path: PathBuf,
    requested: Vec<String>,
}

/// Plan into `definitions` the definition of each capability `asked`
/// requests, read where the source whose agent library is `source` keeps
/// them ([`sourced`]), found only when one is asked; one it lacks refuses
/// ([`Hiring::Undefined`]).
fn define(
    hires: &mut BTreeMap<PathBuf, Hire>,
    asked: Vec<Asking>,
    source: &Path,
    definitions: &Path,
) -> Result<(), Hiring> {
    if asked.is_empty() {
        return Ok(());
    }
    let from = sourced(source)?;
    for Asking {
        name,
        path,
        requested,
    } in asked
    {
        for capability in requested {
            let file = format!("{capability}.json");
            let Some(bytes) = read(&from, &file)? else {
                let definitions = from;
                return Err(Hiring::Undefined {
                    path,
                    capability,
                    definitions,
                });
            };
            let what = format!("capability '{capability}' of agent '{name}'");
            hire(hires, definitions, &file, what, bytes);
        }
    }
    Ok(())
}

/// Where the source whose agent library is `source` keeps the capability
/// definitions its own compile reads, by the compile's rule
/// ([`definitions_of`]) at the root that library stands in, by the one walk
/// of a layout ([`Layout::discover`]) from the directory beside it: the
/// repository's, where that directory is its `.brokkr` folder, else that
/// directory, never a folder by its name alone. A walk that refuses, and a
/// map there that cannot be read, refuse (fail closed); a map whose
/// crossings moved still says where it stands, all `add` asks of it.
fn sourced(source: &Path) -> Result<PathBuf, Hiring> {
    let home = source.parent().unwrap_or(source);
    let root = Layout::discover(home)
        .map_err(|e| unread(home, &e))?
        .workspace();
    let world = World::inspect(&root, None).map_err(|e| unread(&root, &e))?;
    Ok(definitions_of(&root, world.as_ref()))
}

/// Plan `bytes` at `relative` in `library`, once per destination.
fn hire(
    hires: &mut BTreeMap<PathBuf, Hire>,
    library: &Path,
    relative: &str,
    what: String,
    bytes: Vec<u8>,
) {
    let to = library.join(relative);
    hires.entry(to.clone()).or_insert(Hire { what, to, bytes });
}

/// Every agent name the sites of `recipes`' own `bundle.json`s hold.
fn named(recipes: &[&Path]) -> Result<BTreeSet<String>, Hiring> {
    let mut names = BTreeSet::new();
    for recipe in recipes {
        let path = recipe.join("bundle.json");
        let text = std::fs::read(&path).map_err(|e| unread(&path, &e))?;
        let layer: Layer = serde_json::from_slice(&text).map_err(|e| unread(&path, &e))?;
        let mut held = BTreeSet::new();
        layer.seats.values().for_each(|site| site.names(&mut held));
        if let Some(name) = held.iter().find(|name| !valid_name(name)) {
            let name = name.to_string();
            return Err(Hiring::Unnamed { path, name });
        }
        names.extend(held.into_iter().map(str::to_string));
    }
    Ok(names)
}

/// The bytes of the file at `relative` in `source`, walked a part at a time
/// by each part's own entry from `source` itself, never through a link:
/// `None` where nothing stands.
fn read(source: &Path, relative: &str) -> Result<Option<Vec<u8>>, Hiring> {
    if !stands(source, false)? {
        return Ok(None);
    }
    let mut at = source.to_path_buf();
    let parts: Vec<&str> = relative.split(std::path::is_separator).collect();
    for (index, part) in parts.iter().enumerate() {
        at.push(part);
        if !stands(&at, index + 1 == parts.len())? {
            return Ok(None);
        }
    }
    std::fs::read(&at).map(Some).map_err(|e| unread(&at, &e))
}

/// Whether an entry stands at `at`, judged as the entry itself: a `file`,
/// or a directory on the way to one. Anything else there refuses.
fn stands(at: &Path, file: bool) -> Result<bool, Hiring> {
    let found = brokkr_runtime::places::entry(at).map_err(|e| unread(at, &e))?;
    match found {
        None => Ok(false),
        Some(meta) if file && meta.is_file() || !file && meta.is_dir() => Ok(true),
        Some(_) => Err(Hiring::Unplain { path: at.into() }),
    }
}

/// The refusal of the file at `path`, which `cause` kept from being read.
fn unread(path: &Path, cause: &dyn std::fmt::Display) -> Hiring {
    Hiring::Unread {
        path: path.into(),
        cause: cause.to_string(),
    }
}

/// `hires` without the files the workspace's library already holds byte
/// for byte, or the refusal naming each it holds otherwise.
fn held(hires: Vec<Hire>) -> Result<Vec<Hire>, Hiring> {
    let (mut needed, mut differs) = (Vec::new(), Vec::new());
    for hire in hires {
        match brokkr_runtime::places::entry(&hire.to) {
            Ok(None) => needed.push(hire),
            _ if std::fs::read(&hire.to).is_ok_and(|bytes| bytes == hire.bytes) => {}
            _ => differs.push(hire.to),
        }
    }
    match differs.is_empty() {
        true => Ok(needed),
        false => Err(Hiring::Differs { paths: differs }),
    }
}

/// Write each of `hires`, making the directories it needs; every entry
/// made is pushed onto `made` as it is made, before anything is written to
/// it, so a refused install removes it. A file is created new, never
/// through an entry that appeared since it was planned.
pub(super) fn copy(hires: &[Hire], made: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for hire in hires {
        let parent = hire.to.parent().unwrap_or(Path::new(""));
        let missing: Vec<&Path> = parent.ancestors().take_while(|at| !at.exists()).collect();
        for dir in missing.into_iter().rev() {
            std::fs::create_dir(dir)?;
            made.push(dir.to_path_buf());
        }
        let mut file = std::fs::File::create_new(&hire.to)?;
        made.push(hire.to.clone());
        file.write_all(&hire.bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod agents_tests;
