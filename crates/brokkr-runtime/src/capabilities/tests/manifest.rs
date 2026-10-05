//! Decision 0065 slice two, U5f: the capability section of
//! `run-manifest/v12` (SC3, CR1), projected from the sealed outcome. A
//! native holding names its provider and adapter key beside a truthful
//! retention. An `mcp` implementation is typed whole from its dialect's
//! kind while the compile fence still refuses every `mcp` grant, so its
//! records are read here from holdings built past the fence.

use super::binding::MCP;
use super::dialect_policy::context_at;
use super::*;

/// An adapter declaration's digest, and the same adapter's after an edit.
const DECLARATION: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const MOVED: &str = "2222222222222222222222222222222222222222222222222222222222222222";

/// The published contract a compiled manifest claims, by `version`.
fn contract(version: &str) -> jsonschema::Validator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../contracts/run-manifest.{version}.schema.json"
    ));
    let schema: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    jsonschema::draft7::new(&schema).unwrap()
}

/// A whole manifest around one authority and one site's candidates.
fn manifest(authority: &Authority, site: SiteAsks, outcomes: Vec<Outcome>) -> Value {
    let mut capabilities = authority.manifest(&[]);
    capabilities["sites"] =
        json!({"research": SiteCapabilities { asks: site, outcomes }.manifest()});
    json!({"engine": "0.12.0", "event_schema": 1, "database_schema": 1, "driver_protocol": 1,
           "bundle_name": "fast", "files": {}, "capabilities": capabilities})
}

/// `docs-mcp`, retaining as `retained` says, loaded from `root`.
fn docs(root: &Path, connection: Value, retained: bool) -> ToolDialect {
    let mut docs = mcp_dialect("docs-mcp", connection);
    docs["retained"] = json!(retained);
    dialect(root, &docs);
    ToolDialect::load(root, "docs-mcp").unwrap()
}

/// A `library-docs` holding through `dialect`, as the resolver would seal
/// one past the fence, under `grant`.
fn held(dialect: &ToolDialect, grant: &CapabilityGrant) -> Holding {
    Holding {
        classes: vec!["reads".into(), "egress".into()],
        dialect: dialect.name.clone(),
        dialect_sha256: dialect.sha256.clone(),
        definition_sha256: "d".repeat(64),
        tools: grant.tools.clone().unwrap_or_else(|| dialect.tools.clone()),
        restrictions: grant.restrictions.clone(),
        implementation: Implementation::of("library-docs", &dialect.kind).unwrap(),
        retention: Retention::of(dialect, grant),
    }
}

/// One candidate holding `holding`, its adapter declaration `declaration`.
fn holding_outcome(holding: Holding, declaration: &str) -> Outcome {
    Outcome {
        provider: "test-native".into(),
        harness: OPAQUE_HARNESS.into(),
        model: Some("tn-1".into()),
        held: BTreeMap::from([("library-docs".to_string(), holding)]),
        not_held: BTreeMap::new(),
        notices: Vec::new(),
        native: NativePlan::Unmeasured {
            declaration: Some(declaration.into()),
            reason: "never probed".into(),
            provenance: launch::Provenance::default(),
        },
    }
}

/// The `library-docs` grant the map version `schema` reads from `written`.
fn docs_grant(root: &Path, schema: &str, written: Value) -> CapabilityGrant {
    context_at(schema, root, json!({"library-docs": written})).grants["library-docs"].clone()
}

/// A real native resolution records its provider and adapter key, and a
/// retention whose declaration and effect are false while the realm's
/// disposition is truthful: a v6 grant cannot veto, a v8 grant leaving
/// `retain` out inherits, and `retain: false` vetoes. The whole manifest is
/// v12, and is no longer v11, whose holdings are closed without them.
#[test]
fn a_native_holding_records_its_provider_and_a_truthful_retention_under_v12() {
    let root = cq1_root();
    let native = switchable();
    for (schema, written, realm) in [
        (
            "forge.realms/v6",
            json!({"dialect": "search-native"}),
            "inherit",
        ),
        (
            "forge.realms/v8",
            json!({"dialect": "search-native"}),
            "inherit",
        ),
        (
            "forge.realms/v8",
            json!({"dialect": "search-native", "retain": false}),
            "veto",
        ),
    ] {
        let authority = Authority::load(context_at(
            schema,
            root.path(),
            json!({"web-search": written}),
        ))
        .unwrap();
        let site = asks(json!({"web-search": "wants"}));
        let declared = Serving {
            native: Some((&native, DECLARATION)),
            ..serving(&native)
        };
        let outcome = authority.resolve(&site, &declared).unwrap();
        let fallback = Serving {
            provider: "claude",
            model: Some("opus"),
            ..declared
        };
        let unheld = authority.resolve(&site, &fallback).unwrap();
        assert_eq!(
            outcome.manifest()["held"]["web-search"],
            json!({"classes": ["reads", "egress"], "dialect": "search-native",
                   "dialect_sha256": authority.dialects["web-search"].sha256,
                   "definition_sha256": authority.definitions.get("web-search").unwrap().sha256,
                   "tools": ["lookup", "search"], "restrictions": {},
                   "implementation": {"kind": "provider-native", "provider": "test-native",
                                      "adapter_key": "web-search"},
                   "retention": {"declared": false, "realm": realm, "effective": false}}),
            "{schema} {written}"
        );
        // The fallback's record stays its own: it holds nothing.
        assert_eq!(unheld.manifest()["held"], json!({}));
        let whole = manifest(&authority, site, vec![outcome, unheld]);
        assert!(contract("v12").is_valid(&whole), "{whole}");
        assert!(
            !contract("v11").is_valid(&whole),
            "a v12 holding is not a v11 one"
        );
    }
}

/// CR1's four outcomes, read off the record: the declaration and the
/// realm's disposition are pinned as written, and the effective result is
/// the one rule `Retention::effective` holds.
#[test]
fn the_record_pins_the_four_retention_outcomes_beside_their_inputs() {
    let root = cq1_root();
    define(root.path(), "library-docs", &["reads", "egress"]);
    let inherit = docs_grant(
        root.path(),
        "forge.realms/v8",
        json!({"dialect": "docs-mcp"}),
    );
    let veto = docs_grant(
        root.path(),
        "forge.realms/v8",
        json!({"dialect": "docs-mcp", "retain": false}),
    );
    let stdio = json!({"argv": ["docs-mcp"]});
    let records: Vec<Value> = [
        (false, &inherit),
        (false, &veto),
        (true, &inherit),
        (true, &veto),
    ]
    .into_iter()
    .map(|(retained, grant)| {
        let holding = held(&docs(root.path(), stdio.clone(), retained), grant);
        holding_outcome(holding, DECLARATION).manifest()["held"]["library-docs"]["retention"]
            .clone()
    })
    .collect();
    assert_eq!(
        records,
        [
            json!({"declared": false, "realm": "inherit", "effective": false}),
            json!({"declared": false, "realm": "veto", "effective": false}),
            json!({"declared": true, "realm": "inherit", "effective": true}),
            json!({"declared": true, "realm": "veto", "effective": false}),
        ]
    );
}

/// SC3: an `mcp` implementation is typed whole — the server it is carried
/// under, either connection form as written, the pinned version and the
/// secret NAMES — and its record is inside v12. Typing it confers nothing:
/// the same grant, even reaching no office, still refuses the compile.
#[test]
fn an_mcp_implementation_is_typed_whole_and_its_grant_still_refuses_the_compile() {
    let root = cq1_root();
    define(root.path(), "library-docs", &["reads", "egress"]);
    let argv = vec![
        "docs-mcp".to_string(),
        "--token={{secret:DOCS_TOKEN}}".to_string(),
    ];
    let url = "https://docs.example.org/mcp".to_string();
    for (connection, typed) in [
        (json!({"argv": argv}), Connection::Stdio(argv.clone())),
        (json!({"url": url}), Connection::Url(url.clone())),
    ] {
        let dialect = docs(root.path(), connection.clone(), true);
        assert_eq!(
            Implementation::of("library-docs", &dialect.kind),
            Some(Implementation::Mcp {
                server: "cap-library-docs".into(),
                connection: typed,
                version: "1.4.2".into(),
                secrets: vec!["DOCS_TOKEN".into()],
            })
        );
        let grant = docs_grant(
            root.path(),
            "forge.realms/v8",
            json!({"dialect": "docs-mcp"}),
        );
        let record = holding_outcome(held(&dialect, &grant), DECLARATION).manifest();
        assert_eq!(
            record["held"]["library-docs"]["implementation"],
            json!({"kind": "mcp", "server": "cap-library-docs", "connection": connection,
                   "version": "1.4.2", "secrets": ["DOCS_TOKEN"]})
        );
        let site = SiteAsks::of("research", None, None).unwrap();
        let mut whole = manifest(&Authority::nothing("private", root.path()), site, vec![]);
        whole["capabilities"]["sites"]["research"]["candidates"] = json!([record]);
        assert!(contract("v12").is_valid(&whole), "{whole}");
        assert_eq!(
            Authority::load(context(
                root.path(),
                json!({"library-docs": {"dialect": "docs-mcp", "offices": []}})
            ))
            .unwrap_err(),
            MCP
        );
    }
    assert_eq!(Implementation::of("workspace", &DialectKind::Hands), None);
    let search = ToolDialect::load(root.path(), "search-native").unwrap();
    assert_eq!(
        Implementation::of("web-search", &search.kind),
        Some(Implementation::Native {
            provider: "test-native".into(),
            adapter_key: "web-search".into()
        })
    );
}

/// SC3's identity axes, each changed alone, move the candidate record's
/// digest, and identical bound bytes do not: a declaration changed under
/// a veto that keeps the effect false still moves it. No secret value is
/// a fact of a holding, so rotating one cannot reach the record.
#[test]
fn every_identity_axis_moves_the_record_alone_and_identical_bytes_do_not() {
    let root = cq1_root();
    define(root.path(), "library-docs", &["reads", "egress"]);
    let veto = |written: Value| {
        let mut written = written;
        written["retain"] = json!(false);
        docs_grant(root.path(), "forge.realms/v8", written)
    };
    let grant = veto(json!({"dialect": "docs-mcp"}));
    let stdio = json!({"argv": ["docs-mcp"]});
    let base = held(&docs(root.path(), stdio.clone(), true), &grant);
    let digest = |holding: &Holding, declaration: &str| {
        sha256_bytes(&to_bytes(
            &holding_outcome(holding.clone(), declaration).manifest(),
        ))
    };
    let edit = |change: &dyn Fn(&mut Holding)| {
        let mut holding = base.clone();
        change(&mut holding);
        digest(&holding, DECLARATION)
    };
    let mcp = |change: &dyn Fn(&mut Connection, &mut String, &mut Vec<String>)| {
        edit(&|holding| match &mut holding.implementation {
            Implementation::Mcp {
                connection,
                version,
                secrets,
                ..
            } => change(connection, version, secrets),
            Implementation::Native { .. } => unreachable!("the base is an mcp holding"),
        })
    };
    // The dialect's declaration changed under the standing veto, alone:
    // the effect is false either way.
    let undeclared = held(&docs(root.path(), stdio.clone(), false), &grant).retention;
    assert_eq!(
        (undeclared.effective(), base.retention.effective()),
        (false, false)
    );
    let inherit = docs_grant(
        root.path(),
        "forge.realms/v8",
        json!({"dialect": "docs-mcp"}),
    );
    let unvetoed = held(&docs(root.path(), stdio.clone(), true), &inherit);
    let axes = [
        (
            "connection",
            mcp(&|c, _, _| *c = Connection::Stdio(vec!["docs-mcp-2".into()])),
        ),
        (
            "connection form",
            mcp(&|c, _, _| *c = Connection::Url("https://d.example".into())),
        ),
        ("version", mcp(&|_, v, _| *v = "1.4.3".into())),
        ("secret names", mcp(&|_, _, s| s.push("DOCS_USER".into()))),
        ("tool set", edit(&|h| h.tools = vec!["read".into()])),
        (
            "restriction",
            edit(&|h| h.restrictions = json!({"a": 1}).as_object().unwrap().clone()),
        ),
        (
            "declaration under a veto",
            edit(&|h| h.retention = undeclared),
        ),
        ("veto", digest(&unvetoed, DECLARATION)),
        (
            "dialect bytes",
            edit(&|h| h.dialect_sha256 = "e".repeat(64)),
        ),
        (
            "definition bytes",
            edit(&|h| h.definition_sha256 = "f".repeat(64)),
        ),
        ("adapter evidence", digest(&base, MOVED)),
    ];
    let same = digest(&held(&docs(root.path(), stdio, true), &grant), DECLARATION);
    assert_eq!(
        same,
        digest(&base, DECLARATION),
        "identical bound bytes, one identity"
    );
    let mut seen = std::collections::BTreeSet::from([same]);
    for (axis, moved) in axes {
        assert!(seen.insert(moved), "{axis} did not move the record alone");
    }
    // Office scope is the grant's, pinned in the realm-wide half.
    let scoped = |offices: Value| {
        let mut authority = Authority::nothing("private", root.path());
        authority.context.grants = context_at(
            "forge.realms/v8",
            root.path(),
            json!({"library-docs": {"dialect": "docs-mcp", "offices": offices}}),
        )
        .grants;
        authority.manifest(&[])
    };
    assert_ne!(scoped(json!(["researcher"])), scoped(json!([])));
}
