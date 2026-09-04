//! Machine proof (delivery-sequence step 5): the whole engine driven by
//! the scripted fake driver over the real protocol, covering success,
//! retry, hard stop, schema park, indeterminate park, operator retry,
//! protocol garbage, crash recovery, bundle pinning, and the
//! constitutional compile rejections.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
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

/// The same machine with decision 0022's review constitution: a security
/// residual sends the run BACK to implement while implement's visit count
/// stays inside two reforgings, and the exhaustion ladder then stops,
/// parks, or ships it as named debt. The non-security rules are the v1
/// table's, character for character.
const REFORGING_POLICY: &str = r#"{
  "schema": "forge.phase-machine/v2",
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
    {"id": "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"has_security_residual": true, "visits_implement_gte": 3,
              "max_residual_severity_above": "medium"},
     "next": "stop", "severity": "hard",
     "reason": "Two reforgings spent and still above medium; the operator's now."},
    {"id": "REVIEW-REFORGE-EXHAUSTED-MEDIUM", "from": "review", "result": "residual",
     "when": {"has_security_residual": true, "visits_implement_gte": 3,
              "max_residual_severity_above": "low"},
     "park": true,
     "reason": "Two reforgings spent and a medium security residual survives."},
    {"id": "REVIEW-REFORGE-EXHAUSTED-DEBT", "from": "review", "result": "residual",
     "when": {"has_security_residual": true, "visits_implement_gte": 3,
              "fixes_applied": true},
     "next": "ship", "severity": "flagged",
     "reason": "Two reforgings spent; low or info with fixes applied ships as tracked debt."},
    {"id": "REVIEW-REFORGE-EXHAUSTED-UNFIXED", "from": "review", "result": "residual",
     "when": {"has_security_residual": true, "visits_implement_gte": 3},
     "park": true,
     "reason": "Two reforgings spent and a security residual survives unfixed."},
    {"id": "REVIEW-REFORGE", "from": "review", "result": "residual",
     "when": {"has_security_residual": true}, "next": "implement", "severity": "flagged",
     "reason": "Security residual, any severity: back into the fire."},
    {"id": "REVIEW-RESIDUAL-ABOVE-MEDIUM", "from": "review", "result": "residual",
     "when": {"max_residual_severity_above": "medium"}, "next": "stop",
     "severity": "hard", "reason": "Residual severity above medium; not shippable."},
    {"id": "REVIEW-RESIDUAL-OK", "from": "review", "result": "residual", "next": "ship",
     "severity": "flagged",
     "reason": "Non-security residuals at or below medium proceed as tracked debt."},
    {"id": "REVIEW-CLEAN-NO-FIXES", "from": "review", "result": "clean",
     "when": {"fixes_applied": false}, "next": "ship",
     "reason": "Clean with no code changed; verification evidence stands."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "verify",
     "reason": "Clean but fixes applied; re-verify before shipping."},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "Dirty tree at ship time is a defect."},
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "ship",
     "reason": "Gates passed and ledger written; confirm close-out and report shipped."},
    {"id": "SHIP-COMPLETE", "from": "ship", "result": "shipped", "next": "done",
     "reason": "Close-out confirmed: clean, reviewed, verified; done."}
  ]
}"#;

/// The routing surface of `recipes/triage`, kept deliberately small here:
/// the structural test compiles the shipped composed recipe, while this
/// table drives its two ruling-6 outcomes through the real engine and fake
/// protocol driver.
const TRIAGE_POLICY: &str = r#"{
  "schema": "forge.phase-machine/v2",
  "phases": ["triage", "design", "implement", "verify", "review", "ship", "done", "stop"],
  "initial": "triage",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id":"TRIAGE-CHORE","from":"triage","result":"chore","next":"implement","reason":"chore"},
    {"id":"TRIAGE-FEATURE","from":"triage","result":"feature","next":"implement","reason":"feature"},
    {"id":"TRIAGE-DESIGN","from":"triage","result":"design","next":"design","reason":"design"},
    {"id":"TRIAGE-ENGINE","from":"triage","result":"engine","next":"design","reason":"engine"},
    {"id":"TRIAGE-ESCALATE","from":"triage","result":"escalate","park":true,"reason":"escalated"},
    {"id":"DESIGN-OK","from":"design","result":"designed","next":"implement","reason":"designed"},
    {"id":"DESIGN-FAIL","from":"design","result":"fail","next":"stop","reason":"failed"},
    {"id":"IMPL-OK","from":"implement","result":"complete","next":"verify","reason":"complete"},
    {"id":"IMPL-BROKEN","from":"implement","result":"broken","next":"stop","reason":"broken"},
    {"id":"IMPL-BLOCKED","from":"implement","result":"blocked","next":"stop","reason":"blocked"},
    {"id":"IMPL-OVERSIZED-EXHAUSTED","from":"implement","result":"oversized","when":{"visits_triage_gte":2},"park":true,"reason":"exhausted"},
    {"id":"IMPL-OVERSIZED","from":"implement","result":"oversized","next":"triage","reason":"oversized"},
    {"id":"VERIFY-PASS","from":"verify","result":"pass","next":"review","reason":"pass"},
    {"id":"VERIFY-FAIL","from":"verify","result":"fail","next":"implement","reason":"fail"},
    {"id":"REVIEW-CLEAN","from":"review","result":"clean","next":"ship","reason":"clean"},
    {"id":"REVIEW-RESIDUAL","from":"review","result":"residual","next":"stop","reason":"residual"},
    {"id":"REVIEW-HOLD","from":"review","result":"security-hold","next":"stop","reason":"hold"},
    {"id":"SHIP-READY","from":"ship","result":"ready","next":"ship","reason":"ready"},
    {"id":"SHIP-COMPLETE","from":"ship","result":"shipped","next":"done","reason":"shipped"}
  ]
}"#;

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new(script: Value) -> Workspace {
        Workspace::with_policy(script, POLICY)
    }

    fn with_policy(script: Value, policy: &str) -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.bundle_dir();
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        std::fs::write(bundle.join("policy.json"), policy).unwrap();
        let script_path = ws.path().join("script.json");
        std::fs::write(&script_path, serde_json::to_string(&script).unwrap()).unwrap();

        let seat = |results: Vec<&str>| -> Value {
            json!({
                "role": "roles/role.md",
                "results": results,
                "driver": {"command": [
                    brokkr_bin(), "fake-driver",
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

    fn triage(script: Value) -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.bundle_dir();
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::create_dir_all(ws.path().join("dialects")).unwrap();
        std::fs::create_dir_all(ws.path().join("adapters")).unwrap();
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        std::fs::copy(
            root.join("dialects/speckit.json"),
            ws.path().join("dialects/speckit.json"),
        )
        .unwrap();
        std::fs::copy(
            root.join("adapters/exec.json"),
            ws.path().join("adapters/exec.json"),
        )
        .unwrap();
        std::fs::write(
            ws.path().join("realms.json"),
            r#"{"schema":"forge.realms/v3","realms":[{"name":"proof","path":".","default_branch":"main","dialect":"speckit"}],"journal":"forge.db"}"#,
        ).unwrap();
        std::fs::write(bundle.join("policy.json"), TRIAGE_POLICY).unwrap();
        let script_path = ws.path().join("script.json");
        std::fs::write(&script_path, serde_json::to_string(&script).unwrap()).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# test role\n").unwrap();
        let seat = |results: Vec<&str>| -> Value {
            json!({
                "role": "roles/role.md",
                "results": results,
                "driver": {"command": [
                    brokkr_bin(), "fake-driver",
                    "--script", script_path.to_string_lossy(),
                    "--state", ws.path().join("state").to_string_lossy(),
                ]}
            })
        };
        let body = || {
            json!({
                "role": "roles/role.md",
                "driver": {"command": [
                    brokkr_bin(), "fake-driver", "--script", script_path.to_string_lossy(),
                    "--state", ws.path().join("state").to_string_lossy()
                ]}
            })
        };
        let review_panel = |names: &[&str]| {
            let panel: serde_json::Map<String, Value> = names
                .iter()
                .map(|name| ((*name).to_string(), body()))
                .collect();
            json!({"aggregate":"review-panel", "panel":panel})
        };
        let review_sequence = |names: &[&str]| {
            json!({"sequence":[
                {"name":"positions", "aggregate":"review-panel", "panel": names.iter().map(|name| ((*name).to_string(), body())).collect::<serde_json::Map<String, Value>>()},
                {"name":"chief", "role":"roles/role.md", "driver": body()["driver"].clone()}
            ]})
        };
        let config = json!({
            "name": "triage-machine-proof",
            "policy": "policy.json",
            "protected_phase": "review",
            "seats": {
                "triage": seat(vec!["chore", "feature", "design", "engine", "escalate"]),
                "design": seat(vec!["designed", "fail"]),
                "implement": {"results":["complete", "broken", "blocked", "oversized"], "select": {
                    "on":"strategy", "cases": {
                        "chore":body(), "feature":body(), "design":body(), "engine":body()
                    }
                }},
                "verify": seat(vec!["pass", "fail"]),
                "review": {"results":["clean", "residual", "security-hold"], "select": {
                    "on":"strategy", "cases": {
                        "chore":body(),
                        "feature":review_panel(&["correctness", "security"]),
                        "design":review_sequence(&["correctness", "security", "spec-compliance"]),
                        "engine":review_sequence(&["adversarial", "correctness", "security", "spec-compliance"])
                    }
                }},
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

    fn brokkr(&self, args: &[&str]) -> (Option<i32>, Value, String) {
        let (code, stdout, stderr) = self.brokkr_raw(args);
        let value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
        (code, value, stderr)
    }

    /// Like `brokkr`, but returns stdout verbatim for commands whose
    /// output is not JSON (e.g. the tab-separated `runs` listing).
    fn brokkr_raw(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let out = Command::new(brokkr_bin())
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
        let realms = self.path().join("realms.json");
        let mut args = vec![
            "run",
            "--bundle",
            bundle.to_str().unwrap(),
            "--feature",
            "proof feature",
            "--db",
            db.to_str().unwrap(),
        ];
        if realms.exists() {
            args.extend(["--realms", realms.to_str().unwrap()]);
        }
        self.brokkr(&args)
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

    /// Convert one phase's seat into a sequence of fake-driver steps. A
    /// spec {"name": N} becomes a single step; {"name": N, "members":
    /// [...], "aggregate": A} a panel step. Script entries key by
    /// "<phase>:<step>" (single) and "<phase>:<step>:<member>" (panel).
    fn make_sequence(&self, phase: &str, specs: &[Value]) {
        let path = self.bundle_dir().join("bundle.json");
        let mut config: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let driver = config["seats"][phase]["driver"].clone();
        let seat_results = config["seats"][phase]["results"].clone();
        let steps: Vec<Value> = specs
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                let name = spec["name"].as_str().unwrap();
                let mut step = match spec.get("members") {
                    None => json!({
                        "name": name,
                        "results": spec.get("results").unwrap_or(&seat_results).clone(),
                        "role": "roles/role.md", "driver": driver.clone(),
                    }),
                    Some(members) => {
                        let mut panel = serde_json::Map::new();
                        for member in members.as_array().unwrap() {
                            panel.insert(
                                member.as_str().unwrap().to_string(),
                                json!({"role": "roles/role.md", "driver": driver.clone()}),
                            );
                        }
                        json!({
                            "name": name,
                            "results": match spec["aggregate"].as_str() {
                                Some("review-panel") => json!(["clean", "residual", "security-hold"]),
                                _ => json!(["pass", "fail"]),
                            },
                            "panel": panel,
                            "aggregate": spec["aggregate"],
                        })
                    }
                };
                if index + 1 == specs.len() {
                    step.as_object_mut().unwrap().remove("results");
                }
                step
            })
            .collect();
        let seat = config["seats"][phase].as_object_mut().unwrap();
        seat.insert("sequence".into(), json!(steps));
        seat.remove("role");
        seat.remove("driver");
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
    }

    /// Attach a `requires_artifacts` declaration to one policy rule.
    fn set_rule_requires_artifacts(&self, rule_id: &str, artifacts: &[&str]) {
        let path = self.bundle_dir().join("policy.json");
        let mut policy: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let rule = policy["rules"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == rule_id)
            .unwrap_or_else(|| panic!("no rule {rule_id}"));
        rule["requires_artifacts"] = json!(artifacts);
        std::fs::write(&path, serde_json::to_string(&policy).unwrap()).unwrap();
    }

    /// A run workdir distinct from the harness root (which holds the db
    /// and bundle), so replay proofs can delete it outright.
    fn workdir(&self) -> PathBuf {
        let dir = self.path().join("work");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run_in_workdir(&self) -> (Option<i32>, Value, String) {
        let bundle = self.bundle_dir();
        let db = self.db();
        let work = self.workdir();
        self.brokkr(&[
            "run",
            "--bundle",
            bundle.to_str().unwrap(),
            "--feature",
            "proof feature",
            "--db",
            db.to_str().unwrap(),
            "--repo",
            work.to_str().unwrap(),
        ])
    }

    fn resume_in_workdir(&self, run_id: &str) -> (Option<i32>, Value, String) {
        let bundle = self.bundle_dir();
        let db = self.db();
        let work = self.workdir();
        self.brokkr(&[
            "resume",
            "--run",
            run_id,
            "--bundle",
            bundle.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
            "--repo",
            work.to_str().unwrap(),
        ])
    }

    /// Export the run's journal and parse it into events.
    fn exported_events(&self, run_id: &str) -> Vec<Value> {
        let db = self.db();
        let out = self.path().join("export");
        self.brokkr(&[
            "export",
            "--run",
            run_id,
            "--out",
            out.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ]);
        std::fs::read_to_string(out.join(format!("{run_id}.ndjson")))
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .collect()
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
fn triage_routes_chore_and_escalation_parks_with_its_reasoning() {
    let chore = json!({"seats": {
        "triage": [{"behavior":"succeed", "result": {
            "result":"chore", "notes":"bounded maintenance"
        }}],
        "implement:chore": [{"behavior":"succeed", "result": {
            "result":"complete", "inputs":{"strategy":"escalate"}
        }}],
        "verify": [{"behavior":"succeed", "result":{"result":"pass"}}],
        "review:chore": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "ship": [
            {"behavior":"succeed", "result":{"result":"ready"}},
            {"behavior":"succeed", "result":{"result":"shipped"}}
        ]
    }});
    let ws = Workspace::triage(chore);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["strategy"], "chore");
    let run_id = Workspace::run_id(&stderr);
    let decisions: Vec<Value> = ws
        .exported_events(&run_id)
        .into_iter()
        .filter(|event| event["type"] == "transition/decided")
        .collect();
    let implement = decisions
        .iter()
        .find(|event| event["payload"]["from"] == "implement")
        .expect("implement decision");
    assert_eq!(
        implement["payload"]["inputs"]["strategy"], "chore",
        "the seat's forged strategy claim is dropped before the fold-owned fact overlays it"
    );

    let returned = json!({"seats": {
        "triage": [
            {"behavior":"echo", "result":{"result":"feature"}},
            {"behavior":"echo", "result":{"result":"feature"}}
        ],
        "implement:feature": [
            {"behavior":"succeed", "result":{"result":"oversized"}},
            {"behavior":"succeed", "result":{"result":"complete"}}
        ],
        "verify": [{"behavior":"succeed", "result":{"result":"pass"}}],
        "review:feature:correctness": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:feature:security": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "ship": [
            {"behavior":"succeed", "result":{"result":"ready"}},
            {"behavior":"succeed", "result":{"result":"shipped"}}
        ]
    }});
    let ws = Workspace::triage(returned);
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let triage_inputs: Vec<Value> = ws
        .exported_events(&Workspace::run_id(&stderr))
        .into_iter()
        .filter(|event| {
            event["type"] == "effect/succeeded"
                && event["payload"]["result"]["seat_input"]["phase"] == "triage"
        })
        .map(|event| event["payload"]["result"]["seat_input"].clone())
        .collect();
    assert_eq!(triage_inputs.len(), 2);
    for input in triage_inputs {
        assert_eq!(input["feature"], "proof feature");
        assert!(input["context"].get("last_decision").is_none());
        assert!(input["context"].get("returned_from").is_none());
    }

    let escalation = json!({"seats": {
        "triage": [{"behavior":"succeed", "result": {
            "result":"escalate", "notes":"split the commission at the frozen boundary"
        }}]
    }});
    let ws = Workspace::triage(escalation);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert_eq!(summary["status"], "awaiting_operator");
    assert_eq!(summary["strategy"], "escalate");
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.ends_with("split the commission at the frozen boundary"),
        "triage's reasoning is the journaled park reason: {reason}"
    );
}

#[test]
fn triage_selection_serves_the_single_reviewer_and_the_engine_panel_then_chief() {
    let script = json!({"seats": {
        "triage": [{"behavior":"succeed", "result":{"result":"engine"}}],
        "design": [{"behavior":"succeed", "result":{"result":"designed"}}],
        "implement:engine": [{"behavior":"succeed", "result":{"result":"complete"}}],
        "verify": [{"behavior":"succeed", "result":{"result":"pass"}}],
        "review:engine:positions:adversarial": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:engine:positions:correctness": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:engine:positions:security": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:engine:positions:spec-compliance": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:engine:chief": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "ship": [
            {"behavior":"succeed", "result":{"result":"ready"}},
            {"behavior":"succeed", "result":{"result":"shipped"}}
        ]
    }});
    let ws = Workspace::triage(script);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["strategy"], "engine");
    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let entered: Vec<&str> = events
        .iter()
        .filter(|event| event["type"] == "phase/entered")
        .filter_map(|event| event["payload"]["case"].as_str())
        .collect();
    assert_eq!(entered, ["engine", "engine"]);
    let members: Vec<&str> = events
        .iter()
        .filter(|event| event["type"] == "effect/checkpointed")
        .filter_map(|event| event["payload"]["checkpoint"]["member"].as_str())
        .collect();
    for member in ["adversarial", "correctness", "security", "spec-compliance"] {
        assert!(
            members.iter().any(|served| served.ends_with(member)),
            "engine review did not serve {member}"
        );
    }
    assert!(events.iter().any(|event| {
        event["type"] == "effect/checkpointed"
            && event["payload"]["checkpoint"]["member"] == "chief"
    }));
}

#[cfg(target_os = "linux")]
#[test]
fn dialect_validate_expands_the_chiefs_change_and_records_tool_evidence() {
    let script = json!({"seats": {
        "triage": [{"behavior":"succeed", "result":{"result":"design"}}],
        "design:author": [{"behavior":"succeed", "result":{
            "result":"drafted", "inputs":{"change":"change-42"}
        }}],
        "implement:design": [{"behavior":"succeed", "result":{"result":"complete"}}],
        "verify": [{"behavior":"succeed", "result":{"result":"pass"}}],
        "review:design:positions:correctness": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:design:positions:security": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:design:positions:spec-compliance": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "review:design:chief": [{"behavior":"succeed", "result":{"result":"clean"}}],
        "ship": [
            {"behavior":"succeed", "result":{"result":"ready"}},
            {"behavior":"succeed", "result":{"result":"shipped"}}
        ]
    }});
    let ws = Workspace::triage(script);

    let bundle_path = ws.bundle_dir().join("bundle.json");
    let mut bundle: Value = serde_json::from_slice(&std::fs::read(&bundle_path).unwrap()).unwrap();
    let author_driver = bundle["seats"]["design"]["driver"].clone();
    bundle["seats"]["design"] = json!({
        "results":["drafted","fail"], "inputs":["change"], "sequence":[
            {"name":"author", "results":["drafted"], "role":"roles/role.md", "driver":author_driver},
            {"name":"validate", "dialect":"validate"}
        ]
    });
    std::fs::write(&bundle_path, bundle.to_string()).unwrap();
    let policy_path = ws.bundle_dir().join("policy.json");
    let mut policy: Value = serde_json::from_slice(&std::fs::read(&policy_path).unwrap()).unwrap();
    policy["rules"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|rule| rule["id"] == "DESIGN-OK")
        .unwrap()["result"] = json!("drafted");
    std::fs::write(&policy_path, policy.to_string()).unwrap();

    let dialect_path = ws.path().join("dialects/speckit.json");
    let mut dialect: Value =
        serde_json::from_slice(&std::fs::read(&dialect_path).unwrap()).unwrap();
    dialect["phases"]["design"]["validate"] = json!({
        "argv":["sh","-c","cat >/dev/null; test \"$1\" = change-42; printf validated","sh","{change}"],
        "state":["sh","-c","printf framework-state"]
    });
    std::fs::write(&dialect_path, dialect.to_string()).unwrap();

    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(!stderr.contains(" is adopted "), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    let run_id = Workspace::run_id(&stderr);
    let events = brokkr_store::Store::open(&ws.db())
        .unwrap()
        .load(&run_id)
        .unwrap();
    let result = events
        .iter()
        .find_map(|event| {
            (event.event_type == brokkr_core::EventType::EffectSucceeded
                && event.payload["result"]["notes"] == "validated")
                .then_some(&event.payload["result"])
        })
        .expect("dialect validator result");
    assert_eq!(result["inputs"]["change"], "change-42");
    assert_eq!(result["state"], "framework-state");
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
    let (code, replay, _) = ws.brokkr(&["replay", "--run", &run_id, "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(replay["replay"], "deterministic");

    let out = ws.path().join("export");
    let (code, _, _) = ws.brokkr(&[
        "export",
        "--run",
        &run_id,
        "--out",
        out.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let journal = out.join(format!("{run_id}.ndjson"));
    let (code, verified, _) = ws.brokkr(&["verify-run", journal.to_str().unwrap()]);
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

/// A run over the reforging table (decision 0022): the review seat rules
/// whatever the script says, in order, and every other seat walks the
/// happy path.
fn reforging_script(reviews: Value, implement: Value) -> Value {
    json!({"seats": {
        "intake": [{"behavior": "succeed", "result": {"result": "resolved"}}],
        "implement": implement,
        "verify": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review": reviews,
        "ship": [
            {"behavior": "succeed", "result": {"result": "ready"}},
            {"behavior": "succeed", "result": {"result": "shipped"}},
        ],
    }})
}

/// A review ruling that records a security residual — the finding text
/// rides in `notes`, which no rule reads and the returning implement
/// seat does.
fn security_residual(severity: &str, fixes_applied: bool) -> Value {
    json!({"behavior": "succeed", "result": {
        "result": "residual",
        "notes": "the mount is joined with an unsanitised path",
        "inputs": {
            "has_security_residual": true,
            "max_residual_severity": severity,
            "fixes_applied": fixes_applied,
        },
    }})
}

/// Every ruling the run took, in order; a rule-driven park reads as its
/// own rule id with no transition beside it.
fn rule_ids(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|event| event["type"] == "transition/decided")
        .map(|event| {
            event["payload"]["rule_id"]
                .as_str()
                .unwrap_or("NO-RULE")
                .to_string()
        })
        .collect()
}

fn only_review_decision(events: &[Value], nth: usize) -> Value {
    events
        .iter()
        .filter(|event| {
            event["type"] == "transition/decided" && event["payload"]["from"] == "review"
        })
        .nth(nth)
        .expect("a review decision")["payload"]
        .clone()
}

/// Decision 0022 ruling 1 and 4: a security residual — HIGH here, since
/// the back-edge is severity-blind on the way in — sends the run back to
/// implement, the forward path reruns as itself, and a clean re-review
/// reaches ship with the whole arc in one journal.
#[test]
fn a_security_residual_reforges_once_and_the_clean_re_review_ships() {
    let script = reforging_script(
        json!([
            security_residual("high", true),
            {"behavior": "succeed",
             "result": {"result": "clean", "inputs": {"fixes_applied": false}}},
        ]),
        json!([{"behavior": "succeed", "result": {"result": "complete"}}]),
    );
    let ws = Workspace::with_policy(script, REFORGING_POLICY);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["phase"], "done");

    let events = ws.exported_events(&Workspace::run_id(&stderr));
    assert_eq!(
        rule_ids(&events),
        [
            "INTAKE-OK",
            "IMPL-OK",
            "VERIFY-PASS",
            "REVIEW-REFORGE",
            "IMPL-OK",
            "VERIFY-PASS",
            "REVIEW-CLEAN-NO-FIXES",
            "SHIP-READY",
            "SHIP-COMPLETE",
        ]
    );
    // The bound is read from the journal's own count, not from the seat.
    assert_eq!(
        only_review_decision(&events, 0)["inputs"]["visits_implement"],
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|e| e["type"] == "phase/entered" && e["payload"]["phase"] == "implement")
            .count(),
        2
    );
}

/// Decision 0022 ruling 3, all four arms, and the bound that reaches
/// them: two reforgings, then the ladder. Every arm is one run of the
/// same table, differing only in what the third review reports.
#[test]
fn the_bound_ends_at_two_reforgings_and_the_ladder_takes_every_arm() {
    let arm = |last_review: Value| {
        let script = reforging_script(
            json!([
                security_residual("low", true),
                security_residual("low", true),
                last_review,
            ]),
            json!([{"behavior": "succeed", "result": {"result": "complete"}}]),
        );
        let ws = Workspace::with_policy(script, REFORGING_POLICY);
        let (code, summary, stderr) = ws.run();
        let events = ws.exported_events(&Workspace::run_id(&stderr));
        (code, summary, events)
    };

    // Above medium: a hard stop. The machine tried; now it is the operator's.
    let (code, summary, events) = arm(security_residual("critical", true));
    assert_eq!(code, Some(3));
    assert_eq!(summary["status"], "stopped");
    assert_eq!(
        summary["last_decision"]["rule_id"],
        "REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM"
    );
    assert_eq!(
        only_review_decision(&events, 2)["inputs"]["visits_implement"],
        3,
        "the bound is the journal's count of implement's visits"
    );
    assert_eq!(
        rule_ids(&events)
            .iter()
            .filter(|id| *id == "REVIEW-REFORGE")
            .count(),
        2
    );

    // Medium: the run PARKS awaiting the operator, the residual its reason.
    let (code, summary, events) = arm(security_residual("medium", true));
    assert_eq!(code, Some(2));
    assert_eq!(summary["status"], "awaiting_operator");
    assert_eq!(summary["phase"], "review", "parked where it stands");
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.starts_with("REVIEW-REFORGE-EXHAUSTED-MEDIUM for (review, residual):"),
        "a rule-driven park names its rule, never 'no ruling': {reason}"
    );
    assert!(
        reason.contains("medium security residual survives"),
        "{reason}"
    );
    assert_eq!(
        only_review_decision(&events, 2)["severity"],
        Value::Null,
        "no transition was taken, so no ruling severity is claimed"
    );

    // Low with fixes applied: ships as tracked debt, the ruling named.
    let (code, summary, events) = arm(security_residual("low", true));
    assert_eq!(code, Some(0));
    assert_eq!(summary["status"], "completed");
    assert!(rule_ids(&events).contains(&"REVIEW-REFORGE-EXHAUSTED-DEBT".to_string()));
    assert_eq!(only_review_decision(&events, 2)["severity"], "flagged");

    // Low, unfixed: the same operator door a medium takes.
    let (code, summary, _) = arm(security_residual("low", false));
    assert_eq!(code, Some(2));
    assert_eq!(summary["status"], "awaiting_operator");
    assert!(
        summary["park_reason"]
            .as_str()
            .unwrap()
            .starts_with("REVIEW-REFORGE-EXHAUSTED-UNFIXED for (review, residual):"),
        "park reason: {}",
        summary["park_reason"]
    );
}

/// Decision 0022 ruling 5: the non-security rules are untouched. A
/// residual with no security finding rules exactly as it did before —
/// above medium stops, at or below medium ships as tracked debt — and
/// never consumes a reforging.
#[test]
fn non_security_residuals_rule_exactly_as_they_did() {
    let plain = |severity: &str| {
        json!([{"behavior": "succeed", "result": {
            "result": "residual",
            "inputs": {"has_security_residual": false, "max_residual_severity": severity},
        }}])
    };
    let implement = json!([{"behavior": "succeed", "result": {"result": "complete"}}]);

    let ws = Workspace::with_policy(
        reforging_script(plain("high"), implement.clone()),
        REFORGING_POLICY,
    );
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(3));
    assert_eq!(
        summary["last_decision"]["rule_id"],
        "REVIEW-RESIDUAL-ABOVE-MEDIUM"
    );

    let ws = Workspace::with_policy(reforging_script(plain("low"), implement), REFORGING_POLICY);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    let events = ws.exported_events(&Workspace::run_id(&stderr));
    assert!(rule_ids(&events).contains(&"REVIEW-RESIDUAL-OK".to_string()));
}

/// Decision 0022 ruling 1's other half: the finding TRAVELS. The
/// returning implement seat is handed the review effect's whole result —
/// findings, severities, notes — and a seat on its first visit is handed
/// nothing new, so a run that never reforges builds the input, and the
/// digest, it always built.
#[test]
fn the_returning_implement_seat_is_handed_the_review_that_sent_it_back() {
    let script = reforging_script(
        json!([
            security_residual("low", true),
            {"behavior": "succeed",
             "result": {"result": "clean", "inputs": {"fixes_applied": false}}},
        ]),
        // `echo` hands the seat's own input back inside its result, which
        // is the only way a scripted proof can show WHAT a seat was told.
        json!([{"behavior": "echo", "result": {"result": "complete"}}]),
    );
    let ws = Workspace::with_policy(script, REFORGING_POLICY);
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");

    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let seat_inputs: Vec<Value> = events
        .iter()
        .filter(|event| {
            event["type"] == "effect/succeeded"
                && event["payload"]["result"]["seat_input"]["phase"] == "implement"
        })
        .map(|event| event["payload"]["result"]["seat_input"].clone())
        .collect();
    assert_eq!(seat_inputs.len(), 2, "one per implement visit");

    assert!(
        seat_inputs[0]["context"].get("returned_from").is_none(),
        "a first visit receives nothing new: {}",
        seat_inputs[0]["context"]
    );

    let returned = &seat_inputs[1]["context"]["returned_from"];
    assert_eq!(returned["phase"], "review");
    assert_eq!(returned["result"]["result"], "residual");
    assert_eq!(
        returned["result"]["notes"], "the mount is joined with an unsanitised path",
        "the finding text itself, not the boolean it was reduced to"
    );
    assert_eq!(returned["result"]["inputs"]["max_residual_severity"], "low");
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
    assert_eq!(
        code,
        Some(0),
        "first broken must RETRY (counter=1), not hard-stop"
    );
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
    let (code, _, _) = ws.brokkr(&[
        "operator",
        "retry",
        "--run",
        &run_id,
        "--reason",
        "scripted retry",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let bundle = ws.bundle_dir();
    let (code, summary, _) = ws.brokkr(&[
        "resume",
        "--run",
        &run_id,
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
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
    // Machines read --json, which emits the view model verbatim; the
    // human form is a clamped line, pinned by render.rs's goldens.
    let (code, stdout, stderr) = ws.brokkr_raw(&["runs", "--json", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let view: Value = serde_json::from_str(&stdout).unwrap();
    // Decision 0016 moved the wire version to 2: participants gained
    // `provenance`, the run view gained `notices`. It moved to 3 when
    // the phase rail gained `returns`, to 4 for decision 0031's
    // provider-reported served-model cells, and to 5 for decision 0032's
    // common transcript shape. The constant is the one source of truth.
    assert_eq!(view["view_version"], brokkr_view::VIEW_VERSION);
    assert_eq!(view["count"], 1, "the count the trailer used to print");
    let row = &view["runs"][0];
    assert_eq!(row["run_id"], run_id.as_str());
    assert_eq!(row["feature"], "proof feature");
    assert_eq!(row["status"], "completed");
    assert_eq!(row["status_known"], true);
    assert_eq!(row["phase"], "done");

    // The human form: one clamped line per run, and no trailer.
    let (code, human, stderr) = ws.brokkr_raw(&["runs", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(human.lines().count(), 1, "one line per run: {human}");
    let line = human.lines().next().unwrap();
    assert!(line.starts_with(&run_id), "{line}");
    assert!(line.contains("completed"), "{line}");
    assert!(line.contains("done"), "{line}");
    assert!(!human.contains("1 runs"), "the trailer moved to --json");
}

/// The fleet-blinding fault, end to end. An `operator/accepted` on a run
/// that has already finished is `FoldError::AfterTerminal` forever, and
/// one such journal must cost that one run and never the whole fleet:
/// `runs` lists everything and quarantines the poisoned row with the
/// fold's own words, while the verbs aimed at that run still fail.
///
/// `brokkr operator` used to be a way to CREATE this fault — it wrote
/// the acceptance without re-reading the run — and this test used to
/// reach for it. It no longer can, which the first half now proves: the
/// command is fenced, it refuses, and it says why. The poisoned fixture
/// is therefore built directly through the store, because a journal in
/// this state can now only come from a hand-edited file or from one
/// written before the fence existed. Both still have to be readable
/// without blinding the fleet, so the quarantine is still on trial here.
#[test]
fn one_unfoldable_journal_is_quarantined_by_runs_and_still_fatal_to_its_own_verbs() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let healthy = Workspace::run_id(&stderr);

    let db = ws.db();
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.brokkr(&[
        "rerun",
        "--run",
        &healthy,
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let prefix = format!("rerun of {healthy} as ");
    let poisoned = stderr
        .lines()
        .find(|line| line.starts_with(&prefix))
        .map(|line| line[prefix.len()..].split(' ').next().unwrap().to_string())
        .unwrap_or_else(|| panic!("rerun announcement on stderr: {stderr}"));

    // The operator's command lands on a run that has already finished.
    // The fence catches it: refused, not accepted, and journaled as a
    // refusal — a real process against a real journal, which is the
    // between-effects write race proved where it is actually spent.
    let (code, _, stderr) = ws.brokkr_raw(&[
        "operator",
        "--run",
        &poisoned,
        "--reason",
        "stop it now",
        "--db",
        db.to_str().unwrap(),
        "stop",
    ]);
    assert_eq!(code, Some(1), "the fence refuses it: {stderr}");
    assert!(
        stderr.contains("after_terminal"),
        "and names the condition — the run had finished before the command was \
         given, which is a refusal, not a lost race: {stderr}"
    );
    // Refused, so the run is still perfectly readable.
    let (code, _, stderr) =
        ws.brokkr_raw(&["inspect", "--run", &poisoned, "--db", db.to_str().unwrap()]);
    assert_eq!(
        code,
        Some(0),
        "the refusal left the journal foldable: {stderr}"
    );

    // Now poison it deliberately, the only way that is still open: write
    // the acceptance straight into the journal, behind the engine's back.
    {
        let mut store = brokkr_store::Store::open(&db).unwrap();
        for (event_type, payload) in [
            (
                brokkr_core::envelope::EventType::OperatorCommanded,
                json!({"command_id":"hand-edited","command":"stop","args":{},"operator":"operator"}),
            ),
            (
                brokkr_core::envelope::EventType::OperatorAccepted,
                json!({"command_id":"hand-edited","operator":"operator","reason":"stop it now"}),
            ),
        ] {
            store
                .append_next(&poisoned, event_type, payload, None, None)
                .unwrap();
        }
    }

    let (code, stdout, stderr) = ws.brokkr_raw(&["runs", "--json", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "the fleet still lists: {stderr}");
    let view: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(view["count"], 2, "neither run is dropped: {stdout}");
    let row = |run_id: &str| -> Value {
        view["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["run_id"] == run_id)
            .unwrap_or_else(|| panic!("a row for {run_id}: {stdout}"))
            .clone()
    };
    assert_eq!(row(&healthy)["status"], "completed");
    assert_eq!(row(&healthy)["detail"], Value::Null);
    let quarantined = row(&poisoned);
    assert_eq!(quarantined["status"], Value::Null, "printed as '?'");
    assert_eq!(quarantined["status_known"], false);
    assert!(
        quarantined["detail"]
            .as_str()
            .unwrap()
            .contains("event after terminal status"),
        "the row carries the fold's own words: {quarantined}"
    );

    // The human form says it too, under the row it belongs to.
    let (code, human, stderr) = ws.brokkr_raw(&["runs", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(human.lines().count(), 3, "two rows, one explained: {human}");
    assert!(
        human
            .lines()
            .any(|line| line.starts_with("  fold  ") && line.contains("terminal")),
        "{human}"
    );

    // Aimed at that run, the refusal is still fatal.
    for args in [
        vec!["inspect", "--run", &poisoned, "--db"],
        vec!["replay", "--run", &poisoned, "--db"],
    ] {
        let mut args = args;
        let db = db.to_str().unwrap().to_string();
        args.push(&db);
        let (code, _, stderr) = ws.brokkr_raw(&args);
        assert_ne!(code, Some(0), "{args:?} must fail: {stderr}");
        assert!(stderr.contains("event after terminal status"), "{stderr}");
    }
    let (code, _, stderr) = ws.brokkr_raw(&[
        "resume",
        "--run",
        &poisoned,
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_ne!(code, Some(0), "resume must fail: {stderr}");
    assert!(stderr.contains("event after terminal status"), "{stderr}");
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
    let (code, stdout, stderr) = ws.brokkr_raw(&["runs", "--json", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let view: Value = serde_json::from_str(&stdout).unwrap();
    let row = view["runs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["run_id"] == run_id.as_str())
        .expect("the run is listed");
    assert_eq!(row["status"], "awaiting_operator");
    assert_eq!(
        row["phase"], "implement",
        "the phase the operator must act on"
    );
}

#[test]
fn protocol_garbage_fails_closed_and_parks() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([{"behavior": "garbage"}]);
    let ws = Workspace::new(script);
    let (code, summary, _) = ws.run();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.contains("unreadable driver message"),
        "park reason: {reason}"
    );
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
    assert_eq!(
        code,
        Some(0),
        "one automated retry within the declared limit"
    );
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
    assert!(
        reason.contains("failed 2 of 2 attempt(s)"),
        "park reason: {reason}"
    );
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
    assert_eq!(
        code,
        Some(2),
        "indeterminate parks into operator judgment, never auto-retries"
    );
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("indeterminate"), "park reason: {reason}");
}

#[test]
fn panel_unanimous_pass_completes_with_member_evidence() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] = json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
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
    ws.brokkr(&[
        "export",
        "--run",
        &run_id,
        "--out",
        out.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    let journal = std::fs::read_to_string(out.join(format!("{run_id}.ndjson"))).unwrap();
    let events: Vec<Value> = journal
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .collect();

    // Each member's live checkpoint (the fake driver's 'working' step)
    // streams into the journal member-tagged, BEFORE the attempt's
    // terminal effect event. Cross-member arrival order is wall-clock
    // and nondeterministic — compare the member set, not a sequence.
    let live: Vec<(usize, &Value)> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            e["payload"]["checkpoint"]["step"] == "working"
                && e["payload"]["checkpoint"]["member"].is_string()
        })
        .collect();
    let live_members: std::collections::BTreeSet<&str> = live
        .iter()
        .map(|(_, e)| e["payload"]["checkpoint"]["member"].as_str().unwrap())
        .collect();
    assert_eq!(
        live_members,
        ["integration", "unit"].into_iter().collect(),
        "both members' checkpoints journal live, member-tagged"
    );
    let attempt_id = live[0].1["payload"]["attempt_id"].clone();
    let terminal = events
        .iter()
        .position(|e| e["type"] == "effect/succeeded" && e["payload"]["attempt_id"] == attempt_id)
        .expect("panel attempt has a terminal effect event");
    let last_live = live.iter().map(|(i, _)| *i).max().unwrap();
    assert!(
        last_live < terminal,
        "live checkpoints land before the terminal event, not post-join"
    );

    // Member outcomes are journaled as checkpoint evidence, in declared
    // order, AFTER the live checkpoints, under the ONE effect the outer
    // machine saw.
    let summaries: Vec<(usize, &str)> = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e["payload"]["checkpoint"]["step"] == "panel-member-finished")
        .map(|(i, e)| (i, e["payload"]["checkpoint"]["member"].as_str().unwrap()))
        .collect();
    let summary_members: Vec<&str> = summaries.iter().map(|(_, m)| *m).collect();
    assert_eq!(
        summary_members,
        vec!["integration", "unit"],
        "declared (sorted) order"
    );
    assert!(
        summaries.iter().all(|(i, _)| *i > last_live),
        "summaries journal post-join, after the live stream"
    );
}

#[test]
fn panel_one_failing_member_fails_the_phase() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] = json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
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
    assert_eq!(
        code,
        Some(3),
        "one security-hold member holds the whole panel"
    );
    assert_eq!(summary["last_decision"]["rule_id"], "REVIEW-SECURITY-HOLD");
}

#[test]
fn panel_member_vanish_parks_the_whole_attempt_indeterminate() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] = json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
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
    if !cfg!(target_os = "linux") {
        eprintln!("skipping: linux containers only");
        return;
    }
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
    let bin_dir = std::path::Path::new(brokkr_bin()).parent().unwrap();
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
fn panel_member_out_of_vocabulary_parks_never_coerces() {
    let mut script = happy_script();
    script["seats"]["verify:unit"] = json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    script["seats"]["verify:integration"] =
        json!([{"behavior": "succeed", "result": {"result": "banana"}}]);
    let ws = Workspace::new(script);
    ws.make_panel("verify", &["unit", "integration"], "unanimous-pass");
    let (code, summary, _) = ws.run();
    assert_eq!(
        code,
        Some(2),
        "out-of-vocabulary never coerces to fail (law 0001)"
    );
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(reason.contains("schema"), "park reason: {reason}");
}

#[test]
fn compile_rejects_bad_panels() {
    // One member.
    let ws = Workspace::new(happy_script());
    ws.make_panel("verify", &["only"], "unanimous-pass");
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("at least two"), "stderr: {stderr}");

    // Unknown aggregate.
    let ws = Workspace::new(happy_script());
    ws.make_panel("verify", &["a", "b"], "vibes");
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("unknown aggregate"), "stderr: {stderr}");
}

#[test]
fn sequence_final_step_decides_and_steps_checkpoint_in_order() {
    // Two single steps: the first's result is checkpoint evidence only;
    // the second's decides the phase (IMPL-OK) exactly as a single seat
    // would. Step-1's result never has to be in the seat vocabulary.
    let mut script = happy_script();
    script["seats"]["implement:draft"] = json!([{"behavior": "succeed",
        "result": {"result": "drafted", "notes": "positions taken"}}]);
    script["seats"]["implement:chief"] =
        json!([{"behavior": "succeed", "result": {"result": "complete"}}]);
    let ws = Workspace::new(script);
    ws.make_sequence(
        "implement",
        &[
            json!({"name": "draft", "results": ["drafted"]}),
            json!({"name": "chief"}),
        ],
    );
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");

    let events = ws.exported_events(&Workspace::run_id(&stderr));
    // Each step's live checkpoints stream member-tagged with the step
    // name, so the seats console picks steps up unchanged.
    let live_members: std::collections::BTreeSet<&str> = events
        .iter()
        .filter(|e| {
            e["payload"]["checkpoint"]["step"] == "working"
                && e["payload"]["checkpoint"]["member"].is_string()
        })
        .map(|e| e["payload"]["checkpoint"]["member"].as_str().unwrap())
        .collect();
    assert_eq!(live_members, ["chief", "draft"].into_iter().collect());

    // The non-final step's result is journaled as a
    // sequence-step-finished checkpoint BEFORE the terminal event.
    let finished = events
        .iter()
        .position(|e| e["payload"]["checkpoint"]["step"] == "sequence-step-finished")
        .expect("step 1 finished checkpoint");
    let checkpoint = &events[finished]["payload"]["checkpoint"];
    assert_eq!(checkpoint["step_name"], "draft");
    assert_eq!(checkpoint["result"]["result"], "drafted");
    let succeeded = events
        .iter()
        .position(|e| {
            e["type"] == "effect/succeeded" && e["payload"]["result"]["result"] == "complete"
        })
        .expect("terminal effect event carries the FINAL step's result");
    assert!(
        finished < succeeded,
        "step checkpoint precedes the terminal event"
    );
    let decision = events
        .iter()
        .find(|e| e["type"] == "transition/decided" && e["payload"]["from"] == "implement")
        .expect("implement decision");
    assert_eq!(decision["payload"]["rule_id"], "IMPL-OK");
}

#[test]
fn sequence_step_failure_fails_attempt_and_retry_restarts_from_step_one() {
    // Step 2 fails on the first attempt: the WHOLE attempt fails
    // (0006-retryable), and with max_attempts 2 the retry restarts from
    // step 1. The fake driver counts attempts per seat name, so step 1
    // is on its second entry during the retry — its single entry repeats.
    let mut script = happy_script();
    script["seats"]["implement:one"] =
        json!([{"behavior": "succeed", "result": {"result": "first"}}]);
    script["seats"]["implement:two"] = json!([
        {"behavior": "fail", "error": "chief crashed"},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    ws.make_sequence(
        "implement",
        &[
            json!({"name": "one", "results": ["first"]}),
            json!({"name": "two"}),
        ],
    );
    ws.set_seat_limits("implement", 2, 3600);
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");

    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let failed = events
        .iter()
        .find(|e| e["type"] == "effect/failed")
        .expect("first attempt failed");
    let error = failed["payload"]["error"].as_str().unwrap();
    assert!(
        error.contains("sequence step 'two'"),
        "error names the step: {error}"
    );
    assert!(
        error.contains("chief crashed"),
        "error carries the driver error: {error}"
    );
    let step_one_runs = events
        .iter()
        .filter(|e| {
            e["payload"]["checkpoint"]["step"] == "sequence-step-finished"
                && e["payload"]["checkpoint"]["step_name"] == "one"
        })
        .count();
    assert_eq!(
        step_one_runs, 2,
        "the retry restarts from step 1, not step 2"
    );
}

#[test]
fn sequence_panel_step_tags_member_checkpoints_with_step_prefix() {
    // A panel step inside a sequence keeps panel semantics internally;
    // its members' checkpoints are member-tagged "<step>:<member>". The
    // panel is non-final, so its aggregate vocabulary (pass/fail) need
    // not appear in the seat's declared results.
    let mut script = happy_script();
    script["seats"]["implement:positions:econ"] =
        json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    script["seats"]["implement:positions:legal"] =
        json!([{"behavior": "succeed", "result": {"result": "pass"}}]);
    script["seats"]["implement:chief"] =
        json!([{"behavior": "succeed", "result": {"result": "complete"}}]);
    let ws = Workspace::new(script);
    ws.make_sequence(
        "implement",
        &[
            json!({"name": "positions", "members": ["econ", "legal"],
                   "aggregate": "unanimous-pass"}),
            json!({"name": "chief"}),
        ],
    );
    let (code, summary, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");

    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let live_members: std::collections::BTreeSet<&str> = events
        .iter()
        .filter(|e| {
            e["payload"]["checkpoint"]["step"] == "working"
                && e["payload"]["checkpoint"]["member"].is_string()
        })
        .map(|e| e["payload"]["checkpoint"]["member"].as_str().unwrap())
        .collect();
    assert_eq!(
        live_members,
        ["chief", "positions:econ", "positions:legal"]
            .into_iter()
            .collect(),
        "panel-step members tag as '<step>:<member>'"
    );
    let summary_members: Vec<&str> = events
        .iter()
        .filter(|e| e["payload"]["checkpoint"]["step"] == "panel-member-finished")
        .map(|e| e["payload"]["checkpoint"]["member"].as_str().unwrap())
        .collect();
    assert_eq!(summary_members, vec!["positions:econ", "positions:legal"]);
    let finished = events
        .iter()
        .find(|e| e["payload"]["checkpoint"]["step"] == "sequence-step-finished")
        .expect("panel step finished checkpoint");
    assert_eq!(
        finished["payload"]["checkpoint"]["result"]["result"], "pass",
        "the step's recorded result is the panel's aggregate"
    );
}

#[test]
fn compile_rejects_bad_sequences() {
    // One step.
    let ws = Workspace::new(happy_script());
    ws.make_sequence("implement", &[json!({"name": "only"})]);
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("at least two steps"), "stderr: {stderr}");

    // Duplicate step names, case-insensitive.
    let ws = Workspace::new(happy_script());
    ws.make_sequence(
        "implement",
        &[json!({"name": "chief"}), json!({"name": "Chief"})],
    );
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("duplicate step name"), "stderr: {stderr}");
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
    ws.brokkr(&[
        "export",
        "--run",
        &run_id,
        "--out",
        out.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    let journal = std::fs::read_to_string(out.join(format!("{run_id}.ndjson"))).unwrap();
    let review_decision = journal
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap())
        .find(|e| e["type"] == "transition/decided" && e["payload"]["from"] == "review")
        .expect("review decision in journal");
    let inputs = &review_decision["payload"]["inputs"];
    assert_eq!(inputs["fixes_applied"], false);
    assert!(
        inputs.get("high_risk_uncovered").is_none(),
        "inputs: {inputs}"
    );
    assert!(inputs.get("skip_verify").is_none(), "inputs: {inputs}");
}

#[test]
fn compile_rejects_provenance_violations() {
    let cases: [(Value, &str); 4] = [
        (json!(["consecutive_failures"]), "engine-owned"),
        // The phase-visit family is engine-owned too (decision 0022): the
        // journal counts visits, so a seat may never claim one.
        (json!(["visits_implement"]), "engine-owned"),
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
        let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
        assert_eq!(code, Some(1), "declaration {declaration} must be rejected");
        assert!(
            stderr.contains(expected),
            "declaration {declaration}: {stderr}"
        );
    }
}

/// Plain recursive copy for building variant bundles in tests.
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

#[test]
fn rerun_completes_under_variant_bundle_and_lists_both_runs() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let source = Workspace::run_id(&stderr);

    // A variant of the test bundle: same machine and seats, benignly
    // different role text — a different manifest, hence a real rerun
    // under a different delivery strategy.
    let variant = ws.path().join("bundle-variant");
    copy_dir(&ws.bundle_dir(), &variant);
    std::fs::write(variant.join("roles/role.md"), "# variant role\n").unwrap();

    let db = ws.db();
    let (code, summary, stderr) = ws.brokkr(&[
        "rerun",
        "--run",
        &source,
        "--bundle",
        variant.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["phase"], "done");
    assert_eq!(
        summary["feature"], "proof feature",
        "feature copied from the source run"
    );

    let prefix = format!("rerun of {source} as ");
    let line = stderr
        .lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap_or_else(|| panic!("rerun announcement on stderr: {stderr}"));
    let rerun_id = line[prefix.len()..]
        .split(' ')
        .next()
        .expect("new run id in announcement")
        .to_string();
    assert_ne!(rerun_id, source, "a rerun is a NEW run, never the source");
    assert!(
        line.ends_with(" under proof"),
        "bundle name in announcement: {line}"
    );

    // Both runs are independent journal entries.
    let (code, stdout, stderr) = ws.brokkr_raw(&["runs", "--json", "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let view: Value = serde_json::from_str(&stdout).unwrap();
    for run_id in [&source, &rerun_id] {
        let row = view["runs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["run_id"] == run_id.as_str())
            .unwrap_or_else(|| panic!("runs output has a row for {run_id}: {stdout}"));
        assert_eq!(row["feature"], "proof feature");
        assert_eq!(row["status"], "completed");
        assert_eq!(row["phase"], "done");
    }
}

#[test]
fn compare_aligns_two_runs_and_finds_the_review_divergence() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let run_a = Workspace::run_id(&stderr);

    // A variant bundle whose review returns residual low instead of
    // clean: its seats read a second script through a second state dir,
    // which both isolates the fake driver's attempt counters and makes
    // the manifest digest honestly differ (same_recipe: false).
    let variant = ws.path().join("bundle-variant");
    copy_dir(&ws.bundle_dir(), &variant);
    let mut script = happy_script();
    script["seats"]["review"] = json!([{"behavior": "succeed", "result": {
        "result": "residual",
        "inputs": {"max_residual_severity": "low", "has_security_residual": false}
    }}]);
    let script_path = ws.path().join("script-variant.json");
    std::fs::write(&script_path, serde_json::to_string(&script).unwrap()).unwrap();
    let state = ws.path().join("state-variant");
    std::fs::create_dir_all(&state).unwrap();
    let config_path = variant.join("bundle.json");
    let mut config: Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    for (_, seat) in config["seats"].as_object_mut().unwrap() {
        seat["driver"]["command"] = json!([
            brokkr_bin(),
            "fake-driver",
            "--script",
            script_path.to_string_lossy(),
            "--state",
            state.to_string_lossy(),
        ]);
    }
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

    let db = ws.db();
    let (code, summary, stderr) = ws.brokkr(&[
        "rerun",
        "--run",
        &run_a,
        "--bundle",
        variant.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    let prefix = format!("rerun of {run_a} as ");
    let run_b = stderr
        .lines()
        .find(|l| l.starts_with(&prefix))
        .unwrap_or_else(|| panic!("rerun announcement on stderr: {stderr}"))[prefix.len()..]
        .split(' ')
        .next()
        .expect("new run id in announcement")
        .to_string();

    let (code, report, stderr) =
        ws.brokkr(&["compare", &run_a, &run_b, "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0), "stderr: {stderr}");

    // Per-run sections: identity, fold summary, trail, numbers present.
    // Fake-driver checkpoints carry no cost, so assert presence/type only.
    for id in [&run_a, &run_b] {
        let run = &report["runs"][id.as_str()];
        assert_eq!(run["feature"], "proof feature");
        assert_eq!(run["bundle_name"], "proof");
        assert!(run["manifest"]["sha256"].is_string(), "run {id}: {run}");
        assert_eq!(run["status"], "completed");
        assert_eq!(run["phase"], "done");
        assert_eq!(run["park_reason"], Value::Null);
        assert!(run["total_cost_usd"].is_number(), "run {id}: {run}");
        assert!(run["events"].as_u64().unwrap() > 0);
        assert!(run["first_recorded_at"].is_string(), "run {id}: {run}");
        assert!(run["last_recorded_at"].is_string(), "run {id}: {run}");
        assert!(
            run["seats"]["review"]["attempts"].as_u64().unwrap() >= 1,
            "run {id}: {run}"
        );
        assert_eq!(
            run["phases_visited"]["ship"], 2,
            "ready then shipped: {run}"
        );
    }
    assert_eq!(
        report["runs"][run_a.as_str()]["decision_trail"],
        json!([
            "INTAKE-OK",
            "IMPL-OK",
            "VERIFY-PASS",
            "REVIEW-CLEAN-NO-FIXES",
            "SHIP-READY",
            "SHIP-COMPLETE"
        ])
    );
    assert_eq!(
        report["runs"][run_b.as_str()]["decision_trail"][3],
        "REVIEW-RESIDUAL-OK"
    );

    let cmp = &report["comparison"];
    assert_eq!(cmp["same_feature"], true);
    assert_eq!(cmp["same_recipe"], false, "variant manifest must differ");
    assert_eq!(cmp["status_pair"], json!(["completed", "completed"]));
    assert_eq!(cmp["first_divergence"]["index"], 3, "comparison: {cmp}");
    assert_eq!(cmp["first_divergence"]["a"], "REVIEW-CLEAN-NO-FIXES");
    assert_eq!(cmp["first_divergence"]["b"], "REVIEW-RESIDUAL-OK");
    assert!(cmp["cost_delta_usd"].is_number(), "comparison: {cmp}");
    assert!(cmp["attempts_delta"].is_number(), "comparison: {cmp}");

    // Either run missing is a clear exit-1 error naming the run.
    let (code, _, stderr) = ws.brokkr(&[
        "compare",
        "no-such-run",
        &run_b,
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("no-such-run"),
        "stderr names the missing run: {stderr}"
    );
}

#[test]
fn rerun_of_nonexistent_run_errors() {
    let ws = Workspace::new(happy_script());
    let bundle = ws.bundle_dir();
    let db = ws.db();
    let (code, _, stderr) = ws.brokkr(&[
        "rerun",
        "--run",
        "no-such-run",
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("no-such-run"),
        "stderr names the missing run: {stderr}"
    );
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
    let (code, _, stderr) = ws.brokkr(&[
        "resume",
        "--run",
        &run_id,
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(
        code,
        Some(1),
        "an active run never silently changes its bundle"
    );
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
    let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
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
    let (code, _, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(1));
    assert!(
        stderr.contains("constitutionally rejected"),
        "stderr: {stderr}"
    );
}

/// The one implement decision in a run's journal.
fn implement_decision(events: &[Value]) -> Value {
    events
        .iter()
        .find(|e| e["type"] == "transition/decided" && e["payload"]["from"] == "implement")
        .expect("implement decision in journal")
        .clone()
}

#[test]
fn missing_artifact_fails_closed_and_no_subsequent_seat_runs() {
    // AC-1: an advancing ruling naming an absent artifact parks with the
    // canonical evidence; the phase never advances, no seat attests.
    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    let (code, summary, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert_eq!(summary["status"], "awaiting_operator");
    assert_eq!(summary["phase"], "implement");
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.contains("requires_artifacts unmet for rule IMPL-OK: missing: spec.md"),
        "park reason: {reason}"
    );

    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let payload = implement_decision(&events)["payload"].clone();
    assert_eq!(
        payload["rule_id"], "IMPL-OK",
        "the rule DID match; a gate block is not NoRule"
    );
    assert_eq!(payload["result"], "complete");
    assert_eq!(payload["next"], Value::Null, "fails closed");
    assert_eq!(
        payload["severity"],
        Value::Null,
        "no transition taken, no severity"
    );
    assert_eq!(
        payload["problem"],
        "requires_artifacts unmet for rule IMPL-OK: missing: spec.md"
    );
    assert!(
        !events
            .iter()
            .any(|e| e["type"] == "phase/entered" && e["payload"]["phase"] == "verify"),
        "the phase never advances"
    );
}

#[test]
fn empty_and_directory_artifacts_park_with_their_classes() {
    // AC-2: presence is not enough — a zero-byte file and a directory
    // each fail closed under their own class.
    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    std::fs::write(ws.workdir().join("spec.md"), "").unwrap();
    let (code, summary, _) = ws.run_in_workdir();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.contains("requires_artifacts unmet for rule IMPL-OK: empty: spec.md"),
        "park reason: {reason}"
    );

    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    std::fs::create_dir(ws.workdir().join("spec.md")).unwrap();
    let (code, summary, _) = ws.run_in_workdir();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.contains("requires_artifacts unmet for rule IMPL-OK: not-a-file: spec.md"),
        "park reason: {reason}"
    );
}

#[test]
fn invalid_entries_fail_closed_without_resolving() {
    // AC-3: traversal and reserved-syntax entries are class `invalid` at
    // decide time — even when the traversal target exists, it never
    // resolves.
    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["../escape", "{slug}"]);
    std::fs::write(ws.path().join("escape"), "present outside the workdir").unwrap();
    let (code, summary, _) = ws.run_in_workdir();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.contains(
            "requires_artifacts unmet for rule IMPL-OK: invalid: ../escape; invalid: {slug}"
        ),
        "park reason: {reason}"
    );
}

#[test]
fn multiple_failures_produce_one_park_in_table_order() {
    // AC-4: the gate reports the COMPLETE failure list in declaration
    // order — the operator fixes one park, not three sequential ones.
    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md", "plan.md", "repos.yaml"]);
    std::fs::write(ws.workdir().join("plan.md"), "").unwrap();
    std::fs::create_dir(ws.workdir().join("repos.yaml")).unwrap();
    let (code, summary, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(2));
    let reason = summary["park_reason"].as_str().unwrap();
    assert!(
        reason.contains(
            "requires_artifacts unmet for rule IMPL-OK: \
             missing: spec.md; empty: plan.md; not-a-file: repos.yaml"
        ),
        "park reason: {reason}"
    );
    let events = ws.exported_events(&Workspace::run_id(&stderr));
    let parks = events
        .iter()
        .filter(|e| e["type"] == "transition/decided" && !e["payload"]["problem"].is_null())
        .count();
    assert_eq!(parks, 1, "one park carries all the evidence");
}

#[test]
fn artifacts_present_advance_is_byte_equivalent_to_ungated() {
    // AC-5: on the pass path the gate leaves no residue — the decided
    // payload matches an artifact-free rule's advance byte for byte.
    let gated = Workspace::new(happy_script());
    gated.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    std::fs::write(gated.workdir().join("spec.md"), "# spec\n").unwrap();
    let (code, summary, stderr) = gated.run_in_workdir();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    let gated_payload =
        implement_decision(&gated.exported_events(&Workspace::run_id(&stderr)))["payload"].clone();

    let plain = Workspace::new(happy_script());
    let (code, _, stderr) = plain.run_in_workdir();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let plain_payload =
        implement_decision(&plain.exported_events(&Workspace::run_id(&stderr)))["payload"].clone();

    assert_eq!(
        serde_json::to_string(&gated_payload).unwrap(),
        serde_json::to_string(&plain_payload).unwrap(),
        "the pass path leaves no residue"
    );
}

#[test]
fn gate_park_recovers_via_operator_retry_consuming_no_attempt_budget() {
    // AC-6: park -> retry -> the seat re-runs -> the gate re-probes ->
    // advance. max_attempts 1 proves the gate park consumed none of the
    // seat's budget (a gate park is not a seat failure, decision 0006).
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "succeed", "result": {"result": "complete"}},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    ws.set_seat_limits("implement", 1, 3600);
    let (code, summary, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert_eq!(summary["status"], "awaiting_operator");
    let run_id = Workspace::run_id(&stderr);

    std::fs::write(ws.workdir().join("spec.md"), "# spec\n").unwrap();
    let db = ws.db();
    let (code, _, _) = ws.brokkr(&[
        "operator",
        "retry",
        "--run",
        &run_id,
        "--reason",
        "artifacts supplied",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let (code, summary, stderr) = ws.resume_in_workdir(&run_id);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
}

#[test]
fn gate_decisions_replay_with_the_workdir_deleted() {
    // AC-7: the decided payload is the durable record of the gate's
    // observation; replay folds it and never touches the workdir.
    // The parked run.
    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    let (code, _, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(2));
    let run_id = Workspace::run_id(&stderr);
    std::fs::remove_dir_all(ws.workdir()).unwrap();
    let db = ws.db();
    let (code, replay, _) = ws.brokkr(&["replay", "--run", &run_id, "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(replay["replay"], "deterministic");

    // The advanced run, with export/verify stability per the
    // full_delivery_completes_exports_and_replays pattern.
    let ws = Workspace::new(happy_script());
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    std::fs::write(ws.workdir().join("spec.md"), "# spec\n").unwrap();
    let (code, summary, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(summary["status"], "completed");
    let run_id = Workspace::run_id(&stderr);
    std::fs::remove_dir_all(ws.workdir()).unwrap();
    let db = ws.db();
    let (code, replay, _) = ws.brokkr(&["replay", "--run", &run_id, "--db", db.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(replay["replay"], "deterministic");
    let out = ws.path().join("export");
    let (code, _, _) = ws.brokkr(&[
        "export",
        "--run",
        &run_id,
        "--out",
        out.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let journal = out.join(format!("{run_id}.ndjson"));
    let (code, verified, _) = ws.brokkr(&["verify-run", journal.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(verified["chain"], "verified");
    assert_eq!(verified["state"]["status"], "completed");
}

#[test]
fn gate_park_resets_consecutive_failures() {
    // AC-8, pinned: the blocked decision carries the seat's actual
    // result (`complete`, not a FAILURE_RESULT), so the fold resets the
    // phase's failure counter — the counter tracks seat failures, and
    // the seat succeeded; the gate blocked. A stale counter would make
    // the post-retry `broken` read 2 and hard-stop via IMPL-BROKEN-TWICE.
    let mut script = happy_script();
    script["seats"]["implement"] = json!([
        {"behavior": "succeed", "result": {"result": "broken"}},
        {"behavior": "succeed", "result": {"result": "complete"}},
        {"behavior": "succeed", "result": {"result": "broken"}},
        {"behavior": "succeed", "result": {"result": "complete"}},
    ]);
    let ws = Workspace::new(script);
    ws.set_rule_requires_artifacts("IMPL-OK", &["spec.md"]);
    let (code, summary, stderr) = ws.run_in_workdir();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    assert_eq!(summary["status"], "awaiting_operator");
    let run_id = Workspace::run_id(&stderr);

    std::fs::write(ws.workdir().join("spec.md"), "# spec\n").unwrap();
    let db = ws.db();
    let (code, _, _) = ws.brokkr(&[
        "operator",
        "retry",
        "--run",
        &run_id,
        "--reason",
        "artifacts supplied",
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0));
    let (code, summary, stderr) = ws.resume_in_workdir(&run_id);
    assert_eq!(
        code,
        Some(0),
        "a reset counter retries; a stale one hard-stops: {stderr}"
    );
    assert_eq!(summary["status"], "completed");

    let events = ws.exported_events(&run_id);
    let implement_rules: Vec<&Value> = events
        .iter()
        .filter(|e| e["type"] == "transition/decided" && e["payload"]["from"] == "implement")
        .map(|e| &e["payload"])
        .collect();
    let trail: Vec<&Value> = implement_rules.iter().map(|p| &p["rule_id"]).collect();
    assert_eq!(
        trail,
        vec![
            "IMPL-BROKEN-RETRY",
            "IMPL-OK",
            "IMPL-BROKEN-RETRY",
            "IMPL-OK"
        ],
        "never IMPL-BROKEN-TWICE"
    );
    assert_eq!(
        implement_rules[2]["inputs"]["consecutive_failures"], 1,
        "the gate park reset the counter before the post-retry broken"
    );
}

// ------------------------------------------------------------------
// Sealed secret bindings (decision 0012): the Brokkr secrets CLI, the
// layer-6 journal invariant, and the single-call-site grep gate.
// ------------------------------------------------------------------

/// Run Brokkr with a stdin payload (`brokkr secrets set` reads the value
/// from stdin, never argv).
fn brokkr_stdin(cwd: &Path, args: &[&str], payload: &str) -> (Option<i32>, String, String) {
    use std::io::Write;
    let mut child = Command::new(brokkr_bin())
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(payload.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn secrets_cli_round_trips_and_never_prints_values() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("sub/secrets.env");
    let store_arg = store.to_str().unwrap();
    let (code, _, stderr) = brokkr_stdin(
        dir.path(),
        &["secrets", "set", "GH_TOKEN", "--secrets-file", store_arg],
        "tokenvalue-alpha\n",
    );
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let (code, _, _) = brokkr_stdin(
        dir.path(),
        &["secrets", "set", "API_KEY", "--secrets-file", store_arg],
        "keyvalue-beta\n",
    );
    assert_eq!(code, Some(0));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&store).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "created 0600");
    }
    let (code, stdout, stderr) = brokkr_stdin(
        dir.path(),
        &["secrets", "list", "--secrets-file", store_arg],
        "",
    );
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(stdout, "API_KEY\nGH_TOKEN\n", "names only, sorted");
    for value in ["tokenvalue-alpha", "keyvalue-beta"] {
        assert!(
            !stdout.contains(value) && !stderr.contains(value),
            "list printed a value"
        );
    }
    let (code, _, _) = brokkr_stdin(
        dir.path(),
        &["secrets", "remove", "GH_TOKEN", "--secrets-file", store_arg],
        "",
    );
    assert_eq!(code, Some(0));
    let (code, _, stderr) = brokkr_stdin(
        dir.path(),
        &["secrets", "remove", "GH_TOKEN", "--secrets-file", store_arg],
        "",
    );
    assert_eq!(code, Some(1), "removing an absent name is an error");
    assert!(stderr.contains("GH_TOKEN"), "stderr: {stderr}");

    // Refusal classes at set: empty, multi-line, too short, denylisted
    // name, grammar-violating name.
    for (name, payload) in [
        ("EMPTY", "\n"),
        ("MULTI", "two\nlines\n"),
        ("SHORT", "abc\n"),
        ("PATH", "longenough\n"),
        ("FORGE_X", "longenough\n"),
        ("lower", "longenough\n"),
    ] {
        let (code, _, stderr) = brokkr_stdin(
            dir.path(),
            &["secrets", "set", name, "--secrets-file", store_arg],
            payload,
        );
        assert_eq!(code, Some(1), "set {name} must refuse: {stderr}");
    }
    // Short-but-legal warns and succeeds.
    let (code, _, stderr) = brokkr_stdin(
        dir.path(),
        &["secrets", "set", "WARN1", "--secrets-file", store_arg],
        "seven77\n",
    );
    assert_eq!(code, Some(0));
    assert!(stderr.contains("warning"), "stderr: {stderr}");
}

#[cfg(unix)]
mod journal_invariant {
    use super::*;

    const SECRET_VALUE: &str = "tok3n+v4lue!R7x";

    /// A child that leaks the bound value through every listed channel:
    /// every needle encoding on stdout AND stderr (from needles.txt in
    /// the workdir), plus the raw env value and its literal argv in the
    /// result notes. `exit_code` distinguishes the succeeding proof from
    /// the failing one (which exercises the stderr-tail journal path).
    fn leaky_script(exit_code: i32) -> String {
        format!(
            "#!/bin/sh\n\
             prompt=$(cat)\n\
             target=$(printf '%s\\n' \"$prompt\" | sed -n 's/^    \\(.*\\.json\\)$/\\1/p' | head -1)\n\
             cat needles.txt\n\
             cat needles.txt 1>&2\n\
             printf '{{\"result\": \"complete\", \"notes\": \"argv=%s env=%s leaks=%s\"}}' \\\n\
               \"$1\" \"$API_TOKEN\" \"$(tr '\\n' ' ' < needles.txt)\" > \"$target\"\n\
             exit {exit_code}\n"
        )
    }

    /// The adapter tree the workspace compiles against. A seat that
    /// declares secret bindings may only seat a driver the operator
    /// granted them (decision 0021 ruling 4), and this proof's seat is
    /// the exec driver — the one decision 0012 put the resolution in.
    /// The shipped adapters are copied verbatim rather than invented, so
    /// the proof runs against the same declarations the tree ships.
    fn stage_adapters(ws: &Workspace) {
        let shipped = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates/")
            .parent()
            .expect("workspace root")
            .join("adapters");
        copy_dir(&shipped, &ws.path().join("adapters"));
    }

    /// Swap one phase's fake-driver seat for a real exec seat with a
    /// declared secret binding and a {{secret:NAME}} template reference.
    fn make_exec_seat(ws: &Workspace, phase: &str, script: &Path) {
        let path = ws.bundle_dir().join("bundle.json");
        let mut config: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let results = config["seats"][phase]["results"].clone();
        config["seats"][phase] = json!({
            "role": "roles/role.md",
            "results": results,
            "secrets": ["API_TOKEN"],
            "driver": {"command": [
                brokkr_bin(), "driver", "exec", "--",
                script.to_string_lossy(), "{{secret:API_TOKEN}}",
            ]},
        });
        std::fs::write(&path, serde_json::to_string(&config).unwrap()).unwrap();
    }

    fn write_executable(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, body).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Stage the workspace: bind the secret, write every needle the
    /// shared constant produces into the workdir for the child to leak,
    /// and return (workspace, store path, template string).
    fn stage(exit_code: i32) -> (Workspace, PathBuf, String) {
        let ws = Workspace::new(happy_script());
        let script = ws.path().join("leaky.sh");
        write_executable(&script, &leaky_script(exit_code));
        make_exec_seat(&ws, "implement", &script);
        stage_adapters(&ws);
        let store = ws.path().join("secrets.env");
        brokkr_protocol::secret::store_set(&store, "API_TOKEN", SECRET_VALUE).unwrap();
        // The child prints the needles the SAME shared constant defines —
        // the proof and the masker cannot drift apart.
        let mut needles = Vec::new();
        for (_, encode) in brokkr_protocol::secret::NEEDLE_ENCODINGS {
            needles.extend_from_slice(&encode(SECRET_VALUE.as_bytes()));
            needles.push(b'\n');
        }
        std::fs::write(ws.workdir().join("needles.txt"), needles).unwrap();
        let template = format!("{} {}", script.display(), "{{secret:API_TOKEN}}");
        (ws, store, template)
    }

    fn run_with_secrets(ws: &Workspace, store: &Path) -> (Option<i32>, Value, String) {
        let bundle = ws.bundle_dir();
        let db = ws.db();
        let work = ws.workdir();
        ws.brokkr(&[
            "run",
            "--bundle",
            bundle.to_str().unwrap(),
            "--feature",
            "proof feature",
            "--db",
            db.to_str().unwrap(),
            "--repo",
            work.to_str().unwrap(),
            "--secrets-file",
            store.to_str().unwrap(),
        ])
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
    }

    /// Byte-scan the exported journal for the value in every listed
    /// encoding, iterating the shared needle constant. Zero hits or fail.
    fn assert_journal_sealed(ws: &Workspace, run_id: &str) -> Vec<u8> {
        let db = ws.db();
        let out = ws.path().join("export");
        ws.brokkr(&[
            "export",
            "--run",
            run_id,
            "--out",
            out.to_str().unwrap(),
            "--db",
            db.to_str().unwrap(),
        ]);
        let journal = std::fs::read(out.join(format!("{run_id}.ndjson"))).unwrap();
        for (label, encode) in brokkr_protocol::secret::NEEDLE_ENCODINGS {
            let needle = encode(SECRET_VALUE.as_bytes());
            assert!(
                !contains(&journal, &needle),
                "journal leaks the bound value as {label}"
            );
        }
        journal
    }

    #[test]
    fn succeeding_leaky_child_puts_nothing_in_the_journal() {
        let (ws, store, template) = stage(0);
        let (code, summary, stderr) = run_with_secrets(&ws, &store);
        assert_eq!(code, Some(0), "stderr: {stderr}");
        assert_eq!(summary["status"], "completed");
        let run_id = Workspace::run_id(&stderr);
        let journal = assert_journal_sealed(&ws, &run_id);
        let text = String::from_utf8_lossy(&journal);

        // The proof must not be vacuous: the child DID leak, and the
        // leak reached the journal masked (EffectSucceeded result notes).
        assert!(text.contains("[secret:API_TOKEN]"), "masking engaged");
        assert!(
            text.contains("argv=$API_TOKEN"),
            "argv carried the env reference, never the value"
        );

        // The seat-record contract admits no command prose. The pinned
        // bundle still carries the template; neither unresolved nor
        // resolved command text appears in a checkpoint.
        let events: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap())
            .collect();
        let started = events
            .iter()
            .find(|e| e["payload"]["checkpoint"]["step"] == "exec-started")
            .expect("exec-started checkpoint in journal");
        assert!(started["payload"]["checkpoint"].get("target").is_none());
        let resolved = template.replace("{{secret:API_TOKEN}}", "$API_TOKEN");
        assert!(
            !contains(&journal, resolved.as_bytes()),
            "the resolved command line is never journaled"
        );
    }

    #[test]
    fn failing_leaky_child_keeps_the_stderr_tail_sealed() {
        let (ws, store, _) = stage(3);
        let (code, summary, stderr) = run_with_secrets(&ws, &store);
        assert_eq!(code, Some(2), "stderr: {stderr}");
        assert_eq!(summary["status"], "awaiting_operator");
        let reason = summary["park_reason"].as_str().unwrap();
        assert!(reason.contains("exited 3"), "park reason: {reason}");
        // The stderr tail rode the failure into the journal — masked.
        assert!(
            reason.contains("[secret:API_TOKEN]"),
            "the masked stderr tail is the evidence: {reason}"
        );
        let run_id = Workspace::run_id(&stderr);
        assert_journal_sealed(&ws, &run_id);
    }
}

#[test]
fn expose_for_spawn_has_exactly_one_production_call_site() {
    // Layer 4's single-egress property, enforced by grep as decision
    // 0012 prescribes: the plaintext accessor is CALLED exactly once
    // outside secret.rs — the exec adapter's spawn injector.
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let needle: String = ["expose_for", "_spawn("].concat();
    let mut call_sites: Vec<(PathBuf, usize)> = Vec::new();
    let mut stack = vec![crates_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
                // Production code only: each crate's src/ tree, minus
                // the trust-boundary module itself.
                && path.components().any(|c| c.as_os_str() == "src")
                && path.file_name().and_then(|n| n.to_str()) != Some("secret.rs")
                && !path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n == "tests.rs" || n.ends_with("_tests.rs"))
            {
                let content = std::fs::read_to_string(&path).unwrap();
                let count = content.matches(&needle).count();
                if count > 0 {
                    call_sites.push((path, count));
                }
            }
        }
    }
    assert_eq!(
        call_sites.iter().map(|(_, n)| n).sum::<usize>(),
        1,
        "exactly one call site outside secret.rs: {call_sites:?}"
    );
    assert!(
        call_sites[0]
            .0
            .ends_with(Path::new("brokkr-protocol").join("src").join("adapters.rs")),
        "the one call site is the exec spawn injector: {call_sites:?}"
    );
}

#[test]
fn inspect_and_watch_read_the_run_from_the_one_derivation() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);
    let path = ws.db();
    let db = path.to_str().unwrap();

    // --json emits the view model verbatim, and `.summary` is today's
    // `brokkr inspect` output: all nine keys, `cursor` included.
    let (code, view, stderr) = ws.brokkr(&["inspect", "--run", &run_id, "--json", "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    // Decision 0016 moved the wire version to 2: participants gained
    // `provenance`, the run view gained `notices`. It moved to 3 when
    // the phase rail gained `returns`, to 4 for decision 0031's
    // provider-reported served-model cells, and to 5 for decision 0032's
    // common transcript shape. The constant is the one source of truth.
    assert_eq!(view["view_version"], brokkr_view::VIEW_VERSION);
    let summary = &view["summary"];
    for key in [
        "run_id",
        "seq",
        "status",
        "phase",
        "cursor",
        "park_reason",
        "consecutive_failures",
        "last_decision",
        "feature",
    ] {
        assert!(summary.get(key).is_some(), "summarize() key {key}");
    }
    assert_eq!(summary["status"], "completed");
    assert_eq!(summary["run_id"], run_id.as_str());

    // The human readout: header, ruling, seats, trail, and the tree.
    let (code, human, stderr) = ws.brokkr_raw(&["inspect", "--run", &run_id, "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(human.starts_with(&format!("run  {run_id}\n")), "{human}");
    for section in ["ruling  ", "seats", "trail", "graph", "→ "] {
        assert!(human.contains(section), "{section} missing from:\n{human}");
    }
    assert!(!human.contains('\x1b'), "no ANSI down a pipe");

    // Scoping mirrors the console's clicks, and the two are exclusive.
    let (code, scoped, stderr) =
        ws.brokkr_raw(&["inspect", "--run", &run_id, "--phase", "intake", "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(scoped.contains("  intake ×"), "{scoped}");
    assert!(!scoped.contains("  verify ×"), "scoped out: {scoped}");
    let (code, seated, stderr) =
        ws.brokkr_raw(&["inspect", "--run", &run_id, "--seat", "verify", "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(seated.contains("  verify ×"), "{seated}");
    let (code, _, stderr) = ws.brokkr_raw(&[
        "inspect", "--run", &run_id, "--phase", "intake", "--seat", "intake", "--db", db,
    ]);
    assert_ne!(code, Some(0), "the two scopes are mutually exclusive");
    assert!(stderr.contains("cannot be used with"), "{stderr}");

    // A value matching nothing exits nonzero naming the valid ones: an
    // empty table would read as "this phase did nothing".
    let (code, _, stderr) = ws.brokkr_raw(&[
        "inspect", "--run", &run_id, "--phase", "nowhere", "--db", db,
    ]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("visited phases:"), "{stderr}");
    let (code, _, stderr) =
        ws.brokkr_raw(&["inspect", "--run", &run_id, "--seat", "nobody", "--db", db]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("participants:"), "{stderr}");

    // watch --once: one frame, no trail, timestamped because stdout is
    // a pipe, and the journal is byte-identical afterwards.
    let before = ws.brokkr_raw(&["export", "--run", &run_id, "--out", "before", "--db", db]);
    assert_eq!(before.0, Some(0), "stderr: {}", before.2);
    let (code, frame, stderr) = ws.brokkr_raw(&["watch", "--run", &run_id, "--once", "--db", db]);
    assert_eq!(code, Some(0), "a completed run exits 0: {stderr}");
    assert!(frame.starts_with("── "), "appended, timestamped: {frame}");
    assert!(!frame.contains('\x1b'), "no ANSI down a pipe: {frame:?}");
    assert!(
        frame.contains("seats") && frame.contains("graph"),
        "{frame}"
    );
    assert!(!frame.contains("\ntrail\n"), "a frame carries no trail");
    let after = ws.brokkr_raw(&["export", "--run", &run_id, "--out", "after", "--db", db]);
    assert_eq!(after.0, Some(0), "stderr: {}", after.2);
    assert_eq!(
        std::fs::read_to_string(ws.path().join(format!("before/{run_id}.ndjson"))).unwrap(),
        std::fs::read_to_string(ws.path().join(format!("after/{run_id}.ndjson"))).unwrap(),
        "watch never writes the journal"
    );

    // Garbage in --interval is rejected rather than defaulted silently.
    let (code, _, stderr) = ws.brokkr_raw(&[
        "watch",
        "--run",
        &run_id,
        "--once",
        "--interval",
        "soon",
        "--db",
        db,
    ]);
    assert_ne!(code, Some(0));
    assert!(stderr.contains("invalid value"), "{stderr}");
}

#[test]
fn watch_exits_on_a_park_with_the_reason_first() {
    let mut script = happy_script();
    script["seats"]["implement"] = json!([{"behavior": "vanish"}]);
    let ws = Workspace::new(script);
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(2), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);
    let path = ws.db();
    let db = path.to_str().unwrap();

    // A park admits no further events until a human acts, so "keep
    // watching" would be an unbounded CI hang.
    let (code, frame, stderr) = ws.brokkr_raw(&["watch", "--run", &run_id, "--db", db]);
    assert_eq!(code, Some(2), "parked is finish()'s exit 2: {stderr}");
    let park = frame
        .lines()
        .find(|line| line.starts_with("park  "))
        .unwrap_or_else(|| panic!("the park reason leads the frame: {frame}"));
    assert!(park.len() > "park  ".len(), "{park}");
    let seats = frame.lines().position(|line| line == "seats");
    let parked = frame.lines().position(|line| line.starts_with("park  "));
    assert!(parked < seats, "the reason comes before the seats: {frame}");
}

#[test]
fn a_run_id_prefix_and_latest_read_the_same_run_as_the_full_id() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);
    let path = ws.db();
    let db = path.to_str().unwrap();
    let prefix = &run_id[..run_id.len() - 4];

    // The readout is the run's, not the selector's: a prefix and
    // `latest` print what the 41-character id prints, byte for byte.
    let (code, full, stderr) = ws.brokkr_raw(&["inspect", "--run", &run_id, "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    for selector in [prefix, "latest"] {
        let (code, readout, stderr) = ws.brokkr_raw(&["inspect", "--run", selector, "--db", db]);
        assert_eq!(code, Some(0), "stderr: {stderr}");
        assert_eq!(readout, full, "--run {selector} is the same readout");
        let (code, view, stderr) = ws.brokkr(&["inspect", "--run", selector, "--json", "--db", db]);
        assert_eq!(code, Some(0), "stderr: {stderr}");
        assert_eq!(view["summary"]["run_id"], run_id.as_str());
    }

    // Every readout takes them, and none of them writes: replay,
    // export, watch and anchor all resolve through the one helper.
    let (code, _, stderr) = ws.brokkr(&["replay", "--run", prefix, "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let (code, _, stderr) =
        ws.brokkr_raw(&["export", "--run", "latest", "--out", "out", "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(
        ws.path().join(format!("out/{run_id}.ndjson")).exists(),
        "the export is named for the resolved run"
    );
    let (code, frame, stderr) = ws.brokkr_raw(&["watch", "--run", prefix, "--once", "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert!(frame.contains(&run_id), "{frame}");

    // A second run of the same feature shares the source's slug, so the
    // shared prefix is ambiguous and says which runs it matched.
    let variant = ws.path().join("bundle-variant");
    copy_dir(&ws.bundle_dir(), &variant);
    std::fs::write(variant.join("roles/role.md"), "# variant role\n").unwrap();
    let (code, _, stderr) = ws.brokkr(&[
        "rerun",
        "--run",
        &run_id,
        "--bundle",
        variant.to_str().unwrap(),
        "--db",
        db,
    ]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let announcement = format!("rerun of {run_id} as ");
    let rerun_id = stderr
        .lines()
        .find_map(|line| line.strip_prefix(&announcement))
        .and_then(|rest| rest.split(' ').next())
        .expect("new run id in announcement")
        .to_string();

    // The slug is everything before the id's trailing hash — the part
    // two runs of one feature share.
    let slug = run_id.rsplit_once('-').expect("slug-hash run id").0;
    let (code, _, stderr) = ws.brokkr_raw(&["inspect", "--run", slug, "--db", db]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("matches 2 runs"), "{stderr}");
    assert!(stderr.contains(&run_id), "{stderr}");
    assert!(stderr.contains(&rerun_id), "{stderr}");

    // `latest` follows the database, not the operator's memory.
    let (code, view, stderr) = ws.brokkr(&["inspect", "--run", "latest", "--json", "--db", db]);
    assert_eq!(code, Some(0), "stderr: {stderr}");
    assert_eq!(view["summary"]["run_id"], rerun_id.as_str());

    // A selector matching nothing is an error, never a nearest guess.
    let (code, _, stderr) = ws.brokkr_raw(&["inspect", "--run", "nobody", "--db", db]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("no run matching 'nobody'"), "{stderr}");

    // The help says so on every command that takes a selector.
    for command in ["watch", "inspect", "anchor", "export", "replay"] {
        let (code, help, stderr) = ws.brokkr_raw(&[command, "--help"]);
        assert_eq!(code, Some(0), "stderr: {stderr}");
        assert!(
            help.contains("unique run-id prefix, or `latest`"),
            "{command} --help: {help}"
        );
    }
}

/// A directory as bytes: relative path, length, content. `brokkr tui`'s
/// read-only claim is "the operator's disk looks the same afterwards",
/// and an NDJSON export alone would pass cleanly through a created
/// database, a WAL and a migration — so the tree is compared too.
fn tree(dir: &Path) -> Vec<(String, usize, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            for (child, len, bytes) in tree(&path) {
                out.push((format!("{name}/{child}"), len, bytes));
            }
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        out.push((name, bytes.len(), bytes));
    }
    out.sort();
    out
}

/// `brokkr tui` is listed among the read verbs, refuses before it can
/// touch anything, and leaves the workspace byte-identical.
#[test]
fn brokkr_tui_is_a_listed_read_verb_that_refuses_a_pipe_and_writes_nothing() {
    let ws = Workspace::new(happy_script());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "stderr: {stderr}");
    let run_id = Workspace::run_id(&stderr);
    let path = ws.db();
    let db = path.to_str().unwrap();

    let (code, help, _) = ws.brokkr_raw(&["--help"]);
    assert_eq!(code, Some(0));
    assert!(help.contains("tui"), "the verb is listed: {help}");
    let (code, verb_help, _) = ws.brokkr_raw(&["tui", "--help"]);
    assert_eq!(code, Some(0));
    assert!(verb_help.contains("--run"), "{verb_help}");
    assert!(verb_help.contains("Read-only"), "{verb_help}");

    // A database that does not exist is a refusal, not an initialized
    // empty store: `Store::open` creates a file, a WAL and a meta row,
    // so the gate stands in front of it.
    let missing = ws.path().join("nowhere.db");
    let (code, _, stderr) = ws.brokkr_raw(&["tui", "--db", missing.to_str().unwrap()]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("brokkr inspect"), "{stderr}");
    assert!(stderr.contains("brokkr watch"), "{stderr}");
    for suffix in ["", "-wal", "-shm"] {
        let candidate = ws.path().join(format!("nowhere.db{suffix}"));
        assert!(!candidate.exists(), "a read created {candidate:?}");
    }

    // And with a real database: a subprocess's stdout is a pipe, so the
    // not-a-tty refusal fires deterministically here and in CI.
    let before_events = ws.exported_events(&run_id);
    let before_tree = tree(ws.path());
    let (code, _, stderr) = ws.brokkr_raw(&["tui", "--run", &run_id[..8], "--db", db]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("needs a terminal"), "{stderr}");
    assert!(stderr.contains("brokkr inspect") && stderr.contains("brokkr watch"));
    assert_eq!(
        tree(ws.path()),
        before_tree,
        "the whole workspace is untouched"
    );
    assert_eq!(
        ws.exported_events(&run_id),
        before_events,
        "and the journal"
    );
}
