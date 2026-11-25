//! Compile-time evidence bindings for `agy-conversation` (#1227, decision 0075
//! level 2).
//!
//! These tests live under `tests/` rather than `src/` so `include_str!` binds
//! the controller's six `docs/evidence/adapters/agy-cli-1-5/*.transcript.ndjson`
//! files at compile time without any `src/` file including bytes outside the
//! published crate (`packaging::no_crate_includes_a_file_from_outside_itself`).

use brokkr_view::transcript::{project, Block, BlockKind, Projection, Snapshot, TranscriptKind};

const L2_BOX: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2-box.transcript.ndjson");
const L2_KILLED: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2-killed.transcript.ndjson");
const L2_NEON_EFFORT: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2-neon-effort.transcript.ndjson");
const L2_REJOINS: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2-rejoins.transcript.ndjson");
const L2B_PRINTED_VALUE: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2b-printed-value.transcript.ndjson");
const L2B_TWO_REJOINS: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2b-two-rejoins.transcript.ndjson");

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
fn evidence_l2_box_shows_denied_native_tool_and_workspace_calls() {
    let p = project_agy(L2_BOX);
    assert_eq!(p.unavailable, None);
    assert_eq!(p.skipped_lines, 0);
    assert_eq!(p.unrecognized_records, 0);
    assert!(!p.truncated);
    assert_eq!(p.turns.len(), 12);

    // User input keeps agy's wrapper tags verbatim.
    assert_eq!(p.turns[0].role, "user");
    assert!(p.turns[0].blocks[0].text.contains("<USER_REQUEST>"));
    assert!(p.turns[0].blocks[0].text.contains("KIWI-2093"));

    // Denied native read_file call + ERROR tool result using `error`.
    assert_eq!(p.turns[1].role, "assistant");
    assert_eq!(p.turns[1].blocks[0].kind, BlockKind::Tool);
    assert_eq!(
        p.turns[1].blocks[0].text,
        r#"read_file Path="/var/tmp/probe-agy/home-L2box/.gemini/antigravity-cli/mcp/brokkr/workspace.json""#
    );
    assert_eq!(p.turns[2].role, "tool");
    assert_eq!(
        p.turns[2].blocks,
        vec![Block::tool_result(
            "tool call denied by pre-tool hook: the engine's gate does not offer the tool read_file"
        )]
    );

    // Boxed workspace calls (rows 3 and 9) render verbatim args as the row holds them.
    assert_eq!(p.turns[3].role, "assistant");
    assert_eq!(p.turns[3].blocks[0].kind, BlockKind::Tool);
    assert_eq!(
        p.turns[3].blocks[0].text,
        r#"call_mcp_tool Arguments={"command":"cat seed.txt"}, ServerName="brokkr", ToolName="workspace""#
    );
    assert_eq!(
        p.turns[9].blocks[0].text,
        r#"call_mcp_tool Arguments={"command":"echo again > again.txt && cat again.txt"}, ServerName="brokkr", ToolName="workspace""#
    );
    assert_eq!(p.turns[4].role, "tool");
    assert!(p.turns[4].blocks[0].text.contains("seed-l2"));
    assert_eq!(p.turns[5].role, "system");
    assert!(p.turns[5].blocks[0].text.contains("<SYSTEM_MESSAGE>"));
    assert_eq!(p.turns[11].role, "assistant");
    assert_eq!(p.turns[11].blocks, vec![Block::text("KIWI-2093 again")]);
}

#[test]
fn evidence_l2_killed_shows_running_row_as_unfinished_and_not_an_error() {
    let p = project_agy(L2_KILLED);
    assert_eq!(p.unavailable, None);
    assert_eq!(p.skipped_lines, 0);
    assert_eq!(p.unrecognized_records, 0);
    assert_eq!(p.turns.len(), 7);

    // Row 1 is run_command: verbatim args render as the row holds them.
    assert_eq!(
        p.turns[1].blocks[0].text,
        r#"run_command CommandLine="sleep 90", WaitMsBeforeAsync=500, toolSummary="Run sleep command""#
    );

    // Row 2 is status=RUNNING: tool result content kept AND unfinished shown in words.
    assert_eq!(p.turns[2].role, "tool");
    assert_eq!(p.turns[2].blocks.len(), 2);
    assert_eq!(p.turns[2].blocks[0].kind, BlockKind::ToolResult);
    assert!(p.turns[2].blocks[0].text.contains("sleep 90"));
    assert_eq!(
        p.turns[2].blocks[1],
        Block::text("unfinished (step was still running)")
    );
    assert!(p.turns[2].blocks[1].text.contains("unfinished"));

    assert_eq!(p.turns[6].role, "assistant");
    assert!(p.turns[6].blocks[0].text.contains("FIG-4410"));
}

#[test]
fn evidence_l2b_two_rejoins_keeps_duplicate_step_index_four_in_file_order() {
    let p = project_agy(L2B_TWO_REJOINS);
    assert_eq!(p.unavailable, None);
    assert_eq!(p.skipped_lines, 0);
    assert_eq!(p.unrecognized_records, 0);
    assert_eq!(p.turns.len(), 11);

    // First row with step_index=4 is at file index 4 (USER_INPUT asking for BETA).
    assert_eq!(p.turns[4].role, "user");
    assert!(p.turns[4].blocks[0].text.contains("BETA"));
    // Row with step_index=6 is at file index 6 (ALPHA + BETA).
    assert_eq!(p.turns[6].role, "assistant");
    assert_eq!(
        p.turns[6].blocks,
        vec![Block::text("ALPHA LIME-8802\nBETA LIME-8802")]
    );
    // Second row with step_index=4 follows index 6 at file index 7 (ALPHA only).
    assert_eq!(p.turns[7].role, "assistant");
    assert_eq!(p.turns[7].blocks, vec![Block::text("ALPHA LIME-8802")]);
}

#[test]
fn evidence_l2_neon_effort_and_rejoins_and_printed_value_bind() {
    // Turn carries {role, ts, blocks} and no model field, so per-row model slugs
    // (gemini-4-neon-high, gemini-4-neon-low) are absent without loss.
    let neon = project_agy(L2_NEON_EFFORT);
    assert_eq!(neon.unavailable, None);
    assert_eq!(neon.skipped_lines, 0);
    assert_eq!(neon.unrecognized_records, 0);
    assert_eq!(neon.turns.len(), 5);
    assert_eq!(neon.turns[1].blocks, vec![Block::text("READY")]);
    assert_eq!(neon.turns[3].role, "system");
    assert_eq!(neon.turns[4].blocks, vec![Block::text("AGAIN")]);
    for turn in &neon.turns {
        for block in &turn.blocks {
            assert!(!block.text.contains("gemini-4-neon-high"));
            assert!(!block.text.contains("gemini-4-neon-low"));
        }
    }

    let rejoins = project_agy(L2_REJOINS);
    assert_eq!(rejoins.unavailable, None);
    assert_eq!(rejoins.skipped_lines, 0);
    assert_eq!(rejoins.unrecognized_records, 0);
    assert_eq!(rejoins.turns.len(), 11);
    assert_eq!(rejoins.turns[1].blocks, vec![Block::text("STORED")]);
    assert_eq!(rejoins.turns[10].blocks, vec![Block::text("PLUM-7731")]);

    let printed = project_agy(L2B_PRINTED_VALUE);
    assert_eq!(printed.unavailable, None);
    assert_eq!(printed.skipped_lines, 0);
    assert_eq!(printed.unrecognized_records, 0);
    assert_eq!(printed.turns.len(), 4);
    assert_eq!(printed.turns[1].blocks[0].kind, BlockKind::Tool);
    assert_eq!(
        printed.turns[1].blocks[0].text,
        r#"run_command CommandLine="printenv L2B_SENTINEL", WaitMsBeforeAsync=5000, toolSummary="Check L2B_SENTINEL env var""#
    );
    assert_eq!(printed.turns[2].role, "tool");
    assert!(printed.turns[2].blocks[0]
        .text
        .contains("l2b-sentinel-value-7f3a9c2e41d8"));
    assert_eq!(printed.turns[3].blocks, vec![Block::text("DONE")]);
}
