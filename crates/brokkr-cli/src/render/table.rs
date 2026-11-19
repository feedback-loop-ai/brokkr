//! The one table writer the terminal readouts share (decision 0071 ruling
//! 5): columns sized to their widest cell, the header's included. The
//! seats block sizes its columns here; `brokkr graph` draws its states and
//! rules as whole tables here (#1167).

use super::{push_line, tint, Safe, Style, BOLD};

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
}

/// What separates two columns: two spaces where the table fits the
/// terminal at two, else one, so the gaps give way before any name does.
fn gap(widths: &[usize], width: usize) -> &'static str {
    let cells = widths.iter().sum::<usize>();
    match cells + 2 * (widths.len() - 1) <= width {
        true => "  ",
        false => " ",
    }
}

/// Each of the `heads`' columns as wide as its widest cell in `rows`, the
/// header's included. A row may carry cells past the last sized column.
pub(crate) fn widths<R: AsRef<[Safe]>>(heads: &[&str], rows: &[R]) -> Vec<usize> {
    let mut widths: Vec<usize> = heads.iter().map(|head| head.chars().count()).collect();
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row.as_ref()) {
            *width = (*width).max(cell.width());
        }
    }
    widths
}

/// `rows` under a header row, bold where the terminal takes colour; a
/// table with no row is its header alone. A table wider than the
/// terminal narrows its gaps to one space, then gives the overflow up
/// from column `clamped` alone, down to its header's width, so every
/// other cell stays whole.
pub(crate) fn table(
    columns: &[Column],
    rows: &[Vec<Safe>],
    clamped: usize,
    style: &Style,
) -> String {
    let heads: Vec<&str> = columns.iter().map(|column| column.head).collect();
    let mut widths = widths(&heads, rows);
    let gap = gap(&widths, style.width);
    let used = widths.iter().sum::<usize>() + gap.len() * (widths.len() - 1);
    let over = used.saturating_sub(style.width);
    let floor = heads[clamped].chars().count();
    widths[clamped] = widths[clamped].saturating_sub(over).max(floor);
    let heads: Vec<Safe> = heads.iter().map(|head| Safe::new(head)).collect();
    let mut out = String::new();
    let header = line(&heads, columns, &widths, gap);
    push_line(&mut out, &tint(&header, BOLD, style));
    for row in rows {
        push_line(&mut out, &line(row, columns, &widths, gap));
    }
    out
}

/// One row's cells, each clamped to its column's width and set in it,
/// with no trailing space for a bold header to carry.
fn line(cells: &[Safe], columns: &[Column], widths: &[usize], gap: &str) -> String {
    let pieces: Vec<String> = cells
        .iter()
        .zip(columns.iter().zip(widths))
        .map(|(cell, (column, &width))| {
            let text = Safe::new(&brokkr_view::clamp(cell.as_str(), width));
            match column.align {
                Align::Left => text.padded(width),
                Align::Right => format!("{:>width$}", text.as_str()),
            }
        })
        .collect();
    pieces.join(gap).trim_end().to_string()
}
