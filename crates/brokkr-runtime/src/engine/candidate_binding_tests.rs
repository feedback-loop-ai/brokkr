use super::*;
use crate::engine::tests::{
    commit_file, engine, git_commit, single_body, state, CLASSES, DOCS_CLASS,
};
use brokkr_core::decision::PhaseResult;
use brokkr_core::fold::Cursor;
use brokkr_core::policy::Machine;

/// A verify seat beside the review and ship seats the test bundle hires,
/// over a table that verifies into review, parks or holds at review, and
/// ships. Nothing is committed yet.
fn verifying(table: Value) -> (tempfile::TempDir, Engine) {
    let (dir, mut engine) = engine(single_body(vec!["unused".into()]));
    let mut verify = engine.bundle.seats["work"].clone();
    verify.results = vec!["checks-good".into(), "checks-bad".into()];
    engine.bundle.seats.insert("verify".into(), verify);
    let review = engine.bundle.seats.get_mut("review").unwrap();
    review.results.push("security-hold".into());
    review.results.push("needs-operator".into());
    engine.bundle.machine = Machine::from_table(&table).unwrap();
    (dir, engine)
}

fn table() -> Value {
    json!({
        "schema":"forge.phase-machine/v2",
        "phases":["work","verify","review","ship","done","stop"],
        "initial":"work", "terminal":["done","stop"],
        "rules":[
            {"id":"WORKED","from":"work","result":"complete","next":"verify","reason":"work"},
            {"id":"VERIFIED","from":"verify","result":"checks-good","next":"review","reason":"checks"},
            {"id":"UNVERIFIED","from":"verify","result":"checks-bad","next":"work","severity":"flagged","reason":"again"},
            {"id":"HELD","from":"review","result":"security-hold","next":"stop","severity":"hard","reason":"hold"},
            {"id":"PARKED","from":"review","result":"needs-operator","park":true,"reason":"operator rules"},
            {"id":"REVIEWED","from":"review","result":"clean","next":"ship","reason":"review"},
            {"id":"SHIPPED","from":"ship","result":"shipped","next":"done","reason":"ship"}
        ]
    })
}

fn decided(engine: &mut Engine, state: &RunState, result: Value) -> Value {
    engine.decide(state, result).unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    events.last().unwrap().payload.clone()
}

/// A committed repository whose verification passed: the review state
/// carries verify's journaled witness, which names the real HEAD and
/// tree, never the pair the seat claimed.
fn fixture() -> (tempfile::TempDir, Engine, RunState) {
    let (dir, mut engine) = verifying(table());
    let head = git_commit(&engine.repo, "verified");
    let decision = decided(
        &mut engine,
        &state(Some("verify"), Cursor::RequestEffect),
        json!({"result":"checks-good", "inputs":{"verified_candidate":{"head":"spoof"}}}),
    );
    assert_eq!(decision["next"], "review");
    assert_eq!(
        decision["inputs"]["verified_candidate"],
        json!({"head": head, "tree": git_candidate(&engine.repo).unwrap().1})
    );
    (dir, engine, reviewed_after(&decision))
}

/// The review state the fold would reach after verify's `decision`.
fn reviewed_after(decision: &Value) -> RunState {
    let mut reviewed = state(Some("review"), Cursor::RequestEffect);
    reviewed.phase_results.insert(
        "verify".into(),
        PhaseResult {
            result: "checks-good".into(),
            inputs: Some(serde_json::from_value(decision["inputs"].clone()).unwrap()),
        },
    );
    reviewed
}

fn judge(engine: &mut Engine, state: &RunState) -> Value {
    decided(
        engine,
        state,
        json!({"result":"clean","inputs":{"fixes_applied":false,"spec_defect":false}}),
    )
}

#[test]
fn verified_candidate_unchanged_control_and_clean_committed_drift() {
    let (_dir, mut engine, state) = fixture();
    assert_eq!(judge(&mut engine, &state)["next"], "ship");
    git_commit(&engine.repo, "unverified-clean-commit");
    assert!(!git_dirty(&engine.repo));
    let refused = judge(&mut engine, &state);
    assert_eq!(refused["rule_id"], "REVIEWED");
    assert!(refused["next"].is_null());
    assert_eq!(
        refused["problem"],
        "candidate differs from the journaled verified HEAD/tree or is dirty"
    );
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
fn a_verify_ruling_without_a_readable_witness_and_ship_drift_fail_closed() {
    let (_dir, mut engine, mut state) = fixture();
    let witness = state.phase_results.remove("verify").unwrap();
    for unreadable in [json!({}), json!({"verified_candidate":{"head":"spoof"}})] {
        state.phase_results.insert(
            "verify".into(),
            PhaseResult {
                result: "checks-bad".into(),
                inputs: Some(serde_json::from_value(unreadable).unwrap()),
            },
        );
        let refused = judge(&mut engine, &state);
        assert!(refused["next"].is_null());
        assert_eq!(
            refused["problem"],
            "verified candidate witness is missing or unreadable"
        );
    }
    state.phase_results.insert("verify".into(), witness);
    state.phase = Some("ship".into());
    git_commit(&engine.repo, "after-review");
    let shipped = decided(&mut engine, &state, json!({"result":"shipped"}));
    assert!(shipped["next"].is_null());
    assert_eq!(
        shipped["problem"],
        "candidate differs from the journaled verified HEAD/tree or is dirty"
    );
}

/// Decision 0041 ruling 5e: a returned implement whose delta lies in the
/// repository's docs class re-enters review without verify, so the judge
/// and the ship are held to the verified candidate under that delta,
/// classified by the class at the verified head, and a code path in it
/// parks as drift, never as verified.
#[test]
fn a_docs_class_delta_over_the_verified_candidate_is_judged_and_shipped_and_code_is_not() {
    let (_dir, mut engine) = verifying(table());
    git_commit(&engine.repo, "base");
    commit_file(&engine.repo, CLASSES, DOCS_CLASS, "classes");
    let verified = decided(
        &mut engine,
        &state(Some("verify"), Cursor::RequestEffect),
        json!({"result":"checks-good"}),
    );
    let reviewed = reviewed_after(&verified);
    commit_file(&engine.repo, "docs/answer.md", "# answer\n", "answered");
    assert_eq!(judge(&mut engine, &reviewed)["next"], "ship");
    let mut shipping = reviewed.clone();
    shipping.phase = Some("ship".into());
    let shipped = decided(&mut engine, &shipping, json!({"result":"shipped"}));
    assert_eq!(
        (&shipped["next"], &shipped["problem"]),
        (&json!("done"), &Value::Null)
    );
    commit_file(&engine.repo, "src/answer.rs", "fn answer() {}\n", "coded");
    let refused = judge(&mut engine, &reviewed);
    assert_eq!(
        (&refused["next"], &refused["problem"]),
        (
            &Value::Null,
            &json!("candidate differs from the journaled verified HEAD/tree or is dirty")
        )
    );
}

/// Decision 0039's docs tier enters review from classify: a run the table
/// never sent through verify is the table's to route, drifted or not.
#[test]
fn a_run_the_table_routed_past_verify_is_held_to_nothing() {
    let (_dir, mut engine, _) = fixture();
    git_commit(&engine.repo, "never-verified");
    let unverified = state(Some("review"), Cursor::RequestEffect);
    assert_eq!(judge(&mut engine, &unverified)["next"], "ship");
}

/// The research recipe's protected phase is verify itself: its advancing
/// ruling binds the candidate it passed instead of being held to a
/// witness only it could have written.
#[test]
fn verify_as_the_protected_phase_binds_on_its_own_advance() {
    let (_dir, mut engine) = verifying(json!({
        "phases":["verify","done"], "initial":"verify", "terminal":["done"],
        "rules":[{"id":"VERIFIED","from":"verify","result":"checks-good","next":"done","reason":"ok"}]
    }));
    engine.bundle.protected_phase = "verify".into();
    let head = git_commit(&engine.repo, "researched");
    let decision = decided(
        &mut engine,
        &state(Some("verify"), Cursor::RequestEffect),
        json!({"result":"checks-good"}),
    );
    assert_eq!(decision["next"], "done");
    assert_eq!(decision["inputs"]["verified_candidate"]["head"], head);
}

#[test]
fn a_failing_verification_binds_nothing_and_other_phases_owe_nothing() {
    let (_dir, mut engine) = verifying(table());
    git_commit(&engine.repo, "worked");
    let failed = decided(
        &mut engine,
        &state(Some("verify"), Cursor::RequestEffect),
        json!({"result":"checks-bad"}),
    );
    assert_eq!(
        (&failed["next"], &failed["severity"]),
        (&json!("work"), &json!("flagged"))
    );
    assert!(failed["inputs"].get("verified_candidate").is_none());
    std::fs::write(engine.repo.join("worked.txt"), "dirty").unwrap();
    let worked = decided(
        &mut engine,
        &state(Some("work"), Cursor::RequestEffect),
        json!({"result":"complete"}),
    );
    assert_eq!(worked["next"], "verify");
}

/// A ruling that admits nothing is held to nothing: a rule-driven park
/// keeps the rule's own reason and a hard hold keeps its severity, with
/// the tree drifted under both.
#[test]
fn a_park_and_a_hard_ruling_at_the_judge_keep_their_own_words() {
    let (_dir, mut engine, state) = fixture();
    git_commit(&engine.repo, "drifted");
    let parked = decided(&mut engine, &state, json!({"result":"needs-operator"}));
    assert_eq!(
        (&parked["rule_id"], &parked["next"], &parked["problem"]),
        (&json!("PARKED"), &Value::Null, &json!("operator rules"))
    );
    let held = decided(&mut engine, &state, json!({"result":"security-hold"}));
    assert_eq!(
        (&held["next"], &held["severity"], &held["problem"]),
        (&json!("stop"), &json!("hard"), &Value::Null)
    );
}

#[test]
fn verification_refuses_a_dirty_candidate_and_no_verify_seat_stays_compatible() {
    let (_dir, mut engine, _) = fixture();
    let verify = state(Some("verify"), Cursor::RequestEffect);
    std::fs::write(engine.repo.join("verified.txt"), "dirty").unwrap();
    let dirty = decided(&mut engine, &verify, json!({"result":"checks-good"}));
    assert_eq!(
        (&dirty["next"], &dirty["problem"]),
        (&Value::Null, &json!("verification candidate is dirty"))
    );
    engine.bundle.seats.remove("verify");
    assert_eq!(
        judge(&mut engine, &state(Some("review"), Cursor::RequestEffect))["next"],
        "ship"
    );
}

/// A worktree that names no commit, a directory with no repository or an
/// empty one, is held to nothing: verify binds no witness there, and the
/// judge rules without one, as every machine proof runs.
#[test]
fn a_worktree_with_no_commit_binds_nothing_and_is_held_to_nothing() {
    let verify = state(Some("verify"), Cursor::RequestEffect);
    let (_other, mut unrepo, _) = fixture();
    std::fs::remove_dir_all(unrepo.repo.join(".git")).unwrap();
    let (_empty, mut uncommitted) = verifying(table());
    crate::realms::tests::initialised(&uncommitted.repo);
    for engine in [&mut unrepo, &mut uncommitted] {
        let verified = decided(engine, &verify, json!({"result":"checks-good"}));
        assert_eq!(verified["next"], "review");
        assert!(verified["inputs"].get("verified_candidate").is_none());
        assert_eq!(judge(engine, &reviewed_after(&verified))["next"], "ship");
    }
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
