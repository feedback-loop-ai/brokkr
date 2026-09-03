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
    assert_eq!(message["anchor"], "forge.journal-anchor/v3");
    assert_eq!(message["repo_head"], serde_json::Value::Null);
    assert_eq!(message["base"], serde_json::Value::Null);
    assert_eq!(message["patch"], serde_json::Value::Null);
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
    // The vouched head sits on the default branch itself, so the slice
    // measured against that branch is empty: a base, and no entries.
    assert_eq!(report["anchor"], "forge.journal-anchor/v3");
    assert_eq!(report["base"], reviewed);
    assert_eq!(report["patch"], json!({}));
}

fn commit(dir: &Path, path: &str, contents: &str, message: &str) -> String {
    let file = dir.join(path);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(file, contents).unwrap();
    git(dir, &["add", "-A"], None).unwrap();
    git(dir, &["commit", "-q", "-m", message], None).unwrap();
    git(dir, &["rev-parse", "HEAD"], None).unwrap()
}

fn vouch(store: &mut Store, run_id: &str, head: &str) {
    store
        .append_next(
            run_id,
            EventType::TransitionDecided,
            json!({"inputs": {"reviewed_heads": {"repo": head}}}),
            None,
            None,
        )
        .unwrap();
}

fn anchored_message(dir: &Path, sha: &str) -> serde_json::Value {
    serde_json::from_str(&git(dir, &["log", "-1", "--format=%B", sha], None).unwrap()).unwrap()
}

/// Decision 0038 ruling 1: the branch a slice is measured against is
/// the remote's HEAD, then `main`, then `master`, else nothing.
#[test]
fn the_default_branch_is_the_remote_head_then_main_then_master_else_none() {
    let unborn = repo();
    assert_eq!(default_branch(unborn.path()), None);

    let trunk = repo();
    git(
        trunk.path(),
        &["symbolic-ref", "HEAD", "refs/heads/trunk"],
        None,
    )
    .unwrap();
    commit(trunk.path(), "a", "a", "trunk");
    assert_eq!(default_branch(trunk.path()), None);

    let master = repo();
    git(
        master.path(),
        &["symbolic-ref", "HEAD", "refs/heads/master"],
        None,
    )
    .unwrap();
    commit(master.path(), "a", "a", "master");
    assert_eq!(default_branch(master.path()).as_deref(), Some("master"));

    let main = repo();
    git(
        main.path(),
        &["symbolic-ref", "HEAD", "refs/heads/main"],
        None,
    )
    .unwrap();
    let sha = commit(main.path(), "a", "a", "main");
    assert_eq!(default_branch(main.path()).as_deref(), Some("main"));
    git(
        main.path(),
        &["update-ref", "refs/remotes/origin/main", &sha],
        None,
    )
    .unwrap();
    git(
        main.path(),
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
        None,
    )
    .unwrap();
    assert_eq!(default_branch(main.path()).as_deref(), Some("origin/main"));
}

/// Decision 0038 rulings 1 and 2: the anchor records what the slice
/// changed per file; a clean rebase keeps every entry, and a changed
/// hunk moves exactly the entry it lives in.
#[test]
fn the_patch_identity_survives_a_rebase_and_names_the_file_that_changed() {
    let dir = repo();
    git(
        dir.path(),
        &["symbolic-ref", "HEAD", "refs/heads/main"],
        None,
    )
    .unwrap();
    commit(dir.path(), "src/lib.txt", "one\n", "base");
    let base = commit(dir.path(), "README.md", "# readme\n", "base readme");

    git(dir.path(), &["checkout", "-q", "-b", "slice"], None).unwrap();
    std::fs::write(dir.path().join("src/lib.txt"), "one\ntwo\n").unwrap();
    let head = commit(dir.path(), "docs/page.md", "# page\n", "the slice");

    let mut store = empty_store(dir.path(), "first");
    vouch(&mut store, "first", &head);
    let first = anchored_message(dir.path(), &anchor(&store, dir.path(), "first").unwrap());
    assert_eq!(first["anchor"], "forge.journal-anchor/v3");
    assert_eq!(first["repo_head"], head);
    assert_eq!(first["base"], base);
    let patch = first["patch"].as_object().unwrap();
    assert_eq!(
        patch.keys().collect::<Vec<_>>(),
        ["docs/page.md", "src/lib.txt"]
    );
    let report = verify(&store, dir.path(), "first").unwrap();
    assert_eq!(report["patch"], first["patch"]);

    // main moves under the slice; the rebase renews the head and
    // nothing else.
    git(dir.path(), &["checkout", "-q", "main"], None).unwrap();
    commit(dir.path(), "CHANGELOG.md", "moved\n", "main moves");
    git(dir.path(), &["checkout", "-q", "slice"], None).unwrap();
    git(dir.path(), &["rebase", "-q", "main"], None).unwrap();
    let rebased = git(dir.path(), &["rev-parse", "HEAD"], None).unwrap();
    assert_ne!(rebased, head);
    store
        .create_run("second", "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    vouch(&mut store, "second", &rebased);
    let second = anchored_message(dir.path(), &anchor(&store, dir.path(), "second").unwrap());
    assert_ne!(second["base"], base);
    assert_eq!(second["patch"], first["patch"]);

    // One more hunk in the page: only that entry moves.
    let edited = commit(
        dir.path(),
        "docs/page.md",
        "# page\n\nmore\n",
        "edit the page",
    );
    store
        .create_run("third", "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    vouch(&mut store, "third", &edited);
    let third = anchored_message(dir.path(), &anchor(&store, dir.path(), "third").unwrap());
    assert_eq!(third["patch"]["src/lib.txt"], first["patch"]["src/lib.txt"]);
    assert_ne!(
        third["patch"]["docs/page.md"],
        first["patch"]["docs/page.md"]
    );

    // A file moved whole: a rename is never paired, so the map carries
    // the deletion and the addition as two paths, and the old path does
    // not vanish from the record by being moved.
    git(dir.path(), &["mv", "src/lib.txt", "src/moved.txt"], None).unwrap();
    git(dir.path(), &["commit", "-q", "-m", "move the file"], None).unwrap();
    let moved = git(dir.path(), &["rev-parse", "HEAD"], None).unwrap();
    store
        .create_run("fourth", "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    vouch(&mut store, "fourth", &moved);
    let fourth = anchored_message(dir.path(), &anchor(&store, dir.path(), "fourth").unwrap());
    assert_eq!(
        fourth["patch"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        ["docs/page.md", "src/lib.txt", "src/moved.txt"]
    );
    assert_ne!(
        fourth["patch"]["src/lib.txt"],
        first["patch"]["src/lib.txt"]
    );
}

/// Decision 0038 ruling 1 says "per file", and a path is a name, not a
/// pattern: a leading `:` is not pathspec magic, `[1]` is not a glob
/// that also names its sibling, and a non-ASCII byte does not quote the
/// path into one that matches nothing. Each of those would otherwise
/// give the file an id that is empty (and null the whole map) or one
/// that is its sibling's too.
#[test]
fn a_path_is_a_name_not_a_pattern_and_keeps_its_own_id() {
    let dir = repo();
    git(
        dir.path(),
        &["symbolic-ref", "HEAD", "refs/heads/main"],
        None,
    )
    .unwrap();
    commit(dir.path(), "docs/a1.md", "a\n", "base");
    git(dir.path(), &["checkout", "-q", "-b", "slice"], None).unwrap();
    std::fs::write(dir.path().join("docs/a1.md"), "a\nb\n").unwrap();
    let head = commit(dir.path(), "docs/:a[1]ð.md", "x\n", "the slice");

    let mut store = empty_store(dir.path(), "first");
    vouch(&mut store, "first", &head);
    let first = anchored_message(dir.path(), &anchor(&store, dir.path(), "first").unwrap());
    let patch = first["patch"].as_object().expect("a map, not null");
    assert_eq!(
        patch.keys().collect::<Vec<_>>(),
        ["docs/:a[1]ð.md", "docs/a1.md"]
    );
    assert_eq!(patch["docs/:a[1]ð.md"].as_str().unwrap().len(), 40);

    // The sibling the glob would have paired moves; the named path's id
    // does not.
    let edited = commit(dir.path(), "docs/a1.md", "a\nb\nc\n", "edit the sibling");
    store
        .create_run("second", "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    vouch(&mut store, "second", &edited);
    let second = anchored_message(dir.path(), &anchor(&store, dir.path(), "second").unwrap());
    assert_eq!(
        second["patch"]["docs/:a[1]ð.md"],
        first["patch"]["docs/:a[1]ð.md"]
    );
    assert_ne!(second["patch"]["docs/a1.md"], first["patch"]["docs/a1.md"]);

    // The named path itself moves; so does its id, and only its id.
    let named = commit(
        dir.path(),
        "docs/:a[1]ð.md",
        "x\ny\n",
        "edit the named path",
    );
    store
        .create_run("third", "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    vouch(&mut store, "third", &named);
    let third = anchored_message(dir.path(), &anchor(&store, dir.path(), "third").unwrap());
    assert_ne!(
        third["patch"]["docs/:a[1]ð.md"],
        first["patch"]["docs/:a[1]ð.md"]
    );
    assert_eq!(third["patch"]["docs/a1.md"], second["patch"]["docs/a1.md"]);
}

#[test]
fn an_unknown_anchor_version_is_refused_not_guessed() {
    let dir = repo();
    let store = empty_store(dir.path(), "run");
    let (seq, hash) = store.head_hash("run").unwrap();
    let future = json!({
        "anchor": "forge.journal-anchor/v9",
        "run_id": "run",
        "seq": seq,
        "journal_head_hash": hash,
    })
    .to_string();
    let commit = git(dir.path(), &["commit-tree", EMPTY_TREE], Some(&future)).unwrap();
    git(dir.path(), &["update-ref", &ref_name("run"), &commit], None).unwrap();
    assert!(matches!(
        verify(&store, dir.path(), "run"),
        Err(AnchorError::Mismatch(message)) if message.contains("unknown anchor version")
    ));
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
