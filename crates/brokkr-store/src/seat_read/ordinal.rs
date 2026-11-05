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
//! (M1). A row whose ordinal is NULL within the stream - a row the
//! writer's guard does not count, or one a writer from before the column
//! or a lost trigger left unnumbered - is never read as zero: the read
//! says it cannot stand for the participant ([`SeatRead::Kept`]) and the
//! reader reads the run the kept way (ruling 8). The next writable open's
//! backfill numbers it.
//!
//! No preliminary count over the whole participant refuses a window: the
//! count is made only of the ordinals the participant's rows name, never
//! of the member records an attempt holds, so the interleaving a page at
//! the top never reaches does not refuse it (M2).

use std::sync::LazyLock;

use rusqlite::{params, OptionalExtension};

use super::{Gathered, Seat, Stream};
use crate::event_index::{place, CHECKPOINTED};
use crate::{patiently, Opened, Store, StoreError};

/// The two ordinals a stream's row bounds name, by the stream index: a
/// stream whose bound row is missing, or whose ordinal is NULL, cannot be
/// counted.
static ORDINALS: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT seq, stream_ordinal FROM event_index INDEXED BY event_index_stream
         WHERE run_id = ?1 AND attempt_id IS ?2 AND member IS ?3 AND seq IN (?4, ?5)
           AND type = '{CHECKPOINTED}'"
    )
});

/// Whether a checkpoint row inside the stream's bounds is unnumbered: a
/// row the read would count if its ordinal were written, which it must
/// never read as zero. The stale partial index names the unnumbered rows
/// alone, so a journal whose ordinals stand reads one empty range, never
/// the stream.
static GAP: LazyLock<String> = LazyLock::new(|| {
    format!(
        "SELECT EXISTS (SELECT 1 FROM event_index
         INDEXED BY event_index_ordinal_stale
         WHERE run_id = ?1 AND type = '{CHECKPOINTED}'
           AND stream_ordinal IS NULL
           AND attempt_id IS ?2 AND member IS ?3 AND seq BETWEEN ?4 AND ?5)"
    )
});

/// The seq holding `ordinal` within a stream's row bounds, by the stream
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
    /// difference of the ordinals at its bounds, plus one.
    fn count(&self) -> u64 {
        self.last_ord - self.first_ord + 1
    }
}

/// The spans of `streams`, the participant `member`'s, by the ordinals
/// their rows' bounds name: none where a bound row is missing or NULL, or
/// a checkpoint inside them is unnumbered.
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
        let gap: bool = conn.query_row(
            &GAP,
            params![
                run_id,
                stream.attempt,
                member,
                place(stream.first),
                place(stream.last)
            ],
            |row| row.get(0),
        )?;
        if gap {
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
/// unnumbered.
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
        reached += ordinal - span.first_ord + 1;
    }
    Ok(Some(reached))
}

impl Store {
    /// How many checkpoints the participant `seat` holds at `opened`, by
    /// its streams' ordinals alone: none where the seat view's read is
    /// kept, or a stream cannot stand for its row's checkpoints. The count
    /// is the rank the participant level's window asks for, so a page
    /// starts at it rather than reading every page before it (decision
    /// 0089 ruling 9).
    pub fn seat_count(
        &self,
        run_id: &str,
        opened: &Opened,
        seat: Seat<'_>,
    ) -> Result<Option<u64>, StoreError> {
        self.ordinal_span(run_id, opened, seat, |spans| {
            Ok(Some(spans.iter().map(Span::count).sum()))
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
        self.ordinal_span(run_id, opened, seat, |spans| {
            seq_at(&self.conn, run_id, seat.1, spans, rank)
        })
    }

    /// `read` over the spans of the participant `seat` at `opened`, in one
    /// snapshot: none where the seat view's read would be kept, or the
    /// participant's streams are more than [`super::STREAMS`].
    fn ordinal_span<T>(
        &self,
        run_id: &str,
        opened: &Opened,
        seat: Seat<'_>,
        mut read: impl FnMut(&[Span]) -> Result<Option<T>, StoreError>,
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
            read(&spans)
        })
    }
}
