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
use brokkr_protocol::process::Unsettled;
use brokkr_protocol::{AttemptReport, Cleanup};
use brokkr_store::{SeatRecordVersion, Store};
use serde_json::{json, Value};

use super::carry::fixture_tests::{parked_at_source, retry};
use super::carry::{CarryRefusal, Refusal};
use super::resume::{self, SiteContext, SiteKey};
use super::tests::git_commit;
use super::{Engine, FencedCommandOutcome, MemberRetryOutcome, ENGINE_VERSION};
use crate::agents::kinds::Standing;
use crate::agents::{Adapters, DeclaredKinds};
use crate::bundle::{Bundle, SeatClass};

const TRAIL: &str = "trellis-trail";
const THREAD: &str = "trellis-thread";

/// One invocation of the fake trellis driver: the checkpoints it sends,
/// then its successful result, or a failure where there is none, and
/// whether its report is received with its cleanup unresolved
/// ([`unresolved_where_marked`]).
#[derive(Default)]
struct Visit {
    checkpoints: Vec<Value>,
    result: Option<Value>,
    unresolved: bool,
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
        unresolved: false,
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
        let mark = if visit.unresolved { UNRESOLVED } else { "" };
        std::fs::write(dir.join(format!("{n}.err")), mark).unwrap();
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
cat '{at}/'"$n"'.err' >&2
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

/// The trellis adapter's `resume` declaration, and `by`, the shape its
/// root session kind names as the one its seats rejoin by
/// (`record_kinds.resume_shape`; the controller's ruling R14).
struct Rejoin {
    resume: Value,
    by: &'static str,
}

/// A `resume` declaration of one work shape, `supported` or `unsupported`,
/// measured where the trellis seat stands: no hands.
fn resume_declared(supported: bool) -> Rejoin {
    resume_shaped(supported, NO_HANDS)
}

/// Where a site without hands stands, as the engine writes it into the
/// driver's input: its boundary and its hands.
const NO_HANDS: (&str, &str) = ("not applicable", "none");

/// A `resume` declaration of one work shape, `headless`, which the root
/// session kind rejoins by, `supported` or `unsupported`, measured under
/// `boundary` with `hands`.
fn resume_shaped(supported: bool, standing: (&str, &str)) -> Rejoin {
    Rejoin {
        resume: json!({"headless": shape_of(supported, standing)}),
        by: "headless",
    }
}

/// One work shape, `supported` or `unsupported`, measured under
/// `boundary` with `hands`.
fn shape_of(supported: bool, (boundary, hands): (&str, &str)) -> Value {
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
    shape
}

/// Two fixture adapters under `adapters`: `trellis`, invoking `script`
/// and declaring [`TRAIL`] and [`THREAD`] with `rejoin`, and `arbor`,
/// which no seat hires and which declares kinds of its own.
fn fixture_adapters(adapters: &Path, script: &str, rejoin: &Rejoin) {
    std::fs::create_dir_all(adapters).unwrap();
    let adapter = |provider: &str, model: &str, transcript: &str, root: &str| {
        json!({"provider": provider, "trust_tier": "trusted", "egress": "contracted",
               "binary": "sh", "driver": ["sh", "-c", script],
               "models": {provider: model}, "model_flag": "--model",
               "efforts": ["high"], "effort_flag": "--effort",
               "tool_permissions": "unsupported", "mcp": "unsupported",
               "resume": rejoin.resume, "record_kinds": {"transcript": transcript,
               "root_session": root, "resume_shape": rejoin.by}})
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
fn trellis_bundle(dir: &Path, script: &str, resume: &Rejoin, seat: &Value) -> Bundle {
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
    resume: &Rejoin,
) -> (Store, String, Vec<EventEnvelope>) {
    seated_as(dir, visits, resume, &single_seat())
}

/// The run of the `work` seat `seat` through `visits`, as
/// [`seated_with`].
fn seated_as(
    dir: &Path,
    visits: &[Visit],
    resume: &Rejoin,
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
             declare (decision 0075 ruling 2); driver stderr: empty"
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
/// the `boundary` and `hands` its input carries, and that whole input.
#[derive(Debug, PartialEq)]
struct Retry {
    target: Option<Value>,
    resumes: Vec<Value>,
    standing: (Value, Value),
    input: Value,
}

impl Retry {
    /// Whether the retry started cold.
    fn cold(&self) -> bool {
        self.target.is_none() && self.resumes.is_empty()
    }
}

/// The retry of a seat whose first attempt confirmed a persistent root of
/// its harness's declared kind, with `resume` declared for its adapter.
fn retried(resume: &Rejoin) -> Retry {
    retried_after(checkpoint(TRAIL, THREAD, "t-1", "tr-1"), resume)
}

/// The retry of a seat whose first attempt sent `first` and failed.
fn retried_after(first: Value, resume: &Rejoin) -> Retry {
    let dir = tempfile::tempdir().unwrap();
    let visits = [
        Visit {
            checkpoints: vec![first],
            ..Visit::default()
        },
        Visit {
            checkpoints: vec![checkpoint(TRAIL, THREAD, "t-1", "tr-2")],
            result: Some(complete(TRAIL)),
            unresolved: false,
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
    let input = starts[1]["input"].clone();
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
        input,
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
    let context = trellis_site(dir.path());
    let offered = |root: &str| offered_over(&context, vec![rooted(&context, root, "tr-1")]);
    assert_eq!(offered("arbor-root"), None);
    assert_eq!(offered(TRAIL), None);
    assert_eq!(offered(THREAD), Some("tr-1".to_string()));
    assert_eq!(offered("claude-session"), Some("tr-1".to_string()));
}

/// One event of attempt `attempt-1`, as the journal holds it.
fn journaled((kind, payload): (EventType, Value)) -> EventEnvelope {
    crate::envelope_builder::EnvelopeBuilder::new(kind, payload)
        .attempt(Some("attempt-1"))
        .at("2026-11-21T00:00:00Z")
        .hash("c".repeat(64))
        .build()
}

/// A checkpoint at `context`'s site confirming the persistent root `id`
/// of kind `kind`.
fn rooted(context: &SiteContext, kind: &str, id: &str) -> (EventType, Value) {
    let row = json!({"effect_id": "fx", "checkpoint": {"step": "s",
        "site_ref": context.site_ref, "instance_ref": context.instance_ref,
        "root_session": {"kind": kind, "id": id, "harness_version": "1.0.0",
                         "persistent": true}}});
    (EventType::EffectCheckpointed, row)
}

/// The root the `work` seat's site `context` is offered over `rows`,
/// journaled after the seat's request and start.
fn offered_over(context: &SiteContext, rows: Vec<(EventType, Value)>) -> Option<String> {
    target_over(context, rows, false).map(|offer| offer.provider_id)
}

/// The target the `work` seat's single site `context` is offered over
/// `rows`, journaled after the seat's request and start, its legacy
/// instance holding where `legacy` says so.
fn target_over(
    context: &SiteContext,
    rows: Vec<(EventType, Value)>,
    legacy: bool,
) -> Option<resume::ResumeTarget> {
    let mut events = vec![
        journaled((
            EventType::EffectRequested,
            json!({"seat": "work", "effect_id": "fx"}),
        )),
        journaled((EventType::EffectStarted, json!({"effect_id": "fx"}))),
    ];
    events.extend(rows.into_iter().map(journaled));
    let key = SiteKey::single("work", None);
    resume::eligible_offer(&events, &key, context, "work", true, true, |_| legacy)
}

/// A journal in a temporary store of its own, whose `work` seat's effect
/// was requested: a row a seat's writer would write lands by the engine's
/// own sink, stamped by its site's stamper and judged by its harness's
/// kinds, and every read is of what the store holds (the controller's
/// ruling R35).
struct Journal {
    _dir: tempfile::TempDir,
    store: Store,
    cause: Option<String>,
    open: Option<String>,
}

impl Journal {
    fn new() -> Journal {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
        store
            .create_run(
                "run",
                "feature",
                "test",
                &json!({"files": {}, "engine": ENGINE_VERSION}),
            )
            .unwrap();
        for (kind, payload) in [
            (
                EventType::RunStarted,
                json!({"feature": "feature", "manifest": {}}),
            ),
            (EventType::PhaseEntered, json!({"phase": "work"})),
            (
                EventType::EffectRequested,
                json!({"effect_id": "effect", "seat": "work"}),
            ),
        ] {
            store.append_next("run", kind, payload, None, None).unwrap();
        }
        let (cause, open) = (None, None);
        Journal {
            _dir: dir,
            store,
            cause,
            open,
        }
    }

    /// Start attempt `attempt`, failing the one open before it.
    fn attempt(&mut self, attempt: &str) -> &mut Self {
        if let Some(open) = self.open.take() {
            let failed = json!({"effect_id": "effect", "attempt_id": open, "error": "again"});
            self.append(EventType::EffectFailed, failed, &open);
        }
        let started = json!({"effect_id": "effect", "attempt_id": attempt});
        self.append(EventType::EffectStarted, started, attempt);
        self.open = Some(attempt.to_string());
        self
    }

    /// The open attempt's `row` at `context`'s site, carrying the native
    /// call `call` where one is named, written by the engine's sink.
    fn wrote(&mut self, context: &SiteContext, row: Value, call: Option<&str>) -> &mut Self {
        self.wrote_by(context, None, row, call)
    }

    /// As [`Journal::wrote`], the row tagged first with `member` where one
    /// is named, as the engine tags a panel member's row before its stamp.
    fn wrote_by(
        &mut self,
        context: &SiteContext,
        member: Option<&str>,
        row: Value,
        call: Option<&str>,
    ) -> &mut Self {
        let row = match member {
            Some(member) => super::tag_member(row, member),
            None => row,
        };
        let attempt = self.open.clone().unwrap();
        let held = self.events().len();
        let at = super::checkpoints::Attempt {
            run_id: "run",
            effect_id: "effect",
            attempt_id: &attempt,
        };
        let mut sink = super::checkpoints::Checkpoints::new(&mut self.store, &mut self.cause, at);
        let (kinds, seat) = (&context.kinds, "work");
        let declared = super::checkpoints::Declared { kinds, seat };
        let call = Ok(call.map(super::capability_calls::Stamp::native_search));
        let owner = member.unwrap_or("");
        sink.offer_live(owner, context.stamp(row), call, Some(context), declared);
        sink.settle().unwrap();
        assert_eq!(self.events().len(), held + 1, "the sink journaled the row");
        self
    }

    /// The open attempt's `row` at `context`'s site, stamped by its
    /// stamper and appended to the store directly: only for a row the
    /// seat's write check refuses, which the sink would never journal.
    fn appended(&mut self, context: &SiteContext, row: Value) -> &mut Self {
        let attempt = self.open.clone().unwrap();
        let checkpoint = context.stamp(row);
        let payload =
            json!({"effect_id": "effect", "attempt_id": attempt, "checkpoint": checkpoint});
        self.append(EventType::EffectCheckpointed, payload, &attempt);
        self
    }

    fn append(&mut self, kind: EventType, payload: Value, attempt: &str) {
        let (cause, attempt) = (self.cause.take(), Some(attempt.to_string()));
        let appended = self.store.append_next("run", kind, payload, cause, attempt);
        self.cause = Some(appended.unwrap().event_id);
    }

    fn events(&self) -> Vec<EventEnvelope> {
        self.store.load("run").unwrap()
    }

    /// The target the `work` seat's single site `context` is offered over
    /// what the store holds, its legacy instance holding where `legacy`
    /// says so.
    fn offer(&self, context: &SiteContext, legacy: bool) -> Option<resume::ResumeTarget> {
        let key = SiteKey::single("work", None);
        resume::eligible_offer(&self.events(), &key, context, "work", true, true, |_| {
            legacy
        })
    }
}

/// A row the seat's model wrote, its persistent root of kind `root` and
/// id `id`.
fn root_row(root: &str, id: &str) -> Value {
    json!({"step": "s", "model": "m",
           "root_session": {"kind": root, "id": id, "harness_version": "1.0.0",
                            "persistent": true}})
}

/// A row the seat's model wrote, its persistent root `ds-1` of kind `root`
/// beside a transcript of `kind`.
fn dsh_row(root: &str, kind: &str) -> Value {
    let mut row = root_row(root, "ds-1");
    row["transcript"] = json!({"kind": kind, "locator": "sessions/s-1", "home": "/h/.dsh"});
    row
}

/// The locator and home the offer over one attempt's `row` at `context`
/// forwards, its root `ds-1`: written by the sink where `written`, and
/// otherwise appended directly, as the seat's write check refuses it.
fn coordinates_of(context: &SiteContext, row: Value, written: bool) -> Coordinates {
    let mut journal = Journal::new();
    journal.attempt("attempt-1");
    match written {
        true => journal.wrote(context, row, None),
        false => journal.appended(context, row),
    };
    let offer = journal.offer(context, false).unwrap();
    assert_eq!(offer.provider_id, "ds-1");
    (offer.persistence_locator, offer.persistence_home)
}

type Coordinates = (Option<String>, Option<String>);

/// The coordinates [`dsh_row`] forwards.
fn forwarded() -> Coordinates {
    (
        Some("sessions/s-1".to_string()),
        Some("/h/.dsh".to_string()),
    )
}

/// The controller's ruling R26: a stamped row whose persistent
/// `dsh-session` root stands beside a transcript of a kind the seat's
/// harness does not declare (`trellis-trail`, at a site that declares
/// none) is read as holding no transcript: its root is offered with no
/// locator and no home, as a row without a transcript is. The same row
/// beside a declared kind forwards both, a built-in one at that site, or
/// `trellis-trail` beside the trellis seat's own root. Every row is
/// written by the sink but the first, which is appended directly because
/// the writer's kind check refuses `trellis-trail` at that site.
#[test]
fn a_transcript_of_a_kind_the_seat_does_not_declare_forwards_no_coordinates() {
    let dir = tempfile::tempdir().unwrap();
    let plain = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let trellis = trellis_site(dir.path());
    let dsh = "dsh-session";
    assert_eq!(
        coordinates_of(&plain, dsh_row(dsh, TRAIL), false),
        (None, None)
    );
    let bare = root_row(dsh, "ds-1");
    assert_eq!(coordinates_of(&plain, bare.clone(), true), (None, None));
    assert_eq!(coordinates_of(&plain, dsh_row(dsh, dsh), true), forwarded());
    assert_eq!(
        coordinates_of(&trellis, dsh_row(THREAD, TRAIL), true),
        forwarded()
    );
    // A transcript naming no kind is one no store holds (seat-record v8
    // requires its kind), so this read alone is of a journal built by hand.
    let mut kindless = rooted(&plain, dsh, "ds-1");
    kindless.1["checkpoint"]["transcript"] = json!({"locator": "sessions/s-1", "home": "/h"});
    let kindless = target_over(&plain, vec![kindless], false).unwrap();
    assert_eq!(kindless.persistence_locator, None);
}

/// A site of the built-in DSH driver's harness, answering to the shipped
/// `dsh` adapter, which declares no kinds of its own: its write check
/// admits `dsh-session` and `none` alone.
fn dsh_site() -> SiteContext {
    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let adapters = Adapters::load(&workspace.canonicalize().unwrap().join("adapters")).unwrap();
    let argv = ["{brokkr}", "driver", "dsh", "--"].map(String::from);
    let kinds = DeclaredKinds::of(Some(&adapters)).seat(Some("dsh"), &argv);
    SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work).of_harness(kinds, None)
}

/// The controller's ruling Y1 of 2026-11-22: a Qoder seat's retry is
/// offered nothing. Its level 1 keeps no session and its driver refuses a
/// start offered a resume (#1178), so neither of v7's roads is its: a
/// stamped persistent root of each built-in kind, and decision 0030's
/// flat session id on an unstamped row, each offered at a site of no
/// harness, are offered nothing at a Qoder site. The rows are built by
/// hand, as the writer refuses each root at a Qoder seat and no Qoder
/// driver writes a flat id. Nor is a stamped persistent root of the kind
/// a Qoder seat's adapter declares, under a supported shape covering it.
#[test]
fn a_qoder_seats_retry_is_offered_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let shipped = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/examples/adapters/qoder.json");
    std::fs::copy(shipped, dir.path().join("qoder.json")).unwrap();
    let adapters = Adapters::load(dir.path()).unwrap();
    let argv = ["{brokkr}", "driver", "qoder", "--"].map(String::from);
    let kinds = DeclaredKinds::of(Some(&adapters)).seat(Some("qoder"), &argv);
    let plain = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let qoder = plain.clone().of_harness(kinds, None);
    let flat = json!({"effect_id": "fx", "checkpoint": {"step": "s", "session_id": "flat-1"}});
    let flat = vec![(EventType::EffectCheckpointed, flat)];
    let offered = |context: &SiteContext, rows| target_over(context, rows, true);
    assert_eq!(offered(&plain, flat.clone()).unwrap().provider_id, "flat-1");
    assert_eq!(offered(&qoder, flat), None);
    for kind in ["claude-session", "codex-thread", "dsh-session"] {
        let rows = |context| vec![rooted(context, kind, "q-1")];
        assert_eq!(offered(&plain, rows(&plain)).unwrap().provider_id, "q-1");
        assert_eq!(offered(&qoder, rows(&qoder)), None, "{kind}");
    }
    // An adapter declaring a root and a supported shape that covers the
    // site opens no road for a seat whose command runs the Qoder driver:
    // the trellis root is offered at the trellis site, not at this one.
    let adapters = dir.path().join("trellis");
    let trellis = trellis_site(&adapters);
    let adapters = Adapters::load(&adapters).unwrap();
    let kinds = DeclaredKinds::of(Some(&adapters)).seat(Some("trellis"), &argv);
    let standing = Standing {
        boundary: NO_HANDS.0,
        hands: NO_HANDS.1,
    };
    let declared = plain.of_harness(kinds, Some(standing));
    let rows = |context| vec![rooted(context, THREAD, "t-1")];
    assert_eq!(
        offered(&trellis, rows(&trellis)).unwrap().provider_id,
        "t-1"
    );
    assert_eq!(offered(&declared, rows(&declared)), None);
}

/// The controller's ruling R32: the offer's transcript check is the write
/// check, a built-in kind included. At a DSH seat, a persistent
/// `dsh-session` root beside a `codex-thread` transcript is offered with
/// no locator and no home, since a write of that row there is refused;
/// that row is appended directly for that reason. The same root beside
/// the seat's own `dsh-session` transcript, written by the sink, forwards
/// both.
#[test]
fn a_built_in_transcript_kind_the_seats_harness_does_not_declare_forwards_no_coordinates() {
    let dsh = dsh_site();
    let foreign = dsh_row("dsh-session", "codex-thread");
    assert_eq!(coordinates_of(&dsh, foreign, false), (None, None));
    let own = dsh_row("dsh-session", "dsh-session");
    assert_eq!(coordinates_of(&dsh, own, true), forwarded());
}

/// The controller's ruling R27: at an eligible single seat whose harness
/// declares no kinds, a newer unstamped row whose transcript names a kind
/// outside v7's vocabulary (`trellis-trail`) beside a flat `session_id`
/// yields no handle — not its flat id — and ends the scan, so the older
/// `codex-thread` row's handle is not offered either. The older row alone
/// offers its handle, and a newer `claude-session` row offers its flat
/// id, as before. Each row names no model, so the stamper leaves it
/// unstamped; each is written by the sink but the `trellis-trail` row,
/// which is appended directly because the writer's kind check refuses it.
#[test]
fn a_legacy_row_of_a_kind_outside_v7_is_no_handle_and_no_older_row_stands_in() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let transcript = |kind: &str, locator: &str| {
        json!({"step": "s", "session_id": "flat-1",
               "transcript": {"kind": kind, "locator": locator, "home": "/h"}})
    };
    let older = json!({"step": "s", "transcript":
        {"kind": "codex-thread", "locator": "th-old", "home": "/h/.codex"}});
    let offered = |newer: Option<(Value, bool)>| {
        let mut journal = Journal::new();
        journal
            .attempt("attempt-1")
            .wrote(&context, older.clone(), None);
        match newer {
            Some((row, true)) => journal.wrote(&context, row, None),
            Some((row, false)) => journal.appended(&context, row),
            None => &mut journal,
        };
        journal.offer(&context, true).map(|offer| offer.provider_id)
    };
    assert_eq!(offered(None).as_deref(), Some("th-old"));
    assert_eq!(offered(Some((transcript(TRAIL, "tl-1"), false))), None);
    let claude = transcript("claude-session", "c-1");
    assert_eq!(offered(Some((claude, true))).as_deref(), Some("flat-1"));
}

/// The controller's ruling R28: the offered `claude-session` root's
/// history at its site holds its own attempt and calls only, never an
/// older `trellis-thread` root's of the same id there. The offer is the
/// one `eligible_offer` takes from the journal (the controller's ruling
/// R34). Each row is written by the sink, each call stamped as the
/// engine stamps a native call, but the older root's row, which is
/// appended directly because the writer's kind check refuses
/// `trellis-thread` at this site.
#[test]
fn an_offers_history_is_its_roots_kind_and_id_both() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let mut journal = Journal::new();
    (journal.attempt("older"))
        .appended(&context, root_row(THREAD, "same-1"))
        .wrote(&context, call_row(), Some("n-older"));
    (journal.attempt("newer"))
        .wrote(&context, root_row("claude-session", "same-1"), None)
        .wrote(&context, call_row(), Some("n-newer"));
    let history = history_of(&journal, context);
    assert_eq!(history, history_holding("newer", "n-newer"));
}

/// A row the seat's model wrote observing one turn, its call stamped by
/// the sink.
fn call_row() -> Value {
    json!({"step": "s", "model": "m", "turn": 1})
}

/// The history of the root `context` is offered over what `journal`'s
/// store holds, the offer taken from it by `eligible_offer`.
fn history_of(journal: &Journal, context: SiteContext) -> resume::RootHistory {
    history_at(journal, context, &SiteKey::single("work", None))
}

/// As [`history_of`], at the `work` seat's site `key`.
fn history_at(journal: &Journal, context: SiteContext, key: &SiteKey) -> resume::RootHistory {
    let events = journal.events();
    let offer = resume::eligible_offer(&events, key, &context, "work", true, true, |_| false);
    context.offered(&events, key, Some(&offer.unwrap())).history
}

/// A history of the one attempt `attempt` and its one call `call`.
fn history_holding(attempt: &str, call: &str) -> resume::RootHistory {
    resume::RootHistory {
        attempts: [attempt.to_string()].into(),
        calls: [call.to_string()].into(),
    }
}

/// The controller's ruling R33: an attempt holding another root is not
/// the offered root's history. The `mixed` attempt's rooted rows are a
/// `trellis-thread` root carrying a native call and then the offered
/// `claude-session` root, both of id `same-1`: it contributes neither
/// itself nor that call, while the `clean` attempt, whose two rooted rows
/// both hold the offered root, contributes itself and its call. Every row
/// is written by the sink but the `trellis-thread` one, which is appended
/// directly, its call stamped as the sink stamps one, because the
/// writer's kind check refuses `trellis-thread` at this site.
#[test]
fn an_attempt_holding_another_root_is_not_the_offered_roots_history() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let claude = || root_row("claude-session", "same-1");
    let mut other = root_row(THREAD, "same-1");
    other["turn"] = json!(1);
    super::capability_calls::Stamp::native_search("n-mixed").apply(&mut other);
    let mut journal = Journal::new();
    (journal.attempt("clean"))
        .wrote(&context, claude(), None)
        .wrote(&context, call_row(), Some("n-clean"))
        .wrote(&context, claude(), None);
    (journal.attempt("mixed"))
        .appended(&context, other)
        .wrote(&context, claude(), None);
    let history = history_of(&journal, context);
    assert_eq!(history, history_holding("clean", "n-clean"));
}

/// The review of 880e4792 (the controller's ruling R33): the attempt
/// holding a later root of the offered kind under another id is excluded,
/// so admission reads every rooted row, not the first. The `mixed`
/// attempt's rows are the offered `claude-session` root `same-1`, a call,
/// and then a `claude-session` root `other-1`; the journal's offer stays
/// on `same-1` because the newest attempt holds it. Every row is written
/// by the sink.
#[test]
fn an_attempt_holding_a_later_root_of_another_id_is_not_the_offered_roots_history() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let claude = |id: &str| root_row("claude-session", id);
    let mut journal = Journal::new();
    (journal.attempt("mixed"))
        .wrote(&context, claude("same-1"), None)
        .wrote(&context, call_row(), Some("n-mixed"))
        .wrote(&context, claude("other-1"), None);
    (journal.attempt("clean"))
        .wrote(&context, claude("same-1"), None)
        .wrote(&context, call_row(), Some("n-clean"));
    let history = history_of(&journal, context);
    assert_eq!(history, history_holding("clean", "n-clean"));
}

/// The review of 880e4792 (the controller's ruling R33): at an inline
/// work site, a rooted row naming no model is admitted and left unstamped
/// by the stamper, so it names no site; an attempt that writes such a row
/// holding the `codex-thread` root `old-1` and then a stamped row holding
/// the offered `claude-session` root is not that root's history, and its
/// call is not either. Every row is written by the sink.
#[test]
fn an_attempt_whose_unstamped_row_holds_another_root_is_not_the_offered_roots_history() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let mut unstamped = root_row("codex-thread", "old-1");
    unstamped.as_object_mut().unwrap().remove("model");
    let mut journal = Journal::new();
    (journal.attempt("clean"))
        .wrote(&context, root_row("claude-session", "same-1"), None)
        .wrote(&context, call_row(), Some("n-clean"));
    (journal.attempt("mixed"))
        .wrote(&context, unstamped, None)
        .wrote(&context, root_row("claude-session", "same-1"), None)
        .wrote(&context, call_row(), Some("n-mixed"));
    let events = journal.events();
    let unstamped = events.iter().find_map(|event| {
        let checkpoint = event.payload.get("checkpoint")?;
        (checkpoint.pointer("/root_session/id")? == "old-1").then_some(checkpoint)
    });
    assert_eq!(unstamped.unwrap().get("site_ref"), None);
    let history = history_of(&journal, context);
    assert_eq!(history, history_holding("clean", "n-clean"));
}

/// The review of b76fc1e2 (the controller's ruling R33): in one panel
/// attempt, an unstamped row is judged by the member the engine tags it
/// with. Member `beta` writes a row naming no model, so unstamped but
/// tagged `beta`, holding the `codex-thread` root `old-1`, then its own
/// `claude-session` root `b-1` and a call; member `alpha` writes its
/// `claude-session` root `same-1` and a call. Alpha's history holds the
/// attempt and its call, since beta's row is not alpha's; beta's holds
/// neither, since its own row holds another root. Every row is written by
/// the sink, tagged as the engine tags a member's row.
#[test]
fn a_panel_members_unstamped_row_is_judged_at_its_own_member_alone() {
    let site = |site_ref: char| {
        SiteContext::new(
            site_ref.to_string().repeat(64),
            "b".repeat(64),
            SeatClass::Work,
        )
    };
    let (alpha, beta) = (site('a'), site('c'));
    let mut unstamped = root_row("codex-thread", "old-1");
    unstamped.as_object_mut().unwrap().remove("model");
    let mut journal = Journal::new();
    (journal.attempt("shared"))
        .wrote_by(
            &alpha,
            Some("alpha"),
            root_row("claude-session", "same-1"),
            None,
        )
        .wrote_by(&alpha, Some("alpha"), call_row(), Some("n-alpha"))
        .wrote_by(&beta, Some("beta"), unstamped, None)
        .wrote_by(&beta, Some("beta"), root_row("claude-session", "b-1"), None)
        .wrote_by(&beta, Some("beta"), call_row(), Some("n-beta"));
    let key = |member: &str, index| SiteKey::panel_member("work", None, member, index);
    let history = history_at(&journal, alpha, &key("alpha", 0));
    assert_eq!(history, history_holding("shared", "n-alpha"));
    let history = history_at(&journal, beta, &key("beta", 1));
    assert_eq!(history, resume::RootHistory::default());
}

/// The review of f3e6fc12 (the controller's ruling R38): a single seat's
/// key carries no tag, so a `member` on its row is the driver's own. An
/// attempt whose unstamped row, naming no model and carrying the driver's
/// `member` `beta`, holds the `codex-thread` root `old-1`, then whose
/// stamped row holds the offered root `current-1` and its native call on
/// one checkpoint, is not that root's history; without the first row it
/// is. Every row is written by the sink.
#[test]
fn a_single_seats_unstamped_row_counts_whatever_member_its_driver_wrote() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let mut unstamped = root_row("codex-thread", "old-1");
    unstamped.as_object_mut().unwrap().remove("model");
    unstamped["member"] = json!("beta");
    let mut rooted_call = root_row("claude-session", "current-1");
    rooted_call["turn"] = json!(1);
    let journal = |first: Option<Value>| {
        let mut journal = Journal::new();
        let attempt = journal.attempt("mixed");
        let attempt = match first {
            Some(row) => attempt.wrote(&context, row, None),
            None => attempt,
        };
        attempt.wrote(&context, rooted_call.clone(), Some("n-mixed"));
        journal
    };
    let mixed = journal(Some(unstamped));
    let offer = mixed.offer(&context, false).unwrap();
    assert_eq!(offer.provider_id, "current-1");
    let history = history_of(&mixed, context.clone());
    assert_eq!(history, resume::RootHistory::default());
    let history = history_of(&journal(None), context);
    assert_eq!(history, history_holding("mixed", "n-mixed"));
}

/// The controller's rulings R22 item 3 and R24, the resume offer: a site
/// whose harness declares no kinds, an inline one, meeting a root of a
/// kind in the grammar it does not know (`trellis-thread`) on its newest
/// rooted row is offered nothing: it does not guess, no older root of a
/// built-in kind behind that row stands in, and the row's other facts
/// still read.
#[test]
fn a_root_kind_the_site_does_not_know_is_no_offer_and_no_older_root_stands_in() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let older = rooted(&context, "claude-session", "c-1");
    let only_older = offered_over(&context, vec![older.clone()]);
    assert_eq!(only_older, Some("c-1".to_string()));
    let mut newer = rooted(&context, THREAD, "tr-1");
    newer.1["checkpoint"]["root_session"]["harness_version"] = json!("2.0.0");
    let rows = vec![older, newer];
    assert_eq!(offered_over(&context, rows.clone()), None);
    let events: Vec<EventEnvelope> = rows.into_iter().map(journaled).collect();
    let read = resume::originating_root(&events, &context.site_ref).unwrap();
    assert_eq!(read.harness_version.as_deref(), Some("2.0.0"));
}

/// The controller's rulings R22 items 2 and 3, the resume offer: a root of
/// a kind the site knows that stands only inside evidence — a result's
/// `notes.members`, decide's `inputs.raw_result`, an unresolved cleanup's
/// `received` — is offered nothing; the same root at a checkpoint's
/// seat-record place is.
#[test]
fn a_root_standing_only_inside_evidence_is_never_offered() {
    let context = SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work);
    let placed = rooted(&context, "claude-session", "c-1");
    let record = placed.1["checkpoint"].clone();
    let mut rows = vec![
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "fx", "result": {"result": "pass",
                   "notes": {"members": {"judge": record}}}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"from": "work", "inputs": {"raw_result": record}}),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "fx", "received": {"status": "succeeded", "result": record}}),
        ),
    ];
    assert_eq!(offered_over(&context, rows.clone()), None);
    rows.push(placed);
    assert_eq!(offered_over(&context, rows), Some("c-1".to_string()));
}

/// The controller's ruling R22 item 2: a run resumed at decide over a
/// journaled result that fails its schema appends it whole as
/// `inputs.raw_result`, byte for byte as main does, a kind no harness
/// declares under `notes.members` included: the copy is evidence, and
/// adds no record and no kind place.
#[test]
fn a_run_resumed_at_decide_appends_a_schema_invalid_result_whole_as_its_raw_result() {
    let (dir, mut engine) = super::tests::engine(super::tests::single_body(vec!["driver".into()]));
    let stray = json!({"kind": "trellis-stray", "locator": "t-1", "home": "/h"});
    let result = json!({"result": "undeclared",
                        "notes": {"members": {"judge": {"result": "pass", "transcript": stray}}}});
    let requested = json!({"effect_id": "effect", "seat": "work"});
    let started = json!({"effect_id": "effect", "attempt_id": "attempt"});
    let mut succeeded = started.clone();
    succeeded["result"] = result.clone();
    let phase = json!({"phase": "work"});
    let kinds = [EventType::PhaseEntered, EventType::EffectRequested];
    let written = kinds.into_iter().zip([phase, requested]).chain([
        (EventType::EffectStarted, started),
        (EventType::EffectSucceeded, succeeded),
    ]);
    for (kind, payload) in written {
        engine.append(kind, payload, None).unwrap();
    }
    let (bundle, run) = (engine.bundle.clone(), engine.run_id.clone());
    let journal = engine.store.path().to_path_buf();
    drop(engine);
    let (store, work) = (
        Store::open(&journal).unwrap(),
        Some(dir.path().join("work")),
    );
    let mut engine = Engine::resume(store, bundle, &run, work).unwrap();
    engine.drive().unwrap();
    let events = engine.store.load(&run).unwrap();
    let decided = (events.iter())
        .find(|event| event.event_type == EventType::TransitionDecided)
        .unwrap();
    assert_eq!(decided.payload["result"], "__schema-invalid__");
    let raw = decided.payload["inputs"]["raw_result"].to_string();
    assert_eq!(raw, result.to_string());
}

/// A work site of the trellis seat's harness, standing without hands, its
/// adapters loaded from `dir`.
fn trellis_site(dir: &Path) -> SiteContext {
    fixture_adapters(dir, "true", &resume_declared(true));
    let adapters = Adapters::load(dir).unwrap();
    let kinds = DeclaredKinds::of(Some(&adapters)).seat(Some("trellis"), &[]);
    let standing = Some(Standing {
        boundary: NO_HANDS.0,
        hands: NO_HANDS.1,
    });
    SiteContext::new("a".repeat(64), "b".repeat(64), SeatClass::Work).of_harness(kinds, standing)
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
    for (row, result) in [
        (checkpoint(stray, THREAD, "t-1", "tr-1"), pass(TRAIL)),
        (checkpoint(TRAIL, THREAD, "t-1", "tr-1"), pass(stray)),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let judge = Visit {
            checkpoints: vec![row],
            result: Some(result),
            unresolved: false,
        };
        let second = Visit {
            result: Some(pass(TRAIL)),
            ..Visit::default()
        };
        let events = paneled(dir.path(), judge, second);
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
             (decision 0109 ruling 1); driver stderr: empty",
        ),
        (
            checkpoint(TRAIL, THREAD, "t-1", "tr-1"),
            findings,
            "the result was refused: the result carries 'findings' from a work site: only a \
             gate reports findings (decision 0109 ruling 1); driver stderr: empty",
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (_store, _run, events) = seated(dir.path(), &once(row, result));
        failed_with(&events, error);
    }
}

/// What the fake driver writes to its stderr where its visit is
/// [`Visit::unresolved`].
const UNRESOLVED: &str = "kinds-tests: received unresolved";

/// What an attempt whose process group outlived its kill carries as its
/// cleanup.
const HELD: &str = "its process group 7 still had members after the kill";

/// The engine's test-only seam at the one point it receives a driver's
/// report (`engine::received`; the controller's ruling R17): a report
/// whose fake driver wrote [`UNRESOLVED`] to its stderr is received with
/// its process group outliving its kill, as the process layer reports one
/// (`process::tests`), and every other report as it came.
pub(super) fn unresolved_where_marked(mut report: AttemptReport) -> AttemptReport {
    if report.stderr.contains(UNRESOLVED) {
        report.cleanup = Cleanup::Unresolved {
            reason: Unsettled::Group { group: 7 },
        };
    }
    report
}

/// A successful panel member's result, carrying a transcript of `kind`.
fn pass(kind: &str) -> Value {
    json!({"result": "pass", "model": "trellis-1",
           "transcript": {"kind": kind, "locator": "t-1", "home": "/h/.trellis"}})
}

/// A visit sending `checkpoints` and succeeding with `result`, its report
/// received with its cleanup unresolved.
fn unresolved(checkpoints: Vec<Value>, result: Value) -> Visit {
    Visit {
        checkpoints,
        result: Some(result),
        unresolved: true,
    }
}

/// The `work` panel's run, its `judge` member playing `judge` and its
/// `second` member `second`: its journal.
fn paneled(dir: &Path, judge: Visit, second: Visit) -> Vec<EventEnvelope> {
    let resume = resume_declared(false);
    seated_as(dir, &[judge, second], &resume, &panel_seat()).2
}

/// The run's one `effect/indeterminate` payload, and the whole journal as
/// text, which holds no kind place naming any of `strays`.
fn indeterminate<'e>(events: &'e [EventEnvelope], strays: &[Value]) -> &'e Value {
    let parked = of(events, EventType::EffectIndeterminate);
    assert_eq!(parked.len(), 1, "{events:?}");
    let journaled = serde_json::to_string(events).unwrap();
    for stray in strays {
        assert!(
            !journaled.contains(&format!(r#""kind":{stray}"#)),
            "{journaled}"
        );
        assert!(
            !journaled.contains(&format!(r#""root_session":{stray}"#)),
            "{journaled}"
        );
    }
    parked[0]
}

/// The received outcome of a record seat `seat`'s harness refuses for
/// its `member` kind `kind`: the kind refusal, typed.
fn refusal_received(seat: &str, member: &str, kind: &str) -> Value {
    let error = format!(
        "seat '{seat}' recorded {member} kind {kind}, which the seat's harness does not declare \
         (decision 0075 ruling 2)"
    );
    json!({"status": "failed", "error": error})
}

/// The reason an unresolved attempt that reached `reached` parks on, its
/// driver's stderr, [`UNRESOLVED`], named by evidence (#1199).
fn parked_on(reached: &str) -> Value {
    let stderr = brokkr_protocol::process::DriverStderr::of(UNRESOLVED);
    json!(stderr.named(&format!(
        "{reached}; the attempt is not proven over: {HELD}"
    )))
}

/// The results an unresolved cleanup's evidence must never carry
/// (the controller's ruling R13): a transcript kind of no declaration, one
/// outside the grammar, one that is a number, and a root session that is
/// not an object; each with the kind refusal received in its place, whose
/// words never echo a value outside the grammar; and the strays that
/// must not reach the journal at a kind place.
fn strays() -> [(Value, &'static str, &'static str, Value); 4] {
    let mut number = complete(TRAIL);
    number["transcript"]["kind"] = json!(7);
    let mut rootless = complete(TRAIL);
    rootless["root_session"] = json!("trellis--stray");
    let outside = "outside seat-record v8's grammar";
    [
        (
            complete("trellis-stray"),
            "transcript",
            "'trellis-stray'",
            json!("trellis-stray"),
        ),
        (
            complete("trellis--stray"),
            "transcript",
            outside,
            json!("trellis--stray"),
        ),
        (number, "transcript", outside, json!(7)),
        (rootless, "root session", outside, json!("trellis--stray")),
    ]
}

/// No event carries anything at a kind place but a kind the seat's harness
/// declares, an unresolved cleanup's evidence included, on the engine's
/// own road for a single seat (`run_driver`; the controller's rulings R13
/// and R17): each stray result is received as the kind refusal in its
/// place, never echoing a value outside the grammar, whether nothing else
/// refused the report, a live checkpoint's refusal did, or decision
/// 0109's did, each refusal keeping its own words in the reason. A result
/// naming declared kinds is received as it came, refused or not.
#[test]
fn a_single_seats_unresolved_cleanup_receives_the_kind_refusal_in_place_of_a_stray_kind() {
    let live = "a driver's checkpoint claims a record only the engine authors (decision 0109 \
                ruling 1)";
    let by_0109 = "the result was refused: the result carries 'findings' from a work site: only \
                   a gate reports findings (decision 0109 ruling 1)";
    let mut authored = checkpoint(TRAIL, THREAD, "t-1", "tr-1");
    authored["authored"] = json!("engine");
    let reporting = |mut result: Value| {
        result["findings"] = json!([]);
        result
    };
    let parked = |rows: Vec<Value>, result: Value, strays: &[Value]| {
        let dir = tempfile::tempdir().unwrap();
        let events = seated(dir.path(), &[unresolved(rows, result)]).2;
        let payload = indeterminate(&events, strays);
        (payload["reason"].clone(), payload["received"].clone())
    };
    for (result, member, kind, stray) in strays() {
        let received = refusal_received("work", member, kind);
        assert!(!received.to_string().contains("--"), "{received}");
        let by_kind = received["error"].as_str().unwrap().to_string();
        for (rows, result, reason) in [
            (vec![], result.clone(), by_kind.as_str()),
            (vec![authored.clone()], result.clone(), live),
            (vec![], reporting(result), by_0109),
        ] {
            let evidence = parked(rows, result, std::slice::from_ref(&stray));
            assert_eq!(evidence, (parked_on(reason), received.clone()), "{reason}");
        }
    }
    let came = |result: Value| json!({"status": "succeeded", "result": result});
    for (result, reason) in [
        (complete(TRAIL), "the driver reported success"),
        (reporting(complete(TRAIL)), by_0109),
    ] {
        let evidence = parked(vec![], result.clone(), &[]);
        assert_eq!(evidence, (parked_on(reason), came(result)));
    }
}

/// A panel member's unresolved cleanup is judged as a single seat's is,
/// on the engine's own road for a member (`settle_members`; the
/// controller's rulings R13 and R17): the member whose result holds a
/// stray kind is received as the kind refusal, and its sibling, naming
/// its declared kind, as it came.
#[test]
fn a_panel_members_unresolved_cleanup_receives_the_kind_refusal_in_its_results_place() {
    let cleanup = json!({"state": "unresolved", "reason": HELD});
    for (mut result, member, kind, stray) in strays() {
        result["result"] = json!("pass");
        let dir = tempfile::tempdir().unwrap();
        let judge = unresolved(vec![], result);
        let events = paneled(dir.path(), judge, unresolved(vec![], pass(TRAIL)));
        let payload = indeterminate(&events, &[stray]);
        let received = refusal_received("work:judge", member, kind);
        assert_eq!(
            payload["unresolved_members"],
            json!({
                "judge": {"received": received, "cleanup": cleanup},
                "second": {"received": {"status": "succeeded", "result": pass(TRAIL)},
                           "cleanup": cleanup},
            }),
            "{payload}"
        );
    }
}

/// A `work` review panel of two members, `judge` and `second`, each
/// hiring the trellis agent: the aggregate that binds a member's word and
/// copies its payload into the notes of a join it refuses.
fn review_seat() -> Value {
    let member = json!({"agent": "trellis", "class": "work"});
    json!({"results": ["clean", "residual", "security-hold"], "aggregate": "review-panel",
           "panel": {"judge": member, "second": member}})
}

/// A trellis member's result of `word`, carrying a transcript of `kind`.
fn worded(word: &str, kind: &str) -> Value {
    let mut result = pass(kind);
    result["result"] = json!(word);
    result
}

/// A visit succeeding with `result`.
fn succeeding(result: Value) -> Visit {
    Visit {
        result: Some(result),
        ..Visit::default()
    }
}

/// The binding refusal of a trellis member's word `maybe`, as the review
/// panel's notes name it.
const UNBOUND: &str = "member result 'maybe' is not among its declared results [clean, residual]";

/// That no event of `events` holds `kind` at a kind place, and none holds
/// a binding refusal.
fn holds_neither(events: &[EventEnvelope], kind: &str) {
    let journaled = serde_json::to_string(events).unwrap();
    assert!(
        !journaled.contains(&format!(r#""kind":"{kind}""#)),
        "{journaled}"
    );
    assert!(!journaled.contains("is not among"), "{journaled}");
}

/// A record refused for its kind is a failed attempt and is never bound
/// as a word, on the aggregate that binds and copies (the controller's
/// rulings R13, R16 and R19): a review panel member naming its declared
/// kind and a word the join refuses reaches the binding refusal, its
/// payload under the aggregate's `notes.members`; the same member naming
/// a kind undeclared or outside the grammar ends as a failed member on the
/// kind refusal, no binding refusal is journaled for it, and no event
/// holds its kind, under `notes.members` or anywhere else.
#[test]
fn a_member_refused_for_its_kind_fails_and_is_never_bound_as_a_word() {
    let dir = tempfile::tempdir().unwrap();
    let resume = resume_declared(false);
    let (judge, second) = (worded("maybe", TRAIL), worded("clean", TRAIL));
    let visits = [succeeding(judge.clone()), succeeding(second)];
    let events = seated_as(dir.path(), &visits, &resume, &review_seat()).2;
    let (ended, payload) = first_ending(&events);
    assert_eq!(ended, EventType::EffectSucceeded, "{payload}");
    let notes = &payload["result"]["notes"];
    assert_eq!(notes["refused"], json!({"judge": UNBOUND}), "{payload}");
    assert_eq!(notes["members"]["judge"], judge, "{payload}");
    for (stray, shown) in [
        ("trellis--stray", "outside seat-record v8's grammar"),
        ("trellis-stray", "'trellis-stray'"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let visits = [succeeding(worded("maybe", stray)), succeeding(pass(TRAIL))];
        let events = seated_as(dir.path(), &visits, &resume, &review_seat()).2;
        let refusal = refusal_received("work:judge", "transcript", shown);
        let error = format!(
            "panel members failed — judge: {}",
            refusal["error"].as_str().unwrap()
        );
        holds_neither(&events, stray);
        failed_with(&events, &error);
        let journaled = serde_json::to_string(&events).unwrap();
        assert!(!journaled.contains("maybe"), "{journaled}");
    }
}

/// The trellis review panel's run, its first effect parked on attempt
/// `source`, which a released engine ended indeterminate with `judge`'s
/// `settled` result in `settled_members`, unmarked; its driver plays
/// `second` for the `second` member. The engine, and how many events the
/// journal held once it parked.
fn carry_parked(dir: &Path, settled: Value, second: Visit) -> (Engine, usize) {
    let repo = dir.join("repo");
    std::fs::create_dir(&repo).unwrap();
    git_commit(&repo, "init");
    let store = Store::open(&dir.join("forge.db")).unwrap();
    let script = driver(&dir.join("driver"), &[Visit::default(), second]);
    let bundle = trellis_bundle(dir, &script, &resume_declared(false), &review_seat());
    let mut engine = Engine::start(store, bundle, "trellis", Some(repo)).unwrap();
    parked_at_source(&mut engine, |terminal| {
        terminal["settled_members"] = json!({"judge": settled});
    });
    let parked = engine.store.load(&engine.run_id).unwrap().len();
    (engine, parked)
}

/// A carried member contributes to a new verdict, so it is judged for its
/// kinds before it is carried (item 2 of decision 0075's addendum; the
/// controller's rulings R18 and R23): a released engine's
/// unmarked settled result for `judge`, naming a transcript kind outside
/// the grammar or undeclared beside a word the join would refuse, is no
/// verdict to carry, so a retry of `second` alone is refused as it is for
/// a member with no settled result, and no new event holds the kind,
/// under `notes.members` or anywhere else. The same result naming the
/// declared kind carries as before: its payload under `notes.members`
/// beside the binding refusal.
#[test]
fn a_carried_members_result_is_judged_for_its_kinds_before_it_is_carried() {
    for stray in ["trellis--stray", "trellis-stray"] {
        let dir = tempfile::tempdir().unwrap();
        let second = succeeding(worded("clean", TRAIL));
        let (mut engine, parked) = carry_parked(dir.path(), worded("maybe", stray), second);
        let outcome = retry(&mut engine, &["second"]);
        engine.drive().unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();
        holds_neither(&events[parked..], stray);
        assert!(
            matches!(&outcome, MemberRetryOutcome::CarryRefused(CarryRefusal(
                Refusal::Unsettled { members })) if members == "judge"),
            "{outcome:?}"
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let carried = worded("maybe", TRAIL);
    let second = succeeding(worded("clean", TRAIL));
    let (mut engine, parked) = carry_parked(dir.path(), carried.clone(), second);
    let outcome = retry(&mut engine, &["second"]);
    assert!(matches!(
        outcome,
        MemberRetryOutcome::Disposed(FencedCommandOutcome::Accepted { .. })
    ));
    engine.drive().unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let (ended, payload) = first_ending(&events[parked..]);
    assert_eq!(ended, EventType::EffectSucceeded, "{payload}");
    let notes = &payload["result"]["notes"];
    assert_eq!(notes["refused"], json!({"judge": UNBOUND}), "{payload}");
    assert_eq!(notes["members"]["judge"], carried, "{payload}");
}

/// The trellis `headless` shape, `supported` where the site stands, with
/// each of `patch`'s members written over it.
fn shape_with(patch: &Value) -> Rejoin {
    let mut rejoin = resume_shaped(true, NO_HANDS);
    for (key, value) in patch.as_object().unwrap() {
        rejoin.resume["headless"][key] = value.clone();
    }
    rejoin
}

/// The engine offers a root of a new kind no wider than the driver's own
/// resume gate would rejoin (the controller's rulings R11 and R15): for
/// each row, whether the retry was offered the root, beside whether the
/// protocol's gate, asked about the shape the root session kind names
/// (`resume_shape`) on the very input the engine wrote for the retry,
/// enables it. A matching `supported` shape is offered and enabled;
/// another boundary, other hands or an unmeasured status buys neither.
#[test]
fn the_engine_offers_a_new_root_kind_only_where_the_drivers_resume_gate_enables_it() {
    let unmeasured = Rejoin {
        resume: json!({"headless": {"status": "unmeasured",
            "identity": {"unknown": "fixture"}, "reason": "fixture: not measured",
            "classes": ["work"], "boundaries": [NO_HANDS.0], "hands": NO_HANDS.1}}),
        by: "headless",
    };
    let rows = [
        ("matching", resume_shaped(true, NO_HANDS), (true, true)),
        (
            "boundary",
            resume_shaped(true, ("namespace", NO_HANDS.1)),
            (false, false),
        ),
        (
            "hands",
            resume_shaped(true, (NO_HANDS.0, "boxed")),
            (false, false),
        ),
        // The class is the engine's check alone (the controller's ruling
        // R15): the gate reads no class, and only the engine knows a
        // site's, so here the engine declines what the gate would enable.
        (
            "class",
            shape_with(&json!({"classes": ["gate"]})),
            (false, true),
        ),
        ("status", unmeasured, (false, false)),
    ];
    for (row, rejoin, expected) in rows {
        assert_eq!(offered_and_enabled(&rejoin), expected, "{row}");
    }
}

/// Whether the retry under `rejoin` was offered the root, beside whether
/// the driver's resume gate, asked about the shape `rejoin` names on the
/// very input the engine wrote for the retry, enables it.
fn offered_and_enabled(rejoin: &Rejoin) -> (bool, bool) {
    let retry = retried(rejoin);
    let enabled = brokkr_protocol::adapters::resume_enabled(&retry.input, rejoin.by);
    let offered = retry.target.is_some();
    assert_eq!(retry.resumes.len(), usize::from(offered), "{retry:?}");
    (offered, enabled)
}

/// A root session kind is offered only by the one shape it names as the
/// shape its seats rejoin by (the controller's ruling R14): with
/// `headless` unsupported and `interactive` supported and covering the
/// site, an adapter naming `headless` is offered nothing, as the gate
/// asked about `headless` refuses, and one naming `interactive` is
/// offered, as the gate asked about `interactive` enables.
#[test]
fn a_new_root_kind_is_offered_only_by_the_shape_its_root_session_kind_names() {
    let resume = json!({"headless": shape_of(false, NO_HANDS),
                        "interactive": shape_of(true, NO_HANDS)});
    for (by, expected) in [("headless", (false, false)), ("interactive", (true, true))] {
        let resume = resume.clone();
        assert_eq!(
            offered_and_enabled(&Rejoin { resume, by }),
            expected,
            "{by}"
        );
    }
}
