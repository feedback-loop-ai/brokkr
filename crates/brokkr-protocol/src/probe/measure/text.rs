//! What a line of text, or a string an event holds, says of the MCP
//! servers and tools that reached a turn (#484). The reader is
//! closed-world, one rule for stdout, transcripts and stderr: a string
//! that names another MCP server is a reach, and any other is harmless
//! only when, normalised, it is one of [`FORMS`], the probe's own prompt
//! or the reply it asks for, whole. Everything else is unread, so neither
//! JSON syntax, case nor punctuation makes a disclosure harmless. Pure,
//! like `measure`.

use serde_json::{Map, Value};

use super::tools::tool_server;
use super::SESSION_KEYS;
use crate::hands::SERVER_NAME;
use crate::probe::plan::{PROMPT, REPLY, USER_SCOPE_SERVER};

/// What a line of text, or a string an event holds, says of the MCP
/// servers and tools that reached a turn.
#[derive(PartialEq)]
pub(super) enum Said {
    /// It is blank, one of [`FORMS`], the prompt or the reply.
    Nothing,
    /// It names an MCP server other than the hands server, as this says.
    Reach(String),
    /// It is anything else, or starts JSON it does not hold whole.
    Unread,
}

/// The levels a CLI opens a warning with.
const LEVELS: [&str; 3] = ["warn", "warning", "error"];

/// The statuses a CLI reports the hands server by.
const STATUSES: [&str; 2] = ["connected", "failed"];

/// One word of a recognised form, normalised.
#[derive(Clone, Copy)]
enum Word {
    /// This word.
    Is(&'static str),
    /// The hands server's name.
    Hands,
    /// A tool of the hands server, spelled whole.
    HandsTool,
    /// A count, in digits.
    Count,
    /// One of [`STATUSES`].
    Status,
    /// A long flag, as a CLI names one it does not know.
    Flag,
}

/// The closed table of harmless forms, each matched word for word to the
/// line's end after one opening level of [`LEVELS`]: every name a form
/// holds is the hands server's, or a flag the CLI refused. The empty form
/// is a level alone.
const FORMS: [&[Word]; 6] = [
    &[],
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
    &[
        Word::Is("no"),
        Word::Is("stdin"),
        Word::Is("data"),
        Word::Is("received"),
    ],
];

/// The keys whose string a stream tags its events, messages and sessions
/// by. A tag spelled as one name is read as a tag; any other string under
/// one is read like text.
const TAG_KEYS: [&str; 4] = ["type", "subtype", "role", "id"];

/// `text`'s words: split on whitespace, each with its edge punctuation
/// trimmed and its case folded, the empty ones dropped.
fn normalised(text: &str) -> Vec<String> {
    let edge = |c: char| !(c.is_alphanumeric() || matches!(c, '-' | '_'));
    text.split_whitespace()
        .map(|word| word.trim_matches(edge).to_lowercase())
        .filter(|word| !word.is_empty())
        .collect()
}

/// What `text` says: a reach when it names the planted server, a tool of
/// another server or another `MCP server <name>`; else unread when it
/// starts JSON, and read only when it is blank, one of [`FORMS`], the
/// prompt or the reply, its words normalised.
pub(super) fn said(text: &str) -> Said {
    let words = normalised(text);
    if let Some(reach) = reach(text, &words) {
        return Said::Reach(reach);
    }
    let whole = |known: &str| words == normalised(known);
    if text.trim_start().starts_with(['{', '[']) {
        Said::Unread
    } else if recognised(&words) || whole(PROMPT) || whole(REPLY) {
        Said::Nothing
    } else {
        Said::Unread
    }
}

/// A name as a line spells it: letters, digits, `_`, `-` and `.`.
fn spelled(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .unwrap_or(text.len());
    &text[..end]
}

/// The first MCP server other than the hands server that `text` names:
/// the planted one anywhere, in any case, then each tool its `words`
/// spell `mcp__<server>__…` and each `MCP server <name>` among them.
fn reach(text: &str, words: &[String]) -> Option<String> {
    if text.to_lowercase().contains(USER_SCOPE_SERVER) {
        return Some(format!(
            "the planted user-scope MCP server {USER_SCOPE_SERVER}"
        ));
    }
    let tools = words.iter().flat_map(|word| {
        word.match_indices("mcp__").filter_map(|(at, _)| {
            let tool = spelled(&word[at..]);
            let server = tool_server(tool).filter(|server| *server != SERVER_NAME)?;
            Some(format!("{tool} of the MCP server {server}"))
        })
    });
    let servers = words.windows(3).filter_map(|three| {
        let named = three[0] == "mcp" && three[1] == "server" && three[2] != SERVER_NAME;
        named.then(|| format!("the MCP server {}", three[2]))
    });
    tools.chain(servers).next()
}

/// Whether `words`, less one opening level, are one of [`FORMS`] word
/// for word.
fn recognised(words: &[String]) -> bool {
    let body = match words.split_first() {
        Some((level, rest)) if LEVELS.contains(&level.as_str()) => rest,
        _ => words,
    };
    FORMS.iter().any(|form| {
        form.len() == body.len()
            && form
                .iter()
                .zip(body)
                .all(|(want, word)| matches(*want, word))
    })
}

fn matches(want: Word, word: &str) -> bool {
    match want {
        Word::Is(expected) => word == expected,
        Word::Hands => word == SERVER_NAME,
        Word::HandsTool => spelled(word) == word && tool_server(word) == Some(SERVER_NAME),
        Word::Count => word.bytes().all(|byte| byte.is_ascii_digit()),
        Word::Status => STATUSES.contains(&word),
        Word::Flag => word.starts_with("--") && spelled(word) == word,
    }
}

/// Where a string sits in an event: free, or in a listing of tools or of
/// MCP servers, whose reader consumes the names and statuses it holds.
#[derive(Clone, Copy, PartialEq)]
enum Within {
    Free,
    Tools,
    Servers,
}

/// What every string of one event says, by the one rule for text: a
/// string a listing reader consumes is its reader's, a tag spelled as one
/// name is a tag, and any other string, under whatever key, is read by
/// [`said`]. The first reach, and whether any string went unread.
pub(super) fn heard(fields: &Map<String, Value>) -> (Option<String>, bool) {
    let mut each = Vec::new();
    object(fields, Within::Free, &mut each);
    let reach = each.iter().find_map(|said| match said {
        Said::Reach(reach) => Some(reach.clone()),
        Said::Nothing | Said::Unread => None,
    });
    (reach, each.contains(&Said::Unread))
}

/// Each string of `fields`, an object `within` a listing or free.
fn object(fields: &Map<String, Value>, within: Within, each: &mut Vec<Said>) {
    for (key, value) in fields {
        let listing = listed_under(key);
        let consumed = listing != Within::Free
            || match within {
                Within::Tools => key == "name",
                Within::Servers => key == "name" || key == "status",
                Within::Free => false,
            };
        match value {
            Value::String(_) if consumed => {}
            Value::String(text) if is_tag(key, text) => each.extend(reach_only(text)),
            Value::String(text) => each.push(said(text)),
            other => nested(other, listing, each),
        }
    }
}

/// What a listing under `key` holds, if `key` names one.
fn listed_under(key: &str) -> Within {
    match key {
        "tools" => Within::Tools,
        "mcp_servers" => Within::Servers,
        _ => Within::Free,
    }
}

/// Each string inside `value`, the entries of a listing `within` it: a
/// string a listing holds is its reader's, which names it or reports it
/// unread.
fn nested(value: &Value, within: Within, each: &mut Vec<Said>) {
    match value {
        Value::Object(fields) => object(fields, within, each),
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(_) if within != Within::Free => {}
                    Value::String(text) => each.push(said(text)),
                    other => nested(other, within, each),
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// Whether `text`, under `key`, is a tag: a session id or one of
/// [`TAG_KEYS`], spelled as one name.
fn is_tag(key: &str, text: &str) -> bool {
    (TAG_KEYS.contains(&key) || SESSION_KEYS.contains(&key)) && spelled(text) == text
}

/// A tag's reach, when it names another MCP server.
fn reach_only(text: &str) -> Option<Said> {
    reach(text, &normalised(text)).map(Said::Reach)
}
