//! Release preparation uses the existing delivery law and house configuration.

use std::path::PathBuf;

use brokkr_runtime::{Bundle, SeatBody};
use serde_json::Value;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn compile(name: &str) -> Bundle {
    let root = workspace();
    Bundle::compile_with(
        &root.join("recipes").join(name),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap()
}

#[test]
fn release_keeps_the_delivery_policy_and_deterministic_gates() {
    let release = compile("release");
    let fast = compile("fast");
    assert_eq!(release.machine.initial, fast.machine.initial);
    assert_eq!(release.machine.phases, fast.machine.phases);
    assert_eq!(release.machine.terminal, fast.machine.terminal);
    assert_eq!(release.protected_phase, "review");
    // Compare the complete parsed rules, including predicates and return bounds.
    assert_eq!(
        format!("{:?}", release.machine.rules),
        format!("{:?}", fast.machine.rules)
    );
    assert!(!release.seats["implement"].has_gate);
    assert!(release.hands["implement"].network);
    assert!(release.seats["review"].has_gate);
    for phase in ["verify", "ship"] {
        assert!(release.seats[phase].has_gate);
        let SeatBody::Single {
            command,
            candidates,
            ..
        } = &release.seats[phase].body
        else {
            panic!("{phase} must remain deterministic exec")
        };
        let SeatBody::Single {
            command: inherited, ..
        } = &fast.seats[phase].body
        else {
            panic!("fast's {phase} must remain exec")
        };
        assert_eq!(command, inherited);
        assert!(candidates.is_empty());
        assert_eq!(release.seats[phase].results, fast.seats[phase].results);
        assert!(release.hands.contains_key(phase));
        assert!(!release.hands[phase].network);
    }
    let SeatBody::Single { role_path, .. } = &release.seats["implement"].body else {
        panic!("the release manager is a library office")
    };
    assert!(role_path.ends_with("charters/release-manager.md"));
}

#[test]
fn release_configuration_is_in_the_realms_existing_house_pin() {
    let root = workspace();
    let map: Value =
        serde_json::from_slice(&std::fs::read(root.join("realms.json")).unwrap()).unwrap();
    let house =
        std::fs::read_to_string(root.join(map["realms"][0]["house"].as_str().unwrap())).unwrap();
    let configuration = house
        .split_once("## Release configuration")
        .unwrap()
        .1
        .split_once("```json\n")
        .unwrap()
        .1
        .split_once("\n```")
        .unwrap()
        .0;
    let configuration: Value = serde_json::from_str(configuration).unwrap();
    for field in [
        "version_files",
        "documentation",
        "validation",
        "organization_profiles",
        "channels",
        "extensions",
    ] {
        assert!(
            !configuration[field].as_array().unwrap().is_empty(),
            "{field}"
        );
    }
    for path in configuration["version_files"]
        .as_array()
        .unwrap()
        .iter()
        .chain(configuration["documentation"].as_array().unwrap())
    {
        assert!(root.join(path.as_str().unwrap()).exists(), "{path}");
    }
    let charter = std::fs::read_to_string(root.join("agents/charters/release-manager.md")).unwrap();
    assert!(!charter.contains("feedback-loop-ai"));
    assert!(!charter.contains("Cargo.toml"));
    assert!(!charter.contains("profile/README.md"));
    assert!(charter.contains("House\nrules"));
}

#[test]
fn release_manager_uses_the_operators_medium_effort_chain_in_the_box() {
    let agent: Value = serde_json::from_slice(
        &std::fs::read(workspace().join("agents/release-manager.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(agent["models"], serde_json::json!(["opus", "astra"]));
    assert_eq!(
        agent["efforts"],
        serde_json::json!({"opus": "medium", "astra": "medium"})
    );
    assert!(agent.get("tools").is_none());
    assert!(agent.get("bindings").is_none());
    assert_eq!(agent["hands"]["network"], true);
    let binds = agent["hands"]["binds"].as_array().unwrap();
    let registry = binds
        .iter()
        .find(|bind| bind["path"] == "~/.cargo")
        .unwrap();
    assert_eq!(registry["mode"], "overlay");
    assert_eq!(
        registry["mask"],
        serde_json::json!(["credentials.toml", "credentials"])
    );
    let release = compile("release");
    let SeatBody::Single { candidates, .. } = &release.seats["implement"].body else {
        panic!("release preparation is one office")
    };
    assert_eq!(candidates.len(), 2);
    for candidate in candidates {
        assert!(candidate.argv.iter().any(|arg| arg == "medium"));
    }
}
