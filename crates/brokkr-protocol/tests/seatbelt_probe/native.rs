//! macOS adapter for the bounded R3 probe. Compiled only on macOS.
//!
//! This is an experiment, not production enforcement. It attempts the
//! design's named candidate — one transient per-invocation launchd payload
//! job whose job/process coalition is the candidate containment domain, plus
//! a separately launchd-owned guard job that boots the payload out and
//! establishes quiescence before cleanup. If public launchd cannot contain a
//! `setsid`/double-fork descendant, the probe fails and SEATBELT-R3 stays
//! open; that is the expected outcome of a failed hypothesis, not a bug to
//! hide.
//!
//! Every operation here is a real `/usr/bin/sandbox-exec`, real `launchctl`,
//! real process identities and real time. The shared model in the parent
//! module decides pass/fail; a missing prerequisite is a named failure.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use super::{run_probe, Case, CaseResult, Event, Precondition, ProbeHost};

const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
const LAUNCHCTL: &str = "/bin/launchctl";
const PYTHON: &str = "/usr/bin/python3";
const MKFIFO: &str = "/usr/bin/mkfifo";
const KILL: &str = "/bin/kill";
const UID: &str = "/usr/bin/id";
const TEARDOWN: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_secs(1);
const HANDS_BOX_ENV: &str = "BROKKR_HANDS_BOX";

/// The required native test: the whole obligation matrix on one committed
/// revision. It fails on a missing prerequisite, a skipped case or any
/// unmet guarantee, and never reports a pass from Linux.
#[test]
fn native_lifetime_feasibility_probe() {
    if !cfg!(target_os = "macos") {
        // The test exists on every host so the adapter below is compiled and
        // type-checked here; the required run is macOS-only and the macOS CI
        // step selects this exact name and fails if it did not run.
        return;
    }
    let mut host = NativeProbeHost::new(format!("probe-{}", std::process::id()));
    let report = run_probe(&mut host, &Case::ALL);
    assert!(
        report.verdict.is_pass(),
        "{}\n\nnative observations:\n{:#?}",
        report.verdict.render(),
        report.results
    );
}

/// A host that runs the experiment with real macOS facilities.
pub struct NativeProbeHost {
    root: std::path::PathBuf,
    uid: u32,
    precondition: Option<Precondition>,
}

impl NativeProbeHost {
    pub fn new(run_id: String) -> NativeProbeHost {
        let root = std::env::temp_dir().join(format!("brokkr-seatbelt-probe-{run_id}"));
        let uid = current_uid();
        let precondition = check_precondition();
        NativeProbeHost {
            root,
            uid,
            precondition,
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

    fn run_case_inner(&mut self, case: Case) -> Result<CaseResult, String> {
        let root = self.root.join(case.name());
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("payload")).map_err(|e| format!("probe root: {e}"))?;
        fs::create_dir_all(root.join("guard")).map_err(|e| format!("probe root: {e}"))?;
        fs::write(root.join("heartbeat"), "").map_err(|e| format!("heartbeat: {e}"))?;
        fs::write(root.join("identities"), "").map_err(|e| format!("identities: {e}"))?;
        let liveness = root.join("liveness");
        run(MKFIFO, &[&liveness.to_string_lossy()], None)?;

        let payload_label = self.label(case, "payload");
        let guard_label = self.label(case, "guard");
        let peer_label = self.label(case, "peer");
        let escape_label = self.label(case, "escape");
        fs::write(root.join("guard-label"), &guard_label).map_err(|e| e.to_string())?;
        fs::write(root.join("peer-label"), &peer_label).map_err(|e| e.to_string())?;

        let profile = root.join("policy.sb");
        fs::write(&profile, sandbox_profile(&root)).map_err(|e| format!("policy: {e}"))?;
        let payload = root.join("payload/adversary.py");
        fs::write(&payload, adversary()).map_err(|e| format!("adversary: {e}"))?;
        let guard = root.join("guard/guard.sh");
        fs::write(&guard, guard_script()).map_err(|e| format!("guard: {e}"))?;
        fs::write(
            root.join("escape.plist"),
            escape_plist(&escape_label, &root),
        )
        .map_err(|e| format!("escape plist: {e}"))?;

        let payload_plist = root.join("payload.plist");
        write_plist(
            &payload_plist,
            &payload_label,
            &[
                SANDBOX_EXEC,
                "-p",
                &profile.to_string_lossy(),
                PYTHON,
                &payload.to_string_lossy(),
                &root.to_string_lossy(),
                case.name(),
            ],
            &root.join("payload.out"),
            &root.join("payload.err"),
        )?;
        let guard_plist = root.join("guard.plist");
        write_plist(
            &guard_plist,
            &guard_label,
            &[
                "/bin/sh",
                &guard.to_string_lossy(),
                &root.to_string_lossy(),
                &payload_label,
                &guard_label,
                &self.uid.to_string(),
            ],
            &root.join("guard.out"),
            &root.join("guard.err"),
        )?;

        let mut events = vec![Event::Prepared];
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
            &mut events,
        );
        // Descriptor-rooted cleanup of the disposable harness root. The
        // recorded lifecycle already removed the payload's private state.
        let _ = fs::remove_dir_all(&root);
        let mut result = outcome?;
        result.events = events;
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
        events: &mut Vec<Event>,
    ) -> Result<CaseResult, String> {
        let group_kill = case == Case::GroupKillNegativeControl;
        if !group_kill {
            bootstrap(&format!("gui/{}", self.uid), guard_plist)?;
            wait_for_job(&self.service_target(guard_label), true, TEARDOWN)?;
            events.push(Event::GuardRegistered);
        }

        // The liveness channel: the payload must not inherit the write end.
        // Supervisor death is modeled by a separate writer holder that the
        // test SIGKILLs; every other case closes the channel explicitly.
        let writer = match case {
            Case::SupervisorDeath => None,
            _ => Some(open_writer(liveness)?),
        };
        let holder = match case {
            Case::SupervisorDeath => Some(spawn_holder(liveness)?),
            _ => None,
        };

        bootstrap(&format!("gui/{}", self.uid), payload_plist)?;
        events.push(Event::PayloadStarted);
        let positive = wait_for_heartbeat(root, TEARDOWN)?;

        if matches!(case, Case::PeerBootout | Case::GuardInterference) {
            let peer = root.join("peer.plist");
            write_plist(
                &peer,
                peer_label,
                &["/bin/sleep", "300"],
                &root.join("peer.out"),
                &root.join("peer.err"),
            )?;
            bootstrap(&format!("gui/{}", self.uid), &peer)?;
        }

        match case {
            Case::Timeout => std::thread::sleep(Duration::from_millis(300)),
            Case::SupervisorDeath => {
                if let Some(holder) = &holder {
                    run(KILL, &["-9", &holder.id().to_string()], None)?;
                }
            }
            _ => {}
        }
        drop(writer);
        events.push(Event::Terminating);

        let quiesced = if group_kill {
            true
        } else {
            wait_for_quiescence(root, payload_label, self.uid)
        };
        if !group_kill {
            wait_for_job(&self.service_target(payload_label), false, TEARDOWN)?;
            wait_for_job(&self.service_target(guard_label), false, TEARDOWN)?;
        }
        if quiesced {
            events.push(Event::PayloadQuiescent);
        }
        let heartbeat_moved = heartbeat_moved(root);
        let labels_gone = !job_loaded(&self.service_target(payload_label))
            && (group_kill || !job_loaded(&self.service_target(guard_label)));
        let guard_survived = job_loaded(&self.service_target(guard_label));
        let denied = match case {
            Case::GuardInterference => guard_survived,
            Case::PeerBootout => job_loaded(&self.service_target(peer_label)),
            Case::EscapeJob => !job_loaded(&self.service_target(escape_label)),
            _ => true,
        };
        let observed_survivors = live_identities(root);

        if quiesced {
            events.push(Event::PrivateStateRemoved);
        }
        if !group_kill {
            bootout(&self.service_target(guard_label));
            events.push(Event::GuardUnregistered);
        }
        for label in [peer_label, escape_label] {
            bootout(&self.service_target(label));
        }
        for pid in &observed_survivors {
            let _ = Command::new(KILL).args(["-9", &pid.to_string()]).output();
        }

        Ok(CaseResult {
            case,
            ran: true,
            skip: None,
            positive_control: positive,
            heartbeat_before: positive,
            survivors_after: observed_survivors.len(),
            heartbeat_moved_during_quiet: heartbeat_moved,
            labels_gone,
            cleanup_after_quiescence: quiesced,
            guard_survived_payload: guard_survived,
            payload_attempt_denied: denied,
            events: Vec::new(),
        })
    }
}

impl ProbeHost for NativeProbeHost {
    fn precondition(&mut self) -> Option<Precondition> {
        self.precondition.clone()
    }

    fn run_case(&mut self, case: Case) -> CaseResult {
        match self.run_case_inner(case) {
            Ok(result) => result,
            Err(reason) => CaseResult::skipped(case, &reason),
        }
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
    for tool in [PYTHON, MKFIFO, KILL, UID] {
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

fn bootout(domain_target: &str) {
    let _ = Command::new(LAUNCHCTL)
        .args(["bootout", domain_target])
        .output();
}

fn job_loaded(domain_target: &str) -> bool {
    Command::new(LAUNCHCTL)
        .args(["print", domain_target])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
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

fn spawn_holder(liveness: &Path) -> Result<std::process::Child, String> {
    let shell = format!("exec sleep 3600 > '{}'", liveness.to_string_lossy());
    Command::new("/bin/sh")
        .args(["-c", &shell])
        .spawn()
        .map_err(|e| format!("supervisor holder: {e}"))
}

fn wait_for_heartbeat(root: &Path, timeout: Duration) -> Result<bool, String> {
    let path = root.join("heartbeat");
    let first = read_trimmed(&path);
    let started = Instant::now();
    while started.elapsed() < timeout {
        let now = read_trimmed(&path);
        if !now.is_empty() && now != first {
            return Ok(true);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Ok(false)
}

fn wait_for_quiescence(root: &Path, payload_label: &str, uid: u32) -> bool {
    let started = Instant::now();
    while started.elapsed() < TEARDOWN {
        let quiesced = root.join("quiesced").is_file();
        let no_survivors = live_identities(root).is_empty();
        let payload_gone = !job_loaded(&format!("gui/{uid}/{payload_label}"));
        if quiesced && no_survivors && payload_gone {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn heartbeat_moved(root: &Path) -> bool {
    let path = root.join("heartbeat");
    let before = read_trimmed(&path);
    std::thread::sleep(QUIET);
    read_trimmed(&path) != before
}

fn live_identities(root: &Path) -> Vec<u32> {
    read_trimmed(&root.join("identities"))
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter_map(|pid| pid.parse::<u32>().ok())
        .filter(|pid| pid_alive(*pid))
        .collect()
}

fn pid_alive(pid: u32) -> bool {
    Command::new(KILL)
        .args(["-0", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn read_trimmed(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
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

fn escape_plist(label: &str, root: &Path) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <plist version=\"1.0\"><dict>\n\
         <key>Label</key><string>{}</string>\n\
         <key>ProgramArguments</key><array><string>/bin/sleep</string><string>300</string></array>\n\
         <key>RunAtLoad</key><true/>\n\
         <key>StandardOutPath</key><string>{}/escape.out</string>\n\
         <key>StandardErrorPath</key><string>{}/escape.err</string>\n\
         </dict></plist>\n",
        xml_escape(label),
        xml_escape(&root.to_string_lossy()),
        xml_escape(&root.to_string_lossy()),
    )
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The minimum experimental profile: process and the case root, and nothing
/// that would let the payload reach the guard's launchd authority.
fn sandbox_profile(root: &Path) -> String {
    let root = root.to_string_lossy();
    format!(
        "(version 1)\n\
         (deny default)\n\
         (allow process*)\n\
         (allow signal (target self))\n\
         (allow file-read* (subpath \"/usr\") (subpath \"/bin\") (subpath \"/sbin\") \
         (subpath \"/System\") (subpath \"/Library\") (subpath \"/private/tmp\") \
         (subpath \"{root}\") (literal \"/dev/null\") (literal \"/dev/urandom\"))\n\
         (allow file-write* (subpath \"{root}\"))\n\
         (allow sysctl-read)\n\
         (allow ipc-posix-shm)\n"
    )
}

/// The guard: wait for engine-liveness EOF, boot the payload out, wait for one
/// second of heartbeat quiet, then clean up and unregister.
fn guard_script() -> &'static str {
    r#"#!/bin/sh
set -u
ROOT="$1"; PAYLOAD_LABEL="$2"; GUARD_LABEL="$3"; UID="$4"
# Wait for the engine's liveness write end to close (explicit trigger or
# abrupt supervisor death).
cat "$ROOT/liveness" >/dev/null 2>&1 || true
# Boot the payload job out. Detached descendants may survive this; the
# quiescence wait below is what decides.
/bin/launchctl bootout "gui/$UID/$PAYLOAD_LABEL" >/dev/null 2>&1 || true
# Quiescence: the heartbeat unchanged for one full second.
last=""
stable=0
while [ "$stable" -lt 10 ]; do
  current="$(cat "$ROOT/heartbeat" 2>/dev/null || printf none)"
  if [ "$current" = "$last" ]; then stable=$((stable+1)); else stable=0; last="$current"; fi
  sleep 0.1
done
touch "$ROOT/quiesced"
# Private state may be removed only now.
rm -rf "$ROOT/payload" 2>/dev/null || true
touch "$ROOT/cleanup-done"
# The guard unregisters itself last.
/bin/launchctl bootout "gui/$UID/$GUARD_LABEL" >/dev/null 2>&1 || true
"#
}

/// The payload helper. It proves the ordinary-child control, then detaches
/// with `setsid`/double-fork for the adversarial cases, always recording a
/// PID identity and a moving heartbeat.
fn adversary() -> &'static str {
    r#"import os, signal, sys, time
root, case = sys.argv[1], sys.argv[2]

def ident(tag):
    with open(os.path.join(root, "identities"), "a") as f:
        f.write("%s %d\n" % (tag, os.getpid()))

def beat():
    while True:
        with open(os.path.join(root, "heartbeat"), "w") as f:
            f.write(str(time.time()))
        time.sleep(0.05)

def detach():
    pid = os.fork()
    if pid == 0:
        os.setsid()
        if os.fork() != 0:
            os._exit(0)
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(sig, signal.SIG_IGN)
        ident("leaf")
        beat()
    else:
        os._exit(0)

if case == "guard-interference":
    ident("payload")
    label = open(os.path.join(root, "guard-label")).read().strip()
    os.system("/bin/launchctl kill SIGTERM gui/%d/%s" % (os.getuid(), label))
    time.sleep(1)
    beat()
elif case == "peer-bootout":
    ident("payload")
    label = open(os.path.join(root, "peer-label")).read().strip()
    os.system("/bin/launchctl bootout gui/%d/%s" % (os.getuid(), label))
    time.sleep(1)
    beat()
elif case == "escape-job":
    ident("payload")
    os.system("/bin/launchctl bootstrap gui/%d %s" % (
        os.getuid(), os.path.join(root, "escape.plist")))
    time.sleep(1)
    beat()
elif case == "ordinary-child":
    ident("ordinary")
    beat()
else:
    detach()
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seatbelt_probe::{passing_result, run_probe, Verdict};

    struct ScriptedPass;

    impl ProbeHost for ScriptedPass {
        fn precondition(&mut self) -> Option<Precondition> {
            None
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
    fn the_guard_script_names_bootout_quiescence_and_cleanup_order() {
        let script = guard_script();
        let boot = script.find("bootout").unwrap();
        let quiet = script.find("stable").unwrap();
        let cleanup = script.find("rm -rf").unwrap();
        assert!(boot < quiet && quiet < cleanup, "{script}");
    }
}
