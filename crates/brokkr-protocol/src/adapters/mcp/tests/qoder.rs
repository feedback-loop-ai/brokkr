//! The engine's set a qoder launch is served (#1178), read from hand-sealed
//! serving inputs as the rest of this suite seals them, through the one
//! admission every built-in harness passes.

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

/// What `input` gives a qoder launch admitted cold and run by `brokkr`:
/// the admission's refusal, or the set read from the intent it checked.
fn set_at(input: &Value, brokkr: &Path) -> Result<Served, McpRefusal> {
    let edge = Edge::new(input);
    let admitted = isolated("qoder", &edge, &[], false)?;
    qoder_set_at(&edge, &admitted, "/w", brokkr)
}

/// [`set_at`] run by `/bin/brokkr`.
fn set_of(input: &Value) -> Result<Served, McpRefusal> {
    set_at(input, Path::new("/bin/brokkr"))
}

/// The empty set: the measured empty document and no grant.
fn empty() -> Served {
    Served {
        document: json!({"mcpServers": {}}),
        grants: Vec::new(),
    }
}

/// Qoder joins the one admission (T1): its shape is its own, a measured
/// qoder assessment is admitted for the fragment the engine builds, and no
/// grammar reads a qoder serving command back.
#[test]
fn qoder_is_a_serving_shape_whose_measured_assessment_is_admitted() {
    assert_eq!(ServingShape::of("qoder"), Some(ServingShape::Qoder));
    let measured = SealedIsolation {
        cold: SealedAssessment::Measured,
        ..SealedIsolation::default()
    };
    assert_eq!(measured.admit("qoder", Invocation::Cold), Ok(()));
    let composed = crate::native_controls::Composed {
        extra: Vec::new(),
        managed: Vec::new(),
    };
    let serving = crate::native_controls::Serving::default();
    assert_eq!(
        crate::adapters::serving_command(ServingShape::Qoder, &serving, &composed),
        Err("a qoder command, which no modelled grammar reads back")
    );
}

/// A driver run by hand, an empty intent and hands the boundary does not
/// box are all served the empty document with no grant: never ambient MCP,
/// never hands. Boxed hands the standing fence serves unmeasured refuse
/// (#1235): no launch with Qoder's own tools takes the box's place.
#[test]
fn a_qoder_launch_the_admission_serves_no_hands_is_served_the_empty_set() {
    assert_eq!(set_of(&json!({"workdir": "/w"})), Ok(empty()));
    let unboxed = hands_input(SealedBoundary::Harness);
    assert_eq!(set_of(&unboxed), Ok(empty()));
    assert_eq!(
        set_of(&intended(unboxed, "empty", cold_only())),
        Ok(empty())
    );
    // Dispatch's own inputs: the hands set, measured on no shape, standing.
    assert_eq!(
        set_of(&hands_input(SealedBoundary::Namespace)),
        Err(McpRefusal::Unboxed { provider: "qoder" })
    );
}

/// The box's hands are served, with the grant of their one tool, only
/// where the admission passed with the hands set (round two's golden).
#[test]
fn a_measured_hands_intent_is_served_the_boxed_hands_and_their_grant() {
    let measured = intended(hands_input(SealedBoundary::Namespace), "hands", cold_only());
    let spec = crate::hands::HandsSpec::default();
    let args = crate::hands::serve_args(Path::new("/w"), Path::new("/a"), &reach(), &spec);
    assert_eq!(
        set_of(&measured),
        Ok(Served {
            document: json!({"mcpServers": {"brokkr": {"command": "/bin/brokkr", "args": args}}}),
            grants: vec!["mcp__brokkr__workspace".to_string()],
        })
    );
}

/// Past the lifted fence an unmeasured or unsupported cold shape refuses,
/// and a set the sealed inputs do not declare refuses whichever way the
/// fence stands: a sealed empty intent beside boxed hands is no empty set.
#[test]
fn a_qoder_launch_the_admission_refuses_is_served_no_set() {
    let boxed = hands_input(SealedBoundary::Namespace);
    let unmeasured = [json!("unmeasured"), json!("measured"), json!("measured")];
    assert_eq!(
        set_of(&intended(boxed.clone(), "hands", unmeasured)),
        Err(unmeasured_cause("qoder", "cold"))
    );
    let unsupported = [
        json!({"unsupported": PROJECT_REASON}),
        json!("measured"),
        json!("measured"),
    ];
    assert_eq!(
        set_of(&intended(boxed.clone(), "hands", unsupported)),
        Err(unsupported_cause("qoder", PROJECT_REASON))
    );
    for fence in ["lifted", "standing"] {
        let mismatched = fenced(boxed.clone(), "empty", cold_only(), fence);
        assert_eq!(
            set_of(&mismatched),
            Err(McpRefusal::NotSealed { provider: "qoder" }),
            "{fence}"
        );
    }
}

/// Unreadable inputs, a brokered set under either fence and hands no
/// document value binds exactly each refuse with their own cause.
#[test]
fn a_qoder_set_it_cannot_serve_as_sealed_refuses() {
    let unread = json!({"workdir": "/w", SERVING_INPUTS: {"unknown": true}});
    assert_eq!(set_of(&unread), Err(McpRefusal::Unreadable));
    for fence in ["lifted", "standing"] {
        let mut brokered = fenced(
            hands_input(SealedBoundary::Namespace),
            "hands",
            cold_only(),
            fence,
        );
        brokered[SERVING_INPUTS]["isolation"]["servers"] = json!({"brokered": [
            {"capability": "web", "plan": "/p/web.json", "plan_digest": "a".repeat(64),
             "removals": []},
        ]});
        let refused = set_of(&brokered).unwrap_err();
        assert_eq!(refused, unmeasured_cause("qoder", "brokers"), "{fence}");
        assert_eq!(
            refused.at_launch(),
            "refusing to invoke the agent CLI: provider 'qoder' has no measured strict MCP \
             configuration for 'brokers'"
        );
    }
    use std::os::unix::ffi::OsStrExt;
    let lossy = Path::new(std::ffi::OsStr::from_bytes(b"/bin/\xffbrokkr"));
    let measured = intended(hands_input(SealedBoundary::Namespace), "hands", cold_only());
    assert_eq!(
        set_at(&measured, lossy),
        Err(McpRefusal::NotSealed { provider: "qoder" })
    );
}
