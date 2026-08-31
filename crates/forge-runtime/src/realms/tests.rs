use super::*;

const MAP: &str = r#"{
  "schema": "forge.realms/v1",
  "realms": [{"name": "the-forge", "path": "the-forge", "default_branch": "main"}],
  "journal": "state/forge.db"
}"#;

/// A workspace with a map at its root and one realm directory under it.
fn workspace(text: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("the-forge")).unwrap();
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
        dir.path().join("the-forge")
    );
    assert_eq!(
        world.sha256,
        forge_core::canonical::sha256_hex(&world.content)
    );
    assert!(format!("{world:?}").contains("the-forge"), "{world:?}");
}

/// An absolute path in a map is used as written — a world may name a
/// repository that does not live under it.
#[test]
fn absolute_paths_in_a_map_are_left_alone() {
    let dir = workspace(MAP);
    let elsewhere = dir.path().join("elsewhere");
    let text = MAP
        .replace(
            "\"path\": \"the-forge\"",
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
    assert_eq!(found.unwrap().map.realms[0].name, "the-forge");
}

/// The realm a repository IS. A tree the map does not name gets no
/// realm — facts about it are recorded exactly as they were before any
/// map existed, rather than under an invented name.
#[test]
fn a_repository_is_the_realm_whose_path_it_is() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let realm = world.realm_for(&dir.path().join("the-forge")).unwrap();
    assert_eq!(realm.name, "the-forge");
    // The same tree named by a path that needs resolving.
    let indirect = dir.path().join("the-forge/../the-forge");
    assert_eq!(
        world.realm_for(&indirect).map(|r| r.name.as_str()),
        Some("the-forge")
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
            .realm_for(&dir.path().join("the-forge"))
            .map(|realm| realm.name.as_str()),
        Some("the-forge")
    );
}

/// The pin: named, hashed, and embedded whole — so a reader holding only
/// the journal can re-derive the digest from the content beside it.
#[test]
fn a_pinned_world_carries_its_own_answer() {
    let dir = workspace(MAP);
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let manifest = world.pinned(&serde_json::json!({"bundle_name": "b", "files": {}}));
    assert_eq!(manifest["bundle_name"], "b");
    let pin = &manifest["realms"];
    assert_eq!(pin["sha256"], world.sha256);
    assert_eq!(pin["map"], world.content);
    assert!(pin["source"].as_str().unwrap().ends_with("realms.json"));
    assert_eq!(
        forge_core::canonical::sha256_hex(&pin["map"]),
        pin["sha256"].as_str().unwrap()
    );
}
