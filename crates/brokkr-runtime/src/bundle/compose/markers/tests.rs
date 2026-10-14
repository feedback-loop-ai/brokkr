use std::path::Path;

use serde_json::{json, Value};

use super::{Overriding, Removal, OVERRIDES, OVERRIDE_KINDS, REMOVALS, REMOVE_KINDS};
use crate::bundle::compose::resolve;
use crate::bundle::compose_tests::{base_bundle, base_policy, derived, Library};
use crate::bundle::CompileError;

/// What resolving `leaf` refuses, in full.
fn refused(leaf: &Path) -> String {
    match resolve(leaf) {
        Err(CompileError::Invalid(text)) => text,
        Err(other) => panic!("expected an invalid composition, got {other}"),
        Ok(_) => panic!("expected {} to be refused", leaf.display()),
    }
}

/// A layer's own refusal: its file, then what it says.
fn at(leaf: &Path, says: &str) -> String {
    format!("{}: {says}", leaf.join("bundle.json").display())
}

/// The stale-marker refusal, pinned once.
fn stale(leaf: &Path, marker: &str, name: &str, why: &str) -> String {
    at(
        leaf,
        &format!(
            "'{marker}' names '{name}' but {why}; a marker that describes nothing is a lie \
             about the composition"
        ),
    )
}

/// Every kind has one place in the check table and one in its vocabulary,
/// given by an exhaustive match: a new kind does not compile here until it
/// is placed, and a row or word out of place is named.
#[test]
fn every_marker_kind_has_one_row_in_the_order_its_checks_run() {
    let overriding = |kind: Overriding| match kind {
        Overriding::Cases => (0, 1),
        Overriding::Limits => (1, 2),
        Overriding::Seats => (2, 0),
        Overriding::Bundle => (3, 5),
        Overriding::Table => (4, 4),
        Overriding::Rules => (5, 3),
    };
    let removal = |kind: Removal| match kind {
        Removal::Seats => 0,
        Removal::Rules => 1,
        Removal::Phases => 2,
    };
    assert_eq!((REMOVALS.len(), OVERRIDES.len()), (3, 6));
    for (place, rule) in OVERRIDES.iter().enumerate() {
        assert_eq!(overriding(rule.kind).0, place, "{:?}", rule.kind);
        assert_eq!(rule.marker, format!("override.{}", rule.kind.word()));
    }
    for (place, rule) in REMOVALS.iter().enumerate() {
        assert_eq!(removal(rule.kind), place, "{:?}", rule.kind);
        assert_eq!(rule.marker, format!("remove.{}", rule.kind.word()));
    }
    for (place, kind) in OVERRIDE_KINDS.into_iter().enumerate() {
        assert_eq!(overriding(kind).1, place, "{kind:?}");
    }
    for (place, kind) in REMOVE_KINDS.into_iter().enumerate() {
        assert_eq!(removal(kind), place, "{kind:?}");
    }
    assert_eq!(
        OVERRIDE_KINDS.map(Overriding::word).join(", "),
        "seats, cases, limits, rules, table, bundle"
    );
    assert_eq!(
        REMOVE_KINDS.map(Removal::word).join(", "),
        "seats, rules, phases"
    );
}

/// A leaf over `base` carrying `entries`, each `(key, kind, name, why)`;
/// `why` is the refusal's, not the leaf's.
fn marking(entries: &[(&str, &str, &str, &str)]) -> Value {
    let mut leaf = derived(json!({}));
    for (key, kind, name, _) in entries {
        let listed = &mut leaf[*key][*kind];
        if listed.is_null() {
            *listed = json!([]);
        }
        listed.as_array_mut().unwrap().push(json!(name));
    }
    leaf
}

/// Refuse the first of `entries`, drop it, and go on until none is left,
/// which resolves.
fn peel(library: &Library, mut entries: Vec<(&str, &str, &str, &str)>) {
    while let Some(&(key, kind, name, why)) = entries.first() {
        let leaf = library.recipe("derived", &marking(&entries), None);
        let marker = format!("{key}.{kind}");
        assert_eq!(refused(&leaf), stale(&leaf, &marker, name, why), "{marker}");
        entries.remove(0);
    }
    let leaf = library.recipe("derived", &marking(&entries), None);
    resolve(&leaf).unwrap_or_else(|e| panic!("the control resolves: {e}"));
}

/// Every stale entry is refused, one at a time, in the table's order: the
/// removals, then each override beneath, and each override in this layer,
/// each with its row's reason.
#[test]
fn stale_markers_are_refused_one_at_a_time_in_the_tables_order() {
    let library = Library::new();
    rich_base(&library);
    peel(
        &library,
        vec![
            ("remove", "seats", "ghost", "no ancestor defines it"),
            ("remove", "rules", "GHOST", "no ancestor's table has it"),
            ("remove", "phases", "ghost", "no ancestor's table names it"),
            ("override", "cases", "ghost:case", "no ancestor defines it"),
            ("override", "limits", "ghost", "no ancestor defines it"),
            ("override", "seats", "ghost", "no ancestor defines it"),
            ("override", "bundle", "ghost", "no ancestor sets it"),
            ("override", "table", "ghost", "no ancestor's table sets it"),
            ("override", "rules", "GHOST", "no ancestor's table has it"),
        ],
    );
    let unwritten = "this recipe does not redefine it";
    peel(
        &library,
        vec![
            ("override", "cases", "work:feature", unwritten),
            ("override", "limits", "work", unwritten),
            ("override", "seats", "review", unwritten),
            (
                "override",
                "bundle",
                "description",
                "this recipe does not set it",
            ),
            (
                "override",
                "table",
                "initial",
                "this recipe's table does not set it",
            ),
            (
                "override",
                "rules",
                "REVIEW",
                "this recipe's table does not have it",
            ),
        ],
    );
}

/// A layer is judged in one order: its root, its markers' shapes
/// (`override` before `remove`), its table reference, the removals as each
/// is applied, the overrides in written order, and only then its values.
#[test]
fn a_layer_is_judged_root_markers_table_removals_overrides_then_values() {
    let library = Library::new();
    rich_base(&library);
    let write = |extra: Value| library.recipe("derived", &derived(extra), None);
    let root_first = refused(&write(
        json!({"forced_crew": " ", "remove": {"seats": ["ghost"]}}),
    ));
    assert_eq!(root_first, refused(&write(json!({"forced_crew": " "}))));
    let leaf = write(json!({}));
    let shape = "must be an object of name arrays keyed by member kind";
    let overriding = "seats, cases, limits, rules, table, bundle";
    let seat = json!({"results": ["clean"], "role": "roles/role.md", "driver": {"command": ["x"]}});
    for (extra, expected) in [
        (
            json!({"override": "yes", "remove": "no"}),
            at(&leaf, &format!("'override' {shape} ({overriding})")),
        ),
        (
            json!({"remove": "no"}),
            at(&leaf, &format!("'remove' {shape} (seats, rules, phases)")),
        ),
        (
            json!({"override": {"frobnicate": []}}),
            at(
                &leaf,
                &format!("'override.frobnicate' is not a member kind; known: {overriding}"),
            ),
        ),
        (
            json!({"policy": 3, "remove": {"seats": ["ghost"]}}),
            at(&leaf, "'policy' must be a path to this layer's table"),
        ),
        (
            json!({"remove": {"seats": ["review", "review"]}}),
            stale(&leaf, "remove.seats", "review", "no ancestor defines it"),
        ),
        (
            json!({"remove": {"seats": ["review"]}, "override": {"seats": ["review"]},
                   "seats": {"review": seat}}),
            stale(&leaf, "override.seats", "review", "no ancestor defines it"),
        ),
        (
            json!({"override": {"seats": ["ghost", "review"]}}),
            stale(&leaf, "override.seats", "ghost", "no ancestor defines it"),
        ),
        (
            json!({"override": {"cases": ["work"]}}),
            at(
                &leaf,
                "'override.cases' entry 'work' must be '<seat>:<case>'",
            ),
        ),
        (
            json!({"override": {"seats": ["ghost"]}, "description": "x"}),
            stale(&leaf, "override.seats", "ghost", "no ancestor defines it"),
        ),
    ] {
        assert_eq!(refused(&write(extra.clone())), expected, "{extra}");
    }
}

/// A base every marker kind can name: a description, a `work` seat with
/// limits and named cases, and an `audit` seat and phase to remove.
fn rich_base(library: &Library) {
    let mut bundle = base_bundle();
    bundle["description"] = json!("base");
    bundle["seats"]["work"]["limits"] = json!({"max_attempts": 1});
    bundle["seats"]["work"]["select"] = json!({"cases": {"feature": {}, "chore": {}}});
    bundle["seats"]["audit"] = bundle["seats"]["review"].clone();
    let mut policy = base_policy();
    policy["phases"] = json!(["work", "review", "audit", "done"]);
    library.recipe("base", &bundle, Some(&policy));
}

/// One leaf applies every row once and resolves; the digest of what it
/// resolves to was measured on the hand-unrolled checks this table
/// replaced, so a row that reads or changes anything else moves it.
#[test]
fn every_marker_kind_resolves_to_the_bytes_it_did() {
    let library = Library::new();
    rich_base(&library);
    let seat = |driver: &str| json!({"results": ["clean"], "role": "roles/role.md", "driver": {"command": [driver]}});
    let leaf = derived(json!({
        "description": "derived",
        "policy": "policy.json",
        "remove": {"seats": ["audit"], "rules": ["WORK"], "phases": ["audit"]},
        "override": {"cases": ["work:feature"], "limits": ["work"], "seats": ["review"],
                     "bundle": ["description"], "table": ["initial", "terminal"], "rules": ["REVIEW"]},
        "seats": {
            "work": {"select": {"cases": {"feature": {"driver": "replaced"}}}, "limits": {"max_attempts": 2}},
            "review": seat("review"),
            "extra": seat("extra"),
        },
    }));
    let policy = json!({
        "schema": "forge.phase-machine/v1",
        "phases": ["work", "extra"],
        "initial": "extra",
        "terminal": ["extra"],
        "rules": [{"id": "REVIEW", "from": "review", "result": "clean", "next": "extra", "reason": "own"}],
    });
    let resolved = resolve(&library.recipe("derived", &leaf, Some(&policy)))
        .unwrap_or_else(|e| panic!("every kind applied once resolves: {e}"));
    let frozen: Value = json!({
        "document": resolved.document,
        "table": resolved.table,
        "seat_origin": resolved.seat_origin,
        "case_origin": resolved.case_origin,
    });
    assert_eq!(
        brokkr_core::canonical::sha256_hex(&frozen),
        "748124f81248a075037412b80ce7e46a34a5e0a2b07a8cab3ce1647332d87eec",
        "{frozen:#}"
    );
}
