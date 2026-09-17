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
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
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
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
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
        "{\"type\":\"text-chunks\",\"seq0\":20,\"time0\":2000,\"data\":{\"turn\":1,\"step\":2,\"index\":0,\"dt\":[],\"texts\":[\"c\"]}}\n",
    );
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    );
    assert!(projection.unavailable.is_none());
    // Each row coalesces to one chunk at its first member's stamp, and
    // separate rows never merge.
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("ab")]);
    assert_eq!(projection.turns[0].ts, "1000");
    assert_eq!(projection.turns[1].blocks, vec![Block::text("c")]);
    assert_eq!(projection.turns[1].ts, "2000");
}

#[test]
fn dsh_required_unknown_refuses_but_ignorable_omits() {
    let refused = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"future/event\"}\n",
    );
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
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
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
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
    for version in [
        "0", "0.0", "0e0", "-0", "0.000", "3", "3.0", "3e0", "30e-1", "0.3e1",
    ] {
        let header = format!("{{\"type\":\"session\",\"version\":{version}}}\n");
        let projection = dsh(&header);
        assert!(
            projection.unavailable.is_none(),
            "version {version} should admit"
        );
        assert!(projection.turns.is_empty());
    }
    for version in [
        "null",
        "false",
        "\"0\"",
        "\"3\"",
        "[]",
        "{}",
        "1",
        "2",
        "4",
        "-1",
        "-3",
        "0.5",
        "3.1",
        "3e1",
        "2.9999999999999999",
        "3.0000000000000001",
        "1e-400",
        "10e-400",
        "0.1e-400",
        "1.0e-400",
    ] {
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
fn digit_signature_reads_the_recorded_spelling() {
    for token in ["0", "0.0", "-0", "0e0", "0.000"] {
        assert_eq!(digit_signature(token), Some(String::new()), "{token}");
    }
    for token in ["1e-400", "10e-400", "0.1e-400", "1.0e-400", "1e3", "1000.0"] {
        assert_eq!(digit_signature(token), Some("1".to_string()), "{token}");
    }
    for token in ["3", "3.0", "3e0", "30e-1", "0.3e1", "-3", "3e1", "300"] {
        assert_eq!(digit_signature(token), Some("3".to_string()), "{token}");
    }
    assert_eq!(digit_signature("3.1"), Some("31".to_string()));
    assert_eq!(
        digit_signature("2.9999999999999999"),
        Some("29999999999999999".to_string())
    );
    assert_eq!(
        digit_signature("3.0000000000000001"),
        Some("30000000000000001".to_string())
    );
    // The dot-only mantissa carries no digit, which is the case the
    // at-least-one-digit condition exists for.
    for token in ["3e", ".", "e5", ""] {
        assert_eq!(digit_signature(token), None, "{token}");
    }
}

#[test]
fn raw_top_level_token_counts_and_decodes_member_names() {
    assert_eq!(
        raw_top_level_token("{\"version\":3,\"version\":3.0000000000000001}", "version"),
        RawToken::Many
    );
    assert_eq!(
        raw_top_level_token("{\"version\":3,\"\\u0076ersion\":3}", "version"),
        RawToken::Many
    );
    assert_eq!(
        raw_top_level_token("{\"data\":{\"version\":3}}", "version"),
        RawToken::None
    );
    assert_eq!(
        raw_top_level_token("{\"cwd\":\"version\"}", "version"),
        RawToken::None
    );
    assert_eq!(
        raw_top_level_token("{\"version\":  3e0 }", "version"),
        RawToken::One("3e0")
    );
    assert_eq!(
        raw_top_level_token("{\"\\u0076ersion\":3}", "version"),
        RawToken::One("3")
    );
}

#[test]
fn dsh_header_binds_the_version_token_to_the_decoded_member() {
    // The escaped name decodes to `version`, so its one token admits.
    let text = concat!(
        "{\"type\":\"session\",\"\\u0076ersion\":3,\"delegationDepth\":0}\n",
        "{\"type\":\"system/message\",\"data\":{}}\n",
    );
    let projection = dsh(text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert!(projection.turns.is_empty());

    // The same decoded member with a rounding artefact refuses, because
    // the parsed three cannot admit alone.
    let text =
        "{\"type\":\"session\",\"\\u0076ersion\":3.0000000000000001,\"delegationDepth\":0}\n";
    assert_eq!(dsh(text).unavailable, Some(Unavailable::UnsupportedFormat));

    // Four duplicate headers each refuse as an ambiguous recording,
    // whichever occurrence the parser retains.
    for text in [
        "{\"type\":\"session\",\"version\":3,\"version\":3.0000000000000001}\n",
        "{\"type\":\"session\",\"version\":3.0000000000000001,\"version\":3}\n",
        "{\"type\":\"session\",\"version\":3,\"\\u0076ersion\":3}\n",
        "{\"type\":\"session\",\"version\":0,\"version\":3}\n",
    ] {
        let projection = dsh(text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{text}"
        );
        assert_eq!(projection.skipped_lines, 0, "{text}");
        assert_eq!(projection.unrecognized_records, 0, "{text}");
        assert!(projection.turns.is_empty());
    }
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
fn dsh_packed_multi_member_fragments_coalesce_while_ordinary_rows_keep_their_boundaries() {
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

    // The same content packed coalesces each consecutive run into one
    // chunk at its first member's stamp; ordinary rows keep six turns.
    let coalesced = dsh(&packed);
    assert!(coalesced.unavailable.is_none());
    assert_eq!(coalesced.skipped_lines, 0);
    assert_eq!(coalesced.unrecognized_records, 0);
    assert_eq!(coalesced.turns.len(), 2);
    assert_eq!(coalesced.turns[0].blocks, vec![Block::text("abc")]);
    assert_eq!(coalesced.turns[0].ts, "1000");
    assert_eq!(coalesced.turns[1].blocks, vec![Block::reasoning("r1r2r3")]);
    assert_eq!(coalesced.turns[1].ts, "1000");
}

#[test]
fn dsh_single_member_packed_rows_match_their_ordinary_encoding() {
    let ordinary = [
        row(json!({"type":"session","version":0})),
        dsh_chunk(10, 1000, 1, 1, "only"),
    ]
    .concat();
    let packed = [
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[], &["only"]),
    ]
    .concat();
    assert_eq!(dsh(&packed), dsh(&ordinary));
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

    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
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
            if packed {
                // The two uncited packed members coalesce into one chunk.
                assert_eq!(projection.turns.len(), 2, "packed={packed}");
                assert_eq!(projection.turns[0].blocks, vec![Block::text("c10c11")]);
                assert_eq!(projection.turns[1].blocks, vec![Block::text("assembled")]);
            } else {
                assert_eq!(projection.turns.len(), 3, "packed={packed}");
                assert_eq!(projection.turns[0].blocks, vec![Block::text("c10")]);
                assert_eq!(projection.turns[1].blocks, vec![Block::text("c11")]);
                assert_eq!(projection.turns[2].blocks, vec![Block::text("assembled")]);
            }
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

/// Citation suppression splits a packed row's run, so the display cap can
/// stop between the surviving chunks: member 10's chunk fits exactly and
/// member 12's does not.
#[test]
fn dsh_display_cap_stops_between_suppressed_packed_members() {
    let fits = "x".repeat(DISPLAY_CAP - DISPLAY_EVENT_COST);
    let text = format!(
        "{}{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[1, 1], &[fits.as_str(), "middle", "tail"]),
        dsh_assembly(20, 1020, 1, 1, Some(json!([11]))),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert!(projection.truncated);
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text(&fits)]);
}

/// A packed chunk is indivisible under the charged budget: two unsuppressed
/// members whose combined text barely overflows retain no turn at all, and
/// one byte less retains the whole chunk at equality. The reader never
/// keeps only the first member to evade the coalesced boundary.
#[test]
fn dsh_display_cap_never_splits_an_indivisible_packed_chunk() {
    let first = "x".repeat(DISPLAY_CAP - DISPLAY_EVENT_COST - 1);
    let over = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[1], &[first.as_str(), "yy"]),
    );
    let projection = dsh(&over);
    assert!(projection.unavailable.is_none());
    assert!(projection.truncated);
    assert_eq!(projection.unrecognized_records, 0);
    assert!(projection.turns.is_empty());

    let exact = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[1], &[first.as_str(), "y"]),
    );
    let projection = dsh(&exact);
    assert!(projection.unavailable.is_none());
    assert!(!projection.truncated);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(
        projection.turns[0].blocks[0].text.len(),
        DISPLAY_CAP - DISPLAY_EVENT_COST
    );
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
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
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
    // The two text members coalesce into one chunk at the first member's
    // zero stamp; a later member's timestamp creates no second turn.
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("ab")]);
    assert_eq!(projection.turns[0].ts, "0");
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
    // A packed row contributes one observed identity span and no payload
    // object per member, so a row of empty members costs one span.
    let mut spans = Vec::new();
    let object = json!({"type":"text-chunks","seq0":5,"time0":100,
        "data":{"turn":1,"step":1,"index":0,"dt":[0,0],"texts":["","",""]}});
    packed_facts(&object, &object.to_string(), &mut spans).expect("complete packed row");
    assert_eq!(spans, vec![(5, 7)], "the whole member span is observed");

    let mut spans = Vec::new();
    let object = json!({"type":"tool-call-chunks","seq0":5,"time0":100,
        "data":{"turn":1,"step":1,"index":0,"dt":[0],"args":["a","b"],"id":"c1"}});
    packed_facts(&object, &object.to_string(), &mut spans).expect("complete packed row");
    assert_eq!(spans, vec![(5, 6)]);

    // Neither row supplies a displayed turn.
    let text = format!(
        "{}{}{}",
        row(json!({"type":"session","version":0})),
        row(json!({"type":"text-chunks","seq0":5,"time0":100,
            "data":{"turn":1,"step":1,"index":0,"dt":[0,0],"texts":["","",""]}})),
        row(json!({"type":"tool-call-chunks","seq0":20,"time0":100,
            "data":{"turn":1,"step":1,"index":0,"dt":[0],"args":["a","b"],"id":"c1"}})),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none());
    assert!(projection.turns.is_empty());
}

/// Shape (c) of #277: a blockless ordinary DSH event (`tool/result` or
/// `user/message` with absent data) is pure retained waste. Its sequence
/// must still be observed so duplicate identity stays ambiguous, but it
/// must never be pushed into the retained prefix. The final empty filter
/// would hide the difference, so this pins retention itself: removing the
/// guard retains the empty events and fails this assertion.
#[test]
fn blockless_ordinary_dsh_events_are_observed_but_not_retained() {
    let event = |seq: i64, blocks: Vec<DshBlock>| DshEvent {
        blocks,
        role: "tool".to_string(),
        ts: "1".to_string(),
        seq: Some(seq),
        turn: Some(Position::Int(1)),
        step: Some(Position::Int(1)),
        chunk: false,
        assembly: false,
        cited: Vec::new(),
        dedicated: true,
    };

    let mut observed = Vec::new();
    let mut events = Vec::new();
    retain_ordinary_events(
        vec![
            event(7, Vec::new()),
            event(8, vec![DshBlock::plain(Block::tool_result("ok"))]),
            event(9, Vec::new()),
        ],
        &mut observed,
        &mut events,
    );
    assert_eq!(
        observed,
        vec![(7, 7), (8, 8), (9, 9)],
        "every recorded sequence is still observed"
    );
    assert_eq!(events.len(), 1, "only the block-bearing event is retained");
    assert_eq!(events[0].seq, Some(8));
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

    let dup = codex_id(Some("dup")).expect("a nonempty id is shared");
    let mut records = vec![CodexRecord {
        blocks: vec![
            CodexBlock::identified(Block::tool("a"), CodexFact::Call, Some(&dup)),
            CodexBlock::identified(Block::tool("b"), CodexFact::Call, Some(&dup)),
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

/// A row's recorded id is held once and shared by every block that cites
/// it: a row of many empty content members and one long id costs one id
/// allocation, not one copy per member and another per association key,
/// so the bounded source cannot amplify through identity.
#[test]
fn codex_record_id_is_allocated_once_and_shared_by_its_blocks() {
    assert!(codex_id(None).is_none());
    assert!(
        codex_id(Some("")).is_none(),
        "an empty id identifies nothing"
    );

    let long = "x".repeat(32 * 1024);
    let members: Vec<serde_json::Value> = (0..1000)
        .map(|_| json!({"type":"output_text","text":""}))
        .collect();
    let message = json!({
        "type":"response_item",
        "payload":{"type":"message","role":"assistant","id":long,"content":members}
    });
    let record = codex_row(&message);
    assert_eq!(record.blocks.len(), 1000);
    let (fact, first) = record.blocks[0].fact.as_ref().expect("identified");
    assert_eq!(*fact, CodexFact::Message);
    assert_eq!(&**first, long.as_str());
    assert!(
        record.blocks.iter().all(|block| block
            .fact
            .as_ref()
            .is_some_and(|(_, id)| Rc::ptr_eq(id, first))),
        "every block cites the same allocation"
    );
    assert_eq!(
        Rc::strong_count(first),
        1000,
        "one count per block, no copies"
    );

    // Association keys borrow the same allocation and release it after
    // the pass: the projection is unchanged and nothing was copied.
    let mut records = vec![record];
    associate_codex(&mut records);
    let (_, shared) = records[0].blocks[0].fact.as_ref().expect("identified");
    assert_eq!(records[0].blocks.len(), 1000);
    assert_eq!(Rc::strong_count(shared), 1000);

    // The pass must hold that one allocation while its keys are live, not
    // only before and after: observe both key-construction sites with a
    // canonical and a fallback record built from one id. A regression that
    // copies the bytes per key fails on pointer identity and count here,
    // during the pass, which the before/after checks cannot see.
    let id = codex_id(Some("shared-canary")).expect("a nonempty id");
    let mut pair = vec![
        CodexRecord {
            blocks: vec![CodexBlock::identified(
                Block::tool("a"),
                CodexFact::Call,
                Some(&id),
            )],
            role: String::new(),
            ts: String::new(),
            unrecognized: false,
            canonical: true,
        },
        CodexRecord {
            blocks: vec![CodexBlock::identified(
                Block::tool("b"),
                CodexFact::Call,
                Some(&id),
            )],
            role: String::new(),
            ts: String::new(),
            unrecognized: false,
            canonical: false,
        },
    ];
    drop(id);
    let mut during = Vec::new();
    associate_codex_observed(&mut pair, |site, source, key| {
        during.push((site, Rc::ptr_eq(source, key), Rc::strong_count(source)));
    });
    assert_eq!(
        during,
        vec![
            (CodexKeySite::Count, true, 3),
            (CodexKeySite::Count, true, 4),
            (CodexKeySite::Lookup, true, 5),
        ],
        "each live key is the record's own allocation, with only the pass's references"
    );
    assert_eq!(pair[0].blocks.len(), 1, "the canonical block stays");
    assert!(
        pair[1].blocks.is_empty(),
        "the proven fallback block is removed"
    );

    let full = codex(&row(message));
    assert_eq!(full.turns.len(), 1);
    assert_eq!(full.turns[0].blocks.len(), 1000);
    assert!(!full.truncated);

    // The completed-item mirror and the two-block tool families share
    // the same way.
    let (blocks, _, _) = codex_completed_item(&json!({
        "type":"AgentMessage","id":"i1",
        "content":[{"type":"text","text":""},{"type":"text","text":""},{"type":"image"}]
    }));
    let (_, mirror) = blocks[0].fact.as_ref().expect("identified");
    assert_eq!(&**mirror, "i1");
    assert_eq!(Rc::strong_count(mirror), 3);
    let (blocks, _, _) =
        codex_completed_item(&json!({"type":"CommandExecution","id":"c1","command":"ls"}));
    let (call, call_id) = blocks[0].fact.as_ref().expect("identified");
    let (result, result_id) = blocks[1].fact.as_ref().expect("identified");
    assert_eq!((*call, *result), (CodexFact::Call, CodexFact::Result));
    assert!(Rc::ptr_eq(call_id, result_id));
    assert_eq!(Rc::strong_count(call_id), 2);
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
    assert!(dsh_quiet_event("command/run", DshVersion::Zero));
    assert!(!dsh_quiet_event("mystery", DshVersion::Zero));

    assert_eq!(raw_top_level_token("{\"a", "a"), RawToken::None);
    assert_eq!(
        raw_top_level_token("{\"version\":0}", "version"),
        RawToken::One("0")
    );
    assert_eq!(
        raw_top_level_token("{\"version\" : 1e-400 }", "version"),
        RawToken::One("1e-400")
    );
    assert_eq!(
        raw_top_level_token("{\"x\":{\"version\":0}}", "version"),
        RawToken::None
    );
    assert_eq!(
        raw_top_level_token("{\"a\":\"b\",\"version\":0}", "version"),
        RawToken::One("0")
    );
    assert_eq!(
        raw_top_level_token("{\"version\":}", "version"),
        RawToken::None
    );
    assert_eq!(
        raw_top_level_token("{\"version\":0}", "other"),
        RawToken::None
    );

    assert_eq!(digit_signature("0"), Some(String::new()));
    assert_eq!(digit_signature("0.0"), Some(String::new()));
    assert_eq!(digit_signature("-0"), Some(String::new()));
    assert_eq!(digit_signature("0e-400"), Some(String::new()));
    assert_eq!(digit_signature("1e-400"), Some("1".to_string()));
    assert_eq!(digit_signature("123"), Some("123".to_string()));
    assert_eq!(digit_signature("e5"), None);
    assert_eq!(digit_signature("0e"), None);
    assert_eq!(digit_signature("0ex"), None);
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
        dsh_row(&json!("nope"), "\"nope\"", DshVersion::Zero),
        DshRow::Unrecognized
    ));
    assert!(matches!(
        dsh_row(&json!({}), "{}", DshVersion::Zero),
        DshRow::Unrecognized
    ));
    assert!(matches!(
        dsh_row(
            &json!({"type":"tool/result","seq":5,"sourceEventSeqs":5,"data":{}}),
            "{}",
            DshVersion::Zero
        ),
        DshRow::Refused
    ));
    assert!(matches!(
        dsh_row(
            &json!({"type":"assistant/chunk","data":{"chunk":{"type":"text-delta","text":""}}}),
            "{}",
            DshVersion::Zero
        ),
        DshRow::Quiet
    ));
    assert!(matches!(
        dsh_row(
            &json!({"type":"assistant/chunk","data":{"chunk":{"type":"reasoning-delta","text":""}}}),
            "{}",
            DshVersion::Zero
        ),
        DshRow::Quiet
    ));
    assert!(matches!(
        dsh_row(
            &json!({"type":"command/run","data":{}}),
            "{}",
            DshVersion::Zero
        ),
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
    // Consecutive reasoning members coalesce into one chunk.
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::reasoning("ab")]);
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
    let mut retained = vec![DshEvent {
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
    let mut assemblies = Vec::new();
    let mut dedicated = HashMap::new();
    collect_ordinary_facts(&mut retained, 3, true, &mut assemblies, &mut dedicated);
    assert_eq!(
        dedicated.get(&(
            "t".to_string(),
            DshDirection::Call,
            Position::Int(1),
            Position::Int(1)
        )),
        Some(&1),
        "one dedicated event counts its key once however many blocks carry it"
    );
    assert!(
        retained.is_empty(),
        "the observed event is released, not retained"
    );
}

#[test]
fn position_and_time_cover_the_float_edges() {
    assert_eq!(
        Position::parse(&json!(-1.8446744073709552e19)),
        Some(Position::Float((-1.8446744073709552e19f64).to_bits()))
    );

    assert_eq!(dsh_time(Some(&json!(5)), RawToken::One("5")), "5");
    assert_eq!(dsh_time(Some(&json!(-5)), RawToken::One("-5")), "-5");
    assert_eq!(dsh_time(None, RawToken::None), "");
    assert_eq!(dsh_time(Some(&json!(9.3e18)), RawToken::One("9.3e18")), "");
    assert_eq!(dsh_time(Some(&json!(0.5)), RawToken::One("0.5")), "");
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
        RawToken::One("0")
    );
    assert_eq!(
        raw_top_level_token("{\"version\" 0}", "version"),
        RawToken::None
    );
    assert_eq!(
        raw_top_level_token("{\"version\":0,\"a\":1}", "a"),
        RawToken::One("1")
    );
    assert_eq!(
        raw_top_level_token("{\"version\":", "version"),
        RawToken::None
    );
    assert_eq!(digit_signature("0x"), None);
    assert_eq!(digit_signature("-e5"), None);
}

#[test]
fn home_time_and_key_scans_cover_their_false_branches() {
    assert!(!valid_home("C:x"));
    assert!(valid_home("\\\\s"));
    assert_eq!(
        raw_top_level_token("{\"version\": \t0}", "version"),
        RawToken::One("0")
    );
    assert_eq!(
        dsh_time(
            Some(&json!(9007199254740992i64)),
            RawToken::One("9007199254740992")
        ),
        ""
    );
    assert_eq!(
        raw_top_level_token("\"version\"", "version"),
        RawToken::None
    );
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

// -------------------------------------- version-three admission (D3-D8)

/// A version-three header with an explicit `isSeeded` spelling, or none.
fn dsh3_header(seed: Option<&str>) -> String {
    match seed {
        Some(seed) => format!(
            "{{\"type\":\"session\",\"version\":3,\"delegationDepth\":0,\"isSeeded\":{seed}}}\n"
        ),
        None => "{\"type\":\"session\",\"version\":3,\"delegationDepth\":0}\n".to_string(),
    }
}

#[test]
fn dsh_quiet_lists_hold_the_version_relation() {
    let mut expected: Vec<&str> = DSH_QUIET_ZERO
        .iter()
        .copied()
        .filter(|name| !matches!(*name, "tool/code-dispatch" | "tool/code-dispatch-start"))
        .collect();
    expected.extend([
        "system/message",
        "deliverables/presented",
        "feedback/message-delete",
        "feedback/message-put",
        "subagent/catalog",
        "tool/ptc-dispatch",
        "tool/ptc-dispatch-start",
    ]);
    expected.sort_unstable();
    let mut three = DSH_QUIET_THREE.to_vec();
    three.sort_unstable();
    assert_eq!(three, expected);
    assert_eq!(DSH_QUIET_ZERO.len(), 46);
    assert_eq!(DSH_QUIET_THREE.len(), 51);
    for list in [DSH_QUIET_ZERO, DSH_QUIET_THREE] {
        for forbidden in [
            "assistant/attempt",
            "assistant/chunk",
            "text-chunks",
            "reasoning-chunks",
            "tool-call-chunks",
        ] {
            assert!(!list.contains(&forbidden), "{forbidden}");
        }
    }
}

#[test]
fn dsh_v3_quiet_names_project_without_counts() {
    let quiet = [
        "system/message",
        "deliverables/presented",
        "feedback/message-delete",
        "feedback/message-put",
        "subagent/catalog",
        "tool/ptc-dispatch",
        "tool/ptc-dispatch-start",
        "todo/write",
        "turn/end",
        "request/header",
    ];
    let mut rows = String::new();
    rows.push_str(&row(json!({
        "type": "user/message",
        "seq": 1,
        "time": 1,
        "data": {"content": [{"type": "text", "text": "q"}]}
    })));
    for (offset, kind) in quiet.iter().enumerate() {
        let seq = offset as i64 + 2;
        let surface = if *kind == "system/message" {
            json!({"op": "replace", "startSeq": 1, "endSeq": 1})
        } else {
            json!("append")
        };
        rows.push_str(&row(json!({
            "type": kind,
            "seq": seq,
            "surfaceOp": surface,
            "data": {"content": [{"type": "text", "text": "quiet"}]}
        })));
    }
    let projection = dsh(&format!("{}{rows}", dsh3_header(Some("false"))));
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("q")]);
}

#[test]
fn dsh_v3_fragment_and_packed_names_are_required_unknowns() {
    for kind in [
        "assistant/chunk",
        "text-chunks",
        "reasoning-chunks",
        "tool-call-chunks",
        "tool/code-dispatch",
        "tool/code-dispatch-start",
    ] {
        let base = row(json!({
            "type": "user/message",
            "seq": 1,
            "data": {"content": [{"type": "text", "text": "q"}]}
        }));
        let refused = format!(
            "{}{base}{}",
            dsh3_header(Some("false")),
            row(json!({"type": kind, "seq": 2, "data": {}})),
        );
        let projection = dsh(&refused);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{kind}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{kind}");
        assert!(projection.turns.is_empty(), "{kind}");

        let omitted = format!(
            "{}{base}{}",
            dsh3_header(Some("false")),
            row(json!({"type": kind, "seq": 2, "ignorable": true, "data": {}})),
        );
        let projection = dsh(&omitted);
        assert!(projection.unavailable.is_none(), "{kind}");
        assert_eq!(projection.unrecognized_records, 1, "{kind}");
        assert_eq!(projection.turns.len(), 1, "{kind}");
    }
}

#[test]
fn dsh_v3_invalid_packed_row_is_unknown_before_decoding() {
    let base = row(json!({
        "type": "user/message",
        "seq": 1,
        "data": {"content": [{"type": "text", "text": "q"}]}
    }));
    let invalid = "{\"type\":\"text-chunks\",\"seq\":2,\"time0\":0,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[],\"texts\":\"x\"}}\n";
    let projection = dsh(&format!("{}{base}{invalid}", dsh3_header(Some("false"))));
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);

    let marked = "{\"type\":\"text-chunks\",\"ignorable\":true,\"seq\":2,\"time0\":0,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[],\"texts\":\"x\"}}\n";
    let projection = dsh(&format!("{}{base}{marked}", dsh3_header(Some("false"))));
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);

    // The same hostile row under version zero refuses with one count
    // whether or not it carries the marker, because the packed decoder
    // reads its shape.
    for extra in ["", ",\"ignorable\":true"] {
        let packed = format!(
            "{{\"type\":\"text-chunks\"{extra},\"seq\":2,\"time0\":0,\"data\":{{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[],\"texts\":\"x\"}}}}\n"
        );
        let text = format!("{}{packed}", row(json!({"type": "session", "version": 0})));
        let projection = dsh(&text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{extra}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{extra}");
    }
}

#[test]
fn dsh_v3_only_names_are_unknown_under_version_zero() {
    for kind in [
        "assistant/attempt",
        "deliverables/presented",
        "feedback/message-delete",
        "feedback/message-put",
        "subagent/catalog",
        "system/message",
        "tool/ptc-dispatch",
        "tool/ptc-dispatch-start",
    ] {
        let base = row(json!({
            "type": "user/message",
            "seq": 1,
            "data": {"content": [{"type": "text", "text": "q"}]}
        }));
        let refused = format!(
            "{}{base}{}",
            row(json!({"type": "session", "version": 0})),
            row(json!({"type": kind, "seq": 2, "data": {}})),
        );
        let projection = dsh(&refused);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{kind}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{kind}");
        assert!(projection.turns.is_empty(), "{kind}");

        let omitted = format!(
            "{}{base}{}",
            row(json!({"type": "session", "version": 0})),
            row(json!({"type": kind, "seq": 2, "ignorable": true, "data": {}})),
        );
        let projection = dsh(&omitted);
        assert!(projection.unavailable.is_none(), "{kind}");
        assert_eq!(projection.unrecognized_records, 1, "{kind}");
        assert_eq!(projection.turns.len(), 1, "{kind}");
    }
}

#[test]
fn dsh_version_zero_rules_the_three_sampled_types() {
    let base = format!(
        "{}{}{}",
        row(json!({"type": "session", "version": 0})),
        row(json!({"type": "todo/write", "seq": 1, "data": {}})),
        row(json!({"type": "turn/end", "seq": 2, "data": {}})),
    );
    let user = row(json!({
        "type": "user/message",
        "seq": 3,
        "data": {"content": [{"type": "text", "text": "q"}]}
    }));
    let system = row(json!({
        "type": "system/message",
        "seq": 4,
        "data": {"content": [{"type": "text", "text": "sys"}]}
    }));
    let projection = dsh(&format!("{base}{user}{system}"));
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert!(projection.turns.is_empty());
    assert_eq!(projection.unrecognized_records, 1);

    let marked = row(json!({
        "type": "system/message",
        "seq": 4,
        "ignorable": true,
        "data": {"content": [{"type": "text", "text": "sys"}]}
    }));
    let projection = dsh(&format!("{base}{user}{marked}"));
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
}

#[test]
fn dsh_v3_attempt_is_counted_and_shows_nothing() {
    for marker in ["", ",\"ignorable\":true"] {
        let attempt = format!(
            "{{\"type\":\"assistant/attempt\"{marker},\"seq\":2,\"data\":{{\"stream\":[{{\"type\":\"text-delta\",\"text\":\"x\"}}]}}}}\n"
        );
        let rows = format!(
            "{}{attempt}{}",
            row(
                json!({"type": "user/message", "seq": 1, "data": {"content": [{"type": "text", "text": "q"}]}})
            ),
            row(
                json!({"type": "assistant/message", "seq": 3, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "text", "text": "a"}]}}})
            ),
        );
        let projection = dsh(&format!("{}{rows}", dsh3_header(Some("false"))));
        assert!(projection.unavailable.is_none(), "{marker}");
        assert_eq!(projection.unrecognized_records, 1, "{marker}");
        assert_eq!(projection.turns.len(), 2, "{marker}");
        assert_eq!(projection.turns[0].blocks, vec![Block::text("q")]);
        assert_eq!(projection.turns[1].blocks, vec![Block::text("a")]);
    }
}

#[test]
fn dsh_v3_content_projects_on_the_writers_definitions() {
    let user = row(json!({
        "type": "user/message",
        "seq": 1,
        "time": 1000,
        "surfaceOp": "append",
        "data": {
            "id": "u1",
            "role": "user",
            "source": {"kind": "cli"},
            "content": [{"type": "text", "text": "q"}],
            "turn": 7,
            "step": 9
        }
    }));
    let call = row(json!({
        "type": "tool/call",
        "seq": 2,
        "time": 1001,
        "data": {"callId": "c1", "name": "Read", "arguments": {"path": "a"}, "turn": 1, "step": 1}
    }));
    let result = row(json!({
        "type": "tool/result",
        "seq": 3,
        "time": 1002,
        "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "tool-result", "toolCallId": "c1", "content": "out"}]}}
    }));
    let assistant = row(json!({
        "type": "assistant/message",
        "seq": 4,
        "time": 1003,
        "surfaceOp": "append",
        "data": {
            "turn": 1,
            "step": 1,
            "stream": [{"type": "text-delta", "text": "ignored"}],
            "usage": {"input": 1},
            "interrupted": false,
            "message": {"content": [
                {"type": "reasoning", "text": "r"},
                {"type": "text", "text": "a"},
                {"type": "image", "attachment": {"x": 1}},
                {"type": "file", "name": "f"}
            ]}
        }
    }));
    for header in [
        dsh3_header(Some("false")),
        row(json!({"type": "session", "version": 0})),
    ] {
        let text = format!("{header}{user}{call}{result}{assistant}");
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "{projection:?}");
        assert_eq!(projection.unrecognized_records, 1, "{header}");
        assert_eq!(projection.skipped_lines, 0);
        assert_eq!(projection.turns.len(), 4);
        assert_eq!(projection.turns[0].role, "user");
        assert_eq!(projection.turns[0].ts, "1000");
        assert_eq!(projection.turns[0].blocks, vec![Block::text("q")]);
        assert_eq!(projection.turns[1].role, "assistant");
        assert_eq!(
            projection.turns[1].blocks,
            vec![Block::tool("Read {\"path\":\"a\"} [c1]")]
        );
        assert_eq!(projection.turns[2].role, "tool");
        assert_eq!(
            projection.turns[2].blocks,
            vec![Block::tool_result("out [c1]")]
        );
        assert_eq!(projection.turns[3].role, "assistant");
        assert_eq!(
            projection.turns[3].blocks,
            vec![
                Block::reasoning("r"),
                Block::text("a"),
                Block::omitted("[image omitted]"),
            ]
        );
    }

    // A string `content` projects as the one text block.
    let stringly = row(json!({
        "type": "user/message",
        "seq": 1,
        "data": {"content": "plain"}
    }));
    let projection = dsh(&format!("{}{stringly}", dsh3_header(Some("false"))));
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("plain")]);
}

fn dsh3_embedded_call_rows() -> String {
    format!(
        "{}{}{}",
        row(
            json!({"type": "assistant/message", "seq": 1, "time": 1, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "text", "text": "see"}, {"type": "tool-call", "id": "c1", "name": "f", "arguments": {}}]}}})
        ),
        row(
            json!({"type": "tool/call", "seq": 2, "time": 2, "data": {"turn": 1, "step": 1, "callId": "c1", "name": "f", "arguments": {}}})
        ),
        row(
            json!({"type": "tool/result", "seq": 3, "time": 3, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "tool-result", "toolCallId": "c1", "content": "out"}]}}})
        ),
    )
}

#[test]
fn dsh_v3_seeded_gate_suppresses_only_for_boolean_false() {
    for seed in [Some("false"), None] {
        let text = format!("{}{}", dsh3_header(seed), dsh3_embedded_call_rows());
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "{seed:?}");
        assert_eq!(projection.unrecognized_records, 0, "{seed:?}");
        assert_eq!(projection.turns.len(), 3, "{seed:?}");
    }
    // Only `false` suppresses the embedded copy.
    let suppressed = dsh(&format!(
        "{}{}",
        dsh3_header(Some("false")),
        dsh3_embedded_call_rows()
    ));
    assert_eq!(suppressed.turns[0].blocks, vec![Block::text("see")]);
    for seed in ["true", "\"yes\"", "{}", "null"] {
        let text = format!("{}{}", dsh3_header(Some(seed)), dsh3_embedded_call_rows());
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "{seed}");
        assert_eq!(projection.unrecognized_records, 0, "{seed}");
        assert_eq!(projection.turns.len(), 3, "{seed}");
        assert_eq!(projection.turns[0].blocks.len(), 2, "{seed}");
        assert!(projection.turns[0].blocks[0].text.contains("see"), "{seed}");
        assert!(projection.turns[0].blocks[1].text.contains("c1"), "{seed}");
    }

    // Under version zero the association runs whatever `isSeeded` carries.
    for seed in [
        None,
        Some("false"),
        Some("true"),
        Some("\"yes\""),
        Some("{}"),
        Some("null"),
    ] {
        let header = match seed {
            None => row(json!({"type": "session", "version": 0})),
            Some(seed) => format!("{{\"type\":\"session\",\"version\":0,\"isSeeded\":{seed}}}\n"),
        };
        let text = format!("{header}{}", dsh3_embedded_call_rows());
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "{seed:?}");
        assert_eq!(projection.turns.len(), 3, "{seed:?}");
        assert_eq!(
            projection.turns[0].blocks,
            vec![Block::text("see")],
            "{seed:?}"
        );
    }
}

#[test]
fn dsh_v3_ambiguous_identity_keeps_both_dedicated_calls_and_the_copy() {
    let text = format!(
        "{}{}{}",
        row(
            json!({"type": "assistant/message", "seq": 1, "time": 1, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "text", "text": "see"}, {"type": "tool-call", "id": "c1", "name": "f", "arguments": {}}]}}})
        ),
        row(
            json!({"type": "tool/call", "seq": 2, "time": 2, "data": {"turn": 1, "step": 1, "callId": "c1", "name": "f", "arguments": {}}})
        ),
        row(
            json!({"type": "tool/call", "seq": 3, "time": 3, "data": {"turn": 1, "step": 1, "callId": "c1", "name": "f", "arguments": {}}})
        ),
    );
    let projection = dsh(&format!("{}{text}", dsh3_header(Some("false"))));
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 3);
    assert_eq!(projection.turns[0].blocks.len(), 2);
}

#[test]
fn dsh_v3_user_message_positions_do_not_own_an_embedded_result() {
    let rows = format!(
        "{}{}",
        row(
            json!({"type": "user/message", "seq": 1, "time": 1, "data": {"turn": 1, "step": 1, "content": [{"type": "text", "text": "see"}, {"type": "tool-result", "toolCallId": "c1", "content": "out"}]}})
        ),
        row(
            json!({"type": "tool/result", "seq": 2, "time": 2, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "tool-result", "toolCallId": "c1", "content": "out"}]}}})
        ),
    );
    for seed in [Some("false"), Some("true"), None] {
        let text = format!("{}{rows}", dsh3_header(seed));
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "{seed:?}");
        assert_eq!(projection.unrecognized_records, 0, "{seed:?}");
        assert_eq!(projection.turns.len(), 2, "{seed:?}");
        assert_eq!(projection.turns[0].role, "user", "{seed:?}");
        assert_eq!(
            projection.turns[0].blocks,
            vec![Block::text("see"), Block::tool_result("out [c1]")],
            "{seed:?}"
        );
        assert_eq!(projection.turns[1].role, "tool", "{seed:?}");
        assert_eq!(
            projection.turns[1].blocks,
            vec![Block::tool_result("out [c1]")],
            "{seed:?}"
        );
    }
    // Version zero reads the user message's positions, so the dedicated
    // result owns the embedded copy.
    let zero = format!("{}{rows}", row(json!({"type": "session", "version": 0})));
    let projection = dsh(&zero);
    assert!(projection.unavailable.is_none());
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("see")]);
}

#[test]
fn dsh_v3_citations_share_the_grammar_and_suppress_nothing() {
    let prefix = concat!(
        "{\"type\":\"session\",\"version\":3,\"delegationDepth\":0,\"isSeeded\":false}\n",
        "{\"type\":\"system/message\",\"seq\":3,\"time\":3,\"data\":{}}\n",
        "{\"type\":\"assistant/message\",\"seq\":4,\"time\":4,\"data\":{\"turn\":1,\"step\":1,\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"a\"}]}}}\n",
        "{\"type\":\"tool/call\",\"seq\":5,\"time\":5,\"data\":{\"callId\":\"c1\",\"name\":\"f\",\"arguments\":{}}}\n",
    );
    let text = format!(
        "{prefix}{}{}{}",
        "{\"type\":\"user/message\",\"seq\":20,\"time\":20,\"sourceEventSeqs\":[[3,5],7],\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"u\"}]}}\n",
        "{\"type\":\"tool/result\",\"seq\":21,\"time\":21,\"sourceEventSeqs\":[10],\"data\":{\"turn\":1,\"step\":1,\"message\":{\"content\":[{\"type\":\"tool-result\",\"toolCallId\":\"c1\",\"content\":\"out\"}]}}}\n",
        "{\"type\":\"assistant/message\",\"seq\":22,\"time\":22,\"sourceEventSeqs\":[4],\"data\":{\"turn\":1,\"step\":1,\"message\":{\"content\":[{\"type\":\"text\",\"text\":\"b\"}]}}}\n",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 5);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("a")]);
    assert_eq!(projection.turns[2].blocks, vec![Block::text("u")]);
    assert_eq!(
        projection.turns[3].blocks,
        vec![Block::tool_result("out [c1]")]
    );
    assert_eq!(projection.turns[4].blocks, vec![Block::text("b")]);

    for cite in ["null", "[[10, 3]]", "[20]"] {
        let text = format!(
            "{prefix}{{\"type\":\"user/message\",\"seq\":20,\"time\":20,\"sourceEventSeqs\":{cite},\"data\":{{\"content\":[{{\"type\":\"text\",\"text\":\"u\"}}]}}}}\n"
        );
        let projection = dsh(&text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{cite}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{cite}");
        assert!(projection.turns.is_empty(), "{cite}");
    }
}

#[test]
fn dsh_v3_replacement_copy_keeps_the_audit_order() {
    let text = format!(
        "{}{}{}{}{}",
        dsh3_header(Some("false")),
        row(
            json!({"type": "user/message", "seq": 1, "time": 1, "data": {"content": [{"type": "text", "text": "u"}]}})
        ),
        row(
            json!({"type": "assistant/message", "seq": 2, "time": 2, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "text", "text": "a"}]}}})
        ),
        row(
            json!({"type": "system/message", "seq": 3, "time": 3, "surfaceOp": {"op": "replace", "startSeq": 1, "endSeq": 2}, "data": {"content": [{"type": "text", "text": "sys"}]}})
        ),
        row(
            json!({"type": "assistant/message", "seq": 4, "time": 4, "surfaceOp": {"op": "replace", "startSeq": 1, "endSeq": 2}, "data": {"turn": 1, "step": 1, "message": {"content": [{"type": "text", "text": "b"}]}}})
        ),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 3);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("u")]);
    assert_eq!(projection.turns[1].blocks, vec![Block::text("a")]);
    assert_eq!(projection.turns[2].blocks, vec![Block::text("b")]);
}

#[test]
fn dsh_v3_writer_validity_is_not_reader_admission() {
    let prefix = concat!(
        "{\"type\":\"session\",\"version\":3,\"delegationDepth\":0,\"isSeeded\":false}\n",
        "{\"type\":\"user/message\",\"seq\":1,\"time\":1,\"data\":{\"content\":[{\"type\":\"text\",\"text\":\"u\"}]}}\n",
    );
    let scene = |tool_result_cite: &str, assistant_cite: &str| {
        let tool_result = format!(
            "{{\"type\":\"tool/result\",\"seq\":3,\"time\":3,\"sourceEventSeqs\":{tool_result_cite},\"data\":{{\"turn\":1,\"step\":1,\"message\":{{\"content\":[{{\"type\":\"tool-result\",\"toolCallId\":\"c1\",\"content\":\"out\"}}]}}}}}}\n"
        );
        let assistant = format!(
            "{{\"type\":\"assistant/message\",\"seq\":4,\"time\":4,\"sourceEventSeqs\":{assistant_cite},\"data\":{{\"turn\":1,\"step\":1,\"message\":{{\"content\":[{{\"type\":\"text\",\"text\":\"a\"}}]}}}}}}\n"
        );
        format!(
            "{prefix}{}{tool_result}{assistant}{}",
            "{\"type\":\"tool/call\",\"seq\":2,\"time\":2,\"surfaceOp\":\"append\",\"sourceEventSeqs\":null,\"data\":{\"callId\":\"c1\",\"name\":\"f\",\"arguments\":{}}}\n",
            "{\"type\":\"step/end\",\"seq\":5,\"time\":5,\"sourceEventSeqs\":null,\"data\":{}}\n",
        )
    };

    let projection = dsh(&scene("[1]", "[1]"));
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 4);

    let projection = dsh(&scene("[1]", "null"));
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);

    let projection = dsh(&scene("null", "[1]"));
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);
}

fn dsh3_unread_base() -> String {
    format!(
        "{}{}{}{}{}",
        dsh3_header(Some("false")),
        row(
            json!({"type": "user/message", "seq": 1, "time": 1, "data": {"content": [{"type": "text", "text": "u"}]}})
        ),
        row(json!({"type": "todo/write", "seq": 2, "sourceEventSeqs": null, "data": {}})),
        row(
            json!({"type": "deliverables/presented", "seq": 3, "sourceEventSeqs": null, "data": {}})
        ),
        row(
            json!({"type": "assistant/attempt", "seq": 4, "sourceEventSeqs": [[5, 1]], "data": {"stream": [{"type": "text-delta", "text": "x"}]}})
        ),
    )
}

#[test]
fn dsh_v3_unread_cells_change_no_disposition() {
    let projection = dsh(&dsh3_unread_base());
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 1);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("u")]);

    let second = format!(
        "{}{}",
        dsh3_unread_base(),
        row(
            json!({"type": "text-chunks", "seq0": 5, "time0": 5, "sourceEventSeqs": null, "data": {"turn": 1, "step": 1, "index": 0, "dt": [], "texts": ["t"]}})
        ),
    );
    let projection = dsh(&second);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 2);
    assert!(projection.turns.is_empty());

    let third = format!(
        "{}{}",
        dsh3_unread_base(),
        row(
            json!({"type": "assistant/chunk", "seq": 5, "ignorable": true, "sourceEventSeqs": null, "data": {"turn": 1, "step": 1, "chunk": {"type": "text-delta", "text": "c"}}})
        ),
    );
    let projection = dsh(&third);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 2);
    assert_eq!(projection.turns.len(), 1);

    let fourth = format!(
        "{}{}{}{}{}",
        dsh3_header(Some("false")),
        row(
            json!({"type": "user/message", "seq": 1, "time": 1, "sourceEventSeqs": null, "data": {"content": [{"type": "text", "text": "u"}]}})
        ),
        row(json!({"type": "todo/write", "seq": 2, "sourceEventSeqs": null, "data": {}})),
        row(
            json!({"type": "deliverables/presented", "seq": 3, "sourceEventSeqs": null, "data": {}})
        ),
        row(
            json!({"type": "assistant/attempt", "seq": 4, "sourceEventSeqs": [[5, 1]], "data": {"stream": [{"type": "text-delta", "text": "x"}]}})
        ),
    );
    let projection = dsh(&fourth);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 2);
    assert!(projection.turns.is_empty());
}

#[test]
fn dsh_v3_interrupted_message_projects_its_finalized_text() {
    let text = format!(
        "{}{}{}",
        dsh3_header(Some("false")),
        row(json!({"type": "assistant/message", "seq": 1, "time": 1000, "data": {"turn": 1, "step": 1, "interrupted": true, "stream": [{"type": "text-delta", "text": "prefix"}], "usage": {"input": 1}, "message": {"content": [{"type": "text", "text": "prefix"}]}}})),
        "{\"type\":\"assistant/message\",\"seq\":2,\"time\":1001,\"data\":{\"turn\":1,\"step\":1,\"message\":{\"content\":[{\"type\":\"te",
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.skipped_lines, 0);
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("prefix")]);
    assert_eq!(projection.turns[0].ts, "1000");
}

#[test]
fn dsh_v3_partial_support_counts_unsupported_blocks_once() {
    for header in [
        row(json!({"type": "session", "version": 0})),
        dsh3_header(Some("false")),
    ] {
        let text = format!(
            "{header}{}",
            row(
                json!({"type": "assistant/message", "seq": 1, "time": 1, "data": {"turn": 1, "step": 1, "message": {"content": [
                    {"type": "text", "text": "kept"},
                    {"type": "file", "name": "f"},
                    {"type": "future-block"}
                ]}}})
            ),
        );
        let projection = dsh(&text);
        assert!(projection.unavailable.is_none(), "{header}");
        assert_eq!(projection.unrecognized_records, 1, "{header}");
        assert_eq!(projection.turns.len(), 1, "{header}");
        assert_eq!(projection.turns[0].blocks, vec![Block::text("kept")]);
    }
}

#[test]
fn dsh_v3_time_keeps_the_recorded_token_exactness() {
    let text = concat!(
        "{\"type\":\"session\",\"version\":3e0}\n",
        "{\"type\":\"tool/call\",\"seq\":1,\"time\":1e-400,\"data\":{\"callId\":\"c1\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":2,\"time\":1e3,\"data\":{\"callId\":\"c2\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":3,\"time\":1000.0,\"data\":{\"callId\":\"c3\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":4,\"time\":-0.0,\"data\":{\"callId\":\"c4\",\"name\":\"f\",\"arguments\":{}}}\n",
    );
    let projection = dsh(text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 4);
    let stamps: Vec<&str> = projection
        .turns
        .iter()
        .map(|turn| turn.ts.as_str())
        .collect();
    assert_eq!(stamps, vec!["", "1000", "1000", "0"]);
}

#[test]
fn dsh_zero_digit_underflow_and_duplicate_members_are_not_zero() {
    let three = concat!(
        "{\"type\":\"session\",\"version\":3}\n",
        "{\"type\":\"tool/call\",\"seq\":1,\"time\":10e-400,\"data\":{\"callId\":\"c1\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":2,\"time\":0.1e-400,\"data\":{\"callId\":\"c2\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":3,\"time\":1.0e-400,\"data\":{\"callId\":\"c3\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":4,\"time\":1000,\"time\":2000,\"data\":{\"callId\":\"c4\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":5,\"\\u0074ime\":1e-400,\"data\":{\"callId\":\"c5\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":6,\"time\":0,\"data\":{\"callId\":\"c6\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":7,\"time\":0.0,\"data\":{\"callId\":\"c7\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":8,\"time\":-0,\"data\":{\"callId\":\"c8\",\"name\":\"f\",\"arguments\":{}}}\n",
        "{\"type\":\"tool/call\",\"seq\":9,\"time\":0e0,\"data\":{\"callId\":\"c9\",\"name\":\"f\",\"arguments\":{}}}\n",
    );
    let projection = dsh(three);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 9);
    let stamps: Vec<&str> = projection
        .turns
        .iter()
        .map(|turn| turn.ts.as_str())
        .collect();
    assert_eq!(stamps, vec!["", "", "", "", "", "0", "0", "0", "0"]);

    let zero = concat!(
        "{\"type\":\"session\",\"version\":0}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":1,\"time\":10e-400,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"a\"}}}\n",
        "{\"type\":\"assistant/chunk\",\"seq\":2,\"time\":1000,\"time\":2000,\"data\":{\"turn\":1,\"step\":1,\"chunk\":{\"type\":\"text-delta\",\"text\":\"b\"}}}\n",
    );
    let projection = dsh(zero);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].ts, "");
    assert_eq!(projection.turns[1].ts, "");

    for body in [
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":10e-400,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[],\"texts\":[\"a\"]}}\n",
        "{\"type\":\"text-chunks\",\"seq0\":10,\"time0\":1000,\"time0\":2000,\"data\":{\"turn\":1,\"step\":1,\"index\":0,\"dt\":[],\"texts\":[\"a\"]}}\n",
    ] {
        let text = format!("{}{body}", row(json!({"type": "session", "version": 0})));
        let projection = dsh(&text);
        assert_eq!(
            projection.unavailable,
            Some(Unavailable::UnsupportedFormat),
            "{body}"
        );
        assert_eq!(projection.unrecognized_records, 1, "{body}");
    }
}

// ------------------------------------------ #277 bounded projector (#277)

/// The merge ruling's regression witness: a many-member packed run
/// constructs exactly one candidate payload, never one per token, even
/// transiently. The observation counters record actual construction, so an
/// expand-then-coalesce mutation that restores per-member construction
/// still fails this assertion.
#[test]
fn packed_coalescing_constructs_one_candidate_not_one_per_token() {
    observe::reset();
    let gaps = vec![1i64; 999];
    let members = vec!["a"; 1000];
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &gaps, &members),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(
        projection.turns[0].blocks,
        vec![Block::text("a".repeat(1000))]
    );
    assert_eq!(
        observe::packed_candidates(),
        1,
        "one coalesced candidate, not one per member"
    );
    assert_eq!(observe::peak_candidates(), 1);
}

/// The structural charge bounds tiny ordinary rows during projection:
/// 10,000 absent-data `tool/call` rows retain exactly 7,797 one-byte turns,
/// and the retained prefix never exceeds the charged count.
#[test]
fn tiny_ordinary_calls_stay_within_the_structural_budget() {
    observe::reset();
    let mut text = row(json!({"type":"session","version":0}));
    for seq in 1..=10_000 {
        text.push_str(&row(
            json!({"type":"tool/call","seq":seq,"time":1000,"data":{}}),
        ));
    }
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert!(projection.truncated);
    assert_eq!(projection.unrecognized_records, 0);
    assert_eq!(projection.turns.len(), 7_797);
    assert_eq!(
        projection
            .turns
            .iter()
            .map(|turn| turn.blocks[0].text.len())
            .sum::<usize>(),
        7_797
    );
    assert!(
        observe::peak_retained() <= 7_812,
        "retained slots stay within floor(cap / charge)"
    );
}

/// Exactly 7,797 tiny calls fit with no truncation, and the count is one
/// over the charged bound rather than an ordinary-row turn count.
#[test]
fn seventy_seven_ninety_seven_tiny_calls_fit_exactly() {
    let mut text = row(json!({"type":"session","version":0}));
    for seq in 1..=7_797 {
        text.push_str(&row(
            json!({"type":"tool/call","seq":seq,"time":1000,"data":{}}),
        ));
    }
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert!(!projection.truncated);
    assert_eq!(projection.turns.len(), 7_797);
}

/// A required refusal after the display budget is exhausted still returns
/// the complete-prefix diagnostics and no turns: an early display stop
/// never hides a semantic refusal.
#[test]
fn complete_prefix_refusal_survives_the_display_cap() {
    let mut text = row(json!({"type":"session","version":0}));
    for seq in 1..=10_000 {
        text.push_str(&row(
            json!({"type":"tool/call","seq":seq,"time":1000,"data":{}}),
        ));
    }
    text.push_str(&row(json!({"type":"future/required"})));
    let projection = dsh(&text);
    assert_eq!(projection.unavailable, Some(Unavailable::UnsupportedFormat));
    assert_eq!(projection.unrecognized_records, 1);
    assert!(!projection.truncated, "only source truncation would set it");
    assert!(projection.turns.is_empty());
}

/// The shared charge helper is the single rule for every kind: an empty
/// block list supplies no turn, and the structural constant is charged
/// beside the emitted text bytes.
#[test]
fn display_cap_charges_the_constant_and_drops_empty_turns() {
    let mut truncated = false;
    let kept = display_cap(
        vec![
            Turn {
                role: "empty".to_string(),
                ts: String::new(),
                blocks: Vec::new(),
            },
            Turn {
                role: "one".to_string(),
                ts: String::new(),
                blocks: vec![Block::text("x")],
            },
            Turn {
                role: "over".to_string(),
                ts: String::new(),
                blocks: vec![Block::text("x".repeat(DISPLAY_CAP))],
            },
        ],
        &mut truncated,
    );
    assert!(truncated);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].role, "one");
    assert_eq!(display_cost(&[]), None);
    assert_eq!(
        display_cost(&[Block::text("x")]),
        Some(DISPLAY_EVENT_COST + 1)
    );
}

/// The uniqueness index collapses overlapping spans to `multiple` while
/// keeping singly covered values, and its endpoints do not wrap at the
/// signed-integer boundary.
#[test]
fn unique_segments_classify_overlap_and_boundaries() {
    let unique = UniqueSegments::new(&[(10, 12), (11, 11), (20, 20), (i64::MAX, i64::MAX)]);
    assert!(unique.is_unique(10));
    assert!(!unique.is_unique(11), "two spans cover 11");
    assert!(unique.is_unique(12));
    assert!(!unique.is_unique(13));
    assert!(unique.is_unique(20));
    assert!(unique.is_unique(i64::MAX));
    assert!(!unique.is_unique(i64::MAX - 1));
    assert!(!unique.is_unique(i64::MIN));
    let negative = UniqueSegments::new(&[(i64::MIN, i64::MIN)]);
    assert!(negative.is_unique(i64::MIN));
}

/// The collector seals at the first overflow and refuses every later
/// candidate, and an empty block list never supplies a turn. This exercises
/// the defensive branches directly with a bounded collector.
#[test]
fn the_collector_seals_and_discards_every_later_candidate() {
    let mut collector = DshCollector::new(false);
    collector.admit(
        "first".to_string(),
        String::new(),
        vec![Block::text("x".repeat(DISPLAY_CAP))],
    );
    assert!(collector.sealed);
    assert!(collector.turns.is_empty());
    collector.admit("later".to_string(), String::new(), vec![Block::text("y")]);
    assert!(collector.turns.is_empty());
    flush_run(&mut collector, BlockKind::Text, &[json!("z")], 0, 1, 1, 0);
    assert!(collector.turns.is_empty());

    let mut fresh = DshCollector::new(false);
    fresh.admit("empty".to_string(), String::new(), Vec::new());
    assert!(fresh.turns.is_empty() && !fresh.sealed);
}

/// Citation coverage keeps the greatest citing row ordinal per `(turn,
/// step)` region, so a later assembly can suppress what an earlier one
/// already covered without expanding any interval.
#[test]
fn citation_coverage_keeps_the_latest_citing_ordinal() {
    let assemblies = vec![
        AssemblyFact {
            ordinal: 1,
            turn: Position::Int(1),
            step: Position::Int(1),
            cited: vec![(10, 20)],
        },
        AssemblyFact {
            ordinal: 5,
            turn: Position::Int(1),
            step: Position::Int(1),
            cited: vec![(15, 25)],
        },
        AssemblyFact {
            ordinal: 9,
            turn: Position::Int(1),
            step: Position::Int(2),
            cited: vec![(10, 10)],
        },
    ];
    let coverage = CitationCoverage::new(&assemblies);
    assert_eq!(
        coverage.latest_ordinal(Position::Int(1), Position::Int(1), 10),
        Some(1)
    );
    assert_eq!(
        coverage.latest_ordinal(Position::Int(1), Position::Int(1), 15),
        Some(5)
    );
    assert_eq!(
        coverage.latest_ordinal(Position::Int(1), Position::Int(1), 21),
        Some(5)
    );
    assert_eq!(
        coverage.latest_ordinal(Position::Int(1), Position::Int(1), 26),
        None
    );
    assert_eq!(
        coverage.latest_ordinal(Position::Int(1), Position::Int(2), 10),
        Some(9)
    );
    assert_eq!(
        coverage.latest_ordinal(Position::Int(2), Position::Int(1), 10),
        None
    );
}

/// Empty members preserve identity and time without splitting a run: an
/// empty first member contributes its sequence but not the first stamp.
#[test]
fn coalescing_preserves_empty_member_identity_and_the_first_visible_stamp() {
    let text = format!(
        "{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[1, 1, 1], &["", "a", "", "b"]),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.turns.len(), 1);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("ab")]);
    assert_eq!(projection.turns[0].ts, "1001");
}

/// A blockless row that duplicates a packed member's sequence keeps a
/// citation to it ambiguous, so the member is not suppressed; the blockless
/// row itself retains no turn.
#[test]
fn a_blockless_duplicate_sequence_keeps_a_packed_citation_ambiguous() {
    let text = format!(
        "{}{}{}{}",
        row(json!({"type":"session","version":0})),
        packed_text_chunks(10, 1000, 1, 1, &[1], &["a", "b"]),
        row(json!({"type":"user/message","seq":11,"data":{}})),
        dsh_assembly(20, 1020, 1, 1, Some(json!([11]))),
    );
    let projection = dsh(&text);
    assert!(projection.unavailable.is_none(), "{projection:?}");
    assert_eq!(projection.turns.len(), 2);
    assert_eq!(projection.turns[0].blocks, vec![Block::text("ab")]);
    assert_eq!(projection.turns[0].ts, "1000");
    assert_eq!(projection.turns[1].blocks, vec![Block::text("assembled")]);
}

// ------------------------------------ D8's process-memory measurement seam

/// D8's fresh-process peak-RSS proxy. Ignored so ordinary suites skip it:
/// the release-built test executable is run directly under `/usr/bin/time
/// -v`. `BROKKR_MEASURE_FILE` names a pre-generated fixture; this process
/// reads it, projects it once, prints the retained shape and does no
/// generation, network or TUI work. The counters are the returned result,
/// not an allocator report.
#[test]
#[ignore]
fn measure_projection_peak() {
    let path = std::env::var("BROKKR_MEASURE_FILE").expect("BROKKR_MEASURE_FILE");
    let bytes = std::fs::read(&path).expect("fixture");
    let kind = match std::env::var("BROKKR_MEASURE_KIND").as_deref() {
        Ok("claude") => TranscriptKind::ClaudeSession,
        Ok("codex") => TranscriptKind::CodexThread,
        _ => TranscriptKind::DshSession,
    };
    let projection = project(
        kind,
        &Snapshot {
            bytes: &bytes,
            overflow: false,
            eof: true,
        },
    );
    let blocks: usize = projection.turns.iter().map(|turn| turn.blocks.len()).sum();
    let text: usize = projection
        .turns
        .iter()
        .flat_map(|turn| &turn.blocks)
        .map(|block| block.text.len())
        .sum();
    let charged = 512usize * projection.turns.len() + text;
    println!(
        "MEASURE bytes={} turns={} blocks={} text={} charged={} truncated={} unavailable={:?}",
        bytes.len(),
        projection.turns.len(),
        blocks,
        text,
        charged,
        projection.truncated,
        projection.unavailable
    );
}

/// The parse-only control: read the fixture and decode every row as JSON,
/// counting members, without invoking the projector.
#[test]
#[ignore]
fn measure_parse_only() {
    let path = std::env::var("BROKKR_MEASURE_FILE").expect("BROKKR_MEASURE_FILE");
    let bytes = std::fs::read(&path).expect("fixture");
    let text = std::str::from_utf8(&bytes).expect("utf8");
    let mut rows = 0usize;
    let mut members = 0usize;
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("row");
        if value.get("type").and_then(serde_json::Value::as_str) == Some("text-chunks") {
            members += value["data"]["texts"].as_array().map(Vec::len).unwrap_or(0);
        }
        rows += 1;
    }
    println!("PARSE bytes={} rows={rows} members={members}", bytes.len());
}

/// The input-only control: read the fixture and drop it, measuring the
/// resident source bytes without parsing or projecting.
#[test]
#[ignore]
fn measure_input_only() {
    let path = std::env::var("BROKKR_MEASURE_FILE").expect("BROKKR_MEASURE_FILE");
    let bytes = std::fs::read(&path).expect("fixture");
    println!("INPUT bytes={}", bytes.len());
    std::hint::black_box(bytes);
}
