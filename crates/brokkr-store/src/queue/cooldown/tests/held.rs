//! Queue storage version 7 (decision 0106 ruling 2, slice 0106b1): a
//! cool-down row carries its kind. A version 6 journal migrates with every
//! row kept as an ordinary wait, a held wait's row takes its kind from its
//! own event, and the table's checks refuse a kind without its maximum.

use super::*;

/// The kind columns of every row, in seq order.
fn kinds(conn: &Connection) -> Vec<(Option<i64>, Option<i64>)> {
    conn.prepare("SELECT held, max_seconds FROM provider_cooldowns ORDER BY seq")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// A terminal event on `run` whose one `waits` entry for site `a` is held
/// past a two-hour maximum, written with its row.
fn append_held(store: &mut Store, run: &str, until: OffsetDateTime) {
    let wait = json!({
        "site": "a", "provider": "codex", "route": "first", "words": "usage limit",
        "retry_after_seconds": 9000, "seconds": 9000,
        "until": until.format(&Rfc3339).unwrap(), "rounds": 1,
        "held": true, "max_seconds": 7200
    });
    let mut row = row("a", "codex", "first", until);
    row.retry_after_seconds = Some(9000);
    store
        .append_with_cooldowns(
            crate::CooldownAppend {
                run_id: run,
                fenced: None,
                event_type: EventType::EffectFailed,
                payload: json!({"effect_id": "effect", "error": "usage limit", "waits": [wait]}),
                causation_id: None,
                attempt_id: None,
            },
            &[row],
        )
        .unwrap();
}

/// A version 6 journal — its cool-down table without the two kind columns,
/// holding one row — is not intact to this binary, migrates at its next
/// writable open, and reads its row back unchanged as an ordinary wait.
#[test]
fn a_version_6_queue_migrates_to_version_7_and_keeps_its_cool_downs_ordinary() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    append(&mut store, "run-a", &[row("a", "claude", "first", at(60))]).unwrap();
    let held = store.cooldowns_in_force(at(0)).unwrap();
    // Version 6's table: `held` first, because its CHECK names
    // `max_seconds`.
    store
        .conn
        .execute_batch(
            "ALTER TABLE provider_cooldowns DROP COLUMN held;
             ALTER TABLE provider_cooldowns DROP COLUMN max_seconds;
             UPDATE meta SET value = '6' WHERE key = 'queue_schema';",
        )
        .unwrap();
    assert_eq!(
        crate::queue::queue_schema(&store.conn).unwrap(),
        Some(crate::queue::storage::QUEUE_SCHEMA_V6)
    );
    assert!(!crate::queue::queue_intact(&store.conn).unwrap());
    drop(store);

    let mut store = crate::queue::tests::opened_whole(&dir.path().join("forge.db"));
    assert_eq!(crate::queue::queue_schema(&store.conn).unwrap(), Some(7));
    assert_eq!(store.cooldowns_in_force(at(0)).unwrap(), held);
    assert_eq!(kinds(&store.conn), vec![(None, None)]);
    // And the migrated table takes a held row.
    planted(&store, "run-b");
    request(&mut store, "run-b");
    append_held(&mut store, "run-b", at(9000));
    assert_eq!(
        kinds(&store.conn),
        vec![(None, None), (Some(1), Some(7200))]
    );
}

/// A held wait's row carries `held` and `max_seconds` from its own event's
/// entry for its site; an ordinary wait's row carries neither.
#[test]
fn a_held_waits_row_carries_its_kind_from_its_event() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    append_held(&mut store, "run-a", at(9000));
    append(&mut store, "run-a", &[row("a", "claude", "first", at(60))]).unwrap();
    assert_eq!(
        kinds(&store.conn),
        vec![(Some(1), Some(7200)), (None, None)]
    );
}

/// A probe's held round is written with its kind beside an ordinary round,
/// and read back by round (decision 0106 ruling 2, fix round 0106b1r2); a
/// version 6 queue has no kind columns and a version 3 one no table, and
/// each holds no held round.
#[test]
fn a_probes_held_round_is_read_back_by_its_round_and_an_older_queue_holds_none() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, event) = waited_journal(dir.path());
    let held = brokkr_core::Held { max_seconds: 30 };
    let round = next_round("run-a", event.seq, "a", at(120), 2);
    assert_eq!(
        store.append_held_round(round, held).unwrap(),
        RoundAppend::Written
    );
    let round = next_round("run-a", event.seq, "a", at(180), 3);
    assert_eq!(
        store.append_cooldown_round(round).unwrap(),
        RoundAppend::Written
    );
    let read = |store: &Store| store.cooldown_held_rounds("run-a", event.seq, "a").unwrap();
    assert_eq!(read(&store).into_iter().collect::<Vec<_>>(), [(2, held)]);
    assert_eq!(
        kinds(&store.conn),
        vec![(None, None), (Some(1), Some(30)), (None, None)]
    );
    for downgrade in [
        "ALTER TABLE provider_cooldowns DROP COLUMN held;
         ALTER TABLE provider_cooldowns DROP COLUMN max_seconds;
         UPDATE meta SET value = '6' WHERE key = 'queue_schema';",
        "UPDATE meta SET value = '3' WHERE key = 'queue_schema';",
    ] {
        store.conn.execute_batch(downgrade).unwrap();
        assert!(read(&store).is_empty(), "{downgrade}");
    }
}

/// The kind's checks refuse, by name, a held row without its maximum, a
/// maximum without `held`, a `held` that is not 1, and a maximum below one.
#[test]
fn the_kind_checks_refuse_a_kind_without_its_maximum() {
    let dir = tempfile::tempdir().unwrap();
    let store = journal(dir.path());
    for (case, held, max, broken) in [
        (
            "held without a maximum",
            Some(1),
            None,
            "held_names_its_maximum",
        ),
        (
            "a maximum without held",
            None,
            Some(7200),
            "held_names_its_maximum",
        ),
        (
            "held is not 1",
            Some(2),
            Some(7200),
            "held_names_its_maximum",
        ),
        ("a zero maximum", Some(1), Some(0), "max_positive"),
    ] {
        let error = store
            .conn
            .execute(
                "INSERT INTO provider_cooldowns
                    (run_id, event_seq, site, provider, route, until_at, reason,
                     retry_after_seconds, rounds, recorded_at, held, max_seconds)
                 VALUES ('run-a', 1, '', 'codex', 'first', ?1, 'w', 9000, 1, 0, ?2, ?3)",
                params![at(60).unix_timestamp(), held, max],
            )
            .expect_err(case);
        assert!(error.to_string().contains(broken), "{case}: {error}");
    }
}
