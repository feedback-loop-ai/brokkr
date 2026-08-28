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
        "driver": {"command": ["true"]},
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
    let bundle = Bundle::compile(&bundle_dir).unwrap();
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
    assert!(
        std::path::Path::new(store_path).ends_with(".forge/secrets.env"),
        "default store path under the workdir: {store_path}"
    );
    assert!(
        store_path.starts_with(dir.path().join("work").to_str().unwrap()),
        "workdir-relative default: {store_path}"
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
fn forge_runtime_production_code_never_touches_the_secret_store() {
    // Grep-style invariant: resolution lives in the exec driver, so
    // no store-reading function is ever named in forge-runtime's
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
