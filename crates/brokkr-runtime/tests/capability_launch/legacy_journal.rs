//! D9's native legacy boundary matrix (decision 0065 slice two, U4a–U4f;
//! CC1, CC3 and SC4): before any normalized emission, a legacy native
//! record travels the whole production path — compile, a real driver
//! process, the engine, the store's append fence, export and offline
//! verify — at every executable site shape, and arrives exactly as the
//! driver wrote it plus the engine's own stamps, with no attribution
//! group and no private observation. Each U4 merge reruns it.
//!
//! The driver is this test binary re-entered as [`legacy_driver_child`],
//! the deterministic fixture `two_engines_one_journal.rs` established: it
//! writes what the shipped Claude lowering writes for a native search
//! call and a local read, so no provider need be installed.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::Status;
use brokkr_protocol::{Body, Message, ResultStatus};
use brokkr_runtime::{Bundle, Engine};
use brokkr_store::{SeatRecordError, Store, StoreError};
use serde_json::{json, Map, Value};

use super::write;

/// Set by the fixture's own command line, never by this process: the
/// state directory a driver keeps its per-seat attempt count in.
const STATE: &str = "BROKKR_LEGACY_JOURNAL_STATE";
/// The arguments an adapter appended after the fixture's command.
const ARGS: &str = "BROKKR_LEGACY_JOURNAL_ARGS";

/// The fields SC4 reserves for v6's attribution group, and the private
/// observation key a driver could leak before its consumer exists. None
/// may reach the journal at a preparation merge.
const UNRECORDED: [&str; 6] = [
    "capability",
    "dialect",
    "call_id",
    "call_state",
    "response_sha256",
    "observation",
];

const V5: &str = "contracts/seat-record.v5.schema.json";

/// The command that re-enters this binary as one driver session, through
/// `grep` for the reason `two_engines_one_journal.rs` gives: libtest
/// owns the head of stdout, the protocol owns the rest.
fn driver_command(state: &Path) -> Vec<String> {
    let exe = std::env::current_exe().unwrap();
    vec![
        "sh".into(),
        "-c".into(),
        format!(
            "{STATE}='{}' {ARGS}=\"$*\" exec '{}' --exact legacy_journal::legacy_driver_child \
             --nocapture | grep --line-buffered '^{{'",
            state.display(),
            exe.display()
        ),
        "sh".into(),
    ]
}

fn say(body: Body) {
    println!("{}", serde_json::to_string(&Message::new(body)).unwrap());
}

fn read() -> Option<Message> {
    let mut line = String::new();
    match std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => serde_json::from_str(&line).ok(),
    }
}

/// The model an adapter pinned on the command line, or the inline
/// fixture's own name.
fn served_model() -> String {
    let args = std::env::var(ARGS).unwrap_or_default();
    let mut words = args.split_whitespace();
    while let Some(word) = words.next() {
        if word == "--model" {
            return words.next().unwrap_or_default().to_string();
        }
    }
    "fixture-inline".to_string()
}

/// This seat's attempt count before this attempt, advanced on disk.
fn attempt_of(seat: &str) -> u64 {
    let file = PathBuf::from(std::env::var(STATE).unwrap()).join(seat.replace(':', "-"));
    let attempt = std::fs::read_to_string(&file)
        .ok()
        .and_then(|count| count.parse().ok())
        .unwrap_or(0);
    std::fs::write(&file, (attempt + 1).to_string()).unwrap();
    attempt
}

/// The launch row the shipped lowering publishes, with the root it
/// confirmed: `resumed` only where the engine offered `session`.
fn launched(model: &str, seat: &str, attempt: u64, session: Option<&str>) -> Value {
    let id = session.map_or_else(|| format!("{seat}-root-{attempt}"), str::to_string);
    json!({"step": "harness-started", "harness": "claude", "model": model,
           "launch": if session.is_some() { "resumed" } else { "cold" },
           "root_session": {"kind": "claude-session", "id": id,
                            "harness_version": "2.1.266", "persistent": seat == "resumed"}})
}

/// A native search call and a local read, as the shipped lowering
/// writes them today: the tool's name, never a capability.
fn turns(model: &str) -> [Value; 2] {
    [
        json!({"step": "seat-turn", "turn": 1, "model": model, "effort": "high",
               "input_tokens": 13, "output_tokens": 2, "tool": "WebSearch"}),
        json!({"step": "seat-turn", "turn": 2, "model": model, "effort": "high",
               "tool": "Read", "target": "src/lib.rs"}),
    ]
}

/// Not a test of its own: one driver session, run only when
/// [`driver_command`] re-enters this binary with [`STATE`] set. The
/// `resumed` and `replaced` seats fail their first attempt after it
/// accepted, so the engine's retry is served the root it left behind —
/// rejoined where the root persists, cold where it does not.
#[test]
fn legacy_driver_child() {
    if std::env::var(STATE).is_err() {
        return;
    }
    let Some(_hello) = read() else { return };
    say(Body::Capabilities {
        driver: "legacy-journal".into(),
        version: "1".into(),
        supports: vec!["resume".into()],
    });
    let (mut session, mut message) = (None, read());
    if let Some(Message {
        body: Body::Resume { session_ref, .. },
        ..
    }) = &message
    {
        session = Some(session_ref.clone());
        message = read();
    }
    let Some(Message {
        body:
            Body::Start {
                effect_id,
                attempt_id,
                seat,
                ..
            },
        ..
    }) = message
    else {
        return;
    };
    let (model, attempt) = (served_model(), attempt_of(&seat));
    let checkpoint = |data: Value| {
        say(Body::Checkpoint {
            effect_id: effect_id.clone(),
            attempt_id: attempt_id.clone(),
            data,
        });
    };
    say(Body::Accepted {
        effect_id: effect_id.clone(),
        attempt_id: attempt_id.clone(),
        session_ref: Some(format!("{seat}-session")),
    });
    let rejoins = matches!(seat.as_str(), "resumed" | "replaced");
    if rejoins {
        checkpoint(launched(&model, &seat, attempt, session.as_deref()));
    }
    let (status, result, error) = match (rejoins, attempt) {
        (true, 0) => (
            ResultStatus::Failed,
            None,
            Some("the first attempt fails".into()),
        ),
        _ => {
            turns(&model).into_iter().for_each(checkpoint);
            let word = results_of(seat.split(':').next().unwrap())[0];
            let result = json!({"result": word, "notes": "done", "model": model});
            (ResultStatus::Succeeded, Some(result), None)
        }
    };
    say(Body::Result {
        effect_id,
        attempt_id,
        status,
        result,
        error,
    });
    while read().is_some_and(|message| !matches!(message.body, Body::Shutdown)) {}
}

/// A provider served by the fixture, and one whose driver is never
/// installed: every attempt on `absent` fails to start, so a chain that
/// names it first falls back (decision 0016).
fn adapters(root: &Path, state: &Path) {
    let adapter = |provider: &str, model: &str, driver: Vec<String>| {
        json!({"provider": provider, "binary": "sh", "driver": driver,
               "models": {model: format!("{provider}/{model}")},
               "model_flag": "--model", "efforts": ["high"], "effort_flag": "--effort",
               "tool_permissions": "unsupported", "mcp": "unsupported"})
    };
    write(
        root,
        "adapters/fixture.json",
        &adapter("fixture", "served", driver_command(state)),
    );
    write(
        root,
        "adapters/absent.json",
        &adapter(
            "absent",
            "missing",
            vec!["brokkr-legacy-absent-driver".into()],
        ),
    );
}

/// The seats, one per site shape, each stepping to the next: an
/// agent-backed single seat, an inline one, an agent whose primary never
/// starts, a panel and a sequence each with one inline site and one
/// agent site, two inline seats retried after a failed first attempt,
/// and the protected review gate every policy keeps.
const SEATS: [&str; 8] = [
    "ordinary", "inline", "fallback", "panel", "sequence", "resumed", "replaced", "review",
];

/// The results a seat declares; a driver answers with the first.
fn results_of(seat: &str) -> &'static [&'static str] {
    match seat {
        "panel" => &["pass", "fail"],
        "review" => &["clean"],
        _ => &["complete"],
    }
}

fn bundle(root: &Path, state: &Path) -> Bundle {
    adapters(root, state);
    std::fs::create_dir_all(root.join("agents/charters")).unwrap();
    std::fs::write(root.join("agents/charters/office.md"), "# office\n").unwrap();
    for (agent, models) in [
        ("office", json!(["served"])),
        ("chain", json!(["missing", "served"])),
    ] {
        let efforts: Map<String, Value> = models
            .as_array()
            .unwrap()
            .iter()
            .map(|model| (model.as_str().unwrap().to_string(), json!("high")))
            .collect();
        write(
            root,
            &format!("agents/{agent}.json"),
            &json!({"description": "a fixture office", "charter": "charters/office.md",
                    "models": models, "efforts": efforts}),
        );
    }
    std::fs::create_dir_all(root.join("bundle/roles")).unwrap();
    std::fs::write(root.join("bundle/roles/role.md"), "# role\n").unwrap();
    let inline = json!({"role": "roles/role.md", "driver": {"command": driver_command(state)}});
    let mut retried = inline.clone();
    retried["limits"] = json!({"max_attempts": 2});
    let mut step = inline.clone();
    step["name"] = json!("first");
    step["results"] = json!(["complete"]);
    let mut seats = json!({
        "ordinary": {"agent": "office"},
        "inline": inline.clone(),
        "fallback": {"agent": "chain", "limits": {"max_attempts": 2}},
        "panel": {"aggregate": "unanimous-pass",
                  "panel": {"member": inline.clone(), "agent": {"agent": "office"}}},
        "sequence": {"sequence": [step, {"name": "then", "agent": "office"}]},
        "resumed": retried.clone(),
        "replaced": retried,
        "review": inline,
    });
    let mut rules: Vec<Value> = Vec::new();
    for (at, seat) in SEATS.iter().enumerate() {
        let next = SEATS.get(at + 1).copied().unwrap_or("done");
        seats[seat]["results"] = json!(results_of(seat));
        for result in results_of(seat) {
            rules.push(json!({"id": format!("{seat}-{result}"), "from": seat,
                              "result": result, "next": next, "reason": "r"}));
        }
    }
    let mut phases: Vec<&str> = SEATS.to_vec();
    phases.push("done");
    write(
        root,
        "bundle/policy.json",
        &json!({"phases": phases, "initial": "ordinary", "terminal": ["done"], "rules": rules}),
    );
    write(
        root,
        "bundle/bundle.json",
        &json!({"name": "legacy-journal", "policy": "policy.json", "seats": seats}),
    );
    Bundle::compile_with(
        &root.join("bundle"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap_or_else(|refusal| panic!("the legacy matrix compiles: {refusal}"))
}

/// What a site wrote, keyed by its seat and the member or step that
/// wrote it (`""` for a single seat and for a sequence's own rows):
/// panel members run side by side, so only each site's order is fixed.
type Sites = BTreeMap<(String, String), Vec<Value>>;

const SERVED: &str = "fixture/served";
const INLINE: &str = "fixture-inline";

/// Every checkpoint the run journaled, by site, with the engine's two
/// digest stamps (proposed decision 0056 ruling 1) proved lowercase hex
/// and then named rather than spelled: one hashes this machine's command.
fn journaled(events: &[EventEnvelope]) -> Sites {
    let field =
        |event: &EventEnvelope, name: &str| event.payload[name].as_str().unwrap().to_string();
    let seats: BTreeMap<String, String> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .map(|event| (field(event, "effect_id"), field(event, "seat")))
        .collect();
    let mut sites = Sites::new();
    for event in events {
        if event.event_type != EventType::EffectCheckpointed {
            continue;
        }
        let mut checkpoint = event.payload["checkpoint"].clone();
        for stamp in ["site_ref", "instance_ref"] {
            if let Some(digest) = checkpoint.get(stamp).and_then(Value::as_str) {
                let hex = digest
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
                assert!(digest.len() == 64 && hex, "{stamp}: {digest}");
                checkpoint[stamp] = json!(stamp);
            }
        }
        let member = checkpoint["member"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let seat = seats[&field(event, "effect_id")].clone();
        sites.entry((seat, member)).or_default().push(checkpoint);
    }
    sites
}

/// A driver row as the engine journals it: the boundary and, on a row
/// that names a model, both stamps; a member's or step's tag beside.
fn stamped(mut row: Value, member: &str) -> Value {
    row["boundary"] = json!("not applicable");
    row["site_ref"] = json!("site_ref");
    row["instance_ref"] = json!("instance_ref");
    if !member.is_empty() {
        row["member"] = json!(member);
    }
    row
}

/// The journal D9 expects at a preparation merge: the fixture's legacy
/// rows exactly, at every site, with the engine's own panel and
/// sequence rows; the persistent root rejoined, the other one replaced.
fn wanted() -> Sites {
    let mut sites = Sites::new();
    let mut site = |seat: &str, member: &str, rows: Vec<Value>| {
        sites.insert((seat.to_string(), member.to_string()), rows);
    };
    let turned = |model: &str, member: &str| -> Vec<Value> {
        let rows = turns(model).into_iter();
        rows.map(|row| stamped(row, member)).collect()
    };
    for (seat, model) in [
        ("ordinary", SERVED),
        ("inline", INLINE),
        ("fallback", SERVED),
        ("review", INLINE),
    ] {
        site(seat, "", turned(model, ""));
    }
    for (member, model) in [("member", INLINE), ("agent", SERVED)] {
        let mut rows = turned(model, member);
        rows.push(
            json!({"step": "panel-member-finished", "boundary": "not applicable",
                         "inner_checkpoints": 2, "member": member, "model": model,
                         "outcome": "succeeded", "session_ref": format!("panel:{member}-session")}),
        );
        site("panel", member, rows);
    }
    site("sequence", "first", turned(INLINE, "first"));
    site("sequence", "then", turned(SERVED, "then"));
    let result = json!({"result": "complete", "notes": "done", "model": INLINE,
                        "boundary": "not applicable"});
    site(
        "sequence",
        "",
        vec![
            json!({"step": "sequence-step-finished", "step_name": "first",
                    "boundary": "not applicable", "model": INLINE, "result": result}),
        ],
    );
    for (seat, offered) in [("resumed", Some("resumed-root-0")), ("replaced", None)] {
        let mut rows = vec![
            stamped(launched(INLINE, seat, 0, None), ""),
            stamped(launched(INLINE, seat, 1, offered), ""),
        ];
        rows.extend(turned(INLINE, ""));
        site(seat, "", rows);
    }
    sites
}

/// A direct append of a partial attribution group, a whole one, or a
/// private observation, each beside a legacy native row, is refused with
/// the exact v5 violation at the seq it would have taken, and the
/// journal stands still.
fn refuses_unrecorded_fields(store: &mut Store, run_id: &str) {
    let head = store.head_hash(run_id).unwrap();
    let base = json!({"step": "seat-turn", "turn": 3, "model": INLINE, "tool": "WebSearch"});
    let group = json!({"capability": "web-search", "dialect": "claude-native-search",
                       "call_id": "attempt-1:toolu_01", "call_state": "observed"});
    let mut whole = base.clone();
    let fields = group.as_object().unwrap().clone();
    whole.as_object_mut().unwrap().extend(fields);
    let mut partial = base.clone();
    partial["capability"] = json!("web-search");
    let mut private = base;
    private["observation"] = json!({"call": "toolu_01", "name": "WebSearch"});
    for checkpoint in [partial, whole, private] {
        let payload = json!({"effect_id": "fx", "checkpoint": checkpoint});
        let error = store
            .append_next(run_id, EventType::EffectCheckpointed, payload, None, None)
            .unwrap_err();
        let StoreError::SeatRecord(refusal) = error else {
            panic!("{error}");
        };
        let seq = head.0 + 1;
        let want = SeatRecordError {
            seq,
            path: "/".into(),
            contract: V5,
        };
        assert_eq!(refusal, want, "{checkpoint}");
        assert_eq!(store.head_hash(run_id).unwrap(), head);
    }
}

/// D9's preparation proof at this merge: every site shape journals its
/// legacy native rows exactly, the run exports and the export verifies
/// offline, no record carries an attribution field or a private
/// observation, and a direct append of either is refused.
#[test]
fn every_site_shape_journals_its_legacy_native_rows_through_export_and_verify() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let state = root.join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::create_dir_all(root.join("work")).unwrap();
    let store = Store::open(&root.join("forge.db")).unwrap();
    let bundle = bundle(&root, &state);
    let mut engine =
        Engine::start(store, bundle, "Feature: legacy", Some(root.join("work"))).unwrap();
    let end = engine.drive().unwrap();
    assert_eq!(
        end.state.status,
        Status::Completed,
        "{:?}",
        end.state.park_reason
    );

    let run_id = engine.run_id.clone();
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(journaled(&events), wanted());
    // Nothing SC4 reserves, and no private observation, on any record.
    for event in &events {
        for record in [&event.payload["checkpoint"], &event.payload["result"]] {
            for field in UNRECORDED {
                assert_eq!(record.get(field), None, "{field} at seq {}", event.seq);
            }
        }
    }

    let exported = engine.store.export_ndjson(&run_id).unwrap();
    assert_eq!(exported.lines().count(), events.len());
    let verified = brokkr_store::verify_export(&exported).unwrap();
    let journal = (Status::Completed, events.len() as u64);
    assert_eq!((verified.status, verified.seq), journal);
    refuses_unrecorded_fields(&mut engine.store, &run_id);
}
