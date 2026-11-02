//! The run view of one run as its pages read it (#753; decision 0089 slice
//! 3): the one home of `inspect`, `seats`, `watch` and `/api/view/<run>`'s
//! read, of the seat readers' participants ([`participants`]), and of the
//! console's run screens ([`console`]). The run is opened first, its chain
//! read whole and verified, and refused there in the store's words as
//! `Store::load` refuses it (ruling 5); then it is read at the head its
//! opening verified and composed from its trail and seat rows (ruling 7).
//! A read the store says is kept, a run whose summary row records the
//! fold's refusal, and a view the rows do not compose read the run whole,
//! as today (ruling 8).
//!
//! What each reader shows decides what it reads ([`Shows`]): a readout that
//! prints every row and checkpoint keeps every event its opening verifies,
//! bounded by the run, and composes the view once; a terminal frame reads
//! one read's trail, the rows it shows, and no journal page or checkpoint,
//! and so does a seat reader. The console's run level reads a frame and no
//! checkpoint; its fleet detail pane reads that frame and the newest page
//! of each participant at work; its participant level reads that frame and
//! the participant's oldest page, forward as the operator scrolls
//! ([`Console`]). A frame whose head stands
//! reads nothing, and one whose head moved reads the events and checkpoints
//! landed since, never the run whole.

use std::collections::BTreeMap;

use crate::run_pages::fetch::{landed_count, Fetch};
use crate::run_pages::page::{held_start, Checkpoints, WINDOW};
use brokkr_core::fold::{fold, FoldError, RunState};
use brokkr_core::seat::row::SeatRow;
use brokkr_core::{EventEnvelope, EventType};
use brokkr_store::{Opened, RunRead, Store, StoreError};
use brokkr_view::{composed_view, composed_whole, seat_keys, Participant, Parts, RunView};

/// What a reader shows of a run, and so what it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shows {
    /// Every journal row and every participant's checkpoints: `inspect
    /// --json`, `inspect --capability-call`, `seats --json` and
    /// `/api/view/<run>`, which print the view whole.
    Everything,
    /// A terminal frame: `inspect`'s, `seats`' and `watch`'s, which paint
    /// the trail's journal rows (`in_trail`, every one an event of the
    /// trail) and no participant's checkpoints. Its journal holds the
    /// trail's rows, none of them naming its cause, and its participants
    /// no checkpoint. The seat readers' read ([`participants`]).
    Frame,
}

impl Shows {
    /// What a readout shows: everything where it prints the view whole, a
    /// terminal frame otherwise.
    pub(crate) fn of(whole: bool) -> Self {
        match whole {
            true => Shows::Everything,
            false => Shows::Frame,
        }
    }
}

/// A run's view as its reader shows it, and the run's fold, or the fold's
/// refusal, which `inspect` and `seats` refuse the run with.
pub(crate) struct Viewed {
    pub(crate) view: RunView,
    pub(crate) folded: Result<RunState, FoldError>,
}

/// `run`'s view in `store` as a reader that `shows` it reads it: opened,
/// keeping what the opening verifies where it shows everything, then read
/// at the head its opening verified, or read whole where that read is
/// kept.
pub(crate) fn viewed(store: &Store, run: &str, shows: Shows) -> Result<Viewed, StoreError> {
    let paged = match shows {
        Shows::Everything => {
            let (opened, journal) = store.open_kept(run)?;
            composed(store, run, &opened, Some(&journal))?
        }
        Shows::Frame => composed(store, run, &store.open_run(run)?, None)?,
    };
    match paged {
        Some(viewed) => Ok(viewed),
        None => whole(store, run),
    }
}

/// `run`'s participants as a seat reader reads them: `brokkr transcript`,
/// `/api/presentation/<run>/<key>`, `/api/transcript/<run>/<key>` and
/// `/sse/transcript/<run>/<key>`, which each show one participant's row,
/// its key, label, transcript reference, provenance, legacy id and whether
/// it works, and none of its checkpoints. Each is its row as the frame's
/// read composes it (ruling 7), with no checkpoint, which none of them
/// prints; read whole where that read is kept (ruling 8). The store's
/// refusal of the run, or the fold's where it opens and does not fold, as
/// each refused it reading the run whole.
pub(crate) fn participants(
    store: &Store,
    run: &str,
) -> Result<Result<Vec<Participant>, FoldError>, StoreError> {
    let Viewed { view, folded } = viewed(store, run, Shows::Frame)?;
    Ok(folded.map(|_| view.participants))
}

/// `run` read whole and folded, as every reader read it before its pages.
fn whole(store: &Store, run: &str) -> Result<Viewed, StoreError> {
    let events = store.load(run)?;
    let folded = fold(&events);
    let view = brokkr_view::run_view(&events, folded.as_ref().ok());
    Ok(Viewed { view, folded })
}

/// What one read at an opening's head holds, asked past the head so it
/// holds no journal row beside the trail, a checkpoint of the run's first
/// seqs least of all (ruling 9): the trail, the seat rows, the seq each
/// cause names, the fold at the head and the head's stamp.
struct Frame {
    head: u64,
    trail: Vec<EventEnvelope>,
    seats: Vec<SeatRow>,
    causes: BTreeMap<String, u64>,
    state: Box<RunState>,
    last_at: String,
}

impl Frame {
    /// `run` read at `opened`: `None` where the read is kept or its summary
    /// row records the fold's refusal.
    fn read(store: &Store, run: &str, opened: &Opened) -> Result<Option<Frame>, StoreError> {
        let RunRead::Current {
            trail,
            seats,
            causes,
            state: Some(state),
            last_at,
            ..
        } = store.run_read(run, opened, opened.seq)?
        else {
            return Ok(None);
        };
        let head = opened.seq;
        Ok(Some(Frame {
            head,
            trail,
            seats,
            causes,
            state,
            last_at,
        }))
    }

    /// The frame's view, its journal the trail's rows, none naming its
    /// cause: the page asked past the head is empty, and so are the causes
    /// read with it. `None` where the rows do not compose.
    fn view(&self) -> Option<RunView> {
        let parts = Parts {
            trail: &self.trail,
            seats: &self.seats,
            page: &self.trail,
            causes: &self.causes,
            head_seq: self.head,
            last_at: &self.last_at,
        };
        composed_view(&parts, Some(&self.state))
    }
}

/// `run`'s view composed once from one read at `opened` ([`Frame`]). With
/// `journal`, every event the opening kept, it is the view whole, each
/// participant's checkpoints made from it; without, the frame's. `None`
/// where the read is kept, its summary row records the fold's refusal or
/// the rows do not compose.
fn composed(
    store: &Store,
    run: &str,
    opened: &Opened,
    journal: Option<&[EventEnvelope]>,
) -> Result<Option<Viewed>, StoreError> {
    let Some(frame) = Frame::read(store, run, opened)? else {
        return Ok(None);
    };
    let view = match journal {
        Some(journal) => {
            let (trail, seats, state) = (&frame.trail, &frame.seats, Some(&*frame.state));
            composed_whole(trail, seats, journal, &frame.last_at, state)
        }
        None => frame.view(),
    };
    let folded = Ok(*frame.state);
    Ok(view.map(|view| Viewed { view, folded }))
}

/// Which of the console's run screens a frame paints, and so what it reads
/// of the run (ruling 7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Console {
    /// The run level: the graph, the seats and the trail, which its frame
    /// holds. It reads no checkpoint of any participant at work.
    Run,
    /// The fleet's detail pane: the frame and a bounded page of each
    /// participant at work, the live column ordering it by seq. More only
    /// as the operator scrolls, so the detail never holds the stream.
    Detail,
    /// The participant level keyed so: the frame and a bounded window of
    /// its checkpoints from its oldest page forward, newer and older pages
    /// only as the operator scrolls.
    Participant(String),
}

/// The run the console shows: the head it was read at, what was asked, and
/// what was read, from which each frame derives its view. A frame whose
/// head stands reads nothing more ([`console`]).
pub(crate) struct Shown {
    head: Opened,
    asked: Console,
    read: Held,
}

/// What the console holds of its run between frames.
enum Held {
    /// Through the pages: one read's frame, and a bounded window of each
    /// participant the screen shows, by its key, oldest first by seq.
    Paged(Frame, Checkpoints),
    /// Read whole, as before the pages (ruling 8): the run's events and
    /// their fold, none where it refuses.
    Whole(Vec<EventEnvelope>, Option<Box<RunState>>),
}

impl Shown {
    /// The seq of the head the run was read at.
    pub(crate) fn head(&self) -> u64 {
        self.head.seq
    }

    /// The run view the console paints: composed from the frame it holds,
    /// each participant shown given its page of checkpoints, or derived
    /// from the run read whole.
    pub(crate) fn view(&self) -> RunView {
        match &self.read {
            Held::Paged(frame, checkpoints) => {
                let mut view = frame.view().expect("a frame composed when it was read");
                for participant in &mut view.participants {
                    let page = checkpoints.get(&participant.key);
                    let shown = page.into_iter().flat_map(|page| &page.rows);
                    participant.checkpoints = shown.map(|(_, row)| row.clone()).collect();
                }
                view
            }
            Held::Whole(events, state) => brokkr_view::run_view(events, state.as_deref()),
        }
    }

    /// How many rows of the screen's order stand before the page it holds,
    /// so the shell scrolls from the window it has rather than the top of
    /// the stream it dropped (decision 0089 ruling 9).
    pub(crate) fn start(&self, _asked: &Console) -> usize {
        match &self.read {
            Held::Paged(_, held) => held_start(held),
            Held::Whole(..) => 0,
        }
    }

    /// The rank of the screen's stream's end, where the last read asked
    /// for it: the shell resolves the end key's request into the offset
    /// the pane scrolls at (M3).
    pub(crate) fn end(&self, asked: &Console) -> Option<usize> {
        match &self.read {
            Held::Paged(_, held) => held.total,
            Held::Whole(_, state) => match asked {
                Console::Run => None,
                Console::Detail => {
                    // The column's stream is the hired seat's, as the paged
                    // read counts it: a retired seat's pinned member is not
                    // in it (M1).
                    let view = self.view();
                    let keys = state
                        .as_deref()
                        .map_or_else(Vec::new, |state| live_keys(&view, state));
                    Some(
                        view.participants
                            .iter()
                            .filter(|part| keys.contains(&part.key))
                            .map(|part| part.checkpoints.len())
                            .sum(),
                    )
                }
                Console::Participant(key) => self
                    .view()
                    .participants
                    .iter()
                    .find(|part| part.key == *key)
                    .map(|part| part.checkpoints.len()),
            },
        }
    }

    /// Whether what was read paints `asked`: the screen it was read for, a
    /// frame over a run read whole, which holds every participant's
    /// checkpoints, or the run level over any page, which shows no
    /// checkpoint at all (ruling 7).
    fn paints(&self, asked: &Console) -> bool {
        match &self.read {
            Held::Whole(..) => true,
            Held::Paged(..) => match asked {
                Console::Run => true,
                Console::Detail => self.asked == Console::Detail,
                Console::Participant(key) => {
                    matches!(&self.asked, Console::Participant(held) if held == key)
                }
            },
        }
    }

    /// Whether the page `asked` shows has a next one: the live column's
    /// or the participant pane's, so a scroll past the held rows asks the
    /// page after them (decision 0089 slice 3).
    pub(crate) fn more(&self, asked: &Console) -> bool {
        let Held::Paged(_, held) = &self.read else {
            return false;
        };
        match asked {
            Console::Run => false,
            Console::Detail => held.pages.values().any(|page| page.next.is_some()),
            Console::Participant(key) => held.get(key).is_some_and(|page| page.next.is_some()),
        }
    }

    /// Whether what was read holds enough of every participant `asked`
    /// shows: a whole read always, the run level nothing, and a page from
    /// the scroll's requested first row through the rows the viewport
    /// draws, or to the stream's end. The requested start is `offset`, not
    /// a fixed window above `rows`: a tall pane asks for rows `offset` and
    /// up, so a held window that starts below them omits a row the screen
    /// paints and must be read again (M3, ruling 9).
    fn covers(&self, asked: &Console, rows: usize, offset: usize) -> bool {
        let Held::Paged(frame, held) = &self.read else {
            return true;
        };
        match asked {
            Console::Run => true,
            Console::Detail => detail_covers(frame, held, rows, offset),
            Console::Participant(key) => participant_covers(frame, held, key, rows, offset),
        }
    }
}

/// Whether a held detail page reaches `rows` from the screen's `offset`:
/// the frame must compose, and the page's rank must reach the rows the
/// pane draws and start at or above its first (M3).
fn detail_covers(frame: &Frame, held: &Checkpoints, rows: usize, offset: usize) -> bool {
    let Some(_view) = frame.view() else {
        return false;
    };
    let rows_held: usize = held.pages.values().map(|page| page.rows.len()).sum();
    let start = held_start(held);
    let more = held.pages.values().any(|page| page.next.is_some());
    let end = start + rows_held;
    let covers = match rows {
        usize::MAX => !more,
        rows => end >= rows || !more,
    };
    covers && start <= offset
}

/// Whether a held participant page reaches `rows` from the screen's
/// `offset`, and the frame still paints that participant: a page for a
/// participant the frame no longer shows covers nothing (ruling 5).
fn participant_covers(
    frame: &Frame,
    held: &Checkpoints,
    key: &str,
    rows: usize,
    offset: usize,
) -> bool {
    let Some(view) = frame.view() else {
        return false;
    };
    let shown = view.participants.iter().any(|part| part.key == key);
    if !shown {
        return false;
    }
    held.get(key).is_some_and(|page| {
        let end = match rows {
            usize::MAX => page.next.is_none(),
            rows => page.seen() >= rows || page.next.is_none(),
        };
        end && page.start <= offset
    })
}

/// One run in one store, the pair every read of a frame names.
#[derive(Clone, Copy)]
struct Reader<'a> {
    store: &'a Store,
    run: &'a str,
}

/// One console screen's ask: how many rows the read is asked for, the
/// screen's offset, and how many rows its viewport draws. The viewport is
/// the frame's, not the journal's or the scroll's: the merge keeps that
/// many rows and no more, however deep the scroll (M3, ruling 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Reading {
    pub(crate) rows: usize,
    pub(crate) offset: usize,
    pub(crate) visible: usize,
}

/// `run` in `store` as the console shows it `asked`, and how many events
/// the read verified or loaded. `standing` is what an earlier frame read
/// of it, whose rows the caller has counted at its head (ruling 5), and
/// `head` the run's head this frame stated. `Reading` is how many rows the
/// screen asks for, its scroll, and its viewport. Where that head is the
/// one `standing` was read at and what it read paints the screen asked and
/// covers the rows asked for, nothing is read. Where the screen reads a
/// frame, the events landed since are read, verified as the continuation
/// of the head it was read at, then the frame at the new head and the
/// checkpoints landed since: never the run whole. Otherwise the run is
/// opened, its chain verified whole (ruling 5), and read at the head its
/// opening verified. Whichever read the store says is kept, or does not
/// compose, reads the run whole, as before the pages (ruling 8).
pub(crate) fn console(
    store: &Store,
    run: &str,
    asked: Console,
    standing: Option<Shown>,
    head: Option<&(u64, String)>,
    rows: usize,
    offset: usize,
) -> Result<(Shown, usize), StoreError> {
    console_with(
        store,
        run,
        asked,
        standing,
        head,
        Reading {
            rows,
            offset,
            visible: WINDOW,
        },
    )
}

/// [`console`] with the viewport the frame draws, so a tall pane's read
/// holds its visible rows rather than a fixed window (M3).
pub(crate) fn console_with(
    store: &Store,
    run: &str,
    asked: Console,
    standing: Option<Shown>,
    head: Option<&(u64, String)>,
    reading: Reading,
) -> Result<(Shown, usize), StoreError> {
    let reader = Reader { store, run };
    let Reading { rows, offset, .. } = reading;
    // A carried page must include the row the screen's scroll asks to paint
    // first; the absolute-end request asks for no particular row and reads
    // its window afresh. A held window starting below `offset` omits that
    // row, so it is not carried (M3, ruling 9).
    let floor = match offset {
        usize::MAX => 0,
        offset => offset,
    };
    // A frame read for the run level holds no checkpoint, so any screen
    // may keep it and read its own page; a paged screen's rows are its
    // own order, and a switch to another screen reads that screen's page.
    let standing = standing.filter(|shown| {
        shown.paints(&asked) || matches!(&shown.read, Held::Paged(_, held) if held.pages.is_empty())
    });
    match standing {
        Some(shown)
            if offset != usize::MAX
                && stands(&shown.head, head)
                && shown.paints(&asked)
                && shown.covers(&asked, rows, offset) =>
        {
            Ok((Shown { asked, ..shown }, 0))
        }
        Some(Shown {
            head,
            read: Held::Paged(_, held),
            ..
        }) if held_start(&held) <= floor || matches!(asked, Console::Participant(_)) => {
            carried(reader, &head, held, asked, reading)
        }
        _ => opened(reader, asked, reading),
    }
}

/// Whether the run's head `head` is the one a read at `at` was made at.
fn stands(at: &Opened, head: Option<&(u64, String)>) -> bool {
    head.is_some_and(|(seq, hash)| (*seq, hash.as_str()) == (at.seq, at.hash.as_str()))
}

/// `run` read past `head`, which a read through the pages verified and
/// where it held `held`: the events landed since, verified as its
/// continuation, then the screen at the new head, each participant's
/// checkpoints past those held. Where the store does not read them so, a
/// head it no longer holds or a suffix that does not chain onto it, the
/// run is opened again and refused there in its words. A start among the
/// events landed re-opens an attempt, so the boundary its cached rows read
/// is no longer the one the run whole derives and the page is read again
/// (F2, decision 0089 ruling 11).
fn carried(
    reader: Reader,
    head: &Opened,
    held: Checkpoints,
    asked: Console,
    reading: Reading,
) -> Result<(Shown, usize), StoreError> {
    let Ok(landed) = reader.store.load_after(reader.run, head.seq, &head.hash) else {
        return opened(reader, asked, reading);
    };
    let at = landed.last().map_or_else(
        || head.clone(),
        |event| Opened {
            seq: event.seq,
            hash: event.event_hash.clone(),
        },
    );
    let restarted = landed
        .iter()
        .any(|event| event.event_type == EventType::EffectStarted);
    let held = match restarted {
        true => Checkpoints::new(),
        false => held,
    };
    let (shown, loaded) = read_at(reader, at, held, asked, reading, &landed)?;
    Ok((shown, landed.len() + loaded))
}

/// `run` opened, its chain verified whole: read at its head for the run
/// level, the detail pane or a participant's level.
fn opened(reader: Reader, asked: Console, reading: Reading) -> Result<(Shown, usize), StoreError> {
    let at = reader.store.open_run(reader.run)?;
    let verified = usize::try_from(at.seq).unwrap_or(usize::MAX);
    let (shown, loaded) = read_at(reader, at, Checkpoints::new(), asked, reading, &[])?;
    Ok((shown, verified + loaded))
}

/// `run`'s view at `at`, a head verified, as `asked` reads it: the frame,
/// and the page each participant the ask shows holds, extended by the
/// checkpoints `landed` since those held, and how many events a whole read
/// loaded: through the pages, or whole where a read it makes is kept or
/// does not compose (ruling 8).
fn read_at(
    reader: Reader,
    at: Opened,
    held: Checkpoints,
    asked: Console,
    reading: Reading,
    landed: &[EventEnvelope],
) -> Result<(Shown, usize), StoreError> {
    // The run level takes ruling 8's whole read where the frame does not
    // compose: a folded journal can hold a key two participants make,
    // which the whole view collapses and the composed one declines (R6).
    // Every screen reads through the one paged reader; its run arm holds
    // the frame and no checkpoint.
    let read = paged_at(reader, &at, held, &asked, reading, landed)?
        .map(|(frame, checkpoints)| Held::Paged(frame, checkpoints));
    let (read, loaded) = match read {
        Some(read) => (read, 0),
        None => whole_read(reader.store, reader.run)?,
    };
    Ok((
        Shown {
            head: at,
            asked,
            read,
        },
        loaded,
    ))
}

/// `run` read whole and folded, as before the pages (ruling 8).
fn whole_read(store: &Store, run: &str) -> Result<(Held, usize), StoreError> {
    let events = store.load(run)?;
    let (state, loaded) = (fold(&events).ok().map(Box::new), events.len());
    Ok((Held::Whole(events, state), loaded))
}

/// One read's frame and a page of each participant `asked` shows, extended
/// by `landed` and by the pages `rows` asks. None where the frame does not
/// compose, a participant names no seat row, or the store keeps a page.
fn paged_at(
    reader: Reader,
    at: &Opened,
    held: Checkpoints,
    asked: &Console,
    reading: Reading,
    landed: &[EventEnvelope],
) -> Result<Option<(Frame, Checkpoints)>, StoreError> {
    let Reading {
        rows,
        offset,
        visible,
    } = reading;
    let Some(frame) = Frame::read(reader.store, reader.run, at)? else {
        return Ok(None);
    };
    let Some(view) = frame.view() else {
        return Ok(None);
    };
    let fetch = Fetch {
        store: reader.store,
        run: reader.run,
        at,
        rows,
        offset,
        visible,
        landed,
        trail: &frame.trail,
    };
    let seats: BTreeMap<String, (String, Option<String>)> =
        seat_keys(&frame.seats).into_iter().collect();
    let checkpoints = match asked {
        // The run level reads the frame and no checkpoint.
        Console::Run => return Ok(Some((frame, Checkpoints::new()))),
        // One window cut across every member's stream at one seq boundary,
        // so the merged live column is contiguous (R1). A moved head or a
        // window that dropped its newer prefix is read from the newest
        // page again; a carried read keeps its absolute prefix (F1, F4).
        Console::Detail => {
            let keys = live_keys(&view, &frame.state);
            let Some(checkpoints) = detail_checkpoints(&fetch, &keys, &seats, held, offset)? else {
                return Ok(None);
            };
            checkpoints
        }
        Console::Participant(key) => {
            let Some(checkpoints) = participant_checkpoints(&fetch, &seats, held, key)? else {
                return Ok(None);
            };
            checkpoints
        }
    };
    Ok(Some((frame, checkpoints)))
}

/// The detail pane's window: whether its held page is refilled from the
/// newest page again, and the absolute prefix a carried read keeps (F1,
/// F4).
fn detail_checkpoints(
    fetch: &Fetch<'_>,
    keys: &[String],
    seats: &BTreeMap<String, (String, Option<String>)>,
    held: Checkpoints,
    offset: usize,
) -> Result<Option<Checkpoints>, StoreError> {
    let to_end = offset == usize::MAX;
    let missing = keys.iter().any(|key| !held.pages.contains_key(key));
    let refill = to_end
        || !fetch.landed.is_empty() && (fetch.landed.len() > WINDOW || held.start > 0 || missing)
        || fetch.landed.is_empty() && missing;
    let prefix = match refill {
        true => 0,
        false => held.start,
    };
    fetch.window(keys, seats, held.pages, prefix, refill)
}

/// The participant level's page: the carried rank advances by the
/// checkpoints that landed since it, so a narrowed display page never
/// loses the stream's current end (R3, F1).
fn participant_checkpoints(
    fetch: &Fetch<'_>,
    seats: &BTreeMap<String, (String, Option<String>)>,
    mut held: Checkpoints,
    key: &str,
) -> Result<Option<Checkpoints>, StoreError> {
    let Some((effect, member)) = seats.get(key) else {
        return Ok(None);
    };
    let carried = held.pages.contains_key(key);
    let prior = held.total.filter(|_| carried);
    let page = held.pages.remove(key);
    let Some((page, total)) = fetch.participant((effect.as_str(), member.as_deref()), page)? else {
        return Ok(None);
    };
    let total = total.or_else(|| {
        prior.map(|total| total + landed_count((effect.as_str(), member.as_deref()), fetch.landed))
    });
    let mut checkpoints = Checkpoints::new();
    checkpoints.start = page.start;
    checkpoints.total = total;
    checkpoints.pages.insert(key.to_string(), page);
    Ok(Some(checkpoints))
}

/// The keys of the participants the live column paints: the seat the
/// fold hires, and only that seat's members, so a previous seat's pinned
/// member extends no stream the column draws (F4, ruling 5). The paged
/// read and the whole read's end both count this one set.
fn live_keys(view: &RunView, state: &RunState) -> Vec<String> {
    match state.run.cursor().and_then(|cursor| cursor.hire()) {
        Some((seat, _)) => at_work(view, seat),
        None => Vec::new(),
    }
}

/// The keys of the participants the live column paints for `seat`: the
/// same set `brokkr_view::working_checkpoints` reads its rows from, so the
/// stream the screen counts and the stream it draws are derived once
/// (F4, ruling 5).
fn at_work(view: &RunView, seat: &str) -> Vec<String> {
    brokkr_view::working_participants(view, seat)
        .into_iter()
        .map(|part| part.key.clone())
        .collect()
}

mod fetch;
mod page;

#[cfg(test)]
mod tests;
