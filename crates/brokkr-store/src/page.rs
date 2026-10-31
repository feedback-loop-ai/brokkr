//! The fleet read a page at a time (decision 0089 ruling 6): one run's
//! summary row per listed run, read by keyset from the index
//! `(lane, order_at DESC, run_id DESC)`, never a run's events.
//!
//! A section is several ranges of that index, one per lane it holds;
//! [`Store::section`] reads them merged into the one order a page lists
//! (`brokkr_core::page::newest_first`), stepping each statement only as
//! far as the merge consumes it and one row ahead, so a page of n rows
//! visits at most n rows and one more per range. The reader checks each
//! row against its run's last event ([`Store::stands_at`]; ruling 5) and
//! reads a run whole where the two disagree; a row behind its head is
//! listed by [`Store::behind`], to be read whole before it is placed.
//!
//! Every read here is a read: a journal opened read-only pages as one
//! opened to write (decision 0014).

use brokkr_core::fold::Status;
use brokkr_core::page::{newest_first, Bound, Copied, Lane, Range, Row, Whole};
use brokkr_core::policy::Severity;
use brokkr_core::summary::{Bounded, Hired, Refused, Ruled, Stated};
use rusqlite::{OptionalExtension, ToSql};
use thiserror::Error;

use crate::acknowledgement::standing;
use crate::import::RUN_ID_MAX;
use crate::summary::summary_intact;
use crate::{head_of, Store, StoreError};

/// The columns a page selects, each within its bound, none of the whole
/// values that sit last in the row. The run id is read whole: it is the
/// row's key, which the page's order and every cursor compare (ruling 6),
/// and [`admitted`] measures its bytes, so a row whose id no writer
/// admits is told by its length. SQLite neither cuts nor measures it: its
/// `substr` and `length` stop a text at its first NUL, which would read an
/// id as the text before its NUL, another run's.
const COLUMNS: &str = "run_id, order_at, feature_line, created_at,
    last_recorded_at, status, phase, park_reason, rule_id, rule_from, rule_routes,
    hire_seat, hire_failed_attempts, residual_open, refusal_seq, refusal_words,
    folded_seq, folded_hash, acknowledged, ack_operator, ack_reason, ack_at,
    stop_accepted_seq";

/// Why a page cannot read a summary row as a run's account: a value no
/// writer writes. The reader reads such a run whole instead, or, where
/// even its id is past what a writer admits, lists it as refused.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RowRefusal {
    #[error("its run id is past {RUN_ID_MAX} bytes, which no writer admits")]
    RunIdPastBound,
    #[error("its status '{0}' is no status a writer words")]
    Status(String),
    #[error("its residual '{0}' is no severity")]
    Severity(String),
    #[error("a column does not read: {0}")]
    Column(String),
    #[error("its residuals do not read: {0}")]
    Residuals(String),
}

/// A row a page could not read, at its place in the page's order.
#[derive(Debug, PartialEq, Eq)]
pub struct Unread {
    /// The run's id, whole, past its bound too: the row's key.
    pub run_id: String,
    pub order_at: i64,
    pub why: RowRefusal,
}

/// One row of a page: read, or refused at the edge.
pub type Paged = Result<Row, Unread>;

/// A run of the behind lane as [`Store::behind`] hands it out: its place,
/// or, where its id is past what a writer admits, its refusal there.
pub type Keyed = Result<(i64, String), Unread>;

/// `run_id` at `order_at` as a key of the behind lane: refused where its
/// id is past what a writer admits, as a page refuses its row, so no lane
/// hands out a whole id of any length as a run to read (rulings 2 and 5).
pub fn keyed(order_at: i64, run_id: String) -> Keyed {
    match admitted(&run_id) {
        Ok(()) => Ok((order_at, run_id)),
        Err(why) => Err(Unread {
            run_id,
            order_at,
            why,
        }),
    }
}

/// Whether `run_id` is within a writer's length bound: the one bound
/// every lane's key is held to, so an id a journal already holds reads.
fn admitted(run_id: &str) -> Result<(), RowRefusal> {
    if run_id.len() > RUN_ID_MAX {
        return Err(RowRefusal::RunIdPastBound);
    }
    Ok(())
}

/// A paged row's place in its section's order.
pub fn place_of(paged: &Paged) -> (i64, &str) {
    match paged {
        Ok(row) => (row.order_at, &row.run_id),
        Err(unread) => (unread.order_at, &unread.run_id),
    }
}

impl From<rusqlite::Error> for RowRefusal {
    fn from(error: rusqlite::Error) -> RowRefusal {
        RowRefusal::Column(error.to_string())
    }
}

/// A bounded column as a page reads it: its cut, its whole not read.
fn cut(text: String) -> Bounded {
    Bounded {
        cut: text,
        whole: None,
    }
}

/// The row at `row`, read past its run id and place.
fn row_of(row: &rusqlite::Row<'_>, run_id: String, order_at: i64) -> Result<Row, RowRefusal> {
    admitted(&run_id)?;
    let refused = refused_of(row)?;
    let acknowledged = match refused {
        Some(_) => copied_of(row)?,
        None => None,
    };
    let (feature_line, created_at, last_recorded_at) = listed_of(row)?;
    let (status, stop_accepted_seq) = standing_of(row)?;
    Ok(Row {
        run_id,
        order_at,
        feature_line,
        created_at,
        last_recorded_at,
        status,
        stop_accepted_seq,
        phase: bounded(row, 6)?,
        park_reason: bounded(row, 7)?,
        ruled: ruled_of(row)?,
        hired: hired_of(row)?,
        residual_open: severity_of(row)?,
        refused,
        folded: (row.get::<_, i64>(16)? as u64, row.get(17)?),
        acknowledged,
    })
}

/// The bounded column `at` of `row`, where it holds a value.
fn bounded(row: &rusqlite::Row<'_>, at: usize) -> rusqlite::Result<Option<Bounded>> {
    Ok(row.get::<_, Option<String>>(at)?.map(cut))
}

/// What the row lists the run by: its feature's first line, when it was
/// created and when its journal last moved.
fn listed_of(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, Bounded, Option<Bounded>)> {
    Ok((row.get(2)?, cut(row.get(3)?), bounded(row, 4)?))
}

/// Where the row's run stands: its status, and the seq of an operator
/// stop accepted with no ending yet (#909).
fn standing_of(row: &rusqlite::Row<'_>) -> Result<(Option<Status>, Option<u64>), RowRefusal> {
    let stop_accepted_seq = row.get::<_, Option<i64>>(22)?.map(|seq| seq as u64);
    Ok((status_of(row)?, stop_accepted_seq))
}

/// The row's status word, read into the status a writer words by it.
fn status_of(row: &rusqlite::Row<'_>) -> Result<Option<Status>, RowRefusal> {
    match row.get::<_, Option<String>>(5)? {
        Some(word) => Status::named(&word)
            .map(Some)
            .ok_or(RowRefusal::Status(word)),
        None => Ok(None),
    }
}

/// The row's highest open residual, read into its severity.
fn severity_of(row: &rusqlite::Row<'_>) -> Result<Option<Severity>, RowRefusal> {
    match row.get::<_, Option<String>>(13)? {
        Some(word) => Severity::named(&word)
            .map(Some)
            .ok_or(RowRefusal::Severity(word)),
        None => Ok(None),
    }
}

/// The fold's refusal as the row states it, where it states one.
fn refused_of(row: &rusqlite::Row<'_>) -> Result<Option<Refused>, RowRefusal> {
    let Some(words) = row.get::<_, Option<String>>(15)? else {
        return Ok(None);
    };
    Ok(Some(Refused {
        seq: row.get::<_, i64>(14)? as u64,
        words: cut(words),
    }))
}

/// The acknowledgement's copy on the row, where one stands at its head.
fn copied_of(row: &rusqlite::Row<'_>) -> Result<Option<Copied>, RowRefusal> {
    if !row.get::<_, bool>(18)? {
        return Ok(None);
    }
    Ok(Some(Copied {
        operator: row.get(19)?,
        reason: row.get(20)?,
        at: row.get(21)?,
    }))
}

/// The last ruling as the row states it, where it states one.
fn ruled_of(row: &rusqlite::Row<'_>) -> Result<Option<Ruled>, RowRefusal> {
    let Some(from) = row.get::<_, Option<String>>(9)? else {
        return Ok(None);
    };
    Ok(Some(Ruled {
        rule_id: row.get::<_, Option<String>>(8)?.map(cut),
        from: cut(from),
        routes: row.get(10)?,
    }))
}

/// The seat at work as the row states it, where it states one.
fn hired_of(row: &rusqlite::Row<'_>) -> Result<Option<Hired>, RowRefusal> {
    let Some(seat) = row.get::<_, Option<String>>(11)? else {
        return Ok(None);
    };
    Ok(Some(Hired {
        seat: cut(seat),
        failed_attempts: row.get::<_, i64>(12)? as u64,
    }))
}

/// One row of a range, read: refused at the edge where a value is not a
/// writer's, and never past its place, which the merge orders by.
fn paged(row: &rusqlite::Row<'_>) -> rusqlite::Result<Paged> {
    let run_id: String = row.get(0)?;
    let order_at: i64 = row.get(1)?;
    Ok(match row_of(row, run_id.clone(), order_at) {
        Ok(read) => Ok(read),
        Err(why) => Err(Unread {
            run_id,
            order_at,
            why,
        }),
    })
}

/// The rows of `range`, below the cursor when `after` is given: the
/// `WHERE` clause every read of a range states, and the values it binds.
/// An equality on the lane and at most one bound on `order_at` besides
/// the cursor's, so the index's range holds exactly the rows it returns.
fn filter(range: &Range, after: Option<(i64, &str)>) -> (String, Vec<Box<dyn ToSql>>) {
    let mut sql = "lane = ?1".to_string();
    let mut bound: Vec<Box<dyn ToSql>> = vec![Box::new(range.lane.as_str())];
    if let Some(limit) = range.bound {
        let (operator, at) = match limit {
            Bound::Below(at) => ("<", at),
            Bound::AtOrAbove(at) => (">=", at),
            Bound::Above(at) => (">", at),
            Bound::AtOrBelow(at) => ("<=", at),
        };
        bound.push(Box::new(at));
        sql.push_str(&format!(" AND order_at {operator} ?{}", bound.len()));
    }
    if let Some((order_at, run_id)) = after {
        bound.push(Box::new(order_at));
        bound.push(Box::new(run_id.to_string()));
        let (at, id) = (bound.len() - 1, bound.len());
        sql.push_str(&format!(" AND (order_at, run_id) < (?{at}, ?{id})"));
    }
    (sql, bound)
}

/// The order every range is read in: the index's own.
const ORDER: &str = "ORDER BY order_at DESC, run_id DESC";

/// `bound`, as the slice of values a statement binds.
fn values(bound: &[Box<dyn ToSql>]) -> Vec<&dyn ToSql> {
    bound.iter().map(|value| &**value).collect()
}

impl Store {
    /// Whether this journal is read a page at a time: it carries the
    /// summary and its backfill finished (ruling 8). A journal that does
    /// not is read whole, run by run, by a reader that cannot write to
    /// migrate it.
    pub fn pages(&self) -> Result<bool, StoreError> {
        if !summary_intact(&self.conn)? {
            return Ok(false);
        }
        let finished: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'run_summary_backfill'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(finished.is_some())
    }

    /// `read`, every statement it makes reading the journal as it stood
    /// at the first of them: one snapshot, so a listing that a writer
    /// moves the journal under reads one membership of its lanes, and
    /// lists each run once and none nowhere (ruling 6). A read already
    /// inside one is refused in SQLite's words, never read as two.
    pub fn as_one_read<T, E: From<StoreError>>(
        &self,
        read: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let snapshot = self
            .conn
            .unchecked_transaction()
            .map_err(StoreError::from)?;
        let read = read();
        // A read writes nothing, so its end is the same either way.
        drop(snapshot);
        read
    }

    /// Up to `limit` rows of `ranges`, below `after` when given, merged
    /// into the section's one order. Each range's statement is stepped
    /// only as far as the merge takes from it, and one row ahead.
    pub fn section(
        &self,
        ranges: &[Range],
        after: Option<(i64, &str)>,
        limit: usize,
    ) -> Result<Vec<Paged>, StoreError> {
        self.section_with(ranges, after, limit, |_, _| {})
    }

    /// [`Store::section`], handing `stepped` each range and the steps
    /// SQLite's machine took for its statement: how many rows it visited,
    /// returned or not, which the tests hold to what the page returned.
    fn section_with(
        &self,
        ranges: &[Range],
        after: Option<(i64, &str)>,
        limit: usize,
        mut stepped: impl FnMut(&Range, i32),
    ) -> Result<Vec<Paged>, StoreError> {
        let mut statements = Vec::with_capacity(ranges.len());
        let mut binds = Vec::with_capacity(ranges.len());
        for range in ranges {
            let (filter, bound) = filter(range, after);
            let sql = format!("SELECT {COLUMNS} FROM run_summary WHERE {filter} {ORDER}");
            statements.push(self.conn.prepare(&sql)?);
            binds.push(bound);
        }
        let mut cursors = Vec::with_capacity(ranges.len());
        for (statement, bound) in statements.iter_mut().zip(&binds) {
            cursors.push(statement.query(values(bound).as_slice())?);
        }
        let mut ahead: Vec<Option<Paged>> = Vec::with_capacity(cursors.len());
        for rows in &mut cursors {
            ahead.push(rows.next()?.map(paged).transpose()?);
        }
        let mut merged = Vec::new();
        while merged.len() < limit {
            let newest = ahead
                .iter()
                .enumerate()
                .filter_map(|(at, row)| row.as_ref().map(|row| (at, place_of(row))))
                .min_by_key(|(_, place)| newest_first(*place))
                .map(|(at, _)| at);
            let Some(at) = newest else { break };
            let next = cursors[at].next()?.map(paged).transpose()?;
            merged.extend(std::mem::replace(&mut ahead[at], next));
        }
        drop(cursors);
        for (range, statement) in ranges.iter().zip(&statements) {
            stepped(
                range,
                statement.get_status(rusqlite::StatementStatus::VmStep),
            );
        }
        Ok(merged)
    }

    /// Whether `run_id`'s last event is the head its row was folded to,
    /// by seq and hash, read by the run's key (ruling 5).
    pub fn stands_at(&self, run_id: &str, folded: (u64, &str)) -> Result<bool, StoreError> {
        let (seq, hash) = head_of(&self.conn, run_id)?;
        Ok((seq, hash.as_str()) == folded)
    }

    /// The first `limit` runs of the `behind` lane below `after`, newest
    /// first, each at the place its row stands and refused there where its
    /// id is past what a writer admits ([`keyed`]), and how many more it
    /// holds below `after`, counted by the index alone.
    pub fn behind(
        &self,
        after: Option<(i64, &str)>,
        limit: usize,
    ) -> Result<(Vec<Keyed>, u64), StoreError> {
        let behind = Range {
            lane: Lane::Behind,
            bound: None,
        };
        let (filter, mut bound) = filter(&behind, after);
        bound.push(Box::new(i64::try_from(limit).unwrap_or(i64::MAX)));
        let sql = format!(
            "SELECT order_at, run_id FROM run_summary WHERE {filter} {ORDER} LIMIT ?{}",
            bound.len()
        );
        let listed: Vec<Keyed> = self
            .conn
            .prepare(&sql)?
            .query_map(values(&bound).as_slice(), |row| {
                Ok(keyed(row.get(0)?, row.get(1)?))
            })?
            .collect::<Result<_, _>>()?;
        let beyond = self
            .counted_below(&behind, after)?
            .saturating_sub(listed.len() as u64);
        Ok((listed, beyond))
    }

    /// How many rows `range` holds, counted by the index alone.
    pub fn counted(&self, range: &Range) -> Result<u64, StoreError> {
        self.counted_below(range, None)
    }

    /// How many rows `range` holds below `after`, counted by the index.
    fn counted_below(&self, range: &Range, after: Option<(i64, &str)>) -> Result<u64, StoreError> {
        let (filter, bound) = filter(range, after);
        let count: i64 = self.conn.query_row(
            &format!("SELECT count(*) FROM run_summary WHERE {filter}"),
            values(&bound).as_slice(),
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /// `row` completed with the whole values its bounds cut, read by its
    /// run's key, and what the row keeps whole beyond them: what `brokkr
    /// runs`, its JSON and an opened run print (ruling 2). Residuals that
    /// do not read back into the findings a writer states are the row's
    /// refusal, a value no writer writes, and its run is read whole
    /// (ruling 5).
    pub fn whole(&self, row: &mut Row) -> Result<Result<Whole, RowRefusal>, StoreError> {
        let (feature, residuals, values): (String, String, [Option<String>; 8]) =
            self.conn.query_row(
                "SELECT feature, residuals, created_at_whole, last_recorded_at_whole,
                        phase_whole, park_reason_whole, rule_id_whole, rule_from_whole,
                        hire_seat_whole, refusal_words_whole
                 FROM run_summary WHERE run_id = ?1",
                [&row.run_id],
                |found| {
                    let mut values: [Option<String>; 8] = Default::default();
                    for (at, value) in values.iter_mut().enumerate() {
                        *value = found.get(at + 2)?;
                    }
                    Ok((found.get(0)?, found.get(1)?, values))
                },
            )?;
        let [created_at, last_recorded_at, phase, park_reason, rule_id, rule_from, seat, words] =
            values;
        row.created_at.whole = created_at;
        let fill = |bounded: Option<&mut Bounded>, whole: Option<String>| {
            if let Some(bounded) = bounded {
                bounded.whole = whole;
            }
        };
        fill(row.last_recorded_at.as_mut(), last_recorded_at);
        fill(row.phase.as_mut(), phase);
        fill(row.park_reason.as_mut(), park_reason);
        let ruled = row.ruled.as_mut();
        let (ruled_id, from) = match ruled {
            Some(ruled) => (ruled.rule_id.as_mut(), Some(&mut ruled.from)),
            None => (None, None),
        };
        fill(ruled_id, rule_id);
        fill(from, rule_from);
        fill(row.hired.as_mut().map(|hired| &mut hired.seat), seat);
        fill(
            row.refused.as_mut().map(|refused| &mut refused.words),
            words,
        );
        let residuals: Vec<Stated> = match serde_json::from_str(&residuals) {
            Ok(residuals) => residuals,
            Err(unread) => return Ok(Err(RowRefusal::Residuals(unread.to_string()))),
        };
        let acknowledged = standing(&self.conn, &row.run_id)?.map(|standing| Copied {
            operator: standing.operator,
            reason: standing.reason,
            at: standing.recorded_at,
        });
        Ok(Ok(Whole {
            feature,
            residuals,
            acknowledged,
        }))
    }

    /// What `run_id`'s `runs` row lists: its feature and creation stamp.
    pub fn listing(&self, run_id: &str) -> Result<(String, String), StoreError> {
        self.conn
            .query_row(
                "SELECT feature, created_at FROM runs WHERE run_id = ?1",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or_else(|| StoreError::RunNotFound(run_id.to_string()))
    }
}

#[cfg(test)]
mod tests;
