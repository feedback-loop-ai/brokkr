//! Decision 0103 slice 4c2b: a sequence's rate-limited step records its
//! sites' waits — `<step>` for a single step, `<step>:<member>` for a step
//! panel's member — the engine waits and probes every cooled site, and the
//! retry restarts from step 1. An indeterminate step parks carrying its
//! refused sites' waits; a dialect step writes none.

use std::path::Path;

use brokkr_core::envelope::EventType;
use brokkr_core::{EventEnvelope, ProviderWait};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;

#[cfg(target_os = "linux")]
use super::lease::Claimed;
use super::panel::{indeterminate_once, last_terminal, member};
use super::*;
use crate::bundle::{
    Aggregate, DialectExecution, Invocation, PanelMember, SeatBody, SeatClass, SequenceStep,
    StepBody,
};
use crate::engine::cooldown::refused_members;
use crate::engine::start_failure_sites;
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
            aggregate: Aggregate::ReviewPanel,
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

/// A driver that always succeeds, so a test's later step can follow the
/// step under test and the sequence completes.
fn succeeding(dir: &Path, tag: &str) -> Vec<String> {
    driver(
        dir,
        tag,
        &["succeed", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    )
}

/// The final `publish` step every sequence test runs after its refusing
/// step, so the sequence is a shape a compiled bundle has (two steps or
/// more).
fn publish_step(dir: &Path) -> SequenceStep {
    single_step(
        "publish",
        vec![candidate(
            "claude",
            "claude/first",
            succeeding(dir, "publish"),
        )],
        &["complete"],
    )
}

/// A member `m` driver on claude/first that refuses once and then answers.
fn refusing_m(dir: &Path) -> Vec<String> {
    driver(
        dir,
        "m",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    )
}

/// A `draft` single step followed by `publish`, so every refusing step is
/// non-final and the sequence is a shape a compiled bundle has.
fn draft_body(dir: &Path, draft: Vec<String>, results: &[&str]) -> SeatBody {
    sequence(vec![
        single_step(
            "draft",
            vec![candidate("claude", "claude/first", draft)],
            results,
        ),
        publish_step(dir),
    ])
}

/// A terminal event that carries neither a wait nor a refusal record.
fn no_wait(event: &EventEnvelope) {
    assert!(event.payload.get("waits").is_none(), "{event:?}");
    assert!(event.payload.get("refused").is_none(), "{event:?}");
}

/// The probe's context reads only the failed attempt's finished steps: a
/// non-checkpoint event, another effect's checkpoint, another attempt's,
/// a missing checkpoint, a member checkpoint, and a finished step without
/// its name are all skipped, and the valid one is kept.
#[test]
fn a_step_probe_context_reads_only_the_failed_attempts_finished_steps() {
    let checkpoint = |effect: &str, attempt: &str, checkpoint: Value| {
        crate::engine::tests::event(
            EventType::EffectCheckpointed,
            json!({"effect_id": effect, "attempt_id": attempt, "checkpoint": checkpoint}),
        )
    };
    let finished = json!({"step": "sequence-step-finished"});
    let events = vec![
        crate::engine::tests::event(EventType::RunStarted, json!({})),
        checkpoint("other", "a1", json!({"step": "sequence-step-finished"})),
        checkpoint("effect", "a0", json!({"step": "sequence-step-finished"})),
        checkpoint("effect", "a1", Value::Null),
        crate::engine::tests::event(
            EventType::EffectCheckpointed,
            json!({"effect_id": "effect", "attempt_id": "a1"}),
        ),
        checkpoint("effect", "a1", json!({"step": "answer"})),
        checkpoint("effect", "a1", finished),
        checkpoint(
            "effect",
            "a1",
            json!({"step": "sequence-step-finished", "step_name": "draft",
                   "result": {"result": "complete"}}),
        ),
        crate::engine::tests::event(
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "a1"}),
        ),
    ];
    let context = crate::engine::cooldown::probe::step_context(
        &events,
        "effect",
        &json!({"context": {"run_id": "r"}}),
    );
    assert_eq!(context["run_id"], "r");
    assert_eq!(
        context["prior_results"]["draft"]["result"], "complete",
        "{context}"
    );
}

/// A requested sequence engine with its journal and folded state, for the
/// probe-level rows.
fn requested_sequence(
    dir: &Path,
    body: SeatBody,
) -> (Engine, Vec<EventEnvelope>, brokkr_core::fold::RunState) {
    let mut engine = sequence_engine(dir, body, 2);
    request_effect(&mut engine);
    let events = engine.store.load(&engine.run_id).unwrap();
    let state = brokkr_core::fold::fold(&events).unwrap();
    (engine, events, state)
}

/// The single draft step's driver: refuses once, then answers.
fn draft_driver(dir: &Path) -> Vec<String> {
    driver(
        dir,
        "draft",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    )
}

/// The settled facts a single-step wait row shares: the driver ran, probed,
/// and retried, and the terminal event carries one `<step>` wait.
fn single_step_facts(dir: &Path, engine: &Engine) {
    assert_eq!(
        logs(dir, "draft"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    let waits = one_wait(engine);
    assert_eq!(waits[0]["site"], "draft");
}

/// A driven sequence engine, so each row states only its own body.
fn driven(dir: &Path, body: SeatBody, attempts: u64) -> Engine {
    let mut engine = sequence_engine(dir, body, attempts);
    engine.drive().unwrap();
    engine
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

/// The driven panel-step engine the two member tests share: `review` is a
/// non-final step, so its probe exercises the step-input arm a compiled
/// multi-step sequence always has.
fn driven_members(dir: &Path, members: Vec<PanelMember>) -> Engine {
    let body = sequence(vec![
        panel_step("review", members, &["reviewed", "complete"]),
        publish_step(dir),
    ]);
    let mut engine = sequence_engine(dir, body, 2);
    engine.drive().unwrap();
    engine
}

fn driven_panel(dir: &Path, m: Vec<String>, n: Vec<String>) -> Engine {
    driven_members(dir, two_member_panel(m, n))
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
/// `<step>` wait, draft's own probe with the step's own input, and a retry
/// from step 1.
#[test]
fn a_single_steps_rate_limit_records_its_site_and_restarts_at_step_one() {
    let dir = tempfile::tempdir().unwrap();
    let command = draft_driver(dir.path());
    let body = draft_body(dir.path(), command, &["drafted", "complete"]);
    let engine = driven(dir.path(), body, 2);
    single_step_facts(dir.path(), &engine);
    let waits = one_wait(&engine);
    assert_eq!(waits[0]["provider"], "claude");
    assert_eq!(waits[0]["route"], "first");
    assert_eq!(waits[0]["seconds"], 1);
    assert_eq!(waits[0]["rounds"], 1);
    // The probe is the step's own spawn, under the step's driver seat, with
    // the step's own allowed results (not the seat's final-step vocabulary).
    assert_eq!(
        std::fs::read_to_string(dir.path().join("draft.seat")).unwrap(),
        "work:draft"
    );
    let inputs = start_inputs(dir.path(), "draft");
    assert_eq!(inputs.len(), 3, "{inputs:?}");
    assert_eq!(inputs[1]["input"]["seat"], "work:draft");
    assert_eq!(
        inputs[1]["input"]["allowed_results"],
        json!(["drafted", "complete"]),
        "{inputs:?}"
    );
    assert_eq!(inputs[1]["input"]["context"]["prior_results"], json!({}));
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["site"], "draft");
}

/// A sequence-probe row with a dialect prompt: the step's attempt and its
/// cool-down probe both derive the one step input, so both carry the
/// seat's spec_dialect and the probe asks exactly the input the attempt
/// ran.
#[test]
fn a_step_probe_carries_the_seats_dialect_prompt_the_attempt_did() {
    let dir = tempfile::tempdir().unwrap();
    let command = draft_driver(dir.path());
    let body = draft_body(dir.path(), command, &["drafted", "complete"]);
    let mut engine = sequence_engine(dir.path(), body, 2);
    engine
        .bundle
        .dialect_prompts
        .insert("work".into(), "Write the dialect change.".into());
    engine.drive().unwrap();
    single_step_facts(dir.path(), &engine);
    let inputs = start_inputs(dir.path(), "draft");
    assert_eq!(inputs.len(), 3, "{inputs:?}");
    assert_eq!(
        inputs[0]["input"]["spec_dialect"], "Write the dialect change.",
        "the attempt lost the seat's dialect prompt: {inputs:?}"
    );
    assert_eq!(
        inputs[1]["input"]["spec_dialect"], "Write the dialect change.",
        "the probe lost the seat's dialect prompt: {inputs:?}"
    );
}

/// The `review` panel step of a refusing `m` and a succeeding `n`, with
/// `publish` after it: the step-panel rows' shared body.
fn refusing_panel_sequence(dir: &Path) -> SeatBody {
    let m = refusing_m(dir);
    let n = driver(
        dir,
        "n",
        &["succeed", "succeed"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    sequence(vec![
        panel_step("review", two_member_panel(m, n), &["reviewed", "complete"]),
        publish_step(dir),
    ])
}

/// The `draft` single step that refuses once with an eleven-second
/// retry-after, past the test host's ten-second maximum, followed by
/// `publish`: the park rows' shared body.
fn past_maximum_body(dir: &Path) -> SeatBody {
    let draft = driver(
        dir,
        "draft",
        &["refuse"],
        Probed::Answer,
        Some(11),
        "claude",
        "first",
    );
    draft_body(dir, draft, &["complete"])
}

/// A step panel whose member `m` refuses: the wait is tagged
/// `<step>:<member>`, m's probe is derived through `member_runs` at
/// `work:review:m` with the step's own allowed results, and the whole step
/// re-runs from step 1.
#[test]
fn a_step_panels_refused_member_records_its_tag_and_probes_its_own_spawn() {
    let dir = tempfile::tempdir().unwrap();
    let engine = driven(dir.path(), refusing_panel_sequence(dir.path()), 2);
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
    let inputs = start_inputs(dir.path(), "m");
    assert_eq!(inputs.len(), 3, "{inputs:?}");
    assert_eq!(inputs[1]["input"]["seat"], "work:review:m");
    assert_eq!(
        inputs[1]["input"]["allowed_results"],
        json!(["reviewed", "complete"]),
        "{inputs:?}"
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["site"], "review:m");
}

/// A later, non-final step's refusal: one wait at `review`, a probe with
/// the step's own allowed results and the earlier steps' results, and the
/// retry runs `draft` again before `review`.
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
            &["reviewed", "complete"],
        ),
        publish_step(dir.path()),
    ]);
    let engine = driven(dir.path(), body, 2);
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
            "publish:attempt:succeed",
        ]
    );
    let inputs = start_inputs(dir.path(), "review");
    assert_eq!(inputs.len(), 3, "{inputs:?}");
    assert_eq!(
        inputs[1]["input"]["allowed_results"],
        json!(["reviewed", "complete"]),
        "the probe did not ask with the step's own vocabulary: {inputs:?}"
    );
    assert_eq!(
        inputs[1]["input"]["context"]["prior_results"]["draft"]["result"], "complete",
        "the probe did not carry the earlier step's result: {inputs:?}"
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
    let body = sequence(vec![
        single_step(
            "draft",
            vec![candidate(
                "claude",
                "claude/first",
                vec!["/nonexistent/brokkr-test-driver".into()],
            )],
            &["complete"],
        ),
        publish_step(dir.path()),
    ]);
    let engine = driven(dir.path(), body, 2);
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
    let engine = driven(
        dir.path(),
        sequence(vec![step, publish_step(dir.path())]),
        1,
    );
    let failed = failed_here(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    no_wait(&failed[0]);
}

/// The failed arm's park is written in the attempt's own turn: one
/// `drive_once` from the requested effect leaves the failed event and its
/// park adjacent, before the cursor could park through the refused record.
#[test]
fn the_failed_steps_park_lands_in_the_attempts_own_turn() {
    let dir = tempfile::tempdir().unwrap();
    let body = past_maximum_body(dir.path());
    let mut engine = sequence_engine(dir.path(), body, 2);
    request_effect(&mut engine);
    engine.drive_once().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let failed = events
        .iter()
        .position(|event| event.event_type == EventType::EffectFailed)
        .expect("the failed step of the one turn");
    assert_eq!(
        events.get(failed + 1).map(|event| &event.event_type),
        Some(&EventType::RunParked),
        "the failed arm did not park in its own turn: {events:?}"
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("past the declared 10s maximum"), "{reason}");
}

/// The sequence engine `body` whose first wait slice admits a competing
/// claim: its probe's admission is refused, and the run parks naming it.
#[cfg(target_os = "linux")]
fn refused_probe(dir: &Path, body: SeatBody) -> Engine {
    let mut engine = started(dir, body, work(2));
    engine.cooldown.host = Some(host(dir, 1, 10));
    let (mut engine, now) = with_clock(engine);
    let Claimed { _stand, clock } =
        super::lease::claim_after_first_slice(super::lease::home(dir), dir.join("work"), &now);
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    engine
}

/// The park a refused probe admission leaves: the competing run's hold is
/// named and `tag` ran only its refused attempt.
#[cfg(target_os = "linux")]
fn assert_refused_probe(engine: &Engine, dir: &Path, tag: &str) {
    let reason = park_reason(engine);
    assert!(reason.contains("not admitted"), "{reason}");
    assert!(
        reason.contains("run 'run-p' holds this worktree"),
        "{reason}"
    );
    assert_eq!(logs(dir, tag), ["attempt:refuse"]);
}

/// A single step's probe admission refused at the root parks naming the
/// refusal before any probe or retry: `admit_step` and `probe_step_single`
/// return the parked probe.
#[cfg(target_os = "linux")]
#[test]
fn a_single_steps_refused_probe_admission_parks_before_it_spawns() {
    let dir = tempfile::tempdir().unwrap();
    let command = draft_driver(dir.path());
    let body = draft_body(dir.path(), command, &["complete"]);
    let engine = refused_probe(dir.path(), body);
    assert_refused_probe(&engine, dir.path(), "draft");
}

/// A step member's probe admission refused at the root parks the same way:
/// `probe_step_member` returns the parked probe and no driver runs.
#[cfg(target_os = "linux")]
#[test]
fn a_step_members_refused_probe_admission_parks_before_it_spawns() {
    let dir = tempfile::tempdir().unwrap();
    let body = refusing_panel_sequence(dir.path());
    let engine = refused_probe(dir.path(), body);
    assert_refused_probe(&engine, dir.path(), "m");
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

/// A single step with no candidate chain runs its inline command; its
/// probe asks that same command, with no selected link to derive.
#[test]
fn an_inline_single_steps_probe_uses_its_own_command() {
    let dir = tempfile::tempdir().unwrap();
    let command = draft_driver(dir.path());
    let body = sequence(vec![
        step(
            "draft",
            StepBody::Invocation(Invocation::Single {
                role_path: "role.md".into(),
                command,
                candidates: Vec::new(),
                written: Vec::new(),
                class: (),
            }),
            &["complete"],
        ),
        publish_step(dir.path()),
    ]);
    let engine = driven(dir.path(), body, 2);
    single_step_facts(dir.path(), &engine);
}

/// A wait whose retry-after is past the host maximum parks the sequence at
/// its failing step: the failed event carries the refused record, no probe
/// runs and no retry follows.
#[test]
fn a_wait_past_the_maximum_parks_the_sequence_without_a_probe() {
    let dir = tempfile::tempdir().unwrap();
    let body = past_maximum_body(dir.path());
    let mut engine = sequence_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    let failed = failed_here(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].payload["refused"]["site"], "draft");
    assert_eq!(failed[0].payload["refused"]["cause"], "past-maximum");
    // 4c2a's panel writer keeps the computable wait beside the record.
    assert_eq!(waits(&failed[0])[0]["site"], "draft");
    assert_eq!(logs(dir.path(), "draft"), ["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1, "a parked step retried");
    let reason = park_reason(&engine);
    assert!(reason.contains("past the declared 10s maximum"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
}

/// A rate-limit refusal whose cool-down is not declared writes no wait and
/// follows decision 0006: the failed event's note names the absence, and
/// the attempt retries without a probe.
#[test]
fn a_missing_declaration_writes_no_wait_and_retries() {
    let dir = tempfile::tempdir().unwrap();
    let draft = draft_driver(dir.path());
    let body = draft_body(dir.path(), draft, &["complete"]);
    let mut engine = sequence_engine(dir.path(), body, 2);
    engine.cooldown.host = Some(v1_host(dir.path()));
    engine.drive().unwrap();
    let failed = failed_here(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    no_wait(&failed[0]);
    let error = failed[0].payload["error"].as_str().unwrap();
    assert!(
        error.contains("no cool-down is declared for claude/first"),
        "{error}"
    );
    assert_eq!(
        logs(dir.path(), "draft"),
        ["attempt:refuse", "attempt:succeed"]
    );
    assert_eq!(starts(&engine).len(), 2);
}

/// Another member's non-rate-limit refusal beside a rate limit gets no wait
/// and no probe: only the rate-limited member's site is cooled down.
#[test]
fn another_refusal_beside_a_rate_limit_gets_no_wait_and_no_probe() {
    let dir = tempfile::tempdir().unwrap();
    let m = refusing_m(dir.path());
    let mut n = driver(
        dir.path(),
        "n",
        &["refuse"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    let script = n
        .pop()
        .unwrap()
        .replace("\"class\":\"rate_limit\"", "\"class\":\"other\"");
    n.push(script);
    let n_second = succeeding(dir.path(), "n-second");
    let members = vec![
        member("m", vec![candidate("claude", "claude/first", m)]),
        member(
            "n",
            vec![
                candidate("codex", "codex/second", n),
                candidate("codex", "codex/third", n_second),
            ],
        ),
    ];
    let engine = driven_members(dir.path(), members);
    let waits = one_wait(&engine);
    assert_eq!(waits[0]["site"], "review:m");
    assert_eq!(
        logs(dir.path(), "m"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(
        logs(dir.path(), "n"),
        ["attempt:refuse"],
        "n was probed for a refusal that is not a rate limit"
    );
    assert_eq!(
        logs(dir.path(), "n-second"),
        ["attempt:succeed"],
        "n's chain did not advance to its next link"
    );
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(
        started[1]["probes"].as_array().map(Vec::len),
        Some(1),
        "{started:?}"
    );
    assert_eq!(started[1]["probes"][0]["site"], "review:m");
}

/// A wait at `a:b:c` resolves the declared step `a:b` and its member `c`,
/// not step `a`'s member `b:c`: a step name may contain a colon.
#[test]
fn a_member_wait_resolves_the_declared_step_and_member_pair() {
    let dir = tempfile::tempdir().unwrap();
    let c = succeeding(dir.path(), "c");
    let body = sequence(vec![
        panel_step(
            "a",
            vec![member("m", Vec::new()), member("n", Vec::new())],
            &["complete"],
        ),
        panel_step(
            "a:b",
            vec![member("c", vec![candidate("claude", "claude/first", c)])],
            &["complete"],
        ),
    ]);
    let (mut engine, events, state) = requested_sequence(dir.path(), body);
    let wait = ProviderWait {
        site: Some("a:b:c".into()),
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        seconds: 1,
        until: base(),
        rounds: 1,
    };
    let probe = engine
        .probe_member(&events, &state, "effect", "work", &wait, "a:b:c")
        .unwrap();
    match probe {
        crate::engine::cooldown::probe::Probe::Answered(_) => {}
        crate::engine::cooldown::probe::Probe::Parked(reason) => {
            panic!("a:b:c was not a:b's member c: {reason}")
        }
    }
    assert_eq!(logs(dir.path(), "c"), ["probe"]);
}

/// A refused site is a wait even when its cleanup made the step
/// indeterminate: the wait assembly reads every report's typed refusal,
/// while the start-failure fact keeps only a failure to start.
#[test]
fn a_refused_site_is_a_wait_even_when_its_cleanup_makes_the_step_indeterminate() {
    let dir = tempfile::tempdir().unwrap();
    let engine = sequence_engine(dir.path(), sequence(vec![publish_step(dir.path())]), 2);
    let rate: brokkr_protocol::PreSessionRefusal = serde_json::from_value(json!({
        "class": "rate_limit", "provider": "claude", "route": "first",
        "words": "rate_limit: slow down"
    }))
    .unwrap();
    let unsettled = || brokkr_protocol::Cleanup::Unresolved {
        reason: brokkr_protocol::process::Unsettled::Stdout,
    };
    let refused_report = report(Some(Box::new(rate)), unsettled(), Vec::new());
    let indeterminate_report = report(None, unsettled(), Vec::new());
    let reports = vec![
        ("m".to_string(), refused_report),
        ("n".to_string(), indeterminate_report),
    ];
    assert!(
        start_failure_sites(&reports, "review:").is_empty(),
        "a cleanup-indeterminate report is not a failure to start"
    );
    let refused = refused_members(&reports, "review:");
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(refused[0].site, "review:m");
    let terminal = engine
        .step_terminal(
            "effect",
            json!({"effect_id": "effect", "attempt_id": "a1", "reason": "indeterminate"}),
            &refused,
            false,
        )
        .unwrap();
    assert_eq!(terminal.payload["waits"][0]["site"], "review:m");
    assert!(terminal.park.is_none(), "{:?}", terminal.park);
}

/// A sequence wait that names no step, names a panel step without a member,
/// names a member of a single step, or names a member the panel does not
/// declare parks by name before anything spawns.
#[test]
fn a_sequence_wait_at_a_form_it_does_not_name_parks_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let body = sequence(vec![
        single_step("draft", Vec::new(), &["complete"]),
        panel_step(
            "review",
            vec![member("m", Vec::new()), member("n", Vec::new())],
            &["complete"],
        ),
    ]);
    let (mut engine, events, state) = requested_sequence(dir.path(), body);
    let wait = |site: &str| ProviderWait {
        site: Some(site.into()),
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit".into(),
        retry_after_seconds: None,
        seconds: 1,
        until: base(),
        rounds: 1,
    };
    for (site, expected) in [
        ("unknown", "declares no step"),
        ("review", "is not a single driver"),
        ("draft:member", "is not a panel"),
        ("review:unknown", "declares no member"),
        ("review:m", "did not spawn"),
    ] {
        let probe = engine
            .probe_member(&events, &state, "effect", "work", &wait(site), site)
            .unwrap();
        match probe {
            crate::engine::cooldown::probe::Probe::Parked(reason) => {
                assert!(reason.contains(expected), "{site}: {reason}");
            }
            crate::engine::cooldown::probe::Probe::Answered(_) => {
                panic!("{site} was probed")
            }
        }
    }
}
