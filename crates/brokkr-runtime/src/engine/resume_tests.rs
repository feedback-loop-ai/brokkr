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

/// The same driver, emitting the row an engine enacting proposed
/// decision 0056 actually stamps: a launch checkpoint naming a model —
/// which is what makes the engine write `site_ref` and `instance_ref`
/// onto it — and the provider-confirmed root the next offer stands on.
///
/// The legacy shim above carries neither, which is exactly the
/// difference the two halves of the query are for: an unstamped row is
/// decision 0030's evidence and only a single work seat may be offered
/// one, while a stamped row belongs to one structural site and is
/// offered only there.
fn model_driver(dir: &Path, tag: &str, results: &[&str]) -> Vec<String> {
    let command = driver(dir, tag, results);
    let legacy = format!("\"data\":{{\"step\":\"session-started\",\"session_id\":\"{tag}-%s\"}}");
    let stamped = format!(
        "\"data\":{{\"step\":\"harness-started\",\"model\":\"claude-opus-5\",\
         \"launch\":\"cold\",\"root_session\":{{\"kind\":\"claude-session\",\
         \"id\":\"{tag}-%s\",\"harness_version\":\"2.1.266\",\"persistent\":true}}}}"
    );
    let script = command[2].replace(&legacy, &stamped);
    assert_ne!(script, command[2], "the {tag} shim must emit a stamped row");
    vec![command[0].clone(), command[1].clone(), script]
}

/// The same stamped driver, but emitting the row a DSH attempt writes: a
/// `dsh-session` root carrying the certified harness version and the
/// optional wrapper digest, plus the `transcript` object whose `home` and
/// `locator` are the other two coordinates of the owned target. The
/// invocation counter still mints `<tag>-<n>`, so the offered id is the
/// one the first attempt opened.
///
/// The checkpoint is serialized here, once per declared invocation, into
/// a JSONL data file beside the verdicts. The shim selects its indexed
/// row and emits it as a `%s` argument in a fixed format, so a Windows
/// backslash, a percent sign, a quote or an embedded newline in the home
/// or locator is payload rather than shell source or a `printf` directive
/// (R2). A missing row fails the shim explicitly; it never repeats the
/// previous checkpoint.
fn dsh_model_driver(
    dir: &Path,
    tag: &str,
    results: &[&str],
    locator: &str,
    home: &str,
    invocations: usize,
) -> Vec<String> {
    let command = driver(dir, tag, results);
    let digest = "a".repeat(64);
    let mut rows = String::new();
    for n in 1..=invocations {
        let data = json!({
            "step": "harness-started",
            "model": "deepseek-v4-flash",
            "launch": "cold",
            "root_session": {
                "kind": "dsh-session",
                "id": format!("{tag}-{n}"),
                "harness_version": "0.1.5-rc.1",
                "wrapper_digest": digest,
                "persistent": true,
            },
            "transcript": {
                "kind": "dsh-session",
                "locator": locator,
                "home": home,
            },
        });
        rows.push_str(&serde_json::to_string(&data).unwrap());
        rows.push('\n');
    }
    std::fs::write(dir.join(format!("{tag}.checkpoints")), rows).unwrap();
    // The shim reads this path out of its script text, so it is spelled the
    // same forward-slashed way the other fixture paths are.
    let checkpoints = dir
        .join(format!("{tag}.checkpoints"))
        .display()
        .to_string()
        .replace('\\', "/");
    let legacy_closed =
        format!("\"data\":{{\"step\":\"session-started\",\"session_id\":\"{tag}-%s\"}}}}");
    let script = command[2]
        .replace(
            "printf '%s,\"type\":\"checkpoint\"",
            &format!(
                "ckpt=$(sed -n \"${{n}}p\" '{checkpoints}')\n\
                 [ -n \"$ckpt\" ] || {{ echo 'dsh shim: no checkpoint row for this invocation' >&2; exit 1; }}\n\
                 printf '%s,\"type\":\"checkpoint\""
            ),
        )
        .replace(&legacy_closed, "\"data\":%s}")
        .replace(
            "\"$base\" \"$eid\" \"$aid\" \"$n\"",
            "\"$base\" \"$eid\" \"$aid\" \"$ckpt\"",
        );
    assert_ne!(
        script, command[2],
        "the {tag} shim must emit a data-row checkpoint"
    );
    vec![command[0].clone(), command[1].clone(), script]
}

fn seat(body: SeatBody, results: &[&str], max_attempts: u64) -> Seat {
    gate_seat(body, results, max_attempts, false)
}

fn gate_seat(body: SeatBody, results: &[&str], max_attempts: u64, gate: bool) -> Seat {
    Seat {
        has_gate: gate,
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

fn member(name: &str, command: Vec<String>) -> PanelMember {
    PanelMember {
        name: name.into(),
        role_path: PathBuf::from("role.md"),
        command,
        candidates: Vec::new(),
    }
}

fn panel(members: Vec<PanelMember>) -> SeatBody {
    SeatBody::Panel {
        members,
        aggregate: Aggregate::UnanimousPass,
    }
}

fn single(command: Vec<String>, candidates: Vec<Candidate>) -> SeatBody {
    SeatBody::Single {
        role_path: PathBuf::from("role.md"),
        command,
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
        boundary: Boundary::Namespace,
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
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
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

/// A machine whose work seat emits `pass`/`fail`, so a panel joined by
/// `unanimous-pass` can stand at it. `review` sends the run back into
/// `work` once, which is the phase-machine re-entry the ruling names
/// beside the retry.
fn panel_machine() -> Machine {
    Machine::from_table(&json!({
        "phases":["work", "review", "done", "stop"],
        "initial":"work",
        "terminal":["done", "stop"],
        "rules":[
            {"id":"WORK", "from":"work", "result":"pass", "next":"review",
             "reason":"the panel agreed"},
            {"id":"WORK-FAIL", "from":"work", "result":"fail", "next":"review",
             "reason":"the panel did not"},
            {"id":"BACK", "from":"review", "result":"residual", "next":"work",
             "reason":"one more pass"},
            {"id":"DONE", "from":"review", "result":"clean", "next":"done",
             "reason":"clean"}
        ]
    }))
    .unwrap()
}

/// Every work topology of proposed decision 0056 ruling 1 is offered its
/// OWN session, and no site is offered another's.
///
/// The offers are read off the wire each driver actually saw, so what is
/// asserted is what the engine sent, not what it meant to. Before this
/// ruling every one of these sites passed `session_ref: None`: panel
/// members and sequence model steps were the three call sites issue #226
/// names, and they spawned cold however warm their own history was.
#[test]
fn every_work_topology_is_offered_its_own_session_and_no_other() {
    let dir = tempfile::tempdir().unwrap();
    let mut seats = BTreeMap::new();
    // A seat-level work panel: two members, each with its own history.
    seats.insert(
        "work".into(),
        seat(
            panel(vec![
                member("alpha", model_driver(dir.path(), "alpha", &["pass"])),
                member("beta", model_driver(dir.path(), "beta", &["pass"])),
            ]),
            &["pass", "fail"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(
                model_driver(dir.path(), "review", &["residual", "clean"]),
                Vec::new(),
            ),
            &["residual", "clean"],
            1,
        ),
    );
    let mut bundle = bundle(dir.path(), seats);
    bundle.machine = panel_machine();
    let events = run(dir.path(), bundle);

    // Each member is offered ONLY the session it opened itself, on the
    // re-entry — never the other's, and never on the first invocation.
    for tag in ["alpha", "beta"] {
        assert_eq!(
            offers(&received(dir.path(), tag)),
            [None, Some(format!("{tag}-1"))],
            "{tag} is offered its own session and only its own"
        );
        let log = std::fs::read_to_string(dir.path().join(format!("{tag}.log"))).unwrap();
        let sibling = if tag == "alpha" { "beta" } else { "alpha" };
        assert!(
            !log.contains(sibling),
            "{tag} was never told anything about {sibling}"
        );
    }
    // The aggregate carries no member's stamp: it is the engine's own
    // record and establishes no session ownership.
    let aggregates: Vec<&Value> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectSucceeded)
        .map(|event| &event.payload["result"])
        .collect();
    for aggregate in aggregates {
        assert!(
            aggregate.get("site_ref").is_none() && aggregate.get("instance_ref").is_none(),
            "an aggregate never establishes a member's ownership: {aggregate}"
        );
    }
    assert_eq!(fold(&events).unwrap().status, Status::Completed);
}

/// The same question of a sequence: a work-class model step and each
/// member of a work-class panel step is offered its own session, while
/// the deterministic validator beside them gets no offer, no negotiation
/// and no model launch at all.
#[test]
fn a_sequence_offers_its_work_steps_and_members_and_nothing_to_a_validator() {
    let dir = tempfile::tempdir().unwrap();
    let author = SequenceStep {
        name: "author".into(),
        class: SeatClass::Work,
        results: vec!["drafted".into()],
        body: StepBody::Single {
            role_path: PathBuf::from("role.md"),
            command: model_driver(dir.path(), "author", &["drafted"]),
            candidates: Vec::new(),
        },
    };
    let validator = SequenceStep {
        name: "validate".into(),
        class: SeatClass::Work,
        results: vec!["drafted".into()],
        body: StepBody::Single {
            role_path: PathBuf::from("role.md"),
            command: driver(dir.path(), "validate", &["drafted"]),
            candidates: Vec::new(),
        },
    };
    let council = SequenceStep {
        name: "council".into(),
        class: SeatClass::Work,
        results: vec!["pass".into(), "fail".into()],
        body: StepBody::Panel {
            members: vec![
                member("alpha", model_driver(dir.path(), "step-alpha", &["pass"])),
                member("beta", model_driver(dir.path(), "step-beta", &["pass"])),
            ],
            aggregate: Aggregate::UnanimousPass,
        },
    };
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            SeatBody::Sequence {
                steps: vec![author, validator, council],
            },
            &["pass", "fail"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(
                model_driver(dir.path(), "review", &["residual", "clean"]),
                Vec::new(),
            ),
            &["residual", "clean"],
            1,
        ),
    );
    let mut bundle = bundle(dir.path(), seats);
    bundle.machine = panel_machine();
    let events = run(dir.path(), bundle);

    for tag in ["author", "step-alpha", "step-beta"] {
        assert_eq!(
            offers(&received(dir.path(), tag)),
            [None, Some(format!("{tag}-1"))],
            "{tag} is a work model site and rejoins its own session"
        );
    }
    // The validator emits no stamped launch, so it has nothing to be
    // offered — and it is offered nothing on either visit.
    assert_eq!(offers(&received(dir.path(), "validate")), [None, None]);
    // A label reused under a different parent never crosses: the
    // sequence's `alpha` and the seat panel's `alpha` are two sites, and
    // the digest that keys them is structural rather than a flat tag.
    for tag in ["step-alpha", "step-beta"] {
        let log = std::fs::read_to_string(dir.path().join(format!("{tag}.log"))).unwrap();
        assert!(
            !log.contains("author-1"),
            "{tag} was never offered the author's"
        );
    }
    assert_eq!(fold(&events).unwrap().status, Status::Completed);
}

/// Decision 0042 ruling 2, bounding decision 0030: every gate-class model
/// site starts fresh and blind, at every topology, and the single gate
/// that the shipped code offered a session is corrected rather than
/// copied.
#[test]
fn no_gate_topology_is_ever_offered_a_session() {
    for case in [
        "single gate",
        "gate panel",
        "gate model step and a member of a gate panel step",
    ] {
        // Each case gets its own directory so each driver's log is its
        // own, and the shims are built against that directory.
        let dir = tempfile::tempdir().unwrap();
        let body = match case {
            "single gate" => single(
                model_driver(dir.path(), "single", &["pass", "pass"]),
                Vec::new(),
            ),
            "gate panel" => panel(vec![
                member("alpha", model_driver(dir.path(), "panel-alpha", &["pass"])),
                member("beta", model_driver(dir.path(), "panel-beta", &["pass"])),
            ]),
            _ => SeatBody::Sequence {
                steps: vec![
                    SequenceStep {
                        name: "judge".into(),
                        class: SeatClass::Gate,
                        results: vec!["pass".into()],
                        body: StepBody::Single {
                            role_path: PathBuf::from("role.md"),
                            command: model_driver(dir.path(), "step-judge", &["pass"]),
                            candidates: Vec::new(),
                        },
                    },
                    SequenceStep {
                        name: "bench".into(),
                        class: SeatClass::Gate,
                        results: vec!["pass".into(), "fail".into()],
                        body: StepBody::Panel {
                            members: vec![
                                member("alpha", model_driver(dir.path(), "bench-alpha", &["pass"])),
                                member("beta", model_driver(dir.path(), "bench-beta", &["pass"])),
                            ],
                            aggregate: Aggregate::UnanimousPass,
                        },
                    },
                ],
            },
        };
        let mut seats = BTreeMap::new();
        seats.insert("work".into(), gate_seat(body, &["pass", "fail"], 1, true));
        seats.insert(
            "review".into(),
            seat(
                single(
                    model_driver(dir.path(), "review", &["residual", "clean"]),
                    Vec::new(),
                ),
                &["residual", "clean"],
                1,
            ),
        );
        let mut bundle = bundle(dir.path(), seats);
        bundle.machine = panel_machine();
        let events = run(dir.path(), bundle);

        for tag in [
            "single",
            "panel-alpha",
            "panel-beta",
            "step-judge",
            "bench-alpha",
            "bench-beta",
        ] {
            let seen = received(dir.path(), tag);
            if seen.is_empty() {
                continue;
            }
            assert!(
                offers(&seen).iter().all(Option::is_none),
                "{case}/{tag}: a judge reads the artifact afresh, never a prior \
                 provider conversation: {seen:?}"
            );
        }
        // Its own history is there and eligible on every other axis: the
        // ONLY reason it is not offered is that it judges.
        assert!(
            events.iter().any(|event| {
                event.event_type == EventType::EffectCheckpointed
                    && event.payload.pointer("/checkpoint/root_session").is_some()
            }),
            "{case}: the gate did record a confirmed root, and was still offered nothing"
        );
        // The launch a fresh gate accepts is cold with no refusal
        // reason, because no offer was made to refuse.
        for event in &events {
            if let Some(launch) = event.payload.pointer("/checkpoint/launch") {
                assert_eq!(launch, "cold", "{case}");
                assert!(
                    event
                        .payload
                        .pointer("/checkpoint/resume_refusal")
                        .is_none(),
                    "{case}: no offer, so no reason"
                );
            }
        }
    }
}

/// Proposed decision 0056 ruling 2's recovery half: a FRESH engine
/// process derives the same offer from the same journal, the same pinned
/// bundle and the same local origin.
///
/// There is nothing in memory for it to depend on, which is the point —
/// an operator's retry after a park is a second process by definition,
/// and every one of these sites has to reach the same answer its
/// predecessor would have.
#[test]
fn a_fresh_process_derives_the_same_offers_from_the_same_journal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            panel(vec![
                member(
                    "alpha",
                    model_driver(dir.path(), "alpha", &["pass", "fail"]),
                ),
                member("beta", model_driver(dir.path(), "beta", &["fail", "pass"])),
            ]),
            &["pass", "fail"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(model_driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["residual", "clean"],
            1,
        ),
    );
    let mut bundle = bundle(dir.path(), seats);
    bundle.machine = panel_machine();

    // One process opens each member's session and parks on the failing
    // member's verdict.
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    let mut engine = Engine::start(
        store,
        bundle.clone(),
        "resume",
        Some(dir.path().join("work")),
    )
    .unwrap();
    let run_id = engine.run_id.clone();
    engine.drive().unwrap();
    let mut store = engine.store;
    let first = offers(&received(dir.path(), "alpha"));

    // A second process, with nothing of the first in memory.
    operator_command(&mut store, &run_id, "retry", "operator", "once more").unwrap();
    Engine::resume(store, bundle, &run_id, Some(dir.path().join("work")))
        .unwrap()
        .drive()
        .unwrap();

    let after = offers(&received(dir.path(), "alpha"));
    assert_eq!(&after[..first.len()], &first[..], "the record did not move");
    for tag in ["alpha", "beta"] {
        let seen = offers(&received(dir.path(), tag));
        assert_eq!(seen[0], None, "{tag}'s first invocation was cold");
        assert_eq!(
            seen[1],
            Some(format!("{tag}-1")),
            "{tag}'s second invocation, in another process, rejoins its own"
        );
    }
}

/// A work-class site context whose stamps are the ones the fixture rows
/// below carry, so the query has something to match.
fn context(site_ref: &str, instance_ref: &str, class: SeatClass) -> resume::SiteContext {
    resume::SiteContext {
        site_ref: site_ref.to_string(),
        instance_ref: instance_ref.to_string(),
        class,
    }
}

const SITE_A: &str = "aa11000000000000000000000000000000000000000000000000000000000011";
const SITE_B: &str = "bb22000000000000000000000000000000000000000000000000000000000022";
const OWNER: &str = "cc33000000000000000000000000000000000000000000000000000000000033";
const OTHER_OWNER: &str = "dd44000000000000000000000000000000000000000000000000000000000044";

/// The offer's own predicate, one refusal at a time. Everything here is
/// a hand-built journal because the point is what the engine reads out
/// of one — a run whose bundle has moved under it cannot be produced by
/// driving, and that is exactly the case that must fail closed.
///
/// This exercises the LEGACY half of the query (decision 0030's
/// evidence, kept readable by proposed 0056 ruling 9): an unstamped
/// codex row, offered only to a single work seat, judged by the `driver`
/// label and the `provenance` of the attempt that wrote it.
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
    let key = resume::SiteKey::single("work", None);
    let work = context(SITE_A, OWNER, SeatClass::Work);
    let offer = |events: &[EventEnvelope], seat: &str, started: &Value, here: bool, holds: bool| {
        offer_for_site(events, &key, &work, seat, here, holds, started)
            .map(|target| target.provider_id)
    };

    // Every fact agrees: the session is offered.
    assert_eq!(
        offer(&events("work", &started), "work", &started, true, true),
        Some("thread-1".into())
    );

    // The same journal, read for a seat that never opened it.
    assert_eq!(
        offer(&events("other", &started), "work", &started, true, true),
        None
    );

    // A driver binary that moved between the attempts.
    let mut moved = started.clone();
    moved["driver"] = json!("another-driver");
    assert_eq!(
        offer(&events("work", &started), "work", &moved, true, true),
        None
    );

    // A candidate that moved — the chain fallback, at the predicate.
    let mut fell_back = started.clone();
    fell_back["provenance"][0]["model"] = json!("terra");
    assert_eq!(
        offer(&events("work", &started), "work", &fell_back, true, true),
        None
    );

    // A bundle that moved under the run: an adapter edited, a charter
    // rewritten, an engine upgraded. The pin the run took at its first
    // event no longer describes what is about to spawn, so nothing is
    // handed over. The engine answers that question before it asks this
    // one, and hands the answer down.
    assert_eq!(
        offer(&events("work", &started), "work", &started, true, false),
        None
    );

    // A journal with no run/started to pin anything, and one whose
    // attempt journaled no session at all — the shape decision 0053
    // ruling 8 puts a window around: a driver holds its harness's
    // locator until work begins, so an attempt killed before its first
    // turn leaves exactly this journal and its retry starts cold.
    assert_eq!(offer(&[], "work", &started, true, true), None);
    assert_eq!(
        offer(&events("work", &started)[..3], "work", &started, true, true),
        None
    );

    // A session whose opening attempt left no start event to be judged
    // by is no session anyone may be handed.
    let mut orphaned = events("work", &started);
    orphaned.remove(2);
    assert_eq!(offer(&orphaned, "work", &started, true, true), None);

    // Every journaled fact agrees and the journal is somewhere else: a
    // run adopted from another machine (decision 0027), a journal file
    // copied to one, or an installation that cannot say where it is. The
    // chain cannot tell any of those apart — by 0027's design — so the
    // store answers instead, and its `false` ends the offer before any
    // of the above is even asked.
    assert_eq!(
        offer(&events("work", &started), "work", &started, false, true),
        None,
        "a session handle never crosses a machine or an account"
    );

    // A GATE site with exactly the same journal is offered nothing
    // (decision 0042 ruling 2, bounding decision 0030): the single
    // gate's old offer path is removed, not carried forward.
    let judge = context(SITE_A, OWNER, SeatClass::Gate);
    assert_eq!(
        offer_for_site(
            &events("work", &started),
            &key,
            &judge,
            "work",
            true,
            true,
            &started
        ),
        None,
        "a judge is fresh and blind, and reads the artifact instead"
    );

    // A composite site never reaches the legacy path at all: an old
    // row's ancestry was never journaled, so no colon tag is split to
    // guess it.
    let member = resume::SiteKey::panel_member("work", None, "alpha", 0);
    assert_eq!(
        offer_for_site(
            &events("work", &started),
            &member,
            &work,
            "work",
            true,
            true,
            &started
        ),
        None
    );
}

/// The stamped half of the query — the evidence this change's own engine
/// writes (proposed decision 0056 rulings 2 and 3). Every term is
/// exercised alone against one journal.
#[test]
fn a_stamped_row_is_offered_only_to_its_own_site_owner_and_persistent_root() {
    let dir = tempfile::tempdir().unwrap();
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(single(vec!["driver".into()], Vec::new()), &["complete"], 1),
    );
    let bundle = bundle(dir.path(), seats);
    let started = json!({"effect_id":"fx", "attempt_id":"a1", "driver":"driver"});
    let journal = |checkpoint: Value| {
        vec![
            envelope(
                EventType::RunStarted,
                json!({"manifest": bundle.manifest}),
                None,
            ),
            envelope(
                EventType::EffectRequested,
                json!({"effect_id":"fx", "seat":"work"}),
                None,
            ),
            envelope(EventType::EffectStarted, started.clone(), Some("a1")),
            envelope(
                EventType::EffectCheckpointed,
                json!({"effect_id":"fx", "attempt_id":"a1", "checkpoint": checkpoint}),
                Some("a1"),
            ),
        ]
    };
    let row = |site: &str, owner: &str, id: &str, persistent: bool| {
        json!({
            "step":"harness-started", "model":"claude-opus-5", "launch":"resumed",
            "site_ref": site, "instance_ref": owner,
            "root_session":{"kind":"claude-session", "id": id,
                            "harness_version":"2.1.266", "persistent": persistent}
        })
    };
    let key = resume::SiteKey::single("work", None);
    let mine = context(SITE_A, OWNER, SeatClass::Work);
    let ask = |checkpoint: Value, context: &resume::SiteContext| {
        offer_for_site(
            &journal(checkpoint),
            &key,
            context,
            "work",
            true,
            true,
            &started,
        )
        .map(|target| target.provider_id)
    };

    assert_eq!(
        ask(row(SITE_A, OWNER, "session-one", true), &mine),
        Some("session-one".into()),
        "this site's own confirmed, persistent root"
    );
    assert_eq!(
        ask(row(SITE_B, OWNER, "session-one", true), &mine),
        None,
        "another site's stamp is another site's session"
    );
    assert_eq!(
        ask(row(SITE_A, OTHER_OWNER, "session-one", true), &mine),
        None,
        "the newest owner is incompatible, and the query does not look behind it"
    );
    assert_eq!(
        ask(row(SITE_A, OWNER, "session-one", false), &mine),
        None,
        "a root the shape does not persist establishes the owner and cannot be offered"
    );
    // A stamped row with no confirmed root at all. The site stamp means
    // no legacy fallback is available to evade the confirmation it
    // failed, so nothing is offered.
    let mut unconfirmed = row(SITE_A, OWNER, "session-one", true);
    unconfirmed.as_object_mut().unwrap().remove("root_session");
    unconfirmed["launch"] = json!("cold");
    unconfirmed["session_id"] = json!("session-one");
    assert_eq!(ask(unconfirmed, &mine), None);

    // The newest owner is the only owner asked: an older row of this
    // site under THIS instance is not resurrected once a newer row under
    // another instance stands in front of it.
    let mut both = journal(row(SITE_A, OWNER, "older", true));
    both.push(envelope(
        EventType::EffectCheckpointed,
        json!({"effect_id":"fx", "attempt_id":"a1",
               "checkpoint": row(SITE_A, OTHER_OWNER, "newer", true)}),
        Some("a1"),
    ));
    assert_eq!(
        offer_for_site(&both, &key, &mine, "work", true, true, &started),
        None
    );

    // The owned target pairs the provider ID with the locator from the
    // SAME checkpoint, and `originating_root` reads the version and the
    // optional wrapper digest from that one row — never per field from
    // whichever checkpoint is newest.
    let mut with_locator = row(SITE_A, OWNER, "session-one", true);
    with_locator["transcript"] = json!({
        "kind": "dsh-session",
        "locator": "sessions/brokkr/seat-1",
        "home": "/home/operator/.dsh"
    });
    with_locator["root_session"]["wrapper_digest"] = json!("a".repeat(64));
    let journaled = journal(with_locator.clone());
    let target = offer_for_site(&journaled, &key, &mine, "work", true, true, &started)
        .expect("the confirmed root is offered");
    assert_eq!(target.provider_id, "session-one");
    assert_eq!(
        target.persistence_locator.as_deref(),
        Some("sessions/brokkr/seat-1")
    );
    assert_eq!(
        target.persistence_home.as_deref(),
        Some("/home/operator/.dsh"),
        "the home travels off the SAME checkpoint as the root and locator"
    );
    let originating = resume::originating_root(&journaled, SITE_A).expect("a confirmed root");
    assert_eq!(originating.harness_version.as_deref(), Some("2.1.266"));
    assert_eq!(
        originating.wrapper_digest.as_deref(),
        Some("a".repeat(64).as_str())
    );
    assert_eq!(
        originating.persistence_locator.as_deref(),
        Some("sessions/brokkr/seat-1")
    );
    // A confirmed row with no transcript reference still offers its
    // provider ID, and hands a two-coordinate planner no locator to
    // rejoin — which is a decline, never a guess.
    let locatorless = offer_for_site(
        &journal(row(SITE_A, OWNER, "session-one", true)),
        &key,
        &mine,
        "work",
        true,
        true,
        &started,
    )
    .expect("the root is offered");
    assert_eq!(locatorless.persistence_locator, None);
    assert_eq!(locatorless.persistence_home, None);

    // A newer confirmed row for the SAME site and instance that records no
    // transcript never borrows the home (or locator) of an older row: the
    // address is one fact off one checkpoint, and missing evidence stays
    // missing for a home-requiring planner to decline.
    let mut borrowed = journal(with_locator.clone());
    borrowed.push(envelope(
        EventType::EffectCheckpointed,
        json!({"effect_id":"fx", "attempt_id":"a1",
               "checkpoint": row(SITE_A, OWNER, "session-two", true)}),
        Some("a1"),
    ));
    let newest = offer_for_site(&borrowed, &key, &mine, "work", true, true, &started)
        .expect("the newest root is offered");
    assert_eq!(newest.provider_id, "session-two");
    assert_eq!(
        newest.persistence_home, None,
        "the newest row records no home; an older row's home is not borrowed"
    );
    assert_eq!(newest.persistence_locator, None);

    // Distinct old/new coordinates on the SAME site and instance: the
    // newest eligible row supplies all five, never one field from the
    // older row (the split `eligible_offer`/`originating_root` scans).
    let coord = |id: &str, version: Value, digest: Value, locator: &str, home: &str| {
        json!({
            "step":"harness-started", "model":"claude-opus-5", "launch":"resumed",
            "site_ref": SITE_A, "instance_ref": OWNER,
            "root_session":{"kind":"dsh-session", "id": id,
                            "harness_version": version, "persistent": true,
                            "wrapper_digest": digest},
            "transcript":{"kind":"dsh-session", "locator": locator, "home": home}
        })
    };
    let mut distinct = journal(coord(
        "older",
        json!("1.0.0"),
        json!("a".repeat(64)),
        "sessions/brokkr/older",
        "/old/home",
    ));
    distinct.push(envelope(
        EventType::EffectCheckpointed,
        json!({"effect_id":"fx", "attempt_id":"a1",
        "checkpoint": coord(
            "newer",
            json!("2.0.0"),
            json!("b".repeat(64)),
            "sessions/brokkr/newer",
            "/new/home",
        )}),
        Some("a1"),
    ));
    let newest = offer_for_site(&distinct, &key, &mine, "work", true, true, &started)
        .expect("the newest eligible row is offered");
    assert_eq!(newest.provider_id, "newer");
    assert_eq!(
        newest.persistence_locator.as_deref(),
        Some("sessions/brokkr/newer")
    );
    assert_eq!(newest.persistence_home.as_deref(), Some("/new/home"));
    let origin = resume::originating_root(&distinct, SITE_A).expect("a confirmed root");
    assert_eq!(origin.harness_version.as_deref(), Some("2.0.0"));
    assert_eq!(
        origin.wrapper_digest.as_deref(),
        Some("b".repeat(64).as_str())
    );
    assert_eq!(
        origin.persistence_locator.as_deref(),
        Some("sessions/brokkr/newer"),
        "the version, digest and locator come off the same newest row"
    );

    // A mistyped newest version/digest is missing evidence, never borrowed
    // from the older row.
    let mut mistyped = journal(coord(
        "older",
        json!("1.0.0"),
        json!("a".repeat(64)),
        "sessions/brokkr/older",
        "/old/home",
    ));
    mistyped.push(envelope(
        EventType::EffectCheckpointed,
        json!({"effect_id":"fx", "attempt_id":"a1",
        "checkpoint": coord(
            "newer",
            json!(7),
            json!(8),
            "sessions/brokkr/newer",
            "/new/home",
        )}),
        Some("a1"),
    ));
    let offered = offer_for_site(&mistyped, &key, &mine, "work", true, true, &started)
        .expect("the newest root is still eligible");
    assert_eq!(offered.provider_id, "newer");
    let origin = resume::originating_root(&mistyped, SITE_A).expect("a confirmed root");
    assert_eq!(
        origin.harness_version, None,
        "a mistyped version is missing evidence, never the older row's"
    );
    assert_eq!(
        origin.wrapper_digest, None,
        "a mistyped digest is missing evidence, never the older row's"
    );
}

/// The four topologies get four different keys, and every identity axis
/// moves the owner digest on its own (proposed decision 0056 rulings 1
/// and 2). Two sites that flatten to the same display tag do not share a
/// site digest, which is the aliasing the flat tag cannot see.
#[test]
fn the_site_key_is_structural_and_the_owner_key_moves_on_every_axis() {
    let single = resume::SiteKey::single("work", None);
    let cased = resume::SiteKey::single("work", Some("engine"));
    let member = resume::SiteKey::panel_member("work", None, "alpha", 0);
    let step = resume::SiteKey::sequence_step("work", None, "author", 0);
    let nested = resume::SiteKey::sequence_panel_member("work", None, "review", 1, "alpha", 0);
    let digests: Vec<String> = [&single, &cased, &member, &step, &nested]
        .iter()
        .map(|key| key.digest())
        .collect();
    let mut unique = digests.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), digests.len(), "five sites, five keys");
    for digest in &digests {
        assert_eq!(digest.len(), 64);
        assert!(digest
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }
    // Stable across processes: the same structural site hashes the same
    // way every time, which is what lets a fresh engine derive the same
    // offer from the same journal.
    assert_eq!(
        single.digest(),
        resume::SiteKey::single("work", None).digest()
    );

    // The aliasing pair: step `a:b` with member `c`, and step `a` with
    // member `b:c`. Both flatten to `a:b:c`; neither shares the other's
    // key.
    let left = resume::SiteKey::sequence_panel_member("work", None, "a:b", 0, "c", 0);
    let right = resume::SiteKey::sequence_panel_member("work", None, "a", 0, "b:c", 0);
    assert_ne!(left.digest(), right.digest());

    // Every owner axis, one at a time.
    let candidate = Candidate {
        agent: "implementer".into(),
        model: "opus".into(),
        effort: Some("high".into()),
        provider: "claude".into(),
        argv: vec!["{brokkr}".into(), "driver".into(), "claude".into()],
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
    };
    let manifest = json!({"engine":"0.10.0", "files":{}, "hands":{"work":{}}});
    let owner = |candidate: &Candidate,
                 chain: Option<usize>,
                 command: &[String],
                 manifest: &Value,
                 label: &str,
                 engine: &str,
                 class: SeatClass,
                 boundary: Option<Boundary>| {
        resume::InstanceKey::new(
            Some(candidate),
            chain,
            command,
            manifest,
            label,
            engine,
            class,
            boundary,
        )
        .digest()
    };
    let base = owner(
        &candidate,
        Some(0),
        &candidate.argv,
        &manifest,
        "work",
        "0.10.0",
        SeatClass::Work,
        Some(Boundary::Namespace),
    );
    let mut moved = Vec::new();
    for changed in [
        {
            let mut c = candidate.clone();
            c.agent = "other".into();
            c
        },
        {
            let mut c = candidate.clone();
            c.model = "sonnet".into();
            c
        },
        {
            let mut c = candidate.clone();
            c.effort = Some("low".into());
            c
        },
        {
            let mut c = candidate.clone();
            c.provider = "codex".into();
            c
        },
    ] {
        moved.push(owner(
            &changed,
            Some(0),
            &changed.argv,
            &manifest,
            "work",
            "0.10.0",
            SeatClass::Work,
            Some(Boundary::Namespace),
        ));
    }
    // The chain index, the driver and its command template, the pinned
    // bundle, the engine, the class and the boundary.
    moved.push(owner(
        &candidate,
        Some(1),
        &candidate.argv,
        &manifest,
        "work",
        "0.10.0",
        SeatClass::Work,
        Some(Boundary::Namespace),
    ));
    moved.push(owner(
        &candidate,
        Some(0),
        &["other-driver".to_string()],
        &manifest,
        "work",
        "0.10.0",
        SeatClass::Work,
        Some(Boundary::Namespace),
    ));
    let mut edited = manifest.clone();
    edited["files"]["adapters/claude.json"] = json!("c".repeat(64));
    moved.push(owner(
        &candidate,
        Some(0),
        &candidate.argv,
        &edited,
        "work",
        "0.10.0",
        SeatClass::Work,
        Some(Boundary::Namespace),
    ));
    moved.push(owner(
        &candidate,
        Some(0),
        &candidate.argv,
        &manifest,
        "work",
        "0.10.1",
        SeatClass::Work,
        Some(Boundary::Namespace),
    ));
    moved.push(owner(
        &candidate,
        Some(0),
        &candidate.argv,
        &manifest,
        "work",
        "0.10.0",
        SeatClass::Gate,
        Some(Boundary::Namespace),
    ));
    moved.push(owner(
        &candidate,
        Some(0),
        &candidate.argv,
        &manifest,
        "work",
        "0.10.0",
        SeatClass::Work,
        Some(Boundary::Harness),
    ));
    // The hands declaration, read from the manifest's pinned map under
    // this site's label.
    let mut rehanded = manifest.clone();
    rehanded["hands"]["work"] = json!({"binds": ["/elsewhere"]});
    moved.push(owner(
        &candidate,
        Some(0),
        &candidate.argv,
        &rehanded,
        "work",
        "0.10.0",
        SeatClass::Work,
        Some(Boundary::Namespace),
    ));
    for (index, digest) in moved.iter().enumerate() {
        assert_ne!(
            *digest, base,
            "identity axis {index} did not move the owner"
        );
    }
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

// ---------------------------------------------------------------------------
// The engine-side route-overlay binding (design D6 mechanism 1; AS3; 8.10).
//
// The adapter receives only argv, a working directory and the private
// context, so which SHAPE withheld the binding — a nonmember, a shadow,
// an ancestor-layer file, a `..` component, an escaping symlink, the
// `./` expansion — is observable only where the engine builds that
// context. These cases read the `resume_context.route_overlay` member off
// the `Start.input` the logging driver actually received, at both
// production `start_context` call sites: the single site (`run_driver`)
// and a panel member (`MemberRun`).
// ---------------------------------------------------------------------------

/// The manifest digest of one route-overlay file, computed the way the
/// compiler's `walk_files` computes it.
fn overlay_digest(bytes: &[u8]) -> String {
    brokkr_core::canonical::sha256_bytes(bytes)
}

/// Make a driver command carry the seat's one `--patch` pair. The shim
/// ignores trailing arguments; the engine reads the pair off the compiled
/// command it spawns.
fn patched(mut command: Vec<String>, value: &str) -> Vec<String> {
    command.push("--patch".into());
    command.push(value.into());
    command
}

/// The `start` message the named driver was sent, whose `input` carries
/// the private context.
fn route_start(dir: &Path, tag: &str) -> Value {
    received(dir, tag)
        .into_iter()
        .find(|message| message["type"] == "start")
        .unwrap_or_else(|| panic!("{tag}: the driver logged no start message"))
}

/// The `route_overlay` member of that start context, if any.
fn route_binding(dir: &Path, tag: &str) -> Value {
    route_start(dir, tag)["input"]["resume_context"]["route_overlay"].clone()
}

/// A valid leaf-manifest member binds at the SINGLE site: the context
/// carries the actual argv value and the member's compiled manifest
/// digest. The seat is inline, so its assessment is `unmeasured` — the
/// binding is independent of the resume gate and present on a cold start
/// with no offer.
#[test]
fn a_valid_route_overlay_binds_at_the_single_site() {
    let dir = tempfile::tempdir().unwrap();
    let layer = dir.path().join("work").join("recipe");
    let bytes = b"route: one\n";
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::write(layer.join("route.yml"), bytes).unwrap();
    let digest = overlay_digest(bytes);

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            single(
                patched(
                    driver(dir.path(), "work", &["complete"]),
                    "recipe/route.yml",
                ),
                Vec::new(),
            ),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });

    run(dir.path(), bundle);

    let binding = route_binding(dir.path(), "work");
    assert_eq!(binding["value"], "recipe/route.yml");
    assert_eq!(binding["digest"], digest);
}

/// The same valid member binds at the PANEL-MEMBER call site, from that
/// member's own composed argv.
#[test]
fn a_valid_route_overlay_binds_at_the_panel_member() {
    let dir = tempfile::tempdir().unwrap();
    let layer = dir.path().join("work").join("recipe");
    let bytes = b"route: panel\n";
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::write(layer.join("route.yml"), bytes).unwrap();
    let digest = overlay_digest(bytes);

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            panel(vec![member(
                "alpha",
                patched(
                    driver(dir.path(), "alpha", &["complete"]),
                    "recipe/route.yml",
                ),
            )]),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });

    run(dir.path(), bundle);

    let binding = route_binding(dir.path(), "alpha");
    assert_eq!(binding["value"], "recipe/route.yml");
    assert_eq!(binding["digest"], digest);
}

/// Every shape whose binding the engine withholds yields the SAME
/// outcome — no `route_overlay` member — never a start failure. Each
/// member of one panel carries one shape, and every member still starts
/// and is sent its context.
#[test]
fn a_non_binding_route_overlay_withholds_the_member_at_both_call_sites() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    let layer = work.join("recipe");
    let ancestor = work.join("base");
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::create_dir_all(&ancestor).unwrap();
    let member_bytes = b"route: member\n";
    std::fs::write(layer.join("route.yml"), member_bytes).unwrap();
    // A same-shaped file at the working-directory path, not inside the
    // layer: the shadow of the bundled path.
    std::fs::write(work.join("route.yml"), b"route: shadow\n").unwrap();
    // A file inside the layer the manifest does not record.
    std::fs::write(layer.join("other.yml"), b"route: other\n").unwrap();
    // An ancestor layer's file, bindable only through that ancestor's
    // aggregate digest.
    std::fs::write(ancestor.join("ancestor.yml"), b"route: ancestor\n").unwrap();
    // An in-layer symlink whose target resolves outside the layer but
    // inside the working directory.
    std::fs::write(work.join("outside.yml"), b"route: outside\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(work.join("outside.yml"), layer.join("link.yml")).unwrap();

    let absolute = layer.join("route.yml").display().to_string();
    let shapes: Vec<(&str, String)> = vec![
        ("nonmember", "recipe/other.yml".into()),
        ("shadow", "route.yml".into()),
        ("ancestor", "base/ancestor.yml".into()),
        ("traversal", "../escape.yml".into()),
        ("absolute", absolute),
    ];
    #[cfg(unix)]
    let shapes = {
        let mut shapes = shapes;
        shapes.push(("symlink", "recipe/link.yml".into()));
        shapes
    };

    let members: Vec<PanelMember> = shapes
        .iter()
        .map(|(tag, value)| member(tag, patched(driver(dir.path(), tag, &["complete"]), value)))
        .collect();
    let mut seats = BTreeMap::new();
    seats.insert("work".into(), seat(panel(members), &["complete"], 1));
    seats.insert(
        "review".into(),
        seat(
            single(driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.roots = vec![layer.clone(), ancestor.clone()];
    bundle.chain = vec![crate::Ancestor {
        name: "base".into(),
        reached_as: None,
        dir: ancestor.clone(),
        digest: "c".repeat(64),
        files: serde_json::Map::new(),
    }];
    // The manifest records only the real member, so the shadow, the
    // nonmember and the ancestor file are all non-members.
    bundle.manifest["files"] = json!({ "route.yml": overlay_digest(member_bytes) });

    run(dir.path(), bundle);

    for (tag, _) in &shapes {
        let binding = route_binding(dir.path(), tag);
        assert!(
            binding.is_null() || binding.get("value").is_none(),
            "{tag}: a non-binding --patch must carry no route_overlay, got {binding}"
        );
    }
}

/// A member whose bytes changed after compilation still binds with the
/// MANIFEST's recorded digest, never a fresh hash of the resolved file:
/// the adapter's required comparison is what refuses the new bytes.
#[test]
fn a_changed_route_overlay_member_carries_the_manifest_digest() {
    let dir = tempfile::tempdir().unwrap();
    let layer = dir.path().join("work").join("recipe");
    std::fs::create_dir_all(&layer).unwrap();
    let compiled = b"route: before\n";
    let compiled_digest = overlay_digest(compiled);
    std::fs::write(layer.join("route.yml"), b"route: after\n").unwrap();

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            single(
                patched(
                    driver(dir.path(), "work", &["complete"]),
                    "recipe/route.yml",
                ),
                Vec::new(),
            ),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": compiled_digest.clone() });

    run(dir.path(), bundle);

    let binding = route_binding(dir.path(), "work");
    assert_eq!(binding["value"], "recipe/route.yml");
    assert_eq!(
        binding["digest"], compiled_digest,
        "the carried digest is the manifest's, never a hash of the changed file"
    );
    assert_ne!(binding["digest"], overlay_digest(b"route: after\n"));
}

/// The binding is present on an OFFERED start exactly as on a cold one:
/// the same route member and manifest digest ride beside the assessment,
/// the originating-root facts and the owned target. The candidate's
/// argv supplies the `--patch`, so the binding follows the argv the
/// engine actually spawns, and the retry that receives an offer does not
/// lose it.
#[test]
fn a_valid_route_overlay_binds_on_an_offered_start_too() {
    let dir = tempfile::tempdir().unwrap();
    let layer = dir.path().join("work").join("recipe");
    let bytes = b"route: offered\n";
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::write(layer.join("route.yml"), bytes).unwrap();
    let digest = overlay_digest(bytes);

    let argv = patched(
        model_driver(dir.path(), "work", &["fail", "complete"]),
        "recipe/route.yml",
    );
    let candidate = Candidate {
        agent: "implementer".into(),
        model: "deepseek-v4-flash".into(),
        effort: Some("medium".into()),
        provider: "dsh".into(),
        argv: argv.clone(),
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
    };
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(single(argv, vec![candidate]), &["complete"], 2),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });

    run(dir.path(), bundle);

    let starts: Vec<Value> = received(dir.path(), "work")
        .into_iter()
        .filter(|message| message["type"] == "start")
        .collect();
    assert_eq!(starts.len(), 2, "the failing first attempt is retried");
    // The retry is the offered start; both starts carry the same binding.
    assert_eq!(
        offers(&received(dir.path(), "work")),
        [None, Some("work-1".into())]
    );
    for (index, start) in starts.iter().enumerate() {
        let binding = &start["input"]["resume_context"]["route_overlay"];
        assert_eq!(binding["value"], "recipe/route.yml", "start {index}");
        assert_eq!(binding["digest"], digest, "start {index}");
    }
}

// ---------------------------------------------------------------------------
// The engine-side originating-home carrier (design D6, Pass B completion;
// 8.10). The private `owned_target` at the offered start carries the
// recorded `transcript.home` beside the provider ID and locator, read off
// the same confirmed checkpoint, at both production `start_context` call
// sites — the single site (`run_driver`) and a panel member (`MemberRun`).
// The check runs beside the route-overlay binding that already reads those
// two `Start.input`s.
// ---------------------------------------------------------------------------

/// The recorded home rides beside the id and locator at the SINGLE site.
/// The first attempt is cold and carries no `owned_target`; the retry is
/// offered the confirmed root and its complete three-coordinate address.
#[test]
fn an_offered_dsh_start_carries_the_recorded_home_at_the_single_site() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("dsh-home");
    std::fs::create_dir_all(&home).unwrap();
    let home_text = home.display().to_string();
    let locator = "sessions/brokkr/seat-1";
    let argv = dsh_model_driver(
        dir.path(),
        "work",
        &["fail", "complete"],
        locator,
        &home_text,
        2,
    );
    let candidate = Candidate {
        agent: "implementer".into(),
        model: "deepseek-v4-flash".into(),
        effort: Some("medium".into()),
        provider: "dsh".into(),
        argv: argv.clone(),
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: Default::default(),
    };
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(single(argv, vec![candidate]), &["complete"], 2),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(dir.path(), "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    run(dir.path(), bundle(dir.path(), seats));

    let starts: Vec<Value> = received(dir.path(), "work")
        .into_iter()
        .filter(|message| message["type"] == "start")
        .collect();
    assert_eq!(starts.len(), 2, "the failing first attempt is retried");
    assert_eq!(
        offers(&received(dir.path(), "work")),
        [None, Some("work-1".into())]
    );
    assert!(
        starts[0]["input"]["resume_context"]
            .get("owned_target")
            .is_none(),
        "a no-offer start carries no owned target"
    );
    let owned = &starts[1]["input"]["resume_context"]["owned_target"];
    assert_eq!(owned["provider_id"], "work-1");
    assert_eq!(owned["persistence_locator"], locator);
    assert_eq!(owned["persistence_home"], home_text);
    // The other two coordinates come from the SAME confirmed checkpoint:
    // the version and composite the offered root was opened under.
    assert_eq!(
        starts[1]["input"]["resume_context"]["originating_harness_version"],
        "0.1.5-rc.1"
    );
    assert_eq!(
        starts[1]["input"]["resume_context"]["originating_wrapper_digest"],
        "a".repeat(64)
    );
    // The private carrier is not rendered into the prompt context.
    assert!(
        starts[1]["context"].get("owned_target").is_none(),
        "the owned target never reaches the rendered context"
    );
}

/// The same complete address travels at the PANEL-MEMBER call site, from
/// that member's own confirmed checkpoint rather than the panel's.
#[test]
fn an_offered_dsh_start_carries_the_recorded_home_at_the_panel_member() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("dsh-home");
    std::fs::create_dir_all(&home).unwrap();
    let home_text = home.display().to_string();
    let locator = "sessions/brokkr/alpha";
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            panel(vec![member(
                "alpha",
                dsh_model_driver(dir.path(), "alpha", &["pass"], locator, &home_text, 2),
            )]),
            &["pass", "fail"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(
                model_driver(dir.path(), "review", &["residual", "clean"]),
                Vec::new(),
            ),
            &["residual", "clean"],
            1,
        ),
    );
    let mut bundle = bundle(dir.path(), seats);
    bundle.machine = panel_machine();
    run(dir.path(), bundle);

    let starts: Vec<Value> = received(dir.path(), "alpha")
        .into_iter()
        .filter(|message| message["type"] == "start")
        .collect();
    assert_eq!(starts.len(), 2, "the panel is re-entered once");
    assert_eq!(
        offers(&received(dir.path(), "alpha")),
        [None, Some("alpha-1".into())]
    );
    assert!(starts[0]["input"]["resume_context"]
        .get("owned_target")
        .is_none());
    let owned = &starts[1]["input"]["resume_context"]["owned_target"];
    assert_eq!(owned["provider_id"], "alpha-1");
    assert_eq!(owned["persistence_locator"], locator);
    assert_eq!(owned["persistence_home"], home_text);
    assert_eq!(
        starts[1]["input"]["resume_context"]["originating_harness_version"],
        "0.1.5-rc.1"
    );
    assert_eq!(
        starts[1]["input"]["resume_context"]["originating_wrapper_digest"],
        "a".repeat(64)
    );
}

/// Run one shim command the way the engine does: hand it the first line
/// and the start message whose `*start*` arm breaks its read loop, then
/// collect what it wrote to stdout and stderr.
fn run_dsh_shim(argv: &[String]) -> (String, String) {
    use std::io::Write as _;
    let mut child = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, "{{\"type\":\"hello\"}}").unwrap();
        writeln!(
            stdin,
            "{{\"type\":\"start\",\"effect_id\":\"effect-1\",\"attempt_id\":\"attempt-1\"}}"
        )
        .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// R2: the DSH checkpoint transport, exercised through the shim's actual
/// emitted bytes rather than a hand-typed format. A Windows-shaped home
/// with backslashes, a percent sign, a quoted span and an embedded newline
/// must decode back unchanged, with one JSONL frame per checkpoint. This
/// is portable transport evidence, not a native Windows run.
#[test]
fn a_windows_shaped_dsh_home_survives_the_checkpoint_transport() {
    let dir = tempfile::tempdir().unwrap();
    let locator = "sessions/brokkr/seat-1";
    let home = "C:\\Users\\seat\\AppData\\%TEMP%\\\"quoted\"\nnext";
    let argv = dsh_model_driver(dir.path(), "transport", &["pass"], locator, home, 1);
    let (stdout, stderr) = run_dsh_shim(&argv);
    let frames: Vec<Value> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["type"] == "checkpoint")
        .collect();
    assert_eq!(
        frames.len(),
        1,
        "one checkpoint frame per invocation: {stdout} / {stderr}"
    );
    let data = &frames[0]["data"];
    assert_eq!(data["step"], "harness-started");
    assert_eq!(data["root_session"]["id"], "transport-1");
    assert_eq!(data["transcript"]["locator"], locator);
    assert_eq!(
        data["transcript"]["home"], home,
        "the Windows home is data, never reinterpreted: {stdout}"
    );
}

/// R2: a declared invocation with no serialized row fails the shim
/// explicitly instead of repeating the previous checkpoint.
#[test]
fn a_missing_dsh_checkpoint_row_fails_the_shim_without_repeating() {
    let dir = tempfile::tempdir().unwrap();
    let locator = "sessions/brokkr/seat-1";
    let home = "C:\\Users\\seat";
    let argv = dsh_model_driver(dir.path(), "short", &["pass"], locator, home, 1);
    let (first, _) = run_dsh_shim(&argv);
    assert!(
        first.contains("\"type\":\"checkpoint\""),
        "the first invocation has its row: {first}"
    );
    let (second, stderr) = run_dsh_shim(&argv);
    assert!(
        !second.contains("\"type\":\"checkpoint\""),
        "the second invocation must not repeat the first checkpoint: {second}"
    );
    assert!(stderr.contains("no checkpoint row"), "{stderr}");
}
