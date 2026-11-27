//! Conformance and domain-facade tests for [`crate::ports`] (decision 0126).
//!
//! Includes a zero-SQLite in-memory implementation of [`EventLog`],
//! [`ProjectionStore`], and [`CoordinationStore`] to prove [`Journal`]
//! enforces all domain invariants independently of SQLite, plus SQLite
//! conformance tests proving [`Store`] satisfies every port contract.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use serde_json::{json, Value};
use tempfile::tempdir;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::ports::{CoordinationStore, EventLog, Journal, NewEvent, ProjectionStore, SqlitePorts};
use crate::test_support::every_run_reads_as_its_fold;
use crate::{
    Cooldown, CooldownAppend, Cooling, EntryId, EntryState, ImportError, NewEntry, QueueEntry,
    Store, StoreError, VerifyError, RUN_ID_MAX,
};

// ---------------------------------------------------------------------------
// Zero-SQLite in-memory backend
// ---------------------------------------------------------------------------

#[derive(Default)]
struct MemEventLog {
    runs: RefCell<BTreeMap<String, (Value, Vec<EventEnvelope>)>>,
}

fn validate_mem_batch(
    run_id: &str,
    seen: &mut BTreeSet<u64>,
    envelopes: &[EventEnvelope],
) -> Result<(), StoreError> {
    for envelope in envelopes {
        if envelope.run_id != run_id {
            return Err(StoreError::Chain(ChainError::ForeignRun {
                seq: envelope.seq,
            }));
        }
        if !seen.insert(envelope.seq) {
            return Err(StoreError::AppendConflict { seq: envelope.seq });
        }
    }
    Ok(())
}

impl MemEventLog {
    fn with_run<T>(
        &self,
        run_id: &str,
        f: impl FnOnce(&(Value, Vec<EventEnvelope>)) -> T,
    ) -> Result<T, StoreError> {
        let runs = self.runs.borrow();
        let entry = runs
            .get(run_id)
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))?;
        Ok(f(entry))
    }

    fn tamper_payload(&self, run_id: &str, index: usize, payload: Value) {
        let mut runs = self.runs.borrow_mut();
        let (_, events) = runs.get_mut(run_id).expect("run exists");
        events[index].payload = payload;
    }

    fn plant_raw_stream(&self, run_id: &str, manifest: Value, events: Vec<EventEnvelope>) {
        self.runs
            .borrow_mut()
            .insert(run_id.to_string(), (manifest, events));
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
        let mut seen: BTreeSet<u64> = events.iter().map(|existing| existing.seq).collect();
        validate_mem_batch(run_id, &mut seen, envelopes)?;
        events.extend(envelopes.iter().cloned());
        Ok(())
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        let mut runs = self.runs.borrow_mut();
        if runs.contains_key(run_id) {
            return Err(StoreError::RunExists(run_id.to_string()));
        }
        validate_mem_batch(run_id, &mut BTreeSet::new(), envelopes)?;
        runs.insert(run_id.to_string(), (manifest.clone(), envelopes.to_vec()));
        Ok(())
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        self.with_run(run_id, |(_, events)| {
            events.last().map_or_else(
                || (0, ZERO_HASH.to_string()),
                |last| (last.seq, last.event_hash.clone()),
            )
        })
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        self.with_run(run_id, |(_, events)| {
            events
                .iter()
                .filter(|event| event.seq >= from_seq)
                .cloned()
                .collect()
        })
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        self.with_run(run_id, |(manifest, _)| manifest.clone())
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        Ok(self.runs.borrow().keys().cloned().collect())
    }
}

#[derive(Default)]
struct MemProjectionStore {
    runs: RefCell<BTreeMap<String, (String, String)>>,
    applied_seqs: RefCell<BTreeMap<String, BTreeSet<u64>>>,
}

impl MemProjectionStore {
    fn clear(&self) {
        self.runs.borrow_mut().clear();
        self.applied_seqs.borrow_mut().clear();
    }

    fn applied_count(&self, run_id: &str) -> usize {
        self.applied_seqs
            .borrow()
            .get(run_id)
            .map_or(0, BTreeSet::len)
    }
}

impl ProjectionStore for MemProjectionStore {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        let mut runs = self.runs.borrow_mut();
        let mut seqs = self.applied_seqs.borrow_mut();
        let seen = seqs.entry(run_id.to_string()).or_default();
        let manifest_feature = manifest
            .and_then(|value| value.get("feature"))
            .and_then(Value::as_str)
            .filter(|feature| !feature.is_empty());
        for event in envelopes {
            if event.event_type == EventType::RunStarted {
                let existing = runs
                    .get(run_id)
                    .map(|(feature, _)| feature.as_str())
                    .filter(|feature| !feature.is_empty());
                let feature = super::started_feature_of(std::slice::from_ref(event))
                    .or(manifest_feature)
                    .or(existing)
                    .unwrap_or_default()
                    .to_string();
                runs.insert(run_id.to_string(), (feature, event.recorded_at.clone()));
            }
            seen.insert(event.seq);
        }
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
            if events.is_empty() {
                continue;
            }
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
            self.apply_envelopes(&run_id, Some(&manifest), &events)?;
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

fn plain_event<'a>(
    event_type: EventType,
    payload: Value,
    causation_id: Option<&'a str>,
    ts: &'a str,
) -> NewEvent<'a> {
    NewEvent {
        event_type,
        payload,
        causation_id,
        attempt_id: None,
        ts,
    }
}

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

fn panel_member_checkpoint() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "checkpoint": {
            "step": "panel-member-finished",
            "outcome": "succeeded",
            "member": "alpha",
            "authored": "engine",
        }
    })
}

fn bad_settled_payload() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "reason": "member failed",
        "settled_members": { "alpha": {"not_a_result": true} }
    })
}

fn v8_only_succeeded_payload() -> Value {
    json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "result": {
            "step": "seat-turn",
            "model": "m",
            "root_session": {
                "kind": "custom-harness",
                "id": "019c4b7e",
                "harness_version": "1.2.3",
                "persistent": true
            }
        }
    })
}

fn append_valid_checkpoint_flow<E: EventLog, P: ProjectionStore, C: CoordinationStore>(
    journal: &Journal<E, P, C>,
    run_id: &str,
    feature: &str,
) -> EventEnvelope {
    let e1 = journal
        .append_event(
            run_id,
            plain_event(
                EventType::RunStarted,
                started_payload(feature),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    let e2 = journal
        .append_event(
            run_id,
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                Some(&e1.event_id),
                "2026-11-25T10:00:01Z",
            ),
        )
        .unwrap();
    let e3 = journal
        .append_event(
            run_id,
            plain_event(
                EventType::EffectRequested,
                json!({"effect_id": "eff-1", "seat": "implement"}),
                Some(&e2.event_id),
                "2026-11-25T10:00:02Z",
            ),
        )
        .unwrap();
    let e4 = journal
        .append_event(
            run_id,
            plain_event(
                EventType::EffectStarted,
                json!({"effect_id": "eff-1", "attempt_id": "att-1"}),
                Some(&e3.event_id),
                "2026-11-25T10:00:03Z",
            ),
        )
        .unwrap();
    journal
        .append_if_head(
            run_id,
            (4, &e4.event_hash),
            plain_event(
                EventType::EffectCheckpointed,
                valid_checkpoint_payload(),
                Some(&e4.event_id),
                "2026-11-25T10:00:05Z",
            ),
        )
        .unwrap()
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
            plain_event(
                EventType::RunStarted,
                started_payload("chain-test"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    assert_eq!(e1.seq, 1);
    assert_eq!(e1.previous_hash, ZERO_HASH);
    assert_eq!(e1.event_hash, e1.compute_hash());
    assert_eq!(e1.recorded_at, "2026-11-25T10:00:00Z");

    let e2 = journal
        .append_event(
            "run-chain",
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                Some(&e1.event_id),
                "2026-11-25T10:00:05Z",
            ),
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
        plain_event(
            EventType::RunStarted,
            started_payload("missing"),
            None,
            "2026-11-25T10:00:00Z",
        ),
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
            plain_event(
                EventType::RunStarted,
                started_payload("fence"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();

    let wrong_seq = journal.append_if_head(
        "run-fence",
        (0, ZERO_HASH),
        plain_event(
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            "2026-11-25T10:00:01Z",
        ),
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
        plain_event(
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            None,
            "2026-11-25T10:00:02Z",
        ),
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
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                Some(&e1.event_id),
                "2026-11-25T10:00:03Z",
            ),
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
            plain_event(
                EventType::RunStarted,
                started_payload("seat-check"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();

    let refused = journal.append_event(
        "run-seat",
        plain_event(
            EventType::EffectCheckpointed,
            invalid_checkpoint_payload(),
            None,
            "2026-11-25T10:00:01Z",
        ),
    );
    assert!(matches!(
        refused,
        Err(StoreError::SeatRecord(ref err)) if err.seq == 2 && err.path == "/"
    ));
    assert_eq!(journal.event_log().head("run-seat").unwrap().0, 1);

    let accepted = journal
        .append_event(
            "run-seat",
            plain_event(
                EventType::EffectCheckpointed,
                valid_checkpoint_payload(),
                None,
                "2026-11-25T10:00:02Z",
            ),
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
    append_valid_checkpoint_flow(&source, "run-port", "portable");

    let exported = source.export_ndjson("run-port").unwrap();
    assert_eq!(exported.lines().count(), 5);

    let dest = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let with_blank = format!("{exported}\n");
    let count = dest
        .import_ndjson("run-port", &manifest, &with_blank)
        .unwrap();
    assert_eq!(count, 5);
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
    assert!(matches!(dup, Err(ImportError::Collision(id)) if id == "run-port"));

    let foreign = dest.import_ndjson("run-other", &manifest, &exported);
    assert!(matches!(
        foreign,
        Err(ImportError::Verify(VerifyError::Chain(
            ChainError::ForeignRun { seq: 1 }
        )))
    ));
    assert!(matches!(
        dest.event_log().manifest("run-other"),
        Err(StoreError::RunNotFound(id)) if id == "run-other"
    ));

    let bad_json = dest.import_ndjson("run-bad-json", &manifest, "{not json}\n");
    assert!(matches!(
        bad_json,
        Err(ImportError::Verify(VerifyError::Parse(_)))
    ));
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
            plain_event(
                EventType::RunStarted,
                started_payload("feat-1"),
                None,
                "2026-11-25T11:00:00Z",
            ),
        )
        .unwrap();
    journal
        .create_run("run-2", &sample_manifest("feat-2"))
        .unwrap();
    journal
        .append_event(
            "run-2",
            plain_event(
                EventType::RunStarted,
                started_payload("feat-2"),
                None,
                "2026-11-25T11:01:00Z",
            ),
        )
        .unwrap();
    journal
        .append_event(
            "run-2",
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "verify"}),
                None,
                "2026-11-25T11:02:00Z",
            ),
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

// ---------------------------------------------------------------------------
// Shared test helpers & interceptors
// ---------------------------------------------------------------------------

fn sealed_event(
    run_id: &str,
    seq: u64,
    event_type: EventType,
    payload: Value,
    previous_hash: &str,
    ts: &str,
) -> EventEnvelope {
    EventEnvelope {
        run_id: run_id.to_string(),
        seq,
        event_id: format!("ev-{run_id}-{seq}"),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: run_id.to_string(),
        attempt_id: None,
        recorded_at: ts.to_string(),
        previous_hash: previous_hash.to_string(),
        event_hash: String::new(),
    }
    .sealed()
}

fn started_event(run_id: &str, feature: &str, ts: &str) -> EventEnvelope {
    sealed_event(
        run_id,
        1,
        EventType::RunStarted,
        started_payload(feature),
        ZERO_HASH,
        ts,
    )
}

fn single_ndjson(event: &EventEnvelope) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&serde_json::to_value(event).unwrap()).unwrap()
    )
}

fn sqlite_journal(store: &Store) -> Journal<SqlitePorts<'_>, SqlitePorts<'_>, SqlitePorts<'_>> {
    Journal::new(
        SqlitePorts::new(store),
        SqlitePorts::new(store),
        SqlitePorts::new(store),
    )
}

fn sqlite_table_count(path: &std::path::Path, table: &str) -> i64 {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn exec_raw_sql(path: &std::path::Path, sql: &str) {
    rusqlite::Connection::open(path)
        .unwrap()
        .execute_batch(sql)
        .unwrap();
}

fn summary_snapshot(
    path: &std::path::Path,
    run_id: &str,
) -> (
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<i64>,
) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.query_row(
        "SELECT head_seq, folded_seq, status, head_hash, acknowledged
         FROM run_summary WHERE run_id = ?1",
        [run_id],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )
    .unwrap()
}

fn completed_vs_phase_pair(
    run_id: &str,
    feature: &str,
) -> (EventEnvelope, EventEnvelope, EventEnvelope) {
    let e1 = started_event(run_id, feature, "2026-11-25T10:00:00Z");
    let completed = sealed_event(
        run_id,
        2,
        EventType::RunCompleted,
        json!({}),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    let phase = sealed_event(
        run_id,
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &e1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    (e1, completed, phase)
}

struct FailingProjectionStore<P> {
    inner: P,
    fail_next: Cell<bool>,
    auto_reset: bool,
}

impl<P> FailingProjectionStore<P> {
    fn fail_once(inner: P) -> Self {
        Self {
            inner,
            fail_next: Cell::new(true),
            auto_reset: true,
        }
    }

    fn toggleable(inner: P) -> Self {
        Self {
            inner,
            fail_next: Cell::new(false),
            auto_reset: false,
        }
    }

    fn set_fail(&self, fail: bool) {
        self.fail_next.set(fail);
    }
}

impl<P: ProjectionStore> ProjectionStore for FailingProjectionStore<P> {
    fn apply_envelopes(
        &self,
        run_id: &str,
        manifest: Option<&Value>,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        let should_fail = if self.auto_reset {
            self.fail_next.replace(false)
        } else {
            self.fail_next.get()
        };
        if should_fail {
            return Err(StoreError::Contended {
                operation: "apply_envelopes",
                waited_ms: 3,
            });
        }
        self.inner.apply_envelopes(run_id, manifest, envelopes)
    }

    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        self.inner.list_runs()
    }

    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        self.inner.rebuild_from(log)
    }
}

struct InterceptingLog<'a, E> {
    inner: E,
    on_head: Option<Box<dyn Fn() + 'a>>,
    on_list_run_ids: Option<Box<dyn Fn() + 'a>>,
    on_read_stream: Option<Box<dyn Fn() + 'a>>,
    override_run_ids: Option<Vec<String>>,
}

impl<E: EventLog> EventLog for InterceptingLog<'_, E> {
    fn create_run(&self, run_id: &str, manifest: &Value) -> Result<(), StoreError> {
        self.inner.create_run(run_id, manifest)
    }

    fn append_envelopes(
        &self,
        run_id: &str,
        expected_head: (u64, &str),
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        self.inner
            .append_envelopes(run_id, expected_head, envelopes)
    }

    fn import_run_stream(
        &self,
        run_id: &str,
        manifest: &Value,
        envelopes: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        self.inner.import_run_stream(run_id, manifest, envelopes)
    }

    fn head(&self, run_id: &str) -> Result<(u64, String), StoreError> {
        if let Some(hook) = &self.on_head {
            hook();
        }
        self.inner.head(run_id)
    }

    fn read_stream(&self, run_id: &str, from_seq: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        let out = self.inner.read_stream(run_id, from_seq)?;
        if let Some(hook) = &self.on_read_stream {
            hook();
        }
        Ok(out)
    }

    fn manifest(&self, run_id: &str) -> Result<Value, StoreError> {
        self.inner.manifest(run_id)
    }

    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        if let Some(hook) = &self.on_list_run_ids {
            hook();
        }
        if let Some(ids) = &self.override_run_ids {
            return Ok(ids.clone());
        }
        self.inner.list_run_ids()
    }
}

#[test]
fn mem_event_log_append_envelopes_batch_is_atomic() {
    let log = MemEventLog::default();
    log.create_run("batch-1", &sample_manifest("batch"))
        .unwrap();
    let e1 = started_event("batch-1", "batch", "2026-11-25T10:00:00Z");
    assert!(matches!(
        log.append_envelopes("batch-1", (0, ZERO_HASH), &[e1.clone(), e1]),
        Err(StoreError::AppendConflict { seq: 1 })
    ));
    assert_eq!(log.head("batch-1").unwrap(), (0, ZERO_HASH.to_string()));
    assert!(log.read_stream("batch-1", 1).unwrap().is_empty());
}

#[test]
fn manifest_only_feature_round_trips_and_replay_preserves_known_feature() {
    let manifest = sample_manifest("manifest-only");
    let blank_sidecar = json!({"engine": "0.15.0", "bundle_name": "self"});
    let source = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    source.create_run("run-mo", &manifest).unwrap();
    source
        .append_event(
            "run-mo",
            plain_event(
                EventType::RunStarted,
                json!({"manifest": manifest}),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    let exported = source.export_ndjson("run-mo").unwrap();

    // Mem destination round-trips manifest-only feature through export_ndjson -> import_ndjson.
    let mem_dest = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    assert_eq!(
        mem_dest
            .import_ndjson("run-mo", &blank_sidecar, &exported)
            .unwrap(),
        1
    );
    let expected = vec![(
        "run-mo".to_string(),
        "manifest-only".to_string(),
        "2026-11-25T10:00:00Z".to_string(),
    )];
    assert_eq!(mem_dest.projections().list_runs().unwrap(), expected);

    // SQLite destination also round-trips manifest-only feature cleanly.
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("manifest-only.db")).unwrap();
    let sql_dest = sqlite_journal(&store);
    assert_eq!(
        sql_dest
            .import_ndjson("run-mo", &blank_sidecar, &exported)
            .unwrap(),
        1
    );
    assert_eq!(sql_dest.projections().list_runs().unwrap(), expected);

    // Replaying RunStarted without feature and with manifest: None preserves known feature.
    let proj = MemProjectionStore::default();
    let bare_started = sealed_event(
        "run-replay",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    proj.apply_envelopes(
        "run-replay",
        Some(&manifest),
        std::slice::from_ref(&bare_started),
    )
    .unwrap();
    proj.apply_envelopes("run-replay", None, &[bare_started])
        .unwrap();
    assert_eq!(
        proj.list_runs().unwrap(),
        vec![(
            "run-replay".to_string(),
            "manifest-only".to_string(),
            "2026-11-25T10:00:00Z".to_string(),
        )]
    );
}

#[test]
fn empty_top_level_feature_imports_across_store_and_journal() {
    let attested = json!({"engine": "0.15.0", "bundle_name": "self"});
    let started = sealed_event(
        "run-empty-feat",
        1,
        EventType::RunStarted,
        json!({"feature": "", "manifest": attested}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    let ndjson = single_ndjson(&started);
    let expected = vec![(
        "run-empty-feat".to_string(),
        String::new(),
        "2026-11-25T10:00:00Z".to_string(),
    )];

    let dir = tempdir().unwrap();
    let mut legacy_store = Store::open(&dir.path().join("legacy.db")).unwrap();
    legacy_store
        .import_run(
            &ndjson,
            &attested,
            std::path::Path::new("run-empty-feat.ndjson"),
        )
        .unwrap();
    assert_eq!(legacy_store.list_runs().unwrap(), expected);

    let (log, proj, coord) = (
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let mem_dest = Journal::new(log, proj, coord);
    let imported = mem_dest
        .import_ndjson("run-empty-feat", &attested, &ndjson)
        .unwrap();
    assert_eq!(imported, 1);
    assert_eq!(mem_dest.projections().list_runs().unwrap(), expected);

    let sql_store = Store::open(&dir.path().join("ports.db")).unwrap();
    let sql_dest = sqlite_journal(&sql_store);
    assert_eq!(
        sql_dest
            .import_ndjson("run-empty-feat", &attested, &ndjson)
            .unwrap(),
        1
    );
    assert_eq!(sql_dest.projections().list_runs().unwrap(), expected);
}

mod invariants;
mod split;
mod sqlite;
