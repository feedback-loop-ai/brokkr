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
//! line), an empty flow collection (`[]` or `{}` only; any other flow
//! refuses), an alias, a block-scalar header, a comment, or a plain scalar
//! (whose first `: ` starts its value and whose first ` #` starts a
//! comment). After every node kind, ASCII spaces are skipped and a `: `
//! starts a value read by the same rule.

use super::super::Composition;

/// Refuse `Unreadable` if any line of `text` leaves a quote, a non-empty
/// flow collection or an empty sequence item open. Empty lines are the only
/// lines the pass never reads.
pub(crate) fn check(text: &str) -> Result<(), Composition> {
    for line in text.lines() {
        if !line.is_empty() && line_open(line) {
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
/// nothing or only a comment after it) refuse; a bare `?` or `:` ends the
/// line.
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
        if body == "?" || body == ":" {
            return Body::Node("");
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
/// decides.
fn node_open(value: &str) -> bool {
    let value = strip_properties(value);
    match value.chars().next() {
        Some('\'' | '"') => quoted_open(value),
        Some('[' | '{') => flow_open(value),
        Some('*') => alias_open(value),
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
/// open. Only the empty collections `[]` and `{}` are admitted, and only
/// their value after a colon is read; any other flow refuses, since none is
/// a dump line and an open one is a carrier.
fn flow_open(value: &str) -> bool {
    if value.starts_with("[]") || value.starts_with("{}") {
        after_node(value, 2)
    } else {
        true
    }
}

/// Whether the alias at the start of `value` is followed by a value that is
/// left open.
fn alias_open(value: &str) -> bool {
    after_node(value, token_end(value, 0))
}

/// Whether the text after a node, at byte index `end`, is a colon and a
/// value left open by [`node_open`]. ASCII spaces are skipped; anything but
/// `:` followed by a space or the line's end leaves nothing open.
fn after_node(value: &str, end: usize) -> bool {
    let rest = value[end..].trim_start_matches(' ');
    match rest.strip_prefix(':') {
        Some(after) if after.is_empty() || after.starts_with(' ') => node_open(after),
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

/// Whether the byte at `next` ends a key's colon: the line's end, or an
/// ASCII space.
fn colon_ends_key(value: &str, next: usize) -> bool {
    next == value.len() || value.as_bytes()[next] == b' '
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

/// The byte index at which the `!`, `&` or `*` token at `start` ends: the
/// first ASCII space or flow indicator after the indicator, or the line's
/// end.
fn token_end(value: &str, start: usize) -> usize {
    for (offset, c) in value[start..].char_indices().skip(1) {
        if c == ' ' || matches!(c, ',' | '[' | ']' | '{' | '}') {
            return start + offset;
        }
    }
    value.len()
}
