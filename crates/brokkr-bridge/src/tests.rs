use super::*;
use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::dispatch::{
    build_run_manifest_v2, ActorBinding, BudgetBinding, DispatchBounds, LooperBinding,
    ProducerBinding, RecipeBinding, RepositoryBinding, DISPATCH_SCHEMA_V2, PRODUCER_EFFECTS,
};
use time::format_description::well_known::Rfc3339;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

fn fixture() -> (Value, DispatchEnvelopeV2, OffsetDateTime) {
    let bundle = json!({
        "engine":"0.2.0", "event_schema":1, "database_schema":1,
        "driver_protocol":1, "bundle_name":"fast",
        "files":{"bundle.json":"a".repeat(64)}
    });
    let recipe_sha = canonical::sha256_hex(&bundle);
    let dispatch = DispatchEnvelopeV2 {
        schema: DISPATCH_SCHEMA_V2.into(),
        envelope_id: "envelope-1".into(),
        forge_run_id: "forge-run-1".into(),
        issued_at: "2026-08-28T08:00:00Z".into(),
        expires_at: "2026-08-28T09:00:00Z".into(),
        canonical_digest: String::new(),
        looper: LooperBinding {
            organization_id: "org-1".into(),
            product_id: "product-1".into(),
            story_id: "story-1".into(),
            delivery_run_id: "delivery-1".into(),
            request_grant_id: "grant-1".into(),
            feature_path: "084-f".into(),
            immutable_inputs_sha256: "a".repeat(64),
        },
        actor: ActorBinding {
            principal_kind: "api_key".into(),
            principal_id: "key-1".into(),
            actor_kind: "service".into(),
            actor_id: "brokkr".into(),
            accountable_operator_id: "operator-1".into(),
            authority_source: "looper-grant".into(),
            operating_profile: "bounded".into(),
        },
        repository: RepositoryBinding {
            owner: "feedback-loop-ai".into(),
            name: "looper".into(),
            base_sha: "b".repeat(64),
            candidate_sha: None,
            workspace_class: "isolated".into(),
            target_environment: "dogfood".into(),
        },
        recipe: RecipeBinding {
            name: "fast".into(),
            compiled_sha256: recipe_sha,
        },
        budget: BudgetBinding {
            lane_tally_run_id: "lane-1".into(),
            reservation_id: Some("reservation-1".into()),
            cost_state: "known".into(),
            ceiling_microunits: Some(1000),
            currency: Some("USD".into()),
        },
        producer: ProducerBinding {
            registration_id: "registration-1".into(),
            token_reference: "key-1".into(),
            callback_audience: "https://dogfood.feedback-loop.ai".into(),
            accepting_service_id: "looper-api".into(),
            runtime_id: "runtime-1".into(),
            producer_release: "brokkr@test".into(),
            protocol_version: 1,
            starting_cursor: 0,
        },
        allowed_effects: PRODUCER_EFFECTS
            .iter()
            .map(|value| (*value).into())
            .collect(),
        forbidden_actions: vec![
            "grant_create".into(),
            "grant_widen".into(),
            "artifact_decide".into(),
            "workflow_advance".into(),
            "release_promote".into(),
        ],
        bounds: DispatchBounds {
            max_attempts: 3,
            max_parallel_effects: 2,
            max_event_bytes: 65_536,
            max_events_per_ten_seconds: 20,
            replay_retention_seconds: 604_800,
            safe_stop: "boundary".into(),
            cancellation: "fenced".into(),
        },
        evidence_requirements: vec!["ordered_hash_chain".into()],
        attestation_requirement: "self_reported".into(),
    }
    .sealed();
    let manifest = build_run_manifest_v2(&bundle, dispatch.clone()).unwrap();
    let now = OffsetDateTime::parse("2026-08-28T08:30:00Z", &Rfc3339).unwrap();
    (manifest, dispatch, now)
}

#[derive(Default)]
struct MockTransport {
    state: Option<RegistrationState>,
    events: Vec<ProducerEvent>,
    commands: Vec<ProducerCommand>,
    receipts: Vec<CommandReceipt>,
    replay_all: bool,
}

impl ProducerTransport for MockTransport {
    fn register(
        &mut self,
        dispatch: &DispatchEnvelopeV2,
        _: &Value,
    ) -> Result<RegistrationState, BridgeError> {
        Ok(self
            .state
            .get_or_insert_with(|| RegistrationState {
                registration_id: dispatch.producer.registration_id.clone(),
                status: "active".into(),
                last_forge_sequence: 0,
                last_event_hash: ZERO_HASH.into(),
            })
            .clone())
    }

    fn submit(&mut self, event: &ProducerEvent) -> Result<bool, BridgeError> {
        if self.replay_all {
            return Ok(true);
        }
        if self
            .events
            .iter()
            .any(|prior| prior.event_hash == event.event_hash)
        {
            return Ok(true);
        }
        self.events.push(event.clone());
        self.state = Some(RegistrationState {
            registration_id: event.registration_id.clone(),
            status: "active".into(),
            last_forge_sequence: event.forge_sequence,
            last_event_hash: event.event_hash.clone(),
        });
        Ok(false)
    }

    fn commands(&mut self, _: &str, after: u64) -> Result<Vec<ProducerCommand>, BridgeError> {
        Ok(self
            .commands
            .iter()
            .filter(|command| command.cursor > after)
            .cloned()
            .collect())
    }

    fn acknowledge_command(
        &mut self,
        _: &str,
        receipt: &CommandReceipt,
    ) -> Result<(), BridgeError> {
        self.receipts.push(receipt.clone());
        Ok(())
    }
}

#[test]
fn sync_replays_from_server_cursor_without_sqlite_or_transcript_exposure() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (manifest, _, now) = fixture();
    store
        .create_run("forge-run-1", "084-f", "fast", &manifest)
        .unwrap();
    store
        .append_next(
            "forge-run-1",
            EventType::EffectCheckpointed,
            json!({
                "effect_id":"effect-1", "attempt_id":"attempt-1",
                "checkpoint": {
                    "step":"item-completed", "tool":"command_execution",
                    "command":"print-secret", "output":"private", "target":"/private/repo/file.rs",
                    "session_id":"secret-session", "input_tokens":21,
                }
            }),
            None,
            Some("attempt-1".into()),
        )
        .unwrap();
    let mut bridge = Bridge::new(MockTransport::default());
    let first = bridge.sync_once(&mut store, "forge-run-1", now, 0).unwrap();
    assert_eq!(first.submitted, 1);
    let second = bridge.sync_once(&mut store, "forge-run-1", now, 0).unwrap();
    assert_eq!(second.submitted, 0);
    let transport = bridge.into_transport();
    let encoded = serde_json::to_string(&transport.events[0]).unwrap();
    assert!(!encoded.contains("print-secret"));
    assert!(!encoded.contains("private/repo"));
    assert!(!encoded.contains("secret-session"));
    assert!(encoded.contains("target_sha256"));
    assert_eq!(transport.events[0].cost.state, "reconciliation-pending");
    assert_eq!(transport.events[0].cost.lane_tally_run_id, "lane-1");
}

#[test]
fn server_cursor_hash_must_be_a_prefix_of_the_verified_journal() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (manifest, _, now) = fixture();
    store
        .create_run("forge-run-1", "084-f", "fast", &manifest)
        .unwrap();
    store
        .append_next(
            "forge-run-1",
            EventType::RunCompleted,
            json!({}),
            None,
            None,
        )
        .unwrap();
    let transport = MockTransport {
        state: Some(RegistrationState {
            registration_id: "registration-1".into(),
            status: "active".into(),
            last_forge_sequence: 1,
            last_event_hash: "f".repeat(64),
        }),
        ..Default::default()
    };
    let error = Bridge::new(transport)
        .sync_once(&mut store, "forge-run-1", now, 0)
        .unwrap_err();
    assert!(
        matches!(error, BridgeError::RegistrationMismatch),
        "unexpected error: {error:?}"
    );
}

#[test]
fn terminal_registration_is_a_clean_idempotent_sync_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (manifest, _, now) = fixture();
    store
        .create_run("forge-run-1", "084-f", "fast", &manifest)
        .unwrap();
    let completed = store
        .append_next(
            "forge-run-1",
            EventType::RunCompleted,
            json!({}),
            None,
            None,
        )
        .unwrap();
    let transport = MockTransport {
        state: Some(RegistrationState {
            registration_id: "registration-1".into(),
            status: "terminal".into(),
            last_forge_sequence: completed.seq,
            last_event_hash: completed.event_hash,
        }),
        ..Default::default()
    };
    let report = Bridge::new(transport)
        .sync_once(&mut store, "forge-run-1", now, 9)
        .unwrap();
    assert_eq!(report.submitted, 0);
    assert_eq!(report.last_command_cursor, 9);
    assert_eq!(report.last_forge_sequence, 1);
}

#[test]
fn one_shot_command_reports_sanitized_disposition_before_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (manifest, _, now) = fixture();
    store
        .create_run("forge-run-1", "084-f", "fast", &manifest)
        .unwrap();
    for (event_type, payload, attempt_id) in [
        (EventType::RunStarted, json!({"manifest":{}}), None),
        (EventType::PhaseEntered, json!({"phase":"implement"}), None),
        (
            EventType::EffectRequested,
            json!({"effect_id":"effect-1", "seat":"implementer"}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect-1", "attempt_id":"attempt-1"}),
            Some("attempt-1".into()),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id":"effect-1", "attempt_id":"attempt-1"}),
            Some("attempt-1".into()),
        ),
        (
            EventType::RunParked,
            json!({"reason":"executor_lost"}),
            None,
        ),
    ] {
        store
            .append_next("forge-run-1", event_type, payload, None, attempt_id)
            .unwrap();
    }
    let (expected_forge_sequence, expected_event_hash) = store.head_hash("forge-run-1").unwrap();
    let transport = MockTransport {
        commands: vec![ProducerCommand {
            cursor: 1,
            id: "command-1".into(),
            command: "stop".into(),
            expected_forge_sequence,
            expected_event_hash,
            actor: "user:operator-1".into(),
            reason: "operator requested stop".into(),
        }],
        ..Default::default()
    };
    let mut bridge = Bridge::new(transport);
    let report = bridge.sync_once(&mut store, "forge-run-1", now, 0).unwrap();
    assert_eq!(report.commands, 1);
    assert_eq!(report.submitted, 8);
    let transport = bridge.into_transport();
    assert_eq!(transport.receipts.len(), 1);
    assert_eq!(transport.receipts[0].outcome, "accepted");
    let commanded = transport
        .events
        .iter()
        .find(|event| event.payload.get("command_kind").is_some())
        .unwrap();
    assert_eq!(commanded.payload["command_kind"], "stop");
    assert!(commanded.payload.get("command").is_none());
    assert!(!serde_json::to_string(commanded)
        .unwrap()
        .contains("operator requested stop"));
}

fn loopback_server(responses: Vec<(u16, String)>) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0_u8; 4096];
            let header_end = loop {
                let read = stream.read(&mut chunk).unwrap();
                assert!(read > 0, "request ended before headers");
                bytes.extend_from_slice(&chunk[..read]);
                if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                    break index + 4;
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            while bytes.len() < header_end + content_length {
                let read = stream.read(&mut chunk).unwrap();
                assert!(read > 0, "request ended before body");
                bytes.extend_from_slice(&chunk[..read]);
            }
            requests.push(String::from_utf8_lossy(&bytes).into_owned());
            let reason = if status == 200 { "OK" } else { "Unauthorized" };
            write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        }
        requests
    });
    (format!("http://{address}"), handle)
}

#[test]
fn http_transport_round_trips_every_protocol_operation() {
    let responses = vec![
        (
            200,
            json!({"data": {
                "registration_id": "registration-1",
                "status": "active",
                "last_forge_sequence": 0,
                "last_event_hash": ZERO_HASH,
            }})
            .to_string(),
        ),
        (200, json!({"data": {"replayed": true}}).to_string()),
        (200, json!({"data": []}).to_string()),
        (200, json!({"data": {}}).to_string()),
    ];
    let (base_url, server) = loopback_server(responses);
    let (manifest, mut dispatch, _) = fixture();
    dispatch.producer.callback_audience = base_url.clone();
    dispatch = dispatch.sealed();
    let mut transport = HttpTransport::new(format!("{base_url}/"), "top-secret");

    let registration = transport.register(&dispatch, &manifest).unwrap();
    assert_eq!(registration.registration_id, "registration-1");

    let event = normalize_event(
        &dispatch,
        &EventEnvelope {
            run_id: "forge-run-1".into(),
            seq: 1,
            event_id: "event-1".into(),
            event_schema_version: 1,
            event_type: EventType::RunCompleted,
            payload: json!({}),
            causation_id: None,
            correlation_id: "forge-run-1".into(),
            attempt_id: None,
            recorded_at: "2026-08-28T08:10:00Z".into(),
            previous_hash: ZERO_HASH.into(),
            event_hash: "a".repeat(64),
        },
    )
    .unwrap();
    assert!(transport.submit(&event).unwrap());
    assert!(transport.commands("registration-1", 3).unwrap().is_empty());
    transport
        .acknowledge_command(
            "registration-1",
            &CommandReceipt {
                command_id: "command-1".into(),
                outcome: "accepted".into(),
                reason: None,
                forge_sequence: 1,
                event_hash: "a".repeat(64),
            },
        )
        .unwrap();

    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests.iter().all(|request| {
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer top-secret")
    }));
    assert!(requests[0].starts_with("POST /api/v1/delivery/forge-producers/registrations "));
    assert!(requests[1].contains("/registration-1/events"));
    assert!(requests[2].contains("commands?after=3"));
    assert!(requests[3].contains("command-1/receipt"));
}

#[test]
fn http_transport_refuses_origin_status_connection_and_shape_defects() {
    let (_, dispatch, _) = fixture();
    let mut wrong_origin = HttpTransport::new("http://127.0.0.1:1", "secret");
    assert!(matches!(
        wrong_origin.register(&dispatch, &json!({})),
        Err(BridgeError::Transport(message)) if message.contains("sealed callback audience")
    ));

    let (base_url, server) = loopback_server(vec![(401, json!({"error": "no"}).to_string())]);
    let error = HttpTransport::new(&base_url, "secret")
        .request("GET", "/status", None)
        .unwrap_err();
    assert!(matches!(error, BridgeError::Transport(message) if message == "HTTP 401"));
    server.join().unwrap();

    let (base_url, server) = loopback_server(vec![(200, "not-json".into())]);
    let error = HttpTransport::new(&base_url, "secret")
        .request("POST", "/shape", Some(json!({})))
        .unwrap_err();
    assert!(matches!(error, BridgeError::Transport(message) if message.contains("invalid JSON")));
    server.join().unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let unavailable = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let error = HttpTransport::new(unavailable, "secret")
        .request("GET", "/offline", None)
        .unwrap_err();
    assert!(
        matches!(error, BridgeError::Transport(message) if message.contains("connection failed"))
    );

    assert!(matches!(
        data::<RegistrationState>(json!({"data": {}})),
        Err(BridgeError::Transport(message)) if message.contains("invalid response shape")
    ));
}

#[test]
fn every_http_operation_propagates_transport_refusal() {
    let (base_url, server) = loopback_server(vec![
        (503, "{}".into()),
        (503, "{}".into()),
        (503, "{}".into()),
        (503, "{}".into()),
    ]);
    let (manifest, mut dispatch, _) = fixture();
    dispatch.producer.callback_audience = base_url.clone();
    dispatch = dispatch.sealed();
    let mut transport = HttpTransport::new(base_url, "secret");
    assert!(transport.register(&dispatch, &manifest).is_err());
    let event = normalize_event(&dispatch, &raw_event(EventType::RunCompleted, json!({}))).unwrap();
    assert!(transport.submit(&event).is_err());
    assert!(transport.commands("registration-1", 0).is_err());
    assert!(transport
        .acknowledge_command(
            "registration-1",
            &CommandReceipt {
                command_id: "command-1".into(),
                outcome: "rejected".into(),
                reason: Some("test".into()),
                forge_sequence: 0,
                event_hash: ZERO_HASH.into(),
            },
        )
        .is_err());
    assert_eq!(server.join().unwrap().len(), 4);
}

fn raw_event(event_type: EventType, payload: Value) -> EventEnvelope {
    EventEnvelope {
        run_id: "forge-run-1".into(),
        seq: 1,
        event_id: "event-1".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "forge-run-1".into(),
        attempt_id: None,
        recorded_at: "2026-08-28T08:10:00Z".into(),
        previous_hash: ZERO_HASH.into(),
        event_hash: "a".repeat(64),
    }
}

#[test]
fn normalization_covers_the_closed_event_vocabulary_and_redaction_edges() {
    let (_, dispatch, _) = fixture();
    let cases = [
        (
            EventType::RunStarted,
            json!({"manifest": {"private": true}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e", "phase": "work", "seat": "worker"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a", "driver": "fake"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e", "attempt_id": "a", "result": {"secret": true}}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "e", "attempt_id": "a", "error": "private"}),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "e", "attempt_id": "a", "reason": "private"}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "work", "result": "complete", "next": "done"}),
        ),
        (
            EventType::OperatorCommanded,
            json!({"command_id": "c", "command": "retry", "operator": "user"}),
        ),
        (
            EventType::OperatorAccepted,
            json!({"command_id": "c", "operator": "user"}),
        ),
        (
            EventType::OperatorRejected,
            json!({"command_id": "c", "operator": "user"}),
        ),
        (EventType::RunParked, json!({"reason": "private"})),
        (EventType::RunStopped, json!({"reason": "private"})),
        (EventType::RunStopped, json!({})),
        (EventType::RunCompleted, json!({})),
    ];
    for (event_type, payload) in cases {
        let normalized = normalize_event(&dispatch, &raw_event(event_type, payload)).unwrap();
        assert_eq!(normalized.forge_sequence, 1);
        assert_eq!(
            normalized.payload_digest,
            canonical::sha256_hex(&normalized.payload)
        );
    }

    let checkpoint = safe_checkpoint(&json!({
        "step": "x".repeat(100),
        "turn": 2,
        "target": "/private/path",
        "session_ref": "private-session",
        "cache_write_tokens": 7,
        "total_cost_usd": 1.25,
    }));
    assert_eq!(checkpoint["step"].as_str().unwrap().chars().count(), 80);
    assert_eq!(checkpoint["turn"], 2);
    assert_eq!(checkpoint["cache_write_tokens"], 7);
    assert_eq!(checkpoint["target_state"], "withheld-private-path");
    assert_eq!(checkpoint["session_reference_state"], "observed-redacted");
    assert_eq!(checkpoint["forge_observed_cost_usd"], 1.25);
    assert_eq!(
        safe_checkpoint(&json!({"total_cost_usd": -1.0})),
        json!({"state":"withheld", "reason":"no-policy-permitted-checkpoint-fields"})
    );
    assert_eq!(
        safe_checkpoint(&json!({"step":"transcript", "transcript": {
            "kind":"dsh-session", "locator":"sessions/brokkr/seat-private",
            "home":"/private/.dsh"
        }})),
        json!({"step":"transcript", "session_reference_state":"observed-redacted"})
    );

    let mut forbidden = dispatch.clone();
    forbidden.allowed_effects.clear();
    assert!(matches!(
        normalize_event(&forbidden, &raw_event(EventType::RunCompleted, json!({}))),
        Err(BridgeError::EffectNotAllowed)
    ));

    let mut free = dispatch;
    free.budget.cost_state = "not-applicable".into();
    let normalized =
        normalize_event(&free, &raw_event(EventType::RunCompleted, json!({}))).unwrap();
    assert_eq!(normalized.cost.state, "not-applicable");
    assert!(normalized.terminal_identity.is_some());

    assert_eq!(
        selected(
            &json!({"flag": true, "count": 2, "none": null}),
            &["flag", "count", "none"]
        ),
        json!({"flag": true, "count": 2, "none": null})
    );
}

#[test]
fn http_transport_executes_the_full_bridge_sync() {
    let responses = vec![
        (
            200,
            json!({"data": {
                "registration_id": "registration-1",
                "status": "active",
                "last_forge_sequence": 0,
                "last_event_hash": ZERO_HASH,
            }})
            .to_string(),
        ),
        (200, json!({"data": {"replayed": false}}).to_string()),
        (200, json!({"data": []}).to_string()),
    ];
    let (base_url, server) = loopback_server(responses);
    let (old_manifest, mut dispatch, now) = fixture();
    dispatch.producer.callback_audience = base_url.clone();
    dispatch = dispatch.sealed();
    let bundle = bundle_manifest_from_run(&old_manifest).unwrap();
    let manifest = build_run_manifest_v2(&bundle, dispatch).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store
        .create_run("forge-run-1", "084-f", "fast", &manifest)
        .unwrap();
    store
        .append_next(
            "forge-run-1",
            EventType::RunCompleted,
            json!({}),
            None,
            None,
        )
        .unwrap();

    let report = Bridge::new(HttpTransport::new(base_url, "secret"))
        .sync_once(&mut store, "forge-run-1", now, 0)
        .unwrap();
    assert_eq!(report.submitted, 1);
    assert_eq!(report.last_forge_sequence, 1);
    assert_eq!(server.join().unwrap().len(), 3);
}

#[test]
fn limiter_and_event_size_boundaries_are_literal() {
    let mut bridge = Bridge::new(MockTransport::default());
    bridge
        .event_times
        .push_back(Instant::now() - Duration::from_secs(11));
    bridge.await_event_slot(1);
    assert_eq!(bridge.event_times.len(), 1);

    bridge.event_times.clear();
    bridge
        .event_times
        .push_back(Instant::now() - Duration::from_secs(10) + Duration::from_millis(2));
    bridge.await_event_slot(1);
    assert_eq!(bridge.event_times.len(), 1);

    let (_, mut dispatch, _) = fixture();
    dispatch.bounds.max_event_bytes = 1;
    assert!(matches!(
        normalize_event(&dispatch, &raw_event(EventType::RunCompleted, json!({}))),
        Err(BridgeError::EventTooLarge)
    ));
}

fn store_with_manifest(manifest: &Value, run_id: &str) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    store.create_run(run_id, "084-f", "fast", manifest).unwrap();
    (dir, store)
}

fn park(store: &mut Store) {
    for (event_type, payload, attempt_id) in [
        (EventType::RunStarted, json!({"manifest": {}}), None),
        (EventType::PhaseEntered, json!({"phase": "work"}), None),
        (
            EventType::EffectRequested,
            json!({"effect_id": "effect-1", "seat": "worker"}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect-1", "attempt_id": "attempt-1"}),
            Some("attempt-1".into()),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "effect-1", "reason": "lost"}),
            Some("attempt-1".into()),
        ),
        (EventType::RunParked, json!({"reason": "lost"}), None),
    ] {
        store
            .append_next("forge-run-1", event_type, payload, None, attempt_id)
            .unwrap();
    }
}

#[test]
fn sync_refuses_every_registration_and_authority_mismatch() {
    let (manifest, _, now) = fixture();

    let (_, mut unbound) = store_with_manifest(&json!({"bundle_name": "fast"}), "r");
    assert!(matches!(
        Bridge::new(MockTransport::default()).sync_once(&mut unbound, "r", now, 0),
        Err(BridgeError::UnboundRun)
    ));

    let (_, mut wrong_run) = store_with_manifest(&manifest, "different-run");
    assert!(matches!(
        Bridge::new(MockTransport::default()).sync_once(&mut wrong_run, "different-run", now, 0),
        Err(BridgeError::RegistrationMismatch)
    ));

    for state in [
        RegistrationState {
            registration_id: "wrong-registration".into(),
            status: "active".into(),
            last_forge_sequence: 0,
            last_event_hash: ZERO_HASH.into(),
        },
        RegistrationState {
            registration_id: "registration-1".into(),
            status: "active".into(),
            last_forge_sequence: 1,
            last_event_hash: ZERO_HASH.into(),
        },
        RegistrationState {
            registration_id: "registration-1".into(),
            status: "terminal".into(),
            last_forge_sequence: 0,
            last_event_hash: ZERO_HASH.into(),
        },
        RegistrationState {
            registration_id: "registration-1".into(),
            status: "revoked".into(),
            last_forge_sequence: 0,
            last_event_hash: ZERO_HASH.into(),
        },
    ] {
        let (_, mut store) = store_with_manifest(&manifest, "forge-run-1");
        if state.status == "terminal" {
            store
                .append_next(
                    "forge-run-1",
                    EventType::RunCompleted,
                    json!({}),
                    None,
                    None,
                )
                .unwrap();
        }
        let error = Bridge::new(MockTransport {
            state: Some(state),
            ..Default::default()
        })
        .sync_once(&mut store, "forge-run-1", now, 0)
        .unwrap_err();
        assert!(matches!(
            error,
            BridgeError::RegistrationMismatch | BridgeError::AuthorityInactive(_)
        ));
    }
}

#[test]
fn command_application_errors_propagate_and_command_events_can_replay() {
    let (manifest, _, now) = fixture();
    let (_, mut malformed) = store_with_manifest(&manifest, "forge-run-1");
    let invalid = malformed
        .append_next(
            "forge-run-1",
            EventType::PhaseEntered,
            json!({"phase":"work"}),
            None,
            None,
        )
        .unwrap();
    let transport = MockTransport {
        commands: vec![ProducerCommand {
            cursor: 1,
            id: "command-invalid".into(),
            command: "stop".into(),
            expected_forge_sequence: invalid.seq,
            expected_event_hash: invalid.event_hash,
            actor: "operator".into(),
            reason: "stop".into(),
        }],
        ..Default::default()
    };
    assert!(matches!(
        Bridge::new(transport).sync_once(&mut malformed, "forge-run-1", now, 0),
        Err(BridgeError::Runtime(_))
    ));

    let (_, mut parked) = store_with_manifest(&manifest, "forge-run-1");
    park(&mut parked);
    let (head_seq, head_hash) = parked.head_hash("forge-run-1").unwrap();
    let transport = MockTransport {
        commands: vec![ProducerCommand {
            cursor: 1,
            id: "command-replayed".into(),
            command: "stop".into(),
            expected_forge_sequence: head_seq,
            expected_event_hash: head_hash,
            actor: "operator".into(),
            reason: "stop".into(),
        }],
        replay_all: true,
        ..Default::default()
    };
    let report = Bridge::new(transport)
        .sync_once(&mut parked, "forge-run-1", now, 0)
        .unwrap();
    assert!(report.replayed >= 2);
}

#[test]
fn sync_replays_and_receipts_cover_rejected_and_cursor_refusals() {
    let (manifest, _, now) = fixture();
    let (_, mut store) = store_with_manifest(&manifest, "forge-run-1");
    let appended = store
        .append_next(
            "forge-run-1",
            EventType::RunCompleted,
            json!({}),
            None,
            None,
        )
        .unwrap();
    let replay =
        normalize_event(&dispatch_from_run(&manifest).unwrap().unwrap(), &appended).unwrap();
    let report = Bridge::new(MockTransport {
        events: vec![replay],
        ..Default::default()
    })
    .sync_once(&mut store, "forge-run-1", now, 0)
    .unwrap();
    assert_eq!(report.replayed, 1);

    let (_, mut store) = store_with_manifest(&manifest, "forge-run-1");
    park(&mut store);
    let error = Bridge::new(MockTransport {
        commands: ["first", "duplicate"]
            .into_iter()
            .map(|id| ProducerCommand {
                cursor: 1,
                id: id.into(),
                command: "stop".into(),
                expected_forge_sequence: 99,
                expected_event_hash: "f".repeat(64),
                actor: "user:operator".into(),
                reason: "duplicate cursor".into(),
            })
            .collect(),
        ..Default::default()
    })
    .sync_once(&mut store, "forge-run-1", now, 0)
    .unwrap_err();
    assert!(
        matches!(error, BridgeError::RegistrationMismatch),
        "unexpected error: {error:?}"
    );

    let (_, mut store) = store_with_manifest(&manifest, "forge-run-1");
    park(&mut store);
    let transport = MockTransport {
        commands: vec![ProducerCommand {
            cursor: 1,
            id: "stale-command".into(),
            command: "stop".into(),
            expected_forge_sequence: 99,
            expected_event_hash: "f".repeat(64),
            actor: "user:operator".into(),
            reason: "stale".into(),
        }],
        ..Default::default()
    };
    let mut bridge = Bridge::new(transport);
    bridge.sync_once(&mut store, "forge-run-1", now, 0).unwrap();
    let transport = bridge.into_transport();
    assert_eq!(transport.receipts[0].outcome, "rejected");
    assert!(transport.receipts[0].reason.is_some());
}
