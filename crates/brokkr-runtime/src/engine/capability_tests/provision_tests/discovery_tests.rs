//! U7d (SD3, CC2): the servers the engine names the serving candidate
//! (U7c) are what the driver renders into its capability contract, read
//! through the protocol's own renderer: selection and rendering together,
//! each bound by its own removal.

use brokkr_protocol::adapters::{render_prompt, AdapterKind};

use super::*;

/// The guidance naming the fixture's `library-docs` server and its tools.
const SERVED: &str = "\nYou reach `library-docs` through its server `cap-library-docs`, whose \
tools you call as `mcp__cap-library-docs__lookup`, `mcp__cap-library-docs__fetch`.";

/// The guidance's line for Codex's measured `tool_search`.
const LOAD: &str = "\nIf one of these tools is not listed, use `tool_search` to load it before \
calling it.";

/// The capability contract's fixed last sentence, after which the
/// guidance is rendered.
const CLOSES: &str = "what you hold, or the result contract.";

/// Codex deferred discovery from the selected attempt alone, one driver
/// input reused across selections as a retried attempt reuses it: the
/// primary whose adapter declares `tool_search` is told the server, its
/// exact tools and `tool_search`, beside a hands notice naming the same
/// tool; a fallback declaring no discovery tool hears none; a fallback
/// whose set brokers nothing, and a dropped wanted holding, advertise no
/// tool. The prompt names no plan, store or server launch.
#[test]
fn the_selected_servers_render_inside_the_serving_candidates_capability_contract() {
    let mut fixture = Fixture::inline();
    let server = fixture.server();
    fixture.hold("library-docs", admitted("library-docs", &[&server], true));
    let current = state(Some("work"), Cursor::RequestEffect);
    let mut input = (fixture.engine).seat_input(&current, "work", "effect");
    let input = input.as_mut().unwrap();
    let mut rendered = |engine: &Engine, link: Option<&Candidate>| {
        engine.marks().site("work", false, link, input);
        let prompt = render_prompt(input, AdapterKind::Codex).unwrap();
        let guidance = prompt.rsplit_once(CLOSES).unwrap().1.to_string();
        (prompt, guidance)
    };
    let primary = Candidate {
        hands_notice: Some(discovering()),
        ..codex_primary()
    };
    let (told, guidance) = rendered(&fixture.engine, Some(&primary));
    assert_eq!(guidance, format!("{SERVED}{LOAD}\n"));
    assert!(told.contains("If it is not listed, use `tool_search` to load it"));
    let run = fixture.run_dir();
    for absent in [
        server.as_str(),
        run.to_str().unwrap(),
        ".local/state/brokkr",
    ] {
        assert!(!told.contains(absent), "{absent}");
    }
    let (_, fallback) = rendered(&fixture.engine, Some(&codex_primary()));
    assert_eq!(fallback, format!("{SERVED}\n"));
    rendered(&fixture.engine, Some(&primary));
    let (_, brokers_nothing) = rendered(&fixture.engine, Some(&dsh_fallback()));
    assert_eq!(brokers_nothing, "\n");
    rendered(&fixture.engine, Some(&primary));
    let site = fixture.engine.bundle.sites.get_mut("work").unwrap();
    let outcome = &mut site.capabilities.as_mut().unwrap().outcomes[0];
    outcome.held.remove("library-docs");
    let (dropped, guidance) = rendered(&fixture.engine, Some(&primary));
    assert_eq!(guidance, "\n");
    assert!(!dropped.contains("mcp__cap-library-docs"));
}
