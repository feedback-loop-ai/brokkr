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
