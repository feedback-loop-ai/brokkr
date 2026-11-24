//! Exercise the parsed verbs against a separate scratch journal per test.

use std::io;
use std::path::PathBuf;

use brokkr_store::{Attribution, Store};
use clap::Parser;
use rusqlite::Connection;
use serde_json::{json, Value};

use super::*;
use crate::{report_to, Cli, Cmd, GroupedCmd};

const DRAFT: &str = "draft.md";
const COMPLETE: &str = "complete.md";
const REASON: &str = "the operator's reason";

struct Scratch {
    dir: tempfile::TempDir,
    db: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        crate::folder::folder_tests::plant_library(dir.path());
        let db = dir.path().join("orders.db");
        Self { dir, db }
    }

    fn sql(&self) -> Connection {
        Connection::open(&self.db).unwrap()
    }

    fn file(&self, text: &str) -> PathBuf {
        let file = self.dir.path().join("order.md");
        std::fs::write(&file, text).unwrap();
        file
    }

    fn parsed(&self, words: &[&str]) -> Cli {
        let tail = ["--db", self.db.to_str().unwrap()];
        let argv = ["brokkr", "orders"].iter().chain(words).chain(&tail);
        Cli::try_parse_from(argv).unwrap()
    }

    fn invoke(&self, words: &[&str]) -> (Result<ExitCode>, String) {
        let Cmd::Grouped(GroupedCmd::Orders(command)) = self.parsed(words).command else {
            unreachable!("parsed as an orders command");
        };
        let mut out = Vec::new();
        let exit = orders(self.dir.path(), command, &mut out);
        (exit, String::from_utf8(out).unwrap())
    }

    fn stdout(&self, words: &[&str]) -> String {
        let (exit, out) = self.invoke(words);
        assert_eq!(exit.unwrap(), ExitCode::from(Exit::Completed));
        out
    }

    fn refused(&self, words: &[&str]) -> (anyhow::Error, String) {
        let (exit, out) = self.invoke(words);
        assert_eq!(out, "");
        let error = exit.unwrap_err();
        let mut stderr = Vec::new();
        assert_eq!(report_to(&error, &mut stderr), ExitCode::from(Exit::Failed));
        (error, String::from_utf8(stderr).unwrap())
    }

    fn add(&self, file: &Path) -> String {
        self.stdout(&["add", file.to_str().unwrap(), "--reason", REASON])
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/work-order")
        .join(name)
}

fn document(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).unwrap()
}

#[test]
fn add_assigns_an_id_lists_it_and_refuses_a_second_proposal() {
    let scratch = Scratch::new();
    assert_eq!(scratch.add(&fixture(DRAFT)), "proposed WO-1\n");
    assert_eq!(
        scratch.stdout(&["list"]),
        "WO-1  proposed  0  Work order draft\n"
    );
    let recorded = scratch.stdout(&["show", "WO-1"]);
    let duplicate = scratch.file(&recorded);
    let (error, stderr) =
        scratch.refused(&["add", duplicate.to_str().unwrap(), "--reason", REASON]);
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Orders(OrderRefusal::Exists(id))) if id.number() == 1
    ));
    assert_eq!(
        stderr,
        "error: WO-1 was already proposed; edit it to change its text\n"
    );
}

#[test]
fn list_prints_a_title_without_controls_or_directional_overrides() {
    let scratch = Scratch::new();
    let text = document(DRAFT).replace(
        "\"Work order draft\"",
        "\"Work\\u001b order\\u202e draft\\u009b\"",
    );
    scratch.add(&scratch.file(&text));

    assert_eq!(
        scratch.stdout(&["list"]),
        "WO-1  proposed  0  Work order draft\n"
    );
}

#[test]
fn add_explains_that_a_ruled_header_is_only_proposed() {
    let scratch = Scratch::new();
    assert_eq!(
        scratch.add(&fixture(COMPLETE)),
        "proposed WO-12\nthe journal holds it as proposed until `brokkr orders rule`\n"
    );
    let expected = WorkOrder::parse(&document(COMPLETE), &shipped_trackers())
        .unwrap()
        .with_state(WorkOrderState::Proposed)
        .render();
    assert_eq!(scratch.stdout(&["show", "WO-12"]), expected);
}

#[test]
fn ruling_waits_for_closed_asks_and_an_edit_returns_the_order_to_proposed() {
    let scratch = Scratch::new();
    let complete = document(COMPLETE);
    let open = complete.replacen(",\n      \"ruling\": 1", "", 1);
    let file = scratch.file(&open);
    scratch.add(&file);
    let rule = ["rule", "WO-12", "--reason", REASON];
    let (error, stderr) = scratch.refused(&rule);
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Orders(OrderRefusal::OpenAsk(id))) if id.number() == 12
    ));
    assert_eq!(
        stderr,
        "error: WO-12 has an open ask; close its asks before ruling it\n"
    );

    let edit = ["edit", "WO-12", file.to_str().unwrap(), "--reason", REASON];
    let expected_edit = "edited WO-12, proposed until ruled again\n";
    assert_eq!(scratch.file(&complete), file);
    assert_eq!(scratch.stdout(&edit), expected_edit);
    assert_eq!(scratch.stdout(&rule), "ruled WO-12\n");
    assert_eq!(scratch.stdout(&["show", "WO-12"]), complete);
    assert_eq!(scratch.stdout(&edit), expected_edit);
    assert_eq!(
        scratch.stdout(&["show", "WO-12"]),
        complete.replace("\"state\": \"ruled\"", "\"state\": \"proposed\"")
    );

    let connection = scratch.sql();
    let mut statement = connection
        .prepare("SELECT event, operator, reason FROM work_order_events ORDER BY seq")
        .unwrap();
    let rows: Vec<(String, String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        rows,
        ["proposed", "edited", "ruled", "edited"].map(|event| (
            event.to_owned(),
            super::super::delivery::operator_name(),
            REASON.to_owned()
        ))
    );
}

#[test]
fn ruling_an_order_the_cli_cannot_show_journals_no_ruling() {
    let scratch = Scratch::new();
    let extra = r#"{"tracker":"custom","hosts":["github.com"]}"#;
    let extended = Trackers::from_definitions(SHIPPED.iter().copied().chain([extra])).unwrap();
    let document = document(COMPLETE).replace("\"tracker\": \"github\"", "\"tracker\": \"custom\"");
    let order = WorkOrder::parse(&document, &extended).unwrap();
    let by = Attribution {
        operator: "operator",
        reason: "propose from another tracker set",
    };
    Store::open(&scratch.db)
        .unwrap()
        .order_add(order, by)
        .unwrap();

    let rule = ["rule", "WO-12", "--reason", REASON];
    let (error, stderr) = scratch.refused(&rule);
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Orders(OrderRefusal::Unreadable(id, _))) if id.number() == 12
    ));
    assert!(stderr.starts_with(
        "error: the journal's text for WO-12 cannot be read: the work order cites unknown tracker 'custom'"
    ));
    let ruled: u32 = scratch
        .sql()
        .query_row(
            "SELECT COUNT(*) FROM work_order_events WHERE event = 'ruled'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(ruled, 0);
}

#[test]
fn show_prints_each_line_without_controls_or_directional_overrides() {
    let scratch = Scratch::new();
    let draft = document(DRAFT);
    let text = draft.replace(
        "Keep the work clear.",
        "Keep the \x1bwork\u{202e} clear.\n<!-- The review still reads this line.",
    );
    scratch.add(&scratch.file(&text));
    let expected = WorkOrder::parse(
        &draft.replace(
            "Keep the work clear.",
            "Keep the work clear.\n<!-- The review still reads this line.",
        ),
        &shipped_trackers(),
    )
    .unwrap()
    .with_id("WO-1".parse().unwrap())
    .render();
    assert_eq!(scratch.stdout(&["show", "WO-1"]), expected);
}

#[test]
fn a_parser_refusal_prints_the_tracker_word_without_controls() {
    let scratch = Scratch::new();
    let text = document("unknown-tracker.md").replace("\"other\"", "\"oth\\u001ber\\u202e\"");
    let file = scratch.file(&text);
    let (error, stderr) = scratch.refused(&["add", file.to_str().unwrap(), "--reason", REASON]);
    assert!(error.is::<brokkr_core::work_order::WorkOrderError>());
    assert_eq!(
        stderr,
        "error: the work order cites unknown tracker 'other'\n"
    );
}

#[test]
fn unreadable_or_non_utf8_files_are_refused_with_their_path() {
    let scratch = Scratch::new();
    let missing = scratch.dir.path().join("missing.md");
    let invalid = scratch.dir.path().join("invalid.md");
    std::fs::write(&invalid, b"\xff").unwrap();
    for (file, kind) in [
        (missing, io::ErrorKind::NotFound),
        (invalid, io::ErrorKind::InvalidData),
    ] {
        let (error, stderr) = scratch.refused(&["add", file.to_str().unwrap(), "--reason", REASON]);
        assert_eq!(error.downcast_ref::<io::Error>().unwrap().kind(), kind);
        assert!(stderr.starts_with(&format!(
            "error: cannot read work order '{}': ",
            file.display()
        )));
    }
}

#[test]
fn bare_orders_lists_an_empty_fresh_journal_and_a_journal_before_orders() {
    let scratch = Scratch::new();
    drop(Store::open(&scratch.db).unwrap());
    assert_eq!(scratch.stdout(&[]), "");
    let exit = crate::run_with(
        scratch.parsed(&[]),
        scratch.dir.path(),
        |_, _, _| unreachable!(),
        None,
        None,
        |_, _, _| unreachable!(),
    );
    assert_eq!(exit.unwrap(), ExitCode::from(Exit::Completed));
    let connection = scratch.sql();
    connection
        .execute_batch(
            "DELETE FROM meta WHERE key = 'orders_schema'; DROP TABLE work_order_events;",
        )
        .unwrap();
    assert_eq!(scratch.stdout(&[]), "");
    assert_eq!(scratch.stdout(&["list", "--json"]), "[]\n");
    let (error, stderr) = scratch.refused(&["show", "WO-1"]);
    assert!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Orders(OrderRefusal::Unknown(id))) if id.number() == 1
    ));
    assert_eq!(stderr, "error: no work order was proposed as WO-1\n");
    let version_rows: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM meta WHERE key = 'orders_schema'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(version_rows, 0);
}

#[test]
fn readouts_keep_the_refusal_for_a_later_storage_version() {
    let scratch = Scratch::new();
    drop(Store::open(&scratch.db).unwrap());
    scratch
        .sql()
        .execute(
            "UPDATE meta SET value = '2' WHERE key = 'orders_schema'",
            [],
        )
        .unwrap();
    for words in [&["list"][..], &["show", "WO-1"]] {
        let (error, stderr) = scratch.refused(words);
        assert!(matches!(
            error.downcast_ref::<StoreError>(),
            Some(StoreError::Orders(OrderRefusal::SchemaMismatch(2)))
        ));
        assert_eq!(
            stderr,
            "error: this build does not read work order storage version 2\n"
        );
    }
}

#[test]
fn json_lists_the_view_model_in_numeric_order() {
    let scratch = Scratch::new();
    scratch.add(&fixture(COMPLETE));
    let open = document(DRAFT)
        .replace("\"title\":", "\"id\": \"WO-2\", \"title\":")
        .replace(
            "\"asks\": []",
            "\"asks\": [{\"number\": 1, \"question\": \"When?\"}]",
        );
    scratch.add(&scratch.file(&open));
    scratch.stdout(&["rule", "WO-12", "--reason", REASON]);
    let rows: Value = serde_json::from_str(&scratch.stdout(&["list", "--json"])).unwrap();
    assert_eq!(
        rows,
        json!([
            {"id": "WO-2", "state": "proposed", "open_asks": 1, "title": "Work order draft"},
            {"id": "WO-12", "state": "ruled", "open_asks": 0, "title": "Render the work order"}
        ])
    );
}

#[test]
fn every_packaged_tracker_is_embedded_and_parses() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("trackers");
    let mut packaged = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect::<Vec<_>>();
    packaged.sort_unstable();
    let mut embedded = SHIPPED.to_vec();
    embedded.sort_unstable();
    assert_eq!(embedded, packaged);
    shipped_trackers();
}
