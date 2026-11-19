//! A damaged journal through the binary (#1000): every read verb that
//! meets the damage prints the one refusal, naming the file, and exits
//! non-zero; a fleet listing quarantines a run whose envelope is damaged
//! and lists the healthy run beside it (#377), and a many-hearth listing
//! refuses a damaged hearth in the same words beside a healthy one.

use std::path::Path;

use brokkr_store::test_support::damaged::{
    damage_table, overwrite, replace_envelope, written, NOT_JSON,
};
use serde_json::json;

use crate::boundary_readouts::brokkr;

/// The one refusal of `db`, damaged by `cause`, without its `error: `.
fn refusal(db: &Path, cause: &str) -> String {
    format!(
        "the journal '{}' is damaged ({cause}); nothing was read from it; restore it from a \
         backup with `brokkr restore` and see the guide's \"A damaged journal\" (#1000)",
        db.display()
    )
}

const MALFORMED: &str = "SQLite reads its disk image as malformed";
const NO_DATABASE: &str = "SQLite reads it as no database";

/// A journal `name`.db in `dir` holding runs `good` and `bad`, damaged by
/// `damage`.
fn damaged(dir: &Path, name: &str, damage: impl FnOnce(&Path)) -> std::path::PathBuf {
    let db = dir.join(format!("{name}.db"));
    written(&db, &["good", "bad"]);
    damage(&db);
    db
}

/// Each verb of `verbs` on `run` of `db` prints nothing on stdout, the
/// refusal on stderr, and fails.
fn refused(dir: &Path, db: &Path, run: &str, verbs: &[&str], words: &str) {
    let db = db.to_str().unwrap();
    for verb in verbs {
        let mut args = vec![*verb, "--db", db];
        if *verb != "runs" {
            args.extend(["--run", run]);
        }
        if *verb == "watch" {
            args.push("--once");
        }
        let (stdout, stderr, ok) = brokkr(dir, &args);
        assert_eq!(
            (stdout.as_str(), stderr.as_str(), ok),
            ("", format!("error: {words}\n").as_str(), false),
            "{verb} --run {run}"
        );
    }
}

/// A page SQLite reads as corrupt and a header that is no database's are
/// each refused by every read verb in the one sentence, naming the file;
/// the console refuses a journal it cannot resolve a run in the same way.
#[test]
fn every_read_verb_refuses_a_damaged_journal_in_one_sentence() {
    let dir = tempfile::tempdir().unwrap();
    let read = ["runs", "inspect", "export", "seats", "watch"];
    let events = damaged(dir.path(), "events", |db| damage_table(db, "events"));
    refused(
        dir.path(),
        &events,
        "good",
        &read,
        &refusal(&events, MALFORMED),
    );
    let summary = damaged(dir.path(), "summary", |db| damage_table(db, "run_summary"));
    refused(
        dir.path(),
        &summary,
        "good",
        &["runs"],
        &refusal(&summary, MALFORMED),
    );
    let header = damaged(dir.path(), "header", |db| {
        overwrite(db, 0, b"not a journal")
    });
    let words = refusal(&header, NO_DATABASE);
    refused(dir.path(), &header, "good", &read, &words);
    refused(dir.path(), &header, "good", &["tui"], &words);
}

/// A damaged `runs` table refuses every verb that reads a run's row; the
/// listing reads the run summary, whose rows are intact, and answers.
#[test]
fn a_damaged_runs_table_refuses_every_verb_that_reads_a_runs_row() {
    let dir = tempfile::tempdir().unwrap();
    let db = damaged(dir.path(), "runs", |db| damage_table(db, "runs"));
    let verbs = ["inspect", "export", "seats", "watch", "tui"];
    refused(dir.path(), &db, "good", &verbs, &refusal(&db, MALFORMED));
    let (stdout, stderr, ok) = brokkr(dir.path(), &["runs", "--db", db.to_str().unwrap()]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("\n  good running work seq 2 "), "{stdout}");
}

/// The recovered copy in #1000: an envelope that is not JSON refuses
/// every read of its run by run and seq, the listing quarantines that run
/// in the same words and lists the healthy run beside it (#377), and the
/// healthy run, whose rows are intact and whose chain verifies, is still
/// inspected and exported (decision 0001).
#[test]
fn an_envelope_that_is_not_json_is_named_and_hides_no_other_run() {
    let dir = tempfile::tempdir().unwrap();
    let db = damaged(dir.path(), "envelope", |db| {
        replace_envelope(db, "bad", 2, NOT_JSON)
    });
    let words = refusal(&db, "run 'bad' holds an envelope at seq 2 that is not JSON");
    let verbs = ["inspect", "export", "seats", "watch"];
    refused(dir.path(), &db, "bad", &verbs, &words);
    let named = db.to_str().unwrap();
    let (stdout, _, ok) = brokkr(dir.path(), &["runs", "--db", named]);
    assert!(ok, "{stdout}");
    assert!(
        stdout.contains(&format!("\n    fold  {words}\n")),
        "{stdout}"
    );
    assert!(stdout.contains("\n  good running work seq 2 "), "{stdout}");
    for verb in ["inspect", "export"] {
        let (_, stderr, ok) = brokkr(dir.path(), &[verb, "--run", "good", "--db", named]);
        assert!(ok, "{verb}: {stderr}");
    }
}

/// A many-hearth listing refuses a hearth whose journal is damaged in the
/// one sentence and lists the healthy hearth beside it.
#[test]
fn a_damaged_hearth_is_refused_beside_a_healthy_one() {
    let dir = tempfile::tempdir().unwrap();
    for realm in ["alpha", "beta"] {
        std::fs::create_dir(dir.path().join(realm)).unwrap();
    }
    written(&dir.path().join("alpha.db"), &["good"]);
    damaged(dir.path(), "beta", |db| overwrite(db, 0, b"not a journal"));
    let realms = json!({
        "schema": "forge.realms/v2",
        "realms": [
            {"name": "alpha", "path": "alpha", "default_branch": "main",
             "journal": "alpha.db"},
            {"name": "beta", "path": "beta", "default_branch": "main",
             "journal": "beta.db"},
        ],
        "journal": "alpha.db",
    });
    std::fs::write(dir.path().join("realms.json"), realms.to_string()).unwrap();
    let (stdout, stderr, ok) = brokkr(dir.path(), &["runs", "--realms", "realms.json"]);
    assert_eq!((stderr.as_str(), ok), ("", true));
    let (alpha, beta) = stdout.split_once("\n\n").unwrap();
    assert!(
        alpha.starts_with("alpha · 1 run · 0 want a ruling · 0 want an action · alpha.db\n")
            && alpha.contains("\n  good running work seq 2 "),
        "{stdout}"
    );
    let words = refusal(Path::new("beta.db"), NO_DATABASE);
    assert_eq!(
        beta,
        format!(
            "beta · 0 runs · 0 want a ruling · 0 want an action · beta.db\n  journal  {words}\n"
        )
    );
}
