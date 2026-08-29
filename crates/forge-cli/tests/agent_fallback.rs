//! Bounded fallback and per-invocation provenance, driven end to end
//! (decision 0016; spec AC-6, AC-7, AC-14, AC-15, AC-16).
//!
//! Fallback is deliberately narrow. An attempt that FAILS TO START —
//! structurally: `Failed`, never `Accepted`, no checkpoint — retries on
//! the next model in the chain inside decision 0006's existing attempt
//! bounds. An attempt that accepted and then failed does NOT fall back:
//! it produced work and context a different model does not inherit, so
//! it follows 0006 unchanged. That boundary is enforced by the predicate
//! rather than described in a comment, and both sides of it are run here.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn forge_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forge")
}

const POLICY: &str = r#"{
  "phases": ["implement", "review", "done", "stop"],
  "initial": "implement",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "review",
     "reason": "Implementation complete."},
    {"id": "IMPL-PASS", "from": "implement", "result": "pass", "next": "review",
     "reason": "The panel agreed."},
    {"id": "IMPL-FAIL", "from": "implement", "result": "fail", "next": "stop",
     "severity": "hard", "reason": "The panel did not agree."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "Clean review; done."}
  ]
}"#;

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        for sub in ["bundle", "agents/charters", "adapters", "state"] {
            std::fs::create_dir_all(ws.path().join(sub)).unwrap();
        }
        std::fs::write(ws.path().join("bundle/policy.json"), POLICY).unwrap();
        std::fs::write(ws.path().join("agents/charters/work.md"), "# work\n").unwrap();
        // A provider whose binary does not exist: every attempt on it
        // fails to spawn, which satisfies the structural predicate
        // trivially — nothing accepted, nothing checkpointed.
        ws.write(
            "adapters/absent.json",
            json!({
                "provider": "absent",
                "binary": "forge-absent-driver",
                "driver": ["forge-absent-driver-that-is-not-installed"],
                "models": {"first": "absent/first"},
                "model_flag": "--model",
                "tool_permissions": "unsupported",
                "mcp": "unsupported",
            }),
        );
        ws.write("adapters/fake.json", ws.fake_adapter("fake", "second"));
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn db(&self) -> PathBuf {
        self.path().join("forge.db")
    }

    fn write(&self, relative: &str, body: Value) {
        std::fs::write(
            self.path().join(relative),
            serde_json::to_vec_pretty(&body).unwrap(),
        )
        .unwrap();
    }

    /// An adapter backed by the scripted fake driver, serving one
    /// abstract model name. Adding a provider is a file — there is no
    /// Rust edit anywhere in this test's diff that teaches the resolver
    /// about `fake`.
    fn fake_adapter(&self, provider: &str, model: &str) -> Value {
        json!({
            "provider": provider,
            "binary": forge_bin(),
            "driver": [
                forge_bin(), "fake-driver",
                "--script", self.path().join("script.json").to_string_lossy(),
                "--state", self.path().join("state").to_string_lossy(),
            ],
            "models": {model: format!("{provider}/{model}")},
            "model_flag": "--model",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        })
    }

    fn script(&self, body: Value) {
        self.write("script.json", body);
    }

    fn agent(&self, name: &str, models: Value, limits: Option<Value>) {
        let mut body = json!({
            "description": "a test agent",
            "charter": "charters/work.md",
            "models": models,
        });
        if let Some(limits) = limits {
            body["limits"] = limits;
        }
        self.write(&format!("agents/{name}.json"), body);
    }

    fn bundle(&self, seats: Value) {
        self.write(
            "bundle/bundle.json",
            json!({"name": "fallback", "policy": "policy.json", "seats": seats}),
        );
    }

    /// The inline review seat every fixture shares: it references no
    /// agent, so its journal gains no provenance field at all.
    fn inline_review(&self) -> Value {
        json!({
            "results": ["clean"],
            "role": "../agents/charters/work.md",
            "driver": {"command": [
                forge_bin(), "fake-driver",
                "--script", self.path().join("script.json").to_string_lossy(),
                "--state", self.path().join("state").to_string_lossy(),
            ]},
        })
    }

    fn forge(&self, args: &[&str]) -> (String, String) {
        let out = Command::new(forge_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn run(&self) -> String {
        let (_, stderr) = self.forge(&[
            "run",
            "--bundle",
            self.path().join("bundle").to_str().unwrap(),
            "--feature",
            "fallback",
            "--db",
            self.db().to_str().unwrap(),
        ]);
        stderr
            .lines()
            .find_map(|line| line.strip_prefix("run started: "))
            .unwrap_or_else(|| panic!("run id on stderr: {stderr}"))
            .trim()
            .to_string()
    }

    /// The journal, as the store holds it — read by a SECOND process, so
    /// every assertion below is about journaled facts rather than about
    /// anything the running engine kept in memory.
    fn events(&self, run_id: &str) -> Vec<Value> {
        self.forge(&[
            "export",
            "--run",
            run_id,
            "--out",
            self.path().to_str().unwrap(),
            "--db",
            self.db().to_str().unwrap(),
        ]);
        std::fs::read_to_string(self.path().join(format!("{run_id}.ndjson")))
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

fn of_type<'a>(events: &'a [Value], kind: &str) -> Vec<&'a Value> {
    events
        .iter()
        .filter(|event| event["type"] == kind)
        .collect()
}

/// AC-6: the first candidate's binary is absent, so the attempt fails to
/// start; the switch is journaled as a fact and the NEXT model in the
/// chain runs, all inside the seat's `max_attempts`.
#[test]
fn a_fail_to_start_falls_back_to_the_next_model_within_the_bound() {
    let ws = Workspace::new();
    ws.script(json!({"seats": {
        "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
    }}));
    ws.agent(
        "worker",
        json!(["first", "second"]),
        Some(json!({"max_attempts": 2, "timeout_seconds": 60})),
    );
    ws.bundle(json!({
        "implement": {"results": ["complete"], "agent": "worker"},
        "review": ws.inline_review(),
    }));
    let run_id = ws.run();
    let events = ws.events(&run_id);

    let started = of_type(&events, "effect/started");
    let implement: Vec<&&Value> = started
        .iter()
        .filter(|event| event["payload"]["provenance"].is_array())
        .collect();
    assert_eq!(implement.len(), 2, "two attempts, both agent-resolved");
    assert_eq!(implement[0]["payload"]["provenance"][0]["chain_index"], 0);
    assert_eq!(implement[0]["payload"]["provenance"][0]["model"], "first");
    assert_eq!(
        implement[0]["payload"]["provenance"][0]["provider"],
        "absent"
    );
    assert_eq!(implement[0]["payload"]["provenance"][0]["agent"], "worker");
    assert!(implement[0]["payload"]["provenance"][0]["member"].is_null());
    // The switch is journaled as a fact, then the next link runs.
    assert_eq!(implement[1]["payload"]["provenance"][0]["chain_index"], 1);
    assert_eq!(implement[1]["payload"]["provenance"][0]["model"], "second");
    assert_eq!(implement[1]["payload"]["provenance"][0]["provider"], "fake");

    let failed = of_type(&events, "effect/failed");
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["payload"]["start_failure"], json!(true));
    assert_eq!(failed[0]["payload"]["start_failure_sites"], json!([null]));

    // The whole sequence stayed inside max_attempts: the run reached
    // review rather than parking.
    assert_eq!(of_type(&events, "run/completed").len(), 1);
    // The pinned concrete id reached the driver, not just the argv.
    let pinned: Vec<&Value> = of_type(&events, "effect/checkpointed")
        .into_iter()
        .filter(|event| event["payload"]["checkpoint"]["step"] == "model-pinned")
        .collect();
    assert_eq!(pinned[0]["payload"]["checkpoint"]["model"], "fake/second");

    // The inline review seat's events carry no provenance at all.
    let review_started: Vec<&&Value> = started
        .iter()
        .filter(|event| event["payload"]["provenance"].is_null())
        .collect();
    assert_eq!(review_started.len(), 1);
}

/// AC-6's second half: exhausting the bound parks with the last error
/// rather than walking the chain forever.
#[test]
fn exhausting_the_bound_parks_with_the_last_error() {
    let ws = Workspace::new();
    ws.script(json!({"seats": {}}));
    ws.write("adapters/absent2.json", {
        let mut second = json!({
            "provider": "absent2",
            "binary": "forge-absent-driver",
            "driver": ["forge-absent-driver-that-is-not-installed-either"],
            "models": {"third": "absent2/third"},
            "model_flag": "--model",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        });
        second["provider"] = json!("absent2");
        second
    });
    ws.agent(
        "worker",
        json!(["first", "third"]),
        Some(json!({"max_attempts": 2, "timeout_seconds": 60})),
    );
    ws.bundle(json!({
        "implement": {"results": ["complete"], "agent": "worker"},
        "review": ws.inline_review(),
    }));
    let run_id = ws.run();
    let events = ws.events(&run_id);
    assert_eq!(of_type(&events, "effect/started").len(), 2);
    assert_eq!(of_type(&events, "effect/failed").len(), 2);
    let parked = of_type(&events, "run/parked");
    assert_eq!(parked.len(), 1);
    let reason = parked[0]["payload"]["reason"].as_str().unwrap();
    assert!(reason.contains("failed 2 of 2 attempt(s)"), "{reason}");
    assert!(reason.contains("did not spawn"), "{reason}");
}

/// AC-7 and AC-14: a driver that ACCEPTS and then fails has produced
/// work and context a different model does not inherit. It retries on
/// the SAME candidate — fallback is unreachable once `accepted` arrives,
/// by construction rather than by convention.
#[test]
fn a_mid_session_failure_does_not_fall_back() {
    let ws = Workspace::new();
    ws.write("adapters/fake.json", ws.fake_adapter("fake", "second"));
    ws.write("adapters/other.json", ws.fake_adapter("other", "third"));
    ws.script(json!({"seats": {
        "implement": [
            {"behavior": "fail", "error": "quota wall at turn forty"},
            {"behavior": "succeed", "result": {"result": "complete"}},
        ],
        "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
    }}));
    ws.agent(
        "worker",
        json!(["second", "third"]),
        Some(json!({"max_attempts": 2, "timeout_seconds": 60})),
    );
    ws.bundle(json!({
        "implement": {"results": ["complete"], "agent": "worker"},
        "review": ws.inline_review(),
    }));
    let run_id = ws.run();
    let events = ws.events(&run_id);

    let attempts: Vec<&Value> = of_type(&events, "effect/started")
        .into_iter()
        .filter(|event| event["payload"]["provenance"].is_array())
        .collect();
    assert_eq!(attempts.len(), 2);
    for attempt in &attempts {
        assert_eq!(
            attempt["payload"]["provenance"][0]["chain_index"], 0,
            "a mid-session failure never advances the chain"
        );
        assert_eq!(attempt["payload"]["provenance"][0]["model"], "second");
    }
    let failed = of_type(&events, "effect/failed");
    assert_eq!(failed.len(), 1);
    assert!(
        failed[0]["payload"].get("start_failure").is_none(),
        "an accepted-then-failed attempt is not a fail-to-start"
    );
    assert_eq!(of_type(&events, "run/completed").len(), 1);
}

/// AC-16: provenance is per driver INVOCATION, not per attempt. A panel
/// of two members on two providers produces two records inside one
/// attempt, and each site walks its own chain index.
#[test]
fn a_two_provider_panel_produces_two_records_in_one_attempt() {
    let ws = Workspace::new();
    ws.write("adapters/fake.json", ws.fake_adapter("fake", "second"));
    ws.write("adapters/other.json", ws.fake_adapter("other", "third"));
    ws.script(json!({"seats": {
        "implement:a": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "implement:b": [{"behavior": "succeed", "result": {"result": "pass"}}],
        "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
    }}));
    ws.agent("left", json!(["second"]), None);
    ws.agent("right", json!(["third"]), None);
    ws.bundle(json!({
        "implement": {
            "results": ["complete", "pass", "fail"],
            "aggregate": "unanimous-pass",
            "panel": {"a": {"agent": "left"}, "b": {"agent": "right"}},
        },
        "review": ws.inline_review(),
    }));
    let run_id = ws.run();
    let events = ws.events(&run_id);
    let started = of_type(&events, "effect/started");
    let provenance = started
        .iter()
        .find_map(|event| event["payload"]["provenance"].as_array())
        .expect("the panel attempt carries provenance");
    assert_eq!(
        provenance.len(),
        2,
        "one record per invocation, not per attempt"
    );
    assert_eq!(provenance[0]["member"], "a");
    assert_eq!(provenance[0]["agent"], "left");
    assert_eq!(provenance[0]["provider"], "fake");
    assert_eq!(provenance[1]["member"], "b");
    assert_eq!(provenance[1]["agent"], "right");
    assert_eq!(provenance[1]["provider"], "other");
}

/// AC-16's sequence half plus the proof that the library is an OPTION:
/// one seat spans an agent-resolved step and an inline `exec` step, and
/// the readout reports them separately rather than as one label.
#[test]
fn a_sequence_reports_its_agent_step_and_its_inline_step_separately() {
    let ws = Workspace::new();
    ws.script(json!({"seats": {
        "implement:think": [{"behavior": "succeed", "result": {"result": "ok"}}],
        "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
    }}));
    ws.agent("thinker", json!(["second"]), None);
    std::fs::write(
        ws.path().join("bundle/check.sh"),
        "#!/bin/sh\nprintf '%s' '{\"result\":\"complete\"}' > \"$1\"\n",
    )
    .unwrap();
    ws.bundle(json!({
        "implement": {
            "results": ["complete"],
            "limits": {"max_attempts": 1, "timeout_seconds": 60},
            "sequence": [
                {"name": "think", "agent": "thinker"},
                {"name": "check", "role": "../agents/charters/work.md",
                 "driver": {"command": [
                     forge_bin(), "driver", "exec", "--",
                     "sh", "./check.sh", "{result_path}",
                 ]}},
            ],
        },
        "review": ws.inline_review(),
    }));
    let run_id = ws.run();
    let events = ws.events(&run_id);
    let provenance = of_type(&events, "effect/started")
        .iter()
        .find_map(|event| event["payload"]["provenance"].as_array().cloned())
        .expect("the sequence attempt carries provenance");
    // Exactly one record: the inline `exec` step contributes none,
    // because it names no agent and the forge does not invent one for it.
    assert_eq!(provenance.len(), 1);
    assert_eq!(provenance[0]["member"], "think");
    assert_eq!(provenance[0]["agent"], "thinker");
}
