//! The reading of a MANAGED tool list — a list the engine composes (rebuild
//! unit 10; design D6) — moved here from `grammar.rs`, which sits at its line
//! baseline, when a managed name learned the served name of an MCP broker,
//! `mcp__cap-<capability>` (decision 0065 slice two, U9b1d; controller
//! ruling R4 of 2026-11-01).

/// The separators a managed tool list may be joined with. A comma is what
/// the shipped mappings declare; Claude's own splitter also splits at a
/// space outside parentheses, so a space is not a separator the engine
/// may join with.
pub fn managed_separator(separator: &str) -> Result<char, &'static str> {
    match separator {
        "," => Ok(','),
        _ => Err("is not the one separator a managed tool list is joined with, ','"),
    }
}

/// The bounds of a managed tool list.
const PATTERNS_MAX: usize = 64;
const TOOL_NAME_MAX: usize = 128;
const SPECIFIER_MAX: usize = 256;

/// The patterns of one MANAGED tool-list value — a list the engine
/// composes — or the fixed cause it is refused for (rebuild unit 10;
/// design D6). The empty value is an explicit empty list, distinct from
/// no list. Otherwise the value is patterns joined by single commas, each
/// a plain tool name ([`plain`]) optionally followed by one parenthesized
/// specifier. The specifier may hold spaces, `:`, `*`, `/` and `.`, but no
/// parenthesis, comma, quote, backslash or control character, because each
/// is a place where this reading and the harness's own splitter could
/// disagree. A wildcard never stands in a tool name, an empty pattern (a
/// doubled, leading or trailing comma) refuses, and so does a space outside
/// the parentheses. The cause never echoes the value.
pub fn managed_patterns(value: &str) -> Result<Vec<&str>, &'static str> {
    const EMPTY: &str = "joins an empty pattern: a doubled, leading or trailing separator";
    const NAME: &str = "names a tool that is not a plain name of ASCII letters, digits and '_' \
                        leading with a letter, within 128 bytes";
    const SPECIFIER: &str = "carries a specifier that is not one parenthesized, nonempty run \
                             within 256 bytes without a parenthesis, comma, quote, backslash or \
                             control character";
    const COUNT: &str = "joins more than 64 patterns";
    if value.is_empty() {
        return Ok(Vec::new());
    }
    let patterns: Vec<&str> = value.split(',').collect();
    if patterns.len() > PATTERNS_MAX {
        return Err(COUNT);
    }
    for pattern in &patterns {
        if pattern.is_empty() {
            return Err(EMPTY);
        }
        let (name, specifier) = match pattern.split_once('(') {
            Some((name, rest)) => (name, Some(rest)),
            None => (*pattern, None),
        };
        if !plain(name) {
            return Err(NAME);
        }
        if let Some(rest) = specifier {
            let inner = rest.strip_suffix(')').ok_or(SPECIFIER)?;
            if inner.is_empty()
                || inner.len() > SPECIFIER_MAX
                || inner
                    .chars()
                    .any(|c| matches!(c, '(' | ')' | ',' | '"' | '\'' | '\\') || c.is_control())
            {
                return Err(SPECIFIER);
            }
        }
    }
    Ok(patterns)
}

/// Whether `name` is a plain managed tool name: within 128 bytes, leading
/// with an ASCII letter, and of ASCII letters, digits and `_`. The server
/// part of an `mcp__` name — what stands between `mcp__` and the next `__`,
/// or the end — may also hold `-`, never as its first character, so a
/// broker's served name `mcp__cap-<capability>` reads (U9b1d; ruling R4);
/// no other part of any name does. The refusal's text stays true of every
/// name refused here, since this admits a superset of what it admitted.
fn plain(name: &str) -> bool {
    let alphabet = |part: &str| part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let named = match name.strip_prefix("mcp__") {
        Some(rest) => {
            let (server, tool) = rest.split_once("__").unwrap_or((rest, ""));
            !server.starts_with('-') && alphabet(&server.replace('-', "")) && alphabet(tool)
        }
        None => alphabet(name),
    };
    named && name.len() <= TOOL_NAME_MAX && name.starts_with(|c: char| c.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests;
