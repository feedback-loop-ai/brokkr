//! The broker's filtered MCP protocol (decision 0065 slice two, U6e; MB3,
//! CC2, CR3, SC4): MCP 2025-06-18 over newline-delimited JSON-RPC 2.0,
//! spoken with the admitted box's server over its only stdio and nothing
//! else, with no unboxed child, shell or reconnect beside it. [`started`]
//! initializes the server within the startup's absolute deadline: it is
//! asked to initialize at MB3's version, its version is checked against
//! the dialect's pinned one, it is told it is initialized, and its whole
//! catalog is listed and filtered to the plan's grant, every granted tool
//! offered and no name listed twice. Every frame is read by its keys, an
//! object, never by position. A member read through [`present`], such as a
//! frame's id, method, result or error, is refused where it is an explicit
//! null, never taken as absent; a frame's params, an error's data and a
//! page's next cursor read a null as absent. A response must answer the
//! one request outstanding, by its exact typed id: a number and a string
//! are never the same id, and any other response ends the session. A frame
//! that answers nothing is never the harness's: the server's ping is
//! answered, any other request it makes refused, and a notification
//! dropped, each spending the operation's fixed budget of non-response
//! bytes, which never extends its deadline. What the broker sends the
//! server is queued, whole frames in order, and written as far as the
//! server takes it whenever its output is read, so no write waits on it. A
//! fatal ending is the session's first cause ([`Fatal`]), latched through
//! the ledger's own catalogue. The harness-facing call path is
//! [`calls`]'s; until U6f, `serve` refuses before any message reaches the
//! harness.

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsFd;

use brokkr_protocol::broker::ledger::Failure;
use brokkr_protocol::broker::Plan;
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Map, Value};

use super::session::{ensure, polled, Ended, Link, Startup};

mod calls;

#[cfg(test)]
mod tests;

/// The one protocol version admitted (MB3).
const VERSION: &str = "2025-06-18";

/// The most bytes one harness request holds (MB3).
const REQUEST_MAX: usize = 1 << 20;

/// The most bytes one server response holds (MB3).
const RESPONSE_MAX: usize = 8 << 20;

/// The bytes of frames that answer nothing one operation hears at most
/// (MB3).
const NOTICES_MAX: usize = 1 << 20;

/// The deepest either side's JSON nests (MB3).
const DEPTH_MAX: usize = 64;

/// The most tools a catalog lists, over at most so many pages (MB3).
const CATALOG_MAX: usize = 1024;
const PAGES_MAX: usize = 32;

/// The most bytes one read takes from a pipe.
const CHUNK: usize = 64 << 10;

const PROTOCOL: Fatal = Fatal::Failed(Failure::Protocol);
const RESPONSE: Fatal = Fatal::Failed(Failure::Response);
const TIMEOUT: Fatal = Fatal::Failed(Failure::Timeout);

/// Why a served session ends (MB3, CR3), each latched as its closure's
/// first cause through the ledger's own catalogue ([`Fatal::failure`]): a
/// cause the catalogue names, or one of MB3's own that the operator ruled
/// latched under it (2026-10-11): the server's protocol version, a granted
/// tool its complete listing lacks, or a catalog past MB3's bounds. None
/// names the server's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::broker) enum Fatal {
    Failed(Failure),
    Unsupported,
    Missing(String),
    Catalog,
}

impl Fatal {
    /// The failure the session's closure carries.
    pub(in crate::broker) fn failure(&self) -> Failure {
        match self {
            Fatal::Failed(failure) => failure.clone(),
            Fatal::Unsupported | Fatal::Missing(_) => Failure::Protocol,
            Fatal::Catalog => Failure::Response,
        }
    }
}

impl Display for Fatal {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Fatal::Failed(failure) => failure.fmt(formatter),
            Fatal::Unsupported => Refused::Version.fmt(formatter),
            Fatal::Missing(tool) => {
                write!(formatter, "MCP server does not offer granted tool '{tool}'")
            }
            Fatal::Catalog => formatter.write_str("MCP tool catalog exceeds the broker limit"),
        }
    }
}

impl std::error::Error for Fatal {}

/// Why the broker answers a harness request itself with a refusal (MB3,
/// SC4), in their exact words: that request's JSON-RPC error, never the
/// session's end.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Refused {
    Ungranted { tool: String, capability: String },
    Method,
    Limit,
    Invalid,
    Active,
    Version,
}

impl Refused {
    /// The JSON-RPC error code it answers with.
    fn code(&self) -> i64 {
        match self {
            Refused::Method => -32601,
            Refused::Ungranted { .. } | Refused::Version => -32602,
            Refused::Limit | Refused::Invalid | Refused::Active => -32600,
        }
    }
}

impl Display for Refused {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Refused::Ungranted { tool, capability } => write!(
                formatter,
                "tool '{tool}' is not granted to capability '{capability}'"
            ),
            Refused::Method => formatter.write_str("MCP method is not admitted by this broker"),
            Refused::Limit => formatter.write_str("MCP request exceeds the broker limit"),
            Refused::Invalid => formatter.write_str("MCP tool call request is invalid"),
            Refused::Active => formatter.write_str("MCP broker already has an active call"),
            Refused::Version => {
                formatter.write_str("MCP protocol version is not supported by this broker")
            }
        }
    }
}

impl std::error::Error for Refused {}

/// A JSON-RPC request id, typed: a number and a string are never the same
/// id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum Id {
    Number(i64),
    Text(String),
}

/// JSON-RPC's one version, the only `jsonrpc` a frame may carry.
#[derive(Debug, Deserialize)]
enum Two {
    #[serde(rename = "2.0")]
    Two,
}

/// A member that, where present, must be a `T`: absent, it is none, and an
/// explicit null is no `T`, never the same as absent.
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    member: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(member).map(Some)
}

/// Nothing, flattened into each shape read from either side, so that serde
/// reads that shape from an object by its keys alone, still refusing a
/// key twice and, where the shape is closed, an unknown one: serde would
/// otherwise read a struct from an array by position, which no JSON-RPC or
/// MCP shape is.
#[derive(Debug, Deserialize)]
struct Keyed {}

/// One frame from the server: the members JSON-RPC 2.0 gives a request, a
/// notification or a response, and no other. An id is a number or a
/// string, a method a string, a result an object, and an error JSON-RPC's
/// error object.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Frame {
    #[serde(rename = "jsonrpc")]
    _version: Two,
    #[serde(default, deserialize_with = "present")]
    id: Option<Id>,
    #[serde(default, deserialize_with = "present")]
    method: Option<String>,
    #[serde(rename = "params")]
    _params: Option<IgnoredAny>,
    #[serde(default, deserialize_with = "present")]
    result: Option<Map<String, Value>>,
    #[serde(default, deserialize_with = "present")]
    error: Option<Fault>,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// JSON-RPC's error object, its members and no other: what a server's
/// error response carries, relayed to the harness as the data it is.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fault {
    code: i64,
    message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(flatten, skip_serializing)]
    _keyed: Keyed,
}

/// A response's body, each as JSON-RPC names it: its result, an object,
/// or its error.
#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
enum Reply {
    Result(Value),
    Error(Fault),
}

/// What was heard of the server: nothing moved either way; bytes moved but
/// no response, which may be a frame that answers nothing; or a response
/// by its id.
#[derive(Debug)]
enum Heard {
    Quiet,
    Taken,
    Answer(Id, Reply),
}

/// The JSON-RPC error answering `id`, null where it could not be read.
fn error(id: Option<&Id>, code: i64, message: &dyn Display) -> Value {
    let error = json!({"code": code, "message": message.to_string()});
    json!({"jsonrpc": "2.0", "id": id, "error": error})
}

/// What the next of a pipe's frames is.
#[derive(Debug, PartialEq)]
enum Line {
    Frame(Vec<u8>),
    Quiet,
    Partial,
    Over,
    Ended,
}

/// A pipe read as newline-delimited frames without blocking: the bytes
/// past the last whole frame, how many of them are known to hold no
/// newline, and whether a frame over its bound is being passed over.
struct Lines<R> {
    pipe: R,
    held: Vec<u8>,
    searched: usize,
    over: bool,
}

impl<R: Read + AsFd> Lines<R> {
    /// `pipe` read without blocking, or none where it cannot be.
    fn new(pipe: R) -> Option<Lines<R>> {
        rustix::io::ioctl_fionbio(pipe.as_fd(), true).ok()?;
        Some(Lines {
            pipe,
            held: Vec::new(),
            searched: 0,
            over: false,
        })
    }

    /// The next whole frame, its newline dropped, of at most `most` bytes:
    /// one longer is `Over`, once, as soon as it is known to be, and then
    /// passed over to its end; `Quiet` where the pipe has nothing yet;
    /// `Partial` where one read took bytes but no frame is whole, so its
    /// caller reads its deadline again before the next; `Ended` at the
    /// pipe's end or on its error.
    fn next(&mut self, most: usize) -> Line {
        if let Some(line) = self.framed(most) {
            return line;
        }
        let mut chunk = [0; CHUNK];
        let read = self.pipe.read(&mut chunk);
        let blocked = read
            .as_ref()
            .is_err_and(|e| e.kind() == ErrorKind::WouldBlock);
        match (read.unwrap_or(0), blocked) {
            (_, true) => Line::Quiet,
            (0, false) => Line::Ended,
            (read, false) => {
                self.held.extend_from_slice(&chunk[..read]);
                self.framed(most).unwrap_or(Line::Partial)
            }
        }
    }

    /// The next whole frame among the bytes held, or `Over`; none where no
    /// frame is whole yet. A frame passed over is dropped through its
    /// newline, and the frames held after it are read on.
    fn framed(&mut self, most: usize) -> Option<Line> {
        loop {
            let newline = self.held[self.searched..].iter().position(|b| *b == b'\n');
            match (newline, self.over) {
                (Some(at), over) => {
                    let mut frame: Vec<u8> = self.held.drain(..=self.searched + at).collect();
                    frame.pop();
                    (self.searched, self.over) = (0, false);
                    match (over, frame.len() > most) {
                        (true, _) => {}
                        (false, true) => return Some(Line::Over),
                        (false, false) => return Some(Line::Frame(frame)),
                    }
                }
                (None, true) => {
                    self.held.clear();
                    self.searched = 0;
                    return None;
                }
                (None, false) if self.held.len() > most => {
                    self.held.clear();
                    (self.searched, self.over) = (0, true);
                    return Some(Line::Over);
                }
                (None, false) => {
                    self.searched = self.held.len();
                    return None;
                }
            }
        }
    }
}

/// Whether the JSON text `bytes` nests at most [`DEPTH_MAX`] deep: its
/// brackets counted outside strings, where an escaped quote ends none.
fn shallow(bytes: &[u8]) -> bool {
    let (mut depth, mut deepest, mut text, mut escaped) = (0usize, 0, false, false);
    for byte in bytes {
        match (text, escaped, byte) {
            (true, true, _) => escaped = false,
            (true, false, b'\\') => escaped = true,
            (_, false, b'"') => text = !text,
            (false, _, b'[' | b'{') => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            (false, _, b']' | b'}') => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest <= DEPTH_MAX
}

/// The admitted server as the protocol speaks with it: its standard input,
/// written without blocking, and the bytes queued for it not yet taken;
/// its standard output read as frames; the id its next request takes; and
/// the bytes of frames that answer nothing the current operation has heard.
pub(in crate::broker) struct Server<'l> {
    input: &'l mut std::io::PipeWriter,
    queued: Vec<u8>,
    output: Lines<&'l mut std::io::PipeReader>,
    next: i64,
    heard: usize,
}

impl<'l> Server<'l> {
    fn new((input, output): Link<'l>) -> Result<Server<'l>, Fatal> {
        rustix::io::ioctl_fionbio(input.as_fd(), true).or(Err(PROTOCOL))?;
        let output = Lines::new(output).ok_or(PROTOCOL)?;
        Ok(Server {
            input,
            queued: Vec::new(),
            output,
            next: 1,
            heard: 0,
        })
    }

    /// The id the next request takes, never one taken before.
    fn id(&mut self) -> Id {
        self.next += 1;
        Id::Number(self.next - 1)
    }

    /// Begin an operation (MB3): an initialization, a whole listing or a
    /// call, its budget of non-response bytes whole until the next begins.
    fn begin(&mut self) {
        self.heard = 0;
    }

    /// Queue the request `method` with `params` as `id`.
    fn ask(&mut self, id: &Id, method: &str, params: Value) {
        self.queue(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
    }

    /// Queue `message` as one whole frame, after every frame queued before
    /// it, so none is ever interleaved with another.
    fn queue(&mut self, message: &Value) {
        let line = serde_json::to_vec(message).unwrap_or_default();
        self.queued.extend_from_slice(&line);
        self.queued.push(b'\n');
    }

    /// Write as much of what is queued as the server takes now, never
    /// waiting for it: whether it took any. A server whose input has ended,
    /// or fails, ends the session.
    fn flushed(&mut self) -> Result<bool, Fatal> {
        let written = match self.queued.is_empty() {
            true => Ok(0),
            false => self.input.write(&self.queued),
        };
        match written {
            Ok(taken) => {
                self.queued.drain(..taken);
                Ok(taken > 0)
            }
            Err(error) => ensure(error.kind() == ErrorKind::WouldBlock, PROTOCOL).map(|()| false),
        }
    }

    /// The server's next frame, if one is whole, once what is queued for it
    /// is written as far as it takes it: a response, by its id; or a frame
    /// that answers nothing ([`Server::noticed`]). Taken, not quiet, where
    /// it took what was queued but said nothing. Its end, a frame over
    /// MB3's bounds or one not JSON-RPC 2.0 ends the session.
    fn heard(&mut self) -> Result<Heard, Fatal> {
        let took = self.flushed()?;
        let frame = match self.output.next(RESPONSE_MAX) {
            Line::Frame(frame) => frame,
            Line::Quiet if took => return Ok(Heard::Taken),
            Line::Quiet => return Ok(Heard::Quiet),
            Line::Partial => return Ok(Heard::Taken),
            Line::Over => return Err(RESPONSE),
            Line::Ended => return Err(PROTOCOL),
        };
        ensure(shallow(&frame), RESPONSE)?;
        let parsed: Frame = serde_json::from_slice(&frame).or(Err(PROTOCOL))?;
        match (parsed.id, parsed.method, parsed.result, parsed.error) {
            (Some(id), None, Some(result), None) => {
                Ok(Heard::Answer(id, Reply::Result(Value::Object(result))))
            }
            (Some(id), None, None, Some(error)) => Ok(Heard::Answer(id, Reply::Error(error))),
            (asked, Some(method), None, None) => self.noticed(frame.len(), asked, &method),
            _ => Err(PROTOCOL),
        }
    }

    /// A frame of `bytes` that answers nothing, the request `method` as
    /// `asked`, or a notification: its bytes spent from the operation's
    /// budget, past which it ends the session; a ping answered with nothing
    /// and any other request refused, each answer queued.
    fn noticed(&mut self, bytes: usize, asked: Option<Id>, method: &str) -> Result<Heard, Fatal> {
        self.heard += bytes;
        ensure(self.heard <= NOTICES_MAX, RESPONSE)?;
        let answer = asked.map(|id| match method {
            "ping" => json!({"jsonrpc": "2.0", "id": id, "result": {}}),
            _ => error(Some(&id), Refused::Method.code(), &Refused::Method),
        });
        if let Some(answer) = answer {
            self.queue(&answer);
        }
        Ok(Heard::Taken)
    }

    /// The result of the request `method` with `params`, read as `T`: the
    /// one response to it, heard within `startup`, which bounds the request's
    /// write too. An error, a result not of that shape, a response to
    /// anything else, or none in time ends the session.
    fn exchanged<T: DeserializeOwned>(
        &mut self,
        method: &str,
        params: Value,
        startup: Startup<'_>,
    ) -> Result<T, Fatal> {
        let asked = self.id();
        self.ask(&asked, method, params);
        let mut heard = Ok(Heard::Quiet);
        let answered = polled(startup, || {
            heard = self.heard();
            match &heard {
                Ok(Heard::Quiet) => Err(ErrorKind::WouldBlock.into()),
                Ok(Heard::Taken) => Ok(false),
                Ok(Heard::Answer(..)) | Err(_) => Ok(true),
            }
        });
        match (answered, heard?) {
            (true, Heard::Answer(id, Reply::Result(result))) if id == asked => {
                T::deserialize(result).or(Err(PROTOCOL))
            }
            (true, _) => Err(PROTOCOL),
            (false, _) => Err(TIMEOUT),
        }
    }
}

/// What a server's initialize result says that the broker checks, beside
/// whatever else it says.
#[derive(Deserialize)]
struct Initialized {
    #[serde(rename = "protocolVersion")]
    version: String,
    #[serde(rename = "serverInfo")]
    server: Info,
}

/// A server's version, beside whatever else it says of itself.
#[derive(Deserialize)]
struct Info {
    version: String,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// One page of the server's catalog: each tool as the data it is, and the
/// cursor of the next page, if any.
#[derive(Deserialize)]
struct Page {
    tools: Vec<Value>,
    #[serde(rename = "nextCursor")]
    next: Option<String>,
}

/// A listed tool's name.
#[derive(Deserialize)]
struct Tool {
    name: String,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// What the server offers of the grant: the result a harness's listing is
/// answered with, which holds the granted tools as the server describes
/// them, in its order, and nothing else.
pub(in crate::broker) struct Catalog(Vec<u8>);

/// What `serve` does with an admitted box's stdio until U6f completes the
/// serving protections: its server [`started`] for the plan `plan` within
/// `startup`, then let go, so the box is ended before any message reaches
/// the harness; or the session's fatal ending.
pub(in crate::broker) fn initialized<'p>(
    plan: &'p Plan,
    startup: Startup<'p>,
) -> impl FnOnce(Link<'_>) -> Result<(), Ended> + 'p {
    move |link| {
        started(link, plan, startup)
            .map(drop)
            .map_err(Ended::Failed)
    }
}

/// The admitted box's server, over its stdio `link`, initialized within
/// `startup`'s absolute deadline for the plan `plan` (MB3): asked to
/// initialize at MB3's version, which it must answer with, and its server
/// version the dialect's pinned one; told it is initialized; and its whole
/// catalog listed and filtered to the plan's grant ([`listed`]). Nothing of
/// it reaches the harness.
pub(in crate::broker) fn started<'l>(
    link: Link<'l>,
    plan: &Plan,
    startup: Startup<'_>,
) -> Result<(Server<'l>, Catalog), Fatal> {
    let mut server = Server::new(link)?;
    let client = json!({"name": "brokkr-broker", "version": env!("CARGO_PKG_VERSION")});
    let hello = json!({"protocolVersion": VERSION, "capabilities": {}, "clientInfo": client});
    let initialized: Initialized = server.exchanged("initialize", hello, startup)?;
    ensure(initialized.version == VERSION, Fatal::Unsupported)?;
    let pinned = initialized.server.version == plan.dialect.version;
    ensure(pinned, Fatal::Failed(Failure::Version))?;
    server.queue(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let catalog = listed(&mut server, &plan.tools, startup)?;
    Ok((server, catalog))
}

/// The plan's `granted` tools as the server's complete catalog describes
/// them (MB3), heard within `startup`: page by page, at most
/// [`PAGES_MAX`] pages of [`CATALOG_MAX`] tools in all, no cursor and no
/// tool's name twice, and every page one operation, whose one budget of
/// non-response bytes the pages share. What the catalog holds widens
/// nothing: a tool not granted is left out, and a granted tool it lacks
/// ends the session.
fn listed(
    server: &mut Server<'_>,
    granted: &[String],
    startup: Startup<'_>,
) -> Result<Catalog, Fatal> {
    let (mut offered, mut names, mut cursors) = (Vec::new(), BTreeSet::new(), BTreeSet::new());
    let mut cursor = json!({});
    server.begin();
    for _ in 0..PAGES_MAX {
        let page: Page = server.exchanged("tools/list", cursor, startup)?;
        ensure(
            names.len() + page.tools.len() <= CATALOG_MAX,
            Fatal::Catalog,
        )?;
        for tool in page.tools {
            let named = Tool::deserialize(&tool).or(Err(PROTOCOL))?.name;
            ensure(names.insert(named.clone()), Fatal::Catalog)?;
            if granted.contains(&named) {
                offered.push((named, tool));
            }
        }
        let Some(next) = page.next else {
            return offering(granted, offered);
        };
        ensure(cursors.insert(next.clone()), Fatal::Catalog)?;
        cursor = json!({"cursor": next});
    }
    Err(Fatal::Catalog)
}

/// The catalog of the tools `offered`, where they are every `granted` one;
/// the first granted tool they lack ends the session, and so does a
/// listing over MB3's bound on a response, which is never delivered.
fn offering(granted: &[String], offered: Vec<(String, Value)>) -> Result<Catalog, Fatal> {
    let lacked = granted
        .iter()
        .find(|tool| !offered.iter().any(|(name, _)| name == *tool));
    lacked.map_or(Ok(()), |tool| Err(Fatal::Missing(tool.clone())))?;
    let tools: Vec<Value> = offered.into_iter().map(|(_, tool)| tool).collect();
    let listing = serde_json::to_vec(&json!({"tools": tools})).unwrap_or_default();
    ensure(listing.len() <= RESPONSE_MAX, RESPONSE)?;
    Ok(Catalog(listing))
}
