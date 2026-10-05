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

/// The index just past the `[` that opens an attribute at `at`, and
/// whether the attribute is inner — `#![..]`, one a module carries
/// itself —: `#`, an optional `!`, and whitespace or comments between any
/// of them, as rustc reads the three tokens. `None` when no attribute
/// starts there.
fn attribute_open(s: &[char], at: usize) -> Result<Option<(usize, bool)>, String> {
    if s[at] != '#' {
        return Ok(None);
    }
    let mut i = skip_trivia(s, at + 1)?;
    let inner = s.get(i) == Some(&'!');
    if inner {
        i = skip_trivia(s, i + 1)?;
    }
    Ok((s.get(i) == Some(&'[')).then_some((i + 1, inner)))
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

/// Splits `text` at commas outside brackets and strings.
pub(crate) fn top_level(text: &str) -> Result<Vec<String>, String> {
    let s: Vec<char> = text.chars().collect();
    let (mut parts, mut depth, mut from, mut i) = (Vec::new(), 0i32, 0, 0);
    while i < s.len() {
        let next = skip_literal(&s, i)?;
        if next != i {
            i = next;
            continue;
        }
        match s[i] {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(s[from..i].iter().collect::<String>().trim().to_string());
                from = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    let last: String = s[from..].iter().collect::<String>().trim().to_string();
    if !last.is_empty() {
        parts.push(last);
    }
    Ok(parts)
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
        } else if let Some((open, _)) = attribute_open(&s, i)? {
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
/// binary of this host does not carry is not vouched. A `cfg_attr`'s own
/// predicate is asked the same way, of the attributes it applies. Any
/// other predicate the reader cannot evaluate, so it vouches for
/// nothing: a name behind it refuses, never passes.
fn predicate_holds(predicate: &str) -> bool {
    match predicate.trim() {
        "test" | "unix" => true,
        "target_os = \"linux\"" => cfg!(target_os = "linux"),
        "target_os = \"macos\"" => cfg!(target_os = "macos"),
        _ => false,
    }
}

/// The head of an attribute's text and the text inside its parentheses:
/// `name(args)`. `None` for any other attribute, `test` among them.
fn call(text: &str) -> Option<(&str, &str)> {
    let (head, rest) = text.split_once('(')?;
    Some((head.trim_end(), rest.strip_suffix(')')?))
}

/// A `cfg_attr`'s predicate and the attributes it applies, split at
/// top-level commas. One with no predicate is an error naming it.
fn cfg_attr_of(args: &str) -> Result<(String, Vec<String>), String> {
    let mut parts = top_level(args)?;
    let predicate = parts
        .first()
        .filter(|predicate| !predicate.is_empty())
        .cloned()
        .ok_or("an empty cfg_attr")?;
    parts.remove(0);
    Ok((predicate, parts))
}

/// What one attribute of a run tells the reader, a `cfg_attr`'s
/// attributes applied when its own predicate holds: `marked` when one
/// marks its item a test, `vouched` while every `cfg` predicate the run
/// carries holds in the build the binary is made in. An attribute the
/// reader cannot take apart is an error naming it, never a silence.
fn apply_attribute(text: &str, marked: &mut bool, vouched: &mut bool) -> Result<(), String> {
    let trimmed = text.trim();
    let Some((name, args)) = call(trimmed) else {
        if trimmed == "test" {
            *marked = true;
        }
        return Ok(());
    };
    match name {
        "cfg" => *vouched &= predicate_holds(args),
        "cfg_attr" => {
            let (predicate, applied) = cfg_attr_of(args)?;
            if predicate_holds(&predicate) {
                for attr in &applied {
                    apply_attribute(attr, marked, vouched)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// Whether a module's items stand after one inner attribute of its
/// source: a `cfg` predicate is asked of the build, a `cfg_attr`'s own
/// predicate gates the `cfg` it applies, and any other attribute gates
/// nothing. A module that does not stand in this build carries no test
/// of the binary, whatever its source shows.
fn module_stands(text: &str) -> Result<bool, String> {
    let Some((name, args)) = call(text.trim()) else {
        return Ok(true);
    };
    match name {
        "cfg" => Ok(predicate_holds(args)),
        "cfg_attr" => {
            let (predicate, applied) = cfg_attr_of(args)?;
            if !predicate_holds(&predicate) {
                return Ok(true);
            }
            for attr in &applied {
                match call(attr) {
                    Some(("cfg", predicate)) if !predicate_holds(predicate) => return Ok(false),
                    Some(("cfg_attr", _)) if !module_stands(attr)? => return Ok(false),
                    _ => {}
                }
            }
            Ok(true)
        }
        _ => Ok(true),
    }
}

/// The name of the `fn` an attribute run ending at `s[at..]` marks, read
/// past trivia and further attributes from the first one's bracket: the
/// name when the run holds `#[test]` and every `cfg` predicate in it is
/// vouched — a `cfg_attr`'s applied when its own predicate holds
/// ([`apply_attribute`]) — else `None`, a name the binary may not carry
/// is no test. The index returned is just past the name, so the scan
/// resumes inside the item, whose brackets close themselves. A run that
/// ends at no `fn` marks nothing; a `fn` with no name, or a
/// raw-identified one this reader does not read, is an error or no test.
fn marked_fn(s: &[char], at: usize) -> Result<(Option<String>, usize), String> {
    let (mut marked, mut vouched, mut i) = (false, true, at);
    loop {
        let (text, end) = group(s, i, "attribute")?;
        apply_attribute(&text, &mut marked, &mut vouched)?;
        i = skip_trivia(s, end)?;
        match (s.get(i), attribute_open(s, i)?) {
            (Some('#'), Some((open, _))) => i = open,
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

/// The index a module's scan resumes at past the attribute that opens
/// just before `open`, its `#[test]`s pushed into `out`: past the item
/// an outer run marks, past an inner `#![cfg]` that holds. `None` when
/// the inner `#![cfg]` does not hold — the whole module is compiled out
/// then, and no test of it is left to name.
fn attribute_resumes(
    s: &[char],
    open: usize,
    inner: bool,
    out: &mut BTreeSet<String>,
) -> Result<Option<usize>, String> {
    if inner {
        let (text, end) = group(s, open, "attribute")?;
        return Ok(module_stands(&text)?.then_some(end));
    }
    let (marked, end) = marked_fn(s, open)?;
    out.extend(marked);
    Ok(Some(end))
}

/// The `#[test]` functions at the top level of a module's source, by
/// name, as the tokens hold them: an attribute run that holds `#[test]`
/// and ends at a `fn`, outside every literal and comment and at bracket
/// depth 0, so a pair in a string literal, a comment or a nested item is
/// no test. A run's `cfg` and `cfg_attr` predicates are held to what a
/// test binary of the supported hosts carries ([`predicate_holds`]). A
/// module's own inner `#![cfg]` holds the whole module: one that does
/// not stand in this build leaves no test at all. Text it cannot read is
/// an error, never an absence.
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
                if let Some((open, inner)) = attribute_open(&s, i)? {
                    match attribute_resumes(&s, open, inner, &mut out)? {
                        Some(end) => i = end,
                        None => return Ok(BTreeSet::new()),
                    }
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(out)
}
