//! The TUI frame the CPU budgets measure (#342): one frame of the RUN
//! level, drawn by the console's own `draw` into ratatui's `TestBackend`.

use std::path::Path;

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::{Opened, Store, StoreError};
use brokkr_view::{Cursor, RunView, RunsView};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use crate::fleet::Journal;
use crate::run_pages::{console, viewed, Console, Shows};
use crate::tui::{asked_window, draw, Scrolled, Tui, Views};
use crate::tui_read::{fleet_page, Kept};

/// Draw the RUN level of `events` at `width`×`height` from the derivation
/// the console reads. `now` is the clock read the derivation refuses to
/// make itself. A journal that does not fold draws with no state, as the
/// fleet shows a quarantined run.
pub fn run_frame_for_budget(
    events: &[EventEnvelope],
    now: &str,
    width: u16,
    height: u16,
) -> Buffer {
    let state = brokkr_core::fold::fold(events).ok();
    let first = events.first();
    let run_id = first.map_or("", |event| event.run_id.as_str());
    let entry = brokkr_view::RunEntry {
        run_id,
        feature: state
            .as_ref()
            .and_then(|state| state.feature.as_deref())
            .unwrap_or(""),
        created_at: first.map_or("", |event| event.recorded_at.as_str()),
        last_recorded_at: events.last().map(|event| event.recorded_at.as_str()),
        state: state.as_ref(),
        detail: None,
        residuals: &[],
        closing: state
            .as_ref()
            .and_then(|state| brokkr_core::ending::closing(events, state)),
    };
    let views = Views {
        start: 0,
        now: now.to_string(),
        runs: brokkr_view::run_rows(&[entry]),
        run: Some(brokkr_view::run_view(events, state.as_ref())),
        transcript: None,
        note: None,
        unread: Default::default(),
        window: Default::default(),
        ahead: None,
        more: false,
        end: None,
    };
    // Opened at the RUN level, as `brokkr tui --run <id>` opens.
    let tui = Tui::over(Some(run_id.to_string()), Vec::new(), 0);
    painted(&tui, &views, (width, height))
        .backend()
        .buffer()
        .clone()
}

/// `views` drawn by the console's own `draw` as `tui` stands, into a
/// `TestBackend` of `width`×`height`.
fn painted(tui: &Tui, views: &Views, (width, height): (u16, u16)) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height))
        .expect("a TestBackend terminal cannot fail to open");
    terminal
        .draw(|frame| draw(frame, tui, views))
        .expect("a TestBackend draw cannot fail");
    terminal
}

/// Where the console's fleet list stands for a budget's read (decision
/// 0089 ruling 9): the keys past the pages it scrolled beyond, as each
/// frame's `ahead` stated them, whether `a` lists the older runs, and the
/// filter typed over it.
#[derive(Clone, Default)]
pub struct FleetScroll {
    pub scrolled: Vec<Cursor>,
    pub all: bool,
    pub filter: String,
}

/// The console's list as `scroll` stands it: the keys it keeps to step
/// back by, the console's own, held while its window is read as the
/// console holds them, and the window it asks for.
pub struct FleetWindow(Tui);

/// [`FleetWindow`] for `scroll`, the console scrolled past each page
/// `scroll` names as the console scrolls, keeping what the console keeps.
pub fn fleet_window_for_budget(scroll: &FleetScroll) -> FleetWindow {
    let mut tui = Tui::over(None, Vec::new(), 0);
    tui.all = scroll.all;
    tui.filter.clone_from(&scroll.filter);
    tui.scrolled = Scrolled {
        filter: scroll.filter.clone(),
        past: Vec::new(),
    };
    for ahead in &scroll.scrolled {
        tui.scrolled.passed(ahead.clone());
    }
    FleetWindow(tui)
}

/// The console's fleet over the journal at `db` in the window `list`
/// asks for, by the console's own ask, from its read-only open to its
/// rows: the page the window starts at and the one ahead, read and
/// derived as the console reads them, and the key past the window's first
/// page.
pub fn fleet_page_for_budget(
    db: &Path,
    now: &str,
    list: &FleetWindow,
) -> Result<(RunsView, Option<Cursor>), StoreError> {
    let window = asked_window(&list.0);
    let store = Store::open_read_only(db)?;
    let mut kept = Kept::default();
    let listed = fleet_page(&store, &mut kept, now, (&window, None), Journal::at(db))?;
    Ok((listed.runs, listed.ahead))
}

/// `brokkr runs --json` with no other flag over the journal at `db` at
/// `now`, from its read-only open: every run, a page at a time, written
/// to `out` (decision 0089 ruling 6). The table reads the same pages.
pub fn fleet_listing_for_budget(
    db: &Path,
    now: &str,
    out: &mut impl std::io::Write,
) -> anyhow::Result<()> {
    let store = Store::open_read_only(db)?;
    let journal = (Journal::at(db), now);
    crate::fleet::list_hearth(&store, journal, true, |lister| lister.every_json(out))
}

/// `brokkr costs --run <run>` of `run` in `store` after its opening, which
/// verified it to `opened`: what decision 0089 ruling 9 bounds after the
/// opening, to the bytes it prints.
pub fn costs_for_budget(store: &Store, run: &str, opened: &Opened) -> anyhow::Result<String> {
    let costs = crate::compare::costs_at(store, run.to_string(), opened)?;
    Ok(serde_json::to_string_pretty(&costs)?)
}

/// `brokkr compare` of the two runs `runs` names in `store` after their
/// openings, which verified each to the head beside it: what decision
/// 0089 ruling 9 bounds after the openings, to the bytes it prints.
pub fn compare_for_budget(store: &Store, runs: [(&str, &Opened); 2]) -> anyhow::Result<String> {
    crate::compare::compared_at(store, runs)
}

/// `brokkr runs --json` with no other flag over a many-hearth world of
/// the journals `journals`, each its own realm, at `now`: a hearth at a
/// time, each a page at a time, written to `out` (decision 0089 ruling 6).
pub fn fleet_world_listing_for_budget(
    journals: &[std::path::PathBuf],
    now: &str,
    out: &mut impl std::io::Write,
) -> anyhow::Result<()> {
    let hearths: Vec<brokkr_runtime::realms::Hearth> = (0..)
        .zip(journals)
        .map(|(n, journal)| brokkr_runtime::realms::Hearth {
            realms: vec![format!("realm-{n}")],
            journal: journal.clone(),
        })
        .collect();
    let style = crate::render::Style {
        color: false,
        width: 80,
    };
    let every = crate::fleet::Filter::default();
    crate::fleet::list_world(&hearths, (true, every), (now, &style), out)
}

/// The console's first fleet page painted at `width`×`height` by its own
/// `draw` at its RUNS level: open to painted frame, and the frame's text,
/// row after row, each cell its symbol.
pub fn fleet_frame_for_budget(
    db: &Path,
    now: &str,
    width: u16,
    height: u16,
) -> Result<String, StoreError> {
    let views = Views {
        start: 0,
        runs: fleet_page_for_budget(db, now, &fleet_window_for_budget(&FleetScroll::default()))?.0,
        now: now.to_string(),
        ..Views::empty()
    };
    // Opened at the RUNS level, as `brokkr tui` opens.
    Ok(painted_text(
        Tui::over(None, Vec::new(), 0),
        &views,
        (width, height),
    ))
}

/// `views` drawn as `tui` stands at `width`×`height`: the frame's text,
/// row after row, each cell its symbol.
fn painted_text(mut tui: Tui, views: &Views, (width, height): (u16, u16)) -> String {
    (tui.width, tui.height) = (width, height);
    let terminal = painted(&tui, views, (width, height));
    let cells = terminal.backend().buffer().content().iter();
    cells.map(|cell| cell.symbol()).collect()
}

/// The view the console's RUN level paints of `run` in the journal at
/// `db`, from its read-only open, read as the console reads it when the
/// run is selected (decision 0089 slice 3): the frame, and no seat's
/// checkpoints.
pub fn console_run_view_for_budget(db: &Path, run: &str) -> Result<RunView, StoreError> {
    let store = Store::open_read_only(db)?;
    Ok(console(&store, run, Console::Run, None, None, 0, 0)?
        .0
        .view())
}

/// The view the console's fleet detail pane paints of `run` at `rows`: the
/// frame and the merged window across the seats at work, from its
/// read-only open, as the shell asks it at a scroll `rows` deep. The
/// console's detail read, for the heap proof that its intermediate merge
/// is the window's however deep the scroll (F4, decision 0089 ruling 9).
pub fn console_detail_view_for_budget(
    db: &Path,
    run: &str,
    rows: usize,
) -> Result<RunView, StoreError> {
    let store = Store::open_read_only(db)?;
    Ok(console(&store, run, Console::Detail, None, None, rows, 0)?
        .0
        .view())
}

/// The console's RUN level as it is first painted after `run` is selected
/// in the journal at `db`, at `now`, 160×48, by its own `draw`, from its
/// read-only open: the run read as the console reads it at its run level
/// ([`console_run_view_for_budget`]), or with `whole` read whole and
/// folded, as the console read it before the pages; the frame's text.
pub fn console_run_level_for_budget(
    db: &Path,
    run: &str,
    now: &str,
    whole: bool,
) -> Result<String, StoreError> {
    let view = match whole {
        true => {
            let events = Store::open_read_only(db)?.load(run)?;
            brokkr_view::run_view(&events, brokkr_core::fold::fold(&events).ok().as_ref())
        }
        false => console_run_view_for_budget(db, run)?,
    };
    let views = Views {
        start: 0,
        now: now.to_string(),
        run: Some(view),
        ..Views::empty()
    };
    let tui = Tui::over(Some(run.to_string()), Vec::new(), 0);
    Ok(painted_text(tui, &views, (160, 48)))
}

/// What `brokkr watch --once` paints of `run` in the journal at `db`, its
/// terminal frame, or with `whole` what `inspect --json` prints of it:
/// from its read-only open, the run opened and read as those readouts
/// read it (decision 0089 slice 3), at 80 columns and no colour.
pub fn run_readout_for_budget(db: &Path, run: &str, whole: bool) -> Result<String, StoreError> {
    let store = Store::open_read_only(db)?;
    let view = viewed(&store, run, Shows::of(whole))?.view;
    let style = crate::render::Style {
        color: false,
        width: 80,
    };
    Ok(match whole {
        true => serde_json::to_string_pretty(&view.with_scopes()).expect("the run view serializes"),
        false => crate::render::inspect(&view, None, false, &style),
    })
}
