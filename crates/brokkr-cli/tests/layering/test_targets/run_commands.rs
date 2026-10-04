//! The `--test it` commands a tracked file runs, read word by word as the
//! shell reads them (#423; the operator's ruling of 2026-10-04, "do exact
//! command line parsing").
//!
//! A file is first cut into the lines its format makes, each read as its
//! format reads it. A workflow's literal block (`|`) is its lines, and a
//! plain value on its key's line is that value; every other YAML form
//! that names `--test` is refused, since its folding or escapes are not
//! the shell's. Each string of a JSON file is decoded by serde_json and
//! its lines read. A Markdown code span that wraps is joined, as Markdown
//! joins it, and a prose line that does not itself name `--test` gives
//! only its code spans. A file in a format the reader does not know is
//! refused where it names `--test`. Each line is then split into words as
//! the shell splits it: quotes, backslashes and continuations; `|`, `&`,
//! `;`, a parenthesis or a backtick ends one command; a redirect takes its
//! target, which the command never sees, and the words after it are still
//! the command's. A comment, and a quoted word that holds `--test`, are
//! read again as lines of their own, since a person or a wrapper may run
//! them.
//!
//! In each command the words after `cargo` are an invocation. One that
//! passes `--test it` (as `--test it`, `--test=it`, or quoted) is read by
//! closed tables of the flags Cargo and libtest take, and every word the
//! tables do not consume is a filter: before the flag, after it and after
//! `--`. A word, flag or form outside the tables is refused, never
//! skipped; so is a `--test it` outside such an invocation, a line the
//! shell could not split that holds `--test`, and a `--test` the reader
//! cannot read: one a break or the line's end cuts off, or one whose value
//! the shell has yet to make (a parameter, a substitution, a brace or a
//! glob, or a workflow's `${{ }}`) unless its literal text proves it is
//! not `it`. The reader evaluates nothing.
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

/// A word as the command receives it, its quotes and escapes resolved.
#[derive(Debug, PartialEq, Eq)]
struct Word {
    text: String,
    /// Where in `text` the shell begins to make the word: the first `$`,
    /// backtick, brace or glob character outside single quotes, or `${{`,
    /// which a workflow substitutes before any shell reads the line. What
    /// the shell makes there the reader does not know.
    made: Option<usize>,
}

/// What the shell passes a `--test` as.
#[derive(Debug, PartialEq, Eq)]
enum Target<'w> {
    /// `it`, written as a plain literal.
    It,
    /// A target whose literal text proves it is not `it`.
    Other,
    /// A target the reader cannot know is not `it`, by the word that
    /// names it: one the shell has yet to make, or none, a break or the
    /// line's end cutting the command off first.
    Unread(&'w str),
}

/// A shell token.
#[derive(Debug, PartialEq, Eq)]
enum Token {
    Word(Word),
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
    /// Where the shell begins to make the word, as `Word::made`.
    made: Option<usize>,
    /// The redirect whose target the next word is.
    redirect: Option<String>,
}

type Chars<'t> = std::iter::Peekable<std::str::Chars<'t>>;

impl Lexer {
    fn push(&mut self, c: char) {
        self.word.push(c);
        self.started = true;
    }

    /// A character the shell expands rather than passes.
    fn make(&mut self, c: char) {
        self.made.get_or_insert(self.word.len());
        self.push(c);
    }

    fn end_word(&mut self) {
        let made = std::mem::take(&mut self.made);
        if !std::mem::take(&mut self.started) {
            return;
        }
        let text = std::mem::take(&mut self.word);
        let made = [made, text.find("${{")].into_iter().flatten().min();
        self.tokens.push(match self.redirect.take() {
            Some(_) => Token::Target(text),
            None => Token::Word(Word { text, made }),
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
    /// backtick, `"`, itself and the line's end, and an unescaped `$` or
    /// backtick is still the shell's to expand.
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
                Some(c @ ('$' | '`')) => self.make(c),
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
            '$' | '{' | '*' | '?' | '[' => lexer.make(c),
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

/// What the `--test` at `words[index]` passes, or `None` when the word is
/// no `--test`. A target the shell makes is `Other` only when the literal
/// text before what it makes is no prefix of `it`, as `heap_` in
/// `heap_$kind` is not; the reader evaluates nothing.
fn test_target(words: &[Word], index: usize) -> Option<Target<'_>> {
    let word = &words[index];
    let (value, from) = match word.text.strip_prefix("--test=") {
        Some(_) => (word, "--test=".len()),
        None if word.text == "--test" => match words.get(index + 1) {
            Some(value) => (value, 0),
            None => return Some(Target::Unread(&word.text)),
        },
        None => return None,
    };
    Some(match value.made {
        None if value.text[from..] == *"it" => Target::It,
        None => Target::Other,
        Some(made) if "it".starts_with(&value.text[from..made]) => Target::Unread(&value.text),
        Some(_) => Target::Other,
    })
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
fn invocation(args: &[Word]) -> Result<Invocation, &str> {
    let mut words = args.iter().map(|word| word.text.as_str());
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
    let lines = lines_of(file, text)?;
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
            Token::Break => read_command(at, text, &std::mem::take(&mut command), found)?,
        }
    }
    read_command(at, text, &command, found)
}

/// One command's words: an invocation that passes `--test it` by the
/// tables, and every other word loosely. A `cargo` whose `--test` the
/// reader cannot read is refused by that word.
fn read_command(
    at: &str,
    text: &str,
    words: &[Word],
    found: &mut Vec<(String, Invocation)>,
) -> Result<(), Refusal> {
    let cargo =
        (words.iter()).position(|word| word.text == "cargo" || word.text.ends_with("/cargo"));
    let Some(cargo) = cargo else {
        return loose(at, text, words, words.len(), found);
    };
    let args = &words[cargo + 1..];
    let mut passes_it = false;
    for index in 0..args.len() {
        match test_target(args, index) {
            Some(Target::Unread(word)) => return Err(unread_filter(at, word, text)),
            Some(Target::It) => passes_it = true,
            Some(Target::Other) | None => {}
        }
    }
    if !passes_it {
        return loose(at, text, words, words.len(), found);
    }
    let invocation = invocation(args).map_err(|word| unread_filter(at, word, text))?;
    loose(at, text, words, cargo, found)?;
    found.push((text.trim().to_string(), invocation));
    Ok(())
}

/// The first `upto` of a command's words, outside any `--test it`
/// invocation: a `--test it` here is refused, as is a `--test` the reader
/// cannot read and a word that runs `--test` into other text; a quoted
/// word that holds `--test` among other words is read as a line. Only a
/// space or a tab splits a word, and a word a quote made is shorter than
/// its text, so the reading ends.
fn loose(
    at: &str,
    text: &str,
    words: &[Word],
    upto: usize,
    found: &mut Vec<(String, Invocation)>,
) -> Result<(), Refusal> {
    for (index, word) in words.iter().enumerate().take(upto) {
        let word_text = word.text.as_str();
        match test_target(words, index) {
            Some(Target::It) => return Err(unread_filter(at, word_text, text)),
            Some(Target::Unread(unread)) => return Err(unread_filter(at, unread, text)),
            Some(Target::Other) => {}
            None if !mentions_test(word_text) => {}
            None if word_text.contains([' ', '\t']) && word_text.len() < text.len() => {
                read_line(at, word_text, found)?;
            }
            None => return Err(unread_filter(at, word_text, text)),
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

/// A file's lines as its format makes them: a script's, a text's or an
/// extensionless file's as they are, a workflow's, a guide's and a JSON
/// file's as their formats read them. A file in any other format is read
/// as a form the reader does not know.
fn lines_of(file: &str, text: &str) -> Result<Vec<Line>, Refusal> {
    let numbered = || (text.lines().enumerate()).map(|(index, line)| (index + 1, line.to_string()));
    match Path::new(file)
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("yml" | "yaml") => yaml_lines(file, text),
        Some("md") => Ok(markdown_lines(text)),
        Some("json") => json_lines(file, text),
        Some("sh" | "txt") | None => Ok(numbered().collect()),
        Some(_) => unread(file, 1, &text.lines().collect::<Vec<_>>()).map(|()| Vec::new()),
    }
}

/// Lines, numbered from `first`, in a form the reader does not read
/// exactly: they give no command, and the first that names `--test` is
/// refused.
fn unread(file: &str, first: usize, lines: &[&str]) -> Result<(), Refusal> {
    match lines.iter().position(|line| mentions_test(line)) {
        Some(offset) => Err(unread_filter(
            &format!("{file}:{}", first + offset),
            "--test",
            lines[offset],
        )),
        None => Ok(()),
    }
}

/// A JSON file's strings, each decoded by serde_json and given its lines,
/// numbered by the line its literal starts on. An empty line closes each,
/// so a backslash that ends one string continues into no other. A file
/// serde_json cannot parse is read as a form the reader does not know.
fn json_lines(file: &str, text: &str) -> Result<Vec<Line>, Refusal> {
    if serde_json::from_str::<serde::de::IgnoredAny>(text).is_err() {
        return unread(file, 1, &text.lines().collect::<Vec<_>>()).map(|()| Vec::new());
    }
    let (mut read, mut number, mut chars) = (Vec::new(), 1, text.char_indices());
    while let Some((start, c)) = chars.next() {
        match c {
            '\n' => number += 1,
            '"' => {
                let mut escaped = false;
                let (end, _) = (chars.by_ref())
                    .find(|&(_, c)| {
                        let closes = c == '"' && !escaped;
                        escaped = c == '\\' && !escaped;
                        closes
                    })
                    .expect("a string of parsed JSON closes");
                let string: String = serde_json::from_str(&text[start..=end])
                    .expect("a string of parsed JSON decodes");
                read.extend(string.lines().map(|line| (number, line.to_string())));
                read.push((number, String::new()));
            }
            _ => {}
        }
    }
    Ok(read)
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Whether a value opens a literal block, `|`, with its chomping and
/// indentation indicators and a comment after it.
fn opens_literal(value: &str) -> bool {
    let header = value
        .split_once(" #")
        .map_or(value, |(header, _)| header)
        .trim_end();
    (header.strip_prefix('|'))
        .is_some_and(|rest| rest.chars().all(|c| matches!(c, '-' | '+' | '1'..='9')))
}

/// Whether a value is a plain scalar: one that opens with no indicator of
/// another form, a quote, a block, a flow collection, an anchor, alias or
/// tag, or a reserved character.
fn is_plain(value: &str) -> bool {
    let indicator = [
        '|', '>', '"', '\'', '[', ']', '{', '}', '&', '*', '!', '%', '@', '`', ',',
    ];
    !(value.starts_with(indicator)
        || ["- ", "? ", ": "]
            .iter()
            .any(|open| value.starts_with(open)))
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

/// A workflow's lines as YAML holds its scalars, read in the two forms the
/// workflows use and read exactly: a literal block (`|`) under a plain key
/// line by line, and a plain value on its key's line alone, as its value.
/// A comment is read as a line. Every other form is one the reader does
/// not read exactly: a folded block, a quoted value, whose escapes YAML
/// and the shell read apart, a plain value that wraps or starts under its
/// key, a flow collection, and any line under a key the reader does not
/// know, a quoted one among them.
fn yaml_lines(file: &str, text: &str) -> Result<Vec<Line>, Refusal> {
    let lines: Vec<&str> = text.lines().collect();
    let (mut read, mut index) = (Vec::new(), 0);
    while let Some(line) = lines.get(index) {
        let Some((column, value)) = scalar_of(line) else {
            if line.trim_start().starts_with('#') {
                read.push((index + 1, line.to_string()));
            } else {
                unread(file, index + 1, &[line])?;
            }
            index += 1;
            continue;
        };
        let body = (lines[index + 1..].iter())
            .take_while(|body| body.trim().is_empty() || indent(body) > column)
            .count();
        let (end, alone) = (
            index + 1 + body,
            lines[index + 1..index + 1 + body]
                .iter()
                .all(|body| body.trim().is_empty()),
        );
        if opens_literal(value) {
            read.extend((index..end).map(|at| (at + 1, lines[at].to_string())));
        } else if is_plain(value) && alone {
            read.push((index + 1, value.to_string()));
        } else {
            unread(file, index + 1, &lines[index..end])?;
        }
        index = end;
    }
    Ok(read)
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
        ("f", "cargo test -p brokkr-cli --test \"heap_$kind\" x\ncargo test -p brokkr-cli --test=x$y{z} x\n", vec![]),
        ("f.yml", "    run: |\n      cargo test -p brokkr-cli --test it packaging::\n      echo gone::\n", vec![cli("f.yml:2", "packaging")]),
        ("f.yml", "# cargo test -p brokkr-cli --test it packaging::\n  - run: cargo test -p brokkr-cli --test it suppressions:: # note\n  - run: echo gone::\n", vec![cli("f.yml:1", "packaging"), cli("f.yml:2", "suppressions")]),
        ("f.json", "{\"a\": \"cargo test -p brokkr-cli --test\\tit packaging:: \\\\\",\n \"b\": [\"no_such_file::\"]}\n", vec![cli("f.json:1", "packaging")]),
        ("f.md", "Run `cargo test -p brokkr-cli --test\nit packaging::` once; it's quick.\n```sh\ncargo test -p brokkr-cli --test it suppressions::\n```\n", vec![cli("f.md:1", "packaging"), cli("f.md:4", "suppressions")]),
    ] {
        assert_eq!(filters_in(file, text, &roots, &read), Ok(held), "{text}");
    }
}

/// Every spelling of a stale filter the shell would pass is held and
/// refused: each form of `--test it`, a filter after `--skip`'s argument,
/// a second filter, one before the flag, one after a redirect, a second
/// package, a wrapped Markdown span, and a JSON string's escapes, decoded.
#[test]
fn the_filter_reader_refuses_a_stale_filter_in_every_form() {
    let (roots, read) = fixture();
    let refusal = |at: &str, package: &str, module: &str| Refusal::UnknownModule {
        at: at.into(),
        package: package.into(),
        module: module.into(),
    };
    let stale = (STALE.into_iter()).map(|(file, text)| {
        (
            file,
            text,
            refusal(&format!("{file}:1"), "brokkr-cli", "no_such_file"),
        )
    });
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

/// A stale filter in each spelling, by file; each names `no_such_file` on
/// the file's first line.
const STALE: [(&str, &str); 13] = [
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
        "f.md",
        "Run `cargo test -p brokkr-cli --test it\nno_such_file::` once.",
    ),
    (
        "f.json",
        "{\"run\": \"cargo test --locked -p brokkr-cli --test it\\tno_such_file::\"}",
    ),
    (
        "f.json",
        concat!(
            "{\"run\": \"cargo test --locked -p brokkr-cli --test it\\",
            "u0020no_such_file::\"}"
        ),
    ),
];

/// Every word, flag or form the reader does not know is refused by its
/// word, never skipped: a filter that is not a module path, a flag outside
/// the tables or in a form they do not take, a subcommand other than
/// `test`, a `--test it` outside `cargo test`, a `--test` a break or the
/// line's end cuts off or run into other text, a line the shell cannot
/// split, a `--test` the shell has yet to make that could be `it`, and a
/// form a file's format does not let the reader read exactly.
#[test]
fn the_filter_reader_refuses_a_word_or_form_it_cannot_read() {
    let (roots, read) = fixture();
    let space = char::from(0xa0);
    let (nbsp, joined) = (
        format!("cargo test -p brokkr-cli --test{space}it x::"),
        format!("--test{space}it"),
    );
    let in_f = (UNREAD_WORDS.into_iter())
        .chain([(nbsp.as_str(), joined.as_str())])
        .map(|(text, word)| ("f", text, 1, word, text));
    let one_line = (UNREAD_LINES.into_iter()).map(|(file, text)| (file, text, 1, "--test", text));
    for (file, text, line, word, command) in in_f.chain(one_line).chain(UNREAD_FORMS) {
        assert_eq!(
            filters_in(file, text, &roots, &read),
            Err(unread_filter(&format!("{file}:{line}"), word, command)),
            "{text}"
        );
    }
}

/// A command, on a file's first line, the reader refuses by a word.
const UNREAD_WORDS: [(&str, &str); 33] = [
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
    (
        "test_target=it; cargo test --locked -p brokkr-cli --test \"$test_target\" no_such_file::",
        "$test_target",
    ),
    (
        "cargo test --locked -p brokkr-cli --test ${TEST_TARGET:-it} no_such_file::",
        "${TEST_TARGET:-it}",
    ),
    (
        "for t in it; do cargo test --locked -p brokkr-cli --test \"$t\" no_such_file::; done",
        "$t",
    ),
    (
        "cargo test --locked -p brokkr-cli --test {it,no_such_file::}",
        "{it,no_such_file::}",
    ),
    ("cargo test --locked -p brokkr-cli --test i* x::", "i*"),
    (
        "cargo test --locked -p brokkr-cli --test=[i]t x::",
        "--test=[i]t",
    ),
    ("cargo test --locked -p brokkr-cli --test it$x x::", "it$x"),
    (
        "cargo test --locked -p brokkr-cli --test $(echo it) no_such_file::",
        "$",
    ),
    ("cargo test -p brokkr-cli --test `echo it` x::", "--test"),
    ("cargo test -p brokkr-cli --test; echo it", "--test"),
    ("report --test \"$x\" x::", "$x"),
];

/// A one-line form, by file, the reader does not read exactly, refused at
/// `--test` with the line as its command.
const UNREAD_LINES: [(&str, &str); 7] = [
    (
        "f.yml",
        "run: \"cargo test --locked -p brokkr-cli --test it\\tno_such_file::\"",
    ),
    (
        "f.yml",
        concat!(
            "run: \"cargo test --locked -p brokkr-cli --test it\\",
            "u0020no_such_file::\""
        ),
    ),
    (
        "f.yml",
        "run: \"cargo test --locked -p brokkr-cli --test\\x20it no_such_file::\"",
    ),
    ("f.yml", "run: 'cargo test -p brokkr-cli --test it x::'"),
    ("f.yml", "- {run: cargo test -p brokkr-cli --test it x::}"),
    (
        "f.json",
        "{\"run\": \"cargo test -p brokkr-cli --test it x::\",}",
    ),
    ("f.json5", "{run: 'cargo test -p brokkr-cli --test it x::'}"),
];

/// A form, by file, the reader refuses at a line and a word, with the
/// command it names.
const UNREAD_FORMS: [(&str, &str, usize, &str, &str); 7] = [
    (
        "f.yml",
        "run: cargo test --locked -p brokkr-cli --test ${{ matrix.target }} no_such_file::",
        1,
        "${{",
        "cargo test --locked -p brokkr-cli --test ${{ matrix.target }} no_such_file::",
    ),
    (
        "f.yml",
        "run: |\n  cargo test -p brokkr-cli --test '${{ matrix.target }}' x::",
        2,
        "${{ matrix.target }}",
        "cargo test -p brokkr-cli --test '${{ matrix.target }}' x::",
    ),
    (
        "f.yml",
        "run:\n  cargo test --locked -p brokkr-cli --test it\n  no_such_file::b",
        2,
        "--test",
        "cargo test --locked -p brokkr-cli --test it",
    ),
    (
        "f.yml",
        "\"run\": >-\n  cargo test --locked -p brokkr-cli --test it\n  no_such_file::b",
        2,
        "--test",
        "cargo test --locked -p brokkr-cli --test it",
    ),
    (
        "f.yml",
        "run: >-\n  cargo test -p brokkr-cli --test it\n  no_such_file::b",
        2,
        "--test",
        "cargo test -p brokkr-cli --test it",
    ),
    (
        "f.yml",
        "  - run: cargo test -p brokkr-cli\n      --test it packaging::",
        2,
        "--test",
        "--test it packaging::",
    ),
    (
        "f.json",
        "[\"cargo\", \"test\", \"-p\", \"brokkr-cli\", \"--test\", \"it\", \"x::\"]",
        1,
        "--test",
        "--test",
    ),
];

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
