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
                boundary: None,
                publishes: None,
                consumes: None,
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
    let refusal = with(|map| map["schema"] = json!("forge.realms/v6"));
    assert!(
        refusal.contains("it calls itself 'forge.realms/v6'"),
        "{refusal}"
    );
    for label in SCHEMAS {
        assert!(refusal.contains(label), "{label}: {refusal}");
    }
}

/// Decision 0046 ruling 1: the five words parse and display themselves;
/// a sixth is refused with the five and the sentence that a new
/// boundary is a new decision; the record's sentinel is read as an
/// absence and never as a boundary; and the type offers no default — a
/// reader of evidence that carries no word holds `None`.
#[test]
fn the_boundary_vocabulary_is_closed_and_lives_in_one_type() {
    for (boundary, word) in
        BOUNDARIES
            .into_iter()
            .zip(["namespace", "seatbelt", "container", "harness", "open"])
    {
        assert_eq!(boundary.word(), word);
        assert_eq!(boundary.to_string(), word);
        assert_eq!(word.parse::<Boundary>(), Ok(boundary));
        assert_eq!(Boundary::from_recorded(word), Some(Some(boundary)));
        assert_eq!(Boundary::recorded(Some(boundary)), word);
        assert_eq!(
            boundary.is_boxed(),
            matches!(
                boundary,
                Boundary::Namespace | Boundary::Seatbelt | Boundary::Container
            )
        );
    }
    let refusal = "chroot".parse::<Boundary>().unwrap_err().to_string();
    assert!(
        refusal.starts_with("'chroot' is not a boundary"),
        "{refusal}"
    );
    assert!(
        refusal.contains("namespace, seatbelt, container, harness and open"),
        "{refusal}"
    );
    assert!(
        refusal.contains("a new boundary is a new decision (decision 0046 ruling 1)"),
        "{refusal}"
    );
    assert_eq!(Boundary::recorded(None), NOT_APPLICABLE);
    assert_eq!(Boundary::from_recorded(NOT_APPLICABLE), Some(None));
    assert_eq!(Boundary::from_recorded("chroot"), None);
    let absent: Option<Boundary> = serde_json::from_value(json!(null)).unwrap();
    assert_eq!(absent, None);
    assert_eq!(
        serde_json::to_value(Boundary::Harness).unwrap(),
        json!("harness")
    );
}

/// Decision 0046 ruling 1, the map loader: a v4 map declaring the five
/// words loads and each realm reports its word; a v4 realm without the
/// field, and a v3 realm, resolve to `namespace`; the word under a v3
/// label is refused naming the realm, the field and v4; an unknown word
/// in a v4 map is refused naming the realm and the five words.
#[test]
fn a_v4_map_declares_the_boundary_and_older_labels_refuse_it() {
    let realm = |name: &str, boundary: Option<&str>| {
        let mut realm = json!({"name": name, "path": name, "default_branch": "main"});
        if let Some(word) = boundary {
            realm["boundary"] = json!(word);
        }
        realm
    };
    let five: Vec<Value> = BOUNDARIES
        .iter()
        .map(|boundary| realm(boundary.word(), Some(boundary.word())))
        .chain([realm("plain", None)])
        .collect();
    let map = json!({"schema": SCHEMA_V4, "realms": five, "journal": "forge.db"});
    let (map, _) = RealmMap::parse("realms.json", &map.to_string()).unwrap();
    for (realm, boundary) in map.realms.iter().zip(BOUNDARIES) {
        assert_eq!(realm.boundary, Some(boundary));
        assert_eq!(realm.boundary(), boundary);
    }
    let plain = map.realms.last().unwrap();
    assert_eq!(plain.boundary, None);
    assert_eq!(plain.boundary(), Boundary::Namespace);

    let v3 = json!({"schema": SCHEMA_V3, "realms": [realm("app", None)], "journal": "forge.db"});
    let (v3, _) = RealmMap::parse("realms.json", &v3.to_string()).unwrap();
    assert_eq!(v3.realms[0].boundary(), Boundary::Namespace);

    let under_v3 = json!({"schema": SCHEMA_V3, "realms": [realm("app", Some("harness"))], "journal": "forge.db"});
    let held = refusal(&under_v3.to_string());
    assert!(
        held.contains("realm 'app' names its boundary, which is forge.realms/v4 vocabulary"),
        "{held}"
    );
    assert!(held.contains("calling itself forge.realms/v3"), "{held}");

    let unknown = json!({"schema": SCHEMA_V4, "realms": [realm("app", Some("chroot"))], "journal": "forge.db"});
    let sixth = refusal(&unknown.to_string());
    assert!(
        sixth.contains("realm 'app' declares boundary 'chroot' is not a boundary"),
        "{sixth}"
    );
    assert!(
        sixth.contains("namespace, seatbelt, container, harness and open"),
        "{sixth}"
    );
    // A shape other than a string is the malformed-map refusal, as for
    // every other field; the word is judged only where one was written.
    let mut numbered = realm("app", None);
    numbered["boundary"] = json!(7);
    let shaped = json!({"schema": SCHEMA_V4, "realms": [numbered], "journal": "forge.db"});
    let malformed = refusal(&shaped.to_string());
    assert!(malformed.contains("not a readable"), "{malformed}");
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
    assert!(is_repository_relative("h"));
    assert!(is_repository_relative("1:house.md"));
    for value in [
        "/house.md",
        "\\house.md",
        "C:/house.md",
        "C:\\house.md",
        "C:house.md",
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

// ------------------------------------------- crossings (decision 0054)

/// A crossing's pin is over the published file's RAW bytes, never over a
/// canonical form — so the fixture's pin is taken the way a publisher's
/// reader will take it, from bytes that are not JSON at all.
fn pin() -> String {
    crate::canonical::sha256_bytes(b"# orders\n\nThe crossing's own bytes.\n")
}

/// The smallest world that crosses: alpha publishes one file, beta pins
/// it. Two realms, because a crossing is between realms.
fn crossed() -> Value {
    json!({
        "schema": SCHEMA_V5,
        "realms": [
            {"name": "alpha", "path": "alpha", "default_branch": "main",
             "publishes": [{"name": "orders.api", "path": "contracts/orders.v1.schema.json"}]},
            {"name": "beta", "path": "beta", "default_branch": "trunk",
             "consumes": [{"name": "orders.api", "realm": "alpha", "sha256": pin()}]}
        ],
        "journal": "state/world.db"
    })
}

fn crossing_refusal(mutate: impl Fn(&mut Value)) -> String {
    let mut map = crossed();
    mutate(&mut map);
    refusal(&map.to_string())
}

/// Ruling 1 and ruling 2: what a realm publishes is a named file it owns;
/// what a realm consumes is another realm's crossing, pinned by digest.
/// Both lists read back exactly as written, and a realm that draws
/// neither answers with neither.
#[test]
fn a_v5_map_carries_what_each_realm_publishes_and_what_it_pins() {
    let (map, _) = RealmMap::parse("realms.json", &crossed().to_string()).unwrap();
    let published = PublishedCrossing {
        name: "orders.api".to_string(),
        path: "contracts/orders.v1.schema.json".to_string(),
    };
    let consumed = ConsumedCrossing {
        name: "orders.api".to_string(),
        realm: "alpha".to_string(),
        sha256: pin(),
    };
    assert_eq!(map.realms[0].published(), std::slice::from_ref(&published));
    assert_eq!(map.realms[1].consumed(), std::slice::from_ref(&consumed));
    // The publisher consumes nothing and the consumer publishes nothing:
    // absence on either side is an empty list, spelled in one place.
    assert!(map.realms[0].consumed().is_empty());
    assert!(map.realms[1].published().is_empty());
    // Each part of a crossing is part of its identity, and each prints
    // for evidence.
    assert_ne!(
        published,
        PublishedCrossing {
            name: "other".to_string(),
            path: published.path.clone(),
        }
    );
    assert_ne!(
        published,
        PublishedCrossing {
            name: published.name.clone(),
            path: "other".to_string(),
        }
    );
    for other in [
        ConsumedCrossing {
            name: "other".to_string(),
            realm: consumed.realm.clone(),
            sha256: consumed.sha256.clone(),
        },
        ConsumedCrossing {
            name: consumed.name.clone(),
            realm: "other".to_string(),
            sha256: consumed.sha256.clone(),
        },
        ConsumedCrossing {
            name: consumed.name.clone(),
            realm: consumed.realm.clone(),
            sha256: crate::canonical::ZERO_HASH.to_string(),
        },
    ] {
        assert_ne!(consumed, other);
    }
    assert!(format!("{published:?}").contains("orders.api"));
    assert!(format!("{consumed:?}").contains("alpha"));
}

/// A v5 map that draws no crossing is a v4 map: the same realms resolve
/// the same journals, houses, dialects and boundaries, and the two new
/// lists are absent rather than empty. A world that never drew a
/// crossing notices nothing.
#[test]
fn a_v5_map_without_crossings_reads_exactly_as_a_v4_map() {
    let realms = json!([{
        "name": "app", "path": ".", "default_branch": "main",
        "journal": "app.db", "house": "HOUSE.md", "dialect": "openspec",
        "boundary": "harness"
    }]);
    let v4 = json!({"schema": SCHEMA_V4, "realms": realms, "journal": "world.db"});
    let v5 = json!({"schema": SCHEMA_V5, "realms": realms, "journal": "world.db"});
    let (v4, _) = RealmMap::of("realms.json", v4).unwrap();
    let (v5, _) = RealmMap::of("realms.json", v5).unwrap();
    assert_eq!(v4.realms, v5.realms);
    assert_eq!(v5.journal_of(&v5.realms[0]), "app.db");
    assert_eq!(v5.realms[0].boundary(), Boundary::Harness);
    assert_eq!(v5.realms[0].house.as_deref(), Some("HOUSE.md"));
    assert_eq!(v5.realms[0].dialect.as_deref(), Some("openspec"));
    assert_eq!(v5.realms[0].publishes, None);
    assert_eq!(v5.realms[0].consumes, None);
    assert!(v5.realms[0].published().is_empty());
    assert!(v5.realms[0].consumed().is_empty());
}

/// Ruling 3, the version gate: the two words are refused under every
/// label that predates them, the way a v2 `journal` is refused in a v1
/// map — and the refusal names the version that would admit them.
#[test]
fn the_crossing_lists_are_v5_vocabulary_and_older_labels_refuse_them() {
    for label in [SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4] {
        for field in ["publishes", "consumes"] {
            let refusal = with(|map| {
                map["schema"] = json!(label);
                map["realms"][0][field] = match field {
                    "publishes" => json!([{"name": "orders.api", "path": "orders.json"}]),
                    _ => json!([{"name": "orders.api", "realm": "alpha", "sha256": pin()}]),
                };
            });
            assert!(
                refusal.contains(&format!("realm 'brokkr' names what it {field}")),
                "{label}/{field}: {refusal}"
            );
            assert!(refusal.contains(SCHEMA_V5), "{label}/{field}: {refusal}");
            assert!(
                refusal.contains(&format!("calling itself {label}")),
                "{label}/{field}: {refusal}"
            );
        }
    }
}

/// Ruling 1: a published crossing is a name in the realm-name grammar
/// and a file inside the realm that owns it — never an absolute path, a
/// drive letter or a way out of the tree, on exactly the terms `house`
/// and `dialect` are held to.
#[test]
fn a_published_crossing_is_a_named_file_inside_its_own_realm() {
    let bad_name =
        crossing_refusal(|map| map["realms"][0]["publishes"][0]["name"] = json!("Orders"));
    assert!(
        bad_name.contains("realm 'alpha' publishes a crossing named 'Orders'"),
        "{bad_name}"
    );
    assert!(bad_name.contains("lowercase letters"), "{bad_name}");

    let empty = crossing_refusal(|map| map["realms"][0]["publishes"][0]["path"] = json!("  "));
    assert!(
        empty.contains("realm 'alpha' publishes crossing 'orders.api' with no path"),
        "{empty}"
    );

    for outside in ["/orders.json", "C:orders.json", "../orders.json"] {
        let escape =
            crossing_refusal(|map| map["realms"][0]["publishes"][0]["path"] = json!(outside));
        assert!(
            escape.contains(
                "realm 'alpha' publishes crossing 'orders.api' from a non-repository-relative path"
            ),
            "{outside}: {escape}"
        );
    }
}

/// Ruling 3, refusal 1: a crossing is consumed from a realm this world
/// holds. A pin on a realm the map does not name is a dependency on
/// nothing, and it is refused where it is written rather than discovered
/// when somebody goes looking for the file.
#[test]
fn a_crossing_consumed_from_a_realm_the_world_does_not_hold_is_refused() {
    let refusal = crossing_refusal(|map| map["realms"][1]["consumes"][0]["realm"] = json!("gamma"));
    assert!(
        refusal.contains(
            "realm 'beta' consumes crossing 'orders.api' from realm 'gamma', \
             which this world does not hold"
        ),
        "{refusal}"
    );
}

/// Ruling 3, refusal 2: the realm is held, and it publishes no such
/// crossing — including the realm that publishes nothing at all.
#[test]
fn a_crossing_its_realm_does_not_publish_is_refused() {
    let unnamed =
        crossing_refusal(|map| map["realms"][1]["consumes"][0]["name"] = json!("invoices.api"));
    assert!(
        unnamed.contains(
            "realm 'beta' consumes crossing 'invoices.api', which realm 'alpha' does not publish"
        ),
        "{unnamed}"
    );
    let publishes_nothing = crossing_refusal(|map| {
        map["realms"][0]
            .as_object_mut()
            .unwrap()
            .remove("publishes");
    });
    assert!(
        publishes_nothing.contains("which realm 'alpha' does not publish"),
        "{publishes_nothing}"
    );
}

/// Ruling 3, refusal 3: within one realm, a crossing name means one
/// thing. Two published files under one name, or two pins under one
/// name, would make every later reference ambiguous the moment it was
/// written.
#[test]
fn a_crossing_name_is_used_once_in_each_list() {
    let published = crossing_refusal(|map| {
        map["realms"][0]["publishes"] = json!([
            {"name": "orders.api", "path": "contracts/orders.v1.schema.json"},
            {"name": "orders.api", "path": "contracts/orders.v2.schema.json"},
        ]);
    });
    assert!(
        published.contains("realm 'alpha' publishes a crossing named 'orders.api' twice"),
        "{published}"
    );
    let consumed = crossing_refusal(|map| {
        map["realms"][1]["consumes"] = json!([
            {"name": "orders.api", "realm": "alpha", "sha256": pin()},
            {"name": "orders.api", "realm": "alpha", "sha256": crate::canonical::ZERO_HASH},
        ]);
    });
    assert!(
        consumed.contains("realm 'beta' consumes a crossing named 'orders.api' twice"),
        "{consumed}"
    );
}

/// Ruling 2, refusal 4: a pin is a sha256 — 64 lowercase hex characters
/// — judged here as a shape and nowhere as bytes, because this crate
/// reads no file. One spelling of "malformed", shared with every other
/// digest this build judges.
#[test]
fn a_pin_that_is_not_a_sha256_is_refused() {
    for bad in [
        "",
        "not-a-digest",
        &pin()[..63],
        &pin().to_uppercase(),
        &format!("{}0", pin()),
    ] {
        let refusal =
            crossing_refusal(|map| map["realms"][1]["consumes"][0]["sha256"] = json!(bad));
        assert!(
            refusal.contains("realm 'beta' pins crossing 'orders.api' at"),
            "{bad}: {refusal}"
        );
        assert!(
            refusal.contains("64 lowercase hex characters over the published file's raw bytes"),
            "{bad}: {refusal}"
        );
    }
    assert!(crate::canonical::is_sha256_hex(&pin()));
}

/// Ruling 3, refusal 6: a crossing is between realms. A realm that
/// pinned its own published file would be pinning a file it can simply
/// read, and a digest that moves whenever its own tree does — a
/// dependency on itself, recorded as though it were a contract.
#[test]
fn a_realm_does_not_consume_its_own_crossing() {
    let refusal = crossing_refusal(|map| {
        map["realms"][0]["consumes"] =
            json!([{"name": "orders.api", "realm": "alpha", "sha256": pin()}]);
    });
    assert!(
        refusal.contains("realm 'alpha' consumes crossing 'orders.api' from itself"),
        "{refusal}"
    );
    assert!(
        refusal.contains("a crossing is between realms"),
        "{refusal}"
    );
}

/// The vocabulary stays closed inside the new entries too: a field this
/// build does not know is refused there exactly as it is at the map and
/// realm levels, so a crossing's content type, its description or its
/// fetch configuration must arrive as a version rather than as drift in
/// a file still calling itself v5.
#[test]
fn a_crossing_entry_refuses_unknown_and_missing_fields() {
    for (mutate, expected) in [
        (
            json!({"name": "orders.api", "path": "orders.json", "media_type": "json"}),
            "media_type",
        ),
        (json!({"name": "orders.api"}), "path"),
    ] {
        let refusal = crossing_refusal(|map| map["realms"][0]["publishes"][0] = mutate.clone());
        assert!(refusal.contains("not a readable realms map"), "{refusal}");
        assert!(refusal.contains(expected), "{refusal}");
    }
    for (mutate, expected) in [
        (
            json!({"name": "orders.api", "realm": "alpha", "sha256": pin(), "since": "v1"}),
            "since",
        ),
        (json!({"name": "orders.api", "realm": "alpha"}), "sha256"),
    ] {
        let refusal = crossing_refusal(|map| map["realms"][1]["consumes"][0] = mutate.clone());
        assert!(refusal.contains("not a readable realms map"), "{refusal}");
        assert!(refusal.contains(expected), "{refusal}");
    }
}

/// The regression bar for the whole version line: every older map keeps
/// loading exactly as it did, and every rule v5 holds its realms to is a
/// rule its predecessors were already held to.
#[test]
fn every_earlier_map_still_loads_and_v5_holds_every_earlier_rule() {
    for text in [MAP, MANY] {
        RealmMap::parse("realms.json", text).unwrap();
    }
    assert!(older_than(SCHEMA_V1, SCHEMA_V5));
    assert!(!older_than(SCHEMA_V5, SCHEMA_V5));
    assert!(!older_than(SCHEMA_V5, SCHEMA_V1));
    let mutate = |mutate: fn(&mut Value)| {
        let mut map = crossed();
        mutate(&mut map);
        RealmMap::of("realms.json", map).unwrap_err().to_string()
    };
    assert!(mutate(|map| map["realms"][1]["name"] = json!("alpha")).contains("is named twice"));
    assert!(mutate(|map| map["realms"][0]["name"] = json!("Alpha")).contains("realm 0 is named"));
    assert!(mutate(|map| map["realms"][0]["path"] = json!(" ")).contains("has no path"));
    assert!(mutate(|map| map["realms"][1]["default_branch"] = json!("")).contains("no default"));
    assert!(mutate(|map| map["journal"] = json!("")).contains("journal is empty"));
    assert!(mutate(|map| map["realms"] = json!([])).contains("names no realms"));
    assert!(mutate(|map| map["realms"][0]["saga"] = json!("x")).contains("saga"));
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

#[test]
fn the_record_serde_helper_round_trips_words_and_the_sentinel_without_defaulting() {
    #[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct Record {
        #[serde(with = "super::recorded_boundary")]
        boundary: Option<Boundary>,
    }
    for boundary in BOUNDARIES.into_iter().map(Some).chain([None]) {
        let record = Record { boundary };
        let value = serde_json::to_value(&record).unwrap();
        assert_eq!(value, json!({"boundary": Boundary::recorded(boundary)}));
        assert_eq!(serde_json::from_value::<Record>(value).unwrap(), record);
    }
    for value in [
        json!({}),
        json!({"boundary": null}),
        json!({"boundary": "chroot"}),
    ] {
        assert!(serde_json::from_value::<Record>(value).is_err());
    }
    assert!(serde_json::from_value::<Boundary>(json!("not applicable")).is_err());
}
