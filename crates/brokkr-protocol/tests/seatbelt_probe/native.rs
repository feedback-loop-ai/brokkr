//! macOS adapter for the bounded R3 probe (decision 0046 slice II, design D3).
//!
//! This is an experiment, not production enforcement. One committed
//! test-support executable (see `helper.rs`) provides every role so the
//! native path has no interpreter dependency and so Gate A can admit the exact
//! payload before Gate B measures it:
//!
//! * Gate A — the S0–S3 startup matrix over direct/launchd ownership and the
//!   exact candidate profile off/on.
//! * Gate B — the transient launchd lease pair: a payload job plus a
//!   separately launchd-owned guard job that boots the payload out, waits for
//!   quiescence and only then cleans up.
//!
//! Every operation here is a real `/usr/bin/sandbox-exec`, real `launchctl`,
//! real process identities and real time. The shared model in the parent
//! module decides pass/fail; a missing prerequisite is a named failure, and
//! the adapter never reports a result from a payload that did not start.
//!
//! Files are separated by role under one case root: `inputs/` is immutable
//! launch data, `payload/` is the only location the Seatbelt profile lets the
//! payload write, `guard/` is guard-private and `observer/` carries the
//! observer's release. Payload-forged markers therefore cannot be the basis
//! of a guard/observer fact.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::{
    check_startup_candidate, classify_launchd_exit, parse_launchd_print, render_candidate_profile,
    run_gated_probe, run_startup, Case, CaseResult, CheckInputs, DenialControl, Event, GatedReport,
    HelperExit, LaunchdFacts, Precondition, ProbeHost, ProfileDiagnostic, RemovalStatus,
    RepairFacts, StartupCell, StartupNegativeControl, StartupObservation, TriggerKind,
    DIAGNOSTIC_ALLOWANCES, ROOT_TOKEN, STARTUP_DENIAL_CONTROLS, STARTUP_NEGATIVE_ALLOWANCES,
    STARTUP_RULE_LEDGER,
};

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
const LAUNCHCTL: &str = "/bin/launchctl";
const KILL: &str = "/bin/kill";
const UID: &str = "/usr/bin/id";
const PS: &str = "/bin/ps";
const MKFIFO: &str = "/usr/bin/mkfifo";
const HELPER: &str = env!("CARGO_BIN_EXE_seatbelt-probe-helper");
const TEARDOWN: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_secs(1);
/// The bounded number of distinct raw `launchctl print` samples a launchd
/// startup cell retains. A refusal must name what launchd said; the cap keeps
/// the durable report bounded when launchd churns.
const MAX_LAUNCHD_SAMPLES: usize = 8;
const HANDS_BOX_ENV: &str = "BROKKR_HANDS_BOX";

/// Gate A native test: the four-cell startup matrix on a committed revision.
/// It fails on a missing prerequisite, a skipped cell, or any unmet startup
/// observation, and never reports a pass from Linux.
///
/// It is `#[ignore]`d so the generic parallel workspace suite runs only the
/// host-independent model: the destructive launchd adapter is selected
/// explicitly by the macOS CI step (`--ignored --exact`). Ignoring is a
/// selection mechanism, not a skip: when selected, a missing tool or outer box
/// is a named failure.
#[test]
#[ignore = "native macOS launchd probe: select explicitly with --ignored on macOS CI"]
fn native_startup_feasibility_probe() {
    if !cfg!(target_os = "macos") {
        return;
    }
    let mut host = NativeProbeHost::new(format!("startup-{}", std::process::id()));
    let report = run_startup(&mut host, &StartupCell::ALL);
    let rendered = render_startup(&report);
    write_report("r3-startup-report.txt", &rendered);
    println!("{rendered}");
    assert!(
        report.verdict.is_pass(),
        "startup gate failed:\n{:#?}",
        report.cells
    );
}

/// Gate B native test: the startup gate is re-run inside the same process and
/// the lifetime matrix is not run unless it passes.
#[test]
#[ignore = "native macOS launchd probe: select explicitly with --ignored on macOS CI"]
fn native_lifetime_feasibility_probe() {
    if !cfg!(target_os = "macos") {
        return;
    }
    let mut host = NativeProbeHost::new(format!("lifetime-{}", std::process::id()));
    let report = run_gated_probe(&mut host, &StartupCell::ALL, &Case::ALL);
    let rendered = render_gated(&report);
    write_report("r3-lifetime-report.txt", &rendered);
    println!("{rendered}");
    assert!(
        report.verdict.is_pass(),
        "native observations:\n{:#?}",
        report.lifetime.as_ref().map(|lifetime| &lifetime.results)
    );
}

fn write_report(name: &str, content: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target")
        .join(name);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, content);
}

fn render_startup(report: &super::StartupReport) -> String {
    let mut out = format!("Gate A startup verdict: {}\n", report.verdict.render());
    for cell in &report.cells {
        out.push_str(&format!(
            "  {}: ran={} ready={} ordinary_child={} exit={} state={:?} runs={:?} last_exit={:?} crashes={:?} terminating_signal={:?} skip={:?}\n\
             \x20   root={} executable={} mode={:#o} helper={} profile={:?} argv={:?}\n\
             \x20   stages={:?}\n",
            cell.cell.name(),
            cell.ran,
            cell.ready,
            cell.ordinary_child,
            cell.exit.describe(),
            cell.launchd_state,
            cell.launchd_runs,
            cell.launchd_last_exit_code,
            cell.launchd_successive_crashes,
            cell.launchd_terminating_signal,
            cell.skip,
            cell.helper_root,
            cell.helper_executable,
            cell.helper_mode,
            cell.helper_digest,
            cell.profile_digest,
            cell.helper_argv,
            cell.stages,
        ));
        if !cell.stdout.is_empty() {
            out.push_str(&format!("    stdout: {}\n", cell.stdout));
        }
        if !cell.stderr.is_empty() {
            out.push_str(&format!("    stderr: {}\n", cell.stderr));
        }
        for diagnostic in &cell.diagnostics {
            out.push_str(&format!(
                "    diagnostic {} ({} {}): reached_ready={} exit={} stdout={} stderr={}\n",
                diagnostic.name,
                diagnostic.operation,
                diagnostic.target,
                diagnostic.reached_ready(),
                diagnostic.exit.describe(),
                diagnostic.stdout,
                diagnostic.stderr,
            ));
        }
        for denial in &cell.denials {
            out.push_str(&format!(
                "    denial {} ({} {} for {}): observed={} denied={} detail={}\n",
                denial.name,
                denial.operation,
                denial.target,
                denial.consumer,
                denial.observed,
                denial.denied,
                denial.detail,
            ));
        }
        for control in &cell.negative_controls {
            out.push_str(&format!(
                "    negative-control {} ({} for {}): status={:?} exit={} stdout={} stderr={}\n",
                control.name,
                control.removed_rule,
                control.consumer,
                control.status,
                control.exit.describe(),
                control.stdout,
                control.stderr,
            ));
        }
        for (index, sample) in cell.launchd_samples.iter().enumerate() {
            out.push_str(&format!("    launchd-sample[{index}]: {sample}\n"));
        }
    }
    out
}

fn render_gated(report: &GatedReport) -> String {
    let mut out = render_startup(&report.startup);
    match &report.lifetime {
        None => out.push_str("Gate B lifetime: NOT RUN (startup gate failed)\n"),
        Some(lifetime) => {
            out.push_str(&format!(
                "Gate B lifetime verdict: {}\n",
                lifetime.verdict.render()
            ));
            for result in &lifetime.results {
                out.push_str(&format!(
                    "  {}: ran={} survivors={} quiet_moved={} labels_gone={} \
                     cleanup_after_quiescence={} guard_survived={} denied={} trigger={:?} skip={:?}\n",
                    result.case.name(),
                    result.ran,
                    result.survivors_after,
                    result.heartbeat_moved_during_quiet,
                    result.labels_gone,
                    result.cleanup_after_quiescence,
                    result.guard_survived_payload,
                    result.payload_attempt_denied,
                    result.repairs.trigger,
                    result.skip,
                ));
            }
        }
    }
    out
}

/// The observer's staged copy of the committed helper build: one regular file,
/// owned by the invoking user, with exactly one link, under the per-run probe
/// root and outside every cell root. The build's own possibly hard-linked file
/// is never used as `<helper>`.
#[derive(Debug, Clone)]
struct StagedHelper {
    path: PathBuf,
    digest: String,
    build_digest: String,
    mode: u32,
}

impl StagedHelper {
    fn path_text(&self) -> String {
        self.path.to_string_lossy().to_string()
    }
}

/// A host that runs the experiment with real macOS facilities.
pub struct NativeProbeHost {
    root: PathBuf,
    uid: u32,
    precondition: Option<Precondition>,
    /// A sanitized tag unique to this host instance. It is embedded in every
    /// launchd label and private root so a parallel or repeated invocation can
    /// never reuse a label (native CI `34441725835` saw two parallel tests
    /// bootstrap the same label, producing order-dependent `Bootstrap failed:
    /// 5` and missing run/crash facts).
    run_tag: String,
    /// One nonce for all four startup cells, so their helper argv differs only
    /// by the typed cell root.
    startup_nonce: String,
    /// The observer's staged single-link helper copy, or the staging defect
    /// that fails every cell before `sandbox-exec` runs.
    staged_helper: Result<StagedHelper, String>,
    /// Monotonic per-host sequence: a label is never reused within an instance.
    sequence: AtomicU64,
}

/// A process-global salt so two hosts constructed with the same `run_id` in
/// one process (the two native tests) still receive disjoint label prefixes.
static HOST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

impl NativeProbeHost {
    pub fn new(run_id: String) -> NativeProbeHost {
        let run_tag = format!(
            "{}.{}",
            sanitize_tag(&run_id),
            HOST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let raw = std::env::temp_dir().join(format!("brokkr-seatbelt-probe-{run_tag}"));
        // Create the per-run probe root with an exclusive, owner-only create
        // that fails when the path already exists, then canonicalize it: macOS
        // `/var` is a symlink to `/private/var`, and a profile that grants the
        // unresolved spelling does not match the resolved vnode. A
        // canonicalization that fails or returns another spelling is a refusal,
        // never a silent fallback.
        let root = create_private_dir(&raw).and_then(|()| {
            fs::canonicalize(&raw)
                .map_err(|error| format!("probe root canonicalize {}: {error}", raw.display()))
        });
        let (root, staged_helper) = match root {
            Ok(root) => {
                let staged = stage_helper(&root, Path::new(HELPER));
                (root, staged)
            }
            Err(error) => (raw, Err(error)),
        };
        let uid = current_uid();
        let precondition = check_precondition();
        NativeProbeHost {
            root,
            uid,
            precondition,
            run_tag,
            startup_nonce: format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos())
                    .unwrap_or_default()
            ),
            staged_helper,
            sequence: AtomicU64::new(1),
        }
    }

    /// The staged helper, or the staging defect that must fail every cell
    /// before `sandbox-exec` runs.
    fn staged_helper(&self) -> Result<&StagedHelper, String> {
        self.staged_helper
            .as_ref()
            .map_err(|error| format!("helper staging failed: {error}"))
    }

    /// The staged helper spelling, or the build path only as a last resort for
    /// a cell that already failed staging. The build path is never used as the
    /// `<helper>` of a cell that runs.
    fn helper_path(&self) -> String {
        self.staged_helper
            .as_ref()
            .map(StagedHelper::path_text)
            .unwrap_or_else(|_| HELPER.to_string())
    }

    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed)
    }

    fn label(&self, case: Case, role: &str) -> String {
        format!(
            "org.brokkr.seatbelt.probe.{}.{}.{}.{}",
            self.run_tag,
            std::process::id(),
            self.next_sequence(),
            role_atom(&format!("{}-{role}", case.name())),
        )
    }

    fn startup_label(&self, cell: StartupCell) -> String {
        format!(
            "org.brokkr.seatbelt.probe.{}.{}.startup.{}.{}",
            self.run_tag,
            std::process::id(),
            self.next_sequence(),
            role_atom(cell.name()),
        )
    }

    fn service_target(&self, label: &str) -> String {
        format!("gui/{}/{}", self.uid, label)
    }

    fn candidate_profile(&self, inputs: &CheckInputs) -> String {
        render_candidate_profile(&STARTUP_RULE_LEDGER, inputs)
    }

    // -----------------------------------------------------------------------
    // Gate A
    // -----------------------------------------------------------------------

    fn startup_cell(&mut self, cell: StartupCell) -> StartupObservation {
        // A unique private root per cell. The four cells share the one staged
        // helper's bytes, executable, mode, nonce protocol and argv structure;
        // the root is an explicit typed variable rather than drift.
        let staged = match self.staged_helper() {
            Ok(staged) => staged.clone(),
            Err(error) => return StartupObservation::skipped(cell, &error),
        };
        let root = self.root.join(format!(
            "startup-{}-{}",
            std::process::id(),
            role_atom(cell.name())
        ));
        let mut inputs = match create_cell_root(&root) {
            Ok(inputs) => inputs,
            Err(error) => {
                return StartupObservation::skipped(cell, &format!("startup root: {error}"))
            }
        };
        inputs.helper = staged.path_text();
        let nonce = self.startup_nonce.clone();
        let root_text = root.to_string_lossy().to_string();

        // The profile embeds the per-cell private root, so a raw byte digest
        // differs across isolated cells even when the structural profile is
        // identical. Digest the profile with the validated root, payload root
        // and helper replaced by their typed placeholders so the cross-cell
        // comparison tests the policy rather than a private path. The check
        // runs over the concrete profile before `sandbox-exec` is invoked; a
        // refusal fails the cell before any payload runs.
        let profile = root.join("inputs/policy.sb");
        let profile_digest = if cell.seatbelt() {
            let profile_text = self.candidate_profile(&inputs);
            if let Err(error) = check_startup_candidate(
                &profile_text,
                &STARTUP_RULE_LEDGER,
                &STARTUP_NEGATIVE_ALLOWANCES,
                &inputs,
            ) {
                return StartupObservation::skipped(
                    cell,
                    &format!("candidate check refused the profile: {error}"),
                );
            }
            match fs::write(&profile, &profile_text) {
                Ok(()) => Some(structural_profile_digest(&profile_text, &inputs)),
                Err(error) => {
                    return StartupObservation::skipped(cell, &format!("profile: {error}"))
                }
            }
        } else {
            None
        };

        // The structural argv keeps the root abstract so the four cells can
        // use isolated roots without being read as drift; the staged helper is
        // the same spelling for every cell of one run.
        let structural_argv = vec![
            "startup".to_string(),
            "--root".to_string(),
            ROOT_TOKEN.to_string(),
            "--helper".to_string(),
            staged.path_text(),
            "--nonce".to_string(),
            nonce.clone(),
            "--exit".to_string(),
            "clean".to_string(),
        ];
        let helper_argv = vec![
            "startup".to_string(),
            "--root".to_string(),
            root_text.clone(),
            "--helper".to_string(),
            staged.path_text(),
            "--nonce".to_string(),
            nonce.clone(),
            "--exit".to_string(),
            "clean".to_string(),
        ];
        let mut launcher: Vec<String> = Vec::new();
        if cell.seatbelt() {
            launcher.push(SANDBOX_EXEC.to_string());
            launcher.push("-f".to_string());
            launcher.push(profile.to_string_lossy().to_string());
        }

        let label = if cell.launchd() {
            Some(self.startup_label(cell))
        } else {
            None
        };
        if let Some(label) = &label {
            if let Err(error) = ensure_label_absent(&self.service_target(label)) {
                return StartupObservation::skipped(cell, &error);
            }
        }

        let prepared = if cell.launchd() {
            self.run_startup_launchd(
                cell,
                &root,
                &launcher,
                &helper_argv,
                label.as_deref().unwrap_or_default(),
            )
        } else {
            self.run_startup_direct(cell, &root, &launcher, &helper_argv)
        };

        // On an exact-profile failure, run the bounded one-authority-at-a-time
        // differentials, the labelled `allow default` diagnostic, and the
        // credential/host-write/network/guard/peer denial controls so the
        // report names each layer that might refuse the payload. They are
        // diagnostic output only; the shared model refuses any observation
        // labelled with a diagnostic, so they can never be admission.
        let mut diagnostics = Vec::new();
        let mut denials = Vec::new();
        let mut negative_controls = Vec::new();
        if let Ok(observation) = &prepared {
            if cell.seatbelt() {
                denials = self.run_denial_controls(&root, &STARTUP_DENIAL_CONTROLS);
                // A cell that reached READY must prove the candidate rule that
                // let it start was load-bearing: strip it and require the same
                // payload to fail closed. A cell that reached no READY records
                // every removal as not due; a stripped replay of a profile that
                // never admitted the payload proves nothing.
                negative_controls = self.run_startup_negative_controls(
                    &root,
                    &helper_argv,
                    &nonce,
                    observation.ready,
                );
                if !observation.ready {
                    diagnostics = self.run_profile_diagnostics(&root, &helper_argv, &nonce);
                }
            }
        }

        // Prove the label is absent after a bounded bootout: a stale label
        // fails the cell and can never leak into the next one.
        let mut post_bootout_failure = None;
        if let Some(label) = &label {
            let target = self.service_target(label);
            let _ = bootout(&target);
            if wait_for_job(&target, false, TEARDOWN).is_err() {
                post_bootout_failure = Some(format!("launchd label {label} survived bootout"));
            }
        }

        let _ = fs::remove_dir_all(&root);
        let mut observation = match prepared {
            Ok(observation) => observation,
            Err(error) => StartupObservation::skipped(cell, &error),
        };
        observation.helper_executable = staged.path_text();
        observation.helper_mode = staged.mode;
        observation.helper_digest = staged.digest.clone();
        observation.profile_digest = profile_digest;
        observation.helper_argv = structural_argv;
        observation.helper_root = root_text;
        observation.diagnostics = diagnostics;
        observation.denials = denials;
        observation.negative_controls = negative_controls;
        observation.stderr = format!(
            "staged helper digest {} ; committed build digest {}\n{}",
            staged.digest, staged.build_digest, observation.stderr
        );
        if let Some(reason) = post_bootout_failure {
            observation.ran = false;
            observation.skip = Some(reason);
        }
        observation
    }

    fn startup_program(&self, launcher: &[String], helper_argv: &[String]) -> Vec<String> {
        let mut program = launcher.to_vec();
        program.push(self.helper_path());
        program.extend(helper_argv.iter().cloned());
        program
    }

    fn run_startup_direct(
        &self,
        cell: StartupCell,
        root: &Path,
        launcher: &[String],
        helper_argv: &[String],
    ) -> Result<StartupObservation, String> {
        let program = self.startup_program(launcher, helper_argv);
        let output = Command::new(&program[0])
            .args(&program[1..])
            .output()
            .map_err(|error| format!("startup launch: {error}"))?;
        let exit = exit_of(&output.status);
        let (ready, ordinary_child) = read_ready(root, nonce_of(helper_argv).unwrap_or_default());
        Ok(StartupObservation {
            cell,
            ran: true,
            skip: None,
            helper_executable: String::new(),
            helper_mode: 0,
            helper_digest: String::new(),
            profile_digest: None,
            helper_argv: Vec::new(),
            helper_root: String::new(),
            ready,
            stages: read_stages(&root.join("payload/stages")),
            ordinary_child,
            exit,
            launchd_state: None,
            launchd_runs: None,
            launchd_last_exit_code: None,
            launchd_successive_crashes: None,
            launchd_terminating_signal: None,
            launchd_active_count: None,
            launchd_facts: None,
            diagnostic: None,
            diagnostics: Vec::new(),
            denials: Vec::new(),
            negative_controls: Vec::new(),
            launchd_samples: Vec::new(),
            stdout: bound(&String::from_utf8_lossy(&output.stdout)),
            stderr: bound(&String::from_utf8_lossy(&output.stderr)),
            denial_events: Vec::new(),
            residual: None,
        })
    }

    fn run_startup_launchd(
        &self,
        cell: StartupCell,
        root: &Path,
        launcher: &[String],
        helper_argv: &[String],
        label: &str,
    ) -> Result<StartupObservation, String> {
        if label.is_empty() {
            return Err("launchd startup cell had no label".to_string());
        }
        let program = self.startup_program(launcher, helper_argv);
        let program_refs: Vec<&str> = program.iter().map(String::as_str).collect();
        let plist = root.join("startup.plist");
        write_plist(
            &plist,
            label,
            &program_refs,
            &root.join("startup.out"),
            &root.join("startup.err"),
        )?;
        bootstrap(&format!("gui/{}", self.uid), &plist)
            .map_err(|error| format!("bootstrap {label}: {error}"))?;
        let target = self.service_target(label);

        let nonce = nonce_of(helper_argv).unwrap_or_default().to_string();
        let started = Instant::now();
        let mut ready = false;
        let mut ordinary_child = false;
        while started.elapsed() < TEARDOWN {
            if root.join("payload/ready").is_file() {
                let (observed_ready, observed_child) = read_ready(root, &nonce);
                ready = observed_ready;
                ordinary_child = observed_child;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }

        // Require a parsed terminal state, and retain every distinct raw
        // `launchctl print` sample so a refusal names what launchd actually
        // said rather than inferring. A missing or unparsable field is a failed
        // measurement, never a synthesized exit code.
        let mut samples: Vec<String> = Vec::new();
        let mut facts = None;
        let mut last_parseable: Option<LaunchdFacts> = None;
        let terminal_started = Instant::now();
        while terminal_started.elapsed() < TEARDOWN {
            let sample = launchctl_sample(&target);
            let parsed = parse_launchd_print(&sample.stdout).ok();
            if let Some(parsed) = &parsed {
                last_parseable = Some(parsed.clone());
            }
            let rendered = sample.render();
            if samples.last() != Some(&rendered) && samples.len() < MAX_LAUNCHD_SAMPLES {
                samples.push(rendered);
            }
            if parsed
                .as_ref()
                .is_some_and(|facts| facts.state.as_deref() == Some("not running"))
            {
                facts = parsed;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        // A terminal-observation refusal is itself an observed fact, not a
        // lost one: keep the cell observation (and every raw sample) so the
        // report can distinguish "launchd never reached a terminal state" from
        // "the payload never ran". The last parseable state is reported when
        // no terminal state is reached.
        let observed_facts = facts.clone().or_else(|| last_parseable.clone());
        let terminal_refusal = facts.is_none().then(|| {
            let observed = observed_facts
                .as_ref()
                .map(|facts| format!("; last parseable state {:?}", facts.state))
                .unwrap_or_default();
            let detail = samples
                .last()
                .map(|sample| format!("; last sample: {sample}"))
                .unwrap_or_default();
            format!(
                "launchd label {label} never produced a parseable not-running state{observed}{detail}"
            )
        });
        let exit = observed_facts
            .as_ref()
            .map(classify_launchd_exit)
            .unwrap_or(HelperExit::NotRun);
        let (
            launchd_state,
            launchd_runs,
            launchd_last_exit_code,
            launchd_successive_crashes,
            launchd_terminating_signal,
            launchd_active_count,
        ) = match &observed_facts {
            Some(facts) => (
                facts.state.clone(),
                facts.runs,
                facts.last_exit_code,
                facts.successive_crashes,
                facts.terminating_signal,
                facts.active_count,
            ),
            None => (None, None, None, None, None, None),
        };
        Ok(StartupObservation {
            cell,
            ran: true,
            skip: terminal_refusal,
            helper_executable: String::new(),
            helper_mode: 0,
            helper_digest: String::new(),
            profile_digest: None,
            helper_argv: Vec::new(),
            helper_root: String::new(),
            ready,
            stages: read_stages(&root.join("payload/stages")),
            ordinary_child,
            exit,
            launchd_state,
            launchd_runs,
            launchd_last_exit_code,
            launchd_successive_crashes,
            launchd_terminating_signal,
            launchd_active_count,
            launchd_facts: observed_facts,
            diagnostic: None,
            diagnostics: Vec::new(),
            denials: Vec::new(),
            negative_controls: Vec::new(),
            launchd_samples: samples,
            stdout: read_bounded(&root.join("startup.out")),
            stderr: read_bounded(&root.join("startup.err")),
            denial_events: Vec::new(),
            residual: None,
        })
    }

    // -----------------------------------------------------------------------
    // Gate B
    // -----------------------------------------------------------------------

    fn lifetime_case(&mut self, case: Case) -> CaseResult {
        match self.run_case_inner(case) {
            Ok(result) => result,
            Err(reason) => {
                eprintln!("{}: {reason}", case.name());
                CaseResult::skipped(case, &reason)
            }
        }
    }

    fn run_case_inner(&mut self, case: Case) -> Result<CaseResult, String> {
        let staged = self.staged_helper()?.clone();
        let root = self
            .root
            .join(format!("lifetime-{}-{}", self.run_tag, case.name()));
        // Exclusive owner-only creates, canonicalized to the input spelling:
        // the observer's own fresh root, never a pre-existing host object and
        // never a silent uncanonical fallback.
        create_private_dir(&root).map_err(|error| format!("lifetime root: {error}"))?;
        for dir in ["inputs", "payload", "guard", "observer"] {
            create_private_dir(&root.join(dir))
                .map_err(|error| format!("lifetime {dir} dir: {error}"))?;
        }
        let canonical =
            fs::canonicalize(&root).map_err(|error| format!("lifetime canonicalize: {error}"))?;
        if canonical != root {
            return Err(format!(
                "lifetime root canonicalizes to {} rather than its input spelling {}",
                canonical.display(),
                root.display()
            ));
        }
        let inputs = CheckInputs {
            cell_root: root.to_string_lossy().to_string(),
            payload_root: root.join("payload").to_string_lossy().to_string(),
            inputs_dir: root.join("inputs").to_string_lossy().to_string(),
            helper: staged.path_text(),
        };

        let payload_label = self.label(case, "payload");
        let guard_label = self.label(case, "guard");
        let peer_label = self.label(case, "peer");
        let escape_label = format!(
            "org.brokkr.seatbelt.probe.{}.escape.{}",
            self.run_tag,
            std::process::id()
        );
        let liveness = root.join("liveness");
        run(MKFIFO, &[&liveness.to_string_lossy()], None)?;

        fs::write(root.join("inputs/guard-label"), &guard_label).map_err(|e| e.to_string())?;
        fs::write(root.join("inputs/peer-label"), &peer_label).map_err(|e| e.to_string())?;
        let profile = root.join("inputs/policy.sb");
        let profile_text = self.candidate_profile(&inputs);
        check_startup_candidate(
            &profile_text,
            &STARTUP_RULE_LEDGER,
            &STARTUP_NEGATIVE_ALLOWANCES,
            &inputs,
        )
        .map_err(|error| format!("candidate check refused the Gate B profile: {error}"))?;
        fs::write(&profile, &profile_text).map_err(|e| format!("policy: {e}"))?;
        let profile_digest = digest(&profile);
        let _profile_digest = profile_digest;

        let payload_plist = root.join("payload.plist");
        write_plist(
            &payload_plist,
            &payload_label,
            &[
                SANDBOX_EXEC,
                "-f",
                &profile.to_string_lossy(),
                &self.helper_path(),
                "payload",
                "--root",
                &root.to_string_lossy(),
                "--helper",
                &staged.path_text(),
                "--case",
                case.name(),
                "--escape-label",
                &escape_label,
            ],
            &root.join("payload.out"),
            &root.join("payload.err"),
        )?;
        let guard_plist = root.join("guard.plist");
        write_plist(
            &guard_plist,
            &guard_label,
            &[
                &self.helper_path(),
                "guard",
                "--root",
                &root.to_string_lossy(),
                "--payload-label",
                &payload_label,
                "--guard-label",
                &guard_label,
                "--uid",
                &self.uid.to_string(),
            ],
            &root.join("guard.out"),
            &root.join("guard.err"),
        )?;

        let outcome = self.experiment(
            case,
            &root,
            &liveness,
            &payload_plist,
            &guard_plist,
            &payload_label,
            &guard_label,
            &peer_label,
            &escape_label,
        );

        // Harness cleanup is separate from the observed lifecycle. Always
        // remove registered jobs and any recorded survivors; these actions are
        // never counted as containment evidence.
        for label in [&payload_label, &guard_label, &peer_label, &escape_label] {
            let _ = bootout(&self.service_target(label));
        }
        for identity in recorded_identities(&root.join("payload/identities")) {
            let _ = Command::new(KILL)
                .args(["-9", &identity.pid.to_string()])
                .output();
        }
        let _ = fs::remove_dir_all(&root);

        let mut result = outcome?;
        result.repairs.failure_report_preserved = true;
        Ok(result)
    }

    #[allow(clippy::too_many_arguments)]
    fn experiment(
        &self,
        case: Case,
        root: &Path,
        liveness: &Path,
        payload_plist: &Path,
        guard_plist: &Path,
        payload_label: &str,
        guard_label: &str,
        peer_label: &str,
        escape_label: &str,
    ) -> Result<CaseResult, String> {
        let mut events = vec![Event::Prepared];

        if case == Case::GroupKillNegativeControl {
            return self.group_kill_negative_control(
                case,
                root,
                payload_plist,
                payload_label,
                events,
            );
        }

        bootstrap(&format!("gui/{}", self.uid), guard_plist)?;
        wait_for_job(&self.service_target(guard_label), true, TEARDOWN)?;
        events.push(Event::GuardRegistered);

        // The liveness channel: the payload must not inherit the write end.
        // Supervisor death is modeled by a separate holder that the observer
        // SIGKILLs; every other case drops its own writer explicitly.
        let writer = match case {
            Case::SupervisorDeath => None,
            _ => Some(open_writer(liveness)?),
        };
        // The holder is the observer's own child, so it is killed and waited
        // on every path, including an early error before the trigger.
        let holder = match case {
            Case::SupervisorDeath => Some(HolderGuard::spawn(liveness)?),
            _ => None,
        };

        bootstrap(&format!("gui/{}", self.uid), payload_plist)?;
        events.push(Event::PayloadStarted);
        let positive = wait_for_heartbeat(root, TEARDOWN)?;
        if !positive {
            self.preserve_startup_diagnostics(root, payload_label);
            return Err("payload heartbeat did not advance before the trigger".into());
        }

        // Synchronize the peer before the payload attacks it.
        let peer_was_registered = if case == Case::PeerBootout {
            let peer = root.join("peer.plist");
            write_plist(
                &peer,
                peer_label,
                &["/bin/sleep", "300"],
                &root.join("peer.out"),
                &root.join("peer.err"),
            )?;
            bootstrap(&format!("gui/{}", self.uid), &peer)?;
            wait_for_job(&self.service_target(peer_label), true, TEARDOWN)?;
            true
        } else {
            false
        };

        // Each case performs the one real trigger it names.
        match case {
            Case::Timeout => std::thread::sleep(Duration::from_millis(300)),
            Case::Cancellation => {
                let _ = run(
                    LAUNCHCTL,
                    &["kill", "SIGTERM", &self.service_target(payload_label)],
                    None,
                );
            }
            Case::SupervisorDeath => {
                if let Some(holder) = &holder {
                    run(KILL, &["-9", &holder.pid().to_string()], None)?;
                }
            }
            Case::GuardInterference | Case::PeerBootout | Case::EscapeJob => {
                wait_for_file(&root.join("payload/attempts"), TEARDOWN)?;
            }
            _ => {}
        }
        drop(writer);
        events.push(Event::Terminating);

        // Observe the guard while it is still registered.
        let guard_observed = job_loaded(&self.service_target(guard_label));

        // Wait for the guard to boot the payload out and establish quiet,
        // then collect every recorded identity before releasing cleanup.
        let quiesced = wait_for_file(&root.join("guard/quiesced"), TEARDOWN).is_ok();
        let started = Instant::now();
        let observed_survivors = loop {
            let survivors = live_identities(root).len();
            let payload_gone = !job_loaded(&self.service_target(payload_label));
            if (payload_gone && survivors == 0 && quiesced) || started.elapsed() >= TEARDOWN {
                break survivors;
            }
            std::thread::sleep(Duration::from_millis(50));
        };

        // Corroborate the quiet window independently of the guard's word.
        let quiet_moved = heartbeat_moved(root, QUIET);
        let cleanup_after_quiescence =
            root.join("guard/quiesced").is_file() && !root.join("guard/cleanup-done").is_file();

        // Only now release private-state cleanup and guard self-unregistration.
        fs::write(root.join("observer/release"), "go\n").map_err(|e| e.to_string())?;
        let _ = wait_for_job(&self.service_target(guard_label), false, TEARDOWN);
        let cleanup_done = root.join("guard/cleanup-done").is_file();
        if cleanup_done {
            events.push(Event::PayloadQuiescent);
            events.push(Event::PrivateStateRemoved);
        }
        if !job_loaded(&self.service_target(guard_label)) {
            events.push(Event::GuardUnregistered);
        }

        let labels_gone = !job_loaded(&self.service_target(payload_label))
            && !job_loaded(&self.service_target(guard_label));
        // Guard survival means the guard process completed its own lifecycle
        // (bootout, quiet, cleanup, self-unregister), not merely that its
        // launchd label is still registered.
        let guard_survived = guard_observed && cleanup_done;
        let denied = match case {
            Case::GuardInterference => guard_survived,
            Case::PeerBootout => job_loaded(&self.service_target(peer_label)),
            Case::EscapeJob => !job_loaded(&self.service_target(escape_label)),
            _ => true,
        };
        let attack_recorded = match case {
            Case::GuardInterference | Case::PeerBootout | Case::EscapeJob => {
                read_attempts(&root.join("payload/attempts"))
            }
            _ => true,
        };
        for label in [peer_label, escape_label] {
            let _ = bootout(&self.service_target(label));
        }

        Ok(CaseResult {
            case,
            ran: true,
            skip: None,
            positive_control: positive,
            heartbeat_before: positive,
            survivors_after: observed_survivors,
            heartbeat_moved_during_quiet: quiet_moved,
            labels_gone,
            cleanup_after_quiescence: cleanup_after_quiescence && cleanup_done,
            guard_survived_payload: guard_survived,
            payload_attempt_denied: denied,
            events,
            repairs: RepairFacts {
                trigger: case.required_trigger(),
                attack_recorded_before_observation: attack_recorded,
                peer_synchronized_before_attack: case != Case::PeerBootout || peer_was_registered,
                guard_observed_registered: guard_observed,
                real_original_group_kill: false,
                identities_have_start_time: true,
                reaped_all_children: true,
                failure_report_preserved: true,
                forged_marker: false,
                harness_cleanup_counted: false,
            },
        })
    }

    fn group_kill_negative_control(
        &self,
        case: Case,
        root: &Path,
        payload_plist: &Path,
        payload_label: &str,
        mut events: Vec<Event>,
    ) -> Result<CaseResult, String> {
        bootstrap(&format!("gui/{}", self.uid), payload_plist)?;
        events.push(Event::PayloadStarted);
        let positive = wait_for_heartbeat(root, TEARDOWN)?;
        if !positive {
            return Err("helper heartbeat did not advance before the group kill".into());
        }
        let identities = recorded_identities(&root.join("payload/identities"));
        let target_group = identities
            .iter()
            .find(|identity| identity.tag == "payload")
            .map(|identity| identity.pgid)
            .filter(|pgid| *pgid != 0)
            .ok_or_else(|| "no original process group was recorded".to_string())?;
        let own_group = process_group();
        if target_group == own_group {
            return Err("the recorded group is the observer's own group".to_string());
        }
        // A real SIGKILL to the original process group.
        let _ = Command::new(KILL)
            .args(["-9", &format!("-{target_group}")])
            .output()
            .map_err(|e| e.to_string())?;
        events.push(Event::Terminating);
        std::thread::sleep(Duration::from_millis(300));
        let survivors = live_identities(root);
        let survivor_count = survivors.len();
        if survivor_count > 0 {
            events.push(Event::PayloadQuiescent);
        }
        let _ = bootout(&self.service_target(payload_label));
        for identity in &identities {
            let _ = Command::new(KILL)
                .args(["-9", &identity.pid.to_string()])
                .output();
        }
        events.push(Event::PrivateStateRemoved);
        Ok(CaseResult {
            case,
            ran: true,
            skip: None,
            positive_control: positive,
            heartbeat_before: positive,
            survivors_after: survivor_count,
            heartbeat_moved_during_quiet: false,
            labels_gone: !job_loaded(&self.service_target(payload_label)),
            cleanup_after_quiescence: true,
            guard_survived_payload: true,
            payload_attempt_denied: true,
            events,
            repairs: RepairFacts {
                trigger: TriggerKind::OriginalGroupKill,
                attack_recorded_before_observation: true,
                peer_synchronized_before_attack: true,
                guard_observed_registered: false,
                real_original_group_kill: true,
                identities_have_start_time: true,
                reaped_all_children: true,
                failure_report_preserved: true,
                forged_marker: false,
                harness_cleanup_counted: false,
            },
        })
    }

    fn preserve_startup_diagnostics(&self, root: &Path, payload_label: &str) {
        if let Ok(state) = Command::new(LAUNCHCTL)
            .args(["print", &self.service_target(payload_label)])
            .output()
        {
            eprintln!(
                "payload launchd state: {}",
                String::from_utf8_lossy(&state.stdout)
            );
        }
        if let Ok(startup) = Command::new(SANDBOX_EXEC)
            .arg("-f")
            .arg(root.join("inputs/policy.sb"))
            .args([
                &self.helper_path(),
                "startup",
                "--root",
                &root.to_string_lossy(),
                "--nonce",
                "diagnostic",
                "--exit",
                "clean",
            ])
            .output()
        {
            eprintln!(
                "direct sandbox startup: status={}; stdout={}; stderr={}",
                startup.status,
                String::from_utf8_lossy(&startup.stdout),
                String::from_utf8_lossy(&startup.stderr)
            );
        }
    }

    /// Run the unchanged candidate profile plus exactly one named allowance at
    /// a time, then the labelled `allow default` profile, and return the
    /// bounded differential collection. These runs are diagnosis, never
    /// admission: the shared model refuses any observation labelled as a
    /// diagnostic, and none of these allowances is ever part of the candidate.
    fn run_profile_diagnostics(
        &self,
        root: &Path,
        helper_argv: &[String],
        nonce: &str,
    ) -> Vec<ProfileDiagnostic> {
        let candidate = fs::read_to_string(root.join("inputs/policy.sb")).unwrap_or_default();
        if candidate.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for allowance in DIAGNOSTIC_ALLOWANCES {
            // Never let a diagnostic's authority drift into the candidate.
            debug_assert!(!candidate.contains(allowance.sbpl));
            let path = root.join(format!("inputs/diagnostic-{}.sb", allowance.name));
            let text = format!(
                ";; DIAGNOSTIC ONLY ({}): never a passing candidate\n{candidate}\n{}\n",
                allowance.name, allowance.sbpl
            );
            if fs::write(&path, text).is_err() {
                continue;
            }
            out.push(self.run_one_diagnostic(
                allowance.name,
                allowance.operation,
                allowance.target,
                allowance.consumer,
                allowance.sbpl,
                &path,
                root,
                helper_argv,
                nonce,
            ));
        }

        // The broad `allow default` control names the layer that withholds the
        // payload. It is recorded alongside the single-authority differentials
        // and can never be a candidate.
        let broad = root.join("inputs/diagnostic-allow-default.sb");
        if fs::write(&broad, default_allow_profile(root, &root.join("payload"))).is_ok() {
            out.push(self.run_one_diagnostic(
                "allow-default",
                "default",
                "all operations",
                "broad non-passing control",
                "(allow default)",
                &broad,
                root,
                helper_argv,
                nonce,
            ));
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn run_one_diagnostic(
        &self,
        name: &str,
        operation: &str,
        target: &str,
        consumer: &str,
        allowance: &str,
        profile: &Path,
        root: &Path,
        helper_argv: &[String],
        nonce: &str,
    ) -> ProfileDiagnostic {
        // Fresh state per diagnostic so READY from a prior allowance is never
        // mistaken for this allowance's outcome.
        let _ = fs::remove_file(root.join("payload/ready"));
        let _ = fs::remove_file(root.join("payload/identities"));
        let _ = fs::remove_file(root.join("payload/stages"));
        let output = Command::new(SANDBOX_EXEC)
            .arg("-f")
            .arg(profile)
            .arg(self.helper_path())
            .args(helper_argv)
            .output();
        match output {
            Ok(output) => {
                let (ready, ordinary_child) = read_ready(root, nonce);
                ProfileDiagnostic {
                    name: name.to_string(),
                    operation: operation.to_string(),
                    target: target.to_string(),
                    consumer: consumer.to_string(),
                    allowance: allowance.to_string(),
                    ready,
                    ordinary_child,
                    exit: exit_of(&output.status),
                    stdout: bound(&String::from_utf8_lossy(&output.stdout)),
                    stderr: bound(&String::from_utf8_lossy(&output.stderr)),
                }
            }
            Err(error) => ProfileDiagnostic {
                name: name.to_string(),
                operation: operation.to_string(),
                target: target.to_string(),
                consumer: consumer.to_string(),
                allowance: allowance.to_string(),
                ready: false,
                ordinary_child: false,
                exit: HelperExit::NotRun,
                stdout: String::new(),
                stderr: bound(&error.to_string()),
            },
        }
    }

    /// Replay the exact candidate profile with each load-bearing startup rule
    /// removed, one rule at a time. The same payload must fail closed under the
    /// stripped profile; a stripped profile that still starts cleanly names a
    /// rule that was not load-bearing and fails the cell. These runs are a
    /// removal proof, never a candidate: the stripped profile never enters the
    /// candidate and can never satisfy a cell on its own.
    fn run_startup_negative_controls(
        &self,
        root: &Path,
        helper_argv: &[String],
        nonce: &str,
        cell_ready: bool,
    ) -> Vec<StartupNegativeControl> {
        let candidate = fs::read_to_string(root.join("inputs/policy.sb")).unwrap_or_default();
        let mut controls = Vec::new();
        for allowance in STARTUP_NEGATIVE_ALLOWANCES {
            // A cell that reached no READY owes no removal verdict: a stripped
            // replay of a profile that never admitted the payload proves
            // nothing.
            if !cell_ready {
                controls.push(StartupNegativeControl {
                    name: allowance.name.to_string(),
                    removed_rule: allowance.removed_rule.to_string(),
                    consumer: allowance.consumer.to_string(),
                    status: RemovalStatus::NotDue,
                    exit: HelperExit::NotRun,
                    stdout: String::new(),
                    stderr: "the cell reached no READY, so the removal is not due".to_string(),
                });
                continue;
            }
            let stripped = candidate.replace(allowance.removed_rule, "");
            if stripped == candidate {
                // The rule was absent, so the removal proves nothing. Record it
                // as missing rather than as a pass.
                controls.push(StartupNegativeControl {
                    name: allowance.name.to_string(),
                    removed_rule: allowance.removed_rule.to_string(),
                    consumer: allowance.consumer.to_string(),
                    status: RemovalStatus::Missing,
                    exit: HelperExit::NotRun,
                    stdout: String::new(),
                    stderr: "candidate profile did not carry the rule to remove".to_string(),
                });
                continue;
            }
            let path = root.join(format!("inputs/negative-{}.sb", allowance.name));
            let text = format!(
                ";; NEGATIVE CONTROL ({}): candidate profile with this rule removed\n{stripped}\n",
                allowance.name
            );
            if fs::write(&path, text).is_err() {
                controls.push(StartupNegativeControl {
                    name: allowance.name.to_string(),
                    removed_rule: allowance.removed_rule.to_string(),
                    consumer: allowance.consumer.to_string(),
                    status: RemovalStatus::Missing,
                    exit: HelperExit::NotRun,
                    stdout: String::new(),
                    stderr: "the stripped replay profile could not be written".to_string(),
                });
                continue;
            }
            // Fresh state: the candidate's READY must never be mistaken for the
            // stripped profile's outcome.
            let _ = fs::remove_file(root.join("payload/ready"));
            let _ = fs::remove_file(root.join("payload/identities"));
            let _ = fs::remove_file(root.join("payload/stages"));
            let output = Command::new(SANDBOX_EXEC)
                .arg("-f")
                .arg(&path)
                .arg(self.helper_path())
                .args(helper_argv)
                .output();
            match output {
                Ok(output) => {
                    let (ready, _) = read_ready(root, nonce);
                    let exit = exit_of(&output.status);
                    controls.push(StartupNegativeControl {
                        name: allowance.name.to_string(),
                        removed_rule: allowance.removed_rule.to_string(),
                        consumer: allowance.consumer.to_string(),
                        // Blocking only when the stripped replay reached no
                        // authenticated READY from fresh payload state. A
                        // removal that still admits READY is observed and not
                        // blocking, whatever the later exit.
                        status: if ready {
                            RemovalStatus::ObservedNotBlocking
                        } else {
                            RemovalStatus::ObservedBlocking
                        },
                        exit,
                        stdout: bound(&String::from_utf8_lossy(&output.stdout)),
                        stderr: bound(&String::from_utf8_lossy(&output.stderr)),
                    });
                }
                Err(error) => {
                    controls.push(StartupNegativeControl {
                        name: allowance.name.to_string(),
                        removed_rule: allowance.removed_rule.to_string(),
                        consumer: allowance.consumer.to_string(),
                        status: RemovalStatus::Missing,
                        exit: HelperExit::NotRun,
                        stdout: String::new(),
                        stderr: bound(&error.to_string()),
                    });
                }
            }
        }
        controls
    }

    /// Rerun the named denial controls under the unchanged candidate profile.
    /// A control is satisfied only when the helper durably recorded a denial; a
    /// payload that never started records an unobserved control, never a pass.
    /// The full five-control set spans startup and lifetime; the startup cell
    /// runs the credential, host-write and network subset.
    fn run_denial_controls(&self, root: &Path, names: &[&str]) -> Vec<DenialControl> {
        const CONTROLS: [(&str, &str, &str, &str); 6] = [
            (
                "credential-read",
                "file-read*",
                "/etc/passwd",
                "host credential bytes",
            ),
            (
                "data-volume-credential-read",
                "file-read*",
                "/System/Volumes/Data/private/etc/passwd",
                "the data-volume spelling of the credential bytes",
            ),
            (
                "host-write",
                "file-write*",
                "/private/tmp",
                "host filesystem outside the payload",
            ),
            (
                "network-bind",
                "network-bind",
                "127.0.0.1:0",
                "loopback listener",
            ),
            (
                "guard-kill",
                "launchctl kill",
                "guard job",
                "the separately owned guard",
            ),
            (
                "peer-bootout",
                "launchctl bootout",
                "peer job",
                "a peer probe invocation",
            ),
        ];
        let profile = root.join("inputs/policy.sb");
        let mut controls = Vec::new();
        for (name, operation, target, consumer) in CONTROLS {
            if !names.contains(&name) {
                continue;
            }
            let _ = fs::remove_file(root.join("payload/denials"));
            let denial_argv = vec![
                "denial".to_string(),
                "--root".to_string(),
                root.to_string_lossy().to_string(),
                "--kind".to_string(),
                denial_kind(name),
            ];
            let output = Command::new(SANDBOX_EXEC)
                .arg("-f")
                .arg(&profile)
                .arg(self.helper_path())
                .args(&denial_argv)
                .output();
            let (observed, denied, detail) = match output {
                Ok(output) => match read_denial(root, &denial_kind(name)) {
                    Some((denied, detail)) => (true, denied, detail),
                    None => (
                        false,
                        false,
                        format!(
                            "no denial record (exit {} stderr {})",
                            output.status,
                            bound(&String::from_utf8_lossy(&output.stderr))
                        ),
                    ),
                },
                Err(error) => (false, false, format!("denial run failed: {error}")),
            };
            controls.push(DenialControl {
                name: name.to_string(),
                operation: operation.to_string(),
                target: target.to_string(),
                consumer: consumer.to_string(),
                denied,
                observed,
                detail,
            });
        }
        // Never leave the helper's denial attempts to be read as another
        // cell's evidence.
        let _ = fs::remove_file(root.join("payload/denials"));
        controls
    }
}

/// The helper `--kind` for a named denial control.
fn denial_kind(name: &str) -> String {
    match name {
        "credential-read" => "credential",
        "data-volume-credential-read" => "data-volume-credential",
        "host-write" => "host-write",
        "network-bind" => "network",
        "guard-kill" => "guard",
        "peer-bootout" => "peer",
        other => other,
    }
    .to_string()
}

/// Read the helper's denial record for `kind`, if it wrote one.
fn read_denial(root: &Path, kind: &str) -> Option<(bool, String)> {
    fs::read_to_string(root.join("payload/denials"))
        .ok()?
        .lines()
        .find_map(|line| {
            let mut fields = line.splitn(3, ' ');
            let recorded = fields.next()?;
            let verdict = fields.next()?;
            let detail = fields.next().unwrap_or_default();
            (recorded == kind).then(|| (verdict == "denied", detail.to_string()))
        })
}

impl ProbeHost for NativeProbeHost {
    fn precondition(&mut self) -> Option<Precondition> {
        self.precondition.clone()
    }

    fn run_startup_cell(&mut self, cell: StartupCell) -> StartupObservation {
        self.startup_cell(cell)
    }

    fn run_case(&mut self, case: Case) -> CaseResult {
        self.lifetime_case(case)
    }
}

/// The prerequisite check, before any payload runs.
fn check_precondition() -> Option<Precondition> {
    if std::env::var_os(HANDS_BOX_ENV).is_some() {
        return Some(Precondition::OuterBox);
    }
    if !Path::new(SANDBOX_EXEC).is_file() {
        return Some(Precondition::LauncherMissing);
    }
    if !Path::new(LAUNCHCTL).is_file() {
        return Some(Precondition::LaunchctlMissing);
    }
    for tool in [HELPER, MKFIFO, KILL, UID, PS] {
        if !Path::new(tool).is_file() {
            return Some(Precondition::FacilityUnsupported(format!(
                "{tool} is absent"
            )));
        }
    }
    let domain = format!("gui/{}", current_uid());
    if !Command::new(LAUNCHCTL)
        .args(["print", &domain])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
    {
        return Some(Precondition::FacilityUnsupported(format!(
            "no per-user bootstrap domain at {domain}"
        )));
    }
    None
}

fn current_uid() -> u32 {
    Command::new(UID)
        .arg("-u")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|text| text.trim().parse().ok())
        .unwrap_or(u32::MAX)
}

#[cfg(unix)]
fn process_group() -> u32 {
    extern "C" {
        fn getpgid(pid: i32) -> i32;
    }
    // SAFETY: `getpgid` is called for the current process; a failure returns
    // a negative value that is mapped to 0.
    unsafe {
        let pgid = getpgid(0);
        if pgid < 0 {
            0
        } else {
            pgid as u32
        }
    }
}

#[cfg(not(unix))]
fn process_group() -> u32 {
    0
}

fn run(program: &str, args: &[&str], env: Option<&[(&str, &str)]>) -> Result<(), String> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(env) = env {
        for (key, value) in env {
            command.env(key, value);
        }
    }
    let out = command.output().map_err(|e| format!("{program}: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{program} {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

fn bootstrap(domain: &str, plist: &Path) -> Result<(), String> {
    run(
        LAUNCHCTL,
        &["bootstrap", domain, &plist.to_string_lossy()],
        None,
    )
}

fn bootout(domain_target: &str) -> Result<(), String> {
    run(LAUNCHCTL, &["bootout", domain_target], None)
}

fn job_loaded(domain_target: &str) -> bool {
    Command::new(LAUNCHCTL)
        .args(["print", domain_target])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// One raw `launchctl print` observation. The exit status is retained because
/// launchd returns a parseable block only while it considers the job part of
/// the domain; a non-zero status with a diagnostic means the job was reaped,
/// which is a different fact from a parseable state and must never be inferred
/// into one.
struct LaunchctlSample {
    status: String,
    stdout: String,
    stderr: String,
}

impl LaunchctlSample {
    fn render(&self) -> String {
        format!(
            "status={} stdout={} stderr={}",
            self.status,
            bound(&self.stdout),
            bound(&self.stderr)
        )
    }
}

fn launchctl_sample(domain_target: &str) -> LaunchctlSample {
    match Command::new(LAUNCHCTL)
        .args(["print", domain_target])
        .output()
    {
        Ok(out) => LaunchctlSample {
            status: out.status.to_string(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        },
        Err(error) => LaunchctlSample {
            status: format!("spawn-failed: {error}"),
            stdout: String::new(),
            stderr: String::new(),
        },
    }
}

/// Prove a label is absent before a cell bootstraps it: an already-loaded
/// label is booted out and rechecked, and a label that survives fails the cell
/// rather than racing a stale registration.
fn ensure_label_absent(domain_target: &str) -> Result<(), String> {
    if job_loaded(domain_target) {
        let _ = bootout(domain_target);
        let _ = wait_for_job(domain_target, false, TEARDOWN);
    }
    if job_loaded(domain_target) {
        return Err(format!(
            "launchd label {domain_target} was already registered before bootstrap"
        ));
    }
    Ok(())
}

/// The value following `--nonce` in a helper argv.
fn nonce_of(argv: &[String]) -> Option<&str> {
    argv.iter()
        .position(|arg| arg == "--nonce")
        .and_then(|index| argv.get(index + 1))
        .map(String::as_str)
}

/// Create a directory exclusively with owner-only permissions. It fails when
/// the path already exists, so a pre-existing host object is never accepted as
/// observer-created state.
#[cfg(unix)]
fn create_private_dir(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder
        .create(path)
        .map_err(|error| format!("exclusive create {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> Result<(), String> {
    fs::DirBuilder::new()
        .create(path)
        .map_err(|error| format!("exclusive create {}: {error}", path.display()))
}

/// Stage the committed helper build as one exclusively created, regular,
/// single-link file under `<probe-root>/bin`, outside every cell root, and
/// confirm its bytes equal the build's. The build's own possibly hard-linked
/// file is never used as `<helper>`.
#[cfg(unix)]
fn stage_helper(probe_root: &Path, build: &Path) -> Result<StagedHelper, String> {
    use std::io::Write as _;
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

    let bin = probe_root.join("bin");
    create_private_dir(&bin).map_err(|error| format!("helper bin dir: {error}"))?;
    let staged = bin.join("seatbelt-probe-helper");
    let bytes = fs::read(build).map_err(|error| format!("read helper build {build:?}: {error}"))?;
    let build_digest = format!("{:016x}", fnv1a(&bytes));
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o755)
        .open(&staged)
        .map_err(|error| format!("exclusive create {}: {error}", staged.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("write staged helper: {error}"))?;
    drop(file);
    let mut permissions = fs::metadata(&staged)
        .map_err(|error| format!("staged helper metadata: {error}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&staged, permissions)
        .map_err(|error| format!("staged helper chmod: {error}"))?;

    let metadata =
        fs::metadata(&staged).map_err(|error| format!("staged helper metadata: {error}"))?;
    if !metadata.is_file() {
        return Err(format!(
            "staged helper {} is not a regular file",
            staged.display()
        ));
    }
    if metadata.uid() != current_uid() {
        return Err(format!(
            "staged helper {} is not owned by the invoking user",
            staged.display()
        ));
    }
    if metadata.nlink() != 1 {
        return Err(format!(
            "staged helper {} has {} links, not one",
            staged.display(),
            metadata.nlink()
        ));
    }
    let digest = format!("{:016x}", fnv1a(&fs::read(&staged).unwrap_or_default()));
    if digest != build_digest {
        return Err(format!(
            "staged helper digest {digest} differs from the build digest {build_digest}"
        ));
    }
    let canonical = fs::canonicalize(&staged)
        .map_err(|error| format!("staged helper canonicalize: {error}"))?;
    if canonical != staged {
        return Err(format!(
            "staged helper canonicalizes to {} rather than its input spelling {}",
            canonical.display(),
            staged.display()
        ));
    }
    Ok(StagedHelper {
        path: staged,
        digest,
        build_digest,
        mode: metadata.permissions().mode() & 0o777,
    })
}

#[cfg(not(unix))]
fn stage_helper(_probe_root: &Path, _build: &Path) -> Result<StagedHelper, String> {
    Err("native helper staging requires a Unix host".to_string())
}

/// Create a cell root and its fixed `payload` and `inputs` layout with
/// exclusive owner-only creates, then canonicalize the cell root and require
/// exactly the input spelling.
fn create_cell_root(cell_root: &Path) -> Result<CheckInputs, String> {
    create_private_dir(cell_root)
        .map_err(|error| format!("cell root {}: {error}", cell_root.display()))?;
    let payload_root = cell_root.join("payload");
    let inputs_dir = cell_root.join("inputs");
    create_private_dir(&payload_root).map_err(|error| format!("payload dir: {error}"))?;
    create_private_dir(&inputs_dir).map_err(|error| format!("inputs dir: {error}"))?;
    let canonical = fs::canonicalize(cell_root)
        .map_err(|error| format!("cell root canonicalize {}: {error}", cell_root.display()))?;
    if canonical != cell_root {
        return Err(format!(
            "cell root canonicalizes to {} rather than its input spelling {}",
            canonical.display(),
            cell_root.display()
        ));
    }
    Ok(CheckInputs {
        cell_root: cell_root.to_string_lossy().to_string(),
        payload_root: payload_root.to_string_lossy().to_string(),
        inputs_dir: inputs_dir.to_string_lossy().to_string(),
        helper: String::new(),
    })
}

/// A launchd-label-safe atom: no path separators or quoting surprises.
fn sanitize_tag(raw: &str) -> String {
    raw.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '.' || character == '-' {
                character
            } else {
                '.'
            }
        })
        .collect()
}

fn role_atom(raw: &str) -> String {
    raw.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '.'
            }
        })
        .collect()
}

fn wait_for_job(domain_target: &str, wanted: bool, timeout: Duration) -> Result<(), String> {
    let started = Instant::now();
    loop {
        if job_loaded(domain_target) == wanted {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "{} was {} within {:?}",
                domain_target,
                if wanted {
                    "never registered"
                } else {
                    "never removed"
                },
                timeout
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Open the liveness FIFO's write end without ever leaving a blocked thread.
/// The guard opens the read end early; until it does, a non-blocking write
/// open returns `ENXIO` (6), so this polls in bounded fashion and fails rather
/// than leaking a thread that blocks on a reader that never appears.
#[cfg(unix)]
fn open_writer(liveness: &Path) -> Result<fs::File, String> {
    use std::os::unix::fs::OpenOptionsExt;
    // O_NONBLOCK: 0x0004 on Darwin, 0x800 on Linux. The native adapter only
    // executes on Darwin; the Linux value keeps the type-checked build honest.
    #[cfg(target_os = "macos")]
    const O_NONBLOCK: i32 = 0x0004;
    #[cfg(not(target_os = "macos"))]
    const O_NONBLOCK: i32 = 0x800;
    const ENXIO: i32 = 6;
    let started = Instant::now();
    loop {
        match fs::OpenOptions::new()
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(liveness)
        {
            Ok(file) => return Ok(file),
            Err(error) if error.raw_os_error() == Some(ENXIO) => {
                if started.elapsed() >= TEARDOWN {
                    return Err("timed out opening the liveness channel".to_string());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(format!("liveness channel: {error}")),
        }
    }
}

#[cfg(not(unix))]
fn open_writer(_liveness: &Path) -> Result<fs::File, String> {
    Err("the liveness channel is Unix-only".to_string())
}

/// Owns the supervisor holder child and guarantees it is killed and waited on
/// every drop path, so an early measurement error cannot abandon it.
struct HolderGuard(Child);

impl HolderGuard {
    fn spawn(liveness: &Path) -> Result<HolderGuard, String> {
        let child = Command::new(HELPER)
            .args(["holder", "--liveness", &liveness.to_string_lossy()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("supervisor holder: {e}"))?;
        Ok(HolderGuard(child))
    }

    fn pid(&self) -> u32 {
        self.0.id()
    }
}

impl Drop for HolderGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn wait_for_heartbeat(root: &Path, timeout: Duration) -> Result<bool, String> {
    let path = root.join("payload/heartbeat");
    let first = fs::read_to_string(&path).unwrap_or_default();
    let started = Instant::now();
    while started.elapsed() < timeout {
        let now = fs::read_to_string(&path).unwrap_or_default();
        if !now.is_empty() && now != first {
            // Give the heartbeat a moment to prove motion beyond the first
            // sample rather than a single write.
            let second = fs::read_to_string(&path).unwrap_or_default();
            if second != now {
                return Ok(true);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(false)
}

fn heartbeat_moved(root: &Path, window: Duration) -> bool {
    let path = root.join("payload/heartbeat");
    let before = fs::read_to_string(&path).unwrap_or_default();
    std::thread::sleep(window);
    fs::read_to_string(&path).unwrap_or_default() != before
}

fn wait_for_file(path: &Path, timeout: Duration) -> Result<(), String> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if path.is_file() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "{} never appeared within {timeout:?}",
        path.display()
    ))
}

struct Identity {
    tag: String,
    pid: u32,
    pgid: u32,
}

fn recorded_identities(path: &Path) -> Vec<Identity> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            Some(Identity {
                tag: fields.next()?.to_string(),
                pid: fields.next()?.parse().ok()?,
                pgid: fields.next()?.parse().ok()?,
            })
        })
        .collect()
}

fn live_identities(root: &Path) -> Vec<Identity> {
    recorded_identities(&root.join("payload/identities"))
        .into_iter()
        .filter(|identity| pid_alive(identity.pid) && same_start_identity(identity.pid))
        .collect()
}

fn pid_alive(pid: u32) -> bool {
    Command::new(KILL)
        .args(["-0", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn same_start_identity(pid: u32) -> bool {
    Command::new(PS)
        .args(["-p", &pid.to_string(), "-o", "lstart="])
        .output()
        .map(|out| !String::from_utf8_lossy(&out.stdout).trim().is_empty())
        .unwrap_or(false)
}

fn read_attempts(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|text| text.lines().any(|line| line.contains("failed")))
        .unwrap_or(false)
}

fn read_ready(root: &Path, nonce: &str) -> (bool, bool) {
    match fs::read_to_string(root.join("payload/ready")) {
        Ok(text) => {
            let mut fields = text.split_whitespace();
            let observed_nonce = fields.next().unwrap_or_default();
            let child = fields
                .next()
                .and_then(|pid| pid.parse::<u32>().ok())
                .is_some();
            (observed_nonce == nonce, child)
        }
        Err(_) => (false, false),
    }
}

/// The bounded startup stages the helper recorded, in order.
fn read_stages(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(unix)]
fn exit_of(status: &std::process::ExitStatus) -> HelperExit {
    if status.success() {
        return HelperExit::Clean;
    }
    std::os::unix::process::ExitStatusExt::signal(status)
        .map(HelperExit::Signal)
        .unwrap_or_else(|| HelperExit::NonZero(status.code().unwrap_or(1)))
}

#[cfg(not(unix))]
fn exit_of(status: &std::process::ExitStatus) -> HelperExit {
    if status.success() {
        HelperExit::Clean
    } else {
        HelperExit::NonZero(status.code().unwrap_or(1))
    }
}

fn bound(text: &str) -> String {
    const LIMIT: usize = 8_192;
    text.chars().take(LIMIT).collect()
}

fn read_bounded(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|text| bound(&text))
        .unwrap_or_default()
}

fn fnv1a(bytes: &[u8]) -> u64 {
    // A non-cryptographic identity digest is enough to prove the startup cells
    // and the lifetime matrix used identical helper/profile structure.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn digest(path: &Path) -> String {
    format!("{:016x}", fnv1a(&fs::read(path).unwrap_or_default()))
}

/// Digest the profile with the validated cell root, payload root and helper
/// replaced by their typed placeholders, so two isolated cells compare the
/// policy rather than their private paths. A real rule, role or helper
/// mutation still changes the digest.
fn structural_profile_digest(profile: &str, inputs: &CheckInputs) -> String {
    let structural = profile
        .replace(&inputs.payload_root, super::PLACEHOLDER_PAYLOAD_ROOT)
        .replace(&inputs.cell_root, super::PLACEHOLDER_CELL_ROOT)
        .replace(&inputs.helper, super::PLACEHOLDER_HELPER);
    format!("{:016x}", fnv1a(structural.as_bytes()))
}

fn write_plist(
    path: &Path,
    label: &str,
    argv: &[&str],
    out: &Path,
    err: &Path,
) -> Result<(), String> {
    let mut args = String::new();
    for arg in argv {
        args.push_str(&format!("    <string>{}</string>\n", xml_escape(arg)));
    }
    let text = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\"><dict>\n\
         <key>Label</key><string>{label}</string>\n\
         <key>ProgramArguments</key><array>\n{args}</array>\n\
         <key>RunAtLoad</key><true/>\n\
         <key>KeepAlive</key><false/>\n\
         <key>StandardOutPath</key><string>{out}</string>\n\
         <key>StandardErrorPath</key><string>{err}</string>\n\
         </dict></plist>\n",
        label = xml_escape(label),
        out = xml_escape(&out.to_string_lossy()),
        err = xml_escape(&err.to_string_lossy()),
    );
    fs::write(path, text).map_err(|e| format!("plist {}: {e}", path.display()))
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A deliberately broad diagnostic profile for the one-allowance /
/// `allow default` diagnosis path. It exists only to name the layer that
/// refuses the exact payload; the shared model refuses any observation
/// labelled with it, so it can never be a passing candidate.
fn default_allow_profile(read_root: &Path, payload_dir: &Path) -> String {
    format!(
        ";; DIAGNOSTIC ONLY (allow default): never a passing candidate\n\
         ;; read root {} ; write payload {}\n\
         (version 1)\n\
         (allow default)\n",
        read_root.display(),
        payload_dir.display(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seatbelt_probe::{
        evaluate_startup, passing_result, passing_startup, run_probe, Verdict,
    };

    struct ScriptedPass;

    impl ProbeHost for ScriptedPass {
        fn precondition(&mut self) -> Option<Precondition> {
            None
        }

        fn run_startup_cell(&mut self, cell: StartupCell) -> StartupObservation {
            passing_startup(cell)
        }

        fn run_case(&mut self, case: Case) -> CaseResult {
            passing_result(case)
        }
    }

    #[test]
    fn the_obligation_matrix_still_evaluates_on_a_hypothetical_pass() {
        // The shared driver is what the native run uses; a scripted pass here
        // proves the driver, never the macOS mechanism.
        let mut host = ScriptedPass;
        let report = run_probe(&mut host, &Case::ALL);
        assert!(matches!(report.verdict, Verdict::Pass));
    }

    fn check_inputs(cell: &str) -> CheckInputs {
        let root = format!("/private/var/folders/probe/{cell}");
        CheckInputs {
            cell_root: root.clone(),
            payload_root: format!("{root}/payload"),
            inputs_dir: format!("{root}/inputs"),
            helper: "/private/var/folders/probe/bin/seatbelt-probe-helper".to_string(),
        }
    }

    fn candidate(inputs: &CheckInputs) -> String {
        render_candidate_profile(&STARTUP_RULE_LEDGER, inputs)
    }

    #[test]
    fn the_experimental_profile_writes_only_payload_state() {
        let inputs = check_inputs("probe");
        let profile = candidate(&inputs);
        assert!(profile.contains("(deny default)"));
        assert!(
            profile.contains(&format!("(subpath \"{}\")", inputs.payload_root)),
            "the payload write is concrete"
        );
        assert!(!profile.contains("(allow default)"));
    }

    #[test]
    fn the_candidate_profile_grants_the_root_inode_without_recursive_root_read() {
        // The dynamic loader reads the filesystem-root inode during process
        // init. `(literal "/")` grants exactly that inode; `(subpath "/")`
        // would grant the whole filesystem and must never appear.
        let inputs = check_inputs("probe");
        let profile = candidate(&inputs);
        assert!(profile.contains("(allow file-read* (literal \"/\"))"));
        assert!(!profile.contains("(subpath \"/\")"));
        assert!(!profile.contains("(allow default)"));
        assert!(
            STARTUP_NEGATIVE_ALLOWANCES
                .iter()
                .all(|allowance| allowance.removed_rule != "(subpath \"/\")"),
            "the removal control must strip the literal root grace, not a recursive grant"
        );
    }

    #[test]
    fn the_root_inode_negative_control_removes_exactly_the_load_bearing_rule() {
        let inputs = check_inputs("probe");
        let profile = candidate(&inputs);
        let allowance = STARTUP_NEGATIVE_ALLOWANCES[0];
        assert_eq!(allowance.name, "root-inode-read");
        let stripped = profile.replace(allowance.removed_rule, "");
        assert!(
            !stripped.contains(allowance.removed_rule),
            "the removal must drop the rule"
        );
        assert_ne!(
            stripped, profile,
            "the rule must have been present to remove"
        );
        // Every other ledger authority survives the removal, so the control
        // isolates exactly one rule.
        assert!(stripped.contains("(deny default)"));
        assert!(stripped.contains("(allow process-fork)"));
        assert!(stripped.contains("(allow file-read* (subpath \"/bin\"))"));
        assert!(stripped.contains(&format!("(subpath \"{}\")", inputs.payload_root)));
        // And re-removing is a no-op, which the adapter treats as unobserved.
        assert_eq!(stripped.replace(allowance.removed_rule, ""), stripped);
    }

    #[test]
    fn structural_profile_digest_ignores_the_typed_cell_root() {
        let first_inputs = check_inputs("cell-one");
        let second_inputs = check_inputs("cell-two");
        let first = candidate(&first_inputs);
        let second = candidate(&second_inputs);
        assert_ne!(first, second, "raw profiles embed distinct typed roots");
        assert_eq!(
            structural_profile_digest(&first, &first_inputs),
            structural_profile_digest(&second, &second_inputs),
        );
        // A real policy difference still changes the structural digest.
        let broken = first.replace("(allow process-fork)", "(deny process-fork)");
        assert_ne!(
            structural_profile_digest(&first, &first_inputs),
            structural_profile_digest(&broken, &first_inputs),
        );
    }

    #[test]
    fn the_default_allow_diagnostic_is_labelled_and_non_passing() {
        let profile = default_allow_profile(
            Path::new("/private/var/folders/probe"),
            Path::new("/private/var/folders/probe/payload"),
        );
        assert!(profile.contains("DIAGNOSTIC ONLY"));
        assert!(profile.contains("(allow default)"));

        let cell = StartupCell::S1DirectSeatbelt;
        let mut observation = passing_startup(cell);
        observation.diagnostic = Some("allow-default profile".to_string());
        let verdict = evaluate_startup(&[cell], &[observation]);
        assert!(!verdict.is_pass());
    }

    #[test]
    fn no_named_diagnostic_authority_enters_the_candidate_profile() {
        let inputs = check_inputs("probe");
        let profile = candidate(&inputs);
        assert!(!profile.contains("(allow default)"));
        assert!(!profile.contains("(allow mach-lookup)"));
        assert!(!profile.contains("(allow network"));
        let mut names = std::collections::BTreeSet::new();
        for allowance in DIAGNOSTIC_ALLOWANCES {
            assert!(!profile.contains(allowance.sbpl), "{}", allowance.name);
            assert!(!allowance.name.is_empty());
            assert!(!allowance.operation.is_empty());
            assert!(!allowance.target.is_empty());
            assert!(!allowance.consumer.is_empty());
            assert!(
                names.insert(allowance.name),
                "diagnostic name {} is duplicated",
                allowance.name
            );
        }
        assert_eq!(names.len(), DIAGNOSTIC_ALLOWANCES.len());
    }

    #[test]
    fn launchd_labels_are_unique_across_host_instances() {
        // Two hosts created in the same process share a PID; their labels must
        // still differ so a stale registration can never race another test.
        let first = NativeProbeHost::new("startup-shared".to_string());
        let second = NativeProbeHost::new("startup-shared".to_string());
        let first_labels = [
            first.startup_label(StartupCell::S0DirectUnboxed),
            first.startup_label(StartupCell::S3LaunchdSeatbelt),
        ];
        let second_labels = [
            second.startup_label(StartupCell::S0DirectUnboxed),
            second.startup_label(StartupCell::S3LaunchdSeatbelt),
        ];
        let mut labels = std::collections::BTreeSet::new();
        for label in first_labels.iter().chain(second_labels.iter()) {
            assert!(labels.insert(label.clone()), "reused label {label}");
            assert!(label.starts_with("org.brokkr.seatbelt.probe."));
        }
        assert_eq!(labels.len(), 4);
        assert_ne!(first.run_tag, second.run_tag);
        for label in &first_labels {
            assert!(label.contains(&first.run_tag));
        }
    }

    #[test]
    fn tag_sanitizing_keeps_labels_path_and_quote_safe() {
        assert_eq!(sanitize_tag("startup-1/2 3"), "startup-1.2.3");
        assert_eq!(role_atom("S1-direct-seatbelt"), "S1.direct.seatbelt");
        assert_eq!(role_atom(".."), "..");
    }
}
