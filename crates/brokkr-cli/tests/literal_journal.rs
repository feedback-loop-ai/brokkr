//! A `--db` beginning `file:` is the literal file it spells, on every open
//! of a journal (#954; the #431 follow-up to decision 0100 ruling 3). The
//! bundled SQLite reads such a name as a URI, so before the store bound
//! every path it opens, `--db 'file:other.db?mode=rwc'` created and wrote
//! `other.db`, and `--db 'file:victim?x=1'` opened `victim`.

use std::path::Path;

use brokkr_store::test_support::damaged::written;

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

/// `brokkr resume` of a run `db` does not hold, in a Brokkr folder `dir`,
/// refuses after opening `db` for writing.
fn resume_refused(dir: &Path, db: &str) {
    std::fs::create_dir_all(dir.join(".brokkr")).unwrap();
    let (_, stderr, ok) = brokkr(dir, &["resume", "--run", "absent", "--db", db]);
    assert_eq!(
        (stderr.as_str(), ok),
        (
            "error: no run matching 'absent' in this workspace database\n",
            false
        )
    );
}

/// A resume opens the literal journal of that name for writing, and the
/// run-history reads, the queue's and the dispatcher's, and an export read
/// it; the dispatcher's bound and an import write into it, and no file
/// the URI names is ever created.
#[test]
fn a_uri_shaped_db_is_the_literal_journal_on_every_open() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    written(&here.join(RWC), &["good"]);
    resume_refused(here, RWC);
    assert!(
        !here.join("other.db").exists(),
        "the resume created other.db"
    );
    let listed = ran(here, &["runs", "--db", RWC]);
    assert!(listed.contains("\n  good running work seq 2 "), "{listed}");
    assert_eq!(ran(here, &["queue", "list", "--json", "--db", RWC]), "[]\n");
    ran(here, &["dispatch", "status", "--db", RWC]);
    ran(
        here,
        &["export", "--run", "good", "--out", "out", "--db", RWC],
    );
    let bound = [
        "dispatch",
        "set-bound",
        "3",
        "--reason",
        "probe",
        "--db",
        RWC,
    ];
    ran(here, &bound);
    let into = "file:imported.db?mode=rwc";
    ran(here, &["import", "--from", "out/good.ndjson", "--db", into]);
    let listed = ran(here, &["runs", "--db", into]);
    assert!(listed.contains("\n  good running work seq 2 "), "{listed}");
    for uri in ["other.db", "imported.db"] {
        assert!(!here.join(uri).exists(), "the URI's {uri} was created");
    }
}

/// A `--db` that names an operator's file through a URI's query writes
/// the literal journal of that name, and `victim` keeps its bytes.
#[test]
fn a_uri_shaped_db_never_opens_the_file_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let here = dir.path();
    std::fs::write(here.join("victim"), "operator bytes").unwrap();
    resume_refused(here, VICTIM);
    assert!(here.join(VICTIM).is_file(), "the literal journal is absent");
    let bound = [
        "dispatch",
        "set-bound",
        "3",
        "--reason",
        "probe",
        "--db",
        VICTIM,
    ];
    ran(here, &bound);
    ran(here, &["dispatch", "status", "--db", VICTIM]);
    assert_eq!(
        std::fs::read(here.join("victim")).unwrap(),
        b"operator bytes"
    );
}
