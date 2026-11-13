//! The dump's line-property closure (#1052 fix round 1052g7): no line may
//! leave a quote or a flow collection open.
//!
//! The pass is independent of the row grammar and of its block bookkeeping.
//! Under YAML only a quoted scalar or a flow collection can continue onto a
//! line indented at or under its parent; a block scalar's and a plain
//! multi-line scalar's content ends at the first non-blank line at or under
//! the parent. So if no line, read as a structure line at its own position,
//! leaves a quote or a flow collection open, no line can carry a following
//! row's keys away. The pass judges no value and admits nothing the row
//! grammar refuses; it only refuses a line that leaves something open.
//!
//! A line is read after its leading ASCII spaces and any `- ` or `? `
//! markers; a bare marker ends it, and a `#` after the markers is a comment.
//! A node strips any `!` or `&` token, then reads a quoted scalar (which
//! must close on the line), a flow collection (whose brackets must balance
//! and whose quoted nodes must close), or a plain scalar (whose first `: `
//! introduces a value read by the same rule). Quotes inside a plain scalar
//! are plain content, not scanned.

use super::super::Composition;

/// Refuse `Unreadable` if any line of `text` leaves a quote or a flow
/// collection open. Empty lines are the only lines the pass never reads.
pub(crate) fn check(text: &str) -> Result<(), Composition> {
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        if line_open(line) {
            return Err(Composition::Unreadable);
        }
    }
    Ok(())
}

/// Whether `line`, read as a structure line at its own position, leaves a
/// quote or a flow collection open. Leading ASCII spaces and every `- ` or
/// `? ` marker are stripped first; a bare marker ends the line, and a `#`
/// after the markers makes the rest a comment.
fn line_open(line: &str) -> bool {
    let mut body = line.trim_start_matches(' ');
    loop {
        if let Some(rest) = body.strip_prefix("- ") {
            body = rest.trim_start_matches(' ');
            continue;
        }
        if let Some(rest) = body.strip_prefix("? ") {
            body = rest.trim_start_matches(' ');
            continue;
        }
        if body == "-" || body == "?" {
            return false;
        }
        break;
    }
    if body.starts_with('#') {
        return false;
    }
    node_open(body)
}

/// Whether the node at the start of `value` leaves a quote or a flow
/// collection open. A tag or anchor token is stripped first, then the
/// node's own form decides: a quoted scalar must close, a flow collection
/// must balance, and a plain scalar is content up to its first ` #`, with
/// its first `: ` introducing a value read by this same rule.
fn node_open(value: &str) -> bool {
    let value = strip_tags(value);
    match value.chars().next() {
        Some('\'' | '"') => quoted_open(value),
        Some('[' | '{') => flow_end(value).is_none(),
        _ => plain_open(value),
    }
}

/// Whether the quoted scalar at the start of `value` is left open, or is a
/// quoted key whose value read after its colon is left open. A closed
/// quoted scalar that names no key, or is followed by anything but a colon
/// and a value, leaves nothing open.
fn quoted_open(value: &str) -> bool {
    let Some(end) = super::scan_quoted(value) else {
        return true;
    };
    let rest = value[end..].trim_start_matches(' ');
    match rest.strip_prefix(':') {
        Some(after) if after.is_empty() || after.starts_with(' ') => node_open(after),
        _ => false,
    }
}

/// Strip every leading `!` or `&` token: the indicator, its non-space run,
/// and the one space after it. Leading ASCII spaces are stripped before
/// each token. A token that runs to the line's end leaves an empty node.
fn strip_tags(mut value: &str) -> &str {
    loop {
        value = value.trim_start_matches(' ');
        let Some(first) = value.chars().next() else {
            return value;
        };
        if !matches!(first, '!' | '&') {
            return value;
        }
        let mut end = value.len();
        for (index, c) in value.char_indices().skip(1) {
            if c == ' ' {
                end = index + 1;
                break;
            }
        }
        value = &value[end..];
    }
}

/// Whether a plain scalar leaves something open. Content ends at the first
/// ` #`, and the text after the first `: ` within that content is a node
/// read by [`node_open`]. A quote inside the plain scalar is content.
fn plain_open(value: &str) -> bool {
    let head = match value.find(" #") {
        Some(cut) => &value[..cut],
        None => value,
    };
    match head.find(": ") {
        Some(colon) => node_open(&head[colon + 2..]),
        None => false,
    }
}

/// The byte index just past the closing bracket of the flow collection at
/// the start of `value`, or `None` when its brackets do not balance or one
/// of its quoted nodes is left open. A quote starts a flow node at the
/// collection's start, after `[`, `{`, `,`, `:` or a space, and is scanned
/// by the same quote rule the grammar uses.
fn flow_end(value: &str) -> Option<usize> {
    let mut index = 0;
    let mut depth = 0isize;
    let mut previous = None;
    while index < value.len() {
        let c = value[index..].chars().next()?;
        let quote_start = matches!(c, '\'' | '"')
            && (index == 0 || matches!(previous, Some('[' | '{' | ',' | ':' | ' ')));
        if quote_start {
            index += super::scan_quoted(&value[index..])?;
            previous = Some(c);
            continue;
        }
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + c.len_utf8());
                }
            }
            _ => {}
        }
        previous = Some(c);
        index += c.len_utf8();
    }
    None
}
