//! The unanimous panel invocation shared by terminal and storage tests.
use super::*;

pub(super) fn unanimous(
    engine: &mut Engine,
    at: PanelAt<'_>,
    members: &[PanelMember],
    input: &Value,
    seconds: u64,
) -> Result<(), EngineError> {
    engine.execute_panel(
        &at,
        members,
        Aggregate::UnanimousPass,
        input,
        std::time::Duration::from_secs(seconds),
        &Selection::new(),
    )
}

/// Decision 0103 ruling 1(a): a panel drives each member as its own
/// invocation, so each report carries its own typed refusal, with its own
/// provider and route — never one report's refusal standing for the panel.
#[test]
fn a_panels_members_each_carry_their_own_refusal() {
    let (_dir, mut engine) =
        crate::engine::tests::engine(crate::engine::tests::single_body(vec!["unused".into()]));
    let members = [
        crate::engine::tests::member("first", refusal_command("claude", Some("anthropic"))),
        crate::engine::tests::member("second", refusal_command("codex", Some("openai"))),
    ];
    let input = crate::engine::tests::panel_input(&["first", "second"]);
    let runs = engine.member_runs(
        "attempt",
        "work",
        &members,
        &input["members"],
        &input,
        &input["context"],
        &Selection::new(),
        "",
        false,
    );
    let reports = engine
        .run_panel(
            "effect",
            "attempt",
            &runs,
            std::time::Duration::from_secs(5),
            "",
        )
        .unwrap();
    assert_eq!(reports.len(), 2, "{reports:?}");
    let refusal = |at: usize| {
        reports[at]
            .1
            .refusal
            .as_ref()
            .unwrap_or_else(|| panic!("member {at} carries no refusal: {reports:?}"))
    };
    assert_eq!(refusal(0).provider(), "claude");
    assert_eq!(refusal(0).route(), Some("anthropic"));
    assert_eq!(refusal(1).provider(), "codex");
    assert_eq!(refusal(1).route(), Some("openai"));
    assert!(reports.iter().all(|(_, report)| !report.accepted));
}

fn refusal_command(provider: &str, route: Option<&str>) -> Vec<String> {
    let capabilities = serde_json::json!({
        "proto": brokkr_protocol::PROTO, "msg_id": "c", "type": "capabilities",
        "driver": "test", "version": "1", "supports": []
    });
    let result = serde_json::json!({
        "proto": brokkr_protocol::PROTO, "msg_id": "r", "type": "result",
        "effect_id": "effect", "attempt_id": "attempt", "status": "failed",
        "refusal": {
            "class": "rate_limit", "provider": provider, "route": route,
            "words": format!("rate_limit: {provider}")
        }
    });
    vec![
        "sh".into(),
        "-c".into(),
        format!(
            "read -r line; printf '%s\\n' '{capabilities}'; read -r line; printf '%s\\n' '{result}'"
        ),
    ]
}
