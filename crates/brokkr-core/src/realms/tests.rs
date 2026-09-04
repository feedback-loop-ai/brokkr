use super::*;
use serde_json::json;

const MAP: &str = r#"{
  "schema": "forge.realms/v1",
  "realms": [{"name": "brokkr", "path": ".", "default_branch": "main"}],
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
                name: "brokkr".to_string(),
                path: ".".to_string(),
                default_branch: "main".to_string(),
                journal: None,
                house: None,
                dialect: None,
            }],
            journal: ".forge/forge.db".to_string(),
        }
    );
    // The content is returned verbatim, because it is what gets embedded
    // in a run manifest — the digest is over exactly these bytes.
    assert_eq!(content, serde_json::from_str::<Value>(MAP).unwrap());
    // And the parts print for evidence, plainly.
    assert!(format!("{map:?}").contains("brokkr"), "{map:?}");
}

/// Re-indenting a map, or writing its keys in another order, is not a
/// different world: the pin is over canonical JSON.
#[test]
fn the_content_digest_ignores_formatting_but_not_facts() {
    let (_, content) = RealmMap::parse("realms.json", MAP).unwrap();
    let (_, reordered) = RealmMap::parse(
        "realms.json",
        r#"{"journal":".forge/forge.db","realms":[{"default_branch":"main","path":".","name":"brokkr"}],"schema":"forge.realms/v1"}"#,
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
    let refusal = with(|map| map["schema"] = json!("forge.realms/v4"));
    assert!(
        refusal.contains("it calls itself 'forge.realms/v4'"),
        "{refusal}"
    );
    assert!(refusal.contains(SCHEMA_V1), "{refusal}");
    assert!(refusal.contains(SCHEMA_V2), "{refusal}");
    assert!(refusal.contains(SCHEMA_V3), "{refusal}");
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
    for bad in ["", "Brokkr-Realm", "brokkr realm", "-lead"] {
        let refusal = with(|map| map["realms"][0]["name"] = json!(bad));
        assert!(refusal.contains("is named"), "{bad}: {refusal}");
        assert!(refusal.contains("realm 0"), "{bad}: {refusal}");
    }
    for good in ["brokkr", "9lives", "a.b_c", "lane2"] {
        let mut map: Value = serde_json::from_str(MAP).unwrap();
        map["realms"][0]["name"] = json!(good);
        RealmMap::parse("realms.json", &map.to_string())
            .unwrap_or_else(|e| panic!("{good} is a name: {e}"));
    }
}

#[test]
fn a_realm_missing_a_path_or_a_branch_is_refused() {
    let path = with(|map| map["realms"][0]["path"] = json!(""));
    assert!(path.contains("realm 'brokkr' has no path"), "{path}");
    let branch = with(|map| map["realms"][0]["default_branch"] = json!(" "));
    assert!(
        branch.contains("realm 'brokkr' has no default branch"),
        "{branch}"
    );
}

/// Two realms under one name would make every per-realm fact ambiguous
/// the moment it was recorded.
#[test]
fn a_name_used_twice_is_refused() {
    let refusal = with(|map| {
        map["realms"] = json!([
            {"name": "brokkr", "path": "a", "default_branch": "main"},
            {"name": "brokkr", "path": "b", "default_branch": "main"},
        ]);
    });
    assert!(refusal.contains("is named twice"), "{refusal}");
}

// ------------------------------------- many hearths (0026 ruling 1)

/// The v2 map, with two realms and two journals, plus a third realm
/// falling back to the world's. Every hearth case in one file.
const MANY: &str = r#"{
  "schema": "forge.realms/v2",
  "realms": [
    {"name": "alpha", "path": "a", "default_branch": "main", "journal": "a/.forge/forge.db"},
    {"name": "beta", "path": "b", "default_branch": "main", "journal": "b/.forge/forge.db"},
    {"name": "gamma", "path": "c", "default_branch": "main"}
  ],
  "journal": ".forge/forge.db"
}"#;

fn many() -> RealmMap {
    RealmMap::parse("realms.json", MANY).unwrap().0
}

/// Ruling 1: one optional field per realm, and the fallback is the world's
/// journal — which is exactly what a v1 realm has always resolved to.
#[test]
fn a_v2_realm_may_carry_its_own_journal_and_falls_back_when_it_does_not() {
    let map = many();
    assert_eq!(map.schema, SCHEMA_V2);
    let journals: Vec<&str> = map.realms.iter().map(|r| map.journal_of(r)).collect();
    assert_eq!(
        journals,
        vec!["a/.forge/forge.db", "b/.forge/forge.db", ".forge/forge.db"]
    );
}

/// The regression bar: a v1 map keeps loading exactly as it does today,
/// and every one of its realms resolves to the one journal it always had.
#[test]
fn a_v1_map_loads_unchanged_and_every_realm_resolves_to_the_worlds_journal() {
    let (map, _) = RealmMap::parse("realms.json", MAP).unwrap();
    assert_eq!(map.schema, SCHEMA_V1);
    assert!(map.realms.iter().all(|realm| realm.journal.is_none()));
    for realm in &map.realms {
        assert_eq!(map.journal_of(realm), ".forge/forge.db");
    }
}

/// A version is a promise about what a file may say. The one new word is
/// refused in a map still calling itself v1, and the refusal names the
/// version that would admit it.
#[test]
fn a_v1_map_naming_a_per_realm_journal_is_refused_by_version() {
    let refusal = with(|map| map["realms"][0]["journal"] = json!("other.db"));
    assert!(refusal.contains("names its own journal"), "{refusal}");
    assert!(refusal.contains(SCHEMA_V2), "{refusal}");
    assert!(refusal.contains(SCHEMA_V1), "{refusal}");
}

/// Closed vocabulary, at both levels, in v2 as in v1 — so decision 0021's
/// per-realm constraints still cannot drift into a file calling itself v2.
#[test]
fn a_v2_map_refuses_unknown_fields_at_both_levels() {
    let mutate = |mutate: fn(&mut Value)| {
        let mut map: Value = serde_json::from_str(MANY).unwrap();
        mutate(&mut map);
        match RealmMap::of("realms.json", map) {
            Ok(_) => panic!("expected a refusal"),
            Err(error) => error.to_string(),
        }
    };
    let world = mutate(|map| map["driver"] = json!("claude"));
    assert!(world.contains("not a readable realms map"), "{world}");
    assert!(world.contains("driver"), "{world}");
    let realm = mutate(|map| map["realms"][0]["egress"] = json!(["github.com"]));
    assert!(realm.contains("egress"), "{realm}");
}

/// An empty per-realm journal is the same refusal an empty world journal
/// is: validation applies identically at both levels.
#[test]
fn a_v2_realm_with_an_empty_journal_is_refused() {
    let mut map: Value = serde_json::from_str(MANY).unwrap();
    map["realms"][0]["journal"] = json!("  ");
    let refusal = RealmMap::of("realms.json", map).unwrap_err().to_string();
    assert!(
        refusal.contains("realm 'alpha' has an empty journal"),
        "{refusal}"
    );
}

#[test]
fn a_v3_map_loads_both_realm_text_declarations_while_v2_stays_unchanged() {
    let v3 = json!({
        "schema": SCHEMA_V3,
        "realms": [{"name": "app", "path": ".", "default_branch": "main",
                    "house": "HOUSE.md", "dialect": "openspec"}],
        "journal": ".forge/forge.db"
    });
    let map = RealmMap::of("realms.json", v3).unwrap().0;
    assert_eq!(map.realms[0].house.as_deref(), Some("HOUSE.md"));
    assert_eq!(map.realms[0].dialect.as_deref(), Some("openspec"));

    let v2 = many();
    assert!(v2
        .realms
        .iter()
        .all(|realm| realm.house.is_none() && realm.dialect.is_none()));
}

#[test]
fn house_and_dialect_are_v3_vocabulary_and_may_not_be_empty() {
    for field in ["house", "dialect"] {
        let refusal = with(|map| map["realms"][0][field] = json!("value"));
        assert!(refusal.contains(SCHEMA_V3), "{field}: {refusal}");

        let mut v3: Value = serde_json::from_str(MAP).unwrap();
        v3["schema"] = json!(SCHEMA_V3);
        v3["realms"][0][field] = json!("  ");
        let refusal = RealmMap::of("realms.json", v3).unwrap_err().to_string();
        assert!(refusal.contains(&format!("empty {field}")), "{refusal}");
    }
}

#[test]
fn realm_text_declarations_cannot_leave_the_repository() {
    for value in [
        "/house.md",
        "\\house.md",
        "C:/house.md",
        "C:\\house.md",
        "../house.md",
        "docs/../house.md",
    ] {
        for field in ["house", "dialect"] {
            let mut map = json!({
                "schema": SCHEMA_V3,
                "realms": [{"name": "app", "path": ".", "default_branch": "main"}],
                "journal": "forge.db"
            });
            map["realms"][0][field] = json!(value);
            let refusal = RealmMap::of("realms.json", map).unwrap_err().to_string();
            assert!(
                refusal.contains(&format!("non-repository-relative {field}")),
                "{value:?}: {refusal}"
            );
        }
    }
}

/// Names, paths and branches are held to the same rules under v2 — the
/// version widened the vocabulary, not the discipline.
#[test]
fn v2_holds_every_v1_rule() {
    let mutate = |mutate: fn(&mut Value)| {
        let mut map: Value = serde_json::from_str(MANY).unwrap();
        mutate(&mut map);
        RealmMap::of("realms.json", map).unwrap_err().to_string()
    };
    assert!(mutate(|map| map["realms"][1]["name"] = json!("alpha")).contains("is named twice"));
    assert!(mutate(|map| map["realms"][0]["name"] = json!("Alpha")).contains("realm 0 is named"));
    assert!(mutate(|map| map["realms"][0]["path"] = json!(" ")).contains("has no path"));
    assert!(mutate(|map| map["journal"] = json!("")).contains("journal is empty"));
    assert!(mutate(|map| map["realms"] = json!([])).contains("names no realms"));
}

/// The fold-side law: a journal written before any map recorded one
/// unkeyed head, and it keeps being read exactly as it was.
#[test]
fn a_head_recorded_before_any_map_is_still_the_head() {
    let legacy = json!({LEGACY_REALM_KEY: "abc"});
    assert_eq!(recorded_head(&legacy, None), Some("abc"));
    assert_eq!(recorded_head(&legacy, Some("brokkr")), Some("abc"));
}

#[test]
fn a_head_recorded_under_a_realm_answers_to_that_realm() {
    let keyed = json!({"brokkr": "abc", "other": "def"});
    assert_eq!(recorded_head(&keyed, Some("brokkr")), Some("abc"));
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
    let one = json!({"brokkr": "abc"});
    assert_eq!(recorded_head(&one, Some("brokkr")), Some("abc"));
    assert_eq!(recorded_head(&one, None), None);
    assert_eq!(recorded_head(&one, Some("elsewhere")), None);
}

#[test]
fn a_record_that_is_not_a_head_map_answers_nothing() {
    assert_eq!(recorded_head(&json!("abc"), None), None);
    assert_eq!(recorded_head(&json!({}), None), None);
    assert_eq!(recorded_head(&json!({"brokkr": 7}), Some("brokkr")), None);
    assert_eq!(recorded_head(&json!({"brokkr": 7}), None), None);
}
