//! Decision 0075 ruling 2's binding: a fixture harness no code names,
//! `trellis`, declares its transcript and root session kinds in its
//! adapter's `record_kinds`, seats a turn through the engine on a fake
//! driver, and its seat records are appended under seat-record v8, verify,
//! export, verify offline and render. A kind it does not declare is
//! refused at append, naming the kind and the seat.

use std::path::Path;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::{SeatRecordVersion, Store};
use serde_json::{json, Value};

use super::tests::git_commit;
use super::{Engine, ENGINE_VERSION};
use crate::bundle::Bundle;

const TRAIL: &str = "trellis-trail";
const THREAD: &str = "trellis-thread";

/// A fake trellis driver: one checkpoint naming `transcript` and `root`
/// as its kinds, then a successful result naming `result` as its
/// transcript's.
fn driver(transcript: &str, root: &str, result: &str) -> String {
    let checkpoint = json!({"step": "transcript", "model": "trellis-1",
        "transcript": {"kind": transcript, "locator": "t-1", "home": "/h/.trellis"},
        "root_session": {"kind": root, "id": "tr-1", "harness_version": "1.0.0",
                         "persistent": true}});
    let outcome = json!({"result": "complete", "model": "trellis-1",
        "transcript": {"kind": result, "locator": "t-1", "home": "/h/.trellis"}});
    format!(
        r#"
read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"trellis","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"live","type":"checkpoint","effect_id":"%s","attempt_id":"%s","data":{checkpoint}}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{outcome},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#
    )
}

fn write(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

/// A bundle whose `work` seat hires the trellis agent, whose adapter
/// declares [`TRAIL`] and [`THREAD`] and invokes `script`.
fn trellis_bundle(dir: &Path, script: &str) -> Bundle {
    let (bundle, agents, adapters) = (dir.join("bundle"), dir.join("agents"), dir.join("adapters"));
    for made in [bundle.clone(), agents.join("charters"), adapters.clone()] {
        std::fs::create_dir_all(made).unwrap();
    }
    std::fs::write(agents.join("charters/trellis.md"), "# trellis\n").unwrap();
    write(
        &adapters.join("trellis.json"),
        &json!({"provider": "trellis", "trust_tier": "trusted", "egress": "contracted",
                "binary": "sh", "driver": ["sh", "-c", script],
                "models": {"trellis": "trellis-1"}, "model_flag": "--model",
                "efforts": ["high"], "effort_flag": "--effort",
                "tool_permissions": "unsupported", "mcp": "unsupported",
                "record_kinds": {"transcript": TRAIL, "root_session": THREAD}}),
    );
    write(
        &agents.join("trellis.json"),
        &json!({"description": "fixture trellis", "charter": "charters/trellis.md",
                "models": ["trellis"], "efforts": {"trellis": "high"}}),
    );
    // The work seat is the trellis seat; the protected review after it
    // names a driver that never spawns, so the run stops there.
    let rule = |from: &str, result: &str, next: &str| {
        json!({"id": from.to_uppercase(), "from": from, "result": result, "next": next,
               "reason": format!("{from} said {result}")})
    };
    let rules = [
        rule("work", "complete", "review"),
        rule("review", "clean", "done"),
    ];
    write(
        &bundle.join("policy.json"),
        &json!({"schema": "forge.phase-machine/v1", "phases": ["work", "review", "done"],
                "initial": "work", "terminal": ["done"], "shippable_from": ["review"],
                "rules": rules}),
    );
    let seat = json!({"results": ["complete"], "class": "work", "agent": "trellis"});
    let review = json!({"results": ["clean"], "class": "work", "role": "trellis.md",
                        "driver": {"command": ["no-such-review-driver"]}});
    std::fs::write(bundle.join("trellis.md"), "# review\n").unwrap();
    write(
        &bundle.join("bundle.json"),
        &json!({"name": "trellis", "description": "a harness no code names", "cost": "test",
                "policy": "policy.json", "seats": {"work": seat, "review": review}}),
    );
    Bundle::compile_with(&bundle, &agents, &adapters).unwrap()
}

/// The run the trellis seat drove on `script`, its journal and its store.
fn seated(dir: &Path, script: &str) -> (Store, String, Vec<EventEnvelope>) {
    let repo = dir.join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let store = Store::open(&dir.join("forge.db")).unwrap();
    let bundle = trellis_bundle(dir, script);
    let mut engine = Engine::start(store, bundle, "trellis", Some(repo)).unwrap();
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    (engine.store, engine.run_id, events)
}

fn of(events: &[EventEnvelope], kind: EventType) -> Vec<&Value> {
    let found = events.iter().filter(|event| event.event_type == kind);
    found.map(|event| &event.payload).collect()
}

/// The work seat's first attempt's terminal payload.
fn first_ending(events: &[EventEnvelope]) -> (EventType, &Value) {
    let ending = events.iter().find(|event| {
        matches!(
            event.event_type,
            EventType::EffectSucceeded | EventType::EffectFailed
        )
    });
    let ending = ending.expect("the work seat's attempt ended");
    (ending.event_type, &ending.payload)
}

#[test]
fn a_harness_no_code_names_seats_records_verifies_exports_and_renders() {
    assert_eq!(
        SeatRecordVersion::of_engine(ENGINE_VERSION),
        SeatRecordVersion::V8
    );
    let dir = tempfile::tempdir().unwrap();
    let (store, run_id, events) = seated(dir.path(), &driver(TRAIL, THREAD, TRAIL));
    let checkpoint = &of(&events, EventType::EffectCheckpointed)[0]["checkpoint"];
    assert_eq!(
        (
            &checkpoint["transcript"]["kind"],
            &checkpoint["root_session"]["kind"]
        ),
        (&json!(TRAIL), &json!(THREAD))
    );
    let (ended, payload) = first_ending(&events);
    assert_eq!(ended, EventType::EffectSucceeded, "{payload}");
    assert_eq!(payload["result"]["transcript"]["kind"], TRAIL);
    let exported = store.export_ndjson(&run_id).unwrap();
    let verified = brokkr_store::verify_export(&exported).unwrap();
    assert_eq!(verified.seq, events.last().unwrap().seq);
    let view = brokkr_view::run_view(&events, None);
    let cells: Vec<&str> = (view.participants.iter())
        .map(|seat| seat.transcript_cell.text.as_str())
        .collect();
    assert!(
        cells.contains(&"trellis-trail · t-1 · home /h/.trellis"),
        "{cells:?}"
    );
}

/// A kind the trellis adapter does not declare, in the checkpoint's
/// transcript, its root session or the result's transcript, is refused
/// at append by the writer's typed refusal: no record carrying it is
/// journaled, and the attempt fails on the refusal's own words.
#[test]
fn a_kind_the_fixture_does_not_declare_is_refused_at_append() {
    let stray = "trellis-stray";
    let refusal = |member: &str| {
        format!(
            "seat 'work' recorded {member} kind '{stray}', which no loaded adapter declares \
             (decision 0075 ruling 2); stderr tail: "
        )
    };
    for (script, member) in [
        (driver(stray, THREAD, TRAIL), "transcript"),
        (driver(TRAIL, stray, TRAIL), "root session"),
        (driver(TRAIL, THREAD, stray), "transcript"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_store, _run, events) = seated(dir.path(), &script);
        let (ended, payload) = first_ending(&events);
        assert_eq!(
            (ended, payload["error"].as_str()),
            (EventType::EffectFailed, Some(refusal(member).as_str())),
            "{payload}"
        );
        let journaled = serde_json::to_string(&of(&events, EventType::EffectCheckpointed)).unwrap();
        assert!(!journaled.contains(stray), "{journaled}");
    }
}
