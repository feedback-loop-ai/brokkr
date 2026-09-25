//! Without `--repo` the engine operates the directory it stands in, and
//! the git guards read that same tree (#368): a dirty worktree parks ship
//! on SHIP-DIRTY and a gate that moves HEAD parks on GATE-MOVED-HEAD,
//! exactly as they do with `--repo .`.
//!
//! One test in its own binary, because it moves the process's working
//! directory and no other test may run beside it while it does.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use brokkr_runtime::{Bundle, Engine};
use brokkr_store::Store;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;

const POLICY: &str = r#"{
  "phases": ["work", "review", "ship", "done", "stop"],
  "initial": "work",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "WORK-DONE", "from": "work", "result": "complete", "next": "review",
     "reason": "done"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "ship",
     "reason": "clean"},
    {"id": "SHIP-DIRTY", "from": "ship", "result": "ready",
     "when": {"dirty_worktrees": true}, "next": "stop", "severity": "hard",
     "reason": "A dirty worktree at ship time is a defect."},
    {"id": "SHIP-READY", "from": "ship", "result": "ready", "next": "done",
     "reason": "ready"}
  ]
}"#;

/// A driver that answers `result`, first running `before` in the tree it
/// was started in.
fn driver(before: &str, result: &str) -> Vec<String> {
    let script = format!(
        r#"read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
{before}
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{{"result":"{result}"}},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#
    );
    vec!["sh".into(), "-c".into(), script]
}

/// `work` is a gate seat whose driver runs `gate`; the rest answer at once.
fn bundle(dir: &Path, gate: &str) -> Bundle {
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    let seat = |command: Vec<String>, results: &[&str]| {
        json!({
            "role": "roles/role.md",
            "results": results,
            "driver": {"command": command},
        })
    };
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_string(&json!({
            "name": "operated-repo",
            "policy": "policy.json",
            "seats": {
                "work": seat(driver(gate, "complete"), &["complete"]),
                "review": seat(driver(":", "clean"), &["clean"]),
                "ship": seat(driver(":", "ready"), &["ready"]),
            },
        }))
        .unwrap(),
    )
    .unwrap();
    // Seated as a gate after compiling, as the engine's own gate tests
    // do: a `class: gate` in the file would need adapter data.
    let mut compiled = Bundle::compile(&bundle).unwrap();
    compiled.seats.get_mut("work").unwrap().has_gate = true;
    compiled
}

fn git(repo: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .unwrap()
        .success());
}

/// A fresh repository with one commit, and the process standing in it.
fn repository(dir: &Path, name: &str) -> PathBuf {
    let repo = dir.join(name);
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["config", "user.name", "Brokkr Test"]);
    git(&repo, &["config", "user.email", "brokkr@test"]);
    git(&repo, &["config", "commit.gpgSign", "false"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    std::env::set_current_dir(&repo).unwrap();
    repo
}

fn run(dir: &Path, gate: &str, repo: Option<PathBuf>) -> (Status, Option<String>) {
    let store = Store::open(&dir.join(format!("forge-{}.db", uuid()))).unwrap();
    let mut engine = Engine::start(store, bundle(dir, gate), "operated", repo).unwrap();
    let end = engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let decided = events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::TransitionDecided)
        .map(|event| event.payload["rule_id"].as_str().unwrap_or("").to_string());
    (end.state.status, end.state.park_reason.or(decided))
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn without_repo_the_guards_read_the_directory_the_run_stands_in() {
    let dir = tempfile::tempdir().unwrap();
    for (mode, repo) in [("dot", Some(PathBuf::from("."))), ("none", None)] {
        // A dirty worktree parks ship, named or not.
        let tree = repository(dir.path(), &format!("dirty-{mode}"));
        std::fs::write(tree.join("uncommitted.txt"), "mid-thought").unwrap();
        assert_eq!(
            run(dir.path(), ":", repo.clone()),
            (Status::Stopped, Some("SHIP-DIRTY".to_string())),
            "{mode}"
        );

        // A gate that commits parks on the moved head, named or not.
        repository(dir.path(), &format!("gate-{mode}"));
        assert_eq!(
            run(dir.path(), "git commit -q --allow-empty -m moved", repo),
            (
                Status::AwaitingOperator,
                Some("GATE-MOVED-HEAD".to_string())
            ),
            "{mode}"
        );
    }
}
