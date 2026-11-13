//! The dump's line-property closure (#1052 fix round 1052g8): no line may
//! leave a quote, a non-empty flow collection or an empty sequence item
//! open.
//!
//! The pass is independent of the row grammar and of its block bookkeeping.
//! Under YAML only a quoted scalar or a flow collection can continue onto a
//! line indented at or under its parent; js-yaml 4.3.2, dsh's reader, has a
//! third carrier: it folds a less-indented `- ` line into an empty sequence
//! item. A block scalar's and a plain multi-line scalar's content ends at
//! the first non-blank line at or under the parent. So if no line, read as a
//! structure line at its own position, leaves one of those three carriers,
//! no line can carry a following row's keys away. The pass judges no value
//! and admits nothing the row grammar refuses; it only refuses a line that
//! leaves something open.
//!
//! A line is read after its leading ASCII spaces and any `- `, `? ` or `: `
//! markers. A bare `-` refuses, and so does a last `- ` marker followed by
//! nothing or by a comment: an empty sequence item. A bare `?` or `:` ends
//! the line. A node strips every `!` or `&` property (to an ASCII space or a
//! flow indicator), then reads a quoted scalar (which must close on the
//! line), an empty flow collection (exactly `[]` or `{}`; any other flow
//! refuses), a block-scalar header, a comment, or a plain scalar (whose
//! first `: ` starts its value and whose first ` #` starts a comment). An
//! alias is read as a plain scalar, so `*a: "x` is a key whose value is
//! read, and only a property token ends at a flow indicator. After every
//! node kind, ASCII spaces are skipped and a `: ` starts a value read by
//! the same rule.

use super::super::Composition;

/// Refuse `Unreadable` if any line of `text` leaves a quote, a non-empty
/// flow collection or an empty sequence item open.
pub(crate) fn check(text: &str) -> Result<(), Composition> {
    for line in text.lines() {
        if line_open(line) {
            return Err(Composition::Unreadable);
        }
    }
    Ok(())
}

/// The line after its indicators: a node to read, or a refusal for an empty
/// sequence item.
enum Body<'a> {
    Node(&'a str),
    Refuse,
}

/// Whether `line`, read as a structure line at its own position, leaves a
/// quote, a non-empty flow collection or an empty sequence item open.
fn line_open(line: &str) -> bool {
    match strip_indicators(line.trim_start_matches(' ')) {
        Body::Node(body) => node_open(body),
        Body::Refuse => true,
    }
}

/// Strip every leading `- `, `? ` or `: ` marker, with the spaces after
/// each. A bare `-` and an empty sequence item (the last marker a `- ` with
/// nothing or only a comment after it) refuse. A bare `?` or `:` falls
/// through to a plain scalar, which leaves nothing open.
fn strip_indicators(mut body: &str) -> Body<'_> {
    let mut dash = false;
    loop {
        if let Some(rest) = body.strip_prefix("- ") {
            dash = true;
            body = rest.trim_start_matches(' ');
            continue;
        }
        if let Some(rest) = body.strip_prefix("? ") {
            dash = false;
            body = rest.trim_start_matches(' ');
            continue;
        }
        if let Some(rest) = body.strip_prefix(": ") {
            dash = false;
            body = rest.trim_start_matches(' ');
            continue;
        }
        if body == "-" {
            return Body::Refuse;
        }
        break;
    }
    if dash && (body.is_empty() || body.starts_with('#')) {
        return Body::Refuse;
    }
    Body::Node(body)
}

/// Whether the node at the start of `value` leaves one of the three carriers
/// open. A `!` or `&` property is stripped first, then the node's own form
/// decides; an alias is not special, so it is read as a plain scalar.
fn node_open(value: &str) -> bool {
    let value = strip_properties(value);
    match value.chars().next() {
        Some('\'' | '"') => quoted_open(value),
        Some('[' | '{') => flow_open(value),
        Some('|' | '>') | Some('#') => false,
        _ => plain_open(value),
    }
}

/// Whether the quoted scalar at the start of `value` is left open, or is a
/// quoted key whose value after its colon is left open. A closed quoted
/// scalar followed by anything but a colon and a value leaves nothing open.
fn quoted_open(value: &str) -> bool {
    match super::scan_quoted(value) {
        Some(end) => after_node(value, end),
        None => true,
    }
}

/// Whether the flow collection at the start of `value` leaves something
/// open. Only the empty collections `[]` and `{}` are admitted: the two
/// characters are the whole node, followed by the line's end, a space or a
/// colon. Anything else is not the dump's flow and refuses, since an open
/// one is a carrier. The value after a colon is read by [`after_node`].
fn flow_open(value: &str) -> bool {
    if !(value.starts_with("[]") || value.starts_with("{}")) {
        return true;
    }
    match value[2..].chars().next() {
        None | Some(' ' | ':') => after_node(value, 2),
        Some(_) => true,
    }
}

/// Whether the text after a node, at byte index `end`, is a colon and a
/// value left open by [`node_open`]. ASCII spaces are skipped; anything but
/// `:` followed by a space or the line's end leaves nothing open.
fn after_node(value: &str, end: usize) -> bool {
    let rest = value[end..].trim_start_matches(' ');
    match rest.strip_prefix(':') {
        Some(after) if after.starts_with(' ') => node_open(after),
        _ => false,
    }
}

/// Whether a plain scalar leaves something open. Its content ends at the
/// first `: ` (a key, whose value is read by [`node_open`]) or the first
/// ` #` (a comment). A quote inside it is plain content.
fn plain_open(value: &str) -> bool {
    for (index, c) in value.char_indices() {
        if c == ':' && colon_ends_key(value, index + 1) {
            return node_open(&value[index + 1..]);
        }
        if c == ' ' && value[index + 1..].starts_with('#') {
            return false;
        }
    }
    false
}

/// Whether the byte at `next` is an ASCII space, which ends a key's colon.
/// A colon at the line's end is read by [`node_open`] on an empty value,
/// which leaves nothing open either way.
fn colon_ends_key(value: &str, next: usize) -> bool {
    value.as_bytes().get(next) == Some(&b' ')
}

/// Strip every leading `!` or `&` property: the indicator, its run, and the
/// spaces after it. A property token ends at an ASCII space or a flow
/// indicator (`,`, `[`, `]`, `{` or `}`). A token that runs to the line's
/// end leaves an empty node.
fn strip_properties(mut value: &str) -> &str {
    loop {
        value = value.trim_start_matches(' ');
        let Some(first) = value.chars().next() else {
            return value;
        };
        if !matches!(first, '!' | '&') {
            return value;
        }
        value = &value[token_end(value, 0)..];
    }
}

/// The byte index at which the `!` or `&` property token at `start` ends:
/// the first ASCII space or flow indicator after the indicator, or the
/// line's end.
fn token_end(value: &str, start: usize) -> usize {
    for (offset, c) in value[start..].char_indices().skip(1) {
        if c == ' ' || matches!(c, ',' | '[' | ']' | '{' | '}') {
            return start + offset;
        }
    }
    value.len()
}
