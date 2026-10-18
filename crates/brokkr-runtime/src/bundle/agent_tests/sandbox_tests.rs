//! Design D5.3's admission of a typed sandbox class across a model chain:
//! `admit_local_sandbox` judges the links in ascending order and refuses at
//! the first that fails, so a chain whose every link fails names link 1.

use super::*;

#[test]
fn a_chain_whose_every_link_fails_admission_names_its_first_link() {
    let fixture = AgentFixture::new();
    let mut adapter = codex();
    adapter["models"]["sol"] = json!("gpt-6-sol");
    fixture.write("adapters/codex.json", adapter);
    // Both links dispatch codex, whose boxed fragment expresses read-only:
    // each fails the same exact-match check.
    fixture.write(
        "agents/boxed.json",
        boxed_agent(&["astra", "sol"], json!({"sandbox": "workspace-write"})),
    );
    let mut config = fixture.config();
    config["seats"]["work"] = json!({"results": ["complete"], "agent": "boxed"});
    assert_eq!(
        outcome(fixture.compile(config)),
        "bundle: seat 'work' link 1 requests 'tools.sandbox' 'workspace-write', but the \
         `hands.workspace` fragment the engine selects for provider 'codex' under the \
         `namespace` boundary expresses 'read-only'; a fragment is neither called narrower nor \
         clamped, the typed class must match it exactly — refused (design D5.3)"
    );
}
