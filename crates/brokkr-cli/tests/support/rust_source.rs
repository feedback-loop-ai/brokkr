//! A reader of the attributes and `cfg!` invocations in Rust source, for
//! the test files that judge them: `suppressions.rs` counts lint
//! suppressions, `hosts.rs` refuses a Windows conditional, and
//! `layering::test_targets` finds a module's `#[test]` functions by them.
//! `tests/it.rs` declares this once, so the lexer has one home.
//!
//! The lexer skips comments and every string and char literal, so `#[`
//! inside a string is not an attribute, and it reads each unit whole
//! across lines, so a multi-line attribute, a `cfg_attr` and an inner
//! `#![..]` are each one unit. Text it cannot read is refused rather than
//! skipped.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

/// Advances past one string or char literal starting at `at`, or returns
/// `at` unchanged when there is none (a lifetime is not a literal).
pub(crate) fn skip_literal(s: &[char], at: usize) -> Result<usize, String> {
    if at >= s.len() {
        return Ok(at);
    }
    let mut i = at;
    if s[i] == 'b' && i + 1 < s.len() && matches!(s[i + 1], '"' | '\'' | 'r') {
        i += 1;
    }
    if s[i] == 'r' && i + 1 < s.len() && matches!(s[i + 1], '"' | '#') {
        let hashes = s[i + 1..].iter().take_while(|&&c| c == '#').count();
        let open = i + 1 + hashes;
        if s.get(open) != Some(&'"') {
            return Ok(at);
        }
        let close: String = std::iter::once('"')
            .chain("#".repeat(hashes).chars())
            .collect();
        let rest: String = s[open + 1..].iter().collect();
        let end = rest.find(&close).ok_or("an unterminated raw string")?;
        return Ok(open + 1 + rest[..end].chars().count() + close.chars().count());
    }
    match s[i] {
        '"' => {
            let mut j = i + 1;
            while j < s.len() && s[j] != '"' {
                j += if s[j] == '\\' { 2 } else { 1 };
            }
            if j >= s.len() {
                return Err("an unterminated string".into());
            }
            Ok(j + 1)
        }
        '\'' if s.get(i + 1) == Some(&'\\') => {
            let end = s
                .get(i + 3..)
                .and_then(|rest| rest.iter().position(|&c| c == '\''));
            Ok(i + 4 + end.ok_or("an unterminated char literal")?)
        }
        '\'' if s.get(i + 2) == Some(&'\'') => Ok(i + 3),
        _ => Ok(at),
    }
}

/// Advances past a comment at `at`, or returns `at` when there is none.
fn skip_comment(s: &[char], at: usize) -> Result<usize, String> {
    let Some(&first) = s.get(at) else {
        return Ok(at);
    };
    match (first, s.get(at + 1)) {
        ('/', Some('/')) => Ok(s[at..]
            .iter()
            .position(|&c| c == '\n')
            .map_or(s.len(), |p| at + p)),
        ('/', Some('*')) => {
            let (mut depth, mut i) = (1, at + 2);
            while depth > 0 {
                match (s.get(i), s.get(i + 1)) {
                    (Some('/'), Some('*')) => (depth, i) = (depth + 1, i + 2),
                    (Some('*'), Some('/')) => (depth, i) = (depth - 1, i + 2),
                    (Some(_), _) => i += 1,
                    (None, _) => return Err("an unterminated block comment".into()),
                }
            }
            Ok(i)
        }
        _ => Ok(at),
    }
}

/// Advances past whitespace and comments at `at`.
fn skip_trivia(s: &[char], at: usize) -> Result<usize, String> {
    let mut i = at;
    loop {
        let next = skip_comment(s, i)?;
        if next != i {
            i = next;
        } else if s.get(i).is_some_and(|c| c.is_whitespace()) {
            i += 1;
        } else {
            return Ok(i);
        }
    }
}

/// The index just past the `[` that opens an attribute at `at`: `#`, an
/// optional `!`, and whitespace or comments between any of them, as rustc
/// reads the three tokens. `None` when no attribute starts there.
fn attribute_open(s: &[char], at: usize) -> Result<Option<usize>, String> {
    if s[at] != '#' {
        return Ok(None);
    }
    let mut i = skip_trivia(s, at + 1)?;
    if s.get(i) == Some(&'!') {
        i = skip_trivia(s, i + 1)?;
    }
    Ok((s.get(i) == Some(&'[')).then_some(i + 1))
}

/// The index just past the bracket that opens a `cfg!` invocation at `at`:
/// the word `cfg`, not the tail of a longer one, then `!` and any bracket,
/// with whitespace or comments between. `None` when none starts there.
fn cfg_open(s: &[char], at: usize) -> Result<Option<usize>, String> {
    let in_a_word = at > 0 && (s[at - 1].is_alphanumeric() || s[at - 1] == '_');
    if in_a_word || !s[at..].starts_with(&['c', 'f', 'g']) {
        return Ok(None);
    }
    let bang = skip_trivia(s, at + 3)?;
    if s.get(bang) != Some(&'!') {
        return Ok(None);
    }
    let open = skip_trivia(s, bang + 1)?;
    Ok(matches!(s.get(open), Some('(' | '[' | '{')).then_some(open + 1))
}

/// The text inside the bracket group whose opener ends just before
/// `open`, each comment inside it read as a space, and the index just past
/// its closer; `what` names the unit when the group never closes.
fn group(s: &[char], open: usize, what: &str) -> Result<(String, usize), String> {
    let (mut text, mut depth, mut j) = (String::new(), 1, open);
    while depth > 0 {
        if j >= s.len() {
            return Err(format!("an unterminated {what}"));
        }
        let past_comment = skip_comment(s, j)?;
        if past_comment != j {
            text.push(' ');
            j = past_comment;
            continue;
        }
        let past_literal = skip_literal(s, j)?;
        if past_literal != j {
            text.extend(&s[j..past_literal]);
            j = past_literal;
            continue;
        }
        depth += match s[j] {
            '(' | '[' | '{' => 1,
            ')' | ']' | '}' => -1,
            _ => 0,
        };
        text.push(s[j]);
        j += 1;
    }
    text.pop();
    Ok((text, j))
}

/// Every attribute's inner text (`expect(..)` of `#[expect(..)]`) and
/// every `cfg!` invocation's whole text (`cfg!(..)`), each with the lines
/// it starts and ends on, counted from 1.
pub(crate) fn units(source: &str) -> Result<Vec<(RangeInclusive<usize>, String)>, String> {
    let s: Vec<char> = source.chars().collect();
    let newlines: Vec<usize> = (0..s.len()).filter(|&at| s[at] == '\n').collect();
    let line_at = |at: usize| newlines.partition_point(|&newline| newline < at) + 1;
    let (mut out, mut i) = (Vec::new(), 0);
    while i < s.len() {
        let next = skip_literal(&s, skip_comment(&s, i)?)?;
        if next != i {
            i = next;
        } else if let Some(open) = attribute_open(&s, i)? {
            let (text, end) = group(&s, open, "attribute")?;
            out.push((line_at(i)..=line_at(end - 1), text));
            i = end;
        } else if let Some(open) = cfg_open(&s, i)? {
            let (inner, end) = group(&s, open, "cfg! invocation")?;
            let text = format!("cfg!{}{inner}{}", s[open - 1], s[end - 1]);
            out.push((line_at(i)..=line_at(end - 1), text));
            i = end;
        } else {
            i += 1;
        }
    }
    Ok(out)
}

/// Whether a `cfg` predicate holds in every test build the gate's hosts
/// make: `test` holds because the source is read as a module of a
/// `tests/it.rs`, compiled only as a test, and `unix` holds on every
/// supported host (decision 0063, Linux and macOS). A `target_os` of the
/// two supported systems is asked of the compiling host, so a name the
/// binary of this host does not carry is not vouched. Any other predicate
/// the reader cannot evaluate, so it vouches for nothing: a name behind
/// it refuses, never passes.
fn predicate_holds(predicate: &str) -> bool {
    match predicate.trim() {
        "test" | "unix" => true,
        "target_os = \"linux\"" => cfg!(target_os = "linux"),
        "target_os = \"macos\"" => cfg!(target_os = "macos"),
        _ => false,
    }
}

/// The predicate of a `cfg(…)` attribute's text, or `None` for any other
/// attribute. A `cfg_attr` gates the attribute it carries, not the item,
/// and marks nothing here.
fn cfg_predicate(attribute: &str) -> Option<&str> {
    let (head, rest) = attribute.split_once('(')?;
    (head.trim_end() == "cfg")
        .then_some(rest)?
        .strip_suffix(')')
}

/// The name of the `fn` an attribute run ending at `s[at..]` marks, read
/// past trivia and further attributes from the first one's bracket: the
/// name when the run holds `#[test]` and every `cfg` predicate in it is
/// vouched, else `None` — a name the binary may not carry is no test. The
/// index returned is just past the name, so the scan resumes inside the
/// item, whose brackets close themselves. A run that ends at no `fn`
/// marks nothing; a `fn` with no name, or a raw-identified one this
/// reader does not read, is an error or no test.
fn marked_fn(s: &[char], at: usize) -> Result<(Option<String>, usize), String> {
    let (mut marked, mut vouched, mut i) = (false, true, at);
    loop {
        let (text, end) = group(s, i, "attribute")?;
        let trimmed = text.trim();
        if trimmed == "test" {
            marked = true;
        } else if let Some(predicate) = cfg_predicate(trimmed) {
            vouched &= predicate_holds(predicate);
        }
        i = skip_trivia(s, end)?;
        match (s.get(i), attribute_open(s, i)?) {
            (Some('#'), Some(open)) => i = open,
            _ => break,
        }
    }
    let is_fn = s[i..].starts_with(&['f', 'n'])
        && !s
            .get(i + 2)
            .is_some_and(|c| c.is_alphanumeric() || *c == '_');
    if !marked || !is_fn {
        return Ok((None, i));
    }
    let i = skip_trivia(s, (i + 2).min(s.len()))?;
    let name: String = s[i..]
        .iter()
        .take_while(|c| c.is_alphanumeric() || **c == '_')
        .collect();
    if name.is_empty() {
        return Err("a #[test] with no function name".into());
    }
    if name == "r" && s.get(i + 1) == Some(&'#') {
        return Ok((None, i));
    }
    Ok((vouched.then_some(name.clone()), i + name.len()))
}

/// The `#[test]` functions at the top level of a module's source, by
/// name, as the tokens hold them: an attribute run that holds `#[test]`
/// and ends at a `fn`, outside every literal and comment and at bracket
/// depth 0, so a pair in a string literal, a comment or a nested item is
/// no test. A run's `cfg` predicates are held to what a test binary of
/// the supported hosts carries ([`predicate_holds`]). Text it cannot read
/// is an error, never an absence.
pub(crate) fn tests(source: &str) -> Result<BTreeSet<String>, String> {
    let s: Vec<char> = source.chars().collect();
    let (mut out, mut i, mut depth) = (BTreeSet::new(), 0, 0usize);
    while i < s.len() {
        let next = skip_literal(&s, skip_comment(&s, i)?)?;
        if next != i {
            i = next;
            continue;
        }
        match s[i] {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '#' if depth == 0 => {
                if let Some(open) = attribute_open(&s, i)? {
                    let (marked, end) = marked_fn(&s, open)?;
                    out.extend(marked);
                    i = end;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(out)
}
