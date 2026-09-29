//! The TUI frame the CPU budgets measure (#342): one frame of the RUN
//! level, drawn by the console's own `draw` into ratatui's `TestBackend`.

use brokkr_core::envelope::EventEnvelope;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

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
    };
    // Opened at the RUN level, as `brokkr tui --run <id>` opens.
    let tui = Tui::over(Some(run_id.to_string()), Vec::new(), 0);
    let mut terminal = Terminal::new(TestBackend::new(width, height))
        .expect("a TestBackend terminal cannot fail to open");
    terminal
        .draw(|frame| draw(frame, &tui, &views))
        .expect("a TestBackend draw cannot fail");
    terminal.backend().buffer().clone()
}
