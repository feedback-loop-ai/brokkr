//! `brokkr graph`: one recipe's state machine with the journal's traffic
//! on its rules and states (#568, the Library). The table resolves as
//! `brokkr recipes show` resolves a name — the realm's shelf — or else from
//! a bundle directory in the realm whose `bundle.json` names it, as `self`
//! is in an adopter. With no recipe named it draws the realm's busiest, so
//! the console reads a realm's known recipes before any has run. Every
//! recipe on the shelf is compiled: one that does not compile against this
//! Brokkr is drawn with the compiler's refusal and named among those that
//! do not, so a library written for an older Brokkr still lists them. The
//! figures are derived once, in [`brokkr_view::graph`]; this verb finds the
//! table and the hearths, hands them over with the clock, and prints what
//! comes back. It writes nothing (decision 0014).

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use brokkr_runtime::launch::{self, LaunchError};
use brokkr_runtime::realms::{Hearth, World};
use brokkr_runtime::{Bundle, CompileError};
use brokkr_view::{GraphQuery, GraphResolution, GraphTable, GraphView, Query, Range, Table};
use serde::Deserialize;

use super::analytics::{printed, range, read_hearths};
use crate::folder::Layout;
use crate::recipes::listed;
use crate::recipes::personal::ShelfPath;
use crate::{fleet_of, now_rfc3339, Invocation};

#[derive(clap::Args)]
#[group(skip)]
pub(crate) struct GraphArgs {
    /// The recipe whose graph is drawn, by name: on the realm's shelf, or a bundle directory
    /// in the realm whose bundle.json names it (default the realm's recipe with the most runs
    /// in the range, else the first it knows by name).
    #[arg(long)]
    pub(crate) recipe: Option<String>,
    /// The realm the recipe resolves in and whose runs are counted (default the workspace's
    /// realm, else the map's first).
    #[arg(long)]
    pub(crate) realm: Option<String>,
    /// How far back from now: 24h, 7d or 30d.
    #[arg(long, default_value = "7d", value_parser = range)]
    pub(crate) range: Range,
    /// The world's map — every hearth it names is read (default the found folder's
    /// .brokkr/realms.json, or a library's ./realms.json, when present).
    #[arg(long)]
    pub(crate) realms: Option<PathBuf>,
    /// The workspace journal. Outranks the map's journal; without either, the found folder's
    /// .brokkr/state/forge.db, or a library's .forge/forge.db.
    #[arg(long)]
    pub(crate) db: Option<PathBuf>,
    /// Emit the view model verbatim — this is what the console reads.
    #[arg(long)]
    pub(crate) json: bool,
}

/// A realm the map does not name. Its `Display` is the operator's text
/// (decision 0071 ruling 8), by hand as this crate's errors are.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UnknownRealm {
    realm: String,
    known: Vec<String>,
}

impl fmt::Display for UnknownRealm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no realm '{}' in the map: it names {}",
            self.realm,
            self.known.join(", ")
        )
    }
}

impl std::error::Error for UnknownRealm {}

/// No recipe named, and none the realm knows to draw.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NoRecipe {
    realm: String,
}

impl fmt::Display for NoRecipe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "no recipe in {}: none is installed and no run in the range names one",
            self.realm
        )
    }
}

impl std::error::Error for NoRecipe {}

/// The name a bare journal's runs carry when no map names their realm:
/// the label its hearth is listed under.
const UNMAPPED: &str = "world";

/// The realm the graph is drawn in and the folder its recipes resolve
/// from: the named realm's, else the realm the workspace is, else the
/// map's first; with no map, the workspace under the name it was given or
/// its bare journal's.
fn place(
    world: Option<&World>,
    named: Option<&str>,
    workspace: &Path,
) -> Result<(String, PathBuf), UnknownRealm> {
    let Some(world) = world else {
        let realm = named.unwrap_or(UNMAPPED);
        return Ok((realm.to_string(), workspace.to_path_buf()));
    };
    let realms = &world.map.realms;
    let found = match named {
        Some(name) => realms.iter().find(|realm| realm.name == name),
        None => world.realm_for(workspace).or(realms.first()),
    };
    let found = found.ok_or_else(|| UnknownRealm {
        realm: named.unwrap_or_default().to_string(),
        known: realms.iter().map(|realm| realm.name.clone()).collect(),
    })?;
    Ok((found.name.clone(), world.path_of(found)))
}

/// The part of `bundle.json` that names a bundle directory.
#[derive(Deserialize)]
struct Named {
    name: String,
}

/// How deep under the realm's folder a bundle directory is looked for:
/// `recipes/self` is two down.
const BUNDLE_DEPTH: usize = 2;

/// `dir` with the name its `bundle.json` gives it, when it gives one.
fn named(dir: &Path) -> Option<(String, PathBuf)> {
    let bytes = std::fs::read(dir.join("bundle.json")).ok()?;
    let named = serde_json::from_slice::<Named>(&bytes).ok()?;
    Some((named.name, dir.to_path_buf()))
}

/// The bundle directories under `dir`, to `depth`, each with the name its
/// `bundle.json` gives it, sorted by path: hidden directories and build
/// output are not looked in.
fn bundles(dir: &Path, depth: usize) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut subdirs: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        })
        .collect();
    subdirs.sort();
    let mut found = Vec::new();
    for sub in subdirs {
        found.extend(named(&sub));
        if depth > 1 {
            found.extend(bundles(&sub, depth - 1));
        }
    }
    found
}

/// Every recipe the realm's `folder` resolves by name, in the order a name
/// is looked up: its shelf as `recipes show` reads it, then the folder's
/// implicit bundle (`.brokkr` itself, in an adopter), then its visible
/// bundle directories.
fn shelf(folder: &Path) -> Result<Vec<(String, PathBuf)>> {
    let layout = Layout::discover(folder)?;
    layout.refuse_old()?;
    let recipes = ShelfPath::new(folder.join(layout.recipes(None).path()));
    let mut found = listed(&recipes).unwrap_or_default();
    found.extend(layout.bundle().ok().as_deref().and_then(named));
    found.extend(bundles(folder, BUNDLE_DEPTH));
    Ok(found)
}

/// How the recipe asked for resolved where the realm's shelf names it.
enum Compiled {
    /// Its table, compiled.
    Bundle(Box<Bundle>),
    /// No recipe of that name is on the shelf.
    NotOnShelf,
    /// It is on the shelf and does not compile against this Brokkr.
    Refused { reason: String, bundle: String },
}

/// What compiling the realm's recipes found: how the one asked for
/// resolved, and the names of every one that does not compile.
struct Compiles {
    asked: Compiled,
    refused: Vec<String>,
}

/// A compile refusal in its own words, without the `bundle: ` its error
/// wraps it in, once for the bundle and once more for a composed one's
/// chain.
fn reason_of(error: &CompileError) -> String {
    let words = error.to_string();
    let mut reason = words.as_str();
    while let Some(rest) = reason.strip_prefix("bundle: ") {
        reason = rest;
    }
    reason.to_string()
}

/// `dir` as a path in the realm's `folder` when it lies there, else whole:
/// the two read through their links, as a shelf and a map can name one
/// folder by two spellings.
fn within(dir: &Path, folder: &Path) -> String {
    let real = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let (dir, folder) = (real(dir), real(folder));
    dir.strip_prefix(&folder)
        .unwrap_or(&dir)
        .display()
        .to_string()
}

/// Each recipe on the shelf compiled where it resolves, on the one path
/// `recipes show` compiles it on, the first directory of a name being the
/// one it resolves to. A recipe that does not compile is named, never the
/// graph's failure; any other failure to compile is.
fn compiled(
    world: Option<&World>,
    folder: &Path,
    shelf: &[(String, PathBuf)],
    recipe: &str,
) -> Result<Compiles> {
    let workspace = Layout::discover(folder)?.workspace();
    let mut compiles = Compiles {
        asked: Compiled::NotOnShelf,
        refused: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    for (name, dir) in shelf.iter().filter(|(name, _)| seen.insert(name)) {
        let found = match launch::compile_for(&workspace, dir, world, folder) {
            Ok(bundle) => Compiled::Bundle(Box::new(bundle)),
            Err(LaunchError::Compile(error)) => {
                compiles.refused.push(name.clone());
                Compiled::Refused {
                    reason: reason_of(&error),
                    bundle: within(dir, folder),
                }
            }
            Err(error) => return Err(error.into()),
        };
        if name == recipe {
            compiles.asked = found;
        }
    }
    Ok(compiles)
}

/// What the view is asked for, beside the hearths.
struct Asked<'a> {
    recipe: &'a str,
    realm: &'a str,
    range: Range,
    now: &'a str,
}

/// The graph over `hearths`, drawn from the asked recipe's table when it
/// compiled.
fn derive(
    hearths: &[Hearth],
    compiles: &Compiles,
    shelf: &[(String, PathBuf)],
    asked: &Asked,
) -> Result<GraphView> {
    let shelved: Vec<String> = shelf.iter().map(|(name, _)| name.clone()).collect();
    let table = match &compiles.asked {
        Compiled::Bundle(bundle) => GraphResolution::Compiled(GraphTable {
            machine: &bundle.machine,
            manifest: &bundle.manifest,
        }),
        Compiled::NotOnShelf => GraphResolution::NotFound,
        Compiled::Refused { reason, bundle } => GraphResolution::Refused { reason, bundle },
    };
    let view = read_hearths(hearths, |grouped| {
        brokkr_view::graph(&GraphQuery {
            hearths: grouped,
            recipe: asked.recipe,
            realm: asked.realm,
            table,
            shelved: &shelved,
            does_not_compile: &compiles.refused,
            range: asked.range,
            now: asked.now,
        })
    });
    view.map_err(|error| anyhow::Error::msg(error.to_string()))
}

/// The recipe drawn when none is named: the realm's with the most runs in
/// the range as the analytics readout counts them, the first by name where
/// runs tie, else the first the realm's folder resolves by name.
fn chosen(
    hearths: &[Hearth],
    shelf: &[(String, PathBuf)],
    realm: &str,
    range: Range,
    now: &str,
) -> Result<String> {
    let figures = read_hearths(hearths, |grouped| {
        brokkr_view::analytics(&Query {
            hearths: grouped,
            range,
            realm: Some(realm),
            now,
        })
    })
    .map_err(|error| anyhow::Error::msg(error.to_string()))?;
    let mut busiest: Option<(&String, usize)> = None;
    for row in figures.recipes.iter().filter(|row| row.realm == realm) {
        if busiest.is_none_or(|(_, runs)| row.runs > runs) {
            busiest = Some((&row.recipe, row.runs));
        }
    }
    let first = shelf.iter().map(|(name, _)| name).min();
    let recipe = busiest.map(|(recipe, _)| recipe).or(first);
    let recipe = recipe.ok_or_else(|| NoRecipe {
        realm: realm.to_string(),
    })?;
    Ok(recipe.clone())
}

/// `brokkr graph`: the recipe's graph for the range ending now.
pub(crate) fn graph(workspace: &Path, args: GraphArgs) -> Result<ExitCode> {
    let (world, hearths) = fleet_of(workspace, args.realms, args.db, Invocation::resolve)?;
    let (realm, folder) = place(world.as_ref(), args.realm.as_deref(), workspace)?;
    let shelf = shelf(&folder)?;
    let now = now_rfc3339();
    let recipe = match args.recipe {
        Some(recipe) => recipe,
        None => chosen(&hearths, &shelf, &realm, args.range, &now)?,
    };
    let compiles = compiled(world.as_ref(), &folder, &shelf, &recipe)?;
    let asked = Asked {
        recipe: &recipe,
        realm: &realm,
        range: args.range,
        now: &now,
    };
    let view = derive(&hearths, &compiles, &shelf, &asked)?;
    printed(&view, args.json, paint)
}

/// One line per state and per rule, then the rest the view states,
/// painted from the view's own text.
fn paint(view: &GraphView) -> String {
    let mut out = format!(
        "{} / {} · {} → {} · {} runs\n",
        view.realm, view.recipe, view.from, view.to, view.runs
    );
    match &view.table {
        Table::Resolved => {}
        Table::NotFound { text } => out += &format!("{text}\n"),
        Table::Refused {
            text,
            reason,
            bundle,
        } => out += &format!("{text}\n  {reason}\n  bundle {bundle}\n"),
    }
    for state in &view.states {
        out += &format!(
            "state  {} · entered {} · median {} · p90 {} · {} · {}\n",
            state.name,
            state.entered,
            state.median.text,
            state.p90.text,
            state.cost.text,
            state.tokens.line()
        );
    }
    for rule in &view.rules {
        out += &format!(
            "rule   {} · {} → {} · {}\n",
            rule.text,
            rule.from,
            rule.next.as_deref().unwrap_or("park"),
            rule.kind.word()
        );
    }
    for retired in &view.retired {
        let id = retired.rule_id.as_deref().unwrap_or_default();
        out += &format!("retired {id} · {}\n", retired.count);
    }
    if let Some(drift) = &view.drift_text {
        out += &format!("drift  {drift}\n");
    }
    out += &format!("ends   {}\n", view.ends.text);
    for live in &view.live {
        let state = live.state.as_deref().unwrap_or("—");
        out += &format!("live   {} · {state}\n", live.label);
    }
    out
}

#[cfg(test)]
mod tests;
