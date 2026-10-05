//! Decision 0065 slice two, U5f (SC3): a run cannot acquire execution
//! authority on resume. A run pinned with a v11 holding, and a v12 run
//! pinned under another retention declaration, both meet the existing
//! manifest-mismatch door by name; the old pin reads back unchanged.

use super::*;

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
