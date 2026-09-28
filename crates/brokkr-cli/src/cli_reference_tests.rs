//! The command line, documented from the one place it is defined (#362).
//!
//! `docs/reference/cli.md` is rendered here from clap's own `Command`
//! tree and the [`Exit`] enum, and a test fails when the committed page
//! differs, printing the command that regenerates it. A second test reads
//! every fenced `brokkr` line in every living doc and parses it with the
//! same [`Cli`], so an example that no longer parses fails CI.
//!
//! The fence reader fails closed. A `brokkr` line is read in a shell
//! fence (`sh`, `bash`, `shell`, `zsh`, or `console`, where a fence that
//! prompts anywhere holds output too and so only its `$ ` lines are
//! commands), in a blockquote as well as out of one, and every `brokkr`
//! command a line joins with `&&`, `;` or a pipe is parsed; one standing
//! in any other fence, one without a prompt in a console fence that
//! prompts, a fence that never closes, and a shell construct the word
//! splitter does not know are each refused rather than skipped.

use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command, CommandFactory, Parser};

use crate::exit::Exit;
use crate::Cli;

#[path = "../tests/support/tracked.rs"]
mod tracked;

/// Where the rendered page lives, from the workspace root.
const REFERENCE: &str = "docs/reference/cli.md";

/// The command that rewrites [`REFERENCE`] from the definitions.
const REGENERATE: &str =
    "BROKKR_REGENERATE_CLI_REFERENCE=1 cargo test -p brokkr-cli --lib cli_reference";

/// The arguments that name a run through decision 0015's selector: a
/// full id, a unique prefix, or `latest`. Each verb carrying one resolves
/// it through `selector::resolve_run`, and `verbs/tests.rs` pins the verbs
/// that came to it last.
const SELECTORS: [&str; 3] = ["run", "run_a", "run_b"];

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
        Exit::Boxed(_) => None,
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

/// One argument's table row.
fn argument_row(arg: &Arg) -> String {
    let default = if arg.get_action().takes_values() {
        arg.get_default_values()
            .iter()
            .map(|value| format!("`{}`", value.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        String::new()
    };
    let selector = if SELECTORS.contains(&arg.get_id().as_str()) {
        "yes"
    } else {
        ""
    };
    let help = arg
        .get_long_help()
        .or(arg.get_help())
        .map(|help| prose(&help.to_string(), true))
        .unwrap_or_default();
    format!("| `{}` | {default} | {selector} | {help} |\n", spelled(arg))
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
    let arguments: Vec<&Arg> = verb
        .get_arguments()
        .filter(|arg| !arg.is_hide_set())
        .filter(|arg| !matches!(arg.get_action(), ArgAction::Help | ArgAction::Version))
        .collect();
    if !arguments.is_empty() {
        out.push_str("| Argument | Default | Selector | Description |\n");
        out.push_str("| --- | --- | --- | --- |\n");
        for arg in arguments {
            out.push_str(&argument_row(arg));
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

/// The whole page.
fn reference() -> String {
    let mut cli = Cli::command();
    cli.build();
    let mut page = format!(
        "# CLI reference\n\n\
         <!-- Rendered from the clap definitions by \
         crates/brokkr-cli/src/cli_reference_tests.rs; do not edit by hand. \
         Regenerate with: {REGENERATE} -->\n\n\
         Every `brokkr` verb and argument with its default, and every exit \
         code, as the binary defines them; `brokkr <verb> --help` prints the \
         same text. An argument marked **selector** takes a full run id, a \
         unique prefix of one, or `latest`, the run created most recently \
         (decision 0015, and for the write paths its proposed 2026-09-28 \
         addendum). Every verb also takes clap's own `-h`/`--help`, \
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
            "boxed"
        ]
    );
    let table = exit_codes();
    assert!(table.contains("| 4 | contended |"), "{table}");
    assert!(table.contains("| its own | boxed |"), "{table}");
}

/// Tracked docs whose words are fixed when they are written, each with
/// the reason it is not held to today's command line.
const RECORDS: [(&str, &str); 7] = [
    (
        "docs/decisions/",
        "a decision's text is fixed when it is ruled",
    ),
    ("docs/releases/", "release notes say what shipped"),
    ("docs/lore/", "lore is the history as it was told"),
    ("docs/essays/", "an essay is dated"),
    ("docs/evidence/", "evidence records work as it happened"),
    (
        "docs/research/",
        "a research entry reads an article as of its date",
    ),
    (
        "openspec/changes/",
        "a change records its proposal and its work",
    ),
];

/// The fences whose lines are commands, a fence with no language among
/// them, as most of the guides write one. A `console` fence that prompts
/// anywhere shows output beside its commands, so there only a line after
/// a `$ ` prompt is one; a `console` fence with no prompt is all commands.
const SHELLS: [&str; 6] = ["", "sh", "bash", "shell", "zsh", "console"];

/// The fences whose lines are never commands: diagrams, output and data.
const NOT_SHELLS: [&str; 7] = [
    "text", "json", "markdown", "mermaid", "diff", "toml", "yaml",
];

/// What starts a second command on a line: a pipe, `&&`, `||`, `&`, `;`.
const JOINS: [&str; 3] = ["|", "&", ";"];

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

/// Every fence in a Markdown document, in order, or the line of one that
/// never closes.
fn fences(doc: &str) -> Result<Vec<Fence<'_>>, (usize, String)> {
    let mut found = Vec::new();
    let mut open: Option<(Fence<'_>, &str, usize)> = None;
    for (index, line) in doc.lines().enumerate() {
        match open.as_mut() {
            None => {
                let (depth, rest) = unquote(line, usize::MAX);
                if let Some((marker, info)) = fence_marker(rest) {
                    let language = info.split_whitespace().next().unwrap_or("").to_string();
                    let lines = Vec::new();
                    let opened = index + 1;
                    open = Some((
                        Fence {
                            opened,
                            language,
                            lines,
                        },
                        marker,
                        depth,
                    ));
                }
            }
            Some((fence, marker, depth)) => {
                let rest = unquote(line, *depth).1;
                match fence_marker(rest) {
                    Some((close, "")) if close.starts_with(*marker) => {
                        found.extend(open.take().map(|(fence, _, _)| fence));
                    }
                    _ => fence.lines.push((index + 1, rest)),
                }
            }
        }
    }
    match open {
        Some((fence, _, _)) => Err((fence.opened, "this fence never closes".to_string())),
        None => Ok(found),
    }
}

/// Why a fence that prompts cannot hold an unprompted `brokkr` line.
const UNPROMPTED: &str = "a `brokkr` line without a `$ ` prompt in a console fence \
                          that prompts elsewhere; prompt it, or move output into a text fence";

/// A line that runs `brokkr`, with any `$ ` prompt taken off: it starts
/// with `brokkr` or joins a `brokkr` command after another. In a fence
/// that prompts, a line without a prompt is output, and one that reads
/// as a `brokkr` command is refused: a forgotten prompt and output that
/// looks like a command are told apart by the author, not guessed.
fn command_of(line: &str, prompted: bool) -> Result<Option<&str>, &'static str> {
    let line = line.trim_start();
    let (line, unprompted) = match line.strip_prefix("$ ") {
        Some(command) => (command, false),
        None => (line, prompted),
    };
    let runs = line == "brokkr"
        || line.starts_with("brokkr ")
        || JOINS
            .iter()
            .any(|join| line.contains(&format!("{join} brokkr")));
    match (runs, unprompted) {
        (false, _) => Ok(None),
        (true, false) => Ok(Some(line)),
        (true, true) => Err(UNPROMPTED),
    }
}

/// Every `brokkr` command in one fence, continuation lines joined, or
/// the line of one in a fence whose language this reader does not know.
fn commands_in(fence: &Fence<'_>) -> Result<Vec<Fenced>, (usize, String)> {
    let language = fence.language.as_str();
    if NOT_SHELLS.contains(&language) {
        return Ok(Vec::new());
    }
    let console = language == "console";
    let prompted = console
        && fence
            .lines
            .iter()
            .any(|(_, line)| line.trim_start().starts_with("$ "));
    let mut found = Vec::new();
    let mut lines = fence.lines.iter();
    while let Some(&(number, line)) = lines.next() {
        let Some(command) =
            command_of(line, prompted).map_err(|problem| (number, problem.to_string()))?
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
        let mut text = command.to_string();
        while text.ends_with('\\') {
            let Some(&(_, next)) = lines.next() else {
                break;
            };
            // A console continuation carries the shell's `> ` prompt.
            let next = next.trim();
            text.pop();
            text.push(' ');
            text.push_str(next.strip_prefix("> ").filter(|_| console).unwrap_or(next));
        }
        found.push(Fenced { line: number, text });
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
            '<' if word.is_none() => word = Some(placeholder(&mut chars)?),
            '\'' | '"' => quoted(character, &mut chars, word.get_or_insert_with(String::new))?,
            '`' => return Err("a backtick substitution".to_string()),
            '$' if chars.peek() == Some(&'(') => return Err("a $( substitution".to_string()),
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

/// A redirect's operator and target after its `>`: `>>`, `>&` and the
/// word it writes to, which is no argument of the command.
fn redirect(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<(), String> {
    while chars.next_if(|c| matches!(c, '>' | '&')).is_some() {}
    while chars.next_if(|c| c.is_whitespace()).is_some() {}
    let mut target = 0;
    while chars
        .next_if(|c| !c.is_whitespace() && !matches!(c, '|' | '&' | ';'))
        .is_some()
    {
        target += 1;
    }
    match target {
        0 => Err("a redirect with no target".to_string()),
        _ => Ok(()),
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
            // its text stands in for its output.
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
/// command on it does.
fn refusal(line: &str) -> Option<String> {
    match words(line) {
        Ok(commands) => commands
            .iter()
            .filter(|argv| argv[0] == "brokkr")
            .find_map(|argv| clap_refusal(argv)),
        Err(problem) => Some(problem),
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

/// The living docs: every tracked Markdown file outside [`RECORDS`].
fn living_docs(root: &Path) -> Vec<String> {
    tracked::tracked(root, &["*.md"])
        .into_iter()
        .filter(|path| !RECORDS.iter().any(|(record, _)| path.starts_with(record)))
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
fn the_fence_reader_joins_continuations_and_reads_prompts_only_in_console() {
    let doc = "```sh\nbrokkr inspect \\\n  --run latest # the newest\n```\n\
               ```console\n$ brokkr runs\nRUN  STATUS  # output\n```\n";
    assert_eq!(
        fenced_commands(doc).unwrap(),
        [
            Fenced {
                line: 2,
                text: "brokkr inspect  --run latest # the newest".to_string(),
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
                text: "brokkr run --bundle .  --feature x".to_string(),
            },
            Fenced {
                line: 9,
                text: "brokkr run  --bogus".to_string(),
            },
        ]
    );
    assert_eq!(
        refusals_in("doc.md", "```console\nbrokkr runs --bogus\n```\n"),
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
    assert_eq!(
        split("brokkr run --feature \"$(cat $(ls a)) <urls>\" --recipe x"),
        [[
            "brokkr",
            "run",
            "--feature",
            "$(cat $(ls a)) <urls>",
            "--recipe",
            "x"
        ]]
    );
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
