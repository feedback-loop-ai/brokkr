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
//! null, never taken as absent; a frame's params and a page's next cursor
//! read a null as absent, and an error's data keeps an explicit null apart
//! from its absence, each relayed as it was (CR2). A response must answer the
//! one request outstanding, by its exact typed id: a number and a string
//! are never the same id, and any other response ends the session. A frame
//! that answers nothing is never the harness's: the server's ping is
//! answered, any other request it makes refused, and a notification
//! dropped, each spending the operation's fixed budget of non-response
//! bytes, which never extends its deadline. What the broker sends the
//! server is queued, whole frames in order, and written as far as the
//! server takes it whenever its output is read, so no write waits on it. A
//! fatal ending is the session's first cause ([`Fatal`]), latched through
//! the ledger's own catalogue. A response is read strictly as well, at the
//! output edge ([`output`], U6f1a): a key twice or a number read as another
//! value ends the session too. The harness-facing call path is [`calls`]'s,
//! which `serve` takes up once the server has initialized ([`serving`],
//! U6f1b), under the attempt's cancellation and no longer the startup's
//! deadline, and stages a retaining plan's responses before it delivers
//! them (U8b). The server's standard error is drained from its start,
//! during its initialization as while it serves, and its masked tail is
//! kept for the broker's own however the session ends.

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsFd;
use std::time::Duration;

use brokkr_protocol::broker::ledger::{Failure, Ledger};
use brokkr_protocol::broker::Plan;
use rustix::event::{PollFd, PollFlags, Timespec};
use serde::de::{DeserializeOwned, IgnoredAny};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Map, Value};

use super::session::stderr::Drain;
use super::session::{ensure, polled, Bound, Ended, Link, Startup};

mod calls;
#[path = "output.rs"]
pub(in crate::broker) mod output;
pub(in crate::broker) mod pieces;

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
const MASKING: Fatal = Fatal::Failed(Failure::Masking);

/// Output of the broker's not delivered on its standard output or its
/// standard error for any reason but a closed reader
/// ([`Failure::Undelivered`]): the delivery failure, latched as the
/// ledger's own cause and said in its words (operator rulings of
/// 2026-10-14).
pub(in crate::broker) const UNDELIVERED: Fatal = Fatal::Failed(Failure::Undelivered);

/// Why a served session ends (MB3, CR3), each latched as its closure's
/// first cause through the ledger's own catalogue ([`Fatal::failure`]): a
/// cause the catalogue names, output the broker could not deliver
/// ([`UNDELIVERED`]) among them, or one of MB3's own that the operator
/// ruled latched under it (2026-10-11): the server's protocol version, a
/// granted tool its complete listing lacks, or a catalog past MB3's bounds.
/// None names the server's output.
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
            Fatal::Unsupported => {
                formatter.write_str("MCP protocol version is not supported by this broker")
            }
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
    Budget,
}

impl Refused {
    /// The JSON-RPC error code it answers with.
    fn code(&self) -> i64 {
        match self {
            Refused::Method => -32601,
            Refused::Ungranted { .. } => -32602,
            Refused::Limit | Refused::Invalid | Refused::Active => -32600,
            Refused::Budget => -32603,
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
            Refused::Budget => formatter.write_str("capability evidence budget is exhausted"),
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
/// error response carries, relayed to the harness as the data it is. Its
/// data is absent, or present as whatever it holds, an explicit null
/// included, never the one read as the other (CR2).
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Fault {
    code: i64,
    message: String,
    #[serde(default, deserialize_with = "present")]
    #[serde(skip_serializing_if = "Option::is_none")]
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
pub(in crate::broker) enum Line {
    Frame(Vec<u8>),
    Quiet,
    Partial,
    Over,
    Ended,
}

/// The most bytes one write takes once poll finds its descriptor writable:
/// POSIX's `_POSIX_PIPE_BUF`, the least `PIPE_BUF` any POSIX platform
/// sets, used on Linux and macOS alike (decision 0063; operator ruling
/// 2026-10-13), so a pipe takes each such write whole, at once, or not at
/// all.
pub(in crate::broker) const ATOMIC: usize = 512;

/// Whether `fd` is ready for `wanted` within `wait`, as poll finds it,
/// which never changes the status flags every holder of the descriptor
/// shares (U6f1b), and decided apart from the poll ([`pieces::readiness`]):
/// its end, its error or its closing count as ready, so the read or write
/// it allows meets them; a wait a signal interrupted, as not.
pub(in crate::broker) fn ready(
    fd: impl AsFd,
    wanted: PollFlags,
    wait: Duration,
) -> std::io::Result<bool> {
    let mut polled = [PollFd::new(&fd, wanted)];
    let wait = Timespec::try_from(wait).unwrap_or(Timespec {
        tv_sec: i64::MAX,
        tv_nsec: 0,
    });
    let found = rustix::event::poll(&mut polled, Some(&wait));
    pieces::readiness(found.map(|_| polled[0].revents()), wanted)
}

/// Write as much of `bytes` to `sink`, the server's input, as it takes now,
/// never waiting for it and never changing its status flags: [`ATOMIC`]
/// bytes at a time, each only once poll finds it writable. No guard reads
/// what the server is sent, which [`pieces`] cuts only where the harness
/// and the broker's standard error are written. How many it took;
/// would-block where it took none and could take nothing now.
pub(in crate::broker) fn offered(
    sink: &mut (impl Write + AsFd),
    bytes: &[u8],
) -> std::io::Result<usize> {
    let mut taken = 0;
    while taken < bytes.len() && ready(sink.as_fd(), PollFlags::OUT, Duration::ZERO)? {
        let end = bytes.len().min(taken + ATOMIC);
        match sink.write(&bytes[taken..end]) {
            Ok(written) => taken += written,
            Err(_) if taken > 0 => break,
            Err(error) => return Err(error),
        }
    }
    match taken {
        0 if !bytes.is_empty() => Err(ErrorKind::WouldBlock.into()),
        _ => Ok(taken),
    }
}

/// A pipe read as newline-delimited frames without blocking: the bytes
/// past the last whole frame, how many of them are known to hold no
/// newline, and whether a frame over its bound is being passed over. The
/// server's standard error is read as lines this way too.
pub(in crate::broker) struct Lines<R> {
    pipe: R,
    held: Vec<u8>,
    searched: usize,
    over: bool,
}

impl<R: Read + AsFd> Lines<R> {
    /// `pipe` read only once poll finds it readable ([`ready`]), so a read
    /// takes what it holds and never waits, and its status flags, which
    /// every holder of it shares, are never changed.
    pub(in crate::broker) fn new(pipe: R) -> Lines<R> {
        Lines {
            pipe,
            held: Vec::new(),
            searched: 0,
            over: false,
        }
    }

    /// The next whole frame, its newline dropped, of at most `most` bytes:
    /// one longer is `Over`, once, as soon as it is known to be, and then
    /// passed over to its end; `Quiet` where the pipe has nothing yet;
    /// `Partial` where one read took bytes but no frame is whole, so its
    /// caller reads its deadline again before the next; `Ended` at the
    /// pipe's end or on its error, a poll's included.
    pub(in crate::broker) fn next(&mut self, most: usize) -> Line {
        if let Some(line) = self.framed(most) {
            return line;
        }
        let mut chunk = [0; CHUNK];
        let read = match ready(self.pipe.as_fd(), PollFlags::IN, Duration::ZERO) {
            Ok(false) => return Line::Quiet,
            polled => polled.and_then(|_| self.pipe.read(&mut chunk)),
        };
        let interrupted = read
            .as_ref()
            .is_err_and(|e| e.kind() == ErrorKind::Interrupted);
        match (read.unwrap_or(0), interrupted) {
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

    /// The bytes held past the last whole frame, taken: none where a
    /// frame over its bound is being passed over.
    pub(in crate::broker) fn rest(&mut self) -> Vec<u8> {
        self.searched = 0;
        std::mem::take(&mut self.held)
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
    fn new((input, output): Link<'l>) -> Server<'l> {
        Server {
            input,
            queued: Vec::new(),
            output: Lines::new(output),
            next: 1,
            heard: 0,
        }
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
    /// waiting for it ([`offered`]): whether it took any. A server whose
    /// input has ended, or fails, ends the session.
    fn flushed(&mut self) -> Result<bool, Fatal> {
        let written = offered(&mut self.input, &self.queued);
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
    /// MB3's bounds or one not JSON-RPC 2.0 ends the session, and so does a
    /// response not read exactly ([`output::checked`]).
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
        let answer = |id, reply| output::checked(&frame).map(|()| Heard::Answer(id, reply));
        match (parsed.id, parsed.method, parsed.result, parsed.error) {
            (Some(id), None, Some(result), None) => {
                answer(id, Reply::Result(Value::Object(result)))
            }
            (Some(id), None, None, Some(error)) => answer(id, Reply::Error(error)),
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
    /// write too, the server's standard error drained by `stderr` every
    /// turn, so a server that says much there before it answers never
    /// waits on it. An error, a result not of that shape, a response to
    /// anything else, or none in time ends the session.
    fn exchanged<T: DeserializeOwned>(
        &mut self,
        method: &str,
        params: Value,
        (startup, stderr): (Startup<'_>, &mut Drain<'_, impl Read + AsFd>),
    ) -> Result<T, Fatal> {
        let asked = self.id();
        self.ask(&asked, method, params);
        let mut heard = Ok(Heard::Quiet);
        let answered = polled(startup, || {
            let spoke = stderr.drained();
            heard = self.heard();
            match (&heard, spoke) {
                (Ok(Heard::Quiet), false) => Err(ErrorKind::WouldBlock.into()),
                (Ok(Heard::Quiet | Heard::Taken), _) => Ok(false),
                (Ok(Heard::Answer(..)) | Err(_), _) => Ok(true),
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

/// A listed tool: its name, and whatever else the server says of it, which
/// is the server's data.
#[derive(Serialize, Deserialize)]
struct Tool {
    name: String,
    #[serde(flatten)]
    described: Map<String, Value>,
}

/// What the server offers of the grant: the tools a harness's listing is
/// answered with, the granted ones as the server describes them, in its
/// order, and nothing else; masked at the output edge, and each still the
/// tool the grant offered, before any harness is answered with them.
pub(in crate::broker) struct Catalog(Vec<Tool>);

/// What `serve` does with an admitted box's stdio `link` and standard error
/// `errors` once its `bound` ledger and bindings guard it (U6f1b): the
/// server's standard error drained from the first, so a server that says
/// much there never waits on it ([`Drain`]); the session [`initialized`]
/// and served; and then, however it ended, the masked tail of that standard
/// error left with the lifetime, which writes it to the broker's own while
/// those bindings are held: before its ledger is closed where it ended
/// served, after where it failed.
pub(in crate::broker) fn serving(
    link: Link<'_>,
    errors: &mut std::io::PipeReader,
    Bound {
        ledger,
        bindings,
        said,
    }: Bound<'_, '_>,
    (plan, startup): (&Plan, Startup<'_>),
) -> Result<(), Ended> {
    let mut stderr = Drain::new(errors, bindings);
    let edge = output::Edge {
        bindings,
        stderr: &mut stderr,
    };
    let ended = initialized(link, ledger, edge, (plan, startup), held_apart);
    said.borrow_mut().extend(stderr.tail());
    ended
}

/// The broker's own standard input and output, each through a descriptor
/// of its own, or why that could not be set up.
type Stdio = (
    std::io::Result<std::fs::File>,
    std::io::Result<std::fs::File>,
);

/// The broker's own [`Stdio`], held apart from every other holder's.
fn held_apart() -> Stdio {
    let held = |fd: std::os::fd::BorrowedFd<'_>| fd.try_clone_to_owned().map(std::fs::File::from);
    (
        held(std::io::stdin().as_fd()),
        held(std::io::stdout().as_fd()),
    )
}

/// Serving keeps no deadline of the broker's own (MB3): each call, and each
/// frame the harness is told, keeps its own, and the attempt's reaches it as
/// its cancellation, once the broker's starter ends. The startup's deadline
/// bounds the startup alone. A century stands for none.
const SERVING: std::time::Duration = std::time::Duration::from_secs(100 * 365 * 24 * 60 * 60);

/// The session's server [`started`] for the plan `plan` within `startup`,
/// its standard error drained beside it, and the harness served over the
/// broker's own standard input and output ([`calls::served`]) until it
/// ends, under the attempt's cancellation and no deadline of the
/// startup's ([`SERVING`]).
/// A fatal ending of the server's initialization stays the session's
/// cause. Standard input this process cannot hold apart ([`Stdio`], set up
/// by `stdio`) is the protocol's cause; standard output, a stream that
/// cannot be set up, the delivery failure (operator ruling 2026-10-14).
fn initialized<'b>(
    link: Link<'_>,
    ledger: &mut Ledger<'b>,
    edge: output::Edge<'_, 'b, impl Read + AsFd>,
    (plan, startup): (&Plan, Startup<'_>),
    stdio: impl FnOnce() -> Stdio,
) -> Result<(), Ended> {
    let started = started(link, plan, startup, &mut *edge.stderr).map_err(Ended::Failed)?;
    let (input, output) = stdio();
    let input = input.or(Err(Ended::Failed(PROTOCOL)))?;
    let output = output.or(Err(Ended::Failed(UNDELIVERED)))?;
    let attempt = (std::time::Instant::now() + SERVING, startup.1);
    calls::served(started, plan, ledger, (input, output), attempt, edge)
}

/// The admitted box's server, over its stdio `link`, initialized within
/// `startup`'s absolute deadline for the plan `plan` (MB3): asked to
/// initialize at MB3's version, which it must answer with, and its server
/// version the dialect's pinned one; told it is initialized; and its whole
/// catalog listed and filtered to the plan's grant ([`listed`]), its
/// standard error drained by `stderr` every turn of each wait. Nothing of
/// it reaches the harness.
pub(in crate::broker) fn started<'l>(
    link: Link<'l>,
    plan: &Plan,
    startup: Startup<'_>,
    stderr: &mut Drain<'_, impl Read + AsFd>,
) -> Result<(Server<'l>, Catalog), Fatal> {
    let mut server = Server::new(link);
    let client = json!({"name": "brokkr-broker", "version": env!("CARGO_PKG_VERSION")});
    let hello = json!({"protocolVersion": VERSION, "capabilities": {}, "clientInfo": client});
    let initialized: Initialized =
        server.exchanged("initialize", hello, (startup, &mut *stderr))?;
    ensure(initialized.version == VERSION, Fatal::Unsupported)?;
    let pinned = initialized.server.version == plan.dialect.version;
    ensure(pinned, Fatal::Failed(Failure::Version))?;
    server.queue(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let catalog = listed(&mut server, &plan.tools, (startup, stderr))?;
    Ok((server, catalog))
}

/// The plan's `granted` tools as the server's complete catalog describes
/// them (MB3), heard within `startup`, its standard error drained beside:
/// page by page, at most
/// [`PAGES_MAX`] pages of [`CATALOG_MAX`] tools in all, no cursor and no
/// tool's name twice, and every page one operation, whose one budget of
/// non-response bytes the pages share. What the catalog holds widens
/// nothing: a tool not granted is left out, and a granted tool it lacks
/// ends the session.
fn listed(
    server: &mut Server<'_>,
    granted: &[String],
    (startup, stderr): (Startup<'_>, &mut Drain<'_, impl Read + AsFd>),
) -> Result<Catalog, Fatal> {
    let (mut offered, mut names, mut cursors) = (Vec::new(), BTreeSet::new(), BTreeSet::new());
    let mut cursor = json!({});
    server.begin();
    for _ in 0..PAGES_MAX {
        let page: Page = server.exchanged("tools/list", cursor, (startup, &mut *stderr))?;
        ensure(
            names.len() + page.tools.len() <= CATALOG_MAX,
            Fatal::Catalog,
        )?;
        for tool in page.tools {
            let tool = Tool::deserialize(tool).or(Err(PROTOCOL))?;
            ensure(names.insert(tool.name.clone()), Fatal::Catalog)?;
            if granted.contains(&tool.name) {
                offered.push(tool);
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
fn offering(granted: &[String], offered: Vec<Tool>) -> Result<Catalog, Fatal> {
    let lacked = granted
        .iter()
        .find(|tool| !offered.iter().any(|offered| offered.name == **tool));
    lacked.map_or(Ok(()), |tool| Err(Fatal::Missing(tool.clone())))?;
    let listing = serde_json::to_vec(&json!({"tools": offered})).unwrap_or_default();
    ensure(listing.len() <= RESPONSE_MAX, RESPONSE)?;
    Ok(Catalog(offered))
}
