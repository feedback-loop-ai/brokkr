use super::*;

use serde_json::json;

fn refs<'a>(runs: &'a [(&'a str, &'a str)]) -> Vec<RunRef<'a>> {
    runs.iter()
        .map(|(run_id, created_at)| RunRef { run_id, created_at })
        .collect()
}

#[test]
fn an_exact_id_wins_even_when_it_is_also_another_run_s_prefix() {
    let runs = refs(&[
        ("run-alpha", "2026-08-29T10:00:00Z"),
        ("run-alpha-two", "2026-08-29T11:00:00Z"),
    ]);
    assert_eq!(resolve(&runs, "run-alpha").unwrap(), "run-alpha");
    assert_eq!(resolve(&runs, "run-alpha-two").unwrap(), "run-alpha-two");
}

#[test]
fn a_unique_prefix_resolves_and_an_ambiguous_one_names_its_candidates() {
    let runs = refs(&[
        ("run-alpha-one", "2026-08-29T10:00:00Z"),
        ("run-alpha-two", "2026-08-29T11:00:00Z"),
        ("run-beta", "2026-08-29T12:00:00Z"),
    ]);
    assert_eq!(resolve(&runs, "run-b").unwrap(), "run-beta");

    let ambiguous = resolve(&runs, "run-a").unwrap_err().to_string();
    assert!(ambiguous.contains("matches 2 runs"), "{ambiguous}");
    assert!(ambiguous.contains("run-alpha-one"), "{ambiguous}");
    assert!(ambiguous.contains("run-alpha-two"), "{ambiguous}");
    assert!(!ambiguous.contains("run-beta"), "{ambiguous}");
}

#[test]
fn an_unknown_selector_says_so_and_cannot_smuggle_control_characters() {
    let runs = refs(&[("run-alpha", "2026-08-29T10:00:00Z")]);
    let unknown = resolve(&runs, "run-z\u{1b}[2J").unwrap_err().to_string();
    assert!(unknown.contains("no run matching"), "{unknown}");
    assert!(!unknown.contains('\u{1b}'), "{unknown:?}");
}

#[test]
fn an_empty_selector_is_refused_even_where_it_would_name_the_sole_run() {
    let runs = refs(&[("run-alpha", "2026-08-29T10:00:00Z")]);
    let blank = resolve(&runs, "").unwrap_err();
    assert_eq!(refusal_kind(&blank), Some(Refusal::Blank));
    assert_eq!(
        blank.to_string(),
        "an empty --run names no run; pass an id, a prefix or 'latest'"
    );
}

#[test]
fn latest_is_the_newest_created_at_and_an_empty_database_errors() {
    // Deliberately out of stamp order: "newest" is a property of the
    // runs, not of the order they were handed over in.
    let runs = refs(&[
        ("run-middle", "2026-08-29T11:00:00Z"),
        ("run-newest", "2026-08-29T12:00:00Z"),
        ("run-oldest", "2026-08-29T10:00:00Z"),
    ]);
    assert_eq!(resolve(&runs, LATEST).unwrap(), "run-newest");

    let empty = resolve(&[], LATEST).unwrap_err().to_string();
    assert!(empty.contains("'latest' resolves to nothing"), "{empty}");
}

#[test]
fn the_store_facing_form_resolves_from_the_run_table_without_writing() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    for run_id in ["run-alpha-one", "run-alpha-two", "run-beta"] {
        store
            .create_run(run_id, "feature", "test", &json!({"files": {}}))
            .unwrap();
    }

    assert_eq!(
        resolve_run(&store, "run-alpha-one").unwrap(),
        "run-alpha-one"
    );
    assert_eq!(resolve_run(&store, "run-b").unwrap(), "run-beta");
    assert!(resolve_run(&store, LATEST).is_ok());
    let unknown = resolve_run(&store, "nobody").unwrap_err().to_string();
    assert!(unknown.contains("no run matching"), "{unknown}");

    // Resolution is a read: no run gained a journal event.
    for run_id in ["run-alpha-one", "run-alpha-two", "run-beta"] {
        assert!(store.load(run_id).unwrap().is_empty());
    }
}

/// What a selector settled on, or the kind and text it refused with, so
/// two paths compare whole.
fn outcome(result: Result<String>) -> Result<String, (Option<Refusal>, String)> {
    result.map_err(|error| (refusal_kind(&error), error.to_string()))
}

/// Today's whole-table path: every run through `resolve`.
fn over_list_runs(store: &Store, requested: &str) -> Result<String> {
    let runs = store.list_runs()?;
    let refs: Vec<RunRef<'_>> = runs
        .iter()
        .map(|(run_id, _feature, created_at)| RunRef { run_id, created_at })
        .collect();
    resolve(&refs, requested)
}

/// Every committed fixture journal and the hand-built journal the fleet's
/// page tests plant: for every run id, every proper prefix of it,
/// `latest`, a blank and a missing selector, the key-range path returns
/// exactly what `resolve` over `list_runs` returns — the same id, or the
/// same refusal kind and text.
#[test]
fn the_key_range_path_resolves_every_fixture_and_hand_built_selector_as_the_run_table_did() {
    use crate::fleet::page::tests::{fixture_journal, generated, selector_cases};

    let dir = tempfile::tempdir().unwrap();
    let committed = dir.path().join("fixtures.db");
    fixture_journal(&committed);
    let hand_built = dir.path().join("hand-built.db");
    let _ = Store::open(&hand_built).unwrap();
    generated(&hand_built);

    for db in [&committed, &hand_built] {
        let store = Store::open_read_only(db).unwrap();
        let ids = store
            .list_runs()
            .unwrap()
            .into_iter()
            .map(|(run_id, _feature, _created_at)| run_id);
        for selector in selector_cases(ids) {
            assert_eq!(
                outcome(resolve_run(&store, &selector)),
                outcome(over_list_runs(&store, &selector)),
                "selector {selector:?} in {}",
                db.display()
            );
        }
    }
}

/// The refusals keep their exact words and kinds through the key reads:
/// an empty journal's `latest`, a blank selector, and one nothing matches
/// on an empty and on a populated journal.
#[test]
fn the_key_range_path_keeps_the_refusals_exact() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();

    let empty = resolve_run(&store, LATEST).unwrap_err();
    assert_eq!(refusal_kind(&empty), Some(Refusal::Empty));
    assert_eq!(
        empty.to_string(),
        "no runs in this workspace database; 'latest' resolves to nothing"
    );

    let blank = resolve_run(&store, "").unwrap_err();
    assert_eq!(refusal_kind(&blank), Some(Refusal::Blank));
    assert_eq!(
        blank.to_string(),
        "an empty --run names no run; pass an id, a prefix or 'latest'"
    );

    let missing = resolve_run(&store, "nobody").unwrap_err();
    assert_eq!(refusal_kind(&missing), Some(Refusal::Missing));
    assert_eq!(
        missing.to_string(),
        "no run matching 'nobody' in this workspace database"
    );

    store
        .create_run("run-alpha", "feature", "test", &json!({"files": {}}))
        .unwrap();
    let missing = resolve_run(&store, "run-b").unwrap_err();
    assert_eq!(refusal_kind(&missing), Some(Refusal::Missing));
    assert_eq!(
        missing.to_string(),
        "no run matching 'run-b' in this workspace database"
    );
}
