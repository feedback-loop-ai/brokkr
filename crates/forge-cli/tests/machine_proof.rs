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
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "ship",
     "reason": "Gates passed and ledger written; confirm close-out and report shipped."},
    {"id": "SHIPPED-DRIFT", "from": "ship", "result": "shipped",
     "when": {"drift_detected": true}, "next": "review", "severity": "flagged",
     "reason": "HEAD moved between ready and close-out; re-arm a scoped review."},
    {"id": "SHIPPED-DIRTY", "from": "ship", "result": "shipped",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at close-out is a defect."},
    {"id": "SHIP-COMPLETE", "from": "ship", "result": "shipped", "next": "done",
     "reason": "Close-out confirmed: clean, reviewed, verified; done."}
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
                "ship": seat(vec!["ready", "shipped"]),
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
        let (code, stdout, stderr) = self.forge_raw(args);
        let value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
        (code, value, stderr)
    }

    /// Like `forge`, but returns stdout verbatim for commands whose
    /// output is not JSON (e.g. the tab-separated `runs` listing).
    fn forge_raw(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let out = Command::new(forge_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        (out.status.code(), stdout, stderr)
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

    fn set_seat_limits(&self, phase: &str, max_attempts: u64, timeout_seconds: u64) {
        let path = self.bundle_dir().join("bundle.json");
        let mut config: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        config["seats"][phase]["limits"] =
            json!({"max_attempts": max_attempts, "timeout_seconds": timeout_seconds});
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
    }

    /// Convert one phase's seat into a panel of fake-driver members
    /// joined by the named aggregate. Script entries key by
    /// "<phase>:<member>".
    fn make_panel(&self, phase: &str, members: &[&str], aggregate: &str) {
        let path = self.bundle_dir().join("bundle.json");
        let mut config: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let driver = config["seats"][phase]["driver"].clone();
        let mut panel = serde_json::Map::new();
        for member in members {
            panel.insert(
                member.to_string(),
                json!({"role": "roles/role.md", "driver": driver}),
            );
        }
        let seat = config["seats"][phase].as_object_mut().unwrap();
        seat.insert("panel".into(), Value::Object(panel));
        seat.insert("aggregate".into(), json!(aggregate));
        seat.remove("role");
        seat.remove("driver");
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
    }
}

fn happy_script() -> Value {
    json!({"seats": {
        "intake": [{"behavior": "succeed", "result": {"result": "resolved"}}],
        "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review": [{"behavior": "succeed",
                    "result": {"result": "clean", "inputs": {"fixes_applied": false}}}],
        "ship": [
            {"behavior": "succeed", "result": {"result": "ready"}},
            {"behavior": "succeed", "result": {"result": "shipped"}},
        ],
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

    // Causal threading: every engine-appended event names its cause.
    let exported = std::fs::read_to_string(&journal).unwrap();
    for line in exported.lines() {
        let event: Value = serde_json::from_str(line).unwrap();
        if event["seq"].as_u64().unwrap() > 1 {
            assert!(
                event["causation_id"].is_string(),
                "event {} has no causation_id",
                event["seq"]
            );
        }
    }
}

#[test]
fn ready_alone_never_reaches_done() {
    // A ship seat that only ever reports `ready` loops back into ship
    // via SHIP-READY and never completes: `shipped` is the sole entry
    // into done. The second scripted attempt vanishes the driver, so
    // the run parks indeterminate at ship — anywhere but done.
    let mut script = happy_script();
    script["seats"]["ship"] = json!([
        {"behavior": "succeed", "result": {"result": "ready"}},
        {"behavior": "vanish"},
    ]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2), "ready alone must not complete the run");
    assert_eq!(summary["status"], "awaiting_operator");
    assert_eq!(summary["phase"], "ship");
    assert_eq!(summary["last_decision"]["rule_id"], "SHIP-READY");
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
fn runs_lists_completed_run_with_status_and_phase() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);

    let db = ws.db();
    let (code, stdout, stderr) = ws.forge_raw(&["runs", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let line = stdout
        .lines()
        .find(|l| l.starts_with(&run_id))
        .expect("runs output has a line for the run");
    let cols: Vec<&str> = line.split('\t').collect();
    assert_eq!(cols.len(), 5, "run_id, feature, created_at, status, phase: {line}");
    assert_eq!(cols[0], run_id);
    assert_eq!(cols[1], "proof feature");
    assert_eq!(cols[3], "completed");
    assert_eq!(cols[4], "done");
}

#[test]
fn runs_lists_parked_run_as_awaiting_operator_at_its_phase() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([{"behavior": "vanish"}]);
    let ws = Workspace::new(script);
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);

    let db = ws.db();
    let (code, stdout, stderr) = ws.forge_raw(&["runs", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let line = stdout
        .lines()
        .find(|l| l.starts_with(&run_id))
        .expect("runs output has a line for the run");
    let cols: Vec<&str> = line.split('\t').collect();
    assert_eq!(cols.len(), 5, "run_id, feature, created_at, status, phase: {line}");
    assert_eq!(cols[3], "awaiting_operator");
    assert_eq!(cols[4], "implement", "the phase the operator must act on");
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
fn transient_driver_failure_retries_within_limit_and_completes() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "fail", "error": "transient boom"},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    ws.set_seat_limits("implement", 2, 3600);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(0), "one automated retry within the declared limit");
    assert_eq!(summary["status"], "completed");
}

#[test]
fn exhausted_attempt_limit_parks_with_last_error() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "fail", "error": "boom one"},
        {"behavior": "fail", "error": "boom two"},
    ]);
    let ws = Workspace::new(script);
    ws.set_seat_limits("implement", 2, 3600);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("failed 2 of 2 attempt(s)"), "park reason: {reason}");
    assert!(reason.contains("boom two"), "park reason: {reason}");
}

#[test]
fn hung_driver_is_killed_at_its_deadline() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([{"behavior": "hang"}]);
    let ws = Workspace::new(script);
    ws.set_seat_limits("implement", 1, 1);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2), "a hung seat must never hang the run");
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("deadline"), "park reason: {reason}");
}

#[test]
fn indeterminate_is_never_auto_retried_even_with_attempts_left() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "vanish"},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    ws.set_seat_limits("implement", 3, 3600);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2), "indeterminate parks into operator judgment, never auto-retries");
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("indeterminate"), "park reason: {reason}");
}

#[test]
fn panel_unanimous_pass_completes_with_member_evidence() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] =
        json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    script["seats"]["verify:integration"] =
        json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    let ws = Workspace::new(script);
    ws.make_panel("verify", &["unit", "integration"], "unanimous-pass");
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");

    // Member outcomes are journaled as checkpoint evidence, in declared
    // order, under the ONE effect the outer machine saw.
    let run_id = Workspace::run_id(&stderr);
    let db = ws.db();
    let out = ws.path().join("export");
    ws.forge(&[
        "export", "--run", &run_id,
        "--out", out.to_str().unwrap(),
        "--db", db.to_str().unwrap(),
    ]);
    let journal =
        std::fs::read_to_string(out.join(format!("{run_id}.ndjson"))).unwrap();
    let members: Vec<String> = journal
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .filter(|e| e["payload"]["checkpoint"]["step"] == "panel-member-finished")
        .map(|e| e["payload"]["checkpoint"]["member"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(members, vec!["integration", "unit"], "declared (sorted) order");
}

#[test]
fn panel_one_failing_member_fails_the_phase() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] =
        json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    script["seats"]["verify:integration"] =
        json!([{"behavior": "succeed", "result": {"result": "fail"}}]);
    let ws = Workspace::new(script);
    ws.make_panel("verify", &["unit", "integration"], "unanimous-pass");
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(3), "unanimous-pass demands unanimity");
    assert_eq!(summary["last_decision"]["rule_id"], "VERIFY-FAIL");
}

#[test]
fn review_panel_worst_member_wins_and_merges_inputs() {
    let mut script = happy_script();
    script["seats"]["review:correctness"] = json!([{"behavior": "succeed",
        "result": {"result": "clean", "inputs": {"fixes_applied": false}}}]);
    script["seats"]["review:security"] = json!([{"behavior": "succeed",
        "result": {"result": "residual",
                   "inputs": {"max_residual_severity": "low",
                              "has_security_residual": false,
                              "fixes_applied": false}}}]);
    let ws = Workspace::new(script);
    ws.make_panel("review", &["correctness", "security"], "review-panel");
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(0), "residual low proceeds as tracked debt");
    assert_eq!(summary["status"], "completed");
}

#[test]
fn review_panel_security_hold_member_hard_stops() {
    let mut script = happy_script();
    script["seats"]["review:correctness"] = json!([{"behavior": "succeed",
        "result": {"result": "clean", "inputs": {"fixes_applied": false}}}]);
    script["seats"]["review:security"] =
        json!([{"behavior": "succeed", "result": {"result": "security-hold"}}]);
    let ws = Workspace::new(script);
    ws.make_panel("review", &["correctness", "security"], "review-panel");
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(3), "one security-hold member holds the whole panel");
    assert_eq!(summary["last_decision"]["rule_id"], "REVIEW-SECURITY-HOLD");
}

#[test]
fn panel_member_vanish_parks_the_whole_attempt_indeterminate() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] =
        json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    script["seats"]["verify:integration"] = json!([{"behavior": "vanish"}]);
    let ws = Workspace::new(script);
    ws.make_panel("verify", &["unit", "integration"], "unanimous-pass");
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("integration"), "park reason: {reason}");
}

#[test]
fn confined_seat_completes_inside_a_container() {
    // Gated: needs a working docker. The intake seat's fake driver runs
    // inside ubuntu:24.04 with the workdir and bundle mounted; everything
    // else is the trusted native class.
    let docker_ok = std::process::Command::new("docker")
        .arg("info")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !docker_ok {
        eprintln!("skipping: docker unavailable");
        return;
    }
    let ws = Workspace::new(happy_script());
    let bundle = ws.bundle_dir();
    let config_path = bundle.join("bundle.json");
    let mut config: Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    // The fake driver binary lives outside workdir/bundle: declare its
    // directory as an extra read-only mount.
    let bin_dir = std::path::Path::new(forge_bin()).parent().unwrap();
    config["seats"]["intake"]["driver"]["confine"] = json!({
        "image": "ubuntu:24.04",
        "mounts": [bin_dir.to_str().unwrap()],
    });
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
}

#[test]
fn compile_rejects_bad_panels() {
    // One member.
    let ws = Workspace::new(happy_script());
    ws.make_panel("verify", &["only"], "unanimous-pass");
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.forge(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("at least two"), "stderr: {stderr}");

    // Unknown aggregate.
    let ws = Workspace::new(happy_script());
    ws.make_panel("verify", &["a", "b"], "vibes");
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.forge(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("unknown aggregate"), "stderr: {stderr}");
}

#[test]
fn undeclared_seat_inputs_never_reach_the_journal() {
    // Provenance (decision 0007): the review phase's rules reference
    // fixes_applied / has_security_residual / max_residual_severity, so
    // those are the seat's implied declaration; anything else it claims
    // is dropped before evaluation and never recorded.
    let mut script = happy_script();
    script["seats"]["review"] = json!([{"behavior": "succeed", "result": {
        "result": "clean",
        "inputs": {"fixes_applied": false,
                    "high_risk_uncovered": true,
                    "skip_verify": true}
    }}]);
    let ws = Workspace::new(script);
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);

    let db = ws.db();
    let out = ws.path().join("export");
    ws.forge(&[
        "export", "--run", &run_id,
        "--out", out.to_str().unwrap(),
        "--db", db.to_str().unwrap(),
    ]);
    let journal =
        std::fs::read_to_string(out.join(format!("{run_id}.ndjson"))).unwrap();
    let review_decision = journal
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|e| {
            e["type"] == "transition/decided" && e["payload"]["from"] == "review"
        })
        .expect("review decision in journal");
    let inputs = &review_decision["payload"]["inputs"];
    assert_eq!(inputs["fixes_applied"], false);
    assert!(inputs.get("high_risk_uncovered").is_none(), "inputs: {inputs}");
    assert!(inputs.get("skip_verify").is_none(), "inputs: {inputs}");
}

#[test]
fn compile_rejects_provenance_violations() {
    let cases: [(Value, &str); 3] = [
        (json!(["consecutive_failures"]), "engine-owned"),
        (json!(["made_up_fact"]), "unknown input"),
        // Rules from review reference has_security_residual and
        // max_residual_severity; declaring only fixes_applied starves them.
        (json!(["fixes_applied"]), "does not declare"),
    ];
    for (declaration, expected) in cases {
        let ws = Workspace::new(happy_script());
        let bundle = ws.bundle_dir();
        let config_path = bundle.join("bundle.json");
        let mut config: Value =
            serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
        config["seats"]["review"]["inputs"] = declaration.clone();
        std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();
        let (code, _, stderr) =
            ws.forge(&["compile", "--bundle", bundle.to_str().unwrap()]);
        assert_eq!(code, Some(1), "declaration {declaration} must be rejected");
        assert!(stderr.contains(expected), "declaration {declaration}: {stderr}");
    }
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
