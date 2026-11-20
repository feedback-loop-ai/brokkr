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

use serde_json::{json, Value};

/// The two-phase table, the realm suite's own.
use crate::realms::POLICY;

/// The event sequence a two-phase run produces: type, then the payload's
/// keys in sorted order. Recorded from this tree before the agent
/// library existed.
const REQUESTED: (&str, &[&str]) = (
    "effect/requested",
    &[
        "effect_id",
        "idempotency_key",
        "input_digest",
        "phase",
        "seat",
    ],
);

const GOLDEN: [(&str, &[&str]); 11] = [
    // `charters`: the bindings a start checked, which its resume is held
    // to (rebuild unit 19; operator ruling 2026-09-29, admission 2a).
    // Decision 0094 ruling 6 adds the compiled phase order, without moving the manifest.
    (
        "run/started",
        &["charters", "feature", "manifest", "phases"],
    ),
    ("phase/entered", &["phase"]),
    REQUESTED,
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
    REQUESTED,
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

/// Writes the result `$1` at the result path the prompt `$2` names.
const ANSWER: &str = r#"result_path=$(grep -o '/[^[:space:]]*\.json' "$2" | tail -n 1)
mkdir -p "$(dirname "$result_path")"
printf '{"result": "%s"}\n' "$1" > "$result_path"
"#;

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
        // No agent: the built-in exec driver, which has no model surface
        // for SI2 to judge past the lifted fence (U9b1), answers each seat.
        std::fs::write(bundle.join("answer.sh"), ANSWER).unwrap();
        let exec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/exec.json");
        std::fs::create_dir_all(ws.path().join("adapters")).unwrap();
        std::fs::copy(exec, ws.path().join("adapters/exec.json")).unwrap();
        let seat = |results: Vec<&str>| {
            json!({"role": "roles/role.md", "results": results, "driver": {"command": [
                "{brokkr}", "driver", "exec", "--", "sh", "./answer.sh", results[0], "{prompt_file}",
            ]}})
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

    fn brokkr(&self, args: &[&str]) -> (String, String) {
        crate::engine::printed_in(self.path(), args)
    }

    /// `brokkr run` over the bundle: the run id, and the whole stderr.
    fn run(&self) -> (String, String) {
        crate::engine::run_in(self.path(), "witness", &self.db())
    }
}

/// The endings decision 0112 ruling 3 lets carry `usage`.
const ENDINGS: [&str; 3] = ["effect/succeeded", "effect/failed", "effect/indeterminate"];

/// `events` as the golden reads them: each event's type and its sorted
/// payload keys, a run of checkpoints read as one.
///
/// How many checkpoints an attempt reports is its driver's: the exec
/// driver these seats run (U9b1) reports three where the scripted fake
/// driver the golden was recorded under reported one, each of the same
/// shape, so a run of them reads as one. Whether an ending carries
/// `usage` is a tick's: decision 0112 ruling 3 makes it an optional,
/// absent-by-default extension at `event_schema: 1`, present only when a
/// tick saw the driver's tree, so on the three endings that one key is
/// set aside and every other key is still compared.
fn shape_of(events: &[Value]) -> Vec<(String, Vec<String>)> {
    let mut shape: Vec<(String, Vec<String>)> = events
        .iter()
        .map(|event| {
            let event_type = event["type"].as_str().expect("typed event").to_string();
            let ending = ENDINGS.contains(&event_type.as_str());
            let mut keys: Vec<String> = event["payload"]
                .as_object()
                .expect("object payload")
                .keys()
                .filter(|key| !(ending && *key == "usage"))
                .cloned()
                .collect();
            keys.sort();
            (event_type, keys)
        })
        .collect();
    shape.dedup_by(|next, kept| next == kept && kept.0 == "effect/checkpointed");
    shape
}

/// The type of every event of `events` other than the three endings that
/// carries `usage` anywhere in its payload.
fn usage_off_the_endings(events: &[Value]) -> Vec<String> {
    events
        .iter()
        .filter(|event| !ENDINGS.contains(&event["type"].as_str().expect("typed event")))
        .filter(|event| {
            let mut keys = Vec::new();
            keys_deep(&event["payload"], &mut keys);
            keys.iter().any(|key| key == "usage")
        })
        .map(|event| event["type"].as_str().expect("typed event").to_string())
        .collect()
}

/// The witness sets aside `usage` only on the three endings: a planted
/// one there reads as the golden's keys, a stray key beside it still
/// shows, and `usage` on any other type is named.
#[test]
fn only_an_endings_usage_is_set_aside() {
    let events = [
        json!({"type": "run/started", "payload": {"feature": "f", "usage": {}}}),
        json!({"type": "effect/succeeded", "payload": {
            "attempt_id": "a", "effect_id": "e", "result": {}, "usage": {"cpu_ms": 1},
        }}),
        json!({"type": "effect/failed", "payload": {"error": "x", "stray": 1, "usage": {}}}),
        json!({"type": "effect/checkpointed", "payload": {"checkpoint": {"usage": {}}}}),
    ];
    let keys =
        |keys: &[&str]| -> Vec<String> { keys.iter().map(|key| (*key).to_string()).collect() };
    assert_eq!(
        shape_of(&events),
        [
            ("run/started".to_string(), keys(&["feature", "usage"])),
            (
                "effect/succeeded".to_string(),
                keys(&["attempt_id", "effect_id", "result"])
            ),
            ("effect/failed".to_string(), keys(&["error", "stray"])),
            ("effect/checkpointed".to_string(), keys(&["checkpoint"])),
        ]
    );
    assert_eq!(
        usage_off_the_endings(&events),
        ["run/started", "effect/checkpointed"]
    );
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
    let db = ws.db();
    let (run_id, _) = ws.run();

    ws.brokkr(&[
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

    let shape = shape_of(&events);
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
    assert_eq!(
        usage_off_the_endings(&events),
        Vec::<String>::new(),
        "only an attempt's ending carries `usage` (decision 0112 ruling 3)"
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

/// The engine returns a conclusion's gaps and writes to no stream; the CLI
/// prints them after the run's start (#355). A workspace that is no git
/// repository leaves exactly the anchor gap. Its tail is git's own
/// failure, which races: git can exit before its stdin is written, so the
/// tail reads either git's refusal or the broken pipe, and is not pinned.
#[test]
fn a_conclusions_gaps_reach_stderr() {
    let ws = Workspace::new();
    let (run_id, stderr) = ws.run();
    let head = format!("run started: {run_id}\nanchor gap for {run_id}: git ");
    assert!(stderr.starts_with(&head), "{stderr}");
    assert_eq!(stderr.matches(" gap for ").count(), 1, "{stderr}");
}

/// Set, to anything, in this binary re-executed to play
/// [`a_leaked_tree_stands_in_the_shared_tmpdir`].
const LEAKED_ROLE: &str = "BROKKR_TEST_646_LEAKED";

/// A dead server's tree in the TMPDIR this process shares, as a server
/// killed before its session dropped leaves it: its owner's pid dead, its
/// lock held by none. Built under a `.staging-` name no reaper reads, as
/// the session module builds its own, and renamed into place whole.
struct Leaked(PathBuf);

impl Leaked {
    fn plant() -> Leaked {
        let uuid = "6460c0de-3b1e-4a4d-9e5f-000000000646";
        let name = format!("brokkr-hands-server-{}-{uuid}", crate::engine::dead_pid());
        let tmp = std::env::temp_dir();
        let staged = tmp.join(format!(".staging-{name}"));
        std::fs::create_dir_all(staged.join("etc")).unwrap();
        std::fs::write(staged.join(".owner.lock"), "").unwrap();
        let tree = tmp.join(name);
        std::fs::rename(&staged, &tree).unwrap();
        Leaked(tree)
    }
}

/// #646: a tree another test left in the shared TMPDIR is not this run's
/// to say. This binary is re-executed with a TMPDIR of its own standing
/// as the shared one, so no other test's engine reaps the planted tree
/// before the run this test asserts.
#[test]
fn a_tree_left_in_the_shared_tmpdir_does_not_reach_a_runs_stderr() {
    let shared = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "witness_journal::a_leaked_tree_stands_in_the_shared_tmpdir",
            "--ignored",
            "--nocapture",
        ])
        .env("TMPDIR", shared.path())
        .env(LEAKED_ROLE, "1")
        .output()
        .unwrap();
    let said =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success() && said.contains("1 passed"),
        "the re-executed scenario passed: {said}"
    );
    // The scenario played, and planted where this test stood the shared
    // TMPDIR: a role or a plant that stopped reaching it would pass empty.
    let stands = format!(
        "the planted tree stands: {}/brokkr-hands-server-",
        shared.path().display()
    );
    assert!(said.contains(&stands), "{stands} in: {said}");
}

/// The scenario [`a_tree_left_in_the_shared_tmpdir_does_not_reach_a_runs_stderr`]
/// plays in this binary re-executed. The engine's TMPDIR is its
/// workspace's own ([`crate::engine::brokkr_in`]), so its start reaps
/// nothing of the shared one: its stderr reads exactly as
/// [`a_conclusions_gaps_reach_stderr`] pins, and the planted tree stands
/// after it. Run on its own, it plays none.
#[test]
#[ignore = "played only when a test re-executes this binary"]
fn a_leaked_tree_stands_in_the_shared_tmpdir() {
    if std::env::var_os(LEAKED_ROLE).is_none() {
        return;
    }
    let leaked = Leaked::plant();
    a_conclusions_gaps_reach_stderr();
    assert!(leaked.0.join(".owner.lock").is_file(), "{:?}", leaked.0);
    println!("the planted tree stands: {}", leaked.0.display());
}
