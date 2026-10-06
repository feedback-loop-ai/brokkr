//! The `--test it` commands a tracked file runs, read exactly in a closed
//! grammar (#423; the operator's ruling of 2026-10-04, "do exact command
//! line parsing", read as the fifth hold asked: close the grammar).
//!
//! A file is first cut into the lines its format makes, each read as its
//! format reads it. A workflow's literal block (`|`) is its lines, and a
//! plain value on its key's line is that value, its comment a line of its
//! own; every other YAML form that names `--test` is refused, since its
//! folding or escapes are not the shell's. Each string of a JSON file is
//! decoded by serde_json and its lines read. A file in a format the reader
//! does not know is refused where it names `--test`. In a Markdown file a
//! backtick run opens a code span the next run of its length closes, as
//! Markdown pairs them, and a span that wraps is joined, as Markdown joins
//! a paragraph. A prose line is cut at its spans only where each stands as
//! Markdown's delimiters do, at the line's start or after a space or `(`,
//! and at its end or before a space or one of `.,:;)`, and where no run of
//! prose between them is a `cargo` command that names `--test`. A cut line
//! gives its spans, and its runs of prose too when they name `--test`,
//! each a line. Every other line is read whole, so a backtick the shell
//! may run reaches the shell's reader inside its command.
//!
//! A line that is wholly a comment is read as its text. A line names
//! `--test` when its text does, or its text with every character outside
//! the alphabet taken out, so that a quote, an escape or an expansion
//! inside the flag still names it. A line whose every `--test` plainly
//! passes another target, `--test` or `--test=` then a word of the
//! alphabet that is not `it`, passes no `--test it` and is not this gate's
//! business; nor is a line that names no `--test`. Every other line is read
//! in the closed grammar alone: words of the alphabet (letters, digits and
//! `_-./:=,@+`), each bare or a plain word of it in `'…'` or `"…"`, and one
//! space between words. The first character outside that grammar refuses
//! the line, by that character: an expansion, a separator, a redirect, a
//! glob, a continuation, a comment, a tab, a second space, a quote that
//! does not close or holds what no plain word does, and every other. No
//! shell semantics is emulated, and none is needed.
//!
//! The reader of a command's words is the closed grammar's, and this round
//! widens what it refuses without widening what it admits. A read
//! invocation's filters are held to the tests the package's own `it`
//! binary lists (`--list --format terse`): a filter's module must have a
//! listed test of its own, and an `--exact` filter must be a listed name
//! (#543). The binary's list is the fact, and it is the list of the
//! command's own build configuration: default features in the dev profile.
//! The flag tables are the one home of whether a flag can move the build,
//! and a command that names any build-moving flag — a feature set, a
//! profile, `--config`, `-Z`, a `+toolchain` — or an assignment before
//! `cargo` whose name is `CARGO_*` or `RUST*`, is refused, named, because
//! the gate holds no such list. A filter under more than one `-p` is
//! refused too: Cargo unifies the selected packages' features, so no
//! single listed build holds it. Every one of those refusals is the same
//! axis: the list must be the command's own build. The source is never
//! read for tests, so a name a string literal, a comment or a nested item
//! shows, and one a `cfg`, a `cfg_attr` or the module's own `#![cfg]`
//! gates away from the compiling host, is in no list and refuses. A
//! package whose binary cannot be built or listed is refused, named,
//! never skipped. In a workflow or a script the command must run through
//! `scripts/run-it-tests.sh`, the checked entry point that fails a
//! filtered run executing no test; a guide's command is read from its text
//! alone, because an operator copies it and no job here runs it.
//!
//! The threat model is the operator's of 2026-09-26: realistic accidental
//! misuse is caught and what cannot be read is refused. A `--test it` that
//! a program makes from text that never spells it, as `xargs` does from
//! its input, is a low residual.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{is_identifier, Refusal};
use crate::workspace_root::GUARD;

/// The tests each package's `it` binary carries, by package, as the
/// binary itself lists them.
pub(super) type Lists = BTreeMap<String, BTreeSet<String>>;

/// One line as a command reads it: the number of the file line it starts
/// on, and its text.
type Line = (usize, String);

/// A Markdown code span: the byte its opening run starts at, the byte its
/// closing run starts at, and the runs' length.
type Span = (usize, usize, usize);

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

/// A closed table of flags: each row is the name a flag is written with,
/// what it takes, and whether it can move the build the gate lists.
type Table = &'static [(&'static str, Flag, bool)];

/// Cargo's own flags, which it takes before the subcommand as well. A
/// `--config` sets another configuration and `-Z` another unstable
/// feature set, so each can move the build.
const CARGO: Table = &[
    ("--locked", Flag::Switch, false),
    ("--offline", Flag::Switch, false),
    ("--frozen", Flag::Switch, false),
    ("-q", Flag::Switch, false),
    ("--quiet", Flag::Switch, false),
    ("-v", Flag::Switch, false),
    ("--verbose", Flag::Switch, false),
    ("--color", Flag::Valued, false),
    ("--config", Flag::Valued, true),
    ("-Z", Flag::Valued, true),
];

/// `cargo test`'s flags, before `--`. A feature set, another profile or
/// `--release` builds a binary other than the default-feature dev one the
/// gate lists, so each can move the build.
const CARGO_TEST: Table = &[
    ("-p", Flag::Package, false),
    ("--package", Flag::Package, false),
    ("--test", Flag::Valued, false),
    ("-F", Flag::Valued, true),
    ("--features", Flag::Valued, true),
    ("--all-features", Flag::Switch, true),
    ("--no-default-features", Flag::Switch, true),
    ("--release", Flag::Switch, true),
    ("--no-fail-fast", Flag::Switch, false),
    ("-j", Flag::Valued, false),
    ("--jobs", Flag::Valued, false),
    ("--target-dir", Flag::Valued, false),
];

/// libtest's flags, after `--`. None moves the build.
const LIBTEST: Table = &[
    ("--exact", Flag::Exact, false),
    ("--skip", Flag::Valued, false),
    ("--ignored", Flag::Switch, false),
    ("--include-ignored", Flag::Switch, false),
    ("--nocapture", Flag::Switch, false),
    ("--no-capture", Flag::Switch, false),
    ("--show-output", Flag::Switch, false),
    ("--test-threads", Flag::Valued, false),
    ("-q", Flag::Switch, false),
    ("--quiet", Flag::Switch, false),
    ("--color", Flag::Valued, false),
    ("--format", Flag::Valued, false),
];

/// A `cargo test` invocation as the tables read it.
#[derive(Debug, Default, PartialEq, Eq)]
struct Invocation {
    packages: Vec<String>,
    filters: Vec<String>,
    exact: bool,
    /// The build configuration the command asks for, by word, when it is
    /// not the default the gate lists: a feature set or a profile, each
    /// as the command writes its flag.
    configuration: Vec<String>,
}

/// What a `--test` passes.
#[derive(Debug, PartialEq, Eq)]
enum Target {
    /// `it`.
    It,
    /// Another target.
    Other,
    /// None: the command ends first.
    Missing,
}

/// Whether `c` is of the closed alphabet: a letter, a digit, or one of
/// `_-./:=,@+`.
fn is_safe(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ':' | '=' | ',' | '@' | '+')
}

/// `text` with every character outside the alphabet, the space aside,
/// taken out.
fn plain(text: &str) -> String {
    text.chars().filter(|&c| c == ' ' || is_safe(c)).collect()
}

fn is_cargo(word: &str) -> bool {
    word == "cargo" || word.ends_with("/cargo")
}

/// What follows each `--test` that `text` names as a flag could, not run
/// into a longer flag such as `--tests` or `--test-threads`: in its text,
/// and in its text made plain.
fn tests_named(text: &str) -> Vec<String> {
    let plain = plain(text);
    [text, plain.as_str()]
        .into_iter()
        .flat_map(|text| {
            (text.match_indices("--test")).map(move |(at, flag)| &text[at + flag.len()..])
        })
        .filter(|rest| {
            !rest.starts_with(|c: char| c == '_' || c == '-' || c.is_ascii_alphanumeric())
        })
        .map(str::to_string)
        .collect()
}

fn names_test(text: &str) -> bool {
    !tests_named(text).is_empty()
}

/// Whether `text` may pass `--test it`: it names a `--test` that does not
/// plainly pass another target, as `--test` or `--test=` and then a word
/// of the alphabet, not `it`, that a space or the text's end ends.
fn may_pass_it(text: &str) -> bool {
    tests_named(text).iter().any(|rest| {
        let target = (rest.strip_prefix([' ', '='])).and_then(|value| value.split(' ').next());
        !target.is_some_and(|target| {
            !target.is_empty() && target != "it" && target.chars().all(is_safe)
        })
    })
}

/// A line's words in the closed grammar: runs of the alphabet, each bare
/// or a plain word in quotes, the words split by one space. The first
/// character outside it is the error.
fn words_of(line: &str) -> Result<Vec<String>, char> {
    let (mut words, mut word) = (Vec::new(), String::new());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' if chars.peek().is_some_and(|&next| next != ' ') => {
                words.push(std::mem::take(&mut word));
            }
            '\'' | '"' => word.push_str(&quoted(c, &mut chars)?),
            c if is_safe(c) => word.push(c),
            c => return Err(c),
        }
    }
    words.push(word);
    Ok(words)
}

/// The plain word of the alphabet in the quotes `quote` opened, up to the
/// one that closes them. A quote that does not close, or closes on
/// nothing, is refused by that quote.
fn quoted(quote: char, chars: &mut impl Iterator<Item = char>) -> Result<String, char> {
    let mut word = String::new();
    for c in chars {
        match c {
            c if c == quote && !word.is_empty() => return Ok(word),
            c if is_safe(c) => word.push(c),
            c => return Err(c),
        }
    }
    Err(quote)
}

/// What the `--test` at `words[index]` passes, or `None` when the word is
/// no `--test`.
fn test_target(words: &[String], index: usize) -> Option<Target> {
    let word = &words[index];
    let value = match word.strip_prefix("--test=") {
        Some(value) => value,
        None if word == "--test" => match words.get(index + 1) {
            Some(value) => value,
            None => return Some(Target::Missing),
        },
        None => return None,
    };
    Some(if value == "it" {
        Target::It
    } else {
        Target::Other
    })
}

fn unread_filter(at: &str, word: &str, command: &str) -> Refusal {
    Refusal::UnreadFilter {
        at: at.to_string(),
        word: word.to_string(),
        command: command.trim().to_string(),
    }
}

/// The name of a flag word: the `--name` of `--name=value`, else the word.
/// Only a long flag is read with `=`; a short flag stands alone.
fn flag_name(word: &str) -> &str {
    match word.split_once('=') {
        Some((name, _)) if name.starts_with("--") => name,
        _ => word,
    }
}

/// The flag `word` names in `tables`, with its argument and whether its
/// row says it can move the build: after `=`, or the next word when it
/// takes one. A form outside the tables is refused by its word; a short
/// flag is read only alone, so `-pname` is refused.
fn flag<'w>(
    tables: &[Table],
    word: &'w str,
    rest: &mut impl Iterator<Item = &'w str>,
) -> Result<(Flag, bool, Option<&'w str>), &'w str> {
    let name = flag_name(word);
    let inline = (name != word).then(|| &word[name.len() + 1..]);
    let mut known = tables.iter().flat_map(|table| table.iter());
    let (_, flag, moves_build) = known.find(|(known, _, _)| *known == name).ok_or(word)?;
    match (flag.takes_argument(), inline) {
        (true, None) => Ok((*flag, *moves_build, Some(rest.next().ok_or(word)?))),
        (true, Some(_)) | (false, None) => Ok((*flag, *moves_build, inline)),
        (false, Some(_)) => Err(word),
    }
}

/// `args`, the words after `cargo`, read as `[+toolchain] [flags] test
/// [args]`. A flag whose row can move the build, and a `+toolchain`, are
/// recorded on the invocation so `hold` can refuse them, named. The word
/// the tables cannot read is the error.
fn invocation(args: &[String]) -> Result<Invocation, &str> {
    let mut words = args.iter().map(String::as_str);
    let (mut read, mut first) = (Invocation::default(), true);
    loop {
        match words.next().ok_or("cargo")? {
            "test" | "t" => break,
            toolchain if first && toolchain.starts_with('+') => {
                read.configuration.push(toolchain.to_string());
            }
            word => {
                let (_, moves_build, _) = flag(&[CARGO], word, &mut words)?;
                if moves_build {
                    read.configuration.push(word.to_string());
                }
            }
        }
        first = false;
    }
    let mut libtest = false;
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
                (Flag::Package, _, package) => read.packages.extend(package.map(str::to_string)),
                (Flag::Exact, _, _) => read.exact = true,
                (_, true, _) => read.configuration.push(word.to_string()),
                (_, false, _) => {}
            }
        }
    }
    Ok(read)
}

/// Every `--test it` filter in `text`, the file at `file`, each held to
/// the tests `lists` records for the package its command names: the
/// binary's own list is the fact. The first it cannot hold is refused.
pub(super) fn filters_in(file: &str, text: &str, lists: &Lists) -> Result<Vec<Filter>, Refusal> {
    let guard = guarded_root(file);
    let mut filters = Vec::new();
    for (number, line) in lines_of(file, text)? {
        let at = format!("{file}:{number}");
        if let Some((command, invocation)) = read_line(&at, &line, guard)? {
            filters.extend(hold(&at, &command, &invocation, lists)?);
        }
    }
    Ok(filters)
}

/// Whether a file's `--test it` commands must run through [`GUARD`]: the
/// workflows and scripts this repository runs, not a guide or a recipe an
/// operator copies by hand.
fn guarded_root(file: &str) -> bool {
    file.starts_with(".github/") || file.starts_with("scripts/")
}

/// The `--test it` invocation one line runs, with the command it was read
/// from: none when the line may pass no `--test it`, and otherwise the
/// line read in the closed grammar or refused by its first character
/// outside it. A line that is wholly a comment is read as its text.
fn read_line(at: &str, line: &str, guard: bool) -> Result<Option<(String, Invocation)>, Refusal> {
    let line = line.trim_matches(' ');
    let line = (line.strip_prefix('#')).map_or(line, |comment| comment.trim_start_matches(' '));
    if !may_pass_it(line) {
        return Ok(None);
    }
    let words = words_of(line).map_err(|outside| unread_filter(at, &outside.to_string(), line))?;
    let invocation = read_command(at, line, &words, guard)?;
    Ok(invocation.map(|invocation| (line.to_string(), invocation)))
}

/// One command's words: an invocation that passes `--test it` by the
/// tables, and every other word loosely. A `cargo` whose `--test` names
/// no target is refused by that word; in a workflow or script the command
/// must run through the checked entry point; and an assignment before
/// `cargo` that can move the build is recorded so `hold` refuses it.
fn read_command(
    at: &str,
    line: &str,
    words: &[String],
    guard: bool,
) -> Result<Option<Invocation>, Refusal> {
    let Some(cargo) = words.iter().position(|word| is_cargo(word)) else {
        return loose(at, line, words).map(|()| None);
    };
    let args = &words[cargo + 1..];
    let mut passes_it = false;
    for index in 0..args.len() {
        match test_target(args, index) {
            Some(Target::Missing) => return Err(unread_filter(at, &args[index], line)),
            Some(Target::It) => passes_it = true,
            Some(Target::Other) | None => {}
        }
    }
    if !passes_it {
        return loose(at, line, words).map(|()| None);
    }
    if guard && (cargo == 0 || words[cargo - 1] != GUARD) {
        return Err(Refusal::Unguarded {
            at: at.to_string(),
            command: line.trim().to_string(),
        });
    }
    let mut invocation = invocation(args).map_err(|word| unread_filter(at, word, line))?;
    for word in &words[..cargo] {
        if moves_build_assignment(word) {
            invocation.configuration.push(word.clone());
        }
    }
    loose(at, line, &words[..cargo])?;
    Ok(Some(invocation))
}

/// Whether `word` is an environment assignment before `cargo` that can
/// move the build: a `NAME=value` whose name starts `CARGO_` (Cargo's own
/// configuration) or `RUST` (`RUSTFLAGS`, `RUSTC`, `RUSTUP_TOOLCHAIN`,
/// and the rest). One of those set before `cargo` can build another
/// binary than the gate lists, and no line shows it in the flags.
fn moves_build_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
        && (name.starts_with("CARGO_") || name.starts_with("RUST"))
}

/// Words outside any `--test it` invocation: a `--test it` here is
/// refused, as is a `--test` with no target and a word that runs `--test`
/// into other text.
fn loose(at: &str, line: &str, words: &[String]) -> Result<(), Refusal> {
    for (index, word) in words.iter().enumerate() {
        match test_target(words, index) {
            Some(Target::It | Target::Missing) => return Err(unread_filter(at, word, line)),
            Some(Target::Other) => {}
            None if !names_test(word) => {}
            None => return Err(unread_filter(at, word, line)),
        }
    }
    Ok(())
}

/// An invocation's filters, held to the tests `lists` records for each
/// package it names: the binary's own list is the fact, so a filter's
/// module must have a listed test of its own and, under `--exact`, the
/// filter must be a listed name. A name a literal's text or a nested item
/// shows is no test of the binary, and neither is one a `cfg`, a
/// `cfg_attr` or the module's own `#![cfg]` gates away from the build the
/// binary was made in; the list already reflects all of it. A command
/// whose feature set or profile is not the one the list was built of is
/// refused, named, never held to a list of another build. A filter under
/// more than one `-p` is refused: Cargo unifies the selected packages'
/// features, so no single listed build holds it. A package no list was
/// read of is refused, never skipped.
fn hold(
    at: &str,
    command: &str,
    invocation: &Invocation,
    lists: &Lists,
) -> Result<Vec<Filter>, Refusal> {
    if invocation.packages.is_empty() && !invocation.filters.is_empty() {
        return Err(Refusal::NoPackage {
            at: at.to_string(),
            command: command.to_string(),
        });
    }
    if invocation.packages.len() > 1 && !invocation.filters.is_empty() {
        return Err(Refusal::ManyPackages {
            at: at.to_string(),
            packages: invocation.packages.clone(),
            command: command.to_string(),
        });
    }
    let mut held = Vec::new();
    for filter in &invocation.filters {
        let (module, _) =
            path_of(filter, invocation.exact).ok_or_else(|| unread_filter(at, filter, command))?;
        for package in &invocation.packages {
            if !invocation.configuration.is_empty() {
                return Err(Refusal::UnreadList {
                    at: at.to_string(),
                    package: package.clone(),
                    why: format!(
                        "the gate lists it with default features in the dev profile, not `{}`",
                        invocation.configuration.join(" ")
                    ),
                });
            }
            let carried = (lists.get(package)).ok_or_else(|| Refusal::UnreadList {
                at: at.to_string(),
                package: package.clone(),
                why: "the workspace has no tests/it binary for it".into(),
            })?;
            if invocation.exact {
                if !carried.contains(filter) {
                    return Err(Refusal::UnknownTest {
                        at: at.to_string(),
                        package: package.clone(),
                        name: filter.clone(),
                    });
                }
            } else if !carried.iter().any(|test| under_module(test, module)) {
                return Err(Refusal::TestlessModule {
                    at: at.to_string(),
                    package: package.clone(),
                    module: module.to_string(),
                });
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

/// Whether a listed test sits under `module`: its path begins with the
/// module's name and the `::` that follows it, so `suppressions::a_b` is
/// under `suppressions` and `suppressions_other::a_b` is not.
fn under_module(test: &str, module: &str) -> bool {
    test.strip_prefix(module)
        .is_some_and(|rest| rest.starts_with("::"))
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
    match lines.iter().position(|line| names_test(line)) {
        Some(offset) => Err(unread_filter(
            &format!("{file}:{}", first + offset),
            "--test",
            lines[offset],
        )),
        None => Ok(()),
    }
}

/// A JSON file's strings, each decoded by serde_json and given its lines,
/// numbered by the line its literal starts on. A file serde_json cannot
/// parse is read as a form the reader does not know.
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
/// line by line, and a plain value on its key's line alone, as its value,
/// which a ` #` ends, the comment after it read as a line of its own. A
/// comment line is read as a line. Every other form is one the reader does
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
            let (value, comment) = value.split_once(" #").unwrap_or((value, ""));
            read.extend([value, comment].map(|part| (index + 1, part.to_string())));
        } else {
            unread(file, index + 1, &lines[index..end])?;
        }
        index = end;
    }
    Ok(read)
}

/// A line's backtick runs, each by the byte it starts at and its length.
fn backtick_runs(text: &str) -> Vec<(usize, usize)> {
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (at, _) in text.match_indices('`') {
        match runs.last_mut() {
            Some((start, length)) if *start + *length == at => *length += 1,
            _ => runs.push((at, 1)),
        }
    }
    runs
}

/// The code spans Markdown makes of a line's backtick runs: a run opens a
/// span that the next run of its length closes, and the runs between are
/// the span's text. With them, whether a run is left with no partner,
/// which Markdown reads as text and which may yet pair on the next line.
fn code_spans(text: &str) -> (Vec<Span>, bool) {
    let runs = backtick_runs(text);
    let (mut spans, mut unpaired, mut next) = (Vec::new(), false, 0);
    while let Some(&(start, length)) = runs.get(next) {
        let partner = (runs[next + 1..].iter()).position(|&(_, other)| other == length);
        match partner {
            Some(offset) => {
                spans.push((start, runs[next + 1 + offset].0, length));
                next += offset + 2;
            }
            None => {
                unpaired = true;
                next += 1;
            }
        }
    }
    (spans, unpaired)
}

/// A Markdown file's lines: a line that leaves a backtick run with no
/// partner joined to the next with a space, the next's indentation
/// dropped, as Markdown joins a paragraph, until the run pairs or a blank
/// line ends the paragraph. A fence and the
/// lines between fences stand alone.
fn markdown_lines(text: &str) -> Vec<Line> {
    let (mut joined, mut open, mut fenced) = (Vec::new(), None::<Line>, false);
    for (index, line) in text.lines().enumerate() {
        let current = match open.take() {
            Some((number, span)) if !line.trim().is_empty() => {
                (number, format!("{span} {}", line.trim_start()))
            }
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
        if code_spans(&current.1).1 {
            open = Some(current);
        } else {
            joined.push((current, false));
        }
    }
    joined.extend(open.map(|open| (open, false)));
    (joined.into_iter())
        .flat_map(|(line, code)| code_of(line, code))
        .collect()
}

/// The commands a Markdown line holds: a fenced line, or a prose line the
/// reader does not cut at its code spans, whole; otherwise the text of
/// each span, and each run of its prose too when the prose itself names
/// `--test`. The runs that bound a span are Markdown's, cut here, and
/// never reach the shell's reader.
fn code_of((number, text): Line, fenced: bool) -> Vec<Line> {
    let (spans, _) = code_spans(&text);
    let mut prose = Vec::new();
    let mut from = 0;
    for &(start, close, length) in &spans {
        prose.push(&text[from..start]);
        from = close + length;
    }
    prose.push(&text[from..]);
    if fenced || spans.is_empty() || !cut_at_spans(&text, &spans, &prose) {
        return vec![(number, text)];
    }
    let prose_names_test = prose.iter().any(|part| names_test(part));
    let mut pieces = Vec::new();
    for (index, part) in prose.iter().enumerate() {
        if prose_names_test {
            pieces.push((number, (*part).to_string()));
        }
        if let Some(&(start, close, length)) = spans.get(index) {
            pieces.push((number, text[start + length..close].to_string()));
        }
    }
    pieces
}

/// Whether the reader cuts a prose line at its code spans: each opens at
/// the line's start or after a space or `(`, and closes at its end or
/// before a space or one of `.,:;)`, as Markdown's delimiters stand, and
/// no run of prose between them is a `cargo` command that names `--test`.
/// Anywhere else a backtick may be the shell's, and the line is read
/// whole.
fn cut_at_spans(text: &str, spans: &[Span], prose: &[&str]) -> bool {
    let delimits = spans.iter().all(|&(start, close, length)| {
        let before = text[..start].chars().next_back();
        let after = text[close + length..].chars().next();
        before.is_none_or(|c| matches!(c, ' ' | '('))
            && after.is_none_or(|c| matches!(c, ' ' | '.' | ',' | ':' | ';' | ')'))
    });
    let command = |part: &&str| names_test(part) && plain(part).split(' ').any(is_cargo);
    delimits && !prose.iter().any(command)
}

/// The filter reader reads each form of `--test it` Cargo takes, every
/// filter around it, a comment's and each file format's lines, and holds
/// each filter to a module whose tests the package's binary lists and an
/// `--exact` name to a listed test. A line that names no `--test`, or
/// plainly passes another target, is not read, whatever else it holds.
#[test]
fn the_filter_reader_holds_each_filter_to_its_crates_root() {
    let lists = fixture_lists();
    let cli = |at: &str, module: &str| Filter {
        at: at.into(),
        package: "brokkr-cli".into(),
        module: module.into(),
    };
    for (file, text, held) in [
        ("f", "cargo test --locked -p brokkr-cli --test it packaging::\n", vec![cli("f:1", "packaging")]),
        ("f", "cargo --locked test -p brokkr-cli --test=it -- --ignored --exact suppressions::a_b\n", vec![cli("f:1", "suppressions")]),
        ("f", "{\n  X=1 cargo test -p brokkr-cli --test it suppressions::a\n} > /dev/null 2>&1\n", vec![cli("f:2", "suppressions")]),
        ("f", "cargo test -p brokkr-cli --test it -- --skip gone:: --test-threads=1 --nocapture packaging::\n", vec![cli("f:1", "packaging")]),
        ("f", "\"cargo\" test -p 'brokkr-cli' --te\"st\" 'it' pack\"aging\"::\n", vec![cli("f:1", "packaging")]),
        ("f", "# Regenerate: X=1 cargo test -p brokkr-cli --test it suppressions::\n", vec![cli("f:1", "suppressions")]),
        ("f", "cargo test -p brokkr-cli --test it\ncargo test -p brokkr-cli --test heap_dsh \\\n  -- x 2>&1 | tee out\ncargo test --tests --test-threads 1 x | tee out\n", vec![]),
        ("f", "echo `date` $(pwd) @(x) \"2\">f; (cd x)\nBROKKR_X=1 \\\n  cargo test -p brokkr-cli --test it packaging::\n", vec![cli("f:3", "packaging")]),
        ("f.yml", "    run: |\n      cargo test -p brokkr-cli --test it packaging::\n      echo gone::\n", vec![cli("f.yml:2", "packaging")]),
        ("f.yml", "# cargo test -p brokkr-cli --test it packaging::\n  - run: cargo test -p brokkr-cli --test it suppressions:: # note\n  - run: echo gone::\n", vec![cli("f.yml:1", "packaging"), cli("f.yml:2", "suppressions")]),
        ("f.json", "{\"a\": \"X=a\\/b cargo test -p brokkr-cli --test it packaging::\",\n \"b\": [\"no_such_file::\"]}\n", vec![cli("f.json:1", "packaging")]),
        ("f.md", "Run `cargo test -p brokkr-cli --test\nit packaging::` once; it's quick.\n```sh\ncargo test -p brokkr-cli --test it suppressions::\n```\n", vec![cli("f.md:1", "packaging"), cli("f.md:4", "suppressions")]),
        ("f.md", "Pass --test heap_x or `cargo test -p brokkr-cli --test it packaging::` here.\n", vec![cli("f.md:1", "packaging")]),
        ("f.md", "`cargo test -p brokkr-cli --test it packaging::`: or (``cargo test -p brokkr-cli --test it suppressions::``).\n", vec![cli("f.md:1", "packaging"), cli("f.md:1", "suppressions")]),
    ] {
        assert_eq!(filters_in(file, text, &lists), Ok(held), "{text}");
    }
}

/// Every spelling of a stale filter the shell would pass is held and
/// refused: each form of `--test it`, a filter after `--skip`'s argument,
/// a second filter, one before the flag, a second package, a comment's, a
/// workflow value's before its comment, a wrapped Markdown span of either
/// length, and a JSON string's escapes, decoded.
#[test]
fn the_filter_reader_refuses_a_stale_filter_in_every_form() {
    let lists = fixture_lists();
    let refusal = |at: &str, package: &str, module: &str| Refusal::TestlessModule {
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
    let no_package = "cargo test --test it packaging::";
    let many = |command: &str| Refusal::ManyPackages {
        at: "f:1".into(),
        packages: vec!["brokkr-cli".into(), "brokkr-core".into()],
        command: command.into(),
    };
    let tilde = (
        "f.md",
        "~~~sh\ncargo test --locked -p brokkr-cli --test it no_such_file::\n~~~",
        refusal("f.md:2", "brokkr-cli", "no_such_file"),
    );
    let other = [
        (
            "cargo test -p brokkr-cli -p brokkr-core --test it packaging::",
            many("cargo test -p brokkr-cli -p brokkr-core --test it packaging::"),
        ),
        (
            "cargo test -p brokkr-cli -p brokkr-core --test it no_such_file::",
            many("cargo test -p brokkr-cli -p brokkr-core --test it no_such_file::"),
        ),
        (
            "cargo test -p brokkr-cli --test it suppress::",
            refusal("f:1", "brokkr-cli", "suppress"),
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
            "cargo test -p brokkr-cli --test it -- --exact gone::a",
            exact("gone::a"),
        ),
        (
            "cargo test -p brokkr-cli --test it gone::",
            refusal("f:1", "brokkr-cli", "gone"),
        ),
        (
            "cargo test -p brokkr-cli --test it empty::",
            refusal("f:1", "brokkr-cli", "empty"),
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact empty::helper",
            exact("empty::helper"),
        ),
        (
            "cargo test -p brokkr-cli --test it -- --exact broken::a",
            exact("broken::a"),
        ),
        (
            "cargo test -p brokkr-cli --test it broken::",
            refusal("f:1", "brokkr-cli", "broken"),
        ),
        (
            "cargo test -p brokkr-gone --test it x::",
            Refusal::UnreadList {
                at: "f:1".into(),
                package: "brokkr-gone".into(),
                why: "the workspace has no tests/it binary for it".into(),
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
    let other = (other.into_iter()).map(|(text, refused)| ("f", text, refused));
    for (file, text, refused) in stale.chain([tilde]).chain(other) {
        assert_eq!(filters_in(file, text, &lists), Err(refused), "{text}");
    }
}

/// A stale filter in each spelling, by file; each names `no_such_file` on
/// the file's first line.
const STALE: [(&str, &str); 14] = [
    ("f", "cargo test -p brokkr-cli --test=it no_such_file::"),
    ("f", "cargo test -p brokkr-cli --test 'it' no_such_file::"),
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
    ("f", "#cargo test -p brokkr-cli --test it no_such_file::"),
    (
        "f.yml",
        "run: cargo test -p brokkr-cli --test it no_such_file:: # x",
    ),
    (
        "f.md",
        "Run `cargo test -p brokkr-cli --test it\n  no_such_file::` once.",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli --test it no_such_file::`` once.",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli\n--test it no_such_file::`` once.",
    ),
    (
        "f.md",
        "    cargo test --locked -p brokkr-cli --test it no_such_file::",
    ),
    (
        "f.md",
        "| `cargo test --locked -p brokkr-cli --test it no_such_file::` | x |",
    ),
    (
        "f.json",
        "{\"run\": \"cargo test --locked -p brokkr-cli --test it no_such_file::\"}",
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
/// `test`, a `--test it` outside `cargo test`, a `--test` with no target
/// or run into other text, and a form a file's format does not let the
/// reader read exactly.
#[test]
fn the_filter_reader_refuses_a_word_or_form_it_cannot_read() {
    let lists = fixture_lists();
    let in_f = (UNREAD_WORDS.into_iter()).map(|(text, word)| ("f", text, 1, word, text));
    let one_line = (UNREAD_LINES.into_iter()).map(|(file, text)| (file, text, 1, "--test", text));
    for (file, text, line, word, command) in in_f.chain(one_line).chain(UNREAD_FORMS) {
        assert_eq!(
            filters_in(file, text, &lists),
            Err(unread_filter(&format!("{file}:{line}"), word, command)),
            "{text}"
        );
    }
}

/// A command in the closed grammar, on a file's first line, the reader
/// refuses by a word.
const UNREAD_WORDS: [(&str, &str); 17] = [
    ("cargo test -p brokkr-cli --test it packaging", "packaging"),
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
    ("echo --test", "--test"),
    ("cargo test -p brokkr-cli x--test it", "x--test"),
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
const UNREAD_FORMS: [(&str, &str, usize, &str, &str); 6] = [
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
    (
        "f.md",
        "Pass --test it with `cargo b`.",
        1,
        "--test",
        "Pass --test it with",
    ),
];

/// A line that may pass `--test it` is refused by its first character
/// outside the closed grammar, whatever the shell would make of it: every
/// input of the earlier reviews, and each Markdown code form whose
/// backticks the shell may run.
#[test]
fn the_filter_reader_refuses_a_line_by_its_first_character_outside_the_grammar() {
    let lists = fixture_lists();
    let (nbsp, acute) = (char::from(0xa0), char::from(0xe9));
    let foreign = [
        (format!("cargo test -p brokkr-cli --test{nbsp}it x::"), nbsp),
        (
            format!("cargo test -p brokkr-cli --test it pack{acute}::"),
            acute,
        ),
    ];
    let in_f = (OUTSIDE.into_iter())
        .map(|(text, outside)| (text.to_string(), outside))
        .chain(foreign)
        .map(|(text, outside)| ("f", text.clone(), 1, outside, text));
    let forms = (OUTSIDE_FORMS.into_iter()).map(|(file, text, line, outside, command)| {
        (file, text.to_string(), line, outside, command.to_string())
    });
    for (file, text, line, outside, command) in in_f.chain(forms) {
        assert_eq!(
            filters_in(file, &text, &lists),
            Err(unread_filter(
                &format!("{file}:{line}"),
                &outside.to_string(),
                &command
            )),
            "{text}"
        );
    }
}

/// A command, on a file's first line, refused by the character named.
const OUTSIDE: [(&str, char); 51] = [
    ("cargo test -p brokkr-cli --test  it no_such_file::", ' '),
    ("cargo test -p brokkr-cli --test\tit no_such_file::", '\t'),
    ("cargo test -p brokkr-cli --test it \"packaging:: x\"", ' '),
    ("cargo test -p brokkr-cli --test it ''", '\''),
    ("cargo test -p brokkr-cli --test it 'x::", '\''),
    ("cargo test -p brokkr-cli --test it \"x::", '"'),
    ("cargo test -p brokkr-cli --test it \"it's\"", '\''),
    ("cargo test -p brokkr-cli --test it x:: >", '>'),
    ("cargo test -p brokkr-cli --test it x:: \\", '\\'),
    ("cargo test -p brokkr-cli --test it x:: > --test", '>'),
    (
        "cargo test -p brokkr-cli --test it 2>&1 no_such_file::",
        '>',
    ),
    ("cargo test -p brokkr-cli --test it packaging:: # `x`", '#'),
    (
        "test_target=it; cargo test --locked -p brokkr-cli --test \"$test_target\" no_such_file::",
        ';',
    ),
    (
        "cargo test --locked -p brokkr-cli --test ${TEST_TARGET:-it} no_such_file::",
        '$',
    ),
    (
        "for t in it; do cargo test --locked -p brokkr-cli --test \"$t\" no_such_file::; done",
        ';',
    ),
    (
        "cargo test --locked -p brokkr-cli --test {it,no_such_file::}",
        '{',
    ),
    ("cargo test --locked -p brokkr-cli --test i* x::", '*'),
    ("cargo test --locked -p brokkr-cli --test=[i]t x::", '['),
    ("cargo test --locked -p brokkr-cli --test it$x x::", '$'),
    (
        "cargo test --locked -p brokkr-cli --test $(echo it) no_such_file::",
        '$',
    ),
    ("cargo test -p brokkr-cli --test `echo it` x::", '`'),
    ("cargo test -p brokkr-cli --test; echo it", ';'),
    ("report --test \"$x\" x::", '$'),
    (
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
        '`',
    ),
    (
        "cargo test --locked -p brokkr-cli --test=i`printf t` no_such_file::",
        '`',
    ),
    (
        "cargo test --locked -p brokkr-cli --test it`printf ''` no_such_file::",
        '`',
    ),
    (
        "cargo test --locked -p brokkr-cli --test it `echo no_such_file::`",
        '`',
    ),
    (
        "cargo test -p brokkr-cli --test it -- --exact suppressions::a_b`echo x`",
        '`',
    ),
    (
        "echo $(date); cargo test -p brokkr-cli --test it packaging::",
        '$',
    ),
    (
        "echo \"`date`\"; cargo test -p brokkr-cli --test it packaging::",
        '`',
    ),
    (
        "cargo test -p brokkr-cli --test it packaging:: > `mktemp`",
        '>',
    ),
    (
        "cargo test -p brokkr-cli --te\\st it packaging:: > `mktemp`",
        '\\',
    ),
    ("cargo test -p brokkr-cli --test it $(echo $(x) y) z::", '$'),
    ("cargo test -p brokkr-cli --test it x:: `x", '`'),
    (
        "cargo test --locked -p brokkr-cli --test @(it) no_such_file::",
        '(',
    ),
    ("args=(--test it no_such_file::)", '('),
    (
        "(cd x; cargo test -p brokkr-cli --test it packaging::)",
        '(',
    ),
    (
        "it) cargo test -p brokkr-cli --test it no_such_file:: ;;",
        ')',
    ),
    (
        "f() { cargo test -p brokkr-cli --test it packaging::; }",
        '(',
    ),
    (
        "cargo test --locked -p brokkr-cli --test it \"2\">/dev/null",
        '>',
    ),
    ("cargo test -p brokkr-cli --test it 1\"2\">/dev/null", '>'),
    (
        "cargo test -p brokkr-cli --test it 2>/dev/null no_such_file::",
        '>',
    ),
    ("cargo test -p brokkr-cli --test it \\2>/dev/null", '\\'),
    (
        "bash -c 'cargo test -p brokkr-cli --test it `echo x::`'",
        ' ',
    ),
    (
        "cargo test -p brokkr-cli --test it \"$(echo \"no_such_file::\")\"",
        '$',
    ),
    ("cargo test -p brokkr-cli --test it <(echo x)", '<'),
    ("cargo test -p brokkr-cli --test $'it' x::", '$'),
    ("cargo test -p brokkr-cli --test it x:: | tee out", '|'),
    ("cargo test -p brokkr-cli --test it x:: && echo ok", '&'),
    (
        "echo no_such_file:: | xargs cargo test --locked -p brokkr-cli --test it",
        '|',
    ),
    ("cargo test -p brokkr-cli --test it ~/x::", '~'),
];

/// A form, by file, refused at a line by the character named, with the
/// command it names: a workflow's expansions and backticks, a continued
/// line, a decoded tab, and in Markdown every code form whose backticks
/// the shell may run, among them the eleven of the fifth review.
const OUTSIDE_FORMS: [(&str, &str, usize, char, &str); 22] = [
    (
        "f.yml",
        "run: cargo test --locked -p brokkr-cli --test ${{ matrix.target }} no_such_file::",
        1,
        '$',
        "cargo test --locked -p brokkr-cli --test ${{ matrix.target }} no_such_file::",
    ),
    (
        "f.yml",
        "run: |\n  cargo test -p brokkr-cli --test '${{ matrix.target }}' x::",
        2,
        '$',
        "cargo test -p brokkr-cli --test '${{ matrix.target }}' x::",
    ),
    (
        "f.yml",
        "run: |\n  cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
        2,
        '`',
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
    ),
    (
        "f",
        "cargo test -p brokkr-cli --test \\\n  it no_such_file::",
        1,
        '\\',
        "cargo test -p brokkr-cli --test \\",
    ),
    (
        "f",
        "X=1 cargo test -p brokkr-cli --test it \\\n  suppressions::a > /dev/null",
        1,
        '\\',
        "X=1 cargo test -p brokkr-cli --test it \\",
    ),
    (
        "f.json",
        "{\"run\": \"cargo test --locked -p brokkr-cli --test it\\tno_such_file::\"}",
        1,
        '\t',
        "cargo test --locked -p brokkr-cli --test it\tno_such_file::",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::``.",
        1,
        '`',
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli --test=i`printf t` no_such_file::``.",
        1,
        '`',
        "cargo test --locked -p brokkr-cli --test=i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli --test it`printf ''` no_such_file::``.",
        1,
        '`',
        "cargo test --locked -p brokkr-cli --test it`printf ''` no_such_file::",
    ),
    (
        "f.md",
        "Run `` cargo test --locked -p brokkr-cli --test it `echo no_such_file::` `` once.",
        1,
        '`',
        "cargo test --locked -p brokkr-cli --test it `echo no_such_file::`",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli --test it -- --exact suppressions::a_b`echo x` ``.",
        1,
        '`',
        "cargo test --locked -p brokkr-cli --test it -- --exact suppressions::a_b`echo x`",
    ),
    (
        "f.md",
        "Run ``cargo test --locked -p brokkr-cli\n--test i`printf t` no_such_file::``.",
        1,
        '`',
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "~~~sh\ncargo test --locked -p brokkr-cli --test i`printf t` no_such_file::\n~~~",
        2,
        '`',
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "Run:\n\n    cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
        3,
        '`',
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "Run:\n\n    cargo test --locked -p brokkr-cli --test it `echo no_such_file::`",
        3,
        '`',
        "cargo test --locked -p brokkr-cli --test it `echo no_such_file::`",
    ),
    (
        "f.md",
        "> ```sh\n> cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::\n> ```",
        1,
        '>',
        "sh > cargo test --locked -p brokkr-cli --test i`printf t` no_such_file:: >",
    ),
    (
        "f.md",
        "````\n```sh\ncargo test --locked -p brokkr-cli --test i`printf t` no_such_file::\n```\n````",
        3,
        '`',
        "cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "<pre>cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::</pre>",
        1,
        '<',
        "<pre>cargo test --locked -p brokkr-cli --test i`printf t` no_such_file::</pre>",
    ),
    (
        "f.md",
        "Run cargo test -p brokkr-cli `printf -- --test=i`t no_such_file:: here.",
        1,
        '`',
        "Run cargo test -p brokkr-cli `printf -- --test=i`t no_such_file:: here.",
    ),
    (
        "f.md",
        "cargo test -p brokkr-cli `true` --test=i`printf t` no_such_file::",
        1,
        '`',
        "cargo test -p brokkr-cli `true` --test=i`printf t` no_such_file::",
    ),
    (
        "f.md",
        "Run `` cargo test -p brokkr-cli --test it ` no_such_file:: ` `` once.",
        1,
        '`',
        "cargo test -p brokkr-cli --test it ` no_such_file:: `",
    ),
    (
        "f.md",
        "Run `cargo test -p brokkr-cli --test it no_such_file::`s once.",
        1,
        '`',
        "Run `cargo test -p brokkr-cli --test it no_such_file::`s once.",
    ),
];

/// Each character outside the closed grammar, and a second space, put
/// anywhere in a line the reader holds, refuses the line by that
/// character; only a `#` that opens it, making it a comment, leaves it
/// read as it was.
#[test]
fn a_character_outside_the_grammar_refuses_a_held_line_wherever_it_stands() {
    let lists = fixture_lists();
    let outside = [
        '#',
        '$',
        '`',
        '\\',
        ';',
        '&',
        '|',
        '<',
        '>',
        '(',
        ')',
        '{',
        '}',
        '*',
        '?',
        '[',
        ']',
        '!',
        '~',
        '%',
        '^',
        '\t',
        char::from(0xa0),
        char::from(0xe9),
    ];
    for (line, module) in [
        (
            "cargo test --locked -p brokkr-cli --test it packaging::",
            "packaging",
        ),
        (
            "X=1 cargo test -p brokkr-cli --test=it -- --exact suppressions::a_b",
            "suppressions",
        ),
    ] {
        let held = || {
            Ok(vec![Filter {
                at: "f:1".into(),
                package: "brokkr-cli".into(),
                module: module.into(),
            }])
        };
        assert_eq!(filters_in("f", line, &lists), held(), "{line}");
        let spaces = line
            .match_indices(' ')
            .map(|(at, space)| (at, space.to_string()));
        let inserted = (0..=line.len()).flat_map(|at| outside.map(|c| (at, c.to_string())));
        for (at, c) in inserted.chain(spaces) {
            let variant = format!("{}{c}{}", &line[..at], &line[at..]);
            let refused = match (at, c.as_str()) {
                (0, "#") => held(),
                _ => Err(unread_filter("f:1", &c, &variant)),
            };
            assert_eq!(filters_in("f", &variant, &lists), refused, "{variant:?}");
        }
    }
}

/// A test a `cfg_attr` gates and a module its own `#![cfg]` gates are no
/// test of the binary that was built without them: `attr_gated`'s
/// `absent` is behind `#[cfg_attr(test, cfg(target_os = "macos"))]` and
/// `inner_gated`'s every test behind its own `#![cfg]`, so the binary's
/// list carries neither. The list is the fact; the binary has already
/// resolved the gates, and the host's own list is pinned by
/// `the_binarys_own_list_is_what_the_gate_holds`. A module whose other
/// test the list carries is still held, and so is a listed name.
#[test]
fn a_test_or_module_gated_from_this_host_is_no_test_of_the_binary() {
    let lists = fixture_lists();
    let module_form = "cargo test -p brokkr-cli --test it inner_gated::\n";
    assert_eq!(
        filters_in("f", module_form, &lists),
        Err(Refusal::TestlessModule {
            at: "f:1".into(),
            package: "brokkr-cli".into(),
            module: "inner_gated".into(),
        }),
        "{module_form}"
    );
    let exact_form = "cargo test -p brokkr-cli --test it -- --exact attr_gated::absent\n";
    assert_eq!(
        filters_in("f", exact_form, &lists),
        Err(Refusal::UnknownTest {
            at: "f:1".into(),
            package: "brokkr-cli".into(),
            name: "attr_gated::absent".into(),
        }),
        "{exact_form}"
    );
    let held = Ok(vec![Filter {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        module: "attr_gated".into(),
    }]);
    assert_eq!(
        filters_in(
            "f",
            "cargo test -p brokkr-cli --test it attr_gated::\n",
            &lists
        ),
        held,
        "the module's listed test holds it"
    );
    assert_eq!(
        filters_in(
            "f",
            "cargo test -p brokkr-cli --test it -- --exact attr_gated::kept\n",
            &lists
        ),
        held,
        "a listed name is held"
    );
}

/// #543: the list is the binary of the command's own build configuration —
/// default features in the dev profile — so a command that names a feature
/// set, a profile, a `--config`, an unstable `-Z` or a `+toolchain` the
/// gate does not list is refused, named, in both the module and the
/// `--exact` form, and a command of the default configuration is still
/// held. Each can build another binary, whose list is not the command's;
/// the gate must refuse rather than vouch the filter against the wrong
/// list.
#[test]
fn a_command_of_an_unlisted_configuration_is_refused() {
    let lists = fixture_lists();
    let refused = |configuration: &str| Refusal::UnreadList {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        why: format!(
            "the gate lists it with default features in the dev profile, not `{configuration}`"
        ),
    };
    for (text, configuration) in [
        (
            "cargo test --locked -p brokkr-cli --all-features --test it packaging::\n",
            "--all-features",
        ),
        (
            "cargo test --locked -p brokkr-cli --no-default-features --test it packaging::\n",
            "--no-default-features",
        ),
        (
            "cargo test --locked -p brokkr-cli --release --test it packaging::\n",
            "--release",
        ),
        (
            "cargo test --locked -p brokkr-cli --features test-support --test it packaging::\n",
            "--features",
        ),
        (
            "cargo test --locked -p brokkr-cli --features=test-support --test it packaging::\n",
            "--features=test-support",
        ),
        (
            "cargo test --locked -p brokkr-cli -F test-support --test it packaging::\n",
            "-F",
        ),
        (
            "cargo test --locked -p brokkr-cli --config profile.test.debug-assertions=false \
             --test it packaging::\n",
            "--config",
        ),
        (
            "cargo test -p brokkr-cli -Z unstable-options --test it packaging::\n",
            "-Z",
        ),
        (
            "cargo +nightly test -p brokkr-cli --test it packaging::\n",
            "+nightly",
        ),
    ] {
        assert_eq!(
            filters_in("f", text, &lists),
            Err(refused(configuration)),
            "{text}"
        );
    }
    for (text, configuration) in [
        (
            "cargo test --locked -p brokkr-cli --all-features --test it -- --exact \
             suppressions::a_b\n",
            "--all-features",
        ),
        (
            "cargo test --locked -p brokkr-cli --config x --test it -- --exact suppressions::a_b\n",
            "--config",
        ),
        (
            "cargo test -p brokkr-cli -Z unstable-options --test it -- --exact suppressions::a_b\n",
            "-Z",
        ),
        (
            "cargo +nightly test -p brokkr-cli --test it -- --exact suppressions::a_b\n",
            "+nightly",
        ),
    ] {
        assert_eq!(
            filters_in("f", text, &lists),
            Err(refused(configuration)),
            "{text}"
        );
    }
    let held = Ok(vec![Filter {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        module: "packaging".into(),
    }]);
    assert_eq!(
        filters_in(
            "f",
            "cargo test --locked -p brokkr-cli --test it packaging::\n",
            &lists
        ),
        held,
        "a command of the default configuration is held"
    );
}

/// #543: an assignment before `cargo` whose name is `CARGO_*` or `RUST*`
/// sets Cargo's own configuration or rustc's flags, so it can build
/// another binary than the gate lists. It is refused, named, in both the
/// module and the `--exact` form; another assignment is held, as
/// `the_filter_reader_holds_each_filter_to_its_crates_root`'s `X=1` and
/// `BROKKR_X=1` rows show.
#[test]
fn an_assignment_that_can_move_the_build_is_refused() {
    let lists = fixture_lists();
    let refused = |assignment: &str| Refusal::UnreadList {
        at: "f:1".into(),
        package: "brokkr-cli".into(),
        why: format!(
            "the gate lists it with default features in the dev profile, not `{assignment}`"
        ),
    };
    for (text, assignment) in [
        (
            "CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=false cargo test -p brokkr-cli --test it \
             packaging::\n",
            "CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=false",
        ),
        (
            "RUSTFLAGS=--cfg=other cargo test --locked -p brokkr-cli --test it packaging::\n",
            "RUSTFLAGS=--cfg=other",
        ),
        (
            "RUSTUP_TOOLCHAIN=nightly cargo test -p brokkr-cli --test it -- --exact \
             suppressions::a_b\n",
            "RUSTUP_TOOLCHAIN=nightly",
        ),
    ] {
        assert_eq!(
            filters_in("f", text, &lists),
            Err(refused(assignment)),
            "{text}"
        );
    }
}

/// #543: a workflow's or a script's `--test it` command runs through the
/// one checked entry point, which fails a filtered run that executes 0
/// tests; a command that bypasses it is refused. A guide's command is read
/// from its text alone, because an operator copies it and no job here runs
/// it.
#[test]
fn a_workflow_or_script_command_must_run_through_the_checked_entry_point() {
    let lists = fixture_lists();
    let bare = "cargo test --locked -p brokkr-cli --test it packaging::";
    for (file, text) in [
        (".github/workflows/ci.yml", format!("run: {bare}\n")),
        ("scripts/measure-budgets.sh", format!("{bare}\n")),
    ] {
        assert_eq!(
            filters_in(file, &text, &lists),
            Err(Refusal::Unguarded {
                at: format!("{file}:1"),
                command: bare.into(),
            }),
            "{file}"
        );
    }
    let guarded = format!("scripts/run-it-tests.sh {bare}");
    for (file, text) in [
        (".github/workflows/ci.yml", format!("run: {guarded}\n")),
        ("scripts/measure-budgets.sh", format!("{guarded}\n")),
    ] {
        assert_eq!(
            filters_in(file, &text, &lists),
            Ok(vec![Filter {
                at: format!("{file}:1"),
                package: "brokkr-cli".into(),
                module: "packaging".into(),
            }]),
            "{file}"
        );
    }
    assert_eq!(
        filters_in("docs/guides/x.md", &format!("{bare}\n"), &lists),
        Ok(vec![Filter {
            at: "docs/guides/x.md:1".into(),
            package: "brokkr-cli".into(),
            module: "packaging".into(),
        }]),
        "a guide's command needs no entry point"
    );
}

/// The lists the rows are held to, as a binary reports them — one home
/// for both row tests' fixture, this module's and the parent's
/// `a_filter_is_held_to_a_module_that_holds_tests_and_a_name_to_a_test`.
/// brokkr-cli's carries `attr_gated`, `hands`, `layering`, `packaging`
/// and `suppressions` tests; brokkr-core's and brokkr-runtime's carry
/// their own; no support module's name is in any list. `suppressions::a_b`
/// is a listed name and `suppressions::helper` is not, nor
/// `suppressions::nested`: the test under `mod inner` is named
/// `suppressions::inner::nested` and the reader reads no such path.
pub(super) fn fixture_lists() -> Lists {
    BTreeMap::from([
        (
            "brokkr-cli".to_string(),
            BTreeSet::from([
                "attr_gated::kept".to_string(),
                "hands::a_test_the_binary_carries".to_string(),
                "layering::test_targets::a_test_of_the_directory_module".to_string(),
                "packaging::packaging_holds_a_test".to_string(),
                "suppressions::a_b".to_string(),
            ]),
        ),
        (
            "brokkr-core".to_string(),
            BTreeSet::from(["core::a_test_the_binary_carries".to_string()]),
        ),
        (
            "brokkr-runtime".to_string(),
            BTreeSet::from(["witness_digests::a_test_the_binary_carries".to_string()]),
        ),
    ])
}
