//! A boxed office hired from dsh (#896).

use super::*;

/// #896: past Linux's lifted fence, a boxed dsh hands office compiles
/// because the boxed cold and replacement shapes' ambient axes are measured
/// on an engine-only home. Before this unit the same compile refused
/// `provider 'dsh' has no measured strict MCP configuration for 'cold'`.
/// The boxed shapes are measured on Linux alone, and macOS's fence stands,
/// so the test runs nowhere else and asserts no macOS result.
#[cfg(target_os = "linux")]
#[test]
fn a_boxed_dsh_hands_office_compiles_past_the_lifted_fence() {
    let fixture = Fixture::new();
    fixture.write_agent_file("smith", &["glm-flash"], None, true);
    let (agents, adapters) = (fixture.agents(), shipped_adapters());
    let bundle = fixture.stage(boxed_seat("smith", "work"), None);
    // The fixture's default review seat hires a synthetic `judge` no
    // shipped adapter answers for; give it the boxed exec gate decision
    // 0043 ruling 3 admits, so the dsh work seat is the only model serving.
    let mut config: Value =
        serde_json::from_slice(&std::fs::read(bundle.join("bundle.json")).unwrap()).unwrap();
    config["seats"]["review"] = json!({
        "role": "roles/role.md",
        "results": ["clean"],
        "class": "gate",
        "driver": {"command": [
            "{brokkr}", "driver", "exec", "--", "bash", "./scripts/verify.sh"
        ]},
        "hands": {"kind": "workspace", "network": false, "binds": []}
    });
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    fixture.script("scripts/verify.sh");
    let lift = crate::capabilities::McpFence::lift();
    let compiled = Bundle::compile_under(&bundle, &agents, &adapters, Boundary::Namespace);
    drop(lift);
    compiled.expect("Linux's lifted fence admits the measured boxed dsh hands shapes");
}

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

/// #896 round 2, D2: the access comes from the office, not the class. A
/// read-only office hired at work class still carries `read-only`, so its
/// dsh sandbox is read-only rather than workspace-write.
#[test]
fn a_read_only_office_hired_at_work_class_carries_read_only_access() {
    use brokkr_protocol::native_controls::Access;
    let fixture = Fixture::new();
    fixture.write_agent_file("review-adversarial", &["glm-flash"], None, true);
    let path = fixture.agents().join("review-adversarial.json");
    let mut body: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    body["access"] = json!("read-only");
    std::fs::write(&path, serde_json::to_string(&body).unwrap()).unwrap();
    let (agents, adapters) = (fixture.agents(), shipped_adapters());
    let seat = boxed_seat("review-adversarial", "work");
    let bundle = fixture
        .compile_roots(seat, &agents, &adapters, Boundary::Namespace)
        .expect("the read-only office compiles at work class");
    assert_eq!(
        bundle.sites.get("work").unwrap().access,
        Some(Access::ReadOnly)
    );
}
