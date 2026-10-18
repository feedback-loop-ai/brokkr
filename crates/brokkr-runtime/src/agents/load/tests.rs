//! The order the loaders refuse in, step against step (#288 slice L).
//! Each row writes one definition that fails two adjacent steps of
//! `parse_adapter`, `resume_assessment` or `parse_agent`, and the earlier
//! step's refusal is the one reported. A row's text names the file's own
//! description as `{what}`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::{Adapters, Library, LibraryError};

/// Each row's patch written over `loadable` as `file`, alone in a fresh
/// canonical directory beside a charter that declares nothing, and
/// loaded: the refusals observed, beside each row's text with `{what}`
/// read as `described` makes it of the file's path.
fn refusals<T: std::fmt::Debug>(
    rows: &[(Value, &str)],
    (file, loadable): (&str, &Value),
    load: fn(&Path) -> Result<T, LibraryError>,
    described: fn(&Path) -> String,
) -> (Vec<String>, Vec<String>) {
    rows.iter()
        .map(|(patch, expected)| {
            let dir = tempfile::tempdir().unwrap();
            let root: PathBuf = dir.path().canonicalize().unwrap();
            let mut body = loadable.clone();
            for (key, value) in patch.as_object().unwrap() {
                body[key] = value.clone();
            }
            std::fs::create_dir(root.join("charters")).unwrap();
            std::fs::write(root.join("charters/plain.md"), "# office\n").unwrap();
            std::fs::write(root.join(file), body.to_string()).unwrap();
            let refused = load(&root).unwrap_err().to_string();
            (
                refused,
                expected.replace("{what}", &described(&root.join(file))),
            )
        })
        .unzip()
}

/// Each row's refusal from `Adapters::load`, over an adapter that loads.
fn adapter_refusals(rows: &[(Value, &str)]) -> (Vec<String>, Vec<String>) {
    let loadable = json!({
        "effort_flag": "--effort", "efforts": ["high"], "model_flag": "--model",
        "models": {"opus": "claude-opus-5"}, "mcp": "unsupported", "provider": "claude",
        "tool_permissions": "unsupported", "binary": "claude",
        "driver": ["{brokkr}", "driver", "claude", "--"],
    });
    refusals(rows, ("claude.json", &loadable), Adapters::load, |path| {
        format!("adapter 'claude' ({})", path.display())
    })
}

/// A typed MCP shape measured on this adapter's own harness and binary,
/// which admission refuses when the adapter declares no boxed hands.
fn boxed_mcp() -> Value {
    let measured = json!({"measured": "m"});
    json!({"carriage": measured, "shapes": [{
        "invocation": "cold", "hands": "boxed", "ambient": measured,
        "native_write": measured, "store_read": measured, "process_read": measured,
        "measured_on": {"harness": "claude", "binary": "claude", "version": "1.0.0",
                        "host": "linux"},
    }]})
}

#[test]
fn an_adapter_failing_two_adjacent_steps_reports_the_earlier() {
    let rows = [
        // pinning, then hands; and inside pinning, efforts, then effortless routes.
        (
            json!({"efforts": ["Xhigh"], "hands": "maybe"}),
            "{what} 'efforts' names 'Xhigh', which does not match ^[a-z][a-z0-9-]*$",
        ),
        (
            json!({"efforts": ["Xhigh"], "effortless_routes": []}),
            "{what} 'efforts' names 'Xhigh', which does not match ^[a-z][a-z0-9-]*$",
        ),
        // hands, then resume.
        (
            json!({"hands": "maybe", "resume": true}),
            "{what} 'hands' is the string 'maybe'; the only legal string here is \
             \"unsupported\" — a provider's missing capability is declared, never inferred",
        ),
        // resume, then credentials.
        (
            json!({"resume": true, "credentials": []}),
            "{what} 'resume' must be an object of named execution shapes; a bare value \
             cannot say WHICH shape was measured, and a shape nobody named enables nothing",
        ),
        // credentials, then MCP admission.
        (
            json!({"credentials": [], "mcp": boxed_mcp()}),
            "{what} 'credentials' must be an object of route name → environment variable name",
        ),
        // MCP admission, then the fields settled after it.
        (
            json!({"mcp": boxed_mcp(), "trust_tier": "trusetd"}),
            "{what} 'mcp' shape 'cold' with hands 'boxed' on linux names hands this adapter \
             does not declare; a measurement supplies no hands",
        ),
    ];
    let (observed, expected) = adapter_refusals(&rows);
    assert_eq!(observed, expected);
}

#[test]
fn a_resume_shape_failing_two_adjacent_steps_reports_the_earlier() {
    let shape = |patch: Value| {
        let mut shape = json!({"status": "unmeasured", "identity": {"unknown": "u"},
                               "classes": [], "boundaries": [], "hands": "boxed"});
        for (key, value) in patch.as_object().unwrap() {
            shape[key] = value.clone();
        }
        json!({"resume": {"cold": shape}})
    };
    let rows = [
        // the shape's name, then the shape.
        (
            json!({"resume": {"Cold": 7}}),
            "{what} 'resume' names shape 'Cold', which does not match ^[a-z][a-z0-9-]*$",
        ),
        // the shape's keys, then its status.
        (
            json!({"resume": {"cold": {"invented": 1}}}),
            "{what} 'resume' 'cold' has unknown key 'invented'; known keys: status, identity, \
             classes, boundaries, hands, evidence, limitations, reason",
        ),
        // the status, then the identity.
        (
            json!({"resume": {"cold": {"status": "maybe"}}}),
            "{what} 'resume' 'cold' needs 'status': one of unmeasured, unsupported, supported",
        ),
        // the evidence, then the limitations.
        (
            shape(json!({"evidence": 7, "limitations": 7})),
            "{what} 'resume' 'cold' 'evidence' must be a JSON object",
        ),
        // the limitations, then the reason.
        (
            shape(json!({"limitations": [""], "reason": ""})),
            "{what} 'resume' 'cold' 'limitations' must be a bounded non-empty line of at most \
             400 characters",
        ),
        // the reason, then the two enablement rules.
        (
            shape(json!({"status": "supported", "reason": ""})),
            "{what} 'resume' 'cold' needs a non-empty string 'reason'",
        ),
    ];
    let (observed, expected) = adapter_refusals(&rows);
    assert_eq!(observed, expected);
}

#[test]
fn an_agent_failing_two_adjacent_steps_reports_the_earlier() {
    let loadable = json!({"models": ["opus"], "charter": "charters/plain.md", "description": "d"});
    let rows = [
        // capabilities, then the description and charter.
        (
            json!({"capabilities": ["web-search"], "description": ""}),
            "agent 'tester' 'capabilities' must be an object from capability name to \
             \"requires\" or \"wants\"; a request names no dialect, tool or grant, because \
             only realms.json grants (decision 0065 ruling 3)",
        ),
        // the model chain, then the tools.
        (
            json!({"models": [], "tools": []}),
            "{what} 'models' is empty; an agent names at least one model",
        ),
        // the tools, then the hands.
        (
            json!({"tools": [], "hands": 7}),
            "{what} 'tools' must be a JSON object",
        ),
        // the hands, then the limits.
        (
            json!({"hands": 7, "limits": []}),
            "{what} 'hands': hands must be \"workspace\" or an object, got 7",
        ),
        // the limits, then the inputs.
        (
            json!({"limits": [], "inputs": 7}),
            "{what} 'limits' must be a JSON object",
        ),
        // the inputs, then the charter's check of what the office asks for.
        (
            json!({"inputs": 7, "capabilities": {"web-search": "requires"}}),
            "{what} needs 'inputs' as an array of strings",
        ),
    ];
    let (observed, expected) = refusals(&rows, ("tester.json", &loadable), Library::load, |path| {
        format!("agent 'tester' ({})", path.display())
    });
    assert_eq!(observed, expected);
}
