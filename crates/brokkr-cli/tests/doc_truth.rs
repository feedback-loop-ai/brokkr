//! Issue #365: the living docs are held to what is true, not to an
//! inventory of what they once said. A section a page loses is a
//! reviewer's call; a false claim is this file's failure.
//!
//! A living doc is a tracked Markdown file that `status_pages::is_record`
//! does not call a record, outside the [`FROZEN`] surfaces. Each is read as
//! blocks: a paragraph, list or table between blank lines, or a fenced
//! block whole. Four rules hold them:
//!
//! - every repository path a code span names is in the tree (lychee holds
//!   the links);
//! - every version of Brokkr a doc states is the workspace's, unless its
//!   block follows the [`ILLUSTRATIVE`] mark;
//! - every decision a doc or a source comment calls proposed is proposed
//!   in `docs/decisions/README.md`;
//! - no retired term stands in a block that does not cite what retired it,
//!   outside the paths [`ALLOWED`] names with a reason.
//!
//! What a rule cannot read is refused, never skipped: a fence left open,
//! a path written with a character no path here carries, a version word
//! followed by a number that is no version, a source whose comment or
//! literal never closes. A backtick nothing closes is literal text, as
//! CommonMark renders it, not a span.

use std::collections::BTreeSet;

use crate::tracked_files;
use crate::workspace_root::{read, workspace};
use crate::{decisions_index, packaging, rust_source, status_pages};

/// Surfaces whose bytes never move (the house rules): a rule here could
/// only report them, so none is read, each for its reason.
const FROZEN: [(&str, &str); 4] = [
    (
        "contracts/",
        "a contract changes by a new version beside the old one",
    ),
    (
        "fixtures/",
        "the evaluator corpus is versioned, never rewritten",
    ),
    ("reference/", "the reference and its schemas are read-only"),
    ("policy/", "the production table is read-only"),
];

/// Whether the tracked file at `path` is living text these rules hold.
fn is_living(path: &str) -> bool {
    !status_pages::is_record(path) && !FROZEN.iter().any(|(frozen, _)| path.starts_with(frozen))
}

/// A paragraph, list or table between blank lines, or the inside of a
/// fenced block: its text and the line its text starts on.
struct Block {
    first: usize,
    text: String,
    fenced: bool,
}

impl Block {
    /// The line the byte at `at` of the block's text stands on.
    fn line_of(&self, at: usize) -> usize {
        self.first + self.text[..at].matches('\n').count()
    }
}

/// A living doc, read into blocks.
struct Doc {
    path: String,
    blocks: Vec<Block>,
}

/// Appends a line to a block's text.
fn push_line(block: &mut Block, line: &str) {
    if !block.text.is_empty() || !block.fenced {
        block.text.push('\n');
    }
    block.text.push_str(line);
}

/// A Markdown text's blocks, or why it cannot be read.
fn blocks(text: &str) -> Result<Vec<Block>, String> {
    let mut blocks: Vec<Block> = Vec::new();
    let (mut fence, mut open) = (None, false);
    for (at, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let marker = ["```", "~~~"]
            .into_iter()
            .find(|marker| trimmed.starts_with(marker));
        if let Some(opened) = fence {
            match marker == Some(opened) {
                true => fence = None,
                false => push_line(blocks.last_mut().expect("the fenced block"), line),
            }
        } else if let Some(marker) = marker {
            (fence, open) = (Some(marker), false);
            blocks.push(Block {
                first: at + 2,
                text: String::new(),
                fenced: true,
            });
        } else if trimmed.is_empty() {
            open = false;
        } else if open {
            push_line(blocks.last_mut().expect("an open block"), line);
        } else {
            open = true;
            blocks.push(Block {
                first: at + 1,
                text: line.to_string(),
                fenced: false,
            });
        }
    }
    match fence {
        Some(marker) => Err(format!("a block fenced with {marker} never closes")),
        None => Ok(blocks),
    }
}

/// A doc as these rules read it, or why it cannot be read.
fn doc(path: &str, text: &str) -> Result<Doc, String> {
    Ok(Doc {
        path: path.to_string(),
        blocks: blocks(text).map_err(|why| format!("{path}: cannot be read: {why}"))?,
    })
}

/// Every living doc; one that cannot be read is an `Err`.
fn living_docs() -> Vec<Result<Doc, String>> {
    tracked_files::tracked(&workspace(), &["*.md"])
        .into_iter()
        .filter(|path| is_living(path))
        .map(|path| doc(&path, &read(&path)))
        .collect()
}

/// The living docs that can be read; `every_living_doc_is_read` refuses
/// the rest.
fn readable_docs() -> Vec<Doc> {
    living_docs().into_iter().filter_map(Result::ok).collect()
}

#[test]
fn every_living_doc_is_read() {
    let root = workspace();
    for (frozen, reason) in FROZEN {
        assert!(
            root.join(frozen).is_dir(),
            "{frozen} ({reason}) is no longer a directory"
        );
    }
    let (readable, unreadable): (Vec<_>, Vec<_>) =
        living_docs().into_iter().partition(Result::is_ok);
    let unreadable: Vec<String> = unreadable.into_iter().filter_map(Result::err).collect();
    assert_eq!(unreadable, Vec::<String>::new());
    let paths: Vec<String> = readable.into_iter().flatten().map(|doc| doc.path).collect();
    for living in [
        "README.md",
        "ARCHITECTURE.md",
        "docs/guides/quickstart.md",
        "openspec/specs/boundary-guides/spec.md",
    ] {
        assert!(
            paths.iter().any(|path| path == living),
            "{living} is not read"
        );
    }
    for unread in [
        "docs/decisions/README.md",
        "docs/history/target-architecture.md",
        "contracts/README.md",
    ] {
        assert!(!paths.iter().any(|path| path == unread), "{unread} is read");
    }
    assert_eq!(
        doc("docs/a.md", "text\n\n```sh\nnever closed\n").err(),
        Some("docs/a.md: cannot be read: a block fenced with ``` never closes".to_string())
    );
}

/// The code spans of a block's text, each with the byte it starts at, as
/// CommonMark reads them: a run of backticks opens a span the next run of
/// the same length closes, a run nothing closes is literal text, and so is
/// a backtick a backslash escapes outside a span.
fn code_spans(text: &str) -> Vec<(usize, &str)> {
    let run = |at: usize| text[at..].chars().take_while(|&c| c == '`').count();
    let mut spans = Vec::new();
    let mut at = 0;
    while let Some(start) = text[at..].find('`').map(|found| at + found) {
        let ticks = run(start);
        let mut close = start + ticks;
        let closed = loop {
            match text[close..].find('`').map(|found| close + found) {
                Some(found) if run(found) == ticks => break Some(found),
                Some(found) => close = found + run(found),
                None => break None,
            }
        };
        match closed {
            Some(close) if !text[..start].ends_with('\\') => {
                spans.push((start + ticks, text[start + ticks..close].trim()));
                at = close + ticks;
            }
            _ => at = start + ticks,
        }
    }
    spans
}

/// The characters a repository path is written with here, a pattern's
/// placeholders included.
fn path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._-/<>{}*…+@".contains(c)
}

/// The repository path a code span names, if it names one: one word whose
/// first segment is a top-level directory the tree tracks, its `:line` or
/// `#anchor` suffix dropped. A word this cannot read as a path is an
/// `Err`.
fn named_path<'a>(span: &'a str, tops: &BTreeSet<String>) -> Option<Result<&'a str, String>> {
    let (first, _) = span.split_once('/')?;
    if span.contains(char::is_whitespace) || !tops.contains(first) {
        return None;
    }
    let path = span.split_once('#').map_or(span, |(path, _)| path);
    let path = match path.split_once(':') {
        None => path,
        Some((path, at))
            if !at.is_empty() && at.chars().all(|c| c.is_ascii_digit() || c == '-') =>
        {
            path
        }
        Some(_) => return Some(Err(format!("`{span}` reads as no path and line"))),
    };
    Some(match path.chars().find(|&c| !path_char(c)) {
        Some(c) => Err(format!("`{span}` carries `{c}`, which no path here does")),
        None => Ok(path),
    })
}

/// A path pattern's segment with each placeholder (`<name>`, `{a,b}`,
/// `…`, `...`) written as `*`.
fn glob(segment: &str) -> Vec<char> {
    let mut out = Vec::new();
    let mut chars = segment.chars();
    while let Some(c) = chars.next() {
        let close = match c {
            '<' => '>',
            '{' => '}',
            '…' | '*' => {
                out.push('*');
                continue;
            }
            c => {
                out.push(c);
                continue;
            }
        };
        chars.by_ref().find(|&c| c == close);
        out.push('*');
    }
    let text: String = out.into_iter().collect();
    text.replace("...", "*").chars().collect()
}

/// Whether `name` fills a [`glob`]bed segment.
fn fills(pattern: &[char], name: &[char]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some(('*', rest)) => (0..=name.len()).any(|skip| fills(rest, &name[skip..])),
        Some((c, rest)) => name.first() == Some(c) && fills(rest, &name[1..]),
    }
}

/// The tracked tree: every tracked file and every directory above one,
/// and the top-level directories.
struct Tree {
    entries: BTreeSet<String>,
    tops: BTreeSet<String>,
}

impl Tree {
    fn of(files: impl IntoIterator<Item = String>) -> Tree {
        let mut entries = BTreeSet::new();
        for file in files {
            for (at, _) in file.match_indices('/') {
                entries.insert(file[..at].to_string());
            }
            entries.insert(file);
        }
        let tops = entries
            .iter()
            .filter_map(|entry| entry.split_once('/').map(|(top, _)| top.to_string()))
            .collect();
        Tree { entries, tops }
    }

    /// Whether the tree holds `path`, a pattern filled by any entry.
    fn holds(&self, path: &str) -> bool {
        let path = path.trim_end_matches('/');
        let wild = ['<', '{', '*', '…'].iter().any(|&c| path.contains(c)) || path.contains("...");
        if !wild {
            return self.entries.contains(path);
        }
        let pattern: Vec<Vec<char>> = path.split('/').map(glob).collect();
        self.entries.iter().any(|entry| {
            let names: Vec<&str> = entry.split('/').collect();
            names.len() == pattern.len()
                && pattern
                    .iter()
                    .zip(names)
                    .all(|(segment, name)| fills(segment, &name.chars().collect::<Vec<_>>()))
        })
    }
}

/// Paths a living doc names that this repository rightly does not hold:
/// the doc (or the directory of docs), the path it names (or the start of
/// it), and why. An allowance that covers nothing fails, so none outlives
/// its text.
const ABSENT: [(&str, &str, &str); 19] = [
    (
        "docs/guides/",
        "scripts/verify-seat.sh",
        "the verify script `brokkr init` writes into the operator's repository",
    ),
    (
        "docs/guides/",
        "agents/README.md",
        "the agents README `brokkr init` writes into the operator's repository",
    ),
    (
        "docs/guides/cards/bun.md",
        "recipes/bun",
        "says no such recipe ships",
    ),
    (
        "docs/guides/cards/go.md",
        "recipes/go",
        "says no such recipe ships",
    ),
    (
        "docs/guides/cards/python.md",
        "recipes/python",
        "says no such recipe ships",
    ),
    (
        "docs/guides/cards/rust.md",
        "recipes/rust",
        "says no such recipe ships",
    ),
    (
        "docs/guides/contributing-by-hand.md",
        ".cargo/config",
        "names the configuration the repository must not carry",
    ),
    (
        "extensions/dsh/PROVENANCE.md",
        "extensions/dsh/resume-policy/",
        "names the home an extension takes only once one is needed",
    ),
    (
        "openspec/specs/driver-confine-retirement/spec.md",
        "crates/brokkr-runtime/tests/confine_test.rs",
        "requires the file's deletion",
    ),
    (
        "openspec/specs/gate-boundary-policy/spec.md",
        "recipes/sdd",
        "records that the recipe folded into triage",
    ),
    (
        "openspec/specs/gate-boundary-policy/spec.md",
        ".cargo/credentials.toml",
        "a file a scenario plants under a test's own HOME",
    ),
    (
        "openspec/specs/realm-crossing/spec.md",
        "contracts/orders.v1.schema.json",
        "another realm's contract, in a scenario",
    ),
    (
        "openspec/changes/decision-0065-capabilities-slice-one/specs/",
        "capabilities/",
        "a path inside the recipe a scenario builds",
    ),
    (
        "openspec/changes/decision-0065-capabilities-slice-one/specs/",
        "agents/charters/worker.md",
        "a charter in the library a scenario builds",
    ),
    (
        "recipes/wager-harness/README.md",
        "recipes/fast-codex",
        "names the recipe the wager deleted",
    ),
    (
        "openspec/specs/boundary-guides/spec.md",
        "docs/extension-model.md",
        BLUEPRINTS_MOVED,
    ),
    (
        "openspec/specs/boundary-guides/spec.md",
        "docs/target-architecture.md",
        BLUEPRINTS_MOVED,
    ),
    (
        "openspec/changes/boundary-seatbelt-slice-ii/specs/boundary-guides/spec.md",
        "docs/extension-model.md",
        BLUEPRINTS_MOVED,
    ),
    (
        "openspec/changes/boundary-seatbelt-slice-ii/specs/boundary-guides/spec.md",
        "docs/target-architecture.md",
        BLUEPRINTS_MOVED,
    ),
];

/// Why the boundary-guides spec and the delta that restates it still name
/// the two blueprints where they stood before #364 moved them to
/// `docs/history/`: the passage is a baselined data clone, and the two move
/// together with a refresh of `quality/jscpd-baseline-data.json`.
const BLUEPRINTS_MOVED: &str = "a baselined clone of its delta; the path moves with a \
                                data-baseline refresh";

/// Each code span in `docs` that names a path the tree does not hold from
/// the root or from the doc's own directory, outside what `absent`, given
/// [`ABSENT`], allows; each span that cannot be read; and each allowance
/// that covered nothing.
fn missing_paths(docs: &[Doc], tree: &Tree, absent: &[(&str, &str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; absent.len()];
    for doc in docs {
        let here = doc.path.rsplit_once('/').map(|(dir, _)| dir);
        let held = |path: &str| {
            tree.holds(path) || here.is_some_and(|dir| tree.holds(&format!("{dir}/{path}")))
        };
        for block in doc.blocks.iter().filter(|block| !block.fenced) {
            for (at, span) in code_spans(&block.text) {
                let line = block.line_of(at);
                let path = match named_path(span, &tree.tops) {
                    None => continue,
                    Some(Ok(path)) if held(path) => continue,
                    Some(Ok(path)) => path,
                    Some(Err(why)) => {
                        found.push(format!("{}:{line}: {why}", doc.path));
                        continue;
                    }
                };
                match absent.iter().position(|(page, named, _)| {
                    doc.path.starts_with(page) && path.starts_with(named)
                }) {
                    Some(index) => used[index] = true,
                    None => found.push(format!(
                        "{}:{line}: `{span}` names a path the tree does not hold",
                        doc.path
                    )),
                }
            }
        }
    }
    found.extend(stale_allowances(absent, &used));
    found
}

/// Each allowance, `(path, what, why)`, that covered nothing.
fn stale_allowances(allowed: &[(&str, &str, &str)], used: &[bool]) -> Vec<String> {
    allowed
        .iter()
        .zip(used)
        .filter(|(_, used)| !**used)
        .map(|((path, what, _), _)| format!("{path}: allows `{what}`, which it no longer names"))
        .collect()
}

#[test]
fn every_path_a_living_doc_names_is_in_the_tree() {
    let tree = Tree::of(tracked_files::tracked(&workspace(), &[]));
    assert_eq!(
        missing_paths(&readable_docs(), &tree, &ABSENT),
        Vec::<String>::new(),
        "a living doc names a repository path that is not there"
    );
}

#[test]
fn a_path_the_tree_does_not_hold_is_refused() {
    let tree = Tree::of(
        [
            "docs/guides/quickstart.md",
            "recipes/fast/bundle.json",
            "recipes/fast/scripts/verify-seat.sh",
            "scripts/coverage-exact.sh",
        ]
        .map(str::to_string),
    );
    let planted = doc(
        "recipes/fast/README.md",
        "Read `docs/guides/quickstart.md#step-1`, `scripts/coverage-exact.sh:12-20`\n\
         and `recipes/<name>/bundle.json`, run `scripts/verify-seat.sh` here,\n\
         and keep `docs/guides/` and `recipes/*/scripts/`.\n\
         \n\
         Not `docs/target-architecture.md`, nor `recipes/<name>/policy.json`.\n\
         \n\
         `docs/a b.md` and `bundles/self` and `.forge/x` are no repository path.\n\
         \n\
         Unreadable: `docs/guides/quick$start.md`, `scripts/x.sh:main`.\n\
         \n\
         ```\n\
         `docs/never.md` in a fence is a sample\n\
         ```\n\
         \n\
         Escaped \\`docs/y.md\\`, then `docs/b.md`, then one left open: `docs/x.md\n\
         \n\
         There is no `recipes/bun`.\n",
    )
    .unwrap();
    let absent = [
        ("recipes/fast/", "recipes/bun", "says no such recipe ships"),
        (
            "recipes/fast/README.md",
            "recipes/go",
            "says no such recipe ships",
        ),
    ];
    assert_eq!(
        missing_paths(&[planted], &tree, &absent),
        [
            "recipes/fast/README.md:5: `docs/target-architecture.md` names a path the tree does not hold",
            "recipes/fast/README.md:5: `recipes/<name>/policy.json` names a path the tree does not hold",
            "recipes/fast/README.md:9: `docs/guides/quick$start.md` carries `$`, which no path here does",
            "recipes/fast/README.md:9: `scripts/x.sh:main` reads as no path and line",
            "recipes/fast/README.md:15: `docs/b.md` names a path the tree does not hold",
            "recipes/fast/README.md: allows `recipes/go`, which it no longer names",
        ]
    );
}

/// The comment that marks the next block's Brokkr versions illustrative: a
/// sample whose version is beside its point.
const ILLUSTRATIVE: &str = "<!-- illustrative version -->";

/// The spellings that state Brokkr's own version: the text before the
/// version, and the text that must follow it for the spelling to be one.
/// A spelling with nothing to follow it is followed by a version, or the
/// block cannot be read.
const VERSION_SPELLINGS: [(&str, &str); 5] = [
    ("brokkr ", ""),
    ("Brokkr ", ""),
    ("engine ", ""),
    ("\"engine_version\": \"", "\""),
    // A link that headlines a release: "The [v0.12.0 release](…)".
    ("[v", " release"),
];

/// The version `text` opens with (`0.12.0`, `v0.12.0`, `0.1.5-rc.1`),
/// without its `v`, and the text after it.
fn version(text: &str) -> Option<(&str, &str)> {
    let bare = text.strip_prefix('v').unwrap_or(text);
    let mut parts = 0;
    let mut end = 0;
    for part in bare.split('.') {
        let digits = part.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || parts == 3 {
            break;
        }
        parts += 1;
        end += digits;
        if digits < part.len() || parts == 3 {
            break;
        }
        end += 1;
    }
    if parts < 3 {
        return None;
    }
    let tail = bare[end..].strip_prefix('-').map_or(0, |pre| {
        let label = pre
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '.')
            .next()
            .unwrap_or_default();
        1 + label.trim_end_matches('.').len()
    });
    let end = end + tail;
    let rest = &bare[end..];
    let joined = rest.starts_with(|c: char| c.is_ascii_alphanumeric())
        || rest
            .strip_prefix('.')
            .is_some_and(|after| after.starts_with(|c: char| c.is_ascii_digit()));
    (!joined).then(|| (&bare[..end], rest))
}

/// Each version of Brokkr `text` states by a [`VERSION_SPELLINGS`] entry,
/// with the byte it stands at: the version, or the spelling that no
/// version this can read follows.
fn versions_in(text: &str) -> Vec<(usize, Result<&str, &'static str>)> {
    let mut found = Vec::new();
    for (before, after) in VERSION_SPELLINGS {
        for (at, _) in text.match_indices(before) {
            if text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_') {
                continue;
            }
            let rest = &text[at + before.len()..];
            let numbered = rest
                .trim_start_matches('v')
                .starts_with(|c: char| c.is_ascii_digit());
            match version(rest) {
                Some((stated, tail)) if tail.starts_with(after) => found.push((at, Ok(stated))),
                _ if after.is_empty() && numbered => found.push((at, Err(before))),
                _ => {}
            }
        }
    }
    found
}

/// Each Brokkr version in `docs` that is not `current`, outside a block
/// marked [`ILLUSTRATIVE`], and each version word this cannot read.
fn stale_versions(docs: &[Doc], current: &str) -> Vec<String> {
    let mut found = Vec::new();
    for doc in docs {
        let mut marked = false;
        for block in &doc.blocks {
            let illustrative = std::mem::replace(&mut marked, block.text.trim() == ILLUSTRATIVE);
            for (at, stated) in versions_in(&block.text) {
                let line = block.line_of(at);
                match stated {
                    Ok(stated) if stated == current || illustrative => {}
                    Ok(stated) => found.push(format!(
                        "{}:{line}: states Brokkr {stated}; the workspace is {current}",
                        doc.path
                    )),
                    Err(before) => found.push(format!(
                        "{}:{line}: `{before}` is followed by no version this can read",
                        doc.path
                    )),
                }
            }
        }
    }
    found
}

#[test]
fn every_brokkr_version_a_living_doc_states_is_the_workspaces() {
    assert_eq!(
        stale_versions(&readable_docs(), &packaging::workspace_version()),
        Vec::<String>::new(),
        "a living doc states a version of Brokkr that is not Cargo.toml's: \
         write the workspace's, or mark the sample {ILLUSTRATIVE}"
    );
}

#[test]
fn a_stale_version_is_refused_unless_marked_illustrative() {
    let planted = doc(
        "docs/guides/a.md",
        "The [v0.11.0 release](../releases/v0.11.0.md) adds things;\n\
         the [v0.10.0 notes](../releases/v0.10.0.md) say more.\n\
         \n\
         ```\n\
         $ brokkr --version\n\
         brokkr 0.8.0\n\
         ok contracts: engine 0.12.0, event_schema 1\n\
         ```\n\
         \n\
         <!-- illustrative version -->\n\
         \n\
         ```\n\
         { \"type\": \"hello\", \"engine_version\": \"0.5.0\" }\n\
         ```\n\
         \n\
         { \"type\": \"hello\", \"engine_version\": \"0.7.0\" }\n\
         \n\
         Brokkr v0.12.0 ships; the dsh-engine 0.1.5 differs; brokkr init runs.\n\
         \n\
         brokkr 0.12 and engine 13/13 read as no version.\n",
    )
    .unwrap();
    assert_eq!(
        stale_versions(&[planted], "0.12.0"),
        [
            "docs/guides/a.md:1: states Brokkr 0.11.0; the workspace is 0.12.0",
            "docs/guides/a.md:6: states Brokkr 0.8.0; the workspace is 0.12.0",
            "docs/guides/a.md:16: states Brokkr 0.7.0; the workspace is 0.12.0",
            "docs/guides/a.md:20: `brokkr ` is followed by no version this can read",
            "docs/guides/a.md:20: `engine ` is followed by no version this can read",
        ]
    );
    assert_eq!(version("0.1.5-rc.1. More"), Some(("0.1.5-rc.1", ". More")));
    assert_eq!(version("v0.12.0 release"), Some(("0.12.0", " release")));
    assert_eq!(version("0.12.0.1"), None);
}

/// A word without the punctuation around it, lowercased.
fn bare(word: &str) -> String {
    let word = word.trim_end_matches("'s").trim_end_matches("’s");
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// A word that is a four-digit decision number, its punctuation dropped.
fn number(word: Option<&&str>) -> Option<u16> {
    let word = bare(word?);
    (word.len() == 4 && word.chars().all(|c| c.is_ascii_digit()))
        .then(|| word.parse().expect("four digits"))
}

/// Each decision a text calls proposed: `proposed [decision] NNNN`,
/// `proposed by|in|for [decision] NNNN`, or `decision NNNN, proposed`, in
/// any case and across line breaks.
fn called_proposed(text: &str) -> Vec<u16> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut found = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let after = |skip: usize| words.get(at + skip).map(|word| bare(word));
        if bare(word) == "proposed" {
            let mut next = 1;
            if after(next).is_some_and(|word| ["by", "in", "for"].contains(&word.as_str())) {
                next += 1;
            }
            if after(next).is_some_and(|word| word == "decision" || word == "decisions") {
                next += 1;
            }
            found.extend(number(words.get(at + next)));
        } else if bare(word) == "decision"
            && words.get(at + 1).is_some_and(|word| word.ends_with(','))
            && after(2).is_some_and(|word| word == "proposed")
            && number(words.get(at + 3)).is_none()
            && after(3).is_none_or(|word| word != "decision")
        {
            found.extend(number(words.get(at + 1)));
        }
    }
    found
}

/// The prose of a tracked file these rules hold: a doc's blocks, a Rust
/// source's comments, a script's `#` lines, each a run of adjacent lines
/// with the line it starts on.
fn prose(path: &str, text: &str) -> Result<Vec<(usize, String)>, String> {
    let lines: Vec<(usize, String)> = match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("md") => {
            return Ok(blocks(text)?
                .into_iter()
                .map(|block| (block.first, block.text))
                .collect());
        }
        Some("rs") => rust_source::comments(text)?
            .into_iter()
            .map(|(at, comment)| {
                let lines: Vec<&str> = comment
                    .lines()
                    .map(|line| {
                        line.trim()
                            .trim_end_matches("*/")
                            .trim_start_matches(['/', '*', '!'])
                    })
                    .collect();
                (at, lines.join("\n"))
            })
            .collect(),
        _ => text
            .lines()
            .zip(1..)
            .filter_map(|(line, at)| {
                let comment = line.trim_start().strip_prefix('#')?;
                (!comment.starts_with('!')).then(|| (at, comment.to_string()))
            })
            .collect(),
    };
    let mut runs: Vec<(usize, String, usize)> = Vec::new();
    for (at, comment) in lines {
        let end = at + comment.matches('\n').count();
        match runs.last_mut() {
            Some((_, run, last)) if *last + 1 == at => {
                run.push('\n');
                run.push_str(&comment);
                *last = end;
            }
            _ => runs.push((at, comment, end)),
        }
    }
    Ok(runs.into_iter().map(|(at, run, _)| (at, run)).collect())
}

/// Each decision `texts` call proposed that the index does not hold as
/// proposed, and each text that cannot be read.
fn misnamed_proposals(
    texts: &[(String, String)],
    statuses: &std::collections::BTreeMap<u16, String>,
) -> Vec<String> {
    let mut found = Vec::new();
    for (path, text) in texts {
        let runs = match prose(path, text) {
            Ok(runs) => runs,
            Err(why) => {
                found.push(format!("{path}: cannot be read: {why}"));
                continue;
            }
        };
        for (line, run) in runs {
            for cited in called_proposed(&run) {
                match statuses.get(&cited).map(String::as_str) {
                    Some("proposed") => {}
                    Some(status) => found.push(format!(
                        "{path}:{line}: calls decision {cited:04} proposed; the index rules it {status}"
                    )),
                    None => found.push(format!(
                        "{path}:{line}: calls decision {cited:04} proposed; the index holds no such decision"
                    )),
                }
            }
        }
    }
    found
}

#[test]
fn every_decision_called_proposed_is_proposed() {
    let root = workspace();
    let texts: Vec<(String, String)> = tracked_files::tracked(&root, &["*.md", "*.rs", "*.sh"])
        .into_iter()
        .filter(|path| is_living(path))
        .map(|path| {
            let text = read(&path);
            (path, text)
        })
        .collect();
    for known in [
        "README.md",
        "crates/brokkr-protocol/src/adapters.rs",
        "scripts/coverage-exact.sh",
    ] {
        assert!(
            texts.iter().any(|(path, _)| path == known),
            "{known} is not read"
        );
    }
    assert_eq!(
        misnamed_proposals(&texts, &decisions_index::cited_statuses()),
        Vec::<String>::new(),
        "a living doc or a source comment calls a decision proposed that the index does not"
    );
}

#[test]
fn a_decision_called_proposed_is_held_to_the_index() {
    let statuses = [(56, "proposed"), (69, "accepted"), (75, "proposed")]
        .into_iter()
        .map(|(number, status)| (number, status.to_string()))
        .collect();
    let planted = [
        (
            "crates/a/src/lib.rs",
            "/// The stamp (proposed\n/// decision 0056 ruling 1).\nfn a() {}\n\
             // proposed decision 0069 ruling 2\nconst S: &str = \"proposed decision 0069\";\n\
             /* Proposed 0071's tail */\n",
        ),
        (
            "docs/a.md",
            "Costs are proposed in decision 0069, which only the operator\n\
             rules (decision 0075, proposed). A proposed decision is a draft;\n\
             (decision 0069, proposed 0056 ruling 4) (decision 0069, proposed decision 0075)\n\
             the proposed 2026-09-28 addendum and #403 are no number.\n\
             \n\
             ```\n\
             (decision 0069, proposed)\n\
             ```\n",
        ),
        (
            "scripts/a.sh",
            "#!/bin/bash\n# Proposed by decision 0069.\necho proposed decision 0069\n",
        ),
        (
            "crates/a/src/open.rs",
            "fn f() { let s = \"never closed; }\n",
        ),
    ];
    let texts: Vec<(String, String)> = planted
        .iter()
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect();
    assert_eq!(
        misnamed_proposals(&texts, &statuses),
        [
            "crates/a/src/lib.rs:4: calls decision 0069 proposed; the index rules it accepted",
            "crates/a/src/lib.rs:6: calls decision 0071 proposed; the index holds no such decision",
            "docs/a.md:1: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:7: calls decision 0069 proposed; the index rules it accepted",
            "scripts/a.sh:2: calls decision 0069 proposed; the index rules it accepted",
            "crates/a/src/open.rs: cannot be read: an unterminated string",
        ]
    );
}

/// A term the house retired, why, and the citation that lets a block name
/// it: a block that cites what retired it says so, and is read true.
struct Retired {
    term: &'static str,
    why: &'static str,
    cite: Option<&'static str>,
}

/// The decision that retired the third host.
const NO_WINDOWS: &str = "decision 0063: the hosts are Linux and macOS";

/// Never built, as `docs/history/target-architecture.md`'s status records.
const NEVER_BUILT: &str = "never built (#364): the DeepSeek harness is the built-in dsh adapter";

const RETIRED: [Retired; 8] = [
    Retired {
        term: "Windows",
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        term: "winget",
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        term: "scoop",
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        term: "Cordis/DeepSeek",
        why: NEVER_BUILT,
        cite: None,
    },
    Retired {
        term: "Surface",
        why: NEVER_BUILT,
        cite: None,
    },
    Retired {
        term: "brokkr-api",
        why: "never built (#364): `brokkr ui` serves the console from brokkr-cli",
        cite: None,
    },
    Retired {
        term: "policy/schemas",
        why: "removed: `policy/` holds the production table alone",
        cite: None,
    },
    Retired {
        term: "bundles/",
        why: "decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
        cite: None,
    },
];

/// Living docs that may name a retired term, each with its reason. An
/// allowance that covers nothing fails, so none outlives its text.
const ALLOWED: [(&str, &str, &str); 7] = [
    (
        "docs/research/",
        "Windows",
        "a research entry reads another project, whose hosts are its own",
    ),
    (
        "openspec/specs/gate-boundary-policy/spec.md",
        "Windows",
        "its Purpose rules each Windows scenario historical under decision 0063 until \
         boundary-seatbelt-slice-ii's delta folds the requirement",
    ),
    (
        "openspec/specs/boundary-guides/spec.md",
        "Windows",
        "its Purpose rules each Windows requirement historical under decision 0063",
    ),
    (
        "openspec/changes/boundary-seatbelt-slice-ii/specs/",
        "Windows",
        "an in-flight delta written before decision 0063; it folds into the specs whose \
         Purpose already rules it historical",
    ),
    (
        "openspec/changes/2026-09-09-226-session-resumption/specs/",
        "Windows",
        "an in-flight delta written before decision 0063; #226's lane folds it at archive",
    ),
    (
        "openspec/changes/boundary-seatbelt-slice-ii/specs/",
        "bundles/",
        "an in-flight delta written before decision 0078; the specs it folds into read \
         `recipes/`",
    ),
    (
        "openspec/changes/decision-0065-capabilities-slice-two/specs/",
        "bundles/",
        "the slice-two lane's in-flight delta, written before decision 0078; that lane \
         folds it",
    ),
];

/// Whether `term` stands at `at` of `text` as a word of its own: no word
/// runs on into either end of it that is a word's (`bundles/` may open a
/// longer path).
fn whole_word(text: &str, at: usize, term: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let open = term.starts_with(word);
    let close = term.ends_with(word);
    (!open || text[..at].chars().next_back().is_none_or(|c| !word(c)))
        && !text[..at].ends_with('-')
        && (!close
            || text[at + term.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word(c)))
}

/// Each retired term a block's `text` names as a word of its own, unless
/// the block cites what retired it: the byte it stands at, the term, and
/// why it was retired.
fn retired_in(text: &str) -> Vec<(usize, &'static str, &'static str)> {
    let mut found = Vec::new();
    for Retired { term, why, cite } in RETIRED {
        if cite.is_some_and(|cite| text.contains(cite)) {
            continue;
        }
        found.extend(
            text.match_indices(term)
                .filter(|&(at, _)| whole_word(text, at, term))
                .map(|(at, _)| (at, term, why)),
        );
    }
    found
}

/// Each retired term in `docs` outside a block that cites what retired it,
/// and each [`ALLOWED`] entry, given as `allowed`, that covered nothing.
fn retired_terms(docs: &[Doc], allowed: &[(&str, &str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; allowed.len()];
    let blocks = docs
        .iter()
        .flat_map(|doc| doc.blocks.iter().map(move |block| (doc, block)));
    for (doc, block) in blocks {
        for (at, term, why) in retired_in(&block.text) {
            let allowance = allowed
                .iter()
                .position(|(path, allowed, _)| *allowed == term && doc.path.starts_with(path));
            match allowance {
                Some(index) => used[index] = true,
                None => found.push(format!(
                    "{}:{}: names `{term}`, retired: {why}",
                    doc.path,
                    block.line_of(at)
                )),
            }
        }
    }
    found.extend(stale_allowances(allowed, &used));
    found
}

#[test]
fn no_living_doc_names_a_retired_term() {
    assert_eq!(
        retired_terms(&readable_docs(), &ALLOWED),
        Vec::<String>::new(),
        "a living doc names a retired term: say what replaced it, or cite what retired it"
    );
}

#[test]
fn a_retired_term_is_refused_outside_its_citation_and_its_allowance() {
    let planted = doc(
        "docs/guides/a.md",
        "Windows is not a host (decision 0063).\n\
         \n\
         Install with winget or scoop on Windows.\n\
         \n\
         The `brokkr-api` crate, a Surface driver, the Cordis/DeepSeek harness,\n\
         `policy/schemas/` and `bundles/self` are gone; `<repo>/bundles/fast` too.\n\
         \n\
         Context windows, a read surface, recipes/my-bundles/ and cordis.patch.yml are not.\n",
    )
    .unwrap();
    let research = doc("docs/research/a.md", "CI runs on Windows.\n").unwrap();
    let allowed = [
        ("docs/research/", "Windows", "another project's hosts"),
        ("docs/guides/b.md", "winget", "stale"),
    ];
    assert_eq!(
        retired_terms(&[planted, research], &allowed),
        [
            "docs/guides/a.md:3: names `Windows`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:3: names `winget`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:3: names `scoop`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:5: names `Cordis/DeepSeek`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:5: names `Surface`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:5: names `brokkr-api`, retired: never built (#364): `brokkr ui` serves the console from brokkr-cli",
            "docs/guides/a.md:6: names `policy/schemas`, retired: removed: `policy/` holds the production table alone",
            "docs/guides/a.md:6: names `bundles/`, retired: decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
            "docs/guides/a.md:6: names `bundles/`, retired: decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
            "docs/guides/b.md: allows `winget`, which it no longer names",
        ]
    );
}
