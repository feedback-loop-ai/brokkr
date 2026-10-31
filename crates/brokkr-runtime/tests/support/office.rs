//! An office over an operator adapter whose driver is a fixture's own
//! command (decision 0065 slice two, U9b1). Past the MCP compile fence,
//! lifted on Linux, SI2 refuses an inline opaque command, and here
//! `{brokkr}` names this test binary rather than brokkr, so the exec driver
//! cannot carry one. A fixture's seat hires this office instead: its
//! adapter carries synthetic plumbing facts measured under its own harness
//! (`synthetic_mcp`), never U0's.

use std::path::Path;

use serde_json::{json, Value};

/// A driver that answers `result`, first running `before` in the tree it
/// was started in.
pub(crate) fn answering(before: &str, result: &str) -> Vec<String> {
    let script = format!(
        r#"read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
{before}
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{{"result":"{result}"}},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#
    );
    vec!["sh".into(), "-c".into(), script]
}

/// Writes, under `root`, the adapter `office` whose driver is `driver`,
/// with synthetic facts, and the office `office` hiring its one model,
/// holding `limits` where given.
pub(crate) fn hire(root: &Path, office: &str, driver: &[&str], limits: Option<Value>) {
    let adapter = json!({
        "provider": office,
        "binary": driver[0],
        "driver": driver,
        "models": {office: format!("{office}/scripted")},
        "model_flag": "--model",
        "efforts": ["high"],
        "effort_flag": "--effort",
        "tool_permissions": "unsupported",
    });
    let adapter = crate::synthetic_mcp::measured(adapter);
    crate::hired_office::write(root, office, &adapter, limits);
}
