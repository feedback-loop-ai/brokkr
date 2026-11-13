//! The run's lock at the engine's doors (decision 0108 ruling 1): a
//! resume of a run another driver holds is refused at once, naming the
//! holder, and the next resume takes the run once that driver lets go.

use super::*;

/// The bundle every start and resume here pins.
fn bundle(dir: &Path) -> Bundle {
    super::tests::bundle(
        dir,
        super::tests::single_body(vec!["missing-driver".into()]),
    )
}

/// The lock directory beside `db`.
fn runs_path(db: &Path) -> PathBuf {
    let mut at = std::ffi::OsString::from(db);
    at.push(".runs");
    PathBuf::from(at)
}

/// The typed refusal a door answered with, or a panic naming what was
/// expected.
fn refused<T>(result: Result<T, EngineError>, why: &str) -> EngineError {
    match result {
        Ok(_) => panic!("{why}"),
        Err(error) => error,
    }
}

/// Assert a door refused the lock directory, and journaled no run.
fn refused_directory(db: &Path, refused: EngineError) {
    assert!(matches!(
        refused,
        EngineError::RunLock(crate::RunLockError::Directory(_))
    ));
    assert!(Store::open(db).unwrap().list_runs().unwrap().is_empty());
}

/// A journal at `name` whose run lock directory is a regular file, so no
/// start or resume can take a lock.
fn blocked(dir: &Path, name: &str) -> (PathBuf, Store) {
    let db = dir.join(name);
    let store = Store::open(&db).unwrap();
    std::fs::write(runs_path(&db), "").unwrap();
    (db, store)
}

/// A run another driver holds is refused by `resume` at once, naming the
/// holder; once that driver lets go, the next resume takes the run
/// (decision 0108 ruling 1).
#[test]
fn a_held_run_refuses_resume_naming_its_driver() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let db = dir.path().join("forge.db");
    let store = Store::open(&db).unwrap();
    let bundle = bundle(dir.path());
    let engine = Engine::start(
        store,
        bundle.clone(),
        "held resume",
        Some(dir.path().join("work")),
    )
    .unwrap();
    let run = engine.run_id.clone();
    let lock = runs_path(&db).join(format!("{run}.lock"));
    let recorded: crate::run_lock::Holder =
        serde_json::from_str(&std::fs::read_to_string(&lock).unwrap()).unwrap();
    let refused = refused(
        Engine::resume(
            Store::open(&db).unwrap(),
            bundle.clone(),
            &run,
            Some(dir.path().join("work")),
        ),
        "a held run must refuse a second driver",
    );
    assert!(matches!(
        &refused,
        EngineError::RunLock(crate::RunLockError::Driven { run_id, .. }) if run_id == &run
    ));
    assert_eq!(
        refused.to_string(),
        format!(
            "run '{run}' is driven by pid {} on this host since {} (engine {})",
            std::process::id(),
            recorded.since,
            ENGINE_VERSION
        )
    );
    drop(engine);
    Engine::resume(
        Store::open(&db).unwrap(),
        bundle,
        &run,
        Some(dir.path().join("work")),
    )
    .expect("the next driver takes the run");
}

/// The lock directory refuses a fresh start before its first append, so a
/// failed hold journals no run.
#[test]
fn a_start_refuses_a_run_lock_it_cannot_take() {
    let dir = tempfile::tempdir().unwrap();
    let (db, store) = blocked(dir.path(), "start-blocked.db");
    let refused = refused(
        Engine::start(
            store,
            bundle(dir.path()),
            "blocked start",
            Some(dir.path().join("work")),
        ),
        "a start without a lock must refuse",
    );
    refused_directory(&db, refused);
}

/// A dispatched start takes the same lock, and refuses before its row.
#[test]
fn a_dispatched_start_refuses_a_run_lock_it_cannot_take() {
    let dir = tempfile::tempdir().unwrap();
    let (db, store) = blocked(dir.path(), "dispatched-blocked.db");
    let bundle = bundle(dir.path());
    let envelope = super::tests::dispatch(&bundle);
    let refused = refused(
        Engine::start_with_dispatch(
            store,
            bundle,
            "blocked dispatch",
            Some(dir.path().into()),
            envelope,
        ),
        "a dispatched start without a lock must refuse",
    );
    refused_directory(&db, refused);
}

/// A resume takes the lock before reading the manifest, and refuses a lock
/// that is not this user's own by name.
#[test]
fn a_resume_refuses_a_run_lock_it_cannot_take() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let db = dir.path().join("resume-blocked.db");
    let store = Store::open(&db).unwrap();
    let bundle = bundle(dir.path());
    let mut engine = Engine::start(
        store,
        bundle.clone(),
        "blocked resume",
        Some(dir.path().join("work")),
    )
    .unwrap();
    let run = engine.run_id.clone();
    engine.release_run_lock();
    let at = runs_path(&db);
    std::fs::remove_dir_all(&at).unwrap();
    std::fs::write(&at, "").unwrap();
    let refused = refused(
        Engine::resume(
            Store::open(&db).unwrap(),
            bundle,
            &run,
            Some(dir.path().join("work")),
        ),
        "a resume without a lock must refuse",
    );
    assert!(matches!(
        refused,
        EngineError::RunLock(crate::RunLockError::Directory(_))
    ));
}
