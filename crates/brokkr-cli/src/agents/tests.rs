use super::*;
use brokkr_runtime::agents::{report as walk, ResolveError};
use std::path::PathBuf;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn library_root() -> PathBuf {
    workspace().join("agents")
}

fn adapters_root() -> PathBuf {
    workspace().join("adapters")
}

/// AC-10: `list` lists every shipped agent, and a broken definition
/// prints a warning line without aborting — the contract `brokkr recipes
/// list` set, and the reason the library is one file per agent.
#[test]
fn list_lists_every_agent_and_warns_without_aborting() {
    assert!(list(&library_root()).is_ok());

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("agents");
    std::fs::create_dir_all(root.join("charters")).unwrap();
    std::fs::write(root.join("charters/c.md"), "# c\n").unwrap();
    std::fs::write(root.join("broken.json"), "{").unwrap();
    std::fs::write(
        root.join("good.json"),
        serde_json::to_vec(&json!({
            "description": "a good agent",
            "charter": "charters/c.md",
            "models": ["opus"],
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(list(&root).is_ok(), "one broken file never aborts");

    // A missing library is an error for `list`: the operator asked for a
    // library that is not there, and silence would be the wrong answer.
    assert!(list(&dir.path().join("absent")).is_err());
}

/// AC-10: `show` prints the definition as written plus the per-entry
/// resolution — machine readable without a flag, because it cannot drift
/// from the data it prints.
#[test]
fn show_prints_the_definition_and_its_per_entry_resolution() {
    assert!(show("chief-architect", &library_root(), &adapters_root()).is_ok());

    let walked = walk(
        &Library::load(&library_root()).unwrap(),
        &Adapters::load(&adapters_root()).unwrap(),
        &Availability::unspecified(),
        "chief-architect",
    )
    .unwrap();
    let resolution = resolution_value(&walked);
    assert_eq!(resolution["chosen"]["index"], 0);
    assert_eq!(resolution["chosen"]["model"], "fable");
    assert_eq!(resolution["chain"][0]["status"], "ok");
    assert_eq!(resolution["chain"][0]["provider"], "claude");
    assert_eq!(resolution["chain"][0]["presence"], "unknown");
    // Decision 0045: the first step down every claude-first chain crosses
    // the vendor line, so the chief's second link resolves on codex.
    assert_eq!(resolution["chain"][1]["provider"], "codex");
    assert_eq!(resolution["chain"][1]["model"], "astra");
    assert_eq!(resolution["chain"].as_array().unwrap().len(), 3);
}

/// An unknown name errors naming the known set, so the next command is
/// obvious rather than guessable.
#[test]
fn show_names_the_known_set_for_an_unknown_agent() {
    let error = show("nobody", &library_root(), &adapters_root()).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("agent 'nobody' is not in the library"),
        "{message}"
    );
    assert!(message.contains("chief-architect"), "{message}");

    let dir = tempfile::tempdir().unwrap();
    assert!(show("x", &dir.path().join("absent"), &adapters_root()).is_err());
    assert!(show(
        "chief-architect",
        &library_root(),
        &dir.path().join("absent")
    )
    .is_err());
}

/// A blocked or unmapped entry is REPORTED rather than thrown, so a
/// reader sees the whole chain and can act on the link that is wrong.
#[test]
fn the_resolution_block_reports_unmapped_and_blocked_entries() {
    let dir = tempfile::tempdir().unwrap();
    let library = dir.path().join("agents");
    let adapters = dir.path().join("adapters");
    std::fs::create_dir_all(library.join("charters")).unwrap();
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(library.join("charters/c.md"), "# c\n").unwrap();
    std::fs::write(
        library.join("tester.json"),
        serde_json::to_vec(&json!({
            "description": "a test agent",
            "charter": "charters/c.md",
            "models": ["blocked", "unmapped"],
            // Pinned, so the gap this entry reports is the one under
            // test: an unpinned effort is a capability gap too
            // (decision 0035 ruling 5), and leaving it out here would
            // shadow the tool_permissions gap the assertion is for.
            "efforts": {"blocked": "medium"},
            "tools": {"allow": ["cargo"], "mcp": [{"server": "github", "optional": true}]},
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        adapters.join("plain.json"),
        serde_json::to_vec(&json!({
            "provider": "plain",
            "binary": "plain",
            "driver": ["plain"],
            "models": {"blocked": "plain/1"},
            "model_flag": "-m",
            "efforts": ["low", "medium", "high"],
            "effort_flag": "--effort",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }))
        .unwrap(),
    )
    .unwrap();

    let walked = walk(
        &Library::load(&library).unwrap(),
        &Adapters::load(&adapters).unwrap(),
        &Availability::unspecified(),
        "tester",
    )
    .unwrap();
    let resolution = resolution_value(&walked);
    assert_eq!(resolution["chain"][0]["status"], "blocked");
    assert!(resolution["chain"][0]["problem"]
        .as_str()
        .unwrap()
        .contains("tool_permissions unsupported"));
    assert_eq!(resolution["chain"][1]["status"], "unmapped");
    assert!(resolution["chain"][1]["provider"].is_null());
    // Even blocked, `show` prints the whole picture rather than refusing.
    assert!(show("tester", &library, &adapters).is_ok());
}

/// A chain whose links are all unavailable has no choice to report, and
/// says so rather than naming one.
#[test]
fn a_chain_with_no_available_link_reports_no_choice() {
    let library = Library::load(&library_root()).unwrap();
    let adapters = Adapters::load(&adapters_root()).unwrap();
    let mut availability = Availability::unspecified();
    for provider in ["claude", "lanetally", "codex", "dsh", "exec"] {
        availability.record(provider, Presence::Unavailable);
    }
    let walked = walk(&library, &adapters, &availability, "chief-architect").unwrap();
    assert!(resolution_value(&walked)["chosen"].is_null());
    assert_eq!(entry_value(&walked.entries[0])["presence"], "unavailable");
    availability.record("claude", Presence::Available);
    let walked = walk(&library, &adapters, &availability, "chief-architect").unwrap();
    assert_eq!(entry_value(&walked.entries[0])["presence"], "available");
}

#[test]
fn presence_has_a_word_for_every_arm() {
    assert_eq!(presence_word(Presence::Available), "available");
    assert_eq!(presence_word(Presence::Unavailable), "unavailable");
    assert_eq!(presence_word(Presence::Unknown), "unknown");
}

/// The error surface is the resolver's own: `show` does not restate it.
#[test]
fn show_forwards_the_resolvers_own_message() {
    let error = ResolveError::UnknownAgent {
        name: "nobody".into(),
        known: "a, b".into(),
    };
    assert!(error.to_string().contains("known agents: a, b"));
}
