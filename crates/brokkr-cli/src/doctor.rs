//! `brokkr doctor` — verify tools, drivers, the workspace database, and
//! (optionally) a bundle, without executing any agent. Required tools
//! fail the check; optional ones warn. Acceptance criterion: a user can
//! see what is missing before a run wastes a model session on it.

use std::path::Path;
use std::process::Command;

use brokkr_runtime::{resolve_agent, Adapters, Availability, Bundle, Library, Presence};
use brokkr_store::Store;

pub struct Report {
    pub healthy: bool,
    lines: Vec<String>,
}

impl Report {
    fn ok(&mut self, what: &str, detail: String) {
        self.lines.push(format!("ok       {what}: {detail}"));
    }
    fn warn(&mut self, what: &str, detail: String) {
        self.lines.push(format!("warn     {what}: {detail}"));
    }
    fn missing(&mut self, what: &str, detail: String) {
        self.healthy = false;
        self.lines.push(format!("MISSING  {what}: {detail}"));
    }
    pub fn render(&self) -> String {
        self.lines.join("\n")
    }
}

fn tool_version(program: &str) -> Option<String> {
    let out = Command::new(program).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
    )
}

/// Probe every provider an adapter file declares, reporting its binary,
/// the probe result and the abstract models it serves — and collecting
/// the availability facts the resolver's non-`Unknown` arms exist for.
/// A missing provider is a warning: the fleet must work on machines
/// without every tool.
fn probe_providers(
    report: &mut Report,
    adapters_root: &Path,
    probe: fn(&str) -> Option<String>,
) -> Availability {
    let mut availability = Availability::unspecified();
    let adapters = match Adapters::load(adapters_root) {
        Ok(adapters) => adapters,
        Err(error) => {
            report.warn("adapters", format!("{}: {error}", adapters_root.display()));
            return availability;
        }
    };
    for adapter in adapters.providers() {
        let models: Vec<&str> = adapter.models.keys().map(String::as_str).collect();
        let serves = match models.is_empty() {
            true => "serves no abstract model yet".to_string(),
            false => format!("serves {}", models.join(", ")),
        };
        match probe(&adapter.binary) {
            Some(version) => {
                availability.record(&adapter.provider, Presence::Available);
                report.ok(&adapter.provider, format!("{version} · {serves}"));
            }
            None => {
                availability.record(&adapter.provider, Presence::Unavailable);
                // The advice, where the operator wrote one, comes from
                // the adapter file: it belongs beside the binary name,
                // not in a Rust constant that needs a release to fix.
                let hint = match &adapter.hint {
                    Some(hint) => format!(" ({hint})"),
                    None => String::new(),
                };
                report.warn(
                    &adapter.provider,
                    format!(
                        "binary '{}' not found — seats resolving to this provider \
                         will fail to spawn{hint} · {serves}",
                        adapter.binary
                    ),
                );
            }
        }
    }
    availability
}

/// Per agent, which model would be chosen HERE — by calling the same
/// pure `resolve` the compiler calls, with this machine's probed facts.
/// This is the real consumer of availability's non-`Unknown` arms, and
/// it is the surface that catches a mapped-but-uninstalled chain before
/// a run pays for it.
fn report_agents(
    report: &mut Report,
    library_root: &Path,
    adapters_root: &Path,
    availability: &Availability,
) {
    let loaded = Library::load(library_root)
        .and_then(|library| Adapters::load(adapters_root).map(|adapters| (library, adapters)));
    let (library, adapters) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            // No library is a normal state: a tree whose bundles all
            // inline needs none, so this is information, not a failure.
            report.warn("agents", format!("{}: {error}", library_root.display()));
            return;
        }
    };
    for agent in library.agents() {
        match resolve_agent(&library, &adapters, availability, &agent.name) {
            Ok(resolution) => report.ok(
                &format!("agent {}", agent.name),
                format!(
                    "would run {} via {} here (chain {})",
                    resolution.candidates[0].model,
                    resolution.candidates[0].provider,
                    agent.models.join(" → ")
                ),
            ),
            Err(error) => report.warn(&format!("agent {}", agent.name), error.to_string()),
        }
    }
}

pub fn doctor(bundle: Option<&Path>, db: &Path) -> Report {
    doctor_with_probe(
        bundle,
        db,
        Path::new(brokkr_runtime::bundle::DEFAULT_AGENTS_DIR),
        Path::new(brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR),
        tool_version,
    )
}

fn doctor_with_probe(
    bundle: Option<&Path>,
    db: &Path,
    library_root: &Path,
    adapters_root: &Path,
    probe: fn(&str) -> Option<String>,
) -> Report {
    let mut report = Report {
        healthy: true,
        lines: Vec::new(),
    };

    // The pinned contract versions this binary was built against.
    report.ok(
        "contracts",
        format!(
            "engine {}, event_schema {}, database_schema {}, driver_protocol {}",
            brokkr_runtime::ENGINE_VERSION,
            brokkr_runtime::bundle::EVENT_SCHEMA,
            brokkr_store::DATABASE_SCHEMA,
            brokkr_runtime::bundle::DRIVER_PROTOCOL,
        ),
    );

    // Required: the engine's own effects use git (drift/dirty gates).
    match probe("git") {
        Some(v) => report.ok("git", v),
        None => report.missing(
            "git",
            "required for worktree, drift, and dirty gates".into(),
        ),
    }
    // Optional: each agent CLI matters only to bundles whose seats use
    // its driver. The five-tuple that used to live here is now READ FROM
    // THE ADAPTER FILES (decision 0016), so a sixth provider shows up in
    // doctor without a rebuild — the same property that makes "adding a
    // provider is not a release" true.
    let availability = probe_providers(&mut report, adapters_root, probe);
    // Python is not a provider; it is what the `exec` driver's script
    // templates usually invoke, so it stays a named warning of its own.
    match probe("python3") {
        Some(v) => report.ok("python3", v),
        None => report.warn(
            "python3",
            "not found — seats using the exec driver's script templates will \
             fail to spawn"
                .into(),
        ),
    }
    report_agents(&mut report, library_root, adapters_root, &availability);

    match Store::open(db) {
        Ok(_) => report.ok(
            "database",
            format!("{} opens (WAL, append-only triggers)", db.display()),
        ),
        Err(e) => report.missing("database", format!("{}: {e}", db.display())),
    }

    if let Some(dir) = bundle {
        // Against the roots doctor was ASKED about, not the process's
        // own: a bundle's compile now reads the adapter data for
        // decision 0021's refusals as well as for agent resolution, and
        // doctor reporting on one tree while compiling against another
        // would be the forge diagnosing itself wrong.
        match Bundle::compile_with(dir, library_root, adapters_root) {
            Ok(bundle) => report.ok(
                "bundle",
                format!(
                    "'{}' compiles, digest {}",
                    bundle.name,
                    bundle.manifest_digest()
                ),
            ),
            Err(e) => report.missing("bundle", format!("{}: {e}", dir.display())),
        }
    }

    report
}

#[cfg(test)]
mod tests;
