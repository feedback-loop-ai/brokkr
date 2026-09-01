//! The journal witness of the agent-library slice (decision 0016, spec
//! AC-13), pinned BEFORE any production edit.
//!
//! A run over a bundle that references no agent must produce exactly the
//! events it produced before this slice existed, field for field. The
//! golden below is the whole event sequence as `(type, sorted payload
//! keys)`: the two volatile parts of an event — uuids and timestamps —
//! are excluded by construction, and every other payload field is
//! compared. A stray `provenance` or `start_failure` on a non-adopting
//! run fails here, loudly, naming the event.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn forge_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["implement", "review", "done", "stop"],
  "initial": "implement",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "review",
     "reason": "Implementation complete."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "Clean review; done."}
  ]
}"#;

/// The event sequence a two-phase run produces: type, then the payload's
/// keys in sorted order. Recorded from this tree before the agent
/// library existed.
const GOLDEN: [(&str, &[&str]); 11] = [
    ("run/started", &["feature", "manifest"]),
    ("phase/entered", &["phase"]),
    (
        "effect/requested",
        &[
            "effect_id",
            "idempotency_key",
            "input_digest",
            "phase",
            "seat",
        ],
    ),
    ("effect/started", &["attempt_id", "driver", "effect_id"]),
    (
        "effect/checkpointed",
        &["attempt_id", "checkpoint", "effect_id"],
    ),
    ("effect/succeeded", &["attempt_id", "effect_id", "result"]),
    (
        "transition/decided",
        &[
            "from", "inputs", "next", "problem", "result", "rule_id", "severity",
        ],
    ),
    ("phase/entered", &["phase"]),
    (
        "effect/requested",
        &[
            "effect_id",
            "idempotency_key",
            "input_digest",
            "phase",
            "seat",
        ],
    ),
    ("effect/started", &["attempt_id", "driver", "effect_id"]),
    (
        "effect/checkpointed",
        &["attempt_id", "checkpoint", "effect_id"],
    ),
];

/// Fields this slice may add to an effect event. A non-adopting run must
/// carry none of them, anywhere, at any depth of the payload. Decision
/// 0023's `realms` and `realm_facts` join the list on the same terms: a
/// run in a workspace with no map gains neither.
const ADDED_FIELDS: [&str; 5] = [
    "provenance",
    "start_failure",
    "agents",
    "realms",
    "realm_facts",
];

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    fn new() -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.dir.path().join("bundle");
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::create_dir_all(ws.dir.path().join("state")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
        let script = ws.dir.path().join("script.json");
        std::fs::write(
            &script,
            serde_json::to_string(&json!({"seats": {
                "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
                "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
            }}))
            .unwrap(),
        )
        .unwrap();
        let seat = |results: Vec<&str>| {
            json!({
                "role": "roles/role.md",
                "results": results,
                "driver": {"command": [
                    forge_bin(), "fake-driver",
                    "--script", script.to_string_lossy(),
                    "--state", ws.dir.path().join("state").to_string_lossy(),
                ]},
            })
        };
        std::fs::write(
            bundle.join("bundle.json"),
            serde_json::to_string_pretty(&json!({
                "name": "witness",
                "policy": "policy.json",
                "seats": {
                    "implement": seat(vec!["complete"]),
                    "review": seat(vec!["clean"]),
                },
            }))
            .unwrap(),
        )
        .unwrap();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn db(&self) -> PathBuf {
        self.path().join("forge.db")
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
}

/// Every string key anywhere in a JSON value.
fn keys_deep(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                out.push(key.clone());
                keys_deep(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                keys_deep(item, out);
            }
        }
        _ => {}
    }
}

#[test]
fn a_non_adopting_run_journals_exactly_the_events_it_always_did() {
    let ws = Workspace::new();
    let bundle = ws.path().join("bundle");
    let db = ws.db();
    let (_, stderr) = ws.forge(&[
        "run",
        "--bundle",
        bundle.to_str().unwrap(),
        "--feature",
        "witness",
        "--db",
        db.to_str().unwrap(),
    ]);
    let run_id = stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .expect("run id on stderr")
        .trim()
        .to_string();

    ws.forge(&[
        "export",
        "--run",
        &run_id,
        "--out",
        ws.path().to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    let journal = std::fs::read_to_string(ws.path().join(format!("{run_id}.ndjson")))
        .expect("the exported journal");
    let events: Vec<Value> = journal
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("canonical NDJSON"))
        .collect();

    let shape: Vec<(String, Vec<String>)> = events
        .iter()
        .map(|event| {
            let event_type = event["type"].as_str().expect("typed event").to_string();
            let mut keys: Vec<String> = event["payload"]
                .as_object()
                .expect("object payload")
                .keys()
                .cloned()
                .collect();
            keys.sort();
            (event_type, keys)
        })
        .collect();
    let expected: Vec<(String, Vec<String>)> = GOLDEN
        .iter()
        .map(|(kind, keys)| {
            (
                (*kind).to_string(),
                keys.iter().map(|key| (*key).to_string()).collect(),
            )
        })
        .collect();
    assert_eq!(
        &shape[..expected.len()],
        &expected[..],
        "a non-adopting run's journal shape moved"
    );

    for event in &events {
        let mut keys = Vec::new();
        keys_deep(&event["payload"], &mut keys);
        for added in ADDED_FIELDS {
            assert!(
                !keys.iter().any(|key| key == added),
                "event {} carries '{added}'; a non-adopting run gains no field",
                event["type"]
            );
        }
    }
}
