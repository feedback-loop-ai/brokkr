use super::*;
use serde_json::json;

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

/// The adapter tree these bundles compile against. A seat that declares
/// secret bindings must seat a driver the operator granted them
/// (decision 0021 ruling 4), so the fixture grants one — and its name is
/// invented here rather than borrowed from a vendor, which is the whole
/// point: the engine matches on the declaration, never on the name.
fn write_adapters(dir: &Path) -> PathBuf {
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
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }))
        .unwrap(),
    )
    .unwrap();
    adapters
}

/// A minimal compilable bundle whose `work` seat carries the given
/// declaration and command template. The template rides BEHIND the
/// driver dispatch decision 0009 fixed, which is where the compiler
/// reads an inline seat's driver identity from.
fn write_bundle(dir: &Path, secrets: Value, command: Value) -> PathBuf {
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    let mut dispatched = vec![
        json!("{brokkr}"),
        json!("driver"),
        json!("granted"),
        json!("--"),
    ];
    dispatched.extend(command.as_array().unwrap().iter().cloned());
    let mut work = json!({
        "role": "roles/role.md",
        "results": ["built"],
        "driver": {"command": dispatched},
    });
    if !secrets.is_null() {
        work["secrets"] = secrets;
    }
    let config = json!({
        "name": "secrets-lint",
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
        bundle.join("bundle.json"),
        serde_json::to_string_pretty(&config).unwrap(),
    )
    .unwrap();
    bundle
}

/// Compile against the fixture adapter tree beside the bundle. The
/// library root is deliberately absent: no seat here names an agent, so
/// a missing one stays the non-event it always was.
fn compile(dir: &Path, bundle: &Path) -> Result<Bundle, CompileError> {
    Bundle::compile_with(
        bundle,
        Path::new("/nonexistent-library"),
        &write_adapters(dir),
    )
}

fn compile_error(secrets: Value, command: Value) -> String {
    let dir = tempfile::tempdir().unwrap();
    let bundle = write_bundle(dir.path(), secrets, command);
    match compile(dir.path(), &bundle) {
        Ok(_) => panic!("expected a compile refusal"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn declared_and_referenced_template_compiles() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = write_bundle(
        dir.path(),
        json!(["GH_TOKEN"]),
        json!(["bash", "-c", "curl -H 'auth: {{secret:GH_TOKEN}}' x"]),
    );
    let compiled = compile(dir.path(), &bundle).unwrap();
    assert_eq!(compiled.seats["work"].secrets, vec!["GH_TOKEN"]);
}

#[test]
fn declared_but_unreferenced_compiles() {
    // The lint is one-directional (referenced => declared): env-only
    // consumers like `gh` reading GH_TOKEN take no argv reference.
    let dir = tempfile::tempdir().unwrap();
    let bundle = write_bundle(dir.path(), json!(["GH_TOKEN"]), json!(["gh", "pr", "list"]));
    assert_eq!(
        compile(dir.path(), &bundle).unwrap().seats["work"].secrets,
        vec!["GH_TOKEN"]
    );
}

#[test]
fn undeclared_reference_refuses_naming_the_secret() {
    let error = compile_error(Value::Null, json!(["echo", "{{secret:GH_TOKEN}}"]));
    assert!(error.contains("undeclared secret 'GH_TOKEN'"), "{error}");
}

#[test]
fn malformed_references_refuse_at_compile() {
    for (part, why) in [
        ("{{secret:gh_token}}", "lowercase"),
        ("{{secret:}}", "empty"),
        ("{{ secret:GH_TOKEN }}", "interior whitespace"),
        ("{{secret:GH_TOKEN", "unclosed"),
    ] {
        let error = compile_error(json!(["GH_TOKEN"]), json!(["echo", part]));
        assert!(
            error.contains("malformed secret reference"),
            "{why} ({part:?}): {error}"
        );
    }
}

#[test]
fn ill_formed_and_denylisted_declarations_refuse() {
    let error = compile_error(json!(["gh_token"]), json!(["true"]));
    assert!(error.contains("[A-Z][A-Z0-9_]*"), "{error}");
    for name in ["PATH", "FORGE_X"] {
        let error = compile_error(json!([name]), json!(["true"]));
        assert!(error.contains("denylisted"), "{name}: {error}");
        assert!(error.contains(name), "{name}: {error}");
    }
    let error = compile_error(json!(["GH_TOKEN", "GH_TOKEN"]), json!(["true"]));
    assert!(error.contains("twice"), "{error}");
}

#[test]
fn secrets_env_inside_the_bundle_dir_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = write_bundle(dir.path(), Value::Null, json!(["true"]));
    std::fs::write(bundle.join("secrets.env"), "GH_TOKEN=oops\n").unwrap();
    let error = compile(dir.path(), &bundle).unwrap_err().to_string();
    assert!(error.contains("secrets.env"), "{error}");
    assert!(error.contains("outside the bundle"), "{error}");
}

#[test]
fn rotation_never_changes_the_manifest_digest() {
    // End to end: set -> compile -> rotate the value -> compile ->
    // digests byte-equal. Holds because the store lives OUTSIDE the
    // bundle dir and digests carry names only.
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join(".forge/secrets.env");
    brokkr_protocol::secret::store_set(&store, "GH_TOKEN", "first-value").unwrap();
    let bundle = write_bundle(
        dir.path(),
        json!(["GH_TOKEN"]),
        json!(["bash", "-c", "echo {{secret:GH_TOKEN}}"]),
    );
    let before = compile(dir.path(), &bundle).unwrap().manifest_digest();
    brokkr_protocol::secret::store_set(&store, "GH_TOKEN", "rotated-value").unwrap();
    let after = compile(dir.path(), &bundle).unwrap().manifest_digest();
    assert_eq!(before, after, "rotation must never change a digest");
}
