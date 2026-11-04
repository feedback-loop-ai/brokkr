//! The served route of a bare and a qualified id under each driver.

use super::*;

#[test]
fn a_bare_dsh_id_is_served_on_the_default_route_and_no_other_driver_has_one() {
    let served = |kind: AdapterKind| {
        ["deepseek-flash", "spark-glm/GLM-5.3", "meta/meta/muse"].map(|id| kind.served_route(id))
    };
    assert_eq!(
        served(AdapterKind::Dsh),
        [Some("deepseek-official"), Some("spark-glm"), Some("meta")]
    );
    for kind in [
        AdapterKind::Claude,
        AdapterKind::Lanetally,
        AdapterKind::Codex,
        AdapterKind::Exec,
    ] {
        assert_eq!(served(kind), [None, Some("spark-glm"), Some("meta")]);
    }
    assert_eq!(split_route("meta/meta/muse"), (Some("meta"), "meta/muse"));
}

/// Fix round 430s4a6 L2 (decision 0036 ruling 2; decision 0103 ruling 1(a)):
/// the route form is the model id's prefix before its first `/`, colon
/// included, and the producer refuses any other route before emission
/// instead of rewriting it.
#[test]
fn the_route_form_admits_the_derived_prefix_and_the_producer_refuses_another() {
    assert_eq!(
        split_route("featherless:qwen/qwen3"),
        (Some("featherless:qwen"), "qwen3")
    );
    assert!(route_form("featherless:qwen"));
    let refusal = || {
        crate::PreSessionRefusal::from_parts(
            crate::RefusalClass::RateLimit,
            "rate_limit".into(),
            None,
            None,
        )
    };
    for outside in ["", "bad/route", "-bad", &"r".repeat(81)] {
        assert!(!route_form(outside), "{outside:?}");
        assert_eq!(
            refusal().attributed("claude", Some(outside)),
            Err(crate::RefusalError::RouteForm),
            "{outside:?} is refused before emission, never rewritten"
        );
    }
    assert_eq!(
        refusal()
            .attributed("claude", Some("featherless:qwen"))
            .unwrap()
            .route(),
        Some("featherless:qwen")
    );
}
