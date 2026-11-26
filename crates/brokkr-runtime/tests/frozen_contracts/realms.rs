use super::{contract, published, titled};

/// Decision 0065 slice two (SC2, CR1): v8 is v7 plus one reserved grant
/// key, `retain`, whose only legal value is `false`. v7's bytes are pinned
/// above; under v7 the same spelling, any value, stays a restriction.
#[test]
fn the_v8_realm_schema_reserves_only_the_retention_veto() {
    use serde_json::json;
    assert_eq!(
        titled("contracts/realms.v8.schema.json"),
        "Forge realms map v8"
    );
    let [schema, v7] = published(["realms.v8", "realms.v7"]);
    let mut carried = schema["properties"].clone();
    let grant =
        &mut carried["realms"]["items"]["properties"]["capabilities"]["additionalProperties"];
    let added = grant["properties"]
        .as_object_mut()
        .unwrap()
        .remove("retain");
    assert_eq!(
        added.map(|retain| retain["const"].clone()),
        Some(json!(false))
    );
    let mut earlier = v7["properties"].clone();
    earlier["schema"] = json!({"const": "forge.realms/v8"});
    let description = "/realms/items/properties/capabilities/additionalProperties/description";
    *earlier.pointer_mut(description).unwrap() = carried.pointer(description).unwrap().clone();
    assert_eq!(carried, earlier, "v8 moved a property v7 defines");
    for key in ["required", "additionalProperties", "type"] {
        assert_eq!(schema[key], v7[key], "{key} moved between v7 and v8");
    }

    let validator = jsonschema::draft7::new(&schema).unwrap();
    let mut map = json!({"schema": "forge.realms/v8", "journal": "forge.db",
        "provisional_offices": ["researcher"], "realms": [
        {"name": "app", "path": ".", "default_branch": "main",
         "capabilities": {"library-docs": {"dialect": "docs-mcp"}}}]});
    assert!(validator.is_valid(&map), "a v8 grant leaving retain out");
    let retain = "/realms/0/capabilities/library-docs/retain";
    map["realms"][0]["capabilities"]["library-docs"]["retain"] = json!(false);
    assert!(validator.is_valid(&map), "a v8 veto");
    for value in [
        json!(true),
        json!(null),
        json!("false"),
        json!(0),
        json!({}),
    ] {
        *map.pointer_mut(retain).unwrap() = value.clone();
        assert!(!validator.is_valid(&map), "the v8 schema admitted {value}");
        map["schema"] = json!("forge.realms/v7");
        assert!(
            contract("contracts/realms.v7.schema.json").is_valid(&map),
            "v7 refused the restriction {value}"
        );
        map["schema"] = json!("forge.realms/v8");
    }
    *map.pointer_mut(retain).unwrap() = json!(false);
    assert!(validator.is_valid(&map), "the label's v8 control");
    map["schema"] = json!("forge.realms/v7");
    assert!(!validator.is_valid(&map), "v7 label under the v8 schema");
}

#[test]
fn the_v9_realm_schema_adds_the_closed_per_provider_grant() {
    use serde_json::json;

    assert_eq!(
        titled("contracts/realms.v9.schema.json"),
        "Forge realms map v9"
    );
    let [schema, v8] = published(["realms.v9", "realms.v8"]);
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let older = jsonschema::draft7::new(&v8).unwrap();
    let mut map = json!({"schema": "forge.realms/v9", "journal": "forge.db",
        "provisional_offices": ["researcher"], "realms": [
        {"name": "app", "path": ".", "default_branch": "main",
         "capabilities": {"library-docs": {"dialects": {"claude": "docs-mcp", "codex": "docs-native"},
             "offices": ["researcher"], "retain": false}}}]});
    assert!(validator.is_valid(&map), "a scoped per-provider grant");
    let grant = "/realms/0/capabilities/library-docs";
    *map.pointer_mut(grant).unwrap() = json!({"dialects":{"claude":"docs-mcp"}});
    assert!(validator.is_valid(&map), "scope and veto are optional");
    for refused in [
        json!({"dialect":"docs-mcp","dialects":{"claude":"docs-mcp"}}),
        json!({"offices":[]}),
        json!({"dialects":{}}),
        json!({"dialects":[]}),
        json!({"dialects":{"Claude":"docs-mcp"}}),
        json!({"dialects":{"claude":"Docs MCP"}}),
        json!({"dialects":{"claude":"docs-mcp"},"tools":[]}),
        json!({"dialects":{"claude":"docs-mcp"},"allow":{}}),
        json!({"dialects":{"claude":"docs-mcp"},"retain":true}),
    ] {
        *map.pointer_mut(grant).unwrap() = refused.clone();
        assert!(!validator.is_valid(&map), "v9 admitted {refused}");
    }
    *map.pointer_mut(grant).unwrap() = json!({
        "dialect":"docs-mcp", "tools":["read"], "allow":{}, "retain":false
    });
    assert!(validator.is_valid(&map), "the v8 single-dialect shape");
    assert!(!older.is_valid(&map), "v8 refuses the v9 label");
    map["schema"] = json!("forge.realms/v8");
    assert!(older.is_valid(&map), "the v8 label control");
    map["realms"][0]["capabilities"]["library-docs"]["dialects"] = json!({"limit": 3});
    assert!(older.is_valid(&map), "dialects is a v8 restriction");
}
