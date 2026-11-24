//! Decision 0104 rulings 1 and 4: `brokkr doctor` prints each deadline
//! floor the machine's host file declares, and names each route an
//! adapter declares egress `local` that the file does not declare
//! `shared-local`. Neither line refuses: egress is the adapter's network
//! fact and the route class is this machine's capacity fact, and neither
//! implies the other. Decision 0110 ruling 7: it warns, and never
//! refuses, where the host file declares no run ceiling; decision 0125:
//! only where the machine's availability is not measured, which it says.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use brokkr_runtime::admission::{
    declared_routes, host_file, measurable, runs_undeclared, HostRoutes, RouteClass, RouteDecl,
    Unmeasured,
};
use brokkr_runtime::agents::EgressClass;
use brokkr_runtime::Adapters;

use super::{safe, Report};

/// Print the machine's floor lines and egress mismatches, reading the
/// host file where `host_file` places it, and whether this machine's
/// availability is measured.
pub(super) fn report(report: &mut Report, adapters_root: &Path) {
    report_from(
        report,
        adapters_root,
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        measurable(),
    );
}

/// [`report`] with the two directories that place the host file and
/// whether the machine is `measured` given, so a test plants them without
/// moving the process's environment or reading the machine.
pub(super) fn report_from(
    report: &mut Report,
    adapters_root: &Path,
    xdg: Option<OsString>,
    home: Option<OsString>,
    measured: Result<(), Unmeasured>,
) {
    match host_file(xdg, home) {
        Ok(file) => {
            report_at(report, adapters_root, &file);
            runs(report, &file, measured);
        }
        Err(error) => report.warn("host", error.to_string()),
    }
}

/// Decision 0125 ruling 7, amending decision 0110 ruling 7: on a machine
/// whose availability is measured, a host file that declares no run
/// ceiling is one line saying entries are judged by demand; on one that is
/// not, a declared host file is one warning saying so, and one without a
/// ceiling the warning it has been. Neither refuses.
fn runs(report: &mut Report, file: &Path, measured: Result<(), Unmeasured>) {
    let shown = safe(&file.display().to_string());
    let undeclared = runs_undeclared(file);
    match measured {
        Ok(()) => {
            if let Some(version) = undeclared {
                report.ok(
                    "host",
                    format!(
                        "the host file {shown} is {version} and declares no run ceiling; this \
                         machine's availability is measured, so queued entries are judged by \
                         their recipes' measured demand (decision 0125)"
                    ),
                );
            }
        }
        Err(why) => {
            if matches!(declared_routes(file), Ok(HostRoutes::Declared(_))) {
                report.warn(
                    "host",
                    format!(
                        "this machine's availability is not measured ({why}), so admission keeps \
                         its count ceilings and the run ceiling stays required (decision 0125)"
                    ),
                );
            }
            if let Some(version) = undeclared {
                report.warn(
                    "host",
                    format!(
                        "the host file {shown} is {version} and declares no run ceiling, so \
                         every queued entry waits as runs_undeclared; forge.host/v4 declares \
                         runs.ceiling"
                    ),
                );
            }
        }
    }
}

/// [`report`] with the host file given rather than read from the
/// environment, so a test plants one without moving the process's.
pub(super) fn report_at(report: &mut Report, adapters_root: &Path, file: &Path) {
    let Ok(adapters) = Adapters::load(adapters_root) else {
        return;
    };
    let declared: BTreeMap<String, BTreeMap<String, RouteDecl>> = match declared_routes(file) {
        Ok(HostRoutes::Absent) => BTreeMap::new(),
        Ok(HostRoutes::Unreadable(kind)) => {
            report.warn(
                "host",
                format!(
                    "the host configuration at {} cannot be read: {kind}",
                    file.display()
                ),
            );
            return;
        }
        Ok(HostRoutes::Declared(routes)) => routes,
        Err(error) => {
            report.warn("host", error.to_string());
            return;
        }
    };
    for (provider, routes) in &declared {
        for (route, route_decl) in routes {
            if let Some(seconds) = route_decl.timeout_floor_seconds {
                report.ok(
                    "floor",
                    format!(
                        "{}/{} {seconds} seconds on shared-local hardware — seats on this route \
                         run under at least this deadline",
                        safe(provider),
                        safe(route)
                    ),
                );
            }
        }
    }
    for adapter in adapters.providers() {
        for (route, class) in &adapter.routes {
            if *class != EgressClass::Local {
                continue;
            }
            let shared_local = declared
                .get(&adapter.provider)
                .and_then(|routes| routes.get(route))
                .is_some_and(|declared| declared.class == RouteClass::SharedLocal);
            if shared_local {
                continue;
            }
            report.warn(
                "egress",
                format!(
                    "route '{}' of provider '{}' declares egress local, and the host file does \
                     not declare it shared-local: its seats keep their authored deadlines and \
                     count against their provider",
                    safe(route),
                    safe(&adapter.provider)
                ),
            );
        }
    }
}

#[cfg(test)]
mod tests;
