//! Decision 0104 rulings 1(a) and 4: a shared-local route in
//! `forge.host/v3` may declare `timeout_floor_seconds`, a whole number
//! from 1 to 7200 that an attempt launching a candidate on the route runs
//! under, at the larger of it and the seat's own `timeout_seconds`. The
//! floor never shortens an authored limit, and `brokkr doctor` reads the
//! same declarations here to print them.
//!
//! The number is read from its exact token, never through `f64` (the
//! operator's ruling of 2026-11-04, option A), and the class rule is
//! judged here because JSON Schema cannot compare a route's `class` to
//! whether it holds the key. A v1 or v2 file that writes the key at all
//! is refused by its own line, before this module reads a value.

use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::num::NonZeroU32;
use std::path::Path;

use serde::Deserialize;
use serde_json::value::RawValue;
use thiserror::Error;

use super::shape::{Integral, Refusal};
use super::{read as read_host, HostConfig, HostError, Hosting, Provider, Route, RouteClass};

/// The longest floor a route may declare: the fleet's attempt bound, the
/// same 7200 `brokkr-view`'s `ATTEMPT_DEADLINE_SECONDS` states, kept here
/// because the floor may not outrun the staleness bound built on it
/// (decision 0104 ruling 1(a)).
pub(super) const MAX_FLOOR_SECONDS: u32 = 7_200;

/// A written floor kept as its exact token until the file's version and
/// the route's class are known. `null` is a value written empty, never an
/// absent declaration, so it reaches [`read`] and is refused.
#[derive(Debug, Clone)]
pub(super) struct Written(pub(super) Box<RawValue>);

impl<'de> Deserialize<'de> for Written {
    fn deserialize<D>(deserializer: D) -> Result<Written, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Box::<RawValue>::deserialize(deserializer).map(Written)
    }
}

/// Why a route's written floor is not one the loader can obey. Each owns
/// its line, so the file, the provider and the route are always named.
#[derive(Debug, Error)]
pub enum Problem {
    /// The key was written on a route that is not shared-local.
    #[error(
        "the timeout floor of route '{route}' of provider '{provider}' is on a {class} route; \
         only a shared-local route declares a floor"
    )]
    OnCloud {
        provider: String,
        route: String,
        class: RouteClass,
    },
    /// A floor outside the whole numbers 1 through 7200.
    #[error(
        "the timeout floor of route '{route}' of provider '{provider}' is {value}; \
         it must be a whole number from 1 to 7200"
    )]
    Range {
        provider: String,
        route: String,
        value: i128,
    },
    /// A floor past the fleet's attempt bound.
    #[error(
        "the timeout floor of route '{route}' of provider '{provider}' is {value}, \
         past the {maximum}-second maximum"
    )]
    AboveMaximum {
        provider: String,
        route: String,
        value: String,
        maximum: u32,
    },
    /// A floor token too small for the reader, which is below the minimum
    /// however exact it is.
    #[error(
        "the timeout floor of route '{route}' of provider '{provider}' is {value}, \
         below the 1-second minimum"
    )]
    BelowMinimum {
        provider: String,
        route: String,
        value: String,
    },
    /// A floor token with a fractional part, or not a number at all.
    #[error("the timeout floor of route '{route}' of provider '{provider}' is not a whole number")]
    NotWhole { provider: String, route: String },
}

/// One route's written floor as a positive number no greater than
/// [`MAX_FLOOR_SECONDS`], or the refusal naming its provider and route.
pub(super) fn read(
    written: Option<Written>,
    class: RouteClass,
    provider: &str,
    route: &str,
) -> Result<Option<NonZeroU32>, Problem> {
    let Some(Written(token)) = written else {
        return Ok(None);
    };
    let text = token.get();
    let value = match text.parse::<Integral<i128>>() {
        Ok(Integral(value)) => value,
        Err(Refusal::NotANumber | Refusal::NotAnInteger) => {
            return Err(Problem::NotWhole {
                provider: provider.to_string(),
                route: route.to_string(),
            })
        }
        // A token past the integer reader's own range is outside the
        // whole numbers this field admits, but its sign still says which
        // end it passed: only a positive one is over the cap.
        Err(Refusal::OutOfRange | Refusal::ExponentPastBound) if text.starts_with('-') => {
            return Err(Problem::BelowMinimum {
                provider: provider.to_string(),
                route: route.to_string(),
                value: text.to_string(),
            })
        }
        Err(Refusal::OutOfRange | Refusal::ExponentPastBound) => {
            return Err(Problem::AboveMaximum {
                provider: provider.to_string(),
                route: route.to_string(),
                value: text.to_string(),
                maximum: MAX_FLOOR_SECONDS,
            })
        }
    };
    if value < 1 {
        return Err(Problem::Range {
            provider: provider.to_string(),
            route: route.to_string(),
            value,
        });
    }
    if value > i128::from(MAX_FLOOR_SECONDS) {
        return Err(Problem::AboveMaximum {
            provider: provider.to_string(),
            route: route.to_string(),
            value: value.to_string(),
            maximum: MAX_FLOOR_SECONDS,
        });
    }
    let floor = NonZeroU32::new(u32::try_from(value).expect("1..=7200 fits u32"))
        .expect("a floor above zero is nonzero");
    match class {
        RouteClass::SharedLocal => Ok(Some(floor)),
        class => Err(Problem::OnCloud {
            provider: provider.to_string(),
            route: route.to_string(),
            class,
        }),
    }
}

/// The first route that writes the v3-only key, with its provider, so a
/// v1 or v2 file refuses it by its own line before this module reads a
/// value. Every key so written refuses; naming the first keeps the line
/// the file, the provider and the route.
pub(super) fn v3_only(
    declared: &'static str,
    providers: &BTreeMap<String, super::shape::WrittenProvider>,
) -> Option<super::Problem> {
    providers.iter().find_map(|(provider, written)| {
        written
            .routes
            .iter()
            .find(|(_, route)| route.timeout_floor_seconds.is_some())
            .map(|(route, _)| super::Problem::NotV3(declared, provider.clone(), route.clone()))
    })
}

/// Every written provider as the loader keeps it, each route's floor read
/// by [`read`] under the route's own class, or the first floor refused.
pub(super) fn providers(
    written: BTreeMap<String, super::shape::WrittenProvider>,
) -> Result<BTreeMap<String, Provider>, Problem> {
    let mut providers = BTreeMap::new();
    for (name, provider) in written {
        let mut routes = BTreeMap::new();
        for (route, written) in provider.routes {
            let timeout_floor_seconds =
                read(written.timeout_floor_seconds, written.class, &name, &route)?;
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
    Ok(providers)
}

impl HostConfig {
    /// The floor a shared-local route declares, where the file is v3 and
    /// the route sets one (decision 0104 ruling 1(a)). `None` is no
    /// declared floor, which is every v1 and v2 file and every v3 route
    /// that leaves the key out.
    pub(crate) fn timeout_floor(&self, provider: &str, route: &str) -> Option<u32> {
        Some(
            self.providers
                .get(provider)?
                .routes
                .get(route)?
                .timeout_floor_seconds?
                .get(),
        )
    }
}

/// One route as a host file declares it: its class and, where it sets
/// one, its deadline floor. `brokkr doctor` prints these and never
/// refuses them (decision 0104 ruling 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteDecl {
    pub class: RouteClass,
    pub timeout_floor_seconds: Option<u32>,
}

/// What a host file says about its routes: nothing at the path, something
/// that cannot be read, or the classes and floors it declares, keyed
/// provider then route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRoutes {
    Absent,
    Unreadable(ErrorKind),
    Declared(BTreeMap<String, BTreeMap<String, RouteDecl>>),
}

/// The routes a host file declares, for `brokkr doctor`, or why the file
/// cannot be read. A machine with no host file declares no route; nothing
/// here refuses.
pub fn declared_routes(file: &Path) -> Result<HostRoutes, HostError> {
    Ok(match read_host(file)? {
        Hosting::Absent => HostRoutes::Absent,
        Hosting::Unreadable(kind) => HostRoutes::Unreadable(kind),
        Hosting::Declared(config) => HostRoutes::Declared(
            config
                .providers
                .into_iter()
                .map(|(provider, declared)| {
                    let routes = declared
                        .routes
                        .into_iter()
                        .map(|(route, route_decl)| {
                            (
                                route,
                                RouteDecl {
                                    class: route_decl.class,
                                    timeout_floor_seconds: route_decl
                                        .timeout_floor_seconds
                                        .map(NonZeroU32::get),
                                },
                            )
                        })
                        .collect();
                    (provider, routes)
                })
                .collect(),
        ),
    })
}
