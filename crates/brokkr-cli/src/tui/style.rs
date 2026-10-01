//! The style primitives: the one sanitizer, the two constructors that
//! reach a widget, the styles, the brand mark and the pane border.

use super::*;

// -------------------------------------------------------------- the paint

/// The one sanitizer, reused: three surfaces, one `Safe`.
pub(super) fn safe(text: &str) -> String {
    Safe::new(text).as_str().to_string()
}

/// One of the two constructors that reach a widget. Both take sanitized
/// text, so a hostile string cannot arrive at a buffer without a visible
/// `Safe` at the call site — and ratatui then measures exactly the text
/// it draws.
pub(super) fn cell(text: &str, style: Style) -> Cell<'static> {
    cell_lines(&[text.to_string()], style)
}

/// A cell of several lines, each through [`span`]: the one constructor
/// a cell is made by.
pub(super) fn cell_lines(texts: &[String], style: Style) -> Cell<'static> {
    Cell::from(Text::from(
        texts
            .iter()
            .map(|text| line(text, style))
            .collect::<Vec<_>>(),
    ))
}

pub(super) fn span(text: &str, style: Style) -> Span<'static> {
    Span::styled(safe(text), style)
}

pub(super) fn line(text: &str, style: Style) -> Line<'static> {
    Line::from(span(text, style))
}

pub(super) fn plain() -> Style {
    Style::new()
}

pub(super) fn header_style() -> Style {
    Style::new().add_modifier(Modifier::BOLD)
}

pub(super) fn selected_style(selected: bool) -> Style {
    match selected {
        true => Style::new().add_modifier(Modifier::REVERSED),
        false => Style::new(),
    }
}

/// One classification, three renderings of it: `render::tone` is the
/// table, ANSI is `brokkr runs`' rendering of it, and this is the TUI's.
pub(super) fn tone_style(status: &str) -> Style {
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
pub(super) fn brand(fleet_live: bool, ticks: usize, animate: bool) -> Line<'static> {
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

pub(super) fn pane(title: &str, focused: bool) -> Block<'static> {
    let border = match focused {
        true => Style::new().add_modifier(Modifier::BOLD),
        false => Style::new().add_modifier(Modifier::DIM),
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(border)
        .title(Line::from(span(title, border)))
}
