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

/// The same driver, emitting the rows an engine enacting proposed
/// decision 0056 actually stamps: a launch checkpoint naming a model —
/// which is what makes the engine write `site_ref` and `instance_ref`
/// onto it — and the provider-confirmed root the next offer stands on,
/// FOLLOWED by a seat-turn that names the model and no root. That is the
/// shape every shipped driver produces: the launch row is never the
/// newest stamped row of its site, because the telemetry behind it is
/// stamped too, and the offer has to stand on the root row regardless.
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
    // The launch row's argument list ends the one checkpoint printf the
    // legacy shim has; the seat-turn is appended right behind it.
    let launch_args = "\"$base\" \"$eid\" \"$aid\" \"$n\"\n";
    let followed = format!(
        "{launch_args}printf '%s,\"type\":\"checkpoint\",\"effect_id\":\"%s\",\"attempt_id\":\"%s\",\
         \"data\":{{\"step\":\"seat-turn\",\"turn\":1,\"model\":\"claude-opus-5\"}}}}\\n' \
         \"$base\" \"$eid\" \"$aid\"\n"
    );
    let with_turn = script.replace(launch_args, &followed);
    assert_ne!(
        with_turn, script,
        "the {tag} shim must emit a seat-turn behind its launch row"
    );
    vec![command[0].clone(), command[1].clone(), with_turn]
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
        inline_resume: std::collections::BTreeMap::new(),
        sites: Default::default(),
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

    // The production shape. Every shipped driver names a model on every
    // row it forwards, so the seat-turns and the finishing record behind
    // a launch carry the same two stamps and no root: the launch row is
    // never the newest stamped row of its site. The offer stands on the
    // newest ROOT-BEARING row; the telemetry in front of it changes no
    // owner and hides no session.
    let telemetry = |step: &str| {
        json!({
            "step": step, "model":"claude-opus-5",
            "site_ref": SITE_A, "instance_ref": OWNER
        })
    };
    let checkpointed = |checkpoint: Value| {
        envelope(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "attempt_id":"a1", "checkpoint": checkpoint}),
            Some("a1"),
        )
    };
    let mut production = journal(row(SITE_A, OWNER, "session-one", true));
    production.push(checkpointed(telemetry("seat-turn")));
    production.push(checkpointed(telemetry("claude-session-finished")));
    assert_eq!(
        offer_for_site(&production, &key, &mine, "work", true, true, &started)
            .map(|target| target.provider_id),
        Some("session-one".into()),
        "the launch row behind this attempt's telemetry is the offer"
    );

    // A site whose stamped rows carry no root at all is judged on that
    // evidence and nothing older. The same seat's unstamped codex row —
    // offered on its own, because the legacy predicate holds for it — is
    // not resurrected behind a stamped attempt that confirmed nothing.
    let legacy_row = json!({
        "step":"transcript",
        "transcript":{"kind":"codex-thread", "locator":"thread-1", "home":"/test/.codex"}
    });
    assert_eq!(
        ask(legacy_row.clone(), &mine),
        Some("thread-1".into()),
        "the unstamped row alone is decision 0030's offer"
    );
    let mut cold_after_legacy = journal(legacy_row);
    cold_after_legacy.push(checkpointed(telemetry("seat-turn")));
    assert_eq!(
        offer_for_site(
            &cold_after_legacy,
            &key,
            &mine,
            "work",
            true,
            true,
            &started
        ),
        None,
        "a stamped site never falls back to the unstamped rows behind it"
    );

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
        Some("b".repeat(64).as_str()),
        "the version and digest come off the same newest row as the locator"
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

    // A mistyped newest locator or home is missing evidence too: the
    // newest root is still the offer, and the mistyped coordinate reads
    // `None` rather than the older row's while the other one stands.
    for (field, value) in [("locator", json!(9)), ("home", json!(10))] {
        let mut newer = coord(
            "newer",
            json!("2.0.0"),
            json!("b".repeat(64)),
            "sessions/brokkr/newer",
            "/new/home",
        );
        newer["transcript"][field] = value;
        let mut mistyped = journal(coord(
            "older",
            json!("1.0.0"),
            json!("a".repeat(64)),
            "sessions/brokkr/older",
            "/old/home",
        ));
        mistyped.push(envelope(
            EventType::EffectCheckpointed,
            json!({"effect_id":"fx", "attempt_id":"a1", "checkpoint": newer}),
            Some("a1"),
        ));
        let offered = offer_for_site(&mistyped, &key, &mine, "work", true, true, &started)
            .expect("the newest root is still eligible");
        assert_eq!(offered.provider_id, "newer", "{field}");
        let (locator, home) = match field {
            "locator" => (None, Some("/new/home")),
            _ => (Some("sessions/brokkr/newer"), None),
        };
        assert_eq!(
            offered.persistence_locator.as_deref(),
            locator,
            "a mistyped {field} is missing evidence, never the older row's"
        );
        assert_eq!(
            offered.persistence_home.as_deref(),
            home,
            "a mistyped {field} is missing evidence, never the older row's"
        );
    }
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
/// outcome — a private context carrying no `route_overlay` member — never
/// a start failure. Each member of one panel carries one shape, on a
/// canonical root, and every member still starts and is sent its context.
/// Every value names bytes a lookup could find, so each shape is withheld
/// by its own rule and not by a missing file: the shadow and the ancestor
/// file are spelled `route.yml` beside a member of that name, the ancestor
/// records its file in its own `files`, and the `..` value and the
/// absolute `./` expansion both resolve to the member itself. The
/// single-site half of "at either call site" is the nonmember case and
/// the remaining-shape case below.
#[test]
fn a_non_binding_route_overlay_withholds_the_member_at_both_call_sites() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let work = root.join("work");
    let layer = work.join("recipe");
    let ancestor = work.join("base");
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::create_dir_all(&ancestor).unwrap();
    let member_bytes = b"route: member\n";
    let ancestor_bytes = b"route: ancestor\n";
    std::fs::write(layer.join("route.yml"), member_bytes).unwrap();
    // A same-shaped file at the working-directory path, not inside the
    // layer: the shadow of the bundled path.
    std::fs::write(work.join("route.yml"), b"route: shadow\n").unwrap();
    // A file inside the layer the manifest does not record.
    std::fs::write(layer.join("other.yml"), b"route: other\n").unwrap();
    // An ancestor layer's same-named file, recorded in that ancestor's
    // own `files`.
    std::fs::write(ancestor.join("route.yml"), ancestor_bytes).unwrap();
    // An in-layer symlink whose target resolves outside the layer but
    // inside the working directory.
    std::fs::write(work.join("outside.yml"), b"route: outside\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(work.join("outside.yml"), layer.join("link.yml")).unwrap();

    // What `./route.yml` expands to in a compiled command (`bundle.rs`'s
    // `dir.join(rel)`): already absolute.
    let absolute = layer.join("route.yml").display().to_string();
    let shapes: Vec<(&str, String)> = vec![
        ("nonmember", "recipe/other.yml".into()),
        ("shadow", "route.yml".into()),
        ("ancestor", "base/route.yml".into()),
        ("traversal", "../work/recipe/route.yml".into()),
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
        .map(|(tag, value)| member(tag, patched(driver(&root, tag, &["complete"]), value)))
        .collect();
    let mut seats = BTreeMap::new();
    seats.insert("work".into(), seat(panel(members), &["complete"], 1));
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.roots = vec![layer.clone(), ancestor.clone()];
    let mut ancestor_files = serde_json::Map::new();
    ancestor_files.insert("route.yml".into(), json!(overlay_digest(ancestor_bytes)));
    bundle.chain = vec![crate::Ancestor {
        name: "base".into(),
        reached_as: None,
        dir: ancestor.clone(),
        digest: "c".repeat(64),
        files: ancestor_files,
    }];
    // The manifest records only the real member, so the shadow, the
    // nonmember and the ancestor file are all non-members.
    bundle.manifest["files"] = json!({ "route.yml": overlay_digest(member_bytes) });

    run(&root, bundle);

    let mut bound = Vec::new();
    for (tag, _) in &shapes {
        let context = route_start(&root, tag)["input"]["resume_context"].clone();
        assert!(
            context.is_object(),
            "{tag}: the panel member still receives its private context, got {context}"
        );
        if let Some(binding) = context.get("route_overlay") {
            bound.push((*tag, binding.clone()));
        }
    }
    assert_eq!(
        bound,
        Vec::<(&str, Value)>::new(),
        "a non-binding --patch must carry no route_overlay at the panel member"
    );
}

/// The SINGLE site withholds the member for the same shapes. The panel
/// case above drives all six through `MemberRun`; this one drives the
/// nonmember — a file inside the compiled layer the manifest does not
/// record — through `run_driver`, so the withholding half of the clause
/// is proved at either call site rather than at one. The site still
/// receives its private context: withholding is a missing member, never
/// a missing context and never a start failure.
#[test]
fn a_non_binding_route_overlay_withholds_the_member_at_the_single_site() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let layer = root.join("work").join("recipe");
    let member_bytes = b"route: member\n";
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::write(layer.join("route.yml"), member_bytes).unwrap();
    // A file inside the layer the manifest does not record.
    std::fs::write(layer.join("other.yml"), b"route: other\n").unwrap();

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            single(
                patched(driver(&root, "work", &["complete"]), "recipe/other.yml"),
                Vec::new(),
            ),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": overlay_digest(member_bytes) });

    run(&root, bundle);

    let context = route_start(&root, "work")["input"]["resume_context"].clone();
    assert!(
        context.is_object(),
        "the single site still receives its private context, got {context}"
    );
    assert_eq!(
        context.get("route_overlay"),
        None,
        "a nonmember --patch must carry no route_overlay at the single site, got {context}"
    );
}

/// The remaining non-binding shapes at the SINGLE site (B38(ii)), each
/// driven through `run_driver` in a run of its own on a canonical root.
/// Every value names bytes a lookup could find: the shadow and the
/// ancestor file are spelled `route.yml` beside a member of that name,
/// the ancestor records its file in its own `files`, and the `..` value
/// and the absolute `./` expansion both resolve to the member itself —
/// so each is withheld by the rule for its shape, not by a missing file.
/// Each shape that nevertheless carries a `route_overlay` is collected
/// with it, and the assertion is that none does.
#[test]
fn every_remaining_non_binding_shape_is_withheld_at_the_single_site() {
    let member_bytes = b"route: member\n";
    let ancestor_bytes = b"route: ancestor\n";
    let shapes = [
        ("shadow", "route.yml"),
        ("ancestor", "base/route.yml"),
        ("traversal", "../work/recipe/route.yml"),
        ("absolute", ""),
    ];
    let mut bound = Vec::new();
    for (shape, spelled) in shapes {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let work = root.join("work");
        let layer = work.join("recipe");
        let ancestor = work.join("base");
        std::fs::create_dir_all(&layer).unwrap();
        std::fs::create_dir_all(&ancestor).unwrap();
        std::fs::write(layer.join("route.yml"), member_bytes).unwrap();
        // The working-directory shadow of the bundled path.
        std::fs::write(work.join("route.yml"), b"route: shadow\n").unwrap();
        // An ancestor layer's same-named file.
        std::fs::write(ancestor.join("route.yml"), ancestor_bytes).unwrap();
        // What `./route.yml` expands to in a compiled command
        // (`bundle.rs`'s `dir.join(rel)`): already absolute.
        let value = match shape {
            "absolute" => layer.join("route.yml").display().to_string(),
            _ => spelled.to_string(),
        };

        let mut seats = BTreeMap::new();
        seats.insert(
            "work".into(),
            seat(
                single(
                    patched(driver(&root, "work", &["complete"]), &value),
                    Vec::new(),
                ),
                &["complete"],
                1,
            ),
        );
        seats.insert(
            "review".into(),
            seat(
                single(driver(&root, "review", &["clean"]), Vec::new()),
                &["clean"],
                1,
            ),
        );
        let mut bundle = bundle(&layer, seats);
        bundle.roots = vec![layer.clone(), ancestor.clone()];
        let mut ancestor_files = serde_json::Map::new();
        ancestor_files.insert("route.yml".into(), json!(overlay_digest(ancestor_bytes)));
        bundle.chain = vec![crate::Ancestor {
            name: "base".into(),
            reached_as: None,
            dir: ancestor.clone(),
            digest: "c".repeat(64),
            files: ancestor_files,
        }];
        bundle.manifest["files"] = json!({ "route.yml": overlay_digest(member_bytes) });

        run(&root, bundle);

        let context = route_start(&root, "work")["input"]["resume_context"].clone();
        assert!(
            context.is_object(),
            "{shape}: the single site still receives its private context, got {context}"
        );
        if let Some(binding) = context.get("route_overlay") {
            bound.push((shape, binding.clone()));
        }
    }
    assert_eq!(
        bound,
        Vec::<(&str, Value)>::new(),
        "a non-binding --patch must carry no route_overlay at the single site"
    );
}

/// The escaping symlink as a COMPILED member (B38(ii)). The symlink case
/// in the panel above is refused for being absent from the manifest, so
/// it cannot show that membership alone never authorizes a path whose
/// resolution leaves the layer. Here `recipe/link.yml` sits inside the
/// layer, resolves to `work/outside.yml` — outside the layer, inside the
/// working directory — and is listed in `files` with the digest of the
/// bytes it reaches, beside the real member. Returns the canonical layer
/// and the manifest `files` object, after asserting the fixture is the
/// shape the clause names.
#[cfg(unix)]
fn escaping_member_layer(root: &Path) -> (std::path::PathBuf, Value) {
    let work = root.join("work");
    let layer = work.join("recipe");
    std::fs::create_dir_all(&layer).unwrap();
    let member_bytes = b"route: member\n";
    let outside_bytes = b"route: outside\n";
    std::fs::write(layer.join("route.yml"), member_bytes).unwrap();
    std::fs::write(work.join("outside.yml"), outside_bytes).unwrap();
    std::os::unix::fs::symlink(work.join("outside.yml"), layer.join("link.yml")).unwrap();

    assert!(std::fs::symlink_metadata(layer.join("link.yml"))
        .unwrap()
        .file_type()
        .is_symlink());
    let resolved = std::fs::canonicalize(layer.join("link.yml")).unwrap();
    assert_eq!(resolved, work.join("outside.yml"));
    assert!(!resolved.starts_with(&layer));

    let files = json!({
        "route.yml": overlay_digest(member_bytes),
        "link.yml": overlay_digest(outside_bytes),
    });
    (layer, files)
}

/// The compiled escaping member receives no binding at the SINGLE site:
/// the context arrives, carrying no `route_overlay`, although `link.yml`
/// has a `files` entry the lookup would find.
#[cfg(unix)]
#[test]
fn an_escaping_symlink_member_is_withheld_at_the_single_site() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let (layer, files) = escaping_member_layer(&root);

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            single(
                patched(driver(&root, "work", &["complete"]), "recipe/link.yml"),
                Vec::new(),
            ),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = files;

    run(&root, bundle);

    let context = route_start(&root, "work")["input"]["resume_context"].clone();
    assert!(
        context.is_object(),
        "the single site still receives its private context, got {context}"
    );
    assert_eq!(
        context.get("route_overlay"),
        None,
        "a member whose resolution leaves the layer must carry no route_overlay, got {context}"
    );
}

/// The same compiled escaping member at the PANEL-MEMBER call site,
/// beside a sibling carrying the real member, which binds: the run
/// composes a binding where one is due, and withholds only the escape.
#[cfg(unix)]
#[test]
fn an_escaping_symlink_member_is_withheld_at_the_panel_member() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let (layer, files) = escaping_member_layer(&root);
    let member_digest = files["route.yml"].clone();

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            panel(vec![
                member(
                    "alpha",
                    patched(driver(&root, "alpha", &["complete"]), "recipe/route.yml"),
                ),
                member(
                    "escape",
                    patched(driver(&root, "escape", &["complete"]), "recipe/link.yml"),
                ),
            ]),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = files;

    run(&root, bundle);

    let bound = route_binding(&root, "alpha");
    assert_eq!(bound["value"], "recipe/route.yml");
    assert_eq!(bound["digest"], member_digest);
    let context = route_start(&root, "escape")["input"]["resume_context"].clone();
    assert!(
        context.is_object(),
        "the panel member still receives its private context, got {context}"
    );
    assert_eq!(
        context.get("route_overlay"),
        None,
        "a member whose resolution leaves the layer must carry no route_overlay, got {context}"
    );
}

/// A member whose bytes changed after compilation still binds with the
/// MANIFEST's recorded digest, never a fresh hash of the resolved file:
/// the adapter's required comparison is what refuses the new bytes.
/// Driven at the single site here and at the panel member below.
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

/// The same changed member at the PANEL-MEMBER call site, from that
/// member's own composed argv: the carried digest is still the
/// manifest's, and still not the hash of the bytes now on disk — read
/// here off the resolved file rather than off a literal, so the
/// `assert_ne!` compares against what the engine would have hashed.
#[test]
fn a_changed_route_overlay_member_carries_the_manifest_digest_at_the_panel_member() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let layer = root.join("work").join("recipe");
    std::fs::create_dir_all(&layer).unwrap();
    let compiled = b"route: panel before\n";
    let compiled_digest = overlay_digest(compiled);
    std::fs::write(layer.join("route.yml"), b"route: panel after\n").unwrap();

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            panel(vec![member(
                "alpha",
                patched(driver(&root, "alpha", &["complete"]), "recipe/route.yml"),
            )]),
            &["complete"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": compiled_digest.clone() });

    run(&root, bundle);

    let resolved = overlay_digest(&std::fs::read(layer.join("route.yml")).unwrap());
    let binding = route_binding(&root, "alpha");
    assert_eq!(binding["value"], "recipe/route.yml");
    assert_eq!(
        binding["digest"], compiled_digest,
        "the member's carried digest is the manifest's, never a hash of the changed file"
    );
    assert_ne!(binding["digest"], resolved);
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

/// The binding is present on an OFFERED start at the PANEL-MEMBER call
/// site too. The panel positive above is a cold first start, and the
/// offered positive above rides the single site; here the member is sent
/// back into by the phase machine's re-entry and offered the session its
/// first start opened, and both of its starts carry the same member and
/// the manifest's recorded digest.
#[test]
fn a_valid_route_overlay_binds_on_an_offered_panel_member_start_too() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let layer = root.join("work").join("recipe");
    let bytes = b"route: offered panel\n";
    std::fs::create_dir_all(&layer).unwrap();
    std::fs::write(layer.join("route.yml"), bytes).unwrap();
    let digest = overlay_digest(bytes);

    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(
            panel(vec![member(
                "alpha",
                patched(model_driver(&root, "alpha", &["pass"]), "recipe/route.yml"),
            )]),
            &["pass", "fail"],
            1,
        ),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["residual", "clean"]), Vec::new()),
            &["residual", "clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.machine = panel_machine();
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });

    run(&root, bundle);

    let received = received(&root, "alpha");
    assert_eq!(
        offers(&received),
        [None, Some("alpha-1".into())],
        "the member's re-entry is the offered start"
    );
    let bindings: Vec<Value> = received
        .iter()
        .filter(|message| message["type"] == "start")
        .map(|start| start["input"]["resume_context"]["route_overlay"].clone())
        .collect();
    let expected = json!({ "value": "recipe/route.yml", "digest": digest });
    assert_eq!(bindings, [expected.clone(), expected]);
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

/// A spelling no other fixture byte carries, inside every value of the
/// marked route below, so a copy of the route's CONTENT under any field
/// name is found by one search.
const ROUTE_MARKER: &str = "route4amarker";

/// A valid DSH route document (the reader's grammar for a
/// `deepseek/deepseek-v4-flash` pin; `adapters/tests.rs` validates the
/// same bytes) whose display name, key variable and endpoint all carry
/// the marker, written as the compiled leaf layer's `route.yml` under the
/// root's working directory. Returns the layer and the manifest digest.
fn marked_route_layer(root: &Path) -> (PathBuf, String) {
    let layer = root.join("work").join("recipe");
    std::fs::create_dir_all(&layer).unwrap();
    let bytes = format!(
        "- id: llm-pi-ai\n  config:\n    providers:\n      deepseek:\n        \
         displayName: {ROUTE_MARKER}\n        apiKeyEnv: ROUTE4AMARKER_KEY\n        \
         baseURL: https://{ROUTE_MARKER}.example/v1\n        models:\n          \
         - id: deepseek-v4-flash\n            reasoningEfforts:\n              \
         medium: medium\n"
    );
    std::fs::write(layer.join("route.yml"), &bytes).unwrap();
    (layer, overlay_digest(bytes.as_bytes()))
}

/// Task 8.8(d)/8.10 (4993–4995, 5251–5254): no route byte, no binding and
/// no private carrier reaches the journal. Every event is read, the
/// launch rows included. The one sanctioned copy of the file's provenance
/// is the pinned manifest's `files` entry, which the bundle digest already
/// covers (adapter-resume-safety spec, route overlay scenario); it is
/// asserted exactly and then taken out before the search.
fn assert_journal_free_of_route_and_carriers(events: &[EventEnvelope], layer: &Path, digest: &str) {
    let needles = route_and_carrier_needles(layer, digest);
    for event in events {
        let mut payload = event.payload.clone();
        if event.event_type == EventType::RunStarted {
            assert_eq!(payload["manifest"]["files"]["route.yml"], digest);
            payload["manifest"]["files"]
                .as_object_mut()
                .unwrap()
                .remove("route.yml");
        }
        let text = serde_json::to_string(&payload).unwrap().to_lowercase();
        for needle in &needles {
            assert_eq!(
                text.find(&needle.to_lowercase()),
                None,
                "{:?} (seq {}) carries {needle}: {text}",
                event.event_type,
                event.seq
            );
        }
    }
}

/// What no surface a bound route's attempt emits may carry: the route's
/// distinctive content, its argv value, its file path, its digest, and
/// every member of the private start context. Matched case-insensitively.
fn route_and_carrier_needles(layer: &Path, digest: &str) -> Vec<String> {
    let route_file = layer.join("route.yml").display().to_string();
    vec![
        ROUTE_MARKER.to_string(),
        "recipe/route.yml".to_string(),
        route_file,
        digest.to_string(),
        "\"resume_context\"".to_string(),
        "\"route_overlay\"".to_string(),
        "\"owned_target\"".to_string(),
        "\"assessment\"".to_string(),
        "\"originating_harness_version\"".to_string(),
        "\"originating_wrapper_digest\"".to_string(),
        "\"persistence_home\"".to_string(),
        "\"persistence_locator\"".to_string(),
    ]
}

/// The journaled launch rows of one DSH site, in journal order: the
/// `harness-started` checkpoints whose root is a `dsh-session`.
fn dsh_launch_rows(events: &[EventEnvelope]) -> Vec<(String, Value)> {
    events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .filter(|event| {
            event.payload["checkpoint"]["step"] == "harness-started"
                && event.payload["checkpoint"]["root_session"]["kind"] == "dsh-session"
        })
        .map(|event| {
            (
                event.attempt_id.clone().unwrap_or_default(),
                event.payload["checkpoint"].clone(),
            )
        })
        .collect()
}

/// B30's third predicate on the attempt the offered start belongs to: its
/// journaled launch row keeps the confirmed `root_session` and
/// `transcript` exactly as the driver published them, and carries none
/// of the private start context's members.
fn assert_offered_launch_row_retained(
    events: &[EventEnvelope],
    offered: &Value,
    tag: &str,
    locator: &str,
    home: &str,
) {
    let attempt = offered["attempt_id"].as_str().unwrap();
    let rows = dsh_launch_rows(events);
    let row = rows
        .iter()
        .find(|(id, _)| id == attempt)
        .map(|(_, row)| row)
        .unwrap_or_else(|| panic!("the offered attempt journaled no launch row: {rows:?}"));
    assert_eq!(
        row["root_session"],
        json!({
            "kind": "dsh-session",
            "id": format!("{tag}-2"),
            "harness_version": "0.1.5-rc.1",
            "wrapper_digest": "a".repeat(64),
            "persistent": true,
        }),
        "the confirmed root is retained: {row}"
    );
    assert_eq!(
        row["transcript"],
        json!({"kind": "dsh-session", "locator": locator, "home": home}),
        "the confirmed address is retained: {row}"
    );
    for member in [
        "resume_context",
        "owned_target",
        "assessment",
        "route_overlay",
    ] {
        assert_eq!(
            row.get(member),
            None,
            "launch evidence carries {member}: {row}"
        );
    }
}

/// The recorded home rides beside the id and locator at the SINGLE site.
/// The first attempt is cold and carries no `owned_target`; the retry is
/// offered the confirmed root and its complete three-coordinate address.
///
/// Unit 4a: the seat also carries a bound route whose every value spells
/// a marker. The offered start's private context holds the binding, the
/// owned target and the assessment; the journal — the launch row of that
/// attempt included — holds none of them, and the launch row keeps its
/// confirmed `root_session` and `transcript`.
#[test]
fn an_offered_dsh_start_carries_the_recorded_home_at_the_single_site() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let home = root.join("dsh-home");
    std::fs::create_dir_all(&home).unwrap();
    let home_text = home.display().to_string();
    let locator = "sessions/brokkr/seat-1";
    let (layer, digest) = marked_route_layer(&root);
    let argv = patched(
        dsh_model_driver(&root, "work", &["fail", "complete"], locator, &home_text, 2),
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
        // Task 8.8(a): this site's SELECTED declaration, carrying the
        // optional member, so the private start context can be read for
        // it on the real single-site path beside the owned target.
        resume: dsh_assessment_declaring(Some(&"c".repeat(64))),
    };
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(single(argv, vec![candidate]), &["complete"], 2),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });
    let events = run(&root, bundle);

    let starts: Vec<Value> = received(&root, "work")
        .into_iter()
        .filter(|message| message["type"] == "start")
        .collect();
    assert_eq!(starts.len(), 2, "the failing first attempt is retried");
    assert_eq!(
        offers(&received(&root, "work")),
        [None, Some("work-1".into())]
    );
    // The carriers exist where they belong — the private start context of
    // the offered attempt — so their absence below is an exclusion.
    let private = &starts[1]["input"]["resume_context"];
    assert_eq!(
        private["route_overlay"],
        json!({"value": "recipe/route.yml", "digest": digest})
    );
    assert_eq!(private["owned_target"]["provider_id"], "work-1");
    assert_offered_launch_row_retained(&events, &starts[1], "work", locator, &home_text);
    assert_journal_free_of_route_and_carriers(&events, &layer, &digest);
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
    // Task 8.8(a): the DECLARED member is a different fact from the
    // originating checkpoint's, and both travel. The declaration's value
    // rides inside the selected assessment, at both attempts.
    for start in &starts {
        assert_eq!(
            start["input"]["resume_context"]["assessment"]["headless-work"]["identity"]
                ["wrapper_digest"],
            "c".repeat(64),
            "the selected declaration's exact member: {start}"
        );
    }
    assert_ne!(
        starts[1]["input"]["resume_context"]["assessment"]["headless-work"]["identity"]
            ["wrapper_digest"],
        starts[1]["input"]["resume_context"]["originating_wrapper_digest"],
        "the declaration is not the originating observation"
    );
    // The private carrier is not rendered into the prompt context.
    assert!(
        starts[1]["context"].get("owned_target").is_none(),
        "the owned target never reaches the rendered context"
    );
    assert!(
        starts[1]["context"].get("assessment").is_none(),
        "the assessment never reaches the rendered context"
    );
}

/// The same complete address travels at the PANEL-MEMBER call site, from
/// that member's own confirmed checkpoint rather than the panel's — and,
/// under unit 4a, the same exclusion and retention hold for the member's
/// bound route, its launch row and the whole journal.
#[test]
fn an_offered_dsh_start_carries_the_recorded_home_at_the_panel_member() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let home = root.join("dsh-home");
    std::fs::create_dir_all(&home).unwrap();
    let home_text = home.display().to_string();
    let locator = "sessions/brokkr/alpha";
    let (layer, digest) = marked_route_layer(&root);
    let argv = patched(
        dsh_model_driver(&root, "alpha", &["pass"], locator, &home_text, 2),
        "recipe/route.yml",
    );
    let mut alpha = member("alpha", argv.clone());
    // Task 8.8(a): the panel member's own SELECTED declaration, so the
    // second production `start_context` call site is read too.
    alpha.candidates = vec![Candidate {
        agent: "implementer".into(),
        model: "deepseek-v4-flash".into(),
        effort: Some("medium".into()),
        provider: "dsh".into(),
        argv,
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: dsh_assessment_declaring(Some(&"d".repeat(64))),
    }];
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(panel(vec![alpha]), &["pass", "fail"], 1),
    );
    seats.insert(
        "review".into(),
        seat(
            single(
                model_driver(&root, "review", &["residual", "clean"]),
                Vec::new(),
            ),
            &["residual", "clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.machine = panel_machine();
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });
    let events = run(&root, bundle);

    let starts: Vec<Value> = received(&root, "alpha")
        .into_iter()
        .filter(|message| message["type"] == "start")
        .collect();
    assert_eq!(starts.len(), 2, "the panel is re-entered once");
    assert_eq!(
        offers(&received(&root, "alpha")),
        [None, Some("alpha-1".into())]
    );
    let private = &starts[1]["input"]["resume_context"];
    assert_eq!(
        private["route_overlay"],
        json!({"value": "recipe/route.yml", "digest": digest})
    );
    assert_eq!(private["owned_target"]["provider_id"], "alpha-1");
    assert_offered_launch_row_retained(&events, &starts[1], "alpha", locator, &home_text);
    assert_journal_free_of_route_and_carriers(&events, &layer, &digest);
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
    // Task 8.8(a): the member's own declared member travels at this
    // call site too, distinct from the originating observation.
    for start in &starts {
        assert_eq!(
            start["input"]["resume_context"]["assessment"]["headless-work"]["identity"]
                ["wrapper_digest"],
            "d".repeat(64),
            "the panel member's selected declaration: {start}"
        );
    }
}

/// Set in the environment of this suite's own binary when the engine
/// spawns it as a driver: the case below then serves the production DSH
/// adapter on its stdin and stdout instead of running as a test.
#[cfg(unix)]
const SERVE_DSH: &str = "BROKKR_RESUME_TESTS_SERVE_DSH";

/// The one line libtest writes to stdout ahead of the served protocol when
/// this suite's binary runs one case quietly; the process exits before
/// libtest could write its summary.
#[cfg(unix)]
const HARNESS_LINE: &str = "running 1 test";

/// Unit 4a, the seam the two halves meet at (review of `e428ad23`, C1 +
/// SEC-1): the engine drives the REAL DSH adapter — production's
/// `adapters::serve`, spawned as this very test binary — on a seat whose
/// `--patch` route binds, and the journal that attempt appends is read.
/// The gate is closed, as it ships, so this is the cold route production
/// runs today. The first attempt fails with stderr, so the stderr tail the
/// engine journals on a failed attempt is read too; the retry succeeds.
///
/// The route reaches the child on both attempts, and the private context
/// the driver was started with holds the binding and the assessment; the
/// journal — each launch row, the failed attempt's stderr tail, every
/// other event — carries none of the route's content, its path, its
/// digest, the binding or a private carrier, and the stderr tail is
/// exactly the child's own bytes. The adapter's stdout reaches the engine
/// whole but for libtest's one announcement line, and that raw stream is
/// searched too, so a disclosure the adapter prints outside the protocol
/// is neither filtered away nor missed.
#[cfg(unix)]
#[test]
fn the_real_dsh_driver_journals_no_route_byte_and_no_carrier() {
    if std::env::var_os(SERVE_DSH).is_some() {
        let extra = std::env::var(format!("{SERVE_DSH}_EXTRA")).unwrap();
        let extra = extra.split(' ').map(str::to_string).collect();
        let served =
            brokkr_protocol::adapters::serve(brokkr_protocol::adapters::AdapterKind::Dsh, extra);
        // Nothing of the harness may follow the protocol on stdout.
        std::process::exit(if served.is_ok() { 0 } else { 70 });
    }

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let home = root.join("dsh-home");
    std::fs::create_dir_all(&home).unwrap();
    let (layer, digest) = marked_route_layer(&root);
    // The dsh child the adapter launches as `dsh --profile headless
    // --patch <overlay> <prompt>`: it keeps the overlay it was handed,
    // writes one stderr line, and fails its first invocation without a
    // result; the second writes the result the prompt names.
    let dsh = root.join("dsh");
    std::fs::write(
        &dsh,
        format!(
            "#!/bin/sh\n\
             n=$(cat '{count}' 2>/dev/null || echo 0)\n\
             n=$((n+1))\n\
             printf '%s' \"$n\" > '{count}'\n\
             cp \"$4\" '{seen}'-\"$n\"\n\
             printf 'dsh child %s wrote this\\n' \"$n\" >&2\n\
             [ \"$n\" = 1 ] && exit 3\n\
             result=$(printf '%s\\n' \"$5\" | grep '/.forge/results/' | head -n 1 | sed 's/^ *//')\n\
             printf '{{\"result\":\"complete\"}}' > \"$result\"\n",
            count = root.join("dsh.count").display(),
            seen = root.join("seen").display(),
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dsh, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    // The driver: this binary, filtered to this case, in the serving role.
    // Its stdin is logged the way the shim drivers' is, so the start it
    // was sent can be read back. Its whole stdout is kept as it left the
    // process, and the one line the harness announces itself with is the
    // only line taken out before the engine reads it: anything else the
    // adapter writes — protocol or not — reaches the engine, which
    // journals an unreadable line in its failure (review of `f5895001`,
    // SEC-2).
    let this = format!(
        "{}::the_real_dsh_driver_journals_no_route_byte_and_no_carrier",
        module_path!().split_once("::").unwrap().1
    );
    let script = format!(
        "unset DSH_PERMISSION_MODE\n\
         tee -a '{log}' | {SERVE_DSH}=1 {SERVE_DSH}_EXTRA=\"$*\" DSH_HOME='{home}' \
         HOME='{root}' BROKKR_DSH_BIN='{dsh}' '{exe}' '{this}' --exact --nocapture \
         --test-threads=1 -q | tee -a '{raw}' | grep --line-buffered -v -x '{HARNESS_LINE}'\n",
        log = root.join("work.log").display(),
        raw = root.join("work.stdout").display(),
        home = home.display(),
        root = root.display(),
        dsh = dsh.display(),
        exe = std::env::current_exe().unwrap().display(),
    );
    let argv: Vec<String> = [
        "sh",
        "-c",
        &script,
        "sh",
        "--model",
        "deepseek/deepseek-v4-flash",
        "--patch",
        "recipe/route.yml",
    ]
    .map(str::to_string)
    .to_vec();
    // The shipped declaration, read by the production loader: `unmeasured`,
    // so the adapter's gate is closed and it runs the shipped cold route.
    let shipped = crate::Adapters::load(&workspace_root().join("adapters"))
        .expect("the shipped adapters load")
        .adapter("dsh")
        .expect("the shipped dsh adapter")
        .resume
        .clone();
    let candidate = Candidate {
        agent: "implementer".into(),
        model: "deepseek/deepseek-v4-flash".into(),
        effort: None,
        provider: "dsh".into(),
        argv: argv.clone(),
        hands_fragment: Vec::new(),
        harness: HarnessHands::default(),
        resume: shipped,
    };
    let mut seats = BTreeMap::new();
    seats.insert(
        "work".into(),
        seat(single(argv, vec![candidate]), &["complete"], 2),
    );
    seats.insert(
        "review".into(),
        seat(
            single(driver(&root, "review", &["clean"]), Vec::new()),
            &["clean"],
            1,
        ),
    );
    let mut bundle = bundle(&layer, seats);
    bundle.manifest["files"] = json!({ "route.yml": digest.clone() });
    let events = run(&root, bundle);

    // The journal first, before any assertion that presumes the attempts
    // went well: a disclosure that also breaks the protocol is still named
    // here, as a disclosure. Every surface searched is asserted to exist
    // below, so this search is not run over nothing.
    assert_journal_free_of_route_and_carriers(&events, &layer, &digest);

    // The adapter's whole stdout, as it left the process on both attempts:
    // the harness line is all that was taken out, every other line is a
    // protocol message, and none of it carries the route or a carrier.
    let raw = std::fs::read_to_string(root.join("work.stdout")).unwrap();
    let dropped: Vec<&str> = raw.lines().filter(|line| !line.starts_with('{')).collect();
    assert_eq!(dropped, ["", HARNESS_LINE, "", HARNESS_LINE], "{raw}");
    for line in raw.lines().filter(|line| line.starts_with('{')) {
        let message: Value = serde_json::from_str(line).unwrap();
        assert_eq!(message["proto"], "forge-driver/v1", "{line}");
    }
    let lowered = raw.to_lowercase();
    for needle in route_and_carrier_needles(&layer, &digest) {
        assert_eq!(
            lowered.find(&needle.to_lowercase()),
            None,
            "the adapter's stdout carries {needle}: {raw}"
        );
    }

    // Both attempts ran the real adapter, which bound the route and handed
    // it to the child.
    let starts: Vec<Value> = received(&root, "work")
        .into_iter()
        .filter(|message| message["type"] == "start")
        .collect();
    assert_eq!(starts.len(), 2, "the failing first attempt is retried");
    for (index, start) in starts.iter().enumerate() {
        let private = &start["input"]["resume_context"];
        assert_eq!(
            private["route_overlay"],
            json!({"value": "recipe/route.yml", "digest": digest}),
            "start {index}"
        );
        assert_eq!(
            private["assessment"]["headless-work"]["status"], "unmeasured",
            "start {index}: {private}"
        );
        let handed = std::fs::read_to_string(root.join(format!("seen-{}", index + 1))).unwrap();
        assert_eq!(handed.matches(ROUTE_MARKER).count(), 2, "{handed}");
    }

    // The two attempts, as journaled: a launch row each, the first failed
    // with a stderr tail and the second succeeded — the surfaces the
    // exclusion above was read over.
    let attempts: Vec<&str> = starts
        .iter()
        .map(|start| start["attempt_id"].as_str().unwrap())
        .collect();
    let launch_rows = |attempt: &str| -> Vec<Value> {
        events
            .iter()
            .filter(|event| event.attempt_id.as_deref() == Some(attempt))
            .filter(|event| event.event_type == EventType::EffectCheckpointed)
            .map(|event| event.payload["checkpoint"].clone())
            .filter(|row| row["step"] == "harness-started")
            .collect()
    };
    for attempt in &attempts {
        assert_eq!(launch_rows(attempt).len(), 1, "{attempt}: one launch row");
    }
    let failed: Vec<&EventEnvelope> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectFailed)
        .collect();
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].attempt_id.as_deref(), Some(attempts[0]));
    assert!(
        failed[0].payload["error"]
            .as_str()
            .is_some_and(|error| error.contains("; stderr tail: ")),
        "the failed attempt journals a stderr tail: {}",
        failed[0].payload
    );
    assert!(
        events.iter().any(|event| {
            event.event_type == EventType::EffectSucceeded
                && event.attempt_id.as_deref() == Some(attempts[1])
        }),
        "the retry succeeded"
    );

    // And each is exactly what the adapter and the engine own: the stderr
    // tail is the child's own line, nothing added, and the launch row is
    // the shipped route's vocabulary plus the engine's three stamps, whose
    // two references are hashes.
    assert_eq!(
        failed[0].payload["error"],
        "agent CLI exited 3; stderr tail: dsh child 1 wrote this\n"
    );
    for attempt in &attempts {
        let rows = launch_rows(attempt);
        let mut row = rows[0].clone();
        for stamp in ["site_ref", "instance_ref"] {
            let value = row.as_object_mut().unwrap().remove(stamp);
            assert_eq!(
                value.as_ref().and_then(Value::as_str).map(str::len),
                Some(64),
                "{attempt}: {stamp}: {}",
                rows[0]
            );
        }
        assert_eq!(
            row,
            json!({
                "step": "harness-started",
                "harness": "deepseek",
                "launch": "cold",
                "model": "not reported",
                "effort": "not applicable",
                "boundary": "not applicable",
            }),
            "{attempt}"
        );
    }
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

/// The workspace root: this file lives at
/// `crates/brokkr-runtime/src/engine/`.
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

/// A machine whose single work seat is named `implement`, so a test can
/// lift the SHIPPED recipe's compiled assessment map — keyed by that same
/// seat label — onto its own seats without renaming a thing.
fn implement_machine() -> Machine {
    Machine::from_table(&json!({
        "phases":["implement", "review", "done", "stop"],
        "initial":"implement",
        "terminal":["done", "stop"],
        "rules":[
            {"id":"IMPL", "from":"implement", "result":"complete", "next":"review",
             "reason":"implemented"},
            {"id":"DONE", "from":"review", "result":"clean", "next":"done",
             "reason":"clean"}
        ]
    }))
    .unwrap()
}

/// The operator's 2026-09-15 ruling at the engine's own composition
/// boundary: a shipped INLINE Codex work seat's adapter assessment, read
/// by the compiler and carried on the bundle, reaches the driver's private
/// start context exactly as an agent-resolved site's does. Emptying the
/// compiled map makes this fail, so it proves the engine supplies the
/// assessment rather than a test reading a declaration back.
#[test]
fn an_inline_codex_work_seat_carries_the_shipped_assessment_into_its_start() {
    let root = workspace_root();
    let compiled = Bundle::compile_with(
        &root.join("recipes/standby"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .expect("the shipped standby recipe compiles");
    assert_eq!(
        compiled.inline_resume["implement"]["work-site"]["status"], "supported",
        "the composition half: the shipped inline Codex seat carries the preserved assessment"
    );

    let dir = tempfile::tempdir().unwrap();
    let mut seats = BTreeMap::new();
    seats.insert(
        "implement".into(),
        seat(
            single(driver(dir.path(), "implement", &["complete"]), Vec::new()),
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
    let mut bundle = bundle(dir.path(), seats);
    bundle.machine = implement_machine();
    super::tests::set_inline_resume(&mut bundle, &compiled.inline_resume);
    run(dir.path(), bundle);

    let start = route_start(dir.path(), "implement");
    assert_eq!(
        start["input"]["resume_context"]["assessment"]["work-site"]["status"], "supported",
        "the shipped assessment reaches an INLINE site's private start context: {start}"
    );
    assert_eq!(
        start["input"]["resume_context"]["assessment"]["work-site"]["boundaries"],
        json!(["harness", "not applicable"]),
        "the inline coordinate's boundary is the declared one: {start}"
    );
}

/// Recursively copy one directory beside another — the shipped adapters
/// beside a scratch root, so a declaration can be edited without
/// touching the repository's bytes.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// The shipped DSH declaration with its optional `wrapper_digest`
/// member set or removed, read back through the PRODUCTION adapter
/// loader rather than constructed in Rust: by the time the assessment
/// exists, the 64-lowercase-hex grammar has already been enforced at
/// load (task 8.8(a)).
fn dsh_assessment_declaring(wrapper: Option<&str>) -> crate::agents::ResumeAssessment {
    let scratch = tempfile::tempdir().unwrap();
    let adapters = scratch.path().join("adapters");
    copy_tree(&workspace_root().join("adapters"), &adapters);
    let path = adapters.join("dsh.json");
    let mut dsh: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let identity = &mut dsh["resume"]["headless-work"]["identity"];
    match wrapper {
        Some(wrapper) => identity["wrapper_digest"] = json!(wrapper),
        None => {
            identity
                .as_object_mut()
                .expect("the measured identity is an object")
                .remove("wrapper_digest");
        }
    }
    std::fs::write(&path, serde_json::to_vec_pretty(&dsh).unwrap()).unwrap();
    crate::Adapters::load(&adapters)
        .expect("the edited adapters load")
        .adapter("dsh")
        .expect("the shipped dsh adapter")
        .resume
        .clone()
}

/// Task 8.8(a): the optional `wrapper_digest` a declaration carries
/// reaches the driver's PRIVATE start context as the exact member the
/// loader admitted, through the selected assessment and no new wire,
/// store or contract field.
///
/// The declaration is read off disk by the real adapter loader, so the
/// 64-lowercase-hex grammar has already been enforced by the time the
/// assessment exists; the engine then carries it. The control beside it
/// is the same run with the member absent: the key is simply not there,
/// which is how "omission remains valid" is visible rather than assumed.
#[test]
fn a_declared_wrapper_digest_reaches_the_private_start_context() {
    let root = workspace_root();
    let digest = "a".repeat(64);

    let _ = &root;
    let declared = dsh_assessment_declaring;

    let carried = |resume: crate::agents::ResumeAssessment| {
        let dir = tempfile::tempdir().unwrap();
        let argv = driver(dir.path(), "work", &["complete"]);
        let candidate = Candidate {
            agent: "implementer".into(),
            model: "deepseek-v4-flash".into(),
            effort: Some("medium".into()),
            provider: "dsh".into(),
            argv: argv.clone(),
            hands_fragment: Vec::new(),
            harness: HarnessHands::default(),
            resume,
        };
        let mut seats = BTreeMap::new();
        seats.insert(
            "work".into(),
            seat(single(argv, vec![candidate]), &["complete"], 1),
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
        let start = received(dir.path(), "work")
            .into_iter()
            .find(|message| message["type"] == "start")
            .expect("the seat was started");
        start["input"]["resume_context"]["assessment"]["headless-work"]["identity"].clone()
    };

    let with_member = carried(declared(Some(&digest)));
    assert_eq!(
        with_member["wrapper_digest"], digest,
        "the exact declared member reaches the private start context: {with_member}"
    );
    // The carriage is the assessment's, not a side channel: the two
    // members beside it travel in the same object.
    assert!(with_member["version"].is_string(), "{with_member}");
    assert!(with_member["applies_to"].is_string(), "{with_member}");

    let without = carried(declared(None));
    assert!(
        without.get("wrapper_digest").is_none(),
        "omission stays an omission rather than a null: {without}"
    );
    assert!(without["version"].is_string(), "{without}");
}

/// F1: the declaration an inline work seat reads its resume assessment
/// from is part of the bundle's identity. A valid edit to it — status,
/// applicability and scope preserved — moves the manifest the next
/// process compares against the run's pin, so the edited declaration
/// refuses to resume rather than rejoin under a rule the root was never
/// opened with. The unchanged bundle beside it still resumes, which is
/// what makes this a control rather than a declaration read-back.
#[test]
fn an_edited_inline_resume_declaration_moves_identity_and_refuses_the_old_root() {
    let root = workspace_root();
    let scratch = tempfile::tempdir().unwrap();
    let adapters = scratch.path().join("adapters");
    copy_tree(&root.join("adapters"), &adapters);

    let compile = |adapters: &Path| {
        Bundle::compile_with(
            &root.join("recipes/standby"),
            &root.join("agents"),
            adapters,
        )
        .expect("the shipped standby recipe compiles")
    };
    let pinned = |bundle: &Bundle| bundle.manifest["drivers"]["implement"]["codex"].clone();

    let before = compile(&adapters);
    let before_pin = pinned(&before);
    assert!(
        before_pin.is_string(),
        "the inline Codex work seat pins the declaration it reads: {}",
        before.manifest["drivers"]
    );

    // A valid edit to the measured restrictions evidence, keeping the
    // shape's status, applicability and scope: only the declaration
    // bytes move, and the assessment the next process reads with them.
    let path = adapters.join("codex.json");
    let mut codex: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    codex["resume"]["work-site"]["evidence"]["restrictions"] =
        json!("decision 0030 ruling 2, re-read: the class is re-imposed as -c sandbox_mode");
    std::fs::write(&path, serde_json::to_vec_pretty(&codex).unwrap()).unwrap();
    let after = compile(&adapters);

    assert_ne!(
        before_pin,
        pinned(&after),
        "the consulted declaration is pinned, so editing it moves the pin"
    );
    assert_ne!(
        before.manifest_digest(),
        after.manifest_digest(),
        "and the bundle identity moves with it"
    );
    assert_ne!(
        before.inline_resume, after.inline_resume,
        "the assessment the next process reads really did change"
    );

    // The synthetic seats stand in for the shipped inline Codex work
    // seat: the manifest under test is the compiled one, so the identity
    // compared at resume is production's. Each run's bundle is built
    // ONCE and handed to both `start` and `resume` — the driver command
    // is part of the instance key, and rebuilding it would move the key
    // for a reason that has nothing to do with the declaration under
    // test.
    let seats = |dir: &Path| {
        let mut seats = BTreeMap::new();
        seats.insert(
            "implement".into(),
            seat(
                single(
                    model_driver(dir, "implement", &["fail", "complete"]),
                    Vec::new(),
                ),
                &["complete"],
                1,
            ),
        );
        seats.insert(
            "review".into(),
            seat(
                single(model_driver(dir, "review", &["clean"]), Vec::new()),
                &["clean"],
                1,
            ),
        );
        seats
    };
    let run_bundle = |dir: &Path, source: &Bundle| {
        std::fs::create_dir_all(dir).unwrap();
        let mut built = bundle(dir, seats(dir));
        built.machine = implement_machine();
        built.manifest = source.manifest.clone();
        super::tests::set_inline_resume(&mut built, &source.inline_resume);
        built
    };
    let park = |dir: &Path, built: Bundle| {
        std::fs::create_dir_all(dir.join("work")).unwrap();
        let store = Store::open(&dir.join("forge.db")).unwrap();
        let mut engine = Engine::start(store, built, "resume", Some(dir.to_path_buf())).unwrap();
        let run_id = engine.run_id.clone();
        engine.drive().unwrap();
        assert_eq!(
            fold(&engine.store.load(&run_id).unwrap()).unwrap().status,
            Status::AwaitingOperator,
            "the failing attempt parked the run with the root it opened"
        );
        (engine.store, run_id)
    };

    // Positive control: the unchanged bundle resumes, and the retry
    // rejoins the thread the first attempt opened.
    let same_dir = scratch.path().join("same");
    let same = run_bundle(&same_dir, &before);
    let (mut store, run_id) = park(&same_dir, same.clone());
    operator_command(&mut store, &run_id, "retry", "operator", "once more").unwrap();
    Engine::resume(store, same, &run_id, Some(same_dir.clone()))
        .unwrap()
        .drive()
        .unwrap();
    assert_eq!(
        offers(&received(&same_dir, "implement")),
        [None, Some("implement-1".into())],
        "the unchanged pinned bundle still rejoins its own root"
    );

    // Negative: the same journal under the edited declaration is a
    // different identity, and the engine refuses it instead of reusing
    // the root the old declaration opened.
    let edited_dir = scratch.path().join("edited");
    let (store, run_id) = park(&edited_dir, run_bundle(&edited_dir, &before));
    let error = match Engine::resume(
        store,
        run_bundle(&edited_dir, &after),
        &run_id,
        Some(edited_dir.clone()),
    ) {
        Ok(_) => panic!("the edited declaration must not reuse the root the old one opened"),
        Err(error) => error,
    };
    assert!(
        matches!(error, EngineError::ManifestMismatch { .. }),
        "expected the manifest-mismatch refusal, got {error:?}"
    );
}
