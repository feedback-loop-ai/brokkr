//! SC5 (decision 0065 slice two, U2): a grant's binding is read by its
//! dialect's kind, never indexed or assumed native. The compile fence
//! refuses every `mcp` grant before resolution, so these authorities are
//! built past it, at the resolver's own seam; the fence's refusal is
//! pinned unchanged in `an_mcp_grant_refuses_until_slice_two_…`.

use super::*;

/// The `mcp` binding's cause, pinned once for this module.
pub(super) const MCP: &str =
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
    let native = switchable();
    let resolve =
        |strength| authority.resolve(&asks(json!({"library-docs": strength})), &serving(&native));
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

/// MB4 (U5a2): every minimum against every egress. Below it, a requires
/// refuses and a want drops with MB4's exact cause; at or above it the
/// comparison passes, and until U9b builds the broker the fence's own
/// words are what the dialect meets next. Every row is judged before the
/// one comparison, so each row binds on its own.
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
            let cause = match relation {
                "below" => below(egress, minimum),
                _ => MCP.to_string(),
            };
            (minimum, egress, relation, lost(&cause))
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
            lost(&below("uncontracted", "contracted"))
        ]
    );
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
