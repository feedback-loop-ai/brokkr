//! What the harness is told (decision 0065 slice two, U6e, U6f1b; MB3,
//! SD3): whole frames in order, bounded while they wait, never begun past
//! their deadlines, and written as fast as the harness takes them; and a
//! retained plan served as any other.

use std::cell::RefCell;
use std::io::{PipeReader, Read, Write};
use std::os::fd::AsFd;
use std::rc::Rc;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{recover, Disposition, Failure, Ledger, Outcome};
use brokkr_protocol::broker::Plan;
use rustix::io::Errno;
use serde_json::{json, Value};

use super::super::calls::{served, served_within, Delivery, Outbox};
use super::super::output::Edge;
use super::super::pieces::tests::{held, refusing, sink, Sink};
use super::super::pieces::Pieces;
use super::super::Ending;
use super::super::{initialized, started, Fatal, ATOMIC, REQUEST_MAX, RESPONSE_MAX, UNDELIVERED};
use super::retention::Retain;
use super::{admitted, answering_with, call, cancelled_behind, conversed, forwarded, idle_timeout};
use super::{answer, live, mcp_but, tools, within, Harness, Script, Then, DIGEST};
use super::{fake, lookup_failed, mcp, passed, ping_sized, pipe, request, result, session, silent};
use crate::broker::session::lifetime::settled;
use crate::broker::session::stderr::Drain;
use crate::broker::session::Ended;

/// A served session no signal ends.
const UNSIGNALLED: Ending<'static> = (&live, &live);

#[test]
fn what_waits_for_the_harness_is_bounded_and_taken_whole_in_order() {
    // An answer larger than the pipe, then a small one queued behind it:
    // each is taken whole, in order, once the harness reads; and so where
    // it ended its input first, while the session ends. A notification
    // larger than the harness's input pipe holds, which the broker answers
    // nothing, follows them: its send returns only once the broker has
    // read past the small ping, so that its answer is queued behind the
    // large one, which the harness has not begun to take, however either
    // thread is scheduled.
    for ends in [false, true] {
        let (big, id) = ping_sized(512 << 10);
        let message = "n".repeat(512 << 10);
        let params = json!({"progressToken": "t", "progress": 0, "message": message});
        let told = json!({"jsonrpc": "2.0", "method": "notifications/progress", "params": params});
        let conversation = move |harness: &mut Harness| {
            harness.send(&big);
            harness.send(&request(json!("b"), "ping", json!({})));
            harness.send(&told);
            if ends {
                drop(harness.to.take());
            }
            assert_eq!(harness.next(), Some(result(&id, json!({}))));
            assert_eq!(harness.next(), Some(result(&json!("b"), json!({}))));
        };
        let ended = conversed(mcp(), conversation).ended;
        assert_eq!((ends, ended), (ends, Ok(())));
    }
    // Behind a blocked answer, 63 frames waiting leave the cancellation read
    // and 64 do not; nor do MB3's 8 MiB, where eight answers of a 1 MiB
    // request do not reach them and nine do, as the frames the server read
    // show: the cancellation passed on, or not. Either way, with the call's
    // own deadline MB3's 120 seconds and the attempt's thirty, which bounds
    // it, the call settles, and the session ends, before the attempt's
    // deadline and well within half of it from the cancellation, so that
    // neither deadline ended it, however the host is loaded; one whose
    // cancellation was read is given MB3's two seconds from then, which
    // load only lengthens; and it is never retried.
    let small = |count: usize| {
        let pings = (0..count).map(|at| request(json!(at), "ping", json!({})));
        std::iter::once(ping_sized(512 << 10).0)
            .chain(pings)
            .collect()
    };
    let large = |count: usize| vec![ping_sized(REQUEST_MAX - 64).0; count];
    let cases: [(&str, Vec<Value>, bool); 4] = [
        ("63 frames", small(62), true),
        ("64 frames", small(63), false),
        ("8 answers", large(8), true),
        ("9 answers", large(9), false),
    ];
    let ceiling = Duration::from_secs(15);
    for (case, pings, read) in cases {
        let (held, holding) = channel();
        let attempt = within(Duration::from_secs(30));
        let ran = session(silent(), None, attempt, cancelled_behind(pings, held));
        let ended = Instant::now();
        let (cancelled, _, _) = holding.recv().unwrap();
        let mut sent = vec![forwarded(3, "lookup", json!({}))];
        sent.extend(read.then(|| passed(3)));
        assert_eq!((case, &ran.frames[3..]), (case, &sent[..]));
        let settled = ran.durable[2] - cancelled;
        let early = settled < Duration::from_secs(2);
        let before = (ran.durable[2] < attempt.0) & (settled < ceiling);
        assert!(before & !(read & early), "{case}: {settled:?}");
        let taken = ended - cancelled;
        let ended = (ended < attempt.0) & (taken < ceiling);
        assert!(ended, "{case}: {taken:?}");
        lookup_failed(case, ran, Failure::Timeout);
    }
}

#[test]
fn a_frame_past_its_deadline_is_never_begun_though_the_harness_would_take_it() {
    // Behind a frame still in time, one past its deadline: nothing of
    // either is written to an output that takes everything.
    let now = Instant::now();
    let later = now + Duration::from_secs(30);
    let frame = |bytes: &[u8]| Pieces::new(bytes.to_vec()).unwrap();
    let mut outbox = Outbox::default();
    outbox.frames.push_back((frame(b"{}\n"), later));
    outbox.frames.push_back((frame(b"[]\n"), now));
    let (mut reader, mut output) = pipe();
    rustix::io::ioctl_fionbio(&reader, true).unwrap();
    let mut taken = || {
        let mut chunk = [0; 64];
        reader
            .read(&mut chunk)
            .map_or(Vec::new(), |read| chunk[..read].to_vec())
    };
    let going = &|| false;
    assert_eq!(outbox.flushed(&mut output, going), Delivery::Stalled);
    assert_eq!((taken(), outbox.frames.len()), (vec![], 2));
    // Each in time: both taken whole, in order.
    outbox.frames.pop_back();
    outbox.frames.push_back((frame(b"[]\n"), later));
    assert_eq!(outbox.flushed(&mut output, going), Delivery::Open);
    assert_eq!((taken(), outbox.frames.len()), (b"{}\n[]\n".to_vec(), 0));
}

/// An outbox holding one frame of three pieces, `x`s cut before their
/// last, to be taken within `left`, and that frame.
fn three_pieces(left: Duration) -> (Outbox, Vec<u8>) {
    let bytes = [vec![b'x'; 2 * ATOMIC + 9], b"\n".to_vec()].concat();
    let mut outbox = Outbox::default();
    let deadline = Instant::now() + left;
    outbox
        .frames
        .push_back((Pieces::new(bytes.clone()).unwrap(), deadline));
    (outbox, bytes)
}

#[test]
fn a_frame_is_written_a_piece_at_a_time_and_nothing_more_past_a_cancellation_or_deadline() {
    // Each piece once the harness's output is ready: cancelled once the
    // first is taken, or past the frame's deadline, the one that applies
    // to it, once its write returns, the second is never offered, and
    // delivery has stalled; a write taken short ends delivery short, and
    // nothing more is offered (U6f1b; MB3; operator ruling 2026-10-13).
    let (long, going) = (Duration::from_secs(30), &|| false);
    let (mut outbox, bytes) = three_pieces(long);
    let (mut taking, reader) = sink(ATOMIC, usize::MAX, Duration::ZERO);
    let at = Arc::clone(&taking.at);
    let cancelled = move || !at.lock().unwrap().is_empty();
    assert_eq!(outbox.flushed(&mut taking, &cancelled), Delivery::Stalled);
    drop(taking);
    assert_eq!(held(reader), bytes[..ATOMIC].to_vec());
    let (mut outbox, bytes) = three_pieces(Duration::from_millis(100));
    let (mut slow, reader) = sink(ATOMIC, usize::MAX, Duration::from_millis(300));
    assert_eq!(outbox.flushed(&mut slow, going), Delivery::Stalled);
    drop(slow);
    assert_eq!(held(reader), bytes[..ATOMIC].to_vec());
    let attempt = going;
    let (mut outbox, _) = three_pieces(long);
    let (mut partial, reader) = sink(ATOMIC - 1, usize::MAX, Duration::ZERO);
    assert_eq!(outbox.flushed(&mut partial, attempt), Delivery::Undelivered);
    let writes = partial.at.lock().unwrap().len();
    drop(partial);
    assert_eq!((held(reader).len(), writes), (ATOMIC - 1, 1));
    // Taken whole, piece by piece.
    let (mut outbox, bytes) = three_pieces(long);
    let (mut whole, reader) = sink(ATOMIC, usize::MAX, Duration::ZERO);
    assert_eq!(outbox.flushed(&mut whole, attempt), Delivery::Open);
    let writes = whole.at.lock().unwrap().len();
    drop(whole);
    assert_eq!((held(reader), writes, outbox.frames.len()), (bytes, 3, 0));
}

#[test]
fn an_output_that_takes_a_piece_short_ends_the_session_as_a_delivery_failure() {
    // The harness pings, and its output takes three bytes of the answer's
    // one piece, as a sink that is not a pipe may: nothing more is offered,
    // the session ends as a delivery failure, and its ledger closes with
    // that cause, in the broker's own words, never Clean and never the
    // timeout cause (U6f1b; operator rulings 2026-10-13 and 2026-10-14).
    // So while it serves, and so once the harness has ended its input with
    // the answer still waiting for an output that was full until then.
    let undelivered = || Err(Ended::Failed(Fatal::Failed(Failure::Undelivered)));
    let failed = Some(Disposition::Failed(Failure::Undelivered));
    for full in [false, true] {
        let expected = (undelivered(), (Some(b"{\"j".to_vec()), 1), failed.clone());
        let short = sink(3, usize::MAX, Duration::ZERO);
        assert_eq!((full, pinged(full, short)), (full, expected));
    }
    let said = UNDELIVERED.to_string();
    assert_eq!(said, "MCP broker output could not be delivered");
}

#[test]
fn only_a_closed_harness_output_keeps_a_sessions_ending() {
    // The harness pings, and its output refuses every write: a broken
    // pipe, its reader gone, keeps the session's ending, served and Clean;
    // a full device, an I/O error or a write that takes nothing is the
    // delivery failure, never Clean and never the timeout cause (operator
    // ruling 2026-10-14).
    let undelivered = || Err(Ended::Failed(UNDELIVERED));
    let failed = Some(Disposition::Failed(Failure::Undelivered));
    for (case, output, expected) in [
        (
            "EPIPE",
            refusing(Errno::PIPE),
            (Ok(()), 0, Some(Disposition::Clean)),
        ),
        (
            "ENOSPC",
            refusing(Errno::NOSPC),
            (undelivered(), 0, failed.clone()),
        ),
        (
            "EIO",
            refusing(Errno::IO),
            (undelivered(), 0, failed.clone()),
        ),
        (
            "zero",
            sink(0, usize::MAX, Duration::ZERO),
            (undelivered(), 1, failed.clone()),
        ),
    ] {
        let (ended, (wrote, writes), closure) = pinged(false, output);
        let expected = (case, expected.0, Some(vec![]), expected.1, expected.2);
        assert_eq!((case, ended, wrote, writes, closure), expected);
    }
}

#[test]
fn a_harness_output_closed_before_its_calls_answer_keeps_the_sessions_ending() {
    // The harness calls, and has closed its output by the time the server
    // answers: the answer, heard after the turn's check of the harness's
    // output, meets a broken pipe, and the session ends at the next turn's,
    // served and Clean, its call's Terminal recorded before.
    let body = json!({"content": []});
    let output = refusing(Errno::PIPE);
    let (ended, wrote, closure) =
        delivered(answering_with(tools(), body), &call("1", "lookup"), output);
    let expected = (Ok(()), (Some(vec![]), 0), Some(Disposition::Clean));
    assert_eq!((ended, wrote, closure), expected);
}

#[test]
fn an_earlier_cause_stands_where_the_harness_output_then_fails() {
    // The server's answer cannot be cut safely, which ends the session
    // with MB4's cause, and the harness's output then refuses the frame
    // telling it so with a full device: the first cause stands, and is
    // the ledger's closure (operator ruling 2026-10-14).
    let digits = "1".repeat(ATOMIC + 1);
    let body = json!({"content": [{"type": "text", "text": digits}]});
    let output = refusing(Errno::NOSPC);
    let (ended, wrote, closure) =
        delivered(answering_with(tools(), body), &call("1", "lookup"), output);
    let masking = Fatal::Failed(Failure::Masking);
    let expected = (Err(Ended::Failed(masking)), (Some(vec![]), 0));
    assert_eq!((ended, wrote), expected);
    assert_eq!(closure, Some(Disposition::Failed(Failure::Masking)));
}

#[test]
fn a_delivery_failure_found_answering_the_harness_stays_the_first_cause() {
    // The harness pings, its output refuses the answer with a full device,
    // and the server's stray response already waits to be heard in that
    // turn: the delivery failure stays the first cause, and is the ledger's
    // closure. Where the server's stray answer to a call is heard first,
    // and the output then refuses the frame telling the harness so, the
    // protocol's cause stands (operator ruling 2026-10-14).
    let ping = request(json!("p"), "ping", json!({}));
    for (case, early, frame, failure) in [
        ("delivery first", true, ping, Failure::Undelivered),
        (
            "server first",
            false,
            call("1", "lookup"),
            Failure::Protocol,
        ),
    ] {
        let output = refusing(Errno::NOSPC);
        let (ended, _, closure) = delivered(stray(early), &frame, output);
        let fatal = Fatal::Failed(failure.clone());
        let expected = (
            Err(Ended::Failed(fatal)),
            Some(Disposition::Failed(failure)),
        );
        assert_eq!((case, ended, closure), (case, expected.0, expected.1));
    }
}

/// A well-behaved server that, `early`, follows its listing in the same
/// write with a response to a request never made, so that it waits to be
/// heard as soon as the session begins; otherwise, one that answers a call
/// by an id it was never asked by.
fn stray(early: bool) -> Script {
    mcp_but(move |frame, output| {
        let stray = result(&json!(99), json!({}));
        let said = match (frame["method"].as_str(), early) {
            (Some("tools/list"), true) => format!("{}\n{stray}\n", answer(frame)?),
            (Some("tools/call"), false) => format!("{stray}\n"),
            _ => return None,
        };
        output.as_mut()?.write_all(said.as_bytes()).unwrap();
        Some(Then::Read)
    })
}

#[test]
fn a_harness_output_that_cannot_be_set_up_ends_the_session_undelivered() {
    // Standard output the broker cannot hold apart, once its server is
    // initialized: the delivery failure, its ledger closed with it; and
    // standard input, the protocol's cause (operator ruling 2026-10-14).
    let cases = [
        ("output", true, UNDELIVERED, Failure::Undelivered),
        (
            "input",
            false,
            Fatal::Failed(Failure::Protocol),
            Failure::Protocol,
        ),
    ];
    for (case, input, fatal, failure) in cases {
        let plan = super::plan();
        let (mut to, mut from, server) = fake(mcp());
        let mut stderr = Drain::new(pipe().0, &[]);
        let (mut ledger, appended) = in_memory(&plan);
        let edge = Edge {
            bindings: &[],
            stderr: &mut stderr,
        };
        let unheld = || Err(std::io::Error::from(Errno::MFILE));
        let held = || std::fs::File::open("/dev/null");
        let stdio = || match input {
            true => (held(), unheld()),
            false => (unheld(), held()),
        };
        let attempt = (&plan, within(Duration::from_secs(10)), UNSIGNALLED);
        let ended = initialized((&mut to, &mut from), &mut ledger, edge, attempt, stdio);
        let ended = settled(ledger, ended);
        drop((to, from));
        server.join().unwrap();
        let (lifecycle, _) = recover(&appended.borrow(), &plan.owner, DIGEST);
        let closed = lifecycle.closed().cloned();
        let expected = (
            case,
            Err(Ended::Failed(fatal)),
            Some(Disposition::Failed(failure)),
        );
        assert_eq!((case, ended, closed), expected);
    }
}

/// A ledger for `plan` held in memory and bound by no binding, and what it
/// has appended.
fn in_memory(plan: &Plan) -> (Ledger<'static>, Rc<RefCell<Vec<u8>>>) {
    let appended = Rc::new(RefCell::new(Vec::new()));
    let disk = Rc::clone(&appended);
    let append = move |line: &[u8]| {
        disk.borrow_mut().extend_from_slice(line);
        Ok(())
    };
    let mut ledger = Ledger::open_with(append, (&plan.owner, DIGEST), &[]).unwrap();
    ledger.bind(&[]).unwrap();
    (ledger, appended)
}

/// How a settled session ended, what its harness's output took past what
/// filled it, if it took what filled it first, and in how many writes, and
/// the closure its ledger proves.
type Short = (
    Result<(), Ended>,
    (Option<Vec<u8>>, usize),
    Option<Disposition>,
);

/// A session served to a harness that pings, its output `sink`, `full`
/// until the harness has ended its input, after which it is read
/// ([`Short`]).
fn pinged(full: bool, sink: (Sink, PipeReader)) -> Short {
    let ping = request(json!("p"), "ping", json!({}));
    asked(mcp(), &ping, full, sink)
}

/// [`pinged`], its server's `script` asked `frame`, its output never full.
fn delivered(script: Script, frame: &Value, sink: (Sink, PipeReader)) -> Short {
    asked(script, frame, false, sink)
}

/// A session served by the server's `script` to a harness that sends
/// `frame`, its output `sink` ([`pinged`]).
fn asked(script: Script, frame: &Value, full: bool, sink: (Sink, PipeReader)) -> Short {
    heard(None, script, (frame, full), sink).0
}

/// [`asked`], its plan retaining where `retain` is given and its ledger
/// retaining as `retain` has it: what it left ([`Short`]), each frame its
/// server read, and its ledger's bytes.
pub(super) fn heard(
    retain: Option<Retain>,
    script: Script,
    (frame, full): (&Value, bool),
    (short, reader): (Sink, PipeReader),
) -> (Short, Vec<Value>, Vec<u8>) {
    let mut plan = super::plan();
    plan.retained = retain.is_some();
    let (mut input, mut output, server) = fake(script);
    let mut stderr = Drain::new(pipe().0, &[]);
    let (mut ledger, appended) = in_memory(&plan);
    retain.into_iter().for_each(|retain| retain(&mut ledger));
    let attempt = within(Duration::from_secs(10));
    let link = (&mut input, &mut output);
    let started = started(link, &plan, attempt, &mut stderr).unwrap();
    let (asked, mut to) = pipe();
    writeln!(to, "{frame}").unwrap();
    let filled = match full {
        true => filled(&short),
        false => Vec::new(),
    };
    let to = (!full).then_some(to);
    let reading = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        held(reader)
    });
    let at = Arc::clone(&short.at);
    let edge = Edge {
        bindings: &[],
        stderr: &mut stderr,
    };
    let ended = served(
        started,
        &plan,
        &mut ledger,
        (asked, short, &live),
        attempt,
        edge,
    );
    let ended = settled(ledger, ended);
    drop((input, output, to));
    let frames = server.join().unwrap().0;
    let writes = at.lock().unwrap().len();
    let (lifecycle, invalid) = recover(&appended.borrow(), &plan.owner, DIGEST);
    assert_eq!(invalid, None);
    let wrote = reading.join().unwrap();
    let past = wrote.strip_prefix(&filled[..]).map(<[u8]>::to_vec);
    let short = (ended, (past, writes), lifecycle.closed().cloned());
    let bytes = appended.borrow().clone();
    (short, frames, bytes)
}

/// What fills the pipe `sink` writes to, a byte at a time, written through
/// a descriptor of its own until the pipe takes no more.
fn filled(sink: &impl AsFd) -> Vec<u8> {
    let mut fill = std::io::PipeWriter::from(sink.as_fd().try_clone_to_owned().unwrap());
    rustix::io::ioctl_fionbio(&fill, true).unwrap();
    let mut filled = Vec::new();
    while fill.write(b"x").is_ok() {
        filled.push(b'x');
    }
    rustix::io::ioctl_fionbio(&fill, false).unwrap();
    filled
}

#[test]
fn an_answer_with_512_bytes_and_no_safe_point_ends_the_session_masked() {
    // The server's answer holds a run of digits longer than one write:
    // nowhere can it be cut safely, so it is never delivered, and the
    // harness is told MB4's cause instead (U6f1b; operator ruling
    // 2026-10-13).
    let digits = "1".repeat(ATOMIC + 1);
    let body = json!({"content": [{"type": "text", "text": digits}]});
    let ran = conversed(answering_with(tools(), body), move |harness| {
        harness.send(&call("1", "lookup"));
        let cause = "MCP response cannot be safely masked";
        let told =
            json!({"jsonrpc": "2.0", "id": "1", "error": {"code": -32603, "message": cause}});
        assert_eq!(harness.next(), Some(told));
    });
    lookup_failed("digits", ran, Failure::Masking);
}

#[test]
fn an_answer_left_untaken_once_the_harness_ends_its_input_ends_the_session_timed_out() {
    // The harness pings with an id larger than its output pipe holds, ends
    // its input and reads nothing: the answer is not taken by its deadline,
    // the attempt's, and the session ends with the timeout cause, never as
    // served.
    let (ping, _) = ping_sized(512 << 10);
    let (held, holding) = channel();
    let begun = Instant::now();
    let attempt = within(Duration::from_millis(800));
    let ran = session(mcp(), None, attempt, move |harness| {
        harness.send(&ping);
        drop(harness.to.take());
        let from = std::mem::replace(&mut harness.from, pipe().0);
        held.send(from).unwrap();
    });
    drop(holding);
    idle_timeout("ended", ran, begun);
}

#[test]
fn an_answer_the_harness_takes_a_little_at_a_time_once_its_input_ends_is_taken_whole() {
    // The harness pings with an id of 8 KiB, which its input pipe holds
    // whole, and ends its input while its output is full, then takes the
    // answer a piece at a time, its output unready after each: the session,
    // its input ended, writes again at once each time the harness takes
    // some of what waits but not all, and ends served and Clean once the
    // harness has taken the answer whole, past what filled its pipe. Every
    // such turn takes a part and leaves the rest, however the harness's
    // reads are scheduled, where a harness reading all it can may take the
    // rest in the turn's one write.
    let (ping, id) = ping_sized(8 << 10);
    let (mut output, reader) = sink(ATOMIC, usize::MAX, Duration::ZERO);
    output.stutter = 1;
    let (ended, (wrote, _), closure) = asked(mcp(), &ping, true, (output, reader));
    let wrote = wrote.unwrap();
    let (frame, end) = wrote.split_at(wrote.len() - 1);
    let answer: Value = serde_json::from_slice(frame).unwrap();
    let expected = (Ok(()), result(&id, json!({})), &b"\n"[..]);
    assert_eq!((ended, answer, end), expected);
    assert_eq!(closure, Some(Disposition::Clean));
}

#[test]
fn a_near_limit_answer_is_delivered_whole_to_a_reading_harness() {
    // MB3's largest answer but 64 KiB, to a harness that reads all the
    // while. This proves delivery over real pipes. The pacing proof below
    // observes waits directly, independent of how either thread is scheduled.
    let text = "t".repeat(RESPONSE_MAX - (64 << 10));
    let body = json!({"content": [{"type": "text", "text": text}]});
    let answered = result(&json!("1"), body.clone());
    let ran = conversed(answering_with(tools(), body), move |harness| {
        harness.send(&call("1", "lookup"));
        assert_eq!(harness.next(), Some(answered));
    });
    let succeeded = vec![("lookup", Some(Outcome::Succeeded))];
    assert_eq!(
        (ran.ended, ran.proved),
        (Ok::<(), Ended>(()), admitted(succeeded))
    );
}

#[test]
fn a_turn_that_writes_part_of_an_answer_never_waits() {
    // Take exactly one piece per turn, then report WouldBlock. The empty
    // pipe supplies a pollable descriptor; writes are recorded in memory
    // so reader scheduling cannot decide whether a turn makes progress.
    // Each wait the session takes records how much of the answer the
    // harness had been given by then: a wait with any of it still to give
    // is a wait between pieces, which is the failure; one with the answer
    // whole is idle, and expected, since the harness's input ends only
    // once the last piece is written, and a child another test has forked
    // and not yet exec'd holds a copy of that pipe's write end until it
    // does, so its end can reach the session a wait or more later, however
    // the two are scheduled.
    struct Taking {
        pipe: std::io::PipeWriter,
        input: Option<std::io::PipeWriter>,
        bytes: Rc<RefCell<Vec<u8>>>,
        unready: bool,
        writes: Rc<std::cell::Cell<usize>>,
    }
    impl AsFd for Taking {
        fn as_fd(&self) -> std::os::fd::BorrowedFd<'_> {
            self.pipe.as_fd()
        }
    }
    impl Write for Taking {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if std::mem::take(&mut self.unready) {
                return Err(std::io::ErrorKind::WouldBlock.into());
            }
            self.bytes.borrow_mut().extend_from_slice(bytes);
            self.writes.set(self.writes.get() + 1);
            self.unready = true;
            if bytes.ends_with(b"\n") {
                drop(self.input.take());
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let plan = super::plan();
    let (mut input, mut output, server) = fake(mcp());
    let mut stderr = Drain::new(pipe().0, &[]);
    let (mut ledger, appended) = in_memory(&plan);
    let attempt = within(Duration::from_secs(30));
    let started = started((&mut input, &mut output), &plan, attempt, &mut stderr).unwrap();
    let (asked, mut to) = pipe();
    let (ping, id) = ping_sized(2 << 10);
    writeln!(to, "{ping}").unwrap();
    let (_held, descriptor) = pipe();
    let bytes = Rc::new(RefCell::new(Vec::new()));
    let writes = Rc::new(std::cell::Cell::new(0));
    let taking = Taking {
        pipe: descriptor,
        input: Some(to),
        bytes: Rc::clone(&bytes),
        unready: false,
        writes: Rc::clone(&writes),
    };
    let edge = Edge {
        bindings: &[],
        stderr: &mut stderr,
    };
    let waited = RefCell::new(Vec::new());
    let wait = |pause| {
        waited.borrow_mut().push(bytes.borrow().len());
        std::thread::sleep(pause);
    };
    let ended = served_within(
        (Duration::from_secs(120), wait),
        started,
        &plan,
        &mut ledger,
        (asked, taking, &live),
        attempt,
        edge,
    );
    assert_eq!(settled(ledger, ended), Ok(()));
    drop((input, output));
    server.join().unwrap();
    assert!(writes.get() > 1, "the answer must span multiple turns");
    let bytes = bytes.borrow();
    let between: Vec<usize> = waited
        .take()
        .into_iter()
        .filter(|given| *given < bytes.len())
        .collect();
    assert_eq!(
        between,
        Vec::<usize>::new(),
        "waited with that much of {} given",
        bytes.len()
    );
    let (frame, end) = bytes.split_at(bytes.len() - 1);
    let answer: Value = serde_json::from_slice(frame).unwrap();
    assert_eq!((answer, end), (result(&id, json!({})), &b"\n"[..]));
    let (lifecycle, invalid) = recover(&appended.borrow(), &plan.owner, DIGEST);
    assert_eq!(
        (lifecycle.closed(), invalid),
        (Some(&Disposition::Clean), None)
    );
}

#[test]
fn a_retained_plan_is_initialized_and_served_as_any_other() {
    // SD3's retention refusal is gone with U8b: a retaining plan's server
    // is initialized, and its harness, which ends at once, served, its
    // ledger closed Clean with nothing staged.
    let mut plan = super::plan();
    plan.retained = true;
    let (mut input, mut output, server) = fake(mcp());
    let mut stderr = Drain::new(pipe().0, &[]);
    let (mut ledger, appended) = in_memory(&plan);
    ledger.retain_with(|_, _: &[u8]| unreachable!("nothing is staged"), 1);
    let edge = Edge {
        bindings: &[],
        stderr: &mut stderr,
    };
    let link = (&mut input, &mut output);
    let attempt = (&plan, within(Duration::from_secs(10)), UNSIGNALLED);
    let null = |write| {
        std::fs::File::options()
            .write(write)
            .read(!write)
            .open("/dev/null")
    };
    let ended = initialized(link, &mut ledger, edge, attempt, || {
        (null(false), null(true))
    });
    let ended = settled(ledger, ended);
    drop((input, output));
    let frames = server.join().unwrap().0;
    let (lifecycle, _) = recover(&appended.borrow(), &plan.owner, DIGEST);
    let served = (Ok(()), super::initialization(), Some(&Disposition::Clean));
    assert_eq!((ended, frames, lifecycle.closed()), served);
}
