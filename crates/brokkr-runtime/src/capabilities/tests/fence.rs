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
