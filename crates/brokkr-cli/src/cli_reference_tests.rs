//! The command line, documented from the one place it is defined (#362).
//!
//! `docs/reference/cli.md` is rendered here from clap's own `Command`
//! tree and the [`Exit`] enum, and a test fails when the committed page
//! differs, printing the command that regenerates it. A second test reads
//! every fenced `brokkr` line in every living doc and parses it with the
//! same [`Cli`], so an example that no longer parses fails CI.
//!
//! The fence reader fails closed on what it cannot read (the operator's
//! 2026-09-26 ruling). A `brokkr` line is read in a shell fence (`sh`,
//! `bash`, `shell`, `zsh`, `console`, or one with no language; a fence
//! that prompts anywhere holds output too, and so only its `$ ` lines
//! are commands), in a blockquote as well as out of one. A line that
//! names `brokkr` is split by the same word splitter that parses it,
//! continuations joined as the shell joins them, and every `brokkr`
//! command it joins with `&&`, `;` or a pipe is handed whole to clap,
//! past any leading `NAME=value` words and keywords such as `if` and
//! `do`. A path to `brokkr` (`./brokkr`, `target/release/brokkr`) as the
//! program word runs it and is parsed too, as are the words after `--`
//! in `cargo run -p brokkr-cli -- …`; as an argument, such a path names a
//! file. Every other `brokkr` on the line, in a quote, a substitution, an
//! argument, a comment or another program's words, is refused, and so is
//! one in any other fence, one without a prompt in a fence that prompts,
//! a fence that never closes, an unquoted `$` expansion, and a shell
//! construct the word splitter does not know.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command, CommandFactory, Parser};

use crate::exit::Exit;
use crate::Cli;

#[path = "../tests/support/records.rs"]
mod records;
#[path = "../tests/support/tracked.rs"]
mod tracked;

/// Where the rendered page lives, from the workspace root.
const REFERENCE: &str = "docs/reference/cli.md";

/// The command that rewrites [`REFERENCE`] from the definitions.
const REGENERATE: &str =
    "BROKKR_REGENERATE_CLI_REFERENCE=1 cargo test -p brokkr-cli --lib cli_reference";

/// How an argument that names a run reads it: the page's Selector column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Selector {
    /// Always through decision 0015's `selector::resolve_run`: a full
    /// id, a unique prefix, or `latest`.
    Always,
    /// Through the resolver when the workspace journal is there, and
    /// otherwise taken literally with `latest` refused (`keep_ref_run`).
    WithJournal,
    /// Never through the resolver: the id is taken as written.
    Literal,
}

/// Every argument that names a run, by verb and argument id, with how it
/// reads the run. `verbs/tests.rs` pins the verbs that came to the
/// resolver last. An entry that names no argument, and an argument whose
/// id names a run with no entry, fail `every_selector_names_an_argument`.
const SELECTORS: [(&str, &str, Selector); 21] = [
    ("brokkr costs", "run", Selector::Always),
    ("brokkr ledger", "run", Selector::Always),
    ("brokkr anchor", "run", Selector::Always),
    ("brokkr keep-refs plant", "run", Selector::Always),
    ("brokkr keep-refs list", "run", Selector::WithJournal),
    ("brokkr keep-refs delete", "run", Selector::WithJournal),
    ("brokkr tui", "run", Selector::Always),
    ("brokkr resume", "run", Selector::Always),
    ("brokkr rerun", "run", Selector::Always),
    ("brokkr compare", "run_a", Selector::Always),
    ("brokkr compare", "run_b", Selector::Always),
    ("brokkr conclude", "run", Selector::Always),
    ("brokkr operator", "run", Selector::Always),
    ("brokkr operator", "by_run", Selector::Literal),
    ("brokkr inspect", "run", Selector::Always),
    ("brokkr transcript", "run", Selector::Always),
    ("brokkr seats", "run", Selector::Always),
    ("brokkr watch", "run", Selector::Always),
    ("brokkr replay", "run", Selector::Always),
    ("brokkr export", "run", Selector::Always),
    ("brokkr bridge", "run", Selector::Always),
];

/// How `verb`'s argument reads a run, when it names one.
fn selector(verb: &str, arg: &Arg) -> Option<Selector> {
    SELECTORS
        .iter()
        .find(|(path, id, _)| *path == verb && arg.get_id() == *id)
        .map(|(_, _, selector)| *selector)
}

/// The exit the table lists after `exit`, matched exhaustively so a new
/// variant has to be given its place.
fn next(exit: Exit) -> Option<Exit> {
    match exit {
        Exit::Completed => Some(Exit::Failed),
        Exit::Failed => Some(Exit::Running),
        Exit::Running => Some(Exit::Parked),
        Exit::Parked => Some(Exit::Usage),
        Exit::Usage => Some(Exit::Stopped),
        Exit::Stopped => Some(Exit::Contended),
        Exit::Contended => Some(Exit::RunnerFailed),
        Exit::RunnerFailed => Some(Exit::Boxed(0)),
        Exit::Boxed(_) => Some(Exit::Signalled(0)),
        Exit::Signalled(_) => None,
    }
}

/// Every exit, in the order the table lists them.
fn exits() -> Vec<Exit> {
    std::iter::successors(Some(Exit::Completed), |exit| next(*exit)).collect()
}

/// What each exit says, matched exhaustively so a new variant cannot be
/// left out of the table: its name, the code column and its meaning.
fn exit_row(exit: Exit) -> (&'static str, String, &'static str) {
    let code = exit.code().to_string();
    let (name, code) = match exit {
        Exit::Completed => ("completed", code),
        Exit::Failed => ("failed", code),
        Exit::Running => ("running", code),
        Exit::Parked => ("parked", code),
        Exit::Stopped => ("stopped", code),
        Exit::Contended => ("contended", code),
        Exit::Usage => ("usage", code),
        Exit::RunnerFailed => ("runner failed", code),
        Exit::Boxed(_) => ("boxed", "its own".to_string()),
        Exit::Signalled(_) => ("signalled", "128 + signal".to_string()),
    };
    (name, code, exit.meaning())
}

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the workspace root")
        .to_path_buf()
}

/// Text from a doc comment, made safe for a Markdown page: `<` and `>`
/// outside a code span are escaped so `refs/forge/<run>` is not read as a
/// tag, and in a table cell `|` is escaped and lines are joined.
fn prose(text: &str, cell: bool) -> String {
    let mut out = String::new();
    let mut code = false;
    for character in text.chars() {
        match character {
            '`' => {
                code = !code;
                out.push('`');
            }
            '<' if !code => out.push_str("&lt;"),
            '>' if !code => out.push_str("&gt;"),
            '|' if cell => out.push_str("\\|"),
            '\n' if cell => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// The verbs a reader can type, depth first in definition order: clap's
/// own `help` and every hidden verb are left out.
fn verbs(command: &Command) -> Vec<&Command> {
    command
        .get_subcommands()
        .filter(|verb| !verb.is_hide_set() && verb.get_name() != "help")
        .flat_map(|verb| std::iter::once(verb).chain(verbs(verb)))
        .collect()
}

/// An argument as a reader types it: `--flag`, `--flag <VALUE>` or
/// `<VALUE>`, with `...` where it repeats.
fn spelled(arg: &Arg) -> String {
    let value = arg
        .get_value_names()
        .map(|names| names.join(" "))
        .unwrap_or_else(|| arg.get_id().as_str().to_uppercase());
    let repeats = if matches!(arg.get_action(), ArgAction::Append) {
        "..."
    } else {
        ""
    };
    match arg.get_long() {
        Some(long) if arg.get_action().takes_values() => format!("--{long} <{value}>{repeats}"),
        Some(long) => format!("--{long}"),
        None => format!("<{value}>{repeats}"),
    }
}

/// An argument's description, from its doc comment, as a table cell.
fn description(arg: &Arg) -> String {
    arg.get_long_help()
        .or(arg.get_help())
        .map(|help| prose(&help.to_string(), true))
        .unwrap_or_default()
}

/// One argument's table row, in the verb at `path`.
fn argument_row(path: &str, arg: &Arg) -> String {
    let default = if arg.get_action().takes_values() {
        arg.get_default_values()
            .iter()
            .map(|value| format!("`{}`", value.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        String::new()
    };
    let selector = match selector(path, arg) {
        Some(Selector::Always) => "yes",
        Some(Selector::WithJournal) => "with a journal",
        Some(Selector::Literal) | None => "",
    };
    let help = description(arg);
    format!("| `{}` | {default} | {selector} | {help} |\n", spelled(arg))
}

/// The arguments a verb's table lists: clap's own help and version and
/// every hidden argument are left out.
fn listed(verb: &Command) -> Vec<&Arg> {
    verb.get_arguments()
        .filter(|arg| !arg.is_hide_set())
        .filter(|arg| !matches!(arg.get_action(), ArgAction::Help | ArgAction::Version))
        .collect()
}

/// Every listed argument with no description, as `verb argument`: a row
/// the page would print with its Description cell empty.
fn undescribed(cli: &Command) -> Vec<String> {
    verbs(cli)
        .into_iter()
        .flat_map(|verb| {
            let path = verb.get_bin_name().expect("a built command names its path");
            listed(verb)
                .into_iter()
                .filter(|arg| description(arg).trim().is_empty())
                .map(move |arg| format!("{path} {}", spelled(arg)))
        })
        .collect()
}

/// One verb's section: its heading, its description, its usage and a
/// row per argument.
fn section(verb: &Command) -> String {
    let path = verb.get_bin_name().expect("a built command names its path");
    let mut out = format!("## {path}\n\n");
    if let Some(about) = verb.get_long_about().or(verb.get_about()) {
        out.push_str(&format!("{}\n\n", prose(&about.to_string(), false)));
    }
    let usage = verb.clone().render_usage().to_string();
    out.push_str(&format!("```text\n{}\n```\n\n", usage.trim_end()));
    let arguments = listed(verb);
    if !arguments.is_empty() {
        out.push_str("| Argument | Default | Selector | Description |\n");
        out.push_str("| --- | --- | --- | --- |\n");
        for arg in arguments {
            out.push_str(&argument_row(path, arg));
        }
        out.push('\n');
    }
    out
}

/// The exit-code table, from [`exits`].
fn exit_codes() -> String {
    let mut out = String::from(
        "## Exit codes\n\n\
         Every code the binary exits with, from `crates/brokkr-cli/src/exit.rs`.\n\n\
         | Code | Name | Meaning |\n| --- | --- | --- |\n",
    );
    for exit in exits() {
        let (name, code, meaning) = exit_row(exit);
        out.push_str(&format!("| {code} | {name} | {meaning} |\n"));
    }
    out
}

/// The command tree, built so every verb knows its path.
fn built() -> Command {
    let mut cli = Cli::command();
    cli.build();
    cli
}

/// The whole page.
fn reference() -> String {
    let cli = built();
    let mut page = format!(
        "# CLI reference\n\n\
         <!-- Rendered from the clap definitions by \
         crates/brokkr-cli/src/cli_reference_tests.rs; do not edit by hand. \
         Regenerate with: {REGENERATE} -->\n\n\
         Every `brokkr` verb and argument with its default, and every exit \
         code, as the binary defines them; `brokkr <verb> --help` prints the \
         same text. An argument marked **yes** under Selector takes a full \
         run id, a unique prefix of one, or `latest`, the run created most \
         recently (decision 0015, and for the write paths its proposed \
         2026-09-28 addendum); one marked **with a journal** does so only \
         when the workspace journal is there, and otherwise takes the id \
         literally and refuses `latest`. Every verb also takes clap's own `-h`/`--help`, \
         and `brokkr` itself `-V`/`--version`; the tables leave them out.\n\n"
    );
    for verb in verbs(&cli) {
        let path = verb.get_bin_name().expect("a built command names its path");
        let anchor = path.replace(' ', "-");
        let about = verb.get_about().map(|a| a.to_string()).unwrap_or_default();
        page.push_str(&format!(
            "- [`{path}`](#{anchor}): {}\n",
            prose(about.trim(), true)
        ));
    }
    page.push_str("- [Exit codes](#exit-codes)\n\n");
    for verb in verbs(&cli) {
        page.push_str(&section(verb));
    }
    page.push_str(&exit_codes());
    page
}

#[test]
fn the_cli_reference_is_rendered_from_clap() {
    let path = workspace().join(REFERENCE);
    let rendered = reference();
    if std::env::var_os("BROKKR_REGENERATE_CLI_REFERENCE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &rendered).unwrap();
    }
    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == rendered,
        "{REFERENCE} differs from the clap definitions; regenerate it with:\n  {REGENERATE}"
    );
    let undescribed = undescribed(&built());
    assert!(
        undescribed.is_empty(),
        "arguments {REFERENCE} would list with no description; give each a doc \
         comment saying what it takes and what its absence means:\n{}",
        undescribed.join("\n")
    );
}

#[test]
fn an_argument_without_a_description_is_refused() {
    let mut cli = Command::new("brokkr").subcommand(
        Command::new("verb")
            .arg(
                Arg::new("described")
                    .long("described")
                    .help("What it takes."),
            )
            .arg(Arg::new("bare").long("bare"))
            .arg(Arg::new("blank").help(" ")),
    );
    cli.build();
    assert_eq!(
        undescribed(&cli),
        ["brokkr verb --bare <BARE>", "brokkr verb <BLANK>"]
    );
}

#[test]
fn every_selector_names_an_argument() {
    let cli = built();
    let verbs = verbs(&cli);
    for (path, id, _) in SELECTORS {
        let verb = verbs
            .iter()
            .find(|verb| verb.get_bin_name() == Some(path))
            .unwrap_or_else(|| panic!("{path} is no verb"));
        assert!(
            listed(verb).iter().any(|arg| arg.get_id() == id),
            "{path} has no argument {id}"
        );
    }
    // And back: an argument whose id names a run says how it reads one.
    for verb in &verbs {
        let path = verb.get_bin_name().expect("a built command names its path");
        for arg in listed(verb) {
            let names_a_run = arg.get_id().as_str().split('_').any(|part| part == "run");
            assert!(
                !names_a_run || selector(path, arg).is_some(),
                "{path} {} names a run and has no row in SELECTORS",
                spelled(arg)
            );
        }
    }
    let keep_refs_list = verbs
        .iter()
        .find(|verb| verb.get_bin_name() == Some("brokkr keep-refs list"))
        .unwrap();
    let run = listed(keep_refs_list)
        .into_iter()
        .find(|arg| arg.get_id() == "run")
        .unwrap();
    assert_eq!(
        selector("brokkr keep-refs list", run),
        Some(Selector::WithJournal)
    );
    assert_eq!(
        selector("brokkr keep-refs plant", run),
        Some(Selector::Always)
    );
    assert_eq!(selector("brokkr runs", run), None);
}

#[test]
fn every_exit_is_in_the_table_once() {
    let names: Vec<&str> = exits().into_iter().map(|exit| exit_row(exit).0).collect();
    assert_eq!(
        names,
        [
            "completed",
            "failed",
            "running",
            "parked",
            "usage",
            "stopped",
            "contended",
            "runner failed",
            "boxed",
            "signalled"
        ]
    );
    let table = exit_codes();
    assert!(table.contains("| 4 | contended |"), "{table}");
    assert!(table.contains("| its own | boxed |"), "{table}");
    assert!(table.contains("| 128 + signal | signalled |"), "{table}");
}

/// The fences whose lines are commands, a fence with no language among
/// them, as most of the guides write one. A fence that prompts anywhere
/// shows output beside its commands, so there only a line after a `$ `
/// prompt is one; a fence with no prompt is all commands.
const SHELLS: [&str; 6] = ["", "sh", "bash", "shell", "zsh", "console"];

/// The fences whose lines are never commands: diagrams, output and data.
const NOT_SHELLS: [&str; 7] = [
    "text", "json", "markdown", "mermaid", "diff", "toml", "yaml",
];

/// Shell words that may stand before the command they govern: a keyword
/// that opens or continues a compound command, `!`, and `{`.
const KEYWORDS: [&str; 9] = [
    "if", "then", "elif", "else", "while", "until", "do", "!", "{",
];

/// How many of `text`'s tokens `named` accepts. Quotes and backslashes
/// are taken off first, as the word splitter takes them off, and a token
/// runs through letters, digits, `-`, `_`, `.` and `/`.
fn count(text: &str, named: fn(&str) -> bool) -> usize {
    text.replace(['\'', '"', '\\'], "")
        .split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/')))
        .filter(|token| named(token))
        .count()
}

/// How many times `text` names `brokkr` as a word of its own, wherever
/// it stands: quoted, escaped, inside a substitution, in a comment, or
/// before a `:`. A path or a URL that passes through or ends in a
/// `brokkr` directory names a file, and `brokkr-cli` the crate, not it.
fn mentions(text: &str) -> usize {
    count(text, |token| token == "brokkr")
}

/// Whether a word runs `brokkr` when it is a command's program word:
/// `brokkr` itself, or a path to it such as `./brokkr` or
/// `target/release/brokkr`. As an argument, such a path names a file.
fn executable(word: &str) -> bool {
    word == "brokkr" || word.ends_with("/brokkr")
}

/// How many words in `text` could run `brokkr`: an [`executable`] one,
/// or the `brokkr-cli` crate [`cargo_run`] runs. Every line that holds
/// one is read.
fn executables(text: &str) -> usize {
    count(text, |token| executable(token) || token == "brokkr-cli")
}

/// A shell assignment word, `NAME=value`, which may lead a command.
fn assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        name.chars()
            .next()
            .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
            && name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
    })
}

/// Where one split command's program word stands: past any leading
/// assignments and keywords.
fn program_word(argv: &[String]) -> usize {
    argv.iter()
        .position(|word| !assignment(word) && !KEYWORDS.contains(&word.as_str()))
        .unwrap_or(argv.len())
}

/// The `brokkr` command one split command runs, as clap is handed it, or
/// nothing when it runs another program: from an [`executable`] program
/// word on, or what [`cargo_run`] runs. A `brokkr` anywhere else is
/// [`stray`]: this reader cannot tell whether it runs.
fn program(argv: &[String]) -> Option<Vec<String>> {
    let argv = &argv[program_word(argv)..];
    match argv.first() {
        Some(word) if executable(word) => Some(argv.to_vec()),
        _ => cargo_run(argv),
    }
}

/// The `brokkr` command a `cargo run -p brokkr-cli` runs: `brokkr` and
/// the words after `--`, parsed as any other `brokkr` command is.
fn cargo_run(argv: &[String]) -> Option<Vec<String>> {
    let dashes = argv.iter().position(|word| word == "--");
    let (cargo, arguments) = argv.split_at(dashes.unwrap_or(argv.len()));
    let package = cargo
        .windows(2)
        .any(|pair| matches!(pair[0].as_str(), "-p" | "--package") && pair[1] == "brokkr-cli")
        || cargo
            .iter()
            .any(|word| matches!(word.as_str(), "-pbrokkr-cli" | "--package=brokkr-cli"));
    let runs = cargo.first().is_some_and(|word| word == "cargo")
        && cargo.iter().any(|word| word == "run")
        && package;
    let brokkr = std::iter::once("brokkr".to_string());
    runs.then(|| brokkr.chain(arguments.iter().skip(1).cloned()).collect())
}

/// The first word on a split line that names `brokkr` and is not the
/// program word of a `brokkr` command: one nothing parses.
fn stray(commands: &[Vec<String>]) -> Option<&String> {
    commands
        .iter()
        .flat_map(|argv| {
            let parsed = program(argv).map(|_| program_word(argv));
            argv.iter()
                .enumerate()
                .filter(move |(index, _)| Some(*index) != parsed)
                .map(|(_, word)| word)
        })
        .find(|word| mentions(word) > 0)
}

/// Why a `brokkr` the word splitter drops, in a comment or a redirect's
/// target, is refused: it was never handed to clap.
const DROPPED: &str = "an unparsed `brokkr` the word splitter drops, in a comment or a redirect";

/// Why a word that names `brokkr` outside a parsed `brokkr` command is
/// refused.
fn unparsed(word: &str) -> String {
    format!(
        "an unparsed `brokkr` in `{word}`: only a command that begins with \
         `brokkr` is parsed, and every other mention is refused"
    )
}

/// One `brokkr` command a fence carries, continuation lines joined.
#[derive(Debug, PartialEq, Eq)]
struct Fenced {
    line: usize,
    text: String,
}

/// One fence: the line it opens on, its language, and each line inside
/// it, numbered, with the blockquote the fence stands in taken off.
struct Fence<'a> {
    opened: usize,
    language: String,
    lines: Vec<(usize, &'a str)>,
}

/// A line with up to `depth` blockquote markers (`>` and one space)
/// taken off, and how many it had.
fn unquote(line: &str, depth: usize) -> (usize, &str) {
    let mut rest = line;
    let mut taken = 0;
    while taken < depth {
        let Some(inner) = rest.trim_start().strip_prefix('>') else {
            break;
        };
        rest = inner.strip_prefix(' ').unwrap_or(inner);
        taken += 1;
    }
    (taken, rest)
}

/// A fence line's marker and its info string, when the line is one.
fn fence_marker(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let length = trimmed
        .find(|c| c != '`' && c != '~')
        .unwrap_or(trimmed.len());
    (length >= 3).then(|| (&trimmed[..length], trimmed[length..].trim()))
}

/// How many spaces `line` begins with.
fn indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// `line` with up to `indent` leading spaces taken off, as a fence
/// opened that far in takes them off its lines.
fn outdent(line: &str, indent: usize) -> &str {
    &line[indentation(line).min(indent)..]
}

/// Every fence in a Markdown document, in order, or the line of one that
/// never closes.
fn fences(doc: &str) -> Result<Vec<Fence<'_>>, (usize, String)> {
    let mut found = Vec::new();
    let mut open: Option<(Fence<'_>, &str, usize, usize)> = None;
    for (index, line) in doc.lines().enumerate() {
        match open.as_mut() {
            None => {
                let (depth, rest) = unquote(line, usize::MAX);
                if let Some((marker, info)) = fence_marker(rest) {
                    let language = info.split_whitespace().next().unwrap_or("").to_string();
                    let lines = Vec::new();
                    let opened = index + 1;
                    let indent = indentation(rest);
                    open = Some((
                        Fence {
                            opened,
                            language,
                            lines,
                        },
                        marker,
                        depth,
                        indent,
                    ));
                }
            }
            Some((fence, marker, depth, indent)) => {
                let rest = unquote(line, *depth).1;
                match fence_marker(rest) {
                    Some((close, "")) if close.starts_with(*marker) => {
                        found.extend(open.take().map(|(fence, ..)| fence));
                    }
                    _ => fence.lines.push((index + 1, outdent(rest, *indent))),
                }
            }
        }
    }
    match open {
        Some((fence, ..)) => Err((fence.opened, "this fence never closes".to_string())),
        None => Ok(found),
    }
}

/// Why a fence that prompts cannot hold an unprompted line that names
/// `brokkr`.
const UNPROMPTED: &str = "a line naming `brokkr` without a `$ ` prompt in a fence \
                          that prompts elsewhere; prompt it, or move output into a text fence";

/// A line that names `brokkr` or a path to it, with any `$ ` prompt taken
/// off, for [`refusal`] to account for every mention. In a fence that
/// prompts, a line without a prompt is output, and one that names either
/// is refused: a forgotten prompt and output that looks like a command
/// are told apart by the author, not guessed.
fn command_of(line: &str, prompted: bool) -> Result<Option<&str>, &'static str> {
    let line = line.trim_start();
    match line.strip_prefix("$ ") {
        _ if executables(line) == 0 => Ok(None),
        Some(command) => Ok(Some(command)),
        None if prompted => Err(UNPROMPTED),
        None => Ok(Some(line)),
    }
}

/// Every `brokkr` command in one fence, continuation lines joined, or
/// the line of one in a fence whose language this reader does not know.
fn commands_in(fence: &Fence<'_>) -> Result<Vec<Fenced>, (usize, String)> {
    let language = fence.language.as_str();
    if NOT_SHELLS.contains(&language) {
        return Ok(Vec::new());
    }
    let prompted = fence
        .lines
        .iter()
        .any(|(_, line)| line.trim_start().starts_with("$ "));
    // A continuation in a console fence, or in one that prompts, carries
    // the shell's `> ` prompt.
    let continued = prompted || language == "console";
    let mut found = Vec::new();
    let mut lines = fence.lines.iter();
    while let Some(&(number, line)) = lines.next() {
        // A line is judged whole, its continuations joined as the shell
        // joins them, the backslash and the newline taken off and nothing
        // else, so a `brokkr` on one is read with the command it continues.
        let mut text = line.to_string();
        while text.ends_with('\\') {
            let Some(&(_, next)) = lines.next() else {
                break;
            };
            text.pop();
            text.push_str(
                next.strip_prefix("> ")
                    .filter(|_| continued)
                    .unwrap_or(next),
            );
        }
        let Some(command) =
            command_of(&text, prompted).map_err(|problem| (number, problem.to_string()))?
        else {
            continue;
        };
        if !SHELLS.contains(&language) {
            return Err((
                number,
                format!(
                    "a `brokkr` line in a `{language}` fence, which this reader \
                     does not know; fence it as sh, console or text"
                ),
            ));
        }
        found.push(Fenced {
            line: number,
            text: command.to_string(),
        });
    }
    Ok(found)
}

/// Every `brokkr` command in a Markdown document's fences, or the first
/// thing the reader refuses, with its one-based line.
fn fenced_commands(doc: &str) -> Result<Vec<Fenced>, (usize, String)> {
    let mut found = Vec::new();
    for fence in fences(doc)? {
        found.extend(commands_in(&fence)?);
    }
    Ok(found)
}

/// A command line's commands, each split into words the way a shell
/// would: a pipe or list operator ends one command and starts the next, a
/// redirect and its target are dropped, and a comment ends the line.
/// `<placeholder>` is one word. A construct this reader does not know is
/// refused.
fn words(line: &str) -> Result<Vec<Vec<String>>, String> {
    let mut commands = vec![Vec::new()];
    let mut word: Option<String> = None;
    let mut chars = line.chars().peekable();
    while let Some(character) = chars.next() {
        match character {
            c if c.is_whitespace() => last(&mut commands).extend(word.take()),
            '#' if word.is_none() => break,
            '|' | '&' | ';' => {
                last(&mut commands).extend(word.take());
                while chars.next_if(|c| matches!(c, '|' | '&')).is_some() {}
                commands.push(Vec::new());
            }
            '>' if word
                .as_deref()
                .is_none_or(|w| w.chars().all(|c| c.is_ascii_digit())) =>
            {
                word = None;
                redirect(&mut chars)?;
            }
            '<' if word.is_none() && chars.peek() == Some(&'(') => {
                return Err(PROCESS_SUBSTITUTION.to_string());
            }
            '<' if word.is_none() => word = Some(placeholder(&mut chars)?),
            '\'' | '"' => quoted(character, &mut chars, word.get_or_insert_with(String::new))?,
            '`' => return Err("a backtick substitution".to_string()),
            '$' => dollar(&mut chars, &mut word)?,
            '\\' => match chars.next() {
                Some(c) => word.get_or_insert_with(String::new).push(c),
                None => return Err("a trailing backslash".to_string()),
            },
            c => word.get_or_insert_with(String::new).push(c),
        }
    }
    last(&mut commands).extend(word);
    commands.retain(|command| !command.is_empty());
    Ok(commands)
}

/// The command being split, the last one begun.
fn last(commands: &mut [Vec<String>]) -> &mut Vec<String> {
    commands.last_mut().expect("a line starts with one command")
}

/// Why a `<(` or `>(` is refused: the command inside it runs, and the
/// words it is split into are not the ones it runs.
const PROCESS_SUBSTITUTION: &str = "a process substitution";

/// A redirect's operator and target after its `>`: `>>`, `>&` and the
/// word it writes to, which is no argument of the command. A target that
/// opens a `(`, `>(` or `> >(`, runs a command, and is refused.
fn redirect(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<(), String> {
    while chars.next_if(|c| matches!(c, '>' | '&')).is_some() {}
    while chars.next_if(|c| c.is_whitespace()).is_some() {}
    let mut target = String::new();
    while let Some(c) = chars.next_if(|c| !c.is_whitespace() && !matches!(c, '|' | '&' | ';')) {
        target.push(c);
    }
    match target.as_str() {
        "" => Err("a redirect with no target".to_string()),
        _ if target.contains('(') => Err(PROCESS_SUBSTITUTION.to_string()),
        _ => Ok(()),
    }
}

/// An unquoted `$`: a lone one is a literal, and an expansion is refused,
/// because the words it becomes are the shell's to decide, not the page's.
fn dollar(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    word: &mut Option<String>,
) -> Result<(), String> {
    match chars.peek() {
        Some('(') => Err("a $( substitution".to_string()),
        Some(c) if !c.is_whitespace() => Err("an unquoted $ expansion".to_string()),
        _ => {
            word.get_or_insert_with(String::new).push('$');
            Ok(())
        }
    }
}

/// A `<placeholder>` after its `<`, as one word.
fn placeholder(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<String, String> {
    let mut placeholder = String::from('<');
    loop {
        match chars.next() {
            Some('>') => break,
            Some(c) => placeholder.push(c),
            None => return Err("a `<` that no `>` closes".to_string()),
        }
    }
    placeholder.push('>');
    Ok(placeholder)
}

/// A quoted run after its opening `quote`, copied into `text` up to the
/// quote that closes it.
fn quoted(
    quote: char,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    text: &mut String,
) -> Result<(), String> {
    loop {
        match chars.next() {
            Some(c) if c == quote => return Ok(()),
            // Quoted, a `$(...)` is part of one word whatever it prints;
            // its text stands in for its output, and a `brokkr` in it
            // is [`stray`], never parsed.
            Some('$') if quote == '"' && chars.peek() == Some(&'(') => {
                substitution(chars, text)?;
            }
            Some('`') if quote == '"' => return Err("a backtick substitution".to_string()),
            Some(c) => text.push(c),
            None => return Err(format!("a {quote} quote that never closes")),
        }
    }
}

/// A `$(...)` inside double quotes, from its `(` to the `)` that closes
/// it, copied into the word it stands in.
fn substitution(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    text: &mut String,
) -> Result<(), String> {
    text.push('$');
    let mut depth = 0usize;
    for c in chars.by_ref() {
        text.push(c);
        match c {
            '(' => depth += 1,
            ')' if depth == 1 => return Ok(()),
            ')' => depth -= 1,
            _ => {}
        }
    }
    Err("a $( substitution that never closes".to_string())
}

/// Why a documented line does not parse, or nothing when every `brokkr`
/// on it is the program word of a command clap parses: after the split,
/// a `brokkr` in any other word, or one the split drops, is refused.
fn refusal(line: &str) -> Option<String> {
    let commands = match words(line) {
        Ok(commands) => commands,
        Err(problem) => return Some(problem),
    };
    let parsed: Vec<Vec<String>> = commands.iter().filter_map(|argv| program(argv)).collect();
    if let Some(problem) = parsed.iter().find_map(|argv| clap_refusal(argv)) {
        return Some(problem);
    }
    let split: usize = commands.iter().flatten().map(|word| mentions(word)).sum();
    match stray(&commands) {
        Some(word) => Some(unparsed(word)),
        None if mentions(line) > split => Some(DROPPED.to_string()),
        None => None,
    }
}

/// Why clap refuses one `brokkr` command, or nothing when it parses.
/// `--help` and `--version` stop clap's parse but are valid lines.
fn clap_refusal(argv: &[String]) -> Option<String> {
    match Cli::try_parse_from(argv) {
        Ok(_) => None,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            None
        }
        Err(error) => Some(
            error
                .render()
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_string(),
        ),
    }
}

/// Every refusal in one document, as `file:line: problem` lines.
fn refusals_in(file: &str, doc: &str) -> Vec<String> {
    match fenced_commands(doc) {
        Err((line, problem)) => vec![format!("{file}:{line}: {problem}")],
        Ok(commands) => commands
            .into_iter()
            .filter_map(|fenced| {
                refusal(&fenced.text)
                    .map(|problem| format!("{file}:{}: `{}`: {problem}", fenced.line, fenced.text))
            })
            .collect(),
    }
}

/// The living docs: every tracked Markdown file outside
/// [`records::RECORDS`], whose words are not held to today's command line.
fn living_docs(root: &Path) -> Vec<String> {
    tracked::tracked(root, &["*.md"])
        .into_iter()
        .filter(|path| {
            !records::RECORDS
                .iter()
                .any(|(record, _)| path.starts_with(record))
        })
        .collect()
}

#[test]
fn every_fenced_brokkr_command_in_a_living_doc_parses() {
    let root = workspace();
    let docs = living_docs(&root);
    assert!(docs.iter().any(|doc| doc == "docs/guides/quickstart.md"));
    assert!(!docs.iter().any(|doc| doc.starts_with("docs/decisions/")));
    let refused: Vec<String> = docs
        .iter()
        .flat_map(|doc| {
            let text = std::fs::read_to_string(root.join(doc))
                .unwrap_or_else(|error| panic!("{doc}: {error}"));
            refusals_in(doc, &text)
        })
        .collect();
    assert!(
        refused.is_empty(),
        "documented commands that do not parse:\n{}",
        refused.join("\n")
    );
}

#[test]
fn the_fence_reader_joins_continuations_and_reads_only_prompted_lines_beside_a_prompt() {
    let doc = "```sh\nbrokkr inspect \\\n  --run latest # the newest\n```\n\
               ```console\n$ brokkr runs\nRUN  STATUS  # output\n```\n";
    assert_eq!(
        fenced_commands(doc).unwrap(),
        [
            Fenced {
                line: 2,
                text: "brokkr inspect   --run latest # the newest".to_string(),
            },
            Fenced {
                line: 6,
                text: "brokkr runs".to_string(),
            },
        ]
    );
    assert_eq!(refusals_in("doc.md", doc), Vec::<String>::new());
    assert_eq!(
        refusals_in(
            "doc.md",
            "```console\n$ brokkr runs\nbrokkr wacth --run latest\n```\n"
        ),
        [format!("doc.md:3: {UNPROMPTED}")]
    );
    // The shell takes off the backslash and the newline and nothing else,
    // so a word broken across lines is one word, in a fence indented in a
    // list item as out of one.
    let wacth = "error: unrecognized subcommand 'wacth'";
    assert_eq!(
        refusals_in(
            "doc.md",
            "```sh\nbrok\\\nkr wacth\n```\n  ```sh\n  brokkr wa\\\n  cth\n  ```\n"
        ),
        [
            format!("doc.md:2: `brokkr wacth`: {wacth}"),
            format!("doc.md:6: `brokkr wacth`: {wacth}"),
        ]
    );
}

#[test]
fn the_fence_reader_reads_unprompted_console_blockquotes_and_continuation_prompts() {
    let doc = "```console\nbrokkr doctor\n```\n\
               > ```\n> brokkr run --bundle . \\\n>   --feature x\n> ```\n\
               ```console\n$ brokkr run \\\n> --bogus\n```\n";
    assert_eq!(
        fenced_commands(doc).unwrap(),
        [
            Fenced {
                line: 2,
                text: "brokkr doctor".to_string(),
            },
            Fenced {
                line: 5,
                text: "brokkr run --bundle .   --feature x".to_string(),
            },
            Fenced {
                line: 9,
                text: "brokkr run --bogus".to_string(),
            },
        ]
    );
    assert_eq!(
        refusals_in("doc.md", "```console\nbrokkr runs --bogus\n```\n"),
        ["doc.md:2: `brokkr runs --bogus`: error: unexpected argument '--bogus' found"]
    );
    // A fence with no language that prompts carries the same `> `.
    assert_eq!(
        refusals_in("doc.md", "```\n$ brokkr runs \\\n> --bogus\n```\n"),
        ["doc.md:2: `brokkr runs --bogus`: error: unexpected argument '--bogus' found"]
    );
}

#[test]
fn the_fence_reader_refuses_what_it_cannot_read() {
    assert_eq!(fenced_commands("```text\nbrokkr runs\n```\n").unwrap(), []);
    assert_eq!(
        fenced_commands("```fish\nbrokkr runs\n```\n").unwrap_err(),
        (
            2,
            "a `brokkr` line in a `fish` fence, which this reader does not know; \
             fence it as sh, console or text"
                .to_string()
        )
    );
    assert_eq!(
        fenced_commands("prose\n```sh\nbrokkr runs\n").unwrap_err(),
        (2, "this fence never closes".to_string())
    );
}

#[test]
fn the_fence_reader_sees_brokkr_past_tabs_assignments_and_wrappers() {
    let wacth = "error: unrecognized subcommand 'wacth'";
    // `cargo run -p brokkr-cli -- runs` runs `brokkr runs`, which parses;
    // `cargo install brokkr` names `brokkr` where nothing parses it.
    assert_eq!(
        refusals_in(
            "doc.md",
            "```sh\nFOO=x brokkr wacth\nbrokkr\twacth\ncargo run -p brokkr-cli -- runs\n\
             cargo install brokkr\n```\n"
        ),
        [
            format!("doc.md:2: `FOO=x brokkr wacth`: {wacth}"),
            format!("doc.md:3: `brokkr\twacth`: {wacth}"),
            format!("doc.md:5: `cargo install brokkr`: {}", unparsed("brokkr")),
        ]
    );
    let behind = unparsed("brokkr");
    assert_eq!(
        refusals_in(
            "doc.md",
            "```sh\nenv FOO=x brokkr runs\nsudo dnf install brokkr\nsudo -u me brokkr runs\n```\n"
        ),
        [
            format!("doc.md:2: `env FOO=x brokkr runs`: {behind}"),
            format!("doc.md:3: `sudo dnf install brokkr`: {behind}"),
            format!("doc.md:4: `sudo -u me brokkr runs`: {behind}"),
        ]
    );
    assert_eq!(
        fenced_commands("```console\n$ brokkr runs\nsudo brokkr runs\n```\n").unwrap_err(),
        (3, UNPROMPTED.to_string())
    );
    assert_eq!(
        refusal("brokkr inspect --run $RUN"),
        Some("an unquoted $ expansion".to_string())
    );
    assert_eq!(refusal("brokkr inspect --run \"$RUN\""), None);
}

/// A path to `brokkr` as the program word runs it, and `cargo run -p
/// brokkr-cli` runs it with the words after `--`: both are parsed. As an
/// argument, the path names a file.
#[test]
fn the_fence_reader_parses_a_path_to_brokkr_and_what_cargo_run_runs() {
    let wacth = "error: unrecognized subcommand 'wacth'";
    let doc = "```sh\n./brokkr wacth\n~/.cargo/bin/brokkr wacth\n\
               brokkr runs && target/release/brokkr wacth\ncd ./brokkr && ./brokkr runs\n\
               cargo run -p brokkr-cli -- wacth\ncargo run --locked -p brokkr-cli -- runs\n\
               cargo run --package=brokkr-cli\ncargo run -p brokkr-cli -- runs # brokkr\n```\n";
    assert_eq!(
        refusals_in("doc.md", doc),
        [
            format!("doc.md:2: `./brokkr wacth`: {wacth}"),
            format!("doc.md:3: `~/.cargo/bin/brokkr wacth`: {wacth}"),
            format!("doc.md:4: `brokkr runs && target/release/brokkr wacth`: {wacth}"),
            format!("doc.md:6: `cargo run -p brokkr-cli -- wacth`: {wacth}"),
            // With no `--`, clap is handed a bare `brokkr` and refuses it
            // with the help it prints, whose first line is the about.
            "doc.md:8: `cargo run --package=brokkr-cli`: Deterministic delivery engine".to_string(),
            format!("doc.md:9: `cargo run -p brokkr-cli -- runs # brokkr`: {DROPPED}"),
        ]
    );
    assert_eq!(
        fenced_commands("```console\n$ brokkr runs\n./brokkr wacth\n```\n").unwrap_err(),
        (3, UNPROMPTED.to_string())
    );
}

#[test]
fn the_fence_reader_reads_brokkr_wherever_the_word_splitter_finds_it() {
    let wacth = "error: unrecognized subcommand 'wacth'";
    let doc = "```sh\nif brokkr wacth; then :; fi\nfor x in a; do brokkr wacth; done\n\
               'brokkr' wacth\nRUN=$(brokkr costs --run latest)\nsudo \\\n  brokkr runs\n\
               cd ./brokkr && git commit -m \"brokkr starter\"\n```\n";
    assert_eq!(
        refusals_in("doc.md", doc),
        [
            format!("doc.md:2: `if brokkr wacth; then :; fi`: {wacth}"),
            format!("doc.md:3: `for x in a; do brokkr wacth; done`: {wacth}"),
            format!("doc.md:4: `'brokkr' wacth`: {wacth}"),
            "doc.md:5: `RUN=$(brokkr costs --run latest)`: a $( substitution".to_string(),
            format!("doc.md:6: `sudo   brokkr runs`: {}", unparsed("brokkr")),
            format!(
                "doc.md:8: `cd ./brokkr && git commit -m \"brokkr starter\"`: {}",
                unparsed("brokkr starter")
            ),
        ]
    );
}

/// Beside a prompt, a line that names `brokkr` and does not split is not
/// taken for output, and a process substitution is refused, not read as
/// a redirect's target.
#[test]
fn the_fence_reader_refuses_an_unsplittable_brokkr_line_and_a_process_substitution() {
    for line in ["brokkr inspect --run $RUN", "brokkr inspect --run `cat id`"] {
        assert_eq!(
            fenced_commands(&format!("```console\n$ brokkr runs\n{line}\n```\n")).unwrap_err(),
            (3, UNPROMPTED.to_string()),
            "{line}"
        );
    }
    let doc = "```sh\ncat >(brokkr wacth)\ntee >(brokkr wacth)\n\
               brokkr runs > >(brokkr wacth)\ndiff <(brokkr runs) runs.txt\n```\n";
    assert_eq!(
        refusals_in("doc.md", doc),
        [
            format!("doc.md:2: `cat >(brokkr wacth)`: {PROCESS_SUBSTITUTION}"),
            format!("doc.md:3: `tee >(brokkr wacth)`: {PROCESS_SUBSTITUTION}"),
            format!("doc.md:4: `brokkr runs > >(brokkr wacth)`: {PROCESS_SUBSTITUTION}"),
            format!("doc.md:5: `diff <(brokkr runs) runs.txt`: {PROCESS_SUBSTITUTION}"),
        ]
    );
}

/// The operator's 2026-09-26 ruling: after the split, a `brokkr` that is
/// not the program word of a command clap parsed is refused wherever it
/// stands, and beside a prompt an unprompted line that names it is too.
#[test]
fn the_fence_reader_refuses_every_brokkr_it_did_not_parse() {
    let doc = "```sh\n\
               brokkr run --bundle . --feature \"$(cat $(brokkr wacth))\"\n\
               echo 'brokkr wacth'\n\
               bash -c \"brokkr wacth\"\n\
               xargs brokkr wacth\n\
               \"FOO=x brokkr wacth\"\n\
               cat <<'EOF'\n\
               started by brokkr wacth\n\
               EOF\n\
               brokkr runs # then brokkr wacth\n\
               brokkr runs 2> brokkr\n\
               xargs bro'kkr' wacth\n\
               ```\n";
    let brokkr = unparsed("brokkr");
    assert_eq!(
        brokkr,
        "an unparsed `brokkr` in `brokkr`: only a command that begins with \
         `brokkr` is parsed, and every other mention is refused"
    );
    let wacth = unparsed("brokkr wacth");
    assert_eq!(
        refusals_in("doc.md", doc),
        [
            format!(
                "doc.md:2: `brokkr run --bundle . --feature \"$(cat $(brokkr wacth))\"`: {}",
                unparsed("$(cat $(brokkr wacth))")
            ),
            format!("doc.md:3: `echo 'brokkr wacth'`: {wacth}"),
            format!("doc.md:4: `bash -c \"brokkr wacth\"`: {wacth}"),
            format!("doc.md:5: `xargs brokkr wacth`: {brokkr}"),
            format!(
                "doc.md:6: `\"FOO=x brokkr wacth\"`: {}",
                unparsed("FOO=x brokkr wacth")
            ),
            format!("doc.md:8: `started by brokkr wacth`: {brokkr}"),
            format!("doc.md:10: `brokkr runs # then brokkr wacth`: {DROPPED}"),
            format!("doc.md:11: `brokkr runs 2> brokkr`: {DROPPED}"),
            format!("doc.md:12: `xargs bro'kkr' wacth`: {brokkr}"),
        ]
    );
    for line in [
        "bash -c \"brokkr wacth\"",
        "xargs brokkr wacth",
        "realm    brokkr  .  main",
    ] {
        assert_eq!(
            fenced_commands(&format!("```console\n$ brokkr runs\n{line}\n```\n")).unwrap_err(),
            (3, UNPROMPTED.to_string()),
            "{line}"
        );
    }
}

#[test]
fn the_word_splitter_reads_placeholders_quotes_and_the_end_of_a_command() {
    let split = |command: &str| words(command).unwrap();
    assert_eq!(
        split("brokkr operator --run <run id> stop --reason 'a b' | jq ."),
        [
            vec!["brokkr", "operator", "--run", "<run id>", "stop", "--reason", "a b"],
            vec!["jq", "."]
        ]
    );
    assert_eq!(
        split("brokkr runs --json > runs.json && brokkr seats --json 2>&1 # both"),
        [["brokkr", "runs", "--json"], ["brokkr", "seats", "--json"]]
    );
    assert_eq!(
        split("brokkr runs 2>/dev/null; brokkr doctor || echo \"no\""),
        [
            vec!["brokkr", "runs"],
            vec!["brokkr", "doctor"],
            vec!["echo", "no"]
        ]
    );
    assert_eq!(split("brokkr a\\ b"), [["brokkr", "a b"]]);
    for (command, problem) in [
        ("brokkr inspect --run $(cat id)", "a $( substitution"),
        ("brokkr inspect --run `cat id`", "a backtick substitution"),
        (
            "brokkr inspect --run \"`cat id`\"",
            "a backtick substitution",
        ),
        (
            "brokkr inspect --run \"$(cat id\"",
            "a $( substitution that never closes",
        ),
        ("brokkr inspect --run 'open", "a ' quote that never closes"),
        ("brokkr inspect < file", "a `<` that no `>` closes"),
        ("brokkr inspect \\", "a trailing backslash"),
        ("brokkr runs >", "a redirect with no target"),
    ] {
        assert_eq!(words(command).unwrap_err(), problem, "{command}");
    }
}

#[test]
fn a_documented_command_that_clap_refuses_is_named() {
    assert_eq!(refusal("brokkr runs --json"), None);
    assert_eq!(refusal("brokkr run --help"), None);
    assert_eq!(refusal("brokkr --version"), None);
    assert_eq!(
        refusal("brokkr operator --action supersede --run r --reason why"),
        Some("error: unexpected argument '--action' found".to_string())
    );
    assert_eq!(
        refusal("brokkr inspect --run $(x)"),
        Some("a $( substitution".to_string())
    );
    assert_eq!(
        refusals_in("doc.md", "```sh\nbrokkr watch --json --run latest\n```\n"),
        ["doc.md:2: `brokkr watch --json --run latest`: error: unexpected argument '--json' found"]
    );
    assert_eq!(
        refusal("cd repo && brokkr runs && brokkr inspect --bogus"),
        Some("error: unexpected argument '--bogus' found".to_string())
    );
    assert_eq!(
        refusals_in("doc.md", "```sh\ncd repo && brokkr inspect --bogus\n```\n"),
        ["doc.md:2: `cd repo && brokkr inspect --bogus`: error: unexpected argument '--bogus' found"]
    );
}
