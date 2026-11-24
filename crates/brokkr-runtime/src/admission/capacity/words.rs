//! A capacity reason's kind, the word scripts read, and its text, the
//! words the operator reads, one arm for each limit admission checks.

use std::fmt;

use super::Capacity;
use crate::admission::cooling;

impl Capacity {
    /// Does only the operator clear it? A host configuration that cannot
    /// be read, and an entry whose seats cannot be measured, wait on a
    /// mend nothing but the operator makes.
    pub(crate) fn holds(&self) -> bool {
        matches!(self, Capacity::Unreadable { .. } | Capacity::Unmeasured(_))
    }

    /// The reason's kind, as scripts read it.
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Capacity::Undeclared(_) => "host_undeclared",
            Capacity::Unreadable { .. } => "host_unreadable",
            Capacity::Unmeasured(_) => "unmeasured",
            Capacity::RunUnmeasured { .. } => "run_unmeasured",
            Capacity::ProviderUndeclared(_) => "provider_undeclared",
            Capacity::RouteUndeclared { .. } => "route_undeclared",
            Capacity::RunsUndeclared(_) => "runs_undeclared",
            Capacity::RunsFull { .. } => "runs_full",
            Capacity::ProviderFull { .. } => "provider_full",
            Capacity::RouteFull { .. } => "route_full",
            Capacity::FloorUnmeasured { .. } => "floor_unmeasured",
            Capacity::FloorLow { .. } => "floor_low",
            Capacity::BoxedFull { .. } => "boxed_full",
            Capacity::CoolingDown { .. } => "cooling_down",
        }
    }
}

impl fmt::Display for Capacity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let against = |running: &usize, ceiling: &u32| {
            format!("{running} running against a ceiling of {ceiling}")
        };
        match self {
            Capacity::Undeclared(path) => write!(
                f,
                "no host configuration at {} declares capacity",
                path.display()
            ),
            Capacity::Unreadable { path, kind } => write!(
                f,
                "the host configuration at {} cannot be read: {kind}",
                path.display()
            ),
            Capacity::Unmeasured(why) => write!(f, "what it seats cannot be measured: {why}"),
            Capacity::RunUnmeasured { run, why } => {
                write!(f, "running run '{run}' does not say what it seats: {why}")
            }
            Capacity::ProviderUndeclared(provider) => write!(
                f,
                "the host configuration declares no ceiling for provider {provider}"
            ),
            Capacity::RouteUndeclared { provider, route } => write!(
                f,
                "the host configuration declares no ceiling for route {route} of provider \
                 {provider}"
            ),
            Capacity::RunsUndeclared(path) => write!(
                f,
                "the host file {} declares no run ceiling; forge.host/v4 declares `runs.ceiling`",
                path.display()
            ),
            Capacity::RunsFull { running, ceiling } => write!(
                f,
                "the journal drives {running} runs, at its ceiling of {ceiling} (forge.host/v4 \
                 `runs.ceiling`)"
            ),
            Capacity::ProviderFull {
                provider,
                running,
                ceiling,
            } => write!(
                f,
                "provider {provider} is full: {}",
                against(running, ceiling)
            ),
            Capacity::RouteFull {
                provider,
                route,
                class,
                running,
                ceiling,
            } => write!(
                f,
                "{class} route {route} of provider {provider} is full: {}",
                against(running, ceiling)
            ),
            Capacity::FloorUnmeasured { at, kind } => {
                write!(f, "{at} cannot be measured: {kind}")
            }
            Capacity::FloorLow { at, free, floor } => {
                write!(f, "{at} has {free} bytes free, below its floor of {floor}")
            }
            Capacity::BoxedFull { running, ceiling } => {
                write!(f, "boxed builds are full: {}", against(running, ceiling))
            }
            Capacity::CoolingDown {
                provider,
                route,
                standing,
            } => cooling::word(f, (provider, route), standing),
        }
    }
}
