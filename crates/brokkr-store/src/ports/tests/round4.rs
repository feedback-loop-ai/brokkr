//! Round 4 regression tests for decision 0126 Slice 1 (#1362):
//! full-chain co-located verification on `rebuild_from` and `apply_envelopes`,
//! `runs <-> run_summary` invariants across empty runs and skewed `created_at`,
//! explicit `stream_ordinal` parity on decoupled SQLite projections, and
//! decoupled prefix completeness, feature consistency, and monotonic rebuild.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::fold::Status;
use rusqlite::{params, Connection};
use serde_json::json;
use tempfile::tempdir;

use super::regressions::{sealed_event, sqlite_table_count};
use super::round3::{started_event, summary_snapshot};
use super::{query_run_pairs, sample_manifest, started_payload, MemEventLog};
use crate::ports::{EventLog, ProjectionStore, SqlitePorts};
use crate::{Store, StoreError};

fn event_index_ordinals(path: &std::path::Path, run_id: &str) -> Vec<(i64, Option<i64>)> {
    query_run_pairs(
        path,
        "SELECT seq, stream_ordinal FROM event_index WHERE run_id = ?1 ORDER BY seq",
        run_id,
    )
}

pub(super) fn completed_vs_phase_pair(
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

// ---------------------------------------------------------------------------
// HIGH #1 & #2: Full-chain co-located verification on rebuild_from & apply_envelopes
// ---------------------------------------------------------------------------

#[test]
fn colocated_rebuild_from_refuses_empty_local_run_vs_nonempty_external_log() {
    let dir = tempdir().unwrap();
    let local_path = dir.path().join("empty-local.db");
    let local_store = Store::open(&local_path).unwrap();
    let local_ports = SqlitePorts::new(&local_store);
    let manifest = sample_manifest("empty-vs-full");
    local_ports.create_run("r1", &manifest).unwrap();

    let (e1, completed, _) = completed_vs_phase_pair("r1", "empty-vs-full");
    let external = MemEventLog::default();
    external
        .import_run_stream("r1", &manifest, &[e1, completed])
        .unwrap();

    assert!(matches!(
        local_ports.rebuild_from(&external),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "r1"
    ));
    let (head_seq, folded_seq, _, _, _) = summary_snapshot(&local_path, "r1");
    assert_eq!((head_seq, folded_seq), (0, Some(0)));
}

#[test]
fn colocated_rebuild_from_refuses_tampered_local_event_with_matching_external_hash() {
    let dir = tempdir().unwrap();
    let local_path = dir.path().join("tampered-local.db");
    let local_store = Store::open(&local_path).unwrap();
    let local_ports = SqlitePorts::new(&local_store);
    let manifest = sample_manifest("tamper-fence");
    let (e1, completed, external_phase) = completed_vs_phase_pair("t1", "tamper-fence");

    local_ports
        .import_run_stream("t1", &manifest, &[e1.clone(), completed.clone()])
        .unwrap();
    local_ports
        .apply_envelopes("t1", &[e1.clone(), completed.clone()])
        .unwrap();

    let external = MemEventLog::default();
    external
        .import_run_stream("t1", &manifest, &[e1, external_phase.clone()])
        .unwrap();

    // Case 1: forge SQL `events.event_hash` column to match external non-terminal hash
    // while `events.envelope` still holds the genuine RunCompleted event.
    let raw = Connection::open(&local_path).unwrap();
    raw.execute_batch("DROP TRIGGER events_append_only_update;")
        .unwrap();
    raw.execute(
        "UPDATE events SET event_hash = ?1 WHERE run_id = 't1' AND seq = 2",
        params![external_phase.event_hash],
    )
    .unwrap();
    assert!(matches!(
        local_ports.rebuild_from(&external),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "t1"
    ));
    assert_eq!(
        summary_snapshot(&local_path, "t1").2.as_deref(),
        Some("completed")
    );

    // Case 2: also forge `event_hash` inside `events.envelope` JSON to match external seq 2.
    let mut forged = completed;
    forged.event_hash = external_phase.event_hash;
    raw.execute(
        "UPDATE events SET envelope = ?1 WHERE run_id = 't1' AND seq = 2",
        params![serde_json::to_string(&forged).unwrap()],
    )
    .unwrap();
    drop(raw);

    assert!(matches!(
        local_ports.rebuild_from(&external),
        Err(StoreError::Chain(ChainError::BadHash { seq: 2 }))
    ));
    assert_eq!(
        summary_snapshot(&local_path, "t1").2.as_deref(),
        Some("completed")
    );
}

#[test]
fn colocated_apply_envelopes_refuses_divergent_stream_and_preserves_terminal_fence() {
    let dir = tempdir().unwrap();
    let local_path = dir.path().join("coloc-apply.db");
    let local_store = Store::open(&local_path).unwrap();
    let local_ports = SqlitePorts::new(&local_store);
    let manifest = sample_manifest("fence-apply");
    let (e1, completed, divergent_phase) = completed_vs_phase_pair("ca-1", "fence-apply");

    local_ports
        .import_run_stream("ca-1", &manifest, &[e1.clone(), completed.clone()])
        .unwrap();
    // Before run_summary is folded: divergent apply_envelopes must refuse.
    assert!(matches!(
        local_ports.apply_envelopes("ca-1", &[e1.clone(), divergent_phase.clone()]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-1"
    ));
    // Valid apply_envelopes folds the completed run; subsequent divergent call still refuses.
    local_ports
        .apply_envelopes("ca-1", &[e1.clone(), completed.clone()])
        .unwrap();
    assert!(matches!(
        local_ports.apply_envelopes("ca-1", &[e1.clone(), divergent_phase]),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-1"
    ));
    assert_eq!(
        summary_snapshot(&local_path, "ca-1").2.as_deref(),
        Some("completed")
    );

    // Terminal fence still rejects non-annotation appends after seq 2.
    let after_terminal = sealed_event(
        "ca-1",
        3,
        EventType::PhaseEntered,
        json!({"phase": "illegal"}),
        &completed.event_hash,
        "2026-11-25T10:00:10Z",
    );
    assert!(matches!(
        local_ports.append_envelopes(
            "ca-1",
            (2, &completed.event_hash),
            std::slice::from_ref(&after_terminal),
        ),
        Err(StoreError::AfterTerminal {
            ref run_id,
            status: Status::Completed,
            event_type: EventType::PhaseEntered,
        }) if run_id == "ca-1"
    ));

    // Empty local run in `runs` also refuses non-empty apply_envelopes.
    local_ports.create_run("ca-empty", &manifest).unwrap();
    let empty_e1 = started_event("ca-empty", "fence-apply", "2026-11-25T10:00:00Z");
    assert!(matches!(
        local_ports.apply_envelopes("ca-empty", std::slice::from_ref(&empty_e1)),
        Err(StoreError::DivergentEventLog { ref run_id }) if run_id == "ca-empty"
    ));
}

// ---------------------------------------------------------------------------
// HIGH #3: Co-located rebuild_from preserves empty runs & runs.created_at
// ---------------------------------------------------------------------------

#[test]
fn colocated_rebuild_from_preserves_empty_runs_created_at_and_subsequent_append() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("coloc-invariants.db");
    let mut store = Store::open(&path).unwrap();
    {
        let ports = SqlitePorts::new(&store);
        ports
            .create_run("empty-local", &sample_manifest("empty"))
            .unwrap();
        ports
            .create_run("skew-1", &sample_manifest("skew"))
            .unwrap();
        let e1 = started_event("skew-1", "skew", "2020-01-01T00:00:00Z");
        ports
            .append_envelopes("skew-1", (0, ZERO_HASH), &[e1])
            .unwrap();

        let raw = Connection::open(&path).unwrap();
        raw.execute(
            "UPDATE runs SET created_at = '2025-06-01T00:00:00Z' WHERE run_id = 'skew-1'",
            [],
        )
        .unwrap();
        drop(raw);

        assert_eq!(ports.rebuild_from(&ports).unwrap(), 1);
        let raw = Connection::open(&path).unwrap();
        let empty_seat_head: (i64, String) = raw
            .query_row(
                "SELECT folded_seq, folded_hash FROM seat_head WHERE run_id = 'empty-local'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(empty_seat_head, (0, ZERO_HASH.to_string()));
        drop(raw);
        assert!(store.summary_disagreements().unwrap().is_empty());
        assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());
        assert!(ports
            .list_runs()
            .unwrap()
            .iter()
            .any(|(id, _, _)| id == "empty-local"));
    }

    store
        .append_next(
            "empty-local",
            EventType::RunStarted,
            started_payload("empty"),
            None,
            None,
        )
        .unwrap();
    let ports = SqlitePorts::new(&store);
    assert!(store.summary_disagreements().unwrap().is_empty());
    assert_eq!(summary_snapshot(&path, "empty-local").0, 1);
    assert_eq!(ports.list_runs().unwrap(), store.list_runs().unwrap());
}

// ---------------------------------------------------------------------------
// MEDIUM #1: Explicit stream_ordinal parity on decoupled SqlitePorts
// ---------------------------------------------------------------------------

fn ordinal_test_stream(run_id: &str) -> Vec<EventEnvelope> {
    let mut events = vec![started_event(run_id, "ordinals", "2026-11-25T10:00:00Z")];
    let payloads = [
        json!({"effect_id": "eff-1", "attempt_id": "att-1", "checkpoint": {"step": "seat-turn", "turn": 1, "member": "alice"}}),
        json!({"effect_id": "eff-1", "attempt_id": "att-1", "checkpoint": {"step": "seat-turn", "turn": 2, "member": "alice"}}),
        json!({"effect_id": "eff-2", "attempt_id": "att-1", "checkpoint": {"step": "seat-turn", "turn": 1, "member": "alice"}}),
        json!({"effect_id": "eff-1", "attempt_id": "att-1", "checkpoint": {"step": "seat-turn", "turn": 1}}),
        json!({"effect_id": "eff-1", "attempt_id": "att-1", "result": {"member": "alice", "outcome": "ok"}}),
        json!({"effect_id": "eff-1", "checkpoint": {"step": "seat-turn", "turn": 1, "member": "alice"}}),
    ];
    for (idx, payload) in payloads.into_iter().enumerate() {
        let prev = events.last().unwrap().event_hash.clone();
        let seq = (idx + 2) as u64;
        let ts = format!("2026-11-25T10:00:0{seq}Z");
        events.push(sealed_event(
            run_id,
            seq,
            EventType::EffectCheckpointed,
            payload,
            &prev,
            &ts,
        ));
    }
    events
}

#[test]
fn decoupled_event_index_stream_ordinal_matches_colocated_store() {
    let dir = tempdir().unwrap();
    let run_id = "ord-1";
    let manifest = sample_manifest("ordinals");
    let stream = ordinal_test_stream(run_id);

    let coloc_path = dir.path().join("ord-coloc.db");
    let coloc_store = Store::open(&coloc_path).unwrap();
    let coloc_ports = SqlitePorts::new(&coloc_store);
    coloc_ports
        .import_run_stream(run_id, &manifest, &stream)
        .unwrap();
    let expected = event_index_ordinals(&coloc_path, run_id);
    assert_eq!(
        expected,
        vec![
            (1, None),
            (2, Some(1)),
            (3, Some(2)),
            (4, Some(1)),
            (5, Some(1)),
            (6, Some(0)),
            (7, Some(0)),
        ]
    );

    let dec_path = dir.path().join("ord-dec.db");
    let dec_store = Store::open(&dec_path).unwrap();
    let dec_ports = SqlitePorts::new(&dec_store);
    dec_ports.apply_envelopes(run_id, &stream).unwrap();
    assert_eq!(event_index_ordinals(&dec_path, run_id), expected);

    let mem = MemEventLog::default();
    mem.import_run_stream(run_id, &manifest, &stream).unwrap();
    assert_eq!(dec_ports.rebuild_from(&mem).unwrap(), 7);
    assert_eq!(event_index_ordinals(&dec_path, run_id), expected);
}

// ---------------------------------------------------------------------------
// MEDIUM #3: Decoupled prefix completeness, feature consistency, monotonic rebuild
// ---------------------------------------------------------------------------

#[test]
fn decoupled_feature_consistency_and_monotonic_rebuild_from() {
    let dir = tempdir().unwrap();
    let dec_path = dir.path().join("dec-mono.db");
    let dec_store = Store::open(&dec_path).unwrap();
    let dec_ports = SqlitePorts::new(&dec_store);

    // Feature consistency: RunStarted payload feature wins over manifest feature
    // in both apply_envelopes and rebuild_from; rebuild_from falls back cleanly to manifest.
    let f1 = started_event("feat-1", "from-started", "2026-11-25T10:00:00Z");
    let f2 = sealed_event(
        "feat-1",
        2,
        EventType::PhaseEntered,
        json!({"phase": "implement"}),
        &f1.event_hash,
        "2026-11-25T10:00:05Z",
    );
    let fallback_e1 = sealed_event(
        "feat-2",
        1,
        EventType::RunStarted,
        json!({}),
        ZERO_HASH,
        "2026-11-25T10:01:00Z",
    );
    let stale_e1 = started_event("stale-1", "stale", "2026-11-25T09:00:00Z");

    // Seed decoupled projections with feat-1 at seq 2 and stale-1 at seq 1.
    dec_ports
        .apply_envelopes("feat-1", &[f1.clone(), f2.clone()])
        .unwrap();
    dec_ports
        .apply_envelopes("stale-1", std::slice::from_ref(&stale_e1))
        .unwrap();
    assert_eq!(dec_ports.list_runs().unwrap()[1].1, "from-started");

    // Stale log only has feat-1 at seq 1 (older than projected seq 2) plus feat-2 (new),
    // and omits stale-1.
    let stale_log = MemEventLog::default();
    stale_log
        .import_run_stream(
            "feat-1",
            &sample_manifest("from-manifest"),
            std::slice::from_ref(&f1),
        )
        .unwrap();
    stale_log
        .import_run_stream(
            "feat-2",
            &sample_manifest("fallback-manifest"),
            std::slice::from_ref(&fallback_e1),
        )
        .unwrap();

    // rebuild_from must NOT regress feat-1 back to seq 1, MUST clear stale-1,
    // and MUST project feat-2 with manifest fallback.
    assert_eq!(dec_ports.rebuild_from(&stale_log).unwrap(), 2);
    let (head_seq, folded_seq, _, _, _) = summary_snapshot(&dec_path, "feat-1");
    assert_eq!((head_seq, folded_seq), (2, Some(2)));
    assert_eq!(sqlite_table_count(&dec_path, "event_index"), 3);
    assert_eq!(
        dec_ports.list_runs().unwrap(),
        vec![
            (
                "feat-1".to_string(),
                "from-started".to_string(),
                "2026-11-25T10:00:00Z".to_string(),
            ),
            (
                "feat-2".to_string(),
                "fallback-manifest".to_string(),
                "2026-11-25T10:01:00Z".to_string(),
            ),
        ]
    );

    // Rebuilding from a caught-up log still derives "from-started" for feat-1.
    stale_log
        .append_envelopes("feat-1", (1, &f1.event_hash), &[f2])
        .unwrap();
    assert_eq!(dec_ports.rebuild_from(&stale_log).unwrap(), 3);
    assert_eq!(dec_ports.list_runs().unwrap()[0].1, "from-started");
}
