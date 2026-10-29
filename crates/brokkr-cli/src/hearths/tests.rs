use std::path::{Path, PathBuf};

use brokkr_runtime::realms::Hearth;
use brokkr_store::test_support::plant_run;
use brokkr_store::Store;
use rusqlite::params;

use super::{ask, Answers};
use crate::fleet::page::tests::selector_cases;

/// Plant `runs` — id and recorded stamp — into a journal, in the order
/// given, so `list_runs`' tie order is that insertion order.
fn journal(dir: &Path, name: &str, runs: &[(&str, &str)]) -> PathBuf {
    let db = dir.join(name);
    let _ = Store::open(&db).unwrap();
    let conn = rusqlite::Connection::open(&db).unwrap();
    for (run_id, at) in runs {
        plant_run(&conn, run_id, "{}").unwrap();
        conn.execute(
            "UPDATE runs SET created_at = ?1 WHERE run_id = ?2",
            params![at, run_id],
        )
        .unwrap();
    }
    db
}

fn hearth(realm: &str, journal: &Path) -> Hearth {
    Hearth {
        realms: vec![realm.to_string()],
        journal: journal.to_path_buf(),
    }
}

/// Today's whole-table answer, through the same `record` the new path
/// uses: each hearth's `list_runs` handed over as it always was.
fn over_list_runs(hearths: &[Hearth], run: &str) -> Answers {
    let mut answers = Answers {
        refusal: None,
        ambiguous: None,
        answered: Vec::new(),
    };
    for (index, hearth) in hearths.iter().enumerate() {
        if !hearth.journal.is_file() {
            continue;
        }
        let store = Store::open_read_only(&hearth.journal).unwrap();
        let runs: Vec<(String, String)> = store
            .list_runs()
            .unwrap()
            .into_iter()
            .map(|(run_id, _feature, created_at)| (run_id, created_at))
            .collect();
        answers.record(index, &runs, run);
    }
    answers
}

fn said(error: &Option<anyhow::Error>) -> Option<String> {
    error.as_ref().map(ToString::to_string)
}

/// The many-hearth `ask` over two journals: for every run in either, its
/// exact id and every proper prefix, `latest`, a blank and a missing
/// selector, it records exactly what the whole-table answer did — the
/// same hearths and ids, or the same refusal kind and text.
#[test]
fn the_key_range_ask_answers_two_hearths_as_the_run_tables_did() {
    let dir = tempfile::tempdir().unwrap();
    let alpha = journal(
        dir.path(),
        "alpha.db",
        &[
            ("run-alpha", "2026-01-01T00:00:03Z"),
            ("run-alpha-one", "2026-01-01T00:00:01Z"),
            ("run-alpha-two", "2026-01-01T00:00:02Z"),
            ("target-alone", "2026-01-01T00:00:04Z"),
        ],
    );
    let beta = journal(
        dir.path(),
        "beta.db",
        &[
            ("run-beta", "2026-01-01T00:00:05Z"),
            ("run-shared-b", "2026-01-01T00:00:06Z"),
            ("target-one", "2026-01-01T00:00:07Z"),
            ("target-two", "2026-01-01T00:00:08Z"),
        ],
    );
    let world = [hearth("alpha", &alpha), hearth("beta", &beta)];

    let mut ids = Vec::new();
    for hearth in &world {
        let store = Store::open_read_only(&hearth.journal).unwrap();
        ids.extend(
            store
                .list_runs()
                .unwrap()
                .into_iter()
                .map(|(run_id, _feature, _created_at)| run_id),
        );
    }

    for selector in selector_cases(ids) {
        let asked = ask(&world, &selector);
        let whole = over_list_runs(&world, &selector);
        assert_eq!(
            asked.answered, whole.answered,
            "the answering hearths and ids for {selector:?}"
        );
        assert_eq!(
            said(&asked.refusal),
            said(&whole.refusal),
            "the refusal for {selector:?}"
        );
        assert_eq!(
            said(&asked.ambiguous),
            said(&whole.ambiguous),
            "the ambiguous refusal for {selector:?}"
        );
    }
}
