//! Conformance and domain-facade tests for [`crate::ports`] (decision 0126).
//!
//! Includes a zero-SQLite in-memory implementation of [`EventLog`],
//! [`ProjectionStore`], and [`CoordinationStore`] to prove [`Journal`]
//! enforces all domain invariants independently of SQLite, plus SQLite
//! conformance tests proving [`Store`] satisfies every port contract.

use std::cell::RefCell;
use std::collections::BTreeMap;

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use serde_json::{json, Value};
use tempfile::tempdir;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::ports::{CoordinationStore, EventLog, Journal, ProjectionStore};
use crate::test_support::every_run_reads_as_its_fold;
use crate::{
    Cooldown, CooldownAppend, Cooling, EntryId, EntryState, NewEntry, QueueEntry, Store,
    StoreError, RUN_ID_MAX,
};

// ---------------------------------------------------------------------------
// Zero-SQLite in-memory backend
// ---------------------------------------------------------------------------

#[derive(Default)]
struct MemEventLog {
    runs: RefCell<BTreeMap<String, (Value, Vec<EventEnvelope>)>>,
}

impl MemEventLog {
    fn tamper_payload(&self, run_id: &str, index: usize, payload: Value) {
        let mut runs = self.runs.borrow_mut();
        let (_, events) = runs.get_mut(run_id).expect("run exists");
        events[index].payload = payload;
    }
}

impl EventLog for MemEventLog {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        let mut runs = self.runs.borrow_mut();
        if runs.contains_key(run_id) {
            return Err(StoreError::RunExists(run_id.to_string()));
        }
        runs.insert(run_id.to_string(), (manifest.clone(), Vec::new()));
        Ok(())
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        let mut runs = self.runs.borrow_mut();
        let (_, events) = runs
            .get_mut(run_id)
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        let (found_seq, found_hash) = events
            .last()
            .map_or((0, ZERO_HASH), |last| (last.seq, last.event_hash.as_str()));
        if found_seq != expected_head.0 || found_hash != expected_head.1 {
            return Err(StoreError::HeadMoved {
                expected_seq: expected_head.0,
                found_seq,
            });
        }
        for envelope in envelopes {
            if events.iter().any(|existing| existing.seq == envelope.seq) {
                return Err(StoreError::AppendConflict { seq: envelope.seq });
            }
            events.push(envelope.clone());
        }
        Ok(())
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        let runs = self.runs.borrow();
        let (_, events) = runs
            .get(run_id)
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(events.last().map_or_else(
            || (0, ZERO_HASH.to_string()),
            |last| (last.seq, last.event_hash.clone()),
        ))
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        let runs = self.runs.borrow();
        let (_, events) = runs
            .get(run_id)
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(events
            .iter()
            .filter(|event| event.seq >= from_seq)
            .cloned()
            .collect())
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        let runs = self.runs.borrow();
        runs.get(run_id)
            .map(|(manifest, _)| manifest.clone())
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        Ok(self.runs.borrow().keys().cloned().collect())
    }
}

#[derive(Default)]
struct MemProjectionStore {
    runs: RefCell<BTreeMap<String, (String, String)>>,
    applied_counts: RefCell<BTreeMap<String, usize>>,
}

impl MemProjectionStore {
    fn clear(&self) {
        self.runs.borrow_mut().clear();
        self.applied_counts.borrow_mut().clear();
    }

    fn applied_count(&self, run_id: &str) -> usize {
        self.applied_counts
            .borrow()
            .get(run_id)
            .copied()
            .unwrap_or(0)
    }
}

impl ProjectionStore for MemProjectionStore {
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        let mut runs = self.runs.borrow_mut();
        let mut counts = self.applied_counts.borrow_mut();
        for event in envelopes {
            if event.event_type == EventType::RunStarted {
                let feature = event
                    .payload
                    .get("feature")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                runs.insert(run_id.to_string(), (feature, event.recorded_at.clone()));
            }
        }
        *counts.entry(run_id.to_string()).or_insert(0) += envelopes.len();
        Ok(())
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        Ok(self
            .runs
            .borrow()
            .iter()
            .map(|(id, (feature, created_at))| (id.clone(), feature.clone(), created_at.clone()))
            .collect())
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        self.clear();
        let mut total = 0u64;
        for run_id in log.list_run_ids()? {
            let manifest = log.manifest(&run_id)?;
            let events = log.read_stream(&run_id, 1)?;
            let feature = manifest
                .get("feature")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let created_at = events
                .first()
                .map(|event| event.recorded_at.clone())
                .unwrap_or_default();
            self.runs
                .borrow_mut()
                .insert(run_id.clone(), (feature, created_at));
            self.apply_envelopes(&run_id, &events)?;
            total += events.len() as u64;
        }
        Ok(total)
    }
}

#[derive(Default)]
struct MemCoordinationStore {
    next_id: RefCell<i64>,
    entries: RefCell<Vec<QueueEntry>>,
    cooldowns: RefCell<Vec<Cooldown>>,
}

impl MemCoordinationStore {
    fn add_cooldown(&self, cooldown: Cooldown) {
        self.cooldowns.borrow_mut().push(cooldown);
    }
}

impl CoordinationStore for MemCoordinationStore {
    fn enqueue(&self, entry: &NewEntry<'_>, now: &str) -> Result<EntryId, StoreError> {
        OffsetDateTime::parse(now, &Rfc3339).map_err(|_| StoreError::Clock {
            now: now.to_string(),
        })?;
        let mut next = self.next_id.borrow_mut();
        *next += 1;
        let id = EntryId(*next);
        let mut entries = self.entries.borrow_mut();
        let next_pos = entries.len() as u32 + 1;
        entries.push(QueueEntry {
            id,
            position: Some(next_pos),
            state: EntryState::Queued,
            priority: entry.priority,
            waits: entry.waits.to_vec(),
            payload: entry.payload.to_string(),
            pin: None,
            added_at: now.to_string(),
            latch: None,
        });
        entries.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
        for (idx, item) in entries.iter_mut().enumerate() {
            item.position = Some(idx as u32 + 1);
        }
        Ok(id)
    }

    fn list_queue(&self) -> Result<Vec<QueueEntry>, StoreError> {
        Ok(self.entries.borrow().clone())
    }

    fn active_cooldowns(&self, now: &str) -> Result<Vec<Cooldown>, StoreError> {
        let instant = OffsetDateTime::parse(now, &Rfc3339).map_err(|_| StoreError::Clock {
            now: now.to_string(),
        })?;
        Ok(self
            .cooldowns
            .borrow()
            .iter()
            .filter(|cooldown| cooldown.until_at > instant)
            .cloned()
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn sample_manifest(feature: &str) -> Value {
    json!({
        "engine": "0.15.0",
        "feature": feature,
        "bundle_name": "self",
    })
}

fn started_payload(feature: &str) -> Value {
    json!({
        "feature": feature,
        "manifest": sample_manifest(feature),
    })
}

fn valid_checkpoint_payload() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "checkpoint": {
            "step": "seat-turn",
            "turn": 1,
        }
    })
}

fn invalid_checkpoint_payload() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "checkpoint": {
            "not_a_valid_field": true,
        }
    })
}

fn rfc3339(ts: &str) -> OffsetDateTime {
    OffsetDateTime::parse(ts, &Rfc3339).expect("valid RFC3339 fixture")
}

// ---------------------------------------------------------------------------
// Zero-SQLite Journal domain tests
// ---------------------------------------------------------------------------

#[test]
fn memory_journal_enforces_run_creation_and_duplicate_refusal() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("ports-feature");
    journal.create_run("run-alpha", &manifest).unwrap();
    assert_eq!(journal.event_log().manifest("run-alpha").unwrap(), manifest);
    assert_eq!(
        journal.event_log().list_run_ids().unwrap(),
        vec!["run-alpha".to_string()]
    );

    let dup = journal.create_run("run-alpha", &manifest);
    assert!(matches!(dup, Err(StoreError::RunExists(id)) if id == "run-alpha"));

    let empty = journal.create_run("", &manifest);
    assert!(matches!(empty, Err(StoreError::RunIdUnnameable(_))));

    let overlong = "a".repeat(RUN_ID_MAX + 1);
    let too_long = journal.create_run(&overlong, &manifest);
    assert!(matches!(
        too_long,
        Err(StoreError::RunIdTooLong { length }) if length == RUN_ID_MAX + 1
    ));
}

#[test]
fn memory_journal_chains_events_verifies_and_detects_tampering() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("chain-test");
    journal.create_run("run-chain", &manifest).unwrap();

    let e1 = journal
        .append_event(
            "run-chain",
            EventType::RunStarted,
            started_payload("chain-test"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();
    assert_eq!(e1.seq, 1);
    assert_eq!(e1.previous_hash, ZERO_HASH);
    assert_eq!(e1.event_hash, e1.compute_hash());
    assert_eq!(e1.recorded_at, "2026-11-25T10:00:00Z");

    let e2 = journal
        .append_event(
            "run-chain",
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            Some(&e1.event_id),
            "2026-11-25T10:00:05Z",
        )
        .unwrap();
    assert_eq!(e2.seq, 2);
    assert_eq!(e2.previous_hash, e1.event_hash);
    assert_eq!(e2.causation_id.as_deref(), Some(e1.event_id.as_str()));

    let loaded = journal.load_verified("run-chain").unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[1].event_hash, e2.event_hash);
    assert_eq!(journal.projections().applied_count("run-chain"), 2);

    journal
        .event_log()
        .tamper_payload("run-chain", 1, json!({"phase": "tampered"}));
    let err = journal.load_verified("run-chain");
    assert!(matches!(
        err,
        Err(StoreError::Chain(ChainError::BadHash { seq: 2 }))
    ));

    let missing = journal.append_event(
        "no-such-run",
        EventType::RunStarted,
        started_payload("missing"),
        None,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(missing, Err(StoreError::RunNotFound(id)) if id == "no-such-run"));
}

#[test]
fn memory_journal_append_if_head_fences_seq_and_hash() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("run-fence", &sample_manifest("fence"))
        .unwrap();

    let e1 = journal
        .append_if_head(
            "run-fence",
            (0, ZERO_HASH),
            EventType::RunStarted,
            started_payload("fence"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();

    let wrong_seq = journal.append_if_head(
        "run-fence",
        (0, ZERO_HASH),
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        None,
        "2026-11-25T10:00:01Z",
    );
    assert!(matches!(
        wrong_seq,
        Err(StoreError::HeadMoved {
            expected_seq: 0,
            found_seq: 1
        })
    ));

    let wrong_hash = journal.append_if_head(
        "run-fence",
        (1, ZERO_HASH),
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        None,
        "2026-11-25T10:00:02Z",
    );
    assert!(matches!(
        wrong_hash,
        Err(StoreError::HeadMoved {
            expected_seq: 1,
            found_seq: 1
        })
    ));

    let e2 = journal
        .append_if_head(
            "run-fence",
            (1, &e1.event_hash),
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            Some(&e1.event_id),
            "2026-11-25T10:00:03Z",
        )
        .unwrap();
    assert_eq!(e2.seq, 2);
    assert_eq!(
        journal.event_log().head("run-fence").unwrap(),
        (2, e2.event_hash)
    );
}

#[test]
fn memory_journal_validates_seat_record_checkpoints() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("run-seat", &sample_manifest("seat-check"))
        .unwrap();
    journal
        .append_event(
            "run-seat",
            EventType::RunStarted,
            started_payload("seat-check"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();

    let refused = journal.append_event(
        "run-seat",
        EventType::EffectCheckpointed,
        invalid_checkpoint_payload(),
        None,
        "2026-11-25T10:00:01Z",
    );
    assert!(matches!(
        refused,
        Err(StoreError::SeatRecord(ref err)) if err.seq == 2 && err.path == "/"
    ));
    assert_eq!(journal.event_log().head("run-seat").unwrap().0, 1);

    let accepted = journal
        .append_event(
            "run-seat",
            EventType::EffectCheckpointed,
            valid_checkpoint_payload(),
            None,
            "2026-11-25T10:00:02Z",
        )
        .unwrap();
    assert_eq!(accepted.seq, 2);
}

#[test]
fn memory_journal_export_import_round_trip_and_refusals() {
    let source = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("portable");
    source.create_run("run-port", &manifest).unwrap();
    source
        .append_event(
            "run-port",
            EventType::RunStarted,
            started_payload("portable"),
            None,
            "2026-11-25T10:00:00Z",
        )
        .unwrap();
    source
        .append_event(
            "run-port",
            EventType::EffectCheckpointed,
            valid_checkpoint_payload(),
            None,
            "2026-11-25T10:00:05Z",
        )
        .unwrap();

    let exported = source.export_ndjson("run-port").unwrap();
    assert_eq!(exported.lines().count(), 2);

    let dest = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let with_blank = format!("{exported}\n");
    let count = dest
        .import_ndjson("run-port", &manifest, &with_blank)
        .unwrap();
    assert_eq!(count, 2);
    assert_eq!(dest.export_ndjson("run-port").unwrap(), exported);
    assert_eq!(
        dest.projections().list_runs().unwrap(),
        vec![(
            "run-port".to_string(),
            "portable".to_string(),
            "2026-11-25T10:00:00Z".to_string()
        )]
    );

    let dup = dest.import_ndjson("run-port", &manifest, &exported);
    assert!(matches!(dup, Err(StoreError::RunExists(id)) if id == "run-port"));

    let foreign = dest.import_ndjson("run-other", &manifest, &exported);
    assert!(matches!(
        foreign,
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert!(matches!(
        dest.event_log().manifest("run-other"),
        Err(StoreError::RunNotFound(id)) if id == "run-other"
    ));

    let bad_json = dest.import_ndjson("run-bad-json", &manifest, "{not json}\n");
    assert!(matches!(bad_json, Err(StoreError::Json(_))));
}

#[test]
fn memory_journal_reindex_rebuilds_projections_from_event_log() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("run-1", &sample_manifest("feat-1"))
        .unwrap();
    journal
        .append_event(
            "run-1",
            EventType::RunStarted,
            started_payload("feat-1"),
            None,
            "2026-11-25T11:00:00Z",
        )
        .unwrap();
    journal
        .create_run("run-2", &sample_manifest("feat-2"))
        .unwrap();
    journal
        .append_event(
            "run-2",
            EventType::RunStarted,
            started_payload("feat-2"),
            None,
            "2026-11-25T11:01:00Z",
        )
        .unwrap();
    journal
        .append_event(
            "run-2",
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            "2026-11-25T11:02:00Z",
        )
        .unwrap();

    journal.projections().clear();
    assert!(journal.projections().list_runs().unwrap().is_empty());

    let projected = journal.reindex().unwrap();
    assert_eq!(projected, 3);
    assert_eq!(
        journal.projections().list_runs().unwrap(),
        vec![
            (
                "run-1".to_string(),
                "feat-1".to_string(),
                "2026-11-25T11:00:00Z".to_string()
            ),
            (
                "run-2".to_string(),
                "feat-2".to_string(),
                "2026-11-25T11:01:00Z".to_string()
            ),
        ]
    );
}

#[test]
fn memory_journal_coordination_enqueue_list_and_cooldowns() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let low = NewEntry {
        payload: r#"{"feature":"low"}"#,
        priority: 1,
        waits: &[],
    };
    let high = NewEntry {
        payload: r#"{"feature":"high"}"#,
        priority: 10,
        waits: &[],
    };
    let id_low = journal
        .coordination()
        .enqueue(&low, "2026-11-25T12:00:00Z")
        .unwrap();
    let id_high = journal
        .coordination()
        .enqueue(&high, "2026-11-25T12:00:01Z")
        .unwrap();
    assert_eq!(id_low, EntryId(1));
    assert_eq!(id_high, EntryId(2));

    let queued = journal.coordination().list_queue().unwrap();
    assert_eq!(queued.len(), 2);
    assert_eq!(queued[0].id, id_high);
    assert_eq!(queued[0].position, Some(1));
    assert_eq!(queued[1].id, id_low);
    assert_eq!(queued[1].position, Some(2));

    journal.coordination().add_cooldown(Cooldown {
        site: "seat-a".to_string(),
        provider: "claude".to_string(),
        route: "default".to_string(),
        until_at: rfc3339("2026-11-25T12:10:00Z"),
        reason: "rate limit".to_string(),
        retry_after_seconds: Some(600),
        rounds: 1,
        recorded_at: rfc3339("2026-11-25T12:00:00Z"),
    });
    assert_eq!(
        journal
            .coordination()
            .active_cooldowns("2026-11-25T12:05:00Z")
            .unwrap()
            .len(),
        1
    );
    assert!(journal
        .coordination()
        .active_cooldowns("2026-11-25T12:15:00Z")
        .unwrap()
        .is_empty());
}

// ---------------------------------------------------------------------------
// SQLite Store conformance & end-to-end Journal tests
// ---------------------------------------------------------------------------

#[test]
fn sqlite_store_satisfies_event_log_and_projection_ports() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let manifest = sample_manifest("sqlite-conformance");

    assert!(matches!(
        EventLog::head(&store, "absent"),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));
    assert!(matches!(
        EventLog::read_stream(&store, "absent", 1),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));
    assert!(matches!(
        EventLog::append_envelopes(&store, "absent", (0, ZERO_HASH), &[]),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));
    assert!(matches!(
        ProjectionStore::apply_envelopes(&store, "absent", &[]),
        Err(StoreError::RunNotFound(id)) if id == "absent"
    ));

    EventLog::create_run(&store, "sql-1", &manifest).unwrap();
    assert_eq!(
        EventLog::head(&store, "sql-1").unwrap(),
        (0, ZERO_HASH.to_string())
    );
    assert_eq!(EventLog::manifest(&store, "sql-1").unwrap(), manifest);
    assert_eq!(
        EventLog::list_run_ids(&store).unwrap(),
        vec!["sql-1".to_string()]
    );
    assert!(matches!(
        EventLog::create_run(&store, "sql-1", &manifest),
        Err(StoreError::RunExists(id)) if id == "sql-1"
    ));

    let e1 = EventEnvelope {
        run_id: "sql-1".to_string(),
        seq: 1,
        event_id: "ev-1".to_string(),
        event_schema_version: 1,
        event_type: EventType::RunStarted,
        payload: started_payload("sqlite-conformance"),
        causation_id: None,
        correlation_id: "sql-1".to_string(),
        attempt_id: None,
        recorded_at: "2026-11-25T13:00:00Z".to_string(),
        previous_hash: ZERO_HASH.to_string(),
        event_hash: String::new(),
    }
    .sealed();
    let e2 = EventEnvelope {
        run_id: "sql-1".to_string(),
        seq: 2,
        event_id: "ev-2".to_string(),
        event_schema_version: 1,
        event_type: EventType::PhaseEntered,
        payload: json!({"phase": "implement"}),
        causation_id: Some("ev-1".to_string()),
        correlation_id: "sql-1".to_string(),
        attempt_id: None,
        recorded_at: "2026-11-25T13:00:05Z".to_string(),
        previous_hash: e1.event_hash.clone(),
        event_hash: String::new(),
    }
    .sealed();

    assert!(matches!(
        EventLog::append_envelopes(&store, "sql-1", (1, ZERO_HASH), std::slice::from_ref(&e1)),
        Err(StoreError::HeadMoved {
            expected_seq: 1,
            found_seq: 0
        })
    ));
    assert!(matches!(
        EventLog::append_envelopes(&store, "sql-1", (0, "bad-hash"), std::slice::from_ref(&e1),),
        Err(StoreError::HeadMoved {
            expected_seq: 0,
            found_seq: 0
        })
    ));
    assert!(matches!(
        EventLog::append_envelopes(&store, "sql-1", (0, ZERO_HASH), &[e1.clone(), e1.clone()]),
        Err(StoreError::AppendConflict { seq: 1 })
    ));

    EventLog::append_envelopes(&store, "sql-1", (0, ZERO_HASH), &[e1.clone(), e2.clone()]).unwrap();
    ProjectionStore::apply_envelopes(&store, "sql-1", &[e1, e2.clone()]).unwrap();
    assert_eq!(
        EventLog::head(&store, "sql-1").unwrap(),
        (2, e2.event_hash.clone())
    );
    assert_eq!(EventLog::read_stream(&store, "sql-1", 1).unwrap().len(), 2);
    assert_eq!(EventLog::read_stream(&store, "sql-1", 2).unwrap().len(), 1);
    assert_eq!(ProjectionStore::list_runs(&store).unwrap().len(), 1);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());
}

#[test]
fn sqlite_projection_rebuilds_from_external_memory_event_log() {
    let dir = tempdir().unwrap();
    let mem_log = MemEventLog::default();
    let mem_journal = Journal::new(
        &mem_log,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    mem_journal
        .create_run("from-mem", &sample_manifest("external"))
        .unwrap();
    mem_journal
        .append_event(
            "from-mem",
            EventType::RunStarted,
            started_payload("external"),
            None,
            "2026-11-25T13:10:00Z",
        )
        .unwrap();
    let fresh_sql = Store::open(&dir.path().join("rebuilt.db")).unwrap();
    let rebuilt_count = ProjectionStore::rebuild_from(&fresh_sql, &mem_log).unwrap();
    assert_eq!(rebuilt_count, 1);
    let listed = ProjectionStore::list_runs(&fresh_sql).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].0, "from-mem");
    assert_eq!(listed[0].1, "external");
    assert_eq!(
        every_run_reads_as_its_fold(&fresh_sql),
        Vec::<String>::new()
    );
}

#[test]
fn sqlite_store_satisfies_coordination_port() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("coord.db")).unwrap();

    let entry = NewEntry {
        payload: r#"{"recipe":"self"}"#,
        priority: 5,
        waits: &[],
    };
    assert!(matches!(
        CoordinationStore::enqueue(&store, &entry, "not-a-timestamp"),
        Err(StoreError::Clock { ref now }) if now == "not-a-timestamp"
    ));

    let id = CoordinationStore::enqueue(&store, &entry, "2026-11-25T14:00:00Z").unwrap();
    let listed = CoordinationStore::list_queue(&store).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].added_at, "2026-11-25T14:00:00Z");
    assert_eq!(listed[0].payload, entry.payload);

    let manifest = sample_manifest("cool");
    Store::create_run(&mut store, "cool-run", "cool", "self", &manifest).unwrap();
    store
        .append_next(
            "cool-run",
            EventType::RunStarted,
            started_payload("cool"),
            None,
            None,
        )
        .unwrap();
    let until = OffsetDateTime::now_utc() + time::Duration::hours(2);
    let cooling = Cooling {
        site: "implement".to_string(),
        provider: "claude".to_string(),
        route: "primary".to_string(),
        until_at: until,
        reason: "429 rate limited".to_string(),
        retry_after_seconds: Some(7200),
        rounds: 1,
    };
    store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: "cool-run",
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({"effect_id": "e1", "attempt_id": "a1", "reason": "429"}),
                causation_id: None,
                attempt_id: Some("a1".to_string()),
            },
            &[cooling],
        )
        .unwrap();

    let before = (until - time::Duration::hours(1))
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();
    let after = (until + time::Duration::hours(1))
        .replace_nanosecond(0)
        .unwrap()
        .format(&Rfc3339)
        .unwrap();

    let active = CoordinationStore::active_cooldowns(&store, &before).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].provider, "claude");
    assert_eq!(active[0].route, "primary");

    assert!(CoordinationStore::active_cooldowns(&store, &after)
        .unwrap()
        .is_empty());
    assert!(matches!(
        CoordinationStore::active_cooldowns(&store, "bad-clock"),
        Err(StoreError::Clock { ref now }) if now == "bad-clock"
    ));
}

#[test]
fn journal_end_to_end_over_sqlite_references() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("primary.db")).unwrap();
    let journal = Journal::new(&store, &store, &store);

    let manifest = sample_manifest("e2e-sqlite");
    journal.create_run("e2e-1", &manifest).unwrap();
    assert_eq!(
        journal.event_log().list_run_ids().unwrap(),
        vec!["e2e-1".to_string()]
    );

    let e1 = journal
        .append_event(
            "e2e-1",
            EventType::RunStarted,
            started_payload("e2e-1"),
            None,
            "2026-11-25T15:00:00Z",
        )
        .unwrap();
    let e2 = journal
        .append_if_head(
            "e2e-1",
            (1, &e1.event_hash),
            EventType::EffectCheckpointed,
            valid_checkpoint_payload(),
            Some(&e1.event_id),
            "2026-11-25T15:00:10Z",
        )
        .unwrap();
    assert_eq!(e2.seq, 2);

    assert!(matches!(
        journal.append_event(
            "e2e-1",
            EventType::EffectCheckpointed,
            invalid_checkpoint_payload(),
            None,
            "2026-11-25T15:00:15Z",
        ),
        Err(StoreError::SeatRecord(ref err)) if err.seq == 3
    ));

    assert!(matches!(
        journal.append_if_head(
            "e2e-1",
            (1, &e1.event_hash),
            EventType::PhaseEntered,
            json!({"phase": "verify"}),
            None,
            "2026-11-25T15:00:20Z",
        ),
        Err(StoreError::HeadMoved {
            expected_seq: 1,
            found_seq: 2
        })
    ));

    let verified = journal.load_verified("e2e-1").unwrap();
    assert_eq!(verified.len(), 2);
    assert_eq!(journal.projections().list_runs().unwrap().len(), 1);
    assert_eq!(journal.reindex().unwrap(), 2);
    assert_eq!(every_run_reads_as_its_fold(&store), Vec::<String>::new());

    let entry = NewEntry {
        payload: r#"{"recipe":"verify"}"#,
        priority: 3,
        waits: &[],
    };
    let id = journal
        .coordination()
        .enqueue(&entry, "2026-11-25T15:01:00Z")
        .unwrap();
    assert_eq!(journal.coordination().list_queue().unwrap()[0].id, id);
    assert!(journal
        .coordination()
        .active_cooldowns("2026-11-25T15:01:00Z")
        .unwrap()
        .is_empty());

    let ndjson = journal.export_ndjson("e2e-1").unwrap();
    assert_eq!(ndjson, store.export_ndjson("e2e-1").unwrap());

    let replica = Store::open(&dir.path().join("replica.db")).unwrap();
    let replica_journal = Journal::new(&replica, &replica, &replica);
    let imported = replica_journal
        .import_ndjson("e2e-1", &manifest, &ndjson)
        .unwrap();
    assert_eq!(imported, 2);
    assert_eq!(replica_journal.export_ndjson("e2e-1").unwrap(), ndjson);
    assert_eq!(every_run_reads_as_its_fold(&replica), Vec::<String>::new());
}
