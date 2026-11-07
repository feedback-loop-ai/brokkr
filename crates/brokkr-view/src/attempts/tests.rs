//! The attempts readout (decision 0089 ruling 7): the sites brokkr-core
//! reads, each its own start's, and a re-opened participant's session flag
//! as main reads it, so the usage it reports is main's.

use super::*;
use crate::tests::ev;
use brokkr_core::envelope::EventType;
use serde_json::json;

/// One run whose attempt `a`/`m` finishes a session, fails, and starts
/// again to finish another: the two records' output, 100 then 200.
fn reopened() -> Vec<EventEnvelope> {
    vec![
        ev(
            1,
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {}}),
            "2026-01-01T00:00:00Z",
        ),
        ev(
            2,
            EventType::PhaseEntered,
            json!({"phase": "work"}),
            "2026-01-01T00:00:00Z",
        ),
        ev(
            3,
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
            "2026-01-01T00:00:01Z",
        ),
        ev(
            4,
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}]}),
            "2026-01-01T00:00:02Z",
        ),
        ev(
            5,
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "checkpoint": {"member": "m", "step": "claude-session-finished",
                                  "output_tokens": 100}}),
            "2026-01-01T00:00:03Z",
        ),
        ev(
            6,
            EventType::EffectFailed,
            json!({"effect_id": "fx", "attempt_id": "a"}),
            "2026-01-01T00:00:04Z",
        ),
        ev(
            7,
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}]}),
            "2026-01-01T00:00:05Z",
        ),
        ev(
            8,
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "checkpoint": {"member": "m", "step": "claude-session-finished",
                                  "output_tokens": 200}}),
            "2026-01-01T00:00:06Z",
        ),
    ]
}

/// A re-opened attempt is one participant, so its later session record moves
/// the flag off the earlier, as main reads it: the readout's flags are
/// `[false, true]` and only the later record's 200 output counts.
#[test]
fn a_reopened_session_keeps_main_s_usage() {
    let events = reopened();
    let flags: Vec<bool> = checkpoints(&events)
        .into_iter()
        .map(|checkpoint| checkpoint.session)
        .collect();
    assert_eq!(flags, [false, true]);
    let read = attempts(&events);
    assert_eq!(read.sites.len(), 2);
    let keeps = |_: i64, _: Option<&str>| true;
    assert_eq!(read.sites[0].usage(&keeps), None);
    assert_eq!(
        read.sites[1]
            .usage(&keeps)
            .and_then(|usage| usage.output_tokens),
        Some(200)
    );
}
