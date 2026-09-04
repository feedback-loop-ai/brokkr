use super::*;
use serde_json::json;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn openspec() -> Value {
    serde_json::from_slice(&std::fs::read(root().join("dialects/openspec.json")).unwrap()).unwrap()
}

fn refusal(value: Value) -> String {
    Dialect::parse("dialect.json", &serde_json::to_string(&value).unwrap())
        .unwrap_err()
        .to_string()
}

#[test]
fn both_shipped_dialects_load_and_satisfy_the_contract() {
    let schema: Value = serde_json::from_slice(
        &std::fs::read(root().join("contracts/dialect.v1.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    for name in ["openspec", "speckit"] {
        let path = root().join(format!("dialects/{name}.json"));
        let (_, value) = Dialect::load(&path).unwrap();
        assert!(validator.is_valid(&value), "{name} is outside dialect/v1");
    }
}

#[test]
fn the_closed_map_refuses_unknown_unfilled_optional_only_and_reversed_order() {
    let mut unknown = openspec();
    unknown["surprise"] = Value::Bool(true);
    assert!(refusal(unknown).contains("unknown field"));

    let mut unfilled = openspec();
    unfilled["phases"]["design"]["steps"] = serde_json::json!([]);
    assert!(refusal(unfilled).contains("phase 'design' is unfilled"));

    let mut optional = openspec();
    optional["phases"]["tasks"]["steps"][0]["optional"] = Value::Bool(true);
    assert!(refusal(optional).contains("phase 'tasks' has no required step"));

    let mut reversed = openspec();
    reversed["phases"]["specify"]["steps"][0]["artifacts"] =
        serde_json::json!(["specs", "proposal"]);
    let message = refusal(reversed);
    assert!(message.contains("specs"), "{message}");
    assert!(message.contains("proposal"), "{message}");
}

#[test]
fn unsupported_is_accepted_only_in_the_declared_places() {
    let mut value = openspec();
    value["truth"] = serde_json::json!({"unsupported": "there is no living truth"});
    value["verify"] = serde_json::json!({"unsupported": "there is no verifier"});
    assert!(Dialect::parse("dialect.json", &value.to_string()).is_ok());

    value["phases"]["design"] = serde_json::json!({"unsupported": "no design"});
    let message = refusal(value);
    assert!(
        message.contains("missing field") || message.contains("unknown field"),
        "{message}"
    );
}

#[test]
fn library_names_and_realm_paths_resolve_to_their_distinct_roots() {
    assert_eq!(
        library_path(Path::new("/world"), "openspec", Path::new("/realm")),
        Path::new("/world/dialects/openspec.json")
    );
    assert_eq!(
        library_path(
            Path::new("/world"),
            "config/dialect.json",
            Path::new("/realm")
        ),
        Path::new("/realm/config/dialect.json")
    );
    assert_eq!(
        library_path(Path::new("/world"), "custom.json", Path::new("/realm")),
        Path::new("/realm/custom.json")
    );
}

#[test]
fn every_checked_dialect_boundary_is_named() {
    assert!(Dialect::load(Path::new("missing-dialect.json"))
        .unwrap_err()
        .to_string()
        .contains("missing-dialect.json"));
    assert!(Dialect::parse("broken.json", "{")
        .unwrap_err()
        .to_string()
        .contains("malformed"));

    let mut wrong = openspec();
    wrong["schema"] = json!("brokkr.dialect/v2");
    assert!(refusal(wrong).contains("brokkr.dialect/v1"));
    for pointer in ["/name", "/tool/binary", "/tool/version"] {
        let mut value = openspec();
        *value.pointer_mut(pointer).unwrap() = json!(" ");
        assert!(refusal(value).contains("must be non-empty"));
    }
    for pointer in [
        "/phases/specify/steps/0/name",
        "/phases/design/steps/0/name",
    ] {
        let mut value = openspec();
        *value.pointer_mut(pointer).unwrap() = json!("");
        assert!(refusal(value).contains("empty step or artifact list"));
    }
    let mut empty_artifacts = openspec();
    empty_artifacts["phases"]["tasks"]["steps"][0]["artifacts"] = json!([]);
    assert!(refusal(empty_artifacts).contains("empty step or artifact list"));

    let mut duplicate = openspec();
    duplicate["phases"]["design"]["steps"][0]["artifacts"] = json!(["proposal"]);
    assert!(refusal(duplicate).contains("assigned more than once"));
    for (field, expected) in [
        ("before", "unassigned artifact"),
        ("after", "unassigned artifact"),
    ] {
        let mut value = openspec();
        value["order"][0][field] = json!("ghost");
        assert!(refusal(value).contains(expected));
    }
    for phase in ["clarify", "analyze"] {
        let mut value = openspec();
        value["phases"][phase]["taxonomy"] = json!(" ");
        assert!(refusal(value).contains("has no taxonomy"));
    }

    for pointer in [
        "/phases/specify/validate/argv/0",
        "/phases/design/validate/state/0",
        "/phases/tasks/validate/argv/0",
        "/phases/clarify/check/argv/0",
        "/phases/analyze/check/argv/0",
        "/verify/argv/0",
        "/archive/argv/0",
    ] {
        let mut value = openspec();
        *value.pointer_mut(pointer).unwrap() = json!("{unknown}");
        assert!(refusal(value).contains("unknown placeholder"));
    }
    let mut unmatched = openspec();
    unmatched["verify"]["argv"][0] = json!("unknown}");
    assert!(refusal(unmatched).contains("unknown placeholder"));

    for instruction in ["/absolute.md", "../outside.md", "safe/../../outside.md"] {
        let mut value = openspec();
        value["phases"]["specify"]["steps"][0]["instructions"] = json!(instruction);
        let message = refusal(value);
        assert!(message.contains(instruction), "{message}");
        assert!(message.contains("must be relative"), "{message}");
    }
}

#[test]
fn a_realm_path_dialect_loads_beside_its_instructions_and_missing_prose_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let dialect_dir = dir.path().join(".brokkr");
    std::fs::create_dir_all(dialect_dir.join("instructions")).unwrap();
    std::fs::write(
        dialect_dir.join("instructions/prompt.md"),
        "Realm prompt.\n",
    )
    .unwrap();
    let mut value = openspec();
    for phase in ARTIFACT_PHASES {
        for step in value["phases"][phase]["steps"].as_array_mut().unwrap() {
            step["instructions"] = json!("instructions/prompt.md");
            step["return_instructions"] = json!("instructions/prompt.md");
        }
    }
    value["phases"]["clarify"]["taxonomy"] = json!("instructions/prompt.md");
    value["phases"]["analyze"]["taxonomy"] = json!("instructions/prompt.md");
    let path = dialect_dir.join("openspec.json");
    std::fs::write(&path, value.to_string()).unwrap();

    let dialect = Dialect::load(&path).unwrap().0;
    assert_eq!(dialect.rendered["specify"], "Realm prompt.");
    assert_eq!(dialect.rendered["review"], "Realm prompt.");

    std::fs::remove_file(dialect_dir.join("instructions/prompt.md")).unwrap();
    let missing = dialect_dir.join("instructions/prompt.md");
    match Dialect::load(&path).unwrap_err() {
        DialectError::UnreadableInstruction { path, .. } => {
            assert_eq!(Path::new(&path), missing)
        }
        other => panic!("expected missing instruction refusal, got {other}"),
    }
}

#[test]
fn validation_is_total_over_the_closed_phase_vocabulary() {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    for phase in ["specify", "design", "tasks", "clarify", "analyze"] {
        assert!(dialect.validation(phase).is_some(), "{phase}");
    }
    assert!(dialect.validation("implement").is_none());

    let mut value = openspec();
    value["phases"]["tasks"]["validate"] = json!({"unsupported":"no validator"});
    value["phases"]["analyze"]["check"] = json!({"unsupported":"no check"});
    let dialect = Dialect::parse("optional.json", &value.to_string())
        .unwrap()
        .0;
    assert!(dialect.validation("tasks").is_none());
    assert!(dialect.validation("analyze").is_none());
    assert!(dialect.phases.artifact("unknown").is_none());
    assert!(dialect.phases.loop_phase("unknown").is_none());
}

#[test]
fn rendered_instructions_cover_every_seated_phase_and_ignore_no_phase() {
    let dialect = Dialect::load(&root().join("dialects/openspec.json"))
        .unwrap()
        .0;
    for phase in DIALECT_PHASES.into_iter().chain(["implement", "review"]) {
        assert!(
            !dialect
                .prompt_for(&root().join("dialects"), phase)
                .unwrap()
                .is_empty(),
            "{phase}"
        );
    }
    assert_eq!(
        dialect
            .prompt_for(&root().join("dialects"), "ship")
            .unwrap(),
        ""
    );
}
