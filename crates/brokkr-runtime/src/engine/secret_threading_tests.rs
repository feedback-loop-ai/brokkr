use super::*;

const POLICY: &str = r#"{
      "schema": "forge.phase-machine/v1",
      "phases": ["work", "review", "done", "stop"],
      "initial": "work",
      "terminal": ["done", "stop"],
      "shippable_from": ["review"],
      "rules": [
        {"id": "W-OK", "from": "work", "result": "built", "next": "review",
         "reason": "work concluded"},
        {"id": "R-OK", "from": "review", "result": "clean", "next": "done",
         "reason": "review concluded"}
      ]
    }"#;

/// The adapter tree the bundle compiles against: a seat that declares
/// secret bindings must seat a driver the operator granted them
/// (decision 0021 ruling 4), and the fixture grants one under a name no
/// vendor answers to.
fn write_adapters(dir: &std::path::Path) -> std::path::PathBuf {
    let adapters = dir.join("adapters");
    std::fs::create_dir_all(&adapters).unwrap();
    std::fs::write(
        adapters.join("granted.json"),
        serde_json::to_string(&json!({
            "provider": "granted",
            "binding_grant": true,
            "binary": "granted",
            "driver": ["{brokkr}", "driver", "granted", "--"],
            "models": {},
            "model_flag": "unsupported",
            "efforts": [],
            "effort_flag": "unsupported",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }))
        .unwrap(),
    )
    .unwrap();
    adapters
}

/// An engine over a compiled two-seat bundle; `work` optionally
/// declares secret bindings.
fn engine_with(dir: &std::path::Path, secrets: Option<Vec<&str>>) -> Engine {
    let bundle_dir = dir.join("bundle");
    std::fs::create_dir_all(bundle_dir.join("roles")).unwrap();
    std::fs::write(bundle_dir.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle_dir.join("roles/role.md"), "# role\n").unwrap();
    let mut work = json!({
        "role": "roles/role.md",
        "results": ["built"],
        "driver": {"command": ["{brokkr}", "driver", "granted", "--", "true"]},
    });
    if let Some(secrets) = secrets {
        work["secrets"] = json!(secrets);
    }
    let config = json!({
        "name": "threading",
        "policy": "policy.json",
        "seats": {
            "work": work,
            "review": {
                "role": "roles/role.md",
                "results": ["clean"],
                "driver": {"command": ["true"]},
            },
        }
    });
    std::fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_string(&config).unwrap(),
    )
    .unwrap();
    let bundle = Bundle::compile_with(
        &bundle_dir,
        std::path::Path::new("/nonexistent-library"),
        &write_adapters(dir),
    )
    .unwrap();
    let store = Store::open(&dir.join("forge.db")).unwrap();
    Engine::start(store, bundle, "threading proof", Some(dir.join("work"))).unwrap()
}

fn work_input(engine: &Engine) -> Value {
    let state = fold(&engine.store.load(&engine.run_id).unwrap()).unwrap();
    engine.seat_input(&state, "work", "fx").unwrap()
}

#[test]
fn declared_names_and_store_path_ride_the_driver_input() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let engine = engine_with(dir.path(), Some(vec!["GH_TOKEN", "API_KEY"]));
    let input = work_input(&engine);
    assert_eq!(input["secrets"], json!(["GH_TOKEN", "API_KEY"]));
    let store_path = input["secrets_file"].as_str().unwrap();
    let store_path = std::path::Path::new(store_path);
    assert!(
        store_path.ends_with(std::path::Path::new(".forge").join("secrets.env")),
        "default store path under the workdir: {}",
        store_path.display()
    );
    assert!(
        store_path.starts_with(dir.path().join("work")),
        "workdir-relative default: {}",
        store_path.display()
    );
}

#[test]
fn secrets_file_override_wins_over_the_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let mut engine = engine_with(dir.path(), Some(vec!["GH_TOKEN"]));
    engine.secrets_file = Some(dir.path().join("elsewhere.env"));
    let input = work_input(&engine);
    assert_eq!(
        input["secrets_file"].as_str().unwrap(),
        dir.path().join("elsewhere.env").to_str().unwrap()
    );
}

#[test]
fn seats_without_bindings_carry_no_secret_keys() {
    // Pre-0012 bundles must rebuild byte-identical seat inputs, or
    // resumed runs would refuse on a digest mismatch.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let engine = engine_with(dir.path(), None);
    let input = work_input(&engine);
    assert!(input.get("secrets").is_none(), "{input}");
    assert!(input.get("secrets_file").is_none(), "{input}");
}

#[test]
fn brokkr_runtime_production_code_never_touches_the_secret_store() {
    // Grep-style invariant: resolution lives in the exec driver, so
    // no store-reading function is ever named in brokkr-runtime's
    // production code (test modules after #[cfg(test)] excluded).
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let production = source.split("#[cfg(test)]").next().unwrap();
        for needle in ["resolve_bindings", "read_store", "store_names", "store_set"] {
            assert!(
                !production.contains(needle),
                "{} names secret-store function '{needle}' in production code",
                path.display()
            );
        }
    }
}
