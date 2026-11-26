use brokkr_core::realms::SCHEMA_V9;

use super::dialect_policy::context_at;
use super::*;

#[test]
fn every_grant_is_judged_before_any_seat_and_a_bad_one_is_never_an_optional_drop() {
    let root = cq1_root();
    let grant = |extra: Value| {
        let mut grant = json!({"dialect": "search-native"});
        grant
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        json!({"web-search": grant})
    };
    assert_eq!(
        refusal(
            root.path(),
            json!({"web-fetch": {"dialect": "search-native"}})
        ),
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
        refusal(
            root.path(),
            json!({"web-fetch": {"dialect": "search-native"}})
        ),
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
        refusal(
            root.path(),
            grant(json!({"offices": [], "allow": {"hosts": "sourceware.org"}}))
        ),
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
        refusal(
            root.path(),
            json!({"web-search": {"dialect": "reads-only", "offices": []}})
        ),
        "realm 'private': capability 'web-search' in dialect 'reads-only' declares classes \
         [reads], conflicting with abstract definition 'capabilities/web-search.json' classes \
         [reads, egress]"
    );
    // A valid grant loads, the dialect's annotation compared as a SET.
    let loaded = authority(
        root.path(),
        grant(json!({"allow": {"hosts": ["yaml.org"]}})),
    );
    assert_eq!(loaded.dialects["web-search"].name, "search-native");
}

fn provider_context(root: &Path, capability: &str, dialects: Value) -> CapabilityContext {
    context_at(SCHEMA_V9, root, json!({capability: {"dialects": dialects}}))
}

#[test]
fn every_provider_dialect_must_exist_and_serve_the_granted_capability() {
    let root = cq1_root();
    define(root.path(), "web-fetch", &["reads", "egress"]);
    let wrong = native_dialect("fetch-native", "web-fetch", &["fetch"]);
    dialect(root.path(), &wrong);
    let refuse = |name| {
        Authority::load(provider_context(
            root.path(),
            "web-search",
            json!({"test-native": "search-native", "z-provider": name}),
        ))
        .unwrap_err()
    };
    assert_eq!(
        refuse("absent"),
        "realm 'private': capability 'web-search': tool dialect 'absent' is not at \
         'dialects/tools/absent.json' in the operator configuration"
    );
    assert_eq!(
        refuse("fetch-native"),
        "realm 'private' grants capability 'web-search' through dialect 'fetch-native', \
         which serves 'web-fetch'"
    );
}

#[test]
fn a_native_dialect_must_be_under_its_own_provider_key() {
    let root = cq1_root();
    let context = provider_context(
        root.path(),
        "web-search",
        json!({"claude": "search-native"}),
    );
    assert_eq!(
        Authority::load(context).unwrap_err(),
        "realm 'private' grants capability 'web-search' through dialect 'search-native' under \
         provider key 'claude', but the dialect binds provider 'test-native'"
    );
}

#[test]
fn an_mcp_dialect_can_serve_any_provider_key() {
    let root = cq1_root();
    define(root.path(), "library-docs", &["reads", "egress"]);
    dialect(
        root.path(),
        &mcp_dialect("docs-mcp", json!({"argv": ["/nonexistent/docs-mcp"]})),
    );
    let context = provider_context(
        root.path(),
        "library-docs",
        json!({
            "claude": "docs-mcp", "test-native": "docs-mcp"
        }),
    );
    let surveyed = Authority::survey(context).unwrap();
    let names: Vec<_> = surveyed.provider_dialects["library-docs"]
        .iter()
        .map(|(provider, dialect)| {
            (
                provider.as_str(),
                dialect.name.as_str(),
                dialect.kind.word(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("claude", "docs-mcp", "mcp"),
            ("test-native", "docs-mcp", "mcp")
        ]
    );
}

fn claude_search(root: &Path) {
    let mut search = native_dialect("search-claude", "web-search", &["lookup", "search"]);
    search["provider"] = json!("claude");
    dialect(root, &search);
}

#[test]
fn each_provider_holds_and_records_only_its_own_native_dialect() {
    let root = cq1_root();
    claude_search(root.path());
    let written = json!({
        "dialects": {"claude": "search-claude", "test-native": "search-native"},
        "offices": ["researcher"],
        "retain": false
    });
    let authority = Authority::load(context_at(
        SCHEMA_V9,
        root.path(),
        json!({"web-search": written}),
    ))
    .unwrap();
    let native = switchable();
    let site = asks(json!({"web-search": "requires"}));
    let realm = authority.manifest(&[]);
    assert_eq!(realm["grants"]["web-search"], written);
    assert_eq!(
        realm["dialects"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["search-claude", "search-native"],
    );
    for (provider, name) in [
        ("claude", "search-claude"),
        ("test-native", "search-native"),
    ] {
        let candidate = Serving {
            provider,
            ..serving(&native)
        };
        let outcome = authority.resolve(&site, &candidate).unwrap();
        let record = outcome.manifest();
        let held = &record["held"]["web-search"];
        assert_eq!(held["dialect"], name);
        assert_eq!(held["dialect_sha256"], realm["dialects"][name]["sha256"]);
        assert_eq!(held["implementation"]["provider"], provider);
        assert_eq!(held["retention"]["realm"], "veto");
        assert_eq!(record["notices"], json!([]));
        assert_eq!(argv_of(&outcome), ["--search-on"]);
    }
}

#[test]
fn a_provider_without_a_key_never_holds_another_providers_dialect() {
    use super::binding::WHO;

    let root = cq1_root();
    claude_search(root.path());
    let authority = Authority::load(provider_context(
        root.path(),
        "web-search",
        json!({"claude": "search-claude"}),
    ))
    .unwrap();
    let native = switchable();
    let resolve =
        |strength| authority.resolve(&asks(json!({"web-search": strength})), &serving(&native));
    let cause =
        "provider 'test-native' has no dialect in this grant, which serves providers [claude]";
    assert_eq!(
        resolve("requires").unwrap_err(),
        format!(
            "{WHO}: requires capability 'web-search' through the realm's per-provider grant, \
             but {cause}; the capability cannot be held under this grant"
        )
    );
    let outcome = resolve("wants").unwrap();
    let record = outcome.manifest();
    assert_eq!(record["held"], json!({}));
    assert_eq!(argv_of(&outcome), ["--search-off"]);
    assert_eq!(
        record["not_held"]["web-search"],
        format!("{cause}; native capability remains OFF")
    );
    assert_eq!(
        record["notices"],
        json!([format!(
            "{WHO}: dropped wanted capability 'web-search' through the realm's \
             per-provider grant because {cause}; native capability remains OFF"
        )])
    );
}
