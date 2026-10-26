//! A boxed office hired from dsh (#896).

use super::*;

/// The shipped dsh adapter declares hands, so a boxed office is hired from
/// it alone or behind a codex link, its link carrying no hands argument:
/// dsh's hands ride the seat's one overlay, which its driver writes. The
/// missing-workspace refusals still stand for a provider without them.
#[test]
fn a_boxed_office_is_hired_from_the_shipped_dsh_adapter() {
    for (models, efforts) in [
        (&["glm-flash"][..], None),
        (&["sol", "glm-flash"], Some(json!({"sol": "medium"}))),
    ] {
        let fixture = Fixture::new();
        fixture.write_agent_file("smith", models, efforts, true);
        let (agents, adapters) = (fixture.agents(), shipped_adapters());
        let seat = boxed_seat("smith", "work");
        let bundle = fixture.compile_roots(seat, &agents, &adapters, Boundary::Namespace);
        let bundle = bundle.expect("a chain whose links all declare hands composes");
        let dsh = smith_candidates(&bundle).last().unwrap();
        assert_eq!(
            (dsh.provider.as_str(), dsh.hands_fragment.len()),
            ("dsh", 0)
        );
        let pinned = [
            "driver",
            "dsh",
            "--",
            "--model",
            "spark-glm/GLM-5.3-Flash-EXL3",
        ];
        assert_eq!(dsh.argv[1..], pinned, "{models:?}");
    }
}
