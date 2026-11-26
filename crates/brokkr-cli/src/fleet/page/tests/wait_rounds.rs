use super::*;
use brokkr_store::test_support::statements_traced;

#[test]
fn a_paged_row_and_a_whole_read_paint_the_same_wait() {
    let (_dir, db) = crate::tests::cooling::mixed_cooling_journal();
    let mut store = Store::open_read_only(&db).unwrap();
    let at = "2098-12-31T23:59:00Z";
    let whole = one(&store, "r1", at).unwrap();
    let pages = Pages::listing(&store, Journal::at(&db), at).unwrap();
    let ask = Ask {
        now: at,
        whole: &|_| false,
        keep: &|_| true,
    };
    let (page, statements) = statements_traced(&mut store, |store| {
        pages.page(store, ask, None, PAGE_ROWS).unwrap()
    });
    assert_eq!(page.rows.len(), 1);
    let (section, row) = &page.rows[0];
    assert_eq!((*section, &row.waits), (Section::Running, &whole.waits));
    let queries = |kind: &str| statements.iter().filter(|sql| sql.contains(kind)).count();
    // The stale scan and the running scan both see this row, which has
    // two sites. Each site's ordered rounds and held maxima read once.
    assert_eq!(
        (queries("ORDER BY rounds"), queries("AND held = 1")),
        (2, 2)
    );
}

#[test]
fn overlap_keeps_only_a_page_of_waits_during_the_stale_scan() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    folded(&db, |conn| {
        for run in 0..=PAGE_ROWS {
            plant(
                conn,
                &format!("waiting-{run:02}"),
                Shape::Waiting,
                &stamp(now() - STALE - HOUR),
            );
        }
    });
    let store = Store::open_read_only(&db).unwrap();
    let ranges = brokkr_view::ranges(Section::NeedsYou, NOW);
    let rows = store.section(&ranges, None, PAGE_ROWS + 1).unwrap();
    assert_eq!(rows.len(), PAGE_ROWS + 1);
    let ask = Ask {
        now: NOW,
        whole: &|_| false,
        keep: &|_| true,
    };
    let mut read = PageReads::default();
    for row in rows {
        let listed = read
            .row(&store, row, ask, (Section::NeedsYou, None), &[])
            .unwrap();
        assert_eq!(listed.map(|row| row.run_id), None);
    }
    assert_eq!(read.held(), PAGE_ROWS);
}

#[test]
fn a_behind_row_refreshes_its_kind_when_the_end_and_head_stand() {
    use crate::tests::cooling::{
        cooling_attempt, extend_cooling, probe_round, COOLING_LATER, PROBE_MAX,
    };
    use brokkr_view::WaitKind;

    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let seq = cooling_attempt(&db);
    let mut writer = Store::open(&db).unwrap();
    extend_cooling(&mut writer, seq);
    let head = writer.head_hash("r1").unwrap();
    rusqlite::Connection::open(&db)
        .unwrap()
        .execute(
            "UPDATE run_summary SET folded_seq = 1 WHERE run_id = 'r1'",
            [],
        )
        .unwrap();
    let store = Store::open_read_only(&db).unwrap();
    // Once both rows have ended, the aggregate keeps just the newest
    // row for the route. Its end stays the same after the held append.
    let now = "2100-01-01T00:00:00Z";
    let mut pages = Pages::listing(&store, Journal::at(&db), now).unwrap();
    let read = |pages: &Pages| {
        let Pages::Summary(behind) = pages else {
            panic!("the summary pages this journal");
        };
        let wait = &behind.rows[0].waits.waits[0];
        (wait.until.clone(), wait.kind)
    };
    assert_eq!(
        read(&pages),
        (COOLING_LATER.to_string(), WaitKind::Ordinary)
    );
    let later = brokkr_store::CooldownRound {
        rounds: 3,
        ..probe_round(seq, "a", "anthropic")
    };
    assert_eq!(
        writer
            .append_held_round(
                later,
                brokkr_core::Held {
                    max_seconds: PROBE_MAX
                }
            )
            .unwrap(),
        brokkr_store::RoundAppend::Written,
    );
    assert_eq!(store.head_hash("r1").unwrap(), head);
    assert_eq!(
        pages
            .refresh(&store, Journal::at(&db), AHEAD_ROWS, now)
            .unwrap(),
        0
    );
    assert_eq!(
        read(&pages),
        (
            COOLING_LATER.to_string(),
            WaitKind::Held {
                max_seconds: PROBE_MAX
            }
        )
    );

    // A bad envelope at the same indexed head must still refuse the wait.
    let journal = rusqlite::Connection::open(&db).unwrap();
    journal
        .execute_batch("DROP TRIGGER events_append_only_update")
        .unwrap();
    journal
        .execute(
            "UPDATE events SET envelope = 'bad envelope' WHERE run_id = 'r1' AND seq = ?1",
            [seq as i64],
        )
        .unwrap();
    assert_eq!(store.head_hash("r1").unwrap(), head);
    let error = store.last_event("r1", seq).unwrap_err();
    assert!(matches!(error, StoreError::Damaged { .. }), "{error:?}");
    assert_eq!(
        pages
            .refresh(&store, Journal::at(&db), AHEAD_ROWS, now)
            .unwrap(),
        0
    );
    let Pages::Summary(behind) = pages else {
        panic!("the summary pages this journal");
    };
    let row = &behind.rows[0];
    assert_eq!(row.verdict.standing, Standing::Running);
    assert_eq!(
        row.waits,
        brokkr_view::Waits {
            waits: vec![],
            refused: Some(error.to_string()),
        }
    );
}
