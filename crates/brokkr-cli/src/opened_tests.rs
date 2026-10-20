//! Decision 0089 ruling 5: every readout of one run opens it, its chain
//! read whole and verified before anything else of it is read, and keeps
//! the store's refusal of a corrupt run in the store's own words, however
//! the fleet's page, which checks a row against its run's head alone,
//! lists the run.

use std::path::Path;

use brokkr_store::test_support::unmigrate;
use brokkr_store::Store;
use brokkr_view::Quarantine;
use clap::Parser;

use crate::tests::running_store;
use crate::tui_read::tui_views;
use crate::{run, tui, Cli};

/// The store's refusal of a run whose first event was edited.
const EDITED: &str = "chain: event 1: event_hash does not match canonical content";

/// Runs `r1` and `r2` in a fresh journal at `db`, then `r1`'s first event
/// edited beneath its head by a hand at the file: its summary row still
/// stands at its head.
fn edited_below_its_head(db: &Path) {
    running_store(db, "r1");
    running_store(db, "r2");
    let journal = rusqlite::Connection::open(db).unwrap();
    journal
        .execute_batch("DROP TRIGGER events_append_only_update")
        .unwrap();
    let edit = r#"UPDATE events SET envelope = replace(envelope, '"manifest":{}',
        '"manifest":{"edited":true}') WHERE run_id = 'r1' AND seq = 1"#;
    assert_eq!(journal.execute(edit, []).unwrap(), 1);
}

/// The console's frame over `db`, the operator on `r1`: whether it drew a
/// run view, and the fleet's row of `r1`.
fn opened_in_the_console(db: &Path) -> (bool, brokkr_view::RunRow) {
    let ask = tui::Ask {
        run: Some("r1"),
        subject: None,
        force: true,
        fleet: true,
        tab: 0,
        window: Default::default(),
    };
    let clock = || "2026-10-20T00:00:00Z".to_string();
    let views = tui_views(db, true, ask, &mut None, &mut None, clock).unwrap();
    let views = views.expect("a forced frame is drawn");
    let row = views.runs.runs.into_iter().find(|row| row.run_id == "r1");
    (views.run.is_some(), row.expect("the opened run is listed"))
}

/// `line` run as `brokkr` runs it, refused: the refusal's words, and each
/// cause's beneath them.
fn refused(line: &[&str]) -> Vec<String> {
    let cli = Cli::try_parse_from(line).unwrap();
    let refusal = run(cli).unwrap_err();
    refusal.chain().map(ToString::to_string).collect()
}

/// The binding ruling 5 names: a journal with one event below a run's
/// head edited, whose summary row the page's head check passes, is
/// refused by every readout of that run in the store's words — the
/// console's opened run, on a journal the summary pages and on one it does
/// not, shows it quarantined and draws no run — and the run beside it
/// reads. `/api/view` and the participant routes refuse it as a run they
/// do not serve.
#[test]
fn a_corrupt_run_is_refused_wherever_it_is_opened() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    edited_below_its_head(&db);
    let at = db.to_str().unwrap();
    let out = dir.path().join("out");
    let readouts: [&[&str]; 9] = [
        &["brokkr", "inspect", "--run", "r1", "--db", at],
        &["brokkr", "seats", "--run", "r1", "--db", at],
        &[
            "brokkr",
            "watch",
            "--run",
            "r1",
            "--db",
            at,
            "--once",
            "--no-view",
        ],
        &[
            "brokkr",
            "transcript",
            "--run",
            "r1",
            "--seat",
            "work",
            "--db",
            at,
        ],
        &["brokkr", "ledger", "--run", "r1", "--db", at],
        &["brokkr", "replay", "--run", "r1", "--db", at],
        &["brokkr", "costs", "--run", "r1", "--db", at],
        &[
            "brokkr",
            "export",
            "--run",
            "r1",
            "--db",
            at,
            "--out",
            out.to_str().unwrap(),
        ],
        &["brokkr", "compare", "r2", "r1", "--db", at],
    ];
    let chain = "event 1: event_hash does not match canonical content";
    for line in readouts {
        let words = match line[1] {
            "compare" => vec!["loading run 'r1'", EDITED, chain],
            _ => vec![EDITED, chain],
        };
        assert_eq!(refused(line), words, "{line:?}");
    }
    let view = crate::ui::handle(&db, "/api/view/r1");
    assert_eq!(
        (view.status, view.body.as_str()),
        ("404 Not Found", r#"{"error":"r1 not found"}"#)
    );
    assert!(Store::open_read_only(&db).unwrap().pages().unwrap());
    let quarantined = (false, Some(EDITED), Some(Quarantine::DoesNotLoad));
    let (drawn, row) = opened_in_the_console(&db);
    assert_eq!((drawn, row.detail.as_deref(), row.quarantine), quarantined);
    unmigrate(&rusqlite::Connection::open(&db).unwrap()).unwrap();
    let (drawn, row) = opened_in_the_console(&db);
    assert_eq!((drawn, row.detail.as_deref(), row.quarantine), quarantined);
    let beside = Store::open_read_only(&db).unwrap().load("r2").unwrap();
    assert_eq!(beside.len(), 2, "the run beside it reads");
}
