//! Task 46.3 (U9b1c; MB3): MCP's version negotiation through the real
//! binary. A harness asking a version other than MB3's, as Claude Code asks
//! 2025-11-25, is answered with the broker's own 2025-06-18 result, as MCP
//! 2025-06-18's lifecycle requires of a server that does not support the
//! version asked
//! (<https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle>),
//! and the session is held to 2025-06-18: a method it does not admit still
//! refuses, and an initialize with no string version still refuses as before.

use serde_json::{json, Value};

use super::World;

/// What the harness says to its broker, one frame a line: a greeting asking
/// 2025-11-25, its listing, a method 2025-06-18 does not admit, a greeting
/// asking MB3's own version, one with no string version, and one call of
/// `resolve`.
const FRAMES: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"harness","version":"1"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}
{"jsonrpc":"2.0","id":4,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}
{"jsonrpc":"2.0","id":5,"method":"initialize","params":{"protocolVersion":20251125,"capabilities":{}}}
{"jsonrpc":"2.0","id":"c","method":"tools/call","params":{"name":"resolve","arguments":{}}}
"#;

/// The broker's error answer to the harness's request `id`.
fn refused(id: u8, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

/// A harness asking 2025-11-25 is answered 2025-06-18, as one asking
/// 2025-06-18 is; it lists exactly the granted tools and completes a call
/// whose response is retained and opens through `inspect`; a method
/// 2025-06-18 does not admit refuses, and so does a version that is no
/// string, each with its exact cause.
#[test]
fn a_harness_asking_another_version_is_answered_mb3s_and_completes_a_granted_call() {
    let Some(world) = World::new() else {
        return;
    };
    world.frames(FRAMES);
    let ended = world.completes();
    assert_eq!(world.broker_code(), Some(0));
    let greeting = |id: u8| {
        json!({"jsonrpc": "2.0", "id": id, "result": {"protocolVersion": "2025-06-18",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "cap-library-docs", "version": "1.4.2"}}})
    };
    let tools = json!([{"name": "resolve", "inputSchema": {"type": "object"}},
                       {"name": "read", "inputSchema": {"type": "object"}}]);
    let heard = std::fs::read_to_string(world.bin("heard")).unwrap();
    let heard: Vec<Value> = heard
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let said = json!({"content": [{"type": "text", "text": "resolved"}]});
    assert_eq!(
        heard,
        [
            greeting(1),
            json!({"jsonrpc": "2.0", "id": 2, "result": {"tools": tools}}),
            refused(3, -32601, "MCP method is not admitted by this broker"),
            greeting(4),
            refused(5, -32600, "MCP tool call request is invalid"),
            json!({"jsonrpc": "2.0", "id": "c", "result": said}),
        ]
    );
    let view = world.opened(&ended.run);
    let digest = brokkr_core::canonical::sha256_bytes(
        br#"{"result":{"content":[{"text":"resolved","type":"text"}]}}"#,
    );
    assert_eq!(
        (&view["response"], &view["call"]["state"]),
        (&json!({"result": said}), &json!("succeeded")),
        "{view}"
    );
    assert_eq!(
        world.calls(&ended.run),
        [("resolve".into(), "succeeded".into(), Some(digest))]
    );
}
