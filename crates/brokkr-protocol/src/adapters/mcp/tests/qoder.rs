//! The engine's set a qoder launch is served (#1178), read from hand-sealed
//! serving inputs as the rest of this suite seals them.

use super::*;
use crate::adapters::mcp::qoder::qoder_set_at;
use crate::native_controls::mcp::qoder::Served;

/// `input` with the reach and artifact root the engine writes beside
/// serving inputs that seal the box's hands under `stands`.
fn hands_input(stands: SealedBoundary) -> Value {
    let mut input =
        json!({"workdir": "/w", SERVING_INPUTS: hands_under(stands), ARTIFACT_ROOT: "/a"});
    input[SEALED_REACH] = serde_json::to_value(reach()).unwrap();
    input
}

/// A driver run by hand, and hands the boundary does not box, are served
/// the empty document with no grant; boxed hands are served their own
/// server and the grant of its one tool.
#[test]
fn a_qoder_launch_is_served_the_empty_set_or_the_boxed_hands() {
    let brokkr = Path::new("/bin/brokkr");
    let empty = Served {
        document: json!({"mcpServers": {}}),
        grants: Vec::new(),
    };
    let by_hand = json!({"workdir": "/w"});
    assert_eq!(qoder_set_at(&by_hand, "/w", brokkr), Ok(empty.clone()));
    let unboxed = hands_input(SealedBoundary::Harness);
    assert_eq!(qoder_set_at(&unboxed, "/w", brokkr), Ok(empty));
    let boxed = qoder_set_at(&hands_input(SealedBoundary::Namespace), "/w", brokkr).unwrap();
    let spec = crate::hands::HandsSpec::default();
    let args = crate::hands::serve_args(Path::new("/w"), Path::new("/a"), &reach(), &spec);
    assert_eq!(
        boxed,
        Served {
            document: json!({"mcpServers": {"brokkr": {"command": "/bin/brokkr", "args": args}}}),
            grants: vec!["mcp__brokkr__workspace".to_string()],
        }
    );
}

/// Unreadable inputs, a brokered set and hands no document value binds
/// exactly each refuse with their own cause.
#[test]
fn a_qoder_set_it_cannot_serve_as_sealed_refuses() {
    let brokkr = Path::new("/bin/brokkr");
    let unread = json!({"workdir": "/w", SERVING_INPUTS: {"unknown": true}});
    assert_eq!(
        qoder_set_at(&unread, "/w", brokkr),
        Err(McpRefusal::Unreadable)
    );
    let mut brokered = hands_input(SealedBoundary::Namespace);
    brokered[SERVING_INPUTS]["isolation"]["servers"] = json!({"brokered": [
        {"capability": "web", "plan": "/p/web.json", "plan_digest": "a".repeat(64),
         "removals": []},
    ]});
    let refused = qoder_set_at(&brokered, "/w", brokkr).unwrap_err();
    assert_eq!(refused, unmeasured_cause("qoder", "brokers"));
    assert_eq!(
        refused.at_launch(),
        "refusing to invoke the agent CLI: provider 'qoder' has no measured strict MCP \
         configuration for 'brokers'"
    );
    use std::os::unix::ffi::OsStrExt;
    let lossy = Path::new(std::ffi::OsStr::from_bytes(b"/bin/\xffbrokkr"));
    let boxed = hands_input(SealedBoundary::Namespace);
    assert_eq!(
        qoder_set_at(&boxed, "/w", lossy),
        Err(McpRefusal::NotSealed { provider: "qoder" })
    );
}
