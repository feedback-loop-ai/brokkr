//! The one table writer the terminal readouts share (decision 0071 ruling
//! 5): columns sized to their widest cell, the header's included. The
//! seats block sizes its columns here; `brokkr graph` draws its states and
//! rules as whole tables here (#1167), each cell sized, cut and padded
//! in the terminal's cells by one walk, [`fit_by`].

use std::borrow::Cow;
use std::cmp::Reverse;

use ratatui::style::Style as Plain;
use ratatui::text::Span;

use super::{push_line, scalars, tint, Safe, Style, BOLD};
use crate::tui::width_of;

/// How a column's cells sit in it: text to the left, numbers to the right.
#[derive(Clone, Copy)]
pub(crate) enum Align {
    Left,
    Right,
}

/// One column of a table: its header and how its cells sit.
pub(crate) struct Column {
    head: &'static str,
    align: Align,
}

impl Column {
    /// A text column, its cells to the left.
    pub(crate) const fn left(head: &'static str) -> Column {
        Column {
            head,
            align: Align::Left,
        }
    }

    /// A column of numbers, its cells to the right.
    pub(crate) const fn right(head: &'static str) -> Column {
        Column {
            head,
            align: Align::Right,
        }
    }

    /// Whether the column gives width up when the table overflows: text
    /// does, a number never.
    const fn gives_way(&self) -> bool {
        match self.align {
            Align::Left => true,
            Align::Right => false,
        }
    }
}

/// The narrowest a text column gives way to, in cells, its ellipsis
/// included; a column already narrower keeps its width.
const FLOOR: usize = 8;

/// What separates two columns: two spaces where the table fits the
/// terminal at two, else one, so the gaps give way before any name does.
fn gap(widths: &[usize], width: usize) -> &'static str {
    let cells = widths.iter().sum::<usize>();
    match cells + 2 * (widths.len() - 1) <= width {
        true => "  ",
        false => " ",
    }
}

/// Each of the `heads`' columns as wide as its widest cell in `rows` by
/// `measure`, the header's included.
fn sized<R: AsRef<[Safe]>>(
    heads: &[&str],
    rows: &[R],
    measure: &mut impl FnMut(&str) -> usize,
) -> Vec<usize> {
    let mut widths: Vec<usize> = heads.iter().map(|head| measure(head)).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row.as_ref()) {
            *width = (*width).max(measure(cell.as_str()));
        }
    }
    widths
}

/// Each of the `heads`' columns as wide as its widest cell in `rows`, the
/// header's included, in `char`s by the one measure the seats block pads
/// with. A row may carry cells past the last sized column.
pub(crate) fn widths<R: AsRef<[Safe]>>(heads: &[&str], rows: &[R]) -> Vec<usize> {
    sized(heads, rows, &mut scalars)
}

/// `over` cells given up from the text columns, one at a time from the
/// widest above its floor, the first where two tie, until none is left
/// or every text column stands at its floor.
fn give_way(columns: &[Column], widths: &mut [usize], over: usize) {
    let floors: Vec<usize> = widths.iter().map(|width| (*width).min(FLOOR)).collect();
    for _ in 0..over {
        let widest = (0..widths.len())
            .filter(|&at| columns[at].gives_way() && widths[at] > floors[at])
            .max_by_key(|&at| (widths[at], Reverse(at)));
        let Some(at) = widest else {
            break;
        };
        widths[at] -= 1;
    }
}

/// `rows` under a header row, bold where the terminal takes colour; a
/// table with no row is its header alone. A table wider than the
/// terminal narrows its gaps to one space, then gives the overflow up
/// from its text columns, widest first, each down to [`FLOOR`] cells, so
/// every number stays whole.
pub(crate) fn table(columns: &[Column], rows: &[Vec<Safe>], style: &Style) -> String {
    table_by(columns, rows, style, &mut width_of)
}

/// [`table`] with `measure` for [`width_of`]: every cell is sized, cut
/// and padded by one [`fit_by`] walk, so a test can count what a whole
/// table measures (#1167).
pub(super) fn table_by(
    columns: &[Column],
    rows: &[Vec<Safe>],
    style: &Style,
    measure: &mut impl FnMut(&str) -> usize,
) -> String {
    let heads: Vec<&str> = columns.iter().map(|column| column.head).collect();
    let mut widths = sized(&heads, rows, &mut |text| {
        fit_by(text, usize::MAX, measure).cells
    });
    let gap = gap(&widths, style.width);
    let used = widths.iter().sum::<usize>() + gap.len() * (widths.len() - 1);
    give_way(columns, &mut widths, used.saturating_sub(style.width));
    let heads: Vec<Safe> = heads.iter().map(|head| Safe::new(head)).collect();
    let set = Set {
        columns,
        widths: &widths,
        gap,
    };
    let mut out = String::new();
    let header = line(&heads, &set, measure);
    push_line(&mut out, &tint(&header, BOLD, style));
    for row in rows {
        push_line(&mut out, &line(row, &set, measure));
    }
    out
}

/// What a cell keeps of its text in its column, and the terminal cells
/// that takes, its ellipsis included: the padding reads `cells`, so the
/// kept text is never walked again (#1167).
pub(super) struct Fitted<'a> {
    pub(super) text: Cow<'a, str>,
    pub(super) cells: usize,
}

/// `text` cut to `max` terminal cells, the cut marked with `…`, in one
/// walk of its graphemes. Each grapheme costs what [`width_of`] (by
/// `measure`) adds when it follows the one before, `measure(previous +
/// grapheme) - measure(previous)`: a width can hang on its whole
/// sequence (`♥` then U+FE0F draws two cells) or on its neighbour (a
/// Lisu tone letter after another draws none), and the TUI's measure
/// reads both, so the walk sums to the whole text's width wherever a
/// width hangs on no more than the grapheme before, and no prefix is
/// measured again. It keeps what fits the budget less the
/// ellipsis, and cuts there the moment the text passes `max`.
/// The TUI's clamp also bounds `char`s, as its buffer spends a cell on
/// each; a printed row spends only the cells the terminal draws, so a
/// decomposed accent costs nothing here. `max` is at least one, a
/// header's width, and `usize::MAX` measures without a cut.
pub(super) fn fit_by<'a>(
    text: &'a str,
    max: usize,
    measure: &mut impl FnMut(&str) -> usize,
) -> Fitted<'a> {
    let budget = max.saturating_sub(1);
    let (mut spent, mut kept, mut kept_cells) = (0, String::new(), 0);
    let (mut previous, mut alone, mut pair) = ("", 0, String::new());
    let span = Span::raw(text);
    for grapheme in span.styled_graphemes(Plain::new()) {
        pair.clear();
        pair.push_str(previous);
        pair.push_str(grapheme.symbol);
        spent += measure(&pair).saturating_sub(alone);
        if spent > max {
            let cells = kept_cells + measure("…");
            let text = Cow::Owned(kept + "…");
            return Fitted { text, cells };
        }
        if spent <= budget {
            kept.push_str(grapheme.symbol);
            kept_cells = spent;
        }
        (previous, alone) = (grapheme.symbol, measure(grapheme.symbol));
    }
    let text = Cow::Borrowed(text);
    Fitted { text, cells: spent }
}

/// How a table sets each row: its columns, their widths and the gap
/// between them.
struct Set<'a> {
    columns: &'a [Column],
    widths: &'a [usize],
    gap: &'a str,
}

/// One row's cells, each clamped to its column's width in cells and set
/// in it, with no trailing space for a bold header to carry.
fn line(cells: &[Safe], set: &Set<'_>, measure: &mut impl FnMut(&str) -> usize) -> String {
    let pieces: Vec<String> = cells
        .iter()
        .zip(set.columns.iter().zip(set.widths))
        .map(|(cell, (column, &width))| {
            let Fitted { text, cells } = fit_by(cell.as_str(), width, measure);
            let (text, pad) = (text.into_owned(), " ".repeat(width.saturating_sub(cells)));
            match column.align {
                Align::Left => text + &pad,
                Align::Right => pad + &text,
            }
        })
        .collect();
    pieces.join(set.gap).trim_end().to_string()
}
