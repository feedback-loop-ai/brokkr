//! The agents a recipe names, installed with it (#696 ruling 5): `recipes
//! add` copies each agent definition a copied recipe's sites name, and the
//! charter that definition names, from the source's agent library into the
//! workspace's, so the compile still reads one agents directory. An
//! agent's own `extends` base is not copied: it resolves in the workspace's
//! library, where `init` wrote `implementer`, `intake` and `reviewer`.
//! Every file is planned, and every conflict refused, before anything is
//! written; the copies are written from the bytes that were compared.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use brokkr_runtime::agents::{valid_name, NAME_GRAMMAR};
use brokkr_runtime::bundle::DEFAULT_AGENTS_DIR;
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

/// An agent definition, as far as it names a charter.
#[derive(Deserialize)]
struct Definition {
    charter: Option<String>,
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
    /// An entry of the source's agent library that is a link, or not the
    /// file or directory it must be.
    Unplain { path: PathBuf },
    /// Files the workspace's library already holds that differ from the
    /// source's, or cannot be compared with them.
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
            Hiring::Unplain { path } => write!(
                formatter,
                "{} is a link, or not the file or directory recipes add copies from the source's \
                 agent library (#696); nothing was written",
                path.display()
            ),
            Hiring::Differs { paths } => {
                let paths: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
                write!(
                    formatter,
                    "recipes add replaces no agent definition or charter, and the source's \
                     differ from what stands at {} (#696): reconcile or remove each; nothing was \
                     written",
                    paths.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for Hiring {}

/// The agent library beside a local source's recipe library: the bundle
/// directory's parent, when it is named `recipes` (a library, or a
/// folder's shelf), has its agents beside it; any other source has none.
pub(super) fn library_of(bundle: &Path) -> std::io::Result<Option<PathBuf>> {
    let bundle = bundle.canonicalize()?;
    let library = bundle.parent().filter(|at| at.ends_with("recipes"));
    Ok(library
        .and_then(Path::parent)
        .map(|home| home.join(DEFAULT_AGENTS_DIR)))
}

/// The files to copy from `source`, the source's agent library, into
/// `library`, the workspace's: each agent `recipes` name that `source`
/// holds, and the charter its definition names, but never one `library`
/// already holds byte for byte. One `library` holds that differs refuses
/// ([`Hiring::Differs`], naming each), as does a name or a file that
/// cannot be read. A name `source` lacks is the compile's to judge.
pub(super) fn plan(
    recipes: &[&Path],
    source: Option<&Path>,
    library: &Path,
) -> Result<Vec<Hire>, Hiring> {
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    let mut hires = BTreeMap::new();
    for name in named(recipes)? {
        let definition = format!("{name}.json");
        let Some(bytes) = read(source, &definition)? else {
            continue;
        };
        let path = source.join(&definition);
        let parsed = serde_json::from_slice::<Definition>(&bytes);
        let charter = parsed.map_err(|e| unread(&path, &e))?.charter;
        hire(
            &mut hires,
            library,
            &definition,
            format!("agent '{name}'"),
            bytes,
        );
        let Some(charter) = charter else {
            continue;
        };
        if !charter
            .split(std::path::is_separator)
            .all(|part| plain(Path::new(part)))
        {
            return Err(Hiring::Charter { path, charter });
        }
        if let Some(bytes) = read(source, &charter)? {
            let what = format!("charter {charter} of agent '{name}'");
            hire(&mut hires, library, &charter, what, bytes);
        }
    }
    held(hires.into_values().collect())
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
