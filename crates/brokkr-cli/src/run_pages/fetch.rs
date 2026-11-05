//! One screen's page of a participant's checkpoints (#753; decision 0089
//! ruling 9): the newest rows to the depth the screen asks for the live
//! column, the oldest rows forward for the participant level, and what
//! landed since those held. Split from `run_pages.rs` by responsibility so
//! the console's readers stay under their file ceiling.

use std::collections::{BTreeMap, VecDeque};

use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::{Opened, SeatRead, Store, StoreError};
use brokkr_view::{page_rows, CheckpointRow};

use super::page::{Checkpoints, Page, PAGE, WINDOW};
use crate::tui::DISPLAY_RANK;

/// One screen's page of `run` at `at`: the rows it shows, the events landed
/// since those held, and the trail a checkpoint's boundary is read from.
pub(super) struct Fetch<'a> {
    pub(super) store: &'a Store,
    pub(super) run: &'a str,
    pub(super) at: &'a Opened,
    pub(super) rows: usize,
    /// The screen's offset in its own order: the participant level's first
    /// row, whose window the page holds.
    pub(super) offset: usize,
    /// How many rows the frame's viewport draws: the merge holds it whole,
    /// bounded by the frame rather than the journal or the scroll (M3).
    pub(super) visible: usize,
    pub(super) landed: &'a [EventEnvelope],
    pub(super) trail: &'a [EventEnvelope],
}

/// One page of checkpoints, oldest first, and its cursor for the next page.
pub(super) type Fetched = (Vec<(u64, CheckpointRow)>, Option<u64>);

/// Keep `item` among the window's last `window` rows, dropping the oldest
/// as the merge passes it (F4).
fn bury(buffer: &mut VecDeque<Item>, item: Item, window: usize) {
    if buffer.len() == window {
        buffer.pop_front();
    }
    buffer.push_back(item);
}

impl Fetch<'_> {
    /// How many rows the screen asks for, at least one page: a pane of no
    /// measured height still shows its page.
    pub(super) fn limit(&self) -> usize {
        self.rows.max(PAGE)
    }

    /// The participant level's window: the oldest page first, forward to
    /// the depth the scroll asks, and back by older pages when the
    /// operator scrolls up. The pane paints oldest first, as it did before
    /// its pages, so the window starts at the scroll's row and holds
    /// [`WINDOW`] rows from there (R2, F2, F3).
    pub(super) fn participant(
        &self,
        seat: (&str, Option<&str>),
        page: Option<Page>,
    ) -> Result<Option<(Page, Option<usize>)>, StoreError> {
        let limit = self.limit();
        if self.offset == usize::MAX {
            return self.participant_end(seat);
        }
        if limit == usize::MAX {
            // The whole participant is asked for: start at its oldest row
            // and read forward, whatever page was held before.
            return self.participant_whole(seat);
        }
        let Some(mut page) = self.participant_start(seat, page)? else {
            return Ok(None);
        };
        // The carried page may already reach the stream's end. The
        // backward read drops the rows above the window, so the end's
        // rank is read here, before that cut (M1, R3, F1).
        let known = page.next.is_none().then(|| page.seen());
        let Some(()) = self.extend_back(seat, &mut page)? else {
            return Ok(None);
        };
        drop_passed(&mut page, self.offset);
        // Hold one window from the scroll's row, so a step down or up
        // stays inside the page rather than reading a page a row.
        let end = self.offset.saturating_add(WINDOW).max(limit);
        let Some(()) = self.extend_after(seat, &mut page, Some(end))? else {
            return Ok(None);
        };
        // The read reached the stream's end before the window was cut, or
        // the page it started from already did: that is the stream's whole
        // rank, whatever the narrowed display page asked for (R3, F1).
        let total = known.or_else(|| page.next.is_none().then(|| page.seen()));
        if page.seen() > end {
            let keep = end.saturating_sub(page.start);
            page.rows.truncate(keep);
            page.next = page.rows.last().map(|(seq, _)| *seq);
        }
        page.older = match page.start {
            0 => None,
            _ => page.rows.first().map(|(seq, _)| *seq),
        };
        Ok(Some((page, total)))
    }

    /// The page the read starts from: a carried page with the arrivals the
    /// head it still holds lacks, or the page at the scroll's row on a
    /// fresh read, found through the store's ordinals rather than by
    /// reading every page between (decision 0089 ruling 9). A carried empty
    /// page stays empty rather than counting its rows twice (M1, M2).
    fn participant_start(
        &self,
        seat: (&str, Option<&str>),
        page: Option<Page>,
    ) -> Result<Option<Page>, StoreError> {
        // A carried page is not an absent one: an empty page the screen
        // already held is its own stream's state, exhausted or above this
        // member's rows, and reading its newest suffix again would count
        // its rows twice (M1, M2).
        let carried = page.is_some();
        let mut page = page.unwrap_or_default();
        if page.rows.is_empty() && !carried {
            let Some(made) = self.fresh_page(seat)? else {
                return Ok(None);
            };
            page = made;
        }
        // Only a page carried from the old head can lack the arrivals; a
        // fresh read already holds them. A page with a newer cursor stops
        // short of this head and cannot take them.
        if carried && !self.landed.is_empty() && page.next.is_none() {
            let Some(arrived) = landed_rows(seat, self.trail, self.landed) else {
                return Ok(None);
            };
            page.rows.extend(arrived);
        }
        Ok(Some(page))
    }

    /// The page at the screen's offset, found through the store's ordinals
    /// rather than by reading every page between it and the stream's
    /// oldest (decision 0089 ruling 9): the oldest page at the top, else
    /// the page after the seq the store names for the row before it. A
    /// destination past the stream's discovered end keeps its last row
    /// (F3).
    fn fresh_page(&self, seat: (&str, Option<&str>)) -> Result<Option<Page>, StoreError> {
        if self.offset == 0 {
            let Some((rows, newer)) = self.read_after(seat, None)? else {
                return Ok(None);
            };
            return Ok(Some(Page::first(rows, newer)));
        }
        let Some(total) = self.total_of(seat)? else {
            return Ok(None);
        };
        let start = self.offset.min(total.saturating_sub(1));
        let Some((rows, next)) = self.page_at(seat, start)? else {
            return Ok(None);
        };
        let mut page = Page::first(rows, next);
        page.start = start;
        if self.offset >= total && total > 0 {
            page.rows.truncate(1);
            page.next = None;
        }
        Ok(Some(page))
    }

    /// How many checkpoints the participant holds, as a screen rank: none
    /// where the store keeps its read.
    fn total_of(&self, seat: (&str, Option<&str>)) -> Result<Option<usize>, StoreError> {
        let Some(total) = self.store.seat_count(self.run, self.at, seat)? else {
            return Ok(None);
        };
        Ok(Some(usize::try_from(total).unwrap_or(usize::MAX)))
    }

    /// The page whose first row is rank `start`, oldest first, and the row
    /// after it: the stream's oldest page at 0, else the page after the seq
    /// the store's ordinals name for the row before it. No earlier page is
    /// read (decision 0089 ruling 9).
    fn page_at(
        &self,
        seat: (&str, Option<&str>),
        start: usize,
    ) -> Result<Option<Fetched>, StoreError> {
        match start {
            0 => self.read_after(seat, None),
            start => {
                let Some(previous) =
                    self.store
                        .seat_seq_at(self.run, self.at, seat, (start - 1) as u64)?
                else {
                    return Ok(None);
                };
                self.read_after(seat, Some(previous))
            }
        }
    }

    /// The whole participant, oldest first: a fresh read from its oldest
    /// row forward, whatever page was held before.
    fn participant_whole(
        &self,
        seat: (&str, Option<&str>),
    ) -> Result<Option<(Page, Option<usize>)>, StoreError> {
        let Some((rows, newer)) = self.read_after(seat, None)? else {
            return Ok(None);
        };
        let mut page = Page::first(rows, newer);
        let Some(()) = self.extend_after(seat, &mut page, None)? else {
            return Ok(None);
        };
        page.start = 0;
        page.older = None;
        Ok(Some((page, None)))
    }

    /// Extend the page back to the scroll's row, one older page at a time,
    /// stopping at the stream's oldest row or a page the store keeps. The
    /// rows past the window the caller will hold are dropped as the older
    /// pages are prepended, so a jump from the stream's end to its head
    /// holds one window rather than every page between (M1, ruling 9): a
    /// page wholly above that window carries only its rank until the read
    /// reaches it.
    /// The bound is driven by
    /// `a_backward_jump_holds_the_window_not_the_stream`.
    pub(super) fn extend_back(
        &self,
        seat: (&str, Option<&str>),
        page: &mut Page,
    ) -> Result<Option<()>, StoreError> {
        let end = self.offset.saturating_add(WINDOW).max(self.limit());
        // Whether rows above the window have already been dropped: once
        // they have, the forward cursor is the last kept row even when the
        // pages below it fill the window exactly.
        let mut cut = false;
        while page.start > self.offset {
            let Some(before) = page.older else {
                break;
            };
            let Some((rows, older)) = self.read_before(seat, Some(before))? else {
                return Ok(None);
            };
            if rows.is_empty() {
                page.older = None;
                break;
            }
            page.start = page.start.saturating_sub(rows.len());
            let mut held = rows;
            held.append(&mut page.rows);
            page.rows = held;
            page.older = older;
            if page.start >= end {
                // Every retained row is still past the window; keep the
                // rank and the older cursor, not the rows above it.
                cut |= !page.rows.is_empty();
                page.rows.clear();
            } else if cut || page.seen() > end {
                // The window's end is now inside the retained rows: cut
                // the suffix and resume the forward read from its last
                // kept row.
                page.rows.truncate(end - page.start);
                page.next = page.rows.last().map(|(seq, _)| *seq);
            }
        }
        Ok(Some(()))
    }

    /// The participant's last window, asked for by the absolute-end key
    /// (`G`): the page holding today's effective display rank, oldest
    /// first, extended through the frame's drawable suffix, with the whole
    /// stream's rank. The rank and the page are read through the store's
    /// ordinals, so the read decodes one page however long the participant
    /// ran, and the rank is narrowed before the page is chosen, never the
    /// page start subtracted from a narrowed offset (M3, M4, R3).
    fn participant_end(
        &self,
        seat: (&str, Option<&str>),
    ) -> Result<Option<(Page, Option<usize>)>, StoreError> {
        let Some(total) = self.total_of(seat)? else {
            return Ok(None);
        };
        let shown = total.saturating_sub(1).min(DISPLAY_RANK);
        // The page the pane paints is the one whose rows start at the page
        // boundary at or below the narrowed rank, as the forward pages did.
        let start = shown - shown % PAGE;
        let Some((mut rows, mut next)) = self.page_at(seat, start)? else {
            return Ok(None);
        };
        // The pane paints from the narrowed rank at the column's width, so
        // the page must hold the rows below it that the frame draws. The
        // extension is the viewport's, never the journal's: a stream of a
        // million rows still holds one page and the rows it paints (R3).
        let wanted = shown.saturating_sub(start).saturating_add(self.limit());
        while rows.len() < wanted {
            let Some(after) = next else {
                break;
            };
            let Some((more, newer)) = self.read_after(seat, Some(after))? else {
                return Ok(None);
            };
            rows.extend(more);
            next = newer;
        }
        let older = rows.first().map(|(seq, _)| *seq).filter(|_| start > 0);
        let page = Page {
            older,
            rows,
            next,
            start,
        };
        Ok(Some((page, Some(total))))
    }

    /// The detail pane's one window across every member's stream, cut at
    /// the depth `limit` asks and at most [`WINDOW`] rows wide: rows taken
    /// from the members in seq order, the rows above the window counted and
    /// dropped, so intermediate storage stays the window's however deep the
    /// scroll (R1, F4; decision 0089 ruling 9). `prefix` is how many rows
    /// above `held` were already cut, so the window's absolute rank is
    /// preserved through a carried read (F1).
    pub(super) fn window(
        &self,
        keys: &[String],
        seats: &BTreeMap<String, (String, Option<String>)>,
        held: BTreeMap<String, Page>,
        prefix: usize,
        refill: bool,
    ) -> Result<Option<Checkpoints>, StoreError> {
        let Some(mut streams) = self.streams(keys, seats, held, refill)? else {
            return Ok(None);
        };
        let to_end = self.offset == usize::MAX;
        let Some((kept, total)) = self.merge(&mut streams, prefix, to_end)? else {
            return Ok(None);
        };
        let start = match (self.limit() == usize::MAX, to_end) {
            (true, false) => prefix,
            _ => total.saturating_sub(kept.len()),
        };
        let mut window = cut(keys, &streams, &kept, start);
        if to_end {
            window.total = Some(total);
        }
        Ok(Some(window))
    }

    /// One stream per member `keys` names, from its held window or its
    /// newest page, with the events `landed` appended where the window
    /// still holds the head it was read at.
    fn streams(
        &self,
        keys: &[String],
        seats: &BTreeMap<String, (String, Option<String>)>,
        held: BTreeMap<String, Page>,
        refill: bool,
    ) -> Result<Option<Vec<Stream>>, StoreError> {
        let mut streams: Vec<Stream> = Vec::with_capacity(keys.len());
        let mut held = held;
        for key in keys {
            let Some((effect, member)) = seats.get(key) else {
                return Ok(None);
            };
            // A carried page keeps its empty rows, cursor and exhaustion:
            // treating it as absent reads the member's newest suffix again
            // and counts the rows the window already dropped (M1).
            let carried = !refill && held.contains_key(key);
            let mut page = match carried {
                true => held.remove(key).unwrap_or_default(),
                false => Page::default(),
            };
            if page.rows.is_empty() && !carried {
                let Some(made) = self.newest_suffix((effect, member.as_deref()))? else {
                    return Ok(None);
                };
                page = made;
            }
            // A carried member page takes the arrivals, empty or not: its
            // cursor is an older page, not a newer prefix, so appending
            // them never skips a held row (M5).
            if carried && !self.landed.is_empty() {
                let Some(arrived) =
                    landed_rows((effect, member.as_deref()), self.trail, self.landed)
                else {
                    return Ok(None);
                };
                page.rows.extend(arrived);
            }
            let at = page.rows.len();
            streams.push(Stream {
                seat: (effect.clone(), member.clone()),
                rows: page.rows,
                at,
                next: page.next,
                start: page.start,
                pulled: 0,
            });
        }
        Ok(Some(streams))
    }

    /// Merge the streams' rows in seq order until the depth `limit` asks,
    /// keeping the last [`WINDOW`] or the viewport, whichever is larger, in
    /// `buffer` and counting the rest, so the rows above the window are
    /// dropped as they arrive (F4) and a tall pane still holds every row it
    /// draws (M3). Returns the window oldest-last with the number of rows
    /// from the head to its bottom.
    fn merge(
        &self,
        streams: &mut [Stream],
        prefix: usize,
        to_end: bool,
    ) -> Result<Option<(Vec<Item>, usize)>, StoreError> {
        let complete = self.limit() == usize::MAX && !to_end;
        let limit = match to_end {
            true => usize::MAX,
            false => self.limit(),
        };
        let window = match to_end {
            true => WINDOW,
            false => WINDOW.max(self.visible),
        };
        let mut total = prefix;
        let mut buffer: VecDeque<Item> = VecDeque::new();
        let mut all: Vec<Item> = Vec::new();
        loop {
            // A page the store says is kept is not the stream's end: the
            // run must be read whole, as ruling 8 says (M6).
            let Some(more) = self.top_up_ready(streams)? else {
                return Ok(None);
            };
            if !more {
                break;
            }
            // A ready top-up names a stream: it is true only where one has
            // an unread row, which `next_stream` finds. The refill control
            // is `a_scrolled_detail_window_refills_on_a_landing`.
            let pick = Self::next_stream(streams).expect("a ready merge names a stream");
            let stream = &mut streams[pick];
            stream.at -= 1;
            stream.pulled += 1;
            let (seq, row) = stream.rows[stream.at].clone();
            total += 1;
            let item = (pick, seq, row);
            if complete {
                all.push(item);
                continue;
            }
            bury(&mut buffer, item, window);
            if total >= limit {
                break;
            }
        }
        let kept = match complete {
            true => all,
            false => buffer.into_iter().collect(),
        };
        Ok(Some((kept, total)))
    }

    /// The stream whose newest unread row is next in seq order: the merge
    /// compares only unread rows, so a spent stream waits for its page.
    fn next_stream(streams: &[Stream]) -> Option<usize> {
        streams
            .iter()
            .enumerate()
            .filter(|(_, stream)| stream.at > 0)
            .max_by_key(|(_, stream)| stream.rows[stream.at - 1].0)
            .map(|(at, _)| at)
    }

    /// Whether the merge still has a row to compare: `None` where the
    /// store says a page is kept and the run must be read whole (M6).
    fn top_up_ready(&self, streams: &mut [Stream]) -> Result<Option<bool>, StoreError> {
        Ok(match self.top_up(streams)? {
            TopUp::More => Some(true),
            TopUp::Spent => Some(false),
            TopUp::Kept => None,
        })
    }

    /// Load the next older page of every spent stream, so the merge always
    /// compares their newest unread rows. [`TopUp::Kept`] where the store
    /// will not read a page, which the run reads whole.
    fn top_up(&self, streams: &mut [Stream]) -> Result<TopUp, StoreError> {
        let mut ready = false;
        for stream in streams {
            if stream.at == 0 && stream.next.is_some() {
                let (effect, member) = stream.seat.clone();
                let Some((rows, next)) =
                    self.read_before((&effect, member.as_deref()), stream.next)?
                else {
                    return Ok(TopUp::Kept);
                };
                stream.rows = rows;
                stream.at = stream.rows.len();
                stream.next = next;
            }
            ready |= stream.at > 0;
        }
        Ok(match ready {
            true => TopUp::More,
            false => TopUp::Spent,
        })
    }
    /// The newest page of `seat`'s checkpoints, oldest first, without a
    /// window: the detail pane merges every member's rows before it cuts
    /// the one window.
    fn newest_suffix(&self, seat: (&str, Option<&str>)) -> Result<Option<Page>, StoreError> {
        let Some((rows, next)) = self.read_before(seat, None)? else {
            return Ok(None);
        };
        Ok(Some(Page {
            rows,
            next,
            older: None,
            start: 0,
        }))
    }

    /// One page older than `cursor`, oldest first, and the cursor below it.
    fn read_before(
        &self,
        seat: (&str, Option<&str>),
        cursor: Option<u64>,
    ) -> Result<Option<Fetched>, StoreError> {
        self.read_page(seat, cursor, false)
    }

    /// One page newer than `cursor`, oldest first, and the cursor above it.
    fn read_after(
        &self,
        seat: (&str, Option<&str>),
        cursor: Option<u64>,
    ) -> Result<Option<Fetched>, StoreError> {
        self.read_page(seat, cursor, true)
    }

    /// One page of `seat`'s checkpoints from `cursor`, in `forward`'s
    /// direction, oldest first, and its cursor in that direction.
    fn read_page(
        &self,
        seat: (&str, Option<&str>),
        cursor: Option<u64>,
        forward: bool,
    ) -> Result<Option<Fetched>, StoreError> {
        let mut made = |starts: &mut dyn Iterator<Item = EventEnvelope>| {
            let row = page_rows(seat.1, starts);
            move |event: &EventEnvelope| (event.seq, row(event))
        };
        let read = match forward {
            true => self
                .store
                .seat_from(self.run, self.at, seat, cursor, &mut made)?,
            false => self
                .store
                .seat_read(self.run, self.at, seat, cursor, &mut made)?,
        };
        let Some((mut rows, next)) = rows_of(read) else {
            return Ok(None);
        };
        if !forward {
            rows.reverse();
        }
        Ok(Some((rows, next)))
    }

    /// `page` extended by newer pages until `end` (none for the stream's
    /// whole) or the stream's end, dropping the rows the window has passed
    /// as they arrive (F4, ruling 9).
    fn extend_after(
        &self,
        seat: (&str, Option<&str>),
        page: &mut Page,
        end: Option<usize>,
    ) -> Result<Option<()>, StoreError> {
        while end.is_none_or(|end| page.seen() < end) {
            let Some(after) = page.next else {
                break;
            };
            let Some((rows, newer)) = self.read_after(seat, Some(after))? else {
                return Ok(None);
            };
            // A page with no rows is the stream's end: the store hands no
            // newer cursor with it, so the next turn of the loop stops.
            page.rows.extend(rows);
            page.next = newer;
            if end.is_some() {
                drop_passed(page, self.offset);
            }
        }
        Ok(Some(()))
    }
}

/// Drop the rows of `page` the participant window has passed, so a deep
/// scroll's intermediate page is the window, not the stream it read
/// through (F4, ruling 9). A destination past the stream's discovered end
/// keeps the end's last row: the page step resolves to it rather than
/// painting an empty pane (F3).
fn drop_passed(page: &mut Page, offset: usize) {
    let reached = offset.saturating_sub(page.start);
    let past_end = page.next.is_none() && reached >= page.rows.len();
    let drop = match past_end {
        true => page.rows.len().saturating_sub(1),
        false => reached.min(page.rows.len()),
    };
    if drop > 0 {
        page.rows.drain(..drop);
        page.start += drop;
        page.older = page.rows.first().map(|(seq, _)| *seq);
    }
}

/// What the merge's next top-up found: a stream still has rows, every
/// stream is spent, or the store keeps a page and the run is read whole.
enum TopUp {
    More,
    Spent,
    Kept,
}

/// A row pulled by the merge: its member's place, its seq and the row.
type Item = (usize, u64, CheckpointRow);

/// `kept`, the merge's window newest first, as one page per member, each
/// oldest first, with the member's own rows above the window counted into
/// its rank (F1).
fn cut(keys: &[String], streams: &[Stream], kept: &[Item], start: usize) -> Checkpoints {
    let mut checkpoints = Checkpoints::new();
    checkpoints.start = start;
    for (at, key) in keys.iter().enumerate() {
        let rows: Vec<(u64, CheckpointRow)> = kept
            .iter()
            .rev()
            .filter(|(member, _, _)| *member == at)
            .map(|(_, seq, row)| (*seq, row.clone()))
            .collect();
        let stream = &streams[at];
        let older = stream.at > 0 || stream.next.is_some();
        let next = match older {
            true => rows
                .first()
                .map(|(seq, _)| *seq)
                .or_else(|| kept.iter().map(|(_, seq, _)| *seq).min()),
            false => None,
        };
        checkpoints.pages.insert(
            key.clone(),
            Page {
                start: stream.start + stream.pulled - rows.len(),
                rows,
                next,
                older: None,
            },
        );
    }
    checkpoints
}

/// One member's stream, read toward older pages as the merge needs them.
struct Stream {
    seat: (String, Option<String>),
    rows: Vec<(u64, CheckpointRow)>,
    /// The next row to take, from the newest end; zero once the page is
    /// spent.
    at: usize,
    next: Option<u64>,
    /// The member's own rows above the page it started with.
    start: usize,
    /// How many of its rows the merge took.
    pulled: usize,
}

/// `read`'s rows and cursor, oldest first, or none where it is kept.
fn rows_of(read: SeatRead<(u64, Option<CheckpointRow>)>) -> Option<Fetched> {
    let SeatRead::Current(page, next) = read else {
        return None;
    };
    let rows: Option<Vec<(u64, CheckpointRow)>> = page
        .into_iter()
        .map(|(seq, row)| row.map(|row| (seq, row)))
        .collect();
    rows.map(|rows| (rows, next))
}

/// The checkpoints among `landed` that `seat` recorded, oldest first, each
/// made as the run whole makes it by the starts `trail` holds.
fn landed_rows(
    seat: (&str, Option<&str>),
    trail: &[EventEnvelope],
    landed: &[EventEnvelope],
) -> Option<Vec<(u64, CheckpointRow)>> {
    let starts = trail.iter().filter(|event| {
        event.event_type == EventType::EffectStarted
            && event.payload.get("effect_id").and_then(|id| id.as_str()) == Some(seat.0)
    });
    let row = page_rows(seat.1, &mut starts.cloned());
    let mut rows = Vec::new();
    for event in landed {
        if event.event_type != EventType::EffectCheckpointed || !names(event, seat) {
            continue;
        }
        rows.push((event.seq, row(event)?));
    }
    Some(rows)
}

/// How many checkpoints among `landed` `seat` recorded: the rows a carried
/// read adds to the stream's rank it already knows (R3, F1).
pub(super) fn landed_count(seat: (&str, Option<&str>), landed: &[EventEnvelope]) -> usize {
    landed
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed && names(event, seat))
        .count()
}

/// Does `event` name `seat`: its effect, and the member its checkpoint names?
fn names(event: &EventEnvelope, seat: (&str, Option<&str>)) -> bool {
    let effect = event.payload.get("effect_id").and_then(|id| id.as_str());
    let checkpoint = event.payload.get("checkpoint");
    let member = checkpoint
        .and_then(|checkpoint| checkpoint.get("member"))
        .and_then(|member| member.as_str());
    effect == Some(seat.0) && member == seat.1
}
