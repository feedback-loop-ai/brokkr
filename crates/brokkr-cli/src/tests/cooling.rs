//! A cooling run for the readout tests (decision 0103 slice 4f): `r1`'s
//! last event carries one wait, its cool-down table holds the attempt's
//! round, and a probe's later round is the aggregate every readout paints.

use brokkr_core::EventType;
use brokkr_store::{CooldownAppend, CooldownRound, Cooling, RoundAppend, Store};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::tests::running_store;

/// The event's own end.
pub(crate) const COOLING_FIRST: &str = "2099-01-01T00:00:00Z";
/// The aggregate's later end, which every readout must paint.
pub(crate) const COOLING_LATER: &str = "2099-01-01T01:00:00Z";
/// The provider's bounded words.
pub(crate) const COOLING_WORDS: &str = "rate_limit (HTTP 429)";

/// A journal of its own holding the cooling run.
pub(crate) fn cooling_journal() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    cooling_store(&db);
    (dir, db)
}

/// Build the cooling run at `db`, with the attempt's row and a probe's
/// later round.
pub(crate) fn cooling_store(db: &std::path::Path) {
    let event_seq = cooling_attempt(db);
    let mut store = Store::open(db).unwrap();
    extend_cooling(&mut store, event_seq);
}

/// Build the cooling run at `db` through its attempt only: its last event
/// states the wait, its table holds the attempt's row, and no probe round
/// has extended it yet. Answers the event's seq, which [`extend_cooling`]
/// appends the probe's later round to.
pub(crate) fn cooling_attempt(db: &std::path::Path) -> u64 {
    running_store(db, "r1");
    let mut store = Store::open(db).unwrap();
    store
        .append_next(
            "r1",
            EventType::EffectRequested,
            json!({"effect_id": "e", "seat": "work", "phase": "work"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            "r1",
            EventType::EffectStarted,
            json!({"effect_id": "e", "attempt_id": "a"}),
            None,
            None,
        )
        .unwrap();
    let until = OffsetDateTime::parse(COOLING_FIRST, &Rfc3339).unwrap();
    let event = store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: "r1",
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({
                    "effect_id": "e",
                    "error": COOLING_WORDS,
                    "waits": [{
                        "site": "a",
                        "provider": "claude",
                        "route": "anthropic",
                        "words": COOLING_WORDS,
                        "seconds": 600,
                        "until": COOLING_FIRST,
                        "rounds": 1
                    }]
                }),
                causation_id: None,
                attempt_id: Some("a".to_string()),
            },
            &[Cooling {
                site: "a".to_string(),
                provider: "claude".to_string(),
                route: "anthropic".to_string(),
                until_at: until,
                reason: COOLING_WORDS.to_string(),
                retry_after_seconds: None,
                rounds: 1,
                held: None,
            }],
        )
        .unwrap();
    event.envelope.seq
}

/// Append the probe's later round to the attempt at `event_seq`.
pub(crate) fn extend_cooling(store: &mut Store, event_seq: u64) {
    let later = OffsetDateTime::parse(COOLING_LATER, &Rfc3339).unwrap();
    let written = store
        .append_cooldown_round(CooldownRound {
            run_id: "r1",
            event_seq,
            site: "a",
            provider: "claude",
            route: "anthropic",
            until_at: later,
            reason: "probe refused: usage limit",
            retry_after_seconds: Some(3600),
            rounds: 2,
            seconds: 3600,
            ends: None,
        })
        .unwrap();
    assert_eq!(written, RoundAppend::Written);
}

/// The same wait line every surface paints: the provider and route, the
/// aggregate's later end and the provider's words, never the event's first
/// end.
pub(crate) fn assert_painted(printed: &str) {
    assert!(printed.contains("claude/anthropic"), "{printed}");
    assert!(printed.contains(COOLING_WORDS), "{printed}");
    assert!(printed.contains(COOLING_LATER), "{printed}");
    assert!(
        !printed.contains(COOLING_FIRST),
        "the first end is not painted: {printed}"
    );
}
