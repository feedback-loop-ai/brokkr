//! How the terminal wants to be written to: colour and width, read from
//! the environment and stdout once, by rules that are pure.

use std::ffi::OsString;
use std::io::IsTerminal;
use std::os::fd::AsFd;

const DEFAULT_WIDTH: usize = 80;
const MIN_WIDTH: usize = 20;
const MAX_WIDTH: usize = 1000;

/// Where a readout's width comes from.
#[derive(Clone, Copy)]
pub(crate) enum Width {
    /// `COLUMNS` where it is set, else the default: how every readout but
    /// `brokkr graph` has always been sized, so none of them moves.
    Columns,
    /// A `COLUMNS` that is a whole, nonzero number of cells, else the
    /// columns of the stdout the readout writes to where that is a
    /// terminal, else the default (#1167). Never the controlling
    /// terminal, which may be another width than a redirected stdout.
    Stdout,
}

/// How the terminal wants to be written to. Both facts come from the
/// environment; the rules that read them are pure.
pub(crate) struct Style {
    pub color: bool,
    pub width: usize,
}

impl Style {
    /// The style of a readout sized by [`Width::Columns`].
    pub(crate) fn detect() -> Style {
        Style::detect_for(Width::Columns)
    }

    /// The style of a readout sized by `width`, read from this process's
    /// stdout and environment.
    pub(crate) fn detect_for(width: Width) -> Style {
        let tty = std::io::stdout().is_terminal();
        Style::detect_in(tty, std::env::var_os, width, stdout_columns)
    }

    /// [`Style::detect_for`] over a given terminal, environment `var` and
    /// stdout's `columns`, which only [`Width::Stdout`] asks for, and
    /// only where stdout is a terminal.
    pub(crate) fn detect_in(
        tty: bool,
        var: impl Fn(&'static str) -> Option<OsString>,
        width: Width,
        columns: impl FnOnce() -> Option<u16>,
    ) -> Style {
        let set = var("COLUMNS").and_then(|value| value.into_string().ok());
        let width = match width {
            Width::Columns => width_from(set.as_deref()),
            Width::Stdout => measured(set.as_deref(), || tty.then(columns).flatten()),
        };
        Style {
            color: color_enabled(
                tty,
                var("NO_COLOR").is_some(),
                var("TERM") == Some("dumb".into()),
            ),
            width,
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

/// The columns of the window `fd` is, where it is a terminal that says.
pub(super) fn window_columns(fd: impl AsFd) -> Option<u16> {
    rustix::termios::tcgetwinsize(fd)
        .ok()
        .map(|size| size.ws_col)
}

fn stdout_columns() -> Option<u16> {
    window_columns(std::io::stdout())
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

/// [`Width::Stdout`]'s rule: `set` where it is a whole, nonzero number,
/// else what `measure` reads, nonzero, each clamped, else the default.
fn measured(set: Option<&str>, measure: impl FnOnce() -> Option<u16>) -> usize {
    let set = set.and_then(|text| text.parse::<usize>().ok());
    let width = set
        .filter(|&width| width > 0)
        .or_else(|| measure().map(usize::from).filter(|&width| width > 0));
    width.map_or(DEFAULT_WIDTH, |width| width.clamp(MIN_WIDTH, MAX_WIDTH))
}
