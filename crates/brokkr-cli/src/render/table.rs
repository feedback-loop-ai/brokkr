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

/// What separates two columns.
const GAP: &str = "  ";

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

/// `rows` under a header row, bold where the terminal takes colour, or
/// nothing when there is no row. A table wider than the terminal gives
/// the overflow up from column `clamped` alone, down to its header's
/// width, so every other cell stays whole.
pub(crate) fn table(
    columns: &[Column],
    rows: &[Vec<Safe>],
    clamped: usize,
    style: &Style,
) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let heads: Vec<&str> = columns.iter().map(|column| column.head).collect();
    let mut widths = widths(&heads, rows);
    let used = widths.iter().sum::<usize>() + GAP.len() * (widths.len() - 1);
    let over = used.saturating_sub(style.width);
    let floor = heads[clamped].chars().count();
    widths[clamped] = widths[clamped].saturating_sub(over).max(floor);
    let heads: Vec<Safe> = heads.iter().map(|head| Safe::new(head)).collect();
    let mut out = String::new();
    push_line(
        &mut out,
        &tint(&line(&heads, columns, &widths), BOLD, style),
    );
    for row in rows {
        push_line(&mut out, &line(row, columns, &widths));
    }
    out
}

/// One row's cells, each clamped to its column's width and set in it.
fn line(cells: &[Safe], columns: &[Column], widths: &[usize]) -> String {
    let pieces: Vec<String> = cells
        .iter()
        .zip(columns.iter().zip(widths))
        .map(|(cell, (column, width))| {
            let text = brokkr_view::clamp(cell.as_str(), *width);
            let pad = " ".repeat(width.saturating_sub(text.chars().count()));
            match column.align {
                Align::Left => text + &pad,
                Align::Right => pad + &text,
            }
        })
        .collect();
    pieces.join(GAP).trim_end().to_string()
}
