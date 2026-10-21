//! Issue #365: the living docs are held to what is true, not to an
//! inventory of what they once said. A section a page loses is a
//! reviewer's call; a false claim is this file's failure.
//!
//! A living doc is a tracked Markdown file that `status_pages::is_record`
//! does not call a record, outside the [`FROZEN`] surfaces. Each is read as
//! blocks: a paragraph, list or table between lines blank inside their
//! blockquote markers, a deeper quote opening its own, or a fenced
//! block whole, in a blockquote or not, closed only by a line of its own
//! fence character at least as long and nothing else. Four rules hold them:
//!
//! - every repository path a code span outside a fence names is in the
//!   tree, a `./` or `../` spelling resolved against the doc's directory,
//!   and a file named from no directory found under any (lychee holds the
//!   links; a fenced sample is #365's snapshot ask, so this rule leaves it
//!   as the version rule does not);
//! - every version of Brokkr a doc states is the workspace's, unless its
//!   block follows the [`versions::ILLUSTRATIVE`] mark;
//! - every decision a doc, a source comment or a JSON string calls
//!   proposed is proposed in `docs/decisions/README.md`;
//! - no retired term stands in a block that does not cite what retired it,
//!   outside the paths [`retired::ALLOWED`] names with a reason.
//!
//! What a rule cannot read is refused, never skipped: a fence left open
//! or ended by its blockquote, a path written with a character no path here carries, a placeholder
//! nothing closes or climbing above the root, a version spelling followed
//! by no version, a source whose
//! comment or literal never closes. A backtick nothing closes is literal
//! text, as CommonMark renders it, not a span.

use std::fmt;

use crate::records::FROZEN;
use crate::status_pages;
use crate::tracked_files;
use crate::workspace_root::{read, workspace};

mod citations;
mod paths;
mod retired;
mod versions;

/// Why a text, or a word in it, cannot be read.
#[derive(Debug)]
enum Unread {
    /// A fence nothing closes before the text ends.
    Open(Fence),
    /// A fence its blockquote ends before a line closes it.
    Unquoted(Fence),
    /// A path span whose `:` suffix is no line.
    NoLine(String),
    /// A path span with a character no path here carries.
    Carries(String, char),
    /// A path span with a placeholder its segment never closes.
    Placeholder(String, char),
    /// A path that climbs above the root.
    Climbs(String),
    /// A Rust source the comment reader refuses, by its reason.
    Source(String),
    /// A JSON text its parser refuses.
    Json(serde_json::Error),
}

impl fmt::Display for Unread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(fence) => write!(f, "a block fenced with {} never closes", fence.spelled()),
            Self::Unquoted(fence) => write!(
                f,
                "a block fenced with {} ends with its blockquote, unclosed",
                fence.spelled()
            ),
            Self::NoLine(span) => write!(f, "`{span}` reads as no path and line"),
            Self::Carries(span, c) => write!(f, "`{span}` carries `{c}`, which no path here does"),
            Self::Placeholder(span, open) => {
                write!(f, "`{span}` opens a placeholder `{open}` nothing closes")
            }
            Self::Climbs(path) => write!(f, "`{path}` climbs above the repository"),
            Self::Source(why) => write!(f, "{why}"),
            Self::Json(why) => write!(f, "{why}"),
        }
    }
}

/// A tracked file these rules cannot read, and why.
#[derive(Debug)]
struct Unreadable {
    path: String,
    why: Unread,
}

impl fmt::Display for Unreadable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: cannot be read: {}", self.path, self.why)
    }
}

/// Whether the tracked file at `path` is living text these rules hold:
/// no record, and none of the [`FROZEN`] surfaces, which a rule here could
/// only report.
fn is_living(path: &str) -> bool {
    !status_pages::is_record(path) && !FROZEN.iter().any(|(frozen, _)| path.starts_with(frozen))
}

/// A paragraph, list or table between blank lines, as [`blocks`] reads
/// them, or the inside of a fenced block: its text and the line its text
/// starts on.
pub(crate) struct Block {
    pub(crate) first: usize,
    pub(crate) text: String,
    pub(crate) fenced: bool,
}

impl Block {
    /// The line the byte at `at` of the block's text stands on.
    fn line_of(&self, at: usize) -> usize {
        self.first + self.text[..at].matches('\n').count()
    }
}

/// A living doc, read into blocks.
pub(crate) struct Doc {
    pub(crate) path: String,
    pub(crate) blocks: Vec<Block>,
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
#[derive(Clone, Copy, Debug)]
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
/// markers, or why it cannot be read. A line is blank by its text inside
/// those markers, so a bare `>` parts two blocks of one quote, and a line
/// quoted deeper than the open block opens one of its own, as CommonMark's
/// blockquote interrupts a paragraph; a line quoted less runs on lazily.
fn blocks(text: &str) -> Result<Vec<Block>, Unread> {
    let mut blocks: Vec<Block> = Vec::new();
    let (mut fence, mut open) = (None::<Fence>, None::<usize>);
    for (at, line) in text.lines().enumerate() {
        if let Some(opened) = fence {
            let (depth, inside) = unquoted(line, opened.depth);
            if depth < opened.depth {
                return Err(Unread::Unquoted(opened));
            }
            match opened.closed_by(inside) {
                true => fence = None,
                false => push_line(blocks.last_mut().expect("the fenced block"), inside),
            }
            continue;
        }
        let (depth, inside) = unquoted(line, usize::MAX);
        if let Some(opened) = Fence::opened_by(inside, depth) {
            (fence, open) = (Some(opened), None);
            blocks.push(Block {
                first: at + 2,
                text: String::new(),
                fenced: true,
            });
        } else if inside.trim().is_empty() {
            open = None;
        } else if open.is_some_and(|open| depth <= open) {
            push_line(blocks.last_mut().expect("an open block"), inside);
        } else {
            open = Some(depth);
            blocks.push(Block {
                first: at + 1,
                text: inside.to_string(),
                fenced: false,
            });
        }
    }
    match fence {
        Some(opened) => Err(Unread::Open(opened)),
        None => Ok(blocks),
    }
}

/// A doc as these rules read it, or why it cannot be read.
fn doc(path: &str, text: &str) -> Result<Doc, Unreadable> {
    let path = path.to_string();
    match blocks(text) {
        Ok(blocks) => Ok(Doc { path, blocks }),
        Err(why) => Err(Unreadable { path, why }),
    }
}

/// Every living doc; one that cannot be read is an `Err`.
fn living_docs() -> Vec<Result<Doc, Unreadable>> {
    tracked_files::tracked(&workspace(), &["*.md"])
        .into_iter()
        .filter(|path| is_living(path))
        .map(|path| doc(&path, &read(&path)))
        .collect()
}

/// The living docs that can be read; `every_living_doc_is_read` refuses
/// the rest.
pub(crate) fn readable_docs() -> Vec<Doc> {
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
    let unreadable: Vec<String> = unreadable
        .into_iter()
        .filter_map(|doc| doc.err().map(|why| why.to_string()))
        .collect();
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
        doc("docs/a.md", "text\n\n```sh\nnever closed\n")
            .err()
            .map(|why| why.to_string()),
        Some("docs/a.md: cannot be read: a block fenced with ``` never closes".to_string())
    );
}

#[test]
fn a_fence_closes_only_on_its_own_marker_inside_its_quote() {
    let read = |text: &str| {
        blocks(text)
            .map(|blocks| {
                blocks
                    .into_iter()
                    .map(|block| (block.first, block.text, block.fenced))
                    .collect::<Vec<_>>()
            })
            .map_err(|why| why.to_string())
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
            (1, "Quoted:".to_string(), false),
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

/// Each allowance, `(path, what, why)`, that covered nothing.
fn stale_allowances(allowed: &[(&str, &str, &str)], used: &[bool]) -> Vec<String> {
    allowed
        .iter()
        .zip(used)
        .filter(|(_, used)| !**used)
        .map(|((path, what, _), _)| format!("{path}: allows `{what}`, which it no longer names"))
        .collect()
}

/// Whether a backslash escapes the character at `at` of `text`: an odd run
/// of them stands before it, since two escape each other.
fn escaped(text: &str, at: usize) -> bool {
    text[..at].chars().rev().take_while(|&c| c == '\\').count() % 2 == 1
}
