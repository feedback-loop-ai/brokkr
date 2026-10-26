use super::*;
use crate::engine::tests::{engine, git_commit, single_body, state};
use brokkr_core::decision::PhaseResult;
use brokkr_core::fold::Cursor;
use brokkr_core::policy::Machine;

fn fixture() -> (tempfile::TempDir, Engine, RunState) {
    let (dir, mut engine) = engine(single_body(vec!["unused".into()]));
    git_commit(&engine.repo, "verified");
    let mut verify = engine.bundle.seats["work"].clone();
    verify.results = vec!["checks-good".into()];
    engine.bundle.seats.insert("verify".into(), verify);
    engine.bundle.machine = Machine::from_table(&json!({
        "phases":["verify","review","ship","done","stop"],
        "initial":"verify", "terminal":["done","stop"],
        "rules":[
            {"id":"VERIFIED","from":"verify","result":"checks-good","next":"review","reason":"checks"},
            {"id":"HELD","from":"review","result":"security-hold","next":"stop","severity":"hard","reason":"hold"},
            {"id":"REVIEWED","from":"review","result":"clean","next":"ship","reason":"review"},
            {"id":"SHIPPED","from":"ship","result":"shipped","next":"done","reason":"ship"}
        ]
    })).unwrap();
    engine
        .decide(
            &state(Some("verify"), Cursor::RequestEffect),
            json!({"result":"checks-good", "inputs":{"verified_candidate":{"head":"spoof"}}}),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let decision = &events.last().unwrap().payload;
    assert_eq!(decision["next"], "review");
    assert_ne!(decision["inputs"]["verified_candidate"]["head"], "spoof");
    let mut reviewed = state(Some("review"), Cursor::RequestEffect);
    reviewed.phase_results.insert(
        "verify".into(),
        PhaseResult {
            result: "checks-good".into(),
            inputs: Some(serde_json::from_value(decision["inputs"].clone()).unwrap()),
        },
    );
    (dir, engine, reviewed)
}

fn judge(engine: &mut Engine, state: &RunState) -> Value {
    engine
        .decide(
            state,
            json!({"result":"clean","inputs":{"fixes_applied":false,"spec_defect":false}}),
        )
        .unwrap();
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .last()
        .unwrap()
        .payload
        .clone()
}

#[test]
fn verified_candidate_unchanged_control_and_clean_committed_drift() {
    let (_dir, mut engine, state) = fixture();
    assert_eq!(judge(&mut engine, &state)["next"], "ship");
    git_commit(&engine.repo, "unverified-clean-commit");
    assert!(!git_dirty(&engine.repo));
    let refused = judge(&mut engine, &state);
    assert!(refused["next"].is_null());
    assert!(refused["problem"]
        .as_str()
        .unwrap()
        .contains("verified HEAD/tree"));
}

#[test]
fn verified_candidate_dirty_tree_and_index_are_refused() {
    for staged in [false, true] {
        let (_dir, mut engine, state) = fixture();
        std::fs::write(engine.repo.join("verified.txt"), "unverified").unwrap();
        if staged {
            assert!(std::process::Command::new("git")
                .args(["add", "verified.txt"])
                .current_dir(&engine.repo)
                .status()
                .unwrap()
                .success());
        }
        assert!(judge(&mut engine, &state)["next"].is_null());
    }
}

#[test]
fn verified_candidate_missing_witness_and_ship_drift_fail_closed() {
    let (_dir, mut engine, mut state) = fixture();
    let witness = state.phase_results.remove("verify").unwrap();
    assert!(judge(&mut engine, &state)["next"].is_null());
    state.phase_results.insert("verify".into(), witness);
    state.phase = Some("ship".into());
    git_commit(&engine.repo, "after-review");
    engine.decide(&state, json!({"result":"shipped"})).unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(events.last().unwrap().payload["next"].is_null());
}

#[test]
fn verification_refuses_dirty_or_unreadable_candidate_and_no_verify_stays_compatible() {
    let (_dir, mut engine, _) = fixture();
    std::fs::write(engine.repo.join("verified.txt"), "dirty").unwrap();
    engine
        .decide(
            &state(Some("verify"), Cursor::RequestEffect),
            json!({"result":"checks-good"}),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(events.last().unwrap().payload["next"].is_null());
    let (_other, mut unreadable, _) = fixture();
    std::fs::remove_dir_all(unreadable.repo.join(".git")).unwrap();
    unreadable
        .decide(
            &state(Some("verify"), Cursor::RequestEffect),
            json!({"result":"checks-good"}),
        )
        .unwrap();
    let refused = unreadable.store.load(&unreadable.run_id).unwrap();
    assert!(refused.last().unwrap().payload["next"].is_null());
    engine.bundle.seats.remove("verify");
    assert_eq!(
        judge(&mut engine, &state(Some("review"), Cursor::RequestEffect))["next"],
        "ship"
    );
}

#[test]
fn verified_candidate_survives_engine_restart_and_rejects_untracked_source() {
    use brokkr_core::fold::fold;
    use brokkr_store::Store;
    let (dir, template, _) = fixture();
    let path = dir.path().join("restart.db");
    let bundle = template.bundle.clone();
    let repo = template.repo.clone();
    let mut live = Engine::start(
        Store::open(&path).unwrap(),
        bundle.clone(),
        "candidate restart",
        Some(repo.clone()),
    )
    .unwrap();
    for (kind, payload) in [
        (EventType::PhaseEntered, json!({"phase":"verify"})),
        (
            EventType::EffectRequested,
            json!({"effect_id":"e", "seat":"verify"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id":"e", "attempt_id":"a"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id":"e", "attempt_id":"a", "result":{"result":"checks-good"}}),
        ),
    ] {
        live.append(kind, payload, None).unwrap();
    }
    let verified = fold(&live.store.load(&live.run_id).unwrap()).unwrap();
    live.decide(&verified, json!({"result":"checks-good"}))
        .unwrap();
    live.append(EventType::PhaseEntered, json!({"phase":"review"}), None)
        .unwrap();
    let id = live.run_id.clone();
    drop(live);
    let mut resumed = Engine::resume(Store::open(&path).unwrap(), bundle, &id, Some(repo)).unwrap();
    let replayed = fold(&resumed.store.load(&id).unwrap()).unwrap();
    assert!(replayed.phase_results["verify"]
        .inputs
        .as_ref()
        .unwrap()
        .contains_key("verified_candidate"));
    assert_eq!(judge(&mut resumed, &replayed)["next"], "ship");
    assert!(std::process::Command::new("git")
        .args(["config", "status.showUntrackedFiles", "no"])
        .current_dir(&resumed.repo)
        .status()
        .unwrap()
        .success());
    std::fs::write(
        resumed.repo.join("untracked-source.rs"),
        "fn unverified() {}\n",
    )
    .unwrap();
    assert!(judge(&mut resumed, &replayed)["next"].is_null());
}
