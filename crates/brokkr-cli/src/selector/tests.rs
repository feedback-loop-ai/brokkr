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
