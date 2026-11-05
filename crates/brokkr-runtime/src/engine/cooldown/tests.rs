//! Decision 0103 slice 4c1: one seat waits out a provider's rate limit.
//! Each test plants the fake driver's reports and the host file, drives the
//! run, and reads the events, the park reason or the attempt count.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use brokkr_core::canonical::sha256_hex;
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Status};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use super::Clock;
use crate::agents::{Candidate, HarnessHands, Lowering};
use crate::bundle::{Aggregate, Invocation, Limits, PanelMember, SeatBody, SeatClass, StepBody};
use crate::engine::tests::{bundle, single_body, templated};
use crate::engine::Engine;

fn base() -> OffsetDateTime {
    OffsetDateTime::parse("2026-11-01T00:00:00Z", &Rfc3339).unwrap()
}

/// A seat with the limits this test needs.
fn started(dir: &Path, body: SeatBody, limits: Limits) -> Engine {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    let mut compiled = bundle(dir, body);
    compiled.seats.get_mut("work").unwrap().limits = limits;
    Engine::start(store, compiled, "Feature", Some(dir.join("work"))).unwrap()
}

fn work(attempts: u64) -> Limits {
    Limits {
        max_attempts: attempts,
        timeout_seconds: 2,
    }
}

/// A v2 host file declaring cool-downs for claude/first and codex/second.
fn host(dir: &Path, default: u64, max: u64) -> PathBuf {
    let path = dir.join("host.json");
    std::fs::write(
        &path,
        json!({
            "schema": "forge.host/v2",
            "providers": {
                "claude": {"ceiling": 4, "routes": {"first": {"ceiling": 4, "class": "cloud"}}},
                "codex": {"ceiling": 4, "routes": {"second": {"ceiling": 4, "class": "cloud"}}}
            },
            "floors": [{"floor_bytes": 1, "path": "/"}],
            "boxed_builds": {"ceiling": 1},
            "cooldowns": {"default_seconds": default, "max_seconds": max}
        })
        .to_string(),
    )
    .unwrap();
    path
}

/// A v1 host file: capacity, and no cool-downs declared.
fn v1_host(dir: &Path) -> PathBuf {
    let path = dir.join("host-v1.json");
    std::fs::write(
        &path,
        json!({
            "schema": "forge.host/v1",
            "providers": {
                "claude": {"ceiling": 4, "routes": {"first": {"ceiling": 4, "class": "cloud"}}}
            },
            "floors": [{"floor_bytes": 1, "path": "/"}],
            "boxed_builds": {"ceiling": 1}
        })
        .to_string(),
    )
    .unwrap();
    path
}

/// How a probe answers.
#[derive(Clone, Copy)]
enum Probed {
    Answer,
    Refuse,
    Hang,
    Error,
    Unadvertised,
}

/// A `forge-driver/v1` participant in `sh`: its attempts read a verdict
/// per invocation (`refuse` or `succeed`), and its probes do what
/// `probed` says. The log names each invocation in order.
fn driver(
    dir: &Path,
    tag: &str,
    verdicts: &[&str],
    probed: Probed,
    retry_after: Option<u64>,
    provider: &str,
    route: &str,
) -> Vec<String> {
    let log = dir.join(format!("{tag}.log"));
    let count = dir.join(format!("{tag}.count"));
    let verdicts_path = dir.join(format!("{tag}.verdicts"));
    std::fs::write(&verdicts_path, format!("{}\n", verdicts.join("\n"))).unwrap();
    let advertised = !matches!(probed, Probed::Unadvertised);
    let capabilities = format!(
        "{{\"proto\":\"forge-driver/v1\",\"msg_id\":\"c\",\"type\":\"capabilities\",\
         \"driver\":\"test\",\"version\":\"1\",\"supports\":[]{}}}",
        if advertised { ",\"probe\":true" } else { "" }
    );
    let accepted = "printf '%s,\"type\":\"accepted\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\"}\\n' \"$base\" \"$eid\" \"$aid\"";
    let checkpoint = "printf '%s,\"type\":\"checkpoint\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\"data\":{\"step\":\"answer\"}}\\n' \"$base\" \"$eid\" \"$aid\"";
    let succeeded = "printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
        \"status\":\"succeeded\",\"result\":{\"result\":\"complete\"}}\\n' \"$base\" \"$eid\" \"$aid\"";
    let retry = retry_after.map_or_else(String::new, |seconds| {
        format!(",\"retry_after_seconds\":{seconds}")
    });
    let refused = format!(
        "printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
         \"status\":\"failed\",\"error\":\"provider refused\",\
         \"refusal\":{{\"class\":\"rate_limit\",\"provider\":\"{provider}\",\
         \"route\":\"{route}\",\"words\":\"rate_limit: slow down\"{retry}}}}}\\n' \
         \"$base\" \"$eid\" \"$aid\""
    );
    let probe = match probed {
        Probed::Answer => format!("{accepted}\n  {checkpoint}\n  {succeeded}"),
        Probed::Refuse => refused.clone(),
        Probed::Hang => format!("{accepted}\n  sleep 30"),
        Probed::Error => format!(
            "{accepted}\n  printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\
             \"attempt_id\":\"%s\",\"status\":\"failed\",\"error\":\"probe exploded\"}}\\n' \
             \"$base\" \"$eid\" \"$aid\""
        ),
        Probed::Unadvertised => succeeded.to_string(),
    };
    let mut script = String::from(
        r#"n=$(cat '@COUNT@' 2>/dev/null || echo 0)
read -r hello
printf '%s\n' '@CAPABILITIES@'
read -r start
case "$start" in *'"type":"start"'*) ;; *) exit 0 ;; esac
eid=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
aid=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
base='{"proto":"forge-driver/v1","msg_id":"m"'
case "$start" in
  *'"probe":true'*)
    printf '%s\n' 'probe' >> '@LOG@'
    @PROBE@
    ;;
  *)
    n=$((n+1)); printf '%s' "$n" > '@COUNT@'
    verdict=$(sed -n "${n}p" '@VERDICTS@')
    [ -n "$verdict" ] || verdict=$(tail -1 '@VERDICTS@')
    printf '%s\n' "attempt:$verdict" >> '@LOG@'
    if [ "$verdict" = refuse ]; then
      @REFUSED@
    else
      @SUCCEEDED@
    fi
    ;;
esac
read -r bye
"#,
    );
    for (at, with) in [
        ("@COUNT@", count.display().to_string()),
        ("@CAPABILITIES@", capabilities),
        ("@LOG@", log.display().to_string()),
        ("@VERDICTS@", verdicts_path.display().to_string()),
        ("@PROBE@", probe),
        ("@REFUSED@", refused),
        ("@SUCCEEDED@", succeeded.to_string()),
    ] {
        script = script.replace(at, &with);
    }
    vec!["sh".into(), "-c".into(), script]
}

/// The common single-seat command: claude/first refuses once and then
/// succeeds.
fn standard_command(dir: &Path) -> Vec<String> {
    driver(
        dir,
        "work",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    )
}

/// A candidate whose argv runs `script`, with its own provider and model.
fn candidate(provider: &str, model: &str, script: Vec<String>) -> Candidate {
    templated(Candidate {
        agent: "worker".into(),
        model: model.into(),
        effort: Some("high".into()),
        provider: provider.into(),
        argv: script,
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
        lowering: Lowering::Unavailable,
        hands_notice: None,
    })
}

fn logs(dir: &Path, tag: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(format!("{tag}.log")))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn failed(engine: &Engine) -> Vec<brokkr_core::EventEnvelope> {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectFailed)
        .collect()
}

fn park_reason(engine: &Engine) -> String {
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(
        fold(&events).unwrap().run.status(),
        Status::AwaitingOperator
    );
    events
        .iter()
        .rev()
        .find(|event| event.event_type == EventType::RunParked)
        .expect("a parked run")
        .payload["reason"]
        .as_str()
        .unwrap()
        .to_string()
}

fn starts(engine: &Engine) -> Vec<Value> {
    let events = engine.store.load(&engine.run_id).unwrap();
    let first = events
        .iter()
        .find(|event| event.event_type == EventType::EffectStarted)
        .map(|event| event.payload["effect_id"].clone());
    events
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectStarted)
        .filter(|event| Some(&event.payload["effect_id"]) == first.as_ref())
        .map(|event| event.payload)
        .collect()
}

/// Drive the run and pin the log it read, the one assertion the seat tests
/// share.
fn drove(engine: &mut Engine, dir: &Path, expected: &[&str]) {
    engine.drive().unwrap();
    assert_eq!(logs(dir, "work"), expected);
}

fn with_clock_at(mut engine: Engine, at: OffsetDateTime) -> (Engine, Arc<Mutex<OffsetDateTime>>) {
    let (clock, now) = Clock::advancing(at);
    engine.cooldown.clock = clock;
    (engine, now)
}

fn with_clock(engine: Engine) -> (Engine, Arc<Mutex<OffsetDateTime>>) {
    let start = engine.store.load(&engine.run_id).unwrap()[0]
        .recorded_at
        .clone();
    with_clock_at(engine, OffsetDateTime::parse(&start, &Rfc3339).unwrap())
}

/// The common fixture: the seat's driver or a body the test spells, the
/// v2 host file and the run's own clock. A test that needs another host
/// replaces it after this returns.
macro_rules! seated {
    (@in $dir:expr, $body:expr, $attempts:expr) => {{
        let mut engine = started($dir.path(), $body, work($attempts));
        engine.cooldown.host = Some(host($dir.path(), 1, 10));
        let (engine, now) = with_clock(engine);
        ($dir, engine, now)
    }};
    ($body:expr, $attempts:expr) => {{
        let dir = tempfile::tempdir().unwrap();
        seated!(@in dir, $body, $attempts)
    }};
    ($verdicts:expr, $probed:expr, $retry:expr, $attempts:expr) => {{
        let dir = tempfile::tempdir().unwrap();
        let command = driver(
            dir.path(),
            "work",
            $verdicts,
            $probed,
            $retry,
            "claude",
            "first",
        );
        seated!(@in dir, single_body(command), $attempts)
    }};
}

/// One single seat's invocation over `candidates`.
fn chain(candidates: Vec<Candidate>) -> SeatBody {
    SeatBody::Invocation(Invocation::Single {
        role_path: PathBuf::from("role.md"),
        command: vec!["unused".into()],
        candidates,
        written: Vec::new(),
        class: SeatClass::Work,
    })
}

/// The bundle a resumed or synthesized journal runs under, with the two
/// attempts this suite needs.
fn compiled(dir: &Path, body: SeatBody) -> crate::bundle::Bundle {
    let mut bundle = bundle(dir, body);
    bundle.seats.get_mut("work").unwrap().limits = work(2);
    bundle
}

/// An engine holding the crashed attempt's journal, with `waits` on its
/// terminal event, ready for the resume path.
fn journaled(dir: &Path, body: SeatBody, waits: Value) -> Engine {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    let mut engine = Engine::start(
        store,
        compiled(dir, body),
        "Feature",
        Some(dir.join("work")),
    )
    .unwrap();
    append_waited(&mut engine, waits);
    engine
}

#[test]
fn a_rate_limit_waits_for_the_probe_then_retries() {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], Probed::Answer, None, 2);
    engine.drive().unwrap();
    // The probe ran BETWEEN the refused attempt and the retry: the log's
    // order is the engine's, and the removal control that retries first
    // turns it into attempt/probe.
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert!(*now.lock().unwrap() >= base() + time::Duration::seconds(1));
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probe"]["provider"], "claude");
    assert_eq!(started[1]["probe"]["route"], "first");
    assert!(failed(&engine)[0].payload["waits"].is_array());
}

#[test]
fn the_cap_parks_naming_the_cool_down_and_starts_no_third_attempt() {
    let (dir, mut engine, _now) = seated!(&["refuse", "refuse"], Probed::Answer, None, 2);
    drove(
        &mut engine,
        dir.path(),
        &["attempt:refuse", "probe", "attempt:refuse"],
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("waits for claude/first until"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert_eq!(starts(&engine).len(), 2);
}

#[test]
fn a_probe_refused_again_parks_without_a_second_probe_or_attempt() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Refuse, None, 3);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    assert_eq!(starts(&engine).len(), 1);
    assert_eq!(failed(&engine).len(), 1, "the probe spent no attempt");
    let reason = park_reason(&engine);
    assert!(reason.contains("was refused again"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
}

fn parked_probe(probed: Probed) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let (dir, mut engine, now) = seated!(&["refuse", "succeed"], probed, None, 2);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse", "probe"]);
    assert_eq!(starts(&engine).len(), 1);
    (dir, engine, now)
}

#[test]
fn a_probe_that_starts_then_hangs_parks_without_a_retry() {
    let (_dir, engine, _now) = parked_probe(Probed::Hang);
    let reason = park_reason(&engine);
    assert!(reason.contains("ended without an answer"), "{reason}");
}

#[test]
fn a_probe_that_starts_then_errors_parks_without_a_retry() {
    let (_dir, engine, _now) = parked_probe(Probed::Error);
    let reason = park_reason(&engine);
    assert!(reason.contains("ended without an answer"), "{reason}");
    assert!(reason.contains("probe exploded"), "{reason}");
}

#[test]
fn the_probe_asks_the_cooled_route_and_the_retry_takes_the_next_link() {
    let dir = tempfile::tempdir().unwrap();
    let first = driver(
        dir.path(),
        "claude",
        &["refuse"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let second = driver(
        dir.path(),
        "codex",
        &["succeed"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    let body = chain(vec![
        candidate("claude", "claude/first", first),
        candidate("codex", "codex/second", second),
    ]);
    let (dir, mut engine, _now) = seated!(@in dir, body, 2);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "claude"), ["attempt:refuse", "probe"]);
    assert_eq!(logs(dir.path(), "codex"), ["attempt:succeed"]);
    let started = starts(&engine);
    assert_eq!(started.len(), 2);
    assert_eq!(started[1]["provenance"][0]["agent"], "worker");
    assert_eq!(started[1]["probe"]["provider"], "claude");
}

#[test]
fn a_one_candidate_chain_retries_the_cooled_route() {
    let dir = tempfile::tempdir().unwrap();
    let command = standard_command(dir.path());
    let body = chain(vec![candidate("claude", "claude/first", command)]);
    let (dir, mut engine, _now) = seated!(@in dir, body, 2);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(starts(&engine)[1]["probe"]["provider"], "claude");
}

#[test]
fn a_malformed_wait_refuses_by_name_at_and_below_the_cap() {
    for attempts in [2u64, 1] {
        let dir = tempfile::tempdir().unwrap();
        let mut engine = started(
            dir.path(),
            single_body(vec!["unused".into()]),
            work(attempts),
        );
        append_waited(
            &mut engine,
            json!([{"route": "first", "words": "w",
                    "until": "2026-11-01T00:00:00Z", "rounds": 1}]),
        );
        engine.drive().unwrap();
        let reason = park_reason(&engine);
        assert!(reason.contains("does not read"), "{reason}");
        assert!(reason.contains("provider"), "{reason}");
    }
}

#[test]
fn an_operator_stop_during_the_wait_is_honoured() {
    let dir = tempfile::tempdir().unwrap();
    let command = standard_command(dir.path());
    let mut engine = started(dir.path(), single_body(command), work(2));
    engine.cooldown.host = Some(host(dir.path(), 2, 10));
    let (mut engine, now) = with_clock(engine);
    let db = dir.path().join("forge.db");
    let run_id = engine.run_id.clone();
    let stop = Arc::new(Mutex::new(true));
    let read = Arc::clone(&now);
    let advance = Arc::clone(&now);
    let clock = Clock {
        now: Box::new(move || *read.lock().unwrap()),
        sleep: {
            let stop = Arc::clone(&stop);
            Box::new(move |slice: std::time::Duration| {
                *advance.lock().unwrap() += time::Duration::seconds(slice.as_secs() as i64);
                if !std::mem::replace(&mut *stop.lock().unwrap(), false) {
                    return;
                }
                let mut store = brokkr_store::Store::open(&db).unwrap();
                store
                    .append_next(
                        &run_id,
                        EventType::OperatorCommanded,
                        json!({"command_id": "c", "command": "stop", "args": {},
                               "operator": "operator"}),
                        None,
                        None,
                    )
                    .unwrap();
                store
                    .append_next(
                        &run_id,
                        EventType::OperatorAccepted,
                        json!({"command_id": "c", "operator": "operator", "reason": "stop"}),
                        None,
                        None,
                    )
                    .unwrap();
            })
        },
    };
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1);
}

#[test]
fn a_wait_past_the_maximum_parks_with_the_words_and_the_maximum() {
    let (dir, mut engine, _now) = seated!(&["refuse"], Probed::Answer, Some(5), 2);
    engine.cooldown.host = Some(host(dir.path(), 1, 2));
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert!(reason.contains("5s"), "{reason}");
    assert!(reason.contains("declared 2s maximum"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
}

#[test]
fn an_accumulated_episode_past_the_maximum_parks() {
    let (dir, mut engine, _now) =
        seated!(&["refuse", "refuse", "succeed"], Probed::Answer, None, 3);
    engine.cooldown.host = Some(host(dir.path(), 2, 3));
    drove(
        &mut engine,
        dir.path(),
        &["attempt:refuse", "probe", "attempt:refuse"],
    );
    let reason = park_reason(&engine);
    assert!(reason.contains("episode 6s"), "{reason}");
    assert!(reason.contains("declared 3s maximum"), "{reason}");
}

/// A valid wait entry array ending at `until`.
fn wait_value(until: OffsetDateTime) -> Value {
    json!([{"provider": "claude", "route": "first",
            "words": "rate_limit: slow down",
            "until": until.format(&Rfc3339).unwrap(), "rounds": 1}])
}

/// The journal a crashed driver leaves behind: a requested effect, one
/// started attempt and its terminal failure with `waits`.
fn append_waited(engine: &mut Engine, waits: Value) {
    engine
        .append(EventType::PhaseEntered, json!({"phase": "work"}), None)
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let state = fold(&events).unwrap();
    let digest = sha256_hex(&engine.seat_input(&state, "work", "effect").unwrap());
    for (event, payload, attempt) in [
        (
            EventType::EffectRequested,
            json!({"effect_id": "effect", "phase": "work", "seat": "work",
                   "idempotency_key": "k", "input_digest": digest}),
            None,
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "a1", "driver": "d"}),
            Some("a1".to_string()),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
                   "start_failure": true, "start_failure_sites": [null],
                   "waits": waits}),
            Some("a1".to_string()),
        ),
    ] {
        engine.append(event, payload, attempt).unwrap();
    }
}

fn resumed(
    verdicts: &[&str],
    until: OffsetDateTime,
) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    let dir = tempfile::tempdir().unwrap();
    let command = driver(
        dir.path(),
        "work",
        verdicts,
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let body = single_body(command);
    let compiled = compiled(dir.path(), body.clone());
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let store = brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine = Engine::start(
        store,
        compiled.clone(),
        "Feature",
        Some(dir.path().join("work")),
    )
    .unwrap();
    append_waited(&mut engine, wait_value(until));
    let run_id = engine.run_id.clone();
    drop(engine);
    let mut resumed_engine = Engine::resume(
        brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap(),
        compiled,
        &run_id,
        Some(dir.path().join("work")),
    )
    .unwrap();
    let (clock, now) = Clock::advancing(base());
    resumed_engine.cooldown.clock = clock;
    (dir, resumed_engine, now)
}

#[test]
fn a_resume_waits_the_remainder() {
    let (dir, mut engine, now) = resumed(&["succeed"], base() + time::Duration::seconds(3));
    // No declaration on the resume: the recorded end is the wait's.
    engine.cooldown.host = None;
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["probe", "attempt:succeed"]);
    assert_eq!(*now.lock().unwrap(), base() + time::Duration::seconds(3));
    let started = starts(&engine);
    assert!(started.last().unwrap()["probe"].is_object());
}

#[test]
fn a_resume_parks_when_the_remainder_passes_the_maximum() {
    let (dir, mut engine, _now) = resumed(&["succeed"], base() + time::Duration::seconds(5));
    engine.cooldown.host = Some(host(dir.path(), 1, 1));
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty());
    let reason = park_reason(&engine);
    assert!(reason.contains("past the declared 1s maximum"), "{reason}");
}

#[test]
fn a_resume_parks_when_the_host_file_is_invalid() {
    let (dir, mut engine, _now) = resumed(&["succeed"], base() + time::Duration::seconds(1));
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, "not json").unwrap();
    engine.cooldown.host = Some(invalid);
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty());
    let reason = park_reason(&engine);
    assert!(reason.contains("without reading its bounds"), "{reason}");
    assert!(reason.contains("is not a valid"), "{reason}");
}

#[test]
fn a_run_with_no_cool_down_declared_retries_and_names_the_absence_at_the_cap() {
    let (dir, mut engine, _now) = seated!(&["refuse", "refuse"], Probed::Answer, None, 2);
    let file = v1_host(dir.path());
    engine.cooldown.host = Some(file.clone());
    engine.drive().unwrap();
    // (a): today's path exactly — two attempts and no probe.
    assert_eq!(
        logs(dir.path(), "work"),
        ["attempt:refuse", "attempt:refuse"]
    );
    let reason = park_reason(&engine);
    assert!(
        reason.contains(&format!(
            "no cool-down is declared for claude/first in {}",
            file.display()
        )),
        "{reason}"
    );
}

#[test]
fn a_driver_that_does_not_advertise_the_probe_parks_naming_it() {
    let (dir, mut engine, _now) = seated!(&["refuse", "succeed"], Probed::Unadvertised, None, 2);
    engine.drive().unwrap();
    // The transport refuses the probe before any `start`, so the driver's
    // log names only the refused attempt.
    assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
    let reason = park_reason(&engine);
    assert!(reason.contains("does not advertise the probe"), "{reason}");
}

fn no_waits(engine: &Engine) {
    for event in failed(engine) {
        assert!(
            event.payload.get("waits").is_none(),
            "4c2 owns this wait: {}",
            event.payload
        );
    }
}

fn panel_body(command: Vec<String>) -> SeatBody {
    SeatBody::Invocation(Invocation::Panel {
        members: vec![PanelMember {
            name: "a".into(),
            role_path: PathBuf::from("role.md"),
            command,
            candidates: Vec::new(),
            written: Vec::new(),
            class: SeatClass::Work,
        }],
        aggregate: Aggregate::UnanimousPass,
    })
}

#[test]
fn a_panel_refused_by_a_rate_limit_follows_todays_path() {
    let dir = tempfile::tempdir().unwrap();
    let body = panel_body(driver(
        dir.path(),
        "panel",
        &["refuse", "refuse"],
        Probed::Answer,
        None,
        "claude",
        "first",
    ));
    let (_dir, mut engine, _now) = seated!(@in dir, body, 2);
    engine.drive().unwrap();
    no_waits(&engine);
    assert!(starts(&engine)
        .iter()
        .all(|start| start.get("probe").is_none()));
    assert!(
        park_reason(&engine).contains("failed 2 of 2 attempt(s)"),
        "{}",
        park_reason(&engine)
    );
}

#[test]
fn a_sequence_refused_by_a_rate_limit_follows_todays_path() {
    let dir = tempfile::tempdir().unwrap();
    let body = SeatBody::Sequence {
        steps: vec![uncompiled_sequence_step! {
            name: "draft".into(),
            class: SeatClass::Work,
            results: vec!["complete".into()],
            body: StepBody::Invocation(Invocation::Single {
                role_path: PathBuf::from("role.md"),
                command: driver(
                    dir.path(),
                    "step",
                    &["refuse", "refuse"],
                    Probed::Answer,
                    None,
                    "claude",
                    "first",
                ),
                candidates: Vec::new(),
                written: Vec::new(),
                class: (),
            }),
        }],
    };
    let (_dir, mut engine, _now) = seated!(@in dir, body, 2);
    engine.drive().unwrap();
    assert!(!failed(&engine).is_empty());
    no_waits(&engine);
    assert!(starts(&engine)
        .iter()
        .all(|start| start.get("probe").is_none()));
}

/// The host seam's three outcomes, and the two no-declaration shapes. A
/// file that cannot be read or is invalid parks; absence is no declaration.
#[test]
fn the_host_seam_reads_absent_unreadable_invalid_and_undeclared_files() {
    let dir = tempfile::tempdir().unwrap();
    let engine = started(dir.path(), single_body(vec!["unused".into()]), work(1));
    let mut engine = engine;
    // No file at the path.
    engine.cooldown.host = Some(dir.path().join("missing.json"));
    assert_eq!(engine.cooldown_bounds("claude", "first"), Ok(None));
    // A directory at the path cannot be read.
    engine.cooldown.host = Some(dir.path().to_path_buf());
    assert!(engine.cooldown_bounds("claude", "first").is_err());
    // Bytes that are not a host file.
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, "not json").unwrap();
    engine.cooldown.host = Some(invalid);
    assert!(engine.cooldown_bounds("claude", "first").is_err());
    // A v1 file, and a v2 file with no block: no declaration.
    engine.cooldown.host = Some(v1_host(dir.path()));
    assert_eq!(engine.cooldown_bounds("claude", "first"), Ok(None));
    // A declared block reads its numbers.
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    let bounds = engine.cooldown_bounds("claude", "first").unwrap().unwrap();
    assert_eq!(bounds.default_seconds.get(), 1);
    assert_eq!(bounds.max_seconds.get(), 10);
}

/// The episode sums each wait's own formula: the retry-after it recorded,
/// else the declared default doubled by its `rounds`.
#[test]
fn the_episode_sums_each_waits_own_round() {
    use crate::admission::Bounds;
    use crate::envelope_builder::EnvelopeBuilder;
    use std::num::NonZeroU32;
    let bounds = Bounds {
        default_seconds: NonZeroU32::new(2).unwrap(),
        max_seconds: NonZeroU32::new(10).unwrap(),
    };
    let event = |seq, effect, waits| {
        EnvelopeBuilder::new(
            EventType::EffectFailed,
            json!({"effect_id": effect, "waits": waits}),
        )
        .seq(seq)
        .build()
    };
    let wait = |rounds, retry: Option<u64>| {
        let mut wait = json!({"provider": "claude", "route": "first", "words": "w",
                              "until": "2026-11-01T00:00:00Z", "rounds": rounds});
        if let Some(retry) = retry {
            wait["retry_after_seconds"] = json!(retry);
        }
        wait
    };
    assert_eq!(
        super::episode(
            &[event(1, "effect", json!([wait(1, None)]))],
            "effect",
            bounds
        ),
        (1, 2)
    );
    assert_eq!(
        super::episode(
            &[event(1, "effect", json!([wait(2, None)]))],
            "effect",
            bounds
        ),
        (1, 4)
    );
    assert_eq!(
        super::episode(
            &[event(1, "effect", json!([wait(3, Some(5))]))],
            "effect",
            bounds
        ),
        (1, 5)
    );
    let skipped = EnvelopeBuilder::new(EventType::EffectStarted, json!({"effect_id": "effect"}))
        .seq(1)
        .build();
    let other = event(2, "other", json!([wait(1, None)]));
    let malformed = EnvelopeBuilder::new(
        EventType::EffectFailed,
        json!({"effect_id": "effect", "waits": [{"route": "first"}]}),
    )
    .seq(3)
    .build();
    assert_eq!(
        super::episode(&[skipped, other, malformed], "effect", bounds),
        (0, 0)
    );
}

fn wait() -> super::ProviderWait {
    super::ProviderWait {
        site: None,
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        until: base(),
        rounds: 1,
    }
}

fn probe_reason(engine: &Engine, report: brokkr_protocol::AttemptReport) -> String {
    match engine.probe_end(&wait(), report) {
        super::Probe::Parked(reason) => reason,
        super::Probe::Answered(_) => panic!("a probe that never answered is not an answer"),
    }
}

fn report(
    refusal: Option<Box<brokkr_protocol::PreSessionRefusal>>,
    cleanup: brokkr_protocol::Cleanup,
    checkpoints: Vec<Value>,
) -> brokkr_protocol::AttemptReport {
    brokkr_protocol::AttemptReport {
        outcome: brokkr_protocol::AttemptOutcome::Succeeded {
            result: json!({"result": "complete"}),
        },
        refused: None,
        refusal,
        cleanup,
        session_ref: None,
        checkpoints,
        stderr: String::new(),
        accepted: true,
        deadline_killed: false,
    }
}

#[test]
fn a_probe_that_does_not_answer_parks_naming_its_end() {
    let dir = tempfile::tempdir().unwrap();
    let engine = started(dir.path(), single_body(vec!["unused".into()]), work(1));
    let reason = probe_reason(
        &engine,
        report(
            None,
            brokkr_protocol::Cleanup::Unresolved {
                reason: brokkr_protocol::process::Unsettled::Group { group: 7 },
            },
            vec![json!({"step": "answer"})],
        ),
    );
    assert!(reason.contains("cleanup failure"), "{reason}");
    assert!(reason.contains("group 7"), "{reason}");
    let reason = probe_reason(
        &engine,
        report(None, brokkr_protocol::Cleanup::Settled, Vec::new()),
    );
    assert!(reason.contains("no answer checkpoint"), "{reason}");
    let other: brokkr_protocol::PreSessionRefusal = serde_json::from_value(json!({
        "class": "other", "provider": "claude", "route": "first", "words": "auth failed"
    }))
    .unwrap();
    let reason = probe_reason(
        &engine,
        report(
            Some(Box::new(other)),
            brokkr_protocol::Cleanup::Settled,
            Vec::new(),
        ),
    );
    assert!(reason.contains("was refused: auth failed"), "{reason}");
}

#[test]
fn a_wait_recorded_at_a_panel_refuses_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let body = panel_body(vec!["sh".into(), "-c".into(), "read -r hello".into()]);
    let engine = journaled(dir.path(), body, wait_value(base()));
    let (mut engine, _now) = with_clock_at(engine, base());
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("not a single seat"), "{reason}");
}

#[test]
fn a_probe_that_does_not_spawn_parks() {
    let dir = tempfile::tempdir().unwrap();
    let engine = journaled(
        dir.path(),
        single_body(vec!["missing-driver".into()]),
        wait_value(base()),
    );
    let (mut engine, _now) = with_clock_at(engine, base());
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("did not spawn"), "{reason}");
}

#[test]
fn an_unresolved_route_or_an_invalid_host_parks_before_any_wait() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    let refusal = |route: Value| -> brokkr_protocol::PreSessionRefusal {
        serde_json::from_value(json!({
            "class": "rate_limit", "provider": "claude", "route": route, "words": "rate_limit"
        }))
        .unwrap()
    };
    match engine
        .rate_limit_terminal(
            "effect",
            json!({"error": "refused"}),
            Some(&refusal(Value::Null)),
        )
        .unwrap()
    {
        super::Terminal::Park { reason, .. } => {
            assert!(reason.contains("route could not be resolved"), "{reason}");
        }
        super::Terminal::Plain(_) => panic!("an unresolved route cannot be recorded"),
    }
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, "not json").unwrap();
    engine.cooldown.host = Some(invalid);
    match engine
        .rate_limit_terminal(
            "effect",
            json!({"error": "refused"}),
            Some(&refusal(json!("first"))),
        )
        .unwrap()
    {
        super::Terminal::Park { reason, .. } => {
            assert!(reason.contains("is not a valid"), "{reason}");
        }
        super::Terminal::Plain(_) => panic!("an invalid host cannot be read as a constant"),
    }
}

#[test]
fn one_seat_has_one_wait_and_no_host_spells_its_own_absence() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(1));
    let wait = json!({"provider": "claude", "route": "first", "words": "w",
                      "until": "2026-11-01T00:00:00Z", "rounds": 1});
    let event = crate::envelope_builder::EnvelopeBuilder::new(
        EventType::EffectFailed,
        json!({"effect_id": "effect", "waits": [wait, wait]}),
    )
    .build();
    let error = engine.recorded_wait(&[event], "effect").unwrap_err();
    assert!(error.contains("at most one wait"), "{error}");
    assert_eq!(engine.host_spelling(), "no host file");
    let file = dir.path().join("host.json");
    engine.cooldown.host = Some(file.clone());
    assert_eq!(engine.host_spelling(), file.display().to_string());
}
