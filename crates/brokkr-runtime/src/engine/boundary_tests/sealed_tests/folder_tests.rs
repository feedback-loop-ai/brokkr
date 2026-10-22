//! Decision 0091 ruling 1 at the spawn: a seat taken from the folder runs
//! the folder's own script, and decision 0048's walk checks
//! `.brokkr/scripts/` against the map its compile pinned, reached through
//! the roots the compiled bundle holds, never a hand-set walk. An edit to
//! the folder moves the identity resume compares, by its folder key.

use super::*;
use crate::bundle::compose::folder_tests::{compile, from_folder, Folder};

/// A recipe on `folder`'s shelf whose verify is the folder's own, compiled
/// in the folder.
fn compiled(folder: &Folder) -> Bundle {
    let leaf = folder.recipe(json!({"verify": from_folder()}));
    compile(&leaf, &folder.shelves()).unwrap()
}

/// The compiled verify seat's spawn, composed by the engine itself: its
/// script directory is found among the bundle's roots.
fn verify_spawn(engine: &mut Engine) -> SiteSpawn {
    let command = match &engine.bundle.seats["verify"].body {
        SeatBody::Invocation(Invocation::Single { command, .. }) => command.clone(),
        other => panic!("verify is a single site: {other:?}"),
    };
    let hands = engine.bundle.hands["verify"].clone();
    engine.compose("attempt", true, command, Some(&hands), None, "result.json")
}

/// Why the spawn did not start: refused before it, or, its walk passed,
/// the engine that is not there.
fn not_started(engine: &mut Engine, spawn: &SiteSpawn) -> String {
    let deadline = std::time::Duration::from_secs(1);
    let run = engine.run_driver(
        "effect",
        "attempt",
        "verify",
        spawn,
        json!({}),
        deadline,
        None,
        None,
    );
    match run.unwrap() {
        DriverRun::SpawnFailed(error) => error,
        DriverRun::Ran(_) => panic!("the absent engine ran"),
    }
}

/// The spawn refusal naming the folder layer and the moved key.
fn moved(change: &str) -> String {
    format!(
        "driver did not spawn: unboxed exec dispatch refused: layer 'starter' moved since the \
         compile ({change}); the \
         bytes an unboxed gate runs must be the bytes its digest names (decision 0046 ruling 4)"
    )
}

#[test]
fn a_folder_script_edited_removed_or_added_after_the_compile_refuses_at_spawn() {
    if super::already_boxed() {
        return;
    }
    let folder = Folder::new();
    let (_dir, mut engine) = crate::engine::tests::engine(single_body(vec!["unused".into()]));
    engine.bundle = compiled(&folder);
    engine.boundary = Boundary::Harness;
    // The probe's answer is pinned, so every host composes the engine's own
    // argv and the untouched spawn fails as the absent engine, never as
    // `unshare` started where the network prefix works.
    engine.network_prefix = Some(false);
    let spawn = verify_spawn(&mut engine);
    let scripts = folder.folder().canonicalize().unwrap().join("scripts");
    assert_eq!(spawn.rewalk, Some(scripts.clone()));
    // Untouched: the walk passes and the spawn is attempted.
    let untouched = not_started(&mut engine, &spawn);
    let script = scripts.join("verify-seat.sh");
    let absent = format!(
        "driver did not spawn: failed to spawn driver /nonexistent/brokkr driver exec -- bash {} \
         {{prompt_file}}: No such file or directory (os error 2)",
        script.display()
    );
    assert_eq!(untouched, absent);
    let kept = std::fs::read(&script).unwrap();
    std::fs::write(&script, "#!/bin/sh\necho edited\n").unwrap();
    let edited = not_started(&mut engine, &spawn);
    assert_eq!(edited, moved("changed: scripts/verify-seat.sh"));
    std::fs::remove_file(&script).unwrap();
    let removed = not_started(&mut engine, &spawn);
    assert_eq!(removed, moved("missing: scripts/verify-seat.sh"));
    std::fs::write(&script, &kept).unwrap();
    std::fs::write(scripts.join("extra.sh"), "new\n").unwrap();
    let added = not_started(&mut engine, &spawn);
    assert_eq!(added, moved("added: scripts/extra.sh"));
}

/// Resume compares the run's pinned manifest with a compile of today's
/// bytes (decision 0017): an edit to the folder's script names its folder
/// key and nothing else.
#[test]
fn an_edit_to_the_folder_is_named_by_resumes_comparison() {
    let folder = Folder::new();
    let pinned = compiled(&folder);
    let script = folder.folder().join("scripts/verify-seat.sh");
    std::fs::write(&script, "#!/bin/sh\necho edited\n").unwrap();
    let current = compiled(&folder);
    let diff = crate::engine::manifest_diff(&pinned.manifest, &current.manifest);
    assert_eq!(diff, "changed: @folder/starter");
}
