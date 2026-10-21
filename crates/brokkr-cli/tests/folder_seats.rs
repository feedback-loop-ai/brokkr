//! Decision 0091 ruling 1 (#717, slice 3b): a shelf recipe's seat that
//! says `"from": "folder"` runs the folder's own. In a fresh folder made by
//! `git init` and `brokkr init .` with no stack marker, a fixture recipe
//! whose verify is the folder's own installs, compiles with the folder's
//! layer pinned, and a run of it reaches verify and runs the folder's
//! script, which fails closed with `init`'s fill-in text, never the
//! recipe's own, which would pass.
//!
//! The folder is scaffolded for codex, whose `init` writes the `harness`
//! boundary, so no box is needed.

use std::path::Path;

use serde_json::{json, Value};

use crate::brokkr_folder::{git, repository};
use crate::init_stacks::brokkr_for;

/// What the fixture's own verify script notes, were it ever run.
const SENTINEL: &str = "the fixture's own verify ran";

/// `init`'s verifier in a folder with no recognised stack.
const FILL_IN: &str =
    "no stack was recognized; fill in .brokkr/scripts/verify-seat.sh before running this gate";

/// The fixture's own verify script: it passes, noting the sentinel.
fn own_verify() -> String {
    let find = r#"grep -E '^[[:space:]]*/.*\.json$' "$1" | head -n 1 | tr -d '[:space:]'"#;
    let write = r#"printf '{"result": "pass", "notes": "%s"}\n'"#;
    format!("#!/usr/bin/env bash\nout=\"$({find})\"\n{write} \"{SENTINEL}\" > \"$out\"\n")
}

/// A recipe whose one seat, verify, is the folder's own, then stop.
fn fixture(dir: &Path) {
    let rule = |id: &str, result: &str| json!({"id": id, "from": "verify", "result": result, "next": "stop", "reason": result});
    let table = json!({"schema": "forge.phase-machine/v1", "phases": ["verify", "stop"],
                       "initial": "verify", "terminal": ["stop"],
                       "rules": [rule("PASS", "pass"), rule("FAIL", "fail")]});
    let verify = json!({
        "from": "folder",
        "class": "gate",
        "results": ["pass", "fail"],
        "limits": {"max_attempts": 1, "timeout_seconds": 600},
        "driver": {"command": ["{brokkr}", "driver", "exec", "--", "bash",
                               "./scripts/verify-seat.sh", "{prompt_file}"]},
        "hands": {"kind": "workspace", "network": false, "binds": []},
    });
    let bundle = json!({"name": "folder-seat", "policy": "policy.json",
                        "protected_phase": "verify", "seats": {"verify": verify}});
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(dir.join("scripts/verify-seat.sh"), own_verify()).unwrap();
    std::fs::write(dir.join("policy.json"), table.to_string()).unwrap();
    std::fs::write(dir.join("bundle.json"), bundle.to_string()).unwrap();
}

/// Every payload `run` journaled in `db`, in order.
fn payloads(db: &Path, run: &str) -> Vec<Value> {
    let store = brokkr_store::Store::open(db).unwrap();
    let events = store.load(run).unwrap().into_iter();
    events.map(|event| event.payload).collect()
}

#[test]
fn a_shelf_recipes_folder_seat_runs_the_folders_own_script() {
    let (repo, aside, source) = (
        repository(),
        tempfile::tempdir().unwrap(),
        tempfile::tempdir().unwrap(),
    );
    let brokkr = |args: &[&str]| brokkr_for("codex", args, repo.path(), aside.path());
    let (code, _, stderr) = brokkr(&["init", "."]);
    assert_eq!(code, Some(0), "{stderr}");
    fixture(source.path());
    let add = [
        "recipes",
        "add",
        source.path().to_str().unwrap(),
        "--name",
        "folder-seat",
    ];
    let (code, _, stderr) = brokkr(&add);
    assert_eq!(code, Some(0), "{stderr}");

    let (code, stdout, stderr) = brokkr(&["compile", "--bundle", ".brokkr/recipes/folder-seat"]);
    assert_eq!(code, Some(0), "{stderr}");
    let compiled: Value = serde_json::from_str(&stdout).unwrap();
    let files = compiled["manifest"]["files"].as_object().unwrap();
    let digest = files["@folder/starter"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "{digest}");

    let run = [
        "run",
        "--recipe",
        "folder-seat",
        "--feature",
        "x",
        "--no-view",
    ];
    for identity in [["user.name", "t"], ["user.email", "t@example.invalid"]] {
        git(repo.path(), &[&["config"][..], &identity[..]].concat());
    }
    // Either verdict ends the fixture's table at `stop`: the run stopped.
    let (code, _, stderr) = brokkr(&run);
    assert_eq!(code, Some(3), "{stderr}");
    let run = stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .unwrap_or_else(|| panic!("a run started: {stderr}"));
    let journaled = payloads(&repo.path().join(".brokkr/state/forge.db"), run);
    let ruled: Vec<(&Value, &Value)> = journaled
        .iter()
        .filter(|payload| payload.get("rule_id").is_some())
        .map(|payload| (&payload["from"], &payload["result"]))
        .collect();
    assert_eq!(ruled, [(&json!("verify"), &json!("fail"))], "{journaled:?}");
    let text = Value::Array(journaled).to_string();
    assert!(text.contains(FILL_IN), "{text}");
    assert!(!text.contains(SENTINEL), "{text}");
}
