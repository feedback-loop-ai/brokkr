//! Committed test-support executable for the bounded R3 Seatbelt probe
//! (decision 0046 slice II, design D3).
//!
//! One binary serves every probe role so the payload/guard/supervisor lineage
//! has no general-purpose interpreter or repository-script dependency:
//!
//! * `startup` — Gate A: authenticate READY with a nonce, spawn and identify
//!   an ordinary child, then exit cleanly or nonzero as directed.
//! * `payload` — Gate B: record identities, perform one case-specific
//!   detach/attack, and heartbeat.
//! * `detach-child` / `detach-leaf` — the `setsid` and double-fork descendants.
//! * `guard` — the separately launchd-owned lease guard: liveness EOF, public
//!   `bootout`, quiescence, protected cleanup and self-unregistration.
//! * `holder` — the supervisor whose `SIGKILL` closes the liveness channel.
//! * `child` — the ordinary child used by Gate A.
//!
//! It is probe scaffolding only; it is not production Seatbelt code and it
//! makes no containment claim. On non-Unix hosts it compiles but is never
//! selected.

#![allow(clippy::too_many_arguments)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// The one source of the startup denial-control targets, shared with the
// host-independent check by `#[path]` so the helper's attack functions and the
// check read identical constants (design D3).
#[path = "controls.rs"]
mod controls;

/// The exact helper spelling the observer passed as `--helper`, which every
/// exec of the helper must name. It is required before the first stage: a
/// helper started without it fails with a named message and records nothing.
fn helper_exe(args: &[String]) -> Result<PathBuf, String> {
    required(args, "--helper").map(PathBuf::from)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code = match dispatch(&args) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("probe-helper: {message}");
            2
        }
    };
    std::process::exit(code);
}

fn dispatch(args: &[String]) -> Result<i32, String> {
    match args.get(1).map(String::as_str) {
        Some("startup") => run_startup(args),
        Some("payload") => run_payload(args),
        Some("detach-child") => run_detach_child(args),
        Some("detach-leaf") => run_detach_leaf(args),
        Some("guard") => run_guard(args),
        Some("holder") => run_holder(args),
        Some("child") => run_ordinary_child(),
        Some("denial") => run_denial(args),
        _ => Err("unknown mode".to_string()),
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == name {
            return iter.next().cloned();
        }
    }
    None
}

fn required(args: &[String], name: &str) -> Result<String, String> {
    flag(args, name).ok_or_else(|| format!("missing {name}"))
}

fn root_of(args: &[String]) -> Result<PathBuf, String> {
    required(args, "--root").map(PathBuf::from)
}

fn write_at(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|error| format!("{}: {error}", path.display()))
}

fn append_at(path: &Path, content: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn sleep_ms(milliseconds: u64) {
    thread::sleep(Duration::from_millis(milliseconds));
}

// ---------------------------------------------------------------------------
// Gate A: the startup helper
// ---------------------------------------------------------------------------

/// `startup --root DIR --helper PATH --nonce NONCE --exit clean|nonzero`
fn run_startup(args: &[String]) -> Result<i32, String> {
    // The observer's staged helper spelling, read before the first stage. A
    // helper started without it fails here having recorded no stage.
    let helper = helper_exe(args)?;
    let root = root_of(args)?;
    let nonce = required(args, "--nonce")?;
    let directed = flag(args, "--exit").unwrap_or_else(|| "clean".to_string());
    let payload = root.join("payload");
    fs::create_dir_all(&payload).map_err(|error| format!("payload dir: {error}"))?;
    // Bounded, ordered startup stages. The adapter requires the exact sequence
    // before a cell can pass, so an abort localizes to the stage it reached
    // instead of being reported as an opaque signal. READY carries the nonce.
    let stages = payload.join("stages");
    append_at(&stages, "entry\n")?;
    append_at(&stages, "payload-dir\n")?;
    append_at(&stages, "executable\n")?;

    // The ordinary child's streams are explicit pre-opened handles on
    // `/dev/null`, never `Stdio::null()`, so a stream-setup refusal names the
    // stream it failed on before any spawn is attempted.
    let child_stdin = OpenOptions::new()
        .read(true)
        .open("/dev/null")
        .map_err(|error| format!("child streams: stdin /dev/null: {error}"))?;
    let child_stdout = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|error| format!("child streams: stdout /dev/null: {error}"))?;
    let child_stderr = OpenOptions::new()
        .write(true)
        .open("/dev/null")
        .map_err(|error| format!("child streams: stderr /dev/null: {error}"))?;

    let mut child = Command::new(&helper)
        .arg("child")
        .stdin(Stdio::from(child_stdin))
        .stdout(Stdio::from(child_stdout))
        .stderr(Stdio::from(child_stderr))
        .spawn()
        .map_err(|error| format!("child spawn/exec: {error}"))?;
    append_at(&stages, "child-spawn\n")?;
    let child_pid = child.id();
    if child
        .try_wait()
        .map_err(|error| format!("child observation: {error}"))?
        .is_some_and(|status| !status.success())
    {
        return Err("child observation: the ordinary child exited nonzero".to_string());
    }
    append_at(&stages, "child-observed\n")?;
    append_at(
        &payload.join("identities"),
        &format!("ordinary {child_pid} {child_pid}\n"),
    )?;
    write_at(&payload.join("ready"), &format!("{nonce} {child_pid}\n"))?;
    append_at(&stages, "ready\n")?;

    let _ = child.wait();
    if directed == "nonzero" {
        append_at(&stages, "return-nonzero\n")?;
        return Ok(3);
    }
    append_at(&stages, "return-clean\n")?;
    Ok(0)
}

/// The ordinary child for Gate A: live long enough to be observed, then exit.
fn run_ordinary_child() -> Result<i32, String> {
    sleep_ms(1_500);
    Ok(0)
}

/// `denial --root DIR --kind credential|host-write|network|guard|peer`
///
/// Attempt one operation the candidate profile must refuse and record whether
/// it was denied. The observer reads `payload/denials`; the exit code always
/// reports that the helper itself completed, never whether the operation was
/// allowed.
fn run_denial(args: &[String]) -> Result<i32, String> {
    let root = root_of(args)?;
    let kind = required(args, "--kind")?;
    let payload = root.join("payload");
    fs::create_dir_all(&payload).map_err(|error| format!("payload dir: {error}"))?;
    let (denied, detail) = match kind.as_str() {
        "credential" => attempt_credential_read(),
        "data-volume-credential" => attempt_data_volume_credential_read(),
        "host-write" => attempt_host_write(),
        "network" => attempt_network(),
        "guard" => {
            let label = read_trimmed(&root.join("inputs/guard-label")).unwrap_or_default();
            attempt_launchctl(&["kill", "SIGTERM", target(&label).as_str()])
        }
        "peer" => {
            let label = read_trimmed(&root.join("inputs/peer-label")).unwrap_or_default();
            attempt_launchctl(&["bootout", target(&label).as_str()])
        }
        other => return Err(format!("unknown denial kind {other}")),
    };
    append_at(
        &payload.join("denials"),
        &format!(
            "{kind} {} {detail}\n",
            if denied { "denied" } else { "allowed" }
        ),
    )?;
    Ok(0)
}

/// A read of a host credential file must not return bytes; any failure or
/// permission denial counts as denied. The direct and `/private` spellings,
/// plus the `/System/Volumes/Data` spelling of the credential target, come from
/// the one shared control-target source.
fn attempt_credential_read() -> (bool, String) {
    for path in controls::CREDENTIAL_READ_DENIAL_TARGETS {
        match fs::read(path) {
            Ok(_) => return (false, format!("{path} read succeeded")),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                return (true, format!("{path} denied EACCES/EPERM"));
            }
            Err(_) => {}
        }
    }
    (true, "no readable credential file".to_string())
}

/// The data-volume spelling of the credential target must stay denied exactly
/// as the direct spelling is.
fn attempt_data_volume_credential_read() -> (bool, String) {
    let path = controls::DATA_VOLUME_CREDENTIAL_READ_DENIAL_TARGET;
    match fs::read(path) {
        Ok(_) => (false, format!("{path} read succeeded")),
        Err(error) => {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                (true, format!("{path} denied EACCES/EPERM"))
            } else {
                (true, format!("{path} denied: {error}"))
            }
        }
    }
}

/// A write outside the payload must not create a host file.
fn attempt_host_write() -> (bool, String) {
    let path = controls::HOST_WRITE_DENIAL_TARGET;
    match OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
    {
        Ok(_) => {
            let _ = fs::remove_file(path);
            (false, format!("{path} write succeeded"))
        }
        Err(error) => (true, format!("{path} denied: {error}")),
    }
}

/// Binding a loopback listener must fail without network authority.
fn attempt_network() -> (bool, String) {
    match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => {
            drop(listener);
            (false, "loopback bind succeeded".to_string())
        }
        Err(error) => (true, format!("loopback bind denied: {error}")),
    }
}

fn attempt_launchctl(args: &[&str]) -> (bool, String) {
    match attack_launchctl(args) {
        Ok(()) => (false, "launchctl succeeded".to_string()),
        Err(error) => (true, error),
    }
}

// ---------------------------------------------------------------------------
// Gate B: payload, descendants and attacks
// ---------------------------------------------------------------------------

/// `payload --root DIR --case CASE --nonce NONCE`
fn run_payload(args: &[String]) -> Result<i32, String> {
    // The staged helper spelling is read before any stage or exec.
    let helper = helper_exe(args)?;
    let root = root_of(args)?;
    let case = required(args, "--case")?;
    let payload = root.join("payload");
    fs::create_dir_all(&payload).map_err(|error| format!("payload dir: {error}"))?;
    record_identity(&payload, "payload")?;

    match case.as_str() {
        "ordinary-child" => {
            heartbeat(&payload);
            Ok(0)
        }
        "guard-interference" => {
            let label = read_trimmed(&root.join("inputs/guard-label"))?;
            let destination = target(&label);
            record_attempt(
                &payload,
                "guard-kill",
                &attack_launchctl(&["kill", "SIGTERM", destination.as_str()]),
            )?;
            sleep_ms(1_000);
            heartbeat(&payload);
            Ok(0)
        }
        "peer-bootout" => {
            let label = read_trimmed(&root.join("inputs/peer-label"))?;
            let destination = target(&label);
            record_attempt(
                &payload,
                "peer-bootout",
                &attack_launchctl(&["bootout", destination.as_str()]),
            )?;
            sleep_ms(1_000);
            heartbeat(&payload);
            Ok(0)
        }
        "escape-job" => {
            let escape = root.join("payload/escape.plist");
            let escape_label = flag(args, "--escape-label")
                .unwrap_or_else(|| "org.brokkr.seatbelt.probe.escape".to_string());
            write_at(&escape, &escape_plist(&escape_label))?;
            let domain = domain();
            let escape_path = escape.to_string_lossy().to_string();
            record_attempt(
                &payload,
                "escape-bootstrap",
                &attack_launchctl(&["bootstrap", domain.as_str(), escape_path.as_str()]),
            )?;
            sleep_ms(1_000);
            heartbeat(&payload);
            Ok(0)
        }
        "parent-exit" => {
            let exe = helper.clone();
            let _child = Command::new(&exe)
                .args([
                    "detach-leaf",
                    "--root",
                    &root.to_string_lossy(),
                    "--tag",
                    "ordinary-child",
                ])
                .stdin(Stdio::null())
                .spawn()
                .map_err(|error| format!("parent-exit child: {error}"))?;
            Ok(0)
        }
        "double-fork" => {
            let exe = helper.clone();
            let _child = detach_spawn(Command::new(&exe).args([
                "detach-child",
                "--root",
                &root.to_string_lossy(),
            ]))?;
            Ok(0)
        }
        "ignored-signals" => {
            let exe = helper.clone();
            let _child = detach_spawn(Command::new(&exe).args([
                "detach-leaf",
                "--root",
                &root.to_string_lossy(),
                "--tag",
                "leaf",
                "--ignore-signals",
            ]))?;
            Ok(0)
        }
        "retained-pipes" => {
            let exe = helper.clone();
            let _child = detach_spawn(
                Command::new(&exe)
                    .args([
                        "detach-leaf",
                        "--root",
                        &root.to_string_lossy(),
                        "--tag",
                        "leaf",
                    ])
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit()),
            )?;
            Ok(0)
        }
        // timeout, cancellation and supervisor-death each detach one `setsid`
        // descendant; the observer applies a different real trigger to each.
        _ => {
            let exe = helper.clone();
            let _child = detach_spawn(Command::new(&exe).args([
                "detach-leaf",
                "--root",
                &root.to_string_lossy(),
                "--tag",
                "leaf",
            ]))?;
            Ok(0)
        }
    }
}

/// The double-fork intermediate: new session, record itself, spawn the leaf,
/// then exit so only the reparented leaf remains.
fn run_detach_child(args: &[String]) -> Result<i32, String> {
    let helper = helper_exe(args)?;
    let root = root_of(args)?;
    record_identity(&root.join("payload"), "fork-child")?;
    let _child = Command::new(&helper)
        .args([
            "detach-leaf",
            "--root",
            &root.to_string_lossy(),
            "--tag",
            "leaf",
        ])
        .stdin(Stdio::null())
        .spawn()
        .map_err(|error| format!("detach leaf: {error}"))?;
    Ok(0)
}

/// A detached descendant: record its identity, optionally ignore signals,
/// then heartbeat until it is killed or the probe ends.
fn run_detach_leaf(args: &[String]) -> Result<i32, String> {
    let root = root_of(args)?;
    let tag = flag(args, "--tag").unwrap_or_else(|| "leaf".to_string());
    let payload = root.join("payload");
    fs::create_dir_all(&payload).map_err(|error| format!("payload dir: {error}"))?;
    record_identity(&payload, &tag)?;
    if flag(args, "--ignore-signals").is_some() {
        ignore_termination_signals();
    }
    heartbeat(&payload);
    Ok(0)
}

fn record_identity(payload: &Path, tag: &str) -> Result<(), String> {
    let pid = std::process::id();
    let pgid = process_group();
    append_at(
        &payload.join("identities"),
        &format!("{tag} {pid} {pgid}\n"),
    )
}

fn record_attempt(payload: &Path, kind: &str, outcome: &Result<(), String>) -> Result<(), String> {
    let (status, detail) = match outcome {
        Ok(()) => ("ok".to_string(), String::new()),
        Err(error) => ("failed".to_string(), error.clone()),
    };
    append_at(
        &payload.join("attempts"),
        &format!("{kind} {status} {detail}\n"),
    )
}

/// Run `launchctl` from the payload path. Under the candidate Seatbelt
/// profile the launchd bootstrap port is unreachable, so this is expected to
/// fail; the observer records that the attempt happened before the denial.
fn attack_launchctl(args: &[&str]) -> Result<(), String> {
    let out = Command::new("/bin/launchctl")
        .args(args)
        .output()
        .map_err(|error| format!("launchctl: {error}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn domain() -> String {
    format!("gui/{}", current_uid())
}

fn target(label: &str) -> String {
    format!("{}/{label}", domain())
}

// The helper is only ever selected on macOS, but it is compiled and linked on
// every workspace host so a non-Darwin build cannot hide drift. The Unix ABI
// calls are therefore target-gated: the Windows binary must link without a
// `getuid`/`getpgid` reference (native CI run `34441725835` proved it did not).
#[cfg(unix)]
fn current_uid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    // SAFETY: `getuid` has no preconditions and cannot fail.
    unsafe { getuid() }
}

#[cfg(not(unix))]
fn current_uid() -> u32 {
    // Off Unix the launchd domain and its targets are never constructed; the
    // stub keeps the shared code compiled and linked without the symbol.
    0
}

#[cfg(unix)]
fn process_group() -> u32 {
    extern "C" {
        fn getpgid(pid: i32) -> i32;
    }
    // SAFETY: `getpgid` is called for the current process and cannot fail.
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

fn heartbeat(payload: &Path) {
    let path = payload.join("heartbeat");
    loop {
        let stamp = format!("{:?}", Instant::now());
        let _ = fs::write(&path, &stamp);
        sleep_ms(50);
    }
}

fn read_trimmed(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map(|text| text.trim().to_string())
        .map_err(|error| format!("{}: {error}", path.display()))
}

fn escape_plist(label: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <plist version=\"1.0\"><dict>\n\
         <key>Label</key><string>{label}</string>\n\
         <key>ProgramArguments</key><array><string>/bin/sleep</string><string>300</string></array>\n\
         <key>RunAtLoad</key><true/>\n\
         </dict></plist>\n"
    )
}

// ---------------------------------------------------------------------------
// Gate B: the separately launchd-owned guard
// ---------------------------------------------------------------------------

/// `guard --root DIR --payload-label LABEL --guard-label LABEL --uid UID`
fn run_guard(args: &[String]) -> Result<i32, String> {
    let root = root_of(args)?;
    let payload_label = required(args, "--payload-label")?;
    let guard_label = required(args, "--guard-label")?;
    let uid = required(args, "--uid")?;
    let guard = root.join("guard");
    fs::create_dir_all(&guard).map_err(|error| format!("guard dir: {error}"))?;
    write_at(&guard.join("registered"), "guard\n")?;

    // Wait for the engine's liveness write end to close (explicit trigger or
    // abrupt supervisor death).
    wait_liveness(&root.join("liveness"));

    let bootout = Command::new("/bin/launchctl")
        .args(["bootout", &format!("gui/{uid}/{payload_label}")])
        .output()
        .map_err(|error| format!("guard bootout: {error}"))?;
    write_at(
        &guard.join("bootout"),
        &format!(
            "status={} stdout={} stderr={}",
            bootout.status,
            String::from_utf8_lossy(&bootout.stdout).trim(),
            String::from_utf8_lossy(&bootout.stderr).trim()
        ),
    )?;

    // Payload job absence plus one full second of heartbeat quiet is the
    // candidate quiescence event; the observer corroborates it before
    // releasing cleanup.
    let quiesced = wait_quiet(&root.join("payload/heartbeat"), Duration::from_secs(10));
    if !quiesced {
        write_at(&guard.join("quiescence-failed"), "heartbeat kept moving\n")?;
        return Ok(4);
    }
    write_at(&guard.join("quiesced"), "quiesced\n")?;

    // The payload cannot make this durable; the observer must release it.
    if !wait_for_file(&root.join("observer/release"), Duration::from_secs(15)) {
        write_at(
            &guard.join("release-timeout"),
            "observer never corroborated\n",
        )?;
        return Ok(5);
    }

    // Private state may be removed only now.
    let _ = fs::remove_dir_all(root.join("payload"));
    write_at(&guard.join("cleanup-done"), "cleanup\n")?;
    let _ = Command::new("/bin/launchctl")
        .args(["bootout", &format!("gui/{uid}/{guard_label}")])
        .output();
    Ok(0)
}

fn wait_liveness(path: &Path) {
    // Block until the writer opens, then drain until it closes. A missing
    // FIFO is a harness failure, not a containment result.
    if let Ok(file) = fs::OpenOptions::new().read(true).open(path) {
        let _ = std::io::Read::read_to_end(&mut std::io::BufReader::new(file), &mut Vec::new());
    }
}

fn wait_quiet(path: &Path, timeout: Duration) -> bool {
    let started = Instant::now();
    let mut last = String::new();
    let mut stable = 0;
    while started.elapsed() < timeout {
        let current = fs::read_to_string(path).unwrap_or_default();
        if current == last {
            stable += 1;
            if stable >= 10 {
                return true;
            }
        } else {
            stable = 0;
            last = current;
        }
        sleep_ms(100);
    }
    false
}

fn wait_for_file(path: &Path, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if path.is_file() {
            return true;
        }
        sleep_ms(50);
    }
    false
}

/// `holder --liveness PATH`: hold the write end open until the observer kills
/// this process.
fn run_holder(args: &[String]) -> Result<i32, String> {
    let path = PathBuf::from(required(args, "--liveness")?);
    let _file = OpenOptions::new()
        .write(true)
        .open(&path)
        .map_err(|error| format!("holder open: {error}"))?;
    sleep_ms(3_600_000);
    Ok(0)
}

// ---------------------------------------------------------------------------
// Unix primitives
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn detach_spawn(command: &mut Command) -> Result<std::process::Child, String> {
    use std::os::unix::process::CommandExt;
    // SAFETY: `setsid` is async-signal-safe and called between fork and exec
    // in a single-threaded child; it detaches the descendant into a new
    // session so an original-process-group kill cannot reach it.
    unsafe {
        command.pre_exec(|| {
            extern "C" {
                fn setsid() -> i32;
            }
            if setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn().map_err(|error| format!("detach: {error}"))
}

#[cfg(not(unix))]
fn detach_spawn(command: &mut Command) -> Result<std::process::Child, String> {
    command.spawn().map_err(|error| format!("detach: {error}"))
}

#[cfg(unix)]
fn ignore_termination_signals() {
    extern "C" {
        fn signal(signal: i32, handler: usize) -> usize;
    }
    const SIG_IGN: usize = 1;
    const SIGTERM: i32 = 15;
    const SIGINT: i32 = 2;
    const SIGHUP: i32 = 1;
    // SAFETY: installing SIG_IGN for the standard termination signals has no
    // memory-safety preconditions.
    unsafe {
        signal(SIGTERM, SIG_IGN);
        signal(SIGINT, SIG_IGN);
        signal(SIGHUP, SIG_IGN);
    }
}

#[cfg(not(unix))]
fn ignore_termination_signals() {}
