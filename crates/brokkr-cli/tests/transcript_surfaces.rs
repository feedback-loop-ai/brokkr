//! One local derivation across the surfaces (#222, proposed decision
//! 0055; design D11). A synthetic source of each kind is read once
//! through `brokkr_cli::read_local`; the command's text and JSON, the
//! TUI pane keys and both doors, Claude's `/api/session/<id>` body and
//! the browser participant presentation are then compared against that
//! one result. The browser transport is asserted prose-free.

use std::path::{Path, PathBuf};
use std::process::Command;

use brokkr_core::envelope::EventType;
use brokkr_store::Store;
use brokkr_view::transcript::{LegacyProvenance, TranscriptRead};
use serde_json::{json, Value};

struct World {
    dir: tempfile::TempDir,
    db: PathBuf,
    home: PathBuf,
    reference: brokkr_view::Transcript,
    path: String,
}

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

/// Materialize one synthetic source of `kind` under `home` and return the
/// recorded reference and the confirmed path.
fn source(home: &Path, kind: &str, locator: &str, body: &str) -> (brokkr_view::Transcript, String) {
    match kind {
        "claude-session" => {
            let projects = home.join(".claude").join("projects");
            let dir = projects.join("project");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join(format!("{locator}.jsonl"));
            std::fs::write(&file, body).unwrap();
            (
                brokkr_view::Transcript {
                    kind: kind.to_string(),
                    locator: locator.to_string(),
                    home: projects.to_str().unwrap().to_string(),
                },
                file.to_str().unwrap().to_string(),
            )
        }
        "codex-thread" => {
            let dir = home.join("sessions");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join(format!("rollout-{locator}.jsonl"));
            std::fs::write(&file, body).unwrap();
            (
                brokkr_view::Transcript {
                    kind: kind.to_string(),
                    locator: locator.to_string(),
                    home: home.to_str().unwrap().to_string(),
                },
                file.to_str().unwrap().to_string(),
            )
        }
        "dsh-session" => {
            let dir = home.join(locator).join("project").join("seat");
            std::fs::create_dir_all(&dir).unwrap();
            let file = dir.join("session.jsonl");
            std::fs::write(&file, body).unwrap();
            (
                brokkr_view::Transcript {
                    kind: kind.to_string(),
                    locator: locator.to_string(),
                    home: home.to_str().unwrap().to_string(),
                },
                file.to_str().unwrap().to_string(),
            )
        }
        other => panic!("unknown kind {other}"),
    }
}

/// A one-participant run whose checkpoint carries the reference.
fn make_world(kind: &str, locator: &str, body: &str) -> World {
    make_world_named(kind, locator, body, "home")
}

/// The same world under a chosen (possibly hostile) home directory name.
fn make_world_named(kind: &str, locator: &str, body: &str, home_name: &str) -> World {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let home = dir.path().join(home_name);
    std::fs::create_dir_all(&home).unwrap();
    let (reference, path) = source(&home, kind, locator, body);
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
            json!({"effect_id": "eff1", "attempt_id": "att0"}),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff1", "attempt_id": "att0",
                   "checkpoint": {"step": "session-finished",
                                  "transcript": reference}}),
        ),
    ];
    for (event_type, payload) in events {
        store
            .append_next("r222", event_type, payload, None, None)
            .unwrap();
    }
    World {
        dir,
        db,
        home,
        reference,
        path,
    }
}

fn command(world: &World, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["transcript", "--run", "r222", "--seat", "eff1"];
    args.extend_from_slice(extra);
    Command::new(brokkr_bin())
        .args(args)
        .arg("--db")
        .arg(&world.db)
        .env("HOME", &world.home)
        .current_dir(world.dir.path())
        .output()
        .unwrap()
}

fn parse(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "stdout is not one JSON document: {error}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Compare every surface against one shared read, for one source.
fn compare(world: &World, read: &TranscriptRead, selected: Option<usize>) {
    // The API's Claude body agrees on turns and truncation and keeps its
    // three-field envelope.
    if world.reference.kind == "claude-session" {
        let response = brokkr_cli::handle(
            &world.db,
            &format!("/api/session/{}", world.reference.locator),
        );
        assert_eq!(response.status, "200 OK");
        let body: Value = serde_json::from_str(&response.body).unwrap();
        let mut keys: Vec<&str> = body
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["session_id", "truncated", "turns"]);
        assert_eq!(body["turns"], serde_json::to_value(&read.turns).unwrap());
        assert_eq!(body["truncated"], read.truncated);
    }

    // The browser participant presentation carries only the shared
    // metadata, never transcript prose.
    let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
    assert_eq!(response.status, "200 OK");
    let presentation: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(presentation["admitted"], read.is_readable());
    assert_eq!(presentation["path"], json!(read.path));
    assert_eq!(presentation["hint"], json!(read.full_session));
    assert_eq!(
        presentation["reason"],
        json!(read.unavailable.map(|reason| reason.as_str()))
    );
    assert!(presentation.get("turns").is_none(), "{presentation}");
    assert!(presentation.get("blocks").is_none(), "{presentation}");
    for turn in &read.turns {
        for block in &turn.blocks {
            assert!(
                !response.body.contains(&block.text),
                "the browser transport must not carry prose: {presentation}"
            );
        }
    }

    // The TUI pane enumerates the same turns; both doors carry the same
    // turn text, notices and hint through the real renderers.
    let (keys, selected_door, whole_door) =
        brokkr_cli::transcript_surfaces_for_test(read, selected);
    assert_eq!(keys.len(), read.turns.len());
    for index in 0..read.turns.len() {
        assert!(keys.contains(&index.to_string()));
    }
    for turn in &read.turns {
        for block in &turn.blocks {
            assert!(whole_door.contains(&block.text), "{whole_door}");
        }
    }
    for notice in &read.notices {
        assert!(whole_door.contains(notice), "{whole_door}");
    }
    if let Some(hint) = &read.full_session {
        assert!(whole_door.contains(hint), "{whole_door}");
    }
    if let Some(index) = selected.filter(|index| *index < read.turns.len()) {
        for block in &read.turns[index].blocks {
            assert!(selected_door.contains(&block.text), "{selected_door}");
        }
        for notice in &read.notices {
            assert!(selected_door.contains(notice), "{selected_door}");
        }
    }

    // The whole command agrees with the same fields.
    let whole = command(world, &["--json"]);
    assert!(
        whole.status.success(),
        "{}",
        String::from_utf8_lossy(&whole.stderr)
    );
    let document = parse(&whole);
    assert_eq!(
        document["turns"],
        serde_json::to_value(&read.turns).unwrap()
    );
    assert_eq!(document["truncated"], read.truncated);
    assert_eq!(document["skipped_lines"], read.skipped_lines);
    assert_eq!(document["unrecognized_records"], read.unrecognized_records);
    assert_eq!(
        document["notices"],
        serde_json::to_value(&read.notices).unwrap()
    );
    assert_eq!(document["path"], json!(read.path));
    assert_eq!(document["full_session"], json!(read.full_session));

    // The selected command changes only the turn.
    if let Some(index) = selected.filter(|index| *index < read.turns.len()) {
        let selected_command = command(world, &["--json", "--turn", &(index + 1).to_string()]);
        assert!(selected_command.status.success());
        let document = parse(&selected_command);
        assert_eq!(
            document["turns"],
            serde_json::to_value(vec![read.turns[index].clone()]).unwrap()
        );
        assert_eq!(document["turn"], index + 1);
        assert_eq!(document["truncated"], read.truncated);
        assert_eq!(
            document["notices"],
            serde_json::to_value(&read.notices).unwrap()
        );
    }
}

/// One synthetic source of each kind flows through every surface, plus a
/// readable zero-turn source, a truncated source, a counted omission and
/// a DSH header refusal.
#[test]
fn one_derivation_reaches_every_surface() {
    for (kind, locator, body) in [
        (
            "claude-session",
            "abcd-1234",
            "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"claude says hi\"}}\n",
        ),
        (
            "codex-thread",
            "0199mine",
            "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"codex asks\"}]}}\n",
        ),
        (
            "dsh-session",
            "sessions/one",
            "{\"type\":\"session\",\"version\":0}\n\
             {\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"dsh asks\"}]},\"time\":1000}\n",
        ),
    ] {
        let world = make_world(kind, locator, body);
        // The in-process handler and the child process must agree on the
        // local projects root for the Claude drill.
        std::env::set_var("HOME", &world.home);
        let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
        assert!(read.is_readable(), "{kind}: {read:?}");
        assert_eq!(read.path.as_deref(), Some(world.path.as_str()));
        let selected = (read.turns.len() >= 2).then_some(1);
        compare(&world, &read, selected);
    }

    // A readable zero-turn source.
    let world = make_world("codex-thread", "0199zero", "{\"type\":\"turn_context\"}\n");
    std::env::set_var("HOME", &world.home);
    let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
    assert!(read.is_readable() && read.turns.is_empty());
    compare(&world, &read, None);

    // A truncated source: three retained turns, then one over the display
    // budget, and one counted unknown record.
    let huge = "x".repeat(4_000_001);
    let body = format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"one\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"two\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"three\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{huge}\"}}}}\n\
         {{\"type\":\"future-record\"}}\n"
    );
    let world = make_world("claude-session", "abcd-1234", &body);
    std::env::set_var("HOME", &world.home);
    let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
    assert!(read.truncated && read.unrecognized_records == 1, "{read:?}");
    compare(&world, &read, Some(1));

    // A safe counted omission that still reads.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"version\":0}\n\
         {\"type\":\"future/event\",\"ignorable\":true}\n",
    );
    std::env::set_var("HOME", &world.home);
    let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
    assert!(
        read.is_readable() && read.unrecognized_records == 1,
        "{read:?}"
    );
    compare(&world, &read, None);

    // A DSH header-version refusal is a body-stage refusal: the browser
    // presentation's discovery-stage facts stay honest while no surface
    // invents prose, and the command refuses.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"version\":1}\n\
         {\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"hidden\"}]}}\n",
    );
    std::env::set_var("HOME", &world.home);
    let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
    assert_eq!(
        read.unavailable,
        Some(brokkr_view::transcript::Unavailable::UnsupportedFormat)
    );
    let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
    let presentation: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(presentation["admitted"], true, "discovery owns the file");
    assert_eq!(presentation["path"], json!(read.path));
    assert_eq!(presentation["hint"], json!(read.full_session));
    assert!(presentation["reason"].is_null());
    assert!(!response.body.contains("hidden"), "{presentation}");
    let whole = command(&world, &["--json"]);
    assert!(!whole.status.success());
    let document = parse(&whole);
    assert_eq!(document["unavailable"], "unsupported-format");
    assert_eq!(document["turns"], json!([]));
    assert_eq!(document["notices"], json!([]));

    // A DSH ownership refusal is a lookup refusal every surface shares.
    let world = make_world(
        "dsh-session",
        "sessions/one",
        "{\"type\":\"session\",\"delegationDepth\":1,\"version\":0}\n",
    );
    std::env::set_var("HOME", &world.home);
    let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
    assert_eq!(
        read.unavailable,
        Some(brokkr_view::transcript::Unavailable::NotFound)
    );
    let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
    let presentation: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(presentation["admitted"], false);
    assert_eq!(presentation["reason"], "not-found");
    assert_eq!(
        presentation["explanation"],
        "no valid depth-zero DSH session header"
    );
    assert!(presentation["path"].is_null());
    assert!(presentation.get("turns").is_none(), "{presentation}");
    let whole = command(&world, &["--json"]);
    assert!(!whole.status.success());
    let document = parse(&whole);
    assert_eq!(document["unavailable"], "not-found");
    assert_eq!(document["turns"], json!([]));
}

/// Distinct recorded tool ids and the measured MCP/dynamic context reach
/// every surface in the one centralized block text (design D5/D11).
#[test]
fn recorded_tool_identity_and_context_reach_every_surface() {
    let body = concat!(
        "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"F\",\"call_id\":\"c1\",\"arguments\":\"{}\"}}\n",
        "{\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"F\",\"call_id\":\"c2\",\"arguments\":\"{}\"}}\n",
        "{\"timestamp\":\"t3\",\"type\":\"event_msg\",\"payload\":{\"type\":\"mcp_tool_call_begin\",\"call_id\":\"m1\",\"invocation\":{\"server\":\"srv\",\"tool\":\"search\",\"arguments\":\"{}\"}}}\n",
        "{\"timestamp\":\"t4\",\"type\":\"event_msg\",\"payload\":{\"type\":\"dynamic_tool_call_response\",\"call_id\":\"d1\",\"content_items\":[{\"type\":\"inputText\",\"text\":\"done\"}],\"error\":\"boom\"}}\n",
    );
    let world = make_world("codex-thread", "0199mine", body);
    // The recorded common reference needs no ambient HOME, so this test
    // never mutates the process environment another test may be reading.
    let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
    assert!(read.is_readable(), "{read:?}");
    let texts: Vec<String> = read
        .turns
        .iter()
        .flat_map(|turn| turn.blocks.iter().map(|block| block.text.clone()))
        .collect();
    for needle in ["[c1]", "[c2]", "[m1]", "[d1]"] {
        assert!(
            texts.iter().any(|text| text.contains(needle)),
            "the recorded id {needle} is missing: {texts:?}"
        );
    }
    assert!(
        texts
            .iter()
            .any(|text| text.contains("srv") && text.contains("search")),
        "the MCP context is missing: {texts:?}"
    );
    assert!(
        texts
            .iter()
            .any(|text| text.contains("done") && text.contains("boom")),
        "the dynamic response context is missing: {texts:?}"
    );
    compare(&world, &read, None);
}

/// R25: one completed portable-display hint is byte-identical across the
/// view result, the command's text and decoded JSON, the TUI pane and whole
/// door, the presentation response and the served page's text node — with no
/// surface reconstructing a fragment.
#[test]
fn r25_portable_hint_is_identical_across_every_surface() {
    let hostile = "home $(x) `t` ;a&b|c<d>e%f!g \"q\" \\ é😀";
    for (kind, locator, body) in [
        ("codex-thread", "0199mine", "{\"type\":\"turn_context\"}\n"),
        (
            "dsh-session",
            "sessions/one",
            "{\"type\":\"session\",\"version\":0}\n",
        ),
    ] {
        let world = make_world_named(kind, locator, body, hostile);
        // The recorded common reference needs no ambient HOME, so this test
        // never mutates the process environment another test may be reading.
        let read = brokkr_cli::read_local(Some(&world.reference), LegacyProvenance::Absent, None);
        assert!(read.is_readable(), "{kind}: {read:?}");
        let hint = read
            .full_session
            .clone()
            .expect("a confirmed path has a hint");
        for raw in ["$(", "`", ";", "&", "|", "<", ">", "%", "!", "é", "😀"] {
            assert!(
                !hint.contains(raw),
                "{kind}: {raw:?} survived raw in {hint:?}"
            );
        }
        assert!(hint.contains("\\u0020"), "{kind}: {hint}");
        assert!(hint.contains("\\ud83d\\ude00"), "{kind}: {hint}");

        // The presentation transports the exact decoded hint with no prose.
        let response = brokkr_cli::handle(&world.db, "/api/presentation/r222/eff1");
        assert_eq!(response.status, "200 OK");
        let presentation: Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(presentation["hint"], json!(read.full_session), "{kind}");
        assert!(presentation.get("turns").is_none(), "{presentation}");

        // The TUI pane and whole door carry the same completed value; the
        // command's text face does too; its decoded JSON recovers it exactly
        // while the raw document carries the doubled outer escaping.
        let (_, _, whole) = brokkr_cli::transcript_surfaces_for_test(&read, None);
        assert!(whole.contains(&hint), "{kind}: {whole}");
        let text = command(&world, &[]);
        assert!(text.status.success());
        let text = String::from_utf8_lossy(&text.stdout);
        assert!(text.contains(&hint), "{kind}: {text}");
        let json_output = command(&world, &["--json"]);
        let document = parse(&json_output);
        assert_eq!(document["full_session"], json!(read.full_session), "{kind}");
        let raw = String::from_utf8_lossy(&json_output.stdout);
        assert!(raw.contains("\\\\u0020"), "{kind}: {raw}");
        assert!(raw.contains("\\\\ud83d\\\\ude00"), "{kind}: {raw}");

        // The served page paints the transported hint through `el`'s
        // textContent; no route or page quotes a path/home fragment.
        let page = brokkr_cli::handle(&world.db, "/");
        assert!(
            page.body.contains("el('p', 'cause', view.hint)"),
            "the page paints the shared hint verbatim"
        );
    }
}
