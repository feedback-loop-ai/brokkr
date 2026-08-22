//! Subprocess transport for `forge-driver/v1`: spawn the driver command,
//! handshake, send `start`, and drive the attempt to a terminal outcome.
//! Every protocol violation degrades to `Failed` (driver defect, retry
//! is a new attempt); a silent exit degrades to `Indeterminate`.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};

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
    child: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    stderr_thread: std::thread::JoinHandle<String>,
}

impl DriverProcess {
    pub fn spawn(command: &[String], workdir: &std::path::Path) -> Result<Self, SpawnError> {
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
        Ok(DriverProcess {
            child,
            stdin,
            stdout,
            stderr_thread,
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

    fn finish(mut self, outcome: AttemptOutcome, session_ref: Option<String>, checkpoints: Vec<Value>) -> AttemptReport {
        let _ = self.send(Body::Shutdown);
        drop(self.stdin);
        let _ = self.child.wait();
        let stderr = self.stderr_thread.join().unwrap_or_default();
        AttemptReport {
            outcome,
            session_ref,
            checkpoints,
            stderr,
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
                return self.finish(
                    AttemptOutcome::Indeterminate {
                        reason: "driver exited before capabilities".into(),
                    },
                    None,
                    Vec::new(),
                )
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
                    Body::Checkpoint { data, effect_id: eid, .. } => {
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
                    let reason = if accepted {
                        "driver exited after accepting, before a result — attempt \
                         cannot be established as complete"
                    } else {
                        "driver exited before accepting the attempt"
                    };
                    return self.finish(
                        AttemptOutcome::Indeterminate {
                            reason: reason.into(),
                        },
                        session_ref,
                        checkpoints,
                    );
                }
            }
        }
    }
}
