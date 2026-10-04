//! The `--test it` commands a tracked file runs, read word by word as the
//! shell reads them (#423; the operator's ruling of 2026-10-04, "do exact
//! command line parsing").
//!
//! A file is first cut into the lines its format makes. A workflow's
//! literal block (`|`) is its lines; a folded block (`>`, `>-`) or a plain
//! or quoted value that wraps is folded, as YAML folds it. A Markdown code
//! span that wraps is joined, as Markdown joins it, and a prose line that
//! does not itself name `--test` gives only its code spans. Each line is
//! then split into words as the shell splits it: quotes, backslashes and
//! continuations; `|`, `&`, `;`, a parenthesis or a backtick ends one
//! command; a redirect takes its target, which the command never sees,
//! and the words after it are still the command's. A comment, and a
//! quoted word that holds `--test`, are read again as lines of their own,
//! since a person or a wrapper may run them.
//!
//! In each command the words after `cargo` are an invocation. One that
//! passes `--test it` (as `--test it`, `--test=it`, or quoted) is read by
//! closed tables of the flags Cargo and libtest take, and every word the
//! tables do not consume is a filter: before the flag, after it and after
//! `--`. A word, flag or form outside the tables is refused, never
//! skipped; so is a `--test it` outside such an invocation, a line the
//! shell could not split that holds `--test`, and a `--test` whose value
//! the line's end would hide.
//!
//! The threat model is the operator's of 2026-09-26: realistic accidental
//! misuse is caught and what cannot be read is refused. A quote is read
//! within its line, so a command quoted across lines is read in parts;
//! that, like a flag spelled in split quotes on a line the shell could not
//! split, is a low residual.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use super::{is_identifier, unreadable, Refusal};

/// The modules each package's `tests/it.rs` declares, by package and name.
type Roots = BTreeMap<String, BTreeMap<String, PathBuf>>;

/// How the reader reads a module's file, to find an `--exact` name in it.
type Read<'r> = &'r dyn Fn(&Path) -> io::Result<String>;

/// One line as a command reads it: the number of the file line it starts
/// on, and its text.
type Line = (usize, String);

/// One `--test it` filter as a command passes it: where, a package its
/// `-p` names, and the module its first path segment names.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Filter {
    pub(super) at: String,
    pub(super) package: String,
    pub(super) module: String,
}

/// What a flag the reader knows takes, and what it tells the reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flag {
    /// Takes nothing and selects no test.
    Switch,
    /// Takes an argument that is not a filter.
    Valued,
    /// `-p`/`--package`: a package whose root holds the filters.
    Package,
    /// `--exact`: each filter is one whole test name.
    Exact,
}

impl Flag {
    fn takes_argument(self) -> bool {
        match self {
            Self::Switch | Self::Exact => false,
            Self::Valued | Self::Package => true,
        }
    }
}

/// A closed table of flags, by the name each is written with.
type Table = &'static [(&'static str, Flag)];

/// Cargo's own flags, which it takes before the subcommand as well.
const CARGO: Table = &[
    ("--locked", Flag::Switch),
    ("--offline", Flag::Switch),
    ("--frozen", Flag::Switch),
    ("-q", Flag::Switch),
    ("--quiet", Flag::Switch),
    ("-v", Flag::Switch),
    ("--verbose", Flag::Switch),
    ("--color", Flag::Valued),
    ("--config", Flag::Valued),
    ("-Z", Flag::Valued),
];

/// `cargo test`'s flags, before `--`.
const CARGO_TEST: Table = &[
    ("-p", Flag::Package),
    ("--package", Flag::Package),
    ("--test", Flag::Valued),
    ("-F", Flag::Valued),
    ("--features", Flag::Valued),
    ("--all-features", Flag::Switch),
    ("--no-default-features", Flag::Switch),
    ("--release", Flag::Switch),
    ("--no-fail-fast", Flag::Switch),
    ("-j", Flag::Valued),
    ("--jobs", Flag::Valued),
    ("--target-dir", Flag::Valued),
];

/// libtest's flags, after `--`.
const LIBTEST: Table = &[
    ("--exact", Flag::Exact),
    ("--skip", Flag::Valued),
    ("--ignored", Flag::Switch),
    ("--include-ignored", Flag::Switch),
    ("--nocapture", Flag::Switch),
    ("--no-capture", Flag::Switch),
    ("--show-output", Flag::Switch),
    ("--test-threads", Flag::Valued),
    ("-q", Flag::Switch),
    ("--quiet", Flag::Switch),
    ("--color", Flag::Valued),
    ("--format", Flag::Valued),
];

/// A `cargo test` invocation as the tables read it.
#[derive(Debug, Default, PartialEq, Eq)]
struct Invocation {
    packages: Vec<String>,
    filters: Vec<String>,
    exact: bool,
}

/// A shell token.
#[derive(Debug, PartialEq, Eq)]
enum Token {
    /// A word as the command receives it, its quotes and escapes resolved.
    Word(String),
    /// The word a redirect names, which the command never receives.
    Target(String),
    /// `|`, `&`, `;`, a parenthesis or a backtick: one command ends.
    Break,
    /// The text after a `#` that starts a word.
    Comment(String),
}

/// Why the shell could not split a line.
#[derive(Debug, PartialEq, Eq)]
enum Unlexed {
    /// A backslash ends it: the command goes on on the next line.
    Continues,
    /// A quote it never closes.
    Unclosed(char),
    /// A redirect with no word to name.
    Dangling(String),
}

impl Unlexed {
    /// The text the refusal names.
    fn word(&self) -> String {
        match self {
            Self::Continues => "\\".to_string(),
            Self::Unclosed(quote) => quote.to_string(),
            Self::Dangling(operator) => operator.clone(),
        }
    }
}

/// The shell's split of one line, built a character at a time.
#[derive(Default)]
struct Lexer {
    tokens: Vec<Token>,
    word: String,
    /// Whether a word has begun, which an empty quoted word does.
    started: bool,
    /// The redirect whose target the next word is.
    redirect: Option<String>,
}

type Chars<'t> = std::iter::Peekable<std::str::Chars<'t>>;

impl Lexer {
    fn push(&mut self, c: char) {
        self.word.push(c);
        self.started = true;
    }

    fn end_word(&mut self) {
        if !std::mem::take(&mut self.started) {
            return;
        }
        let word = std::mem::take(&mut self.word);
        self.tokens.push(match self.redirect.take() {
            Some(_) => Token::Target(word),
            None => Token::Word(word),
        });
    }

    fn dangling(&self) -> Result<(), Unlexed> {
        match &self.redirect {
            Some(operator) => Err(Unlexed::Dangling(operator.clone())),
            None => Ok(()),
        }
    }

    fn command_ends(&mut self) -> Result<(), Unlexed> {
        self.end_word();
        self.dangling()?;
        self.tokens.push(Token::Break);
        Ok(())
    }

    fn single(&mut self, chars: &mut Chars<'_>) -> Result<(), Unlexed> {
        self.started = true;
        loop {
            match chars.next() {
                Some('\'') => return Ok(()),
                Some(c) => self.word.push(c),
                None => return Err(Unlexed::Unclosed('\'')),
            }
        }
    }

    /// A double-quoted part, in which a backslash escapes only `$`, a
    /// backtick, `"`, itself and the line's end.
    fn double(&mut self, chars: &mut Chars<'_>) -> Result<(), Unlexed> {
        self.started = true;
        loop {
            match chars.next() {
                Some('"') => return Ok(()),
                Some('\\') => match chars.next() {
                    Some(c @ ('$' | '`' | '"' | '\\')) => self.word.push(c),
                    Some(c) => self.word.extend(['\\', c]),
                    None => return Err(Unlexed::Continues),
                },
                Some(c) => self.word.push(c),
                None => return Err(Unlexed::Unclosed('"')),
            }
        }
    }

    /// A redirect: `>`, `>>`, `>|`, `>&`, `<`, `<<`, `<<<`, `<&`, `<>`,
    /// `&>` or `&>>`. A word of digits just before it names a descriptor,
    /// not an argument.
    fn redirect(&mut self, first: char, chars: &mut Chars<'_>) -> Result<(), Unlexed> {
        if self.started && self.word.chars().all(|c| c.is_ascii_digit()) {
            self.word.clear();
            self.started = false;
        } else {
            self.end_word();
        }
        self.dangling()?;
        let mut operator = first.to_string();
        while let Some(c) = chars.next_if(|&c| matches!(c, '<' | '>')) {
            operator.push(c);
        }
        operator.extend(chars.next_if(|&c| matches!(c, '&' | '|')));
        self.redirect = Some(operator);
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<Token>, Unlexed> {
        self.end_word();
        self.dangling()?;
        Ok(self.tokens)
    }
}

/// `text` split as the shell splits one line.
fn lex(text: &str) -> Result<Vec<Token>, Unlexed> {
    let mut lexer = Lexer::default();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => lexer.end_word(),
            '\'' => lexer.single(&mut chars)?,
            '"' => lexer.double(&mut chars)?,
            '\\' => lexer.push(chars.next().ok_or(Unlexed::Continues)?),
            '#' if !lexer.started => lexer.tokens.push(Token::Comment(chars.by_ref().collect())),
            '|' | ';' | '(' | ')' | '`' => lexer.command_ends()?,
            '&' if chars.peek() != Some(&'>') => lexer.command_ends()?,
            '&' | '<' | '>' => lexer.redirect(c, &mut chars)?,
            _ => lexer.push(c),
        }
    }
    lexer.finish()
}

/// Whether `text` holds `--test` as a flag could, not run into a longer
/// flag such as `--tests` or `--test-threads`.
fn mentions_test(text: &str) -> bool {
    text.match_indices("--test").any(|(at, flag)| {
        !text[at + flag.len()..]
            .starts_with(|c: char| c == '_' || c == '-' || c.is_ascii_alphanumeric())
    })
}

/// The target the `--test` at `words[index]` names, `Some(None)` when the
/// command ends before it does, and `None` when the word is no `--test`.
fn test_target(words: &[String], index: usize) -> Option<Option<&str>> {
    let word = words[index].as_str();
    match word.strip_prefix("--test=") {
        Some(target) => Some(Some(target)),
        None if word == "--test" => Some(words.get(index + 1).map(String::as_str)),
        None => None,
    }
}

fn unread_filter(at: &str, word: &str, command: &str) -> Refusal {
    Refusal::UnreadFilter {
        at: at.to_string(),
        word: word.to_string(),
        command: command.trim().to_string(),
    }
}

/// The flag `word` names in `tables`, with its argument: after `=`, or the
/// next word when it takes one. A form outside the tables is refused by
/// its word; a short flag is read only alone, so `-pname` is refused.
fn flag<'w>(
    tables: &[Table],
    word: &'w str,
    rest: &mut impl Iterator<Item = &'w str>,
) -> Result<(Flag, Option<&'w str>), &'w str> {
    let (name, inline) = match word.split_once('=') {
        Some((name, inline)) if name.starts_with("--") => (name, Some(inline)),
        _ => (word, None),
    };
    let mut known = tables.iter().flat_map(|table| table.iter());
    let (_, flag) = known.find(|(known, _)| *known == name).ok_or(word)?;
    match (flag.takes_argument(), inline) {
        (true, None) => Ok((*flag, Some(rest.next().ok_or(word)?))),
        (true, Some(_)) | (false, None) => Ok((*flag, inline)),
        (false, Some(_)) => Err(word),
    }
}

/// `args`, the words after `cargo`, read as `[+toolchain] [flags] test
/// [args]`. The word the tables cannot read is the error.
fn invocation(args: &[String]) -> Result<Invocation, &str> {
    let mut words = args.iter().map(String::as_str);
    let mut first = true;
    loop {
        match words.next().ok_or("cargo")? {
            "test" | "t" => break,
            toolchain if first && toolchain.starts_with('+') => {}
            word => {
                flag(&[CARGO], word, &mut words)?;
            }
        }
        first = false;
    }
    let (mut read, mut libtest) = (Invocation::default(), false);
    while let Some(word) = words.next() {
        if word == "--" && !libtest {
            libtest = true;
        } else if !word.starts_with('-') {
            read.filters.push(word.to_string());
        } else {
            let tables: &[Table] = if libtest {
                &[LIBTEST]
            } else {
                &[CARGO_TEST, CARGO]
            };
            match flag(tables, word, &mut words)? {
                (Flag::Package, package) => read.packages.extend(package.map(str::to_string)),
                (Flag::Exact, _) => read.exact = true,
                (Flag::Switch | Flag::Valued, _) => {}
            }
        }
    }
    Ok(read)
}

/// Every `--test it` filter in `text`, the file at `file`, each held to
/// the modules `roots` gives each package its command names, and an
/// `--exact` name to a `#[test]` of its module, read through `read`. The
/// first it cannot hold or read is refused.
pub(super) fn filters_in(
    file: &str,
    text: &str,
    roots: &Roots,
    read: Read<'_>,
) -> Result<Vec<Filter>, Refusal> {
    let lines = lines_of(file, text);
    let (mut filters, mut next) = (Vec::new(), 0);
    while let Some((number, first)) = lines.get(next) {
        let mut command = first.clone();
        next += 1;
        while let (Err(Unlexed::Continues), Some((_, more))) = (lex(&command), lines.get(next)) {
            command.pop();
            command.push_str(more);
            next += 1;
        }
        let (at, mut found) = (format!("{file}:{number}"), Vec::new());
        read_line(&at, &command, &mut found)?;
        for (command, invocation) in found {
            filters.extend(hold(&at, &command, &invocation, roots, read)?);
        }
    }
    Ok(filters)
}

/// One line's `--test it` invocations, each with the text it was read
/// from, into `found`.
fn read_line(at: &str, text: &str, found: &mut Vec<(String, Invocation)>) -> Result<(), Refusal> {
    let tokens = match lex(text) {
        Ok(tokens) => tokens,
        Err(_) if !mentions_test(text) => return Ok(()),
        Err(unlexed) => return Err(unread_filter(at, &unlexed.word(), text)),
    };
    let mut command = Vec::new();
    for token in tokens {
        match token {
            Token::Word(word) => command.push(word),
            Token::Target(target) if mentions_test(&target) => {
                return Err(unread_filter(at, &target, text));
            }
            Token::Target(_) => {}
            Token::Comment(comment) => read_line(at, &comment, found)?,
            Token::Break => read_command(at, text, &std::mem::take(&mut command), false, found)?,
        }
    }
    read_command(at, text, &command, true, found)
}

/// One command's words: an invocation that passes `--test it` by the
/// tables, and every other word loosely. `last` is whether the command
/// ends its line.
fn read_command(
    at: &str,
    text: &str,
    words: &[String],
    last: bool,
    found: &mut Vec<(String, Invocation)>,
) -> Result<(), Refusal> {
    let cargo = words
        .iter()
        .position(|word| word == "cargo" || word.ends_with("/cargo"));
    let Some(cargo) = cargo else {
        return loose(at, text, words, words.len(), last, found);
    };
    let args = &words[cargo + 1..];
    let passes_it = (0..args.len()).any(|index| test_target(args, index) == Some(Some("it")));
    if !passes_it {
        return loose(at, text, words, words.len(), last, found);
    }
    let invocation = invocation(args).map_err(|word| unread_filter(at, word, text))?;
    loose(at, text, words, cargo, false, found)?;
    found.push((text.trim().to_string(), invocation));
    Ok(())
}

/// The first `upto` of a command's words, outside any `--test it`
/// invocation: a `--test it` here is refused, as is a `--test` the line's
/// end leaves without a value and a word that runs `--test` into other
/// text; a quoted word that holds `--test` among other words is read as a
/// line. Only a space or a tab splits a word, and a word a quote made is
/// shorter than its text, so the reading ends.
fn loose(
    at: &str,
    text: &str,
    words: &[String],
    upto: usize,
    last: bool,
    found: &mut Vec<(String, Invocation)>,
) -> Result<(), Refusal> {
    for (index, word) in words.iter().enumerate().take(upto) {
        match test_target(words, index) {
            Some(Some("it")) => return Err(unread_filter(at, word, text)),
            Some(None) if last => return Err(unread_filter(at, word, text)),
            Some(_) => {}
            None if !mentions_test(word) => {}
            None if word.contains([' ', '\t']) && word.len() < text.len() => {
                read_line(at, word, found)?;
            }
            None => return Err(unread_filter(at, word, text)),
        }
    }
    Ok(())
}

/// An invocation's filters, held to the module `roots` gives each package
/// it names, and under `--exact` to a `#[test]` of that module.
fn hold(
    at: &str,
    command: &str,
    invocation: &Invocation,
    roots: &Roots,
    read: Read<'_>,
) -> Result<Vec<Filter>, Refusal> {
    if invocation.packages.is_empty() && !invocation.filters.is_empty() {
        return Err(Refusal::NoPackage {
            at: at.to_string(),
            command: command.to_string(),
        });
    }
    let mut held = Vec::new();
    for filter in &invocation.filters {
        let (module, name) =
            path_of(filter, invocation.exact).ok_or_else(|| unread_filter(at, filter, command))?;
        for package in &invocation.packages {
            let file =
                (roots.get(package).and_then(|modules| modules.get(module))).ok_or_else(|| {
                    Refusal::UnknownModule {
                        at: at.to_string(),
                        package: package.clone(),
                        module: module.to_string(),
                    }
                })?;
            if invocation.exact {
                let text = read(file).map_err(|error| unreadable(file, &error))?;
                if !tests_of(&text).contains(name) {
                    return Err(Refusal::UnknownTest {
                        at: at.to_string(),
                        package: package.clone(),
                        name: filter.clone(),
                    });
                }
            }
            held.push(Filter {
                at: at.to_string(),
                package: package.clone(),
                module: module.to_string(),
            });
        }
    }
    Ok(held)
}

/// A filter's module and the rest of its path: `<module>::…`, each
/// segment an identifier but an empty last one, or under `--exact` just
/// `<module>::<name>`.
fn path_of(filter: &str, exact: bool) -> Option<(&str, &str)> {
    let (module, rest) = filter.split_once("::")?;
    let segments: Vec<&str> = rest.split("::").collect();
    let (last, inner) = segments.split_last()?;
    let readable = is_identifier(module)
        && inner.iter().all(|segment| is_identifier(segment))
        && (is_identifier(last) || (last.is_empty() && !exact));
    (readable && (!exact || inner.is_empty())).then_some((module, rest))
}

/// The `#[test]` functions at the top level of a module's text: a `fn` at
/// a line's start under a run of attributes and doc comments, also at
/// their lines' start, that holds `#[test]`.
fn tests_of(text: &str) -> BTreeSet<&str> {
    let (mut tests, mut marked) = (BTreeSet::new(), false);
    for line in text.lines() {
        if marked {
            let name = line
                .strip_prefix("fn ")
                .and_then(|rest| rest.split_once('('));
            tests.extend(name.map(|(name, _)| name));
        }
        marked =
            line == "#[test]" || (marked && (line.starts_with("#[") || line.starts_with("///")));
    }
    tests
}

/// A file's lines as its format makes them.
fn lines_of(file: &str, text: &str) -> Vec<Line> {
    match Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("yml" | "yaml") => yaml_lines(text),
        Some("md") => markdown_lines(text),
        _ => (text.lines().enumerate())
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect(),
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// A YAML block scalar's style.
enum Block {
    /// `|`: its lines are kept.
    Literal,
    /// `>`: its lines are folded.
    Folded,
}

/// The block style a value's header names, with its chomping and
/// indentation indicators and a comment after it; none for a value.
fn block_of(value: &str) -> Option<Block> {
    let header = value
        .split_once(" #")
        .map_or(value, |(header, _)| header)
        .trim_end();
    let indicators = |rest: &str| rest.chars().all(|c| matches!(c, '-' | '+' | '1'..='9'));
    if header.strip_prefix('|').is_some_and(indicators) {
        Some(Block::Literal)
    } else if header.strip_prefix('>').is_some_and(indicators) {
        Some(Block::Folded)
    } else {
        None
    }
}

/// The column a line's value hangs from, with the value, when it holds
/// one: `key: value` from the key's column, `- value` from the dash's. A
/// comment, or a key with no value, holds none.
fn scalar_of(line: &str) -> Option<(usize, &str)> {
    let column = |rest: &str| line.len() - rest.len();
    let trimmed = line.trim_start();
    let item = trimmed.strip_prefix("- ").map(str::trim_start);
    let node = item.unwrap_or(trimmed);
    let is_key = |key: &str| {
        !key.is_empty()
            && key
                .chars()
                .all(|c| c == '_' || c == '-' || c.is_ascii_alphanumeric())
    };
    let (at, value) = match node.split_once(": ") {
        Some((key, value)) if is_key(key) => (column(node), value.trim()),
        _ if item.is_some() => (column(trimmed), node.trim()),
        _ => return None,
    };
    (!value.is_empty() && !value.starts_with('#')).then_some((at, value))
}

/// Folds `lines`, numbered from `first`, as YAML folds a scalar: lines join
/// with a space and a blank line starts a new one; in a block indented to
/// `block`, a line indented past it stands alone.
fn fold(lines: &[&str], first: usize, block: Option<usize>, read: &mut Vec<Line>) {
    let mut open: Option<Line> = None;
    for (offset, line) in lines.iter().enumerate() {
        let blank = line.trim().is_empty();
        if blank || block.is_some_and(|base| indent(line) > base) {
            read.extend(open.take());
            read.extend((!blank).then(|| (first + offset, line.to_string())));
        } else if let Some((_, text)) = open.as_mut() {
            text.push(' ');
            text.push_str(line.trim());
        } else {
            open = Some((first + offset, line.to_string()));
        }
    }
    read.extend(open);
}

/// A workflow's lines as YAML holds its scalars: a literal block line by
/// line, and a folded block, or a plain or quoted value that wraps,
/// folded. Every other line is its own.
fn yaml_lines(text: &str) -> Vec<Line> {
    let lines: Vec<&str> = text.lines().collect();
    let (mut read, mut index) = (Vec::new(), 0);
    while let Some(line) = lines.get(index) {
        let Some((column, value)) = scalar_of(line) else {
            read.push((index + 1, line.to_string()));
            index += 1;
            continue;
        };
        let body = (lines[index + 1..].iter())
            .take_while(|body| body.trim().is_empty() || indent(body) > column)
            .count();
        let end = index + 1 + body;
        match block_of(value) {
            Some(Block::Literal) => {
                read.extend((index..end).map(|at| (at + 1, lines[at].to_string())));
            }
            Some(Block::Folded) => {
                read.push((index + 1, line.to_string()));
                let body = &lines[index + 1..end];
                let base = body.iter().find(|line| !line.trim().is_empty());
                fold(
                    body,
                    index + 2,
                    Some(base.map_or(0, |line| indent(line))),
                    &mut read,
                );
            }
            None => fold(&lines[index..end], index + 1, None, &mut read),
        }
        index = end;
    }
    read
}

/// A Markdown file's lines: a code span a line leaves open joined to the
/// next with a space, as Markdown joins a paragraph, until it closes or a
/// blank line ends the paragraph. A fence and the lines between fences
/// stand alone.
fn markdown_lines(text: &str) -> Vec<Line> {
    let (mut joined, mut open, mut fenced) = (Vec::new(), None::<Line>, false);
    for (index, line) in text.lines().enumerate() {
        let current = match open.take() {
            Some((number, span)) if !line.trim().is_empty() => (number, format!("{span} {line}")),
            left => {
                joined.extend(left.map(|left| (left, false)));
                let fence = line.trim_start().starts_with("```");
                fenced ^= fence;
                if fence || fenced {
                    joined.push(((index + 1, line.to_string()), true));
                    continue;
                }
                (index + 1, line.to_string())
            }
        };
        if current.1.matches('`').count() % 2 == 0 {
            joined.push((current, false));
        } else {
            open = Some(current);
        }
    }
    joined.extend(open.map(|open| (open, false)));
    (joined.into_iter())
        .flat_map(|(line, code)| code_of(line, code))
        .collect()
}

/// The commands a Markdown line holds: a fenced line whole; a prose line
/// whole when it has no code span or its prose itself names `--test`, and
/// otherwise each of its code spans.
fn code_of((number, text): Line, fenced: bool) -> Vec<Line> {
    let parts: Vec<&str> = text.split('`').collect();
    let prose_names_test = parts.iter().step_by(2).any(|part| mentions_test(part));
    if fenced || parts.len() == 1 || prose_names_test {
        return vec![(number, text)];
    }
    (parts.iter().skip(1).step_by(2))
        .map(|span| (number, (*span).to_string()))
        .collect()
}

/// The filter reader reads each form of `--test it` Cargo takes, every
/// filter around it, a wrapper's, a comment's and each file format's
/// lines, and holds each filter to its crate's root and an `--exact` name
/// to a test of its module.
#[test]
fn the_filter_reader_holds_each_filter_to_its_crates_root() {
    let (roots, read) = fixture();
    let cli = |at: &str, module: &str| Filter {
        at: at.into(),
        package: "brokkr-cli".into(),
        module: module.into(),
    };
    for (file, text, held) in [
        ("f", "cargo test --locked -p brokkr-cli --test it packaging::\n", vec![cli("f:1", "packaging")]),
        ("f", "cargo +nightly --locked test -p brokkr-cli --test=it -- --ignored --exact suppressions::a_b\n", vec![cli("f:1", "suppressions")]),
        ("f", "\nX=1 cargo test -p brokkr-cli --test it \\\n  suppressions::a > /dev/null 2>&1\n", vec![cli("f:2", "suppressions")]),
        ("f", "cargo test -p brokkr-cli --test it -- --skip gone:: --test-threads=1 --nocapture packaging::\n", vec![cli("f:1", "packaging")]),
        ("f", "run \"cargo test -p brokkr-cli --test it packaging::\" cargo test -p brokkr-cli --test it suppressions::\n", vec![cli("f:1", "packaging"), cli("f:1", "suppressions")]),
        ("f", "# Regenerate: X=1 cargo test -p brokkr-cli --test it suppressions::\n", vec![cli("f:1", "suppressions")]),
        ("f", "cargo test -p brokkr-cli --test it\ncargo test -p brokkr-cli --test heap_dsh x\ncargo test --tests --test-threads 1 x | tee out\n", vec![]),
        ("f.yml", "    run: |\n      cargo test -p brokkr-cli --test it packaging::\n      echo gone::\n", vec![cli("f.yml:2", "packaging")]),
        ("f.yml", "    run: >-\n      cargo test -p brokkr-cli\n      --test it packaging::\n", vec![cli("f.yml:2", "packaging")]),
        ("f.yml", "  - run: cargo test -p brokkr-cli\n      --test it suppressions::\n  - run: echo gone::\n", vec![cli("f.yml:1", "suppressions")]),
        ("f.md", "Run `cargo test -p brokkr-cli --test\nit packaging::` once; it's quick.\n```sh\ncargo test -p brokkr-cli --test it suppressions::\n```\n", vec![cli("f.md:1", "packaging"), cli("f.md:4", "suppressions")]),
    ] {
        assert_eq!(filters_in(file, text, &roots, &read), Ok(held), "{text}");
    }
}

/// Every spelling of a stale filter the shell would pass is held and
/// refused: each form of `--test it`, a filter after `--skip`'s argument,
/// a second filter, one before the flag, one after a redirect, a second
/// package, and a folded YAML block or a wrapped Markdown span.
#[test]
fn the_filter_reader_refuses_a_stale_filter_in_every_form() {
    let (roots, read) = fixture();
    let refusal = |at: &str, package: &str, module: &str| Refusal::UnknownModule {
        at: at.into(),
        package: package.into(),
        module: module.into(),
    };
    let stale = [
        ("f", "cargo test -p brokkr-cli --test=it no_such_file::"),
        ("f", "cargo test -p brokkr-cli --test  it no_such_file::"),
        ("f", "cargo test -p brokkr-cli --test\tit no_such_file::"),
        ("f", "cargo test -p brokkr-cli --test 'it' no_such_file::"),
        (
            "f",
            "cargo test -p brokkr-cli --test \\\n  it no_such_file::",
        ),
        (
            "f",
            "cargo test -p brokkr-cli --test it -- --skip packaging:: no_such_file::",
        ),
        (
            "f",
            "cargo test -p brokkr-cli --test it -- packaging:: no_such_file::",
        ),
        (
            "f",
            "cargo test --locked -p brokkr-cli no_such_file:: --test it",
        ),
        (
            "f",
            "cargo test -p brokkr-cli --test it 2>&1 no_such_file::",
        ),
        (
            "f",
            "cargo test -p brokkr-cli -p brokkr-core --test it no_such_file::",
        ),
        (
            "f.yml",
            "run: >-\n  cargo test -p brokkr-cli --test it packaging::\n  no_such_file::",
        ),
        (
            "f.md",
            "Run `cargo test -p brokkr-cli --test it\nno_such_file::` once.",
        ),
    ];
    let at = |file| format!("{file}:{}", if file == "f.yml" { 2 } else { 1 });
    let stale = (stale.into_iter())
        .map(|(file, text)| (file, text, refusal(&at(file), "brokkr-cli", "no_such_file")));
    let exact = |name: &str| Refusal::UnknownTest {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        name: name.into(),
    };
    let no_package = "cargo b && cargo test --test it packaging::";
    let other = [
        (
            "cargo test -p brokkr-cli -p brokkr-core --test it packaging::",
            refusal("f:1", "brokkr-core", "packaging"),
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact suppressions::helper",
            exact("suppressions::helper"),
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact suppressions::nested",
            exact("suppressions::nested"),
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact packaging::a",
            Refusal::Unreadable {
                path: "packaging.rs".into(),
                kind: io::ErrorKind::NotFound,
            },
        ),
        (
            no_package,
            Refusal::NoPackage {
                at: "f:1".into(),
                command: no_package.into(),
            },
        ),
    ];
    for (file, text, refused) in stale.chain(
        other
            .into_iter()
            .map(|(text, refused)| ("f", text, refused)),
    ) {
        assert_eq!(
            filters_in(file, text, &roots, &read),
            Err(refused),
            "{text}"
        );
    }
}

/// Every word, flag or form the reader does not know is refused by its
/// word, never skipped: a filter that is not a module path, a flag outside
/// the tables or in a form they do not take, a subcommand other than
/// `test`, a `--test it` outside `cargo test`, a `--test` the line's end
/// cuts off or run into other text, and a line the shell cannot split.
#[test]
fn the_filter_reader_refuses_a_word_or_form_it_cannot_read() {
    let (roots, read) = fixture();
    let unreadable = [
        ("cargo test -p brokkr-cli --test it packaging", "packaging"),
        (
            "cargo test -p brokkr-cli --test it \"packaging:: x\"",
            "packaging:: x",
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact suppressions::a::b",
            "suppressions::a::b",
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact suppressions::",
            "suppressions::",
        ),
        (
            "cargo test -p brokkr-cli --test it --workspace x::",
            "--workspace",
        ),
        ("cargo test -pbrokkr-cli --test it x::", "-pbrokkr-cli"),
        (
            "cargo test --locked=1 -p brokkr-cli --test it x::",
            "--locked=1",
        ),
        (
            "cargo test -p brokkr-cli --test it --test-threads 1 x::",
            "--test-threads",
        ),
        ("cargo test -p brokkr-cli --test it -- -- x::", "--"),
        ("cargo test -p brokkr-cli --test it -- x:: --skip", "--skip"),
        ("cargo -C crates test -p brokkr-cli --test it x::", "-C"),
        ("cargo build -p brokkr-cli --test it x::", "build"),
        ("cargo --color --test=it", "cargo"),
        ("report -p brokkr-cli --test it x::", "--test"),
        ("report -p brokkr-cli --test=it x::", "--test=it"),
        ("cargo test -p brokkr-cli --test", "--test"),
        ("cargo test -p brokkr-cli x--test it", "x--test"),
        ("cargo test -p brokkr-cli --test it 'x::", "'"),
        ("cargo test -p brokkr-cli --test it \"x::", "\""),
        ("cargo test -p brokkr-cli --test it x:: >", ">"),
        ("cargo test -p brokkr-cli --test it x:: \\", "\\"),
        ("cargo test -p brokkr-cli --test it x:: > --test", "--test"),
    ];
    let space = char::from(0xa0);
    let (nbsp, joined) = (
        format!("cargo test -p brokkr-cli --test{space}it x::"),
        format!("--test{space}it"),
    );
    for (text, word) in unreadable
        .into_iter()
        .chain([(nbsp.as_str(), joined.as_str())])
    {
        assert_eq!(
            filters_in("f", text, &roots, &read),
            Err(unread_filter("f:1", word, text)),
            "{text}"
        );
    }
}

/// Two packages' roots, and the one module file whose tests an `--exact`
/// name is held to.
fn fixture() -> (Roots, impl Fn(&Path) -> io::Result<String>) {
    let roots = BTreeMap::from([
        (
            "brokkr-cli".to_string(),
            BTreeMap::from([
                ("packaging".to_string(), PathBuf::from("packaging.rs")),
                ("suppressions".to_string(), PathBuf::from("suppressions.rs")),
            ]),
        ),
        ("brokkr-core".to_string(), BTreeMap::new()),
    ]);
    let read = |path: &Path| {
        match path.to_str() {
        Some("suppressions.rs") => Ok("#[test]\n#[ignore = \"x\"]\n/// A test.\nfn a_b() {}\n\nfn helper() {}\nmod inner {\n    #[test]\n    fn nested() {}\n}\n".to_string()),
        _ => Err(io::Error::from(io::ErrorKind::NotFound)),
    }
    };
    (roots, read)
}
