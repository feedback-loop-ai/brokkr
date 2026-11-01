//! The output edge (decision 0065 slice two, U6f1a; MB4, CR2, SD3): every
//! response masked by the shared masker and prepared once, read exactly,
//! and refused with MB4's cause where a bound secret cannot be masked away;
//! the server's standard error drained and masked across reads; and,
//! retained, the very masked bytes staged, or, refused, nothing (U8b).

use std::io::Write;
use std::sync::mpsc::channel;
use std::time::Duration;

use brokkr_core::canonical;
use brokkr_protocol::broker::ledger::{Denial, Failure, Invalid, Outcome};
use brokkr_protocol::secret::{self, BoundSecret, Guard};
use serde_json::{json, Value};

use super::super::output::{checked, correlated, framed, named, prepared, watched, Kind, Named};
use super::super::{Fatal, Fault, Id, Reply, ATOMIC, TIMEOUT};
use super::retention::{self, retained};
use super::PLAN;
use super::{
    admitted, answering_with, call, edged, failed, initializing, mcp, mcp_but, pipe, plan,
};
use super::{request, result, slow, startup, tools, within, Beside, Harness, Ran, Script, Then};
use crate::broker::session::stderr::{Drain, Said, SERVER};
use crate::broker::session::Ended;

/// A bound value holding a quote and a character of two bytes.
const TOKEN: &str = "tok\"\u{e9}-4e1d";

/// MB4's digits-only canary, exactly representable as a JSON number.
const CANARY: &str = "876543210";

const MASKING: Fatal = Fatal::Failed(Failure::Masking);
const PROTOCOL: Fatal = Fatal::Failed(Failure::Protocol);

/// `TOKEN` bound as `DOCS_TOKEN` and `CANARY` as `DOCS_PIN`, from a store
/// on a canonical temporary root.
fn bound() -> Vec<BoundSecret> {
    bound_as(&[("DOCS_TOKEN", TOKEN), ("DOCS_PIN", CANARY)])
}

/// Each `(name, value)` bound from a store on a canonical temporary root.
fn bound_as(named: &[(&str, &str)]) -> Vec<BoundSecret> {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().canonicalize().unwrap().join("secrets.env");
    for (name, value) in named {
        secret::store_set(&store, name, value).unwrap();
    }
    let names: Vec<String> = named.iter().map(|(name, _)| name.to_string()).collect();
    secret::resolve_bindings(&store, &names).unwrap()
}

/// `TOKEN`'s shared encoding `name`.
fn encoded(name: &str) -> String {
    let (_, encode) = secret::NEEDLE_ENCODINGS
        .iter()
        .find(|(encoding, _)| *encoding == name)
        .unwrap();
    String::from_utf8(encode(TOKEN.as_bytes())).unwrap()
}

/// Each bound value's needle `surfaces` hold: every shared encoding, and
/// its JSON string escaping, by the surface's index and the encoding's
/// name. A clean surface holds none.
fn leaks(surfaces: &[&[u8]]) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for value in [TOKEN, CANARY] {
        let escaped = serde_json::to_string(value).unwrap();
        let escaped = escaped.as_bytes()[1..escaped.len() - 1].to_vec();
        let needles = secret::NEEDLE_ENCODINGS
            .iter()
            .map(|(name, encode)| (name.to_string(), encode(value.as_bytes())))
            .chain([("json-string".to_string(), escaped)]);
        for (name, needle) in needles {
            let at = surfaces
                .iter()
                .position(|surface| surface.windows(needle.len()).any(|bytes| bytes == needle));
            found.extend(at.map(|at| (at, format!("{value:?} {name}"))));
        }
    }
    found
}

/// The test plan retaining its responses or not.
fn retaining(retained: bool) -> super::Plan {
    let plan = PLAN.replace('@', &"b".repeat(64));
    let plan = plan.replace("\"retained\": false", &format!("\"retained\": {retained}"));
    serde_json::from_str(&plan).unwrap()
}

/// When a harness ends its input, its frames sent and answered.
enum Ending {
    /// At once: a session with no call active ends served.
    Hanging,
    /// Only once the broker has ended its output, so a session the server
    /// ends after the last answer is never cut short by the harness first.
    Outlasting,
}

/// A session over `script`'s server beside `beside`'s edge, given ten
/// seconds, whose harness waits `pause`, sends `frames` one at a time,
/// taking the answer to each of the first `count` as its very bytes before
/// the next, and then ends its input: the session, and the answers.
fn heard(
    script: Script,
    beside: Beside,
    talk: (Duration, Vec<Value>, usize),
) -> (Ran, Vec<Vec<u8>>) {
    heard_as(Ending::Hanging, script, beside, talk)
}

/// [`heard`], its harness ending its input as `ending` has it.
fn heard_as(
    ending: Ending,
    script: Script,
    beside: Beside,
    (pause, frames, count): (Duration, Vec<Value>, usize),
) -> (Ran, Vec<Vec<u8>>) {
    let (said, hearing) = channel();
    let conversation = move |harness: &mut Harness| {
        std::thread::sleep(pause);
        let mut lines = Vec::new();
        for (at, frame) in frames.iter().enumerate() {
            harness.send(frame);
            lines.extend((at < count).then(|| harness.line().unwrap()));
        }
        match ending {
            Ending::Hanging => drop(harness.to.take()),
            // Held open; dropped with the harness once the broker has ended.
            Ending::Outlasting => {}
        }
        assert_eq!(harness.line(), None);
        said.send(lines).unwrap();
    };
    let attempt = (within(Duration::from_secs(10)), None);
    let ran = edged(script, (None, attempt), beside, conversation);
    (ran, hearing.recv().unwrap())
}

/// The edge of the test plan with `bound` secrets and nothing on standard
/// error.
fn masked_by(plan: super::Plan) -> Beside {
    Beside {
        retain: retained(&plan),
        plan,
        bindings: bound(),
        ..Beside::plain()
    }
}

/// A well-behaved server whose answer to each call is the line `said`
/// writes for the broker's request id, written in two parts, the first of
/// `at` bytes.
fn written(said: impl Fn(&Value) -> String + Send + 'static, at: usize) -> Script {
    mcp_but(move |frame, output| {
        (frame["method"] == "tools/call").then(|| {
            let line = said(&frame["id"]) + "\n";
            let (first, rest) = line.as_bytes().split_at(at.min(line.len()));
            for part in [first, rest] {
                output.as_mut().unwrap().write_all(part).unwrap();
                std::thread::sleep(Duration::from_millis(20));
            }
            Then::Read
        })
    })
}

/// The harness's request `id` failed with the session's `cause`.
fn told(id: Value, cause: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32603, "message": cause}})
}

/// A tools/call `reply` prepared at the edge alone, masked by `bindings`,
/// and its bytes passed by the final guard.
fn edge(reply: Reply, bindings: &[BoundSecret]) -> Result<Vec<u8>, Fatal> {
    edge_as(reply, Kind::Call, bindings)
}

/// A `reply` read as `kind` at the edge alone ([`edge`]).
fn edge_as(reply: Reply, kind: Kind, bindings: &[BoundSecret]) -> Result<Vec<u8>, Fatal> {
    let bytes = prepared(reply, kind, bindings)?;
    let passed = Guard::new(bindings).json(&bytes);
    passed.then_some(bytes).ok_or(MASKING)
}

/// A tools/call result `body` at the edge alone ([`edge`]).
fn alone(body: Value, bindings: &[BoundSecret]) -> Result<Vec<u8>, Fatal> {
    edge(Reply::Result(body), bindings)
}

/// The server's JSON-RPC error `error`, as written.
fn fault(error: &str) -> Reply {
    Reply::Error(serde_json::from_str::<Fault>(error).unwrap())
}

/// The session `ran`, whose harness heard `line`, failed its one call with
/// MB4's cause; `case` names it.
fn masking_failed<C: std::fmt::Debug + PartialEq>(case: C, ran: &Ran, line: &[u8]) {
    let told = told(json!("1"), "MCP response cannot be safely masked");
    assert_eq!((&case, read(line)), (&case, told));
    let failure = failed(vec![("lookup", Some(Outcome::Failed))], Failure::Masking);
    let ended = Err(Ended::Failed(MASKING));
    assert_eq!((&case, &ran.ended, &ran.proved), (&case, &ended, &failure));
    // Refused before anything is staged, retained or not: its Terminal
    // cites no digest (U8b).
    let (staged, cited) = (retention::staged().len(), retention::responses(&ran.ledger));
    assert_eq!((&case, staged, cited), (&case, 0, vec![Value::Null]));
}

/// The bytes `line` holds, read as JSON.
fn read(line: &[u8]) -> Value {
    serde_json::from_slice(line).unwrap()
}

/// What a call's masked answer holds, its canonical bytes without the
/// envelope, and their SHA-256.
const BODY: &str = concat!(
    r#"{"result":{"content":[{"text":"raw [secret:DOCS_TOKEN]","type":"text"},"#,
    r#"{"text":"b64 [secret:DOCS_TOKEN]","type":"text"},"#,
    r#"{"text":"hex [secret:DOCS_TOKEN] pin [secret:DOCS_PIN]","type":"text"},"#,
    r#"{"text":"pct [secret:DOCS_TOKEN]","type":"text"}],"#,
    r#""structuredContent":{"[secret:DOCS_TOKEN]":{"n":7},"list":[3,1,2.5]}}}"#,
);
const BODY_SHA256: &str = "780367eac084d1b2c004cd2998a100fddab0aa1e694f40d44f602cb0ce7b6d43";

/// What the server answers each call with: `TOKEN` raw, in three shared
/// encodings and as a key, and `CANARY` as text, beside unrelated numbers.
fn unmasked() -> Value {
    let text = |text: String| json!({"type": "text", "text": text});
    json!({
        "content": [
            text(format!("raw {TOKEN}")),
            text(format!("b64 {}", encoded("base64-std-padded"))),
            text(format!("hex {} pin {CANARY}", encoded("hex-lower"))),
            text(format!("pct {}", encoded("percent-upper"))),
        ],
        "structuredContent": {TOKEN: {"n": 7}, "list": [3, 1, 2.5]},
    })
}

#[test]
fn every_shared_encoding_is_masked_into_one_canonical_buffer_whatever_the_id() {
    // Two calls, answered alike, each line written in two parts split
    // inside TOKEN's two-byte character: each answer is that one masked
    // buffer, exactly, beside the harness's own id.
    let body = unmasked();
    let answer = move |id: &Value| result(id, body.clone()).to_string();
    let at = answer(&json!(3)).find('\u{e9}').unwrap() + 1;
    let frames = vec![
        call("1", "lookup"),
        request(json!(2), "tools/call", json!({"name": "read"})),
    ];
    let beside = masked_by(plan());
    let (ran, lines) = heard(written(answer, at), beside, (Duration::ZERO, frames, 2));
    let enveloped = |id: &str| format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},{}\n", &BODY[1..]);
    assert_eq!(
        lines,
        [enveloped("\"1\""), enveloped("2")].map(String::into_bytes)
    );
    let succeeded = vec![
        ("lookup", Some(Outcome::Succeeded)),
        ("read", Some(Outcome::Succeeded)),
    ];
    assert_eq!((ran.ended, ran.proved), (Ok(()), admitted(succeeded)));
    // The same bytes, prepared at the edge alone, and their digest.
    let prepared = alone(unmasked(), &bound()).ok().unwrap();
    assert_eq!(String::from_utf8(prepared).unwrap(), BODY);
    assert_eq!(canonical::sha256_bytes(BODY.as_bytes()), BODY_SHA256);
    let surfaces: Vec<&[u8]> = lines.iter().map(Vec::as_slice).collect();
    assert_eq!(leaks(&surfaces), []);
}

#[test]
fn a_digits_only_canary_as_a_bare_number_fails_its_forwarded_call_unmasked() {
    // CANARY as a bare number in structuredContent, retained or not: the
    // call fails with MB4's cause, its Terminal holds no digest, and
    // nothing of the response reaches the harness. Neither the refusal,
    // the ledger nor the masked standard error holds any bound value.
    for retained in [false, true] {
        let body = json!({"content": [{"type": "text", "text": "pin"}],
                          "structuredContent": {"pin": 876_543_210}});
        let mut beside = masked_by(retaining(retained));
        beside.stderr = format!("starting {CANARY}\nlast {TOKEN}").into_bytes();
        let pause = Duration::from_millis(300);
        let script = answering_with(tools(), body);
        let (ran, lines) = heard(script, beside, (pause, vec![call("1", "lookup")], 1));
        let cause = "MCP response cannot be safely masked";
        masking_failed(retained, &ran, &lines[0]);
        let tail = labelled("starting [secret:DOCS_PIN]\nlast [secret:DOCS_TOKEN]\n");
        assert_eq!(String::from_utf8(ran.diagnostics.clone()).unwrap(), tail);
        let surfaces = [
            &lines[0][..],
            &ran.ledger,
            &ran.diagnostics,
            cause.as_bytes(),
        ];
        assert_eq!((retained, leaks(&surfaces)), (retained, vec![]));
    }
}

#[test]
fn the_canary_as_text_is_masked_and_an_unrelated_number_is_kept_exactly() {
    // The controls: CANARY as text and as a key is masked, an encoded
    // string too, and numbers that hold no bound value are delivered as
    // the very values the server wrote.
    let big = u64::MAX;
    let structured = |pin: &str, b64: &str| json!({"pin": pin, pin: 1, "b64": b64, "n": 12.5, "m": -42, "big": big});
    for retained in [false, true] {
        let text = |text: String| json!([{"type": "text", "text": text}]);
        let body = json!({"content": text(format!("pin {CANARY}")),
                          "structuredContent": structured(CANARY, &encoded("base64-url-unpadded"))});
        let frames = vec![call("1", "lookup")];
        let beside = masked_by(retaining(retained));
        let script = answering_with(tools(), body);
        let (ran, lines) = heard(script, beside, (Duration::ZERO, frames, 1));
        let masked = json!({"content": text("pin [secret:DOCS_PIN]".into()),
                            "structuredContent": structured("[secret:DOCS_PIN]", "[secret:DOCS_TOKEN]")});
        assert_eq!(
            (retained, read(&lines[0])),
            (retained, result(&json!("1"), masked.clone()))
        );
        // Retained, its very masked bytes are staged and cited (U8b), and
        // delivered beside the harness's id.
        let object = canonical::to_bytes(&json!({"result": masked}));
        let enveloped = [&br#"{"jsonrpc":"2.0","id":"1","#[..], &object[1..], b"\n"].concat();
        let cite = json!({"sha256": canonical::sha256_bytes(&object), "bytes": object.len()});
        let (cites, stages) = match retained {
            true => (vec![cite], vec![object]),
            false => (vec![Value::Null], vec![]),
        };
        let staged: Vec<_> = retention::staged()
            .into_iter()
            .map(|staged| staged.1)
            .collect();
        let kept = (
            lines[0] == enveloped,
            retention::responses(&ran.ledger),
            staged,
        );
        assert_eq!((retained, kept), (retained, (true, cites, stages)));
        let succeeded = admitted(vec![("lookup", Some(Outcome::Succeeded))]);
        assert_eq!(
            (retained, ran.ended, ran.proved),
            (retained, Ok(()), succeeded)
        );
        assert_eq!(leaks(&[&lines[0]]), []);
    }
}

/// A response frame answering request `id` with `result`, as written.
fn frame(id: &Value, result: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{result}}}"#)
}

/// A call's session whose server answers it with the frame `said` writes
/// for the broker's id: the session, and the harness's one answer as its
/// very bytes.
fn served_as(said: impl Fn(&Value) -> String + Send + 'static) -> (Ran, Vec<u8>) {
    let frames = vec![call("1", "lookup")];
    let (ran, mut lines) = heard(
        written(said, 0),
        masked_by(plan()),
        (Duration::ZERO, frames, 1),
    );
    (ran, lines.remove(0))
}

/// A call's session whose server answers it with `result` as written: the
/// session, and the harness's one answer.
fn answered_as(result: &'static str) -> (Ran, Value) {
    let (ran, line) = served_as(move |id| frame(id, result));
    (ran, read(&line))
}

/// The session `ran`, whose harness heard `answer`, failed its one call
/// with the protocol cause.
fn protocol_failed((ran, answer): (Ran, Value)) {
    assert_eq!(answer, told(json!("1"), "MCP server protocol is invalid"));
    let failure = failed(vec![("lookup", Some(Outcome::Failed))], Failure::Protocol);
    assert_eq!(
        (ran.ended, ran.proved),
        (Err(Ended::Failed(PROTOCOL)), failure)
    );
}

#[test]
fn a_number_read_as_another_value_is_the_servers_protocol_fault() {
    // Precise decimals and integers past 64 bits change their value when
    // read; numbers written inside strings are text, and never read.
    let changed = [
        r#"{"n":1.00000000000000000001}"#,
        r#"{"n":12345678901234567890123}"#,
        r#"{"n":18446744073709551616}"#,
        r#"{"n":-9223372036854775809}"#,
        r#"{"n":[1,{"m":0.30000000000000000001}]}"#,
    ];
    for written in changed {
        let checked = checked(frame(&json!(1), written).as_bytes());
        assert_eq!((written, checked), (written, Err(PROTOCOL)));
    }
    let kept = [
        r#"{"n":18446744073709551615,"m":-9223372036854775808}"#,
        r#"{"n":0.1,"m":0.30000000000000004,"e":1e2,"f":1.5E-3,"g":-2.5e+3}"#,
        r#"{"z":0,"y":-0,"x":0.0,"w":-0.0e5}"#,
        r#"{"n":876543210,"t":true,"f":false,"o":null}"#,
        r#"{"t":"1.00000000000000000001 \"12345678901234567890123\" \\"}"#,
    ];
    for written in kept {
        let checked = checked(frame(&json!(1), written).as_bytes());
        assert_eq!((written, checked), (written, Ok(())));
    }
    // Served, the call fails and nothing of its response is delivered.
    let big = r#"{"content":[],"structuredContent":{"n":12345678901234567890123}}"#;
    protocol_failed(answered_as(big));
}

#[test]
fn a_key_written_twice_at_any_depth_is_the_servers_protocol_fault() {
    let twice = [
        r#"{"a":1,"a":1}"#,
        r#"{"a":{"b":[{"c":1,"c":2}]}}"#,
        r#"{"content":[],"structuredContent":{"k":"v","k":"v"}}"#,
    ];
    for written in twice {
        let checked = checked(frame(&json!(1), written).as_bytes());
        assert_eq!((written, checked), (written, Err(PROTOCOL)));
    }
    let once = r#"{"a":1,"b":{"a":1},"c":[{"a":1},{"a":1}]}"#;
    assert_eq!(checked(frame(&json!(1), once).as_bytes()), Ok(()));
    protocol_failed(answered_as(twice[2]));
}

#[test]
fn a_servers_startup_answers_are_read_exactly_too() {
    // Its initialize result, a key twice or a number read as another value
    // in it, ends the session before any listing.
    let version = r#"{"protocolVersion":"2025-06-18","#;
    let results = [
        r#""capabilities":{},"serverInfo":{"name":"d","name":"d","version":"1.4.2"}}"#,
        r#""capabilities":{"n":1.00000000000000000001},"serverInfo":{"version":"1.4.2"}}"#,
    ];
    for said in results.map(|rest| format!("{version}{rest}")) {
        let answered = initializing(format!("{}\n", frame(&json!(1), &said)));
        let (offered, _) = startup(answered, Duration::from_secs(5));
        assert_eq!((&said, offered), (&said, Err(PROTOCOL)));
    }
}

#[test]
fn a_key_masking_makes_collide_with_another_is_the_servers_protocol_fault() {
    // A key of structured content or of an error's data holding a bound
    // value masks to a key its object already has, at the top or deep in
    // an array; one that collides with nothing is masked and kept. A
    // block's keys are never masked, so one holding a bound value is
    // refused unmasked instead.
    let bindings = bound();
    let collides = [
        json!({"content": [], "structuredContent": {format!("k-{TOKEN}"): 1, "k-[secret:DOCS_TOKEN]": 2}}),
        json!({"content": [], "structuredContent": {"a": [{CANARY: 1, "[secret:DOCS_PIN]": 1}]}}),
    ];
    for body in collides {
        let refused = alone(body.clone(), &bindings).err();
        assert_eq!((&body, refused), (&body, Some(PROTOCOL)));
    }
    let data =
        format!(r#"{{"code":-1,"message":"no","data":{{"{CANARY}":1,"[secret:DOCS_PIN]":2}}}}"#);
    assert_eq!(edge(fault(&data), &bindings), Err(PROTOCOL));
    let block =
        json!({"content": [{"type": "text", "text": "t", CANARY: 1, "[secret:DOCS_PIN]": 1}]});
    assert_eq!(alone(block, &bindings), Err(MASKING));
    let apart =
        json!({"content": [], "structuredContent": {format!("k-{TOKEN}"): 1, "k-other": 2}});
    let masked = alone(apart, &bindings).ok().unwrap();
    let bytes = concat!(
        r#"{"result":{"content":[],"#,
        r#""structuredContent":{"k-[secret:DOCS_TOKEN]":1,"k-other":2}}}"#
    );
    assert_eq!(String::from_utf8(masked).unwrap(), bytes);
}

#[test]
fn masking_that_changes_what_a_reply_says_cannot_be_safely_masked() {
    // A bound value that is one of MCP's own keys: masking never touches
    // it, so the result would still say its tool failed, and the error
    // still be an error, but the key is refused unmasked. The control, a
    // value that is no key, keeps what each says.
    let failing = || Reply::Result(json!({"content": [], "isError": true}));
    let error = || fault(r#"{"code":-1,"message":"no"}"#);
    let keys = bound_as(&[("WORD", "isError"), ("TEXT", "message")]);
    assert_eq!(edge(failing(), &keys), Err(MASKING));
    assert_eq!(edge(error(), &keys), Err(MASKING));
    let words = bound_as(&[("WORD", "elsewhere")]);
    let delivered =
        |reply: Reply| super::super::calls::delivered(reply, &words).map(|(outcome, _)| outcome);
    assert_eq!(delivered(failing()), Ok(Outcome::Failed));
    assert_eq!(delivered(error()), Ok(Outcome::Failed));
}

/// An empty tools/call result, as written.
const EMPTY: &str = "{\"content\":[]}";

/// A case's name, its server, the harness's frames and how many of them
/// are answered.
type Case = (&'static str, Script, Vec<Value>, usize);

/// A server answering the first call by its own id, and each later one by
/// the id `instead`.
fn later_as(instead: Value) -> Script {
    written(
        move |id| frame(if *id == 3 { id } else { &instead }, EMPTY),
        0,
    )
}

#[test]
fn a_response_id_holding_a_bound_value_ends_the_session_masked_and_is_copied_nowhere() {
    // The first call is answered; the second's answer, and in the last
    // case a response after the first that answers nothing, carry an id
    // holding CANARY. The session ends with MB4's cause: a call already
    // ended keeps its outcome, the active one fails, and the id reaches
    // neither the harness, the ledger nor the diagnostics. The harness
    // outlasts the session: hanging up after its one answer in the last
    // case would end it served before the broker read the response (#843).
    let two = vec![call("1", "lookup"), call("2", "lookup")];
    let unasked = |id: &Value| [frame(id, EMPTY), frame(&json!(876_543_210), "{}")].join("\n");
    let cases: [Case; 3] = [
        ("number", later_as(json!(876_543_210)), two.clone(), 2),
        ("text", later_as(json!("r-876543210")), two, 2),
        ("unasked", written(unasked, 0), vec![call("1", "lookup")], 1),
    ];
    for (case, script, frames, count) in cases {
        let beside = masked_by(plan());
        let talk = (Duration::ZERO, frames, count);
        let (ran, lines) = heard_as(Ending::Outlasting, script, beside, talk);
        let answered = result(&json!("1"), json!({"content": []}));
        let mut expected = vec![answered];
        let mut calls = vec![("lookup", Some(Outcome::Succeeded))];
        if count == 2 {
            expected.push(told(json!("2"), "MCP response cannot be safely masked"));
            calls.push(("lookup", Some(Outcome::Failed)));
        }
        let heard: Vec<Value> = lines.iter().map(|line| read(line)).collect();
        assert_eq!((case, heard), (case, expected));
        let failure = failed(calls, Failure::Masking);
        let ended = Err(Ended::Failed(MASKING));
        assert_eq!((case, ran.ended, ran.proved), (case, ended, failure));
        let mut surfaces: Vec<&[u8]> = lines.iter().map(Vec::as_slice).collect();
        surfaces.extend([&ran.ledger[..], &ran.diagnostics]);
        assert_eq!((case, leaks(&surfaces)), (case, vec![]));
    }
}

#[test]
fn the_catalog_is_masked_before_the_harness_hears_it_or_ends_the_session_unheard() {
    // A granted tool described with TOKEN is listed masked; one whose
    // schema holds CANARY as a number ends the session before the harness
    // is read, with no call recorded.
    let described = |schema: Value| {
        let mut listed = tools();
        listed[0] = json!({"name": "lookup", "description": format!("uses {TOKEN}"), "inputSchema": schema});
        listed
    };
    let listing = request(json!(1), "tools/list", json!({}));
    let script = answering_with(described(json!({"type": "object"})), json!({}));
    let (ran, lines) = heard(
        script,
        masked_by(plan()),
        (Duration::ZERO, vec![listing], 1),
    );
    let masked = json!({"name": "lookup", "description": "uses [secret:DOCS_TOKEN]", "inputSchema": {"type": "object"}});
    let offered = json!({"tools": [masked, tools()[2]]});
    assert_eq!(read(&lines[0]), result(&json!(1), offered));
    assert_eq!((ran.ended, ran.proved), (Ok(()), admitted(vec![])));
    let script = answering_with(described(json!({"default": 876_543_210})), json!({}));
    unlisted(script, masked_by(plan()));
}

/// A session over `script`'s server beside `beside`'s edge ended with MB4's
/// cause before its harness was read, telling it nothing and recording no
/// call.
fn unlisted(script: Script, beside: Beside) {
    let (ran, lines) = heard(script, beside, (Duration::ZERO, vec![], 0));
    assert_eq!(lines, Vec::<Vec<u8>>::new());
    let ended = Err(Ended::Failed(MASKING));
    assert_eq!(
        (ran.ended, ran.proved),
        (ended, failed(vec![], Failure::Masking))
    );
}

/// `lines` as the drain keeps them: each labelled the server's (U6f2).
fn labelled(lines: &str) -> String {
    let labelled = lines
        .split_inclusive('\n')
        .map(|line| format!("{SERVER}{line}"));
    labelled.collect()
}

/// What a drain keeps of `chunks` written one after another to its pipe,
/// each read as far as the pipe holds it before the next is written; the
/// pipe ended after the last where `ends`.
fn drained(bindings: &[BoundSecret], chunks: &[&[u8]], ends: bool) -> String {
    let (reader, mut writer) = pipe();
    let mut drain = Drain::new(reader, bindings);
    // A chunk is written a page at a time, so none waits on a full pipe.
    for piece in chunks.iter().flat_map(|chunk| chunk.chunks(4096)) {
        writer.write_all(piece).unwrap();
        while drain.drained() {}
    }
    if ends {
        drop(writer);
        while drain.drained() {}
        return drain.tail().iter().map(Said::as_str).collect();
    }
    let tail = drain.tail().iter().map(Said::as_str).collect();
    drop(writer);
    tail
}

#[test]
fn standard_error_is_masked_across_reads_and_decoded_only_once_whole() {
    // TOKEN raw, TOKEN in hex and CANARY, each split across reads, the
    // first inside its two-byte character, a word split inside its own,
    // and a last line without its newline once the pipe has ended: each
    // masked, each character whole.
    let bindings = bound();
    let raw = format!("raw {TOKEN}\n").into_bytes();
    let at = raw.iter().position(|byte| *byte == 0xc3).unwrap() + 1;
    let word = "\u{e9}t\u{e9}";
    let hex = format!(
        "hex {} pin {CANARY}\n{word}\nlast {TOKEN}",
        encoded("hex-lower")
    );
    let hex = hex.into_bytes();
    let within = hex.iter().position(|byte| *byte == 0xc3).unwrap() + 1;
    let (hexed, pinned) = (9, 34);
    let chunks = [
        &raw[..at],
        &raw[at..],
        &hex[..hexed],
        &hex[hexed..pinned],
        &hex[pinned..within],
        &hex[within..],
    ];
    let tail = drained(&bindings, &chunks, true);
    let masked = "raw [secret:DOCS_TOKEN]\nhex [secret:DOCS_TOKEN] pin [secret:DOCS_PIN]\n";
    let last = "last [secret:DOCS_TOKEN]\n";
    assert_eq!(tail, labelled(&format!("{masked}{word}\n{last}")));
    assert_eq!(leaks(&[tail.as_bytes()]), []);
}

#[test]
fn a_last_line_is_kept_only_once_nothing_can_complete_it() {
    // A line still open when the session ends could be the start of a
    // bound value, so nothing of it is kept; once the pipe has ended it is
    // whole.
    let bindings = bound();
    let chunks: [&[u8]; 1] = [b"first\npin 8765"];
    let first = labelled("first\n");
    assert_eq!(drained(&bindings, &chunks, false), first);
    let whole = labelled("first\npin 8765\n");
    assert_eq!(drained(&bindings, &chunks, true), whole);
    assert_eq!(drained(&bindings, &[b"first\n"], true), first);
}

#[test]
fn standard_error_is_proved_as_the_text_it_decodes_to() {
    // The review's line: a lone 0xFF before x, raw, decodes to U+FFFD and
    // x, a bound value the raw bytes never hold.
    let odd = bound_as(&[("ODD", "\u{fffd}x")]);
    let masked = labelled("[secret:ODD]\n");
    assert_eq!(drained(&odd, &[b"\xffx\n"], true), masked);
    // A line holding a bound value only by its value is kept as its
    // marker, which passes the same masker and guard (U6f1b): one masking
    // leaves holding a bound value by its value, as a name masking writes
    // holds 2, keeps nothing of it.
    let marker = "[MCP server stderr line cannot be safely masked]\n";
    let lines: [&[u8]; 2] = [b"pin 8.7654321e8\n", b"next\n"];
    let kept = labelled(&format!("{marker}next\n"));
    assert_eq!(drained(&bound(), &lines, true), kept);
    let word = bound_as(&[("PIN", CANARY), ("WORD", "safely masked")]);
    let masked = labelled("[MCP server stderr line cannot be [secret:WORD]]\nnext\n");
    assert_eq!(drained(&word, &lines, true), masked);
    let named = [("PIN", CANARY), ("WORD2", "safely masked"), ("TWO", "0002")];
    assert_eq!(drained(&bound_as(&named), &lines, true), labelled("next\n"));
}

#[test]
fn a_line_past_its_bound_is_passed_over_and_the_tail_keeps_its_bound() {
    let long = [vec![b'x'; (64 << 10) + 1], b"\nnext\n".to_vec()].concat();
    let over = "[MCP server stderr line exceeds the broker limit]\n";
    let kept = labelled(&format!("{over}next\n"));
    assert_eq!(drained(&[], &[&long], true), kept);
    // Lines one write takes whole, labelled, past the tail's 64 KiB: the
    // last of them that fit are kept.
    let line = [vec![b'y'; ATOMIC - 1 - SERVER.len()], b"\n".to_vec()].concat();
    let lines = vec![&line[..]; (64 << 10) / ATOMIC + 6];
    let tail = drained(&[], &lines, true);
    let whole = labelled(&String::from_utf8(line).unwrap());
    assert_eq!(tail, whole.repeat((64 << 10) / ATOMIC));
}

#[test]
fn a_bound_value_never_holds_the_newline_the_drain_splits_at() {
    // The removal control of the guard the drain leaves out: a value is
    // set only as one line, and a store whose value runs past its line
    // binds nothing; so no bound value, nor any shared encoding of one,
    // holds a newline.
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().canonicalize().unwrap().join("secrets.env");
    let set = secret::store_set(&store, "SPLIT", "line-one\nline-two");
    let single = "secret value must be a single line (the store is line-oriented)";
    assert_eq!(set, Err(single.to_string()));
    std::fs::write(&store, "SPLIT=line-one\nline-two\n").unwrap();
    let mode = std::os::unix::fs::PermissionsExt::from_mode(0o600);
    std::fs::set_permissions(&store, mode).unwrap();
    let bound = secret::resolve_bindings(&store, &["SPLIT".to_string()]).map(|_| ());
    let refused = format!("secrets store {} line 2 is not NAME=value", store.display());
    assert_eq!(bound, Err(refused));
    let unbroken: Vec<u8> = (0..=u8::MAX).filter(|byte| *byte != b'\n').collect();
    for (name, encode) in secret::NEEDLE_ENCODINGS {
        assert_eq!((name, encode(&unbroken).contains(&b'\n')), (name, false));
    }
}

/// A tools/call result whose structured content holds the number
/// `written`, read from the very text a server writes.
fn numbered(written: &str) -> Value {
    let text = format!(r#"{{"content":[],"structuredContent":{{"n":{written}}}}}"#);
    serde_json::from_str(&text).unwrap()
}

#[test]
fn a_bound_number_is_refused_by_its_value_however_either_side_spells_it() {
    // Both reviews' counterexamples, each a bound value and the number the
    // server writes: an exponent written otherwise, a point and trailing
    // zeros, leading zeros on the bound side, where JSON writes none, and
    // every zero the one zero.
    let refused = [
        ("100000000000000000000", "100000000000000000000"),
        ("1200000000000000000000", "1200000000000000000000"),
        ("12345678901234568", "12345678901234568.0"),
        ("1e+20", "1.0e20"),
        ("1500", "1.50e3"),
        ("1e20", "1e020"),
        ("876543210", "87.654321e7"),
        ("876543210", "876543210.000000000"),
        ("000000000876543210", "876543210"),
        ("876543210", "8.7654321e08"),
        ("0876543210", "-876543210"),
        ("0000", "0"),
        ("-0000", "0"),
        ("0000", "-0.0"),
        ("5e-3", "0.005"),
    ];
    for (value, written) in refused {
        let refused = alone(numbered(written), &bound_as(&[("NUMBER", value)]));
        assert_eq!((value, written, refused), (value, written, Err(MASKING)));
    }
    // PIN=0000 bound beside NUMBER=0876543210: each read on its own, the
    // number is still refused, and an unrelated one kept.
    let both = bound_as(&[("PIN", "0000"), ("NUMBER", "0876543210")]);
    assert_eq!(alone(numbered("876543210"), &both), Err(MASKING));
    let seven = r#"{"result":{"content":[],"structuredContent":{"n":7}}}"#;
    let kept = alone(numbered("7"), &both).map(String::from_utf8);
    assert_eq!(kept, Ok(Ok(seven.to_string())));
    // The controls: the canary as text is redacted, and numbers that are
    // no bound value are kept exactly; a bound value of zeros alone
    // refuses zero and no other number.
    let pin = json!([{"type": "text", "text": format!("pin {CANARY}")}]);
    let numbers = json!([876_543_211, 87_654_321, 12.5, -42, 0, 0.005, u64::MAX]);
    let kept = alone(
        json!({"content": pin, "structuredContent": {"n": numbers}}),
        &bound(),
    );
    let bytes = concat!(
        r#"{"result":{"content":[{"text":"pin [secret:DOCS_PIN]","type":"text"}],"#,
        r#""structuredContent":{"n":[876543211,87654321,12.5,-42,0,0.005,18446744073709551615]}}}"#,
    );
    assert_eq!(kept.map(String::from_utf8), Ok(Ok(bytes.to_string())));
    let zeros = alone(numbered("7"), &bound_as(&[("PIN", "0000")]));
    assert_eq!(zeros.map(String::from_utf8), Ok(Ok(seven.to_string())));
    // The residual: a bound value written across numbers, which none of
    // them is, is found in the prepared bytes alone.
    let list = alone(numbered("[3,1,2.5]"), &bound_as(&[("LIST", "3,1,2.5")]));
    assert_eq!(list, Err(MASKING));
}

#[test]
fn a_bound_number_spelled_otherwise_fails_its_forwarded_call_unmasked() {
    // Served, retained or not: the call fails with MB4's cause, nothing of
    // the response reaches the harness, and neither its answer nor the
    // ledger holds the value, as bound, as written or as serialized.
    let cases = [
        ("100000000000000000000", "100000000000000000000", "1e20"),
        ("1200000000000000000000", "1200000000000000000000", "1.2e21"),
        (
            "12345678901234568",
            "12345678901234568.0",
            "1.2345678901234568e16",
        ),
    ];
    let retained = cases.map(|case| [(case, false), (case, true)]);
    for ((value, said, serialized), retained) in retained.into_iter().flatten() {
        let plan = retaining(retained);
        let beside = Beside {
            retain: retention::retained(&plan),
            plan,
            bindings: bound_as(&[("NUMBER", value)]),
            ..Beside::plain()
        };
        let body = format!(r#"{{"content":[],"structuredContent":{{"n":{said}}}}}"#);
        let script = written(move |id| frame(id, &body), 0);
        let frames = vec![call("1", "lookup")];
        let (ran, lines) = heard(script, beside, (Duration::ZERO, frames, 1));
        masking_failed((said, retained), &ran, &lines[0]);
        for needle in [value, said, serialized].map(str::as_bytes) {
            let held = [&lines[0], &ran.ledger]
                .map(|surface| surface.windows(needle.len()).any(|bytes| bytes == needle));
            assert_eq!((needle, held), (needle, [false, false]));
        }
    }
}

/// What a server answers request `id` with on `surface`, CANARY's value
/// spelled `8.7654321e8` there, which no masker finds.
fn spelled_on(surface: &str, id: &Value) -> String {
    let content = |text: &str| format!(r#"{{"content":[{{"type":"text","text":"{text}"}}]"#);
    match surface {
        "text" => frame(id, &(content("pin 8.7654321e8") + "}")),
        "structured" => frame(
            id,
            &(content("pin") + r#","structuredContent":{"pin":"8.7654321e8"}}"#),
        ),
        "key" => frame(
            id,
            &(content("pin") + r#","structuredContent":{"8.7654321e8":1}}"#),
        ),
        _ => format!(
            r#"{{"jsonrpc":"2.0","id":{id},"error":{{"code":-32000,"message":"pin 8.7654321e8"}}}}"#
        ),
    }
}

#[test]
fn a_bound_number_a_string_spells_fails_its_forwarded_call_on_every_surface() {
    // CANARY's value spelled 8.7654321e8 in a text block, a structured
    // string, a key and an error's message: each read by its value
    // (operator ruling 2026-10-13), so the call fails with MB4's cause,
    // its Terminal holds no digest, and neither the harness's answer nor
    // the ledger holds the spelling or any bound value, whether retention
    // is on, off or vetoed. A served plan carries only its effective
    // retention, and a veto is served as retention off (CR1).
    let surfaces = ["text", "structured", "key", "error"];
    let retention = [("on", true), ("off", false), ("veto", false)];
    for surface in surfaces {
        for (retained, effective) in retention {
            let script = written(move |id| spelled_on(surface, id), 0);
            let frames = vec![call("1", "lookup")];
            let beside = masked_by(retaining(effective));
            let (ran, lines) = heard(script, beside, (Duration::ZERO, frames, 1));
            let case = (surface, retained);
            masking_failed(case, &ran, &lines[0]);
            let held = [&lines[0], &ran.ledger]
                .map(|surface| surface.windows(11).any(|bytes| bytes == b"8.7654321e8"));
            assert_eq!((case, held), (case, [false, false]));
            assert_eq!((case, leaks(&[&lines[0], &ran.ledger])), (case, vec![]));
        }
    }
    // The controls: another number spelled so in each is no bound value,
    // and the call succeeds with the answer as the server wrote it.
    for surface in surfaces {
        let other = move |id: &Value| spelled_on(surface, id).replace("e8", "e7");
        let (ran, line) = served_as(other);
        let expected = other(&json!("1"));
        assert_eq!((surface, read(&line)), (surface, read(expected.as_bytes())));
        let outcome = match surface {
            "error" => Outcome::Failed,
            _ => Outcome::Succeeded,
        };
        let proved = admitted(vec![("lookup", Some(outcome))]);
        assert_eq!((surface, ran.ended, ran.proved), (surface, Ok(()), proved));
    }
}

/// A session over a well-behaved server beside the test plan's edge, its
/// secrets [`bound`], whose harness sends `frames` and then hears nothing
/// before the session ends.
fn unheard(frames: Vec<Value>) -> Ran {
    unheard_by(mcp(), None, masked_by(plan()), frames)
}

/// A session over `script`'s server beside `beside`'s edge, its ledger's
/// append `failing` failing, whose harness sends `frames` and then hears
/// nothing before the session ends.
fn unheard_by(script: Script, failing: Option<usize>, beside: Beside, frames: Vec<Value>) -> Ran {
    let conversation = move |harness: &mut Harness| {
        for frame in &frames {
            harness.send(frame);
        }
        assert_eq!(harness.line(), None);
    };
    let bounds = (within(Duration::from_secs(10)), None);
    edged(script, (failing, bounds), beside, conversation)
}

/// The test plan's edge with `named` secrets bound and nothing on standard
/// error.
fn beside(named: &[(&str, &str)]) -> Beside {
    Beside {
        plan: plan(),
        bindings: bound_as(named),
        stderr: Vec::new(),
        ..Beside::plain()
    }
}

#[test]
fn a_harness_id_is_guarded_as_it_arrives_whatever_it_writes() {
    // The review's ids: "\u0000" under a bound u0000, which its own
    // escaping writes; TOKEN in a text id, escaped where it is written; and
    // a number that is a bound value by what it is worth. Each ends the
    // session with MB4's cause before anything is answered or recorded.
    let ping = |id: Value| request(id, "ping", json!({}));
    let cases = [
        ("nul", ping(json!("\u{0}")), beside(&[("NUL", "u0000")])),
        ("escaped", ping(json!(TOKEN)), masked_by(plan())),
        (
            "valued",
            ping(json!(876_543_210)),
            beside(&[("PIN", "0876543210")]),
        ),
    ];
    let ended = Err(Ended::Failed(MASKING));
    for (case, asked, beside) in cases {
        let ran = unheard_by(mcp(), None, beside, vec![asked]);
        let failure = failed(vec![], Failure::Masking);
        assert_eq!((case, &ran.ended, ran.proved), (case, &ended, failure));
    }
    // A bound value the whole frame alone holds, across a safe id and a
    // safe body: the listing, or the broker's own answer, is refused
    // whole, and nothing is told.
    for asked in [request(json!(7), "tools/list", json!({})), ping(json!(7))] {
        let seam = beside(&[("SEAM", "7,\"result")]);
        let ran = unheard_by(mcp(), None, seam, vec![asked]);
        let failure = failed(vec![], Failure::Masking);
        assert_eq!((&ran.ended, ran.proved), (&ended, failure));
    }
    // Its unrecorded path: a call whose Started would fail to be recorded
    // still ends with MB4's cause, since its id is refused before it, and
    // nothing of it is recorded; the failing append is the closure's.
    let unsafe_call = request(json!(876_543_210), "tools/call", json!({"name": "lookup"}));
    let ran = unheard_by(mcp(), Some(2), masked_by(plan()), vec![unsafe_call]);
    assert_eq!((&ran.ended, ran.proved.0), (&ended, vec![]));
    // Its failed path: an active call is failed with MB4's cause, told to
    // the harness by its own safe id, and the unsafe id is told nothing.
    let unsafe_ping = ping(json!(876_543_210));
    let conversation = move |harness: &mut Harness| {
        harness.send(&call("1", "lookup"));
        harness.send(&unsafe_ping);
        let cause = "MCP response cannot be safely masked";
        assert_eq!(harness.next(), Some(told(json!("1"), cause)));
        assert_eq!(harness.next(), None);
    };
    let bounds = (None, (within(Duration::from_secs(10)), None));
    let script = slow(Duration::from_millis(300));
    let ran = edged(script, bounds, masked_by(plan()), conversation);
    let failure = failed(vec![("lookup", Some(Outcome::Failed))], Failure::Masking);
    assert_eq!((ran.ended, ran.proved), (ended, failure));
}

#[test]
fn a_harness_id_holding_a_bound_value_is_answered_on_no_path() {
    // CANARY in the harness's own id, a number or in text, on each path
    // that would answer it: a call to a granted tool, one to a tool not
    // granted, one invalid, a ping, an initialize at MB3's version and at
    // another, a listing and a method not admitted. The id is checked as
    // it arrives, so nothing is forwarded, recorded or answered: no frame
    // is made, no call recorded, and the session ends with MB4's cause.
    let (pin, text) = (|| json!(876_543_210), || json!("r-876543210"));
    let version = |version: &str| json!({"protocolVersion": version});
    let cases = [
        (
            "call",
            request(pin(), "tools/call", json!({"name": "lookup"})),
        ),
        (
            "ungranted",
            request(text(), "tools/call", json!({"name": "admin"})),
        ),
        ("invalid", request(pin(), "tools/call", json!({"name": ""}))),
        ("ping", request(text(), "ping", json!({}))),
        (
            "initialize",
            request(pin(), "initialize", version("2025-06-18")),
        ),
        (
            "version",
            request(text(), "initialize", version("2024-11-05")),
        ),
        ("listing", request(pin(), "tools/list", json!({}))),
        ("other", request(text(), "resources/list", json!({}))),
    ];
    for (case, asked) in cases {
        let ran = unheard(vec![asked]);
        let ended = Err(Ended::Failed(MASKING));
        let failure = failed(vec![], Failure::Masking);
        assert_eq!((case, ran.ended, ran.proved), (case, ended, failure));
        let surfaces = [&ran.ledger[..], &ran.diagnostics];
        assert_eq!((case, leaks(&surfaces)), (case, vec![]));
    }
    // The control: an id holding no bound value, though digits of one, is
    // answered with the very bytes of the broker's envelope.
    let ping = vec![request(json!(87_654_321), "ping", json!({}))];
    let (ran, lines) = heard(mcp(), masked_by(plan()), (Duration::ZERO, ping, 1));
    let pong = "{\"jsonrpc\":\"2.0\",\"id\":87654321,\"result\":{}}\n";
    assert_eq!(lines, [pong.as_bytes()]);
    assert_eq!((ran.ended, ran.proved), (Ok(()), admitted(vec![])));
}

#[test]
fn a_bound_value_in_a_fixed_position_is_refused_and_never_rewritten() {
    // A bound value that is a content block's type and its text key, an
    // annotation's key or value, or structuredContent: masking leaves each
    // as it is, and the guard refuses it.
    let annotations = json!({"audience": ["user"], "priority": 0.5});
    let annotated = json!({"type": "text", "text": "t", "annotations": annotations});
    let cases = [
        ("text", json!({"content": [{"type": "text", "text": "t"}]})),
        ("audience", json!({"content": [annotated.clone()]})),
        ("user", json!({"content": [annotated.clone()]})),
        (
            "structuredContent",
            json!({"content": [], "structuredContent": {}}),
        ),
    ];
    for (value, body) in cases {
        let refused = alone(body, &bound_as(&[("WORD", value)]));
        assert_eq!((value, refused), (value, Err(MASKING)));
    }
    // A schema's type, a tool's name: the catalog would change what it
    // offers, so the session ends before the harness is read.
    unlisted(mcp(), beside(&[("WORD", "object")]));
    unlisted(mcp(), beside(&[("TOOL", "lookup")]));
    // The controls: such a word as text, or as a key of structured
    // content, is redacted and the reply kept; and a block whose members
    // hold no bound value is kept whole.
    let said = json!({"type": "text", "text": "an audience"});
    let body = json!({"content": [said], "structuredContent": {"audience": 1}});
    let kept = alone(body, &bound_as(&[("WORD", "audience")]));
    let bytes = concat!(
        r#"{"result":{"content":[{"text":"an [secret:WORD]","type":"text"}],"#,
        r#""structuredContent":{"[secret:WORD]":1}}}"#,
    );
    assert_eq!(kept.map(String::from_utf8), Ok(Ok(bytes.to_string())));
    let whole = alone(json!({"content": [annotated]}), &bound());
    let bytes = concat!(
        r#"{"result":{"content":[{"annotations":{"audience":["user"],"priority":0.5},"#,
        r#""text":"t","type":"text"}]}}"#,
    );
    assert_eq!(whole.map(String::from_utf8), Ok(Ok(bytes.to_string())));
    // A listing's free text is redacted at every depth of its schemas, a
    // property named title included, and nothing else: names, types,
    // required, enum and the like come out as they went in.
    let listing = json!({"tools": [{
        "name": "lookup", "title": "a word", "description": "word",
        "annotations": {"title": "word", "readOnlyHint": true},
        "inputSchema": {"type": "object", "title": null, "required": ["title"],
            "properties": {"title": {"type": "string", "description": "the word"},
                           "n": {"enum": [1, "x"], "minimum": 0}}},
        "outputSchema": {"type": "object", "description": ["word"]},
        "_meta": {"description": "kept"}}]});
    let words = bound_as(&[("WORD", "word")]);
    let listed = prepared(Reply::Result(listing), Kind::Listing, &words);
    let bytes = concat!(
        r#"{"result":{"tools":[{"_meta":{"description":"kept"},"#,
        r#""annotations":{"readOnlyHint":true,"title":"[secret:WORD]"},"#,
        r#""description":"[secret:WORD]","inputSchema":{"properties":{"#,
        r#""n":{"enum":[1,"x"],"minimum":0},"#,
        r#""title":{"description":"the [secret:WORD]","type":"string"}},"#,
        r#""required":["title"],"title":null,"type":"object"},"name":"lookup","#,
        r#""outputSchema":{"description":["word"],"type":"object"},"#,
        r#""title":"a [secret:WORD]"}]}}"#,
    );
    assert_eq!(listed.map(String::from_utf8), Ok(Ok(bytes.to_string())));
    // A block's title and description are free text; its other members,
    // in another block's shape, are not, and a title not text is kept.
    let link = json!({"type": "resource_link", "uri": "u", "name": "n",
                      "title": "a word", "description": ["word"]});
    let kept = prepared(
        Reply::Result(json!({"content": [link]})),
        Kind::Call,
        &words,
    );
    let bytes = concat!(
        r#"{"result":{"content":[{"description":["word"],"name":"n","#,
        r#""title":"a [secret:WORD]","type":"resource_link","uri":"u"}]}}"#,
    );
    assert_eq!(kept.map(String::from_utf8), Ok(Ok(bytes.to_string())));
}

#[test]
fn a_schemas_instance_data_is_never_masked_and_refuses_a_bound_value() {
    // A title or description written inside a schema's enum, const,
    // default or examples is its fixed instance data: each comes out of
    // masking byte-identical, so the bound value in it is refused.
    let words = bound_as(&[("WORD", "word")]);
    let listed =
        |schema: Value| Reply::Result(json!({"tools": [{"name": "t", "inputSchema": schema}]}));
    let cases = [
        ("enum", json!([{"title": "word"}])),
        ("const", json!({"title": "word"})),
        ("default", json!({"description": "word"})),
        ("examples", json!([{"title": "word"}])),
    ];
    for (keyword, data) in cases {
        let schema = json!({"type": "object", keyword: data});
        let whole = canonical::to_bytes(
            &json!({"result": {"tools": [{"name": "t", "inputSchema": schema}]}}),
        );
        let kept = prepared(listed(schema.clone()), Kind::Listing, &words);
        assert_eq!((keyword, kept), (keyword, Ok(whole)));
        let refused = edge_as(listed(schema), Kind::Listing, &words);
        assert_eq!((keyword, refused), (keyword, Err(MASKING)));
    }
    // The control: a schema named for each keyword, in each place schemas
    // are named, is a schema, its description free text and redacted.
    let places = [
        "properties",
        "patternProperties",
        "dependentSchemas",
        "$defs",
        "definitions",
    ];
    let keywords = ["enum", "const", "default", "examples"];
    for (place, keyword) in places
        .into_iter()
        .flat_map(|place| keywords.map(|keyword| (place, keyword)))
    {
        let schema = json!({place: {keyword: {"description": "a word"}}});
        let kept = edge_as(listed(schema), Kind::Listing, &words).map(String::from_utf8);
        let masked = json!({place: {keyword: {"description": "a [secret:WORD]"}}});
        let bytes = canonical::to_bytes(
            &json!({"result": {"tools": [{"inputSchema": masked, "name": "t"}]}}),
        );
        assert_eq!(
            (place, keyword, kept),
            (place, keyword, Ok(String::from_utf8(bytes)))
        );
    }
}

#[test]
fn a_record_the_ledgers_guard_refuses_ends_the_session_with_the_masking_cause() {
    // STATE=succeeded: the call's Terminal is never appended, the harness
    // is told MB4's cause, and the session ends with it, the call begun.
    let frames = vec![call("1", "lookup")];
    let (ran, lines) = heard(
        mcp(),
        beside(&[("STATE", "succeeded")]),
        (Duration::ZERO, frames, 1),
    );
    assert_eq!(
        read(&lines[0]),
        told(json!("1"), "MCP response cannot be safely masked")
    );
    let ended = Err(Ended::Failed(MASKING));
    let calls = vec![("lookup".to_string(), None)];
    assert_eq!(
        (ran.ended, ran.proved),
        (ended, (calls, None, Some(Invalid::Lifecycle)))
    );
    let held = ran.ledger.windows(9).any(|bytes| bytes == b"succeeded");
    assert!(!held);
}

#[test]
fn a_tool_name_holding_a_bound_value_reaches_no_record_refusal_or_frame() {
    // An ungranted tool named CANARY: recorded under its masked name alone,
    // refused as ungranted, and the harness told MB4's cause before the
    // session ends with it. Neither the ledger nor the frame holds it.
    let frames = vec![call("1", CANARY)];
    let (ran, lines) = heard(mcp(), masked_by(plan()), (Duration::ZERO, frames, 1));
    let cause = "MCP response cannot be safely masked";
    assert_eq!(read(&lines[0]), told(json!("1"), cause));
    let refused = Some(Outcome::Refused(Denial::Ungranted));
    let failure = failed(vec![("[secret:DOCS_PIN]", refused)], Failure::Masking);
    let ended = Err(Ended::Failed(MASKING));
    assert_eq!((&ran.ended, ran.proved), (&ended, failure));
    assert_eq!(leaks(&[&lines[0], &ran.ledger]), []);
    // Named by its value, which masking its text leaves as it is: recorded
    // wholly masked, refused and told the same, and the alias itself
    // reaches no record or frame.
    let frames = vec![call("1", "8.7654321e8")];
    let (ran, lines) = heard(mcp(), masked_by(plan()), (Duration::ZERO, frames, 1));
    assert_eq!(read(&lines[0]), told(json!("1"), cause));
    let failure = failed(vec![("[secret]", refused)], Failure::Masking);
    assert_eq!((&ran.ended, ran.proved), (&ended, failure));
    for surface in [&lines[0], &ran.ledger] {
        let held = surface.windows(11).any(|bytes| bytes == b"8.7654321e8");
        assert!(!held);
    }
}

#[test]
fn the_final_guard_reads_the_very_bytes_and_every_form_they_decode_to() {
    // Each a JSON line or a text the guard is given, beside bound values
    // whose JSON escaping, decoded, says what the bytes do not.
    let bindings = bound_as(&[
        ("NUL", "u0000"),
        ("SLASH", "a/b/c/d"),
        ("FACE", "\u{1f600}-face"),
        ("ODD", "\u{fffd}x"),
        ("CTRL", "t\tb\u{8}f\u{c}x"),
        ("PIN", CANARY),
    ]);
    let guard = Guard::new(&bindings);
    let json = [
        (r#"{"a":"\u0000"}"#, false),
        (r#"{"a":"a\/b\/c\/d"}"#, false),
        (r#"{"a":"😀-face"}"#, false),
        (r#"{"a":"\ud800x"}"#, false),
        (r#"{"a":"t\tb\bf\fx"}"#, false),
        (r#"{"a":"876543210"}"#, false),
        (r#"{"n":8.7654321e8}"#, false),
        // A number a string spells, a key's or a value's, decoded first, is
        // read by its value as one written outside strings is (operator
        // ruling 2026-10-13); one that is no bound value passes.
        (r#"{"t":"8.7654321e8","w":"\r\n\"\\\/"}"#, false),
        (r#"{"8.7654321e8":1}"#, false),
        (concat!(r#"{"t":""#, "\\", r#"u0038.7654321e8"}"#), false),
        (r#"{"t":"pin=8.7654321e8;"}"#, false),
        (r#"{"t":"8.7654321e7 v2.0","w":"\r\n\"\\\/"}"#, true),
    ];
    for (bytes, passed) in json {
        assert_eq!((bytes, guard.json(bytes.as_bytes())), (bytes, passed));
    }
    // A text is read as the text it is, whether or not it parses as JSON:
    // a whole JSON string holding a bound number's alias refuses as that
    // alias in plain text does.
    let text = [
        (r"\u12 \uZZZZ \u00 \q \", true),
        (r#"pin "8.7654321e8""#, false),
        (r#""8.7654321e8""#, false),
        ("x--8.7654321e8", false),
        ("2026-8.7654321e8 ", false),
        (r"pin 8.7654321e8", false),
        // A number glued to a word, a point or another number's exponent
        // is read as the number it spells, whichever side it is glued on;
        // one a digit precedes is the longer number it continues.
        ("x8.7654321e8", false),
        ("token8.7654321e8", false),
        ("8.7654321e8th", false),
        (".8.7654321e8", false),
        ("1e8.7654321e8", false),
        ("1e5x 1.2.3 -.5e+ e8 18.7654321e8 v2.0", true),
    ];
    for (bytes, passed) in text {
        assert_eq!((bytes, guard.text(bytes.as_bytes())), (bytes, passed));
    }
}

#[test]
fn the_final_guard_ends_a_frames_scan_at_its_deadline_or_cancellation() {
    // A frame of many numbers, none bound: scanned within its deadline it
    // is made; past it, or with the attempt cancelled, the scan ends with
    // the timeout cause, the session's for each, and nothing is made.
    let bindings = bound();
    let guard = Guard::new(&bindings);
    let numbers: Vec<u32> = (0..300).collect();
    let body = canonical::to_bytes(&json!({"result": {"structuredContent": {"n": numbers}}}));
    let (past, later) = (std::time::Instant::now(), within(Duration::from_secs(10)).0);
    let (going, cancelled) = (|| false, || true);
    let id = Id::Number(7);
    let cases = [
        ((past, &going as &(dyn Fn() -> bool + Sync)), Err(TIMEOUT)),
        ((later, &cancelled), Err(TIMEOUT)),
    ];
    for (scan, expected) in cases {
        let made = framed(Some(&id), &body, &guard, watched(scan));
        assert_eq!(made, expected);
    }
    let made = framed(Some(&id), &body, &guard, watched((later, &going)));
    let line = [&b"{\"jsonrpc\":\"2.0\",\"id\":7,"[..], &body[1..], b"\n"].concat();
    assert_eq!(made.map(|made| made.bytes().to_vec()), Ok(line));
}

#[test]
fn an_errors_data_keeps_an_explicit_null_apart_from_its_absence() {
    // The server's error without data, and with data null: each prepared
    // and delivered as it was, the one never read as the other.
    let errors = [
        (
            r#"{"code":-1,"message":"no"}"#,
            r#"{"error":{"code":-1,"message":"no"}}"#,
        ),
        (
            r#"{"code":-1,"message":"no","data":null}"#,
            r#"{"error":{"code":-1,"data":null,"message":"no"}}"#,
        ),
    ];
    for (error, body) in errors {
        let prepared = edge(fault(error), &bound()).map(String::from_utf8);
        assert_eq!(prepared, Ok(Ok(body.to_string())));
        let said = move |id: &Value| format!(r#"{{"jsonrpc":"2.0","id":{id},"error":{error}}}"#);
        let (ran, line) = served_as(said);
        let delivered = format!("{{\"jsonrpc\":\"2.0\",\"id\":\"1\",{}\n", &body[1..]);
        assert_eq!((error, line), (error, delivered.into_bytes()));
        let calls = admitted(vec![("lookup", Some(Outcome::Failed))]);
        assert_eq!((error, ran.ended, ran.proved), (error, Ok(()), calls));
    }
}

#[test]
fn every_whole_line_a_pipe_still_holds_is_kept_when_the_session_ends() {
    // After one turn, a closed pipe's two lines, and a live pipe's two
    // whole lines without the one it has not finished.
    for (written, ends) in [(&b"first\nlast\n"[..], true), (b"first\nlast\nhal", false)] {
        let (reader, mut writer) = pipe();
        let mut drain = Drain::new(reader, &[]);
        writer.write_all(written).unwrap();
        let open = (!ends).then_some(writer);
        assert_eq!((ends, drain.drained()), (ends, true));
        let tail: String = drain.tail().iter().map(Said::as_str).collect();
        assert_eq!((ends, tail), (ends, labelled("first\nlast\n")));
        drop(open);
    }
    // Within its bound alone: a closed pipe read for four bytes keeps the
    // line that passes them, and nothing past it, whole or not.
    let (reader, mut writer) = pipe();
    let drain = Drain::new(reader, &[]);
    writer.write_all(b"a\nb\nc\nd\ne").unwrap();
    drop(writer);
    let kept = drain.tail_within(4);
    assert_eq!(
        kept.iter().map(Said::as_str).collect::<String>(),
        labelled("a\nb\nc\n")
    );
}

/// Each way the sweep writes a bound value, as JSON: TOKEN and CANARY in
/// every shared encoding and each escaped whole, as JSON strings, and
/// CANARY as numbers by its value.
fn spellings() -> Vec<String> {
    let mut written = Vec::new();
    for value in [TOKEN, CANARY] {
        for (_, encode) in secret::NEEDLE_ENCODINGS {
            let encoded = String::from_utf8(encode(value.as_bytes())).unwrap();
            written.push(serde_json::to_string(&encoded).unwrap());
        }
        let units = value.encode_utf16().map(|unit| format!("\\u{unit:04x}"));
        written.push(format!("\"{}\"", units.collect::<String>()));
    }
    let numbers = ["876543210", "8.7654321e8", "876543210.0", "87654321e1"];
    written.extend(numbers.map(String::from));
    written
}

/// A session over `script`'s server beside the test plan's edge, its
/// secrets [`bound`] and `stderr` on its standard error, whose harness
/// writes `frame`, hears one answer or none, ends its input and hears the
/// rest: the session, and every byte the harness heard.
fn swept(script: Script, stderr: Vec<u8>, frame: String) -> (Ran, Vec<u8>) {
    let (said, hearing) = channel();
    let conversation = move |harness: &mut Harness| {
        harness.raw(format!("{frame}\n").as_bytes());
        let mut heard = harness.line().unwrap_or_default();
        drop(harness.to.take());
        while let Some(line) = harness.line() {
            heard.extend(line);
        }
        said.send(heard).unwrap();
    };
    let mut beside = masked_by(plan());
    beside.stderr = stderr;
    let bounds = (None, (within(Duration::from_secs(10)), None));
    let ran = edged(script, bounds, beside, conversation);
    (ran, hearing.recv().unwrap())
}

#[test]
fn no_spelling_or_escape_of_a_bound_value_leaves_by_any_path() {
    // Each spelling, written by the server into a call's result where
    // masking never reaches and onto its standard error; written by the
    // harness as its own id; and, where it may name a tool, as the tool it
    // calls. Whatever the session tells the harness, records or writes to
    // its diagnostics holds no spelling of a bound value, as bound,
    // encoded, escaped or as written.
    for spelled in spellings() {
        let answer = format!(r#"{{"content":[],"_meta":{{"k":{spelled}}}}}"#);
        let script = written(move |id| frame(id, &answer), 0);
        let stderr = format!("said {spelled}\n").into_bytes();
        let mut runs = vec![swept(script, stderr, call("1", "lookup").to_string())];
        let ping = format!(r#"{{"jsonrpc":"2.0","id":{spelled},"method":"ping"}}"#);
        runs.push(swept(mcp(), Vec::new(), ping));
        let named = match serde_json::from_str::<Value>(&spelled).unwrap() {
            Value::String(text) => text,
            _ => spelled.clone(),
        };
        let tool = named
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() | b"._:/-".contains(&byte));
        if tool {
            runs.push(swept(mcp(), Vec::new(), call("1", &named).to_string()));
        }
        let inner = spelled.trim_matches('"').as_bytes();
        for (ran, heard) in &runs {
            let surfaces = [&heard[..], &ran.ledger, &ran.diagnostics];
            assert_eq!((&spelled, leaks(&surfaces)), (&spelled, vec![]));
            let held =
                surfaces.map(|surface| surface.windows(inner.len()).any(|bytes| bytes == inner));
            assert_eq!((&spelled, held), (&spelled, [false; 3]));
        }
    }
}

/// The self-labelled secret: a value that is its own placeholder, which
/// masking replaces with the very bytes it was.
const LABELLED: &str = "[secret:STATE]";

#[test]
fn a_secret_that_is_its_own_placeholder_is_found_where_it_occurs() {
    // STATE=[secret:STATE]: the legacy masker leaves it as it is, so it is
    // found where it occurs, never by what masking changed, in a fixed
    // field, the whole frame holding it, a string correlation id and a
    // line of standard error.
    let bindings = bound_as(&[("STATE", LABELLED)]);
    let guard = Guard::new(&bindings);
    let same = secret::mask_bytes(LABELLED.as_bytes(), &bindings);
    assert_eq!(same, LABELLED.as_bytes());
    let fixed = json!({"content": [], "_meta": {"fixed": LABELLED}});
    let whole = r#"{"jsonrpc":"2.0","id":"safe","result":{"content":[],"_meta":{"fixed":"[secret:STATE]"}}}"#;
    let id = Id::Text(LABELLED.to_string());
    let kept = drained(&bindings, &[b"said [secret:STATE]\n"], true);
    let each = (
        alone(fixed, &bindings),
        guard.json(whole.as_bytes()),
        correlated(&id, &guard),
        kept.as_str(),
    );
    let marker = labelled("[MCP server stderr line cannot be safely masked]\n");
    assert_eq!(each, (Err(MASKING), false, Err(MASKING), marker.as_str()));
    // The control: another value's placeholder is no value, and passes.
    let other = bound_as(&[("STATE", "succeeded")]);
    let guard = Guard::new(&other);
    assert!(guard.json(whole.as_bytes()));
    assert_eq!(correlated(&id, &guard), Ok(()));
    let kept = drained(&other, &[b"said [secret:STATE]\n"], true);
    assert_eq!(kept, labelled("said [secret:STATE]\n"));
}

#[test]
fn a_number_glued_to_a_word_is_read_in_ids_names_and_standard_error() {
    // CANARY spelled as no masker finds it, glued to a word: a harness id,
    // a tool name and a line of standard error, each refused as itself;
    // an unrelated number glued the same way passes.
    let bindings = bound();
    let guard = Guard::new(&bindings);
    for (glued, passed) in [("x8.7654321e8", false), ("x8.7654321e7", true)] {
        let id = Id::Text(glued.to_string());
        let safe = passed.then_some(()).ok_or(MASKING);
        assert_eq!((glued, correlated(&id, &guard)), (glued, safe));
        let name = match passed {
            true => Named::Safe,
            false => Named::Masked("[secret]".to_string()),
        };
        assert_eq!((glued, named(glued, &bindings, &guard)), (glued, name));
        let line = format!("said {glued}\n");
        let kept = match passed {
            true => labelled(&line),
            false => labelled("[MCP server stderr line cannot be safely masked]\n"),
        };
        let drained = drained(&bindings, &[line.as_bytes()], true);
        assert_eq!((glued, drained), (glued, kept));
    }
}

#[test]
fn a_ledger_record_the_guard_refuses_ends_the_session_masked_on_every_path() {
    // STATE=failed: the server's protocol fault fails the active call, and
    // its Terminal saying so is refused, as is the closure that follows:
    // the session ends with the masking cause, never the protocol's, the
    // harness told it, and the call stays begun.
    let unclosed = |calls: Vec<(&str, Option<Outcome>)>| {
        let calls = calls
            .into_iter()
            .map(|(tool, outcome)| (tool.to_string(), outcome));
        (calls.collect::<Vec<_>>(), None, Some(Invalid::Lifecycle))
    };
    let cause = "MCP response cannot be safely masked";
    let script = written(|id| frame(id, r#"{"content":"x"}"#), 0);
    let frames = vec![call("1", "lookup")];
    let state = beside(&[("STATE", "failed")]);
    let (ran, lines) = heard(script, state, (Duration::ZERO, frames.clone(), 1));
    assert_eq!(read(&lines[0]), told(json!("1"), cause));
    let ended = Err(Ended::Failed(MASKING));
    assert_eq!(
        (&ran.ended, ran.proved),
        (&ended, unclosed(vec![("lookup", None)]))
    );
    // STATE=closed: the call is answered and recorded, and the closure,
    // guarded by the session's bindings, which the ledger was given only
    // once the session began, is refused, so the session ends masked.
    let (ran, lines) = heard(
        mcp(),
        beside(&[("STATE", "closed")]),
        (Duration::ZERO, frames, 1),
    );
    let text = json!({"content": [{"text": "lookup", "type": "text"}]});
    assert_eq!(read(&lines[0])["result"], text);
    let calls = vec![("lookup", Some(Outcome::Succeeded))];
    assert_eq!((&ran.ended, ran.proved), (&ended, unclosed(calls)));
    let held = ran.ledger.windows(6).any(|bytes| bytes == b"closed");
    assert!(!held);
}

#[test]
fn the_opened_record_is_guarded_once_the_sessions_bindings_are_known() {
    // The ledger opens before any binding is known, holding the plan's
    // digest; a binding inside that digest is found in the Opened's very
    // line once the session hands the ledger its bindings, and the session
    // ends masked before the harness is read, the closure carrying it.
    let digest = &super::DIGEST[..8];
    let (ran, _) = heard(
        mcp(),
        beside(&[("STATE", digest)]),
        (Duration::ZERO, vec![], 0),
    );
    let ended = Err(Ended::Failed(MASKING));
    assert_eq!(
        (ran.ended, ran.proved),
        (ended, failed(vec![], Failure::Masking))
    );
}
