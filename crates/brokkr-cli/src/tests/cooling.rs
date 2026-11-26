//! A cooling run for the readout tests (decision 0103 slice 4f): `r1`'s
//! last event carries one wait, its cool-down table holds the attempt's
//! round, and a probe's later round is the aggregate every readout paints.

use brokkr_core::{EventType, Held};
use brokkr_store::{CooldownAppend, CooldownRound, Cooling, RoundAppend, Store};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::tests::running_store;

/// The event's own end.
pub(crate) const COOLING_FIRST: &str = "2099-01-01T00:00:00Z";
/// The aggregate's later end, which every readout must paint.
pub(crate) const COOLING_LATER: &str = "2099-01-01T01:00:00Z";
/// The second site's original end, before its probe extends it.
pub(crate) const HELD_FIRST: &str = "2099-01-01T00:30:00Z";
pub(crate) const HELD_MAX: u64 = 300;
pub(crate) const PROBE_MAX: u64 = 400;
/// The provider's bounded words.
pub(crate) const COOLING_WORDS: &str = "rate_limit (HTTP 429)";

/// A journal of its own holding the cooling run.
pub(crate) fn cooling_journal() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    cooling_store(&db);
    (dir, db)
}

/// A journal whose ordinary and held sites print together.
pub(crate) fn mixed_cooling_journal() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let seq = mixed_cooling_attempt(&db);
    extend_held_cooling(&mut Store::open(&db).unwrap(), seq, "b", "backup");
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
    attempt(db, [wait("a", "anthropic", COOLING_FIRST, None)])
}

pub(crate) fn mixed_cooling_attempt(db: &std::path::Path) -> u64 {
    attempt(
        db,
        [
            wait("a", "anthropic", COOLING_FIRST, None),
            wait(
                "b",
                "backup",
                HELD_FIRST,
                Some(Held {
                    max_seconds: HELD_MAX,
                }),
            ),
        ],
    )
}

fn wait(site: &str, route: &str, until: &str, held: Option<Held>) -> (Value, Cooling) {
    let mut payload = json!({
        "site": site, "provider": "claude", "route": route, "words": COOLING_WORDS,
        "seconds": 600, "until": until, "rounds": 1
    });
    if let Some(held) = held {
        payload["held"] = json!(true);
        payload["max_seconds"] = json!(held.max_seconds);
        payload["retry_after_seconds"] = json!(600);
    }
    let row = Cooling {
        site: site.to_string(),
        provider: "claude".to_string(),
        route: route.to_string(),
        until_at: OffsetDateTime::parse(until, &Rfc3339).unwrap(),
        reason: COOLING_WORDS.to_string(),
        retry_after_seconds: held.map(|_| 600),
        rounds: 1,
        held,
    };
    (payload, row)
}

fn attempt(db: &std::path::Path, sites: impl IntoIterator<Item = (Value, Cooling)>) -> u64 {
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
    let (sites, rows): (Vec<_>, Vec<_>) = sites.into_iter().unzip();
    let event = store
        .append_with_cooldowns(
            CooldownAppend {
                run_id: "r1",
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({
                    "effect_id": "e",
                    "error": COOLING_WORDS,
                    "waits": sites
                }),
                causation_id: None,
                attempt_id: Some("a".to_string()),
            },
            &rows,
        )
        .unwrap();
    event.envelope.seq
}

/// Append the probe's later round to the attempt at `event_seq`.
pub(crate) fn extend_cooling(store: &mut Store, event_seq: u64) {
    let written = store
        .append_cooldown_round(probe_round(event_seq, "a", "anthropic"))
        .unwrap();
    assert_eq!(written, RoundAppend::Written);
}

/// Append a held probe round for `site` at the same terminal event.
pub(crate) fn extend_held_cooling(store: &mut Store, seq: u64, site: &str, route: &str) {
    let written = store
        .append_held_round(
            probe_round(seq, site, route),
            Held {
                max_seconds: PROBE_MAX,
            },
        )
        .unwrap();
    assert_eq!(written, RoundAppend::Written);
}

pub(crate) fn probe_round<'a>(event_seq: u64, site: &'a str, route: &'a str) -> CooldownRound<'a> {
    CooldownRound {
        run_id: "r1",
        event_seq,
        site,
        provider: "claude",
        route,
        until_at: OffsetDateTime::parse(COOLING_LATER, &Rfc3339).unwrap(),
        reason: "probe refused: usage limit",
        retry_after_seconds: Some(3600),
        rounds: 2,
        seconds: 3600,
        ends: None,
    }
}

/// Both sites print their own kind and driver after their effective end.
pub(crate) fn assert_painted(printed: &str) {
    let ordinary = format!(
        "claude/anthropic at a until {COOLING_FIRST} · ordinary · the engine: {COOLING_WORDS}"
    );
    let held = format!(
        "claude/backup at b until {COOLING_LATER} · held (maximum {PROBE_MAX}s) · brokkr resume: {COOLING_WORDS}"
    );
    assert!(printed.contains(&ordinary), "{printed}");
    assert!(printed.contains(&held), "{printed}");
}
