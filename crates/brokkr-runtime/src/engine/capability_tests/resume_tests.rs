//! Decision 0065 slice two, U5f (SC3): a run cannot acquire execution
//! authority on resume. A run pinned with a v11 holding, and a v12 run
//! pinned under another retention declaration, both meet the existing
//! manifest-mismatch door by name; the old pin reads back unchanged.
//!
//! U7c (CR3): nor can it mint a second budget. An attempt's inventory and
//! shares are provisioned once; a retry, a reconnect or a resumed engine
//! that provisions the same attempt again is refused before launch.

use super::super::broker::Unprovisioned;
use super::provision_tests::{admitted, observing, sealed, Fixture};
use super::*;

/// Every file under the attempt directory `control`, by name, with its
/// bytes.
fn files(control: &Path) -> BTreeMap<String, Vec<u8>> {
    let entries = std::fs::read_dir(control).unwrap().map(Result::unwrap);
    let read = |entry: std::fs::DirEntry| {
        let name = entry.file_name().into_string().unwrap();
        (name, std::fs::read(entry.path()).unwrap())
    };
    entries.map(read).collect()
}

/// An attempt is provisioned once: provisioning it again — as a resumed
/// engine would — is refused and moves no byte of its inventory or plans;
/// an attempt whose directory stands without an inventory, as one cut
/// short leaves it, is refused too and gains none; a new attempt has its
/// own directory, inventory and budget.
#[test]
fn an_attempts_budget_is_minted_once_and_never_again_on_resume() {
    let test = "an_attempts_budget_is_minted_once_and_never_again_on_resume";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    let server = fixture.server();
    fixture.hold("library-docs", admitted("library-docs", &[&server], true));
    let first = fixture.provision("attempt-1", &observing);
    let first = first.unwrap().unwrap();
    let written = files(&first);
    assert_eq!(written.len(), 2);
    let again = fixture.provision("attempt-1", &observing);
    assert_eq!(again, Err(Unprovisioned::Provisioned));
    assert_eq!(files(&first), written);
    let cut = fixture.run_dir().join("attempt-2");
    std::fs::create_dir(&cut).unwrap();
    let missing = fixture.provision("attempt-2", &observing);
    assert_eq!(missing, Err(Unprovisioned::Provisioned));
    assert_eq!(files(&cut), BTreeMap::new());
    let next = fixture.provision("attempt-3", &observing);
    let next = next.unwrap().unwrap();
    assert_eq!(next, fixture.run_dir().join("attempt-3"));
    let (inventory, plans) = sealed(&next);
    assert_eq!(inventory.plans[0].locator, next.join("plan-0.json"));
    assert_eq!(plans[0]["owner"]["attempt"], "attempt-3");
}

/// M4: an attempt whose start is durable but whose inventory never landed
/// — the engine ended between `EffectStarted` and the inventory, leaving
/// its plan and no pin — is settled by the existing recovery as
/// indeterminate, never run or replayed: its directory is never
/// provisioned again and gains nothing, so no budget is minted twice and
/// the broker finds nothing bound there. A new attempt has its own.
#[test]
fn a_start_whose_inventory_never_landed_settles_indeterminate_and_mints_nothing_again() {
    let test = "a_start_whose_inventory_never_landed_settles_indeterminate_and_mints_nothing_again";
    let Some(mut fixture) = Fixture::protected(test) else {
        return;
    };
    // The engine ends inside provisioning, after the durable start: here
    // its refusal's own append fails, so nothing settles the attempt.
    let mut unsealed = admitted("status", &["true"], true);
    unsealed.admitted = None;
    fixture.hold("status", unsealed);
    let db = fixture.root.join("forge.db");
    super::super::tests::fail_event(&db, "effect/failed");
    let crashed = fixture.engine.drive();
    assert!(matches!(crashed, Err(EngineError::Store(_))), "{crashed:?}");
    let run = fixture.engine.run_id.clone();
    let events = fixture.engine.store.load(&run).unwrap();
    let started = events.last().unwrap();
    assert_eq!(started.event_type, EventType::EffectStarted);
    let attempt = started.payload["attempt_id"].as_str().unwrap().to_string();
    let effect = started.payload["effect_id"].clone();
    let dropped = rusqlite::Connection::open(&db).unwrap();
    (dropped.execute_batch("DROP TRIGGER forced_event_failure")).unwrap();
    // Its plan was written and its inventory never was.
    fixture.hold("status", admitted("status", &["true"], true));
    let control = fixture.provision(&attempt, &observing).unwrap().unwrap();
    std::fs::remove_file(control.join("inventory.json")).unwrap();
    let cut = files(&control);
    let store = Store::open(&db).unwrap();
    let (bundle, work) = (fixture.engine.bundle.clone(), fixture.root.join("work"));
    let mut resumed = Engine::resume(store, bundle, &run, Some(work)).unwrap();
    let end = resumed.drive().unwrap();
    let events = resumed.store.load(&run).unwrap();
    let settled = events.iter().rev().find(|event| event.attempt_id.is_some());
    let settled = settled.unwrap();
    assert_eq!(settled.event_type, EventType::EffectIndeterminate);
    let reason = "engine restarted while the attempt was in flight; completion cannot be \
                  established";
    let payload = json!({"effect_id": effect, "attempt_id": attempt, "reason": reason});
    assert_eq!(settled.payload, payload);
    let parked = end.state.run.park_reason().map(ParkReason::as_str);
    assert!(
        parked.is_some_and(|parked| parked.contains(reason)),
        "{parked:?}"
    );
    let again = fixture.provision(&attempt, &observing);
    assert_eq!(again, Err(Unprovisioned::Provisioned));
    assert_eq!(files(&control), cut);
    assert_eq!(cut.keys().collect::<Vec<_>>(), ["plan-0.json"]);
    let next = fixture.provision("attempt-2", &observing).unwrap().unwrap();
    assert_eq!(sealed(&next).1[0]["owner"]["attempt"], "attempt-2");
}

/// A run's bundle whose one site holds `web-search` as `holding` says.
fn holding(dir: &Path, holding: Value) -> Bundle {
    let mut bundle = bundle(dir, single_body(vec!["driver".into()]));
    bundle.manifest["capabilities"] = json!({"realm": "<unmapped>", "grants": {},
        "sites": {"work": {"candidates": [{"held": {"web-search": holding}}]}}});
    bundle
}

#[test]
fn a_v11_holding_or_a_moved_v12_declaration_is_refused_on_resume_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let store = || Store::open(&dir.path().join("forge.db")).unwrap();
    let v11 = json!({"dialect": "codex-native-search", "tools": ["web_search"]});
    let v12 = |declared: bool| {
        let mut record = v11.clone();
        record["implementation"] =
            json!({"kind": "provider-native", "provider": "codex", "adapter_key": "web-search"});
        record["retention"] = json!({"declared": declared, "realm": "veto", "effective": false});
        holding(dir.path(), record)
    };
    let moved = "capabilities differ: the run's pinned sites no longer match what the bundle \
                 compiles to here — a grant, an abstract definition, a tool dialect or an \
                 adapter's native declaration was added, removed or edited since the run started";
    for (pinned, resumed) in [
        (holding(dir.path(), v11.clone()), v12(false)),
        (v12(true), v12(false)),
    ] {
        let run = Engine::start(store(), pinned.clone(), "f", Some(repo.clone()))
            .unwrap()
            .run_id;
        let written = store().manifest(&run).unwrap();
        match Engine::resume(store(), resumed, &run, Some(repo.clone())) {
            Err(EngineError::ManifestMismatch { detail, .. }) => assert_eq!(detail, moved),
            Err(other) => panic!("expected the manifest mismatch, got {other}"),
            Ok(_) => panic!("a changed capability identity resumed"),
        }
        // The pinned run reads back exactly as written, and still resumes
        // under the bundle it was started with.
        assert_eq!(store().manifest(&run).unwrap(), written);
        assert_eq!(written["capabilities"], pinned.manifest["capabilities"]);
        let unmoved = Engine::resume(store(), pinned, &run, Some(repo.clone()));
        assert_eq!(
            unmoved
                .map(|engine| engine.run_id)
                .map_err(|e| e.to_string()),
            Ok(run)
        );
    }
}
