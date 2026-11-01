//! U9b1e's 46.8 (CR3, CR4, MB1): a harness that ends its broker by a signal,
//! as Claude Code 2.1.316 ends every MCP server it started with SIGINT,
//! rather than by closing its input. Through real dispatch, the harness keeps
//! the broker's input open until the broker has ended, so no end of input can
//! stand in for the signal: the broker closes its ledger and the run reads it.
//! A signal during admission is `windows`' (`a_signal_at_…`).

use std::time::Duration;

use serde_json::{json, Value};

use super::{put, World, ANSWERS, STARTS};

/// What the broker retains of the server's answer, and its digest.
const RETAINED: &str = r#"{"result":{"content":[{"text":"resolved","type":"text"}]}}"#;

/// A world whose harness sends `signals` to its broker once it has talked,
/// stopping at its call's Started where `mid_call`, and whose server runs
/// `script`.
fn signalling(signals: &str, mid_call: bool, script: Option<String>) -> Option<World> {
    let world = World::new()?;
    put(&world.bin("signal"), signals);
    if mid_call {
        put(&world.bin("mid-call"), "");
    }
    if let Some(script) = script {
        world.server(&script);
    }
    Some(world)
}

impl World {
    /// How long after the harness sent its `nth` signal, counted from 0, it saw
    /// its broker end.
    fn ended_after(&self, nth: usize) -> Duration {
        let read = |name: &str| std::fs::read_to_string(self.bin(name)).unwrap();
        let sent = read("signalled");
        let harness = read("harness.pid");
        let ended: u64 = read(&format!("ended.{}", harness.trim()))
            .trim()
            .parse()
            .unwrap();
        let sent: u64 = sent.lines().nth(nth).unwrap().parse().unwrap();
        Duration::from_nanos(ended - sent)
    }

    /// The broker's ledger's records, by kind, and its closure's disposition.
    fn closure(&self) -> (Vec<Value>, Value) {
        let records = self.ledger();
        let closed = records.last().unwrap()["disposition"].clone();
        (
            records
                .iter()
                .map(|record| record["record"].clone())
                .collect(),
            closed,
        )
    }
}

/// The records of a ledger whose one call ended, closed, and its closure.
fn called(closed: Value) -> (Vec<Value>, Value) {
    let kinds = ["opened", "started", "terminal", "closed"];
    (kinds.map(Value::from).to_vec(), closed)
}

/// The run of `world`, completed: its broker exiting 0, its ledger Closed
/// Clean, and its one call settled `succeeded`, its response retained and read
/// back through `inspect`. How long after the first signal the broker ended.
fn completed_clean(world: &World) -> Duration {
    let ended = world.completes();
    assert_eq!(world.broker_code(), Some(0));
    assert_eq!(world.closure(), called(json!("clean")));
    let digest = brokkr_core::canonical::sha256_bytes(RETAINED.as_bytes());
    assert_eq!(
        world.calls(&ended.run),
        [("resolve".into(), "succeeded".into(), Some(digest))]
    );
    let said: Value = serde_json::from_str(RETAINED).unwrap();
    assert_eq!(world.opened(&ended.run)["response"], said);
    world.ended_after(0)
}

/// The proof that a broker `signals` end once its call is answered: the run
/// completes clean ([`completed_clean`]), within seconds of the signal.
fn answered_then_ended_by(signals: &str) {
    let Some(world) = signalling(signals, false, None) else {
        return;
    };
    let took = completed_clean(&world);
    assert!(took < Duration::from_secs(10), "{signals}: {took:?}");
}

#[test]
fn a_harness_ending_its_broker_by_sigint_after_its_call_completes_with_the_ledger_closed_clean() {
    answered_then_ended_by("INT");
}

#[test]
fn a_harness_ending_its_broker_by_sigterm_after_its_call_completes_with_the_ledger_closed_clean() {
    answered_then_ended_by("TERM");
}

/// SIGINT while the call is in flight: the broker reads no more of its
/// harness, still hears its server, whose answer two seconds on settles the
/// call `succeeded` and is delivered, and then ends its session served.
#[test]
fn a_call_in_flight_at_sigint_settles_by_its_answer_and_the_ledger_closes_clean() {
    let late = format!("{STARTS}sleep 2\n{ANSWERS}\nread -r line\n");
    let Some(world) = signalling("INT", true, Some(late)) else {
        return;
    };
    let took = completed_clean(&world);
    assert!(
        took >= Duration::from_secs(1),
        "ended before its answer: {took:?}"
    );
    let heard = std::fs::read_to_string(world.bin("heard")).unwrap();
    let last = heard.lines().last().unwrap();
    assert!(last.contains(r#""text":"resolved""#), "{heard}");
}

/// SIGINT while the call's server never answers, then SIGTERM a second on,
/// as Claude Code follows its SIGINT: the second signal ends the wait, the
/// call failed by the timeout, its Terminal and a Failed closure recorded, and
/// the run parks on that cause past the harness's report of success.
#[test]
fn a_call_unanswered_when_a_second_signal_comes_fails_by_the_timeout_and_the_ledger_closes() {
    let silent = format!("{STARTS}exec sleep 600\n");
    let Some(world) = signalling("INT TERM", true, Some(silent)) else {
        return;
    };
    let ended = world.run();
    let timeout = brokkr_protocol::broker::ledger::Failure::Timeout.to_string();
    assert_eq!(
        (ended.status.as_str(), ended.cause.as_deref()),
        ("awaiting_operator", Some(timeout.as_str()))
    );
    assert_eq!(world.broker_code(), Some(1));
    let took = world.ended_after(1);
    assert!(took < Duration::from_secs(5), "after the second: {took:?}");
    assert_eq!(world.closure(), called(json!({"failed": "timeout"})));
    assert_eq!(
        world.calls(&ended.run),
        [("resolve".into(), "failed".into(), None)]
    );
}
