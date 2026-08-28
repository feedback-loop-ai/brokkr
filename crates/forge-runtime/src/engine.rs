//! The durable engine loop. Every external effect is requested durably
//! before execution and completed, failed, cancelled, or marked
//! indeterminate by a later event (decision 0003). The loop derives its
//! next action purely from `fold(journal)` + the pinned bundle; nothing
//! in here decides a transition — only the policy does.

use std::path::{Path, PathBuf};
use std::process::Command;

use forge_core::envelope::EventType;
use forge_core::dispatch::{
    build_run_manifest_v2, bundle_manifest_from_run, DispatchEnvelopeV2, DispatchError,
};
use forge_core::fold::{computed_inputs, fold, Cursor, RunState, Status};
use forge_core::policy::Outcome;
use forge_core::EventEnvelope;
use forge_protocol::process::DriverProcess;
use forge_protocol::AttemptOutcome;
use forge_store::Store;
use serde_json::{json, Map, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::bundle::{
    Aggregate, Bundle, Confine, PanelMember, SeatBody, SequenceStep, StepBody,
    ENGINE_OWNED_INPUTS, ENGINE_VERSION,
};
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
    #[error("dispatch: {0}")]
    Dispatch(#[from] forge_core::dispatch::DispatchError),
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

fn verify_dispatch_bundle_bounds(
    dispatch: &DispatchEnvelopeV2,
    bundle: &Bundle,
) -> Result<(), DispatchError> {
    let max_attempts = bundle
        .seats
        .values()
        .map(|seat| seat.limits.max_attempts)
        .max()
        .unwrap_or(1);
    let max_parallel = bundle
        .seats
        .values()
        .map(|seat| match &seat.body {
            SeatBody::Single { .. } => 1,
            SeatBody::Panel { members, .. } => members.len(),
            SeatBody::Sequence { steps } => steps
                .iter()
                .map(|step| match &step.body {
                    StepBody::Single { .. } => 1,
                    StepBody::Panel { members, .. } => members.len(),
                })
                .max()
                .unwrap_or(1),
        })
        .max()
        .unwrap_or(1);
    if max_attempts > u64::from(dispatch.bounds.max_attempts)
        || max_parallel > dispatch.bounds.max_parallel_effects as usize
    {
        return Err(DispatchError::UnsafeBounds);
    }
    Ok(())
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

    /// Start a Looper-bound run under the exact Forge run id and immutable
    /// dispatch envelope. The ordinary bundle manifest remains recoverable
    /// for resume compatibility, while the stored/exported manifest is v2.
    pub fn start_with_dispatch(
        mut store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
        dispatch: DispatchEnvelopeV2,
    ) -> Result<Engine, EngineError> {
        let run_id = dispatch.forge_run_id.clone();
        dispatch.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
        verify_dispatch_bundle_bounds(&dispatch, &bundle)?;
        let manifest = build_run_manifest_v2(&bundle.manifest, dispatch)?;
        store.create_run(&run_id, feature, &bundle.name, &manifest)?;
        store.append_next(
            &run_id,
            EventType::RunStarted,
            json!({"feature": feature, "manifest": manifest}),
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
        let pinned_bundle = bundle_manifest_from_run(&pinned)?;
        if let Some(dispatch) = forge_core::dispatch::dispatch_from_run(&pinned)? {
            dispatch.verify(time::OffsetDateTime::now_utc(), &bundle.manifest_digest())?;
            verify_dispatch_bundle_bounds(&dispatch, &bundle)?;
        }
        if pinned_bundle != bundle.manifest {
            let detail = manifest_diff(&pinned_bundle, &bundle.manifest);
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
            SeatBody::Sequence { steps } => {
                // The requested-time input enumerates the WHOLE sequence;
                // per-step driver inputs are derived from it
                // deterministically at execution time. Step ORDER is
                // load-bearing (steps run serially, and the digest must
                // rebuild identically): an array carries it — a JSON
                // object would sort its keys.
                let step_values: Vec<Value> = steps
                    .iter()
                    .map(|step| match &step.body {
                        StepBody::Single { role_path, .. } => json!({
                            "name": step.name,
                            "role_path": role_path.to_string_lossy(),
                            "result_path": workdir
                                .join(".forge/results")
                                .join(format!("{effect_id}-{}.json", step.name))
                                .to_string_lossy(),
                        }),
                        StepBody::Panel { members, aggregate } => {
                            let mut member_map = Map::new();
                            for member in members {
                                member_map.insert(
                                    member.name.clone(),
                                    json!({
                                        "role_path": member.role_path.to_string_lossy(),
                                        "result_path": workdir
                                            .join(".forge/results")
                                            .join(format!(
                                                "{effect_id}-{}-{}.json",
                                                step.name, member.name
                                            ))
                                            .to_string_lossy(),
                                    }),
                                );
                            }
                            json!({
                                "name": step.name,
                                "aggregate": format!("{aggregate:?}"),
                                "members": Value::Object(member_map),
                            })
                        }
                    })
                    .collect();
                Ok(json!({
                    "feature": self.feature,
                    "phase": phase,
                    "seat": phase,
                    "workdir": workdir.to_string_lossy(),
                    "steps": step_values,
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
            SeatBody::Sequence { steps } => format!("sequence[{}]", steps.len()),
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

        match &seat.body {
            SeatBody::Panel { members, aggregate } => self.execute_panel(
                effect_id, &attempt_id, seat_name, members, *aggregate, &input, deadline,
            ),
            SeatBody::Sequence { steps } => {
                self.execute_sequence(effect_id, &attempt_id, seat_name, steps, &input, deadline)
            }
            SeatBody::Single { command, confine, .. } => {
                let command =
                    confined_command(command, confine.as_ref(), &workdir, &self.bundle.dir);
                let run = self.run_driver(
                    effect_id, &attempt_id, seat_name, &command, input, deadline, None,
                )?;
                self.conclude_single(effect_id, &attempt_id, run)
            }
        }
    }

    /// Conclude a single-driver attempt with its terminal effect event.
    /// The stderr tail rides on failed/indeterminate outcomes; a spawn
    /// failure has no stderr and carries none.
    fn conclude_single(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        run: DriverRun,
    ) -> Result<(), EngineError> {
        let report = match run {
            DriverRun::SpawnFailed(error) => {
                self.append(
                    EventType::EffectFailed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": error,
                    }),
                    Some(attempt_id.to_string()),
                )?;
                return Ok(());
            }
            DriverRun::Ran(report) => report,
        };
        let stderr_tail = stderr_tail(&report.stderr);
        match report.outcome {
            AttemptOutcome::Succeeded { result } => {
                self.append(
                    EventType::EffectSucceeded,
                    json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
                    Some(attempt_id.to_string()),
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
                    Some(attempt_id.to_string()),
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
                    Some(attempt_id.to_string()),
                )?;
            }
        }
        Ok(())
    }

    /// Spawn one driver and run one attempt, journaling its live
    /// checkpoints as they stream — member-tagged when `member_tag`
    /// names a sequence step. Appends NO terminal effect event: the
    /// caller owns the attempt's conclusion.
    #[allow(clippy::too_many_arguments)]
    fn run_driver(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        driver_seat: &str,
        command: &[String],
        input: Value,
        deadline: std::time::Duration,
        member_tag: Option<&str>,
    ) -> Result<DriverRun, EngineError> {
        let workdir = self.workdir();
        let process = match DriverProcess::spawn(command, &workdir, Some(deadline)) {
            Err(e) => return Ok(DriverRun::SpawnFailed(format!("driver did not spawn: {e}"))),
            Ok(process) => process,
        };
        let mut checkpoint_error: Option<EngineError> = None;
        let store = &mut self.store;
        let current_cause = &mut self.current_cause;
        let run_id = self.run_id.clone();
        let report = process.run_attempt(
            ENGINE_VERSION,
            effect_id,
            attempt_id,
            driver_seat,
            input,
            |data| {
                if checkpoint_error.is_none() {
                    let checkpoint = match member_tag {
                        None => data.clone(),
                        Some(tag) => tag_member(data.clone(), tag),
                    };
                    match store.append_next(
                        &run_id,
                        EventType::EffectCheckpointed,
                        json!({
                            "effect_id": effect_id,
                            "attempt_id": attempt_id,
                            "checkpoint": checkpoint,
                        }),
                        current_cause.clone(),
                        Some(attempt_id.to_string()),
                    ) {
                        // Causal chain advances through checkpoints
                        // too — the closing effect event names the
                        // last checkpoint as its cause.
                        Ok(envelope) => {
                            *current_cause = Some(envelope.event_id);
                        }
                        Err(e) => checkpoint_error = Some(e.into()),
                    }
                }
            },
        );
        if let Some(e) = checkpoint_error {
            return Err(e);
        }
        Ok(DriverRun::Ran(report))
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
        let runs = self.member_runs(
            seat_name,
            members,
            &panel_input["members"],
            panel_input,
            &panel_input["context"],
        );
        let reports = self.run_panel(effect_id, attempt_id, &runs, deadline, "")?;
        self.journal_panel_members(effect_id, attempt_id, &reports, "")?;
        match panel_outcome(aggregate, reports) {
            AttemptOutcome::Indeterminate { reason } => {
                self.append(
                    EventType::EffectIndeterminate,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "reason": reason,
                    }),
                    Some(attempt_id.to_string()),
                )?;
            }
            AttemptOutcome::Failed { error } => {
                self.append(
                    EventType::EffectFailed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": error,
                    }),
                    Some(attempt_id.to_string()),
                )?;
            }
            AttemptOutcome::Succeeded { result } => {
                self.append(
                    EventType::EffectSucceeded,
                    json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
                    Some(attempt_id.to_string()),
                )?;
            }
        }
        Ok(())
    }

    /// Derive one driver invocation per panel member from the
    /// requested-time input: `members_meta` holds the per-member
    /// role/result paths, `driver_seat_prefix` is the seat name (or
    /// `<seat>:<step>` inside a sequence), and `context` already carries
    /// any accumulated prior step results.
    fn member_runs(
        &self,
        driver_seat_prefix: &str,
        members: &[PanelMember],
        members_meta: &Value,
        seat_input: &Value,
        context: &Value,
    ) -> Vec<MemberRun> {
        let workdir = self.workdir();
        members
            .iter()
            .map(|member| MemberRun {
                name: member.name.clone(),
                driver_seat: format!("{driver_seat_prefix}:{}", member.name),
                command: confined_command(
                    &member.command,
                    member.confine.as_ref(),
                    &workdir,
                    &self.bundle.dir,
                ),
                input: json!({
                    "feature": seat_input["feature"],
                    "phase": seat_input["phase"],
                    "seat": format!("{driver_seat_prefix}:{}", member.name),
                    "role_path": members_meta[&member.name]["role_path"],
                    "workdir": seat_input["workdir"],
                    "result_path": members_meta[&member.name]["result_path"],
                    "allowed_results": seat_input["allowed_results"],
                    "context": context,
                }),
            })
            .collect()
    }

    /// Run panel members concurrently INSIDE one attempt, journaling
    /// live member checkpoints as they arrive, and return the reports in
    /// declared order. Appends NO terminal effect event. `tag_prefix` is
    /// empty for a seat-level panel and `<step>:` inside a sequence, so
    /// the journaled member tag reads `<member>` or `<step>:<member>`.
    fn run_panel(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        runs: &[MemberRun],
        deadline: std::time::Duration,
        tag_prefix: &str,
    ) -> Result<Vec<(String, AttemptReport)>, EngineError> {
        let workdir = self.workdir();
        // Split the borrows: member threads run drivers, while the
        // main-thread receive loop below needs the store and causal cursor.
        let store = &mut self.store;
        let current_cause = &mut self.current_cause;
        let run_id = self.run_id.clone();
        let mut checkpoint_error: Option<EngineError> = None;
        let reports: Vec<(String, AttemptReport)> = std::thread::scope(|scope| {
            let (sender, receiver) = std::sync::mpsc::channel::<(String, Value)>();
            let handles: Vec<_> = runs
                .iter()
                .map(|run| {
                    let name = run.name.clone();
                    let checkpoint_name = format!("{tag_prefix}{}", run.name);
                    let workdir = workdir.clone();
                    let sender = sender.clone();
                    scope.spawn(move || {
                        let report = match DriverProcess::spawn(
                            &run.command,
                            &workdir,
                            Some(deadline),
                        ) {
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
                                &run.driver_seat,
                                run.input.clone(),
                                // Live telemetry: hand each checkpoint to the
                                // main thread — the store has one writer.
                                |data| {
                                    let _ = sender
                                        .send((checkpoint_name.clone(), data.clone()));
                                },
                            ),
                        };
                        (name, report)
                    })
                })
                .collect();
            // The main thread journals live member checkpoints as they
            // arrive (wall-clock order — checkpoints are temporal evidence,
            // nothing aggregates from them). Dropping our sender makes the
            // loop end when the last member finishes emitting. On append
            // error, latch and keep draining — an abandoned channel must
            // not deadlock the members.
            drop(sender);
            for (member, checkpoint) in receiver {
                if checkpoint_error.is_some() {
                    continue;
                }
                let checkpoint = tag_member(checkpoint, &member);
                match store.append_next(
                    &run_id,
                    EventType::EffectCheckpointed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "checkpoint": checkpoint,
                    }),
                    current_cause.clone(),
                    Some(attempt_id.to_string()),
                ) {
                    Ok(envelope) => {
                        *current_cause = Some(envelope.event_id);
                    }
                    Err(e) => checkpoint_error = Some(e.into()),
                }
            }
            handles
                .into_iter()
                .map(|h| h.join().expect("panel member thread"))
                .collect()
        });
        if let Some(e) = checkpoint_error {
            return Err(e);
        }
        Ok(reports)
    }

    /// Journal member outcomes as checkpoint evidence in declared
    /// (stable) order — never wall-clock completion order.
    fn journal_panel_members(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        reports: &[(String, AttemptReport)],
        tag_prefix: &str,
    ) -> Result<(), EngineError> {
        for (name, report) in reports {
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
                        "member": format!("{tag_prefix}{name}"),
                        "outcome": kind,
                        "session_ref": report.session_ref,
                        "inner_checkpoints": report.checkpoints.len(),
                    },
                }),
                Some(attempt_id.to_string()),
            )?;
        }
        Ok(())
    }

    /// Run named steps one after another INSIDE one attempt (decision
    /// 0002's serial form). Per-step driver inputs are derived
    /// deterministically from the requested-time seat input; earlier
    /// steps' result objects reach later steps as
    /// `context.prior_results`. Any step failing fails the WHOLE attempt
    /// (0006-retryable — a retry restarts from step 1); an indeterminate
    /// step parks it. The FINAL step's result object is the effect's
    /// single typed result — decide() validates it exactly as today.
    fn execute_sequence(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        seat_name: &str,
        steps: &[SequenceStep],
        seq_input: &Value,
        deadline: std::time::Duration,
    ) -> Result<(), EngineError> {
        let mut prior_results = Map::new();
        for (index, step) in steps.iter().enumerate() {
            let step_meta = &seq_input["steps"][index];
            let context = {
                let mut context = seq_input["context"]
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                context.insert("prior_results".into(), Value::Object(prior_results.clone()));
                Value::Object(context)
            };
            let outcome = match &step.body {
                StepBody::Single { command, confine, .. } => {
                    let driver_seat = format!("{seat_name}:{}", step.name);
                    let input = json!({
                        "feature": seq_input["feature"],
                        "phase": seq_input["phase"],
                        "seat": driver_seat,
                        "role_path": step_meta["role_path"],
                        "workdir": seq_input["workdir"],
                        "result_path": step_meta["result_path"],
                        "allowed_results": seq_input["allowed_results"],
                        "context": context,
                    });
                    let command = confined_command(
                        command,
                        confine.as_ref(),
                        &self.workdir(),
                        &self.bundle.dir,
                    );
                    match self.run_driver(
                        effect_id,
                        attempt_id,
                        &driver_seat,
                        &command,
                        input,
                        deadline,
                        Some(&step.name),
                    )? {
                        DriverRun::SpawnFailed(error) => AttemptOutcome::Failed { error },
                        DriverRun::Ran(report) => {
                            // The step driver's stderr tail rides on the
                            // attempt's terminal event, as a single
                            // seat's does.
                            let tail = stderr_tail(&report.stderr);
                            match report.outcome {
                                AttemptOutcome::Succeeded { result } => {
                                    AttemptOutcome::Succeeded { result }
                                }
                                AttemptOutcome::Failed { error } => AttemptOutcome::Failed {
                                    error: format!("{error}; stderr tail: {tail}"),
                                },
                                AttemptOutcome::Indeterminate { reason } => {
                                    AttemptOutcome::Indeterminate {
                                        reason: format!("{reason}; stderr tail: {tail}"),
                                    }
                                }
                            }
                        }
                    }
                }
                StepBody::Panel { members, aggregate } => {
                    let tag_prefix = format!("{}:", step.name);
                    let runs = self.member_runs(
                        &format!("{seat_name}:{}", step.name),
                        members,
                        &step_meta["members"],
                        seq_input,
                        &context,
                    );
                    let reports =
                        self.run_panel(effect_id, attempt_id, &runs, deadline, &tag_prefix)?;
                    self.journal_panel_members(effect_id, attempt_id, &reports, &tag_prefix)?;
                    panel_outcome(*aggregate, reports)
                }
            };
            let result = match outcome {
                AttemptOutcome::Succeeded { result } => result,
                AttemptOutcome::Failed { error } => {
                    self.append(
                        EventType::EffectFailed,
                        json!({
                            "effect_id": effect_id,
                            "attempt_id": attempt_id,
                            "error": format!("sequence step '{}': {error}", step.name),
                        }),
                        Some(attempt_id.to_string()),
                    )?;
                    return Ok(());
                }
                AttemptOutcome::Indeterminate { reason } => {
                    self.append(
                        EventType::EffectIndeterminate,
                        json!({
                            "effect_id": effect_id,
                            "attempt_id": attempt_id,
                            "reason": format!("sequence step '{}': {reason}", step.name),
                        }),
                        Some(attempt_id.to_string()),
                    )?;
                    return Ok(());
                }
            };
            if index + 1 == steps.len() {
                self.append(
                    EventType::EffectSucceeded,
                    json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
                    Some(attempt_id.to_string()),
                )?;
            } else {
                self.append(
                    EventType::EffectCheckpointed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "checkpoint": {
                            "step": "sequence-step-finished",
                            "step_name": step.name,
                            "result": result,
                        },
                    }),
                    Some(attempt_id.to_string()),
                )?;
                prior_results.insert(step.name.clone(), result);
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
                requires_artifacts,
                ..
            } => {
                // Artifact gate: an advancing ruling that names artifacts
                // advances only if each exists as a non-empty regular file
                // in the workdir, probed exactly once, here, at decide
                // time. A miss fails closed through the park path — no
                // seat is asked, no seat attests (decision 0001).
                let failures = if requires_artifacts.is_empty() {
                    Vec::new()
                } else {
                    artifact_failures(&self.workdir(), &requires_artifacts)
                };
                if failures.is_empty() {
                    json!({
                        "from": phase,
                        "result": result,
                        "rule_id": rule_id,
                        "next": next_phase,
                        "severity": severity,
                        "inputs": inputs,
                        "problem": null,
                    })
                } else {
                    // Severity is a property of a taken transition; none
                    // is taken. rule_id stays: the rule DID match, and
                    // that identity distinguishes a gate block from
                    // NoRule in the journal.
                    json!({
                        "from": phase,
                        "result": result,
                        "rule_id": rule_id,
                        "next": null,
                        "severity": null,
                        "inputs": inputs,
                        "problem": artifact_problem(&rule_id, &failures),
                    })
                }
            }
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
    let head_event = store.load(run_id)?.last().map(|e| e.event_id.clone());
    let commanded = store.append_next(
        run_id,
        EventType::OperatorCommanded,
        json!({"command_id": command_id, "command": command, "args": {}, "operator": operator}),
        head_event,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FencedCommandOutcome {
    Accepted { head_seq: u64, head_hash: String },
    Rejected {
        reason: String,
        head_seq: u64,
        head_hash: String,
    },
}

/// Apply a command received through the Looper producer bridge. The command id
/// is supplied by Looper, the expected cursor/hash fences concurrent operator
/// activity, and both acceptance and rejection become Forge journal evidence
/// before any control-state effect is possible.
pub fn apply_fenced_operator_command(
    store: &mut Store,
    run_id: &str,
    command_id: &str,
    command: &str,
    operator: &str,
    reason: &str,
    expected_seq: u64,
    expected_hash: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    let events = store.load(run_id)?;
    if let Some(commanded) = events.iter().find(|event| {
        event.event_type == EventType::OperatorCommanded
            && event.payload.get("command_id").and_then(Value::as_str) == Some(command_id)
    }) {
        if let Some(disposition) = events.iter().find(|event| {
            event.seq > commanded.seq
                && matches!(
                    event.event_type,
                    EventType::OperatorAccepted | EventType::OperatorRejected
                )
                && event.payload.get("command_id").and_then(Value::as_str) == Some(command_id)
        }) {
            return Ok(if disposition.event_type == EventType::OperatorAccepted {
                FencedCommandOutcome::Accepted {
                    head_seq: disposition.seq,
                    head_hash: disposition.event_hash.clone(),
                }
            } else {
                FencedCommandOutcome::Rejected {
                    reason: disposition
                        .payload
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("previously_rejected")
                        .to_string(),
                    head_seq: disposition.seq,
                    head_hash: disposition.event_hash.clone(),
                }
            });
        }
        let rejected = store.append_next(
            run_id,
            EventType::OperatorRejected,
            json!({
                "command_id": command_id,
                "operator": operator,
                "reason": "incomplete_command_replay",
            }),
            Some(commanded.event_id.clone()),
            None,
        )?;
        fold(&store.load(run_id)?)?;
        return Ok(FencedCommandOutcome::Rejected {
            reason: "incomplete_command_replay".into(),
            head_seq: rejected.seq,
            head_hash: rejected.event_hash,
        });
    }
    let state = fold(&events)?;
    let (head_seq, head_hash) = store.head_hash(run_id)?;
    let rejection = if command != "retry" && command != "stop" {
        Some("command_not_allowed")
    } else if head_seq != expected_seq || head_hash != expected_hash {
        Some("stale_cursor")
    } else if state.status != Status::AwaitingOperator {
        Some("run_not_awaiting_operator")
    } else {
        None
    };
    let cause = events.last().map(|event| event.event_id.clone());
    let commanded = store.append_next(
        run_id,
        EventType::OperatorCommanded,
        json!({
            "command_id": command_id,
            "command": command,
            "args": {},
            "operator": operator,
        }),
        cause,
        None,
    )?;
    let disposition = if let Some(rejection) = rejection {
        store.append_next(
            run_id,
            EventType::OperatorRejected,
            json!({"command_id": command_id, "operator": operator, "reason": rejection}),
            Some(commanded.event_id),
            None,
        )?;
        let (head_seq, head_hash) = store.head_hash(run_id)?;
        FencedCommandOutcome::Rejected {
            reason: rejection.into(),
            head_seq,
            head_hash,
        }
    } else {
        store.append_next(
            run_id,
            EventType::OperatorAccepted,
            json!({"command_id": command_id, "operator": operator, "reason": reason}),
            Some(commanded.event_id),
            None,
        )?;
        let (head_seq, head_hash) = store.head_hash(run_id)?;
        FencedCommandOutcome::Accepted {
            head_seq,
            head_hash,
        }
    };
    // Prove the newly appended pair does not corrupt fold semantics before the
    // bridge acknowledges it to Looper.
    fold(&store.load(run_id)?)?;
    Ok(disposition)
}

/// A driver invocation that got as far as running, or one that never
/// spawned. Spawn failures carry no stderr, and their terminal error
/// grows no stderr tail — the distinction keeps event payloads exactly
/// as they were before this enum existed.
enum DriverRun {
    SpawnFailed(String),
    Ran(AttemptReport),
}

/// One panel member's derived driver invocation: the `seat` string the
/// driver sees, the (possibly confined) command, and its input.
struct MemberRun {
    name: String,
    driver_seat: String,
    command: Vec<String>,
    input: Value,
}

/// Tag a checkpoint payload with the member/step it came from — the
/// `member` field the seats console groups by.
fn tag_member(checkpoint: Value, member: &str) -> Value {
    match checkpoint {
        Value::Object(mut object) => {
            object.insert("member".into(), Value::String(member.to_string()));
            Value::Object(object)
        }
        other => json!({"member": member, "value": other}),
    }
}

fn stderr_tail(stderr: &str) -> String {
    stderr.chars().rev().take(2000).collect::<String>().chars().rev().collect()
}

/// Join member reports into the attempt's single outcome: any
/// indeterminate member parks the whole attempt; otherwise any failed
/// member fails it (retryable under 0006); otherwise the declared
/// aggregate produces the one typed result.
fn panel_outcome(aggregate: Aggregate, reports: Vec<(String, AttemptReport)>) -> AttemptOutcome {
    let indeterminate: Vec<&str> = reports
        .iter()
        .filter(|(_, r)| matches!(r.outcome, AttemptOutcome::Indeterminate { .. }))
        .map(|(n, _)| n.as_str())
        .collect();
    if !indeterminate.is_empty() {
        return AttemptOutcome::Indeterminate {
            reason: format!("panel members {indeterminate:?} could not establish completion"),
        };
    }
    let failures: Vec<String> = reports
        .iter()
        .filter_map(|(n, r)| match &r.outcome {
            AttemptOutcome::Failed { error } => Some(format!("{n}: {error}")),
            _ => None,
        })
        .collect();
    if !failures.is_empty() {
        return AttemptOutcome::Failed {
            error: format!("panel members failed — {}", failures.join("; ")),
        };
    }
    let member_results: Vec<(String, Value)> = reports
        .into_iter()
        .map(|(n, r)| match r.outcome {
            AttemptOutcome::Succeeded { result } => (n, result),
            _ => unreachable!("filtered above"),
        })
        .collect();
    AttemptOutcome::Succeeded {
        result: aggregate_results(aggregate, &member_results),
    }
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
            // Only the declared vocabulary aggregates; anything else is
            // never coerced to "fail" (law 0001) — it poisons the result
            // so decide() parks with the member evidence.
            if parsed.iter().any(|(_, r, _)| *r != "pass" && *r != "fail") {
                return json!({
                    "result": "__member-schema-invalid__",
                    "notes": meta,
                });
            }
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

/// Wrap a driver command for the policy-confined trust class: pinned
/// image, stdio through, workdir mounted writable at the same path (so
/// role/result paths stay valid), bundle dir read-only, extra declared
/// mounts read-only, network off unless granted. Absence of confinement
/// is the trusted class: a native child process.
pub fn confined_command(
    command: &[String],
    confine: Option<&Confine>,
    workdir: &std::path::Path,
    bundle_dir: &std::path::Path,
) -> Vec<String> {
    let Some(confine) = confine else {
        return command.to_vec();
    };
    let mut wrapped = vec![
        "docker".to_string(),
        "run".to_string(),
        "--rm".to_string(),
        "-i".to_string(),
        "-v".to_string(),
        format!("{}:{}", workdir.display(), workdir.display()),
        "-v".to_string(),
        format!("{}:{}:ro", bundle_dir.display(), bundle_dir.display()),
        "-w".to_string(),
        workdir.display().to_string(),
    ];
    if !confine.network {
        wrapped.push("--network=none".to_string());
    }
    for mount in &confine.mounts {
        wrapped.push("-v".to_string());
        wrapped.push(format!("{mount}:{mount}:ro"));
    }
    wrapped.push(confine.image.clone());
    wrapped.extend(command.iter().cloned());
    wrapped
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

/// The `requires_artifacts` gate's verdict: every failing entry with its
/// class, in table order — the complete list, not first-fail, so one park
/// carries all the evidence. Empty means the gate passes. Entries are
/// strictly static workdir-relative paths, verified verbatim: no
/// substitution, no tokens (`{ } $ < >` are reserved, not assigned), and
/// nothing that could resolve outside the workdir. Every probe error
/// fails closed. This is the only place the gate touches the filesystem.
fn artifact_failures(workdir: &Path, required: &[String]) -> Vec<(String, &'static str)> {
    let mut failures = Vec::new();
    for entry in required {
        let lexically_valid = !entry.is_empty()
            && !entry.starts_with('/')
            && !entry.contains(['\\', '\0', '{', '}', '$', '<', '>'])
            && !entry.split('/').any(|component| component == "." || component == "..");
        let class = if !lexically_valid {
            Some("invalid")
        } else {
            // metadata, not symlink_metadata: the gate asserts presence
            // of content, not provenance — a dangling symlink is missing.
            match std::fs::metadata(workdir.join(entry)) {
                Err(_) => Some("missing"),
                Ok(meta) if !meta.is_file() => Some("not-a-file"),
                Ok(meta) if meta.len() == 0 => Some("empty"),
                Ok(_) => None,
            }
        };
        if let Some(class) = class {
            failures.push((entry.clone(), class));
        }
    }
    failures
}

/// The single producer of the gate's journal-borne evidence string —
/// machine-stable contract, pinned character-exact by spec and proof.
fn artifact_problem(rule_id: &str, failures: &[(String, &'static str)]) -> String {
    let list = failures
        .iter()
        .map(|(entry, class)| format!("{class}: {entry}"))
        .collect::<Vec<_>>()
        .join("; ");
    format!("requires_artifacts unmet for rule {rule_id}: {list}")
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

#[cfg(test)]
mod artifact_gate_tests {
    use super::{artifact_failures, artifact_problem};

    fn classes(workdir: &std::path::Path, entries: &[&str]) -> Vec<(String, &'static str)> {
        let required: Vec<String> = entries.iter().map(|e| e.to_string()).collect();
        artifact_failures(workdir, &required)
    }

    #[test]
    fn present_non_empty_file_passes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("spec.md"), "content").unwrap();
        assert!(classes(dir.path(), &["spec.md"]).is_empty());
    }

    #[test]
    fn nested_path_passes_and_absent_nested_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("docs/plan.md"), "x").unwrap();
        assert!(classes(dir.path(), &["docs/plan.md"]).is_empty());
        assert_eq!(
            classes(dir.path(), &["docs/other.md"]),
            vec![("docs/other.md".to_string(), "missing")]
        );
    }

    #[test]
    fn absent_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            classes(dir.path(), &["spec.md"]),
            vec![("spec.md".to_string(), "missing")]
        );
    }

    #[test]
    fn zero_byte_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("spec.md"), "").unwrap();
        assert_eq!(
            classes(dir.path(), &["spec.md"]),
            vec![("spec.md".to_string(), "empty")]
        );
    }

    #[test]
    fn directory_is_not_a_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("spec.md")).unwrap();
        assert_eq!(
            classes(dir.path(), &["spec.md"]),
            vec![("spec.md".to_string(), "not-a-file")]
        );
    }

    #[test]
    fn lexical_predicate_fails_closed_as_invalid() {
        let dir = tempfile::tempdir().unwrap();
        // Traversal, absolute, current-dir component, backslash, NUL,
        // empty — none ever reaches the filesystem.
        for entry in ["../escape", "a/../b", "..", "/etc/hosts", "./spec.md", ".",
                      "a\\b", "a\0b", ""] {
            assert_eq!(
                classes(dir.path(), &[entry]),
                vec![(entry.to_string(), "invalid")],
                "entry {entry:?} must be invalid"
            );
        }
    }

    #[test]
    fn reserved_characters_are_fenced() {
        let dir = tempfile::tempdir().unwrap();
        for entry in ["{slug}", "specs/{slug}/spec.md", "$HOME", "a<b", "a>b", "x}y"] {
            assert_eq!(
                classes(dir.path(), &[entry]),
                vec![(entry.to_string(), "invalid")],
                "entry {entry:?} must be invalid"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_followed_and_dangling_reads_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real.md"), "content").unwrap();
        std::os::unix::fs::symlink(dir.path().join("real.md"), dir.path().join("link.md"))
            .unwrap();
        std::os::unix::fs::symlink(dir.path().join("gone.md"), dir.path().join("dangling.md"))
            .unwrap();
        assert!(classes(dir.path(), &["link.md"]).is_empty(), "content presence, not provenance");
        assert_eq!(
            classes(dir.path(), &["dangling.md"]),
            vec![("dangling.md".to_string(), "missing")]
        );
    }

    #[test]
    fn failures_keep_table_order_across_classes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("empty.md"), "").unwrap();
        std::fs::create_dir(dir.path().join("dir.md")).unwrap();
        std::fs::write(dir.path().join("ok.md"), "x").unwrap();
        assert_eq!(
            classes(dir.path(), &["gone.md", "empty.md", "{slug}", "ok.md", "dir.md"]),
            vec![
                ("gone.md".to_string(), "missing"),
                ("empty.md".to_string(), "empty"),
                ("{slug}".to_string(), "invalid"),
                ("dir.md".to_string(), "not-a-file"),
            ]
        );
    }

    #[test]
    fn problem_string_is_character_exact() {
        let failures = vec![
            ("spec.md".to_string(), "missing"),
            ("plan.md".to_string(), "empty"),
            ("{slug}".to_string(), "invalid"),
        ];
        assert_eq!(
            artifact_problem("ARCH-OK", &failures),
            "requires_artifacts unmet for rule ARCH-OK: \
             missing: spec.md; empty: plan.md; invalid: {slug}"
        );
        assert_eq!(
            artifact_problem("R1", &[("spec.md".to_string(), "missing")]),
            "requires_artifacts unmet for rule R1: missing: spec.md"
        );
    }
}
