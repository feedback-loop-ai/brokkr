//! The console's models and state: the frame's views, the selected
//! transcript subject, what the shell asks the journal for, and the
//! owned selection that survives a refresh (decision 0014).

use super::*;

// ------------------------------------------------------------- the models

/// One frame's worth of derivation, produced by the injected source and
/// dropped after the frame. **No `brokkr-view` model is retained**, which
/// is why "selection survives a refresh" and "selection clears when its
/// subject vanishes" are the absence of code rather than a diff routine.
pub(crate) struct Views {
    /// The one clock read the derivation refuses to make itself.
    pub now: String,
    pub runs: RunsView,
    pub run: Option<RunView>,
    /// The one selected participant's complete bounded local read. `None`
    /// means no participant is open, never "the file is missing" — a
    /// missing, refused or unowned source is the reader's own result and
    /// stays visible rather than collapsing into an absence.
    pub transcript: Option<TranscriptRead>,
    /// What this frame has to SAY about the hearth it was read from, as
    /// opposed to what it failed to do. A realm whose journal is not
    /// there yet is empty rather than unreadable (decision 0026 ruling
    /// 2), so it arrives as an ordinary frame carrying a sentence — the
    /// status line states it and the keys stay live, where an `Err`
    /// would have counted toward the give-up bound and ended the
    /// console over a realm that has simply not run yet.
    pub note: Option<String>,
    /// What the fleet's pages leave unread (decision 0089 ruling 6).
    pub unread: Unread,
    /// The window of the fleet this frame was read for.
    pub window: Window,
    /// The key past the window's first page, where the page ahead
    /// begins, when the fleet runs on past it.
    pub ahead: Option<brokkr_view::Cursor>,
    /// Whether the selected screen's held page has a next one, so a
    /// scroll past its rows asks for it (decision 0089 slice 3).
    pub more: bool,
    /// How many rows of the selected screen's order stand before the page
    /// it holds, so the panes scroll from the window they have rather than
    /// the top of the stream it dropped (decision 0089 ruling 9).
    pub start: usize,
    /// The selected screen's whole rank, where its end key asked for it:
    /// the offset the end key's request resolves to (M3).
    pub end: Option<usize>,
}

/// Where the fleet's list reads from (decision 0089 ruling 6): the key
/// past the rows it has scrolled beyond, or its top, and the filter it
/// keeps rows by. A frame reads the page there and the one ahead.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Window {
    pub after: Option<brokkr_view::Cursor>,
    pub filter: String,
}

/// How far the fleet's list has scrolled: the key past each of the last
/// [`SCROLL_BACK`] pages it has scrolled beyond, in order, under the filter
/// it scrolled with. It reads from the last; a list under another filter
/// reads from its top.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Scrolled {
    pub filter: String,
    pub past: Vec<brokkr_view::Cursor>,
}

/// How many pages back the list keeps the keys to step to, so what it
/// holds does not grow with how far it scrolls (decision 0089 rulings 6
/// and 9). A step back past them reaches its top, until the fleet's last
/// page by the reverse order, a later slice's, reads back by keyset.
pub(crate) const SCROLL_BACK: usize = 8;

impl Scrolled {
    /// The list scrolled past the page `ahead` ends, keeping the keys of
    /// the last [`SCROLL_BACK`] pages alone.
    pub(crate) fn passed(&mut self, ahead: brokkr_view::Cursor) {
        if self.past.len() == SCROLL_BACK {
            self.past.remove(0);
        }
        self.past.push(ahead);
    }
}

/// The runs a frame's pages did not read: the older ones past them,
/// counted by the summary's index, and the runs behind their summary past
/// those a refresh reads whole, which "needs you" counts in one line.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Unread {
    pub older: usize,
    pub behind: u64,
}

impl Views {
    /// What the first frame draws when the journal is unreadable: an
    /// empty fleet, never an invented one (decision 0001).
    pub(crate) fn empty() -> Views {
        Views {
            now: String::new(),
            runs: RunsView::of(Vec::new()),
            run: None,
            transcript: None,
            note: None,
            unread: Unread::default(),
            window: Window::default(),
            ahead: None,
            more: false,
            start: 0,
            end: None,
        }
    }
}

/// What one refresh answers: fresh models, or `None` for "the head has
/// not moved, keep the frame you have".
pub(crate) type Refreshed = Option<Views>;

/// What the shell's source has for it at one poll (#744). A source on
/// the loop has always answered; one read off it may not have yet, and
/// a read still out is neither a read that found nothing new nor one
/// that failed: it counts for and against nothing.
pub(crate) enum Polled {
    /// A read answered, as a source on the loop answers.
    Answered(Box<Result<Refreshed>>),
    /// A read is still out.
    Pending,
}

impl From<Result<Refreshed>> for Polled {
    fn from(answer: Result<Refreshed>) -> Polled {
        Polled::Answered(Box::new(answer))
    }
}

/// The selected transcript subject, as the view derives it once for every
/// surface (#351).
pub(crate) use brokkr_view::Subject;

/// What the shell asks the journal for. Selection reaches a store only
/// through this struct — the TUI itself never holds one.
pub(crate) struct Ask<'a> {
    pub run: Option<&'a str>,
    /// The participant whose transcript this frame is about, or `None`
    /// away from the participant level.
    pub subject: Option<Subject>,
    /// The RUN level is the screen on the frame: it reads the run's frame
    /// and no seat's checkpoints (decision 0089 slice 3).
    pub run_level: bool,
    /// `r`, a level change, or the first frame: rebuild regardless.
    pub force: bool,
    /// The fleet's slower cadence is due.
    pub fleet: bool,
    /// Which hearth is being read (decision 0026 ruling 2). The shell
    /// asks about the ACTIVE tab and no other, so a tab nobody has
    /// visited has had no store opened for it — the laziness is the
    /// absence of a question, not a cache.
    pub tab: usize,
    /// The window of the fleet the list asks to be read: the page its
    /// window starts at and the one ahead (decision 0089 ruling 6).
    pub window: Window,
    /// How many checkpoint rows the screen shows, so how many it asks the
    /// run's pages for: the live column's or the participant pane's
    /// scroll and the pane's height, and no more (decision 0089 slice 3).
    pub live_rows: usize,
    /// The scroll's own row: the participant pane's first row, whose
    /// window the page holds (R2, F2, F3).
    pub live_offset: usize,
}

/// How many rows the selected screen shows of a seat's checkpoints: the
/// live column's or the participant pane's scroll plus the pane's height
/// on the frame the shell measured. The run level shows none.
pub(crate) fn asked_rows(tui: &Tui) -> usize {
    match tui.level {
        Level::Run => 0,
        // The end key asks for the stream's end, not for a rank-plus-height
        // that saturates: the read holds the frame's viewport past the rank
        // it resolves, so the page it keeps is the pane's, not `usize::MAX`
        // (M3, M4). A held key's request is the ask until it is answered.
        Level::Runs | Level::Participant => match ask_offset(tui) {
            usize::MAX => usize::from(tui.height),
            offset => offset.saturating_add(usize::from(tui.height)),
        },
    }
}

/// The page the shell asks the read for: the held movement key's request
/// while one waits for its page, else the pane's own scroll. A request
/// never moves the cursor; it only says which rows the answer must hold.
pub(crate) fn ask_offset(tui: &Tui) -> usize {
    tui.want.unwrap_or_else(|| checkpoint_offset(tui))
}

/// The checkpoint pane's scroll as the frame displays it: the live
/// column's, or the participant pane's when that pane has the focus. An
/// unfocused participant pane paints from its top, so its read asks for
/// the oldest window and the focused offset waits for Tab back (M4).
pub(crate) fn checkpoint_offset(tui: &Tui) -> usize {
    match tui.level {
        Level::Runs => tui.live_offset,
        Level::Participant => match tui.pane {
            0 => display_rank(tui.offset),
            _ => 0,
        },
        Level::Run => 0,
    }
}

/// The absolute rank today's participant renderer can display: its
/// `u16` scroll narrows every offset above it. The read chooses the page
/// that rank names before translating into the window, rather than
/// subtracting an unclamped page start from the narrowed rank (M4). The
/// end key's `usize::MAX` request stays whole for the read to answer.
fn display_rank(offset: usize) -> usize {
    match offset {
        usize::MAX => offset,
        offset => offset.min(DISPLAY_RANK),
    }
}

/// A console ask away from the RUN level: the tests that open a run from
/// the fleet do not spell the field that tells its run level from the
/// fleet's (decision 0089 slice 3).
#[cfg(test)]
macro_rules! ask {
    ($($field:tt)*) => { $crate::tui::Ask { run_level: false, live_rows: 0, live_offset: 0, $($field)* } };
}

#[cfg(test)]
pub(crate) use ask;

// -------------------------------------------------------------- the state

/// What a session of the console watches besides its keys (#508): one
/// run view, whichever verb opened it.
#[derive(Clone, Debug)]
pub(crate) enum Watched {
    /// `brokkr tui`: nothing, and only a key ends it.
    Console,
    /// `brokkr run` and `resume`: the engine driving the run in the same
    /// process sets how its drive ended once it has returned or unwound,
    /// and Ctrl+C stops the run as it does without the view.
    // Spelled out: the graph's own `Arc` is the road back.
    Driven(std::sync::Arc<std::sync::OnceLock<DriveEnd>>),
    /// `brokkr watch`: the run named, which has ended once its journal
    /// folds to a status other than running, as watch's frames end then.
    Journal(String),
}

/// How the drive beside a run view ended (#508).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DriveEnd {
    /// The drive returned: the view holds its final frame until a key.
    Returned,
    /// The drive panicked: the panic hook has already left the terminal,
    /// so the view closes at once and draws nothing more.
    Unwound,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Level {
    Runs,
    Run,
    Participant,
}

/// One tab's own place in its hearth: what was selected, what was
/// filtered for, and where a paragraph pane was scrolled to. Parked when
/// a tab is left and restored when it is returned to, so a switch
/// neither resets a tab nor bleeds one tab's state into another's
/// (decision 0026 ruling 2).
#[derive(Clone, Default)]
pub(crate) struct TabState {
    pub cursor: Option<String>,
    pub filter: String,
    pub offset: usize,
    /// Whether this hearth's fleet lists its older runs (`a`).
    pub all: bool,
    /// The running runs this hearth's fleet has opened in place (`→`).
    pub expanded: BTreeSet<String>,
    /// The columns this hearth's fleet shows beside its list (`d`, `f`).
    pub toggles: Toggles,
    /// Whether its dashboard shows the whole commission (`c`).
    pub commission: bool,
    /// How far this hearth's fleet has scrolled.
    pub scrolled: Scrolled,
}

/// The cursor a held run replaced with the absolute-end marker: the
/// position the operator left, kept so cancelling the run can put it back
/// instead of leaving a movement the queue no longer owns (M1, decision
/// 0071 ruling 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldOrigin {
    /// The live column's cursor: the checkpoint row and the line within it.
    Live { offset: usize, line: usize },
    /// The participant level's checkpoint rank, in the stream's own
    /// coordinate.
    Rank(usize),
}

/// A movement key held while a page read is out, or while its own
/// destination is not in the held window: a run of the same step is one
/// entry with a count, so a long hold stays bounded without losing an
/// order (decision 0071 ruling 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Held {
    pub(crate) step: Step,
    pub(crate) count: usize,
    /// The cursor this run's one application replaced with the
    /// absolute-end marker, while the answer that names the end is still
    /// out. Only the head carries it; dropping or evicting the run
    /// restores it, and only that run's own answer consumes it (M1).
    pub(crate) end: Option<HeldOrigin>,
}

/// How many held runs the queue keeps. Past it the oldest run is dropped,
/// and the status line says so.
pub(crate) const HELD_BOUND: usize = 64;

/// The view a held movement was pressed on: the level, the run and the
/// pane that owned the paged window, so an answer is applied only while
/// the operator is still looking at it (decision 0071 ruling 5). A frame
/// that changes any of these leaves the queue behind rather than moving
/// the fleet or another seat with it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct HeldView {
    tab: usize,
    level: Level,
    run: Option<String>,
    seat: Option<String>,
    pane: usize,
    selected: Option<String>,
    column: FleetColumn,
}

/// The view `tui` is showing under `views`: the fields a held movement
/// belongs to. `selected` is the fleet's selected run and `column` the
/// pane the keys move, so a reselected fleet or a column a narrowed frame
/// no longer draws is a different view.
pub(crate) fn held_view(tui: &Tui, views: &Views) -> HeldView {
    HeldView {
        tab: tui.tab,
        level: tui.level,
        run: tui.run.clone(),
        seat: tui.seat.clone(),
        pane: tui.pane,
        selected: selected_run(tui, views).map(|row| key_of(row).to_string()),
        column: focused(tui, views),
    }
}

/// Owned scalars only. Selection is by **stable key** — `RunRow.run_id`,
/// `Phase.name`, `Participant.key`, `JournalRow.seq` — resolved against
/// whatever models the current frame carries.
#[expect(clippy::struct_excessive_bools, reason = "baseline 2026-09, #288")]
pub(crate) struct Tui {
    pub level: Level,
    pub run: Option<String>,
    pub seat: Option<String>,
    pub scope: Option<brokkr_view::Scope>,
    /// One key per pane slot; cleared when the run changes.
    pub cursor: [Option<String>; 3],
    /// The `Node.key` the graph cursor has walked into, subordinate to
    /// `cursor[0]`. It **scopes**: a node whose key names a participant
    /// makes that seat the scope, one level down from the rail's own
    /// "moving IS scoping" (see [`graph_scope`]); a structural node and
    /// the empty lane both fall back to the rail's phase. `Enter` still
    /// scopes the phase whatever it says. Vanishing is the absence of
    /// code — a stale key matches no drawn node and no participant, so
    /// nothing highlights and nothing stays scoped.
    pub node: Option<String>,
    /// The transcript pane's cursor: the selected turn's INDEX in the
    /// stream, held as a key like every other cursor. Live prose
    /// streaming only APPENDS turns, so an index is as stable a key as
    /// a `seq` — the cursor survives an appending refresh by the same
    /// absence of code as every list.
    pub turn: Option<String>,
    /// Animation is enabled exactly when colour is (§the pulse): no new
    /// flag, no new env var, no new doc surface.
    pub animate: bool,
    /// The viewport of a paragraph pane, which has an offset rather than
    /// a cursor.
    pub offset: usize,
    pub pane: usize,
    pub filter: String,
    pub typing: bool,
    pub help: bool,
    /// A row opened for reading: panes clamp their text to the frame,
    /// so a long one (a feature text, a park reason, an error) would
    /// otherwise be unreadable — truncation with no way through is a
    /// dead end, not evidence.
    pub reading: Option<String>,
    /// True when the open reader is the transcript pane's door, so a
    /// notice-only refresh recomposes it from the shared read instead of
    /// leaving a stale notice set behind.
    pub reading_transcript: bool,
    /// Scroll within the reader, in wrapped lines.
    pub read_offset: usize,
    pub status: Option<String>,
    /// A sentence a key must still show after a frame answers in the same
    /// tick: the held queue's overflow notice, which `arrive` would
    /// otherwise overwrite with a frame's own note before the draw (M2).
    /// The shell clears it once the frame carrying it has been drawn.
    pub notice: Option<String>,
    pub ticks: usize,
    /// Consumed by the shell: `r`, a level change, or the first frame.
    pub force: bool,
    /// The world's hearths, as realm names, when it holds more than one
    /// journal (decision 0026 ruling 2). EMPTY or one-long is a world
    /// that draws no tab bar and behaves exactly as it always did.
    pub tabs: Vec<String>,
    /// Which of them is being read. Always `0` without tabs.
    pub tab: usize,
    /// One parked [`TabState`] per tab, so switching away and back is a
    /// return rather than a reset.
    pub parked: Vec<TabState>,
    /// `a`: the fleet lists its older runs too, where it otherwise folds
    /// them into one count line (#491).
    pub all: bool,
    /// How far the fleet's list has scrolled, by keyset (decision 0089
    /// ruling 6).
    pub scrolled: Scrolled,
    /// The running runs the fleet has opened in place, by id (#503): `→`,
    /// `l` or Space open the selected one, `←` or `h` fold it.
    pub expanded: BTreeSet<String>,
    /// The width of the frame, measured by the shell before it draws, so
    /// the frame, its footer and the keys pressed against it agree on
    /// which of the fleet's columns are on it. Zero until the first
    /// frame.
    pub width: u16,
    /// The frame's height, measured with its width: how many older runs
    /// the fleet's list has room to draw (#503). Zero until the first
    /// frame, when the list draws none of them.
    pub height: u16,
    /// The columns the fleet shows beside its list, `d` and `f` (#503).
    pub toggles: Toggles,
    /// `c`: the dashboard shows the selected run's whole commission.
    pub commission: bool,
    /// The live or findings column's scroll: a checkpoint row in the
    /// stream for the live column, a drawn line for the findings, and the
    /// drawn line within that row the viewport starts at.
    pub live_offset: usize,
    pub live_line: usize,
    /// The movement keys pressed while a page read is out, in order,
    /// between them and their answer. The answer's window settles them;
    /// a key that crosses it asks the next page and keeps its place at
    /// the head (decision 0071 ruling 5).
    pub held: Vec<Held>,
    /// The view the queue was pressed on; a different view drops it.
    pub held_for: Option<HeldView>,
    /// The page a held key waits for: the rank the next read must hold.
    /// It is not the cursor; the cursor waits where the operator left it.
    pub want: Option<usize>,
    /// Whether a read is still out, as the shell last polled. Keys that
    /// move a paged window are held while it is.
    pub pending: bool,
    /// What this session watches besides its keys (#508).
    pub watched: Watched,
    /// The run it watched has ended: the frame stays until any key.
    pub ended: bool,
}

impl Tui {
    /// The console over a world that named no hearths — the shape every
    /// headless test drives, and exactly what [`Tui::over`] reduces to
    /// when the world holds one journal.
    #[cfg(test)]
    pub(crate) fn new(run: Option<String>) -> Tui {
        Tui::over(run, Vec::new(), 0)
    }

    /// The console over a world's hearths, opening on the one that holds
    /// the named run. One hearth (or none named) is the console this file
    /// has always drawn: no bar, no switching, and nothing on the frame
    /// that says a tab exists.
    ///
    /// `--run <id>` opens at the RUN level for that run; `Esc` then walks
    /// the ladder to the full fleet rather than exiting, so the flag
    /// needs no special case anywhere else.
    pub(crate) fn over(run: Option<String>, tabs: Vec<String>, tab: usize) -> Tui {
        let level = match run {
            Some(_) => Level::Run,
            None => Level::Runs,
        };
        let parked = vec![TabState::default(); tabs.len()];
        Tui {
            tab: tab.min(tabs.len().saturating_sub(1)),
            tabs,
            parked,
            level,
            run,
            seat: None,
            scope: None,
            cursor: [None, None, None],
            node: None,
            turn: None,
            animate: false,
            offset: 0,
            pane: 0,
            filter: String::new(),
            typing: false,
            help: false,
            reading: None,
            reading_transcript: false,
            read_offset: 0,
            status: None,
            notice: None,
            ticks: 0,
            force: true,
            all: false,
            scrolled: Scrolled::default(),
            expanded: BTreeSet::new(),
            width: 0,
            height: 0,
            toggles: Toggles::default(),
            commission: false,
            live_offset: 0,
            live_line: 0,
            held: Vec::new(),
            held_for: None,
            want: None,
            pending: false,
            watched: Watched::Console,
            ended: false,
        }
    }

    /// The only writer of `run`, and the invariant a run-qualified
    /// selection type would otherwise buy: a new run clears every
    /// selection made under the old one.
    pub(super) fn assign_run(&mut self, id: String) {
        self.run = Some(id);
        self.level = Level::Run;
        self.pane = 0;
        self.scope = None;
        self.seat = None;
        self.filter.clear();
        self.typing = false;
        self.cursor = [None, None, None];
        self.node = None;
        self.turn = None;
        self.offset = 0;
        self.live_line = 0;
        self.held.clear();
        self.held_for = None;
        self.want = None;
        self.force = true;
    }
}

/// The runs pane has tabs exactly when the world holds more than one
/// journal. One hearth draws no bar and binds no tab key: a world that
/// never drew two journals notices nothing (decision 0026 ruling 2).
pub(super) fn tabbed(tui: &Tui) -> bool {
    tui.tabs.len() > 1
}

/// Move to another hearth. Each tab's selection, filter and scroll are
/// parked on the way out and restored on the way in; everything below
/// the fleet — the open run, the seat, the scope — is NOT carried
/// across, because a run id lives in exactly one journal and the one
/// being left is not the one being entered (ruling 3).
pub(super) fn switch(tui: &mut Tui, index: usize) {
    if !tabbed(tui) || index >= tui.tabs.len() || index == tui.tab {
        return;
    }
    tui.parked[tui.tab] = TabState {
        cursor: tui.cursor[0].clone(),
        filter: tui.filter.clone(),
        offset: tui.offset,
        all: tui.all,
        expanded: std::mem::take(&mut tui.expanded),
        toggles: tui.toggles,
        commission: tui.commission,
        scrolled: std::mem::take(&mut tui.scrolled),
    };
    let resumed = tui.parked[index].clone();
    tui.tab = index;
    tui.cursor = [resumed.cursor, None, None];
    tui.filter = resumed.filter;
    tui.offset = resumed.offset;
    tui.all = resumed.all;
    tui.expanded = resumed.expanded;
    tui.toggles = resumed.toggles;
    tui.commission = resumed.commission;
    tui.scrolled = resumed.scrolled;
    tui.live_offset = 0;
    tui.live_line = 0;
    tui.held.clear();
    tui.held_for = None;
    tui.want = None;
    tui.level = Level::Runs;
    tui.pane = 0;
    tui.run = None;
    tui.seat = None;
    tui.scope = None;
    tui.node = None;
    tui.turn = None;
    tui.typing = false;
    tui.reading = None;
    tui.reading_transcript = false;
    tui.read_offset = 0;
    // This hearth's journal has not been read yet; the shell asks for it
    // on the next frame, which is when its store is first opened at all.
    tui.force = true;
}

/// Any run in the fleet is running: the gate for the brand mark's
/// pulse, refreshed on the fleet cadence the shell already keeps. A
/// stale run is listed as needing the operator, not as running (#503),
/// so it never makes the mark pulse.
pub(super) fn fleet_live(views: &Views) -> bool {
    views.runs.runs.iter().any(|row| {
        row.status.as_deref() == Some("running") && brokkr_view::need(row, &views.now).is_none()
    })
}

/// The panes `Tab` moves across: the fleet's are the columns on its
/// frame (#503).
pub(super) fn panes_at(tui: &Tui, views: &Views) -> usize {
    match tui.level {
        Level::Runs => fleet_columns(tui, views).len(),
        Level::Run => 3,
        Level::Participant => 2,
    }
}

/// The run the shell reads a view of: the open run, or at the fleet the
/// selected one while a column beside the list shows it (#503), through
/// the same question the run level asks.
pub(super) fn asked_run(tui: &Tui, views: &Views) -> Option<String> {
    match tui.level {
        Level::Runs => detail_row(tui, views).map(|row| key_of(row).to_string()),
        Level::Run | Level::Participant => tui.run.clone(),
    }
}

/// The scoped seat, when the scope is a seat at all. `Option` is the
/// exclusivity rule: one scope at a time is one field, not a policy.
pub(super) fn scoped_seat(tui: &Tui) -> Option<&str> {
    match &tui.scope {
        Some(brokkr_view::Scope::Seat(key)) => Some(key),
        _ => None,
    }
}

pub(super) fn participant<'a>(views: &'a Views, key: &str) -> Option<&'a Participant> {
    views
        .run
        .as_ref()?
        .participants
        .iter()
        .find(|part| part.key == key)
}

/// Resolve the scope against the **fresh** models. `.ok().flatten()` is
/// load-bearing: `lens_for`'s `Err` arm means "this run has no such
/// phase or seat", which for a TUI *is* the vanished-subject case, so
/// one mechanism answers both requirements.
pub(super) fn lens_of(tui: &Tui, views: &Views) -> Option<brokkr_view::Lens> {
    views.run.as_ref().and_then(|view| {
        brokkr_view::lens_for(view, tui.scope.as_ref())
            .ok()
            .flatten()
    })
}

pub(super) fn seat_of<'a>(tui: &Tui, views: &'a Views) -> Option<&'a Participant> {
    tui.seat.as_deref().and_then(|key| participant(views, key))
}

/// The subject the shell re-resolves: exactly the selected participant at
/// the participant level, in the active hearth.
pub(super) fn subject_of(tui: &Tui, views: &Views) -> Option<Subject> {
    if tui.level != Level::Participant {
        return None;
    }
    let part = seat_of(tui, views)?;
    let run = tui.run.clone()?;
    Some(Subject::of(
        tui.tab,
        tui.tabs.get(tui.tab).cloned(),
        run,
        part,
    ))
}

/// Whether a refreshed read replaces the previously displayed turns. A
/// pure append — the new projection begins with every old turn, in order,
/// under the same authority — keeps navigation. Removal, replacement,
/// reorder, shrink, a changed reference or path, and every refusal
/// (ambiguity, ownership loss, format refusal) invalidate the cursor and
/// close an open door before new indices are shown. A notice-only change
/// leaves the turns alone and therefore keeps the cursor.
pub(super) fn transcript_invalidates(
    old: Option<&TranscriptRead>,
    new: Option<&TranscriptRead>,
) -> bool {
    let (Some(old), Some(new)) = (old, new) else {
        // A subject appearing or vanishing is a new list either way.
        return old.is_some() != new.is_some();
    };
    if new.unavailable.is_some() || old.unavailable.is_some() {
        return true;
    }
    // D8's source-identity rule: a same-content replacement (a new inode
    // or changed member provenance) invalidates the cursor and overlay
    // even when every projected field is identical.
    if new.source_identity != old.source_identity {
        return true;
    }
    if new.reference != old.reference || new.path != old.path || new.kind != old.kind {
        return true;
    }
    if new.turns.len() < old.turns.len() {
        return true;
    }
    new.turns[..old.turns.len()] != old.turns
}
