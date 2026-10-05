//! The codex seat's items at every seat rendering (#550): one journal
//! planted the way the driver writes it, read back through `brokkr
//! seats`, `brokkr inspect`, the console's `--json` and the TUI's own
//! drawn frame. One derived cell everywhere — and a seat-turn journal
//! (claude, dsh) reading exactly as it did before.

use std::path::Path;

use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::boundary_readouts::brokkr;

/// One seat planted as the engine records it, `checkpoints` emitted
/// verbatim under its attempt: the journal both the verbs and the TUI
/// frame read below.
fn plant(db: &Path, run_id: &str, feature: &str, checkpoints: Vec<Value>) {
    let mut journal: Vec<(EventType, Value)> = vec![
        (
            EventType::RunStarted,
            json!({"feature": feature, "manifest": {"engine": "0.9.0"}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "verify"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "verify", "phase": "verify"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "a1", "driver": "d"}),
        ),
    ];
    journal.extend(checkpoints.into_iter().map(|checkpoint| {
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "e1", "attempt_id": "a1", "checkpoint": checkpoint}),
        )
    }));
    journal.push((
        EventType::EffectSucceeded,
        json!({"effect_id": "e1", "attempt_id": "a1", "result": {"result": "pass"}}),
    ));
    let mut store = Store::open(db).unwrap();
    store
        .create_run(run_id, feature, "test", &json!({"engine": "0.9.0"}))
        .unwrap();
    for (kind, payload) in journal {
        store
            .append_next(run_id, kind, payload, None, None)
            .unwrap();
    }
}

/// The work inside one codex turn, as the driver journals it: the turn
/// opens, three items complete carrying their `tool` types, the turn
/// closes, the thread closes.
fn codex_checkpoints() -> Vec<Value> {
    let mut checkpoints = vec![json!({"step": "turn-started", "turn": 1, "harness": "codex"})];
    for tool in ["command_execution", "reasoning", "agent_message"] {
        checkpoints
            .push(json!({"step": "item-completed", "turn": 1, "tool": tool, "harness": "codex"}));
    }
    checkpoints.push(json!({"step": "turn-completed", "turn": 1, "harness": "codex"}));
    checkpoints
        .push(json!({"step": "codex-session-finished", "exit_code": 0, "session_id": "01a"}));
    checkpoints
}

/// The seats block alone, as the verb prints it.
fn seats_text(dir: &Path, db: &str, run: &str) -> String {
    let (out, _, ok) = brokkr(dir, &["seats", "--run", run, "--db", db]);
    assert!(ok, "{out}");
    out
}

/// The seats verb's `--json` face: the same view, parsed.
fn seats_json(dir: &Path, db: &str, run: &str) -> Value {
    let (out, _, ok) = brokkr(dir, &["seats", "--run", run, "--db", db, "--json"]);
    assert!(ok, "{out}");
    serde_json::from_str(&out).unwrap()
}

#[test]
fn a_codex_seats_items_stand_beside_its_turn_in_every_seat_rendering() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    plant(&db, "codexed", "items beside the turn", codex_checkpoints());
    let db = db.to_str().unwrap();

    // The view model: the exact cell, derived once. Every rendering
    // below paints it.
    let view = seats_json(dir.path(), db, "codexed");
    assert_eq!(view["participants"][0]["turns"], 1);
    assert_eq!(view["participants"][0]["turns_cell"]["text"], "1 · 3 items");
    assert_eq!(view["participants"][0]["turns_cell"]["absent"], false);

    let seats = seats_text(dir.path(), db, "codexed");
    assert!(seats.contains(" 1 · 3 items "), "{seats}");

    // `brokkr inspect` prints the seats block `seats` prints.
    let (inspect, _, ok) = brokkr(dir.path(), &["inspect", "--run", "codexed", "--db", db]);
    assert!(ok, "{inspect}");
    assert!(inspect.contains(" 1 · 3 items "), "{inspect}");

    // The TUI's seat table, drawn by the console's own draw path.
    let events = Store::open(Path::new(db)).unwrap().load("codexed").unwrap();
    let frame = brokkr_cli::run_frame_for_budget(&events, "2026-01-01T00:07:03Z", 160, 48);
    let shown: String = (0..frame.area.height)
        .map(|row| {
            (0..frame.area.width)
                .map(|column| frame[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(shown.contains(" 1 · 3 items "), "{shown}");
}

#[test]
fn a_seat_turn_journal_reads_exactly_as_it_did_before() {
    // Claude and dsh count turns and journal no items: their cell is
    // the turn count alone, in the JSON and in the painted block.
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    plant(
        &db,
        "turncounted",
        "turns alone",
        vec![
            json!({"step": "seat-turn", "turn": 1, "tool": "Read"}),
            json!({"step": "claude-session-finished", "exit_code": 0}),
        ],
    );
    let db = db.to_str().unwrap();

    let view = seats_json(dir.path(), db, "turncounted");
    assert_eq!(view["participants"][0]["turns"], 1);
    assert_eq!(view["participants"][0]["turns_cell"]["text"], "1");

    assert!(!seats_text(dir.path(), db, "turncounted").contains("items"));
}
