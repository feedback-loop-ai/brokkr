//! Decision 0113 ruling 4 through the binary: every old flat spelling is
//! its new one, byte for byte on stdout and in its exit code, with one
//! line on stderr before anything else, and no verb that did not move
//! prints that line. Every command runs in a scratch directory against a
//! scratch journal, so no folder or journal of this checkout is opened.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Each moved root, by its old name, and its new spelling (the
/// decision's table).
const MOVED: [(&str, &str); 30] = [
    ("inspect", "runs inspect"),
    ("seats", "runs seats"),
    ("transcript", "runs transcript"),
    ("compare", "runs compare"),
    ("replay", "runs replay"),
    ("rerun", "runs rerun"),
    ("conclude", "runs conclude"),
    ("operator", "runs operator"),
    ("muninn", "runs muninn"),
    ("export", "journal export"),
    ("import", "journal import"),
    ("verify-run", "journal verify"),
    ("backup", "journal backup"),
    ("restore", "journal restore"),
    ("anchor", "journal anchor"),
    ("keep-refs", "journal keep-refs"),
    ("evidence", "journal evidence"),
    ("recipes", "library recipes"),
    ("agents", "library agents"),
    ("compile", "library compile"),
    ("dispatch", "queue dispatch"),
    ("sweep", "queue sweep"),
    ("analytics", "stats analytics"),
    ("graph", "stats graph"),
    ("costs", "stats costs"),
    ("ledger", "stats ledger"),
    ("realms", "system realms"),
    ("secrets", "system secrets"),
    ("probe", "system probe"),
    ("ui", "system ui"),
];

/// A real exported run, with its manifest beside it, as `import` requires.
const RUN: &str = "tui-graph-the-selection-box-gets-80f98deb";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/journals")
        .join(format!("{RUN}.ndjson"))
}

fn brokkr(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brokkr"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("brokkr starts")
}

fn note(old: &str, new: &str) -> String {
    format!("brokkr: '{old}' is now 'brokkr {new}'; the old spelling goes in 0.17\n")
}

/// `old` (the alias spelling, its arguments after it) against `new`: the
/// same stdout and exit code, and the note and then `new`'s own stderr.
fn same(dir: &Path, old: &[&str], new: &[&str], said: &str) {
    let (alias, moved) = (brokkr(dir, old), brokkr(dir, new));
    assert_eq!(alias.stdout, moved.stdout, "{old:?} against {new:?}");
    assert_eq!(alias.status.code(), moved.status.code(), "{old:?}");
    let expected = format!("{said}{}", String::from_utf8_lossy(&moved.stderr));
    assert_eq!(String::from_utf8_lossy(&alias.stderr), expected, "{old:?}");
}

#[test]
fn every_old_spelling_prints_its_new_ones_help_after_one_line_on_stderr() {
    let dir = tempfile::tempdir().unwrap();
    for (old, new) in MOVED {
        let path: Vec<&str> = new.split(' ').collect();
        let asked = [path.as_slice(), &["--help"]].concat();
        same(dir.path(), &[old, "--help"], &asked, &note(old, new));
        let help = brokkr(dir.path(), &asked);
        assert_eq!(help.status.code(), Some(0), "{new}");
        let usage = String::from_utf8_lossy(&help.stdout).into_owned();
        assert!(usage.contains(&format!("Usage: brokkr {new}")), "{usage}");
    }
}

#[test]
fn an_old_spelling_reads_a_fixture_run_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("journal.db");
    let (db, from) = (db.to_str().unwrap(), fixture());
    let from = from.to_str().unwrap();
    let imported = brokkr(
        dir.path(),
        &["journal", "import", "--db", db, "--from", from],
    );
    assert_eq!(imported.status.code(), Some(0), "{imported:?}");
    let missing = "no-such-run";
    let pairs: [(&str, &[&str], &[&str]); 9] = [
        (
            "inspect",
            &["--run", RUN, "--db", db, "--json"],
            &["runs", "inspect"],
        ),
        (
            "inspect",
            &["--run", missing, "--db", db],
            &["runs", "inspect"],
        ),
        (
            "seats",
            &["--run", RUN, "--db", db, "--json"],
            &["runs", "seats"],
        ),
        ("replay", &["--run", RUN, "--db", db], &["runs", "replay"]),
        ("compare", &[RUN, RUN, "--db", db], &["runs", "compare"]),
        ("costs", &["--run", RUN, "--db", db], &["stats", "costs"]),
        ("ledger", &["--run", RUN, "--db", db], &["stats", "ledger"]),
        ("verify-run", &[from], &["journal", "verify"]),
        ("realms", &["--db", db, "--json"], &["system", "realms"]),
    ];
    let mut codes = Vec::new();
    for (old, args, path) in pairs {
        let old_line = [&[old], args].concat();
        let new_line = [path, args].concat();
        same(
            dir.path(),
            &old_line,
            &new_line,
            &note(old, &path.join(" ")),
        );
        codes.push(brokkr(dir.path(), &new_line).status.code());
    }
    // The missing run, and a world with no map, are each refused alike.
    let refused = [1, 8].map(|at| codes[at]);
    assert_eq!(refused, [Some(1); 2], "{codes:?}");
    assert_eq!(codes.iter().filter(|code| **code == Some(0)).count(), 7);
}

#[test]
fn a_verb_that_did_not_move_prints_no_note() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("journal.db");
    let db = db.to_str().unwrap();
    for line in [
        &["runs", "--db", db, "--json"][..],
        &["runs", "list", "--db", db, "--json"],
        &["driver", "no-such-kind"],
        &["hands", "--help"],
        &["bridge", "--help"],
        &["init", "--help"],
        &["queue", "list", "--db", db],
    ] {
        let said = String::from_utf8_lossy(&brokkr(dir.path(), line).stderr).into_owned();
        assert!(!said.contains("is now 'brokkr"), "{line:?}: {said}");
    }
    // A bare `runs` still lists runs, as `runs list` does.
    let bare = brokkr(dir.path(), &["runs", "--db", db, "--json"]);
    let listed = brokkr(dir.path(), &["runs", "list", "--db", db, "--json"]);
    assert_eq!((bare.stdout, bare.status), (listed.stdout, listed.status));
}
