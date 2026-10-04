//! SC5 (decision 0065 slice two, U2): a grant's binding is read by its
//! dialect's kind, never indexed or assumed native. The compile fence
//! refuses every `mcp` grant before resolution, so these authorities are
//! built past it, at the resolver's own seam; the fence's refusal is
//! pinned unchanged in `an_mcp_grant_refuses_until_slice_two_…`.

use super::*;

/// The `mcp` binding's cause, pinned once for this module.
const MCP: &str = "realm 'private' grants capability 'library-docs' through dialect 'docs-mcp' \
                   of kind 'mcp', whose broker support is not implemented until decision 0065 \
                   slice two";

/// The cause of a capability whose dialect is not loaded.
const MISSING: &str = "realm 'private' has no loaded dialect for capability 'web-search', so it \
                       is bound to no provider";

/// The site `research` of office `researcher`, as the resolver opens on it.
const WHO: &str = "seat 'research' (office 'researcher') in realm 'private'";

/// `test-native`'s `web-search` grant, beside a structurally valid
/// `library-docs` grant through the `mcp` dialect `docs-mcp`, inserted
/// past the compile fence.
fn past_the_fence(root: &Path) -> Authority {
    define(root, "library-docs", &["reads", "egress"]);
    let docs = mcp_dialect("docs-mcp", json!({"argv": ["/nonexistent/docs-mcp"]}));
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
    let authority = past_the_fence(root.path());
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
