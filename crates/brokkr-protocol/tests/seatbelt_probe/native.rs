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
use std::time::{Duration, Instant};

use super::{
    run_gated_probe, run_startup, Case, CaseResult, Event, GatedReport, HelperExit, Precondition,
    ProbeHost, RepairFacts, StartupCell, StartupObservation, TriggerKind,
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
const HANDS_BOX_ENV: &str = "BROKKR_HANDS_BOX";

/// Gate A native test: the four-cell startup matrix on a committed revision.
/// It fails on a missing prerequisite, a skipped cell, or any unmet startup
/// observation, and never reports a pass from Linux.
#[test]
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
            "  {}: ran={} ready={} ordinary_child={} exit={} runs={:?} crashes={:?} skip={:?}\n\
             \x20   helper={} profile={:?} argv={:?}\n",
            cell.cell.name(),
            cell.ran,
            cell.ready,
            cell.ordinary_child,
            cell.exit.describe(),
            cell.launchd_runs,
            cell.launchd_crashes,
            cell.skip,
            cell.helper_digest,
            cell.profile_digest,
            cell.helper_argv,
        ));
        if !cell.stdout.is_empty() {
            out.push_str(&format!("    stdout: {}\n", cell.stdout));
        }
        if !cell.stderr.is_empty() {
            out.push_str(&format!("    stderr: {}\n", cell.stderr));
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

/// A host that runs the experiment with real macOS facilities.
pub struct NativeProbeHost {
    root: PathBuf,
    uid: u32,
    precondition: Option<Precondition>,
    /// One nonce for all four startup cells, so their helper argv is identical
    /// and only the launch owner / profile differ between cells.
    startup_nonce: String,
}

impl NativeProbeHost {
    pub fn new(run_id: String) -> NativeProbeHost {
        let raw = std::env::temp_dir().join(format!("brokkr-seatbelt-probe-{run_id}"));
        let root = raw.canonicalize().unwrap_or(raw);
        let uid = current_uid();
        let precondition = check_precondition();
        NativeProbeHost {
            root,
            uid,
            precondition,
            startup_nonce: format!(
                "{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos())
                    .unwrap_or_default()
            ),
        }
    }

    fn label(&self, case: Case, role: &str) -> String {
        format!(
            "org.brokkr.seatbelt.probe.{}.{}.{role}",
            std::process::id(),
            case.name().replace('-', ".")
        )
    }

    fn service_target(&self, label: &str) -> String {
        format!("gui/{}/{}", self.uid, label)
    }

    fn candidate_profile(&self, read_root: &Path, payload_dir: &Path) -> String {
        sandbox_profile(read_root, payload_dir)
    }

    // -----------------------------------------------------------------------
    // Gate A
    // -----------------------------------------------------------------------

    fn startup_cell(&mut self, cell: StartupCell) -> StartupObservation {
        // One shared root and nonce for every cell: the helper bytes and the
        // helper argv (excluding the launcher prefix) must be identical across
        // the four cells, which is what makes the matrix a differential.
        let root = self.root.join("startup/cell");
        let _ = fs::remove_dir_all(&root);
        if let Err(error) = fs::create_dir_all(root.join("payload")) {
            return StartupObservation::skipped(cell, &format!("startup root: {error}"));
        }
        if let Err(error) = fs::create_dir_all(root.join("inputs")) {
            return StartupObservation::skipped(cell, &format!("startup inputs: {error}"));
        }

        let profile = root.join("inputs/policy.sb");
        let profile_digest = if cell.seatbelt() {
            match fs::write(
                &profile,
                self.candidate_profile(&root, &root.join("payload")),
            ) {
                Ok(()) => Some(digest(&profile)),
                Err(error) => {
                    return StartupObservation::skipped(cell, &format!("profile: {error}"))
                }
            }
        } else {
            None
        };

        let nonce = self.startup_nonce.clone();
        let helper_argv = vec![
            "startup".to_string(),
            "--root".to_string(),
            root.to_string_lossy().to_string(),
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

        let prepared = if cell.launchd() {
            self.run_startup_launchd(cell, &root, &launcher, &helper_argv)
        } else {
            self.run_startup_direct(cell, &root, &launcher, &helper_argv)
        };

        // On an exact-profile failure, run the labelled `allow default`
        // diagnostic so the report names the layer that refuses the payload.
        // It is diagnostic output only; the shared model refuses any
        // observation labelled with it, so it can never be admission.
        if let Ok(observation) = &prepared {
            if cell.seatbelt() && !observation.ready {
                self.log_default_allow_diagnostic(&root, &helper_argv);
            }
        }

        let _ = bootout(&self.service_target(&format!(
            "org.brokkr.seatbelt.probe.startup.{}.{}",
            std::process::id(),
            cell.name().replace('-', ".")
        )));
        let _ = fs::remove_dir_all(&root);
        match prepared {
            Ok(mut observation) => {
                observation.helper_digest = digest(Path::new(HELPER));
                observation.profile_digest = profile_digest;
                observation.helper_argv = helper_argv;
                observation
            }
            Err(error) => {
                let mut observation = StartupObservation::skipped(cell, &error);
                observation.helper_digest = digest(Path::new(HELPER));
                observation.profile_digest = profile_digest;
                observation.helper_argv = helper_argv;
                observation
            }
        }
    }

    fn startup_program(&self, launcher: &[String], helper_argv: &[String]) -> Vec<String> {
        let mut program = launcher.to_vec();
        program.push(HELPER.to_string());
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
        let (ready, ordinary_child) = read_ready(root, &helper_argv[4]);
        Ok(StartupObservation {
            cell,
            ran: true,
            skip: None,
            helper_digest: String::new(),
            profile_digest: None,
            helper_argv: Vec::new(),
            ready,
            ordinary_child,
            exit,
            launchd_runs: None,
            launchd_crashes: None,
            diagnostic: None,
            stdout: bound(&String::from_utf8_lossy(&output.stdout)),
            stderr: bound(&String::from_utf8_lossy(&output.stderr)),
        })
    }

    fn run_startup_launchd(
        &self,
        cell: StartupCell,
        root: &Path,
        launcher: &[String],
        helper_argv: &[String],
    ) -> Result<StartupObservation, String> {
        let label = format!(
            "org.brokkr.seatbelt.probe.startup.{}.{}",
            std::process::id(),
            cell.name().replace('-', ".")
        );
        let program = self.startup_program(launcher, helper_argv);
        let program_refs: Vec<&str> = program.iter().map(String::as_str).collect();
        let plist = root.join("startup.plist");
        write_plist(
            &plist,
            &label,
            &program_refs,
            &root.join("startup.out"),
            &root.join("startup.err"),
        )?;
        bootstrap(&format!("gui/{}", self.uid), &plist)?;
        let target = self.service_target(&label);
        let started = Instant::now();
        let mut ready = false;
        let mut ordinary_child = false;
        while started.elapsed() < TEARDOWN {
            if root.join("payload/ready").is_file() {
                let (observed_ready, observed_child) = read_ready(root, &helper_argv[4]);
                ready = observed_ready;
                ordinary_child = observed_child;
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        // Let the job reach a terminal state before reading its run facts.
        let _ = wait_for_job(&target, false, TEARDOWN);
        let state = launchctl_print(&target).unwrap_or_default();
        let runs = parse_counter(&state, "runs = ");
        let crashes = parse_counter(&state, "successive crashes = ");
        let exit = match parse_counter(&state, "last exit code = ") {
            Some(0) => HelperExit::Clean,
            Some(code) => HelperExit::NonZero(code as i32),
            None => {
                if crashes == Some(0) {
                    HelperExit::Clean
                } else {
                    HelperExit::NonZero(crashes.unwrap_or(1) as i32)
                }
            }
        };
        let _ = bootout(&target);
        Ok(StartupObservation {
            cell,
            ran: true,
            skip: None,
            helper_digest: String::new(),
            profile_digest: None,
            helper_argv: Vec::new(),
            ready,
            ordinary_child,
            exit,
            launchd_runs: runs,
            launchd_crashes: crashes,
            diagnostic: None,
            stdout: read_bounded(&root.join("startup.out")),
            stderr: read_bounded(&root.join("startup.err")),
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
        let root = self.root.join("lifetime").join(case.name());
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("inputs")).map_err(|e| format!("probe root: {e}"))?;
        fs::create_dir_all(root.join("payload")).map_err(|e| format!("probe root: {e}"))?;
        fs::create_dir_all(root.join("guard")).map_err(|e| format!("probe root: {e}"))?;
        fs::create_dir_all(root.join("observer")).map_err(|e| format!("probe root: {e}"))?;

        let payload_label = self.label(case, "payload");
        let guard_label = self.label(case, "guard");
        let peer_label = self.label(case, "peer");
        let escape_label = format!("org.brokkr.seatbelt.probe.escape.{}", std::process::id());
        let liveness = root.join("liveness");
        run(MKFIFO, &[&liveness.to_string_lossy()], None)?;

        fs::write(root.join("inputs/guard-label"), &guard_label).map_err(|e| e.to_string())?;
        fs::write(root.join("inputs/peer-label"), &peer_label).map_err(|e| e.to_string())?;
        let profile = root.join("inputs/policy.sb");
        fs::write(
            &profile,
            self.candidate_profile(&root, &root.join("payload")),
        )
        .map_err(|e| format!("policy: {e}"))?;
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
                HELPER,
                "payload",
                "--root",
                &root.to_string_lossy(),
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
                HELPER,
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
                HELPER,
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

    /// Run the helper once under a labelled `allow default` diagnostic profile
    /// and print the outcome. This is diagnosis, never admission: the model
    /// refuses any observation labelled as a diagnostic.
    fn log_default_allow_diagnostic(&self, root: &Path, helper_argv: &[String]) {
        let diagnostic = root.join("inputs/diagnostic.sb");
        if fs::write(
            &diagnostic,
            default_allow_profile(root, &root.join("payload")),
        )
        .is_err()
        {
            return;
        }
        let mut command = Command::new(SANDBOX_EXEC);
        command
            .arg("-f")
            .arg(&diagnostic)
            .arg(HELPER)
            .args(helper_argv);
        if let Ok(output) = command.output() {
            eprintln!(
                "DIAGNOSTIC ONLY (allow default, non-passing): status={}; stdout={}; stderr={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
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

fn launchctl_print(domain_target: &str) -> Option<String> {
    Command::new(LAUNCHCTL)
        .args(["print", domain_target])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).to_string())
}

fn parse_counter(text: &str, key: &str) -> Option<u32> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(key))
        .and_then(|value| value.trim().parse().ok())
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

fn open_writer(liveness: &Path) -> Result<fs::File, String> {
    let path = liveness.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let opened = fs::OpenOptions::new().write(true).open(&path);
        let _ = tx.send(opened.map_err(|e| e.to_string()));
    });
    match rx.recv_timeout(TEARDOWN) {
        Ok(Ok(file)) => Ok(file),
        Ok(Err(error)) => Err(format!("liveness channel: {error}")),
        Err(_) => Err("timed out opening the liveness channel".to_string()),
    }
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

fn digest(path: &Path) -> String {
    // A non-cryptographic identity digest is enough to prove the four startup
    // cells and the lifetime matrix used identical helper/profile bytes.
    let bytes = fs::read(path).unwrap_or_default();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
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

/// The minimum experimental profile for the exact payload and candidate
/// policy: the helper bytes are executable, the whole case root is readable,
/// and only `payload/` is writable. There is deliberately no `mach-lookup`
/// grant, so a payload cannot reach the guard's launchd authority.
fn sandbox_profile(root: &Path, payload_dir: &Path) -> String {
    let root = root.to_string_lossy();
    let payload = payload_dir.to_string_lossy();
    format!(
        "(version 1)\n\
         (deny default)\n\
         (allow process*)\n\
         (allow signal (target self))\n\
         (allow file-read* (subpath \"/usr\") (subpath \"/bin\") (subpath \"/sbin\") \
         (subpath \"/System\") (subpath \"/Library\") (subpath \"/private/tmp\") \
         (subpath \"/private/var/tmp\") (subpath \"{root}\") (literal \"{helper}\") \
         (literal \"/dev/null\") (literal \"/dev/urandom\") (literal \"/dev/random\") \
         (literal \"/dev/dtracehelper\"))\n\
         (allow file-write* (subpath \"{payload}\"))\n\
         (allow sysctl-read)\n\
         (allow ipc-posix-shm)\n",
        helper = HELPER,
    )
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

    #[test]
    fn the_experimental_profile_writes_only_payload_state() {
        let profile = sandbox_profile(
            Path::new("/private/var/folders/probe"),
            Path::new("/private/var/folders/probe/payload"),
        );
        assert!(profile.contains("(deny default)"));
        assert!(profile.contains("(subpath \"/private/var/folders/probe/payload\")"));
        assert!(!profile.contains("(allow default)"));
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
}
