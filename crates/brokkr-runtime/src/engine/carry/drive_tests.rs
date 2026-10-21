use super::fixture_tests::*;
use super::*;
use crate::engine::{FencedCommandOutcome, MemberRetryOutcome};
use brokkr_core::EventType;
use serde_json::json;

#[test]
fn recovery_rebuilds_the_request_and_carries_only_journaled_verdicts() {
    let (_dir, mut runtime) = parked(body(), false);
    assert!(matches!(
        retry(&mut runtime, &["b"]),
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    runtime.drive_once().unwrap();
    let journal = events(&runtime);
    let state = brokkr_core::fold::fold(&journal).unwrap();
    let requested = journal.last().unwrap();
    let effect = requested.payload["effect_id"].as_str().unwrap();
    let plan = runtime
        .carry_selection(&journal, &state, effect)
        .unwrap()
        .unwrap();
    let store = brokkr_store::Store::open(runtime.store.path()).unwrap();
    let mut resumed = Engine::resume(
        store,
        runtime.bundle.clone(),
        &runtime.run_id,
        Some(runtime.repo.clone()),
    )
    .unwrap();
    assert_eq!(
        brokkr_core::canonical::sha256_hex(&resumed.seat_input(&state, "work", effect).unwrap()),
        requested.payload["input_digest"].as_str().unwrap()
    );
    assert_eq!(
        serde_json::to_value(plan.carried()).unwrap(),
        serde_json::to_value(
            resumed
                .carry_selection(&journal, &state, effect)
                .unwrap()
                .unwrap()
                .carried()
        )
        .unwrap()
    );
    // There is no source result file: recovery must read the verdict in the journal.
    let results = resumed.run_local().join("results");
    if results.exists() {
        std::fs::remove_dir_all(results).unwrap();
    }
    resumed.drive_once().unwrap();
    let journal = events(&resumed);
    let start = journal
        .iter()
        .rev()
        .find(|e| e.event_type == EventType::EffectStarted)
        .unwrap();
    assert_eq!(
        start.payload["carried"],
        json!([{"member":"a","attempt_id":"source"}])
    );
    let markers = markers(&journal, &start.payload["attempt_id"]);
    assert_eq!(markers, [json!("b")]);
    assert_eq!(
        journal.last().unwrap().event_type,
        EventType::EffectSucceeded
    );
    assert_eq!(
        journal.last().unwrap().payload["result"]["notes"]["verdicts"]["a"],
        "clean"
    );
}

#[test]
fn accepted_args_and_secret_store_drift_park_before_a_member_spawns() {
    for args in [
        json!({"members":null}),
        json!({"members":[]}),
        json!({"members":"b"}),
        json!({"other":true}),
        json!({"members":["b"]}),
    ] {
        let (dir, mut runtime) = parked(body(), true);
        append(
            &mut runtime,
            EventType::OperatorCommanded,
            json!({"command_id":"retry","command":"retry","args":args,"operator":"test"}),
        );
        let command_seq = events(&runtime).last().unwrap().seq;
        append(
            &mut runtime,
            EventType::OperatorAccepted,
            json!({"command_id":"retry","operator":"test","reason":"again"}),
        );
        if args == json!({"members":["b"]}) {
            runtime.secrets_file = Some(dir.path().join("other.env"));
        }
        let end = runtime.drive().unwrap();
        assert_eq!(
            end.state.run.status(),
            brokkr_core::fold::Status::AwaitingOperator
        );
        let journal = events(&runtime);
        let terminal = journal
            .iter()
            .rev()
            .find(|e| e.event_type == EventType::EffectIndeterminate)
            .unwrap();
        let reason = terminal.payload["reason"].as_str().unwrap();
        if args == json!({"members":["b"]}) {
            assert!(reason.starts_with("inputs moved for effects "), "{reason}");
        } else {
            assert!(
                reason.starts_with(&format!(
                    "command at sequence {command_seq} field 'args' does not read:"
                )),
                "{reason}"
            );
        }
        let started = journal
            .iter()
            .rev()
            .find(|e| e.event_type == EventType::EffectStarted)
            .unwrap();
        assert_eq!(started.payload["driver"], "none");
        assert_eq!(started.payload.get("carried"), None);
        assert_eq!(journal.last().unwrap().payload["evidence"], json!({}));
        assert_eq!(
            end.state.run.park_reason().unwrap().as_str(),
            format!(
                "effect {} indeterminate: {reason}",
                terminal.payload["effect_id"].as_str().unwrap()
            )
        );
    }
}

fn sequence(fresh: AttemptOutcome, chief: Option<AttemptOutcome>) -> SeatBody {
    use crate::bundle::{SeatClass, SequenceStep};
    let mut steps = vec![SequenceStep {
        name: "positions".into(),
        class: SeatClass::Work,
        results: vec!["clean".into(), "residual".into(), "security-hold".into()],
        body: StepBody::Invocation(Invocation::Panel {
            members: vec![
                crate::engine::tests::member(
                    "a",
                    command(AttemptOutcome::Succeeded {
                        result: json!({"result":"clean"}),
                    }),
                ),
                crate::engine::tests::member("b", command(fresh)),
            ],
            aggregate: Aggregate::ReviewPanel,
        }),
    }];
    if let Some(chief) = chief {
        steps.push(SequenceStep {
            name: "chief".into(),
            class: SeatClass::Work,
            results: vec![],
            body: crate::engine::tests::step_body(command(chief)),
        });
    }
    SeatBody::Sequence { steps }
}

fn rename_source(record: &mut Value) {
    let result = record["settled_members"]["a"].take();
    record["settled_members"] = json!({"positions:a":result});
}

fn execute_retry(runtime: &mut Engine, name: &str) {
    assert!(matches!(
        retry(runtime, &[name]),
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    runtime.drive_once().unwrap();
    runtime.drive_once().unwrap();
}

#[test]
fn sequence_tags_carry_into_the_chief_and_only_the_panel_stamps_its_own_terminal() {
    for end_at_panel in [false, true] {
        let clean = || AttemptOutcome::Succeeded {
            result: json!({"result":"clean"}),
        };
        let lost = || AttemptOutcome::Indeterminate {
            reason: "vanished".into(),
        };
        let body = sequence(
            if end_at_panel { lost() } else { clean() },
            (!end_at_panel).then(lost),
        );
        let (_dir, mut runtime) = parked_record(body, false, rename_source);
        assert!(
            matches!(retry(&mut runtime, &["b"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::UnknownMember { tags, .. })) if tags == "positions:a, positions:b")
        );
        execute_retry(&mut runtime, "positions:b");
        let journal = events(&runtime);
        let terminal = journal.last().unwrap();
        assert_eq!(terminal.event_type, EventType::EffectIndeterminate);
        assert_eq!(terminal.payload.get("tree").is_some(), end_at_panel);
        assert_eq!(terminal.payload.get("head").is_some(), end_at_panel);
        let started = journal
            .iter()
            .rev()
            .find(|e| e.event_type == EventType::EffectStarted)
            .unwrap();
        assert_eq!(
            started.payload["carried"],
            json!([{"member":"positions:a", "attempt_id":"source"}])
        );
        let markers = markers(&journal, &started.payload["attempt_id"]);
        assert_eq!(markers, [json!("positions:b")]);
        if !end_at_panel {
            let step = journal
                .iter()
                .find(|e| e.payload["checkpoint"]["step"] == "sequence-step-finished")
                .unwrap();
            assert_eq!(
                step.payload["checkpoint"]["result"]["notes"]["verdicts"],
                json!({"a":"clean","b":"clean"})
            );
            runtime.drive().unwrap();
            assert!(
                matches!(retry(&mut runtime, &["positions:b"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::Shape { why: Shape::EndedAtLaterStep { step, .. } })) if step == "chief")
            );
            assert!(matches!(
                retry(&mut runtime, &["positions:b", "positions:a"]),
                MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
            ));
        }
    }
}

fn moving_body(action: &str, gate: bool) -> SeatBody {
    let mut body = body();
    let SeatBody::Invocation(Invocation::Panel { members, .. }) = &mut body else {
        unreachable!()
    };
    for member in members.iter_mut() {
        member.class = if gate {
            crate::bundle::SeatClass::Gate
        } else {
            crate::bundle::SeatClass::Work
        };
    }
    let script = &mut members[1].command[2];
    let at = script.rfind("; printf").unwrap();
    script.insert_str(at, &format!("; {action}"));
    body
}

#[test]
fn a_fresh_members_commit_or_edit_cannot_publish_a_mixed_subject_aggregate() {
    for (action, gate, moved_head) in [
        ("git commit -q --allow-empty -m fresh-commit", false, true),
        ("printf edited > base.txt", true, false),
        ("git commit -q --allow-empty -m gate-commit", true, true),
    ] {
        let (_dir, mut runtime) = parked(moving_body(action, gate), false);
        runtime.bundle.seats.get_mut("work").unwrap().has_gate = gate;
        execute_retry(&mut runtime, "b");
        let journal = events(&runtime);
        let terminal = journal.last().unwrap();
        assert_eq!(terminal.event_type, EventType::EffectIndeterminate);
        let reason = terminal.payload["reason"].as_str().unwrap();
        if gate && moved_head {
            assert!(reason.starts_with("GATE-MOVED-HEAD"), "{reason}");
            assert_eq!(terminal.payload.get("tree"), None);
        } else {
            assert!(reason.starts_with("carrying attempt '"), "{reason}");
            assert_eq!(terminal.payload["tree"]["at_start"], "clean");
            assert_eq!(
                terminal.payload["tree"]["at_end"],
                if moved_head { "clean" } else { "dirty" }
            );
            assert_eq!(terminal.payload["settled_members"]["b"]["result"], "clean");
            assert_eq!(
                terminal.payload["head"]["at_start"] != terminal.payload["head"]["at_end"],
                moved_head
            );
        }
        let end = runtime.drive().unwrap();
        assert_eq!(
            end.state.run.status(),
            brokkr_core::fold::Status::AwaitingOperator
        );
        assert!(!events(&runtime)
            .iter()
            .any(|e| e.event_type == EventType::EffectSucceeded));
        if !(gate && moved_head) {
            let refusal = retry(&mut runtime, &["b"]);
            assert!(matches!(
                refusal,
                MemberRetryOutcome::CarryRefused(CarryRefusal(
                    Refusal::HeadMoved { .. } | Refusal::TreeNotClean { .. }
                ))
            ));
        }
        let outcome = crate::engine::operator_command(
            &mut runtime.store,
            &runtime.run_id,
            brokkr_core::fold::OperatorCommand::Retry,
            "test",
            "whole panel",
        )
        .unwrap();
        assert!(matches!(outcome, FencedCommandOutcome::Accepted { .. }));
    }
}

#[test]
fn bounded_chain_fallback_keeps_original_carry_and_reruns_every_named_member() {
    let mut body = body();
    let SeatBody::Invocation(Invocation::Panel { members, .. }) = &mut body else {
        unreachable!()
    };
    let mut c = members[1].clone();
    c.name = "c".into();
    for (model, argv) in [
        ("first", vec!["missing-synthetic-driver".into()]),
        ("second", c.command.clone()),
    ] {
        let mut link = crate::engine::agent_tests::candidate("fresh-reviewer", model);
        link.argv = argv;
        c.candidates.push(crate::engine::tests::templated(link));
    }
    members.push(c);
    let (_dir, mut runtime) = parked(body, false);
    assert!(matches!(
        retry(&mut runtime, &["c", "b"]),
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    runtime.drive_once().unwrap();
    runtime.drive_once().unwrap();
    assert_eq!(
        events(&runtime).last().unwrap().event_type,
        EventType::EffectFailed
    );
    assert_eq!(
        events(&runtime).last().unwrap().payload["settled_members"]["b"]["result"],
        "clean"
    );
    runtime.drive_once().unwrap();
    let journal = events(&runtime);
    assert_eq!(
        journal.last().unwrap().event_type,
        EventType::EffectSucceeded
    );
    let starts: Vec<_> = journal
        .iter()
        .filter(|e| e.event_type == EventType::EffectStarted && e.payload.get("carried").is_some())
        .collect();
    assert_eq!(starts.len(), 2);
    for start in &starts {
        assert_eq!(
            start.payload["carried"],
            json!([{"member":"a","attempt_id":"source"}])
        );
        let ran = markers(&journal, &start.payload["attempt_id"]);
        assert_eq!(ran, [json!("b"), json!("c")]);
    }
    assert_eq!(starts[0].payload["provenance"][0]["member"], "c");
    assert_eq!(starts[0].payload["provenance"][0]["model"], "first");
    assert_eq!(starts[1].payload["provenance"][0]["model"], "second");
}

fn markers(journal: &[EventEnvelope], attempt: &Value) -> Vec<Value> {
    journal
        .iter()
        .filter(|e| {
            e.payload["attempt_id"] == *attempt
                && e.payload["checkpoint"]["step"] == "panel-member-finished"
        })
        .map(|e| e.payload["checkpoint"]["member"].clone())
        .collect()
}
