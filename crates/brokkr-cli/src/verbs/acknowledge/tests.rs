//! `brokkr operator acknowledge` (#511): refused on a run that folds,
//! answering either quarantine beside an untouched journal, and lapsing
//! when the journal moves.

use std::path::Path;
use std::process::ExitCode;

use brokkr_core::envelope::EventType;
use brokkr_store::{Store, StoreError};
use brokkr_view::Section;
use serde_json::{json, Value};

use super::{moved, quarantine};
use crate::cli_args::OperatorArgs;
use crate::tests::{at, broken_chain_store, poisoned_store, running_store};
use crate::verbs::delivery::{operator, operator_name};
use crate::{failure_line, fleet, now_rfc3339, Exit};

const REASON: &str = "history from an engine since changed";

fn acknowledging(run: &str, db: &Path) -> OperatorArgs {
    OperatorArgs {
        run: run.to_string(),
        command: "acknowledge".to_string(),
        reason: REASON.to_string(),
        findings: Vec::new(),
        by_run: None,
        by_seq: None,
        by_realm: None,
        journal: at(db),
    }
}

fn acknowledge(run: &str, db: &Path) -> anyhow::Result<ExitCode> {
    operator(db.parent().unwrap(), acknowledging(run, db))
}

/// The journal as `brokkr runs --json` derives it: the JSON it prints, and
/// each section's run ids at this moment.
fn listing(db: &Path) -> (Value, Vec<(Section, Vec<String>)>) {
    let listed = fleet::read_journal(db).listed().unwrap();
    let entries: Vec<_> = listed.iter().map(fleet::ListedRun::entry).collect();
    let view = brokkr_view::run_rows(&entries);
    let json = serde_json::to_value(&view).unwrap();
    let sections = brokkr_view::sections(&view.runs, &now_rfc3339())
        .into_iter()
        .map(|(section, rows)| {
            let mut ids: Vec<String> = rows.iter().map(|row| row.run_id.clone()).collect();
            ids.sort();
            (section, ids)
        })
        .collect();
    (json, sections)
}

fn row<'a>(json: &'a Value, run_id: &str) -> &'a Value {
    let rows = json["runs"].as_array().unwrap();
    rows.iter().find(|row| row["run_id"] == run_id).unwrap()
}

fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

#[test]
fn acknowledging_a_run_that_folds_is_refused_and_names_conclude() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "healthy");
    let refused = acknowledge("healthy", &db).unwrap_err();
    assert_eq!(
        failure_line(&refused),
        "error: run healthy folds, so it is not quarantined and there is nothing to \
         acknowledge; nothing was recorded. A run that folds is closed with brokkr conclude \
         --run healthy --reason <why>"
    );
    let store = Store::open_read_only(&db).unwrap();
    assert_eq!(store.acknowledgement("healthy").unwrap(), None);
}

#[test]
fn both_quarantine_kinds_acknowledged_list_under_older_and_runs_json_reports_both() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    poisoned_store(&db, "unfolded");
    broken_chain_store(&db, "unloaded");
    let (before, sections) = listing(&db);
    assert_eq!(
        sections[0],
        (Section::NeedsYou, ids(&["unfolded", "unloaded"]))
    );
    assert_eq!(row(&before, "unfolded")["acknowledgement"], Value::Null);

    for run in ["unfolded", "unloaded"] {
        let done = acknowledge(run, &db).unwrap();
        assert_eq!(done, ExitCode::from(Exit::Completed));
    }
    let (after, sections) = listing(&db);
    assert_eq!(
        sections,
        [
            (Section::NeedsYou, ids(&[])),
            (Section::Running, ids(&[])),
            (Section::Recent, ids(&[])),
            (Section::Older, ids(&["unfolded", "unloaded"])),
        ]
    );
    for (run, quarantine, seq, refusal) in [
        (
            "unfolded",
            "does_not_fold",
            6,
            "event 6: operator/accepted without a matching command",
        ),
        (
            "unloaded",
            "does_not_load",
            3,
            "chain: event 3: previous_hash does not match event 2",
        ),
    ] {
        let listed = row(&after, run);
        assert_eq!(listed["status"], Value::Null, "{run} keeps its ?");
        assert_eq!(listed["verdict"]["standing"], "quarantined");
        assert_eq!(listed["quarantine"], quarantine);
        assert_eq!(listed["detail"], refusal);
        let acknowledgement = &listed["acknowledgement"];
        assert_eq!(
            (
                &acknowledgement["seq"],
                &acknowledgement["operator"],
                &acknowledgement["reason"]
            ),
            (&json!(seq), &json!(operator_name()), &json!(REASON)),
            "{run}"
        );
    }

    // The same head twice is refused in the store's words.
    let again = acknowledge("unfolded", &db).unwrap_err();
    assert_eq!(
        failure_line(&again),
        "error: run 'unfolded' is already acknowledged at event 6; nothing was recorded"
    );
}

#[test]
fn a_moved_journal_lapses_the_acknowledgement_back_to_needs_you() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    poisoned_store(&db, "unfolded");
    acknowledge("unfolded", &db).unwrap();
    let (_, sections) = listing(&db);
    assert_eq!(sections[3], (Section::Older, ids(&["unfolded"])));

    Store::open(&db)
        .unwrap()
        .append_next(
            "unfolded",
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            None,
        )
        .unwrap();
    let (json, sections) = listing(&db);
    assert_eq!(sections[0], (Section::NeedsYou, ids(&["unfolded"])));
    assert_eq!(sections[3], (Section::Older, ids(&[])));
    assert_eq!(row(&json, "unfolded")["acknowledgement"], Value::Null);
}

#[test]
fn export_bytes_are_identical_before_and_after_an_acknowledgement() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    poisoned_store(&db, "unfolded");
    let export = || {
        Store::open_read_only(&db)
            .unwrap()
            .export_ndjson("unfolded")
    };
    let before = export().unwrap();
    acknowledge("unfolded", &db).unwrap();
    assert_eq!(export().unwrap(), before);
}

#[test]
fn supersede_arguments_are_refused_on_acknowledge() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    poisoned_store(&db, "unfolded");
    let cited = OperatorArgs {
        findings: vec![3],
        ..acknowledging("unfolded", &db)
    };
    let refused = operator(dir.path(), cited).unwrap_err();
    assert_eq!(
        failure_line(&refused),
        "error: --findings, --by-run, --by-seq and --by-realm belong to 'supersede'; \
         'acknowledge' takes --run, --reason, --realms and --db"
    );
    let unknown = OperatorArgs {
        command: "forgive".to_string(),
        ..acknowledging("unfolded", &db)
    };
    assert_eq!(
        failure_line(&operator(dir.path(), unknown).unwrap_err()),
        "error: operator command must be 'retry', 'stop', 'supersede' or 'acknowledge'"
    );
    let store = Store::open_read_only(&db).unwrap();
    assert_eq!(store.acknowledgement("unfolded").unwrap(), None);
}

/// A head that moved between the read and the write says what to do; a
/// journal the store refuses keeps the store's words; and contention on
/// the read is no refusal of the journal, so it passes on.
#[test]
fn a_moved_head_says_what_to_do_and_contention_passes_on() {
    let moved_head = moved(
        StoreError::HeadMoved {
            expected_seq: 6,
            found_seq: 7,
        },
        "r1",
    );
    assert_eq!(
        failure_line(&moved_head),
        "error: the journal of r1 moved while it was read; nothing was recorded; look again \
         with brokkr runs: head moved: expected seq 6, found 7"
    );
    let unread = quarantine(Err(StoreError::RunNotFound("r1".to_string())));
    assert_eq!(unread.unwrap().as_deref(), Some("run 'r1' not found"));
    let contended = quarantine(Err(StoreError::Contended {
        operation: "load",
        waited_ms: 30_000,
    }));
    assert!(
        matches!(
            contended,
            Err(StoreError::Contended {
                operation: "load",
                waited_ms: 30_000
            })
        ),
        "{contended:?}"
    );
}
