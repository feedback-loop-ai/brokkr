//! Authenticated Looper producer bridge.
//!
//! The bridge reads only through `brokkr-store`'s verified public API. It never
//! exposes SQLite, never forwards raw driver output, and never treats producer
//! evidence as Looper authority. Ordered Forge facts are normalized into the
//! closed producer vocabulary; separately authorized operator commands are
//! fenced by the exact Forge journal head before becoming control events.

use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_core::dispatch::{bundle_manifest_from_run, dispatch_from_run, DispatchEnvelopeV2};
use brokkr_core::{EventEnvelope, EventType};
use brokkr_runtime::{apply_fenced_operator_command, FencedCommandOutcome};
use brokkr_store::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;
use time::OffsetDateTime;

pub const PRODUCER_EVENT_SCHEMA: &str = "looper.forge-producer-event/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalCost {
    pub state: String,
    pub amount_microunits: Option<u64>,
    pub currency: Option<String>,
    pub coverage: String,
    pub authority: String,
    pub payer: String,
    pub lane_tally_run_id: String,
    pub reconciliation: String,
    pub observed_usage: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerEvent {
    pub schema: String,
    pub registration_id: String,
    pub looper_delivery_run_id: String,
    pub forge_run_id: String,
    pub forge_sequence: u64,
    pub event_id: String,
    pub previous_hash: String,
    pub event_hash: String,
    pub forge_event_schema: u32,
    pub causation_id: Option<String>,
    pub correlation_id: String,
    pub attempt_id: Option<String>,
    pub recorded_at: String,
    pub semantic_type: String,
    pub payload_version: u32,
    pub payload: Value,
    pub payload_digest: String,
    pub runtime_id: String,
    pub request_grant_id: String,
    pub producer_release: String,
    pub attestation: String,
    pub terminal_identity: Option<String>,
    pub cost: CanonicalCost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrationState {
    pub registration_id: String,
    pub status: String,
    pub last_forge_sequence: u64,
    pub last_event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerCommand {
    pub cursor: u64,
    pub id: String,
    pub command: String,
    pub expected_forge_sequence: u64,
    pub expected_event_hash: String,
    pub actor: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandReceipt {
    pub command_id: String,
    pub outcome: String,
    pub reason: Option<String>,
    pub forge_sequence: u64,
    pub event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncReport {
    pub registered: bool,
    pub submitted: usize,
    pub replayed: usize,
    pub commands: usize,
    pub last_command_cursor: u64,
    pub last_forge_sequence: u64,
}

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("store: {0}")]
    Store(#[from] brokkr_store::StoreError),
    #[error("dispatch: {0}")]
    Dispatch(#[from] brokkr_core::dispatch::DispatchError),
    #[error("runtime: {0}")]
    Runtime(#[from] brokkr_runtime::EngineError),
    #[error("run is not bound to a Looper dispatch")]
    UnboundRun,
    #[error("producer transport: {0}")]
    Transport(String),
    #[error("normalized event exceeds dispatch bound")]
    EventTooLarge,
    #[error("producer semantic type is outside the dispatch grant")]
    EffectNotAllowed,
    #[error("registration identity or state does not match the dispatch")]
    RegistrationMismatch,
    #[error("producer authority is {0}")]
    AuthorityInactive(String),
}

pub trait ProducerTransport {
    fn register(
        &mut self,
        dispatch: &DispatchEnvelopeV2,
        run_manifest: &Value,
    ) -> Result<RegistrationState, BridgeError>;
    fn submit(&mut self, event: &ProducerEvent) -> Result<bool, BridgeError>;
    fn commands(
        &mut self,
        registration_id: &str,
        after: u64,
    ) -> Result<Vec<ProducerCommand>, BridgeError>;
    fn acknowledge_command(
        &mut self,
        registration_id: &str,
        receipt: &CommandReceipt,
    ) -> Result<(), BridgeError>;
}

/// Blocking HTTPS transport for the one native Forge binary. The bearer value
/// is held in memory and is never serialized into a run, event, or error.
pub struct HttpTransport {
    base_url: String,
    bearer: String,
}

impl HttpTransport {
    pub fn new(base_url: impl Into<String>, bearer: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            bearer: bearer.into(),
        }
    }

    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, BridgeError> {
        let url = format!("{}{}", self.base_url, path);
        let request = ureq::request(method, &url)
            .set("Authorization", &format!("Bearer {}", self.bearer))
            .set("Content-Type", "application/json");
        let response = match body {
            Some(body) => request.send_json(body),
            None => request.call(),
        }
        .map_err(|error| BridgeError::Transport(http_error(error)))?;
        response
            .into_json()
            .map_err(|error| BridgeError::Transport(format!("invalid JSON response: {error}")))
    }
}

fn http_error(error: ureq::Error) -> String {
    match error {
        ureq::Error::Status(status, _) => format!("HTTP {status}"),
        ureq::Error::Transport(error) => format!("connection failed: {error}"),
    }
}

fn data<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T, BridgeError> {
    serde_json::from_value(value.get("data").cloned().unwrap_or(Value::Null))
        .map_err(|error| BridgeError::Transport(format!("invalid response shape: {error}")))
}

impl ProducerTransport for HttpTransport {
    fn register(
        &mut self,
        dispatch: &DispatchEnvelopeV2,
        run_manifest: &Value,
    ) -> Result<RegistrationState, BridgeError> {
        if self.base_url != dispatch.producer.callback_audience.trim_end_matches('/') {
            return Err(BridgeError::Transport(
                "transport origin does not match the sealed callback audience".into(),
            ));
        }
        let response = self.request(
            "POST",
            "/api/v1/delivery/forge-producers/registrations",
            Some(json!({"dispatch": dispatch, "runManifest": run_manifest})),
        )?;
        data(response)
    }

    fn submit(&mut self, event: &ProducerEvent) -> Result<bool, BridgeError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResultBody {
            replayed: bool,
        }
        let response = self.request(
            "POST",
            &format!(
                "/api/v1/delivery/forge-producers/{}/events",
                event.registration_id
            ),
            Some(serde_json::to_value(event).expect("event serializes")),
        )?;
        let body: ResultBody = data(response)?;
        Ok(body.replayed)
    }

    fn commands(
        &mut self,
        registration_id: &str,
        after: u64,
    ) -> Result<Vec<ProducerCommand>, BridgeError> {
        let response = self.request(
            "GET",
            &format!("/api/v1/delivery/forge-producers/{registration_id}/commands?after={after}"),
            None,
        )?;
        data(response)
    }

    fn acknowledge_command(
        &mut self,
        registration_id: &str,
        receipt: &CommandReceipt,
    ) -> Result<(), BridgeError> {
        let response = self.request(
            "POST",
            &format!(
                "/api/v1/delivery/forge-producers/{registration_id}/commands/{}/receipt",
                receipt.command_id
            ),
            Some(serde_json::to_value(receipt).expect("receipt serializes")),
        )?;
        let _: Value = data(response)?;
        Ok(())
    }
}

fn bounded(value: Option<&Value>, max: usize) -> Option<Value> {
    let text = value?.as_str()?;
    Some(Value::String(text.chars().take(max).collect()))
}

fn digest_string(value: Option<&Value>) -> Option<Value> {
    let text = value?.as_str()?;
    Some(Value::String(canonical::sha256_bytes(text.as_bytes())))
}

fn safe_checkpoint(checkpoint: &Value) -> Value {
    let mut output = Map::new();
    for key in [
        "step", "member", "outcome", "tool", "model", "provider", "harness", "profile",
    ] {
        if let Some(value) = bounded(checkpoint.get(key), 80) {
            output.insert(key.to_string(), value);
        }
    }
    for key in [
        "turn",
        "num_turns",
        "input_tokens",
        "output_tokens",
        "cache_read_tokens",
        "inner_checkpoints",
        "exit_code",
    ] {
        if let Some(value) = checkpoint.get(key).and_then(Value::as_i64) {
            output.insert(key.to_string(), Value::from(value));
        }
    }
    if let Some(target) = digest_string(checkpoint.get("target")) {
        output.insert("target_sha256".into(), target);
        output.insert(
            "target_state".into(),
            Value::String("withheld-private-path".into()),
        );
    }
    if checkpoint.get("session_id").is_some() || checkpoint.get("session_ref").is_some() {
        output.insert(
            "session_reference_state".into(),
            Value::String("observed-redacted".into()),
        );
    }
    if let Some(cost) = checkpoint.get("total_cost_usd").and_then(Value::as_f64) {
        // serde_json::Number cannot represent NaN or infinity, so a parsed
        // f64 is already finite; only the policy's non-negative bound remains.
        if cost >= 0.0 {
            output.insert("forge_observed_cost_usd".into(), Value::from(cost));
        }
    }
    if output.is_empty() {
        json!({"state":"withheld", "reason":"no-policy-permitted-checkpoint-fields"})
    } else {
        Value::Object(output)
    }
}

fn selected(payload: &Value, keys: &[&str]) -> Value {
    let mut output = Map::new();
    for key in keys {
        if let Some(value) = payload.get(*key) {
            match value {
                Value::String(value) => {
                    output.insert(
                        (*key).to_string(),
                        Value::String(value.chars().take(256).collect()),
                    );
                }
                Value::Bool(_) | Value::Number(_) | Value::Null => {
                    output.insert((*key).to_string(), value.clone());
                }
                _ => {
                    output.insert(
                        format!("{key}_sha256"),
                        Value::String(canonical::sha256_hex(value)),
                    );
                    output.insert(
                        format!("{key}_state"),
                        Value::String("withheld-structured-content".into()),
                    );
                }
            }
        }
    }
    Value::Object(output)
}

fn normalize_payload(event: &EventEnvelope) -> Value {
    match event.event_type {
        EventType::RunStarted => selected(&event.payload, &["manifest"]),
        EventType::PhaseEntered => selected(&event.payload, &["phase"]),
        EventType::EffectRequested => selected(
            &event.payload,
            &[
                "effect_id",
                "phase",
                "seat",
                "idempotency_key",
                "input_digest",
            ],
        ),
        EventType::EffectStarted => {
            selected(&event.payload, &["effect_id", "attempt_id", "driver"])
        }
        EventType::EffectCheckpointed => json!({
            "effect_id": event.payload.get("effect_id"),
            "attempt_id": event.payload.get("attempt_id"),
            "checkpoint": safe_checkpoint(&event.payload["checkpoint"]),
        }),
        EventType::EffectSucceeded => {
            selected(&event.payload, &["effect_id", "attempt_id", "result"])
        }
        EventType::EffectFailed | EventType::EffectIndeterminate => {
            let mut payload = selected(&event.payload, &["effect_id", "attempt_id"]);
            let message = event
                .payload
                .get("error")
                .or_else(|| event.payload.get("reason"));
            if let (Some(output), Some(digest)) = (payload.as_object_mut(), digest_string(message))
            {
                output.insert("reason_sha256".into(), digest);
                output.insert("reason_state".into(), Value::String("withheld".into()));
            }
            payload
        }
        EventType::TransitionDecided => selected(
            &event.payload,
            &["from", "result", "rule_id", "next", "severity"],
        ),
        EventType::OperatorCommanded => json!({
            "command_id": bounded(event.payload.get("command_id"), 256),
            "command_kind": bounded(event.payload.get("command"), 32),
            "operator": bounded(event.payload.get("operator"), 256),
        }),
        EventType::OperatorAccepted | EventType::OperatorRejected => {
            selected(&event.payload, &["command_id", "operator"])
        }
        EventType::RunParked | EventType::RunStopped => {
            let mut output = Map::new();
            if let Some(digest) = digest_string(event.payload.get("reason")) {
                output.insert("reason_sha256".into(), digest);
                output.insert("reason_state".into(), Value::String("withheld".into()));
            }
            Value::Object(output)
        }
        EventType::RunCompleted => json!({}),
    }
}

fn semantic_type(event_type: EventType) -> (&'static str, &'static str) {
    match event_type {
        EventType::EffectCheckpointed => ("checkpoint", "checkpoint"),
        EventType::EffectSucceeded
        | EventType::EffectFailed
        | EventType::EffectIndeterminate
        | EventType::TransitionDecided => ("report", "report"),
        EventType::OperatorAccepted | EventType::OperatorRejected => {
            ("intervention_response", "intervention_response")
        }
        EventType::RunCompleted | EventType::RunStopped => ("terminal_report", "terminal_report"),
        _ => ("observation", "observation"),
    }
}

fn observed_usage(payload: &Value) -> Value {
    let checkpoint = payload.get("checkpoint").unwrap_or(&Value::Null);
    let mut usage = BTreeMap::new();
    for key in [
        "num_turns",
        "input_tokens",
        "output_tokens",
        "cache_read_tokens",
        "forge_observed_cost_usd",
    ] {
        if let Some(value) = checkpoint.get(key) {
            usage.insert(key, value.clone());
        }
    }
    serde_json::to_value(usage).expect("usage serializes")
}

pub fn normalize_event(
    dispatch: &DispatchEnvelopeV2,
    event: &EventEnvelope,
) -> Result<ProducerEvent, BridgeError> {
    let (semantic_type, required_effect) = semantic_type(event.event_type);
    if !dispatch
        .allowed_effects
        .iter()
        .any(|effect| effect == required_effect)
    {
        return Err(BridgeError::EffectNotAllowed);
    }
    let payload = normalize_payload(event);
    let payload_digest = canonical::sha256_hex(&payload);
    let terminal_identity = matches!(
        event.event_type,
        EventType::RunCompleted | EventType::RunStopped
    )
    .then(|| {
        canonical::sha256_hex(&json!({
            "forge_run_id": event.run_id,
            "event_id": event.event_id,
            "event_hash": event.event_hash,
        }))
    });
    let dispatch_not_applicable = dispatch.budget.cost_state == "not-applicable";
    let event = ProducerEvent {
        schema: PRODUCER_EVENT_SCHEMA.into(),
        registration_id: dispatch.producer.registration_id.clone(),
        looper_delivery_run_id: dispatch.looper.delivery_run_id.clone(),
        forge_run_id: event.run_id.clone(),
        forge_sequence: event.seq,
        event_id: event.event_id.clone(),
        previous_hash: event.previous_hash.clone(),
        event_hash: event.event_hash.clone(),
        forge_event_schema: event.event_schema_version,
        causation_id: event.causation_id.clone(),
        correlation_id: event.correlation_id.clone(),
        attempt_id: event.attempt_id.clone(),
        recorded_at: event.recorded_at.clone(),
        semantic_type: semantic_type.into(),
        payload_version: 1,
        payload: payload.clone(),
        payload_digest,
        runtime_id: dispatch.producer.runtime_id.clone(),
        request_grant_id: dispatch.looper.request_grant_id.clone(),
        producer_release: dispatch.producer.producer_release.clone(),
        attestation: "self_reported".into(),
        terminal_identity,
        cost: CanonicalCost {
            state: if dispatch_not_applicable {
                "not-applicable".into()
            } else {
                "reconciliation-pending".into()
            },
            amount_microunits: None,
            currency: dispatch.budget.currency.clone(),
            coverage: if dispatch_not_applicable {
                "unavailable".into()
            } else {
                "partial".into()
            },
            authority: "provisional".into(),
            payer: "accountable-operator".into(),
            lane_tally_run_id: dispatch.budget.lane_tally_run_id.clone(),
            reconciliation: if dispatch_not_applicable {
                "not-required-by-policy".into()
            } else {
                "lane-tally-required".into()
            },
            observed_usage: observed_usage(&payload),
        },
    };
    let bytes = serde_json::to_vec(&event).expect("event serializes").len();
    if bytes > dispatch.bounds.max_event_bytes as usize {
        return Err(BridgeError::EventTooLarge);
    }
    Ok(event)
}

pub struct Bridge<T> {
    transport: T,
    event_times: VecDeque<Instant>,
}

impl<T: ProducerTransport> Bridge<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            event_times: VecDeque::new(),
        }
    }

    fn await_event_slot(&mut self, limit: u32) {
        let window = Duration::from_secs(10);
        let mut now = Instant::now();
        while self
            .event_times
            .front()
            .is_some_and(|recorded| now.duration_since(*recorded) >= window)
        {
            self.event_times.pop_front();
        }
        if self.event_times.len() >= limit as usize {
            let wait = window.saturating_sub(now.duration_since(self.event_times[0]));
            std::thread::sleep(wait);
            now = Instant::now();
            while self
                .event_times
                .front()
                .is_some_and(|recorded| now.duration_since(*recorded) >= window)
            {
                self.event_times.pop_front();
            }
        }
        self.event_times.push_back(now);
    }

    pub fn sync_once(
        &mut self,
        store: &mut Store,
        run_id: &str,
        now: OffsetDateTime,
        command_after: u64,
    ) -> Result<SyncReport, BridgeError> {
        let manifest = store.manifest(run_id)?;
        let dispatch = dispatch_from_run(&manifest)?.ok_or(BridgeError::UnboundRun)?;
        let bundle_manifest = bundle_manifest_from_run(&manifest)?;
        let bundle_sha = canonical::sha256_hex(&bundle_manifest);
        dispatch.verify(now, &bundle_sha)?;
        if dispatch.forge_run_id != run_id {
            return Err(BridgeError::RegistrationMismatch);
        }
        let registration = self.transport.register(&dispatch, &manifest)?;
        if registration.registration_id != dispatch.producer.registration_id {
            return Err(BridgeError::RegistrationMismatch);
        }
        let events = store.load(run_id)?;
        if registration.last_forge_sequence > events.len() as u64
            || (registration.last_forge_sequence > 0
                && events[(registration.last_forge_sequence - 1) as usize].event_hash
                    != registration.last_event_hash)
        {
            return Err(BridgeError::RegistrationMismatch);
        }
        if registration.status == "terminal" {
            if registration.last_forge_sequence != events.len() as u64 {
                return Err(BridgeError::RegistrationMismatch);
            }
            return Ok(SyncReport {
                registered: true,
                submitted: 0,
                replayed: 0,
                commands: 0,
                last_command_cursor: command_after,
                last_forge_sequence: registration.last_forge_sequence,
            });
        }
        if registration.status != "active" {
            return Err(BridgeError::AuthorityInactive(registration.status));
        }
        let mut submitted = 0;
        let mut replayed = 0;
        let mut submitted_through = registration.last_forge_sequence;
        for event in events
            .iter()
            .filter(|event| event.seq > registration.last_forge_sequence)
        {
            self.await_event_slot(dispatch.bounds.max_events_per_ten_seconds);
            if self.transport.submit(&normalize_event(&dispatch, event)?)? {
                replayed += 1;
            } else {
                submitted += 1;
            }
            submitted_through = event.seq;
        }

        let commands = self
            .transport
            .commands(&dispatch.producer.registration_id, command_after)?;
        let mut last_command_cursor = command_after;
        for command in &commands {
            if command.cursor <= last_command_cursor {
                return Err(BridgeError::RegistrationMismatch);
            }
            last_command_cursor = command.cursor;
            let result = apply_fenced_operator_command(
                store,
                run_id,
                &command.id,
                &command.command,
                &command.actor,
                &command.reason,
                command.expected_forge_sequence,
                &command.expected_event_hash,
            )?;
            // A one-shot bridge invocation is still a complete round trip: send
            // the durable command/disposition evidence before acknowledging it.
            let command_start = submitted_through;
            for event in store
                .load(run_id)?
                .iter()
                .filter(|event| event.seq > command_start)
            {
                self.await_event_slot(dispatch.bounds.max_events_per_ten_seconds);
                if self.transport.submit(&normalize_event(&dispatch, event)?)? {
                    replayed += 1;
                } else {
                    submitted += 1;
                }
                submitted_through = event.seq;
            }
            let receipt = match result {
                FencedCommandOutcome::Accepted {
                    head_seq,
                    head_hash,
                } => CommandReceipt {
                    command_id: command.id.clone(),
                    outcome: "accepted".into(),
                    reason: None,
                    forge_sequence: head_seq,
                    event_hash: head_hash,
                },
                FencedCommandOutcome::Rejected {
                    reason,
                    head_seq,
                    head_hash,
                } => CommandReceipt {
                    command_id: command.id.clone(),
                    outcome: "rejected".into(),
                    reason: Some(reason),
                    forge_sequence: head_seq,
                    event_hash: head_hash,
                },
            };
            self.transport
                .acknowledge_command(&dispatch.producer.registration_id, &receipt)?;
        }
        let (last_forge_sequence, _) = store.head_hash(run_id)?;
        Ok(SyncReport {
            registered: true,
            submitted,
            replayed,
            commands: commands.len(),
            last_command_cursor,
            last_forge_sequence,
        })
    }

    pub fn into_transport(self) -> T {
        self.transport
    }
}

#[cfg(test)]
mod tests;
