//! The routes declaration's overlay form: one valid object and every
//! refusal, each pinned to the adapter, the route and the field.

use brokkr_core::canonical::sha256_bytes;
use serde_json::{json, Value};

use crate::agents::tests::{claude_body, Tree};
use crate::agents::EgressClass;

fn dsh_body() -> Value {
    json!({
        "provider": "dsh",
        "binary": "dsh",
        "driver": ["{brokkr}", "driver", "dsh", "--"],
        "models": {"qwen": "dashscope/qwen3.8-max"},
        "model_flag": "--model",
        "efforts": ["low", "medium", "high"],
        "effort_flag": "--effort",
        "tool_permissions": "unsupported",
        "mcp": "unsupported",
        "routes": {"dashscope": "local"},
    })
}

fn declared(file: &str, sha256: &str) -> Value {
    json!({"file": file, "sha256": sha256})
}

fn write_overlay(tree: &Tree, text: &str) {
    std::fs::create_dir_all(tree.root.join("adapters/overlays")).unwrap();
    tree.raw("adapters/overlays/spark.yml", text);
}

fn with_route(tree: &Tree, mut body: Value, route: Value) {
    body["routes"] = json!({"spark-glm": route});
    tree.write("adapters/dsh.json", &body);
}

fn refusal(tree: &Tree, field: &str, value: &str) -> String {
    let error = tree.adapters_error();
    assert!(error.contains("adapter 'dsh'"), "{error}");
    assert!(error.contains("routes.spark-glm"), "{error}");
    assert!(error.contains(field), "{error}");
    assert!(!error.contains(value), "the refusal names a value: {error}");
    error
}

#[test]
fn the_object_form_loads_with_a_valid_overlay_and_the_string_form_is_unchanged() {
    let tree = Tree::new();
    let text = "provider:\n  display_name: spark-glm\n";
    write_overlay(&tree, text);
    let digest = sha256_bytes(text.as_bytes());
    let mut body = dsh_body();
    body["routes"] = json!({
        "spark": "local",
        "spark-glm": {"egress": "local", "overlay": declared("overlays/spark.yml", &digest)},
    });
    tree.write("adapters/dsh.json", &body);

    let adapters = tree.adapters();
    let dsh = adapters.adapter("dsh").expect("the adapter loads");
    assert_eq!(dsh.routes["spark"], EgressClass::Local);
    assert_eq!(dsh.routes["spark-glm"], EgressClass::Local);
    assert!(adapters.route_overlay("dsh", "spark/anything").is_none());
    let overlay = adapters
        .route_overlay("dsh", "spark-glm/anything")
        .expect("the route declares its overlay");
    assert_eq!(overlay.sha256, digest);
    assert_eq!(overlay.file, "overlays/spark.yml");
    assert_eq!(
        overlay.patch_value(&tree.root),
        "adapters/overlays/spark.yml",
        "the composed value is relative to the run's working directory"
    );
}

#[test]
fn an_unknown_route_or_overlay_key_refuses_naming_the_field() {
    let tree = Tree::new();
    let text = "a: b\n";
    write_overlay(&tree, text);
    let digest = sha256_bytes(text.as_bytes());
    let mut overlay = declared("overlays/spark.yml", &digest);
    overlay["extra"] = json!(true);
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": overlay}),
    );
    let error = tree.adapters_error();
    assert!(error.contains("adapter 'dsh'"), "{error}");
    assert!(error.contains("routes.spark-glm"), "{error}");
    assert!(error.contains("unknown key 'extra'"), "{error}");
}

#[test]
fn a_missing_overlay_file_refuses_naming_the_field() {
    let tree = Tree::new();
    let digest = "a".repeat(64);
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("overlays/missing.yml", &digest)}),
    );
    refusal(
        &tree,
        "routes.spark-glm.overlay.file",
        "overlays/missing.yml",
    );
}

#[test]
fn a_digest_mismatch_refuses_without_echoing_either_digest() {
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    let declared_digest = "a".repeat(64);
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("overlays/spark.yml", &declared_digest)}),
    );
    refusal(&tree, "routes.spark-glm.overlay.sha256", &declared_digest);
}

#[test]
fn an_absolute_or_traversing_overlay_path_refuses() {
    let tree = Tree::new();
    let digest = "a".repeat(64);
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("/etc/passwd", &digest)}),
    );
    let error = refusal(&tree, "routes.spark-glm.overlay.file", "/etc/passwd");
    assert!(error.contains("must be relative"), "{error}");

    let tree = Tree::new();
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("../outside.yml", &digest)}),
    );
    let error = refusal(&tree, "routes.spark-glm.overlay.file", "../outside.yml");
    assert!(error.contains("must not traverse"), "{error}");
}

#[test]
fn a_symlink_that_resolves_out_of_the_adapters_directory_refuses() {
    let tree = Tree::new();
    std::fs::write(tree.root.join("outside.yml"), "a: b\n").unwrap();
    std::fs::create_dir_all(tree.root.join("adapters/overlays")).unwrap();
    std::os::unix::fs::symlink(
        "../../outside.yml",
        tree.root.join("adapters/overlays/escape.yml"),
    )
    .unwrap();
    let digest = sha256_bytes(b"a: b\n");
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("overlays/escape.yml", &digest)}),
    );
    refusal(&tree, "routes.spark-glm.overlay.file", "outside.yml");
}

#[test]
fn an_over_bound_overlay_refuses_before_any_run() {
    let tree = Tree::new();
    let bytes = vec![b'x'; 64 * 1024 + 1];
    std::fs::create_dir_all(tree.root.join("adapters/overlays")).unwrap();
    std::fs::write(tree.root.join("adapters/overlays/big.yml"), &bytes).unwrap();
    let digest = sha256_bytes(&bytes);
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("overlays/big.yml", &digest)}),
    );
    refusal(&tree, "routes.spark-glm.overlay.file", "overlays/big.yml");
}

#[test]
fn a_provider_that_does_not_admit_a_route_overlay_refuses() {
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    let digest = sha256_bytes(b"a: b\n");
    let mut body = claude_body();
    body["routes"] = json!({
        "spark-glm": {"egress": "local", "overlay": declared("overlays/spark.yml", &digest)},
    });
    tree.write("adapters/claude.json", &body);
    let error = tree.adapters_error();
    assert!(error.contains("adapter 'claude'"), "{error}");
    assert!(error.contains("routes.spark-glm"), "{error}");
    assert!(error.contains("only the 'dsh' driver"), "{error}");
    assert!(
        !error.contains(&digest),
        "the refusal names a value: {error}"
    );
}
