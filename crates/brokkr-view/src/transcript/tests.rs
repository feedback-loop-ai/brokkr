use super::*;

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
