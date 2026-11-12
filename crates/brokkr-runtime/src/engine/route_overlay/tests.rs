//! The engine's route-overlay binding: the argv shape it reads, the
//! compiled leaf-layer member, and the route's declared overlay with the
//! digest the adapter pinned at load.

use std::path::Path;

use brokkr_core::canonical::sha256_bytes;
use serde_json::json;

use crate::agents::DeclaredRouteOverlay;
use crate::engine::tests::{bundle, single_body};

fn declared(dir: &Path, file: &str, text: &str) -> DeclaredRouteOverlay {
    DeclaredRouteOverlay {
        file: file.to_string(),
        path: dir.join(file).canonicalize().unwrap(),
        sha256: sha256_bytes(text.as_bytes()),
    }
}

#[test]
fn a_single_patch_value_admits_exactly_one_well_formed_pair() {
    let one = |parts: &[&str]| {
        let owned: Vec<String> = parts.iter().map(|part| part.to_string()).collect();
        super::single_patch_value(&owned).map(str::to_string)
    };
    assert_eq!(one(&["--patch", "route.yml"]).as_deref(), Some("route.yml"));
    assert_eq!(
        one(&["-x", "value", "--patch", "route.yml", "tail"]).as_deref(),
        Some("route.yml")
    );
    // A second --patch, an empty value, a flag value, a joined spelling and
    // a bare --patch all yield no value.
    assert_eq!(one(&["--patch", "a", "--patch", "b"]), None);
    assert_eq!(one(&["--patch", ""]), None);
    assert_eq!(one(&["--patch", "--other"]), None);
    assert_eq!(one(&["--patch=a"]), None);
    assert_eq!(one(&["--patch"]), None);
}

#[test]
fn a_route_overlay_binding_refuses_a_resolved_path_that_is_not_a_regular_file() {
    // `resolved` canonicalizes inside the layer but is a directory: the
    // binding is absent, and the adapter's pre-work failure to start is
    // the one refusal (design D6 mechanism 1). The manifest names the
    // path, so only the regular-file check keeps this from binding.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("as-directory")).unwrap();
    let mut bundle = bundle(dir.path(), single_body(vec!["missing-driver".into()]));
    bundle.manifest["files"]["as-directory"] = json!("a".repeat(64));
    let command = vec!["--patch".to_string(), "as-directory".to_string()];
    assert!(super::route_overlay_binding(&bundle, &command, dir.path(), None).is_none());

    // The declaration's own path is a directory: resolution and
    // containment both hold, so only the regular-file guard withholds it.
    let declared = DeclaredRouteOverlay {
        file: "as-directory".into(),
        path: dir.path().join("as-directory").canonicalize().unwrap(),
        sha256: "a".repeat(64),
    };
    assert!(super::route_overlay_binding(&bundle, &command, dir.path(), Some(&declared)).is_none());
}

#[test]
fn a_declared_overlay_binds_the_declared_digest_and_a_later_edit_is_the_drivers_to_refuse() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("adapters/overlays")).unwrap();
    let text = "provider:\n  display_name: spark-glm\n";
    std::fs::write(dir.path().join("adapters/overlays/spark.yml"), text).unwrap();
    let declared = declared(dir.path(), "adapters/overlays/spark.yml", text);
    let bundle = bundle(dir.path(), single_body(vec!["missing-driver".into()]));
    let command = vec![
        "--patch".to_string(),
        "adapters/overlays/spark.yml".to_string(),
    ];
    let binding = super::route_overlay_binding(&bundle, &command, dir.path(), Some(&declared))
        .expect("the declared overlay binds");
    assert_eq!(binding.value, "adapters/overlays/spark.yml");
    assert_eq!(binding.digest, declared.sha256);

    // The engine never re-hashes the file: a changed byte still travels
    // with the declared digest, and the driver's one read and required
    // equality is the check that refuses it.
    std::fs::write(
        dir.path().join("adapters/overlays/spark.yml"),
        "provider:\n  display_name: changed\n",
    )
    .unwrap();
    let after = super::route_overlay_binding(&bundle, &command, dir.path(), Some(&declared))
        .expect("the binding is still the declaration's");
    assert_eq!(after.digest, declared.sha256);
    assert_ne!(
        sha256_bytes(b"provider:\n  display_name: changed\n"),
        after.digest
    );
}

#[test]
fn a_library_outside_the_working_directory_binds_nothing() {
    let run = tempfile::tempdir().unwrap();
    let library = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(library.path().join("overlays")).unwrap();
    std::fs::write(library.path().join("overlays/spark.yml"), "a: b\n").unwrap();
    let declared = declared(library.path(), "overlays/spark.yml", "a: b\n");
    let bundle = bundle(run.path(), single_body(vec!["missing-driver".into()]));
    let command = vec!["--patch".to_string(), "overlays/spark.yml".to_string()];
    let binds =
        |workdir: &Path| super::route_overlay_binding(&bundle, &command, workdir, Some(&declared));
    assert!(
        binds(run.path()).is_none(),
        "a file outside the run's tree has no working-directory-relative value"
    );
    // A link out of the run tree resolves to the declared file but lies
    // outside workdir: the containment guard withholds the binding.
    std::os::unix::fs::symlink(library.path().join("overlays"), run.path().join("overlays"))
        .unwrap();
    assert!(
        binds(run.path()).is_none(),
        "a symlink out of the run tree has no binding"
    );
    // A same-spelled file in the run tree is a lookalike, not the declared
    // file: the path equality guard withholds the binding.
    std::fs::remove_file(run.path().join("overlays")).unwrap();
    std::fs::create_dir_all(run.path().join("overlays")).unwrap();
    std::fs::write(run.path().join("overlays/spark.yml"), "a: b\n").unwrap();
    assert!(
        binds(run.path()).is_none(),
        "a lookalike is not the declared file"
    );
}

#[test]
fn a_declared_binding_refuses_an_absolute_or_traversing_value() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("adapters/overlays")).unwrap();
    std::fs::write(dir.path().join("adapters/overlays/spark.yml"), "a: b\n").unwrap();
    let declared = declared(dir.path(), "adapters/overlays/spark.yml", "a: b\n");
    let bundle = bundle(dir.path(), single_body(vec!["missing-driver".into()]));
    let valid = vec![
        "--patch".to_string(),
        "adapters/overlays/spark.yml".to_string(),
    ];
    assert!(
        super::route_overlay_binding(&bundle, &valid, dir.path(), Some(&declared)).is_some(),
        "the declared overlay binds before the path checks are exercised"
    );
    for value in ["/etc/passwd", "../outside.yml"] {
        let command = vec!["--patch".to_string(), value.to_string()];
        assert!(
            super::route_overlay_binding(&bundle, &command, dir.path(), Some(&declared)).is_none(),
            "{value}"
        );
    }
}
