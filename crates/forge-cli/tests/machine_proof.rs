//! Machine proof (delivery-sequence step 5): the whole engine driven by
//! the scripted fake driver over the real protocol, covering success,
//! retry, hard stop, schema park, indeterminate park, operator retry,
//! protocol garbage, crash recovery, bundle pinning, and the
//! constitutional compile rejections.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn forge_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forge")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["intake", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "intake",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "INTAKE-OK", "from": "intake", "result": "resolved", "next": "implement",
     "reason": "Task framed and recorded."},
    {"id": "IMPL-BROKEN-TWICE", "from": "implement", "result": "broken",
     "when": {"consecutive_failures_gte": 2}, "next": "stop", "severity": "hard",
     "reason": "Two consecutive broken implement runs; stop rather than thrash."},
    {"id": "IMPL-BROKEN-RETRY", "from": "implement", "result": "broken",
     "next": "implement", "reason": "First broken run; one re-run permitted."},
    {"id": "IMPL-BLOCKED", "from": "implement", "result": "blocked", "next": "stop",
     "severity": "hard", "reason": "Implementer blocked; report, never silently continue."},
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "verify",
     "reason": "Implementation complete and committed."},
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop",
     "severity": "hard", "reason": "Verification failed; not shippable."},
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review",
     "reason": "Suite green; reviewers read verified code."},
    {"id": "REVIEW-SECURITY-HOLD", "from": "review", "result": "security-hold",
     "next": "stop", "severity": "hard",
     "reason": "Unresolved security findings. NEVER ship."},
    {"id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"max_residual_severity_above": "medium"}, "next": "stop",
     "severity": "hard", "reason": "Residual severity above medium; not shippable."},
    {"id": "REVIEW-RESIDUAL-SECURITY", "from": "review", "result": "residual",
     "when": {"has_security_residual": true}, "next": "stop", "severity": "hard",
     "reason": "Security residuals never take the tracked-debt path."},
    {"id": "REVIEW-RESIDUAL-OK", "from": "review", "result": "residual", "next": "ship",
     "severity": "flagged",
     "reason": "Non-security residuals at or below medium proceed as tracked debt."},
    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean",
     "when": {"fixes_applied": false}, "next": "ship",
     "reason": "Clean with no code changed; verification evidence stands."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "verify",
     "reason": "Clean but fixes applied; re-verify before shipping."},
    {"id": "SHIP-DRIFT", "from": "ship", "result": "ready",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved after review; re-arm a scoped review."},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at ship time is a defect."},
    {"id": "SHIP-OK", "from": "ship", "result": "ready", "next": "done",
     "reason": "Clean, reviewed, verified; ship."}
  ]
}"#;

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new(script: Value) -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.bundle_dir();
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        let script_path = ws.path().join("script.json");
        std::fs::write(&script_path, serde_json::to_string(&script).unwrap()).unwrap();

        let seat = |results: Vec<&str>| -> Value {
            json!({
                "role": "roles/role.md",
                "results": results,
                "driver": {"command": [
                    forge_bin(), "fake-driver",
                    "--script", script_path.to_string_lossy(),
                    "--state", ws.path().join("state").to_string_lossy(),
                ]}
            })
        };
        std::fs::write(bundle.join("roles/role.md"), "# test role\n").unwrap();
        let config = json!({
            "name": "proof",
            "policy": "policy.json",
            "seats": {
                "intake": seat(vec!["resolved"]),
                "implement": seat(vec!["complete", "broken", "blocked"]),
                "verify": seat(vec!["pass", "fail"]),
                "review": seat(vec!["clean", "residual", "security-hold"]),
                "ship": seat(vec!["ready"]),
            }
        });
        std::fs::write(
            bundle.join("bundle.json"),
            serde_json::to_string_pretty(&config).unwrap(),
        )
        .unwrap();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }
    fn bundle_dir(&self) -> PathBuf {
        self.path().join("bundle")
    }
    fn db(&self) -> PathBuf {
        self.path().join("forge.db")
    }

    fn forge(&self, args: &[&str]) -> (Option<i32>, Value, String) {
        let out = Command::new(forge_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        let value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
        (out.status.code(), value, stderr)
    }

    fn run(&self) -> (Option<i32>, Value, String) {
        let bundle = self.bundle_dir();
        let db = self.db();
        self.forge(&[
            "run",
            "--bundle",
            bundle.to_str().unwrap(),
            "--feature",
            "proof feature",
            "--db",
            db.to_str().unwrap(),
        ])
    }

    fn run_id(stderr: &str) -> String {
        stderr
            .lines()
            .find_map(|l| l.strip_prefix("run started: "))
            .expect("run id on stderr")
            .trim()
            .to_string()
    }
}

fn happy_script() -> Value {
    json!({"seats": {
        "intake": [{"behavior": "succeed", "result": {"result": "resolved"}}],
        "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review": [{"behavior": "succeed",
                    "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "ship": [{"behavior": "succeed", "result": {"result": "ready"}}],
    }})
}

#[test]
fn full_delivery_completes_exports_and_replays() {
    let ws = Workspace::new(happy_script());
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["phase"], "done");
    let run_id = Workspace::run_id(&stderr);

    let db = ws.db();
    let (code, replay, _) =
        ws.forge(&["replay", "--run", &run_id, "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(replay["replay"], "deterministic");

    let out = ws.path().join("export");
    let (code, _, _) = ws.forge(&[
        "export", "--run", &run_id,
        "--out", out.to_str().unwrap(),
        "--db", db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let journal = out.join(format!("{run_id}.ndjson"));
    let (code, verified, _) = ws.forge(&["verify-run", journal.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(verified["chain"], "verified");
    assert_eq!(verified["state"]["status"], "completed");
}

#[test]
fn residual_ships_as_tracked_debt_but_flagged() {
    let mut script = happy_script();
    script["seats"]["review"] = json!([{"behavior": "succeed", "result": {
        "result": "residual",
        "inputs": {"max_residual_severity": "low", "has_security_residual": false}
    }}]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(0));
    assert_eq!(summary["status"], "completed");
}

#[test]
fn security_hold_hard_stops() {
    let mut script = happy_script();
    script["seats"]["review"] =
        json!([{"behavior": "succeed", "result": {"result": "security-hold"}}]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(3));
    assert_eq!(summary["status"], "stopped");
    assert_eq!(summary["last_decision"]["rule_id"], "REVIEW-SECURITY-HOLD");
}

#[test]
fn broken_retries_once_then_hard_stops_on_second() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "succeed", "result": {"result": "broken"}},
        {"behavior": "succeed", "result": {"result": "broken"}},
    ]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(3));
    assert_eq!(summary["last_decision"]["rule_id"], "IMPL-BROKEN-TWICE");
    assert_eq!(
        summary["last_decision"]["inputs"]["consecutive_failures"], 2,
        "journal-computed counter, never accepted from the seat"
    );
}

#[test]
fn broken_once_then_complete_recovers() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "succeed", "result": {"result": "broken"}},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(0));
    assert_eq!(summary["status"], "completed");
}

#[test]
fn undeclared_result_parks_with_schema_evidence() {
    let mut script = happy_script();
    script["seats"]["implement"] =
        json!([{"behavior": "succeed", "result": {"result": "nonsense"}}]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2), "schema violations park, never guess");
    assert_eq!(summary["status"], "awaiting_operator");
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("schema"), "park reason: {reason}");
}

#[test]
fn seat_supplied_engine_inputs_are_dropped() {
    // A seat claiming consecutive_failures: 99 must not reach the table.
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "succeed",
         "result": {"result": "broken", "inputs": {"consecutive_failures": 99}}},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(0), "first broken must RETRY (counter=1), not hard-stop");
    assert_eq!(summary["status"], "completed");
}

#[test]
fn vanished_driver_parks_indeterminate_and_operator_retry_completes() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "vanish"},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("indeterminate"), "park reason: {reason}");
    let run_id = Workspace::run_id(&stderr);

    let db = ws.db();
    let (code, _, _) = ws.forge(&[
        "operator", "retry", "--run", &run_id,
        "--reason", "scripted retry",
        "--db", db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let bundle = ws.bundle_dir();
    let (code, summary, _) = ws.forge(&[
        "resume", "--run", &run_id,
        "--bundle", bundle.to_str().unwrap(),
        "--db", db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    assert_eq!(summary["status"], "completed");
}

#[test]
fn protocol_garbage_fails_closed_and_parks() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([{"behavior": "garbage"}]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("unreadable driver message"), "park reason: {reason}");
}

#[test]
fn resume_refuses_an_edited_bundle() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([{"behavior": "vanish"}]);
    let ws = Workspace::new(script);
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(2));
    let run_id = Workspace::run_id(&stderr);

    std::fs::write(ws.bundle_dir().join("roles/role.md"), "# edited role\n").unwrap();
    let db = ws.db();
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.forge(&[
        "resume", "--run", &run_id,
        "--bundle", bundle.to_str().unwrap(),
        "--db", db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1), "an active run never silently changes its bundle");
    assert!(stderr.contains("different bundle"), "stderr: {stderr}");
    assert!(stderr.contains("roles/role.md"), "stderr: {stderr}");
}

#[test]
fn compile_rejects_uncovered_results_and_review_bypass() {
    // Seat result with no covering rule.
    let ws = Workspace::new(happy_script());
    let bundle = ws.bundle_dir();
    let config_path = bundle.join("bundle.json");
    let mut config: Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    config["seats"]["implement"]["results"] = json!(["complete", "broken", "weird"]);
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();
    let (code, _, stderr) = ws.forge(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("no rule covers"), "stderr: {stderr}");

    // A path to ship that bypasses review.
    let ws = Workspace::new(happy_script());
    let bundle = ws.bundle_dir();
    let policy_path = bundle.join("policy.json");
    let mut policy: Value =
        serde_json::from_str(&std::fs::read_to_string(&policy_path).unwrap()).unwrap();
    policy["rules"].as_array_mut().unwrap().push(json!({
        "id": "SNEAK", "from": "implement", "result": "sneak", "next": "ship",
        "reason": "bypass review"
    }));
    std::fs::write(&policy_path, serde_json::to_string(&policy).unwrap()).unwrap();
    let (code, _, stderr) = ws.forge(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("constitutionally rejected"), "stderr: {stderr}");
}
