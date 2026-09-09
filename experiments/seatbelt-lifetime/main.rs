//! Standalone, std-only falsification experiment. Not a production boundary.
//! Compile with rustc; see README.md for limits and the native runner.
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::process::CommandExt;

    unsafe extern "C" {
        fn fork() -> i32;
        fn setsid() -> i32;
        fn getpid() -> i32;
        fn getpgrp() -> i32;
        fn kill(pid: i32, sig: i32) -> i32;
        fn signal(sig: i32, handler: usize) -> usize;
        fn _exit(status: i32) -> !;
    }

    const TICK: Duration = Duration::from_millis(25);
    const TEARDOWN: Duration = Duration::from_secs(5);
    const SELF_EXPIRY: Duration = Duration::from_secs(30);
    const SIGKILL: i32 = 9;

    fn error(message: &str) -> io::Error {
        io::Error::other(message)
    }

    fn wait_for(path: &Path, budget: Duration) -> io::Result<()> {
        let start = Instant::now();
        while !path.is_file() {
            if start.elapsed() >= budget {
                return Err(error(&format!("readiness timeout: {}", path.display())));
            }
            sleep(TICK);
        }
        Ok(())
    }

    fn atomic(path: &Path, body: &str) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, body)?;
        fs::rename(tmp, path)
    }

    // All fork calls occur in this fresh, single-threaded helper process.
    // Helpers never exec arbitrary commands and never enumerate host processes.
    fn heartbeat(root: &Path, role: &str) -> io::Result<()> {
        let pid = unsafe { getpid() };
        let group = unsafe { getpgrp() };
        atomic(
            &root.join(format!("{role}.identity")),
            &format!("{pid} {group}\n"),
        )?;
        let start = Instant::now();
        let mut count = 0u64;
        loop {
            if root.join("cleanup").exists() {
                return Ok(());
            }
            if start.elapsed() >= SELF_EXPIRY {
                atomic(
                    &root.join(format!("{role}.expired")),
                    "fixture safety expiry\n",
                )?;
                return Ok(());
            }
            if role == "root" && root.join("parent-exit").exists() {
                return Ok(());
            }
            count += 1;
            atomic(
                &root.join(format!("{role}.heartbeat")),
                &format!("{count}\n"),
            )?;
            sleep(TICK);
        }
    }

    fn payload(root: &Path, topology: &str) -> io::Result<()> {
        // SIG_IGN = 1 on the supported Unix hosts. SIGKILL is never intercepted.
        unsafe {
            signal(15, 1);
        }
        let child = unsafe { fork() };
        if child < 0 {
            return Err(io::Error::last_os_error());
        }
        if child == 0 {
            if topology != "ordinary" && unsafe { setsid() } < 0 {
                unsafe { _exit(71) }
            }
            if topology == "double-fork" {
                let grandchild = unsafe { fork() };
                if grandchild < 0 {
                    unsafe { _exit(72) }
                }
                if grandchild > 0 {
                    unsafe { _exit(0) }
                }
            }
            let status = if heartbeat(root, "leaf").is_ok() {
                0
            } else {
                73
            };
            unsafe { _exit(status) }
        }
        heartbeat(root, "root")
    }

    fn supervisor(root: &Path, topology: &str, native: bool, control: bool) -> io::Result<()> {
        let exe = env::current_exe()?;
        let mut command = if native {
            let mut c = Command::new("/usr/bin/sandbox-exec");
            // Intentionally no filesystem/network claim: isolate lifetime semantics.
            c.args(["-p", "(version 1) (allow default)"]).arg(&exe);
            c
        } else {
            Command::new(&exe)
        };
        let mut child = command
            .arg("payload")
            .arg(root)
            .arg(topology)
            .process_group(0)
            .stdin(Stdio::null())
            .spawn()?;
        atomic(&root.join("group"), &child.id().to_string())?;
        wait_for(&root.join("armed"), Duration::from_secs(12))?;
        let started = Instant::now();
        // Retain the unreaped leader until group signalling: its PID cannot be reused.
        // parent-exit is detected by fixture notification, not wait/try_wait reaping.
        loop {
            if root.join("cleanup").exists() {
                break;
            }
            if root.join("cancel").exists()
                || root.join("parent-exit").exists()
                || (root.join("timeout-mode").exists()
                    && started.elapsed() >= Duration::from_millis(300))
            {
                if root.join("parent-exit").exists() {
                    // Observe parent exit without reaping the group leader before signalling.
                    let deadline = Instant::now() + Duration::from_secs(2);
                    let id = identity(root, "root")?;
                    while alive(id, root)? {
                        if Instant::now() >= deadline {
                            return Err(error("direct parent did not exit"));
                        }
                        sleep(TICK);
                    }
                }
                if !control {
                    let result = unsafe { kill(-(child.id() as i32), SIGKILL) };
                    if result != 0 {
                        return Err(io::Error::last_os_error());
                    }
                }
                atomic(
                    &root.join("action"),
                    if control {
                        "no cleanup control\n"
                    } else {
                        "SIGKILL original process group\n"
                    },
                )?;
                // Even the deliberately broken control remains bounded by fixture expiry.
                let _ = child.wait()?;
                return Ok(());
            }
            sleep(TICK);
        }
        let _ = child.wait()?;
        Ok(())
    }

    #[derive(Clone, Copy, Debug)]
    struct Identity {
        pid: i32,
        group: i32,
    }

    fn identity(root: &Path, role: &str) -> io::Result<Identity> {
        let body = fs::read_to_string(root.join(format!("{role}.identity")))?;
        let parts = body
            .split_whitespace()
            .map(str::parse::<i32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| error("invalid helper identity"))?;
        if parts.len() != 2 || parts[0] <= 1 || parts[1] <= 1 {
            return Err(error("invalid helper identity"));
        }
        Ok(Identity {
            pid: parts[0],
            group: parts[1],
        })
    }

    // Fixtures do not exec/change their argv. A unique per-case root in argv
    // distinguishes a recycled PID without signalling it. Zombies cannot execute;
    // stopped processes CAN resume and remain survivors. Observation errors fail closed.
    fn alive(id: Identity, root: &Path) -> io::Result<bool> {
        let mut child = Command::new("/bin/ps")
            .args([
                "-ww",
                "-p",
                &id.pid.to_string(),
                "-o",
                "stat=",
                "-o",
                "args=",
            ])
            .env("LC_ALL", "C")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(1);
        while child.try_wait()?.is_none() {
            if Instant::now() >= deadline {
                child.kill()?;
                let _ = child.wait()?;
                return Err(error("ps observer exceeded one-second budget"));
            }
            sleep(TICK);
        }
        let output = child.wait_with_output()?;
        let text = String::from_utf8(output.stdout).map_err(|_| error("non-UTF8 ps output"))?;
        if text.trim().is_empty() {
            if output.status.success() || output.status.code() == Some(1) {
                return Ok(false);
            }
            return Err(error("ps observation failed"));
        }
        if !output.status.success() {
            return Err(error("ps observation failed"));
        }
        let state = text
            .split_whitespace()
            .next()
            .ok_or_else(|| error("missing process state"))?;
        if state.starts_with('Z') {
            return Ok(false);
        }
        if !text.contains(&root.to_string_lossy().to_string()) {
            return Ok(false);
        }
        Ok(true)
    }

    struct Fixture {
        root: PathBuf,
        supervisor: Child,
    }
    impl Fixture {
        fn cleanup(&mut self) -> io::Result<()> {
            atomic(
                &self.root.join("cleanup"),
                "test observer cleanup; never containment evidence\n",
            )?;
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(2) {
                if self.supervisor.try_wait()?.is_some() {
                    return Ok(());
                }
                sleep(TICK);
            }
            // Direct Child is unreaped and owned: cannot target a recycled PID.
            self.supervisor.kill()?;
            let _ = self.supervisor.wait()?;
            Ok(())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = self.cleanup();
        }
    }

    fn json_string(text: &str) -> String {
        let mut out = String::from("\"");
        for c in text.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    fn case(
        dir: &Path,
        topology: &str,
        trigger: &str,
        native: bool,
        control: bool,
    ) -> io::Result<bool> {
        fs::create_dir(dir)?;
        let supervisor = Command::new(env::current_exe()?)
            .arg("supervisor")
            .arg(dir)
            .arg(topology)
            .arg(if native { "native" } else { "portable" })
            .arg(if control { "control" } else { "candidate" })
            .stdin(Stdio::null())
            .stdout(Stdio::from(fs::File::create(dir.join("stdout.txt"))?))
            .stderr(Stdio::from(fs::File::create(dir.join("stderr.txt"))?))
            .spawn()?;
        let mut fixture = Fixture {
            root: dir.to_owned(),
            supervisor,
        };
        let result = (|| {
            wait_for(&dir.join("root.identity"), Duration::from_secs(3))?;
            wait_for(&dir.join("leaf.identity"), Duration::from_secs(3))?;
            let root = identity(dir, "root")?;
            let leaf = identity(dir, "leaf")?;
            let group = fs::read_to_string(dir.join("group"))?
                .parse::<i32>()
                .map_err(|_| error("invalid group"))?;
            if root.group != group || (topology == "ordinary") != (leaf.group == group) {
                return Err(error("helper did not establish requested process topology"));
            }
            // Require live helpers and advancing heartbeats before applying the trigger.
            wait_for(&dir.join("root.heartbeat"), Duration::from_secs(1))?;
            wait_for(&dir.join("leaf.heartbeat"), Duration::from_secs(1))?;
            let before = fs::read(dir.join("leaf.heartbeat"))?;
            sleep(Duration::from_millis(150));
            if !alive(root, dir)?
                || !alive(leaf, dir)?
                || before == fs::read(dir.join("leaf.heartbeat"))?
            {
                return Err(error("positive liveness control failed"));
            }
            if trigger == "timeout" {
                atomic(&dir.join("timeout-mode"), "1")?;
            }
            atomic(&dir.join("armed"), "1")?;
            match trigger {
                "timeout" => {}
                "cancel" => atomic(&dir.join("cancel"), "1")?,
                "parent-exit" => atomic(&dir.join("parent-exit"), "1")?,
                "supervisor-death" => {
                    fixture.supervisor.kill()?;
                    let _ = fixture.supervisor.wait()?;
                }
                _ => return Err(error("invalid trigger")),
            }
            let observed_start = Instant::now();
            if trigger != "supervisor-death" {
                wait_for(&dir.join("action"), Duration::from_secs(3))?;
            }
            let deadline = Instant::now() + TEARDOWN;
            let mut survivors;
            loop {
                survivors = [alive(root, dir)?, alive(leaf, dir)?];
                if !survivors.iter().any(|v| *v) || Instant::now() >= deadline {
                    break;
                }
                sleep(Duration::from_millis(100));
            }
            let stopped_heartbeat = fs::read(dir.join("leaf.heartbeat"))?;
            sleep(Duration::from_secs(1));
            let heartbeat_changed = stopped_heartbeat != fs::read(dir.join("leaf.heartbeat"))?;
            let expired = dir.join("root.expired").exists() || dir.join("leaf.expired").exists();
            let live_after = [alive(root, dir)?, alive(leaf, dir)?];
            if expired {
                return Err(error(
                    "fixture safety expiry preceded verdict; invalid measurement",
                ));
            }
            let clean =
                !survivors.iter().chain(live_after.iter()).any(|v| *v) && !heartbeat_changed;
            atomic(&dir.join("result.json"), &format!(
                "{{\"topology\":{},\"trigger\":{},\"control\":{},\"native\":{},\"root_pid\":{},\"leaf_pid\":{},\"root_group\":{},\"leaf_group\":{},\"root_survived\":{},\"leaf_survived\":{},\"heartbeat_changed\":{},\"observed_ms\":{},\"no_survivor_observed\":{},\"residual\":{}}}\n",
                json_string(topology), json_string(trigger), control, native, root.pid, leaf.pid, root.group, leaf.group,
                survivors[0] || live_after[0], survivors[1] || live_after[1], heartbeat_changed, observed_start.elapsed().as_millis(), clean,
                if clean { "null" } else { "\"SEATBELT-R3\"" }
            ))?;
            Ok(clean)
        })();
        let cleanup = fixture.cleanup();
        // Verify observer cleanup independently, after the verdict. It cannot erase a residual.
        let until = Instant::now() + Duration::from_secs(2);
        loop {
            let mut live = false;
            for role in ["root", "leaf"] {
                if let Ok(id) = identity(dir, role) {
                    live |= alive(id, dir)?;
                }
            }
            if !live {
                break;
            }
            if Instant::now() >= until {
                return Err(error(
                    "fixture cleanup incomplete; helpers self-expire within 30 seconds",
                ));
            }
            sleep(TICK);
        }
        cleanup?;
        atomic(
            &dir.join("cleanup-verified"),
            "fixture cleanup verified after measurement\n",
        )?;
        result
    }

    pub fn run(args: &[String]) -> io::Result<u8> {
        match args.first().map(String::as_str) {
            Some("payload") if args.len() == 3 => {
                payload(Path::new(&args[1]), &args[2])?;
                return Ok(0);
            }
            Some("supervisor") if args.len() == 5 => {
                supervisor(
                    Path::new(&args[1]),
                    &args[2],
                    args[3] == "native",
                    args[4] == "control",
                )?;
                return Ok(0);
            }
            Some("--native") | Some("--portable-self-test") if args.len() == 2 => {}
            _ => {
                return Err(error(
                    "usage: lifetime --native|--portable-self-test NEW_OUTPUT_DIRECTORY",
                ))
            }
        }
        let native = args[0] == "--native";
        if native && !cfg!(target_os = "macos") {
            return Err(error("native measurement requires macOS"));
        }
        let dir = PathBuf::from(&args[1]);
        fs::create_dir(&dir)?; // never overwrite earlier evidence
        let dir = dir.canonicalize()?;
        // A known-broken control must exhibit a survivor or the observer is not credible.
        let control = case(
            &dir.join("negative-control"),
            "ordinary",
            "cancel",
            native,
            true,
        );
        let control_ok = matches!(control, Ok(false));
        let mut errors = Vec::new();
        if !control_ok {
            errors.push(format!("negative-control: {control:?}"));
        }
        let mut count = 0;
        let mut residuals = 0;
        let mut expected = true;
        let topologies = ["ordinary", "setsid", "double-fork"];
        let triggers = ["timeout", "cancel", "parent-exit", "supervisor-death"];
        if control_ok {
            for topology in topologies {
                for trigger in &triggers {
                    count += 1;
                    let name = format!("{topology}-{trigger}");
                    let outcome = case(&dir.join(&name), topology, trigger, native, false);
                    println!(
                        "{name}: {}",
                        match &outcome {
                            Ok(true) => "no survivor observed in this case".to_string(),
                            Ok(false) => "RESIDUAL: payload survived".to_string(),
                            Err(e) => format!("ERROR: {e}"),
                        }
                    );
                    match outcome {
                        Ok(clean) => {
                            residuals += usize::from(!clean);
                            expected &=
                                clean == (topology == "ordinary" && *trigger != "supervisor-death");
                        }
                        Err(e) => {
                            expected = false;
                            errors.push(format!("{name}: {e}"));
                        }
                    }
                }
            }
        }
        if !expected {
            errors.push("unexpected process-group outcome; investigate fixture failure or external interference before interpreting containment".to_string());
        }
        let valid = control_ok && errors.is_empty() && count == 12;
        let json_errors = errors
            .iter()
            .map(|s| json_string(s))
            .collect::<Vec<_>>()
            .join(",");
        atomic(&dir.join("summary.json"), &format!(
            "{{\"schema\":\"brokkr.seatbelt-lifetime-experiment/v1\",\"native\":{native},\"candidate\":\"original-process-group\",\"cases\":{count},\"negative_control_detected\":{control_ok},\"valid_measurement\":{valid},\"residual_cases\":{residuals},\"errors\":[{json_errors}],\"seatbelt_activation_authorized\":false,\"r3_closed\":false}}\n"
        ))?;
        if native {
            // Even a zero-survivor experiment is not arbitrary-payload or hands integration proof.
            Ok(if valid { 1 } else { 2 })
        } else {
            Ok(if valid && expected { 0 } else { 2 })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn diagnostic_json_escapes_paths_and_control_characters() {
            assert_eq!(
                json_string("quote\" slash\\\n\t\u{0}"),
                "\"quote\\\" slash\\\\\\n\\t\\u0000\""
            );
        }
        #[test]
        fn invalid_modes_do_not_create_output() {
            assert!(run(&[]).is_err());
            assert!(run(&["--native".into()]).is_err());
        }
    }
}

fn main() -> ExitCode {
    #[cfg(unix)]
    match unix::run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("experiment error: {error}");
            ExitCode::from(2)
        }
    }
    #[cfg(not(unix))]
    {
        eprintln!("requires Unix; native evidence requires macOS");
        ExitCode::from(2)
    }
}
