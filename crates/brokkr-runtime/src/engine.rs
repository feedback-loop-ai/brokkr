//! The durable engine loop. Every external effect is requested durably
//! before execution and completed, failed, cancelled, or marked
//! indeterminate by a later event (decision 0003). The loop derives its
//! next action purely from `fold(journal)` + the pinned bundle; nothing
//! in here decides a transition — only the policy does.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::dispatch::{
    build_run_manifest_v2, bundle_manifest_from_run, DispatchEnvelopeV2, DispatchError,
};
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{computed_inputs, fold, Cursor, RunState, Status};
use brokkr_core::policy::Outcome;
use brokkr_core::realms::{recorded_head, LEGACY_REALM_KEY};
use brokkr_core::EventEnvelope;
use brokkr_protocol::process::DriverProcess;
use brokkr_protocol::AttemptOutcome;
use brokkr_store::{SeatRecordError, Store, StoreError};
use serde_json::{json, Map, Value};
use thiserror::Error;
use uuid::Uuid;

use crate::agents::Candidate;
use crate::bundle::{
    dialect_results, Aggregate, Bundle, Confine, ExecutableBody, PanelMember, Seat, SeatBody,
    SeatClass, SequenceStep, StepBody, ENGINE_VERSION, REALM_FACTS,
};
use brokkr_core::policy::{SEVERITY_ORDER, VISIT_PREFIX};
use brokkr_protocol::AttemptReport;

fn nearest_change(context: &Value) -> Option<String> {
    ["tasks", "design", "specify", "triage", "intake"]
        .iter()
        .find_map(|phase| {
            context
                .pointer(&format!("/results/{phase}/inputs/change"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn expand_dialect_argv(argv: &[String], change: &str) -> Vec<String> {
    argv.iter()
        .map(|token| token.replace("{change}", change))
        .collect()
}

fn dialect_attempt_outcome(run: DriverRun) -> AttemptOutcome {
    match run {
        DriverRun::SpawnFailed(error) => AttemptOutcome::Failed { error },
        DriverRun::Ran(report) => report.outcome,
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] brokkr_store::StoreError),
    #[error("fold: {0}")]
    Fold(#[from] brokkr_core::FoldError),
    #[error("run '{run_id}' pins a different bundle: {detail}")]
    ManifestMismatch { run_id: String, detail: String },
    /// `conclude` refuses a run that already has its conclusion. There is
    /// nothing lawful left to append — a second `run/stopped` would fail
    /// the fold as an event after terminal — so the refusal comes before
    /// the first append, not after a half-written closure.
    #[error("run '{run_id}' is already concluded ({status}); conclude appends nothing")]
    AlreadyConcluded { run_id: String, status: String },
    /// `supersede` refused a citation, and wrote nothing (decision 0047
    /// ruling 2). Unlike `retry` and `stop`, a refused supersede leaves
    /// no `operator/rejected` behind: there is no pending command to
    /// dispose of, so the refusal is the operator's to read at the
    /// terminal and the journal stays exactly as it was.
    #[error("supersede refused, nothing was written: {0}")]
    SupersedeRefused(String),
    #[error("engine: {0}")]
    Other(String),
    #[error("dispatch: {0}")]
    Dispatch(#[from] brokkr_core::dispatch::DispatchError),
    #[error("realms: {0}")]
    World(#[from] crate::realms::WorldError),
}

impl EngineError {
    /// The contention this error carries, if that is what it is.
    ///
    /// [`EngineError::Store`] is `transparent`, so the store error it
    /// holds is NOT a link in its `source()` chain — a caller walking an
    /// `anyhow` chain would step straight past it. This is the door
    /// through, and it asks the store's own predicate rather than
    /// reading error text.
    pub fn contention(&self) -> Option<&brokkr_store::StoreError> {
        match self {
            EngineError::Store(error) if error.is_contention() => Some(error),
            _ => None,
        }
    }
}

pub struct Engine {
    pub store: Store,
    pub bundle: Bundle,
    pub run_id: String,
    pub feature: String,
    pub repo: Option<PathBuf>,
    /// The world this run was invoked into (decision 0023), when a map
    /// was in effect. It is pinned into the run manifest at start, so
    /// this field is the run's *live* copy of a fact the journal already
    /// answers for; the facts a decision records are keyed by the realm
    /// the repository is, when the map names one.
    pub world: Option<crate::realms::World>,
    /// The event_id every append chains to as `causation_id` — refreshed
    /// to the journal head each drive iteration, then to each event this
    /// iteration appends, so causal links mirror the engine's actual
    /// decision order (rendered by the UI timeline).
    current_cause: Option<String>,
    /// Operator-side secrets store override (decision 0012). Defaults to
    /// `<workdir>/.forge/secrets.env`. The engine only ever threads this
    /// PATH (plus declared names) into the driver start input — values
    /// are resolved by the exec driver at spawn time; no store read
    /// exists anywhere in brokkr-runtime.
    pub secrets_file: Option<PathBuf>,
    /// Present only while this process is executing a gate-bearing site,
    /// and it holds exactly one observation: the head as that site's own
    /// span began. The span is the gate STEP inside a sequence and the
    /// whole effect for a gate single seat or panel — never both at once,
    /// so a sequence's steps are the only observation a sequence makes.
    /// The observations are compared at the end of the span that armed
    /// them; only a mismatch is journaled, as the raw evidence on the
    /// resulting park. A span that ends through an `Err` — appending no
    /// terminal event — clears it rather than leaving it for the next
    /// effect to spend.
    active_gate_head: Option<Option<String>>,
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
    fn body_parallel(body: &SeatBody) -> usize {
        match body {
            SeatBody::Single { .. } => 1,
            SeatBody::Panel { members, .. } => members.len(),
            SeatBody::Sequence { steps } => steps
                .iter()
                .map(|step| match &step.body {
                    StepBody::Single { .. } => 1,
                    StepBody::Panel { members, .. } => members.len(),
                    StepBody::Dialect { .. } => 1,
                })
                .max()
                .unwrap_or(1),
            SeatBody::Select { cases, default, .. } => cases
                .values()
                .chain(default.iter().map(Box::as_ref))
                .map(body_parallel)
                .max()
                .unwrap_or(1),
        }
    }
    let max_parallel = bundle
        .seats
        .values()
        .map(|seat| body_parallel(&seat.body))
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
    fn enter_phase(&mut self, phase: &str, state: &RunState) -> Result<(), EngineError> {
        let seat = self.bundle.seats.get(phase);
        if let Some(seat) = seat {
            if seat.body.selected(state.strategy.as_deref()).is_none() {
                self.append(
                    EventType::RunParked,
                    json!({
                        "reason": format!(
                            "SELECT-NO-DEFAULT: seat '{phase}' selects on strategy, but the journal carries no matching triage result and the seat has no default"
                        ),
                        "evidence": {}
                    }),
                    None,
                )?;
                return Ok(());
            }
        }
        let payload = self.phase_entered_payload(phase, state);
        self.append(EventType::PhaseEntered, payload, None)?;
        Ok(())
    }

    pub fn start(
        store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
    ) -> Result<Engine, EngineError> {
        Engine::start_in_world(store, bundle, feature, repo, None)
    }

    /// Start a run inside a world (decision 0023). The map is pinned by
    /// content hash AND embedded verbatim into the run manifest — which
    /// rides inside `run/started` — so "what world did this run believe
    /// in?" is answerable from the journal alone, forever, whatever
    /// later became of the file. With no map the manifest is byte-for-
    /// byte the one this engine has always written.
    pub fn start_in_world(
        mut store: Store,
        bundle: Bundle,
        feature: &str,
        repo: Option<PathBuf>,
        world: Option<crate::realms::World>,
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
        let manifest = match &world {
            Some(world) => world.pinned(&bundle.manifest, repo.as_deref())?,
            None => bundle.manifest.clone(),
        };
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
            world,
            current_cause: None,
            secrets_file: None,
            active_gate_head: None,
        })
    }

    /// Start a Looper-bound run under the exact wire run id and immutable
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
            // A Looper-bound run pins a run-manifest/v2, whose bytes a
            // counterpart system reads; the map's pin belongs to the
            // v1→v5 local lineage. The CLI refuses the combination
            // rather than dropping a world silently.
            world: None,
            current_cause: None,
            secrets_file: None,
            active_gate_head: None,
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
        if let Some(dispatch) = brokkr_core::dispatch::dispatch_from_run(&pinned)? {
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
        let feature = fold(&events)?.feature.unwrap_or("unknown".to_string());
        Ok(Engine {
            store,
            bundle,
            run_id: run_id.to_string(),
            feature,
            repo,
            // Resume takes no map — and needs none. The world this run
            // believed in is pinned in the manifest just read, content
            // and all, so it is rehydrated from evidence rather than off
            // a disk that may have moved on. Without this a resumed run
            // would silently stop keying its facts by realm, changing
            // fact-shape mid-run depending on which verb was typed.
            world: crate::realms::World::from_manifest(&pinned)?,
            current_cause: None,
            secrets_file: None,
            active_gate_head: None,
        })
    }

    /// Drive the run until it parks, completes, or stops.
    ///
    /// Or until a peer's lock on the shared journal outlasts the store's
    /// whole patience — which is a fourth ending, and it is an ending,
    /// not a death. See [`Engine::lawful_end_under_contention`].
    pub fn drive(&mut self) -> Result<DriveEnd, EngineError> {
        loop {
            match self.drive_once() {
                Ok(Some(end)) => return Ok(end),
                Ok(None) => {}
                Err(error) => return self.lawful_end_under_contention(error),
            }
        }
    }

    /// One turn of the loop: `Some(end)` when the run has reached its
    /// conclusion, `None` when there is more to do.
    fn drive_once(&mut self) -> Result<Option<DriveEnd>, EngineError> {
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
                    // And the exhibits the journal cites, kept
                    // reachable past the branch delete and the gc
                    // that follow a landing (decision 0028). Same
                    // shape of act as the anchor: derived entirely
                    // from the journal, writing refs and never
                    // branches, so it crosses into no authority the
                    // operator keeps. Best-effort in the same way —
                    // a ref-planting gap is reported, never fatal.
                    if let Some(gap) =
                        crate::keep_refs::plant_or_report(&self.store, repo, &self.run_id)
                    {
                        eprintln!("{gap}");
                    }
                }
                return Ok(Some(DriveEnd { state }));
            }
            (Status::Running, _) => {
                self.advance_running(&events, state)?;
            }
        }
        Ok(None)
    }

    /// An engine that meets contention on the shared journal ends
    /// lawfully or not at all — it never dies of it.
    ///
    /// [`StoreError::Contended`] is the store saying it waited its whole
    /// patience for a peer's write lock and wrote nothing. That is an
    /// accident of timing, not a verdict: the same call made later is
    /// the same call. Every other error — including the fenced-append
    /// refusals, which are verdicts — leaves here exactly as it arrived.
    ///
    /// Where the fold admits a park, the contention is SAID, in the
    /// journal, in the run's own words: `run/parked` naming the lock it
    /// lost. `brokkr_core::fold` admits `run/parked` at exactly two
    /// cursors, so where it does not, this returns the typed contention
    /// instead of forging an event the fold would refuse — an engine
    /// that ends on contention must leave a journal that still folds.
    /// Either way the run is intact and `brokkr resume` picks it up:
    /// nothing was written, so nothing was lost.
    fn lawful_end_under_contention(&mut self, error: EngineError) -> Result<DriveEnd, EngineError> {
        let EngineError::Store(store_error) = &error else {
            return Err(error);
        };
        if !store_error.is_contention() {
            return Err(error);
        }
        let reason = format!("journal contention: {store_error}");
        let events = self.store.load(&self.run_id)?;
        let state = fold(&events)?;
        if !matches!(
            state.cursor,
            Cursor::Park { .. } | Cursor::ExecuteEffect { .. }
        ) {
            return Err(error);
        }
        self.current_cause = events.last().map(|e| e.event_id.clone());
        let parked = json!({"reason": reason, "evidence": {}});
        self.append(EventType::RunParked, parked, None)?;
        let state = fold(&self.store.load(&self.run_id)?)?;
        Ok(DriveEnd { state })
    }

    fn advance_running(
        &mut self,
        events: &[EventEnvelope],
        state: RunState,
    ) -> Result<(), EngineError> {
        match state.cursor.clone() {
            Cursor::Start => {
                let initial = self.bundle.machine.initial.clone();
                self.enter_phase(&initial, &state)?;
            }
            Cursor::EnterPhase { phase } => {
                self.enter_phase(&phase, &state)?;
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
                    .map(|seat| seat.limits)
                    .unwrap_or_default();
                if failed_attempts >= limits.max_attempts {
                    // Bounded retry exhausted (decision 0006): park with the
                    // last recorded error.
                    let last_error = events
                        .iter()
                        .rev()
                        .find(|event| {
                            event.event_type == EventType::EffectFailed
                                && event.payload.get("effect_id").and_then(Value::as_str)
                                    == Some(effect_id.as_str())
                        })
                        .and_then(|event| event.payload.get("error").and_then(Value::as_str))
                        .unwrap_or("no error recorded")
                        .to_string();
                    self.append(
                        EventType::RunParked,
                        json!({
                            "reason": format!(
                                "effect {effect_id} failed {failed_attempts} of {} attempt(s); \
                                 last error: {last_error}",
                                limits.max_attempts
                            ),
                            "evidence": {},
                        }),
                        None,
                    )?;
                } else {
                    // `execute` is the one place a gate head is armed, for
                    // every body shape, and an attempt that leaves it
                    // through an `Err` — a checkpoint that would not
                    // journal, say — appends no terminal event and so
                    // never reaches the compare. The observation dies with
                    // the span that took it: a stale head here would be
                    // spent on the NEXT effect's terminal event, which
                    // could then read a park no gate earned.
                    let dispatched = self.execute(events, &state, &effect_id, &seat);
                    if dispatched.is_err() {
                        self.active_gate_head = None;
                    }
                    dispatched?
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
                        "attempt_id": attempt_id.clone(),
                        "reason": "engine restarted while the attempt was in flight; \
                                   completion cannot be established",
                    }),
                    Some(attempt_id),
                )?;
            }
            Cursor::Decide { effect_id, result } => self.decide(&state, &effect_id, result)?,
            Cursor::Park { reason } => {
                let evidence = if reason == "GATE-MOVED-HEAD" {
                    gate_head_evidence(events)
                } else {
                    json!({})
                };
                self.append(
                    EventType::RunParked,
                    json!({"reason": reason, "evidence": evidence}),
                    None,
                )?;
            }
            Cursor::Stop => {
                self.append(
                    EventType::RunStopped,
                    json!({"reason": operator_stop_reason(events)}),
                    None,
                )?;
            }
            Cursor::Idle => {
                return Err(EngineError::Other(
                    "running state reached the terminal idle cursor".into(),
                ))
            }
        }
        Ok(())
    }

    fn append(
        &mut self,
        event_type: EventType,
        payload: Value,
        attempt_id: Option<String>,
    ) -> Result<EventEnvelope, EngineError> {
        if matches!(
            event_type,
            EventType::EffectSucceeded | EventType::EffectFailed | EventType::EffectIndeterminate
        ) {
            let effect_id = payload
                .get("effect_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            if let Some(moved) = self.finish_gate_head_check(effect_id, attempt_id.clone())? {
                return Ok(moved);
            }
        }
        self.append_raw(event_type, payload, attempt_id)
    }

    fn finish_gate_head_check(
        &mut self,
        effect_id: &str,
        attempt_id: Option<String>,
    ) -> Result<Option<EventEnvelope>, EngineError> {
        let Some(start) = self.active_gate_head.take() else {
            return Ok(None);
        };
        let end = self.repo.as_deref().and_then(git_head);
        if start == end {
            return Ok(None);
        }
        let evidence = json!({"head_at_start": start, "head_at_end": end});
        // EffectIndeterminate has only a reason string in the frozen event
        // contract. Keep the evidence packed there and attach its structured
        // copy to run/parked rather than widening that contract in this slice.
        self.append_raw(
            EventType::EffectIndeterminate,
            json!({
                "effect_id": effect_id,
                "attempt_id": attempt_id,
                "reason": format!("GATE-MOVED-HEAD {evidence}"),
            }),
            attempt_id,
        )
        .map(Some)
    }

    fn append_raw(
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
                    .unwrap_or("stopped".to_string());
                self.append(EventType::RunStopped, json!({"reason": reason}), None)?;
            } else {
                self.append(EventType::RunCompleted, json!({}), None)?;
            }
            return Ok(());
        }
        let effect_id = Uuid::new_v4().to_string();
        let input = self.seat_input(state, &phase, &effect_id)?;
        let digest = brokkr_core::canonical::sha256_hex(&input);
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
            EngineError::Other(format!(
                "no seat for phase '{phase}' (compile enforces this)"
            ))
        })?;
        let (body, _) = seat
            .body
            .selected(state.strategy.as_deref())
            .ok_or_else(|| {
                EngineError::Other(format!("seat '{phase}' selector has no resolved body"))
            })?;
        let workdir = self.workdir();
        let mut context = Map::new();
        context.insert("run_id".into(), json!(self.run_id));
        // Triage is the fresh-and-blind chief's office (decision 0041
        // ruling 6). Even on the bounded oversized return it receives
        // the commission and current tree, not journal history.
        if phase != "triage" {
            context.insert("last_decision".into(), json!(state.last_decision));
        }
        let current = self
            .bundle
            .machine
            .phases
            .iter()
            .position(|known| known == phase)
            .unwrap_or_default();
        let results: Map<String, Value> = self.bundle.machine.phases[..current]
            .iter()
            .filter_map(|earlier| {
                state
                    .phase_results
                    .get(earlier)
                    .cloned()
                    .map(|result| (earlier.clone(), result))
            })
            .collect();
        if !results.is_empty() {
            context.insert("results".into(), Value::Object(results));
        }
        // Reforging (decision 0022): a seat the run RETURNS to receives
        // the result that sent it back — the review's findings,
        // severities and notes reach the implementer who has to answer
        // them, because a precise finding is only useful to whoever
        // reads it. A seat on its FIRST visit of the run gets nothing
        // new, so a run that never revisits builds the input, and the
        // digest, it always built.
        if phase != "triage" && state.visits.get(phase).copied().unwrap_or(0) > 1 {
            context.insert(
                "returned_from".into(),
                json!({
                    "phase": state.last_decision.as_ref().and_then(|d| d.get("from")),
                    "result": state.last_result,
                }),
            );
        }
        let context = Value::Object(context);
        let has_change = nearest_change(&context).is_some();
        let mut input = match body {
            ExecutableBody::Single { role_path, .. } => json!({
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
            }),
            ExecutableBody::Panel { members, aggregate } => {
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
                json!({
                    "feature": self.feature,
                    "phase": phase,
                    "seat": phase,
                    "aggregate": format!("{aggregate:?}"),
                    "workdir": workdir.to_string_lossy(),
                    "members": Value::Object(member_map),
                    "allowed_results": seat.results,
                    "context": context,
                })
            }
            ExecutableBody::Sequence { steps } => {
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
                            "allowed_results": step.results,
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
                                "allowed_results": step.results,
                                "aggregate": format!("{aggregate:?}"),
                                "members": Value::Object(member_map),
                            })
                        }
                        StepBody::Dialect { execution } => json!({
                            "name": step.name,
                            "allowed_results": step.results,
                            "role_path": "",
                            "result_path": workdir
                                .join(".forge/results")
                                .join(format!("{effect_id}-{}.json", step.name))
                                .to_string_lossy(),
                            "dialect": {"argv": execution.argv, "state": execution.state},
                        }),
                    })
                    .collect();
                json!({
                    "feature": self.feature,
                    "phase": phase,
                    "seat": phase,
                    "workdir": workdir.to_string_lossy(),
                    "steps": step_values,
                    "allowed_results": seat.results,
                    "context": context,
                })
            }
        };
        // The mark is part of the requested input, so the digest covers it;
        // a panel or sequence seat has no hands of its own at this label.
        self.mark_boxed(phase, &mut input);
        // Sealed secret bindings (decision 0012): the engine threads
        // exactly two facts to the driver — the declared NAMES and the
        // store PATH, both journal-safe. Values are resolved at spawn
        // time inside the exec driver; no store read exists anywhere in
        // brokkr-runtime. Absent when the seat binds nothing, so
        // pre-0012 bundles rebuild byte-identical seat inputs.
        if !seat.secrets.is_empty() {
            input["secrets"] = json!(seat.secrets);
            input["secrets_file"] = json!(self.secrets_store_path().to_string_lossy());
        }
        if let (Some(world), Some(repo)) = (&self.world, self.repo.as_deref()) {
            if let Some(house) = world.house_for(repo)? {
                input["house_rules"] = json!(house);
            }
        }
        if phase != "review" && (phase != "implement" || has_change) {
            if let Some(instructions) = self.bundle.dialect_prompts.get(phase) {
                input["spec_dialect"] = json!(instructions);
            }
        }
        Ok(input)
    }

    fn workdir(&self) -> PathBuf {
        self.repo
            .clone()
            .unwrap_or_else(|| std::env::current_dir().expect("cwd"))
    }

    /// The operator-side store path threaded to drivers: the CLI
    /// override, or `<workdir>/.forge/secrets.env`.
    fn secrets_store_path(&self) -> PathBuf {
        self.secrets_file
            .clone()
            .unwrap_or_else(|| self.workdir().join(".forge/secrets.env"))
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
        if brokkr_core::canonical::sha256_hex(&input) != requested_digest {
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

        // The journal is an execution resource, not part of the durable seat
        // request: equivalent --db spellings on resume must rebuild the same
        // input digest. Only the deterministic ship script receives it.
        let mut input = input;
        if phase == "ship" {
            let journal =
                std::fs::canonicalize(self.store.path()).unwrap_or(self.store.path().to_path_buf());
            input["context"]["journal"] = Value::String(journal.to_string_lossy().into_owned());
        }

        let seat = self.bundle.seats[seat_name].clone();
        // `seat_input` above has already refused an unresolved selector
        // against this same folded state.
        let (body, selected_case) = seat
            .body
            .selected(state.strategy.as_deref())
            .expect("seat input resolved the selector");
        let site_name = selected_case
            .map(|case| format!("{seat_name}:{case}"))
            .unwrap_or_else(|| seat_name.to_string());
        let attempt_id = Uuid::new_v4().to_string();
        let driver_label = match body {
            ExecutableBody::Single { command, .. } => command[0].clone(),
            ExecutableBody::Panel { members, aggregate } => {
                format!("panel[{}]:{aggregate:?}", members.len())
            }
            ExecutableBody::Sequence { steps } => format!("sequence[{}]", steps.len()),
        };
        // Which link of each agent's chain runs this attempt, decided
        // from journaled facts before anything spawns. The existing
        // `driver` label is untouched: a display string is not a control
        // channel, and five consumers plus the engine would otherwise
        // have to parse a packed grammar to make a control decision.
        let (selection, provenance) = select_candidates(events, effect_id, body);
        let workdir = self.workdir();
        // The single-driver argv is composed HERE, ahead of the durable
        // start, because the session question is asked of it: which
        // instance this attempt resolves to is part of what decides
        // whether a prior session may be handed back to it.
        let runtime_hands = self.runtime_hands(&site_name);
        let single = match body {
            ExecutableBody::Single {
                command, confine, ..
            } => Some(hands_command(
                confined_command(
                    argv_for(&selection, &None, command),
                    confine,
                    &workdir,
                    &self.bundle.roots,
                ),
                runtime_hands.as_ref(),
                &workdir,
                &self.bundle.roots,
            )),
            _ => None,
        };
        let mut started = json!({
            "effect_id": effect_id,
            "attempt_id": attempt_id,
            "driver": driver_label,
        });
        if let Some(provenance) = provenance {
            started["provenance"] = provenance;
        }
        // Which session — if any — this attempt may rejoin, decided from
        // journaled facts before anything spawns (decision 0030). The
        // `started` payload IS the identity the offer is judged against,
        // which is why the question is asked here, after the driver
        // label and the provenance are in it.
        //
        // The offer itself adds no field to the payload. It is a pure
        // function of this journal and the pinned bundle, so recording
        // it would record a derivation, not a fact — and the fact, what
        // the driver DID with the offer, is the driver's own checkpoint
        // to journal. The engine widens no event vocabulary to say
        // something the record already answers.
        let offer = match single {
            Some(_) => resume_offer(
                events,
                &self.bundle,
                seat_name,
                &started,
                self.store.started_here(&self.run_id)?,
            ),
            None => None,
        };
        // started is durable BEFORE the driver spawns: a crash in between
        // recovers as indeterminate, never as a silent double-execution.
        if arms_effect_gate_head(&body, &seat, state.strategy.as_deref()) {
            self.active_gate_head = Some(self.repo.as_deref().and_then(git_head));
        }
        self.append(EventType::EffectStarted, started, Some(attempt_id.clone()))?;

        std::fs::create_dir_all(workdir.join(".forge/results")).ok();
        let deadline = std::time::Duration::from_secs(seat.limits.timeout_seconds);

        match body {
            ExecutableBody::Panel { members, aggregate } => self.execute_panel(
                effect_id,
                &attempt_id,
                &site_name,
                members,
                aggregate,
                &input,
                deadline,
                &selection,
            ),
            ExecutableBody::Sequence { steps } => self.execute_sequence(
                effect_id,
                &attempt_id,
                &site_name,
                steps,
                &input,
                deadline,
                &selection,
            ),
            // Agents choose the argv (model selection); composition
            // decides what is mounted — a composed bundle spans every
            // recipe directory in its chain, not one dir. Both were
            // settled above, where the session question needed them.
            ExecutableBody::Single { .. } => {
                let command = single.expect("a single seat composed its command");
                let run = self.run_driver(
                    effect_id,
                    &attempt_id,
                    &site_name,
                    &command,
                    input,
                    deadline,
                    None,
                    offer,
                )?;
                self.conclude_single(effect_id, &attempt_id, run, &selection)
            }
        }
    }

    /// A ship seat reads the already-open journal. When that journal is
    /// outside the worktree (the ordinary same-realm fire), mount only its
    /// parent and mount it read-only. This run-time resource is deliberately
    /// absent from the manifest and the requested-input digest.
    /// Decision 0043: a boxed site is told that it is, because the one
    /// tool the box serves is the only thing that can write its result
    /// file — a harness's own shell runs outside the box and a file
    /// written through it never reaches the engine. The first
    /// astra-judged gate wrote its verdict through that shell twice.
    fn mark_boxed(&self, label: &str, input: &mut Value) {
        if self.bundle.hands.contains_key(label) {
            input["hands"] = json!("boxed");
        }
    }

    fn runtime_hands(&self, seat_name: &str) -> Option<brokkr_protocol::hands::HandsSpec> {
        let mut spec = self.bundle.hands.get(seat_name)?.clone();
        let phase = seat_name
            .split_once(':')
            .map_or(seat_name, |(phase, _)| phase);
        if phase == "ship" {
            let workdir = std::fs::canonicalize(self.workdir()).unwrap_or_else(|_| self.workdir());
            let journal =
                std::fs::canonicalize(self.store.path()).unwrap_or(self.store.path().to_path_buf());
            if !journal.starts_with(&workdir) {
                let parent = journal.parent().unwrap_or(&journal);
                spec.binds.push(brokkr_protocol::hands::Bind {
                    path: parent.to_string_lossy().into_owned(),
                    mode: brokkr_protocol::hands::BindMode::Ro,
                    mask: Vec::new(),
                });
            }
        }
        Some(spec)
    }

    /// Conclude a single-driver attempt with its terminal effect event.
    /// The stderr tail rides on failed/indeterminate outcomes; a spawn
    /// failure has no stderr and carries none.
    fn conclude_single(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        run: DriverRun,
        selection: &Selection,
    ) -> Result<(), EngineError> {
        let report = match run {
            DriverRun::SpawnFailed(error) => {
                // A driver binary that is absent satisfies the structural
                // predicate trivially: nothing was accepted and nothing
                // checkpointed, because nothing ran.
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": error,
                });
                start_failure_fields(&mut payload, selection, vec![None]);
                self.append(
                    EventType::EffectFailed,
                    payload,
                    Some(attempt_id.to_string()),
                )?;
                return Ok(());
            }
            DriverRun::Ran(report) => report,
        };
        let start_failure = failed_to_start(&report);
        let stderr_tail = stderr_tail(&report.stderr);
        match report.outcome {
            AttemptOutcome::Succeeded { result } => {
                self.append_succeeded(effect_id, attempt_id, result, |refusal| {
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": format!("{refusal}; stderr tail: {stderr_tail}"),
                    })
                })?;
            }
            AttemptOutcome::Failed { error } => {
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": format!("{error}; stderr tail: {stderr_tail}"),
                });
                if start_failure {
                    start_failure_fields(&mut payload, selection, vec![None]);
                }
                self.append(
                    EventType::EffectFailed,
                    payload,
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
    /// names a sequence step. `session_ref` is the seat's prior session,
    /// offered to a driver that declares it can rejoin one (decision
    /// 0030). Appends NO terminal effect event: the caller owns the
    /// attempt's conclusion.
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
        session_ref: Option<String>,
    ) -> Result<DriverRun, EngineError> {
        let workdir = self.workdir();
        let process = match DriverProcess::spawn(command, &workdir, Some(deadline)) {
            Err(e) => return Ok(DriverRun::SpawnFailed(format!("driver did not spawn: {e}"))),
            Ok(process) => process,
        };
        let mut checkpoint_error: Option<EngineError> = None;
        // A checkpoint the journal refused under the seat-record fence
        // (decision 0034, ruling 6). The driver keeps running — nothing
        // here can stop it, and killing it would only lose its stderr —
        // but the attempt is already lost: no later checkpoint is
        // journaled, and the refusal becomes the attempt's outcome once
        // the process ends.
        let mut refusal: Option<SeatRecordError> = None;
        let store = &mut self.store;
        let current_cause = &mut self.current_cause;
        let run_id = self.run_id.clone();
        let mut report = process.run_attempt_resuming(
            ENGINE_VERSION,
            effect_id,
            attempt_id,
            driver_seat,
            input,
            session_ref,
            |data| {
                if checkpoint_error.is_none() && refusal.is_none() {
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
                        Err(StoreError::SeatRecord(error)) => refusal = Some(error),
                        Err(e) => checkpoint_error = Some(e.into()),
                    }
                }
            },
        );
        if let Some(e) = checkpoint_error {
            return Err(e);
        }
        if let Some(refusal) = refusal {
            report.outcome = refused_outcome(report.outcome, &refusal);
        }
        Ok(DriverRun::Ran(report))
    }

    /// Append the attempt's successful result — or, when the journal
    /// refuses that result under the seat-record fence (decision 0034,
    /// ruling 6), the failure the refusal is. The failed payload is the
    /// caller's to build, because each seat shape carries its own facts
    /// on a failed attempt (a stderr tail, the sites that failed to
    /// start); the closure receives the refusal's text and nothing else.
    fn append_succeeded(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        result: Value,
        failed: impl FnOnce(String) -> Value,
    ) -> Result<(), EngineError> {
        match self.append(
            EventType::EffectSucceeded,
            json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
            Some(attempt_id.to_string()),
        ) {
            Ok(_) => Ok(()),
            Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                self.append(
                    EventType::EffectFailed,
                    failed(refusal.to_string()),
                    Some(attempt_id.to_string()),
                )?;
                Ok(())
            }
            Err(error) => Err(error),
        }
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
        selection: &Selection,
    ) -> Result<(), EngineError> {
        let runs = self.member_runs(
            seat_name,
            members,
            &panel_input["members"],
            panel_input,
            &panel_input["context"],
            selection,
            "",
        );
        let reports = self.run_panel(effect_id, attempt_id, &runs, deadline, "")?;
        self.journal_panel_members(effect_id, attempt_id, &reports, "")?;
        let start_failures = start_failure_sites(&reports, "");
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
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": error,
                });
                // Per member, not per attempt: a member that failed to
                // start advances its OWN chain index, and a member that
                // ran does not.
                start_failure_fields(&mut payload, selection, start_failures);
                self.append(
                    EventType::EffectFailed,
                    payload,
                    Some(attempt_id.to_string()),
                )?;
            }
            AttemptOutcome::Succeeded { result } => {
                // The aggregate is the engine's own record — `result`,
                // `inputs`, `notes` and nothing else, with every member's
                // evidence under `notes` — so it conforms by
                // construction and the seat-record fence has nothing to
                // refuse here. A refusal would be an engine defect, and
                // it propagates as one, like any other storage error;
                // it is not dressed as a member's failure.
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
    #[allow(clippy::too_many_arguments)]
    fn member_runs(
        &self,
        driver_seat_prefix: &str,
        members: &[PanelMember],
        members_meta: &Value,
        seat_input: &Value,
        context: &Value,
        selection: &Selection,
        tag_prefix: &str,
    ) -> Vec<MemberRun> {
        let workdir = self.workdir();
        members
            .iter()
            .map(|member| {
                let site = Some(format!("{tag_prefix}{}", member.name));
                let mut input = json!({
                    "feature": seat_input["feature"],
                    "phase": seat_input["phase"],
                    "seat": format!("{driver_seat_prefix}:{}", member.name),
                    "role_path": members_meta[&member.name]["role_path"],
                    "workdir": seat_input["workdir"],
                    "result_path": members_meta[&member.name]["result_path"],
                    "allowed_results": seat_input["allowed_results"],
                    "house_rules": seat_input["house_rules"],
                    "context": context,
                });
                if seat_input["phase"] == "review" && member.name == "spec-compliance" {
                    input["spec_dialect"] = self
                        .bundle
                        .dialect_prompts
                        .get("review")
                        .map_or(Value::Null, |text| json!(text));
                } else if !seat_input["spec_dialect"].is_null() {
                    input["spec_dialect"] = seat_input["spec_dialect"].clone();
                }
                copy_secret_binding_facts(&mut input, seat_input);
                self.mark_boxed(&format!("{driver_seat_prefix}:{}", member.name), &mut input);
                MemberRun {
                    name: member.name.clone(),
                    driver_seat: format!("{driver_seat_prefix}:{}", member.name),
                    command: hands_command(
                        confined_command(
                            argv_for(selection, &site, &member.command),
                            member.confine.as_ref(),
                            &workdir,
                            &self.bundle.roots,
                        ),
                        self.bundle
                            .hands
                            .get(&format!("{driver_seat_prefix}:{}", member.name)),
                        &workdir,
                        &self.bundle.roots,
                    ),
                    input,
                }
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
        // The member whose checkpoint the fence refused, if one was
        // (decision 0034, ruling 6): the same latch a single driver
        // carries, keyed by the tagged member name the checkpoint rode
        // under, so the refusal lands on that member's report alone.
        let mut refusal: Option<(String, SeatRecordError)> = None;
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
                        let report =
                            match DriverProcess::spawn(&run.command, &workdir, Some(deadline)) {
                                Err(e) => AttemptReport {
                                    outcome: AttemptOutcome::Failed {
                                        error: format!("member driver did not spawn: {e}"),
                                    },
                                    session_ref: None,
                                    checkpoints: Vec::new(),
                                    stderr: String::new(),
                                    // Nothing ran, so nothing was
                                    // accepted: the structural
                                    // fail-to-start predicate holds.
                                    accepted: false,
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
                                        let _ =
                                            sender.send((checkpoint_name.clone(), data.clone()));
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
                if checkpoint_error.is_some() || refusal.is_some() {
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
                    Err(StoreError::SeatRecord(error)) => refusal = Some((member, error)),
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
        let Some((refused, refusal)) = refusal else {
            return Ok(reports);
        };
        Ok(reports
            .into_iter()
            .map(|(name, mut report)| {
                if format!("{tag_prefix}{name}") == refused {
                    report.outcome = refused_outcome(report.outcome, &refusal);
                }
                (name, report)
            })
            .collect())
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
            let model = match &report.outcome {
                AttemptOutcome::Succeeded { result } => result
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                _ => None,
            }
            .or_else(|| {
                report
                    .checkpoints
                    .iter()
                    .rev()
                    .find_map(|checkpoint| checkpoint.get("model").and_then(Value::as_str))
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "not reported".to_string());
            self.append(
                EventType::EffectCheckpointed,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "checkpoint": {
                        "step": "panel-member-finished",
                        "member": format!("{tag_prefix}{name}"),
                        "outcome": kind,
                        "model": model,
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
    #[allow(clippy::too_many_arguments)]
    fn execute_sequence(
        &mut self,
        effect_id: &str,
        attempt_id: &str,
        seat_name: &str,
        steps: &[SequenceStep],
        seq_input: &Value,
        deadline: std::time::Duration,
        selection: &Selection,
    ) -> Result<(), EngineError> {
        let mut prior_results = Map::new();
        let mut nearest_change = nearest_change(&seq_input["context"]);
        let declares_change = self
            .bundle
            .seats
            .get(seat_name)
            .is_some_and(|seat| seat.inputs.iter().any(|input| input == "change"));
        let mut deterministic_failure: Option<(String, String)> = None;
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
            // Which sites of THIS step failed to start, if the step is
            // the one that fails the attempt.
            let mut start_failures: Vec<Site> = Vec::new();
            // The gate span inside a sequence is THIS step: armed here,
            // compared and cleared at this step's own end below, before
            // any later step gets to move the tree lawfully (decision
            // 0042 reads an author as a work step). Nothing outer arms
            // for a sequence, so this is the only observation taken.
            if step.class == SeatClass::Gate {
                self.active_gate_head = Some(self.repo.as_deref().and_then(git_head));
            }
            let outcome = match &step.body {
                StepBody::Single {
                    command, confine, ..
                } => {
                    let site = Some(step.name.clone());
                    let driver_seat = format!("{seat_name}:{}", step.name);
                    let step_label = driver_seat.clone();
                    let mut input = json!({
                        "feature": seq_input["feature"],
                        "phase": seq_input["phase"],
                        "seat": driver_seat,
                        "role_path": step_meta["role_path"],
                        "workdir": seq_input["workdir"],
                        "result_path": step_meta["result_path"],
                        "allowed_results": if index + 1 == steps.len() {
                            &seq_input["allowed_results"]
                        } else {
                            &step_meta["allowed_results"]
                        },
                        "house_rules": seq_input["house_rules"],
                        "context": context,
                    });
                    if !seq_input["spec_dialect"].is_null() {
                        input["spec_dialect"] = seq_input["spec_dialect"].clone();
                    }
                    copy_secret_binding_facts(&mut input, seq_input);
                    self.mark_boxed(&step_label, &mut input);
                    let command = hands_command(
                        confined_command(
                            argv_for(selection, &site, command),
                            confine.as_ref(),
                            &self.workdir(),
                            &self.bundle.roots,
                        ),
                        self.bundle.hands.get(&format!("{seat_name}:{}", step.name)),
                        &self.workdir(),
                        &self.bundle.roots,
                    );
                    // A sequence step is not a seat: decision 0030 hands
                    // a session back to the same SEAT of the same run,
                    // and a step's session has no such identity to be
                    // matched by. Steps start cold, as they always did.
                    match self.run_driver(
                        effect_id,
                        attempt_id,
                        &driver_seat,
                        &command,
                        input,
                        deadline,
                        Some(&step.name),
                        None,
                    )? {
                        DriverRun::SpawnFailed(error) => {
                            start_failures.push(site);
                            AttemptOutcome::Failed { error }
                        }
                        DriverRun::Ran(report) => {
                            // The step driver's stderr tail rides on the
                            // attempt's terminal event, as a single
                            // seat's does.
                            let tail = stderr_tail(&report.stderr);
                            if failed_to_start(&report) {
                                start_failures.push(site);
                            }
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
                    let mut step_input = seq_input.clone();
                    step_input["allowed_results"] = if index + 1 == steps.len() {
                        seq_input["allowed_results"].clone()
                    } else {
                        step_meta["allowed_results"].clone()
                    };
                    let runs = self.member_runs(
                        &format!("{seat_name}:{}", step.name),
                        members,
                        &step_meta["members"],
                        &step_input,
                        &context,
                        selection,
                        &tag_prefix,
                    );
                    let reports =
                        self.run_panel(effect_id, attempt_id, &runs, deadline, &tag_prefix)?;
                    self.journal_panel_members(effect_id, attempt_id, &reports, &tag_prefix)?;
                    start_failures = start_failure_sites(&reports, &tag_prefix);
                    panel_outcome(*aggregate, reports)
                }
                StepBody::Dialect { execution } => {
                    let needs_change = execution
                        .argv
                        .iter()
                        .any(|token| token.contains("{change}"))
                        || execution
                            .state
                            .iter()
                            .flatten()
                            .any(|token| token.contains("{change}"));
                    if needs_change && nearest_change.is_none() {
                        return self.append(
                            EventType::EffectIndeterminate,
                            json!({
                                "effect_id": effect_id,
                                "attempt_id": attempt_id,
                                "reason": format!(
                                    "sequence step '{}': cannot expand {{change}} because no preceding successful result carries it",
                                    step.name
                                ),
                            }),
                            Some(attempt_id.to_string()),
                        ).map(|_| ());
                    }
                    let change = nearest_change.as_deref().unwrap_or("");
                    let argv = expand_dialect_argv(&execution.argv, change);
                    let command = std::iter::once(
                        std::env::current_exe()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    )
                    .chain(["driver", "exec", "--"].into_iter().map(str::to_string))
                    .chain(argv)
                    .collect::<Vec<_>>();
                    let site = Some(step.name.clone());
                    let driver_seat = format!("{seat_name}:{}", step.name);
                    let step_label = driver_seat.clone();
                    let mut input = json!({
                        "feature": seq_input["feature"],
                        "phase": seq_input["phase"],
                        "seat": driver_seat,
                        "role_path": "",
                        "workdir": seq_input["workdir"],
                        "result_path": step_meta["result_path"],
                        "allowed_results": if index + 1 == steps.len() {
                            &seq_input["allowed_results"]
                        } else {
                            &step_meta["allowed_results"]
                        },
                        "house_rules": seq_input["house_rules"],
                        "context": context,
                        "dialect_exec": {
                            "success_result": dialect_results(seq_input["phase"].as_str().unwrap_or(""))[0],
                            "failure_result": dialect_results(seq_input["phase"].as_str().unwrap_or(""))[1],
                            "state": execution.state.as_ref().map(|state| expand_dialect_argv(state, change)),
                            "change": if change.is_empty() { Value::Null } else { Value::String(change.to_string()) },
                        },
                    });
                    copy_secret_binding_facts(&mut input, seq_input);
                    self.mark_boxed(&step_label, &mut input);
                    let command = hands_command(
                        argv_for(selection, &site, &command).to_vec(),
                        self.bundle.hands.get(&format!("{seat_name}:{}", step.name)),
                        &self.workdir(),
                        &self.bundle.roots,
                    );
                    dialect_attempt_outcome(self.run_driver(
                        effect_id,
                        attempt_id,
                        &driver_seat,
                        &command,
                        input,
                        deadline,
                        Some(&step.name),
                        None,
                    )?)
                }
            };
            if self
                .finish_gate_head_check(effect_id, Some(attempt_id.to_string()))?
                .is_some()
            {
                return Ok(());
            }
            let result = match outcome {
                AttemptOutcome::Succeeded { result } => result,
                AttemptOutcome::Failed { error } => {
                    let mut payload = json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "error": format!("sequence step '{}': {error}", step.name),
                    });
                    start_failure_fields(&mut payload, selection, start_failures);
                    self.append(
                        EventType::EffectFailed,
                        payload,
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
            if index + 1 != steps.len() {
                let vocabulary_problem = match result.get("result").and_then(Value::as_str) {
                    None => Some("has no 'result' string".to_string()),
                    Some(word) if !step.results.iter().any(|allowed| allowed == word) => {
                        Some(format!(
                            "reported '{word}', outside its declared results {:?}",
                            step.results
                        ))
                    }
                    Some(_) => None,
                };
                if let Some(problem) = vocabulary_problem {
                    self.append(
                        EventType::EffectFailed,
                        json!({
                            "effect_id": effect_id,
                            "attempt_id": attempt_id,
                            "error": format!("sequence step '{}': {problem}", step.name),
                        }),
                        Some(attempt_id.to_string()),
                    )?;
                    return Ok(());
                }
                // A seat result which no remaining compiled step can emit is
                // a declared sequence-ending boundary. The comparison is
                // against the remaining steps' actual vocabularies, not the
                // enclosing seat vocabulary inherited by the old final-step
                // representation.
                let ends_sequence =
                    result
                        .get("result")
                        .and_then(Value::as_str)
                        .is_some_and(|word| {
                            let seat_declares = seq_input["allowed_results"]
                                .as_array()
                                .is_some_and(|allowed| allowed.iter().any(|value| value == word));
                            let later_can_emit = steps[index + 1..]
                                .iter()
                                .any(|later| later.results.iter().any(|allowed| allowed == word));
                            seat_declares && !later_can_emit
                        });
                if ends_sequence {
                    return self.append(
                        EventType::EffectSucceeded,
                        json!({"effect_id": effect_id, "attempt_id": attempt_id, "result": result}),
                        Some(attempt_id.to_string()),
                    ).map(drop);
                }
                let phase = seq_input["phase"].as_str().unwrap_or("");
                if matches!(step.body, StepBody::Dialect { .. })
                    && matches!(phase, "clarify" | "analyze")
                    && result["result"] == dialect_results(phase)[1]
                {
                    deterministic_failure = Some((
                        step.name.clone(),
                        result["result"].as_str().unwrap().to_string(),
                    ));
                }
            }
            let model = result
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or("not reported")
                .to_string();
            // A step's result the fence refuses (decision 0034, ruling
            // 6) fails the attempt at that step, with the same facts a
            // step that failed on its own would carry.
            let refused = |refusal: String| {
                let mut payload = json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "error": format!("sequence step '{}': {refusal}", step.name),
                });
                start_failure_fields(&mut payload, selection, start_failures);
                payload
            };
            if declares_change {
                if let Some(value) = result.pointer("/inputs/change") {
                    let change = value.as_str();
                    if !change.is_some_and(brokkr_core::policy::is_identifier) {
                        self.append(
                            EventType::EffectIndeterminate,
                            json!({
                                "effect_id": effect_id,
                                "attempt_id": attempt_id,
                                "reason": format!(
                                    "sequence step '{}': declared input 'change' must match ^[a-z0-9][a-z0-9._-]*$, got {}",
                                    step.name,
                                    value
                                ),
                            }),
                            Some(attempt_id.to_string()),
                        )?;
                        return Ok(());
                    }
                    nearest_change = change.map(str::to_string);
                }
            }
            if index + 1 == steps.len() {
                let phase = seq_input["phase"].as_str().unwrap_or("");
                if let Some((check, failure)) = &deterministic_failure {
                    if result["result"] == dialect_results(phase)[0] {
                        return self.append(
                            EventType::EffectIndeterminate,
                            json!({
                                "effect_id": effect_id,
                                "attempt_id": attempt_id,
                                "reason": format!(
                                    "sequence step '{}': result '{}' contradicts deterministic step '{}' result '{}'",
                                    step.name, result["result"], check, failure
                                ),
                            }),
                            Some(attempt_id.to_string()),
                        ).map(drop);
                    }
                }
                self.append_succeeded(effect_id, attempt_id, result, refused)?;
            } else {
                match self.append(
                    EventType::EffectCheckpointed,
                    json!({
                        "effect_id": effect_id,
                        "attempt_id": attempt_id,
                        "checkpoint": {
                            "step": "sequence-step-finished",
                            "step_name": step.name,
                            "model": model,
                            "result": result,
                        },
                    }),
                    Some(attempt_id.to_string()),
                ) {
                    Ok(_) => {}
                    Err(EngineError::Store(StoreError::SeatRecord(refusal))) => {
                        self.append(
                            EventType::EffectFailed,
                            refused(refusal.to_string()),
                            Some(attempt_id.to_string()),
                        )?;
                        return Ok(());
                    }
                    Err(error) => return Err(error),
                }
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
                Some(_) => object
                    .get("inputs")
                    .and_then(Value::as_object)
                    .and_then(|inputs| inputs.get("change"))
                    .filter(|_| seat.inputs.iter().any(|input| input == "change"))
                    .and_then(|value| match value.as_str() {
                        Some(change) if brokkr_core::policy::is_identifier(change) => None,
                        _ => Some(format!(
                            "declared input 'change' must match ^[a-z0-9][a-z0-9._-]*$, got {value}"
                        )),
                    }),
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
        let result = object["result"]
            .as_str()
            .expect("checked above")
            .to_string();

        // Seat-supplied facts: everything the engine owns is dropped, and
        // only the seat's DECLARED inputs survive (decision 0007) — an
        // undeclared claim never reaches the table or the journal record.
        let mut inputs: Map<String, Value> = object
            .get("inputs")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        inputs.retain(|key, _| !crate::bundle::is_engine_owned(key));
        inputs.retain(|key, _| seat.inputs.iter().any(|declared| declared == key));
        // Journal-computed inputs overlay (never accepted from the seat).
        for (key, value) in computed_inputs(state, &phase, &result) {
            inputs.insert(key, value);
        }
        if result == "fail"
            && self
                .bundle
                .machine
                .reads_counter(&phase, "consecutive_failures")
        {
            let prior = state.consecutive_failures.get(&phase).copied().unwrap_or(0);
            inputs.insert("consecutive_failures".into(), Value::from(prior + 1));
        }
        // The phase-visit predicate (decision 0022), supplied for exactly
        // the phases this phase's rules ask about — counted from
        // `phase/entered`, never claimed by a seat.
        for visited in self.bundle.machine.visit_phases(&phase) {
            let count = state.visits.get(&visited).copied().unwrap_or(0);
            inputs.insert(format!("{VISIT_PREFIX}{visited}"), Value::from(count));
        }
        if let Some(repo) = &self.repo {
            // The realm this repository IS, when a map named it
            // (decision 0023): repository facts are recorded under the
            // realm's name — the shape the heritage protocol recorded and
            // the shape multi-realm runs will need. Unmapped, they are
            // recorded exactly as they always were.
            let realm = self
                .world
                .as_ref()
                .and_then(|world| world.realm_for(repo))
                .map(|realm| realm.name.clone());
            let key = realm
                .clone()
                .unwrap_or_else(|| LEGACY_REALM_KEY.to_string());
            if phase == self.bundle.protected_phase {
                if let Some(head) = git_head(repo) {
                    inputs.insert("reviewed_heads".into(), json!({ key: &head }));
                    if let Some(docs_only) = self.fixes_docs_only(repo, &phase, &head) {
                        inputs.insert("fixes_docs_only".into(), Value::Bool(docs_only));
                    }
                }
            }
            // Decision 0041 ruling 5: the smith owns review findings. Expose
            // the docs-only shortcut only when review sent this implement
            // visit back; verify failures and implement self-loops must still
            // pass through verify even when their delta happens to be prose.
            let returned_from_review = phase == "implement"
                && state
                    .last_decision
                    .as_ref()
                    .and_then(|decision| decision.get("from"))
                    .and_then(Value::as_str)
                    == Some(self.bundle.protected_phase.as_str());
            if returned_from_review {
                if let Some(head) = git_head(repo) {
                    if let Some(docs_only) = self.fixes_docs_only(repo, &phase, &head) {
                        inputs.insert("fixes_docs_only".into(), Value::Bool(docs_only));
                    }
                }
            }
            if phase == "ship" {
                let dirty = git_dirty(repo);
                let head = git_head(repo);
                inputs.insert("dirty_worktrees".into(), Value::Bool(dirty));
                // Fail-closed: when the protected phase RECORDED heads,
                // ship always answers the drift question. A repo that
                // no longer resolves to a recorded realm, or a realm
                // whose head was never recorded, is indistinct from
                // drift — silence here shipped where the old code
                // re-armed review (this run's own review caught it).
                let drifted = state.reviewed_heads.as_ref().map(|recorded| {
                    match recorded_head(recorded, realm.as_deref()) {
                        Some(reviewed) => head.as_deref() != Some(reviewed),
                        None => true,
                    }
                });
                if let Some(drifted) = drifted {
                    inputs.insert("drift_detected".into(), Value::Bool(drifted));
                }
                // The same facts, keyed by realm — one realm today,
                // several when multi-realm runs arrive. Recorded only in
                // a mapped world, so an unmapped run's decision payload
                // is byte-for-byte the one it always wrote.
                if let Some(realm) = &realm {
                    let mut facts = Map::new();
                    if let Some(head) = &head {
                        facts.insert("head".into(), Value::from(head.clone()));
                    }
                    facts.insert("dirty_worktrees".into(), Value::Bool(dirty));
                    if let Some(drifted) = drifted {
                        facts.insert("drift_detected".into(), Value::Bool(drifted));
                    }
                    inputs.insert(
                        REALM_FACTS.into(),
                        json!({ realm.clone(): Value::Object(facts) }),
                    );
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
            // A rule-driven park (decision 0022). The v1 event vocabulary
            // already spells it: a decision with a rule_id and no next is
            // exactly "a rule matched and no transition was taken" — the
            // shape the artifact gate writes. `severity` stays null
            // because severity is a property of a taken transition.
            Outcome::Park { rule_id, reason } => json!({
                "from": phase,
                "result": result,
                "rule_id": rule_id,
                "next": null,
                "severity": null,
                "inputs": inputs,
                "problem": if phase == "triage" && result == "escalate" {
                    object.get("notes").and_then(Value::as_str)
                        .filter(|notes| !notes.is_empty())
                        .unwrap_or(&reason)
                } else {
                    &reason
                },
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

/// The rule id the engine cites when the operator's command — not the
/// policy table — is what takes a run to stop. All-caps hyphenated, the
/// same vocabulary the table's own rule ids use (`WORK`, `SHIP-COMPLETE`),
/// so a reader of `run/stopped` can tell the two causes apart at a glance
/// and grep for either.
pub const OPERATOR_STOP_RULE: &str = "OPERATOR-STOP";

/// The conclusion an accepted operator stop is journaled with: the rule
/// id above, the command it names, and the operator who gave it with the
/// reason they recorded. `run/stopped`'s v1 payload is closed at
/// `{reason}` (`contracts/README.md`, additionalProperties false), so the
/// citation lives INSIDE the reason string — no new field, no second
/// vocabulary — exactly as `request_or_finish`'s policy-driven hard stop
/// cites its `rule_id` there. The cause is read back from the journal
/// that `operator_command` wrote: `fold` spends the pending command when
/// it accepts it, so the events are the only place it survives.
fn operator_stop_reason(events: &[EventEnvelope]) -> String {
    let accepted = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::OperatorAccepted);
    let accepted_field = |field: &str| {
        accepted
            .and_then(|event| event.payload.get(field))
            .and_then(Value::as_str)
    };
    // The acceptance carries only the command's id; the command itself
    // is on the `operator/commanded` it disposes of.
    let command_id = accepted_field("command_id").unwrap_or("unrecorded");
    let command = events
        .iter()
        .find(|event| {
            event.event_type == EventType::OperatorCommanded
                && event.payload.get("command_id").and_then(Value::as_str) == Some(command_id)
        })
        .and_then(|event| event.payload.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("stop");
    format!(
        "{OPERATOR_STOP_RULE}: operator '{}' commanded {command} ({command_id}): {}",
        accepted_field("operator").unwrap_or("unrecorded"),
        accepted_field("reason").unwrap_or("no reason recorded"),
    )
}

/// The reason a command is refused because the run MOVED out from under
/// it — it was legal when the operator asked and illegal by the time the
/// disposition was written. A race, and the journal says so in one word.
///
/// The reasons below name the other thing a refusal can mean: a command
/// the run could never have taken, no race involved. `lost_fence` used to
/// carry both, which told an operator reading a refused `retry` that they
/// had been unlucky when in fact they had asked for something the run was
/// never in a state to give.
pub const LOST_FENCE: &str = "lost_fence";

/// Not `retry` or `stop`. The same word the fenced path uses.
const COMMAND_NOT_ALLOWED: &str = "command_not_allowed";

/// The run had already reached `Completed`/`Stopped`. Named for the
/// `FoldError` an acceptance there would mint, since that error is what
/// the refusal exists to prevent.
const AFTER_TERMINAL: &str = "after_terminal";

/// `retry` asked of a run that is not parked. The same word the fenced
/// path uses for the same condition.
const RUN_NOT_AWAITING_OPERATOR: &str = "run_not_awaiting_operator";

/// The caller's cursor no longer describes the run's head. The fenced
/// path's word for a race, which is `lost_fence`'s counterpart on the
/// side that HAS a cursor to be stale.
const STALE_CURSOR: &str = "stale_cursor";

/// How many times [`operator_command`] re-decides against a moving head
/// before it refuses. Each turn is spent only when a peer appended in the
/// microseconds between the deciding fold and the fenced write, and costs
/// one load and one fold. A command that loses four in a row is not
/// racing a burn, and a refusal is always the safe answer — `fold` reads
/// `operator/rejected` back in every state there is.
const FENCE_ATTEMPTS: usize = 4;

/// Would an `operator/accepted` for `command` still fold against this
/// state — and if not, WHICH condition stops it? Asked BEFORE the
/// acceptance is written, because `fold` will ask it of every reader
/// forever afterward and events are immutable — an acceptance that lands
/// where fold refuses it is not a mistake that can be taken back, it is a
/// journal that stops folding from that seq on.
///
/// The condition is returned rather than a verdict because the same
/// condition means two different things depending on WHEN it first held:
/// true already when the operator asked, it is an illegal request; true
/// only once the command had landed, it is [`LOST_FENCE`], a run that
/// moved. [`operator_command`] asks twice and names the refusal
/// accordingly.
///
/// The rule is exactly `fold`'s (`brokkr-core::fold`), read from the
/// other side:
/// - A run that has gone `Completed`/`Stopped` exempts only
///   `operator/commanded` and `operator/rejected`; an acceptance there is
///   `FoldError::AfterTerminal` forever.
/// - `"retry"` moves a run from parked back to running, so it needs the
///   run still parked, and a phase to return to.
/// - `"stop"` is a live kill switch and deliberately lands wherever the
///   run stands, so anywhere non-terminal is legal for it.
///
/// [`apply_fenced_operator_command`] does not call this: its own
/// `run_not_awaiting_operator` check is strictly stronger (a parked run
/// is non-terminal and has a phase), so it is already inside this rule.
/// The unfenced path cannot borrow that stronger check, because
/// demanding a parked run would break `"stop"`'s whole purpose — the
/// fence it needs is the narrower one this predicate states.
fn refusal_for(state: &RunState, command: &str) -> Option<&'static str> {
    if command != "retry" && command != "stop" {
        return Some(COMMAND_NOT_ALLOWED);
    }
    if matches!(state.status, Status::Completed | Status::Stopped) {
        return Some(AFTER_TERMINAL);
    }
    // No phase check beside the status: `fold` refuses a park outside a
    // phase (`RunParked` at `Start` is out of place), so a run awaiting
    // an operator always has somewhere for a retry to re-enter.
    if command == "retry" && state.status != Status::AwaitingOperator {
        return Some(RUN_NOT_AWAITING_OPERATOR);
    }
    None
}

/// Journal a refusal and report it. A rejection needs no fence of its
/// own: `fold` reads `operator/rejected` back in every state there is,
/// terminal included, which is exactly why refusing is always the safe
/// answer to a race.
fn refuse(
    store: &mut Store,
    run_id: &str,
    command_id: &str,
    operator: &str,
    reason: &str,
    cause: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    store.append_next(
        run_id,
        EventType::OperatorRejected,
        json!({"command_id": command_id, "operator": operator, "reason": reason}),
        Some(cause.to_string()),
        None,
    )?;
    let (head_seq, head_hash) = store.head_hash(run_id)?;
    Ok(FencedCommandOutcome::Rejected {
        reason: reason.into(),
        head_seq,
        head_hash,
    })
}

/// Append an operator command and its disposition (the CLI is the
/// operator's console; approval is a signed journal entry, not prose).
///
/// Unfenced in the sense that the caller supplies no cursor — an
/// operator at a terminal has not read a head hash — but not unfenced
/// against the run. An engine process driving this same run is appending
/// concurrently, and between the operator's decision and this write it
/// can conclude the run or un-park it. The old code appended
/// `operator/accepted` unconditionally into that window, which on a run
/// that had gone terminal wrote a journal that no longer folds: silent
/// at write time, irreversible afterward, and surfacing only the next
/// time anyone read the run.
///
/// So the state is re-established here, as close to the write as the
/// store API allows — which, with [`Store::append_next_if_head`], is all
/// the way. `operator/commanded` is appended first (fold exempts it even
/// after a terminal, so it is safe anywhere and it records that the
/// operator asked). Then the run is folded again, and what THAT fold says
/// — not what the operator saw — decides the disposition. An acceptance
/// is written with the head that fold read as its fence, so it can only
/// land on the state it was decided against: a peer that appended in
/// between takes the head away, nothing is written, and the decision is
/// made again against what the journal now says. Decide-and-append is
/// atomic, not merely narrow, and no acceptance this function writes can
/// be one `fold` later refuses.
///
/// A refusal is written unfenced, because `fold` reads
/// `operator/rejected` back in every state there is.
pub fn operator_command(
    store: &mut Store,
    run_id: &str,
    command: &str,
    operator: &str,
    reason: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    operator_command_racing(store, run_id, command, operator, reason, |_| {})
}

/// [`operator_command`], with the window it fences made reachable.
///
/// `between` is called at the one instant the fence exists for: after the
/// deciding fold has read the journal, before the disposition is written.
/// Production passes a no-op. The tests pass an engine's append, because
/// a race proved by two real threads could only be asserted on when it
/// happened to interleave, and a fence is either always there or it is
/// not a fence.
fn operator_command_racing(
    store: &mut Store,
    run_id: &str,
    command: &str,
    operator: &str,
    reason: &str,
    mut between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    let command_id = Uuid::new_v4().to_string();
    let events = store.load(run_id)?;
    // A journal that does not fold cannot host a legal acceptance at
    // all, so refuse before writing anything rather than adding to it.
    let asked = fold(&events)?;
    // What the run already refused before the command was even journaled
    // is not a race, and is not reported as one.
    let illegal_when_asked = refusal_for(&asked, command);
    let head_event = events.last().map(|e| e.event_id.clone());
    let commanded = store.append_next(
        run_id,
        EventType::OperatorCommanded,
        json!({"command_id": command_id, "command": command, "args": {}, "operator": operator}),
        head_event,
        None,
    )?;

    let mut lost = 0;
    // One line per refusal callsite: the exact-coverage gate reads a
    // multi-line call's `?` edges as their own regions, and a refusal
    // whose failure edge cannot be reached deterministically would read
    // as an uncovered line forever. Sharing the line makes the gate see
    // what is actually exercised.
    let refusal_of = |store: &mut Store, why: &'static str, commanded: &EventEnvelope| {
        refuse(
            store,
            run_id,
            &command_id,
            operator,
            why,
            &commanded.event_id,
        )
    };
    let disposition = loop {
        let events = store.load(run_id)?;
        let state = fold(&events)?;
        let (head_seq, head_hash) = events
            .last()
            .map(|event| (event.seq, event.event_hash.clone()))
            .expect("the command just appended is in the journal");
        between(store);
        if let Some(condition) = refusal_for(&state, command) {
            let refusal = if illegal_when_asked.is_some() {
                condition
            } else {
                LOST_FENCE
            };
            break refusal_of(store, refusal, &commanded)?;
        }
        // An acceptance disposes of the command still pending, and `fold`
        // reads it back only as disposing of THAT one. A second operator
        // commanding in this window takes the pending place, and an
        // acceptance written into it is `NoMatchingCommand` for every
        // reader afterwards — the same irreversible shape as an
        // acceptance after a terminal, arriving from a peer at another
        // terminal rather than from an engine.
        if !matches!(&state.pending_command, Some((pending, _)) if *pending == command_id) {
            break refusal_of(store, LOST_FENCE, &commanded)?;
        }
        match store.append_next_if_head(
            run_id,
            head_seq,
            &head_hash,
            EventType::OperatorAccepted,
            json!({"command_id": command_id, "operator": operator, "reason": reason}),
            Some(commanded.event_id.clone()),
            None,
        ) {
            Ok(_) => {
                let (head_seq, head_hash) = store.head_hash(run_id)?;
                break FencedCommandOutcome::Accepted {
                    head_seq,
                    head_hash,
                };
            }
            // A peer appended between the fold and the write. The
            // acceptance was never written; decide again against what it
            // wrote, and refuse rather than spin forever.
            Err(brokkr_store::StoreError::HeadMoved { .. }) if lost < FENCE_ATTEMPTS => {
                lost += 1;
            }
            Err(brokkr_store::StoreError::HeadMoved { .. }) => {
                break refusal_of(store, LOST_FENCE, &commanded)?;
            }
            Err(error) => return Err(error.into()),
        }
    };
    // The pair that just landed must read back. Same proof the fenced
    // path takes before it acknowledges anything to Looper.
    fold(&store.load(run_id)?)?;
    Ok(disposition)
}

/// What `brokkr operator supersede` says, as the operator typed it
/// (decision 0047 ruling 1): which of this run's residual findings are
/// closed, the run and ruling that closed them, and why.
pub struct Supersede<'a> {
    /// The sequence numbers of the rulings whose residuals are closed.
    pub findings: &'a [u64],
    /// The realm the superseding run was read in, or `None` for the
    /// workspace journal — decision 0026 ruling 3's key, on a citation.
    pub by_realm: Option<&'a str>,
    pub by_run: &'a str,
    pub by_seq: u64,
    pub reason: &'a str,
    pub operator: &'a str,
}

/// Record that a terminal run's residual findings are closed by another
/// run (decision 0047 ruling 1). One `operator/commanded` event with
/// `command: "supersede"`, and nothing else: no `operator/accepted`
/// follows, because there is nothing to execute and `fold` refuses an
/// acceptance on a terminal run by design. The event is the record.
///
/// Every citation is verified BEFORE anything is written (ruling 2), and
/// a refusal writes nothing at all — which is why this returns
/// [`EngineError::SupersedeRefused`] rather than journaling a rejection
/// the way `retry` and `stop` do. There is no pending command here for a
/// rejection to dispose of.
///
/// `by_journal` is the journal `by_realm` names, opened by the caller
/// (the workspace journal when the citation names no realm): a
/// superseding run may live in another hearth, and a citation into a
/// journal this process never opened is one nobody can follow back.
///
/// The write is FENCED on the head the verification read (decision
/// 0029). The event is legal wherever it lands — a terminal status is
/// absorbing, so `fold` exempts this annotation at any later seq — but
/// a head that moved is a journal that grew under a verification, and
/// re-deciding beats writing against a reading that is no longer the
/// whole story.
pub fn operator_supersede(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    ask: &Supersede,
) -> Result<EventEnvelope, EngineError> {
    operator_supersede_racing(store, by_journal, run_id, ask, |_| {})
}

/// [`operator_supersede`], with the window its fence exists for made
/// reachable. `between` runs after the verifying load, before the
/// fenced append. Production passes a no-op; the test passes a peer's
/// append, because a fence is either always there or it is not a fence.
fn operator_supersede_racing(
    store: &mut Store,
    by_journal: &Store,
    run_id: &str,
    ask: &Supersede,
    mut between: impl FnMut(&mut Store),
) -> Result<EventEnvelope, EngineError> {
    // One voice for every refusal below, and every one of them comes
    // before the first append: a refused supersede leaves the journal
    // byte for byte as it found it.
    let refused = EngineError::SupersedeRefused;
    let events = store.load(run_id)?;
    let state = fold(&events)?;
    if !matches!(state.status, Status::Completed | Status::Stopped) {
        return Err(refused(format!(
            "run '{run_id}' is {}, not completed or stopped; on a run that is still \
             going the fold would hold this as a PENDING command and the \
             operator/accepted that disposes of it would be refused as unknown",
            brokkr_view::status_str(&state.status)
        )));
    }
    if ask.by_run == run_id {
        return Err(refused(format!(
            "run '{run_id}' cannot supersede its own findings; name the run that \
             closed them"
        )));
    }
    if ask.findings.is_empty() {
        return Err(refused(format!(
            "a supersede on run '{run_id}' that names no finding closes nothing; \
             name the seq of every ruling whose residual is closed"
        )));
    }
    let derived = brokkr_view::residual_findings(run_id, &events);
    for finding in ask.findings {
        if !derived.iter().any(|known| known.seq == *finding) {
            return Err(refused(format!(
                "seq {finding} is not a residual finding of run '{run_id}'; \
                 brokkr inspect --run {run_id} lists the rulings that carry one"
            )));
        }
    }
    let realm = match ask.by_realm {
        Some(realm) => format!("realm '{realm}'"),
        None => "the workspace journal".to_string(),
    };
    let cited = by_journal
        .load(ask.by_run)
        .map_err(|error| refused(format!("run '{}' in {realm}: {error}", ask.by_run)))?;
    if !cited
        .iter()
        .any(|event| event.seq == ask.by_seq && event.event_type == EventType::TransitionDecided)
    {
        return Err(refused(format!(
            "seq {} of run '{}' in {realm} is not a transition/decided; a supersede \
             cites the RULING that closed the finding",
            ask.by_seq, ask.by_run
        )));
    }
    let head = events
        .last()
        .expect("a run that folded to a terminal status has events");
    let payload = json!({
        "command_id": Uuid::new_v4().to_string(),
        "command": brokkr_view::SUPERSEDE,
        "operator": ask.operator,
        "args": {
            "findings": ask.findings,
            "by": {"realm": ask.by_realm, "run_id": ask.by_run, "seq": ask.by_seq},
            "reason": ask.reason,
        },
    });
    between(store);
    // A head that moved comes back in the store's own words — `head
    // moved: expected seq N, found seq M` — rather than paraphrased
    // into a refusal of this verb's own. Nothing was written either
    // way, and the operator reads the journal again and asks again.
    let written = store.append_next_if_head(
        run_id,
        head.seq,
        &head.event_hash,
        EventType::OperatorCommanded,
        payload,
        Some(head.event_id.clone()),
        None,
    )?;
    // The annotation must read back, and reading back must change
    // nothing (ruling 3). Same proof the fenced command path takes.
    fold(&store.load(run_id)?)?;
    Ok(written)
}

/// Close a run from its journal alone: no bundle, no recipe, no process.
///
/// `resume` is bundle-in, bundle-out — it compiles the exact pinned
/// recipe and refuses on any drift (`ManifestMismatch`) before it looks
/// at the cursor, because the branches it drives (`RequestEffect`,
/// `ExecuteEffect`, `Decide`) SPEND money against a pinned policy and
/// must not spend it against a different one. That gate is correct, and
/// it is also why a run journaled under an engine that has since moved
/// can never reach a lawful conclusion: the door it needs is behind a
/// lock that exists for other doors.
///
/// This is the other door. An operator stop conclusion appends nothing
/// but bookkeeping — `operator/commanded`, `operator/accepted`,
/// at most one `effect/indeterminate`, and `run/stopped` — and reads no
/// policy to append any of it: `fold`'s `"stop"` arm lands at any cursor
/// (riding an in-flight attempt to its boundary, concluding where it
/// stands otherwise), and the boundary close is the same event
/// `advance_running` writes at `Cursor::EffectInFlight` on a fresh
/// process, which consults no bundle either. A closure that spawns
/// nothing needs no pinned recipe to be honest about what it wrote.
///
/// Deterministic throughout (law 2): the caller supplies a run id, an
/// operator identity, and a reason — never a cursor or a status. Every
/// position is re-derived by re-folding the journal after each append,
/// and an unexpected one is an error rather than a guess.
///
/// Every write is FENCED (the operator's park ruling, 2026-09-01,
/// applying the compare-and-append the concurrent-writers slice
/// landed): the stop command re-decides on a moved head and its
/// refusal ends the conclusion, and both closing appends land only on
/// the exact head this process just folded. A run something else is
/// still driving therefore refuses instead of being closed over: ANY
/// movement of the head is evidence the run is not dead, and a
/// conclusion is for a run believed dead. `resume`'s fresh-process
/// branch still carries the unfenced hazard; decision 0029 (proposed)
/// rules on fencing it. `brokkr runs` remains the way to look first.
pub fn conclude(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
) -> Result<RunState, EngineError> {
    conclude_racing(store, run_id, operator, reason, |_| {})
}

/// [`conclude`] with the windows held open: `between` runs before the
/// stop command and inside each fence — after the head is taken, before
/// the append lands on it. Production passes a no-op; tests pass the
/// live driver the fences exist to refuse.
fn conclude_racing(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
    mut between: impl FnMut(&mut Store),
) -> Result<RunState, EngineError> {
    // `load` verifies the hash chain and never returns a partial journal,
    // so a broken chain refuses the whole conclusion here — before any
    // append. No second verification, and the error is never swallowed.
    let state = fold(&store.load(run_id)?)?;
    if matches!(state.status, Status::Completed | Status::Stopped) {
        return Err(EngineError::AlreadyConcluded {
            run_id: run_id.to_string(),
            status: match state.status {
                Status::Completed => "completed",
                _ => "stopped",
            }
            .to_string(),
        });
    }

    // A stop already in force is not re-commanded: the operator who
    // typed `brokkr operator stop` is the cause the journal already
    // names, and a second command would put a second name on the
    // conclusion. Only a run with no stop pending gets one, naming the
    // operator invoking `conclude`.
    between(store);
    if state.cursor != Cursor::Stop && !state.riding_stop {
        if let FencedCommandOutcome::Rejected {
            reason: refusal, ..
        } = operator_command(store, run_id, "stop", operator, reason)?
        {
            return Err(EngineError::Other(format!(
                "conclude: run '{run_id}' refused the stop ({refusal}); the                  journal moved beneath the conclusion, so something may still                  be driving this run — look with `brokkr runs` before closing"
            )));
        }
    }

    // The accepted stop rides an in-flight attempt to its boundary. This
    // process holds no driver for that attempt, so completion cannot be
    // established: close it indeterminate, exactly as a fresh drive
    // would. Closing the boundary is what SPENDS the ride (fold's
    // `conclude`), so the loop turns at most once — but what ends it is
    // the re-folded cursor, never a count kept here.
    let mut events = store.load(run_id)?;
    while let Some((effect_id, attempt_id)) = riding_attempt(run_id, &fold(&events)?)? {
        let head = events.last().expect("a foldable journal has a head");
        let (head_seq, head_hash, head_cause) =
            (head.seq, head.event_hash.clone(), head.event_id.clone());
        between(store);
        concluded_or_alive(
            run_id,
            store.append_next_if_head(
                run_id,
                head_seq,
                &head_hash,
                EventType::EffectIndeterminate,
                json!({
                    "effect_id": effect_id,
                    "attempt_id": attempt_id,
                    "reason": "the run was concluded from its journal while the attempt \
                               was in flight; completion cannot be established",
                }),
                Some(head_cause),
                Some(attempt_id),
            ),
        )?;
        events = store.load(run_id)?;
    }

    // `riding_attempt` answering None IS the statement that the cursor is
    // `Cursor::Stop`; it refuses anything else rather than letting a
    // `run/stopped` be appended somewhere it does not belong.
    let head = events.last().expect("a foldable journal has a head");
    let (head_seq, head_hash, head_cause) =
        (head.seq, head.event_hash.clone(), head.event_id.clone());
    between(store);
    concluded_or_alive(
        run_id,
        store.append_next_if_head(
            run_id,
            head_seq,
            &head_hash,
            EventType::RunStopped,
            json!({"reason": operator_stop_reason(&events)}),
            Some(head_cause),
            None,
        ),
    )?;
    Ok(fold(&store.load(run_id)?)?)
}

/// The fence's verdict, read as `conclude` must read it: a head that
/// moved beneath a conclusion is not a race to win but evidence the run
/// is alive, so it refuses with the look-first instruction instead of
/// retrying against a journal something else is writing.
fn concluded_or_alive(
    run_id: &str,
    written: Result<EventEnvelope, brokkr_store::StoreError>,
) -> Result<EventEnvelope, EngineError> {
    match written {
        Err(brokkr_store::StoreError::HeadMoved { .. }) => Err(EngineError::Other(format!(
            "conclude: the journal moved beneath the conclusion of run '{run_id}',              so something may still be driving it — a conclusion is for a run              believed dead; look with `brokkr runs` before closing"
        ))),
        other => Ok(other?),
    }
}

/// Where a run with an accepted stop stands, read off the cursor a
/// re-fold produced rather than predicted: `None` at `Cursor::Stop` —
/// the position `run/stopped` belongs at — and the in-flight attempt the
/// ride must close first otherwise. Under an accepted stop the fold
/// admits no third position, so any other cursor is a fold or engine
/// defect and refuses rather than guessing at a conclusion.
fn riding_attempt(run_id: &str, state: &RunState) -> Result<Option<(String, String)>, EngineError> {
    match &state.cursor {
        Cursor::Stop => Ok(None),
        Cursor::EffectInFlight {
            effect_id,
            attempt_id,
            ..
        } if state.riding_stop => Ok(Some((effect_id.clone(), attempt_id.clone()))),
        cursor => Err(EngineError::Other(format!(
            "conclude: run '{run_id}' stands at {cursor:?} with an accepted stop; \
             a stop reaches Stop or rides an in-flight attempt and nothing else"
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FencedCommandOutcome {
    Accepted {
        head_seq: u64,
        head_hash: String,
    },
    Rejected {
        reason: String,
        head_seq: u64,
        head_hash: String,
    },
}

/// Apply a command received through the Looper producer bridge. The command id
/// is supplied by Looper, the expected cursor/hash fences concurrent operator
/// activity, and both acceptance and rejection become Brokkr journal evidence
/// before any control-state effect is possible. The acceptance is written
/// against the head the cursor check covered ([`Store::append_next_if_head`]),
/// so an engine append between that check and the write loses the fence
/// instead of slipping under it.
// The arguments intentionally keep each security-relevant wire field explicit
// at this narrow trust boundary rather than hiding them in an unvalidated bag.
#[allow(clippy::too_many_arguments)]
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
    apply_fenced_racing(
        store,
        run_id,
        command_id,
        command,
        operator,
        reason,
        expected_seq,
        expected_hash,
        |_| {},
    )
}

/// [`apply_fenced_operator_command`] with the window held open: `between`
/// runs after `operator/commanded` lands and before the acceptance is
/// written against it — the instant [`Store::append_next_if_head`]'s
/// fence exists for. Production passes a no-op; tests pass a peer.
#[allow(clippy::too_many_arguments)]
fn apply_fenced_racing(
    store: &mut Store,
    run_id: &str,
    command_id: &str,
    command: &str,
    operator: &str,
    reason: &str,
    expected_seq: u64,
    expected_hash: &str,
    mut between: impl FnMut(&mut Store),
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
        Some(COMMAND_NOT_ALLOWED)
    } else if head_seq != expected_seq || head_hash != expected_hash {
        Some(STALE_CURSOR)
    } else if state.status != Status::AwaitingOperator {
        Some(RUN_NOT_AWAITING_OPERATOR)
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
    between(store);
    let disposition = if let Some(rejection) = rejection {
        refuse(
            store,
            run_id,
            command_id,
            operator,
            rejection,
            &commanded.event_id,
        )?
    } else {
        // The cursor was checked above, before `operator/commanded` was
        // written; the acceptance is written against the head that check
        // covers — the command itself — so a peer's append in between
        // cannot slip underneath it. It cannot be re-decided here the way
        // the unfenced path re-decides: this caller HOLDS a cursor, and
        // deciding against a head they never saw is what the fence exists
        // to prevent. They re-read and re-issue; the journal says why.
        match store.append_next_if_head(
            run_id,
            commanded.seq,
            &commanded.event_hash,
            EventType::OperatorAccepted,
            json!({"command_id": command_id, "operator": operator, "reason": reason}),
            Some(commanded.event_id.clone()),
            None,
        ) {
            Ok(_) => {
                let (head_seq, head_hash) = store.head_hash(run_id)?;
                FencedCommandOutcome::Accepted {
                    head_seq,
                    head_hash,
                }
            }
            Err(brokkr_store::StoreError::HeadMoved { .. }) => {
                let cause = &commanded.event_id;
                refuse(store, run_id, command_id, operator, STALE_CURSOR, cause)?
            }
            Err(error) => return Err(error.into()),
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

/// The outcome of an attempt whose checkpoint the journal refused
/// (decision 0034, ruling 6). A driver that went on to succeed did not:
/// its account is nonconforming, and the attempt fails on the refusal.
/// A driver that failed on its own keeps its own error beside the
/// refusal. A driver that was lost stays lost — indeterminate always
/// parks (decision 0006), and a refused checkpoint is no reason to
/// retry a process whose end nobody saw.
fn refused_outcome(outcome: AttemptOutcome, refusal: &SeatRecordError) -> AttemptOutcome {
    match outcome {
        AttemptOutcome::Succeeded { .. } => AttemptOutcome::Failed {
            error: refusal.to_string(),
        },
        AttemptOutcome::Failed { error } => AttemptOutcome::Failed {
            error: format!("{refusal}; the driver then failed: {error}"),
        },
        AttemptOutcome::Indeterminate { reason } => AttemptOutcome::Indeterminate { reason },
    }
}

/// The invocation-site tag an agent-resolved site is journaled under:
/// `None` for a single seat, `<member>` for a panel member, `<step>` for
/// a sequence step, `<step>:<member>` for a member inside a step panel —
/// exactly the tags the engine already uses for checkpoints.
type Site = Option<String>;

/// The candidate chosen for each agent-resolved invocation site of this
/// attempt, derived from journaled facts BEFORE anything spawns. Inline
/// sites are absent from this map, which is what keeps their execute
/// path exactly as it was.
type Selection = BTreeMap<Site, Candidate>;

/// Every invocation site of a seat body, with the site's fallback chain.
/// Inline sites carry an empty chain.
fn invocation_sites(body: ExecutableBody<'_>) -> Vec<(Site, &[Candidate])> {
    fn panel<'a>(members: &'a [PanelMember], prefix: Option<&str>) -> Vec<(Site, &'a [Candidate])> {
        members
            .iter()
            .map(|member| {
                let tag = match prefix {
                    None => member.name.clone(),
                    Some(step) => format!("{step}:{}", member.name),
                };
                (Some(tag), member.candidates.as_slice())
            })
            .collect()
    }
    match body {
        ExecutableBody::Single { candidates, .. } => vec![(None, candidates)],
        ExecutableBody::Panel { members, .. } => panel(members, None),
        ExecutableBody::Sequence { steps } => steps
            .iter()
            .flat_map(|step| match &step.body {
                StepBody::Single { candidates, .. } => {
                    vec![(Some(step.name.clone()), candidates.as_slice())]
                }
                StepBody::Panel { members, .. } => panel(members, Some(&step.name)),
                StepBody::Dialect { .. } => {
                    vec![(Some(step.name.clone()), &[] as &[Candidate])]
                }
            })
            .collect(),
    }
}

/// How far along its chain a site has walked: the number of PRIOR
/// attempts for this effect whose terminal event recorded a fail-to-start
/// for this site, clamped to the last candidate.
///
/// Derived by scanning the effect's own events, the way `advance_running`
/// already scans them for the last error — so `fold` and `RunState` are
/// untouched, and a crash between attempts cannot change which model runs
/// next.
fn chain_index(events: &[EventEnvelope], effect_id: &str, site: &Site, chain: usize) -> usize {
    let failures = events
        .iter()
        .filter(|event| {
            event.event_type == EventType::EffectFailed
                && event.payload.get("effect_id").and_then(Value::as_str) == Some(effect_id)
        })
        .filter(|event| {
            event
                .payload
                .get("start_failure_sites")
                .and_then(Value::as_array)
                .is_some_and(|sites| sites.iter().any(|entry| site_matches(entry, site)))
        })
        .count();
    failures.min(chain.saturating_sub(1))
}

fn site_matches(entry: &Value, site: &Site) -> bool {
    match site {
        None => entry.is_null(),
        Some(tag) => entry.as_str() == Some(tag.as_str()),
    }
}

/// The per-site selection for this attempt, plus the provenance list the
/// `effect/started` payload carries. `None` when no site is
/// agent-resolved: a non-adopting run's journal gains no field at all.
fn select_candidates(
    events: &[EventEnvelope],
    effect_id: &str,
    body: ExecutableBody<'_>,
) -> (Selection, Option<Value>) {
    let mut selection = Selection::new();
    let mut provenance = Vec::new();
    for (site, chain) in invocation_sites(body) {
        if chain.is_empty() {
            continue;
        }
        let index = chain_index(events, effect_id, &site, chain.len());
        let candidate = chain[index].clone();
        provenance.push(json!({
            "member": site,
            "agent": candidate.agent,
            "model": candidate.model,
            // The other half of the hire, journaled beside the model it
            // was pinned with (decision 0035 ruling 5). Additive, like
            // `model` before it: a candidate an effortless provider
            // serves carries `null`, and a run journaled before this
            // decision carries no key at all — which is exactly what
            // ruling 6 wants the view to show as absent.
            "effort": candidate.effort,
            "provider": candidate.provider,
            "chain_index": index,
        }));
        selection.insert(site, candidate);
    }
    let provenance = (!provenance.is_empty()).then_some(Value::Array(provenance));
    (selection, provenance)
}

/// The session this seat is holding in THIS run: the last transcript
/// locator any attempt of this seat journaled, with that attempt.
///
/// Journaled checkpoints are the only channel read — `state =
/// fold(events)`, and a driver's transcript locator reaches the record as
/// evidence the moment its harness announces one, which is what lets an
/// attempt killed on its deadline still hand its thread to the retry
/// that follows.
fn seat_session(events: &[EventEnvelope], seat: &str) -> Option<(String, String)> {
    let effects: Vec<&str> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .filter(|event| event.payload.get("seat").and_then(Value::as_str) == Some(seat))
        .filter_map(|event| event.payload.get("effect_id").and_then(Value::as_str))
        .collect();
    events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| {
            event
                .payload
                .get("effect_id")
                .and_then(Value::as_str)
                .is_some_and(|effect_id| effects.contains(&effect_id))
        })
        .find_map(|event| {
            let checkpoint = event.payload.get("checkpoint")?;
            // Decision 0032's common shape is first. The old flat id stays
            // readable so a run opened before the ruling can still resume.
            let session = checkpoint
                .pointer("/transcript/locator")
                .and_then(Value::as_str)
                .filter(|locator| !locator.is_empty())
                .or_else(|| checkpoint.get("session_id").and_then(Value::as_str))?;
            let attempt = event.attempt_id.clone()?;
            Some((attempt, session.to_string()))
        })
}

/// Is the bundle this attempt runs under the one the run pinned at its
/// first event? The pin carries the engine version, every bundle file's
/// digest, each referenced charter and agent definition, and the adapter
/// declaration digest that authorised each driver — so this single
/// comparison is decision 0030 ruling 4's "an adapter edit, or an engine
/// upgrade between attempts spawns cold", asked of the journal rather
/// than argued from `Engine::resume` refusing the same thing one layer
/// out. Two doors, one answer, and this one is on the path that hands
/// over a session handle.
fn pinned_bundle_holds(events: &[EventEnvelope], bundle: &Bundle) -> bool {
    events
        .iter()
        .find(|event| event.event_type == EventType::RunStarted)
        .and_then(|event| event.payload.get("manifest"))
        .and_then(|manifest| bundle_manifest_from_run(manifest).ok())
        .is_some_and(|pinned| pinned == bundle.manifest)
}

/// The session id this attempt may be handed, or `None` — which is the
/// answer for every attempt whose seat holds no session, and for every
/// attempt that is not the instance that opened the one it holds.
///
/// A session is one model's memory of one tree, held by the credential
/// and client that opened it. Handing it anywhere else is a terms
/// violation before it is a bug, so this offers one only when every
/// journaled fact about the two attempts agrees (decision 0030 ruling 4):
///
/// - the same run and the same seat, by construction — `seat_session`
///   reads this run's journal and only this seat's effects, so a retry
///   and a phase-machine re-entry qualify and nothing else does;
/// - the same driver binary and the same resolved candidate, by
///   comparing the `driver` label and the `provenance` this attempt is
///   about to journal against the ones the attempt that opened the
///   session did. A decision-0016 chain fallback moves the provenance;
///   a bundle that names a different driver moves the label;
/// - the same adapter declarations and the same engine, by the run's own
///   pin.
///
/// Only these two fields are compared, and deliberately so: `effect_id`
/// and `attempt_id` differ on every attempt by definition, and they are
/// the only other fields a single seat's start event carries.
///
/// `started_here` carries the axis the journal cannot: the same machine
/// and the same account. It is beyond the ruling's list, which is a
/// floor and not a ceiling — withholding an offer needs no permission —
/// and it is here because decision 0027 made runs portable. A journal
/// exported mid-flight and adopted elsewhere resumes as a first-class
/// run, by design indistinguishable INSIDE the chain, so no comparison
/// of journaled facts can tell that the attempt which opened the thread
/// ran under another machine's credential. Codex would refuse such a
/// thread today (its rollouts are local), but a driver whose provider
/// keeps sessions server-side would not be, and by then the offer has
/// been made. The store answers it instead, from bookkeeping that an
/// export does not carry.
fn resume_offer(
    events: &[EventEnvelope],
    bundle: &Bundle,
    seat: &str,
    started: &Value,
    started_here: bool,
) -> Option<String> {
    if !started_here || !pinned_bundle_holds(events, bundle) {
        return None;
    }
    let (attempt, session) = seat_session(events, seat)?;
    let opened_by = events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectStarted)
        .find(|event| event.attempt_id.as_deref() == Some(attempt.as_str()))
        .map(|event| &event.payload)?;
    let same_instance = ["driver", "provenance"]
        .iter()
        .all(|field| opened_by.get(field) == started.get(field));
    same_instance.then_some(session)
}

/// The argv to spawn for one invocation site: the selected candidate's,
/// or the compiled command when the site is inline.
fn argv_for<'a>(selection: &'a Selection, site: &Site, inline: &'a [String]) -> &'a [String] {
    match selection.get(site) {
        Some(candidate) => &candidate.argv,
        None => inline,
    }
}

/// The STRUCTURAL fail-to-start predicate (decision 0016): `Failed`, and
/// no `Accepted` ever received, and no checkpoint emitted. No stderr
/// sniffing and no "model not found" regex — the engine pattern-matching
/// a provider's prose to make a control decision is the control-plane
/// repair decision 0001 forbids.
///
/// Once `Accepted` arrives, this is false and fallback is unreachable by
/// construction: decision 0016's mid-session boundary is mechanised here
/// rather than described in a comment. A seat that ran for forty turns
/// and then hit a quota wall has produced work a different model does
/// not inherit, and it follows 0006 unchanged.
fn failed_to_start(report: &AttemptReport) -> bool {
    matches!(report.outcome, AttemptOutcome::Failed { .. })
        && !report.accepted
        && report.checkpoints.is_empty()
}

/// Which panel members failed to start, tagged as the journal tags them.
fn start_failure_sites(reports: &[(String, AttemptReport)], tag_prefix: &str) -> Vec<Site> {
    reports
        .iter()
        .filter(|(_, report)| failed_to_start(report))
        .map(|(name, _)| Some(format!("{tag_prefix}{name}")))
        .collect()
}

/// The fail-to-start fields for a terminal `effect/failed` payload —
/// absent entirely when no agent-resolved site failed to start, so an
/// inline seat's journal is byte-identical to what it always was.
fn start_failure_fields(payload: &mut Value, selection: &Selection, sites: Vec<Site>) {
    let journaled: Vec<Value> = sites
        .into_iter()
        .filter(|site| selection.contains_key(site))
        .map(|site| match site {
            None => Value::Null,
            Some(tag) => Value::String(tag),
        })
        .collect();
    if journaled.is_empty() {
        return;
    }
    payload["start_failure"] = Value::Bool(true);
    payload["start_failure_sites"] = Value::Array(journaled);
}

/// One panel member's derived driver invocation: the `seat` string the
/// driver sees, the (possibly confined) command, and its input.
struct MemberRun {
    name: String,
    driver_seat: String,
    command: Vec<String>,
    input: Value,
}

/// Copy the seat-level secret-binding facts (decision 0012: declared
/// names + store path, never values) into a derived member/step driver
/// input, when the seat binds any.
fn copy_secret_binding_facts(input: &mut Value, seat_input: &Value) {
    if let Some(secrets) = seat_input.get("secrets") {
        input["secrets"] = secrets.clone();
        input["secrets_file"] = seat_input["secrets_file"].clone();
    }
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
    stderr
        .chars()
        .rev()
        .take(2000)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// Join member reports into the attempt's single outcome: any
/// indeterminate member parks the whole attempt; otherwise any failed
/// member fails it (retryable under 0006); otherwise the declared
/// aggregate produces the one typed result.
fn panel_outcome(aggregate: Aggregate, reports: Vec<(String, AttemptReport)>) -> AttemptOutcome {
    let mut indeterminate = Vec::new();
    let mut failures = Vec::new();
    let mut member_results = Vec::new();
    for (name, report) in reports {
        match report.outcome {
            AttemptOutcome::Succeeded { result } => member_results.push((name, result)),
            AttemptOutcome::Failed { error } => failures.push(format!("{name}: {error}")),
            AttemptOutcome::Indeterminate { .. } => indeterminate.push(name),
        }
    }
    if !indeterminate.is_empty() {
        return AttemptOutcome::Indeterminate {
            reason: format!("panel members {indeterminate:?} could not establish completion"),
        };
    }
    if !failures.is_empty() {
        return AttemptOutcome::Failed {
            error: format!("panel members failed — {}", failures.join("; ")),
        };
    }
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
                let evidence: Map<String, Value> = members.iter().cloned().collect();
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
            let mut spec_defect = false;
            for (_, _, payload) in &parsed {
                if let Some(inputs) = payload.get("inputs").and_then(Value::as_object) {
                    if let Some(s) = inputs.get("max_residual_severity").and_then(Value::as_str) {
                        if let Some(i) = SEVERITY_ORDER.iter().position(|x| *x == s) {
                            severity_rank = severity_rank.max(i);
                        }
                    }
                    has_security |= inputs
                        .get("has_security_residual")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    spec_defect |= inputs
                        .get("spec_defect")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                }
            }
            let mut inputs = Map::new();
            // Backward-compatible evidence for frozen/third-party tables:
            // judges never fix, so the only lawful value is false. Shipped
            // tables no longer declare or read it and therefore drop it.
            inputs.insert("fixes_applied".into(), Value::Bool(false));
            inputs.insert("spec_defect".into(), Value::Bool(spec_defect));
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
/// role/result paths stay valid), every bundle ROOT read-only, extra
/// declared mounts read-only, network off unless granted. Absence of
/// confinement is the trusted class: a native child process.
///
/// A composed recipe has one root per layer (decision 0017): an
/// inherited confined seat's role file lives in its ancestor's
/// directory, and an unmounted role is a run-time break hours into a
/// run. For a bundle that composed nothing `roots` is `[dir]` and the
/// emitted argv is byte-identical to what it was before composition.
pub fn confined_command(
    command: &[String],
    confine: Option<&Confine>,
    workdir: &std::path::Path,
    roots: &[PathBuf],
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
    ];
    for root in roots {
        wrapped.push("-v".to_string());
        wrapped.push(format!("{}:{}:ro", root.display(), root.display()));
    }
    wrapped.push("-w".to_string());
    wrapped.push(workdir.display().to_string());
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

/// Decision 0043: put a site's hands in the box. A model seat's argv has
/// the adapter's fragment already; its two tokens are expanded here,
/// where the workdir and this binary's path are known — the MCP server a
/// harness spawns is `brokkr hands serve` on this very executable. An
/// `exec` dispatch is boxed whole instead: `brokkr hands exec` builds the
/// namespace at run time and passes the driver's stdio straight through.
/// A site without hands gets its command back untouched.
pub fn hands_command(
    command: Vec<String>,
    hands: Option<&brokkr_protocol::hands::HandsSpec>,
    workdir: &std::path::Path,
    roots: &[PathBuf],
) -> Vec<String> {
    let Some(spec) = hands else {
        return command;
    };
    let brokkr = std::env::current_exe().unwrap_or_default();
    let is_exec = command.len() >= 3 && command[1] == "driver" && command[2] == "exec";
    if is_exec {
        let bundle_root = roots
            .iter()
            .find(|root| {
                command
                    .iter()
                    .any(|part| Path::new(part).strip_prefix(root).is_ok())
            })
            .or_else(|| roots.first());
        let command = command
            .into_iter()
            .map(|part| {
                bundle_root
                    .and_then(|root| Path::new(&part).strip_prefix(root).ok())
                    .map(|relative| {
                        brokkr_protocol::hands::namespace_join(
                            brokkr_protocol::hands::SANDBOX_BUNDLE,
                            relative,
                        )
                    })
                    .unwrap_or(part)
            })
            .collect::<Vec<_>>();
        let mut boxed = vec![
            brokkr.to_string_lossy().into_owned(),
            "hands".to_string(),
            "exec".to_string(),
            "--workdir".to_string(),
            workdir.to_string_lossy().into_owned(),
            "--spec".to_string(),
            spec.to_value().to_string(),
        ];
        if let Some(root) = bundle_root {
            boxed.extend([
                "--bundle-root".to_string(),
                root.to_string_lossy().into_owned(),
            ]);
        }
        boxed.push("--".to_string());
        boxed.extend(command);
        return boxed;
    }
    let mcp_json = brokkr_protocol::hands::mcp_config(&brokkr, workdir, spec).to_string();
    let args_toml = format!(
        "[{}]",
        brokkr_protocol::hands::serve_args(workdir, spec)
            .iter()
            .map(|arg| format!("\"{}\"", arg.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(",")
    );
    command
        .into_iter()
        .map(|part| {
            part.replace("{hands_mcp_json}", &mcp_json)
                .replace("{hands_args_toml}", &args_toml)
                .replace("{brokkr}", &brokkr.to_string_lossy())
        })
        .collect()
}

fn manifest_diff(pinned: &Value, current: &Value) -> String {
    let empty = Map::new();
    let pinned_files = pinned
        .get("files")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let current_files = current
        .get("files")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
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
            && !entry
                .split('/')
                .any(|component| component == "." || component == "..");
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

impl Engine {
    /// The payload of `phase/entered`: the phase, and for the protected
    /// phase the repository head it was entered at (decision 0039), so
    /// the phase's own commits can later be told from the ones it judged.
    /// Optional and absent by default — no repository, no head — and
    /// published as `contracts/phase-entered-head.v1.schema.json`; `fold`
    /// never reads it.
    fn phase_entered_payload(&self, phase: &str, state: &RunState) -> Value {
        let mut payload = json!({"phase": phase});
        if let Some((_, Some(case))) = self
            .bundle
            .seats
            .get(phase)
            .and_then(|seat| seat.body.selected(state.strategy.as_deref()))
        {
            payload["case"] = Value::String(case.to_string());
        }
        let returning_implement =
            phase == "implement" && state.visits.get("implement").copied().unwrap_or(0) > 0;
        if phase == self.bundle.protected_phase || returning_implement {
            if let Some(head) = self.repo.as_deref().and_then(git_head) {
                payload["head"] = Value::String(head);
            }
        }
        payload
    }

    /// Decision 0039: whether every commit the protected phase added
    /// between the head it was entered at and `head`, the repository's
    /// head now, lies in the docs class the repository declared AT THAT
    /// ENTRY HEAD. `None` whenever the question has no honest answer —
    /// the phase's latest entry recorded no head or one that is not a
    /// commit id, the same head, nothing committed, a diff git cannot
    /// take, or no class declared at the entry head — and an absent
    /// input never satisfies a rule (decision 0004). A repository whose
    /// head cannot be read never reaches here: the caller records no
    /// head and asks nothing.
    ///
    /// The entry head is the LATEST entry's, taken as recorded: an older
    /// visit's head is never borrowed, because the phase's own commits
    /// are exactly the ones since it was last entered. The head is
    /// checked against the contract's shape before git sees it — the
    /// chain is unkeyed, and a journal row is not an argument list.
    fn fixes_docs_only(&self, repo: &std::path::Path, phase: &str, head: &str) -> Option<bool> {
        let events = self.store.load(&self.run_id).ok()?;
        let entered = events
            .iter()
            .rev()
            .find(|event| {
                event.event_type == EventType::PhaseEntered && event.payload["phase"] == phase
            })?
            .payload["head"]
            .as_str()
            .filter(|entered| is_commit_id(entered))?;
        if entered == head {
            return None;
        }
        let class = docs_class(repo, entered)?;
        let paths = crate::anchor::changed_paths(repo, entered, head)?;
        if paths.is_empty() {
            return None;
        }
        Some(
            paths
                .iter()
                .all(|path| class.iter().any(|pattern| pattern.is_match(path))),
        )
    }
}

/// The shape `contracts/phase-entered-head.v1.schema.json` promises:
/// forty lowercase hex digits, and nothing that could read as an option.
fn is_commit_id(candidate: &str) -> bool {
    candidate.len() == 40
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Where the repository declares its own delivery classes (decision 0038
/// ruling 3) — the same file the contribution gate reads.
const DELIVERY_CLASSES: &str = ".github/delivery-classes.json";

/// The docs class the repository declared at `head`: `classes.docs.paths`
/// of `.github/delivery-classes.json` as committed there, regular
/// expressions over the repository-relative path. Read from the commit
/// and never from the working tree, for the reason the gate reads the
/// base branch's copy: the tree at ruling is the judged phase's own, and
/// a phase that could widen the class its fixes are judged by has been
/// handed the gate. A change to the class file is then a path like any
/// other, classified by the class it was entered under. Absent at that
/// head, or malformed, means no class — and therefore no answer, never a
/// guess. The patterns are compiled here by the `regex` crate and joined
/// by the gate into jq's; a pattern only one dialect accepts is refused
/// here, and refusal reads as no class.
fn docs_class(repo: &std::path::Path, head: &str) -> Option<Vec<regex::Regex>> {
    let out = Command::new("git")
        .args(["cat-file", "blob", &format!("{head}:{DELIVERY_CLASSES}")])
        .current_dir(repo)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value: Value = serde_json::from_slice(&out.stdout).ok()?;
    value["classes"]["docs"]["paths"]
        .as_array()?
        .iter()
        .map(|pattern| pattern.as_str().and_then(|s| regex::Regex::new(s).ok()))
        .collect()
}

/// The repository's observed HEAD, or nothing when there is no readable
/// git tree there. Public because `brokkr realms` reads out the same
/// fact the ship gate compares against — one reader, not two.
pub fn git_head(repo: &std::path::Path) -> Option<String> {
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

/// Whether the EFFECT's own span carries the gate-head observation
/// (decision 0041 ruling 4). A gate single seat or an all-gate panel has
/// no inner spans, so the effect is the gate and the effect is watched.
/// A sequence never is: its Gate steps arm and compare at their own
/// ends, and `has_gate` — true of an all-gate sequence too — would
/// otherwise put a second, outer observation into the same one field,
/// an observation nothing compares at the right end and nothing accounts
/// for. One observation per gate step, and nothing outer.
fn arms_effect_gate_head(body: &ExecutableBody<'_>, seat: &Seat, strategy: Option<&str>) -> bool {
    !matches!(body, ExecutableBody::Sequence { .. })
        && seat.body.selected_is_gate(strategy, seat.has_gate)
}

/// Recover the raw observations carried by the most recent gate defect. The
/// indeterminate reason is an existing string field; the structured copy is
/// attached to `run/parked`, the contract's evidence envelope.
fn gate_head_evidence(events: &[EventEnvelope]) -> Value {
    events
        .iter()
        .rev()
        .filter(|event| event.event_type == EventType::EffectIndeterminate)
        .find_map(|event| {
            event.payload["reason"]
                .as_str()?
                .strip_prefix("GATE-MOVED-HEAD ")
                .and_then(|raw| serde_json::from_str(raw).ok())
        })
        .unwrap_or(json!({}))
}

#[cfg(test)]
mod agent_tests;

#[cfg(test)]
mod artifact_gate_tests;

#[cfg(test)]
mod conclude_tests;

#[cfg(test)]
mod contention_tests;

#[cfg(test)]
mod resume_tests;

#[cfg(test)]
mod secret_threading_tests;

#[cfg(test)]
mod tests;
