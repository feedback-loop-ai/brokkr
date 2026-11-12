//! The whole-run fallback's stale-seat tests (decision 0089 slice 4, U3d):
//! the fallback names exactly the runs the statement it replaced named, and
//! the seat arm visits the stale runs alone, whatever the journal's current
//! runs.

use brokkr_core::canonical::ZERO_HASH;
use rusqlite::params;

use super::*;
use crate::event_index::{CHECKPOINTED, CHECKPOINT_STALE};
use crate::seat_summary::{lags, SHAPE};

/// The runs the fallback's old statement named, held as this test's oracle:
/// the `run_summary` scan with the seat-currentness rule, before U3d
/// replaced it with the write-side set and the shape index.
fn old() -> String {
    format!(
        "SELECT r.run_id
           FROM run_summary AS r LEFT JOIN seat_head AS h ON h.run_id = r.run_id
          WHERE {outside} AND {seat}
            AND (?1 IS NULL OR r.realm = ?1)
         UNION
         SELECT e.run_id
           FROM event_index AS e INDEXED BY event_index_checkpoint_stale
           JOIN run_summary AS r ON r.run_id = e.run_id
          WHERE e.type = '{CHECKPOINTED}' AND {CHECKPOINT_STALE}
            AND {outside}
            AND (?1 IS NULL OR r.realm = ?1)
          ORDER BY 1",
        outside = crate::aggregate::outside("r."),
        seat = lags("r", "h", "?2"),
    )
}

/// The run ids `sql` names with `realm` and this build's shape, in the
/// statement's order.
fn named(store: &Store, sql: &str, realm: Option<&str>) -> Vec<String> {
    let mut statement = store.conn.prepare(sql).unwrap();
    let mut rows = statement.query(params![realm, SHAPE.as_str()]).unwrap();
    let mut found = Vec::new();
    while let Some(row) = rows.next().unwrap() {
        found.push(row.get::<_, String>(0).unwrap());
    }
    found
}

/// A current run the journal holds: its `runs` row, its summary row at its
/// head, and its seat head of this build's shape. The set is empty for it.
fn run(store: &Store, run_id: &str, realm: &str) {
    crate::test_support::plant_run(&store.conn, run_id, "{}").unwrap();
    store
        .conn
        .execute(
            "UPDATE run_summary
                SET realm = ?2, status = 'running', recipe = 'one',
                    folded_seq = 0, folded_hash = ?3, aggregated_seq = 0,
                    stop_folded_seq = 0, ending_folded_seq = 0
              WHERE run_id = ?1",
            params![run_id, realm, ZERO_HASH],
        )
        .unwrap();
    store
        .conn
        .execute(
            "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash, shape)
             VALUES (?1, 0, ?2, ?3)",
            params![run_id, ZERO_HASH, SHAPE.as_str()],
        )
        .unwrap();
}

/// A run with no seat head whose summary row is at its head: a run whose
/// participant record landed before its effect's first request
/// (`Account::seats_kept` false), so no writer keeps a head for it.
fn headless_at_its_head(store: &Store, run_id: &str) {
    crate::test_support::plant_run(&store.conn, run_id, "{}").unwrap();
    store
        .conn
        .execute(
            "UPDATE run_summary
                SET folded_seq = 0, folded_hash = ?2, aggregated_seq = 0,
                    stop_folded_seq = 0, ending_folded_seq = 0
              WHERE run_id = ?1",
            params![run_id, ZERO_HASH],
        )
        .unwrap();
}

/// Over a journal holding every lag shape and every write order, the
/// fallback names exactly the runs the old statement named, under every
/// realm filter and under none.
#[test]
fn the_fallback_names_the_runs_the_statement_named() {
    let (_dir, store) = crate::tests::store();
    // Current, in two realms.
    run(&store, "current", "alpha");
    run(&store, "current-beta", "beta");
    // No seat head, at its head and behind it.
    headless_at_its_head(&store, "headless");
    crate::test_support::plant_run(&store.conn, "headless-behind", "{}").unwrap();
    // A seat head at another seq than the summary row names.
    run(&store, "seq-lag", "alpha");
    store
        .conn
        .execute(
            "INSERT OR REPLACE INTO seat_head (run_id, folded_seq, folded_hash, shape)
             VALUES ('seq-lag', -1, 'h', ?1)",
            [SHAPE.as_str()],
        )
        .unwrap();
    // The same seq, another hash.
    run(&store, "hash-lag", "alpha");
    store
        .conn
        .execute(
            "UPDATE run_summary SET head_hash = 'h2' WHERE run_id = 'hash-lag'",
            [],
        )
        .unwrap();
    // A seat head of another shape, at the summary's own seq and hash.
    run(&store, "shape-lag", "alpha");
    store
        .conn
        .execute(
            "UPDATE seat_head SET shape = 'another-build' WHERE run_id = 'shape-lag'",
            [],
        )
        .unwrap();
    run(&store, "shape-lag-beta", "beta");
    store
        .conn
        .execute(
            "UPDATE seat_head SET shape = 'another-build' WHERE run_id = 'shape-lag-beta'",
            [],
        )
        .unwrap();
    // An older writer's append: straight to `events`, so the summary head
    // moves by the journal's trigger and the seat head does not.
    run(&store, "older-append", "alpha");
    crate::test_support::plant_event(
        &store.conn,
        "older-append",
        &1i64,
        "h1",
        r#"{"recorded_at":"2026-01-01T00:00:00Z"}"#,
    )
    .unwrap();
    // A stale checkpoint's run, current in its seat row.
    run(&store, "checkpoint-stale", "alpha");
    plant_checkpoint(&store.conn, "checkpoint-stale", 2, FROM + 20);
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL
              WHERE run_id = 'checkpoint-stale' AND seq = 2",
            [],
        )
        .unwrap();

    let statement = old();
    for realm in [None, Some("alpha"), Some("beta"), Some("nowhere")] {
        let oracle = named(&store, &statement, realm);
        let fallback: Vec<String> = {
            let mut found = Vec::new();
            store
                .window_fallbacks(FROM, TO, realm, |run| found.push(run.run_id.to_string()))
                .unwrap();
            found
        };
        assert_eq!(fallback, oracle, "realm {realm:?}");
    }
    assert_eq!(
        named(&store, &statement, None),
        [
            "checkpoint-stale".to_string(),
            "hash-lag".to_string(),
            "headless".to_string(),
            "seq-lag".to_string(),
            "shape-lag".to_string(),
            "shape-lag-beta".to_string()
        ],
        "the journal holds every lag shape and no current run; an older \
         writer's append is in the `behind` lane, which both statements \
         leave to `behind_runs`"
    );
}

/// The seat arm visits the set and the shape index alone: adding 1,000
/// current runs leaves the fallback's VM-step count equal, while a run the
/// shape index finds adds a visit. The base journal holds 1,000 runs so
/// both measurements reach the index at the same depth, which is what
/// makes "equal to the step" the claim rather than "within a page".
#[test]
fn the_fallback_visits_only_the_stale_seats() {
    let (_dir, store) = crate::tests::store();
    for index in 0..1_000 {
        run(&store, &format!("current-{index}"), "alpha");
    }
    assert_eq!(
        fallbacks(&store, FROM, TO),
        Vec::<String>::new(),
        "every run is current"
    );
    let base = store.window_fallbacks_steps(None).unwrap();
    for index in 1_000..2_000 {
        run(&store, &format!("current-{index}"), "alpha");
    }
    assert_eq!(
        store.window_fallbacks_steps(None).unwrap(),
        base,
        "1,000 current runs add no visit"
    );

    store
        .conn
        .execute(
            "UPDATE seat_head SET shape = 'another-build' WHERE run_id = 'current-0'",
            [],
        )
        .unwrap();
    assert!(
        store.window_fallbacks_steps(None).unwrap() > base,
        "a run the shape index finds adds a visit"
    );
}
