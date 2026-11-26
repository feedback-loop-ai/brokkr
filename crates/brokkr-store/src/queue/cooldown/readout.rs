//! The newest cool-down rows for the queue's readout.

use std::collections::hash_map::Entry;
use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use super::{Cooldown, Raw, ROWS};
use crate::{Store, StoreError};

/// Every row in force at `now`, and, for a provider and route no row holds
/// any more, the newest row that ended it: what a readout paints, so a
/// probe's later end survives the instant it passes instead of reverting
/// to the event's earlier one (decision 0103 slice 4f). Admission reads
/// [`super::in_force`], which the expired row never joins. A row whose stored
/// instant this reader cannot spell refuses by name, carrying the row's
/// `seq`.
pub(super) fn newest(store: &Store, now: i64) -> Result<Vec<Cooldown>, StoreError> {
    newest_with(store, now, |_| {}, |_| {})
}

/// [`newest_in`] under one snapshot ([`Store::read_snapshot`]): the scan
/// and every kept row's lookup read the journal as it stood at the scan's
/// first row, so a writer's commit at any point in the read neither grows
/// it nor splits itself across its rows (#1112). The snapshot ends with
/// the read, its refusal included, and a read already inside one joins it.
/// `between` runs after each scanned row is taken, with its `seq`: the
/// instant a peer's commit lands, which the tests stage.
pub(super) fn newest_with(
    store: &Store,
    now: i64,
    between: impl FnMut(i64),
    stepped: impl FnOnce(i64),
) -> Result<Vec<Cooldown>, StoreError> {
    store.read_snapshot(|| newest_in(&store.conn, now, between, stepped))
}

/// [`newest`]'s two statements (#1112). The first scans the table once in
/// `seq` order, its primary key, reading only each row's key, provider,
/// route and end ([`Scanned`]), so SQLite sorts nothing and never extracts
/// a row's words; [`Newest`] decides which rows are kept. It reads one row
/// per run, the least `seq` from `from` on, and each run's reset releases
/// that row's names before the next is read, so a streaming scan's
/// registers never hold one row's wide route beside the next row's wide
/// provider. The second reads each kept row's whole projection by its
/// `seq`, in `seq` order, so a discarded row's words never enter SQLite's
/// heap. Every kept row is read before any is typed, so a column SQLite
/// cannot hand over refuses before an earlier row's unreadable time, as the
/// single statement this replaced refused. The scan reads every row's key
/// columns before any payload, so a key column's refusal precedes an
/// earlier kept row's payload refusal, which that statement would have
/// reported first; both refuse. A row is append-only and never
/// deleted, and [`newest_with`] holds both statements to one snapshot, so
/// a kept `seq` reads back the row the scan saw. Both statements' VM steps
/// go to `stepped`, so the test that binds the read's cost measures them.
fn newest_in(
    conn: &Connection,
    now: i64,
    mut between: impl FnMut(i64),
    stepped: impl FnOnce(i64),
) -> Result<Vec<Cooldown>, StoreError> {
    let mut scan = conn.prepare(
        "SELECT seq, provider, route, until_at FROM provider_cooldowns
         WHERE seq >= ?1 ORDER BY seq LIMIT 1",
    )?;
    let mut newest = Newest::default();
    let mut from = Some(i64::MIN);
    while let Some(at) = from {
        let Some(row) = scan.query_row([at], Scanned::read).optional()? else {
            break;
        };
        let seq = row.seq;
        from = seq.checked_add(1);
        newest.take(row, now);
        between(seq);
    }
    let mut payload = conn.prepare(&format!(
        "SELECT {ROWS} FROM provider_cooldowns WHERE seq = ?1"
    ))?;
    let kept = newest
        .kept()
        .into_iter()
        .map(|seq| payload.query_row([seq], Raw::read))
        .collect::<Result<Vec<_>, _>>()?;
    let steps = |statement: &rusqlite::Statement<'_>| {
        i64::from(statement.get_status(rusqlite::StatementStatus::VmStep))
    };
    stepped(steps(&scan) + steps(&payload));
    kept.into_iter().map(Raw::typed).collect()
}

/// A row as [`newest`]'s scan reads it: its key, its provider and route,
/// and its end, never its words.
struct Scanned {
    seq: i64,
    provider: String,
    route: String,
    until: i64,
}

impl Scanned {
    fn read(row: &rusqlite::Row<'_>) -> rusqlite::Result<Scanned> {
        Ok(Scanned {
            seq: row.get(0)?,
            provider: row.get(1)?,
            route: row.get(2)?,
            until: row.get(3)?,
        })
    }
}

/// Where one provider and route stands in [`Newest`]'s scan so far.
enum Standing {
    /// A row in force holds it: no expired row of it is kept.
    Held,
    /// Every row of it read so far has ended, and this, by its `seq` and
    /// its end, is the newest.
    Ended { seq: i64, until: i64 },
}

/// The readout's rows, decided over a scan in `seq` order: every row in
/// force at `now`, and, for a provider and route no such row holds, the
/// row with the latest end, the later-written on a tie. A row is kept once
/// at most, and only its `seq` is held.
#[derive(Default)]
struct Newest {
    in_force: Vec<i64>,
    routes: HashMap<(String, String), Standing>,
}

impl Newest {
    /// Take the scan's next row, which is later-written than every row
    /// taken before it.
    fn take(&mut self, row: Scanned, now: i64) {
        let key = (row.provider, row.route);
        if row.until > now {
            self.routes.insert(key, Standing::Held);
            self.in_force.push(row.seq);
            return;
        }
        let ended = Standing::Ended {
            seq: row.seq,
            until: row.until,
        };
        match self.routes.entry(key) {
            Entry::Vacant(first) => {
                first.insert(ended);
            }
            Entry::Occupied(mut seen) => {
                let later =
                    matches!(seen.get(), Standing::Ended { until, .. } if row.until >= *until);
                if later {
                    seen.insert(ended);
                }
            }
        }
    }

    /// The kept rows' `seq`s in the order they were written.
    fn kept(self) -> Vec<i64> {
        let mut kept = self.in_force;
        kept.extend(
            self.routes
                .into_values()
                .filter_map(|standing| match standing {
                    Standing::Ended { seq, .. } => Some(seq),
                    Standing::Held => None,
                }),
        );
        kept.sort_unstable();
        kept
    }
}

/// The VM steps [`newest`]'s two statements take at `now`, for the test
/// that holds the read to one scan.
#[cfg(test)]
pub(super) fn newest_steps(store: &Store, now: i64) -> Result<i64, StoreError> {
    let mut steps = 0;
    newest_with(store, now, |_| {}, |taken| steps = taken)?;
    Ok(steps)
}
