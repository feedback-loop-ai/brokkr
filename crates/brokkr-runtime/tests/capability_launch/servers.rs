//! The engine's complete MCP server set at the final check of a real
//! compiled launch (decision 0065 slice two, U7a; MB1, SI2). The parent's
//! check reads a sealed broker set from the serving inputs, as the serving
//! edge will once a driver carries one (U7d); until then the engine's own
//! composition carries the box's hands alone, so a broker set sealed beside
//! it refuses before any spawn, and no grant is enabled here.

use brokkr_protocol::native_controls::{SealedBroker, SealedServers, SealedServing};

use super::*;

/// The brokers `carried` seals beside the box's hands, none for any other
/// server set.
pub(super) fn sealed_brokers(carried: &SealedServing) -> &[SealedBroker] {
    match &carried.isolation.servers {
        SealedServers::Brokered(brokers) => brokers,
        SealedServers::Empty | SealedServers::Hands | SealedServers::NoModelSurface => &[],
    }
}

/// A boxed Claude seat and its Codex fallback, compiled with no grant: the
/// hands set they were sealed with checks as composed, and the same launch
/// sealed with two brokers beside the hands refuses, Claude with the
/// final-set cause and Codex, whose measured fragment carries `brokkr`
/// alone, as hands that are not the engine's.
#[test]
fn a_compiled_launch_is_checked_against_its_complete_sealed_server_set() {
    let operator = Operator::new();
    let ungranted = operator.context(json!({}));
    let bundle = (operator.compile(&ungranted, Boundary::Namespace, None, None)).unwrap();
    let brokers: Vec<SealedBroker> = ["library-docs", "issue-tracker"]
        .map(|capability| SealedBroker {
            capability: capability.into(),
            plan: format!("/state/plans/{capability}.json"),
            plan_digest: "a".repeat(64),
            removals: Vec::new(),
        })
        .to_vec();
    let final_form = |problem: &str| {
        format!(
            "the final command of harness 'codex' {problem}; a complete command is parsed back \
             before its spawn and must express exactly the capability state its sealed plan \
             records, so it is refused rather than spawned (operator ruling 2 of 2026-09-23; \
             design D6)"
        )
    };
    for (candidate, refusal) in [
        (
            0,
            "provider 'claude' final MCP configuration is not the engine's sealed server set"
                .to_string(),
        ),
        (
            1,
            final_form("is sealed with hands that are not the engine's workspace hands"),
        ),
    ] {
        let facts = &bundle.sites["chain"];
        let (spawn, _) = sealed(&bundle, "chain", candidate);
        let mut carried = brokkr_runtime::engine::serving_inputs(
            facts.chain.get(candidate),
            Some(facts),
            spawn.class,
            bundle.boundary,
        )
        .unwrap();
        assert_eq!(carried.isolation.servers, SealedServers::Hands);
        assert_eq!(
            checked_against(&bundle, "chain", candidate, &carried),
            sealed_launch(&bundle, "chain", candidate),
            "{candidate}"
        );
        carried.isolation.servers = SealedServers::Brokered(brokers.clone());
        assert_eq!(
            checked_against(&bundle, "chain", candidate, &carried),
            Err(refusal),
            "{candidate}"
        );
    }
}
