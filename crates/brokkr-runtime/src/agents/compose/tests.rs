//! A declared route overlay's composition: exactly one `--patch` on the
//! route that declares it, none on a near-named route, the working-
//! directory-relative value, and the two-overlay refusal.

use brokkr_core::canonical::sha256_bytes;
use brokkr_core::realms::Boundary;
use serde_json::{json, Value};

use crate::agents::tests::Tree;
use crate::agents::ResolveError;

fn body(driver: Value, routes: Value) -> Value {
    json!({
        "provider": "dsh",
        "binary": "dsh",
        "driver": driver,
        "models": {"qwen": "dashscope/qwen3.8-max"},
        "model_flag": "--model",
        "efforts": ["low", "medium", "high"],
        "effort_flag": "--effort",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
        "routes": routes,
    })
}

fn tree(adapter: Value, overlay: &str) -> Tree {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.root.join("adapters/overlays")).unwrap();
    tree.raw("adapters/overlays/spark.yml", overlay);
    let digest = sha256_bytes(overlay.as_bytes());
    let mut adapter = adapter;
    adapter["routes"] = json!({
        "dashscope": {
            "egress": "local",
            "overlay": {"file": "overlays/spark.yml", "sha256": digest},
        },
    });
    tree.write("adapters/dsh.json", &adapter);
    tree.write(
        "agents/tester.json",
        &json!({
            "description": "a test agent",
            "charter": "charters/c.md",
            "models": ["qwen"],
            "efforts": {"qwen": "high"},
            "limits": {"max_attempts": 2, "timeout_seconds": 60},
        }),
    );
    tree
}

fn composed(tree: &Tree, concrete: &str) -> Vec<String> {
    let adapters = tree.adapters();
    let adapter = adapters.adapter("dsh").unwrap();
    let declared = adapters.route_overlay("dsh", concrete);
    let library = tree.library();
    let agent = library.agent("tester").unwrap();
    super::compose(
        agent,
        adapter,
        "qwen",
        concrete,
        Boundary::Namespace,
        declared,
        &tree.root,
    )
    .expect("the composition succeeds")
    .argv()
}

fn patches(argv: &[String]) -> Vec<&str> {
    argv.iter()
        .enumerate()
        .filter(|(_, token)| token.as_str() == "--patch")
        .filter_map(|(at, _)| argv.get(at + 1).map(String::as_str))
        .collect()
}

#[test]
fn a_seat_on_the_declaring_route_gets_exactly_one_patch() {
    let tree = tree(
        body(json!(["{brokkr}", "driver", "dsh", "--"]), json!({})),
        "a: b\n",
    );
    let argv = composed(&tree, "dashscope/qwen3.8-max");
    assert_eq!(
        patches(&argv),
        vec!["adapters/overlays/spark.yml"],
        "{argv:?}"
    );
}

#[test]
fn a_seat_on_another_route_gets_none_even_when_the_name_is_a_prefix() {
    let tree = tree(
        body(json!(["{brokkr}", "driver", "dsh", "--"]), json!({})),
        "a: b\n",
    );
    let argv = composed(&tree, "dash/qwen3.8-max");
    assert!(patches(&argv).is_empty(), "{argv:?}");
}

#[test]
fn a_seat_with_both_a_declared_overlay_and_an_authored_patch_refuses() {
    let tree = tree(
        body(
            json!(["{brokkr}", "driver", "dsh", "--", "--patch", "authored.yml"]),
            json!({}),
        ),
        "a: b\n",
    );
    let adapters = tree.adapters();
    let adapter = adapters.adapter("dsh").unwrap();
    let declared = adapters.route_overlay("dsh", "dashscope/qwen3.8-max");
    let library = tree.library();
    let agent = library.agent("tester").unwrap();
    let error = super::compose(
        agent,
        adapter,
        "qwen",
        "dashscope/qwen3.8-max",
        Boundary::Namespace,
        declared,
        &tree.root,
    )
    .expect_err("two overlays refuse");
    assert_eq!(
        error,
        ResolveError::RouteOverlayConflict {
            agent: "tester".into(),
            declared: "overlays/spark.yml".into(),
            authored: "--patch authored.yml".into(),
        }
    );
    let message = error.to_string();
    assert!(message.contains("overlays/spark.yml"), "{message}");
    assert!(message.contains("--patch"), "{message}");
}

#[test]
fn the_resolved_candidate_carries_the_declared_overlay_and_its_relative_patch() {
    let tree = tree(
        body(json!(["{brokkr}", "driver", "dsh", "--"]), json!({})),
        "a: b\n",
    );
    let report = crate::agents::report_narrowed_in(
        &tree.library(),
        &tree.adapters(),
        &crate::agents::Availability::unspecified(),
        "tester",
        Boundary::Namespace,
        &crate::agents::LocalTools::unspecified(),
        &tree.root,
    )
    .expect("the report composes");
    let resolution = crate::agents::resolve_report(report, &tree.adapters()).unwrap();
    let candidate = &resolution.candidates[0];
    assert_eq!(
        patches(&candidate.argv),
        vec!["adapters/overlays/spark.yml"],
        "{:?}",
        candidate.argv
    );
    assert_eq!(
        candidate
            .route_overlay
            .as_ref()
            .map(|overlay| overlay.file.as_str()),
        Some("overlays/spark.yml")
    );
}
