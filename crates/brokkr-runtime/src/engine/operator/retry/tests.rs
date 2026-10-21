use super::*;
use crate::engine::carry::fixture_tests::*;
use crate::engine::carry::{RecordId, Refusal};
use crate::engine::tests::git_commit;
use crate::engine::SeatBody;
use brokkr_core::{fold::Refusal as FoldRefusal, EventType};
use serde_json::{json, Value};

#[test]
fn member_admission_keeps_order_checks_secret_inputs_and_writes_nothing_on_refusal() {
    let (dir, mut runtime) = parked(body(), true);
    let before = events(&runtime).len();
    for store in [None, Some(dir.path().join("other.env"))] {
        runtime.secrets_file = store;
        assert!(
            matches!(retry(&mut runtime, &["b"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::InputMoved { effects }))
            if effects == events(&runtime).iter().find(|e| e.event_type == EventType::EffectRequested).unwrap().payload["effect_id"].as_str().unwrap())
        );
        assert_eq!(events(&runtime).len(), before);
    }
    runtime.secrets_file = Some(dir.path().join("ops.env"));
    assert!(matches!(
        retry(&mut runtime, &["b", "a"]),
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    let journal = events(&runtime);
    let commanded = journal
        .iter()
        .rev()
        .find(|e| e.event_type == EventType::OperatorCommanded)
        .unwrap();
    assert_eq!(commanded.payload["args"], json!({"members":["a","b"]}));
    let tip = journal.len();
    assert!(matches!(
        Engine::read_member_retry(&mut runtime.store, &runtime.run_id).unwrap(),
        Err(MemberRetryOutcome::FoldRefused(
            FoldRefusal::RunNotAwaitingOperator
        ))
    ));
    assert_eq!(events(&runtime).len(), tip);
}

#[test]
fn member_refusals_precede_writes_and_unread_records_precede_building_the_engine() {
    let (_dir, mut runtime) = parked(body(), false);
    let before = events(&runtime).len();
    assert!(
        matches!(retry(&mut runtime, &["missing"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::UnknownMember { name, tags })) if name == "missing" && tags == "a, b")
    );
    assert!(
        matches!(retry(&mut runtime, &["a"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::Unsettled { members })) if members == "b")
    );
    git_commit(&runtime.repo, "moved");
    assert!(matches!(
        retry(&mut runtime, &["b"]),
        MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::HeadMoved { .. }))
    ));
    assert_eq!(events(&runtime).len(), before);
    // A malformed terminal is journal-edge data, refused even when all members are named.
    let (_dir, mut unread_runtime) = parked_record(body(), false, |record| {
        record["tree"] = json!({"at_start":"unknown","at_end":"clean"});
    });
    assert!(
        matches!(Engine::read_member_retry(&mut unread_runtime.store, &unread_runtime.run_id).unwrap(),
        Err(MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::RecordUnread { record: RecordId::Attempt(id), field, .. }))) if id == "source" && field == "tree")
    );
}

fn fold_refusal(runtime: &mut Engine) -> FoldRefusal {
    let before = events(runtime).len();
    let outcome = Engine::read_member_retry(&mut runtime.store, &runtime.run_id).unwrap();
    assert_eq!(events(runtime).len(), before);
    match outcome {
        Err(MemberRetryOutcome::FoldRefused(refusal)) => refusal,
        Err(other) => panic!("{other:?}"),
        Ok(_) => panic!("admitted"),
    }
}

#[test]
fn every_fold_word_a_retry_can_meet_refuses_before_anything_is_written() {
    let (_dir, mut runtime) = crate::engine::tests::engine(body());
    runtime.drive_once().unwrap();
    assert_eq!(
        fold_refusal(&mut runtime),
        FoldRefusal::RunNotAwaitingOperator
    );
    let (_dir, mut runtime) = crate::engine::tests::engine(SeatBody::Select {
        cases: Default::default(),
        default: None,
    });
    assert_eq!(
        runtime.drive().unwrap().state.run.status(),
        brokkr_core::fold::Status::AwaitingOperator
    );
    assert_eq!(fold_refusal(&mut runtime), FoldRefusal::NoPhaseToRetry);
    let (_dir, mut runtime) = parked(body(), false);
    let stopped = super::super::super::operator_command(
        &mut runtime.store,
        &runtime.run_id,
        OperatorCommand::Stop,
        "test",
        "enough",
    )
    .unwrap();
    assert!(matches!(stopped, FencedCommandOutcome::Accepted { .. }));
    assert_eq!(
        runtime.drive().unwrap().state.run.status(),
        brokkr_core::fold::Status::Stopped
    );
    assert_eq!(fold_refusal(&mut runtime), FoldRefusal::AfterTerminal);
}

fn drifted_request(dir: &std::path::Path, runtime: &Engine) -> crate::launch::LaunchRequest {
    crate::launch::LaunchRequest {
        workspace: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        bundle: crate::launch::BundleSource::Dir(crate::launch::tests::bundle_at(dir, false)),
        journal: runtime.store.path().to_path_buf(),
        repo: Some(runtime.repo.clone()),
        secrets: None,
        host_path: std::env::var_os("PATH").unwrap_or_default(),
        shelf: None,
    }
}

#[test]
fn a_drifted_bundle_is_refused_by_resume_after_the_records_and_before_any_panel_fact() {
    let (dir, runtime) = parked(body(), false);
    let before = events(&runtime).len();
    for names in [vec!["b"], vec!["unknown"], vec!["a", "b"]] {
        let names: Vec<String> = names.into_iter().map(str::to_owned).collect();
        let refusal = crate::launch::operator_retry_members(
            drifted_request(dir.path(), &runtime),
            &runtime.run_id,
            &names,
            "test",
            "again",
        )
        .expect_err("refused");
        assert!(
            matches!(refusal, crate::launch::LaunchError::Engine(EngineError::ManifestMismatch { ref run_id, .. }) if *run_id == runtime.run_id),
            "{refusal}"
        );
        assert!(refusal
            .to_string()
            .starts_with(&format!("run '{}' pins a different bundle", runtime.run_id)));
    }
    assert_eq!(events(&runtime).len(), before);
    let (dir, unread) = parked_record(body(), false, |record| {
        record["head"] = json!({"at_start":"not a head"});
    });
    for names in [vec!["b"], vec!["a", "b"]] {
        let names: Vec<String> = names.into_iter().map(str::to_owned).collect();
        let outcome = crate::launch::operator_retry_members(
            drifted_request(dir.path(), &unread),
            &unread.run_id,
            &names,
            "test",
            "again",
        )
        .unwrap();
        assert!(
            matches!(outcome, MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::RecordUnread { record: RecordId::Attempt(ref id), ref field, .. })) if id == "source" && field == "head"),
            "{outcome:?}"
        );
    }
}

fn commanded_args(runtime: &Engine) -> Value {
    events(runtime)
        .iter()
        .rev()
        .find(|e| e.event_type == EventType::OperatorCommanded)
        .unwrap()
        .payload["args"]
        .clone()
}

#[test]
fn the_panel_facts_and_exact_tags_are_asked_at_the_command() {
    let (_dir, mut runtime) = parked(crate::engine::tests::single_body(command(clean())), false);
    assert!(
        matches!(retry(&mut runtime, &["a"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::NoPanel { phase })) if phase == "work")
    );
    let later = SeatBody::Sequence {
        steps: vec![
            single_step("chief", clean()),
            panel_step("positions", vec![("a", clean()), ("b", clean())]),
        ],
    };
    let (_dir, mut runtime) = parked(later, false);
    assert!(
        matches!(retry(&mut runtime, &["positions:b"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::PanelStepFollows { step, first })) if step == "positions" && first == "chief")
    );
    let positions = ["correctness", "security", "spec-compliance", "simplicity"];
    let review = SeatBody::Sequence {
        steps: vec![
            panel_step("positions", positions.map(|name| (name, clean())).to_vec()),
            single_step("chief", clean()),
        ],
    };
    let (_dir, mut runtime) = parked_record(review, false, |record| {
        record["settled_members"] = json!({
            "positions:security": {"result":"clean"},
            "positions:spec-compliance": {"result":"clean"},
            "positions:simplicity": {"result":"clean"},
        });
    });
    assert!(
        matches!(retry(&mut runtime, &["correctness"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::UnknownMember { name, tags })) if name == "correctness" && tags == "positions:correctness, positions:security, positions:spec-compliance, positions:simplicity")
    );
    assert!(matches!(
        retry(&mut runtime, &["positions:correctness"]),
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    assert_eq!(
        commanded_args(&runtime),
        json!({"members":["positions:correctness"]})
    );
    let nested = SeatBody::Sequence {
        steps: vec![panel_step("p", vec![("x", clean()), ("p:x", clean())])],
    };
    let (_dir, mut runtime) = parked_record(nested, false, |record| {
        record["settled_members"] = json!({"p:p:x": {"result":"clean"}});
    });
    assert!(
        matches!(retry(&mut runtime, &["x"]), MemberRetryOutcome::CarryRefused(CarryRefusal(Refusal::UnknownMember { name, tags })) if name == "x" && tags == "p:x, p:p:x")
    );
    assert!(matches!(
        retry(&mut runtime, &["p:x"]),
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    assert_eq!(commanded_args(&runtime), json!({"members":["p:x"]}));
}

#[test]
fn a_peer_retry_before_or_after_the_member_command_loses_the_original_fence() {
    for after in [false, true] {
        let (_dir, mut runtime) = parked(body(), false);
        let read = Engine::read_member_retry(&mut runtime.store, &runtime.run_id)
            .unwrap()
            .unwrap();
        let run = runtime.run_id.clone();
        let mut peer = |store: &mut Store| {
            let outcome = super::super::super::operator_command(
                store,
                &run,
                OperatorCommand::Retry,
                "peer",
                "again",
            )
            .unwrap();
            assert!(matches!(outcome, FencedCommandOutcome::Accepted { .. }));
        };
        let outcome = if after {
            let mut once = false;
            disposed(
                &mut runtime.store,
                &runtime.run_id,
                Commanded {
                    command: OperatorCommand::Retry,
                    operator: "member",
                    reason: "again",
                    args: json!({"members":["b"]}),
                },
                read.deciding,
                |store| {
                    if !once {
                        once = true;
                        peer(store);
                    }
                },
            )
            .unwrap()
            .outcome()
        } else {
            match runtime
                .retry_members_racing(read, &["b".into()], "member", "again", &mut peer)
                .unwrap()
            {
                MemberRetryOutcome::Disposed(outcome) => outcome,
                other => panic!("{other:?}"),
            }
        };
        assert!(
            matches!(outcome, FencedCommandOutcome::Rejected { reason, .. } if reason == "lost_fence")
        );
        let journal = events(&runtime);
        let accepted: Vec<_> = journal
            .iter()
            .filter(|e| e.event_type == EventType::OperatorAccepted)
            .collect();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].payload["operator"], "peer");
    }
}
