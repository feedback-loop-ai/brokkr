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

/// The same refusal, read the way a resume reads a run's pin.
fn pinned_refusal(value: Value) -> String {
    Dialect::parse_pinned("dialect.json", &serde_json::to_string(&value).unwrap())
        .unwrap_err()
        .to_string()
}

#[test]
fn both_shipped_dialects_load_and_satisfy_the_contract() {
    let schema: Value = serde_json::from_slice(
        &std::fs::read(root().join("contracts/dialect.v3.schema.json")).unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    for name in ["openspec", "speckit"] {
        let path = root().join(format!("dialects/{name}.json"));
        let (_, value) = Dialect::load(&path).unwrap();
        assert!(validator.is_valid(&value), "{name} is outside dialect/v3");
    }
    let speckit = Dialect::load(&root().join("dialects/speckit.json"))
        .unwrap()
        .0;
    assert!(speckit.validation("specify").is_none());
    assert!(speckit.validation("clarify").is_none());
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

    // A version this surface does not read is refused BY ITS VERSION, and
    // the refusal names the version that is read. The check runs on the
    // bytes before they are given a shape, so a file from another version
    // is not refused for its unknown fields instead.
    let mut wrong = openspec();
    wrong["schema"] = json!("brokkr.dialect/v4");
    wrong["surprise"] = json!("a field no read version knows");
    let message = refusal(wrong);
    assert!(message.contains("brokkr.dialect/v4"), "{message}");
    assert!(message.contains("brokkr.dialect/v3"), "{message}");
    assert!(message.contains("a run's own pin"), "{message}");

    // A FILE is the newest version: an older one may not be written now
    // to escape what the newest requires.
    let mut older = openspec();
    older["schema"] = json!("brokkr.dialect/v2");
    older["archive"]
        .as_object_mut()
        .unwrap()
        .remove("instructions");
    let message = refusal(older);
    assert!(message.contains("brokkr.dialect/v2"), "{message}");
    assert!(message.contains("brokkr.dialect/v3"), "{message}");
    for pointer in [
        "/name",
        "/tool/binary",
        "/tool/version",
        "/tool/install/package",
    ] {
        let mut value = openspec();
        *value.pointer_mut(pointer).unwrap() = json!(" ");
        assert!(refusal(value).contains("must be non-empty"));
    }
    for binary in [
        "/bin/echo",
        "tools/openspec",
        "tools\\openspec",
        "C:openspec.exe",
    ] {
        let mut value = openspec();
        value["tool"]["binary"] = json!(binary);
        let message = refusal(value);
        assert!(message.contains("bare filename"), "{binary}: {message}");
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

    // Spellings a host might not call absolute: the leading separator in
    // either direction and a Windows prefix are refused everywhere, so the
    // boundary reads the same on every platform.
    for instruction in [
        "/absolute.md",
        "\\\\absolute.md",
        "C:/absolute.md",
        "../outside.md",
        "safe/../../outside.md",
    ] {
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
    value["archive"]["instructions"] = json!("instructions/prompt.md");
    let path = dialect_dir.join("openspec.json");
    std::fs::write(&path, value.to_string()).unwrap();

    let dialect = Dialect::load(&path).unwrap().0;
    assert_eq!(dialect.rendered["specify"], "Realm prompt.");
    assert_eq!(dialect.rendered["review"], "Realm prompt.");
    assert!(dialect.rendered["implement"].contains("Realm prompt."));

    std::fs::remove_file(dialect_dir.join("instructions/prompt.md")).unwrap();
    let missing = dialect_dir.join("instructions/prompt.md");
    match Dialect::load(&path).unwrap_err() {
        DialectError::UnreadableInstruction { path, .. } => {
            assert_eq!(Path::new(&path), missing)
        }
        other => panic!("expected missing instruction refusal, got {other}"),
    }

    // The archive instruction is read after the change-location line, not
    // in the loop above, so a missing archive file is its own refusal and
    // must name the archive path rather than the first artifact prose.
    std::fs::write(
        dialect_dir.join("instructions/prompt.md"),
        "Realm prompt.\n",
    )
    .unwrap();
    std::fs::write(dialect_dir.join("instructions/archive.md"), "Archive.\n").unwrap();
    value["archive"]["instructions"] = json!("instructions/archive.md");
    std::fs::write(&path, value.to_string()).unwrap();
    Dialect::load(&path).unwrap();
    std::fs::remove_file(dialect_dir.join("instructions/archive.md")).unwrap();
    match Dialect::load(&path).unwrap_err() {
        DialectError::UnreadableInstruction { path, .. } => {
            assert_eq!(
                Path::new(&path),
                dialect_dir.join("instructions/archive.md")
            )
        }
        other => panic!("expected missing archive instruction refusal, got {other}"),
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

/// Decision 0042's addendum of 2026-09-06: the archive step appends one
/// provenance line per capability it touched, so a dialect that promotes
/// a living truth tree names the instruction that says how. A dialect
/// with no truth tree declares the archive unsupported and carries none.
#[test]
fn a_promoting_dialect_names_the_archive_instruction_that_appends_provenance() {
    let (dialect, _) =
        Dialect::parse("openspec", &serde_json::to_string(&openspec()).unwrap()).unwrap();
    let ArchiveOrUnsupported::Command(archive) = &dialect.archive else {
        panic!("the OpenSpec dialect promotes a truth tree and folds changes into it");
    };
    assert_eq!(
        archive.instructions.as_deref(),
        Some("openspec/archive.md"),
        "the archive instruction is dialect data, not a hard-coded path"
    );
    let implement = dialect
        .prompt_for(&root().join("dialects"), "implement")
        .unwrap();
    assert!(
        implement.contains("## Provenance"),
        "the implement prompt must tell the smith how folding records the change: {implement}"
    );
    assert!(implement.contains("never rewrite, reorder or remove"));

    let (speckit, _) = Dialect::parse(
        "speckit",
        &String::from_utf8(std::fs::read(root().join("dialects/speckit.json")).unwrap()).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        speckit.archive,
        ArchiveOrUnsupported::Unsupported(_)
    ));

    // A v3 command archive without the instruction is refused, and the
    // refusal says what is missing: folding without the provenance rule
    // would silently break the two-way trail.
    let mut bare = openspec();
    bare["archive"] = json!({"argv": ["openspec", "archive", "{change}", "--yes"]});
    let refused = refusal(bare);
    assert!(
        refused.contains("names the instruction it carries"),
        "a v3 command archive without an instruction must be refused: {refused}"
    );
}

/// A dialect is not only a file on disk: a run pins its resolved content
/// into the manifest, and a resume rehydrates that pin through this same
/// module. So every version this build has ever written is read FROM A
/// PIN, and an older one folds exactly as it did — the archive step of a
/// v2 dialect carries no provenance instruction, because on the day it
/// was pinned there was none. Each version keeps its own shape all the
/// same, and a file on disk is still the newest version only.
#[test]
fn the_older_dialect_versions_a_run_may_have_pinned_still_read() {
    // v2 is v3 without the archive instruction: the shape every run
    // pinned between 2026-09-05 and this slice carries.
    let mut v2 = openspec();
    v2["schema"] = json!(SCHEMA_V2);
    v2["archive"] = json!({"argv": ["openspec", "archive", "{change}", "--yes"]});
    let (parsed, _) = Dialect::parse_pinned("pinned.json", &v2.to_string())
        .expect("a v2 dialect pinned by an older run still reads");
    let ArchiveOrUnsupported::Command(archive) = &parsed.archive else {
        panic!("a v2 archive is still a command");
    };
    assert_eq!(archive.instructions, None);
    let implement = parsed
        .prompt_for(&root().join("dialects"), "implement")
        .expect("a v2 dialect renders the prompt it always rendered");
    assert!(implement.contains("Archive operation:"), "{implement}");
    assert!(
        !implement.contains("## Provenance"),
        "a v2 fold predates the provenance rule and must not claim it: {implement}"
    );

    // v1 is v2 without the install identity.
    let mut v1 = v2.clone();
    v1["schema"] = json!(SCHEMA_V1);
    v1["tool"]["install"] = Value::Null;
    v1["tool"].as_object_mut().unwrap().remove("install");
    let (older, _) = Dialect::parse_pinned("pinned.json", &v1.to_string())
        .expect("a v1 dialect pinned by the first slice's runs still reads");
    assert!(older.tool.install.is_none());

    // A file on disk is the newest version, so an older one cannot be
    // written today to escape what the newest requires.
    let file = refusal(v2.clone());
    assert!(file.contains("a run's own pin"), "{file}");

    // No version borrows a field that landed after it.
    let mut borrowed = v1.clone();
    borrowed["tool"]["install"] = openspec()["tool"]["install"].clone();
    assert!(
        pinned_refusal(borrowed).contains("the install identity landed in brokkr.dialect/v2"),
        "a v1 dialect may not carry v2's install identity"
    );
    let mut instructed = v2.clone();
    instructed["archive"]["instructions"] = json!("openspec/archive.md");
    assert!(
        pinned_refusal(instructed).contains("the archive instruction landed in brokkr.dialect/v3"),
        "a v2 dialect may not carry v3's archive instruction"
    );
    let mut uninstalled = openspec();
    uninstalled["tool"]
        .as_object_mut()
        .unwrap()
        .remove("install");
    assert!(
        refusal(uninstalled).contains("names what installs its tool"),
        "a v3 dialect must carry the install identity"
    );
}

/// Decision 0042's addendum: a dialect names what installs its tool,
/// because the binary's name is not the package's. Measured 2026-09-04:
/// the bare npm name `openspec` is a 0.0.0 placeholder, and spec-kit is
/// a git tag rather than a registry package at all.
#[test]
fn every_shipped_dialect_names_the_package_that_installs_its_binary() {
    for name in ["openspec", "speckit"] {
        let raw = std::fs::read(root().join(format!("dialects/{name}.json"))).unwrap();
        let (dialect, _) = Dialect::parse(name, &String::from_utf8(raw).unwrap()).unwrap();
        let install = dialect
            .tool
            .install
            .expect("every shipped dialect is v3 and names its install");
        assert_ne!(
            install.package, dialect.tool.binary,
            "{name} installs by binary name, which is the trap this field exists for"
        );
    }
    let (parsed, _) =
        Dialect::parse("openspec", &serde_json::to_string(&openspec()).unwrap()).unwrap();
    let install = parsed.tool.install.expect("the shipped dialect is v3");
    assert_eq!(install.manager, Manager::Npm);
    assert_eq!(install.package, "@fission-ai/openspec");
    assert_eq!(install.source, None);

    // An unknown manager is a refusal: this engine never guesses a
    // shell command for an installer it does not know.
    let mut unknown = openspec();
    unknown["tool"]["install"]["manager"] = json!("brew");
    assert!(refusal(unknown).contains("malformed"));

    // A source, where present, is the coordinate the manager is given;
    // an empty one is refused rather than silently dropped.
    let mut blank = openspec();
    blank["tool"]["install"]["source"] = json!(" ");
    assert!(refusal(blank).contains("must be non-empty"));
}
