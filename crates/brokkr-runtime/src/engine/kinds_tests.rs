//! Decision 0075 ruling 2's binding: a fixture harness no code names,
//! `trellis`, declares its transcript and root session kinds in its
//! adapter's `record_kinds`, seats a turn through the engine on a fake
//! driver, and its seat records are appended under seat-record v8, verify,
//! export, verify offline and render. A kind its own harness does not
//! declare — another fixture's, `arbor`'s, loaded beside it, or a built-in
//! driver's — is refused at append wherever the record names it, naming
//! the kind and the seat. A root of its kind is offered to a retry only
//! where its adapter declares a resume for the site that is not
//! `unsupported`.

use std::path::Path;

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_store::{SeatRecordVersion, Store};
use serde_json::{json, Value};

use super::resume::{self, SiteContext, SiteKey};
use super::tests::git_commit;
use super::{Engine, ENGINE_VERSION};
use crate::agents::kinds::Standing;
use crate::agents::{Adapters, DeclaredKinds};
use crate::bundle::{Bundle, SeatClass};

const TRAIL: &str = "trellis-trail";
const THREAD: &str = "trellis-thread";

/// One invocation of the fake trellis driver: the checkpoints it sends,
/// then its successful result, or a failure where there is none.
struct Visit {
    checkpoints: Vec<Value>,
    result: Option<Value>,
}

/// The trellis checkpoint naming `transcript` and `root` as its kinds,
/// its transcript at `locator`, its root `id`.
fn checkpoint(transcript: &str, root: &str, locator: &str, id: &str) -> Value {
    json!({"step": "transcript", "model": "trellis-1",
           "transcript": {"kind": transcript, "locator": locator, "home": "/h/.trellis"},
           "root_session": {"kind": root, "id": id, "harness_version": "1.0.0",
                            "persistent": true}})
}

/// A successful result carrying a transcript of `kind`.
fn complete(kind: &str) -> Value {
    json!({"result": "complete", "model": "trellis-1",
           "transcript": {"kind": kind, "locator": "t-1", "home": "/h/.trellis"}})
}

/// One visit sending `checkpoint` and succeeding with `result`.
fn once(checkpoint: Value, result: Value) -> Vec<Visit> {
    vec![Visit {
        checkpoints: vec![checkpoint],
        result: Some(result),
    }]
}

/// A fake driver that logs every message it is sent to `dir/log` and plays
/// `visits` one per invocation, the last repeating; a panel's `judge`
/// member always plays the first and its `second` member the second. It
/// declares it can receive a resume, so what the engine offers is read
/// off the wire.
fn driver(dir: &Path, visits: &[Visit]) -> String {
    std::fs::create_dir_all(dir).unwrap();
    for n in 1..=2 {
        let visit = &visits[(n - 1).min(visits.len() - 1)];
        let rows: String = (visit.checkpoints.iter())
            .map(|row| format!("{row}\n"))
            .collect();
        std::fs::write(dir.join(format!("{n}.rows")), rows).unwrap();
        let end = match &visit.result {
            Some(result) => format!(r#""status":"succeeded","result":{result},"error":null"#),
            None => r#""status":"failed","error":"a first look""#.to_string(),
        };
        std::fs::write(dir.join(format!("{n}.end")), end).unwrap();
    }
    let at = dir.display();
    format!(
        r#"
n=$(cat '{at}/count' 2>/dev/null || echo 0); n=$((n+1)); printf '%s' "$n" > '{at}/count'
read -r hello
printf '%s\n' '{{"proto":"forge-driver/v1","msg_id":"cap","type":"capabilities","driver":"trellis","version":"1","supports":["resume"]}}'
while read -r line; do printf '%s\n' "$line" >> '{at}/log'; case "$line" in *'"type":"start"'*) break ;; esac; done
case "$line" in *'"seat":"work:judge"'*) n=1 ;; *'"seat":"work:second"'*) n=2 ;; esac
eid=$(printf '%s' "$line" | sed -n 's/.*"effect_id":"\([^"]*\)".*/\1/p')
aid=$(printf '%s' "$line" | sed -n 's/.*"attempt_id":"\([^"]*\)".*/\1/p')
head='{{"proto":"forge-driver/v1","msg_id":"m","effect_id":"'"$eid"'","attempt_id":"'"$aid"'"'
printf '%s,"type":"accepted","session_ref":null}}\n' "$head"
while read -r data; do printf '%s,"type":"checkpoint","data":%s}}\n' "$head" "$data"; done < '{at}/'"$n"'.rows'
printf '%s,"type":"result",%s}}\n' "$head" "$(cat '{at}/'"$n"'.end')"
read -r done
"#
    )
}

/// Every message the fake driver at `dir` was sent, in order.
fn received(dir: &Path) -> Vec<Value> {
    let log = std::fs::read_to_string(dir.join("log")).unwrap_or_default();
    log.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn write(path: &Path, value: &Value) {
    std::fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

/// A `resume` declaration of one work shape, `supported` or `unsupported`,
/// measured where the trellis seat stands: no hands.
fn resume_declared(supported: bool) -> Value {
    resume_shaped(supported, NO_HANDS)
}

/// Where a site without hands stands, as the engine writes it into the
/// driver's input: its boundary and its hands.
const NO_HANDS: (&str, &str) = ("not applicable", "none");

/// A `resume` declaration of one work shape, `supported` or `unsupported`,
/// measured under `boundary` with `hands`.
fn resume_shaped(supported: bool, (boundary, hands): (&str, &str)) -> Value {
    let mut shape = json!({"classes": ["work"], "boundaries": [boundary], "hands": hands});
    let measured = json!({"status": "supported",
        "identity": {"version": "1.0.0", "applies_to": "1.0.0"},
        "evidence": {"interface": "fixture", "restrictions": "fixture", "root": "fixture",
                     "accounting": "fixture"}});
    let unsupported = json!({"status": "unsupported", "identity": {"unknown": "fixture"},
                             "reason": "fixture: the trellis harness cannot rejoin"});
    let status = if supported { measured } else { unsupported };
    shape
        .as_object_mut()
        .unwrap()
        .extend(status.as_object().unwrap().clone());
    json!({"headless": shape})
}

/// Two fixture adapters under `adapters`: `trellis`, invoking `script`
/// and declaring [`TRAIL`] and [`THREAD`] with `resume`, and `arbor`,
/// which no seat hires and which declares kinds of its own.
fn fixture_adapters(adapters: &Path, script: &str, resume: &Value) {
    std::fs::create_dir_all(adapters).unwrap();
    let adapter = |provider: &str, model: &str, transcript: &str, root: &str| {
        json!({"provider": provider, "trust_tier": "trusted", "egress": "contracted",
               "binary": "sh", "driver": ["sh", "-c", script],
               "models": {provider: model}, "model_flag": "--model",
               "efforts": ["high"], "effort_flag": "--effort",
               "tool_permissions": "unsupported", "mcp": "unsupported", "resume": resume,
               "record_kinds": {"transcript": transcript, "root_session": root}})
    };
    write(
        &adapters.join("trellis.json"),
        &adapter("trellis", "trellis-1", TRAIL, THREAD),
    );
    write(
        &adapters.join("arbor.json"),
        &adapter("arbor", "arbor-1", "arbor-log", "arbor-root"),
    );
}

/// The `work` seat: it hires the trellis agent and may try twice.
fn single_seat() -> Value {
    json!({"results": ["complete"], "class": "work", "agent": "trellis",
           "limits": {"max_attempts": 2}})
}

/// A `work` panel of two members, `judge` and `second`, each hiring the
/// trellis agent, so each plays the same visit.
fn panel_seat() -> Value {
    let member = json!({"agent": "trellis", "class": "work"});
    json!({"results": ["pass", "fail"], "aggregate": "unanimous-pass",
           "panel": {"judge": member, "second": member}})
}

/// A bundle whose `work` seat is `seat`, and whose trellis adapter
/// invokes `script` and declares `resume`.
fn trellis_bundle(dir: &Path, script: &str, resume: &Value, seat: &Value) -> Bundle {
    let (bundle, agents, adapters) = (dir.join("bundle"), dir.join("agents"), dir.join("adapters"));
    for made in [bundle.clone(), agents.join("charters")] {
        std::fs::create_dir_all(made).unwrap();
    }
    fixture_adapters(&adapters, script, resume);
    std::fs::write(agents.join("charters/trellis.md"), "# trellis\n").unwrap();
    write(
        &agents.join("trellis.json"),
        &json!({"description": "fixture trellis", "charter": "charters/trellis.md",
                "models": ["trellis"], "efforts": {"trellis": "high"}}),
    );
    // The work seat is the trellis seat; the protected review after it
    // names a driver that never spawns, so the run stops there.
    let rule = |from: &str, result: &str, next: &str| {
        json!({"id": format!("{from}-{result}").to_uppercase(), "from": from, "result": result, "next": next,
               "reason": format!("{from} said {result}")})
    };
    let results = seat["results"].as_array().unwrap().iter();
    let mut rules: Vec<Value> = results
        .map(|result| rule("work", result.as_str().unwrap(), "review"))
        .collect();
    rules.push(rule("review", "clean", "done"));
    write(
        &bundle.join("policy.json"),
        &json!({"schema": "forge.phase-machine/v1", "phases": ["work", "review", "done"],
                "initial": "work", "terminal": ["done"], "shippable_from": ["review"],
                "rules": rules}),
    );
    let review = json!({"results": ["clean"], "class": "work", "role": "trellis.md",
                        "driver": {"command": ["no-such-review-driver"]}});
    std::fs::write(bundle.join("trellis.md"), "# review\n").unwrap();
    write(
        &bundle.join("bundle.json"),
        &json!({"name": "trellis", "description": "a harness no code names", "cost": "test",
                "policy": "policy.json", "seats": {"work": seat, "review": review}}),
    );
    Bundle::compile_with(&bundle, &agents, &adapters).unwrap()
}

/// The run the trellis seat drove through `visits`, its journal and its
/// store, with `resume` declared for its adapter.
fn seated_with(
    dir: &Path,
    visits: &[Visit],
    resume: &Value,
) -> (Store, String, Vec<EventEnvelope>) {
    seated_as(dir, visits, resume, &single_seat())
}

/// The run of the `work` seat `seat` through `visits`, as
/// [`seated_with`].
fn seated_as(
    dir: &Path,
    visits: &[Visit],
    resume: &Value,
    seat: &Value,
) -> (Store, String, Vec<EventEnvelope>) {
    let repo = dir.join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "base");
    let store = Store::open(&dir.join("forge.db")).unwrap();
    let script = driver(&dir.join("driver"), visits);
    let bundle = trellis_bundle(dir, &script, resume, seat);
    let mut engine = Engine::start(store, bundle, "trellis", Some(repo)).unwrap();
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    (engine.store, engine.run_id, events)
}

fn seated(dir: &Path, visits: &[Visit]) -> (Store, String, Vec<EventEnvelope>) {
    seated_with(dir, visits, &resume_declared(false))
}

fn of(events: &[EventEnvelope], kind: EventType) -> Vec<&Value> {
    let found = events.iter().filter(|event| event.event_type == kind);
    found.map(|event| &event.payload).collect()
}

/// The work seat's first attempt's terminal payload.
fn first_ending(events: &[EventEnvelope]) -> (EventType, &Value) {
    let ending = events.iter().find(|event| {
        matches!(
            event.event_type,
            EventType::EffectSucceeded | EventType::EffectFailed
        )
    });
    let ending = ending.expect("the work seat's attempt ended");
    (ending.event_type, &ending.payload)
}

/// That the work seat's first attempt failed on exactly `error`.
fn failed_with(events: &[EventEnvelope], error: &str) {
    let (ended, payload) = first_ending(events);
    let failed = (ended, payload["error"].as_str());
    assert_eq!(failed, (EventType::EffectFailed, Some(error)), "{payload}");
}

/// The transcript cells the view renders for the run's seats, each its
/// text and the note an absent one carries.
fn cells(events: &[EventEnvelope]) -> Vec<(String, Option<String>)> {
    let view = brokkr_view::run_view(events, None);
    let cells = view
        .participants
        .into_iter()
        .map(|seat| seat.transcript_cell);
    cells.map(|cell| (cell.text, cell.note)).collect()
}

#[test]
fn a_harness_no_code_names_seats_records_verifies_exports_and_renders() {
    assert_eq!(
        SeatRecordVersion::of_engine(ENGINE_VERSION),
        SeatRecordVersion::V8
    );
    let dir = tempfile::tempdir().unwrap();
    let visits = once(checkpoint(TRAIL, THREAD, "t-1", "tr-1"), complete(TRAIL));
    let (store, run_id, events) = seated(dir.path(), &visits);
    let checkpoint = &of(&events, EventType::EffectCheckpointed)[0]["checkpoint"];
    assert_eq!(
        (
            &checkpoint["transcript"]["kind"],
            &checkpoint["root_session"]["kind"]
        ),
        (&json!(TRAIL), &json!(THREAD))
    );
    let (ended, payload) = first_ending(&events);
    assert_eq!(ended, EventType::EffectSucceeded, "{payload}");
    assert_eq!(payload["result"]["transcript"]["kind"], TRAIL);
    let exported = store.export_ndjson(&run_id).unwrap();
    let verified = brokkr_store::verify_export(&exported).unwrap();
    assert_eq!(verified.seq, events.last().unwrap().seq);
}

/// P5's two rows told apart (the controller's ruling Q4): a transcript a
/// checkpoint reports renders at its own coordinates; one only the
/// successful result reports renders as none recorded, which is today's
/// reading for every kind (`inspect` paints the same cell, see
/// `render::tests::a_result_only_transcript_reads_as_none_recorded_for_every_kind`).
#[test]
fn a_checkpoint_transcript_renders_and_a_result_only_one_reads_as_none_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let mut row = checkpoint(TRAIL, THREAD, "t-1", "tr-1");
    let mut bare = json!({"result": "complete", "model": "trellis-1"});
    let (_store, _run, events) = seated(dir.path(), &once(row.clone(), bare.clone()));
    assert_eq!(first_ending(&events).0, EventType::EffectSucceeded);
    let rendered = (format!("{TRAIL} · t-1 · home /h/.trellis"), None);
    assert!(cells(&events).contains(&rendered), "{:?}", cells(&events));

    let dir = tempfile::tempdir().unwrap();
    row.as_object_mut().unwrap().remove("transcript");
    bare["transcript"] = json!({"kind": TRAIL, "locator": "t-2", "home": "/h/.trellis-2"});
    let (_store, _run, events) = seated(dir.path(), &once(row, bare));
    let (ended, payload) = first_ending(&events);
    assert_eq!(ended, EventType::EffectSucceeded, "{payload}");
    assert_eq!(payload["result"]["transcript"]["locator"], "t-2");
    let none = (
        "—".to_string(),
        Some("no transcript reference recorded".to_string()),
    );
    let cells = cells(&events);
    assert_eq!(
        cells,
        [none.clone(), none],
        "the work seat's and the unspawned review's"
    );
}

/// A kind the trellis harness does not declare — a stray word, the other
/// fixture's, a built-in driver's — at every place a record names one, is
/// refused at append by the writer's typed refusal: no record carrying it
/// is journaled, and the attempt fails on the refusal's own words.
#[test]
fn a_kind_the_seats_harness_does_not_declare_is_refused_wherever_it_is_named() {
    let stray = "trellis-stray";
    let refusal = |member: &str, kind: &str| {
        format!(
            "seat 'work' recorded {member} kind '{kind}', which the seat's harness does not \
             declare (decision 0075 ruling 2); stderr tail: "
        )
    };
    let declared = || checkpoint(TRAIL, THREAD, "t-1", "tr-1");
    let nested = |member: &str| {
        let mut row = declared();
        row["result"] = json!({member: {"kind": stray}});
        row
    };
    for (row, result, member, kind) in [
        (
            checkpoint(stray, THREAD, "t-1", "tr-1"),
            complete(TRAIL),
            "transcript",
            stray,
        ),
        (
            checkpoint(TRAIL, stray, "t-1", "tr-1"),
            complete(TRAIL),
            "root session",
            stray,
        ),
        (declared(), complete(stray), "transcript", stray),
        (
            nested("transcript"),
            complete(TRAIL),
            "nested result's transcript",
            stray,
        ),
        (
            nested("root_session"),
            complete(TRAIL),
            "nested result's root session",
            stray,
        ),
        (
            checkpoint("arbor-log", THREAD, "t-1", "tr-1"),
            complete(TRAIL),
            "transcript",
            "arbor-log",
        ),
        (
            checkpoint(TRAIL, "codex-thread", "t-1", "tr-1"),
            complete(TRAIL),
            "root session",
            "codex-thread",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_store, _run, events) = seated(dir.path(), &once(row, result));
        failed_with(&events, &refusal(member, kind));
        let journaled = serde_json::to_string(&of(&events, EventType::EffectCheckpointed)).unwrap();
        assert!(!journaled.contains(kind), "{journaled}");
    }
}

/// What the retry of a failed first attempt was sent: the second start's
/// private `owned_target`, the `Body::Resume` sent before it, if any, and
/// the `boundary` and `hands` its input carries.
#[derive(Debug, PartialEq)]
struct Retry {
    target: Option<Value>,
    resumes: Vec<Value>,
    standing: (Value, Value),
}

impl Retry {
    /// Whether the retry started cold.
    fn cold(&self) -> bool {
        self.target.is_none() && self.resumes.is_empty()
    }
}

/// The retry of a seat whose first attempt confirmed a persistent root of
/// its harness's declared kind, with `resume` declared for its adapter.
fn retried(resume: &Value) -> Retry {
    retried_after(checkpoint(TRAIL, THREAD, "t-1", "tr-1"), resume)
}

/// The retry of a seat whose first attempt sent `first` and failed.
fn retried_after(first: Value, resume: &Value) -> Retry {
    let dir = tempfile::tempdir().unwrap();
    let visits = [
        Visit {
            checkpoints: vec![first],
            result: None,
        },
        Visit {
            checkpoints: vec![checkpoint(TRAIL, THREAD, "t-1", "tr-2")],
            result: Some(complete(TRAIL)),
        },
    ];
    let (_store, _run, events) = seated_with(dir.path(), &visits, resume);
    assert_eq!(
        first_ending(&events).0,
        EventType::EffectFailed,
        "the first attempt failed"
    );
    let received = received(&dir.path().join("driver"));
    let starts: Vec<&Value> = received.iter().filter(|m| m["type"] == "start").collect();
    assert_eq!(starts.len(), 2, "the failed attempt was retried");
    assert_eq!(
        starts[0]["input"]["resume_context"].get("owned_target"),
        None
    );
    let input = &starts[1]["input"];
    let target = input["resume_context"].get("owned_target").cloned();
    let standing = (input["boundary"].clone(), input["hands"].clone());
    let resumes = received
        .into_iter()
        .filter(|m| m["type"] == "resume")
        .collect();
    Retry {
        target,
        resumes,
        standing,
    }
}

/// A root of a kind no built-in driver writes is offered only where its
/// seat's adapter declares a resume for the site that is not
/// `unsupported` (the controller's ruling Q3): declared unsupported, the
/// retry starts cold; declared supported, the offer travels.
#[test]
fn a_new_root_kind_is_offered_to_a_retry_only_where_its_adapter_declares_a_resume() {
    assert!(retried(&resume_declared(false)).cold());
    let retry = retried(&resume_declared(true));
    assert_eq!(retry.target.unwrap()["provider_id"], "tr-1");
    assert_eq!(retry.resumes.len(), 1, "{:?}", retry.resumes);
    assert_eq!(retry.resumes[0]["session_ref"], "tr-1");
}

/// A root of a new kind is offered only to a site its adapter's resume
/// covers (the controller's ruling R4): a supported shape measured under
/// another boundary, or with other hands, buys no offer; the shape
/// measured where the site stands does, and the boundary and hands it
/// matched are the words the driver's input carries, which its resume
/// gate matches in turn.
#[test]
fn a_new_root_kind_is_offered_only_where_the_resume_shape_covers_the_sites_standing() {
    let (boundary, hands) = NO_HANDS;
    for elsewhere in [("namespace", hands), (boundary, "boxed")] {
        let retry = retried(&resume_shaped(true, elsewhere));
        assert!(retry.cold(), "{elsewhere:?}: {retry:?}");
    }
    let retry = retried(&resume_shaped(true, NO_HANDS));
    assert_eq!(retry.target.unwrap()["provider_id"], "tr-1");
    assert_eq!(retry.resumes.len(), 1, "{:?}", retry.resumes);
    assert_eq!(retry.standing, (json!(boundary), json!(hands)));
}

/// The flat session id is not an offer for a harness that declares its
/// kinds (the controller's ruling R5): the trellis adapter, its resume
/// unsupported, whose first attempt wrote one checkpoint naming no model
/// — so no site stamp — carrying a flat `session_id` beside its declared
/// root, has its retry started cold.
#[test]
fn a_flat_session_id_is_never_an_offer_for_a_harness_that_declares_its_kinds() {
    let mut flat = checkpoint(TRAIL, THREAD, "t-1", "tr-1");
    let flat_fields = flat.as_object_mut().unwrap();
    flat_fields.remove("model");
    flat_fields.insert("session_id".into(), json!("tr-flat"));
    let retry = retried_after(flat, &resume_declared(false));
    assert!(retry.cold(), "{retry:?}");
}

/// A journal that somehow holds a persistent root of a kind the seat's
/// harness does not declare — constructed here, since the writer refuses
/// one — supplies no offer, while its declared kind and a built-in
/// driver's kind do.
#[test]
fn a_root_of_a_kind_the_seats_harness_does_not_declare_is_never_offered() {
    let dir = tempfile::tempdir().unwrap();
    fixture_adapters(dir.path(), "true", &resume_declared(true));
    let adapters = Adapters::load(dir.path()).unwrap();
    let kinds = DeclaredKinds::of(Some(&adapters)).seat(Some("trellis"), &[]);
    let (site, instance) = ("a".repeat(64), "b".repeat(64));
    let standing = Some(Standing {
        boundary: "not applicable",
        hands: "none",
    });
    let context = SiteContext::new(site.clone(), instance.clone(), SeatClass::Work)
        .of_harness(kinds, standing);
    let event = |kind: EventType, payload: Value| {
        crate::envelope_builder::EnvelopeBuilder::new(kind, payload)
            .attempt(Some("attempt-1"))
            .at("2026-11-21T00:00:00Z")
            .hash("c".repeat(64))
            .build()
    };
    let offered = |root: &str| {
        let row = json!({"effect_id": "fx", "checkpoint": {"step": "s", "site_ref": site,
            "instance_ref": instance, "root_session": {"kind": root, "id": "tr-1",
            "harness_version": "1.0.0", "persistent": true}}});
        let events = [
            event(
                EventType::EffectRequested,
                json!({"seat": "work", "effect_id": "fx"}),
            ),
            event(EventType::EffectStarted, json!({"effect_id": "fx"})),
            event(EventType::EffectCheckpointed, row),
        ];
        let key = SiteKey::single("work", None);
        resume::eligible_offer(&events, &key, &context, "work", true, true, |_| false)
            .map(|offer| offer.provider_id)
    };
    assert_eq!(offered("arbor-root"), None);
    assert_eq!(offered(TRAIL), None);
    assert_eq!(offered(THREAD), Some("tr-1".to_string()));
    assert_eq!(offered("claude-session"), Some("tr-1".to_string()));
}

/// The refusal of seat `seat`'s transcript kind `kind`, as journaled.
fn undeclared_transcript(seat: &str, kind: &str) -> String {
    format!(
        "seat '{seat}' recorded transcript kind '{kind}', which the seat's harness does not \
         declare (decision 0075 ruling 2)"
    )
}

/// A panel member is judged by its own harness as a single seat is (the
/// controller's ruling G): the `judge` member's live checkpoint, and its
/// successful result, each naming a kind the trellis harness does not
/// declare, are refused, and no checkpoint carrying it is journaled,
/// while its sibling, which names none, passes.
#[test]
fn a_panel_members_undeclared_kind_is_refused_live_and_at_its_result() {
    let stray = "trellis-stray";
    let pass = |kind: &str| {
        json!({"result": "pass", "model": "trellis-1",
               "transcript": {"kind": kind, "locator": "t-1", "home": "/h/.trellis"}})
    };
    for (row, result) in [
        (checkpoint(stray, THREAD, "t-1", "tr-1"), pass(TRAIL)),
        (checkpoint(TRAIL, THREAD, "t-1", "tr-1"), pass(stray)),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let judge = Visit {
            checkpoints: vec![row],
            result: Some(result),
        };
        let second = Visit {
            checkpoints: Vec::new(),
            result: Some(pass(TRAIL)),
        };
        let visits = [judge, second];
        let resume = resume_declared(false);
        let (_store, _run, events) = seated_as(dir.path(), &visits, &resume, &panel_seat());
        let error = format!(
            "panel members failed — judge: {}",
            undeclared_transcript("work:judge", stray)
        );
        failed_with(&events, &error);
        let journaled = serde_json::to_string(&of(&events, EventType::EffectCheckpointed)).unwrap();
        assert!(!journaled.contains(stray), "{journaled}");
    }
}

/// Decision 0109's refusals stay 0109's (the controller's ruling R7): a
/// live checkpoint that claims the engine's `authored` record, and a work
/// site's result that reports findings, each also naming a kind the
/// trellis harness does not declare, fail their attempt on 0109's
/// refusal, in the words f26bccae journals, and never on the kind's.
#[test]
fn a_record_decision_0109_refuses_is_refused_by_0109_whatever_kind_it_names() {
    let stray = "trellis-stray";
    let mut authored = checkpoint(stray, THREAD, "t-1", "tr-1");
    authored["authored"] = json!("engine");
    let mut findings = complete(stray);
    findings["findings"] = json!([]);
    for (row, result, error) in [
        (
            authored,
            complete(TRAIL),
            "a driver's checkpoint claims a record only the engine authors \
             (decision 0109 ruling 1); stderr tail: ",
        ),
        (
            checkpoint(TRAIL, THREAD, "t-1", "tr-1"),
            findings,
            "the result was refused: the result carries 'findings' from a work site: only a \
             gate reports findings (decision 0109 ruling 1); stderr tail: ",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_store, _run, events) = seated(dir.path(), &once(row, result));
        failed_with(&events, error);
    }
}
