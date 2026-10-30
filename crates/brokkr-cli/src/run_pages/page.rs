//! The bounded window the console holds of a participant's checkpoints
//! (#753; decision 0089 ruling 9): rows oldest first by seq, the cursors
//! either side of the window, and how many rows of its screen's order
//! stand before it. The live column reads newest first; the participant
//! level reads oldest first, as its pane paints (decision 0089, amended
//! 2026-10-28). One module, so the window's arithmetic and the console's
//! coordinates are read and proved in one place.

use std::collections::BTreeMap;

use brokkr_view::CheckpointRow;

/// A held window of one participant's checkpoints, oldest first by seq:
/// [`WINDOW`] rows at most around what the screen shows, the cursor for
/// the next page in the read's direction, and how many rows of the
/// screen's order stand before the window.
#[derive(Clone, Default)]
pub(super) struct Page {
    pub(super) rows: Vec<(u64, CheckpointRow)>,
    /// The cursor for the next page in the screen's read direction: the
    /// older page for the newest-first live column, the newer page for
    /// the oldest-first participant level.
    pub(super) next: Option<u64>,
    /// The older page of an oldest-first screen; none where the window
    /// already holds the stream's oldest row.
    pub(super) older: Option<u64>,
    /// How many rows of the screen's order stand before `rows`.
    pub(super) start: usize,
}

impl Page {
    /// The last rank the window reaches in the screen's order.
    pub(super) fn seen(&self) -> usize {
        self.start + self.rows.len()
    }

    /// A fresh page at the stream's oldest row: its rows, the cursor past
    /// them, and no older page.
    pub(super) fn first(rows: Vec<(u64, CheckpointRow)>, next: Option<u64>) -> Page {
        Page {
            rows,
            next,
            older: None,
            start: 0,
        }
    }
}

/// Each participant's page, by its key, and how many rows of the screen's
/// order stand before the merged window a detail pane shows. A detail
/// pane's pages are one window cut from every member's stream at the same
/// seq boundary, so the merged rows are contiguous however the members'
/// seqs interleave (R1).
#[derive(Default)]
pub(super) struct Checkpoints {
    pub(super) pages: BTreeMap<String, Page>,
    pub(super) start: usize,
    /// The stream's whole rank, where the screen asked for its absolute
    /// end: the shell resolves the end key's request from it (M3).
    pub(super) total: Option<usize>,
}

impl Checkpoints {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn get(&self, key: &str) -> Option<&Page> {
        self.pages.get(key)
    }
}

/// Ruling 7's page of a participant's checkpoints: 50 rows.
pub(super) const PAGE: usize = 50;

/// The rows a screen holds from its page at once: four pages, so a pane
/// taller than one page and the row it scrolled to are both held, and a
/// deep scroll cuts what it passed instead of keeping the stream.
pub(super) const WINDOW: usize = PAGE * 4;

/// How many rows of `asked`'s order the pages `held` stand past, so a
/// screen switch or a scroll above its window opens the run again rather
/// than advancing the wrong cursor (decision 0071 rulings 2 and 9).
pub(super) fn held_start(held: &Checkpoints) -> usize {
    held.start
}
