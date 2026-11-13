//! The composed dump's one row grammar (#1052 round 1052g5): every line of
//! the whole dump is read by one closed reader, not only the
//! `sandbox-policy` span.
//!
//! The dump is one sequence of `- id: <v>` rows. The refusal order is the
//! dump-wide break, control and tab rule, then the line-property closure
//! ([`carriers`]), then this row grammar. A line is refused if it holds a
//! character a YAML reader may break on or that the gate will not read as
//! text: a C0 control other than tab, a tab anywhere, DEL, a C1 control, LS,
//! PS, a BOM, U+FFFE or U+FFFF. The closure then refuses any line that
//! leaves a quote or a flow collection open, independent of the grammar's
//! block bookkeeping. Only after both does the grammar read the line: only
//! an empty line, a line of ASCII spaces, or a line whose first character
//! after those is `#` is skipped. A column-zero `- id: <v>` starts a row and
//! any other non-skip column-zero line refuses. A row's two-space level is
//! closed to `  name: <v>`, `  disabled: <v>`, `  config:` and `  inject:`,
//! each at most once. `config:` or `inject:` opens a block; a line indented
//! three or more spaces is admitted only while a block is open, and the
//! `sandbox-policy` config block reads only one `    mode: <w>`.
//!
//! Every other deeper line of an unread block must be a line js-yaml's
//! dump writes, so no quoted scalar, flow collection or anchor can carry a
//! later row's keys away. A block-scalar header opens content: every
//! following blank line, or line indented more than the header's node
//! column, is skipped unread, and the first non-blank line at or above that
//! column ends the content and is read normally. The node column is the
//! key's column when a key was read and the last marker's column for a bare
//! item. Every other deeper line is read for structure: sequence markers
//! are stripped, a quoted or plain key is
//! followed by one value, and that value must be empty, a block-scalar
//! header, an empty flow collection, a quoted scalar that closes on its
//! line, or a plain scalar. An open quote, an open flow collection, an
//! anchor, an alias, an explicit key, a directive, a reserved indicator and
//! a block-scalar indentation indicator refuse. The `sandbox-policy`
//! `inject:` block is admitted only as the one measured
//! `    - headlessStartup` line. A one-space line refuses.

use super::super::{HANDS_ID, SANDBOX_POLICY};
use super::{Composition, Row};

pub(super) mod carriers;

/// The one deeper line the measured 0.2.2-rc.1 `sandbox-policy` `inject:`
/// block carries.
const POLICY_INJECT_ITEM: &str = "    - headlessStartup";

/// Every row of a `--dump-config` sequence, read by the one grammar, or
/// [`Composition::Unreadable`] for any line the grammar does not admit.
pub(crate) fn rows(bytes: &[u8]) -> Result<Vec<Row>, Composition> {
    let text = std::str::from_utf8(bytes).map_err(|_| Composition::Unreadable)?;
    if text.lines().any(has_break_or_control) {
        return Err(Composition::Unreadable);
    }
    super::closure(text)?;
    let mut rows = Vec::new();
    let mut current: Option<(Row, RowState)> = None;
    for line in text.lines() {
        if is_scalar_content(&mut current, line) {
            continue;
        }
        if is_skipped(line) {
            continue;
        }
        if let Some(rest) = line.strip_prefix("- id: ") {
            finish_row(&mut rows, current.take())?;
            current = Some((new_row(rest)?, RowState::default()));
            continue;
        }
        let Some((row, state)) = current.as_mut() else {
            return Err(Composition::Unreadable);
        };
        read_line(row, state, line)?;
    }
    finish_row(&mut rows, current.take())?;
    finish(rows)
}

/// Whether `line` is content of the block scalar the current row has open:
/// every blank line, and every line indented more than the header's node
/// column. The first non-blank line at or above that column closes the
/// content and is left for the ordinary grammar. A line before any row, or
/// with no block scalar open, is never content.
fn is_scalar_content(current: &mut Option<(Row, RowState)>, line: &str) -> bool {
    let Some((_, state)) = current.as_mut() else {
        return false;
    };
    let Some(header) = state.scalar else {
        return false;
    };
    if line.trim_matches(' ').is_empty() {
        return true;
    }
    let spaces = line.len() - line.trim_start_matches(' ').len();
    if spaces > header {
        return true;
    }
    state.scalar = None;
    false
}

/// Push the row being read, unless the `sandbox-policy` row opened an
/// `inject:` block without its measured item. The block is admitted only as
/// that one line, so an empty one refuses the tree.
fn finish_row(rows: &mut Vec<Row>, current: Option<(Row, RowState)>) -> Result<(), Composition> {
    let Some((row, state)) = current else {
        return Ok(());
    };
    if row.id == SANDBOX_POLICY && state.seen.contains(&Key::Inject) && !state.policy_inject_item {
        return Err(Composition::Unreadable);
    }
    rows.push(row);
    Ok(())
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
/// have been seen, which of its blocks (`config:` or `inject:`) is open,
/// the indent of the block-scalar header whose content is being skipped,
/// and whether the `sandbox-policy` row's measured inject item has been
/// read. The seen set and the block state have one home, so the policy row
/// and every other row are read by one shape.
#[derive(Default)]
struct RowState {
    seen: Vec<Key>,
    block: Block,
    /// The node column of the block-scalar header whose content is being
    /// skipped, or `None` when no content is open: the key's column when a
    /// key was read, and the last marker's column for a bare item.
    scalar: Option<usize>,
    /// Whether the `sandbox-policy` row's one measured `inject:` item has
    /// been read.
    policy_inject_item: bool,
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

/// One row started by a column-zero `- id: <v>`. The value is read by the
/// same block value rule every row value uses, so an open quote, a block
/// header, an anchor, an alias or an open flow refuses.
fn new_row(rest: &str) -> Result<Row, Composition> {
    check_row_value(rest)?;
    Ok(Row {
        id: unquote(ascii_trim(rest)),
        name: String::new(),
        disabled: false,
        mode: None,
    })
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
/// key, a flow-style value, an unknown key and a second key refuse. The
/// `sandbox-policy` row's `inject:` block is admitted only as its one
/// measured item; its `config:` block is the span read above.
fn read_key(row: &mut Row, state: &mut RowState, key: &str) -> Result<(), Composition> {
    if let Some(value) = key.strip_prefix("name: ") {
        state.admit(Key::Name)?;
        state.block = Block::None;
        check_row_value(value)?;
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
        state.admit(Key::Inject)?;
        state.block = Block::Inject;
        return Ok(());
    }
    Err(Composition::Unreadable)
}

/// One line deeper than the row's two-space level. It is admitted only
/// while a block is open. The `sandbox-policy` `config:` block reads only
/// its one `    mode: <w>` line, and its `inject:` block only the one
/// measured item. Every other row's blocks are unread, and each of their
/// lines is read for the structure js-yaml's dump writes.
fn read_deep(
    row: &mut Row,
    state: &mut RowState,
    line: &str,
    spaces: usize,
) -> Result<(), Composition> {
    match (row.id == SANDBOX_POLICY, state.block) {
        (_, Block::None) => Err(Composition::Unreadable),
        (true, Block::Config) => {
            if spaces != 4 {
                return Err(Composition::Unreadable);
            }
            read_mode(row, &line[4..])
        }
        (false, Block::Config) => read_unread(state, line, spaces),
        (true, Block::Inject) => read_policy_inject(state, line),
        (false, Block::Inject) => read_unread(state, line, spaces),
    }
}

/// The only line the `sandbox-policy` `inject:` block may carry, once: the
/// measured `    - headlessStartup`. Any other line, a second item
/// included, refuses `Unreadable`. An empty block is refused when its row
/// closes, by [`finish_row`].
fn read_policy_inject(state: &mut RowState, line: &str) -> Result<(), Composition> {
    if line != POLICY_INJECT_ITEM || state.policy_inject_item {
        return Err(Composition::Unreadable);
    }
    state.policy_inject_item = true;
    Ok(())
}

/// Which node a block-scalar header belongs to, so its content is measured
/// from the node's own column rather than the line's leading spaces.
#[derive(Clone, Copy)]
enum Header {
    /// A header read as a key's value: content must be indented more than
    /// the key's column.
    Key,
    /// A header that is a sequence item's own body: content must be
    /// indented more than the last `- ` marker's column.
    Item,
}

/// One line of an unread block. It must be a line js-yaml's dump writes: a
/// block-scalar header opens content that [`is_scalar_content`] skips, and
/// every other line must read as a sequence item, a key and value, or a
/// bare value. A block-scalar header records its node column, so content
/// ends where the next line returns to it.
fn read_unread(state: &mut RowState, line: &str, spaces: usize) -> Result<(), Composition> {
    let (body, width) = strip_markers(&line[spaces..])?;
    let Some(header) = read_unread_body(body)? else {
        return Ok(());
    };
    let column = match header {
        Header::Key => spaces + width,
        Header::Item => spaces + width.saturating_sub(2),
    };
    state.scalar = Some(column);
    Ok(())
}

/// Whether `body`, one unread block line without its indentation and
/// markers, is a block-scalar header, and whether that header is a key's
/// value or a bare sequence item. A quoted scalar followed by nothing but
/// spaces is an item; spaces before a quoted key's colon are allowed. Every
/// other admitted shape returns `None`.
fn read_unread_body(body: &str) -> Result<Option<Header>, Composition> {
    if body.starts_with('\'') || body.starts_with('"') {
        let end = scan_quoted(body).ok_or(Composition::Unreadable)?;
        let rest = body[end..].trim_start_matches(' ');
        if rest.is_empty() {
            return Ok(None);
        }
        let value = rest.strip_prefix(':').ok_or(Composition::Unreadable)?;
        return read_value(after_colon(value)?).map(|header| header.then_some(Header::Key));
    }
    match split_plain_key(body) {
        Some((key, rest)) => {
            if !plain_key_ok(key) {
                return Err(Composition::Unreadable);
            }
            read_value(after_colon(rest)?).map(|header| header.then_some(Header::Key))
        }
        None => read_value(body).map(|header| header.then_some(Header::Item)),
    }
}

/// Strip every leading `- ` sequence marker, returning the item's body and
/// the width of the markers stripped, so the caller can record the key's
/// column or the last marker's column. A bare `-` is a YAML sequence marker
/// too, but not the one a dump writes, so it refuses.
fn strip_markers(mut body: &str) -> Result<(&str, usize), Composition> {
    let mut width = 0;
    while let Some(rest) = body.strip_prefix("- ") {
        body = rest;
        width += 2;
    }
    if body == "-" {
        return Err(Composition::Unreadable);
    }
    Ok((body, width))
}

/// Find a plain key's separator: the first `:` followed by end of line or
/// an ASCII space. A colon followed by any other character is plain
/// content. A tab is refused dump-wide before this reader runs, so it is not
/// a separator here.
fn split_plain_key(body: &str) -> Option<(&str, &str)> {
    let bytes = body.as_bytes();
    (0..bytes.len())
        .find(|&i| bytes[i] == b':' && (i + 1 == bytes.len() || bytes[i + 1] == b' '))
        .map(|i| (&body[..i], &body[i + 1..]))
}

/// Whether `key` is a plain scalar's start: non-empty, and not one of the
/// indicators a dump quotes or refuses.
fn plain_key_ok(key: &str) -> bool {
    match key.chars().next() {
        Some(first) => !refused_plain_start(first),
        None => false,
    }
}

/// The one space after a key's colon: a value is empty, or follows exactly
/// one ASCII space. A word jammed against the colon is not the dump's line.
fn after_colon(rest: &str) -> Result<&str, Composition> {
    if rest.is_empty() {
        return Ok("");
    }
    rest.strip_prefix(' ').ok_or(Composition::Unreadable)
}

/// One value after an optional tag. `true` means the value is a
/// block-scalar header, whose content [`is_scalar_content`] then skips.
/// The admitted values are the dump's: empty, a block-scalar header, an
/// empty flow collection, a quoted scalar that closes on this line, or a
/// plain scalar the dump writes. An open quote or flow collection, an
/// anchor, an alias, a directive, a reserved indicator, an indentation
/// indicator and the plain markers a reader takes for structure refuse.
fn read_value(value: &str) -> Result<bool, Composition> {
    let value = strip_tag(value);
    if value.starts_with(' ') {
        return Err(Composition::Unreadable);
    }
    if matches!(value, "|" | "|-" | "|+" | ">" | ">-" | ">+") {
        return Ok(true);
    }
    if value == "[]" || value == "{}" {
        return Ok(false);
    }
    match value.chars().next() {
        None => Ok(false),
        Some('\'' | '"') => {
            let end = scan_quoted(value).ok_or(Composition::Unreadable)?;
            match value[end..].chars().all(|c| c == ' ') {
                true => Ok(false),
                false => Err(Composition::Unreadable),
            }
        }
        Some(first) if refused_plain_start(first) => Err(Composition::Unreadable),
        Some(_) if refused_plain_value(value) => Err(Composition::Unreadable),
        Some(_) => Ok(false),
    }
}

/// Check one row value (`- id:`, `name:` or `disabled:`) with the same
/// block value rule an unread block's value uses, and refuse a block-scalar
/// header: a row value is one scalar, never a header whose content a later
/// line could continue. An open quote, an open flow collection, an anchor,
/// an alias and a plain marker refuse here, before the value's word is read.
fn check_row_value(value: &str) -> Result<(), Composition> {
    match read_value(value)? {
        true => Err(Composition::Unreadable),
        false => Ok(()),
    }
}

/// Strip one optional tag: `!` and the non-space run after it, then the one
/// space before the value. A `!` with no space before the line's end is not
/// the dump's tag shape and is left for the refusal.
fn strip_tag(value: &str) -> &str {
    if let Some(rest) = value.strip_prefix('!') {
        if let Some(space) = rest.find(' ') {
            return &rest[space + 1..];
        }
    }
    value
}

/// Whether `first` may not start a plain scalar: the indicators a dump
/// quotes, or the ones a reader treats as structure (flow, anchor, alias,
/// block scalar, explicit key, tag, directive, reserved, backquote, and a
/// flow closer or comma).
fn refused_plain_start(first: char) -> bool {
    matches!(
        first,
        '\'' | '"'
            | '['
            | '{'
            | ']'
            | '}'
            | ','
            | '&'
            | '*'
            | '|'
            | '>'
            | '?'
            | '!'
            | '%'
            | '@'
            | '`'
    )
}

/// Whether a plain value is one the dump never writes and a reader reads as
/// structure: an explicit `-` or `:` marker on its own or followed by a
/// space, or a `: ` key separator inside. `?` is not repeated here:
/// [`refused_plain_start`] already refuses every value that starts with
/// `?`. `-1` and `a-b` stay plain values.
fn refused_plain_value(value: &str) -> bool {
    matches!(value, "-" | ":")
        || value.starts_with("- ")
        || value.starts_with(": ")
        || value.contains(": ")
}

/// The byte index just past the closing quote of the quoted scalar at the
/// start of `value`, or `None` when the line ends inside it. In single
/// quotes `''` is an escaped quote; in double quotes a backslash escapes
/// the next character and a trailing backslash is a continuation, so the
/// scalar is open.
fn scan_quoted(value: &str) -> Option<usize> {
    let quote = value.chars().next().unwrap_or_default();
    let mut chars = value.char_indices().peekable();
    chars.next();
    loop {
        let (index, c) = chars.next()?;
        if quote == '\'' {
            if c == '\'' {
                match chars.peek() {
                    Some((_, '\'')) => {
                        chars.next();
                    }
                    _ => return Some(index + 1),
                }
            }
        } else if c == '\\' {
            chars.next()?;
        } else if c == '"' {
            return Some(index + 1);
        }
    }
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
/// without the leading spaces. The value is read by the same block value
/// rule as `name:` and `- id:`, so an open quote refuses before its word is
/// taken. The other rows keep the judge's reading:
/// only the exact word `true` is disabled, so `false` and a `!!js`
/// expression are enabled and a row that must be disabled refuses
/// `Enabled`. The `sandbox-policy` row is admitted only when the word is
/// exactly `false`: its mode must be in force. The hands row is admitted
/// only when the word is exactly `false` too; every other word marks it
/// disabled here, so the judge's [`Composition::HandsDisabled`] refuses
/// it with its unchanged text. A word jammed against the colon, or a bare
/// key, reads as the empty word, as it always has.
fn read_disabled(row: &mut Row, key: &str) -> Result<(), Composition> {
    let word = match key.strip_prefix("disabled: ") {
        Some(value) => {
            check_row_value(value)?;
            ascii_trim(value)
        }
        None => "",
    };
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
/// or that the gate will not read as text: a C0 control other than tab, a
/// tab anywhere, DEL, a C1 control, LS, PS, a BOM, or U+FFFE/U+FFFF (outside
/// YAML's printable set). A bare CR is a YAML break, NEL, LS and PS are YAML
/// 1.1 breaks, and none of them is skip material. A tab is YAML white space
/// with its own separation rules (a tag ends at any white space, a tab can
/// open a quoted value), so the dump's language refuses it everywhere as one
/// member of this set.
fn has_break_or_control(line: &str) -> bool {
    line.chars().any(|c| {
        matches!(
            c,
            '\u{0000}'..='\u{0008}'
                | '\u{0009}'
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
/// ASCII spaces, or its first character after those is `#`. Text after the
/// `#` is comment content, non-ASCII characters included. No other white
/// space is skip material, so a line of only NBSP, or NBSP then `#`, is
/// refused rather than skipped. A tab is refused dump-wide first, so it is
/// not skip material here.
fn is_skipped(line: &str) -> bool {
    let rest = line.trim_start_matches(' ');
    rest.is_empty() || rest.starts_with('#')
}

/// The one trim of a value read from the dump: ASCII spaces only, since a
/// tab is refused dump-wide. Unicode white space is not trimmed, so a padded
/// word cannot be read as the word the gate compares.
fn ascii_trim(value: &str) -> &str {
    value.trim_matches(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One admitted row, so a two-space value can be read into it.
    fn row() -> Row {
        Row {
            id: "tool-bash".into(),
            name: String::new(),
            disabled: false,
            mode: None,
        }
    }

    /// C2's row grammar: a `disabled:` value is read by the block value
    /// rule, so an open quote, a block header, an anchor, an alias and an
    /// open flow refuse before the word is taken. Reading the bare word
    /// again admits the quote, which is the removal control.
    #[test]
    fn a_two_space_value_refuses_an_open_quote_or_a_header() {
        let mut row = row();
        for (what, key) in [
            ("a double quote", "disabled: \"x"),
            ("a single quote", "disabled: 'x"),
            ("a block header", "disabled: >-"),
            ("an anchor", "disabled: &a value"),
            ("an alias", "disabled: *a"),
            ("an open flow", "disabled: [a,"),
        ] {
            assert_eq!(
                read_disabled(&mut row, key),
                Err(Composition::Unreadable),
                "{what}"
            );
        }
        assert_eq!(read_disabled(&mut row, "disabled: true"), Ok(()));
        assert!(row.disabled);
        assert_eq!(read_disabled(&mut row, "disabled: !!js true"), Ok(()));
        assert!(!row.disabled);
    }

    /// C2's other two row values: `- id:` and `name:` read by the same
    /// block value rule. A closed quote is admitted and unquoted; an open
    /// quote and a block header refuse.
    #[test]
    fn a_row_id_and_name_refuse_an_open_quote() {
        for (what, value) in [
            ("a double quote", "\"x"),
            ("a single quote", "'x"),
            ("a block header", ">-"),
        ] {
            assert_eq!(
                new_row(value).err(),
                Some(Composition::Unreadable),
                "{what}"
            );
        }
        assert_eq!(new_row("tool-bash").unwrap().id, "tool-bash");

        let mut row = row();
        let mut state = RowState::default();
        for (what, key) in [
            ("a double quote", "name: \"x"),
            ("a single quote", "name: 'x"),
            ("a block header", "name: >-"),
        ] {
            let mut state = RowState::default();
            assert_eq!(
                read_key(&mut row, &mut state, key),
                Err(Composition::Unreadable),
                "{what}"
            );
        }
        assert_eq!(
            read_key(&mut row, &mut state, "name: '@deepseek-ai/dsh-tool-bash'"),
            Ok(())
        );
        assert_eq!(row.name, "@deepseek-ai/dsh-tool-bash");
    }
}
