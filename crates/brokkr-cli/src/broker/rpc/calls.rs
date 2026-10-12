//! The harness-facing call path (decision 0065 slice two, U6e; MB3, CC2,
//! CR3, SC4), which `serve` takes up once U6f completes the serving
//! protections; until then its tests are its only consumer. The harness's
//! frames are read whole, at most 1 MiB and 64 deep, and each is checked
//! before anything is recorded: initialize at MB3's version, ping and the
//! filtered listing are answered here, a cancellation of the active call
//! is passed on, notifications are taken, and any other method refused. A
//! tools/call naming a tool in seat-record's vocabulary within SC4's 256
//! bytes is accepted only once its Started is durable, for which the
//! ledger keeps room: at its call bound the harness is still read, and the
//! next call ends the session with the ledger's limit. An accepted call to
//! a tool the plan does not grant, or made while another is active, is
//! refused with a durable Terminal and forwarded nowhere; any other is
//! forwarded, one at a time, under its own absolute deadline and with a
//! request id of the broker's own, which its response must carry exactly,
//! in JSON-RPC's and MCP's shapes. Its request, and its cancellation, which
//! is passed on once, are queued for the server and written as it takes
//! them while the harness is still read, so a server that takes neither
//! holds nothing past the deadline that applies. Its Terminal is durable before
//! the response is delivered, within the call's own deadline, never past
//! MB3's 8 MiB with the harness's own id, and a cancelled call's response
//! is never delivered. What the harness is told is queued too, whole frames
//! in order, each with the deadline that applies to it, and written as far
//! as the harness takes it on every turn, so the harness is still read
//! while it leaves its output full, and a cancellation behind an answer it
//! has not taken still settles within MB3's two seconds. It is not read
//! while 64 frames, or MB3's 8 MiB, wait for it, and an active call then
//! has those two seconds, from when they first did, to end, so that a
//! cancellation left unread behind them settles as soon. The attempt's
//! deadline and cancellation are read before every step, idle or not; a
//! frame not taken by its deadline is never begun or continued, and ends
//! the session, as does one left untaken once the harness has ended its
//! input. A fatal ending fails the active call and ends the session with
//! its cause; a record that cannot be made ends it before anything acts on
//! it.

use std::collections::{BTreeMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{Call, Denial, Failure, Ledger, Outcome};
use brokkr_protocol::broker::Plan;
use serde::de::IgnoredAny;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use super::{error, present, shallow, Catalog, Fatal, Heard, Id, Keyed, Line, Lines};
use super::{Refused, Reply, Server, Two};
use super::{PROTOCOL, REQUEST_MAX, RESPONSE, RESPONSE_MAX, TIMEOUT, VERSION};
use crate::broker::session::{ensure, polled, timely, Ended, Pace, Startup};

/// MB3's bound on one forwarded call.
const CALL: Duration = Duration::from_secs(120);

/// How long a cancelled call's server has to settle it (MB3).
const SETTLE: Duration = Duration::from_secs(2);

/// How long one answer to the harness may take to write: MB3's bound on a
/// listing.
const ANSWER: Duration = Duration::from_secs(30);

/// How long the session's last frame, telling the harness why its call
/// failed, may take to write, whatever deadline is left or has passed: a
/// fixed bound that nothing extends, and after which nothing waits.
const CLOSING: Duration = Duration::from_secs(1);

/// The most frames, and bytes, that wait for the harness before its next
/// frame is read no more until it takes them: room for any one answer, so
/// that a cancellation behind it is still read, and no unbounded queue.
const UNTAKEN_FRAMES: usize = 64;
const UNTAKEN_MAX: usize = RESPONSE_MAX;

/// The longest tool name a call carries, in bytes (SC4).
const TOOL_MAX: usize = 256;

/// The JSON-RPC error code of a session's own failure.
const INTERNAL: i64 = -32603;

/// The ledger's limit, the session's cause once it has no room.
const LIMIT: Fatal = Fatal::Failed(Failure::Limit);

/// One frame from the harness: the members JSON-RPC 2.0 gives a request or
/// a notification, and no other. An id, where present, is a number or a
/// string, never null.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Asked {
    #[serde(rename = "jsonrpc")]
    _version: Two,
    #[serde(default, deserialize_with = "present")]
    id: Option<Id>,
    method: String,
    params: Option<Value>,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A harness's initialize parameters: the version it asks for, beside
/// whatever else it says.
#[derive(Deserialize)]
struct Greeting {
    #[serde(rename = "protocolVersion")]
    version: String,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A harness's cancellation: the request it cancels, beside its reason.
#[derive(Deserialize)]
struct Cancelled {
    #[serde(rename = "requestId")]
    request: Id,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A tools/call's parameters, MCP's members and no other: the tool, its
/// arguments, which are the server's data and never the broker's, and its
/// metadata, which is not passed on. Each that is present is an object,
/// never null.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Called {
    name: String,
    #[serde(default, deserialize_with = "present")]
    arguments: Option<Map<String, Value>>,
    #[serde(rename = "_meta", default, deserialize_with = "present")]
    _meta: Option<BTreeMap<String, IgnoredAny>>,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A tools/call result in MCP's shape, as far as the broker reads it: its
/// content blocks, each an object; its word on whether the tool failed;
/// and its structured content and metadata, each an object where present.
#[derive(Deserialize)]
struct Answered {
    #[serde(rename = "content")]
    _content: Vec<Map<String, Value>>,
    #[serde(rename = "isError", default, deserialize_with = "present")]
    error: Option<bool>,
    #[serde(rename = "structuredContent", default, deserialize_with = "present")]
    _structured: Option<Map<String, Value>>,
    #[serde(rename = "_meta", default, deserialize_with = "present")]
    _meta: Option<BTreeMap<String, IgnoredAny>>,
}

/// A harness request, checked whole.
enum Request {
    Initialize(Id, String),
    Ping(Id),
    List(Id),
    Call(Id, Called),
    Cancel(Id),
    Told,
    Other(Id),
}

/// The request the harness's frame `bytes` holds, or why it is refused
/// before anything is recorded, with its id where it has one. A
/// notification needs no answer: one the broker does not take is told
/// nothing.
fn decoded(bytes: &[u8]) -> Result<Request, (Option<Id>, Refused)> {
    ensure(shallow(bytes), (None, Refused::Limit))?;
    let asked: Asked = serde_json::from_slice(bytes).or(Err((None, Refused::Invalid)))?;
    let params = asked.params.unwrap_or(Value::Null);
    match (asked.method.as_str(), asked.id) {
        ("notifications/cancelled", None) => {
            let cancelled = Cancelled::deserialize(params);
            Ok(cancelled.map_or(Request::Told, |cancelled| {
                Request::Cancel(cancelled.request)
            }))
        }
        (_, None) => Ok(Request::Told),
        ("initialize", Some(id)) => match Greeting::deserialize(params) {
            Ok(greeting) => Ok(Request::Initialize(id, greeting.version)),
            Err(_) => Err((Some(id), Refused::Invalid)),
        },
        ("ping", Some(id)) => Ok(Request::Ping(id)),
        ("tools/list", Some(id)) => Ok(Request::List(id)),
        ("tools/call", Some(id)) => match called(params) {
            Ok(called) => Ok(Request::Call(id, called)),
            Err(refused) => Err((Some(id), refused)),
        },
        (_, Some(id)) => Ok(Request::Other(id)),
    }
}

/// A tools/call's parameters: MCP's shape, or invalid; then a name within
/// SC4's byte bound, or the request limit; then in its vocabulary, or
/// invalid. None is recorded, nor its name echoed.
fn called(params: Value) -> Result<Called, Refused> {
    let called = Called::deserialize(params).or(Err(Refused::Invalid))?;
    ensure(called.name.len() <= TOOL_MAX, Refused::Limit)?;
    ensure(vocabulary(&called.name), Refused::Invalid)?;
    Ok(called)
}

/// Whether `tool` is in seat-record's tool vocabulary (SC4): an ASCII
/// letter or digit, then letters, digits and `._:/-`.
fn vocabulary(tool: &str) -> bool {
    let mut bytes = tool.bytes();
    let head = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    head & bytes.all(|byte| byte.is_ascii_alphanumeric() | b"._:/-".contains(&byte))
}

/// Whether a tools/call `result` says its tool succeeded; none where it is
/// not MCP's result shape, which is the server's protocol fault and never
/// a tool's outcome.
fn outcome(result: &Value) -> Option<Outcome> {
    let answered = Answered::deserialize(result).ok()?;
    Some(match answered.error {
        Some(true) => Outcome::Failed,
        Some(false) | None => Outcome::Succeeded,
    })
}

/// What became of the harness's output: open; closed by the harness,
/// which ends the session as its input's end does; or a frame not taken by
/// its deadline, which ends it with a timeout. Nothing is written after
/// either, so no frame is ever left unfinished and another begun.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Delivery {
    Open,
    Gone,
    Stalled,
}

/// What waits for the harness to take it: whole frames in order, each with
/// the deadline by which it must be taken, and how many bytes of the first
/// it has taken already.
#[derive(Default)]
pub(super) struct Outbox {
    pub(super) frames: VecDeque<(Vec<u8>, Instant)>,
    taken: usize,
}

impl Outbox {
    /// Whether the harness's next frame may be read: fewer than
    /// [`UNTAKEN_FRAMES`] frames, and fewer than [`UNTAKEN_MAX`] bytes,
    /// wait for it.
    fn room(&self) -> bool {
        let bytes: usize = self.frames.iter().map(|(line, _)| line.len()).sum();
        (self.frames.len() < UNTAKEN_FRAMES) & (bytes - self.taken < UNTAKEN_MAX)
    }

    /// Write as much of what waits to `output` as it takes now, never
    /// waiting for it, and no frame begun before the one ahead of it is
    /// whole: stalled, with nothing written, where a frame waiting is past
    /// its deadline; gone where the output has ended or fails; open
    /// otherwise.
    pub(super) fn flushed(&mut self, output: &mut impl Write) -> Delivery {
        let now = Instant::now();
        if self.frames.iter().any(|(_, deadline)| now >= *deadline) {
            return Delivery::Stalled;
        }
        while let Some((line, _)) = self.frames.front() {
            let taken = match output.write(&line[self.taken..]) {
                Ok(taken) => taken,
                Err(error) if error.kind() == ErrorKind::WouldBlock => break,
                Err(_) => return Delivery::Gone,
            };
            self.taken += taken;
            if self.taken < line.len() {
                break;
            }
            self.frames.pop_front();
            self.taken = 0;
        }
        Delivery::Open
    }
}

/// A response the harness is answered with: the server's body, as the
/// answer to the harness's own id.
#[derive(Serialize)]
struct Answer<'a> {
    jsonrpc: &'static str,
    id: &'a Id,
    #[serde(flatten)]
    reply: &'a Reply,
}

/// The one forwarded call: its acceptance, the harness's id for it and the
/// broker's, its absolute deadline, and whether the harness cancelled it.
struct Active {
    call: Call,
    id: Id,
    asked: Id,
    deadline: Instant,
    cancelled: bool,
}

/// A session beside its server: the plan, its catalog, its ledger, the
/// harness's output, what waits for it there, since when that has been
/// at its bound, and what became of it, the attempt's deadline and
/// cancellation, the bound on one call, and the one forwarded call.
struct Session<'s, 'l, W> {
    plan: &'s Plan,
    catalog: &'s Catalog,
    server: Server<'l>,
    ledger: &'s mut Ledger,
    output: W,
    outbox: Outbox,
    full: Option<Instant>,
    delivery: Delivery,
    attempt: Startup<'s>,
    call: Duration,
    active: Option<Active>,
}

/// Serve the harness's requests on `harness` over the initialized
/// `server`, its `catalog` and the plan `plan`, recording each call in
/// `ledger`, until the harness ends its input or its output; or the cause
/// that ended the session first. The `attempt`'s deadline and cancellation
/// are read before every step, idle or not, and bound every call's
/// deadline and every frame's. What still waits for the harness then is
/// written as it takes it, by each frame's deadline, and, where a cause
/// ended the session, within [`CLOSING`] and no longer, that cause staying
/// the first; where none did, a frame not taken in time ends the session
/// with the timeout cause. A call still active when the harness ends stays
/// begun, which recovery reads as interrupted.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "consumed when U6f removes the serving refusal")
)]
pub(in crate::broker) fn served(
    started: (Server<'_>, Catalog),
    plan: &Plan,
    ledger: &mut Ledger,
    harness: (impl Read + AsFd, impl Write + AsFd),
    attempt: Startup<'_>,
) -> Result<(), Ended> {
    served_within(CALL, started, plan, ledger, harness, attempt)
}

/// [`served`], each forwarded call bound by `call`, which only tests make
/// shorter than MB3's.
pub(super) fn served_within(
    call: Duration,
    (server, catalog): (Server<'_>, Catalog),
    plan: &Plan,
    ledger: &mut Ledger,
    (input, output): (impl Read + AsFd, impl Write + AsFd),
    attempt: Startup<'_>,
) -> Result<(), Ended> {
    let mut harness = Lines::new(input).ok_or(Ended::Failed(PROTOCOL))?;
    rustix::io::ioctl_fionbio(output.as_fd(), true).or(Err(Ended::Failed(PROTOCOL)))?;
    let mut session = Session {
        plan,
        catalog: &catalog,
        server,
        ledger,
        output,
        outbox: Outbox::default(),
        full: None,
        delivery: Delivery::Open,
        attempt,
        call,
        active: None,
    };
    match session.serving(&mut harness) {
        Ok(()) => session.drained(attempt.0),
        Err(ended) => {
            session.drained(Instant::now() + CLOSING).ok();
            Err(ended)
        }
    }
}

impl<'s, W: Write + AsFd> Session<'s, '_, W> {
    /// Serve the frames `harness` sends until it ends its input or its
    /// output, or a cause ends the session first: its next frame read only
    /// while what waits for the harness has room, and only while the
    /// ledger has room or a call is active; then the server heard, and what
    /// waits for the harness written as far as it takes it, every turn. A
    /// turn waits ([`Pace`]) only where nothing moved: no byte read from
    /// the harness or the server, and none written to either.
    fn serving(&mut self, harness: &mut Lines<impl Read + AsFd>) -> Result<(), Ended> {
        let mut pace = Pace::new();
        while self.going()? {
            let room = self.ledger.room().is_ok();
            ensure(room | self.active.is_some(), Ended::Failed(LIMIT))?;
            let asked = match self.untaken() {
                false => harness.next(REQUEST_MAX),
                true => Line::Quiet,
            };
            let quiet = match asked {
                Line::Frame(frame) => self.requested(&frame).map(|()| false)?,
                Line::Over => {
                    self.refuse(None, &Refused::Limit);
                    false
                }
                Line::Quiet => true,
                Line::Partial => false,
                Line::Ended => return Ok(()),
            };
            let heard = self.heard()?;
            let told = self.flush();
            pace.after(!(heard & quiet) | told);
        }
        Ok(())
    }

    /// Whether the session goes on: not once the harness has closed its
    /// output; and, ending it with a timeout, not past the attempt's
    /// deadline or its cancellation, nor once the harness has left an
    /// answer untaken past its own.
    fn going(&mut self) -> Result<bool, Ended> {
        match (self.delivery, timely(self.attempt)) {
            (Delivery::Stalled, _) | (_, false) => self.failed(TIMEOUT),
            (Delivery::Gone, true) => Ok(false),
            (Delivery::Open, true) => Ok(true),
        }
    }

    /// Whether what waits for the harness is at its bound, so that its
    /// next frame stays unread ([`Outbox::room`]); since when it has been
    /// is kept until it has room again.
    fn untaken(&mut self) -> bool {
        let full = !self.outbox.room();
        self.full = full.then(|| self.full.unwrap_or_else(Instant::now));
        full
    }

    /// When the active call, if any, ends unless answered: its own
    /// deadline, and never more than MB3's two seconds after what waits for
    /// the harness reached its bound, so that a cancellation it has not
    /// been read for settles as soon as one read would.
    fn due(&self) -> Option<Instant> {
        let active = self.active.as_ref()?;
        let settled = self.full.map_or(active.deadline, |full| full + SETTLE);
        Some(active.deadline.min(settled))
    }

    /// The deadline of a frame told now: the active call's, or else MB3's
    /// bound on an answer, and never past the attempt's.
    fn bound(&self) -> Instant {
        let answer = Instant::now() + ANSWER;
        let deadline = self
            .active
            .as_ref()
            .map_or(answer, |active| active.deadline);
        deadline.min(self.attempt.0)
    }

    /// Tell the harness `message` as one frame by [`Session::bound`].
    fn tell(&mut self, message: &Value) {
        let mut line = serde_json::to_vec(message).unwrap_or_default();
        line.push(b'\n');
        self.write(line, self.bound());
    }

    /// Queue `line` for the harness, to be taken whole by `deadline`, after
    /// every frame queued before it; then write what it takes now, which is
    /// nothing once its output is closed or stalled ([`Session::flush`]).
    fn write(&mut self, line: Vec<u8>, deadline: Instant) {
        self.outbox.frames.push_back((line, deadline));
        self.flush();
    }

    /// Write what waits for the harness as far as it takes it now, unless
    /// its output is already closed or stalled, or the attempt, whose
    /// harness is then told nothing more, is cancelled
    /// ([`Outbox::flushed`]): a frame it has not taken whole by its
    /// deadline is the last, and ends the session. Whether it took any.
    fn flush(&mut self) -> bool {
        let waiting = (self.outbox.frames.len(), self.outbox.taken);
        if (self.delivery == Delivery::Open) & !(self.attempt.1)() {
            self.delivery = self.outbox.flushed(&mut self.output);
        }
        waiting != (self.outbox.frames.len(), self.outbox.taken)
    }

    /// Wait, until `until` unless the attempt is cancelled, for the harness
    /// to take what waits for it, each frame by its own deadline, written
    /// again at once while it takes some: the timeout cause where it has
    /// not taken it all by then and has not closed its output.
    fn drained(&mut self, until: Instant) -> Result<(), Ended> {
        let drained = polled((until, self.attempt.1), || {
            let told = self.flush();
            match (self.delivery, self.outbox.frames.is_empty(), told) {
                (Delivery::Open, false, false) => Err(ErrorKind::WouldBlock.into()),
                (Delivery::Open, false, true) => Ok(false),
                _ => Ok(true),
            }
        });
        match (drained, self.delivery) {
            (true, Delivery::Open | Delivery::Gone) => Ok(()),
            _ => Err(Ended::Failed(TIMEOUT)),
        }
    }

    /// Refuse the harness's request `id` with `refused`'s own words.
    fn refuse(&mut self, id: Option<&Id>, refused: &Refused) {
        self.tell(&error(id, refused.code(), refused));
    }

    /// Take the harness's frame `bytes`: answered here, accepted as a
    /// call, a cancellation passed on, taken as a notification, or
    /// refused before anything is recorded.
    fn requested(&mut self, bytes: &[u8]) -> Result<(), Ended> {
        let request = match decoded(bytes) {
            Ok(request) => request,
            Err((id, refused)) => {
                self.refuse(id.as_ref(), &refused);
                return Ok(());
            }
        };
        match request {
            Request::Initialize(id, version) if version == VERSION => self.greet(&id),
            Request::Initialize(id, _) => self.refuse(Some(&id), &Refused::Version),
            Request::Ping(id) => self.tell(&json!({"jsonrpc": "2.0", "id": id, "result": {}})),
            Request::List(id) => return self.listing(&id),
            Request::Call(id, called) => return self.call(id, called),
            Request::Cancel(id) => self.cancel(&id),
            Request::Told => {}
            Request::Other(id) => self.refuse(Some(&id), &Refused::Method),
        }
        Ok(())
    }

    /// Answer the harness's initialize `id` as the plan's server, at MB3's
    /// version and the dialect's pinned one, offering tools alone.
    fn greet(&mut self, id: &Id) {
        let server = json!({"name": self.plan.server, "version": self.plan.dialect.version});
        let tools = json!({"tools": {}});
        let result =
            json!({"protocolVersion": VERSION, "capabilities": tools, "serverInfo": server});
        self.tell(&json!({"jsonrpc": "2.0", "id": id, "result": result}));
    }

    /// Answer the harness's listing `id` with the filtered catalog, the
    /// same whatever the server has since said of its tools; or, where
    /// that answer with its id would pass MB3's bound on a response, tell
    /// it so, never a truncated listing, and end the session.
    fn listing(&mut self, id: &Id) -> Result<(), Ended> {
        let named = serde_json::to_vec(id).unwrap_or_default();
        let head = [
            &b"{\"jsonrpc\":\"2.0\",\"id\":"[..],
            &named,
            b",\"result\":",
        ]
        .concat();
        let line = [&head[..], &self.catalog.0, b"}"].concat();
        if line.len() > RESPONSE_MAX {
            self.tell(&error(Some(id), INTERNAL, &RESPONSE));
            return self.failed(RESPONSE);
        }
        self.write([&line[..], b"\n"].concat(), self.bound());
        Ok(())
    }

    /// Accept the harness's call `called`, its request `id`: its Started
    /// recorded, then refused here where its tool is not granted or another
    /// call is active, with its refused Terminal recorded; otherwise
    /// forwarded. A call that cannot be recorded is neither refused nor
    /// forwarded, and ends the session; one made at the ledger's call
    /// bound ends it with the ledger's limit, failing the active call.
    fn call(&mut self, id: Id, called: Called) -> Result<(), Ended> {
        let begun = self.ledger.room().map(|room| room.begin(&called.name));
        let call = match begun {
            Ok(Ok(call)) => call,
            Ok(Err(unrecorded)) => {
                self.tell(&error(Some(&id), INTERNAL, &unrecorded));
                return Err(Ended::Unrecorded(unrecorded));
            }
            Err(_) => return self.failed(LIMIT),
        };
        let granted = self.plan.tools.contains(&called.name);
        let denied = match (granted, self.active.is_some()) {
            (false, _) => Some((
                Denial::Ungranted,
                Refused::Ungranted {
                    tool: called.name.clone(),
                    capability: self.plan.capability.clone(),
                },
            )),
            (true, true) => Some((Denial::Active, Refused::Active)),
            (true, false) => None,
        };
        let Some((denial, refused)) = denied else {
            self.forwarded(call, id, called);
            return Ok(());
        };
        self.ended(call, Outcome::Refused(denial), &id)?;
        self.refuse(Some(&id), &refused);
        Ok(())
    }

    /// Forward the accepted `call` of `called` to the server under its own
    /// deadline, as the harness's request `id`, with the broker's own id:
    /// queued, and written as the server takes it.
    fn forwarded(&mut self, call: Call, id: Id, called: Called) {
        let deadline = (Instant::now() + self.call).min(self.attempt.0);
        let asked = self.server.id();
        let params =
            json!({"name": called.name, "arguments": called.arguments.unwrap_or_default()});
        self.server.begin();
        self.server.ask(&asked, "tools/call", params);
        self.active = Some(Active {
            call,
            id,
            asked,
            deadline,
            cancelled: false,
        });
    }

    /// The harness's cancellation of its request `id`: where that is the
    /// active call, passed on to the server, queued after whatever of the
    /// call it has not yet taken, and the server given MB3's two seconds
    /// to settle it, once: a cancellation already passed on, or of
    /// anything else, is nothing to cancel. A server that takes neither is
    /// ended by that deadline, and one whose input has ended ends the
    /// session when its queue is next written.
    fn cancel(&mut self, id: &Id) {
        let cancelling = |active: &&mut Active| (active.id == *id) & !active.cancelled;
        let Some(active) = self.active.as_mut().filter(cancelling) else {
            return;
        };
        active.cancelled = true;
        active.deadline = active.deadline.min(Instant::now() + SETTLE);
        let params = json!({"requestId": active.asked});
        let notice =
            json!({"jsonrpc": "2.0", "method": "notifications/cancelled", "params": params});
        self.server.queue(&notice);
    }

    /// The active call's deadline kept ([`Session::due`]), whatever the
    /// server has sent meanwhile; then the server's next frame, if any: the
    /// active call's response, by its very id, delivered; a response to
    /// anything else, or anything else that ends the session, ending it.
    /// Whether nothing was heard.
    fn heard(&mut self) -> Result<bool, Ended> {
        let late = self.due().is_some_and(|due| Instant::now() >= due);
        let heard = match late {
            true => Err(TIMEOUT),
            false => self.server.heard(),
        };
        match heard {
            Ok(Heard::Quiet) => Ok(true),
            Ok(Heard::Taken) => Ok(false),
            Ok(Heard::Answer(id, reply)) => self.answered(id, reply).map(|()| false),
            Err(fatal) => self.failed(fatal),
        }
    }

    /// The server's response `reply` to the request `id`: the active
    /// call's, its Terminal recorded first, then delivered as the answer to
    /// the harness's own id within the call's own deadline unless the
    /// harness cancelled the call. A response to anything else, a result
    /// not in MCP's shape, or an answer that would pass MB3's bound on a
    /// response ends the session, failing the call, and nothing of it is
    /// delivered.
    fn answered(&mut self, id: Id, reply: Reply) -> Result<(), Ended> {
        let bound = self.bound();
        let Some(active) = self.active.take_if(|active| active.asked == id) else {
            return self.failed(PROTOCOL);
        };
        let outcome = match &reply {
            Reply::Result(result) => outcome(result),
            Reply::Error(_) => Some(Outcome::Failed),
        };
        let answer = Answer {
            jsonrpc: "2.0",
            id: &active.id,
            reply: &reply,
        };
        let mut line = serde_json::to_vec(&answer).unwrap_or_default();
        let fatal = match (outcome, line.len() > RESPONSE_MAX) {
            (Some(outcome), false) => {
                self.ended(active.call, outcome, &active.id)?;
                line.push(b'\n');
                if !active.cancelled {
                    self.write(line, bound);
                }
                return Ok(());
            }
            (None, _) => PROTOCOL,
            (Some(_), true) => RESPONSE,
        };
        self.active = Some(active);
        self.failed(fatal)
    }

    /// `call`'s Terminal recorded with `outcome`; or the harness told its
    /// request `id` could not be, nothing else delivered, and the session
    /// ended.
    fn ended(&mut self, call: Call, outcome: Outcome, id: &Id) -> Result<(), Ended> {
        let recorded = self.ledger.end(call, outcome);
        recorded.map_err(|unrecorded| {
            self.tell(&error(Some(id), INTERNAL, &unrecorded));
            Ended::Unrecorded(unrecorded)
        })
    }

    /// End the session with `fatal`, which stays its first cause: the
    /// active call, if any, failed and its Terminal recorded, and the
    /// harness, unless it cancelled the call, told why; or, where that
    /// Terminal could not be recorded, told so instead. That last frame
    /// has [`CLOSING`] to be taken, and no longer.
    fn failed<T>(&mut self, fatal: Fatal) -> Result<T, Ended> {
        if let Some(active) = self.active.take() {
            let told = match self.ledger.end(active.call, Outcome::Failed) {
                Ok(()) => error(Some(&active.id), INTERNAL, &fatal),
                Err(unrecorded) => error(Some(&active.id), INTERNAL, &unrecorded),
            };
            let mut line = serde_json::to_vec(&told).unwrap_or_default();
            line.push(b'\n');
            if !active.cancelled {
                self.write(line, Instant::now() + CLOSING);
            }
        }
        Err(Ended::Failed(fatal))
    }
}
