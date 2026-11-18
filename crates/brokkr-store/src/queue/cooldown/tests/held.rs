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
    row.held = Some(brokkr_core::Held { max_seconds: 7200 });
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
             DROP TABLE probe_holds;
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
    // And version 7's probe holds' table, empty.
    let hold = store
        .hold_probe(&probe_hold("run-b", at(30)), at(0))
        .unwrap();
    assert!(hold.is_some());
}

/// A probe hold on claude/first for `run` until `deadline`.
fn probe_hold(run: &str, deadline: OffsetDateTime) -> crate::ProbeHold<'_> {
    crate::ProbeHold {
        run_id: run,
        site: "",
        provider: "claude",
        route: "first",
        deadline,
    }
}

/// A probe hold is read by the in-force read, the pass's and the claim's
/// one statement, while it is unended and before its deadline (fix round
/// 0106b1r4, F1): with its deadline as its end and words naming the
/// probing run. At its deadline, or once ended, it holds nothing; it ends
/// once, and a version 6 queue takes none.
#[test]
fn a_probe_hold_is_in_force_until_it_ends_or_its_deadline_passes() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = journal(dir.path());
    let first = store
        .hold_probe(&probe_hold("run-a", at(30)), at(0))
        .unwrap()
        .expect("a version 7 queue takes the hold");
    let read = store.cooldowns_in_force(at(10)).unwrap();
    assert_eq!(
        read,
        [Cooldown {
            site: String::new(),
            provider: "claude".into(),
            route: "first".into(),
            until_at: at(30),
            reason: "a cool-down probe of run run-a holds the route while it is in flight".into(),
            retry_after_seconds: None,
            rounds: 0,
            recorded_at: at(0),
        }]
    );
    assert_eq!(store.cooldowns_in_force(at(30)).unwrap(), []);
    // The aggregate a wait sleeps to is the table's rows only.
    assert_eq!(store.cooldown("claude", "first", at(10)).unwrap(), None);
    let second = store
        .hold_probe(&probe_hold("run-a", at(60)), at(0))
        .unwrap()
        .unwrap();
    store.end_probe_hold(Some(second), at(5)).unwrap();
    store.end_probe_hold(Some(second), at(6)).unwrap();
    store.end_probe_hold(None, at(6)).unwrap();
    assert_eq!(store.cooldowns_in_force(at(10)).unwrap(), read);
    store.end_probe_hold(Some(first), at(7)).unwrap();
    assert_eq!(store.cooldowns_in_force(at(10)).unwrap(), []);
    let ended: Vec<i64> = store
        .conn
        .prepare("SELECT ended_at FROM probe_holds ORDER BY seq")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(ended, [at(7).unix_timestamp(), at(5).unix_timestamp()]);
    // A hold left standing is joined by no older queue's in-force read.
    store
        .hold_probe(&probe_hold("run-a", at(60)), at(0))
        .unwrap()
        .unwrap();
    assert_eq!(store.cooldowns_in_force(at(10)).unwrap().len(), 1);
    store
        .conn
        .execute("UPDATE meta SET value = '6' WHERE key = 'queue_schema'", [])
        .unwrap();
    assert_eq!(store.cooldowns_in_force(at(10)).unwrap(), []);
    assert_eq!(
        store
            .hold_probe(&probe_hold("run-a", at(90)), at(0))
            .unwrap(),
        None
    );
}

/// A held wait's row carries `held` and `max_seconds` as its writer derived
/// them from its event's entry for its site, and an ordinary wait's row
/// carries neither: the store parses no payload for the kind (fix round
/// 0106b1r3).
#[test]
fn a_held_waits_row_carries_the_kind_its_writer_derived() {
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

/// A probe round the table takes without writing its row (a trigger that
/// ignores the insert) refuses by name, and its transaction ends no hold:
/// the hold stands and the round is not read back (fix round 0106b1r5,
/// H3).
#[test]
fn a_round_the_table_does_not_write_refuses_and_leaves_its_hold_standing() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, event) = waited_journal(dir.path());
    let hold = (store.hold_probe(&probe_hold("run-a", at(90)), at(0))).unwrap();
    store
        .conn
        .execute_batch(
            "CREATE TRIGGER ignored BEFORE INSERT ON provider_cooldowns
             BEGIN SELECT RAISE(IGNORE); END;",
        )
        .unwrap();
    let mut round = next_round("run-a", event.seq, "a", at(120), 2);
    round.ends = hold;
    let refused = store.append_cooldown_round(round).unwrap_err();
    let StoreError::Queue(QueueRefusal::CooldownRoundUnwritten(unwritten)) = &refused else {
        panic!("an unwritten round was not refused by name: {refused:?}");
    };
    assert_eq!(
        **unwritten,
        crate::queue::CooldownRoundMismatch {
            run_id: "run-a".into(),
            event_seq: event.seq,
            site: "a".into(),
            rounds: 2,
            standing: 1,
        }
    );
    assert_eq!(
        refused.to_string(),
        format!(
            "the journal's provider cool-downs wrote no row for run 'run-a' event {} site 'a' \
             round 2, so the probe's hold on its route stands and nothing was written",
            event.seq
        )
    );
    let rounds = store.cooldown_rounds("run-a", event.seq, "a").unwrap();
    assert_eq!(rounds.len(), 1);
    let unended: i64 = (store.conn)
        .query_row(
            "SELECT COUNT(*) FROM probe_holds WHERE ended_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(unended, 1);
}
