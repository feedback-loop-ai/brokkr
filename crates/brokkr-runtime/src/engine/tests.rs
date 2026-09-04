use super::*;
use crate::bundle::{Limits, Seat};
use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::dispatch::PRODUCER_EFFECTS;
use brokkr_core::policy::Machine;
use brokkr_protocol::{Body, Message, ResultStatus};
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

pub(super) fn single_body(command: Vec<String>) -> SeatBody {
    SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command,
        confine: None,
        candidates: Vec::new(),
    }
}

pub(super) fn bundle(dir: &Path, body: SeatBody) -> Bundle {
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        Seat {
            has_gate: false,
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
                has_gate: false,
                results: vec![result.into()],
                limits: Limits::default(),
                inputs: Vec::new(),
                secrets: Vec::new(),
                body: single_body(vec!["missing-driver".into()]),
            },
        );
    }
    Bundle {
        dialect_prompts: Default::default(),
        name: "test".into(),
        description: String::new(),
        cost: String::new(),
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
        hands: BTreeMap::new(),
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

fn selecting(default: bool) -> SeatBody {
    let cases = [
        (
            "chore".to_string(),
            single_body(vec!["chore-driver".into()]),
        ),
        (
            "feature".to_string(),
            single_body(vec!["feature-driver".into()]),
        ),
        (
            "design".to_string(),
            single_body(vec!["design-driver".into()]),
        ),
        (
            "engine".to_string(),
            single_body(vec!["engine-driver".into()]),
        ),
    ]
    .into_iter()
    .collect();
    SeatBody::Select {
        cases,
        default: default.then(|| Box::new(single_body(vec!["default-driver".into()]))),
        case_gates: BTreeMap::new(),
        default_gate: false,
    }
}

#[test]
fn selected_case_is_journal_derived_and_phase_entry_records_it_or_parks() {
    let (dir, mut runtime) = engine(selecting(false));
    let mut ruled = state(None, Cursor::Start);
    ruled.strategy = Some("engine".into());
    let payload = runtime.phase_entered_payload("work", &ruled);
    assert_eq!(payload, json!({"phase":"work", "case":"engine"}));
    let first = runtime.seat_input(&ruled, "work", "effect").unwrap();
    let resumed = runtime.seat_input(&ruled, "work", "effect").unwrap();
    assert_eq!(first, resumed, "resume rebuilds from the same journal fact");
    assert!(first["role_path"].as_str().unwrap().ends_with("role.md"));

    runtime.drive_once().unwrap();
    let parked = fold(&runtime.store.load(&runtime.run_id).unwrap()).unwrap();
    assert_eq!(parked.status, Status::AwaitingOperator);
    assert!(parked.park_reason.unwrap().contains("SELECT-NO-DEFAULT"));

    let unresolved = state(None, Cursor::Idle);
    assert!(runtime.seat_input(&unresolved, "work", "effect").is_err());

    let (_kept, with_default) = engine(selecting(true));
    let state = state(None, Cursor::Start);
    assert_eq!(
        with_default.phase_entered_payload("work", &state),
        json!({"phase":"work", "case":"default"})
    );
    drop(dir);
}

#[test]
fn forward_context_carries_only_typed_phase_facts() {
    let (_dir, mut runtime) = engine(single_body(vec!["driver".into()]));
    runtime.bundle.machine.phases.insert(0, "specify".into());
    let mut current = state(Some("work"), Cursor::Idle);
    current.phase_results.insert(
        "specify".into(),
        json!({"result":"drafted", "inputs":{"change":"0042-dialect"}}),
    );
    let input = runtime.seat_input(&current, "work", "effect").unwrap();
    assert_eq!(input["context"]["results"]["specify"]["result"], "drafted");
    assert_eq!(
        input["context"]["results"]["specify"]["inputs"]["change"],
        "0042-dialect"
    );
    assert!(input["context"]["results"]["specify"]
        .get("notes")
        .is_none());
}

#[test]
fn implement_receives_dialect_instructions_only_after_a_change_exists() {
    let (_dir, mut runtime) = engine(single_body(vec!["driver".into()]));
    let implement = runtime.bundle.seats.remove("work").unwrap();
    runtime.bundle.seats.insert("implement".into(), implement);
    runtime.bundle.machine.phases[0] = "specify".into();
    runtime.bundle.machine.phases.insert(1, "implement".into());
    runtime
        .bundle
        .dialect_prompts
        .insert("implement".into(), "Archive the dialect change.".into());

    let without_change = runtime
        .seat_input(
            &state(Some("implement"), Cursor::Idle),
            "implement",
            "fast-effect",
        )
        .unwrap();
    assert!(without_change.get("spec_dialect").is_none());

    let mut sdd = state(Some("implement"), Cursor::Idle);
    sdd.phase_results.insert(
        "specify".into(),
        json!({"result":"drafted", "inputs":{"change":"decision-0042"}}),
    );
    let with_change = runtime.seat_input(&sdd, "implement", "sdd-effect").unwrap();
    assert_eq!(with_change["spec_dialect"], "Archive the dialect change.");
}

#[test]
fn an_undeclared_change_claim_is_dropped() {
    let (_dir, mut runtime) = engine(single_body(vec!["driver".into()]));
    runtime
        .decide(
            &state(Some("work"), Cursor::Idle),
            "effect",
            json!({"result":"complete", "inputs":{"change":"must-not-pass"}}),
        )
        .unwrap();
    let events = runtime.store.load(&runtime.run_id).unwrap();
    assert!(events.last().unwrap().payload["inputs"]
        .get("change")
        .is_none());

    let (_dir, mut declared) = engine(single_body(vec!["driver".into()]));
    declared.bundle.seats.get_mut("work").unwrap().inputs = vec!["change".into()];
    declared
        .decide(
            &state(Some("work"), Cursor::Idle),
            "effect",
            json!({"result":"complete", "inputs":{"change":"Not/a/change"}}),
        )
        .unwrap();
    let events = declared.store.load(&declared.run_id).unwrap();
    assert!(events.last().unwrap().payload["problem"]
        .as_str()
        .unwrap()
        .contains("must match"));
}

#[test]
fn dialect_change_expands_from_typed_history_and_absence_parks() {
    assert!(matches!(
        dialect_attempt_outcome(DriverRun::SpawnFailed("gone".into())),
        AttemptOutcome::Failed { error } if error == "gone"
    ));
    assert!(matches!(
        dialect_attempt_outcome(DriverRun::Ran(report(
            AttemptOutcome::Indeterminate { reason: "lost".into() },
            ""
        ))),
        AttemptOutcome::Indeterminate { reason } if reason == "lost"
    ));
    let step = SequenceStep {
        name: "validate".into(),
        results: vec!["drafted".into(), "fail".into()],
        class: SeatClass::Gate,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec![
                    "sh".into(),
                    "-c".into(),
                    "test \"$1\" = expected".into(),
                    "sh".into(),
                    "{change}".into(),
                ],
                state: None,
            },
        },
    };
    let input = |context: Value| {
        json!({
            "feature":"feature", "phase":"design", "workdir":".",
            "allowed_results":["drafted", "fail"], "context":context,
            "steps":[{"role_path":"", "result_path":"result.json", "allowed_results":["drafted", "fail"]}],
        })
    };

    assert_eq!(
        expand_dialect_argv(
            &["openspec".into(), "validate".into(), "{change}".into()],
            "expected",
        ),
        ["openspec", "validate", "expected"]
    );

    let (_kept, mut absent) = engine(single_body(vec!["driver".into()]));
    absent
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[step],
            &input(json!({})),
            std::time::Duration::from_secs(5),
            &Selection::new(),
        )
        .unwrap();
    let events = absent.store.load(&absent.run_id).unwrap();
    assert_eq!(
        events.last().unwrap().event_type,
        EventType::EffectIndeterminate
    );
    assert!(events.last().unwrap().payload["reason"]
        .as_str()
        .unwrap()
        .contains("no preceding successful result carries it"));

    let (_kept, mut verify_without_change) = engine(single_body(vec!["driver".into()]));
    let verify_input = json!({
        "feature":"feature", "phase":"verify", "workdir":".",
        "allowed_results":["pass", "fail"],
        "context":{},
        "steps":[{"role_path":"", "result_path":"result.json", "allowed_results":["pass", "fail"]}],
    });
    let verify_steps = [SequenceStep {
        name: "dialect-verify".into(),
        results: vec!["pass".into(), "fail".into()],
        class: SeatClass::Gate,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec!["validator".into(), "{change}".into()],
                state: None,
            },
        },
    }];
    verify_without_change
        .execute_sequence(
            "effect",
            "attempt",
            "verify",
            &verify_steps,
            &verify_input,
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .unwrap();
    let events = verify_without_change
        .store
        .load(&verify_without_change.run_id)
        .unwrap();
    assert_eq!(
        events.last().unwrap().event_type,
        EventType::EffectIndeterminate
    );
    assert!(events.last().unwrap().payload["reason"]
        .as_str()
        .unwrap()
        .contains("no preceding successful result carries it"));

    let metadata_step = SequenceStep {
        name: "validate".into(),
        results: vec!["drafted".into(), "fail".into()],
        class: SeatClass::Gate,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec!["validator".into()],
                state: Some(vec!["state".into()]),
            },
        },
    };
    let (_kept, metadata) = engine(SeatBody::Sequence {
        steps: vec![metadata_step],
    });
    let rendered = metadata
        .seat_input(&state(Some("work"), Cursor::Idle), "work", "effect")
        .unwrap();
    assert_eq!(rendered["steps"][0]["dialect"]["argv"][0], "validator");

    // Exercise the complete dispatch construction for each dialect result
    // vocabulary. The unit-test executable is deliberately not the brokkr
    // CLI, so each child fails after receiving the fully expanded argv; the
    // exec adapter's successful synthesis is covered in brokkr-protocol.
    for (phase, allowed) in [
        ("clarify", vec!["clear", "ambiguous"]),
        ("analyze", vec!["consistent", "drift"]),
        ("verify", vec!["pass", "fail"]),
        ("design", vec!["drafted", "fail"]),
    ] {
        let (_kept, mut present) = engine(single_body(vec!["driver".into()]));
        let dialect_step = SequenceStep {
            name: "validate".into(),
            results: allowed.iter().map(|value| value.to_string()).collect(),
            class: SeatClass::Work,
            body: StepBody::Dialect {
                execution: crate::bundle::DialectExecution {
                    argv: if phase == "clarify" {
                        vec!["true".into()]
                    } else {
                        vec!["true".into(), "{change}".into()]
                    },
                    state: Some(vec!["printf".into(), "{change}".into()]),
                },
            },
        };
        let dispatch = json!({
            "feature":"feature", "phase":phase, "workdir":present.workdir(),
            "allowed_results":allowed,
            "house_rules":"rules",
            "context":{"results":{"design":{"result":"drafted","inputs":{"change":"expected"}}}},
            "steps":[{"role_path":"", "result_path":"result.json", "allowed_results":allowed}],
        });
        let mut steps = vec![dialect_step];
        if phase == "clarify" {
            steps.push(SequenceStep {
                name: "judge".into(),
                results: vec!["clear".into(), "ambiguous".into()],
                class: SeatClass::Work,
                body: StepBody::Single {
                    role_path: "role".into(),
                    command: vec!["missing".into()],
                    confine: None,
                    candidates: Vec::new(),
                },
            });
        }
        present
            .execute_sequence(
                "effect",
                "attempt",
                "work",
                &steps,
                &dispatch,
                std::time::Duration::from_secs(2),
                &Selection::new(),
            )
            .unwrap();
        assert!(present.store.load(&present.run_id).unwrap().len() > 1);
    }

    let (_kept, mut no_token) = engine(single_body(vec!["driver".into()]));
    let no_token_step = SequenceStep {
        name: "validate".into(),
        results: vec!["drafted".into(), "fail".into()],
        class: SeatClass::Work,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec!["true".into()],
                state: None,
            },
        },
    };
    no_token
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[no_token_step],
            &input(json!({})),
            std::time::Duration::from_secs(2),
            &Selection::new(),
        )
        .unwrap();

    let (_kept, mut storage_error) = engine_failing("effect/checkpointed");
    let mut selection = Selection::new();
    selection.insert(
        Some("validate".into()),
        Candidate {
            agent: "dialect".into(),
            model: "none".into(),
            effort: None,
            provider: "exec".into(),
            argv: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"drafted"}),
                },
            ),
        },
    );
    let failing_step = SequenceStep {
        name: "validate".into(),
        results: vec!["drafted".into(), "fail".into()],
        class: SeatClass::Work,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec!["unused".into()],
                state: None,
            },
        },
    };
    assert!(storage_error
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &[failing_step],
            &input(json!({})),
            std::time::Duration::from_secs(2),
            &selection,
        )
        .is_err());
}

#[test]
fn a_sequence_fences_a_malformed_change_before_the_dialect_tool_runs() {
    let first = SequenceStep {
        name: "author".into(),
        results: vec!["drafted".into()],
        class: SeatClass::Work,
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({
                        "result":"drafted",
                        "inputs":{"change":"--config=../../.."}
                    }),
                },
            ),
            confine: None,
            candidates: Vec::new(),
        },
    };
    let validate = SequenceStep {
        name: "validate".into(),
        results: vec!["drafted".into(), "fail".into()],
        class: SeatClass::Gate,
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec!["validator".into(), "{change}".into()],
                state: None,
            },
        },
    };
    let input = json!({
        "feature":"feature", "phase":"design", "workdir":".",
        "allowed_results":["drafted", "fail"], "context":{},
        "steps":[
            {"role_path":"role.md", "result_path":"author.json", "allowed_results":["drafted"]},
            {"role_path":"", "result_path":"validate.json", "allowed_results":["drafted", "fail"]}
        ]
    });
    let (_kept, mut runtime) = engine(single_body(vec!["driver".into()]));
    runtime.bundle.seats.get_mut("work").unwrap().inputs = vec!["change".into()];
    let mut selection = Selection::new();
    selection.insert(
        Some("validate".into()),
        Candidate {
            agent: "dialect".into(),
            model: "none".into(),
            effort: None,
            provider: "exec".into(),
            argv: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"drafted"}),
                },
            ),
        },
    );
    let steps = [first, validate];
    runtime
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &steps,
            &input,
            std::time::Duration::from_secs(2),
            &selection,
        )
        .unwrap();

    let events = runtime.store.load(&runtime.run_id).unwrap();
    assert_eq!(
        events.last().unwrap().event_type,
        EventType::EffectIndeterminate
    );
    assert!(events.last().unwrap().payload["reason"]
        .as_str()
        .unwrap()
        .contains("must match"));
    assert_eq!(
        events
            .iter()
            .filter(|event| event.payload["checkpoint"]["step"] == "live")
            .count(),
        1,
        "only the author ran; validate never received an argv"
    );

    let (_kept, mut undeclared) = engine(single_body(vec!["driver".into()]));
    undeclared
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &steps,
            &input,
            std::time::Duration::from_secs(2),
            &selection,
        )
        .unwrap();
    let events = undeclared.store.load(&undeclared.run_id).unwrap();
    assert_eq!(
        events.last().unwrap().event_type,
        EventType::EffectIndeterminate,
        "an input the seat did not declare is ignored, leaving no change to expand"
    );
    assert!(events.last().unwrap().payload["reason"]
        .as_str()
        .unwrap()
        .contains("no preceding successful result carries it"));

    let (_kept, mut storage_error) = engine_failing("effect/indeterminate");
    storage_error.bundle.seats.get_mut("work").unwrap().inputs = vec!["change".into()];
    assert!(storage_error
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &steps,
            &input,
            std::time::Duration::from_secs(2),
            &selection,
        )
        .is_err());
}

#[test]
fn process_recovery_resumes_the_case_pinned_by_the_triage_journal() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("work");
    std::fs::create_dir(&repo).unwrap();
    let script = r#"
read -r hello
printf '%s\n' '{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}\n' "$effect_id" "$attempt_id"
printf '{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"feature"},"error":null}\n' "$effect_id" "$attempt_id"
read -r done
"#;
    let mut compiled = bundle(dir.path(), selecting(false));
    compiled.machine = Machine::from_table(&json!({
        "phases":["triage", "work", "review", "ship", "done", "stop"],
        "initial":"triage", "terminal":["done", "stop"],
        "rules":[
            {"id":"TRIAGE", "from":"triage", "result":"feature", "next":"work", "reason":"route"},
            {"id":"WORK", "from":"work", "result":"complete", "next":"review", "reason":"work"},
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"ship", "reason":"review"},
            {"id":"SHIP", "from":"ship", "result":"shipped", "next":"done", "reason":"ship"}
        ]
    }))
    .unwrap();
    compiled.seats.insert(
        "triage".into(),
        Seat {
            has_gate: false,
            results: vec!["feature".into()],
            limits: Limits::default(),
            inputs: Vec::new(),
            secrets: Vec::new(),
            body: single_body(vec!["sh".into(), "-c".into(), script.into()]),
        },
    );
    let database = dir.path().join("forge.db");
    let store = Store::open(&database).unwrap();
    let mut runtime = Engine::start(
        store,
        compiled.clone(),
        "resume selection",
        Some(repo.clone()),
    )
    .unwrap();

    // Stop the process immediately after the selected phase was entered:
    // no in-memory state survives into the Engine::resume below.
    for _ in 0..5 {
        if let Some(end) = runtime.drive_once().unwrap() {
            panic!("run ended before selected phase entry: {:?}", end.state)
        }
    }
    let run_id = runtime.run_id.clone();
    let before = fold(&runtime.store.load(&run_id).unwrap()).unwrap();
    assert_eq!(before.phase.as_deref(), Some("work"));
    assert_eq!(before.strategy.as_deref(), Some("feature"));
    assert_eq!(before.cursor, Cursor::RequestEffect);
    drop(runtime);

    let resumed = Engine::resume(
        Store::open(&database).unwrap(),
        compiled,
        &run_id,
        Some(repo),
    )
    .unwrap();
    let rebuilt = fold(&resumed.store.load(&run_id).unwrap()).unwrap();
    assert_eq!(
        resumed.phase_entered_payload("work", &rebuilt),
        json!({"phase":"work", "case":"feature"})
    );
    let (body, case) = resumed.bundle.seats["work"]
        .body
        .selected(rebuilt.strategy.as_deref())
        .unwrap();
    assert_eq!(case, Some("feature"));
    let ExecutableBody::Single { command, .. } = body else {
        panic!("the feature case must remain a single seat")
    };
    assert_eq!(command, &["feature-driver"]);
}

pub(super) fn state(phase: Option<&str>, cursor: Cursor) -> RunState {
    RunState {
        run_id: "run".into(),
        seq: 1,
        last_hash: ZERO_HASH.into(),
        status: Status::Running,
        phase: phase.map(str::to_string),
        cursor,
        consecutive_failures: BTreeMap::new(),
        visits: BTreeMap::new(),
        strategy: None,
        last_result: None,
        phase_results: BTreeMap::new(),
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
        "read -r line; printf '%s\\n' '{capabilities}'; read -r line; printf '%s\\n' '{accepted}'; printf '%s\\n' '{checkpoint}'"
    );
    if let Some(terminal) = terminal {
        script.push_str(&format!("; printf '%s\\n' '{terminal}'; read -r line"));
    }
    vec!["sh".into(), "-c".into(), script]
}

#[test]
fn a_gate_driver_that_commits_parks_with_raw_head_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let script = r#"
read -r hello
printf '%s\n' '{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","body":{"driver":"test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
git commit -q --allow-empty -m gate-moved-head
printf '{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","body":{"effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{"proto":"forge-driver/v1","msg_id":"result","type":"result","body":{"effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete"},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#;
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut compiled = bundle(
        dir.path(),
        single_body(vec!["sh".into(), "-c".into(), script.into()]),
    );
    compiled.seats.get_mut("work").unwrap().has_gate = true;
    let mut engine = Engine::start(store, compiled, "gate moves", Some(repo)).unwrap();
    let end = engine.drive().unwrap();
    assert_eq!(end.state.status, Status::AwaitingOperator);
    assert_eq!(end.state.park_reason.as_deref(), Some("GATE-MOVED-HEAD"));
    let events = engine.store.load(&engine.run_id).unwrap();
    let parked = events
        .iter()
        .find(|event| event.event_type == EventType::RunParked)
        .unwrap();
    let evidence = &parked.payload["evidence"];
    assert_ne!(evidence["head_at_start"], evidence["head_at_end"]);
    assert!(evidence["head_at_start"].is_string());
    assert!(evidence["head_at_end"].is_string());
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded));
}

#[test]
fn a_gate_driver_that_leaves_head_unchanged_keeps_its_own_failure() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let script = r#"
read -r hello
printf '%s\\n' '{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","body":{"driver":"test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","body":{"effect_id":"%s","attempt_id":"%s","session_ref":null}}\\n' "$effect_id" "$attempt_id"
printf '{"proto":"forge-driver/v1","msg_id":"result","type":"result","body":{"effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete","fixes_applied":false},"error":null}}\\n' "$effect_id" "$attempt_id"
read -r done
"#;
    let script = script.replace(r"\\", r"\").replace(r"\\", r"\");
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut compiled = bundle(
        dir.path(),
        single_body(vec!["sh".into(), "-c".into(), script]),
    );
    compiled.seats.get_mut("work").unwrap().has_gate = true;
    let mut engine = Engine::start(store, compiled, "gate stays", Some(repo)).unwrap();
    let end = engine.drive().unwrap();
    assert_eq!(end.state.status, Status::AwaitingOperator);
    assert_eq!(end.state.phase.as_deref(), Some("work"));
    assert_ne!(end.state.park_reason.as_deref(), Some("GATE-MOVED-HEAD"));
    assert!(end
        .state
        .park_reason
        .as_deref()
        .unwrap()
        .contains("failed 2 of 2"));
}

fn compiled_sequence_with_gate(dir: &Path, gate_commits: bool) -> Bundle {
    let bundle_dir = dir.join("bundle");
    let agents = dir.join("agents");
    let adapters = dir.join("adapters");
    std::fs::create_dir_all(bundle_dir.join("roles")).unwrap();
    std::fs::create_dir_all(agents.join("charters")).unwrap();
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(bundle_dir.join("roles/chief.md"), "# chief\n").unwrap();
    std::fs::write(bundle_dir.join("roles/review.md"), "# review\n").unwrap();
    std::fs::write(agents.join("charters/validator.md"), "# validator\n").unwrap();

    let driver = |commit: bool, result: &str| {
        format!(
            r#"
read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
{}
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{{"result":"{}"}},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#,
            if commit {
                "git commit -q --allow-empty -m site-moved-head"
            } else {
                ":"
            },
            result
        )
    };
    let chief_driver = driver(true, "drafted");
    let validator_driver = driver(gate_commits, "complete");
    std::fs::write(
        adapters.join("judge.json"),
        serde_json::to_vec_pretty(&json!({
            "provider": "judge",
            "trust_tier": "trusted",
            "binding_grant": true,
            "binary": "sh",
            "driver": ["sh", "-c", validator_driver],
            "models": {"judge": "judge-1"},
            "judges": ["judge"],
            "model_flag": "--model",
            "efforts": ["high"],
            "effort_flag": "--effort",
            "tool_permissions": "unsupported",
            "mcp": "unsupported"
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        agents.join("validator.json"),
        serde_json::to_vec_pretty(&json!({
            "description": "fixture validator",
            "charter": "charters/validator.md",
            "models": ["judge"],
            "efforts": {"judge": "high"}
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        bundle_dir.join("policy.json"),
        serde_json::to_vec_pretty(&json!({
            "schema": "forge.phase-machine/v1",
            "phases": ["work", "review", "done", "stop"],
            "initial": "work",
            "terminal": ["done", "stop"],
            "shippable_from": ["review"],
            "rules": [
                {"id": "WORK", "from": "work", "result": "complete", "next": "review", "reason": "worked"},
                {"id": "REVIEW", "from": "review", "result": "clean", "next": "done", "reason": "reviewed"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_vec_pretty(&json!({
            "name": "gate-sequence",
            "description": "compiled gate sequence",
            "cost": "test",
            "policy": "policy.json",
            "protected_phase": "review",
            "seats": {
                "work": {
                    "results": ["complete"],
                    "sequence": [
                        {"name": "chief", "results": ["drafted"], "class": "work", "role": "roles/chief.md",
                         "driver": {"command": ["sh", "-c", chief_driver]}},
                        {"name": "validator", "class": "gate", "agent": "validator"}
                    ]
                },
                "review": {"results": ["clean"], "class": "work", "role": "roles/review.md",
                           "driver": {"command": ["missing-review-driver"]}}
            }
        }))
        .unwrap(),
    )
    .unwrap();
    let compiled = Bundle::compile_with(&bundle_dir, &agents, &adapters).unwrap();
    assert!(!compiled.seats["work"].has_gate);
    let SeatBody::Sequence { steps } = &compiled.seats["work"].body else {
        panic!("the fixture must compile as a sequence")
    };
    assert_eq!(steps[0].class, SeatClass::Work);
    assert_eq!(steps[1].class, SeatClass::Gate);
    compiled
}

#[test]
fn a_compiled_mixed_sequence_guards_the_gate_step_not_the_work_step() {
    for gate_commits in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir(&repo).unwrap();
        let base = git_commit(&repo, "base");
        let compiled = compiled_sequence_with_gate(dir.path(), gate_commits);
        let store = Store::open(&dir.path().join("forge.db")).unwrap();
        let mut engine =
            Engine::start(store, compiled, "mixed sequence", Some(repo.clone())).unwrap();
        let end = engine.drive().unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();

        assert_ne!(
            git_head(&repo).as_deref(),
            Some(base.as_str()),
            "the work chief commits"
        );
        if gate_commits {
            assert_eq!(end.state.park_reason.as_deref(), Some("GATE-MOVED-HEAD"));
            assert!(!events
                .iter()
                .any(|event| event.event_type == EventType::EffectSucceeded));
        } else {
            assert_ne!(end.state.park_reason.as_deref(), Some("GATE-MOVED-HEAD"));
            assert!(
                events
                    .iter()
                    .any(|event| event.event_type == EventType::EffectSucceeded),
                "{events:#?}"
            );
        }
    }
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
            "actor_id":"brokkr","accountable_operator_id":"operator","authority_source":"looper-grant",
            "operating_profile":"bounded"},
        "repository":{"owner":"owner","name":"repo","base_sha":"b".repeat(64),
            "candidate_sha":null,"workspace_class":"isolated","target_environment":"dogfood"},
        "recipe":{"name":"test","compiled_sha256":bundle.manifest_digest()},
        "budget":{"lane_tally_run_id":"lane","reservation_id":null,"cost_state":"known",
            "ceiling_microunits":1000,"currency":"USD"},
        "producer":{"registration_id":"registration","token_reference":"key",
            "callback_audience":"https://dogfood.example","accepting_service_id":"looper-api",
            "runtime_id":"runtime","producer_release":"brokkr@test","protocol_version":1,
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
                    results: vec!["single-result".into()],
                    class: SeatClass::Work,
                    body: StepBody::Single {
                        role_path: "role".into(),
                        command: vec!["driver".into()],
                        confine: None,
                        candidates: Vec::new(),
                    },
                },
                SequenceStep {
                    name: "panel".into(),
                    results: vec!["panel-result".into()],
                    class: SeatClass::Work,
                    body: StepBody::Panel {
                        members,
                        aggregate: Aggregate::UnanimousPass,
                    },
                },
                SequenceStep {
                    name: "dialect".into(),
                    results: vec!["drafted".into()],
                    class: SeatClass::Gate,
                    body: StepBody::Dialect {
                        execution: crate::bundle::DialectExecution {
                            argv: vec!["validate".into()],
                            state: None,
                        },
                    },
                },
            ],
        },
    );
    assert_eq!(verify_dispatch_bundle_bounds(&base, &sequence), Ok(()));
    assert!(
        invocation_sites(sequence.seats["work"].body.selected(None).unwrap().0)
            .iter()
            .any(|(site, candidates)| site.as_deref() == Some("dialect") && candidates.is_empty())
    );

    let selected = bundle(dir.path(), selecting(true));
    assert_eq!(verify_dispatch_bundle_bounds(&base, &selected), Ok(()));

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
                None,
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
                json!({"result":"clean", "inputs":{"spec_defect":true}}),
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
    assert_eq!(review["inputs"]["fixes_applied"], false);
    assert_eq!(review["inputs"]["spec_defect"], true);
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
        results: vec!["failed-result".into()],
        class: SeatClass::Work,
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
        results: vec!["lost-result".into()],
        class: SeatClass::Work,
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
        for (key, value) in [("user.name", "Brokkr Test"), ("user.email", "brokkr@test")] {
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

    // Park is a general table operation. Only the triage/escalate pair
    // substitutes the seat's reasoning; another triage result keeps the
    // table's reason.
    engine.bundle.machine = Machine::from_table(&json!({
        "schema":"forge.phase-machine/v2",
        "phases":["triage", "done", "stop"],
        "initial":"triage",
        "terminal":["done", "stop"],
        "rules":[
            {"id":"TRIAGE-CHORE-PARK", "from":"triage", "result":"chore",
             "park":true, "reason":"table reason"}
        ]
    }))
    .unwrap();
    let mut triage_seat = engine.bundle.seats["work"].clone();
    triage_seat.results = vec!["chore".into()];
    engine.bundle.seats.insert("triage".into(), triage_seat);
    engine
        .decide(
            &state(Some("triage"), Cursor::Idle),
            "effect",
            json!({"result":"chore", "notes":"seat reason"}),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(events.last().unwrap().payload["problem"], "table reason");
}

/// A run stopped mid-attempt: the effect is in flight, so the run is
/// running and the seat is still journaling under it. The state a burn
/// spends most of its life in, and the one where an operator's command
/// meets a concurrently-appending engine.
fn in_flight_store(path: &Path, run_id: &str) -> Store {
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
    ] {
        store
            .append_next(run_id, event_type, payload, None, attempt_id)
            .unwrap();
    }
    store
}

fn parked_store(path: &Path, run_id: &str) -> Store {
    let mut store = in_flight_store(path, run_id);
    for (event_type, payload, attempt_id) in [
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

/// A command the run cannot take is refused, and the refusal names the
/// condition rather than crying race. Nothing here is a race: every
/// command below was already illegal when the operator asked for it, and
/// the journal says so in the word it records — `lost_fence` is reserved
/// for a run that MOVED, and an operator reading a refused `retry` must
/// be able to tell "you were unlucky" from "that run was never parked".
///
/// Every one must come back refused and — the whole point — must leave a
/// journal that still folds.
#[test]
fn an_operator_command_the_run_cannot_take_is_refused_never_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(&dir.path().join("raced.db"), "raced");

    // Single-writer behaviour first, unchanged: the run is parked, the
    // stop is legal, it is accepted.
    assert!(
        matches!(
            operator_command(&mut store, "raced", "stop", "operator", "enough").unwrap(),
            FencedCommandOutcome::Accepted { .. }
        ),
        "a stop on a parked run is still accepted",
    );

    // Un-parked: the accepted stop above already moved the run back to
    // running, and `retry` is legal only on a parked run. The run did not
    // move under this command — it was in the wrong state before the
    // command was even journaled — so the refusal says which state.
    let refused = operator_command(&mut store, "raced", "retry", "operator", "once more").unwrap();
    assert!(
        matches!(&refused, FencedCommandOutcome::Rejected { reason, .. }
            if reason == RUN_NOT_AWAITING_OPERATOR),
        "{refused:?}",
    );

    // Already concluded. This is the irreversible case: fold exempts only
    // `operator/commanded` and `operator/rejected` after a terminal, so an
    // acceptance here is `AfterTerminal` for every reader from now on.
    // Both commands must refuse — including `stop`, which anywhere
    // non-terminal is a live kill switch.
    store
        .append_next(
            "raced",
            EventType::RunStopped,
            json!({"reason": "OPERATOR-STOP: enough"}),
            None,
            None,
        )
        .unwrap();
    for command in ["retry", "stop"] {
        let refused = operator_command(&mut store, "raced", command, "operator", "too late")
            .unwrap_or_else(|error| panic!("{command} errored instead of refusing: {error}"));
        assert!(
            matches!(&refused, FencedCommandOutcome::Rejected { reason, .. }
                if reason == AFTER_TERMINAL),
            "{command}: {refused:?}",
        );
    }

    let events = store.load("raced").unwrap();
    // The journal still folds — three refusals later.
    assert_eq!(fold(&events).unwrap().status, Status::Stopped);
    let counted = |event_type| {
        events
            .iter()
            .filter(|event| event.event_type == event_type)
            .count()
    };
    assert_eq!(
        counted(EventType::OperatorAccepted),
        1,
        "only the one command that was still legal was ever accepted",
    );
    assert_eq!(counted(EventType::OperatorRejected), 3);
    // Every refusal names a condition, never a race, and disposes of
    // exactly the command it refused — so the journal reads back as three
    // answered commands rather than three loose ones.
    for rejected in events
        .iter()
        .filter(|event| event.event_type == EventType::OperatorRejected)
    {
        assert!(
            rejected.payload["reason"] == json!(RUN_NOT_AWAITING_OPERATOR)
                || rejected.payload["reason"] == json!(AFTER_TERMINAL),
            "a refusal that was not a race must not claim one: {}",
            rejected.payload["reason"],
        );
        let command_id = &rejected.payload["command_id"];
        assert!(events.iter().any(|event| {
            event.event_type == EventType::OperatorCommanded
                && &event.payload["command_id"] == command_id
        }));
    }

    // The hazard is not hypothetical. Written by hand — exactly the pair
    // the unfenced path used to write into this position — the journal
    // stops folding, permanently, because events are immutable.
    let mut poisoned = parked_store(&dir.path().join("poisoned.db"), "poisoned");
    operator_command(&mut poisoned, "poisoned", "stop", "operator", "enough").unwrap();
    poisoned
        .append_next(
            "poisoned",
            EventType::RunStopped,
            json!({"reason": "stopped"}),
            None,
            None,
        )
        .unwrap();
    for (event_type, payload) in [
        (
            EventType::OperatorCommanded,
            json!({"command_id":"late","command":"stop","args":{},"operator":"operator"}),
        ),
        (
            EventType::OperatorAccepted,
            json!({"command_id":"late","operator":"operator","reason":"too late"}),
        ),
    ] {
        poisoned
            .append_next("poisoned", event_type, payload, None, None)
            .unwrap();
    }
    assert!(
        fold(&poisoned.load("poisoned").unwrap()).is_err(),
        "the acceptance the fence now refuses is exactly what breaks the fold",
    );
    // And a fenced command arriving at that already-broken journal
    // refuses to add to it rather than folding it again.
    assert!(operator_command(&mut poisoned, "poisoned", "stop", "operator", "later").is_err());
}

/// A command the vocabulary does not know is refused by NAME before any
/// question of fences or cursors: `refusal_for` closes the verb set.
#[test]
fn a_command_outside_the_vocabulary_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(&dir.path().join("verbs.db"), "verbs");
    let refused = operator_command(&mut store, "verbs", "dance", "operator", "please").unwrap();
    assert!(
        matches!(&refused, FencedCommandOutcome::Rejected { reason, .. } if reason == COMMAND_NOT_ALLOWED),
        "{refused:?}"
    );
}

/// A contender that NEVER yields: the head moves in every window the
/// fence opens, and after `FENCE_ATTEMPTS` lost rounds the command is
/// refused rather than spun forever — the loop's exhaustion arm is a
/// bound, and a bound must be reachable to be real.
#[test]
fn a_fence_lost_every_round_is_refused_not_spun_forever() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = in_flight_store(&dir.path().join("relentless.db"), "relentless");
    let mut checkpoints = 0;
    let refused = operator_command_racing(
        &mut store,
        "relentless",
        "stop",
        "operator",
        "enough",
        |store: &mut Store| {
            // A checkpoint is fold-neutral for a run mid-effect: the
            // status stays running, the pending command stays pending,
            // and only the head moves — which is the whole point.
            checkpoints += 1;
            store
                .append_next(
                    "relentless",
                    EventType::EffectCheckpointed,
                    json!({"effect_id":"effect","attempt_id":"attempt","note":checkpoints}),
                    None,
                    Some("attempt".into()),
                )
                .unwrap();
        },
    )
    .unwrap();
    assert!(
        matches!(&refused, FencedCommandOutcome::Rejected { reason, .. } if reason == LOST_FENCE),
        "{refused:?}"
    );
    assert!(
        checkpoints > FENCE_ATTEMPTS,
        "the bound was reached, not merely approached: {checkpoints}"
    );
    fold(&store.load("relentless").unwrap()).unwrap();
}

/// The fenced path's own append-time race: the cursor check passed, the
/// command landed, and a peer appended before the acceptance could — the
/// acceptance must NOT land on a head the cursor never covered, and the
/// caller is told their cursor went stale.
#[test]
fn a_fenced_acceptance_beaten_to_the_head_reports_a_stale_cursor() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(&dir.path().join("beaten.db"), "beaten");
    let (seq, hash) = store.head_hash("beaten").unwrap();
    let mut peers = 1;
    let refused = apply_fenced_racing(
        &mut store,
        "beaten",
        "looper-command",
        "retry",
        "operator",
        "once more",
        seq,
        &hash,
        |store: &mut Store| {
            if peers > 0 {
                peers -= 1;
                store
                    .append_next(
                        "beaten",
                        EventType::OperatorCommanded,
                        json!({"command_id":"peer","command":"stop","args":{},"operator":"other"}),
                        None,
                        None,
                    )
                    .unwrap();
            }
        },
    )
    .unwrap();
    assert!(
        matches!(&refused, FencedCommandOutcome::Rejected { reason, .. } if reason == "stale_cursor"),
        "{refused:?}"
    );
    let events = store.load("beaten").unwrap();
    fold(&events).unwrap();
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::OperatorAccepted));
}

/// Where the first event of a type sits in a journal — the order of the
/// three events a raced command leaves behind is what proves the peer
/// landed inside the window rather than before or after it.
fn seq_of(events: &[EventEnvelope], event_type: EventType) -> u64 {
    events
        .iter()
        .find(|event| event.event_type == event_type)
        .unwrap_or_else(|| panic!("no {event_type:?} in the journal"))
        .seq
}

/// The between-effects write race itself, driven rather than hoped for.
///
/// An operator at a terminal supplies no cursor, so nothing but the
/// engine's own re-read stands between their decision and the write — and
/// a run being driven concurrently can conclude in exactly that gap. The
/// probe below IS that gap: it appends what the engine would have
/// appended, at the one instant the acceptance is decided and not yet
/// written. A race two threads have to be lucky to reproduce is a race
/// that cannot be asserted on; this one happens every time.
///
/// The acceptance must not land. Not narrowly, not usually: the head it
/// was decided against is gone, so the store refuses it, the run is
/// folded again, and the command is answered with the refusal that names
/// the race.
#[test]
fn an_operator_command_that_lost_its_fence_is_refused_never_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = in_flight_store(&dir.path().join("raced.db"), "raced");

    let mut engine_appends = 1;
    let refused = operator_command_racing(
        &mut store,
        "raced",
        "stop",
        "operator",
        "enough",
        |store: &mut Store| {
            // The engine, concluding the run in the window. `stop` was
            // legal against the fold that just ran and is illegal against
            // the journal by the time the acceptance would be written.
            if engine_appends > 0 {
                engine_appends -= 1;
                store
                    .append_next(
                        "raced",
                        EventType::EffectSucceeded,
                        json!({"effect_id":"effect","result":{"result":"complete"}}),
                        None,
                        None,
                    )
                    .unwrap();
                store
                    .append_next("raced", EventType::RunCompleted, json!({}), None, None)
                    .unwrap();
            }
        },
    )
    .unwrap();
    assert!(
        matches!(&refused, FencedCommandOutcome::Rejected { reason, .. } if reason == LOST_FENCE),
        "a command that WAS legal when it was decided reports the race: {refused:?}",
    );

    let events = store.load("raced").unwrap();
    assert!(
        seq_of(&events, EventType::OperatorCommanded) < seq_of(&events, EventType::RunCompleted)
            && seq_of(&events, EventType::RunCompleted)
                < seq_of(&events, EventType::OperatorRejected),
        "the engine's conclusion landed INSIDE the window, which is the race being proved",
    );
    assert_eq!(
        fold(&events).unwrap().status,
        Status::Completed,
        "the run reached the conclusion the engine wrote, undisturbed",
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::OperatorAccepted),
        "the acceptance the run could no longer take was never written",
    );
}

/// The other side of the same fence: a peer that appends in the window
/// without taking the command's legality away must not turn an acceptance
/// into a refusal. The head moved, so the store refuses the write — and
/// the command is then decided again against what the peer wrote, which
/// still permits it, so it lands. A fence that refused here would be
/// spite, not safety.
#[test]
fn a_command_still_legal_after_the_head_moved_is_decided_again_and_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = in_flight_store(&dir.path().join("checkpointed.db"), "checkpointed");

    let mut checkpoints = 1;
    let accepted = operator_command_racing(
        &mut store,
        "checkpointed",
        "stop",
        "operator",
        "enough",
        |store: &mut Store| {
            // A seat's checkpoint: the head moves, the run does not.
            if checkpoints > 0 {
                checkpoints -= 1;
                store
                    .append_next(
                        "checkpointed",
                        EventType::EffectCheckpointed,
                        json!({"effect_id":"effect","attempt_id":"attempt","note":"still working"}),
                        None,
                        Some("attempt".into()),
                    )
                    .unwrap();
            }
        },
    )
    .unwrap();
    assert!(
        matches!(accepted, FencedCommandOutcome::Accepted { .. }),
        "{accepted:?}",
    );
    let events = store.load("checkpointed").unwrap();
    assert!(
        seq_of(&events, EventType::OperatorCommanded)
            < seq_of(&events, EventType::EffectCheckpointed)
            && seq_of(&events, EventType::EffectCheckpointed)
                < seq_of(&events, EventType::OperatorAccepted),
        "the peer's append landed INSIDE the window: the acceptance was refused once, \
         re-decided, and written after it",
    );
    // A stop accepted mid-flight rides: the attempt is untouched and the
    // effect's own boundary concludes the run. Unchanged by the retry.
    assert!(fold(&events).unwrap().riding_stop);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == EventType::OperatorAccepted)
            .count(),
        1,
        "re-deciding wrote one acceptance, not one per attempt",
    );
}

/// The peer at the other terminal. Two operators commanding at once are
/// the same race as an operator against an engine, and it lands somewhere
/// nastier: `fold` reads an acceptance as disposing of the command still
/// PENDING, so an acceptance written after a peer's command has taken
/// that place is `NoMatchingCommand` for every reader afterwards.
#[test]
fn a_second_operators_command_in_the_window_takes_the_acceptance_with_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(&dir.path().join("two-terminals.db"), "two-terminals");

    let mut peers = 1;
    let refused = operator_command_racing(
        &mut store,
        "two-terminals",
        "retry",
        "operator",
        "once more",
        |store: &mut Store| {
            if peers > 0 {
                peers -= 1;
                store
                    .append_next(
                        "two-terminals",
                        EventType::OperatorCommanded,
                        json!({"command_id":"peer","command":"stop","args":{},"operator":"other"}),
                        None,
                        None,
                    )
                    .unwrap();
            }
        },
    )
    .unwrap();
    assert!(
        matches!(&refused, FencedCommandOutcome::Rejected { reason, .. } if reason == LOST_FENCE),
        "{refused:?}",
    );
    let events = store.load("two-terminals").unwrap();
    assert!(
        fold(&events).is_ok(),
        "the journal reads back — which it would not, had the acceptance landed",
    );
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::OperatorAccepted));
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

/// A journal built event by event, under the pinned bundle manifest so
/// `Engine::resume` accepts it. Nothing here fabricates an operator's
/// database: every store is a fresh temp file.
fn journal(path: &Path, run_id: &str, manifest: &Value, events: &[(EventType, Value)]) -> Store {
    let mut store = Store::open(path).unwrap();
    store
        .create_run(run_id, "feature", "test", manifest)
        .unwrap();
    for (event_type, payload) in events {
        store
            .append_next(run_id, *event_type, payload.clone(), None, None)
            .unwrap();
    }
    store
}

/// An accepted operator stop is a sentence the engine must finish: it
/// reaches the run's next lawful position, journals `run/stopped` there
/// citing the command that caused it, and the run reads `stopped` — never
/// `running` forever with the process exiting as if all were well. The
/// three shapes: mid-flight to a boundary the journal already records,
/// mid-flight to the boundary the ENGINE itself produces, and between
/// effects with nothing to wait for.
#[test]
fn an_accepted_operator_stop_is_carried_to_a_conclusion_that_cites_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let bundle = bundle(dir.path(), single_body(vec!["driver".into()]));
    let manifest = bundle.manifest.clone();
    let started = json!({"feature":"feature","manifest":manifest});
    let in_flight = vec![
        (EventType::RunStarted, started.clone()),
        (EventType::PhaseEntered, json!({"phase":"work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id":"effect","seat":"work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"effect","attempt_id":"attempt"}),
        ),
    ];
    let drive = |store: Store, run_id: &str| {
        let mut engine = Engine::resume(store, bundle.clone(), run_id, None).unwrap();
        let end = engine.drive().unwrap();
        let events = engine.store.load(run_id).unwrap();
        // Round trip: the fold reads back every journal the engine
        // writes, and reads it as the conclusion it is.
        let folded = fold(&events).unwrap();
        assert_eq!(folded.status, Status::Stopped);
        assert_eq!(folded.cursor, Cursor::Idle);
        assert_eq!(end.state.status, Status::Stopped);
        events
    };
    let reason = |events: &[EventEnvelope]| {
        let tail = events.last().unwrap();
        assert_eq!(tail.event_type, EventType::RunStopped);
        tail.payload["reason"].as_str().unwrap().to_string()
    };

    // The fixture's shape: the stop accepted while the attempt was in
    // flight, the attempt journaling on past it to its own boundary
    // (decision 0006 — the stop overrides what happens AFTER the
    // boundary, never the attempt's bounds), then the conclusion.
    let mut store = journal(&dir.path().join("mid.db"), "mid", &manifest, &in_flight);
    // An earlier command that was rejected: the citation must name the
    // command that was ACCEPTED, not the last one anybody typed.
    for (event_type, payload) in [
        (
            EventType::OperatorCommanded,
            json!({"command_id":"earlier","command":"retry","args":{},"operator":"someone"}),
        ),
        (
            EventType::OperatorRejected,
            json!({"command_id":"earlier","operator":"someone","reason":"not now"}),
        ),
    ] {
        store
            .append_next("mid", event_type, payload, None, None)
            .unwrap();
    }
    operator_command(
        &mut store,
        "mid",
        "stop",
        "vyanakiev",
        "the mock reads wrong",
    )
    .unwrap();
    for (event_type, payload) in [
        (
            EventType::EffectCheckpointed,
            json!({"effect_id":"effect","data":{"step":"after the stop"}}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id":"effect","result":{"result":"complete"}}),
        ),
    ] {
        store
            .append_next("mid", event_type, payload, None, None)
            .unwrap();
    }
    let events = drive(store, "mid");
    let cited = reason(&events);
    assert!(cited.starts_with("OPERATOR-STOP: "), "{cited}");
    assert!(cited.contains("commanded stop"), "{cited}");
    assert!(cited.contains("vyanakiev"), "{cited}");
    assert!(cited.contains("the mock reads wrong"), "{cited}");
    assert!(
        !cited.contains("someone"),
        "the rejected command is not the cause: {cited}"
    );
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == EventType::TransitionDecided),
        "a stopped run takes no further transition",
    );

    // The boundary the engine itself reaches: a fresh process closes the
    // in-flight attempt as indeterminate, which alone would PARK. The
    // accepted stop is what the run concludes on instead.
    let mut store = journal(
        &dir.path().join("boundary.db"),
        "boundary",
        &manifest,
        &in_flight,
    );
    operator_command(&mut store, "boundary", "stop", "vyanakiev", "enough").unwrap();
    let events = drive(store, "boundary");
    let types: Vec<EventType> = events.iter().rev().take(2).map(|e| e.event_type).collect();
    assert_eq!(
        types,
        vec![EventType::RunStopped, EventType::EffectIndeterminate],
        "the attempt reached its boundary, and there the run stopped",
    );
    assert!(reason(&events).contains("enough"));

    // Between effects: nothing is in flight, so the conclusion is
    // immediate — no effect is requested after an accepted stop.
    let mut store = journal(
        &dir.path().join("between.db"),
        "between",
        &manifest,
        &in_flight[..2],
    );
    operator_command(
        &mut store,
        "between",
        "stop",
        "vyanakiev",
        "between effects",
    )
    .unwrap();
    let events = drive(store, "between");
    assert_eq!(
        events.len(),
        5,
        "started, phase, commanded, accepted, stopped"
    );
    assert!(reason(&events).contains("between effects"));
}

pub(super) fn fail_event(path: &Path, event_type: &str) {
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

    let (kept, mut selected) = engine(selecting(false));
    fail_event(&kept.path().join("forge.db"), "run/parked");
    assert!(selected
        .enter_phase("work", &state(None, Cursor::Start))
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
    // No operator events to cite (only reachable with a journal the
    // cursor did not come from): the conclusion is still named as one,
    // never a silent or missing event.
    let stopped = cursor_shapes
        .store
        .load(&cursor_shapes.run_id)
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(stopped.event_type, EventType::RunStopped);
    assert_eq!(
        stopped.payload["reason"],
        json!(
            "OPERATOR-STOP: operator 'unrecorded' commanded stop (unrecorded): \
               no reason recorded"
        )
    );
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
fn terminal_drive_anchors_keeps_the_exhibits_and_reports_gaps() {
    let (missing_dir, mut missing) = engine(single_body(vec!["driver".into()]));
    missing.repo = Some(missing_dir.path().join("not-a-repository"));
    // The run cites a head, so the conclusion has an exhibit it cannot
    // keep in a repository that is not one: the anchor gap AND the
    // keep-ref gap are both reported, and neither fails the run.
    for (event_type, payload) in [
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "effect", "seat": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "attempt"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "effect", "result": {"result": "complete"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({
                "from": "work", "result": "complete", "next": "done",
                "inputs": {"reviewed_heads": {"repo": "a".repeat(40)}},
            }),
        ),
        (EventType::RunCompleted, json!({})),
    ] {
        missing.append(event_type, payload, None).unwrap();
    }
    assert_eq!(missing.drive().unwrap().state.status, Status::Completed);

    let (dir, mut anchored) = engine(single_body(vec!["driver".into()]));
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let head = git_commit(&repo, "base");
    anchored.repo = Some(repo.clone());
    // A run that reached a decision citing this repository's head: the
    // exhibit its own conclusion must keep, with no operator verb (0026).
    for (event_type, payload) in [
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "effect", "seat": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "attempt"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "effect", "result": {"result": "complete"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({
                "from": "work", "result": "complete", "next": "done",
                "inputs": {"reviewed_heads": {"repo": head}},
            }),
        ),
    ] {
        anchored.append(event_type, payload, None).unwrap();
    }
    assert_eq!(anchored.drive().unwrap().state.status, Status::Completed);
    let verify = |name: String| {
        Command::new("git")
            .args(["show-ref", "--verify", &name])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success()
    };
    assert!(verify(format!("refs/forge/{}", anchored.run_id)));
    assert!(
        verify(format!("refs/forge/keep/{}/{head}", anchored.run_id)),
        "the conclusion plants the keep-ref itself"
    );
}

fn requested(engine: &Engine, effect_id: &str) -> EventEnvelope {
    let current = state(Some("work"), Cursor::Idle);
    let input = engine.seat_input(&current, "work", effect_id).unwrap();
    event(
        EventType::EffectRequested,
        json!({
            "effect_id":effect_id,
            "input_digest":brokkr_core::canonical::sha256_hex(&input),
        }),
    )
}

#[test]
fn ship_journal_is_a_runtime_read_only_bind_not_a_digested_input() {
    use brokkr_protocol::hands::{BindMode, HandsSpec};

    let (dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine
        .bundle
        .hands
        .insert("ship".into(), HandsSpec::default());
    let input = engine
        .seat_input(&state(Some("ship"), Cursor::Idle), "ship", "effect")
        .unwrap();
    assert!(input["context"].get("journal").is_none());

    let hands = engine.runtime_hands("ship").unwrap();
    assert_eq!(hands.binds.len(), 1);
    assert_eq!(hands.binds[0].mode, BindMode::Ro);
    // The store helper's temp-path rule applies: runtime_hands deliberately
    // reports a canonical mount, so canonicalize the expectation too.
    assert_eq!(
        Path::new(&hands.binds[0].path),
        engine
            .store
            .path()
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
    );

    let workdir = dir.path().join("work");
    engine.store = Store::open(&workdir.join("journal.db")).unwrap();
    assert!(engine.runtime_hands("ship").unwrap().binds.is_empty());
    engine.repo = Some(dir.path().join("not-created"));
    assert_eq!(engine.runtime_hands("ship").unwrap().binds.len(), 1);
    assert!(engine.runtime_hands("missing").is_none());
}

#[test]
fn selected_ship_site_receives_the_runtime_journal_bind() {
    use brokkr_protocol::hands::{BindMode, HandsSpec};

    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine
        .bundle
        .hands
        .insert("ship:feature".into(), HandsSpec::default());

    let hands = engine.runtime_hands("ship:feature").unwrap();
    assert_eq!(hands.binds.len(), 1);
    assert_eq!(hands.binds[0].mode, BindMode::Ro);
    assert_eq!(
        Path::new(&hands.binds[0].path),
        engine
            .store
            .path()
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
    );
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
            "read -r line; printf '%s\\n' '{capabilities}'; read -r line; printf '%s\\n' \
             '{accepted}'; printf '%s\\n' '{first}'; printf '%s\\n' '{second}'; \
             printf '%s\\n' '{terminal}'; read -r line"
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
        results: vec!["failed-result".into()],
        class: SeatClass::Work,
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
        results: vec!["lost-result".into()],
        class: SeatClass::Work,
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
        results: vec!["ok-result".into()],
        class: SeatClass::Work,
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
        results: vec!["checkpoint-result".into()],
        class: SeatClass::Work,
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
            results: vec!["first-result".into()],
            class: SeatClass::Work,
            body: StepBody::Single {
                role_path: "role.md".into(),
                command: driver_command(
                    "effect",
                    "attempt",
                    AttemptOutcome::Succeeded {
                        result: json!({"result":"first-result"}),
                    },
                ),
                confine: None,
                candidates: Vec::new(),
            },
        },
        SequenceStep {
            name: "second".into(),
            results: vec!["second-result".into()],
            class: SeatClass::Work,
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

    let invalid_steps = [
        SequenceStep {
            name: "invalid".into(),
            results: vec!["declared".into()],
            class: SeatClass::Work,
            body: StepBody::Single {
                role_path: "role.md".into(),
                command: driver_command(
                    "effect",
                    "attempt",
                    AttemptOutcome::Succeeded {
                        result: json!({"result":"invented"}),
                    },
                ),
                confine: None,
                candidates: Vec::new(),
            },
        },
        SequenceStep {
            name: "never".into(),
            results: vec!["complete".into()],
            class: SeatClass::Work,
            body: StepBody::Single {
                role_path: "role.md".into(),
                command: vec!["must-not-run".into()],
                confine: None,
                candidates: Vec::new(),
            },
        },
    ];
    let invalid_input = json!({
        "feature":"feature", "phase":"work", "workdir":".",
        "allowed_results":["complete"], "context":{},
        "steps":[
            {"role_path":"role.md", "result_path":"first.json"},
            {"role_path":"role.md", "result_path":"second.json"}
        ],
    });
    let (_kept, mut invalid_sequence) = engine_failing("effect/failed");
    assert!(invalid_sequence
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &invalid_steps,
            &invalid_input,
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

/// A map, written where a world's map lives, naming one realm at `repo`.
fn world_over(dir: &Path, repo: &Path, name: &str) -> crate::realms::World {
    let path = dir.join("realms.json");
    std::fs::write(
        &path,
        json!({
            "schema": brokkr_core::realms::SCHEMA_V1,
            "realms": [{
                "name": name,
                "path": repo.to_string_lossy(),
                "default_branch": "main",
            }],
            "journal": "forge.db",
        })
        .to_string(),
    )
    .unwrap();
    crate::realms::World::load(&path).unwrap()
}

fn world_with_house(dir: &Path, repo: &Path, house: &str) -> crate::realms::World {
    std::fs::write(repo.join("HOUSE.md"), house).unwrap();
    let path = dir.join("realms.json");
    std::fs::write(
        &path,
        json!({
            "schema": brokkr_core::realms::SCHEMA_V3,
            "realms": [{"name": "brokkr", "path": repo.to_string_lossy(),
                        "default_branch": "main", "house": "HOUSE.md"}],
            "journal": "forge.db",
        })
        .to_string(),
    )
    .unwrap();
    crate::realms::World::load(&path).unwrap()
}

fn engine_in(dir: &Path, world: Option<crate::realms::World>, repo: &Path) -> Engine {
    let store = Store::open(&dir.join("forge.db")).unwrap();
    Engine::start_in_world(
        store,
        bundle(dir, single_body(vec!["driver".into()])),
        "feature",
        Some(repo.to_path_buf()),
        world,
    )
    .unwrap()
}

#[test]
fn every_seat_input_carries_the_realms_house_and_an_unhoused_realm_carries_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let housed = world_with_house(dir.path(), &repo, "One realm rule.\n");
    let engine = engine_in(dir.path(), Some(housed), &repo);
    let input = engine
        .seat_input(&state(Some("work"), Cursor::Idle), "work", "effect")
        .unwrap();
    assert_eq!(input["house_rules"], "One realm rule.\n");

    let plain_dir = tempfile::tempdir().unwrap();
    let plain_repo = plain_dir.path().join("repo");
    std::fs::create_dir(&plain_repo).unwrap();
    let plain_world = world_over(plain_dir.path(), &plain_repo, "brokkr");
    let plain = engine_in(plain_dir.path(), Some(plain_world), &plain_repo)
        .seat_input(&state(Some("work"), Cursor::Idle), "work", "effect")
        .unwrap();
    assert!(plain.get("house_rules").is_none());
}

/// Pinned AND embedded (decision 0023 ruling 4): the manifest carries
/// the map's content hash and the map itself, and the manifest rides
/// inside run/started — so the world a run believed in is answerable
/// from the journal alone.
#[test]
fn a_run_in_a_world_pins_the_maps_hash_and_embeds_the_map() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let world = world_over(dir.path(), &repo, "brokkr");
    let (digest, content) = (world.sha256.clone(), world.content.clone());
    let engine = engine_in(dir.path(), Some(world), &repo);

    let manifest = engine.store.manifest(&engine.run_id).unwrap();
    assert_eq!(manifest["realms"]["sha256"], json!(digest));
    assert_eq!(manifest["realms"]["map"], content);
    assert_eq!(manifest["realms"]["map"]["realms"][0]["name"], "brokkr");
    assert!(manifest["realms"]["source"]
        .as_str()
        .unwrap()
        .ends_with("realms.json"));
    // Everything else is the manifest this engine always wrote.
    assert_eq!(manifest["bundle_name"], "test");

    let started = &engine.store.load(&engine.run_id).unwrap()[0];
    assert_eq!(started.payload["manifest"], manifest);

    // And the pin never makes the run unresumable: resume compares the
    // BUNDLE manifest, which the map was never part of.
    assert_eq!(
        brokkr_core::dispatch::bundle_manifest_from_run(&manifest).unwrap(),
        engine.bundle.manifest
    );
}

/// A world that never drew a map notices nothing: the manifest is the
/// bundle manifest, with no key added anywhere.
#[test]
fn a_run_with_no_map_writes_exactly_the_manifest_it_always_did() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let engine = engine_in(dir.path(), None, &repo);
    let manifest = engine.store.manifest(&engine.run_id).unwrap();
    assert_eq!(manifest, engine.bundle.manifest);
    assert!(manifest.get("realms").is_none());
}

/// Ruling 5: the repository facts a decision records are keyed by the
/// realm the repository is — the heritage shape, ready for the day a
/// world has more than one.
#[test]
fn repository_facts_are_recorded_under_the_realm_name() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let world = world_over(dir.path(), &repo, "brokkr");
    let mut engine = engine_in(dir.path(), Some(world), &repo);

    let reviewed = git_commit(&repo, "reviewed");
    engine
        .decide(
            &state(Some("review"), Cursor::Idle),
            "effect",
            json!({"result":"clean"}),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(
        events[1].payload["inputs"]["reviewed_heads"],
        json!({ "brokkr": reviewed })
    );

    git_commit(&repo, "moved");
    let mut ship = state(Some("ship"), Cursor::Idle);
    ship.reviewed_heads = Some(json!({ "brokkr": reviewed }));
    engine
        .decide(&ship, "effect", json!({"result":"shipped"}))
        .unwrap();
    let inputs = engine.store.load(&engine.run_id).unwrap()[2].payload["inputs"].clone();
    assert_eq!(inputs["drift_detected"], json!(true));
    assert_eq!(inputs["dirty_worktrees"], json!(false));
    let facts = &inputs["realm_facts"]["brokkr"];
    assert_eq!(facts["drift_detected"], json!(true));
    assert_eq!(facts["dirty_worktrees"], json!(false));
    assert_eq!(facts["head"], json!(git_head(&repo).unwrap()));
}

/// Fail-closed at the gate: heads WERE recorded, but ship's repo no
/// longer resolves to a recorded realm — the drift question cannot be
/// answered, and an unanswerable question is drift, never silence.
/// (This run's own review caught the silent arm: resume --repo pointed
/// at an unmapped tree used to fall through SHIP-DRIFT to SHIP-OK.)
#[test]
fn an_unresolvable_realm_at_ship_is_drift_not_silence() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    let world = world_over(dir.path(), &repo, "brokkr");
    let mut engine = engine_in(dir.path(), Some(world), &elsewhere);
    let reviewed = git_commit(&repo, "reviewed");
    git_commit(&elsewhere, "unrelated");
    let mut ship = state(Some("ship"), Cursor::Idle);
    ship.reviewed_heads = Some(json!({ "brokkr": reviewed }));
    engine
        .decide(&ship, "effect", json!({"result":"shipped"}))
        .unwrap();
    let inputs = engine.store.load(&engine.run_id).unwrap()[1].payload["inputs"].clone();
    assert_eq!(
        inputs["drift_detected"],
        json!(true),
        "an unresolvable realm answers the drift question with drift"
    );
}

/// The both-shapes law at the gate that reads them: a head recorded
/// before any map — unkeyed — still answers for the one realm this run
/// works in, so an in-flight run keeps its drift check when a map is
/// drawn under it.
#[test]
fn a_head_recorded_before_the_map_still_drives_the_ship_gate() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let world = world_over(dir.path(), &repo, "brokkr");
    let mut engine = engine_in(dir.path(), Some(world), &repo);
    let reviewed = git_commit(&repo, "reviewed");
    let mut ship = state(Some("ship"), Cursor::Idle);
    ship.reviewed_heads = Some(json!({ "repo": reviewed }));
    engine
        .decide(&ship, "effect", json!({"result":"shipped"}))
        .unwrap();
    let inputs = engine.store.load(&engine.run_id).unwrap()[1].payload["inputs"].clone();
    assert_eq!(inputs["drift_detected"], json!(false));
    assert_eq!(
        inputs["realm_facts"]["brokkr"]["drift_detected"],
        json!(false)
    );
}

/// A repository the map does not name gets no realm: its facts are
/// recorded exactly as they were before any map existed, and no
/// per-realm key is invented for it.
#[test]
fn a_repository_the_map_does_not_name_keeps_the_unkeyed_facts() {
    let dir = tempfile::tempdir().unwrap();
    let mapped = dir.path().join("mapped");
    let stranger = dir.path().join("stranger");
    std::fs::create_dir(&mapped).unwrap();
    std::fs::create_dir(&stranger).unwrap();
    let world = world_over(dir.path(), &mapped, "brokkr");
    let mut engine = engine_in(dir.path(), Some(world), &stranger);
    let reviewed = git_commit(&stranger, "reviewed");
    engine
        .decide(
            &state(Some("review"), Cursor::Idle),
            "effect",
            json!({"result":"clean"}),
        )
        .unwrap();
    let inputs = engine.store.load(&engine.run_id).unwrap()[1].payload["inputs"].clone();
    assert_eq!(inputs["reviewed_heads"], json!({ "repo": reviewed }));
    assert!(inputs.get("realm_facts").is_none());
}

/// A realm whose tree has no git head, and a ship with nothing reviewed
/// to compare against: the facts that exist are recorded, the ones that
/// do not are absent rather than invented.
#[test]
fn realm_facts_state_only_what_the_tree_answers() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let world = world_over(dir.path(), &repo, "solo");
    let mut engine = engine_in(dir.path(), Some(world), &repo);
    engine
        .decide(
            &state(Some("ship"), Cursor::Idle),
            "effect",
            json!({"result":"shipped"}),
        )
        .unwrap();
    let inputs = engine.store.load(&engine.run_id).unwrap()[1].payload["inputs"].clone();
    // No repository there, so no head; and nothing reviewed, so no
    // drift. Both are absent from the realm's facts rather than
    // invented, and the dirty answer is the one the tree gave.
    assert!(inputs.get("drift_detected").is_none());
    let facts = &inputs["realm_facts"]["solo"];
    assert_eq!(facts["dirty_worktrees"], inputs["dirty_worktrees"]);
    assert!(facts.get("head").is_none());
    assert!(facts.get("drift_detected").is_none());
    // And a seat may not claim any of it.
    assert!(crate::bundle::is_engine_owned(crate::bundle::REALM_FACTS));
}

/// Ruling 5 must not degrade with the verb typed. `brokkr resume` takes
/// no map — it names a journal — so the world is rehydrated from the
/// run's own pin, and a resumed run keeps keying its facts by realm
/// instead of quietly reverting to the unkeyed shape mid-run.
#[test]
fn a_resumed_run_keeps_the_world_its_manifest_pinned() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let world = world_over(dir.path(), &repo, "brokkr");
    let engine = engine_in(dir.path(), Some(world), &repo);
    let (run_id, bundle) = (engine.run_id.clone(), engine.bundle.clone());

    // The map is DELETED before the resume: a run's answer about its own
    // world may not depend on the file still being there.
    std::fs::remove_file(dir.path().join("realms.json")).unwrap();
    let mut resumed = Engine::resume(engine.store, bundle, &run_id, Some(repo.clone())).unwrap();
    assert_eq!(
        resumed
            .world
            .as_ref()
            .map(|world| world.map.realms[0].name.as_str()),
        Some("brokkr")
    );

    let reviewed = git_commit(&repo, "reviewed");
    resumed
        .decide(
            &state(Some("review"), Cursor::Idle),
            "effect",
            json!({"result": "clean"}),
        )
        .unwrap();
    let events = resumed.store.load(&run_id).unwrap();
    assert_eq!(
        events.last().unwrap().payload["inputs"]["reviewed_heads"],
        json!({"brokkr": reviewed}),
        "a resumed run keys by realm exactly as the run that started it did"
    );
}

/// A run started with no map resumes with no world, and a manifest whose
/// pin no longer answers for itself refuses — the pin is the only copy
/// of that world, so a broken one is evidence of tampering rather than a
/// reason to guess.
#[test]
fn resume_carries_no_world_where_the_run_had_none_and_refuses_a_broken_pin() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let engine = engine_in(dir.path(), None, &repo);
    let (run_id, bundle) = (engine.run_id.clone(), engine.bundle.clone());
    assert!(Engine::resume(engine.store, bundle.clone(), &run_id, None)
        .unwrap()
        .world
        .is_none());

    let mut store = Store::open(&dir.path().join("tampered.db")).unwrap();
    let mut manifest = bundle.manifest.clone();
    manifest["realms"] = json!({
        "source": "realms.json",
        "sha256": "0".repeat(64),
        "map": {"schema": brokkr_core::realms::SCHEMA_V1,
                "realms": [{"name": "brokkr", "path": ".", "default_branch": "main"}],
                "journal": "forge.db"},
    });
    store
        .create_run("tampered", "feature", &bundle.name, &manifest)
        .unwrap();
    store
        .append_next(
            "tampered",
            EventType::RunStarted,
            json!({"feature": "feature", "manifest": manifest}),
            None,
            None,
        )
        .unwrap();
    match Engine::resume(store, bundle, "tampered", None) {
        Ok(_) => panic!("a pin that does not hash to its content must refuse"),
        Err(error) => assert!(error.to_string().contains("not the pinned"), "{error}"),
    }
}

/// A driver that records the `start` message it was handed before
/// answering with a fixed result. `driver_command` above is enough when
/// only the outcome matters; a sequence's later step is judged by what
/// it was TOLD, so this one keeps the evidence.
fn capturing_driver_command(
    effect_id: &str,
    attempt_id: &str,
    capture: &Path,
    result: Value,
) -> Vec<String> {
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
    let terminal = wire(Body::Result {
        effect_id: effect_id.into(),
        attempt_id: attempt_id.into(),
        status: ResultStatus::Succeeded,
        result: Some(result),
        error: None,
    });
    let path = capture.display();
    vec![
        "sh".into(),
        "-c".into(),
        format!(
            "read -r hello; printf '%s\\n' '{capabilities}'; read -r start; \
             printf '%s' \"$start\" > '{path}'; printf '%s\\n' '{accepted}'; \
             printf '%s\\n' '{terminal}'; read -r line"
        ),
    ]
}

#[test]
fn a_sequence_fake_driver_sees_step_results_then_the_seat_results() {
    let captures = tempfile::tempdir().unwrap();
    let first_capture = captures.path().join("first.json");
    let final_capture = captures.path().join("final.json");
    let final_peer_capture = captures.path().join("final-peer.json");
    let step_result = json!({"result": "drafted", "notes": "first"});
    let final_result = json!({"result": "pass", "notes": "final"});
    let steps = vec![
        SequenceStep {
            name: "draft".into(),
            class: SeatClass::Work,
            results: vec!["drafted".into(), "blocked".into()],
            body: StepBody::Single {
                role_path: "draft.md".into(),
                command: capturing_driver_command(
                    "vocabulary-effect",
                    "vocabulary-attempt",
                    &first_capture,
                    step_result,
                ),
                confine: None,
                candidates: Vec::new(),
            },
        },
        SequenceStep {
            name: "finish".into(),
            class: SeatClass::Work,
            results: vec!["pass".into(), "fail".into()],
            body: StepBody::Panel {
                members: vec![
                    member(
                        "final",
                        capturing_driver_command(
                            "vocabulary-effect",
                            "vocabulary-attempt",
                            &final_capture,
                            final_result.clone(),
                        ),
                    ),
                    member(
                        "peer",
                        capturing_driver_command(
                            "vocabulary-effect",
                            "vocabulary-attempt",
                            &final_peer_capture,
                            final_result,
                        ),
                    ),
                ],
                aggregate: Aggregate::UnanimousPass,
            },
        },
    ];
    let (dir, mut engine) = engine(SeatBody::Sequence {
        steps: steps.clone(),
    });
    let repo = dir.path().join("work");
    engine.world = Some(world_with_house(dir.path(), &repo, "One realm rule.\n"));
    engine.bundle.seats.get_mut("work").unwrap().results = vec!["pass".into(), "fail".into()];
    let seq_input = engine
        .seat_input(
            &state(Some("work"), Cursor::Idle),
            "work",
            "vocabulary-effect",
        )
        .unwrap();
    engine
        .execute_sequence(
            "vocabulary-effect",
            "vocabulary-attempt",
            "work",
            &steps,
            &seq_input,
            std::time::Duration::from_secs(10),
            &Selection::new(),
        )
        .unwrap();

    let first: Value = serde_json::from_slice(&std::fs::read(first_capture).unwrap()).unwrap();
    let final_step: Value = serde_json::from_slice(&std::fs::read(final_capture).unwrap()).unwrap();
    assert_eq!(
        first["input"]["allowed_results"],
        json!(["drafted", "blocked"])
    );
    assert_ne!(first["input"]["allowed_results"], json!(["pass", "fail"]));
    assert_eq!(
        final_step["input"]["allowed_results"],
        json!(["pass", "fail"])
    );
    assert_eq!(first["input"]["house_rules"], "One realm rule.\n");
    assert_eq!(
        final_step["input"]["house_rules"], "One realm rule.\n",
        "a sequence panel passes the realm house through to every member"
    );
    let first_prompt = brokkr_protocol::adapters::render_prompt(&first["input"]);
    let final_prompt = brokkr_protocol::adapters::render_prompt(&final_step["input"]);
    assert!(first_prompt.contains("<one of: drafted, blocked>"));
    assert!(!first_prompt.contains("<one of: pass, fail>"));
    assert!(final_prompt.contains("<one of: pass, fail>"));
}

#[test]
fn a_non_final_sequence_result_is_enforced_before_it_becomes_prior_context() {
    for (result, expected) in [
        (
            json!({"result": "invented"}),
            "outside its declared results",
        ),
        (json!({"notes": "missing word"}), "has no 'result' string"),
    ] {
        let steps = vec![
            SequenceStep {
                name: "draft".into(),
                class: SeatClass::Work,
                results: vec!["drafted".into()],
                body: StepBody::Single {
                    role_path: "draft.md".into(),
                    command: driver_command(
                        "vocabulary-effect",
                        "vocabulary-attempt",
                        AttemptOutcome::Succeeded { result },
                    ),
                    confine: None,
                    candidates: Vec::new(),
                },
            },
            SequenceStep {
                name: "finish".into(),
                class: SeatClass::Work,
                results: vec!["complete".into()],
                body: StepBody::Single {
                    role_path: "finish.md".into(),
                    command: vec!["must-not-run".into()],
                    confine: None,
                    candidates: Vec::new(),
                },
            },
        ];
        let (_dir, mut engine) = engine(SeatBody::Sequence {
            steps: steps.clone(),
        });
        let input = engine
            .seat_input(
                &state(Some("work"), Cursor::Idle),
                "work",
                "vocabulary-effect",
            )
            .unwrap();
        engine
            .execute_sequence(
                "vocabulary-effect",
                "vocabulary-attempt",
                "work",
                &steps,
                &input,
                std::time::Duration::from_secs(10),
                &Selection::new(),
            )
            .unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();
        let failure = events
            .iter()
            .find(|event| event.event_type == EventType::EffectFailed)
            .expect("the invalid intermediate result fails its effect");
        assert!(failure.payload["error"]
            .as_str()
            .unwrap()
            .contains(expected));
        assert!(!events.iter().any(|event| {
            event.event_type == EventType::EffectCheckpointed
                && event.payload["checkpoint"]["step_name"] == "draft"
        }));
    }
}

fn compiled_triage_engine() -> (tempfile::TempDir, Engine) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let dialect = crate::dialect::Dialect::load(&root.join("dialects/openspec.json"))
        .unwrap()
        .0;
    let mut bundle = Bundle::compile_with_realm(
        &root.join("recipes/triage"),
        &root.join("agents"),
        &root.join("adapters"),
        Some("brokkr"),
        Some(&dialect),
    )
    .unwrap();
    // These unit scenarios replace the compiled commands with the protocol
    // fake below; confinement itself has its dedicated boxed proof.
    bundle.hands.clear();
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let engine = Engine::start(store, bundle, "compiled SDD proof", Some(work)).unwrap();
    (dir, engine)
}

/// The real compiled triage design route records the dialect validator's
/// actual vocabulary. Therefore a chief's `upstream` ends the sequence,
/// reaches policy, returns to specify below the bound, and parks at it.
#[test]
fn compiled_design_upstream_reenters_specify_then_exhausts() {
    for (visits, expected_rule, expected_next) in [
        (2, "DESIGN-UPSTREAM", Some("specify")),
        (3, "DESIGN-UPSTREAM-EXHAUSTED", None),
    ] {
        let (_dir, mut engine) = compiled_triage_engine();
        let SeatBody::Sequence { mut steps } = engine.bundle.seats["design"].body.clone() else {
            panic!("compiled triage design is a sequence");
        };
        assert_eq!(steps.last().unwrap().results, ["drafted", "fail"]);
        for step in &mut steps {
            match (&step.name[..], &mut step.body) {
                ("positions", StepBody::Panel { members, .. }) => {
                    for member in members {
                        member.command = driver_command(
                            "upstream-effect",
                            "upstream-attempt",
                            AttemptOutcome::Succeeded {
                                result: json!({"result":"pass"}),
                            },
                        );
                        member.candidates.clear();
                    }
                }
                (
                    "chief",
                    StepBody::Single {
                        command,
                        candidates,
                        ..
                    },
                ) => {
                    *command = driver_command(
                        "upstream-effect",
                        "upstream-attempt",
                        AttemptOutcome::Succeeded {
                            result: json!({"result":"upstream", "notes":"the specification is at fault"}),
                        },
                    );
                    candidates.clear();
                }
                _ => {}
            }
        }
        let mut current = state(Some("design"), Cursor::Idle);
        current.visits.insert("specify".into(), visits);
        let input = engine
            .seat_input(&current, "design", "upstream-effect")
            .unwrap();
        engine
            .execute_sequence(
                "upstream-effect",
                "upstream-attempt",
                "design",
                &steps,
                &input,
                std::time::Duration::from_secs(10),
                &Selection::new(),
            )
            .unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();
        let result = events
            .iter()
            .find(|event| event.event_type == EventType::EffectSucceeded)
            .unwrap()
            .payload["result"]
            .clone();
        assert_eq!(result["result"], "upstream");
        assert!(!events
            .iter()
            .any(|event| event.payload["checkpoint"]["step_name"] == "validate"));

        engine.decide(&current, "upstream-effect", result).unwrap();
        let decided = engine.store.load(&engine.run_id).unwrap().pop().unwrap();
        assert_eq!(decided.payload["rule_id"], expected_rule);
        assert_eq!(decided.payload["next"].as_str(), expected_next);
    }
}

/// A deterministic non-zero loop check is evidence a later judge cannot
/// overrule. Clarify parks on a contradictory `clear`; analyze still lets
/// its judge classify the finding and routes on the returned `drift_in`.
#[test]
fn compiled_loop_check_failure_cannot_be_judged_away() {
    let (_dir, mut engine) = compiled_triage_engine();
    let SeatBody::Sequence { mut steps } = engine.bundle.seats["clarify"].body.clone() else {
        panic!("compiled triage clarify is a sequence");
    };
    let StepBody::Single {
        command,
        candidates,
        ..
    } = &mut steps[1].body
    else {
        panic!("the compiled judge is a single step");
    };
    *command = driver_command(
        "check-effect",
        "check-attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result":"clear"}),
        },
    );
    candidates.clear();
    let mut selection = Selection::new();
    selection.insert(
        Some("check".into()),
        Candidate {
            agent: "dialect".into(),
            model: "none".into(),
            effort: None,
            provider: "exec".into(),
            argv: driver_command(
                "check-effect",
                "check-attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"ambiguous"}),
                },
            ),
        },
    );
    let mut current = state(Some("clarify"), Cursor::Idle);
    current.phase_results.insert(
        "specify".into(),
        json!({"result":"drafted", "inputs":{"change":"proof-change"}}),
    );
    let input = engine
        .seat_input(&current, "clarify", "check-effect")
        .unwrap();
    engine
        .execute_sequence(
            "check-effect",
            "check-attempt",
            "clarify",
            &steps,
            &input,
            std::time::Duration::from_secs(10),
            &selection,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let contradiction = events
        .iter()
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .unwrap_or_else(|| panic!("the contradictory judge advanced: {events:#?}"));
    assert!(contradiction.payload["reason"]
        .as_str()
        .unwrap()
        .contains("contradicts deterministic step 'check' result 'ambiguous'"));

    let (_dir, mut engine) = compiled_triage_engine();
    let SeatBody::Sequence { mut steps } = engine.bundle.seats["clarify"].body.clone() else {
        panic!("compiled triage clarify is a sequence");
    };
    let StepBody::Single {
        command,
        candidates,
        ..
    } = &mut steps[1].body
    else {
        panic!("the compiled judge is a single step");
    };
    *command = driver_command(
        "clean-effect",
        "clean-attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result":"clear"}),
        },
    );
    candidates.clear();
    let mut selection = Selection::new();
    selection.insert(
        Some("check".into()),
        Candidate {
            agent: "dialect".into(),
            model: "none".into(),
            effort: None,
            provider: "exec".into(),
            argv: driver_command(
                "clean-effect",
                "clean-attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"clear"}),
                },
            ),
        },
    );
    let mut current = state(Some("clarify"), Cursor::Idle);
    current.phase_results.insert(
        "specify".into(),
        json!({"result":"drafted", "inputs":{"change":"proof-change"}}),
    );
    let input = engine
        .seat_input(&current, "clarify", "clean-effect")
        .unwrap();
    engine
        .execute_sequence(
            "clean-effect",
            "clean-attempt",
            "clarify",
            &steps,
            &input,
            std::time::Duration::from_secs(10),
            &selection,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(events.iter().any(|event| {
        event.event_type == EventType::EffectSucceeded
            && event.payload["result"]["result"] == "clear"
    }));

    let (_dir, mut engine) = compiled_triage_engine();
    let SeatBody::Sequence { mut steps } = engine.bundle.seats["analyze"].body.clone() else {
        panic!("compiled triage analyze is a sequence");
    };
    let StepBody::Single {
        command,
        candidates,
        ..
    } = &mut steps[1].body
    else {
        panic!("the compiled analyst is a single step");
    };
    *command = driver_command(
        "analyze-effect",
        "analyze-attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result":"drift", "inputs":{"drift_in":"design"}}),
        },
    );
    candidates.clear();
    let mut selection = Selection::new();
    selection.insert(
        Some("check".into()),
        Candidate {
            agent: "dialect".into(),
            model: "none".into(),
            effort: None,
            provider: "exec".into(),
            argv: driver_command(
                "analyze-effect",
                "analyze-attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"drift"}),
                },
            ),
        },
    );
    let mut current = state(Some("analyze"), Cursor::Idle);
    current.visits.insert("analyze".into(), 1);
    current.phase_results.insert(
        "specify".into(),
        json!({"result":"drafted", "inputs":{"change":"proof-change"}}),
    );
    let input = engine
        .seat_input(&current, "analyze", "analyze-effect")
        .unwrap();
    engine
        .execute_sequence(
            "analyze-effect",
            "analyze-attempt",
            "analyze",
            &steps,
            &input,
            std::time::Duration::from_secs(10),
            &selection,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let result = events
        .iter()
        .find(|event| event.event_type == EventType::EffectSucceeded)
        .unwrap_or_else(|| panic!("the analyst did not classify drift: {events:#?}"))
        .payload["result"]
        .clone();
    assert_eq!(result["inputs"]["drift_in"], "design");
    engine.decide(&current, "analyze-effect", result).unwrap();
    let decided = engine.store.load(&engine.run_id).unwrap().pop().unwrap();
    assert_eq!(decided.payload["rule_id"], "ANALYZE-DRIFT-DESIGN");
    assert_eq!(decided.payload["next"], "design");
}

/// Triage's design and engine routes review with a sequence: a positions panel, then
/// a single `chief` gate step. Because `positions` is NOT the final step
/// its `review-panel` output never reaches `decide()` — so the shape is
/// only safe if the panel's verdict arrives at the chief intact and the
/// chief's own result is what the effect reports.
///
/// Both halves are pinned here, including the uncomfortable one: the
/// chief CAN rule below the panel, and the engine will accept it. That
/// is why `agents/charters/review-chief.md` states the floor as
/// the seat's first law, and why this test asserts the mechanism rather
/// than pretending the engine forbids the lowering.
#[test]
fn chief_synthesis_carries_a_panel_security_hold_to_the_machine() {
    for (chief_rules, expected) in [("security-hold", "security-hold"), ("residual", "residual")] {
        let capture = tempfile::tempdir().unwrap();
        let start_line = capture.path().join("chief-start.json");
        let steps = vec![
            SequenceStep {
                name: "positions".into(),
                results: vec!["clean".into(), "residual".into(), "security-hold".into()],
                class: SeatClass::Work,
                body: StepBody::Panel {
                    members: vec![
                        member(
                            "correctness",
                            driver_command(
                                "review-effect",
                                "review-attempt",
                                AttemptOutcome::Succeeded {
                                    result: json!({
                                        "result": "residual",
                                        "inputs": {"max_residual_severity": "low",
                                                   "has_security_residual": false},
                                        "notes": "one untested branch",
                                    }),
                                },
                            ),
                        ),
                        member(
                            "security",
                            driver_command(
                                "review-effect",
                                "review-attempt",
                                AttemptOutcome::Succeeded {
                                    result: json!({
                                        "result": "security-hold",
                                        "inputs": {"max_residual_severity": "critical",
                                                   "has_security_residual": true},
                                        "notes": "unchecked input reaches the journal",
                                    }),
                                },
                            ),
                        ),
                    ],
                    aggregate: Aggregate::ReviewPanel,
                },
            },
            SequenceStep {
                name: "chief".into(),
                results: vec!["clean".into(), "residual".into(), "security-hold".into()],
                class: SeatClass::Work,
                body: StepBody::Single {
                    role_path: "chief.md".into(),
                    command: capturing_driver_command(
                        "review-effect",
                        "review-attempt",
                        &start_line,
                        json!({"result": chief_rules, "notes": "synthesised"}),
                    ),
                    confine: None,
                    candidates: Vec::new(),
                },
            },
        ];
        let seq_input = json!({
            "feature": "feature", "phase": "review", "workdir": ".",
            "allowed_results": ["clean", "residual", "security-hold"],
            "context": {},
            "steps": [
                {"name": "positions", "members": {
                    "correctness": {"role_path": "c.md", "result_path": "c.json"},
                    "security": {"role_path": "s.md", "result_path": "s.json"}}},
                {"name": "chief", "role_path": "chief.md", "result_path": "chief.json"},
            ],
        });

        let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
        engine
            .execute_sequence(
                "review-effect",
                "review-attempt",
                "review",
                &steps,
                &seq_input,
                std::time::Duration::from_secs(10),
                &Selection::new(),
            )
            .unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();

        // 1. The panel's worst-member verdict is journaled as the
        //    non-final step's checkpoint, not as the effect's result.
        let checkpoint = events
            .iter()
            .find(|event| {
                event.payload["checkpoint"]["step"] == "sequence-step-finished"
                    && event.payload["checkpoint"]["step_name"] == "positions"
            })
            .expect("the positions step checkpoints its joined verdict");
        assert_eq!(
            checkpoint.payload["checkpoint"]["result"]["result"], "security-hold",
            "review-panel takes the worst member; security-hold outranks residual"
        );

        // 2. It REACHES the chief, in its driver input, whole — verdict,
        //    maxed severity, OR-ed security flag and both members' notes.
        let start: Value = serde_json::from_slice(&std::fs::read(&start_line).unwrap()).unwrap();
        let positions = &start["input"]["context"]["prior_results"]["positions"];
        assert_eq!(positions["result"], "security-hold");
        assert_eq!(positions["inputs"]["has_security_residual"], true);
        assert_eq!(positions["inputs"]["max_residual_severity"], "critical");
        assert_eq!(positions["notes"]["verdicts"]["security"], "security-hold");
        assert_eq!(positions["notes"]["verdicts"]["correctness"], "residual");

        // 3. The FINAL step's result is the effect's typed result — the
        //    one decide() checks and the rule table rules on. Which is
        //    also the hazard: a chief that rules 'residual' over a panel
        //    'security-hold' is obeyed by the engine, so the floor lives
        //    in the chief's charter and nowhere else.
        let succeeded = events
            .iter()
            .find(|event| event.event_type == EventType::EffectSucceeded)
            .expect("the sequence concludes on its final step");
        assert_eq!(
            succeeded.payload["result"]["result"], expected,
            "the chief's result, never the panel's, is the seat's result"
        );
    }
}

// ── decision 0039: the review's own commits, classified ──────────────

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit_file(repo: &Path, path: &str, contents: &str, message: &str) -> String {
    let file = repo.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, contents).unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", message]);
    git_head(repo).unwrap()
}

const CLASSES: &str = ".github/delivery-classes.json";

const DOCS_CLASS: &str = r#"{"schema":"forge.delivery-classes/v1","classes":{"docs":{"paths":["(^|/)[^/]+\\.md$","^docs/","^assets/"]}}}"#;

/// A class whose one pattern matches every path — what a phase would
/// commit if it could choose the class its own fixes are judged by.
const EVERYTHING_IS_DOCS: &str = r#"{"classes":{"docs":{"paths":[""]}}}"#;

/// Append a `phase/entered` for the protected phase with the given
/// payload — what the engine writes (0039), or what a journal carries.
fn enter_review(engine: &mut Engine, payload: Value) {
    engine
        .store
        .append_next(&engine.run_id, EventType::PhaseEntered, payload, None, None)
        .unwrap();
}

/// Record that the protected phase was entered at the given head.
fn enter_review_at(engine: &mut Engine, head: &str) {
    enter_review(engine, json!({"phase": "review", "head": head}));
}

fn review_inputs(engine: &mut Engine, claim: Value) -> Map<String, Value> {
    engine
        .decide(
            &state(Some("review"), Cursor::Idle),
            "effect",
            json!({"result": "clean", "inputs": claim}),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    events.last().unwrap().payload["inputs"]
        .as_object()
        .cloned()
        .unwrap()
}

#[test]
fn a_returning_implement_exposes_its_docs_delta_and_takes_review_directly() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "classes");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = Bundle::compile_with(
        &root.join("bundles/self"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine = Engine::start(store, compiled, "docs return", Some(repo.clone())).unwrap();
    engine
        .store
        .append_next(
            &engine.run_id,
            EventType::PhaseEntered,
            json!({"phase": "implement", "head": entered}),
            None,
            None,
        )
        .unwrap();
    commit_file(&repo, "docs/answer.md", "# answer\n", "answer finding");
    let mut returned = state(Some("implement"), Cursor::Idle);
    returned.visits.insert("implement".into(), 2);
    returned.last_decision = Some(json!({"from": "review"}));
    engine
        .decide(
            &returned,
            "effect",
            json!({"result": "complete", "inputs": {"fixes_docs_only": false}}),
        )
        .unwrap();
    let event = engine.store.load(&engine.run_id).unwrap().pop().unwrap();
    assert_eq!(event.payload["inputs"]["fixes_docs_only"], true);
    assert_eq!(event.payload["rule_id"], "IMPL-OK-DOCS-RETURN");
    assert_eq!(event.payload["next"], "review");
}

#[test]
fn a_verify_fail_return_with_a_docs_delta_still_goes_through_verify() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "classes");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = Bundle::compile_with(
        &root.join("bundles/self"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine = Engine::start(store, compiled, "verify return", Some(repo.clone())).unwrap();
    engine
        .store
        .append_next(
            &engine.run_id,
            EventType::PhaseEntered,
            json!({"phase": "implement", "head": entered}),
            None,
            None,
        )
        .unwrap();
    commit_file(
        &repo,
        "docs/answer.md",
        "# answer\n",
        "answer verify failure",
    );
    let mut returned = state(Some("implement"), Cursor::Idle);
    returned.visits.insert("implement".into(), 2);
    returned.last_decision = Some(json!({"from": "verify"}));
    engine
        .decide(
            &returned,
            "effect",
            json!({"result": "complete", "inputs": {}}),
        )
        .unwrap();
    let event = engine.store.load(&engine.run_id).unwrap().pop().unwrap();
    assert!(event.payload["inputs"].get("fixes_docs_only").is_none());
    assert_eq!(event.payload["rule_id"], "IMPL-OK");
    assert_eq!(event.payload["next"], "verify");
}

#[test]
fn a_review_return_exposes_no_docs_fact_without_both_heads() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    commit_file(&repo, CLASSES, DOCS_CLASS, "classes");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = Bundle::compile_with(
        &root.join("bundles/self"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine = Engine::start(store, compiled, "review return", Some(repo.clone())).unwrap();
    let mut returned = state(Some("implement"), Cursor::Idle);
    returned.last_decision = Some(json!({"from": "review"}));

    engine
        .decide(&returned, "first", json!({"result": "complete"}))
        .unwrap();
    let event = engine.store.load(&engine.run_id).unwrap().pop().unwrap();
    assert!(event.payload["inputs"].get("fixes_docs_only").is_none());

    std::fs::remove_dir_all(repo.join(".git")).unwrap();
    engine
        .decide(&returned, "second", json!({"result": "complete"}))
        .unwrap();
    let event = engine.store.load(&engine.run_id).unwrap().pop().unwrap();
    assert!(event.payload["inputs"].get("fixes_docs_only").is_none());
}

/// Ruling 1: the protected phase and a returning implement carry their
/// entry heads; first-visit work and an unreadable repository carry none.
#[test]
fn the_protected_phase_is_entered_at_a_recorded_head() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let probe = engine_in(dir.path(), None, &repo);
    let first_visit = state(None, Cursor::Idle);
    // Not a git repository yet: no head to record.
    assert_eq!(
        probe.phase_entered_payload("review", &first_visit),
        json!({"phase": "review"})
    );
    let head = git_commit(&repo, "base");
    assert_eq!(
        probe.phase_entered_payload("review", &first_visit),
        json!({"phase": "review", "head": head})
    );
    assert_eq!(
        probe.phase_entered_payload("implement", &first_visit),
        json!({"phase": "implement"})
    );
    let mut returned = first_visit.clone();
    returned.visits.insert("implement".into(), 1);
    assert_eq!(
        probe.phase_entered_payload("implement", &returned),
        json!({"phase": "implement", "head": git_head(&repo).unwrap()})
    );

    // Driven, not hand-fed: the first phase is entered without a head,
    // because the first phase is not the protected one.
    let (_dir, mut engine) = engine(single_body(vec!["driver".into()]));
    engine.drive_once().unwrap();
    let entered = &engine.store.load(&engine.run_id).unwrap()[1];
    assert_eq!(entered.event_type, EventType::PhaseEntered);
    assert!(entered.payload.get("head").is_none());
}

/// Rulings 2 and 3: docs-only commits read true, a code commit reads
/// false, and the seat's own claim is overwritten either way.
#[test]
fn docs_only_review_commits_are_classified_and_never_claimed() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "classes");
    let mut engine = engine_in(dir.path(), None, &repo);

    enter_review_at(&mut engine, &entered);
    commit_file(&repo, "docs/page.md", "# page\n", "a page");
    commit_file(&repo, "README.md", "# readme\n", "the readme");
    let inputs = review_inputs(&mut engine, json!({"fixes_docs_only": false}));
    assert_eq!(inputs["fixes_docs_only"], json!(true), "{inputs:?}");

    let entered = git_head(&repo).unwrap();
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, "docs/page.md", "# page\n\nmore\n", "more prose");
    commit_file(&repo, "src/lib.rs", "fn f() {}\n", "and code");
    let inputs = review_inputs(&mut engine, json!({"fixes_docs_only": true}));
    assert_eq!(inputs["fixes_docs_only"], json!(false), "{inputs:?}");

    // Renames are unpaired: a code file moved under a docs name is a
    // deletion in `src/` and an addition in `docs/`, and the deletion is
    // code. Paired, git would report one docs path and the answer would
    // be a lie — the same trap the contribution gate closes (0038).
    let entered = git_head(&repo).unwrap();
    enter_review_at(&mut engine, &entered);
    git(&repo, &["mv", "src/lib.rs", "docs/lib.md"]);
    git(&repo, &["commit", "-q", "-m", "move the code under docs"]);
    let inputs = review_inputs(&mut engine, json!({"fixes_docs_only": true}));
    assert_eq!(inputs["fixes_docs_only"], json!(false), "{inputs:?}");

    // A docs name that is not ASCII is looked up as itself. Porcelain
    // `git diff` would quote it — `"docs/caf\303\251.md"` — into a
    // path no pattern matches, and an honest prose fix would buy a
    // verify; the diff is the anchor's plumbing, and the gate's.
    let entered = git_head(&repo).unwrap();
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, "docs/café.md", "# café\n", "prose, accented");
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(true), "{inputs:?}");
}

/// Ruling 3: the class is the one committed at the entry head, so the
/// phase being judged cannot widen the class its own fixes are judged
/// by — neither in the working tree nor in a commit of its own, where the
/// class file is a path classified by the class it was entered under.
#[test]
fn the_docs_class_is_read_at_the_entry_head_and_not_from_the_tree() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "classes");
    let mut engine = engine_in(dir.path(), None, &repo);

    // Widened in the working tree only: the tree is not consulted, so a
    // prose fix still reads true and a code fix still reads false.
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, "docs/page.md", "# page\n", "a page");
    std::fs::write(repo.join(CLASSES), EVERYTHING_IS_DOCS).unwrap();
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(true), "{inputs:?}");
    git(&repo, &["checkout", "-q", "--", CLASSES]);
    commit_file(&repo, "src/lib.rs", "fn f() {}\n", "and code");
    std::fs::write(repo.join(CLASSES), EVERYTHING_IS_DOCS).unwrap();
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(false), "{inputs:?}");
    git(&repo, &["checkout", "-q", "--", CLASSES]);

    // Widened in a commit of the phase's own, and nothing else touched:
    // the class file is under `.github/`, which the class it was entered
    // under calls code.
    let entered = git_head(&repo).unwrap();
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, CLASSES, EVERYTHING_IS_DOCS, "widen the class");
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(false), "{inputs:?}");

    // Widened alongside code, entered under the honest class: false,
    // for both paths.
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "narrow it back");
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, CLASSES, EVERYTHING_IS_DOCS, "widen the class again");
    commit_file(&repo, "src/lib.rs", "fn g() {}\n", "code, judged as docs?");
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(false), "{inputs:?}");

    // The boundary, stated: a widening that read false above bought the
    // verify and the review that judge it as the code change it is, and
    // only THEN does it govern an entry. A run's later visit is entered
    // under a class its earlier visits were judged for — and the pull
    // request gate still reads the base branch's copy, never this one.
    let entered = git_head(&repo).unwrap();
    enter_review_at(&mut engine, &entered);
    commit_file(
        &repo,
        "src/lib.rs",
        "fn h() {}\n",
        "code under the judged widening",
    );
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(true), "{inputs:?}");

    // An empty class declared at the entry head is a class that calls
    // every path code: false, not absent — the honest answer to a
    // repository that says nothing of it is prose.
    let entered = commit_file(
        &repo,
        CLASSES,
        r#"{"classes":{"docs":{"paths":[]}}}"#,
        "nothing is docs",
    );
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, "docs/page.md", "# page\n\nagain\n", "a page, again");
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(false), "{inputs:?}");
}

/// Ruling 3's absences: the input is left out whenever the question has
/// no honest answer, and an absent input never satisfies a rule.
#[test]
fn fixes_docs_only_is_absent_when_the_question_has_no_answer() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "classes");
    let mut engine = engine_in(dir.path(), None, &repo);

    // No entry recorded for the phase at all.
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");

    // Entered at the current head: nothing committed since.
    enter_review_at(&mut engine, &entered);
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");

    // A commit that touches nothing.
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "empty"]);
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");

    // Real commits, but the class committed at the entry head does not
    // parse, is not a regular expression, declares no docs class, or
    // declares a pattern that is not one.
    for broken in [
        "{not json",
        r#"{"classes":{"docs":{"paths":["("]}}}"#,
        r#"{"classes":{}}"#,
        r#"{"classes":{"docs":{"paths":[7]}}}"#,
    ] {
        let entered = commit_file(&repo, CLASSES, broken, "a class that is not one");
        enter_review_at(&mut engine, &entered);
        commit_file(&repo, "docs/page.md", broken, "a page");
        let inputs = review_inputs(&mut engine, json!({}));
        assert!(
            inputs.get("fixes_docs_only").is_none(),
            "{broken}: {inputs:?}"
        );
    }
    // No class file at the entry head at all.
    git(&repo, &["rm", "-q", CLASSES]);
    git(&repo, &["commit", "-q", "-m", "no class"]);
    enter_review_at(&mut engine, &git_head(&repo).unwrap());
    commit_file(&repo, "docs/page.md", "# page\n", "a page");
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");
    // And a class declared again answers again.
    let entered = commit_file(&repo, CLASSES, DOCS_CLASS, "classes, restored");
    enter_review_at(&mut engine, &entered);
    commit_file(&repo, "docs/page.md", "# page\n\nmore\n", "more prose");
    let inputs = review_inputs(&mut engine, json!({}));
    assert_eq!(inputs["fixes_docs_only"], json!(true), "{inputs:?}");

    // The phase's LATEST entry recorded no head: an older visit's head
    // is not borrowed, even though borrowing would read true here —
    // the commits since that older entry are not this visit's.
    enter_review(&mut engine, json!({"phase": "review"}));
    commit_file(
        &repo,
        "docs/page.md",
        "# page\n\nstill more\n",
        "still more prose",
    );
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");

    // An entry head that is not the shape the contract promises never
    // reaches git: a journal row is not an argument list.
    let stolen = dir.path().join("stolen");
    enter_review_at(&mut engine, &format!("--output={}", stolen.display()));
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");
    assert!(
        !stolen.exists(),
        "the journal named a file and git wrote it"
    );
    enter_review_at(&mut engine, &entered.to_ascii_uppercase());
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");

    // An entry head git does not have.
    enter_review_at(&mut engine, &"d".repeat(40));
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");

    // A repository whose head can no longer be read.
    enter_review_at(&mut engine, &entered);
    std::fs::remove_dir_all(repo.join(".git")).unwrap();
    let inputs = review_inputs(&mut engine, json!({}));
    assert!(inputs.get("fixes_docs_only").is_none(), "{inputs:?}");
}

// ── // ── decision 0043: the hands go in the box at spawn ─────────────────

#[test]
fn a_site_without_hands_spawns_its_command_untouched() {
    let command = vec!["claude".to_string(), "--model".to_string(), "x".to_string()];
    assert_eq!(
        hands_command(command.clone(), None, Path::new("/work"), &[]),
        command
    );
}

#[test]
fn a_model_seat_with_hands_gets_the_server_config_expanded_into_its_argv() {
    let spec = brokkr_protocol::hands::HandsSpec::parse(&json!({
        "kind": "workspace", "network": true,
        "binds": [{"path": "~/.cargo", "mode": "rw", "mask": ["credentials.toml"]}]
    }))
    .unwrap();
    let argv = hands_command(
        vec![
            "{brokkr}".to_string(),
            "driver".to_string(),
            "claude".to_string(),
            "--mcp-config".to_string(),
            "{hands_mcp_json}".to_string(),
            "-c".to_string(),
            "mcp_servers.brokkr.args={hands_args_toml}".to_string(),
        ],
        Some(&spec),
        Path::new("/work"),
        &[],
    );
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(argv[0], exe, "the {{brokkr}} token names this binary");
    let config: Value = serde_json::from_str(&argv[4]).unwrap();
    assert_eq!(config["mcpServers"]["brokkr"]["command"], exe);
    assert_eq!(config["mcpServers"]["brokkr"]["args"][3], "/work");
    let toml = argv[6].strip_prefix("mcp_servers.brokkr.args=").unwrap();
    assert!(
        toml.starts_with("[\"hands\",\"serve\",\"--workdir\",\"/work\",\"--spec\",\""),
        "{toml}"
    );
    assert!(
        toml.contains("\\\"network\\\":true"),
        "quotes inside the spec are TOML-escaped: {toml}"
    );
}

#[test]
fn only_an_exec_dispatch_is_boxed_whole() {
    let spec = brokkr_protocol::hands::HandsSpec::default();
    let short = vec!["true".to_string()];
    assert_eq!(
        hands_command(short.clone(), Some(&spec), Path::new("/w"), &[]),
        short
    );
    let not_a_dispatch = vec!["a".to_string(), "b".to_string(), "exec".to_string()];
    assert_eq!(
        hands_command(not_a_dispatch.clone(), Some(&spec), Path::new("/w"), &[],),
        not_a_dispatch
    );
    let other_driver = vec!["x".to_string(), "driver".to_string(), "claude".to_string()];
    assert_eq!(
        hands_command(other_driver.clone(), Some(&spec), Path::new("/w"), &[],),
        other_driver
    );
}

#[test]
fn an_exec_seat_with_hands_is_boxed_whole() {
    let spec = brokkr_protocol::hands::HandsSpec::default();
    let inner = vec![
        "/usr/local/bin/brokkr".to_string(),
        "driver".to_string(),
        "exec".to_string(),
        "--".to_string(),
        "bash".to_string(),
        "/bundle/scripts/verify.sh".to_string(),
        "{prompt_file}".to_string(),
    ];
    let argv = hands_command(
        inner,
        Some(&spec),
        Path::new("/work"),
        &[PathBuf::from("/bundle")],
    );
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(&argv[..3], [exe.as_str(), "hands", "exec"]);
    assert_eq!(&argv[3..5], ["--workdir", "/work"]);
    assert_eq!(argv[5], "--spec");
    assert_eq!(&argv[7..9], ["--bundle-root", "/bundle"]);
    assert_eq!(argv[9], "--");
    assert_eq!(argv[10], "/usr/local/bin/brokkr");
    assert_eq!(argv[15], "/runtime/bundle/scripts/verify.sh");
    assert_eq!(argv[16], "{prompt_file}");

    // The helper also remains total for an invented bundle with no
    // roots; production bundles always carry at least their leaf root.
    let rootless_inner = vec![
        "/usr/local/bin/brokkr".to_string(),
        "driver".to_string(),
        "exec".to_string(),
        "--".to_string(),
        "true".to_string(),
    ];
    let rootless = hands_command(rootless_inner.clone(), Some(&spec), Path::new("/work"), &[]);
    assert_eq!(rootless[7], "--");
    assert_eq!(&rootless[8..], &rootless_inner);
}
