use super::*;

use brokkr_core::envelope::EventType;
use serde_json::json;

/// A repository with an identity, exactly like `anchor`'s tests build.
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

fn commit(dir: &Path, name: &str) -> String {
    std::fs::write(dir.join(name), name).unwrap();
    git(dir, &["add", name], None).unwrap();
    git(dir, &["commit", "-q", "-m", name], None).unwrap();
    git(dir, &["rev-parse", "HEAD"], None).unwrap()
}

fn store(dir: &Path) -> Store {
    Store::open(&dir.join("forge.db")).unwrap()
}

/// A fixture journal: a run whose decisions cite the given heads, in the
/// two shapes the contract records them in. Events, not a live run — the
/// exhibits are what is under test, not the protocol.
fn journal(store: &mut Store, run_id: &str, reviewed: &[&str], shipped: &[&str]) {
    store
        .create_run(run_id, "feature", "bundle", &json!({"files": {}}))
        .unwrap();
    for head in reviewed {
        store
            .append_next(
                run_id,
                EventType::TransitionDecided,
                json!({
                    "from": "review",
                    "result": "ok",
                    "next": "ship",
                    "inputs": {"reviewed_heads": {"the-forge": head}},
                }),
                None,
                None,
            )
            .unwrap();
    }
    for head in shipped {
        store
            .append_next(
                run_id,
                EventType::TransitionDecided,
                json!({
                    "from": "ship",
                    "result": "ok",
                    "next": "done",
                    "inputs": {"realm_facts": {"the-forge": {"head": head, "dirty_worktrees": false}}},
                }),
                None,
                None,
            )
            .unwrap();
    }
}

fn refs(dir: &Path) -> String {
    git(
        dir,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            KEEP_PREFIX,
        ],
        None,
    )
    .unwrap()
}

fn resolvable(dir: &Path, sha: &str) -> bool {
    git(dir, &["cat-file", "-e", &format!("{sha}^{{commit}}")], None).is_ok()
}

/// The whole point: the branch that carried the exhibits is deleted and
/// collected, and the cited commits are still there — held by nothing
/// but their keep-refs. The uncited tip is the control: it goes.
#[test]
fn cited_commits_outlive_the_branch_and_the_collector() {
    let dir = repo();
    let path = dir.path();
    let base = git(path, &["symbolic-ref", "--short", "HEAD"], None).unwrap();
    commit(path, "base");

    git(path, &["checkout", "-q", "-b", "exhibits"], None).unwrap();
    let reviewed = commit(path, "one");
    let shipped = commit(path, "two");
    let uncited = commit(path, "three");
    git(path, &["checkout", "-q", &base], None).unwrap();

    let mut store = store(path);
    // The same head cited twice, in both shapes: one exhibit, one ref.
    journal(
        &mut store,
        "run-a",
        &[&reviewed, &reviewed],
        &[&shipped, &reviewed],
    );
    let planted = plant(&store, path, "run-a").unwrap();
    assert_eq!(planted.kept.len(), 2, "one ref per distinct cited object");
    assert!(planted.absent.is_empty());

    git(path, &["branch", "-D", "exhibits"], None).unwrap();
    git(
        path,
        &[
            "reflog",
            "expire",
            "--expire=now",
            "--expire-unreachable=now",
            "--all",
        ],
        None,
    )
    .unwrap();
    git(path, &["gc", "--prune=now", "--quiet"], None).unwrap();

    assert!(resolvable(path, &reviewed), "kept by refs/forge/keep");
    assert!(resolvable(path, &shipped), "kept by refs/forge/keep");
    assert!(
        !resolvable(path, &uncited),
        "an uncited commit is exactly as mortal as it was — the keep-refs \
         are why the other two survived"
    );
}

/// Replanting is a no-op, not a second plant: same names, same targets,
/// byte for byte, and no error from writing a ref its existing value.
#[test]
fn replanting_moves_nothing() {
    let dir = repo();
    let path = dir.path();
    let head = commit(path, "one");
    let mut store = store(path);
    journal(&mut store, "run-a", &[&head], &[]);

    let first = plant(&store, path, "run-a").unwrap();
    let before = refs(path);
    let second = plant(&store, path, "run-a").unwrap();
    assert_eq!(first, second);
    assert_eq!(before, refs(path));
    assert_eq!(
        before,
        format!("{}/run-a/{head} {head}", KEEP_PREFIX),
        "named by the object, pointing at it"
    );
}

/// One `for-each-ref` answers which runs hold which exhibits.
#[test]
fn listing_groups_every_runs_exhibits() {
    let dir = repo();
    let path = dir.path();
    let one = commit(path, "one");
    let two = commit(path, "two");
    let three = commit(path, "three");
    let mut store = store(path);
    journal(&mut store, "run-a", &[&one], &[&two]);
    journal(&mut store, "run-b", &[&three], &[]);
    plant(&store, path, "run-a").unwrap();
    plant(&store, path, "run-b").unwrap();

    let mut expected_a = vec![one.clone(), two.clone()];
    expected_a.sort();
    assert_eq!(
        list(path).unwrap(),
        BTreeMap::from([
            ("run-a".to_string(), expected_a),
            ("run-b".to_string(), vec![three.clone()]),
        ])
    );

    // Deletion is one run's alone, and "already gone" is not an error.
    assert_eq!(delete(path, "run-a").unwrap(), 2);
    assert_eq!(
        list(path).unwrap(),
        BTreeMap::from([("run-b".to_string(), vec![three])])
    );
    assert_eq!(delete(path, "run-a").unwrap(), 0);
}

/// Releasing is one run's alone even when another run's id begins with
/// it. `delete` names the namespace by pattern, and a pattern that took
/// `run-a-second` along with `run-a` would destroy exhibits no operator
/// asked to release — so the boundary is asserted, not assumed.
#[test]
fn releasing_one_run_spares_a_run_whose_id_extends_it() {
    let dir = repo();
    let path = dir.path();
    let one = commit(path, "one");
    let two = commit(path, "two");
    let mut store = store(path);
    journal(&mut store, "run-a", &[&one], &[]);
    journal(&mut store, "run-a-second", &[&two], &[]);
    plant(&store, path, "run-a").unwrap();
    plant(&store, path, "run-a-second").unwrap();

    assert_eq!(delete(path, "run-a").unwrap(), 1, "exactly its own");
    assert_eq!(
        list(path).unwrap(),
        BTreeMap::from([("run-a-second".to_string(), vec![two])]),
        "the neighbour keeps its exhibits"
    );
}

/// A citation this repository cannot resolve is reported, not planted
/// and not fatal: another realm's head, or an object already collected.
#[test]
fn absent_citations_are_named_rather_than_swallowed() {
    let dir = repo();
    let path = dir.path();
    let head = commit(path, "one");
    let elsewhere = "1234567890123456789012345678901234567890";
    let mut store = store(path);
    journal(&mut store, "run-a", &[&head, elsewhere], &[]);

    let planted = plant(&store, path, "run-a").unwrap();
    assert_eq!(planted.kept, vec![head.clone()]);
    assert_eq!(planted.absent, vec![elsewhere.to_string()]);
    assert_eq!(refs(path), format!("{}/run-a/{head} {head}", KEEP_PREFIX));
}

/// What a concluding run says: nothing when every exhibit is held, the
/// citations it could not keep when some are absent, and the refusal
/// itself when git would not answer at all.
#[test]
fn a_conclusion_reports_only_what_it_could_not_keep() {
    let dir = repo();
    let path = dir.path();
    let head = commit(path, "one");
    let elsewhere = "1234567890123456789012345678901234567890";
    let mut store = store(path);
    journal(&mut store, "run-a", &[&head], &[]);
    journal(&mut store, "run-b", &[&head, elsewhere], &[]);

    assert_eq!(plant_or_report(&store, path, "run-a"), None);
    assert_eq!(
        plant_or_report(&store, path, "run-b"),
        Some(format!(
            "keep-ref gap for run-b: 1 cited object(s) are not in this repository: {elsewhere}"
        ))
    );
    let refused = plant_or_report(&store, &path.join("not-a-repository"), "run-a").unwrap();
    assert!(
        refused.starts_with("keep-ref gap for run-a: git "),
        "{refused}"
    );
}

/// A journal citing nothing plants nothing — no refs, no git call that
/// could fail, no empty namespace left behind.
#[test]
fn a_run_that_cites_nothing_plants_nothing() {
    let dir = repo();
    let path = dir.path();
    commit(path, "one");
    let mut store = store(path);
    journal(&mut store, "run-a", &[], &[]);

    assert_eq!(
        plant(&store, path, "run-a").unwrap(),
        Planted {
            run_id: "run-a".to_string(),
            kept: vec![],
            absent: vec![],
        }
    );
    assert!(refs(path).is_empty());
    assert!(list(path).unwrap().is_empty());
}

/// Only refs shaped `<prefix>/<run>/<sha>` name a holder. A hand-planted
/// ref elsewhere in the namespace is passed over — the listing reports
/// what runs cited, never what someone left lying there.
#[test]
fn only_a_run_and_an_object_make_a_holder() {
    assert_eq!(
        holder("refs/forge/keep/run-a/abc def"),
        Some(("run-a", "def"))
    );
    assert_eq!(holder("refs/forge/keep/run-a/abc"), None, "no target");
    assert_eq!(
        holder("refs/forge/run-a abc"),
        None,
        "outside the namespace"
    );
    assert_eq!(holder("refs/forge/keepish/r/s abc"), None, "not the prefix");
    assert_eq!(holder("refs/forge/keep/loose abc"), None, "no object part");

    let dir = repo();
    let path = dir.path();
    let head = commit(path, "one");
    git(
        path,
        &["update-ref", &format!("{KEEP_PREFIX}/loose"), &head],
        None,
    )
    .unwrap();
    assert!(list(path).unwrap().is_empty());
}

/// A run the workspace does not hold is the store's refusal, unchanged.
#[test]
fn planting_for_an_unknown_run_is_the_journals_own_refusal() {
    let dir = repo();
    let path = dir.path();
    let store = store(path);
    assert!(matches!(
        plant(&store, path, "no-such-run"),
        Err(KeepRefsError::Store(_))
    ));
}

#[test]
fn a_run_id_that_could_reshape_the_namespace_is_refused() {
    let dir = repo();
    let path = dir.path();
    let store = store(path);
    for hostile in [
        "",
        "../escape",
        "refs/heads/main",
        "-dashed",
        ".hidden",
        "trailing.",
        "a b",
    ] {
        assert!(
            matches!(usable(hostile), Err(KeepRefsError::UnusableRunId(_))),
            "{hostile} must not name a keep-ref"
        );
        assert!(matches!(
            plant(&store, path, hostile),
            Err(KeepRefsError::UnusableRunId(_))
        ));
        assert!(matches!(
            delete(path, hostile),
            Err(KeepRefsError::UnusableRunId(_))
        ));
    }
}

#[test]
fn git_failures_name_their_boundary() {
    let missing = Path::new("/forge/definitely/missing/repository");
    assert!(matches!(
        git(missing, &["cat-file", "--batch-check"], Some("")),
        Err(KeepRefsError::Git { verb: "spawn", .. })
    ));
    assert!(matches!(
        git(missing, &["for-each-ref"], None),
        Err(KeepRefsError::Git { verb: "run", .. })
    ));
    assert!(matches!(
        git_io::<()>(Err(std::io::Error::other("wait failed")), "wait"),
        Err(KeepRefsError::Git { verb: "wait", .. })
    ));
    assert!(matches!(list(missing), Err(KeepRefsError::Git { .. })));
}
