//! The durable engine loop. Every external effect is requested durably
//! before execution and completed, failed, cancelled, or marked
//! indeterminate by a later event (decision 0003). The loop derives its
//! next action purely from `fold(journal)` + the pinned bundle; nothing
//! in here decides a transition — only the policy does.

use std::path::PathBuf;
use std::process::Command;

use forge_core::envelope::EventType;
use forge_core::fold::{computed_inputs, fold, Cursor, RunState, Status};
use forge_core::policy::Outcome;
use forge_core::EventEnvelope;
use forge_protocol::process::DriverProcess;
use forge_protocol::AttemptOutcome;
use forge_store::Store;
use serde_json::{json, Map, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::bundle::{Bundle, ENGINE_VERSION};

/// Inputs the engine owns. A seat-supplied value for any of these is
/// dropped before evaluation: journal-computed truth is never accepted
/// from a caller (README law 2).
const ENGINE_OWNED_INPUTS: [&str; 4] = [
    "consecutive_failures",
    "drift_detected",
    "dirty_worktrees",
    "reviewed_heads",
];

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] forge_store::StoreError),
    #[error("fold: {0}")]
    Fold(#[from] forge_core::FoldError),
    #[error("run '{run_id}' pins a different bundle: {detail}")]
    ManifestMismatch { run_id: String, detail: String },
    #[error("engine: {0}")]
    Other(String),
}

pub struct Engine {
    pub store: Store,
    pub bundle: Bundle,
    pub run_id: String,
    pub feature: String,
    pub repo: Option<PathBuf>,
}

#[derive(Debug)]
pub struct DriveEnd {
    pub state: RunState,
}

impl Engine {
    pub fn start(
        mut store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
    ) -> Result<Engine, EngineError> {
        let slug: String = feature
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let slug = slug.chars().take(32).collect::<String>();
        let run_id = format!("{slug}-{}", &Uuid::new_v4().to_string()[..8]);
        store.create_run(&run_id, feature, &bundle.name, &bundle.manifest)?;
        store.append_next(
            &run_id,
            EventType::RunStarted,
            json!({"feature": feature, "manifest": bundle.manifest}),
            None,
            None,
        )?;
        Ok(Engine {
            store,
            bundle,
            run_id,
            feature: feature.to_string(),
            repo,
        })
    }

    /// Resume uses the exact pinned bundle or refuses with a diagnostic.
    pub fn resume(
        store: Store,
        bundle: Bundle,
        run_id: &str,
        repo: Option<PathBuf>,
    ) -> Result<Engine, EngineError> {
        let pinned = store.manifest(run_id)?;
        if pinned != bundle.manifest {
            let detail = manifest_diff(&pinned, &bundle.manifest);
            return Err(EngineError::ManifestMismatch {
                run_id: run_id.to_string(),
                detail,
            });
        }
        let events = store.load(run_id)?;
        let feature = fold(&events)?
            .feature
            .unwrap_or_else(|| "unknown".to_string());
        Ok(Engine {
            store,
            bundle,
            run_id: run_id.to_string(),
            feature,
            repo,
        })
    }

    /// Drive the run until it parks, completes, or stops.
    pub fn drive(&mut self) -> Result<DriveEnd, EngineError> {
        loop {
            let events = self.store.load(&self.run_id)?;
            let state = fold(&events)?;
            match (&state.status, &state.cursor) {
                (Status::Completed | Status::Stopped, _) | (Status::AwaitingOperator, _) => {
                    return Ok(DriveEnd { state })
                }
                (Status::Running, cursor) => match cursor.clone() {
                    Cursor::Start => {
                        let initial = self.bundle.machine.initial.clone();
                        self.append(EventType::PhaseEntered, json!({"phase": initial}), None)?;
                    }
                    Cursor::EnterPhase { phase } => {
                        self.append(EventType::PhaseEntered, json!({"phase": phase}), None)?;
                    }
                    Cursor::RequestEffect => self.request_or_finish(&state)?,
                    Cursor::ExecuteEffect { effect_id, seat } => {
                        self.execute(&events, &state, &effect_id, &seat)?
                    }
                    Cursor::EffectInFlight {
                        effect_id,
                        attempt_id,
                        ..
                    } => {
                        // Fresh process, no live driver: completion cannot be
                        // established. Park rather than guess or re-pay.
                        self.append(
                            EventType::EffectIndeterminate,
                            json!({
                                "effect_id": effect_id,
                                "attempt_id": attempt_id,
                                "reason": "engine restarted while the attempt was in \
                                           flight; completion cannot be established",
                            }),
                            Some(attempt_id),
                        )?;
                    }
                    Cursor::Decide { effect_id, result } => {
                        self.decide(&state, &effect_id, result)?
                    }
                    Cursor::Park { reason } => {
                        self.append(
                            EventType::RunParked,
                            json!({"reason": reason, "evidence": {}}),
                            None,
                        )?;
                    }
                    Cursor::Stop => {
                        self.append(
                            EventType::RunStopped,
                            json!({"reason": "operator stop accepted"}),
                            None,
                        )?;
                    }
                    Cursor::Idle => return Ok(DriveEnd { state }),
                },
            }
        }
    }

    fn append(
        &mut self,
        event_type: EventType,
        payload: Value,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, EngineError> {
        Ok(self
            .store
            .append_next(&self.run_id, event_type, payload, None, attempt_id)?)
    }

    fn request_or_finish(&mut self, state: &RunState) -> Result<(), EngineError> {
        let phase = state
            .phase
            .clone()
            .ok_or_else(|| EngineError::Other("RequestEffect with no phase".into()))?;
        if self.bundle.machine.terminal.contains(&phase) {
            if phase == "stop" {
                let reason = state
                    .last_decision
                    .as_ref()
                    .and_then(|d| d.get("rule_id"))
                    .and_then(Value::as_str)
                    .map(|r| format!("hard stop ruled by {r}"))
                    .unwrap_or_else(|| "stopped".to_string());
                self.append(EventType::RunStopped, json!({"reason": reason}), None)?;
            } else {
                self.append(EventType::RunCompleted, json!({}), None)?;
            }
            return Ok(());
        }
        let effect_id = Uuid::new_v4().to_string();
        let input = self.seat_input(state, &phase, &effect_id)?;
        let digest = forge_core::canonical::sha256_hex(&input);
        let seq = state.seq + 1;
        self.append(
            EventType::EffectRequested,
            json!({
                "effect_id": effect_id,
                "phase": phase,
                "seat": phase,
                "idempotency_key": format!("{}:{seq}", self.run_id),
                "input_digest": digest,
            }),
            None,
        )?;
        Ok(())
    }

    /// Seat input is a pure function of (journal, pinned bundle, feature):
    /// recovery rebuilds it and the digest recorded at request time must
    /// match, or the run parks instead of running something else.
    fn seat_input(
        &self,
        state: &RunState,
        phase: &str,
        effect_id: &str,
    ) -> Result<Value, EngineError> {
        let seat = self.bundle.seats.get(phase).ok_or_else(|| {
            EngineError::Other(format!("no seat for phase '{phase}' (compile enforces this)"))
        })?;
        let workdir = self.workdir();
        Ok(json!({
            "feature": self.feature,
            "phase": phase,
            "seat": phase,
            "role_path": seat.role_path.to_string_lossy(),
            "workdir": workdir.to_string_lossy(),
            "result_path": workdir
                .join(".forge/results")
                .join(format!("{effect_id}.json"))
                .to_string_lossy(),
            "allowed_results": seat.results,
            "context": {
                "run_id": self.run_id,
                "last_decision": state.last_decision,
            },
        }))
    }

    fn workdir(&self) -> PathBuf {
        self.repo
            .clone()
            .unwrap_or_else(|| std::env::current_dir().expect("cwd"))
    }

    fn execute(
        &mut self,
        events: &[EventEnvelope],
        state: &RunState,
        effect_id: &str,
        seat_name: &str,
    ) -> Result<(), EngineError> {
        let phase = state
            .phase
            .clone()
            .ok_or_else(|| EngineError::Other("effect without a phase".into()))?;
        let requested_digest = events
            .iter()
            .rev()
            .find(|e| {
                e.event_type == EventType::EffectRequested
                    && e.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
            })
            .and_then(|e| e.payload.get("input_digest").and_then(Value::as_str))
            .map(str::to_string)
            .ok_or_else(|| EngineError::Other(format!("no requested event for {effect_id}")))?;
        let input = self.seat_input(state, &phase, effect_id)?;
        if forge_core::canonical::sha256_hex(&input) != requested_digest {
            // The world changed between request and execution (bundle edit,
            // repo move). Never run something other than what was requested.
            let attempt_id = Uuid::new_v4().to_string();
            self.append(
                EventType::EffectStarted,
                json!({"effect_id": effect_id, "attempt_id": attempt_id, "driver": "none"}),
                Some(attempt_id.clone()),
            )?;
            self.append(
                EventType::EffectFailed,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": "rebuilt seat input does not match the digest recorded \
                              at request time; refusing to execute a different effect",
                }),
                Some(attempt_id),
            )?;
            return Ok(());
        }

        let seat = self.bundle.seats[seat_name].clone();
        let attempt_id = Uuid::new_v4().to_string();
        // started is durable BEFORE the driver spawns: a crash in between
        // recovers as indeterminate, never as a silent double-execution.
        self.append(
            EventType::EffectStarted,
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "driver": seat.command[0],
            }),
            Some(attempt_id.clone()),
        )?;

        let workdir = self.workdir();
        std::fs::create_dir_all(workdir.join(".forge/results")).ok();
        let spawned = DriverProcess::spawn(&seat.command, &workdir);
        let report = match spawned {
            Err(e) => {
                self.append(
                    EventType::EffectFailed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": format!("driver did not spawn: {e}"),
                    }),
                    Some(attempt_id),
                )?;
                return Ok(());
            }
            Ok(process) => {
                let mut checkpoint_error: Option<EngineError> = None;
                let report = process.run_attempt(
                    ENGINE_VERSION,
                    effect_id,
                    &attempt_id,
                    seat_name,
                    input,
                    |data| {
                        if checkpoint_error.is_none() {
                            if let Err(e) = self.store.append_next(
                                &self.run_id,
                                EventType::EffectCheckpointed,
                                json!({
                                    "effect_id": effect_id,
                                    "attempt_id": attempt_id,
                                    "checkpoint": data,
                                }),
                                None,
                                Some(attempt_id.clone()),
                            ) {
                                checkpoint_error = Some(e.into());
                            }
                        }
                    },
                );
                if let Some(e) = checkpoint_error {
                    return Err(e);
                }
                report
            }
        };

        let stderr_tail: String = report.stderr.chars().rev().take(2000).collect::<String>()
            .chars().rev().collect();
        match report.outcome {
            AttemptOutcome::Succeeded { result } => {
                self.append(
                    EventType::EffectSucceeded,
                    json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
                    Some(attempt_id),
                )?;
            }
            AttemptOutcome::Failed { error } => {
                self.append(
                    EventType::EffectFailed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": format!("{error}; stderr tail: {stderr_tail}"),
                    }),
                    Some(attempt_id),
                )?;
            }
            AttemptOutcome::Indeterminate { reason } => {
                self.append(
                    EventType::EffectIndeterminate,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "reason": format!("{reason}; stderr tail: {stderr_tail}"),
                    }),
                    Some(attempt_id),
                )?;
            }
        }
        Ok(())
    }

    fn decide(
        &mut self,
        state: &RunState,
        _effect_id: &str,
        raw_result: Value,
    ) -> Result<(), EngineError> {
        let phase = state
            .phase
            .clone()
            .ok_or_else(|| EngineError::Other("decide without a phase".into()))?;
        let seat = &self.bundle.seats[&phase];

        // Result schema (decision 0001): an object with a declared result
        // string. Violations park with the raw evidence attached — never
        // repaired, coerced, or handed to a model to fix.
        let schema_problem = match raw_result.as_object() {
            None => Some("seat result is not an object".to_string()),
            Some(object) => match object.get("result").and_then(Value::as_str) {
                None => Some("seat result has no 'result' string".to_string()),
                Some(r) if !seat.results.iter().any(|allowed| allowed == r) => Some(format!(
                    "seat result '{r}' is not among declared results {:?}",
                    seat.results
                )),
                Some(_) => None,
            },
        };
        if let Some(problem) = schema_problem {
            self.append(
                EventType::TransitionDecided,
                json!({
                    "from": phase,
                    "result": "__schema-invalid__",
                    "rule_id": null,
                    "next": null,
                    "severity": null,
                    "inputs": {"raw_result": raw_result},
                    "problem": format!("result failed schema validation: {problem}"),
                }),
                None,
            )?;
            return Ok(());
        }

        let object = raw_result.as_object().expect("checked above");
        let result = object["result"].as_str().expect("checked above").to_string();

        // Seat-supplied facts, minus everything the engine owns.
        let mut inputs: Map<String, Value> = object
            .get("inputs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for owned in ENGINE_OWNED_INPUTS {
            inputs.remove(owned);
        }
        // Journal-computed inputs overlay (never accepted from the seat).
        for (key, value) in computed_inputs(state, &phase, &result) {
            inputs.insert(key, value);
        }
        if let Some(repo) = &self.repo {
            if phase == self.bundle.protected_phase {
                if let Some(head) = git_head(repo) {
                    inputs.insert("reviewed_heads".into(), json!({ "repo": head }));
                }
            }
            if phase == "ship" {
                inputs.insert("dirty_worktrees".into(), Value::Bool(git_dirty(repo)));
                if let Some(reviewed) = state
                    .reviewed_heads
                    .as_ref()
                    .and_then(|h| h.get("repo"))
                    .and_then(Value::as_str)
                {
                    let drifted = git_head(repo).as_deref() != Some(reviewed);
                    inputs.insert("drift_detected".into(), Value::Bool(drifted));
                }
            }
        }

        let payload = match self.bundle.machine.evaluate(&phase, &result, &inputs) {
            Outcome::Ruling {
                rule_id,
                next_phase,
                severity,
                ..
            } => json!({
                "from": phase,
                "result": result,
                "rule_id": rule_id,
                "next": next_phase,
                "severity": severity,
                "inputs": inputs,
                "problem": null,
            }),
            Outcome::NoRule { problem } => json!({
                "from": phase,
                "result": result,
                "rule_id": null,
                "next": null,
                "severity": null,
                "inputs": inputs,
                "problem": problem.unwrap_or_else(|| "no rule matched".to_string()),
            }),
        };
        self.append(EventType::TransitionDecided, payload, None)?;
        Ok(())
    }
}

/// Append an operator command and its acceptance (the CLI is the
/// operator's console; approval is a signed journal entry, not prose).
pub fn operator_command(
    store: &mut Store,
    run_id: &str,
    command: &str,
    operator: &str,
    reason: &str,
) -> Result<(), EngineError> {
    let command_id = Uuid::new_v4().to_string();
    store.append_next(
        run_id,
        EventType::OperatorCommanded,
        json!({"command_id": command_id, "command": command, "args": {}, "operator": operator}),
        None,
        None,
    )?;
    store.append_next(
        run_id,
        EventType::OperatorAccepted,
        json!({"command_id": command_id, "operator": operator, "reason": reason}),
        None,
        None,
    )?;
    Ok(())
}

fn manifest_diff(pinned: &Value, current: &Value) -> String {
    let empty = Map::new();
    let pinned_files = pinned.get("files").and_then(Value::as_object).unwrap_or(&empty);
    let current_files = current.get("files").and_then(Value::as_object).unwrap_or(&empty);
    let mut diffs = Vec::new();
    for (path, digest) in pinned_files {
        match current_files.get(path) {
            None => diffs.push(format!("missing: {path}")),
            Some(d) if d != digest => diffs.push(format!("changed: {path}")),
            _ => {}
        }
    }
    for path in current_files.keys() {
        if !pinned_files.contains_key(path) {
            diffs.push(format!("added: {path}"));
        }
    }
    if diffs.is_empty() {
        "non-file manifest fields differ (engine or contract version)".to_string()
    } else {
        diffs.join(", ")
    }
}

fn git_head(repo: &std::path::Path) -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn git_dirty(repo: &std::path::Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()
        .map(|out| !out.stdout.is_empty())
        .unwrap_or(true) // unreadable repo counts as dirty: fail closed
}
