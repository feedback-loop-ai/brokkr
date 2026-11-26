//! Domain-invariant and security tests for [`Journal`] and storage ports
//! (decision 0126, Slice 1): manifest validation, method non-shadowing,
//! adoption gates, atomic import, post-terminal refusal, seat-record version
//! resolution, settled members, seq overflow, prefix/foreign-run tampering,
//! `attempt_id` round-trip, and `ProjectionLag` recovery.

use std::cell::Cell;
use std::path::Path;

use brokkr_core::fold::{FoldError, Status};

use super::*;
use crate::{ManifestRefusal, SeatRecordVersion};

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

fn v8_succeeded_event() -> NewEvent<'static> {
    NewEvent {
        event_type: EventType::EffectSucceeded,
        payload: v8_only_succeeded_payload(),
        causation_id: None,
        attempt_id: Some("att-1"),
        ts: "2026-11-25T10:00:05Z",
    }
}

fn assert_v8_refused_under_v1(
    journal: &Journal<SqlitePorts<'_>, SqlitePorts<'_>, SqlitePorts<'_>>,
    run_id: &str,
) {
    let err = journal
        .append_event(run_id, v8_succeeded_event())
        .unwrap_err();
    assert!(matches!(
        err,
        StoreError::SeatRecord(ref e) if e.contract == SeatRecordVersion::V1.contract()
    ));
}

fn assert_invalid_manifest(
    journal: &Journal<MemEventLog, MemProjectionStore, MemCoordinationStore>,
    ports: &SqlitePorts<'_>,
    id: &str,
    bad: &Value,
    expected: ManifestRefusal,
) -> String {
    let check = |err: StoreError| {
        assert!(matches!(
            err,
            StoreError::InvalidManifest { ref run_id, reason }
                if run_id == id && reason == expected
        ));
        err.to_string()
    };
    let text = check(journal.create_run(id, bad).unwrap_err());
    check(ports.create_run(id, bad).unwrap_err());
    check(ports.import_run_stream(id, bad, &[]).unwrap_err());
    text
}

#[test]
fn manifest_validation_refuses_non_object_and_non_string_engine() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("manifest-val.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let mem_journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );

    let groups = [
        (
            "non-obj",
            ManifestRefusal::NotAnObject,
            "invalid manifest for run 'non-obj-0': manifest must be a JSON object; nothing was recorded",
            vec![json!(null), json!(42), json!("v1"), json!(["a"])],
        ),
        (
            "bad-eng",
            ManifestRefusal::EngineNotString,
            "invalid manifest for run 'bad-eng-0': manifest field 'engine' must be a JSON string; nothing was recorded",
            vec![
                json!({"engine": 15}),
                json!({"engine": null}),
                json!({"engine": true}),
                json!({"engine": ["0.15.0"]}),
                json!({"engine": {"major": 15}}),
            ],
        ),
        (
            "bad-feat",
            ManifestRefusal::FeatureNotString,
            "invalid manifest for run 'bad-feat-0': manifest field 'feature' must be a JSON string; nothing was recorded",
            vec![
                json!({"engine": "0.15.0", "feature": 42}),
                json!({"engine": "0.15.0", "feature": null}),
                json!({"engine": "0.15.0", "feature": true}),
                json!({"feature": ["feat"]}),
                json!({"feature": {"name": "feat"}}),
            ],
        ),
    ];
    for (prefix, reason, first_msg, cases) in groups {
        for (idx, bad) in cases.iter().enumerate() {
            let id = format!("{prefix}-{idx}");
            let msg = assert_invalid_manifest(&mem_journal, &ports, &id, bad, reason);
            if idx == 0 {
                assert_eq!(msg, first_msg);
            }
        }
    }

    assert!(mem_journal.event_log().list_run_ids().unwrap().is_empty());
    assert!(ports.list_run_ids().unwrap().is_empty());

    // Valid manifests: missing `"engine"`, missing `"feature"`, and both strings.
    let no_engine = json!({"feature": "legacy", "bundle_name": "self"});
    let no_feature = json!({"engine": "0.15.0"});
    mem_journal.create_run("ok-no-eng", &no_engine).unwrap();
    ports.create_run("ok-no-eng", &no_engine).unwrap();
    ports.create_run("ok-no-feat", &no_feature).unwrap();
    ports
        .create_run("ok-str-eng", &sample_manifest("ok"))
        .unwrap();
    assert_eq!(ports.list_run_ids().unwrap().len(), 3);
}

#[test]
fn journal_refuses_invalid_run_started_and_attested_manifests() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("run-started-manifest.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let sql_journal = sqlite_journal(&store);
    let mem_journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let no_engine = json!({"feature": "legacy", "bundle_name": "self"});
    mem_journal.create_run("ok-no-eng", &no_engine).unwrap();
    ports
        .create_run("ok-str-eng", &sample_manifest("ok"))
        .unwrap();
    let ts = "2026-11-25T10:00:00Z";
    for (bad_manifest, reason) in [
        (json!({"engine": 15}), ManifestRefusal::EngineNotString),
        (
            json!({"engine": "0.15.0", "feature": 42}),
            ManifestRefusal::FeatureNotString,
        ),
        (json!(42), ManifestRefusal::NotAnObject),
    ] {
        let payload = json!({"feature": "ok", "manifest": bad_manifest});
        assert!(matches!(
            mem_journal.append_event(
                "ok-no-eng",
                plain_event(EventType::RunStarted, payload.clone(), None, ts),
            ),
            Err(StoreError::InvalidManifest { ref run_id, reason: got })
                if run_id == "ok-no-eng" && got == reason
        ));
        assert!(matches!(
            sql_journal.append_if_head(
                "ok-str-eng",
                (0, ZERO_HASH),
                plain_event(EventType::RunStarted, payload, None, ts),
            ),
            Err(StoreError::InvalidManifest { ref run_id, reason: got })
                if run_id == "ok-str-eng" && got == reason
        ));
    }
    assert_eq!(
        mem_journal.event_log().head("ok-no-eng").unwrap(),
        (0, ZERO_HASH.to_string())
    );
    assert_eq!(
        ports.head("ok-str-eng").unwrap(),
        (0, ZERO_HASH.to_string())
    );

    // Journal::import_ndjson also validates the attested RunStarted manifest across backends.
    let bad_attested = sealed_event(
        "bad-imp",
        1,
        EventType::RunStarted,
        json!({"feature": "ok", "manifest": {"engine": 15}}),
        ZERO_HASH,
        ts,
    );
    assert!(matches!(
        mem_journal.import_ndjson("bad-imp", &sample_manifest("ok"), &single_ndjson(&bad_attested)),
        Err(ImportError::Store(StoreError::InvalidManifest {
            ref run_id,
            reason: ManifestRefusal::EngineNotString,
        })) if run_id == "bad-imp"
    ));
}

#[test]
fn wildcard_ports_import_does_not_shadow_store_methods() {
    use crate::ports::*;

    let dir = tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("shadow.db")).unwrap();
    let manifest = sample_manifest("no-shadow");
    store
        .create_run("shadow-1", "no-shadow", "self", &manifest)
        .unwrap();
    assert_eq!(store.list_runs().unwrap().len(), 1);
    assert_eq!(store.manifest("shadow-1").unwrap(), manifest);
    let ports = SqlitePorts::new(&store);
    assert!(ports.list_queue().unwrap().is_empty());
}

#[test]
fn import_ndjson_enforces_adoption_gates_and_attested_manifest() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("adopt.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let journal = Journal::new(&ports, &ports, &ports);
    let sidecar = sample_manifest("sidecar");

    for bad_id in ["", "a/b", "..", "with space", "bidi\u{202E}id"] {
        assert!(matches!(
            journal.import_ndjson(bad_id, &sidecar, ""),
            Err(ImportError::UnadoptableRunId(_))
        ));
    }
    assert!(matches!(
        journal.import_ndjson("empty-run", &sidecar, ""),
        Err(ImportError::Verify(VerifyError::Fold(FoldError::Empty)))
    ));

    let bad_proto = sealed_event(
        "proto-bad",
        1,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("proto-bad", &sidecar, &single_ndjson(&bad_proto)),
        Err(ImportError::Verify(VerifyError::Fold(_)))
    ));

    let redacted_sidecar = json!({"engine": "0.15.0", "redacted": true});
    assert!(matches!(
        journal.import_ndjson("redacted-1", &redacted_sidecar, ""),
        Err(ImportError::Redacted(id)) if id == "redacted-1"
    ));

    let redacted_started = sealed_event(
        "redacted-2",
        1,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {"engine": "0.15.0", "redacted": true}}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("redacted-2", &sidecar, &single_ndjson(&redacted_started)),
        Err(ImportError::Redacted(id)) if id == "redacted-2"
    ));

    let no_feature = sealed_event(
        "no-feat",
        1,
        EventType::RunStarted,
        json!({"manifest": sample_manifest("x")}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("no-feat", &sidecar, &single_ndjson(&no_feature)),
        Err(ImportError::Unattested { field: "feature" })
    ));

    let no_manifest = sealed_event(
        "no-man",
        1,
        EventType::RunStarted,
        json!({"feature": "x"}),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        journal.import_ndjson("no-man", &sidecar, &single_ndjson(&no_manifest)),
        Err(ImportError::Unattested { field: "manifest" })
    ));

    let attested_manifest = sample_manifest("attested-feature");
    let started = sealed_event(
        "attested-1",
        1,
        EventType::RunStarted,
        json!({"feature": "attested-feature", "manifest": attested_manifest}),
        ZERO_HASH,
        "2026-11-25T11:00:00Z",
    );
    let downgraded_sidecar = json!({"engine": "0.7.0", "feature": "evil", "bundle_name": "evil"});
    assert_eq!(
        journal
            .import_ndjson("attested-1", &downgraded_sidecar, &single_ndjson(&started))
            .unwrap(),
        1
    );
    assert_eq!(ports.manifest("attested-1").unwrap(), attested_manifest);
    assert!(!store.started_here("attested-1").unwrap());
}

#[test]
fn import_run_stream_is_atomic_on_failure_and_allows_retry() {
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("atomic.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    let mem = MemEventLog::default();
    let manifest = sample_manifest("atomic");
    let e1 = started_event("atomic-1", "atomic", "2026-11-25T11:10:00Z");
    let foreign = started_event("other-run", "atomic", "2026-11-25T11:10:00Z");

    assert!(matches!(
        ports.import_run_stream("atomic-1", &manifest, &[e1.clone(), e1.clone()]),
        Err(StoreError::Chain(ChainError::SeqGap {
            seq: 1,
            expected: 2
        }))
    ));
    assert!(matches!(
        mem.import_run_stream("atomic-1", &manifest, &[e1.clone(), e1.clone()]),
        Err(StoreError::AppendConflict { seq: 1 })
    ));
    for log in [&ports as &dyn EventLog, &mem as &dyn EventLog] {
        assert!(matches!(
            log.import_run_stream("atomic-1", &manifest, std::slice::from_ref(&foreign)),
            Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
        ));
        assert!(log.list_run_ids().unwrap().is_empty());
        assert!(matches!(
            log.manifest("atomic-1"),
            Err(StoreError::RunNotFound(id)) if id == "atomic-1"
        ));
        log.import_run_stream("atomic-1", &manifest, std::slice::from_ref(&e1))
            .unwrap();
        assert_eq!(log.head("atomic-1").unwrap(), (1, e1.event_hash.clone()));
    }
    assert!(!store.started_here("atomic-1").unwrap());
    ports
        .import_run_stream("empty-import", &manifest, &[])
        .unwrap();
    assert!(!store.started_here("empty-import").unwrap());
}

#[test]
fn post_terminal_refusal_on_journal() {
    let mem_journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("term");
    for (run_id, terminal_type, expected_status) in [
        ("term-done", EventType::RunCompleted, Status::Completed),
        ("term-stop", EventType::RunStopped, Status::Stopped),
    ] {
        mem_journal.create_run(run_id, &manifest).unwrap();
        mem_journal
            .append_event(
                run_id,
                plain_event(
                    EventType::RunStarted,
                    started_payload("term"),
                    None,
                    "2026-11-25T10:00:00Z",
                ),
            )
            .unwrap();
        mem_journal
            .append_event(
                run_id,
                plain_event(terminal_type, json!({}), None, "2026-11-25T10:00:05Z"),
            )
            .unwrap();
        assert!(matches!(
            mem_journal.append_event(
                run_id,
                plain_event(
                    EventType::PhaseEntered,
                    json!({"phase": "late"}),
                    None,
                    "2026-11-25T10:00:06Z",
                )
            ),
            Err(StoreError::AfterTerminal {
                status,
                event_type: EventType::PhaseEntered,
                ..
            }) if status == expected_status
        ));
        let note = mem_journal
            .append_event(
                run_id,
                plain_event(
                    EventType::OperatorRejected,
                    json!({"reason": "post-mortem"}),
                    None,
                    "2026-11-25T10:00:07Z",
                ),
            )
            .unwrap();
        assert_eq!(note.seq, 3);
    }
}

#[test]
fn post_terminal_refusal_on_sqlite_ports() {
    let manifest = sample_manifest("term");
    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("term.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    ports.create_run("sql-term", &manifest).unwrap();
    let (e1, e2, _) = completed_vs_phase_pair("sql-term", "term");
    ports
        .append_envelopes("sql-term", (0, ZERO_HASH), std::slice::from_ref(&e1))
        .unwrap();
    let e3_bad = sealed_event(
        "sql-term",
        3,
        EventType::PhaseEntered,
        json!({"phase": "after"}),
        &e2.event_hash,
        "2026-11-25T10:00:10Z",
    );
    assert!(matches!(
        ports.append_envelopes("sql-term", (1, &e1.event_hash), &[e2.clone(), e3_bad]),
        Err(StoreError::AfterTerminal {
            status: Status::Completed,
            event_type: EventType::PhaseEntered,
            ..
        })
    ));
    assert_eq!(ports.head("sql-term").unwrap().0, 1);

    ports
        .append_envelopes("sql-term", (1, &e1.event_hash), std::slice::from_ref(&e2))
        .unwrap();
    let e3_note = sealed_event(
        "sql-term",
        3,
        EventType::OperatorRejected,
        json!({"reason": "note"}),
        &e2.event_hash,
        "2026-11-25T10:00:11Z",
    );
    ports
        .append_envelopes("sql-term", (2, &e2.event_hash), &[e3_note])
        .unwrap();
    assert_eq!(ports.head("sql-term").unwrap().0, 3);
}

#[test]
fn journal_resolves_stricter_seat_record_version_from_manifest_and_run_started() {
    let dir = tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("manifest-version.db")).unwrap();
    let v1_manifest = json!({"engine": "0.1.0", "bundle_name": "self"});
    let v8_manifest = json!({"engine": "0.15.0", "bundle_name": "self"});

    store
        .create_run("store-v1", "parity", "self", &v1_manifest)
        .unwrap();
    store
        .append_next(
            "store-v1",
            EventType::RunStarted,
            json!({"feature": "parity", "manifest": v8_manifest}),
            None,
            None,
        )
        .unwrap();
    let store_err = store
        .append_next(
            "store-v1",
            EventType::EffectSucceeded,
            v8_only_succeeded_payload(),
            None,
            Some("att-1".to_string()),
        )
        .unwrap_err();
    assert!(matches!(
        store_err,
        StoreError::SeatRecord(ref err) if err.contract == SeatRecordVersion::V1.contract()
    ));

    let journal = sqlite_journal(&store);
    for (run_id, persisted, started) in [
        (
            "port-v1",
            &v1_manifest,
            json!({"feature": "f", "manifest": v8_manifest}),
        ),
        (
            "port-v8-down",
            &v8_manifest,
            json!({"feature": "f", "manifest": v1_manifest}),
        ),
        ("r-no-engine", &v8_manifest, json!({"feature": "no-engine"})),
        (
            "r-empty-manifest",
            &v8_manifest,
            json!({"feature": "empty-manifest", "manifest": {}}),
        ),
    ] {
        journal.create_run(run_id, persisted).unwrap();
        journal
            .append_event(
                run_id,
                plain_event(EventType::RunStarted, started, None, "2026-11-25T10:00:00Z"),
            )
            .unwrap();
        assert_v8_refused_under_v1(&journal, run_id);
        assert!(journal.export_ndjson(run_id).is_ok());
    }

    journal.create_run("port-v8-ok", &v8_manifest).unwrap();
    journal
        .append_event(
            "port-v8-ok",
            plain_event(
                EventType::RunStarted,
                json!({"feature": "v8-ok", "manifest": v8_manifest}),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    let ok = journal
        .append_event("port-v8-ok", v8_succeeded_event())
        .unwrap();
    assert_eq!(ok.seq, 2);
    assert!(journal.export_ndjson("port-v8-ok").is_ok());
}

#[test]
fn settled_members_validation_on_effect_failed_and_indeterminate() {
    let journal = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("v8-panel", &sample_manifest("v8-panel"))
        .unwrap();
    journal
        .append_event(
            "v8-panel",
            plain_event(
                EventType::RunStarted,
                started_payload("v8-panel"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    journal
        .append_event(
            "v8-panel",
            plain_event(
                EventType::EffectCheckpointed,
                panel_member_checkpoint(),
                None,
                "2026-11-25T10:00:01Z",
            ),
        )
        .unwrap();
    let bad_settled = json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "reason": "member failed",
        "settled_members": { "alpha": {"not_a_result": true} }
    });
    for event_type in [EventType::EffectFailed, EventType::EffectIndeterminate] {
        assert!(matches!(
            journal.append_event(
                "v8-panel",
                plain_event(event_type, bad_settled.clone(), None, "2026-11-25T10:00:02Z")
            ),
            Err(StoreError::SeatRecord(ref err))
                if err.seq == 3 && err.settled_member.as_deref().map(String::as_str) == Some("alpha")
        ));
    }
    let good_settled = json!({
        "effect_id": "eff-1",
        "attempt_id": "att-1",
        "reason": "peer failed",
        "settled_members": { "alpha": {"result": "pass", "notes": "ok"} }
    });
    let appended = journal
        .append_event(
            "v8-panel",
            plain_event(
                EventType::EffectFailed,
                good_settled,
                None,
                "2026-11-25T10:00:03Z",
            ),
        )
        .unwrap();
    assert_eq!(appended.seq, 3);
}

#[test]
fn seq_overflow_and_single_head_call_in_append_event() {
    let head_calls = Cell::new(0usize);
    let counting = InterceptingLog {
        inner: MemEventLog::default(),
        on_head: Some(Box::new(|| head_calls.set(head_calls.get() + 1))),
        on_list_run_ids: None,
        on_read_stream: None,
        override_run_ids: None,
    };
    let journal = Journal::new(
        counting,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("cnt-1", &sample_manifest("cnt"))
        .unwrap();
    let e1 = journal
        .append_event(
            "cnt-1",
            plain_event(
                EventType::RunStarted,
                started_payload("cnt"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();
    assert_eq!(head_calls.get(), 1);

    let e2 = journal
        .append_if_head(
            "cnt-1",
            (1, &e1.event_hash),
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "verify"}),
                None,
                "2026-11-25T10:00:01Z",
            ),
        )
        .unwrap();
    assert_eq!(e2.seq, 2);
    assert_eq!(head_calls.get(), 1);

    assert!(matches!(
        journal.append_if_head(
            "cnt-1",
            (u64::MAX, ZERO_HASH),
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "overflow"}),
                None,
                "2026-11-25T10:00:02Z",
            )
        ),
        Err(StoreError::Chain(ChainError::SeqOverflow { after })) if after == u64::MAX
    ));

    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("overflow.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    ports.create_run("ov-1", &sample_manifest("ov")).unwrap();
    let over_i64 = sealed_event(
        "ov-1",
        i64::MAX as u64 + 1,
        EventType::RunStarted,
        started_payload("ov"),
        ZERO_HASH,
        "2026-11-25T10:00:00Z",
    );
    assert!(matches!(
        ports.append_envelopes("ov-1", (0, ZERO_HASH), std::slice::from_ref(&over_i64)),
        Err(StoreError::Chain(ChainError::SeqOverflow { after })) if after == i64::MAX as u64
    ));
}

#[test]
fn journal_refuses_tampered_prefix_and_foreign_run_streams() {
    let mem = MemEventLog::default();
    let journal = Journal::new(
        &mem,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    journal
        .create_run("run-a", &sample_manifest("alpha"))
        .unwrap();
    let e1 = journal
        .append_event(
            "run-a",
            plain_event(
                EventType::RunStarted,
                started_payload("alpha"),
                None,
                "2026-11-25T10:00:00Z",
            ),
        )
        .unwrap();

    mem.tamper_payload("run-a", 0, json!({"feature": "evil"}));
    assert!(matches!(
        journal.append_event(
            "run-a",
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                None,
                "2026-11-25T10:00:01Z",
            )
        ),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));
    assert!(matches!(
        journal.append_if_head(
            "run-a",
            (1, &e1.event_hash),
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "implement"}),
                None,
                "2026-11-25T10:00:02Z",
            )
        ),
        Err(StoreError::Chain(ChainError::BadHash { seq: 1 }))
    ));

    let foreign_e1 = started_event("run-a", "alpha", "2026-11-25T10:00:00Z");
    mem.plant_raw_stream("run-b", sample_manifest("beta"), vec![foreign_e1.clone()]);
    assert!(matches!(
        journal.load_verified("run-b"),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert!(matches!(
        journal.append_event(
            "run-b",
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "verify"}),
                None,
                "2026-11-25T10:00:03Z",
            )
        ),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
    assert!(matches!(
        journal.import_ndjson("run-c", &sample_manifest("c"), &single_ndjson(&foreign_e1)),
        Err(ImportError::Verify(VerifyError::Chain(
            ChainError::ForeignRun { seq: 1 }
        )))
    ));
    mem.create_run("run-c", &sample_manifest("c")).unwrap();
    assert!(matches!(
        mem.append_envelopes("run-c", (0, ZERO_HASH), std::slice::from_ref(&foreign_e1)),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));

    let dir = tempdir().unwrap();
    let store = Store::open(&dir.path().join("foreign.db")).unwrap();
    let ports = SqlitePorts::new(&store);
    ports.create_run("run-c", &sample_manifest("c")).unwrap();
    assert!(matches!(
        ports.append_envelopes("run-c", (0, ZERO_HASH), std::slice::from_ref(&foreign_e1)),
        Err(StoreError::Chain(ChainError::ForeignRun { seq: 1 }))
    ));
}

#[test]
fn journal_seals_and_preserves_attempt_id_through_export_and_import() {
    let dir = tempdir().unwrap();
    let src = Store::open(&dir.path().join("attempt-src.db")).unwrap();
    let journal = sqlite_journal(&src);
    let manifest = sample_manifest("attempt-roundtrip");
    journal.create_run("r-att", &manifest).unwrap();

    let append = |et, payload, cause: Option<&str>, attempt: Option<&str>, ts| {
        journal
            .append_event(
                "r-att",
                NewEvent {
                    event_type: et,
                    payload,
                    causation_id: cause,
                    attempt_id: attempt,
                    ts,
                },
            )
            .unwrap()
    };
    let e1 = append(
        EventType::RunStarted,
        started_payload("attempt-roundtrip"),
        None,
        None,
        "2026-11-25T10:00:00Z",
    );
    let e2 = append(
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        Some(&e1.event_id),
        None,
        "2026-11-25T10:00:01Z",
    );
    let e3 = append(
        EventType::EffectRequested,
        json!({"effect_id": "eff-1", "seat": "implement"}),
        Some(&e2.event_id),
        None,
        "2026-11-25T10:00:02Z",
    );
    let e4 = append(
        EventType::EffectStarted,
        json!({"effect_id": "eff-1", "attempt_id": "att-1"}),
        Some(&e3.event_id),
        Some("att-1"),
        "2026-11-25T10:00:03Z",
    );
    let e5 = journal
        .append_if_head(
            "r-att",
            (e4.seq, &e4.event_hash),
            NewEvent {
                event_type: EventType::EffectCheckpointed,
                payload: valid_checkpoint_payload(),
                causation_id: Some(&e4.event_id),
                attempt_id: Some("att-1"),
                ts: "2026-11-25T10:00:04Z",
            },
        )
        .unwrap();
    let e6 = append(
        EventType::EffectSucceeded,
        json!({
            "effect_id": "eff-1",
            "attempt_id": "att-1",
            "result": {"result": "complete", "notes": "done"}
        }),
        Some(&e5.event_id),
        Some("att-1"),
        "2026-11-25T10:00:05Z",
    );
    assert_eq!(e1.attempt_id, None);
    assert_eq!(e4.attempt_id.as_deref(), Some("att-1"));
    assert_eq!(e5.attempt_id.as_deref(), Some("att-1"));
    assert_eq!(e6.attempt_id.as_deref(), Some("att-1"));

    let ndjson = journal.export_ndjson("r-att").unwrap();
    let mut dst = Store::open(&dir.path().join("attempt-dst.db")).unwrap();
    dst.import_run(&ndjson, &manifest, Path::new("r-att.ndjson"))
        .unwrap();
    let imported = dst.load("r-att").unwrap();
    let ids: Vec<Option<&str>> = imported.iter().map(|e| e.attempt_id.as_deref()).collect();
    assert_eq!(
        ids,
        vec![
            None,
            None,
            None,
            Some("att-1"),
            Some("att-1"),
            Some("att-1")
        ]
    );
}

struct FailingImportLog;

impl EventLog for FailingImportLog {
    fn create_run(&self, _: &str, _: &Value) -> Result<(), StoreError> {
        unreachable!()
    }
    fn append_envelopes(
        &self,
        _: &str,
        _: (u64, &str),
        _: &[EventEnvelope],
    ) -> Result<(), StoreError> {
        unreachable!()
    }
    fn import_run_stream(&self, _: &str, _: &Value, _: &[EventEnvelope]) -> Result<(), StoreError> {
        Err(StoreError::Contended {
            operation: "import_run",
            waited_ms: 1,
        })
    }
    fn head(&self, _: &str) -> Result<(u64, String), StoreError> {
        unreachable!()
    }
    fn read_stream(&self, _: &str, _: u64) -> Result<Vec<EventEnvelope>, StoreError> {
        unreachable!()
    }
    fn manifest(&self, _: &str) -> Result<Value, StoreError> {
        unreachable!()
    }
    fn list_run_ids(&self) -> Result<Vec<String>, StoreError> {
        unreachable!()
    }
}

struct FailOnceProjectionStore<P> {
    inner: P,
    fail_next: Cell<bool>,
}

impl<P: ProjectionStore> ProjectionStore for FailOnceProjectionStore<P> {
    fn apply_envelopes(&self, run_id: &str, envelopes: &[EventEnvelope]) -> Result<(), StoreError> {
        if self.fail_next.replace(false) {
            return Err(StoreError::Contended {
                operation: "apply_envelopes",
                waited_ms: 3,
            });
        }
        self.inner.apply_envelopes(run_id, envelopes)
    }
    fn list_runs(&self) -> Result<Vec<(String, String, String)>, StoreError> {
        self.inner.list_runs()
    }
    fn rebuild_from(&self, log: &dyn EventLog) -> Result<u64, StoreError> {
        self.inner.rebuild_from(log)
    }
}

#[test]
fn post_commit_projection_lag_is_not_contention_and_heals_via_reindex() {
    let mem_src = Journal::new(
        MemEventLog::default(),
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    let manifest = sample_manifest("heal-feat");
    mem_src.create_run("heal-1", &manifest).unwrap();
    mem_src
        .append_event(
            "heal-1",
            plain_event(
                EventType::RunStarted,
                started_payload("heal-feat"),
                None,
                "2026-11-25T11:00:00Z",
            ),
        )
        .unwrap();
    mem_src
        .append_event(
            "heal-1",
            plain_event(
                EventType::PhaseEntered,
                json!({"phase": "verify"}),
                None,
                "2026-11-25T11:00:05Z",
            ),
        )
        .unwrap();
    let ndjson = mem_src.export_ndjson("heal-1").unwrap();

    let dst_journal = Journal::new(
        MemEventLog::default(),
        FailOnceProjectionStore {
            inner: MemProjectionStore::default(),
            fail_next: Cell::new(true),
        },
        MemCoordinationStore::default(),
    );
    let err = dst_journal
        .import_ndjson("heal-1", &manifest, &ndjson)
        .unwrap_err();
    assert!(matches!(
        err,
        ImportError::Store(ref store_err @ StoreError::ProjectionLag { ref run_id, ref source })
            if run_id == "heal-1"
                && !store_err.is_contention()
                && matches!(
                    source.as_ref(),
                    StoreError::Contended {
                        operation: "apply_envelopes",
                        waited_ms: 3,
                    }
                )
    ));
    assert!(matches!(
        dst_journal.import_ndjson("heal-1", &manifest, &ndjson),
        Err(ImportError::Collision(ref id)) if id == "heal-1"
    ));
    assert!(dst_journal.projections().list_runs().unwrap().is_empty());
    assert_eq!(dst_journal.reindex().unwrap(), 2);
    assert_eq!(
        dst_journal.projections().list_runs().unwrap(),
        vec![(
            "heal-1".to_string(),
            "heal-feat".to_string(),
            "2026-11-25T11:00:00Z".to_string(),
        )]
    );

    let contended_journal = Journal::new(
        FailingImportLog,
        MemProjectionStore::default(),
        MemCoordinationStore::default(),
    );
    assert!(matches!(
        contended_journal.import_ndjson("heal-1", &manifest, &ndjson),
        Err(ImportError::Store(StoreError::Contended {
            operation: "import_run",
            ..
        }))
    ));
}
