//! The machine's capacity as its operator declares it (decision 0068
//! ruling 3, and ruling 5's data): `forge.host/v1`, `forge.host/v2` or
//! `forge.host/v3`, one file per machine and never a realm's (the
//! operator's ruling of 2026-10-10, option B). A machine has one scratch
//! disk, one set of provider accounts and its own local hardware, whichever
//! realm a run operates, so the tracked realms map carries none of it
//! (#432).
//!
//! The file is found where [`host_file`] says, and read by [`read`] into
//! one of three outcomes that are never confused: nothing at the path is
//! capacity undeclared; a path that holds something that cannot be read (a
//! link that loops or dangles, a directory, a FIFO or a device, a
//! permission refused) is unreadable, never absent (#430's H5) and never
//! waited on; and a file read but not a valid `forge.host/v1`,
//! `forge.host/v2` or `forge.host/v3` refuses, naming the problem and,
//! where one version's rules own it, that version. A ceiling of zero, a
//! negative one and one past `u32` are refused, never read as unlimited,
//! and so is a provider or route written twice: the ceiling the operator
//! reads first is the one admission obeys, or none is. Every number is
//! read from its exact token, never through `f64` (the operator's ruling
//! of 2026-11-04, option A): an integral decimal or exponent notation
//! reads exactly as the schema admits, and a token that is not exactly an
//! integer in range is a shape refusal. The free-space floors are a list
//! of at least one (the operator's ruling of 2026-10-10, option c), each
//! naming exactly one place or absolute path, and none named twice.
//! `forge.host/v3` adds one optional shared-local deadline floor, whose
//! reading and refusals are [`floor`]'s (decision 0104 ruling 1(a)).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, ErrorKind};
use std::num::{NonZeroU32, NonZeroU64};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::capabilities::manifest::present;

mod floor;
mod shape;

pub use floor::{declared_routes, HostRoutes, RouteDecl};
use shape::Integral;

/// A host configuration, read whole: every key known, every ceiling at
/// least one, and the cool-down bounds v2 may declare resolved through
/// their inheritance (decision 0103 ruling 3).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "WrittenHost")]
pub(crate) struct HostConfig {
    /// Read only to refuse another version.
    _schema: Schema,
    pub(crate) providers: BTreeMap<String, Provider>,
    pub(crate) floors: Floors,
    pub(crate) boxed_builds: Ceiling,
    /// `None` where the file declares no cool-downs (a v1 file, or a v2
    /// file without the block), so a rate-limit refusal parks.
    cooldowns: Option<Cooldowns>,
}

/// The file's version, its first key: a file in any other is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum Schema {
    #[serde(rename = "forge.host/v1")]
    V1,
    #[serde(rename = "forge.host/v2")]
    V2,
    #[serde(rename = "forge.host/v3")]
    V3,
}

/// A host configuration as either version writes it, before the
/// cross-field rules only the loader can judge.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenHost {
    #[serde(rename = "schema", deserialize_with = "shape::word")]
    schema: Schema,
    #[serde(deserialize_with = "shape::strict_map")]
    providers: BTreeMap<String, shape::WrittenProvider>,
    floors: Floors,
    #[serde(deserialize_with = "shape::object")]
    boxed_builds: Ceiling,
    #[serde(default, deserialize_with = "shape::present_object")]
    cooldowns: Option<WrittenCooldowns>,
    #[serde(default, deserialize_with = "shape::present_word")]
    containment: Option<Containment>,
}

/// What a host demands of containment (decision 0080 ruling 6):
/// `best-effort` takes the first rung the ladder finds, and `required`
/// refuses a start where none is offered. `required` is 0080's and is not
/// built yet (#472).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Containment {
    BestEffort,
    Required,
}

/// A site's cool-down bounds after inheritance: its own number, else the
/// provider's, else the block's (decision 0103 ruling 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Bounds {
    pub(crate) default_seconds: NonZeroU32,
    pub(crate) max_seconds: NonZeroU32,
}

/// The longest cool-down any site may declare: the shipped seats' longest
/// attempt deadline, under the fleet's staleness bound (decision 0103
/// ruling 3).
const MAX_COOLDOWN_SECONDS: u32 = 7_200;

/// The cool-downs a v2 file declares: the block's numbers, which every
/// declared provider inherits, and each provider's and route's own.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cooldowns {
    default_seconds: NonZeroU32,
    max_seconds: NonZeroU32,
    providers: BTreeMap<String, ProviderCooldowns>,
}

/// One provider's cool-down overrides and its routes'.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ProviderCooldowns {
    bounds: Overrides,
    routes: BTreeMap<String, Overrides>,
}

/// Numbers a provider or route may leave out to inherit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Overrides {
    default_seconds: Option<NonZeroU32>,
    max_seconds: Option<NonZeroU32>,
}

/// A cool-downs block as the file writes it, before its values are
/// checked.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenCooldowns {
    default_seconds: Integral<u32>,
    max_seconds: Integral<u32>,
    #[serde(default, deserialize_with = "shape::strict_map")]
    providers: BTreeMap<String, WrittenProviderCooldowns>,
}

/// One provider's written overrides and its routes'.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenProviderCooldowns {
    #[serde(default, deserialize_with = "present")]
    default_seconds: Option<Integral<u32>>,
    #[serde(default, deserialize_with = "present")]
    max_seconds: Option<Integral<u32>>,
    #[serde(default, deserialize_with = "shape::strict_map")]
    routes: BTreeMap<String, WrittenOverrides>,
}

/// Numbers a provider or route may write, each optional so the missing
/// one inherits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenOverrides {
    #[serde(default, deserialize_with = "present")]
    default_seconds: Option<Integral<u32>>,
    #[serde(default, deserialize_with = "present")]
    max_seconds: Option<Integral<u32>>,
}

/// A site a cool-down refusal names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Site<'a> {
    Block,
    Provider(&'a str),
    Route { provider: &'a str, route: &'a str },
}

impl fmt::Display for Site<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Site::Block => f.write_str("the cooldowns block"),
            Site::Provider(provider) => write!(f, "provider '{provider}'"),
            Site::Route { provider, route } => {
                write!(f, "route '{route}' of provider '{provider}'")
            }
        }
    }
}

impl TryFrom<WrittenCooldowns> for Cooldowns {
    type Error = Problem;

    fn try_from(written: WrittenCooldowns) -> Result<Cooldowns, Problem> {
        let default_seconds = positive(written.default_seconds.0, Site::Block, "default_seconds")?;
        let max_seconds = capped(written.max_seconds.0, Site::Block)?;
        let mut providers = BTreeMap::new();
        for (name, provider) in written.providers {
            let site = Site::Provider(&name);
            let bounds = Overrides::read(
                WrittenOverrides {
                    default_seconds: provider.default_seconds,
                    max_seconds: provider.max_seconds,
                },
                site,
            )?;
            let mut routes = BTreeMap::new();
            for (route, overrides) in provider.routes {
                let site = Site::Route {
                    provider: &name,
                    route: &route,
                };
                let overrides = Overrides::read(overrides, site)?;
                routes.insert(route, overrides);
            }
            providers.insert(name, ProviderCooldowns { bounds, routes });
        }
        Ok(Cooldowns {
            default_seconds,
            max_seconds,
            providers,
        })
    }
}

impl Overrides {
    /// One written override, each number it leaves out inheriting.
    fn read(written: WrittenOverrides, site: Site<'_>) -> Result<Overrides, Problem> {
        let default_seconds = written
            .default_seconds
            .map(|value| positive(value.0, site, "default_seconds"))
            .transpose()?;
        let max_seconds = written
            .max_seconds
            .map(|value| capped(value.0, site))
            .transpose()?;
        Ok(Overrides {
            default_seconds,
            max_seconds,
        })
    }
}

/// One written number as a positive integer, or the site and field it
/// names: zero is never read as no wait.
fn positive(value: u32, site: Site<'_>, field: &'static str) -> Result<NonZeroU32, Problem> {
    NonZeroU32::new(value).ok_or_else(|| Problem::Zero {
        site: site.to_string(),
        field,
    })
}

/// A written maximum as a positive integer no greater than two hours, or
/// the site it names.
fn capped(value: u32, site: Site<'_>) -> Result<NonZeroU32, Problem> {
    let max = positive(value, site, "max_seconds")?;
    if max.get() > MAX_COOLDOWN_SECONDS {
        return Err(Problem::AboveMaximum {
            site: site.to_string(),
            value,
            maximum: MAX_COOLDOWN_SECONDS,
        });
    }
    Ok(max)
}

impl HostConfig {
    /// The bounds one site obeys, after a route's own number, else its
    /// provider's, else the block's (decision 0103 ruling 3). `None` means
    /// the file declares no cool-downs, so a rate-limit refusal parks as
    /// today.
    pub(crate) fn cooldown(&self, provider: &str, route: Option<&str>) -> Option<Bounds> {
        let cooldowns = self.cooldowns.as_ref()?;
        let declared = cooldowns.providers.get(provider);
        let provider_default = declared
            .and_then(|provider| provider.bounds.default_seconds)
            .unwrap_or(cooldowns.default_seconds);
        let provider_max = declared
            .and_then(|provider| provider.bounds.max_seconds)
            .unwrap_or(cooldowns.max_seconds);
        let overrides = route.and_then(|route| declared?.routes.get(route));
        Some(Bounds {
            default_seconds: overrides
                .and_then(|bounds| bounds.default_seconds)
                .unwrap_or(provider_default),
            max_seconds: overrides
                .and_then(|bounds| bounds.max_seconds)
                .unwrap_or(provider_max),
        })
    }

    /// Refuse a cool-downs block that names a site the capacity block does
    /// not declare, or whose inherited default passes its inherited
    /// maximum. Each value's positivity and the 7200-second cap were
    /// checked when it was read.
    fn validate_cooldowns(&self, cooldowns: &Cooldowns) -> Result<(), Problem> {
        check_bounds(
            cooldowns.default_seconds,
            cooldowns.max_seconds,
            Site::Block,
        )?;
        for (name, provider) in &cooldowns.providers {
            let declared = self
                .providers
                .get(name)
                .ok_or_else(|| Problem::ProviderUndeclared(name.clone()))?;
            if provider.bounds.default_seconds.is_some() || provider.bounds.max_seconds.is_some() {
                self.check_site(name, None)?;
            }
            for route in provider.routes.keys() {
                if !declared.routes.contains_key(route) {
                    return Err(Problem::RouteUndeclared {
                        provider: name.clone(),
                        route: route.clone(),
                    });
                }
                self.check_site(name, Some(route))?;
            }
        }
        Ok(())
    }

    /// A provider's or a route's inherited bounds, refused where its
    /// default passes its maximum. The block itself is checked directly.
    fn check_site(&self, provider: &str, route: Option<&str>) -> Result<(), Problem> {
        let site = route.map_or(Site::Provider(provider), |route| Site::Route {
            provider,
            route,
        });
        let bounds = self
            .cooldown(provider, route)
            .expect("a cooldown site is inside a cooldowns block");
        check_bounds(bounds.default_seconds, bounds.max_seconds, site)
    }
}

/// A site's default no greater than its maximum.
fn check_bounds(
    default_seconds: NonZeroU32,
    max_seconds: NonZeroU32,
    site: Site<'_>,
) -> Result<(), Problem> {
    if default_seconds > max_seconds {
        return Err(Problem::DefaultAboveMaximum {
            site: site.to_string(),
            default: default_seconds.get(),
            max: max_seconds.get(),
        });
    }
    Ok(())
}

impl TryFrom<WrittenHost> for HostConfig {
    type Error = Problem;

    fn try_from(written: WrittenHost) -> Result<HostConfig, Problem> {
        match written.schema {
            Schema::V1 => {
                if written.cooldowns.is_some() {
                    return Err(Problem::NotV2("cooldowns"));
                }
                if written.containment.is_some() {
                    return Err(Problem::NotV2("containment"));
                }
                if let Some(problem) = floor::v3_only("forge.host/v1", &written.providers) {
                    return Err(problem);
                }
            }
            Schema::V2 => {
                if written.containment == Some(Containment::Required) {
                    return Err(Problem::ContainmentRequired);
                }
                if let Some(problem) = floor::v3_only("forge.host/v2", &written.providers) {
                    return Err(problem);
                }
            }
            Schema::V3 => {
                if written.containment == Some(Containment::Required) {
                    return Err(Problem::ContainmentRequired);
                }
            }
        }
        let cooldowns = written.cooldowns.map(Cooldowns::try_from).transpose()?;
        let mut providers = BTreeMap::new();
        for (name, provider) in written.providers {
            let mut routes = BTreeMap::new();
            for (route, written) in provider.routes {
                let timeout_floor_seconds =
                    floor::read(written.timeout_floor_seconds, written.class, &name, &route)?;
                routes.insert(
                    route,
                    Route {
                        ceiling: written.ceiling,
                        class: written.class,
                        timeout_floor_seconds,
                    },
                );
            }
            providers.insert(
                name,
                Provider {
                    ceiling: provider.ceiling,
                    routes,
                },
            );
        }
        let config = HostConfig {
            _schema: written.schema,
            providers,
            floors: written.floors,
            boxed_builds: written.boxed_builds,
            cooldowns,
        };
        if let Some(cooldowns) = &config.cooldowns {
            config.validate_cooldowns(cooldowns)?;
        }
        Ok(config)
    }
}

/// One provider's ceiling, and the routes of it the machine declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Provider {
    pub(crate) ceiling: NonZeroU32,
    pub(crate) routes: BTreeMap<String, Route>,
}

/// One route's ceiling, class and, where it declares one, its deadline
/// floor (decision 0104 ruling 1(a)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Route {
    pub(crate) ceiling: NonZeroU32,
    pub(crate) class: RouteClass,
    pub(crate) timeout_floor_seconds: Option<NonZeroU32>,
}

/// Where a route's model runs (decision 0068 ruling 5): behind a cloud
/// account, counted against its provider's ceiling and its own, or on
/// local hardware the machine shares with other work, counted against its
/// own ceiling in place of its provider's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteClass {
    Cloud,
    SharedLocal,
}

impl fmt::Display for RouteClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RouteClass::Cloud => "cloud",
            RouteClass::SharedLocal => "shared-local",
        })
    }
}

/// The free-space floors every entry must find held, in the file's order
/// (the operator's ruling of 2026-10-10, option c): at least one, and no
/// place or path named twice, since two floors on one would leave the
/// operator reading one number while admission obeys another.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Vec<Floor>")]
pub(crate) struct Floors(pub(crate) Vec<Floor>);

impl TryFrom<Vec<Floor>> for Floors {
    type Error = Unfloored;

    fn try_from(floors: Vec<Floor>) -> Result<Floors, Unfloored> {
        if floors.is_empty() {
            return Err(Unfloored::Empty);
        }
        for (index, floor) in floors.iter().enumerate() {
            if floors[..index].iter().any(|before| before.at == floor.at) {
                return Err(Unfloored::Repeated(floor.at.clone()));
            }
        }
        Ok(Floors(floors))
    }
}

/// One free-space floor: the bytes the filesystem it names must hold free
/// for another run to start.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "shape::Object<WrittenFloor>")]
pub(crate) struct Floor {
    pub(crate) at: At,
    pub(crate) floor_bytes: NonZeroU64,
}

/// A floor as the file writes it: exactly one of a path and a place, each
/// left out rather than `null`, as the contract says.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WrittenFloor {
    #[serde(deserialize_with = "shape::positive_u64")]
    floor_bytes: NonZeroU64,
    #[serde(default, deserialize_with = "present")]
    path: Option<Absolute>,
    #[serde(default, deserialize_with = "shape::present_word")]
    place: Option<Place>,
}

impl TryFrom<shape::Object<WrittenFloor>> for Floor {
    type Error = Unfloored;

    fn try_from(shape::Object(written): shape::Object<WrittenFloor>) -> Result<Floor, Unfloored> {
        let at = match (written.path, written.place) {
            (Some(path), None) => At::Path(path),
            (None, Some(place)) => At::Place(place),
            (Some(_), Some(_)) => return Err(Unfloored::Both),
            (None, None) => return Err(Unfloored::Neither),
        };
        Ok(Floor {
            at,
            floor_bytes: written.floor_bytes,
        })
    }
}

/// What a floor measures: a place admission resolves, or an absolute path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum At {
    Place(Place),
    Path(Absolute),
}

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            At::Place(place) => write!(f, "place '{place}'"),
            At::Path(path) => write!(f, "path '{}'", path.0.display()),
        }
    }
}

/// A place a floor names, resolved where admission measures: the seat
/// scratch of the repository the queued run operates
/// (`engine::SEAT_SCRATCH`), or the host's temp directory, where
/// boxed seats' private `/tmp` and `HOME` and the oneshot and probe
/// scratch are made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Place {
    SeatScratch,
    Temp,
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Place::SeatScratch => "seat-scratch",
            Place::Temp => "temp",
        })
    }
}

/// Why the floors a file declares are not a list admission can obey.
#[derive(Debug, Error)]
pub(crate) enum Unfloored {
    #[error("the floor path {} is not absolute", .0.display())]
    Relative(PathBuf),
    #[error("a floor names both a path and a place")]
    Both,
    #[error("a floor names neither a path nor a place")]
    Neither,
    #[error("no floor is declared")]
    Empty,
    #[error("the floor at {0} is written twice")]
    Repeated(At),
}

/// A ceiling on its own: the boxed builds'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ceiling {
    #[serde(deserialize_with = "shape::positive_u32")]
    pub(crate) ceiling: NonZeroU32,
}

/// An absolute path: a relative one would be measured against whatever
/// directory admission runs in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "PathBuf")]
pub(crate) struct Absolute(pub(crate) PathBuf);

impl TryFrom<PathBuf> for Absolute {
    type Error = Unfloored;

    fn try_from(path: PathBuf) -> Result<Absolute, Unfloored> {
        match path.is_absolute() {
            true => Ok(Absolute(path)),
            false => Err(Unfloored::Relative(path)),
        }
    }
}

/// What the host configuration's path holds.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Hosting {
    /// Nothing: capacity is undeclared.
    Absent,
    /// Something that cannot be read, and the kind of error that said so.
    Unreadable(ErrorKind),
    Declared(HostConfig),
}

/// Why no host configuration can be read at all.
#[derive(Debug, Error)]
pub enum HostError {
    #[error(
        "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME names an absolute \
         directory"
    )]
    Unplaced,
    #[error(
        "the host configuration at {} is not a valid {} file",
        path.display(),
        source.contract()
    )]
    Invalid {
        path: PathBuf,
        #[source]
        source: Problem,
    },
}

/// What makes a read host configuration invalid.
#[derive(Debug, Error)]
pub enum Problem {
    /// Not JSON, or not the contract's shape.
    #[error(transparent)]
    Shape(#[from] serde_json::Error),
    /// A key written twice in one object, in the strict reader's words.
    #[error("{0}")]
    Repeated(String),
    /// A key only `forge.host/v2` declares, in a v1 file.
    #[error("forge.host/v1 declares {0}; only forge.host/v2 does")]
    NotV2(&'static str),
    /// A key only `forge.host/v3` declares, in a v1 or v2 file.
    #[error("route '{2}' of provider '{1}' has a v3-only timeout floor; only forge.host/v3 does")]
    NotV3(&'static str, String, String),
    /// A floor the loader cannot obey, in the floor module's own words.
    #[error(transparent)]
    Floor(#[from] floor::Problem),
    /// Decision 0080's `required`, which is not built yet (#472).
    #[error("containment required is decision 0080's, not built yet (#472)")]
    ContainmentRequired,
    /// A cool-down number of zero, never read as no wait.
    #[error("the cool-down {field} of {site} is zero")]
    Zero { site: String, field: &'static str },
    /// A maximum past the shipped seats' longest attempt deadline.
    #[error("the cool-down max_seconds of {site} is {value}, past the {maximum}-second maximum")]
    AboveMaximum {
        site: String,
        value: u32,
        maximum: u32,
    },
    /// A default longer than the maximum that bounds it.
    #[error("the cool-down default_seconds of {site} is {default}, above its max_seconds {max}")]
    DefaultAboveMaximum {
        site: String,
        default: u32,
        max: u32,
    },
    /// A provider cool-down the capacity block does not declare.
    #[error("the cool-down provider '{0}' is not declared under providers")]
    ProviderUndeclared(String),
    /// A route cool-down the provider's capacity block does not declare.
    #[error(
        "the cool-down route '{route}' of provider '{provider}' is not declared under providers"
    )]
    RouteUndeclared { provider: String, route: String },
}

impl Problem {
    /// The contract a refusal names: the problem's own version where only
    /// one version can have it, and the version-neutral name where either
    /// version's shape may be wrong.
    fn contract(&self) -> &'static str {
        match self {
            Problem::Shape(_) | Problem::Repeated(_) => "host configuration",
            Problem::NotV2(_) => "forge.host/v1",
            Problem::NotV3(declared, ..) => declared,
            Problem::ContainmentRequired
            | Problem::Zero { .. }
            | Problem::AboveMaximum { .. }
            | Problem::DefaultAboveMaximum { .. }
            | Problem::ProviderUndeclared(_)
            | Problem::RouteUndeclared { .. } => "forge.host/v2",
            Problem::Floor(_) => "forge.host/v3",
        }
    }
}

/// Where the machine's host configuration is: `host.json` in the
/// machine's configuration directory ([`config_dir`]).
pub fn host_file(xdg: Option<OsString>, home: Option<OsString>) -> Result<PathBuf, HostError> {
    Ok(config_dir(xdg, home)?.join("host.json"))
}

/// The machine's configuration directory: `brokkr` under
/// `$XDG_CONFIG_HOME`, else under `$HOME/.config`, on Linux and macOS
/// alike. A value that is not an absolute directory is passed over, as
/// the XDG base directory specification asks of a relative one. The one
/// root of `host.json` and of the personal shelf (#697).
pub(crate) fn config_dir(
    xdg: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, HostError> {
    let absolute = |dir: Option<OsString>| dir.map(PathBuf::from).filter(|dir| dir.is_absolute());
    absolute(xdg)
        .or_else(|| absolute(home).map(|home| home.join(".config")))
        .map(|root| root.join("brokkr"))
        .ok_or(HostError::Unplaced)
}

/// Read the host configuration at `file`. Only a path with nothing at it
/// is absent; whatever is there and cannot be read is unreadable, and so
/// is a path that cannot be reached because a directory on it is a link
/// that loops or dangles.
pub(crate) fn read(file: &Path) -> Result<Hosting, HostError> {
    if let Err(absent) = std::fs::symlink_metadata(file) {
        if absent.kind() == ErrorKind::NotFound {
            return Ok(unreached(file).map_or(Hosting::Absent, Hosting::Unreadable));
        }
    }
    let bytes = match regular(file) {
        Ok(bytes) => bytes,
        Err(unread) => return Ok(Hosting::Unreadable(unread.kind())),
    };
    config(&bytes)
        .map(Hosting::Declared)
        .map_err(|source| HostError::Invalid {
            path: file.to_path_buf(),
            source,
        })
}

/// The bytes of the regular file at `file`, opened non-blocking so a FIFO
/// at the path, or a link to one, is refused by its handle's kind and
/// never waited on; a directory is refused as one, and a device or socket
/// as unsupported, so nothing is read without end.
fn regular(file: &Path) -> io::Result<Vec<u8>> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let mut held = std::fs::File::from(rustix::fs::open(file, flags, Mode::empty())?);
    let kind = held.metadata()?.file_type();
    if kind.is_dir() {
        return Err(ErrorKind::IsADirectory.into());
    }
    if !kind.is_file() {
        return Err(ErrorKind::Unsupported.into());
    }
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut held, &mut bytes)?;
    Ok(bytes)
}

/// A host configuration's bytes, read as the contract's shape and then
/// through the house's strict reader, which refuses a key written twice
/// in one object where serde's map keeps the last copy and says nothing.
/// The shape is judged first, so its refusal keeps the parser's place in
/// the file; bytes it admits are UTF-8, so the lossy text is the file.
fn config(bytes: &[u8]) -> Result<HostConfig, Problem> {
    let shape::Object(written) = serde_json::from_slice::<shape::Object<WrittenHost>>(bytes)?;
    brokkr_core::canonical::parse_strict(&String::from_utf8_lossy(bytes))
        .map_err(Problem::Repeated)?;
    HostConfig::try_from(written)
}

/// Why the deepest component of `file`'s path that is there cannot be
/// followed, where it is a link that loops or dangles; `None` where it is
/// a directory below which the rest of the path is simply absent.
fn unreached(file: &Path) -> Option<ErrorKind> {
    let there = (file.ancestors().skip(1)).find(|dir| std::fs::symlink_metadata(dir).is_ok());
    there
        .and_then(|dir| std::fs::metadata(dir).err())
        .map(|error| error.kind())
}

/// The bytes free to an unprivileged writer on the filesystem holding
/// `path`, or that would hold it once made: that of its deepest component
/// that is there, since a seat's scratch tree is made when its attempt
/// starts. Only a component with nothing at it is passed over: one that
/// is a link that dangles or loops is there, and cannot be measured,
/// never measured on its parent's filesystem. The free-space probe
/// production admission measures with.
pub fn free_bytes(path: &Path) -> io::Result<u64> {
    let there = (path.ancestors()).find_map(|at| match std::fs::symlink_metadata(at) {
        Err(absent) if absent.kind() == ErrorKind::NotFound => None,
        reached => Some(reached.map(|_| at)),
    });
    let stat = rustix::fs::statvfs(there.unwrap_or(Ok(path))?)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

#[cfg(test)]
mod tests;
