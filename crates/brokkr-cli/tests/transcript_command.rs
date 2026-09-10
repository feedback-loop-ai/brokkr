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

/// A run whose effects are the given `(effect_id, seat, provider)`
/// triples, each with one open attempt so a checkpoint can attach.
fn world_effects(effects: &[(&str, &str, Option<&str>)]) -> World {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r222", "feat", "self", &json!({"files": {}}))
        .unwrap();
    store
        .append_next(
            "r222",
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r222",
            EventType::PhaseEntered,
            json!({"phase": "intake"}),
            None,
            None,
        )
        .unwrap();
    for (index, (effect_id, seat, provider)) in effects.iter().enumerate() {
        let attempt_id = format!("att{index}");
        store
            .append_next(
                "r222",
                EventType::EffectRequested,
                json!({"effect_id": effect_id, "seat": seat, "phase": "intake"}),
                None,
                None,
            )
            .unwrap();
        let mut started = json!({"effect_id": effect_id, "attempt_id": attempt_id});
        if let Some(provider) = provider {
            started["provenance"] = json!([{"provider": provider}]);
        }
        store
            .append_next("r222", EventType::EffectStarted, started, None, None)
            .unwrap();
    }
    World { dir, db, home }
}

/// Append one checkpoint to an already-started attempt.
fn checkpoint(world: &World, effect_id: &str, attempt_id: &str, checkpoint: Value) {
    let mut store = Store::open(&world.db).unwrap();
    store
        .append_next(
            "r222",
            EventType::EffectCheckpointed,
            json!({"effect_id": effect_id, "attempt_id": attempt_id, "checkpoint": checkpoint}),
            None,
            None,
        )
        .unwrap();
}

/// Materialize a full DSH source below the recorded locator.
fn write_dsh_body(world: &World, locator: &str, body: &str) -> String {
    let session = world.home.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&session).unwrap();
    let file = session.join("session.jsonl");
    std::fs::write(&file, body).unwrap();
    file.to_str().unwrap().to_string()
}

fn claude_reference(world: &World, id: &str) -> Value {
    json!({"kind": "claude-session", "locator": id,
           "home": world.projects().to_str().unwrap()})
}

fn codex_reference(world: &World, id: &str) -> Value {
    json!({"kind": "codex-thread", "locator": id,
           "home": world.home.to_str().unwrap()})
}

fn dsh_reference(world: &World, locator: &str) -> Value {
    json!({"kind": "dsh-session", "locator": locator,
           "home": world.home.to_str().unwrap()})
}

/// The shipped Claude projection fixture from `brokkr-view`, whose
/// `(skipped_lines, unrecognized_records)` counts are pinned at `(1, 0)`.
const SHIPPED_CLAUDE: &str = concat!(
    "not json\n",
    "{\"type\":\"summary\"}\n",
    "{\"type\":\"user\"}\n",
    "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",",
    "\"content\":\"the plain form\"},\"timestamp\":\"2026-01-01T00:00:00Z\"}\n",
    "{\"type\":\"user\",\"message\":{\"content\":[",
    "{\"type\":\"text\",\"text\":\"what happened\"},",
    "{\"type\":\"text\",\"text\":\"   \"},",
    "{\"type\":\"text\"},",
    "{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{\"file_path\":\"src/lib.rs\"}},",
    "{\"type\":\"tool_use\",\"name\":\"Bash\"},",
    "{\"type\":\"thinking\"}]}}\n",
);

fn read_document(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not one JSON document: {error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
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

fn write_codex(world: &World, id: &str, body: &str) -> String {
    let sessions = world.home.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let file = sessions.join(format!("rollout-{id}.jsonl"));
    std::fs::write(&file, body).unwrap();
    file.to_str().unwrap().to_string()
}

fn write_dsh(world: &World, locator: &str, header: &str) -> String {
    let session = world.home.join(locator).join("project").join("seat");
    std::fs::create_dir_all(&session).unwrap();
    let file = session.join("session.jsonl");
    std::fs::write(&file, format!("{header}\n")).unwrap();
    file.to_str().unwrap().to_string()
}

/// A Codex rollout needs no header: a `turn_context`-only file is a
/// readable zero-turn result with its confirmed path and shared hint.
#[test]
fn a_headerless_codex_rollout_is_readable() {
    let world = world();
    let path = write_codex(&world, "0199mine", "{\"type\":\"turn_context\"}\n");
    record(
        &world,
        json!({"kind": "codex-thread", "locator": "0199mine",
               "home": world.home.to_str().unwrap()}),
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
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert_eq!(
        document["full_session"],
        format!(
            "full session: \"{path}\"; codex exec resume 0199mine; home: \"{}\"",
            world.home.to_str().unwrap()
        )
    );
}

/// A missing Codex rollout carries the fixed `rollout unavailable` hint.
#[test]
fn a_missing_codex_rollout_has_its_fixed_hint() {
    let world = world();
    record(
        &world,
        json!({"kind": "codex-thread", "locator": "019c-222a",
               "home": "/retained/codex"}),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["unavailable"], "not-found");
    assert!(document["path"].is_null());
    assert_eq!(
        document["full_session"],
        "full session: rollout unavailable; codex exec resume 019c-222a; home: \"/retained/codex\""
    );
}

/// DSH file lookups have a null hint; an invalid depth has the fixed
/// explanation and never borrows a later header.
#[test]
fn dsh_lookup_failures_have_their_fixed_words() {
    let world = world();
    record(
        &world,
        json!({"kind": "dsh-session", "locator": "sessions/one",
               "home": world.home.to_str().unwrap()}),
    );
    let missing = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!missing.status.success());
    let document: Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(document["unavailable"], "not-found");
    assert!(document["full_session"].is_null());

    write_dsh(
        &world,
        "sessions/one",
        "{\"type\":\"session\",\"delegationDepth\":\"zero\",\"version\":0}",
    );
    let invalid = run(&world, &["transcript", "--run", "r222", "--seat", "eff1"]);
    assert!(!invalid.status.success());
    assert!(invalid.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&invalid.stderr).contains("no valid depth-zero DSH session header"),
        "{}",
        String::from_utf8_lossy(&invalid.stderr)
    );
}

/// An owned DSH file with a foreign version refuses as `unsupported-format`
/// with the confirmed path, zero counts and the DSH path hint.
#[test]
fn a_foreign_dsh_version_refuses_with_its_document() {
    let world = world();
    let path = write_dsh(
        &world,
        "sessions/one",
        "{\"type\":\"session\",\"version\":1}\n{\"type\":\"user/message\"}",
    );
    record(
        &world,
        json!({"kind": "dsh-session", "locator": "sessions/one",
               "home": world.home.to_str().unwrap()}),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(document["unavailable"], "unsupported-format");
    assert_eq!(document["path"], path);
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert_eq!(document["skipped_lines"], 0);
    assert_eq!(document["unrecognized_records"], 0);
    assert_eq!(
        document["full_session"],
        format!("full session: \"{path}\"")
    );
}

// ---------------------------------------------- 8.8 selection and turns

/// `--run latest` and a unique label reach the same shared read.
#[test]
fn latest_and_a_unique_label_resolve_the_shared_read() {
    let world = world_effects(&[("eff1", "review", None)]);
    let path = write_claude(&world, "abcd-1234", SHIPPED_CLAUDE);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let output = run(
        &world,
        &[
            "transcript",
            "--run",
            "latest",
            "--seat",
            "review",
            "--json",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document = read_document(&output);
    assert_eq!(document["run_id"], "r222");
    assert_eq!(document["seat"], "eff1");
    assert_eq!(document["path"], path);
    assert_eq!(document["turns"].as_array().unwrap().len(), 2);
}

/// Two `implement` participants from two phase visits: the first effect
/// must be decided and the phase advanced before the second is legal.
fn world_repeated_label() -> World {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r222", "feat", "self", &json!({"files": {}}))
        .unwrap();
    let none = json!({"kind": "none", "locator": "", "home": ""});
    let events: Vec<(EventType, Value)> = vec![
        (
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "intake"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff1", "seat": "implement", "phase": "intake"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff1", "attempt_id": "att0"}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att0",
                   "checkpoint": {"step": "session-finished", "transcript": none}}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "eff1", "attempt_id": "att0",
                   "result": {"result": "intook"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"rule_id": "INTAKE-OK", "severity": "normal",
                   "from": "intake", "next": "design", "result": "intook"}),
        ),
        (EventType::PhaseEntered, json!({"phase": "design"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff2", "seat": "implement", "phase": "design"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff2", "attempt_id": "att1"}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff2", "attempt_id": "att1",
                   "checkpoint": {"step": "session-finished", "transcript": none}}),
        ),
    ];
    for (event_type, payload) in events {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    World { dir, db, home }
}

/// A repeated label lists both keys and writes nothing; an exact key
/// selects that visit.
#[test]
fn a_repeated_label_names_both_keys_and_an_exact_key_selects() {
    let world = world_repeated_label();
    let ambiguous = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "implement",
            "--json",
        ],
    );
    assert!(!ambiguous.status.success());
    assert!(
        ambiguous.stdout.is_empty(),
        "selection failures write no stdout"
    );
    let stderr = String::from_utf8_lossy(&ambiguous.stderr);
    assert!(
        stderr.contains("eff1") && stderr.contains("eff2"),
        "{stderr}"
    );
    let exact = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff2", "--json"],
    );
    let document = read_document(&exact);
    assert_eq!(document["seat"], "eff2");
}

/// A panel parent reads only its own reference; a member reads its file.
#[test]
fn a_panel_parent_has_no_reference_while_its_member_reads() {
    let world = world_effects(&[("eff1", "panel", None)]);
    let path = write_claude(
        &world,
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"member\"}}\n",
    );
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "panel-member-finished",
            "member": "m1",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let parent = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!parent.status.success());
    let document = read_document(&parent);
    assert_eq!(document["unavailable"], "no-reference");
    assert!(document["turns"].as_array().unwrap().is_empty());
    let member = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1:m1", "--json"],
    );
    assert!(
        member.status.success(),
        "{}",
        String::from_utf8_lossy(&member.stderr)
    );
    let document = read_document(&member);
    assert_eq!(document["seat"], "eff1:m1");
    assert_eq!(document["path"], path);
}

/// A Codex rollout of metadata, two messages, a tool pair and malformed
/// lines: `--turn 1` skips the metadata and the malformed line, and
/// `--turn 4` is the tool result.
#[test]
fn turn_selection_skips_metadata_and_selects_the_tool_result() {
    let world = world_effects(&[("eff1", "review", None)]);
    let body = concat!(
        "{\"timestamp\":\"t0\",\"type\":\"turn_context\",\"payload\":{\"model\":\"m\"}}\n",
        "not json\n",
        "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
        "{\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"reasoning\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"think\"}]}}\n",
        "{\"timestamp\":\"t3\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"Read\",\"call_id\":\"c1\",\"arguments\":\"{}\"}}\n",
        "{\"timestamp\":\"t4\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"c1\",\"output\":\"ok\"}}\n",
    );
    let path = write_codex(&world, "0199mine", body);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": codex_reference(&world, "0199mine"),
        }),
    );
    let whole = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(
        whole.status.success(),
        "{}",
        String::from_utf8_lossy(&whole.stderr)
    );
    let document = read_document(&whole);
    assert_eq!(document["path"], path);
    assert_eq!(document["turns"].as_array().unwrap().len(), 4);
    assert_eq!(document["skipped_lines"], 1);
    let first = run(
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
    let document = read_document(&first);
    assert_eq!(document["turns"].as_array().unwrap().len(), 1);
    assert_eq!(document["turns"][0]["role"], "user");
    assert_eq!(document["turns"][0]["blocks"][0]["text"], "hello");
    assert_eq!(document["skipped_lines"], 1);
    let fourth = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "4",
        ],
    );
    let document = read_document(&fourth);
    assert_eq!(document["turn"], 4);
    assert_eq!(document["turns"].as_array().unwrap().len(), 1);
    assert_eq!(document["turns"][0]["blocks"][0]["kind"], "tool-result");
    assert_eq!(document["turns"][0]["blocks"][0]["text"], "ok");
}

/// Selecting a retained turn keeps the whole read's cap and unknown
/// notices; an index past the truncated prefix is `turn-not-retained`.
#[test]
fn selecting_a_retained_turn_keeps_the_whole_reads_notices() {
    let world = world_effects(&[("eff1", "review", None)]);
    let huge = "x".repeat(4_000_001);
    let body = format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"one\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"two\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"three\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{huge}\"}}}}\n\
         {{\"type\":\"future-record\"}}\n"
    );
    write_claude(&world, "abcd-1234", &body);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let whole = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(whole.status.success());
    let document = read_document(&whole);
    assert_eq!(document["truncated"], true);
    assert_eq!(document["turns"].as_array().unwrap().len(), 3);
    assert_eq!(document["unrecognized_records"], 1);
    let selected = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "2",
        ],
    );
    assert!(selected.status.success());
    let document = read_document(&selected);
    assert_eq!(document["turns"].as_array().unwrap().len(), 1);
    assert_eq!(document["turns"][0]["blocks"][0]["text"], "two");
    assert_eq!(document["truncated"], true);
    assert_eq!(document["unrecognized_records"], 1);
    assert_eq!(
        document["notices"],
        json!([
            "transcript truncated (size cap)",
            "unrecognized transcript records: 1",
        ])
    );
    let past = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "4",
        ],
    );
    assert!(!past.status.success());
    let document = read_document(&past);
    assert_eq!(document["unavailable"], "turn-not-retained");
    assert_eq!(document["truncated"], true);
    assert_eq!(document["turn"], 4);
}

/// A turn past a complete projection is `turn-not-retained`, never the
/// last available turn.
#[test]
fn a_turn_past_a_complete_projection_is_not_retained() {
    let world = world_effects(&[("eff1", "review", None)]);
    write_claude(
        &world,
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"one\"}}\n\
         {\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"two\"}}\n",
    );
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
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
            "999",
        ],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "turn-not-retained");
    assert_eq!(document["truncated"], false);
    assert!(document["turns"].as_array().unwrap().is_empty());
}

/// A packed DSH text-chunks row yields separately selectable turns, and
/// the equivalent ordinary chunk rows agree exactly.
#[test]
fn packed_dsh_members_have_separate_selectable_turns() {
    let world = world_effects(&[("eff1", "review", None)]);
    let packed = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[-1,5],\"texts\":[\"a\",\"b\",\"c\"]}}\n",
        "{\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"q\"}]},\"time\":2000}\n",
    );
    write_dsh_body(&world, "sessions/one", packed);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    let whole = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(whole.status.success());
    let document = read_document(&whole);
    assert_eq!(document["turns"].as_array().unwrap().len(), 4);
    let second = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "2",
        ],
    );
    let packed_second = read_document(&second);
    assert_eq!(packed_second["turns"][0]["blocks"][0]["text"], "b");
    assert_eq!(packed_second["turns"][0]["ts"], "999");
    let fourth = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "4",
        ],
    );
    let packed_fourth = read_document(&fourth);
    assert_eq!(packed_fourth["turns"][0]["blocks"][0]["text"], "q");
    assert_eq!(packed_fourth["turns"][0]["ts"], "2000");

    // The equivalent ordinary chunk rows.
    let world = world_effects(&[("eff1", "review", None)]);
    let ordinary = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":10,\"time\":1000,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"a\"}}}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":11,\"time\":999,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"b\"}}}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":12,\"time\":1004,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"c\"}}}\n",
        "{\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"q\"}]},\"time\":2000}\n",
    );
    write_dsh_body(&world, "sessions/one", ordinary);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    let ordinary = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "2",
        ],
    );
    assert_eq!(read_document(&ordinary)["turns"], packed_second["turns"]);
}

/// `--turn` rejects zero, negatives, non-integers and overflow before
/// any document exists.
#[test]
fn invalid_turn_arguments_are_usage_errors() {
    let world = world_effects(&[("eff1", "review", None)]);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": {"kind": "none", "locator": "", "home": ""},
        }),
    );
    for bad in ["0", "-1", "abc", "18446744073709551616"] {
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
                bad,
            ],
        );
        assert!(!output.status.success(), "turn {bad} is a usage error");
        assert!(output.stdout.is_empty(), "turn {bad} writes no stdout");
    }
}

// ------------------------------------------- 8.9 JSON and text states

/// The rejected common references a journal can carry keep their recorded
/// strings and fixed reasons, with no lookup. `unsupported-kind` is
/// structurally unreachable here: the frozen seat-record contract admits
/// only the four known kinds and the fold drops an unknown one before it
/// reaches the command, so the view's `validate_common` table owns that
/// branch.
#[test]
fn json_retains_every_rejected_common_reference() {
    for (reference, reason) in [
        (json!({"kind": "none", "locator": "", "home": ""}), "none"),
        (
            json!({"kind": "codex-thread", "locator": "", "home": "/retained/codex"}),
            "unannounced",
        ),
        (
            json!({"kind": "codex-thread", "locator": "0199mine", "home": ""}),
            "missing-home",
        ),
        (
            json!({"kind": "codex-thread", "locator": "-abc", "home": "/retained/codex"}),
            "invalid-reference",
        ),
    ] {
        let world = world_effects(&[("eff1", "review", None)]);
        checkpoint(
            &world,
            "eff1",
            "att0",
            json!({"step": "session-finished", "transcript": reference.clone()}),
        );
        let output = run(
            &world,
            &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
        );
        assert!(!output.status.success(), "{reason}");
        let document = read_document(&output);
        assert_eq!(document["unavailable"], reason);
        assert_eq!(document["transcript"], reference);
        assert_eq!(document["legacy"], false);
        assert!(document["path"].is_null());
        assert!(document["full_session"].is_null());
        assert!(document["turns"].as_array().unwrap().is_empty());
        assert_eq!(document["truncated"], false);
        assert_eq!(document["skipped_lines"], 0);
        assert_eq!(document["unrecognized_records"], 0);
        assert!(document["notices"].as_array().unwrap().is_empty());
    }
}

/// An empty valid file is readable; a missing file is `not-found` with
/// no fabricated path.
#[test]
fn json_distinguishes_an_empty_file_from_a_missing_one() {
    let world = world_effects(&[("eff1", "review", None)]);
    let path = write_claude(&world, "abcd-1234", "");
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let empty = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(empty.status.success());
    let document = read_document(&empty);
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert!(document["unavailable"].is_null());
    assert_eq!(document["path"], path);

    let world = world_effects(&[("eff1", "review", None)]);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let missing = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!missing.status.success());
    let document = read_document(&missing);
    assert_eq!(document["unavailable"], "not-found");
    assert!(document["path"].is_null());
    assert!(document["turns"].as_array().unwrap().is_empty());
}

/// Five ignorable unknown DSH records are a readable zero-turn source
/// with one counted notice, in JSON and text alike.
#[test]
fn five_ignorable_dsh_events_are_counted_once() {
    let world = world_effects(&[("eff1", "review", None)]);
    let mut body = String::from("{\"type\":\"session\",\"version\":0}\n");
    for index in 0..5 {
        body.push_str(&format!(
            "{{\"type\":\"future/{index}\",\"ignorable\":true}}\n"
        ));
    }
    write_dsh_body(&world, "sessions/one", &body);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(output.status.success());
    let document = read_document(&output);
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert!(document["unavailable"].is_null());
    assert_eq!(document["truncated"], false);
    assert_eq!(document["skipped_lines"], 0);
    assert_eq!(document["unrecognized_records"], 5);
    assert_eq!(
        document["notices"],
        json!(["unrecognized transcript records: 5"])
    );

    let text = run(&world, &["transcript", "--run", "r222", "--seat", "eff1"]);
    assert!(text.status.success());
    let body = String::from_utf8_lossy(&text.stdout);
    assert!(body.contains("no readable turns"), "{body}");
    assert!(
        body.contains("notice unrecognized transcript records: 5"),
        "{body}"
    );
}

/// The shipped Claude fixture keeps `(skipped_lines, unrecognized_records)
/// = (1, 0)` under whole and selected reads.
#[test]
fn the_shipped_claude_fixture_keeps_its_counts_under_selection() {
    let world = world_effects(&[("eff1", "review", None)]);
    write_claude(&world, "abcd-1234", SHIPPED_CLAUDE);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let whole = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(whole.status.success());
    let document = read_document(&whole);
    assert_eq!(document["skipped_lines"], 1);
    assert_eq!(document["unrecognized_records"], 0);
    assert_eq!(document["turns"].as_array().unwrap().len(), 2);
    assert_eq!(
        document["notices"],
        json!(["malformed transcript lines skipped: 1"])
    );
    let selected = run(
        &world,
        &[
            "transcript",
            "--run",
            "r222",
            "--seat",
            "eff1",
            "--json",
            "--turn",
            "2",
        ],
    );
    let document = read_document(&selected);
    assert_eq!(document["skipped_lines"], 1);
    assert_eq!(document["unrecognized_records"], 0);
    assert_eq!(document["turns"].as_array().unwrap().len(), 1);
    assert_eq!(document["turns"][0]["role"], "user");
    assert_eq!(
        document["notices"],
        json!(["malformed transcript lines skipped: 1"])
    );
}

/// Claude lookup refusals keep the non-null shared Claude hint and zero
/// counts.
#[test]
fn claude_lookup_refusals_keep_the_hint() {
    // Two qualifying files are ambiguous.
    let world = world_effects(&[("eff1", "review", None)]);
    let reference = claude_reference(&world, "abcd-1234");
    let body = "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"x\"}}\n";
    for project in ["one", "two"] {
        let dir = world.projects().join(project);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("abcd-1234.jsonl"), body).unwrap();
    }
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({"step": "session-finished", "transcript": reference.clone()}),
    );
    let ambiguous = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!ambiguous.status.success());
    let document = read_document(&ambiguous);
    assert_eq!(document["unavailable"], "ambiguous-source");
    assert!(document["path"].is_null());
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert_eq!(document["skipped_lines"], 0);
    assert_eq!(document["unrecognized_records"], 0);
    assert_eq!(
        document["full_session"],
        "full session: claude --resume abcd-1234"
    );

    // A spent entry bound outranks a provisional match.
    let world = world_effects(&[("eff1", "review", None)]);
    let projects = world.projects();
    std::fs::create_dir_all(projects.join("winning")).unwrap();
    std::fs::write(projects.join("winning/abcd-1234.jsonl"), body).unwrap();
    for index in 0..10_001 {
        std::fs::write(projects.join(format!("filler-{index:05}")), b"x").unwrap();
    }
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({"step": "session-finished", "transcript": claude_reference(&world, "abcd-1234")}),
    );
    let limited = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!limited.status.success());
    let document = read_document(&limited);
    assert_eq!(document["unavailable"], "discovery-limit");
    assert!(document["path"].is_null());
    assert_eq!(
        document["full_session"],
        "full session: claude --resume abcd-1234"
    );
}

/// A symlink-only Claude candidate is `unsafe-path` with the shared hint.
#[cfg(unix)]
#[test]
fn a_symlink_only_claude_candidate_is_unsafe_path() {
    let world = world_effects(&[("eff1", "review", None)]);
    let project = world.projects().join("one");
    std::fs::create_dir_all(&project).unwrap();
    let outside = world.path().join("outside.jsonl");
    std::fs::write(
        &outside,
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"x\"}}\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(&outside, project.join("abcd-1234.jsonl")).unwrap();
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": claude_reference(&world, "abcd-1234"),
        }),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "unsafe-path");
    assert!(document["path"].is_null());
    assert_eq!(
        document["full_session"],
        "full session: claude --resume abcd-1234"
    );
}

/// An explicit Codex provenance never synthesizes a Claude reference,
/// even when a matching local Claude file exists.
#[test]
fn a_legacy_codex_participant_is_unavailable_without_a_reference() {
    let world = world_effects(&[("eff1", "review", Some("codex"))]);
    write_claude(
        &world,
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"x\"}}\n",
    );
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "claude-session-finished",
            "session_id": "abcd-1234",
        }),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "no-reference");
    assert!(document["transcript"].is_null());
    assert_eq!(document["legacy"], false);
    assert!(document["path"].is_null());
    assert!(document["full_session"].is_null());
    assert!(document["turns"].as_array().unwrap().is_empty());
}

/// The guarded pre-0032 fallback is visible as `legacy: true` with the
/// effective synthesized reference.
#[test]
fn legacy_synthesis_is_visible() {
    let world = world_effects(&[("eff1", "review", None)]);
    let path = write_claude(
        &world,
        "abcd-1234",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"legacy\"}}\n",
    );
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "claude-session-finished",
            "session_id": "abcd-1234",
        }),
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
    let document = read_document(&output);
    assert_eq!(document["legacy"], true);
    assert_eq!(document["transcript"]["kind"], "claude-session");
    assert_eq!(document["transcript"]["locator"], "abcd-1234");
    assert_eq!(document["path"], path);
}

/// A DSH event refusal keeps the confirmed path and all three ordered
/// notices; text mode refuses on stderr without earlier prose.
#[test]
fn dsh_event_refusal_carries_all_three_notices() {
    let world = world_effects(&[("eff1", "review", None)]);
    let mut body = String::from(
        "{\"type\":\"session\",\"version\":0}\nnot json\nalso not json\n\
         {\"type\":\"future/a\"}\n{\"type\":\"future/b\"}\n",
    );
    body.push_str(&"x".repeat(34_000_000));
    body.push('\n');
    let path = write_dsh_body(&world, "sessions/one", &body);
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "unsupported-format");
    assert_eq!(document["path"], path);
    assert_eq!(
        document["full_session"],
        format!("full session: \"{path}\"")
    );
    assert_eq!(document["truncated"], true);
    assert_eq!(document["skipped_lines"], 2);
    assert_eq!(document["unrecognized_records"], 2);
    assert_eq!(
        document["notices"],
        json!([
            "transcript truncated (size cap)",
            "malformed transcript lines skipped: 2",
            "unrecognized transcript records: 2",
        ])
    );
    assert!(document["turns"].as_array().unwrap().is_empty());

    let text = run(&world, &["transcript", "--run", "r222", "--seat", "eff1"]);
    assert!(!text.status.success());
    assert!(text.stdout.is_empty(), "refusals write no stdout body");
    let stderr = String::from_utf8_lossy(&text.stderr);
    assert!(stderr.contains("unsupported-format"), "{stderr}");
    assert!(
        stderr.contains("DSH transcript format is not supported"),
        "{stderr}"
    );
    assert!(
        stderr.contains("transcript truncated (size cap)"),
        "{stderr}"
    );
}

/// A rejected DSH header version keeps the confirmed path and hint, zero
/// counts and empty notices, whatever the requested turn.
#[test]
fn a_rejected_dsh_header_version_has_fixed_documents() {
    let world = world_effects(&[("eff1", "review", None)]);
    let path = write_dsh_body(
        &world,
        "sessions/one",
        "{\"type\":\"session\",\"version\":1}\n\
         {\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"x\"}]}}\n",
    );
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    for turn in [None, Some("1"), Some("999")] {
        let mut args = vec!["transcript", "--run", "r222", "--seat", "eff1", "--json"];
        if let Some(turn) = turn {
            args.push("--turn");
            args.push(turn);
        }
        let output = run(&world, &args);
        assert!(!output.status.success());
        let document = read_document(&output);
        assert_eq!(document["unavailable"], "unsupported-format");
        assert_eq!(document["path"], path);
        assert_eq!(
            document["full_session"],
            format!("full session: \"{path}\"")
        );
        assert_eq!(document["truncated"], false);
        assert_eq!(document["skipped_lines"], 0);
        assert_eq!(document["unrecognized_records"], 0);
        assert!(document["notices"].as_array().unwrap().is_empty());
        assert!(document["turns"].as_array().unwrap().is_empty());
        match turn {
            None => assert!(document["turn"].is_null()),
            Some(turn) => assert_eq!(document["turn"], turn.parse::<u64>().unwrap()),
        }
    }
}

/// Discovery ambiguity outranks header support: two owned roots, one
/// version zero and one version one, are `ambiguous-source`.
#[test]
fn dsh_format_support_cannot_select_between_roots() {
    let world = world_effects(&[("eff1", "review", None)]);
    for (project, version) in [("zero", 0), ("one", 1)] {
        let session = world.home.join("sessions/one").join(project).join("seat");
        std::fs::create_dir_all(&session).unwrap();
        std::fs::write(
            session.join("session.jsonl"),
            format!("{{\"type\":\"session\",\"version\":{version}}}\n"),
        )
        .unwrap();
    }
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "ambiguous-source");
    assert!(document["path"].is_null());
    assert!(document["full_session"].is_null());
    assert!(document["turns"].as_array().unwrap().is_empty());
    assert_eq!(document["truncated"], false);
    assert!(document["notices"].as_array().unwrap().is_empty());
    let text = run(&world, &["transcript", "--run", "r222", "--seat", "eff1"]);
    assert!(!text.status.success());
    assert!(text.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&text.stderr).contains("ambiguous"),
        "{}",
        String::from_utf8_lossy(&text.stderr)
    );
}

/// A valid packed row's three members do not multiply an invalid packed
/// row's single diagnostic row.
#[test]
fn one_invalid_packed_row_counts_once() {
    let world = world_effects(&[("eff1", "review", None)]);
    let path = write_dsh_body(
        &world,
        "sessions/one",
        concat!(
            "{\"type\":\"session\",\"version\":0}\n",
            "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[-1,5],\"texts\":[\"a\",\"b\",\"c\"]}}\n",
            "{\"type\":\"text-chunks\",\"seq0\":20,\"time0\":2000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[5],\"texts\":[\"a\",\"b\",\"c\"]}}\n",
        ),
    );
    checkpoint(
        &world,
        "eff1",
        "att0",
        json!({
            "step": "session-finished",
            "transcript": dsh_reference(&world, "sessions/one"),
        }),
    );
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "unsupported-format");
    assert_eq!(document["path"], path);
    assert_eq!(
        document["full_session"],
        format!("full session: \"{path}\"")
    );
    assert_eq!(document["skipped_lines"], 0);
    assert_eq!(document["unrecognized_records"], 1);
    assert!(document["turns"].as_array().unwrap().is_empty());
}

/// JSON mode still writes the sanitized refusal explanation to stderr,
/// beside the complete document on stdout.
#[test]
fn json_mode_still_writes_the_sanitized_stderr_explanation() {
    let world = world_effects(&[("eff1", "review", None)]);
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "eff1", "--json"],
    );
    assert!(!output.status.success());
    let document = read_document(&output);
    assert_eq!(document["unavailable"], "no-reference");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("transcript unavailable: no-reference"),
        "JSON mode must still explain the refusal on stderr: {stderr:?}"
    );
}

/// An ambiguous label sanitizes every candidate key it names, not only
/// the requested label.
#[test]
fn an_ambiguous_label_sanitizes_every_candidate_key() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("r222", "feat", "self", &json!({"files": {}}))
        .unwrap();
    let none = json!({"kind": "none", "locator": "", "home": ""});
    let events: Vec<(EventType, Value)> = vec![
        (
            EventType::RunStarted,
            json!({"feature": "feat", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "intake"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff\u{1b}[31mred", "seat": "same-label",
                   "phase": "intake"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff\u{1b}[31mred", "attempt_id": "att0"}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff\u{1b}[31mred", "attempt_id": "att0",
                   "checkpoint": {"step": "session-finished", "transcript": none}}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "eff\u{1b}[31mred", "attempt_id": "att0",
                   "result": {"result": "intook"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"rule_id": "INTAKE-OK", "severity": "normal",
                   "from": "intake", "next": "design", "result": "intook"}),
        ),
        (EventType::PhaseEntered, json!({"phase": "design"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff2", "seat": "same-label", "phase": "design"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff2", "attempt_id": "att1"}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff2", "attempt_id": "att1",
                   "checkpoint": {"step": "session-finished", "transcript": none}}),
        ),
    ];
    for (event_type, payload) in events {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    let world = World { dir, db, home };
    let output = run(
        &world,
        &["transcript", "--run", "r222", "--seat", "same-label"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous"), "{stderr:?}");
    assert!(
        !stderr.contains('\u{1b}'),
        "a candidate key's terminal control must be sanitized: {stderr:?}"
    );
    assert!(
        stderr.contains("eff") && stderr.contains("eff2"),
        "{stderr:?}"
    );
}
