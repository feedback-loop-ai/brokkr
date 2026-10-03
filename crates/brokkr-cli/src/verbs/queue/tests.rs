//! `brokkr queue`, parsed as the operator types it: each command is its
//! own invocation, opening the journal afresh, as separate processes do.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use brokkr_store::{EntryId, NewEntry, Store};
use clap::Parser;
use serde_json::json;

use super::*;
use crate::{failure_line, report, Cli, Cmd};

/// The exit a command that succeeded leaves with.
fn completed() -> ExitCode {
    ExitCode::from(Exit::Completed)
}

/// Parse `brokkr queue <words>` against `db` and run it in `workspace`.
fn queue_in(workspace: &Path, db: &Path, words: &[&str]) -> Result<ExitCode> {
    let tail = ["--db", db.to_str().unwrap()];
    let argv = ["brokkr", "queue"].iter().chain(words).chain(&tail);
    let Cmd::Queue { command } = Cli::try_parse_from(argv).unwrap().command else {
        unreachable!("parsed as a queue command")
    };
    queue(workspace, command)
}

/// The listing as the handler reads it: each entry and its launch.
fn listed(db: &Path) -> Vec<(QueueEntry, QueuedLaunch)> {
    let store = Store::open(db).unwrap();
    let entries = store.queue_list().unwrap().into_iter();
    entries
        .map(|entry| {
            let launch = QueuedLaunch::decode(&entry.payload).unwrap();
            (entry, launch)
        })
        .collect()
}

#[test]
fn the_queue_keeps_order_holds_and_runs_across_invocations() {
    let dir = tempfile::tempdir().unwrap();
    let (ws, db) = (dir.path(), dir.path().join("forge.db"));
    let commands: [&[&str]; 6] = [
        &[
            "add",
            "--bundle",
            "b",
            "--feature",
            "first",
            "--reason",
            "r",
        ],
        &[
            "add",
            "--recipe",
            "story",
            "--feature",
            "second\x1b[2J",
            "--after",
            "1:completed",
            "--priority",
            "-2",
            "--reason",
            "after one",
        ],
        &[
            "add",
            "--bundle",
            "b",
            "--feature",
            "third",
            "--reason",
            "r",
        ],
        &["move", "3", "--to", "1", "--reason", "first now"],
        &["hold", "2", "--reason", "not yet"],
        &["drop", "1", "--reason", "not wanted"],
    ];
    for words in commands {
        assert_eq!(queue_in(ws, &db, words).unwrap(), completed(), "{words:?}");
    }
    let mut store = Store::open(&db).unwrap();
    store
        .create_run("run-3", "third", "b", &json!({"schema": "run-manifest/v1"}))
        .unwrap();
    store.queue_claim(EntryId(3), "run-3").unwrap();
    for words in [
        ["release", "2", "--reason", "go"],
        ["hold", "2", "--reason", "again"],
    ] {
        assert_eq!(queue_in(ws, &db, &words).unwrap(), completed(), "{words:?}");
    }

    let entries = listed(&db);
    // Every path the entry holds is anchored to the workspace it was
    // queued in.
    let (b, recipes) = (ws.join("b"), ws.join("recipes"));
    let shown = b.display().to_string();
    let w = shown.len();
    assert_eq!(
        table(&rows(&entries)),
        format!(
            "PLACE  ENTRY  STATE    PRIORITY  AFTER        RUN    {:w$}  FEATURE\n\
             1      2      held     -2        1:completed  -      {:w$}  second[2J\n\
             -      3      claimed  0         -            run-3  {shown}  third\n",
            "BUNDLE", "recipe story"
        )
    );
    let stamp = |n: usize| entries[n].0.added_at.clone();
    let launch = |bundle, feature| {
        json!({"encoding": "queued-launch/v1", "workspace": ws, "bundle": bundle,
               "repo": null, "secrets": null, "feature": feature, "map": "unmapped",
               "dispatch": null})
    };
    let recipe = json!({"recipe": {"name": "story", "recipes_dir": recipes}});
    assert_eq!(
        serde_json::to_value(rows(&entries)).unwrap(),
        json!([
            {"place": 1, "entry": 2, "state": "held", "run": null, "priority": -2,
             "waits": [{"entry": 1, "on": "completed"}], "added_at": stamp(0),
             "launch": launch(recipe, "second\x1b[2J")},
            {"place": null, "entry": 3, "state": "claimed", "run": "run-3", "priority": 0,
             "waits": [], "added_at": stamp(1), "launch": launch(json!({"dir": b}), "third")},
        ])
    );
    for json in [&["list"][..], &["list", "--json"]] {
        assert_eq!(queue_in(ws, &db, json).unwrap(), completed());
    }
}

#[test]
fn an_entry_names_the_workspace_it_was_queued_in_absolutely() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let words = ["add", "--bundle", "b", "--feature", "f", "--reason", "r"];
    queue_in(Path::new("."), &db, &words).unwrap();
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(listed(&db)[0].1.workspace, here);
}

/// `brokkr queue` reaches its handler through the CLI's own dispatch,
/// not only through the handler this file calls directly.
#[test]
fn the_cli_dispatches_brokkr_queue_to_its_handler() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    Store::open(&db).unwrap();
    let argv = ["brokkr", "queue", "list", "--db", db.to_str().unwrap()];
    let cli = Cli::try_parse_from(argv).unwrap();
    assert_eq!(crate::run(cli).unwrap(), completed());
}

#[test]
fn an_empty_queue_says_so() {
    assert_eq!(table(&[]), "the queue is empty\n");
    assert_eq!(serde_json::to_value(rows(&[])).unwrap(), json!([]));
}

#[test]
fn each_refusal_leaves_with_its_own_words() {
    let dir = tempfile::tempdir().unwrap();
    let (ws, db) = (dir.path(), dir.path().join("forge.db"));
    let add = ["add", "--bundle", "b", "--feature", "f", "--reason", "r"];
    queue_in(ws, &db, &add).unwrap();
    queue_in(ws, &db, &["hold", "1", "--reason", "r"]).unwrap();
    let refused = |words: &[&str]| {
        let error = queue_in(ws, &db, words).unwrap_err();
        (failure_line(&error), report(&error))
    };
    let failed = |line: &str| (line.to_string(), ExitCode::from(Exit::Failed));
    assert_eq!(
        refused(&["hold", "1", "--reason", "r"]),
        failed("error: queue entry 1 is already held")
    );
    assert_eq!(
        refused(&["move", "1", "--to", "2", "--reason", "r"]),
        failed("error: queue entry 1 cannot move to place 2: the queue's places run 1 to 1")
    );
    assert_eq!(
        refused(&["drop", "7", "--reason", "r"]),
        failed("error: queue entry 7 does not exist")
    );
    assert_eq!(
        refused(&[&add[..], &["--after", "7:ended"]].concat()),
        failed("error: a new queue entry cannot wait for entry 7, which does not exist")
    );
    // What `brokkr run` refuses before reading anything is not queued.
    let map = json!({"schema": "forge.realms/v4", "journal": "forge.db",
        "realms": [{"name": "here", "path": ".", "default_branch": "main"}]});
    std::fs::write(ws.join("map.json"), map.to_string()).unwrap();
    let realms = ws.join("map.json");
    let beside = ["--realms", realms.to_str().unwrap(), "--dispatch", "d.json"];
    assert_eq!(
        refused(&[&add[..], &beside].concat()),
        failed(
            "error: a run with --dispatch cannot pin the map named by --realms: the \
             Looper-bound run-manifest/v2 lineage carries no world, and dropping the map \
             silently would leave the run unable to say which one it believed in. Run \
             without --dispatch, or without --realms, until a jointly agreed v2-lineage \
             manifest version exists"
        )
    );
    assert_eq!(listed(&db).len(), 1);

    // A payload this brokkr cannot read refuses the listing, naming it.
    let mut store = Store::open(&db).unwrap();
    let foreign = NewEntry {
        payload: "{}",
        priority: 0,
        waits: &[],
    };
    let by = brokkr_store::Attribution {
        operator: "o",
        reason: "r",
    };
    store.queue_add(foreign, by).unwrap();
    assert_eq!(
        refused(&["list"]),
        failed(
            "error: queue entry 2: reading a queued launch: missing field `encoding` at line 1 \
             column 2"
        )
    );

    // A condition the queue does not know never reaches it.
    let argv = [
        "brokkr",
        "queue",
        "add",
        "--bundle",
        "b",
        "--feature",
        "f",
        "--reason",
        "r",
    ];
    let unknown = Cli::try_parse_from(argv.iter().chain(&["--after", "1:named"]));
    assert_eq!(
        unknown.err().unwrap().to_string(),
        "error: invalid value '1:named' for '--after <ENTRY:CONDITION>': '1:named' is not \
         ENTRY:CONDITION, with CONDITION one of completed, ended\n\n\
         For more information, try '--help'.\n"
    );

    // Nor does a flag only `brokkr run` reads: queuing opens no run view.
    let no_view = Cli::try_parse_from(argv.iter().chain(&["--no-view"]));
    assert_eq!(
        no_view.err().unwrap().kind(),
        clap::error::ErrorKind::UnknownArgument
    );
}
