use serde_json::{json, Map, Value};

use super::super::claude_fold::{fold_stream_event, Source::Claude};
use super::super::tests::fold_codex_into;
use super::super::{dsh_effort, fold_dsh_event, CodexThreadEcho, Row};
use super::{Format, Observation, Tool, OBSERVATION_KEY};
use crate::secret::{self, BoundSecret};
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

fn codex_observation(item: Value) -> Observation {
    Observation::codex(&Row::new(&codex_item(item), &[]))
}

#[test]
fn codex_classifies_raw_mcp_and_masks_each_quoted_field() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("bindings");
    crate::secret::store_set(&store, "S", "tool").unwrap();
    let bindings = crate::secret::resolve_bindings(&store, &["S".into()]).unwrap();
    let event = codex_item(json!({
        "id": "tool_1", "type": "mcp_tool_call", "server": "toolbox", "tool": "read_tool"
    }));
    let row = Row::new(&event, &bindings);
    let call = Observation::codex(&row);
    assert_eq!(
        call,
        seen(
            Format::Codex,
            Some("[secret:S]_1"),
            mcp("[secret:S]box", "read_[secret:S]")
        )
    );
    assert_eq!(
        call.onto_codex(&row, json!({}))["tool"],
        "mcp_[secret:S]_call"
    );
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
            Observation::claude(&Row::new(&claude_block("toolu_01", "WebSearch"), &[])),
            seen(Format::Claude, Some("toolu_01"), named("WebSearch")),
        ),
        (
            Observation::claude(&Row::new(
                &claude_block("toolu_02", "mcp__plugin_docs_srv__lookup"),
                &[],
            )),
            seen(
                Format::Claude,
                Some("toolu_02"),
                mcp("plugin_docs_srv", "lookup"),
            ),
        ),
        (
            Observation::claude(&Row::new(&claude_block("toolu_03", &long), &[])),
            seen(Format::Claude, Some("toolu_03"), mcp("engine", LONG_TOOL)),
        ),
        (
            Observation::claude(&Row::new(&claude_block("toolu_04", "mcp__engine"), &[])),
            seen(
                Format::Claude,
                Some("toolu_04"),
                unidentified("mcp__engine"),
            ),
        ),
        (
            Observation::claude(&Row::new(&json!({"type": "tool_use", "name": 7}), &[])),
            seen(Format::Claude, None, Tool::Missing),
        ),
        (
            codex_observation(json!({"id": "item_0",
                "type": "command_execution", "command": "ls", "exit_code": 0})),
            seen(Format::Codex, Some("item_0"), named("command_execution")),
        ),
        (
            codex_observation(json!({"id": "item_1", "type": "mcp_tool_call",
                "server": "engine", "tool": LONG_TOOL, "status": "completed"})),
            seen(Format::Codex, Some("item_1"), mcp("engine", LONG_TOOL)),
        ),
        (
            codex_observation(json!({"id": "item_2", "type": "mcp_tool_call",
                "server": "", "tool": "probe"})),
            seen(Format::Codex, Some("item_2"), unidentified("mcp_tool_call")),
        ),
        (
            codex_observation(json!({"id": "item_3", "type": "mcp_tool_call",
                "server": "s", "tool": ""})),
            seen(Format::Codex, Some("item_3"), unidentified("mcp_tool_call")),
        ),
        (
            codex_observation(json!({"type": "mcp_tool_call", "server": "engine"})),
            seen(Format::Codex, None, unidentified("mcp_tool_call")),
        ),
        (
            codex_observation(json!({})),
            seen(Format::Codex, None, Tool::Missing),
        ),
        (
            Observation::dsh(&Row::new(&dsh_call("call_1", "bash"), &[])),
            seen(Format::Dsh, Some("call_1"), named("bash")),
        ),
        (
            Observation::dsh(&Row::new(
                &dsh_call("call_2", "mcp__engine__u0_engine_probe"),
                &[],
            )),
            seen(
                Format::Dsh,
                Some("call_2"),
                mcp("engine", "u0_engine_probe"),
            ),
        ),
        (
            Observation::dsh(&Row::new(&dsh_call("call_3", "mcp____probe"), &[])),
            seen(Format::Dsh, Some("call_3"), unidentified("mcp____probe")),
        ),
        (
            Observation::dsh(&Row::new(&dsh_call("call_4", "mcp__server__"), &[])),
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
            Observation::claude(&Row::new(&claude_block("toolu_01", "WebSearch"), &[])),
            json!({"format": "claude", "call": "toolu_01",
                   "tool": {"kind": "named", "name": "WebSearch"}}),
        ),
        (
            codex_observation(json!({"id": "item_1", "type": "mcp_tool_call",
                "server": "engine", "tool": "probe"})),
            json!({"format": "codex", "call": "item_1",
                   "tool": {"kind": "mcp", "server": "engine", "tool": "probe"}}),
        ),
        (
            Observation::dsh(&Row::new(&dsh_call("call_3", "mcp____probe"), &[])),
            json!({"format": "dsh", "call": "call_3",
                   "tool": {"kind": "mcp_unidentified", "reported": "mcp____probe"}}),
        ),
        (
            codex_observation(json!({})),
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
        &Row::new(&event, &[]),
        Claude,
        &mut super::super::ClaudeFold::default(),
        &mut Map::new(),
        &mut transcript,
        &mut |row| {
            rows.push(row.row.clone());
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
        let refused = fold_codex_into(
            &Row::new(event, &[]),
            &mut turn,
            &mut meta,
            &mut transcript,
            &mut echo,
            &mut rows,
        );
        assert_eq!(refused, None);
    }
    rows
}

fn dsh_rows(turns: u64, events: &[Value], bindings: &[BoundSecret]) -> Vec<Value> {
    let mut rows = Vec::new();
    let (mut turns, mut meta) = (turns, Map::new());
    let mut effort = dsh_effort::Echo::default();
    for event in events {
        let raw = Row::new(event, bindings);
        fold_dsh_event(
            event,
            &raw,
            &mut effort,
            None,
            &mut turns,
            &mut meta,
            &mut |row| {
                rows.push(row.row.clone());
            },
        );
    }
    rows
}

/// `row` carrying `observation` under the private key, as the engine
/// receives it.
fn carrying(mut row: Value, observation: Observation) -> Value {
    row[OBSERVATION_KEY] = serde_json::to_value(observation).unwrap();
    row
}

/// U4f2: each row a call lowers to carries the tool it always showed,
/// clamped to 80 characters (Codex's item category for every item, both
/// its start and its completion), and beside it the call's observation,
/// whole and unclamped, with its harness id or `null` where the event has
/// none, never a guessed one. No other key is added, a Claude call that
/// names nothing shows no tool, and dsh writes no row it never wrote.
#[test]
fn the_folds_write_each_calls_shown_tool_and_its_observation() {
    let long = format!("mcp__engine__{LONG_TOOL}");
    let shown: String = long.chars().take(80).collect();
    let turn = json!({"step": "seat-turn", "turn": 1, "model": "claude-opus-5-5",
                      "effort": "not reported"});
    let claude = |row: &Value, tool: Option<&str>, call: Option<&str>, seen_tool: Tool| {
        let mut row = row.clone();
        if let Some(tool) = tool {
            row["tool"] = json!(tool);
        }
        carrying(row, seen(Format::Claude, call, seen_tool))
    };
    let mut first = turn.clone();
    first["input_tokens"] = json!(13);
    first["output_tokens"] = json!(2);
    let blocks = vec![
        claude_block("toolu_01", "WebSearch"),
        claude_block("toolu_02", &long),
        claude_block("toolu_03", "mcp__engine"),
        json!({"type": "tool_use", "id": "toolu_04", "name": ""}),
        json!({"type": "tool_use", "name": "WebSearch"}),
    ];
    let search = Some("WebSearch");
    assert_eq!(
        claude_rows(blocks),
        vec![
            claude(&first, search, Some("toolu_01"), named("WebSearch")),
            claude(
                &turn,
                Some(&shown),
                Some("toolu_02"),
                mcp("engine", LONG_TOOL)
            ),
            claude(
                &turn,
                Some("mcp__engine"),
                Some("toolu_03"),
                unidentified("mcp__engine")
            ),
            claude(&turn, None, Some("toolu_04"), named("")),
            claude(&turn, search, None, named("WebSearch")),
        ]
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
    let item = |step: &str, tool: &str, call: &str, seen_tool: Tool| {
        let row = json!({"step": step, "turn": 1, "tool": tool, "harness": "codex"});
        carrying(row, seen(Format::Codex, Some(call), seen_tool))
    };
    let category = "mcp_tool_call";
    assert_eq!(
        codex_rows(&events),
        vec![
            item("item-started", category, "item_0", mcp("engine", LONG_TOOL)),
            item(
                "item-completed",
                category,
                "item_0",
                mcp("engine", LONG_TOOL)
            ),
            item("item-completed", category, "item_1", unidentified(category)),
            item(
                "item-started",
                "command_execution",
                "item_2",
                named("command_execution")
            ),
            item("item-completed", "", "item_3", named("")),
            item("item-completed", "unknown", "item_4", Tool::Missing),
        ]
    );

    let calls = [
        dsh_call("call_1", "bash"),
        dsh_call("call_2", &long),
        dsh_call("call_3", ""),
        json!({"type": "tool/call", "data": {"callId": "call_4"}}),
    ];
    let row = |tool: &str, call: &str, seen_tool: Tool| {
        let row = json!({"step": "seat-turn", "turn": 1, "harness": "deepseek",
                         "model": "not reported", "effort": "not reported", "tool": tool});
        carrying(row, seen(Format::Dsh, Some(call), seen_tool))
    };
    assert_eq!(
        dsh_rows(1, &calls, &[]),
        vec![
            row("bash", "call_1", named("bash")),
            row(&shown, "call_2", mcp("engine", LONG_TOOL)),
        ]
    );
    assert_eq!(dsh_rows(0, &calls, &[]), Vec::<Value>::new());
}

#[test]
fn bound_name_across_separator() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("bindings");
    secret::store_set(&store, "S", "v__t").unwrap();
    let bindings = secret::resolve_bindings(&store, &["S".into()]).unwrap();
    let call = dsh_call("call_1", "mcp__srv__tool");
    let reported = "mcp__sr[secret:S]ool";
    let rows = dsh_rows(1, &[call], &bindings);
    assert_eq!(
        rows,
        [json!({
            "step": "seat-turn", "turn": 1, "harness": "deepseek",
            "model": "not reported", "effort": "not reported", "tool": reported,
            "observation": {"format": "dsh", "call": "call_1",
                            "tool": {"kind": "mcp_unidentified", "reported": reported}},
        })],
    );
}
