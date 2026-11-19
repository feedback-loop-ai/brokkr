//! How the terminal wants to be written to: colour and width, read from
//! the environment and the terminal once, by rules that are pure.

use std::ffi::OsString;
use std::io::IsTerminal;

const DEFAULT_WIDTH: usize = 80;
const MIN_WIDTH: usize = 20;
const MAX_WIDTH: usize = 1000;

/// How the terminal wants to be written to. Both facts come from the
/// environment; the rules that read them are pure.
pub(crate) struct Style {
    pub color: bool,
    pub width: usize,
}

impl Style {
    pub(crate) fn detect() -> Style {
        Style::detect_in(
            std::io::stdout().is_terminal(),
            std::env::var_os,
            ratatui::crossterm::terminal::size,
        )
    }

    /// [`Style::detect`] over a given terminal, environment `var` and
    /// terminal `size`. The width is `COLUMNS` where it is set, which a
    /// shell seldom exports, else the terminal's own columns where stdout
    /// is one (#1167), else the default.
    pub(crate) fn detect_in(
        tty: bool,
        var: impl Fn(&'static str) -> Option<OsString>,
        size: fn() -> std::io::Result<(u16, u16)>,
    ) -> Style {
        let measured = || match tty {
            true => size().ok().map(|(columns, _)| columns.to_string()),
            false => None,
        };
        let columns = var("COLUMNS").and_then(|v| v.into_string().ok());
        Style {
            color: color_enabled(
                tty,
                var("NO_COLOR").is_some(),
                var("TERM") == Some("dumb".into()),
            ),
            width: width_from(columns.or_else(measured).as_deref()),
        }
    }

    /// The shape every golden runs in: colour is a post-processing wrap,
    /// so the goldens prove content and exactly one test proves colour.
    #[cfg(test)]
    pub(crate) fn plain(width: usize) -> Style {
        Style {
            color: false,
            width,
        }
    }
}

pub(super) fn color_enabled(is_terminal: bool, no_color: bool, term_is_dumb: bool) -> bool {
    is_terminal && !no_color && !term_is_dumb
}

pub(super) fn width_from(value: Option<&str>) -> usize {
    match value.and_then(|text| text.parse::<usize>().ok()) {
        Some(width) => width.clamp(MIN_WIDTH, MAX_WIDTH),
        None => DEFAULT_WIDTH,
    }
}
