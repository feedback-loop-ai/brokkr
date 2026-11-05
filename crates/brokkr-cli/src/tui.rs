//! `brokkr tui` — the interactive, read-only console (decision 0014).
//!
//! A **third renderer** over `brokkr-view`'s models, never a fourth
//! derivation: every value drawn below is a model field, and the only
//! computation here is selecting, filtering, arranging and laying out.
//! A renderer may branch on a model field; it may not compute one.
//!
//! **Read-only, structurally.** Nothing in this module names a store, a
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
//!
//! **The seams** (#288). One submodule per responsibility the console
//! already had, in the order a frame is made: [`state`] holds the models
//! and the selection; [`keys`] translates a keypress and runs the pure
//! state machine over [`movement`]; [`footer`] says which keys are live;
//! [`style`] holds the sanitized constructors; [`panes`] lays out the
//! frame and draws the fleet and run panes; [`columns`] are the fleet's
//! dashboard and live columns beside its list; [`glyphs`], [`layout`] and
//! [`painter`] are the graph's vocabulary, its geometry and its painter;
//! [`seats`] and [`participant`] are the remaining panes; and
//! [`terminal`] is the shell. This root holds the module's one import
//! list and its constants, and every submodule reads them, and its
//! siblings, through `use super::*`: the vocabulary is named once.

use std::collections::BTreeSet;
use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use brokkr_core::policy::Severity;
use brokkr_view::{
    Column, Need, Node, Participant, Phase, RunRow, RunView, RunsView, Section, Standing,
};
use ratatui::backend::Backend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use ratatui::{Frame, Terminal};

use crate::render::{self, Safe, Tone};
use brokkr_view::transcript::{BlockKind, TranscriptRead, Turn, Unavailable};

/// Below this the frame cannot hold its panes, and a drawn frame would
/// be a corrupted one.
pub(crate) const MIN_WIDTH: u16 = 60;
pub(crate) const MIN_HEIGHT: u16 = 12;

/// Whether a frame of `width` x `height` holds the console at all. Below
/// either minimum the frame paints only "this terminal is too small", so
/// every pane is off it: the one test the painter, the startup refusal,
/// the status line's notice and a held movement's view all ask (753cL3's
/// C1, decision 0071 rulings 5 and 9).
pub(crate) fn fits(width: u16, height: u16) -> bool {
    width >= MIN_WIDTH && height >= MIN_HEIGHT
}

/// The input poll. Latency is bounded by the keypress, not the tick: an
/// operator holding `j` must not feel a quarter-second sleep.
const TICK: Duration = Duration::from_millis(250);

/// The fleet's slower cadence, ≈2s. Re-folding every event of every run
/// four times a second against a `brokkr run` holding the write lock is
/// the cost this constant exists to refuse (spec §6).
const RUNS_REFRESH_TICKS: usize = 8;

/// One `PageUp`/`PageDown` in list rows.
const PAGE: usize = 10;

/// The absolute rank today's participant renderer can display: its `u16`
/// scroll narrows every offset above it. The one home of the cutoff the
/// read chooses a page by, the movement narrows with and the pane paints
/// through, so the three cannot drift (L1, decision 0071 ruling 5).
pub(crate) const DISPLAY_RANK: usize = u16::MAX as usize;

/// The fleet list's columns, left to right (#491): how the run stands (a
/// glyph and its widest word, the ending `delivered-elsewhere`, decision
/// 0064 ruling 6), its verdict, its residual
/// (the widest, `critical`), its title, its age (`999h59m`) and its id.
/// The title may take what the frame leaves and takes no more than its
/// widest line (#503), and a list too narrow for the rest folds the age
/// and then the residual away (`fleet_widths`).
const FLEET_COLUMNS: [u16; 6] = [
    21,
    brokkr_view::VERDICT_COLUMNS as u16,
    8,
    brokkr_view::TITLE_COLUMNS as u16,
    7,
    ID_COLUMNS as u16,
];

/// The narrowest the fleet list stands beside another column: its two
/// borders, the five gaps between its columns, and the columns, so the
/// title's is a whole title wide. A wider list gives its titles the rest.
const LIST_COLUMNS: u16 = 2
    + 5
    + FLEET_COLUMNS[0]
    + FLEET_COLUMNS[1]
    + FLEET_COLUMNS[2]
    + FLEET_COLUMNS[3]
    + FLEET_COLUMNS[4]
    + FLEET_COLUMNS[5];

/// The fewest columns the fleet's title is left. With the age and the
/// residual folded, the list is exactly [`MIN_WIDTH`] wide at it.
const TITLE_MIN_COLUMNS: u16 = 12;

/// The narrowest the fleet's run dashboard is drawn (#503), borders
/// included: below it the dashboard is not on the frame and the footer
/// says how wide the frame must be. It wraps at whatever it is given.
const DASHBOARD_MIN: u16 = 72;

/// The narrowest the fleet's live or findings column is drawn (#503).
const LIVE_MIN: u16 = 62;

/// The width of a run id in the fleet list. An id longer than this is
/// shortened in the middle, and its minted hash — the last
/// [`ID_HASH_CHARS`] characters — is never cut: it is the only part
/// that tells two runs of one commission apart.
const ID_COLUMNS: usize = 14;

/// The hash the engine mints at the end of every run id.
const ID_HASH_CHARS: usize = 8;

/// One pulse frame per this many shell ticks: four frames × 2 × `TICK`
/// ≈ a two-second breath, the terminal's answer to the console's 1.8s
/// keyframe. **No new clock and no new wakeup**: [`drive`] already
/// redraws every `TICK` and already carries `Tui::ticks`, so animation
/// adds exactly zero draws and zero timers.
const PULSE_TICKS: usize = 2;
const PULSE_FRAMES: usize = 4;

mod columns;
mod footer;
mod glyphs;
mod keys;
mod layout;
mod movement;
mod painter;
mod panes;
mod participant;
mod reader;
mod seats;
mod settle;
mod state;
mod style;
mod terminal;

use self::columns::*;
use self::footer::*;
use self::glyphs::*;
use self::keys::*;
use self::layout::*;
use self::movement::*;
use self::painter::*;
use self::participant::*;
use self::seats::*;
use self::settle::*;
use self::state::*;
use self::style::*;
// No sibling reads the frame's panes or the shell beyond the entries
// re-exported below, a fleet row's title lines, which the list's room
// for its older runs counts, and the graph's rows, which the dashboard
// deals as the run level does (#508); the tests reach the rest through
// the root.
#[cfg(test)]
use self::panes::*;
use self::panes::{graph_rows, title_lines, tone_of, GRAPH_SHARE_MAX};
#[cfg(test)]
use self::terminal::*;

// What the rest of the crate reaches: the models the shell's refresh
// source builds, the draw path the budget frame measures, and the entry.
#[cfg(test)]
pub(crate) use self::keys::Step;
pub(crate) use self::movement::{asked_window, keeps};
pub(crate) use self::panes::draw;
pub use self::participant::transcript_surfaces_for_test;
pub(crate) use self::reader::behind;
#[cfg(test)]
pub(crate) use self::settle::{hold, settle_end, settle_when};
#[cfg(test)]
pub(crate) use self::state::{ask_offset, checkpoint_offset, Held, Level, HELD_BOUND};
pub(crate) use self::state::{
    asked_rows, Ask, DriveEnd, Refreshed, Scrolled, Subject, Tui, Unread, Views, Watched, Window,
};
pub(crate) use self::terminal::{production_ops, start, Closed, Session};

#[cfg(test)]
mod columns_tests;

#[cfg(test)]
mod fleet_tests;

#[cfg(test)]
mod reader_tests;

#[cfg(test)]
mod screens_tests;

#[cfg(test)]
pub(crate) use self::state::ask;

#[cfg(test)]
pub(crate) use self::screens_tests::{run_level_screens, screens, ENDS};

/// The real paging keys, for the run readers' key/read/draw proofs (#753):
/// the console's own key handler, not a second movement path.
#[cfg(test)]
pub(crate) fn page_key(tui: &mut Tui, views: &Views, down: bool) {
    let key = match down {
        true => Key::PageDown,
        false => Key::PageUp,
    };
    let _ = apply(tui, views, key);
}

/// The real `g` and Down keys, for the endpoints and within-row proofs.
#[cfg(test)]
pub(crate) fn top_key(tui: &mut Tui, views: &Views) {
    let _ = apply(tui, views, Key::Char('g'));
}

#[cfg(test)]
pub(crate) fn bottom_key(tui: &mut Tui, views: &Views) {
    let _ = apply(tui, views, Key::Char('G'));
}

#[cfg(test)]
pub(crate) fn tab_key(tui: &mut Tui, views: &Views) {
    let _ = apply(tui, views, Key::Tab);
}

#[cfg(test)]
pub(crate) fn down_key(tui: &mut Tui, views: &Views) {
    let _ = apply(tui, views, Key::Down);
}

#[cfg(test)]
pub(crate) fn up_key(tui: &mut Tui, views: &Views) {
    let _ = apply(tui, views, Key::Up);
}

/// A real character key, for the hearth and other bound keys (#753).
#[cfg(test)]
pub(crate) fn char_key(tui: &mut Tui, views: &Views, character: char) {
    let _ = apply(tui, views, Key::Char(character));
}

/// The real `Esc`, for the ladder and the held movement it drops.
#[cfg(test)]
pub(crate) fn escape_key(tui: &mut Tui, views: &Views) {
    let _ = apply(tui, views, Key::Escape);
}

/// A frame drawn by the console's own draw path, as the key/read/draw
/// proofs compare it with the whole read's.
#[cfg(test)]
pub(crate) fn frame_for_test(tui: &Tui, views: &Views, width: u16, height: u16) -> String {
    self::tests::frame_of(tui, views, width, height)
}

/// The live column alone, drawn at the width the real layout gives it, so
/// a wrapped live line is measured where the console draws it (R1).
#[cfg(test)]
pub(crate) fn live_frame_for_test(
    tui: &Tui,
    views: &Views,
    run: &str,
    width: u16,
    height: u16,
) -> String {
    let Some((_, live)) = layout(tui.toggles, width)
        .into_iter()
        .find(|(column, _)| *column == FleetColumn::Live)
    else {
        return String::new();
    };
    let Some(row) = views.runs.runs.iter().find(|row| key_of(row) == run) else {
        return String::new();
    };
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(live, height)).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            draw_live(frame, area, tui, views, row);
        })
        .unwrap();
    terminal.backend().to_string()
}

/// The console at a participant's level, by the real state machine's own
/// shape, for the continuation proof (R2).
#[cfg(test)]
pub(crate) fn participant_level(tui: &mut Tui, key: &str) {
    tui.level = Level::Participant;
    tui.seat = Some(key.to_string());
    tui.pane = 0;
}

#[cfg(test)]
mod coverage_tests;

#[cfg(test)]
mod held_tests;

#[cfg(test)]
mod snapshot_tests;

#[cfg(test)]
mod source_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod view_tests;
