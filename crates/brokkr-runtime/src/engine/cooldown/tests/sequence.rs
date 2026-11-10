//! Decision 0103 slice 4c2b: a sequence's rate-limited step records its
//! sites' waits — `<step>` for a single step, `<step>:<member>` for a step
//! panel's member — the engine waits and probes every cooled site, and the
//! retry restarts from step 1. An indeterminate step parks carrying its
//! refused sites' waits; a dialect step writes none.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

use super::panel::{indeterminate_once, last_terminal, member};
use super::*;
use crate::bundle::{
    Aggregate, DialectExecution, Invocation, PanelMember, SeatBody, SeatClass, SequenceStep,
    StepBody,
};
use crate::engine::Engine;

/// One sequence step in `body`, with its own results vocabulary.
fn step(name: &str, body: StepBody, results: &[&str]) -> SequenceStep {
    uncompiled_sequence_step! {
        name: name.into(),
        class: SeatClass::Work,
        results: results.iter().map(|result| (*result).to_string()).collect(),
        body,
    }
}

/// One single-driver step resolved to `candidates`.
fn single_step(name: &str, candidates: Vec<Candidate>, results: &[&str]) -> SequenceStep {
    step(
        name,
        StepBody::Invocation(Invocation::Single {
            role_path: "role.md".into(),
            command: vec!["unused".into()],
            candidates,
            written: Vec::new(),
            class: (),
        }),
        results,
    )
}

/// One panel step of `members`.
fn panel_step(name: &str, members: Vec<PanelMember>, results: &[&str]) -> SequenceStep {
    step(
        name,
        StepBody::Invocation(Invocation::Panel {
            members,
            aggregate: Aggregate::UnanimousPass,
        }),
        results,
    )
}

/// The two-route members the step-panel tests refuse: `m` on claude/first
/// and `n` on codex/second.
fn two_member_panel(m: Vec<String>, n: Vec<String>) -> Vec<PanelMember> {
    vec![
        member("m", vec![candidate("claude", "claude/first", m)]),
        member("n", vec![candidate("codex", "codex/second", n)]),
    ]
}

/// A sequence seat over `steps`.
fn sequence(steps: Vec<SequenceStep>) -> SeatBody {
    SeatBody::Sequence { steps }
}

/// A sequence engine with the standard host and a test clock.
fn sequence_engine(dir: &Path, body: SeatBody, attempts: u64) -> Engine {
    let mut engine = started(dir, body, work(attempts));
    engine.cooldown.host = Some(host(dir, 1, 10));
    with_clock(engine).0
}

/// The waits one terminal event spells.
fn waits(event: &EventEnvelope) -> Vec<Value> {
    event.payload["waits"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

/// The first requested effect's failed events, so a later phase's own
/// failure is not read as this sequence's.
fn failed_here(engine: &Engine) -> Vec<EventEnvelope> {
    let events = engine.store.load(&engine.run_id).unwrap();
    let first = events
        .iter()
        .find(|event| event.event_type == EventType::EffectRequested)
        .map(|event| event.payload["effect_id"].clone());
    failed(engine)
        .into_iter()
        .filter(|event| Some(&event.payload["effect_id"]) == first.as_ref())
        .collect()
}

/// The driven panel-step engine the two member tests share.
fn driven_panel(dir: &Path, m: Vec<String>, n: Vec<String>) -> Engine {
    let body = sequence(vec![panel_step(
        "review",
        two_member_panel(m, n),
        &["complete"],
    )]);
    let mut engine = sequence_engine(dir, body, 2);
    engine.drive().unwrap();
    engine
}

/// The one wait the first effect's terminal event carries.
fn one_wait(engine: &Engine) -> Vec<Value> {
    let failed = failed_here(engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    let waits = waits(&failed[0]);
    assert_eq!(waits.len(), 1, "{waits:?}");
    waits
}

/// The shared log every fake driver appends its tagged line to, in the
/// engine's own order.
fn shared(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("all.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// A sequence whose single `draft` step refuses once, then succeeds: one
/// `<step>` wait, draft's own probe, and a retry from step 1.
#[test]
fn a_single_steps_rate_limit_records_its_site_and_restarts_at_step_one() {
    let dir = tempfile::tempdir().unwrap();
    let command = driver(
        dir.path(),
        "draft",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let body = sequence(vec![single_step(
        "draft",
        vec![candidate("claude", "claude/first", command)],
        &["complete"],
    )]);
    let mut engine = sequence_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "draft"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    let waits = one_wait(&engine);
    assert_eq!(waits[0]["site"], "draft");
    assert_eq!(waits[0]["provider"], "claude");
    assert_eq!(waits[0]["route"], "first");
    assert_eq!(waits[0]["seconds"], 1);
    assert_eq!(waits[0]["rounds"], 1);
    // The probe is the step's own spawn, under the step's driver seat.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("draft.seat")).unwrap(),
        "work:draft"
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["site"], "draft");
}

/// A step panel whose member `m` refuses: the wait is tagged
/// `<step>:<member>`, m's probe is derived through `member_runs` at
/// `work:review:m`, and the whole step re-runs from step 1.
#[test]
fn a_step_panels_refused_member_records_its_tag_and_probes_its_own_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let m = driver(
        dir.path(),
        "m",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let n = driver(
        dir.path(),
        "n",
        &["succeed", "succeed"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    let engine = driven_panel(dir.path(), m, n);
    let waits = one_wait(&engine);
    assert_eq!(waits[0]["site"], "review:m");
    assert_eq!(waits[0]["provider"], "claude");
    assert_eq!(waits[0]["route"], "first");
    assert_eq!(
        logs(dir.path(), "m"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    // A succeeded member re-runs with the whole step, from step 1.
    assert_eq!(
        logs(dir.path(), "n"),
        ["attempt:succeed", "attempt:succeed"]
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("m.seat")).unwrap(),
        "work:review:m"
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["site"], "review:m");
}

/// A later step's refusal: one wait at `review`, and the retry runs `draft`
/// again before `review`.
#[test]
fn a_later_steps_refusal_restarts_the_sequence_from_step_one() {
    let dir = tempfile::tempdir().unwrap();
    let draft = driver(
        dir.path(),
        "draft",
        &["succeed", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let review = driver(
        dir.path(),
        "review",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let body = sequence(vec![
        single_step(
            "draft",
            vec![candidate("claude", "claude/first", draft)],
            &["complete"],
        ),
        single_step(
            "review",
            vec![candidate("claude", "claude/first", review)],
            &["complete"],
        ),
    ]);
    let mut engine = sequence_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    let waits = one_wait(&engine);
    assert_eq!(waits[0]["site"], "review");
    assert_eq!(
        shared(dir.path()),
        [
            "draft:attempt:succeed",
            "review:attempt:refuse",
            "review:probe",
            "draft:attempt:succeed",
            "review:attempt:succeed",
        ]
    );
}

/// A step panel with one member indeterminate and one refused: the
/// `effect/indeterminate` carries the refused member's `<step>:<member>`
/// wait, the run parks, and there is no probe and no retry.
#[test]
fn an_indeterminate_step_carries_the_refused_members_wait_and_probes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let m = driver(
        dir.path(),
        "m",
        &["refuse"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let n = indeterminate_once(dir.path(), "n");
    let engine = driven_panel(dir.path(), m, n);
    let terminal = last_terminal(&engine);
    assert_eq!(terminal.event_type, EventType::EffectIndeterminate);
    let waits = waits(&terminal);
    assert_eq!(waits.len(), 1, "{waits:?}");
    assert_eq!(waits[0]["site"], "review:m");
    assert_eq!(logs(dir.path(), "m"), ["attempt:refuse"]);
    assert_eq!(logs(dir.path(), "n"), ["attempt:indeterminate"]);
    assert_eq!(starts(&engine).len(), 1, "the indeterminate step retried");
    assert_eq!(
        brokkr_core::fold::fold(&engine.store.load(&engine.run_id).unwrap())
            .unwrap()
            .run
            .status(),
        brokkr_core::fold::Status::AwaitingOperator
    );
}

/// A single step whose candidate cannot spawn writes its start-failure site
/// and no wait: a spawn failure has no report and so no typed refusal.
#[test]
fn a_spawn_failure_writes_its_start_failure_site_and_no_wait() {
    let dir = tempfile::tempdir().unwrap();
    let body = sequence(vec![single_step(
        "draft",
        vec![candidate(
            "claude",
            "claude/first",
            vec!["/nonexistent/brokkr-test-driver".into()],
        )],
        &["complete"],
    )]);
    let mut engine = sequence_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    let failed = failed_here(&engine);
    assert!(!failed.is_empty(), "the spawn failure journaled nothing");
    for event in &failed {
        assert_eq!(event.payload["start_failure"], json!(true), "{event:?}");
        assert_eq!(
            event.payload["start_failure_sites"],
            json!(["draft"]),
            "{event:?}"
        );
        assert!(event.payload.get("waits").is_none(), "{event:?}");
    }
}

/// A dialect step writes no wait: it has no typed refusal and its
/// `start_failures` stays empty.
#[test]
fn a_dialect_step_writes_no_wait() {
    let dir = tempfile::tempdir().unwrap();
    let step = uncompiled_sequence_step! {
        name: "validate".into(),
        class: SeatClass::Work,
        results: vec!["complete".into()],
        body: StepBody::Dialect {
            execution: DialectExecution {
                argv: vec!["true".into()],
                state: None,
            },
        },
    };
    let mut engine = sequence_engine(dir.path(), sequence(vec![step]), 1);
    engine.drive().unwrap();
    let failed = failed_here(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert!(failed[0].payload.get("waits").is_none(), "{:?}", failed[0]);
    assert!(
        failed[0].payload.get("refused").is_none(),
        "{:?}",
        failed[0]
    );
}

/// A fresh engine over a journal whose last `effect/failed` carries a
/// `review:m` wait waits the remainder, probes m, and restarts from step 1.
#[test]
fn a_resume_waits_the_recorded_remainder_probes_the_member_and_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let draft = driver(
        dir.path(),
        "draft",
        &["succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let m = driver(
        dir.path(),
        "m",
        &["succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let n = driver(
        dir.path(),
        "n",
        &["succeed"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    let members = two_member_panel(m, n);
    let body = sequence(vec![
        single_step(
            "draft",
            vec![candidate("claude", "claude/first", draft)],
            &["complete"],
        ),
        panel_step("review", members, &["complete"]),
    ]);
    let mut engine = started(dir.path(), body, work(2));
    request_effect(&mut engine);
    engine
        .append(
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "a1", "driver": "d"}),
            Some("a1".to_string()),
        )
        .unwrap();
    engine
        .append(
            EventType::EffectFailed,
            json!({
                "effect_id": "effect",
                "attempt_id": "a1",
                "error": "sequence step 'review': provider refused",
                "start_failure": true,
                "start_failure_sites": ["review:m"],
                "waits": [{
                    "site": "review:m",
                    "provider": "claude",
                    "route": "first",
                    "words": "rate_limit: slow down",
                    "seconds": 1,
                    "until": (base() + time::Duration::seconds(1))
                        .format(&Rfc3339)
                        .unwrap(),
                    "rounds": 1,
                }],
            }),
            Some("a1".to_string()),
        )
        .unwrap();
    let compiled = engine.bundle.clone();
    let run = engine.run_id.clone();
    drop(engine);
    let resumed = resume_engine(dir.path(), compiled, &run);
    let mut resumed = resumed;
    resumed.cooldown.host = Some(host(dir.path(), 1, 10));
    let (mut resumed, now) = with_clock_at(resumed, base());
    resumed.drive().unwrap();
    assert_eq!(
        *now.lock().unwrap(),
        base() + time::Duration::seconds(1),
        "the resume did not wait the recorded remainder"
    );
    let log = shared(dir.path());
    assert_eq!(log.len(), 4, "{log:?}");
    assert_eq!(log[0], "m:probe", "the probe did not come before the retry");
    // The panel's two members succeed concurrently, so only their presence
    // is ordered; the retry itself is the only draft attempt.
    for line in [
        "draft:attempt:succeed",
        "m:attempt:succeed",
        "n:attempt:succeed",
    ] {
        assert!(log[1..].iter().any(|entry| entry == line), "{log:?}");
    }
    let started = starts(&resumed);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["site"], "review:m");
}
