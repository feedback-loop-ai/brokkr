//! The house's prose rule, held by a test rather than by memory (issue
//! #339): any list of the agent CLIs a seat can run reads claude, codex or
//! dsh. A list that names claude and codex and leaves dsh out is refused,
//! unless its own paragraph names dsh, which is how a sentence about
//! exactly those two says it considered the third.
//!
//! Every tracked Markdown file is read, except under the records named in
//! [`RECORDS`], whose words are fixed when they are written. Each paragraph
//! and each list item is one unit: lowercased, with its emphasis and code
//! marks dropped and its line breaks joined, so neither a line break nor
//! `**` nor a code span can hide a list. A list joins its names with `,`,
//! `/`, `&`, `+`, `;`, `|`, `and`, `or` or `nor`, and a name may follow its
//! joiner after one other word (`or the Codex CLI`, `claude, gemini or
//! codex`). Adjacent list items and table rows that each lead with a name
//! are one list too. Fenced code is not prose.

use std::path::PathBuf;

/// Paths whose prose is a record, each with the reason it is not rewritten.
const RECORDS: [(&str, &str); 9] = [
    ("contracts/", "frozen contract bodies"),
    ("reference/", "frozen heritage"),
    ("fixtures/", "frozen fixtures"),
    (
        "docs/decisions/",
        "a decision's text is fixed when it is ruled",
    ),
    (
        "docs/essays/",
        "an essay reports what happened, as it happened",
    ),
    (
        "docs/research/",
        "a research entry keeps its source's own list",
    ),
    ("docs/evidence/", "evidence records a past run"),
    ("docs/releases/", "release notes say what shipped"),
    (
        "openspec/",
        "a spec or change states each harness's own behaviour",
    ),
];

const HARNESSES: [&str; 3] = ["claude", "codex", "dsh"];

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// Every tracked Markdown file, as git lists it.
fn tracked_markdown() -> Vec<String> {
    let output = std::process::Command::new("git")
        .current_dir(workspace())
        .args(["ls-files", "-z", "--", "*.md"])
        .output()
        .expect("git ls-files");
    assert!(output.status.success(), "git ls-files failed: {output:?}");
    String::from_utf8(output.stdout)
        .expect("UTF-8 paths")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whether a line opens a new unit: a heading, a list item, a table row or
/// a quote.
fn opens_a_unit(trimmed: &str) -> bool {
    let ordered = trimmed
        .split_once(['.', ')'])
        .is_some_and(|(number, rest)| {
            !number.is_empty()
                && number.chars().all(|c| c.is_ascii_digit())
                && rest.starts_with(' ')
        });
    ordered
        || trimmed.starts_with(['#', '|', '>'])
        || ["- ", "* ", "+ "]
            .iter()
            .any(|marker| trimmed.starts_with(marker))
}

/// Whether a line opens a list item or a table row.
fn is_item(trimmed: &str) -> bool {
    opens_a_unit(trimmed) && !trimmed.starts_with(['#', '>'])
}

/// A paragraph, list item or table row.
struct Unit {
    /// The line it starts on.
    line: usize,
    text: String,
    /// Whether it is a list item or a table row.
    item: bool,
    /// Units share a run when they are list items or rows with no blank
    /// line and nothing else between them.
    run: usize,
}

/// The units of a Markdown text, outside fenced code.
fn units(text: &str) -> Vec<Unit> {
    let mut units: Vec<Unit> = Vec::new();
    let (mut open, mut fenced, mut run) = (false, false, 0);
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            (fenced, open, run) = (!fenced, false, run + 1);
            continue;
        }
        if fenced || trimmed.is_empty() {
            (open, run) = (false, run + 1);
            continue;
        }
        if open && !opens_a_unit(trimmed) {
            let unit = units.last_mut().expect("an open unit");
            unit.text.push(' ');
            unit.text.push_str(trimmed);
            continue;
        }
        let item = is_item(trimmed);
        if !item || units.last().is_some_and(|last| !last.item) {
            run += 1;
        }
        units.push(Unit {
            line: index + 1,
            text: trimmed.to_string(),
            item,
            run,
        });
        open = true;
    }
    units
}

/// The marks that join a list, kept as tokens of their own.
const JOINING_MARKS: [char; 6] = [',', '/', '&', '+', ';', '|'];

/// A unit's words, lowercased, with [`JOINING_MARKS`] kept as tokens of
/// their own and every other mark (emphasis, code, dashes) a separator.
fn tokens(unit: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    for c in unit.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            tokens.push(std::mem::take(&mut word));
        }
        if JOINING_MARKS.contains(&c) {
            tokens.push(c.to_string());
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

/// The harness named at `at`, and where the words after it begin: a
/// product's own suffix (`Claude Code`, `Codex CLI`) belongs to the name.
fn harness(tokens: &[String], at: usize) -> Option<(&'static str, usize)> {
    let name = HARNESSES
        .into_iter()
        .find(|name| tokens.get(at).is_some_and(|token| token == name))?;
    let suffix = tokens
        .get(at + 1)
        .is_some_and(|next| next == "code" || next == "cli");
    Some((name, at + 1 + usize::from(suffix)))
}

/// The words that join a list.
const JOINING_WORDS: [&str; 3] = ["and", "or", "nor"];

fn is_joiner(token: &str) -> bool {
    JOINING_WORDS.contains(&token) || token.chars().all(|c| JOINING_MARKS.contains(&c))
}

/// Where the next item begins after a list's joiner at `at`: a mark, a
/// word, or a mark then a word (`, and`, `, or`).
fn joined(tokens: &[String], at: usize) -> Option<usize> {
    let word = |offset: usize| tokens.get(at + offset).map(String::as_str);
    match (word(0)?, word(1)) {
        (first, Some(second))
            if JOINING_MARKS.iter().any(|mark| first == mark.to_string())
                && JOINING_WORDS.contains(&second) =>
        {
            Some(at + 2)
        }
        (first, _) if is_joiner(first) => Some(at + 1),
        _ => None,
    }
}

/// The names of the list that starts with the name at `at`, and where it
/// ends. After each joiner comes a name, one other word and a name, or one
/// other word that is itself an item because another joiner follows it.
fn list_from(tokens: &[String], at: usize) -> Option<(Vec<&'static str>, usize)> {
    let (first, mut next) = harness(tokens, at)?;
    let mut names = vec![first];
    while let Some(item) = joined(tokens, next) {
        if let Some((name, after)) = harness(tokens, item) {
            names.push(name);
            next = after;
            continue;
        }
        let word = tokens.get(item).filter(|token| !is_joiner(token));
        if let Some((name, after)) = word.and_then(|_| harness(tokens, item + 1)) {
            names.push(name);
            next = after;
        } else if word.is_some() && joined(tokens, item + 1).is_some() {
            next = item + 1;
        } else {
            break;
        }
    }
    Some((names, next))
}

fn claude_and_codex(names: &[&str]) -> bool {
    names.contains(&"claude") && names.contains(&"codex")
}

/// The lists in one unit that name claude and codex, as their names.
fn lists_in(tokens: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    let mut at = 0;
    while at < tokens.len() {
        match list_from(tokens, at) {
            Some((names, next)) => {
                if claude_and_codex(&names) {
                    found.push(names.join(" "));
                }
                at = next;
            }
            None => at += 1,
        }
    }
    found
}

/// Every list of harnesses in `text` that names claude and codex, leaves
/// dsh out, and sits where dsh is never named: in one unit, or across a run
/// of adjacent list items or rows that each lead with a name. Its line and
/// names.
fn lists_without_dsh(text: &str) -> Vec<(usize, String)> {
    let units = units(text);
    let tokens: Vec<Vec<String>> = units.iter().map(|unit| tokens(&unit.text)).collect();
    let names_dsh = |at: usize| tokens[at].iter().any(|token| token == "dsh");
    let mut found = Vec::new();
    for (at, unit) in units.iter().enumerate() {
        if !names_dsh(at) {
            found.extend(
                lists_in(&tokens[at])
                    .into_iter()
                    .map(|names| (unit.line, names)),
            );
        }
    }
    let mut start = 0;
    while start < units.len() {
        let run = units[start].run;
        let end = start
            + units[start..]
                .iter()
                .take_while(|unit| unit.run == run)
                .count();
        let items = start..end;
        let leading: Vec<&str> = items
            .clone()
            .filter(|&at| units[at].item)
            .filter_map(|at| harness(&tokens[at], 0).map(|(name, _)| name))
            .collect();
        let refused = found
            .iter()
            .any(|(line, _)| items.clone().any(|at| units[at].line == *line));
        if claude_and_codex(&leading) && !refused && !items.clone().any(names_dsh) {
            found.push((units[start].line, leading.join(" ")));
        }
        start = end;
    }
    found.sort();
    found
}

#[test]
fn every_list_of_agent_clis_names_dsh() {
    let root = workspace();
    for (record, reason) in RECORDS {
        assert!(
            root.join(record).is_dir(),
            "{record} ({reason}) is no longer a directory, so its exemption covers nothing"
        );
    }
    let files: Vec<String> = tracked_markdown()
        .into_iter()
        .filter(|file| !RECORDS.iter().any(|(record, _)| file.starts_with(record)))
        .collect();
    for known in [
        "README.md",
        "docs/guides/quickstart.md",
        "docs/house-rules.md",
    ] {
        assert!(
            files.iter().any(|file| file == known),
            "{known} was not read"
        );
    }
    let offenses: Vec<String> = files
        .iter()
        .flat_map(|file| {
            let text = std::fs::read_to_string(root.join(file)).expect(file);
            lists_without_dsh(&text)
                .into_iter()
                .map(move |(line, names)| format!("{file}:{line}: {names}"))
        })
        .collect();
    assert_eq!(
        offenses,
        Vec::<String>::new(),
        "a list of agent CLIs reads claude, codex or dsh"
    );
}

/// The rule bites however the list is written: case, emphasis, code spans,
/// a line break, a slash, `&`, `+`, `;`, `nor`, a table row, one word
/// between a joiner and a name, adjacent bullets, the product's own name,
/// either order. It passes a list that names dsh, a paragraph that names
/// dsh beside the two, adjacent bullets one of which names dsh, and fenced
/// code; a later paragraph naming dsh does not excuse an earlier one.
#[test]
fn a_list_that_leaves_dsh_out_is_refused_however_it_is_written() {
    let planted = "\
Install claude and codex first.

**Claude** or *Codex* will do.

`claude`, `codex` on PATH.

A seat runs Claude
or Codex under the recipe.

- a claude/codex split
- Codex or Claude Code, either
- CLAUDE, CODEX, AND OTHERS

Any of claude, codex or dsh.

Claude or Codex hold the review gate; dsh alone is not enough.

```
claude or codex
```

The claude and codex transcripts differ.

dsh keeps its own session file.

Pair claude & codex.

Pair claude + codex.

Pair claude; codex.

Neither claude nor codex.

| claude | codex |

Claude Code or the Codex CLI.

Any of claude, gemini or codex.

- **claude**: the Anthropic CLI.
- **codex**: the OpenAI CLI.

- claude reads the house.
- codex reads it too.
- dsh reads it last.

Claude is the default. The codex tooling differs.
";
    assert_eq!(
        lists_without_dsh(planted),
        [
            (1, "claude codex".to_string()),
            (3, "claude codex".to_string()),
            (5, "claude codex".to_string()),
            (7, "claude codex".to_string()),
            (10, "claude codex".to_string()),
            (11, "codex claude".to_string()),
            (12, "claude codex".to_string()),
            (22, "claude codex".to_string()),
            (26, "claude codex".to_string()),
            (28, "claude codex".to_string()),
            (30, "claude codex".to_string()),
            (32, "claude codex".to_string()),
            (34, "claude codex".to_string()),
            (36, "claude codex".to_string()),
            (38, "claude codex".to_string()),
            (40, "claude codex".to_string()),
        ]
    );
}
