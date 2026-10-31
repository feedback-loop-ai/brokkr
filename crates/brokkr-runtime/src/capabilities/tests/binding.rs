//! SC5 (decision 0065 slice two, U2): a grant's binding is read by its
//! dialect's kind, never indexed or assumed native. The compile fence
//! refuses every `mcp` grant before resolution, so these authorities are
//! built past it, at the resolver's own seam; the fence's refusal is
//! pinned unchanged in `an_mcp_grant_refuses_until_slice_two_…`.

use super::*;

/// The `mcp` binding's cause, pinned once for this module.
pub(crate) const MCP: &str =
    "realm 'private' grants capability 'library-docs' through dialect 'docs-mcp' \
                   of kind 'mcp', whose broker support is not implemented until decision 0065 \
                   slice two";

/// The cause of a capability whose dialect is not loaded.
const MISSING: &str = "realm 'private' has no loaded dialect for capability 'web-search', so it \
                       is bound to no provider";

/// The site `research` of office `researcher`, as the resolver opens on it.
pub(super) const WHO: &str = "seat 'research' (office 'researcher') in realm 'private'";

/// `test-native`'s `web-search` grant, beside a structurally valid
/// `library-docs` grant through the `mcp` dialect `docs-mcp`, reached by
/// `connection`, inserted past the compile fence. Its egress meets the
/// absent binding minimum, so what these proofs reach is the cause they
/// name; MB4's comparison is proved below on its own.
pub(super) fn past_the_fence(root: &Path, connection: Value) -> Authority {
    define(root, "library-docs", &["reads", "egress"]);
    let mut docs = mcp_dialect("docs-mcp", connection);
    docs["egress"] = json!("contracted");
    dialect(root, &docs);
    let mut authority = bound_to_test_native(root);
    let grants = context(root, json!({"library-docs": {"dialect": "docs-mcp"}})).grants;
    authority.context.grants.extend(grants);
    let docs = ToolDialect::load(root, "docs-mcp").unwrap();
    authority.dialects.insert("library-docs".into(), docs);
    authority
}

#[test]
fn an_mcp_grant_at_the_resolver_is_a_typed_refusal_and_never_a_native_binding() {
    let root = cq1_root();
    let authority = past_the_fence(root.path(), json!({"argv": ["/nonexistent/docs-mcp"]}));
    assert_eq!(
        authority.binding("library-docs"),
        Err(Unbound::Mcp {
            realm: "private".into(),
            capability: "library-docs".into(),
            dialect: "docs-mcp".into(),
        })
    );
    assert_eq!(
        authority.binding("web-search"),
        Ok(Native {
            dialect: &authority.dialects["web-search"],
            provider: "test-native",
            adapter_key: "web-search",
        })
    );
    let native = switchable();
    let resolve = |requests| authority.resolve(&asks(requests), &serving(&native));
    assert_eq!(
        resolve(json!({"library-docs": "requires"})).unwrap_err(),
        format!(
            "{WHO}: requires capability 'library-docs' through dialect 'docs-mcp', but {MCP}; \
             the capability cannot be held under this grant"
        )
    );
    // A want drops; the native control beside it is held and switched ON.
    let outcome = resolve(json!({"library-docs": "wants", "web-search": "wants"})).unwrap();
    assert_eq!(outcome.held.keys().collect::<Vec<_>>(), ["web-search"]);
    assert_eq!(
        outcome.notices,
        [(
            "library-docs".to_string(),
            format!(
                "{WHO}: dropped wanted capability 'library-docs' through dialect 'docs-mcp' \
                 because {MCP}; no native denial is claimed"
            )
        )]
    );
    assert_eq!(argv_of(&outcome), ["--search-on"]);
}

#[test]
fn a_missing_binding_is_a_typed_refusal_and_never_a_panic() {
    let root = cq1_root();
    let mut authority = bound_to_test_native(root.path());
    authority.dialects.remove("web-search");
    assert_eq!(
        authority.binding("web-search"),
        Err(Unbound::Missing {
            realm: "private".into(),
            capability: "web-search".into(),
        })
    );
    let native = switchable();
    let resolve = |requests| authority.resolve(&asks(requests), &serving(&native));
    assert_eq!(
        resolve(json!({"web-search": "requires"})).unwrap_err(),
        format!(
            "{WHO}: requires capability 'web-search' through dialect 'search-native', but \
             {MISSING}; the capability cannot be held under this grant"
        )
    );
    let outcome = resolve(json!({"web-search": "wants"})).unwrap();
    assert!(outcome.held.is_empty());
    assert_eq!(
        outcome.notices,
        [(
            "web-search".to_string(),
            format!(
                "{WHO}: dropped wanted capability 'web-search' through dialect 'search-native' \
                 because {MISSING}; native capability remains OFF"
            )
        )]
    );
    assert_eq!(argv_of(&outcome), ["--search-off"]);
}

/// MB4's cause for `docs-mcp` at `egress` under `minimum`.
fn below(egress: &str, minimum: &str) -> String {
    format!("MCP dialect 'docs-mcp' has egress '{egress}' below binding minimum '{minimum}'")
}

/// `docs-mcp` past the fence at `egress`, judged against the bundle's
/// `minimum`, or against the absent one where none is bound.
fn docs_at(root: &Path, egress: &str, minimum: Option<&str>) -> Authority {
    let class = |word| crate::agents::EgressClass::parse(word).unwrap();
    let mut authority = past_the_fence(root, json!({"argv": ["/nonexistent/docs-mcp"]}));
    authority.dialects.get_mut("library-docs").unwrap().egress = class(egress);
    match minimum {
        Some(minimum) => authority.with_minimum(class(minimum)),
        None => authority,
    }
}

/// What `requires` and `wants` of `library-docs` come to: the refusal,
/// and what the want still holds beside its notices.
type Judged = (String, Vec<String>, Vec<(String, String)>);

fn judged(authority: &Authority) -> Judged {
    judged_under(authority, &[])
}

/// [`judged`], served by a candidate whose harness authenticates with
/// `auth`.
fn judged_under(authority: &Authority, auth: &[String]) -> Judged {
    let native = switchable();
    let candidate = Serving {
        auth,
        ..serving(&native)
    };
    let resolve =
        |strength| authority.resolve(&asks(json!({"library-docs": strength})), &candidate);
    let wanted = resolve("wants").unwrap();
    (
        resolve("requires").unwrap_err(),
        wanted.held.keys().cloned().collect(),
        wanted.notices,
    )
}

/// `docs-mcp` lost for `cause`, in GP1's required and optional forms.
fn lost(cause: &str) -> Judged {
    let notice = format!(
        "{WHO}: dropped wanted capability 'library-docs' through dialect 'docs-mcp' because \
         {cause}; no native denial is claimed"
    );
    (
        format!(
            "{WHO}: requires capability 'library-docs' through dialect 'docs-mcp', but {cause}; \
             the capability cannot be held under this grant"
        ),
        Vec::new(),
        vec![("library-docs".to_string(), notice)],
    )
}

/// `docs-mcp` lost for a `cause` that names its dialect itself (MB4's
/// egress comparison, D3's deferred restriction): both of GP1's forms end
/// at the cause, with no incompatibility suffix and no native tail (U5a2
/// F1, U9b1).
pub(super) fn closed(cause: &str) -> Judged {
    let notice = format!("{WHO}: dropped wanted capability 'library-docs' because {cause}");
    (
        format!("{WHO}: requires capability 'library-docs' but {cause}"),
        Vec::new(),
        vec![("library-docs".to_string(), notice)],
    )
}

/// MB4 (U5a2): every minimum against every egress. Below it, a requires
/// refuses and a want drops with MB4's exact cause; at or above it the
/// comparison passes, and where the fence stands, as it does on macOS and
/// on this thread with no `Lift` held, the fence's own words are what the
/// dialect meets next. Every row is judged before the one comparison, so
/// each row binds on its own.
#[test]
fn an_mcp_dialect_below_the_binding_minimum_is_refused_or_dropped_and_at_or_above_it_passes() {
    let root = cq1_root();
    let rows = [
        ("local", "local", "at"),
        ("local", "contracted", "below"),
        ("local", "uncontracted", "below"),
        ("contracted", "local", "above"),
        ("contracted", "contracted", "at"),
        ("contracted", "uncontracted", "below"),
        ("uncontracted", "local", "above"),
        ("uncontracted", "contracted", "above"),
        ("uncontracted", "uncontracted", "at"),
    ];
    let observed: Vec<_> = rows
        .iter()
        .map(|&(minimum, egress, relation)| {
            let judged = judged(&docs_at(root.path(), egress, Some(minimum)));
            (minimum, egress, relation, judged)
        })
        .collect();
    let expected: Vec<_> = rows
        .iter()
        .map(|&(minimum, egress, relation)| {
            let judged = match relation {
                "below" => closed(&below(egress, minimum)),
                _ => lost(MCP),
            };
            (minimum, egress, relation, judged)
        })
        .collect();
    assert_eq!(observed, expected);
}

/// A bundle that binds no minimum is judged at `contracted`, the minimum
/// its absence has always meant (decision 0036 ruling 4).
#[test]
fn an_absent_binding_minimum_is_contracted() {
    let root = cq1_root();
    let observed = ["local", "contracted", "uncontracted"]
        .map(|egress| judged(&docs_at(root.path(), egress, None)));
    assert_eq!(
        observed,
        [
            lost(MCP),
            lost(MCP),
            closed(&below("uncontracted", "contracted"))
        ]
    );
}

/// The receipt's policy digest of each binding minimum: the sha256 of the
/// canonical bytes of exactly the minimum's word, computed outside Brokkr.
fn policy(minimum: &str) -> String {
    match minimum {
        "local" => "ea17e25c6c7aa6ef9407f976a670e87e37bc7e2a86b7bd4a3305bb16f1ba6052",
        "contracted" => "b42c5e32cc3ac505924a63bfaddd62d346a5dd8a70f98c9a998eb63c034acca9",
        "uncontracted" => "6929c044957c06bc18e8bab5bbbdc1659d915e7ad97b10ce5e485a4876ab40f3",
        other => panic!("no binding minimum '{other}'"),
    }
    .to_string()
}

/// U7c0: where MB4's comparison admits `docs-mcp`, it seals the network
/// its egress projects to (`local` isolated, every other route shared)
/// and the receipt of the dialect's digest and the minimum it read; below
/// the minimum it seals nothing and names MB4's cause. Each fact follows
/// its own input alone: egress moves only the network, the minimum only
/// the policy digest, the dialect's digest only the receipt's dialect.
#[test]
fn an_admitted_mcp_dialect_seals_its_network_and_receipt_and_each_follows_its_own_input() {
    use brokkr_protocol::broker::{Clearance, Network};
    let root = cq1_root();
    let class = |word| crate::agents::EgressClass::parse(word).unwrap();
    let admitted =
        |authority: &Authority| admitted(&authority.dialects["library-docs"], authority.minimum());
    let digest = docs_at(root.path(), "local", None).dialects["library-docs"]
        .sha256
        .clone();
    let sealed = |network, minimum, dialect: &str| {
        Ok(Admitted {
            network,
            clearance: Clearance {
                dialect: dialect.to_string(),
                policy: policy(minimum),
            },
        })
    };
    let words = ["local", "contracted", "uncontracted"];
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for minimum in words {
        for egress in words {
            let authority = docs_at(root.path(), egress, Some(minimum));
            observed.push((minimum, egress, admitted(&authority)));
            expected.push((
                minimum,
                egress,
                match class(egress) < class(minimum) {
                    true => Err(Below {
                        dialect: "docs-mcp".into(),
                        egress: class(egress),
                        minimum: class(minimum),
                    }),
                    false if egress == "local" => sealed(Network::Isolated, minimum, &digest),
                    false => sealed(Network::Shared, minimum, &digest),
                },
            ));
        }
    }
    assert_eq!(observed, expected);
    // The absent minimum's receipt names `contracted`, the minimum it means.
    let absent = docs_at(root.path(), "contracted", None);
    assert_eq!(
        admitted(&absent),
        sealed(Network::Shared, "contracted", &digest)
    );
    // Other dialect bytes move the receipt's dialect alone.
    let mut moved = docs_at(root.path(), "local", Some("local"));
    moved.dialects.get_mut("library-docs").unwrap().sha256 = "e".repeat(64);
    assert_eq!(
        admitted(&moved),
        sealed(Network::Isolated, "local", &"e".repeat(64))
    );
}

/// U7c0 on the resolver's own path: the holding `sealed` builds for an
/// admitted `docs-mcp` grant carries exactly what MB4's comparison
/// admitted, its network following the dialect's egress and its receipt
/// the minimum read; a grant below the minimum seals no holding and names
/// MB4's cause; and where the fence stands, as it does on macOS and on this
/// thread with no `Lift` held, the fence's own words follow the sealed
/// holding.
#[test]
fn the_resolver_seals_an_admitted_mcp_grant_on_its_holding_and_the_fence_still_follows_it() {
    use brokkr_protocol::broker::{Clearance, Network};
    let root = cq1_root();
    let site = asks(json!({"library-docs": "requires"}));
    let resolver = docs_at(root.path(), "local", None);
    let required = |cause: Cause| cause.required(resolver.who(&site), "library-docs");
    let sealed =
        |egress, minimum| docs_at(root.path(), egress, minimum).sealed(&site, "library-docs");
    let digest = docs_at(root.path(), "local", None).dialects["library-docs"]
        .sha256
        .clone();
    let admitted = |network, minimum| Admitted {
        network,
        clearance: Clearance {
            dialect: digest.clone(),
            policy: policy(minimum),
        },
    };
    let observed = [
        ("local", Some("local")),
        ("contracted", None),
        ("uncontracted", Some("uncontracted")),
    ]
    .map(|(egress, minimum)| match sealed(egress, minimum) {
        Ok(holding) => (
            holding.admitted.clone(),
            holding
                .native("private", "library-docs", McpFence::current())
                .err(),
        ),
        Err(_) => panic!("docs-mcp at {egress} is admitted under {minimum:?}"),
    });
    let fenced = |held: Option<Unheld>| match held {
        Some(Unheld::Cause(cause)) => required(cause),
        _ => panic!("the fence's words follow a sealed mcp holding"),
    };
    let observed = observed.map(|(admitted, fence)| (admitted, fenced(fence)));
    let fence = lost(MCP).0;
    assert_eq!(
        observed,
        [
            (Some(admitted(Network::Isolated, "local")), fence.clone()),
            (Some(admitted(Network::Shared, "contracted")), fence.clone()),
            (Some(admitted(Network::Shared, "uncontracted")), fence),
        ]
    );
    let below_minimum = match sealed("contracted", Some("local")) {
        Err(Unheld::Cause(cause)) => required(cause),
        _ => panic!("docs-mcp at contracted is below local"),
    };
    assert_eq!(below_minimum, closed(&below("contracted", "local")).0);
}

/// A native holding is never judged by MB4, so it carries no admitted
/// facts under any minimum, its dialect's egress meeting it or not.
#[test]
fn a_native_holding_carries_no_admitted_facts_under_any_binding_minimum() {
    use crate::agents::EgressClass::{Contracted, Local, Uncontracted};
    let root = cq1_root();
    let native = switchable();
    let admitted: Vec<_> = [Local, Contracted, Uncontracted]
        .map(|minimum| {
            let authority = bound_to_test_native(root.path()).with_minimum(minimum);
            let outcome = authority
                .resolve(&asks(json!({"web-search": "requires"})), &serving(&native))
                .unwrap();
            outcome.held["web-search"].admitted.clone()
        })
        .into();
    assert_eq!(admitted, [None, None, None]);
}

/// MB4 judges an `mcp` dialect alone: a native holding, whose dialect is
/// `uncontracted`, is what slice one made it under every minimum.
#[test]
fn a_native_holding_is_unchanged_under_every_binding_minimum() {
    use crate::agents::EgressClass::{Contracted, Local, Uncontracted};
    let root = cq1_root();
    let native = switchable();
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for minimum in [Local, Contracted, Uncontracted] {
        let authority = bound_to_test_native(root.path()).with_minimum(minimum);
        assert_eq!(authority.dialects["web-search"].egress, Uncontracted);
        for strength in ["requires", "wants"] {
            let outcome = authority
                .resolve(&asks(json!({"web-search": strength})), &serving(&native))
                .unwrap();
            let held: Vec<&String> = outcome.held.keys().collect();
            let shown = format!("{held:?} {:?} {:?}", outcome.notices, argv_of(&outcome));
            observed.push((minimum, strength, shown));
            let unchanged = r#"["web-search"] [] ["--search-on"]"#.to_string();
            expected.push((minimum, strength, unchanged));
        }
    }
    assert_eq!(observed, expected);
}

/// MB4's collision cause for `name` (U7d3), pinned once.
pub(crate) fn collides(name: &str) -> String {
    format!("MCP secret binding '{name}' collides with harness authentication")
}

/// `docs-mcp` past the fence binding `secrets`: its egress meets the absent
/// minimum, so what follows MB4's collision check is the fence's words.
pub(crate) fn binding_secrets(root: &Path, secrets: &[&str]) -> Authority {
    let mut authority = past_the_fence(root, json!({"argv": ["/nonexistent/docs-mcp"]}));
    let docs = authority.dialects.get_mut("library-docs").unwrap();
    let DialectKind::Mcp(server) = &mut docs.kind else {
        panic!("docs-mcp is an mcp dialect")
    };
    server.secrets = secrets.iter().map(|name| name.to_string()).collect();
    authority
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

/// MB4 at the serving candidate (U7d3): a secret the dialect binds that the
/// candidate's harness reads to authenticate refuses a requires and drops a
/// want with MB4's exact cause, whichever of its secrets it is. A name that
/// collides with nothing, a harness that declares no name and a dialect that
/// binds none pass the check, and the fence's own words are what follows.
#[test]
fn a_secret_colliding_with_harness_authentication_refuses_or_drops_and_any_other_passes() {
    let root = cq1_root();
    let rows: [(&[&str], &[&str], String); 5] = [
        (&["DOCS_TOKEN"], &["DOCS_TOKEN"], collides("DOCS_TOKEN")),
        (
            &["DOCS_TOKEN", "SECOND_TOKEN"],
            &["OTHER_KEY", "SECOND_TOKEN"],
            collides("SECOND_TOKEN"),
        ),
        (&["DOCS_TOKEN"], &["OTHER_KEY"], MCP.to_string()),
        (&["DOCS_TOKEN"], &[], MCP.to_string()),
        (&[], &["DOCS_TOKEN"], MCP.to_string()),
    ];
    let observed: Vec<Judged> = rows
        .iter()
        .map(|(secrets, auth, _)| {
            judged_under(&binding_secrets(root.path(), secrets), &names(auth))
        })
        .collect();
    let expected: Vec<Judged> = rows.iter().map(|(_, _, cause)| lost(cause)).collect();
    assert_eq!(observed, expected);
}

/// Each shipped adapter's authentication names (U7d3), as an independent
/// literal: lanetally runs claude's harness and reads claude's names, dsh's
/// are its route credentials, and exec reads none.
const SHIPPED: [(&str, &[&str]); 5] = [
    ("claude", &CLAUDE_AUTH),
    (
        "codex",
        &["CODEX_ACCESS_TOKEN", "CODEX_API_KEY", "OPENAI_API_KEY"],
    ),
    (
        "dsh",
        &[
            "DASHSCOPE_API_KEY",
            "DEEPSEEK_API_KEY",
            "OPENROUTER_API_KEY",
            "SPARK_API_KEY",
        ],
    ),
    ("exec", &[]),
    ("lanetally", &CLAUDE_AUTH),
];

/// Claude Code 2.1.299's seven provider credential variables, and the
/// twelve AWS, GCP and Azure credential-chain names U9b1's operator ruling
/// 5 adds (U1d3).
const CLAUDE_AUTH: [&str; 19] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_AWS_API_KEY",
    "ANTHROPIC_FOUNDRY_API_KEY",
    "ANTHROPIC_FOUNDRY_AUTH_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_BEARER_TOKEN_BEDROCK",
    "AWS_PROFILE",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AZURE_CLIENT_CERTIFICATE_PASSWORD",
    "AZURE_CLIENT_CERTIFICATE_PATH",
    "AZURE_CLIENT_ID",
    "AZURE_CLIENT_SECRET",
    "AZURE_PASSWORD",
    "AZURE_TENANT_ID",
    "AZURE_USERNAME",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "GOOGLE_APPLICATION_CREDENTIALS",
];

/// The shipped adapters, loaded as a compile loads them.
fn shipped() -> crate::agents::Adapters {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters");
    crate::agents::Adapters::load(&root).unwrap()
}

/// For each shipped adapter (U7d3): it loads exactly its measured names; a
/// dialect binding each of them is refused or dropped with MB4's cause when
/// that adapter serves; and a name it does not read, another adapter's
/// included, collides with nothing, so an adapter that declares no name
/// collides with nothing at all.
#[test]
fn each_shipped_adapter_collides_with_its_own_authentication_names_alone() {
    let root = cq1_root();
    let shipped = shipped();
    let loaded: Vec<(&str, Vec<String>)> = SHIPPED
        .map(|(provider, _)| {
            (
                provider,
                shipped.adapter(provider).unwrap().harness_auth.clone(),
            )
        })
        .into();
    let declared: Vec<(&str, Vec<String>)> =
        SHIPPED.map(|(provider, own)| (provider, names(own))).into();
    assert_eq!(loaded, declared);
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for (provider, own) in SHIPPED {
        let auth = &shipped.adapter(provider).unwrap().harness_auth;
        for secret in own.iter().chain(&["DOCS_TOKEN", "DEEPSEEK_API_KEY"]) {
            let judged = judged_under(&binding_secrets(root.path(), &[secret]), auth);
            observed.push((provider, *secret, judged));
            let cause = match own.contains(secret) {
                true => collides(secret),
                false => MCP.to_string(),
            };
            expected.push((provider, *secret, lost(&cause)));
        }
    }
    assert_eq!(observed, expected);
}

/// The serving adapter's names reach the check through the adapter-level
/// assessment `brokkr doctor` submits too: the shipped dsh adapter's route
/// key, bound by the dialect, refuses its plan with MB4's cause, and a key
/// it does not read leaves the fence's own words.
#[test]
fn an_adapter_assessment_judges_the_collision_by_that_adapters_names() {
    let root = cq1_root();
    let shipped = shipped();
    let dsh = shipped.adapter("dsh").unwrap();
    let requests = json!({"library-docs": "requires"});
    let requests = parse_requests("office 'researcher'", &requests).unwrap();
    let assessed = |secret| {
        let authority = binding_secrets(root.path(), &[secret]);
        authority
            .assess(dsh, "researcher", requests.clone())
            .unwrap_err()
    };
    let refused = |cause: &str| {
        format!(
            "seat 'adapter-plan' (office 'researcher') in realm 'private': requires capability \
             'library-docs' through dialect 'docs-mcp', but {cause}; the capability cannot be \
             held under this grant"
        )
    };
    assert_eq!(
        [assessed("DEEPSEEK_API_KEY"), assessed("DOCS_TOKEN")],
        [refused(&collides("DEEPSEEK_API_KEY")), refused(MCP)]
    );
}
