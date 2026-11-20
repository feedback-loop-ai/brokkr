//! The gate mode and the hook that runs it (#1200, G8).

use serde_json::{json, Value};

use super::*;

fn names(listed: &[&str]) -> Vec<String> {
    listed.iter().map(|name| name.to_string()).collect()
}

/// What the gate handed `listed` prints for `input`, parsed.
fn answered(listed: &[&str], input: &str) -> Value {
    let mut printed = Vec::new();
    gate(&names(listed), input.as_bytes(), &mut printed).unwrap();
    let printed = String::from_utf8(printed).unwrap();
    assert_eq!(printed.matches('\n').count(), 1, "{printed}");
    serde_json::from_str(&printed).unwrap()
}

/// Only an agy driver handed `--gate` first is the gate.
#[test]
fn the_gate_is_an_agy_driver_handed_its_word_first() {
    let handed = names(&["--gate", "read_file", "run_command"]);
    assert_eq!(gate_listed(AdapterKind::Agy, &handed), Some(&handed[1..]));
    assert_eq!(gate_listed(AdapterKind::Agy, &handed[..1]), Some(&[][..]));
    assert_eq!(gate_listed(AdapterKind::Qoder, &handed), None);
    let seat = names(&["--model", "gemini-4-neon", "--gate"]);
    assert_eq!(gate_listed(AdapterKind::Agy, &seat), None);
    assert_eq!(gate_listed(AdapterKind::Agy, &[]), None);
}

/// A listed tool is left to agy's own permission check; any other is
/// denied, by its name and nothing else (`gate-prior-grant`).
#[test]
fn a_listed_tool_is_left_to_its_rules_and_another_is_denied_by_name() {
    let listed = ["read_file", "run_command"];
    let call = |name: &str| json!({"hookEventName": "PreToolUse", "toolCall": {"name": name}});
    assert_eq!(
        answered(&listed, &call("run_command").to_string()),
        json!({"decision": "deny_unless_prior_grant"})
    );
    assert_eq!(
        answered(&listed, &call("search_web").to_string()),
        json!({"decision": "deny", "reason": "the engine's gate does not offer the tool search_web"})
    );
}

/// What the gate cannot read is denied: no JSON, or JSON naming no tool.
/// A listed name inside another tool's arguments, or anywhere but the
/// call's own name, allows nothing.
#[test]
fn an_unread_call_and_a_name_only_in_the_arguments_are_denied() {
    let unread = json!({"decision": "deny", "reason": UNREAD});
    for input in [
        "not json",
        "",
        r#"{"toolCall": {}}"#,
        r#"{"name": "run_command"}"#,
    ] {
        assert_eq!(answered(&["run_command"], input), unread, "{input}");
    }
    let smuggled = json!({
        "lastUserInput": "run_command",
        "toolCall": {"name": "invoke_subagent", "args": {"name": "run_command", "tool": "run_command"}},
    });
    assert_eq!(
        answered(&["run_command"], &smuggled.to_string()),
        json!({"decision": "deny", "reason": "the engine's gate does not offer the tool invoke_subagent"})
    );
}

/// The engine HOME's one hook: every tool, ten seconds, the engine's own
/// binary quoted as one shell word, the level-1 list, and `call_mcp_tool`
/// only where the document names a server.
#[test]
fn the_hook_runs_the_gate_on_every_tool_with_the_level_one_list() {
    let hook = |served: bool| -> Value {
        serde_json::from_str(&hooks(Path::new("/opt/it's/brokkr"), served).unwrap()).unwrap()
    };
    let command = "'/opt/it'\\''s/brokkr' driver agy -- --gate read_file write_file edit_file \
                   run_command manage_task";
    let document = |command: String| {
        json!({"hooks": {"PreToolUse": [{"matcher": "", "hooks": [
            {"type": "command", "command": command, "timeout": 10},
        ]}]}})
    };
    assert_eq!(hook(false), document(command.to_string()));
    assert_eq!(hook(true), document(format!("{command} call_mcp_tool")));
    use std::os::unix::ffi::OsStrExt;
    let lossy = Path::new(std::ffi::OsStr::from_bytes(b"/opt/\xfdbrokkr"));
    assert!(matches!(hooks(lossy, false), Err(LaunchError::Unnamed)));
}
