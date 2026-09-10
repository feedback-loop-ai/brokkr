use super::*;
use serde_json::json;

fn reference(kind: &str, locator: &str, home: &str) -> Transcript {
    Transcript {
        kind: kind.to_string(),
        locator: locator.to_string(),
        home: home.to_string(),
    }
}

#[test]
fn claude_ids_follow_the_leading_hexadecimal_language() {
    assert!(valid_claude_id("a"));
    assert!(valid_claude_id("abcd-1234"));
    assert!(valid_claude_id(&"a".repeat(64)));
    assert!(!valid_claude_id(""));
    assert!(!valid_claude_id("-abc"));
    assert!(!valid_claude_id("g123"));
    assert!(!valid_claude_id(&"a".repeat(65)));
    assert!(!valid_claude_id("a_b"));
}

#[test]
fn codex_ids_follow_the_engine_language() {
    assert!(valid_codex_id("a"));
    assert!(valid_codex_id("0199mine"));
    assert!(valid_codex_id("a-bC09"));
    assert!(valid_codex_id(&format!("a{}", "b".repeat(127))));
    assert!(!valid_codex_id(""));
    assert!(!valid_codex_id("-abc"));
    assert!(!valid_codex_id("ab_cd"));
    assert!(!valid_codex_id(&format!("a{}", "b".repeat(128))));
    assert!(!valid_codex_id("aé"));
}

#[test]
fn dsh_locators_are_relative_forward_slashed_components() {
    assert!(valid_dsh_locator("sessions/brokkr/seat-222"));
    assert!(!valid_dsh_locator(""));
    assert!(!valid_dsh_locator("/abs"));
    assert!(!valid_dsh_locator("../other-seat"));
    assert!(!valid_dsh_locator("a/../b"));
    assert!(!valid_dsh_locator("C:/x"));
    assert!(!valid_dsh_locator("a//b"));
    assert!(!valid_dsh_locator("a\\b"));
}

#[test]
fn reference_refusals_keep_their_recorded_strings_and_legacy_false() {
    for (recorded, reason) in [
        (reference("none", "", ""), Unavailable::None),
        (
            reference("future-session", "opaque-222", "/retained/future"),
            Unavailable::UnsupportedKind,
        ),
        (
            reference("codex-thread", "", "/retained/codex"),
            Unavailable::Unannounced,
        ),
        (
            reference("codex-thread", "0199mine", ""),
            Unavailable::MissingHome,
        ),
        (
            reference("codex-thread", "-abc", "/retained/codex"),
            Unavailable::InvalidReference,
        ),
    ] {
        let selection = select_reference(
            Some(&recorded),
            LegacyProvenance::Claude,
            Some("stale-legacy-id"),
            Some("/home/op/.claude/projects"),
        );
        assert_eq!(selection.outcome, Err(reason));
        assert_eq!(selection.reference, Some(recorded.clone()));
        assert!(!selection.legacy);
    }
}

#[test]
fn an_absent_common_reference_synthesizes_an_eligible_legacy_claude_id() {
    let selection = select_reference(
        None,
        LegacyProvenance::Claude,
        Some("abcd-1234"),
        Some("/home/op/.claude/projects"),
    );
    assert!(selection.legacy);
    let valid = selection.outcome.clone().unwrap();
    assert_eq!(valid.kind, TranscriptKind::ClaudeSession);
    assert_eq!(valid.locator, "abcd-1234");
    assert_eq!(selection.reference.unwrap().kind, "claude-session");
}

#[test]
fn an_absent_common_reference_with_no_legacy_id_is_no_reference() {
    let selection = select_reference(None, LegacyProvenance::Claude, None, Some("/h/p"));
    assert_eq!(selection.outcome, Err(Unavailable::NoReference));
    assert_eq!(selection.reference, None);
    assert!(!selection.legacy);
}

#[test]
fn a_present_but_invalid_legacy_id_is_invalid_not_absent() {
    let selection = select_reference(
        None,
        LegacyProvenance::Claude,
        Some("-abc"),
        Some("/home/op/.claude/projects"),
    );
    assert_eq!(selection.outcome, Err(Unavailable::InvalidReference));
    assert!(!selection.legacy);
}

#[test]
fn an_explicit_codex_provenance_refuses_claude_legacy_synthesis() {
    let selection = select_reference(
        None,
        LegacyProvenance::Other,
        Some("abcd-1234"),
        Some("/home/op/.claude/projects"),
    );
    assert_eq!(selection.outcome, Err(Unavailable::NoReference));
    assert_eq!(selection.reference, None);
}

#[test]
fn full_session_follows_the_kind_table() {
    let claude = ValidReference {
        kind: TranscriptKind::ClaudeSession,
        locator: "abcd-1234".to_string(),
        home: "/h/.claude/projects".to_string(),
    };
    assert_eq!(
        full_session(&claude, None).unwrap(),
        "full session: claude --resume abcd-1234"
    );
    let codex = ValidReference {
        kind: TranscriptKind::CodexThread,
        locator: "019c-222a".to_string(),
        home: "/retained/codex".to_string(),
    };
    assert_eq!(
        full_session(&codex, None).unwrap(),
        "full session: rollout unavailable; codex exec resume 019c-222a; home: \"/retained/codex\""
    );
    assert_eq!(
        full_session(&codex, Some("/retained/codex/sessions/rollout-019c-222a.jsonl")).unwrap(),
        "full session: \"/retained/codex/sessions/rollout-019c-222a.jsonl\"; codex exec resume 019c-222a; home: \"/retained/codex\""
    );
    let dsh = ValidReference {
        kind: TranscriptKind::DshSession,
        locator: "sessions/brokkr/seat-222".to_string(),
        home: "/retained/dsh".to_string(),
    };
    assert_eq!(full_session(&dsh, None), None);
    assert_eq!(
        full_session(&dsh, Some("/retained/dsh/s.jsonl")).unwrap(),
        "full session: \"/retained/dsh/s.jsonl\""
    );
}

#[test]
fn json_string_quoting_matches_the_delta() {
    assert_eq!(json_string("/a/b"), "\"/a/b\"");
    assert_eq!(json_string("a\"b"), "\"a\\\"b\"");
    assert_eq!(json_string("a\\b"), "\"a\\\\b\"");
    assert_eq!(json_string("a\u{1}b"), "\"a\\u0001b\"");
    assert_eq!(json_string("é"), "\"é\"");
}

#[test]
fn notices_are_ordered_and_omit_false_conditions() {
    assert!(notices(false, 0, 0).is_empty());
    assert_eq!(
        notices(true, 1, 2),
        vec![
            "transcript truncated (size cap)".to_string(),
            "malformed transcript lines skipped: 1".to_string(),
            "unrecognized transcript records: 2".to_string(),
        ]
    );
}

#[test]
fn admit_accepts_valid_utf8_and_rejects_invalid_bytes() {
    let bytes = b"{\"a\":1}\n";
    let admitted = admit(&Snapshot {
        bytes,
        overflow: false,
        eof: true,
    })
    .unwrap();
    assert_eq!(admitted.text, "{\"a\":1}\n");
    assert!(admit(&Snapshot {
        bytes: &[0xff, 0xfe],
        overflow: false,
        eof: true
    })
    .is_err());
}

#[test]
fn admit_treats_a_cap_cut_code_point_as_a_boundary_fragment() {
    // "é" is 0xC3 0xA9; cutting after 0xC3 leaves an incomplete sequence.
    let admitted = admit(&Snapshot {
        bytes: &[b'a', 0xC3],
        overflow: true,
        eof: false,
    })
    .unwrap();
    assert_eq!(admitted.text, "a");
    assert!(admitted.overflow);
}

#[test]
fn rows_include_a_valid_final_newline_less_row_and_drop_a_provisional_one() {
    let text = "{\"a\":1}\n{\"b\":2}";
    let admitted = Admitted {
        text,
        overflow: false,
    };
    let parsed = rows(&admitted);
    assert_eq!(parsed.len(), 2);
    assert!(parsed[0].newline_terminated);
    assert!(parsed[1].final_fragment);
    // Above the cap the newline-less tail never participates.
    let admitted = Admitted {
        text,
        overflow: true,
    };
    assert_eq!(rows(&admitted).len(), 1);
}

// ------------------------------------------------------- projection

fn snapshot(text: &str) -> Projection {
    project(
        TranscriptKind::ClaudeSession,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    )
}

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

#[test]
fn claude_projection_matches_the_shipped_fixture() {
    let projection = snapshot(SHIPPED_CLAUDE);
    assert_eq!(projection.skipped_lines, 1);
    assert_eq!(projection.unrecognized_records, 0);
    assert!(!projection.truncated);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].role, "assistant");
    assert_eq!(projection.turns[0].ts, "2026-01-01T00:00:00Z");
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::text("the plain form")]
    );
    assert_eq!(projection.turns[1].blocks.len(), 3);
    assert_eq!(projection.turns[1].blocks[0], Block::text("what happened"));
    assert_eq!(
        projection.turns[1].blocks[1],
        Block::tool("Read · src/lib.rs")
    );
    assert_eq!(projection.turns[1].blocks[2], Block::tool("Bash"));
}

#[test]
fn claude_counts_unrecognized_records_once() {
    let text = concat!(
        "{\"type\":\"future-record\"}\n",
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":[",
        "{\"type\":\"text\",\"text\":\"kept\"},{\"type\":\"future-block\"},{\"type\":\"other\"}]}}\n",
    );
    let projection = snapshot(text);
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 2);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);
}

#[test]
fn the_display_cap_stops_before_the_first_oversized_turn() {
    let huge = "x".repeat(DISPLAY_CAP + 1);
    let text = format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"{huge}\"}}}}\n\
         {{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":\"later\"}}}}\n"
    );
    let projection = snapshot(&text);
    assert!(projection.truncated);
    assert!(projection.turns.is_empty());
}

#[test]
fn codex_projects_ordered_content() {
    let text = concat!(
        "{\"timestamp\":\"t1\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
        "{\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"reasoning\",\"summary\":[{\"type\":\"summary_text\",\"text\":\"think\"}]}}\n",
        "{\"timestamp\":\"t3\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"name\":\"Read\",\"call_id\":\"c1\",\"arguments\":\"{}\"}}\n",
        "{\"timestamp\":\"t4\",\"type\":\"response_item\",\"payload\":{\"type\":\"function_call_output\",\"call_id\":\"c1\",\"output\":\"ok\"}}\n",
    );
    let projection = project(
        TranscriptKind::CodexThread,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 4);
    assert_eq!(projection.turns[0].role, "user");
    assert_eq!(projection.turns[1].blocks, vec![Block::reasoning("think")]);
    assert_eq!(projection.turns[2].blocks, vec![Block::tool("Read {}")]);
    assert_eq!(projection.turns[3].blocks, vec![Block::tool_result("ok")]);
}

#[test]
fn codex_quiet_metadata_and_unknown_events_count() {
    let text = concat!(
        "{\"timestamp\":\"t\",\"type\":\"turn_context\",\"payload\":{\"model\":\"m\"}}\n",
        "{\"timestamp\":\"t\",\"type\":\"event_msg\",\"payload\":{\"type\":\"token_count\"}}\n",
        "{\"timestamp\":\"t\",\"type\":\"event_msg\",\"payload\":{\"type\":\"future_event\"}}\n",
    );
    let projection = project(
        TranscriptKind::CodexThread,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert_eq!(projection.unrecognized_records, 1);
    assert!(projection.turns.is_empty());
}

#[test]
fn dsh_admits_numeric_zero_and_projects_messages() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"user/message\",\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"q\"}]},\"time\":1000}\n",
        "{\"type\":\"assistant/message\",\"data\":{\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"a\"},{\"type\":\"reasoning\",\"text\":\"r\"}]}},\"time\":1004}\n",
    );
    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].ts, "1000");
    assert_eq!(projection.turns[0].blocks, vec![Block::text("q")]);
    assert_eq!(projection.turns[1].ts, "1004");
    assert_eq!(projection.turns[1].blocks.len(), 2);
}

#[test]
fn dsh_rejects_foreign_versions_with_zero_counts() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":1}\n",
        "not json\n",
        "{\"type\":\"future/event\"}\n",
    );
    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);
    assert!(projection.turns.is_empty());
}

#[test]
fn dsh_packed_rows_reconstruct_sequence_and_time() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[5],\"texts\":[\"a\",\"b\"]}}\n",
    );
    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].ts, "1000");
    assert_eq!(projection.turns[1].ts, "1005");
}

#[test]
fn dsh_required_unknown_refuses_but_ignorable_omits() {
    let refused = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"future/event\"}\n",
    );
    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: refused.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);
    let omitted = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"future/event\",\"ignorable\":true}\n",
    );
    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: omitted.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);
}

// ------------------------------------------------------ test fixtures

fn row(value: serde_json::Value) -> String {
    let mut out = value.to_string();
    out.push('\n');
    out
}

fn project_text(kind: TranscriptKind, text: &str) -> Projection {
    project(
        kind,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    )
}

fn codex(text: &str) -> Projection {
    project_text(TranscriptKind::CodexThread, text)
}

fn dsh(text: &str) -> Projection {
    project_text(TranscriptKind::DshSession, text)
}

fn dsh_chunk(seq: i64, time: i64, turn: i64, step: i64, text: &str) -> String {
    row(json!({
        "type": "assistant/chunk",
        "seq": seq,
        "time": time,
        "data": {"turn": turn, "step": step, "chunk": {"type": "text-delta", "text": text}}
    }))
}

fn dsh_reasoning(seq: i64, time: i64, turn: i64, step: i64, text: &str) -> String {
    row(json!({
        "type": "assistant/chunk",
        "seq": seq,
        "time": time,
        "data": {"turn": turn, "step": step, "chunk": {"type": "reasoning-delta", "text": text}}
    }))
}

fn packed_text_chunks(
    seq0: i64,
    time0: i64,
    turn: i64,
    step: i64,
    dt: &[i64],
    texts: &[&str],
) -> String {
    row(json!({
        "type": "text-chunks",
        "seq0": seq0,
        "time0": time0,
        "data": {"turn": turn, "step": step, "index": 0, "dt": dt, "texts": texts}
    }))
}

fn packed_reasoning_chunks(
    seq0: i64,
    time0: i64,
    turn: i64,
    step: i64,
    dt: &[i64],
    texts: &[&str],
) -> String {
    row(json!({
        "type": "reasoning-chunks",
        "seq0": seq0,
        "time0": time0,
        "data": {"turn": turn, "step": step, "index": 0, "dt": dt, "texts": texts}
    }))
}

fn dsh_assembly(
    seq: i64,
    time: i64,
    turn: i64,
    step: i64,
    cites: Option<serde_json::Value>,
) -> String {
    let mut value = json!({
        "type": "assistant/message",
        "seq": seq,
        "time": time,
        "data": {"turn": turn, "step": step, "message": {"content": [{"type": "text", "text": "assembled"}]}}
    });
    if let Some(cites) = cites {
        value
            .as_object_mut()
            .expect("assembly is an object")
            .insert("sourceEventSeqs".to_string(), cites);
    }
    row(value)
}

fn dsh_embed(seq: i64, time: i64, turn: i64, step: i64, content: serde_json::Value) -> String {
    row(json!({
        "type": "assistant/message",
        "seq": seq,
        "time": time,
        "data": {"turn": turn, "step": step, "message": {"content": content}}
    }))
}

fn dsh_tool_call(
    seq: i64,
    time: i64,
    turn: i64,
    step: i64,
    call_id: &str,
    name: &str,
    arguments: &str,
) -> String {
    row(json!({
        "type": "tool/call",
        "seq": seq,
        "time": time,
        "data": {"turn": turn, "step": step, "callId": call_id, "name": name, "arguments": arguments}
    }))
}

fn dsh_tool_result(
    seq: i64,
    time: i64,
    turn: i64,
    step: i64,
    call_id: &str,
    output: &str,
) -> String {
    row(json!({
        "type": "tool/result",
        "seq": seq,
        "time": time,
        "data": {"turn": turn, "step": step, "message": {"role": "tool", "content": [{"type": "tool-result", "toolCallId": call_id, "content": output}]}}
    }))
}

// ------------------------------------------------ Codex association (D5)

#[test]
fn codex_five_record_ruling_reads_in_order() {
    let text = [
        row(json!({"timestamp":"t1","type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"question"}]}})),
        row(json!({"timestamp":"t2","type":"response_item","payload":{"type":"reasoning","id":"r1","summary":[{"type":"summary_text","text":"consider"}]}})),
        row(json!({"timestamp":"t3","type":"response_item","payload":{"type":"function_call","name":"Read","call_id":"c1","arguments":"{\"path\":\"a\"}"}})),
        row(json!({"timestamp":"t4","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"contents"}})),
        row(json!({"timestamp":"t5","type":"response_item","payload":{"type":"message","role":"assistant","id":"m1","content":[{"type":"output_text","text":"ruling"}]}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.turns.len(), 5);
    let roles: Vec<&str> = projection
        .turns
        .iter()
        .map(|turn| turn.role.as_str())
        .collect();
    assert_eq!(
        roles,
        vec!["user", "assistant", "assistant", "tool", "assistant"]
    );
    assert_eq!(projection.turns[0].blocks, vec![Block::text("question")]);
    assert_eq!(projection.turns[0].ts, "t1");
    assert_eq!(
        projection.turns[1].blocks,
        vec![Block::reasoning("consider")]
    );
    assert_eq!(
        projection.turns[2].blocks,
        vec![Block::tool("Read {\"path\":\"a\"}")]
    );
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool_result("contents")]
    );
    assert_eq!(projection.turns[4].blocks, vec![Block::text("ruling")]);
    assert_eq!(projection.turns[4].ts, "t5");
}

#[test]
fn codex_canonical_records_beside_matching_events_show_once() {
    let text = [
        row(json!({"timestamp":"t1","type":"response_item","payload":{"type":"message","role":"assistant","id":"m1","content":[{"type":"output_text","text":"canonical"}]}})),
        row(json!({"timestamp":"t1b","type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"m1","content":[{"type":"output_text","text":"mirror"}]}}})),
        row(json!({"timestamp":"t2","type":"response_item","payload":{"type":"reasoning","id":"r1","summary":[{"type":"summary_text","text":"think"}]}})),
        row(json!({"timestamp":"t2b","type":"event_msg","payload":{"type":"item_completed","item":{"type":"Reasoning","id":"r1","summary_text":["think"]}}})),
        row(json!({"timestamp":"t3","type":"response_item","payload":{"type":"function_call","name":"Read","call_id":"c1","arguments":"{}"}})),
        row(json!({"timestamp":"t3b","type":"event_msg","payload":{"type":"exec_command_begin","call_id":"c1","command":"ls"}})),
        row(json!({"timestamp":"t4","type":"response_item","payload":{"type":"function_call_output","call_id":"c1","output":"ok"}})),
        row(json!({"timestamp":"t4b","type":"event_msg","payload":{"type":"exec_command_end","call_id":"c1","stdout":"ok"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 4);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("canonical")]);
    assert_eq!(projection.turns[0].ts, "t1");
    assert_eq!(projection.turns[1].blocks, vec![Block::reasoning("think")]);
    assert_eq!(projection.turns[2].blocks, vec![Block::tool("Read {}")]);
    assert_eq!(projection.turns[3].blocks, vec![Block::tool_result("ok")]);
}

#[test]
fn codex_event_only_snapshot_reads_all_five_kinds() {
    let text = [
        row(json!({"timestamp":"t1","type":"event_msg","payload":{"type":"user_message","message":"q"}})),
        row(json!({"timestamp":"t2","type":"event_msg","payload":{"type":"item_completed","item":{"type":"Reasoning","id":"r1","summary_text":["think"]}}})),
        row(json!({"timestamp":"t3","type":"event_msg","payload":{"type":"item_completed","item":{"type":"FunctionCallOutput","call_id":"c1","output":"ok"}}})),
        row(json!({"timestamp":"t4","type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","id":"c2","command":"ls","stdout":"files"}}})),
        row(json!({"timestamp":"t5","type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"m1","content":[{"type":"output_text","text":"ruling"}]}}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 5);
    assert_eq!(projection.turns[0].role, "user");
    assert_eq!(projection.turns[0].blocks, vec![Block::text("q")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::reasoning("think")]);
    assert_eq!(projection.turns[2].role, "tool");
    assert_eq!(projection.turns[2].blocks, vec![Block::tool_result("ok")]);
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool("ls"), Block::tool_result("files")]
    );
    assert_eq!(projection.turns[4].blocks, vec![Block::text("ruling")]);
}

#[test]
fn codex_late_canonical_replaces_fallback_and_keeps_unrelated_events() {
    let text = [
        row(json!({"timestamp":"t1","type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"m1","content":[{"type":"output_text","text":"fallback"}]}}})),
        row(json!({"timestamp":"t2","type":"response_item","payload":{"type":"message","role":"assistant","id":"m1","content":[{"type":"output_text","text":"canonical"}]}})),
        row(json!({"timestamp":"t3","type":"event_msg","payload":{"type":"user_message","message":"unrelated"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("canonical")]);
    assert_eq!(projection.turns[0].ts, "t2");
    assert_eq!(projection.turns[1].blocks, vec![Block::text("unrelated")]);
}

#[test]
fn codex_absent_or_incomplete_canonical_cannot_suppress_the_visible_event() {
    let fallback = row(
        json!({"timestamp":"t1","type":"event_msg","payload":{"type":"item_completed","item":{"type":"AgentMessage","id":"m1","content":[{"type":"output_text","text":"fallback"}]}}}),
    );
    // The canonical counterpart lies beyond the bounded snapshot.
    let projection = codex(&fallback);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("fallback")]);

    // Above the source cap the newline-less canonical tail never
    // participates, so it cannot suppress the visible event.
    let canonical = concat!(
        "{\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",",
        "\"role\":\"assistant\",\"id\":\"m1\",\"content\":[{\"type\":\"output_text\",\"text\":\"canonical\"}]}}"
    );
    let text = format!("{fallback}{canonical}");
    let projection = project(
        TranscriptKind::CodexThread,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: true,
            eof: false,
        },
    );
    assert!(projection.truncated);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("fallback")]);

    // An incomplete canonical append does not parse and cannot suppress.
    let incomplete = concat!(
        "{\"timestamp\":\"t2\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",",
        "\"role\":\"assistant\",\"id\":\"m1\",\"content\":[{\"type\":\"output_text\",\"text\":\"canonical\"}"
    );
    let text = format!("{fallback}{incomplete}");
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("fallback")]);
    assert_eq!(projection.skipped_lines, 0);
}

#[test]
fn codex_identical_text_with_distinct_identities_is_kept_twice() {
    let text = [
        row(json!({"type":"response_item","payload":{"type":"message","role":"assistant","id":"m1","content":[{"type":"output_text","text":"same"}]}})),
        row(json!({"type":"response_item","payload":{"type":"message","role":"assistant","id":"m2","content":[{"type":"output_text","text":"same"}]}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, projection.turns[1].blocks);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("same")]);
}

#[test]
fn codex_two_calls_with_equal_arguments_keep_distinct_ids() {
    let text = [
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"F","call_id":"c1","arguments":"{}"}})),
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"F","call_id":"c2","arguments":"{}"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("F {}")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::tool("F {}")]);
}

#[test]
fn codex_orphan_output_and_encrypted_reasoning_are_honest() {
    let text = [
        row(json!({"timestamp":"t1","type":"response_item","payload":{"type":"function_call_output","call_id":"c9","output":"orphan"}})),
        row(json!({"timestamp":"t2","type":"response_item","payload":{"type":"reasoning","id":"r9","encrypted_content":"AAAA"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::tool_result("orphan")]
    );
    assert_eq!(projection.turns[0].ts, "t1");
}

#[test]
fn codex_unknown_completed_item_counts_once() {
    let text = row(
        json!({"type":"event_msg","payload":{"type":"item_completed","item":{"type":"FutureItem","id":"x","content":[]}}}),
    );
    let projection = codex(&text);
    assert_eq!(projection.unrecognized_records, 1);
    assert!(projection.turns.is_empty());
}

#[test]
fn codex_command_output_prefers_streams_then_aggregate_then_formatted() {
    let text = [
        row(json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"a","stdout":"out","stderr":"err","aggregated_output":"agg","formatted_output":"fmt"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"b","aggregated_output":"agg","formatted_output":"fmt"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"c","formatted_output":"fmt"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"d"}})),
    ]
    .concat();
    let projection = codex(&text);
    let blocks: Vec<Vec<Block>> = projection
        .turns
        .iter()
        .map(|turn| turn.blocks.clone())
        .collect();
    assert_eq!(
        blocks,
        vec![
            vec![Block::tool_result("outerr")],
            vec![Block::tool_result("agg")],
            vec![Block::tool_result("fmt")],
            vec![Block::tool_result("")],
        ]
    );
}

#[test]
fn codex_direction_separates_calls_from_results() {
    let text = [
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"F","call_id":"c1","arguments":"{}"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_end","call_id":"c1","stdout":"ok"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("F {}")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::tool_result("ok")]);
}

#[test]
fn codex_colliding_identities_suppress_nothing() {
    // Two canonical records share an identity, so neither proves a mirror.
    let text = [
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"F","call_id":"c1","arguments":"{}"}})),
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"F","call_id":"c1","arguments":"{}"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_begin","call_id":"c1","command":"ls"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 3);

    // Two fallback events share an identity, so neither is suppressed.
    let text = [
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"F","call_id":"c1","arguments":"{}"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_begin","call_id":"c1","command":"one"}})),
        row(json!({"type":"event_msg","payload":{"type":"exec_command_begin","call_id":"c1","command":"two"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 3);
}

#[test]
fn codex_composite_event_keeps_its_uncovered_output() {
    // The canonical call covers only the event's call block; the output
    // has no canonical counterpart and stays visible.
    let text = [
        row(json!({"type":"response_item","payload":{"type":"function_call","name":"ls","call_id":"c2","arguments":"{}"}})),
        row(json!({"type":"event_msg","payload":{"type":"item_completed","item":{"type":"CommandExecution","id":"c2","command":"ls","stdout":"files"}}})),
    ]
    .concat();
    let projection = codex(&text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("ls {}")]);
    assert_eq!(projection.turns[1].role, "assistant");
    assert_eq!(
        projection.turns[1].blocks,
        vec![Block::tool_result("files")]
    );
}

// --------------------------------------------------- DSH format (D6)

#[test]
fn dsh_header_version_matrix_admits_only_numeric_zero() {
    for version in ["0", "0.0", "0e0", "-0"] {
        let header = format!("{{\"type\":\"session\",\"version\":{version}}}\n");
        let projection = dsh(&header);
        assert!(
            projection.unavailable.is_none(),
            "version {version} should admit"
        );
        assert!(projection.turns.is_empty());
    }
    for version in ["null", "false", "\"0\"", "[]", "{}", "1", "-1", "0.5"] {
        let header = format!("{{\"type\":\"session\",\"version\":{version}}}\n");
        let projection = dsh(&header);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "version {version} should refuse"
        );
        assert_eq!(projection.skipped_lines, 0, "version {version}");
        assert_eq!(projection.unrecognized_records, 0, "version {version}");
        assert!(projection.turns.is_empty());
    }
    let projection = dsh("{\"type\":\"session\"}\n");
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
}

#[test]
fn dsh_header_depth_matrix_accepts_zero_and_omitted() {
    for depth in [None, Some("0")] {
        let header = match depth {
            None => "{\"type\":\"session\",\"version\":0}\n".to_string(),
            Some(depth) => {
                format!("{{\"type\":\"session\",\"version\":0,\"delegationDepth\":{depth}}}\n")
            }
        };
        let projection = dsh(&header);
        assert!(
            projection.unavailable.is_none(),
            "depth {depth:?} should admit"
        );
    }
    for depth in ["1", "-1", "\"zero\"", "0.0", "0.5", "true"] {
        let header =
            format!("{{\"type\":\"session\",\"version\":0,\"delegationDepth\":{depth}}}\n");
        let projection = dsh(&header);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "depth {depth} should refuse"
        );
        assert_eq!(projection.unrecognized_records, 0);
        assert_eq!(projection.skipped_lines, 0);
    }
}

#[test]
fn dsh_header_only_at_eof_without_a_newline_is_readable_zero_turns() {
    let projection = dsh("{\"type\":\"session\",\"version\":0}");
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);
    assert!(!projection.truncated);
}

#[test]
fn dsh_opening_row_cannot_borrow_a_later_header() {
    let session = row(json!({"type":"session","version":0}));
    for opening in ["not json", "[1]", "{\"type\":\"other\"}", ""] {
        let text = format!("{opening}\n{session}");
        let projection = dsh(&text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "opening {opening:?}"
        );
        assert!(projection.turns.is_empty(), "opening {opening:?}");
    }
    let projection = dsh("not json\n");
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
}

#[test]
fn dsh_packed_and_ordinary_fragments_are_equivalent() {
    let ordinary = [
        row(json!({"type":"session","version":0})),
        dsh_chunk(10, 1000, 1, 1, "a"),
        dsh_chunk(11, 999, 1, 1, "b"),
        dsh_chunk(12, 1004, 1, 1, "c"),
        dsh_reasoning(20, 1000, 1, 1, "r1"),
        dsh_reasoning(21, 999, 1, 1, "r2"),
        dsh_reasoning(22, 1004, 1, 1, "r3"),
    ]
    .concat();
    let packed = [
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[-1, 5], &["a", "b", "c"]),
        packed_reasoning_chunks(20, 1000, 1, 1, &[-1, 5], &["r1", "r2", "r3"]),
    ]
    .concat();
    let projection = dsh(&ordinary);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 6);
    let stamps: Vec<&str> = projection
        .turns
        .iter()
        .map(|turn| turn.ts.as_str())
        .collect();
    assert_eq!(stamps, vec!["1000", "999", "1004", "1000", "999", "1004"]);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("a")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::text("b")]);
    assert_eq!(projection.turns[2].blocks, vec![Block::text("c")]);
    assert_eq!(projection.turns[3].blocks, vec![Block::reasoning("r1")]);
    assert_eq!(projection.turns[4].blocks, vec![Block::reasoning("r2")]);
    assert_eq!(projection.turns[5].blocks, vec![Block::reasoning("r3")]);
    assert_eq!(dsh(&packed), projection);
}

#[test]
fn dsh_invalid_packed_rows_refuse_once() {
    let cases = [
        (
            "wrong dt length",
            json!({"type":"text-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"texts":["a","b","c"]}}),
        ),
        (
            "non-string member",
            json!({"type":"text-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5,5],"texts":["a",5,"c"]}}),
        ),
        (
            "overflowing reconstruction",
            json!({"type":"text-chunks","seq0":9007199254740991_i64,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"texts":["a","b"]}}),
        ),
        (
            "missing tool id",
            json!({"type":"tool-call-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"args":["a","b"]}}),
        ),
        (
            "non-string tool id",
            json!({"type":"tool-call-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"args":["a","b"],"id":5}}),
        ),
        (
            "non-string optional tool name",
            json!({"type":"tool-call-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"args":["a","b"],"id":"c1","name":5}}),
        ),
        (
            "key outside the text shape",
            json!({"type":"text-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"texts":["a","b"],"args":["x"]}}),
        ),
        (
            "missing required data key",
            json!({"type":"text-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"dt":[5],"texts":["a","b"]}}),
        ),
        (
            "key outside the envelope",
            json!({"type":"text-chunks","seq0":10,"time0":1000,"extra":1,"data":{"turn":1,"step":1,"index":0,"dt":[5],"texts":["a","b"]}}),
        ),
        (
            "unsafe time0",
            json!({"type":"text-chunks","seq0":10,"time0":9007199254740992_i64,"data":{"turn":1,"step":1,"index":0,"dt":[5],"texts":["a","b"]}}),
        ),
        (
            "unsafe dt member between safe endpoints",
            json!({"type":"text-chunks","seq0":10,"time0":-1,"data":{"turn":1,"step":1,"index":0,"dt":[9007199254740992_i64],"texts":["a","b"]}}),
        ),
        (
            "negative-zero seq0",
            json!({"type":"text-chunks","seq0":-0.0,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[5],"texts":["a","b"]}}),
        ),
    ];
    for (label, value) in cases {
        let text = format!(
            "{}{}",
            row(json!({"type":"session","version":0})),
            row(value)
        );
        let projection = dsh(&text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{label}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{label}");
        assert_eq!(projection.skipped_lines, 0, "{label}");
        assert!(projection.turns.is_empty(), "{label}");
    }
}

#[test]
fn dsh_tool_argument_run_invents_no_call() {
    let packing = row(
        json!({"type":"tool-call-chunks","seq0":10,"time0":1000,"data":{"turn":1,"step":1,"index":0,"dt":[1],"args":["{\"a\"","\":1}"],"id":"c1","name":"Read"}}),
    );
    let text = format!("{}{}", row(json!({"type":"session","version":0})), packing);
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);
    assert!(!projection.truncated);

    // The equivalent ordinary tool-call deltas invent no call either.
    let ordinary = [
        row(json!({"type":"session","version":0})),
        row(json!({"type":"assistant/chunk","seq":10,"time":1000,"data":{"turn":1,"step":1,"chunk":{"type":"tool-call-delta","argumentsDelta":"{\"a\"","id":"c1","name":"Read","index":0}}})),
        row(json!({"type":"assistant/chunk","seq":11,"time":1000,"data":{"turn":1,"step":1,"chunk":{"type":"tool-call-delta","argumentsDelta":"\":1}","id":"c1","name":"Read","index":0}}})),
    ]
    .concat();
    let projection = dsh(&ordinary);
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);
}

#[test]
fn dsh_incomplete_and_cap_cut_packed_rows_supply_no_members() {
    let partial = concat!(
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{",
        "\"turn\":1,\"step\":1,\"index\":0,\"dt\":[5],\"texts\":[\"a\",\"b\""
    );
    let text = format!("{}{partial}", row(json!({"type":"session","version":0})));
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);

    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: true,
            eof: false,
        },
    );
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
    assert!(projection.truncated);
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);
}

fn citation_projection(cites: Option<serde_json::Value>, packed: bool) -> Projection {
    let mut text = row(json!({"type":"session","version":0}));
    if packed {
        text.push_str(&packed_text_chunks(
            10,
            1000,
            1,
            1,
            &[1, 1],
            &["c10", "c11", "c12"],
        ));
        text.push_str(&dsh_chunk(14, 1014, 1, 1, "c14"));
    } else {
        for seq in [10, 11, 12, 14] {
            text.push_str(&dsh_chunk(seq, 1000 + seq, 1, 1, &format!("c{seq}")));
        }
    }
    text.push_str(&dsh_assembly(20, 1020, 1, 1, cites));
    dsh(&text)
}

fn two_chunk_citation(cites: Option<serde_json::Value>, packed: bool) -> Projection {
    let mut text = row(json!({"type":"session","version":0}));
    if packed {
        text.push_str(&packed_text_chunks(10, 1000, 1, 1, &[1], &["c10", "c11"]));
    } else {
        text.push_str(&dsh_chunk(10, 1000, 1, 1, "c10"));
        text.push_str(&dsh_chunk(11, 1001, 1, 1, "c11"));
    }
    text.push_str(&dsh_assembly(20, 1020, 1, 1, cites));
    dsh(&text)
}

#[test]
fn dsh_citation_ranges_suppress_only_observed_same_step_chunks() {
    for cites in [Some(json!([[10, 12]])), Some(json!([10, 11, 12]))] {
        for packed in [false, true] {
            let projection = citation_projection(cites.clone(), packed);
            assert!(
                projection.unavailable.is_none(),
                "{cites:?} packed={packed}"
            );
            assert_eq!(
                projection.unrecognized_records, 0,
                "{cites:?} packed={packed}"
            );
            assert_eq!(projection.turns.len(), 2, "{cites:?} packed={packed}");
            assert_eq!(projection.turns[0].blocks, vec![Block::text("c14")]);
            assert_eq!(projection.turns[1].blocks, vec![Block::text("assembled")]);
            assert_eq!(projection.turns[1].ts, "1020");
        }
    }
}

#[test]
fn dsh_citation_entry_order_is_not_transcript_order() {
    for packed in [false, true] {
        let projection = citation_projection(Some(json!([[12, 14], [10, 12], 11])), packed);
        assert!(projection.unavailable.is_none(), "packed={packed}");
        assert_eq!(projection.unrecognized_records, 0, "packed={packed}");
        assert_eq!(projection.skipped_lines, 0, "packed={packed}");
        assert!(!projection.truncated, "packed={packed}");
        assert_eq!(projection.turns.len(), 1, "packed={packed}");
        assert_eq!(projection.turns[0].blocks, vec![Block::text("assembled")]);
        assert!(
            notices(
                projection.truncated,
                projection.skipped_lines,
                projection.unrecognized_records
            )
            .is_empty(),
            "packed={packed}"
        );
    }
}

#[test]
fn dsh_empty_absent_and_partial_citations_preserve_uncited_content() {
    for packed in [false, true] {
        for cites in [Some(json!([])), None] {
            let projection = two_chunk_citation(cites, packed);
            assert!(projection.unavailable.is_none(), "packed={packed}");
            assert_eq!(projection.turns.len(), 3, "packed={packed}");
            assert_eq!(projection.turns[0].blocks, vec![Block::text("c10")]);
            assert_eq!(projection.turns[1].blocks, vec![Block::text("c11")]);
            assert_eq!(projection.turns[2].blocks, vec![Block::text("assembled")]);
        }
        let projection = two_chunk_citation(Some(json!([10])), packed);
        assert!(projection.unavailable.is_none(), "packed={packed}");
        assert_eq!(projection.turns.len(), 2, "packed={packed}");
        assert_eq!(projection.turns[0].blocks, vec![Block::text("c11")]);
        assert_eq!(projection.turns[1].blocks, vec![Block::text("assembled")]);
    }
}

#[test]
fn dsh_citation_scope_and_ambiguity_do_not_invent_association() {
    for packed in [false, true] {
        let mut text = row(json!({"type":"session","version":0}));
        if packed {
            text.push_str(&packed_text_chunks(10, 1000, 1, 1, &[], &["same-step"]));
        } else {
            text.push_str(&dsh_chunk(10, 1000, 1, 1, "same-step"));
        }
        text.push_str(&dsh_chunk(11, 1001, 1, 2, "cross-step"));
        text.push_str(&dsh_chunk(12, 1002, 1, 1, "shared-a"));
        text.push_str(&dsh_chunk(12, 1003, 1, 1, "shared-b"));
        text.push_str(&row(json!({"type":"tool/call","seq":14,"time":1004,"data":{"turn":1,"step":1,"callId":"c9","name":"F","arguments":"{}"}})));
        text.push_str(&dsh_assembly(
            20,
            1020,
            1,
            1,
            Some(json!([10, 11, 12, 13, 14])),
        ));
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "packed={packed}");
        assert_eq!(projection.unrecognized_records, 0, "packed={packed}");
        assert_eq!(projection.turns.len(), 5, "packed={packed}");
        assert_eq!(projection.turns[0].blocks, vec![Block::text("cross-step")]);
        assert_eq!(projection.turns[1].blocks, vec![Block::text("shared-a")]);
        assert_eq!(projection.turns[2].blocks, vec![Block::text("shared-b")]);
        assert_eq!(projection.turns[3].blocks, vec![Block::tool("F {}")]);
        assert_eq!(projection.turns[4].blocks, vec![Block::text("assembled")]);
    }
}

#[test]
fn dsh_citation_validation_refuses_and_large_ranges_stay_bounded() {
    for (label, cites) in [
        ("null", json!(null)),
        ("self", json!([20])),
        ("future", json!([21])),
        ("reversed", json!([[12, 10]])),
        ("non-integer endpoint", json!([10, "12"])),
    ] {
        let text = format!(
            "{}{}{}",
            row(json!({"type":"session","version":0})),
            dsh_chunk(10, 1000, 1, 1, "c10"),
            dsh_assembly(20, 1020, 1, 1, Some(cites)),
        );
        let projection = dsh(&text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{label}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{label}");
        assert!(projection.turns.is_empty(), "{label}");
    }

    // An owning sequence above the safe integer range is refused even
    // when every cited value is itself small.
    let text = format!(
        "{}{}{}",
        row(json!({"type":"session","version":0})),
        dsh_chunk(10, 1000, 1, 1, "c10"),
        dsh_assembly(9007199254740992_i64, 1020, 1, 1, Some(json!([10]))),
    );
    let projection = dsh(&text);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);
    assert!(projection.turns.is_empty());

    // A valid huge range tests only observed identities, with no
    // allocation or iteration proportional to its width.
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        dsh_assembly(
            9007199254740991_i64,
            1020,
            1,
            1,
            Some(json!([0, 9007199254740990_i64])),
        ),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("assembled")]);
}

#[test]
fn dsh_display_cap_stops_between_packed_members() {
    let huge = "x".repeat(DISPLAY_CAP);
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[1], &[&huge, "tail"]),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert!(projection.truncated);
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks[0].text.len(), DISPLAY_CAP);
}

#[test]
fn dsh_event_refusal_preserves_prefix_counts_and_all_notices() {
    let text = [
        row(json!({"type":"session","version":0})),
        "not json\n".to_string(),
        "also not json\n".to_string(),
        row(json!({"type":"future/a"})),
        row(json!({"type":"future/b"})),
    ]
    .concat();
    let projection = project(
        TranscriptKind::DshSession,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: true,
            eof: false,
        },
    );
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.skipped_lines, 2);
    assert_eq!(projection.unrecognized_records, 2);
    assert!(projection.truncated);
    assert!(projection.turns.is_empty());
    assert_eq!(
        notices(
            projection.truncated,
            projection.skipped_lines,
            projection.unrecognized_records
        ),
        vec![
            "transcript truncated (size cap)".to_string(),
            "malformed transcript lines skipped: 2".to_string(),
            "unrecognized transcript records: 2".to_string(),
        ]
    );
}

#[test]
fn dsh_unknown_events_require_exact_boolean_true() {
    for marker in ["false", "null", "\"true\"", "1"] {
        let text = format!(
            "{}{{\"type\":\"future/event\",\"ignorable\":{marker}}}\n",
            row(json!({"type":"session","version":0})),
        );
        let projection = dsh(&text);
        assert_eq!(projection.unrecognized_records, 1, "marker {marker}");
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "marker {marker}"
        );
    }
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"future/event\",\"ignorable\":true}\n",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);

    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"future/event\"}\n",
    );
    let projection = dsh(&text);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);
}

#[test]
fn dsh_recognized_envelope_counts_two_unsupported_blocks_once() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        row(
            json!({"type":"user/message","data":{"content":[{"type":"text","text":"kept"},{"type":"future-a"},{"type":"future-b"}]}})
        ),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);

    // The same rule holds on a `tool/result` surface envelope.
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        row(
            json!({"type":"tool/result","data":{"turn":1,"step":1,"message":{"role":"tool","content":[{"type":"text","text":"kept"},{"type":"future-a"},{"type":"future-b"}]}}})
        ),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);
}

// ------------------------------------------- DSH tool association (D6)

#[test]
fn dsh_dedicated_call_owns_only_its_matching_embedded_block() {
    let text = [
        row(json!({"type":"session","version":0})),
        dsh_embed(
            10,
            100,
            1,
            1,
            json!([
                {"type":"text","text":"answer"},
                {"type":"reasoning","text":"think"},
                {"type":"tool-call","id":"c1","name":"Read","arguments":"{}"}
            ]),
        ),
        dsh_tool_call(11, 101, 1, 1, "c1", "Read", "{}"),
    ]
    .concat();
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::text("answer"), Block::reasoning("think")]
    );
    assert_eq!(projection.turns[0].ts, "100");
    assert_eq!(projection.turns[1].blocks, vec![Block::tool("Read {}")]);
    assert_eq!(projection.turns[1].ts, "101");
}

#[test]
fn dsh_dedicated_call_before_its_message_still_suppresses_once() {
    let text = [
        row(json!({"type":"session","version":0})),
        dsh_tool_call(10, 100, 1, 1, "c1", "Read", "{}"),
        dsh_embed(
            11,
            101,
            1,
            1,
            json!([
                {"type":"text","text":"answer"},
                {"type":"tool-call","id":"c1","name":"Read","arguments":"{}"}
            ]),
        ),
    ]
    .concat();
    let projection = dsh(&text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("Read {}")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::text("answer")]);
}

#[test]
fn dsh_dedicated_call_mismatch_keeps_both_copies() {
    for (call_id, turn, step) in [("c2", 1, 1), ("c1", 1, 2)] {
        let text = [
            row(json!({"type":"session","version":0})),
            dsh_embed(
                10,
                100,
                1,
                1,
                json!([
                    {"type":"text","text":"answer"},
                    {"type":"tool-call","id":"c1","name":"Read","arguments":"{}"}
                ]),
            ),
            dsh_tool_call(11, 101, turn, step, call_id, "Read", "{}"),
        ]
        .concat();
        let projection = dsh(&text);
        assert_eq!(
            projection.turns.len(),
            2,
            "call_id={call_id} turn/step={turn}/{step}"
        );
        assert_eq!(projection.turns[0].blocks.len(), 2);
        assert_eq!(projection.turns[1].blocks, vec![Block::tool("Read {}")]);
    }
}

#[test]
fn dsh_embedded_call_without_a_dedicated_record_stays_once() {
    let text = [
        row(json!({"type":"session","version":0})),
        dsh_embed(
            10,
            100,
            1,
            1,
            json!([
                {"type":"text","text":"answer"},
                {"type":"tool-call","id":"c1","name":"Read","arguments":"{}"}
            ]),
        ),
    ]
    .concat();
    let projection = dsh(&text);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::text("answer"), Block::tool("Read {}")]
    );
}

#[test]
fn dsh_two_dedicated_calls_colliding_keep_every_record() {
    let text = [
        row(json!({"type":"session","version":0})),
        dsh_embed(
            10,
            100,
            1,
            1,
            json!([
                {"type":"text","text":"answer"},
                {"type":"tool-call","id":"c1","name":"Read","arguments":"{}"}
            ]),
        ),
        dsh_tool_call(11, 101, 1, 1, "c1", "Read", "{}"),
        dsh_tool_call(12, 102, 1, 1, "c1", "Read", "{}"),
    ]
    .concat();
    let projection = dsh(&text);
    assert_eq!(projection.turns.len(), 3);
    assert_eq!(projection.turns[0].blocks.len(), 2);
    assert_eq!(projection.turns[1].blocks, vec![Block::tool("Read {}")]);
    assert_eq!(projection.turns[2].blocks, vec![Block::tool("Read {}")]);
}

#[test]
fn dsh_dedicated_result_matches_embedded_result_in_either_order() {
    let message = dsh_embed(
        10,
        100,
        1,
        1,
        json!([
            {"type":"text","text":"answer"},
            {"type":"tool-result","toolCallId":"c1","content":"ok"}
        ]),
    );
    let result = dsh_tool_result(11, 101, 1, 1, "c1", "ok");
    for result_first in [false, true] {
        let text = if result_first {
            format!(
                "{}{}{}",
                row(json!({"type":"session","version":0})),
                result,
                message
            )
        } else {
            format!(
                "{}{}{}",
                row(json!({"type":"session","version":0})),
                message,
                result
            )
        };
        let projection = dsh(&text);
        assert_eq!(projection.turns.len(), 2, "result_first={result_first}");
        if result_first {
            assert_eq!(projection.turns[0].blocks, vec![Block::tool_result("ok")]);
            assert_eq!(projection.turns[1].blocks, vec![Block::text("answer")]);
        } else {
            assert_eq!(projection.turns[0].blocks, vec![Block::text("answer")]);
            assert_eq!(projection.turns[1].blocks, vec![Block::tool_result("ok")]);
        }
    }
}

#[test]
fn dsh_suppressed_only_block_emits_no_empty_turn() {
    let text = [
        row(json!({"type":"session","version":0})),
        dsh_embed(
            10,
            100,
            1,
            1,
            json!([
                {"type":"tool-call","id":"c1","name":"Read","arguments":"{}"}
            ]),
        ),
        dsh_tool_call(11, 101, 1, 1, "c1", "Read", "{}"),
    ]
    .concat();
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("Read {}")]);
}

// --------------------------------------- offline evidence for the
// review's validation, omission, time and media findings

#[test]
fn dsh_unknown_nested_chunk_variant_is_a_counted_omission() {
    for chunk in [json!({"type":"future-chunk","text":"x"}), json!({})] {
        let text = format!(
            "{}{}",
            row(json!({"type":"session","version":0})),
            row(json!({"type":"assistant/chunk","seq":10,"time":1000,
                       "data":{"turn":1,"step":1,"chunk":chunk}})),
        );
        let projection = dsh(&text);
        assert!(
            projection.unavailable.is_none(),
            "an unsupported nested variant never refuses the source: {chunk}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{chunk}");
        assert!(projection.turns.is_empty());
    }
}

#[test]
fn dsh_assembly_without_readable_blocks_does_not_suppress_chunks() {
    let text = format!(
        "{}{}{}{}",
        row(json!({"type":"session","version":0})),
        dsh_chunk(10, 1000, 1, 1, "kept-a"),
        dsh_chunk(11, 1001, 1, 1, "kept-b"),
        row(json!({"type":"assistant/message","seq":12,"time":1002,
                   "data":{"message":{"content":[]},"turn":1,"step":1},
                   "sourceEventSeqs":[[10,11]]})),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(
        projection.turns.len(),
        2,
        "an assembly with no readable projected blocks suppresses nothing"
    );
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept-a")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::text("kept-b")]);
}

#[test]
fn dsh_negative_zero_time_renders_zero() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"assistant/chunk\",\"seq\":10,\"time\":-0.0,\
         \"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"a\"}}}\n",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns[0].ts, "0");

    let packed = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":-0.0,\
         \"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[1],\"texts\":[\"a\",\"b\"]}}\n",
    );
    let projection = dsh(&packed);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].ts, "0");
    assert_eq!(projection.turns[1].ts, "1");
}

#[test]
fn source_utf8_refusal_keeps_established_overflow() {
    let bytes = [0xffu8, 0xfe];
    let projection = project(
        TranscriptKind::ClaudeSession,
        &Snapshot {
            bytes: &bytes,
            overflow: true,
            eof: false,
        },
    );
    assert_eq!(projection.unavailable, Some(Unavailable::Unreadable));
    assert!(
        projection.truncated,
        "the bounded read's overflow fact survives a UTF-8 refusal"
    );
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);
    assert!(projection.turns.is_empty());
}

#[test]
fn codex_media_parts_name_their_class() {
    let text = row(json!({"timestamp":"t1","type":"response_item","payload":{
        "type":"message","role":"user",
        "content":[{"type":"input_text","text":"see"},{"type":"input_image"},
                   {"type":"input_audio"}]}}));
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(
        projection.turns[0].blocks,
        vec![
            Block::text("see"),
            Block::omitted("[image omitted]"),
            Block::omitted("[audio omitted]"),
        ]
    );
}

#[test]
fn codex_unassociated_user_mirrors_are_both_preserved() {
    let text = [
        row(json!({"timestamp":"t1","type":"response_item","payload":{
            "type":"message","role":"user",
            "content":[{"type":"input_text","text":"question"}]}})),
        row(json!({"timestamp":"t1b","type":"event_msg","payload":{
            "type":"user_message","message":"question"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(
        projection.turns.len(),
        2,
        "id-less user mirrors are deliberately unassociated and both stay visible"
    );
    assert_eq!(projection.turns[0].role, "user");
    assert_eq!(projection.turns[0].blocks, vec![Block::text("question")]);
    assert_eq!(projection.turns[1].role, "user");
    assert_eq!(projection.turns[1].blocks, vec![Block::text("question")]);
}
