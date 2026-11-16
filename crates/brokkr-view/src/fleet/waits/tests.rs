//! The waits' one derivation (decision 0103 slice 4f): the event's waits
//! with the aggregate's later end, a wait that has ended, a wait holding
//! the staleness bound, and a present entry that refuses by name.

use super::*;

use crate::fleet::{need, section_of, Section, Standing};
use crate::tests::late_wait;
use crate::tests::{ev, listed, state};
use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use serde_json::json;

const AT: &str = "2026-01-02T00:00:00Z";
const LAST: &str = "2026-01-01T23:59:00Z";
const FIRST_END: &str = "2026-01-02T00:10:00Z";
const LATER_END: &str = "2026-01-02T00:20:00Z";
const SHORTER_END: &str = "2026-01-02T00:15:00Z";
const WORDS: &str = "rate_limit (HTTP 429)";

/// One terminal event carrying `waits` as the extension writes them.
fn failed(waits: serde_json::Value) -> EventEnvelope {
    ev(
        8,
        EventType::EffectFailed,
        json!({"effect_id": "e", "waits": waits}),
        LAST,
    )
}

/// One wait's entry, ends at `until`, provider and route as named.
fn entry(site: &str, provider: &str, route: &str, until: &str, words: &str) -> serde_json::Value {
    json!({
        "site": site,
        "provider": provider,
        "route": route,
        "words": words,
        "seconds": 600,
        "until": until,
        "rounds": 1
    })
}

/// The event's two waits: claude on anthropic, codex on openai.
fn two_waits() -> EventEnvelope {
    failed(json!([
        entry("a", "claude", "anthropic", FIRST_END, WORDS),
        entry("b", "codex", "openai", FIRST_END, "usage limit"),
    ]))
}

/// The row of `event` named `r1`, its journal last moved at `LAST`.
fn row_of(event: &EventEnvelope, cooling: &[Cooling]) -> crate::RunRow {
    let mut entry = listed("r1", "feature", "2026-01-01T00:00:00Z", None);
    let running = state(Some("work"), Status::Running, None);
    entry.state = Some(&running);
    entry.last_recorded_at = Some(LAST);
    entry.waits = WaitFact::of(Some(event));
    crate::run_rows(&[entry], cooling).runs.remove(0)
}

/// One cooling row for `provider` and `route`, ending at `until`.
fn cooling(provider: &str, route: &str, until: &str) -> Cooling {
    Cooling {
        provider: provider.to_string(),
        route: route.to_string(),
        until: until.to_string(),
    }
}

#[test]
fn a_last_event_with_waits_reads_running_with_every_wait_and_its_words() {
    let event = two_waits();
    let row = row_of(&event, &[]);
    assert_eq!(row.status_word(), "running");
    assert_eq!(row.verdict.standing, Standing::Running);
    assert_eq!(row.waits.waits.len(), 2);
    assert_eq!(row.waits.waits[0].site.as_deref(), Some("a"));
    assert_eq!(row.waits.waits[0].provider, "claude");
    assert_eq!(row.waits.waits[0].route, "anthropic");
    assert_eq!(row.waits.waits[0].until, FIRST_END);
    assert_eq!(row.waits.waits[0].words, WORDS);
    assert_eq!(row.waits.waits[1].provider, "codex");
    assert_eq!(
        row.waits.lines(AT),
        vec![
            format!("wait      claude/anthropic at a until {FIRST_END}: {WORDS}"),
            "wait      codex/openai at b until 2026-01-02T00:10:00Z: usage limit".to_string(),
        ]
    );
    assert!(need(&row, AT).is_none(), "a wait is not a park");
    assert_eq!(section_of(&row, AT), Section::Running);
}

#[test]
fn a_probe_s_later_round_is_the_end_and_a_shorter_row_never_shortens_it() {
    let event = failed(json!([entry("a", "claude", "anthropic", FIRST_END, WORDS)]));
    let cooling = [
        cooling("claude", "anthropic", LATER_END),
        cooling("claude", "anthropic", SHORTER_END),
        cooling("codex", "openai", "2026-01-03T00:00:00Z"),
    ];
    let row = row_of(&event, &cooling);
    let wait = &row.waits.waits[0];
    assert_eq!(
        wait.until, LATER_END,
        "the aggregate's latest end, not the shorter row"
    );
    assert_eq!(
        row.waits.lines(AT),
        vec![format!(
            "wait      claude/anthropic at a until {LATER_END}: {WORDS}"
        )]
    );
}

#[test]
fn an_earlier_aggregate_never_shortens_a_wait() {
    let event = failed(json!([entry("a", "claude", "anthropic", FIRST_END, WORDS)]));
    let earlier = "2026-01-02T00:05:00Z";
    let row = row_of(&event, &[cooling("claude", "anthropic", earlier)]);
    assert_eq!(
        row.waits.waits[0].until, FIRST_END,
        "the event's own end outlasts an earlier aggregate row"
    );
}

#[test]
fn an_ended_wait_still_reads_as_the_wait_and_says_it_has_ended() {
    let event = failed(json!([entry("a", "claude", "anthropic", FIRST_END, WORDS)]));
    let row = row_of(&event, &[]);
    assert_eq!(row.verdict.standing, Standing::Running);
    assert_eq!(row.waits.waits.len(), 1, "the ended wait is still the wait");
    assert_eq!(
        row.waits.lines("2026-01-02T00:25:00Z"),
        vec![format!(
            "wait      claude/anthropic at a until {FIRST_END} (ended): {WORDS}"
        )]
    );
    assert!(
        need(&row, "2026-01-02T00:25:00Z").is_none(),
        "not needs you"
    );
    assert_eq!(section_of(&row, "2026-01-02T00:25:00Z"), Section::Running);
}

#[test]
fn a_wait_holds_the_staleness_bound_until_its_end_passes() {
    let event = failed(json!([entry(
        "a",
        "claude",
        "anthropic",
        "2026-01-02T05:00:00Z",
        WORDS
    )]));
    let row = row_of(&event, &[]);
    // Three and a half hours of silence, past the bound, but the wait
    // has not ended: still running, and the guard is what holds it.
    assert!(need(&row, "2026-01-02T03:30:00Z").is_none());
    assert_eq!(section_of(&row, "2026-01-02T03:30:00Z"), Section::Running);
    // Past the wait's end and the ordinary bound: stale, as before.
    assert!(matches!(
        need(&row, "2026-01-02T05:30:00Z"),
        Some(crate::Need::Stale { .. })
    ));
    assert_eq!(section_of(&row, "2026-01-02T05:30:00Z"), Section::NeedsYou);
}

#[test]
fn an_unreadable_wait_refuses_by_name_never_as_no_wait() {
    let event = failed(json!([{
        "site": "a",
        "provider": "claude",
        "route": "anthropic",
        "words": WORDS,
        "seconds": 600,
        "rounds": 1
    }]));
    let row = row_of(&event, &[]);
    let refused = row.waits.refused.as_deref().expect("the entry refuses");
    assert_eq!(refused, "event 8: the waits entry does not read: until");
    assert!(row.waits.waits.is_empty());
    assert_eq!(
        row.waits.lines(AT),
        vec!["wait      refused: event 8: the waits entry does not read: until".to_string()]
    );
}

#[test]
fn a_wait_whose_end_core_cannot_write_refuses_by_name() {
    let event = failed(json!([entry("a", "claude", "anthropic", FIRST_END, WORDS)]));
    let mut wait = match WaitFact::of(Some(&event)) {
        WaitFact::Waits(waits) => waits.into_iter().next().expect("one wait"),
        fact => panic!("expected waits, got {fact:?}"),
    };
    wait.until = wait
        .until
        .replace_year(-1)
        .expect("a year the RFC cannot spell");
    let painted = Waits::of(&WaitFact::Waits(vec![wait]), &[]);
    assert!(painted.waits.is_empty(), "the unspellable wait is no wait");
    assert_eq!(
        painted.refused.as_deref(),
        Some("the waits entry does not read: until")
    );
    assert_eq!(
        painted.lines(AT),
        vec!["wait      refused: the waits entry does not read: until".to_string()]
    );
}

#[test]
fn a_late_tail_stands_no_wait_on_the_run_view() {
    // A trail read that ends in a repaired tail, its repair record outside
    // this read: the tail's `effect/failed` carries waits, and the view
    // must paint none, because the tail is kept and shown and never
    // applied (decision 0108 ruling 4). Reading the journal's last row
    // instead paints the late wait.
    let events = late_wait::trail("late");
    let mut running = state(Some("work"), Status::Running, None);
    running.late_tail = Some(late_wait::tail());
    let view = crate::run_view_with(&events, Some(&running), &[]);
    assert!(view.waits.waits.is_empty(), "{:?}", view.waits);
    assert_eq!(view.waits.refused, None);
}

#[test]
fn the_run_view_derives_the_same_wait_and_the_same_later_end() {
    let event = failed(json!([entry("a", "claude", "anthropic", FIRST_END, WORDS)]));
    let cooling = [cooling("claude", "anthropic", LATER_END)];
    let row = row_of(&event, &cooling);
    let running = state(Some("work"), Status::Running, None);
    let view = crate::run_view_with(&[event], Some(&running), &cooling);
    assert_eq!(view.waits, row.waits);
    assert_eq!(view.waits.waits[0].until, LATER_END);
    assert_eq!(
        view.waits.lines(AT),
        vec![format!(
            "wait      claude/anthropic at a until {LATER_END}: {WORDS}"
        )]
    );
}
