//! `forge tui` — the interactive, read-only console (decision 0014).
//!
//! A **third renderer** over `forge-view`'s models, never a fourth
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
use forge_view::{Participant, RunView, RunsView};
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
/// four times a second against a `forge run` holding the write lock is
/// the cost this constant exists to refuse (spec §6).
const RUNS_REFRESH_TICKS: usize = 8;

/// One `PageUp`/`PageDown` in list rows.
const PAGE: usize = 10;

// ------------------------------------------------------------- the models

/// One frame's worth of derivation, produced by the injected source and
/// dropped after the frame. **No `forge-view` model is retained**, which
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
                view_version: forge_view::VIEW_VERSION,
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
        self.offset = 0;
        self.force = true;
    }
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

// --------------------------------------------------------------- the keys

/// Our own key vocabulary. Translation happens once, at the boundary.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Key {
    Up,
    Down,
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

/// How many lines the focused PARTICIPANT pane holds — model fields
/// both: the seat's checkpoint stream, or the transcript's turns.
fn stream_len(tui: &Tui, views: &Views) -> usize {
    match tui.pane {
        0 => seat_of(tui, views).map_or(0, |part| part.checkpoints.len()),
        _ => views
            .transcript
            .as_ref()
            .map_or(0, |(turns, _)| turns.len()),
    }
}

fn step(tui: &mut Tui, views: &Views, step: Step) {
    if tui.level == Level::Participant {
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
        // PARTICIPANT panes are paragraphs, not doors.
        Level::Participant => {}
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
        Key::Up => step(tui, views, Step::Up),
        Key::Down => step(tui, views, Step::Down),
        Key::PageUp => step(tui, views, Step::PageUp),
        Key::PageDown => step(tui, views, Step::PageDown),
    }
    Flow::Continue
}

// ------------------------------------------------------ footer and help

/// Discoverability is a requirement, not a nicety: the footer names the
/// keys available in the CURRENT context, and differs per (level, pane,
/// typing, help) so a constant footer cannot pass its test.
pub(crate) fn footer_for(tui: &Tui) -> String {
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
        (Level::Run, 0) => format!("↑↓/jk move · Enter scope phase · Tab pane · Esc back {tail}"),
        (Level::Run, 1) => {
            let verb = match (scoped_seat(tui), tui.cursor[1].as_deref()) {
                (Some(scoped), Some(cursor)) if scoped == cursor => "Enter open seat",
                _ => "Enter scope seat",
            };
            format!("↑↓/jk move · {verb} · Tab pane · Esc back {tail}")
        }
        (Level::Run, 2) => format!("↑↓/jk move · Enter read row · Tab pane · Esc back {tail}"),
        (Level::Run, _) => format!("↑↓/jk move · Tab pane · Esc back {tail}"),
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

const HELP: [&str; 12] = [
    "forge tui — a read-only console over the same models as",
    "forge inspect, forge watch and forge ui. It issues no",
    "operator commands and writes nothing to the journal.",
    "",
    "↑ ↓ j k     move          Enter   descend / scope",
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
/// table, ANSI is `forge runs`' rendering of it, and this is the TUI's.
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
    frame.render_widget(Paragraph::new(line(&status_line(tui), plain())), status);
    frame.render_widget(
        Paragraph::new(line(
            &footer_for(tui),
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
        line("this terminal is too small for forge tui", header_style()),
        line(
            "try `forge inspect --run <id>` or `forge watch --run <id>`,",
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
        let age = match forge_view::age(&row.created_at, &views.now) {
            Some(age) => age,
            None => forge_view::ABSENT.to_string(),
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
    draw_graph(frame, graph, tui, view, lens.as_ref());
    draw_seats(frame, seats, tui, view, lens.as_ref());
    draw_trail(frame, trail, tui, view, lens.as_ref());
}

/// The 0013 tree: `⑂` precedes parallel members, `→` a sequential step,
/// both nested under their phase. The markers are content, not colour,
/// so the graph is uncoloured exactly as `graph_block` leaves it.
fn draw_graph(
    frame: &mut Frame,
    area: Rect,
    tui: &Tui,
    view: &RunView,
    lens: Option<&render::Lens>,
) {
    let cursor = tui.cursor[0].as_deref();
    let mut lines: Vec<Line> = Vec::new();
    // Run-level notices first: a fallback selection and an optional
    // capability gap are facts an operator must SEE, not find.
    for notice in &view.notices {
        lines.push(line(
            &format!("note  {} — {}", notice.kind, notice.text),
            tone_style("working"),
        ));
    }
    for phase in &view.phases {
        // The scope's own marker, decided by the crate's ONE phase
        // predicate — the same call `graph_block` makes.
        let mut head = match lens.is_some() && render::keeps_phase(lens, phase) {
            true => "▸ ".to_string(),
            false => "  ".to_string(),
        };
        head.push_str(&phase.name);
        head.push_str(" ×");
        head.push_str(&phase.visits.to_string());
        if phase.current {
            head.push_str("  ←current");
        }
        lines.push(line(
            &head,
            selected_style(cursor == Some(phase.name.as_str())),
        ));
        for column in &phase.columns {
            match column.nodes.as_slice() {
                [node] => {
                    let label = match &column.label {
                        Some(label) => label.clone(),
                        None => node.label.clone(),
                    };
                    lines.push(line(&format!("  → {label} · {}", node.state), plain()));
                }
                nodes => {
                    let label = match &column.label {
                        Some(label) => format!(" {label}"),
                        None => String::new(),
                    };
                    lines.push(line(&format!("  ⑂{label}"), plain()));
                    for node in nodes {
                        lines.push(line(
                            &format!("    {} · {}", node.label, node.state),
                            plain(),
                        ));
                    }
                }
            }
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(pane("graph", tui.pane == 0)),
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
        _ => forge_view::ABSENT.to_string(),
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
                None => format!("served by  {}", forge_view::ABSENT),
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

    let lines = match &views.transcript {
        Some((turns, truncated)) => transcript_lines(turns, *truncated),
        None => vec![line(
            "no local session transcript on this machine — the `claude --resume` line above opens the full session",
            plain(),
        )],
    };
    frame.render_widget(
        Paragraph::new(lines)
            .scroll((offset_for(tui, 1), 0))
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
fn transcript_lines(turns: &[Turn], truncated: bool) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    for turn in turns {
        lines.push(line(
            &format!("{} · {}", turn.role, turn.ts),
            header_style(),
        ));
        for block in &turn.blocks {
            lines.push(line(&format!("  {}", block.text), plain()));
        }
    }
    if truncated {
        lines.push(line(
            "transcript truncated (size cap) — claude --resume carries the rest",
            header_style(),
        ));
    }
    lines
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

/// The three startup refusals, as one pure rule. Both `forge inspect`
/// and `forge watch` are named: an operator who cannot have the console
/// must be told what they can have instead.
pub(crate) fn refuse(is_tty: bool, size: (u16, u16), db_is_file: bool) -> Option<String> {
    let instead = "use `forge inspect --run <id>` or `forge watch --run <id>` instead";
    if !db_is_file {
        return Some(format!(
            "no workspace database to read, and a read never creates one; {instead}"
        ));
    }
    if !is_tty {
        return Some(format!(
            "`forge tui` needs a terminal and stdout is not one; {instead}"
        ));
    }
    if size.0 < MIN_WIDTH || size.1 < MIN_HEIGHT {
        return Some(format!(
            "this terminal is {}×{}, below the {MIN_WIDTH}×{MIN_HEIGHT} `forge tui` needs; {instead}",
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
            force: std::mem::take(&mut tui.force),
            fleet: tui.ticks % RUNS_REFRESH_TICKS == 0,
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
    let code = drive(&mut terminal, &ops, source, &mut state, max_iterations);
    // Uninstalled on the normal path: a panic later in this process must
    // not restore a terminal this function has already left.
    let _ = std::panic::take_hook();
    code
}

#[cfg(test)]
mod tests;
