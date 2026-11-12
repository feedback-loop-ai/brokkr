//! The composed dump's one row grammar (#1052 round 1052g5): every line of
//! the whole dump is read by one closed reader, not only the
//! `sandbox-policy` span.
//!
//! The dump is one sequence of `- id: <v>` rows. Before anything else each
//! line is refused if it holds a character a YAML reader may break on or
//! that the gate will not read as text: a C0 control other than tab, DEL,
//! a C1 control, LS, PS, a BOM, U+FFFE or U+FFFF. Then only an empty line,
//! a line of ASCII spaces and tabs, or a line whose first character after
//! those is `#` is skipped. A column-zero `- id: <v>` starts a row and any
//! other non-skip column-zero line refuses. A row's two-space level is
//! closed to `  name: <v>`, `  disabled: <v>`, `  config:` and `  inject:`,
//! each at most once. `config:` or `inject:` opens a block; a line indented
//! three or more spaces is admitted only while a block is open, the
//! `sandbox-policy` block reads only one `    mode: <w>`, and every other
//! row's block stays unread. A one-space line and a non-skip line led by a
//! tab refuse.

use super::super::{HANDS_ID, SANDBOX_POLICY};
use super::{Composition, Row};

/// Every row of a `--dump-config` sequence, read by the one grammar, or
/// [`Composition::Unreadable`] for any line the grammar does not admit.
pub(crate) fn rows(bytes: &[u8]) -> Result<Vec<Row>, Composition> {
    let text = std::str::from_utf8(bytes).map_err(|_| Composition::Unreadable)?;
    let mut rows = Vec::new();
    let mut current: Option<(Row, RowState)> = None;
    for line in text.lines() {
        if has_break_or_control(line) {
            return Err(Composition::Unreadable);
        }
        if is_skipped(line) {
            continue;
        }
        if let Some(rest) = line.strip_prefix("- id: ") {
            if let Some((row, _)) = current.take() {
                rows.push(row);
            }
            current = Some((new_row(rest), RowState::default()));
            continue;
        }
        let Some((row, state)) = current.as_mut() else {
            return Err(Composition::Unreadable);
        };
        read_line(row, state, line)?;
    }
    if let Some((row, _)) = current.take() {
        rows.push(row);
    }
    finish(rows)
}

/// Finish the row sequence: a dump with no row, or a row that names no
/// package, is not the sequence dsh writes.
fn finish(rows: Vec<Row>) -> Result<Vec<Row>, Composition> {
    if rows.is_empty() || rows.iter().any(|row| row.name.is_empty()) {
        return Err(Composition::Unreadable);
    }
    Ok(rows)
}

/// One row's state while its lines are read: which of its two-space keys
/// have been seen, and which of its blocks (`config:` or `inject:`) is
/// open. The seen set and the block state have one home, so the policy row
/// and every other row are read by one shape.
#[derive(Default)]
struct RowState {
    seen: Vec<Key>,
    block: Block,
}

/// One of the row's four two-space keys, held once in [`RowState::seen`],
/// so a second occurrence is a second entry and refuses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Name,
    Disabled,
    Config,
    Inject,
}

impl RowState {
    /// Mark `key` seen, or refuse the row's second occurrence of it.
    fn admit(&mut self, key: Key) -> Result<(), Composition> {
        if self.seen.contains(&key) {
            return Err(Composition::Unreadable);
        }
        self.seen.push(key);
        Ok(())
    }
}

/// The row's one open block, if any. `name:`, `disabled:` and the next row
/// close it; a skipped line leaves it as it is.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Block {
    #[default]
    None,
    Config,
    Inject,
}

/// One row started by a column-zero `- id: <v>`.
fn new_row(rest: &str) -> Row {
    Row {
        id: unquote(ascii_trim(rest)),
        name: String::new(),
        disabled: false,
        mode: None,
    }
}

/// One admitted row line: a line at the row's two-space level, or a deeper
/// line while one of its blocks is open.
fn read_line(row: &mut Row, state: &mut RowState, line: &str) -> Result<(), Composition> {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    match spaces {
        0 | 1 => Err(Composition::Unreadable),
        2 => read_key(row, state, &line[2..]),
        deeper => read_deep(row, state, line, deeper),
    }
}

/// One two-space line. Only the four keys the dump writes are admitted,
/// each at most once, and `config:`/`inject:` are bare. A quoted or spaced
/// key, a flow-style value, an unknown key and a second key refuse.
fn read_key(row: &mut Row, state: &mut RowState, key: &str) -> Result<(), Composition> {
    if let Some(value) = key.strip_prefix("name: ") {
        state.admit(Key::Name)?;
        state.block = Block::None;
        row.name = unquote(ascii_trim(value));
        return Ok(());
    }
    if key.starts_with("disabled:") {
        state.admit(Key::Disabled)?;
        state.block = Block::None;
        return read_disabled(row, key);
    }
    if key == "config:" {
        state.admit(Key::Config)?;
        state.block = Block::Config;
        return Ok(());
    }
    if key == "inject:" {
        if row.id == SANDBOX_POLICY {
            return Err(Composition::Unreadable);
        }
        state.admit(Key::Inject)?;
        state.block = Block::Inject;
        return Ok(());
    }
    Err(Composition::Unreadable)
}

/// One line deeper than the row's two-space level. It is admitted only
/// while a block is open; the `sandbox-policy` block reads only its one
/// `    mode: <w>` line, and every other row's block stays unread.
fn read_deep(
    row: &mut Row,
    state: &RowState,
    line: &str,
    spaces: usize,
) -> Result<(), Composition> {
    if state.block == Block::None {
        return Err(Composition::Unreadable);
    }
    if row.id != SANDBOX_POLICY {
        return Ok(());
    }
    if spaces != 4 {
        return Err(Composition::Unreadable);
    }
    read_mode(row, &line[4..])
}

/// The one `    mode: <word>` line a `sandbox-policy` `config` block may
/// carry, read only while its block is open. A missing space, a deeper
/// indent, a `Mode:` spelling or a second mode refuses the tree. The word
/// is taken with [`ascii_trim`], so a non-ASCII-padded mode is not the
/// word the gate compares.
fn read_mode(row: &mut Row, rest: &str) -> Result<(), Composition> {
    let Some(value) = rest.strip_prefix("mode: ") else {
        return Err(Composition::Unreadable);
    };
    if row.mode.is_some() {
        return Err(Composition::Unreadable);
    }
    row.mode = Some(unquote(ascii_trim(value)));
    Ok(())
}

/// One `disabled:` line of the row being read, from its two-space key
/// without the leading spaces. The other rows keep the judge's reading:
/// only the exact word `true` is disabled, so `false` and a `!!js`
/// expression are enabled and a row that must be disabled refuses
/// `Enabled`. The `sandbox-policy` row is admitted only when the word is
/// exactly `false`: its mode must be in force. The hands row is admitted
/// only when the word is exactly `false` too; every other word marks it
/// disabled here, so the judge's [`Composition::HandsDisabled`] refuses
/// it with its unchanged text. A word jammed against the colon, or a bare
/// key, reads as the empty word, as it always has.
fn read_disabled(row: &mut Row, key: &str) -> Result<(), Composition> {
    let word = key
        .strip_prefix("disabled: ")
        .map(ascii_trim)
        .unwrap_or_default();
    row.disabled = if row.id == HANDS_ID {
        word != "false"
    } else {
        word == "true"
    };
    if row.id == SANDBOX_POLICY && word != "false" {
        return Err(Composition::PolicyDisabled { id: row.id.clone() });
    }
    Ok(())
}

/// One YAML single-quoted scalar's body, or the bare text as written.
fn unquote(value: &str) -> String {
    match value
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
    {
        Some(body) => body.replace("''", "'"),
        None => value.to_string(),
    }
}

/// Whether `line` holds a character a YAML reader may read as a line break
/// or that the gate will not read as text: a C0 control other than tab,
/// DEL, a C1 control, LS, PS, a BOM, or U+FFFE/U+FFFF (outside YAML's
/// printable set). A bare CR is a YAML break, NEL, LS and PS are YAML 1.1
/// breaks, and none of them is skip material.
fn has_break_or_control(line: &str) -> bool {
    line.chars().any(|c| {
        matches!(
            c,
            '\u{0000}'..='\u{0008}'
                | '\u{000a}'..='\u{001f}'
                | '\u{007f}'
                | '\u{0080}'..='\u{009f}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{feff}'
                | '\u{fffe}'
                | '\u{ffff}'
        )
    })
}

/// Whether the reader skips `line` everywhere: it is empty, it holds only
/// ASCII spaces and tabs, or its first character after those is `#`. Text
/// after the `#` is comment content, non-ASCII characters included. No
/// other white space is skip material, so a line of only NBSP, or NBSP
/// then `#`, is refused rather than skipped.
fn is_skipped(line: &str) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    rest.is_empty() || rest.starts_with('#')
}

/// The one trim of a value read from the dump: ASCII spaces and tabs
/// only. Unicode white space is not trimmed, so a padded word cannot be
/// read as the word the gate compares.
fn ascii_trim(value: &str) -> &str {
    value.trim_matches([' ', '\t'])
}
