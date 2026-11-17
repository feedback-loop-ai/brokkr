//! A trail read that ends in a repaired tail, its repair record outside
//! the read: a run's opening and a late `effect/failed` carrying one
//! provider wait. Read as the journal's last row it would paint a wait;
//! read as the fold left it, it is a late tail and stands none (decision
//! 0108 ruling 4; decision 0103 slice 4f).
//!
//! Each test binary includes this file once through `#[path]`; it is test
//! code by its path, so it stays outside the coverage denominator.

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{EventEnvelope, EventType, EVENT_SCHEMA_VERSION};
use serde_json::{json, Value};

/// The trail's two events: the opening at seq 1, the late wait at seq 8.
pub(crate) fn trail(run: &str) -> [EventEnvelope; 2] {
    let at = "2026-11-13T00:00:00Z";
    let started = envelope(
        run,
        1,
        EventType::RunStarted,
        json!({"feature": "f", "manifest": {}}),
        at,
        ZERO_HASH,
    );
    let failed = envelope(
        run,
        8,
        EventType::EffectFailed,
        json!({"effect_id": "late", "waits": [{
            "site": "a", "provider": "claude", "route": "anthropic",
            "words": "rate_limit", "seconds": 600,
            "until": "2026-11-13T00:10:00Z", "rounds": 1
        }]}),
        at,
        &started.event_hash,
    );
    [started, failed]
}

/// The repaired tail those events stand in: seq 8, named by the repair
/// record outside this read.
pub(crate) fn tail() -> brokkr_core::fold::LateTail {
    brokkr_core::fold::LateTail {
        repaired_at: 9,
        terminal_seq: 7,
        first_seq: 8,
        last_seq: 8,
        event_types: vec![EventType::EffectFailed],
        operator: "op".to_string(),
    }
}

/// One sealed event at `seq`, chained onto `previous`.
fn envelope(
    run: &str,
    seq: u64,
    event_type: EventType,
    payload: Value,
    at: &str,
    previous: &str,
) -> EventEnvelope {
    EventEnvelope {
        run_id: run.to_string(),
        seq,
        event_id: format!("{run}-{seq}"),
        event_schema_version: EVENT_SCHEMA_VERSION,
        event_type,
        payload,
        causation_id: None,
        correlation_id: run.to_string(),
        attempt_id: None,
        recorded_at: at.to_string(),
        previous_hash: previous.to_string(),
        event_hash: String::new(),
    }
    .sealed()
}
