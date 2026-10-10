use super::*;
use serde_json::json;

const MAP: &str = r#"{
  "schema": "forge.realms/v1",
  "realms": [{"name": "brokkr", "path": ".", "default_branch": "main"}],
  "journal": ".forge/forge.db"
}"#;

fn refusal(text: &str) -> RealmsError {
    match RealmMap::parse("realms.json", text) {
        Ok(_) => panic!("expected {text} to be refused"),
        Err(error) => error,
    }
}

fn with(mutate: impl Fn(&mut Value)) -> RealmsError {
    let mut map: Value = serde_json::from_str(MAP).unwrap();
    mutate(&mut map);
    refusal(&map.to_string())
}

impl RealmsError {
    /// Why `realms.json` is not usable: every map these tests refuse
    /// this way parses.
    fn unusable(self) -> Unusable {
        match self {
            RealmsError::Invalid { path, problem } if path == "realms.json" => problem,
            other => panic!("expected realms.json to be unusable, got {other:?}"),
        }
    }

    /// Serde's own account of why `realms.json` does not parse.
    fn unreadable(self) -> String {
        match self {
            RealmsError::Malformed { path, detail } if path == "realms.json" => detail,
            other => panic!("expected realms.json to be unreadable, got {other:?}"),
        }
    }
}

/// A refusal's realm and the crossing it names, as its variant holds them.
fn crossed_at(realm: &str, crossing: &str) -> (String, String) {
    (realm.to_string(), crossing.to_string())
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
                publishes: CrossingList::Absent,
                consumes: CrossingList::Absent,
                capabilities: None,
                grants: Default::default(),
            }],
            journal: ".forge/forge.db".to_string(),
            provisional_offices: Vec::new(),
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
    // Serde names the field it does not know; the words are serde's own.
    let map = with(|map| map["driver"] = json!("claude")).unreadable();
    assert!(map.starts_with("unknown field `driver`"), "{map}");

    let realm = with(|map| map["realms"][0]["egress"] = json!(["github.com"])).unreadable();
    assert!(realm.starts_with("unknown field `egress`"), "{realm}");
}

#[test]
fn text_that_is_not_json_is_refused_naming_the_file() {
    let refusal = refusal("realms, but not json").to_string();
    assert!(
        refusal.starts_with("realms.json is not a readable"),
        "{refusal}"
    );
}

#[test]
fn a_map_that_calls_itself_another_version_is_refused_by_name() {
    let refusal = with(|map| map["schema"] = json!("forge.realms/v9")).unusable();
    assert_eq!(refusal, Unusable::UnknownSchema("forge.realms/v9".into()));
    let text = refusal.to_string();
    for label in SCHEMAS {
        assert!(text.contains(label), "{label}: {text}");
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
    let refusal = "chroot".parse::<Boundary>().unwrap_err();
    assert_eq!(refusal, BoundaryError("chroot".into()));
    // The one place this refusal's rendered words are pinned.
    assert_eq!(
        refusal.to_string(),
        "'chroot' is not a boundary; the vocabulary is namespace, seatbelt, container, harness \
         and open, and a new boundary is a new decision (decision 0046 ruling 1)"
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
    assert_eq!(
        refusal(&under_v3.to_string()).unusable(),
        Unusable::Unversioned {
            realm: "app".into(),
            word: Word::Boundary,
            schema: SCHEMA_V3.into(),
        }
    );

    let unknown = json!({"schema": SCHEMA_V4, "realms": [realm("app", Some("chroot"))], "journal": "forge.db"});
    assert_eq!(
        refusal(&unknown.to_string()).unusable(),
        Unusable::Boundary {
            realm: Some("app".into()),
            error: BoundaryError("chroot".into()),
        }
    );
    // A shape other than a string is the malformed-map refusal, as for
    // every other field; the word is judged only where one was written.
    let mut numbered = realm("app", None);
    numbered["boundary"] = json!(7);
    let shaped = json!({"schema": SCHEMA_V4, "realms": [numbered], "journal": "forge.db"});
    let malformed = refusal(&shaped.to_string()).unreadable();
    assert!(
        malformed.starts_with("invalid type: integer `7`"),
        "{malformed}"
    );
}

#[test]
fn a_map_with_nothing_in_it_is_refused() {
    let empty = with(|map| map["realms"] = json!([]));
    assert_eq!(empty.unusable(), Unusable::NoRealms);
    let journal = with(|map| map["journal"] = json!("  "));
    assert_eq!(journal.unusable(), Unusable::EmptyJournal);
}

/// A realm name is the key its facts are journaled under, so it is held
/// to the shape a key can have — checked here rather than discovered
/// later inside somebody's evidence.
#[test]
fn a_realm_name_that_could_not_be_read_back_is_refused() {
    for bad in ["", "Brokkr-Realm", "brokkr realm", "-lead"] {
        let refusal = with(|map| map["realms"][0]["name"] = json!(bad));
        assert_eq!(
            refusal.unusable(),
            Unusable::Name {
                index: 0,
                name: bad.into(),
            }
        );
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
    assert_eq!(path.unusable(), Unusable::NoPath("brokkr".into()));
    let branch = with(|map| map["realms"][0]["default_branch"] = json!(" "));
    assert_eq!(branch.unusable(), Unusable::NoBranch("brokkr".into()));
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
    assert_eq!(refusal.unusable(), Unusable::NamedTwice("brokkr".into()));
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
    assert_eq!(
        refusal.unusable(),
        Unusable::Unversioned {
            realm: "brokkr".into(),
            word: Word::Journal,
            schema: SCHEMA_V1.into(),
        }
    );
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
            Err(error) => error.unreadable(),
        }
    };
    // Serde names the field it does not know; the words are serde's own.
    let world = mutate(|map| map["driver"] = json!("claude"));
    assert!(world.starts_with("unknown field `driver`"), "{world}");
    let realm = mutate(|map| map["realms"][0]["egress"] = json!(["github.com"]));
    assert!(realm.starts_with("unknown field `egress`"), "{realm}");
}

/// An empty per-realm journal is the same refusal an empty world journal
/// is: validation applies identically at both levels.
#[test]
fn a_v2_realm_with_an_empty_journal_is_refused() {
    let mut map: Value = serde_json::from_str(MANY).unwrap();
    map["realms"][0]["journal"] = json!("  ");
    assert_eq!(
        RealmMap::of("realms.json", map).unwrap_err().unusable(),
        Unusable::Empty {
            realm: "alpha".into(),
            word: Word::Journal,
        }
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
    for (field, word) in [("house", Word::House), ("dialect", Word::Dialect)] {
        let refusal = with(|map| map["realms"][0][field] = json!("value"));
        assert_eq!(
            refusal.unusable(),
            Unusable::Unversioned {
                realm: "brokkr".into(),
                word,
                schema: SCHEMA_V1.into(),
            }
        );

        let mut v3: Value = serde_json::from_str(MAP).unwrap();
        v3["schema"] = json!(SCHEMA_V3);
        v3["realms"][0][field] = json!("  ");
        assert_eq!(
            RealmMap::of("realms.json", v3).unwrap_err().unusable(),
            Unusable::Empty {
                realm: "brokkr".into(),
                word,
            }
        );
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
        for (field, word) in [("house", Word::House), ("dialect", Word::Dialect)] {
            let mut map = json!({
                "schema": SCHEMA_V3,
                "realms": [{"name": "app", "path": ".", "default_branch": "main"}],
                "journal": "forge.db"
            });
            map["realms"][0][field] = json!(value);
            assert_eq!(
                RealmMap::of("realms.json", map).unwrap_err().unusable(),
                Unusable::Outside {
                    realm: "app".into(),
                    word,
                },
                "{value:?}"
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
        RealmMap::of("realms.json", map).unwrap_err().unusable()
    };
    assert_eq!(
        mutate(|map| map["realms"][1]["name"] = json!("alpha")),
        Unusable::NamedTwice("alpha".into())
    );
    assert_eq!(
        mutate(|map| map["realms"][0]["name"] = json!("Alpha")),
        Unusable::Name {
            index: 0,
            name: "Alpha".into(),
        }
    );
    assert_eq!(
        mutate(|map| map["realms"][0]["path"] = json!(" ")),
        Unusable::NoPath("alpha".into())
    );
    assert_eq!(
        mutate(|map| map["journal"] = json!("")),
        Unusable::EmptyJournal
    );
    assert_eq!(mutate(|map| map["realms"] = json!([])), Unusable::NoRealms);
}

// ------------------------------------------- crossings (decision 0057)

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

fn crossing_refusal(mutate: impl Fn(&mut Value)) -> RealmsError {
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
    assert_eq!(v5.realms[0].publishes, CrossingList::Absent);
    assert_eq!(v5.realms[0].consumes, CrossingList::Absent);
    assert!(!v5.realms[0].publishes.is_written());
    assert!(!v5.realms[0].consumes.is_written());
    assert!(v5.realms[0].published().is_empty());
    assert!(v5.realms[0].consumed().is_empty());
}

/// Ruling 3, the version gate: the two words are refused under every
/// label that predates them, the way a v2 `journal` is refused in a v1
/// map — and the refusal names the version that would admit them.
///
/// Judged for every way the word can be WRITTEN, an empty array and a
/// `null` included: the gate answers for presence, not for content, so
/// a map cannot slip v5 vocabulary under a v4 label by naming nothing
/// with it.
#[test]
fn the_crossing_lists_are_v5_vocabulary_and_older_labels_refuse_them() {
    for label in [SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4] {
        for (field, word) in [("publishes", Word::Publishes), ("consumes", Word::Consumes)] {
            let written = |value: Value| {
                with(|map| {
                    map["schema"] = json!(label);
                    map["realms"][0][field] = value.clone();
                })
            };
            let entry = match word {
                Word::Publishes => json!([{"name": "orders.api", "path": "orders.json"}]),
                _ => json!([{"name": "orders.api", "realm": "alpha", "sha256": pin()}]),
            };
            for value in [entry, json!([]), json!(null)] {
                assert_eq!(
                    written(value.clone()).unusable(),
                    Unusable::Unversioned {
                        realm: "brokkr".into(),
                        word,
                        schema: label.into(),
                    },
                    "{label}/{field}={value}"
                );
            }
        }
    }
}

/// Ruling 3, refusal 7: `null` is not an absence. `realms.v5` types both
/// lists `array`, so a map writing `"publishes": null` is a map its own
/// contract file refuses — and core refuses it too, rather than reading
/// the word as though it had never been written and accepting a map no
/// validator would.
#[test]
fn a_crossing_list_written_as_null_is_refused_rather_than_read_as_absent() {
    for (field, word) in [("publishes", Word::Publishes), ("consumes", Word::Consumes)] {
        let refusal = crossing_refusal(|map| map["realms"][1][field] = json!(null));
        assert_eq!(
            refusal.unusable(),
            Unusable::NullList {
                realm: "beta".into(),
                word,
            }
        );
    }
    // Told apart from absence where it is read, and not by the accessor:
    // a written null names no crossing, exactly as an absent word names
    // none, and only presence separates them.
    let null: CrossingList<PublishedCrossing> = serde_json::from_value(json!(null)).unwrap();
    assert_eq!(null, CrossingList::Null);
    assert!(null.is_written() && null.is_null() && null.entries().is_empty());
    let absent = CrossingList::<ConsumedCrossing>::default();
    assert_eq!(absent, CrossingList::Absent);
    assert!(!absent.is_written() && !absent.is_null() && absent.entries().is_empty());
    let empty: CrossingList<PublishedCrossing> = serde_json::from_value(json!([])).unwrap();
    assert!(empty.is_written() && !empty.is_null() && empty.entries().is_empty());
    assert!(format!("{null:?}").contains("Null"));
}

/// Ruling 1: a published crossing is a name in the realm-name grammar
/// and a file inside the realm that owns it — never an absolute path, a
/// drive letter or a way out of the tree, on exactly the terms `house`
/// and `dialect` are held to.
#[test]
fn a_published_crossing_is_a_named_file_inside_its_own_realm() {
    let bad_name =
        crossing_refusal(|map| map["realms"][0]["publishes"][0]["name"] = json!("Orders"));
    let (realm, crossing) = crossed_at("alpha", "Orders");
    assert_eq!(
        bad_name.unusable(),
        Unusable::CrossingName { realm, crossing }
    );

    let empty = crossing_refusal(|map| map["realms"][0]["publishes"][0]["path"] = json!("  "));
    let (realm, crossing) = crossed_at("alpha", "orders.api");
    assert_eq!(
        empty.unusable(),
        Unusable::NoCrossingPath { realm, crossing }
    );

    for outside in ["/orders.json", "C:orders.json", "../orders.json"] {
        let escape =
            crossing_refusal(|map| map["realms"][0]["publishes"][0]["path"] = json!(outside));
        let (realm, crossing) = crossed_at("alpha", "orders.api");
        assert_eq!(
            escape.unusable(),
            Unusable::CrossingOutside { realm, crossing },
            "{outside}"
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
    let (realm, crossing) = crossed_at("beta", "orders.api");
    assert_eq!(
        refusal.unusable(),
        Unusable::NoPublisher {
            realm,
            crossing,
            publisher: "gamma".into(),
        }
    );
}

/// Ruling 3, refusal 2: the realm is held, and it publishes no such
/// crossing — including the realm that publishes nothing at all.
#[test]
fn a_crossing_its_realm_does_not_publish_is_refused() {
    let unnamed =
        crossing_refusal(|map| map["realms"][1]["consumes"][0]["name"] = json!("invoices.api"));
    let (realm, crossing) = crossed_at("beta", "invoices.api");
    assert_eq!(
        unnamed.unusable(),
        Unusable::Unpublished {
            realm,
            crossing,
            publisher: "alpha".into(),
        }
    );
    let publishes_nothing = crossing_refusal(|map| {
        map["realms"][0]
            .as_object_mut()
            .unwrap()
            .remove("publishes");
    });
    let (realm, crossing) = crossed_at("beta", "orders.api");
    assert_eq!(
        publishes_nothing.unusable(),
        Unusable::Unpublished {
            realm,
            crossing,
            publisher: "alpha".into(),
        }
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
    let (realm, crossing) = crossed_at("alpha", "orders.api");
    assert_eq!(
        published.unusable(),
        Unusable::PublishedTwice { realm, crossing }
    );
    let consumed = crossing_refusal(|map| {
        map["realms"][1]["consumes"] = json!([
            {"name": "orders.api", "realm": "alpha", "sha256": pin()},
            {"name": "orders.api", "realm": "alpha", "sha256": crate::canonical::ZERO_HASH},
        ]);
    });
    let (realm, crossing) = crossed_at("beta", "orders.api");
    assert_eq!(
        consumed.unusable(),
        Unusable::ConsumedTwice { realm, crossing }
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
        let (realm, crossing) = crossed_at("beta", "orders.api");
        assert_eq!(
            refusal.unusable(),
            Unusable::Pin {
                realm,
                crossing,
                pin: bad.into(),
            }
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
    let (realm, crossing) = crossed_at("alpha", "orders.api");
    assert_eq!(
        refusal.unusable(),
        Unusable::ConsumesItself { realm, crossing }
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
        let refusal =
            crossing_refusal(|map| map["realms"][0]["publishes"][0] = mutate.clone()).unreadable();
        assert!(refusal.contains(expected), "{refusal}");
    }
    for (mutate, expected) in [
        (
            json!({"name": "orders.api", "realm": "alpha", "sha256": pin(), "since": "v1"}),
            "since",
        ),
        (json!({"name": "orders.api", "realm": "alpha"}), "sha256"),
    ] {
        let refusal =
            crossing_refusal(|map| map["realms"][1]["consumes"][0] = mutate.clone()).unreadable();
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
    let mutate_with = |mutate: fn(&mut Value)| {
        let mut map = crossed();
        mutate(&mut map);
        RealmMap::of("realms.json", map).unwrap_err()
    };
    let unusable = |mutate: fn(&mut Value)| mutate_with(mutate).unusable();
    assert_eq!(
        unusable(|map| map["realms"][1]["name"] = json!("alpha")),
        Unusable::NamedTwice("alpha".into())
    );
    assert_eq!(
        unusable(|map| map["realms"][0]["name"] = json!("Alpha")),
        Unusable::Name {
            index: 0,
            name: "Alpha".into(),
        }
    );
    assert_eq!(
        unusable(|map| map["realms"][0]["path"] = json!(" ")),
        Unusable::NoPath("alpha".into())
    );
    assert_eq!(
        unusable(|map| map["realms"][1]["default_branch"] = json!("")),
        Unusable::NoBranch("beta".into())
    );
    assert_eq!(
        unusable(|map| map["journal"] = json!("")),
        Unusable::EmptyJournal
    );
    assert_eq!(
        unusable(|map| map["realms"] = json!([])),
        Unusable::NoRealms
    );
    let saga = mutate_with(|map| map["realms"][0]["saga"] = json!("x")).unreadable();
    assert!(saga.starts_with("unknown field `saga`"), "{saga}");
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

fn v6(capabilities: Option<Value>) -> String {
    let mut realm = json!({"name": "private", "path": "repo", "default_branch": "main"});
    if let Some(capabilities) = capabilities {
        realm["capabilities"] = capabilities;
    }
    json!({"schema": SCHEMA_V6, "realms": [realm], "journal": ".forge/forge.db"}).to_string()
}

/// Decision 0065 ruling 3: a v6 realm lists what it grants, and the two
/// optional lists keep absence apart from emptiness because they mean
/// opposite things. Every other key is the dialect's, carried as written.
#[test]
fn a_v6_realm_declares_grants_and_keeps_absent_lists_apart_from_empty_ones() {
    let text = v6(Some(json!({
        "web-fetch": {"dialect": "fetch-native", "tools": ["fetch"],
                      "allow": {"hosts": ["sourceware.org", "yaml.org"]}},
        "web-search": {"dialect": "codex-native-search", "offices": ["researcher"]},
        "library-docs": {"dialect": "docs", "tools": [], "offices": []}
    })));
    let (map, _) = RealmMap::parse("realms.json", &text).unwrap();
    let grants = &map.realms[0].grants;
    assert_eq!(
        grants["web-fetch"],
        CapabilityGrant {
            dialect: "fetch-native".into(),
            tools: Some(vec!["fetch".into()]),
            offices: None,
            retention: GrantRetention::Unreserved,
            restrictions: json!({"allow": {"hosts": ["sourceware.org", "yaml.org"]}})
                .as_object()
                .unwrap()
                .clone(),
        }
    );
    // The grant is pinned exactly as the realm wrote it, array order kept.
    assert_eq!(
        grants["web-fetch"].value(),
        json!({"dialect": "fetch-native", "tools": ["fetch"],
               "allow": {"hosts": ["sourceware.org", "yaml.org"]}})
    );
    assert_eq!(
        grants["web-search"].value(),
        json!({"dialect": "codex-native-search", "offices": ["researcher"]})
    );
    // No scope reaches every office that asks; a named scope reaches
    // only the named; an empty scope reaches none.
    assert!(grants["web-fetch"].reaches("implementer"));
    assert!(grants["web-search"].reaches("researcher"));
    assert!(!grants["web-search"].reaches("implementer"));
    assert!(!grants["library-docs"].reaches("researcher"));
    assert_eq!(grants["library-docs"].tools, Some(Vec::new()));
}

/// Ruling 4: omission and an explicit empty map both grant nothing, and
/// so does every older version — by having no such word at all.
#[test]
fn an_absent_and_an_empty_capabilities_map_both_grant_nothing() {
    for text in [v6(None), v6(Some(json!({})))] {
        let (map, _) = RealmMap::parse("realms.json", &text).unwrap();
        assert!(map.realms[0].grants.is_empty());
    }
    let (map, _) = RealmMap::parse("realms.json", MAP).unwrap();
    assert!(map.realms[0].grants.is_empty());
    assert_eq!(map.realms[0].capabilities, None);
}

/// The word is refused under every label older than its own — written
/// empty, written null or written in full — naming the realm, the word and
/// the version that admits it.
#[test]
fn capabilities_are_refused_under_every_older_version_even_written_empty() {
    for label in &SCHEMAS[..5] {
        for written in [
            json!({}),
            json!(null),
            json!({"web-search": {"dialect": "d"}}),
        ] {
            let mut map: Value = serde_json::from_str(&v6(Some(written))).unwrap();
            map["schema"] = json!(label);
            assert_eq!(
                refusal(&map.to_string()).unusable(),
                Unusable::Unversioned {
                    realm: "private".into(),
                    word: Word::Capabilities,
                    schema: (*label).into(),
                }
            );
        }
    }
}

#[test]
fn a_malformed_grant_is_refused_naming_the_realm_the_capability_and_the_field() {
    let usable = "realms.json is not a usable realms map: ";
    for (written, problem) in [
        (
            json!(null),
            "realm 'private' writes capabilities as null; a capabilities map is an object from \
             capability name to grant, and a realm that grants nothing leaves the word out",
        ),
        (
            json!({"Web Search": {"dialect": "d"}}),
            "realm 'private' grants a capability named 'Web Search'; a capability name is \
             lowercase letters, digits, '.', '_' and '-', starting with a letter or digit",
        ),
        (
            json!({"web-search": "codex-native-search"}),
            "realm 'private' grants capability 'web-search' as \"codex-native-search\"; a grant \
             is an object naming the tool dialect that serves the capability",
        ),
        (
            json!({"web-search": {"tools": ["web_search"]}}),
            "realm 'private' grants capability 'web-search' without a tool dialect name; \
             'dialect' names a file under dialects/tools/ in the realm-name grammar",
        ),
        (
            json!({"web-search": {"dialect": "../escape"}}),
            "realm 'private' grants capability 'web-search' without a tool dialect name; \
             'dialect' names a file under dialects/tools/ in the realm-name grammar",
        ),
        (
            json!({"web-search": {"dialect": "d", "tools": null}}),
            "realm 'private' grants capability 'web-search' with a malformed 'tools'; it is a \
             list of distinct non-empty strings, and leaving it out is how a grant says all",
        ),
        (
            json!({"web-search": {"dialect": "d", "tools": ["a", "a"]}}),
            "realm 'private' grants capability 'web-search' with a malformed 'tools'; it is a \
             list of distinct non-empty strings, and leaving it out is how a grant says all",
        ),
        (
            json!({"web-search": {"dialect": "d", "offices": ["researcher", " "]}}),
            "realm 'private' grants capability 'web-search' with a malformed 'offices'; it is \
             a list of distinct non-empty strings, and leaving it out is how a grant says all",
        ),
        (
            json!({"web-search": {"dialect": "d", "offices": [7]}}),
            "realm 'private' grants capability 'web-search' with a malformed 'offices'; it is \
             a list of distinct non-empty strings, and leaving it out is how a grant says all",
        ),
    ] {
        assert_eq!(
            refusal(&v6(Some(written))).to_string(),
            format!("{usable}{problem}")
        );
    }
}

/// A grant written twice would be granted as whichever copy came second.
/// The rule arrives with v6; an older map keeps the reading it always had.
#[test]
fn a_v6_map_refuses_a_key_written_twice_and_an_older_map_reads_as_it_did() {
    let twice = r#"{"schema":"forge.realms/v6","realms":[{"name":"private","path":"repo","default_branch":"main","capabilities":{"web-search":{"dialect":"a"},"web-search":{"dialect":"b"}}}],"journal":"forge.db"}"#;
    assert_eq!(
        refusal(twice).to_string(),
        "realms.json is not a readable realms map: key 'web-search' is written twice at line 1 \
         column 168"
    );
    let older = r#"{"schema":"forge.realms/v1","realms":[{"name":"a","path":"x","path":".","default_branch":"main"}],"journal":"forge.db"}"#;
    // Last-wins, as it has always been for a map that carries no grant.
    let (map, _) = RealmMap::parse("realms.json", older).unwrap();
    assert_eq!(map.realms[0].path, ".");
}

/// A world read back out of a manifest pin arrives as a value, and its
/// grants are judged exactly as a file's are.
#[test]
fn a_map_embedded_as_a_value_carries_the_same_grants() {
    let content: Value =
        serde_json::from_str(&v6(Some(json!({"web-search": {"dialect": "d"}})))).unwrap();
    let (map, _) = RealmMap::of("pinned", content).unwrap();
    assert_eq!(map.realms[0].grants["web-search"].dialect, "d");
}

/// Proposed decision 0075 ruling 5: the operator's list of offices a
/// provisional model may hold is v7 vocabulary on the world, read back
/// exactly as written, and none at all where the map names none.
#[test]
fn a_v7_map_lists_the_provisional_offices_and_an_absent_list_is_none() {
    let listed = |offices: Value| {
        let mut map: Value = serde_json::from_str(MAP).unwrap();
        map["schema"] = json!(SCHEMA_V7);
        map["provisional_offices"] = offices;
        RealmMap::of("realms.json", map)
    };
    let (map, _) = listed(json!(["researcher", "review-correctness"])).unwrap();
    assert_eq!(
        map.provisional_offices,
        ["researcher", "review-correctness"]
    );
    let (empty, _) = listed(json!([])).unwrap();
    assert_eq!(empty.provisional_offices, [] as [&str; 0]);
    let (unwritten, _) = RealmMap::parse("realms.json", &MAP.replace("/v1", "/v7")).unwrap();
    assert_eq!(unwritten.provisional_offices, [] as [&str; 0]);

    let refused = |offices: Value| listed(offices).unwrap_err();
    assert_eq!(refused(Value::Null), invalid(Unusable::ProvisionalNull));
    assert_eq!(
        refused(json!(["researcher", " "])),
        invalid(Unusable::EmptyOffice(1))
    );
    assert_eq!(
        refused(json!(["researcher", "researcher"])),
        invalid(Unusable::OfficeTwice("researcher".into()))
    );
}

/// The refusal `realms.json` earns for `problem`.
fn invalid(problem: Unusable) -> RealmsError {
    RealmsError::Invalid {
        path: "realms.json".to_string(),
        problem,
    }
}

/// The list is refused under every label older than the one that
/// introduced it, written empty or null as much as in full; and the
/// unknown-label refusal spells out all eight labels this build reads.
#[test]
fn provisional_offices_under_an_older_label_are_refused_by_version() {
    for label in &SCHEMAS[..6] {
        for written in [json!(["researcher"]), json!([]), Value::Null] {
            let mut map: Value = serde_json::from_str(MAP).unwrap();
            map["schema"] = json!(label);
            map["provisional_offices"] = written.clone();
            assert_eq!(
                RealmMap::parse("realms.json", &map.to_string()).unwrap_err(),
                invalid(Unusable::ProvisionalUnversioned((*label).into()))
            );
        }
    }
    let mut map: Value = serde_json::from_str(MAP).unwrap();
    map["schema"] = json!("forge.realms/v0");
    assert_eq!(
        RealmMap::parse("realms.json", &map.to_string()).unwrap_err(),
        invalid(Unusable::UnknownSchema("forge.realms/v0".into()))
    );
}

/// A v7 map that leaves the list out reads exactly as the same map under
/// v6, grants included, and keeps v6's refusal of a key written twice.
#[test]
fn a_v7_map_without_the_list_reads_exactly_as_v6() {
    let v6_text = v6(Some(
        json!({"web-search": {"dialect": "d", "offices": ["researcher"]}}),
    ));
    let (as_v6, _) = RealmMap::parse("realms.json", &v6_text).unwrap();
    let (as_v7, _) = RealmMap::parse("realms.json", &v6_text.replace("/v6", "/v7")).unwrap();
    assert_eq!(as_v7.schema, SCHEMA_V7);
    assert_eq!(
        RealmMap {
            schema: SCHEMA_V6.to_string(),
            ..as_v7
        },
        as_v6
    );
    let journal_twice = MAP
        .replace("/v1", "/v7")
        .replace("\"journal\"", "\"journal\": \"a.db\", \"journal\"");
    assert_eq!(
        refusal(&journal_twice).to_string(),
        "realms.json is not a readable realms map: key 'journal' is written twice at line 5 \
         column 1"
    );
    // The same text under v1 keeps the last-wins reading it always had.
    let (older, _) = RealmMap::parse("realms.json", &journal_twice.replace("/v7", "/v1")).unwrap();
    assert_eq!(older.journal, ".forge/forge.db");
}

/// One refused map, the variant it earns, and that variant's bytes.
type Refused = (RealmsError, Unusable, &'static str);

/// The v1 map relabelled `schema`, then `mutate`d, as refused.
fn labelled(schema: &str, mutate: impl Fn(&mut Value)) -> RealmsError {
    with(|map| {
        map["schema"] = json!(schema);
        mutate(map);
    })
}

/// The map's version, the world's fields and each realm's own words.
fn world_refusals() -> Vec<Refused> {
    let unversioned = |word: Word, schema: &str| Unusable::Unversioned {
        realm: "brokkr".into(),
        word,
        schema: schema.into(),
    };
    let chroot = || BoundaryError("chroot".into());
    vec![
        (
            labelled(SCHEMA_V4, |m| m["realms"][0]["boundary"] = json!("chroot")),
            Unusable::Boundary {
                realm: Some("brokkr".into()),
                error: chroot(),
            },
            "realm 'brokkr' declares boundary 'chroot' is not a boundary; the vocabulary is \
             namespace, seatbelt, container, harness and open, and a new boundary is a new \
             decision (decision 0046 ruling 1)",
        ),
        (
            labelled(SCHEMA_V4, |m| {
                m["realms"][0]["boundary"] = json!("chroot");
                m["realms"][0]["name"] = json!(7);
            }),
            Unusable::Boundary {
                realm: None,
                error: chroot(),
            },
            "realm '?' declares boundary 'chroot' is not a boundary; the vocabulary is \
             namespace, seatbelt, container, harness and open, and a new boundary is a new \
             decision (decision 0046 ruling 1)",
        ),
        (
            with(|m| m["schema"] = json!("forge.realms/v9")),
            Unusable::UnknownSchema("forge.realms/v9".into()),
            "it calls itself 'forge.realms/v9'; this build reads forge.realms/v1, forge.realms/v2, \
             forge.realms/v3, forge.realms/v4, forge.realms/v5, forge.realms/v6, forge.realms/v7 \
             and forge.realms/v8",
        ),
        (
            labelled(SCHEMA_V5, |m| m["realms"][0]["capabilities"] = json!({})),
            unversioned(Word::Capabilities, SCHEMA_V5),
            "realm 'brokkr' names its capabilities, which is forge.realms/v6 vocabulary in a map \
             calling itself forge.realms/v5",
        ),
        (with(|m| m["realms"] = json!([])), Unusable::NoRealms, "it names no realms"),
        (with(|m| m["journal"] = json!(" ")), Unusable::EmptyJournal, "its journal is empty"),
        (
            with(|m| m["realms"][0]["name"] = json!("Brokkr")),
            Unusable::Name {
                index: 0,
                name: "Brokkr".into(),
            },
            "realm 0 is named 'Brokkr'; a realm name is lowercase letters, digits, '.', '_' and \
             '-', starting with a letter or digit",
        ),
        (
            with(|m| m["realms"][0]["path"] = json!("")),
            Unusable::NoPath("brokkr".into()),
            "realm 'brokkr' has no path",
        ),
        (
            with(|m| m["realms"][0]["default_branch"] = json!("")),
            Unusable::NoBranch("brokkr".into()),
            "realm 'brokkr' has no default branch",
        ),
        (
            with(|m| {
                let realm = m["realms"][0].clone();
                m["realms"].as_array_mut().unwrap().push(realm);
            }),
            Unusable::NamedTwice("brokkr".into()),
            "realm 'brokkr' is named twice",
        ),
        (
            with(|m| m["realms"][0]["journal"] = json!("x.db")),
            unversioned(Word::Journal, SCHEMA_V1),
            "realm 'brokkr' names its own journal, which is forge.realms/v2 vocabulary in a map \
             calling itself forge.realms/v1",
        ),
        (
            labelled(SCHEMA_V3, |m| m["realms"][0]["boundary"] = json!("open")),
            unversioned(Word::Boundary, SCHEMA_V3),
            "realm 'brokkr' names its boundary, which is forge.realms/v4 vocabulary in a map \
             calling itself forge.realms/v3",
        ),
    ]
}

/// The realm text words, and what a realm says about its crossings.
fn word_refusals() -> Vec<Refused> {
    let in_v3 = |field: &str, written: &str| {
        let written = json!(written);
        labelled(SCHEMA_V3, move |m| m["realms"][0][field] = written.clone())
    };
    let (realm, word) = ("brokkr".to_string(), Word::House);
    vec![
        (
            labelled(SCHEMA_V2, |m| m["realms"][0]["house"] = json!("H.md")),
            Unusable::Unversioned {
                realm: realm.clone(),
                word,
                schema: SCHEMA_V2.into(),
            },
            "realm 'brokkr' names its house, which is forge.realms/v3 vocabulary in a map \
             calling itself forge.realms/v2",
        ),
        (
            labelled(SCHEMA_V2, |m| m["realms"][0]["journal"] = json!(" ")),
            Unusable::Empty {
                realm: realm.clone(),
                word: Word::Journal,
            },
            "realm 'brokkr' has an empty journal",
        ),
        (
            in_v3("dialect", " "),
            Unusable::Empty {
                realm: realm.clone(),
                word: Word::Dialect,
            },
            "realm 'brokkr' has an empty dialect",
        ),
        (
            in_v3("house", "/h"),
            Unusable::Outside {
                realm: realm.clone(),
                word,
            },
            "realm 'brokkr' has a non-repository-relative house",
        ),
        (
            labelled(SCHEMA_V4, |m| m["realms"][0]["consumes"] = json!([])),
            Unusable::Unversioned {
                realm,
                word: Word::Consumes,
                schema: SCHEMA_V4.into(),
            },
            "realm 'brokkr' names what it consumes, which is forge.realms/v5 vocabulary in a map \
             calling itself forge.realms/v4",
        ),
        (
            crossing_refusal(|m| m["realms"][0]["publishes"] = json!(null)),
            Unusable::NullList {
                realm: "alpha".into(),
                word: Word::Publishes,
            },
            "realm 'alpha' writes publishes as null; a crossing list is an array, and a realm \
             that draws no crossing leaves the word out",
        ),
    ]
}

/// The crossings each realm publishes, and the world's provisional
/// offices.
fn published_refusals() -> Vec<Refused> {
    let publishes = |field: &str, written: Value| {
        crossing_refusal(move |m| m["realms"][0]["publishes"][0][field] = written.clone())
    };
    let (realm, crossing) = crossed_at("alpha", "orders.api");
    let labelled_v7 = |written: Value| {
        labelled(SCHEMA_V7, move |m| {
            m["provisional_offices"] = written.clone()
        })
    };
    vec![
        (
            publishes("name", json!("Orders")),
            Unusable::CrossingName {
                realm: realm.clone(),
                crossing: "Orders".into(),
            },
            "realm 'alpha' publishes a crossing named 'Orders'; a crossing name is lowercase \
             letters, digits, '.', '_' and '-', starting with a letter or digit",
        ),
        (
            publishes("path", json!(" ")),
            Unusable::NoCrossingPath {
                realm: realm.clone(),
                crossing: crossing.clone(),
            },
            "realm 'alpha' publishes crossing 'orders.api' with no path",
        ),
        (
            publishes("path", json!("../o")),
            Unusable::CrossingOutside { realm, crossing },
            "realm 'alpha' publishes crossing 'orders.api' from a non-repository-relative path",
        ),
        (
            crossing_refusal(|m| {
                m["realms"][0]["publishes"] =
                    json!([{"name": "o", "path": "a"}, {"name": "o", "path": "b"}]);
            }),
            Unusable::PublishedTwice {
                realm: "alpha".into(),
                crossing: "o".into(),
            },
            "realm 'alpha' publishes a crossing named 'o' twice",
        ),
        (
            labelled(SCHEMA_V6, |m| m["provisional_offices"] = json!([])),
            Unusable::ProvisionalUnversioned(SCHEMA_V6.into()),
            "it names provisional offices, which is forge.realms/v7 vocabulary in a map calling \
             itself forge.realms/v6",
        ),
        (
            labelled_v7(Value::Null),
            Unusable::ProvisionalNull,
            "it writes provisional_offices as null; the list is an array, and a map that lists no \
             office leaves the word out",
        ),
        (
            labelled_v7(json!(["a", ""])),
            Unusable::EmptyOffice(1),
            "provisional office 1 is empty",
        ),
        (
            labelled_v7(json!(["a", "a"])),
            Unusable::OfficeTwice("a".into()),
            "provisional office 'a' is listed twice",
        ),
    ]
}

/// The crossings each realm consumes, against itself and the world.
fn consumed_refusals() -> Vec<Refused> {
    let consumes = |field: &str, written: &str| {
        let written = json!(written);
        crossing_refusal(move |m| m["realms"][1]["consumes"][0][field] = written.clone())
    };
    let (realm, crossing) = crossed_at("beta", "orders.api");
    let entry = json!({"name": "orders.api", "realm": "alpha", "sha256": pin()});
    vec![
        (
            consumes("sha256", "abc"),
            Unusable::Pin {
                realm: realm.clone(),
                crossing: crossing.clone(),
                pin: "abc".into(),
            },
            "realm 'beta' pins crossing 'orders.api' at 'abc', which is not a sha256: a pin is 64 \
             lowercase hex characters over the published file's raw bytes",
        ),
        (
            consumes("realm", "beta"),
            Unusable::ConsumesItself {
                realm: realm.clone(),
                crossing: crossing.clone(),
            },
            "realm 'beta' consumes crossing 'orders.api' from itself; a crossing is between \
             realms, and a realm reads its own file as a file",
        ),
        (
            crossing_refusal(move |m| m["realms"][1]["consumes"] = json!([entry, entry])),
            Unusable::ConsumedTwice {
                realm: realm.clone(),
                crossing: crossing.clone(),
            },
            "realm 'beta' consumes a crossing named 'orders.api' twice",
        ),
        (
            consumes("realm", "gamma"),
            Unusable::NoPublisher {
                realm: realm.clone(),
                crossing,
                publisher: "gamma".into(),
            },
            "realm 'beta' consumes crossing 'orders.api' from realm 'gamma', which this world \
             does not hold",
        ),
        (
            consumes("name", "other"),
            Unusable::Unpublished {
                realm,
                crossing: "other".into(),
                publisher: "alpha".into(),
            },
            "realm 'beta' consumes crossing 'other', which realm 'alpha' does not publish",
        ),
    ]
}

/// Every way a map that parses is not usable: the map that earns it, the
/// variant it earns, and the one place that variant's bytes are pinned —
/// the text the operator has always read (#353). A grant's refusals are
/// pinned beside the grants they refuse.
#[test]
fn every_unusable_map_reads_as_it_always_has() {
    let refusals = [
        world_refusals(),
        word_refusals(),
        published_refusals(),
        consumed_refusals(),
    ];
    for (refused, expected, text) in refusals.into_iter().flatten() {
        assert_eq!(
            refused.to_string(),
            format!("realms.json is not a usable realms map: {text}")
        );
        assert_eq!(refused.unusable(), expected);
    }
}
