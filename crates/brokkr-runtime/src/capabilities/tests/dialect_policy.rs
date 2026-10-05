//! Decision 0065 slice two, U5a: the extracted dialect loader keeps every
//! typed v1 fact (SC1), a grant's reserved keys follow the map version
//! that wrote it (SC2), and a holding carries what it retains (CR1). An
//! `mcp` grant still meets the compile fence first; its typed policy is
//! read here at the seams past it, as U2's binding proofs are.

use super::binding::{past_the_fence, MCP, WHO};
use super::*;
use crate::agents::EgressClass;
use crate::capabilities::binding::Unserved;
use crate::capabilities::dialect::{conforms, Outside, Undeclared};
use brokkr_core::realms::{GrantRetention, GRANT_KEYS};

/// A `forge.realms/v8` grant colliding with a dialect claiming `retain`.
const RETAIN_RESERVED: &str = "realm 'private': capability 'web-search': tool dialect \
                               'claims-retain' restriction schema redefines reserved grant key \
                               'retain'";

/// The `private` realm's grants as the map version `schema` reads them,
/// beside the root [`context`] names.
pub(super) fn context_at(schema: &str, root: &Path, capabilities: Value) -> CapabilityContext {
    let private = json!([{"name": "private", "path": "repo", "default_branch": "main",
                          "capabilities": capabilities}]);
    let written = json!({"schema": schema, "journal": "forge.db", "realms": private});
    let grants = RealmMap::of("realms.json", written).unwrap().0.realms[0]
        .grants
        .clone();
    CapabilityContext {
        grants,
        ..context(root, json!({}))
    }
}

/// An `mcp` launch reaches a credential only by a declared binding NAME
/// (decision 0012). The check is textual — no store is opened, no server
/// started — and no refusal repeats the argument or the URL, either of
/// which may be where a credential was pasted by hand.
#[test]
fn an_mcp_launch_names_only_the_secrets_its_dialect_declares_and_echoes_no_credential() {
    let root = TempDir::new().unwrap();
    let load = |connection: Value| {
        dialect(root.path(), &mcp_dialect("docs", connection));
        ToolDialect::load(root.path(), "docs")
    };
    // The declared binding, referenced twice and beside plain text, loads.
    load(json!({"argv": ["docs-mcp", "--token={{secret:DOCS_TOKEN}}", "{{secret:DOCS_TOKEN}}"]}))
        .unwrap();
    assert_eq!(
        load(
            json!({"argv": ["docs-mcp", "{{secret:DOCS_TOKEN}}", "--key", "{{secret:OTHER_KEY}}"]})
        )
        .unwrap_err(),
        "tool dialect 'dialects/tools/docs.json' connection argv[3] references secret \
         'OTHER_KEY', which its 'secrets' does not declare; a server reaches only the bindings \
         its dialect names (decision 0012)"
    );
    let malformed = load(json!({"argv": ["docs-mcp", "hunter2-{{secret:lower}}"]})).unwrap_err();
    assert_eq!(
        malformed,
        "tool dialect 'dialects/tools/docs.json' connection argv[1] carries a malformed secret \
         reference; a reference is {{secret:NAME}} with NAME matching [A-Z][A-Z0-9_]*"
    );
    // Userinfo in a URL is refused by the contract's own pattern, and the
    // refusal names the FIELD and the clause, never the value.
    let userinfo = load(json!({"url": "https://operator:hunter2@docs.invalid/mcp"})).unwrap_err();
    assert_eq!(
        userinfo,
        "tool dialect 'dialects/tools/docs.json' is outside brokkr.tool-dialect/v1 at \
         '/connection/url': it does not satisfy '/properties/connection/properties/url/pattern'"
    );
    for refusal in [malformed, userinfo] {
        assert!(!refusal.contains("hunter2"), "{refusal}");
    }
}

/// SC1 and ruling 8: the two edge checks answer with typed variants, and
/// the loader alone renders them, in the words pinned above. A URL's
/// launch names nothing to check here.
#[test]
fn the_dialect_edge_checks_refuse_with_typed_variants() {
    let file = "dialects/tools/docs.json";
    let server = |argv: &[&str]| McpServer {
        connection: Connection::Stdio(argv.iter().map(|part| part.to_string()).collect()),
        version: "1.4.2".into(),
        secrets: vec!["DOCS_TOKEN".into()],
        retained: false,
    };
    let url = McpServer {
        connection: Connection::Url("https://docs.invalid/mcp".into()),
        ..server(&[])
    };
    assert_eq!(
        [
            server(&["docs-mcp", "{{secret:DOCS_TOKEN}}"]).undeclared(file),
            server(&["docs-mcp", "hunter2-{{secret:lower}}"]).undeclared(file),
            server(&["docs-mcp", "x", "{{secret:OTHER_KEY}}"]).undeclared(file),
            url.undeclared(file),
        ],
        [
            Ok(()),
            Err(Undeclared::Malformed {
                file: file.into(),
                index: 1
            }),
            Err(Undeclared::Secret {
                file: file.into(),
                index: 2,
                name: "OTHER_KEY".into()
            }),
            Ok(()),
        ]
    );
    let userinfo = mcp_dialect(
        "docs",
        json!({"url": "https://operator:hunter2@docs.invalid/mcp"}),
    );
    assert_eq!(
        conforms(file, &mcp_dialect("docs", json!({"argv": ["d"]}))),
        Ok(())
    );
    assert_eq!(
        conforms(file, &userinfo),
        Err(Outside {
            file: file.into(),
            problem: "at '/connection/url': it does not satisfy \
                      '/properties/connection/properties/url/pattern'"
                .into()
        })
    );
}

/// The embedded contract is the published one, byte for byte: a dialect
/// the loader admits is a dialect the contract admits. Its egress words
/// are decision 0036's whole vocabulary, which the loader's typed reading
/// relies on.
#[test]
fn the_embedded_tool_dialect_contract_is_the_published_file() {
    let published = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/tool-dialect.v1.schema.json"),
    )
    .unwrap();
    assert_eq!(crate::capabilities::dialect::TOOL_DIALECT_SCHEMA, published);
    let contract: Value = serde_json::from_str(&published).unwrap();
    let words = contract["properties"]["egress"]["enum"].as_array().unwrap();
    let classes: Vec<_> = words
        .iter()
        .map(|word| EgressClass::parse(word.as_str().unwrap()))
        .collect();
    let all = [
        EgressClass::Local,
        EgressClass::Contracted,
        EgressClass::Uncontracted,
    ];
    assert_eq!(classes, all.map(Some));
}

/// SC1: what an `mcp` dialect declares survives loading whole, and its
/// identity is the file's bytes — every declared fact moves it, while a
/// secret's VALUE, which no dialect holds, does not. Loading launches
/// nothing and reads no store.
#[test]
fn an_mcp_dialect_keeps_every_executable_fact_and_runs_nothing() {
    let root = cq1_root();
    let marker = root.path().join("launched");
    let launch = json!({"argv": ["/bin/sh", "-c", format!("touch {}", marker.display())]});
    let store = root.path().join("secrets.json");
    let load = |mutate: &dyn Fn(&mut Value)| {
        let mut docs = mcp_dialect("docs", launch.clone());
        mutate(&mut docs);
        dialect(root.path(), &docs);
        ToolDialect::load(root.path(), "docs").unwrap()
    };
    brokkr_protocol::secret::store_set(&store, "DOCS_TOKEN", "first-value").unwrap();
    let loaded = load(&|_| {});
    let argv = serde_json::from_value(launch["argv"].clone()).unwrap();
    let server = McpServer {
        connection: Connection::Stdio(argv),
        version: "1.4.2".into(),
        secrets: vec!["DOCS_TOKEN".into()],
        retained: true,
    };
    assert_eq!(loaded.kind, DialectKind::Mcp(server.clone()));
    let bytes = std::fs::read(root.path().join(ToolDialect::source_of("docs"))).unwrap();
    assert_eq!(loaded.sha256, sha256_bytes(&bytes));
    brokkr_protocol::secret::store_set(&store, "DOCS_TOKEN", "second-value").unwrap();
    assert_eq!(load(&|_| {}).sha256, loaded.sha256);
    // Omitted, the declaration reads false and is still a change of bytes.
    let omitted = load(&|docs| {
        docs.as_object_mut().unwrap().remove("retained");
    });
    let undeclared = McpServer {
        retained: false,
        ..server
    };
    assert_eq!(omitted.kind, DialectKind::Mcp(undeclared));
    let mut digests = vec![loaded.sha256.clone(), omitted.sha256];
    for (key, changed) in [
        ("connection", json!({"url": "https://docs.invalid/mcp"})),
        ("version", json!("1.4.3")),
        ("secrets", json!(["DOCS_TOKEN", "DOCS_USER"])),
        ("retained", json!(false)),
    ] {
        digests.push(load(&|docs| docs[key] = changed.clone()).sha256);
    }
    digests.sort();
    digests.dedup();
    assert_eq!(digests.len(), 6);
    // Granted, the realm-wide fence still refuses it before any seat, and
    // nothing was ever launched.
    define(root.path(), "library-docs", &["reads", "egress"]);
    dialect(root.path(), &mcp_dialect("docs-mcp", launch.clone()));
    let fenced = refusal(
        root.path(),
        json!({"library-docs": {"dialect": "docs-mcp"}}),
    );
    assert_eq!((fenced.as_str(), marker.exists()), (MCP, false));
}

/// SC1's order: the contract, then the kind's own facts, then the
/// restriction schema — and a malformed declaration is refused by the
/// contract, never defaulted by the typed reading.
#[test]
fn a_secret_reference_is_judged_before_the_restriction_schema_and_nothing_defaults() {
    let root = TempDir::new_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let load = |docs: &Value| {
        dialect(root.path(), docs);
        ToolDialect::load(root.path(), "docs").unwrap_err()
    };
    let mut docs = mcp_dialect("docs", json!({"argv": ["docs-mcp", "{{secret:OTHER}}"]}));
    docs["restrictions"] = json!({"properties": {"tools": {}}});
    assert_eq!(
        load(&docs),
        "tool dialect 'dialects/tools/docs.json' connection argv[1] references secret 'OTHER', \
         which its 'secrets' does not declare; a server reaches only the bindings its dialect \
         names (decision 0012)"
    );
    let outside = "tool dialect 'dialects/tools/docs.json' is outside brokkr.tool-dialect/v1 at";
    docs["retained"] = json!("yes");
    assert_eq!(
        load(&docs),
        format!("{outside} '/retained': it does not satisfy '/properties/retained/type'")
    );
    let mut native = native_dialect("docs", "web-search", &["lookup"]);
    native["retained"] = json!(false);
    assert_eq!(
        load(&native),
        format!("{outside} '': it does not satisfy '/additionalProperties'")
    );
}

/// The `web-search` grant `grant`, in the `private` realm of map version
/// `schema`, through dialect `claims-retain` whose restriction schema is
/// `restrictions`: the loaded grant's veto and the restrictions its
/// dialect's schema judged, or the refusal.
fn claims_under(
    root: &Path,
    restrictions: &Value,
    schema: &str,
    grant: &Value,
) -> Result<(GrantRetention, Value), String> {
    let mut claims = native_dialect("claims-retain", "web-search", &["lookup", "search"]);
    claims["restrictions"] = restrictions.clone();
    dialect(root, &claims);
    let written = json!({"web-search": grant});
    Authority::load(context_at(schema, root, written)).map(|loaded| {
        let grant = &loaded.context.grants["web-search"];
        (grant.retention, Value::Object(grant.restrictions.clone()))
    })
}

/// The rows of a table whose outcome differs from the one expected.
fn moved<'r, T: PartialEq>(seen: &'r [T], expected: &'r [T]) -> Vec<(&'r T, &'r T)> {
    assert_eq!(seen.len(), expected.len());
    seen.iter()
        .zip(expected)
        .filter(|(seen, expected)| seen != expected)
        .collect()
}

/// SC2's collision (operator ruling, 2026-10-04): a restriction schema
/// claims a reserved key DIRECTLY by a key of its root `properties` or
/// `dependencies`, or an entry of its root `required` or of a root
/// `dependencies` list. Each form, for each key, is refused exactly: the
/// three every version reserves under v6, v7 and v8 alike, and `retain`
/// under v8 only, inheriting or vetoing. Under v6 and v7 the same `retain`
/// claim is the dialect's own, and the written `retain` its restriction.
#[test]
fn a_direct_claim_of_a_reserved_key_is_refused_by_the_versions_that_reserve_it() {
    let root = cq1_root();
    let inherit = json!({"dialect": "claims-retain", "allow": {"retain": false}});
    let veto = json!({"dialect": "claims-retain", "allow": {"retain": false}, "retain": false});
    let under = [
        ("forge.realms/v8", &inherit),
        ("forge.realms/v8", &veto),
        ("forge.realms/v6", &veto),
        ("forge.realms/v7", &veto),
    ];
    let restriction = json!({"allow": {"retain": false}, "retain": false});
    let (mut seen, mut expected) = (Vec::new(), Vec::new());
    for key in GRANT_KEYS.into_iter().chain(["retain"]) {
        let forms = [
            ("properties", json!({"properties": {key: {}}})),
            ("dependencies", json!({"dependencies": {key: ["allow"]}})),
            ("required", json!({"required": [key]})),
            (
                "dependencies list",
                json!({"dependencies": {"allow": [key]}}),
            ),
        ];
        for (form, restrictions) in &forms {
            for (schema, grant) in under {
                let outcome = claims_under(root.path(), restrictions, schema, grant);
                seen.push((key, *form, schema, outcome));
                let outcome = match (key, schema) {
                    ("retain", "forge.realms/v8") => Err(RETAIN_RESERVED.to_string()),
                    ("retain", _) => Ok((GrantRetention::Unreserved, restriction.clone())),
                    _ => Err(format!(
                        "realm 'private': capability 'web-search': tool dialect \
                         'dialects/tools/claims-retain.json' restriction schema redefines \
                         '{key}', which is a key of the grant the engine owns"
                    )),
                };
                expected.push((key, *form, schema, outcome));
            }
        }
    }
    assert_eq!((seen.len(), moved(&seen, &expected)), (64, Vec::new()));
}

/// SC2's guarantee (operator ruling, 2026-10-04): a grant's reserved keys
/// never reach its dialect's restriction validation, so no schema decides
/// one, however it is composed. Each schema below forbids `retain` only
/// indirectly — through a reference, a conditional, a dependency schema,
/// or all three — and the validator itself shows each claim is live: it
/// refuses the grant's restrictions with `retain` beside them. A v8 veto
/// loads anyway, its restrictions exactly the rest; under v6, where
/// `retain` is no reserved key but a restriction, the same claim decides
/// the grant.
#[test]
fn a_reserved_key_never_reaches_restriction_validation() {
    let root = cq1_root();
    let forbids = json!({"properties": {"retain": false}});
    let reference = json!({"$ref": "#/definitions/forbids"});
    let conditional = json!({"if": {"required": ["allow"]}, "then": forbids});
    let dependency = json!({"allow": forbids});
    let claims = [
        (
            "reference",
            json!({"definitions": {"forbids": forbids}, "allOf": [reference]}),
            "/definitions/forbids/properties/retain",
        ),
        (
            "conditional",
            conditional.clone(),
            "/then/properties/retain",
        ),
        (
            "dependency",
            json!({"dependencies": dependency}),
            "/dependencies/allow/properties/retain",
        ),
        (
            "all three",
            json!({"definitions": {"forbids": forbids}, "allOf": [reference, conditional],
                   "dependencies": dependency}),
            "/dependencies/allow/properties/retain",
        ),
    ];
    let restrictions = json!({"allow": {"hosts": ["docs.invalid"]}});
    let mut retaining = restrictions.clone();
    retaining["retain"] = json!(false);
    let mut veto = retaining.clone();
    veto["dialect"] = json!("claims-retain");
    let (mut seen, mut expected) = (Vec::new(), Vec::new());
    for (form, schema, clause) in &claims {
        let validator = jsonschema::draft7::new(schema).unwrap();
        let live = (
            validator.is_valid(&restrictions),
            validator.is_valid(&retaining),
        );
        let v8 = claims_under(root.path(), schema, "forge.realms/v8", &veto);
        let v6 = claims_under(root.path(), schema, "forge.realms/v6", &veto);
        seen.push((*form, live, v8, v6));
        let decided = format!(
            "realm 'private' grants capability 'web-search' through dialect 'claims-retain' \
             with an invalid restriction at '/retain': it does not satisfy '{clause}'"
        );
        let judged = Ok((GrantRetention::Veto, restrictions.clone()));
        expected.push((*form, (true, false), judged, Err(decided)));
    }
    assert_eq!((seen.len(), moved(&seen, &expected)), (4, Vec::new()));
}

/// The restriction schemas that land a reference on one definition for
/// the validator and on another for a plain JSON pointer: a malformed
/// tilde escape, a percent-encoded pointer and an `$id` that rebases a
/// nested reference. Each row is `(form, schema)`: `validator` is written
/// where the validator lands and `plain` where a plain pointer would, so
/// that a caller can put the claim on either.
fn landing_apart(validator: &Value, plain: &Value) -> Vec<(&'static str, Value)> {
    let tilde = |written: &str, lands: &str, misses: &str| {
        json!({"definitions": {lands: validator, misses: plain},
               "$ref": format!("#/definitions/{written}")})
    };
    vec![
        ("~~0", tilde("~~0", "~~0", "~~")),
        ("~~1", tilde("~~1", "~~1", "~/")),
        ("~~~~0", tilde("~~~~0", "~~~~0", "~~~~")),
        ("percent", tilde("a%20b", "a b", "a%20b")),
        (
            "inner $id",
            json!({"definitions": {"z": plain},
                   "allOf": [{"$id": "http://brokkr.invalid/inner",
                              "definitions": {"z": validator},
                              "allOf": [{"$ref": "#/definitions/z"}]}]}),
        ),
    ]
}

/// The shapes three security holds found a static walk misreading
/// (operator ruling, 2026-10-04), each now inert: a claim the validator
/// lands on through `~~0`, `~~1`, `~~~~0`, a percent-encoded pointer or an
/// `$id`-rebased reference, and the decoy a plain pointer would land on
/// instead; a claim under an orphan `then` or `else`, an inactive `else`
/// or an `additionalItems`; and claims through composition, a reference,
/// a pattern, a dependency schema, a nested key and beside a `$ref`. Each
/// claim forbids its key with a `false` schema and the grant writes every
/// key it can, so a claim that decided anything would refuse it. For each
/// reserved key every shape loads, the v8 veto kept and the restrictions
/// exactly the rest.
#[test]
fn an_indirect_claim_of_a_reserved_key_is_inert_however_it_is_reached() {
    let root = cq1_root();
    let restrictions = json!({"allow": {"hosts": ["docs.invalid"]}});
    let mut grant = restrictions.clone();
    grant["dialect"] = json!("claims-retain");
    grant["tools"] = json!(["lookup"]);
    grant["retain"] = json!(false);
    let (mut seen, mut expected) = (Vec::new(), Vec::new());
    for key in GRANT_KEYS.into_iter().chain(["retain"]) {
        let claim = json!({"properties": {key: false}});
        let pattern = format!("^{key}$");
        let mut shapes: Vec<_> = landing_apart(&claim, &json!({}))
            .into_iter()
            .map(|(form, schema)| ((form, "lands"), schema))
            .chain(
                landing_apart(&json!({}), &claim)
                    .into_iter()
                    .map(|(form, schema)| ((form, "decoy"), schema)),
            )
            .collect();
        shapes.extend([
            (("orphan then", ""), json!({"then": claim})),
            (("orphan else", ""), json!({"else": claim})),
            (("inactive else", ""), json!({"if": true, "else": claim})),
            (("additionalItems", ""), json!({"additionalItems": claim})),
            (
                ("additionalItems after items", ""),
                json!({"items": [{}], "additionalItems": claim}),
            ),
            (("composition", ""), json!({"allOf": [claim]})),
            (
                ("reference", ""),
                json!({"definitions": {"top": {"anyOf": [claim]}}, "$ref": "#/definitions/top"}),
            ),
            (
                ("pattern", ""),
                json!({"patternProperties": {pattern: false}}),
            ),
            (
                ("dependency", ""),
                json!({"dependencies": {"allow": claim}}),
            ),
            (
                ("dependency reference", ""),
                json!({"definitions": {"claim": claim},
                       "dependencies": {"allow": {"$ref": "#/definitions/claim"}}}),
            ),
            (("nested", ""), json!({"properties": {"allow": claim}})),
            (
                ("beside a $ref", ""),
                json!({"definitions": {"a": {}}, "$ref": "#/definitions/a", "allOf": [claim]}),
            ),
        ]);
        for (form, schema) in &shapes {
            let outcome = claims_under(root.path(), schema, "forge.realms/v8", &grant);
            seen.push((key, *form, outcome));
            let judged = Ok((GrantRetention::Veto, restrictions.clone()));
            expected.push((key, *form, judged));
        }
    }
    assert_eq!((seen.len(), moved(&seen, &expected)), (88, Vec::new()));
}

/// SC1's containment, as slice one walked it and independent of the
/// direct reserved-key check: every `$ref` in the whole document, data
/// keywords aside, is a fragment of the dialect file that a plain JSON
/// pointer finds in it — in an unused definition, an orphan `then` or an
/// item list too. Resolving without a fetch is not containment: the
/// validator compiles the meta-schema, its default base spelled out, a
/// percent-encoded pointer, a plain-name anchor and a missing pointer in a
/// branch it never applies, and each is refused anyway. An `$id` or a `$ref` inside data, an unused definition's `$id`
/// and an applied `$id` load. No refusal repeats the authored reference.
#[test]
fn every_reference_stays_inside_the_dialect_file_wherever_it_hides() {
    let root = TempDir::new_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let load = |restrictions: &Value| {
        let mut harmless = native_dialect("d", "web-search", &["lookup"]);
        harmless["restrictions"] = restrictions.clone();
        dialect(root.path(), &harmless);
        ToolDialect::load(root.path(), "d").map(|loaded| loaded.name)
    };
    let refused = |fault: &str| {
        Err(format!(
            "tool dialect 'dialects/tools/d.json' restriction schema has a '$ref' {fault}"
        ))
    };
    let external = refused("outside the dialect file; a restriction schema is never fetched");
    let missing = refused("that names nothing in the dialect file");
    let meta = json!({"$ref": "http://json-schema.org/draft-07/schema#"});
    let shapes = [
        (
            json!({"properties": {"allow": {"default": {"$id": "https://hunter2.invalid/d"}}}}),
            true,
            Ok("d".to_string()),
        ),
        (
            json!({"properties": {"allow": {"examples": [{"$ref": "https://hunter2.invalid"}]}}}),
            true,
            Ok("d".to_string()),
        ),
        (
            json!({"definitions": {"unused": {"$id": "https://example.invalid/unused"}}}),
            true,
            Ok("d".to_string()),
        ),
        (
            json!({"properties": {"allow": {"$id": "https://example.invalid/a", "type": "object"}}}),
            true,
            Ok("d".to_string()),
        ),
        (json!({"not": meta}), true, external.clone()),
        (
            json!({"definitions": {"unused": {"$ref": "https://hunter2.invalid/x"}}}),
            false,
            external.clone(),
        ),
        (
            json!({"definitions": {"a": {}}, "$ref": "json-schema:///#/definitions/a"}),
            true,
            external,
        ),
        (
            json!({"definitions": {"unused": {"$ref": "#/hunter2"}}}),
            true,
            missing.clone(),
        ),
        (
            json!({"then": {"items": [{}, {"$ref": "#/definitions/hunter2"}]}}),
            true,
            missing.clone(),
        ),
        (
            json!({"definitions": {"a b": {}}, "properties": {"allow": {"$ref": "#/definitions/a%20b"}}}),
            true,
            missing.clone(),
        ),
        (
            json!({"definitions": {"x": {"$id": "#anchor", "type": "object"}}, "$ref": "#anchor"}),
            true,
            missing,
        ),
    ];
    let seen: Vec<_> = shapes
        .iter()
        .map(|(schema, _, _)| (jsonschema::draft7::new(schema).is_ok(), load(schema)))
        .collect();
    let expected: Vec<_> = shapes
        .iter()
        .map(|(_, compiles, loads)| (*compiles, loads.clone()))
        .collect();
    assert_eq!((seen.len(), moved(&seen, &expected)), (11, Vec::new()));
    assert!(!format!("{seen:?}").contains("hunter2"));
}

/// SC1's safe diagnostics (ruling 8): a restriction schema refused before
/// it is compiled is named by the keyword or the reserved key, never by an
/// authored schema, reference or `$id`, any of which may be where a
/// credential was pasted. Each fault is its typed variant; the loader
/// renders one here, and `a_restriction_schema_stays_inside_its_file_*`
/// pins the rest.
#[test]
fn a_refused_restriction_schema_names_no_authored_text() {
    use crate::capabilities::dialect::{embedded_schema_fault, SchemaFault};
    assert_eq!(
        [
            json!({"$schema": "hunter2"}),
            json!({"$id": "http://[hunter2", "type": "object"}),
            json!({"$ref": "#/const", "const": {"$id": "http://[hunter2"}}),
            json!({"properties": {"allow": {"$ref": "https://hunter2.invalid/hosts.json"}}}),
            json!({"properties": {"tools": {"$ref": "#/definitions/hunter2"}}}),
            json!({"dependencies": {"allow": ["offices"]}, "default": "hunter2"}),
        ]
        .map(|schema| embedded_schema_fault(&schema)),
        [
            Some(SchemaFault::Draft),
            Some(SchemaFault::Unindexed),
            Some(SchemaFault::Unindexed),
            Some(SchemaFault::External),
            Some(SchemaFault::Missing),
            Some(SchemaFault::Reserved { key: "offices" }),
        ]
    );
    // The validator's resolver reaches every `$id` behind a reference into
    // data, and below an applicator there, so one that is not a URI is
    // refused before the schema is compiled.
    for schema in [
        json!({"allOf": [{"$id": "http://[hunter2"}]}),
        json!({"$ref": "#/const/a", "const": {"a": {"$id": "http://[hunter2"}}}),
        json!({"$ref": "#/const", "const": {"properties": {"x": {"$id": "http://[hunter2"}}}}),
        json!({"$ref": "#/enum/0", "enum": [{"allOf": [{"$id": "http://[hunter2"}]}]}),
    ] {
        assert_eq!(
            embedded_schema_fault(&schema),
            Some(SchemaFault::Unindexed),
            "{schema}"
        );
    }
    let root = TempDir::new_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut claims = native_dialect("d", "web-search", &["lookup"]);
    claims["restrictions"] = json!({"properties": {"tools": {"$ref": "#/definitions/x"}},
                                    "definitions": {"x": {"default": "hunter2"}}});
    dialect(root.path(), &claims);
    assert_eq!(
        ToolDialect::load(root.path(), "d").unwrap_err(),
        "tool dialect 'dialects/tools/d.json' restriction schema redefines 'tools', which is a \
         key of the grant the engine owns"
    );
}

/// Ruling 8 for the compiler: a schema contained in its file and clear of
/// the engine's keys is still compiled whole, and the compiler's refusal —
/// whose own words would repeat an authored pointer or value — is named
/// by its typed fault alone. A local reference hidden in data passes
/// containment and reaches the compiler, which follows it.
#[test]
fn a_schema_the_compiler_refuses_is_named_by_its_fault_alone() {
    use crate::capabilities::dialect::embedded_schema_fault;
    let root = TempDir::new_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let shapes = [
        json!({"type": "hunter2"}),
        json!({"$ref": "#/const", "const": {"$ref": "#/definitions/hunter2"}}),
    ];
    let seen = shapes.clone().map(|schema| {
        let compiler = jsonschema::draft7::new(&schema).unwrap_err().to_string();
        let mut d = native_dialect("d", "web-search", &["lookup"]);
        d["restrictions"] = schema.clone();
        dialect(root.path(), &d);
        let loaded = ToolDialect::load(root.path(), "d").unwrap_err();
        (
            embedded_schema_fault(&schema),
            compiler.contains("hunter2"),
            loaded,
        )
    });
    let uncompiled = "tool dialect 'dialects/tools/d.json' restriction schema is not valid \
                      draft-07: the validator does not compile it";
    assert_eq!(seen, [0, 1].map(|_| (None, true, uncompiled.to_string())));
}

/// The fail-closed side of SC2's strip: a v8 grant's `retain` is removed
/// before its restrictions are judged, so a dialect that requires it only
/// indirectly refuses the grant — it is never satisfied by the veto.
#[test]
fn an_indirect_requirement_of_retain_fails_a_v8_grant_closed() {
    let root = cq1_root();
    let requires = json!({"allOf": [{"required": ["retain"]}]});
    let veto = json!({"dialect": "claims-retain", "retain": false});
    assert_eq!(
        claims_under(root.path(), &requires, "forge.realms/v8", &veto),
        Err(
            "realm 'private' grants capability 'web-search' through dialect 'claims-retain' \
             with an invalid restriction at '': it does not satisfy '/allOf/0/required'"
                .to_string()
        )
    );
}

/// SC2 and D11: an older map's `retain` is never the newer veto — it is a
/// nonempty restriction, and reaches only the restriction outcomes.
#[test]
fn an_older_retain_stays_a_restriction_and_reaches_only_the_nonempty_restriction_outcomes() {
    let root = cq1_root();
    let mut keeps = native_dialect("keeps-retain", "web-search", &["lookup", "search"]);
    keeps["restrictions"] = json!({"type": "object", "additionalProperties": false,
                                   "properties": {"retain": {"type": "boolean"}}});
    dialect(root.path(), &keeps);
    let native = switchable();
    let (through, cause) = (
        "through dialect 'keeps-retain'",
        "provider 'test-native' cannot express restriction 'retain'",
    );
    for schema in ["forge.realms/v6", "forge.realms/v7"] {
        let written = json!({"web-search": {"dialect": "keeps-retain", "retain": false}});
        let loaded = Authority::load(context_at(schema, root.path(), written));
        let read = loaded.as_ref().map(|loaded| {
            let grant = &loaded.context.grants["web-search"];
            (grant.retention, Value::Object(grant.restrictions.clone()))
        });
        assert_eq!(
            read,
            Ok((GrantRetention::Unreserved, json!({"retain": false})))
        );
        let loaded = loaded.unwrap();
        let resolve = |requests| loaded.resolve(&asks(requests), &serving(&native));
        assert_eq!(
            resolve(json!({"web-search": "requires"})).unwrap_err(),
            format!(
                "{WHO}: requires capability 'web-search' {through}, but {cause}; the capability \
                 cannot be held under this grant"
            )
        );
        let dropped = resolve(json!({"web-search": "wants"})).unwrap();
        assert_eq!(
            (dropped.held.len(), dropped.notices[0].1.as_str()),
            (
                0,
                format!(
                    "{WHO}: dropped wanted capability 'web-search' {through} because {cause}; \
                     native capability remains OFF"
                )
                .as_str()
            )
        );
    }
}

/// CR1's four outcomes: only a dialect that retains, under a grant that
/// does not veto, retains. A native holding declares nothing, carries its
/// grant's veto anyway, and a veto on a grant no office reaches stays
/// pinned and inactive.
#[test]
fn a_holding_carries_the_grants_veto_and_only_a_retaining_dialect_without_one_retains() {
    let root = cq1_root();
    let docs = |retained: Option<bool>| {
        let mut docs = mcp_dialect("docs-mcp", json!({"argv": ["/nonexistent/docs-mcp"]}));
        docs["retained"] = json!(retained);
        if retained.is_none() {
            docs.as_object_mut().unwrap().remove("retained");
        }
        dialect(root.path(), &docs);
        ToolDialect::load(root.path(), "docs-mcp").unwrap()
    };
    let grant = |schema: &str, written: Value| {
        context_at(schema, root.path(), json!({"library-docs": written})).grants["library-docs"]
            .clone()
    };
    let inherit = grant("forge.realms/v8", json!({"dialect": "docs-mcp"}));
    let veto = grant(
        "forge.realms/v8",
        json!({"dialect": "docs-mcp", "retain": false}),
    );
    let older = grant("forge.realms/v6", json!({"dialect": "docs-mcp"}));
    let outcomes: Vec<_> = [
        (None, &inherit),
        (None, &veto),
        (Some(true), &inherit),
        (Some(true), &veto),
        (Some(false), &inherit),
        (Some(true), &older),
    ]
    .into_iter()
    .map(|(retained, grant)| {
        let retention = Retention::of(&docs(retained), grant);
        (retention.declared, retention.realm, retention.effective())
    })
    .collect();
    use GrantRetention::{Inherit, Unreserved, Veto};
    assert_eq!(
        outcomes,
        [
            (false, Inherit, false),
            (false, Veto, false),
            (true, Inherit, true),
            (true, Veto, false),
            (false, Inherit, false),
            (true, Unreserved, true),
        ]
    );
    let native = switchable();
    let v8 = |written: Value| {
        Authority::load(context_at(
            "forge.realms/v8",
            root.path(),
            json!({"web-search": written}),
        ))
        .unwrap()
    };
    let vetoed = v8(json!({"dialect": "search-native", "retain": false}));
    let held = vetoed.resolve(&asks(json!({"web-search": "requires"})), &serving(&native));
    let retention = held.unwrap().held["web-search"].retention;
    assert_eq!((retention.realm, retention.effective()), (Veto, false));
    let inherited = v8(json!({"dialect": "search-native"}));
    let held = inherited.resolve(&asks(json!({"web-search": "requires"})), &serving(&native));
    let retention = held.unwrap().held["web-search"].retention;
    assert_eq!((retention.realm, retention.effective()), (Inherit, false));
    // The veto is identity even where it changes nothing, and on a grant
    // no office reaches it stays pinned while the grant stays idle.
    let idle = v8(json!({"dialect": "search-native", "retain": false, "offices": []}));
    let outcome = idle.resolve(&asks(json!({"web-search": "wants"})), &serving(&native));
    assert_eq!(
        outcome.unwrap().not_held["web-search"],
        "the realm grants it to no office"
    );
    assert_eq!(
        idle.manifest(&[])["grants"]["web-search"],
        json!({"dialect": "search-native", "offices": [], "retain": false})
    );
    assert_ne!(
        vetoed.manifest(&[])["grants"],
        inherited.manifest(&[])["grants"]
    );
}

/// SC1 past the fence: an `mcp` connection Brokkr cannot execute answers
/// with its own compatibility cause, in GP1's required and optional forms,
/// while the fence's words stay what an executable one meets until U9b.
#[test]
fn past_the_fence_an_unexecutable_connection_answers_with_its_own_cause() {
    let native = switchable();
    for (connection, cause) in [
        (
            json!({"url": "https://docs.invalid/mcp"}),
            "MCP URL connections are not implemented in decision 0065 slice two",
        ),
        (
            json!({"argv": ["docs-mcp", "--token={{secret:DOCS_TOKEN}}"]}),
            "MCP connection argv cannot contain secret references; declare environment bindings \
             in secrets",
        ),
        (json!({"argv": ["/nonexistent/docs-mcp"]}), MCP),
    ] {
        let root = cq1_root();
        let authority = past_the_fence(root.path(), connection);
        let resolve = |requests| authority.resolve(&asks(requests), &serving(&native));
        assert_eq!(
            resolve(json!({"library-docs": "requires"})).unwrap_err(),
            format!(
                "{WHO}: requires capability 'library-docs' through dialect 'docs-mcp', but \
                 {cause}; the capability cannot be held under this grant"
            )
        );
        assert_eq!(
            resolve(json!({"library-docs": "wants"})).unwrap().notices[0].1,
            format!(
                "{WHO}: dropped wanted capability 'library-docs' through dialect 'docs-mcp' \
                 because {cause}; no native denial is claimed"
            )
        );
        // Granted for real, the fence still answers first.
        let fenced = refusal(
            root.path(),
            json!({"library-docs": {"dialect": "docs-mcp"}}),
        );
        assert_eq!(fenced, MCP);
    }
}

/// SC4: an `mcp` holding's identity is carried whole or refused — never
/// truncated — before its connection is judged. A native holding is held
/// to the same bound where its attribution is compiled (U4d).
#[test]
fn an_mcp_or_native_identity_is_carried_whole_or_refused() {
    let root = cq1_root();
    let search = ToolDialect::load(root.path(), "search-native").unwrap();
    let named = |name: String| ToolDialect {
        name,
        ..search.clone()
    };
    let fits = |capability: &str, dialect: &ToolDialect, tool: &str| {
        crate::capabilities::binding::representable(capability, dialect, &[tool.to_string()])
    };
    let (name, long) = ("n".repeat(128), "n".repeat(129));
    let (tool, overlong) = (format!("a{}", "._:/-".repeat(51)), "t".repeat(257));
    assert_eq!(tool.len(), 256);
    let refused = Err(Unserved::Identity);
    assert_eq!(
        [
            fits(&name, &named(name.clone()), &tool),
            fits(&long, &search, "lookup"),
            fits("web-search", &named(long.clone()), "lookup"),
            fits("web-search", &search, &overlong),
            fits("web-search", &search, "web search"),
            fits("web-search", &search, "_lookup"),
        ],
        [
            Ok(()),
            refused.clone(),
            refused.clone(),
            refused.clone(),
            refused.clone(),
            refused
        ]
    );
    let identity = "capability call identity cannot be represented by seat-record v6";
    let native = switchable();
    dialect(
        root.path(),
        &native_dialect(&long, "web-search", &["lookup", "search"]),
    );
    let granted = authority(root.path(), json!({"web-search": {"dialect": long}}));
    let held = granted.resolve(&asks(json!({"web-search": "requires"})), &serving(&native));
    let context = format!("(capability 'web-search' through dialect '{long}')");
    assert_eq!(held, Err(format!("{WHO}: {identity} {context}")));
    // Past the fence, an unrepresentable identity refuses the compile
    // whatever the ask's strength: a wanted capability is never dropped
    // over it (SC4).
    let mut docs = past_the_fence(root.path(), json!({"url": "https://docs.invalid/mcp"}));
    docs.dialects.get_mut("library-docs").unwrap().tools = vec![overlong];
    let refused = ["wants", "requires"]
        .map(|strength| docs.resolve(&asks(json!({"library-docs": strength})), &serving(&native)));
    assert_eq!(
        refused.map(|outcome| outcome.map(|_| ())),
        [
            Err(format!("{WHO}: {identity}")),
            Err(format!("{WHO}: {identity}"))
        ]
    );
}

/// SC4's tool vocabulary is seat-record's own: an identifier `representable`
/// admits is exactly one the published `tool` pattern admits, checked for
/// every ASCII character in the first place and in a later one. The byte
/// bound is SC4's 256, which seat-record v6 widens v5's 80 to.
#[test]
fn the_tool_vocabulary_is_seat_records_own() {
    let published: Value = serde_json::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../contracts/seat-record.v6.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let pattern = &published["definitions"]["checkpoint"]["properties"]["tool"]["pattern"];
    let contract = jsonschema::draft7::new(&json!({"type": "string", "pattern": pattern})).unwrap();
    let root = cq1_root();
    let search = ToolDialect::load(root.path(), "search-native").unwrap();
    let probes: Vec<String> = (0..=127u8)
        .map(char::from)
        .flat_map(|c| [c.to_string(), format!("a{c}")])
        .chain(["é".to_string(), "aé".to_string()])
        .collect();
    let disagree: Vec<_> = probes
        .iter()
        .filter(|tool| {
            let held = crate::capabilities::binding::representable(
                "web-search",
                &search,
                &[tool.to_string()],
            );
            held.is_ok() != contract.is_valid(&json!(tool))
        })
        .collect();
    assert_eq!((probes.len(), disagree), (258, Vec::<&String>::new()));
}

/// The U2 residual: an [`Authority`] is constructed only by its loaders.
/// Its private field keeps a struct literal out of every other module, and
/// this accounts for every production construction there is.
#[test]
fn only_the_loaders_construct_an_authority() {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut sources = Vec::new();
    production_sources(crates_dir, &mut sources);
    let mut constructions: Vec<(String, String)> = Vec::new();
    for (file, content) in &sources {
        let literal = |before: &&str| {
            !["impl ", "struct ", "-> "]
                .iter()
                .any(|word| before.ends_with(word))
        };
        let prefixes = content
            .match_indices("Authority {")
            .map(|(at, _)| &content[..at]);
        for before in prefixes.filter(literal) {
            // A construction sits in the nearest function opened before it.
            let opened = before.rfind("fn ").map_or("", |at| &before[at + 3..]);
            let function = opened.split('(').next().unwrap_or_default().to_string();
            constructions.push((
                file.strip_prefix(crates_dir).unwrap().display().to_string(),
                function,
            ));
        }
    }
    constructions.sort();
    let file = "brokkr-runtime/src/capabilities.rs".to_string();
    assert_eq!(
        constructions,
        [
            (file.clone(), "load".to_string()),
            (file, "nothing".to_string())
        ]
    );
}

/// Every production Rust source under `dir`, with its text: a crate's
/// `src/` tree, minus test modules and `tests/` directories.
fn production_sources(dir: &Path, sources: &mut Vec<(PathBuf, String)>) {
    for path in std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
    {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let test = name == "tests.rs" || name.ends_with("_tests.rs");
        if path.is_dir() && !["tests", "target"].contains(&name.as_str()) {
            production_sources(&path, sources);
        } else if name.ends_with(".rs") && !test && path.to_string_lossy().contains("/src/") {
            let text = std::fs::read_to_string(&path).unwrap();
            sources.push((path, text));
        }
    }
}
