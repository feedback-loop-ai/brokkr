//! `brokkr doctor` — verify tools, drivers, the workspace database, and
//! (optionally) a bundle, without executing any agent. Required tools
//! fail the check; optional ones warn. Acceptance criterion: a user can
//! see what is missing before a run wastes a model session on it.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::{
    dsh_composite_prepared, DshComposite, DshInvocation, DshPrepared, DshSeams, DshSelection,
    DshUnprepared, DshUnselected,
};
use brokkr_protocol::hands::HandsSpec;
use brokkr_runtime::agents::{Adapter, ResumeIdentity, ResumeStatus};
use brokkr_runtime::{resolve_agent, Adapters, Availability, Bundle, Library, Presence};
use brokkr_store::Store;

use crate::boundary;
use crate::render::Safe;

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
    Some(safe_line(&out.stdout))
}

/// The first line of a probe's output, escaped for a terminal: a version
/// banner, or the reason a box gave for not running one. Both are output
/// doctor did not author, so both go through `Safe`.
fn safe_line(output: &[u8]) -> String {
    let line = String::from_utf8_lossy(output);
    Safe::new(line.lines().next().unwrap_or_default().trim())
        .as_str()
        .to_string()
}

/// Quote one word for the `bash -lc` string `hands::execute` runs. The
/// dialect's binary is a bare filename (`Dialect::check` refuses a
/// slash), but a dialect file is realm data: a name may still carry a
/// shell metacharacter, and the probe must run the tool, never the
/// metacharacter.
fn shell_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// How long doctor waits for a probe inside the box. A version banner is
/// milliseconds' work; anything near this bound is a box that is not
/// running, not a slow tool.
const PROBE_TIMEOUT: Duration = Duration::from_secs(30);

/// The exit `bash -lc` reports for a command it cannot find. It is the
/// one non-zero code that answers doctor's question about the TOOL;
/// every other one is a fact about the box.
const NOT_ON_PATH: i32 = 127;

/// Probe a dialect tool where its gate will run (issue #218): run the
/// binary's own `--version` inside a box built from the very spec the
/// compiler builds for a dialect `validate`/`check` step. `Ok(None)`
/// means the box was built and the tool was not on its PATH; `Err` means
/// no answer came from a box at all — a different fact, said differently.
fn probe_in_box(spec: &HandsSpec, workdir: &Path, program: &str) -> Result<Option<String>, String> {
    let command = if program == "sh" {
        "sh -c \"printf 'POSIX shell'\"".to_string()
    } else {
        format!("{} --version", shell_quote(program))
    };
    let session = brokkr_protocol::hands::session_dir("doctor").map_err(unbuilt)?;
    let result = brokkr_protocol::hands::execute(spec, workdir, &session, &command, PROBE_TIMEOUT);
    let _ = std::fs::remove_dir_all(&session);
    match result {
        Ok(executed) => box_answer(&executed),
        Err(error) => Err(unbuilt(error)),
    }
}

/// The reason a box never stood, in the words the report prints.
fn unbuilt(error: String) -> String {
    format!("the box could not be built: {error}")
}

/// Read one probe run inside the box. Only two outcomes say anything
/// about the tool: exit 0 carries its version, and 127 means the box was
/// built and the name is not on its PATH. Everything else — a bubblewrap
/// that a kernel or container policy refuses a namespace to, a probe that
/// never returned — is a fact about the box, and reporting it as a fact
/// about the tool is the same defect issue #218 exists to close: doctor
/// would tell an operator to install a tool that is already installed.
fn box_answer(executed: &brokkr_protocol::hands::Executed) -> Result<Option<String>, String> {
    if executed.timed_out {
        return Err(format!(
            "the probe was still running in the box after {} seconds",
            PROBE_TIMEOUT.as_secs()
        ));
    }
    match executed.exit_code {
        0 => Ok(Some(safe_line(executed.stdout.as_bytes()))),
        NOT_ON_PATH => Ok(None),
        code => {
            let said = safe_line(executed.stderr.as_bytes());
            match said.is_empty() {
                true => Err(format!("the probe did not run in the box: exit {code}")),
                false => Err(format!(
                    "the probe did not run in the box: exit {code}, {said}"
                )),
            }
        }
    }
}

/// Which surface answered for a dialect's tool. The words are the fix as
/// much as the probe is: doctor must never answer for the host PATH
/// while the gate runs in a box that cannot see it (issue #218).
enum Surface {
    /// Inside the box the dialect's `validate`/`check` step runs in.
    Box,
    /// On the host PATH because the boundary builds no box of Brokkr's
    /// (`harness`, `open`), which doctor says rather than implying a box.
    Host,
    /// The boundary is boxed but no answer came from a box on this
    /// machine; the answer came from the host PATH, and the reason it
    /// had to rides along.
    NoBox(String),
}

impl Surface {
    fn probed(&self, boundary: Boundary) -> String {
        match self {
            Surface::Box => "probed inside the box".to_string(),
            Surface::Host => {
                format!("probed on the host PATH (boundary `{boundary}` builds no box of Brokkr's)")
            }
            Surface::NoBox(reason) => format!("probed on the host PATH ({reason})"),
        }
    }
}

/// What doctor observed of one provider's binary: the executable it
/// actually looked for, the version that executable answered with, and
/// the provider-specific suffix its line carries.
struct Observed {
    /// The executable probed. For every provider but DSH this is the
    /// declared `adapter.binary`; for DSH it is what the adapter's own
    /// seam selected, which is the whole of the 8.8(c) correction.
    binary: String,
    version: Option<String>,
    warning: bool,
    suffix: String,
    /// Why no executable was selected, when selection itself failed:
    /// the cause rides beside the declared spelling into the
    /// unavailable line, so an absent `PATH` is reported as an absent
    /// `PATH` rather than as a binary nobody looked for (security hold
    /// 2026-09-20, S1). `None` for every probe that had an executable.
    cause: Option<String>,
}

/// How doctor reads the DSH provider line: one seam resolution, the
/// version of THAT executable, and the composite over the same seams.
/// Injected so the shipped tests build synthetic homes instead of
/// resolving the operator's real one.
type CompositeProbe = fn(&Adapter, fn(&str) -> Option<String>) -> Observed;

/// The composite's canonical digest and plugin component, as the detail
/// line and the qualification record both need them. Named rather than
/// inline so a unit test can read the mapping without a DSH install; the
/// real `dsh_provider_line` reaches it only through the real seams.
fn composite_identity(composite: DshComposite) -> (String, String) {
    (
        composite.canonical().to_string(),
        composite.plugin().to_string(),
    )
}

/// The DSH provider line (task 8.8(c)).
///
/// The version and the composite must describe ONE installation. Before
/// this correction doctor probed the bare declared `adapter.binary` on
/// PATH while the composite followed the adapter's seam, so a
/// `BROKKR_DSH_BIN` override produced a version from the PATH install
/// beside a digest from the overridden one (measured both ways,
/// 2026-09-19). Both halves now come from one `DshSeams::selected`, and
/// nothing is probed unless that selection SUCCEEDED: the first routing
/// through the seam probed the declared spelling after a failed lookup,
/// which under an absent `PATH` executed a `dsh` sitting in the working
/// directory (security hold 2026-09-20, S1).
///
/// The report's ordinary probe runs a NAME, and is not used here: DSH is
/// probed as the INVOCATION its selection carries (`invocation_version`).
fn dsh_provider_line(adapter: &Adapter, _: fn(&str) -> Option<String>) -> Observed {
    dsh_provider_line_probing(adapter, invocation_version)
}

/// `dsh_provider_line` over an injected version probe and the real seams.
fn dsh_provider_line_probing(
    adapter: &Adapter,
    probe: fn(&DshInvocation) -> Option<String>,
) -> Observed {
    dsh_provider_line_with(adapter, probe, DshSeams::selected, |prepared| {
        dsh_composite_prepared(prepared)
            .map(composite_identity)
            .map_err(|error| error.to_string())
    })
}

/// The version of a SELECTED executable, asked as native asks it: the
/// candidate the seam's search found, under the name it was looked up
/// by. Probing the canonical target instead reported `env`'s version for
/// a `dsh` symlinked to it, where native exits 1 on the name mismatch
/// (review of run `124cca78`, R2). Nothing is searched for again: the
/// invocation's program is a path.
fn invocation_version(invocation: &DshInvocation) -> Option<String> {
    let out = invocation.command().arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(safe_line(&out.stdout))
}

/// `dsh_provider_line` over an injected seam resolver and composite
/// producer, so every disposition is a plain test without a real DSH
/// install and its node probe. Production reaches it only through the
/// real seams.
fn dsh_provider_line_with(
    adapter: &Adapter,
    probe: impl Fn(&DshInvocation) -> Option<String>,
    selected: impl FnOnce() -> Result<DshSelection, DshUnselected>,
    composite: impl FnOnce(&DshPrepared) -> Result<(String, String), String>,
) -> Observed {
    // The DSH work shape's name, one spelling in Rust so the guide
    // sample and this line cannot drift apart silently.
    const DSH_SHAPE: &str = "headless-work";
    let shape = adapter.resume.shape(DSH_SHAPE);
    let declared = shape.and_then(|shape| match &shape.identity {
        ResumeIdentity::Measured { wrapper_digest, .. } => wrapper_digest.clone(),
        ResumeIdentity::Unknown { .. } => None,
    });
    let supported = shape.is_some_and(|shape| shape.status == ResumeStatus::Supported);
    // One resolution of both seams, matched BEFORE anything is probed.
    // A failed selection has no executable: the declared spelling and
    // the cause go to the report, and neither DSH nor Node is spawned.
    // A failed home leaves a successful selection intact: it is a named
    // composite failure, not evidence that the binary is missing.
    let DshSelection {
        executable: binary,
        admission,
    } = match selected() {
        Ok(selection) => selection,
        Err(DshUnselected { declared, cause }) => {
            return Observed {
                binary: declared,
                version: None,
                warning: false,
                suffix: String::new(),
                cause: Some(cause.to_string()),
            };
        }
    };
    // The home's profile and pnpm lock were admitted with the selection,
    // BEFORE this point, and the admission is what holds the invocation.
    // A located lock that failed it leaves nothing to probe: the refusal
    // is reported beside the declaration, with no version, and neither
    // DSH nor Node runs for a lock YAML refuses (R1, R3). A home that was
    // never located is a different fact — the executable's availability
    // does not depend on it — so its invocation is still asked.
    let (invocation, prepared) = match admission {
        Ok(prepared) => (prepared.invocation().clone(), Ok(prepared)),
        Err(DshUnprepared::Unlocated { invocation, cause }) => (invocation, Err(cause.to_string())),
        Err(DshUnprepared::Refused { cause }) => {
            let cause = cause.to_string();
            let (warning, suffix) = composite_detail(declared.as_deref(), supported, Err(&cause));
            return Observed {
                binary,
                version: None,
                warning,
                suffix,
                cause: None,
            };
        }
    };
    let version = probe(&invocation);
    if version.is_none() {
        // The executable doctor selected did not answer. Nothing the
        // composite could say describes an installation this report
        // reached, and a PATH decoy is never tried in its place.
        return Observed {
            binary,
            version,
            warning: false,
            suffix: String::new(),
            cause: None,
        };
    }
    let identity = prepared.and_then(|prepared| composite(&prepared));
    let (warning, suffix) = match &identity {
        Ok((digest, plugin)) => {
            composite_detail(declared.as_deref(), supported, Ok((digest, plugin)))
        }
        Err(error) => composite_detail(declared.as_deref(), supported, Err(error)),
    };
    Observed {
        binary,
        version,
        warning,
        suffix,
        cause: None,
    }
}

/// The pure classifier behind `dsh_provider_line`, so every disposition
/// is a plain test: informational while no `supported` shape declares a
/// digest; a warning flag only when one does and the composite differs
/// or is unreadable. The `Ok` pair carries the canonical composite and
/// the plugin component, both of which the qualification record needs.
///
/// An unreadable composite has no equality result, so it reports neither
/// `matches` nor `differs`. It keeps the declaration context all the
/// same: whether a digest was declared is a fact doctor knows even when
/// it could not read the installation to compare against (design D10's
/// doctor state decision).
fn composite_detail(
    declared: Option<&str>,
    supported: bool,
    composite: Result<(&str, &str), &str>,
) -> (bool, String) {
    match composite {
        Ok((digest, plugin)) => {
            let detail = match declared {
                Some(declared) if declared == digest => {
                    "matches the declared wrapper_digest".to_string()
                }
                Some(declared) => format!("differs from the declared wrapper_digest {declared}"),
                None => "no declared wrapper_digest".to_string(),
            };
            let warning = supported && declared.is_some() && declared != Some(digest);
            (
                warning,
                format!("composite {digest} plugin {plugin} ({detail})"),
            )
        }
        Err(error) => {
            let detail = match declared {
                Some(declared) => {
                    format!("declared wrapper_digest {declared}; comparison unavailable")
                }
                None => "no declared wrapper_digest".to_string(),
            };
            // The reason came from a filesystem, a lock file or a child
            // process, so it is output doctor did not author.
            let reason = Safe::new(error).as_str().to_string();
            (
                supported && declared.is_some(),
                format!("composite unreadable: {reason} ({detail})"),
            )
        }
    }
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
    composite: CompositeProbe,
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
        // DSH resolves its own executable through the adapter seam, so
        // its version and its composite describe one installation; every
        // other provider keeps the declared name and the bare probe.
        let observed = match adapter.provider.as_str() {
            "dsh" => composite(adapter, probe),
            _ => Observed {
                binary: adapter.binary.clone(),
                version: probe(&adapter.binary),
                warning: false,
                suffix: String::new(),
                cause: None,
            },
        };
        let Observed {
            binary,
            version,
            warning,
            suffix,
            cause,
        } = observed;
        match version {
            Some(version) => {
                availability.record(&adapter.provider, Presence::Available);
                let detail = match suffix.is_empty() {
                    true => format!("{version} · {serves}"),
                    false => format!("{version} · {serves} · {suffix}"),
                };
                if warning {
                    report.warn(&adapter.provider, detail);
                } else {
                    report.ok(&adapter.provider, detail);
                }
            }
            // Selected, and deliberately NOT probed: the refusal that
            // stopped the probe rides in the suffix. It is not a missing
            // binary, and saying so would send an operator holding a
            // malformed pnpm lock to reinstall an executable that is
            // there; nothing is recorded about an availability that was
            // never asked.
            None if !suffix.is_empty() => {
                let binary = Safe::new(&binary).as_str().to_string();
                report.warn(
                    &adapter.provider,
                    format!("binary '{binary}' selected and not probed · {serves} · {suffix}"),
                );
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
                // The binary is an operator's override or a seam's
                // selection and the cause came from a filesystem lookup:
                // neither is output doctor authored, so both go through
                // `Safe` at this, their only interpolation. Rendered raw,
                // a nonexistent override carrying a newline and a
                // clear-screen sequence reached the terminal verbatim
                // (security hold 2026-09-20, S2).
                let binary = Safe::new(&binary).as_str().to_string();
                let cause = match cause {
                    Some(cause) => format!(": {}", Safe::new(&cause).as_str()),
                    None => String::new(),
                };
                report.warn(
                    &adapter.provider,
                    format!(
                        "binary '{binary}' not found{cause} — seats resolving to this \
                         provider will fail to spawn{hint} · {serves}"
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
    let workspace = std::env::current_dir().unwrap_or_default();
    // The world is read once, before the bundle compiles: a bundle
    // doctor is asked about compiles in the discovered realm, under its
    // boundary (decision 0046 ruling 2), and the realm's own lines follow
    // the machine's.
    //
    // `inspect`, not `discover`: a doctor line reports and never refuses
    // (decision 0046's Addendum), so a crossing that has moved is one
    // realm's line here rather than the `Err` that would replace every
    // house, dialect and boundary line in this report with a single
    // "realms map" one and tell an operator their world is broken when
    // one contract moved (decision 0057).
    let world = brokkr_runtime::realms::World::inspect(&workspace, realms);
    let boundary = match &world {
        Ok(Some(world)) => world.boundary_for(&workspace),
        _ => Boundary::Namespace,
    };
    let (mut report, availability) = doctor_observed(
        bundle,
        db,
        Path::new(brokkr_runtime::bundle::DEFAULT_AGENTS_DIR),
        Path::new(brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR),
        secrets_store,
        tool_version,
        ambient_variable,
        boundary,
        bundle.map(|dir| {
            super::compile_in_realm(
                &workspace,
                dir,
                world.as_ref().ok().and_then(Option::as_ref),
                &workspace,
            )
        }),
        dsh_provider_line,
    );
    report_capabilities(
        &mut report,
        &world,
        &workspace,
        Path::new(brokkr_runtime::bundle::DEFAULT_ADAPTERS_DIR),
        &availability,
    );
    report_realm_world(&mut report, world, &workspace, tool_version, probe_in_box);
    report
}

fn report_realm_world(
    report: &mut Report,
    world: Result<Option<brokkr_runtime::realms::World>, brokkr_runtime::realms::WorldError>,
    workdir: &Path,
    probe: fn(&str) -> Option<String>,
    inside: fn(&HandsSpec, &Path, &str) -> Result<Option<String>, String>,
) {
    match world {
        Ok(Some(world)) => {
            report_realm_house_for_world(report, &world);
            report_realm_crossings(report, &world);
            report_realm_dialects(report, &world, workdir, probe, inside);
        }
        Ok(None) => {
            report.ok("house rules", "no realms map; none declared".into());
            report.ok("dialect", "no realms map; none declared".into());
        }
        Err(error) => report.missing("realms map", error.to_string()),
    }
}

/// How one grant's office scope reads. The three are different facts and
/// an empty list must never print as "all".
fn scope_words(grant: &brokkr_core::realms::CapabilityGrant) -> String {
    match grant.offices.as_deref() {
        None => "all requesting offices".to_string(),
        Some([]) => "no offices".to_string(),
        Some(offices) => format!("offices [{}] only", offices.join(", ")),
    }
}

/// How one declared native control reads, as the adapter data it is: the
/// argv that switches it, the measured default that needs none, the tool
/// lists it contributes to, or the reason it cannot be or has not been
/// measured. Words for a readout, never a claim that the control held.
fn disposition_words(disposition: &brokkr_runtime::capabilities::Disposition) -> String {
    use brokkr_runtime::capabilities::Disposition;
    match disposition {
        Disposition::Argv(argv) => format!("argv [{}]", argv.join(" ")),
        Disposition::Default(reason) => format!("the harness default ({reason})"),
        Disposition::Selection(lists) => format!(
            "tool lists include [{}] allow [{}] deny [{}]",
            lists.include.join(", "),
            lists.allow.join(", "),
            lists.deny.join(", ")
        ),
        Disposition::Unsupported(reason) => format!("cannot be switched ({reason})"),
        Disposition::Unmeasured(reason) => format!("unmeasured ({reason})"),
    }
}

/// Design D8's unknown-authority half: a realms map that could not be read
/// names no realm, so nothing here is keyed to one and nothing is said to
/// be granted, not granted or switched off anywhere — an empty grant set
/// invented for the occasion would be a denial doctor never read. What an
/// installed harness HAS is still adapter data, and is still named: its
/// tools, how ON and OFF are declared, the evidence scope and what stays
/// unmeasured, or the reason its inventory is unmeasured at all.
fn report_native_assessments(report: &mut Report, installed: &[&Adapter]) {
    use brokkr_runtime::capabilities::NativeInventory;
    for adapter in installed {
        let provider = &adapter.provider;
        let natives = match &adapter.native {
            NativeInventory::Known { known, .. } => known,
            NativeInventory::Unmeasured(reason) => {
                report.warn(
                    &format!("capabilities native {provider}"),
                    format!(
                        "native inventory unmeasured: {reason}. No native grant or denial is \
                         claimed"
                    ),
                );
                continue;
            }
        };
        for native in natives.values() {
            report.warn(
                &format!("capabilities native {provider} '{}'", native.capability),
                Safe::new(&format!(
                    "declared by the adapter: tools [{}] · ON: {} · OFF: {} · evidence: {} · \
                     still unmeasured: {} · whether any realm grants it is UNKNOWN, so neither \
                     a grant nor a denial is claimed",
                    native.tools.join(", "),
                    disposition_words(&native.on),
                    disposition_words(&native.off),
                    native.evidence.scope,
                    native.evidence.limitations.join("; ")
                ))
                .as_str()
                .to_string(),
            );
        }
    }
}

/// Decision 0065 ruling 4: per realm, what it grants — by which dialect,
/// to which offices, with which tools and restrictions — and then every
/// native capability an INSTALLED harness declares that the realm has not
/// granted, so the day a realm loses a power it used without permission is
/// loud rather than discovered later.
///
/// A line reports and never refuses, and independent results survive one
/// another (design D8). Each realm is read under ITS OWN grants, never the
/// current directory's, and each GRANT is validated on its own: one that
/// does not validate — a missing definition, a conflicting dialect, an
/// `mcp` grant slice two has not built — is its own failing line carrying
/// the compiler's refusal, the realm's other grants are still shown, and a
/// native power of the failing grant's name reads UNKNOWN under that realm
/// rather than denied, because doctor could not read what would cover it.
/// Definitions that cannot be read are one failing line of their own. A
/// map that could not be read leaves authority UNKNOWN: no empty grant set
/// is invented, and the installed harnesses are still assessed.
///
/// Nothing is run to say any of this: no model request, search, fetch or
/// capability server. "Installed" is the availability `probe_providers`
/// already recorded; a provider whose binary is absent keeps that line and
/// gets no native one, because nothing was observed about it here. A
/// declared control is adapter data — its evidence scope and its open
/// limitations are printed beside it, and a green argv test upgrades
/// neither. Adapter declarations that cannot be read say so here, so a
/// report with no native line is never read as a harness with no native
/// power.
fn report_capabilities(
    report: &mut Report,
    world: &Result<Option<brokkr_runtime::realms::World>, brokkr_runtime::realms::WorldError>,
    workspace: &Path,
    adapters_root: &Path,
    availability: &Availability,
) {
    use brokkr_runtime::capabilities::{
        restriction_names, Authority, CapabilityContext, Definitions, Denial, NativeInventory,
        Transport, UNMAPPED,
    };
    let adapters = match Adapters::load(adapters_root) {
        Ok(adapters) => Some(adapters),
        Err(error) => {
            // A warning on `probe_providers`' terms: a tree with no
            // adapters is a normal state there, and stays one here.
            report.warn(
                "capabilities native",
                format!(
                    "the adapter declarations at {} could not be read ({error}), so no \
                     harness's native capabilities are named below; that is NOT a finding that \
                     an installed harness has none",
                    adapters_root.display()
                ),
            );
            None
        }
    };
    let installed: Vec<&Adapter> = adapters
        .iter()
        .flat_map(|adapters| adapters.providers())
        .filter(|adapter| availability.presence(&adapter.provider) == Presence::Available)
        .collect();
    let (root, realms): (std::path::PathBuf, Vec<(String, _, &str)>) = match world {
        Ok(Some(world)) => (
            workspace
                .join(&world.source)
                .parent()
                .map_or_else(|| workspace.to_path_buf(), Path::to_path_buf),
            world
                .map
                .realms
                .iter()
                .map(|realm| (realm.name.clone(), realm.grants.clone(), "grants nothing"))
                .collect(),
        ),
        Ok(None) => (
            workspace.to_path_buf(),
            vec![(
                UNMAPPED.to_string(),
                Default::default(),
                "no realms map, so no capability grants are declared",
            )],
        ),
        Err(_) => {
            report.warn(
                "capabilities",
                "the realms map could not be read, so what each realm grants is UNKNOWN; \
                 nothing is assumed granted and nothing is assumed denied"
                    .into(),
            );
            report_native_assessments(report, &installed);
            return;
        }
    };
    if let Err(problem) = Definitions::load(&root) {
        report.missing(
            "capabilities definitions",
            format!(
                "{problem}; no grant can be validated and every compile under this \
                 configuration refuses until it is repaired"
            ),
        );
    }
    for (realm, grants, nothing) in realms {
        let what = format!("capabilities {realm}");
        if grants.is_empty() {
            report.ok(
                &what,
                format!(
                    "{nothing}; every native capability is governed by the no-grant default — \
                     switched off, or the seat is refused"
                ),
            );
        }
        // Each grant is judged ALONE, by the compiler's own validation:
        // one that fails is its own line and takes no neighbour with it.
        let mut valid = std::collections::BTreeMap::new();
        let mut unread = BTreeSet::new();
        for (capability, grant) in &grants {
            let alone = CapabilityContext {
                realm: realm.clone(),
                grants: [(capability.clone(), grant.clone())].into(),
                root: root.clone(),
            };
            let authority = match Authority::load(alone) {
                Ok(authority) => authority,
                Err(problem) => {
                    report.missing(&format!("{what} '{capability}'"), problem);
                    unread.insert(capability.as_str());
                    continue;
                }
            };
            let dialect = &authority.dialects[capability];
            let (provider, key) = authority
                .binding(capability)
                .expect("a grant that loaded is bound to a provider");
            let tools = grant.tools.as_ref().unwrap_or(&dialect.tools);
            let restrictions = match grant.restrictions.is_empty() {
                true => "none".to_string(),
                false => serde_json::Value::Object(grant.restrictions.clone()).to_string(),
            };
            // A declaration is not a claim of usable authority: what the
            // bound provider's adapter says of itself decides whether any
            // seat can hold the capability through this grant.
            let bound = adapters
                .as_ref()
                .and_then(|adapters| adapters.adapter(provider))
                .map(|adapter| &adapter.native);
            let unusable = match bound {
                Some(NativeInventory::Known { known, .. }) => known
                    .get(key)
                    .filter(|native| {
                        !grant.restrictions.is_empty()
                            && matches!(native.restrictions, Transport::Unsupported(_))
                    })
                    .map(|native| {
                        // What becomes of the seat that dropped the want is
                        // the OFF disposition's to say, never the grant's
                        // (finding M1): the same assessment the launch uses.
                        let dropped = match native.denial() {
                            Denial::Delivered => {
                                "one that wants it drops it with the native capability OFF"
                                    .to_string()
                            }
                            Denial::Impossible(reason) => format!(
                                "one that wants it drops it and is then refused, because the \
                                 native capability cannot be switched off ({reason})"
                            ),
                            Denial::Unmeasured(reason) => format!(
                                "one that wants it drops it and is then refused, because the \
                                 native capability's OFF control is unmeasured ({reason}) and \
                                 no denial is claimed"
                            ),
                        };
                        format!(
                            " · provider '{provider}' cannot express restriction '{}': a seat \
                             that requires the capability is refused, {dropped}, and it never \
                             runs unrestricted",
                            restriction_names("", &grant.restrictions).join("', '")
                        )
                    })
                    .unwrap_or_default(),
                Some(NativeInventory::Unmeasured(reason)) => format!(
                    " · provider '{provider}' declares its native capabilities unmeasured \
                     ({reason}): no seat can hold the capability through this grant, and no \
                     native denial is claimed"
                ),
                None => String::new(),
            };
            report.ok(
                &format!("{what} '{capability}'"),
                Safe::new(&format!(
                    "dialect '{}' ({}, provider '{provider}') · tools [{}] · {} · restrictions \
                     {restrictions}{unusable}",
                    dialect.name,
                    dialect.kind.word(),
                    tools.join(", "),
                    scope_words(grant),
                ))
                .as_str()
                .to_string(),
            );
            valid.insert(capability.as_str(), (provider.to_string(), key.to_string()));
        }
        for adapter in &installed {
            let provider = &adapter.provider;
            let natives = match &adapter.native {
                NativeInventory::Known { known, .. } => known,
                NativeInventory::Unmeasured(reason) => {
                    report.warn(
                        &format!("{what} native {provider}"),
                        format!(
                            "native inventory unmeasured: {reason}. Nothing is granted through \
                             it and no native denial is claimed"
                        ),
                    );
                    continue;
                }
            };
            for (key, native) in natives {
                let capability = &native.capability;
                let line = format!("{what} native {provider} '{capability}'");
                let granted = grants.get(capability).filter(|_| {
                    valid.get(capability.as_str()) == Some(&(provider.clone(), key.clone()))
                });
                let evidence = format!(
                    "evidence: {} · still unmeasured: {}",
                    native.evidence.scope,
                    native.evidence.limitations.join("; ")
                );
                // The OFF disposition is judged BEFORE the grant is described
                // (finding M1): however a grant is scoped it leaves seats
                // that do not hold the power, and what happens to those is
                // the launch's own assessment — a declared control denies,
                // an impossible one refuses, an unmeasured one claims
                // nothing and refuses too (ruling 4).
                let held_by = granted.map(|grant| {
                    format!(
                        "granted to {} through dialect '{}'",
                        scope_words(grant),
                        grant.dialect
                    )
                });
                match (
                    held_by,
                    unread.contains(capability.as_str()),
                    native.denial(),
                ) {
                    (Some(held_by), _, Denial::Delivered) => report.ok(
                        &line,
                        format!(
                            "{held_by}; every other seat on {provider} is launched with it \
                             switched off · {evidence}"
                        ),
                    ),
                    (Some(held_by), _, Denial::Impossible(reason)) => report.warn(
                        &line,
                        format!(
                            "{held_by}, and it cannot be switched off ({reason}): a seat on \
                             {provider} that does not hold it refuses compilation (decision \
                             0065 ruling 4)"
                        ),
                    ),
                    (Some(held_by), _, Denial::Unmeasured(reason)) => report.warn(
                        &line,
                        format!(
                            "{held_by}, and its OFF control is unmeasured ({reason}); no denial \
                             is claimed, and a seat on {provider} that does not hold it refuses \
                             compilation (decision 0065 ruling 4)"
                        ),
                    ),
                    // The realm DECLARES a grant of this name and doctor
                    // could not read it: neither half may be asserted.
                    (None, true, _) => report.warn(
                        &line,
                        format!(
                            "UNKNOWN here: this realm's grant of '{capability}' did not \
                             validate (its failing line is above), so neither a grant nor a \
                             denial is claimed, and no seat compiles in this realm until it is \
                             repaired · {evidence}"
                        ),
                    ),
                    (None, false, Denial::Impossible(reason)) => report.warn(
                        &line,
                        format!(
                            "NOT granted here, and it cannot be switched off ({reason}): \
                             seating {provider} in this realm without granting it refuses \
                             compilation (decision 0065 ruling 4)"
                        ),
                    ),
                    (None, false, Denial::Unmeasured(reason)) => report.warn(
                        &line,
                        format!(
                            "NOT granted here, and its OFF control is unmeasured ({reason}); no \
                             denial is claimed, and seating {provider} in this realm without \
                             granting it refuses compilation (decision 0065 ruling 4)"
                        ),
                    ),
                    (None, false, Denial::Delivered) => report.warn(
                        &line,
                        format!(
                            "NOT granted here: every seat on {provider} is launched with it \
                             switched off by the adapter's declared control · {evidence}"
                        ),
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
fn report_realm_house(report: &mut Report, workspace: &Path, named: Option<&Path>) {
    match brokkr_runtime::realms::World::discover(workspace, named) {
        Ok(Some(world)) => report_realm_house_for_world(report, &world),
        Ok(None) => report.ok("house rules", "no realms map; none declared".into()),
        Err(error) => report.missing("realms map", error.to_string()),
    }
}

fn report_realm_house_for_world(report: &mut Report, world: &brokkr_runtime::realms::World) {
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

/// Decision 0057's crossings, per realm, on decision 0046's Addendum's
/// terms: doctor REPORTS what `run` refuses, and refuses nothing itself.
///
/// One line per realm that draws a crossing at all — a world that never
/// drew one gets none, exactly as it writes no manifest key and prints
/// none at compile. A published file that is gone, or a pin that no
/// longer matches, is its OWN line, keyed to the failing realm and the
/// failing crossing rather than collapsing the world; the detail is the
/// refusal `run` would have given, read out of the error itself so the
/// two surfaces cannot word one fact twice.
///
/// A pin whose publisher could not publish is neither: it is its own
/// `warn`, naming the realm that owes the bytes. The realm consuming it is
/// not at fault and is not marked unhealthy for it — the publisher's line
/// already is — but it does not get the sound realm's line either, because
/// "n pin(s) matching" would claim a contract was verified against bytes
/// nobody could read.
fn report_realm_crossings(report: &mut Report, world: &brokkr_runtime::realms::World) {
    for crossings in world.crossings_report() {
        let what = format!("crossings {}", crossings.realm);
        if crossings.failures.is_empty() && crossings.unchecked.is_empty() {
            report.ok(
                &what,
                format!(
                    "{} published file(s) present, {} pin(s) matching",
                    crossings.published, crossings.consumed
                ),
            );
            continue;
        }
        for failure in &crossings.failures {
            report.missing(
                &format!("{what} '{}'", failure.crossing()),
                failure.error().to_string(),
            );
        }
        for pin in &crossings.unchecked {
            report.warn(
                &format!("{what} '{}'", pin.crossing),
                format!("pin not checked: {pin}"),
            );
        }
    }
}

/// Decision 0042 ruling 8's dialect line, with issue #218's correction:
/// the tool is probed where its `validate`/`check` step will actually
/// run. Under a boxed boundary that is a box built from the compiler's
/// own gate spec; under `harness` or `open` no box of Brokkr's stands,
/// so the host PATH is the honest surface and doctor names it. A host
/// that has the tool while the box does not is its own line, because
/// that is exactly the green check that lost a run two chief passes.
///
/// Each realm is judged under ITS OWN boundary (decision 0046 ruling 1),
/// never under the boundary of whichever realm holds the current
/// directory: a map may put an `open` realm beside a `namespace` one, and
/// answering for the reader's realm would put #218's defect back on the
/// realm axis — doctor building a box for a realm whose dialect gate is
/// refused at compile, or reading the host for a realm that will run
/// boxed.
fn report_realm_dialects(
    report: &mut Report,
    world: &brokkr_runtime::realms::World,
    workdir: &Path,
    host: fn(&str) -> Option<String>,
    inside: fn(&HandsSpec, &Path, &str) -> Result<Option<String>, String>,
) {
    let gate = brokkr_runtime::bundle::dialect_gate_hands();
    for realm in &world.map.realms {
        let boundary = realm.boundary();
        let what = format!("dialect {}", realm.name);
        let dialect = match world.dialect_for_realm(realm) {
            Ok(Some(dialect)) => dialect,
            Ok(None) => {
                report.ok(&what, "none declared".into());
                continue;
            }
            Err(error) => {
                report.missing(&what, error.to_string());
                continue;
            }
        };
        let binary = &dialect.tool.binary;
        let pinned = &dialect.tool.version;
        // The host answer is taken once whether or not it is the surface:
        // it is what tells "not found anywhere" from "found on the host,
        // unreachable in the box", the distinction this fix exists for.
        let on_host = host(binary);
        let (found, surface) = if boundary.is_boxed() {
            match inside(&gate, workdir, binary) {
                Ok(found) => (found, Surface::Box),
                Err(reason) => (on_host.clone(), Surface::NoBox(reason)),
            }
        } else {
            (on_host.clone(), Surface::Host)
        };
        match &found {
            Some(version) if version.contains(pinned) => report.ok(
                &what,
                format!(
                    "{} · tool '{binary}' {version} · pinned {pinned} · {}",
                    dialect.name,
                    surface.probed(boundary)
                ),
            ),
            Some(version) => report.warn(
                &what,
                format!(
                    "{} · tool '{binary}' {version} · pinned {pinned} (version differs) · {}",
                    dialect.name,
                    surface.probed(boundary)
                ),
            ),
            None => match (&surface, &on_host) {
                (Surface::Box, Some(_)) => report.warn(
                    &what,
                    format!(
                        "{} · tool '{binary}' present on PATH, not reachable inside the box; \
                         install under /usr/local or declare a bind · pinned {pinned}",
                        dialect.name
                    ),
                ),
                // Not found anywhere doctor could look — and the line
                // says where that was, because under a boxed boundary
                // whose box did not stand this is the host's answer to a
                // question the gate will ask of a box.
                _ => report.warn(
                    &what,
                    format!(
                        "{} · tool binary '{binary}' not found · pinned {pinned} — the design route will refuse to run · {}",
                        dialect.name,
                        surface.probed(boundary)
                    ),
                ),
            },
        }

        let realm_root = world.path_of(realm);
        for required in &dialect.requires {
            let path = realm_root.join(required);
            let required_what = format!("dialect {} requires {required}", realm.name);
            if path.is_file() {
                report.ok(&required_what, format!("present at {}", path.display()));
            } else {
                report.missing(&required_what, format!("missing at {}", path.display()));
            }
        }
    }
}

/// A composite producer that has nothing to say: the unit tests' stand-in
/// for the DSH composite reader, so a report is assembled without a
/// provider home on disk.
#[cfg(test)]
fn no_composite(adapter: &Adapter, probe: fn(&str) -> Option<String>) -> Observed {
    Observed {
        binary: adapter.binary.clone(),
        version: probe(&adapter.binary),
        warning: false,
        suffix: String::new(),
        cause: None,
    }
}

/// The machine's report in no realm: what every unit test asks, and
/// what `doctor` asks under the boundary the discovered realm declares.
#[cfg(test)]
fn doctor_with_probe(
    bundle: Option<&Path>,
    db: &Path,
    library_root: &Path,
    adapters_root: &Path,
    secrets_store: &Path,
    probe: fn(&str) -> Option<String>,
    ambient: fn(&str) -> bool,
) -> Report {
    doctor_in(
        bundle,
        db,
        library_root,
        adapters_root,
        secrets_store,
        probe,
        ambient,
        Boundary::Namespace,
        None,
        no_composite,
    )
}

/// The machine's report alone, which is what every unit test of it reads.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn doctor_in(
    bundle: Option<&Path>,
    db: &Path,
    library_root: &Path,
    adapters_root: &Path,
    secrets_store: &Path,
    probe: fn(&str) -> Option<String>,
    ambient: fn(&str) -> bool,
    boundary: Boundary,
    compiled: Option<anyhow::Result<Bundle>>,
    composite: CompositeProbe,
) -> Report {
    doctor_observed(
        bundle,
        db,
        library_root,
        adapters_root,
        secrets_store,
        probe,
        ambient,
        boundary,
        compiled,
        composite,
    )
    .0
}

#[allow(clippy::too_many_arguments)]
fn doctor_observed(
    bundle: Option<&Path>,
    db: &Path,
    library_root: &Path,
    adapters_root: &Path,
    secrets_store: &Path,
    probe: fn(&str) -> Option<String>,
    ambient: fn(&str) -> bool,
    boundary: Boundary,
    compiled: Option<anyhow::Result<Bundle>>,
    composite: CompositeProbe,
) -> (Report, Availability) {
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
    let compiled = bundle.map(|dir| {
        (
            dir,
            compiled.unwrap_or_else(|| {
                Bundle::compile_under(dir, library_root, adapters_root, boundary)
                    .map_err(Into::into)
            }),
        )
    });
    let hands: Vec<&str> = compiled
        .as_ref()
        .and_then(|(_, result)| result.as_ref().ok())
        .map(|bundle| bundle.hands.keys().map(String::as_str).collect())
        .unwrap_or_default();
    // Decision 0046 ruling 2: one line naming the boundaries a run can
    // start under here, and the `hands` line judged against the realm's
    // boundary rather than against bubblewrap alone (decision 0043's
    // consequence, generalised). The boundary is never simulated.
    let offers = boundary::offered(&probe);
    report.ok("boundaries", boundary::doctor_line(&offers));
    match boundary::hands_line(boundary, &offers[&boundary], &hands) {
        (true, line) => report.ok("hands", line),
        (false, line) => report.warn("hands", line),
    }
    // Optional: each agent CLI matters only to bundles whose seats use
    // its driver. The five-tuple that used to live here is now READ FROM
    // THE ADAPTER FILES (decision 0016), so a sixth provider shows up in
    // doctor without a rebuild — the same property that makes "adding a
    // provider is not a release" true.
    let availability = probe_providers(&mut report, adapters_root, probe, composite);
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

    // The availability rides out beside the report: the capability lines
    // (decision 0065 ruling 4) name the native powers of the harnesses
    // that were actually found, and probing each binary a second time to
    // learn that would be a second, possibly different, answer.
    (report, availability)
}

#[cfg(test)]
mod capability_tests;
#[cfg(test)]
mod tests;
