//! The engine's set an agy launch is served (#1200), read through the one
//! admission every built-in harness passes, from serving inputs sealed by
//! hand as the rest of this suite seals them (#1178 round three's cases).

use super::*;
use crate::adapters::mcp::agy::agy_set_at;
use crate::native_controls::mcp::agy::Served;

/// What `input` gives an agy launch run by `brokkr`: the admission's
/// refusal, or the set read from the intent it checked.
fn agy_served(input: &Value, brokkr: &str) -> Result<Served, McpRefusal> {
    let edge = Edge::new(input);
    let isolated = isolated("agy", &edge, &[], false)?;
    agy_set_at(&edge, &isolated, "/w", Path::new(brokkr))
}

/// Serving inputs sealing the box's hands under `stands`, with the reach
/// and artifact root the engine writes beside them.
fn sealed_hands(stands: SealedBoundary) -> Value {
    let mut input = json!({"workdir": "/w", ARTIFACT_ROOT: "/a"});
    input[SERVING_INPUTS] = hands_under(stands);
    input[SEALED_REACH] = serde_json::to_value(reach()).unwrap();
    input
}

/// The empty set: the measured empty document and no rule.
fn nothing_served() -> Result<Served, McpRefusal> {
    let document = json!({"mcpServers": {}});
    Ok(Served {
        document,
        rules: Vec::new(),
    })
}

/// Agy joins the one admission: its shape is its own, a measured cold
/// assessment admits, and no grammar reads an agy serving command back.
#[test]
fn agy_is_a_serving_shape_whose_measured_cold_assessment_admits() {
    assert_eq!(ServingShape::of("agy"), Some(ServingShape::Agy));
    let isolation = SealedIsolation {
        cold: SealedAssessment::Measured,
        ..SealedIsolation::default()
    };
    assert_eq!(isolation.admits("agy", false), Ok(()));
    let (serving, composed) = (
        crate::native_controls::Serving::default(),
        crate::native_controls::Composed {
            extra: Vec::new(),
            managed: Vec::new(),
        },
    );
    let command = crate::adapters::serving_command(ServingShape::Agy, &serving, &composed);
    assert_eq!(
        command,
        Err("an agy command, which no modelled grammar reads back")
    );
}

/// Every launch the admission serves no hands is served the empty set: a
/// driver run by hand, unboxed hands, an empty intent beside them, and the
/// hands dispatch seals measured on no shape while the fence stands.
#[test]
fn an_agy_launch_served_no_hands_is_served_the_empty_document() {
    let unboxed = sealed_hands(SealedBoundary::Harness);
    let inputs = [
        json!({"workdir": "/w"}),
        unboxed.clone(),
        intended(unboxed, "empty", cold_only()),
        sealed_hands(SealedBoundary::Namespace),
    ];
    for input in inputs {
        assert_eq!(
            agy_served(&input, "/bin/brokkr"),
            nothing_served(),
            "{input}"
        );
    }
}

/// Hands the admission passed are the one server, and their one tool is
/// granted as Agy spells a rule (`iso-07`).
#[test]
fn admitted_hands_are_served_with_their_mcp_rule() {
    let input = intended(
        sealed_hands(SealedBoundary::Namespace),
        "hands",
        cold_only(),
    );
    let spec = crate::hands::HandsSpec::default();
    let args = crate::hands::serve_args(Path::new("/w"), Path::new("/a"), &reach(), &spec);
    let server = json!({"command": "/opt/brokkr", "args": args});
    let served = agy_served(&input, "/opt/brokkr").unwrap();
    assert_eq!(served.document, json!({"mcpServers": {"brokkr": server}}));
    assert_eq!(served.rules, ["mcp(brokkr/workspace)"]);
}

/// Past the lifted fence an unmeasured or unsupported cold shape refuses;
/// a set the inputs do not declare refuses under either fence, and so do
/// a brokered set, unreadable inputs, and hands no document value binds.
#[test]
fn an_agy_launch_the_admission_refuses_is_served_no_set() {
    let boxed = || sealed_hands(SealedBoundary::Namespace);
    let measured = || json!("measured");
    let cold = |assessment: Value| [assessment, measured(), measured()];
    let unsupported = json!({"unsupported": PROJECT_REASON});
    let mut cases = vec![
        (
            intended(boxed(), "hands", cold(json!("unmeasured"))),
            unmeasured_cause("agy", "cold"),
        ),
        (
            intended(boxed(), "hands", cold(unsupported)),
            unsupported_cause("agy", PROJECT_REASON),
        ),
        (
            json!({"workdir": "/w", SERVING_INPUTS: {"unknown": true}}),
            McpRefusal::Unreadable,
        ),
    ];
    for fence in ["lifted", "standing"] {
        let not_sealed = McpRefusal::NotSealed { provider: "agy" };
        cases.push((fenced(boxed(), "empty", cold_only(), fence), not_sealed));
        let mut brokered = fenced(boxed(), "hands", cold_only(), fence);
        brokered[SERVING_INPUTS]["isolation"]["servers"] = json!({"brokered": [
            {"capability": "web", "plan": "/p/web.json", "plan_digest": "b".repeat(64),
             "removals": []},
        ]});
        cases.push((brokered, unmeasured_cause("agy", "brokers")));
    }
    for (input, refusal) in cases {
        assert_eq!(agy_served(&input, "/bin/brokkr"), Err(refusal), "{input}");
    }
    use std::os::unix::ffi::OsStrExt;
    let measured = intended(boxed(), "hands", cold_only());
    let edge = Edge::new(&measured);
    let admitted = isolated("agy", &edge, &[], false).unwrap();
    let lossy = Path::new(std::ffi::OsStr::from_bytes(b"/opt/\xfebrokkr"));
    assert_eq!(
        agy_set_at(&edge, &admitted, "/w", lossy),
        Err(McpRefusal::NotSealed { provider: "agy" })
    );
}
