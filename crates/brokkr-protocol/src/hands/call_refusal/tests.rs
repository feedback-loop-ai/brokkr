//! `call_refusal.rs`'s own unit tests: each check of a `tools/call`'s
//! params names its own variant, each bound admits what it always has,
//! and every variant reads as the text the operator has always seen
//! (#353).

use std::time::Duration;

use serde_json::{json, Map, Value};

use super::*;
use crate::hands::{checked_call, DEFAULT_TIMEOUT_MS};

/// A call of the served tool with `fields` set over its params; a null
/// field is dropped, so `{"name": null}` is a call without a name.
fn call(fields: Value) -> Value {
    let mut params = Map::new();
    params.insert("name".to_string(), json!(TOOL_NAME));
    params.insert("arguments".to_string(), json!({"command": "pwd"}));
    params.extend(fields.as_object().unwrap().clone());
    params.retain(|_, value| !value.is_null());
    Value::Object(params)
}

/// A call whose arguments are `pwd` with `timeout_ms` beside it.
fn timed(timeout_ms: Value) -> Value {
    call(json!({"arguments": {"command": "pwd", "timeoutMs": timeout_ms}}))
}

#[test]
fn each_call_check_refuses_in_its_own_variant() {
    let refused = |params: &Value, refusal: CallRefusal<'_>| {
        assert_eq!(checked_call(params), Err(refusal), "{params}");
    };
    for name in [Value::Null, json!(7), json!("shell")] {
        refused(&call(json!({"name": name})), CallRefusal::UnknownTool);
    }
    refused(
        &call(json!({"arguments": {"command": "pwd", "cwd": "/"}})),
        CallRefusal::UnknownArgument { key: "cwd" },
    );
    let long = "x".repeat(16_385);
    for arguments in [
        json!({}),
        json!({"command": "\t \n"}),
        json!({"command": long}),
    ] {
        refused(&call(json!({"arguments": arguments})), CallRefusal::Command);
    }
    for arguments in [Value::Null, json!("pwd")] {
        refused(&call(json!({"arguments": arguments})), CallRefusal::Command);
    }
    for timeout_ms in [json!(99), json!(3_600_001), json!("long"), json!(1000.5)] {
        refused(&timed(timeout_ms), CallRefusal::Timeout);
    }
}

/// Words that say characters, a check that counts bytes: 4,097 four-byte
/// characters are 16,388 bytes, and refused, as they always were (an ask
/// on #353, kept exactly).
#[test]
fn a_command_s_bound_counts_bytes_not_characters() {
    let wide = "\u{1F980}".repeat(4_097);
    assert_eq!(
        checked_call(&call(json!({"arguments": {"command": wide}}))),
        Err(CallRefusal::Command)
    );
}

#[test]
fn each_bound_admits_its_exact_arguments() {
    let admitted = |params: Value, command: &str, timeout_ms: u64| {
        let expected = (command.to_string(), Duration::from_millis(timeout_ms));
        assert_eq!(checked_call(&params), Ok(expected), "{params}");
    };
    let longest = "y".repeat(16_384);
    admitted(
        call(json!({"arguments": {"command": longest}})),
        &longest,
        DEFAULT_TIMEOUT_MS,
    );
    admitted(timed(json!(100)), "pwd", 100);
    admitted(timed(json!(600_001)), "pwd", 600_001);
    admitted(timed(json!(3_600_000)), "pwd", 3_600_000);
    admitted(call(json!({})), "pwd", DEFAULT_TIMEOUT_MS);
}

#[test]
fn every_call_refusal_reads_as_it_always_has() {
    for (refusal, text) in [
        (
            CallRefusal::UnknownTool,
            "unknown tool; this server serves only 'workspace'",
        ),
        (
            CallRefusal::UnknownArgument { key: "cwd" },
            "workspace: unknown argument 'cwd'",
        ),
        (
            CallRefusal::Command,
            "workspace: command must contain 1-16384 characters",
        ),
        (
            CallRefusal::Timeout,
            "workspace: timeoutMs must be an integer from 100 through 3600000",
        ),
    ] {
        assert_eq!(refusal.to_string(), text);
    }
}

/// Decision 0123: the tool a harness is shown advertises the bound the
/// call check holds, an hour, in its schema and its description.
#[test]
fn the_advertised_timeout_bound_is_the_checked_one() {
    let definition = crate::hands::tool_definition();
    let timeout = &definition["inputSchema"]["properties"]["timeoutMs"];
    assert_eq!(timeout["minimum"], 100);
    assert_eq!(timeout["maximum"], 3_600_000);
    let description = definition["description"].as_str().unwrap();
    assert!(
        description.ends_with("may not exceed 3600000."),
        "{description}"
    );
}
