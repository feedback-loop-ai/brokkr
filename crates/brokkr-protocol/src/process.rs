//! Subprocess transport for `forge-driver/v1`: spawn the driver command,
//! handshake, offer the seat's prior session to a driver that declared
//! it can rejoin one, send `start`, and drive the attempt to a terminal
//! outcome.
//! Every protocol violation degrades to `Failed` (driver defect, retry
//! is a new attempt); a silent exit degrades to `Indeterminate`, and so
//! does a driver past the transport's limits (`limits`, #433); a
//! deadline expiry kills the driver and degrades to `Failed` — the kill
//! makes non-completion determinate, so bounded retry stays safe
//! (decision 0006).
//!
//! The attempt is the driver's whole process tree (#403, `tree`): no
//! report is returned until the tree is proven gone, so a retry the
//! engine starts on the report cannot overlap it. An end that cannot be
//! proven is carried as an unresolved `Cleanup` beside the outcome the
//! driver reached, and parks.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;
use thiserror::Error;

use crate::{AttemptOutcome, AttemptReport, Body, Cleanup, Message, ResultStatus, PROTO};

mod attempts;
mod limits;
mod own_engine;
mod table;
mod tree;

use attempts::{Attempt, Unspawned};
use limits::{Exceeded, Frame, Limits, Retained};
// A test seam, hidden from documentation and outside the supported
// surface: the probe's tests and the runtime engine's play a test whose
// launches must see no other test's orphan through this one helper.
#[doc(hidden)]
pub use own_engine::in_its_own_engine;
pub use tree::Unsettled;
use tree::{Bounds, Host};

/// How many unread stdout lines the engine holds before the driver's
/// writes meet the pipe's backpressure again.
const STDOUT_LINES: usize = 64;

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("driver command is empty")]
    EmptyCommand,
    #[error("failed to spawn driver {command}: {source}")]
    Spawn {
        command: String,
        source: std::io::Error,
    },
    /// #403: the process table could not be read before the spawn, so the
    /// attempt's tree could not be told from what ran before it.
    #[error("refused to spawn driver {command}: {source}")]
    Unwatched { command: String, source: Unsettled },
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
    /// A line past the frame limit (`limits`).
    Exceeded(Exceeded),
    Eof,
}

/// How the attempt loop ended: a protocol violation, which fails the
/// attempt; the driver gone; the outcome its result reached; or a limit
/// it exceeded, which leaves the attempt indeterminate (#433).
enum Ended {
    Failed(String),
    Lost,
    Reached(AttemptOutcome),
    Exceeded(Exceeded),
}

/// What the attempt loop has kept of the driver's account.
#[derive(Default)]
struct Account {
    accepted: bool,
    session_ref: Option<String>,
    retained: Retained,
}

pub struct DriverProcess {
    /// The leader of the attempt's process group; reaped only by `finish`.
    child: Child,
    /// The attempt among the engine's live ones (`attempts`).
    attempt: Attempt,
    stdin: Box<dyn Write + Send>,
    stdout: mpsc::Receiver<Stdout>,
    stderr: mpsc::Receiver<String>,
    timed_out: Arc<AtomicBool>,
    /// Dropping the sender disarms the watchdog; `finish` joins the
    /// thread before it reaps, so no kill can land after the reap.
    watchdog: Option<(mpsc::Sender<()>, JoinHandle<()>)>,
    deadline: Option<Duration>,
    started: Instant,
    bounds: Bounds,
    limits: Limits,
    host: Host,
}

/// The driver's stdout, read on a thread of its own so that waiting on it
/// can be bounded: a process that left the group while holding the pipe
/// would otherwise hold the attempt open. `Eof` is sent only at EOF.
fn read_stdout(stdout: impl Read + Send + 'static, frame_bytes: usize) -> mpsc::Receiver<Stdout> {
    let (tx, rx) = mpsc::sync_channel(STDOUT_LINES);
    std::thread::spawn(move || forward(BufReader::new(stdout), frame_bytes, &tx));
    rx
}

/// Hand each line over, waiting while `STDOUT_LINES` are unread, so a
/// flooding driver meets backpressure, and each no longer than
/// `frame_bytes`. Once the engine has stopped listening, nothing more is
/// read: the pipe closes on the writer. A read that fails, a line that is
/// not UTF-8 and a line past the limit are handed over as such, and the
/// rest of the pipe is drained unread to its EOF.
fn forward(mut reader: impl BufRead, frame_bytes: usize, tx: &SyncSender<Stdout>) {
    let refusal = loop {
        match limits::read_frame(&mut reader, frame_bytes) {
            Ok(Frame::Line(line)) => match String::from_utf8(line) {
                Ok(line) => {
                    if tx.send(Stdout::Line(line)).is_err() {
                        return;
                    }
                }
                Err(_) => {
                    break Stdout::Failed(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "stream did not contain valid UTF-8",
                    ))
                }
            },
            Ok(Frame::Eof) => break Stdout::Eof,
            Ok(Frame::Over { head }) => {
                break Stdout::Exceeded(Exceeded::Frame {
                    limit: frame_bytes,
                    head,
                })
            }
            Err(error) => break Stdout::Failed(error),
        }
    };
    if !matches!(refusal, Stdout::Eof) {
        drop(tx.send(refusal));
        let _ = std::io::copy(&mut reader, &mut std::io::sink());
    }
    drop(tx.send(Stdout::Eof));
}

fn write_message(stdin: &mut dyn Write, body: Body) -> std::io::Result<()> {
    let line = serde_json::to_string(&Message::new(body))?;
    stdin.write_all(line.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()
}

/// The driver's stderr once its pipe reaches EOF, no more than
/// `stderr_bytes` of it kept (`limits::retained_stderr`).
fn read_stderr(stderr: impl Read + Send + 'static, stderr_bytes: usize) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        drop(tx.send(limits::retained_stderr(stderr, stderr_bytes)));
    });
    rx
}

impl DriverProcess {
    /// Spawn the driver as the leader of a session, and so a process
    /// group, of its own (`attempts`). With a
    /// deadline, a watchdog kills the group when it expires; the attempt
    /// then reports Failed(timeout) rather than hanging the run forever.
    pub fn spawn(
        command: &[String],
        workdir: &std::path::Path,
        deadline: Option<Duration>,
        env: &SpawnEnv,
    ) -> Result<Self, SpawnError> {
        Self::spawn_with(command, workdir, deadline, env, Host::REAL)
    }

    /// `spawn` on `host`, which the watchdog's kill and the attempt's end
    /// call, and which the tracker and the stop handler call when this
    /// is the engine's first attempt.
    fn spawn_with(
        command: &[String],
        workdir: &std::path::Path,
        deadline: Option<Duration>,
        env: &SpawnEnv,
        host: Host,
    ) -> Result<Self, SpawnError> {
        Self::spawn_limited(command, workdir, deadline, env, host, Limits::DEFAULT)
    }

    /// `spawn_with`, holding what the driver sends to `limits` (#433).
    fn spawn_limited(
        command: &[String],
        workdir: &std::path::Path,
        deadline: Option<Duration>,
        env: &SpawnEnv,
        host: Host,
        limits: Limits,
    ) -> Result<Self, SpawnError> {
        let (program, args) = command.split_first().ok_or(SpawnError::EmptyCommand)?;
        let mut builder = Command::new(program);
        builder
            .args(args)
            .current_dir(workdir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let SpawnEnv::Exactly(table) = env {
            builder.env_clear().envs(table);
        }
        let (mut child, attempt) = Attempt::spawn(&mut builder, host).map_err(|unspawned| {
            let command = command.join(" ");
            match unspawned {
                Unspawned::Table(source) => SpawnError::Unwatched { command, source },
                Unspawned::Spawn(source) => SpawnError::Spawn { command, source },
            }
        })?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = read_stdout(
            child.stdout.take().expect("piped stdout"),
            limits.frame_bytes,
        );
        let stderr = read_stderr(
            child.stderr.take().expect("piped stderr"),
            limits.stderr_bytes,
        );
        let timed_out = Arc::new(AtomicBool::new(false));
        let key = attempt.key();
        let watchdog = deadline.map(|deadline| {
            let (tx, rx) = mpsc::channel::<()>();
            let timed_out = Arc::clone(&timed_out);
            let thread = std::thread::spawn(move || {
                if let Err(RecvTimeoutError::Timeout) = rx.recv_timeout(deadline) {
                    timed_out.store(true, Ordering::SeqCst);
                    // A refusal here is met again, and carried, by
                    // `tree::end`, whose kill reads the table and signals
                    // the same tree once more.
                    drop(Attempt::kill(key, host));
                }
            });
            (tx, thread)
        });
        Ok(DriverProcess {
            child,
            attempt,
            stdin: Box::new(stdin),
            stdout,
            stderr,
            timed_out,
            watchdog,
            deadline,
            started: Instant::now(),
            bounds: Bounds::DEFAULT,
            limits,
            host,
        })
    }

    fn send(&mut self, body: Body) -> std::io::Result<()> {
        write_message(&mut self.stdin, body)
    }

    /// Send `shutdown`, waiting for the write no later than `until`: a
    /// driver that stopped reading its stdin must not hold the attempt
    /// open on a full pipe. A write still blocked is left to its thread,
    /// which the kill releases, and closing stdin follows the write.
    fn shutdown(&mut self, until: Instant) {
        let mut stdin = std::mem::replace(&mut self.stdin, Box::new(std::io::sink()));
        let (written, done) = mpsc::channel();
        std::thread::spawn(move || {
            drop(write_message(&mut stdin, Body::Shutdown));
            let _ = written.send(());
        });
        let _ = done.recv_timeout(until.saturating_duration_since(Instant::now()));
    }

    /// The next read of stdout. With a deadline it is waited on until the
    /// deadline plus the drain bound and no longer: by then the watchdog
    /// has killed the tree, and only a process outside it can still hold
    /// the pipe. Giving up reads as EOF, whatever is still unread.
    fn next_stdout(&self) -> Option<Stdout> {
        match self.deadline {
            None => self.stdout.recv().ok(),
            Some(deadline) => {
                let give_up = self.started + deadline + self.bounds.drain;
                let wait = give_up.checked_duration_since(Instant::now())?;
                self.stdout.recv_timeout(wait).ok()
            }
        }
    }

    /// The next message and the bytes of its line, or how reading ended.
    fn recv(&self) -> Result<(Message, usize), Ended> {
        loop {
            let line = match self.next_stdout() {
                Some(Stdout::Line(line)) => line,
                Some(Stdout::Failed(e)) => {
                    return Err(Ended::Failed(format!("driver stdout read failed: {e}")))
                }
                Some(Stdout::Exceeded(exceeded)) => return Err(Ended::Exceeded(exceeded)),
                Some(Stdout::Eof) | None => return Err(Ended::Lost),
            };
            if line.trim().is_empty() {
                continue;
            }
            let message = serde_json::from_str::<Message>(&line)
                .map_err(|e| Ended::Failed(format!("unreadable driver message: {e}: {line}")))?;
            if message.proto != PROTO {
                return Err(Ended::Failed(format!(
                    "driver spoke '{}', want '{PROTO}'",
                    message.proto
                )));
            }
            return Ok((message, line.len()));
        }
    }

    /// Read stdout on to EOF by `until`, and not past it however much is
    /// ready. What the driver says after its terminal message is not
    /// protocol; only the pipe's closing counts.
    fn stdout_closed(&self, until: Instant) -> Result<(), Unsettled> {
        loop {
            let wait = until
                .checked_duration_since(Instant::now())
                .ok_or(Unsettled::Stdout)?;
            match self.stdout.recv_timeout(wait) {
                Ok(Stdout::Eof) | Err(RecvTimeoutError::Disconnected) => return Ok(()),
                Ok(Stdout::Line(_) | Stdout::Failed(_) | Stdout::Exceeded(_)) => {}
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
        let grace = Instant::now() + self.bounds.grace;
        self.shutdown(grace);
        let ended = tree::end(
            &mut self.child,
            &self.attempt,
            grace,
            &self.bounds,
            self.host,
        );
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
            outcome,
            refused: None,
            cleanup: match ended {
                Ok(()) => Cleanup::Settled,
                Err(reason) => Cleanup::Unresolved { reason },
            },
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
        // Declared before the first refusal so every terminal path — the
        // handshake failures included — reports whether the driver ever
        // accepted. That single bit is the engine's fail-to-start
        // predicate; leaving it out of a path would let one shape of
        // failure lie about which side of the mid-session boundary it is
        // on.
        let mut account = Account::default();

        macro_rules! fail {
            ($($arg:tt)*) => {
                return self.end(Ended::Failed(format!($($arg)*)), account)
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
                return self.end(Ended::Lost, account);
            }
            fail!("could not greet driver: {e}");
        }
        let supports = match self.recv() {
            Ok((
                Message {
                    body: Body::Capabilities { supports, .. },
                    ..
                },
                _,
            )) => supports,
            Ok((other, _)) => fail!("expected capabilities, got {:?}", other.body),
            Err(ended) => return self.end(ended, account),
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
        let ended = self.converse(effect_id, &mut account, &mut on_checkpoint);
        self.end(ended, account)
    }

    /// Read the driver's account of the attempt to its end. Each
    /// checkpoint is kept within the limits before `on_checkpoint`
    /// journals it, so a checkpoint past them is neither.
    fn converse(
        &self,
        effect_id: &str,
        account: &mut Account,
        on_checkpoint: &mut impl FnMut(&Value),
    ) -> Ended {
        loop {
            let (message, bytes) = match self.recv() {
                Ok(received) => received,
                Err(ended) => return ended,
            };
            match message.body {
                Body::Accepted {
                    effect_id: eid,
                    session_ref,
                    ..
                } => {
                    if eid != effect_id {
                        return Ended::Failed(format!(
                            "driver accepted a different effect '{eid}'"
                        ));
                    }
                    account.accepted = true;
                    account.session_ref = session_ref;
                }
                Body::Checkpoint {
                    data,
                    effect_id: eid,
                    ..
                } => {
                    if eid != effect_id {
                        return Ended::Failed(format!("checkpoint for foreign effect '{eid}'"));
                    }
                    match account.retained.keep(data, bytes, &self.limits) {
                        Ok(kept) => on_checkpoint(kept),
                        Err(exceeded) => return Ended::Exceeded(exceeded),
                    }
                }
                Body::Result {
                    effect_id: eid,
                    status,
                    result,
                    error,
                    ..
                } => {
                    if eid != effect_id {
                        return Ended::Failed(format!("result for foreign effect '{eid}'"));
                    }
                    return Ended::Reached(reached(status, result, error));
                }
                other => return Ended::Failed(format!("unexpected driver message {other:?}")),
            }
        }
    }

    /// End the attempt as the loop ended. A protocol violation keeps
    /// nothing of the driver's account but whether it accepted.
    fn end(self, ended: Ended, account: Account) -> AttemptReport {
        let Account {
            accepted,
            session_ref,
            retained,
        } = account;
        let outcome = match ended {
            Ended::Failed(error) => {
                let outcome = AttemptOutcome::Failed { error };
                return self.finish(outcome, None, Vec::new(), accepted);
            }
            Ended::Lost => self.eof_outcome(accepted),
            Ended::Reached(outcome) => outcome,
            Ended::Exceeded(exceeded) => exceeded.outcome(),
        };
        self.finish(outcome, session_ref, retained.checkpoints, accepted)
    }
}

/// The outcome a driver's result reached.
fn reached(status: ResultStatus, result: Option<Value>, error: Option<String>) -> AttemptOutcome {
    match status {
        ResultStatus::Succeeded => match result {
            Some(result) => AttemptOutcome::Succeeded { result },
            None => AttemptOutcome::Failed {
                error: "succeeded result carried no payload".into(),
            },
        },
        ResultStatus::Failed => AttemptOutcome::Failed {
            error: error.unwrap_or_else(|| "driver reported failure".into()),
        },
    }
}

/// A process run to its exit or a deadline outside the driver protocol:
/// the harness probe's launch (#484). It is one of the engine's live
/// attempts, so the tracker neither reaps it nor takes it for a stray
/// (`attempts`), and its tree ends as a driver's does (`tree::end`).
pub(crate) struct Launched {
    pub(crate) child: Child,
    attempt: Attempt,
}

impl Launched {
    /// Spawn `builder`, leading a session of its own (`attempts`).
    pub(crate) fn spawn(builder: &mut Command) -> std::io::Result<Self> {
        let (child, attempt) = Attempt::spawn(builder, Host::REAL)?;
        Ok(Launched { child, attempt })
    }

    /// Has the leader exited? Observed without reaping it, so the group
    /// id stays the launch's.
    pub(crate) fn exited(&self) -> bool {
        tree::exited(rustix::process::Pid::from_child(&self.child))
    }

    /// End the tree now: the group and every descendant recorded under
    /// it killed, the leader reaped, and the table read settled. The
    /// leader's exit code, `None` for a signal.
    pub(crate) fn end(mut self) -> Result<Option<i32>, Unsettled> {
        let (now, bounds) = (Instant::now(), &Bounds::DEFAULT);
        tree::end(&mut self.child, &self.attempt, now, bounds, Host::REAL)?;
        let status = self.child.wait().ok();
        Ok(status.as_ref().and_then(std::process::ExitStatus::code))
    }
}

/// A launch that was not spawned, in its own words: the spawn's error,
/// or the read before it, which refuses the launch as it does a driver.
impl From<Unspawned> for std::io::Error {
    fn from(unspawned: Unspawned) -> Self {
        match unspawned {
            Unspawned::Table(unwatched) => std::io::Error::other(unwatched),
            Unspawned::Spawn(error) => error,
        }
    }
}

#[cfg(test)]
mod tests;
