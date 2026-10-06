use serde_json::{json, Map, Value};

use super::super::{fold_codex_event, fold_dsh_event, fold_stream_event, CodexThreadEcho};
use super::{Format, Observation, Tool};
use crate::transcript::{Kind as TranscriptKind, Transcript};

/// An MCP tool name that, under its `mcp__engine__` prefix, runs past the
/// 80-character clamp.
const LONG_TOOL: &str =
    "a_tool_whose_name_runs_well_past_the_eighty_characters_a_legacy_row_shows_of_it";

fn seen(format: Format, call: Option<&str>, tool: Tool) -> Observation {
    Observation {
        format,
        call: call.map(str::to_string),
        tool,
    }
}

fn named(name: &str) -> Tool {
    Tool::Named {
        name: name.to_string(),
    }
}

fn mcp(server: &str, tool: &str) -> Tool {
    Tool::Mcp {
        server: server.to_string(),
        tool: tool.to_string(),
    }
}

fn unidentified(reported: &str) -> Tool {
    Tool::McpUnidentified {
        reported: reported.to_string(),
    }
}

fn claude_block(id: &str, name: &str) -> Value {
    json!({"type": "tool_use", "id": id, "name": name, "input": {"query": "brokkr"}})
}

fn codex_item(item: Value) -> Value {
    json!({"type": "item.completed", "item": item})
}

fn dsh_call(call: &str, name: &str) -> Value {
    json!({"type": "tool/call", "seq": 7,
           "data": {"turn": 1, "step": 1, "callId": call, "name": name, "arguments": "{}"}})
}

/// U0's measured shapes, read whole: the concrete tool, the MCP server and
/// tool, and the harness's call id. Codex's `mcp_tool_call` is a category
/// and is never taken for a tool; a name the measured grammar does not
/// split stays an unidentified MCP call, and nothing is clamped here.
#[test]
fn each_format_reads_the_measured_call_identity_whole() {
    let long = format!("mcp__engine__{LONG_TOOL}");
    let cases = [
        (
            Observation::claude(&claude_block("toolu_01", "WebSearch")),
            seen(Format::Claude, Some("toolu_01"), named("WebSearch")),
        ),
        (
            Observation::claude(&claude_block("toolu_02", "mcp__plugin_docs_srv__lookup")),
            seen(
                Format::Claude,
                Some("toolu_02"),
                mcp("plugin_docs_srv", "lookup"),
            ),
        ),
        (
            Observation::claude(&claude_block("toolu_03", &long)),
            seen(Format::Claude, Some("toolu_03"), mcp("engine", LONG_TOOL)),
        ),
        (
            Observation::claude(&claude_block("toolu_04", "mcp__engine")),
            seen(
                Format::Claude,
                Some("toolu_04"),
                unidentified("mcp__engine"),
            ),
        ),
        (
            Observation::claude(&json!({"type": "tool_use", "name": 7})),
            seen(Format::Claude, None, Tool::Missing),
        ),
        (
            Observation::codex(&codex_item(json!({"id": "item_0",
                "type": "command_execution", "command": "ls", "exit_code": 0}))),
            seen(Format::Codex, Some("item_0"), named("command_execution")),
        ),
        (
            Observation::codex(&codex_item(json!({"id": "item_1", "type": "mcp_tool_call",
                "server": "engine", "tool": LONG_TOOL, "status": "completed"}))),
            seen(Format::Codex, Some("item_1"), mcp("engine", LONG_TOOL)),
        ),
        (
            Observation::codex(&codex_item(json!({"id": "item_2", "type": "mcp_tool_call",
                "server": "", "tool": "probe"}))),
            seen(Format::Codex, Some("item_2"), unidentified("mcp_tool_call")),
        ),
        (
            Observation::codex(&codex_item(json!({"id": "item_3", "type": "mcp_tool_call",
                "server": "s", "tool": ""}))),
            seen(Format::Codex, Some("item_3"), unidentified("mcp_tool_call")),
        ),
        (
            Observation::codex(&codex_item(
                json!({"type": "mcp_tool_call", "server": "engine"}),
            )),
            seen(Format::Codex, None, unidentified("mcp_tool_call")),
        ),
        (
            Observation::codex(&codex_item(json!({}))),
            seen(Format::Codex, None, Tool::Missing),
        ),
        (
            Observation::dsh(&dsh_call("call_1", "bash")),
            seen(Format::Dsh, Some("call_1"), named("bash")),
        ),
        (
            Observation::dsh(&dsh_call("call_2", "mcp__engine__u0_engine_probe")),
            seen(
                Format::Dsh,
                Some("call_2"),
                mcp("engine", "u0_engine_probe"),
            ),
        ),
        (
            Observation::dsh(&dsh_call("call_3", "mcp____probe")),
            seen(Format::Dsh, Some("call_3"), unidentified("mcp____probe")),
        ),
        (
            Observation::dsh(&dsh_call("call_4", "mcp__server__")),
            seen(Format::Dsh, Some("call_4"), unidentified("mcp__server__")),
        ),
    ];
    for (read, want) in cases {
        assert_eq!(read, want);
    }
}

/// The one wire encoding: each reading encodes to exactly this object,
/// its tool's variant under `kind` and an absent call id as `null`, and
/// decodes back to the same observation.
#[test]
fn each_reading_has_one_encoding_and_decodes_back() {
    let cases = [
        (
            Observation::claude(&claude_block("toolu_01", "WebSearch")),
            json!({"format": "claude", "call": "toolu_01",
                   "tool": {"kind": "named", "name": "WebSearch"}}),
        ),
        (
            Observation::codex(&codex_item(json!({"id": "item_1", "type": "mcp_tool_call",
                "server": "engine", "tool": "probe"}))),
            json!({"format": "codex", "call": "item_1",
                   "tool": {"kind": "mcp", "server": "engine", "tool": "probe"}}),
        ),
        (
            Observation::dsh(&dsh_call("call_3", "mcp____probe")),
            json!({"format": "dsh", "call": "call_3",
                   "tool": {"kind": "mcp_unidentified", "reported": "mcp____probe"}}),
        ),
        (
            Observation::codex(&codex_item(json!({}))),
            json!({"format": "codex", "call": null, "tool": {"kind": "missing"}}),
        ),
    ];
    for (read, wire) in cases {
        assert_eq!(serde_json::to_value(&read).unwrap(), wire);
        let decoded: Observation = serde_json::from_value(wire).unwrap();
        assert_eq!(decoded, read);
    }
}

fn claude_rows(blocks: Vec<Value>) -> Vec<Value> {
    let mut rows = Vec::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    let event = json!({"type": "assistant", "message": {"model": "claude-opus-5-5",
        "usage": {"input_tokens": 13, "output_tokens": 2}, "content": blocks}});
    let refused = fold_stream_event(
        &event,
        &mut 0,
        &mut Map::new(),
        &mut transcript,
        &mut |row| {
            rows.push(row.clone());
        },
    );
    assert_eq!(refused, None);
    rows
}

fn codex_rows(events: &[Value]) -> Vec<Value> {
    let mut rows = Vec::new();
    let mut transcript = Transcript::resolve(TranscriptKind::CodexThread).unwrap();
    let mut echo = CodexThreadEcho::default();
    let (mut turn, mut meta) = (1, Map::new());
    for event in events {
        let mut emit = |row: &Value| rows.push(row.clone());
        let refused = fold_codex_event(
            event,
            &mut turn,
            &mut meta,
            &mut transcript,
            &mut echo,
            &mut emit,
        );
        assert_eq!(refused, None);
    }
    rows
}

fn dsh_rows(turns: u64, events: &[Value]) -> Vec<Value> {
    let mut rows = Vec::new();
    let (mut turns, mut meta) = (turns, Map::new());
    for event in events {
        fold_dsh_event(event, None, &mut turns, &mut meta, &mut |row| {
            rows.push(row.clone());
        });
    }
    rows
}

/// The rows the three folds write are exactly the legacy rows: the tool a
/// row always showed, clamped to 80 characters, Codex's item category
/// for every item and both its start and completion, and no call id,
/// server or any other key the normalized reading holds.
#[test]
fn the_folds_still_write_exactly_the_legacy_rows() {
    let long = format!("mcp__engine__{LONG_TOOL}");
    let shown: String = long.chars().take(80).collect();
    let turn = json!({"step": "seat-turn", "turn": 1, "model": "claude-opus-5-5",
                      "effort": "not reported"});
    let mut first = turn.clone();
    first["input_tokens"] = json!(13);
    first["output_tokens"] = json!(2);
    first["tool"] = json!("WebSearch");
    let mut second = turn.clone();
    second["tool"] = json!(shown);
    let mut third = turn.clone();
    third["tool"] = json!("mcp__engine");
    let blocks = vec![
        claude_block("toolu_01", "WebSearch"),
        claude_block("toolu_02", &long),
        claude_block("toolu_03", "mcp__engine"),
        json!({"type": "tool_use", "id": "toolu_04", "name": ""}),
    ];
    assert_eq!(
        claude_rows(blocks),
        vec![first, second, third, turn.clone()]
    );

    let mcp_item = json!({"id": "item_0", "type": "mcp_tool_call", "server": "engine",
                          "tool": LONG_TOOL, "arguments": {}, "status": "in_progress"});
    let events = [
        json!({"type": "item.started", "item": mcp_item}),
        json!({"type": "item.completed", "item": mcp_item}),
        json!({"type": "item.completed", "item": {"id": "item_1", "type": "mcp_tool_call"}}),
        json!({"type": "item.started", "item": {"id": "item_2", "type": "command_execution"}}),
        json!({"type": "item.completed", "item": {"id": "item_3", "type": ""}}),
        json!({"type": "item.completed", "item": {"id": "item_4"}}),
    ];
    let item =
        |step: &str, tool: &str| json!({"step": step, "turn": 1, "tool": tool, "harness": "codex"});
    assert_eq!(
        codex_rows(&events),
        vec![
            item("item-started", "mcp_tool_call"),
            item("item-completed", "mcp_tool_call"),
            item("item-completed", "mcp_tool_call"),
            item("item-started", "command_execution"),
            item("item-completed", ""),
            item("item-completed", "unknown"),
        ]
    );

    let calls = [
        dsh_call("call_1", "bash"),
        dsh_call("call_2", &long),
        dsh_call("call_3", ""),
        json!({"type": "tool/call", "data": {"callId": "call_4"}}),
    ];
    let row = |tool: &str| {
        json!({"step": "seat-turn", "turn": 1, "harness": "deepseek", "model": "not reported",
               "effort": "not reported", "tool": tool})
    };
    assert_eq!(dsh_rows(1, &calls), vec![row("bash"), row(&shown)]);
    assert_eq!(dsh_rows(0, &calls), Vec::<Value>::new());
}
