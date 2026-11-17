//! Decision 0109 slice 0109a1 through the real result path: a scripted
//! driver's result is admitted, stamped and journaled by a driven run,
//! or refused and failed, never repaired.

use serde_json::{json, Value};

use crate::bundle::{Aggregate, GateAssessment, Invocation, SeatBody, SiteKey};
use crate::engine::tests::{bundle, member, single_body};
use crate::engine::Engine;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::Store;

/// A driver that answers one attempt with `result`, which must hold
/// neither a `%` nor a `'`.
pub(super) fn driver(result: &Value) -> Vec<String> {
    let script = format!(
        r#"read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"test","version":"1","supports":[]}}'
read -r start
effect_id=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
attempt_id=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
printf '{{"proto":"forge-driver/v1","msg_id":"accepted","type":"accepted","effect_id":"%s","attempt_id":"%s","session_ref":null}}\n' "$effect_id" "$attempt_id"
printf '{{"proto":"forge-driver/v1","msg_id":"result","type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{result},"error":null}}\n' "$effect_id" "$attempt_id"
read -r done
"#
    );
    vec!["sh".into(), "-c".into(), script]
}

/// A finding as a seat writes it: no site.
pub(super) fn reported(id: &str, line: u64) -> Value {
    json!({"id": id, "file": "crates/a/src/lib.rs", "line": line,
           "summary": "the length is not checked", "severity": "medium", "security": false})
}

/// Drive a run whose `work` seat has `body` over a fresh journal in a
/// temporary directory, to wherever it stops.
pub(super) fn driven(body: SeatBody, gate: bool) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut compiled = bundle(dir.path(), body);
    if gate {
        compiled.seats.get_mut("work").unwrap().gate = GateAssessment::AllGate;
    }
    let mut engine =
        Engine::start(store, compiled, "findings", Some(dir.path().join("work"))).unwrap();
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    (dir, events)
}

/// The `work` effect's journaled successes and failures, in order.
pub(super) fn ended(events: &[EventEnvelope], kind: EventType) -> Vec<Value> {
    let work = events
        .iter()
        .find(|event| event.event_type == EventType::EffectRequested)
        .map(|event| event.payload["effect_id"].clone())
        .unwrap();
    events
        .iter()
        .filter(|event| event.event_type == kind && event.payload["effect_id"] == work)
        .map(|event| event.payload.clone())
        .collect()
}

#[test]
fn a_single_seats_findings_carry_its_own_site_through_the_real_result_path() {
    let result = json!({"result": "complete", "findings": [reported("unchecked-len", 12)]});
    let (_dir, events) = driven(single_body(driver(&result)), false);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    let site = SiteKey::single("work", None).digest();
    assert_eq!(
        succeeded[0]["result"]["findings"],
        json!([{"site": site, "id": "unchecked-len", "file": "crates/a/src/lib.rs", "line": 12,
                "summary": "the length is not checked", "severity": "medium",
                "security": false}])
    );
    let plain = json!({"result": "complete"});
    let (_dir, events) = driven(single_body(driver(&plain)), false);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    assert!(
        succeeded[0]["result"].get("findings").is_none(),
        "{succeeded:?}"
    );
}

/// Ruling 3's three refusals, each through the real result path: the
/// attempt fails with the refusal named, the bad result is never
/// journaled, and the cap parks the run as any determinate failure does.
#[test]
fn a_seat_supplied_site_a_duplicate_id_and_a_bad_id_each_fail_the_attempt() {
    let site = SiteKey::single("work", None).digest();
    let mut sited = reported("unchecked-len", 12);
    sited["site"] = json!(site);
    let cases = [
        (
            json!([sited]),
            "finding 0 carries a site: the engine stamps each finding's site (decision 0109 \
             ruling 3)",
        ),
        (
            json!([reported("twice", 1), reported("twice", 9)]),
            "finding 'twice' is reported twice by one site (decision 0109 ruling 3)",
        ),
        (
            json!([reported("Not.An.Id", 1)]),
            "finding 0 has no id matching ^[a-z0-9][a-z0-9._-]*$ of at most 64 bytes \
             (decision 0109 ruling 3)",
        ),
    ];
    for (findings, refusal) in cases {
        let result = json!({"result": "complete", "findings": findings});
        let (_dir, events) = driven(single_body(driver(&result)), false);
        assert!(
            ended(&events, EventType::EffectSucceeded).is_empty(),
            "{refusal}"
        );
        let failed = ended(&events, EventType::EffectFailed);
        assert_eq!(failed.len(), 2, "{refusal}");
        for failure in failed {
            let error = failure["error"].as_str().unwrap();
            let want = format!("the result was refused: {refusal}; stderr tail: ");
            assert!(error.starts_with(&want), "{error}");
        }
        assert!(events
            .iter()
            .any(|event| event.event_type == EventType::RunParked));
    }
}

/// A panel's aggregate joins each member's findings stamped with that
/// member's own site: the same id from two members is two keys.
#[test]
fn a_panels_member_findings_each_carry_their_members_site() {
    let a = json!({"result": "complete", "findings": [reported("same-id", 3)]});
    let b = json!({"result": "complete", "findings": [reported("same-id", 4)]});
    let body = SeatBody::Invocation(Invocation::Panel {
        members: vec![member("a", driver(&a)), member("b", driver(&b))],
        aggregate: Aggregate::ReviewPanel,
    });
    let (_dir, events) = driven(body, false);
    let succeeded = ended(&events, EventType::EffectSucceeded);
    let findings = succeeded[0]["result"]["findings"].as_array().unwrap();
    let sites: Vec<&Value> = findings.iter().map(|finding| &finding["site"]).collect();
    assert_eq!(
        sites,
        [
            &json!(SiteKey::panel_member("work", None, "a", 0).digest()),
            &json!(SiteKey::panel_member("work", None, "b", 1).digest()),
        ]
    );
    let lines: Vec<&Value> = findings.iter().map(|finding| &finding["line"]).collect();
    assert_eq!(lines, [&json!(3), &json!(4)]);
}
