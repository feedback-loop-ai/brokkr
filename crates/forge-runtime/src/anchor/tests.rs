use super::*;
use serde_json::json;

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
