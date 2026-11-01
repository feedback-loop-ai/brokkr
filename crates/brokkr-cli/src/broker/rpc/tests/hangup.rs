//! A harness that hangs up by a signal (decision 0065 slice two, U9b1e;
//! CR3, CR4): its input is read no more, as a closed one is not, and its
//! session ends served once no call is active; an active call is still
//! heard until its answer, a fatal ending or the attempt's end, and a
//! fatal cause stays the first. The real signals are the CLI suite's.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use brokkr_protocol::broker::ledger::{Failure, Outcome};
use serde_json::json;

use super::{admitted, answer, call, cancellable, echoed, edged, lookup_failed, mcp, mcp_but};
use super::{request, result, say, within, Beside, Harness, Ran, Script, Then};
use crate::broker::session::Startup;

/// Whether the harness hung up, and the flag that hangs it up.
fn hangup() -> (&'static dyn Fn() -> bool, &'static AtomicBool) {
    let flag: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
    (
        Box::leak(Box::new(move || flag.load(Ordering::SeqCst))),
        flag,
    )
}

/// The session served over `script`'s server within `attempt`, its harness
/// speaking `conversation` and hung up once `hung_up` says so.
fn hung(
    script: Script,
    hung_up: &'static dyn Fn() -> bool,
    attempt: Startup<'static>,
    conversation: impl FnOnce(&mut Harness) + Send + 'static,
) -> Ran {
    let beside = Beside {
        hung_up,
        ..Beside::plain()
    };
    edged(script, (None, (attempt, None)), beside, conversation)
}

/// A server that hangs its harness up as it reads a forwarded call, so the
/// call was accepted first, and then, `after` that, says what `answering`
/// makes of the call's frame, if anything.
fn hanging_up_at_its_call(
    flag: &'static AtomicBool,
    after: Duration,
    answering: fn(&serde_json::Value) -> Option<serde_json::Value>,
) -> Script {
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/call").then(|| {
            flag.store(true, Ordering::SeqCst);
            std::thread::sleep(after);
            answering(frame).map(|answer| say(output, &answer));
            Then::Read
        })
    })
}

#[test]
fn a_harness_hung_up_by_a_signal_ends_its_session_served_with_its_input_open() {
    let (hung_up, flag) = hangup();
    let ran = hung(mcp(), hung_up, within(Duration::from_secs(10)), |harness| {
        harness.send(&request(json!(1), "ping", json!({})));
        assert_eq!(harness.next(), Some(result(&json!(1), json!({}))));
        flag.store(true, Ordering::SeqCst);
        // Its input stays open: only the hang-up ends the session.
        assert_eq!(harness.next(), None);
    });
    assert_eq!((ran.ended, ran.proved), (Ok(()), admitted(vec![])));
}

#[test]
fn a_call_active_when_its_harness_hangs_up_is_heard_until_its_answer() {
    // Answered after the hang-up: delivered, recorded succeeded, and the
    // session ended served.
    let (hung_up, flag) = hangup();
    let script = hanging_up_at_its_call(flag, Duration::from_millis(300), answer);
    let ran = hung(
        script,
        hung_up,
        within(Duration::from_secs(10)),
        |harness| {
            harness.send(&call("1", "lookup"));
            assert_eq!(harness.next(), Some(echoed("1", "lookup")));
            assert_eq!(harness.next(), None);
        },
    );
    let served = admitted(vec![("lookup", Some(Outcome::Succeeded))]);
    assert_eq!((ran.ended, ran.proved), (Ok(()), served));
    // Answered by a response to anything else: the protocol's fatal ending
    // is the first cause, and the hang-up never makes it Clean.
    let (hung_up, flag) = hangup();
    let stray = |_: &serde_json::Value| Some(result(&json!(99), json!({})));
    let script = hanging_up_at_its_call(flag, Duration::from_millis(300), stray);
    let ran = hung(
        script,
        hung_up,
        within(Duration::from_secs(10)),
        |harness| {
            harness.send(&call("1", "lookup"));
            let told = harness.next().unwrap();
            assert_eq!(told["error"]["message"], Failure::Protocol.to_string());
        },
    );
    lookup_failed("a stray answer", ran, Failure::Protocol);
}

#[test]
fn a_call_still_unanswered_when_the_attempt_ends_after_a_hang_up_fails_by_the_timeout() {
    // The session waits on its silent call past the hang-up, telling the
    // harness nothing, until the attempt's cancellation, a second signal's,
    // fails the call at once, its harness told nothing more.
    let (hung_up, flag) = hangup();
    let (attempt, cut) = cancellable(Duration::from_secs(10));
    let script = hanging_up_at_its_call(flag, Duration::ZERO, |_| None);
    let begun = Instant::now();
    let ran = hung(script, hung_up, attempt, move |harness| {
        harness.send(&call("1", "lookup"));
        while !flag.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(300));
        cut.store(true, Ordering::SeqCst);
        assert_eq!(harness.next(), None);
    });
    lookup_failed("hung up, then cut", ran, Failure::Timeout);
    assert!(begun.elapsed() < Duration::from_secs(3));
}
