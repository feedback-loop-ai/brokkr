//! The readout's newest read is one scan (#1112): it answers
//! exactly the rows the correlated `NOT EXISTS` it replaced answered, in
//! the same order, and its VM steps grow with the table, not its square.

use super::*;

/// The statement `newest` ran before #1112, kept as the oracle the scan is
/// held to: every row in force, and every row no later row of its
/// provider and route outlasts.
fn oracle(conn: &Connection, now: i64) -> Vec<Cooldown> {
    let sql = format!(
        "SELECT {ROWS} FROM provider_cooldowns AS held
         WHERE held.until_at > ?1
            OR NOT EXISTS (
                SELECT 1 FROM provider_cooldowns AS later
                WHERE later.provider = held.provider AND later.route = held.route
                  AND (later.until_at > held.until_at
                       OR (later.until_at = held.until_at AND later.seq > held.seq)))
         ORDER BY held.seq"
    );
    read_rows(conn, &sql, now).unwrap()
}

/// A journal holding run-a's event 1, which every planted row hangs from.
fn planting() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = journal(dir.path());
    (dir, store)
}

/// Plant one row per `(provider, route, until)`, in order, each its own
/// round of site `''` and each named by its place, so two rows never read
/// back alike.
fn plant(conn: &Connection, rows: &[(&str, &str, i64)]) {
    let first: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(rounds), 0) FROM provider_cooldowns",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let mut insert = conn
        .prepare(
            "INSERT INTO provider_cooldowns
                (run_id, event_seq, site, provider, route, until_at, reason,
                 retry_after_seconds, rounds, recorded_at)
             VALUES ('run-a', 1, '', ?1, ?2, ?3, ?4, NULL, ?5, ?3 - 1)",
        )
        .unwrap();
    for (place, (provider, route, until)) in rows.iter().enumerate() {
        let round = first + place as i64 + 1;
        insert
            .execute(params![
                provider,
                route,
                until,
                format!("row {round}"),
                round
            ])
            .unwrap();
    }
}

/// A deterministic stream of small numbers, so the shapes the test draws
/// are the same on every run.
struct Draw(u64);

impl Draw {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        (self.0 >> 33) % bound
    }
}

/// The scan answers the oracle's rows in the oracle's order at every
/// instant across the table's ends.
fn holds_to_the_oracle(store: &Store, ends: &[i64]) {
    let low = ends.iter().copied().min().unwrap_or(0);
    let high = ends.iter().copied().max().unwrap_or(0);
    for now in [low - 1, low, (low + high) / 2, high - 1, high, high + 1] {
        assert_eq!(
            newest(store, now).unwrap(),
            oracle(&store.conn, now),
            "at {now} over ends {ends:?}"
        );
    }
}

/// Every shape: empty, one row, one route, many routes and two providers,
/// every route expired, some in force, ties on `until_at` (the later `seq`
/// wins), negative and equal ends, and a route's name shared across
/// providers. The scan answers exactly the old statement's rows.
#[test]
fn the_scan_answers_the_correlated_forms_rows_in_its_order() {
    let named: [(&str, &str, i64); 8] = [
        ("claude", "first", 50),
        ("claude", "first", 50),
        ("codex", "first", 50),
        ("claude", "second", -20),
        ("claude", "second", -20),
        ("claude", "first", 10),
        ("codex", "first", LAST_SECOND),
        ("codex", "first", 90),
    ];
    for take in 0..=named.len() {
        let (_dir, store) = planting();
        plant(&store.conn, &named[..take]);
        let ends: Vec<i64> = named[..take].iter().map(|row| row.2).collect();
        holds_to_the_oracle(&store, &ends);
    }
    let providers = ["claude", "codex"];
    let routes = ["r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9"];
    let mut draw = Draw(1112);
    for shape in 0..40 {
        let (_dir, store) = planting();
        let size = draw.below(60) as usize;
        let spread = [1, 3, 1_000][shape % 3];
        let width = [1, 3, 10][shape / 3 % 3];
        let rows: Vec<(&str, &str, i64)> = (0..size)
            .map(|_| {
                (
                    providers[draw.below(2) as usize],
                    routes[draw.below(width) as usize],
                    1_000 + draw.below(spread) as i64,
                )
            })
            .collect();
        plant(&store.conn, &rows);
        let ends: Vec<i64> = rows.iter().map(|row| row.2).collect();
        holds_to_the_oracle(&store, &ends);
    }
}

/// `rows` rows spread over `routes` routes of one provider, every one
/// expired at the read (the worst case the old statement rescanned for),
/// with each route's ends rising so its newest is its last.
fn expired_table(rows: usize, routes: usize) -> (tempfile::TempDir, Store) {
    let (dir, store) = planting();
    let names: Vec<String> = (0..routes).map(|route| format!("route-{route}")).collect();
    let planted: Vec<(&str, &str, i64)> = (0..rows)
        .map(|at| ("claude", names[at % routes].as_str(), 1_000 + at as i64))
        .collect();
    store.conn.execute_batch("BEGIN").unwrap();
    plant(&store.conn, &planted);
    store.conn.execute_batch("COMMIT").unwrap();
    (dir, store)
}

/// The statement's steps at 10 and 10,000 rows, over ten routes and over
/// one, grow no faster than `n log n`: a thousand times the rows by at most
/// four thousand times the steps. The correlated form it replaced took
/// 998 and 380,934,683 over ten routes, and 962 and 650,305,007 over one;
/// the narrow scan, a lookup per row, and its kept rows' lookups take 299
/// and 130,169 over ten routes, and 155 and 130,025 over one.
#[test]
fn the_newest_read_is_one_scan() {
    let bound = (10_000.0 * 10_000f64.log2()) / (10.0 * 10f64.log2());
    for routes in [10, 1] {
        let steps = |rows| {
            let (_dir, store) = expired_table(rows, routes);
            newest_steps(&store, 1_000_000).unwrap()
        };
        let (few, many) = (steps(10), steps(10_000));
        eprintln!(
            "newest over {routes} route(s): {few} steps at 10 rows, {many} at 10,000 (SQLite {})",
            rusqlite::version()
        );
        assert!(
            (many as f64) <= few as f64 * bound,
            "over {routes} route(s): {few} steps at 10 rows grew to {many} at 10,000, past n log n"
        );
    }
}

/// Rows at both ends of `seq`'s range, which only direct SQL writes, are
/// read as the oracle reads them: the row-by-row scan starts below the
/// least `seq` and stops after the greatest.
#[test]
fn the_scan_reads_both_ends_of_the_seq_range() {
    let (_dir, store) = planting();
    plant_at(
        &store.conn,
        &[
            (i64::MIN, "first", 50),
            (0, "second", 50),
            (i64::MAX, "third", 50),
        ],
    );
    assert_eq!(newest(&store, 100).unwrap().len(), 3);
    holds_to_the_oracle(&store, &[50]);
}

/// Plant one row per `(seq, route, until)` of provider `claude`, each at
/// the `seq` it names, which only direct SQL writes, and each its own
/// round, named by its `seq`.
fn plant_at(conn: &Connection, rows: &[(i64, &str, i64)]) {
    let first: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(rounds), 0) FROM provider_cooldowns",
            [],
            |row| row.get(0),
        )
        .unwrap();
    for (place, (seq, route, until)) in rows.iter().enumerate() {
        conn.execute(
            "INSERT INTO provider_cooldowns
                (seq, run_id, event_seq, site, provider, route, until_at, reason,
                 retry_after_seconds, rounds, recorded_at)
             VALUES (?1, 'run-a', 1, '', 'claude', ?2, ?3, ?4, NULL, ?5, 0)",
            params![
                seq,
                route,
                until,
                format!("seq {seq}"),
                first + place as i64 + 1
            ],
        )
        .unwrap();
    }
}

/// A second connection onto `store`'s WAL journal: the peer whose commits
/// land while the read is under way.
fn peer(store: &Store) -> Connection {
    Connection::open(store.path()).unwrap()
}

/// [`newest`] at `now`, `commit` run once, after the scan's first row is
/// taken; the read's rows and how many rows the scan took.
fn read_across(store: &Store, now: i64, commit: impl FnOnce()) -> (Vec<Cooldown>, usize) {
    let mut commit = Some(commit);
    let mut taken = 0;
    let read = newest_with(
        store,
        now,
        |_| {
            taken += 1;
            if let Some(commit) = commit.take() {
                commit();
            }
        },
        |_| {},
    );
    (read.unwrap(), taken)
}

/// A peer's 1,024 rows in force, committed after the scan's first row is
/// read, never join the read: the scan and its lookups read the one
/// snapshot the read began on, so its work is bounded by the rows present
/// then. Read lookup by lookup, the scan followed the commit to 1,025 rows.
#[test]
fn a_commit_during_the_read_does_not_grow_it() {
    let (_dir, store) = planting();
    plant(&store.conn, &[("claude", "first", 500)]);
    let at_start = oracle(&store.conn, 100);
    assert_eq!(at_start.len(), 1);
    let writer = peer(&store);
    let later = vec![("claude", "first", 500); 1_024];
    let (read, taken) = read_across(&store, 100, || {
        writer.execute_batch("BEGIN").unwrap();
        plant(&writer, &later);
        writer.execute_batch("COMMIT").unwrap();
    });
    assert_eq!(read, at_start, "the read's rows are its start's");
    assert_eq!(taken, 1, "the scan took the one row present at its start");
    assert_eq!(
        oracle(&store.conn, 100).len(),
        1_025,
        "the peer's rows landed"
    );
}

/// A peer's one commit of seqs 0 and 2, landing after seq 1 is read, is
/// read whole or not at all: the read answers seq 1 alone, as the journal
/// stood at its start, never seqs 1 and 2, a set no snapshot held.
#[test]
fn a_commit_during_the_read_is_not_split_across_it() {
    let (_dir, store) = planting();
    plant_at(&store.conn, &[(1, "one", 500)]);
    let at_start = oracle(&store.conn, 100);
    let writer = peer(&store);
    let (read, _) = read_across(&store, 100, || {
        writer.execute_batch("BEGIN").unwrap();
        plant_at(&writer, &[(0, "zero", 500), (2, "two", 500)]);
        writer.execute_batch("COMMIT").unwrap();
    });
    let reasons =
        |rows: &[Cooldown]| -> Vec<String> { rows.iter().map(|row| row.reason.clone()).collect() };
    assert_eq!(read, at_start);
    assert_eq!(reasons(&read), ["seq 1"]);
    assert_eq!(
        reasons(&oracle(&store.conn, 100)),
        ["seq 0", "seq 1", "seq 2"],
        "the peer's one commit landed whole"
    );
}

/// The read's snapshot ends with it, its refusal included, so the
/// connection's next read sees a peer's later commit; and a read already
/// inside one read joins it rather than refusing a second.
#[test]
fn the_reads_snapshot_ends_with_its_refusal_and_joins_an_outer_one() {
    let (_dir, store) = planting();
    plant_values(&store.conn, vec![(text("claude"), "first", 50, blob(b'a'))]);
    let refused = newest(&store, 100).unwrap_err();
    assert!(refuses_blob(&refused, 5, "reason"), "{refused:?}");
    plant(&peer(&store), &[("claude", "second", 500)]);
    let count: i64 = store
        .conn
        .query_row("SELECT count(*) FROM provider_cooldowns", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(count, 2, "the refused read held no snapshot past its end");
    let joined = store
        .as_one_read(|| newest(&store, 100).map(|rows| rows.len()))
        .unwrap_err();
    assert!(refuses_blob(&joined, 5, "reason"), "{joined:?}");
}

/// A row whose words SQLite cannot hand over as text refuses before an
/// earlier row's unreadable time, as the oracle's read refused: every
/// kept row is read before any is typed.
#[test]
fn a_column_refusal_precedes_an_earlier_rows_time_refusal() {
    let (_dir, store) = planting();
    plant_values(
        &store.conn,
        vec![
            (text("claude"), "first", i64::MAX, text("words")),
            (text("claude"), "second", 50, blob(b'a')),
        ],
    );
    let refused = newest(&store, 100).unwrap_err();
    assert!(refuses_blob(&refused, 5, "reason"), "{refused:?}");
}

/// A later kept row's provider that SQLite cannot hand over as text
/// refuses before an earlier kept row's unreadable words: the scan reads
/// every row's key columns before any kept row's payload is read. The
/// single statement this replaced refused the earlier row's `reason`
/// (column 5) first. Both refuse, and the scan's order stands (#1112,
/// ruled in 1112r3).
#[test]
fn a_key_column_refusal_precedes_an_earlier_kept_rows_payload_refusal() {
    let (_dir, store) = planting();
    plant_values(
        &store.conn,
        vec![
            (text("claude"), "first", 500, blob(b'w')),
            (blob(b'p'), "second", 500, text("words")),
        ],
    );
    let refused = newest(&store, 100).unwrap_err();
    assert!(refuses_blob(&refused, 1, "provider"), "{refused:?}");
}

/// Plant one row per `(provider, route, until, reason)` into an empty
/// table, each its own round, the provider and the words of whatever type
/// SQLite stores, which only direct SQL writes.
fn plant_values(conn: &Connection, rows: Vec<(Value, &str, i64, Value)>) {
    for (round, (provider, route, until, reason)) in rows.into_iter().enumerate() {
        conn.execute(
            "INSERT INTO provider_cooldowns
                (run_id, event_seq, site, provider, route, until_at, reason,
                 retry_after_seconds, rounds, recorded_at)
             VALUES ('run-a', 1, '', ?1, ?2, ?3, ?4, NULL, ?5, 0)",
            params![provider, route, until, reason, round as i64 + 1],
        )
        .unwrap();
    }
}

type Value = rusqlite::types::Value;

/// A column's text.
fn text(words: &str) -> Value {
    words.to_string().into()
}

/// A column's one-byte BLOB, which no text reader takes.
fn blob(byte: u8) -> Value {
    vec![byte].into()
}

/// Whether `refused` is SQLite refusing column `index`, named `name`, as
/// a BLOB.
fn refuses_blob(refused: &StoreError, index: usize, name: &str) -> bool {
    matches!(
        refused,
        StoreError::Sqlite(rusqlite::Error::InvalidColumnType(
            at,
            column,
            rusqlite::types::Type::Blob,
        )) if *at == index && column == name
    )
}
