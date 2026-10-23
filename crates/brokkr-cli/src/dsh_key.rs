//! Whether a folder's dsh can reach a key for each route its scaffold
//! pins (#717; the operator's ruling of 2026-10-22, item 2). dsh 0.2's
//! shipped headless bundle disables its `credentials` service, so a
//! headless dsh reads a route's key only from its launching environment,
//! `<cwd>/.env` and `$DSH_HOME/.env`, unless the headless profile's patch
//! re-enables the service. A seat's dsh runs in the repository, so its
//! `<cwd>` is the folder's root.
//!
//! dsh names no command that reports a route's credential status short of
//! a model call: its launcher's flags are `-V`, `--profile`, `--patch`,
//! `--dump-config` and `--dump-default-config`, and its headless
//! profile's `-h` alone (`adapters/dsh.json`'s hands record). So no dsh is
//! asked. The check reads the PRESENCE of each documented source and
//! never a value: a variable is asked whether it is set and non-empty, a
//! `.env` file is scanned for the name before an `=` and whether anything
//! follows it, the bytes after that `=` never collected, and the
//! profile's patch is read for the one row that re-enables the service.
//! Nothing is logged, journaled or passed to a seat, and every line names
//! the variable and never its value.
//!
//! It fails closed: a file it cannot read, a patch row it does not
//! recognise, and a route whose variable neither the adapter nor dsh's
//! own default names are no key.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use brokkr_protocol::adapters::{split_route, AdapterKind, DshSeams};
use brokkr_runtime::agents::Adapter;
use brokkr_runtime::Adapters;

use crate::render::Safe;

/// The key dsh's default route reads when the adapter names none: the
/// variable `dsh-llm-deepseek` authenticates with (its
/// `DEFAULT_API_KEY_ENV`, `adapters/dsh.json`'s `harness_auth` record).
const DEFAULT_KEY: &str = "DEEPSEEK_API_KEY";

/// The row a patch writes to switch dsh's credentials service back on.
const SERVICE: &str = "credentials";

/// Where a headless dsh reaches a route's key, in the order it is asked.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// The variable is set in the environment `brokkr` was launched in,
    /// which a seat's dsh inherits.
    Environment,
    /// A `.env` file dsh reads names it.
    File(PathBuf),
    /// The headless profile's patch at this path re-enables dsh's
    /// credentials service, whose store dsh answers for.
    Service(PathBuf),
}

/// One route's answer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    Reached {
        route: String,
        variable: String,
        source: Source,
    },
    /// No source names `variable`; `files` are the `.env` files asked,
    /// and `patch` the headless profile's patch, or why no dsh home was
    /// resolved to hold one.
    Unreached {
        route: String,
        variable: String,
        files: Vec<PathBuf>,
        patch: Result<PathBuf, String>,
    },
    /// Neither the adapter's `credentials` nor dsh's default names the
    /// variable this route reads, so nothing can be asked for it.
    Undeclared { route: String },
}

impl Reach {
    /// The line's name, on both surfaces.
    pub(crate) fn what(&self) -> String {
        let (Reach::Reached { route, .. }
        | Reach::Unreached { route, .. }
        | Reach::Undeclared { route }) = self;
        format!("dsh key {}", Safe::new(route).as_str())
    }

    /// The line's words, on both surfaces.
    pub(crate) fn detail(&self) -> String {
        match self {
            Reach::Reached {
                variable, source, ..
            } => format!(
                "dsh reaches {} {}",
                Safe::new(variable).as_str(),
                source.through()
            ),
            Reach::Unreached {
                variable,
                files,
                patch,
                ..
            } => unreached(Safe::new(variable).as_str(), files, patch),
            Reach::Undeclared { .. } => "the dsh adapter names no credential variable for \
                                         this route, so whether dsh reaches a key for it is \
                                         unknown; declare one under the adapter's `credentials`"
                .to_string(),
        }
    }
}

impl Source {
    fn through(&self) -> String {
        match self {
            Source::Environment => "through the launching environment".to_string(),
            Source::File(path) => format!("through {}, which names it", path.display()),
            Source::Service(patch) => format!(
                "through dsh's credentials service, which {} re-enables; its store is \
                 dsh's to answer for and is not read",
                patch.display()
            ),
        }
    }
}

/// An unreached route's words: what was asked, and the two remedies.
fn unreached(variable: &str, files: &[PathBuf], patch: &Result<PathBuf, String>) -> String {
    let files: Vec<String> = files
        .iter()
        .map(|file| file.display().to_string())
        .collect();
    let patch = match patch {
        Ok(patch) => patch.display().to_string(),
        Err(cause) => format!(
            "the headless profile's cordis.patch.yml (no dsh home was resolved: {})",
            Safe::new(cause).as_str()
        ),
    };
    format!(
        "dsh reaches no {variable}: it is not set in the launching environment, no .env file \
         at {} names it, and the headless profile leaves dsh's credentials service disabled; \
         export {variable} where brokkr runs, or add the row `- id: {SERVICE}` with \
         `  disabled: false` to {patch} and store {variable} through dsh's credentials service",
        files.join(" or ")
    )
}

/// The sources one check asks: whether a variable is set, the repository
/// a seat's dsh runs in, and the dsh home, or why none was resolved.
pub(crate) struct Sources<'a> {
    pub(crate) set: fn(&str) -> bool,
    pub(crate) repo: &'a Path,
    pub(crate) home: Result<PathBuf, String>,
}

/// Each route the folder's dsh adapter under `adapters` pins, answered
/// for `repo` and the dsh home dsh's own seam resolves; none where the
/// folder hires no dsh, or its adapters do not load (doctor's adapters
/// line already says so).
pub(crate) fn of_folder(adapters: &Path, repo: &Path) -> Vec<Reach> {
    let home = DshSeams::resolve().map(|seams| seams.home);
    let sources = Sources {
        set: set_and_not_empty,
        repo,
        home: home.map_err(|cause| cause.to_string()),
    };
    in_folder(adapters, &sources)
}

/// [`of_folder`] over injected sources.
pub(crate) fn in_folder(adapters: &Path, sources: &Sources<'_>) -> Vec<Reach> {
    Adapters::load(adapters)
        .map(|loaded| {
            let dsh = loaded.adapter("dsh");
            dsh.map_or_else(Vec::new, |dsh| reaches(dsh, sources))
        })
        .unwrap_or_default()
}

/// Whether `name` is set to anything at all: a boolean, never the value.
fn set_and_not_empty(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

/// Each route `adapter`'s models are served on, once, with the variable
/// it reads: the adapter's `credentials` row, else dsh's default key for
/// a bare id, which rides dsh's default route.
fn routes(adapter: &Adapter) -> BTreeMap<&str, Option<&str>> {
    let mut routes: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    for id in adapter.models.values() {
        let route = AdapterKind::Dsh.served_route(id).unwrap_or(id);
        let declared = adapter.credentials.get(route).map(String::as_str);
        let variable = declared.or(split_route(id).0.is_none().then_some(DEFAULT_KEY));
        let entry = routes.entry(route).or_insert(variable);
        *entry = entry.or(variable);
    }
    routes
}

/// Each of `adapter`'s routes, answered against `sources`.
pub(crate) fn reaches(adapter: &Adapter, sources: &Sources<'_>) -> Vec<Reach> {
    let mut answers = Vec::new();
    for (route, variable) in routes(adapter) {
        let route = route.to_string();
        let Some(variable) = variable else {
            answers.push(Reach::Undeclared { route });
            continue;
        };
        let files: Vec<PathBuf> = std::iter::once(sources.repo.join(".env"))
            .chain(sources.home.as_ref().ok().map(|home| home.join(".env")))
            .collect();
        let patch = sources.home.as_ref().map(|home| {
            home.join("profiles")
                .join("headless")
                .join("cordis.patch.yml")
        });
        let variable = variable.to_string();
        answers.push(match source(&variable, sources.set, &files, &patch) {
            Some(source) => Reach::Reached {
                route,
                variable,
                source,
            },
            None => Reach::Unreached {
                route,
                variable,
                files,
                patch: patch.map_err(Clone::clone),
            },
        });
    }
    answers
}

/// The first source that names `variable`, in dsh's order.
fn source(
    variable: &str,
    set: fn(&str) -> bool,
    files: &[PathBuf],
    patch: &Result<PathBuf, &String>,
) -> Option<Source> {
    if set(variable) {
        return Some(Source::Environment);
    }
    if let Some(file) = files.iter().find(|file| names(file, variable)) {
        return Some(Source::File(file.clone()));
    }
    let patch = patch.as_ref().ok()?;
    let text = std::fs::read_to_string(patch).ok()?;
    re_enables(&text).then(|| Source::Service(patch.clone()))
}

/// Where a `.env` scan stands on its current line.
enum Line {
    /// Before any `=`, holding at most a name's worth of bytes.
    Name(Vec<u8>),
    /// After the `=` of the variable asked about, until a byte that is
    /// a value's.
    Value,
    /// The rest of a line that is not the variable's, unread.
    Skip,
}

/// Whether the `.env` file at `path` gives `variable` a value: a line
/// `[export ]<variable>=` with anything but blanks and quotes after it.
/// The bytes after an `=` are never collected, and a line's name never
/// grows past the longest name that could match.
fn names(path: &Path, variable: &str) -> bool {
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut line = Line::Name(Vec::new());
    for byte in std::io::BufReader::new(file).bytes() {
        let Ok(byte) = byte else {
            return false;
        };
        match step(line, byte, variable) {
            Some(next) => line = next,
            None => return true,
        }
    }
    false
}

/// The scan's next state after `byte`, or `None` once a byte of
/// `variable`'s value is seen, which is the answer and is not kept.
fn step(line: Line, byte: u8, variable: &str) -> Option<Line> {
    let longest = variable.len() + "export ".len() + 16;
    Some(match (line, byte) {
        (_, b'\n') => Line::Name(Vec::new()),
        (Line::Name(name), b'=') if is_variable(&name, variable) => Line::Value,
        (Line::Name(_), b'=') => Line::Skip,
        (Line::Name(name), _) if name.len() >= longest => Line::Skip,
        (Line::Name(mut name), byte) => {
            name.push(byte);
            Line::Name(name)
        }
        (Line::Value, byte) if !matches!(byte, b' ' | b'\t' | b'\r' | b'"' | b'\'') => return None,
        (Line::Value, _) => Line::Value,
        (Line::Skip, _) => Line::Skip,
    })
}

/// Whether a line's bytes before its `=` name `variable`.
fn is_variable(name: &[u8], variable: &str) -> bool {
    let name = String::from_utf8_lossy(name);
    let name = name.trim();
    name.strip_prefix("export ").unwrap_or(name).trim() == variable
}

/// One top-level row of a profile patch, as far as this check reads it.
#[derive(Default)]
struct Row {
    service: bool,
    disabled: Option<bool>,
}

/// Whether a profile patch's LAST row for the credentials service writes
/// `disabled: false`. Only a top-level row (`- ` at the line's start) and
/// its own keys (two spaces in) are read; a value other than `false` is
/// read as disabled, and a nested key is not the row's.
fn re_enables(patch: &str) -> bool {
    let mut rows: Vec<Row> = Vec::new();
    for line in patch.lines() {
        let key = match line.strip_prefix("- ") {
            Some(key) => {
                rows.push(Row::default());
                key
            }
            None => line.strip_prefix("  ").unwrap_or_default(),
        };
        let (Some(row), Some((key, value))) = (rows.last_mut(), key.split_once(':')) else {
            continue;
        };
        let value = value.split(" #").next().unwrap_or_default().trim();
        let value = value.trim_matches(|quote| quote == '\'' || quote == '"');
        match key {
            "id" => row.service = value == SERVICE,
            "disabled" => row.disabled = Some(value != "false"),
            _ => {}
        }
    }
    let last = rows
        .iter()
        .rev()
        .find(|row| row.service && row.disabled.is_some());
    last.is_some_and(|row| row.disabled == Some(false))
}

#[cfg(test)]
mod tests;
