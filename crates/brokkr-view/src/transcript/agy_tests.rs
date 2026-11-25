//! Unit tests for `agy-conversation` reference validation, projection rules,
//! diagnostics, and display caps (#1227, decision 0075 level 2).
//!
//! Compile-time bindings against the six `docs/evidence/adapters/agy-cli-1-5/`
//! transcripts live in `tests/agy_transcript.rs`.

use super::agy::UNFINISHED_NOTICE;
use super::*;

fn project_agy(text: &str) -> Projection {
    project(
        TranscriptKind::AgyConversation,
        &Snapshot {
            bytes: text.as_bytes(),
            overflow: false,
            eof: true,
        },
    )
}

#[test]
fn agy_conversation_kind_round_trips_and_validates_lowercase_uuid_locators() {
    assert_eq!(
        TranscriptKind::parse("agy-conversation"),
        Some(TranscriptKind::AgyConversation)
    );
    assert_eq!(TranscriptKind::AgyConversation.as_str(), "agy-conversation");

    let valid = Transcript {
        kind: "agy-conversation".into(),
        locator: "32786eb8-d649-4c35-9110-c8c87a1cdb7d".into(),
        home: "/home/op/.local/state/brokkr/agy-conversations".into(),
    };
    let checked = validate_common(&valid).expect("valid lowercase UUID");
    assert_eq!(checked.kind, TranscriptKind::AgyConversation);
    assert_eq!(
        full_session(
            &checked,
            Some("/home/op/.local/state/brokkr/agy-conversations/32786eb8-d649-4c35-9110-c8c87a1cdb7d/transcript.jsonl")
        ),
        Some(
            "full session: path \"/home/op/.local/state/brokkr/agy-conversations/32786eb8-d649-4c35-9110-c8c87a1cdb7d/transcript.jsonl\""
                .into()
        )
    );
    assert_eq!(full_session(&checked, None), None);

    for bad in [
        "32786EB8-d649-4c35-9110-c8c87a1cdb7d",
        "../32786eb8-d649-4c35-9110-c8c87a1cdb7d",
        "32786eb8-d649-4c35-9110-c8c87a1cdb7",
        "32786eb8-d649-4c35-9110-c8c87a1cdb7de",
        "32786eb8_d649_4c35_9110_c8c87a1cdb7d",
        "zzzzzzzz-d649-4c35-9110-c8c87a1cdb7d",
    ] {
        let mut r = valid.clone();
        r.locator = bad.into();
        assert_eq!(
            validate_common(&r),
            Err(Unavailable::InvalidReference),
            "rejected {bad}"
        );
    }
}

#[test]
fn planner_thinking_and_diagnostics_and_caps_follow_shared_rules() {
    let raw = concat!(
        "not json at all\n",
        "123\n",
        "{\"source\":\"MODEL\",\"type\":\"UNKNOWN_KIND\",\"status\":\"DONE\",\"content\":\"x\"}\n",
        "{\"source\":\"UNKNOWN_SRC\",\"type\":\"USER_INPUT\",\"status\":\"DONE\",\"content\":\"x\"}\n",
        "{\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"DONE\",\"created_at\":\"2026-11-24T00:00:00Z\",\"thinking\":\"plan first\",\"content\":\"answer\",\"tool_calls\":[{\"name\":\"ls\",\"args\":{\"path\":\".\"}}]}\n",
        "{\"source\":\"MODEL\",\"type\":\"GENERIC\",\"status\":\"RUNNING\",\"created_at\":\"2026-11-24T00:00:01Z\"}\n",
    );
    let p = project_agy(raw);
    assert_eq!(p.unavailable, None);
    assert_eq!(p.skipped_lines, 1);
    assert_eq!(p.unrecognized_records, 3);
    assert_eq!(p.turns.len(), 2);

    assert_eq!(p.turns[0].role, "assistant");
    assert_eq!(p.turns[0].ts, "2026-11-24T00:00:00Z");
    assert_eq!(
        p.turns[0].blocks,
        vec![
            Block::reasoning("plan first"),
            Block::text("answer"),
            Block::tool("ls path=."),
        ]
    );

    // RUNNING with no content still shows unfinished in words.
    assert_eq!(p.turns[1].role, "tool");
    assert_eq!(p.turns[1].blocks, vec![Block::text(UNFINISHED_NOTICE)]);

    // Display cap seals cleanly as for every other kind.
    let huge = format!(
        "{{\"source\":\"USER_EXPLICIT\",\"type\":\"USER_INPUT\",\"status\":\"DONE\",\"content\":\"{}\"}}\n",
        "a".repeat(DISPLAY_CAP + 1)
    );
    let capped = project_agy(&huge);
    assert!(capped.truncated);
    assert!(capped.turns.is_empty());
}

/// #1227 W1: `tool_calls[].args` members render verbatim as the row holds them
/// after the row's own one JSON parse — never re-parsed and never
/// re-serialized — so string values preserve key order, whitespace, and
/// single-layer JSON escaping, while non-string values render as compact JSON.
#[test]
fn tool_call_args_render_verbatim_without_reparsing_or_reserializing() {
    let secret = "tok\"en\\7f3a9c";
    let cmd_once = serde_json::to_string(&format!("echo {secret}")).unwrap();
    let mcp_inner = serde_json::json!({"command": format!("cat {secret}")}).to_string();
    let json_secret = r#"{"z": 1, "secret": "tok\"en\\7f3a9c"}"#;
    let json_cmd = serde_json::to_string(json_secret).unwrap();

    let row = serde_json::json!({
        "source": "MODEL",
        "type": "PLANNER_RESPONSE",
        "status": "DONE",
        "created_at": "2026-11-24T03:41:19Z",
        "tool_calls": [
            {
                "name": "run_command",
                "args": {
                    "CommandLine": cmd_once,
                    "WaitMsBeforeAsync": "500",
                    "IsDaemon": false,
                    "Extra": [1, 2]
                }
            },
            {
                "name": "call_mcp_tool",
                "args": {
                    "Arguments": mcp_inner,
                    "ServerName": "\"brokkr\"",
                    "ToolName": "\"workspace\""
                }
            },
            {
                "name": "run_command",
                "args": {
                    "CommandLine": json_cmd
                }
            }
        ]
    });
    let p = project_agy(&format!("{row}\n"));
    assert_eq!(p.turns.len(), 1);
    assert_eq!(p.turns[0].ts, "2026-11-24T03:41:19Z");
    assert!(p.notices.is_empty(), "{:?}", p.notices);
    assert_eq!(p.turns[0].blocks.len(), 3);

    // String values stand verbatim (escaped once inside agy's JSON string);
    // non-string values render as compact JSON.
    let single_escaped = "tok\\\"en\\\\7f3a9c";
    assert_eq!(
        p.turns[0].blocks[0].text,
        format!(
            "run_command CommandLine=\"echo {single_escaped}\", Extra=[1,2], IsDaemon=false, WaitMsBeforeAsync=500"
        )
    );
    assert_eq!(
        p.turns[0].blocks[1].text,
        format!(
            "call_mcp_tool Arguments={{\"command\":\"cat {single_escaped}\"}}, ServerName=\"brokkr\", ToolName=\"workspace\""
        )
    );
    // A JSON-text value preserves its exact key order (`z` before `secret`)
    // and spaces without being re-parsed or re-serialized.
    assert_eq!(
        p.turns[0].blocks[2].text,
        format!("run_command CommandLine={json_cmd}")
    );
}

/// #1227 W2: `created_at` admits only exact 20-byte `YYYY-MM-DDTHH:MM:SSZ`
/// timestamps with valid date/clock; any other value (secret vector,
/// fractional seconds, invalid calendar date, non-string, or 5 MiB string)
/// yields an empty timestamp, leaves the row shown (`skipped_lines == 0`),
/// does not retain oversized bytes, and is counted in a notice.
#[test]
fn created_at_admits_only_utc_seconds_and_drops_vector_and_five_mib() {
    use super::agy::invalid_timestamp_notice;

    let big = "a".repeat(5 * 1024 * 1024);
    let rows = [
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": "2024-02-29T23:59:59Z", "content": "leap-ok"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": "tok\"en\\7f3a9c", "content": "vector"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": "2026-11-22T19:15:34.123Z", "content": "fractional"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": "2026-02-29T00:00:00Z", "content": "bad-leap"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": 1732302934, "content": "numeric"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": big, "content": "five-mib"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "created_at": null, "content": "null-ts"
        }),
        serde_json::json!({
            "source": "USER_EXPLICIT", "type": "USER_INPUT", "status": "DONE",
            "content": "absent-ok"
        }),
    ];
    let text: String = rows.iter().map(|r| format!("{r}\n")).collect();
    let p = project_agy(&text);

    assert_eq!(p.skipped_lines, 0);
    assert_eq!(p.unrecognized_records, 0);
    assert_eq!(p.turns.len(), 8);
    assert_eq!(p.turns[0].ts, "2024-02-29T23:59:59Z");
    for (i, label) in [
        (1, "vector"),
        (2, "fractional"),
        (3, "bad-leap"),
        (4, "numeric"),
        (5, "five-mib"),
        (6, "null-ts"),
        (7, "absent-ok"),
    ] {
        assert_eq!(p.turns[i].ts, "", "{label}");
        assert_eq!(p.turns[i].blocks, vec![Block::text(label)]);
    }
    assert_eq!(p.notices, vec![invalid_timestamp_notice(6)]);
    // The 5 MiB created_at was dropped rather than retained in Turn.ts.
    let total_ts_bytes: usize = p.turns.iter().map(|t| t.ts.len()).sum();
    assert_eq!(total_ts_bytes, 20);
}

/// #1227 U7: a kept `agy-conversation` file is complete after `agy` exits, so a
/// malformed final line without a trailing newline counts as skipped. A valid
/// newline-less final line still projects, an overflow-cut tail stays uncounted
/// as malformed (reported as truncated), and live kinds (`claude-session`,
/// `codex-thread`, `dsh-session`) keep treating a final fragment as provisional.
#[test]
fn agy_counts_malformed_newline_less_final_line_while_live_kinds_stay_provisional() {
    let valid_line =
        r#"{"source":"USER_EXPLICIT","type":"USER_INPUT","status":"DONE","content":"hi"}"#;
    let malformed_tail = format!("{valid_line}\n{{\"source\":\"MODEL\"");
    let p = project_agy(&malformed_tail);
    assert_eq!(p.turns.len(), 1);
    assert_eq!(p.skipped_lines, 1);

    // Valid newline-less final line is still projected with 0 skipped.
    let valid_no_nl = project_agy(valid_line);
    assert_eq!(valid_no_nl.turns.len(), 1);
    assert_eq!(valid_no_nl.skipped_lines, 0);

    // Overflow-cut tail is truncated, not counted as malformed.
    let overflowed = project(
        TranscriptKind::AgyConversation,
        &Snapshot {
            bytes: malformed_tail.as_bytes(),
            overflow: true,
            eof: false,
        },
    );
    assert!(overflowed.truncated);
    assert_eq!(overflowed.skipped_lines, 0);

    // Live kinds remain provisional: malformed newline-less final fragment is uncounted.
    for (kind, sample) in [
        (
            TranscriptKind::ClaudeSession,
            "{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n{broken",
        ),
        (
            TranscriptKind::CodexThread,
            "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":\"hi\"}}\n{broken",
        ),
        (
            TranscriptKind::DshSession,
            "{\"type\":\"session\",\"version\":0}\n{broken",
        ),
    ] {
        let live = project(
            kind,
            &Snapshot {
                bytes: sample.as_bytes(),
                overflow: false,
                eof: true,
            },
        );
        assert_eq!(live.skipped_lines, 0, "{kind:?} stays provisional");
    }
}

/// #1227 H5: every timestamp separator, non-digit position, month/day bound,
/// leap-year rule, clock bound, non-standard status, ill-typed row, and
/// `tool_calls[].args` variant exercises its branch.
#[test]
fn agy_timestamp_bounds_row_shapes_and_tool_arg_variants_bind() {
    use super::agy::invalid_timestamp_notice;

    let valid_ts = [
        "2026-01-31T00:00:00Z",
        "2026-04-30T12:34:56Z",
        "2026-02-28T23:59:59Z",
        "2024-02-29T00:00:00Z",
        "2000-02-29T23:59:60Z",
    ];
    let invalid_ts = [
        "2026/11-24T00:00:00Z",
        "2026-11/24T00:00:00Z",
        "2026-11-24 00:00:00Z",
        "2026-11-24T00-00:00Z",
        "2026-11-24T00:00-00Z",
        "2026-11-24T00:00:00+",
        "202a-11-24T00:00:00Z",
        "2026-1a-24T00:00:00Z",
        "2026-11-2aT00:00:00Z",
        "2026-11-24T0a:00:00Z",
        "2026-11-24T00:0a:00Z",
        "2026-11-24T00:00:0aZ",
        "2026-00-15T00:00:00Z",
        "2026-13-15T00:00:00Z",
        "2026-01-00T00:00:00Z",
        "2026-01-32T00:00:00Z",
        "2026-04-31T00:00:00Z",
        "2024-02-30T00:00:00Z",
        "1900-02-29T00:00:00Z",
        "2026-02-29T00:00:00Z",
        "2026-01-15T24:00:00Z",
        "2026-01-15T23:60:00Z",
        "2026-01-15T23:59:61Z",
    ];
    let mut lines = String::new();
    for ts in valid_ts.iter().chain(invalid_ts.iter()) {
        lines.push_str(&format!(
            "{{\"source\":\"USER_EXPLICIT\",\"type\":\"USER_INPUT\",\"status\":\"DONE\",\"created_at\":\"{ts}\",\"content\":\"{ts}\"}}\n"
        ));
    }
    let p = project_agy(&lines);
    assert_eq!(p.skipped_lines, 0);
    assert_eq!(p.unrecognized_records, 0);
    assert_eq!(p.turns.len(), valid_ts.len() + invalid_ts.len());
    for (idx, ts) in valid_ts.iter().enumerate() {
        assert_eq!(p.turns[idx].ts, *ts, "valid {ts}");
    }
    for (offset, ts) in invalid_ts.iter().enumerate() {
        assert_eq!(p.turns[valid_ts.len() + offset].ts, "", "invalid {ts}");
    }
    assert_eq!(
        p.notices,
        vec![invalid_timestamp_notice(invalid_ts.len() as u64)]
    );

    let shapes = concat!(
        "{\"source\":123,\"type\":\"USER_INPUT\"}\n",
        "{\"source\":\"MODEL\",\"type\":\"GENERIC\",\"status\":\"CANCELLED\",\"content\":\"stopped\"}\n",
        "{\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"DONE\",\"tool_calls\":[",
        "{\"name\":\"\",\"args\":{}},",
        "{\"name\":\"no_args\"},",
        "{\"name\":\"null_args\",\"args\":null},",
        "{\"name\":\"empty_obj\",\"args\":{}},",
        "{\"name\":\"str_args\",\"args\":\"raw\"},",
        "{\"name\":\"arr_args\",\"args\":[1,2]}",
        "]}\n",
    );
    let q = project_agy(shapes);
    assert_eq!(q.skipped_lines, 0);
    assert_eq!(q.unrecognized_records, 1);
    assert_eq!(q.turns.len(), 2);
    assert_eq!(q.turns[0].role, "tool");
    assert_eq!(q.turns[0].blocks, vec![Block::tool_result("stopped")]);
    assert_eq!(q.turns[1].role, "assistant");
    assert_eq!(
        q.turns[1].blocks,
        vec![
            Block::tool("no_args"),
            Block::tool("null_args"),
            Block::tool("empty_obj"),
            Block::tool("str_args raw"),
            Block::tool("arr_args [1,2]"),
        ]
    );
}

/// #1227 N1: a `MODEL`/`GENERIC` row with `status: "ERROR"` falls back to
/// non-empty `content` when `error` is absent or empty, and emits no block
/// when both `error` and `content` are empty.
#[test]
fn generic_error_falls_back_to_content_and_omits_when_both_empty() {
    let raw = concat!(
        "{\"source\":\"MODEL\",\"type\":\"GENERIC\",\"status\":\"ERROR\",\"content\":\"fallback without error\"}\n",
        "{\"source\":\"MODEL\",\"type\":\"GENERIC\",\"status\":\"ERROR\",\"error\":\"\",\"content\":\"fallback after empty error\"}\n",
        "{\"source\":\"MODEL\",\"type\":\"GENERIC\",\"status\":\"ERROR\",\"error\":\"\",\"content\":\"\"}\n",
    );
    let p = project_agy(raw);
    assert_eq!(p.unavailable, None);
    assert_eq!(p.skipped_lines, 0);
    assert_eq!(p.unrecognized_records, 0);
    assert_eq!(p.turns.len(), 2);
    assert_eq!(p.turns[0].role, "tool");
    assert_eq!(
        p.turns[0].blocks,
        vec![Block::tool_result("fallback without error")]
    );
    assert_eq!(p.turns[1].role, "tool");
    assert_eq!(
        p.turns[1].blocks,
        vec![Block::tool_result("fallback after empty error")]
    );
}
