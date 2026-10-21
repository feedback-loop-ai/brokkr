//! Decision 0094's constructor, resume window and first-return controls.
use super::*;
use crate::bundle::entry::EntryRefusal;
use crate::bundle::GateAssessment;
use brokkr_core::entry::{ContinuedRun, EntryRead, RunEntry};

fn gate_bundle(root: &Path) -> Bundle {
    let mut compiled = bundle(root, single_body(vec!["missing-driver".into()]));
    for phase in ["review", "ship"] {
        compiled.seats.get_mut(phase).unwrap().gate = GateAssessment::AllGate;
    }
    compiled
}

fn entry(phase: &str) -> RunEntry {
    RunEntry {
        phase: phase.into(),
        continues: vec![ContinuedRun {
            run_id: "earlier-12345678".into(),
            feature: "earlier work\nsynthetic".into(),
        }],
    }
}

#[test]
fn a_started_entry_resumes_at_its_gate_under_the_same_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let compiled = gate_bundle(dir.path());
    let manifest = compiled.manifest.clone();
    let engine = Engine::start_in_world(
        Store::open(&db).unwrap(),
        compiled,
        &"long feature ".repeat(6),
        Some(dir.path().into()),
        None,
        Some(entry("review")),
    )
    .unwrap();
    assert!(brokkr_core::entry::is_native_run_id(&engine.run_id));
    assert_eq!(engine.store.manifest(&engine.run_id).unwrap(), manifest);
    let id = engine.run_id.clone();
    let events = engine.store.load(&id).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].payload["phases"],
        json!(engine.bundle.machine.phases)
    );
    assert_eq!(events[0].payload["entry"], json!(entry("review")));
    drop(engine);
    let mut resumed = Engine::resume(
        Store::open(&db).unwrap(),
        gate_bundle(dir.path()),
        &id,
        Some(dir.path().into()),
    )
    .unwrap();
    assert_eq!(resumed.entry(), Some(&entry("review")));
    resumed.drive_once().unwrap();
    let events = resumed.store.load(&id).unwrap();
    assert_eq!(events[1].event_type, EventType::PhaseEntered);
    assert_eq!(events[1].payload["phase"], "review");
}

#[test]
fn an_invalid_record_never_reaches_the_store_and_a_refused_phase_never_enters() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut twice = entry("review");
    twice.continues.push(twice.continues[0].clone());
    let error = Engine::start_in_world(
        Store::open(&db).unwrap(),
        gate_bundle(dir.path()),
        "synthetic",
        Some(dir.path().into()),
        None,
        Some(twice),
    )
    .err()
    .unwrap();
    assert!(
        matches!(error,EngineError::EntryRecord(EntryRead::Twice { run_id })
        if run_id == "earlier-12345678")
    );
    assert_eq!(Store::open(&db).unwrap().list_runs().unwrap(), vec![]);
    let mut engine = Engine::start_in_world(
        Store::open(&db).unwrap(),
        gate_bundle(dir.path()),
        "synthetic",
        Some(dir.path().into()),
        None,
        Some(entry("work")),
    )
    .unwrap();
    let error = engine.drive_once().unwrap_err();
    assert!(
        matches!(error,EngineError::Entry(error) if *error.refusal ==
        EntryRefusal::Work { site:crate::bundle::WorkSite::seat() })
    );
    assert_eq!(engine.store.load(&engine.run_id).unwrap().len(), 1);
}

#[test]
fn an_ordinary_start_has_no_entry_and_enters_initial() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = Engine::start(
        Store::open(&dir.path().join("forge.db")).unwrap(),
        gate_bundle(dir.path()),
        "ordinary",
        Some(dir.path().into()),
    )
    .unwrap();
    assert_eq!(engine.entry(), None);
    let events = engine.store.load(&engine.run_id).unwrap();
    assert_eq!(events[0].payload.get("entry"), None);
    engine.drive_once().unwrap();
    assert_eq!(
        engine.store.load(&engine.run_id).unwrap()[1].payload["phase"],
        "work"
    );
}

#[test]
fn an_entered_first_return_carries_the_finding_and_the_chain() {
    let dir = tempfile::tempdir().unwrap();
    let compiled = gate_bundle(dir.path());
    let mut engine = Engine::start_in_world(
        Store::open(&dir.path().join("forge.db")).unwrap(),
        compiled,
        "entered",
        Some(dir.path().into()),
        None,
        Some(entry("review")),
    )
    .unwrap();
    let mut current = state(Some("work"), Cursor::RequestEffect);
    current.visits.insert("work".into(), 1);
    current.last_decision = Some(
        serde_json::from_value(json!({
            "rule_id":"REVIEW-REFORGE","from":"review","result":"residual","next":"work",
            "severity":"normal","inputs":{}
        }))
        .unwrap(),
    );
    let context = engine.seat_input(&current, "work", "effect").unwrap()["context"].clone();
    assert_eq!(context["entry"], json!(entry("review")));
    assert_eq!(context["returned_from"]["phase"], "review");
    assert_eq!(context.get("results"), None);
    engine
        .bundle
        .seats
        .insert("triage".into(), engine.bundle.seats["work"].clone());
    let triage = engine.seat_input(&current, "triage", "effect").unwrap()["context"].clone();
    assert_eq!(triage.get("entry"), None);
    assert_eq!(triage.get("last_decision"), None);
    assert_eq!(triage.get("returned_from"), None);
}

#[test]
fn resume_refuses_a_record_outside_the_language_or_an_entry_that_no_longer_admits() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("forge.db");
    let mut store = Store::open(&db).unwrap();
    let compiled = gate_bundle(dir.path());
    let id = "unreadable-12345678";
    store
        .create_run(id, "synthetic", &compiled.name, &compiled.manifest)
        .unwrap();
    let raw = crate::entry_records::invalid_records()
        .into_iter()
        .find(|(_, raw)| raw.get("unknown").is_some())
        .unwrap()
        .1;
    store
        .append_next(
            id,
            EventType::RunStarted,
            json!({"feature":"synthetic","entry":raw,"charters":[]}),
            None,
            None,
        )
        .unwrap();
    drop(store);
    let error = Engine::resume(
        Store::open(&db).unwrap(),
        compiled,
        id,
        Some(dir.path().into()),
    )
    .err()
    .unwrap();
    assert!(
        matches!(error, EngineError::EntryRecord(EntryRead::Shape { .. })),
        "{error:?}"
    );
    let engine = Engine::start_in_world(
        Store::open(&db).unwrap(),
        gate_bundle(dir.path()),
        "window",
        Some(dir.path().into()),
        None,
        Some(entry("work")),
    )
    .unwrap();
    let id = engine.run_id.clone();
    drop(engine);
    assert!(
        matches!(Engine::resume(Store::open(&db).unwrap(),gate_bundle(dir.path()),&id,Some(dir.path().into())),
        Err(EngineError::Entry(error)) if *error.refusal == EntryRefusal::Work { site:crate::bundle::WorkSite::seat() })
    );
}

#[test]
fn the_nearest_change_is_the_latest_drafting_phases() {
    let mut context = json!({"results": {}});
    for phase in ["intake", "triage", "specify", "design", "tasks"] {
        context["results"][phase] = json!({"inputs": {"change": format!("{phase}-change")}});
    }
    for expected in ["tasks", "design", "specify", "triage", "intake"] {
        assert_eq!(nearest_change(&context), Some(format!("{expected}-change")));
        context["results"].as_object_mut().unwrap().remove(expected);
    }
    assert_eq!(nearest_change(&context), None);
}

pub(super) fn fixture_seat(result: &str, script: &str) -> Seat {
    Seat {
        gate: GateAssessment::ContainsWork(crate::bundle::WorkSite::seat()),
        results: vec![result.into()],
        limits: Limits::default(),
        inputs: vec![],
        secrets: vec![],
        body: single_body(vec!["sh".into(), "-c".into(), script.into()]),
    }
}

pub(super) fn gate_engine(root: &Path, repo: PathBuf, script: &str, feature: &str) -> Engine {
    let store = Store::open(&root.join("forge.db")).unwrap();
    let mut compiled = bundle(
        root,
        single_body(vec!["sh".into(), "-c".into(), script.into()]),
    );
    compiled.seats.get_mut("work").unwrap().gate = GateAssessment::AllGate;
    Engine::start(store, compiled, feature, Some(repo)).unwrap()
}

pub(super) fn routing_machine(result: &str, stop_id: &str, stop_reason: &str) -> Machine {
    let mut table = machine();
    table.phases.insert(0, "triage".into());
    table.initial = "triage".into();
    let mut halt = table.rules.remove(0);
    halt.id = stop_id.into();
    halt.from = "triage".into();
    halt.result = if result == "engine" { "halt" } else { "stop" }.into();
    halt.severity = "normal".into();
    halt.reason = stop_reason.into();
    let mut route = table.rules[0].clone();
    route.id = "TRIAGE".into();
    route.from = "triage".into();
    route.result = result.into();
    route.next = Some("work".into());
    route.reason = "route".into();
    table.rules.splice(0..0, [route, halt]);
    table
}

#[test]
fn an_ordinary_runs_input_digest_keeps_its_pre_entry_value() {
    let (_dir, mut engine) = engine(single_body(vec!["missing-driver".into()]));
    engine.run_id = "ordinary-12345678".into();
    engine.feature = "ordinary synthetic feature".into();
    engine.repo = "/synthetic/repo".into();
    engine.bundle.dir = "/synthetic/bundle".into();
    engine.state = Some("/synthetic/state".into());
    let current = state(Some("work"), Cursor::RequestEffect);
    let input = engine.seat_input(&current, "work", "effect-fixed").unwrap();
    assert_eq!(
        brokkr_core::canonical::sha256_hex(&input),
        "4411e79874d2425d59b198c1c8a7ddfc4a0cabdc6f29cc9aac48b9c66d1198ce"
    );
}
