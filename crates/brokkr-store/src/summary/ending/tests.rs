//! The ending's columns (decision 0064, addendum choice 5): each append
//! keeps them as its fold and reading state them, a page reads back what
//! the run read whole states for every ending, ruling 4's comparison holds
//! every one, a writer from before them sends its row behind and the next
//! writable open refolds it, the open's repair does so where their trigger
//! was gone, and a table from before them is made anew while
//! `DATABASE_SCHEMA` stays where it was.

use brokkr_core::ending::{closing, Citation, Closing, Ending};
use brokkr_core::envelope::EventType;
use brokkr_core::page::{Lane, Range};
use brokkr_core::summary::CITATION_BYTES;
use rusqlite::types::Value;
use serde_json::{json, Value as Json};

use super::{PART, WHOLE};
use crate::schema::DATABASE_SCHEMA;
use crate::summary::aggregate::tests::{
    columns_of, each_edit_named, read_whole_then_repaired, ruled_turn,
};
use crate::summary::summary_intact;
use crate::summary::tests::{older_table, planted, reopened, started};
use crate::test_support::{append, every_run_reads_as_its_fold, lags, lane, text};
use crate::tests::store;
use crate::Store;

/// Every ending column of `run_id`'s row, in the row's order.
fn kept(store: &Store, run_id: &str) -> Vec<Value> {
    columns_of(store, run_id, PART.columns)
}

/// The six columns: the ending's word, the realm, run and ref it cites,
/// whether it stood over an earlier one, and the stamp.
fn columns(ending: &str, cited: [Option<&str>; 3], stood_over: i64, seq: i64) -> Vec<Value> {
    let cited = cited.map(|cited| cited.map_or(Value::Null, text));
    let mut columns = vec![text(ending)];
    columns.extend(cited);
    columns.extend([Value::Integer(stood_over), Value::Integer(seq)]);
    columns
}

/// `run_id` started and stopped by an accepted stop whose `args` are
/// `args`: four events.
fn stopped(store: &mut Store, run_id: &str, args: Json) {
    started(store, run_id);
    let commanded = json!({"command_id": "s", "command": "stop", "operator": "op", "args": args});
    append(store, run_id, EventType::OperatorCommanded, commanded);
    let accepted = json!({"command_id": "s", "operator": "op", "reason": "enough"});
    append(store, run_id, EventType::OperatorAccepted, accepted);
    append(store, run_id, EventType::RunStopped, json!({}));
}

/// The `operator/commanded` payload of a `classify` naming `cause`.
fn classify(cause: &str) -> Json {
    let args = json!({"cause": cause, "reason": "named later"});
    json!({"command_id": "c", "command": "classify", "operator": "op", "args": args})
}

/// `run_id` started, ruled from `phase` on `result`, flagged where
/// `flagged`, and ended there: completed where the ruling routes to `done`,
/// else the policy table's hard stop.
fn policy(store: &mut Store, run_id: &str, (phase, result, flagged): (&str, &str, bool)) {
    started(store, run_id);
    let done = phase == "review" && result != "security-hold";
    let next = if done { "done" } else { "stop" };
    let mut ruling = json!({"from": phase, "result": result, "rule_id": "R", "next": next});
    if flagged {
        ruling["severity"] = json!("flagged");
    }
    for (event_type, payload) in ruled_turn(phase, "e1", ruling) {
        append(store, run_id, event_type, payload);
    }
    if done {
        append(store, run_id, EventType::RunCompleted, json!({}));
        return;
    }
    let stop = json!({"phase": "stop"});
    append(store, run_id, EventType::PhaseEntered, stop);
    append(store, run_id, EventType::RunStopped, json!({}));
}

/// One run of each of decision 0064's twelve endings, each named for it.
fn every_ending(store: &mut Store) {
    policy(store, "shipped", ("review", "clean", false));
    policy(store, "shipped-flagged", ("review", "residual", true));
    policy(store, "refused", ("review", "security-hold", false));
    policy(store, "failed", ("verify", "fail", false));
    policy(store, "blocked", ("implement", "blocked", false));
    let cause = |cause: &str| json!({ "cause": cause });
    for word in ["exhausted", "infrastructure", "operator-stopped"] {
        stopped(store, word, cause(word));
    }
    let by = json!({"cause": "superseded", "by": {"realm": "next-door", "run_id": "later"}});
    stopped(store, "superseded", by);
    let landed = json!({"cause": "delivered-elsewhere", "ref": "abc123"});
    stopped(store, "delivered-elsewhere", landed);
    stopped(store, "withdrawn", cause("operator-stopped"));
    append(
        store,
        "withdrawn",
        EventType::OperatorCommanded,
        classify("withdrawn"),
    );
    stopped(store, "unclassified", json!({}));
}

/// Addendum choice 5: every finished row a page reads states the ending,
/// citation and stood-over fact the run read whole states, by the one
/// reading, for each of the twelve endings; and the comparison `brokkr
/// doctor` makes finds no row of them that is not its run's, and names
/// the one an edit moved.
#[test]
fn a_paged_row_states_what_its_run_read_whole_states_for_every_ending() {
    let (_dir, mut store) = store();
    every_ending(&mut store);
    let finished = Range {
        lane: Lane::Finished,
        bound: None,
    };
    let paged = store.section(&[finished], None, 20).unwrap().into_iter();
    let mut read: Vec<(String, Option<Closing>)> = paged
        .map(|paged| paged.map(|row| (row.run_id, row.closing)).unwrap())
        .collect();
    read.sort_by(|a, b| a.0.cmp(&b.0));
    let mut whole: Vec<(String, Option<Closing>)> = Ending::ALL
        .iter()
        .map(|ending| {
            let events = store.load(ending.word()).unwrap();
            let state = brokkr_core::fold(&events).unwrap();
            (ending.word().to_string(), closing(&events, &state))
        })
        .collect();
    whole.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(read, whole);
    let cited = |run_id: &str| {
        let (_, closing) = read.iter().find(|(id, _)| id == run_id).unwrap();
        let closing = closing.as_ref().expect("every finished run has an ending");
        assert_eq!(closing.ended.ending.word(), run_id);
        assert_eq!(closing.stood_over, run_id == "withdrawn", "{run_id}");
        closing.ended.cited.clone()
    };
    let named: Vec<_> = Ending::ALL
        .iter()
        .map(|ending| cited(ending.word()))
        .collect();
    let run = Citation::Run {
        realm: Some("next-door".into()),
        run_id: "later".into(),
    };
    let reference = Citation::Ref("abc123".into());
    assert_eq!((&named[6], &named[8]), (&Some(run), &Some(reference)));
    assert_eq!(named.iter().flatten().count(), 2, "only those two cite");
    assert_eq!(store.summary_disagreements().unwrap().len(), 0);
    let edit = "UPDATE run_summary SET ending_stood_over = 0 WHERE run_id = 'withdrawn'";
    store.conn.execute(edit, []).unwrap();
    let found = store.summary_disagreements().unwrap();
    let words = "run 'withdrawn': its summary row stands at its head, and its \
                 ending_stood_over is not its fold's";
    assert_eq!(
        found.iter().map(ToString::to_string).collect::<Vec<_>>(),
        [words]
    );
    let restore = "UPDATE run_summary SET ending_stood_over = 1 WHERE run_id = 'withdrawn'";
    store.conn.execute(restore, []).unwrap();
}

#[test]
fn each_append_keeps_the_ending_its_fold_and_reading_state() {
    let (_dir, mut store) = store();
    started(&mut store, "live");
    let mut live = vec![Value::Null; 5];
    live.push(Value::Integer(1));
    assert_eq!(kept(&store, "live"), live, "a live run has no ending");
    started(&mut store, "shipped");
    append(&mut store, "shipped", EventType::RunCompleted, json!({}));
    let shipped = columns("shipped", [None; 3], 0, 2);
    assert_eq!(kept(&store, "shipped"), shipped);
    let here = json!({"cause": "superseded", "by": {"realm": null, "run_id": "later"}});
    stopped(&mut store, "here", here);
    let superseded = columns("superseded", [None, Some("later"), None], 0, 4);
    assert_eq!(kept(&store, "here"), superseded);
    stopped(&mut store, "named", json!({"cause": "infrastructure"}));
    append(
        &mut store,
        "named",
        EventType::OperatorCommanded,
        classify("withdrawn"),
    );
    let over = columns("withdrawn", [None; 3], 1, 5);
    assert_eq!(
        kept(&store, "named"),
        over,
        "a classify stood over the cause"
    );
    stopped(&mut store, "causeless", json!({}));
    let unclassified = columns("unclassified", [None; 3], 0, 4);
    assert_eq!(kept(&store, "causeless"), unclassified);
    append(
        &mut store,
        "causeless",
        EventType::OperatorCommanded,
        classify("exhausted"),
    );
    let named = columns("exhausted", [None; 3], 0, 5);
    assert_eq!(kept(&store, "causeless"), named, "nothing to stand over");
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// Ruling 4's comparison, which `brokkr doctor` and every writer store's
/// drop make, holds each ending column to the run's fold and reading: an
/// edit of any one is named by that column.
#[test]
fn each_ending_column_that_is_not_its_fold_s_is_named() {
    let (_dir, mut store) = store();
    let by = json!({"cause": "superseded", "by": {"realm": "next-door", "run_id": "later"}});
    stopped(&mut store, "r", by);
    let every: Vec<(&str, &str)> = PART.columns.iter().chain(&WHOLE).copied().collect();
    let names: Vec<&str> = every.iter().map(|(name, _)| *name).collect();
    assert_eq!(names.len(), 9);
    assert_eq!(each_edit_named(&store, &every), names);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A binary from before the ending's columns folds a new head and writes
/// `ending` in its own three words, leaving the stamp off: the trigger
/// sends its row behind, and the next writable open refolds it to the row
/// a fresh fold writes.
#[test]
fn a_writer_from_before_the_ending_columns_sends_its_row_behind() {
    let (_dir, mut store) = store();
    stopped(&mut store, "r", json!({"cause": "withdrawn"}));
    assert_eq!(kept(&store, "r"), columns("withdrawn", [None; 3], 0, 4));
    let named = classify("infrastructure");
    planted(&store, "r", EventType::OperatorCommanded, named);
    let older = "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash,
                     aggregated_seq = head_seq, stop_folded_seq = head_seq,
                     ending = 'by_operator'";
    store.conn.execute(older, []).unwrap();
    let folded = columns_of(&store, "r", &[("folded_seq", "INTEGER")]);
    assert_eq!(folded, [Value::Null], "the trigger sent it behind");
    lags(&mut store, &["r"]);
    let store = reopened(store);
    assert_eq!(lane(&store, "r").as_deref(), Some("finished"));
    let refolded = columns("infrastructure", [None; 3], 1, 5);
    assert_eq!(kept(&store, "r"), refolded);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

/// A writer from before the ending's columns may fold while their trigger
/// is gone: a reader that does not repair reads the row behind meanwhile,
/// and the open's repair sends it behind on the ending's own stamp for the
/// backfill to refold.
#[test]
fn a_repairing_ending_trigger_refolds_the_heads_it_missed() {
    let (_dir, mut store) = store();
    stopped(&mut store, "r", json!({"cause": "withdrawn"}));
    let dropped = format!("DROP TRIGGER {}", PART.trigger);
    store.conn.execute_batch(&dropped).unwrap();
    let older = "UPDATE run_summary SET folded_seq = head_seq, folded_hash = head_hash,
                     ending = 'by_operator', ending_folded_seq = NULL";
    store.conn.execute(older, []).unwrap();
    assert_eq!(
        kept(&store, "r")[5],
        Value::Null,
        "no trigger sent it behind"
    );
    lags(&mut store, &["r"]);
    let reader = Store::open_read_only(store.path()).unwrap();
    assert_eq!(lane(&reader, "r").as_deref(), Some("behind"));
    drop(reader);
    refolded_as_withdrawn(&reopened(store));
}

/// `store`'s summary intact, its trigger back, and run `r` refolded to its
/// four events stopped as `withdrawn`, the row a fresh fold writes.
fn refolded_as_withdrawn(store: &Store) {
    assert!(summary_intact(&store.conn).unwrap(), "the summary is whole");
    assert_eq!(kept(store, "r"), columns("withdrawn", [None; 3], 0, 4));
    assert_eq!(every_run_reads_as_its_fold(store), Vec::<String>::new());
}

/// A journal whose summary a build from before the ending's columns made,
/// its `ending` in three words, is read whole by a reader that cannot
/// write: it is not intact, so it does not page (#744's kept read). The
/// next writable open makes it anew and refolds every run into the twelve
/// words. `DATABASE_SCHEMA` does not move for it.
#[test]
fn a_summary_from_before_the_ending_columns_is_made_anew_and_refolded() {
    assert_eq!(DATABASE_SCHEMA, 1);
    // Every column but the ending's new ones, its `ending` in its own
    // words; and every column but the citation's whole parts.
    let older_tables = [
        "name NOT LIKE 'ending\\_%' ESCAPE '\\' OR name = 'ending_rule'",
        "name NOT LIKE 'ending\\_%\\_whole' ESCAPE '\\'",
    ];
    for kept in older_tables {
        let (_dir, mut store) = store();
        stopped(&mut store, "r", json!({"cause": "withdrawn"}));
        let path = store.path().to_path_buf();
        drop(store);
        let older = rusqlite::Connection::open(&path).unwrap();
        older_table(&older, kept);
        let words = "UPDATE run_summary SET ending = 'by_operator'";
        older.execute_batch(words).unwrap();
        drop(older);
        read_whole_then_repaired(&path);
        let store = Store::open(&path).unwrap();
        assert!(store.pages().unwrap());
        refolded_as_withdrawn(&store);
    }
}

/// Decision 0089 ruling 2 for the citation: each part of it is held
/// within [`CITATION_BYTES`] where a page selects it and kept whole last
/// in the row where the bound cut it, so a page reads no citation past
/// its bound, and the row completed by `Store::whole` states what its run
/// read whole states, lossless; ruling 4's comparison holds the whole
/// parts too.
#[test]
fn a_citation_past_its_bound_pages_cut_and_reads_back_whole() {
    let (_dir, mut store) = store();
    let wide = "w".repeat(4 * CITATION_BYTES);
    let cut = "w".repeat(CITATION_BYTES);
    let far = json!({"cause": "superseded", "by": {"realm": wide, "run_id": wide}});
    stopped(&mut store, "far", far);
    let here = json!({"cause": "superseded", "by": {"realm": null, "run_id": wide}});
    stopped(&mut store, "here", here);
    stopped(
        &mut store,
        "landed",
        json!({"cause": "delivered-elsewhere", "ref": wide}),
    );
    let near = json!({"cause": "superseded", "by": {"realm": "next-door", "run_id": "later"}});
    stopped(&mut store, "near", near);
    stopped(&mut store, "uncited", json!({"cause": "withdrawn"}));
    let (wide, cut) = (Some(wide.as_str()), Some(cut.as_str()));
    let held = |run_id: &str| (kept(&store, run_id), columns_of(&store, run_id, &WHOLE));
    let whole = |parts: [Option<&str>; 3]| parts.map(|part| part.map_or(Value::Null, text));
    let rows = [
        ("far", [cut, cut, None], [wide, wide, None]),
        ("here", [None, cut, None], [None, wide, None]),
        ("landed", [None, None, cut], [None, None, wide]),
        ("near", [Some("next-door"), Some("later"), None], [None; 3]),
    ];
    for (run_id, page, kept_whole) in rows {
        let ending = if run_id == "landed" {
            "delivered-elsewhere"
        } else {
            "superseded"
        };
        let expected = (columns(ending, page, 0, 4), whole(kept_whole).to_vec());
        assert_eq!(held(run_id), expected, "{run_id}");
    }
    let finished = Range {
        lane: Lane::Finished,
        bound: None,
    };
    let cut = || "w".repeat(CITATION_BYTES);
    let mut paged: Vec<(String, Option<Citation>)> = Vec::new();
    for row in store.section(&[finished], None, 20).unwrap() {
        let mut row = row.unwrap();
        let cited = row
            .closing
            .as_ref()
            .and_then(|closing| closing.ended.cited.clone());
        paged.push((row.run_id.clone(), cited));
        store.whole(&mut row).unwrap().unwrap();
        let events = store.load(&row.run_id).unwrap();
        let state = brokkr_core::fold(&events).unwrap();
        assert_eq!(row.closing, closing(&events, &state), "{}", row.run_id);
    }
    paged.sort_by(|a, b| a.0.cmp(&b.0));
    let run = |realm: Option<String>, run_id: String| Some(Citation::Run { realm, run_id });
    let expected = [
        ("far", run(Some(cut()), cut())),
        ("here", run(None, cut())),
        ("landed", Some(Citation::Ref(cut()))),
        ("near", run(Some("next-door".into()), "later".into())),
        ("uncited", None),
    ]
    .map(|(run_id, cited)| (run_id.to_string(), cited));
    assert_eq!(paged, expected, "a page reads each part within its bound");
    assert_eq!(store.summary_disagreements().unwrap().len(), 0);
}
