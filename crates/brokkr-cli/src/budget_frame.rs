//! The TUI frame the CPU budgets measure (#342): one frame of the RUN
//! level, drawn by the console's own `draw` into ratatui's `TestBackend`.

use std::path::Path;

use brokkr_core::envelope::EventEnvelope;
use brokkr_store::{Store, StoreError};
use brokkr_view::RunsView;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

use crate::fleet::{Journal, Pages, AHEAD_ROWS};
use crate::tui::{draw, Tui, Views};

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
    };
    let views = Views {
        now: now.to_string(),
        runs: brokkr_view::run_rows(&[entry]),
        run: Some(brokkr_view::run_view(events, state.as_ref())),
        transcript: None,
        note: None,
        unread: Default::default(),
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

/// The console's first fleet page over the journal at `db`, from its
/// read-only open to its rows (decision 0089 ruling 9): the visible page
/// and the one ahead, read and derived as the console reads them.
pub fn fleet_page_for_budget(db: &Path, now: &str) -> Result<RunsView, StoreError> {
    let store = Store::open_read_only(db)?;
    let mut pages = Pages::default();
    pages.refresh(&store, Journal::at(db), AHEAD_ROWS)?;
    let page = pages.page(&store, now, None, AHEAD_ROWS, &|_| false)?;
    let mut runs = RunsView::of(page.rows.into_iter().map(|(_, row)| row).collect());
    runs.page = Some(brokkr_view::Paged { next: page.next });
    Ok(runs)
}

/// [`fleet_page_for_budget`] painted at `width`×`height` by the console's
/// own `draw` at its RUNS level: the first page, open to painted frame,
/// and the frame's text, row after row, each cell its symbol.
pub fn fleet_frame_for_budget(
    db: &Path,
    now: &str,
    width: u16,
    height: u16,
) -> Result<String, StoreError> {
    let views = Views {
        runs: fleet_page_for_budget(db, now)?,
        now: now.to_string(),
        ..Views::empty()
    };
    // Opened at the RUNS level, as `brokkr tui` opens.
    let mut tui = Tui::over(None, Vec::new(), 0);
    (tui.width, tui.height) = (width, height);
    let terminal = painted(&tui, &views, (width, height));
    let cells = terminal.backend().buffer().content().iter();
    Ok(cells.map(|cell| cell.symbol()).collect())
}
