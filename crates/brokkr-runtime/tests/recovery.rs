//! Crash recovery: a journal cut off at a durable boundary resumes
//! without losing a committed fact and without converting an uncertain
//! effect into a success (first-release acceptance criteria).

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use brokkr_runtime::{Bundle, Engine};
use brokkr_store::Store;
use serde_json::json;

const POLICY: &str = r#"{
  "phases": ["intake", "review", "done"],
  "initial": "intake",
  "terminal": ["done"],
  "rules": [
    {"id": "INTAKE-OK", "from": "intake", "result": "resolved", "next": "review",
     "reason": "framed"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "clean"}
  ]
}"#;

fn bundle_dir(dir: &std::path::Path) -> std::path::PathBuf {
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    // Never spawned: each test parks first. The exec driver serves no
    // model, so SI2 past the lifted fence does not judge it (U9b1).
    let seat = json!({
        "role": "roles/role.md",
        "results": ["resolved"],
        "driver": {"command": ["{brokkr}", "driver", "exec", "--", "false"]},
    });
    let mut review_seat = seat.clone();
    review_seat["results"] = json!(["clean"]);
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_string(&json!({
            "name": "recovery",
            "policy": "policy.json",
            "seats": {"intake": seat, "review": review_seat},
        }))
        .unwrap(),
    )
    .unwrap();
    bundle
}

/// The crash test's engine in a directory of its own, where this process
/// is `test` run again under a HOME of its own ([`crate::own_home`]); none
/// in the process that ran it so.
fn started(test: &str) -> Option<(tempfile::TempDir, Engine)> {
    if !crate::own_home::in_own_home(test) {
        return None;
    }
    let dir = tempfile::tempdir().unwrap();
    let bundle = Bundle::compile(&bundle_dir(dir.path())).unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let engine = Engine::start(store, bundle, "crash test", Some(dir.path().into())).unwrap();
    Some((dir, engine))
}

#[test]
fn attempt_in_flight_at_restart_parks_indeterminate() {
    let test = "recovery::attempt_in_flight_at_restart_parks_indeterminate";
    let Some((_dir, mut engine)) = started(test) else {
        return;
    };
    let run_id = engine.run_id.clone();

    // Simulate the journal of a process that died mid-attempt.
    engine
        .store
        .append_next(
            &run_id,
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    engine
        .store
        .append_next(
            &run_id,
            EventType::EffectRequested,
            json!({"effect_id": "fx", "phase": "intake", "seat": "intake",
                   "idempotency_key": "k", "input_digest": "d"}),
            None,
            None,
        )
        .unwrap();
    engine
        .store
        .append_next(
            &run_id,
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a1", "driver": "false"}),
            None,
            Some("a1".into()),
        )
        .unwrap();

    let reason = drive_to_park(&mut engine);
    assert!(
        reason.contains("completion cannot be established"),
        "park reason: {reason}"
    );
}

#[test]
fn requested_effect_with_stale_input_refuses_to_execute() {
    let test = "recovery::requested_effect_with_stale_input_refuses_to_execute";
    let Some((_dir, mut engine)) = started(test) else {
        return;
    };
    let run_id = engine.run_id.clone();

    engine
        .store
        .append_next(
            &run_id,
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    // Requested under a digest the rebuilt input cannot match.
    engine
        .store
        .append_next(
            &run_id,
            EventType::EffectRequested,
            json!({"effect_id": "fx", "phase": "intake", "seat": "intake",
                   "idempotency_key": "k", "input_digest": "stale-digest"}),
            None,
            None,
        )
        .unwrap();

    let reason = drive_to_park(&mut engine);
    assert!(
        reason.contains("refusing to execute"),
        "never run something other than what was requested; got: {reason}"
    );
}

/// Drive `engine` to its end, which must be a park, and name its reason;
/// only in a process whose HOME is its own.
fn drive_to_park(engine: &mut Engine) -> String {
    crate::own_home::required();
    let end = engine.drive().unwrap();
    assert_eq!(end.state.run.status(), Status::AwaitingOperator);
    end.state.run.park_reason().unwrap().as_str().to_string()
}
