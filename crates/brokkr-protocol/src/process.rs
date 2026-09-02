//! Subprocess transport for `forge-driver/v1`: spawn the driver command,
//! handshake, offer the seat's prior session to a driver that declared
//! it can rejoin one, send `start`, and drive the attempt to a terminal
//! outcome.
//! Every protocol violation degrades to `Failed` (driver defect, retry
//! is a new attempt); a silent exit degrades to `Indeterminate`; a
//! deadline expiry kills the driver and degrades to `Failed` — the kill
//! makes non-completion determinate, so bounded retry stays safe
//! (decision 0006).

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::{AttemptOutcome, AttemptReport, Body, Message, ResultStatus, PROTO};

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("driver command is empty")]
    EmptyCommand,
    #[error("failed to spawn driver {command}: {source}")]
    Spawn {
        command: String,
        source: std::io::Error,
    },
}

pub struct DriverProcess {
    child: Arc<Mutex<Child>>,
    stdin: Box<dyn Write>,
    stdout: BufReader<std::process::ChildStdout>,
    stderr_thread: std::thread::JoinHandle<String>,
    timed_out: Arc<AtomicBool>,
    /// Dropping this disarms the watchdog.
    watchdog_disarm: Option<mpsc::Sender<()>>,
    deadline: Option<Duration>,
}

/// The deadline kill. Its job is to make non-completion determinate
/// (decision 0006), and a kill the harness cannot observe the end of is
/// not determinate: the driver's stdout/stderr are pipes, and EOF only
/// arrives when the last copy of the write end is gone.
///
/// On unix `Child::kill()` is that kill, unchanged — SIGKILL on the
/// process the harness holds. On Windows it is not enough. A driver
/// that spawned anything of its own leaves those grandchildren holding
/// the inherited handles, so the reader waits on an EOF that cannot
/// come and `wait()` never returns: the observed failure was a runner
/// hung for forty minutes on a driver blocked reading stdin, not a
/// deadline. So the tree goes first, by PID, while the driver is still
/// alive to be the root of it — after the direct kill the PID is dead
/// and `/T` has no tree to walk. `taskkill` finding nothing to kill is
/// not a failure here (the driver may have exited on its own between
/// the deadline and the lock), so its status and its output are both
/// discarded; the direct kill below is the backstop that still holds if
/// `taskkill` could not be run at all.
///
/// `taskkill` is named by absolute path, not by bare name: Windows
/// resolves a bare program name against the calling process's current
/// directory as well as `PATH`, and the harness's current directory is
/// a repository that seats write into. Resolving from `%SystemRoot%`
/// keeps a file a seat dropped in the working tree out of the kill
/// path.
fn kill_driver(child: &mut Child) {
    #[cfg(windows)]
    {
        let system_root =
            std::env::var("SystemRoot").unwrap_or_else(|_| String::from(r"C:\Windows"));
        let taskkill = std::path::Path::new(&system_root)
            .join("System32")
            .join("taskkill.exe");
        let _ = Command::new(taskkill)
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

impl DriverProcess {
    /// Spawn the driver. With a deadline, a watchdog kills the child when
    /// it expires; the attempt then reports Failed(timeout) rather than
    /// hanging the run forever.
    pub fn spawn(
        command: &[String],
        workdir: &std::path::Path,
        deadline: Option<Duration>,
    ) -> Result<Self, SpawnError> {
        let (program, args) = command.split_first().ok_or(SpawnError::EmptyCommand)?;
        let mut child = Command::new(program)
            .args(args)
            .current_dir(workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|source| SpawnError::Spawn {
                command: command.join(" "),
                source,
            })?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        let mut stderr = child.stderr.take().expect("piped stderr");
        let stderr_thread = std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = stderr.read_to_string(&mut buf);
            buf
        });
        let child = Arc::new(Mutex::new(child));
        let timed_out = Arc::new(AtomicBool::new(false));
        let watchdog_disarm = deadline.map(|deadline| {
            let (tx, rx) = mpsc::channel::<()>();
            let child = Arc::clone(&child);
            let timed_out = Arc::clone(&timed_out);
            std::thread::spawn(move || {
                if let Err(mpsc::RecvTimeoutError::Timeout) = rx.recv_timeout(deadline) {
                    timed_out.store(true, Ordering::SeqCst);
                    if let Ok(mut child) = child.lock() {
                        kill_driver(&mut child);
                    }
                }
            });
            tx
        });
        Ok(DriverProcess {
            child,
            stdin: Box::new(stdin),
            stdout,
            stderr_thread,
            timed_out,
            watchdog_disarm,
            deadline,
        })
    }

    fn send(&mut self, body: Body) -> std::io::Result<()> {
        let line = serde_json::to_string(&Message::new(body))?;
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()
    }

    fn recv(&mut self) -> Option<Result<Message, String>> {
        let mut line = String::new();
        loop {
            line.clear();
            match self.stdout.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    return Some(
                        serde_json::from_str::<Message>(&line)
                            .map_err(|e| format!("unreadable driver message: {e}: {line}"))
                            .and_then(|m| {
                                if m.proto == PROTO {
                                    Ok(m)
                                } else {
                                    Err(format!("driver spoke '{}', want '{PROTO}'", m.proto))
                                }
                            }),
                    );
                }
                Err(e) => return Some(Err(format!("driver stdout read failed: {e}"))),
            }
        }
    }

    fn finish(
        mut self,
        outcome: AttemptOutcome,
        session_ref: Option<String>,
        checkpoints: Vec<Value>,
        accepted: bool,
    ) -> AttemptReport {
        // Disarm the watchdog before shutdown so a slow exit is not killed.
        drop(self.watchdog_disarm.take());
        let _ = self.send(Body::Shutdown);
        drop(self.stdin);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.wait();
        }
        let stderr = self.stderr_thread.join().unwrap_or_default();
        AttemptReport {
            outcome,
            session_ref,
            checkpoints,
            stderr,
            accepted,
        }
    }

    /// The outcome for an EOF: a deadline kill is a determinate failure
    /// (we killed it before any result); anything else depends on
    /// whether the attempt was accepted.
    fn eof_outcome(&self, accepted: bool) -> AttemptOutcome {
        if self.timed_out.load(Ordering::SeqCst) {
            let secs = self.deadline.map(|d| d.as_secs()).unwrap_or_default();
            return AttemptOutcome::Failed {
                error: format!("attempt exceeded its {secs}s deadline and was killed"),
            };
        }
        let reason = if accepted {
            "driver exited after accepting, before a result — attempt \
             cannot be established as complete"
        } else {
            "driver exited before accepting the attempt"
        };
        AttemptOutcome::Indeterminate {
            reason: reason.into(),
        }
    }

    /// Run one attempt to a terminal outcome. `on_checkpoint` is called
    /// with each checkpoint payload so the engine can journal it before
    /// the attempt concludes.
    pub fn run_attempt(
        self,
        engine_version: &str,
        effect_id: &str,
        attempt_id: &str,
        seat: &str,
        input: Value,
        on_checkpoint: impl FnMut(&Value),
    ) -> AttemptReport {
        self.run_attempt_resuming(
            engine_version,
            effect_id,
            attempt_id,
            seat,
            input,
            None,
            on_checkpoint,
        )
    }

    /// The same attempt, offering the seat's prior session first when the
    /// engine has one to hand back (decision 0030). The offer rides
    /// `resume`, ahead of the `start` that describes the work, and it
    /// reaches the driver ONLY when the driver's own `capabilities`
    /// declared `resume`: a session handle belongs to the credential and
    /// client that opened it, so it is never posted to a driver that did
    /// not say it knows what to do with one.
    #[allow(clippy::too_many_arguments)]
    pub fn run_attempt_resuming(
        mut self,
        engine_version: &str,
        effect_id: &str,
        attempt_id: &str,
        seat: &str,
        input: Value,
        offered: Option<String>,
        mut on_checkpoint: impl FnMut(&Value),
    ) -> AttemptReport {
        let mut session_ref: Option<String> = None;
        // Declared before the first refusal so every terminal path — the
        // handshake failures included — reports whether the driver ever
        // accepted. That single bit is the engine's fail-to-start
        // predicate; leaving it out of a path would let one shape of
        // failure lie about which side of the mid-session boundary it is
        // on.
        let mut accepted = false;
        let mut checkpoints: Vec<Value> = Vec::new();

        macro_rules! fail {
            ($($arg:tt)*) => {
                return self.finish(
                    AttemptOutcome::Failed { error: format!($($arg)*) },
                    None,
                    Vec::new(),
                    accepted,
                )
            };
        }

        if let Err(e) = self.send(Body::Hello {
            engine_version: engine_version.to_string(),
        }) {
            // A pipe broken at the greeting is a driver already gone —
            // the same fact as exiting without accepting, so it takes
            // the same arm instead of racing the driver's exit for
            // which error the operator reads.
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                let outcome = self.eof_outcome(false);
                return self.finish(outcome, None, Vec::new(), accepted);
            }
            fail!("could not greet driver: {e}");
        }
        let supports = match self.recv() {
            Some(Ok(Message {
                body: Body::Capabilities { supports, .. },
                ..
            })) => supports,
            Some(Ok(other)) => fail!("expected capabilities, got {:?}", other.body),
            Some(Err(e)) => fail!("{e}"),
            None => {
                let outcome = self.eof_outcome(false);
                return self.finish(outcome, None, Vec::new(), accepted);
            }
        };
        if let Some(session_ref) =
            offered.filter(|_| supports.iter().any(|feature| feature == "resume"))
        {
            if let Err(e) = self.send(Body::Resume {
                effect_id: effect_id.to_string(),
                attempt_id: attempt_id.to_string(),
                session_ref,
            }) {
                fail!("could not send resume: {e}");
            }
        }
        if let Err(e) = self.send(Body::Start {
            effect_id: effect_id.to_string(),
            attempt_id: attempt_id.to_string(),
            seat: seat.to_string(),
            input,
        }) {
            fail!("could not send start: {e}");
        }

        loop {
            match self.recv() {
                Some(Ok(message)) => match message.body {
                    Body::Accepted {
                        effect_id: eid,
                        session_ref: sref,
                        ..
                    } => {
                        if eid != effect_id {
                            fail!("driver accepted a different effect '{eid}'");
                        }
                        accepted = true;
                        session_ref = sref;
                    }
                    Body::Checkpoint {
                        data,
                        effect_id: eid,
                        ..
                    } => {
                        if eid != effect_id {
                            fail!("checkpoint for foreign effect '{eid}'");
                        }
                        on_checkpoint(&data);
                        checkpoints.push(data);
                    }
                    Body::Result {
                        effect_id: eid,
                        status,
                        result,
                        error,
                        ..
                    } => {
                        if eid != effect_id {
                            fail!("result for foreign effect '{eid}'");
                        }
                        let outcome = match status {
                            ResultStatus::Succeeded => match result {
                                Some(result) => AttemptOutcome::Succeeded { result },
                                None => AttemptOutcome::Failed {
                                    error: "succeeded result carried no payload".into(),
                                },
                            },
                            ResultStatus::Failed => AttemptOutcome::Failed {
                                error: error.unwrap_or_else(|| "driver reported failure".into()),
                            },
                        };
                        return self.finish(outcome, session_ref, checkpoints, accepted);
                    }
                    other => fail!("unexpected driver message {:?}", other),
                },
                Some(Err(e)) => fail!("{e}"),
                None => {
                    let outcome = self.eof_outcome(accepted);
                    return self.finish(outcome, session_ref, checkpoints, accepted);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
