//! `brokkr backup` and `brokkr restore` through the binary (decision 0100,
//! the journal half of #431): the two verbs are reachable as the operator
//! types them, each prints its document and its one stderr line, and a
//! backup the store refuses fails the command with the refusal's words.

use brokkr_core::envelope::EventType;
use brokkr_store::test_support::append;
use brokkr_store::Store;
use serde_json::{json, Value};
use std::path::Path;

use crate::boundary_readouts::brokkr;

/// A journal at `dir/forge.db` holding one run with one event.
fn one_run_journal(dir: &Path) {
    let mut store = Store::open(&dir.join("forge.db")).unwrap();
    store
        .create_run("alpha", "feature", "test", &json!({"files": {}}))
        .unwrap();
    append(
        &mut store,
        "alpha",
        EventType::PhaseEntered,
        json!({"phase": "work"}),
    );
}

#[test]
fn backup_and_restore_run_as_typed_and_a_refused_backup_fails_in_its_words() {
    let dir = tempfile::tempdir().unwrap();
    one_run_journal(dir.path());
    let path = |name: &str| dir.path().join(name).to_str().unwrap().to_owned();

    let (stdout, stderr, ok) = brokkr(
        dir.path(),
        &["backup", "--to", &path("backup"), "--db", &path("forge.db")],
    );
    assert!(ok, "{stderr}");
    let document: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(document["boundary"], json!({"runs": 1, "events": 1}));
    assert!(stderr.contains("(1 runs, 1 events)"), "{stderr}");

    let (stdout, stderr, ok) = brokkr(
        dir.path(),
        &[
            "restore",
            "--from",
            &path("backup"),
            "--to",
            &path("restored.db"),
        ],
    );
    assert!(ok, "{stderr}");
    let document: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        document,
        json!({"restored": path("restored.db"), "runs": 1, "events": 1})
    );
    assert!(stderr.contains("nothing was started"), "{stderr}");

    std::fs::write(dir.path().join("a-file"), "operator bytes").unwrap();
    let (_, stderr, ok) = brokkr(
        dir.path(),
        &["backup", "--to", &path("a-file"), "--db", &path("forge.db")],
    );
    assert!(!ok, "a backup into a file succeeded");
    assert!(
        stderr.contains("exists and is not a directory; nothing was written"),
        "{stderr}"
    );
}

/// A path beginning `file:` is the literal path it spells, in both verbs
/// (decision 0100 ruling 3; landing 431L4's H1). The bundled SQLite reads
/// such a name as a URI, so before each verb bound its paths, a backup to
/// `file:victim?unused=1` wrote its snapshot over `victim`, and one to
/// `file:escaped` left a stray in `escaped/`.
#[test]
fn a_file_colon_path_is_the_literal_path_it_spells_in_both_verbs() {
    let dir = tempfile::tempdir().unwrap();
    one_run_journal(dir.path());
    std::fs::write(dir.path().join("victim"), "").unwrap();
    std::fs::create_dir(dir.path().join("escaped")).unwrap();

    let (_, stderr, ok) = brokkr(
        dir.path(),
        &["backup", "--to", "file:victim?unused=1", "--db", "forge.db"],
    );
    assert!(ok, "{stderr}");
    let victim = std::fs::metadata(dir.path().join("victim")).unwrap();
    assert_eq!(victim.len(), 0, "the file the URI would name is untouched");
    let spelled = dir.path().join("file:victim?unused=1");
    assert!(spelled.join("journal.db").is_file(), "{stderr}");
    assert!(spelled.join("inventory.json").is_file(), "{stderr}");

    let (_, stderr, ok) = brokkr(
        dir.path(),
        &["backup", "--to", "file:escaped", "--db", "forge.db"],
    );
    assert!(ok, "{stderr}");
    let strays = std::fs::read_dir(dir.path().join("escaped")).unwrap();
    assert_eq!(
        strays.count(),
        0,
        "nothing is written outside the destination"
    );
    assert!(dir.path().join("file:escaped/journal.db").is_file());

    let (_, stderr, ok) = brokkr(
        dir.path(),
        &[
            "restore",
            "--from",
            "file:escaped",
            "--to",
            "file:rest/journal.db",
        ],
    );
    assert!(ok, "{stderr}");
    assert!(
        dir.path().join("file:rest/journal.db").is_file(),
        "{stderr}"
    );
    assert!(!dir.path().join("rest").exists(), "{stderr}");
}
