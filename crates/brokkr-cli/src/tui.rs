//! `brokkr tui` — the interactive, read-only console (decision 0014).
//!
//! A **third renderer** over `brokkr-view`'s models, never a fourth
//! derivation: every value drawn below is a model field, and the only
//! computation here is selecting, filtering, arranging and laying out.
//! A renderer may branch on a model field; it may not compute one.
//!
//! **Read-only, structurally.** Nothing in this file names a store, a
//! runtime, or a journal write. The pure core and the draw path receive
//! view models; the shell receives one injected refresh source. The code
//! that *could* write is not reachable from here, and `src/tests.rs`
//! holds that as a source-level property.
//!
//! **The partition.** `apply(&mut Tui, &Views, Key) -> Flow` is a pure
//! state machine — no terminal, no store, no I/O — so every navigation
//! path is unit-tested headlessly. `draw` is generic over
//! `ratatui::backend::Backend`, so `TestBackend` reaches every widget.
//! The five process-global crossterm calls are `fn`-pointer fields whose
//! production values are crossterm's own function items, and the shell
//! is driven by an injected key source and an injected refresh source,
//! so its loop, its error arms and its transient-busy arms all execute
//! without a terminal.
//!
//! **Terminal safety.** Journal text is seat-authored. Exactly two
//! constructors reach a widget — [`cell`] and [`span`] — and both take a
//! [`Safe`], so "did this one get sanitized" is answerable by grep. The
//! widths used for layout are ratatui's own measurement of that same
//! sanitized text, so the invariant holds by construction rather than by
//! a second width implementation.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::Result;
use brokkr_view::{Column, Node, Participant, Phase, RunView, RunsView};
use ratatui::backend::Backend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, Wrap};
use ratatui::{Frame, Terminal};

use crate::render::{self, Safe, Tone};
use crate::ui::Turn;

/// Below this the frame cannot hold its panes, and a drawn frame would
/// be a corrupted one.
pub(crate) const MIN_WIDTH: u16 = 60;
pub(crate) const MIN_HEIGHT: u16 = 12;

/// The input poll. Latency is bounded by the keypress, not the tick: an
/// operator holding `j` must not feel a quarter-second sleep.
const TICK: Duration = Duration::from_millis(250);

/// The fleet's slower cadence, ≈2s. Re-folding every event of every run
/// four times a second against a `brokkr run` holding the write lock is
/// the cost this constant exists to refuse (spec §6).
const RUNS_REFRESH_TICKS: usize = 8;

/// One `PageUp`/`PageDown` in list rows.
const PAGE: usize = 10;

/// One pulse frame per this many shell ticks: four frames × 2 × `TICK`
/// ≈ a two-second breath, the terminal's answer to the console's 1.8s
/// keyframe. **No new clock and no new wakeup**: [`drive`] already
/// redraws every `TICK` and already carries `Tui::ticks`, so animation
/// adds exactly zero draws and zero timers.
const PULSE_TICKS: usize = 2;
const PULSE_FRAMES: usize = 4;

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
    pub transcript: Option<(Vec<Turn>, bool)>,
}

impl Views {
    /// What the first frame draws when the journal is unreadable: an
    /// empty fleet, never an invented one (decision 0001).
    pub(crate) fn empty() -> Views {
        Views {
            now: String::new(),
            runs: RunsView {
                view_version: brokkr_view::VIEW_VERSION,
                runs: Vec::new(),
                count: 0,
            },
            run: None,
            transcript: None,
        }
    }
}

/// What one refresh answers: fresh models, or `None` for "the head has
/// not moved, keep the frame you have".
pub(crate) type Refreshed = Option<Views>;

/// What the shell asks the journal for. Selection reaches a store only
/// through this struct — the TUI itself never holds one.
pub(crate) struct Ask<'a> {
    pub run: Option<&'a str>,
    pub session: Option<&'a str>,
    /// Whether that session can still gain words. The shell watches the
    /// transcript file only while this holds; a concluded seat's
    /// transcript is already whole, so there is nothing to poll for.
    pub working: bool,
    /// `r`, a level change, or the first frame: rebuild regardless.
    pub force: bool,
    /// The fleet's slower cadence is due.
    pub fleet: bool,
}

// -------------------------------------------------------------- the state

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Level {
    Runs,
    Run,
    Participant,
}

/// Owned scalars only. Selection is by **stable key** — `RunRow.run_id`,
/// `Phase.name`, `Participant.key`, `JournalRow.seq` — resolved against
/// whatever models the current frame carries.
pub(crate) struct Tui {
    pub level: Level,
    pub run: Option<String>,
    pub seat: Option<String>,
    pub scope: Option<render::Scope>,
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
    /// Scroll within the reader, in wrapped lines.
    pub read_offset: usize,
    pub status: Option<String>,
    pub ticks: usize,
    /// Consumed by the shell: `r`, a level change, or the first frame.
    pub force: bool,
}

impl Tui {
    /// `--run <id>` opens at the RUN level for that run; `Esc` then walks
    /// the ladder to the full fleet rather than exiting, so the flag
    /// needs no special case anywhere else.
    pub(crate) fn new(run: Option<String>) -> Tui {
        let level = match run {
            Some(_) => Level::Run,
            None => Level::Runs,
        };
        Tui {
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
            read_offset: 0,
            status: None,
            ticks: 0,
            force: true,
        }
    }

    /// The only writer of `run`, and the invariant a run-qualified
    /// selection type would otherwise buy: a new run clears every
    /// selection made under the old one.
    fn assign_run(&mut self, id: String) {
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
        self.force = true;
    }
}

/// Any run in the fleet is running: the gate for the brand mark's
/// pulse, refreshed on the fleet cadence the shell already keeps.
fn fleet_live(views: &Views) -> bool {
    views
        .runs
        .runs
        .iter()
        .any(|row| row.status.as_deref() == Some("running"))
}

fn panes_at(level: Level) -> usize {
    match level {
        Level::Runs => 1,
        Level::Run => 3,
        Level::Participant => 2,
    }
}

/// The scoped seat, when the scope is a seat at all. `Option` is the
/// exclusivity rule: one scope at a time is one field, not a policy.
fn scoped_seat(tui: &Tui) -> Option<&str> {
    match &tui.scope {
        Some(render::Scope::Seat(key)) => Some(key),
        _ => None,
    }
}

fn participant<'a>(views: &'a Views, key: &str) -> Option<&'a Participant> {
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
fn lens_of(tui: &Tui, views: &Views) -> Option<render::Lens> {
    views
        .run
        .as_ref()
        .and_then(|view| render::lens_for(view, tui.scope.as_ref()).ok().flatten())
}

/// Selection clears itself when its subject disappears — the rule the
/// console already follows. A **filter** never clears a scope: absence
/// from a filtered list is a display fact, and this runs against the
/// unfiltered models.
fn settle(tui: &mut Tui, views: &Views) {
    if lens_of(tui, views).is_none() {
        tui.scope = None;
    }
    seed_cursor(tui, views);
    if seat_of(tui, views).is_none() {
        tui.seat = None;
        if tui.level == Level::Participant {
            tui.level = Level::Run;
            tui.pane = 1;
        }
    }
    // A fleet that lists runs and does not list this one has lost it.
    // An empty fleet is the unreadable-journal frame, which is not
    // evidence that the run went away.
    let vanished = match &tui.run {
        Some(run) => {
            !views.runs.runs.is_empty() && !views.runs.runs.iter().any(|r| r.run_id == *run)
        }
        None => false,
    };
    if vanished {
        tui.run = None;
        tui.level = Level::Runs;
        tui.pane = 0;
        tui.scope = None;
        tui.seat = None;
        tui.cursor = [None, None, None];
    }
}

fn seat_of<'a>(tui: &Tui, views: &'a Views) -> Option<&'a Participant> {
    tui.seat.as_deref().and_then(|key| participant(views, key))
}

/// The session id the shell asks the transcript lookup for: a model
/// field, read only while the operator is reading that seat.
fn session_of<'a>(tui: &Tui, views: &'a Views) -> Option<&'a str> {
    match tui.level {
        Level::Participant => seat_of(tui, views).and_then(|part| part.session_id.as_deref()),
        _ => None,
    }
}

/// Whether the session the shell is asking for can still gain prose:
/// `status` is the same model field the seats table branches on, and
/// this branches on it rather than deriving anything. Pure, and asked
/// once where the `Ask` is built — never inside [`apply`], which stays a
/// state machine over models with no notion of a file at all.
fn session_is_live(tui: &Tui, views: &Views) -> bool {
    session_of(tui, views).is_some()
        && seat_of(tui, views).is_some_and(|part| part.status == "working")
}

// --------------------------------------------------------------- the keys

/// Our own key vocabulary. Translation happens once, at the boundary.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Key {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Tab,
    Enter,
    Escape,
    Backspace,
    Char(char),
    Quit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Flow {
    Continue,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Step {
    Up,
    Down,
    Top,
    Bottom,
    PageUp,
    PageDown,
}

/// Mouse, paste and focus events are ignored by **named** arms: a
/// wildcard here is an untested claim about what a terminal can send.
pub(crate) fn from_crossterm(event: Event) -> Option<Key> {
    match event {
        Event::Key(key) => from_key(key),
        Event::Mouse(_) => None,
        Event::Paste(_) => None,
        Event::FocusGained => None,
        Event::FocusLost => None,
        Event::Resize(_, _) => None,
    }
}

/// Windows delivers key **release** events too; a handler matching on
/// `KeyCode` alone would process every keystroke twice on the exact CI
/// leg crossterm exists to make survivable. `Ctrl+C` quits alongside
/// `q`, because raw mode disables SIGINT.
fn from_key(key: KeyEvent) -> Option<Key> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Some(Key::Quit);
    }
    match key.code {
        KeyCode::Up => Some(Key::Up),
        KeyCode::Down => Some(Key::Down),
        KeyCode::Left => Some(Key::Left),
        KeyCode::Right => Some(Key::Right),
        KeyCode::PageUp => Some(Key::PageUp),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::BackTab => Some(Key::Tab),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Escape),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Char(character) => Some(Key::Char(character)),
        _ => None,
    }
}

// ----------------------------------------------------------- the movement

/// Every navigable list is a `Vec` of stable keys, filtered over
/// **sanitized** labels: the filter is matched against exactly the text
/// the operator can see.
fn keys_for(tui: &Tui, views: &Views) -> Vec<String> {
    let needle = Safe::new(&tui.filter).as_str().to_lowercase();
    labels_for(tui, views)
        .into_iter()
        .filter(|(_, label)| label.to_lowercase().contains(&needle))
        .map(|(key, _)| key)
        .collect()
}

fn labels_for(tui: &Tui, views: &Views) -> Vec<(String, String)> {
    match (tui.level, views.run.as_ref()) {
        (Level::Runs, _) => views
            .runs
            .runs
            .iter()
            .map(|row| {
                let mut label = row.run_id.clone();
                label.push(' ');
                label.push_str(&row.feature);
                (row.run_id.clone(), safe(&label))
            })
            .collect(),
        (Level::Run, Some(view)) => run_labels(tui, view, lens_of(tui, views).as_ref()),
        // The PARTICIPANT panes are paragraphs with an offset, and a run
        // that will not load has no lists to move over.
        _ => Vec::new(),
    }
}

fn run_labels(tui: &Tui, view: &RunView, lens: Option<&render::Lens>) -> Vec<(String, String)> {
    match tui.pane {
        // The graph is the SELECTOR: it lists every phase the run
        // visited, scoped or not, because a pane that hid the phases
        // you might scope next could never replace one scope with
        // another. The lens marks; it does not hide, here.
        0 => view
            .phases
            .iter()
            .map(|phase| (phase.name.clone(), safe(&phase.name)))
            .collect(),
        1 => view
            .participants
            .iter()
            .filter(|part| render::keeps_participant(lens, part))
            .map(|part| (part.key.clone(), safe(&part.label)))
            .collect(),
        _ => view
            .journal
            .iter()
            .filter(|row| row.in_trail && render::keeps_row(lens, row))
            .map(|row| (row.seq.to_string(), safe(&row.what.text)))
            .collect(),
    }
}

fn index_of(keys: &[String], cursor: &Option<String>) -> Option<usize> {
    let cursor = cursor.as_deref()?;
    keys.iter().position(|key| key == cursor)
}

/// The one place wrap-around, `g`/`G` and paging exist, for every list
/// at every level. A cursor whose key is gone — the subject vanished, or
/// the filter excluded it — restarts from the top and renders no
/// highlight until it moves.
fn move_to(keys: &[String], cursor: &mut Option<String>, step: Step) {
    if keys.is_empty() {
        *cursor = None;
        return;
    }
    let last = keys.len() - 1;
    let index = match index_of(keys, cursor) {
        Some(index) => match step {
            Step::Up if index == 0 => last,
            Step::Up => index - 1,
            Step::Down if index == last => 0,
            Step::Down => index + 1,
            Step::Top => 0,
            Step::Bottom => last,
            Step::PageUp => index.saturating_sub(PAGE),
            Step::PageDown => (index + PAGE).min(last),
        },
        None => match step {
            Step::Bottom => last,
            _ => 0,
        },
    };
    *cursor = Some(keys[index].clone());
}

/// How many lines the checkpoint pane holds — the one PARTICIPANT pane
/// that is still a paragraph with an offset rather than a cursor.
fn stream_len(tui: &Tui, views: &Views) -> usize {
    seat_of(tui, views).map_or(0, |part| part.checkpoints.len())
}

/// The transcript pane's list: one key per turn, the turn's index in
/// the stream. Live prose streaming only APPENDS turns, so an index is
/// a stable key, and the cursor survives an appending refresh by the
/// same absence of code as every other list.
fn turn_keys(views: &Views) -> Vec<String> {
    let count = views
        .transcript
        .as_ref()
        .map_or(0, |(turns, _)| turns.len());
    (0..count).map(|index| index.to_string()).collect()
}

/// The transcript's live selection: a turn the cursor's index still
/// names. A stale index — a transcript that shrank — selects nothing,
/// exactly like a stale key anywhere else.
fn selected_turn<'a>(tui: &Tui, views: &'a Views) -> Option<(usize, &'a Turn)> {
    let (turns, _) = views.transcript.as_ref()?;
    let index = index_of(&turn_keys(views), &tui.turn)?;
    Some((index, &turns[index]))
}

/// The truncation notice, written once: the transcript pane shows it as
/// its last line and the reader repeats it as the reader's last line.
/// Silently short evidence is worse than none (decision 0001), so the
/// door that opens the WHOLE transcript must not be the surface that
/// quietly hides the cap.
const TRUNCATED_NOTICE: &str = "transcript truncated (size cap) — claude --resume carries the rest";

/// The whole turn, composed for the reader: a header naming the role
/// and the timestamp, then every block in order — prose in full, tool
/// blocks as the same `⚙ name · target` marker the console shows.
/// Every part is seat-authored, so every part passes through [`safe`].
fn turn_text(turn: &Turn) -> String {
    let mut text = format!("{}  {}\n", safe(&turn.role), safe(&turn.ts));
    for block in &turn.blocks {
        text.push('\n');
        if block.kind == "tool" {
            text.push_str("⚙ ");
        }
        text.push_str(safe(&block.text).as_str());
    }
    text
}

/// The WHOLE transcript, composed for the same reader: every turn in
/// stream order, each composed by [`turn_text`] itself — reused, never
/// re-derived, so the two doors cannot drift — a blank line between
/// turns, and the pane's own truncation notice as the final line when
/// the stream was capped. Every part is [`safe`] because `turn_text` is.
fn transcript_text(turns: &[Turn], truncated: bool) -> String {
    let mut parts: Vec<String> = turns
        .iter()
        // The separator owns the blank line, so a turn with no blocks
        // cannot smuggle a second one in on its header's newline.
        .map(|turn| turn_text(turn).trim_end_matches('\n').to_string())
        .collect();
    if truncated {
        parts.push(TRUNCATED_NOTICE.to_string());
    }
    parts.join("\n\n")
}

fn step(tui: &mut Tui, views: &Views, step: Step) {
    if tui.level == Level::Participant {
        // The transcript pane moves a cursor over TURNS — never over
        // wrapped lines — through the same `move_to` as every list.
        if tui.pane == 1 {
            let keys = turn_keys(views);
            move_to(&keys, &mut tui.turn, step);
            return;
        }
        // A paragraph pane's offset moves through the SAME function, so
        // wrap-around and paging have exactly one implementation.
        let keys: Vec<String> = (0..stream_len(tui, views))
            .map(|line| line.to_string())
            .collect();
        let mut cursor = Some(tui.offset.to_string());
        move_to(&keys, &mut cursor, step);
        tui.offset = cursor.and_then(|line| line.parse().ok()).unwrap_or(0);
        return;
    }
    let keys = keys_for(tui, views);
    let pane = tui.pane;
    move_to(&keys, &mut tui.cursor[pane], step);
    // On the graph, moving IS scoping — the console scopes on a click,
    // and an operator who has moved the rail cursor onto a phase has
    // said which phase they mean. Enter then descends; Esc clears.
    if in_graph(tui) {
        tui.scope = tui.cursor[0].clone().map(render::Scope::Phase);
    }
}

/// A graph that opens with nothing selected is a graph whose `Enter`
/// does nothing, which reads as a broken key rather than as an empty
/// selection. The rail cursor starts on the run's CURRENT phase — where
/// an operator is already looking — and only when it has none.
fn seed_cursor(tui: &mut Tui, views: &Views) {
    if tui.level != Level::Run || tui.cursor[0].is_some() {
        return;
    }
    let Some(view) = views.run.as_ref() else {
        return;
    };
    tui.cursor[0] = view
        .phases
        .iter()
        .find(|phase| phase.current)
        // A journal that folds to no status has no current phase; the
        // last phase entered is still where an operator is looking.
        .or_else(|| view.phases.last())
        .map(|phase| phase.name.clone());
}

/// The graph is the one pane whose primary axis is horizontal, so it is
/// the one pane where `←→` and `↑↓` do different things.
fn in_graph(tui: &Tui) -> bool {
    tui.level == Level::Run && tui.pane == 0
}

/// The selected phase's nodes in draw order — the lane cursor's list. A
/// plain phase has none, and `move_to` over an empty list sets `None`,
/// so `↑↓` there are inert by construction rather than by a special case.
fn lane_keys(tui: &Tui, views: &Views) -> Vec<String> {
    let Some(view) = views.run.as_ref() else {
        return Vec::new();
    };
    let Some(cursor) = tui.cursor[0].as_deref() else {
        return Vec::new();
    };
    view.phases
        .iter()
        .filter(|phase| phase.name == cursor)
        .flat_map(|phase| phase.columns.iter())
        .flat_map(|column| column.nodes.iter())
        .map(|node| node.key.clone())
        .collect()
}

/// The participant the lane cursor is standing on, resolved against the
/// **fresh** models like every other selection. A fork member's node key
/// is that member's `Participant.key` and a plain step's is its seat's,
/// so this is a lookup rather than a second mapping. A structural node —
/// a finished step nobody tagged — answers `None`, and so does a lane
/// cursor that is nowhere.
fn lane_member<'a>(tui: &Tui, views: &'a Views) -> Option<&'a Participant> {
    tui.node.as_deref().and_then(|key| participant(views, key))
}

/// What the graph's two cursors say the scope is, in one place. On the
/// rail, moving IS scoping (the standing law); in the lanes it is the
/// same law one level down — a member node scopes that seat, through the
/// same `Scope`/`lens_for` the seats pane's own `Enter` produces, never a
/// second filtering mechanism. Anything the lanes cannot resolve falls
/// back to exactly what the rail already set.
fn graph_scope(tui: &Tui, views: &Views) -> Option<render::Scope> {
    match lane_member(tui, views) {
        Some(part) => Some(render::Scope::Seat(part.key.clone())),
        None => tui.cursor[0].clone().map(render::Scope::Phase),
    }
}

/// `↑↓` walk the lanes inside the graph pane, and the focused list
/// everywhere else. Both go through the same `move_to`, so wrap-around
/// and the empty-list case are already specified and already tested.
fn arrow(tui: &mut Tui, views: &Views, direction: Step) {
    match in_graph(tui) {
        true => {
            let keys = lane_keys(tui, views);
            move_to(&keys, &mut tui.node, direction);
            // The lane cursor landed somewhere, so it has said what the
            // operator means: this seat, or — off the members — the
            // rail's phase again.
            tui.scope = graph_scope(tui, views);
        }
        false => step(tui, views, direction),
    }
}

/// `←→` walk the rail. Everywhere else they are a **named** no-op: the
/// lists in the other panes have no horizontal axis, and a wildcard here
/// would be an untested claim about what they do with an arrow key.
fn rail_move(tui: &mut Tui, views: &Views, direction: Step) {
    // The guard IS the naming: outside the graph pane `←→` change no
    // state at all, and a test presses them everywhere to prove it.
    if !in_graph(tui) {
        return;
    }
    let keys = keys_for(tui, views);
    move_to(&keys, &mut tui.cursor[0], direction);
    // The rail moved, so whatever lane the cursor was in is gone — and
    // with it any seat that lane had scoped. The phase under the rail is
    // what remains selected, which is what rail movement has always said.
    tui.node = None;
    tui.scope = graph_scope(tui, views);
}

/// The live selection: a cursor key still present in the current list.
/// A stale key descends into nothing.
fn selected(tui: &Tui, views: &Views) -> Option<String> {
    let keys = keys_for(tui, views);
    index_of(&keys, &tui.cursor[tui.pane]).map(|index| keys[index].clone())
}

// ---------------------------------------------------------- the ladder

/// `Enter` pushes one rung, `Esc` pops one. At the RUN level the rungs
/// are `unscoped → scoped → descended`, which is how "Enter descends"
/// and "selecting a phase or a participant scopes the run level" are
/// both honoured without inventing a key the ruling does not name.
fn enter(tui: &mut Tui, views: &Views) {
    if tui.typing {
        tui.typing = false;
        return;
    }
    match tui.level {
        // The checkpoint pane is a paragraph, not a door. The
        // transcript is a list of TURNS, and a long turn clamped by
        // its pane is not readable evidence — so Enter opens the
        // selected turn whole in the trail's own reader (the operator
        // re-ruled the paragraph contract). With NO turn selected the
        // same key opens the WHOLE transcript, because a stream
        // clamped by its pane is no more readable than one long turn
        // (the operator's ruling, superseding the inert pin). The
        // per-turn reader stays the drilldown; a pane holding no
        // transcript at all still holds no door.
        Level::Participant => {
            if tui.pane == 1 {
                match selected_turn(tui, views) {
                    Some((_, turn)) => {
                        tui.reading = Some(turn_text(turn));
                        tui.read_offset = 0;
                    }
                    None => {
                        if let Some((turns, truncated)) = views.transcript.as_ref() {
                            tui.reading = Some(transcript_text(turns, *truncated));
                            tui.read_offset = 0;
                        }
                    }
                }
            }
        }
        Level::Runs => {
            if let Some(key) = selected(tui, views) {
                tui.assign_run(key);
            }
        }
        Level::Run => {
            let Some(key) = selected(tui, views) else {
                return;
            };
            match tui.pane {
                0 => tui.scope = Some(render::Scope::Phase(key)),
                1 => {
                    if scoped_seat(tui) == Some(key.as_str()) {
                        tui.level = Level::Participant;
                        tui.seat = Some(key);
                        tui.pane = 0;
                        tui.offset = 0;
                        // A turn cursor belongs to the transcript it
                        // was moved over, never to the next seat's.
                        tui.turn = None;
                        tui.force = true;
                    } else {
                        tui.scope = Some(render::Scope::Seat(key));
                    }
                }
                // The trail is evidence — and evidence you cannot read is not
                // evidence, so Enter opens the row's full text rather than
                // descending.
                _ => {
                    let row = views
                        .run
                        .as_ref()
                        .and_then(|view| view.journal.iter().find(|row| row.seq.to_string() == key))
                        .expect("a selected trail key resolves against the same view");
                    tui.reading = Some(format!(
                        "seq {}  {}  {}\n\n{}\n\npayload\n{}",
                        row.seq,
                        safe(&row.event_type),
                        safe(&row.recorded_at),
                        safe(&row.what.text),
                        safe(&row.payload_json),
                    ));
                    tui.read_offset = 0;
                }
            }
        }
    }
}

/// A precedence ladder. **`Esc` never quits**, so a fat-fingered `Esc`
/// cannot kill the console.
fn escape(tui: &mut Tui) {
    if tui.help {
        tui.help = false;
        return;
    }
    if tui.typing || !tui.filter.is_empty() {
        tui.typing = false;
        tui.filter.clear();
        return;
    }
    if tui.level == Level::Participant {
        // The transcript pane's first rung clears the TURN selection.
        // It is what keeps the whole-transcript door reachable after a
        // turn has been read — one obvious key, not a hidden
        // combination (decision 0014's discoverability rule) — and the
        // ladder ascends on the next press, as it always did.
        if tui.pane == 1 && tui.turn.is_some() {
            tui.turn = None;
            return;
        }
        ascend(tui);
        return;
    }
    if tui.scope.is_some() {
        tui.scope = None;
        return;
    }
    ascend(tui);
}

/// Rungs 3 and 5 alone: ascending without ever clearing a scope. This is
/// `Backspace` outside filter mode, and the tail of the `Esc` ladder.
fn ascend(tui: &mut Tui) {
    // A filter belongs to the list it was typed over.
    tui.filter.clear();
    tui.typing = false;
    match tui.level {
        Level::Participant => {
            tui.level = Level::Run;
            tui.seat = None;
            // Land back on the seat you were reading: the symmetric pop.
            tui.pane = 1;
            tui.force = true;
        }
        Level::Run => {
            tui.level = Level::Runs;
            tui.pane = 0;
            tui.force = true;
        }
        Level::Runs => {}
    }
}

fn backspace(tui: &mut Tui) {
    if tui.typing {
        // What makes `/` incremental.
        tui.filter.pop();
        return;
    }
    ascend(tui);
}

/// A character key. Bindings are read **here**, not at the crossterm
/// boundary: while a filter is being typed, `q` is a letter.
fn typed(tui: &mut Tui, views: &Views, character: char) -> Flow {
    if tui.typing {
        // Operator input is sanitized like every other string: it is
        // echoed in the footer and a bracketed paste can carry an
        // escape sequence.
        tui.filter.push_str(safe(&character.to_string()).as_str());
        return Flow::Continue;
    }
    match character {
        'q' => return Flow::Quit,
        'j' => step(tui, views, Step::Down),
        'k' => step(tui, views, Step::Up),
        'g' => step(tui, views, Step::Top),
        'G' => step(tui, views, Step::Bottom),
        'r' => tui.force = true,
        '/' => tui.typing = true,
        '?' => tui.help = !tui.help,
        _ => {}
    }
    Flow::Continue
}

/// The pure state machine: view models plus a key, in; a flow, out. No
/// terminal, no store, no I/O.
pub(crate) fn apply(tui: &mut Tui, views: &Views, key: Key) -> Flow {
    // The reader owns movement while it is open: an operator scrolling
    // a long payload must not also be moving the list behind it.
    if tui.reading.is_some() {
        match key {
            Key::Quit => return Flow::Quit,
            Key::Char('q') => return Flow::Quit,
            Key::Escape | Key::Backspace | Key::Enter | Key::Char('?') => {
                tui.reading = None;
                tui.read_offset = 0;
            }
            Key::Down | Key::Char('j') => tui.read_offset = tui.read_offset.saturating_add(1),
            Key::Up | Key::Char('k') => tui.read_offset = tui.read_offset.saturating_sub(1),
            Key::PageDown => tui.read_offset = tui.read_offset.saturating_add(10),
            Key::PageUp => tui.read_offset = tui.read_offset.saturating_sub(10),
            Key::Char('g') => tui.read_offset = 0,
            _ => {}
        }
        return Flow::Continue;
    }
    match key {
        Key::Quit => return Flow::Quit,
        Key::Char(character) => return typed(tui, views, character),
        Key::Enter => enter(tui, views),
        Key::Escape => escape(tui),
        Key::Backspace => backspace(tui),
        Key::Tab => tui.pane = (tui.pane + 1) % panes_at(tui.level),
        Key::Up => arrow(tui, views, Step::Up),
        Key::Down => arrow(tui, views, Step::Down),
        Key::Left => rail_move(tui, views, Step::Up),
        Key::Right => rail_move(tui, views, Step::Down),
        Key::PageUp => step(tui, views, Step::PageUp),
        Key::PageDown => step(tui, views, Step::PageDown),
    }
    Flow::Continue
}

// ------------------------------------------------------ footer and help

/// Discoverability is a requirement, not a nicety: the footer names the
/// keys available in the CURRENT context, and differs per (level, pane,
/// typing, help) so a constant footer cannot pass its test.
pub(crate) fn footer_for(tui: &Tui, views: &Views) -> String {
    if tui.help {
        return "? or Esc close help · q quit".to_string();
    }
    if tui.reading.is_some() {
        return "↑↓/jk scroll · PgUp/PgDn page · g top · Esc or Enter close · q quit".to_string();
    }
    if tui.typing {
        let mut line = String::from("/");
        line.push_str(safe(&tui.filter).as_str());
        line.push_str("▏ filtering · Enter keep · Esc clear · ⌫ delete");
        return line;
    }
    let tail = "· / filter · r refresh · ? help · q quit";
    match (tui.level, tui.pane) {
        (Level::Runs, _) => format!("↑↓/jk move · Enter open run · g/G top/bottom {tail}"),
        // The lane cursor scopes, so the footer must say so where it
        // happens (decision 0014's discoverability rule): an operator
        // who watches the seats and the trail narrow under `↑↓` should
        // read WHY on the same line, named by the seat's own label —
        // what the seats pane displays — rather than by its raw key.
        // It says so only while that member IS the scope: `Enter` and
        // `j`/`k` both re-scope the phase and leave the lane cursor
        // standing, and a footer naming a seat the panes are not
        // filtered to would contradict the status line above it.
        (Level::Run, 0) => {
            let named = lane_member(tui, views)
                .filter(|part| scoped_seat(tui) == Some(part.key.as_str()));
            let lanes = match named {
                Some(part) => format!("↑↓ lanes · scoped to {}", safe(&part.label)),
                None => "↑↓ lanes".to_string(),
            };
            format!("←→ rail · {lanes} · Enter scope phase · Tab pane · Esc back {tail}")
        }
        (Level::Run, 1) => {
            let verb = match (scoped_seat(tui), tui.cursor[1].as_deref()) {
                (Some(scoped), Some(cursor)) if scoped == cursor => "Enter open seat",
                _ => "Enter scope seat",
            };
            format!("↑↓/jk move · {verb} · Tab pane · Esc back {tail}")
        }
        (Level::Run, 2) => format!("↑↓/jk move · Enter read row · Tab pane · Esc back {tail}"),
        (Level::Run, _) => format!("↑↓/jk move · Tab pane · Esc back {tail}"),
        // Two doors, so two footers: the key that opens the turn under
        // the cursor, and the key that opens the whole transcript —
        // each named where it is the one Enter does, with `Esc
        // unselect` naming the way back to the other.
        (Level::Participant, 1) => match tui.turn {
            Some(_) => {
                format!("↑↓/jk move · Enter read turn · Esc unselect · g/G top/bottom · Tab pane {tail}")
            }
            None => format!(
                "↑↓/jk move · Enter read whole transcript · g/G top/bottom · Tab pane · Esc back {tail}"
            ),
        },
        (Level::Participant, _) => {
            format!("↑↓/jk scroll · g/G top/bottom · Tab pane · Esc back {tail}")
        }
    }
}

/// The breadcrumb, or the sentence a transient store error earns. One
/// line, always drawn: an operator must be able to see WHY a frame is
/// standing still.
fn status_line(tui: &Tui) -> String {
    if let Some(status) = &tui.status {
        return status.clone();
    }
    let mut line = String::from("runs");
    if let Some(run) = &tui.run {
        line.push_str(" · run ");
        line.push_str(safe(run).as_str());
    }
    if let Some(seat) = &tui.seat {
        line.push_str(" · seat ");
        line.push_str(safe(seat).as_str());
    }
    match &tui.scope {
        Some(render::Scope::Phase(name)) => {
            line.push_str(" · phase ");
            line.push_str(safe(name).as_str());
        }
        Some(render::Scope::Seat(key)) => {
            line.push_str(" · scoped ");
            line.push_str(safe(key).as_str());
        }
        None => {}
    }
    line
}

const HELP: [&str; 13] = [
    "brokkr tui — a read-only console over the same models as",
    "brokkr inspect, brokkr watch and brokkr ui. It issues no",
    "operator commands and writes nothing to the journal.",
    "",
    "↑ ↓ j k     move          Enter   descend / scope",
    "← →         the graph rail        ↑ ↓ its lanes",
    "Esc         back          ⌫       back (keeps the scope)",
    "Tab         next pane     g G     top / bottom",
    "PgUp PgDn   page          /       filter this list",
    "r           refresh       ?       this help",
    "q Ctrl+C    quit",
    "",
    "Selecting a phase or a seat scopes the run level; Esc clears it.",
];

// -------------------------------------------------------------- the paint

/// The one sanitizer, reused: three surfaces, one `Safe`.
fn safe(text: &str) -> String {
    Safe::new(text).as_str().to_string()
}

/// One of the two constructors that reach a widget. Both take sanitized
/// text, so a hostile string cannot arrive at a buffer without a visible
/// `Safe` at the call site — and ratatui then measures exactly the text
/// it draws.
fn cell(text: &str, style: Style) -> Cell<'static> {
    Cell::from(span(text, style))
}

fn span(text: &str, style: Style) -> Span<'static> {
    Span::styled(safe(text), style)
}

fn line(text: &str, style: Style) -> Line<'static> {
    Line::from(span(text, style))
}

fn plain() -> Style {
    Style::new()
}

fn header_style() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

fn selected_style(selected: bool) -> Style {
    match selected {
        true => Style::new().add_modifier(Modifier::REVERSED),
        false => Style::new(),
    }
}

/// One classification, three renderings of it: `render::tone` is the
/// table, ANSI is `brokkr runs`' rendering of it, and this is the TUI's.
fn tone_style(status: &str) -> Style {
    match render::tone(status) {
        Tone::Good => Style::new().fg(Color::Green),
        Tone::Bad => Style::new().fg(Color::Red),
        Tone::Live => Style::new().add_modifier(Modifier::BOLD),
        Tone::Quiet => Style::new().add_modifier(Modifier::DIM),
    }
}

/// The focused pane is the one with a bright border — the only focus
/// affordance, and one that needs no legend.
/// The brand mark, right-aligned on a pane's top border — the
/// console's top-left logo, translated: its three rail nodes and the
/// wordmark, with the third node pulsing on the shared live ramp
/// whenever the fleet is forging (the favicon flip, in cells). Idle,
/// the third node stands as the calibrated dot; the wordmark itself
/// never animates.
///
/// The wordmark is BROKKR (decision 0019 ruling 1). The mark is the
/// whole of the lore the TUI is allowed to wear: ruling 6's law 4
/// keeps myth out of the machine's mouth, so nothing else here says it.
fn brand(fleet_live: bool, ticks: usize, animate: bool) -> Line<'static> {
    let third = match fleet_live {
        true => LIVE_RAMP[pulse(ticks, true, animate)],
        false => "⏺",
    };
    let tone = match fleet_live {
        true => Style::new().fg(Color::Green).add_modifier(Modifier::BOLD),
        false => Style::new().add_modifier(Modifier::DIM),
    };
    Line::from(vec![
        span("[ ", Style::new().add_modifier(Modifier::DIM)),
        span("∙ ∙ ", Style::new().fg(Color::Magenta)),
        span(third, tone),
        span(" BROKKR", Style::new().add_modifier(Modifier::BOLD)),
        span(" ]", Style::new().add_modifier(Modifier::DIM)),
    ])
}

fn pane(title: &str, focused: bool) -> Block<'static> {
    let border = match focused {
        true => Style::new().add_modifier(Modifier::BOLD),
        false => Style::new().add_modifier(Modifier::DIM),
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Line::from(span(title, border)))
}

pub(crate) fn draw(frame: &mut Frame, tui: &Tui, views: &Views) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        draw_too_small(frame, area);
        return;
    }
    let [body, status, footer] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    match (tui.level, views.run.as_ref(), seat_of(tui, views)) {
        (Level::Participant, _, Some(part)) => draw_participant(frame, body, tui, views, part),
        (Level::Run, Some(view), _) => draw_run(frame, body, tui, views, view),
        _ => draw_runs(frame, body, tui, views),
    }
    // The forging beacon — the terminal's answer to the console's
    // pulsing favicon. Whenever ANY run in the fleet is live it pulses
    // in the status line's right corner, at every level, so an
    // operator browsing an old run still knows the machine is at work.
    // It rides the fleet the shell already refreshes and the tick the
    // pulse already uses: no store read, no extra poll.
    // The corner 'forging' text retired when the brand mark landed on
    // the graph pane (operator's ruling): ONE forging signal, the
    // logo's pulsing rail node, visible at every level.
    frame.render_widget(Paragraph::new(line(&status_line(tui), plain())), status);
    frame.render_widget(
        Paragraph::new(line(
            &footer_for(tui, views),
            Style::new().add_modifier(Modifier::REVERSED),
        )),
        footer,
    );
    if let Some(text) = &tui.reading {
        draw_reader(frame, area, text, tui.read_offset);
    }
    if tui.help {
        draw_help(frame, area);
    }
}

/// A row opened for reading: the full text, wrapped and scrollable, over
/// the whole frame. Every line is already sanitized at the call site
/// that filled `reading`.
fn draw_reader(frame: &mut Frame, area: Rect, text: &str, offset: usize) {
    let lines: Vec<Line> = text.lines().map(|text| line(text, plain())).collect();
    // Clamp so scrolling past the end cannot leave an empty frame with
    // no way back.
    let last = lines.len().saturating_sub(1);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((u16::try_from(offset.min(last)).unwrap_or(u16::MAX), 0))
            .block(pane("row · Esc closes", true)),
        area,
    );
}

/// A terminal resized below the minimum draws this, and keeps its
/// session: an operator dragging a window edge does not lose their
/// place, and no frame is ever corrupted.
fn draw_too_small(frame: &mut Frame, area: Rect) {
    let lines = vec![
        line("this terminal is too small for brokkr tui", header_style()),
        line(
            "try `brokkr inspect --run <id>` or `brokkr watch --run <id>`,",
            plain(),
        ),
        line("or make the window bigger. q or Ctrl+C quits.", plain()),
    ];
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines), area);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = HELP.iter().map(|text| line(text, plain())).collect();
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines).block(pane("help", true)), area);
}

fn draw_runs(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views) {
    let keys = keys_for(tui, views);
    let cursor = tui.cursor[0].as_deref();
    let header = Row::new(
        ["id", "status", "phase", "seq", "age", "feature"]
            .iter()
            .map(|name| cell(name, header_style()))
            .collect::<Vec<Cell>>(),
    );
    let mut rows: Vec<Row> = Vec::new();
    for row in &views.runs.runs {
        if !keys.iter().any(|key| key == &row.run_id) {
            continue;
        }
        // Every cell is a model field; the absence marks are the
        // model's, and a run whose journal does not fold keeps its row.
        let status = match &row.status {
            Some(status) => status.clone(),
            None => "?".to_string(),
        };
        let phase = match &row.phase {
            Some(phase) => phase.clone(),
            None => "-".to_string(),
        };
        let seq = match row.seq {
            Some(seq) => seq.to_string(),
            None => "-".to_string(),
        };
        let age = match brokkr_view::age(&row.created_at, &views.now) {
            Some(age) => age,
            None => brokkr_view::ABSENT.to_string(),
        };
        rows.push(
            Row::new(vec![
                cell(&row.run_id, plain()),
                cell(&status, tone_style(&status)),
                cell(&phase, plain()),
                cell(&seq, plain()),
                cell(&age, plain()),
                cell(&row.feature, plain()),
            ])
            .style(selected_style(cursor == Some(row.run_id.as_str()))),
        );
    }
    let widths = [
        Constraint::Length(24),
        Constraint::Length(9),
        Constraint::Length(12),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Min(10),
    ];
    frame.render_widget(
        Table::new(rows, widths)
            .header(header)
            .block(pane("runs", true)),
        area,
    );
}

fn draw_run(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, view: &RunView) {
    let lens = lens_of(tui, views);
    let [graph, seats, trail] = Layout::vertical([
        Constraint::Percentage(34),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ])
    .areas(area);
    draw_graph(frame, graph, tui, views, view, lens.as_ref());
    draw_seats(frame, seats, tui, view, lens.as_ref());
    draw_trail(frame, trail, tui, view, lens.as_ref());
}

// --------------------------------------------------------------- the graph
//
// The console's grammar (`ui.html::renderLoops`), drawn in characters:
// ONE horizontal rail; sequential steps joined by arrowed edges that
// read as *then*; a panel FORKING into vertically symmetric lanes that
// REJOIN the rail before the next step — the rejoin *is* the join
// dependency, and it is the one thing the 0013 tree could not say;
// phase names on one shared baseline with `×N` revisit markers; and the
// active node pulsing while the run is live.
//
// **A character grid, and deliberately not ratatui's plotting surface.**
// Colour on that surface is stored per cell, last writer wins, so a node
// circle drawn over a rail recolours the rail's cells — which makes a
// fixed per-node colour vocabulary structurally unsatisfiable wherever a
// node shares a cell with an edge, and at terminal densities that is
// most nodes. Its own text call is a third path into a buffer whose
// apparent safety is borrowed from ratatui's undocumented filtering
// rather than from `Safe`'s enumerated table. And at five inner rows
// there is no sub-cell resolution to win: a `●` at one-cell scale is a
// truer circle than any dot-matrix approximation of one. What is
// conceded is stroke fidelity; the grammar being matched is topological
// — rail, arrow, fork, rejoin, baseline — and every element of it
// survives at cell resolution. The ruling is held by a test over this
// file's own source, not by memory.
//
// **Two functions, one boundary.** [`plan`] is pure owned integer
// geometry over the models and the rect; [`paint`] walks that plan and
// emits spans through the one sanitized constructor. The plan fits by
// construction, so the painter has no clipping branch — a painter that
// clips is a painter whose failure mode is invisible.

/// The arrowed edge between two steps: `──→`, the console's *then*.
const ARROW_WIDTH: usize = 3;

/// How the selection box breathes: this many columns from a wall to the
/// nearest glyph of the phase it holds, on EVERY side. Two, because one
/// is not breathing room and because a wall two columns out can never
/// land on the arrowhead that flies into the node — the head sits one
/// column off the rail content, inside the boundary where it belongs.
const BOX_PAD: usize = 2;

/// A label clamps here before anything structural gives way.
const LABEL_MAX: usize = 14;

/// A corrupt fold must not put twenty digits on the name baseline.
const VISITS_MAX: u64 = 99;

/// The node vocabulary: `ui.html`'s `NODE_CLASS` allowlist plus its
/// `phase.current × summary.status` branch, transliterated. Seven
/// classes — deliverable 2's six, plus the one fallback arm Rust obliges.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Class {
    Visited,
    Current,
    Park,
    Failed,
    Finished,
    Active,
    Unknown,
}

/// One classification, one rendering of it: a **(colour, marker ramp)**
/// pair per class. The ramp is the pulse's four frames; classes that do
/// not pulse repeat one glyph, so the frame index is inert for them and
/// costs no branch. `Active`'s still glyph differs from `Finished`'s in
/// the GLYPH channel, so the distinction survives a terminal with no
/// colour and an operator with no animation.
///
/// This is deliberately **not** `render::tone`: `tone` maps
/// `awaiting_operator` to `Quiet`, while the graph needs **park** as a
/// distinct yellow class, and widening `tone` would move `brokkr runs`'
/// colour. One classification per question, not one table for two.
/// Node glyphs are calibrated BY THE OPERATOR'S EYE, like the
/// arrowhead: `⏺` (U+23FA) is the filled node whose centre sits on the
/// dash axis in their font, and the math operators `⊗`/`⊙` share that
/// axis by design. The geometric shapes `●○◉◎` do not — they are the
/// reason this table exists. Every live class pulses on the SAME ramp,
/// so all live nodes breathe in phase — and the ramp STARTS on `∙`, so
/// a live node differs from a finished `⏺` in the glyph channel even
/// with colour off and animation frozen (the property the vocabulary
/// test pins).
const LIVE_RAMP: [&str; 4] = ["∙", "⏺", "∙", "·"];

fn look(class: Class) -> (Style, [&'static str; 4]) {
    match class {
        Class::Visited => (
            Style::new().fg(Color::Magenta).add_modifier(Modifier::DIM),
            ["⏺", "⏺", "⏺", "⏺"],
        ),
        Class::Current => (Style::new().fg(Color::Green), LIVE_RAMP),
        Class::Park => (Style::new().fg(Color::Yellow), ["⊙", "⊙", "⊙", "⊙"]),
        Class::Failed => (Style::new().fg(Color::Red), ["⊗", "⊗", "⊗", "⊗"]),
        Class::Finished => (Style::new().fg(Color::Green), ["⏺", "⏺", "⏺", "⏺"]),
        Class::Active => (
            Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            LIVE_RAMP,
        ),
        Class::Unknown => (
            Style::new().add_modifier(Modifier::DIM),
            ["·", "·", "·", "·"],
        ),
    }
}

/// A phase's own rail node. An unrecognised `summary.status` renders
/// **quiet, never live** — a deliberate, named divergence from
/// `ui.html`, which falls through to green. The terminal declines to
/// guess, in the manner of `render::tone`'s own `_ => Tone::Quiet`.
fn class_for_phase(current: bool, status: &str) -> Class {
    match (current, status) {
        (false, _) => Class::Visited,
        (true, "running" | "completed") => Class::Current,
        (true, "awaiting_operator") => Class::Park,
        (true, "stopped") => Class::Failed,
        (true, _) => Class::Unknown,
    }
}

/// A node inside a phase, from the model's closed-set key. **No journal
/// string reaches this table**, and an unlisted key is the quiet arm.
fn class_for_node(state_class: &str) -> Class {
    match state_class {
        "on-phosphor" => Class::Finished,
        "in-active" => Class::Active,
        "on-park" => Class::Park,
        "on-halt" => Class::Failed,
        _ => Class::Unknown,
    }
}

/// The pulse: a pure, total function of a tick counter and two model
/// facts. It reads no store — there is nothing here to read *from* — and
/// it moves the GLYPH, never the colour and never a position, so a live
/// node can never look briefly parked and geometry never varies with
/// `tick`. `!live` or `!animate` is frame 0, the still frame, at every
/// tick, so an idle console costs exactly nothing extra.
fn pulse(tick: usize, live: bool, animate: bool) -> usize {
    match live && animate {
        true => (tick / PULSE_TICKS) % PULSE_FRAMES,
        false => 0,
    }
}

/// ratatui's own measurement of the sanitized text — which is what the
/// buffer will actually draw. `Safe::width()` is a `char` count and
/// reports 6 for `設計フェーズ` where the terminal draws 12; the rail is
/// the first pane here that places its own x positions, so it is the
/// first that can be lied to. **Every width in the graph comes from
/// here**, so there is no second measurement to disagree with the first.
fn width_of(text: &str) -> usize {
    span(text, plain()).width()
}

/// Clamp to a display width, marking the cut with `…`. Text is what
/// gives way when a segment is tight; the rail, the arrows, the fork
/// corners and the rejoin never are.
fn clamp(text: &str, max: usize) -> String {
    let text = safe(text);
    // TWO bounds, deliberately. Display width is what the layout plans
    // in, but the buffer consumes a cell per `char`, and a zero-width
    // char (a combining mark, a variation selector) costs 0 columns and
    // 1 cell. Bounding on width alone let a label of N such marks plan
    // one column and overwrite N — erasing the rail, the arrows and a
    // neighbour's state glyph. Bounding on the char count too is
    // structural: it holds for every zero-width class, present and
    // future, where enumerating them would not.
    if width_of(&text) <= max && text.chars().count() <= max {
        return text;
    }
    if max == 0 {
        return String::new();
    }
    let mut out = String::new();
    for character in text.chars() {
        let mut wider = out.clone();
        wider.push(character);
        if width_of(&wider) + 1 > max || wider.chars().count() + 1 > max {
            break;
        }
        out = wider;
    }
    out.push('…');
    out
}

/// `×N` only when the phase was revisited — the console's rule, not
/// today's unconditional `×1` — and clamped, because `visits` is a `u64`
/// crossing `VIEW_VERSION` rather than a promise.
fn visits_text(visits: u64) -> Option<String> {
    match visits {
        0 | 1 => None,
        2..=VISITS_MAX => Some(format!("×{visits}")),
        _ => Some(format!("×{VISITS_MAX}+")),
    }
}

/// The name baseline's text: the scope marker the crate's ONE phase
/// predicate decides — called, never reimplemented — then the name,
/// then the revisit marker.
fn name_text(phase: &Phase, lens: Option<&render::Lens>) -> String {
    let mut text = String::new();
    if lens.is_some() && render::keeps_phase(lens, phase) {
        text.push('▸');
    }
    text.push_str(&phase.name);
    if let Some(visits) = visits_text(phase.visits) {
        text.push(' ');
        text.push_str(&visits);
    }
    text
}

/// What a column drawn as one node is called: the step's own name when
/// the model gave it one, otherwise the node's, and nothing at all for a
/// column the derivation left empty.
fn column_label(column: &Column) -> &str {
    match (&column.label, column.nodes.first()) {
        (Some(label), _) => label,
        (None, Some(node)) => &node.label,
        (None, None) => "",
    }
}

/// A label's footprint beside its node: one space and the text, or
/// nothing at all when there is no text.
fn label_span(label: &str) -> usize {
    match label.is_empty() {
        true => 0,
        false => 1 + width_of(label),
    }
}

/// Which member speaks for a column drawn as a single node: the worst
/// state wins, so a compacted fork never reads healthier than its
/// members do, and a still-working member outranks a finished one.
fn worst(nodes: &[Node]) -> usize {
    let rank = |node: &Node| match node.state_class.as_str() {
        "on-halt" => 0,
        "on-park" => 1,
        "in-active" => 3,
        "on-phosphor" => 4,
        _ => 2,
    };
    nodes
        .iter()
        .enumerate()
        .min_by_key(|(_, node)| rank(node))
        .map_or(0, |(index, _)| index)
}

/// Where member `k` of `n` sits, as a row offset from the rail:
/// symmetric about it, and for an even count the rail row itself is left
/// to the rail. The lane span a fork needs is therefore `n / 2`.
fn lane_offset(k: usize, n: usize) -> isize {
    let half = (n / 2) as isize;
    let raw = k as isize - half;
    match n % 2 == 1 || raw < 0 {
        true => raw,
        false => raw + 1,
    }
}

/// Degradation's vertical axis. Lanes are vertical, and at `MIN_HEIGHT`
/// the graph pane's inner rect is about one row — no fork fits at any
/// terminal `refuse()` admits. Three named modes and one predicate;
/// every mode that cannot draw lanes still says `⑂n`, so the small forms
/// are compact rather than a lie about the shape of the run.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Mode {
    /// Rail, arrows, symmetric lanes, rejoin and the name baseline.
    Full,
    /// Rail, arrows and names; every fork collapses to one rail node
    /// bearing `⑂n` and the worst member's state.
    Rail,
    /// One row: one node per phase, its name and its `⑂n`, arrowed.
    Compressed,
}

fn mode_for(rows: usize, lane_span: usize) -> Mode {
    if rows < 2 {
        return Mode::Compressed;
    }
    if rows < 4 || lane_span == 0 {
        return Mode::Rail;
    }
    Mode::Full
}

/// A node on the rail or in a lane.
#[derive(Clone, PartialEq, Debug)]
struct Mark {
    x: usize,
    row: usize,
    class: Class,
    /// The pulse gate, read from a model field — `summary.status` for a
    /// phase's own node, `Node.state` for one inside it.
    live: bool,
    label: String,
    selected: bool,
}

/// A fork: the lanes leave the rail at `x0` and rejoin it at `x1`.
#[derive(Clone, PartialEq, Debug)]
struct Join {
    x0: usize,
    x1: usize,
    /// The rows the lanes run on, in draw order.
    rows: Vec<usize>,
    /// True when the member count is odd and one member rides the rail
    /// row itself — the `┼` case.
    on_rail: bool,
    /// The step's own name, centred on the rail row when no member is
    /// there to occupy it.
    label: Option<String>,
}

/// A return arc: the road back, under the name baseline, from beneath
/// the phase whose ruling sent the run back to beneath the phase it
/// landed in. Absolute plan-level columns, like `Plan.rail` and
/// `Plan.edges` and unlike a `Join` — an arc spans from one segment's
/// rail to another's, with whole phases possibly between them.
#[derive(Clone, PartialEq, Debug)]
struct Arc {
    /// The landing column, where the mirror head points: beneath the
    /// target phase's own rail content.
    to: usize,
    /// The departing column, beneath the source phase's.
    from: usize,
}

/// One phase: a span of the rail, the marks and forks on it, and the
/// name it wears on the shared baseline.
#[derive(Clone, PartialEq, Debug)]
struct Seg {
    /// `Phase.name` — the cursor key, and the reason `brokkr-view` pins
    /// name uniqueness with a test of its own.
    key: String,
    /// The inclusive span of this phase's own rail content — where the
    /// connector from the phase before lands, and where the one to the
    /// phase after leaves from.
    rail: (usize, usize),
    /// The inclusive span of everything the phase DRAWS: its rail
    /// extent and its name, whichever reaches further. A name wider
    /// than the rail hangs off its node into the gaps, so this is a
    /// union and not a box the name widened.
    x0: usize,
    x1: usize,
    /// Already clamped, and already carrying its `▸` and its `×N`.
    name: String,
    name_x: usize,
    /// `Some` only for the CURRENT phase, which takes the run's own
    /// colour on the baseline exactly as `ui.html` does — so the current
    /// phase is distinguished even where it has no single rail node of
    /// its own to fill. Selection is a different channel entirely.
    class: Option<Class>,
    selected: bool,
    marks: Vec<Mark>,
    joins: Vec<Join>,
}

/// One frame's geometry: owned integers, no borrow of a model, nothing
/// that varies with `tick`.
#[derive(Clone, PartialEq, Debug)]
struct Plan {
    mode: Mode,
    width: usize,
    rows: usize,
    rail_row: usize,
    name_row: usize,
    /// The inclusive span of the rail line, absent when nothing is drawn.
    rail: Option<(usize, usize)>,
    /// The reserved row for the selection box's lower edge.
    box_row: Option<usize>,
    /// The reserved row the return arcs run on, under everything else.
    /// Absent when the journal recorded no return, and absent when the
    /// pane cannot hold the row — half an arc is the box's own lesson.
    arc_row: Option<usize>,
    /// The roads back, at most one per `(from, to)` pair however many
    /// times it was taken: repeats ride the `×N` marker.
    arcs: Vec<Arc>,
    /// Where the `→` heads sit.
    edges: Vec<usize>,
    segments: Vec<Seg>,
    left_elided: bool,
    right_elided: bool,
}

/// The facts a column placement needs that are not the column itself.
struct Ink<'a> {
    status: &'a str,
    node: Option<&'a str>,
    rail_row: usize,
    lane_span: usize,
    lanes: bool,
    budget: usize,
}

/// One phase, laid out relative to its own rail start.
struct Built {
    rail_width: usize,
    /// How far the name spills past its own rail content, left and
    /// right. The name centres on the rail's centre and is NEVER moved
    /// off it; what a wide name claims it claims from the gaps beside
    /// the phase, not as padding inside the rail.
    lead: usize,
    trail: usize,
    name: String,
    /// Where the name starts, from the segment's leftmost drawn column.
    name_off: usize,
    marks: Vec<Mark>,
    joins: Vec<Join>,
    edges: Vec<usize>,
    /// The phase continues past the pane's edge.
    truncated: bool,
}

/// Where a name sits against the rail content it names. The name's own
/// start is the rail's centre less half the name, so its overhang is
/// that subtraction's shortfall on the left (`lead`) and its remainder
/// past the rail's last column on the right (`trail`); `off` is the
/// same placement measured from the segment's leftmost drawn column,
/// which is where the layout finally needs it. One computation, three
/// readers: the segment's drawn extent, the connector, and the box.
struct Named {
    lead: usize,
    trail: usize,
    off: usize,
}

fn name_place(rail_width: usize, name_width: usize) -> Named {
    let centre = rail_width.saturating_sub(1) / 2;
    let half = name_width / 2;
    let lead = half.saturating_sub(centre);
    // The name's last column from the rail's start: its centre plus the
    // half that rounds up. An empty name ends where it begins and
    // overhangs nothing, which needs no branch of its own.
    let end = centre + (name_width - half).saturating_sub(1);
    Named {
        lead,
        trail: end.saturating_sub(rail_width.saturating_sub(1)),
        off: centre + lead - half,
    }
}

/// One column, laid out from `x`. Placement and measurement are the same
/// code, so the two cannot disagree about where the next column starts.
fn place_column(
    column: &Column,
    x: usize,
    ink: &Ink,
    marks: &mut Vec<Mark>,
    joins: &mut Vec<Join>,
) -> usize {
    let members = column.nodes.len();
    if !ink.lanes || members < 2 {
        // One node on the rail with its label beside it — and, where a
        // parallel column could not be drawn as lanes, the `⑂n` that
        // keeps it reading as parallel rather than as a sequential step.
        let label = match members > 1 {
            true => format!("⑂{members}"),
            false => clamp(column_label(column), LABEL_MAX),
        };
        let chosen = column.nodes.get(worst(&column.nodes));
        marks.push(Mark {
            x,
            row: ink.rail_row,
            class: chosen.map_or(Class::Unknown, |node| class_for_node(&node.state_class)),
            live: chosen.is_some_and(|node| node.state == "active" && ink.status == "running"),
            selected: chosen.is_some_and(|node| ink.node == Some(node.key.as_str())),
            label: label.clone(),
        });
        return 1 + label_span(&label);
    }
    // A fork: it leaves the rail at `x` and REJOINS it, symmetric about
    // the rail, member `k` of `n` at row offset `lane_offset(k, n)`.
    let mut rows = Vec::new();
    let mut lanes: Vec<Mark> = Vec::new();
    let mut on_rail = false;
    let mut overflow = 0usize;
    for (k, member) in column.nodes.iter().enumerate() {
        let offset = lane_offset(k, members);
        if offset.unsigned_abs() > ink.lane_span {
            overflow += 1;
            continue;
        }
        on_rail = on_rail || offset == 0;
        let row = ink.rail_row.saturating_add_signed(offset);
        rows.push(row);
        lanes.push(Mark {
            x: x + 2,
            row,
            class: class_for_node(&member.state_class),
            live: member.state == "active" && ink.status == "running",
            label: clamp(&member.label, LABEL_MAX),
            selected: ink.node == Some(member.key.as_str()),
        });
    }
    // The members the lane budget could not hold are counted on the
    // outermost lane that was drawn: honest, never silently dropped.
    for lane in lanes.iter_mut().rev().take(overflow.min(1)) {
        lane.label.push_str(&format!(" +{overflow}"));
    }
    let mut inner = 0usize;
    for lane in &lanes {
        inner = inner.max(1 + label_span(&lane.label));
    }
    let label = match (on_rail, &column.label) {
        (false, Some(text)) => Some(clamp(text, LABEL_MAX)),
        _ => None,
    };
    if let Some(text) = &label {
        inner = inner.max(width_of(text));
    }
    marks.append(&mut lanes);
    joins.push(Join {
        x0: x,
        x1: x + inner + 3,
        rows,
        on_rail,
        label,
    });
    inner + 4
}

/// One phase's rail content and its name, relative to its own origin.
fn build(phase: &Phase, lens: Option<&render::Lens>, ink: &Ink) -> Built {
    let mut marks = Vec::new();
    let mut joins = Vec::new();
    let mut edges = Vec::new();
    let mut truncated = false;
    let mut x = 0usize;
    match phase.columns.is_empty() {
        // A plain phase is one node on the rail: the console's r=7 for
        // the current phase against r=5.5 for a visited one, expressed
        // in the only axis a cell has.
        true => {
            marks.push(Mark {
                x: 0,
                row: ink.rail_row,
                class: class_for_phase(phase.current, ink.status),
                live: phase.current && ink.status == "running",
                label: String::new(),
                selected: false,
            });
            x = 1;
        }
        false => {
            for (index, column) in phase.columns.iter().enumerate() {
                let gap = ARROW_WIDTH * usize::from(index > 0);
                let mut column_marks = Vec::new();
                let mut column_joins = Vec::new();
                let width =
                    place_column(column, x + gap, ink, &mut column_marks, &mut column_joins);
                if index > 0 && x + gap + width > ink.budget {
                    // The rest of this phase continues past the pane's
                    // edge, and `›` says so.
                    truncated = true;
                    break;
                }
                if let Some(origin) = (index > 0).then_some(x + gap) {
                    edges.push(origin - 1);
                }
                marks.append(&mut column_marks);
                joins.append(&mut column_joins);
                x += gap + width;
            }
        }
    }
    let name = clamp(&name_text(phase, lens), ink.budget);
    // A wide name no longer widens its segment. It used to, and the
    // rail filled the padding with dashes indistinguishable from the
    // gap, so the connector after a wide label read `────ᐳ` where its
    // neighbours read `──ᐳ` — an accident of arithmetic on the one
    // line whose rhythm is the grammar. The name now hangs off its own
    // node and the frame carries ONE connector length (`connector_of`),
    // so the dash run between any two phases is the same. The name is
    // still centred on its node, which is what the earlier attempt at
    // this got wrong: it clamped names into segment boxes and cascaded
    // dense ones away from the nodes they name.
    let place = name_place(x, width_of(&name));
    Built {
        rail_width: x,
        lead: place.lead,
        trail: place.trail,
        name,
        name_off: place.off,
        marks,
        joins,
        edges,
        truncated,
    }
}

/// `Compressed`: one node per phase, wearing the phase's own name and
/// the `⑂n` of its widest parallel column. Today's information on one
/// line — never a blank pane and never a refusal, because losing the
/// graph while dragging a window edge is worse than a compact one.
fn compressed(phase: &Phase, lens: Option<&render::Lens>, ink: &Ink) -> Built {
    let mut label = name_text(phase, lens);
    let widest = phase
        .columns
        .iter()
        .map(|column| column.nodes.len())
        .filter(|count| *count > 1)
        .max();
    if let Some(count) = widest {
        label.push_str(&format!(" ⑂{count}"));
    }
    let label = clamp(&label, ink.budget.saturating_sub(2));
    let width = 1 + label_span(&label);
    Built {
        rail_width: width,
        lead: 0,
        trail: 0,
        name: String::new(),
        name_off: 0,
        marks: vec![Mark {
            x: 0,
            row: ink.rail_row,
            class: class_for_phase(phase.current, ink.status),
            live: phase.current && ink.status == "running",
            label,
            selected: false,
        }],
        joins: Vec::new(),
        edges: Vec::new(),
        truncated: false,
    }
}

/// The window is derived from the cursor **every frame** — there is no
/// scroll-offset field, so there is no second piece of state that can
/// desynchronise from the selection.
fn anchor_of(phases: &[Phase], cursor: Option<&str>) -> usize {
    phases
        .iter()
        .position(|phase| Some(phase.name.as_str()) == cursor)
        .or_else(|| phases.iter().position(|phase| phase.current))
        .unwrap_or(0)
}

/// ONE connector length for the whole frame: the arrow, plus whatever
/// the names on either side of the hungriest gap claim from it. Every
/// pair then joins with the same dash run, which is the operator's
/// ruling — a connector whose length varies with a neighbour's label
/// reads as arithmetic, not as rhythm.
fn connector_of(built: &[Built]) -> usize {
    built
        .windows(2)
        .map(|pair| pair[0].trail + pair[1].lead + ARROW_WIDTH)
        .max()
        .unwrap_or(ARROW_WIDTH)
}

/// The columns a run of segments claims: the rail extents, one
/// connector between each pair, and the overhang the outermost names
/// carry past the rail at either end. Interior overhang is already paid
/// for by the connector, which is what makes this a sum and not a walk.
fn span_of(built: &[Built], start: usize, end: usize, connector: usize) -> usize {
    built[start..=end]
        .iter()
        .map(|item| item.rail_width)
        .sum::<usize>()
        + connector * (end - start)
        + built[start].lead
        + built[end].trail
}

/// Every return the rail could draw, as `(landing, departure)` pairs of
/// PHASE INDICES. The pairs themselves are `Phase.returns` — derived
/// once in `brokkr-view` from the transition that caused the revisit —
/// so nothing here reads a journal, infers backwardness from `visits`,
/// or names a phase.
///
/// Two pairs are dropped, both for want of geometry and neither as a
/// judgement about the journal: a departure naming no phase on this
/// rail has no column to leave from, and a landing sitting LATER on the
/// rail than its departure is not a road drawn leftward — the arc's
/// head has one direction, and the rail's own `ᐳ` is not the arc's to
/// borrow.
fn returns_of(phases: &[Phase]) -> Vec<(usize, usize)> {
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for (to, phase) in phases.iter().enumerate() {
        for source in &phase.returns {
            match phases.iter().position(|other| other.name == *source) {
                Some(from) if from > to => pairs.push((to, from)),
                _ => {}
            }
        }
    }
    pairs
}

/// Where a phase's road meets the arc row: the centre of its rail
/// content, so an arc's end sits under the phase's own node rather than
/// under the gap beside it. Two segments' rails are disjoint and a
/// connector apart, so two ends are never the same column and never
/// closer than the head plus its corner.
fn centre_of(seg: &Seg) -> usize {
    (seg.rail.0 + seg.rail.1) / 2
}

/// The visible run of segments: the anchor, then grow right, then left,
/// while the budget holds. The console's answer to width was horizontal
/// scrolling; ours is this window, walked by the arrow keys.
fn window(built: &[Built], anchor: usize, connector: usize, budget: usize) -> (usize, usize) {
    if built.is_empty() {
        return (0, 0);
    }
    let mut start = anchor;
    let mut end = anchor;
    loop {
        if end + 1 < built.len() && span_of(built, start, end + 1, connector) <= budget {
            end += 1;
            continue;
        }
        if start > 0 && span_of(built, start - 1, end, connector) <= budget {
            start -= 1;
            continue;
        }
        return (start, end + 1);
    }
}

/// The pure geometry: models and a rect in, owned integers out. It calls
/// `render::keeps_phase` rather than reimplementing the scope predicate,
/// lists **every** phase whether scoped or not — the lens marks, it does
/// not hide, here — and returns a layout that fits by construction.
#[allow(clippy::too_many_arguments)]
fn plan(
    phases: &[Phase],
    lens: Option<&render::Lens>,
    status: &str,
    cursor: Option<&str>,
    node: Option<&str>,
    width: usize,
    height: usize,
) -> Plan {
    let needed = phases
        .iter()
        .flat_map(|phase| phase.columns.iter())
        .map(|column| column.nodes.len() / 2)
        .max()
        .unwrap_or(0);
    let mode = mode_for(height, needed);
    // Row allocation is fixed and ordered: the name baseline is the last
    // row, the rail sits one row above the deepest lane it needs, and
    // the lanes spread symmetrically outward from the rail. Anything
    // left over is headroom, so the names stay under their own graph.
    // One row under the names is reserved for the selection box's
    // lower edge, so the box the operator draws around a phase can be
    // symmetric: an upper edge with no lower edge is two floating
    // lines, not a boundary. Reserved only when there is room.
    //
    // Under THAT, one row for the roads back — the last row of the
    // pane, so an arc passes beneath the box rather than through it and
    // the two can never meet by arithmetic. The row exists only for a
    // run whose journal recorded a return (`returns_of` is a fact about
    // the model, not about the layout, so it is safe to ask before the
    // rows are dealt) and only where the pane can hold it: a pane too
    // short omits the arc WHOLE, which is the box's own ruling — half
    // an arc is worse than none.
    let pairs = returns_of(phases);
    let arc_row = (!pairs.is_empty() && height >= 5).then(|| height - 1);
    let box_row = (height >= 4).then(|| height - 1 - usize::from(arc_row.is_some()));
    let name_row =
        height.saturating_sub(1 + usize::from(box_row.is_some()) + usize::from(arc_row.is_some()));
    let lane_span = match mode {
        Mode::Full => needed.min(name_row.saturating_sub(1) / 2),
        _ => 0,
    };
    let ink = Ink {
        status,
        node,
        rail_row: match mode {
            Mode::Compressed => 0,
            _ => name_row.saturating_sub(1 + lane_span),
        },
        lane_span,
        lanes: mode == Mode::Full,
        // Column 0 and the last column are the elision marks' own, and
        // one `BOX_PAD` inside each of those is the selection box's
        // walls — reserved like the box ROW is, so the box breathes the
        // same at the frame's edge as it does in the middle of it.
        budget: width.saturating_sub(2 * (1 + BOX_PAD)),
    };
    let built: Vec<Built> = phases
        .iter()
        .map(|phase| match mode {
            Mode::Compressed => compressed(phase, lens, &ink),
            _ => build(phase, lens, &ink),
        })
        .collect();
    let connector = connector_of(&built);
    let (start, end) = window(&built, anchor_of(phases, cursor), connector, ink.budget);

    let mut segments: Vec<Seg> = Vec::new();
    let mut edges: Vec<usize> = Vec::new();
    let mut rail: Option<(usize, usize)> = None;
    // The first rail node stands far enough in that the leftmost thing
    // the frame draws — the leading name's overhang — clears the
    // elision column and the box's own wall.
    let lead = built[start..end].first().map_or(0, |item| item.lead);
    let mut rail_x = 1 + BOX_PAD + lead;
    for (index, item) in built[start..end].iter().enumerate() {
        let phase = &phases[start + index];
        // Every gap is the same `connector` columns wide, arrowhead
        // last — the rail's rhythm does not depend on who its
        // neighbours are.
        if let Some((_, previous)) = rail {
            rail_x = previous + 1 + connector;
            edges.push(rail_x - 1);
        }
        edges.extend(item.edges.iter().map(|edge| edge + rail_x));
        let last = rail_x + item.rail_width - 1;
        rail = Some(match rail {
            Some((first, _)) => (first, last),
            None => (rail_x, last),
        });
        // `x0`/`x1` are everything the segment DRAWS: its rail extent
        // and the name hanging off its node, whichever reaches further.
        // The box pads that union, so it breathes evenly by
        // construction rather than by a clamp that could collapse.
        let x0 = rail_x - item.lead;
        segments.push(Seg {
            key: phase.name.clone(),
            rail: (rail_x, last),
            x0,
            x1: last + item.trail,
            name: item.name.clone(),
            // Centred on the rail content, and nothing moves it off:
            // the overhang IS the offset, so the label and its own node
            // cannot round apart.
            name_x: x0 + item.name_off,
            class: phase
                .current
                .then(|| class_for_phase(phase.current, status)),
            selected: cursor == Some(phase.name.as_str()),
            marks: item
                .marks
                .iter()
                .map(|mark| Mark {
                    x: mark.x + rail_x,
                    ..mark.clone()
                })
                .collect(),
            joins: item
                .joins
                .iter()
                .map(|join| Join {
                    x0: join.x0 + rail_x,
                    x1: join.x1 + rail_x,
                    ..join.clone()
                })
                .collect(),
        });
    }
    // Both ends or neither. A pair reaching a phase the window scrolled
    // away would have to land on the elision mark's own column, so it is
    // not drawn at all — the arithmetic is the window's own bounds, and
    // the painter is told nothing. The ROW stays reserved either way, so
    // walking the rail never lifts the baseline under the operator's eye.
    let arcs: Vec<Arc> = match arc_row {
        None => Vec::new(),
        Some(_) => pairs
            .iter()
            .filter(|(to, from)| *to >= start && *from < end)
            .map(|(to, from)| Arc {
                to: centre_of(&segments[to - start]),
                from: centre_of(&segments[from - start]),
            })
            .collect(),
    };
    Plan {
        mode,
        width,
        rows: height,
        rail_row: ink.rail_row,
        name_row,
        rail,
        box_row,
        arc_row,
        arcs,
        edges,
        segments,
        left_elided: start > 0,
        right_elided: end < phases.len() || built[start..end].iter().any(|item| item.truncated),
    }
}

/// One frame's cells. A cell holding `None` is the second column of a
/// double-width glyph, already paid for by its neighbour — which is why
/// every width here is ratatui's own measurement rather than a count.
type Cells = Vec<Vec<Option<(String, Style)>>>;

/// Write sanitized text at a cell position. Bounded by the grid itself
/// rather than by a branch: the plan fits, and the zip is what makes
/// that structural instead of asserted.
fn put(cells: &mut Cells, x: usize, row: usize, text: &str, style: Style) {
    let glyphs: Vec<Option<(String, Style)>> = safe(text)
        .chars()
        .flat_map(|character| {
            let glyph = character.to_string();
            let wide = width_of(&glyph) > 1;
            let mut pair = vec![Some((glyph, style))];
            pair.resize(1 + usize::from(wide), None);
            pair
        })
        .collect();
    for line in cells.iter_mut().skip(row).take(1) {
        for (cell, glyph) in line.iter_mut().skip(x).zip(glyphs.iter()) {
            cell.clone_from(glyph);
        }
    }
}

/// What is already painted at a cell — bounded by the grid the same way
/// `put` is, by iteration rather than by a branch, so an off-grid read
/// answers with the empty string instead of a panic or an `Option` the
/// caller would have to unwrap.
fn under(cells: &Cells, x: usize, row: usize) -> String {
    cells
        .iter()
        .skip(row)
        .take(1)
        .flat_map(|line| line.iter().skip(x).take(1))
        .flatten()
        .map(|(glyph, _)| glyph.as_str())
        .collect()
}

/// The spine at the fork and at the rejoin, drawn top to bottom in one
/// pass so no glyph can overwrite another's arms: a corner at the
/// outermost lane, the tee the rail wears at the trunk, and a tee for
/// every lane in between.
fn spine(cells: &mut Cells, join: &Join, rail_row: usize) {
    let up = join
        .rows
        .iter()
        .copied()
        .min()
        .unwrap_or(rail_row)
        .min(rail_row);
    let down = join
        .rows
        .iter()
        .copied()
        .max()
        .unwrap_or(rail_row)
        .max(rail_row);
    for row in up..=down {
        let (left, right) = match (row == up, row == down, row == rail_row, join.on_rail) {
            (true, _, _, _) => ("┌", "┐"),
            (_, true, _, _) => ("└", "┘"),
            (_, _, true, true) => ("┼", "┼"),
            (_, _, true, false) => ("┤", "├"),
            _ => ("├", "┤"),
        };
        put(cells, join.x0, row, left, plain());
        put(cells, join.x1, row, right, plain());
    }
}

/// The SOLID box around the selected phase — the terminal's answer to
/// the console's selection ring. The vocabulary is `─ │ ╭ ╮ ╰ ╯ ┼` and
/// nothing else: the dashed set `╌ ┆` left the graph by the operator's
/// ruling of 2026-08-31, because those glyphs are drawn with a gap at
/// every cell boundary BY DESIGN, so no amount of correct geometry can
/// make their edges touch the corners in any font. The geometry was
/// never the fault; the vocabulary was.
///
/// Lower edge on the reserved box row, walls on EVERY row between the
/// corners — and where a wall crosses the rail, the cell is `┼`, a true
/// junction, so the wall and the rail both read continuous through it.
/// That satisfies the unbroken-rail ruling rather than fighting it. A
/// pane too short to reserve the box row draws none: half a box is two
/// floating lines, which is what this replaced.
fn selection_box(cells: &mut Cells, seg: &Seg, plan: &Plan) {
    let Some(bottom) = plan.box_row else {
        return;
    };
    // The box spans the lane envelope OF THIS PHASE — one row above the
    // topmost row the phase actually occupies. A plain phase gets a
    // snug box with no empty `│    │` air rows; a fork's grows to hold
    // its own lanes. The shape is a function of the selected phase,
    // never of the pane, so moving the selection between two plain
    // phases cannot change it (operator's ruling, superseding the fixed
    // full-envelope box that made every plain phase wear a fork's).
    let top = seg
        .marks
        .iter()
        .map(|mark| mark.row)
        .chain(seg.joins.iter().flat_map(|join| join.rows.iter().copied()))
        .min()
        .unwrap_or(plan.rail_row)
        .saturating_sub(1);
    // `x0`/`x1` are already everything the segment draws, so padding
    // them by the same `BOX_PAD` is even breathing BY CONSTRUCTION: a
    // name can never sit flush against one wall while the other floats.
    // Two columns out also puts every wall clear of the arrowheads —
    // the head that lands on this phase's node is one column off its
    // rail content, and so stays inside the boundary it points into.
    // A wall does still CROSS the rail's dashes at the rail row, and
    // deliberately: the rail is one unbroken line by an earlier ruling,
    // so the only alternative is a hole in the track at every
    // selection. Crossing a line is a boundary; standing on an arrow's
    // point was the accident.
    let (left, right) = (seg.x0.saturating_sub(BOX_PAD), seg.x1 + BOX_PAD);
    for x in left..=right {
        put(cells, x, top, "─", plain());
        put(cells, x, bottom, "─", plain());
    }
    for row in top + 1..bottom {
        for column in [left, right] {
            // A junction may only replace a DASH: where the wall crosses
            // live rail the cell becomes `┼` and both lines read through
            // it, but a wall standing where there is no rail — the outer
            // side of the first or the last phase — is a plain wall.
            // Reading the cell rather than trusting `plan.rail` is what
            // keeps the junction honest about what is actually painted.
            let wall = match (row == plan.rail_row, under(cells, column, row) == "─") {
                (true, true) => "┼",
                _ => "│",
            };
            put(cells, column, row, wall, plain());
        }
    }
    put(cells, left, top, "╭", plain());
    put(cells, right, top, "╮", plain());
    put(cells, left, bottom, "╰", plain());
    put(cells, right, bottom, "╯", plain());
}

/// The road back. A reforging is a road, and a road is drawn: without
/// this the rail says `review` finished and `implement` lit up again,
/// and an operator watching a live run reads teleportation instead of a
/// loop.
///
/// The vocabulary is the SOLID set — `╰ ─ ╯` from the same rounded
/// corners the selection box uses, plus the mirror head `ᐸ` (U+1438),
/// the sibling of the rail's own operator-calibrated `ᐳ`. Never the
/// dashed set `╌ ┆`: those glyphs are drawn with a gap at every cell
/// boundary BY DESIGN, so no amount of correct geometry makes them
/// touch a corner — the operator's ruling of 2026-08-31, learned once
/// on the box and applied here from the start rather than after.
///
/// The corners rise toward the phases they belong to, so the arc reads
/// as one road leaving the rail and returning to it. Only the LANDING
/// wears a head, matching the rail's own asymmetry: `ᐳ` marks arrival,
/// never departure. The head sits inside the landing corner, pointing
/// into it.
fn arc(cells: &mut Cells, arc: &Arc, row: usize) {
    for x in arc.to..=arc.from {
        put(cells, x, row, "─", plain());
    }
    put(cells, arc.to, row, "╰", plain());
    put(cells, arc.from, row, "╯", plain());
    put(cells, arc.to + 1, row, "ᐸ", plain());
}

/// A fork and its rejoin. **The rejoin is drawn always** — it is the
/// join dependency, and the whole reason a fork is not two steps.
fn fork(cells: &mut Cells, join: &Join, rail_row: usize) {
    // Between the fork and its rejoin the rail gives way to the lanes,
    // unless the member count is odd and one member rides the rail row.
    // A labelled fork parts the rail to make room for its name; an
    // UNLABELLED one (a panel with no step name) must not leave a
    // gap — a rail that stops for eighteen columns reads as broken
    // track, not as parallelism.
    let filler = match join.label {
        Some(_) => " ",
        None => "─",
    };
    for x in join.x0 + 1..join.x1 {
        put(cells, x, rail_row, filler, plain());
    }
    for row in &join.rows {
        for x in join.x0 + 1..join.x1 {
            put(cells, x, *row, "─", plain());
        }
    }
    spine(cells, join, rail_row);
    if let Some(label) = &join.label {
        let inner = join.x1 - join.x0 - 3;
        let x = join.x0 + 2 + inner.saturating_sub(width_of(label)) / 2;
        put(cells, x, rail_row, label, plain());
    }
}

/// Adjacent cells sharing a style become one span, and a span is the
/// ONE sanitized constructor — so the graph opens no third path into a
/// buffer and the discipline test stays unamended.
fn row_line(row: &[Option<(String, Style)>]) -> Line<'static> {
    let mut runs: Vec<(String, Style)> = Vec::new();
    for (glyph, style) in row.iter().flatten() {
        match runs.last_mut() {
            Some((text, last)) if last == style => text.push_str(glyph),
            _ => runs.push((glyph.clone(), *style)),
        }
    }
    Line::from(
        runs.iter()
            .map(|(text, style)| span(text, *style))
            .collect::<Vec<Span<'static>>>(),
    )
}

/// The painter: it walks the plan and writes cells, and computes no
/// geometry of its own. Selection is `REVERSED` on the name or the node
/// label — the TUI's existing selection idiom, identical to the runs
/// table and the trail; current is the filled, coloured rail glyph.
/// Different attribute, different cell, different axis.
fn paint(plan: &Plan, tick: usize, animate: bool) -> Vec<Line<'static>> {
    let blank = Some((" ".to_string(), plain()));
    let mut cells: Cells = vec![vec![blank; plan.width]; plan.rows];
    // One rail, from the first segment's first node to the last one's.
    if let Some((from, to)) = plan.rail {
        for x in from..=to {
            put(&mut cells, x, plan.rail_row, "─", plain());
        }
    }
    for seg in &plan.segments {
        for join in &seg.joins {
            fork(&mut cells, join, plan.rail_row);
        }
    }
    for head in &plan.edges {
        // `ᐳ` (U+1433), chosen BY THE OPERATOR'S EYE from a rendered
        // specimen of seven candidates: `→`'s stem falls short of the
        // box-drawing stroke, `►` and `>` sit a pixel below the cell
        // midline in the operator's font, and the syllabics glyph is
        // the one whose point sits dead-centre on the dash axis.
        // Runners-up, recorded for the next font that disagrees: `⟩`
        // (close second), and box-drawing `╼`, which cannot misalign
        // by construction but reads as a heavy tip, not a point.
        put(&mut cells, *head, plan.rail_row, "ᐳ", plain());
    }
    for seg in &plan.segments {
        for mark in &seg.marks {
            let (style, ramp) = look(mark.class);
            let glyph = ramp[pulse(tick, mark.live, animate)];
            put(&mut cells, mark.x, mark.row, glyph, style);
            if !mark.label.is_empty() {
                // One clear cell between a node and its own text, so the
                // rail does not read as part of the word.
                put(&mut cells, mark.x + 1, mark.row, " ", plain());
                put(
                    &mut cells,
                    mark.x + 2,
                    mark.row,
                    &mark.label,
                    selected_style(mark.selected),
                );
            }
        }
        // The selected phase sits in a solid box — the terminal's
        // answer to the console's selection ring. It hugs the rows THIS
        // phase occupies and pads what it draws evenly on both sides,
        // and where a wall crosses the rail it wears the junction `┼`,
        // so the rail is SEEN to pass through the boundary rather than
        // stopping at it — an arrow head is never overwritten.
        if seg.selected {
            selection_box(&mut cells, seg, plan);
        }
        let current = match seg.class {
            Some(class) => look(class).0,
            None => plain(),
        };
        put(
            &mut cells,
            seg.name_x,
            plan.name_row,
            &seg.name,
            current.patch(selected_style(seg.selected)),
        );
    }
    // The roads back run on their own reserved row, under the names and
    // under the box's lower edge: nothing else is drawn there, so the
    // arcs overwrite nothing and are overwritten by nothing.
    if let Some(row) = plan.arc_row {
        for road in &plan.arcs {
            arc(&mut cells, road, row);
        }
    }
    if plan.left_elided {
        put(&mut cells, 0, plan.rail_row, "‹", plain());
    }
    if plan.right_elided {
        let x = plan.width.saturating_sub(1);
        put(&mut cells, x, plan.rail_row, "›", plain());
    }
    cells.iter().map(|row| row_line(row)).collect()
}

fn draw_graph(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    views: &Views,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    // Run-level notices first — a fallback selection or an optional
    // capability gap is a fact an operator must SEE, not find (decision
    // 0016) — then the graph, planned into whatever rows remain.
    let mut lines: Vec<Line> = Vec::new();
    for notice in &view.notices {
        lines.push(line(
            &format!("note  {} — {}", notice.kind, notice.text),
            tone_style("working"),
        ));
    }
    let status = view
        .summary
        .as_ref()
        .map_or("", |summary| summary.status.as_str());
    let plan = plan(
        &view.phases,
        lens,
        status,
        tui.cursor[0].as_deref(),
        tui.node.as_deref(),
        usize::from(area.width.saturating_sub(2)),
        usize::from(area.height.saturating_sub(2)).saturating_sub(lines.len()),
    );
    lines.extend(paint(&plan, tui.ticks, tui.animate));
    frame.render_widget(
        Paragraph::new(lines).block(
            pane("graph", tui.pane == 0)
                .title_top(brand(fleet_live(views), tui.ticks, tui.animate).right_aligned()),
        ),
        area,
    );
}

fn draw_seats(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    let cursor = tui.cursor[1].as_deref();
    let header = Row::new(
        [
            "participant",
            "status",
            "attempts",
            "turns",
            "cost",
            "activity",
        ]
        .iter()
        .map(|name| cell(name, header_style()))
        .collect::<Vec<Cell>>(),
    );
    let mut rows: Vec<Row> = Vec::new();
    for part in view
        .participants
        .iter()
        .filter(|part| render::keeps_participant(lens, part))
    {
        // `activity.text` IS the model's composition of `tool` and
        // `target_short` while a seat works, and its result-and-duration
        // once it concludes. The live/concluded distinction is a model
        // field too — `tool` is `Some` exactly while the seat works — so
        // it tints the cell rather than recomposing its text.
        let live = match part.activity.tool {
            Some(_) => tone_style("working"),
            None => plain(),
        };
        rows.push(
            Row::new(vec![
                cell(&part.label, plain()),
                cell(&part.status, tone_style(&part.status)),
                cell(&part.attempts.to_string(), plain()),
                cell(&part.turns_cell.text, plain()),
                cell(&part.cost_cell.text, plain()),
                cell(&part.activity.text, live),
            ])
            .style(selected_style(cursor == Some(part.key.as_str()))),
        );
        // Which agent, model and provider actually served this seat
        // (decision 0016). The sentence is the model's; this pane only
        // places it, so a fallback cannot go unmentioned here while it
        // shows elsewhere.
        if let Some(provenance) = &part.provenance {
            // In the widest column, under its seat: the narrow leading
            // columns would clip the sentence, and a clipped honesty
            // rule is not one.
            rows.push(Row::new(vec![
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell("", plain()),
                cell(&format!("↳ {}", provenance.line), plain()),
            ]));
        }
    }
    let widths = [
        Constraint::Length(22),
        Constraint::Length(13),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(10),
        Constraint::Min(10),
    ];
    frame.render_widget(
        Table::new(rows, widths)
            .header(header)
            .block(pane("seats", tui.pane == 1)),
        area,
    );
}

fn draw_trail(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    let cursor = tui.cursor[2].as_deref();
    let lines: Vec<Line> = view
        .journal
        .iter()
        .filter(|row| row.in_trail && render::keeps_row(lens, row))
        .map(|row| {
            let seq = row.seq.to_string();
            line(
                &format!("{seq}  {}  {}", row.event_type, row.what.text),
                selected_style(cursor == Some(seq.as_str())),
            )
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(pane("trail", tui.pane == 2)),
        area,
    );
}

fn draw_participant(frame: &mut Frame, area: Rect, tui: &Tui, views: &Views, part: &Participant) {
    let [head, stream, transcript] = Layout::vertical([
        Constraint::Length(7),
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .areas(area);
    // The `claude --resume` line is always here: the id is the model's
    // `session_id`, and its absence is the model's absence mark rather
    // than a pasteable lie.
    // A session id that could not name a transcript must not be
    // rendered into a pasteable command either: an id failing the
    // shared guard is an absence, not a suggestion.
    let session = match &part.session_id {
        Some(session) if crate::ui::valid_session_id(session) => session.clone(),
        _ => brokkr_view::ABSENT.to_string(),
    };
    let lines = vec![
        Line::from(vec![
            span(&part.label, header_style()),
            span(" · ", plain()),
            span(&part.status, tone_style(&part.status)),
        ]),
        line(&format!("terminal  {}", part.terminal_line.text), plain()),
        line(&format!("full session: claude --resume {session}"), plain()),
        line(
            &match &part.provenance {
                Some(provenance) => format!("served by  {}", provenance.line),
                None => format!("served by  {}", brokkr_view::ABSENT),
            },
            plain(),
        ),
        line(
            &format!(
                "attempts {} · turns {} · cost {}",
                part.attempts, part.turns_cell.text, part.cost_cell.text
            ),
            plain(),
        ),
    ];
    frame.render_widget(Paragraph::new(lines).block(pane("seat", false)), head);

    let checkpoints: Vec<Line> = part
        .checkpoints
        .iter()
        .map(|row| {
            line(
                &format!(
                    "{}  {}  {}  {}",
                    row.turn.text, row.step, row.target.text, row.recorded_at
                ),
                plain(),
            )
        })
        .collect();
    frame.render_widget(
        Paragraph::new(checkpoints)
            .scroll((offset_for(tui, 0), 0))
            .block(pane("checkpoints", tui.pane == 0)),
        stream,
    );

    let (lines, scroll) = match &views.transcript {
        Some((turns, truncated)) => transcript_lines(
            turns,
            *truncated,
            selected_turn(tui, views).map(|(index, _)| index),
        ),
        None => (
            vec![line(
                "no local session transcript on this machine — the `claude --resume` line above opens the full session",
                plain(),
            )],
            0,
        ),
    };
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((u16::try_from(scroll).unwrap_or(u16::MAX), 0))
            .block(pane("transcript", tui.pane == 1)),
        transcript,
    );
}

/// The scroll of the focused pane only: the other pane keeps its top.
fn offset_for(tui: &Tui, pane: usize) -> u16 {
    match tui.pane == pane {
        true => u16::try_from(tui.offset).unwrap_or(u16::MAX),
        false => 0,
    }
}

/// Transcript prose is arbitrary text from outside the store, so it goes
/// through `Safe` like everything else, and the truncation flag is
/// **shown**: silently short evidence is worse than none (decision 0001).
/// The selected turn wears the same mark as every selected row, and the
/// returned scroll is that turn's own first line, so the pane follows
/// the cursor rather than holding a second offset that could drift.
fn transcript_lines(
    turns: &[Turn],
    truncated: bool,
    selected: Option<usize>,
) -> (Vec<Line<'static>>, usize) {
    let mut lines: Vec<Line> = Vec::new();
    let mut scroll = 0usize;
    for (index, turn) in turns.iter().enumerate() {
        let picked = selected == Some(index);
        if picked {
            scroll = lines.len();
        }
        lines.push(line(
            &format!("{} · {}", turn.role, turn.ts),
            header_style().patch(selected_style(picked)),
        ));
        for block in &turn.blocks {
            lines.push(line(&format!("  {}", block.text), selected_style(picked)));
        }
    }
    if truncated {
        lines.push(line(TRUNCATED_NOTICE, header_style()));
    }
    (lines, scroll)
}

// ---------------------------------------------------------- the terminal

/// The five process-global crossterm calls, held as **function-pointer
/// fields**. Their production values are crossterm's own function items,
/// never our wrappers and never closures: either would be a counted
/// function only production could execute.
pub(crate) struct TerminalOps {
    pub enter_raw: fn() -> std::io::Result<()>,
    pub leave_raw: fn() -> std::io::Result<()>,
    pub poll: fn(Duration) -> std::io::Result<bool>,
    pub read: fn() -> std::io::Result<Event>,
    pub size: fn() -> std::io::Result<(u16, u16)>,
}

pub(crate) fn production_ops() -> TerminalOps {
    TerminalOps {
        enter_raw: enable_raw_mode,
        leave_raw: disable_raw_mode,
        poll: ratatui::crossterm::event::poll,
        read: ratatui::crossterm::event::read,
        size: ratatui::crossterm::terminal::size,
    }
}

/// Restoration is RAII: a trailing `disable_raw_mode()` is skipped by
/// every `?` between setup and the end of the loop, which is precisely
/// the bug "prove restoration on the error path" exists to catch. A
/// guard makes `?` safe by construction.
struct Guard<W: Write> {
    out: W,
    leave_raw: fn() -> std::io::Result<()>,
}

impl<W: Write> Drop for Guard<W> {
    fn drop(&mut self) {
        // Errors are swallowed deliberately: a failing restore must not
        // replace the error the operator actually needs to read, and a
        // panicking `Drop` during unwinding aborts the process.
        let _ = execute!(self.out, LeaveAlternateScreen, Show);
        let _ = (self.leave_raw)();
    }
}

/// The panic hook's restore, as a bare `fn` so the hook holds no closure
/// over a terminal it does not own.
fn restore_stdout() {
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen, Show);
    let _ = disable_raw_mode();
}

/// Restore **first**, chain to the previous hook **second**: a panic
/// message printed into a raw-mode alternate screen is unreadable, and
/// swallowing the message entirely is worse than an ugly one.
pub(crate) fn install_panic_hook(restore: fn()) {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        previous(info);
    }));
}

/// The three startup refusals, as one pure rule. Both `brokkr inspect`
/// and `brokkr watch` are named: an operator who cannot have the console
/// must be told what they can have instead.
pub(crate) fn refuse(is_tty: bool, size: (u16, u16), db_is_file: bool) -> Option<String> {
    let instead = "use `brokkr inspect --run <id>` or `brokkr watch --run <id>` instead";
    if !db_is_file {
        return Some(format!(
            "no workspace database to read, and a read never creates one; {instead}"
        ));
    }
    if !is_tty {
        return Some(format!(
            "`brokkr tui` needs a terminal and stdout is not one; {instead}"
        ));
    }
    if size.0 < MIN_WIDTH || size.1 < MIN_HEIGHT {
        return Some(format!(
            "this terminal is {}×{}, below the {MIN_WIDTH}×{MIN_HEIGHT} `brokkr tui` needs; {instead}",
            size.0, size.1
        ));
    }
    None
}

/// The bounded shell: draw, poll, apply, repeat. Everything impure it
/// touches arrives as a parameter, so the whole loop — its quit arm, its
/// error arm and its transient-busy arms — runs under `TestBackend`.
fn drive<B: Backend>(
    terminal: &mut Terminal<B>,
    ops: &TerminalOps,
    source: &mut dyn FnMut(Ask) -> Result<Refreshed>,
    tui: &mut Tui,
    max_iterations: usize,
) -> Result<ExitCode>
where
    // A backend's own error reaches the operator through `anyhow`.
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut views = Views::empty();
    let mut failures = 0usize;
    for _ in 0..max_iterations {
        let session = session_of(tui, &views).map(str::to_string);
        let ask = Ask {
            run: tui.run.as_deref(),
            session: session.as_deref(),
            working: session_is_live(tui, &views),
            force: std::mem::take(&mut tui.force),
            fleet: tui.ticks.is_multiple_of(RUNS_REFRESH_TICKS),
        };
        match source(ask) {
            Ok(Some(fresh)) => {
                views = fresh;
                failures = 0;
                tui.status = None;
            }
            Ok(None) => failures = 0,
            Err(error) => {
                // A transient store error (SQLITE_BUSY while a `forge
                // run` holds the write lock) is a frame that says so,
                // with keys still live. A persistent one gives up.
                failures += 1;
                tui.status = Some(format!("the journal is not readable right now: {error}"));
                anyhow::ensure!(
                    failures < crate::WATCH_TRANSIENT_FRAMES,
                    "giving up after {failures} unreadable polls: {error}"
                );
            }
        }
        settle(tui, &views);
        terminal.draw(|frame| draw(frame, tui, &views))?;
        if (ops.poll)(TICK)? {
            if let Some(key) = from_crossterm((ops.read)()?) {
                if apply(tui, &views, key) == Flow::Quit {
                    return Ok(ExitCode::SUCCESS);
                }
            }
        }
        tui.ticks += 1;
    }
    Ok(ExitCode::SUCCESS)
}

/// Enter the terminal, run the console, leave the terminal — with every
/// environment fact and every terminal call arriving as a parameter, so
/// the whole of this function executes in tests as well as in
/// production. Nothing here exits the process outright — that would run
/// past the guard's `Drop` and leave a terminal in raw mode — so the TUI
/// returns an `ExitCode` like every other arm.
#[allow(clippy::too_many_arguments)]
pub(crate) fn start<B: Backend, R: Write>(
    db_is_file: bool,
    run: Option<String>,
    ops: TerminalOps,
    is_tty: bool,
    // Animation is enabled exactly when colour is: the same line kind
    // as `is_tty`, read once at the call site and injected here, so a
    // test sets it directly and touches no environment.
    animate: bool,
    backend: B,
    restore: R,
    source: &mut dyn FnMut(Ask) -> Result<Refreshed>,
    max_iterations: usize,
) -> Result<ExitCode>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let size = (ops.size)().unwrap_or((0, 0));
    if let Some(message) = refuse(is_tty, size, db_is_file) {
        anyhow::bail!("{message}");
    }
    (ops.enter_raw)()?;
    let mut guard = Guard {
        out: restore,
        leave_raw: ops.leave_raw,
    };
    execute!(guard.out, EnterAlternateScreen, Hide)?;
    install_panic_hook(restore_stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut state = Tui::new(run);
    state.animate = animate;
    let code = drive(&mut terminal, &ops, source, &mut state, max_iterations);
    // Uninstalled on the normal path: a panic later in this process must
    // not restore a terminal this function has already left.
    let _ = std::panic::take_hook();
    code
}

#[cfg(test)]
mod tests;
