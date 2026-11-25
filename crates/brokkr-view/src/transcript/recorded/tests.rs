use serde_json::{json, Value};

use super::super::{project, Block, Projection, Snapshot, TranscriptKind};
use super::timestamp;

const VALID_TIMESTAMP: &str = "2026-01-01T00:00:01Z";
const BOUND_VALUE: &str = "ghp-bound-7f3a9c";
const READERS: [TranscriptKind; 2] = [TranscriptKind::ClaudeSession, TranscriptKind::CodexThread];

#[test]
fn recorded_timestamps_require_the_rfc_3339_shape() {
    for word in [
        VALID_TIMESTAMP,
        "2026-01-01T00:00:01+02:30",
        "2026-01-01T00:00:01.123Z",
        "2026-01-01T00:00:01.1Z",
        "2026-01-01T00:00:01.123456789-02:30",
        "2026-99-99T99:99:99Z",
    ] {
        assert!(timestamp(word), "refused {word:?}");
    }

    for word in [
        "",
        "2026-01-01T00:00:0",
        "2026-01-01T00:00:01",
        "2026-01-01T00:00:01ZZ",
        "2026-01-01T00:00:01.123456789-02:30Z",
        "2026-01-01T00:00:0aZ",
        "2026-01-01 00:00:01Z",
        "2026-01-01t00:00:01Z",
        "2026-01-01T00:00:01z",
        "2026-01-01T00:00:01.Z",
        "2026-01-01T00:00:01.1234567890Z",
        "2026-01-01T00:00:01+02:3",
        "2026-01-01T00:00:01-02:300",
        "2026-01-01T00:00:01+02;30",
        "2026-01-01T00:00:01-0a:30",
        "2026-01-01T00:00:01\u{1b}",
        "2026-01-01T00:00:0\u{661}Z",
        "2026-01-01T00:00:01\u{202e}",
    ] {
        assert!(!timestamp(word), "admitted {word:?}");
    }
}

#[test]
fn recorded_roles_use_the_closed_harness_vocabulary() {
    for kind in READERS {
        for word in ["user", "assistant", "tool", "developer", "system"] {
            let projection = project_prose(kind, None, word);
            assert_eq!(projection.turns[0].role, word, "{kind:?}");
            assert_eq!(projection.unrecognized_records, 0, "{kind:?} {word}");
        }
        for word in [
            "",
            "System_2-alpha",
            &"a".repeat(33),
            "two words",
            "tool/system",
            "tool\u{1b}",
            "tool\u{661}",
            "tool\u{202e}",
        ] {
            let projection = project_prose(kind, None, word);
            assert_eq!(projection.turns[0].role, "unknown", "{kind:?} {word:?}");
            assert_eq!(projection.unrecognized_records, 1, "{kind:?} {word:?}");
        }
    }
}

fn project_message(
    kind: TranscriptKind,
    timestamp: Option<Value>,
    role: &str,
    parts: impl FnOnce(&str) -> Vec<Value>,
) -> Projection {
    let mut row = match kind {
        TranscriptKind::ClaudeSession => json!({
            "type": "assistant",
            "message": {"role": role, "content": parts("text")},
        }),
        TranscriptKind::CodexThread => json!({
            "type": "response_item",
            "payload": {"type": "message", "role": role, "content": parts("output_text")},
        }),
        TranscriptKind::DshSession | TranscriptKind::None => panic!("unexpected reader {kind:?}"),
    };
    if let Some(timestamp) = timestamp {
        row["timestamp"] = timestamp;
    }
    project_rows(kind, &format!("{row}\n"))
}

fn project_rows(kind: TranscriptKind, body: &str) -> Projection {
    project(
        kind,
        &Snapshot {
            bytes: body.as_bytes(),
            overflow: false,
            eof: true,
        },
    )
}

fn project_prose(kind: TranscriptKind, timestamp: Option<Value>, role: &str) -> Projection {
    project_message(kind, timestamp, role, |part| {
        vec![json!({"type": part, "text": "x"})]
    })
}

fn assert_projected_timestamps(kind: TranscriptKind) {
    for (input, expected, count) in [
        (Some(json!(BOUND_VALUE)), "", 1),
        (Some(json!("")), "", 1),
        (None, "", 0),
        (Some(json!(42)), "", 0),
        (Some(json!(VALID_TIMESTAMP)), VALID_TIMESTAMP, 0),
    ] {
        let projection = project_prose(kind, input, "assistant");
        assert_eq!(projection.turns.len(), 1);
        assert_eq!(projection.turns[0].ts, expected, "{kind:?} timestamp");
        assert_eq!(projection.unrecognized_records, count, "{kind:?} count");
    }
}

#[test]
fn claude_projects_only_timestamp_shaped_strings() {
    assert_projected_timestamps(TranscriptKind::ClaudeSession);
}

#[test]
fn codex_projects_only_timestamp_shaped_strings() {
    assert_projected_timestamps(TranscriptKind::CodexThread);
}

#[test]
fn invalid_recorded_roles_use_a_fixed_word_and_count_once() {
    for kind in READERS {
        let invalid = project_prose(kind, Some(json!(VALID_TIMESTAMP)), BOUND_VALUE);
        assert_eq!(invalid.turns[0].role, "unknown", "{kind:?} role");
        assert_eq!(invalid.turns[0].ts, VALID_TIMESTAMP, "{kind:?} timestamp");
        assert_eq!(invalid.unrecognized_records, 1, "{kind:?} count");

        let combined = project_message(kind, Some(json!(BOUND_VALUE)), BOUND_VALUE, |part| {
            vec![
                json!({"type": part, "text": "x"}),
                json!({"type": "future"}),
            ]
        });
        assert_eq!(combined.turns[0].role, "unknown", "{kind:?} role");
        assert_eq!(combined.turns[0].ts, "", "{kind:?} timestamp");
        assert_eq!(combined.unrecognized_records, 1, "{kind:?} count");
    }
}

#[test]
fn metadata_keeps_diagnostics_for_records_with_no_turn() {
    for kind in READERS {
        for (parts, count) in [(vec![], 0), (vec![json!({"type": "future"})], 1)] {
            let projection =
                project_message(kind, Some(json!("bad stamp")), "role/unknown", |_| parts);
            assert_eq!(projection.turns.len(), 0, "{kind:?} turns");
            assert_eq!(projection.unrecognized_records, count, "{kind:?} count");
        }
    }
}

#[test]
fn codex_counts_a_bad_stamp_only_when_its_fallback_survives_association() {
    let fallback = json!({
        "timestamp": "bad stamp",
        "type": "event_msg",
        "payload": {"type": "item_completed", "item": {
            "type": "AgentMessage", "id": "m1",
            "content": [{"type": "output_text", "text": "fallback"}],
        }},
    });
    let canonical = json!({
        "timestamp": VALID_TIMESTAMP,
        "type": "response_item",
        "payload": {"type": "message", "role": "assistant", "id": "m1",
                    "content": [{"type": "output_text", "text": "canonical"}]},
    });

    let kind = TranscriptKind::CodexThread;
    let mirrored = project_rows(kind, &format!("{fallback}\n{canonical}\n"));
    assert_eq!(mirrored.turns.len(), 1);
    assert_eq!(mirrored.turns[0].blocks, [Block::text("canonical")]);
    assert_eq!(mirrored.unrecognized_records, 0);

    let unmirrored = project_rows(kind, &format!("{fallback}\n"));
    assert_eq!(unmirrored.turns.len(), 1);
    assert_eq!(unmirrored.turns[0].blocks, [Block::text("fallback")]);
    assert_eq!(unmirrored.turns[0].ts, "");
    assert_eq!(unmirrored.unrecognized_records, 1);
}

#[test]
fn oversized_timestamps_leave_only_the_one_byte_block() {
    let oversized = "x".repeat(64 * 1024);
    for kind in READERS {
        let projection = project_prose(kind, Some(json!(oversized)), "assistant");
        assert_eq!(projection.turns.len(), 1);
        let turn = &projection.turns[0];
        assert_eq!(turn.ts.len(), 0, "{kind:?} timestamp bytes");
        assert_eq!(turn.blocks, [Block::text("x")]);
        assert_eq!(projection.unrecognized_records, 1, "{kind:?} count");
        assert!(!projection.truncated, "{kind:?} fits the display cap");
    }
}
