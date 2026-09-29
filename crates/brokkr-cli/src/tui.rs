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
//! frame and draws the fleet and run panes; [`glyphs`], [`layout`] and
//! [`painter`] are the graph's vocabulary, its geometry and its painter;
//! [`seats`] and [`participant`] are the remaining panes; and
//! [`terminal`] is the shell. This root holds the module's one import
//! list and its constants, and every submodule reads them, and its
//! siblings, through `use super::*`: the vocabulary is named once.

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
use brokkr_view::transcript::{BlockKind, LegacyProvenance, TranscriptRead, Turn, Unavailable};

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

mod footer;
mod glyphs;
mod keys;
mod layout;
mod movement;
mod painter;
mod panes;
mod participant;
mod seats;
mod state;
mod style;
mod terminal;

use self::footer::*;
use self::glyphs::*;
use self::keys::*;
use self::layout::*;
use self::movement::*;
use self::painter::*;
use self::participant::*;
use self::seats::*;
use self::state::*;
use self::style::*;
// No sibling reads the frame's panes or the shell beyond the entries
// re-exported below; the tests reach the rest through the root.
#[cfg(test)]
use self::panes::*;
#[cfg(test)]
use self::terminal::*;

// What the rest of the crate reaches: the models the shell's refresh
// source builds, the draw path the budget frame measures, and the entry.
pub(crate) use self::panes::draw;
pub use self::participant::transcript_surfaces_for_test;
pub(crate) use self::state::{Ask, Refreshed, Subject, Tui, Views};
pub(crate) use self::terminal::{production_ops, start};

#[cfg(test)]
mod snapshot_tests;

#[cfg(test)]
mod source_tests;

#[cfg(test)]
mod tests;
