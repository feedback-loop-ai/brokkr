//! Issue #365: the living docs are held to what is true, not to an
//! inventory of what they once said. A section a page loses is a
//! reviewer's call; a false claim is this file's failure.
//!
//! A living doc is a tracked Markdown file that `status_pages::is_record`
//! does not call a record, outside the [`FROZEN`] surfaces. Each is read as
//! blocks: a paragraph, list or table between blank lines, or a fenced
//! block whole, in a blockquote or not, closed only by a line of its own
//! fence character at least as long and nothing else. Four rules hold them:
//!
//! - every repository path a code span outside a fence names is in the
//!   tree, a `./` or `../` spelling resolved against the doc's directory,
//!   and a file named from no directory found under any (lychee holds the
//!   links; a fenced sample is #365's snapshot ask, so this rule leaves it
//!   as the version rule does not);
//! - every version of Brokkr a doc states is the workspace's, unless its
//!   block follows the [`ILLUSTRATIVE`] mark;
//! - every decision a doc, a source comment or a JSON string calls
//!   proposed is proposed in `docs/decisions/README.md`;
//! - no retired term stands in a block that does not cite what retired it,
//!   outside the paths [`ALLOWED`] names with a reason.
//!
//! What a rule cannot read is refused, never skipped: a fence left open
//! or ended by its blockquote, a path written with a character no path here carries, a placeholder
//! nothing closes or climbing above the root, a version spelling followed
//! by no version, a source whose
//! comment or literal never closes. A backtick nothing closes is literal
//! text, as CommonMark renders it, not a span.

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

/// A line's text inside the blockquote markers (`> `) that open it, at
/// most `most` of them, and how many it carried.
fn unquoted(line: &str, most: usize) -> (usize, &str) {
    let (mut depth, mut rest) = (0, line);
    while depth < most {
        let Some(inner) = rest.trim_start().strip_prefix('>') else {
            break;
        };
        (depth, rest) = (depth + 1, inner.strip_prefix(' ').unwrap_or(inner));
    }
    (depth, rest)
}

/// An open fence: the character it is written with, how many, and the
/// blockquote depth it opened at.
#[derive(Clone, Copy)]
struct Fence {
    mark: char,
    len: usize,
    depth: usize,
}

impl Fence {
    /// The fence a line's unquoted text opens, at `depth`.
    fn opened_by(text: &str, depth: usize) -> Option<Self> {
        let text = text.trim_start();
        let mark = text
            .chars()
            .next()
            .filter(|mark| ['`', '~'].contains(mark))?;
        let len = text.chars().take_while(|&c| c == mark).count();
        (len >= 3).then_some(Self { mark, len, depth })
    }

    /// Whether an unquoted line closes this fence: its own character, at
    /// least as many, and nothing else.
    fn closed_by(self, text: &str) -> bool {
        let text = text.trim();
        text.len() >= self.len && text.chars().all(|c| c == self.mark)
    }

    /// The fence as it is written.
    fn spelled(self) -> String {
        self.mark.to_string().repeat(self.len)
    }
}

/// A Markdown text's blocks, each line's text inside its blockquote
/// markers, or why it cannot be read.
fn blocks(text: &str) -> Result<Vec<Block>, String> {
    let mut blocks: Vec<Block> = Vec::new();
    let (mut fence, mut open) = (None::<Fence>, false);
    for (at, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(opened) = fence {
            let (depth, inside) = unquoted(line, opened.depth);
            if depth < opened.depth {
                return Err(format!(
                    "a block fenced with {} ends with its blockquote, unclosed",
                    opened.spelled()
                ));
            }
            match opened.closed_by(inside) {
                true => fence = None,
                false => push_line(blocks.last_mut().expect("the fenced block"), inside),
            }
            continue;
        }
        let (depth, inside) = unquoted(line, usize::MAX);
        if let Some(opened) = Fence::opened_by(inside, depth) {
            (fence, open) = (Some(opened), false);
            blocks.push(Block {
                first: at + 2,
                text: String::new(),
                fenced: true,
            });
        } else if trimmed.is_empty() {
            open = false;
        } else if open {
            push_line(blocks.last_mut().expect("an open block"), inside);
        } else {
            open = true;
            blocks.push(Block {
                first: at + 1,
                text: inside.to_string(),
                fenced: false,
            });
        }
    }
    match fence {
        Some(opened) => Err(format!(
            "a block fenced with {} never closes",
            opened.spelled()
        )),
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
    for living in ["README.md", "ARCHITECTURE.md", "docs/guides/quickstart.md"] {
        assert!(
            paths.iter().any(|path| path == living),
            "{living} is not read"
        );
    }
    for unread in [
        "docs/decisions/README.md",
        "docs/history/target-architecture.md",
        "contracts/README.md",
        "openspec/specs/boundary-guides/spec.md",
    ] {
        assert!(!paths.iter().any(|path| path == unread), "{unread} is read");
    }
    assert_eq!(
        doc("docs/a.md", "text\n\n```sh\nnever closed\n").err(),
        Some("docs/a.md: cannot be read: a block fenced with ``` never closes".to_string())
    );
}

#[test]
fn a_fence_closes_only_on_its_own_marker_inside_its_quote() {
    let read = |text: &str| {
        blocks(text).map(|blocks| {
            blocks
                .into_iter()
                .map(|block| (block.first, block.text, block.fenced))
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(
        read("````md\n```\nv0.1.0\n```\n`````\nafter\n"),
        Ok(vec![
            (2, "```\nv0.1.0\n```".to_string(), true),
            (6, "after".to_string(), false),
        ])
    );
    assert_eq!(
        read("> Quoted:\n>\n> ```sh\n> brokkr 0.1.0\n>   `a/b`\n> ```\n> after\n"),
        Ok(vec![
            (1, "Quoted:\n".to_string(), false),
            (4, "brokkr 0.1.0\n  `a/b`".to_string(), true),
            (7, "after".to_string(), false),
        ])
    );
    for (text, why) in [
        ("````\n```\n", "a block fenced with ```` never closes"),
        ("~~~\n```\n", "a block fenced with ~~~ never closes"),
        (
            "```\n``` and more\n",
            "a block fenced with ``` never closes",
        ),
        (
            "> ```\n> x\n\n```\n",
            "a block fenced with ``` ends with its blockquote, unclosed",
        ),
    ] {
        assert_eq!(read(text), Err(why.to_string()), "{text}");
    }
}

/// The code spans of a block's text, each with the byte it starts at, as
/// CommonMark reads them: a run of backticks opens a span the next run of
/// the same length closes, a run nothing closes is literal text, and so is
/// a backtick a backslash escapes outside a span: an odd run of them, since
/// two escape each other. The escape takes that one backtick, and the rest
/// of its run still opens a span.
fn code_spans(text: &str) -> Vec<(usize, &str)> {
    let run = |at: usize| text[at..].chars().take_while(|&c| c == '`').count();
    let escaped = |at: usize| text[..at].chars().rev().take_while(|&c| c == '\\').count() % 2 == 1;
    let mut spans = Vec::new();
    let mut at = 0;
    while let Some(start) = text[at..].find('`').map(|found| at + found) {
        if escaped(start) {
            at = start + 1;
            continue;
        }
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
            Some(close) => {
                spans.push((start + ticks, text[start + ticks..close].trim()));
                at = close + ticks;
            }
            None => at = start + ticks,
        }
    }
    spans
}

/// The characters a repository path is written with here, a pattern's
/// placeholders included.
fn path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._-/<>{}*…+@".contains(c)
}

/// The kinds of file a name is read by when the doc leaves unsaid which
/// directory holds it: `README.md`, `scripts/coverage-exact.sh`'s
/// `coverage-exact.sh`, `layering/scan.rs`.
const FILE_KINDS: [&str; 9] = [
    "md", "rs", "json", "sh", "toml", "yml", "yaml", "txt", "lock",
];

/// Where a path that opens with neither `./`, `../` nor a top-level
/// directory stands outside the repository, never read.
const FOREIGN: [(&str, &str); 8] = [
    ("/", "an absolute path names the host's tree"),
    ("~", "a path under a home directory"),
    ("$", "a path under an environment variable"),
    ("<", "a leading placeholder names a root the reader fills"),
    (".forge/", "a run's own working files, never tracked"),
    (
        ".brokkr/",
        "an adopter's Brokkr folder, which this repository, a recipe library, does not hold",
    ),
    ("target/", "cargo's build output"),
    ("node_modules/", "a Node repository's installed packages"),
];

/// Where a path a code span names is read from.
enum Named<'a> {
    /// From the root or the doc's directory, as [`resolved`] reads it: it
    /// opens with `./`, `../` or a top-level directory the tree tracks.
    Placed(&'a str),
    /// Under any directory: the name of a file of one of the
    /// [`FILE_KINDS`], or a path ending in one, whose start the doc leaves
    /// unsaid.
    Within(&'a str),
}

/// The repository path a code span names, if it names one: one word,
/// [`Named`] by how it opens, its `:line` or `#anchor` suffix dropped. A
/// word this cannot read as a path is an `Err`: a character no path here
/// carries, a suffix that is no line, or a placeholder nothing closes.
fn named_path<'a>(span: &'a str, tops: &BTreeSet<String>) -> Option<Result<Named<'a>, String>> {
    if span.contains(char::is_whitespace) {
        return None;
    }
    let path = span.split_once('#').map_or(span, |(path, _)| path);
    let (path, line) = path
        .split_once(':')
        .map_or((path, None), |(path, at)| (path, Some(at)));
    let named = match path.split_once('/') {
        Some((first, _)) if [".", ".."].contains(&first) || tops.contains(first) => {
            Named::Placed(path)
        }
        _ if FOREIGN.iter().any(|(root, _)| path.starts_with(root)) => return None,
        _ => match path.rsplit('/').next()?.rsplit_once('.') {
            Some((name, kind)) if !name.is_empty() && FILE_KINDS.contains(&kind) => {
                Named::Within(path)
            }
            _ => return None,
        },
    };
    let lined =
        line.is_none_or(|at| !at.is_empty() && at.chars().all(|c| c.is_ascii_digit() || c == '-'));
    Some(if !lined {
        Err(format!("`{span}` reads as no path and line"))
    } else if let Some(c) = path.chars().find(|&c| !path_char(c)) {
        Err(format!("`{span}` carries `{c}`, which no path here does"))
    } else if let Some(open) = unclosed(path) {
        Err(format!(
            "`{span}` opens a placeholder `{open}` nothing closes"
        ))
    } else {
        Ok(named)
    })
}

/// The first placeholder opener (`<`, `{`) in `path` that its segment
/// never closes.
fn unclosed(path: &str) -> Option<char> {
    path.split('/').find_map(|segment| {
        let mut open = None;
        for c in segment.chars() {
            match (open, c) {
                (None, '<' | '{') => open = Some(c),
                (Some('<'), '>') | (Some('{'), '}') => open = None,
                _ => {}
            }
        }
        open
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

    /// Whether the tree holds `path`, a pattern filled by any entry: from
    /// the root when `anchored`, else as the end of an entry's path.
    fn holds(&self, path: &str, anchored: bool) -> bool {
        let path = path.trim_end_matches('/');
        let wild = ['<', '{', '*', '…'].iter().any(|&c| path.contains(c)) || path.contains("...");
        if !wild && anchored {
            return self.entries.contains(path);
        }
        let pattern: Vec<Vec<char>> = path.split('/').map(glob).collect();
        self.entries.iter().any(|entry| {
            let names: Vec<&str> = entry.split('/').collect();
            let Some(start) = names.len().checked_sub(pattern.len()) else {
                return false;
            };
            (start == 0 || !anchored)
                && pattern
                    .iter()
                    .zip(&names[start..])
                    .all(|(segment, name)| fills(segment, &name.chars().collect::<Vec<_>>()))
        })
    }
}

/// Paths a living doc names that this repository rightly does not hold:
/// the doc (or the directory of docs, `""` for any), the path it names (or
/// the start of it), and why. An allowance that covers nothing fails, so
/// none outlives its text.
const ABSENT: [(&str, &str, &str); 20] = [
    (
        "",
        "credentials.toml",
        "a credential file the box masks in a home directory",
    ),
    (
        "",
        "CLAUDE.md",
        "Claude Code's memory file, the operator's own",
    ),
    (
        "",
        "tsconfig.json",
        "a Node repository's compiler configuration",
    ),
    (
        "",
        "*-tests.rs",
        "a test-file spelling the coverage gate reads, which no file here carries",
    ),
    (
        "",
        "alias/../",
        "a library spelling refused for climbing out of its directory",
    ),
    (
        "docs/research/github-peers/",
        "",
        "each page reads another project, whose files are its own",
    ),
    (
        "crates/brokkr-cli/dialects/speckit/",
        "specs/",
        "spec-kit's layout in the operator's repository",
    ),
    (
        "dialects/speckit/",
        "specs/",
        "spec-kit's layout in the operator's repository",
    ),
    (
        "docs/guides/contributing-by-hand.md",
        "coverage-",
        "the coverage gate's reports under `target/coverage/`",
    ),
    (
        "quality/README.md",
        "src/",
        "two files of an invented tree the example pairs",
    ),
    (
        "docs/guides/provider-adapters.md",
        "config.toml",
        "codex's own configuration",
    ),
    (
        "docs/guides/provider-adapters.md",
        "index.lock",
        "git's lock in a workspace's own `.git`",
    ),
    (
        "docs/guides/recipe-authoring.md",
        "../",
        "the spellings a recipe refuses, which never become paths",
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
        "recipes/wager-harness/README.md",
        "recipes/fast-codex",
        "names the recipe the wager deleted",
    ),
];

/// The paths from the root that `path`, named in a doc in the directory
/// `here` (`None` at the root), may be: a path that climbs with `../` is
/// resolved against `here`, and any other is read from the root and from
/// `here`, a leading `./` dropped. A path that climbs above the root is an
/// `Err`.
fn resolved(path: &str, here: Option<&str>) -> Result<Vec<String>, String> {
    let mut dir: Vec<&str> = here.map_or_else(Vec::new, |here| here.split('/').collect());
    let mut rest = path;
    let mut climbed = false;
    loop {
        if let Some(after) = rest.strip_prefix("./") {
            rest = after;
        } else if let Some(after) = rest.strip_prefix("../") {
            dir.pop()
                .ok_or_else(|| format!("`{path}` climbs above the repository"))?;
            (rest, climbed) = (after, true);
        } else {
            break;
        }
    }
    let within = |dir: &[&str]| match dir {
        [] => rest.to_string(),
        dir => format!("{}/{rest}", dir.join("/")),
    };
    Ok(match climbed {
        true => vec![within(&dir)],
        false => vec![rest.to_string(), within(&dir)],
    })
}

/// Why a code span in a doc in the directory `here` is false: it names a
/// path the tree does not hold, read as its [`Named`] says, or it cannot
/// be read. `None` when it names no path or a path the tree holds.
fn unheld(span: &str, tree: &Tree, here: Option<&str>) -> Option<String> {
    let held = named_path(span, &tree.tops)?.and_then(|named| match named {
        Named::Placed(path) => {
            resolved(path, here).map(|paths| paths.iter().any(|path| tree.holds(path, true)))
        }
        Named::Within(path) => Ok(tree.holds(path, false)),
    });
    match held {
        Ok(true) => None,
        Ok(false) => Some(format!("`{span}` names a path the tree does not hold")),
        Err(why) => Some(why),
    }
}

/// Each code span in `docs` that names a path the tree does not hold, and
/// each span that cannot be read, outside what `absent`, given [`ABSENT`],
/// allows by the span's start; and each allowance that covered nothing.
fn missing_paths(docs: &[Doc], tree: &Tree, absent: &[(&str, &str, &str)]) -> Vec<String> {
    let mut found = Vec::new();
    let mut used = vec![false; absent.len()];
    for doc in docs {
        let here = doc.path.rsplit_once('/').map(|(dir, _)| dir);
        for block in doc.blocks.iter().filter(|block| !block.fenced) {
            for (at, span) in code_spans(&block.text) {
                let Some(why) = unheld(span, tree, here) else {
                    continue;
                };
                match absent.iter().position(|(page, named, _)| {
                    doc.path.starts_with(page) && span.starts_with(named)
                }) {
                    Some(index) => used[index] = true,
                    None => found.push(format!("{}:{}: {why}", doc.path, block.line_of(at))),
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
         There is no `recipes/bun`.\n\
         \n\
         Relative: `./recipes/fast/bundle.json`, `./bundle.json`, `../fast/bundle.json`,\n\
         `../../scripts/coverage-exact.sh`, `./gone.md`, `../../scripts/gone.sh`,\n\
         `../bundle.json`, `../../../x.md`, `./bad[1].sh`.\n\
         \n\
         Unsaid: `quickstart.md`, `guides/quickstart.md`, `bundle.json:3`, `doc/guides/quickstart.md`,\n\
         `README-missing.md`, `missing.rs:main`, `owner/model`, `CHANGELOG`.\n\
         Unclosed: `docs/<guide/quickstart.md`, `recipes/{name/bundle.json`, `recipes/<name>/{x}.json`.\n\
         Foreign: `~/.cargo/credentials.toml`, `$HOME/x.md`, `/usr/x.sh`, `<repo>/x.md`,\n\
         `.forge/x.json`, `target/x.json`, `node_modules/x.json`.\n",
    )
    .unwrap();
    let root = doc("README.md", "Not `scripts/verify-seat.sh` from the root.\n").unwrap();
    let absent = [
        ("recipes/fast/", "recipes/bun", "says no such recipe ships"),
        (
            "recipes/fast/README.md",
            "recipes/go",
            "says no such recipe ships",
        ),
        ("recipes/fast/", "./bad[", "a spelling refused on purpose"),
    ];
    assert_eq!(
        missing_paths(&[planted, root], &tree, &absent),
        [
            "recipes/fast/README.md:5: `docs/target-architecture.md` names a path the tree does not hold",
            "recipes/fast/README.md:5: `recipes/<name>/policy.json` names a path the tree does not hold",
            "recipes/fast/README.md:9: `docs/guides/quick$start.md` carries `$`, which no path here does",
            "recipes/fast/README.md:9: `scripts/x.sh:main` reads as no path and line",
            "recipes/fast/README.md:15: `docs/b.md` names a path the tree does not hold",
            "recipes/fast/README.md:20: `./gone.md` names a path the tree does not hold",
            "recipes/fast/README.md:20: `../../scripts/gone.sh` names a path the tree does not hold",
            "recipes/fast/README.md:21: `../bundle.json` names a path the tree does not hold",
            "recipes/fast/README.md:21: `../../../x.md` climbs above the repository",
            "recipes/fast/README.md:23: `doc/guides/quickstart.md` names a path the tree does not hold",
            "recipes/fast/README.md:24: `README-missing.md` names a path the tree does not hold",
            "recipes/fast/README.md:24: `missing.rs:main` reads as no path and line",
            "recipes/fast/README.md:25: `docs/<guide/quickstart.md` opens a placeholder `<` nothing closes",
            "recipes/fast/README.md:25: `recipes/{name/bundle.json` opens a placeholder `{` nothing closes",
            "README.md:1: `scripts/verify-seat.sh` names a path the tree does not hold",
            "recipes/fast/README.md: allows `recipes/go`, which it no longer names",
        ]
    );
    let escapes = doc(
        "docs/guides/b.md",
        r"Two escape each other: \\`docs/two.md`; three escape it: \\\`docs/three.md`.",
    )
    .unwrap();
    let run = doc(
        "docs/guides/c.md",
        r"An escape takes one: \``docs/one.md`; of three: \```docs/run.md``.",
    )
    .unwrap();
    assert_eq!(
        missing_paths(&[escapes, run], &tree, &[]),
        [
            "docs/guides/b.md:1: `docs/two.md` names a path the tree does not hold",
            "docs/guides/c.md:1: `docs/one.md` names a path the tree does not hold",
            "docs/guides/c.md:1: `docs/run.md` names a path the tree does not hold",
        ]
    );
}

/// The comment that marks the next block's Brokkr versions illustrative: a
/// sample whose version is beside its point.
const ILLUSTRATIVE: &str = "<!-- illustrative version -->";

/// A spelling that states Brokkr's own version: the text before the
/// version, whether a [`gap`] parts the two, the characters that open it,
/// each after any whitespace, and the text that closes it.
struct Spelling {
    before: &'static str,
    gapped: bool,
    open: &'static str,
    close: &'static str,
}

/// A word a version follows past a [`gap`]: `brokkr 0.12.0`, `Brokkr
/// **0.12.0**`, `Brokkr version 0.12.0`, the word ending one line and the
/// version opening the next.
const fn word(before: &'static str) -> Spelling {
    Spelling {
        before,
        gapped: true,
        open: "",
        close: "",
    }
}

/// The text after the gap that parts a word from the version it names:
/// whitespace, line breaks included, inline markup (`*`, `_`, `` ` ``) and
/// a `version` word. `None` when nothing parts them.
fn gap(text: &str) -> Option<&str> {
    let parts = |c: char| c.is_whitespace() || "*_`".contains(c);
    let rest = text.trim_start_matches(parts);
    if rest.len() == text.len() {
        return None;
    }
    Some(match rest.strip_prefix("version") {
        Some(after) if after.starts_with(parts) => after.trim_start_matches(parts),
        _ => rest,
    })
}

/// The spellings that state Brokkr's own version. One with an opener is
/// read once the opener is; one without is read when a number follows its
/// text and, when it has a closer, its closer follows that word. A spelling
/// read but followed by no version this can read is refused.
const VERSION_SPELLINGS: [Spelling; 5] = [
    word("brokkr"),
    word("Brokkr"),
    word("engine"),
    // `"engine_version": "0.12.0"` and serde_json's compact form alike.
    Spelling {
        before: "\"engine_version\"",
        gapped: false,
        open: ":\"",
        close: "\"",
    },
    // A link that headlines a release: "The [v0.12.0 release](…)".
    Spelling {
        before: "[v",
        gapped: false,
        open: "",
        close: " release",
    },
];

/// The text after `open` at the start of `text`, any whitespace before
/// each of its characters skipped.
fn opened<'a>(text: &'a str, open: &str) -> Option<&'a str> {
    open.chars()
        .try_fold(text, |rest, c| rest.trim_start().strip_prefix(c))
}

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
    for Spelling {
        before,
        gapped,
        open,
        close,
    } in VERSION_SPELLINGS
    {
        for (at, _) in text.match_indices(before) {
            if text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '-' || c == '_') {
                continue;
            }
            let after = &text[at + before.len()..];
            let Some(rest) = (match gapped {
                true => gap(after),
                false => Some(after),
            })
            .and_then(|after| opened(after, open)) else {
                continue;
            };
            let numbered = rest
                .trim_start_matches('v')
                .starts_with(|c: char| c.is_ascii_digit());
            let word = rest.find(char::is_whitespace).unwrap_or(rest.len());
            let read = !open.is_empty() || (numbered && rest[word..].starts_with(close));
            match version(rest) {
                Some((stated, tail)) if tail.starts_with(close) => found.push((at, Ok(stated))),
                _ if read => found.push((at, Err(before))),
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
         brokkr 0.12 and engine 13/13 read as no version.\n\
         \n\
         {\"engine_version\":\"0.8.0\"}, {\"engine_version\" : \"0.12\"}, the [v0.12 release](x).\n\
         \n\
         Released as Brokkr\n\
         0.11.0 last week; brokkr  0.10.0, brokkr\t0.9.0,\n\
         Brokkr **0.8.1** and Brokkr version 0.8.2; `brokkr_core 0.1.0` and brokkr-cli 0.1.0 are none.\n",
    )
    .unwrap();
    assert_eq!(
        stale_versions(&[planted], "0.12.0"),
        [
            "docs/guides/a.md:1: states Brokkr 0.11.0; the workspace is 0.12.0",
            "docs/guides/a.md:6: states Brokkr 0.8.0; the workspace is 0.12.0",
            "docs/guides/a.md:16: states Brokkr 0.7.0; the workspace is 0.12.0",
            "docs/guides/a.md:20: `brokkr` is followed by no version this can read",
            "docs/guides/a.md:20: `engine` is followed by no version this can read",
            "docs/guides/a.md:22: states Brokkr 0.8.0; the workspace is 0.12.0",
            "docs/guides/a.md:22: `\"engine_version\"` is followed by no version this can read",
            "docs/guides/a.md:22: `[v` is followed by no version this can read",
            "docs/guides/a.md:25: states Brokkr 0.10.0; the workspace is 0.12.0",
            "docs/guides/a.md:25: states Brokkr 0.9.0; the workspace is 0.12.0",
            "docs/guides/a.md:24: states Brokkr 0.11.0; the workspace is 0.12.0",
            "docs/guides/a.md:26: states Brokkr 0.8.1; the workspace is 0.12.0",
            "docs/guides/a.md:26: states Brokkr 0.8.2; the workspace is 0.12.0",
        ]
    );
    let quoted = doc(
        "docs/guides/q.md",
        "> Released as Brokkr\n> 0.11.0 last week.\n",
    )
    .unwrap();
    assert_eq!(
        stale_versions(&[quoted], "0.12.0"),
        ["docs/guides/q.md:1: states Brokkr 0.11.0; the workspace is 0.12.0"]
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

/// A word that is a four-digit decision number, its punctuation and the
/// target of a link it is the text of (`[0069](…)`) dropped.
fn number(word: Option<&&str>) -> Option<u16> {
    let word = word?;
    let word = bare(word.split_once(']').map_or(*word, |(text, _)| text));
    (word.len() == 4 && word.chars().all(|c| c.is_ascii_digit()))
        .then(|| word.parse().expect("four digits"))
}

/// The decision numbers a list starting at `words[from]` names (`0015`,
/// `0015 and 0069`, `0015, 0016, or 0069`, `0015,0069`), and the word that
/// ends it.
fn listed(words: &[&str], from: usize) -> (Vec<u16>, Option<usize>) {
    let (mut numbers, mut last, mut at) = (Vec::new(), None, from);
    while let Some(cited) = number(words.get(at)) {
        numbers.push(cited);
        last = Some(at);
        let joined = words
            .get(at + 1)
            .is_some_and(|word| ["and", "or"].contains(&bare(word).as_str()));
        if !joined && !words[at].ends_with(',') {
            break;
        }
        at += 1 + usize::from(joined);
    }
    (numbers, last)
}

/// Whether the citation ends at `words[at]`: the word closes it (`)`, `.`,
/// `;` or `:` after its letters), or the next word opens another (`(`).
fn closes(words: &[&str], at: usize) -> bool {
    let tail = words[at].rsplit(char::is_alphanumeric).next();
    tail.is_some_and(|tail| tail.contains([')', '.', ';', ':']))
        || words.get(at + 1).is_some_and(|next| next.starts_with('('))
}

/// Each decision `proposed` at `words[at]` names after it: `proposed
/// [decision|decisions] NNNN` or `proposed by|in|for [decision|decisions]
/// NNNN`, each number of a [`listed`] run, and none past a word that
/// [`closes`] the citation.
fn prefixed_proposed(words: &[&str], at: usize) -> Vec<u16> {
    if closes(words, at) {
        return Vec::new();
    }
    let after = |skip: usize| words.get(at + skip).map(|word| bare(word));
    let mut next = 1;
    if after(next).is_some_and(|word| ["by", "in", "for"].contains(&word.as_str())) {
        next += 1;
    }
    if after(next).is_some_and(|word| word == "decision" || word == "decisions") {
        next += 1;
    }
    listed(words, at + next).0
}

/// Each decision a list after `decision` or `decisions` at `words[at]`
/// calls proposed by a trailing `, proposed` that proposes nothing else:
/// one [`prefixed_proposed`] reads no decision after.
fn trailed_proposed(words: &[&str], at: usize) -> Vec<u16> {
    let (cited, Some(last)) = listed(words, at + 1) else {
        return Vec::new();
    };
    let trailed = words[last].ends_with(',')
        && words
            .get(last + 1)
            .is_some_and(|word| bare(word) == "proposed")
        && prefixed_proposed(words, last + 1).is_empty();
    if trailed {
        cited
    } else {
        Vec::new()
    }
}

/// Each decision a text calls proposed: a [`prefixed_proposed`] citation
/// or a [`trailed_proposed`] one, in any case and across line breaks. A
/// comma parts words as a space does, so `0056,0069` lists two.
fn called_proposed(text: &str) -> Vec<u16> {
    let words: Vec<&str> = text
        .split_whitespace()
        .flat_map(|word| word.split_inclusive(','))
        .collect();
    let mut found = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let word = bare(word);
        if word == "proposed" {
            found.extend(prefixed_proposed(&words, at));
        } else if word == "decision" || word == "decisions" {
            found.extend(trailed_proposed(&words, at));
        }
    }
    found
}

/// The comment a shell line carries: the text after the first `#` that
/// opens a word, unless it is a shebang. Quotes are not read, so a `#`
/// opening a word inside them starts a comment too, which can only read
/// more.
fn shell_comment(line: &str) -> Option<&str> {
    let (at, _) = line.char_indices().find(|&(at, c)| {
        c == '#'
            && line[..at]
                .chars()
                .next_back()
                .is_none_or(char::is_whitespace)
    })?;
    let comment = &line[at + 1..];
    (!comment.starts_with('!')).then_some(comment)
}

/// Each string a JSON text holds, its keys among them, decoded, with the
/// line it opens on; a text that is not JSON is refused by its parser.
fn json_strings(text: &str) -> Result<Vec<(usize, String)>, serde_json::Error> {
    serde_json::from_str::<serde::de::IgnoredAny>(text)?;
    let (mut found, mut line, mut open, mut escaped) = (Vec::new(), 1, None, false);
    for (at, c) in text.char_indices() {
        match (open, c) {
            (None, '"') => open = Some(at),
            (None, '\n') => line += 1,
            (None, _) => {}
            (Some(_), _) if escaped => escaped = false,
            (Some(_), '\\') => escaped = true,
            (Some(start), '"') => {
                found.push((line, serde_json::from_str(&text[start..=at])?));
                open = None;
            }
            (Some(_), _) => {}
        }
    }
    Ok(found)
}

/// The prose of a tracked file these rules hold: a doc's blocks, each
/// string of a JSON text alone, a Rust source's comments, a script's
/// [`shell_comment`]s, each a run of adjacent lines with the line it
/// starts on.
fn prose(path: &str, text: &str) -> Result<Vec<(usize, String)>, String> {
    let lines: Vec<(usize, String)> = match path.rsplit_once('.').map(|(_, ext)| ext) {
        Some("md") => {
            return Ok(blocks(text)?
                .into_iter()
                .map(|block| (block.first, block.text))
                .collect());
        }
        Some("json") => return json_strings(text).map_err(|why| why.to_string()),
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
            .filter_map(|(line, at)| Some((at, shell_comment(line)?.to_string())))
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
    let texts: Vec<(String, String)> =
        tracked_files::tracked(&root, &["*.md", "*.rs", "*.sh", "*.json"])
            .into_iter()
            .filter(|path| is_living(path))
            .map(|path| {
                let text = read(&path);
                (path, text)
            })
            .collect();
    for known in [
        "README.md",
        "adapters/codex.json",
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
        "a living doc, a source comment or a JSON string calls a decision proposed that the index does not"
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
             ```\n\
             \n\
             proposed decision [0069](0069-x.md); (decision [0069](x), proposed).\n\
             \n\
             proposed decisions 0056 and 0069; (decisions 0075, 0056, and 0069, proposed).\n\
             \n\
             (decision 0069, proposed decisions 0056 and 0075) (decision 0069, proposed by\n\
             decision 0056) (decision 0069, proposed in 0075) (decision 0069, proposed by us).\n\
             \n\
             (decision 0069, proposed). Decision 0075 describes the stamp.\n\
             \n\
             (decision 0069, proposed) (decision 0075).\n\
             \n\
             Decision 0069, proposed (decision 0075 describes the stamp).\n\
             \n\
             > A proposed decision\n\
             > 0069; (decision 0069,\n\
             > proposed); proposed decisions 0056 and\n\
             > 0069.\n\
             \n\
             proposed decisions 0056,0069; (decisions 0075,0069, proposed).\n",
        ),
        (
            "scripts/a.sh",
            "#!/bin/bash\n# Proposed by decision 0069.\necho proposed decision 0069\n\
             true # proposed decision 0069\necho \"#a\" proposed decision 0069 b#c\n",
        ),
        (
            "crates/a/src/open.rs",
            "fn f() { let s = \"never closed; }\n",
        ),
        (
            "adapters/a.json",
            "{\"notes\": [\"motivate it (proposed\",\n\
             \"decision 0069)\", \"a \\\"proposed decision 0069\\\" quote\"],\n\
             \"proposed decision 0056\": \"a\\\\\", \"b\": \"(decision 0069, proposed)\"}\n",
        ),
        ("adapters/open.json", "{\"a\": \"proposed decision 0069\""),
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
            "docs/a.md:10: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:10: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:12: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:12: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:14: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:17: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:19: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:21: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:23: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:23: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:23: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:28: calls decision 0069 proposed; the index rules it accepted",
            "docs/a.md:28: calls decision 0069 proposed; the index rules it accepted",
            "scripts/a.sh:2: calls decision 0069 proposed; the index rules it accepted",
            "scripts/a.sh:4: calls decision 0069 proposed; the index rules it accepted",
            "crates/a/src/open.rs: cannot be read: an unterminated string",
            "adapters/a.json:2: calls decision 0069 proposed; the index rules it accepted",
            "adapters/a.json:3: calls decision 0069 proposed; the index rules it accepted",
            "adapters/open.json: cannot be read: EOF while parsing an object at line 1 column 30",
        ]
    );
}

/// A term the house retired, by each spelling it is written in, why, and
/// the citation that lets a block name it: a block that cites what retired
/// it says so, and is read true.
struct Retired {
    spellings: &'static [&'static str],
    why: &'static str,
    cite: Option<&'static str>,
}

/// The decision that retired the third host.
const NO_WINDOWS: &str = "decision 0063: the hosts are Linux and macOS";

/// Never built, as `docs/history/target-architecture.md`'s status records.
const NEVER_BUILT: &str = "never built (#364): the DeepSeek harness is the built-in dsh adapter";

const RETIRED: [Retired; 8] = [
    // Lowercase `windows` alone is a context window; opening a hyphenated
    // word it is a runner label (`windows-latest`, `windows-2022`).
    Retired {
        spellings: &["Windows", "WINDOWS", "windows-"],
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        spellings: &["winget", "Winget", "WinGet", "WINGET"],
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    Retired {
        spellings: &["scoop", "Scoop"],
        why: NO_WINDOWS,
        cite: Some("0063"),
    },
    // The Cordis/DeepSeek harness; Cordis stays dsh's own framework, which
    // [`ALLOWED`] lets the dsh extension name.
    Retired {
        spellings: &["Cordis"],
        why: NEVER_BUILT,
        cite: None,
    },
    Retired {
        spellings: &["Surface"],
        why: NEVER_BUILT,
        cite: None,
    },
    Retired {
        spellings: &["brokkr-api"],
        why: "never built (#364): `brokkr ui` serves the console from brokkr-cli",
        cite: None,
    },
    Retired {
        spellings: &["policy/schemas"],
        why: "removed: `policy/` holds the production table alone",
        cite: None,
    },
    Retired {
        spellings: &["bundles/"],
        why: "decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
        cite: None,
    },
];

/// Living docs that may name a retired term, each with its reason. An
/// allowance that covers nothing fails, so none outlives its text.
const ALLOWED: [(&str, &str, &str); 2] = [
    (
        "extensions/dsh/",
        "Cordis",
        "dsh's own framework, which the vendored extension builds on",
    ),
    (
        "docs/research/",
        "Windows",
        "a research entry reads another project, whose hosts are its own",
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
    for Retired {
        spellings,
        why,
        cite,
    } in RETIRED
    {
        if cite.is_some_and(|cite| text.contains(cite)) {
            continue;
        }
        for &term in spellings {
            found.extend(
                text.match_indices(term)
                    .filter(|&(at, _)| whole_word(text, at, term))
                    .map(|(at, _)| (at, term, why)),
            );
        }
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
         Context windows, a read surface, recipes/my-bundles/ and cordis.patch.yml are not.\n\
         \n\
         WINDOWS hosts, Winget, WinGet, WINGET, Scoop and a Cordis hook.\n\
         \n\
         runs-on: windows-latest; not context windows, context-windows-wide or 356-windows-is.\n",
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
            "docs/guides/a.md:5: names `Cordis`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:5: names `Surface`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:5: names `brokkr-api`, retired: never built (#364): `brokkr ui` serves the console from brokkr-cli",
            "docs/guides/a.md:6: names `policy/schemas`, retired: removed: `policy/` holds the production table alone",
            "docs/guides/a.md:6: names `bundles/`, retired: decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
            "docs/guides/a.md:6: names `bundles/`, retired: decision 0078 moved Brokkr's own bundles into `recipes/` (#359)",
            "docs/guides/a.md:10: names `WINDOWS`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `Winget`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `WinGet`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `WINGET`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `Scoop`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/a.md:10: names `Cordis`, retired: never built (#364): the DeepSeek harness is the built-in dsh adapter",
            "docs/guides/a.md:12: names `windows-`, retired: decision 0063: the hosts are Linux and macOS",
            "docs/guides/b.md: allows `winget`, which it no longer names",
        ]
    );
}
