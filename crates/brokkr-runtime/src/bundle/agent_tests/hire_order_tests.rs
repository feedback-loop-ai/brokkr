//! The order of a candidate's refusals in `agents::compose`: a candidate
//! that fails on several counts names the model pin before the effort pin,
//! and the effort pin before the agent's tools.

use super::*;

#[test]
fn a_candidate_failing_several_counts_names_the_model_pin_then_the_effort_then_the_tools() {
    let fixture = AgentFixture::new();
    // The model cannot be told, and the worker's effort 'high' cannot be
    // either: the model pin is named.
    let mut adapter = claude();
    adapter["model_flag"] = json!("unsupported");
    adapter["effort_flag"] = json!("unsupported");
    adapter["efforts"] = json!([]);
    fixture.write("adapters/claude.json", adapter);
    let unpinned = outcome(fixture.compile(fixture.config()));
    // The model is pinned, but the effort is outside the vocabulary and the
    // tool list names a tool the adapter does not map: the effort is named.
    fixture.write("adapters/claude.json", claude());
    let written = std::fs::read(fixture.library().join("worker.json")).unwrap();
    let mut worker: Value = serde_json::from_slice(&written).unwrap();
    worker["efforts"] = json!({"opus": "xhigh"});
    worker["tools"] = json!({"allow": ["cargo", "git"]});
    fixture.write("agents/worker.json", worker);
    let unknown = outcome(fixture.compile(fixture.config()));
    let refused = |cause: &str| {
        format!(
            "bundle: seat 'work': agent 'worker' cannot be served by provider 'claude' on model \
             'opus': the provider {cause}. A capability the provider cannot express fails \
             compilation here rather than degrading silently at run time"
        )
    };
    assert_eq!(
        [unpinned, unknown],
        [
            refused(
                "declares model_flag unsupported, so the chosen model could not be pinned and \
                 the provider's own default would run"
            ),
            refused("declares no effort 'xhigh'; its vocabulary is low, medium, high"),
        ]
    );
}
