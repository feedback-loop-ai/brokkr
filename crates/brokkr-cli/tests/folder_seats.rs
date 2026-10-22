//! Decision 0091 ruling 1 (#717, slice 3b): a shelf recipe's seat that
//! says `"from": "folder"` runs the folder's own. In a fresh folder made by
//! `git init` and `brokkr init .` with no stack marker, a fixture recipe
//! whose verify is the folder's own installs, compiles with the folder's
//! layer pinned, and a run of it reaches verify and runs the folder's
//! script, which fails closed with `init`'s fill-in text, never the
//! recipe's own, which would pass. A folder whose verify declares no hands
//! refuses such a seat at compile and at run.
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

/// The fixture's run, as each test starts it.
const RUN: [&str; 6] = [
    "run",
    "--recipe",
    "folder-seat",
    "--feature",
    "x",
    "--no-view",
];

/// The fixture's compile, as each test asks for it.
const COMPILE: [&str; 3] = ["compile", "--bundle", ".brokkr/recipes/folder-seat"];

/// A fresh folder `init` scaffolded for codex, the home `brokkr` runs
/// under, and the fixture written beside them, not yet added.
struct Scratch {
    repo: tempfile::TempDir,
    aside: tempfile::TempDir,
    source: tempfile::TempDir,
}

impl Scratch {
    fn new() -> Scratch {
        let scratch = Scratch {
            repo: repository(),
            aside: tempfile::tempdir().unwrap(),
            source: tempfile::tempdir().unwrap(),
        };
        let (code, _, stderr) = scratch.brokkr(&["init", "."]);
        assert_eq!(code, Some(0), "{stderr}");
        fixture(scratch.source.path());
        scratch
    }

    /// `brokkr` with `args`, in the folder, for codex.
    fn brokkr(&self, args: &[&str]) -> (Option<i32>, String, String) {
        brokkr_for("codex", args, self.repo.path(), self.aside.path())
    }

    /// Add the fixture to the folder's shelf as `folder-seat`.
    fn add(&self) {
        let source = self.source.path().to_str().unwrap();
        let (code, _, stderr) = self.brokkr(&["recipes", "add", source, "--name", "folder-seat"]);
        assert_eq!(code, Some(0), "{stderr}");
    }
}

#[test]
fn a_shelf_recipes_folder_seat_runs_the_folders_own_script() {
    let scratch = Scratch::new();
    scratch.add();
    let (repo, brokkr) = (scratch.repo.path(), |args: &[&str]| scratch.brokkr(args));

    let (code, stdout, stderr) = brokkr(&COMPILE);
    assert_eq!(code, Some(0), "{stderr}");
    let compiled: Value = serde_json::from_str(&stdout).unwrap();
    let files = compiled["manifest"]["files"].as_object().unwrap();
    let digest = files["@folder/starter"].as_str().unwrap();
    assert_eq!(digest.len(), 64, "{digest}");

    for identity in [["user.name", "t"], ["user.email", "t@example.invalid"]] {
        git(repo, &[&["config"][..], &identity[..]].concat());
    }
    // Either verdict ends the fixture's table at `stop`: the run stopped.
    let (code, _, stderr) = brokkr(&RUN);
    assert_eq!(code, Some(3), "{stderr}");
    let run = stderr
        .lines()
        .find_map(|line| line.strip_prefix("run started: "))
        .unwrap_or_else(|| panic!("a run started: {stderr}"));
    let journaled = payloads(&repo.join(".brokkr/state/forge.db"), run);
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

/// Remove `key` from the verify seat the bundle `file` declares.
fn strip_verify(file: &Path, key: &str) {
    let mut declared: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let verify = declared["seats"]["verify"].as_object_mut().unwrap();
    assert!(verify.remove(key).is_some(), "{verify:?}");
    std::fs::write(file, declared.to_string()).unwrap();
}

/// A folder whose own verify declares no hands refuses the fixture's
/// folder seat at compile with the seventh refusal's line, and a run of
/// it refuses before any seat spawns. Both verifies are work seats, which
/// with no hands would otherwise compile and run the folder's script.
#[test]
fn a_folder_seat_whose_folder_declares_no_hands_refuses() {
    let scratch = Scratch::new();
    let file = scratch.repo.path().join(".brokkr/bundle.json");
    for bundle in [&file, &scratch.source.path().join("bundle.json")] {
        strip_verify(bundle, "class");
    }
    scratch.add();
    strip_verify(&file, "hands");

    let refusal = format!(
        "bundle: seat 'verify' is the folder's own (decision 0091), and the folder's declares no \
         hands, so its script would spawn without the walk that checks it: declare its hands in \
         {}",
        file.canonicalize().unwrap().display()
    );
    let (code, stdout, stderr) = scratch.brokkr(&COMPILE);
    assert_eq!((code, stdout.as_str()), (Some(1), ""), "{stderr}");
    assert_eq!(stderr, format!("error: {refusal}\n"));

    let (code, _, stderr) = scratch.brokkr(&RUN);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(!stderr.contains("run started: "), "{stderr}");
    assert_eq!(
        stderr.lines().last(),
        Some(format!("error: {refusal}").as_str())
    );
}
