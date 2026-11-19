//! The one table writer the terminal readouts share (decision 0071 ruling
//! 5): columns sized to their widest cell, the header's included. The
//! seats block sizes its columns here; `brokkr graph` draws its states and
//! rules as whole tables here (#1167), measured in the terminal's cells.

use std::cmp::Reverse;

use super::{push_line, tint, Safe, Style, BOLD};
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
fn sized<R: AsRef<[Safe]>>(heads: &[&str], rows: &[R], measure: fn(&str) -> usize) -> Vec<usize> {
    let mut widths: Vec<usize> = heads.iter().map(|head| measure(head)).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row.as_ref()) {
            *width = (*width).max(measure(cell.as_str()));
        }
    }
    widths
}

/// Each of the `heads`' columns as wide as its widest cell in `rows`, the
/// header's included, in `char`s as the seats block pads them. A row may
/// carry cells past the last sized column.
pub(crate) fn widths<R: AsRef<[Safe]>>(heads: &[&str], rows: &[R]) -> Vec<usize> {
    sized(heads, rows, |text| text.chars().count())
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
    let heads: Vec<&str> = columns.iter().map(|column| column.head).collect();
    let mut widths = sized(&heads, rows, width_of);
    let gap = gap(&widths, style.width);
    let used = widths.iter().sum::<usize>() + gap.len() * (widths.len() - 1);
    give_way(columns, &mut widths, used.saturating_sub(style.width));
    let heads: Vec<Safe> = heads.iter().map(|head| Safe::new(head)).collect();
    let mut out = String::new();
    let header = line(&heads, columns, &widths, gap);
    push_line(&mut out, &tint(&header, BOLD, style));
    for row in rows {
        push_line(&mut out, &line(row, columns, &widths, gap));
    }
    out
}

/// `text` cut to `max` terminal cells, the cut marked with `…`. The
/// TUI's clamp also bounds `char`s, as its buffer spends a cell on each;
/// a printed row spends only the cells the terminal draws, so a
/// decomposed accent costs nothing here (#1167). `max` is at least one,
/// a header's width.
fn fit(text: &str, max: usize) -> String {
    if width_of(text) <= max {
        return text.to_string();
    }
    let kept: String = text
        .chars()
        .scan(String::new(), |seen, character| {
            seen.push(character);
            (width_of(seen) < max).then_some(character)
        })
        .collect();
    kept + "…"
}

/// One row's cells, each clamped to its column's width in cells and set
/// in it, with no trailing space for a bold header to carry.
fn line(cells: &[Safe], columns: &[Column], widths: &[usize], gap: &str) -> String {
    let pieces: Vec<String> = cells
        .iter()
        .zip(columns.iter().zip(widths))
        .map(|(cell, (column, &width))| {
            let text = fit(cell.as_str(), width);
            let pad = " ".repeat(width.saturating_sub(width_of(&text)));
            match column.align {
                Align::Left => text + &pad,
                Align::Right => pad + &text,
            }
        })
        .collect();
    pieces.join(gap).trim_end().to_string()
}
