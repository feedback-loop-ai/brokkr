//! What a line of stderr text says of the MCP servers and tools that
//! reached a turn (#484). The scanner reads a line as harmless only when
//! it recognises the line whole: a line that names another MCP server is
//! a reach, and one that mentions MCP or a tool, or names a tool the
//! plain turn listed or the adapter declares, is unread unless it is one
//! of [`FORMS`], read to its end. Pure, like `measure`.

use super::tools::tool_server;
use crate::hands::SERVER_NAME;
use crate::probe::plan::USER_SCOPE_SERVER;

/// What a line of text says of the MCP servers and tools that reached a
/// turn.
pub(super) enum Said {
    /// It mentions no MCP server or tool, or it is one of [`FORMS`].
    Nothing,
    /// It names an MCP server other than the hands server, as this says.
    Reach(String),
    /// It mentions MCP or a tool in no form the scanner reads whole, or
    /// starts JSON it does not hold whole.
    Unread,
}

/// The levels a CLI opens a warning with, folded to lowercase.
const LEVELS: [&str; 3] = ["warn:", "warning:", "error:"];

/// The statuses a CLI reports the hands server by.
const STATUSES: [&str; 2] = ["connected", "failed"];

/// One word of a recognised form.
#[derive(Clone, Copy)]
enum Word {
    /// This word, in any case.
    Is(&'static str),
    /// The hands server's name.
    Hands,
    /// A tool of the hands server, spelled whole.
    HandsTool,
    /// A count, in digits.
    Count,
    /// One of [`STATUSES`].
    Status,
    /// A flag in single quotes, as a CLI names one it does not know.
    Flag,
}

/// The forms a line that mentions MCP or a tool is read by, each matched
/// word for word to the line's end, after one opening level of
/// [`LEVELS`]: every name a form holds is the hands server's, or a flag
/// the CLI refused.
const FORMS: [&[Word]; 4] = [
    &[Word::Is("unknown"), Word::Is("option"), Word::Flag],
    &[
        Word::Is("mcp"),
        Word::Is("server"),
        Word::Hands,
        Word::Status,
    ],
    &[
        Word::Is("only"),
        Word::HandsTool,
        Word::Is("is"),
        Word::Is("allowed"),
    ],
    &[Word::Count, Word::Is("tools"), Word::Is("available")],
];

/// What `text` says, its whitespace runs folded to one space: a reach
/// when it names the planted server, a tool of another server or another
/// `MCP server <name>`; else unread when it starts JSON, or mentions MCP,
/// a tool or one of `tools` in no form of [`FORMS`].
pub(super) fn said(text: &str, tools: &[String]) -> Said {
    let line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if let Some(reach) = reach(&line) {
        return Said::Reach(reach);
    }
    let lower = line.to_ascii_lowercase();
    let mentions = lower.contains("mcp")
        || lower.contains("tool")
        || words(&line).any(|word| tools.iter().any(|tool| tool == word));
    if line.starts_with(['{', '[']) || (mentions && !recognised(&line)) {
        Said::Unread
    } else {
        Said::Nothing
    }
}

/// A name as a line spells it: letters, digits, `_`, `-` and `.`.
fn spelled(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .unwrap_or(text.len());
    &text[..end]
}

/// Every name `line` spells.
fn words(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
}

/// The first MCP server other than the hands server that `line` names:
/// the planted one anywhere, then each tool it spells `mcp__<server>__…`
/// and each `MCP server <name>`, every one of them checked.
fn reach(line: &str) -> Option<String> {
    if line.contains(USER_SCOPE_SERVER) {
        return Some(format!(
            "the planted user-scope MCP server {USER_SCOPE_SERVER}"
        ));
    }
    let tools = line.match_indices("mcp__").filter_map(|(at, _)| {
        let tool = spelled(&line[at..]);
        let server = tool_server(tool).filter(|server| *server != SERVER_NAME)?;
        Some(format!("{tool} of the MCP server {server}"))
    });
    let lower = line.to_ascii_lowercase();
    let servers = lower.match_indices("mcp server ").filter_map(|(at, word)| {
        let name = spelled(&line[at + word.len()..]);
        (!name.is_empty() && name != SERVER_NAME).then(|| format!("the MCP server {name}"))
    });
    tools.chain(servers).next()
}

/// Whether `line`, less one opening level, is one of [`FORMS`] word for
/// word.
fn recognised(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let body = match LEVELS.iter().find(|level| lower.starts_with(*level)) {
        Some(level) => line[level.len()..].trim_start(),
        None => line,
    };
    let words: Vec<&str> = body.split(' ').collect();
    FORMS.iter().any(|form| {
        form.len() == words.len()
            && form
                .iter()
                .zip(&words)
                .all(|(want, word)| matches(*want, word))
    })
}

fn matches(want: Word, word: &str) -> bool {
    match want {
        Word::Is(expected) => word.eq_ignore_ascii_case(expected),
        Word::Hands => word == SERVER_NAME,
        Word::HandsTool => spelled(word) == word && tool_server(word) == Some(SERVER_NAME),
        Word::Count => word.bytes().all(|byte| byte.is_ascii_digit()),
        Word::Status => STATUSES.contains(&word),
        Word::Flag => word
            .strip_prefix('\'')
            .and_then(|word| word.strip_suffix('\''))
            .is_some_and(|flag| flag.starts_with("--") && spelled(flag) == flag),
    }
}
