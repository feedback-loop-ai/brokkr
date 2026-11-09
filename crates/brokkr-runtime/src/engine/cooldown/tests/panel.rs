//! Decision 0103 slice 4c2a: a panel's rate-limited members each record
//! their own wait, the engine waits the latest of them, probes every cooled
//! member in declared order, and retries the whole panel only after every
//! probe has answered, inside decision 0006's attempt cap.

use std::path::{Path, PathBuf};

use brokkr_core::envelope::EventType;
use brokkr_core::fold::{fold, Status};
use brokkr_core::EventEnvelope;
use brokkr_protocol::PreSessionRefusal;
use serde_json::{json, Value};
use time::OffsetDateTime;

use super::*;
use crate::agents::Candidate;
use crate::bundle::{Aggregate, Bundle, Invocation, PanelMember, SeatBody, SeatClass};
use crate::engine::cooldown::sites::RefusedMember;
use crate::engine::tests::{bundle, single_body};
use crate::engine::Engine;

/// A unanimous panel of `members`.
fn panel_of(members: Vec<PanelMember>) -> SeatBody {
    SeatBody::Invocation(Invocation::Panel {
        members,
        aggregate: Aggregate::UnanimousPass,
    })
}

/// One panel member resolved to `candidates`, its own command never used.
fn member(name: &str, candidates: Vec<Candidate>) -> PanelMember {
    PanelMember {
        name: name.into(),
        role_path: "role.md".into(),
        command: vec!["unused".into()],
        candidates,
        written: Vec::new(),
        class: SeatClass::Work,
    }
}

/// One panel member resolved to one provider/route candidate.
fn site_member(name: &str, provider: &str, route: &str) -> PanelMember {
    member(
        name,
        vec![candidate(
            provider,
            &format!("{provider}/{route}"),
            vec!["unused".into()],
        )],
    )
}

/// The two-route panel every writer test starts from: a on claude/first and
/// b on codex/second, neither ever spawned.
fn writer_panel() -> SeatBody {
    panel_of(vec![
        site_member("a", "claude", "first"),
        site_member("b", "codex", "second"),
    ])
}

/// An engine over `body` with two attempts under the standard host.
fn plain_engine(dir: &Path, body: SeatBody, attempts: u64) -> Engine {
    panel_engine(dir, body, attempts, route_host(dir, (1, 10), (1, 10)))
}

/// A refused member the writer tests hand to the terminal.
fn refused_at(site: &str, provider: &str, route: Option<&str>) -> RefusedMember {
    RefusedMember {
        site: site.into(),
        refusal: rate_limit(provider, route),
    }
}

/// Request `effect` and journal one started attempt on `engine`, returning
/// the events a writer reads.
fn start_attempt(engine: &mut Engine) -> Vec<EventEnvelope> {
    request_effect(engine);
    engine
        .append(
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": "a1", "driver": "d"}),
            Some("a1".to_string()),
        )
        .unwrap();
    engine.store.load(&engine.run_id).unwrap()
}

/// A fresh engine over a journal whose terminal event carries `waits`.
fn journal_resumed(dir: &Path, body: SeatBody, waits: Value, host: PathBuf) -> Engine {
    let (run, compiled) = panel_journal(dir, body, waits);
    resume_run(dir, compiled, &run, host)
}

/// The run `run` resumed under `compiled` at `host`.
pub(super) fn resume_run(dir: &Path, compiled: Bundle, run: &str, host: PathBuf) -> Engine {
    let mut engine = crate::engine::Engine::resume(
        brokkr_store::Store::open(&dir.join("forge.db")).unwrap(),
        compiled,
        run,
        Some(dir.join("work")),
    )
    .unwrap();
    engine.cooldown.host = Some(host);
    engine
}

/// Journal `payload` as the failed attempt, then resume and drive a fresh
/// engine over it at `at`, with the panel fixtures' host.
fn resume_failed(dir: &Path, mut engine: Engine, payload: Value, at: OffsetDateTime) -> Engine {
    let compiled = engine.bundle.clone();
    let run = engine.run_id.clone();
    engine
        .append(EventType::EffectFailed, payload, Some("a1".to_string()))
        .unwrap();
    drop(engine);
    let mut resumed = resume_run(dir, compiled, &run, route_host(dir, (1, 10), (1, 10)));
    resumed.cooldown.clock = Clock::advancing(at).0;
    resumed.drive().unwrap();
    resumed
}

/// One inline panel member, whose own command runs.
fn inline(name: &str, command: Vec<String>) -> PanelMember {
    crate::engine::tests::member(name, command)
}

/// A v2 host file with per-route defaults and maxima for claude/first and
/// codex/second, the two routes the panel fixtures use.
fn route_host(dir: &Path, claude: (u64, u64), codex: (u64, u64)) -> PathBuf {
    host_with(
        dir,
        "panel-host.json",
        json!({
            "default_seconds": 1,
            "max_seconds": 20,
            "providers": {
                "claude": {"routes": {"first": {
                    "default_seconds": claude.0, "max_seconds": claude.1}}},
                "codex": {"routes": {"second": {
                    "default_seconds": codex.0, "max_seconds": codex.1}}}
            }
        }),
    )
}

/// The waits a terminal payload spells.
fn payload_waits(payload: &Value) -> Vec<Value> {
    payload["waits"].as_array().cloned().unwrap_or_default()
}

/// The waits on one terminal event, as the journal spells them.
fn wait_entries(event: &EventEnvelope) -> Vec<Value> {
    payload_waits(&event.payload)
}

/// The answers the retry's `effect/started` carries, in the order the engine
/// probed.
fn start_probes(engine: &Engine) -> Vec<Value> {
    starts(engine)
        .last()
        .and_then(|start| start["probes"].as_array().cloned())
        .unwrap_or_default()
}

/// The shared log every fake driver appends its tagged line to, so a panel
/// test can read the engine's own order across members.
fn shared_log(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("all.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// `argv` after waiting for `line` in the shared log: makes the concurrent
/// panel's refusal order the declared one.
fn delayed_after(dir: &Path, line: &str, mut argv: Vec<String>) -> Vec<String> {
    let script = argv.pop().expect("a driver script");
    argv.push(format!(
        "while ! grep -qx '{line}' '{}'; do sleep 0.01; done\n{script}",
        dir.join("all.log").display()
    ));
    argv
}

/// `argv` whose *retry* (its second attempt) waits for a's success before
/// logging, so the concurrent panel's two re-runs log in declared order.
fn retry_after_a_succeeds(dir: &Path, mut argv: Vec<String>) -> Vec<String> {
    let script = argv.pop().expect("a driver script");
    let log = dir.join("all.log");
    let gate = format!(
        "if [ \"$n\" -gt 1 ]; then while ! grep -qx 'a:attempt:succeed' '{}'; do sleep 0.01; done; fi\n",
        log.display()
    );
    argv.push(script.replace(
        "printf '%s\\n' \"attempt:$verdict\"",
        &format!("{gate}printf '%s\\n' \"attempt:$verdict\""),
    ));
    argv
}

/// A rate-limit refusal for the `sum` command's provider and route, with an
/// optional retry-after.
fn rate_limit(provider: &str, route: Option<&str>) -> PreSessionRefusal {
    serde_json::from_value(json!({
        "class": "rate_limit", "provider": provider, "route": route,
        "words": "rate_limit: slow down"
    }))
    .unwrap()
}

/// A one-member wait entry at `site`, ending at `until`.
fn wait_at(site: &str, provider: &str, route: &str, until: OffsetDateTime, seconds: u64) -> Value {
    json!({"site": site, "provider": provider, "route": route,
           "words": "rate_limit: slow down", "seconds": seconds,
           "until": until.format(&Rfc3339).unwrap(), "rounds": 1})
}

/// An engine over a panel, its v2 host file set, with a test clock.
fn panel_engine(dir: &Path, body: SeatBody, attempts: u64, host: PathBuf) -> Engine {
    let mut engine = started(dir, body, work(attempts));
    engine.cooldown.host = Some(host);
    with_clock(engine).0
}

/// A two-attempt two-route panel engine whose members both refuse, `b`'s
/// probe doing `b_probed`.
fn two_route_refusing_engine(dir: &Path, b_probed: Probed) -> Engine {
    two_route_engine(
        dir,
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        b_probed,
        2,
    )
}

/// A two-route panel engine with `attempts` allowed.
fn two_route_engine(
    dir: &Path,
    a_verdicts: &[&str],
    b_verdicts: &[&str],
    a_probed: Probed,
    b_probed: Probed,
    attempts: u64,
) -> Engine {
    let body = two_route_panel(dir, a_verdicts, b_verdicts, a_probed, b_probed);
    plain_engine(dir, body, attempts)
}

/// A panel bundle the crash tests rebuild in their resumed engine.
fn panel_bundle(dir: &Path, body: SeatBody) -> Bundle {
    let mut compiled = bundle(dir, body);
    compiled.seats.get_mut("work").unwrap().limits = work(2);
    compiled
}

/// Journal a panel's failed attempt with `waits` on its terminal event,
/// under `body`, and return the run and bundle a resume uses.
fn panel_journal(dir: &Path, body: SeatBody, waits: Value) -> (String, Bundle) {
    let (mut engine, run) = panel_journal_start(dir, body.clone());
    engine
        .append(
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
                   "start_failure": true, "start_failure_sites": ["a", "b"],
                   "waits": waits}),
            Some("a1".to_string()),
        )
        .unwrap();
    drop(engine);
    (run, panel_bundle(dir, body))
}

/// An engine over `compiled`, its phase and request made.
pub(super) fn starting_engine(dir: &Path, compiled: Bundle) -> Engine {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = brokkr_store::Store::open(&dir.join("forge.db")).unwrap();
    Engine::start(store, compiled, "Feature", Some(dir.join("work"))).unwrap()
}

/// The run and its one started attempt a journal carries before its terminal.
fn panel_journal_start(dir: &Path, body: SeatBody) -> (Engine, String) {
    let mut engine = starting_engine(dir, panel_bundle(dir, body));
    request_effect(&mut engine);
    append_started(&mut engine, "a1");
    let run = engine.run_id.clone();
    (engine, run)
}

/// Journal one started attempt named `attempt` on `engine`.
fn append_started(engine: &mut Engine, attempt: &str) {
    engine
        .append(
            EventType::EffectStarted,
            json!({"effect_id": "effect", "attempt_id": attempt, "driver": "d"}),
            Some(attempt.to_string()),
        )
        .unwrap();
}

/// The malformed earlier terminal a writer reads as an unreadable episode.
fn append_malformed_episode(engine: &mut Engine) {
    append_started(engine, "a0");
    engine
        .append(
            EventType::EffectFailed,
            json!({"effect_id": "effect", "attempt_id": "a0", "error": "refused",
                   "start_failure": true, "start_failure_sites": [null],
                   "waits": malformed_earlier()}),
            Some("a0".to_string()),
        )
        .unwrap();
}

/// A writer engine and its events after an earlier wait that does not read.
fn malformed_episode_writer(dir: &Path) -> (Engine, Vec<EventEnvelope>) {
    let body = writer_panel();
    let mut engine = plain_engine(dir, body, 3);
    request_effect(&mut engine);
    append_malformed_episode(&mut engine);
    append_started(&mut engine, "a1");
    let events = engine.store.load(&engine.run_id).unwrap();
    (engine, events)
}

/// Journal a panel's indeterminate attempt with `waits` on its terminal
/// event, under `body`, and return the run and bundle an operator retry
/// stands on.
fn panel_indeterminate(dir: &Path, body: SeatBody, waits: Value) -> (String, Bundle) {
    let (mut engine, run) = panel_journal_start(dir, body.clone());
    for (event, payload, attempt) in [
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "effect", "attempt_id": "a1",
                   "reason": "panel member a indeterminate", "waits": waits}),
            Some("a1".to_string()),
        ),
        (
            EventType::RunParked,
            json!({"reason": "effect effect indeterminate", "evidence": {}}),
            None,
        ),
    ] {
        engine.append(event, payload, attempt).unwrap();
    }
    drop(engine);
    (run, panel_bundle(dir, body))
}

fn append_retry(db: &Path, run_id: &str) {
    let mut store = brokkr_store::Store::open(db).unwrap();
    store
        .append_next(
            run_id,
            EventType::OperatorCommanded,
            json!({"command_id": "r", "command": "retry", "args": {},
                   "operator": "operator"}),
            None,
            None,
        )
        .unwrap();
    store
        .append_next(
            run_id,
            EventType::OperatorAccepted,
            json!({"command_id": "r", "operator": "operator", "reason": "retry"}),
            None,
            None,
        )
        .unwrap();
}

/// A panel whose two members both succeed, probed by answering drivers.
fn succeeding_panel(dir: &Path) -> SeatBody {
    two_route_panel(
        dir,
        &["succeed"],
        &["succeed"],
        Probed::Answer,
        Probed::Answer,
    )
}

/// A wait at `site` ending `seconds` from the test's base instant.
fn wait_in(site: &str, provider: &str, route: &str, seconds: u64) -> Value {
    wait_at(
        site,
        provider,
        route,
        base() + time::Duration::seconds(seconds as i64),
        seconds,
    )
}

/// A two-route panel's two drivers: a on claude/first and b on codex/second.
fn two_route_drivers(
    dir: &Path,
    a_verdicts: &[&str],
    b_verdicts: &[&str],
    a_probed: Probed,
    b_probed: Probed,
) -> (Vec<String>, Vec<String>) {
    let a = driver(dir, "a", a_verdicts, a_probed, None, "claude", "first");
    let b = driver(dir, "b", b_verdicts, b_probed, None, "codex", "second");
    (a, b)
}

/// One refused member and its site, for a writer's collection.
fn refused_pair(a: RefusedMember, b: RefusedMember) -> Vec<RefusedMember> {
    vec![a, b]
}

/// The terminal a panel writer makes for `payload` under `refused`.
fn writer_terminal(
    engine: &Engine,
    events: &[EventEnvelope],
    refused: &[RefusedMember],
    payload: Value,
    retry: bool,
) -> crate::engine::cooldown::sites::PanelTerminal {
    engine.panel_rate_limit_terminal(events, "effect", payload, refused, retry)
}

/// The two candidates' panel: a on claude/first and b on codex/second.
fn two_candidate_panel(a: Vec<String>, b: Vec<String>) -> SeatBody {
    panel_of(vec![
        member("a", vec![candidate("claude", "claude/first", a)]),
        member("b", vec![candidate("codex", "codex/second", b)]),
    ])
}

/// A panel whose `a` runs `a` inline and whose `b` refuses once on
/// codex/second and then succeeds.
fn b_only_panel(dir: &Path, a: Vec<String>) -> SeatBody {
    let b = driver(
        dir,
        "b",
        &["refuse", "succeed"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    panel_of(vec![
        inline("a", a),
        member("b", vec![candidate("codex", "codex/second", b)]),
    ])
}

/// A writer engine over one site member, its request made.
fn single_writer_engine(dir: &Path, attempts: u64) -> Engine {
    let body = panel_of(vec![site_member("a", "claude", "first")]);
    let mut engine = plain_engine(dir, body, attempts);
    request_effect(&mut engine);
    engine
}

/// A writer engine over one site member with one started attempt, and the
/// events a writer reads.
fn single_writer(dir: &Path, attempts: u64) -> (Engine, Vec<EventEnvelope>) {
    let engine = single_writer_engine(dir, attempts);
    let events = engine.store.load(&engine.run_id).unwrap();
    (engine, events)
}

/// A fresh engine over a journal resumed at the base instant.
fn journal_engine(dir: &Path, body: SeatBody, waits: Value) -> Engine {
    let mut engine = journal_resumed(dir, body, waits, route_host(dir, (1, 10), (1, 10)));
    engine.cooldown.clock = Clock::advancing(base()).0;
    engine
}

/// The terminal a writer makes when `a`'s route never resolved and `b`'s
/// wait computes, both refused with a rate limit.
fn unresolved_a_terminal(
    engine: &Engine,
    events: &[EventEnvelope],
    payload: Value,
) -> crate::engine::cooldown::sites::PanelTerminal {
    let refused = refused_pair(
        refused_at("a", "claude", None),
        refused_at("b", "codex", Some("second")),
    );
    writer_terminal(engine, events, &refused, payload, true)
}

/// The terminal a two-refused-member writer makes, both computed.
fn writer_terminal_pair(
    engine: &Engine,
    events: &[EventEnvelope],
    payload: Value,
) -> crate::engine::cooldown::sites::PanelTerminal {
    let refused = refused_pair(
        refused_at("a", "claude", Some("first")),
        refused_at("b", "codex", Some("second")),
    );
    writer_terminal(engine, events, &refused, payload, true)
}

/// The started payloads an engine journaled, in order.
fn started_payloads(engine: &Engine) -> Vec<Value> {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectStarted)
        .map(|event| event.payload)
        .collect()
}

/// The newest terminal event a panel journaled.
fn last_terminal(engine: &Engine) -> EventEnvelope {
    engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .rev()
        .find(|event| {
            matches!(
                event.event_type,
                EventType::EffectIndeterminate | EventType::EffectFailed
            )
        })
        .expect("a terminal event")
}

/// An inline panel member whose driver refuses once on claude/first.
fn inline_refusing_a(dir: &Path) -> Vec<String> {
    driver(
        dir,
        "a",
        &["refuse"],
        Probed::Answer,
        None,
        "claude",
        "first",
    )
}

/// The cap's reason a resume reads after unresolved `a` and computable `b`,
/// with `sites` the journal's own candidate list.
fn unresolved_a_cap_reason(dir: &Path, body: SeatBody, sites: Value) -> String {
    let mut engine = plain_engine(dir, body, 1);
    let events = start_attempt(&mut engine);
    let terminal = unresolved_a_terminal(
        &engine,
        &events,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "start_failure": true, "start_failure_sites": sites}),
    );
    assert!(terminal.park.is_none(), "{:?}", terminal.park);
    assert_eq!(terminal.payload["refused"]["site"], "a");
    let resumed = resume_failed(dir, engine, terminal.payload, base());
    park_reason(&resumed)
}

/// Drive `engine`, require one failed event with one wait, and return it.
fn drove_one_wait(engine: &mut Engine) -> Value {
    engine.drive().unwrap();
    let failed = failed(engine);
    assert_eq!(failed.len(), 1);
    let waits = wait_entries(&failed[0]);
    assert_eq!(waits.len(), 1, "{waits:?}");
    waits[0].clone()
}

/// The one wait a writer's terminal records, and the terminal itself.
fn writer_first_wait(
    engine: &Engine,
    events: &[EventEnvelope],
    refused: &[RefusedMember],
    payload: Value,
    retry: bool,
) -> (Value, crate::engine::cooldown::sites::PanelTerminal) {
    let terminal = writer_terminal(engine, events, refused, payload, retry);
    let waits = payload_waits(&terminal.payload);
    assert_eq!(waits.len(), 1, "{waits:?}");
    (waits[0].clone(), terminal)
}

/// A crash test's resumed engine, driven to its end.
#[cfg(target_os = "linux")]
fn resume_and_drive(dir: &Path, compiled: Bundle, run: &str, clock: Clock) -> Engine {
    let mut engine = resume_panel(dir, compiled, run, clock);
    engine.drive().unwrap();
    engine
}

/// A two-route panel: `a` on claude/first and `b` on codex/second, each with
/// its own verdict list and probe behaviour.
fn two_route_panel(
    dir: &Path,
    a_verdicts: &[&str],
    b_verdicts: &[&str],
    a_probed: Probed,
    b_probed: Probed,
) -> SeatBody {
    let (a, b) = two_route_drivers(dir, a_verdicts, b_verdicts, a_probed, b_probed);
    two_candidate_panel(a, b)
}

#[test]
fn a_panels_rate_limited_members_each_record_their_own_wait() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_route_drivers(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
    );
    let b = retry_after_a_succeeds(dir.path(), delayed_after(dir.path(), "a:attempt:refuse", b));
    let body = two_candidate_panel(a, b);
    let mut engine = panel_engine(
        dir.path(),
        body,
        2,
        route_host(dir.path(), (2, 20), (3, 20)),
    );
    engine.drive().unwrap();
    let failed = failed(&engine);
    assert_eq!(failed.len(), 1, "{failed:?}");
    let waits = wait_entries(&failed[0]);
    assert_eq!(waits.len(), 2, "{waits:?}");
    assert_eq!(waits[0]["site"], "a");
    assert_eq!(waits[0]["provider"], "claude");
    assert_eq!(waits[0]["route"], "first");
    assert_eq!(waits[0]["seconds"], 2);
    assert_eq!(waits[0]["rounds"], 1);
    assert_eq!(waits[1]["site"], "b");
    assert_eq!(waits[1]["provider"], "codex");
    assert_eq!(waits[1]["route"], "second");
    assert_eq!(waits[1]["seconds"], 3);
    assert_eq!(waits[1]["rounds"], 1);
    let started = starts(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    let probes = start_probes(&engine);
    assert_eq!(probes.len(), 2, "{probes:?}");
    assert_eq!(probes[0]["site"], "a");
    assert_eq!(probes[1]["site"], "b");
    // Each probe starts under its own member's derived driver seat, as the
    // retried member's attempt does, not the panel's bare seat name.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.seat")).unwrap(),
        "work:a"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("b.seat")).unwrap(),
        "work:b"
    );
    assert_eq!(
        shared_log(dir.path()),
        [
            "a:attempt:refuse",
            "b:attempt:refuse",
            "a:probe",
            "b:probe",
            "a:attempt:succeed",
            "b:attempt:succeed",
        ]
    );
}

#[test]
fn a_panel_retries_only_after_every_cooled_member_has_answered() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_refusing_engine(dir.path(), Probed::Hang);
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "a"), ["attempt:refuse", "probe"]);
    assert_eq!(logs(dir.path(), "b"), ["attempt:refuse", "probe"]);
    assert_eq!(
        starts(&engine).len(),
        1,
        "the panel retried before b answered"
    );
    assert_eq!(failed(&engine).len(), 1, "the probe spent no attempt");
    let reason = park_reason(&engine);
    assert!(reason.contains("ended without an answer"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
}

#[test]
fn a_succeeded_member_beside_a_refused_one_re_runs_on_the_retry() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_engine(
        dir.path(),
        &["succeed", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
        2,
    );
    let wait = drove_one_wait(&mut engine);
    assert_eq!(wait["site"], "b");
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:succeed", "attempt:succeed"],
        "the settled member did not re-run with the whole panel"
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(start_probes(&engine).len(), 1);
}

/// A member whose first attempt is indeterminate and whose second succeeds:
/// the operator retry's own case.
fn indeterminate_once(dir: &Path, tag: &str) -> Vec<String> {
    let count = dir.join(format!("{tag}.count")).display().to_string();
    let log = dir.join(format!("{tag}.log")).display().to_string();
    let glog = dir.join("all.log").display().to_string();
    let capabilities = json!({
        "proto": brokkr_protocol::PROTO, "msg_id": "c", "type": "capabilities",
        "driver": "test", "version": "1", "supports": []
    })
    .to_string();
    let accepted = r#"printf '%s,"type":"accepted","effect_id":"%s","attempt_id":"%s"}\n' "$base" "$eid" "$aid""#;
    let succeeded = r#"printf '%s,"type":"result","effect_id":"%s","attempt_id":"%s","status":"succeeded","result":{"result":"complete"}}\n' "$base" "$eid" "$aid""#;
    vec![
        "sh".into(),
        "-c".into(),
        format!(
            r#"n=$(cat '{count}' 2>/dev/null || echo 0)
read -r hello
printf '%s\n' '{capabilities}'
read -r start
case "$start" in *'"type":"start"'*) ;; *) exit 0 ;; esac
eid=$(printf '%s' "$start" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
aid=$(printf '%s' "$start" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
base='{{"proto":"forge-driver/v1","msg_id":"m"'
n=$((n+1)); printf '%s' "$n" > '{count}'
printf '%s\n' 'attempt:indeterminate' >> '{log}'
printf '%s:attempt:indeterminate\n' '{tag}' >> '{glog}'
{accepted}
if [ "$n" -gt 1 ]; then
  {succeeded}
fi
read -r bye
"#
        ),
    ]
}

#[test]
fn an_indeterminate_member_parks_with_the_refused_members_wait_and_no_probe() {
    let dir = tempfile::tempdir().unwrap();
    let a = indeterminate_once(dir.path(), "a");
    let body = b_only_panel(dir.path(), a);
    let mut engine = plain_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    let terminal = last_terminal(&engine);
    assert_eq!(terminal.event_type, EventType::EffectIndeterminate);
    let waits = wait_entries(&terminal);
    assert_eq!(waits.len(), 1, "{waits:?}");
    assert_eq!(waits[0]["site"], "b");
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse"],
        "the indeterminate panel probed b"
    );
    assert_eq!(starts(&engine).len(), 1);
    // The operator retry takes b's recorded wait first, then probes it, and
    // only then runs the panel again.
    append_retry(&dir.path().join("forge.db"), &engine.run_id);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "attempt:succeed"],
        "the operator retry did not wait and probe b before its attempt"
    );
    let started = started_payloads(&engine);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"][0]["site"], "b");
}

/// An indeterminate panel publishes no `error`: the missing cool-down note
/// belongs on a failed event's error tail, not on a terminal whose contract
/// has no such field. The panel still parks on its own indeterminate reason.
#[test]
fn an_indeterminate_panel_keeps_an_undeclared_note_off_its_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let body = b_only_panel(dir.path(), indeterminate_once(dir.path(), "a"));
    let mut engine = panel_engine(dir.path(), body, 2, v1_host(dir.path()));
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let terminal = last_terminal(&engine);
    assert_eq!(terminal.event_type, EventType::EffectIndeterminate);
    assert!(
        terminal.payload.get("error").is_none(),
        "the indeterminate event carries an unpublished field: {}",
        terminal.payload
    );
    assert!(
        terminal.payload.get("waits").is_none(),
        "{}",
        terminal.payload
    );
    assert!(
        terminal.payload["reason"].as_str().is_some(),
        "{}",
        terminal.payload
    );
    assert_eq!(
        fold(&events).unwrap().run.status(),
        Status::AwaitingOperator
    );
}

#[test]
fn another_refusal_beside_a_rate_limit_gets_no_wait_and_no_probe() {
    let dir = tempfile::tempdir().unwrap();
    let (a, mut b) = two_route_drivers(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse"],
        Probed::Answer,
        Probed::Answer,
    );
    let script = b
        .pop()
        .unwrap()
        .replace("\"class\":\"rate_limit\"", "\"class\":\"other\"");
    b.push(script);
    let second = driver(
        dir.path(),
        "b-second",
        &["succeed"],
        Probed::Answer,
        None,
        "codex",
        "third",
    );
    let body = panel_of(vec![
        member("a", vec![candidate("claude", "claude/first", a)]),
        member(
            "b",
            vec![
                candidate("codex", "codex/second", b),
                candidate("codex", "codex/third", second),
            ],
        ),
    ]);
    let mut engine = plain_engine(dir.path(), body, 2);
    let wait = drove_one_wait(&mut engine);
    assert_eq!(wait["site"], "a");
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse"],
        "b was probed for a refusal that is not a rate limit"
    );
    assert_eq!(
        logs(dir.path(), "b-second"),
        ["attempt:succeed"],
        "b's chain did not advance to its next link"
    );
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    assert_eq!(start_probes(&engine).len(), 1);
    assert_eq!(starts(&engine).len(), 2);
    let provenance = starts(&engine)[1]["provenance"].clone();
    let b = provenance
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["member"] == "b")
        .expect("b's provenance");
    assert_eq!(b["model"], "codex/third");
}

#[test]
fn each_member_advances_its_own_chain() {
    let dir = tempfile::tempdir().unwrap();
    let first = driver(
        dir.path(),
        "a-first",
        &["refuse"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let second = driver(
        dir.path(),
        "a-second",
        &["succeed"],
        Probed::Answer,
        None,
        "codex",
        "second",
    );
    let b = driver(
        dir.path(),
        "b",
        &["succeed", "succeed"],
        Probed::Answer,
        None,
        "claude",
        "first",
    );
    let body = panel_of(vec![
        member(
            "a",
            vec![
                candidate("claude", "claude/first", first),
                candidate("codex", "codex/second", second),
            ],
        ),
        member("b", vec![candidate("claude", "claude/first", b)]),
    ]);
    let mut engine = plain_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "a-first"),
        ["attempt:refuse", "probe"],
        "the probe did not ask the link the refusal named"
    );
    assert_eq!(logs(dir.path(), "a-second"), ["attempt:succeed"]);
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:succeed", "attempt:succeed"],
        "b's link moved with a's chain"
    );
    let provenance = starts(&engine)[1]["provenance"].clone();
    let a = provenance
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["member"] == "a")
        .expect("a's provenance");
    assert_eq!(a["model"], "codex/second");
    let b = provenance
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["member"] == "b")
        .expect("b's provenance");
    assert_eq!(b["chain_index"], 0);
}

#[test]
fn rounds_are_counted_per_site() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_engine(
        dir.path(),
        &["refuse", "refuse", "succeed"],
        &["refuse", "succeed", "succeed"],
        Probed::Answer,
        Probed::Answer,
        3,
    );
    engine.drive().unwrap();
    let failed = failed(&engine);
    assert_eq!(failed.len(), 2, "{failed:?}");
    let first = wait_entries(&failed[0]);
    assert_eq!(
        (first[0]["site"].clone(), first[0]["rounds"].clone()),
        (json!("a"), json!(1))
    );
    assert_eq!(
        (first[1]["site"].clone(), first[1]["rounds"].clone()),
        (json!("b"), json!(1))
    );
    let second = wait_entries(&failed[1]);
    assert_eq!(second.len(), 1, "{second:?}");
    assert_eq!(second[0]["site"], "a");
    assert_eq!(second[0]["rounds"], 2);
    assert_eq!(
        second[0]["seconds"], 2,
        "a's second wait was not doubled for its own second round"
    );
    assert_eq!(starts(&engine).len(), 3);
}

#[test]
fn the_episode_sums_across_members_and_parks_the_member_past_its_own_maximum() {
    let dir = tempfile::tempdir().unwrap();
    let body = two_route_panel(
        dir.path(),
        &["refuse", "refuse"],
        &["refuse", "refuse"],
        Probed::Answer,
        Probed::Answer,
    );
    let mut engine = panel_engine(dir.path(), body, 3, route_host(dir.path(), (1, 10), (1, 5)));
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("member b"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
    assert!(reason.contains("episode 6s"), "{reason}");
    assert!(reason.contains("declared 5s maximum"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    let failed = failed(&engine);
    let waits = wait_entries(&failed[1]);
    assert_eq!(waits.len(), 2, "the computable waits are kept: {waits:?}");
    assert_eq!(failed[1].payload["refused"]["site"], "b");
    assert_eq!(failed[1].payload["refused"]["cause"], "past-maximum");
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "probe", "attempt:refuse"]
    );
}

#[test]
fn a_park_cause_at_one_member_keeps_the_other_wait_and_resumes_with_none() {
    let dir = tempfile::tempdir().unwrap();
    let body = writer_panel();
    let mut engine = plain_engine(dir.path(), body, 2);
    let events = start_attempt(&mut engine);
    let refused = vec![
        refused_at("a", "claude", None),
        refused_at("b", "codex", Some("second")),
    ];
    let (wait, terminal) = writer_first_wait(
        &engine,
        &events,
        &refused,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "start_failure": true, "start_failure_sites": ["a", "b"]}),
        true,
    );
    assert_eq!(wait["site"], "b");
    assert_eq!(terminal.payload["refused"]["site"], "a");
    assert_eq!(terminal.payload["refused"]["cause"], "route-unresolved");
    let reason = terminal.park.expect("a parking member parks below the cap");
    assert!(reason.starts_with("member a: "), "{reason}");
    assert!(reason.contains("route could not be resolved"), "{reason}");
    // A crash between the failed event and its park resumes the record: the
    // same reason, no wait taken, no driver invoked.
    let resumed = resume_failed(
        dir.path(),
        engine,
        terminal.payload,
        base() + time::Duration::seconds(1),
    );
    let parked = park_reason(&resumed);
    assert_eq!(parked, reason);
    assert!(logs(dir.path(), "a").is_empty());
    assert!(logs(dir.path(), "b").is_empty());
}

#[test]
fn a_member_with_no_declared_cool_down_writes_no_wait_and_the_note_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let body = two_route_panel(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
    );
    let mut engine = panel_engine(dir.path(), body, 2, v1_host(dir.path()));
    engine.drive().unwrap();
    let failed = failed(&engine);
    let waits = wait_entries(&failed[0]);
    assert!(waits.is_empty(), "{waits:?}");
    let error = failed[0].payload["error"].as_str().unwrap();
    // The built v2 reader requires the block's own defaults, so a route is
    // either declared through them or the whole file is not: here, 4c1's
    // option (a) names the absent declaration for the first member.
    assert!(
        error.contains("no cool-down is declared for claude/first"),
        "{error}"
    );
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:refuse", "attempt:succeed"],
        "a was probed without a declared cool-down"
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse", "attempt:succeed"],
        "b was probed without a declared cool-down"
    );
}

#[test]
fn the_cap_parks_after_the_second_attempt_and_names_each_member() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_engine(
        dir.path(),
        &["refuse", "refuse", "succeed"],
        &["refuse", "refuse", "succeed"],
        Probed::Answer,
        Probed::Answer,
        2,
    );
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.starts_with("effect "), "{reason}");
    assert!(reason.contains("failed 2 of 2 attempt(s)"), "{reason}");
    let a = reason.find("member a").expect("a's facts");
    let b = reason.find("member b").expect("b's facts");
    assert!(
        a < b,
        "the members' facts are not in declared order: {reason}"
    );
    assert!(reason.contains("claude/first"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
    assert!(reason.contains("rate_limit: slow down"), "{reason}");
    assert_eq!(starts(&engine).len(), 2, "a third attempt started");
}

/// At the cap, a panel whose first member cannot compute a wait keeps that
/// member's refusal and the other member's wait, each with its own facts and
/// in declared order: a refusal is never paired with another member's wait.
#[test]
fn the_cap_keeps_a_parking_members_refusal_beside_the_other_members_wait() {
    let dir = tempfile::tempdir().unwrap();
    let reason = unresolved_a_cap_reason(dir.path(), writer_panel(), json!(["a", "b"]));
    let a = reason.find("member a").expect("a's refusal");
    let b = reason.find("member b").expect("b's own wait facts");
    assert!(a < b, "the facts are not in declared order: {reason}");
    assert!(reason.contains("claude/an unresolved route"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
    assert!(reason.contains("until"), "{reason}");
    assert!(logs(dir.path(), "a").is_empty());
    assert!(logs(dir.path(), "b").is_empty());
}

/// At the cap, an inline member's facts keep their declared place. The
/// journal's `start_failure_sites` names candidates only, so the cap reads
/// the panel's own declaration; a and b both refuse here, and a is inline.
#[test]
fn the_cap_keeps_an_inline_members_wait_in_declared_order() {
    let dir = tempfile::tempdir().unwrap();
    let body = b_only_panel(dir.path(), inline_refusing_a(dir.path()));
    let mut engine = plain_engine(dir.path(), body, 1);
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    let a = reason.find("member a").expect("a's own wait facts");
    let b = reason.find("member b").expect("b's own wait facts");
    assert!(a < b, "the facts are not in declared order: {reason}");
    assert!(reason.contains("claude/first"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
    assert_eq!(starts(&engine).len(), 1, "the cap retried the panel");
}

/// An inline member that cannot compute a wait keeps its refused record in
/// its declared place, before the candidate member's wait, even though the
/// journal lists only the candidate.
#[test]
fn the_cap_keeps_an_inline_members_refusal_in_declared_order() {
    let dir = tempfile::tempdir().unwrap();
    let body = b_only_panel(dir.path(), inline_refusing_a(dir.path()));
    let reason = unresolved_a_cap_reason(dir.path(), body, json!(["b"]));
    let a = reason.find("member a").expect("a's refusal");
    let b = reason.find("member b").expect("b's own wait facts");
    assert!(a < b, "the facts are not in declared order: {reason}");
    assert!(reason.contains("with no until"), "{reason}");
    assert!(reason.contains("claude/an unresolved route"), "{reason}");
    assert!(logs(dir.path(), "a").is_empty());
    assert!(logs(dir.path(), "b").is_empty());
}

/// The chief's R1 case: a selector's default wraps the declared
/// `[a=claude/first, b=codex/second]` panel, both members refuse a rate
/// limit, and the cap's waits follow the panel's declaration, a before b,
/// not the journal's candidate-only `start_failure_sites`.
fn selected_inline_panel(dir: &Path) -> SeatBody {
    SeatBody::Select {
        cases: Default::default(),
        default: Some(Box::new(b_only_panel(dir, inline_refusing_a(dir)))),
    }
}

/// Both members refuse under a selector-wrapped panel at the cap: the two
/// waits are the declaration's order, a before b, because the cap resolves
/// the body through the established selection path.
#[test]
fn chief_selected_panel_cap_wait_order() {
    let dir = tempfile::tempdir().unwrap();
    let body = selected_inline_panel(dir.path());
    let mut engine = plain_engine(dir.path(), body, 1);
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("failed 1 of 1 attempt(s)"), "{reason}");
    let a = reason.find("member a").expect("a's own wait facts");
    let b = reason.find("member b").expect("b's own wait facts");
    assert!(
        a < b,
        "the selector-wrapped panel's waits are not in declared order: {reason}"
    );
    assert!(reason.contains("claude/first"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
    assert_eq!(starts(&engine).len(), 1, "the cap retried the panel");
}

/// The chief's alternate R1 case: `a`'s route is unresolved and only `b`
/// has a wait under a selector-wrapped panel. The cap keeps `a`'s refusal
/// before `b`'s wait, the declaration's order, even though the journal's
/// `start_failure_sites` names only the candidate `b`.
#[test]
fn chief_selected_panel_cap_refusal_order() {
    let dir = tempfile::tempdir().unwrap();
    let body = selected_inline_panel(dir.path());
    let reason = unresolved_a_cap_reason(dir.path(), body, json!(["b"]));
    let a = reason.find("member a").expect("a's refusal");
    let b = reason.find("member b").expect("b's own wait facts");
    assert!(
        a < b,
        "the selector-wrapped panel's facts are not in declared order: {reason}"
    );
    assert!(reason.contains("claude/an unresolved route"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
    assert!(logs(dir.path(), "a").is_empty());
    assert!(logs(dir.path(), "b").is_empty());
}

/// A declared site with neither a wait nor a matching refusal adds nothing,
/// while a refused record at no declared site and no wait is still named at
/// the cap: the two `cap_ordered_facts` arms a panel can present.
#[test]
fn the_cap_names_a_refusal_at_neither_a_declared_site_nor_a_wait() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = plain_engine(dir.path(), writer_panel(), 1);
    let _events = start_attempt(&mut engine);
    let payload = json!({
        "effect_id": "effect", "attempt_id": "a1", "error": "refused",
        "start_failure": true, "start_failure_sites": ["b"],
        "waits": [wait_in("b", "codex", "second", 1)],
        "refused": {"cause": "past-maximum", "site": "c", "provider": "claude",
                    "route": "first", "words": "rate_limit: slow down",
                    "seconds": 1, "max_seconds": 10}
    });
    let resumed = resume_failed(dir.path(), engine, payload, base());
    let reason = park_reason(&resumed);
    assert!(!reason.contains("member a"), "{reason}");
    assert!(reason.contains("member c"), "{reason}");
    assert!(reason.contains("codex/second"), "{reason}");
}

#[test]
fn a_stop_after_one_probe_answers_probes_no_later_member_and_retries_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_engine(
        dir.path(),
        &["refuse", "succeed"],
        &["refuse", "succeed"],
        Probed::Barrier,
        Probed::Answer,
        2,
    );
    let db = dir.path().join("forge.db");
    let run = engine.run_id.clone();
    let engine_dir = dir.path().to_path_buf();
    let driver = std::thread::spawn(move || {
        engine.drive().unwrap();
        engine
    });
    let started = engine_dir.join("a.started");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !started.exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "probe a never started"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    append_stop(&db, &run);
    std::fs::write(engine_dir.join("a.release"), "").unwrap();
    let engine = driver.join().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:refuse"],
        "probe b ran after the stop"
    );
    assert_eq!(starts(&engine).len(), 1);
}

#[test]
fn a_resume_waits_the_later_remainder_and_probes_every_member() {
    let dir = tempfile::tempdir().unwrap();
    let body = succeeding_panel(dir.path());
    let until = base() + time::Duration::seconds(3);
    let waits = json!([
        wait_in("a", "claude", "first", 1),
        wait_in("b", "codex", "second", 3)
    ]);
    let mut engine = journal_resumed(
        dir.path(),
        body,
        waits,
        route_host(dir.path(), (1, 10), (1, 10)),
    );
    let (clock, now) = Clock::advancing(base());
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    assert_eq!(
        *now.lock().unwrap(),
        until,
        "the resume did not wait the later end"
    );
    assert_eq!(logs(dir.path(), "a"), ["probe", "attempt:succeed"]);
    assert_eq!(logs(dir.path(), "b"), ["probe", "attempt:succeed"]);
    assert_eq!(starts(&engine).len(), 2);
    assert_eq!(start_probes(&engine).len(), 2);
}

#[test]
fn a_member_wait_at_a_body_that_is_not_a_panel_refuses_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let body = single_body(vec!["unused".into()]);
    let mut engine = journal_engine(
        dir.path(),
        body,
        json!([wait_in("a", "claude", "first", 1)]),
    );
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(
        reason.contains("member a stands at a body that is not a panel"),
        "{reason}"
    );
}

#[test]
fn a_member_wait_the_panel_does_not_declare_refuses_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let body = succeeding_panel(dir.path());
    let mut engine = journal_engine(
        dir.path(),
        body,
        json!([wait_in("ghost", "claude", "first", 1)]),
    );
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(
        reason.contains("the panel declares no member ghost"),
        "{reason}"
    );
}

#[test]
fn an_inline_members_own_command_is_probed_and_retried() {
    let dir = tempfile::tempdir().unwrap();
    let a = delayed_after(
        dir.path(),
        "b:attempt:refuse",
        driver(
            dir.path(),
            "a",
            &["refuse", "succeed"],
            Probed::Answer,
            None,
            "claude",
            "first",
        ),
    );
    let body = b_only_panel(dir.path(), a);
    let mut engine = plain_engine(dir.path(), body, 2);
    engine.drive().unwrap();
    assert_eq!(
        logs(dir.path(), "a"),
        ["attempt:refuse", "probe", "attempt:succeed"]
    );
    let waits = wait_entries(&failed(&engine)[0]);
    assert_eq!(waits.len(), 2, "{waits:?}");
    assert_eq!(start_probes(&engine).len(), 2);
}

/// The episode an earlier wait cannot read: the failed panel parks naming
/// the first refused member even though every member's own wait is
/// computable in isolation.
#[test]
fn an_unreadable_episode_parks_the_first_refused_member() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, events) = malformed_episode_writer(dir.path());
    let terminal = writer_terminal_pair(
        &engine,
        &events,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused"}),
    );
    let reason = terminal.park.expect("an unreadable episode parks");
    assert!(reason.starts_with("member a: "), "{reason}");
    assert!(reason.contains("an earlier wait does not read"), "{reason}");
}

/// An unreadable episode whose first refused member never resolved a route:
/// the record stays schema-valid by naming that member's own route-unresolved
/// park, and a fresh engine reads it back to the same reason.
#[test]
fn an_unreadable_episode_keeps_a_readable_route_unresolved_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, events) = malformed_episode_writer(dir.path());
    let terminal = unresolved_a_terminal(
        &engine,
        &events,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused"}),
    );
    assert_eq!(terminal.payload["refused"]["cause"], "route-unresolved");
    assert_eq!(terminal.payload["refused"]["site"], "a");
    assert!(terminal.payload["refused"].get("route").is_none());
    let reason = terminal.park.clone().expect("an unreadable episode parks");
    assert!(reason.contains("route could not be resolved"), "{reason}");
    let resumed = resume_failed(dir.path(), engine, terminal.payload, base());
    assert_eq!(park_reason(&resumed), reason);
}

/// A host file that cannot be read parks the first refused member before any
/// wait is computed, and the failed event carries no invented wait.
#[test]
fn an_unreadable_host_parks_the_first_refused_member_with_no_wait() {
    let dir = tempfile::tempdir().unwrap();
    let body = writer_panel();
    let mut engine = panel_engine(dir.path(), body, 3, dir.path().to_path_buf());
    let events = start_attempt(&mut engine);
    let terminal = writer_terminal_pair(
        &engine,
        &events,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused"}),
    );
    assert!(
        terminal.payload.get("waits").is_none(),
        "{}",
        terminal.payload
    );
    assert_eq!(terminal.payload["refused"]["site"], "a");
    assert_eq!(terminal.payload["refused"]["cause"], "host-unreadable");
    let reason = terminal.park.expect("unreadable bounds park");
    assert!(
        reason.contains("host configuration could not be read"),
        "{reason}"
    );
}

/// A member whose end the journal cannot spell parks the panel and keeps
/// the other member's computable wait beside its own refusal.
#[test]
fn a_member_with_no_representable_end_parks_keeping_the_other_wait() {
    let dir = tempfile::tempdir().unwrap();
    let body = writer_panel();
    let at = OffsetDateTime::parse("9999-12-31T23:59:50Z", &Rfc3339).unwrap();
    let mut engine = plain_engine(dir.path(), body, 3);
    engine.cooldown.clock = Clock::advancing(at).0;
    let events = start_attempt(&mut engine);
    // The reader refuses a non-positive retry-after; the test's refusal is
    // built through the same typed door the driver uses.
    let endless: PreSessionRefusal = serde_json::from_value(json!({
        "class": "rate_limit", "provider": "claude", "route": "first",
        "words": "rate_limit: slow down", "retry_after_seconds": 1_000_000_000_000u64
    }))
    .unwrap();
    let refused = vec![
        RefusedMember {
            site: "a".into(),
            refusal: endless,
        },
        refused_at("b", "codex", Some("second")),
    ];
    let (wait, terminal) = writer_first_wait(
        &engine,
        &events,
        &refused,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused"}),
        true,
    );
    assert_eq!(wait["site"], "b");
    assert_eq!(terminal.payload["refused"]["site"], "a");
    assert_eq!(terminal.payload["refused"]["cause"], "no-end");
    let reason = terminal.park.expect("a no-end member parks");
    assert!(reason.contains("has no representable end"), "{reason}");
}

/// An indeterminate panel whose only refused member has no computable wait
/// still parks by its own indeterminate reason and writes no wait.
#[test]
fn an_indeterminate_panel_with_no_computable_wait_writes_none() {
    let dir = tempfile::tempdir().unwrap();
    let (engine, events) = single_writer(dir.path(), 2);
    let refused = vec![refused_at("a", "claude", None)];
    let terminal = writer_terminal(
        &engine,
        &events,
        &refused,
        json!({"effect_id": "effect", "attempt_id": "a1", "reason": "indeterminate"}),
        false,
    );
    assert!(
        terminal.payload.get("waits").is_none(),
        "{}",
        terminal.payload
    );
    assert!(terminal.park.is_none(), "{:?}", terminal.park);
}

/// The panel's two-wait journal and the bundle a crash test resumes under.
#[cfg(target_os = "linux")]
fn crashed_panel(dir: &Path, a: Probed, b: Probed) -> (String, Bundle, PathBuf) {
    let body = two_route_panel(dir, &["succeed"], &["succeed"], a, b);
    let until = base() + time::Duration::seconds(1);
    let waits = json!([
        wait_at("a", "claude", "first", until, 1),
        wait_in("b", "codex", "second", 1)
    ]);
    let (run, compiled) = panel_journal(dir, body, waits);
    (run, compiled, route_host(dir, (1, 10), (1, 10)))
}

/// The run resumed under `compiled`, with the crash tests' host and clock.
#[cfg(target_os = "linux")]
fn resume_panel(dir: &Path, compiled: Bundle, run: &str, clock: Clock) -> Engine {
    let mut engine = resume_run(dir, compiled, run, route_host(dir, (1, 10), (1, 10)));
    engine.cooldown.clock = clock;
    engine
}

/// A crash after probe b's cleanup but before its settlement: a's probe was
/// already released, b's one attempt stands proven settled, and the resume
/// releases it and probes every member again from the first.
#[cfg(target_os = "linux")]
#[test]
fn a_crash_mid_round_releases_the_proven_reservation_and_probes_every_member_again() {
    use crate::engine::broker::admission::tests::reservation_path;

    let test = "engine::cooldown::tests::panel::a_crash_mid_round_releases_the_proven_reservation_and_probes_every_member_again";
    let Some((dir, home, _stand)) = super::crash::own_engine(test) else {
        return;
    };
    let (run, compiled, _host) = crashed_panel(dir.path(), Probed::Answer, Probed::Answer);
    let now = std::sync::Arc::new(std::sync::Mutex::new(base()));
    let mut crashed = resume_panel(
        dir.path(),
        compiled.clone(),
        &run,
        super::crash::crashing_clock(&dir.path().join("b.done"), &now),
    );
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crashed.drive()));
    assert!(
        panic.is_err(),
        "the clock did not crash after b's probe cleanup"
    );
    drop(crashed);
    let at = reservation_path(&home, &dir.path().join("work"), &run);
    let (standing, _) = super::crash::reservation_after(&at);
    assert_eq!(standing, 1, "a's probe was not released");
    let first = super::crash::standing_attempt(&at);
    let sealed: Value = serde_json::from_slice(&std::fs::read(&at).unwrap()).unwrap();
    assert_eq!(
        sealed["attempts"][&first]["cleanup"], "settling",
        "the crash did not land between b's cleanup and its settlement: {sealed}"
    );
    let resumed = resume_and_drive(
        dir.path(),
        compiled,
        &run,
        Clock::advancing(base() + time::Duration::seconds(1)).0,
    );
    assert_eq!(
        logs(dir.path(), "a"),
        ["probe", "probe", "attempt:succeed"],
        "the resume did not probe a again from the first"
    );
    assert_eq!(
        logs(dir.path(), "b"),
        ["probe", "probe", "attempt:succeed"],
        "the resume did not probe b again"
    );
    let started = started_payloads(&resumed);
    assert_eq!(started.len(), 2, "{started:?}");
    assert_eq!(started[1]["probes"].as_array().map(Vec::len), Some(2));
    let (standing, settled) = super::crash::reservation_after(&at);
    assert_eq!(standing, 0, "b's proven reservation was not released");
    assert!(settled.iter().any(|entry| entry == &first));
}

/// The child half of the live mid-round test: resume the shared run and
/// block in probe b, which its parent kills before it answers.
#[cfg(target_os = "linux")]
const PANEL_LIVE_DIR: &str = "BROKKR_430_PANEL_LIVE_DIR";
#[cfg(target_os = "linux")]
const PANEL_LIVE_RUN: &str = "BROKKR_430_PANEL_LIVE_RUN";

#[cfg(target_os = "linux")]
fn live_panel_child(dir: &Path, run: &str) -> ! {
    use crate::engine::broker::admission::tests::Coordinated;
    let home = dir.join("home");
    let _stand = Coordinated::at(&home);
    let body = two_route_panel(
        dir,
        &["succeed"],
        &["succeed"],
        Probed::Answer,
        Probed::Hang,
    );
    let mut engine = resume_run(
        dir,
        panel_bundle(dir, body),
        run,
        route_host(dir, (1, 10), (1, 10)),
    );
    engine.cooldown.clock = Clock::advancing(base()).0;
    engine.drive().unwrap();
    panic!("the engine was not killed in its probe");
}

/// The live mid-round fixture the two cleanup tests stand on: a killed
/// engine whose probe b driver still lives, the reservation it left, and
/// the fresh resume's park reason.
#[cfg(target_os = "linux")]
struct LiveMidRound {
    dir: tempfile::TempDir,
    engine: Engine,
    reason: String,
    at: PathBuf,
    attempt: String,
    driver: rustix::process::Pid,
    _stand: crate::engine::broker::admission::tests::Coordinated,
}

#[cfg(target_os = "linux")]
fn live_mid_round(test: &str) -> Option<LiveMidRound> {
    if let (Some(dir), Some(run)) = (
        std::env::var_os(PANEL_LIVE_DIR),
        std::env::var_os(PANEL_LIVE_RUN),
    ) {
        live_panel_child(Path::new(&dir), &run.to_string_lossy());
    }
    let (dir, home, stand) = super::crash::own_engine(test)?;
    let (run, compiled, _host) = crashed_panel(dir.path(), Probed::Answer, Probed::Hang);
    let pidlog = dir.path().join("b.pid");
    let done = dir.path().join("b.done");
    let mut child =
        super::crash::spawn_live(test, dir.path(), &run, PANEL_LIVE_DIR, PANEL_LIVE_RUN);
    super::crash::wait_for_probe(&mut child, &done, &pidlog, "probe b");
    child.kill().unwrap();
    child.wait().unwrap();
    let driver = std::fs::read_to_string(&pidlog)
        .unwrap()
        .trim()
        .parse::<i32>()
        .expect("probe b's pid");
    let driver = rustix::process::Pid::from_raw(driver).unwrap();
    assert!(
        rustix::process::test_kill_process(driver).is_ok(),
        "the crashed engine's probe b was not left live"
    );
    let at = crate::engine::broker::admission::tests::reservation_path(
        &home,
        &dir.path().join("work"),
        &run,
    );
    let attempt = super::crash::standing_attempt(&at);
    let engine = resume_and_drive(
        dir.path(),
        compiled,
        &run,
        Clock::advancing(base() + time::Duration::seconds(1)).0,
    );
    let reason = park_reason(&engine);
    Some(LiveMidRound {
        dir,
        engine,
        reason,
        at,
        attempt,
        driver,
        _stand: stand,
    })
}

/// While probe b's driver still lives, the resume finds its reservation and
/// parks with ruling 4's cleanup refusal: no second probe, no retry.
#[cfg(target_os = "linux")]
#[test]
fn a_live_mid_round_probe_parks_the_resume_with_the_cleanup_refusal() {
    let test = "engine::cooldown::tests::panel::a_live_mid_round_probe_parks_the_resume_with_the_cleanup_refusal";
    let Some(live) = live_mid_round(test) else {
        return;
    };
    let reason = &live.reason;
    assert!(
        reason.contains("did not prove its process tree gone"),
        "the resume did not park with the cleanup refusal: {reason}"
    );
    assert!(reason.contains(&live.at.display().to_string()), "{reason}");
    assert!(reason.contains(&live.attempt), "{reason}");
    assert_eq!(
        logs(live.dir.path(), "b"),
        ["probe"],
        "the resume probed over the live probe's reservation"
    );
    assert_eq!(starts(&live.engine).len(), 1, "the effect attempted again");
    assert_eq!(failed(&live.engine).len(), 1, "an effect attempt was spent");
    super::crash::kill_probe(live.driver);
}

/// The chief's R2 case: the standing reservation is probe b's, on
/// codex/second, while probe a on claude/first already answered and
/// released. The cleanup refusal names b's own route, the reservation's
/// attempt, not the first wait in the journal.
#[cfg(target_os = "linux")]
#[test]
fn chief_live_mid_round_cleanup_names_the_live_route() {
    let test = "engine::cooldown::tests::panel::chief_live_mid_round_cleanup_names_the_live_route";
    let Some(live) = live_mid_round(test) else {
        return;
    };
    assert!(
        live.reason
            .starts_with("the cool-down probe for codex/second"),
        "the cleanup refusal does not name the live reservation's route: {}",
        live.reason
    );
    assert_eq!(
        super::crash::reservation_after(&live.at).0,
        1,
        "the live reservation was released"
    );
    assert_eq!(logs(live.dir.path(), "b"), ["probe"]);
    assert_eq!(starts(&live.engine).len(), 1);
    assert_eq!(failed(&live.engine).len(), 1);
    super::crash::kill_probe(live.driver);
}

/// Resume an operator-retried indeterminate panel at `RequestEffect`.
fn retried_indeterminate(dir: &Path, waits: Value) -> Engine {
    let body = two_route_panel(
        dir,
        &["succeed"],
        &["succeed"],
        Probed::Answer,
        Probed::Answer,
    );
    let (run, compiled) = panel_indeterminate(dir, body, waits);
    let engine = resume_run(dir, compiled, &run, route_host(dir, (1, 2), (1, 2)));
    append_retry(&dir.join("forge.db"), &run);
    engine
}

#[test]
fn an_operator_retry_of_an_indeterminate_panel_parks_a_wait_past_its_maximum() {
    let dir = tempfile::tempdir().unwrap();
    let waits = json!([{
        "site": "b", "provider": "codex", "route": "second",
        "words": "rate_limit: slow down", "seconds": 5,
        "until": (base() + time::Duration::seconds(5)).format(&Rfc3339).unwrap(),
        "rounds": 1
    }]);
    let mut engine = retried_indeterminate(dir.path(), waits);
    engine.cooldown.clock = Clock::advancing(base()).0;
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("member b"), "{reason}");
    assert!(reason.contains("past the declared 2s maximum"), "{reason}");
    assert_eq!(logs(dir.path(), "b"), Vec::<String>::new());
}

#[test]
fn an_operator_retry_of_an_indeterminate_panel_honours_a_stop_during_the_wait() {
    let dir = tempfile::tempdir().unwrap();
    let waits = json!([{
        "site": "b", "provider": "codex", "route": "second",
        "words": "rate_limit: slow down", "seconds": 1,
        "until": (base() + time::Duration::seconds(2)).format(&Rfc3339).unwrap(),
        "rounds": 1
    }]);
    let mut engine = retried_indeterminate(dir.path(), waits);
    let db = dir.path().join("forge.db");
    let run = engine.run_id.clone();
    let now = std::sync::Arc::new(std::sync::Mutex::new(base()));
    engine.cooldown.clock = clock_acting(&now, Acting::AfterFirstSlice, move || {
        append_stop(&db, &run)
    });
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(fold(&events).unwrap().run.status(), Status::Stopped);
    assert_eq!(logs(dir.path(), "b"), Vec::<String>::new());
}

#[test]
fn an_operator_retry_with_an_unreadable_wait_parks_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = retried_indeterminate(dir.path(), malformed_earlier());
    engine.cooldown.clock = Clock::advancing(base()).0;
    engine.drive().unwrap();
    let reason = park_reason(&engine);
    assert!(reason.contains("refusing to execute effect"), "{reason}");
    assert!(reason.contains("does not read"), "{reason}");
}

#[test]
fn an_operator_retry_of_a_failed_panel_requests_the_next_effect_without_a_wait() {
    let dir = tempfile::tempdir().unwrap();
    let body = succeeding_panel(dir.path());
    let (run, compiled) = panel_journal(
        dir.path(),
        body,
        json!([wait_in("b", "codex", "second", 1)]),
    );
    let mut engine = resume_run(
        dir.path(),
        compiled,
        &run,
        route_host(dir.path(), (1, 10), (1, 10)),
    );
    // The failed event carries a refusal that parks, so the run is parked
    // and only the operator's retry moves it to RequestEffect.
    engine
        .append(
            EventType::RunParked,
            json!({"reason": "a member's route could not be resolved", "evidence": {}}),
            None,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(events.last().unwrap().event_type, EventType::RunParked);
    append_retry(&dir.path().join("forge.db"), &run);
    engine.cooldown.clock = Clock::advancing(base()).0;
    engine.drive().unwrap();
    let requested = engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .into_iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .count();
    assert_eq!(requested, 2, "the retry did not request the next effect");
    assert_eq!(
        logs(dir.path(), "b"),
        ["attempt:succeed"],
        "the new effect's attempt did not run"
    );
    assert!(
        start_probes(&engine).is_empty(),
        "a failed panel's wait was taken by a later effect"
    );
}

#[test]
fn an_indeterminate_panel_with_no_representable_end_writes_no_wait() {
    let dir = tempfile::tempdir().unwrap();
    let body = panel_of(vec![site_member("a", "claude", "first")]);
    let at = OffsetDateTime::parse("9999-12-31T23:59:50Z", &Rfc3339).unwrap();
    let mut engine = plain_engine(dir.path(), body, 2);
    engine.cooldown.clock = Clock::advancing(at).0;
    request_effect(&mut engine);
    let events = engine.store.load(&engine.run_id).unwrap();
    let endless: PreSessionRefusal = serde_json::from_value(json!({
        "class": "rate_limit", "provider": "claude", "route": "first",
        "words": "rate_limit: slow down", "retry_after_seconds": 1_000_000_000_000u64
    }))
    .unwrap();
    let terminal = engine.panel_rate_limit_terminal(
        &events,
        "effect",
        json!({"effect_id": "effect", "attempt_id": "a1", "reason": "indeterminate"}),
        &[RefusedMember {
            site: "a".into(),
            refusal: endless,
        }],
        false,
    );
    assert!(
        terminal.payload.get("waits").is_none(),
        "{}",
        terminal.payload
    );
    assert!(terminal.park.is_none());
}

#[test]
fn an_indeterminate_panel_with_an_unreadable_episode_follows_its_own_park() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = single_writer_engine(dir.path(), 2);
    append_malformed_episode(&mut engine);
    let events = engine.store.load(&engine.run_id).unwrap();
    let terminal = engine.panel_rate_limit_terminal(
        &events,
        "effect",
        json!({"effect_id": "effect", "attempt_id": "a1", "reason": "indeterminate"}),
        &[refused_at("a", "claude", Some("first"))],
        false,
    );
    assert!(
        terminal.payload.get("waits").is_none(),
        "{}",
        terminal.payload
    );
    assert!(
        terminal.payload.get("refused").is_none(),
        "{}",
        terminal.payload
    );
    assert!(terminal.park.is_none());
}

/// A panel whose first probe's admission is refused by a competing claim
/// parks naming the refusal, spending no effect attempt and starting no
/// later member's probe.
#[cfg(target_os = "linux")]
#[test]
fn a_refused_admission_at_a_member_probe_parks_naming_the_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = two_route_refusing_engine(dir.path(), Probed::Answer);
    let home = super::lease::home(dir.path());
    let worktree = dir.path().join("work");
    let now = std::sync::Arc::new(std::sync::Mutex::new(base()));
    let super::lease::Claimed { _stand, clock } =
        super::lease::claim_after_first_slice(home, worktree, &now);
    engine.cooldown.clock = clock;
    engine.drive().unwrap();
    assert_eq!(logs(dir.path(), "a"), ["attempt:refuse"]);
    assert_eq!(logs(dir.path(), "b"), ["attempt:refuse"]);
    assert_eq!(starts(&engine).len(), 1);
    let reason = park_reason(&engine);
    assert!(reason.contains("not admitted"), "{reason}");
    assert!(reason.contains("holds this worktree"), "{reason}");
}
