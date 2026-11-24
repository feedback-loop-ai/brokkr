use std::collections::BTreeSet;

use super::super::claude_fold::Source::Claude;
use super::decision_0114a::{bindings, claude_launch, wire_rows, Driven, Stream};
use super::*;
use crate::RefusalClass;

const MODEL: &str = "echo-v2";
const EFFORT: &str = "medium";
const SESSION: &str = "local-1234";
const PIN: &str = "Fast-v2";
const TARGET: &str = "src/private-value.rs";
const PRIVATE: &str = "private-value";

fn init(model: &str, id: &str) -> Value {
    json!({"type":"system","subtype":"init","session_id":id,"model":model})
}

fn assistant(name: &str, usage: &Value) -> Value {
    json!({
        "type":"assistant", "effort":EFFORT, "message":{
            "id":"message-1", "model":MODEL, "usage":usage, "content":[
                {"type":"text","text":PRIVATE},
                {"type":"tool_use","id":"call-1","name":name,"input":{"file_path":TARGET}},
            ],
        },
    })
}

fn usage(counts: [u64; 5]) -> Value {
    let [input, output, read, write, thinking] = counts;
    json!({"input_tokens":input, "output_tokens":output, "cache_read_input_tokens":read,
        "cache_creation_input_tokens":write, "output_tokens_details":{"thinking_tokens":thinking}})
}

fn family(stream: &Stream, kind: AdapterKind) -> Driven {
    let extra = if kind == AdapterKind::Qoder {
        vec!["-m".into(), PIN.into()]
    } else {
        Vec::new()
    };
    stream.drive(kind, &extra)
}

pub(super) fn turns(driven: &Driven) -> impl Iterator<Item = &Value> {
    driven
        .emitted
        .iter()
        .filter_map(|(step, row)| (*step == Step::SeatTurn).then_some(row))
}

fn tuple(row: &Value) -> (&Value, &Value, &Value, &Value, &Value) {
    (
        &row["input_tokens"],
        &row["output_tokens"],
        &row["cache_read_tokens"],
        &row["cache_write_tokens"],
        &row["turn"],
    )
}

pub(super) fn assert_work(driven: &Driven) {
    assert!(matches!(driven.bodies.first(), Some(Body::Accepted { .. })));
    assert!(driven.bodies.iter().all(|body| !matches!(
        body,
        Body::Result {
            refusal: Some(_),
            ..
        }
    )));
}

fn assert_claude_projection(plain: &Driven) {
    assert_work(plain);
    let steps = plain
        .emitted
        .iter()
        .map(|(step, _)| *step)
        .collect::<Vec<_>>();
    assert_eq!(
        steps,
        [Step::Transcript, Step::HarnessStarted, Step::SeatTurn]
    );
    let (turn, spare) = {
        let mut turns = turns(plain);
        (turns.next().unwrap(), turns.next())
    };
    assert_eq!(spare, None);
    assert_eq!(
        tuple(turn),
        (&json!(7), &json!(8), &json!(4), &json!(5), &json!(1))
    );
    assert_eq!(turn["model"], MODEL);
    assert_eq!(turn["effort"], EFFORT);
    assert_eq!(turn["target"], TARGET);
    assert_eq!(turn["tool"], "Read");
    let meta = plain.meta.as_ref().unwrap();
    let row = &plain.emitted[0].1;
    assert_eq!(row["transcript"]["locator"], SESSION);
    assert_eq!(meta["transcript"], row["transcript"]);
    for (key, value) in [
        ("num_turns", json!(1)),
        ("model", json!(MODEL)),
        ("effort", json!(EFFORT)),
        ("input_tokens", json!(18)),
        ("cache_read_tokens", json!(11)),
        ("cache_write_tokens", json!(12)),
        ("output_tokens", json!(13)),
        ("reasoning_output_tokens", json!(4)),
        ("total_cost_usd", json!(0.25)),
    ] {
        assert_eq!(meta.get(key), Some(&value), "{key}");
    }
}

pub(in crate::adapters) fn words<'a>(words: &[&'a str]) -> BTreeSet<&'a str> {
    words.iter().flat_map(|word| [*word, &word[..4]]).collect()
}

pub(super) fn assert_bound_projection(
    kind: AdapterKind,
    structure: &[&str],
    plain: &Driven,
    mut drive: impl FnMut(&str) -> (Driven, Vec<secret::BoundSecret>),
    project: impl Fn(Step, &Value, &str) -> Value,
) {
    for word in words(structure) {
        let (driven, bound) = drive(word);
        assert_work(&driven);
        assert_eq!(driven.emitted.len(), plain.emitted.len(), "{kind:?} {word}");
        assert_eq!(driven.meta, plain.meta, "{kind:?} {word}");
        let guard = secret::Guard::new(&bound);
        for (((step, plain), (actual_step, projected)), actual) in plain
            .emitted
            .iter()
            .zip(&driven.emitted)
            .zip(wire_rows(&driven.bodies))
        {
            assert_eq!(actual_step, step, "{kind:?} {word}");
            assert_eq!(projected, &project(*step, plain, word), "{kind:?} {word}");
            assert_eq!(
                actual,
                &seat_row(kind, &bound, projected),
                "{kind:?} {word}"
            );
            // seat_row inserts these required keys after its mask.
            if !["model", "mode", "effort", "effo"].contains(&word) {
                assert!(guard.json(actual.to_string().as_bytes()), "{kind:?} {word}");
            }
        }
    }
}

fn assert_family_bound_projection(kind: AdapterKind, rows: &[Value], plain: &Driven) {
    // id and __ are below MIN_VALUE_BYTES. Refusal-only terms run on
    // their own shapes below.
    let structure = &[
        "type",
        "subtype",
        "system",
        "init",
        "session_id",
        "assistant",
        "result",
        "message",
        "usage",
        "content",
        "tool_use",
        "input",
        "file_path",
        "name",
        "num_turns",
        "output_tokens_details",
        "thinking_tokens",
        "total_cost_usd",
        "model",
        "data",
        "source",
        "effort",
        "turn_context",
        "input_tokens",
        "cache_read_input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
    ];
    assert_bound_projection(
        kind,
        structure,
        plain,
        |word| {
            let stream = Stream::rows(rows, &[("S", word)]);
            (family(&stream, kind), bindings(&stream))
        },
        |_, plain, _| plain.clone(),
    );
    let stream = Stream::rows(rows, &[("S", PRIVATE)]);
    let driven = family(&stream, kind);
    let turn = turns(&driven).next().unwrap();
    assert_eq!(turn["target"], "src/[secret:S].rs");
    let bound = bindings(&stream);
    let guard = secret::Guard::new(&bound);
    assert!(guard.json(turn.to_string().as_bytes()));
    assert!(guard.json(serde_json::to_string(&driven.meta).unwrap().as_bytes()));
}

fn assert_model_and_effort(kind: AdapterKind, ordinary: &Value) {
    let changed = json!({"type":"assistant","model":"tall-sensitive","effort":"sensitive-high",
        "message":{"model":"would-be-fallback","content":[]}});
    let end = json!({"type":"result","model":"sensitive-result","turn_context":{"effort":"sensitive-low"}});
    for previous in [false, true] {
        let mut rows = Vec::new();
        if previous {
            rows.push(ordinary.clone());
        }
        rows.extend([changed.clone(), end.clone()]);
        let stream = Stream::rows(&rows, &[("S", "sensitive")]);
        let driven = family(&stream, kind);
        let turn = turns(&driven).last().unwrap();
        assert_eq!(turn["model"], MODEL_NOT_REPORTED);
        assert_eq!(turn["effort"], EFFORT_NOT_REPORTED);
        let meta = driven.meta.as_ref().unwrap();
        assert_eq!(
            meta.get("model").and_then(Value::as_str),
            previous.then_some(MODEL)
        );
        assert_eq!(
            meta.get("effort").and_then(Value::as_str),
            previous.then_some(EFFORT)
        );
    }
}

fn assert_qoder_projection() {
    let zero = usage([0; 5]);
    let mut turn = assistant("Read", &zero);
    turn["model"] = json!("top-level-model");
    let init = init(PIN, SESSION);
    let result = json!({"type":"result","model":"result-model","session_id":SESSION,
        "num_turns":1,"usage":zero,"total_cost_usd":0});
    for secrets in [vec![], vec![("S", "assi")]] {
        let stream = Stream::rows(&[init.clone(), turn.clone(), result.clone()], &secrets);
        let driven = family(&stream, AdapterKind::Qoder);
        assert_work(&driven);
        let rows = wire_rows(&driven.bodies).collect::<Vec<_>>();
        assert_eq!(rows[0]["step"], "seat-turn");
        assert_eq!(rows[0]["turn"], 1);
        assert_eq!(rows[0]["model"], PIN);
        assert_eq!(
            rows[1]["transcript"],
            json!({"kind":"none","locator":"","home":""})
        );
        assert_eq!(rows[2]["model"], PIN);
        let meta = driven.meta.as_ref().unwrap();
        assert_eq!(meta["model"], PIN);
        assert_eq!(meta["transcript"]["locator"], "");
        assert_eq!(meta.get("session_id"), None);
        for key in [
            "input_tokens",
            "output_tokens",
            "cache_read_tokens",
            "cache_write_tokens",
            "reasoning_output_tokens",
            "total_cost_usd",
        ] {
            assert_eq!(meta.get(key), None, "{key}");
            assert_eq!(rows[0].get(key), None, "{key}");
        }
    }

    // Qoder's own D1 words are swept through its seat as the Claude
    // family's are (F2): its admission, pin, turns and projection read the
    // raw row, so a bound word changes nothing but the wire's mask.
    let rows = [init.clone(), turn.clone(), result];
    let plain = family(&Stream::rows(&rows, &[]), AdapterKind::Qoder);
    assert_family_bound_projection(AdapterKind::Qoder, &rows, &plain);

    let stream = Stream::rows(&[init.clone(), turn, init], &[("S", "assi")]);
    let lost = family(&stream, AdapterKind::Qoder);
    let [Body::Accepted { .. }, Body::Checkpoint { data, .. }, Body::Checkpoint {
        data: transcript, ..
    }] = &lost.bodies[..]
    else {
        panic!(
            "expected a counted turn and no result after the late stop: {:?}",
            lost.bodies
        );
    };
    assert_eq!(
        (&data["step"], &data["turn"]),
        (&json!("seat-turn"), &json!(1))
    );
    assert_eq!(transcript["step"], "transcript");
    assert_eq!(transcript["transcript"]["locator"], "");
    assert!(lost.meta.is_none());
}

fn assert_raw_refusals() {
    let cases = [
        (
            json!({"type":"rate_limit_event","apiErrorStatus":429,"rate_limit_info":{"resetsAt":70},
                "message":PRIVATE}),
            &[
                "type",
                "rate_limit_event",
                "apiErrorStatus",
                "rate_limit_info",
                "resetsAt",
                "message",
                PRIVATE,
            ][..],
            Some(60),
        ),
        (
            json!({"type":"assistant","isApiErrorMessage":true,"apiErrorStatus":429,
                "error":"rate_limit","message":{"content":[{"type":"text","text":PRIVATE}]}}),
            &[
                "type",
                "assistant",
                "isApiErrorMessage",
                "error",
                "rate_limit",
                "message",
                "content",
                "text",
                PRIVATE,
            ],
            None,
        ),
        (
            json!({"type":"result","is_error":true,"apiErrorStatus":429,"error":"rate_limit",
                "result":PRIVATE}),
            &["type", "result", "is_error", "error", "rate_limit", PRIVATE],
            None,
        ),
    ];
    for (row, vocabulary, retry_after) in cases {
        let words = "rate_limit (HTTP 429): private-value";
        let plain =
            super::super::refusal::claude_refusal_at(&Row::new(&row, &[]), Some(10)).unwrap();
        assert_eq!(
            (
                plain.class(),
                plain.status(),
                plain.retry_after_seconds(),
                plain.words()
            ),
            (RefusalClass::RateLimit, Some(429), retry_after, words)
        );
        for word in self::words(vocabulary) {
            let bound = [binding("S", word)];
            let refusal =
                super::super::refusal::claude_refusal_at(&Row::new(&row, &bound), Some(10))
                    .unwrap();
            let expected = secret::mask_text(words.as_bytes(), &bound);
            assert_eq!(
                (
                    refusal.class(),
                    refusal.status(),
                    refusal.retry_after_seconds()
                ),
                (RefusalClass::RateLimit, Some(429), retry_after),
                "{word}"
            );
            // The rate-limit event's classifier supplies its own token.
            let wire = super::super::masked_refusal(Some(refusal), &bound).unwrap();
            assert_eq!(wire.words(), expected, "{word}");
            assert!(
                secret::Guard::new(&bound).text(wire.words().as_bytes()),
                "{word}"
            );
        }
    }
}

#[test]
fn bound_word_claude_family() {
    let mut env = EnvGuard::lock();
    let home = tempfile::tempdir().unwrap();
    env.set("HOME", home.path());
    let turn = assistant("Read", &usage([3, 8, 4, 5, 0]));
    let result = json!({"type":"result","result":PRIVATE,"num_turns":1,
        "usage":usage([7, 13, 11, 12, 4]),"total_cost_usd":0.25});
    let rows = [init(MODEL, SESSION), turn.clone(), result];
    for kind in [AdapterKind::Claude, AdapterKind::Lanetally] {
        let plain = family(&Stream::rows(&rows, &[]), kind);
        assert_claude_projection(&plain);
        assert_family_bound_projection(kind, &rows, &plain);
        assert_model_and_effort(kind, &turn);
    }
    assert_qoder_projection();
    assert_raw_refusals();
}

#[test]
fn bound_name_across_separator() {
    let usage = usage([0; 5]);
    let name = "mcp__srv__tool";
    let masked = "mcp__sr[secret:S]ool";
    let shape = assistant(name, &usage);
    for (secrets, shown, tool) in [
        (
            vec![],
            name,
            json!({"kind":"mcp","server":"srv","tool":"tool"}),
        ),
        (
            vec![("S", "v__t")],
            masked,
            json!({"kind":"mcp_unidentified","reported":masked}),
        ),
    ] {
        let stream = Stream::rows(std::slice::from_ref(&shape), &secrets);
        let driven = family(&stream, AdapterKind::Claude);
        let expected = json!({"format":"claude","call":"call-1","tool":tool});
        let row = turns(&driven).next().unwrap();
        assert_eq!(row["observation"], expected);
        let wire = wire_rows(&driven.bodies)
            .find(|row| row["step"] == "seat-turn")
            .unwrap();
        assert_eq!(
            (&wire["tool"], &wire["observation"]),
            (&json!(shown), &expected)
        );
        assert!(secret::Guard::new(&bindings(&stream)).json(wire.to_string().as_bytes()));
    }
}

#[test]
fn bound_claude_init_and_result_keep_the_offered_identity() {
    let result = json!({"type":"result","session_id":SESSION});
    for (row, word) in [(init(MODEL, SESSION), "init"), (result, "result")] {
        let stream = Stream::rows(&[row], &[("WORD", word), ("ID", "1234")]);
        let (launch, rows) = claude_launch(&stream, Some(SESSION));
        assert_eq!(launch, LaunchTerminal::Resumed, "{word}");
        assert_eq!(rows[0]["transcript"]["locator"], "local-[secret:ID]");
        assert_eq!(rows[1]["launch"], "resumed");
        assert_eq!(rows[1].get("root_session"), None);
    }
}

#[test]
fn claude_does_not_record_or_resume_a_truncated_session_id() {
    let prefix = "a".repeat(80);
    let id = format!("{prefix}b");
    let stream = Stream::rows(&[init(MODEL, &id)], &[]);
    let (cold, rows) = claude_launch(&stream, None);
    assert_eq!(cold, LaunchTerminal::Cold);
    assert_eq!(rows[0]["transcript"]["locator"], prefix);
    assert_eq!(rows[1]["launch"], "cold");
    assert_eq!(rows[1].get("root_session"), None);
    let (rejoin, rows) = claude_launch(&stream, Some(&prefix));
    assert_eq!(rejoin, LaunchTerminal::Mismatch);
    assert_eq!(rows.len(), 1);
}

#[test]
fn init_journals_the_shared_transcript_checkpoint_with_the_id() {
    // The id used to be stashed for the session-finished checkpoint
    // only, which meant a WORKING seat had no session id in the
    // journal — so the transcript drilldowns could not locate, let
    // alone live-stream, the prose being written. init carries the id
    // in the first stream message; it is journaled immediately.
    let mut turns = ClaudeFold::default();
    let mut meta = serde_json::Map::new();
    let mut emitted: Vec<serde_json::Value> = Vec::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    fold_stream_event(
        &Row::new(&init(MODEL, "abcd-1234-ef"), &[]),
        Claude,
        &mut turns,
        &mut meta,
        &mut transcript,
        &mut |value| emitted.push(value.row.clone()),
    );
    assert_eq!(emitted.len(), 1, "one checkpoint, immediately");
    assert_eq!(emitted[0]["step"], "transcript");
    assert_eq!(emitted[0]["transcript"]["locator"], "abcd-1234-ef");
    assert_eq!(
        meta["transcript"]["locator"], "abcd-1234-ef",
        "and the meta still feeds the finish"
    );

    // A non-string id is refused wholesale, not stringified.
    let mut emitted: Vec<serde_json::Value> = Vec::new();
    fold_stream_event(
        &Row::new(
            &json!({"type": "system", "subtype": "init", "session_id": 7}),
            &[],
        ),
        Claude,
        &mut turns,
        &mut meta,
        &mut transcript,
        &mut |value| emitted.push(value.row.clone()),
    );
    assert!(emitted.is_empty(), "no string, no checkpoint");
}
