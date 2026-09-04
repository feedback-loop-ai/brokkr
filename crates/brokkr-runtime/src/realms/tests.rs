use super::*;

const MAP: &str = r#"{
  "schema": "forge.realms/v1",
  "realms": [{"name": "brokkr", "path": "brokkr", "default_branch": "main"}],
  "journal": "state/forge.db"
}"#;

/// A workspace with a map at its root and one realm directory under it.
fn workspace(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("brokkr")).unwrap();
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
    let first = world.pinned(&json!({"files": {}}), Some(&repo)).unwrap();
    assert_eq!(world.house_for(&repo).unwrap(), Some("First rule.\n"));
    assert_eq!(first["realms"]["house"]["content"], "First rule.\n");
    assert_eq!(first["realms"]["dialect"]["source"], "openspec");

    let rehydrated = World::from_manifest(&first).unwrap().unwrap();
    assert_eq!(rehydrated.house_for(&repo).unwrap(), Some("First rule.\n"));

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
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let repo = dir.path().join("brokkr");
    let manifest = world.pinned(&json!({"files": {}}), Some(&repo)).unwrap();
    assert_eq!(manifest["realms"]["dialect"]["content"], "spec/dialect.md");
    assert!(World::from_manifest(&manifest).unwrap().is_some());
    let unselected = world.pinned(&json!({"files": {}}), None).unwrap();
    let rehydrated_unselected = World::from_manifest(&unselected).unwrap().unwrap();
    assert_eq!(rehydrated_unselected.house_for(&repo).unwrap(), None);
    let repinned = rehydrated_unselected.pin(Some(&repo)).unwrap();
    assert_eq!(repinned["realm"], "brokkr");
    assert!(repinned.get("house").is_none());
    assert!(repinned.get("dialect").is_none());

    for (field, expected) in [
        ("source", "carries no source"),
        ("sha256", "carries no sha256"),
        ("content", "carries no content"),
    ] {
        let mut damaged = manifest.clone();
        damaged["realms"]["house"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        let message = refusal(World::from_manifest(&damaged));
        assert!(message.contains(expected), "{message}");
    }

    let mut damaged = manifest.clone();
    damaged["realms"]["dialect"]["content"] = json!("moved-dialect");
    let message = refusal(World::from_manifest(&damaged));
    assert!(message.contains("pinned dialect hashes"), "{message}");

    for key in ["house", "dialect"] {
        let mut missing = manifest.clone();
        missing["realms"].as_object_mut().unwrap().remove(key);
        let message = refusal(World::from_manifest(&missing));
        assert!(message.contains(&format!("names a {key}")), "{message}");
        assert!(message.contains("pins none"), "{message}");
    }
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
