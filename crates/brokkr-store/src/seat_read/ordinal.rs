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
//! sequential effects on one attempt stay apart by their rows' bounds
//! (M1). The ordinals restart at 1 wherever another effect's row breaks
//! the index stream (H2), so a span whose stream holds such a restart, or
//! a row still to number, cannot be counted by difference: the read says
//! it cannot stand for the participant ([`SeatRead::Kept`]) and the reader
//! reads the run the kept way (ruling 8). A counted row carries a positive
//! ordinal; a row the guarded page read does not count carries 0, which a
//! member's stream refuses within its reach.
//!
//! Where the guarded page read refuses, these reads refuse for the reach
//! they cover, and only there: the seat's own participant wherever a
//! member's not-counted row of the span's attempt lies in the range (the
//! result-only record its fold counts but the index keys by the result's
//! member, H1), a member's wherever its stream holds one, and any
//! participant wherever a row still to number or a restart lies in it. A
//! member's own checkpoint is a positive ordinal and another participant's
//! row, so it refuses neither the seat's count nor a member's.
//! `seat_count` covers every span whole; `seat_seq_at` covers each span's
//! rows at or below the seq it returns. No preliminary count over the
//! whole participant refuses a window it never reaches (M2).

use std::sync::LazyLock;

use rusqlite::{params, OptionalExtension, Params};

use super::{Gathered, Seat, Stream};
use crate::event_index::{place, CHECKPOINTED};
use crate::{patiently, Opened, Store, StoreError};

/// The two ordinals a stream's row bounds name, by the stream index: a
/// stream whose bound row is missing, or whose ordinal is 0 or NULL, cannot
/// be counted.
static ORDINALS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq, stream_ordinal FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3 AND seq IN (?4, ?5)
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
static RESTART: LazyLock<String> = LazyLock::new(|| {
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
/// refuses, which the ordinal never reads as a place.
static UNCOUNTED: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND stream_ordinal = 0
           AND seq BETWEEN ?4 AND ?5)"
    )
});

/// The seq holding `ordinal` within a stream's row bounds, by the written
/// index.
static AT: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq FROM event_index INDEXED BY event_index_ordinal_written
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND type = '{CHECKPOINTED}' AND stream_ordinal = ?6
           AND seq BETWEEN ?4 AND ?5"
    )
});

/// The last ordinal a stream's rows hold at or below `seq`: how many of its
/// checkpoints a candidate seq reaches, for the rank search among streams
/// whose seq ranges interleave.
static BEFORE: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT stream_ordinal FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3
           AND seq BETWEEN ?4 AND ?5 AND type = '{CHECKPOINTED}'
         ORDER BY seq DESC LIMIT 1"
    )
});

/// A participant's stream as its count and rank read it: the attempt its
/// seat row names, the row's first and last checkpoint, and their ordinals.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Span {
    attempt: String,
    first: u64,
    last: u64,
    first_ord: u64,
    last_ord: u64,
}

impl Span {
    /// How many of the participant's checkpoints the stream holds: the
    /// difference of the ordinals at its bounds, plus one. It is the count
    /// only where [`Store::refuses`] finds no restart inside the span.
    fn count(&self) -> u64 {
        self.last_ord - self.first_ord + 1
    }
}

/// The spans of `streams`, the participant `member`'s, by the ordinals
/// their rows' bounds name: none where a bound row is missing, carries no
/// positive ordinal, or falls below its first.
fn spans(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    streams: &[Stream],
) -> Result<Option<Vec<Span>>, StoreError> {
    let mut spans = Vec::with_capacity(streams.len());
    for stream in streams {
        let bound = params![
            run_id,
            stream.attempt,
            member,
            place(stream.first),
            place(stream.last)
        ];
        let mut statement = conn.prepare_cached(&ORDINALS)?;
        let mut rows = statement.query(bound)?;
        let (mut first_ord, mut last_ord) = (None, None);
        while let Some(row) = rows.next()? {
            let seq: u64 = row.get(0)?;
            let ordinal: Option<i64> = row.get(1)?;
            let Some(ordinal) = ordinal.and_then(|ordinal| u64::try_from(ordinal).ok()) else {
                return Ok(None);
            };
            if ordinal == 0 {
                return Ok(None);
            }
            if seq == stream.first {
                first_ord = Some(ordinal);
            }
            if seq == stream.last {
                last_ord = Some(ordinal);
            }
        }
        drop(rows);
        let (Some(first_ord), Some(last_ord)) = (first_ord, last_ord) else {
            return Ok(None);
        };
        if last_ord < first_ord {
            return Ok(None);
        }
        spans.push(Span {
            attempt: stream.attempt.clone(),
            first: stream.first,
            last: stream.last,
            first_ord,
            last_ord,
        });
    }
    Ok(Some(spans))
}

/// The seq of the participant's `rank`-th checkpoint (0-based), none past
/// its end: the stream that holds it is found by the counts, and the seq by
/// its ordinal, unless the streams' seq ranges interleave, where the rank
/// is found by asking each stream how many checkpoints a candidate seq
/// reaches.
fn seq_at(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    spans: &[Span],
    rank: u64,
) -> Result<Option<u64>, StoreError> {
    let total: u64 = spans.iter().map(Span::count).sum();
    if rank >= total {
        return Ok(None);
    }
    let mut ordered: Vec<&Span> = spans.iter().collect();
    ordered.sort_unstable_by_key(|span| span.first);
    let disjoint = ordered.windows(2).all(|pair| pair[0].last < pair[1].first);
    if disjoint {
        let mut before = 0u64;
        for span in ordered {
            if rank < before + span.count() {
                let ordinal = span.first_ord + (rank - before);
                let found = conn
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
                return Ok(found);
            }
            before += span.count();
        }
        return Ok(None);
    }
    let mut low = ordered.first().map_or(0, |span| span.first);
    let mut high = ordered.iter().map(|span| span.last).max().unwrap_or(0);
    while low < high {
        let mid = low + (high - low) / 2;
        let Some(reached) = before(conn, run_id, member, spans, mid)? else {
            return Ok(None);
        };
        if reached > rank {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Ok(Some(low))
}

/// How many of the participant's checkpoints `seq` reaches, by each
/// stream's last ordinal at or below it: none where a candidate row is
/// unnumbered or carries the not-counted value.
fn before(
    conn: &rusqlite::Connection,
    run_id: &str,
    member: Option<&str>,
    spans: &[Span],
    seq: u64,
) -> Result<Option<u64>, StoreError> {
    let mut reached = 0u64;
    for span in spans {
        if seq < span.first {
            continue;
        }
        if seq >= span.last {
            reached += span.count();
            continue;
        }
        let ordinal: Option<i64> = conn
            .query_row(
                &BEFORE,
                params![run_id, span.attempt, member, place(span.first), place(seq)],
                |row| row.get(0),
            )
            .optional()?;
        let Some(ordinal) = ordinal.and_then(|ordinal| u64::try_from(ordinal).ok()) else {
            return Ok(None);
        };
        if ordinal < span.first_ord {
            return Ok(None);
        }
        reached += ordinal - span.first_ord + 1;
    }
    Ok(Some(reached))
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
    /// reach refuses.
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
            total += span.count();
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
            None => self.uncounted_across(run_id, &span.attempt, low, high),
        }
    }

    /// Whether any member of `attempt` holds a row still carrying the
    /// not-counted value inside the range: the result-only record the
    /// seat's own fold counts but the index keys by its result's member
    /// (H1). A member's own checkpoint carries a positive ordinal and is
    /// another participant's, so it does not refuse the seat's count; the
    /// work is the attempt's members, as the page read's is.
    fn uncounted_across(
        &self,
        run_id: &str,
        attempt: &str,
        low: u64,
        high: u64,
    ) -> Result<bool, StoreError> {
        let found = |named: &str| self.uncounted_member(run_id, attempt, named, low, high);
        self.any_member(run_id, attempt, |_, named| found(named))
    }

    /// Whether member `named` of `attempt` holds a not-counted row between
    /// `low` and `high`, by the member's stream index.
    fn uncounted_member(
        &self,
        run_id: &str,
        attempt: &str,
        named: &str,
        low: u64,
        high: u64,
    ) -> Result<bool, StoreError> {
        let bound = params![run_id, attempt, Some(named), place(low), place(high)];
        exists(&self.conn, &UNCOUNTED, bound)
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
            let Some(spans) = spans(&self.conn, run_id, seat.1, &streams)? else {
                return Ok(None);
            };
            read(self, &spans)
        })
    }
}
