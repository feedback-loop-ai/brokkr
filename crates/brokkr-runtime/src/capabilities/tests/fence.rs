//! The MCP compile fence an authority is loaded under (decision 0065 slice
//! two, U1g1; SC5 and SI2), and the builder the engine's suites record a
//! site's capability facts through.

use super::*;

impl SiteCapabilities {
    /// A site's facts as the engine's suites build them by hand: `outcomes`
    /// for `asks`, each beside an empty SI2 record, as a serving with no
    /// model shape has.
    pub(crate) fn unjudged(asks: SiteAsks, outcomes: Vec<Outcome>) -> SiteCapabilities {
        let strict = vec![Vec::new(); outcomes.len()];
        SiteCapabilities {
            asks,
            outcomes,
            strict,
        }
    }
}

/// Every authority is loaded under the standing fence, a readout's as much
/// as a compile's. Only a test lifts it, on its own thread, while its lift
/// is held: the next load stores the lifted fence, and once the lift is
/// dropped, a panic unwinding through it included, the fence stands again.
#[test]
fn an_authority_stores_the_fence_it_was_loaded_under_and_only_a_held_lift_lifts_it() {
    let root = TempDir::new().unwrap();
    let fences = || {
        let loaded = Authority::load(CapabilityContext::no_grants("private", root.path()));
        let nothing = Authority::nothing("private", root.path());
        (loaded.unwrap().fence, nothing.fence)
    };
    assert_eq!(fences(), (McpFence::Standing, McpFence::Standing));
    let lift = McpFence::lift();
    assert_eq!(fences(), (McpFence::Lifted, McpFence::Lifted));
    drop(lift);
    assert_eq!(fences(), (McpFence::Standing, McpFence::Standing));
    let unwound = std::panic::catch_unwind(|| {
        let _lift = McpFence::lift();
        panic!("unwinding through a held lift");
    });
    let message = *unwound.unwrap_err().downcast::<&str>().unwrap();
    assert_eq!(message, "unwinding through a held lift");
    assert_eq!(fences(), (McpFence::Standing, McpFence::Standing));
}

/// The world past the fence's proofs share, over [`cq1_root`]'s:
/// `web-search` granted through the `mcp` dialect `search-mcp`, the same
/// name `test-native`'s native power serves, whose schema admits
/// `allow.hosts` as `search-native`'s does, under `restrictions`.
fn search_mcp(root: &Path, restrictions: Value) -> CapabilityContext {
    let mut search = mcp_dialect("search-mcp", json!({"argv": ["/nonexistent/search-mcp"]}));
    search["serves"] = json!("web-search");
    search["egress"] = json!("contracted");
    let native = std::fs::read(root.join(ToolDialect::source_of("search-native"))).unwrap();
    let native: Value = serde_json::from_slice(&native).unwrap();
    search["restrictions"] = native["restrictions"].clone();
    dialect(root, &search);
    let mut grant = json!({"dialect": "search-mcp"});
    grant
        .as_object_mut()
        .unwrap()
        .extend(restrictions.as_object().unwrap().clone());
    context(root, json!({ "web-search": grant }))
}

/// U9b1 (D3 steps 2 and 7; operator ruling 1 of 2026-10-18): past the
/// lifted fence an `mcp` grant loads and its holding is the broker's, so
/// no native key turns ON for it: `test-native`'s same-name power stays
/// OFF, whatever the seat asks, with no notice. Standing, as on macOS, the
/// same grant is the realm-wide refusal it always was.
#[test]
fn past_the_lifted_fence_an_mcp_holding_is_the_brokers_and_its_native_namesake_stays_off() {
    let root = cq1_root();
    let standing = Authority::load(search_mcp(root.path(), json!({})));
    assert_eq!(
        standing.unwrap_err(),
        "realm 'private' grants capability 'web-search' through dialect 'search-mcp' of kind \
         'mcp', whose broker support is not implemented until decision 0065 slice two"
    );
    let _lift = McpFence::lift();
    let authority = Authority::load(search_mcp(root.path(), json!({}))).unwrap();
    let native = switchable();
    let observed = ["requires", "wants"].map(|strength| {
        let site = asks(json!({ "web-search": strength }));
        let outcome = authority.resolve(&site, &serving(&native)).unwrap();
        let server = match &outcome.held["web-search"].implementation {
            Implementation::Mcp { server, .. } => server.clone(),
            Implementation::Native { .. } => panic!("a native carrier for an mcp holding"),
        };
        (server, argv_of(&outcome), outcome.notices)
    });
    let off = || {
        let argv = vec!["--search-off".to_string()];
        ("cap-web-search".to_string(), argv, Vec::new())
    };
    assert_eq!(observed, [off(), off()]);
}

/// U9b1 (D3 step 6; D11): past the lifted fence the broker carries only
/// the empty restriction, so a nonempty one is never discarded alone: a
/// requires refuses and a want drops with the deferred cause, both forms
/// ending at it, and the native namesake stays OFF.
#[test]
fn past_the_lifted_fence_a_nonempty_mcp_restriction_refuses_or_drops_with_the_deferred_cause() {
    let root = cq1_root();
    let _lift = McpFence::lift();
    let hosts = json!({"allow": {"hosts": ["docs.rs"]}});
    let authority = Authority::load(search_mcp(root.path(), hosts)).unwrap();
    let native = switchable();
    let resolve = |strength| {
        let site = asks(json!({ "web-search": strength }));
        authority.resolve(&site, &serving(&native))
    };
    let cause = "MCP nonempty restrictions are deferred to the restriction-transport slice";
    let who = "seat 'research' (office 'researcher') in realm 'private'";
    assert_eq!(
        resolve("requires").unwrap_err(),
        format!("{who}: requires capability 'web-search' but {cause}")
    );
    let wanted = resolve("wants").unwrap();
    let notice = format!("{who}: dropped wanted capability 'web-search' because {cause}");
    let off = vec!["--search-off".to_string()];
    assert_eq!(
        (wanted.held.len(), argv_of(&wanted), wanted.notices),
        (0, off, vec![("web-search".to_string(), notice)])
    );
}
