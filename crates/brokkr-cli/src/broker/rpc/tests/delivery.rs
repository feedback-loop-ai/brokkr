//! What the harness is told (decision 0065 slice two, U6e; MB3): whole
//! frames in order, bounded while they wait, never begun past their
//! deadlines, and written as fast as the harness takes them.

use std::io::Read;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{Failure, Outcome};
use serde_json::{json, Value};

use super::super::calls::{Delivery, Outbox};
use super::super::{REQUEST_MAX, RESPONSE_MAX};
use super::{admitted, answering_with, call, cancelled_behind, conversed, forwarded, idle_timeout};
use super::{lookup_failed, mcp, passed, ping_sized, pipe, request, result, session, silent};
use super::{tools, within, Harness, TURN};
use crate::broker::session::Ended;

#[test]
fn what_waits_for_the_harness_is_bounded_and_taken_whole_in_order() {
    // An answer larger than the pipe, then a small one queued behind it:
    // each is taken whole, in order, once the harness reads; and so where
    // it ended its input first, while the session ends.
    for ends in [false, true] {
        let (big, id) = ping_sized(512 << 10);
        let conversation = move |harness: &mut Harness| {
            harness.send(&big);
            harness.send(&request(json!("b"), "ping", json!({})));
            if ends {
                drop(harness.to.take());
            }
            std::thread::sleep(Duration::from_millis(200));
            assert_eq!(harness.next(), Some(result(&id, json!({}))));
            assert_eq!(harness.next(), Some(result(&json!("b"), json!({}))));
        };
        let ended = conversed(mcp(), conversation).ended;
        assert_eq!((ends, ended), (ends, Ok(())));
    }
    // Behind a blocked answer, 63 frames waiting leave the cancellation read
    // and 64 do not; nor do MB3's 8 MiB, where eight answers of a 1 MiB
    // request do not reach them and nine do. Either way, with the call's
    // own deadline MB3's 120 seconds and the attempt's thirty, the call
    // settles within two seconds of the cancellation, give or take one
    // second's scheduling on a loaded two-CPU host, and is never retried.
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
    for (case, pings, read) in cases {
        let (held, holding) = channel();
        let attempt = within(Duration::from_secs(30));
        let ran = session(silent(), None, attempt, cancelled_behind(pings, held));
        let ended = Instant::now();
        let (cancelled, _, _) = holding.recv().unwrap();
        let settled = ran.durable[2] - cancelled;
        let early = settled < Duration::from_secs(2);
        assert!(
            (settled < Duration::from_secs(3)) & !(read & early),
            "{case}: {settled:?}"
        );
        let taken = ended - cancelled;
        assert!(taken < Duration::from_secs(4), "{case}: {taken:?}");
        let mut sent = vec![forwarded(3, "lookup", json!({}))];
        sent.extend(read.then(|| passed(3)));
        assert_eq!((case, &ran.frames[3..]), (case, &sent[..]));
        lookup_failed(case, ran, Failure::Timeout);
    }
}

#[test]
fn a_frame_past_its_deadline_is_never_begun_though_the_harness_would_take_it() {
    // Behind a frame still in time, one past its deadline: nothing of
    // either is written to an output that takes everything.
    let now = Instant::now();
    let later = now + Duration::from_secs(30);
    let mut outbox = Outbox::default();
    outbox.frames.push_back((b"{}\n".to_vec(), later));
    outbox.frames.push_back((b"[]\n".to_vec(), now));
    let mut taken = Vec::new();
    assert_eq!(outbox.flushed(&mut taken), Delivery::Stalled);
    assert_eq!((taken.len(), outbox.frames.len()), (0, 2));
    // Each in time: both taken whole, in order.
    outbox.frames.pop_back();
    outbox.frames.push_back((b"[]\n".to_vec(), later));
    assert_eq!(outbox.flushed(&mut taken), Delivery::Open);
    assert_eq!((&taken[..], outbox.frames.len()), (&b"{}\n[]\n"[..], 0));
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
fn an_answer_is_written_as_fast_as_the_harness_takes_it() {
    // MB3's largest answer but 64 KiB, to a harness that reads all the
    // while. A turn that wrote some of it waits for nothing, so it is not
    // paced at one wait per pipe's worth: on Linux's 64 KiB pipes that is
    // 127 waits of 10 ms, over 1.27 seconds, and on macOS's 16 KiB ones
    // four times as many. Timed from its first byte, after the server's
    // answer has been read whole, it takes well under 800 ms.
    let text = "t".repeat(RESPONSE_MAX - (64 << 10));
    let body = json!({"content": [{"type": "text", "text": text}]});
    let answered = result(&json!("1"), body.clone());
    let ran = conversed(answering_with(tools(), body), move |harness| {
        harness.send(&call("1", "lookup"));
        let (until, mut first) = (Instant::now() + Duration::from_secs(20), [0]);
        while !matches!(harness.from.read(&mut first), Ok(1)) {
            assert!(Instant::now() < until, "the broker answered nothing");
            std::thread::sleep(TURN);
        }
        let begun = Instant::now();
        harness.held.push(first[0]);
        assert_eq!(harness.next(), Some(answered));
        let taken = begun.elapsed();
        assert!(taken < Duration::from_millis(800), "{taken:?}");
    });
    let succeeded = vec![("lookup", Some(Outcome::Succeeded))];
    assert_eq!(
        (ran.ended, ran.proved),
        (Ok::<(), Ended>(()), admitted(succeeded))
    );
}
