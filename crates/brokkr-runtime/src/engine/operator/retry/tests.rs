use super::*;
use crate::engine::carry::fixture_tests::*;
use crate::engine::carry::{RecordId, Refusal};
use crate::engine::tests::git_commit;
use brokkr_core::{fold::Refusal as FoldRefusal, EventType};
use serde_json::json;

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
