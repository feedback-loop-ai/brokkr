//! The routes declaration's overlay form: one valid object and every
//! refusal, each pinned to the adapter, the route and the field.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::sha256_bytes;
use serde_json::{json, Value};

use super::{Declared, LibraryError, OverlayFault, OverlayRefusal, RouteFault, RouteRefusal};
use crate::agents::tests::{claude_body, Tree};
use crate::agents::{DeclaredRouteOverlay, EgressClass};

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

const WHAT: &str = "adapter 'dsh'";
const ROUTE: &str = "spark-glm";

/// The adapter's own directory, canonical, as `declarations` resolves it.
fn adapters_directory(tree: &Tree) -> PathBuf {
    std::fs::create_dir_all(tree.root.join("adapters")).unwrap();
    tree.root.join("adapters").canonicalize().unwrap()
}

/// `overlay_declaration_with` for `adapter 'dsh'` and route `spark-glm`.
fn injected(
    raw: &Value,
    directory: &Path,
    metadata: impl FnOnce(&Path) -> std::io::Result<std::fs::Metadata>,
    read: impl FnOnce(&Path) -> std::io::Result<Vec<u8>>,
) -> Result<DeclaredRouteOverlay, OverlayFault<'static>> {
    super::overlay_declaration_with(raw, WHAT, ROUTE, directory, metadata, read)
}

/// The same over the real file system.
fn on_disk(raw: &Value, directory: &Path) -> Result<DeclaredRouteOverlay, OverlayFault<'static>> {
    injected(
        raw,
        directory,
        |path| std::fs::metadata(path),
        |path| std::fs::read(path),
    )
}

/// The typed refusal a declaration took, or a panic naming what it took.
fn refused(result: Result<DeclaredRouteOverlay, OverlayFault<'static>>) -> OverlayRefusal<'static> {
    match result {
        Err(OverlayFault::Refused(refusal)) => refusal,
        other => panic!("not an overlay refusal: {other:?}"),
    }
}

/// `overlay_path` for `adapter 'dsh'` and route `spark-glm`.
fn path_refusal(file: &str, directory: &Path) -> OverlayRefusal<'static> {
    super::overlay_path(file, WHAT, ROUTE, directory).unwrap_err()
}

/// A metadata fault and a read fault each take their own unreadable
/// variant, and an over-bound read takes the read bound, not the metadata
/// bound. The injected closures stand where the real `std::fs` calls do;
/// the directory is the adapter's own.
#[test]
fn a_metadata_or_read_fault_takes_the_injected_refusal() {
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    let directory = adapters_directory(&tree);
    let raw = json!({"file": "overlays/spark.yml", "sha256": "a".repeat(64)});
    let fault_metadata = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    let fault_read = |_: &Path| Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    let (what, route) = (WHAT, ROUTE);
    assert_eq!(
        refused(injected(&raw, &directory, fault_metadata, |path| {
            std::fs::read(path)
        })),
        OverlayRefusal::MetadataUnreadable { what, route }
    );
    assert_eq!(
        refused(injected(
            &raw,
            &directory,
            |path| std::fs::metadata(path),
            fault_read
        )),
        OverlayRefusal::ReadUnreadable { what, route }
    );
    let oversized = |_: &Path| Ok(vec![b'x'; 64 * 1024 + 1]);
    assert_eq!(
        refused(injected(
            &raw,
            &directory,
            |path| std::fs::metadata(path),
            oversized
        )),
        OverlayRefusal::ReadOverBound { what, route }
    );
}

/// The four path checks, each reached through `overlay_path` on a real
/// tree, in the order they run.
#[test]
fn each_path_check_refuses_in_its_own_variant() {
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    std::fs::write(tree.root.join("outside.yml"), "a: b\n").unwrap();
    std::os::unix::fs::symlink(
        "../../outside.yml",
        tree.root.join("adapters/overlays/escape.yml"),
    )
    .unwrap();
    let directory = adapters_directory(&tree);
    let (what, route) = (WHAT, ROUTE);
    assert_eq!(
        path_refusal("/etc/passwd", &directory),
        OverlayRefusal::Absolute { what, route }
    );
    assert_eq!(
        path_refusal("../outside.yml", &directory),
        OverlayRefusal::Traverses { what, route }
    );
    assert_eq!(
        path_refusal("overlays/missing.yml", &directory),
        OverlayRefusal::Unresolved { what, route }
    );
    assert_eq!(
        path_refusal("overlays/escape.yml", &directory),
        OverlayRefusal::Outside { what, route }
    );
}

/// The file and digest checks on real files: a directory, a file over
/// the bound, a malformed digest and a digest that does not match; and a
/// missing `sha256` carries the shared reader's own refusal unchanged.
#[test]
fn each_file_and_digest_check_refuses_in_its_own_variant() {
    let tree = Tree::new();
    write_overlay(&tree, "a: b\n");
    std::fs::create_dir_all(tree.root.join("adapters/overlays/dir.yml")).unwrap();
    let big = vec![b'x'; 64 * 1024 + 1];
    std::fs::write(tree.root.join("adapters/overlays/big.yml"), &big).unwrap();
    let directory = adapters_directory(&tree);
    let (what, route) = (WHAT, ROUTE);
    let wrong = "a".repeat(64);
    assert_eq!(
        refused(on_disk(&declared("overlays/dir.yml", &wrong), &directory)),
        OverlayRefusal::NotRegular { what, route }
    );
    assert_eq!(
        refused(on_disk(
            &declared("overlays/big.yml", &sha256_bytes(&big)),
            &directory
        )),
        OverlayRefusal::MetadataOverBound { what, route }
    );
    assert_eq!(
        refused(on_disk(&declared("overlays/spark.yml", "abc"), &directory)),
        OverlayRefusal::Malformed { what, route }
    );
    assert_eq!(
        refused(on_disk(&declared("overlays/spark.yml", &wrong), &directory)),
        OverlayRefusal::Mismatch { what, route }
    );
    let missing = json!({"file": "overlays/spark.yml"});
    match on_disk(&missing, &directory) {
        Err(OverlayFault::Library(LibraryError::Invalid(text))) => assert_eq!(
            text,
            "adapter 'dsh' 'routes.spark-glm.overlay' needs a non-empty string 'sha256'"
        ),
        other => panic!("not the reader's own refusal: {other:?}"),
    }
}

/// Every variant's text, written out, for `adapter 'dsh'` and route
/// `spark-glm`: the bytes the loader has always printed.
#[test]
fn every_route_overlay_refusal_reads_as_it_always_has() {
    let (what, route) = (WHAT, ROUTE);
    let cases = [
        (
            OverlayRefusal::Absolute { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' must be relative to the adapter \
             file's own directory",
        ),
        (
            OverlayRefusal::Traverses { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' must not traverse out of the \
             adapter file's own directory",
        ),
        (
            OverlayRefusal::Unresolved { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' does not resolve to a regular file \
             inside the adapter file's own directory",
        ),
        (
            OverlayRefusal::Outside { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' resolves outside the adapter \
             file's own directory",
        ),
        (
            OverlayRefusal::MetadataUnreadable { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' does not resolve to a readable \
             regular file inside the adapter file's own directory",
        ),
        (
            OverlayRefusal::NotRegular { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' does not resolve to a regular file \
             inside the adapter file's own directory",
        ),
        (
            OverlayRefusal::MetadataOverBound { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' is past the route overlay's byte \
             bound",
        ),
        (
            OverlayRefusal::Malformed { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.sha256' must be a 64-character lowercase \
             hexadecimal digest",
        ),
        (
            OverlayRefusal::ReadUnreadable { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' does not resolve to a readable \
             regular file inside the adapter file's own directory",
        ),
        (
            OverlayRefusal::ReadOverBound { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.file' is past the route overlay's byte \
             bound",
        ),
        (
            OverlayRefusal::Mismatch { what, route },
            "adapter 'dsh' 'routes.spark-glm.overlay.sha256' does not equal the digest of the \
             file this route declares",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text);
    }
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

/// An adapter file whose directory never resolves.
const ABSENT: &str = "/nonexistent/forge-1052b/adapters/dsh.json";

/// The drivers that dispatch `dsh` and that dispatch no built-in driver.
fn dsh_driver() -> Vec<String> {
    ["{brokkr}", "driver", "dsh", "--"]
        .map(String::from)
        .to_vec()
}

fn custom_driver() -> Vec<String> {
    ["sh", "-c", "true", "--"].map(String::from).to_vec()
}

/// An adapter file in a tree whose directory resolves.
fn resolvable(tree: &Tree) -> PathBuf {
    adapters_directory(tree).join("dsh.json")
}

/// `declared_routes` for `adapter 'dsh'` over an adapter body.
fn routes_of<'a>(
    body: &'a Value,
    path: &Path,
    driver: &'a [String],
) -> Result<Declared, RouteFault<'a>> {
    super::declared_routes(body.as_object().unwrap(), WHAT, path, driver)
}

/// The typed refusal a declaration took, or a panic naming what it took.
fn route_refused(result: Result<Declared, RouteFault<'_>>) -> RouteRefusal<'_> {
    match result {
        Err(RouteFault::Refused(refusal)) => refusal,
        other => panic!("not a route refusal: {other:?}"),
    }
}

/// `declared_routes` over a body whose `routes` is `routes` refuses first
/// with `expected`.
fn refuses_first(routes: Value, path: &Path, driver: &[String], expected: RouteRefusal<'_>) {
    let body = json!({ "routes": routes });
    assert_eq!(route_refused(routes_of(&body, path, driver)), expected);
}

/// Each check refuses in its own variant, reached through the function
/// that builds it.
#[test]
fn each_route_check_refuses_in_its_own_variant() {
    let tree = Tree::new();
    let path = resolvable(&tree);
    let (dsh, custom) = (dsh_driver(), custom_driver());
    let (what, route) = (WHAT, ROUTE);
    let overlay = declared("overlays/spark.yml", &"a".repeat(64));
    refuses_first(json!([]), &path, &dsh, RouteRefusal::NotObject { what });
    assert_eq!(
        super::directory(Path::new(ABSENT), WHAT),
        Err(RouteRefusal::NoDirectory { what })
    );
    let bad = json!({"us/east": "local"});
    let named = RouteRefusal::BadName {
        what,
        route: "us/east",
    };
    refuses_first(bad, &path, &dsh, named);
    let lacking = json!({ROUTE: {"overlay": overlay}});
    refuses_first(
        lacking,
        &path,
        &dsh,
        RouteRefusal::NeedsEgress { what, route },
    );
    assert_eq!(
        super::route_egress(&json!("lokal"), ROUTE, WHAT),
        Err(RouteRefusal::BadEgress { what, route })
    );
    let lacking = json!({ROUTE: {"egress": "local"}});
    refuses_first(
        lacking,
        &path,
        &dsh,
        RouteRefusal::NeedsOverlay { what, route },
    );
    let kind = "<custom>";
    let whole = json!({ROUTE: {"egress": "local", "overlay": overlay}});
    refuses_first(
        whole,
        &path,
        &custom,
        RouteRefusal::WrongDriver { what, route, kind },
    );
    let neither = json!({ROUTE: 7});
    refuses_first(
        neither,
        &path,
        &dsh,
        RouteRefusal::NeitherForm { what, route },
    );
}

/// Where two checks could both fire, the first in the order wins: the
/// directory before the name, the shape before the directory, the egress
/// before the overlay and the egress class before the driver.
#[test]
fn the_route_checks_run_in_their_order() {
    let tree = Tree::new();
    let absent = Path::new(ABSENT);
    let (dsh, custom) = (dsh_driver(), custom_driver());
    let (what, route) = (WHAT, ROUTE);
    let path = resolvable(&tree);
    let empty = json!({ROUTE: {}});
    refuses_first(
        empty,
        &path,
        &dsh,
        RouteRefusal::NeedsEgress { what, route },
    );
    let overlay = declared("overlays/spark.yml", &"a".repeat(64));
    let both = json!({ROUTE: {"egress": "lokal", "overlay": overlay}});
    refuses_first(
        both,
        &path,
        &custom,
        RouteRefusal::BadEgress { what, route },
    );
    let bad = json!({"us/east": "local"});
    refuses_first(bad, absent, &dsh, RouteRefusal::NoDirectory { what });
    refuses_first(json!([]), absent, &dsh, RouteRefusal::NotObject { what });
}

/// What the declaration admits: an absent `routes` before any directory
/// is read, an empty object, and a string entry's class with no overlay.
#[test]
fn an_absent_empty_or_string_routes_declaration_admits_exactly_its_classes() {
    let tree = Tree::new();
    let path = resolvable(&tree);
    let dsh = dsh_driver();
    let empty = |declared: Declared| {
        assert_eq!(declared.routes, BTreeMap::new());
        assert_eq!(declared.overlays, BTreeMap::new());
    };
    let body = json!({});
    empty(routes_of(&body, Path::new(ABSENT), &dsh).unwrap());
    let body = json!({"routes": {}});
    empty(routes_of(&body, &path, &dsh).unwrap());
    let body = json!({"routes": {ROUTE: "contracted"}});
    let declared = routes_of(&body, &path, &dsh).unwrap();
    assert_eq!(
        declared.routes,
        BTreeMap::from([(ROUTE.to_string(), EgressClass::Contracted)])
    );
    assert_eq!(declared.overlays, BTreeMap::new());
}

/// `declarations` hands the typed refusal on as the loader's own text.
#[test]
fn a_route_refusal_reaches_the_loader_as_its_text() {
    let tree = Tree::new();
    let body = json!({"routes": {"us/east": "local"}});
    match super::declarations(body.as_object().unwrap(), WHAT, &resolvable(&tree), &[]) {
        Err(LibraryError::Invalid(text)) => assert_eq!(
            text,
            "adapter 'dsh' 'routes' names 'us/east', which does not match ^[A-Za-z0-9._:-]+$"
        ),
        other => panic!("not the route refusal's text: {other:?}"),
    }
}

/// Every variant's text, written out, for `adapter 'dsh'`, route
/// `spark-glm` and the kind `<custom>`: the bytes the loader has always
/// printed.
#[test]
fn every_route_declaration_refusal_reads_as_it_always_has() {
    let (what, route) = (WHAT, ROUTE);
    let cases = [
        (
            RouteRefusal::NotObject { what },
            "adapter 'dsh' 'routes' must be an object of route name → egress class",
        ),
        (
            RouteRefusal::NoDirectory { what },
            "adapter 'dsh' 'routes' cannot resolve the adapter file's own directory",
        ),
        (
            RouteRefusal::BadName { what, route },
            "adapter 'dsh' 'routes' names 'spark-glm', which does not match ^[A-Za-z0-9._:-]+$",
        ),
        (
            RouteRefusal::NeedsEgress { what, route },
            "adapter 'dsh' 'routes.spark-glm' needs 'egress': the route's class is declared in \
             both forms",
        ),
        (
            RouteRefusal::BadEgress { what, route },
            "adapter 'dsh' 'routes.spark-glm.egress' is not one of \"local\", \"contracted\" or \
             \"uncontracted\"; a route's class is the same closed vocabulary in both forms \
             (decision 0036 ruling 1)",
        ),
        (
            RouteRefusal::NeedsOverlay { what, route },
            "adapter 'dsh' 'routes.spark-glm' needs 'overlay': the object form is how a route \
             declares one",
        ),
        (
            RouteRefusal::WrongDriver {
                what,
                route,
                kind: "<custom>",
            },
            "adapter 'dsh' 'routes.spark-glm.overlay' declares a route overlay on an adapter \
             that dispatches '<custom>'; only the 'dsh' driver reads a route overlay, so a \
             driver that does not is refused at load rather than handed a flag it ignores",
        ),
        (
            RouteRefusal::NeitherForm { what, route },
            "adapter 'dsh' 'routes.spark-glm' must be a string egress class or an object with \
             'egress' and 'overlay'",
        ),
    ];
    for (refusal, text) in cases {
        assert_eq!(refusal.to_string(), text);
    }
}
