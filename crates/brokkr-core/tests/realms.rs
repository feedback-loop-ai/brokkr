//! The grant read by the version of the map that wrote it (decision 0065
//! slice two, SC2 and CR1): `forge.realms/v8` reserves `retain` as the
//! realm's retention veto, and v6 and v7 keep the same spelling as the
//! dialect's restriction.

use brokkr_core::realms::{GrantError, GrantRetention, RealmMap, RealmsError, Unusable, SCHEMA_V8};
use serde_json::{json, Value};

/// A one-realm map under `schema` granting `capabilities`.
fn world(schema: &str, capabilities: Value) -> Value {
    json!({"schema": schema, "journal": ".forge/forge.db", "realms": [
        {"name": "private", "path": "repo", "default_branch": "main",
         "capabilities": capabilities}]})
}

fn parsed(map: &Value) -> Result<RealmMap, RealmsError> {
    RealmMap::parse("realms.json", &map.to_string()).map(|(map, _)| map)
}

/// Under v6 and v7 a written `retain: false` is the dialect's restriction,
/// exactly as before the word was reserved; every grant round-trips as
/// written, and absent lists stay apart from empty ones.
#[test]
fn an_older_grant_keeps_retain_as_a_restriction_and_round_trips_exactly() {
    for schema in ["forge.realms/v6", "forge.realms/v7"] {
        let written = json!({
            "library-docs": {"dialect": "docs-mcp", "tools": [], "offices": ["researcher"],
                             "retain": false},
            "web-search": {"dialect": "codex-native-search"}
        });
        let map = parsed(&world(schema, written.clone())).unwrap();
        let grants = &map.realms[0].grants;
        let docs = &grants["library-docs"];
        assert_eq!(docs.retention, GrantRetention::Unreserved, "{schema}");
        assert_eq!(
            Value::Object(docs.restrictions.clone()),
            json!({"retain": false})
        );
        assert_eq!(
            (docs.tools.clone(), docs.offices.clone()),
            (Some(vec![]), Some(vec!["researcher".to_string()]))
        );
        let search = &grants["web-search"];
        assert_eq!((search.tools.clone(), search.offices.clone()), (None, None));
        assert!(search.restrictions.is_empty());
        for (name, grant) in grants {
            assert_eq!(grant.value(), written[name], "{schema} {name}");
        }
    }
}

/// Under v8 `retain: false` is the veto: it never reaches the dialect as a
/// restriction, and the grant's identity keeps it. Leaving it out inherits.
#[test]
fn a_v8_grant_reads_retain_false_as_the_veto_and_omission_as_inherit() {
    let written = json!({
        "library-docs": {"dialect": "docs-mcp", "retain": false,
                         "allow": {"libraries": ["serde"]}},
        "web-search": {"dialect": "codex-native-search", "offices": ["researcher"]}
    });
    let map = parsed(&world(SCHEMA_V8, written.clone())).unwrap();
    let grants = &map.realms[0].grants;
    let (docs, search) = (&grants["library-docs"], &grants["web-search"]);
    assert_eq!(
        (docs.retention, search.retention),
        (GrantRetention::Veto, GrantRetention::Inherit)
    );
    assert_eq!(
        Value::Object(docs.restrictions.clone()),
        json!({"allow": {"libraries": ["serde"]}})
    );
    assert!(search.restrictions.is_empty());
    assert_eq!(docs.value(), written["library-docs"]);
    assert_eq!(search.value(), written["web-search"]);
}

/// The realm may veto retention, never require it: under v8 every value
/// but `false` is refused naming the realm and the capability, `null`
/// included, rather than coerced.
#[test]
fn a_v8_retain_other_than_false_is_refused_by_realm_and_capability() {
    for value in [
        json!(true),
        Value::Null,
        json!({}),
        json!("false"),
        json!(0),
    ] {
        let map = world(
            SCHEMA_V8,
            json!({"library-docs": {"dialect": "docs-mcp", "retain": value.clone()}}),
        );
        let refused = parsed(&map).unwrap_err();
        assert_eq!(
            refused,
            RealmsError::Invalid {
                path: "realms.json".into(),
                problem: Unusable::Grant(GrantError::Retain {
                    realm: "private".into(),
                    capability: "library-docs".into(),
                }),
            },
            "{value}"
        );
    }
    // The one place this refusal's rendered words are pinned.
    let refused = parsed(&world(
        SCHEMA_V8,
        json!({"web-search": {"dialect": "d", "retain": true}}),
    ))
    .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "realms.json is not a usable realms map: realm 'private' capability 'web-search': retain \
         must be false when present; the realm may veto retention, never require it"
    );
}

/// v8 keeps every v7 word: the provisional offices read back as written,
/// a map writing no `retain` reads as the same map under v7 but for the
/// version and what that version reserves, and a key written twice is
/// still refused.
#[test]
fn a_v8_map_keeps_the_v7_provisional_offices_and_its_refusals() {
    let mut map = world(SCHEMA_V8, json!({"web-search": {"dialect": "d"}}));
    map["provisional_offices"] = json!(["researcher", "review-correctness"]);
    let as_v8 = parsed(&map).unwrap();
    assert_eq!(
        as_v8.provisional_offices,
        ["researcher", "review-correctness"]
    );
    map["schema"] = json!("forge.realms/v7");
    let as_v7 = parsed(&map).unwrap();
    let grant = |map: &RealmMap| map.realms[0].grants["web-search"].clone();
    assert_eq!(grant(&as_v8).retention, GrantRetention::Inherit);
    assert_eq!(grant(&as_v7).retention, GrantRetention::Unreserved);
    assert_eq!(grant(&as_v8).value(), grant(&as_v7).value());
    assert_eq!(
        (as_v8.realms[0].name.as_str(), as_v8.journal.as_str()),
        (as_v7.realms[0].name.as_str(), as_v7.journal.as_str())
    );
    let twice = r#"{"schema":"forge.realms/v8","realms":[{"name":"private","path":"repo","default_branch":"main","capabilities":{"web-search":{"dialect":"a"},"web-search":{"dialect":"b"}}}],"journal":"forge.db"}"#;
    assert_eq!(
        RealmMap::parse("realms.json", twice).unwrap_err(),
        RealmsError::Malformed {
            path: "realms.json".into(),
            detail: "key 'web-search' is written twice at line 1 column 168".into(),
        }
    );
}
