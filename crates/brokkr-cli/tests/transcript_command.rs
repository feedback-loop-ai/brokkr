//! `brokkr transcript`: the scriptable face of the shared local reader
//! (#222, proposed decision 0055). Each case builds a synthetic journal
//! and synthetic test-owned homes; no real operator session is copied.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

struct World {
    dir: tempfile::TempDir,
    db: PathBuf,
    home: PathBuf,
}

impl World {
    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn projects(&self) -> PathBuf {
        self.home.join(".claude").join("projects")
    }
}

/// A run with one participant and no transcript reference yet.
fn world() -> World {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r222", "feat", "self", &json!({"files": {}}))
        .unwrap();
    let events: Vec<(EventType, Value)> = vec![
        (
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "intake"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "review", "phase": "intake"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att1"}),
        ),
    ];
    for (event_type, payload) in events {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    World { dir, db, home }
}

/// Record the selected participant's transcript reference.
fn record(world: &World, transcript: Value) {
    let mut store = Store::open(&world.db).unwrap();
    store
        .append_next(
            "r222",
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att1",
                   "checkpoint": {"step": "session-finished", "transcript": transcript}}),
            None,
            None,
        )
        .unwrap();
}

fn run(world: &World, args: &[&str]) -> std::process::Output {
    Command::new(brokkr_bin())
        .args(args)
        .arg("--db")
        .arg(&world.db)
        .env("HOME", &world.home)
        .current_dir(world.path())
        .output()
        .unwrap()
}

/// Materialize one Claude transcript under an immediate project directory.
fn write_claude(world: &World, id: &str, body: &str) -> String {
    let project = world.projects().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let file = project.join(format!("{id}.jsonl"));
    std::fs::write(&file, body).unwrap();
    file.to_str().unwrap().to_string()
}

/// The id guard rejects a leading hyphen before any path is formed.
#[test]
fn an_invalid_reference_writes_no_document() {
    let world = world();
    record(
        &world,
        json!({"kind": "codex-thread", "locator": "-abc", "home": "/retained/codex"}),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["schema"], "brokkr.transcript/v1");
    assert_eq!(document["unavailable"], "invalid-reference");
    assert_eq!(document["transcript"]["locator"], "-abc");
    assert_eq!(document["legacy"], false);
    assert!(document["path"].is_null());
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert!(document["full_session"].is_null());
}

/// A readable Claude file renders the shared turns and the exact hint.
#[test]
fn a_readable_claude_source_serializes_its_turns() {
    let world = world();
    record(
        &world,
        json!({"kind": "claude-session", "locator": "abcd-1234",
               "home": world.projects().to_str().unwrap()}),
    );
    let path = write_claude(
        &world,
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"hello\"},\"timestamp\":\"T\"}\n",
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["unavailable"], Value::Null);
    assert_eq!(document["path"], path);
    assert_eq!(document["turns"][0]["role"], "assistant");
    assert_eq!(document["turns"][0]["blocks"][0]["text"], "hello");
    assert_eq!(
        document["full_session"],
        "full session: claude --resume abcd-1234"
    );
    assert_eq!(document["turn"], Value::Null);
}

/// `--turn` selects one displayed position and keeps the whole read's
/// metadata; a refused read keeps its own reason whatever index is asked.
#[test]
fn turn_selection_cannot_override_a_refusal() {
    let world = world();
    record(&world, json!({"kind": "none", "locator": "", "home": ""}));
    let output = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "1",
        ],
    );
    assert!(!output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["unavailable"], "none");
    assert_eq!(document["turn"], 1);
}

/// Text mode keeps terminal controls out of the frame while JSON carries
/// them as escaped data.
#[test]
fn text_mode_sanitizes_what_json_preserves() {
    let world = world();
    record(
        &world,
        json!({"kind": "claude-session", "locator": "abcd-1234",
               "home": world.projects().to_str().unwrap()}),
    );
    write_claude(
        &world,
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"\\u001b[2Jboom\"}}\n",
    );
    let output = run(&world, &["transcript", "--run", "r222", "--seat", "eff1"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        !text.contains('\u{1b}'),
        "the escape byte never reaches the tty"
    );
    assert!(text.contains("boom"));

    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["turns"][0]["blocks"][0]["text"], "\u{1b}[2Jboom");
}

/// A unique label resolves to the exact key; a missing seat leaves
/// stdout empty because selection happens before any document exists.
#[test]
fn a_unique_label_resolves_and_a_missing_seat_writes_nothing() {
    let world = world();
    record(&world, json!({"kind": "none", "locator": "", "home": ""}));
    // The fixture has one participant: label `review`, key `eff1`.
    let by_label = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "review", "--json"],
    );
    // The read is an explicit `none`, so the command exits one, but the
    // label resolved to the exact key and produced the document.
    let document: Value = serde_json::from_slice(&by_label.stdout).unwrap();
    assert_eq!(document["seat"], "eff1");
    assert_eq!(document["unavailable"], "none");

    let missing = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "nobody", "--json"],
    );
    assert!(!missing.status.success());
    assert!(
        missing.stdout.is_empty(),
        "selection failures write no stdout"
    );
}

/// `--turn 0` is a usage error that writes nothing, including under `--json`.
#[test]
fn turn_zero_is_a_usage_error() {
    let world = world();
    record(&world, json!({"kind": "none", "locator": "", "home": ""}));
    let output = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "0",
        ],
    );
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
