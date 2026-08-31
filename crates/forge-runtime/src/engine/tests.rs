use super::*;
use crate::bundle::{Limits, Seat};
use forge_core::canonical::ZERO_HASH;
use forge_core::dispatch::PRODUCER_EFFECTS;
use forge_core::policy::Machine;
use forge_protocol::{Body, Message, ResultStatus};
use std::collections::BTreeMap;
use time::format_description::well_known::Rfc3339;

fn machine() -> Machine {
    Machine::from_table(&json!({
        "phases":["work", "review", "ship", "done", "stop"],
        "initial":"work",
        "terminal":["done", "stop"],
        "rules":[
            {"id":"WORK", "from":"work", "result":"complete", "next":"review", "reason":"work"},
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"ship", "reason":"review"},
            {"id":"SHIP", "from":"ship", "result":"shipped", "next":"done", "reason":"ship"}
        ]
    }))
    .unwrap()
}

fn single_body(command: Vec<String>) -> SeatBody {
    SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command,
        confine: None,
        candidates: Vec::new(),
    }
}

fn bundle(dir: &Path, body: SeatBody) -> Bundle {
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        Seat {
            results: vec!["complete".into()],
            limits: Limits {
                max_attempts: 2,
                timeout_seconds: 2,
            },
            inputs: vec!["fixes_applied".into()],
            secrets: Vec::new(),
            body,
        },
    );
    for (phase, result) in [("review", "clean"), ("ship", "shipped")] {
        seats.insert(
            phase.into(),
            Seat {
                results: vec![result.into()],
                limits: Limits::default(),
                inputs: Vec::new(),
                secrets: Vec::new(),
                body: single_body(vec!["missing-driver".into()]),
            },
        );
    }
    Bundle {
        name: "test".into(),
        dir: dir.to_path_buf(),
        roots: vec![dir.to_path_buf()],
        chain: Vec::new(),
        machine: machine(),
        seats,
        manifest: json!({
            "engine":ENGINE_VERSION, "event_schema":1, "database_schema":1,
            "driver_protocol":1, "bundle_name":"test",
            "files":{"bundle.json":"a".repeat(64)}
        }),
        protected_phase: "review".into(),
    }
}

fn engine(body: SeatBody) -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let engine = Engine::start(
        store,
        bundle(dir.path(), body),
        "Feature: exact!",
        Some(dir.path().join("work")),
    )
    .unwrap();
    (dir, engine)
}

fn state(phase: Option<&str>, cursor: Cursor) -> RunState {
    RunState {
        run_id: "run".into(),
        seq: 1,
        last_hash: ZERO_HASH.into(),
        status: Status::Running,
        phase: phase.map(str::to_string),
        cursor,
        consecutive_failures: BTreeMap::new(),
        reviewed_heads: None,
        last_decision: None,
        park_reason: None,
        feature: Some("feature".into()),
        pending_command: None,
        riding_stop: false,
    }
}

fn report(outcome: AttemptOutcome, stderr: &str) -> AttemptReport {
    AttemptReport {
        outcome,
        session_ref: Some("session".into()),
        checkpoints: vec![json!({"step":"inner"})],
        stderr: stderr.into(),
        accepted: true,
    }
}

fn wire(body: Body) -> String {
    serde_json::to_string(&Message::new(body)).unwrap()
}

fn driver_command(effect_id: &str, attempt_id: &str, outcome: AttemptOutcome) -> Vec<String> {
    let capabilities = wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec![],
    });
    let accepted = wire(Body::Accepted {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        session_ref: Some("session".into()),
    });
    let checkpoint = wire(Body::Checkpoint {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        data: json!({"step":"live"}),
    });
    let terminal = match outcome {
        AttemptOutcome::Succeeded { result } => Some(wire(Body::Result {
            effect_id: effect_id.into(),
            attempt_id: attempt_id.into(),
            status: ResultStatus::Succeeded,
            result: Some(result),
            error: None,
        })),
        AttemptOutcome::Failed { error } => Some(wire(Body::Result {
            effect_id: effect_id.into(),
            attempt_id: attempt_id.into(),
            status: ResultStatus::Failed,
            result: None,
            error: Some(error),
        })),
        AttemptOutcome::Indeterminate { .. } => None,
    };
    let mut script = format!(
        "read line; printf '%s\\n' '{capabilities}'; read line; printf '%s\\n' '{accepted}'; printf '%s\\n' '{checkpoint}'"
    );
    if let Some(terminal) = terminal {
        script.push_str(&format!("; printf '%s\\n' '{terminal}'; read line"));
    }
    vec!["sh".into(), "-c".into(), script]
}

fn event(event_type: EventType, payload: Value) -> EventEnvelope {
    EventEnvelope {
        run_id: "run".into(),
        seq: 2,
        event_id: "event".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: None,
        recorded_at: "2026-08-28T00:00:00Z".into(),
        previous_hash: ZERO_HASH.into(),
        event_hash: "a".repeat(64),
    }
}

fn dispatch(bundle: &Bundle) -> DispatchEnvelopeV2 {
    let now = time::OffsetDateTime::now_utc();
    serde_json::from_value::<DispatchEnvelopeV2>(json!({
        "schema":"forge-dispatch/v2", "envelope_id":"envelope", "forge_run_id":"bound-run",
        "issued_at":(now-time::Duration::minutes(1)).format(&Rfc3339).unwrap(),
        "expires_at":(now+time::Duration::minutes(5)).format(&Rfc3339).unwrap(),
        "canonical_digest":"",
        "looper":{"organization_id":"org","product_id":"product","story_id":"story",
            "delivery_run_id":"delivery","request_grant_id":"grant","feature_path":"feature",
            "immutable_inputs_sha256":"a".repeat(64)},
        "actor":{"principal_kind":"api_key","principal_id":"key","actor_kind":"service",
            "actor_id":"forge","accountable_operator_id":"operator","authority_source":"looper-grant",
            "operating_profile":"bounded"},
        "repository":{"owner":"owner","name":"repo","base_sha":"b".repeat(64),
            "candidate_sha":null,"workspace_class":"isolated","target_environment":"dogfood"},
        "recipe":{"name":"test","compiled_sha256":bundle.manifest_digest()},
        "budget":{"lane_tally_run_id":"lane","reservation_id":null,"cost_state":"known",
            "ceiling_microunits":1000,"currency":"USD"},
        "producer":{"registration_id":"registration","token_reference":"key",
            "callback_audience":"https://dogfood.example","accepting_service_id":"looper-api",
            "runtime_id":"runtime","producer_release":"forge@test","protocol_version":1,
            "starting_cursor":0},
        "allowed_effects":PRODUCER_EFFECTS,"forbidden_actions":["grant_create","grant_widen",
            "artifact_decide","workflow_advance","release_promote"],
        "bounds":{"max_attempts":3,"max_parallel_effects":4,"max_event_bytes":65536,
            "max_events_per_ten_seconds":40,"replay_retention_seconds":604800,
            "safe_stop":"boundary","cancellation":"fenced"},
        "evidence_requirements":["ordered_hash_chain"],"attestation_requirement":"self_reported"
    }))
    .unwrap()
    .sealed()
}

#[test]
fn dispatch_bounds_cover_single_panel_sequence_defaults_and_refusals() {
    let dir = tempfile::tempdir().unwrap();
    let single = bundle(dir.path(), single_body(vec!["driver".into()]));
    let base = dispatch(&single);
    assert_eq!(verify_dispatch_bundle_bounds(&base, &single), Ok(()));

    let mut empty = single.clone();
    empty.seats.clear();
    assert_eq!(verify_dispatch_bundle_bounds(&base, &empty), Ok(()));

    let members = vec![
        PanelMember {
            name: "one".into(),
            role_path: "role".into(),
            command: vec!["driver".into()],
            confine: None,
            candidates: Vec::new(),
        },
        PanelMember {
            name: "two".into(),
            role_path: "role".into(),
            command: vec!["driver".into()],
            confine: None,
            candidates: Vec::new(),
        },
    ];
    let panel = bundle(
        dir.path(),
        SeatBody::Panel {
            members: members.clone(),
            aggregate: Aggregate::UnanimousPass,
        },
    );
    assert_eq!(verify_dispatch_bundle_bounds(&base, &panel), Ok(()));

    let sequence = bundle(
        dir.path(),
        SeatBody::Sequence {
            steps: vec![
                SequenceStep {
                    name: "single".into(),
                    body: StepBody::Single {
                        role_path: "role".into(),
                        command: vec!["driver".into()],
                        confine: None,
                        candidates: Vec::new(),
                    },
                },
                SequenceStep {
                    name: "panel".into(),
                    body: StepBody::Panel {
                        members,
                        aggregate: Aggregate::UnanimousPass,
                    },
                },
            ],
        },
    );
    assert_eq!(verify_dispatch_bundle_bounds(&base, &sequence), Ok(()));

    let mut too_many_attempts = single.clone();
    too_many_attempts
        .seats
        .get_mut("work")
        .unwrap()
        .limits
        .max_attempts = 4;
    assert_eq!(
        verify_dispatch_bundle_bounds(&base, &too_many_attempts),
        Err(DispatchError::UnsafeBounds)
    );
    let mut narrow = base;
    narrow.bounds.max_parallel_effects = 1;
    assert_eq!(
        verify_dispatch_bundle_bounds(&narrow, &panel),
        Err(DispatchError::UnsafeBounds)
    );
}

#[test]
fn bound_start_resume_and_manifest_differences_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = bundle(dir.path(), single_body(vec!["driver".into()]));
    let store = Store::open(&dir.path().join("bound.db")).unwrap();
    let dispatch = dispatch(&bundle);
    let engine = Engine::start_with_dispatch(
        store,
        bundle.clone(),
        "bound",
        Some(dir.path().into()),
        dispatch,
    )
    .unwrap();
    assert_eq!(engine.run_id, "bound-run");
    let resumed = Engine::resume(
        engine.store,
        bundle.clone(),
        "bound-run",
        Some(dir.path().into()),
    )
    .unwrap();
    assert_eq!(resumed.feature, "bound");

    let mut changed = bundle.clone();
    changed.manifest["files"] = json!({"changed":"b".repeat(64)});
    let error = match Engine::resume(resumed.store, changed, "bound-run", None) {
        Ok(_) => panic!("changed dispatch-bound bundle must refuse"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        EngineError::Dispatch(DispatchError::RecipeMismatch)
    ));

    assert_eq!(
        manifest_diff(
            &json!({"files":{"gone":"a","changed":"a"}}),
            &json!({"files":{"added":"b","changed":"b"}})
        ),
        "changed: changed, missing: gone, added: added"
    );
    assert!(manifest_diff(&json!({"engine":"old"}), &json!({"engine":"new"})).contains("non-file"));
}

/// A two-layer recipe library on disk: `base`, and a `derived` that
/// extends it and replaces only the review seat. Returns the leaf
/// directory, which is what a run is started from.
fn composed_library(library: &Path) -> PathBuf {
    let policy = json!({
        "phases": ["work", "review", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [
            {"id":"WORK", "from":"work", "result":"complete", "next":"review", "reason":"work"},
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"done", "reason":"review"},
        ],
    });
    let seat = |result: &str| {
        json!({
            "results": [result],
            "role": "roles/role.md",
            "driver": {"command": ["driver"]},
        })
    };
    for (name, document) in [
        (
            "base",
            json!({"name": "base", "policy": "policy.json",
                   "seats": {"work": seat("complete"), "review": seat("clean")}}),
        ),
        (
            "derived",
            json!({"name": "derived", "extends": "base",
                   "override": {"seats": ["review"]},
                   "seats": {"review": seat("clean")}}),
        ),
    ] {
        let dir = library.join(name);
        std::fs::create_dir_all(dir.join("roles")).unwrap();
        std::fs::write(dir.join("roles/role.md"), format!("# {name}\n")).unwrap();
        std::fs::write(
            dir.join("bundle.json"),
            serde_json::to_vec(&document).unwrap(),
        )
        .unwrap();
        if name == "base" {
            std::fs::write(
                dir.join("policy.json"),
                serde_json::to_vec(&policy).unwrap(),
            )
            .unwrap();
        }
    }
    library.join("derived")
}

#[test]
fn a_composed_run_resumes_and_refuses_when_its_base_moved() {
    // Decision 0017's pinning, end to end. The chain rides inside the
    // manifest's `files` map, so it survives the v2 dispatch round-trip
    // — `bundle_manifest_from_run` rebuilds from six enumerated keys and
    // `dispatch_from_run` re-hashes that reconstruction against the
    // stored `bundle_sha256`. A top-level member would not.
    let dir = tempfile::tempdir().unwrap();
    let leaf = composed_library(dir.path());
    let composed = Bundle::compile(&leaf).unwrap();
    assert_eq!(composed.chain.len(), 1);
    assert!(composed.manifest["files"]
        .as_object()
        .unwrap()
        .contains_key("@compose/0000/base"));

    let store = Store::open(&dir.path().join("composed.db")).unwrap();
    let envelope = dispatch(&composed);
    let engine = Engine::start_with_dispatch(
        store,
        composed.clone(),
        "composed",
        Some(dir.path().into()),
        envelope,
    )
    .unwrap();
    let resumed = Engine::resume(
        engine.store,
        composed.clone(),
        "bound-run",
        Some(dir.path().into()),
    )
    .unwrap();
    assert_eq!(resumed.feature, "composed");

    // A base that moved under a plain run surfaces BY NAME: resume
    // recompiles, re-resolves from the same library, and the existing
    // digest-mismatch refusal reports which `files` entry moved.
    let plain = Store::open(&dir.path().join("plain.db")).unwrap();
    let engine = Engine::start(plain, composed, "plain", Some(dir.path().into())).unwrap();
    let run_id = engine.run_id.clone();
    std::fs::write(dir.path().join("base/roles/role.md"), "# moved\n").unwrap();
    let moved = Bundle::compile(&leaf).unwrap();
    match Engine::resume(engine.store, moved, &run_id, None) {
        Ok(_) => panic!("a changed base must refuse"),
        Err(EngineError::ManifestMismatch { detail, .. }) => {
            assert_eq!(detail, "changed: @compose/0000/base")
        }
        Err(other) => panic!("expected a manifest mismatch: {other}"),
    }
}

#[test]
fn request_finish_input_and_execute_refusals_are_journaled() {
    let (_dir, mut engine) = engine(single_body(vec!["missing-driver".into()]));
    assert!(engine
        .request_or_finish(&state(None, Cursor::RequestEffect))
        .unwrap_err()
        .to_string()
        .contains("no phase"));

    let mut stop = state(Some("stop"), Cursor::RequestEffect);
    stop.last_decision = Some(json!({"rule_id":"SECURITY"}));
    engine.request_or_finish(&stop).unwrap();
    engine
        .request_or_finish(&state(Some("done"), Cursor::RequestEffect))
        .unwrap();
    engine
        .request_or_finish(&state(Some("work"), Cursor::RequestEffect))
        .unwrap();
    assert!(engine
        .seat_input(&state(Some("work"), Cursor::Idle), "missing", "effect")
        .unwrap_err()
        .to_string()
        .contains("no seat"));

    assert!(engine
        .execute(&[], &state(None, Cursor::Idle), "effect", "work")
        .unwrap_err()
        .to_string()
        .contains("without a phase"));
    assert!(engine
        .execute(&[], &state(Some("work"), Cursor::Idle), "effect", "work")
        .unwrap_err()
        .to_string()
        .contains("no requested event"));

    let requested = event(
        EventType::EffectRequested,
        json!({"effect_id":"effect", "input_digest":"wrong"}),
    );
    engine
        .execute(
            &[requested],
            &state(Some("work"), Cursor::Idle),
            "effect",
            "work",
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(events.iter().any(|event| {
        event.event_type == EventType::EffectFailed
            && event.payload["error"]
                .as_str()
                .unwrap()
                .contains("does not match")
    }));
}

#[test]
fn single_conclusion_driver_and_checkpoint_failures_cover_every_outcome() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine
        .conclude_single(
            "e1",
            "a1",
            DriverRun::SpawnFailed("spawn".into()),
            &Selection::new(),
        )
        .unwrap();
    engine
        .conclude_single(
            "e2",
            "a2",
            DriverRun::Ran(report(
                AttemptOutcome::Succeeded {
                    result: json!({"result":"complete"}),
                },
                "",
            )),
            &Selection::new(),
        )
        .unwrap();
    engine
        .conclude_single(
            "e3",
            "a3",
            DriverRun::Ran(report(
                AttemptOutcome::Failed {
                    error: "failed".into(),
                },
                "stderr",
            )),
            &Selection::new(),
        )
        .unwrap();
    engine
        .conclude_single(
            "e4",
            "a4",
            DriverRun::Ran(report(
                AttemptOutcome::Indeterminate {
                    reason: "lost".into(),
                },
                "stderr",
            )),
            &Selection::new(),
        )
        .unwrap();

    assert!(matches!(
        engine
            .run_driver(
                "effect",
                "attempt",
                "work",
                &["missing-driver".into()],
                json!({}),
                std::time::Duration::from_secs(1),
                None,
            )
            .unwrap(),
        DriverRun::SpawnFailed(_)
    ));
    let command = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result":"complete"}),
        },
    );
    assert!(matches!(
        engine
            .run_driver(
                "effect",
                "attempt",
                "work",
                &command,
                json!({}),
                std::time::Duration::from_secs(2),
                Some("step"),
            )
            .unwrap(),
        DriverRun::Ran(_)
    ));

    let saved_run = engine.run_id.clone();
    engine.run_id = "missing-run".into();
    assert!(engine
        .run_driver(
            "effect",
            "attempt",
            "work",
            &command,
            json!({}),
            std::time::Duration::from_secs(2),
            None,
        )
        .is_err());
    engine.run_id = saved_run;
}

#[test]
fn panel_sequence_and_aggregation_cover_all_terminal_shapes() {
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    let outcomes = vec![
        (
            "ok".into(),
            report(
                AttemptOutcome::Succeeded {
                    result: json!({"result":"pass", "notes":"ok"}),
                },
                "",
            ),
        ),
        (
            "bad".into(),
            report(
                AttemptOutcome::Failed {
                    error: "bad".into(),
                },
                "",
            ),
        ),
        (
            "lost".into(),
            report(
                AttemptOutcome::Indeterminate {
                    reason: "lost".into(),
                },
                "",
            ),
        ),
    ];
    engine
        .journal_panel_members("effect", "attempt", &outcomes, "step:")
        .unwrap();
    assert!(matches!(
        panel_outcome(Aggregate::UnanimousPass, outcomes.clone()),
        AttemptOutcome::Indeterminate { .. }
    ));
    assert!(matches!(
        panel_outcome(Aggregate::UnanimousPass, outcomes[..2].to_vec()),
        AttemptOutcome::Failed { .. }
    ));
    assert!(matches!(
        panel_outcome(Aggregate::UnanimousPass, outcomes[..1].to_vec()),
        AttemptOutcome::Succeeded { .. }
    ));

    assert_eq!(
        aggregate_results(
            Aggregate::UnanimousPass,
            &[("bad".into(), json!({"notes":"missing result"}))]
        )["result"],
        "__member-schema-invalid__"
    );
    assert_eq!(
        aggregate_results(
            Aggregate::UnanimousPass,
            &[("bad".into(), json!({"result":"invented"}))]
        )["result"],
        "__member-schema-invalid__"
    );
    assert_eq!(
        aggregate_results(
            Aggregate::UnanimousPass,
            &[
                ("one".into(), json!({"result":"pass"})),
                ("two".into(), json!({"result":"fail"}))
            ]
        )["result"],
        "fail"
    );
    let review = aggregate_results(
        Aggregate::ReviewPanel,
        &[
            (
                "clean".into(),
                json!({"result":"clean", "inputs":{"fixes_applied":true}}),
            ),
            (
                "residual".into(),
                json!({"result":"residual", "inputs":{
                    "max_residual_severity":"high", "has_security_residual":true
                }}),
            ),
            ("unknown".into(), json!({"result":"invented"})),
        ],
    );
    assert_eq!(review["result"], "invented");
    assert_eq!(review["inputs"]["max_residual_severity"], "high");
    assert_eq!(review["inputs"]["has_security_residual"], true);
    assert_eq!(review["inputs"]["fixes_applied"], true);
    let unknown_severity = aggregate_results(
        Aggregate::ReviewPanel,
        &[(
            "residual".into(),
            json!({"result":"residual","inputs":{"max_residual_severity":"invented"}}),
        )],
    );
    assert_eq!(unknown_severity["inputs"]["max_residual_severity"], "none");

    assert_eq!(
        tag_member(json!({"step":"x"}), "member")["member"],
        "member"
    );
    assert_eq!(tag_member(json!(2), "member")["value"], 2);
    let mut secret_input = json!({});
    copy_secret_binding_facts(
        &mut secret_input,
        &json!({"secrets":["TOKEN"], "secrets_file":"path"}),
    );
    assert_eq!(secret_input["secrets"], json!(["TOKEN"]));
    let long = format!("start{}", "x".repeat(2200));
    assert_eq!(stderr_tail(&long).chars().count(), 2000);
}

#[test]
fn git_helpers_and_confinement_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .unwrap()
        .success());
    assert_eq!(git_head(dir.path()), None);
    assert!(!git_dirty(dir.path()));
    std::fs::write(dir.path().join("dirty"), "x").unwrap();
    assert!(git_dirty(dir.path()));
    let missing = dir.path().join("missing");
    assert_eq!(git_head(&missing), None);
    assert!(git_dirty(&missing));

    let command = vec!["driver".into()];
    let confined = confined_command(
        &command,
        Some(&Confine {
            image: "image@sha256:abc".into(),
            network: true,
            mounts: vec!["/extra".into()],
        }),
        dir.path(),
        &[dir.path().to_path_buf()],
    );
    assert!(!confined.contains(&"--network=none".to_string()));
    assert!(confined.iter().any(|part| part == "/extra:/extra:ro"));
}

fn member(name: &str, command: Vec<String>) -> PanelMember {
    PanelMember {
        name: name.into(),
        role_path: "role.md".into(),
        command,
        confine: None,
        candidates: Vec::new(),
    }
}

fn panel_input(names: &[&str]) -> Value {
    let members = names
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                json!({"role_path":"role.md", "result_path":format!("{name}.json")}),
            )
        })
        .collect::<Map<String, Value>>();
    json!({
        "feature":"feature", "phase":"work", "workdir":".",
        "allowed_results":["complete"], "context":{}, "members":members,
    })
}

#[test]
fn panel_execution_covers_spawn_failure_indeterminate_and_success_joins() {
    let (_dir, mut failed) = engine(single_body(vec!["driver".into()]));
    failed
        .execute_panel(
            "failed-effect",
            "failed-attempt",
            "work",
            &[member("missing", vec!["missing-driver".into()])],
            Aggregate::UnanimousPass,
            &panel_input(&["missing"]),
            std::time::Duration::from_secs(1),
            &Selection::new(),
        )
        .unwrap();
    assert!(failed
        .store
        .load(&failed.run_id)
        .unwrap()
        .iter()
        .any(|event| {
            event.event_type == EventType::EffectFailed
                && event.payload["error"]
                    .as_str()
                    .unwrap()
                    .contains("did not spawn")
        }));

    let indeterminate_command = driver_command(
        "lost-effect",
        "lost-attempt",
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    );
    let (_dir, mut lost) = engine(single_body(vec!["driver".into()]));
    lost.execute_panel(
        "lost-effect",
        "lost-attempt",
        "work",
        &[member("lost", indeterminate_command)],
        Aggregate::UnanimousPass,
        &panel_input(&["lost"]),
        std::time::Duration::from_secs(2),
        &Selection::new(),
    )
    .unwrap();
    assert!(lost
        .store
        .load(&lost.run_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == EventType::EffectIndeterminate));

    let success_command = driver_command(
        "ok-effect",
        "ok-attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result":"pass"}),
        },
    );
    let (_dir, mut succeeded) = engine(single_body(vec!["driver".into()]));
    succeeded
        .execute_panel(
            "ok-effect",
            "ok-attempt",
            "work",
            &[member("ok", success_command)],
            Aggregate::UnanimousPass,
            &panel_input(&["ok"]),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .unwrap();
    assert!(succeeded
        .store
        .load(&succeeded.run_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded));
}

fn step_input() -> Value {
    json!({
        "feature":"feature", "phase":"work", "workdir":".",
        "allowed_results":["complete"], "context":{},
        "steps":[{"role_path":"role.md", "result_path":"result.json"}],
    })
}

#[test]
fn sequence_execution_covers_spawn_failure_and_indeterminate_terminal_shapes() {
    let failed_step = SequenceStep {
        name: "failed".into(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: vec!["missing-driver".into()],
            confine: None,
            candidates: Vec::new(),
        },
    };
    let (_dir, mut failed) = engine(single_body(vec!["driver".into()]));
    failed
        .execute_sequence(
            "failed-effect",
            "failed-attempt",
            "work",
            &[failed_step],
            &step_input(),
            std::time::Duration::from_secs(1),
            &Selection::new(),
        )
        .unwrap();
    assert!(failed
        .store
        .load(&failed.run_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == EventType::EffectFailed));

    let lost_step = SequenceStep {
        name: "lost".into(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: driver_command(
                "lost-effect",
                "lost-attempt",
                AttemptOutcome::Indeterminate {
                    reason: "lost".into(),
                },
            ),
            confine: None,
            candidates: Vec::new(),
        },
    };
    let (_dir, mut lost) = engine(single_body(vec!["driver".into()]));
    lost.execute_sequence(
        "lost-effect",
        "lost-attempt",
        "work",
        &[lost_step],
        &step_input(),
        std::time::Duration::from_secs(2),
        &Selection::new(),
    )
    .unwrap();
    let event = lost
        .store
        .load(&lost.run_id)
        .unwrap()
        .into_iter()
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .unwrap();
    assert!(event.payload["reason"]
        .as_str()
        .unwrap()
        .contains("stderr tail"));
}

fn git_commit(repo: &Path, message: &str) -> String {
    if !repo.join(".git").exists() {
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo)
            .status()
            .unwrap()
            .success());
        for (key, value) in [("user.name", "Forge Test"), ("user.email", "forge@test")] {
            assert!(Command::new("git")
                .args(["config", key, value])
                .current_dir(repo)
                .status()
                .unwrap()
                .success());
        }
        assert!(Command::new("git")
            .args(["config", "commit.gpgSign", "false"])
            .current_dir(repo)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(repo.join(format!("{message}.txt")), message).unwrap();
    assert!(Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["commit", "-q", "-m", message])
        .current_dir(repo)
        .status()
        .unwrap()
        .success());
    git_head(repo).unwrap()
}

#[test]
fn decide_covers_schema_no_rule_review_head_and_ship_drift() {
    let (dir, mut engine) = engine(single_body(vec!["driver".into()]));
    assert!(engine
        .decide(
            &state(None, Cursor::Idle),
            "effect",
            json!({"result":"complete"})
        )
        .unwrap_err()
        .to_string()
        .contains("without a phase"));
    engine
        .decide(&state(Some("work"), Cursor::Idle), "effect", json!(2))
        .unwrap();
    engine
        .decide(
            &state(Some("work"), Cursor::Idle),
            "effect",
            json!({"notes":"missing result"}),
        )
        .unwrap();

    engine
        .bundle
        .seats
        .get_mut("work")
        .unwrap()
        .results
        .push("unruled".into());
    engine
        .decide(
            &state(Some("work"), Cursor::Idle),
            "effect",
            json!({"result":"unruled"}),
        )
        .unwrap();

    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let reviewed = git_commit(&repo, "reviewed");
    engine.repo = Some(repo.clone());
    engine
        .decide(
            &state(Some("review"), Cursor::Idle),
            "effect",
            json!({"result":"clean"}),
        )
        .unwrap();
    git_commit(&repo, "moved");
    let mut ship = state(Some("ship"), Cursor::Idle);
    ship.reviewed_heads = Some(json!({"repo":reviewed}));
    engine
        .decide(&ship, "effect", json!({"result":"shipped"}))
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(events.iter().any(|event| {
        event.payload["inputs"]["drift_detected"] == true
            && event.payload["inputs"]["dirty_worktrees"] == false
    }));
}

fn parked_store(path: &Path, run_id: &str) -> Store {
    let mut store = Store::open(path).unwrap();
    store
        .create_run(run_id, "feature", "test", &json!({"files":{}}))
        .unwrap();
    for (event_type, payload, attempt_id) in [
        (
            EventType::RunStarted,
            json!({"feature":"feature","manifest":{}}),
            None,
        ),
        (EventType::PhaseEntered, json!({"phase":"work"}), None),
        (
            EventType::EffectRequested,
            json!({"effect_id":"effect","seat":"work"}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect","attempt_id":"attempt"}),
            Some("attempt".into()),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id":"effect","attempt_id":"attempt"}),
            Some("attempt".into()),
        ),
        (EventType::RunParked, json!({"reason":"lost"}), None),
    ] {
        store
            .append_next(run_id, event_type, payload, None, attempt_id)
            .unwrap();
    }
    store
}

#[test]
fn operator_and_fenced_replay_cover_every_disposition() {
    let dir = tempfile::tempdir().unwrap();
    let mut ordinary = parked_store(&dir.path().join("ordinary.db"), "ordinary");
    operator_command(&mut ordinary, "ordinary", "retry", "operator", "reason").unwrap();

    let mut accepted = parked_store(&dir.path().join("accepted.db"), "accepted");
    let (seq, hash) = accepted.head_hash("accepted").unwrap();
    let first = apply_fenced_operator_command(
        &mut accepted,
        "accepted",
        "command",
        "retry",
        "operator",
        "reason",
        seq,
        &hash,
    )
    .unwrap();
    assert!(matches!(first, FencedCommandOutcome::Accepted { .. }));
    let replay = apply_fenced_operator_command(
        &mut accepted,
        "accepted",
        "command",
        "retry",
        "operator",
        "reason",
        seq,
        &hash,
    )
    .unwrap();
    assert!(matches!(replay, FencedCommandOutcome::Accepted { .. }));

    let mut incomplete = parked_store(&dir.path().join("incomplete.db"), "incomplete");
    incomplete
        .append_next(
            "incomplete",
            EventType::OperatorCommanded,
            json!({"command_id":"incomplete-command","command":"retry","args":{},"operator":"operator"}),
            None,
            None,
        )
        .unwrap();
    incomplete
        .append_next(
            "incomplete",
            EventType::OperatorCommanded,
            json!({"command_id":"other-command","command":"retry","args":{},"operator":"operator"}),
            None,
            None,
        )
        .unwrap();
    assert!(matches!(
        apply_fenced_operator_command(
            &mut incomplete,
            "incomplete",
            "incomplete-command",
            "retry",
            "operator",
            "reason",
            0,
            ZERO_HASH,
        )
        .unwrap(),
        FencedCommandOutcome::Rejected { reason, .. } if reason == "incomplete_command_replay"
    ));

    let mut forbidden = parked_store(&dir.path().join("forbidden.db"), "forbidden");
    let (seq, hash) = forbidden.head_hash("forbidden").unwrap();
    assert!(matches!(
        apply_fenced_operator_command(
            &mut forbidden, "forbidden", "bad", "widen", "operator", "reason", seq, &hash,
        )
        .unwrap(),
        FencedCommandOutcome::Rejected { reason, .. } if reason == "command_not_allowed"
    ));

    let mut stale_hash = parked_store(&dir.path().join("stale-hash.db"), "stale-hash");
    let (seq, _) = stale_hash.head_hash("stale-hash").unwrap();
    assert!(matches!(
        apply_fenced_operator_command(
            &mut stale_hash,
            "stale-hash",
            "stale",
            "retry",
            "operator",
            "reason",
            seq,
            &"f".repeat(64),
        )
        .unwrap(),
        FencedCommandOutcome::Rejected { reason, .. } if reason == "stale_cursor"
    ));

    let (_kept, mut running_engine) = engine(single_body(vec!["driver".into()]));
    let (seq, hash) = running_engine
        .store
        .head_hash(&running_engine.run_id)
        .unwrap();
    let run_id = running_engine.run_id.clone();
    assert!(matches!(
        apply_fenced_operator_command(
            &mut running_engine.store, &run_id, "early", "stop", "operator", "reason", seq, &hash,
        )
        .unwrap(),
        FencedCommandOutcome::Rejected { reason, .. } if reason == "run_not_awaiting_operator"
    ));
}

fn fail_event(path: &Path, event_type: &str) {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .execute_batch(&format!(
            "CREATE TRIGGER forced_event_failure BEFORE INSERT ON events
             WHEN instr(NEW.envelope, '{event_type}') > 0
             BEGIN SELECT RAISE(ABORT, 'forced event failure'); END;"
        ))
        .unwrap();
}

fn engine_failing(event_type: &str) -> (tempfile::TempDir, Engine) {
    let (dir, engine) = engine(single_body(vec!["driver".into()]));
    fail_event(&dir.path().join("forge.db"), event_type);
    (dir, engine)
}

#[test]
fn start_append_and_running_cursor_storage_failures_propagate() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("start.db");
    let store = Store::open(&db).unwrap();
    fail_event(&db, "run/started");
    assert!(Engine::start(
        store,
        bundle(dir.path(), single_body(vec!["driver".into()])),
        "feature",
        None,
    )
    .is_err());

    let bound_db = dir.path().join("bound-start.db");
    let store = Store::open(&bound_db).unwrap();
    fail_event(&bound_db, "run/started");
    let bound_bundle = bundle(dir.path(), single_body(vec!["driver".into()]));
    let envelope = dispatch(&bound_bundle);
    assert!(Engine::start_with_dispatch(store, bound_bundle, "feature", None, envelope).is_err());

    let (_kept, mut direct) = engine_failing("phase/entered");
    assert!(direct
        .append(EventType::PhaseEntered, json!({"phase":"work"}), None)
        .is_err());

    let (_kept, mut requested) = engine_failing("effect/requested");
    assert!(requested
        .request_or_finish(&state(Some("work"), Cursor::RequestEffect))
        .is_err());

    let (_kept, mut exhausted) = engine_failing("run/parked");
    let exhausted_state = state(
        Some("work"),
        Cursor::ExecuteEffect {
            effect_id: "effect".into(),
            seat: "work".into(),
            failed_attempts: 2,
        },
    );
    assert!(exhausted.advance_running(&[], exhausted_state).is_err());

    let (_kept, mut in_flight) = engine_failing("effect/indeterminate");
    let in_flight_state = state(
        Some("work"),
        Cursor::EffectInFlight {
            effect_id: "effect".into(),
            attempt_id: "attempt".into(),
            seat: "work".into(),
            failed_attempts: 0,
        },
    );
    assert!(in_flight.advance_running(&[], in_flight_state).is_err());

    let (_kept, mut parked) = engine_failing("run/parked");
    assert!(parked
        .advance_running(
            &[],
            state(
                Some("work"),
                Cursor::Park {
                    reason: "park".into(),
                },
            ),
        )
        .is_err());

    let (_kept, mut stopped) = engine_failing("run/stopped");
    assert!(stopped
        .advance_running(&[], state(Some("work"), Cursor::Stop))
        .is_err());

    let (_kept, mut cursor_shapes) = engine(single_body(vec!["driver".into()]));
    cursor_shapes
        .advance_running(&[], state(Some("work"), Cursor::Stop))
        .unwrap();
    assert!(cursor_shapes
        .advance_running(&[], state(Some("work"), Cursor::Idle))
        .unwrap_err()
        .to_string()
        .contains("terminal idle cursor"));

    for events in [
        vec![event(EventType::PhaseEntered, json!({"phase":"work"}))],
        vec![event(
            EventType::EffectFailed,
            json!({"effect_id":"other","error":"wrong"}),
        )],
        vec![event(
            EventType::EffectFailed,
            json!({"effect_id":"effect"}),
        )],
        vec![event(
            EventType::EffectFailed,
            json!({"effect_id":"effect","error":"recorded"}),
        )],
    ] {
        let (_kept, mut engine) = engine(single_body(vec!["driver".into()]));
        engine
            .advance_running(
                &events,
                state(
                    Some("work"),
                    Cursor::ExecuteEffect {
                        effect_id: "effect".into(),
                        seat: "work".into(),
                        failed_attempts: 2,
                    },
                ),
            )
            .unwrap();
    }
}

#[test]
fn terminal_drive_attempts_anchor_and_reports_anchor_gaps() {
    let (missing_dir, mut missing) = engine(single_body(vec!["driver".into()]));
    missing.repo = Some(missing_dir.path().join("not-a-repository"));
    missing
        .append(EventType::RunCompleted, json!({}), None)
        .unwrap();
    assert_eq!(missing.drive().unwrap().state.status, Status::Completed);

    let (dir, mut anchored) = engine(single_body(vec!["driver".into()]));
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    anchored.repo = Some(repo.clone());
    anchored
        .append(EventType::RunCompleted, json!({}), None)
        .unwrap();
    assert_eq!(anchored.drive().unwrap().state.status, Status::Completed);
    assert!(Command::new("git")
        .args([
            "show-ref",
            "--verify",
            &format!("refs/forge/{}", anchored.run_id)
        ])
        .current_dir(repo)
        .status()
        .unwrap()
        .success());
}

fn requested(engine: &Engine, effect_id: &str) -> EventEnvelope {
    let current = state(Some("work"), Cursor::Idle);
    let input = engine.seat_input(&current, "work", effect_id).unwrap();
    event(
        EventType::EffectRequested,
        json!({
            "effect_id":effect_id,
            "input_digest":forge_core::canonical::sha256_hex(&input),
        }),
    )
}

fn two_checkpoint_command(effect_id: &str, attempt_id: &str) -> Vec<String> {
    let capabilities = wire(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec![],
    });
    let accepted = wire(Body::Accepted {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        session_ref: None,
    });
    let first = wire(Body::Checkpoint {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        data: json!({"step":"first"}),
    });
    let second = wire(Body::Checkpoint {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        data: json!({"step":"second"}),
    });
    let terminal = wire(Body::Result {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        status: ResultStatus::Succeeded,
        result: Some(json!({"result":"complete"})),
        error: None,
    });
    vec![
        "sh".into(),
        "-c".into(),
        format!(
            "read line; printf '%s\\n' '{capabilities}'; read line; printf '%s\\n' \
             '{accepted}'; printf '%s\\n' '{first}'; printf '%s\\n' '{second}'; \
             printf '%s\\n' '{terminal}'; read line"
        ),
    ]
}

#[test]
fn execute_conclusion_and_checkpoint_storage_failures_propagate() {
    let wrong = event(
        EventType::EffectRequested,
        json!({"effect_id":"effect","input_digest":"wrong"}),
    );
    for event_type in ["effect/started", "effect/failed"] {
        let (_kept, mut engine) = engine_failing(event_type);
        assert!(engine
            .execute(
                std::slice::from_ref(&wrong),
                &state(Some("work"), Cursor::Idle),
                "effect",
                "work",
            )
            .is_err());
    }

    let (_kept, mut started) = engine_failing("effect/started");
    let correct = requested(&started, "effect");
    assert!(started
        .execute(
            &[correct],
            &state(Some("work"), Cursor::Idle),
            "effect",
            "work",
        )
        .is_err());

    let command = two_checkpoint_command("effect", "attempt");
    let (dir, mut checkpointed) = engine(SeatBody::Single {
        role_path: "role.md".into(),
        command,
        confine: None,
        candidates: Vec::new(),
    });
    fail_event(&dir.path().join("forge.db"), "effect/checkpointed");
    let correct = requested(&checkpointed, "effect");
    assert!(checkpointed
        .execute(
            &[correct],
            &state(Some("work"), Cursor::Idle),
            "effect",
            "work",
        )
        .is_err());

    let cases = [
        ("effect/failed", DriverRun::SpawnFailed("spawn".into())),
        (
            "effect/succeeded",
            DriverRun::Ran(report(
                AttemptOutcome::Succeeded {
                    result: json!({"result":"complete"}),
                },
                "",
            )),
        ),
        (
            "effect/failed",
            DriverRun::Ran(report(
                AttemptOutcome::Failed {
                    error: "failed".into(),
                },
                "stderr",
            )),
        ),
        (
            "effect/indeterminate",
            DriverRun::Ran(report(
                AttemptOutcome::Indeterminate {
                    reason: "lost".into(),
                },
                "stderr",
            )),
        ),
    ];
    for (event_type, outcome) in cases {
        let (_kept, mut engine) = engine_failing(event_type);
        assert!(engine
            .conclude_single("effect", "attempt", outcome, &Selection::new())
            .is_err());
    }
}

#[test]
fn panel_and_sequence_storage_failures_propagate() {
    let (_kept, mut failed_panel) = engine_failing("effect/failed");
    assert!(failed_panel
        .execute_panel(
            "effect",
            "attempt",
            "work",
            &[member("missing", vec!["missing-driver".into()])],
            Aggregate::UnanimousPass,
            &panel_input(&["missing"]),
            std::time::Duration::from_secs(1),
            &Selection::new(),
        )
        .is_err());

    let lost_command = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    );
    let (_kept, mut lost_panel) = engine_failing("effect/indeterminate");
    assert!(lost_panel
        .execute_panel(
            "effect",
            "attempt",
            "work",
            &[member("lost", lost_command)],
            Aggregate::UnanimousPass,
            &panel_input(&["lost"]),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .is_err());

    let success_command = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result":"pass"}),
        },
    );
    let (_kept, mut ok_panel) = engine_failing("effect/succeeded");
    assert!(ok_panel
        .execute_panel(
            "effect",
            "attempt",
            "work",
            &[member("ok", success_command)],
            Aggregate::UnanimousPass,
            &panel_input(&["ok"]),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .is_err());

    let (dir, mut live_panel) = engine(single_body(vec!["driver".into()]));
    fail_event(&dir.path().join("forge.db"), "effect/checkpointed");
    let members = [member(
        "member",
        two_checkpoint_command("effect", "attempt"),
    )];
    let input = panel_input(&["member"]);
    let runs = live_panel.member_runs(
        "work",
        &members,
        &input["members"],
        &input,
        &input["context"],
        &Selection::new(),
        "",
    );
    assert!(live_panel
        .run_panel(
            "effect",
            "attempt",
            &runs,
            std::time::Duration::from_secs(2),
            "",
        )
        .is_err());

    let (_kept, mut journal) = engine_failing("effect/checkpointed");
    assert!(journal
        .journal_panel_members(
            "effect",
            "attempt",
            &[(
                "member".into(),
                report(
                    AttemptOutcome::Succeeded {
                        result: json!({"result":"pass"}),
                    },
                    "",
                ),
            )],
            "",
        )
        .is_err());

    let failed_step = SequenceStep {
        name: "failed".into(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: vec!["missing-driver".into()],
            confine: None,
            candidates: Vec::new(),
        },
    };
    let (_kept, mut failed_sequence) = engine_failing("effect/failed");
    assert!(failed_sequence
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[failed_step],
            &step_input(),
            std::time::Duration::from_secs(1),
            &Selection::new(),
        )
        .is_err());

    let lost_step = SequenceStep {
        name: "lost".into(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Indeterminate {
                    reason: "lost".into(),
                },
            ),
            confine: None,
            candidates: Vec::new(),
        },
    };
    let (_kept, mut lost_sequence) = engine_failing("effect/indeterminate");
    assert!(lost_sequence
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[lost_step],
            &step_input(),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .is_err());

    let ok_step = SequenceStep {
        name: "ok".into(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"complete"}),
                },
            ),
            confine: None,
            candidates: Vec::new(),
        },
    };
    let (_kept, mut ok_sequence) = engine_failing("effect/succeeded");
    assert!(ok_sequence
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[ok_step],
            &step_input(),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .is_err());

    let checkpoint_step = SequenceStep {
        name: "checkpoint".into(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: two_checkpoint_command("effect", "attempt"),
            confine: None,
            candidates: Vec::new(),
        },
    };
    let (_kept, mut checkpoint_sequence) = engine_failing("effect/checkpointed");
    assert!(checkpoint_sequence
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[checkpoint_step],
            &step_input(),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .is_err());

    let steps = [
        SequenceStep {
            name: "first".into(),
            body: StepBody::Single {
                role_path: "role.md".into(),
                command: driver_command(
                    "effect",
                    "attempt",
                    AttemptOutcome::Succeeded {
                        result: json!({"result":"complete"}),
                    },
                ),
                confine: None,
                candidates: Vec::new(),
            },
        },
        SequenceStep {
            name: "second".into(),
            body: StepBody::Single {
                role_path: "role.md".into(),
                command: vec!["missing-driver".into()],
                confine: None,
                candidates: Vec::new(),
            },
        },
    ];
    let input = json!({
        "feature":"feature", "phase":"work", "workdir":".",
        "allowed_results":["complete"], "context":{},
        "steps":[
            {"role_path":"role.md", "result_path":"first.json"},
            {"role_path":"role.md", "result_path":"second.json"}
        ],
    });
    let (dir, mut between_steps) = engine(single_body(vec!["driver".into()]));
    fail_event(&dir.path().join("forge.db"), "sequence-step-finished");
    assert!(between_steps
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &steps,
            &input,
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .is_err());
}

#[test]
fn decision_and_operator_storage_failures_propagate() {
    let (_kept, mut decision) = engine_failing("transition/decided");
    assert!(decision
        .decide(&state(Some("work"), Cursor::Idle), "effect", json!(2))
        .is_err());

    let dir = tempfile::tempdir().unwrap();
    let first_path = dir.path().join("operator-commanded.db");
    let mut first = parked_store(&first_path, "first");
    fail_event(&first_path, "operator/commanded");
    assert!(operator_command(&mut first, "first", "retry", "operator", "reason").is_err());

    let second_path = dir.path().join("operator-accepted.db");
    let mut second = parked_store(&second_path, "second");
    fail_event(&second_path, "operator/accepted");
    assert!(operator_command(&mut second, "second", "retry", "operator", "reason").is_err());

    let incomplete_path = dir.path().join("incomplete-failure.db");
    let mut incomplete = parked_store(&incomplete_path, "incomplete-failure");
    incomplete
        .append_next(
            "incomplete-failure",
            EventType::OperatorCommanded,
            json!({"command_id":"incomplete","command":"retry","args":{},"operator":"operator"}),
            None,
            None,
        )
        .unwrap();
    fail_event(&incomplete_path, "operator/rejected");
    assert!(apply_fenced_operator_command(
        &mut incomplete,
        "incomplete-failure",
        "incomplete",
        "retry",
        "operator",
        "reason",
        0,
        ZERO_HASH,
    )
    .is_err());

    let commanded_path = dir.path().join("fenced-commanded.db");
    let mut commanded = parked_store(&commanded_path, "fenced-commanded");
    let (seq, hash) = commanded.head_hash("fenced-commanded").unwrap();
    fail_event(&commanded_path, "operator/commanded");
    assert!(apply_fenced_operator_command(
        &mut commanded,
        "fenced-commanded",
        "command",
        "retry",
        "operator",
        "reason",
        seq,
        &hash,
    )
    .is_err());

    let rejected_path = dir.path().join("fenced-rejected.db");
    let mut rejected = parked_store(&rejected_path, "fenced-rejected");
    let (seq, hash) = rejected.head_hash("fenced-rejected").unwrap();
    fail_event(&rejected_path, "operator/rejected");
    assert!(apply_fenced_operator_command(
        &mut rejected,
        "fenced-rejected",
        "command",
        "widen",
        "operator",
        "reason",
        seq,
        &hash,
    )
    .is_err());

    let accepted_path = dir.path().join("fenced-accepted.db");
    let mut accepted = parked_store(&accepted_path, "fenced-accepted");
    let (seq, hash) = accepted.head_hash("fenced-accepted").unwrap();
    fail_event(&accepted_path, "operator/accepted");
    assert!(apply_fenced_operator_command(
        &mut accepted,
        "fenced-accepted",
        "command",
        "retry",
        "operator",
        "reason",
        seq,
        &hash,
    )
    .is_err());

    let replay_path = dir.path().join("replayed-rejection.db");
    let mut replayed = parked_store(&replay_path, "replayed-rejection");
    let (seq, hash) = replayed.head_hash("replayed-rejection").unwrap();
    let rejected = apply_fenced_operator_command(
        &mut replayed,
        "replayed-rejection",
        "same-command",
        "widen",
        "operator",
        "reason",
        seq,
        &hash,
    )
    .unwrap();
    assert!(matches!(rejected, FencedCommandOutcome::Rejected { .. }));
    assert!(matches!(
        apply_fenced_operator_command(
            &mut replayed,
            "replayed-rejection",
            "same-command",
            "widen",
            "operator",
            "reason",
            seq,
            &hash,
        )
        .unwrap(),
        FencedCommandOutcome::Rejected { .. }
    ));
}
