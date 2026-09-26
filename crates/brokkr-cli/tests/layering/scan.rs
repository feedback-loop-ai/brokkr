//! The purity scan of decision 0071 ruling 1 (#336): a lexer that strips
//! comments and reads literals before anything is matched, the closed std,
//! macro and source allowlists, the attribute-string reader, and the module
//! resolver that decides which files a pure crate compiles outside
//! `#[cfg(test)]`.

use super::*;

/// The crates a path may be rooted at to reach the standard library.
const ROOTS: [&str; 3] = ["std", "core", "alloc"];

/// Ruling 1's allowlist: the only paths under `std`, `core` or `alloc`
/// the pure crates may name, each a whole module or a single item, and
/// each one they use today. Every other path under a root is refused,
/// the root itself included.
const PURE_STD: [&str; 8] = [
    "cell",
    "collections::BTreeMap",
    "collections::BTreeSet",
    "fmt",
    "iter",
    "mem",
    "rc",
    "str",
];

/// The macros the pure crates may invoke besides those they define with
/// `macro_rules!`. `thread_local` serves view's test-only counters, and
/// clippy refuses the `LocalKey` it makes anywhere else.
const PURE_MACROS: [&str; 5] = ["format", "json", "matches", "thread_local", "vec"];

/// Keywords that may stand before `!(`, where the `!` is a negation.
const NEGATING_KEYWORDS: [&str; 6] = ["break", "if", "in", "match", "return", "while"];

/// Every macro std and core define, reached through the prelude or a path,
/// stable and unstable. A pure crate may not define a macro by one of these
/// names, so a call by one always means the builtin: only those in
/// [`PURE_MACROS`] pass.
const BUILTIN_MACROS: [&str; 56] = [
    "addr_of",
    "addr_of_mut",
    "asm",
    "assert",
    "assert_eq",
    "assert_matches",
    "assert_ne",
    "cfg",
    "cfg_match",
    "cfg_select",
    "column",
    "compile_error",
    "concat",
    "concat_bytes",
    "concat_idents",
    "const_format_args",
    "dbg",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_matches",
    "debug_assert_ne",
    "env",
    "eprint",
    "eprintln",
    "file",
    "format",
    "format_args",
    "format_args_nl",
    "global_asm",
    "include",
    "include_bytes",
    "include_str",
    "is_aarch64_feature_detected",
    "is_x86_feature_detected",
    "line",
    "log_syntax",
    "matches",
    "module_path",
    "naked_asm",
    "offset_of",
    "option_env",
    "panic",
    "pin",
    "print",
    "println",
    "ready",
    "stringify",
    "thread_local",
    "todo",
    "trace_macros",
    "try",
    "unimplemented",
    "unreachable",
    "vec",
    "write",
    "writeln",
];

/// Clock and randomness sources named apart from the effect table: std's
/// clock and hasher types and uuid's generators, refused wherever they are
/// named. The time crate's clock functions have their home in `EFFECTS`.
const SOURCES: [&str; 8] = [
    "SystemTime",
    "Instant",
    "new_v4",
    "new_v7",
    "now_v1",
    "now_v6",
    "now_v7",
    "RandomState",
];

/// What every string, byte string, raw string and char literal lexes to.
const LITERAL: &str = "<literal>";

/// A token and the line it starts on.
type Token = (usize, String);

/// The token at `at`, or `""` past the end.
fn text(tokens: &[Token], at: usize) -> &str {
    tokens.get(at).map_or("", |(_, token)| token.as_str())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_ident(token: &str) -> bool {
    token.starts_with(|c: char| c.is_alphabetic() || c == '_') && token.chars().all(is_word)
}

fn word_end(chars: &[char], at: usize) -> usize {
    at + chars[at..].iter().take_while(|c| is_word(**c)).count()
}

/// Rust source as tokens, lexed before anything is matched: line, block
/// (nested) and doc comments are dropped, and each literal reads as
/// [`LITERAL`], so neither can hide code or pose as it. Words (a raw
/// identifier's `r#` dropped), `::` and lifetimes are one token each,
/// every other non-space character is its own. Beside the tokens, the
/// source text of every [`LITERAL`] token, keyed by the token's index,
/// quotes and prefixes included.
fn lex_with_literals(source: &str) -> (Vec<Token>, BTreeMap<usize, String>) {
    let chars: Vec<char> = source.chars().collect();
    let (mut tokens, mut literals) = (Vec::new(), BTreeMap::new());
    let (mut at, mut line) = (0, 1);
    while at < chars.len() {
        let (end, token) = next_token(&chars, at);
        if token.as_deref() == Some(LITERAL) {
            literals.insert(tokens.len(), chars[at..end].iter().collect());
        }
        tokens.extend(token.map(|token| (line, token)));
        line += chars[at..end].iter().filter(|c| **c == '\n').count();
        at = end;
    }
    (tokens, literals)
}

/// The token at `at`, if it is not a comment or space, and where it ends.
fn next_token(chars: &[char], at: usize) -> (usize, Option<String>) {
    match (chars[at], chars.get(at + 1)) {
        ('/', Some('/')) => {
            let length = chars[at..].iter().take_while(|c| **c != '\n').count();
            (at + length, None)
        }
        ('/', Some('*')) => (block_comment_end(chars, at), None),
        ('"', _) => (quoted_end(chars, at + 1, '"'), Some(LITERAL.into())),
        ('\'', _) => char_or_lifetime(chars, at),
        (':', Some(':')) => (at + 2, Some("::".into())),
        (c, _) if is_word(c) => word_or_literal(chars, at),
        (c, _) if c.is_whitespace() => (at + 1, None),
        (c, _) => (at + 1, Some(c.to_string())),
    }
}

/// The end of the block comment opening at `at`, which nests.
fn block_comment_end(chars: &[char], at: usize) -> usize {
    let (mut depth, mut index) = (0, at);
    while index < chars.len() {
        match (chars[index], chars.get(index + 1)) {
            ('/', Some('*')) => depth += 1,
            ('*', Some('/')) => depth -= 1,
            _ => {
                index += 1;
                continue;
            }
        }
        index += 2;
        if depth == 0 {
            return index;
        }
    }
    chars.len()
}

/// The end of a literal whose body starts at `from` and closes at an
/// unescaped `quote`.
fn quoted_end(chars: &[char], from: usize, quote: char) -> usize {
    let mut index = from;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 2,
            c if c == quote => return index + 1,
            _ => index += 1,
        }
    }
    chars.len()
}

/// A char literal (`'x'`, `'\n'`) or a lifetime or label (`'a`).
fn char_or_lifetime(chars: &[char], at: usize) -> (usize, Option<String>) {
    match (chars.get(at + 1), chars.get(at + 2)) {
        (Some('\\'), _) => (quoted_end(chars, at + 1, '\''), Some(LITERAL.into())),
        (Some(_), Some('\'')) => (at + 3, Some(LITERAL.into())),
        _ => {
            let end = word_end(chars, at + 1);
            (end, Some(chars[at..end].iter().collect()))
        }
    }
}

/// A word, or the literal or raw identifier a word prefix opens: `r"…"`,
/// `br#"…"#`, `cr"…"`, `b"…"`, `c"…"`, `b'…'` and `r#ident`.
fn word_or_literal(chars: &[char], at: usize) -> (usize, Option<String>) {
    let end = word_end(chars, at);
    let word: String = chars[at..end].iter().collect();
    let hashes = chars[end..].iter().take_while(|c| **c == '#').count();
    match (word.as_str(), hashes, chars.get(end + hashes)) {
        ("r" | "br" | "cr", _, Some('"')) => {
            let body = end + hashes + 1;
            let close = (body..chars.len()).find(|&index| {
                chars[index] == '"'
                    && chars[index + 1..]
                        .iter()
                        .take(hashes)
                        .filter(|c| **c == '#')
                        .count()
                        == hashes
            });
            (
                close.map_or(chars.len(), |index| index + 1 + hashes),
                Some(LITERAL.into()),
            )
        }
        ("r", 1, Some(c)) if is_word(*c) => {
            let ident_end = word_end(chars, end + 1);
            (ident_end, Some(chars[end + 1..ident_end].iter().collect()))
        }
        ("b" | "c", 0, Some('"')) => (quoted_end(chars, end + 1, '"'), Some(LITERAL.into())),
        ("b", 0, Some('\'')) => (quoted_end(chars, end + 1, '\''), Some(LITERAL.into())),
        _ => (end, Some(word)),
    }
}

/// A `use` tree in a form the reader does not know.
struct Unreadable;

/// One path a `use` tree imports, the line of its last segment, and
/// whether it is renamed.
struct Import {
    line: usize,
    path: Vec<String>,
    renamed: bool,
}

/// Resolves the `use` tree at `at` onto `path`, nested groups, `self`,
/// globs and renames included, and leaves `at` past it. `Err` for a form
/// this reader does not know.
fn use_tree(
    tokens: &[Token],
    at: &mut usize,
    mut path: Vec<String>,
    imports: &mut Vec<Import>,
) -> Result<(), Unreadable> {
    if text(tokens, *at) == "::" {
        *at += 1;
    }
    loop {
        let (line, token) = tokens.get(*at).ok_or(Unreadable)?.clone();
        *at += 1;
        if token == "{" {
            return use_group(tokens, at, &path, imports);
        }
        if token != "*" && !is_ident(&token) {
            return Err(Unreadable);
        }
        let glob = token == "*";
        path.push(token);
        match (glob, text(tokens, *at)) {
            (false, "::") => *at += 1,
            (false, "as") => {
                *at += 2;
                imports.push(Import {
                    line,
                    path,
                    renamed: true,
                });
                return Ok(());
            }
            _ => {
                imports.push(Import {
                    line,
                    path,
                    renamed: false,
                });
                return Ok(());
            }
        }
    }
}

/// The trees of a `{ … }` group, each resolved onto `path`.
fn use_group(
    tokens: &[Token],
    at: &mut usize,
    path: &[String],
    imports: &mut Vec<Import>,
) -> Result<(), Unreadable> {
    loop {
        if text(tokens, *at) == "}" {
            *at += 1;
            return Ok(());
        }
        use_tree(tokens, at, path.to_vec(), imports)?;
        match text(tokens, *at) {
            "," => *at += 1,
            "}" => {}
            _ => return Err(Unreadable),
        }
    }
}

/// Whether a path rooted at `std`, `core` or `alloc` falls under the
/// allowlist. The root alone never does.
fn pure_std(path: &[String]) -> bool {
    PURE_STD.iter().any(|entry| {
        let entry: Vec<&str> = entry.split("::").collect();
        path.len() > entry.len()
            && path[1..=entry.len()]
                .iter()
                .zip(&entry)
                .all(|(a, b)| a == b)
    })
}

/// The refusal for one import, if it reaches a root: from the first root
/// segment on (so `crate::std::fs` is `std::fs`), a rename of the root
/// itself, or a path outside the allowlist.
fn import_refusal(import: &Import) -> Option<(usize, String)> {
    let root = import
        .path
        .iter()
        .position(|segment| ROOTS.contains(&segment.as_str()))?;
    let mut path = import.path[root..].to_vec();
    if path.len() > 1 && path.last().is_some_and(|segment| segment == "self") {
        path.pop();
    }
    if path.len() == 1 && import.renamed {
        return Some((import.line, format!("a rename of {}", path[0])));
    }
    (!pure_std(&path)).then(|| {
        (
            import.line,
            format!("{}: outside the pure std allowlist", path.join("::")),
        )
    })
}

/// The path rooted at `tokens[at]` outside an import, and where it ends.
/// A segment that is not an identifier (a macro's `$`) ends the path and
/// is kept, so the path falls outside the allowlist; a turbofish ends it.
fn expression_path(tokens: &[Token], at: usize) -> (Vec<String>, usize) {
    let mut path = vec![tokens[at].1.clone()];
    let mut end = at + 1;
    while text(tokens, end) == "::" && !["", "<"].contains(&text(tokens, end + 1)) {
        path.push(text(tokens, end + 1).to_string());
        end += 2;
        if !is_ident(text(tokens, end - 1)) {
            break;
        }
    }
    (path, end)
}

/// Every import and every other path that reaches a root and falls
/// outside the allowlist, and every rename of a root.
fn std_refusals(tokens: &[Token]) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        let line = tokens[at].0;
        if text(tokens, at) == "use" && text(tokens, at + 1) != "<" {
            let (mut end, mut imports) = (at + 1, Vec::new());
            if use_tree(tokens, &mut end, Vec::new(), &mut imports).is_ok()
                && text(tokens, end) == ";"
            {
                found.extend(imports.iter().filter_map(import_refusal));
                at = end + 1;
                continue;
            }
            found.push((line, "an import the scanner cannot read".to_string()));
        } else if ROOTS.contains(&text(tokens, at)) {
            let (path, end) = expression_path(tokens, at);
            if !pure_std(&path) {
                let path = path.join("::");
                found.push((line, format!("{path}: outside the pure std allowlist")));
            }
            at = end;
            continue;
        }
        at += 1;
    }
    found
}

/// Whether the attribute opening at `at` sets a `path`, which could make
/// a module read a file this scan never lists.
fn sets_a_path(tokens: &[Token], at: usize) -> bool {
    attribute_end(tokens, at).is_some_and(|end| {
        (at..end).any(|index| text(tokens, index) == "path" && text(tokens, index + 1) == "=")
    })
}

/// Where each token sits among the brace blocks: `at[i]` is the `{` of
/// the innermost block open at token `i`, and `parent[&b]` the `{` of the
/// block around the one opening at `b`. `None` is the file's top level.
struct Blocks {
    at: Vec<Option<usize>>,
    parent: BTreeMap<usize, Option<usize>>,
}

impl Blocks {
    fn of(tokens: &[Token]) -> Self {
        let mut open: Vec<usize> = Vec::new();
        let mut blocks = Blocks {
            at: Vec::with_capacity(tokens.len()),
            parent: BTreeMap::new(),
        };
        for (index, (_, token)) in tokens.iter().enumerate() {
            blocks.at.push(open.last().copied());
            match token.as_str() {
                "{" => {
                    blocks.parent.insert(index, open.last().copied());
                    open.push(index);
                }
                "}" => {
                    open.pop();
                }
                _ => {}
            }
        }
        blocks
    }

    /// Whether token `inner` lies in the block `outer`, directly or in a
    /// block nested in it.
    fn within(&self, outer: Option<usize>, inner: usize) -> bool {
        let mut block = self.at[inner];
        loop {
            if block == outer {
                return true;
            }
            match block {
                Some(open) => block = self.parent[&open],
                None => return false,
            }
        }
    }
}

/// Whether a `macro_rules!` of `name` is in scope at token `at`: defined
/// earlier, in the block that holds `at` or one around it. A builtin name
/// is never in scope, because defining one is refused.
fn defined_before(tokens: &[Token], blocks: &Blocks, name: &str, at: usize) -> bool {
    !BUILTIN_MACROS.contains(&name)
        && (2..at).any(|index| {
            text(tokens, index - 2) == "macro_rules"
                && text(tokens, index - 1) == "!"
                && text(tokens, index) == name
                && blocks.within(blocks.at[index - 2], at)
        })
}

/// The index of the `]` closing the attribute, inner or outer, whose `#`
/// is at `at`. `None` when no `[` follows.
fn attribute_end(tokens: &[Token], at: usize) -> Option<usize> {
    let open = at + 1 + usize::from(text(tokens, at + 1) == "!");
    if text(tokens, open) != "[" {
        return None;
    }
    let mut depth = 0;
    for (index, (_, token)) in tokens.iter().enumerate().skip(open) {
        match token.as_str() {
            "[" => depth += 1,
            "]" if depth == 1 => return Some(index),
            "]" => depth -= 1,
            _ => {}
        }
    }
    None
}

/// Whether the attribute from `at` to `end` lowers a lint in
/// [`PURITY_LINTS`]: it holds `allow`, `expect` or `warn`, and names one.
fn lowers_purity(tokens: &[Token], at: usize, end: usize) -> bool {
    let words: Vec<&str> = (at..=end).map(|index| text(tokens, index)).collect();
    let lowers = words
        .iter()
        .any(|word| ["allow", "expect", "warn"].contains(word));
    let names = words.iter().enumerate().any(|(index, word)| {
        let path = match words.get(index + 1..index + 3) {
            Some(["::", lint]) if *word == "clippy" => format!("clippy::{lint}"),
            _ => (*word).to_string(),
        };
        PURITY_LINTS.contains(&path.as_str())
    });
    lowers && names
}

/// One thread-local counter as the admitted shape spells it.
const COUNTER: [&str; 22] = [
    "pub", "(", "super", ")", "static", "<name>", ":", "Cell", "<", "usize", ">", "=", "const",
    "{", "Cell", "::", "new", "(", "0", ")", "}", ";",
];

/// The one exemption from the purity lints a pure crate may carry: an
/// outer `#[expect(clippy::disallowed_types, reason = "…")]` on a module
/// that holds only `use std::cell::Cell;` and a `thread_local!` of
/// `Cell<usize>` counters, itself inside a `#[cfg(test)]` module. The
/// view's test-only projector counters take it; nothing else can.
fn admitted_exemption(tokens: &[Token], blocks: &Blocks, at: usize, end: usize) -> bool {
    let attribute = [
        "#",
        "[",
        "expect",
        "(",
        "clippy",
        "::",
        "disallowed_types",
        ",",
        "reason",
        "=",
        LITERAL,
        ")",
        "]",
    ];
    let head = [
        "mod",
        "<name>",
        "{",
        "use",
        "std",
        "::",
        "cell",
        "::",
        "Cell",
        ";",
        "thread_local",
        "!",
        "{",
    ];
    let fits = |from: usize, shape: &[&str]| {
        shape.iter().enumerate().all(|(offset, want)| {
            let got = text(tokens, from + offset);
            if *want == "<name>" {
                is_ident(got)
            } else {
                got == *want
            }
        })
    };
    if end + 1 - at != attribute.len() || !fits(at, &attribute) || !fits(end + 1, &head) {
        return false;
    }
    let enclosing = blocks.at[at].is_some_and(|open| {
        open >= 2
            && text(tokens, open - 2) == "mod"
            && is_ident(text(tokens, open - 1))
            && cfg_test_gated(tokens, open - 2)
    });
    let open = end + head.len();
    let Some(close) = (open + 1..tokens.len())
        .find(|&index| text(tokens, index) == "}" && blocks.at[index] == Some(open))
    else {
        return false;
    };
    let length = close - open - 1;
    enclosing
        && length.is_multiple_of(COUNTER.len())
        && (0..length / COUNTER.len()).all(|n| fits(open + 1 + n * COUNTER.len(), &COUNTER))
        && text(tokens, close + 1) == "}"
        && blocks.at[close + 1] == Some(end + 3)
}

/// The index of the bracket closing the one opening at `open`, `(`, `[`
/// or `{`, counting only brackets of that kind, or `None` if it never does.
fn closing(tokens: &[Token], open: usize) -> Option<usize> {
    let (opener, closer) = match text(tokens, open) {
        "(" => ("(", ")"),
        "[" => ("[", "]"),
        "{" => ("{", "}"),
        _ => return None,
    };
    let mut depth = 0;
    for (index, (_, token)) in tokens.iter().enumerate().skip(open) {
        if token == opener {
            depth += 1;
        } else if token == closer {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

/// The spans of every `macro_rules!` body, whatever its delimiter. An
/// unclosed body runs to the end of the file.
fn macro_bodies(tokens: &[Token]) -> Vec<(usize, usize)> {
    (0..tokens.len())
        .filter(|&at| text(tokens, at) == "macro_rules" && text(tokens, at + 1) == "!")
        .map(|at| (at, closing(tokens, at + 3).unwrap_or(tokens.len())))
        .collect()
}

/// The outer attributes written before the item whose keyword is at `at`,
/// past its visibility, as `(#, ])` index pairs.
fn outer_attributes(tokens: &[Token], at: usize) -> Vec<(usize, usize)> {
    let mut start = at;
    if text(tokens, start.wrapping_sub(1)) == ")" {
        let open = (0..start - 1)
            .rev()
            .find(|&index| closing(tokens, index) == Some(start - 1));
        if let Some(open) = open.filter(|&open| text(tokens, open.wrapping_sub(1)) == "pub") {
            start = open - 1;
        }
    } else if text(tokens, start.wrapping_sub(1)) == "pub" {
        start -= 1;
    }
    let mut attributes = Vec::new();
    while text(tokens, start.wrapping_sub(1)) == "]" {
        let Some(open) = (0..start - 1)
            .rev()
            .find(|&index| closing(tokens, index) == Some(start - 1))
        else {
            break;
        };
        if open == 0 || text(tokens, open - 1) != "#" {
            break;
        }
        attributes.push((open - 1, start - 1));
        start = open - 1;
    }
    attributes
}

/// Whether one of `attributes` is exactly `#[cfg(test)]`.
fn gated_by_cfg_test(tokens: &[Token], attributes: &[(usize, usize)]) -> bool {
    let exact = ["#", "[", "cfg", "(", "test", ")", "]"];
    attributes
        .iter()
        .any(|&(from, to)| (from..=to).map(|index| text(tokens, index)).eq(exact))
}

/// Whether the item or attribute at `at` sits under exactly
/// `#[cfg(test)]`, among the outer attributes written before it.
fn cfg_test_gated(tokens: &[Token], at: usize) -> bool {
    gated_by_cfg_test(tokens, &outer_attributes(tokens, at))
}

/// One lexed source file of a pure crate, and where it sits in the
/// workspace, which a `#[path]` resolves against and names the crate.
struct Scanned<'a> {
    tokens: &'a [Token],
    literals: &'a BTreeMap<usize, String>,
    blocks: &'a Blocks,
    file: &'a Path,
}

/// Whether a workspace-relative `path` is one the exact coverage gate
/// treats as a test, in the gate's one vocabulary (`support/test_paths.rs`).
fn is_gate_test_path(path: &Path) -> bool {
    path.to_str()
        .is_some_and(crate::test_paths::is_gate_test_path)
}

/// `relative` joined onto `dir` with `.` and `..` folded, or `None` when it
/// climbs out of the workspace.
fn resolve(dir: &Path, relative: &str) -> Option<PathBuf> {
    let mut resolved = Vec::new();
    for part in dir.iter().chain(Path::new(relative).iter()) {
        match part.to_str()? {
            "." => {}
            ".." => {
                resolved.pop()?;
            }
            part => resolved.push(part),
        }
    }
    Some(resolved.iter().collect())
}

/// Whether the `#` at `at` opens the one `#[path]` form test code may
/// take: `#[cfg(test)] #[path = "…"] mod name;` at the file's top level,
/// the path a plain string that resolves, from the file's directory, to a
/// gate test path inside the workspace.
fn test_path_module(scan: &Scanned, at: usize) -> bool {
    let Some(end) = attribute_end(scan.tokens, at) else {
        return false;
    };
    let declares = text(scan.tokens, end + 1) == "mod"
        && is_ident(text(scan.tokens, end + 2))
        && text(scan.tokens, end + 3) == ";";
    let target = plain_path(scan, at, end).and_then(|path| resolve(scan.file.parent()?, path));
    declares
        && scan.blocks.at[at].is_none()
        && cfg_test_gated(scan.tokens, at)
        && target.is_some_and(|target| is_gate_test_path(&target))
}

/// The string an attribute spanning `from..=to` sets as a module's path,
/// when the attribute is exactly `#[path = "…"]` and the string is plain,
/// with no raw prefix and no escape; `None` for any other form.
fn plain_path<'a>(scan: &Scanned<'a>, from: usize, to: usize) -> Option<&'a str> {
    let shape = ["#", "[", "path", "=", LITERAL, "]"];
    (from..=to)
        .map(|index| text(scan.tokens, index))
        .eq(shape)
        .then(|| scan.literals.get(&(from + 4)))
        .flatten()
        .and_then(|literal| literal.strip_prefix('"')?.strip_suffix('"'))
        .filter(|path| !path.contains(['\\', '"']))
}

/// Whether the `extern` at `at` is the one `extern crate` test code may
/// take: `#[cfg(test)] extern crate self as <this crate>;`, which lets
/// the workspace's shared test support name the crate as other crates do.
fn test_self_alias(scan: &Scanned, at: usize) -> bool {
    let own = scan
        .file
        .strip_prefix("crates")
        .ok()
        .and_then(|rest| rest.iter().next()?.to_str())
        .map(|name| name.replace('-', "_"));
    cfg_test_gated(scan.tokens, at)
        && text(scan.tokens, at + 2) == "self"
        && text(scan.tokens, at + 3) == "as"
        && own.is_some_and(|own| text(scan.tokens, at + 4) == own)
        && text(scan.tokens, at + 5) == ";"
}

/// Every item form the pure crates may not take: an `extern crate`, a
/// `#[path]`, a `mod tests` outside `#[cfg(test)]`, a `mod` inside a
/// `macro_rules!` body, a lowered purity lint,
/// a `macro_rules!` shadowing a builtin, a macro outside the allowlist or
/// out of its definition's scope, and a clock or randomness source. Code
/// under exactly `#[cfg(test)]` may take two of them: a `#[path]` to a
/// gate test path, and an alias of the crate itself.
fn form_refusals(scan: &Scanned) -> Vec<(usize, String)> {
    let (tokens, blocks) = (scan.tokens, scan.blocks);
    let bodies = macro_bodies(tokens);
    let mut found = Vec::new();
    for (at, (line, token)) in tokens.iter().enumerate() {
        let what = match (token.as_str(), text(tokens, at + 1), text(tokens, at + 2)) {
            ("extern", "crate", _) if !test_self_alias(scan, at) => "an extern crate".to_string(),
            ("mod", _, _) if bodies.iter().any(|(from, to)| (*from..*to).contains(&at)) => {
                "a `mod` inside a macro_rules! body".to_string()
            }
            ("mod", "tests", ";" | "{")
                if !gated_by_cfg_test(tokens, &outer_attributes(tokens, at)) =>
            {
                "a `mod tests` outside #[cfg(test)]".to_string()
            }
            ("#", _, _) if sets_a_path(tokens, at) && !test_path_module(scan, at) => {
                "a #[path] attribute".to_string()
            }
            ("#", _, _)
                if attribute_end(tokens, at).is_some_and(|end| {
                    lowers_purity(tokens, at, end) && !admitted_exemption(tokens, blocks, at, end)
                }) =>
            {
                "an attribute that lowers a purity lint".to_string()
            }
            ("macro_rules", "!", name) if BUILTIN_MACROS.contains(&name) => {
                format!("macro_rules! {name}: shadows a builtin macro")
            }
            (name, "!", "(" | "[" | "{")
                if is_ident(name)
                    && !NEGATING_KEYWORDS.contains(&name)
                    && !PURE_MACROS.contains(&name)
                    && !defined_before(tokens, blocks, name, at) =>
            {
                format!("{name}!: outside the pure macro allowlist")
            }
            ("::", "now", _) => "::now: a clock or randomness source".to_string(),
            (name, _, _) if SOURCES.contains(&name) => {
                format!("{name}: a clock or randomness source")
            }
            (name, _, _) if effect_of(name).is_some() => {
                format!(
                    "{name}: an effectful item of {}",
                    effect_of(name).unwrap_or_default()
                )
            }
            _ => continue,
        };
        found.push((*line, what));
    }
    found
}

/// Attribute keys whose string is a path the attribute compiles into
/// code: serde's callbacks, conversions and bounds, and `crate`.
const PATH_KEYS: [&str; 12] = [
    "bound",
    "crate",
    "default",
    "deserialize_with",
    "from",
    "getter",
    "into",
    "remote",
    "serialize_with",
    "skip_serializing_if",
    "try_from",
    "with",
];

/// Attribute keys whose string is data the attribute never runs: names,
/// messages, reasons and `cfg` values. `serialize` and `deserialize` are
/// data under these and a path under [`PATH_KEYS`].
const DATA_KEYS: [&str; 21] = [
    "alias",
    "content",
    "deprecated",
    "doc",
    "expecting",
    "feature",
    "must_use",
    "note",
    "reason",
    "rename",
    "rename_all",
    "rename_all_fields",
    "since",
    "tag",
    "target_arch",
    "target_endian",
    "target_env",
    "target_family",
    "target_os",
    "target_pointer_width",
    "target_vendor",
];

/// Why a path `segments` names leaves ruling 1, if it does: outside the
/// std allowlist from its first root on, a clock or randomness source, or
/// an effectful item of a dependency.
fn path_refusal(segments: &[String]) -> Option<String> {
    if let Some(root) = segments
        .iter()
        .position(|segment| ROOTS.contains(&segment.as_str()))
    {
        if !pure_std(&segments[root..]) {
            let path = segments[root..].join("::");
            return Some(format!("{path}: outside the pure std allowlist"));
        }
    }
    if let Some(source) = segments
        .iter()
        .find(|segment| SOURCES.contains(&segment.as_str()))
    {
        return Some(format!("{source}: a clock or randomness source"));
    }
    if segments.iter().skip(1).any(|segment| segment == "now") {
        return Some("::now: a clock or randomness source".to_string());
    }
    segments.iter().find_map(|segment| {
        effect_of(segment).map(|effect| format!("{segment}: an effectful item of {effect}"))
    })
}

/// The segments of a plain string literal that spells a Rust path, or
/// `None`: raw and escaped strings and anything but `::`-joined
/// identifiers are not read as paths.
fn literal_path(literal: &str) -> Option<Vec<String>> {
    let body = literal.strip_prefix('"')?.strip_suffix('"')?;
    let body = body.strip_prefix("::").unwrap_or(body);
    let segments: Vec<String> = body.split("::").map(str::to_string).collect();
    segments
        .iter()
        .all(|segment| is_ident(segment))
        .then_some(segments)
}

/// The identifier naming the `( … )` group the token at `at` sits in,
/// within the attribute opening at `open`, or `None` at its top level.
fn enclosing_call(tokens: &[Token], open: usize, at: usize) -> Option<&str> {
    let mut depth = 0;
    for index in (open + 1..at).rev() {
        match text(tokens, index) {
            ")" => depth += 1,
            "(" if depth == 0 => return Some(text(tokens, index - 1)),
            "(" => depth -= 1,
            _ => {}
        }
    }
    None
}

/// How one attribute string literal is read, by its key and the group
/// around it: `None` for data, a path's refusal, or a refusal of a string
/// the scanner cannot classify.
fn attribute_literal(scan: &Scanned, open: usize, at: usize) -> Option<String> {
    let literal = scan.literals.get(&at).map_or("", String::as_str);
    let unclassified = || {
        Some(format!(
            "{literal} in an attribute: a string the scanner cannot classify"
        ))
    };
    let enclosing = enclosing_call(scan.tokens, open, at);
    let key = (text(scan.tokens, at - 1) == "=").then(|| text(scan.tokens, at - 2));
    let names_a_path = match (key, enclosing) {
        (Some("path"), _) | (None, Some("error")) => return None,
        (Some("serialize" | "deserialize"), Some(group)) if DATA_KEYS.contains(&group) => {
            return None
        }
        (Some("serialize" | "deserialize"), Some(group)) => PATH_KEYS.contains(&group),
        (Some(key), _) if DATA_KEYS.contains(&key) => return None,
        (Some(key), _) => PATH_KEYS.contains(&key),
        (None, _) => false,
    };
    if !names_a_path {
        return unclassified();
    }
    match literal_path(literal) {
        Some(segments) => {
            path_refusal(&segments).map(|why| format!("{literal} in an attribute: {why}"))
        }
        None => unclassified(),
    }
}

/// Every attribute string literal that names a path outside ruling 1, or
/// that the scanner cannot classify: a string inside an attribute is code
/// whenever its key makes it one.
fn attribute_refusals(scan: &Scanned) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < scan.tokens.len() {
        let Some(end) = (text(scan.tokens, at) == "#")
            .then(|| attribute_end(scan.tokens, at))
            .flatten()
        else {
            at += 1;
            continue;
        };
        for index in at..end {
            if scan.tokens[index].1 == LITERAL {
                if let Some(why) = attribute_literal(scan, at, index) {
                    found.push((scan.tokens[index].0, why));
                }
            }
        }
        at = end + 1;
    }
    found
}

/// Every place a pure crate's source leaves ruling 1, as `line: what`.
/// `file` is the source's path relative to the workspace.
fn impurities(source: &str, file: &Path) -> Vec<String> {
    let (tokens, literals) = lex_with_literals(source);
    let blocks = Blocks::of(&tokens);
    let scan = Scanned {
        tokens: &tokens,
        literals: &literals,
        blocks: &blocks,
        file,
    };
    let mut found = std_refusals(&tokens);
    found.extend(form_refusals(&scan));
    found.extend(attribute_refusals(&scan));
    found.sort_by_key(|(line, _)| *line);
    found
        .into_iter()
        .map(|(line, what)| format!("{line}: {what}"))
        .collect()
}

/// The scanner's cases: a source, and exactly what it refuses there, by
/// line. Every form the allowlists do not name is refused; the last
/// rows are clean or unterminated sources that refuse nothing.
const SCANNER_CASES: [(&str, &[&str]); 48] = [
    (
        "use std::{collections::BTreeMap, /* c */ os::unix::process::parent_id};",
        &["1: std::os::unix::process::parent_id: outside the pure std allowlist"],
    ),
    (
        "use std::{\n    collections::BTreeMap,\n    // c\n    os::unix::process::parent_id,\n};",
        &["4: std::os::unix::process::parent_id: outside the pure std allowlist"],
    ),
    (
        "/* /* */ std::fs::read(p) */ std::process::id()",
        &["1: std::process::id: outside the pure std allowlist"],
    ),
    (
        "/// a \"quote\n//! std::fs\n/** std::env */\nuse std::io::Read;",
        &["4: std::io::Read: outside the pure std allowlist"],
    ),
    (
        "let s = \"/*\"; let t = \"std::fs\"; std::env::var(s)",
        &["1: std::env::var: outside the pure std allowlist"],
    ),
    (
        "let s = r#\"\\\"#; let t = br##\"\"#\"##; std::process::id()",
        &["1: std::process::id: outside the pure std allowlist"],
    ),
    (
        "let q = '\"'; let e = '\\''; let b = b'\"';\nfn f<'a>(x: &'a str) { std::fs::read(x) }",
        &["2: std::fs::read: outside the pure std allowlist"],
    ),
    (
        "use std as s;\nuse core::{self as c};\nuse std::collections as h;",
        &["1: a rename of std", "2: a rename of core", "3: std::collections: outside the pure std allowlist"],
    ),
    (
        "extern crate alloc as a;",
        &["1: alloc: outside the pure std allowlist", "1: an extern crate"],
    ),
    (
        "use std::*;\nuse std::collections::*;\nuse std::fmt::*;",
        &["1: std::*: outside the pure std allowlist", "2: std::collections::*: outside the pure std allowlist"],
    ),
    (
        "use ::std::{collections::{BTreeMap, hash_map::HashMap}, io::{self, Write}};",
        &["1: std::collections::hash_map::HashMap: outside the pure std allowlist", "1: std::io: outside the pure std allowlist", "1: std::io::Write: outside the pure std allowlist"],
    ),
    (
        "macro_rules! reach { ($m:ident) => { std::$m::id() }; }\nreach!(process);",
        &["1: std::$: outside the pure std allowlist"],
    ),
    (
        "macro_rules! at { ($r:ident) => { $r::fs::read(p) }; }\nat!(std);",
        &["2: std: outside the pure std allowlist"],
    ),
    (
        "use crate::std::fs;\nlet f = r#std::fs::read;",
        &["1: std::fs: outside the pure std allowlist", "2: std::fs::read: outside the pure std allowlist"],
    ),
    (
        "use std::{$m};",
        &["1: an import the scanner cannot read", "1: std::{: outside the pure std allowlist"],
    ),
    (
        "println!(\"x\");\ninclude!(\"f.rs\");\nif !(a) { format!(\"{}\", vec![1]) }",
        &["1: println!: outside the pure macro allowlist", "2: include!: outside the pure macro allowlist"],
    ),
    (
        "let now = Instant::now();\nlet t = time::OffsetDateTime::now_utc();\nlet u = time::UtcDateTime::now();\nlet c = clock.now();\nlet d = age(now);",
        &["1: Instant: a clock or randomness source", "1: ::now: a clock or randomness source", "2: now_utc: an effectful item of time 0.3.55", "3: ::now: a clock or randomness source"],
    ),
    (
        "#[path = \"../x.rs\"]\nmod x;\n#[cfg_attr(test, path = \"y.rs\")]\nmod y;\n#![doc(hidden)]",
        &["1: a #[path] attribute", "3: a #[path] attribute"],
    ),
    (
        "mod tests;\n#[cfg(test)]\nmod tests;\n#[cfg(any(test))]\nmod tests;",
        &["1: a `mod tests` outside #[cfg(test)]", "5: a `mod tests` outside #[cfg(test)]"],
    ),
    (
        "#[derive(Deserialize)]\nstruct S {\n    #[serde(default = \"std::os::unix::process::parent_id\")]\n    pid: u32,\n    #[serde(default = \"time::OffsetDateTime::now_utc\")]\n    at: u64,\n}",
        &["3: \"std::os::unix::process::parent_id\" in an attribute: std::os::unix::process::parent_id: outside the pure std allowlist", "5: \"time::OffsetDateTime::now_utc\" in an attribute: now_utc: an effectful item of time 0.3.55"],
    ),
    (
        "#[serde(deserialize_with = \"url::Url::socket_addrs\")]\n#[serde(serialize_with = \"::std::fs::read\", with = \"Instant\")]\n#[cfg_attr(test, serde(getter = \"time::UtcDateTime::now\"))]",
        &["1: \"url::Url::socket_addrs\" in an attribute: socket_addrs: an effectful item of url 2.5.8", "2: \"::std::fs::read\" in an attribute: std::fs::read: outside the pure std allowlist", "2: \"Instant\" in an attribute: Instant: a clock or randomness source", "3: \"time::UtcDateTime::now\" in an attribute: ::now: a clock or randomness source"],
    ),
    (
        "#[serde(bound = \"T: Serialize\")]\n#[serde(foo = \"b\")]\n#[serde(with = r\"std::fs\")]\n#[serde(from = \"std::\\x66s\")]\n#[serde(\"x\")]",
        &["1: \"T: Serialize\" in an attribute: a string the scanner cannot classify", "2: \"b\" in an attribute: a string the scanner cannot classify", "3: r\"std::fs\" in an attribute: a string the scanner cannot classify", "4: \"std::\\x66s\" in an attribute: a string the scanner cannot classify", "5: \"x\" in an attribute: a string the scanner cannot classify"],
    ),
    (
        "#[serde(skip_serializing_if = \"Option::is_none\", rename = \"x\")]\n#[serde(rename(serialize = \"a\"), rename_all = \"snake_case\")]\n#[error(\"{0} failed\")]\n#[expect(clippy::too_many_lines, reason = \"std::fs\")]\n#[cfg(feature = \"std::fs\")]\n#[deprecated(note = \"n\")]",
        &[],
    ),
    (
        "let a = url.socket_addrs(|| None);\ntime::UtcOffset::local_offset_at(t);\nlet s = time::util::refresh_tz();",
        &["1: socket_addrs: an effectful item of url 2.5.8", "2: local_offset_at: an effectful item of time 0.3.55", "3: refresh_tz: an effectful item of time 0.3.55"],
    ),
    (
        "mod tests {\n    fn f() {}\n}\n#[cfg(not(test))]\nmod tests {\n    pub(crate) mod evil;\n}\n#[cfg(test)]\nmod tests {}\n#[cfg(test)]\n#[allow(dead_code)]\npub mod tests;",
        &["1: a `mod tests` outside #[cfg(test)]", "5: a `mod tests` outside #[cfg(test)]"],
    ),
    (
        "macro_rules! hide {\n    () => { mod hidden; };\n}\nmacro_rules! paren ( () => ( mod p; ) );",
        &["2: a `mod` inside a macro_rules! body", "4: a `mod` inside a macro_rules! body"],
    ),
    (
        "macro_rules! decl {\n    ($n:ident) => {\n        mod $n;\n    };\n}\ndecl!(clock);",
        &["3: a `mod` inside a macro_rules! body"],
    ),
    (
        "macro_rules! mods {\n    ($($m:ident),*) => { $(pub mod $m;)* };\n}\nmods!(a, b);\nmacro_rules! raw { () => { r#mod x; } }",
        &["2: a `mod` inside a macro_rules! body", "5: a `mod` inside a macro_rules! body"],
    ),
    (
        "#![allow(renamed_and_removed_lints)]\n#[allow(clippy::disallowed_method)]\nfn f() {}",
        &["1: an attribute that lowers a purity lint", "2: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nextern crate self as brokkr_core;\n#[cfg(test)]\n#[path = \"../../../tests/support/envelope.rs\"]\nmod envelope_builder;",
        &[],
    ),
    (
        "#[path = \"../../../tests/support/envelope.rs\"]\nmod envelope_builder;\nextern crate self as brokkr_core;",
        &["1: a #[path] attribute", "3: an extern crate"],
    ),
    (
        "#[cfg(test)]\n#[path = \"other.rs\"]\nmod a;\n#[cfg(test)]\n#[path = \"../../../../tests/b.rs\"]\nmod b;\n#[cfg(test)]\n#[path = r\"../../../tests/c.rs\"]\nmod c;",
        &["2: a #[path] attribute", "5: a #[path] attribute", "8: a #[path] attribute"],
    ),
    (
        "mod m {\n    #[cfg(test)]\n    #[path = \"../../../tests/support/envelope.rs\"]\n    mod d;\n}\n#[cfg(any(test))]\n#[path = \"../../../tests/support/envelope.rs\"]\nmod e;\n#[path = \"../../../tests/support/envelope.rs\"]\n#[cfg(test)]\nmod f;\n#[cfg(test)]\n#[cfg_attr(test, path = \"../../../tests/support/envelope.rs\")]\nmod g;",
        &["3: a #[path] attribute", "7: a #[path] attribute", "9: a #[path] attribute", "13: a #[path] attribute"],
    ),
    (
        "#[cfg(test)]\n#[path = \"fold_tests.rs\"]\nmod h;\n#[cfg(test)]\n#[path = \"inner/tests.rs\"]\nmod i;",
        &[],
    ),
    (
        "#[cfg(test)]\nextern crate self as brokkr_view;\n#[cfg(test)]\nextern crate self;\n#[cfg(test)]\nextern crate serde as brokkr_core;\n#[cfg(any(test))]\nextern crate self as brokkr_core;",
        &["2: an extern crate", "4: an extern crate", "6: an extern crate", "8: an extern crate"],
    ),
    (
        "mod m {\n    macro_rules! include { () => {}; }\n    include!();\n}\ninclude!(\"../../../outside/pid.rs\");",
        &["2: macro_rules! include: shadows a builtin macro", "3: include!: outside the pure macro allowlist", "5: include!: outside the pure macro allowlist"],
    ),
    (
        "m!();\nmacro_rules! m { () => {}; }\nm!();\nmod a {\n    macro_rules! n { () => {}; }\n}\nn!();",
        &["1: m!: outside the pure macro allowlist", "7: n!: outside the pure macro allowlist"],
    ),
    (
        "macro_rules! vec { () => {}; }\nmacro_rules! env { () => {}; }\nenv!();",
        &["1: macro_rules! vec: shadows a builtin macro", "2: macro_rules! env: shadows a builtin macro", "3: env!: outside the pure macro allowlist"],
    ),
    (
        "#[allow(clippy::disallowed_types)]\nfn f() {}\n#![expect(clippy::all)]\n#[cfg_attr(test, allow(warnings))]\nfn g() {}\n#[warn(clippy::style)]\nfn h() {}\n#[allow(clippy::too_many_arguments)]\nfn i() {}\n#[deny(warnings)]\nfn j() {}",
        &["1: an attribute that lowers a purity lint", "3: an attribute that lowers a purity lint", "4: an attribute that lowers a purity lint", "6: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nmod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<usize> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n    }\n}",
        &[],
    ),
    (
        "mod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<usize> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n    }\n}",
        &["2: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nmod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<usize> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n        pub fn leak() {}\n    }\n}",
        &["3: an attribute that lowers a purity lint"],
    ),
    (
        "#[cfg(test)]\nmod observe {\n    #[expect(clippy::disallowed_types, reason = \"r\")]\n    mod cells {\n        use std::cell::Cell;\n        thread_local! {\n            pub(super) static A: Cell<u64> = const { Cell::new(0) };\n            pub(super) static B: Cell<usize> = const { Cell::new(0) };\n        }\n    }\n}",
        &["3: an attribute that lowers a purity lint"],
    ),
    (
        "use std::collections::{BTreeMap, BTreeSet};\nuse std::{fmt, rc::Rc, str::FromStr};\nlet s = std::str::from_utf8(b).map(std::mem::take);\nstd::iter::once::<u8>(1);\nmacro_rules! m { () => {} }\nm!();\nfn f<'a>() -> impl Sized + use<'a> { m!() }\nmod inner { fn g() { m!(); } }\n#",
        &[],
    ),
    (
        "\"open",
        &[],
    ),
    (
        "/* open",
        &[],
    ),
    (
        "r#\"open",
        &[],
    ),
    (
        "'",
        &[],
    ),
];

/// The scanner lexes before it matches, so no comment or literal hides a
/// path or poses as one, and it refuses every form the allowlists do not
/// name: an unlisted module or item, a rename or glob of a root, a nested
/// group, a macro's metavariable, an unlisted macro, a builtin shadowed or
/// called out of scope, an `extern crate`, a `#[path]`, an ungated
/// `mod tests;`, a lowered purity lint and a clock or randomness source.
#[test]
fn the_scanner_refuses_every_form_its_allowlists_do_not_name() {
    for (source, expected) in SCANNER_CASES {
        let file = Path::new("crates/brokkr-core/src/lib.rs");
        assert_eq!(impurities(source, file), expected, "{source}");
    }
}

/// A pure crate's modules as rustc builds them, followed from its
/// `src/lib.rs` through every `mod` declaration, inline or file, with each
/// `#[path]`: the workspace-relative files some chain of declarations
/// reaches outside exactly `#[cfg(test)]`, the files only test code
/// reaches, and each production declaration the scan cannot follow.
#[derive(Default)]
struct ModuleTree {
    production: BTreeSet<PathBuf>,
    test_only: BTreeSet<PathBuf>,
    refused: Vec<String>,
}

/// The directory a file's child modules live in: its own for `lib.rs`,
/// `main.rs` and `mod.rs`, one named after it for any other file.
fn module_dir(file: &Path) -> PathBuf {
    let parent = file.parent().unwrap_or(Path::new(""));
    match file.file_name().and_then(|name| name.to_str()) {
        Some("lib.rs" | "main.rs" | "mod.rs") => parent.to_path_buf(),
        _ => parent.join(file.file_stem().unwrap_or_default()),
    }
}

/// The names of the inline modules around token `at`, outermost first,
/// and whether any of them is declared under exactly `#[cfg(test)]`.
fn inline_chain(tokens: &[Token], blocks: &Blocks, at: usize) -> (Vec<String>, bool) {
    let (mut names, mut test) = (Vec::new(), false);
    let mut block = blocks.at[at];
    while let Some(open) = block {
        if open >= 2 && text(tokens, open - 2) == "mod" {
            names.push(text(tokens, open - 1).to_string());
            test |= gated_by_cfg_test(tokens, &outer_attributes(tokens, open - 2));
        }
        block = blocks.parent[&open];
    }
    names.reverse();
    (names, test)
}

/// The file a `mod name;` at `at` in `file` resolves to, as rustc finds
/// it, or why the scan cannot say: a `#[path]` that is not a plain string,
/// a path climbing out of the workspace, or not exactly one candidate.
fn module_file(
    scan: &Scanned,
    root: &Path,
    at: usize,
    inline: &[String],
) -> Result<PathBuf, String> {
    let name = text(scan.tokens, at + 1);
    let mut dir = module_dir(scan.file);
    dir.extend(inline);
    let attributes = outer_attributes(scan.tokens, at);
    let path_attribute = attributes
        .iter()
        .find(|&&(from, to)| (from..=to).any(|index| text(scan.tokens, index) == "path"));
    if let Some(&(from, to)) = path_attribute {
        let literal = plain_path(scan, from, to);
        let base = if inline.is_empty() {
            scan.file.parent().unwrap_or(Path::new("")).to_path_buf()
        } else {
            dir
        };
        return literal
            .and_then(|path| resolve(&base, path))
            .ok_or_else(|| format!("mod {name}: a #[path] the scan cannot follow"));
    }
    let found: Vec<PathBuf> = [
        dir.join(format!("{name}.rs")),
        dir.join(name).join("mod.rs"),
    ]
    .into_iter()
    .filter(|candidate| root.join(candidate).is_file())
    .collect();
    match found.as_slice() {
        [file] => Ok(file.clone()),
        _ => Err(format!("mod {name}: {} candidate files", found.len())),
    }
}

/// The module tree of the crate whose root is `lib`, both relative to the
/// workspace `root`. A file reached by any production chain is
/// production; one reached only through a declaration under exactly
/// `#[cfg(test)]`, or inside one, is test code. Every production `mod` the
/// resolver does not follow, one generated in a `macro_rules!` body
/// included, is refused rather than skipped.
fn resolve_modules(root: &Path, lib: &Path) -> ModuleTree {
    let mut tree = ModuleTree::default();
    let mut pending = vec![(lib.to_path_buf(), false)];
    let mut seen = BTreeSet::new();
    while let Some((file, test)) = pending.pop() {
        if !seen.insert((file.clone(), test)) {
            continue;
        }
        if test {
            tree.test_only.insert(file.clone());
        } else {
            tree.production.insert(file.clone());
        }
        let Ok(source) = std::fs::read_to_string(root.join(&file)) else {
            tree.refused.push(format!("{}: unreadable", file.display()));
            continue;
        };
        let (tokens, literals) = lex_with_literals(&source);
        let blocks = Blocks::of(&tokens);
        let scan = Scanned {
            tokens: &tokens,
            literals: &literals,
            blocks: &blocks,
            file: &file,
        };
        let bodies = macro_bodies(&tokens);
        for at in 0..tokens.len() {
            if text(&tokens, at) != "mod" {
                continue;
            }
            let in_macro = bodies.iter().any(|(from, to)| (*from..*to).contains(&at));
            let named = !in_macro && is_ident(text(&tokens, at + 1));
            if named && text(&tokens, at + 2) == "{" {
                continue;
            }
            let (inline, chain_test) = inline_chain(&tokens, &blocks, at);
            let gated = test || chain_test || cfg_test_gated(&tokens, at);
            let child = if named && text(&tokens, at + 2) == ";" {
                module_file(&scan, root, at, &inline)
            } else {
                Err(format!(
                    "mod at line {}: the scan cannot follow it",
                    tokens[at].0
                ))
            };
            match child {
                Ok(child) => pending.push((child, gated)),
                Err(why) if !gated => tree.refused.push(format!("{}: {why}", file.display())),
                Err(_) => {}
            }
        }
    }
    tree.test_only
        .retain(|file| !tree.production.contains(file));
    tree
}

/// The resolver follows declarations, not names: a `tests` module outside
/// `#[cfg(test)]` is production wherever its file lies, a gated module is
/// test code whatever it is called, and a production declaration the scan
/// cannot follow is refused.
#[test]
fn the_module_tree_follows_declarations_not_file_names() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("layering-modules");
    let src = Path::new("crates/brokkr-core/src");
    let files = [
        (
            "lib.rs",
            "#[cfg(test)]\nmod tests;\n#[cfg(not(test))]\nmod tests {\n    pub(crate) mod evil;\n}\n\
             pub mod plain;\n#[cfg(test)]\nmod gated {\n    mod inner;\n}\nmod missing;\n\
             #[cfg(test)]\nmod absent;\n#[path = \"moved.rs\"]\nmod renamed;\n\
             macro_rules! hide { () => { mod hidden; } }\n\
             macro_rules! decl { ($n:ident) => { mod $n; }; }\ndecl!(clock);\n",
        ),
        ("tests.rs", ""),
        ("tests/evil.rs", ""),
        ("plain.rs", "mod child;\n"),
        ("plain/child.rs", ""),
        ("gated/inner.rs", ""),
        ("moved.rs", ""),
        ("hidden.rs", ""),
        ("clock.rs", "pub fn pid() -> u32 { std::os::unix::process::parent_id() }\n"),
    ];
    let _ = std::fs::remove_dir_all(&root);
    for (name, source) in files {
        let path = root.join(src).join(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("a scratch dir");
        std::fs::write(&path, source).expect("a scratch file");
    }
    let tree = resolve_modules(&root, &src.join("lib.rs"));
    let names = |set: &BTreeSet<PathBuf>| -> Vec<String> {
        set.iter()
            .map(|file| {
                file.strip_prefix(src)
                    .expect("under src")
                    .display()
                    .to_string()
            })
            .collect()
    };
    assert_eq!(
        names(&tree.production),
        [
            "lib.rs",
            "moved.rs",
            "plain/child.rs",
            "plain.rs",
            "tests/evil.rs"
        ]
    );
    assert_eq!(names(&tree.test_only), ["gated/inner.rs", "tests.rs"]);
    assert_eq!(
        tree.refused,
        [
            "crates/brokkr-core/src/lib.rs: mod missing: 0 candidate files",
            "crates/brokkr-core/src/lib.rs: mod at line 17: the scan cannot follow it",
            "crates/brokkr-core/src/lib.rs: mod at line 18: the scan cannot follow it",
        ]
    );
}

/// Ruling 1: core's and view's production source names nothing outside
/// the allowlists, and every file their module trees compile outside
/// `#[cfg(test)]` is scanned.
#[test]
fn the_pure_crates_name_nothing_outside_the_allowlists() {
    let root = workspace();
    let (mut production, mut test_only, mut found) = (BTreeSet::new(), BTreeSet::new(), Vec::new());
    for krate in ["brokkr-core", "brokkr-view"] {
        let tree = resolve_modules(&root, &Path::new("crates").join(krate).join("src/lib.rs"));
        production.extend(tree.production);
        test_only.extend(tree.test_only);
        found.extend(tree.refused);
    }
    for relative in &production {
        let source = std::fs::read_to_string(root.join(relative)).expect("readable source");
        found.extend(
            impurities(&source, relative)
                .into_iter()
                .map(|what| format!("{}:{what}", relative.display())),
        );
    }
    assert_eq!(
        found,
        Vec::<String>::new(),
        "core and view are pure (decision 0071 ruling 1): effects live in store and runtime"
    );
    let crates = Path::new("crates");
    assert!(production.contains(&crates.join("brokkr-core/src/lib.rs")));
    assert!(production.contains(&crates.join("brokkr-view/src/transcript.rs")));
    assert!(test_only.contains(&crates.join("brokkr-view/src/transcript/tests.rs")));
    assert!(test_only.contains(&crates.join("brokkr-view/src/tests.rs")));
    assert!(test_only.contains(Path::new("tests/support/envelope.rs")));
}
