//! Level 3 of #1224: where the engine's set serves the box's hands, an agy
//! launch is left one tool, the engine's. Its gate lists `call_mcp_tool`
//! alone, it carries no mode, its one `--settings` value holds the engine's
//! rules alone, and a seat that grants a native tool beside them is refused
//! before agy starts. The unboxed launch stays level 1's.

use super::*;
use crate::native_controls::SealedBoundary::Namespace;

/// The engine's one rule for the box's hands.
const RULE: &str = "mcp(brokkr/workspace)";

/// A recorded turn of the fourth measurement, read when the test runs: the
/// record lives outside the crate, so no `include_str!` may reach it
/// (packaging's `no_crate_includes_a_file_from_outside_itself`).
fn recorded_box(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/evidence/adapters/agy-cli-1-5")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

/// A boxed launch: the stream head with no mode, the pin, the effort, and
/// the one settings value holding the engine's rules and nothing else.
#[test]
fn a_boxed_launch_carries_no_mode_and_the_engines_rules_alone() {
    let rules = [RULE.to_string()];
    let seat = argv(&["--effort", "high", "--model", "gemini-4-neon"]);
    let launched = launch("agy", &seat, Hands::Boxed(&rules)).unwrap();
    let golden = [
        "agy",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--model",
        "gemini-4-neon",
        "--effort",
        "high",
        "--settings",
        r#"{"permissions":{"allow":["mcp(brokkr/workspace)"]}}"#,
    ];
    assert_eq!(launched.command, argv(&golden));
    assert!(!launched.command.iter().any(|part| part == "--mode"));
}

/// A boxed seat that grants a native tool is refused by `--allow`'s name,
/// never its value, whatever it grants; the same grant is admitted unboxed.
#[test]
fn a_boxed_seat_granting_a_native_tool_is_refused_naming_allow_alone() {
    let rules = [RULE.to_string()];
    for grant in ["command(git)", RULE, "command(cargo),command(ls)"] {
        let seat = argv(&["--model", "gemini-4-neon", "--allow", grant]);
        let refused = launch("agy", &seat, Hands::Boxed(&rules)).unwrap_err();
        assert!(matches!(refused, LaunchError::BoxedGrant), "{refused:?}");
        assert!(!refused.to_string().contains(grant), "{refused}");
        assert!(launch("agy", &seat, Hands::Native).is_ok(), "{grant}");
    }
}

/// The seat's input with the box's hands sealed and admitted past the
/// lifted fence, with the reach and artifact root the engine writes.
fn boxed_input(world: &World) -> Value {
    let serving = sealed(Namespace, "hands", json!("measured"), "lifted");
    let mut input = world.input(Some(serving));
    let reach = super::super::super::mcp::tests::reach();
    input[crate::hands::SEALED_REACH] = serde_json::to_value(reach).unwrap();
    input[crate::native_controls::mcp::ARTIFACT_ROOT] = json!(world.path("artifacts"));
    input
}

/// A boxed seat run with `--allow` ends as the typed refusal: the fake
/// `agy`, which records its start, never runs, and no engine HOME is left.
#[test]
fn a_boxed_seat_with_allow_is_refused_before_agy_starts_and_leaves_no_home() {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    let extra = ["--model", FLASH, "--allow", "command(git)"];
    let messages = world.run(&bin, &extra, boxed_input(&world), None);
    let error = ended(&messages).and_then(|(_, error, _)| error);
    assert_eq!(error, Some(LaunchError::BoxedGrant.to_string()));
    assert!(!world.started());
    assert!(world.homes_gone());
}

/// What one seat on `input` handed agy through the fake: its argv, its
/// HOME's MCP document, and its gate's command split as a shell splits it.
fn handed(input: impl FnOnce(&World) -> Value) -> (Vec<String>, Value, Vec<String>) {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
    let input = input(&world);
    let messages = world.run(&bin, &["--model", FLASH, "--effort", "low"], input, None);
    let (status, _, _) = ended(&messages).unwrap();
    assert_eq!(status, crate::ResultStatus::Succeeded);
    let argv = world.recorded("argv").lines().map(str::to_string).collect();
    let mcp = serde_json::from_str(&world.recorded("mcp")).unwrap();
    let hooks: Value = serde_json::from_str(&world.recorded("hooks")).unwrap();
    let command = hooks["brokkr-gate"]["PreToolUse"][0]["hooks"][0]["command"].clone();
    let words = std::process::Command::new("sh")
        .args([
            "-c",
            &format!("printf '%s\\n' {}", command.as_str().unwrap()),
        ])
        .output()
        .unwrap();
    let words = String::from_utf8(words.stdout).unwrap();
    (argv, mcp, words.lines().map(str::to_string).collect())
}

/// B6: the unboxed launch is level 1's, byte for byte, and the boxed one
/// differs from it in exactly the mode, the gate's list, the rules and the
/// MCP document, which names the hands server.
#[test]
fn a_boxed_launch_differs_from_the_unboxed_in_its_mode_list_rules_and_document_alone() {
    let (open_argv, open_mcp, open_gate) = handed(|world| world.input(None));
    let (boxed_argv, boxed_mcp, boxed_gate) = handed(boxed_input);
    let tail = ["--model", FLASH, "--effort", "low", "--settings"];
    let open = [&LEAD[1..], &tail, &[r#"{"permissions":{"allow":[]}}"#]].concat();
    assert_eq!(open_argv, open);
    assert_eq!(open_mcp, json!({"mcpServers": {}}));
    let level_one = [
        "read_file",
        "write_file",
        "edit_file",
        "run_command",
        "manage_task",
    ];
    let (program, list) = open_gate.split_at(5);
    assert_eq!(program[1..], ["driver", "agy", "--", "--gate"]);
    assert_eq!(list, level_one);
    // The mode out and the rule in, and nothing else of the argv.
    let mut moded = boxed_argv.clone();
    moded.splice(4..4, argv(&["--mode", "accept-edits"]));
    *moded.last_mut().unwrap() = r#"{"permissions":{"allow":[]}}"#.to_string();
    assert_eq!(moded, open_argv);
    let settings = format!(r#"{{"permissions":{{"allow":["{RULE}"]}}}}"#);
    assert_eq!(boxed_argv.last(), Some(&settings));
    assert!(!boxed_argv.iter().any(|part| part == "--mode"));
    let servers: Vec<&String> = boxed_mcp["mcpServers"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert_eq!(servers, ["brokkr"]);
    let args = boxed_mcp["mcpServers"]["brokkr"]["args"]
        .as_array()
        .unwrap();
    assert_eq!(args[..2], [json!("hands"), json!("serve")]);
    assert_eq!(boxed_gate[..5], *program);
    assert_eq!(boxed_gate[5..], ["call_mcp_tool"]);
}

/// The recorded boxed turns fold through the driver's fold to their
/// answers: each native call the record's gate denied is a tool error the
/// turn went on past, and none is a `denied_actions` failure.
#[test]
fn the_recorded_boxed_turns_answer_past_the_natives_the_gate_denied() {
    let natives = ["read_file", "run_command", "write_file"];
    let turns = [
        ("box-engine-workspace-tool.ndjson", 4, &natives[..]),
        (
            "box-default-notice.ndjson",
            6,
            &["read_file", "list_resources"],
        ),
    ];
    for (name, responses, denied) in turns {
        let stdout = recorded_box(name);
        let read = read(&stdout, FLASH);
        assert_eq!(read.stopped, None, "{name}");
        let answered = answer(read.fold.verdict(0, false));
        assert_eq!(json!(answered), response(&stdout), "{name}");
        let rows: Vec<Value> = (1..=responses).map(|turn| turn_row(turn, FLASH)).collect();
        assert_eq!(read.rows, rows, "{name}");
        for tool in denied {
            let step =
                format!("\"state\":\"ERROR\",\"step_type\":\"tool\",\"tool_name\":\"{tool}\"");
            assert!(stdout.contains(&step), "{name}: {tool}");
        }
        assert!(!stdout.contains("denied_actions"), "{name}");
    }
}
