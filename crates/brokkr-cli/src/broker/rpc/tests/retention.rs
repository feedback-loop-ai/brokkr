//! Retained responses (decision 0065 slice two, U8b; CR1, CR2, CR3, CC2,
//! MB5): a retaining plan's prepared response is staged and synced before
//! the Terminal that cites it, and delivered as staged; each forwarded call
//! reserves room in the plan's share first, or is refused for the budget
//! before any external work, and keeps what it staged charged; a stage or
//! a Terminal that cannot be made delivers nothing. A harness that ends
//! while a call is active fails it, and the session, with the attempt's
//! cancellation cause (operator ruling 2026-10-18, item 6).

use std::cell::RefCell;
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_protocol::broker::ledger::{Denial, Failure, Invalid, Ledger, Outcome, Unrecorded};
use brokkr_protocol::broker::Plan;
use serde_json::{json, Value};

use super::super::TIMEOUT;
use super::Script;
use super::{admitted, answering, answering_with, call, edged, exchange, failed, initialization};
use super::{forwarded, owned, plan, refusal, result, slow, tools, within, Beside, Harness, Ran};
use crate::broker::session::Ended;

/// How a test ledger is given its retention.
pub(super) type Retain = Box<dyn FnOnce(&mut Ledger<'_>)>;

/// One way a harness speaks.
type Conversation = Box<dyn FnOnce(&mut Harness) + Send>;

std::thread_local! {
    /// What this thread's retaining ledgers staged: each call's number, its
    /// bytes, and when its stage returned. A session's ledger runs on its
    /// test's own thread.
    static STAGED: RefCell<Vec<(u64, Vec<u8>, Instant)>> = const { RefCell::new(Vec::new()) };
}

/// A ledger's retention, its share that of one of `retaining` retaining
/// plans, every stage taken into [`STAGED`] but that of call `failing`,
/// which fails with nothing of it staged.
pub(super) fn retain(retaining: usize, failing: Option<u64>) -> Retain {
    Box::new(move |ledger| {
        let stage = move |call, bytes: &[u8]| match Some(call) == failing {
            true => Err(std::io::Error::other("stage refused")),
            false => {
                let staged = (call, bytes.to_vec(), Instant::now());
                STAGED.with_borrow_mut(|all| all.push(staged));
                Ok(())
            }
        };
        ledger.retain_with(stage, retaining);
    })
}

/// The retention of `plan`'s ledger: one retaining plan's share where it
/// retains, as the engine provisions a lone retaining slot, else none.
pub(super) fn retained(plan: &Plan) -> Option<Retain> {
    plan.retained.then(|| retain(1, None))
}

/// What this thread's ledgers staged since it last looked, taken.
pub(super) fn staged() -> Vec<(u64, Vec<u8>, Instant)> {
    STAGED.take()
}

/// Each Terminal's response in the ledger `bytes`, as written, or null
/// where it cites none.
pub(super) fn responses(bytes: &[u8]) -> Vec<Value> {
    let records = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty());
    let terminals = records
        .map(|line| serde_json::from_slice::<Value>(line).unwrap())
        .filter(|record| record["record"] == "terminal");
    terminals.map(|record| record["response"].clone()).collect()
}

/// The response a Terminal cites for staged `bytes`.
fn cited(bytes: &str) -> Value {
    json!({"sha256": canonical::sha256_bytes(bytes.as_bytes()), "bytes": bytes.len()})
}

/// The test plan, retaining its responses or not.
fn retaining(retained: bool) -> Plan {
    let mut plan = plan();
    plan.retained = retained;
    plan
}

/// A session over `script` for the test plan, retaining as `retain` has
/// it, its ledger's `failing` append failing, whose harness has
/// `conversation`.
fn retaining_session(
    script: Script,
    (retain, failing): (Option<Retain>, Option<usize>),
    conversation: impl FnOnce(&mut Harness) + Send + 'static,
) -> Ran {
    let beside = Beside {
        plan: retaining(retain.is_some()),
        retain,
        ..Beside::plain()
    };
    let attempt = (within(Duration::from_secs(10)), None);
    edged(script, (failing, attempt), beside, conversation)
}

/// What a well-behaved server answers every call with: nested structured
/// content, unrelated numbers among it, and its word that it succeeded.
fn body() -> Value {
    json!({"content": [{"type": "text", "text": "page"}], "isError": false,
           "structuredContent": {"z": {"n": [3, 1, 2.5, -42]}, "a": true}})
}

/// [`body`] as it is prepared, staged and delivered: its canonical bytes,
/// keys in order, as the one result object.
const PREPARED: &str = concat!(
    r#"{"result":{"content":[{"text":"page","type":"text"}],"isError":false,"#,
    r#""structuredContent":{"a":true,"z":{"n":[3,1,2.5,-42]}}}}"#,
);

#[test]
fn a_retained_response_is_staged_before_its_terminal_cites_it_and_delivered_as_staged() {
    // Two calls answered alike under different ids: each response is the
    // one prepared object, staged after its Started and before its
    // Terminal, which cites its digest and length, both alike whatever the
    // id; and delivered, after both, as the very staged bytes beside the
    // harness's own id. A plan that retains nothing stages nothing and
    // cites no digest.
    for retained in [false, true] {
        let (heard, hearing) = std::sync::mpsc::channel();
        let conversation = move |harness: &mut Harness| {
            for id in [json!("1"), json!(2)] {
                harness.send(&super::request(id, "tools/call", json!({"name": "lookup"})));
                let line = harness.line().unwrap();
                heard.send((line, Instant::now())).unwrap();
            }
        };
        let retention = retained.then(|| retain(1, None));
        let script = answering_with(tools(), body());
        let ran = retaining_session(script, (retention, None), conversation);
        let lines: Vec<_> = hearing.iter().collect();
        let enveloped =
            |id: &str| format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},{}\n", &PREPARED[1..]);
        let said: Vec<_> = lines.iter().map(|(line, _)| line.clone()).collect();
        assert_eq!(
            said,
            [enveloped("\"1\""), enveloped("2")].map(String::into_bytes)
        );
        let succeeded = Some(Outcome::Succeeded);
        let calls = vec![("lookup", succeeded), ("lookup", succeeded)];
        assert_eq!((ran.ended, ran.proved), (Ok(()), admitted(calls)));
        let staged = staged();
        let (cites, stages) = match retained {
            true => (vec![cited(PREPARED); 2], vec![(1, PREPARED), (2, PREPARED)]),
            false => (vec![Value::Null; 2], vec![]),
        };
        let bytes: Vec<_> = staged
            .iter()
            .map(|(call, bytes, _)| (*call, bytes.clone()))
            .collect();
        let stages: Vec<_> = stages
            .iter()
            .map(|(call, bytes)| (*call, bytes.as_bytes().to_vec()))
            .collect();
        assert_eq!(
            (retained, responses(&ran.ledger), bytes),
            (retained, cites, stages)
        );
        // Opened, then each call's Started and Terminal: each stage between
        // them, and each delivery after its Terminal.
        for (at, (_, _, when)) in staged.iter().enumerate() {
            let (started, terminal) = (ran.durable[1 + 2 * at], ran.durable[2 + 2 * at]);
            assert!(
                (started <= *when) & (*when <= terminal),
                "stage {at} out of order"
            );
        }
        for (at, (_, heard)) in lines.iter().enumerate() {
            assert!(
                ran.durable[2 + 2 * at] <= *heard,
                "delivery {at} before its Terminal"
            );
        }
    }
}

#[test]
fn an_error_response_is_retained_as_its_failed_calls_complete_object() {
    // A JSON-RPC error, its data included, is the call's failed outcome,
    // staged and cited whole as the one error object.
    let error = json!({"code": -32000, "message": "no page", "data": {"k": [1]}});
    let erring = answering(move |frame| {
        vec![json!({"jsonrpc": "2.0", "id": frame["id"], "error": error.clone()})]
    });
    let asked = vec![call("1", "lookup")];
    let told = json!({"jsonrpc": "2.0", "id": "1",
                      "error": {"code": -32000, "message": "no page", "data": {"k": [1]}}});
    let ran = retaining_session(
        erring,
        (Some(retain(1, None)), None),
        exchange(asked, vec![told]),
    );
    let object = r#"{"error":{"code":-32000,"data":{"k":[1]},"message":"no page"}}"#;
    let failed_call = admitted(vec![("lookup", Some(Outcome::Failed))]);
    assert_eq!((ran.ended, ran.proved), (Ok(()), failed_call));
    assert_eq!(responses(&ran.ledger), [cited(object)]);
    let staged: Vec<_> = staged()
        .into_iter()
        .map(|(call, bytes, _)| (call, bytes))
        .collect();
    assert_eq!(staged, [(1, object.as_bytes().to_vec())]);
}

/// The harness's refusal of call `id` for the budget.
fn exhausted(id: &str) -> Value {
    refusal(json!(id), -32603, "capability evidence budget is exhausted")
}

/// What a well-behaved server answers a call of `lookup` with, delivered.
fn echoed(id: &str) -> Value {
    result(
        &json!(id),
        json!({"content": [{"type": "text", "text": "lookup"}]}),
    )
}

#[test]
fn a_share_without_room_for_one_reservation_refuses_before_any_external_work() {
    // Thirty-three retaining plans share the budget, each under 8 MiB: the
    // call is accepted, then refused for the budget with its Terminal and
    // forwarded nowhere, nothing staged, and the session goes on.
    let asked = vec![call("1", "lookup")];
    let ran = retaining_session(
        super::mcp(),
        (Some(retain(33, None)), None),
        exchange(asked, vec![exhausted("1")]),
    );
    let refused = Some(Outcome::Refused(Denial::Budget));
    assert_eq!(ran.proved, admitted(vec![("lookup", refused)]));
    assert_eq!(
        (ran.ended, ran.frames, staged().len()),
        (Ok(()), initialization(), 0)
    );
    assert_eq!(responses(&ran.ledger), [Value::Null]);
}

#[test]
fn what_a_call_stages_stays_charged_and_only_the_rest_of_its_reservation_returns() {
    // Thirty-two retaining plans: a share of exactly one reservation. The
    // first call is forwarded and staged; an identical second, its digest
    // the same, finds its bytes still charged and is refused before it is
    // forwarded. Sixteen: two reservations, of which each small answer
    // charges only its bytes, so a third call still finds room. Each call
    // is asked once the one before it is answered.
    let asking = |answers: Vec<Value>| {
        move |harness: &mut Harness| {
            for (at, answer) in answers.into_iter().enumerate() {
                harness.send(&call(&(at + 1).to_string(), "lookup"));
                assert_eq!(harness.next(), Some(answer));
            }
        }
    };
    let ran = retaining_session(
        super::mcp(),
        (Some(retain(32, None)), None),
        asking(vec![echoed("1"), exhausted("2")]),
    );
    let refused = Some(Outcome::Refused(Denial::Budget));
    let expected = admitted(vec![
        ("lookup", Some(Outcome::Succeeded)),
        ("lookup", refused),
    ]);
    assert_eq!(ran.proved, expected);
    assert_eq!(ran.frames[3..], [forwarded(3, "lookup", json!({}))]);
    assert_eq!(staged().len(), 1);
    let answers = vec![echoed("1"), echoed("2"), echoed("3")];
    let ran = retaining_session(
        super::mcp(),
        (Some(retain(16, None)), None),
        asking(answers),
    );
    let succeeded = vec![("lookup", Some(Outcome::Succeeded)); 3];
    assert_eq!((ran.proved, staged().len()), (admitted(succeeded), 3));
}

#[test]
fn a_call_refused_while_another_is_active_leaves_the_active_ones_reservation_held() {
    // A share of exactly one reservation, held by the active call: a call
    // made meanwhile is refused as active, never for the budget, and takes
    // nothing of what the active one holds, which is still staged.
    let asked = vec![call("1", "lookup"), call("2", "read")];
    let active = refusal(json!("2"), -32600, "MCP broker already has an active call");
    let ran = retaining_session(
        slow(Duration::from_millis(300)),
        (Some(retain(32, None)), None),
        exchange(asked, vec![active, echoed("1")]),
    );
    let refused = Some(Outcome::Refused(Denial::Active));
    let calls = vec![("lookup", Some(Outcome::Succeeded)), ("read", refused)];
    assert_eq!((ran.proved, staged().len()), (admitted(calls), 1));
}

#[test]
fn a_stage_or_terminal_that_cannot_be_made_delivers_nothing_and_leaves_the_call_begun() {
    // The stage fails: no Terminal is written, nothing of the response is
    // delivered, and the harness is told the call could not be recorded.
    // The Terminal's append fails after its stage: the staged bytes remain,
    // proving no result, and the same. Either way the call stays begun for
    // settlement, which reads it interrupted, and the session is unclosed.
    let unrecorded = refusal(json!("1"), -32603, "broker call could not be recorded");
    for (case, retain, failing, stages) in [
        ("stage", retain(1, Some(1)), None, 0),
        ("terminal", retain(1, None), Some(3), 1),
    ] {
        let told = unrecorded.clone();
        let conversation = move |harness: &mut Harness| {
            harness.send(&call("1", "lookup"));
            assert_eq!(harness.next(), Some(told));
            assert_eq!(harness.next(), None);
        };
        let ran = retaining_session(super::mcp(), (Some(retain), failing), conversation);
        let begun = (
            owned(vec![("lookup", None)]),
            None,
            Some(Invalid::Lifecycle),
        );
        let ended = Err(Ended::Unrecorded(Unrecorded::Record));
        assert_eq!((case, ran.ended, ran.proved), (case, ended, begun));
        assert_eq!((case, staged().len()), (case, stages));
        assert_eq!((case, responses(&ran.ledger)), (case, vec![]));
    }
}

#[test]
fn a_harness_that_ends_during_a_call_fails_it_and_the_session_with_its_cancellation() {
    // Operator ruling 2026-10-18, item 6: a harness that closes its input
    // while its call is active, politely, still reading, or because it was
    // killed, its output closed too, leaves no unfinished ledger: the call
    // fails with its Terminal, and the session closes Failed with MB5's
    // cancellation cause, the attempt's timeout.
    let timed_out = refusal(json!("1"), -32603, "MCP server call timed out");
    let polite = move |harness: &mut Harness| {
        harness.send(&call("1", "lookup"));
        drop(harness.to.take());
        assert_eq!(harness.next(), Some(timed_out));
        assert_eq!(harness.next(), None);
    };
    let killed = |harness: &mut Harness| harness.send(&call("1", "lookup"));
    let cases: [(&str, Conversation); 2] =
        [("polite", Box::new(polite)), ("killed", Box::new(killed))];
    for (case, conversation) in cases {
        let ran = retaining_session(slow(Duration::from_millis(300)), (None, None), conversation);
        let ended = Err(Ended::Failed(TIMEOUT));
        let finished = failed(vec![("lookup", Some(Outcome::Failed))], Failure::Timeout);
        assert_eq!((case, ran.ended, ran.proved), (case, ended, finished));
    }
}
