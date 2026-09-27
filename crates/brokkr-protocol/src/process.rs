//! Subprocess transport for `forge-driver/v1`: spawn the driver command,
//! handshake, offer the seat's prior session to a driver that declared
//! it can rejoin one, send `start`, and drive the attempt to a terminal
//! outcome.
//! Every protocol violation degrades to `Failed` (driver defect, retry
//! is a new attempt); a silent exit degrades to `Indeterminate`; a
//! deadline expiry kills the driver and degrades to `Failed` — the kill
//! makes non-completion determinate, so bounded retry stays safe
//! (decision 0006).
//!
//! The attempt is the driver's whole process tree (#403, `tree`): no
//! report is returned until the tree is proven gone, so a retry the
//! engine starts on the report cannot overlap it. An end that cannot be
//! proven degrades to `Indeterminate`, which parks.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rustix::process::Pid;
use serde_json::Value;
use thiserror::Error;

use crate::{AttemptOutcome, AttemptReport, Body, Message, ResultStatus, PROTO};

mod tree;

use tree::{Bounds, Unsettled};

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

/// The environment a driver starts in (decision 0046 ruling 4; design
/// DD18). `Inherit` is the engine's own environment — every model site
/// under every boundary, because its harness needs the operator's keys,
/// and every driver before this enum existed. `Exactly` is a composed
/// table and nothing else: an unboxed exec dispatch under `harness` or
/// `open` starts from an empty environment holding the box's own
/// allow-list, so the credentials the environment carried never reach
/// the script. A named two-variant enum rather than an `Option`, because
/// a `None` at a spawn site would read as *no environment*, the opposite
/// of inheriting the engine's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnEnv {
    Inherit,
    Exactly(BTreeMap<String, String>),
}

/// One read of the driver's stdout, as its reader thread hands it over.
enum Stdout {
    Line(String),
    Failed(std::io::Error),
    Eof,
}

pub struct DriverProcess {
    /// The leader of the attempt's process group; reaped only by `finish`.
    child: Child,
    stdin: Box<dyn Write>,
    stdout: mpsc::Receiver<Stdout>,
    stderr: mpsc::Receiver<String>,
    timed_out: Arc<AtomicBool>,
    /// Dropping the sender disarms the watchdog; `finish` joins the
    /// thread before it reaps, so no kill can land after the reap.
    watchdog: Option<(mpsc::Sender<()>, JoinHandle<()>)>,
    deadline: Option<Duration>,
    started: Instant,
    bounds: Bounds,
    /// Does the group still have a member? `tree::group_alive`.
    alive: fn(Pid) -> bool,
}

/// The driver's stdout, read on a thread of its own so that waiting on it
/// can be bounded: a process that left the group while holding the pipe
/// would otherwise hold the attempt open. `Eof` is sent only at EOF.
fn read_stdout(stdout: ChildStdout) -> mpsc::Receiver<Stdout> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => drop(tx.send(Stdout::Line(line))),
                Err(error) => {
                    drop(tx.send(Stdout::Failed(error)));
                    let _ = std::io::copy(&mut reader, &mut std::io::sink());
                    break;
                }
            }
        }
        drop(tx.send(Stdout::Eof));
    });
    rx
}

/// The driver's stderr, whole, once its pipe reaches EOF.
fn read_stderr(mut stderr: ChildStderr) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        drop(tx.send(String::from_utf8(bytes).unwrap_or_default()));
    });
    rx
}

/// An attempt whose end could not be proven parks (decision 0003): the
/// outcome it reached is kept in the reason, and nothing certifies it as
/// settled or lets a retry start beside what may still be running.
fn settled(outcome: AttemptOutcome, ended: Result<(), Unsettled>) -> AttemptOutcome {
    let Err(unsettled) = ended else {
        return outcome;
    };
    let reached = match outcome {
        AttemptOutcome::Succeeded { .. } => "the driver reported success".to_string(),
        AttemptOutcome::Failed { error } => error,
        AttemptOutcome::Indeterminate { reason } => reason,
    };
    AttemptOutcome::Indeterminate {
        reason: format!("{reached}; the attempt is not proven over: {unsettled}"),
    }
}

impl DriverProcess {
    /// Spawn the driver as the leader of its own process group. With a
    /// deadline, a watchdog kills the group when it expires; the attempt
    /// then reports Failed(timeout) rather than hanging the run forever.
    pub fn spawn(
        command: &[String],
        workdir: &std::path::Path,
        deadline: Option<Duration>,
        env: &SpawnEnv,
    ) -> Result<Self, SpawnError> {
        let (program, args) = command.split_first().ok_or(SpawnError::EmptyCommand)?;
        let mut builder = Command::new(program);
        builder
            .args(args)
            .current_dir(workdir)
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let SpawnEnv::Exactly(table) = env {
            builder.env_clear().envs(table);
        }
        let mut child = builder.spawn().map_err(|source| SpawnError::Spawn {
            command: command.join(" "),
            source,
        })?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = read_stdout(child.stdout.take().expect("piped stdout"));
        let stderr = read_stderr(child.stderr.take().expect("piped stderr"));
        let timed_out = Arc::new(AtomicBool::new(false));
        let group = Pid::from_child(&child);
        let watchdog = deadline.map(|deadline| {
            let (tx, rx) = mpsc::channel::<()>();
            let timed_out = Arc::clone(&timed_out);
            let thread = std::thread::spawn(move || {
                if let Err(RecvTimeoutError::Timeout) = rx.recv_timeout(deadline) {
                    timed_out.store(true, Ordering::SeqCst);
                    tree::kill_group(group);
                }
            });
            (tx, thread)
        });
        Ok(DriverProcess {
            child,
            stdin: Box::new(stdin),
            stdout,
            stderr,
            timed_out,
            watchdog,
            deadline,
            started: Instant::now(),
            bounds: Bounds::DEFAULT,
            alive: tree::group_alive,
        })
    }

    fn send(&mut self, body: Body) -> std::io::Result<()> {
        let line = serde_json::to_string(&Message::new(body))?;
        self.stdin.write_all(line.as_bytes())?;
        self.stdin.write_all(b"\n")?;
        self.stdin.flush()
    }

    /// The next read of stdout. With a deadline it is waited on until the
    /// deadline plus the drain bound and no longer: by then the watchdog
    /// has killed the group, and only a process that left it can still
    /// hold the pipe. Giving up reads as EOF.
    fn next_stdout(&self) -> Option<Stdout> {
        match self.deadline {
            None => self.stdout.recv().ok(),
            Some(deadline) => {
                let give_up = self.started + deadline + self.bounds.drain;
                let wait = give_up.saturating_duration_since(Instant::now());
                self.stdout.recv_timeout(wait).ok()
            }
        }
    }

    fn recv(&self) -> Option<Result<Message, String>> {
        loop {
            let line = match self.next_stdout()? {
                Stdout::Line(line) => line,
                Stdout::Failed(e) => return Some(Err(format!("driver stdout read failed: {e}"))),
                Stdout::Eof => return None,
            };
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
    }

    /// Read stdout on to EOF by `until`. What the driver says after its
    /// terminal message is not protocol; only the pipe's closing counts.
    fn stdout_closed(&self, until: Instant) -> Result<(), Unsettled> {
        loop {
            match self
                .stdout
                .recv_timeout(until.saturating_duration_since(Instant::now()))
            {
                Ok(Stdout::Eof) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
                Ok(Stdout::Line(_) | Stdout::Failed(_)) => {}
                Err(RecvTimeoutError::Timeout) => return Err(Unsettled::Stdout),
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
        // Disarm the watchdog and wait it out before shutdown, so a slow
        // exit is not killed by it and no kill lands after the reap.
        if let Some((disarm, watchdog)) = self.watchdog.take() {
            drop(disarm);
            let _ = watchdog.join();
        }
        let _ = self.send(Body::Shutdown);
        self.stdin = Box::new(std::io::sink());
        let ended = tree::end(&mut self.child, &self.bounds, self.alive);
        let drain = Instant::now() + self.bounds.drain;
        let ended = ended.and(self.stdout_closed(drain));
        let (stderr, ended) = match self
            .stderr
            .recv_timeout(drain.saturating_duration_since(Instant::now()))
        {
            Ok(stderr) => (stderr, ended),
            Err(_) => (String::new(), ended.and(Err(Unsettled::Stderr))),
        };
        AttemptReport {
            outcome: settled(outcome, ended),
            session_ref,
            checkpoints,
            stderr,
            accepted,
            // Read after the watchdog is joined and the tree ended, so
            // the bit is the one that decided this attempt's end.
            deadline_killed: self.timed_out.load(Ordering::SeqCst),
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
    #[expect(clippy::too_many_arguments, reason = "baseline 2026-09, #288")]
    #[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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
