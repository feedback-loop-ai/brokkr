//! Exercise the parsed verbs against a separate scratch journal per test.

use brokkr_store::{Attribution, Store};
use clap::Parser;
use rusqlite::Connection;
use serde_json::{json, Value};

use super::*;
use crate::{report_to, Cli, Cmd, GroupedCmd};

const DRAFT: &str = "draft.md";
const COMPLETE: &str = "complete.md";
const REASON: &str = "the operator's reason";
const CUSTOM_TRACKER: &str = r#"{"tracker":"custom","hosts":["github.com"]}"#;
const COUNTERFEIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

struct Scratch {
    dir: tempfile::TempDir,
    db: PathBuf,
}

impl Scratch {
    fn bare() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("orders.db");
        Self { dir, db }
    }

    fn new() -> Self {
        let scratch = Self::bare();
        crate::folder::folder_tests::plant_library(scratch.dir.path());
        scratch
    }

    fn folder() -> Self {
        let scratch = Self::bare();
        fs::create_dir(scratch.dir.path().join(".brokkr")).unwrap();
        scratch
    }

    fn tracker(&self, name: &str, text: &str) -> PathBuf {
        let file = self.dir.path().join(name);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, text).unwrap();
        file
    }

    fn sql(&self) -> Connection {
        Connection::open(&self.db).unwrap()
    }

    fn file(&self, text: &str) -> PathBuf {
        let file = self.dir.path().join("order.md");
        std::fs::write(&file, text).unwrap();
        file
    }

    fn parse(&self, words: &[&str]) -> Result<Cli, clap::Error> {
        let tail = ["--db", self.db.to_str().unwrap()];
        let argv = ["brokkr", "orders"].iter().chain(words).chain(&tail);
        Cli::try_parse_from(argv)
    }

    fn parsed(&self, words: &[&str]) -> Cli {
        self.parse(words).unwrap()
    }

    fn shown_digest(&self, id: &str) -> String {
        let shown = self.stdout(&["show", id]);
        digest_line(&shown)
            .strip_prefix("digest ")
            .unwrap()
            .to_owned()
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

    fn refused_order(&self, words: &[&str]) -> (OrderRefusal, String) {
        let (error, stderr) = self.refused(words);
        match error.downcast::<StoreError>() {
            Ok(StoreError::Orders(refusal)) => (refusal, stderr),
            Ok(other) => panic!("expected a work order refusal, got store error {other:?}"),
            Err(other) => panic!("expected a work order refusal, got {other:?}"),
        }
    }

    fn readout_refusals(&self) -> impl Iterator<Item = (anyhow::Error, String)> + '_ {
        [&["list"][..], &["show", "WO-1"]]
            .into_iter()
            .map(|words| self.refused(words))
    }

    fn refused_before_open(&self, check: impl Fn(TrackerError, String)) {
        let file = self.dir.path().join("missing.md");
        let file = file.to_str().unwrap();
        for words in [
            &["list"][..],
            &["show", "WO-1"],
            &["add", file, "--reason", REASON],
            &["edit", "WO-1", file, "--reason", REASON],
            &rule_words("WO-1", COUNTERFEIT),
        ] {
            let (error, stderr) = self.refused(words);
            check(error.downcast::<TrackerError>().unwrap(), stderr);
        }
        assert!(!self.db.exists());
    }

    fn refused_definition(&self, file: &Path, detail: &str) {
        self.refused_before_open(|error, stderr| {
            assert!(matches!(error, TrackerError::Definition { path, .. } if path == file));
            assert_eq!(
                stderr,
                format!(
                    "error: invalid tracker definition in '{}': {detail}\n",
                    file.display()
                )
            );
        });
    }

    fn add(&self, file: &Path) -> String {
        self.stdout(&["add", file.to_str().unwrap(), "--reason", REASON])
    }
}

fn rule_words<'a>(id: &'a str, digest: &'a str) -> [&'a str; 6] {
    ["rule", id, "--digest", digest, "--reason", REASON]
}

fn digest_line(shown: &str) -> &str {
    shown.lines().next().unwrap()
}

fn shown(text: &str, digest: &str) -> String {
    format!("digest {digest}\n{text}")
}

fn shipped_trackers() -> Trackers {
    trackers(None).unwrap()
}

fn custom_reference() -> String {
    document(COMPLETE).replace("\"tracker\": \"github\"", "\"tracker\": \"custom\"")
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
    let (digest, document) = recorded.split_once('\n').unwrap();
    assert_eq!(
        digest,
        "digest 167afdc611b69fc43f54e9c13f4e5aba53d2b99e52f7114125217225b32659ae"
    );
    let duplicate = scratch.file(document);
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
fn list_prints_a_title_with_format_marks_and_without_controls() {
    let scratch = Scratch::new();
    let text = document(DRAFT).replace(
        "\"Work order draft\"",
        "\"Work\\u001b order\\u202e draft\\u009b\"",
    );
    scratch.add(&scratch.file(&text));

    assert_eq!(
        scratch.stdout(&["list"]),
        "WO-1  proposed  0  Work order<U+202E> draft\n"
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
    let digest = brokkr_core::work_order::text_digest(&expected);
    assert_eq!(
        scratch.stdout(&["show", "WO-12"]),
        shown(&expected, &digest)
    );
}

#[test]
fn ruling_waits_for_closed_asks_and_an_edit_returns_the_order_to_proposed() {
    let scratch = Scratch::new();
    let complete = document(COMPLETE);
    let open = complete.replacen(",\n      \"ruling\": 1", "", 1);
    let file = scratch.file(&open);
    scratch.add(&file);
    let stale = scratch.shown_digest("WO-12");
    let stale_rule = rule_words("WO-12", &stale);
    let (refusal, stderr) = scratch.refused_order(&stale_rule);
    assert!(matches!(refusal, OrderRefusal::OpenAsk(id) if id.number() == 12));
    assert_eq!(
        stderr,
        "error: WO-12 has an open ask; close its asks before ruling it\n"
    );

    let edit = ["edit", "WO-12", file.to_str().unwrap(), "--reason", REASON];
    let expected_edit = "edited WO-12, proposed until ruled again\n";
    assert_eq!(scratch.file(&complete), file);
    assert_eq!(scratch.stdout(&edit), expected_edit);
    let (refusal, stderr) = scratch.refused_order(&stale_rule);
    assert!(matches!(refusal, OrderRefusal::StaleDigest(id) if id.number() == 12));
    assert_eq!(
        stderr,
        "error: the text of WO-12 changed since that digest was read; show it again and rule the digest it prints\n"
    );

    let digest = scratch.shown_digest("WO-12");
    assert_eq!(
        scratch.stdout(&rule_words("WO-12", &digest)),
        "ruled WO-12\n"
    );
    assert_eq!(
        scratch.stdout(&["show", "WO-12"]),
        shown(&complete, &digest)
    );
    assert_eq!(scratch.stdout(&edit), expected_edit);
    let proposed = complete.replace("\"state\": \"ruled\"", "\"state\": \"proposed\"");
    assert_eq!(
        scratch.stdout(&["show", "WO-12"]),
        shown(&proposed, &digest)
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
    let extended =
        Trackers::from_definitions(SHIPPED.iter().copied().chain([CUSTOM_TRACKER])).unwrap();
    let order = WorkOrder::parse(&custom_reference(), &extended).unwrap();
    let by = Attribution {
        operator: "operator",
        reason: "propose from another tracker set",
    };
    Store::open(&scratch.db)
        .unwrap()
        .order_add(order, by)
        .unwrap();

    let digest = "0".repeat(64);
    let (refusal, stderr) = scratch.refused_order(&rule_words("WO-12", &digest));
    assert!(matches!(refusal, OrderRefusal::Unreadable(id, _) if id.number() == 12));
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
fn show_prints_each_line_with_format_marks_and_without_controls() {
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
            "Keep the work<U+202E> clear.\n<!-- The review still reads this line.",
        ),
        &shipped_trackers(),
    )
    .unwrap()
    .with_id("WO-1".parse().unwrap())
    .render();
    let canonical = WorkOrder::parse(&text, &shipped_trackers())
        .unwrap()
        .with_id("WO-1".parse().unwrap())
        .render();
    let digest = brokkr_core::work_order::text_digest(&canonical);
    assert_eq!(scratch.stdout(&["show", "WO-1"]), shown(&expected, &digest));
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
        "error: the work order cites unknown tracker 'other<U+202E>'\n"
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
    for (error, stderr) in scratch.readout_refusals() {
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
    let digest = scratch.shown_digest("WO-12");
    scratch.stdout(&rule_words("WO-12", &digest));
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
fn rule_requires_a_lowercase_digest_and_edit_names_its_id() {
    use clap::error::ErrorKind;

    let scratch = Scratch::new();
    let error = scratch
        .parse(&["rule", "WO-1", "--reason", REASON])
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::MissingRequiredArgument);
    assert!(error.to_string().contains("--digest <HEX>"));

    for digest in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        let error = scratch.parse(&rule_words("WO-1", &digest)).err().unwrap();
        assert_eq!(error.kind(), ErrorKind::ValueValidation);
        assert!(error
            .to_string()
            .contains("expected a digest of 64 lowercase hexadecimal characters"));
    }

    let help = scratch.parse(&["edit", "--help"]).err().unwrap();
    assert_eq!(help.kind(), ErrorKind::DisplayHelp);
    assert!(help
        .to_string()
        .contains("Usage: brokkr orders edit [OPTIONS] --reason <WHY> <ID> <FILE>"));
}

#[test]
fn list_and_show_refuse_damaged_work_order_storage() {
    for sql in [
        "UPDATE meta SET value = 'damaged' WHERE key = 'orders_schema'",
        "DROP TABLE work_order_events",
    ] {
        let scratch = Scratch::new();
        scratch.add(&fixture(DRAFT));
        scratch.sql().execute_batch(sql).unwrap();
        for (error, stderr) in scratch.readout_refusals() {
            assert!(matches!(
                error.downcast_ref::<StoreError>(),
                Some(StoreError::Damaged { journal, damage: brokkr_store::Damage::Orders }) if *journal == scratch.db
            ));
            assert!(stderr.starts_with(&format!(
                "error: the journal '{}' is damaged (its work-order storage is damaged);",
                scratch.db.display()
            )));
        }
    }
}

#[test]
fn folder_and_library_trackers_are_used_by_add_show_list_and_rule() {
    for (scratch, tracker) in [
        (Scratch::folder(), ".brokkr/trackers/custom.json"),
        (Scratch::new(), "trackers/custom.json"),
    ] {
        scratch.tracker(tracker, CUSTOM_TRACKER);
        let text = custom_reference();
        scratch.add(&scratch.file(&text));
        let digest = scratch.shown_digest("WO-12");
        let proposed = text.replace("\"state\": \"ruled\"", "\"state\": \"proposed\"");
        assert_eq!(
            scratch.stdout(&["show", "WO-12"]),
            shown(&proposed, &digest)
        );
        assert_eq!(
            scratch.stdout(&rule_words("WO-12", &digest)),
            "ruled WO-12\n"
        );
        assert_eq!(
            scratch.stdout(&["list"]),
            "WO-12  ruled  0  Render the work order\n"
        );
    }
}

#[test]
fn shipped_trackers_work_without_an_overlay_and_ignore_other_files() {
    let bare = Scratch::bare();
    bare.tracker("trackers/ignored.json", "invalid");
    let library = Scratch::new();
    library.tracker("trackers/ignored.txt", "invalid");
    for scratch in [bare, library, Scratch::new()] {
        assert!(scratch
            .add(&fixture(COMPLETE))
            .starts_with("proposed WO-12\n"));
        assert_eq!(
            scratch.stdout(&[]),
            "WO-12  proposed  0  Render the work order\n"
        );
    }
}

#[test]
fn tracker_redefinitions_name_the_file_and_word_before_every_verb() {
    let shipped = Scratch::new();
    let duplicate = shipped.tracker(
        "trackers/shipped.json",
        r#"{"tracker":"github","hosts":["impostor.example"]}"#,
    );
    let adopter = Scratch::new();
    let later = adopter.tracker("trackers/a.json", CUSTOM_TRACKER);
    adopter.tracker("trackers/Z.json", CUSTOM_TRACKER);
    for (scratch, file, word) in [(shipped, duplicate, "github"), (adopter, later, "custom")] {
        scratch.refused_definition(
            &file,
            &format!("tracker word '{word}' already has a definition"),
        );
    }
}

#[test]
fn malformed_tracker_definitions_refuse_before_every_verb_and_name_the_file() {
    let scratch = Scratch::new();
    let file = scratch.tracker("trackers/bad.json", r#"{"tracker":"custom","hosts":[]}"#);
    scratch.refused_definition(
        &file,
        "a tracker definition must list at least one link host",
    );
}

#[test]
fn unreadable_tracker_paths_refuse_before_every_verb_and_name_the_path() {
    let directory = Scratch::new();
    let not_dir = directory.tracker("trackers", "");
    let file = Scratch::new();
    let bad = file.tracker("trackers/bad.json", "");
    fs::write(&bad, b"\xff").unwrap();
    let dangling = Scratch::new();
    let link = dangling.dir.path().join("trackers");
    std::os::unix::fs::symlink("absent", &link).unwrap();
    for (scratch, path, kind) in [
        (directory, not_dir, io::ErrorKind::NotADirectory),
        (file, bad, io::ErrorKind::InvalidData),
        (dangling, link, io::ErrorKind::NotFound),
    ] {
        scratch.refused_before_open(|error, stderr| {
            assert!(matches!(error, TrackerError::Read { path: named, source }
                if named == path && source.kind() == kind));
            assert!(stderr.starts_with(&format!(
                "error: cannot read trackers at '{}': ",
                path.display()
            )));
        });
    }
}

#[test]
fn show_marks_tag_block_letters_in_the_intent() {
    let scratch = Scratch::new();
    let tags: String = (0xe0041..=0xe005a)
        .map(|point| char::from_u32(point).unwrap())
        .collect();
    let text = document(DRAFT).replace("Keep the work clear.", &format!("Keep {tags} visible."));
    scratch.add(&scratch.file(&text));
    let shown = scratch.stdout(&["show", "WO-1"]);
    let line = shown.lines().find(|line| line.starts_with("Keep "));
    assert_eq!(
        line,
        Some(concat!(
            "Keep <U+E0041><U+E0042><U+E0043><U+E0044><U+E0045><U+E0046><U+E0047>",
            "<U+E0048><U+E0049><U+E004A><U+E004B><U+E004C><U+E004D><U+E004E>",
            "<U+E004F><U+E0050><U+E0051><U+E0052><U+E0053><U+E0054><U+E0055>",
            "<U+E0056><U+E0057><U+E0058><U+E0059><U+E005A> visible.",
        ))
    );
}

#[test]
fn show_puts_the_real_digest_above_a_counterfeit_and_rule_refuses_the_counterfeit() {
    let scratch = Scratch::new();
    let counterfeit_line = format!("digest {COUNTERFEIT}");
    let text = document(DRAFT).replace("Keep the work clear.", &counterfeit_line);
    scratch.add(&scratch.file(&text));
    let order = WorkOrder::parse(&text, &shipped_trackers())
        .unwrap()
        .with_id("WO-1".parse().unwrap());
    let digest = brokkr_core::work_order::text_digest(&order.render());
    let shown = scratch.stdout(&["show", "WO-1"]);
    assert_eq!(digest_line(&shown), format!("digest {digest}"));
    assert!(shown.lines().skip(1).any(|line| line == counterfeit_line));
    let (refusal, _) = scratch.refused_order(&rule_words("WO-1", COUNTERFEIT));
    assert!(matches!(refusal, OrderRefusal::StaleDigest(id) if id.number() == 1));
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
