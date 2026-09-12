use super::*;
use brokkr_core::realms::CrossingList;

const MAP: &str = r#"{
  "schema": "forge.realms/v1",
  "realms": [{"name": "brokkr", "path": "brokkr", "default_branch": "main"}],
  "journal": "state/forge.db"
}"#;

/// A workspace with a map at its root and one realm directory under it.
fn workspace(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("brokkr")).unwrap();
    std::fs::create_dir(dir.path().join("dialects")).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    std::fs::copy(
        root.join("dialects/openspec.json"),
        dir.path().join("dialects/openspec.json"),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("dialects/openspec")).unwrap();
    for name in [
        "specify", "return", "design", "tasks", "clarify", "analyze", "archive",
    ] {
        std::fs::copy(
            root.join(format!("dialects/openspec/{name}.md")),
            dir.path().join(format!("dialects/openspec/{name}.md")),
        )
        .unwrap();
    }
    std::fs::write(dir.path().join("realms.json"), text).unwrap();
    dir
}

fn refusal<T>(result: Result<T, WorldError>) -> String {
    match result {
        Ok(_) => panic!("expected the map to be refused"),
        Err(error) => error.to_string(),
    }
}

/// The map's relative paths are relative to the map file, so a world
/// travels with the workspace it describes.
#[test]
fn a_loaded_world_resolves_its_journal_and_its_realms_against_the_map() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    assert_eq!(world.journal(), dir.path().join("state/forge.db"));
    assert_eq!(
        world.path_of(&world.map.realms[0]),
        dir.path().join("brokkr")
    );
    assert_eq!(
        world.sha256,
        brokkr_core::canonical::sha256_hex(&world.content)
    );
    assert!(format!("{world:?}").contains("brokkr"), "{world:?}");
}

/// An absolute path in a map is used as written — a world may name a
/// repository that does not live under it.
#[test]
fn absolute_paths_in_a_map_are_left_alone() {
    let dir = workspace(MAP);
    let elsewhere = dir.path().join("elsewhere");
    let text = MAP
        .replace(
            "\"path\": \"brokkr\"",
            &format!("\"path\": {:?}", elsewhere),
        )
        .replace(
            "\"state/forge.db\"",
            &format!("{:?}", elsewhere.join("j.db")),
        );
    std::fs::write(dir.path().join("realms.json"), &text).unwrap();
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    assert_eq!(world.path_of(&world.map.realms[0]), elsewhere);
    assert_eq!(world.journal(), elsewhere.join("j.db"));
}

/// A map named at invocation and missing is a refusal, before anything
/// is opened or spawned — never a silent fallback to the default world.
#[test]
fn a_named_map_that_is_not_there_is_a_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let named = dir.path().join("clientx.json");
    let named_refusal = refusal(World::discover(dir.path(), Some(&named)));
    assert!(
        named_refusal.starts_with("no realms map at"),
        "{named_refusal}"
    );
    assert!(named_refusal.contains("clientx.json"), "{named_refusal}");
    // A directory is not a map either.
    let directory = refusal(World::load(dir.path()));
    assert!(directory.starts_with("no realms map at"), "{directory}");
}

/// And a map that is there but does not parse refuses too: the world an
/// operator meant to open is not one the engine may guess at.
#[test]
fn a_malformed_map_refuses_wherever_it_was_found() {
    let named = workspace("{ not json");
    let refusal = refusal(World::discover(named.path(), None));
    assert!(refusal.contains("not a readable realms map"), "{refusal}");
}

/// A file that is not text at all fails at the read, and says which
/// file — the same refusal, one layer earlier.
#[test]
fn a_map_that_is_not_text_names_itself_in_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("realms.json");
    std::fs::write(&path, [0x66, 0x6f, 0xff, 0xfe]).unwrap();
    let refusal = refusal(World::load(&path));
    assert!(refusal.starts_with("reading realms map"), "{refusal}");
    assert!(refusal.contains("realms.json"), "{refusal}");
}

/// No map named, and none in the workspace: no world, and every default
/// downstream stays exactly what it was.
#[test]
fn a_workspace_with_no_map_discovers_no_world() {
    let dir = tempfile::tempdir().unwrap();
    assert!(World::discover(dir.path(), None).unwrap().is_none());
    let found = World::discover(workspace(MAP).path(), None).unwrap();
    assert_eq!(found.unwrap().map.realms[0].name, "brokkr");
}

/// The realm a repository IS. A tree the map does not name gets no
/// realm — facts about it are recorded exactly as they were before any
/// map existed, rather than under an invented name.
#[test]
fn a_repository_is_the_realm_whose_path_it_is() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let realm = world.realm_for(&dir.path().join("brokkr")).unwrap();
    assert_eq!(realm.name, "brokkr");
    // The same tree named by a path that needs resolving.
    let indirect = dir.path().join("brokkr/../brokkr");
    assert_eq!(
        world.realm_for(&indirect).map(|r| r.name.as_str()),
        Some("brokkr")
    );
    assert!(world.realm_for(&dir.path().join("stranger")).is_none());
    // A repository the map does not name has no realm and therefore no
    // house: the selected-realm lookup answers absence rather than
    // borrowing a neighbour's text.
    assert_eq!(world.house_for(&dir.path().join("stranger")).unwrap(), None);
    assert!(format!("{realm:?}").contains("main"), "{realm:?}");
}

/// A map may describe a workspace that is not checked out yet; a realm
/// whose directory is absent compares as written rather than taking the
/// whole lookup down with it.
#[test]
fn a_realm_whose_tree_is_not_there_yet_still_compares() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("realms.json"), MAP).unwrap();
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    assert_eq!(
        world
            .realm_for(&dir.path().join("brokkr"))
            .map(|realm| realm.name.as_str()),
        Some("brokkr")
    );
}

/// The pin: named, hashed, and embedded whole — so a reader holding only
/// the journal can re-derive the digest from the content beside it.
#[test]
fn a_pinned_world_carries_its_own_answer() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let manifest = world
        .pinned(
            &serde_json::json!({"bundle_name": "b", "files": {}}),
            Some(&dir.path().join("brokkr")),
        )
        .unwrap();
    assert_eq!(manifest["bundle_name"], "b");
    let pin = &manifest["realms"];
    assert_eq!(pin["sha256"], world.sha256);
    assert_eq!(pin["map"], world.content);
    assert!(pin["source"].as_str().unwrap().ends_with("realms.json"));
    assert_eq!(
        brokkr_core::canonical::sha256_hex(&pin["map"]),
        pin["sha256"].as_str().unwrap()
    );
    let unselected = world.pinned(&serde_json::json!({}), None).unwrap();
    let replayed = World::from_manifest(&unselected).unwrap().unwrap();
    assert!(replayed
        .house_for(&dir.path().join("brokkr"))
        .unwrap()
        .is_none());
}

/// The pin read back: `brokkr resume` names a journal and no map, and
/// still knows the world, because the run's own manifest carries it —
/// content, digest and source. That is what embedding is for.
#[test]
fn a_run_reads_its_world_back_out_of_its_own_manifest() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let manifest = world
        .pinned(
            &serde_json::json!({"bundle_name": "b", "files": {}}),
            Some(&dir.path().join("brokkr")),
        )
        .unwrap();

    let rehydrated = World::from_manifest(&manifest).unwrap().unwrap();
    assert_eq!(rehydrated.sha256, world.sha256);
    assert_eq!(rehydrated.content, world.content);
    assert_eq!(rehydrated.source, world.source);
    assert_eq!(rehydrated.map.realms[0].name, "brokkr");
    // And it resolves like the world it came from: the source path is
    // pinned too, so the realm the repository IS still answers.
    assert_eq!(
        rehydrated
            .realm_for(&dir.path().join("brokkr"))
            .map(|realm| realm.name.as_str()),
        Some("brokkr")
    );

    // A manifest with no pin is a run that had no world — not an error.
    assert!(World::from_manifest(&serde_json::json!({"files": {}}))
        .unwrap()
        .is_none());
}

/// A pin that does not answer for itself is a refusal, never a quiet
/// fall back to the unkeyed shape: the whole point of the embedding is
/// that the evidence, not the disk, decides.
#[test]
fn a_pin_that_cannot_answer_for_itself_is_refused() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let pinned = |pin: Value| serde_json::json!({"files": {}, "realms": pin});

    assert!(refusal(World::from_manifest(&pinned(json!({})))).contains("it names no source"));
    assert!(
        refusal(World::from_manifest(&pinned(json!({"source": "m.json"}))))
            .contains("it carries no digest")
    );
    assert!(refusal(World::from_manifest(&pinned(
        json!({"source": "m.json", "sha256": world.sha256})
    )))
    .contains("it embeds no map"));

    let tampered = refusal(World::from_manifest(&pinned(json!({
        "source": "m.json",
        "sha256": "0".repeat(64),
        "map": world.content,
    }))));
    assert!(tampered.contains("not the pinned"), "{tampered}");
    assert!(tampered.contains(&world.sha256), "{tampered}");

    // The embedded content is held to the same rules the file was.
    let junk = json!({"schema": "forge.realms/v1", "realms": [], "journal": "j"});
    let refused = refusal(World::from_manifest(&pinned(json!({
        "source": "m.json",
        "sha256": brokkr_core::canonical::sha256_hex(&junk),
        "map": junk,
    }))));
    assert!(refused.contains("names no realms"), "{refused}");
}

// ------------------------------------- many hearths (0026 ruling 1)

/// A v2 world: two realms with their own journals, one falling back to
/// the world's, and a fourth sharing the second realm's hearth.
const MANY: &str = r#"{
  "schema": "forge.realms/v2",
  "realms": [
    {"name": "alpha", "path": "brokkr", "default_branch": "main", "journal": "a/forge.db"},
    {"name": "beta", "path": "brokkr", "default_branch": "main", "journal": "b/forge.db"},
    {"name": "gamma", "path": "brokkr", "default_branch": "main"},
    {"name": "delta", "path": "brokkr", "default_branch": "main", "journal": "b/forge.db"}
  ],
  "journal": "state/forge.db"
}"#;

/// A realm's own journal resolves against the MAP FILE's directory, like
/// every other path a map carries — not against the world journal's.
#[test]
fn a_realms_own_journal_resolves_against_the_map_file() {
    let dir = workspace(MANY);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    assert_eq!(
        world.journal_of(&world.map.realms[0]),
        dir.path().join("a/forge.db")
    );
    // The world's journal is unmoved, and the realm naming none gets it.
    assert_eq!(world.journal(), dir.path().join("state/forge.db"));
    assert_eq!(
        world.journal_of(&world.map.realms[2]),
        dir.path().join("state/forge.db")
    );
}

/// The distinct journals, in map order, with realms sharing one hearth
/// listed together — a fleet reader opens each journal exactly once.
#[test]
fn a_many_hearth_world_enumerates_its_distinct_journals_once_each() {
    let dir = workspace(MANY);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let hearths = world.hearths();
    assert_eq!(
        hearths,
        vec![
            Hearth {
                realms: vec!["alpha".to_string()],
                journal: dir.path().join("a/forge.db"),
            },
            Hearth {
                realms: vec!["beta".to_string(), "delta".to_string()],
                journal: dir.path().join("b/forge.db"),
            },
            Hearth {
                realms: vec!["gamma".to_string()],
                journal: dir.path().join("state/forge.db"),
            },
        ]
    );
    assert_eq!(hearths[1].label(), "beta+delta");
    // A hearth built from a bare journal — a workspace with no map at
    // all — has no realm to name itself by, and says so rather than
    // labelling itself with nothing.
    assert_eq!(
        Hearth {
            realms: Vec::new(),
            journal: PathBuf::from("j.db"),
        }
        .label(),
        "world"
    );
}

/// The regression bar: a v1 world is ONE hearth, so every surface that
/// groups by hearth draws it exactly as it drew it before this existed.
#[test]
fn a_v1_world_is_one_hearth_carrying_the_journal_it_always_had() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let hearths = world.hearths();
    assert_eq!(hearths.len(), 1);
    assert_eq!(hearths[0].journal, world.journal());
    assert_eq!(hearths[0].label(), "brokkr");
}

/// The degenerate many-hearth case: a v2 map whose one realm names the
/// journal the world already names is still ONE hearth, not two.
#[test]
fn a_v2_realm_naming_the_worlds_own_journal_adds_no_hearth() {
    let dir = workspace(
        r#"{
  "schema": "forge.realms/v2",
  "realms": [{"name": "solo", "path": "brokkr", "default_branch": "main",
              "journal": "state/forge.db"}],
  "journal": "state/forge.db"
}"#,
    );
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    assert_eq!(
        world.hearths(),
        vec![Hearth {
            realms: vec!["solo".to_string()],
            journal: dir.path().join("state/forge.db"),
        }]
    );
}

#[test]
fn a_v3_world_pins_house_and_dialect_and_house_content_moves_run_identity() {
    let map = r#"{
  "schema": "forge.realms/v3",
  "realms": [{"name": "brokkr", "path": "brokkr", "default_branch": "main",
              "house": "HOUSE.md", "dialect": "openspec"}],
  "journal": "state/forge.db"
}"#;
    let dir = workspace(map);
    let house = dir.path().join("brokkr").join("HOUSE.md");
    std::fs::write(&house, "First rule.\n").unwrap();
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let repo = dir.path().join("brokkr");
    assert_eq!(world.dialect_for(&repo).unwrap().unwrap().name, "openspec");
    assert!(world.dialect_for(dir.path()).unwrap().is_none());
    let known = &world.map.realms[0];
    let unknown = Realm {
        name: "unknown".into(),
        path: known.path.clone(),
        default_branch: known.default_branch.clone(),
        journal: known.journal.clone(),
        house: known.house.clone(),
        dialect: known.dialect.clone(),
        boundary: None,
        publishes: CrossingList::Absent,
        consumes: CrossingList::Absent,
    };
    assert!(world.dialect_for_realm(&unknown).unwrap().is_none());
    let first = world.pinned(&json!({"files": {}}), Some(&repo)).unwrap();
    assert_eq!(world.house_for(&repo).unwrap(), Some("First rule.\n"));
    assert_eq!(first["realms"]["house"]["content"], "First rule.\n");
    assert_eq!(
        Path::new(first["realms"]["dialect"]["source"].as_str().unwrap()),
        dir.path().join("dialects/openspec.json")
    );

    let rehydrated = World::from_manifest(&first).unwrap().unwrap();
    assert_eq!(rehydrated.house_for(&repo).unwrap(), Some("First rule.\n"));
    assert!(
        rehydrated.dialect_for(&repo).unwrap().unwrap().rendered["specify"].contains("OpenSpec")
    );

    let mut damaged = first.clone();
    damaged["realms"]["dialect"]
        .as_object_mut()
        .unwrap()
        .remove("instructions");
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("carries no instructions"), "{message}");

    let mut damaged = first.clone();
    damaged["realms"]["dialect"]
        .as_object_mut()
        .unwrap()
        .remove("instructions_sha256");
    let message = refusal(World::from_manifest(&damaged));
    assert!(
        message.contains("instructions carry no sha256"),
        "{message}"
    );

    let mut damaged = first.clone();
    damaged["realms"]["dialect"]["instructions_sha256"] = json!("0".repeat(64));
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("instructions hash"), "{message}");

    let mut damaged = first.clone();
    damaged["realms"]["dialect"]["instructions"] = json!("not a phase map");
    damaged["realms"]["dialect"]["instructions_sha256"] = json!(
        brokkr_core::canonical::sha256_hex(&damaged["realms"]["dialect"]["instructions"])
    );
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("instructions are malformed"), "{message}");

    std::fs::write(&house, "Changed rule.\n").unwrap();
    let changed = World::load(&dir.path().join("realms.json"))
        .unwrap()
        .pinned(&json!({"files": {}}), Some(&repo))
        .unwrap();
    assert_ne!(
        first, changed,
        "house content is part of the run's pinned identity"
    );
    assert_ne!(
        first["realms"]["house"]["sha256"],
        changed["realms"]["house"]["sha256"]
    );
}

/// The resume bar for decision 0042's provenance addendum: a run that
/// pinned its dialect before the addendum landed pinned a
/// `brokkr.dialect/v2` body, and rehydrating that manifest reads it
/// through the same parser a file goes through. The older world must come
/// back exactly as it was pinned — its fold carries no provenance
/// instruction, because on the day it was pinned there was none.
#[test]
fn a_run_that_pinned_an_older_dialect_still_rehydrates_its_world() {
    let map = r#"{
  "schema": "forge.realms/v3",
  "realms": [{"name": "brokkr", "path": "brokkr", "default_branch": "main",
              "dialect": "openspec"}],
  "journal": "state/forge.db"
}"#;
    let dir = workspace(map);
    let repo = dir.path().join("brokkr");
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let mut manifest = world.pinned(&json!({"files": {}}), Some(&repo)).unwrap();

    // Age the pinned content back to the shape a pre-addendum run wrote.
    let dialect = manifest["realms"]["dialect"].as_object_mut().unwrap();
    let mut content = dialect["content"].clone();
    content["schema"] = json!("brokkr.dialect/v2");
    content["archive"]
        .as_object_mut()
        .unwrap()
        .remove("instructions");
    dialect.insert(
        "sha256".into(),
        json!(brokkr_core::canonical::sha256_hex(&content)),
    );
    dialect.insert("content".into(), content);

    let rehydrated = World::from_manifest(&manifest)
        .expect("a v2-pinned world resumes")
        .unwrap();
    let pinned = rehydrated.dialect_for(&repo).unwrap().unwrap();
    assert_eq!(pinned.schema, "brokkr.dialect/v2");
    assert!(pinned.rendered["specify"].contains("OpenSpec"));

    // A version this build does not read is refused BY ITS VERSION, so
    // the operator learns what happened rather than reading a variant
    // mismatch from serde.
    let mut future = manifest.clone();
    let dialect = future["realms"]["dialect"].as_object_mut().unwrap();
    let mut content = dialect["content"].clone();
    content["schema"] = json!("brokkr.dialect/v9");
    dialect.insert(
        "sha256".into(),
        json!(brokkr_core::canonical::sha256_hex(&content)),
    );
    dialect.insert("content".into(), content);
    let message = refusal(World::from_manifest(&future));
    assert!(message.contains("brokkr.dialect/v9"), "{message}");
    assert!(
        message.contains("a pinned dialect may be any of"),
        "{message}"
    );
}

#[test]
fn a_path_dialect_is_pinned_and_every_declared_text_pin_must_answer_for_itself() {
    let map = r#"{
  "schema": "forge.realms/v3",
  "realms": [
    {"name": "brokkr", "path": "brokkr", "default_branch": "main",
     "house": "HOUSE.md", "dialect": "spec/dialect.md"},
    {"name": "second", "path": "brokkr", "default_branch": "main",
     "dialect": "dialect.json"}
  ],
  "journal": "state/forge.db"
}"#;
    let dir = workspace(map);
    std::fs::write(dir.path().join("brokkr/HOUSE.md"), "House.\n").unwrap();
    std::fs::create_dir_all(dir.path().join("brokkr/spec")).unwrap();
    std::fs::copy(
        dir.path().join("dialects/openspec.json"),
        dir.path().join("brokkr/spec/dialect.md"),
    )
    .unwrap();
    std::fs::copy(
        dir.path().join("dialects/openspec.json"),
        dir.path().join("brokkr/dialect.json"),
    )
    .unwrap();
    for base in [dir.path().join("brokkr/spec"), dir.path().join("brokkr")] {
        std::fs::create_dir_all(base.join("openspec")).unwrap();
        for name in [
            "specify", "return", "design", "tasks", "clarify", "analyze", "archive",
        ] {
            std::fs::copy(
                dir.path().join(format!("dialects/openspec/{name}.md")),
                base.join(format!("openspec/{name}.md")),
            )
            .unwrap();
        }
    }
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let repo = dir.path().join("brokkr");
    let manifest = world.pinned(&json!({"files": {}}), Some(&repo)).unwrap();
    assert_eq!(manifest["realms"]["dialect"]["content"]["name"], "openspec");
    assert!(manifest["realms"]["dialect"]["instructions"]["specify"]
        .as_str()
        .unwrap()
        .contains("OpenSpec"));
    assert!(World::from_manifest(&manifest).unwrap().is_some());
    let unselected = world.pinned(&json!({"files": {}}), None).unwrap();
    let rehydrated_unselected = World::from_manifest(&unselected).unwrap().unwrap();
    assert_eq!(rehydrated_unselected.house_for(&repo).unwrap(), None);
    let repinned = rehydrated_unselected.pin(Some(&repo)).unwrap();
    assert_eq!(repinned["realm"], "brokkr");
    assert!(repinned.get("house").is_none());
    assert!(repinned.get("dialect").is_none());

    for key in ["house", "dialect"] {
        for (field, expected) in [
            ("source", "carries no source"),
            ("sha256", "carries no sha256"),
            ("content", "carries no content"),
        ] {
            let mut damaged = manifest.clone();
            damaged["realms"][key]
                .as_object_mut()
                .unwrap()
                .remove(field);
            let message = refusal(World::from_manifest(&damaged));
            assert!(message.contains(expected), "{key}: {message}");
        }
    }

    let mut damaged = manifest.clone();
    damaged["realms"]["house"]["content"] = json!("tampered house");
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("pinned house hashes"), "{message}");

    let mut damaged = manifest.clone();
    damaged["realms"]["dialect"]["content"] = json!("moved-dialect");
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("pinned dialect hashes"), "{message}");

    let mut damaged = manifest.clone();
    damaged["realms"]["dialect"]["content"] = json!({"not":"a dialect"});
    damaged["realms"]["dialect"]["sha256"] = json!(brokkr_core::canonical::sha256_hex(
        &damaged["realms"]["dialect"]["content"]
    ));
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("malformed"), "{message}");

    for key in ["house", "dialect"] {
        let mut missing = manifest.clone();
        missing["realms"].as_object_mut().unwrap().remove(key);
        let message = refusal(World::from_manifest(&missing));
        assert!(message.contains(&format!("names a {key}")), "{message}");
        assert!(message.contains("pins none"), "{message}");
    }
}

#[test]
fn a_declared_broken_dialect_is_refused_only_for_its_realm() {
    let map = r#"{
  "schema":"forge.realms/v3",
  "realms":[{"name":"brokkr","path":"brokkr","default_branch":"main",
             "dialect":"broken.json"}],
  "journal":"state/forge.db"
}"#;
    let dir = workspace(map);
    std::fs::write(dir.path().join("brokkr/broken.json"), "{").unwrap();
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let realm = &world.map.realms[0];
    let message = refusal(world.dialect_for_realm(realm));
    assert!(
        message.contains("realm 'brokkr' dialect is unusable"),
        "{message}"
    );
    let message = refusal(world.pin(Some(&dir.path().join("brokkr"))));
    assert!(
        message.contains("realm 'brokkr' dialect is unusable"),
        "{message}"
    );
}

#[test]
fn a_declared_house_must_be_a_readable_file() {
    let map = r#"{
  "schema": "forge.realms/v3",
  "realms": [{"name": "brokkr", "path": "brokkr", "default_branch": "main",
              "house": "missing.md"}],
  "journal": "state/forge.db"
}"#;
    let dir = workspace(map);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let refusal = refusal(world.house_for(&dir.path().join("brokkr")));
    assert!(refusal.contains("realm 'brokkr' names house"), "{refusal}");
    assert!(refusal.contains("missing.md"), "{refusal}");
}

// ---------------------- two DISTINCT repositories (0023 ruling 1, phase 2 ground)

/// A world of two repositories. Every multi-realm map exercised before
/// this pointed its realms at ONE tree; this one names two, each a real
/// git repository with its own commit — so two different HEADs — and its
/// own hearth. The map sits at the workspace root and names both by
/// relative path.
pub(crate) const TWO_REPOSITORIES: &str = r#"{
  "schema": "forge.realms/v2",
  "realms": [
    {"name": "alpha", "path": "alpha", "default_branch": "main",
     "journal": "state/alpha.db"},
    {"name": "beta", "path": "beta", "default_branch": "trunk",
     "journal": "state/beta.db"}
  ],
  "journal": "state/world.db"
}"#;

/// A repository with one commit of its own; the commit's message is the
/// file it adds, so two repositories never share a tree or a sha.
pub(crate) fn repository(root: &Path, name: &str) -> (PathBuf, String) {
    let repo = root.join(name);
    std::fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str]| {
        assert!(std::process::Command::new("git")
            .args(args)
            .current_dir(&repo)
            .status()
            .unwrap()
            .success());
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "Brokkr Test"]);
    git(&["config", "user.email", "brokkr@test"]);
    git(&["config", "commit.gpgSign", "false"]);
    std::fs::write(repo.join(format!("{name}.txt")), name).unwrap();
    git(&["add", "."]);
    git(&["commit", "-q", "-m", name]);
    let head = crate::git_head(&repo).expect("a committed repository has a HEAD");
    (repo, head)
}

/// The fixture: a workspace whose map names two realms in two separate
/// repositories at two different HEADs. Returns the workspace and each
/// repository's HEAD, in map order.
pub(crate) fn two_repositories() -> (tempfile::TempDir, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let (_, alpha) = repository(dir.path(), "alpha");
    let (_, beta) = repository(dir.path(), "beta");
    assert_ne!(alpha, beta, "two repositories, two commits, two shas");
    std::fs::write(dir.path().join("realms.json"), TWO_REPOSITORIES).unwrap();
    (dir, alpha, beta)
}

/// Proof 1. A world of two repositories resolves BOTH realms' paths
/// against the map file's own directory, and `realm_for` answers each
/// repository with its own realm — never the first realm for both, and
/// never a realm for a tree the map does not name.
#[test]
fn a_world_of_two_repositories_resolves_each_realm_to_its_own_tree() {
    let (dir, alpha_head, beta_head) = two_repositories();
    let world = World::discover(dir.path(), None).unwrap().unwrap();
    assert_eq!(world.map.realms.len(), 2);
    let alpha = dir.path().join("alpha");
    let beta = dir.path().join("beta");
    assert_eq!(world.path_of(&world.map.realms[0]), alpha);
    assert_eq!(world.path_of(&world.map.realms[1]), beta);
    assert_eq!(world.realm_for(&alpha).unwrap().name, "alpha");
    assert_eq!(world.realm_for(&beta).unwrap().name, "beta");
    assert_eq!(world.realm_for(&beta).unwrap().default_branch, "trunk");
    assert!(
        world.realm_for(dir.path()).is_none(),
        "the workspace itself is no realm"
    );
    // Two repositories, two HEADs, read from the trees the world resolved.
    assert_eq!(
        crate::git_head(&world.path_of(&world.map.realms[0])),
        Some(alpha_head.clone())
    );
    assert_eq!(
        crate::git_head(&world.path_of(&world.map.realms[1])),
        Some(beta_head.clone())
    );
    assert_ne!(alpha_head, beta_head);
    // Two hearths, one per realm, and the world's own journal unread by
    // either — the fleet reads them side by side, never merged.
    let hearths = world.hearths();
    assert_eq!(
        hearths,
        vec![
            Hearth {
                realms: vec!["alpha".into()],
                journal: dir.path().join("state/alpha.db"),
            },
            Hearth {
                realms: vec!["beta".into()],
                journal: dir.path().join("state/beta.db"),
            },
        ]
    );

    // The same world, drawn from a map that lives one level down and
    // names the repositories by `..`: paths are relative to the MAP
    // FILE's directory, not to the process or the workspace root, so a
    // moved map still finds both trees and still tells them apart.
    std::fs::create_dir(dir.path().join("maps")).unwrap();
    let nested = dir.path().join("maps/world.json");
    std::fs::write(
        &nested,
        TWO_REPOSITORIES
            .replace("\"path\": \"alpha\"", "\"path\": \"../alpha\"")
            .replace("\"path\": \"beta\"", "\"path\": \"../beta\""),
    )
    .unwrap();
    let moved = World::discover(dir.path(), Some(&nested)).unwrap().unwrap();
    assert_eq!(moved.realm_for(&alpha).unwrap().name, "alpha");
    assert_eq!(moved.realm_for(&beta).unwrap().name, "beta");
    assert_eq!(
        moved.journal_of(&moved.map.realms[1]),
        dir.path().join("maps/state/beta.db"),
        "a realm's journal follows the map file too"
    );

    // And each repository pins under its OWN realm name — the run that
    // starts in beta believes in beta, not in whichever realm came first.
    let pinned = world.pin(Some(&beta)).unwrap();
    assert_eq!(pinned["realm"], json!("beta"));
    assert_eq!(world.pin(Some(&alpha)).unwrap()["realm"], json!("alpha"));
    let replayed = World::from_manifest(&world.pinned(&json!({}), Some(&beta)).unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(replayed.realm_for(&beta).unwrap().name, "beta");
    assert_eq!(replayed.realm_for(&alpha).unwrap().name, "alpha");
}

/// The file `alpha` publishes, written with the bytes given. Returns
/// where it was written and the sha256 over exactly those bytes — the pin
/// a consuming realm would carry if it were built against them.
///
/// `pub(crate)`, like the two-repository fixture above and for the same
/// reason: the engine's own tests record this world into a run manifest
/// and must stand on the crossing this module already resolves, rather
/// than on a second fixture that could drift away from it.
pub(crate) fn published_by_alpha(dir: &Path, bytes: &str) -> (PathBuf, String) {
    let crossing = dir.join("alpha").join("contracts/orders.v1.schema.json");
    std::fs::create_dir_all(crossing.parent().unwrap()).unwrap();
    std::fs::write(&crossing, bytes).unwrap();
    let pin = brokkr_core::canonical::sha256_bytes(bytes.as_bytes());
    (crossing, pin)
}

/// The two-repository map with a crossing drawn across it: `alpha`
/// publishes `orders.api`, `beta` consumes it at the pin given. The
/// surgery is on [`TWO_REPOSITORIES`] itself, so every crossing test
/// stands on slice (i)'s world of two DISTINCT repositories rather than a
/// fixture of its own.
pub(crate) fn crossing_map(pin: &str) -> String {
    TWO_REPOSITORIES
        .replace("forge.realms/v2", "forge.realms/v5")
        .replace(
            "\"journal\": \"state/alpha.db\"",
            "\"journal\": \"state/alpha.db\",
     \"publishes\": [{\"name\": \"orders.api\", \"path\": \"contracts/orders.v1.schema.json\"}]",
        )
        .replace(
            "\"journal\": \"state/beta.db\"",
            &format!(
                "\"journal\": \"state/beta.db\",
     \"consumes\": [{{\"name\": \"orders.api\", \"realm\": \"alpha\", \"sha256\": \"{pin}\"}}]"
            ),
        )
}

/// Decision 0057, over the world Phase 2 slice (i) proved: two DISTINCT
/// repositories, one of which publishes a file the other pins. The map
/// loads, each realm answers with its own crossings, and the map's own
/// pin travels into the run manifest verbatim. This is the VOCABULARY's
/// proof — what the map may say, and what a run testifies to; what the
/// loader does with those bytes is proved by the tests below it.
#[test]
fn a_world_of_two_repositories_can_draw_a_crossing_between_them() {
    let (dir, _, _) = two_repositories();
    let (_, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    let world = World::discover(dir.path(), None).unwrap().unwrap();
    let (alpha, beta) = (&world.map.realms[0], &world.map.realms[1]);
    assert_eq!(alpha.published()[0].name, "orders.api");
    assert_eq!(alpha.published()[0].path, "contracts/orders.v1.schema.json");
    assert!(alpha.consumed().is_empty());
    assert_eq!(beta.consumed()[0].realm, "alpha");
    assert_eq!(beta.consumed()[0].sha256, pin);
    assert!(beta.published().is_empty());
    // Two repositories still, each resolved to its own tree: the
    // crossing changed the vocabulary, not the world.
    assert_eq!(world.path_of(alpha), dir.path().join("alpha"));
    assert_eq!(world.path_of(beta), dir.path().join("beta"));
    // And the pinned world carries the crossings verbatim, so a run can
    // testify to the contract it was built against.
    let pinned = world.pin(Some(&dir.path().join("beta"))).unwrap();
    assert_eq!(
        pinned["map"]["realms"][1]["consumes"][0]["sha256"],
        json!(pin)
    );
}

// ---------------------- the crossing, resolved and verified (0057 rulings 1 and 2)

/// The whole crossing, working: `alpha`'s file is where its map row says
/// it is, `beta`'s pin is the digest of those bytes, and the world loads.
/// The resolved crossing is then carried on the World — path and digest —
/// so nothing downstream has to open that file again to say what it was.
#[test]
fn a_matching_pin_loads_and_the_world_carries_what_it_resolved() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    let world = World::discover(dir.path(), None).unwrap().unwrap();

    // Keyed by the realm that PUBLISHES it, resolved against that realm's
    // own worktree — not the consumer's, and not the map's directory.
    let resolved = world
        .crossing("alpha", "orders.api")
        .expect("the published crossing is resolved at load");
    assert_eq!(Path::new(&resolved.source), path);
    assert_eq!(resolved.sha256, pin);
    assert!(
        world.crossing("beta", "orders.api").is_none(),
        "beta consumes the crossing; it does not publish one"
    );
    assert!(
        format!("{world:?}").contains(&pin),
        "a world says what it resolved: {world:?}"
    );

    // And the consuming realm reaches it through its own `consumes` entry,
    // which is the whole of what slice (iv) will need to record.
    let consumed = &world.map.realms[1].consumed()[0];
    let seen = world.crossing(&consumed.realm, &consumed.name).unwrap();
    assert_eq!(seen.sha256, consumed.sha256);

    // Raw bytes, never a canonical form (ruling 2): the same JSON value
    // canonicalised hashes to something else entirely, and a crossing that
    // was Markdown or a `.proto` would have no canonical form at all.
    assert_ne!(
        pin,
        brokkr_core::canonical::sha256_hex(&json!({"title": "orders"})),
        "the pin is over the file's bytes, not over the JSON it happens to hold"
    );
}

/// A byte moves in the publishing realm and the consuming realm's world
/// stops loading. The refusal names both digests, because a reader has to
/// be able to see WHICH contract moved without opening either repository.
#[test]
fn a_moved_contract_refuses_the_load_and_names_both_digests() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    World::discover(dir.path(), None)
        .expect("the pin matches the bytes")
        .unwrap();

    // One byte, in the publisher's tree. The map is untouched.
    std::fs::write(&path, "{\"title\": \"Orders\"}\n").unwrap();
    let observed = brokkr_core::canonical::sha256_bytes(&std::fs::read(&path).unwrap());
    assert_ne!(observed, pin);

    // The refusal is the Result of `World::discover` itself — this is the
    // whole test, before anything else in this process runs.
    let message = refusal(World::discover(dir.path(), None));
    assert!(
        message.contains("realm 'beta' consumes crossing 'orders.api'"),
        "{message}"
    );
    assert!(message.contains("from realm 'alpha'"), "{message}");
    assert!(message.contains(&pin), "the pinned digest: {message}");
    assert!(
        message.contains(&observed),
        "the observed digest: {message}"
    );
    // The contract is named at the publisher's own declared path, never
    // the host location it resolved to: a refusal reaches run journals
    // and seat inputs that must not carry the operator's layout.
    assert!(
        message.contains("contracts/orders.v1.schema.json"),
        "and the file that moved: {message}"
    );
    assert!(
        !message.contains(&path.display().to_string()),
        "no host path in the refusal: {message}"
    );

    // At load, and only at load: no hearth of this world was opened to
    // reach that refusal, so nothing was journaled and no seat could have
    // spawned against a contract that had already moved.
    for journal in ["state/world.db", "state/alpha.db", "state/beta.db"] {
        assert!(
            !dir.path().join(journal).exists(),
            "{journal} was opened before the world was judged"
        );
    }
}

/// A published file that is not there is the PUBLISHER's refusal. The
/// consumer pinned a digest of something; it is not the consumer's fault
/// that the file it names is gone, and the message says so.
#[test]
fn a_published_file_that_is_gone_refuses_and_names_its_publisher() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    std::fs::remove_file(&path).unwrap();
    let message = refusal(World::discover(dir.path(), None));
    assert!(
        message.contains("realm 'alpha' publishes crossing 'orders.api'"),
        "{message}"
    );
    assert!(
        message.contains(&path.display().to_string()),
        "and the path it named: {message}"
    );
    assert!(
        !message.contains("beta"),
        "a publisher's missing file is not read as the consumer's wrong pin: {message}"
    );
}

/// The realm is in the map, its tree is not on the disk. Nothing is
/// fetched: how a consumed crossing's bytes reach a realm that is not
/// co-located is deliberately unsettled (0057's Context), and a world
/// that cannot see the file simply refuses to load.
#[test]
fn a_crossing_from_a_realm_that_is_not_checked_out_refuses_rather_than_fetches() {
    let (dir, _, _) = two_repositories();
    let (_, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(
        dir.path().join("realms.json"),
        crossing_map(&pin).replace("\"path\": \"alpha\"", "\"path\": \"not-checked-out\""),
    )
    .unwrap();
    let message = refusal(World::discover(dir.path(), None));
    assert!(
        message.contains("realm 'alpha' publishes crossing 'orders.api'"),
        "{message}"
    );
    assert!(message.contains("not-checked-out"), "{message}");
    assert!(
        !dir.path().join("not-checked-out").exists(),
        "nothing was fetched, and nothing was written to make the map true"
    );
}

// ---------------------- what the run stood on (0057, recorded on 0023 ruling 4's terms)

/// The pin the manifest carries: keyed by the realm that PUBLISHES the
/// file and the name it publishes it under, carrying the path the bytes
/// were read from and the digest they hashed to. A sibling of `realms`,
/// never nested inside it, because the map is what the world DECLARED
/// and this is what it OBSERVED.
#[test]
fn a_pinned_world_carries_the_crossings_it_stood_on_beside_the_map() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    let world = World::discover(dir.path(), None).unwrap().unwrap();
    let beta = dir.path().join("beta");
    let manifest = world.pinned(&json!({"files": {}}), Some(&beta)).unwrap();

    let crossings = &manifest["crossings"];
    assert_eq!(
        crossings["alpha"]["orders.api"],
        json!({"source": path.display().to_string(), "sha256": pin}),
    );
    // Keyed by the publisher, so the consuming realm — which is the realm
    // this run is standing in — adds no key of its own.
    assert!(crossings.get("beta").is_none(), "{crossings}");
    // Beside the map, not inside it: the declaration and the observation
    // are two answers to two questions and a reader may hold both.
    assert!(manifest["realms"].get("crossings").is_none());
    assert_eq!(
        manifest["realms"]["map"]["realms"][1]["consumes"][0]["sha256"],
        json!(pin),
        "the DECLARED pin still rides inside the embedded map, from v4"
    );
    // Everything the world pinned before this existed is untouched.
    assert_eq!(manifest["realms"]["sha256"], json!(world.sha256));
    assert_eq!(manifest["files"], json!({}));
}

/// OBSERVED, never a copy of the declaration. A crossing nobody consumes
/// is DECLARED nowhere at all — no `consumes` entry names a digest — and
/// the manifest still records what was on the disk. Rewriting the
/// published file and reloading moves the recorded digest with the bytes,
/// which is what tells observation and declaration apart.
#[test]
fn the_recorded_digest_follows_the_bytes_and_not_any_declaration() {
    let (dir, _, _) = two_repositories();
    let (_, first) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    // The crossing map with beta's `consumes` taken back out: alpha still
    // publishes, and now nothing in the world declares a digest.
    let mut map: Value = serde_json::from_str(&crossing_map(&first)).unwrap();
    map["realms"][1].as_object_mut().unwrap().remove("consumes");
    let write = |map: &Value| std::fs::write(dir.path().join("realms.json"), map.to_string());
    write(&map).unwrap();
    let published_only = World::discover(dir.path(), None).unwrap().unwrap();
    let pinned = |world: &World| world.pinned(&json!({}), None).unwrap();
    assert_eq!(
        pinned(&published_only)["crossings"]["alpha"]["orders.api"]["sha256"],
        json!(first),
        "a crossing with no consumer is still what this run stood on"
    );
    assert!(
        !serde_json::to_string(&map).unwrap().contains(&first),
        "and nothing in the map declares that digest"
    );

    // The bytes move; the world still loads, because no pin was broken —
    // and the manifest records the NEW digest. A second copy of a
    // declaration could not have moved here, and would not have.
    let (_, second) = published_by_alpha(dir.path(), "{\"title\": \"orders v2\"}\n");
    assert_ne!(first, second);
    let moved = World::discover(dir.path(), None).unwrap().unwrap();
    assert_eq!(
        pinned(&moved)["crossings"]["alpha"]["orders.api"]["sha256"],
        json!(second),
    );
}

/// A world that draws no crossing writes the exact shape it always wrote:
/// the key is omitted, never written empty. The proof is field for field
/// against the manifest the same world pinned before this property
/// existed — which is that manifest with nothing added.
#[test]
fn a_world_with_no_crossing_pins_the_exact_shape_it_always_did() {
    let (dir, _, _) = two_repositories();
    let world = World::discover(dir.path(), None).unwrap().unwrap();
    let beta = dir.path().join("beta");
    let manifest = world.pinned(&json!({"files": {}}), Some(&beta)).unwrap();
    assert!(manifest.get("crossings").is_none(), "{manifest}");
    assert_eq!(
        manifest,
        json!({"files": {}, "realms": world.pin(Some(&beta)).unwrap()}),
        "a world with no crossing pins the map and nothing else",
    );
    // And a world replayed from that manifest resolves none, deliberately:
    // the pin is testimony about the past, not a live reading.
    let replayed = World::from_manifest(&manifest).unwrap().unwrap();
    assert!(replayed.crossing("alpha", "orders.api").is_none());
    assert!(replayed
        .pinned(&json!({}), None)
        .unwrap()
        .get("crossings")
        .is_none());
}

// ---------------------- the same fact, refused and reported (0046's Addendum)

/// Doctor's half of decision 0057. The world a crossing has moved under
/// still LOADS through `World::inspect` — every realm, every path, every
/// house and dialect still answerable — and the mismatch comes back as
/// data, keyed to the realm and the crossing that failed. `World::load`
/// over the same bytes still refuses, in the same breath, with the same
/// words: one reading of the disk, one wording, two behaviours.
#[test]
fn a_moved_crossing_is_data_to_inspect_and_a_refusal_to_load() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    std::fs::write(&path, "{\"title\": \"Orders\"}\n").unwrap();
    let observed = brokkr_core::canonical::sha256_bytes(&std::fs::read(&path).unwrap());

    // The verbs that start or continue a run still end here, unchanged.
    let refused = refusal(World::discover(dir.path(), None));
    assert!(
        refused.contains("realm 'beta' consumes crossing 'orders.api'"),
        "{refused}"
    );

    // Doctor's world exists anyway, and is a whole world: both realms,
    // both trees, both hearths — not one line saying the map is broken.
    let world = World::inspect(dir.path(), None).unwrap().unwrap();
    assert_eq!(world.map.realms.len(), 2);
    assert_eq!(world.path_of(&world.map.realms[1]), dir.path().join("beta"));
    assert_eq!(world.hearths().len(), 2);

    // One report per realm that draws a crossing: alpha publishes and is
    // sound, beta consumes and is not — and beta's failure carries the
    // refusal verbatim, never a second wording of it.
    let reports = world.crossings_report();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].realm, "alpha");
    assert_eq!((reports[0].published, reports[0].consumed), (1, 0));
    assert!(reports[0].failures.is_empty(), "alpha published its file");
    assert_eq!(reports[1].realm, "beta");
    assert_eq!((reports[1].published, reports[1].consumed), (0, 1));
    let failure = &reports[1].failures[0];
    assert_eq!(
        (failure.realm(), failure.crossing()),
        ("beta", "orders.api")
    );
    assert_eq!(failure.error().to_string(), refused);
    // The refusal names the contract at the publisher's own declared
    // path — never the host location the bytes resolved to, which no run
    // journal and no seat input may learn (decision 0020 ruling 1).
    for fact in [&pin, &observed, "contracts/orders.v1.schema.json"] {
        assert!(refused.contains(fact), "{refused}");
    }
    assert!(
        !refused.contains(&path.display().to_string()),
        "the refusal names no host path: {refused}"
    );
    assert_eq!(failure.publisher(), Some("alpha"));

    // And the published file that IS there was still resolved, so a
    // sound realm's crossing reads back under a world holding a broken
    // one — an inspected world is short of what it could not read, not
    // short of everything.
    assert_eq!(
        world.crossing("alpha", "orders.api").unwrap().sha256,
        observed
    );
}

/// The publisher's fault stays the publisher's, on both surfaces: an
/// unreadable published file is reported against the realm that publishes
/// it, and it is the refusal `World::load` gives even though a consumer's
/// pin cannot match a file that is not there either. The consumer is
/// charged nothing — there is nothing to compare — but its pin is carried
/// back as UNCHECKED, so no reader is told the pin was verified against
/// bytes that were never read.
#[test]
fn a_published_file_that_is_gone_is_reported_against_its_publisher() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    std::fs::remove_file(&path).unwrap();

    let world = World::inspect(dir.path(), None).unwrap().unwrap();
    let reports = world.crossings_report();
    let failure = &reports[0].failures[0];
    assert_eq!(
        (failure.realm(), failure.crossing()),
        ("alpha", "orders.api")
    );
    // An unreadable publication is the PUBLISHER's own fault and has no
    // separate publishing realm to name; only a moved pin does.
    assert_eq!(failure.publisher(), None);
    assert!(!failure.moved(), "an unreadable file is not a moved pin");
    assert!(reports[0].unchecked.is_empty(), "alpha consumes nothing");
    assert!(reports[1].failures.is_empty(), "beta pinned nothing wrong");
    assert_eq!(reports[1].consumed, 1);
    let unchecked = &reports[1].unchecked;
    assert_eq!(unchecked.len(), 1, "beta's one pin met no bytes");
    assert_eq!(
        (
            unchecked[0].publisher.as_str(),
            unchecked[0].crossing.as_str()
        ),
        ("alpha", "orders.api"),
        "the pin names the realm that owes the bytes"
    );
    assert_eq!(
        failure.error().to_string(),
        refusal(World::discover(dir.path(), None)),
        "one fact, one wording, whichever surface asks"
    );
}

/// A world that draws no crossing reports none: doctor adds no line to a
/// world that never heard the word, exactly as such a world writes no
/// manifest key. And `inspect` answers such a world identically to
/// `discover`, which is what makes it safe to be doctor's only loader.
#[test]
fn a_world_with_no_crossing_reports_none_and_inspects_as_it_discovers() {
    let (dir, _, _) = two_repositories();
    let inspected = World::inspect(dir.path(), None).unwrap().unwrap();
    let discovered = World::discover(dir.path(), None).unwrap().unwrap();
    assert!(inspected.crossings_report().is_empty());
    assert_eq!(inspected.sha256, discovered.sha256);
    assert_eq!(
        inspected.pinned(&json!({}), None).unwrap(),
        discovered.pinned(&json!({}), None).unwrap()
    );
}

/// The fence a resumed run stands behind. A world rehydrated from a run
/// manifest has met no disk — `from_manifest` resolves no crossing, and
/// reports none — so `verify_crossings` is what asks the disk, against the
/// workspace the operator is standing in. It passes while the bytes are
/// the bytes the run was built on, and refuses with the same four facts
/// the moment they are not.
#[test]
fn a_replayed_world_is_fenced_against_the_disk_it_stands_on_now() {
    let (dir, _, _) = two_repositories();
    let (path, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    let world = World::discover(dir.path(), None).unwrap().unwrap();
    let manifest = world
        .pinned(&json!({}), Some(&dir.path().join("beta")))
        .unwrap();

    // What resume rehydrates: the run's own map, and no crossing at all.
    let replayed = World::from_manifest(&manifest).unwrap().unwrap();
    assert!(replayed.crossing("alpha", "orders.api").is_none());
    assert!(replayed.crossings_report().is_empty());

    // The bytes are still the bytes, so the fence lets the run through.
    // The pinned source is `<dir>/realms.json`, absolute here, so the
    // workspace given is the one a `--repo`-less verb would stand in.
    replayed
        .verify_crossings(dir.path())
        .expect("the contract has not moved");

    // One byte in the publishing realm, and the same refusal — named, and
    // carrying which realm, which crossing, the pin and what is there now.
    std::fs::write(&path, "{\"title\": \"Orders\"}\n").unwrap();
    let observed = brokkr_core::canonical::sha256_bytes(&std::fs::read(&path).unwrap());
    let message = refusal(replayed.verify_crossings(dir.path()));
    assert!(
        message.contains("realm 'beta' consumes crossing 'orders.api'"),
        "{message}"
    );
    assert!(message.contains("from realm 'alpha'"), "{message}");
    assert!(message.contains(&pin), "the pinned digest: {message}");
    assert!(
        message.contains(&observed),
        "the observed digest: {message}"
    );

    // And the same two repositories with no crossing drawn between them:
    // the fence reads nothing and lets the run through, so a resume in a
    // world that never heard the word reaches its drive as it always did.
    std::fs::write(dir.path().join("realms.json"), TWO_REPOSITORIES).unwrap();
    let plain = World::load(&dir.path().join("realms.json")).unwrap();
    let replayed = World::from_manifest(&plain.pinned(&json!({}), None).unwrap())
        .unwrap()
        .unwrap();
    replayed.verify_crossings(dir.path()).unwrap();
}

#[test]
fn an_unreadable_neighbour_house_does_not_refuse_the_selected_realm() {
    let map = r#"{
  "schema": "forge.realms/v3",
  "realms": [
    {"name": "here", "path": "here", "default_branch": "main",
     "house": "HOUSE.md"},
    {"name": "away", "path": "not-checked-out", "default_branch": "main",
     "house": "HOUSE.md"}
  ],
  "journal": "state/forge.db"
}"#;
    let dir = workspace(map);
    std::fs::create_dir_all(dir.path().join("here")).unwrap();
    std::fs::write(dir.path().join("here/HOUSE.md"), "Here.\n").unwrap();
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let here = dir.path().join("here");
    assert_eq!(world.house_for(&here).unwrap(), Some("Here.\n"));
    assert_eq!(
        world.pinned(&json!({"files": {}}), Some(&here)).unwrap()["realms"]["house"]["content"],
        "Here.\n"
    );
}
