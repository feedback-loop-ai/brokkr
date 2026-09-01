use super::*;
use serde_json::json;

const MAP: &str = r#"{
  "schema": "forge.realms/v1",
  "realms": [{"name": "the-forge", "path": ".", "default_branch": "main"}],
  "journal": ".forge/forge.db"
}"#;

fn refusal(text: &str) -> String {
    match RealmMap::parse("realms.json", text) {
        Ok(_) => panic!("expected {text} to be refused"),
        Err(error) => error.to_string(),
    }
}

fn with(mutate: impl Fn(&mut Value)) -> String {
    let mut map: Value = serde_json::from_str(MAP).unwrap();
    mutate(&mut map);
    refusal(&map.to_string())
}

/// The whole v1 shape, and nothing beside it: the realms with their
/// names, paths and default branches, and the world's journal.
#[test]
fn the_minimal_map_parses_into_the_shape_the_ruling_names() {
    let (map, content) = RealmMap::parse("realms.json", MAP).unwrap();
    assert_eq!(
        map,
        RealmMap {
            schema: SCHEMA_V1.to_string(),
            realms: vec![Realm {
                name: "the-forge".to_string(),
                path: ".".to_string(),
                default_branch: "main".to_string(),
            }],
            journal: ".forge/forge.db".to_string(),
        }
    );
    // The content is returned verbatim, because it is what gets embedded
    // in a run manifest — the digest is over exactly these bytes.
    assert_eq!(content, serde_json::from_str::<Value>(MAP).unwrap());
    // And the parts print for evidence, plainly.
    assert!(format!("{map:?}").contains("the-forge"), "{map:?}");
}

/// Re-indenting a map, or writing its keys in another order, is not a
/// different world: the pin is over canonical JSON.
#[test]
fn the_content_digest_ignores_formatting_but_not_facts() {
    let (_, content) = RealmMap::parse("realms.json", MAP).unwrap();
    let (_, reordered) = RealmMap::parse(
        "realms.json",
        r#"{"journal":".forge/forge.db","realms":[{"default_branch":"main","path":".","name":"the-forge"}],"schema":"forge.realms/v1"}"#,
    )
    .unwrap();
    assert_eq!(
        crate::canonical::sha256_hex(&content),
        crate::canonical::sha256_hex(&reordered)
    );
    let (_, other) = RealmMap::parse("realms.json", &MAP.replace("\"main\"", "\"trunk\"")).unwrap();
    assert_ne!(
        crate::canonical::sha256_hex(&content),
        crate::canonical::sha256_hex(&other)
    );
}

/// The point of the version: a field this build does not know is a
/// REFUSAL, so decision 0021's per-realm constraints must arrive as
/// `forge.realms/v2` rather than as drift inside a v1 file.
#[test]
fn an_unknown_field_is_refused_at_both_levels() {
    let map = with(|map| map["driver"] = json!("claude"));
    assert!(map.contains("not a readable realms map"), "{map}");
    assert!(map.contains("driver"), "{map}");

    let realm = with(|map| map["realms"][0]["egress"] = json!(["github.com"]));
    assert!(realm.contains("egress"), "{realm}");
}

#[test]
fn text_that_is_not_json_is_refused_naming_the_file() {
    let refusal = refusal("realms, but not json");
    assert!(
        refusal.starts_with("realms.json is not a readable"),
        "{refusal}"
    );
}

#[test]
fn a_map_that_calls_itself_another_version_is_refused_by_name() {
    let refusal = with(|map| map["schema"] = json!("forge.realms/v2"));
    assert!(
        refusal.contains("it calls itself 'forge.realms/v2'"),
        "{refusal}"
    );
    assert!(refusal.contains(SCHEMA_V1), "{refusal}");
}

#[test]
fn a_map_with_nothing_in_it_is_refused() {
    let empty = with(|map| map["realms"] = json!([]));
    assert!(empty.contains("names no realms"), "{empty}");
    let journal = with(|map| map["journal"] = json!("  "));
    assert!(journal.contains("journal is empty"), "{journal}");
}

/// A realm name is the key its facts are journaled under, so it is held
/// to the shape a key can have — checked here rather than discovered
/// later inside somebody's evidence.
#[test]
fn a_realm_name_that_could_not_be_read_back_is_refused() {
    for bad in ["", "The-Forge", "the forge", "-lead"] {
        let refusal = with(|map| map["realms"][0]["name"] = json!(bad));
        assert!(refusal.contains("is named"), "{bad}: {refusal}");
        assert!(refusal.contains("realm 0"), "{bad}: {refusal}");
    }
    for good in ["the-forge", "9lives", "a.b_c", "lane2"] {
        let mut map: Value = serde_json::from_str(MAP).unwrap();
        map["realms"][0]["name"] = json!(good);
        RealmMap::parse("realms.json", &map.to_string())
            .unwrap_or_else(|e| panic!("{good} is a name: {e}"));
    }
}

#[test]
fn a_realm_missing_a_path_or_a_branch_is_refused() {
    let path = with(|map| map["realms"][0]["path"] = json!(""));
    assert!(path.contains("realm 'the-forge' has no path"), "{path}");
    let branch = with(|map| map["realms"][0]["default_branch"] = json!(" "));
    assert!(
        branch.contains("realm 'the-forge' has no default branch"),
        "{branch}"
    );
}

/// Two realms under one name would make every per-realm fact ambiguous
/// the moment it was recorded.
#[test]
fn a_name_used_twice_is_refused() {
    let refusal = with(|map| {
        map["realms"] = json!([
            {"name": "the-forge", "path": "a", "default_branch": "main"},
            {"name": "the-forge", "path": "b", "default_branch": "main"},
        ]);
    });
    assert!(refusal.contains("is named twice"), "{refusal}");
}

/// The fold-side law: a journal written before any map recorded one
/// unkeyed head, and it keeps being read exactly as it was.
#[test]
fn a_head_recorded_before_any_map_is_still_the_head() {
    let legacy = json!({LEGACY_REALM_KEY: "abc"});
    assert_eq!(recorded_head(&legacy, None), Some("abc"));
    assert_eq!(recorded_head(&legacy, Some("the-forge")), Some("abc"));
}

#[test]
fn a_head_recorded_under_a_realm_answers_to_that_realm() {
    let keyed = json!({"the-forge": "abc", "other": "def"});
    assert_eq!(recorded_head(&keyed, Some("the-forge")), Some("abc"));
    assert_eq!(recorded_head(&keyed, Some("other")), Some("def"));
    // Several realms and no name to ask by: nothing is guessed.
    assert_eq!(recorded_head(&keyed, None), None);
    assert_eq!(recorded_head(&keyed, Some("elsewhere")), None);
}

/// Two shapes are ruled and two are read: a realm-keyed head answers to
/// its own name and to no other, however few realms are recorded. The
/// lone entry is the tempting case and the wrong one — answering it to
/// any name would compare one realm's HEAD against another realm's tree.
/// Nothing needs the guess: a resumed run rehydrates its world from its
/// own manifest pin, so the reader knows the name to ask by.
#[test]
fn a_realm_keyed_head_answers_to_its_realm_alone() {
    let one = json!({"the-forge": "abc"});
    assert_eq!(recorded_head(&one, Some("the-forge")), Some("abc"));
    assert_eq!(recorded_head(&one, None), None);
    assert_eq!(recorded_head(&one, Some("elsewhere")), None);
}

#[test]
fn a_record_that_is_not_a_head_map_answers_nothing() {
    assert_eq!(recorded_head(&json!("abc"), None), None);
    assert_eq!(recorded_head(&json!({}), None), None);
    assert_eq!(
        recorded_head(&json!({"the-forge": 7}), Some("the-forge")),
        None
    );
    assert_eq!(recorded_head(&json!({"the-forge": 7}), None), None);
}
