//! Decision 0109 slice 0109a1 through the real result path: a scripted
//! driver's result is admitted, stamped and journaled by a driven run,
//! or refused and failed, never repaired.

use serde_json::{json, Value};

use crate::bundle::{Aggregate, GateAssessment, Invocation, SeatBody, SiteKey};
use crate::engine::heads::HeadReads;
use crate::engine::tests::{accepted, bundle, capabilities, git_commit, member, single_body, wire};
use crate::engine::Engine;
use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_protocol::{Body, ResultStatus};
use brokkr_store::Store;

/// The effect and attempt a scripted message names until the driver
/// learns the real ones from its start message.
const EFFECT: &str = "effect-unknown-yet";
const ATTEMPT: &str = "attempt-unknown-yet";

/// A driver that answers one attempt of any effect with `result`, which
/// must hold no `'`: its messages are the protocol's own, written with
/// placeholder ids the script replaces with the pair its start names.
pub(super) fn driver(result: &Value) -> Vec<String> {
    let answered = wire(Body::Result {
        refusal: None,
        effect_id: EFFECT.into(),
        attempt_id: ATTEMPT.into(),
        status: ResultStatus::Succeeded,
        result: Some(result.clone()),
        error: None,
    });
    let (hello, started) = (capabilities(), accepted(EFFECT, ATTEMPT, None));
    let script = format!(
        r#"read -r hello
printf '%s\n' '{hello}'
read -r start
eid=${{start#*\"effect_id\":\"}}; eid=${{eid%%\"*}}
aid=${{start#*\"attempt_id\":\"}}; aid=${{aid%%\"*}}
printf '%s\n' '{started}' '{answered}' | sed "s/{EFFECT}/$eid/; s/{ATTEMPT}/$aid/"
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
    driven_with(body, |compiled| {
        if gate {
            compiled.seats.get_mut("work").unwrap().gate = GateAssessment::AllGate;
        }
    })
}

/// [`driven`], with the bundle shaped by `shape` first, in a worktree
/// that is a git repository with one commit.
pub(super) fn driven_with(
    body: SeatBody,
    shape: impl FnOnce(&mut crate::bundle::Bundle),
) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    driven_over(body, shape, |_| HeadReads::default())
}

/// [`driven_with`], with the engine's HEAD reads answered by what
/// `heads` returns for the worktree, after the base commit.
pub(super) fn driven_over(
    body: SeatBody,
    shape: impl FnOnce(&mut crate::bundle::Bundle),
    heads: impl FnOnce(&std::path::Path) -> HeadReads,
) -> (tempfile::TempDir, Vec<EventEnvelope>) {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir(&work).unwrap();
    git_commit(&work, "base");
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut compiled = bundle(dir.path(), body);
    shape(&mut compiled);
    let heads = heads(&work);
    let mut engine = Engine::start(store, compiled, "findings", Some(work)).unwrap();
    engine.heads = heads;
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    (dir, events)
}

/// The `work` effect's journaled successes and failures, in order.
pub(super) fn ended(events: &[EventEnvelope], kind: EventType) -> Vec<Value> {
    ended_in(events, "work", kind)
}

/// The first `phase` effect's journaled events of `kind`, in order.
pub(super) fn ended_in(events: &[EventEnvelope], phase: &str, kind: EventType) -> Vec<Value> {
    let effect = events
        .iter()
        .find(|event| {
            event.event_type == EventType::EffectRequested && event.payload["phase"] == phase
        })
        .map(|event| event.payload["effect_id"].clone())
        .unwrap();
    events
        .iter()
        .filter(|event| event.event_type == kind && event.payload["effect_id"] == effect)
        .map(|event| event.payload.clone())
        .collect()
}

#[test]
fn a_single_seats_findings_carry_its_own_site_through_the_real_result_path() {
    let result = json!({"result": "complete", "findings": [reported("unchecked-len", 12)]});
    let (_dir, events) = driven(single_body(driver(&result)), true);
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

/// Ruling 3's three refusals at a gate, and ruling 1's of findings from a
/// work site, each through the real result path: the attempt fails with
/// the refusal named, the bad result is never journaled, and the cap
/// parks the run as any determinate failure does.
#[test]
fn a_seat_supplied_site_a_duplicate_id_and_a_bad_id_each_fail_the_attempt() {
    let site = SiteKey::single("work", None).digest();
    let mut sited = reported("unchecked-len", 12);
    sited["site"] = json!(site);
    let cases = [
        (
            json!([sited]),
            true,
            "finding 0 carries a site: the engine stamps each finding's site (decision 0109 \
             ruling 3)",
        ),
        (
            json!([reported("twice", 1), reported("twice", 9)]),
            true,
            "finding 'twice' is reported twice by one site (decision 0109 ruling 3)",
        ),
        (
            json!([reported("Not.An.Id", 1)]),
            true,
            "finding 0 has no id matching ^[a-z0-9][a-z0-9._-]*$ of at most 64 bytes \
             (decision 0109 ruling 3)",
        ),
        (
            json!([reported("unchecked-len", 12)]),
            false,
            "the result carries 'findings' from a work site: only a gate reports findings \
             (decision 0109 ruling 1)",
        ),
    ];
    for (findings, gate, refusal) in cases {
        let result = json!({"result": "complete", "findings": findings});
        let (_dir, events) = driven(single_body(driver(&result)), gate);
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
    let (_dir, events) = driven(body, true);
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
