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
        "full session: rollout unavailable, codex exec resume 019c-222a, home \"/retained/codex\""
    );
    assert_eq!(
        full_session(&codex, Some("/retained/codex/sessions/rollout-019c-222a.jsonl")).unwrap(),
        "full session: path \"/retained/codex/sessions/rollout-019c-222a.jsonl\", codex exec resume 019c-222a, home \"/retained/codex\""
    );
    let dsh = ValidReference {
        kind: TranscriptKind::DshSession,
        locator: "sessions/brokkr/seat-222".to_string(),
        home: "/retained/dsh".to_string(),
    };
    assert_eq!(full_session(&dsh, None), None);
    assert_eq!(
        full_session(&dsh, Some("/retained/dsh/s.jsonl")).unwrap(),
        "full session: path \"/retained/dsh/s.jsonl\""
    );
}

#[test]
fn portable_display_literal_is_reversible_and_restricted() {
    // The restricted direct alphabet is emitted verbatim.
    assert_eq!(portable_display_literal("aZ09/._-:"), "\"aZ09/._-:\"");
    assert_eq!(
        portable_display_literal("sessions/seat-222/s.jsonl"),
        "\"sessions/seat-222/s.jsonl\""
    );
    // Every other scalar is a lowercase four-digit escape; no short escapes.
    assert_eq!(portable_display_literal(" "), "\"\\u0020\"");
    assert_eq!(portable_display_literal("\""), "\"\\u0022\"");
    assert_eq!(portable_display_literal("\\"), "\"\\u005c\"");
    assert_eq!(portable_display_literal("\n"), "\"\\u000a\"");
    assert_eq!(portable_display_literal("\t"), "\"\\u0009\"");
    assert_eq!(portable_display_literal("$"), "\"\\u0024\"");
    assert_eq!(portable_display_literal("`"), "\"\\u0060\"");
    assert_eq!(portable_display_literal(";"), "\"\\u003b\"");
    assert_eq!(portable_display_literal("&"), "\"\\u0026\"");
    assert_eq!(portable_display_literal("|"), "\"\\u007c\"");
    assert_eq!(portable_display_literal("<"), "\"\\u003c\"");
    assert_eq!(portable_display_literal(">"), "\"\\u003e\"");
    assert_eq!(portable_display_literal("%"), "\"\\u0025\"");
    assert_eq!(portable_display_literal("!"), "\"\\u0021\"");
    // BMP and non-BMP scalars: one escape, and a JSON surrogate pair.
    assert_eq!(portable_display_literal("é"), "\"\\u00e9\"");
    assert_eq!(portable_display_literal("\u{20ac}"), "\"\\u20ac\"");
    assert_eq!(portable_display_literal("😀"), "\"\\ud83d\\ude00\"");
    // Exact JSON round-trip for the R25 hostile matrix.
    let hostile = " /a b/\"q\"\\x\u{1}$(cmd)`tick`;a&b|c<d>e%f!g é😀\n";
    let literal = portable_display_literal(hostile);
    let decoded: String = serde_json::from_str(&literal).unwrap();
    assert_eq!(decoded, hostile);
    // No raw hostile fragment and no short escape survives inside the quotes.
    let body = &literal[1..literal.len() - 1];
    assert!(!body.contains(' '));
    assert!(!body.contains('"'));
    assert!(!body.contains(';'));
    assert!(!body.contains('&'));
    assert!(!body.contains('|'));
    assert!(!body.contains('<'));
    assert!(!body.contains('>'));
    assert!(!body.contains('`'));
    assert!(!body.contains('$'));
    assert!(!body.contains('%'));
    assert!(!body.contains('!'));
    assert!(!body.contains('é'));
    assert!(!body.contains("\\n") && !body.contains("\\t") && !body.contains("\\r"));
    for byte in body.bytes() {
        let direct =
            byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-' | b':');
        assert!(direct || byte == b'\\' || byte == b'u' || byte.is_ascii_hexdigit());
    }
}

#[test]
fn full_session_hint_is_portable_and_hostile_inert() {
    // A hostile confirmed Codex path/home round-trips through the literal and
    // leaves the fixed comma framing with no shell operator raw.
    let codex = ValidReference {
        kind: TranscriptKind::CodexThread,
        locator: "019c-222a".to_string(),
        home: "/retained/codex $(x)".to_string(),
    };
    let path = "/retained/codex/rollout \"q\" `c`;a&b.jsonl";
    let hint = full_session(&codex, Some(path)).unwrap();
    let display_path = portable_display_literal(path);
    let display_home = portable_display_literal(&codex.home);
    assert_eq!(
        hint,
        format!(
            "full session: path {display_path}, codex exec resume 019c-222a, home {display_home}"
        )
    );
    assert!(!hint.contains(';'));
    assert!(!hint.contains('&'));
    assert!(!hint.contains('`'));
    assert!(!hint.contains("$("));
    // Each display literal round-trips to the exact path/home.
    let path_literal_start = "full session: path ".len();
    let path_literal_end = path_literal_start + display_path.len();
    let decoded_path: String =
        serde_json::from_str(&hint[path_literal_start..path_literal_end]).unwrap();
    assert_eq!(decoded_path, path);
    let home_literal_end = hint.len();
    let home_literal_start = home_literal_end - display_home.len();
    let decoded_home: String =
        serde_json::from_str(&hint[home_literal_start..home_literal_end]).unwrap();
    assert_eq!(decoded_home, codex.home);
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
    assert_eq!(
        projection.turns[2].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool_result("ok [c1]")]
    );
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
        vec![Block::tool("Read {\"path\":\"a\"} [c1]")]
    );
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool_result("contents [c1]")]
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
    assert_eq!(
        projection.turns[2].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool_result("ok [c1]")]
    );
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
    assert_eq!(
        projection.turns[2].blocks,
        vec![Block::tool_result("ok [c1]")]
    );
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool("ls [c2]"), Block::tool_result("files [c2]")]
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
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("F {} [c1]")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::tool("F {} [c2]")]);
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
        vec![Block::tool_result("orphan [c9]")]
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
            vec![Block::tool_result("outerr [a]")],
            vec![Block::tool_result("agg [b]")],
            vec![Block::tool_result("fmt [c]")],
            vec![Block::tool_result("[d]")],
        ]
    );
}

#[test]
fn codex_mcp_and_dynamic_records_keep_their_measured_context() {
    // The displayed text retains the recorded id and the measured
    // provider context: MCP server/tool/arguments, and a dynamic
    // response's own content items and error rather than its request.
    let text = [
        row(json!({"type":"event_msg","payload":{"type":"mcp_tool_call_begin","call_id":"m1","invocation":{"server":"srv","tool":"search","arguments":"{\"q\":\"x\"}"}}})),
        row(json!({"type":"event_msg","payload":{"type":"dynamic_tool_call_response","call_id":"d1","content_items":[{"type":"inputText","text":"done"}],"error":"boom"}})),
        row(json!({"type":"event_msg","payload":{"type":"item_completed","item":{"type":"McpToolCall","id":"m2","server":"srv2","tool":"read","arguments":"{}","result":{"content":"body"},"error":{"message":"nope"}}}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 3);
    let call0 = &projection.turns[0].blocks[0].text;
    assert!(
        call0.contains("srv") && call0.contains("search") && call0.contains("\"q\""),
        "the MCP begin keeps server, tool and arguments: {call0}"
    );
    assert!(call0.contains("[m1]"), "the id is visible: {call0}");
    let result1 = &projection.turns[1].blocks[0].text;
    assert!(
        result1.contains("done") && result1.contains("boom"),
        "the dynamic response keeps its own content and error: {result1}"
    );
    assert!(result1.contains("[d1]"), "the id is visible: {result1}");
    let call2 = &projection.turns[2].blocks[0].text;
    assert!(
        call2.contains("srv2") && call2.contains("read"),
        "the completed MCP item keeps server and tool: {call2}"
    );
    assert!(call2.contains("[m2]"), "the id is visible: {call2}");
    let result2 = &projection.turns[2].blocks[1].text;
    assert!(
        result2.contains("body") && result2.contains("nope"),
        "the completed MCP result keeps content and error: {result2}"
    );
    assert!(result2.contains("[m2]"), "the id is visible: {result2}");
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
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("F {} [c1]")]);
    assert_eq!(
        projection.turns[1].blocks,
        vec![Block::tool_result("ok [c1]")]
    );
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
    assert_eq!(projection.turns[0].blocks, vec![Block::tool("ls {} [c2]")]);
    assert_eq!(projection.turns[1].role, "assistant");
    assert_eq!(
        projection.turns[1].blocks,
        vec![Block::tool_result("files [c2]")]
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
fn dsh_refused_opening_header_never_decodes_later_rows() {
    // A foreign version followed by malformed JSON, an unknown
    // non-ignorable event and an allocation-heavy packed row: the header
    // refusal returns before any later row is decoded or counted.
    let text = format!(
        "{}{}{}{}",
        row(json!({"type":"session","version":1})),
        "not json\n",
        row(json!({"type":"future/event"})),
        row(json!({"type":"text-chunks","seq0":0,"time0":0,
                   "data":{"turn":1,"step":1,"index":0,"dt":[],"texts":["a"]}})),
    );
    let projection = dsh(&text);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.unrecognized_records, 0);
    assert!(projection.turns.is_empty());
    assert!(!projection.truncated);
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
        assert_eq!(projection.turns[3].blocks, vec![Block::tool("F {} [c9]")]);
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
    assert_eq!(
        projection.turns[1].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
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
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
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
        assert_eq!(
            projection.turns[1].blocks,
            vec![Block::tool(format!("Read {{}} [{call_id}]"))]
        );
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
        vec![Block::text("answer"), Block::tool("Read {} [c1]")]
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
    assert_eq!(
        projection.turns[1].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
    assert_eq!(
        projection.turns[2].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
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
            assert_eq!(
                projection.turns[0].blocks,
                vec![Block::tool_result("ok [c1]")]
            );
            assert_eq!(projection.turns[1].blocks, vec![Block::text("answer")]);
        } else {
            assert_eq!(projection.turns[0].blocks, vec![Block::text("answer")]);
            assert_eq!(
                projection.turns[1].blocks,
                vec![Block::tool_result("ok [c1]")]
            );
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
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::tool("Read {} [c1]")]
    );
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

// -------------------------------- review-repair regression evidence
// (controller-chief-review-7505: M2, M3, M4, M5, L1, L7, L11)

#[test]
fn codex_declared_content_arrays_project_text_and_named_media_omissions() {
    let mcp_end = row(json!({
        "type": "event_msg",
        "payload": {
            "type": "mcp_tool_call_end",
            "call_id": "m1",
            "result": {"Ok": {"content": [
                {"type": "text", "text": "body"},
                {"type": "image", "data": "BASE64-MEDIA"}
            ]}}
        }
    }));
    let dynamic_response = row(json!({
        "type": "event_msg",
        "payload": {
            "type": "dynamic_tool_call_response",
            "call_id": "d1",
            "content_items": [
                {"type": "inputText", "text": "done"},
                {"type": "inputImage", "data": "BASE64-MEDIA"}
            ],
            "error": "boom"
        }
    }));
    let completed_mcp = row(json!({
        "type": "event_msg",
        "payload": {"type": "item_completed", "item": {
            "type": "McpToolCall",
            "id": "m2",
            "server": "srv",
            "tool": "read",
            "result": {"content": [
                {"type": "text", "text": "body2"},
                {"type": "image", "data": "BASE64-MEDIA"}
            ]}
        }}
    }));
    let completed_dynamic = row(json!({
        "type": "event_msg",
        "payload": {"type": "item_completed", "item": {
            "type": "DynamicToolCall",
            "id": "d2",
            "tool": "run",
            "content_items": [
                {"type": "inputText", "text": "done2"},
                {"type": "inputAudio", "data": "BASE64-MEDIA"}
            ]
        }}
    }));
    let text = [mcp_end, dynamic_response, completed_mcp, completed_dynamic].concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    let rendered: Vec<String> = projection
        .turns
        .iter()
        .flat_map(|turn| turn.blocks.iter())
        .map(|block| block.text.clone())
        .collect();
    for text in &rendered {
        assert!(
            !text.contains("BASE64-MEDIA"),
            "an encoded media body never reaches tool text: {text}"
        );
    }
    assert!(
        rendered
            .iter()
            .any(|text| text.contains("body") && text.contains("[image omitted]")),
        "{rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|text| text.contains("done") && text.contains("boom")),
        "{rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|text| text.contains("body2") && text.contains("[image omitted]")),
        "{rendered:?}"
    );
    assert!(
        rendered
            .iter()
            .any(|text| text.contains("done2") && text.contains("[audio omitted]")),
        "{rendered:?}"
    );
}

#[test]
fn dsh_nested_tool_result_content_names_media_omissions() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        row(json!({"type":"tool/result","seq":10,"time":1000,
            "data":{"turn":1,"step":1,"message":{"role":"tool","content":[
                {"type":"tool-result","toolCallId":"c1","content":[
                    {"type":"text","text":"read"},
                    {"type":"image","data":"BASE64-MEDIA"}]}]}}})),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 1);
    let text = &projection.turns[0].blocks[0].text;
    assert!(
        text.contains("read") && text.contains("[image omitted]"),
        "{text}"
    );
    assert!(!text.contains("BASE64-MEDIA"), "{text}");
}

#[test]
fn dsh_omission_only_assembly_does_not_suppress_cited_chunks() {
    let text = format!(
        "{}{}{}{}",
        row(json!({"type":"session","version":0})),
        dsh_chunk(10, 1000, 1, 1, "kept-a"),
        dsh_chunk(11, 1001, 1, 1, "kept-b"),
        row(json!({"type":"assistant/message","seq":12,"time":1002,
            "data":{"message":{"content":[{"type":"image","data":"x"}]},"turn":1,"step":1},
            "sourceEventSeqs":[[10,11]]})),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(
        projection.turns.len(),
        3,
        "an omission-only assembly is not readable projected content"
    );
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept-a")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::text("kept-b")]);
    assert_eq!(
        projection.turns[2].blocks,
        vec![Block::omitted("[image omitted]")]
    );
}

#[test]
fn wrong_typed_text_members_count_once_and_keep_supported_siblings() {
    let message = row(json!({"timestamp":"t1","type":"response_item","payload":{
        "type":"message","role":"assistant","id":"m1",
        "content":[{"type":"output_text","text":"kept"},{"type":"output_text","text":7}]}}));
    let projection = codex(&message);
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);

    let reasoning = row(json!({"timestamp":"t2","type":"response_item","payload":{
        "type":"reasoning","id":"r1",
        "summary":[{"type":"summary_text","text":"kept"},{"type":"summary_text","text":7}]}}));
    let projection = codex(&reasoning);
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::reasoning("kept")]);

    let completed = row(
        json!({"type":"event_msg","payload":{"type":"item_completed","item":{
        "type":"AgentMessage","id":"a1",
        "content":[{"type":"output_text","text":"kept"},{"type":"output_text","text":7}]}}}),
    );
    let projection = codex(&completed);
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);

    let dsh_text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        row(json!({"type":"user/message","seq":10,"time":1000,
            "data":{"turn":1,"step":1,"content":[{"type":"text","text":"kept"},
                                                 {"type":"text","text":7}]}})),
    );
    let projection = dsh(&dsh_text);
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);
}

#[test]
fn dsh_nonzero_underflow_version_is_foreign() {
    let projection = dsh("{\"type\":\"session\",\"version\":1e-400}\n");
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);

    for spelling in ["0", "0.0", "-0.0", "0e0", "0.000"] {
        let text = format!("{{\"type\":\"session\",\"version\":{spelling}}}\n");
        let projection = dsh(&text);
        assert!(
            projection.unavailable.is_none(),
            "an exact zero spelling admits: {spelling}"
        );
    }
}

#[test]
fn dsh_nonzero_underflow_time_is_invalid_not_zero() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"assistant/chunk\",\"seq\":10,\"time\":1e-400,\
         \"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"a\"}}}\n",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(
        projection.turns[0].ts, "",
        "an underflowed nonzero time is invalid, not a zero millisecond stamp"
    );
}

#[test]
fn dsh_exact_integral_time_spellings_render_the_integer() {
    let text = format!(
        "{}{}{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"assistant/chunk\",\"seq\":10,\"time\":1e3,\
         \"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"a\"}}}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":11,\"time\":1000.0,\
         \"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"b\"}}}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":12,\"time\":-0.0,\
         \"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"c\"}}}\n",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 3);
    assert_eq!(projection.turns[0].ts, "1000");
    assert_eq!(projection.turns[1].ts, "1000");
    assert_eq!(projection.turns[2].ts, "0");
}

#[test]
fn dsh_packed_nonzero_underflow_time0_refuses() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1e-400,\
         \"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[],\"texts\":[\"a\"]}}\n",
    );
    let projection = dsh(&text);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);
}

#[test]
fn dsh_packed_positions_accept_fractional_numbers() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\
         \"data\":{\"turn\":0.5,\"step\":0.25,\"index\":2.5,\"dt\":[],\"texts\":[\"a\"]}}\n",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("a")]);
}

#[test]
fn dsh_integral_float_positions_match_their_integer_spelling() {
    let text = format!(
        "{}{}{}",
        row(json!({"type":"session","version":0})),
        dsh_chunk(10, 1000, 1, 1, "kept"),
        row(json!({"type":"assistant/message","seq":11,"time":1001,
            "data":{"message":{"content":[{"type":"text","text":"assembled"}]},
                    "turn":1.0,"step":1.0},
            "sourceEventSeqs":[10]})),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(
        projection.turns.len(),
        1,
        "an integral float position is the same recorded position"
    );
    assert_eq!(projection.turns[0].blocks, vec![Block::text("assembled")]);
}

#[test]
fn packed_empty_members_allocate_no_events_but_stay_observed() {
    let object = json!({"type":"text-chunks","seq0":5,"time0":100,
        "data":{"turn":1,"step":1,"index":0,"dt":[0,0],"texts":["","",""]}});
    let map = object.as_object().expect("object");
    match dsh_packed("text-chunks", map, object.get("data").unwrap(), "") {
        DshRow::Packed(events, observed) => {
            assert!(
                events.is_empty(),
                "empty text members allocate no projected events"
            );
            assert_eq!(observed, vec![5, 6, 7]);
        }
        _ => panic!("a complete packed row is Packed"),
    }

    let object = json!({"type":"tool-call-chunks","seq0":5,"time0":100,
        "data":{"turn":1,"step":1,"index":0,"dt":[0],"args":["a","b"],"id":"c1"}});
    let map = object.as_object().expect("object");
    match dsh_packed("tool-call-chunks", map, object.get("data").unwrap(), "") {
        DshRow::Packed(events, observed) => {
            assert!(
                events.is_empty(),
                "argument fragments never allocate a projected call"
            );
            assert_eq!(observed, vec![5, 6]);
        }
        _ => panic!("a complete packed row is Packed"),
    }
}

#[test]
fn codex_quiet_top_level_without_payload_is_not_counted() {
    for kind in [
        "session_meta",
        "turn_context",
        "compacted",
        "token_usage_record",
        "world_state",
        "security_risk_score",
    ] {
        let text = row(json!({"timestamp":"t","type":kind}));
        let projection = codex(&text);
        assert_eq!(projection.unrecognized_records, 0, "{kind}");
        assert!(projection.turns.is_empty());
    }
    let projection = codex(&row(json!({"timestamp":"t","type":"future-top"})));
    assert_eq!(projection.unrecognized_records, 1);
}

#[test]
fn dsh_undeclared_blocks_alias_supplies_no_content() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        row(json!({"type":"assistant/message","seq":10,"time":1000,
            "data":{"turn":1,"step":1,
                    "message":{"blocks":[{"type":"text","text":"alias"}]}}})),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert!(
        projection.turns.is_empty(),
        "an undeclared content alias supplies no prose"
    );
    assert_eq!(projection.unrecognized_records, 0);

    let declared = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        row(json!({"type":"assistant/message","seq":10,"time":1000,
            "data":{"turn":1,"step":1,
                    "message":{"content":[{"type":"text","text":"declared"}]}}})),
    );
    let projection = dsh(&declared);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("declared")]);
}

/// Coverage companion for the remaining declared Codex response variants:
/// the tool-search, shell/web action, custom-tool and compaction records
/// each project their recorded data or stay recognized-quiet.
#[test]
fn codex_remaining_declared_variants_project_or_stay_quiet() {
    let text = [
        row(json!({"timestamp":"t1","type":"response_item","payload":{
            "type":"tool_search_call","call_id":"s1","arguments":{"q":"x"}}})),
        row(json!({"timestamp":"t2","type":"response_item","payload":{
            "type":"tool_search_output","call_id":"s1","tools":[{"name":"read"}]}})),
        row(json!({"timestamp":"t3","type":"response_item","payload":{
            "type":"local_shell_call","call_id":"s2","action":{"command":"ls"}}})),
        row(json!({"timestamp":"t4","type":"response_item","payload":{
            "type":"web_search_call","call_id":"s3","action":{"query":"rust"}}})),
        row(json!({"timestamp":"t5","type":"response_item","payload":{
            "type":"custom_tool_call","call_id":"s4","name":"apply","input":"patch"}})),
        row(json!({"timestamp":"t6","type":"response_item","payload":{
            "type":"custom_tool_call_output","call_id":"s4","output":"done"}})),
        row(json!({"timestamp":"t7","type":"response_item","payload":{"type":"additional_tools"}})),
        row(json!({"timestamp":"t8","type":"response_item","payload":{"type":"compaction"}})),
        row(json!({"timestamp":"t9","type":"response_item","payload":{"type":"compaction_summary"}})),
        row(json!({"timestamp":"t10","type":"response_item","payload":{"type":"context_compaction"}})),
        row(json!({"timestamp":"t11","type":"response_item","payload":{"type":"compaction_trigger"}})),
        row(json!({"timestamp":"t12","type":"event_msg","payload":{"type":"user_message"}})),
        row(json!({"timestamp":"t13","type":"event_msg","payload":{"type":"agent_message"}})),
        row(json!({"timestamp":"t14","type":"event_msg","payload":{"type":"agent_reasoning"}})),
    ]
    .concat();
    let projection = codex(&text);
    assert!(projection.unavailable.is_none());
    assert_eq!(
        projection.unrecognized_records, 0,
        "recognized variants and quiet metadata are not counted"
    );
    assert_eq!(projection.turns.len(), 6, "{:?}", projection.turns);

    let kinds: Vec<BlockKind> = projection
        .turns
        .iter()
        .flat_map(|turn| turn.blocks.iter().map(|block| block.kind))
        .collect();
    assert_eq!(
        kinds,
        vec![
            BlockKind::Tool,
            BlockKind::ToolResult,
            BlockKind::Tool,
            BlockKind::Tool,
            BlockKind::Tool,
            BlockKind::ToolResult,
        ]
    );
    let rendered: Vec<&str> = projection
        .turns
        .iter()
        .flat_map(|turn| turn.blocks.iter().map(|block| block.text.as_str()))
        .collect();
    assert!(rendered[0].contains("tool_search") && rendered[0].contains("[s1]"));
    assert!(rendered[1].contains("read") && rendered[1].contains("[s1]"));
    assert!(rendered[2].contains("ls") && rendered[2].contains("[s2]"));
    assert!(rendered[3].contains("rust") && rendered[3].contains("[s3]"));
    assert!(rendered[4].contains("apply") && rendered[4].contains("[s4]"));
    assert!(rendered[5].contains("done") && rendered[5].contains("[s4]"));
}

// ------------------------------------- closed tables and defensive edges

/// A borrowed admitted snapshot for the pure reader functions.
fn admitted(text: &str, overflow: bool) -> Admitted<'_> {
    Admitted { text, overflow }
}

#[test]
fn closed_vocabulary_accessors_cover_every_variant() {
    assert_eq!(TranscriptKind::None.as_str(), "none");
    assert_eq!(BlockKind::Reasoning.as_str(), "reasoning");
    assert_eq!(BlockKind::Omitted.as_str(), "omitted");
    assert_eq!(Unavailable::Unreadable.as_str(), "unreadable");
    let none = ValidReference {
        kind: TranscriptKind::None,
        locator: "x".to_string(),
        home: "/h".to_string(),
    };
    assert_eq!(full_session(&none, Some("/p")), None);
    assert_eq!(
        explanation_for(Unavailable::TurnNotRetained),
        "the requested turn is outside the retained projection"
    );
}

#[test]
fn selection_helpers_echo_the_reference_and_the_refusal() {
    let accepted = Selection {
        reference: Some(reference("claude-session", "abcd", "/h")),
        legacy: false,
        outcome: Ok(ValidReference {
            kind: TranscriptKind::ClaudeSession,
            locator: "abcd".to_string(),
            home: "/h".to_string(),
        }),
    };
    assert_eq!(accepted.kind(), Some(TranscriptKind::ClaudeSession));
    let read = TranscriptRead::from_selection(&accepted);
    assert_eq!(read.unavailable, Some(Unavailable::NotFound));
    assert!(read.full_session.is_some());
    assert_eq!(
        read.reference,
        Some(reference("claude-session", "abcd", "/h"))
    );

    let refused = Selection {
        reference: Some(reference("none", "", "")),
        legacy: false,
        outcome: Err(Unavailable::None),
    };
    assert_eq!(refused.kind(), None);
    let read = TranscriptRead::from_selection(&refused);
    assert_eq!(read.unavailable, Some(Unavailable::None));
    assert_eq!(read.full_session, None);
}

#[test]
fn home_and_locator_validation_covers_each_spelling() {
    assert!(!valid_home(""));
    assert!(!valid_home("a\u{7}b"));
    assert!(valid_home("/srv/home"));
    assert!(valid_home("C:\\Users\\op"));
    assert!(valid_home("C:/Users/op"));
    assert!(valid_home("\\\\server\\share"));
    assert!(valid_home("//server/share"));
    assert!(!valid_home("relative/path"));
    assert!(!valid_home("\\\\"));
    assert!(!clean_path("x\u{7}y"));
    assert!(clean_path("plain/path"));

    assert!(!valid_dsh_locator("a/./b"));
    assert!(!valid_dsh_locator("a/b\u{7}"));

    assert_eq!(
        validate_common(&reference("claude-session", "abcd", "relative")),
        Err(Unavailable::InvalidReference)
    );
    assert_eq!(
        validate_common(&reference("claude-session", "ab\u{7}cd", "/h")),
        Err(Unavailable::InvalidReference)
    );
    assert!(!locator_matches_kind(TranscriptKind::None, "abcd"));
    assert!(locator_matches_kind(TranscriptKind::DshSession, "a/b"));
}

#[test]
fn rows_drop_an_overflow_tail_and_admit_a_true_eof_fragment() {
    let overflowed = admitted("a\nb", true);
    let parsed = rows(&overflowed);
    assert_eq!(parsed.len(), 1);
    assert!(parsed[0].newline_terminated);

    let fragment = admitted("a\nb", false);
    let parsed = rows(&fragment);
    assert_eq!(parsed.len(), 2);
    assert!(!parsed[1].newline_terminated);
    assert!(parsed[1].final_fragment);
    assert_eq!(parsed[1].text, "b");

    let terminated = admitted("a\n", false);
    assert_eq!(rows(&terminated).len(), 1);
}

#[test]
fn claude_row_classifies_malformed_and_wrong_typed_shapes() {
    assert_eq!(
        claude_row(&json!("nope")),
        (Vec::new(), String::new(), String::new(), true)
    );
    assert_eq!(
        claude_row(&json!({"message": {}})),
        (Vec::new(), String::new(), String::new(), true)
    );

    let (blocks, _, _, unrecognized) =
        claude_row(&json!({"type":"assistant","message":{"role":"assistant","content":null}}));
    assert!(blocks.is_empty());
    assert!(!unrecognized);

    let (blocks, _, _, unrecognized) = claude_row(&json!({
        "type":"assistant","message":{"role":"assistant","content":[
            {"type":"text","text":7},{"type":"bogus"}]}}));
    assert!(blocks.is_empty());
    assert!(unrecognized);

    assert!(claude_row(&json!({"type":"assistant","message":7})).3);
    assert!(claude_row(&json!({"type":"assistant","message":{"role":"assistant","content":7}})).3);
    assert!(
        claude_row(&json!({"type":"assistant","message":{"role":"assistant","content":["x"]}})).3
    );
}

#[test]
fn payload_and_content_helpers_keep_fallbacks_lossless() {
    assert_eq!(payload_text(None), None);
    assert_eq!(payload_text(Some(&Value::Null)), None);
    assert_eq!(payload_text(Some(&json!("x"))), Some("x".to_string()));
    assert_eq!(
        payload_text(Some(&json!({"a":1}))),
        Some("{\"a\":1}".to_string())
    );

    assert_eq!(content_text(&Value::Null), "");
    assert_eq!(content_text(&json!("x")), "x");
    assert_eq!(content_text(&json!({"a":1})), "{\"a\":1}");
    assert_eq!(content_text(&json!([null, "x", 5])), "x 5");

    assert_eq!(content_member(&Value::Null), "");
    assert_eq!(content_member(&json!("x")), "x");
    assert_eq!(content_member(&json!(5)), "5");
    assert_eq!(content_member(&json!({"a":1})), "{\"a\":1}");
    assert_eq!(
        content_member(&json!({"type":"weird"})),
        "{\"type\":\"weird\"}"
    );

    assert_eq!(tool_text(None, "ctx"), "ctx");
    assert_eq!(tool_text(Some(""), "ctx"), "ctx");
    assert_eq!(tool_text(Some("id"), ""), "[id]");
}

#[test]
fn codex_row_edges_and_duplicate_identity() {
    assert!(codex_row(&json!("nope")).unrecognized);
    assert!(codex_row(&json!({"type":"response_item"})).unrecognized);
    assert!(codex_row(&json!({"type":"event_msg"})).unrecognized);
    assert!(codex_row(&json!({"type":"totally_unknown"})).unrecognized);
    assert!(codex_row(&json!({"type":"session_meta"})).blocks.is_empty());
    assert!(codex_response_item(&json!({})).2);

    let mut records = vec![CodexRecord {
        blocks: vec![
            CodexBlock::identified(Block::tool("a"), CodexFact::Call, Some("dup")),
            CodexBlock::identified(Block::tool("b"), CodexFact::Call, Some("dup")),
        ],
        role: String::new(),
        ts: String::new(),
        unrecognized: false,
        canonical: false,
    }];
    associate_codex(&mut records);
    assert_eq!(
        records[0].blocks.len(),
        2,
        "no canonical counterpart removes nothing"
    );
}

/// A row that projects no block is counted and released, never retained
/// as an empty record: a source of quiet or malformed rows must not
/// amplify into one record per row before association (design D4).
#[test]
fn codex_blockless_rows_are_counted_without_being_retained() {
    let mut text = String::new();
    for _ in 0..1000 {
        text.push_str("1\n");
        text.push_str(&row(json!({"type":"session_meta","timestamp":"t"})));
        text.push_str(&row(json!({"type":"totally_unknown"})));
        text.push_str(&row(
            json!({"type":"response_item","payload":{"type":"future_item"}}),
        ));
    }
    text.push_str(&row(json!({
        "type":"response_item",
        "payload":{"type":"message","role":"assistant","id":"m1",
                   "content":[{"type":"output_text","text":"kept"}]}
    })));
    text.push_str(&row(json!({
        "type":"event_msg",
        "payload":{"type":"agent_message","message":"kept"}
    })));
    text.push_str("not json\n");

    let mut projection = Projection::default();
    let records = collect_codex(&admitted(&text, false), &mut projection);
    assert_eq!(records.len(), 2, "only block-bearing rows are retained");
    assert!(records.iter().all(|record| !record.blocks.is_empty()));
    assert_eq!(records[0].role, "assistant");
    assert!(records[0].canonical);
    assert!(!records[1].canonical);
    assert_eq!(projection.unrecognized_records, 3000);
    assert_eq!(projection.skipped_lines, 1);

    let full = codex(&text);
    assert_eq!(full.turns.len(), 2);
    assert_eq!(full.unrecognized_records, 3000);
    assert_eq!(full.skipped_lines, 1);
}

#[test]
fn codex_response_item_classifies_wrong_typed_members() {
    assert!(codex_response_item(&json!({"type":"message","content":[{"type":"nope"}]})).2);
    assert!(!codex_response_item(&json!({"type":"message","content":null})).2);
    assert!(codex_response_item(&json!({"type":"message","content":5})).2);

    assert!(codex_response_item(&json!({"type":"reasoning","summary":[{"type":"nope"}]})).2);
    assert!(!codex_response_item(&json!({"type":"reasoning","summary":null})).2);
    assert!(codex_response_item(&json!({"type":"reasoning","summary":5})).2);

    let (blocks, _, unrecognized) =
        codex_response_item(&json!({"type":"function_call","name":"f"}));
    assert!(!unrecognized);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].block.text, "f");

    assert!(codex_response_item(&json!({"type":"future_item"})).2);
}

#[test]
fn codex_event_and_completed_items_cover_declared_variants() {
    assert!(codex_event_msg(&json!({})).2);
    let (blocks, role, unrecognized) =
        codex_event_msg(&json!({"type":"agent_reasoning","text":"deliberation"}));
    assert_eq!(role, "assistant");
    assert!(!unrecognized);
    assert_eq!(blocks[0].block.text, "deliberation");

    let (blocks, _, _) = codex_event_msg(&json!({
        "type":"dynamic_tool_call_request","callId":"c1","tool":"f","arguments":{"a":1}
    }));
    assert_eq!(blocks.len(), 1);
    assert!(blocks[0].block.text.contains("f"));
    assert!(blocks[0].block.text.contains("[c1]"));

    assert!(codex_completed_item(&json!({})).2);
    assert!(codex_completed_item(&json!({"type":"FutureItem"})).2);

    let (blocks, role, _) = codex_completed_item(
        &json!({"type":"UserMessage","content":[{"type":"text","text":"hi"}]}),
    );
    assert_eq!(role, "user");
    assert!(blocks[0].block.text.contains("hi"));

    let (blocks, _, unrecognized) = codex_completed_item(
        &json!({"type":"AgentMessage","content":[{"type":"image"},{"type":"audio"}]}),
    );
    assert!(!unrecognized);
    assert_eq!(blocks[0].block.text, "[image omitted]");
    assert_eq!(blocks[1].block.text, "[audio omitted]");
    assert!(codex_completed_item(&json!({"type":"AgentMessage","content":[{"type":"nope"}]})).2);
    assert!(!codex_completed_item(&json!({"type":"AgentMessage","content":null})).2);
    assert!(codex_completed_item(&json!({"type":"AgentMessage","content":5})).2);

    assert!(codex_completed_item(&json!({"type":"Reasoning","summary_text":[5]})).2);
    assert!(!codex_completed_item(&json!({"type":"Reasoning","summary_text":null})).2);
    assert!(codex_completed_item(&json!({"type":"Reasoning","summary_text":5})).2);

    assert!(codex_completed_item(&json!({"type":"Plan"})).0.is_empty());
    let (blocks, _, _) = codex_completed_item(&json!({"type":"Plan","text":"steps"}));
    assert_eq!(blocks[0].block.text, "steps");

    assert_eq!(codex_command_output(&json!({})), "");
}

#[test]
fn raw_top_level_token_and_zero_number_edges() {
    assert!(dsh_quiet_event("command/run"));
    assert!(!dsh_quiet_event("mystery"));

    assert_eq!(raw_top_level_token("{\"a", "a"), None);
    assert_eq!(raw_top_level_token("{\"version\":0}", "version"), Some("0"));
    assert_eq!(
        raw_top_level_token("{\"version\" : 1e-400 }", "version"),
        Some("1e-400")
    );
    assert_eq!(
        raw_top_level_token("{\"x\":{\"version\":0}}", "version"),
        None
    );
    assert_eq!(
        raw_top_level_token("{\"a\":\"b\",\"version\":0}", "version"),
        Some("0")
    );
    assert_eq!(raw_top_level_token("{\"version\":}", "version"), None);
    assert_eq!(raw_top_level_token("{\"version\":0}", "other"), None);

    assert!(zero_number_token("0"));
    assert!(zero_number_token("0.0"));
    assert!(zero_number_token("-0"));
    assert!(zero_number_token("0e-400"));
    assert!(!zero_number_token("1e-400"));
    assert!(!zero_number_token("123"));
    assert!(!zero_number_token("e5"));
    assert!(!zero_number_token("0e"));
    assert!(!zero_number_token("0ex"));
}

#[test]
fn numeric_positions_and_millis_cover_their_ranges() {
    assert_eq!(
        Position::parse(&json!(u64::MAX)),
        Some(Position::UInt(u64::MAX))
    );
    assert_eq!(Position::parse(&json!(-3)), Some(Position::Int(-3)));
    assert_eq!(
        Position::parse(&json!(1.5)),
        Some(Position::Float(1.5f64.to_bits()))
    );
    assert_eq!(Position::parse(&json!(0.0)), Some(Position::Int(0)));
    assert_eq!(
        Position::parse(&json!(1.8446744073709552e19)),
        Some(Position::UInt(u64::MAX))
    );
    assert_eq!(
        Position::parse(&json!(1e30)),
        Some(Position::Float(1e30f64.to_bits()))
    );
    assert_eq!(Position::parse(&json!(true)), None);
    assert_eq!(Position::parse(&json!("x")), None);

    assert_eq!(dsh_millis(&json!(5)), Some(5));
    assert_eq!(dsh_millis(&json!(1000.0)), Some(1000));
    assert_eq!(dsh_millis(&json!(0.5)), None);
    assert_eq!(dsh_millis(&json!(1e30)), None);
    assert_eq!(dsh_millis(&json!(true)), None);
}

#[test]
fn first_physical_row_reports_absent_and_malformed() {
    let absent = admitted("", false);
    assert!(matches!(first_physical_row(&absent), FirstRow::Absent));
    let overflowed = admitted("no-newline", true);
    assert!(matches!(first_physical_row(&overflowed), FirstRow::Absent));
    let malformed = admitted("not json", false);
    assert!(matches!(
        first_physical_row(&malformed),
        FirstRow::Malformed
    ));
    let value = admitted("{\"type\":\"session\",\"version\":0}", false);
    assert!(matches!(first_physical_row(&value), FirstRow::Value(_, _)));
}

#[test]
fn dsh_quiet_and_ignorable_rows_are_observed_without_refusal() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"command/run\",\"seq\":1,\"data\":{}}\n",
        "{\"type\":\"mystery/event\",\"seq\":2,\"ignorable\":true,\"data\":{}}\n",
        "{\"type\":\"tool/call\",\"seq\":3,\"data\":{\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":4,\"time\":5,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"x\"}}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 2);
}

#[test]
fn dsh_citation_suppression_runs_the_interval_index() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":1,\"time\":1,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"chunk\"}}}\n",
        "{\"type\":\"assistant/message\",\"seq\":2,\"time\":2,\"sourceEventSeqs\":[1],\"data\":{\"turn\":1,\"step\":1,\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"assembled\"}]}}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("assembled")]);
}

#[test]
fn dsh_message_blocks_count_wrong_typed_members() {
    let reasoning = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/message\",\"data\":{\"message\":{\"content\":[{\"type\":\"reasoning\",\"text\":7}]}}}\n",
    );
    assert_eq!(
        project_text(TranscriptKind::DshSession, reasoning).unrecognized_records,
        1
    );

    let wrong_content = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/message\",\"data\":{\"message\":{\"content\":7}}}\n",
    );
    assert_eq!(
        project_text(TranscriptKind::DshSession, wrong_content).unrecognized_records,
        1
    );

    let mixed = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/message\",\"data\":{\"message\":{\"content\":[{\"type\":\"text\",\"text\":7},{\"type\":\"nope\"}]}}}\n",
    );
    assert_eq!(
        project_text(TranscriptKind::DshSession, mixed).unrecognized_records,
        1
    );
}

#[test]
fn dsh_row_refuses_out_of_contract_rows_and_quiet_empty_chunks() {
    assert!(matches!(
        dsh_row(&json!("nope"), "\"nope\""),
        DshRow::Unrecognized
    ));
    assert!(matches!(dsh_row(&json!({}), "{}"), DshRow::Unrecognized));
    assert!(matches!(
        dsh_row(
            &json!({"type":"tool/result","seq":5,"sourceEventSeqs":5,"data":{}}),
            "{}"
        ),
        DshRow::Refused
    ));
    assert!(matches!(
        dsh_row(
            &json!({"type":"assistant/chunk","data":{"chunk":{"type":"text-delta","text":""}}}),
            "{}"
        ),
        DshRow::Quiet
    ));
    assert!(matches!(
        dsh_row(
            &json!({"type":"assistant/chunk","data":{"chunk":{"type":"reasoning-delta","text":""}}}),
            "{}"
        ),
        DshRow::Quiet
    ));
    assert!(matches!(
        dsh_row(&json!({"type":"command/run","data":{}}), "{}"),
        DshRow::Quiet
    ));
}

#[test]
fn dsh_citation_validation_refuses_out_of_contract_ranges() {
    assert_eq!(dsh_citations(&json!({}), None), Ok(Vec::new()));
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":5}), Some(1)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[]}), Some(1)),
        Ok(Vec::new())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[1]}), None),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[1]}), Some(1)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[[3,2]]}), Some(10)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[[-1,2]]}), Some(10)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[["x"]]}), Some(10)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[[1,2,3]]}), Some(10)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[[1,9]]}), Some(10)),
        Ok(vec![(1, 9)])
    );
}

#[test]
fn packed_rows_refuse_every_structural_violation() {
    let header = "{\"type\":\"session\",\"version\":0}\n";
    let cases = [
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":5}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":{\"turn\":\"x\",\"step\":0,\"index\":0,\"dt\":[],\"texts\":[\"a\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":{\"turn\":0,\"step\":\"x\",\"index\":0,\"dt\":[],\"texts\":[\"a\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":{\"turn\":0,\"step\":0,\"index\":\"x\",\"dt\":[],\"texts\":[\"a\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":-1,\"time0\":0,\"data\":{\"turn\":0,\"step\":0,\"index\":0,\"dt\":[],\"texts\":[\"a\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":{\"turn\":0,\"step\":0,\"index\":0,\"dt\":5,\"texts\":[\"a\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":{\"turn\":0,\"step\":0,\"index\":0,\"dt\":[],\"texts\":[]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":0,\"data\":{\"turn\":0,\"step\":0,\"index\":0,\"dt\":[0.5],\"texts\":[\"a\",\"b\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":0,\"time0\":9007199254740991,\"data\":{\"turn\":0,\"step\":0,\"index\":0,\"dt\":[9007199254740991],\"texts\":[\"a\",\"b\"]}}\n",
    ];
    for case in cases {
        let text = format!("{header}{case}");
        let projection = project_text(TranscriptKind::DshSession, &text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "case refused: {case}"
        );
    }
}

#[test]
fn packed_reasoning_chunks_project_reasoning() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"reasoning-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[5],\"texts\":[\"a\",\"b\"]}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::reasoning("a")]);
}

#[test]
fn none_kind_projects_an_empty_admitted_snapshot() {
    let projection = project_text(TranscriptKind::None, "{\"x\":1}\n");
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
}

#[test]
fn codex_call_ids_fall_back_to_the_id_member() {
    let (blocks, _, unrecognized) = codex_response_item(
        &json!({"type":"local_shell_call","id":"l1","action":{"command":"ls"}}),
    );
    assert!(!unrecognized);
    assert!(blocks[0].block.text.contains("[l1]"));

    let (blocks, _, unrecognized) =
        codex_response_item(&json!({"type":"tool_search_call","id":"s1","arguments":{"q":"x"}}));
    assert!(!unrecognized);
    assert!(blocks[0].block.text.contains("[s1]"));

    let (blocks, _, unrecognized) =
        codex_response_item(&json!({"type":"tool_search_output","id":"s1","tools":[{"name":"r"}]}));
    assert!(!unrecognized);
    assert!(blocks[0].block.text.contains("[s1]"));
}

#[test]
fn codex_mcp_end_reads_the_err_pointer() {
    let (blocks, _, unrecognized) = codex_event_msg(
        &json!({"type":"mcp_tool_call_end","call_id":"m1","result":{"Err":"boom"}}),
    );
    assert!(!unrecognized);
    assert!(blocks[0].block.text.contains("boom"));
}

#[test]
fn dsh_tool_result_falls_back_to_the_text_member() {
    let (blocks, unrecognized) = dsh_message_blocks(
        &json!({"content":[{"type":"tool-result","toolCallId":"t1","text":"out"}]}),
    );
    assert!(!unrecognized);
    assert!(blocks[0].block.text.contains("out"));
}

#[test]
fn citation_and_chunk_indexes_skip_events_without_positions() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":1,\"data\":{\"chunk\":{\"type\":\"text-delta\",\"text\":\"x\"}}}\n",
        "{\"type\":\"assistant/message\",\"seq\":2,\"sourceEventSeqs\":[1],\"data\":{\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"a\"}]}}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 2);
}

#[test]
fn unrecognized_and_omission_rows_without_a_sequence_are_counted() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"mystery/event\",\"ignorable\":true,\"data\":{}}\n",
        "{\"type\":\"assistant/chunk\",\"data\":{\"chunk\":{\"type\":\"future-variant\"}}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 2);
}

#[test]
fn dedicated_tool_events_deduplicate_blocks_within_one_event() {
    fn tool(id: &str) -> DshBlock {
        DshBlock {
            block: Block::tool("f"),
            tool: Some(DshTool {
                id: id.to_string(),
                direction: DshDirection::Call,
            }),
        }
    }
    let mut events = vec![DshEvent {
        blocks: vec![tool("t"), tool("t")],
        role: "assistant".to_string(),
        ts: String::new(),
        seq: Some(1),
        turn: Some(Position::Int(1)),
        step: Some(Position::Int(1)),
        chunk: false,
        assembly: false,
        cited: Vec::new(),
        dedicated: true,
    }];
    associate_dsh_tools(&mut events);
    assert_eq!(events[0].blocks.len(), 2);
}

#[test]
fn position_and_time_cover_the_float_edges() {
    assert_eq!(
        Position::parse(&json!(-1.8446744073709552e19)),
        Some(Position::Float((-1.8446744073709552e19f64).to_bits()))
    );

    assert_eq!(dsh_time(Some(&json!(5)), None), "5");
    assert_eq!(dsh_time(Some(&json!(-5)), None), "-5");
    assert_eq!(dsh_time(None, None), "");
    assert_eq!(dsh_time(Some(&json!(9.3e18)), None), "");
    assert_eq!(dsh_time(Some(&json!(0.5)), None), "");
}

#[test]
fn citation_ranges_cover_each_contract_boundary() {
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[9007199254740992i64]}), Some(10)),
        Err(())
    );
    assert_eq!(
        dsh_citations(
            &json!({"sourceEventSeqs":[[1, 9007199254740992i64]]}),
            Some(10)
        ),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[[1, 10]]}), Some(10)),
        Err(())
    );
    assert_eq!(
        dsh_citations(&json!({"sourceEventSeqs":[[1, 9]]}), Some(10)),
        Ok(vec![(1, 9)])
    );
}

#[test]
fn exact_keys_rejects_missing_required_and_extra_members() {
    let object: serde_json::Map<String, Value> = serde_json::from_str("{\"a\":1}").unwrap();
    assert!(exact_keys(&object, &["a"], &[]));
    assert!(exact_keys(&object, &["a"], &["b"]));
    assert!(!exact_keys(&object, &["a", "b"], &[]));

    let extra: serde_json::Map<String, Value> = serde_json::from_str("{\"a\":1,\"c\":2}").unwrap();
    assert!(!exact_keys(&extra, &["a"], &[]));
}

#[test]
fn packed_reasoning_with_an_empty_member_allocates_nothing() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"reasoning-chunks\",\"seq0\":10,\"time0\":1000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[5],\"texts\":[\"a\",\"\"]}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::reasoning("a")]);
}

#[test]
fn command_output_falls_back_when_only_stderr_is_recorded() {
    assert_eq!(codex_command_output(&json!({"stderr":"e"})), "e");
    assert_eq!(codex_command_output(&json!({"stdout":"o"})), "o");
}

#[test]
fn home_spellings_cover_the_short_forms() {
    assert!(valid_home("C:/x"));
    assert!(valid_home("//s"));
    assert!(!valid_home("C"));
    assert!(!valid_home("ab:"));
}

#[test]
fn raw_token_scans_whitespace_and_nonmatching_keys() {
    assert_eq!(
        raw_top_level_token("{\"version\":   0 }", "version"),
        Some("0")
    );
    assert_eq!(raw_top_level_token("{\"version\" 0}", "version"), None);
    assert_eq!(
        raw_top_level_token("{\"version\":0,\"a\":1}", "a"),
        Some("1")
    );
    assert_eq!(raw_top_level_token("{\"version\":", "version"), None);
    assert!(!zero_number_token("0x"));
    assert!(!zero_number_token("-e5"));
}

#[test]
fn home_time_and_key_scans_cover_their_false_branches() {
    assert!(!valid_home("C:x"));
    assert!(valid_home("\\\\s"));
    assert_eq!(
        raw_top_level_token("{\"version\": \t0}", "version"),
        Some("0")
    );
    assert_eq!(dsh_time(Some(&json!(9007199254740992i64)), None), "");
    assert_eq!(raw_top_level_token("\"version\"", "version"), None);
    assert_eq!(dsh_millis(&json!(-1e30)), None);

    let object: serde_json::Map<String, Value> = serde_json::from_str("{\"a\":1}").unwrap();
    assert!(!exact_keys(&object, &["b"], &[]));
}
#[test]
fn dsh_omission_with_a_sequence_is_observed() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":7,\"data\":{\"chunk\":{\"type\":\"future-variant\"}}}\n",
    );
    let projection = project_text(TranscriptKind::DshSession, text);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);
}

/// A quiet DSH row that carries no `seq` contributes no position: the
/// `dsh_seq` lookup returns `None`, the row is still quiet, and the
/// projection keeps zero counts and no turn.
#[test]
fn dsh_quiet_row_without_a_sequence_contributes_no_position() {
    let text = format!(
        "{}{}",
        "{\"type\":\"session\",\"version\":0}\n",
        row(json!({
            "type": "assistant/chunk",
            "data": {"turn": 1, "step": 1, "chunk": {"type": "text-delta", "text": ""}}
        }))
    );
    let projection = dsh(&text);
    assert_eq!(projection.unavailable, None);
    assert!(projection.turns.is_empty());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);
    assert!(!projection.truncated);
}
