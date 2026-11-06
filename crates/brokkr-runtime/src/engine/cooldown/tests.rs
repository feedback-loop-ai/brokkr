//! Decision 0103 slice 4c1: one seat waits out a provider's rate limit.
//! Each test plants the fake driver's reports and the host file, drives the
//! run, and reads the events, the park reason or the attempt count.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use brokkr_core::canonical::sha256_hex;
use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Status};
use brokkr_protocol::hands::HandsSpec;
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

/// A v2 host file declaring cool-downs for claude/first and codex/second,
/// with `cooldowns` as the block's own JSON.
fn host_with(dir: &Path, name: &str, cooldowns: Value) -> PathBuf {
    let path = dir.join(name);
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
            "cooldowns": cooldowns
        })
        .to_string(),
    )
    .unwrap();
    path
}

/// A v2 host file with one block-wide default and maximum.
fn host(dir: &Path, default: u64, max: u64) -> PathBuf {
    host_with(
        dir,
        "host.json",
        json!({"default_seconds": default, "max_seconds": max}),
    )
}

/// Point `engine` at a host file that is present but is not JSON.
fn invalid_host(dir: &Path, engine: &mut Engine) {
    let invalid = dir.join("invalid.json");
    std::fs::write(&invalid, "not json").unwrap();
    engine.cooldown.host = Some(invalid);
}

/// A v2 host file whose bounds differ per route: the block's default 1
/// and maximum 25, claude/first defaults to 10s, codex/second must not pass
/// 5s (decision 0103 ruling 3: each wait keeps the length its own route
/// computed).
fn routes_host(dir: &Path) -> PathBuf {
    host_with(
        dir,
        "routes.json",
        json!({
            "default_seconds": 1,
            "max_seconds": 25,
            "providers": {
                "claude": {"routes": {"first": {"default_seconds": 10}}},
                "codex": {"routes": {"second": {"max_seconds": 5}}}
            }
        }),
    )
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
    /// The probe writes its started marker, waits for its release marker,
    /// then answers: the lease test holds it open while it claims.
    Barrier,
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
        Probed::Barrier => format!(
            "{accepted}\n  : > '@STARTED@'\n  i=0\n  while [ ! -e '@RELEASE@' ] && \
             [ $i -lt 600 ]; do sleep 0.05; i=$((i+1)); done\n  {checkpoint}\n  {succeeded}"
        ),
    };
    let started = dir.join(format!("{tag}.started"));
    let release = dir.join(format!("{tag}.release"));
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
    seat=$(printf '%s' "$start" | grep -o '"seat":"[^"]*"' | head -1 | cut -d'"' -f4)
    printf '%s' "$seat" > '@SEATLOG@'
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
        (
            "@SEATLOG@",
            dir.join(format!("{tag}.seat")).display().to_string(),
        ),
        ("@VERDICTS@", verdicts_path.display().to_string()),
        ("@PROBE@", probe),
        ("@REFUSED@", refused),
        ("@SUCCEEDED@", succeeded.to_string()),
        ("@STARTED@", started.display().to_string()),
        ("@RELEASE@", release.display().to_string()),
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

/// Where a test clock's one action runs: after its first slice, or on its
/// first read at or after an instant.
#[derive(Clone, Copy)]
enum Acting {
    AfterFirstSlice,
    At(OffsetDateTime),
}

/// The one action a test's clock runs, taken when it runs.
type OneAction = Arc<Mutex<Option<Box<dyn FnMut() + Send>>>>;

/// A clock that advances the shared instant on each slice and runs `act`
/// once, where `when` says.
fn clock_acting(
    now: &Arc<Mutex<OffsetDateTime>>,
    when: Acting,
    act: impl FnMut() + Send + 'static,
) -> Clock {
    let read = Arc::clone(now);
    let advance = Arc::clone(now);
    let act: OneAction = Arc::new(Mutex::new(Some(Box::new(act))));
    let act_read = Arc::clone(&act);
    let at = match when {
        Acting::At(at) => Some(at),
        Acting::AfterFirstSlice => None,
    };
    Clock {
        now: Box::new(move || {
            let at_now = *read.lock().unwrap();
            if at.is_some_and(|at| at_now >= at) {
                if let Some(mut act) = act_read.lock().unwrap().take() {
                    act();
                }
            }
            at_now
        }),
        sleep: Box::new(move |slice: std::time::Duration| {
            *advance.lock().unwrap() += time::Duration::seconds(slice.as_secs() as i64);
            if at.is_none() {
                if let Some(mut act) = act.lock().unwrap().take() {
                    act();
                }
            }
        }),
    }
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

/// The two-route chain the cross-route tests drive: claude/first then
/// codex/second, each with its own verdicts.
fn two_route_body(dir: &Path, first: &[&str], second: &[&str]) -> SeatBody {
    let claude = driver(
        dir,
        "claude",
        first,
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let codex = driver(
        dir,
        "codex",
        second,
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    chain(vec![
        candidate("claude", "claude/first", claude),
        candidate("codex", "codex/second", codex),
    ])
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
    let body = two_route_body(dir.path(), &["refuse"], &["succeed"]);
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
            json!([{"route": "first", "words": "w", "seconds": 1,
                    "until": "2026-11-01T00:00:00Z", "rounds": 1}]),
        );
        engine.drive().unwrap();
        let reason = park_reason(&engine);
        assert!(reason.contains("does not read"), "{reason}");
        assert!(reason.contains("provider"), "{reason}");
    }
}

/// Journal an operator's accepted stop on the run.
fn append_stop(db: &Path, run_id: &str) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .append_next(
            run_id,
            EventType::OperatorCommanded,
            json!({"command_id": "c", "command": "stop", "args": {},
                   "operator": "operator"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            run_id,
            EventType::OperatorAccepted,
            json!({"command_id": "c", "operator": "operator", "reason": "stop"}),
            None,
            None,
        )
        .unwrap();
}

/// A seat whose refused attempt waits two seconds under a declared host.
fn stop_fixture() -> (
    tempfile::TempDir,
    Engine,
    Arc<Mutex<OffsetDateTime>>,
    PathBuf,
    String,
) {
    let dir = tempfile::tempdir().unwrap();
    let command = standard_command(dir.path());
    let mut engine = started(dir.path(), single_body(command), work(2));
    engine.cooldown.host = Some(host(dir.path(), 2, 10));
    let (engine, now) = with_clock(engine);
    let db = dir.path().join("forge.db");
    let run_id = engine.run_id.clone();
    (dir, engine, now, db, run_id)
}

fn assert_stopped(dir: &Path, engine: &Engine) {
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    assert_eq!(logs(dir, "work"), ["attempt:refuse"]);
    assert_eq!(starts(engine).len(), 1);
}

#[test]
fn an_operator_stop_during_the_wait_is_honoured() {
    let (dir, mut engine, now, db, run_id) = stop_fixture();
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        append_stop(&db, &run_id)
    });
    engine.drive().unwrap();
    assert_stopped(dir.path(), &engine);
}

/// F3: the operator's accepted stop lands after the caller folded
/// `ExecuteEffect` but before the wait's first sample. It must be seen
/// before the first slice, so the probe never runs and the fenced retry is
/// never attempted.
#[test]
fn a_stop_before_the_first_sample_stops_without_a_probe() {
    let (dir, mut engine, now, db, run_id) = stop_fixture();
    engine.cooldown.clock =
        clock_acting(&now, Acting::At(base()), move || append_stop(&db, &run_id));
    engine.drive().unwrap();
    assert_stopped(dir.path(), &engine);
}

/// The stop check before the first slice: the caller folded
/// `ExecuteEffect`, the operator's accepted stop landed, and `take_cooldown`
/// is handed the stale fold. It sees the stop before sleeping or probing.
#[test]
fn a_stop_already_journaled_is_seen_before_the_first_slice() {
    let (dir, mut engine, now) = resumed(&["succeed"], base() + time::Duration::seconds(2), 2);
    let events = engine.store.load(&engine.run_id).unwrap();
    let state = fold(&events).unwrap();
    let wait = engine.recorded_wait(&events, "effect").unwrap().unwrap();
    append_stop(&dir.path().join("forge.db"), &engine.run_id);
    let before = *now.lock().unwrap();
    match engine
        .take_cooldown(&events, &state, "effect", "work", &wait)
        .unwrap()
    {
        super::Waited::Stopped => {}
        super::Waited::Proceed(_) => panic!("a stopped run proceeded to the probe"),
        super::Waited::Parked(reason) => panic!("a stopped run parked: {reason}"),
    }
    assert_eq!(
        *now.lock().unwrap(),
        before,
        "a journaled stop must be seen before the first slice"
    );
    assert!(logs(dir.path(), "work").is_empty(), "no probe may run");
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

/// F4: a retry-after too large for a nanosecond `until` must not panic.
/// The maximum is judged from the computed seconds first, the failed event
/// is written without a wait, and the run parks naming the retry-after and
/// the maximum.
#[test]
fn a_huge_retry_after_parks_without_a_panic_or_a_wait() {
    for retry in [1_000_000_000_000u64, i64::MAX as u64] {
        let (dir, mut engine, _now) = seated!(&["refuse"], Probed::Answer, Some(retry), 2);
        engine.drive().unwrap();
        assert_eq!(logs(dir.path(), "work"), ["attempt:refuse"]);
        assert_eq!(starts(&engine).len(), 1);
        let failed = failed(&engine);
        assert_eq!(failed.len(), 1, "the attempt's failed event was written");
        assert!(
            failed[0].payload.get("waits").is_none(),
            "no wait is recorded when no until can be represented: {}",
            failed[0].payload
        );
        let reason = park_reason(&engine);
        assert!(reason.contains(&retry.to_string()), "{reason}");
        assert!(reason.contains("declared 10s maximum"), "{reason}");
        assert!(reason.contains("past the declared 10s maximum"), "{reason}");
        assert!(reason.contains("rate_limit: slow down"), "{reason}");
    }
}

/// F5: the second refusal brings the two-attempt cap, and its episode is
/// also past the 2s maximum. The cap's park is the one written, with 0006's
/// text first and the provider, route, until, words and the passed maximum
/// beside it; the episode's own park never speaks.
#[test]
fn the_cap_reason_speaks_first_and_names_the_passed_maximum() {
    let (dir, mut engine, _now) = seated!(&["refuse", "refuse"], Probed::Answer, None, 2);
    engine.cooldown.host = Some(host(dir.path(), 1, 2));
    drove(
        &mut engine,
        dir.path(),
        &["attempt:refuse", "probe", "attempt:refuse"],
    );
    let reason = park_reason(&engine);
    assert!(reason.starts_with("effect "), "{reason}");
    assert!(reason.contains("failed 2 of 2 attempt(s)"), "{reason}");
    assert!(reason.contains("waits for claude/first until"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert!(
        reason.contains("episode 3s is past the declared 2s maximum"),
        "{reason}"
    );
    assert!(!reason.starts_with("cool-down for"), "{reason}");
    assert_eq!(starts(&engine).len(), 2);
}

/// M1: at the cap, a refusal whose `until` cannot be represented must not
/// lose its facts. The cap's 0006 text speaks first, then the provider,
/// route, retry-after, words and the passed maximum from the failed
/// event's `refused` record, saying plainly that no `until` exists.
#[test]
fn a_no_until_cap_names_the_refusal_facts() {
    let (dir, mut engine, _now) = seated!(&["refuse"], Probed::Answer, Some(1_000_000_000_000), 1);
    drove(&mut engine, dir.path(), &["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert!(reason.starts_with("effect "), "{reason}");
    assert!(reason.contains("failed 1 of 1 attempt(s)"), "{reason}");
    assert!(reason.contains("claude/first"), "{reason}");
    assert!(reason.contains("1000000000000"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert!(reason.contains("the declared maximum was 10s"), "{reason}");
    assert!(reason.contains("no until"), "{reason}");
    // The failed event carries the record the cap read, and invents no wait.
    let failed = failed(&engine);
    assert_eq!(failed.len(), 1, "one attempt's failure");
    assert_eq!(failed[0].payload["refused"]["cause"], "no-end");
    assert_eq!(failed[0].payload["refused"]["max_seconds"], 10);
    no_waits(&engine);
}

/// A valid wait entry array ending at `until`, carrying the length the
/// entry recorded under its own route's bounds.
fn wait_value(until: OffsetDateTime, seconds: u64) -> Value {
    json!([{"provider": "claude", "route": "first",
            "words": "rate_limit: slow down", "seconds": seconds,
            "until": until.format(&Rfc3339).unwrap(), "rounds": 1}])
}

/// Journal the phase and the effect's request, so a test can write the
/// attempts and the terminal itself.
fn request_effect(engine: &mut Engine) {
    engine
        .append(EventType::PhaseEntered, json!({"phase": "work"}), None)
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let state = fold(&events).unwrap();
    let digest = sha256_hex(&engine.seat_input(&state, "work", "effect").unwrap());
    engine
        .append(
            EventType::EffectRequested,
            json!({"effect_id": "effect", "phase": "work", "seat": "work",
                   "idempotency_key": "k", "input_digest": digest}),
            None,
        )
        .unwrap();
}

/// The journal a crashed driver leaves behind: a requested effect, one
/// started attempt and the terminal payload the test spells.
fn append_failed_payload(engine: &mut Engine, payload: Value) {
    request_effect(engine);
    for (event, payload, attempt) in [
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "a1", "driver": "d"}),
            Some("a1".to_string()),
        ),
        (EventType::EffectFailed, payload, Some("a1".to_string())),
    ] {
        engine.append(event, payload, attempt).unwrap();
    }
}

/// An engine at the start of `effect`, with the answering driver a resume
/// would invoke and `attempts` allowed.
fn engine_for(dir: &Path, attempts: u64) -> (Engine, crate::bundle::Bundle) {
    let command = driver(
        dir,
        "work",
        &["succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let mut compiled = bundle(dir, single_body(command));
    compiled.seats.get_mut("work").unwrap().limits = work(attempts);
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    let engine = Engine::start(store, compiled.clone(), "Feature", Some(dir.join("work"))).unwrap();
    (engine, compiled)
}

/// Resume the journal at `dir` under `compiled`.
fn resume_engine(dir: &Path, compiled: crate::bundle::Bundle, run_id: &str) -> Engine {
    Engine::resume(
        brokkr_store::Store::open(&dir.join("forge.db")).unwrap(),
        compiled,
        run_id,
        Some(dir.join("work")),
    )
    .unwrap()
}

/// The journal a crashed driver leaves behind with `waits` on its terminal
/// failure.
fn append_waited(engine: &mut Engine, waits: Value) {
    append_failed_payload(
        engine,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "start_failure": true, "start_failure_sites": [null],
               "waits": waits}),
    );
}

fn resumed(
    verdicts: &[&str],
    until: OffsetDateTime,
    seconds: u64,
) -> (tempfile::TempDir, Engine, Arc<Mutex<OffsetDateTime>>) {
    resumed_at(verdicts, until, seconds, base(), false)
}

fn resumed_at(
    verdicts: &[&str],
    until: OffsetDateTime,
    seconds: u64,
    at: OffsetDateTime,
    boxed: bool,
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
    let mut compiled = compiled(dir.path(), body.clone());
    if boxed {
        crate::engine::tests::set_site_hands(&mut compiled, "work", HandsSpec::default());
    }
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let store = brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine = Engine::start(
        store,
        compiled.clone(),
        "Feature",
        Some(dir.path().join("work")),
    )
    .unwrap();
    append_waited(&mut engine, wait_value(until, seconds));
    let run_id = engine.run_id.clone();
    drop(engine);
    let mut resumed_engine = Engine::resume(
        brokkr_store::Store::open(&dir.path().join("forge.db")).unwrap(),
        compiled,
        &run_id,
        Some(dir.path().join("work")),
    )
    .unwrap();
    let (clock, now) = Clock::advancing(at);
    resumed_engine.cooldown.clock = clock;
    (dir, resumed_engine, now)
}

/// A typed rate-limit refusal the driver could have reported, with an
/// optional resolved route and retry-after.
fn a_rate_limit_with(
    route: Option<&str>,
    retry: Option<u64>,
) -> brokkr_protocol::PreSessionRefusal {
    let mut value = json!({
        "class": "rate_limit", "provider": "claude", "route": route,
        "words": "rate_limit: slow down"
    });
    if let Some(retry) = retry {
        value["retry_after_seconds"] = json!(retry);
    }
    serde_json::from_value(value).unwrap()
}

/// The engine's own terminal decision, still in the process that made it.
/// `earlier` plants a prior failed event, for the episode-unreadable cause;
/// the caller decides whether its park is journaled.
fn terminal_event(
    dir: &Path,
    attempts: u64,
    host: Option<PathBuf>,
    refusal: &brokkr_protocol::PreSessionRefusal,
    earlier: Option<Value>,
) -> (Engine, crate::bundle::Bundle, super::Terminal) {
    let (mut engine, compiled) = engine_for(dir, attempts);
    engine.cooldown.host = host;
    request_effect(&mut engine);
    if let Some(earlier) = earlier {
        engine
            .append(
                EventType::EffectStarted,
                json!({"effect_id": "effect", "attempt_id": "a0", "driver": "d"}),
                Some("a0".to_string()),
            )
            .unwrap();
        engine
            .append(EventType::EffectFailed, earlier, Some("a0".to_string()))
            .unwrap();
    }
    engine
        .append(
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "a1", "driver": "d"}),
            Some("a1".to_string()),
        )
        .unwrap();
    let terminal = engine
        .rate_limit_terminal(
            "effect",
            json!({"effect_id": "effect", "attempt_id": "a1",
                   "error": "provider refused", "start_failure": true,
                   "start_failure_sites": [null]}),
            Some(refusal),
        )
        .unwrap();
    (engine, compiled, terminal)
}

/// Which half of a rate-limit terminal a journal stands in: the live
/// process, which journals the park its terminal decided, or a crash that
/// left only the failed event and is resumed after it.
#[derive(Clone, Copy)]
enum TerminalMode {
    Live,
    Crashed,
}

/// The journal a terminal leaves in either half. A live journal is written
/// exactly as the engine writes it; a crashed one keeps only the failed
/// payload and is resumed again after the crash.
fn terminal_case(
    dir: &Path,
    attempts: u64,
    host: Option<PathBuf>,
    refusal: &brokkr_protocol::PreSessionRefusal,
    earlier: Option<Value>,
    mode: TerminalMode,
) -> (Engine, Arc<Mutex<OffsetDateTime>>) {
    let (mut engine, compiled, terminal) =
        terminal_event(dir, attempts, host.clone(), refusal, earlier);
    match mode {
        TerminalMode::Live => {
            terminal.journal(&mut engine, "a1").unwrap();
            let (clock, now) = Clock::advancing(base());
            engine.cooldown.clock = clock;
            (engine, now)
        }
        TerminalMode::Crashed => {
            let payload = match terminal {
                super::Terminal::Plain(payload) | super::Terminal::Park { payload, .. } => payload,
            };
            engine
                .append(EventType::EffectFailed, payload, Some("a1".to_string()))
                .unwrap();
            let run_id = engine.run_id.clone();
            drop(engine);
            let mut resumed = resume_engine(dir, compiled, &run_id);
            resumed.cooldown.host = host;
            let (clock, now) = Clock::advancing(base() + time::Duration::seconds(2));
            resumed.cooldown.clock = clock;
            (resumed, now)
        }
    }
}

/// Drive a crash-before-the-park journal and require the resume to park on
/// the record's facts with no wait, probe or driver invocation.
fn assert_crash_parks(
    dir: &Path,
    attempts: u64,
    host: Option<PathBuf>,
    refusal: &brokkr_protocol::PreSessionRefusal,
    earlier: Option<Value>,
    expected: &[&str],
) {
    let expected_starts = 1 + usize::from(earlier.is_some());
    let (mut engine, _now) =
        terminal_case(dir, attempts, host, refusal, earlier, TerminalMode::Crashed);
    engine.drive().unwrap();
    assert!(
        logs(dir, "work").is_empty(),
        "a crash-before-the-park resume invoked the driver: {:?}",
        logs(dir, "work")
    );
    assert_eq!(
        starts(&engine).len(),
        expected_starts,
        "the resume started an attempt"
    );
    let reason = park_reason(&engine);
    for fact in expected {
        assert!(reason.contains(fact), "{reason} omits {fact}");
    }
}

/// H1: the failed event records the refusal it parks on, so a crash before
/// the separate `run/parked` does not turn a maximum refusal into a retry.
/// This is the no-end cause: the journal cannot spell the `until`.
#[test]
fn a_no_until_crash_resumes_the_maximum_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), Some(1_000_000_000_000));
    assert_crash_parks(
        dir.path(),
        2,
        Some(host(dir.path(), 1, 10)),
        &refusal,
        None,
        &[
            "has no representable end",
            "declared 10s maximum",
            "rate_limit: slow down",
        ],
    );
}

#[test]
fn a_crash_before_a_route_unresolved_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(None, None);
    assert_crash_parks(
        dir.path(),
        2,
        None,
        &refusal,
        None,
        &["route could not be resolved", "rate_limit: slow down"],
    );
}

/// A cap reached after a crash keeps a refusal whose route never resolved:
/// the cap states the record's facts without inventing a length or `until`.
#[test]
fn a_cap_after_a_crash_keeps_a_route_unresolved_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(None, Some(7));
    let (mut engine, _now) =
        terminal_case(dir.path(), 1, None, &refusal, None, TerminalMode::Crashed);
    drove(&mut engine, dir.path(), &[]);
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert!(reason.starts_with("effect "), "{reason}");
    assert!(reason.contains("claude/an unresolved route"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert!(reason.contains("retry-after 7s"), "{reason}");
    assert!(reason.contains("no until"), "{reason}");
}

#[test]
fn a_crash_before_a_host_unreadable_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), None);
    assert_crash_parks(
        dir.path(),
        2,
        Some(dir.path().to_path_buf()),
        &refusal,
        None,
        &[
            "claude/first",
            "host configuration could not be read",
            "rate_limit: slow down",
        ],
    );
}

#[test]
fn a_crash_before_an_episode_unreadable_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), None);
    assert_crash_parks(
        dir.path(),
        3,
        Some(host(dir.path(), 1, 10)),
        &refusal,
        Some(
            json!({"effect_id": "effect", "attempt_id": "a0", "error": "refused",
                    "start_failure": true, "start_failure_sites": [null],
                    "waits": malformed_earlier()}),
        ),
        &[
            "claude/first",
            "an earlier wait does not read",
            "rate_limit: slow down",
        ],
    );
}

#[test]
fn a_crash_before_a_past_maximum_park_resumes_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let refusal = a_rate_limit_with(Some("first"), Some(5));
    assert_crash_parks(
        dir.path(),
        2,
        Some(host(dir.path(), 1, 2)),
        &refusal,
        None,
        &[
            "would wait 5s",
            "episode 5s is past the declared 2s maximum",
            "rate_limit: slow down",
        ],
    );
}

/// A resumed engine whose journal holds one failed payload the test
/// spells, with `attempts` allowed.
fn resumed_payload(dir: &Path, attempts: u64, payload: Value) -> Engine {
    let (mut engine, compiled) = engine_for(dir, attempts);
    append_failed_payload(&mut engine, payload);
    let run_id = engine.run_id.clone();
    drop(engine);
    let mut resumed = resume_engine(dir, compiled, &run_id);
    resumed.cooldown.clock = Clock::advancing(base() + time::Duration::seconds(2)).0;
    resumed
}

/// Resume a journal whose terminal payload the test spells, require no
/// driver to run, and return the park reason after checking `facts`.
fn assert_payload_parks(dir: &Path, attempts: u64, payload: Value, expected: &[&str]) -> String {
    let mut engine = resumed_payload(dir, attempts, payload);
    engine.drive().unwrap();
    assert!(logs(dir, "work").is_empty(), "a resume invoked the driver");
    let reason = park_reason(&engine);
    for fact in expected {
        assert!(reason.contains(fact), "{reason} omits {fact}");
    }
    reason
}

/// A present `refused` that does not read refuses by name, before any wait
/// or driver, at and below the cap.
#[test]
fn a_malformed_refusal_refuses_by_name_at_and_below_the_cap() {
    for (attempts, with_wait) in [(1u64, true), (2, true), (1, false)] {
        let dir = tempfile::tempdir().unwrap();
        let mut payload = json!({
            "effect_id": "effect", "attempt_id": "a1", "error": "refused",
            "start_failure": true, "start_failure_sites": [null],
            "refused": {"cause": "past-maximum", "provider": "claude",
                        "route": "first", "words": "rate_limit"},
        });
        if with_wait {
            payload["waits"] = wait_value(base(), 1);
        }
        let reason = assert_payload_parks(
            dir.path(),
            attempts,
            payload,
            &[
                "the failed event's refusal does not read",
                "does not read: seconds",
            ],
        );
        match attempts {
            1 => assert!(reason.starts_with("effect "), "{reason}"),
            _ => assert!(
                reason.contains("refusing to execute effect effect"),
                "{reason}"
            ),
        }
    }
}

/// A past-maximum record whose wait entry does not read still states the
/// maximum from the record, with no invented episode.
#[test]
fn a_past_maximum_record_with_no_readable_episode_states_the_maximum() {
    for attempts in [1u64, 2] {
        let dir = tempfile::tempdir().unwrap();
        let facts: &[&str] = match attempts {
            1 => &[
                "the computed wait was 5s; the declared maximum was 2s",
                "does not read",
                "rate_limit: slow down",
            ],
            _ => &[
                "would wait 5s",
                "past the declared 2s maximum",
                "rate_limit: slow down",
            ],
        };
        let reason = assert_payload_parks(
            dir.path(),
            attempts,
            json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
                   "start_failure": true, "start_failure_sites": [null],
                   "refused": {"cause": "past-maximum", "provider": "claude",
                               "route": "first", "words": "rate_limit: slow down",
                               "seconds": 5, "max_seconds": 2},
                   "waits": malformed_earlier()}),
            facts,
        );
        assert!(!reason.contains("episode"), "{reason}");
    }
}

#[test]
fn a_resume_waits_the_remainder() {
    let (dir, mut engine, now) = resumed(&["succeed"], base() + time::Duration::seconds(3), 3);
    // No declaration on the resume: the recorded end is the wait's.
    engine.cooldown.host = None;
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "work"), ["probe", "attempt:succeed"]);
    assert_eq!(*now.lock().unwrap(), base() + time::Duration::seconds(3));
    let started = starts(&engine);
    assert!(started.last().unwrap()["probe"].is_object());
}

#[test]
fn a_resume_parks_when_the_recorded_wait_passes_the_maximum() {
    let (dir, mut engine, _now) = resumed(&["succeed"], base() + time::Duration::seconds(5), 5);
    engine.cooldown.host = Some(host(dir.path(), 1, 1));
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty());
    let reason = park_reason(&engine);
    assert!(reason.contains("past the declared 1s maximum"), "{reason}");
}

/// F1: the failed event committed an 11-second wait under a 10s maximum,
/// and the engine crashed before its park. A resume at `at` applies the
/// recorded maximum; the remainder only sets how long the wait sleeps.
fn a_resume_past_the_recorded_maximum_is_refused(at: i64) {
    let until = base() + time::Duration::seconds(11);
    let (dir, mut engine, _now) = resumed_at(
        &["succeed"],
        until,
        11,
        base() + time::Duration::seconds(at),
        false,
    );
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty());
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert!(reason.contains("episode 11s"), "{reason}");
    assert!(reason.contains("declared 10s maximum"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
}

/// F1, early: at T+2 the remainder is 9s, below the maximum, but the
/// recorded 11s is past it.
#[test]
fn a_resume_early_keeps_the_recorded_maximum_refusal() {
    a_resume_past_the_recorded_maximum_is_refused(2);
}

/// F1, late: a resume past `until` applies the same recorded maximum, not
/// a zero remainder.
#[test]
fn a_resume_late_keeps_the_recorded_maximum_refusal() {
    a_resume_past_the_recorded_maximum_is_refused(12);
}

/// F2: claude/first recorded 10s under its own route's default, and
/// codex/second's 2s is computed under codex's default. The episode is 12s
/// and passes codex's 5s maximum, so codex is never probed or retried.
#[test]
fn an_episode_sums_each_routes_recorded_length() {
    let dir = tempfile::tempdir().unwrap();
    let body = two_route_body(dir.path(), &["refuse"], &["refuse", "succeed"]);
    let (dir, mut engine, _now) = seated!(@in dir, body, 3);
    engine.cooldown.host = Some(routes_host(dir.path()));
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "claude"), ["attempt:refuse", "probe"]);
    assert_eq!(logs(dir.path(), "codex"), ["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 2);
    let reason = park_reason(&engine);
    assert!(reason.contains("episode 12s"), "{reason}");
    assert!(reason.contains("declared 5s maximum"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
}

#[test]
fn a_resume_parks_when_the_host_file_is_invalid() {
    let (dir, mut engine, _now) = resumed(&["succeed"], base() + time::Duration::seconds(1), 1);
    invalid_host(dir.path(), &mut engine);
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

mod lease;

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
    assert!(matches!(
        engine.cooldown_bounds("claude", "first"),
        Ok(None)
    ));
    // A directory at the path cannot be read.
    engine.cooldown.host = Some(dir.path().to_path_buf());
    match engine.cooldown_bounds("claude", "first") {
        Err(super::CooldownError::HostUnreadable { path, kind }) => {
            assert_eq!(path, dir.path());
            assert_eq!(kind, std::io::ErrorKind::IsADirectory);
        }
        other => panic!("a directory read as {other:?}, not an unreadable host"),
    }
    // Bytes that are not a host file.
    let invalid = dir.path().join("invalid.json");
    std::fs::write(&invalid, "not json").unwrap();
    engine.cooldown.host = Some(invalid.clone());
    match engine.cooldown_bounds("claude", "first") {
        Err(super::CooldownError::HostInvalid { source }) => {
            assert!(source.to_string().contains("is not a valid"), "{source}");
        }
        other => panic!("invalid bytes read as {other:?}, not an invalid host"),
    }
    // A v1 file, and a v2 file with no block: no declaration.
    engine.cooldown.host = Some(v1_host(dir.path()));
    assert!(matches!(
        engine.cooldown_bounds("claude", "first"),
        Ok(None)
    ));
    // A declared block reads its numbers.
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    let bounds = engine.cooldown_bounds("claude", "first").unwrap().unwrap();
    assert_eq!(bounds.default_seconds.get(), 1);
    assert_eq!(bounds.max_seconds.get(), 10);
}

/// The episode sums each wait's own recorded `seconds`, never recomputing
/// an earlier wait under the current route's bounds; an earlier entry that
/// does not read refuses by name instead of counting as nothing (decision
/// 0103 ruling 3, this fix round).
#[test]
fn the_episode_sums_each_waits_own_round() {
    use crate::envelope_builder::EnvelopeBuilder;
    let event = |seq, effect, waits| {
        EnvelopeBuilder::new(
            EventType::EffectFailed,
            json!({"effect_id": effect, "waits": waits}),
        )
        .seq(seq)
        .build()
    };
    let wait = |seconds, rounds| {
        json!({"provider": "claude", "route": "first", "words": "w", "seconds": seconds,
               "until": "2026-11-01T00:00:00Z", "rounds": rounds})
    };
    assert_eq!(
        super::episode(&[event(1, "effect", json!([wait(10, 1)]))], "effect").unwrap(),
        (1, 10)
    );
    assert_eq!(
        super::episode(
            &[
                event(1, "effect", json!([wait(10, 1)])),
                event(2, "effect", json!([wait(2, 2)])),
            ],
            "effect"
        )
        .unwrap(),
        (2, 12)
    );
    // A terminal with no waits counts nothing, and is not a malformed one.
    assert_eq!(
        super::episode(
            &[
                EnvelopeBuilder::new(EventType::EffectFailed, json!({"effect_id": "effect"}),)
                    .seq(5)
                    .build()
            ],
            "effect"
        )
        .unwrap(),
        (0, 0)
    );
    let skipped = EnvelopeBuilder::new(EventType::EffectStarted, json!({"effect_id": "effect"}))
        .seq(1)
        .build();
    let other = event(2, "other", json!([wait(4, 1)]));
    let malformed = event(
        3,
        "effect",
        json!([{"route": "first", "words": "w", "seconds": 1,
                "until": "2026-11-01T00:00:00Z", "rounds": 1}]),
    );
    match super::episode(&[skipped, other, malformed], "effect") {
        Err(super::CooldownError::EarlierWaitUnreadable {
            seq: 3,
            source: brokkr_core::ProviderWaitError::Field { field, .. },
        }) if field == "provider" => {}
        Err(other) => panic!("the earlier wait refused as {other:?}, not by its provider"),
        Ok(episode) => panic!("an unreadable earlier wait counted as {episode:?}"),
    }
}

/// F6: an earlier same-effect failed event whose wait does not read refuses
/// by name at the cap too, with the event's seq and the read error, instead
/// of counting as zero rounds and zero seconds.
#[test]
fn an_unreadable_earlier_wait_parks_naming_its_event() {
    let (_dir, mut engine) = two_terminal_engine(2);
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("failed 2 of 2 attempt(s)"), "{reason}");
    assert!(reason.contains("an earlier wait does not read"), "{reason}");
    assert!(reason.contains("does not read: provider"), "{reason}");
}

/// The journal two terminal events leave: `earlier` (which may be
/// malformed for the episode) and a readable second terminal.
fn two_terminals(engine: &mut Engine, earlier: Value) {
    append_waited(engine, earlier);
    for (event, payload, attempt) in [
        (
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "a2", "driver": "d"}),
            Some("a2".to_string()),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "a2", "error": "refused",
                   "start_failure": true, "start_failure_sites": [null],
                   "waits": wait_value(base() + time::Duration::seconds(2), 2)}),
            Some("a2".to_string()),
        ),
    ] {
        engine.append(event, payload, attempt).unwrap();
    }
}

fn a_rate_limit(retry: Option<u64>) -> brokkr_protocol::PreSessionRefusal {
    a_rate_limit_with(Some("first"), retry)
}

/// An earlier terminal whose `waits` entry is missing its provider.
fn malformed_earlier() -> Value {
    json!([{"route": "first", "words": "w", "seconds": 1,
            "until": "2026-11-01T00:00:01Z", "rounds": 1}])
}

/// A run whose journal holds the malformed earlier terminal and a readable
/// second one, with the host declared and `attempts` allowed.
fn two_terminal_engine(attempts: u64) -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(
        dir.path(),
        single_body(vec!["unused".into()]),
        work(attempts),
    );
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    two_terminals(&mut engine, malformed_earlier());
    (dir, engine)
}

/// The terminal a rate-limit refusal writes, through the one public door.
fn terminal(engine: &mut Engine, retry: Option<u64>) -> super::Terminal {
    engine
        .rate_limit_terminal(
            "effect",
            json!({"error": "refused"}),
            Some(&a_rate_limit(retry)),
        )
        .unwrap()
}

/// The resume path reads its episode the same way the terminal does: an
/// earlier entry that does not read refuses, never as zero rounds.
#[test]
fn a_resume_refuses_an_unreadable_earlier_wait() {
    let (dir, mut engine) = two_terminal_engine(3);
    engine.drive().unwrap();
    assert!(logs(dir.path(), "work").is_empty());
    let reason = park_reason(&engine);
    assert!(reason.contains("without reading its episode"), "{reason}");
    assert!(reason.contains("an earlier wait does not read"), "{reason}");
}

/// The terminal path refuses an unreadable earlier entry by name, and
/// writes the failed event it was called with.
#[test]
fn a_terminal_refuses_an_unreadable_earlier_wait() {
    let (_dir, mut engine) = two_terminal_engine(4);
    match terminal(&mut engine, None) {
        super::Terminal::Park { reason, .. } => {
            assert!(
                reason.contains("refusing to retry claude/first"),
                "{reason}"
            );
            assert!(reason.contains("an earlier wait does not read"), "{reason}");
        }
        super::Terminal::Plain(_) => panic!("an unreadable earlier wait was read as nothing"),
    }
}

/// A journal that requested no seat reaches the cap's default: one attempt,
/// so a terminal below no declaration is plain and carries its wait.
#[test]
fn a_terminal_without_a_requested_seat_reads_the_default_cap() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(2));
    engine.cooldown.host = Some(host(dir.path(), 1, 10));
    match terminal(&mut engine, None) {
        super::Terminal::Plain(payload) => {
            assert!(payload["waits"].is_array(), "{payload}");
        }
        super::Terminal::Park { reason, .. } => panic!("the default cap parked: {reason}"),
    }
}

/// A capped wait with no declared bounds reports no maximum, and one whose
/// bounds cannot be read reports that refusal (the cap's reason carries
/// whichever).
#[test]
fn a_capped_wait_reports_no_maximum_or_its_unreadable_bounds() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(1));
    let events = [crate::envelope_builder::EnvelopeBuilder::new(
        EventType::EffectFailed,
        json!({"effect_id": "effect", "waits": wait_value(base(), 1)}),
    )
    .build()];
    let wait = engine.recorded_wait(&events, "effect").unwrap().unwrap();
    assert_eq!(engine.wait_maximum(&events, "effect", &wait).unwrap(), None);
    engine.cooldown.host = Some(dir.path().to_path_buf());
    match engine.wait_maximum(&events, "effect", &wait) {
        Err(super::CooldownError::HostUnreadable { .. }) => {}
        other => panic!("unreadable bounds read as {other:?}"),
    }
}

/// A wait doubled past the representable `until` with no provider
/// retry-after parks naming the computed length, the maximum and the words.
#[test]
fn a_doubled_wait_with_no_representable_end_parks() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = started(dir.path(), single_body(vec!["unused".into()]), work(3));
    engine.cooldown.host = Some(host(dir.path(), 7_200, 7_200));
    let waits: Vec<Value> = (0..33)
        .map(|_| {
            json!({"provider": "claude", "route": "first", "words": "w", "seconds": 1,
                   "until": "2026-11-01T00:00:01Z", "rounds": 1})
        })
        .collect();
    append_waited(&mut engine, json!(waits));
    match terminal(&mut engine, None) {
        super::Terminal::Park { reason, .. } => {
            assert!(reason.contains("has no representable end"), "{reason}");
            assert!(reason.contains("7200s maximum"), "{reason}");
            assert!(reason.contains("rate_limit: slow down"), "{reason}");
        }
        super::Terminal::Plain(_) => panic!("a wait with no end was recorded as a wait"),
    }
}

fn wait() -> super::ProviderWait {
    super::ProviderWait {
        site: None,
        provider: "claude".into(),
        route: "first".into(),
        words: "rate_limit: slow down".into(),
        retry_after_seconds: None,
        seconds: 1,
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
    let engine = journaled(dir.path(), body, wait_value(base(), 1));
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
        wait_value(base(), 1),
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
    request_effect(&mut engine);
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
                      "seconds": 1, "until": "2026-11-01T00:00:00Z", "rounds": 1});
    let event = crate::envelope_builder::EnvelopeBuilder::new(
        EventType::EffectFailed,
        json!({"effect_id": "effect", "waits": [wait, wait]}),
    )
    .seq(4)
    .build();
    match engine.recorded_wait(&[event], "effect") {
        Err(super::CooldownError::TooManyWaits { seq, count }) => {
            assert_eq!((seq, count), (4, 2));
        }
        other => panic!("two waits read as {other:?}, not a refusal"),
    }
    assert_eq!(engine.host_spelling(), "no host file");
    let file = dir.path().join("host.json");
    engine.cooldown.host = Some(file.clone());
    assert_eq!(engine.host_spelling(), file.display().to_string());
}

/// Each `CooldownError` variant pins its `Display` once: the operator's
/// text is the variant's, never a string built at the park.
#[test]
fn a_cooldown_error_names_an_unreadable_host() {
    let error = super::CooldownError::HostUnreadable {
        path: "/tmp/host.json".into(),
        kind: std::io::ErrorKind::PermissionDenied,
    };
    assert_eq!(
        error.to_string(),
        "the host configuration at /tmp/host.json cannot be read: permission denied"
    );
}

#[test]
fn a_cooldown_error_names_an_invalid_host() {
    let source = crate::admission::HostError::Unplaced;
    let error = super::CooldownError::HostInvalid { source };
    assert_eq!(
        error.to_string(),
        "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME names an absolute \
         directory"
    );
}

#[test]
fn a_cooldown_error_names_an_unreadable_last_wait() {
    let error = super::CooldownError::LastWaitUnreadable(brokkr_core::ProviderWaitError::Field {
        seq: 4,
        field: "provider".into(),
    });
    assert_eq!(
        error.to_string(),
        "event 4: the waits entry does not read: provider"
    );
}

#[test]
fn a_cooldown_error_names_too_many_waits() {
    let error = super::CooldownError::TooManyWaits { seq: 5, count: 2 };
    assert_eq!(
        error.to_string(),
        "event 5: a single seat carries at most one wait, not 2"
    );
}

#[test]
fn a_cooldown_error_names_an_unreadable_earlier_wait() {
    let error = super::CooldownError::EarlierWaitUnreadable {
        seq: 3,
        source: brokkr_core::ProviderWaitError::Field {
            seq: 3,
            field: "seconds".into(),
        },
    };
    assert_eq!(
        error.to_string(),
        "an earlier wait does not read: event 3: the waits entry does not read: seconds"
    );
}

#[test]
fn a_cooldown_error_names_an_unreadable_refusal() {
    let error = super::CooldownError::RefusedUnreadable(brokkr_core::ProviderRefusalError::Field {
        seq: 6,
        field: "cause".into(),
    });
    assert_eq!(
        error.to_string(),
        "the failed event's refusal does not read: event 6: the refused record does not read: cause"
    );
}

mod coverage;
mod refusal_cap;
