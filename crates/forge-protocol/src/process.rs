//! Subprocess transport for `forge-driver/v1`: spawn the driver command,
//! handshake, send `start`, and drive the attempt to a terminal outcome.
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
                        let _ = child.kill();
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
        mut self,
        engine_version: &str,
        effect_id: &str,
        attempt_id: &str,
        seat: &str,
        input: Value,
        mut on_checkpoint: impl FnMut(&Value),
    ) -> AttemptReport {
        macro_rules! fail {
            ($($arg:tt)*) => {
                return self.finish(AttemptOutcome::Failed { error: format!($($arg)*) }, None, Vec::new())
            };
        }

        if let Err(e) = self.send(Body::Hello {
            engine_version: engine_version.to_string(),
        }) {
            fail!("could not greet driver: {e}");
        }
        match self.recv() {
            Some(Ok(Message {
                body: Body::Capabilities { .. },
                ..
            })) => {}
            Some(Ok(other)) => fail!("expected capabilities, got {:?}", other.body),
            Some(Err(e)) => fail!("{e}"),
            None => {
                let outcome = self.eof_outcome(false);
                return self.finish(outcome, None, Vec::new());
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

        let mut session_ref: Option<String> = None;
        let mut accepted = false;
        let mut checkpoints: Vec<Value> = Vec::new();
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
                        return self.finish(outcome, session_ref, checkpoints);
                    }
                    other => fail!("unexpected driver message {:?}", other),
                },
                Some(Err(e)) => fail!("{e}"),
                None => {
                    let outcome = self.eof_outcome(accepted);
                    return self.finish(outcome, session_ref, checkpoints);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
