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
fn holds_to_the_oracle(conn: &Connection, ends: &[i64]) {
    let low = ends.iter().copied().min().unwrap_or(0);
    let high = ends.iter().copied().max().unwrap_or(0);
    for now in [low - 1, low, (low + high) / 2, high - 1, high, high + 1] {
        assert_eq!(
            newest(conn, now).unwrap(),
            oracle(conn, now),
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
        holds_to_the_oracle(&store.conn, &ends);
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
        holds_to_the_oracle(&store.conn, &ends);
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
/// the narrow scan and its kept rows' lookups take 227 and 60,167 over ten
/// routes, and 83 and 60,023 over one.
#[test]
fn the_newest_read_is_one_scan() {
    let bound = (10_000.0 * 10_000f64.log2()) / (10.0 * 10f64.log2());
    for routes in [10, 1] {
        let steps = |rows| {
            let (_dir, store) = expired_table(rows, routes);
            newest_steps(&store.conn, 1_000_000).unwrap()
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
