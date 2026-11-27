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
        r#"[["run_command"]]"#,
        r#"{"toolCall": ["run_command"]}"#,
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

/// The hooks document the record's gate turns ran under
/// (`documents/gate.hooks.json`), its hook named `gate`, read when the
/// test runs: the record lives outside the crate, so no `include_str!`
/// may reach it (packaging's `no_crate_includes_a_file_from_outside_itself`).
fn recorded_hooks() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/evidence/adapters/agy-cli-1-5/documents/gate.hooks.json"
    );
    std::fs::read_to_string(path).unwrap()
}

/// A `brokkr` path holding a space and a quote.
const BROKKR: &str = "/opt/it's my/brokkr";

/// `document`'s one hook's command, taken out of it.
fn command_of(document: &mut Value) -> String {
    let hook = &mut document["PreToolUse"][0]["hooks"][0];
    let command = hook["command"].as_str().unwrap().to_string();
    hook["command"] = Value::Null;
    command
}

/// `command` split as a shell splits it.
fn split(command: &str) -> Vec<String> {
    let out = std::process::Command::new("sh")
        .args(["-c", &format!("printf '%s\\n' {command}")])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let printed = String::from_utf8(out.stdout).unwrap();
    printed.lines().map(str::to_string).collect()
}

/// The engine HOME's one hook is the record's document under the name
/// `brokkr-gate`: every tool, ten seconds. Its command, as a shell splits
/// it, is the engine's own binary in gate mode handed the level-1 list
/// where the hands are native (J10), and `call_mcp_tool` alone where they
/// are boxed (#1224, `box-engine-workspace-tool`).
#[test]
fn the_hook_runs_the_gate_on_every_tool_with_the_level_one_list_or_the_mcp_tool_alone() {
    let mut recorded: Value = serde_json::from_str(&recorded_hooks()).unwrap();
    let mut golden = recorded["gate"].take();
    command_of(&mut golden);
    let program = [BROKKR, "driver", "agy", "--", "--gate"];
    let level_one = [
        "read_file",
        "write_file",
        "edit_file",
        "run_command",
        "manage_task",
    ];
    let rules = ["mcp(brokkr/workspace)".to_string()];
    let lists: [(Hands<'_>, &[&str]); 2] = [
        (Hands::Native, &level_one),
        (Hands::Boxed(&rules), &["call_mcp_tool"]),
    ];
    for (hands, list) in lists {
        let written = hooks(Path::new(BROKKR), hands).unwrap();
        let mut document: Value = serde_json::from_str(&written).unwrap();
        let keys: Vec<&String> = document.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["brokkr-gate"]);
        let mut hook = document["brokkr-gate"].take();
        let command = command_of(&mut hook);
        assert_eq!(hook, golden);
        let words = names(&[&program[..], list].concat());
        assert_eq!(split(&command), words, "{command}");
        // Every word quoted, the program's quote closed, escaped, reopened.
        let quoted: Vec<String> = words
            .iter()
            .skip(1)
            .map(|word| format!("'{word}'"))
            .collect();
        let program = r"'/opt/it'\''s my/brokkr'";
        assert_eq!(command, format!("{program} {}", quoted.join(" ")));
    }
    use std::os::unix::ffi::OsStrExt;
    let lossy = Path::new(std::ffi::OsStr::from_bytes(b"/opt/\xfdbrokkr"));
    assert!(matches!(
        hooks(lossy, Hands::Native),
        Err(LaunchError::Unnamed)
    ));
}

/// Every tool name the fourth measurement's gates saw, `call_mcp_tool`
/// among them, read from their logs when the test runs.
fn recorded_box_names() -> std::collections::BTreeSet<String> {
    let dir = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/evidence/adapters/agy-cli-1-5"
    );
    let logs = [
        "box-engine-workspace-tool",
        "box-reach",
        "box-other-natives",
    ];
    let notices = ["box-default-notice", "box-default-notice-carbon"];
    let mut seen = std::collections::BTreeSet::new();
    for log in logs.iter().chain(&notices) {
        let text = std::fs::read_to_string(format!("{dir}/{log}.hooklog")).unwrap();
        for line in text.lines() {
            let rest = line.strip_prefix("box3 gate saw tool: ").unwrap();
            seen.insert(rest.split(' ').next().unwrap().to_string());
        }
    }
    seen
}

/// The gate a boxed launch hands its list answers `deny_unless_prior_grant`
/// for `call_mcp_tool` and denies by name each of the thirteen native names
/// the record's gates saw (#1224, `box-other-natives`).
#[test]
fn the_boxed_gate_leaves_the_mcp_tool_and_denies_every_native_the_record_saw() {
    let rules = ["mcp(brokkr/workspace)".to_string()];
    let boxed = names(listed(Hands::Boxed(&rules)));
    let call = |name: &str| json!({"toolCall": {"name": name}}).to_string();
    let answer = |name: &str| {
        let mut printed = Vec::new();
        gate(&boxed, call(name).as_bytes(), &mut printed).unwrap();
        serde_json::from_slice::<Value>(&printed).unwrap()
    };
    let mut seen = recorded_box_names();
    assert!(seen.remove("call_mcp_tool"));
    assert_eq!(seen.len(), 13, "{seen:?}");
    assert_eq!(
        answer("call_mcp_tool"),
        json!({"decision": "deny_unless_prior_grant"})
    );
    for native in &seen {
        let reason = format!("the engine's gate does not offer the tool {native}");
        assert_eq!(
            answer(native),
            json!({"decision": "deny", "reason": reason}),
            "{native}"
        );
    }
}
