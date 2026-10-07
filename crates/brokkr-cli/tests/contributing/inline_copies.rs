//! The copies a linked section's prose writes inline (#450): each span
//! that reads as a command is a command its row's leg runs, or one a
//! table here holds with its reason, and a span one edit from a line its
//! row accepts is the drifted copy it reads as.

use std::path::Path;

use super::{links, CheckRow, Line, Link, LEG_LINES};

/// Every inline code span in a linked section's prose, as a
/// whitespace-collapsed string, whether or not it reads as a command:
/// `inline_commands` keeps the ones that do, and `assert_inline_copies`
/// compares every span against the lines its row accepts, so a copy that
/// has drifted past reading as a command — the `carg` a mistyped copy of
/// `cargo` writes — is still seen. A span left open fails the test.
fn inline_spans(check: &str, link: &Link) -> Vec<String> {
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
        .map(collapsed)
        .collect()
}

/// `text` as one whitespace-collapsed string, the shape a span and the
/// line it copies are compared in.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The spans `inline_spans` reads that read as a command: a span whose
/// first word, past its `NAME=value` prefixes, is one `FIRST_WORDS`
/// lists, alone or with arguments, or whose first word, listed or not,
/// is passed a flag or a path, which only a command takes. So a copy of
/// a one-word line is compared, a one-word span one edit from a line
/// `LEG_LINES` holds is compared as the drifted copy it is,
/// and a mistyped first word beside a flag or a path fails rather than
/// going unread. Any other span is a name, a path, a flag or output, and
/// does not read as a command.
fn inline_commands(check: &str, link: &Link) -> Vec<String> {
    inline_spans(check, link)
        .into_iter()
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
        [program] => first_word(program) || held_one_word(program),
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
/// runs cargo through it. `scripts/run-it-tests.sh` is listed too: the
/// workflows run their one-binary test commands through the checked entry
/// point that fails a run executing 0 tests (#543).
const FIRST_WORDS: &str = "*,by-hand,*) bash cargo case docker echo esac exit fi git grep if npm \
                           quality/ratchet.sh rustup scripts/run-it-tests.sh set sw_vers test uname }";

/// Whether `FIRST_WORDS` lists `word`.
fn first_word(word: &str) -> bool {
    FIRST_WORDS.split(' ').any(|listed| listed == word)
}

/// Whether a one-word span's word is a one-word line a checked leg holds
/// word for word in `LEG_LINES`, or one edit from one: `sw_vers`, `esac`,
/// `fi`, `*,by-hand,*)` and `}`, read from `LEG_LINES` itself, so the
/// fact has one home and there is no second list to go stale — a line a
/// leg starts holding is compared from the moment `LEG_LINES` holds it,
/// and an entry no leg holds any more is gone rather than reading spans
/// no line backs. `FIRST_WORDS` compares such a span when it is the line
/// word for word; a span one edit from a line held here — the `sw_verss`
/// a mistyped copy of `sw_vers` writes, or its transposition `sw_vesr` —
/// is compared as the drifted copy it is, rather than going unread.
fn held_one_word(word: &str) -> bool {
    LEG_LINES
        .iter()
        .flat_map(|(check, _)| unwritten_lines(check))
        .filter(|line| command_words(line).len() == 1)
        .any(|held| within_one_edit(held, word))
}

/// Whether `a` and `b` are the same word up to one edit — an insertion, a
/// deletion, a substitution or an adjacent transposition — so `sw_verss`
/// is the drifted copy of `sw_vers` it reads as and `sw_vesr` its
/// transposition, while `tee`, `rustfmt` and `ok` are no copy of anything
/// held and stay names.
fn within_one_edit(a: &str, b: &str) -> bool {
    one_insertion_deletion_or_substitution(a, b) || one_transposition(a, b)
}

/// Whether `b` is `a` up to one insertion, deletion or substitution.
fn one_insertion_deletion_or_substitution(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    if long.len() - short.len() > 1 {
        return false;
    }
    let mut edits = 0;
    let (mut i, mut j) = (0, 0);
    while i < short.len() && j < long.len() {
        if short[i] == long[j] {
            i += 1;
            j += 1;
        } else {
            edits += 1;
            if edits > 1 {
                return false;
            }
            if short.len() < long.len() {
                j += 1;
            } else {
                i += 1;
                j += 1;
            }
        }
    }
    edits + (long.len() - j) + (short.len() - i) <= 1
}

/// Whether `b` is `a` with one adjacent pair of characters swapped — the
/// `sw_vesr` `sw_vers` transposes to, which is no insertion, deletion or
/// substitution: the two differ in exactly two neighbouring positions,
/// each holding the other's character. Identical words are no
/// transposition; `one_insertion_deletion_or_substitution` already holds
/// them.
fn one_transposition(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len() != b.len() {
        return false;
    }
    let Some(i) = a.iter().zip(&b).position(|(x, y)| x != y) else {
        return false;
    };
    i + 1 < a.len() && a[i] == b[i + 1] && a[i + 1] == b[i] && a[i + 2..] == b[i + 2..]
}

/// The line `accepted` holds that `span` is one edit from, when `span`
/// is not itself a line `accepted` holds: the copy has drifted from a
/// command the row accepts, and is refused even where it no longer reads
/// as a command — the `carg deny check licenses bans sources` a mistyped
/// copy of the run line writes, or the `sw_ vers` of the held
/// `sw_vers`. A line the row accepts is no drifted copy, however else it
/// sits, and a span no edit from any accepted line is no drift.
fn drifted_from<'a>(span: &str, accepted: &'a [String]) -> Option<&'a str> {
    let lines: Vec<&str> = accepted.iter().map(String::as_str).collect();
    if lines.contains(&span) {
        return None;
    }
    lines.into_iter().find(|line| within_one_edit(line, span))
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
/// anchor: an entry no span writes is stale, and
/// `assert_inline_only_used` refuses it.
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
/// drifts from its command fails here. And every span is compared
/// against the lines the row accepts, read or not: a span one edit from
/// one of them is the drifted copy it reads as — the `carg deny check
/// licenses bans sources` a mistyped copy of the run line writes no
/// longer reads as a command, and does not therefore go unread.
pub(super) fn assert_inline_copies(root: &Path, row: &CheckRow, guide: &str, runs: &[String]) {
    let held = unwritten_lines(&row.check);
    for link in links(&row.check, guide, &row.command) {
        let script = SECTION_SCRIPTS
            .iter()
            .filter(|(anchor, _)| *anchor == link.anchor)
            .flat_map(|(_, script)| script_commands(root, script))
            .collect::<Vec<_>>();
        let accepted: Vec<String> = runs
            .iter()
            .map(|line| collapsed(line))
            .chain(held.iter().map(|line| collapsed(line)))
            .chain(script.iter().map(|line| collapsed(line)))
            .chain(
                INLINE_ONLY
                    .iter()
                    .filter(|(anchor, _)| *anchor == link.anchor)
                    .map(|(_, command)| collapsed(command)),
            )
            .collect();
        let spans = inline_spans(&row.check, &link);
        for span in &spans {
            if let Some(line) = drifted_from(span, &accepted) {
                panic!(
                    "{}'s row links #{}, whose prose writes `{span}`, one edit from `{line}`, which its row accepts: the copy has drifted",
                    row.check, link.anchor
                );
            }
        }
        for command in spans.iter().filter(|span| text_reads_as_command(span)) {
            assert!(
                runs.contains(command)
                    || held.contains(&command.as_str())
                    || script.contains(command)
                    || INLINE_ONLY.contains(&(link.anchor, command.as_str())),
                "{}'s row links #{}, whose prose writes `{command}`, which neither its leg nor the section's script runs",
                row.check,
                link.anchor
            );
        }
    }
}

/// The entries of `allowances` no span writes, as (anchor, command) pairs
/// in `allowances`'s order: an entry no span uses is stale — it allows
/// nothing and hides the copy it was written for.
fn unused_allowances<'a>(
    allowances: &[(&'a str, &'a str)],
    used: &[(String, String)],
) -> Vec<(&'a str, &'a str)> {
    allowances
        .iter()
        .copied()
        .filter(|(anchor, command)| {
            !used.contains(&((*anchor).to_string(), (*command).to_string()))
        })
        .collect()
}

/// Every allowance `INLINE_ONLY` holds is a span some row's linked
/// section writes, as an (anchor, command) pair: an entry no span uses is
/// stale — it allows nothing and hides the copy it was written for — and
/// fails here, as a stale `LOCAL_ONLY` line does.
pub(super) fn assert_inline_only_used(rows: &[CheckRow], guide: &str) {
    let mut used: Vec<(String, String)> = Vec::new();
    for row in rows {
        for link in links(&row.check, guide, &row.command) {
            for command in inline_commands(&row.check, &link) {
                used.push((link.anchor.to_string(), command));
            }
        }
    }
    if let Some(&(anchor, command)) = unused_allowances(&INLINE_ONLY, &used).first() {
        panic!("no span writes `{command}` at #{anchor}, which INLINE_ONLY holds");
    }
}

/// The one-word arm pinned by exact values: a word one edit from a line
/// `LEG_LINES` holds — the insertion `sw_verss`, the deletion
/// `sw_ver`, the substitution `sw_vars`, the transposition `sw_vesr` —
/// reads as a command, and `tee`, `rustfmt` and `ok` are no copy of
/// anything held and stay names. A change that stops reading a drifted
/// copy, or reads a name as one, fails here rather than leaving a copy
/// in the guide unread.
#[test]
fn a_drifted_copy_of_a_held_line_reads_as_a_command() {
    assert!(text_reads_as_command("sw_verss"));
    assert!(text_reads_as_command("sw_ver"));
    assert!(text_reads_as_command("sw_vars"));
    assert!(text_reads_as_command("sw_vesr"));
    assert!(!text_reads_as_command("tee"));
    assert!(!text_reads_as_command("rustfmt"));
    assert!(!text_reads_as_command("ok"));
}

/// The drift arm pinned by exact values: a span one edit from a line its
/// row accepts is the drifted copy it reads as, read or not — the
/// mistyped `carg` and the transposed `crago` of the run line `cargo
/// deny check licenses bans sources`, and the space-swallowed `sw_ vers`
/// of the held `sw_vers` — while the line itself, and `tee` and
/// `rustfmt`, no edit from anything accepted, are no drift. A change
/// that stops reading a drifted copy fails here rather than leaving a
/// copy in the guide unread.
#[test]
fn a_span_one_edit_from_an_accepted_line_is_its_drifted_copy() {
    let accepted = vec![
        "cargo deny check licenses bans sources".to_string(),
        "sw_vers".to_string(),
        "sw_verss".to_string(),
    ];
    assert_eq!(
        drifted_from("carg deny check licenses bans sources", &accepted),
        Some("cargo deny check licenses bans sources")
    );
    assert_eq!(
        drifted_from("crago deny check licenses bans sources", &accepted),
        Some("cargo deny check licenses bans sources")
    );
    assert_eq!(drifted_from("sw_ vers", &accepted), Some("sw_vers"));
    assert_eq!(
        drifted_from("cargo deny check licenses bans sources", &accepted),
        None
    );
    // The line itself is no drift, however else it sits: `sw_vers` is a
    // line `accepted` holds, though `sw_verss` held beside it is one edit
    // from it.
    assert_eq!(drifted_from("sw_vers", &accepted), None);
    assert_eq!(drifted_from("tee", &accepted), None);
    assert_eq!(drifted_from("rustfmt", &accepted), None);
}

/// `unused_allowances` pinned by exact values: the entries no span
/// writes are the ones returned, in `allowances`'s order — an allowance
/// written at one anchor is no allowance at another — while every entry
/// a span writes is not; with nothing used every entry is stale, and
/// with no allowances none is. A check that stops seeing a stale entry —
/// its predicate held true, or its walk dropped — fails here rather than
/// allowing a copy nothing writes.
#[test]
fn an_inline_only_allowance_no_span_writes_is_stale() {
    let used = vec![
        (
            "the-workspace-suite".to_string(),
            "test result: ok".to_string(),
        ),
        ("formatting".to_string(), "cargo fmt --all".to_string()),
    ];
    let allowances: [(&str, &str); 5] = [
        ("the-workspace-suite", "test result: ok"),
        ("the-macos-startup-gate", "test result: ok"),
        ("formatting", "cargo bogus --never-written"),
        ("formatting", "cargo fmt --all"),
        ("the-release-binary", "cargo test"),
    ];
    assert_eq!(
        unused_allowances(&allowances, &used),
        [
            ("the-macos-startup-gate", "test result: ok"),
            ("formatting", "cargo bogus --never-written"),
            ("the-release-binary", "cargo test"),
        ]
    );
    assert_eq!(unused_allowances(&allowances, &[]), allowances);
    assert_eq!(unused_allowances(&[], &used), []);
}

/// The stale-allowance refusal itself, bound: with no rows at all no
/// span is read, so every allowance `INLINE_ONLY` holds is stale and the
/// check refuses its first. A refusal compiled away — its predicate held
/// true, or its walk dropped — leaves no panic, and fails here.
#[test]
#[should_panic(
    expected = "no span writes `test result: ok` at #the-workspace-suite, which INLINE_ONLY holds"
)]
fn a_stale_inline_only_allowance_fails_the_check() {
    assert_inline_only_used(&[], "");
}

/// The drift refusal's call, bound: a row whose linked section's prose
/// writes `sw_ vers`, one edit from the `sw_vers` its leg runs, fails
/// with the drift named. The span does not even read as a command —
/// `sw_` is no first word, and `vers` takes no flag or path — so a
/// refusal made inert at the call leaves no panic at all, and fails
/// here.
#[test]
#[should_panic(expected = "the copy has drifted")]
fn a_row_whose_prose_drifts_from_an_accepted_line_fails() {
    let row = CheckRow {
        number: "1".to_string(),
        check: "my check".to_string(),
        job: "my-job".to_string(),
        file: "ci.yml",
        command: "see [the gate](#my-check)".to_string(),
    };
    let guide = "# A guide\n\n## My check\n\nCopy it as `sw_ vers` and the row refuses it.\n";
    assert_inline_copies(Path::new("."), &row, guide, &["sw_vers".to_string()]);
}
