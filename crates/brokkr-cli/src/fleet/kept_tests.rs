//! The kept fleet read's proofs (#744): a refresh pays for what landed
//! since the last one, its rows are the whole read's rows byte for byte,
//! and a run it cannot carry is read whole.

use std::path::Path;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::test_support::{plant_envelope, plant_run};
use brokkr_store::{Attribution, Store};
use serde_json::{json, Value};

use super::kept::rules;
use super::{read_hearth, read_journal, read_kept, Journal, KeptFleet, ListedRun};
use crate::tests::envelope_builder::EnvelopeBuilder;
use crate::tests::{broken_chain_store, poisoned_store, running_store};

/// The fleet as the whole read derives it, as `brokkr runs --json` prints
/// it.
fn whole_rows(db: &Path) -> String {
    let store = Store::open_read_only(db).unwrap();
    let listed = read_hearth(&store).listed().unwrap();
    let entries: Vec<_> = listed.iter().map(ListedRun::entry).collect();
    serde_json::to_string_pretty(&brokkr_view::run_rows(&entries)).unwrap()
}

/// The fleet as `kept` derives it.
fn kept_rows(kept: &KeptFleet) -> String {
    serde_json::to_string_pretty(&brokkr_view::run_rows(&kept.entries())).unwrap()
}

/// One refresh of `kept` over a fresh read-only connection, as the
/// console opens one each refresh: how many events it read.
fn refreshed(kept: &mut KeptFleet, db: &Path) -> usize {
    let before = Journal::at(db);
    kept.refresh(&Store::open_read_only(db).unwrap(), before)
        .unwrap()
}

/// Append `event_type` with `payload` to `run_id` as the engine would.
fn append(db: &Path, run_id: &str, event_type: EventType, payload: Value) -> EventEnvelope {
    Store::open(db)
        .unwrap()
        .append_next(run_id, event_type, payload, None, None)
        .unwrap()
}

/// The events `run_id`'s journal holds.
fn events_of(db: &Path, run_id: &str) -> usize {
    Store::open_read_only(db)
        .unwrap()
        .load(run_id)
        .unwrap()
        .len()
}

/// A stopped run whose review ruled a security residual, at seq 6.
fn held(db: &Path, run_id: &str) {
    Store::open(db)
        .unwrap()
        .create_run(run_id, "close the finding by name", "test", &json!({}))
        .unwrap();
    for (event_type, payload) in [
        (
            EventType::RunStarted,
            json!({"feature": "held", "manifest": {}}),
        ),
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "review", "phase": "review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff", "attempt_id": "att"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "eff", "attempt_id": "att", "result": {"result": "residual"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "review", "result": "residual", "rule_id": "REVIEW-SECURITY-HOLD",
                   "next": null, "severity": null, "problem": null,
                   "inputs": {"has_security_residual": true}}),
        ),
        (
            EventType::RunStopped,
            json!({"reason": "security residual"}),
        ),
    ] {
        append(db, run_id, event_type, payload);
    }
}

/// The operator's word that finding `seq` of a run is closed.
fn supersede(seq: u64) -> Value {
    json!({"command_id": "cmd-s", "command": "supersede", "operator": "op",
           "args": {"findings": [seq], "by": {"run_id": "later", "seq": 9},
                    "reason": "fixed and shipped"}})
}

const STAMP: &str = "2026-10-01T00:00:00Z";

/// Run `run_id`'s journal chained as the store chains it: each event
/// sealed onto the one before.
fn chain(run_id: &str, events: impl Iterator<Item = (EventType, Value)>) -> Vec<EventEnvelope> {
    let mut chained: Vec<EventEnvelope> = Vec::new();
    for (seq, (event_type, payload)) in (1..).zip(events) {
        let previous = chained
            .last()
            .map_or(ZERO_HASH.to_string(), |event| event.event_hash.clone());
        chained.push(
            EnvelopeBuilder::new(event_type, payload)
                .run(run_id)
                .seq(seq)
                .event_id(format!("{run_id}-{seq}"))
                .at(STAMP)
                .previous(previous)
                .sealed(),
        );
    }
    chained
}

/// The runs of the large journal, and each one's checkpoints beside the
/// four events that open its effect.
const RUNS: usize = 100;
const CHECKPOINTS: usize = 4_996;

/// One run of the large journal: an effect in flight, checkpointing.
fn checkpointing(run_id: &str) -> Vec<EventEnvelope> {
    let opening = [
        (EventType::RunStarted, json!({"feature": run_id})),
        (EventType::PhaseEntered, json!({"phase": "work"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "eff", "seat": "work", "phase": "work"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "eff", "attempt_id": "att"}),
        ),
    ];
    let checkpoints = std::iter::repeat_n(
        (EventType::EffectCheckpointed, json!({"effect_id": "eff"})),
        CHECKPOINTS,
    );
    chain(run_id, opening.into_iter().chain(checkpoints))
}

/// A journal of half a million events, nearly all of them checkpoints,
/// planted as the store's own writers would leave it. Sealed across
/// threads: the generation is the test's cost, not the read's.
fn large_journal(db: &Path) -> (usize, usize) {
    drop(Store::open(db).unwrap());
    let runs: Vec<String> = (0..RUNS).map(|run| format!("run-{run:03}")).collect();
    let journals: Vec<Vec<EventEnvelope>> = std::thread::scope(|scope| {
        let sealers: Vec<_> = runs
            .chunks(RUNS / 10)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|run| checkpointing(run))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        sealers
            .into_iter()
            .flat_map(|sealer| sealer.join().unwrap())
            .collect()
    });
    let mut conn = rusqlite::Connection::open(db).unwrap();
    let planting = conn.transaction().unwrap();
    let (mut events, mut checkpoints) = (0, 0);
    for (run, journal) in runs.iter().zip(&journals) {
        plant_run(&planting, run, "{}").unwrap();
        for event in journal {
            plant_envelope(&planting, event).unwrap();
            events += 1;
            checkpoints += usize::from(event.event_type == EventType::EffectCheckpointed);
        }
    }
    planting.commit().unwrap();
    (events, checkpoints)
}

/// The acceptance (#744): on a journal of half a million events, 99.9%
/// of them checkpoints, the first refresh reads every event once, a
/// refresh with nothing new reads none, and one after three appends reads
/// those three — the cost grows with what landed, not with the journal.
/// Work is counted as events deserialized, never timed, so a loaded CI
/// host cannot flake it. Forcing every run to be read whole again (a
/// `carried` that returns `None`) fails the second refresh with 500000.
#[test]
fn a_refresh_reads_what_landed_since_the_last_one_and_not_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let (events, checkpoints) = large_journal(&db);
    assert_eq!((events, checkpoints), (500_000, 499_600));
    assert!(checkpoints * 100 >= events * 95, "at least 95% checkpoints");

    let mut kept = KeptFleet::default();
    assert_eq!(
        refreshed(&mut kept, &db),
        500_000,
        "the first read is whole"
    );
    assert_eq!(refreshed(&mut kept, &db), 0, "nothing landed, nothing read");

    let mut last = None;
    for _ in 0..3 {
        last = Some(append(
            &db,
            "run-042",
            EventType::EffectCheckpointed,
            json!({"effect_id": "eff"}),
        ));
    }
    assert_eq!(refreshed(&mut kept, &db), 3, "three landed, three read");
    assert_eq!(refreshed(&mut kept, &db), 0);
    let entries = kept.entries();
    let moved = entries
        .iter()
        .find(|entry| entry.run_id == "run-042")
        .unwrap();
    assert_eq!(
        moved.last_recorded_at,
        Some(last.unwrap().recorded_at.as_str()),
        "a checkpoint dates its run"
    );
    let resting = entries
        .iter()
        .find(|entry| entry.run_id == "run-041")
        .unwrap();
    assert_eq!(resting.last_recorded_at, Some(STAMP));
    assert_eq!(moved.state.unwrap().seq, 5_003);
}

/// The kept rows are the whole read's rows, byte for byte, whatever the
/// journal does between refreshes: a residual superseded after it was
/// read, a running run moving on, a quarantined run whose journal moved
/// and one whose journal did not but whose acknowledgement arrived, an
/// empty run's first event, and a fold refused in what landed.
#[test]
fn the_kept_rows_are_the_whole_reads_rows_as_the_journal_moves() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    held(&db, "held");
    running_store(&db, "working");
    poisoned_store(&db, "poisoned");
    broken_chain_store(&db, "broken");
    Store::open(&db)
        .unwrap()
        .create_run("empty", "nothing yet", "test", &json!({}))
        .unwrap();
    let poisoned = events_of(&db, "poisoned");

    let mut kept = KeptFleet::default();
    assert_eq!(refreshed(&mut kept, &db), 7 + 2 + poisoned);
    assert_eq!(kept_rows(&kept), whole_rows(&db));

    append(&db, "held", EventType::OperatorCommanded, supersede(6));
    append(
        &db,
        "working",
        EventType::EffectRequested,
        json!({"effect_id": "eff", "seat": "work", "phase": "work"}),
    );
    append(
        &db,
        "working",
        EventType::EffectStarted,
        json!({"effect_id": "eff", "attempt_id": "att"}),
    );
    append(
        &db,
        "working",
        EventType::EffectCheckpointed,
        json!({"effect_id": "eff"}),
    );
    append(
        &db,
        "poisoned",
        EventType::PhaseEntered,
        json!({"phase": "on"}),
    );
    append(
        &db,
        "empty",
        EventType::RunStarted,
        json!({"feature": "now"}),
    );
    let mut store = Store::open(&db).unwrap();
    let (seq, hash) = store.head_hash("broken").unwrap();
    let by = Attribution {
        operator: "op",
        reason: "read it, leaving it",
    };
    store
        .acknowledge_quarantine("broken", (seq, &hash), by)
        .unwrap();
    drop(store);
    // The suffixes; the moved quarantine read whole; the empty run's
    // first event; and nothing at all for the unmoved quarantine.
    assert_eq!(refreshed(&mut kept, &db), 1 + 3 + (poisoned + 1) + 1);
    assert_eq!(kept_rows(&kept), whole_rows(&db));
    let entries = kept.entries();
    let row = |run_id| entries.iter().find(|entry| entry.run_id == run_id).unwrap();
    let mark = row("held").residuals[0]
        .superseded
        .as_ref()
        .map(|mark| mark.seq);
    assert_eq!(
        mark,
        Some(8),
        "a finding read earlier, marked by what landed"
    );
    let acknowledged = row("broken")
        .detail
        .unwrap()
        .acknowledgement
        .map(|ack| ack.seq);
    assert_eq!(acknowledged, Some(seq), "acknowledged where it stood");

    // An event after the run stopped: the fold refuses it in the suffix,
    // and the run is quarantined where the whole read quarantines it.
    append(
        &db,
        "held",
        EventType::EffectRequested,
        json!({"effect_id": "late", "seat": "review", "phase": "review"}),
    );
    assert_eq!(refreshed(&mut kept, &db), 1);
    assert_eq!(kept_rows(&kept), whole_rows(&db));
    assert_eq!(refreshed(&mut kept, &db), 0, "refused, and unmoved");
    assert_eq!(kept_rows(&kept), whole_rows(&db));
}

/// A fleet kept from a journal under `dir` that holds one running run,
/// `r1`, read once: where the journal is, and the fleet.
fn kept_running(dir: &Path) -> (std::path::PathBuf, KeptFleet) {
    let db = dir.join("forge.db");
    running_store(&db, "r1");
    let mut kept = KeptFleet::default();
    assert_eq!(refreshed(&mut kept, &db), 2);
    (db, kept)
}

/// The next refresh of `kept` over `db` reads its one run whole and is
/// refused in `refusal`'s words, its rows the whole read's. `case` names
/// the case.
fn read_whole_and_refused(kept: &mut KeptFleet, db: &Path, refusal: &str, case: &str) {
    assert_eq!(refreshed(kept, db), 0, "{case}: a whole read, refused");
    let entries = kept.entries();
    let words = entries[0].detail.map(|refusal| refusal.words);
    assert_eq!((entries.len(), words), (1, Some(refusal)), "{case}");
    assert_eq!(kept_rows(kept), whole_rows(db), "{case}");
}

/// A journal replaced beneath the reader holds a different chain for the
/// same run: the head the kept fold stands at is not its row any more,
/// so the run is read whole — three events, not the one past the old
/// head — and shown as the new journal says.
#[test]
fn a_run_whose_head_the_journal_no_longer_holds_is_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut kept) = kept_running(dir.path());

    for file in ["forge.db", "forge.db-wal", "forge.db-shm"] {
        let _ = std::fs::remove_file(dir.path().join(file));
    }
    Store::open(&db)
        .unwrap()
        .create_run("r1", "replaced", "test", &json!({}))
        .unwrap();
    append(
        &db,
        "r1",
        EventType::RunStarted,
        json!({"feature": "replaced"}),
    );
    append(
        &db,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "other"}),
    );
    append(
        &db,
        "r1",
        EventType::PhaseEntered,
        json!({"phase": "again"}),
    );
    assert_eq!(refreshed(&mut kept, &db), 3, "read whole");
    assert_eq!(kept_rows(&kept), whole_rows(&db));
}

/// What the whole read says of a run whose first event is gone.
pub(crate) const FIRST_EVENT_GONE: &str = "chain: event 2: seq is not contiguous (expected 1)";

/// A journal in `dir` that opens, its schema recorded, but holds no
/// `runs` table: the store refuses to list its runs. Where it is.
pub(crate) fn unlisted(dir: &Path) -> std::path::PathBuf {
    let db = dir.join("forge.db");
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO meta (key, value) VALUES ('database_schema', '1');",
        )
        .unwrap();
    db
}

/// What the store says of a journal it cannot list the runs of.
pub(crate) const UNLISTED: &str = "sqlite: no such table: runs";

/// `brokkr runs` over a journal whose runs the store cannot list is the
/// verb's refusal, in the store's own words, as the console's frame is.
#[test]
fn runs_over_a_journal_whose_runs_cannot_be_listed_refuses_in_the_stores_words() {
    let dir = tempfile::tempdir().unwrap();
    let runs = crate::Cmd::Runs(crate::cli_args::RunsArgs {
        realms: None,
        db: Some(unlisted(dir.path())),
        json: true,
    });
    let refused = crate::run(crate::tests::cli(runs)).unwrap_err();
    assert_eq!(refused.to_string(), UNLISTED);
}

/// `db`'s journal, its connections all closed, copied beside it with
/// `run_id`'s first event gone and every other row as it was: the run's
/// head, seq and hash both, is still the copy's. Where the copy is.
pub(crate) fn without_its_first_event(db: &Path, run_id: &str) -> std::path::PathBuf {
    copied_past(
        db,
        "events_append_only_delete",
        &format!("DELETE FROM events WHERE run_id = '{run_id}' AND seq = 1"),
    )
}

/// What the whole read says of a run whose first event was rewritten.
const FIRST_EVENT_REWRITTEN: &str = "chain: event 1: event_hash does not match canonical content";

/// [`without_its_first_event`], with the first event's time rewritten
/// rather than its row gone: every row count, and the head, as they were.
fn with_its_first_event_rewritten(db: &Path, run_id: &str) -> std::path::PathBuf {
    copied_past(
        db,
        "events_append_only_update",
        &format!(
            "UPDATE events SET envelope =
                 json_set(envelope, '$.recorded_at', '2000-01-01T00:00:00Z')
             WHERE run_id = '{run_id}' AND seq = 1"
        ),
    )
}

/// `db`'s journal copied beside it, with `trigger` dropped and `change`
/// made to exactly one row of the copy. Where the copy is.
fn copied_past(db: &Path, trigger: &str, change: &str) -> std::path::PathBuf {
    let copy = db.with_extension("copy");
    std::fs::copy(db, &copy).unwrap();
    let journal = rusqlite::Connection::open(&copy).unwrap();
    journal
        .execute_batch(&format!("DROP TRIGGER {trigger}"))
        .unwrap();
    assert_eq!(journal.execute(change, []).unwrap(), 1);
    copy
}

/// How a damaged journal is put where the reader's was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Swap {
    /// Moved into place: a new inode of the same length.
    Moved,
    /// Vacuumed, and copied over the old file in place: the same inode,
    /// grown shorter.
    Shorter,
    /// Copied over the old file in place unvacuumed, which keeps its
    /// length, and its modification time put back as `cp -p` puts it:
    /// the same inode, the same length and the same modification time,
    /// only the change time moved.
    InPlace,
}

/// Put `copy` where `db` is, as `swap` says.
pub(crate) fn swapped_in(copy: &Path, db: &Path, swap: Swap) {
    if swap == Swap::Shorter {
        let journal = rusqlite::Connection::open(copy).unwrap();
        journal.execute_batch("VACUUM").unwrap();
    }
    let modified = std::fs::metadata(db).unwrap().modified().unwrap();
    match swap {
        Swap::Moved => std::fs::rename(copy, db).unwrap(),
        Swap::Shorter => drop(std::fs::copy(copy, db).unwrap()),
        Swap::InPlace => {
            std::fs::copy(copy, db).unwrap();
            let file = std::fs::File::options().write(true).open(db).unwrap();
            file.set_modified(modified).unwrap();
        }
    }
}

/// A journal replaced beneath the reader by one that kept a run's head
/// and lost its first event, or had it rewritten, is read whole, and the
/// run shown quarantined as the whole read shows it, whichever way it was
/// put in place ([`Swap`]). The suffix read alone confirms the head and
/// carries the run on with nothing read, showing it folded at seq 2: so
/// does a `refresh` that skips the count of the rows below each kept head,
/// for the lost event copied over in place; one
/// that ignores the journal's identity, for the rewritten event moved into
/// place; and one that ignores its length, for the rewritten event grown
/// shorter. A rewritten event copied over in place at its length is not
/// told (decision 0088 ruling 2), and is not asked here.
#[test]
fn a_journal_replaced_beneath_the_reader_is_read_whole_though_it_holds_the_kept_head() {
    use std::os::unix::fs::MetadataExt;
    type Damage = fn(&Path, &str) -> std::path::PathBuf;
    let lost: Damage = without_its_first_event;
    let rewritten: Damage = with_its_first_event_rewritten;
    for (damage, swap, refusal) in [
        (lost, Swap::Moved, FIRST_EVENT_GONE),
        (lost, Swap::Shorter, FIRST_EVENT_GONE),
        (lost, Swap::InPlace, FIRST_EVENT_GONE),
        (rewritten, Swap::Moved, FIRST_EVENT_REWRITTEN),
        (rewritten, Swap::Shorter, FIRST_EVENT_REWRITTEN),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("forge.db");
        running_store(&db, "r1");
        let pad = json!({"pad": "x".repeat(1 << 16)});
        Store::open(&db)
            .unwrap()
            .create_run("padding", "a manifest to drop", "test", &pad)
            .unwrap();
        let mut kept = KeptFleet::default();
        assert_eq!(refreshed(&mut kept, &db), 2);
        let head = Store::open_read_only(&db).unwrap().head_hash("r1").unwrap();
        let before = std::fs::metadata(&db).unwrap();

        let copy = damage(&db, "r1");
        let journal = rusqlite::Connection::open(&copy).unwrap();
        journal
            .execute("DELETE FROM runs WHERE run_id = 'padding'", [])
            .unwrap();
        drop(journal);
        swapped_in(&copy, &db, swap);
        let after = std::fs::metadata(&db).unwrap();
        assert_eq!(
            after.ino() == before.ino(),
            swap != Swap::Moved,
            "the same file"
        );
        assert_eq!(
            after.len() < before.len(),
            swap == Swap::Shorter,
            "grown shorter"
        );
        if swap == Swap::InPlace {
            let modified = |file: &std::fs::Metadata| (file.mtime(), file.mtime_nsec());
            assert_eq!(modified(&after), modified(&before), "as cp -p leaves it");
        }
        let store = Store::open_read_only(&db).unwrap();
        assert_eq!(store.head_hash("r1").unwrap(), head, "the kept head stands");
        drop(store);

        let case = format!("{swap:?}: {refusal}");
        read_whole_and_refused(&mut kept, &db, refusal, &case);
    }
}

/// The journal file's length, and its modification and change times.
fn clocks(db: &Path) -> (u64, (i64, i64), (i64, i64)) {
    use std::os::unix::fs::MetadataExt;
    let file = std::fs::metadata(db).unwrap();
    let modified = (file.mtime(), file.mtime_nsec());
    (file.len(), modified, (file.ctime(), file.ctime_nsec()))
}

/// `run_id`'s first event deleted from `db` past the append fence, by a
/// read-write connection held open with the loss in its write-ahead log
/// and no checkpoint, as any connection under the 1000-page
/// auto-checkpoint leaves a small commit. The journal file itself is not
/// written: its length and both its times are as they were, and the log
/// holds the loss. The connection, to hold for as long as the loss must
/// stay in the log.
pub(crate) fn lost_in_its_log(db: &Path, run_id: &str) -> rusqlite::Connection {
    let before = clocks(db);
    let held = rusqlite::Connection::open(db).unwrap();
    held.execute_batch(
        "PRAGMA wal_autocheckpoint = 0;
         DROP TRIGGER events_append_only_delete;",
    )
    .unwrap();
    let loss = "DELETE FROM events WHERE run_id = ?1 AND seq = 1";
    assert_eq!(held.execute(loss, [run_id]).unwrap(), 1);
    assert_eq!(clocks(db), before, "the journal file is not written");
    let log = std::fs::metadata(db.with_extension("db-wal")).unwrap();
    assert!(log.len() > 0, "the loss is in the log");
    held
}

/// A row lost below a kept head through the store's own write path — a
/// commit into the write-ahead log of a connection held open, the journal
/// file untouched — is seen at the next refresh: the run is read whole and
/// shown quarantined as the whole read shows it. A `refresh` that counts
/// only when the journal file's length or times moved carries the run
/// folded at seq 2.
#[test]
fn a_row_lost_in_the_write_ahead_log_is_read_whole_at_the_next_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let (db, mut kept) = kept_running(dir.path());
    let _held = lost_in_its_log(&db, "r1");
    read_whole_and_refused(&mut kept, &db, FIRST_EVENT_GONE, "in the log");
}

/// A read is bound to the journal file only when the file stated before
/// its store opened is still the file once the read is done. A swap
/// between the two is planted both ways — the file stated before is a
/// copy beside the journal, and then the store reads the copy — and each
/// time the next refresh reads every run whole: 2 events, not 0. Binding
/// the read to the statement after it alone carries the runs in the
/// first, and to the one before it alone in the second.
#[test]
fn a_read_whose_journal_was_another_by_the_time_it_was_done_is_read_whole_next() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let copy = dir.path().join("copy.db");
    std::fs::copy(&db, &copy).unwrap();
    for (stated, read) in [(&copy, &db), (&db, &copy)] {
        let mut kept = KeptFleet::default();
        let store = Store::open_read_only(read).unwrap();
        assert_eq!(kept.refresh(&store, Journal::at(stated)).unwrap(), 2);
        let next = refreshed(&mut kept, &db);
        assert_eq!(next, 2, "stated {stated:?}, read {read:?}");
        assert_eq!(refreshed(&mut kept, &db), 0, "and bound");
    }
}

/// A load refused by timing rather than by the journal's bytes — here
/// SQLite's own error over a row it cannot read as text, standing in for
/// a peer's lock — quarantines its run for that refresh only, and one
/// refused by the bytes (envelope text that is not JSON) is kept for as
/// long as its head stands. Both rows are then made readable beneath an
/// unmoved head, past the append fence: the next refresh reads the first
/// whole again and shows it folded, with nothing appended, and leaves the
/// second as it was. A `lasting` that answers true for SQLite's error
/// reads 0; one that answers false for a torn envelope reads 2.
#[test]
fn a_refusal_is_kept_at_its_head_only_when_the_journals_bytes_made_it() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    drop(Store::open(&db).unwrap());
    let journal = rusqlite::Connection::open(&db).unwrap();
    let mut texts = Vec::new();
    for (run_id, planted) in [("locked", "CAST(?3 AS BLOB)"), ("torn", "'{'")] {
        let started = chain(
            run_id,
            std::iter::once((EventType::RunStarted, json!({"feature": run_id}))),
        );
        let text = serde_json::to_string(&started[0]).unwrap();
        plant_run(&journal, run_id, "{}").unwrap();
        let insert = format!(
            "INSERT INTO events (run_id, seq, event_hash, envelope)
             SELECT ?1, 1, ?2, {planted} WHERE length(?3) > 0"
        );
        let row = rusqlite::params![run_id, started[0].event_hash, text];
        journal.execute(&insert, row).unwrap();
        texts.push((run_id, text));
    }
    let state = |kept: &KeptFleet, run_id: &str| {
        let entries = kept.entries();
        let row = entries.iter().find(|entry| entry.run_id == run_id);
        row.unwrap().state.map(|state| state.seq)
    };

    let mut kept = KeptFleet::default();
    assert_eq!(refreshed(&mut kept, &db), 0);
    assert_eq!((state(&kept, "locked"), state(&kept, "torn")), (None, None));
    assert_eq!(kept_rows(&kept), whole_rows(&db));

    journal
        .execute_batch("DROP TRIGGER events_append_only_update")
        .unwrap();
    for (run_id, text) in texts {
        let repair = "UPDATE events SET envelope = ?2 WHERE run_id = ?1";
        journal.execute(repair, [run_id, &text]).unwrap();
    }
    assert_eq!(refreshed(&mut kept, &db), 1, "only the timing read again");
    assert_eq!(
        (state(&kept, "locked"), state(&kept, "torn")),
        (Some(1), None)
    );
}

/// A whole read's own connection that will not open is the refresh's
/// refusal, in the store's words, and nothing is kept from it.
#[test]
fn a_whole_read_that_cannot_open_its_connection_refuses_the_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let before = Journal::at(&db);
    let store = Store::open_read_only(&db).unwrap();
    std::fs::remove_file(&db).unwrap();
    let mut kept = KeptFleet::default();
    let refused = kept.refresh(&store, before).unwrap_err();
    assert_eq!(
        refused.to_string(),
        Store::open_read_only(&db).err().unwrap().to_string()
    );
    assert_eq!(kept.entries().len(), 0);
}

/// `brokkr runs` over many hearths reads each the kept way, and a
/// journal that is not there is that hearth's refusal in the words the
/// whole read gave it.
#[test]
fn a_hearth_read_kept_says_what_the_whole_read_said_of_a_missing_journal() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent.db");
    let (kept, refused) = read_kept(&missing);
    assert_eq!(kept.entries().len(), 0);
    assert_eq!(refused, read_journal(&missing).detail);
    let words = format!(
        "sqlite: unable to open database file: {}",
        missing.display()
    );
    assert_eq!(refused, Some(words));
}

/// [`rules`] keeps what [`brokkr_view::residual_findings`] reads: over a
/// journal of every event type, each carrying a residual ruling's inputs
/// and a supersede's arguments, the findings from the kept rulings are
/// the findings from the whole journal. Dropping `operator/commanded`
/// from `rules` loses the mark; dropping `transition/decided` loses the
/// finding.
#[test]
fn the_kept_rulings_are_what_the_residual_findings_read() {
    use EventType::*;
    let every = [
        RunStarted,
        PhaseEntered,
        EffectRequested,
        EffectStarted,
        EffectCheckpointed,
        EffectSucceeded,
        EffectFailed,
        EffectIndeterminate,
        TransitionDecided,
        OperatorCommanded,
        OperatorAccepted,
        OperatorRejected,
        RunParked,
        RunCompleted,
        RunStopped,
    ];
    let ruling = |event_type| {
        let mut payload = supersede(9);
        payload["from"] = json!("review");
        payload["inputs"] = json!({"max_residual_severity": "high"});
        (event_type, payload)
    };
    let journal = chain("run", every.into_iter().map(ruling));
    let kept: Vec<EventEnvelope> = journal
        .iter()
        .filter(|event| rules(event.event_type))
        .cloned()
        .collect();
    let whole = brokkr_view::residual_findings("run", &journal);
    assert_eq!(
        serde_json::to_value(brokkr_view::residual_findings("run", &kept)).unwrap(),
        serde_json::to_value(&whole).unwrap()
    );
    assert_eq!(whole.len(), 1);
    assert_eq!(whole[0].seq, 9, "read from the ruling");
    let mark = whole[0].superseded.as_ref().map(|mark| mark.seq);
    assert_eq!(mark, Some(10), "and marked by the command");
}
