//! The terminal event's cool-downs as queue rows (decision 0103 ruling
//! 2(a), slice 4d1): the engine derives them from the event's own `waits` at
//! the one append that writes it, the store writes them with it in one
//! transaction, and a contended terminal event still lands with them.

use brokkr_core::envelope::EventType;
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::panel::{
    b_only_panel, indeterminate_once, last_terminal, panel_engine, route_host, two_route_engine,
    wait_entries,
};
use super::*;
use crate::engine::contention_tests::{in_flight, write_lock_on};
use crate::engine::cooldown::cooldown_rows;

/// The rows in force at `event`'s own recording instant, as every test
/// here reads them: a row written by the event is in force then.
fn rows_of(engine: &Engine, event: &brokkr_core::EventEnvelope) -> Vec<brokkr_store::Cooldown> {
    let at = OffsetDateTime::parse(&event.recorded_at, &Rfc3339).unwrap();
    engine.store.cooldowns_in_force(at).unwrap()
}

/// One event with two refused sites — a two-route panel, claude/first and
/// codex/second — writes two rows, each at its own site and route.
#[test]
fn a_two_route_panel_writes_a_row_per_refused_site() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_engine(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
        2,
    );
    // Bounds long enough that the failed event records while both waits
    // still run, however loaded the host is: the table holds what is in
    // force when the event records it.
    engine.cooldown.host = Some(route_host(dir.path(), (30, 60), (40, 60)));
    engine.drive().unwrap();

    let failed = failed(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    let mut rows = rows_of(&engine, &failed[0]);
    rows.sort_by(|a, b| a.site.cmp(&b.site));
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert_row(&rows[0], "a", "claude", "first");
    assert_row(&rows[1], "b", "codex", "second");
    // The wait stays on the event as its record, unchanged.
    let waits = wait_entries(&failed[0]);
    assert_eq!(waits.len(), 2, "{waits:?}");
}

/// The one row a site's refusal wrote: its site, provider and route.
fn assert_row(row: &brokkr_store::Cooldown, site: &str, provider: &str, route: &str) {
    assert_eq!(
        (row.site.as_str(), row.provider.as_str(), row.route.as_str()),
        (site, provider, route)
    );
}

/// An indeterminate member beside a refused one still records the refused
/// site's row, and retries nothing: the retry behaviour is unchanged.
#[test]
fn an_indeterminate_panel_records_the_refused_site() {
    let dir = tempfile::tempdir().unwrap();
    let a = indeterminate_once(dir.path(), "a");
    let body = b_only_panel(dir.path(), a);
    let host = route_host(dir.path(), (10, 20), (10, 20));
    let mut engine = panel_engine(dir.path(), body, 2, host);
    engine.drive().unwrap();

    let terminal = last_terminal(&engine);
    assert_eq!(terminal.event_type, EventType::EffectIndeterminate);
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse"],
        "the indeterminate panel retried or probed b"
    );
    assert_eq!(starts(&engine).len(), 1);
    let rows = rows_of(&engine, &terminal);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_row(&rows[0], "b", "codex", "second");
}

/// A terminal event the lock outlasts is held, and its landing writes its
/// rows in the same transaction: the contended event lands with them.
#[test]
fn a_held_terminal_event_lands_with_its_rows() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = in_flight(dir.path());
    let holder = write_lock_on(&dir.path().join("realm.db"));
    // An end no loaded host reaches while the lock is held and the held
    // outcome lands, so the row is still in force when it records.
    let until = OffsetDateTime::now_utc() + time::Duration::seconds(60);
    let payload = json!({
        "effect_id": "effect-1",
        "attempt_id": "attempt-1",
        "error": "provider refused",
        "start_failure": true,
        "start_failure_sites": [null],
        "waits": wait_value(until, 60),
    });
    // The terminal event meets the lock past every settling patience and
    // is held; the lawful end's own tries meet it too.
    let contended = engine
        .append(
            EventType::EffectFailed,
            payload,
            Some("attempt-1".to_string()),
        )
        .expect_err("a lock held throughout cannot be appended past");
    assert!(engine.held_outcome.is_some(), "the outcome was not held");
    let outlasted = engine
        .lawful_end_under_contention(contended)
        .expect_err("nothing is writable behind the lock");
    assert_eq!(failed(&engine).len(), 0, "the held event landed anyway");
    drop(holder);

    engine
        .lawful_end_under_contention(outlasted)
        .expect("the held outcome lands once the lock lets go");
    let failed = failed(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    let rows = rows_of(&engine, &failed[0]);
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_row(&rows[0], "", "claude", "first");
}

/// The rows a terminal payload records, read directly: one per wait, and
/// none for a record that does not read or an event type that carries no
/// waits.
#[test]
fn the_rows_a_terminal_payload_records() {
    let now = base();
    let payload = |extra: Value| {
        let mut payload = json!({"effect_id": "effect"});
        payload
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        payload
    };
    let recorded = |event_type, payload: &Value| {
        cooldown_rows(event_type, payload)
            .into_iter()
            .map(|row| (row.site, row.provider, row.route, row.rounds))
            .collect::<Vec<_>>()
    };

    // One row per `waits` entry.
    let waits = json!({"waits": wait_value(now + time::Duration::seconds(2), 2)});
    assert_eq!(
        recorded(EventType::EffectFailed, &payload(waits.clone())),
        [("".into(), "claude".into(), "first".into(), 1)]
    );
    // A present but unreadable entry writes none: the engine's own reader
    // refuses it by name.
    let malformed = json!({"waits": [{"site": "a"}]});
    assert_eq!(recorded(EventType::EffectFailed, &payload(malformed)), []);
    // No waits at all, and no other event type, writes none.
    assert_eq!(recorded(EventType::EffectFailed, &payload(json!({}))), []);
    assert_eq!(
        recorded(EventType::EffectSucceeded, &payload(waits.clone())),
        []
    );
}
