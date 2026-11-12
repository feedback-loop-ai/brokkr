//! A declared route overlay's composition: exactly one `--patch` on the
//! route that declares it, none on a near-named route, the working-
//! directory-relative value, and the two-overlay refusal.

use brokkr_core::canonical::sha256_bytes;
use std::path::Path;

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
    tree_declaring(adapter, overlay, "dashscope")
}

fn tree_declaring(adapter: Value, overlay: &str, route: &str) -> Tree {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.root.join("adapters/overlays")).unwrap();
    tree.raw("adapters/overlays/spark.yml", overlay);
    let digest = sha256_bytes(overlay.as_bytes());
    let mut adapter = adapter;
    let mut routes = serde_json::Map::new();
    routes.insert(
        route.to_string(),
        json!({
            "egress": "local",
            "overlay": {"file": "overlays/spark.yml", "sha256": digest},
        }),
    );
    adapter["routes"] = Value::Object(routes);
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

fn composed_in(tree: &Tree, concrete: &str, workdir: &std::path::Path) -> Vec<String> {
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
        workdir,
    )
    .expect("the composition succeeds")
    .argv()
}

fn composed(tree: &Tree, concrete: &str) -> Vec<String> {
    composed_in(tree, concrete, &tree.root)
}

/// The composition of a seat carrying both overlays, refused.
fn conflict_error(tree: &Tree) -> crate::agents::ResolveError {
    let adapters = tree.adapters();
    let adapter = adapters.adapter("dsh").unwrap();
    let declared = adapters.route_overlay("dsh", "dashscope/qwen3.8-max");
    let library = tree.library();
    let agent = library.agent("tester").unwrap();
    super::compose(
        agent,
        adapter,
        "qwen",
        "dashscope/qwen3.8-max",
        Boundary::Namespace,
        declared,
        &tree.root,
    )
    .expect_err("two overlays refuse")
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
fn a_seat_on_a_near_named_route_gets_none_because_the_route_is_its_prefix() {
    // The declared route is `dash`; the pinned concrete id is on
    // `dashscope`. Decision 0036 ruling 2 splits at the first `/`, so the
    // lookup asks for `dashscope` and finds nothing — a prefix test would
    // find `dash` and compose a patch the route never declared.
    let tree = tree_declaring(
        body(json!(["{brokkr}", "driver", "dsh", "--"]), json!({})),
        "a: b\n",
        "dash",
    );
    let argv = composed(&tree, "dashscope/qwen3.8-max");
    assert!(patches(&argv).is_empty(), "{argv:?}");
    // The same adapter composes on the route it actually declares.
    let argv = composed(&tree, "dash/qwen3.8-max");
    assert_eq!(
        patches(&argv),
        vec!["adapters/overlays/spark.yml"],
        "{argv:?}"
    );
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
    let error = conflict_error(&tree);
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

#[test]
fn a_declared_overlay_outside_the_run_tree_keeps_its_adapter_relative_spelling() {
    let tree = tree(
        body(json!(["{brokkr}", "driver", "dsh", "--"]), json!({})),
        "a: b\n",
    );
    let argv = composed_in(&tree, "dashscope/qwen3.8-max", Path::new("/nonexistent"));
    assert_eq!(patches(&argv), vec!["overlays/spark.yml"], "{argv:?}");
}

#[test]
fn an_equals_joined_authored_patch_is_named_whole_in_the_refusal() {
    let tree = tree(
        body(
            json!(["{brokkr}", "driver", "dsh", "--", "--patch=authored.yml"]),
            json!({}),
        ),
        "a: b\n",
    );
    let error = conflict_error(&tree);
    let message = error.to_string();
    assert!(message.contains("--patch=authored.yml"), "{message}");
}

#[test]
fn an_effort_flag_that_spells_patch_is_the_second_overlay_and_refuses() {
    // The authored check runs again behind the effort pin: a provider
    // whose declared effort flag is `--patch` composes a second overlay
    // there, which the refusal names whole rather than leaving the seat
    // to bind nothing and fail at start.
    let mut adapter = body(json!(["{brokkr}", "driver", "dsh", "--"]), json!({}));
    adapter["effort_flag"] = json!("--patch");
    let tree = tree(adapter, "a: b\n");
    let error = conflict_error(&tree);
    assert_eq!(
        error,
        ResolveError::RouteOverlayConflict {
            agent: "tester".into(),
            declared: "overlays/spark.yml".into(),
            authored: "--patch high".into(),
        }
    );
}
