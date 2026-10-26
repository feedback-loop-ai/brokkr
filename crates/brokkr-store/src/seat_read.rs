//! The seat view's read (decision 0089 ruling 7, slice 2b): after a run's
//! opening ([`Store::open_run`]), a participant's seat rows and a page of
//! 50 of its checkpoints, newest first, through `event_index`'s stream
//! index, keyed `(run_id, attempt_id, member, seq)`, all read in one
//! snapshot at the head the opening verified, as the run view's read is
//! ([`Store::run_read`]).
//!
//! A participant is an effect and a member, as the run view keys it,
//! across its requests and attempts. Its effect's seat rows are read by
//! the run's key and the effect, one at a time, so no other effect's row
//! is held, and of each only the columns its stream is read by ([`HELD`]),
//! its digest checked a value at a time, so no row's lists are held. Each
//! of its rows that holds a checkpoint names the attempt
//! its stream is read by, and its first and last checkpoint's seq, which
//! bound the stream. A page keeps only the streams that can reach it
//! ([`Gathered`]): once the streams kept vouch for a page and one more
//! checkpoints at or above a seq, their rows' first and last, a stream
//! wholly below that seq is dropped, so what a page holds is the page's,
//! at most [`STREAMS`] streams, whatever the participant's history. The
//! streams are read newest first, a page and one more kept of their
//! checkpoints, until no stream left can reach the page. The caller is
//! handed, once a page, the start each attempt on the page last names, the
//! seq its effect's rows of that attempt keep as their last start, read by
//! its key one at a time, so the page holds one start at a time while it
//! makes of each what a checkpoint's row reads its boundary from; each
//! checkpoint on the page is then parsed, made into its row and dropped, so
//! the page holds one checkpoint's envelope at a time (ruling 9), whatever
//! the participant's other attempts and the run's other seats hold.
//!
//! Where the streams cannot stand for the participant's checkpoints, the
//! read says so ([`SeatRead::Kept`]) and the reader reads the run the kept
//! way (ruling 8): where the run view's read would ([`Store::run_read`]),
//! or the effect's rows would read stale; where a row's checkpoints name
//! no attempt, which no stream keys; two of the participant's rows that
//! a page keeps are keyed by one attempt; more than [`STREAMS`] streams
//! can reach the page; a stream the page reads does not begin at its
//! row's last checkpoint or end at its first; a checkpoint on the page
//! names another effect or member than the participant's; an attempt on
//! the page has no start its effect's rows keep; or, for the seat's own,
//! a member's checkpoint of its attempt stands within the page's reach.
//! That last is a record the index keys by the member its result names,
//! where it carries no `checkpoint`, and the seat rows read as the seat's
//! own: only its body tells it from the member's own checkpoint, and no
//! body off the page is read, so the read takes the kept way.

use std::cmp::Reverse;

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::seat::row::Datum;
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

use crate::event_index::{place, seqs, CHECKPOINT_PAGE, STREAM};
use crate::seat_summary::effect_rows_at;
use crate::{Opened, Store, StoreError};

/// What the seat view reads of one participant at the head its opening
/// verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SeatRead<T> {
    /// Read at that head: a page of at most 50 of the participant's
    /// checkpoints, newest first, each the row the caller made of it; and
    /// the cursor the next, older page is asked before, the seq of this
    /// page's oldest checkpoint, none where no older one is left.
    Current(Vec<T>, Option<u64>),
    /// Not read at that head, for a reason the module names. The reader
    /// reads the run the kept way (ruling 8).
    Kept,
}

/// The places of the columns a row's stream is read by
/// (`SeatRow::COLUMNS`): its attempt and its member, the seq of its
/// attempt's last start, and of its first and last checkpoint.
const ATTEMPT: usize = 2;
const MEMBER: usize = 3;
const SINCE: usize = 8;
const FIRST: usize = 9;
const LAST: usize = 11;

/// The columns of a row the read holds, through its last checkpoint's
/// seq: the rest, its lists among them, are checked by its digest and
/// never held ([`effect_rows_at`]).
const HELD: usize = LAST + 1;

/// The least member at or above `?3` an attempt's index rows name, read
/// by the stream index's key.
const LEAST: &str = "SELECT member FROM event_index INDEXED BY event_index_stream
     WHERE run_id = ?1 AND attempt_id = ?2 AND member >= ?3 ORDER BY member LIMIT 1";

/// The least member above `?3` an attempt's index rows name.
const NEXT: &str = "SELECT member FROM event_index INDEXED BY event_index_stream
     WHERE run_id = ?1 AND attempt_id = ?2 AND member > ?3 ORDER BY member LIMIT 1";

/// The most streams a page keeps (ruling 9): a page and one more
/// checkpoints, each of an attempt of its own, the stream a page's cursor
/// cuts, and room for attempts whose checkpoints interleave. A participant
/// with more streams that can reach a page reads the kept way.
pub(crate) const STREAMS: usize = 64;

/// A participant: its effect, and its member, `None` for the seat's own.
pub(crate) type Seat<'a> = (&'a str, Option<&'a str>);

/// One stream of a participant's checkpoints: the attempt its row names,
/// and the seq of the row's first and last checkpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stream {
    attempt: String,
    first: u64,
    last: u64,
}

/// A checkpoint the streams found: its seq, and the place of its stream.
type Found = (u64, usize);

/// What a participant's effect's rows keep of the streams that can reach
/// the page below `before`, gathered a row at a time ([`Gathered::row`]).
#[derive(Debug)]
struct Gathered {
    before: Option<u64>,
    /// The streams that can reach the page, at most [`STREAMS`] and one
    /// more, as many as their capacity holds.
    streams: Vec<Stream>,
    /// Whether a row of the participant that holds checkpoints names no
    /// attempt, or one a stream kept names, whose stream would hold both;
    /// or more than [`STREAMS`] streams can reach the page.
    refused: bool,
}

impl Gathered {
    fn below(before: Option<u64>) -> Gathered {
        Gathered {
            before,
            streams: Vec::with_capacity(STREAMS + 1),
            refused: false,
        }
    }

    /// Keep what `data`, a row of the participant's effect, keeps of the
    /// participant `member`'s streams that can reach the page.
    fn row(&mut self, data: &[Datum], member: &Datum) {
        let (Datum::Integer(first), Datum::Integer(last)) = (&data[FIRST], &data[LAST]) else {
            return;
        };
        if self.refused || data[MEMBER] != *member {
            return;
        }
        let Datum::Text(attempt) = &data[ATTEMPT] else {
            self.refused = true;
            return;
        };
        if self.streams.iter().any(|kept| kept.attempt == *attempt) {
            self.refused = true;
            return;
        }
        let stream = Stream {
            attempt: attempt.clone(),
            first: *first as u64,
            last: *last as u64,
        };
        // A stream with no checkpoint below the cursor is none of the
        // page's, and no stream kept holds none: `streamed` reads each from
        // its first checkpoint.
        if upper(&stream, self.before) >= stream.first {
            self.streams.push(stream);
            self.reach();
        }
    }

    /// Drop every stream wholly below the floor: the first checkpoint of
    /// the stream, its streams newest first by their first, at which those
    /// kept vouch for a page and one more ([`vouched`]). The page's
    /// checkpoints and the one after it stand at or above it.
    fn reach(&mut self) {
        let before = self.before;
        self.streams
            .sort_unstable_by_key(|stream| Reverse(stream.first));
        let mut held = 0;
        let floor = self.streams.iter().find_map(|stream| {
            held += vouched(stream, before);
            (held > CHECKPOINT_PAGE).then_some(stream.first)
        });
        if let Some(floor) = floor {
            self.streams.retain(|stream| upper(stream, before) >= floor);
        }
        self.refused |= self.streams.len() > STREAMS;
    }

    /// The streams that can reach the page: none where a row refused them.
    fn streams(self) -> Option<Vec<Stream>> {
        (!self.refused).then_some(self.streams)
    }
}

impl Store {
    /// The page of the participant `seat` below `before`, inside the seat
    /// view's read ([`Store::seat_read`]), at `opened`, the head its
    /// opening verified, which the read stands at.
    pub(crate) fn seat_page<T, R: FnMut(&EventEnvelope) -> T>(
        &self,
        run_id: &str,
        opened: &Opened,
        seat: Seat<'_>,
        before: Option<u64>,
        rows: &mut impl FnMut(&mut dyn Iterator<Item = EventEnvelope>) -> R,
    ) -> Result<SeatRead<T>, StoreError> {
        let Some(mut streams) = self.streams(run_id, opened, seat, before)? else {
            return Ok(SeatRead::Kept);
        };
        let Some(mut found) = self.found(run_id, seat.1, &mut streams, before)? else {
            return Ok(SeatRead::Kept);
        };
        let cut = found.get(CHECKPOINT_PAGE).map_or(0, |(seq, _)| seq + 1);
        let next = (found.len() > CHECKPOINT_PAGE).then(|| found[CHECKPOINT_PAGE - 1].0);
        found.truncate(CHECKPOINT_PAGE);
        if seat.1.is_none() && self.interleaved(run_id, &streams, before, cut)? {
            return Ok(SeatRead::Kept);
        }
        let Some(since) = self.since(run_id, opened, seat.0, &streams, &found)? else {
            return Ok(SeatRead::Kept);
        };
        drop(streams);
        let row = self.started(run_id, &since, rows)?;
        self.paged(run_id, seat, &found, row, next)
    }

    /// The streams the effect's rows at `opened` keep of the participant
    /// `seat` that can reach its page below `before` ([`Gathered`]): none
    /// where they read stale, or refuse them.
    fn streams(
        &self,
        run_id: &str,
        opened: &Opened,
        (effect_id, member): Seat<'_>,
        before: Option<u64>,
    ) -> Result<Option<Vec<Stream>>, StoreError> {
        let member = member.map_or(Datum::Null, |member| Datum::Text(member.to_string()));
        let mut gathered = Gathered::below(before);
        let head = (opened.seq, opened.hash.as_str());
        let read = effect_rows_at(&self.conn, run_id, head, (effect_id, HELD), |data| {
            gathered.row(data, &member);
        })?;
        Ok(read.then(|| gathered.streams()).flatten())
    }

    /// The seq of the start each attempt with a checkpoint among `found`
    /// last names, in seq order: the largest `since` the effect's rows of
    /// that attempt keep, by the seq every row of the attempt takes at each
    /// start naming it, read again by the effect's key, so no attempt off
    /// the page is held. None where such an attempt's rows keep no start.
    fn since(
        &self,
        run_id: &str,
        opened: &Opened,
        effect_id: &str,
        streams: &[Stream],
        found: &[Found],
    ) -> Result<Option<Vec<u64>>, StoreError> {
        let mut on_page: Vec<usize> = found.iter().map(|(_, at)| *at).collect();
        on_page.sort_unstable();
        on_page.dedup();
        let mut since: Vec<Option<u64>> = vec![None; on_page.len()];
        let head = (opened.seq, opened.hash.as_str());
        let read = effect_rows_at(&self.conn, run_id, head, (effect_id, HELD), |data| {
            let (Datum::Text(attempt), Datum::Integer(at)) = (&data[ATTEMPT], &data[SINCE]) else {
                return;
            };
            let slot = on_page
                .iter()
                .position(|on| streams[*on].attempt == *attempt);
            if let Some(slot) = slot {
                since[slot] = since[slot].max(Some(*at as u64));
            }
        })?;
        let since: Option<Vec<u64>> = since.into_iter().collect();
        Ok(since.filter(|_| read).map(|mut since| {
            since.sort_unstable();
            since
        }))
    }

    /// What `rows` makes of the starts at `since`, handed to it one at a
    /// time, each read by its key and dropped once `rows` has made of it
    /// what makes a row: the first a start's read refuses, where one does.
    fn started<R>(
        &self,
        run_id: &str,
        since: &[u64],
        rows: &mut impl FnMut(&mut dyn Iterator<Item = EventEnvelope>) -> R,
    ) -> Result<R, StoreError> {
        let mut refused = None;
        let made = {
            let mut starts =
                since
                    .iter()
                    .map_while(|seq| match self.envelopes_at(run_id, &[*seq]) {
                        Ok(mut start) => start.pop(),
                        Err(error) => {
                            refused = Some(error);
                            None
                        }
                    });
            rows(&mut starts)
        };
        refused.map_or(Ok(made), Err)
    }

    /// The page of `found`, the participant `seat`'s, each made into its
    /// row by `row` and dropped, `next` its cursor.
    fn paged<T>(
        &self,
        run_id: &str,
        seat: Seat<'_>,
        found: &[Found],
        mut row: impl FnMut(&EventEnvelope) -> T,
        next: Option<u64>,
    ) -> Result<SeatRead<T>, StoreError> {
        let mut page = Vec::with_capacity(found.len());
        for (seq, _) in found {
            for event in self.envelopes_at(run_id, &[*seq])? {
                if !names(&event, seat) {
                    return Ok(SeatRead::Kept);
                }
                page.push(row(&event));
            }
        }
        Ok(SeatRead::Current(page, next))
    }

    /// The newest of `streams`' checkpoints below `before`, newest first,
    /// at most a page and one more of them, each with its stream's place,
    /// kept so as each stream is read, the streams newest first by the
    /// highest seq each is read from, until none left can reach the page
    /// and one more: none where a stream's are not its row's ([`reaches`]).
    /// `streams` keeps the streams read, in that order.
    fn found(
        &self,
        run_id: &str,
        member: Option<&str>,
        streams: &mut Vec<Stream>,
        before: Option<u64>,
    ) -> Result<Option<Vec<Found>>, StoreError> {
        streams.sort_unstable_by_key(|stream| Reverse(upper(stream, before)));
        let mut found: Vec<Found> = Vec::new();
        let mut read = 0;
        for (at, stream) in streams.iter().enumerate() {
            let upper = upper(stream, before);
            if found
                .get(CHECKPOINT_PAGE)
                .is_some_and(|(seq, _)| upper < *seq)
            {
                break;
            }
            let Some(seqs) = self.streamed(run_id, member, stream, upper)? else {
                return Ok(None);
            };
            found.extend(seqs.into_iter().map(|seq| (seq, at)));
            found.sort_unstable_by(|one, other| other.cmp(one));
            found.truncate(CHECKPOINT_PAGE + 1);
            read = at + 1;
        }
        streams.truncate(read);
        Ok(Some(found))
    }

    /// `stream`'s checkpoints at or below `upper`, newest first, at most
    /// one more than a page: none where they are not its row's. `upper` is
    /// at or above the stream's first checkpoint, as every stream
    /// [`Gathered`] keeps is (bound by
    /// `each_page_is_handed_over_the_start_each_of_its_attempts_last_names`,
    /// whose page below seq 6 reads the kept way where a stream above it is
    /// kept).
    fn streamed(
        &self,
        run_id: &str,
        member: Option<&str>,
        stream: &Stream,
        upper: u64,
    ) -> Result<Option<Vec<u64>>, StoreError> {
        let (first, last) = (place(stream.first), place(upper));
        let limit = CHECKPOINT_PAGE as i64 + 1;
        let bound = params![run_id, stream.attempt, member, first, last, limit];
        let found = seqs(&self.conn, &STREAM, bound)?;
        Ok(reaches(&found, stream, upper).then_some(found))
    }

    /// Whether, within the page's reach of one of `streams`, the seat's
    /// own streams the page read, from `cut` up, a member's checkpoint of
    /// its attempt stands; a stream the page did not read is below `cut`. Each
    /// member of the attempt is found by the stream index's key and its
    /// stream asked for one checkpoint between the bounds, so the work is
    /// the attempt's members, whatever their records hold, and no body is
    /// read.
    fn interleaved(
        &self,
        run_id: &str,
        streams: &[Stream],
        before: Option<u64>,
        cut: u64,
    ) -> Result<bool, StoreError> {
        for stream in streams {
            let (low, high) = (stream.first.max(cut), upper(stream, before));
            if low <= high && self.members_within(run_id, &stream.attempt, low, high)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a member's stream of `attempt` holds a checkpoint between
    /// `low` and `high`.
    fn members_within(
        &self,
        run_id: &str,
        attempt: &str,
        low: u64,
        high: u64,
    ) -> Result<bool, StoreError> {
        let mut member = self.member(LEAST, run_id, attempt, "")?;
        while let Some(named) = member {
            let bound = params![run_id, attempt, named, place(low), place(high), 1];
            if !seqs(&self.conn, &STREAM, bound)?.is_empty() {
                return Ok(true);
            }
            member = self.member(NEXT, run_id, attempt, &named)?;
        }
        Ok(false)
    }

    /// The member `sql` ([`LEAST`] or [`NEXT`]) finds of `attempt` from
    /// `from`.
    fn member(
        &self,
        sql: &str,
        run_id: &str,
        attempt: &str,
        from: &str,
    ) -> Result<Option<String>, StoreError> {
        let mut statement = self.conn.prepare_cached(sql)?;
        let member = statement.query_row(params![run_id, attempt, from], |row| row.get(0));
        Ok(member.optional()?)
    }
}

/// The highest seq `stream` is read from below `before`.
fn upper(stream: &Stream, before: Option<u64>) -> u64 {
    let below = before.map_or(stream.last, |before| before.saturating_sub(1));
    below.min(stream.last)
}

/// How many checkpoints below `before` `stream`'s row vouches for, at or
/// above its first: its first, and its last where it is another one below
/// `before`. A stream the page reads is held to them ([`reaches`]).
fn vouched(stream: &Stream, before: Option<u64>) -> usize {
    let last = stream.first < stream.last && upper(stream, before) == stream.last;
    1 + usize::from(last)
}

/// Are `found`, `stream`'s checkpoints read newest first from `upper`, its
/// row's? They begin at its last checkpoint where the read starts there,
/// and end at its first where a page does not cut them.
fn reaches(found: &[u64], stream: &Stream, upper: u64) -> bool {
    let begins = upper < stream.last || found.first() == Some(&stream.last);
    let ends = found.len() > CHECKPOINT_PAGE || found.last() == Some(&stream.first);
    begins && ends
}

/// Does `event` name the participant `seat` as its seat rows read a
/// checkpoint: its effect, and the member its checkpoint names?
fn names(event: &EventEnvelope, (effect_id, member): Seat<'_>) -> bool {
    let checkpoint = event.payload.get("checkpoint").unwrap_or(&Value::Null);
    text(&event.payload, "effect_id") == Some(effect_id) && text(checkpoint, "member") == member
}

/// `value`'s `key`, where it is text.
fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

#[cfg(test)]
mod tests;
