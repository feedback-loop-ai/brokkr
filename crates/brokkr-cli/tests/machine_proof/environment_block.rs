//! Decision 0092's acceptance, through the real engine and the scripted
//! fake driver: a seat its environment blocked parks for the operator
//! with its notes, and the operator's retry re-runs that one seat with
//! the earlier phases' results intact; a seat the work blocked, and one
//! that names no blocker, stop as the table says.

use serde_json::{json, Value};

use super::{happy_script, implement_decision, Workspace, REFORGING_POLICY};

const NOTES: &str = "the signing agent is unreachable from the seat's box";

/// The reforging table with `recipes/fast`'s environment park above its
/// unconditional blocked stop.
fn parking_policy() -> String {
    let mut policy: Value = serde_json::from_str(REFORGING_POLICY).unwrap();
    let rules = policy["rules"].as_array_mut().unwrap();
    let stop = rules
        .iter()
        .position(|r| r["id"] == "IMPL-BLOCKED")
        .unwrap();
    let park = json!({"id": "IMPL-BLOCKED-ENVIRONMENT", "from": "implement",
        "result": "blocked", "when": {"blocked_by_in": ["environment"]}, "park": true,
        "reason": "The environment blocked the implementer."});
    rules.insert(stop, park);
    policy.to_string()
}

/// A workspace whose implementer first reports `blocked` with `inputs`
/// and the notes, then completes, echoing what it was handed.
fn blocked_once(inputs: Value) -> Workspace {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "succeed",
         "result": {"result": "blocked", "inputs": inputs, "notes": NOTES}},
        {"behavior": "echo", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::with_policy(script, &parking_policy());
    ws.set_seat_limits("implement", 1, 3600);
    ws
}

#[test]
fn an_environment_block_parks_and_its_retry_re_runs_only_that_seat() {
    let ws = blocked_once(json!({"blocked_by": "environment"}));
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert_eq!(summary["status"], "awaiting_operator");
    assert_eq!(
        summary["park_reason"],
        format!("IMPL-BLOCKED-ENVIRONMENT for (implement, blocked): {NOTES}")
    );
    let run_id = Workspace::run_id(&stderr);

    let db = ws.db();
    let retry = [
        "operator",
        "retry",
        "--run",
        &run_id,
        "--reason",
        "agent forwarded",
    ];
    let (code, _, _) = ws.brokkr(&[&retry[..], &["--db", db.to_str().unwrap()]].concat());
    assert_eq!(code, Some(0));
    let bundle = ws.bundle_dir();
    let (code, summary, stderr) = ws.brokkr(&[
        "resume",
        "--run",
        &run_id,
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    // `max_attempts` 1: the park spent none of the seat's budget.
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");

    let events = ws.exported_events(&run_id);
    let entered = |phase: &str| {
        let entry = |e: &&Value| e["type"] == "phase/entered" && e["payload"]["phase"] == phase;
        events.iter().filter(entry).count()
    };
    assert_eq!((entered("intake"), entered("implement")), (1, 1));
    let rerun = events
        .iter()
        .filter(|e| e["type"] == "effect/succeeded")
        .find_map(|e| e["payload"]["result"].get("seat_input"))
        .expect("the re-run implementer echoes its input");
    assert_eq!(rerun["phase"], "implement");
    assert_eq!(rerun["context"]["results"]["intake"]["result"], "resolved");
}

#[test]
fn a_work_block_and_a_block_naming_no_blocker_stop_as_before() {
    for inputs in [json!({"blocked_by": "work"}), json!({})] {
        let ws = blocked_once(inputs.clone());
        let (code, summary, stderr) = ws.run();
        assert_eq!(code, Some(3), "{inputs}: {stderr}");
        assert_eq!(summary["status"], "stopped", "{inputs}");
        let decision = implement_decision(&ws.exported_events(&Workspace::run_id(&stderr)));
        let ruled = &decision["payload"];
        assert_eq!(
            (&ruled["rule_id"], &ruled["next"], &ruled["inputs"]),
            (&json!("IMPL-BLOCKED"), &json!("stop"), &inputs),
            "{inputs}"
        );
    }
}
