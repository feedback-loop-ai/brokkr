use brokkr_core::realms::{
    GrantDialect, GrantError, GrantRetention, RealmMap, RealmsError, Unusable, SCHEMA_V8, SCHEMA_V9,
};
use serde_json::json;

use super::{parsed, world};

#[test]
fn v9_grants_round_trip_in_both_forms() {
    let written = json!({
        "web-search": {
            "dialects": {"claude": "search-claude", "test-native": "search-native"},
            "offices": ["researcher"],
            "retain": false
        },
        "library-docs": {
            "dialect": "docs-mcp",
            "tools": ["lookup"],
            "allow": {"libraries": ["serde"]}
        }
    });
    let map = parsed(&world(SCHEMA_V9, written.clone())).unwrap();
    let grants = &map.realms[0].grants;
    let search = &grants["web-search"];
    assert_eq!(
        search.dialect,
        GrantDialect::PerProvider(
            [
                ("claude".into(), "search-claude".into()),
                ("test-native".into(), "search-native".into()),
            ]
            .into()
        )
    );
    assert_eq!(search.retention, GrantRetention::Veto);
    assert_eq!(search.value(), written["web-search"]);
    let docs = &grants["library-docs"];
    let older = parsed(&world(
        SCHEMA_V8,
        json!({"library-docs": written["library-docs"]}),
    ))
    .unwrap();
    assert_eq!(docs, &older.realms[0].grants["library-docs"]);
    assert_eq!(docs.value(), written["library-docs"]);
}

#[test]
fn v8_keeps_dialects_as_a_restriction() {
    let dialects = json!({"claude": "search-claude"});
    let written = json!({"dialect": "search-native", "dialects": dialects});
    let map = parsed(&world(SCHEMA_V8, json!({"web-search": written}))).unwrap();
    let grant = &map.realms[0].grants["web-search"];
    assert_eq!(grant.dialect, GrantDialect::Single("search-native".into()));
    assert_eq!(grant.restrictions["dialects"], dialects);
    assert_eq!(grant.value(), written);
}

#[test]
fn v9_grant_refusals_are_typed_and_name_the_realm_and_capability() {
    let realm = || "private".to_string();
    let capability = || "web-search".to_string();
    let map = json!({"claude": "search-claude"});
    let cases = [
        (
            json!({"dialect": "search-native", "dialects": map}),
            GrantError::BothDialects { realm: realm(), capability: capability() },
            "with both 'dialect' and 'dialects'; write exactly one",
        ),
        (
            json!({}),
            GrantError::NoDialect { realm: realm(), capability: capability() },
            "with neither 'dialect' nor 'dialects'; write exactly one",
        ),
        (
            json!({"dialects": {}}),
            GrantError::Dialects { realm: realm(), capability: capability() },
            "with malformed 'dialects'; write a non-empty object from provider name to tool dialect name",
        ),
        (
            json!({"dialects": null}),
            GrantError::Dialects { realm: realm(), capability: capability() },
            "with malformed 'dialects'; write a non-empty object from provider name to tool dialect name",
        ),
        (
            json!({"dialects": {"Claude": "search-claude"}}),
            GrantError::DialectProvider { realm: realm(), capability: capability(), provider: "Claude".into() },
            "with invalid provider name 'Claude' in 'dialects'; use the realm-name grammar",
        ),
        (
            json!({"dialects": {"claude": "../search"}}),
            GrantError::ProviderDialect { realm: realm(), capability: capability(), provider: "claude".into() },
            "with an invalid tool dialect name for provider 'claude'; use the realm-name grammar",
        ),
    ];
    for (grant, error, clause) in cases {
        assert_refusal(grant, error, clause);
    }
    for key in ["tools", "allow"] {
        let mut grant = json!({"dialects": map});
        grant[key] = json!([]);
        assert_refusal(
            grant,
            GrantError::ProviderGrantKey { realm: realm(), capability: capability(), key: key.into() },
            &format!("with '{key}' beside 'dialects'; only 'offices' and 'retain' may accompany 'dialects'"),
        );
    }
}

fn assert_refusal(grant: serde_json::Value, error: GrantError, clause: &str) {
    let refused = parsed(&world(SCHEMA_V9, json!({"web-search": grant}))).unwrap_err();
    assert_eq!(
        refused,
        RealmsError::Invalid {
            path: "realms.json".into(),
            problem: Unusable::Grant(error),
        }
    );
    assert_eq!(
        refused.to_string(),
        format!("realms.json is not a usable realms map: realm 'private' grants capability 'web-search' {clause}")
    );
}

#[test]
fn v9_refuses_a_provider_key_written_twice() {
    let text = r#"{
        "schema":"forge.realms/v9",
        "realms":[{"name":"private","path":"repo","default_branch":"main","capabilities":{
            "web-search":{"dialects":{"claude":"a","claude":"b"}}}}],
        "journal":"forge.db"
    }"#;
    assert_eq!(
        RealmMap::parse("realms.json", text).unwrap_err(),
        RealmsError::Malformed {
            path: "realms.json".into(),
            detail: "key 'claude' is written twice at line 4 column 64".into(),
        }
    );
}
