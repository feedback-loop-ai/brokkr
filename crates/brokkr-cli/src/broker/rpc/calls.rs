//! The harness-facing call path (decision 0065 slice two, U6e; MB3, CC2,
//! CR3, SC4), which `serve` takes up once its server has initialized
//! (U6f1b). The harness's
//! frames are read whole, at most 1 MiB and 64 deep, and each is checked
//! before anything is recorded: initialize, answered at MB3's version
//! whatever version it asks for (U9b1c), ping and the
//! filtered listing are answered here, a cancellation of the active call
//! is passed on, notifications are taken, and any other method refused. A
//! tools/call naming a tool in seat-record's vocabulary within SC4's 256
//! bytes is accepted only once its Started is durable, for which the
//! ledger keeps room: at its call bound the harness is still read, and the
//! next call ends the session with the ledger's limit. An accepted call to
//! a tool the plan does not grant, or made while another is active, is
//! refused with a durable Terminal and forwarded nowhere, and so is one a
//! retaining plan's share has no room left to reserve for (U8b; CR3); any
//! other is forwarded, one at a time, under its own absolute deadline and
//! with a
//! request id of the broker's own, which its response must carry exactly,
//! in JSON-RPC's and MCP's shapes. Its request, and its cancellation, which
//! is passed on once, are queued for the server and written as it takes
//! them while the harness is still read, so a server that takes neither
//! holds nothing past the deadline that applies. Its Terminal is durable
//! before the response is delivered, and a retaining plan's prepared
//! response is staged and synced before that Terminal, which cites it
//! (CR2), within the call's own deadline, never past
//! MB3's 8 MiB with the harness's own id, and a cancelled call's response
//! is never delivered. What the harness is told is queued too, whole frames
//! in order, each with the deadline that applies to it and cut at its safe
//! points, and written a piece at a time as far as the harness takes it on
//! every turn ([`outbox`]), so the harness is still read
//! while it leaves its output full, and a cancellation behind an answer it
//! has not taken still settles within MB3's two seconds. It is not read
//! while 64 frames, or MB3's 8 MiB, wait for it, and an active call then
//! has those two seconds, from when they first did, to end, so that a
//! cancellation left unread behind them settles as soon. The attempt's
//! deadline and cancellation are read before every step, idle or not; a
//! frame not taken by its deadline is never begun or continued, and ends
//! the session, as does one left untaken once the harness has ended its
//! input. A fatal ending fails the active call and ends the session with
//! its cause, as does a harness that ends while a call is active; a
//! record that cannot be made ends it before anything acts on it.

use std::fmt::Display;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_protocol::broker::ledger::{Call, Denial, Failure, Ledger, Outcome, Unrecorded};
use brokkr_protocol::broker::Plan;
use brokkr_protocol::secret::{BoundSecret, Guard};
use serde::Deserialize;
use serde_json::{json, Value};

use super::output::{self, Answered, Edge, Kind, Named};
use super::pieces::Pieces;
use super::VERSION;
use super::{Catalog, Fatal, Fault, Heard, Id, Keyed, Line, Lines, Refused, Reply, Server};
use super::{MASKING, PROTOCOL, REQUEST_MAX, RESPONSE, RESPONSE_MAX, TIMEOUT, UNDELIVERED};
use crate::broker::session::stderr::Drain;
use crate::broker::session::{ensure, polled, timely, Ended, Pace, Startup};
use request::{decoded, Called, Request};

pub(super) use outbox::{Delivery, Outbox};

mod outbox;
mod request;

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

/// The longest frame the harness is told: MB3's bound on a response, and
/// the newline every frame ends with.
const FRAME_MAX: usize = RESPONSE_MAX + 1;

/// The JSON-RPC error code of a session's own failure.
const INTERNAL: i64 = -32603;

/// The ledger's limit, the session's cause once it has no room.
const LIMIT: Fatal = Fatal::Failed(Failure::Limit);

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

/// The server's `reply` prepared for the harness at the output edge
/// ([`output::prepared`]), and the outcome it says: read before masking,
/// where a result not in MCP's shape is the server's protocol fault and
/// never a tool's outcome. Masking never touches what says it.
pub(super) fn delivered(
    reply: Reply,
    bindings: &[BoundSecret],
) -> Result<(Outcome, Vec<u8>), Fatal> {
    let outcome = match &reply {
        Reply::Result(result) => outcome(result).ok_or(PROTOCOL)?,
        Reply::Error(_) => Outcome::Failed,
    };
    let prepared = output::prepared(reply, Kind::Call, bindings)?;
    Ok((outcome, prepared))
}

/// The broker's own JSON-RPC error: `code`, and `message`'s words.
fn fault(code: i64, message: &dyn Display) -> Reply {
    Reply::Error(Fault {
        code,
        message: message.to_string(),
        data: None,
        _keyed: Keyed {},
    })
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

/// A session beside its server: the plan, its catalog prepared at the
/// output edge, the bound secrets every output is masked by and their final
/// guard, the server's drained standard error, its ledger, the harness's
/// output, what waits for it there, since when that has been at its bound,
/// and what became of it, the attempt's deadline and cancellation, the
/// bound on one call, and the one forwarded call.
struct Session<'s, 'l, 'g, W, E> {
    plan: &'s Plan,
    catalog: Vec<u8>,
    bindings: &'s [BoundSecret],
    guard: Guard<'s>,
    stderr: &'s mut Drain<'g, E>,
    server: Server<'l>,
    ledger: &'s mut Ledger<'g>,
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
/// with the timeout cause. A call still active when the harness ends is
/// failed, and so is the session ([`Session::hung_up`]). Everything the
/// session says
/// reaches the harness only through the output edge, masked by the `edge`'s
/// bindings and guarded whole: the catalog before the harness's first frame
/// is read, and each response before it is staged or its Terminal
/// recorded. Those
/// bindings guard the ledger already, which was bound once, as soon as
/// they were looked up ([`Ledger::bind`]), and is never bound again here.
/// The server's standard error, drained since its startup, is drained every
/// turn; its masked tail is the caller's to write once the session ends.
pub(in crate::broker) fn served<'b>(
    started: (Server<'_>, Catalog),
    plan: &Plan,
    ledger: &mut Ledger<'b>,
    harness: (impl Read + AsFd, impl Write + AsFd),
    attempt: Startup<'_>,
    edge: Edge<'_, 'b, impl Read + AsFd>,
) -> Result<(), Ended> {
    served_within(
        (CALL, std::thread::sleep),
        started,
        plan,
        ledger,
        harness,
        attempt,
        edge,
    )
}

/// [`served`], each forwarded call bound by `call`, and an idle turn
/// waiting with `wait`. Tests can observe the wait decisions directly.
pub(super) fn served_within<'b>(
    (call, wait): (Duration, impl FnMut(Duration)),
    (server, catalog): (Server<'_>, Catalog),
    plan: &Plan,
    ledger: &mut Ledger<'b>,
    (input, output): (impl Read + AsFd, impl Write + AsFd),
    attempt: Startup<'_>,
    edge: Edge<'_, 'b, impl Read + AsFd>,
) -> Result<(), Ended> {
    let mut harness = Lines::new(input);
    let guard = Guard::new(edge.bindings);
    let listing = Reply::Result(json!({"tools": catalog.0}));
    let listing = output::prepared(listing, Kind::Listing, edge.bindings).and_then(|listing| {
        let passed = guard.json_within(&listing, output::watched(attempt))?;
        ensure(passed, MASKING).map(|()| listing)
    });
    let mut session = Session {
        plan,
        catalog: listing.map_err(Ended::Failed)?,
        bindings: edge.bindings,
        guard,
        stderr: edge.stderr,
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
    match session.serving(&mut harness, wait) {
        Ok(()) => session.drained(attempt.0),
        Err(ended) => {
            session.drained(Instant::now() + CLOSING).ok();
            Err(ended)
        }
    }
}

impl<'s, W: Write + AsFd, E: Read + AsFd> Session<'s, '_, '_, W, E> {
    /// Serve the frames `harness` sends until it ends its input or its
    /// output, or a cause ends the session first: its next frame read only
    /// while what waits for the harness has room, and only while the
    /// ledger has room or a call is active; then, unless answering it found
    /// the harness's output failed or closed, whose cause stays the first
    /// ([`Session::going`]), the server heard, its standard error drained,
    /// and what waits for the harness written as far as it takes it, every
    /// turn. A turn waits ([`Pace`]) only where nothing moved: no byte read
    /// from the harness or the server, and none written to either. A
    /// harness that ends while a call is active hung up on it
    /// ([`Session::hung_up`]).
    fn serving(
        &mut self,
        harness: &mut Lines<impl Read + AsFd>,
        mut wait: impl FnMut(Duration),
    ) -> Result<(), Ended> {
        let mut pace = Pace::new();
        while self.going()? {
            let room = self.ledger.room().is_ok();
            ensure(room | self.active.is_some(), Ended::Failed(LIMIT))?;
            let Some(quiet) = self.asked(harness)? else {
                break;
            };
            if !self.going()? {
                break;
            }
            let heard = self.heard()?;
            let spoke = self.stderr.drained();
            let told = self.flush();
            pace.after(!(heard & quiet) | spoke | told, &mut wait);
        }
        self.hung_up()
    }

    /// The harness has ended, by closing its input or its output, politely
    /// or because it was killed and its exit closed them (operator ruling
    /// 2026-10-18, item 6): a call still active is failed, its Terminal
    /// recorded, and the session ends with MB5's cancellation cause, the
    /// attempt's timeout, so its ledger ends with that Terminal and a
    /// Failed closure, never unfinished; with no call active, it ends
    /// served.
    fn hung_up(&mut self) -> Result<(), Ended> {
        match self.active {
            Some(_) => self.failed(TIMEOUT),
            None => Ok(()),
        }
    }

    /// Read the harness's next frame from `harness`, unless what waits for
    /// it has no room ([`Session::untaken`]), and take it: a request
    /// handled, an oversized frame refused. Whether the turn read nothing;
    /// or none once the harness has ended its input.
    fn asked(&mut self, harness: &mut Lines<impl Read + AsFd>) -> Result<Option<bool>, Ended> {
        let asked = match self.untaken() {
            false => harness.next(REQUEST_MAX),
            true => Line::Quiet,
        };
        match asked {
            Line::Frame(frame) => self.requested(&frame).map(|()| Some(false)),
            Line::Over => self.refuse(None, &Refused::Limit).map(|()| Some(false)),
            Line::Quiet => Ok(Some(true)),
            Line::Partial => Ok(Some(false)),
            Line::Ended => Ok(None),
        }
    }

    /// Whether the session goes on: not once the harness has closed its
    /// output; not, ending it as a delivery failure, once its output took a
    /// piece short or not at all, or failed otherwise; and, ending it with
    /// a timeout, not past the attempt's deadline or its cancellation, nor
    /// once the harness has left an answer untaken past its own.
    fn going(&mut self) -> Result<bool, Ended> {
        match (self.delivery, timely(self.attempt)) {
            (Delivery::Undelivered, _) => self.failed(UNDELIVERED),
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

    /// Tell the harness's request `id` the broker's own `reply` as one
    /// frame by `deadline`, prepared and guarded at the output edge like
    /// any other ([`output::framed`]); or nothing, and why.
    fn told(&mut self, id: Option<&Id>, reply: Reply, deadline: Instant) -> Result<(), Fatal> {
        let body = output::prepared(reply, Kind::Local, self.bindings)?;
        let watch = output::watched((deadline, self.attempt.1));
        let line = output::framed(id, &body, &self.guard, watch)?;
        self.write(line, deadline);
        Ok(())
    }

    /// Tell the harness's request `id` `reply` by [`Session::bound`]; or,
    /// where the edge refuses that frame, nothing, and end the session
    /// with its cause.
    fn tell(&mut self, id: Option<&Id>, reply: Reply) -> Result<(), Ended> {
        let told = self.told(id, reply, self.bound());
        told.or_else(|fatal| self.failed(fatal))
    }

    /// Queue the guarded frame `line` for the harness, its newline
    /// included, to be taken whole by `deadline`, after every frame queued
    /// before it; then write what it takes now, which is nothing once its
    /// output is closed or stalled ([`Session::flush`]).
    fn write(&mut self, line: Pieces, deadline: Instant) {
        self.outbox.frames.push_back((line, deadline));
        self.flush();
    }

    /// Write what waits for the harness as far as it takes it now, unless
    /// its output is already closed or stalled, a piece at a time, each
    /// only while its frame's deadline is ahead and the attempt, whose
    /// harness is then told nothing more, is not cancelled
    /// ([`Outbox::flushed`]): a frame it has not taken whole by its
    /// deadline is the last, and ends the session. Whether it took any.
    fn flush(&mut self) -> bool {
        let waiting = (self.outbox.frames.len(), self.outbox.taken);
        if self.delivery == Delivery::Open {
            self.delivery = self.outbox.flushed(&mut self.output, self.attempt.1);
        }
        waiting != (self.outbox.frames.len(), self.outbox.taken)
    }

    /// Wait, until `until` unless the attempt is cancelled, for the harness
    /// to take what waits for it, each frame by its own deadline, written
    /// again at once while it takes some: the delivery failure where it
    /// took a piece short or not at all, or failed otherwise; the timeout
    /// cause where it has not taken it all by then and has not closed its
    /// output.
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
            (_, Delivery::Undelivered) => Err(Ended::Failed(UNDELIVERED)),
            _ => Err(Ended::Failed(TIMEOUT)),
        }
    }

    /// Refuse the harness's request `id` with `refused`'s own words.
    fn refuse(&mut self, id: Option<&Id>, refused: &Refused) -> Result<(), Ended> {
        self.tell(id, fault(refused.code(), refused))
    }

    /// Take the harness's frame `bytes`: its id first found safe to answer
    /// by ([`output::correlated`]), before anything is forwarded, recorded
    /// or answered, or the session ended with the masking cause and nothing
    /// made of the frame; then answered here, accepted as a call, a
    /// cancellation passed on, taken as a notification, or refused before
    /// anything is recorded.
    fn requested(&mut self, bytes: &[u8]) -> Result<(), Ended> {
        let request = decoded(bytes);
        let id = match &request {
            Ok(request) => request.id(),
            Err((id, _)) => id.as_ref(),
        };
        let unsafe_id = id.map(|id| output::correlated(id, &self.guard));
        if let Some(Err(fatal)) = unsafe_id {
            return self.failed(fatal);
        }
        let request = match request {
            Ok(request) => request,
            Err((id, refused)) => return self.refuse(id.as_ref(), &refused),
        };
        match request {
            Request::Initialize(id) => self.greet(&id),
            Request::Ping(id) => self.tell(Some(&id), Reply::Result(json!({}))),
            Request::List(id) => self.listing(&id),
            Request::Call(id, called) => self.call(id, called),
            Request::Cancel(id) => {
                self.cancel(&id);
                Ok(())
            }
            Request::Told => Ok(()),
            Request::Other(id) => self.refuse(Some(&id), &Refused::Method),
        }
    }

    /// Answer the harness's initialize `id` as the plan's server, at MB3's
    /// version and the dialect's pinned one, offering tools alone: MB3's
    /// version whatever version it asked, as MCP 2025-06-18's lifecycle
    /// negotiates one (U9b1c), and the session held to it.
    fn greet(&mut self, id: &Id) -> Result<(), Ended> {
        let server = json!({"name": self.plan.server, "version": self.plan.dialect.version});
        let tools = json!({"tools": {}});
        let result =
            json!({"protocolVersion": VERSION, "capabilities": tools, "serverInfo": server});
        self.tell(Some(id), Reply::Result(result))
    }

    /// Answer the harness's listing `id` with the filtered catalog, the
    /// same whatever the server has since said of its tools; or, where
    /// that answer with its id would pass MB3's bound on a response, tell
    /// it so, never a truncated listing, and end the session; or, where
    /// the edge refuses the frame, end it with that cause.
    fn listing(&mut self, id: &Id) -> Result<(), Ended> {
        let watch = output::watched((self.bound(), self.attempt.1));
        let line = match output::framed(Some(id), &self.catalog, &self.guard, watch) {
            Ok(line) => line,
            Err(fatal) => return self.failed(fatal),
        };
        if line.bytes().len() > FRAME_MAX {
            self.tell(Some(id), fault(INTERNAL, &RESPONSE))?;
            return self.failed(RESPONSE);
        }
        self.write(line, self.bound());
        Ok(())
    }

    /// Accept the harness's call `called`, its request `id`: its Started
    /// recorded, then refused here where its tool is not granted, another
    /// call is active or a retaining plan's share has no room left for its
    /// response, with its refused Terminal recorded; otherwise forwarded.
    /// A call that cannot be recorded is neither refused nor
    /// forwarded, and ends the session; one made at the ledger's call
    /// bound ends it with the ledger's limit, failing the active call. A
    /// tool name holding a bound secret is never recorded, refused by or
    /// forwarded as itself ([`output::named`]): the call is recorded under
    /// its masked name alone, refused as the ungranted tool it is, and the
    /// session ends with the masking cause.
    fn call(&mut self, id: Id, called: Called) -> Result<(), Ended> {
        let (recorded, masked) = match output::named(&called.name, self.bindings, &self.guard) {
            Named::Safe => (called.name.clone(), false),
            Named::Masked(name) => (name, true),
        };
        let begun = self.ledger.room().map(|room| room.begin(&recorded));
        let mut call = match begun {
            Ok(Ok(call)) => call,
            Ok(Err(unrecorded)) => {
                self.told(Some(&id), fault(INTERNAL, &unrecorded), self.bound())
                    .ok();
                return Err(unrecorded.into());
            }
            Err(_) => return self.failed(LIMIT),
        };
        if masked {
            self.ended(call, Outcome::Refused(Denial::Ungranted), &id)?;
            self.told(Some(&id), fault(INTERNAL, &MASKING), self.bound())
                .ok();
            return self.failed(MASKING);
        }
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
        // A call that may be forwarded reserves room for its response
        // first, where the plan retains it, or is refused for the budget
        // before any external work (CR3).
        let budget = |denial| (denial, Refused::Budget);
        let denied = denied.or_else(|| self.ledger.reserve(&mut call).err().map(budget));
        let Some((denial, refused)) = denied else {
            self.forwarded(call, id, called);
            return Ok(());
        };
        self.ended(call, Outcome::Refused(denial), &id)?;
        self.refuse(Some(&id), &refused)
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
    /// call's, its id first found safe to correlate by, and the reply
    /// prepared at the output edge ([`delivered`]); then, where the plan
    /// retains its responses, the very prepared bytes staged and synced,
    /// and its Terminal recorded citing them, or else its Terminal alone;
    /// then those bytes delivered as the answer to the harness's own id
    /// within the call's own deadline unless the harness cancelled the
    /// call (CR2, CR3). An id of either side holding a bound secret, a
    /// response to anything else, a reply the edge refuses, as one it
    /// cannot safely mask, or an answer that would pass MB3's bound on a
    /// response ends the session, failing the call before anything is
    /// staged or its Terminal recorded, whatever the plan retains, and
    /// nothing of it is delivered.
    fn answered(&mut self, id: Id, reply: Reply) -> Result<(), Ended> {
        let bound = self.bound();
        let watch = output::watched((bound, self.attempt.1));
        output::correlated(&id, &self.guard).or_else(|fatal| self.failed(fatal))?;
        let Some(active) = self.active.take_if(|active| active.asked == id) else {
            return self.failed(PROTOCOL);
        };
        let answer = delivered(reply, self.bindings).and_then(|(outcome, body)| {
            let line = output::framed(Some(&active.id), &body, &self.guard, watch)?;
            let fits = line.bytes().len() <= FRAME_MAX;
            ensure(fits, RESPONSE).map(|()| (outcome, body, line))
        });
        let (outcome, body, line) = match answer {
            Ok(answer) => answer,
            Err(fatal) => {
                self.active = Some(active);
                return self.failed(fatal);
            }
        };
        let digest = canonical::sha256_bytes(&body);
        let recorded = self.ledger.answered(active.call, outcome, (&body, &digest));
        self.recorded(recorded, &active.id)?;
        if !active.cancelled {
            self.write(line, bound);
        }
        Ok(())
    }

    /// `call`'s Terminal recorded with `outcome`, citing no response.
    fn ended(&mut self, call: Call, outcome: Outcome, id: &Id) -> Result<(), Ended> {
        let recorded = self.ledger.end(call, outcome);
        self.recorded(recorded, id)
    }

    /// Nothing where the Terminal was `recorded`; or the harness told its
    /// request `id` it could not be, nothing else delivered, and the
    /// session ended ([`Ended::from`]).
    fn recorded(&mut self, recorded: Result<(), Unrecorded>, id: &Id) -> Result<(), Ended> {
        recorded.map_err(|unrecorded| {
            self.told(Some(id), fault(INTERNAL, &unrecorded), self.bound())
                .ok();
            Ended::from(unrecorded)
        })
    }

    /// End the session with `fatal`, which stays its first cause: the
    /// active call, if any, failed and its Terminal recorded, and the
    /// harness, unless it cancelled the call, told why; or, where that
    /// Terminal could not be recorded, told so instead, and where the
    /// ledger's guard refused it, the session ended with the masking cause.
    /// That last frame has [`CLOSING`] to be taken, and no longer, and is
    /// never made where the edge refuses it ([`output::framed`]).
    fn failed<T>(&mut self, fatal: Fatal) -> Result<T, Ended> {
        let mut ended = Ended::Failed(fatal);
        if let Some(active) = self.active.take() {
            let told = match self.ledger.end(active.call, Outcome::Failed) {
                Ok(()) => fault(INTERNAL, &ended),
                Err(Unrecorded::Masking) => {
                    ended = Unrecorded::Masking.into();
                    fault(INTERNAL, &ended)
                }
                Err(unrecorded) => fault(INTERNAL, &unrecorded),
            };
            if !active.cancelled {
                self.told(Some(&active.id), told, Instant::now() + CLOSING)
                    .ok();
            }
        }
        Err(ended)
    }
}
