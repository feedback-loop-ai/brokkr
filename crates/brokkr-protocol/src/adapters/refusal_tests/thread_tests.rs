//! The record's own id (F4, R1): the header's id is this record's own, so
//! an announced thread is compared exactly and, without one, the id is the
//! full id the codex file name ends with. A fixed word or any fragment of
//! the real id correlates nothing, and the lost-result path stands.

use super::*;

/// F4: an empty id, an id the file name does not carry, and every fragment
/// of the real id correlate nothing. The zero-stdout fake writes its work
/// marker, exits 0 and leaves no result, so each case is driven on both
/// paths and must send no result and no refusal.
#[cfg(unix)]
#[test]
fn review_chief_1037f_record_header_id_must_be_the_records_own() {
    for (label, id) in [
        ("an empty id is no record's own", ""),
        (
            "an id the file name does not carry is no record's own",
            "not-this-thread",
        ),
        ("the fixed word before the id is no record's own", "rollout"),
        ("the leading group is no record's own", "01a0619c"),
        ("the trailing group is no record's own", "9eaa94c3aec1"),
        ("a middle fragment is no record's own", "928b-7ad3"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut header = correlated_header(dir.path());
        header["payload"]["id"] = json!(id);
        let [started, complete] = started_and_quota_complete();
        let rows = [header, started, complete];
        assert_lost_on_both_paths(dir.path(), &rows, label);
    }
}

/// F4's name reader: a name carries an id only in codex's full 8-4-4-4-12
/// lowercase `rollout-<timestamp>-<id>.jsonl` shape. A name that is not a
/// rollout `.jsonl`, whose trailing id is not that shape (a bad character
/// or a bad hyphen position), or whose trailing 36 bytes split a character
/// carries no id, even when the header spells those characters, so the
/// record correlates nothing.
#[test]
fn a_name_that_does_not_carry_a_full_codex_id_correlates_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let cwd = dir.path().to_string_lossy();
    let mut bad_shape = THREAD.to_string();
    bad_shape.pop();
    bad_shape.push('z');
    let bad_hyphen = THREAD.replacen('-', "0", 1);
    let split = format!("é{}", "a".repeat(35));
    for (name, id) in [
        (format!("rollout-{bad_shape}.jsonl"), bad_shape.as_str()),
        (format!("rollout-{bad_hyphen}.jsonl"), bad_hyphen.as_str()),
        (format!("{THREAD}.jsonl"), THREAD),
        (format!("rollout-{THREAD}"), THREAD),
        (format!("rollout-{split}.jsonl"), split.as_str()),
    ] {
        let path = dir.path().join(&name);
        let header = json!({"type": "session_meta",
            "payload": {"id": id, "timestamp": "9999-12-31T00:00:00Z", "cwd": cwd}});
        std::fs::write(&path, record(&[header])).unwrap();
        assert!(
            thread_header(&path, None).is_none(),
            "a name that does not carry a full codex id carries no id: {name}"
        );
    }
}
