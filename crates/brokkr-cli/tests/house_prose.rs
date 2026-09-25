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
//! `**` nor a code span can hide a list. Fenced code is not prose.

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

/// The paragraphs and list items of a Markdown text, outside fenced code,
/// each with the line it starts on.
fn units(text: &str) -> Vec<(usize, String)> {
    let mut units: Vec<(usize, String)> = Vec::new();
    let mut open = false;
    let mut fenced = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fenced = !fenced;
            open = false;
            continue;
        }
        if fenced || trimmed.is_empty() {
            open = false;
            continue;
        }
        if open && !opens_a_unit(trimmed) {
            let (_, unit) = units.last_mut().expect("an open unit");
            unit.push(' ');
            unit.push_str(trimmed);
        } else {
            units.push((index + 1, trimmed.to_string()));
            open = true;
        }
    }
    units
}

/// A unit's words, lowercased, with `,` and `/` kept as tokens of their
/// own and every other mark (emphasis, code, dashes) a separator.
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
        if matches!(c, ',' | '/') {
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

/// Where the next name begins after a list's joining word at `at`: `,`,
/// `/`, `and`, `or`, `, and` or `, or`.
fn joined(tokens: &[String], at: usize) -> Option<usize> {
    let word = |offset: usize| tokens.get(at + offset).map(String::as_str);
    match (word(0)?, word(1)) {
        (",", Some("and" | "or")) => Some(at + 2),
        ("," | "/" | "and" | "or", _) => Some(at + 1),
        _ => None,
    }
}

/// Every list of harnesses in `text` that names claude and codex, leaves
/// dsh out, and sits in a unit that never names dsh: its line and names.
fn lists_without_dsh(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (line, unit) in units(text) {
        let tokens = tokens(&unit);
        if tokens.iter().any(|token| token == "dsh") {
            continue;
        }
        let mut at = 0;
        while at < tokens.len() {
            let Some((first, mut next)) = harness(&tokens, at) else {
                at += 1;
                continue;
            };
            let mut names = vec![first];
            while let Some((name, after)) =
                joined(&tokens, next).and_then(|at| harness(&tokens, at))
            {
                names.push(name);
                next = after;
            }
            if names.contains(&"claude") && names.contains(&"codex") {
                found.push((line, names.join(" ")));
            }
            at = next;
        }
    }
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
/// a line break, a slash, the product's own name, either order. It passes a
/// list that names dsh, a paragraph that names dsh beside the two, and
/// fenced code; a later paragraph naming dsh does not excuse an earlier one.
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
        ]
    );
}
