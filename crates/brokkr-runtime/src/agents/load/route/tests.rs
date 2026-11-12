//! The routes declaration's overlay form: one valid object and every
//! refusal, each pinned to the adapter, the route and the field.

use std::path::Path;

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
    assert_eq!(
        overlay.patch_value(Path::new("")),
        "overlays/spark.yml",
        "an empty working directory falls back to the adapter-relative spelling"
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

#[test]
fn an_adapter_named_dsh_whose_driver_is_not_dsh_refuses_its_overlay() {
    // The gate is the DRIVER that reads the overlay, not the provider the
    // file is named for: a command that dispatches no built-in driver
    // would be handed a `--patch` it never reads.
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    let digest = sha256_bytes(b"a: b\n");
    let mut body = dsh_body();
    body["driver"] = json!(["sh", "-c", "true", "--"]);
    with_route(
        &tree,
        body,
        json!({"egress": "local", "overlay": declared("overlays/spark.yml", &digest)}),
    );
    let error = tree.adapters_error();
    assert!(error.contains("adapter 'dsh'"), "{error}");
    assert!(error.contains("routes.spark-glm"), "{error}");
    assert!(error.contains("dispatches '<custom>'"), "{error}");
    assert!(
        !error.contains(&digest),
        "the refusal names a value: {error}"
    );
}

#[test]
fn the_object_form_requires_its_two_fields_and_reads_the_same_egress_vocabulary() {
    let tree = Tree::new();
    let digest = "a".repeat(64);
    with_route(
        &tree,
        dsh_body(),
        json!({"overlay": declared("overlays/spark.yml", &digest)}),
    );
    let error = tree.adapters_error();
    assert!(error.contains("needs 'egress'"), "{error}");

    let tree = Tree::new();
    with_route(&tree, dsh_body(), json!({"egress": "local"}));
    let error = tree.adapters_error();
    assert!(error.contains("needs 'overlay'"), "{error}");

    let tree = Tree::new();
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "lokal", "overlay": declared("overlays/spark.yml", &digest)}),
    );
    let error = tree.adapters_error();
    assert!(error.contains("is not one of"), "{error}");

    let tree = Tree::new();
    with_route(&tree, dsh_body(), json!(7));
    let error = tree.adapters_error();
    assert!(error.contains("must be a string egress class"), "{error}");
}

#[test]
fn a_routes_declaration_whose_directory_cannot_be_resolved_refuses() {
    let map = json!({"routes": {"spark": "local"}});
    let absent = Path::new("/nonexistent/forge-1052b/adapters/dsh.json");
    let error = super::declarations(map.as_object().unwrap(), "adapter 'dsh'", absent, &[])
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("cannot resolve the adapter file's own directory"),
        "{error}"
    );
}

#[test]
fn a_malformed_or_uppercase_digest_refuses_naming_the_field() {
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    for malformed in ["abc".to_string(), sha256_bytes(b"a: b\n").to_uppercase()] {
        with_route(
            &tree,
            dsh_body(),
            json!({"egress": "local", "overlay": declared("overlays/spark.yml", &malformed)}),
        );
        let error = tree.adapters_error();
        assert!(error.contains("routes.spark-glm.overlay.sha256"), "{error}");
        assert!(
            error.contains("64-character lowercase hexadecimal"),
            "{error}"
        );
        assert!(
            !error.contains(&malformed),
            "the refusal names a value: {error}"
        );
    }
}

#[test]
fn an_overlay_that_is_not_an_object_or_lacks_a_field_refuses() {
    let tree = Tree::new();
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": "overlays/spark.yml"}),
    );
    let error = tree.adapters_error();
    assert!(error.contains("overlay' must be a JSON object"), "{error}");

    let tree = Tree::new();
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": {"sha256": "a".repeat(64)}}),
    );
    let error = tree.adapters_error();
    assert!(error.contains("needs a non-empty string 'file'"), "{error}");

    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": {"file": "overlays/spark.yml"}}),
    );
    let error = tree.adapters_error();
    assert!(
        error.contains("needs a non-empty string 'sha256'"),
        "{error}"
    );
}

#[test]
fn a_file_naming_a_directory_refuses() {
    let tree = Tree::new();
    std::fs::create_dir_all(tree.root.join("adapters/overlays/spark.yml")).unwrap();
    with_route(
        &tree,
        dsh_body(),
        json!({"egress": "local", "overlay": declared("overlays/spark.yml", &"a".repeat(64))}),
    );
    refusal(&tree, "routes.spark-glm.overlay.file", "overlays/spark.yml");
}

/// A metadata fault and a read fault each take the shared unreadable
/// refusal, and an over-bound read takes the bound refusal the metadata
/// check would have taken. The injected closures stand where the real
/// `std::fs` calls do; the directory is the adapter's own.
#[test]
fn a_metadata_or_read_fault_takes_the_injected_refusal() {
    fn injected(
        raw: &Value,
        directory: &Path,
        metadata: impl FnOnce(&Path) -> std::io::Result<std::fs::Metadata>,
        read: impl FnOnce(&Path) -> std::io::Result<Vec<u8>>,
    ) -> String {
        super::overlay_declaration_with(
            raw,
            "adapter 'dsh'",
            "spark-glm",
            directory,
            metadata,
            read,
        )
        .unwrap_err()
        .to_string()
    }
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    let directory = tree.root.join("adapters").canonicalize().unwrap();
    let raw = json!({"file": "overlays/spark.yml", "sha256": "a".repeat(64)});
    let fault_metadata = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    let fault_read = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    let error = injected(&raw, &directory, fault_metadata, |path| std::fs::read(path));
    assert!(
        error.contains("does not resolve to a readable regular file"),
        "{error}"
    );
    let error = injected(&raw, &directory, |path| std::fs::metadata(path), fault_read);
    assert!(
        error.contains("does not resolve to a readable regular file"),
        "{error}"
    );
    let oversized = |_: &Path| Ok(vec![b'x'; 64 * 1024 + 1]);
    let error = injected(&raw, &directory, |path| std::fs::metadata(path), oversized);
    assert!(
        error.contains("past the route overlay's byte bound"),
        "{error}"
    );
}

/// An adapter path with no parent resolves its overlays against the empty
/// path, which never canonicalizes, so the loader refuses by name; and the
/// route overlay's `--patch` pair needs a non-empty value.
#[test]
fn a_parentless_adapter_path_refuses_and_an_empty_patch_value_is_no_pair() {
    let error = super::directory(Path::new("/"), "adapter 'dsh'")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("'routes' cannot resolve the adapter file's own directory"),
        "{error}"
    );
    let pair =
        |value: &str| crate::agents::is_patch_pair(&["--patch".to_string(), value.to_string()]);
    assert!(pair("overlays/spark.yml"));
    assert!(!pair(""));
}
