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

use crate::bundle::{Aggregate, Bundle, PanelMember, SeatBody, ENGINE_OWNED_INPUTS, ENGINE_VERSION};
use forge_core::policy::SEVERITY_ORDER;
use forge_protocol::AttemptReport;

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
    /// The event_id every append chains to as `causation_id` — refreshed
    /// to the journal head each drive iteration, then to each event this
    /// iteration appends, so causal links mirror the engine's actual
    /// decision order (rendered by the UI timeline).
    current_cause: Option<String>,
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
            current_cause: None,
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
            current_cause: None,
        })
    }

    /// Drive the run until it parks, completes, or stops.
    pub fn drive(&mut self) -> Result<DriveEnd, EngineError> {
        loop {
            let events = self.store.load(&self.run_id)?;
            self.current_cause = events.last().map(|e| e.event_id.clone());
            let state = fold(&events)?;
            match (&state.status, &state.cursor) {
                (Status::Completed | Status::Stopped, _) | (Status::AwaitingOperator, _) => {
                    // Best-effort tamper-evidence: anchor the journal head
                    // in refs/forge/<run>. Gaps are reported, never fatal
                    // (the referee-era anchor-gap lore).
                    if let Some(repo) = &self.repo {
                        if let Err(e) = crate::anchor::anchor(&self.store, repo, &self.run_id) {
                            eprintln!("anchor gap for {}: {e}", self.run_id);
                        }
                    }
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
                    Cursor::ExecuteEffect {
                        effect_id,
                        seat,
                        failed_attempts,
                    } => {
                        let limits = self
                            .bundle
                            .seats
                            .get(&seat)
                            .map(|s| s.limits)
                            .unwrap_or_default();
                        if failed_attempts >= limits.max_attempts {
                            // Bounded retry exhausted (decision 0006):
                            // park with the last recorded error.
                            let last_error = events
                                .iter()
                                .rev()
                                .find(|e| {
                                    e.event_type == EventType::EffectFailed
                                        && e.payload.get("effect_id").and_then(Value::as_str)
                                            == Some(effect_id.as_str())
                                })
                                .and_then(|e| e.payload.get("error").and_then(Value::as_str))
                                .unwrap_or("no error recorded")
                                .to_string();
                            self.append(
                                EventType::RunParked,
                                json!({
                                    "reason": format!(
                                        "effect {effect_id} failed {failed_attempts} of {} \
                                         attempt(s); last error: {last_error}",
                                        limits.max_attempts
                                    ),
                                    "evidence": {},
                                }),
                                None,
                            )?;
                        } else {
                            self.execute(&events, &state, &effect_id, &seat)?
                        }
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
        let envelope = self.store.append_next(
            &self.run_id,
            event_type,
            payload,
            self.current_cause.clone(),
            attempt_id,
        )?;
        self.current_cause = Some(envelope.event_id.clone());
        Ok(envelope)
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
        let context = json!({
            "run_id": self.run_id,
            "last_decision": state.last_decision,
        });
        match &seat.body {
            SeatBody::Single { role_path, .. } => Ok(json!({
                "feature": self.feature,
                "phase": phase,
                "seat": phase,
                "role_path": role_path.to_string_lossy(),
                "workdir": workdir.to_string_lossy(),
                "result_path": workdir
                    .join(".forge/results")
                    .join(format!("{effect_id}.json"))
                    .to_string_lossy(),
                "allowed_results": seat.results,
                "context": context,
            })),
            SeatBody::Panel { members, aggregate } => {
                let mut member_map = Map::new();
                for member in members {
                    member_map.insert(
                        member.name.clone(),
                        json!({
                            "role_path": member.role_path.to_string_lossy(),
                            "result_path": workdir
                                .join(".forge/results")
                                .join(format!("{effect_id}-{}.json", member.name))
                                .to_string_lossy(),
                        }),
                    );
                }
                Ok(json!({
                    "feature": self.feature,
                    "phase": phase,
                    "seat": phase,
                    "aggregate": format!("{aggregate:?}"),
                    "workdir": workdir.to_string_lossy(),
                    "members": Value::Object(member_map),
                    "allowed_results": seat.results,
                    "context": context,
                }))
            }
        }
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
        let driver_label = match &seat.body {
            SeatBody::Single { command, .. } => command[0].clone(),
            SeatBody::Panel { members, aggregate } => {
                format!("panel[{}]:{aggregate:?}", members.len())
            }
        };
        // started is durable BEFORE the driver spawns: a crash in between
        // recovers as indeterminate, never as a silent double-execution.
        self.append(
            EventType::EffectStarted,
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "driver": driver_label,
            }),
            Some(attempt_id.clone()),
        )?;

        let workdir = self.workdir();
        std::fs::create_dir_all(workdir.join(".forge/results")).ok();
        let deadline = std::time::Duration::from_secs(seat.limits.timeout_seconds);

        let (members, aggregate, command) = match &seat.body {
            SeatBody::Panel { members, aggregate } => (members.clone(), Some(*aggregate), Vec::new()),
            SeatBody::Single { command, .. } => (Vec::new(), None, command.clone()),
        };
        if let Some(aggregate) = aggregate {
            return self.execute_panel(
                effect_id, &attempt_id, seat_name, &members, aggregate, &input, deadline,
            );
        }
        let spawned = DriverProcess::spawn(&command, &workdir, Some(deadline));
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
                                self.current_cause.clone(),
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

    /// Run a parallel panel INSIDE one effect (decision 0002): members
    /// execute concurrently, join as a barrier in declared order, each
    /// member's outcome is journaled as checkpoint evidence, and the
    /// declared deterministic aggregate produces the single typed result
    /// the outer machine sees. Any indeterminate member makes the whole
    /// attempt indeterminate (park); otherwise any failed member fails
    /// the attempt (retryable under 0006).
    #[allow(clippy::too_many_arguments)]
    fn execute_panel(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        seat_name: &str,
        members: &[PanelMember],
        aggregate: Aggregate,
        panel_input: &Value,
        deadline: std::time::Duration,
    ) -> Result<(), EngineError> {
        let workdir = self.workdir();
        let reports: Vec<(String, AttemptReport)> = std::thread::scope(|scope| {
            let handles: Vec<_> = members
                .iter()
                .map(|member| {
                    let member_input = json!({
                        "feature": panel_input["feature"],
                        "phase": panel_input["phase"],
                        "seat": format!("{seat_name}:{}", member.name),
                        "role_path": panel_input["members"][&member.name]["role_path"],
                        "workdir": panel_input["workdir"],
                        "result_path": panel_input["members"][&member.name]["result_path"],
                        "allowed_results": panel_input["allowed_results"],
                        "context": panel_input["context"],
                    });
                    let member_seat = format!("{seat_name}:{}", member.name);
                    let command = member.command.clone();
                    let name = member.name.clone();
                    let workdir = workdir.clone();
                    scope.spawn(move || {
                        let report = match DriverProcess::spawn(&command, &workdir, Some(deadline))
                        {
                            Err(e) => AttemptReport {
                                outcome: AttemptOutcome::Failed {
                                    error: format!("member driver did not spawn: {e}"),
                                },
                                session_ref: None,
                                checkpoints: Vec::new(),
                                stderr: String::new(),
                            },
                            Ok(process) => process.run_attempt(
                                ENGINE_VERSION,
                                effect_id,
                                attempt_id,
                                &member_seat,
                                member_input,
                                |_| {}, // member checkpoints journal after the join
                            ),
                        };
                        (name, report)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("panel member thread"))
                .collect()
        });

        // Journal member evidence in declared (stable) order — never
        // wall-clock completion order.
        for (name, report) in &reports {
            let kind = match &report.outcome {
                AttemptOutcome::Succeeded { .. } => "succeeded",
                AttemptOutcome::Failed { .. } => "failed",
                AttemptOutcome::Indeterminate { .. } => "indeterminate",
            };
            self.append(
                EventType::EffectCheckpointed,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "checkpoint": {
                        "step": "panel-member-finished",
                        "member": name,
                        "outcome": kind,
                        "session_ref": report.session_ref,
                        "inner_checkpoints": report.checkpoints.len(),
                    },
                }),
                Some(attempt_id.to_string()),
            )?;
        }

        let indeterminate: Vec<&str> = reports
            .iter()
            .filter(|(_, r)| matches!(r.outcome, AttemptOutcome::Indeterminate { .. }))
            .map(|(n, _)| n.as_str())
            .collect();
        if !indeterminate.is_empty() {
            self.append(
                EventType::EffectIndeterminate,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "reason": format!(
                        "panel members {indeterminate:?} could not establish completion"
                    ),
                }),
                Some(attempt_id.to_string()),
            )?;
            return Ok(());
        }
        let failures: Vec<String> = reports
            .iter()
            .filter_map(|(n, r)| match &r.outcome {
                AttemptOutcome::Failed { error } => Some(format!("{n}: {error}")),
                _ => None,
            })
            .collect();
        if !failures.is_empty() {
            self.append(
                EventType::EffectFailed,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": format!("panel members failed — {}", failures.join("; ")),
                }),
                Some(attempt_id.to_string()),
            )?;
            return Ok(());
        }
        let member_results: Vec<(String, Value)> = reports
            .into_iter()
            .map(|(n, r)| match r.outcome {
                AttemptOutcome::Succeeded { result } => (n, result),
                _ => unreachable!("filtered above"),
            })
            .collect();
        let aggregated = aggregate_results(aggregate, &member_results);
        self.append(
            EventType::EffectSucceeded,
            json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": aggregated}),
            Some(attempt_id.to_string()),
        )?;
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

        // Seat-supplied facts: everything the engine owns is dropped, and
        // only the seat's DECLARED inputs survive (decision 0007) — an
        // undeclared claim never reaches the table or the journal record.
        let mut inputs: Map<String, Value> = object
            .get("inputs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for owned in ENGINE_OWNED_INPUTS {
            inputs.remove(owned);
        }
        inputs.retain(|key, _| seat.inputs.iter().any(|declared| declared == key));
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
    let commanded = store.append_next(
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
        Some(commanded.event_id),
        None,
    )?;
    Ok(())
}

/// Deterministic, order-independent aggregation of member results. A
/// member payload without a readable result string poisons the aggregate
/// into a marker the engine's own schema validation parks with the full
/// member evidence attached — never coerced (law 0001). A member result
/// outside the vocabulary ranks WORST and flows to decide(), whose
/// declared-results check parks it with evidence.
fn aggregate_results(aggregate: Aggregate, members: &[(String, Value)]) -> Value {
    let mut notes = Map::new();
    let mut verdicts = Map::new();
    let mut parsed: Vec<(&str, &str, &Value)> = Vec::new();
    for (name, payload) in members {
        if let Some(note) = payload.get("notes") {
            notes.insert(name.clone(), note.clone());
        }
        match payload.get("result").and_then(Value::as_str) {
            Some(result) => {
                verdicts.insert(name.clone(), Value::String(result.to_string()));
                parsed.push((name.as_str(), result, payload));
            }
            None => {
                let evidence: Map<String, Value> =
                    members.iter().cloned().collect();
                return json!({
                    "result": "__member-schema-invalid__",
                    "notes": {"members": Value::Object(evidence)},
                });
            }
        }
    }
    let meta = json!({"members": notes, "verdicts": verdicts});
    match aggregate {
        Aggregate::UnanimousPass => {
            let all_pass = parsed.iter().all(|(_, r, _)| *r == "pass");
            json!({
                "result": if all_pass { "pass" } else { "fail" },
                "notes": meta,
            })
        }
        Aggregate::ReviewPanel => {
            let rank = |r: &str| match r {
                "clean" => 0,
                "residual" => 1,
                "security-hold" => 2,
                _ => 3, // unknown ranks worst: fail closed via decide()
            };
            let worst = parsed
                .iter()
                .max_by_key(|(_, r, _)| rank(r))
                .expect("panels have members");
            let mut severity_rank = 0usize;
            let mut has_security = false;
            let mut fixes_applied = false;
            for (_, _, payload) in &parsed {
                if let Some(inputs) = payload.get("inputs").and_then(Value::as_object) {
                    if let Some(s) = inputs.get("max_residual_severity").and_then(Value::as_str)
                    {
                        if let Some(i) = SEVERITY_ORDER.iter().position(|x| *x == s) {
                            severity_rank = severity_rank.max(i);
                        }
                    }
                    has_security |= inputs
                        .get("has_security_residual")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    fixes_applied |= inputs
                        .get("fixes_applied")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                }
            }
            let mut inputs = Map::new();
            inputs.insert("fixes_applied".into(), Value::Bool(fixes_applied));
            if worst.1 == "residual" || severity_rank > 0 {
                inputs.insert(
                    "max_residual_severity".into(),
                    Value::String(SEVERITY_ORDER[severity_rank].to_string()),
                );
                inputs.insert("has_security_residual".into(), Value::Bool(has_security));
            }
            json!({"result": worst.1, "inputs": inputs, "notes": meta})
        }
    }
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
