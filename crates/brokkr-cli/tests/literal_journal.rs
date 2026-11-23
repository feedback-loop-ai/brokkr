//! A `--db` beginning `file:` is the literal file it spells, on every open
//! of a journal (#954; the #431 follow-up to decision 0100 ruling 3). The
//! bundled SQLite reads such a name as a URI, so before the store bound
//! every path it opens, `--db 'file:other.db?mode=rwc'` created and wrote
//! `other.db`, and `--db 'file:victim?x=1'` opened `victim`.

use std::path::Path;

use brokkr_store::test_support::damaged::written;
use brokkr_store::{Attribution, NewEntry, Store};

use crate::boundary_readouts::brokkr;

const RWC: &str = "file:other.db?mode=rwc";
const VICTIM: &str = "file:victim?x=1";

/// `args` through the binary in `dir` succeeds, its stderr shown when it
/// does not, and its stdout is returned.
fn ran(dir: &Path, args: &[&str]) -> String {
    let (stdout, stderr, ok) = brokkr(dir, args);
    assert!(ok, "{args:?}: {stderr}");
    stdout
}

/// `args` through the binary in `dir` fails, and its stderr is returned.
fn refused(dir: &Path, args: &[&str]) -> String {
    let (_, stderr, ok) = brokkr(dir, args);
    assert!(!ok, "{args:?} succeeded");
    stderr
}

/// The journal `db` in `dir` holding the run `good` and one queued entry
/// whose payload no brokkr reads, so a listing of it refuses by name where
/// an empty journal lists `[]`.
fn seeded(dir: &Path, db: &str) {
    written(&dir.join(db), &["good"]);
    let entry = NewEntry {
        payload: "{}",
        priority: 0,
        waits: &[],
    };
    let by = Attribution {
        operator: "o",
        reason: "r",
    };
    let mut store = Store::open(&dir.join(db)).unwrap();
    store.queue_add(entry, by).unwrap();
}

/// Every open path, given the URI-shaped `db` in the Brokkr folder `dir`,
/// reads and writes the literal journal [`seeded`] there: a resume opens
/// it for writing, the run history, the queue and the dispatcher read it,
/// the dispatcher's bound is written into it and read back, and an export
/// of it is imported into the URI-shaped `into`, whose history then holds
/// the run.
fn every_open_is_literal(dir: &Path, db: &str, into: &str) {
    std::fs::create_dir_all(dir.join(".brokkr")).unwrap();
    seeded(dir, db);
    assert_eq!(
        refused(dir, &["resume", "--run", "absent", "--db", db]),
        "error: no run matching 'absent' in this workspace database\n"
    );
    let listed = ran(dir, &["runs", "--db", db]);
    assert!(listed.contains("\n  good running work seq 2 "), "{listed}");
    assert_eq!(
        refused(dir, &["queue", "list", "--json", "--db", db]),
        "error: queue entry 1: reading a queued launch: missing field `encoding` at line 1 \
         column 2\n"
    );
    let bound = ["queue", "dispatch", "set-bound", "3", "--reason", "probe"];
    ran(dir, &[&bound[..], &["--db", db]].concat());
    assert_eq!(
        ran(dir, &["queue", "dispatch", "status", "--db", db]),
        "not held; no dispatcher has taken this journal\nbound: 3 counted refusals hold an entry\n"
    );
    ran(
        dir,
        &[
            "journal", "export", "--run", "good", "--out", "out", "--db", db,
        ],
    );
    ran(
        dir,
        &[
            "journal",
            "import",
            "--from",
            "out/good.ndjson",
            "--db",
            into,
        ],
    );
    let listed = ran(dir, &["runs", "--db", into]);
    assert!(listed.contains("\n  good running work seq 2 "), "{listed}");
}

/// A `--db` whose URI would create a file writes and reads only the
/// literal journals of the names given, and no file the URIs name is ever
/// created.
#[test]
fn a_uri_shaped_db_is_the_literal_journal_on_every_open() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    every_open_is_literal(here, RWC, "file:imported.db?mode=rwc");
    for uri in ["other.db", "imported.db"] {
        assert!(!here.join(uri).exists(), "the URI's {uri} was created");
    }
}

/// A `--db` that names an operator's file through a URI's query reads and
/// writes the literal journals of the names given on every open, and
/// `victim` keeps its bytes.
#[test]
fn a_uri_shaped_db_never_opens_the_file_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    std::fs::write(here.join("victim"), "operator bytes").unwrap();
    every_open_is_literal(here, VICTIM, "file:victim?y=2");
    assert!(here.join("file:victim?y=2").is_file(), "no literal import");
    assert_eq!(
        std::fs::read(here.join("victim")).unwrap(),
        b"operator bytes"
    );
}

/// A `--db` of `:memory:`, which SQLite opened as a private database that
/// vanished with the process, is the literal journal of that name (#954):
/// a resume creates the file, and a later read finds it.
#[test]
fn a_memory_db_is_the_literal_journal_of_that_name() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    std::fs::create_dir_all(here.join(".brokkr")).unwrap();
    assert_eq!(
        refused(here, &["resume", "--run", "absent", "--db", ":memory:"]),
        "error: no run matching 'absent' in this workspace database\n"
    );
    assert!(here.join(":memory:").is_file(), "no literal journal");
    assert_eq!(
        ran(here, &["runs", "--db", ":memory:"]),
        "world · 0 runs · 0 want a ruling · 0 want an action · :memory:\n"
    );
}

/// A journal the writer cannot open is refused naming the path as the
/// operator gave it, not the absolute path SQLite was handed.
#[test]
fn a_journal_that_cannot_be_opened_is_named_as_given() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    std::fs::create_dir_all(here.join(".brokkr")).unwrap();
    std::fs::write(here.join("blocked"), "").unwrap();
    assert_eq!(
        refused(
            here,
            &["resume", "--run", "absent", "--db", "blocked/journal.db"]
        ),
        "error: sqlite: unable to open database file: blocked/journal.db: unable to open \
         database file: blocked/journal.db: Error code 14: unable to open database file\n"
    );
}
