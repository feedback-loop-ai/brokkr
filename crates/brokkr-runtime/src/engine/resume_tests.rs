//! Handing a seat back the session its own earlier attempt opened
//! (decision 0030) — and refusing to hand it to anything else.
//!
//! The drivers here are real `forge-driver/v1` participants written in
//! `sh`: each logs every message the engine sends it, reports a session
//! id of its own per invocation, and answers. What the engine offered is
//! therefore read off the wire the driver actually saw, never inferred.

use super::*;
use crate::agents::Candidate;
use crate::bundle::{Limits, Seat};
use brokkr_core::policy::Machine;
use brokkr_protocol::{Body, Message};
use std::collections::BTreeMap;

/// work → review → work → done: the phase machine's own way back into a
/// seat, which is the re-entry the ruling names beside the retry.
fn machine() -> Machine {
    Machine::from_table(&json!({
        "phases":["work", "review", "done", "stop"],
        "initial":"work",
        "terminal":["done", "stop"],
        "rules":[
            {"id":"WORK", "from":"work", "result":"complete", "next":"review",
             "reason":"work"},
            {"id":"BACK", "from":"review", "result":"residual", "next":"work",
             "reason":"one more pass"},
            {"id":"DONE", "from":"review", "result":"clean", "next":"done",
             "reason":"clean"}
        ]
    }))
    .unwrap()
}

/// A driver that logs what it is sent, announces a session id of its own
/// on every invocation, and reads its verdicts from `results` — one per
/// invocation, the last repeating. `"start-failure"` fails without ever
/// accepting and without a checkpoint, which is the structural
/// fail-to-start decision 0016 walks a chain on.
fn driver(dir: &Path, tag: &str, results: &[&str]) -> Vec<String> {
    let capabilities = serde_json::to_string(&Message::new(Body::Capabilities {
        driver: "test".into(),
        version: "1".into(),
        supports: vec!["resume".into()],
    }))
    .unwrap();
    let mut script = String::from(
        "n=$(cat '@COUNT@' 2>/dev/null || echo 0)\n\
         n=$((n+1))\n\
         printf '%s' \"$n\" > '@COUNT@'\n\
         verdict=$(sed -n \"${n}p\" '@VERDICTS@')\n\
         [ -n \"$verdict\" ] || verdict=$(tail -1 '@VERDICTS@')\n\
         read -r line\n\
         printf '%s\\n' '@CAPABILITIES@'\n\
         while read -r line; do\n\
         printf '%s\\n' \"$line\" >> '@LOG@'\n\
         case \"$line\" in *start*) break ;; esac\n\
         done\n\
         eid=$(printf '%s' \"$line\" | sed 's/.*\"effect_id\":\"\\([^\"]*\\)\".*/\\1/')\n\
         aid=$(printf '%s' \"$line\" | sed 's/.*\"attempt_id\":\"\\([^\"]*\\)\".*/\\1/')\n\
         base='{\"proto\":\"forge-driver/v1\",\"msg_id\":\"m\"'\n\
         if [ \"$verdict\" = start-failure ]; then\n\
         printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
         \"status\":\"failed\",\"error\":\"no session opened\"}\\n' \"$base\" \"$eid\" \"$aid\"\n\
         read -r line\n\
         exit 0\n\
         fi\n\
         printf '%s,\"type\":\"accepted\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\"}\\n' \
         \"$base\" \"$eid\" \"$aid\"\n\
         printf '%s,\"type\":\"checkpoint\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
         \"data\":{\"step\":\"session-started\",\"session_id\":\"@TAG@-%s\"}}\\n' \
         \"$base\" \"$eid\" \"$aid\" \"$n\"\n\
         if [ \"$verdict\" = fail ]; then\n\
         printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
         \"status\":\"failed\",\"error\":\"a first look\"}\\n' \"$base\" \"$eid\" \"$aid\"\n\
         else\n\
         printf '%s,\"type\":\"result\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
         \"status\":\"succeeded\",\"result\":{\"result\":\"'\"$verdict\"'\"}}\\n' \
         \"$base\" \"$eid\" \"$aid\"\n\
         fi\n\
         read -r line\n",
    );
    std::fs::write(dir.join(format!("{tag}.verdicts")), results.join("\n")).unwrap();
    // The paths ride inside an `sh` script: Windows spells them with
    // backslashes, which `sh` eats — forward-slashed here and quoted in
    // the script, one spelling works on every leg.
    let spelled = |name: &str| dir.join(name).display().to_string().replace('\\', "/");
    for (marker, value) in [
        ("@CAPABILITIES@", capabilities),
        ("@LOG@", spelled(&format!("{tag}.log"))),
        ("@COUNT@", spelled(&format!("{tag}.count"))),
        ("@VERDICTS@", spelled(&format!("{tag}.verdicts"))),
        ("@TAG@", tag.to_string()),
    ] {
        script = script.replace(marker, &value);
    }
    vec!["sh".into(), "-c".into(), script]
}

/// Every message one driver was sent, in order.
fn received(dir: &Path, tag: &str) -> Vec<Value> {
    std::fs::read_to_string(dir.join(format!("{tag}.log")))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn seat(body: SeatBody, results: &[&str], max_attempts: u64) -> Seat {
    Seat {
        has_gate: false,
        results: results.iter().map(|r| r.to_string()).collect(),
        limits: Limits {
            max_attempts,
            timeout_seconds: 10,
        },
        inputs: Vec::new(),
        secrets: Vec::new(),
        body,
    }
}

fn single(command: Vec<String>, candidates: Vec<Candidate>) -> SeatBody {
    SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command,
        confine: None,
        candidates,
    }
}

fn bundle(dir: &Path, seats: BTreeMap<String, Seat>) -> Bundle {
    Bundle {
        dialect_prompts: Default::default(),
        name: "resume".into(),
        description: String::new(),
        cost: String::new(),
        dir: dir.to_path_buf(),
        roots: vec![dir.to_path_buf()],
        chain: Vec::new(),
        machine: machine(),
        seats,
        manifest: json!({
            "engine":ENGINE_VERSION, "event_schema":1, "database_schema":1,
            "driver_protocol":1, "bundle_name":"resume",
            "files":{"bundle.json":"b".repeat(64)}
        }),
        protected_phase: "review".into(),
        hands: std::collections::BTreeMap::new(),
    }
}

fn run(dir: &Path, bundle: Bundle) -> Vec<EventEnvelope> {
    std::fs::create_dir_all(dir.join("work")).unwrap();
    let store = Store::open(&dir.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "resume", Some(dir.join("work"))).unwrap();
    engine.drive().unwrap();
    engine.store.load(&engine.run_id).unwrap()
}

/// Which sessions the engine offered, in journal order, and whether it
/// said so on the attempt's own start event.
fn offers(received: &[Value]) -> Vec<Option<String>> {
    let mut offers = Vec::new();
    let mut pending: Option<String> = None;
    for message in received {
        match message["type"].as_str() {
            Some("resume") => {
                pending = Some(message["session_ref"].as_str().unwrap().to_string());
            }
            Some("start") => offers.push(pending.take()),
            _ => {}
        }
    }
    offers
}

/// The round trip, both ways the ruling allows it: a retry of the same
/// seat resumes the thread its first attempt opened, and the seat the
/// phase machine sends back into resumes the thread it last held —
/// never an older one, and never on the attempt that opened the first.
#[test]
fn a_retry_and_a_re_entry_both_resume_the_thread_the_seat_last_held() {
    let dir = tempfile::tempdir().unwrap();
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            single(
                driver(dir.path(), "work", &["fail", "complete"]),
                Vec::new(),
            ),
            &["complete"],
            2,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(
                driver(dir.path(), "review", &["residual", "clean"]),
                Vec::new(),
            ),
            &["residual", "clean"],
            1,
        ),
    );
    let events = run(dir.path(), bundle(dir.path(), seats));

    // Three invocations of the work seat: the first attempt, the retry
    // that follows its failure, and the re-entry after review sent the
    // run back. The retry is offered what the first attempt opened; the
    // re-entry is offered what the retry opened — the LAST session the
    // seat held, not the first.
    assert_eq!(
        offers(&received(dir.path(), "work")),
        [None, Some("work-1".into()), Some("work-2".into())]
    );
    // The review seat opened sessions of its own and was offered its
    // own on its second visit: seats do not borrow each other's.
    assert_eq!(
        offers(&received(dir.path(), "review")),
        [None, Some("review-1".into())]
    );

    // The journal gained no field for any of it: the offer is derived
    // from what is already there, and the driver's own checkpoints say
    // what became of it. A start event reads exactly as it always did.
    let started: Vec<&Value> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectStarted)
        .map(|event| &event.payload)
        .collect();
    assert_eq!(started.len(), 5, "work, retry, review, re-entry, review");
    for payload in started {
        let mut keys: Vec<&String> = payload.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(keys, ["attempt_id", "driver", "effect_id"], "{payload}");
    }
    assert_eq!(
        fold(&events).unwrap().status,
        Status::Completed,
        "the run still reaches its own end"
    );
}

/// A decision-0016 chain fallback is a different instance, and a session
/// is one model's memory of one tree. When the seat's candidate moves
/// between the attempt that opened a thread and the one asking for it,
/// NO session_ref is handed over — this is provider policy (a session
/// belongs to the credential and client that opened it) before it is
/// ours, so the suppression is proved on the wire, not argued.
#[test]
fn a_chain_fallback_is_handed_no_session_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let candidate = |model: &str, command: Vec<String>| Candidate {
        agent: "implementer".into(),
        model: model.into(),
        effort: Some("medium".into()),
        provider: "codex".into(),
        argv: command,
    };
    // The first link fails to START on its first invocation and behaves
    // on every one after it: that is what lets the second link open a
    // thread on one effect and the first link run on the next.
    let first = driver(dir.path(), "first", &["start-failure", "complete"]);
    let second = driver(dir.path(), "second", &["complete"]);
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            single(
                first.clone(),
                vec![
                    candidate("sol", first.clone()),
                    candidate("terra", second.clone()),
                ],
            ),
            &["complete"],
            2,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(
                driver(dir.path(), "review", &["residual", "clean"]),
                Vec::new(),
            ),
            &["residual", "clean"],
            1,
        ),
    );
    let events = run(dir.path(), bundle(dir.path(), seats));

    // The second link opened `second-1` and finished the first visit.
    assert_eq!(offers(&received(dir.path(), "second")), [None]);
    // The first link ran twice — the attempt that failed to start, and
    // the re-entry that resolved back to it — and was offered nothing
    // either time. It never opened `second-1`, so it is never handed it.
    assert_eq!(offers(&received(dir.path(), "first")), [None, None]);
    assert!(
        std::fs::read_to_string(dir.path().join("first.log"))
            .unwrap()
            .find("second-1")
            .is_none(),
        "the thread the other link opened never reached this one, in any message"
    );
    assert_eq!(fold(&events).unwrap().status, Status::Completed);
}

/// A run whose journal has moved to another machine is offered nothing,
/// on the wire, even though every fact INSIDE the chain still agrees.
///
/// Both halves matter and both are here. An operator's retry after a
/// park is a second engine process by definition, and it still resumes:
/// the session survives the process that opened it, because what the
/// offer rests on is durable. And the same run, continued from a journal
/// that is no longer where it started — adopted under decision 0027,
/// or a `.db` carried to another machine — resumes nothing, because a
/// provider session belongs to the credential that opened it.
#[test]
fn a_journal_that_moved_is_offered_nothing_and_one_that_stayed_still_is() {
    // One seat, one attempt, failing: the run parks with a session
    // opened and an operator holding the next move.
    let parked = |dir: &Path, moved: bool| {
        std::fs::create_dir_all(dir.join("work")).unwrap();
        let mut seats = BTreeMap::new();
        seats.insert(
            "work".into(),
            seat(
                single(driver(dir, "work", &["fail", "complete"]), Vec::new()),
                &["complete"],
                1,
            ),
        );
        seats.insert(
            "review".into(),
            seat(
                single(driver(dir, "review", &["clean"]), Vec::new()),
                &["clean"],
                1,
            ),
        );
        let db = dir.join("forge.db");
        let store = Store::open(&db).unwrap();
        let bundle = bundle(dir, seats);
        let mut engine =
            Engine::start(store, bundle.clone(), "resume", Some(dir.join("work"))).unwrap();
        let run_id = engine.run_id.clone();
        engine.drive().unwrap();
        assert_eq!(
            fold(&engine.store.load(&run_id).unwrap()).unwrap().status,
            Status::AwaitingOperator,
            "the failed attempt parked the run"
        );

        // The journal is carried elsewhere — the row travels with the
        // file, the machine does not. A second connection, because this
        // is exactly the tampering the store is asked to notice.
        if moved {
            rusqlite::Connection::open(&db)
                .unwrap()
                .execute("UPDATE runs SET origin_host = 'elsewhere'", [])
                .unwrap();
        }

        let mut store = engine.store;
        operator_command(&mut store, &run_id, "retry", "operator", "once more").unwrap();
        Engine::resume(store, bundle, &run_id, Some(dir.join("work")))
            .unwrap()
            .drive()
            .unwrap();
        offers(&received(dir, "work"))
    };

    let stayed = tempfile::tempdir().unwrap();
    assert_eq!(
        parked(stayed.path(), false),
        [None, Some("work-1".into())],
        "a retry across a park still rejoins the thread its seat opened"
    );

    let travelled = tempfile::tempdir().unwrap();
    assert_eq!(
        parked(travelled.path(), true),
        [None, None],
        "the same journal, somewhere else, is handed nothing"
    );
}

/// The offer's own predicate, one refusal at a time. Everything here is
/// a hand-built journal because the point is what the engine reads out
/// of one — a run whose bundle has moved under it cannot be produced by
/// driving, and that is exactly the case that must fail closed.
#[test]
fn every_fact_the_offer_rests_on_can_refuse_it_alone() {
    let dir = tempfile::tempdir().unwrap();
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(single(vec!["driver".into()], Vec::new()), &["complete"], 1),
    );
    let bundle = bundle(dir.path(), seats);
    let manifest = bundle.manifest.clone();
    let started = json!({
        "effect_id":"fx", "attempt_id":"a1", "driver":"driver",
        "provenance":[{"member":null, "agent":"implementer", "model":"sol",
                       "provider":"codex", "chain_index":0}],
    });
    let events = |seat: &str, started: &Value| {
        vec![
            envelope(EventType::RunStarted, json!({"manifest": manifest}), None),
            envelope(
                EventType::EffectRequested,
                json!({"effect_id":"fx", "seat": seat}),
                None,
            ),
            envelope(EventType::EffectStarted, started.clone(), Some("a1")),
            envelope(
                EventType::EffectCheckpointed,
                json!({"effect_id":"fx", "attempt_id":"a1",
                "checkpoint":{"step":"transcript", "transcript":{
                    "kind":"codex-thread", "locator":"thread-1",
                    "home":"/test/.codex"
                }}}),
                Some("a1"),
            ),
        ]
    };

    // Every fact agrees: the session is offered.
    assert_eq!(
        resume_offer(&events("work", &started), &bundle, "work", &started, true),
        Some("thread-1".into())
    );

    // The same journal, read for a seat that never opened it.
    assert_eq!(
        resume_offer(&events("other", &started), &bundle, "work", &started, true),
        None
    );

    // A driver binary that moved between the attempts.
    let mut moved = started.clone();
    moved["driver"] = json!("another-driver");
    assert_eq!(
        resume_offer(&events("work", &started), &bundle, "work", &moved, true),
        None
    );

    // A candidate that moved — the chain fallback, at the predicate.
    let mut fell_back = started.clone();
    fell_back["provenance"][0]["model"] = json!("terra");
    assert_eq!(
        resume_offer(&events("work", &started), &bundle, "work", &fell_back, true),
        None
    );

    // A bundle that moved under the run: an adapter edited, a charter
    // rewritten, an engine upgraded. The pin the run took at its first
    // event no longer describes what is about to spawn, so nothing is
    // handed over.
    let mut edited = bundle.clone();
    edited.manifest["files"]["adapters/codex.json"] = json!("c".repeat(64));
    assert_eq!(
        resume_offer(&events("work", &started), &edited, "work", &started, true),
        None
    );

    // A journal with no run/started to pin anything, and one whose
    // attempt journaled no session at all.
    assert_eq!(resume_offer(&[], &bundle, "work", &started, true), None);
    assert_eq!(
        resume_offer(
            &events("work", &started)[..3],
            &bundle,
            "work",
            &started,
            true
        ),
        None
    );

    // A session whose opening attempt left no start event to be judged
    // by is no session anyone may be handed.
    let mut orphaned = events("work", &started);
    orphaned.remove(2);
    assert_eq!(
        resume_offer(&orphaned, &bundle, "work", &started, true),
        None
    );

    // Every journaled fact agrees and the journal is somewhere else: a
    // run adopted from another machine (decision 0027), a journal file
    // copied to one, or an installation that cannot say where it is. The
    // chain cannot tell any of those apart — by 0027's design — so the
    // store answers instead, and its `false` ends the offer before any
    // of the above is even asked.
    assert_eq!(
        resume_offer(&events("work", &started), &bundle, "work", &started, false),
        None,
        "a session handle never crosses a machine or an account"
    );
}

/// One journaled event, hand-built: the store seals real ones, and these
/// are fixtures for a predicate that only reads.
fn envelope(event_type: EventType, payload: Value, attempt_id: Option<&str>) -> EventEnvelope {
    EventEnvelope {
        run_id: "run".into(),
        seq: 1,
        event_id: "event".into(),
        event_schema_version: 1,
        event_type,
        payload,
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: attempt_id.map(str::to_string),
        recorded_at: "2026-09-02T00:00:00Z".into(),
        previous_hash: brokkr_core::canonical::ZERO_HASH.into(),
        event_hash: "a".repeat(64),
    }
}
