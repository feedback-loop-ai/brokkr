//! Whether a folder's dsh can reach a key for each route its scaffold
//! pins (#717; the operator's ruling of 2026-10-22, item 2). dsh 0.2's
//! shipped headless bundle disables its `credentials` service, so a
//! headless dsh reads a route's key only from its launching environment,
//! `<cwd>/.env` and `$DSH_HOME/.env`, unless a patch layer re-enables the
//! service. A seat's dsh runs in the repository, so its `<cwd>` is the
//! folder's root.
//!
//! dsh names no command that reports a route's credential status short of
//! a model call: its launcher's flags are `-V`, `--profile`, `--patch`,
//! `--dump-config` and `--dump-default-config`, and its headless
//! profile's `-h` alone (`adapters/dsh.json`'s hands record). So no dsh is
//! asked, and no key is read (the operator's ruling of 2026-10-24, "Never
//! open .env"):
//!
//! - a variable is asked of the environment the process already holds
//!   whether it is set and non-empty, through `getenv`'s pointer, and no
//!   byte of its value is copied;
//! - a `.env` file is never opened: its name is looked up, and a present
//!   one is reported present but unread, since Brokkr cannot confirm the
//!   key is in it;
//! - the patch layers dsh composes after its shipped bundles are read for
//!   the `credentials` row alone, in dsh's order: the headless profile's
//!   `cordis.patch.yml`, then the home's own (docs/evidence/adapters/
//!   slice-two-mcp-isolation.md, "the profile patch, the home patch,
//!   `--patch` overlays"). The seat's `--patch` overlay is the engine's
//!   route row, which brokkr-protocol admits only as `llm-pi-ai`'s, so it
//!   writes no `credentials` row. A layer is read by [`patch`]'s reader,
//!   and one it cannot read answers "can't tell", never ok.
//!
//! Nothing is logged, journaled or passed to a seat, and every line names
//! the variable and never its value.
//!
//! What it models is a seat served from the operator's own dsh home, the
//! one `DshSeams::resolve` resolves. A seat whose launch carries the
//! engine's isolation intent (decision 0065 slice two) is served from an
//! engine-only home instead, where neither the operator's patches nor
//! `$DSH_HOME/.env` are read and a `<workdir>/.env` refuses the launch.
//! Which seats those are is decided inside brokkr-protocol alone, and is
//! not yet modelled here (#717's fix round 717d2, reported blocked on it).
//! That path is re-checked when U8h lands and hides the secrets store
//! inside every workspace box: the check would then ask whether
//! `<repo>/.forge/secrets.env` is present by looking its name up, and
//! whether the seat binds the route's variable by name, and never read
//! the store.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_protocol::adapters::{split_route, AdapterKind, CompositeError, DshSeams};
use brokkr_runtime::agents::Adapter;
use brokkr_runtime::Adapters;

use crate::render::Safe;
use patch::{Unread, Write, SERVICE};

mod patch;

/// The key dsh's default route reads when the adapter names none: the
/// variable `dsh-llm-deepseek` authenticates with (its
/// `DEFAULT_API_KEY_ENV`, `adapters/dsh.json`'s `harness_auth` record).
const DEFAULT_KEY: &str = "DEEPSEEK_API_KEY";

/// Where a headless dsh reaches a route's key, as far as this check can
/// confirm it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// The variable is set in the environment `brokkr` was launched in,
    /// which a seat's dsh inherits.
    Environment,
    /// The patch layer at this path re-enables dsh's credentials service
    /// last, whose store dsh answers for.
    Service(PathBuf),
}

/// Why a source dsh reads cannot be told to give the key or not.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Untold {
    /// A `.env` file dsh reads is present, and is never opened.
    Dotenv(PathBuf),
    /// The patch layer at this path is present and not read past.
    Patch(PathBuf, Unread),
}

/// Why dsh's credentials service stays disabled for a seat.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Off {
    /// dsh's shipped headless bundle disables it and no layer re-enables
    /// it; a row in the headless profile's patch at this path would.
    Shipped(PathBuf),
    /// The layer at this path disables it last.
    Layer(PathBuf),
}

/// One route's answer.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reach {
    Reached {
        route: String,
        variable: String,
        source: Source,
    },
    /// The variable is not in the environment, and a source dsh reads
    /// after it cannot be told.
    Untold {
        route: String,
        variable: String,
        why: Untold,
    },
    /// No source gives `variable`; `files` are the `.env` names looked
    /// up, and `off` why the service stays disabled, or why no dsh home
    /// was resolved to hold a patch.
    Unreached {
        route: String,
        variable: String,
        files: Vec<PathBuf>,
        off: Result<Off, CompositeError>,
    },
    /// Neither the adapter's `credentials` nor dsh's default names the
    /// variable this route reads, so nothing can be asked for it.
    Undeclared { route: String },
}

impl Reach {
    /// The line's name, on both surfaces.
    pub(crate) fn what(&self) -> String {
        let (Reach::Reached { route, .. }
        | Reach::Untold { route, .. }
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
            Reach::Untold { variable, why, .. } => untold(Safe::new(variable).as_str(), why),
            Reach::Unreached {
                variable,
                files,
                off,
                ..
            } => unreached(Safe::new(variable).as_str(), files, off),
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
            Source::Service(patch) => format!(
                "through dsh's credentials service, which {} re-enables; its store is \
                 dsh's to answer for and is not read",
                patch.display()
            ),
        }
    }
}

/// An untold route's words: what could not be told, and why.
fn untold(variable: &str, why: &Untold) -> String {
    let unset = format!("{variable} is not set in the launching environment");
    match why {
        Untold::Dotenv(file) => format!(
            "{unset}, and {} is present but unread: Brokkr does not read it, so it can't confirm \
             {variable} is in it",
            file.display()
        ),
        Untold::Patch(patch, unread) => format!(
            "{unset} and no .env file dsh reads is present, but whether a patch re-enables dsh's \
             credentials service can't be told: {}: {unread}",
            patch.display()
        ),
    }
}

/// An unreached route's words: what was asked, and the two remedies.
fn unreached(variable: &str, files: &[PathBuf], off: &Result<Off, CompositeError>) -> String {
    let files: Vec<String> = files
        .iter()
        .map(|file| file.display().to_string())
        .collect();
    let (off, patch) = match off {
        Ok(Off::Shipped(patch)) => (
            "dsh's shipped headless bundle leaves its credentials service disabled".to_string(),
            patch.display().to_string(),
        ),
        Ok(Off::Layer(patch)) => (
            format!("{} disables dsh's credentials service", patch.display()),
            patch.display().to_string(),
        ),
        Err(cause) => (
            format!(
                "no dsh home was resolved ({}), so no patch re-enables dsh's credentials service",
                Safe::new(&cause.to_string()).as_str()
            ),
            "the headless profile's cordis.patch.yml".to_string(),
        ),
    };
    format!(
        "dsh reaches no {variable}: it is not set in the launching environment, no .env file is \
         present at {}, and {off}; export {variable} where brokkr runs, or write the row \
         `- id: {SERVICE}` with `  disabled: false` in {patch} and store {variable} through \
         dsh's credentials service",
        files.join(" or ")
    )
}

/// The sources one check asks: whether a variable is set, the repository
/// a seat's dsh runs in, and the dsh home, or why none was resolved.
pub(crate) struct Sources<'a> {
    pub(crate) set: fn(&str) -> bool,
    pub(crate) repo: &'a Path,
    pub(crate) home: Result<PathBuf, CompositeError>,
}

impl<'a> Sources<'a> {
    /// What a seat served from the operator's home reads: the process's
    /// environment, `repo`, and the home of dsh's own `seams`.
    fn served(repo: &'a Path, seams: Result<DshSeams, CompositeError>) -> Sources<'a> {
        Sources {
            set: set_and_not_empty,
            repo,
            home: seams.map(|seams| seams.home),
        }
    }
}

/// Each route the folder's dsh adapter under `adapters` pins, answered
/// for `repo` and the dsh home dsh's own seam resolves; none where the
/// folder hires no dsh, or its adapters do not load (doctor's adapters
/// line already says so).
pub(crate) fn of_folder(adapters: &Path, repo: &Path) -> Vec<Reach> {
    in_folder(adapters, &Sources::served(repo, DshSeams::resolve()))
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

extern "C" {
    fn getenv(name: *const std::os::raw::c_char) -> *const std::os::raw::c_char;
}

/// Whether `name` is set to anything at all, asked of the environment the
/// process already holds: `getenv` hands back a pointer into it, and only
/// whether that pointer is null and its first byte NUL is read, so no byte
/// of the value is copied (`std::env::var_os` copies the whole value).
fn set_and_not_empty(name: &str) -> bool {
    let Ok(name) = std::ffi::CString::new(name) else {
        return false;
    };
    // SAFETY: `name` is NUL-terminated and outlives the call. `getenv`
    // returns null or a pointer to a NUL-terminated value inside the
    // process's environment, so its first byte is readable. brokkr-cli
    // never writes its own environment (it calls no `set_var` or
    // `remove_var`), so no write can move the value while it is read.
    unsafe {
        let value = getenv(name.as_ptr());
        !value.is_null() && *value != 0
    }
}

/// Whether a `.env` dsh reads is there: its name is looked up and the
/// file never opened, so nothing of it reaches Brokkr. A name that cannot
/// be looked up counts as present, as brokkr-protocol's own `<workdir>/.env`
/// refusal counts it.
fn present(path: &Path) -> bool {
    !matches!(
        std::fs::symlink_metadata(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
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
    routes(adapter)
        .into_iter()
        .map(|(route, variable)| match variable {
            Some(variable) => reach(route.to_string(), variable.to_string(), sources),
            None => Reach::Undeclared {
                route: route.to_string(),
            },
        })
        .collect()
}

/// One route's answer, each source in dsh's order: the environment, each
/// `.env` dsh reads, then the patch layers of the home.
fn reach(route: String, variable: String, sources: &Sources<'_>) -> Reach {
    if (sources.set)(&variable) {
        let source = Source::Environment;
        return Reach::Reached {
            route,
            variable,
            source,
        };
    }
    let files: Vec<PathBuf> = std::iter::once(sources.repo.join(".env"))
        .chain(sources.home.as_ref().ok().map(|home| home.join(".env")))
        .collect();
    if let Some(file) = files.iter().find(|file| present(file)) {
        let why = Untold::Dotenv(file.clone());
        return Reach::Untold {
            route,
            variable,
            why,
        };
    }
    let service = match &sources.home {
        Ok(home) => service(home),
        Err(cause) => Service::Unhomed(cause.clone()),
    };
    match service {
        Service::On(patch) => Reach::Reached {
            route,
            variable,
            source: Source::Service(patch),
        },
        Service::Off(off) => Reach::Unreached {
            route,
            variable,
            files,
            off: Ok(off),
        },
        Service::Unhomed(cause) => Reach::Unreached {
            route,
            variable,
            files,
            off: Err(cause),
        },
        Service::Untold(why) => Reach::Untold {
            route,
            variable,
            why,
        },
    }
}

/// What a seat's patch layers leave of dsh's credentials service.
enum Service {
    /// The layer at this path re-enables it last.
    On(PathBuf),
    Off(Off),
    /// No dsh home was resolved to hold a layer.
    Unhomed(CompositeError),
    /// A layer cannot be read, so neither is claimed.
    Untold(Untold),
}

/// What `home`'s patch layers leave of dsh's credentials service, each
/// read in dsh's order and the last write winning.
fn service(home: &Path) -> Service {
    let profile = home
        .join("profiles")
        .join("headless")
        .join("cordis.patch.yml");
    let mut state = Service::Off(Off::Shipped(profile.clone()));
    for layer in [profile, home.join("cordis.patch.yml")] {
        state = match patch::layer(&layer) {
            Err(unread) => return Service::Untold(Untold::Patch(layer, unread)),
            Ok(Some(Write::Enables)) => Service::On(layer),
            Ok(Some(Write::Disables)) => Service::Off(Off::Layer(layer)),
            Ok(None) => state,
        };
    }
    state
}

#[cfg(test)]
mod tests;
