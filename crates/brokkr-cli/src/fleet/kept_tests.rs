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
use super::{read_hearth, read_journal, read_kept, KeptFleet, ListedRun};
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
    kept.refresh(&Store::open_read_only(db).unwrap()).unwrap()
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

/// A journal replaced beneath the reader holds a different chain for the
/// same run: the head the kept fold stands at is not its row any more,
/// so the run is read whole — three events, not the one past the old
/// head — and shown as the new journal says.
#[test]
fn a_run_whose_head_the_journal_no_longer_holds_is_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let mut kept = KeptFleet::default();
    assert_eq!(refreshed(&mut kept, &db), 2);

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

/// A whole read's own connection that will not open is the refresh's
/// refusal, in the store's words, and nothing is kept from it.
#[test]
fn a_whole_read_that_cannot_open_its_connection_refuses_the_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    running_store(&db, "r1");
    let store = Store::open_read_only(&db).unwrap();
    std::fs::remove_file(&db).unwrap();
    let mut kept = KeptFleet::default();
    let refused = kept.refresh(&store).unwrap_err();
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
