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
//!   writes no `credentials` row. A layer is reached by a walk from the
//!   dsh home that follows no link below it and read by [`patch`]'s
//!   reader, and one it cannot read answers "can't tell", never ok.
//!
//! Nothing is logged, journaled or passed to a seat, and every line names
//! the variable and never its value.
//!
//! Which home serves each dsh seat is modelled from the folder's compile
//! ([`home`]; the served-home model, #717's ruled follow-up): the isolation
//! intent the seat's launch carries, the one dispatch seals beside it
//! (`brokkr_runtime::engine::sealed_isolation`), and then brokkr-protocol's
//! own served-home decision (`brokkr_protocol::dsh_served`), which the
//! launch is served by, through the admission the driver applies. A seat
//! served from the operator's own dsh home, the one `DshSeams::resolve`
//! resolves, is asked of every source above. One served from an
//! engine-only home (decision 0065 slice two) reads a key from the
//! launching environment alone, so a key only the operator's home gives
//! is no key there, and a `<repo>/.env` refuses its launch
//! (brokkr-protocol's `WorkdirCredentials`, looked up by
//! `brokkr_protocol::holds_dotenv`). A seat the decision refuses is named
//! with its refusal.
//!
//! Where the folder did not compile, no seat is modelled: every route the
//! adapter pins carries [`Line::Served`], which says what that route's
//! check asked ([`Asked`]; nothing, for a route that names no variable)
//! and that the serving home can't be told; a key only the operator's home
//! gives warns rather than reads ok, and a `<repo>/.env` beside the
//! folder's dsh seats warns as [`Line::Workdir`]. A seat whose launch the
//! compile does not carry (an inline dsh site, or a candidate pinning no
//! `--model`) is named by its site in [`Line::Unmodelled`], every other
//! seat keeps its home, and each route no read seat rides is checked as
//! an unmodelled folder's is, with its caveat.
//!
//! [`patch`]'s second, narrower reader stays, the bend of decision 0071
//! ruling 5 the operator accepted on 2026-10-25, until brokkr-protocol's
//! route-overlay reader is reachable with the same component-wise link
//! guard. When the secrets store is hidden inside every workspace box
//! (U8h), the check could also ask whether `<repo>/.forge/secrets.env` is
//! present by looking its name up, and whether the seat binds the route's
//! variable by name, and never read the store.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_protocol::adapters::{split_route, AdapterKind, CompositeError, DshSeams};
use brokkr_protocol::holds_dotenv;
use brokkr_runtime::agents::Adapter;
use brokkr_runtime::bundle::Bundle;
use brokkr_runtime::Adapters;

use crate::render::Safe;
pub(crate) use home::Home;
use patch::{Layer, Unread, Write, SERVICE};

mod home;
mod patch;

/// The name of each patch layer dsh composes, in a profile and the home.
const PATCH: &str = "cordis.patch.yml";

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
    /// The route's seats are served from an engine-only home, which reads
    /// a key from the launching environment alone, and `variable` is not
    /// set there.
    Unset { route: String, variable: String },
}

impl Reach {
    /// The route this answer is for.
    fn route(&self) -> &str {
        let (Reach::Reached { route, .. }
        | Reach::Untold { route, .. }
        | Reach::Unreached { route, .. }
        | Reach::Undeclared { route }
        | Reach::Unset { route, .. }) = self;
        route
    }

    /// The line's name, on both surfaces.
    pub(crate) fn what(&self) -> String {
        format!("dsh key {}", Safe::new(self.route()).as_str())
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
            Reach::Unset { variable, .. } => {
                let variable = Safe::new(variable);
                let variable = variable.as_str();
                format!(
                    "dsh reaches no {variable}: Brokkr serves this route from its own engine-only \
                     home (decision 0065), which reads a key from the launching environment \
                     alone, and {variable} is not set there; export {variable} where brokkr runs"
                )
            }
        }
    }
}

/// One line of the check, as both surfaces print it: a route's answer,
/// the home that serves its seats or the caveat that it can't be told, or
/// the workdir's `.env`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Line {
    Key(Reach),
    /// The homes `seats`, by site, are served from on `route`, as the
    /// protocol's served-home decision rules it for each ([`home`]): `cold`
    /// at a cold launch, offered no rejoin, and `resumed` at a resumed one,
    /// the replacement a rejoin offer launches.
    Home {
        route: String,
        seats: Vec<String>,
        cold: Home,
        resumed: Home,
    },
    /// The sites of the folder's dsh seats whose launch can't be read from
    /// its compile, so the home that serves them can't be told.
    Unmodelled(Vec<String>),
    /// Whether the route's seats are served from the operator's home, or
    /// from an engine-only one, can't be told, because no seat riding it
    /// was modelled; `asked` is what the route's check asked, none
    /// where the route names no variable.
    Served {
        route: String,
        asked: Option<Asked>,
    },
    /// A `.env` in the repository the folder's dsh seats run in, present
    /// and never opened, which an engine-only home refuses to launch with.
    Workdir(PathBuf),
}

/// What one route's check asked, in dsh's order, each source only where
/// none before it answered.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Asked {
    /// The variable asked of the launching environment.
    pub(crate) variable: String,
    /// Each `.env` name looked up after it, never opened.
    pub(crate) files: Vec<PathBuf>,
    /// What the check did at each patch layer after those.
    pub(crate) patches: Vec<AtLayer>,
}

/// What one route's check did at one patch layer, by its path.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AtLayer {
    /// No file is there, or no directory on the way to it.
    Absent(PathBuf),
    /// Read for its `credentials` row.
    Read(PathBuf),
    /// Present and not read past, for this reason.
    Refused(PathBuf, Unread),
}

impl AtLayer {
    /// What the check did at the layer at `path`, which `patch::layer`
    /// `found`.
    fn of(path: PathBuf, found: &Result<Layer, Unread>) -> AtLayer {
        match found {
            Ok(Layer::Absent) => AtLayer::Absent(path),
            Ok(Layer::Read(_)) => AtLayer::Read(path),
            Err(unread) => AtLayer::Refused(path, unread.clone()),
        }
    }

    /// The layer's clause in the served-home caveat.
    fn words(&self) -> String {
        match self {
            AtLayer::Absent(path) => format!("found no {}", path.display()),
            AtLayer::Read(path) => format!("read {} for dsh's credentials row", path.display()),
            AtLayer::Refused(path, unread) => format!("refused {}: {unread}", path.display()),
        }
    }
}

impl Line {
    /// The line's name, on both surfaces.
    pub(crate) fn what(&self) -> String {
        match self {
            Line::Key(reach) => reach.what(),
            Line::Home { route, .. } | Line::Served { route, .. } => {
                format!("dsh home {}", Safe::new(route).as_str())
            }
            Line::Unmodelled(_) => "dsh home".to_string(),
            Line::Workdir(_) => "dsh workdir".to_string(),
        }
    }

    /// The line's words, on both surfaces.
    pub(crate) fn detail(&self) -> String {
        match self {
            Line::Key(reach) => reach.detail(),
            Line::Home {
                seats,
                cold,
                resumed,
                ..
            } => cold.words(resumed, seats),
            Line::Unmodelled(seats) => home::unmodelled(seats),
            Line::Served { asked, .. } => served(asked.as_ref()),
            Line::Workdir(file) => format!(
                "{} is present, unread: if Brokkr serves a dsh seat from its own engine-only home \
                 (decision 0065), it refuses that launch, since dsh would read the file for a key",
                file.display()
            ),
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

/// The served-home caveat's words: only what the route's check `asked`,
/// or that nothing was where the route names no variable, and that the
/// home serving the route can't yet be told, where no seat riding it was
/// modelled.
fn served(asked: Option<&Asked>) -> String {
    let unknown = "Brokkr can't yet tell whether it serves this route from its own engine-only \
                   home (decision 0065), which reads neither patch nor the home's .env";
    let Some(asked) = asked else {
        return format!(
            "this check asked nothing of the operator's dsh home, since no variable is named \
             for this route; {unknown}"
        );
    };
    let environment = format!(
        "whether {} is set where brokkr runs",
        Safe::new(&asked.variable).as_str()
    );
    if asked.files.is_empty() {
        return format!(
            "this check asked only {environment}, and nothing of the operator's dsh home; \
             {unknown}"
        );
    }
    let files: Vec<String> = asked
        .files
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    let patches: String = asked
        .patches
        .iter()
        .map(|layer| format!(", then {}", layer.words()))
        .collect();
    format!(
        "this check asked {environment}, then looked up {} by name, unread{patches}; {unknown}, \
         so past the environment this check can't confirm the key reaches a seat served there",
        files.join(" and ")
    )
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

/// The folder's dsh key check, for `repo` and the dsh home dsh's own seam
/// resolves: each dsh seat of `compiled`, the folder's compile, grouped by
/// the route it rides and the home that serves it, beside those whose
/// launch can't be read ([`home::modelled`]); where the folder did not
/// compile, each route the folder's dsh adapter under `adapters` pins with
/// its caveats ([`with_caveats`]). None where the folder hires no dsh, or
/// its adapters do not load (doctor's adapters line already says so).
pub(crate) fn of_folder(adapters: &Path, repo: &Path, compiled: Option<&Bundle>) -> Vec<Line> {
    let sources = Sources::served(repo, DshSeams::resolve());
    match compiled.map(home::seats) {
        Some(seats) => with_dsh(adapters, |dsh| home::modelled(dsh, &seats, &sources)),
        None => with_caveats(in_folder(adapters, &sources), repo),
    }
}

/// Each of `answers` followed by its [`Line::Served`] caveat, then a `.env`
/// present in `repo`, looked up by name alone, as [`Line::Workdir`]; none
/// where no route was answered.
fn with_caveats(answers: Vec<(Reach, Option<Asked>)>, repo: &Path) -> Vec<Line> {
    if answers.is_empty() {
        return Vec::new();
    }
    let workdir = holds_dotenv(repo).then(|| repo.join(".env"));
    let mut lines = Vec::new();
    for (reach, asked) in answers {
        let route = reach.route().to_string();
        lines.push(Line::Key(reach));
        lines.push(Line::Served { route, asked });
    }
    lines.extend(workdir.map(Line::Workdir));
    lines
}

/// [`of_folder`]'s unmodelled check over injected sources: each route's
/// answer and what its check asked.
pub(crate) fn in_folder(adapters: &Path, sources: &Sources<'_>) -> Vec<(Reach, Option<Asked>)> {
    with_dsh(adapters, |dsh| reaches(dsh, sources))
}

/// What `answer` says of the folder's dsh adapter under `adapters`;
/// nothing where the folder hires no dsh, or its adapters do not load.
fn with_dsh<T>(adapters: &Path, answer: impl FnOnce(&Adapter) -> Vec<T>) -> Vec<T> {
    Adapters::load(adapters)
        .map(|loaded| loaded.adapter("dsh").map_or_else(Vec::new, answer))
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
    // process's environment, so its first byte is readable. brokkr-cli's
    // production code never writes its own environment, and a test writes
    // it only through `EnvGuard` (tests/support/env_guard.rs), so no write
    // can move the value while it is read.
    unsafe {
        let value = getenv(name.as_ptr());
        !value.is_null() && *value != 0
    }
}

/// The route dsh serves the model `id` on, and the variable it reads
/// there: `adapter`'s `credentials` row, else dsh's default key for a bare
/// id, which rides dsh's default route.
fn keyed<'a>(adapter: &'a Adapter, id: &'a str) -> (&'a str, Option<&'a str>) {
    let route = AdapterKind::Dsh.served_route(id).unwrap_or(id);
    let declared = adapter.credentials.get(route).map(String::as_str);
    (
        route,
        declared.or(split_route(id).0.is_none().then_some(DEFAULT_KEY)),
    )
}

/// Each route `adapter`'s models are served on, once, with the variable
/// it reads ([`keyed`]).
fn routes(adapter: &Adapter) -> BTreeMap<&str, Option<&str>> {
    let mut routes: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    for id in adapter.models.values() {
        let (route, variable) = keyed(adapter, id);
        let entry = routes.entry(route).or_insert(variable);
        *entry = entry.or(variable);
    }
    routes
}

/// Each of `adapter`'s routes, answered against `sources`, with what its
/// check asked.
pub(crate) fn reaches(adapter: &Adapter, sources: &Sources<'_>) -> Vec<(Reach, Option<Asked>)> {
    routes(adapter)
        .into_iter()
        .map(|(route, variable)| match variable {
            Some(variable) => {
                let (reach, asked) = reach(route.to_string(), variable.to_string(), sources);
                (reach, Some(asked))
            }
            None => {
                let route = route.to_string();
                (Reach::Undeclared { route }, None)
            }
        })
        .collect()
}

/// One route's answer, each source in dsh's order: the environment, each
/// `.env` dsh reads, then the patch layers of the home; and what was asked
/// before a source answered.
fn reach(route: String, variable: String, sources: &Sources<'_>) -> (Reach, Asked) {
    let mut asked = Asked {
        variable: variable.clone(),
        files: Vec::new(),
        patches: Vec::new(),
    };
    if (sources.set)(&variable) {
        let source = Source::Environment;
        let reach = Reach::Reached {
            route,
            variable,
            source,
        };
        return (reach, asked);
    }
    // Each `.env` dsh reads is looked up by name and never opened, so
    // nothing of it reaches Brokkr; one that cannot be looked up counts as
    // present, as brokkr-protocol's own `<workdir>/.env` refusal counts it.
    let dirs: Vec<&Path> = std::iter::once(sources.repo)
        .chain(sources.home.as_deref().ok())
        .collect();
    let files: Vec<PathBuf> = dirs.iter().map(|dir| dir.join(".env")).collect();
    for (dir, file) in dirs.into_iter().zip(&files) {
        asked.files.push(file.clone());
        if holds_dotenv(dir) {
            let why = Untold::Dotenv(file.clone());
            let reach = Reach::Untold {
                route,
                variable,
                why,
            };
            return (reach, asked);
        }
    }
    let service = match &sources.home {
        Ok(home) => service(home, &mut asked.patches),
        Err(cause) => Service::Unhomed(cause.clone()),
    };
    let reach = match service {
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
    };
    (reach, asked)
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
/// asked in dsh's order, what the check did at each pushed onto `asked`,
/// and the last write winning.
fn service(home: &Path, asked: &mut Vec<AtLayer>) -> Service {
    let (profile, own) = (&["profiles", "headless"][..], &[][..]);
    let path = |below: &[&str]| {
        below
            .iter()
            .fold(home.to_path_buf(), |path, name| path.join(name))
    };
    let mut state = Service::Off(Off::Shipped(path(profile).join(PATCH)));
    for below in [profile, own] {
        let (layer, found) = (path(below).join(PATCH), patch::layer(home, below, PATCH));
        asked.push(AtLayer::of(layer.clone(), &found));
        state = match found {
            Err(unread) => return Service::Untold(Untold::Patch(layer, unread)),
            Ok(Layer::Read(Some(Write::Enables))) => Service::On(layer),
            Ok(Layer::Read(Some(Write::Disables))) => Service::Off(Off::Layer(layer)),
            Ok(Layer::Absent | Layer::Read(None)) => state,
        };
    }
    state
}

#[cfg(test)]
mod tests;
