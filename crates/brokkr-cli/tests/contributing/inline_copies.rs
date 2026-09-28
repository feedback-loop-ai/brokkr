//! The copies a linked section's prose writes inline (#450): each span that
//! reads as a command is a command its row's leg runs, or one a table here
//! holds with its reason.

use std::path::Path;

use super::{links, CheckRow, Line, Link, LEG_LINES};

/// Every inline code span in a linked section that reads as a command, as
/// a whitespace-collapsed string: a span whose first word, past its
/// `NAME=value` prefixes, is one `FIRST_WORDS` lists, alone or with
/// arguments, or whose first word, listed or not, is passed a flag or a
/// path, which only a command takes. So a copy of a one-word line is
/// compared, and a mistyped first word beside a flag or a path fails
/// rather than going unread. Any other span is a name, a path, a flag or
/// output, and is not compared. A span left open fails the test.
fn inline_commands(check: &str, link: &Link) -> Vec<String> {
    let prose: Vec<&str> = link.section.split("```").step_by(2).collect();
    let prose = prose.concat();
    let spans: Vec<&str> = prose.split('`').collect();
    assert!(
        spans.len() % 2 == 1,
        "{check}'s row links #{}, which leaves a code span open that this test cannot read",
        link.anchor
    );
    spans
        .into_iter()
        .skip(1)
        .step_by(2)
        .map(|span| span.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|span| text_reads_as_command(span))
        .collect()
}

/// Whether `text` reads as a command, as `inline_commands` states.
pub(super) fn text_reads_as_command(text: &str) -> bool {
    reads_as_command(&command_words(text))
}

/// Whether a span's words read as a command, as `inline_commands` states.
fn reads_as_command(words: &[&str]) -> bool {
    match words {
        [] => false,
        [program, ..] if first_word(program) => true,
        [_, arguments @ ..] => arguments
            .iter()
            .any(|word| word.starts_with('-') || word.contains('/')),
    }
}

/// A command's words past its `name=value` prefixes, a double-quoted
/// string one word.
fn command_words(command: &str) -> Vec<&str> {
    let mut quoted = false;
    command
        .split(|c: char| {
            quoted ^= c == '"';
            c == ' ' && !quoted
        })
        .filter(|word| !word.is_empty())
        .skip_while(|word| is_assignment(word))
        .collect()
}

/// Whether `word` is a variable assignment, `name=value`.
fn is_assignment(word: &str) -> bool {
    word.split_once('=').is_some_and(|(name, _)| {
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// The first word, past its variable assignments, of every line a checked
/// leg runs, written or held, and of every command a section's script
/// runs, shell grammar included: `assert_first_words_listed` refuses a
/// line whose first word this does not list. A word stays listed when its
/// leg stops running it, so a prose copy the change orphans still reads as
/// a command and fails as one. `rustup` is listed too: a toolchain override
/// runs cargo through it.
const FIRST_WORDS: &str = "*,by-hand,*) bash cargo case docker echo esac exit fi git grep if npm \
                           quality/ratchet.sh rustup set sw_vers test uname }";

/// Whether `FIRST_WORDS` lists `word`.
fn first_word(word: &str) -> bool {
    FIRST_WORDS.split(' ').any(|listed| listed == word)
}

/// The lines `LEG_LINES` holds word for word for `check`'s leg.
fn unwritten_lines(check: &str) -> Vec<&'static str> {
    LEG_LINES
        .iter()
        .filter(|(listed, _)| *listed == check)
        .flat_map(|(_, lines)| lines.iter())
        .filter_map(|line| match line {
            Line::Unwritten(text) => Some(*text),
            Line::Written => None,
        })
        .collect()
}

/// Every first word a checked leg or a section's script runs is one
/// `FIRST_WORDS` lists, so what reads as a command in prose does not
/// depend on what the legs run today.
pub(super) fn assert_first_words_listed(root: &Path, runs: &[Vec<String>]) {
    let held = LEG_LINES
        .iter()
        .flat_map(|(check, _)| unwritten_lines(check))
        .map(str::to_string);
    let scripts = SECTION_SCRIPTS
        .iter()
        .flat_map(|(_, script)| script_commands(root, script));
    for line in runs.iter().flatten().cloned().chain(held).chain(scripts) {
        if let Some(word) = command_words(&line).first() {
            assert!(
                first_word(word),
                "FIRST_WORDS does not list `{word}`, the first word of `{line}`"
            );
        }
    }
}

/// Spans a linked section's prose writes that read as commands and that
/// its row's leg does not run, each with its reason, by the section's
/// anchor.
const INLINE_ONLY: [(&str, &str); 8] = [
    // A test binary's summary line, quoted as output: "reported `test
    // result: ok` for every one of them".
    ("the-workspace-suite", "test result: ok"),
    // The same output line: "a name that selects nothing still prints
    // `test result: ok`".
    ("the-macos-startup-gate", "test result: ok"),
    // The shell to paste the block into: "start `bash` first".
    ("the-macos-startup-gate", "bash"),
    // The block's checks, named as a tool: "the quiet `grep`s".
    ("the-macos-startup-gate", "grep"),
    // The fix for a refusal, not the check: "The fix is `cargo fmt --all`".
    ("formatting", "cargo fmt --all"),
    // A step the coverage script takes itself, named to say whose report it is.
    ("exact-coverage", "cargo llvm-cov clean --workspace"),
    // The command named, not run: "a `cargo test`-only path".
    ("the-release-binary", "cargo test"),
    // The landing itself, which the section lights and no required check runs.
    (
        "the-landing-let-the-machine-finish-what-you-wrote-by-hand",
        r#"brokkr run --recipe landing --repo . --feature "landing: <what the branch is>""#,
    ),
];

/// Linked sections that describe a script rather than their row's leg:
/// each inline command such a section writes is one the script runs, as
/// its `if ! <command> > "$output"` lines run them.
const SECTION_SCRIPTS: [(&str, &str); 1] = [(
    "the-landing-let-the-machine-finish-what-you-wrote-by-hand",
    "recipes/fast/scripts/verify-seat.sh",
)];

/// The commands `script` runs as `if ! <command> > "$output"`.
fn script_commands(root: &Path, script: &str) -> Vec<String> {
    let text = std::fs::read_to_string(root.join(script)).unwrap();
    let commands: Vec<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix("if ! ")?.split_once(" > \"$output\""))
        .map(|(command, _)| command.to_string())
        .collect();
    assert!(
        !commands.is_empty(),
        "{script} runs no command this test reads"
    );
    commands
}

/// Every inline command a row's linked sections write is a command its
/// leg runs, written or held, one `INLINE_ONLY` holds, or one the script
/// `SECTION_SCRIPTS` names for the section runs: a copy in prose that
/// drifts from its command fails here.
pub(super) fn assert_inline_copies(root: &Path, row: &CheckRow, guide: &str, runs: &[String]) {
    let held = unwritten_lines(&row.check);
    for link in links(&row.check, guide, &row.command) {
        let script = SECTION_SCRIPTS
            .iter()
            .filter(|(anchor, _)| *anchor == link.anchor)
            .flat_map(|(_, script)| script_commands(root, script))
            .collect::<Vec<_>>();
        for command in inline_commands(&row.check, &link) {
            assert!(
                runs.contains(&command)
                    || held.contains(&command.as_str())
                    || script.contains(&command)
                    || INLINE_ONLY.contains(&(link.anchor, command.as_str())),
                "{}'s row links #{}, whose prose writes `{command}`, which neither its leg nor the section's script runs",
                row.check,
                link.anchor
            );
        }
    }
}
