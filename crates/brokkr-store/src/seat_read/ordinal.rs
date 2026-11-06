//! A participant's checkpoint count and the seq at a rank, read from the
//! ordinals the writer keeps (#994) rather than by walking its pages: each
//! of the participant's streams is counted as the difference of the
//! ordinals at its row's first and last checkpoint, and the seq at a rank
//! is found through those ordinals. The work is the participant's streams
//! (at most [`STREAMS`]), not its checkpoints, so a fresh end or middle
//! page reads its page and a bounded number of seeks whatever the
//! participant's history.
//!
//! A stream is the seat row's attempt and the participant's member, and
//! its bounds are the row's own first and last checkpoint, so two
//! sequential effects on one attempt stay apart by their rows' bounds.
//! The ordinals restart at 1 wherever another effect's row breaks the
//! index stream (H2), so a span whose stream holds such a restart, or a
//! row still to number, cannot be counted by difference: the read says it
//! cannot stand for the participant ([`SeatRead::Kept`]) and the reader
//! reads the run the kept way (ruling 8). A counted row carries a positive
//! ordinal; a row the guarded page read does not count carries 0, which a
//! member's stream refuses within its reach.
//!
//! A participant whose spans interleave, where in seq order one span's
//! first lies at or below the previous span's last, cannot be counted by
//! difference either: `seat_count` and `seat_seq_at` take the kept way
//! before any rank work, and the reader reads the run whole (ruling 8).
//!
//! Where the guarded page read refuses, these reads refuse for the reach
//! they cover, and only there: the seat's own participant wherever a
//! member's checkpoint of the span's attempt lies in the range (the
//! guarded page read's own `members_within`, H1), a member's wherever its
//! stream holds a not-counted row, and any participant wherever a row
//! still to number or a restart lies in it. A member's own checkpoint is a
//! positive ordinal and a row of the seat's own participant, so it refuses
//! the seat's count within its reach, as the page read does, while a
//! member's rank never reads it: the member's stream keys only its own
//! rows. `seat_count` covers every span whole; `seat_seq_at` covers each
//! span's rows at or below the seq it returns, and reads a span's last row
//! only where its count is needed.
//!
//! Every bound the final reach rests on is read by key before it decides:
//! each span's first row, and the last row of every span the rank passes.
//! A missing row, a NULL or an ordinal that is not positive refuses, and a
//! span's first row must carry exactly 1: every valid first is 1, since
//! the stream's first numbered row of an effect, or the first after
//! another effect's row, is numbered 1, and a restart inside a span
//! already refuses. Every ordinal read decodes a NULL as none rather than
//! an SQL type error.

use std::sync::LazyLock;

use rusqlite::{params, OptionalExtension, Params};

use super::{Gathered, Seat, Stream};
use crate::event_index::{place, CHECKPOINTED};
use crate::{patiently, Opened, Store, StoreError};

/// The ordinal one row of a stream carries, where it carries a positive
/// one: a row missing, still to number, or not counted is no bound.
pub(super) static BOUND: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT stream_ordinal FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3 AND seq = ?4
           AND type = '{CHECKPOINTED}'"
    )
});

/// Whether a row still to number lies inside the range: a row the read
/// would count if its ordinal were written, which it must never read as
/// zero. The stale partial index names the unnumbered rows alone, keyed by
/// the stream they belong to, so a journal whose ordinals stand reads one
/// empty range and another member's or attempt's stale rows are never
/// scanned (M3).
pub(super) static GAP: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index
         INDEXED BY event_index_ordinal_stale
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND stream_ordinal IS NULL
           AND seq BETWEEN ?4 AND ?5)"
    )
});

/// Whether the stream restarts inside the range: a 1 past the span's first
/// row is another effect's row (or an interleaved one), so the ordinals no
/// longer count one effect's checkpoints by difference. The written index
/// names the counted rows alone, so this is one seek.
pub(super) static RESTART: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index
         INDEXED BY event_index_ordinal_written
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND stream_ordinal = 1
           AND seq > ?4 AND seq <= ?5)"
    )
});

/// Whether a checkpoint row of a member's stream inside the range carries
/// the not-counted value (0): a record the guarded page read's `names`
/// refuses, which the ordinal never reads as a place. The written index
/// keys the value, so a journal with none reads one empty seek whatever
/// the stream holds (M1).
pub(super) static UNCOUNTED: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index INDEXED BY event_index_ordinal_written
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND stream_ordinal = 0
           AND seq BETWEEN ?4 AND ?5)"
    )
});

/// The seq holding `ordinal` within a stream's row bounds, by the written
/// index.
pub(super) static AT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_ordinal_written
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND stream_ordinal = ?6
           AND seq BETWEEN ?4 AND ?5"
    )
});

/// A participant's stream as its count and rank read it: the attempt its
/// seat row names, and the row's first and last checkpoint. Its ordinals
/// are read only where a read needs them, so a span above the returned
/// rank is never refused for a bound it does not reach (M2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    attempt: String,
    first: u64,
    last: u64,
}

/// The spans of `streams`, the participant `member`'s, by the bounds their
/// rows name.
fn spans(streams: &[Stream]) -> Vec<Span> {
    streams
        .iter()
        .map(|stream| Span {
            attempt: stream.attempt.clone(),
            first: stream.first,
            last: stream.last,
        })
        .collect()
}

/// Whether any two of `spans` interleave: in seq order, one span's first
/// lies at or below the previous span's last. A participant whose spans do
/// cannot be counted or ranked by difference, so both reads take the kept
/// way.
fn interleaved(spans: &[Span]) -> bool {
    let mut ordered: Vec<&Span> = spans.iter().collect();
    ordered.sort_unstable_by_key(|span| span.first);
    ordered.windows(2).any(|pair| pair[1].first <= pair[0].last)
}

/// The positive ordinal `span`'s row at `seq` carries: none where the row
/// is missing, still to number, or not counted.
fn bound(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    span: &Span,
    seq: u64,
) -> Result<Option<u64>, StoreError> {
    let ordinal: Option<Option<i64>> = conn
        .query_row(
            &BOUND,
            params![run_id, span.attempt, member, place(seq)],
            |row| row.get(0),
        )
        .optional()?;
    Ok(positive(ordinal.flatten()))
}

/// The positive ordinal `span`'s first row carries, where it is a valid
/// first: exactly 1. Every valid first is 1, since the stream's first
/// numbered row of an effect, or the first after another effect's row, is
/// numbered 1. A missing row, a NULL, a not-counted 0 and any other
/// positive value are no bound the walk may rest on.
fn first_bound(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    span: &Span,
) -> Result<Option<u64>, StoreError> {
    Ok(bound(conn, run_id, member, span, span.first)?.filter(|ordinal| *ordinal == 1))
}

/// A stored ordinal as a place, where it is one: NULL, a negative and the
/// not-counted 0 are none. Every read of the column takes this, so a NULL
/// never surfaces as an SQL type error (M4).
fn positive(ordinal: Option<i64>) -> Option<u64> {
    ordinal
        .and_then(|ordinal| u64::try_from(ordinal).ok())
        .filter(|ordinal| *ordinal > 0)
}

/// How many checkpoints a span holds between its first and last ordinal,
/// none where the last falls below the first.
fn held(first_ord: u64, last_ord: u64) -> Option<u64> {
    last_ord
        .checked_sub(first_ord)
        .map(|difference| difference + 1)
}

/// The seq of the participant's `rank`-th checkpoint (0-based), none past
/// its end: the participant's streams stand apart, so the span that holds
/// it is found by the counts and the seq by its ordinal.
fn seq_at(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    spans: &[Span],
    rank: u64,
) -> Result<Option<u64>, StoreError> {
    let mut ordered: Vec<&Span> = spans.iter().collect();
    ordered.sort_unstable_by_key(|span| span.first);
    seq_in_order(conn, run_id, member, &ordered, rank)
}

/// The seq of the participant's `rank`-th checkpoint among `ordered` spans
/// whose seq ranges stand apart: the span that holds it is walked to by the
/// counts, and its last row is read only where the rank passes it. Every
/// first row and every passed last row is read by key and refused where it
/// is missing, NULL or not the positive place it must be.
fn seq_in_order(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    ordered: &[&Span],
    rank: u64,
) -> Result<Option<u64>, StoreError> {
    let mut before = 0u64;
    for span in ordered {
        let Some(first_ord) = first_bound(conn, run_id, member, span)? else {
            return Ok(None);
        };
        let Some(ordinal) = rank
            .checked_sub(before)
            .and_then(|offset| first_ord.checked_add(offset))
        else {
            return Ok(None);
        };
        let found: Option<u64> = conn
            .query_row(
                &AT,
                params![
                    run_id,
                    span.attempt,
                    member,
                    place(span.first),
                    place(span.last),
                    place(ordinal)
                ],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(seq) = found {
            return Ok(Some(seq));
        }
        let Some(last_ord) = bound(conn, run_id, member, span, span.last)? else {
            return Ok(None);
        };
        let Some(count) = held(first_ord, last_ord) else {
            return Ok(None);
        };
        before = before.saturating_add(count);
    }
    Ok(None)
}

/// Whether `sql` selects a row with `bound`.
fn exists(conn: &rusqlite::Connection, sql: &str, bound: impl Params) -> Result<bool, StoreError> {
    let mut statement = conn.prepare_cached(sql)?;
    Ok(statement.query_row(bound, |row| row.get(0))?)
}

impl Store {
    /// How many checkpoints the participant `seat` holds at `opened`, by
    /// its streams' ordinals alone: none where the seat view's read is
    /// kept, or the participant's whole reach holds a refusal. The count
    /// is the rank the participant level's window asks for, so a page
    /// starts at it rather than reading every page before it (decision
    /// 0089 ruling 9).
    pub fn seat_count(
        &self,
        run_id: &str,
        opened: &Opened,
        seat: Seat<'_>,
    ) -> Result<Option<u64>, StoreError> {
        self.ordinal_span(run_id, opened, seat, |store, spans| {
            store.span_count(run_id, seat.1, spans)
        })
    }

    /// The seq of the participant `seat`'s `rank`-th checkpoint (0-based)
    /// at `opened`, by its streams' ordinals alone: none where the read is
    /// kept or the participant holds no row at `rank`. The caller reads one
    /// page after it, so the page's rows are the only envelopes decoded.
    pub fn seat_seq_at(
        &self,
        run_id: &str,
        opened: &Opened,
        seat: Seat<'_>,
        rank: u64,
    ) -> Result<Option<u64>, StoreError> {
        self.ordinal_span(run_id, opened, seat, |store, spans| {
            store.rank_seq(&store.conn, run_id, seat.1, spans, rank)
        })
    }

    /// The participant's count over `spans`, whole: none where any span's
    /// reach refuses, or any span's first or last row is not the positive
    /// place it must be.
    fn span_count(
        &self,
        run_id: &str,
        member: Option<&str>,
        spans: &[Span],
    ) -> Result<Option<u64>, StoreError> {
        let mut total = 0u64;
        for span in spans {
            if self.refuses(run_id, member, span, span.first, span.last)? {
                return Ok(None);
            }
            let Some(first_ord) = first_bound(&self.conn, run_id, member, span)? else {
                return Ok(None);
            };
            let Some(last_ord) = bound(&self.conn, run_id, member, span, span.last)? else {
                return Ok(None);
            };
            let Some(count) = held(first_ord, last_ord) else {
                return Ok(None);
            };
            total = total.saturating_add(count);
        }
        Ok(Some(total))
    }

    /// The seq of the participant's `rank`-th checkpoint, none where a span
    /// at or below that seq refuses: the reach is every span whole below
    /// the seq and every span from its first up to it.
    fn rank_seq(
        &self,
        conn: &rusqlite::Connection,
        run_id: &str,
        member: Option<&str>,
        spans: &[Span],
        rank: u64,
    ) -> Result<Option<u64>, StoreError> {
        let Some(seq) = seq_at(conn, run_id, member, spans, rank)? else {
            return Ok(None);
        };
        for span in spans {
            if span.first > seq {
                continue;
            }
            let high = span.last.min(seq);
            if self.refuses(run_id, member, span, span.first, high)? {
                return Ok(None);
            }
        }
        Ok(Some(seq))
    }

    /// Whether the span's `[low, high]` reach holds what the guarded page
    /// read would refuse: a row still to number, a restart past the span's
    /// first row, or a not-counted row, which the seat's own takes from any
    /// member of the attempt and a member's from its own stream.
    fn refuses(
        &self,
        run_id: &str,
        member: Option<&str>,
        span: &Span,
        low: u64,
        high: u64,
    ) -> Result<bool, StoreError> {
        let gap = params![run_id, span.attempt, member, place(low), place(high)];
        if exists(&self.conn, &GAP, gap)? {
            return Ok(true);
        }
        let restart = params![run_id, span.attempt, member, place(span.first), place(high)];
        if exists(&self.conn, &RESTART, restart)? {
            return Ok(true);
        }
        match member {
            Some(_) => {
                let uncounted = params![run_id, span.attempt, member, place(low), place(high)];
                exists(&self.conn, &UNCOUNTED, uncounted)
            }
            None => {
                // The seat's own stream's own not-counted row is not a
                // member's, so the attempt's member walk cannot see it; the
                // guarded page read still refuses it inside its reach.
                let own = params![run_id, span.attempt, member, place(low), place(high)];
                if exists(&self.conn, &UNCOUNTED, own)? {
                    return Ok(true);
                }
                self.members_within(run_id, &span.attempt, low, high)
            }
        }
    }

    /// `read` over the spans of the participant `seat` at `opened`, in one
    /// snapshot: none where the seat view's read would be kept, or the
    /// participant's streams are more than [`super::STREAMS`].
    fn ordinal_span<T>(
        &self,
        run_id: &str,
        opened: &Opened,
        seat: Seat<'_>,
        mut read: impl FnMut(&Store, &[Span]) -> Result<Option<T>, StoreError>,
    ) -> Result<Option<T>, StoreError> {
        patiently("load", self.patience, || {
            let _snapshot = self.conn.unchecked_transaction()?;
            if !self.headed_at(run_id, opened)? {
                return Ok(None);
            }
            // Every stream: a checkpoint's seq is at least 1, so above 0
            // keeps all, and more than STREAMS refuses, as a page does.
            let Some(streams) = self.gather(run_id, opened, seat, Gathered::above(0))? else {
                return Ok(None);
            };
            let spans = spans(&streams);
            if interleaved(&spans) {
                return Ok(None);
            }
            read(self, &spans)
        })
    }
}
