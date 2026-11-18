//! Claude's MCP client log, from 2.1.326: one file for each MCP server
//! claude starts, whose path names the server. The file is the listing;
//! its lines are inert.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::read::{decode, Decoded, Given, Harness, Said, Server};
use super::{Event, Fault, Lines, Stream};

/// How a transcript at `path` is read, and the server it is the MCP
/// client log of, when `harness` writes one.
pub(super) fn reading(harness: Harness, path: &str) -> (Lines, Option<String>) {
    let server = match harness {
        Harness::Claude => server_at(path),
        Harness::Codex | Harness::Dsh => None,
    };
    let lines = if server.is_some() {
        Lines::McpLog
    } else {
        Lines::Transcript
    };
    (lines, server)
}

/// A log file of `server` lists the server as reaching the turn, as an
/// event before the first of its lines, which is `line`.
pub(super) fn list_in(stream: &mut Stream, server: Option<String>, line: usize) {
    if let Some(name) = server {
        let servers = vec![Server {
            name,
            status: "logged",
        }];
        let event = Event {
            line,
            label: "mcp-log-file".to_string(),
            said: vec![Said::Servers { at: "/", servers }],
        };
        stream.events.insert(0, event);
    }
}

/// The MCP server a file of claude's is the client log of, from 2.1.326:
/// claude writes one for each server it starts,
/// `~/.cache/claude-cli-nodejs/<cwd>/mcp-logs-<server>/<when>.jsonl`, so
/// the file is that server reaching the turn, whatever its lines say of
/// how the connection went.
fn server_at(path: &str) -> Option<String> {
    let rest = path.strip_prefix("~/.cache/claude-cli-nodejs/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.as_slice() {
        [_cwd, dir, file] if file.ends_with(".jsonl") => {
            dir.strip_prefix("mcp-logs-").map(str::to_string)
        }
        _ => None,
    }
}

/// A line of claude's MCP client log. Inert, every key: the client's own
/// account of the server its file is named after, which that name already
/// lists.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct McpLogLine {
    #[serde(rename = "cwd")]
    _cwd: String,
    #[serde(rename = "sessionId")]
    _session: String,
    #[serde(rename = "timestamp")]
    _timestamp: String,
    #[serde(default, rename = "debug")]
    _debug: Given<String>,
    #[serde(default, rename = "error")]
    _error: Given<String>,
}

/// One line of claude's MCP client log, decoded, or why it is unread.
pub(super) fn row(fields: Map<String, Value>) -> Result<Decoded, Fault> {
    let _: McpLogLine = decode(Harness::Claude, fields)?;
    Ok(Decoded {
        label: "mcp-log".to_string(),
        said: Vec::new(),
    })
}
