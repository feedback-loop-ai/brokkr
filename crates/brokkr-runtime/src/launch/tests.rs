//! One admission, refusal by refusal and entry point by entry point
//! (#350): each refusal fires from its one site for `start`, `resume`
//! and `rerun` alike, and before the journal (for a new run) or any row
//! of it (for a run the journal already holds) exists.

use std::path::{Path, PathBuf};

use serde_json::json;

use super::*;
use crate::realms::tests::{crossing_map, published_by_alpha, two_repositories};

/// The repository root: its `agents/` and `adapters/` are what a compile
/// reads, as any workspace's are.
fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A two-phase bundle of exec seats under `root/bundle`; the `work` seat
/// boxes its hands in the workspace when `boxed`.
pub(super) fn bundle_at(root: &Path, boxed: bool) -> PathBuf {
    let dir = root.join("bundle");
    std::fs::create_dir_all(dir.join("roles")).unwrap();
    std::fs::write(dir.join("roles/seat.md"), "# seat\n").unwrap();
    let table = json!({
        "schema": "forge.phase-machine/v1", "initial": "work",
        "phases": ["work", "review", "done"], "terminal": ["done"],
        "rules": [
            {"id": "WORKED", "from": "work", "result": "complete", "next": "review", "reason": "worked"},
            {"id": "CLEAN", "from": "review", "result": "clean", "next": "done", "reason": "clean"}
        ]
    });
    std::fs::write(dir.join("policy.json"), table.to_string()).unwrap();
    let seat = |result: &str| {
        json!({"role": "roles/seat.md", "results": [result],
               "driver": {"command": ["{brokkr}", "driver", "exec", "--", "true"]}})
    };
    let mut work = seat("complete");
    if boxed {
        work["hands"] = json!("workspace");
    }
    let config = json!({"name": "launch", "policy": "policy.json",
                        "seats": {"work": work, "review": seat("clean")}});
    std::fs::write(dir.join("bundle.json"), config.to_string()).unwrap();
    dir
}

/// A request over `dir`, journaled at `root/forge.db`, operating `repo`,
/// looking for a boundary's tool on `host`.
fn request(root: &Path, dir: &Path, repo: &Path, host: &Path) -> LaunchRequest {
    LaunchRequest {
        workspace: workspace(),
        bundle: BundleSource::Dir(dir.to_path_buf()),
        journal: root.join("forge.db"),
        repo: Some(repo.to_path_buf()),
        secrets: Some(root.join("secrets.env")),
        host_path: host.as_os_str().to_owned(),
    }
}

fn new_run(world: Option<World>) -> NewRun {
    NewRun {
        feature: "launch".into(),
        map: world.map_or(RunMap::Unmapped, RunMap::Ambient),
        ..NewRun::default()
    }
}

/// A new run under the map at `root`, named, beside a dispatch that is
/// never read.
fn named_and_dispatched(root: &Path) -> NewRun {
    NewRun {
        map: RunMap::Named(World::discover(root, None).unwrap().unwrap()),
        dispatch: Some(root.join("never-read.json")),
        ..new_run(None)
    }
}

fn silent(_: &str) {}

/// Every row the journal holds, run by run: what a refusal must leave
/// exactly as it found it.
fn rows(journal: &Path) -> Vec<(String, usize)> {
    let store = Store::open(journal).unwrap();
    store
        .list_runs()
        .unwrap()
        .into_iter()
        .map(|(run, ..)| {
            let events = store.load(&run).unwrap().len();
            (run, events)
        })
        .collect()
}

/// A search path holding a `bwrap` — enough for a `workspace` box with
/// no overlay bind, which asks bubblewrap for no version.
fn host_with_bwrap(root: &Path) -> PathBuf {
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("bwrap"), "").unwrap();
    bin
}

#[test]
fn a_named_map_beside_a_dispatch_is_refused_before_a_journal_exists() {
    let (dir, _, _) = two_repositories();
    let bundle = bundle_at(dir.path(), false);
    let asked = request(dir.path(), &bundle, dir.path(), dir.path());
    let run = named_and_dispatched(dir.path());
    let refusal = start(asked, run, &mut silent).err().expect("refused");
    assert!(matches!(refusal, LaunchError::RealmsWithDispatch));
    assert!(refusal
        .to_string()
        .starts_with("a run with --dispatch cannot pin the map named by --realms: "));
    assert!(!dir.path().join("forge.db").exists());
}

/// A recipe is resolved inside the launch, where each verb always
/// resolved it: after the refusals that always preceded it (a named map
/// beside a dispatch; a source run the journal does not hold; a run it
/// holds no manifest for), and before anything is compiled or written.
#[test]
fn a_recipe_that_is_not_installed_is_refused_at_every_entry_point_before_a_row_is_written() {
    let (dir, _, _) = two_repositories();
    let bundle = bundle_at(dir.path(), false);
    let asked = request(dir.path(), &bundle, dir.path(), dir.path());
    let run = start(asked.clone(), new_run(None), &mut silent)
        .unwrap()
        .run_id
        .clone();
    let before = rows(&asked.journal);
    let recipes_dir = dir.path().join("recipes");
    let uninstalled = LaunchRequest {
        bundle: BundleSource::Recipe {
            name: "absent".into(),
            recipes_dir: recipes_dir.clone(),
        },
        ..asked.clone()
    };
    let missing = format!(
        "recipe 'absent' not found under {}; install it with `brokkr recipes add <source> \
         --name absent`",
        recipes_dir.display()
    );

    let elsewhere = LaunchRequest {
        journal: dir.path().join("never-created.db"),
        ..uninstalled.clone()
    };
    let refusal = start(elsewhere.clone(), new_run(None), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(refusal, LaunchError::RecipeNotFound { ref name, .. } if name == "absent"));
    assert_eq!(refusal.to_string(), missing);
    let named = named_and_dispatched(dir.path());
    let refusal = start(elsewhere.clone(), named, &mut silent).err();
    assert!(matches!(refusal, Some(LaunchError::RealmsWithDispatch)));
    assert!(!elsewhere.journal.exists());

    let refusal = resume(uninstalled.clone(), &run).err().expect("refused");
    assert_eq!(refusal.to_string(), missing);
    let refusal = rerun(uninstalled.clone(), &run, None)
        .err()
        .expect("refused");
    assert_eq!(refusal.to_string(), missing);
    let refusal = rerun(uninstalled.clone(), "gone", None).err();
    assert!(matches!(refusal, Some(LaunchError::SourceRun { ref run, .. }) if run == "gone"));
    let refusal = resume(uninstalled.clone(), "gone").err();
    assert!(
        matches!(refusal, Some(LaunchError::Store(StoreError::RunNotFound(ref run))) if run == "gone")
    );
    assert_eq!(rows(&asked.journal), before);

    let installed = BundleSource::Recipe {
        name: "bundle".into(),
        recipes_dir: dir.path().to_path_buf(),
    };
    assert_eq!(installed.resolve().unwrap(), bundle);
}

#[test]
fn an_unboxable_bundle_is_refused_at_every_entry_point_before_a_row_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = bundle_at(dir.path(), true);
    let bare = dir.path().join("empty");
    let unboxable = request(dir.path(), &bundle, dir.path(), &bare);
    let missing = "the `namespace` boundary needs `bwrap` on PATH and none was found; the seats \
                   [\"work\"] declare hands and cannot run on this machine — the boundary is \
                   never simulated (decision 0046 ruling 2)";

    let refusal = start(unboxable.clone(), new_run(None), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(
        refusal,
        LaunchError::Unboxable(Unboxable::MissingTool { tool: "bwrap", .. })
    ));
    assert_eq!(refusal.to_string(), missing);
    assert!(!dir.path().join("forge.db").exists());

    let boxed = request(
        dir.path(),
        &bundle,
        dir.path(),
        &host_with_bwrap(dir.path()),
    );
    let engine = start(boxed, new_run(None), &mut silent).unwrap();
    assert_eq!(engine.secrets_file, Some(dir.path().join("secrets.env")));
    let run = engine.run_id.clone();
    drop(engine);
    let before = rows(&unboxable.journal);

    let refusal = resume(unboxable.clone(), &run).err().expect("refused");
    assert_eq!(refusal.to_string(), missing);
    let refusal = rerun(unboxable.clone(), &run, None).err().expect("refused");
    assert_eq!(refusal.to_string(), missing);
    assert_eq!(rows(&unboxable.journal), before);
}

#[test]
fn a_moved_crossing_is_refused_at_every_entry_point_before_a_row_is_written() {
    let (dir, _, _) = two_repositories();
    let (published, pin) = published_by_alpha(dir.path(), "{\"title\": \"orders\"}\n");
    std::fs::write(dir.path().join("realms.json"), crossing_map(&pin)).unwrap();
    let world = || World::discover(dir.path(), None).unwrap();
    let bundle = bundle_at(dir.path(), false);
    let asked = request(dir.path(), &bundle, &dir.path().join("beta"), dir.path());
    let engine = start(asked.clone(), new_run(world()), &mut silent).unwrap();
    let run = engine.run_id.clone();
    drop(engine);
    let before = rows(&asked.journal);

    // A world loaded while the contract held, fenced after it moved.
    let loaded = world();
    std::fs::write(&published, "{\"title\": \"Orders\"}\n").unwrap();
    let moved = loaded
        .as_ref()
        .unwrap()
        .verify_crossings(dir.path())
        .unwrap_err()
        .to_string();
    assert!(moved.contains("realm 'beta' consumes crossing 'orders.api'"));

    let elsewhere = LaunchRequest {
        journal: dir.path().join("never-created.db"),
        ..asked.clone()
    };
    let refusal = start(elsewhere.clone(), new_run(loaded), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(refusal, LaunchError::World(_)));
    assert_eq!(refusal.to_string(), moved);
    assert!(!elsewhere.journal.exists());

    let reloaded = World::inspect(dir.path(), None).unwrap();
    let refusal = rerun(asked.clone(), &run, reloaded).err().expect("refused");
    assert_eq!(refusal.to_string(), moved);
    let refusal = resume(asked.clone(), &run).err().expect("refused");
    assert_eq!(refusal.to_string(), moved);
    assert_eq!(rows(&asked.journal), before);
}

#[test]
fn a_dispatch_that_cannot_be_read_or_parsed_is_refused_before_a_journal_exists() {
    let (dir, _, _) = two_repositories();
    let bundle = bundle_at(dir.path(), false);
    let asked = request(dir.path(), &bundle, &dir.path().join("beta"), dir.path());
    let dispatched = |path: PathBuf, world: Option<World>| NewRun {
        dispatch: Some(path),
        ..new_run(world)
    };

    // An ambient map is not pinned into a dispatched run, and says so
    // before the envelope is read.
    let mut notes = Vec::new();
    let missing = dir.path().join("missing.json");
    let world = World::discover(dir.path(), None).unwrap();
    let refusal = start(
        asked.clone(),
        dispatched(missing.clone(), world),
        &mut |note| notes.push(note.to_string()),
    )
    .err()
    .expect("refused");
    assert!(matches!(refusal, LaunchError::ReadDispatch { ref path, .. } if *path == missing));
    assert_eq!(
        notes,
        [format!(
            "note: {} is not pinned into this run: --dispatch writes a run-manifest/v2, which \
             carries no world",
            dir.path().join("realms.json").display()
        )]
    );

    let malformed = dir.path().join("malformed.json");
    std::fs::write(&malformed, "not json").unwrap();
    let refusal = start(asked.clone(), dispatched(malformed, None), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(refusal, LaunchError::ParseDispatch(_)));
    assert_eq!(refusal.to_string(), "parsing forge-dispatch/v2");
    assert!(!asked.journal.exists());
}

/// An envelope sealed for the bundle but bounding fewer attempts than a
/// seat may make is refused with the engine's own text before a journal
/// exists; one that bounds it passes, and meets the engine's lineage
/// refusal (decision 0065 ruling 8). And a bad envelope is refused ahead
/// of a peer's lock on the journal, which is never waited on for it.
#[test]
fn a_dispatch_that_does_not_bound_the_bundle_is_refused_before_a_journal_exists() {
    let (dir, _, _) = two_repositories();
    let dir_bundle = bundle_at(dir.path(), false);
    let asked = request(dir.path(), &dir_bundle, dir.path(), dir.path());
    let compiled = || compile_for(&workspace(), &dir_bundle, None, dir.path()).unwrap();
    let envelope = dir.path().join("envelope.json");
    let dispatched = || NewRun {
        dispatch: Some(envelope.clone()),
        ..new_run(None)
    };
    let seal = |bundle: &Bundle| {
        let sealed = crate::engine::tests::dispatch(bundle);
        std::fs::write(&envelope, serde_json::to_string(&sealed).unwrap()).unwrap();
    };

    let config = dir_bundle.join("bundle.json");
    let bounded = std::fs::read_to_string(&config).unwrap();
    let mut widened: Value = serde_json::from_str(&bounded).unwrap();
    widened["seats"]["work"]["limits"] = json!({"max_attempts": 4});
    std::fs::write(&config, widened.to_string()).unwrap();
    seal(&compiled());
    let refusal = start(asked.clone(), dispatched(), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(
        refusal,
        LaunchError::Engine(EngineError::Dispatch(DispatchError::UnsafeBounds))
    ));
    assert_eq!(
        refusal.to_string(),
        "dispatch: dispatch execution bounds are unsafe"
    );
    assert!(!asked.journal.exists());

    std::fs::write(&config, bounded).unwrap();
    seal(&compiled());
    // Bounded, it is past every check the launch makes, and meets decision
    // 0065 ruling 8 (design D7) in the engine: every compiled bundle pins
    // its capability authority, which the frozen v2 lineage cannot carry,
    // so the start is refused by that key's name and writes no row.
    let refusal = start(asked.clone(), dispatched(), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(
        refusal,
        LaunchError::Engine(EngineError::Dispatch(
            DispatchError::ManifestKeyUnsupportedByDispatchLineage(ref key)
        )) if key == "capabilities"
    ));
    assert!(rows(&asked.journal).is_empty());

    // A peer mid-way through a journal's first open: its rollback-journal
    // file held exclusively, which an open waits out (its `ensure_wal`).
    let held = LaunchRequest {
        journal: dir.path().join("held.db"),
        ..asked
    };
    let peer = rusqlite::Connection::open(&held.journal).unwrap();
    peer.execute_batch("BEGIN EXCLUSIVE; CREATE TABLE t (x)")
        .unwrap();
    std::fs::remove_file(&envelope).unwrap();
    let refusal = start(held, dispatched(), &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(refusal, LaunchError::ReadDispatch { ref path, .. } if *path == envelope));
    assert!(refusal.contention().is_none());
}

#[test]
fn a_rerun_of_a_run_the_journal_does_not_hold_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = bundle_at(dir.path(), false);
    let asked = request(dir.path(), &bundle, dir.path(), dir.path());
    let refusal = rerun(asked, "absent", None).err().expect("refused");
    assert!(matches!(refusal, LaunchError::SourceRun { ref run, .. } if run == "absent"));
    assert!(refusal
        .to_string()
        .starts_with("loading source run 'absent'"));
}

/// Rebuild unit 12-fix-f, the council's A-E1 (design D6): a resume whose
/// capability authority cannot be reproduced re-wraps the compiler's raw
/// reason, so the whole line the engine renders is made through the one
/// refusal sink — one line, its newline escaped, cut to 512 scalar values.
#[test]
fn an_unreproducible_resume_leaves_through_the_one_refusal_sink() {
    let reason = format!("realm 'a\nb': {}", "x".repeat(600));
    let line = unreproducible(
        "run-1",
        LaunchError::Compile(CompileError::Capability(reason)),
    )
    .to_string();
    let head = "run 'run-1' pins a different bundle: capabilities differ: the capability \
                authority the run was started under cannot be reproduced here — realm \
                'a\\nb': ";
    assert_eq!(
        line,
        format!("{head}{}…", "x".repeat(511 - head.chars().count()))
    );
    assert_eq!(line.chars().count(), 512);
}

#[test]
fn resume_compilation_reads_the_dialect_from_the_pinned_world() {
    let root = workspace();
    assert!(compile_from_manifest(
        &root,
        &root.join("recipes/triage"),
        &json!({"bundle_name":"unadopted"}),
        &root,
    )
    .is_ok());

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("dialects")).unwrap();
    std::fs::copy(
        root.join("dialects/openspec.json"),
        dir.path().join("dialects/openspec.json"),
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("dialects/openspec")).unwrap();
    for name in [
        "specify", "return", "design", "tasks", "clarify", "analyze", "archive",
    ] {
        std::fs::copy(
            root.join(format!("dialects/openspec/{name}.md")),
            dir.path().join(format!("dialects/openspec/{name}.md")),
        )
        .unwrap();
    }
    // The pinned world is where the operator's abstract definitions are
    // read from, and a compile that loads the shipped library resolves the
    // asks of EVERY loaded agent (decision 0066 ruling 8) — the researcher's
    // two wants included, though triage seats it nowhere. A map directory
    // without them refuses the compile, so this world carries them.
    std::fs::create_dir(dir.path().join("capabilities")).unwrap();
    for entry in std::fs::read_dir(root.join("capabilities")).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(
            entry.path(),
            dir.path().join("capabilities").join(entry.file_name()),
        )
        .unwrap();
    }
    let map = json!({
        "schema":"forge.realms/v3",
        "realms":[{"name":"pinned","path":root,"default_branch":"main","dialect":"openspec"}],
        "journal":"forge.db"
    });
    std::fs::write(dir.path().join("realms.json"), map.to_string()).unwrap();
    let world = World::load(&dir.path().join("realms.json")).unwrap();
    let manifest = world
        .pinned(&json!({"bundle_name":"triage"}), Some(&root))
        .unwrap();
    let bundle =
        compile_from_manifest(&root, &root.join("recipes/triage"), &manifest, &root).unwrap();
    assert_eq!(bundle.manifest["bundle_name"], "triage");
    assert!(
        compile_from_manifest(&root, &dir.path().join("missing-bundle"), &manifest, &root).is_err()
    );

    let mut broken = manifest;
    broken["realms"]["sha256"] = json!("0".repeat(64));
    assert!(compile_from_manifest(&root, &root.join("recipes/triage"), &broken, &root).is_err());
}

#[test]
fn contention_is_found_through_the_transparent_variants_and_nothing_else_is() {
    let contended = || StoreError::Contended {
        operation: "append",
        waited_ms: 30_000,
    };
    let store = LaunchError::Store(contended());
    assert!(store.contention().unwrap().is_contention());
    let engine = LaunchError::Engine(EngineError::Store(contended()));
    assert!(engine.contention().unwrap().is_contention());
    let moved = LaunchError::Store(StoreError::HeadMoved {
        expected_seq: 4,
        found_seq: 5,
    });
    assert!(moved.contention().is_none());
    assert!(LaunchError::RealmsWithDispatch.contention().is_none());
}
