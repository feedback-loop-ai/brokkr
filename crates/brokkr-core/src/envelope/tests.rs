use super::*;
use serde_json::json;

fn envelope(seq: u64, prev: &str) -> EventEnvelope {
    EventEnvelope {
        run_id: "r1".into(),
        seq,
        event_id: format!("e{seq}"),
        event_schema_version: 1,
        event_type: EventType::PhaseEntered,
        payload: json!({"phase": "intake"}),
        causation_id: None,
        correlation_id: "r1".into(),
        attempt_id: None,
        recorded_at: "2026-08-23T00:00:00Z".into(),
        previous_hash: prev.into(),
        event_hash: String::new(),
    }
    .sealed()
}

#[test]
fn chain_verifies_and_detects_tamper() {
    let e1 = envelope(1, ZERO_HASH);
    let e2 = envelope(2, &e1.event_hash);
    verify_chain(&[e1.clone(), e2.clone()]).unwrap();

    let mut tampered = e1.clone();
    tampered.payload = json!({"phase": "ship"});
    assert_eq!(
        verify_chain(&[tampered, e2.clone()]),
        Err(ChainError::BadHash { seq: 1 })
    );

    let e2_orphan = envelope(2, ZERO_HASH);
    assert_eq!(
        verify_chain(&[e1, e2_orphan]),
        Err(ChainError::BrokenChain {
            seq: 2,
            prev_seq: 1
        })
    );
}

#[test]
fn chain_refuses_every_identity_and_sequence_defect() {
    assert_eq!(verify_chain(&[]), Ok(()));

    let mut gap = envelope(2, ZERO_HASH);
    assert_eq!(
        verify_chain(&[gap.clone()]),
        Err(ChainError::SeqGap {
            seq: 2,
            expected: 1,
        })
    );

    gap.seq = 1;
    gap.event_schema_version = 2;
    gap = gap.sealed();
    assert_eq!(
        verify_chain(&[gap]),
        Err(ChainError::BadSchemaVersion { seq: 1, found: 2 })
    );

    let first = envelope(1, ZERO_HASH);
    let mut foreign = envelope(2, &first.event_hash);
    foreign.run_id = "other-run".into();
    foreign = foreign.sealed();
    assert_eq!(
        verify_chain(&[first, foreign]),
        Err(ChainError::ForeignRun { seq: 2 })
    );
}
