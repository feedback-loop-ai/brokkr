//! Composition proof (decision 0017). The resolver is a pure function
//! over recipe sources, so every test here is a library of small JSON
//! documents on disk and an assertion about the ONE flat bundle they
//! resolve to — or about the refusal that names the file and the key.
use std::collections::BTreeMap;

use super::compose::{resolve, Ancestor};
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
            json!({"override": {"bundle": ["absent"]}, "absent": 1}),
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

/// Decision 0039 moved `recipes/fast` here as it moved it in the witness
/// file: the table gained `REVIEW-CLEAN-DOCS-FIXES`, and a table that
/// rules differently is a different bundle. Nothing else in this list moved.
/// Decision 0042 moved every pinned bundle at once, and for one reason:
/// every adapter file gained `hands` — how the provider puts its hands in
/// the box, or the measured reason it cannot — and a bundle whose inline
/// gate pins the adapter declaration that authorised it (decision 0021)
/// carries that file's digest in its identity. The bundles hiring the
/// review agents moved further: those agents now chain fable@high →
/// opus@xhigh → sol@xhigh and declare boxed hands, so their resolution
/// records and the manifest's `hands` key changed. Nothing else moved.
/// The five bundles that declare no `extends`. Their digests are pinned
/// to what MAIN produces without composition — three of them moved when
/// those recipes adopted agents (decision 0016), and four moved again
/// when their tables gained the reforging back-edge (decision 0022);
/// each time that is a different bundle and so legitimately a different
/// identity, re-recorded as the identity change it is. All five moved
/// again with decision 0019's rename: the `{forge}` token in bundle
/// argv became `{brokkr}`, and the adapters and charters those bundles
/// resolve moved with it. All five moved a fourth time when decision
/// 0021's compile-time refusals landed: every seat, step and panel
/// member gained the class that says whether it works or judges, and
/// every adapter gained the trust tier and binding grant those refusals
/// read — the law is part of what a bundle IS, so a bundle that now
/// declares it is a different bundle. Every one of them is a different
/// bundle now. Closing decision 0019's rename moved the living role text
/// and policy descriptions once more. The two with INLINE gates — `recipes/fast` and
/// `bundles/verify` — moved once more when those gates began pinning
/// the adapter declaration that authorises them, so a tier demoted in
/// `adapters/` moves the identity of the bundles it was standing behind.
/// Decision 0033 moved the three recipe entries once more by adding the
/// descriptions and relative costs rendered in the contributing guide;
/// the two system bundles did not move.
/// Decision 0035 ruling 5 moved all five: every model pin now carries an
/// effort pin, whether the pin lives in an inline argv (`recipes/fast`,
/// `bundles/verify`) or in the agent library a bundle resolves against
/// (`recipes/panel-review`, `recipes/sdd`, `bundles/self`, whose agents
/// now name the effort they hire beside the model). A hire that gained
/// half its terms is a different hire, and therefore a different bundle.
/// What this proves is that COMPOSITION moves none of them:
/// the recipe library must not shift under recipes that opted into
/// nothing. A move here means composition changed a bundle it was never
/// asked to touch — or the engine version did, which is the other thing
/// a bundle's identity legitimately covers.
const UNCOMPOSED: [(&str, &str); 5] = [
    (
        "recipes/fast",
        "36c369e4cd5e30a87c83702ad937426245dd3d34d53dd3b4c0b2468e8029ded3",
    ),
    (
        "recipes/panel-review",
        "1607d7a82b948a2b71bb146cc88b2ebbacf543e2ae1f2c06db4079e757bf252d",
    ),
    (
        "recipes/sdd",
        "a43ac28ab9cddf115aa15bc1d9d96c83e1c730028cc64881481e98483f4b8f40",
    ),
    (
        "bundles/self",
        "48ca9166e77c5745a7b2b2869d82157cf73c106c41d2453bf45c0fd1645ffcdb",
    ),
    (
        "bundles/verify",
        "e7f7e3db903da3f71dcea248e96f3913359a9deb9b8f055ef306af96782622c2",
    ),
];

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
    for (path, digest) in UNCOMPOSED {
        // Explicit roots: the defaults are relative, and a test's cwd
        // is the crate, not the workspace.
        let bundle = Bundle::compile_with(
            &workspace().join(path),
            &workspace().join("agents"),
            &workspace().join("adapters"),
        )
        .unwrap();
        assert_eq!(bundle.manifest_digest(), digest, "{path} digest moved");
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
    // AC-22: the golden. `recipes/sdd-paranoid` is SDD with a different
    // review panel; its manifest names its own files and, under the
    // reserved prefix, the base it was composed from.
    let compiled = |path: &str| {
        Bundle::compile_with(
            &workspace().join(path),
            &workspace().join("agents"),
            &workspace().join("adapters"),
        )
        .unwrap()
    };
    let bundle = compiled("recipes/sdd-paranoid");
    let sdd = compiled("recipes/sdd");
    // A chain entry pins the base's FILES as a layer. That is not the
    // base's standalone compile digest any more: since agents landed
    // (decision 0016) a standalone compile also folds in its agent
    // resolution, which belongs to the composed bundle rather than to
    // any layer of it.
    let layer_digest = brokkr_core::canonical::sha256_hex(
        &super::manifest_for(&sdd.dir, "sdd", &[], None, None, &BTreeMap::new()).unwrap(),
    );
    assert_eq!(
        bundle.chain,
        vec![Ancestor {
            reached_as: Some("sdd".to_string()),
            name: "sdd".into(),
            dir: sdd.dir.clone(),
            digest: layer_digest,
        }]
    );
    // The same layer digest rides in `files`, so the chain survives the
    // dispatch manifest round-trip.
    assert_eq!(
        bundle.manifest["files"]["@compose/0000/sdd"],
        json!(bundle.chain[0].digest)
    );
    assert_eq!(
        bundle.manifest_digest(),
        "0a6c14473bd7b608ace293cdd8d325a53a03a4007e4ef7fd55c29b7720fad4c9",
        "the composed golden — it moved when the base adopted agents, \
         again when the base's table gained the reforging back-edge \
         (decision 0022), again when the base's argv token was renamed \
         (decision 0019), again when its inline review panel began \
         pinning the adapter declarations that authorise it to judge, \
         (decision 0021), then when the closing 0019 sweep changed the \
         base charters, then when decision 0033 gave both layers \
         contributor-facing description and cost data, and now that \
         decision 0035 ruling 5 makes every model pin carry an effort \
         pin — in this layer's own argv and in the agents its base \
         resolves — which is this test's own principle: changing a \
         base changes the digest of everything derived from it"
    );
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
