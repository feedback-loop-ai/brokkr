//! `brokkr doctor` — verify tools, drivers, the workspace database, and
//! (optionally) a bundle, without executing any agent. Required tools
//! fail the check; optional ones warn. Acceptance criterion: a user can
//! see what is missing before a run wastes a model session on it.

use std::collections::BTreeSet;
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
    // POSIX `sh` has no portable `--version`; dash exits 2 for it even
    // though the executable is healthy. The exec adapter needs presence,
    // not a shell brand, so probe the one operation every `sh` promises.
    if program == "sh" {
        return Command::new(program)
            .args(["-c", "printf 'POSIX shell'"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).into_owned());
    }
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

/// What this report was able to learn about the seats that would bind a
/// credential — ruling 4's question when a bundle could answer it, and,
/// when none could, WHICH of the two ways that happened. The distinction
/// is only wording, but the wording is the whole of the second half of
/// ruling 4: doctor says what it checked, so it must not tell an
/// operator holding a broken bundle that they passed none.
///
/// Passed and matched by reference throughout: a derived `Clone` nobody
/// calls is a function the exact-coverage gate counts and no test can
/// reach.
enum Seats<'a> {
    /// Every name declared in some seat's `secrets`, through the
    /// composed bundle.
    Declared(&'a BTreeSet<String>),
    NoBundleGiven,
    BundleDidNotCompile,
}

/// Decision 0036 ruling 5: a credential a route declares, which no seat
/// binds and the process environment DOES hold, is reported by route
/// name. The ambient channel is not forbidden here — that would strand
/// every route whose class the operator has not yet ruled — but it stops
/// being invisible. It is the one channel the machine cannot refuse at
/// compile time, cannot record in the journal and cannot move a digest
/// for, so this line is the only place it can be seen at all.
///
/// Decision 0040 ruling 4 fixes what "ambient" tests. A name sitting in
/// the bindings STORE that no seat declares in its `secrets` is never
/// bound to the driver: the run passes it nothing, so if the launching
/// shell exports it the driver takes it ambiently and the old reading —
/// store membership — said nothing. A false negative on exactly the
/// channel ruling 5 exists to make visible. So the test is now the
/// inspected bundle's declared names, and store membership is necessary
/// for a binding but is not one.
///
/// Necessary is the half that is easy to drop, so it is spelled out: a
/// name a seat DECLARES and the store does not hold is bound to nothing
/// either. The declaring seat refuses at spawn, which closes its own
/// half — but `declared` is a union over every seat, so a sibling seat
/// on the same route that does not declare the name spawns anyway and
/// its driver reads the launching shell's copy. Reading declaration
/// alone as coverage would put that back under the same silence ruling
/// 4 was written to lift.
///
/// With no bundle to inspect doctor reports store membership and SAYS
/// that is what it checked — a weaker question honestly named beats a
/// strong one silently missed — and says WHICH way it came to have no
/// seats to ask, because an operator who passed `--bundle` and reads
/// "no bundle" beside their `MISSING bundle` line is owed the link.
///
/// Names only, on every side: `store_names` never reads a value, a
/// seat's `secrets` is a list of names, and the ambient probe answers
/// "is this variable set", never with what it is set to.
fn report_ambient_credentials(
    report: &mut Report,
    adapters_root: &Path,
    secrets_store: &Path,
    seats: &Seats<'_>,
    ambient: fn(&str) -> bool,
) {
    // An unreadable adapters tree is already a warning of its own from
    // `probe_providers`; saying it twice would be noise.
    let Ok(adapters) = Adapters::load(adapters_root) else {
        return;
    };
    let held = brokkr_protocol::secret::store_names(secrets_store).unwrap_or_default();
    let in_store = |variable: &String| held.contains(variable);
    for adapter in adapters.providers() {
        for (route, variable) in &adapter.credentials {
            let covered = match *seats {
                // A binding is both halves at once: a seat that names
                // the variable, and a store that can answer for it.
                Seats::Declared(declared) => declared.contains(variable) && in_store(variable),
                Seats::NoBundleGiven | Seats::BundleDidNotCompile => in_store(variable),
            };
            if covered || !ambient(variable) {
                continue;
            }
            let checked = match *seats {
                Seats::Declared(declared) if declared.contains(variable) => format!(
                    "the seat declaring it can be handed nothing the bindings \
                     store at {} does not hold (decision 0040 ruling 4 — store \
                     membership is necessary for a binding)",
                    secrets_store.display()
                ),
                Seats::Declared(_) => "no seat of the inspected bundle binds it (decision \
                                       0040 ruling 4 — a name in the store that no seat \
                                       declares is bound to nothing)"
                    .to_string(),
                Seats::NoBundleGiven => format!(
                    "no bundle was given to inspect, so this checked membership \
                     of the bindings store at {} and not whether any seat binds \
                     it (decision 0040 ruling 4)",
                    secrets_store.display()
                ),
                Seats::BundleDidNotCompile => format!(
                    "the bundle given does not compile, so it declares no seats \
                     to ask and this checked membership of the bindings store at \
                     {} and not whether any seat binds it (decision 0040 ruling 4)",
                    secrets_store.display()
                ),
            };
            report.warn(
                &format!("route {route}"),
                format!(
                    "credential '{variable}' is satisfied from the process \
                     environment — an ambient value is journaled nowhere and \
                     moves no digest (decision 0036 ruling 5); {checked}"
                ),
            );
        }
    }
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

/// Is this variable set in the process environment? A boolean, never the
/// value: decision 0012's rule holds for a variable doctor only reports
/// the EXISTENCE of.
fn ambient_variable(name: &str) -> bool {
    std::env::var_os(name).is_some()
}

pub fn doctor(
    bundle: Option<&Path>,
    db: &Path,
    secrets_store: &Path,
    realms: Option<&Path>,
) -> Report {
    let mut report = doctor_with_probe(
        bundle,
        db,
        Path::new(brokkr_runtime::bundle::DEFAULT_AGENTS_DIR),
        Path::new(brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR),
        secrets_store,
        tool_version,
        ambient_variable,
    );
    let workspace = std::env::current_dir().unwrap_or_default();
    report_realm_house(&mut report, &workspace, realms);
    report
}

fn report_realm_house(report: &mut Report, workspace: &Path, named: Option<&Path>) {
    match brokkr_runtime::realms::World::discover(workspace, named) {
        Ok(Some(world)) => {
            let mut houses = 0;
            let mut failures = Vec::new();
            for realm in world
                .map
                .realms
                .iter()
                .filter(|realm| realm.house.is_some())
            {
                match world.house_for_realm(realm) {
                    Ok(_) => houses += 1,
                    Err(error) => failures.push(error),
                }
            }
            if failures.is_empty() {
                report.ok(
                    "house rules",
                    format!("{houses} realm declaration(s) readable"),
                );
            } else {
                for error in failures {
                    report.missing("house rules", error.to_string());
                }
            }
        }
        Ok(None) => report.ok("house rules", "no realms map; none declared".into()),
        Err(error) => report.missing("realms map", error.to_string()),
    }
}

fn doctor_with_probe(
    bundle: Option<&Path>,
    db: &Path,
    library_root: &Path,
    adapters_root: &Path,
    secrets_store: &Path,
    probe: fn(&str) -> Option<String>,
    ambient: fn(&str) -> bool,
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
    // Compile now so the hands probe can name the exact boxed sites in
    // the bundle the operator asked doctor to inspect. The bundle result
    // is still rendered at the end, after the other diagnostics.
    let compiled = bundle.map(|dir| (dir, Bundle::compile_with(dir, library_root, adapters_root)));
    let hands: Vec<&str> = compiled
        .as_ref()
        .and_then(|(_, result)| result.as_ref().ok())
        .map(|bundle| bundle.hands.keys().map(String::as_str).collect())
        .unwrap_or_default();
    // Decision 0043: a bundle whose seats box their hands cannot run
    // without bubblewrap, and the boundary is never simulated.
    match probe("bwrap") {
        Some(v) if hands.is_empty() => report.ok("hands", format!("{v} · boxed seats can run")),
        Some(v) => report.ok(
            "hands",
            format!("{v} · seats {hands:?} declare hands and can run"),
        ),
        None if hands.is_empty() => report.warn(
            "hands",
            "bubblewrap (bwrap) not found — seats declaring hands will refuse to spawn".into(),
        ),
        None => report.warn(
            "hands",
            format!(
                "bubblewrap (bwrap) not found — seats {hands:?} declare hands and will refuse \
                 to spawn"
            ),
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

    // Compiled before the ambient report and reported after it: decision
    // 0040 ruling 4 asks whether any SEAT binds a variable, which is a
    // fact only the composed bundle holds, while the bundle line belongs
    // at the end of the report where an operator has been reading it.
    //
    // Against the roots doctor was ASKED about, not the process's own: a
    // bundle's compile now reads the adapter data for decision 0021's
    // refusals as well as for agent resolution, and doctor reporting on
    // one tree while compiling against another would be the machine
    // diagnosing itself wrong.
    // A bundle that does not compile declares nothing this report can
    // trust, so it is no bundle to inspect — and it says which of the
    // two silences it is, rather than reading an empty set as "no seat
    // binds anything" or telling the operator they passed no bundle.
    let declared: Option<BTreeSet<String>> = compiled.as_ref().and_then(|(_, result)| {
        result.as_ref().ok().map(|bundle| {
            bundle
                .seats
                .values()
                .flat_map(|seat| seat.secrets.iter().cloned())
                .collect()
        })
    });
    let seats = match (&compiled, &declared) {
        (_, Some(declared)) => Seats::Declared(declared),
        (Some(_), None) => Seats::BundleDidNotCompile,
        (None, None) => Seats::NoBundleGiven,
    };
    report_ambient_credentials(&mut report, adapters_root, secrets_store, &seats, ambient);

    match Store::open(db) {
        Ok(_) => report.ok(
            "database",
            format!("{} opens (WAL, append-only triggers)", db.display()),
        ),
        Err(e) => report.missing("database", format!("{}: {e}", db.display())),
    }

    if let Some((dir, result)) = compiled {
        match result {
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
