//! CC1 and SC4 at compile (decision 0065 slice two, U4d): a native tool is
//! attributed only to the holding this candidate selected, by its exact
//! name; a tool two held capabilities claim, or a held name seat-record v6
//! could carry only truncated, refuses the compile whatever the strength.

use super::binding::WHO;
use super::*;
use crate::capabilities::attribution::{Attributed, Attribution, Refusal};

/// `test-native` knowing each `(capability, tools)` power under the
/// capability's own key, switched ON and OFF by its own argument.
fn knowing(powers: &[(&str, &[&str])]) -> NativeInventory {
    let known: Map<String, Value> = powers
        .iter()
        .map(|(capability, tools)| {
            let power = json!({
                "capability": capability, "tools": tools,
                "on": {"argv": [format!("--{capability}-on")]},
                "off": {"argv": [format!("--{capability}-off")]},
                "restrictions": {"unsupported": "no transport"},
                "evidence": {"source": "a test", "scope": "a test", "limitations": []}
            });
            (capability.to_string(), power)
        })
        .collect();
    NativeInventory::parse("adapter 'test-native'", Some(&json!({ "known": known }))).unwrap()
}

fn held_through(capability: &str, dialect: &str) -> Attributed {
    Attributed {
        capability: capability.into(),
        dialect: dialect.into(),
    }
}

/// SC4's refusal at the research seat, for a holding of `capability`
/// through `dialect`.
fn unrepresentable(capability: &str, dialect: &str) -> String {
    format!(
        "{WHO}: capability call identity cannot be represented by seat-record v6 (capability \
         '{capability}' through dialect '{dialect}')"
    )
}

#[test]
fn a_held_capability_is_switched_on_and_is_fully_attributable() {
    let root = cq1_root();
    let granted = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native", "offices": ["researcher"]}}),
    );
    let native = switchable();
    let outcome = granted
        .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
        .unwrap();
    let holding = &outcome.held["web-search"];
    assert_eq!(holding.classes, ["reads", "egress"]);
    assert_eq!(holding.dialect, "search-native");
    assert_eq!(holding.tools, ["lookup", "search"]);
    assert_eq!(
        holding.dialect_sha256,
        granted.dialects["web-search"].sha256
    );
    assert_eq!(
        holding.definition_sha256,
        granted.definitions.get("web-search").unwrap().sha256
    );
    // ON, and no OFF beside it.
    assert_eq!(argv_of(&outcome), ["--search-on"]);
    assert!(outcome.not_held.is_empty() && outcome.notices.is_empty());
    let manifest = outcome.manifest();
    assert_eq!(manifest["provider"], "test-native");
    assert_eq!(manifest["model"], "tn-1");
    assert_eq!(
        manifest["native"],
        json!({"inventory": "known", "declaration": "d1ge57",
                                          "on": ["web-search"], "off": []})
    );
    assert_eq!(manifest["held"]["web-search"]["restrictions"], json!({}));
    assert_eq!(
        outcome.prompt(),
        json!({"held": {"web-search": {"tools": ["lookup", "search"]}}, "not_held": {}})
    );
    // The grant is scoped: another office in the same realm holds nothing.
    let implementer = SiteAsks::of("implement", None, None).unwrap();
    let other = granted.resolve(&implementer, &serving(&native)).unwrap();
    assert!(other.held.is_empty());
    assert_eq!(argv_of(&other), ["--search-off"]);
    assert_eq!(
        other.not_held["web-search"],
        "provider 'test-native' has it natively, the realm does not grant it to this seat, \
         and it is switched off"
    );
}

/// The exact selected tool, and nothing the inventory merely knows: the
/// realm narrowed `search` away, so it is the harness's tool and no
/// holding's, and no near name is read as `lookup`.
#[test]
fn a_native_tool_is_attributed_by_its_exact_name_to_the_selected_holding_alone() {
    let root = cq1_root();
    let narrowed = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native", "tools": ["lookup"]}}),
    );
    let selecting = test_native(
        json!({"selection": {"include": ["lookup", "search"], "allow": [], "deny": []}}),
        json!({"selection": {"include": [], "allow": [], "deny": ["lookup", "search"]}}),
        json!({"unsupported": "no transport"}),
    );
    let wanting = asks(json!({"web-search": "wants"}));
    let outcome = narrowed.resolve(&wanting, &serving(&selecting)).unwrap();
    let index = narrowed.attribution("test-native", &outcome.held).unwrap();
    assert_eq!(
        index.of("lookup"),
        Some(&held_through("web-search", "search-native"))
    );
    assert_eq!(selecting.capability_of("search"), Some("web-search"));
    assert_eq!(index.of("search"), None);
    for near in ["look", "ookup", "Lookup", "lookup2", "lookup ", ""] {
        assert_eq!(index.of(near), None, "{near:?}");
    }
    // A candidate that lost the want holds nothing, so attributes nothing,
    // whatever its neighbour holds through the same inventory.
    let claude = Serving {
        provider: "claude",
        ..serving(&selecting)
    };
    let dropped = narrowed.resolve(&wanting, &claude).unwrap();
    assert_eq!(
        narrowed.attribution("claude", &dropped.held),
        Ok(Attribution::default())
    );
}

/// CC1's ambiguous reverse mapping: neither holding is chosen, and a
/// want is refused as a requirement is. A key the inventory shares but
/// the seat does not hold makes no ambiguity.
#[test]
fn a_tool_two_held_capabilities_claim_refuses_the_compile_and_neither_is_chosen() {
    let root = cq1_root();
    define(root.path(), "web-fetch", &["reads", "egress"]);
    dialect(
        root.path(),
        &native_dialect("fetch-native", "web-fetch", &["lookup"]),
    );
    let granted = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native"},
               "web-fetch": {"dialect": "fetch-native"}}),
    );
    let native = knowing(&[
        ("web-search", &["lookup", "search"]),
        ("web-fetch", &["lookup"]),
    ]);
    for strength in ["requires", "wants"] {
        let both = asks(json!({"web-search": strength, "web-fetch": strength}));
        assert_eq!(
            granted.resolve(&both, &serving(&native)).unwrap_err(),
            format!(
                "{WHO}: tool 'lookup' maps to more than one held capability for provider \
                 'test-native'"
            )
        );
    }
    let alone = |capability: &str| {
        let site = asks(json!({ capability: "requires" }));
        granted.resolve(&site, &serving(&native)).unwrap().held
    };
    let (search, fetch) = (alone("web-search"), alone("web-fetch"));
    let index = granted.attribution("test-native", &search).unwrap();
    assert_eq!(
        index.of("lookup"),
        Some(&held_through("web-search", "search-native"))
    );
    let both: BTreeMap<String, Holding> = search.into_iter().chain(fetch).collect();
    assert_eq!(
        granted.attribution("test-native", &both),
        Err(Refusal::Ambiguous {
            tool: "lookup".into(),
            provider: "test-native".into()
        })
    );
}

/// SC4 at compile: a held tool beyond 256 bytes or outside v5's tool
/// vocabulary, or a dialect or capability name beyond 128 bytes, refuses
/// whatever the strength, naming the holding and never the tool. A valid
/// 256-byte name is held whole, and two names sharing an 80-character
/// prefix stay two tools.
#[test]
fn a_held_name_v6_cannot_carry_whole_refuses_and_a_long_valid_one_stays_distinct() {
    let root = cq1_root();
    let prefix = "a".repeat(80);
    let (first, second, longest) = (
        format!("{prefix}.first"),
        format!("{prefix}.second"),
        "t".repeat(256),
    );
    let tools = [first.as_str(), second.as_str(), longest.as_str()];
    let seat = |capability: &str, dialect_name: &str, tools: &[&str], strength: &str| {
        dialect(
            root.path(),
            &native_dialect(dialect_name, capability, tools),
        );
        let granted = authority(
            root.path(),
            json!({ capability: {"dialect": dialect_name} }),
        );
        let native = knowing(&[(capability, tools)]);
        let held = granted
            .resolve(&asks(json!({ capability: strength })), &serving(&native))
            .map(|outcome| outcome.held);
        held.map(|held| granted.attribution("test-native", &held).unwrap())
    };
    let index = seat("web-search", "search-long", &tools, "requires").unwrap();
    for tool in tools {
        assert_eq!(
            index.of(tool),
            Some(&held_through("web-search", "search-long"))
        );
    }
    assert_eq!(index.of(&prefix), None);
    let named = format!("search-{}", "n".repeat(121));
    assert_eq!(named.len(), 128);
    assert!(seat("web-search", &named, &["lookup"], "requires").is_ok());

    let overlong = "t".repeat(257);
    for tool in [
        overlong.as_str(),
        "web search",
        "Bash(git:*)",
        ".hidden",
        "wéb",
    ] {
        for strength in ["requires", "wants"] {
            assert_eq!(
                seat("web-search", "search-odd", &[tool], strength).unwrap_err(),
                unrepresentable("web-search", "search-odd"),
                "{tool:?}"
            );
        }
    }
    let dialect_name = format!("{named}x");
    assert_eq!(
        seat("web-search", &dialect_name, &["lookup"], "wants").unwrap_err(),
        unrepresentable("web-search", &dialect_name)
    );
    let capability = format!("web-{}", "c".repeat(125));
    define(root.path(), &capability, &["reads"]);
    assert_eq!(
        seat(&capability, "search-wide", &["lookup"], "requires").unwrap_err(),
        unrepresentable(&capability, "search-wide")
    );
}

/// An `mcp` tool is told apart by its `cap-` server (CC1), so the native
/// index never claims it, even where its name is a native tool's.
#[test]
fn an_mcp_holding_never_enters_the_native_index() {
    let root = cq1_root();
    let granted = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native"}}),
    );
    let native = switchable();
    let mut held = granted
        .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
        .unwrap()
        .held;
    held.get_mut("web-search").unwrap().implementation = Implementation::Mcp {
        server: "cap-web-search".into(),
        connection: Connection::Stdio(vec!["docs-server".into()]),
        version: "1.0.0".into(),
        secrets: Vec::new(),
    };
    assert_eq!(
        granted.attribution("test-native", &held),
        Ok(Attribution::default())
    );
}
