//! Task 46.6 (U9b1d; MB1, SI2) through the real binary: a Claude seat holding
//! brokered grants is launched allowing the box's hands tool and then each
//! broker by its one served name, `mcp__cap-<capability>`, once, in sealed
//! order, as the harness's own argv shows; a seat holding none is launched
//! with the hands tool alone and a document serving the hands alone. A
//! headless seat cannot approve a tool, so without the allowance no brokered
//! call on Claude could run (the controller's real-harness check, R3). The
//! final check's refusal of an altered allowance is the protocol suite's
//! `each_departure_from_the_brokered_set_refuses_at_the_final_boundary`.

use serde_json::Value;

use super::{answering, World};

/// How many `--allowedTools` the seat's harness was last handed, and the
/// value of the first.
fn allowance(world: &World) -> (usize, String) {
    let argv = std::fs::read_to_string(world.bin("argv")).unwrap();
    let argv: Vec<&str> = argv.lines().collect();
    let flags: Vec<usize> = (0..argv.len())
        .filter(|at| argv[*at] == "--allowedTools")
        .collect();
    (flags.len(), argv[flags[0] + 1].to_string())
}

/// The servers the seat's harness was last handed, by name.
fn servers(world: &World) -> Vec<String> {
    let config = std::fs::read_to_string(world.bin("config.json")).unwrap();
    let config: Value = serde_json::from_str(&config).unwrap();
    let servers = config["mcpServers"].as_object().unwrap().keys();
    servers.cloned().collect()
}

/// One brokered grant: the hands tool, then `mcp__cap-library-docs`.
#[test]
fn a_brokered_claude_seat_allows_its_hands_tool_and_its_broker_by_name() {
    let Some(world) = World::new() else {
        return;
    };
    world.completes();
    assert_eq!(servers(&world), ["brokkr", "cap-library-docs"]);
    assert_eq!(
        allowance(&world),
        (
            1,
            "mcp__brokkr__workspace,mcp__cap-library-docs".to_string()
        )
    );
}

/// Two brokered grants: both servers, once each, in sealed order.
#[test]
fn two_brokered_grants_are_each_allowed_once_in_sealed_order() {
    let Some(world) = World::new() else {
        return;
    };
    super::boxes::second_server(&world, &answering(""));
    world.completes();
    assert_eq!(
        servers(&world),
        ["brokkr", "cap-library-docs", "cap-more-docs"]
    );
    assert_eq!(
        allowance(&world),
        (
            1,
            "mcp__brokkr__workspace,mcp__cap-library-docs,mcp__cap-more-docs".to_string()
        )
    );
}

/// A seat of an office holding no grant, in the same realm, is launched as
/// before: the hands alone served and the hands tool alone allowed.
#[test]
fn a_seat_holding_no_brokered_grant_allows_the_hands_tool_alone() {
    let Some(world) = World::new() else {
        return;
    };
    world.plain_office("plain");
    let at = world.repo.join(".brokkr/bundle.json");
    let mut bundle: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    bundle["seats"]["work"]["agent"] = "plain".into();
    world.write("bundle.json", &bundle);
    world.completes();
    assert_eq!(servers(&world), ["brokkr"]);
    assert_eq!(allowance(&world), (1, "mcp__brokkr__workspace".to_string()));
    let argv = std::fs::read_to_string(world.bin("argv")).unwrap();
    assert!(!argv.contains("cap-"), "{argv}");
}
