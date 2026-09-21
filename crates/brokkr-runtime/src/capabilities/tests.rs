//! Decision 0065 slice one, proved at the module that rules it. Every
//! refusal and every notice is asserted WHOLE: an `is_err()` would pass
//! on the wrong refusal, and the wrong refusal is the bug.

use super::*;
use brokkr_core::realms::RealmMap;
use tempfile::TempDir;

fn write(root: &Path, relative: &str, value: &Value) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn define(root: &Path, name: &str, classes: &[&str]) {
    write(
        root,
        &Definition::source_of(name),
        &json!({"name": name, "classes": classes}),
    );
}

/// A provider-native dialect for the synthetic `test-native` provider.
fn native_dialect(name: &str, serves: &str, tools: &[&str]) -> Value {
    json!({
        "schema": "brokkr.tool-dialect/v1", "name": name, "serves": serves,
        "kind": "provider-native", "provider": "test-native", "adapter_key": serves,
        "tools": tools,
        "sends": {"description": "a query the model composes", "seat_composed": true}
    })
}

fn dialect(root: &Path, value: &Value) {
    let name = value["name"].as_str().unwrap();
    write(root, &ToolDialect::source_of(name), value);
}

/// The CQ1 world: `web-search` defined reads+egress, served by
/// `search-native`, whose schema admits `allow.hosts`.
fn cq1_root() -> TempDir {
    let root = TempDir::new().unwrap();
    define(root.path(), "web-search", &["reads", "egress"]);
    let mut search = native_dialect("search-native", "web-search", &["lookup", "search"]);
    search["classes"] = json!(["egress", "reads"]);
    search["restrictions"] = json!({
        "type": "object", "additionalProperties": false,
        "properties": {"allow": {"type": "object", "additionalProperties": false,
            "properties": {"hosts": {"type": "array", "items": {"type": "string"}}}}}
    });
    dialect(root.path(), &search);
    root
}

fn context(root: &Path, capabilities: Value) -> CapabilityContext {
    let map = json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
        {"name": "private", "path": "repo", "default_branch": "main",
         "capabilities": capabilities}]});
    let (map, _) = RealmMap::of("realms.json", map).unwrap();
    CapabilityContext {
        realm: "private".into(),
        grants: map.realms[0].grants.clone(),
        root: root.to_path_buf(),
    }
}

fn authority(root: &Path, capabilities: Value) -> Authority {
    Authority::load(context(root, capabilities)).unwrap()
}

fn refusal(root: &Path, capabilities: Value) -> String {
    Authority::load(context(root, capabilities)).unwrap_err()
}

/// `test-native`'s declaration: one `web-search` power with two tools.
fn test_native(on: Value, off: Value, restrictions: Value) -> NativeInventory {
    NativeInventory::parse(
        "adapter 'test-native'",
        Some(&json!({
            "known": {"web-search": {
                "capability": "web-search", "tools": ["lookup", "search"],
                "on": on, "off": off, "restrictions": restrictions,
                "evidence": {"source": "a test", "scope": "a test", "limitations": []},
                "authored": {"flags": ["--search"]}
            }},
            "selection": {
                "include": {"flag": "--tools", "separator": ","},
                "allow": {"flag": "--allow", "separator": ","},
                "deny": {"flag": "--deny", "separator": ","}
            }
        })),
    )
    .unwrap()
}

fn switchable() -> NativeInventory {
    test_native(
        json!({"argv": ["--search-on"]}),
        json!({"argv": ["--search-off"]}),
        json!({"unsupported": "no transport"}),
    )
}

fn asks(requests: Value) -> SiteAsks {
    let agent = parse_requests("agent 'researcher'", &requests).unwrap();
    SiteAsks::of("research", Some(("researcher", &agent)), None).unwrap()
}

fn serving(native: &NativeInventory) -> Serving<'_> {
    Serving {
        provider: "test-native",
        model: Some("tn-1"),
        native: Some((native, "d1ge57")),
        authored: &[],
    }
}

fn argv_of(outcome: &Outcome) -> Vec<String> {
    serde_json::from_value(outcome.controls()["argv"].clone()).unwrap()
}

// ------------------------------------------------------------ requests

#[test]
fn a_request_map_is_two_words_and_every_other_value_is_refused_not_dropped() {
    let requests = parse_requests(
        "agent 'researcher'",
        &json!({"web-fetch": "wants", "library-docs": "requires"}),
    )
    .unwrap();
    assert_eq!(requests["web-fetch"], Strength::Wants);
    assert_eq!(requests["library-docs"], Strength::Requires);
    assert_eq!(Strength::Wants.word(), "wants");
    assert_eq!(Strength::Requires.word(), "requires");
    assert_eq!(
        parse_requests("seat 'research'", &json!(["web-fetch"])).unwrap_err(),
        "seat 'research' 'capabilities' must be an object from capability name to \"requires\" \
         or \"wants\"; a request names no dialect, tool or grant, because only realms.json \
         grants (decision 0065 ruling 3)"
    );
    assert_eq!(
        parse_requests("agent 'researcher'", &json!({"Web Fetch": "wants"})).unwrap_err(),
        "agent 'researcher' requests a capability named 'Web Fetch'; a capability name is \
         lowercase letters, digits, '.', '_' and '-', starting with a letter or digit"
    );
    for value in [
        json!("required"),
        json!("optional"),
        json!(true),
        json!(null),
        json!({"dialect": "fetch-mcp", "tools": ["fetch"]}),
        json!({"classes": ["reads"]}),
    ] {
        assert_eq!(
            parse_requests("agent 'researcher'", &json!({"web-fetch": value})).unwrap_err(),
            format!(
                "agent 'researcher' requests capability 'web-fetch' as {value}; a request is \
                 \"requires\" or \"wants\" and nothing else — a dialect, a tool list, a class \
                 or a grant belongs to realms.json and the operator's definitions (decision \
                 0065 ruling 3)"
            )
        );
    }
}

#[test]
fn a_seat_inherits_subtracts_and_never_adds_or_re_rates() {
    let office =
        parse_requests("agent 'researcher'", &json!({"web-search": "wants", "library-docs": "requires"}))
            .unwrap();
    let agent = Some(("researcher", &office));
    // Omission inherits.
    let inherited = SiteAsks::of("research", agent, None).unwrap();
    assert_eq!(inherited.office, "researcher");
    assert_eq!(inherited.asks, office);
    assert!(inherited.subtracted.is_empty());
    // An explicit subset subtracts what it leaves out.
    let narrowed =
        SiteAsks::of("research", agent, Some(&json!({"library-docs": "requires"}))).unwrap();
    assert_eq!(narrowed.asks.len(), 1);
    assert_eq!(narrowed.subtracted, ["web-search"]);
    // An empty map subtracts everything — a requires included, deliberately.
    let none = SiteAsks::of("research", agent, Some(&json!({}))).unwrap();
    assert!(none.asks.is_empty());
    assert_eq!(none.subtracted, ["library-docs", "web-search"]);
    // An inline site's map IS its office's asks, and its office is its label.
    let inline = SiteAsks::of("verify:checks", None, Some(&json!({"web-fetch": "wants"}))).unwrap();
    assert_eq!(inline.office, "verify:checks");
    assert_eq!(inline.asks["web-fetch"], Strength::Wants);
    assert_eq!(SiteAsks::of("implement", None, None).unwrap().asks.len(), 0);

    assert_eq!(
        SiteAsks::of("research", agent, Some(&json!({"web-fetch": "wants"}))).unwrap_err(),
        "seat 'research' (office 'researcher') adds capability 'web-fetch', which its office \
         does not ask for; a seat may subtract from its office's asks and never add to them"
    );
    assert_eq!(
        SiteAsks::of("research", agent, Some(&json!({"library-docs": "wants"}))).unwrap_err(),
        "seat 'research' (office 'researcher') changes capability 'library-docs' from requires \
         to wants; a seat may subtract from its office's asks and never change their strength"
    );
    assert_eq!(
        SiteAsks::of("research", agent, Some(&json!({"web-search": "maybe"}))).unwrap_err(),
        "seat 'research' requests capability 'web-search' as \"maybe\"; a request is \
         \"requires\" or \"wants\" and nothing else — a dialect, a tool list, a class or a \
         grant belongs to realms.json and the operator's definitions (decision 0065 ruling 3)"
    );
}

// --------------------------------------------------------- definitions

#[test]
fn definitions_load_from_the_operators_directory_and_a_missing_one_is_empty() {
    let root = TempDir::new().unwrap();
    assert_eq!(Definitions::load(root.path()).unwrap(), Definitions::default());
    define(root.path(), "operator-library-docs", &["reads", "egress"]);
    std::fs::write(root.path().join("capabilities/README.md"), "not a definition").unwrap();
    let definitions = Definitions::load(root.path()).unwrap();
    let definition = definitions.get("operator-library-docs").unwrap();
    assert_eq!(definition.classes, ["reads", "egress"]);
    assert_eq!(definition.source, "capabilities/operator-library-docs.json");
    let bytes = std::fs::read(root.path().join(&definition.source)).unwrap();
    assert_eq!(definition.sha256, sha256_bytes(&bytes));
    assert!(definitions.get("web-search").is_none());
    assert_eq!(
        definitions.require("agent 'researcher'", "web-search").unwrap_err(),
        "agent 'researcher': capability 'web-search' has no abstract definition at \
         'capabilities/web-search.json' in the operator configuration; declare its classes \
         before requesting it"
    );
}

#[test]
fn an_invalid_definition_is_refused_naming_the_file_and_the_closed_vocabulary() {
    let tail = "; a definition is exactly a name and a non-empty set of classes from reads, \
                writes and egress";
    for (body, problem) in [
        (
            json!({"name": "c", "classes": []}),
            "at '/classes': it does not satisfy '/properties/classes/minItems'",
        ),
        (
            json!({"name": "c", "classes": ["reads", "reads"]}),
            "at '/classes': it does not satisfy '/properties/classes/uniqueItems'",
        ),
        (
            json!({"name": "c", "classes": ["network"]}),
            "at '/classes/0': it does not satisfy '/properties/classes/items/enum'",
        ),
        (
            json!({"name": "c", "classes": ["reads"], "dialect": "d"}),
            "at '': it does not satisfy '/additionalProperties'",
        ),
    ] {
        let root = TempDir::new().unwrap();
        write(root.path(), "capabilities/c.json", &body);
        assert_eq!(
            Definitions::load(root.path()).unwrap_err(),
            format!("capability definition 'capabilities/c.json' {problem}{tail}")
        );
    }
    // A name is its file's stem, in the grammar.
    for (stem, name) in [("web-search", "web-fetch"), ("Web Search", "Web Search")] {
        let root = TempDir::new().unwrap();
        write(
            root.path(),
            &Definition::source_of(stem),
            &json!({"name": name, "classes": ["reads"]}),
        );
        assert_eq!(
            Definitions::load(root.path()).unwrap_err(),
            format!(
                "capability definition 'capabilities/{stem}.json' names '{name}'; a \
                 definition's name is its file's stem, in the capability-name grammar"
            )
        );
    }
    // A key written twice is refused rather than read as its second copy.
    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(root.path().join("capabilities")).unwrap();
    std::fs::write(
        root.path().join("capabilities/c.json"),
        r#"{"name":"c","classes":["reads"],"classes":["reads","egress"]}"#,
    )
    .unwrap();
    assert_eq!(
        Definitions::load(root.path()).unwrap_err(),
        "'capabilities/c.json': key 'classes' is written twice at line 1 column 61"
    );
}

#[cfg(unix)]
#[test]
fn a_definition_cannot_be_read_from_outside_the_operators_directory() {
    let outside = TempDir::new().unwrap();
    define(outside.path(), "stolen", &["reads"]);
    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(root.path().join("capabilities")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("capabilities/stolen.json"),
        root.path().join("capabilities/stolen.json"),
    )
    .unwrap();
    assert_eq!(
        Definitions::load(root.path()).unwrap_err(),
        "'capabilities/stolen.json' resolves outside the operator configuration directory"
    );
    // A link to nothing, and a directory that only looks like a file.
    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(root.path().join("capabilities")).unwrap();
    std::os::unix::fs::symlink("/nonexistent", root.path().join("capabilities/gone.json")).unwrap();
    assert_eq!(
        Definitions::load(root.path()).unwrap_err(),
        "'capabilities/gone.json' names nothing readable"
    );
    let root = TempDir::new().unwrap();
    std::fs::create_dir_all(root.path().join("capabilities/dir.json")).unwrap();
    assert!(Definitions::load(root.path())
        .unwrap_err()
        .starts_with("'capabilities/dir.json': "));
}

// -------------------------------------------------------- tool dialects

fn mcp_dialect(name: &str, connection: Value) -> Value {
    json!({
        "schema": "brokkr.tool-dialect/v1", "name": name, "serves": "library-docs",
        "kind": "mcp", "connection": connection, "version": "1.4.2",
        "secrets": ["DOCS_TOKEN"], "tools": ["resolve", "read"], "retained": true,
        "sends": {"description": "a library name", "seat_composed": true}
    })
}

#[test]
fn every_kind_loads_as_data_and_nothing_is_executed_or_contacted() {
    let root = TempDir::new().unwrap();
    dialect(root.path(), &native_dialect("search-native", "web-search", &["lookup"]));
    let native = ToolDialect::load(root.path(), "search-native").unwrap();
    assert_eq!(
        native.kind,
        DialectKind::Native {
            provider: "test-native".into(),
            adapter_key: "web-search".into()
        }
    );
    assert_eq!(native.kind.word(), "provider-native");
    assert_eq!(native.tools, ["lookup"]);
    // An absent egress class is uncontracted, never local, and the class
    // annotation is absent rather than invented.
    assert_eq!(native.egress, "uncontracted");
    assert_eq!(native.classes, None);
    assert!(native.seat_composed);
    assert_eq!(native.source, "dialects/tools/search-native.json");
    // A binary that does not exist and a host that does not resolve: both
    // connection forms are read as data.
    for (name, connection) in [
        ("docs-stdio", json!({"argv": ["/nonexistent/docs-mcp", "--stdio"]})),
        ("docs-url", json!({"url": "https://docs.invalid/mcp"})),
    ] {
        dialect(root.path(), &mcp_dialect(name, connection));
        let loaded = ToolDialect::load(root.path(), name).unwrap();
        assert_eq!(loaded.kind, DialectKind::Mcp);
        assert_eq!(loaded.kind.word(), "mcp");
    }
    dialect(
        root.path(),
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "hands", "serves": "workspace",
                "kind": "hands", "tools": ["workspace"], "egress": "local",
                "sends": {"description": "nothing leaves the box", "seat_composed": false}}),
    );
    let hands = ToolDialect::load(root.path(), "hands").unwrap();
    assert_eq!(hands.kind, DialectKind::Hands);
    assert_eq!(hands.kind.word(), "hands");
    assert_eq!(hands.egress, "local");
    assert!(!hands.seat_composed);
    assert_eq!(
        ToolDialect::load(root.path(), "absent").unwrap_err(),
        "tool dialect 'absent' is not at 'dialects/tools/absent.json' in the operator \
         configuration"
    );
}

#[test]
fn a_dialect_outside_the_contract_is_refused_naming_the_file_and_the_field() {
    let outside = "tool dialect 'dialects/tools/d.json' is outside brokkr.tool-dialect/v1 ";
    let native = native_dialect("d", "web-search", &["lookup"]);
    let with = |mutate: &dyn Fn(&mut Value)| {
        let mut value = native.clone();
        mutate(&mut value);
        let root = TempDir::new().unwrap();
        dialect(root.path(), &value);
        ToolDialect::load(root.path(), "d").unwrap_err()
    };
    let at = |place: &str, clause: &str| {
        format!("{outside}at '{place}': it does not satisfy '{clause}'")
    };
    assert_eq!(
        with(&|d| d["kind"] = json!("plugin")),
        at("/kind", "/properties/kind/enum")
    );
    assert_eq!(
        with(&|d| {
            d.as_object_mut().unwrap().remove("serves");
        }),
        at("", "/required")
    );
    // The two vocabularies do not merge: `reads` is no egress class, and
    // `contracted` is no operation class.
    assert_eq!(
        with(&|d| d["egress"] = json!("reads")),
        at("/egress", "/properties/egress/enum")
    );
    assert_eq!(
        with(&|d| d["classes"] = json!(["contracted"])),
        at("/classes/0", "/properties/classes/items/enum")
    );
    // A missing binding and a mixture of kinds fail the kind's own branch.
    assert_eq!(
        with(&|d| {
            d.as_object_mut().unwrap().remove("adapter_key");
        }),
        at("", "/required")
    );
    assert_eq!(
        with(&|d| d["connection"] = json!({"url": "https://docs.invalid/mcp"})),
        at("", "/additionalProperties")
    );
    // The unsafe and incomplete MCP shapes, each naming its field — and
    // none echoing the credential somebody wrote by value.
    let mcp = |mutate: &dyn Fn(&mut Value)| {
        with(&|d| {
            *d = mcp_dialect("d", json!({"argv": ["/nonexistent/docs-mcp"]}));
            mutate(d);
        })
    };
    assert_eq!(
        mcp(&|d| d["connection"] = json!({"argv": ["x"], "url": "https://docs.invalid/mcp"})),
        at("/connection", "/properties/connection/maxProperties")
    );
    assert_eq!(
        mcp(&|d| d["connection"] = json!({"url": "https://user:hunter2@docs.invalid/mcp"})),
        at("/connection/url", "/properties/connection/properties/url/pattern")
    );
    assert_eq!(
        mcp(&|d| {
            d.as_object_mut().unwrap().remove("version");
        }),
        at("", "/required")
    );
    assert_eq!(
        mcp(&|d| {
            d.as_object_mut().unwrap().remove("tools");
        }),
        at("", "/required")
    );
    assert_eq!(mcp(&|d| d["token"] = json!("hunter2")), at("", "/additionalProperties"));
    assert_eq!(
        mcp(&|d| d["secrets"] = json!(["hunter2"])),
        at("/secrets/0", "/properties/secrets/items/pattern")
    );
    let root = TempDir::new().unwrap();
    write(
        root.path(),
        &ToolDialect::source_of("d"),
        &native_dialect("other", "web-search", &["lookup"]),
    );
    assert_eq!(
        ToolDialect::load(root.path(), "d").unwrap_err(),
        "tool dialect 'dialects/tools/d.json' names itself 'other'; a dialect's name is its \
         file's stem"
    );
}

#[test]
fn a_restriction_schema_stays_inside_its_file_and_off_the_engines_keys() {
    let schema_of = |restrictions: Value| {
        let mut value = native_dialect("d", "web-search", &["lookup"]);
        value["restrictions"] = restrictions;
        let root = TempDir::new().unwrap();
        dialect(root.path(), &value);
        ToolDialect::load(root.path(), "d")
    };
    assert_eq!(
        schema_of(json!({"properties": {"allow": {"$ref": "https://example.org/hosts.json"}}}))
            .unwrap_err(),
        "tool dialect 'dialects/tools/d.json' restriction schema references \
         'https://example.org/hosts.json', which is outside the dialect file; a restriction \
         schema is never fetched"
    );
    for restrictions in [
        json!({"properties": {"offices": {"type": "array"}}}),
        json!({"allOf": [{"required": ["tools"]}]}),
    ] {
        let problem = schema_of(restrictions).unwrap_err();
        assert!(
            problem.ends_with("', which is a key of the grant the engine owns"),
            "{problem}"
        );
    }
    assert!(schema_of(json!({"type": "no-such-type"}))
        .unwrap_err()
        .starts_with(
            "tool dialect 'dialects/tools/d.json' restriction schema is not valid draft-07: "
        ));
    // A local reference is the file's own, and loads.
    let local = schema_of(json!({
        "definitions": {"hosts": {"type": "array", "items": {"type": "string"}}},
        "type": "object", "properties": {"allow": {"$ref": "#/definitions/hosts"}}
    }))
    .unwrap();
    assert_eq!(local.restrictions["properties"]["allow"]["$ref"], "#/definitions/hosts");
    assert_eq!(
        violation(&json!({"type": "no-such-type"}), &json!({})).is_err(),
        true
    );
}

// ------------------------------------------------ native declarations

#[test]
fn a_native_declaration_states_both_halves_or_says_why_it_cannot() {
    assert_eq!(
        NativeInventory::parse("adapter 'older'", None).unwrap(),
        NativeInventory::Unmeasured(ABSENT_ASSESSMENT.to_string())
    );
    assert_eq!(
        NativeInventory::parse("adapter 'dsh'", Some(&json!({"unmeasured": "never probed"})))
            .unwrap(),
        NativeInventory::Unmeasured("never probed".to_string())
    );
    let NativeInventory::Known { known, selection } = switchable() else {
        panic!("a known inventory");
    };
    assert_eq!(known["web-search"].on, Disposition::Argv(vec!["--search-on".into()]));
    assert_eq!(known["web-search"].authored.flags, ["--search"]);
    assert_eq!(selection.unwrap().deny.flag, "--deny");

    let entry = |mutate: &dyn Fn(&mut Value)| {
        let mut raw = json!({"known": {"web-search": {
            "capability": "web-search", "tools": ["lookup"],
            "on": {"default": "measured on by default"}, "off": {"argv": ["--off"]},
            "restrictions": {"unsupported": "none"},
            "evidence": {"source": "s", "scope": "s", "limitations": []}}}});
        mutate(&mut raw);
        NativeInventory::parse("adapter 'test-native'", Some(&raw)).unwrap_err()
    };
    let at = "adapter 'test-native' 'native_capabilities' at '/known/web-search";
    // Each refusal names the adapter, the place in its declaration and
    // the clause it broke — the missing half, the missing reason, the
    // unexplained empty argv, the mixed variants.
    let broke = |problem: String, place: &str, keyword: &str| {
        assert!(
            problem.starts_with(&format!("{at}{place}': it does not satisfy '"))
                && problem.ends_with(&format!("/{keyword}'")),
            "{problem}"
        );
    };
    broke(
        entry(&|raw| {
            raw["known"]["web-search"].as_object_mut().unwrap().remove("off");
        }),
        "",
        "required",
    );
    broke(
        entry(&|raw| raw["known"]["web-search"]["off"] = json!({"unsupported": ""})),
        "/off/unsupported",
        "minLength",
    );
    broke(
        entry(&|raw| raw["known"]["web-search"]["off"] = json!({"argv": []})),
        "/off/argv",
        "minItems",
    );
    broke(
        entry(&|raw| raw["known"]["web-search"]["off"] =
            json!({"argv": ["--off"], "unsupported": "both"})),
        "/off",
        "maxProperties",
    );
    // Both inventories at once is neither.
    assert_eq!(
        entry(&|raw| raw["unmeasured"] = json!("and also known")),
        "adapter 'test-native' 'native_capabilities' at '': it does not satisfy '/oneOf'"
    );
    assert_eq!(
        entry(&|raw| raw["known"]["web-search"]["off"] =
            json!({"selection": {"include": [], "allow": [], "deny": ["lookup"]}})),
        "adapter 'test-native' 'native_capabilities' key 'web-search' uses a selection \
         control, but the adapter declares no 'selection' list flags to express it with"
    );
    for (template, slots) in [(json!(["--restrict"]), 0), (json!(["{restrictions_json}", "x{restrictions_json}"]), 2)] {
        assert_eq!(
            entry(&|raw| raw["known"]["web-search"]["restrictions"] = json!({"argv": template})),
            format!(
                "adapter 'test-native' 'native_capabilities' key 'web-search' restriction \
                 transport carries {slots} '{{restrictions_json}}' slots; it carries exactly one"
            )
        );
    }
}

// ------------------------------------------------------------ authority

#[test]
fn every_grant_is_judged_before_any_seat_and_a_bad_one_is_never_an_optional_drop() {
    let root = cq1_root();
    let grant = |extra: Value| {
        let mut grant = json!({"dialect": "search-native"});
        grant.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        json!({"web-search": grant})
    };
    assert_eq!(
        refusal(root.path(), json!({"web-fetch": {"dialect": "search-native"}})),
        "realm 'private': capability 'web-fetch' has no abstract definition at \
         'capabilities/web-fetch.json' in the operator configuration; declare its classes \
         before granting it"
    );
    assert_eq!(
        refusal(root.path(), json!({"web-search": {"dialect": "absent"}})),
        "realm 'private': capability 'web-search': tool dialect 'absent' is not at \
         'dialects/tools/absent.json' in the operator configuration"
    );
    define(root.path(), "web-fetch", &["reads", "egress"]);
    assert_eq!(
        refusal(root.path(), json!({"web-fetch": {"dialect": "search-native"}})),
        "realm 'private' grants capability 'web-fetch' through dialect 'search-native', which \
         serves 'web-search'"
    );
    assert_eq!(
        refusal(root.path(), grant(json!({"tools": ["lookup", "crawl"]}))),
        "realm 'private' grants capability 'web-search' tool 'crawl', which dialect \
         'search-native' does not name; its tools are [lookup, search]"
    );
    // CQ1: an invalid restriction refuses whatever any seat asks — even
    // for a grant scoped to no office at all.
    assert_eq!(
        refusal(root.path(), grant(json!({"offices": [], "allow": {"hosts": "sourceware.org"}}))),
        "realm 'private' grants capability 'web-search' through dialect 'search-native' with \
         an invalid restriction at '/allow/hosts': it does not satisfy \
         '/properties/allow/properties/hosts/type'"
    );
    assert_eq!(
        refusal(root.path(), grant(json!({"deny": {"hosts": []}}))),
        "realm 'private' grants capability 'web-search' through dialect 'search-native' with \
         an invalid restriction at '': it does not satisfy '/additionalProperties'"
    );
    // CQ2: a dialect's class annotation asserts equality and never overrides.
    let mut reads_only = native_dialect("reads-only", "web-search", &["lookup"]);
    reads_only["classes"] = json!(["reads"]);
    dialect(root.path(), &reads_only);
    assert_eq!(
        refusal(root.path(), json!({"web-search": {"dialect": "reads-only", "offices": []}})),
        "realm 'private': capability 'web-search' in dialect 'reads-only' declares classes \
         [reads], conflicting with abstract definition 'capabilities/web-search.json' classes \
         [reads, egress]"
    );
    // A valid grant loads, the dialect's annotation compared as a SET.
    let loaded = authority(root.path(), grant(json!({"allow": {"hosts": ["yaml.org"]}})));
    assert_eq!(loaded.dialects["web-search"].name, "search-native");
}

#[test]
fn an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved() {
    let root = cq1_root();
    define(root.path(), "library-docs", &["reads", "egress"]);
    define(root.path(), "workspace", &["reads", "writes"]);
    dialect(root.path(), &mcp_dialect("docs-mcp", json!({"argv": ["/nonexistent/docs-mcp"]})));
    dialect(
        root.path(),
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "hands", "serves": "workspace",
                "kind": "hands", "tools": ["workspace"],
                "sends": {"description": "nothing", "seat_composed": false}}),
    );
    let slice_two = "realm 'private' grants capability 'library-docs' through dialect \
                     'docs-mcp' of kind 'mcp', whose broker support is not implemented until \
                     decision 0065 slice two";
    // No seat has been looked at: a want, no ask, and an empty scope are
    // all the same refusal.
    for grant in [json!({"dialect": "docs-mcp"}), json!({"dialect": "docs-mcp", "offices": []})] {
        assert_eq!(refusal(root.path(), json!({"library-docs": grant})), slice_two);
    }
    assert_eq!(
        refusal(root.path(), json!({"workspace": {"dialect": "hands"}})),
        "realm 'private' grants capability 'workspace' through dialect 'hands' of kind \
         'hands', which is reserved: the workspace tool stays governed by decisions 0043 and \
         0046 and is not a realm grant"
    );
    // Validity comes first: an invalid neighbour is reported, not masked.
    assert_eq!(
        refusal(
            root.path(),
            json!({"library-docs": {"dialect": "docs-mcp"},
                   "web-search": {"dialect": "search-native", "tools": ["crawl"]}})
        ),
        "realm 'private' grants capability 'web-search' tool 'crawl', which dialect \
         'search-native' does not name; its tools are [lookup, search]"
    );
}

// ----------------------------------------------------------- resolution

#[test]
fn a_missing_grant_refuses_a_requirement_and_records_a_dropped_want() {
    let root = cq1_root();
    let nothing = authority(root.path(), json!({}));
    let native = switchable();
    assert_eq!(
        nothing
            .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
            .unwrap_err(),
        "seat 'research' (office 'researcher') in realm 'private': requires capability \
         'web-search' but the realm does not grant it to this office"
    );
    let outcome = nothing
        .resolve(&asks(json!({"web-search": "wants"})), &serving(&native))
        .unwrap();
    assert!(outcome.held.is_empty());
    assert_eq!(
        outcome.notices,
        [(
            "web-search".to_string(),
            "seat 'research' (office 'researcher') in realm 'private': dropped wanted \
             capability 'web-search' because the realm does not grant it to this office"
                .to_string()
        )]
    );
    assert_eq!(
        outcome.not_held["web-search"],
        "the realm does not grant it to this office"
    );
    // The known native power is switched OFF all the same.
    assert_eq!(argv_of(&outcome), ["--search-off"]);
}

#[test]
fn a_request_with_no_definition_is_invalid_before_optionality_or_subtraction() {
    let root = TempDir::new().unwrap();
    let nothing = authority(root.path(), json!({}));
    let native = switchable();
    let missing = "seat 'research' (office 'researcher') in realm 'private': capability \
                   'operator-library-docs' has no abstract definition at \
                   'capabilities/operator-library-docs.json' in the operator configuration; \
                   declare its classes before requesting it";
    for strength in ["requires", "wants"] {
        let site = asks(json!({"operator-library-docs": strength}));
        assert_eq!(nothing.resolve(&site, &serving(&native)).unwrap_err(), missing);
    }
    // Even an ask the seat subtracts must have been a valid one.
    let office = parse_requests("a", &json!({"operator-library-docs": "wants"})).unwrap();
    let subtracted = SiteAsks::of("research", Some(("researcher", &office)), Some(&json!({}))).unwrap();
    assert_eq!(nothing.resolve(&subtracted, &serving(&native)).unwrap_err(), missing);
    // Supplying the definition turns it into the ordinary outcomes, with
    // no dialect anywhere.
    define(root.path(), "operator-library-docs", &["reads", "egress"]);
    let defined = authority(root.path(), json!({}));
    assert_eq!(
        defined
            .resolve(&asks(json!({"operator-library-docs": "requires"})), &serving(&native))
            .unwrap_err(),
        "seat 'research' (office 'researcher') in realm 'private': requires capability \
         'operator-library-docs' but the realm does not grant it to this office"
    );
    let dropped = defined
        .resolve(&asks(json!({"operator-library-docs": "wants"})), &serving(&native))
        .unwrap();
    assert_eq!(
        dropped.notices[0].1,
        "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability \
         'operator-library-docs' because the realm does not grant it to this office"
    );
    let held_nothing = defined.resolve(&subtracted, &serving(&native)).unwrap();
    assert_eq!(
        held_nothing.not_held["operator-library-docs"],
        "this seat subtracted it from its office's asks"
    );
    assert!(held_nothing.notices.is_empty());
}

#[test]
fn office_scope_and_an_empty_tool_list_only_narrow() {
    let root = cq1_root();
    let native = switchable();
    for (grant, but) in [
        (
            json!({"dialect": "search-native", "offices": ["review-security", "reviewer"]}),
            "the realm grants it only to offices [review-security, reviewer], not to this office",
        ),
        (
            json!({"dialect": "search-native", "offices": []}),
            "the realm grants it to no office",
        ),
    ] {
        let scoped = authority(root.path(), json!({"web-search": grant}));
        assert_eq!(
            scoped
                .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
                .unwrap_err(),
            format!(
                "seat 'research' (office 'researcher') in realm 'private': requires \
                 capability 'web-search' but {but}"
            )
        );
        let dropped = scoped
            .resolve(&asks(json!({"web-search": "wants"})), &serving(&native))
            .unwrap();
        assert_eq!(
            dropped.notices[0].1,
            format!(
                "seat 'research' (office 'researcher') in realm 'private': dropped wanted \
                 capability 'web-search' because {but}"
            )
        );
        assert_eq!(argv_of(&dropped), ["--search-off"]);
    }
    let no_tools = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native", "tools": []}}),
    );
    assert_eq!(
        no_tools
            .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
            .unwrap_err(),
        "seat 'research' (office 'researcher') in realm 'private': requires capability \
         'web-search' through dialect 'search-native', but the realm's grant admits no tool; \
         the capability cannot be held under this grant"
    );
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
    assert_eq!(holding.dialect_sha256, granted.dialects["web-search"].sha256);
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
    assert_eq!(manifest["native"], json!({"inventory": "known", "declaration": "d1ge57",
                                          "on": ["web-search"], "off": []}));
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

#[test]
fn provider_compatibility_cannot_expand_a_holding() {
    let root = cq1_root();
    let granted = authority(root.path(), json!({"web-search": {"dialect": "search-native"}}));
    let site = asks(json!({"web-search": "requires"}));
    let wanting = asks(json!({"web-search": "wants"}));
    let head = "seat 'research' (office 'researcher') in realm 'private': requires capability \
                'web-search' through dialect 'search-native', but ";
    let tail = "; the capability cannot be held under this grant";
    let refused = |serving: &Serving<'_>| granted.resolve(&site, serving).unwrap_err();

    // Another provider is not a silent dialect substitution.
    let claude = switchable();
    let other = Serving { provider: "claude", ..serving(&claude) };
    assert_eq!(
        refused(&other),
        format!("{head}provider 'claude' cannot carry a binding to provider 'test-native'{tail}")
    );
    let dropped = granted.resolve(&wanting, &other).unwrap();
    assert_eq!(
        dropped.notices[0].1,
        "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability \
         'web-search' through dialect 'search-native' because provider 'claude' cannot carry \
         a binding to provider 'test-native'; native capability remains OFF"
    );

    // An unmeasured inventory cannot satisfy a request, and claims no denial.
    let unmeasured = NativeInventory::Unmeasured("never probed".into());
    assert_eq!(
        refused(&serving(&unmeasured)),
        format!(
            "{head}provider 'test-native' declares its native capabilities unmeasured (never \
             probed){tail}"
        )
    );
    let dropped = granted.resolve(&wanting, &serving(&unmeasured)).unwrap();
    assert_eq!(
        dropped.notices[0].1,
        "seat 'research' (office 'researcher') in realm 'private': dropped wanted capability \
         'web-search' through dialect 'search-native' because provider 'test-native' declares \
         its native capabilities unmeasured (never probed); no native denial is claimed"
    );
    assert_eq!(
        dropped.manifest()["native"],
        json!({"inventory": "unmeasured", "reason": "never probed", "declaration": "d1ge57"})
    );
    assert_eq!(
        dropped.controls(),
        json!({"inventory": "unmeasured", "reason": "never probed"})
    );
    assert_eq!(
        dropped.prompt()["native"],
        "Provider 'test-native' declares its native capabilities unmeasured (never probed); \
         nothing is claimed about what it can reach on its own"
    );
    // No adapter answers at all: an opaque driver.
    let opaque = Serving { native: None, model: None, ..serving(&unmeasured) };
    assert_eq!(
        refused(&opaque),
        format!("{head}no adapter declares provider 'test-native'{tail}")
    );
    let nothing = authority(root.path(), json!({}));
    let outcome = nothing.resolve(&SiteAsks::of("s", None, None).unwrap(), &opaque).unwrap();
    assert_eq!(
        outcome.manifest(),
        json!({"provider": "test-native", "held": {}, "not_held": {}, "notices": [],
               "native": {"inventory": "unmeasured",
                          "reason": "no adapter declares provider 'test-native'"}})
    );

    // The adapter has no such key, or the key serves something else.
    let mut elsewhere = native_dialect("search-elsewhere", "web-search", &["lookup"]);
    elsewhere["adapter_key"] = json!("web-lookup");
    dialect(root.path(), &elsewhere);
    let rebound = authority(root.path(), json!({"web-search": {"dialect": "search-elsewhere"}}));
    assert_eq!(
        rebound.resolve(&site, &serving(&claude)).unwrap_err(),
        "seat 'research' (office 'researcher') in realm 'private': requires capability \
         'web-search' through dialect 'search-elsewhere', but provider 'test-native' declares \
         no native capability 'web-lookup' serving it; the capability cannot be held under \
         this grant"
    );
    // A tool the harness does not have.
    dialect(root.path(), &native_dialect("search-crawl", "web-search", &["crawl"]));
    let crawling = authority(root.path(), json!({"web-search": {"dialect": "search-crawl"}}));
    assert_eq!(
        crawling.resolve(&site, &serving(&claude)).unwrap_err(),
        "seat 'research' (office 'researcher') in realm 'private': requires capability \
         'web-search' through dialect 'search-crawl', but provider 'test-native' native \
         'web-search' has no tool 'crawl'; the capability cannot be held under this grant"
    );
    // ON that cannot be switched, or that nobody measured.
    for (on, but) in [
        (json!({"unsupported": "no flag"}), "provider 'test-native' cannot switch it on (no flag)"),
        (
            json!({"unmeasured": "untried"}),
            "provider 'test-native' declares its ON control unmeasured (untried)",
        ),
    ] {
        let native = test_native(on, json!({"argv": ["--off"]}), json!({"unsupported": "x"}));
        assert_eq!(refused(&serving(&native)), format!("{head}{but}{tail}"));
    }
}

#[test]
fn tool_narrowing_is_exact_or_the_binding_is_incompatible() {
    let root = cq1_root();
    let narrowed = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native", "tools": ["lookup"]}}),
    );
    let site = asks(json!({"web-search": "requires"}));
    // A whole-set switch would enable the excluded tool.
    for on in [json!({"argv": ["--search-on"]}), json!({"default": "on by default"})] {
        let native = test_native(on, json!({"argv": ["--off"]}), json!({"unsupported": "x"}));
        assert_eq!(
            narrowed.resolve(&site, &serving(&native)).unwrap_err(),
            "seat 'research' (office 'researcher') in realm 'private': requires capability \
             'web-search' through dialect 'search-native', but provider 'test-native' \
             switches native 'web-search' on as a whole and cannot admit only [lookup] of its \
             tools [lookup, search]; the capability cannot be held under this grant"
        );
    }
    // A selection admits exactly the subset and denies the rest by name.
    let selecting = test_native(
        json!({"selection": {"include": ["lookup", "search"], "allow": ["lookup", "search"],
                             "deny": []}}),
        json!({"selection": {"include": [], "allow": [], "deny": ["lookup", "search"]}}),
        json!({"unsupported": "x"}),
    );
    let outcome = narrowed.resolve(&site, &serving(&selecting)).unwrap();
    assert_eq!(outcome.held["web-search"].tools, ["lookup"]);
    assert_eq!(
        outcome.controls()["selection"],
        json!({"include": ["lookup"], "allow": ["lookup"], "deny": ["search"],
               "flags": {"include": {"flag": "--tools", "separator": ","},
                         "allow": {"flag": "--allow", "separator": ","},
                         "deny": {"flag": "--deny", "separator": ","}}})
    );
    // Not held at all: the OFF selection denies both.
    let nothing = authority(root.path(), json!({}));
    let denied = nothing
        .resolve(&SiteAsks::of("implement", None, None).unwrap(), &serving(&selecting))
        .unwrap();
    assert_eq!(denied.controls()["selection"]["deny"], json!(["lookup", "search"]));
    assert_eq!(denied.controls()["selection"]["include"], json!([]));
}

#[test]
fn cq1_an_inexpressible_restriction_refuses_a_requirement_drops_a_want_and_idles_unused() {
    let root = cq1_root();
    let restricted = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native", "offices": ["researcher"],
                              "allow": {"hosts": ["sourceware.org", "yaml.org"]}}}),
    );
    let native = switchable();
    assert_eq!(
        restricted
            .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
            .unwrap_err(),
        "seat 'research' (office 'researcher') in realm 'private': requires capability \
         'web-search' through dialect 'search-native', but provider 'test-native' cannot \
         express restriction 'allow.hosts'; the capability cannot be held under this grant"
    );
    let dropped = restricted
        .resolve(&asks(json!({"web-search": "wants"})), &serving(&native))
        .unwrap();
    assert_eq!(
        dropped.notices,
        [(
            "web-search".to_string(),
            "seat 'research' (office 'researcher') in realm 'private': dropped wanted \
             capability 'web-search' through dialect 'search-native' because provider \
             'test-native' cannot express restriction 'allow.hosts'; native capability \
             remains OFF"
                .to_string()
        )]
    );
    // OFF, never an unrestricted ON.
    assert_eq!(argv_of(&dropped), ["--search-off"]);
    assert_eq!(
        dropped.not_held["web-search"],
        "provider 'test-native' cannot express restriction 'allow.hosts'"
    );
    // Unused: no ask, or an ask subtracted — inactive, OFF, and no notice
    // is invented for an ask that does not exist.
    let office = parse_requests("a", &json!({"web-search": "wants"})).unwrap();
    for site in [
        SiteAsks::of("research", Some(("researcher", &office)), Some(&json!({}))).unwrap(),
        SiteAsks::of("implement", None, None).unwrap(),
    ] {
        let idle = restricted.resolve(&site, &serving(&native)).unwrap();
        assert!(idle.held.is_empty() && idle.notices.is_empty());
        assert_eq!(argv_of(&idle), ["--search-off"]);
    }
    // The inactive restriction stays in realm context, exactly as written.
    assert_eq!(
        restricted.manifest(&[])["grants"]["web-search"]["allow"],
        json!({"hosts": ["sourceware.org", "yaml.org"]})
    );
    // Scope exclusion precedes restriction compatibility.
    assert_eq!(
        restricted
            .resolve(
                &SiteAsks::of("implement", None, Some(&json!({"web-search": "requires"}))).unwrap(),
                &serving(&native)
            )
            .unwrap_err(),
        "seat 'implement' (office 'implement') in realm 'private': requires capability \
         'web-search' but the realm grants it only to offices [researcher], not to this office"
    );
}

#[test]
fn an_expressible_restriction_rides_one_typed_argument_unchanged() {
    let root = cq1_root();
    let restricted = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native",
                              "allow": {"hosts": ["yaml.org", "sourceware.org"]}}}),
    );
    let carrying = test_native(
        json!({"argv": ["--search-on"]}),
        json!({"argv": ["--search-off"]}),
        json!({"argv": ["--search-restrict", "{restrictions_json}"]}),
    );
    let site = asks(json!({"web-search": "requires"}));
    let outcome = restricted.resolve(&site, &serving(&carrying)).unwrap();
    assert_eq!(
        argv_of(&outcome),
        [
            "--search-on",
            "--search-restrict",
            r#"{"allow":{"hosts":["yaml.org","sourceware.org"]}}"#
        ]
    );
    // The structured value is kept beside its encoding, array order intact.
    assert_eq!(
        Value::Object(outcome.held["web-search"].restrictions.clone()),
        json!({"allow": {"hosts": ["yaml.org", "sourceware.org"]}})
    );
    // No restriction, no transport argument.
    let plain = authority(root.path(), json!({"web-search": {"dialect": "search-native"}}));
    assert_eq!(
        argv_of(&plain.resolve(&site, &serving(&carrying)).unwrap()),
        ["--search-on"]
    );
    assert_eq!(
        restriction_names("", json!({"allow": {"hosts": [], "ports": {}}, "deny": 1}).as_object().unwrap()),
        ["allow.hosts", "allow.ports", "deny"]
    );
}

#[test]
fn a_native_power_that_cannot_be_switched_off_refuses_the_seat_whatever_it_asks() {
    let root = cq1_root();
    let stuck = test_native(
        json!({"default": "always on"}),
        json!({"unsupported": "the 9.9 CLI has no flag or config key that removes the tool"}),
        json!({"unsupported": "x"}),
    );
    let impossible = |site: &SiteAsks, label: &str, office: &str| {
        format!(
            "seat '{label}' (office '{office}') in realm 'private': provider 'test-native' \
             cannot switch off its native capability 'web-search', which this seat does not \
             hold (the 9.9 CLI has no flag or config key that removes the tool); an ungranted \
             native capability that cannot be disabled cannot be seated in this realm \
             (decision 0065 ruling 4)"
        ) + &site.label[..0]
    };
    // No ask at all, in a realm that grants nothing.
    let nothing = authority(root.path(), json!({}));
    let implement = SiteAsks::of("implement", None, None).unwrap();
    assert_eq!(
        nothing.resolve(&implement, &serving(&stuck)).unwrap_err(),
        impossible(&implement, "implement", "implement")
    );
    // A want does not excuse it, and neither does a grant to someone else.
    let wanting = asks(json!({"web-search": "wants"}));
    assert_eq!(
        nothing.resolve(&wanting, &serving(&stuck)).unwrap_err(),
        impossible(&wanting, "research", "researcher")
    );
    let elsewhere = authority(
        root.path(),
        json!({"web-search": {"dialect": "search-native", "offices": ["reviewer"]}}),
    );
    assert_eq!(
        elsewhere.resolve(&implement, &serving(&stuck)).unwrap_err(),
        impossible(&implement, "implement", "implement")
    );
    // Held through the grant, the same harness seats: ON is its default.
    let granted = authority(root.path(), json!({"web-search": {"dialect": "search-native"}}));
    let held = granted
        .resolve(&asks(json!({"web-search": "requires"})), &serving(&stuck))
        .unwrap();
    assert!(argv_of(&held).is_empty());
    // An OFF nobody measured is not a refusal and not a denial.
    let unknown = test_native(
        json!({"default": "on"}),
        json!({"unmeasured": "never tried"}),
        json!({"unsupported": "x"}),
    );
    let outcome = nothing.resolve(&implement, &serving(&unknown)).unwrap();
    assert_eq!(outcome.manifest()["native"]["unmeasured"], json!(["web-search"]));
    assert_eq!(outcome.manifest()["native"]["off"], json!([]));
    assert_eq!(
        outcome.not_held["web-search"],
        "provider 'test-native' has it natively and its OFF control is unmeasured (never \
         tried); it is not granted and no denial is claimed"
    );
    // Off by default needs no argv, and is still recorded as off.
    let quiet = test_native(
        json!({"argv": ["--search-on"]}),
        json!({"default": "off unless asked"}),
        json!({"unsupported": "x"}),
    );
    let outcome = nothing.resolve(&implement, &serving(&quiet)).unwrap();
    assert!(argv_of(&outcome).is_empty());
    assert_eq!(outcome.manifest()["native"]["off"], json!(["web-search"]));
}

#[test]
fn an_authored_native_control_is_refused_at_compile_naming_the_seat() {
    let root = cq1_root();
    let nothing = authority(root.path(), json!({}));
    let native = switchable();
    let authored = ["--model".to_string(), "m".to_string(), "--search".to_string()];
    let contending = Serving { authored: &authored, ..serving(&native) };
    assert_eq!(
        nothing
            .resolve(&SiteAsks::of("implement", None, None).unwrap(), &contending)
            .unwrap_err(),
        "seat 'implement' (office 'implement') in realm 'private': its arguments carry \
         '--search', which controls native capability 'web-search' of provider 'test-native'. \
         Only the realm grants a capability, and the engine composes the one control the \
         grant resolves to; request 'web-search' by name under 'capabilities' instead \
         (decision 0065 rulings 3 and 4)"
    );
}

#[test]
fn a_site_records_each_candidate_apart_and_serves_the_selected_one() {
    let root = cq1_root();
    let granted = authority(root.path(), json!({"web-search": {"dialect": "search-native"}}));
    let site = asks(json!({"web-search": "wants"}));
    let native = switchable();
    let primary = granted.resolve(&site, &serving(&native)).unwrap();
    let fallback = granted
        .resolve(&site, &Serving { provider: "claude", model: Some("opus"), ..serving(&native) })
        .unwrap();
    let recorded = SiteCapabilities {
        asks: site,
        outcomes: vec![primary.clone(), fallback.clone()],
    };
    // The fallback never borrows the primary's holding.
    assert_eq!(recorded.serving(Some(("test-native", "tn-1"))), Some(&primary));
    assert_eq!(recorded.serving(Some(("claude", "opus"))), Some(&fallback));
    assert!(recorded.serving(Some(("claude", "opus"))).unwrap().held.is_empty());
    assert_eq!(recorded.serving(Some(("codex", "astra"))), None);
    assert_eq!(recorded.serving(None), Some(&primary));
    let manifest = recorded.manifest();
    assert_eq!(manifest["office"], "researcher");
    assert_eq!(manifest["asks"], json!({"web-search": "wants"}));
    assert_eq!(manifest["subtracted"], json!([]));
    assert_eq!(manifest["candidates"][0]["held"]["web-search"]["dialect"], "search-native");
    assert_eq!(manifest["candidates"][1]["held"], json!({}));
    // The realm-wide half pins the grant, its definition and its dialect —
    // and a consulted definition no grant names.
    define(root.path(), "operator-library-docs", &["reads"]);
    let consulted = authority(root.path(), json!({"web-search": {"dialect": "search-native"}}))
        .manifest(&["operator-library-docs".to_string(), "undefined".to_string()]);
    assert_eq!(consulted["realm"], "private");
    assert_eq!(consulted["grants"], json!({"web-search": {"dialect": "search-native"}}));
    assert_eq!(consulted["dialects"]["search-native"]["kind"], "provider-native");
    assert_eq!(consulted["dialects"]["search-native"]["serves"], "web-search");
    assert_eq!(
        consulted["definitions"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["operator-library-docs", "web-search"]
    );
    let none = CapabilityContext::no_grants(UNMAPPED, root.path());
    assert_eq!(none.realm, "<unmapped>");
    assert!(none.grants.is_empty());
}

/// The embedded contract is the published one, byte for byte: a dialect
/// the loader admits is a dialect the contract admits.
#[test]
fn the_embedded_tool_dialect_contract_is_the_published_file() {
    let published = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/tool-dialect.v1.schema.json"),
    )
    .unwrap();
    assert_eq!(TOOL_DIALECT_SCHEMA, published);
}

/// What ships: two abstract definitions, three provider-native dialects
/// and nothing else — no MCP server, no general tool — each dialect bound
/// to a native key its adapter really declares, with the same tools.
#[test]
fn the_shipped_operator_data_is_native_only_and_agrees_with_the_adapters() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let definitions = Definitions::load(&workspace).unwrap();
    for name in ["web-search", "web-fetch"] {
        assert_eq!(sorted(&definitions.get(name).unwrap().classes), ["egress", "reads"]);
    }
    let adapters = crate::agents::Adapters::load(&workspace.join("adapters")).unwrap();
    let mut shipped: Vec<String> = std::fs::read_dir(workspace.join(DIALECTS_DIR))
        .unwrap()
        .map(|entry| entry.unwrap().path().file_stem().unwrap().to_str().unwrap().to_string())
        .collect();
    shipped.sort();
    assert_eq!(shipped, ["claude-native-fetch", "claude-native-search", "codex-native-search"]);
    for name in &shipped {
        let dialect = ToolDialect::load(&workspace, name).unwrap();
        let DialectKind::Native { provider, adapter_key } = &dialect.kind else {
            panic!("{name} is not provider-native");
        };
        let NativeInventory::Known { known, .. } = &adapters.adapter(provider).unwrap().native
        else {
            panic!("{provider} declares no known inventory");
        };
        assert_eq!(known[adapter_key].capability, dialect.serves);
        assert_eq!(known[adapter_key].tools, dialect.tools);
        assert!(dialect.seat_composed && dialect.egress == "uncontracted");
        // Shipped native dialects admit no restriction until a control exists.
        assert!(violation(&dialect.restrictions, &json!({"allow": {}})).is_err());
    }
    // Codex: the measured cold default ON, the exact OFF pair, the scope.
    let NativeInventory::Known { known, .. } = &adapters.adapter("codex").unwrap().native else {
        panic!("codex declares a known inventory");
    };
    let search = &known["web-search"];
    assert!(matches!(search.on, Disposition::Default(_)));
    assert_eq!(
        search.off,
        Disposition::Argv(vec!["-c".into(), "web_search=\"disabled\"".into()])
    );
    assert_eq!(
        search.evidence.source,
        ".forge/tasks/controller-codex-web-search-switch-2026-09-21.json"
    );
    assert!(search.evidence.scope.starts_with("codex-cli 0.154.0, cold `codex exec` only"));
    assert!(search.evidence.limitations.iter().any(|gap| gap.contains("RESUMED")));
    // DSH, LaneTally and exec keep their own uncertainty.
    for (provider, phrase) in [
        ("dsh", "does not establish that dsh has no native egress"),
        ("lanetally", "Claude's declarations and evidence are not inherited"),
        ("exec", "cannot certify what an arbitrary child program reaches"),
    ] {
        let NativeInventory::Unmeasured(reason) = &adapters.adapter(provider).unwrap().native
        else {
            panic!("{provider} must stay unmeasured");
        };
        assert!(reason.contains(phrase), "{provider}: {reason}");
    }
}
