use super::*;
use crate::envelope_builder::EnvelopeBuilder;
use serde_json::json;

fn envelope(seq: u64, prev: &str) -> EventEnvelope {
    EnvelopeBuilder::new(EventType::PhaseEntered, json!({"phase": "intake"}))
        .run("r1")
        .seq(seq)
        .event_id(format!("e{seq}"))
        .at("2026-08-23T00:00:00Z")
        .previous(prev)
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

/// `as_str` is the name serde writes and reads, for every listed type.
/// The table is listed by hand; the match below has no wildcard, so a
/// new type stops this test compiling and points its author at the table.
#[test]
fn every_event_type_names_itself_as_serde_does() {
    use EventType::*;
    for (event_type, name) in [
        (RunStarted, "run/started"),
        (PhaseEntered, "phase/entered"),
        (EffectRequested, "effect/requested"),
        (EffectStarted, "effect/started"),
        (EffectCheckpointed, "effect/checkpointed"),
        (EffectSucceeded, "effect/succeeded"),
        (EffectFailed, "effect/failed"),
        (EffectIndeterminate, "effect/indeterminate"),
        (TransitionDecided, "transition/decided"),
        (OperatorCommanded, "operator/commanded"),
        (OperatorAccepted, "operator/accepted"),
        (OperatorRejected, "operator/rejected"),
        (RunParked, "run/parked"),
        (RunCompleted, "run/completed"),
        (RunStopped, "run/stopped"),
    ] {
        match event_type {
            RunStarted | PhaseEntered | EffectRequested | EffectStarted | EffectCheckpointed
            | EffectSucceeded | EffectFailed | EffectIndeterminate | TransitionDecided
            | OperatorCommanded | OperatorAccepted | OperatorRejected | RunParked
            | RunCompleted | RunStopped => {}
        }
        assert_eq!(event_type.as_str(), name);
        assert_eq!(serde_json::to_value(event_type).unwrap(), json!(name));
        assert_eq!(
            serde_json::from_value::<EventType>(json!(name)).unwrap(),
            event_type
        );
    }
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
    let refusal = verify_chain(&[gap]).unwrap_err();
    assert_eq!(refusal, ChainError::BadSchemaVersion { seq: 1, found: 2 });
    assert_eq!(
        refusal.to_string(),
        "event 1: event_schema_version 2 is not supported (want 1)"
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

#[test]
fn a_suffix_verifies_against_the_head_it_continues() {
    let e1 = envelope(1, ZERO_HASH);
    let e2 = envelope(2, &e1.event_hash);
    let e3 = envelope(3, &e2.event_hash);
    let suffix = [e2.clone(), e3.clone()];
    let last = std::slice::from_ref(&e3);
    assert_eq!(verify_chain_after("r1", 1, &e1.event_hash, &suffix), Ok(()));
    assert_eq!(verify_chain_after("r1", 3, &e3.event_hash, &[]), Ok(()));
    assert_eq!(
        verify_chain_after("r1", 1, &e1.event_hash, last),
        Err(ChainError::SeqGap {
            seq: 3,
            expected: 2
        })
    );
    assert_eq!(
        verify_chain_after("r1", 1, ZERO_HASH, &suffix),
        Err(ChainError::BrokenChain {
            seq: 2,
            prev_seq: 1
        })
    );
    assert_eq!(
        verify_chain_after("r1", 2, &e1.event_hash, last),
        Err(ChainError::BrokenChain {
            seq: 3,
            prev_seq: 2
        })
    );
    assert_eq!(
        verify_chain_after("r2", 1, &e1.event_hash, &suffix),
        Err(ChainError::ForeignRun { seq: 2 })
    );
}

/// A head at the top of `u64` has no successor: the suffix refuses
/// rather than panicking or wrapping to seq 0, and so does the event
/// after one that took the last seq.
#[test]
fn a_suffix_past_the_last_seq_refuses_instead_of_wrapping() {
    let wrapped = envelope(0, ZERO_HASH);
    assert_eq!(
        verify_chain_after("r1", u64::MAX, ZERO_HASH, std::slice::from_ref(&wrapped)),
        Err(ChainError::SeqOverflow { after: u64::MAX })
    );
    let last = envelope(u64::MAX, ZERO_HASH);
    let after = envelope(0, &last.event_hash);
    assert_eq!(
        verify_chain_after("r1", u64::MAX - 1, ZERO_HASH, &[last, after]),
        Err(ChainError::SeqOverflow { after: u64::MAX })
    );
}
