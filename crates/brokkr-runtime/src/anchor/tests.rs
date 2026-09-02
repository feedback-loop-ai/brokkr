use super::*;
use serde_json::json;

/// git's well-known empty tree, used to forge malformed legacy anchors.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        &["init", "-q"][..],
        &["config", "user.email", "test@example.invalid"],
        &["config", "user.name", "test"],
        &["config", "commit.gpgsign", "false"],
    ] {
        git(dir.path(), args, None).unwrap();
    }
    dir
}

fn empty_store(dir: &Path, run_id: &str) -> Store {
    let mut store = Store::open(&dir.join("forge.db")).unwrap();
    store
        .create_run(run_id, "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    store
}

#[test]
fn anchor_embeds_the_export_and_vouches_the_repository_head() {
    let dir = repo();
    let mut store = empty_store(dir.path(), "run");

    let unborn = anchor(&store, dir.path(), "run").unwrap();
    let message: serde_json::Value = serde_json::from_str(
        &git(dir.path(), &["log", "-1", "--format=%B", &unborn], None).unwrap(),
    )
    .unwrap();
    assert_eq!(message["anchor"], "forge.journal-anchor/v2");
    assert_eq!(message["repo_head"], serde_json::Value::Null);
    assert_eq!(
        git(dir.path(), &["show", &format!("{unborn}:run.ndjson")], None).unwrap(),
        store.export_ndjson("run").unwrap()
    );

    std::fs::write(dir.path().join("change"), "vouched").unwrap();
    git(dir.path(), &["add", "change"], None).unwrap();
    git(dir.path(), &["commit", "-q", "-m", "vouched head"], None).unwrap();
    let reviewed = git(dir.path(), &["rev-parse", "HEAD"], None).unwrap();
    store
        .append_next(
            "run",
            brokkr_core::envelope::EventType::TransitionDecided,
            json!({"inputs": {"reviewed_heads": {"repo": reviewed}}}),
            None,
            None,
        )
        .unwrap();
    std::fs::write(dir.path().join("change"), "not reviewed").unwrap();
    git(dir.path(), &["add", "change"], None).unwrap();
    git(dir.path(), &["commit", "-q", "-m", "later head"], None).unwrap();
    anchor(&store, dir.path(), "run").unwrap();
    let report = verify(&store, dir.path(), "run").unwrap();
    assert_eq!(report["repo_head"], reviewed);
    assert_eq!(report["chain_length"], 2);
}

#[test]
fn a_vouched_head_is_one_full_recorded_object_id_or_nothing() {
    let event = |reviewed_heads: serde_json::Value| EventEnvelope {
        run_id: "run".into(),
        seq: 1,
        event_id: "event".into(),
        event_schema_version: 1,
        event_type: EventType::TransitionDecided,
        payload: json!({"inputs": {"reviewed_heads": reviewed_heads}}),
        causation_id: None,
        correlation_id: "run".into(),
        attempt_id: None,
        recorded_at: "2026-09-02T00:00:00Z".into(),
        previous_hash: "0".repeat(64),
        event_hash: "a".repeat(64),
    };
    let head = "A".repeat(40);
    assert_eq!(
        vouched_head(&[event(json!({"repo": head}))]),
        Some("a".repeat(40))
    );
    for invalid in [
        json!(null),
        json!({}),
        json!({"repo": null}),
        json!({"repo": "short"}),
        json!({"repo": "z".repeat(40)}),
        json!({"repo": "a".repeat(40), "other": "b".repeat(40)}),
    ] {
        assert_eq!(vouched_head(&[event(invalid)]), None);
    }
    let mut unrelated = event(json!({"repo": "b".repeat(40)}));
    unrelated.event_type = EventType::RunStarted;
    assert_eq!(vouched_head(&[unrelated]), None);
    let mut absent = event(json!({"repo": "b".repeat(40)}));
    absent.payload = json!({});
    assert_eq!(vouched_head(&[absent]), None);
}

#[test]
fn git_spawn_and_run_errors_name_the_boundary() {
    let missing = Path::new("/forge/definitely/missing/repository");
    assert!(matches!(
        git(missing, &["hash-object", "-t", "tree", "--stdin"], Some("")),
        Err(AnchorError::Git { verb: "spawn", .. })
    ));
    assert!(matches!(
        git(missing, &["status"], None),
        Err(AnchorError::Git { verb: "run", .. })
    ));
    assert!(matches!(
        git_io::<()>(Err(std::io::Error::other("wait failed")), "wait"),
        Err(AnchorError::Git { verb: "wait", .. })
    ));
}

#[test]
fn malformed_and_hash_only_mismatched_anchors_refuse() {
    let dir = repo();
    let store = empty_store(dir.path(), "run");
    let reference = ref_name("run");

    let malformed = git(dir.path(), &["commit-tree", EMPTY_TREE], Some("not-json")).unwrap();
    git(dir.path(), &["update-ref", &reference, &malformed], None).unwrap();
    assert!(matches!(
        verify(&store, dir.path(), "run"),
        Err(AnchorError::Mismatch(message)) if message.contains("unreadable anchor message")
    ));

    let wrong_hash = json!({
        "anchor":"forge.journal-anchor/v1",
        "run_id":"run",
        "seq":0,
        "journal_head_hash":"wrong"
    })
    .to_string();
    let commit = git(dir.path(), &["commit-tree", EMPTY_TREE], Some(&wrong_hash)).unwrap();
    git(dir.path(), &["update-ref", &reference, &commit], None).unwrap();
    assert!(matches!(
        verify(&store, dir.path(), "run"),
        Err(AnchorError::Mismatch(_))
    ));
}
