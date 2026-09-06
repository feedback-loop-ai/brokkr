//! Decision 0046 at the engine: the entry fences (rulings 1 and 6), the
//! argv and environment every site with hands is composed from under
//! each boundary (ruling 4), the spawn-time re-walk of an unboxed exec
//! dispatch, and the record — `effect/started.boundary`, the stamp
//! beside every model, the seat input's word and marker (ruling 3).

use super::tests::{bundle, checkpointing_command, driver_command, member, single_body, state};
use super::*;
use crate::agents::{HarnessHands, ResultDoor};
use crate::realms::World;
use brokkr_core::canonical::sha256_bytes;
use brokkr_protocol::hands::network_prefix;

fn candidate(provider: &str, hands_fragment: Vec<&str>, harness: HarnessHands) -> Candidate {
    let mut argv = vec![
        "{brokkr}".to_string(),
        "driver".to_string(),
        provider.to_string(),
        "--".to_string(),
        "--model".to_string(),
        "m-1".to_string(),
    ];
    argv.extend(hands_fragment.iter().map(|part| part.to_string()));
    Candidate {
        agent: "judge".into(),
        model: "m".into(),
        effort: Some("high".into()),
        provider: provider.into(),
        argv,
        hands_fragment: hands_fragment.iter().map(|part| part.to_string()).collect(),
        harness,
    }
}

fn codex_harness() -> HarnessHands {
    HarnessHands {
        gate: Some(vec![
            "--sandbox".into(),
            "read-only".into(),
            "--output-last-message".into(),
            "{result_path}".into(),
        ]),
        gate_gap: None,
        work: Some(vec!["--sandbox".into(), "workspace-write".into()]),
        work_gap: None,
        result: ResultDoor::LastMessage,
    }
}

const CODEX_FRAGMENT: [&str; 4] = [
    "--sandbox",
    "read-only",
    "-c",
    "mcp_servers.brokkr.args={hands_args_toml}",
];

fn exec_dispatch(script: &Path) -> Vec<String> {
    vec![
        "/usr/local/bin/brokkr".into(),
        "driver".into(),
        "exec".into(),
        "--".into(),
        "bash".into(),
        script.display().to_string(),
        "{prompt_file}".into(),
    ]
}

/// A workspace map at `dir/realms.json` naming `dir/work` as its one
/// realm, with or without a boundary word.
fn world(dir: &Path, boundary: Option<&str>) -> World {
    let mut realm = json!({"name": "app", "path": "work", "default_branch": "main"});
    if let Some(word) = boundary {
        realm["boundary"] = json!(word);
    }
    let map = json!({"schema": "forge.realms/v4", "realms": [realm], "journal": "forge.db"});
    let path = dir.join("realms.json");
    std::fs::write(&path, map.to_string()).unwrap();
    World::load(&path).unwrap()
}

/// A bundle compiled under `boundary` whose one seat boxes its hands:
/// the manifest pins the site under `hands` and `boundary`, as the
/// compiler writes them.
fn boxed_bundle(dir: &Path, boundary: Boundary, command: Vec<String>) -> Bundle {
    let mut bundle = bundle(dir, single_body(command));
    bundle.boundary = boundary;
    bundle.hands.insert("work".into(), HandsSpec::default());
    bundle.manifest["hands"] = json!({"work": HandsSpec::default().to_value()});
    bundle.manifest["boundary"] = json!({"work": boundary.word()});
    bundle
}

fn store_at(dir: &Path) -> Store {
    Store::open(&dir.join("forge.db")).unwrap()
}

// ───────────────────────────── realm-boundary: the engine's entry fence

#[test]
fn the_engine_starts_a_run_only_under_the_boundary_its_bundle_was_compiled_under() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let work = dir.path().join("work");
    let command = vec!["driver".to_string()];

    // A `harness` bundle against a world whose realm declares no
    // boundary: refused naming both words, and no row is written.
    let refused = Engine::start_in_world(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Harness, command.clone()),
        "f",
        Some(work.clone()),
        Some(world(dir.path(), None)),
    );
    let error = refused.err().expect("the fence refuses").to_string();
    assert!(
        error.contains("compiled under the `harness` boundary"),
        "{error}"
    );
    assert!(error.contains("resolves `namespace`"), "{error}");
    assert!(error.contains("decision 0046 ruling 1"), "{error}");
    assert!(store_at(dir.path()).list_runs().unwrap().is_empty());

    // The same bundle with no world at all: no world resolves `namespace`.
    let refused = Engine::start(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Harness, command.clone()),
        "f",
        Some(work.clone()),
    );
    assert!(matches!(
        refused.err(),
        Some(EngineError::BoundaryMismatch {
            compiled: Boundary::Harness,
            world: Boundary::Namespace
        })
    ));
    assert!(store_at(dir.path()).list_runs().unwrap().is_empty());

    // A `namespace` bundle with no world starts as today.
    let started = Engine::start(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Namespace, command.clone()),
        "f",
        Some(work.clone()),
    )
    .unwrap();
    assert_eq!(started.boundary, Boundary::Namespace);

    // A `harness` bundle with a world declaring `harness` starts, and
    // its `run/started` manifest's `boundary` map says so.
    let started = Engine::start_in_world(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Harness, command.clone()),
        "f",
        Some(work.clone()),
        Some(world(dir.path(), Some("harness"))),
    )
    .unwrap();
    assert_eq!(started.boundary, Boundary::Harness);
    let manifest = started.store.manifest(&started.run_id).unwrap();
    assert_eq!(manifest["boundary"], json!({"work": "harness"}));
    assert_eq!(manifest["realms"]["realm"], "app");

    // With no `--repo`, the operated repository is the directory the
    // engine stands in, and the world answers for it the same way.
    let cwd = std::env::current_dir().unwrap();
    let mut realm_here = json!({"name": "here", "path": cwd.display().to_string(),
        "default_branch": "main", "boundary": "open"});
    realm_here["path"] = json!(cwd.display().to_string());
    let map = json!({"schema": "forge.realms/v4", "realms": [realm_here], "journal": "forge.db"});
    let here = dir.path().join("here.json");
    std::fs::write(&here, map.to_string()).unwrap();
    let started = Engine::start_in_world(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Open, command),
        "f",
        None,
        Some(World::load(&here).unwrap()),
    )
    .unwrap();
    assert_eq!(started.boundary, Boundary::Open);
}

// ─────────────────────── boundary-availability: the engine's own fence

#[test]
fn a_boundary_this_engine_does_not_build_refuses_at_every_entry_before_any_row() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let work = dir.path().join("work");
    let command = vec!["driver".to_string()];
    let unbuilt = |boundary: Boundary| boxed_bundle(dir.path(), boundary, command.clone());

    for (boundary, slice) in [(Boundary::Seatbelt, "ii"), (Boundary::Container, "iii")] {
        let refused = Engine::start(
            store_at(dir.path()),
            unbuilt(boundary),
            "f",
            Some(work.clone()),
        );
        let error = refused
            .err()
            .expect("unbuilt boundaries refuse")
            .to_string();
        assert!(
            error.contains(&format!(
                "`{boundary}` boundary is built by slice ({slice})"
            )),
            "{error}"
        );
        assert!(error.contains("[\"work\"]"), "{error}");
        assert!(
            error.contains("a realm may declare `harness` today"),
            "{error}"
        );
        assert_eq!(unbuilt_slice(boundary), Some(slice));
    }
    for boundary in [Boundary::Namespace, Boundary::Harness, Boundary::Open] {
        assert_eq!(unbuilt_slice(boundary), None);
    }
    assert!(store_at(dir.path()).list_runs().unwrap().is_empty());

    // `resume` and `start_with_dispatch` are fenced the same way, before
    // the pinned manifest is read or the dispatch verified.
    let running = Engine::start(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Namespace, command.clone()),
        "f",
        Some(work.clone()),
    )
    .unwrap();
    let run_id = running.run_id.clone();
    drop(running);
    let refused = Engine::resume(
        store_at(dir.path()),
        unbuilt(Boundary::Seatbelt),
        &run_id,
        Some(work.clone()),
    );
    assert!(matches!(
        refused.err(),
        Some(EngineError::UnbuiltBoundary {
            boundary: Boundary::Seatbelt,
            slice: "ii",
            ..
        })
    ));
    let bound = unbuilt(Boundary::Container);
    let envelope = super::tests::dispatch(&bound);
    let refused = Engine::start_with_dispatch(
        store_at(dir.path()),
        bound,
        "f",
        Some(work.clone()),
        envelope,
    );
    assert!(matches!(
        refused.err(),
        Some(EngineError::UnbuiltBoundary {
            boundary: Boundary::Container,
            ..
        })
    ));
    assert_eq!(store_at(dir.path()).list_runs().unwrap().len(), 1);

    // A plain bundle under `seatbelt` starts: no box is asked for.
    let mut plain = bundle(dir.path(), single_body(command));
    plain.boundary = Boundary::Seatbelt;
    Engine::start_in_world(
        store_at(dir.path()),
        plain,
        "f",
        Some(work),
        Some(world(dir.path(), Some("seatbelt"))),
    )
    .unwrap();
}

// ────────────────────────────── gate-boundary-policy: argv composition

#[test]
fn compose_site_follows_the_boundary_and_the_class() {
    let workdir = Path::new("/work");
    let roots = vec![PathBuf::from("/bundle")];
    let spec = HandsSpec::default();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let codex = candidate("codex", CODEX_FRAGMENT.to_vec(), codex_harness());
    let base: Vec<String> = codex.argv[..6].to_vec();

    // A site without hands: its command untouched, under every boundary.
    for boundary in [Boundary::Namespace, Boundary::Harness, Boundary::Open] {
        let spawn = compose_site(
            built_boundary(boundary).unwrap(),
            SeatClass::Gate,
            base.clone(),
            None,
            Some(&codex),
            workdir,
            &roots,
            "/r/p.json",
            None,
        );
        assert_eq!(spawn, SiteSpawn::inherit(base.clone()));
    }

    // `namespace`: today's path token for token, for a model site and
    // for an exec dispatch, in the engine's environment.
    let boxed_model = compose_site(
        BuiltBoundary::Namespace,
        SeatClass::Gate,
        codex.argv.clone(),
        Some(&spec),
        Some(&codex),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(
        boxed_model.argv,
        hands_command(codex.argv.clone(), Some(&spec), workdir, &roots)
    );
    assert_eq!(boxed_model.env, SpawnEnv::Inherit);
    assert_eq!(boxed_model.rewalk, None);
    let exec = exec_dispatch(Path::new("/bundle/scripts/verify.sh"));
    let boxed_exec = compose_site(
        BuiltBoundary::Namespace,
        SeatClass::Gate,
        exec.clone(),
        Some(&spec),
        None,
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(
        boxed_exec.argv,
        hands_command(exec.clone(), Some(&spec), workdir, &roots)
    );
    assert_eq!(boxed_exec.argv[1], "hands");
    assert_eq!(boxed_exec.env, SpawnEnv::Inherit);

    // `harness`, a codex gate: unboxed resolution supplies the base
    // argv; the adapter's gate fragment goes in with `{result_path}` expanded, and
    // no MCP server is served; the environment stays the engine's.
    let gate = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Gate,
        base.clone(),
        Some(&spec),
        Some(&codex),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    let mut expected = base.clone();
    expected.extend(
        [
            "--sandbox",
            "read-only",
            "--output-last-message",
            "/r/p.json",
        ]
        .map(String::from),
    );
    assert_eq!(gate.argv, expected);
    assert!(!gate
        .argv
        .iter()
        .any(|part| part.contains("mcp_servers.brokkr")));
    assert!(!gate
        .argv
        .iter()
        .any(|part| part.contains("{hands_mcp_json}")));
    assert!(!gate.argv.iter().any(|part| part.contains("{result_path}")));
    assert_eq!(gate.env, SpawnEnv::Inherit);
    assert_eq!(gate.rewalk, None);

    // A codex work site under `harness`: the writable class, no server.
    let work = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Work,
        base.clone(),
        Some(&spec),
        Some(&codex),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    let mut expected = base.clone();
    expected.extend(["--sandbox", "workspace-write"].map(String::from));
    assert_eq!(work.argv, expected);

    // A fragment naming `{brokkr}` expands to this binary.
    let mut branded = codex_harness();
    branded.gate = Some(vec!["--hook".into(), "{brokkr}".into()]);
    let branded = candidate("codex", CODEX_FRAGMENT.to_vec(), branded);
    let hooked = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Gate,
        base.clone(),
        Some(&spec),
        Some(&branded),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(hooked.argv[hooked.argv.len() - 1], exe);

    // A link declaring no fragment for the class appends nothing; a
    // site with no link (nothing resolved) is left its own argv.
    let bare = candidate("silent", CODEX_FRAGMENT.to_vec(), HarnessHands::default());
    let nothing = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Gate,
        bare.argv[..6].to_vec(),
        Some(&spec),
        Some(&bare),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(nothing.argv, bare.argv[..6].to_vec());
    let unlinked = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Gate,
        base.clone(),
        Some(&spec),
        None,
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(unlinked.argv, base);

    // `open`, a work site: the base driver argv and nothing of Brokkr's,
    // in the engine's environment because the harness needs the keys.
    let open = compose_site(
        BuiltBoundary::Open,
        SeatClass::Work,
        base.clone(),
        Some(&spec),
        Some(&codex),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(open.argv, base);
    assert_eq!(open.env, SpawnEnv::Inherit);

    // A site declaring `network: false` keeps its manifest entry and
    // gets no network switch of its own in the argv.
    let quiet = HandsSpec::parse(&json!({"kind": "workspace", "network": false})).unwrap();
    let quieted = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Gate,
        base.clone(),
        Some(&quiet),
        Some(&codex),
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert!(!quieted.argv.iter().any(|part| part.contains("network")));
    assert!(!quiet.network);

    // An exec dispatch under `harness` and `open` is the compiled
    // command behind the prefix, in the fixed environment, with the
    // declaring layer marked for the re-walk — and the class changes
    // nothing (proposal D32).
    let table: BTreeMap<String, String> = [("PATH".to_string(), "/bin".to_string())].into();
    let unboxed = Unboxed {
        env: table.clone(),
        prefix: network_prefix(7, 8),
    };
    let mut expected = network_prefix(7, 8);
    expected.extend(exec.clone());
    for boundary in [Boundary::Harness, Boundary::Open] {
        let as_gate = compose_site(
            built_boundary(boundary).unwrap(),
            SeatClass::Gate,
            exec.clone(),
            Some(&spec),
            None,
            workdir,
            &roots,
            "/r/p.json",
            Some(&unboxed),
        );
        let as_work = compose_site(
            built_boundary(boundary).unwrap(),
            SeatClass::Work,
            exec.clone(),
            Some(&spec),
            None,
            workdir,
            &roots,
            "/r/p.json",
            Some(&unboxed),
        );
        assert_eq!(as_gate, as_work);
        assert_eq!(as_gate.argv, expected);
        assert_eq!(as_gate.env, SpawnEnv::Exactly(table.clone()));
        assert_eq!(as_gate.rewalk, Some(PathBuf::from("/bundle/scripts")));
    }
    // With the probe failing (no prefix) the argv is the command alone,
    // and with nothing prepared at all the environment is empty.
    let plain = compose_site(
        BuiltBoundary::Open,
        SeatClass::Gate,
        exec.clone(),
        Some(&spec),
        None,
        workdir,
        &roots,
        "/r/p.json",
        Some(&Unboxed {
            env: table.clone(),
            prefix: Vec::new(),
        }),
    );
    assert_eq!(plain.argv, exec);
    let unprepared = compose_site(
        BuiltBoundary::Open,
        SeatClass::Gate,
        exec.clone(),
        Some(&spec),
        None,
        workdir,
        &roots,
        "/r/p.json",
        None,
    );
    assert_eq!(unprepared.env, SpawnEnv::Exactly(BTreeMap::new()));
    // A dispatch no root owns marks no layer.
    let rootless = compose_site(
        BuiltBoundary::Open,
        SeatClass::Gate,
        exec.clone(),
        Some(&spec),
        None,
        workdir,
        &[],
        "/r/p.json",
        None,
    );
    assert_eq!(rootless.rewalk, None);
    assert_eq!(network_prefix_if(false, 1, 2), Vec::<String>::new());
    assert_eq!(network_prefix_if(true, 1, 2), network_prefix(1, 2));
}

#[test]
fn retiring_confine_leaves_plain_seat_member_and_step_argv_untouched() {
    let command: Vec<String> = [
        "driver",
        "--model",
        "m",
        "--effort",
        "high",
        "{prompt_file}",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let (_dir, mut engine) = super::tests::engine(single_body(command.clone()));
    for boundary in brokkr_core::realms::BOUNDARIES {
        engine.boundary = boundary;
        let single = engine.compose("attempt", false, command.clone(), None, None, "result.json");
        assert_eq!(single, SiteSpawn::inherit(command.clone()));
        // The member composer is shared by a panel and a panel step.
        for (site, prefix) in [("work", ""), ("work:check", "check:")] {
            let runs = engine.member_runs(
                "attempt",
                site,
                &[member("judge", command.clone())],
                &json!({"judge":{"role_path":"role.md","result_path":"result.json"}}),
                &json!({}),
                &json!({}),
                &Selection::new(),
                prefix,
                false,
            );
            assert_eq!(runs[0].spawn, single);
        }
    }
}

/// The dispatch `bundles/self`'s verify seat becomes under `harness` on
/// Linux with the probe passing, token for token; under `open`, on the
/// other hosts, and with the probe failing, the compiled command alone.
#[test]
fn the_unboxed_exec_dispatch_is_pinned_token_for_token() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let bundle_dir = root.join("bundles/self");
    let bundle = Bundle::compile_under(
        &bundle_dir,
        &root.join("agents"),
        &root.join("adapters"),
        Boundary::Namespace,
    )
    .unwrap();
    let SeatBody::Single { command, .. } = &bundle.seats["verify"].body else {
        panic!("the verify seat is a single exec site");
    };
    let spec = &bundle.hands["verify"];
    let (uid, gid) = brokkr_protocol::hands::ids();
    let script = bundle.dir.join("scripts/verify-seat.sh");
    let prefixed = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Gate,
        command.clone(),
        Some(spec),
        None,
        Path::new("/repo"),
        &bundle.roots,
        "/r/p.json",
        Some(&Unboxed {
            env: BTreeMap::new(),
            prefix: network_prefix(uid, gid),
        }),
    );
    let expected: Vec<String> = [
        "unshare",
        "--map-root-user",
        "--net",
        "--",
        "sh",
        "-c",
        &format!("ip link set lo up && exec unshare --map-user={uid} --map-group={gid} -- \"$@\""),
        "sh",
        &command[0],
        "driver",
        "exec",
        "--",
        "bash",
        &script.display().to_string(),
        "{prompt_file}",
    ]
    .map(String::from)
    .to_vec();
    assert_eq!(prefixed.argv, expected);
    assert_eq!(prefixed.rewalk, Some(bundle.dir.join("scripts")));
    for boundary in [Boundary::Harness, Boundary::Open] {
        let alone = compose_site(
            built_boundary(boundary).unwrap(),
            SeatClass::Gate,
            command.clone(),
            Some(spec),
            None,
            Path::new("/repo"),
            &bundle.roots,
            "/r/p.json",
            Some(&Unboxed::default()),
        );
        assert_eq!(alone.argv, *command);
    }
}

/// The probe is asked once per engine process and remembered: a second
/// dispatch of the same engine spawns no second probe.
#[test]
fn the_network_probe_is_asked_once_and_remembered() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let spec = HandsSpec::default();
    engine.network_prefix = Some(true);
    let (uid, gid) = brokkr_protocol::hands::ids();
    let first = engine.unboxed("attempt-1", &spec);
    assert_eq!(
        first.prefix,
        network_prefix_if(cfg!(target_os = "linux"), uid, gid)
    );
    assert!(Path::new(&first.env["HOME"]).ends_with(Path::new("attempt-1").join("home")));
    assert!(Path::new(&first.env["TMPDIR"]).ends_with(Path::new("attempt-1").join("tmp")));
    assert!(Path::new(&first.env["HOME"]).is_dir());
    engine.network_prefix = Some(false);
    let second = engine.unboxed("attempt-2", &spec);
    assert!(second.prefix.is_empty());
    // With no answer remembered the probe runs once, in the dispatch's
    // environment, and the answer is kept for the next dispatch.
    engine.network_prefix = None;
    let probed = engine.unboxed("attempt-3", &spec);
    let answer = engine.network_prefix;
    assert_eq!(answer.is_some(), cfg!(target_os = "linux"));
    assert_eq!(probed.prefix.is_empty(), !answer.unwrap_or(false));
    let again = engine.unboxed("attempt-4", &spec);
    assert_eq!(engine.network_prefix, answer);
    assert_eq!(again.prefix, probed.prefix);
}

// ─────────────────────── gate-boundary-policy: the spawn-time re-walk

/// A layer on disk whose pinned identity the bundle carries: the
/// bundle's own files map for the leaf, computed by the same digest.
fn pinned_layer(dir: &Path) -> (PathBuf, Bundle) {
    let layer = dir.join("layer");
    std::fs::create_dir_all(layer.join("scripts")).unwrap();
    std::fs::write(layer.join("scripts/gate.sh"), "#!/bin/sh\ntrue\n").unwrap();
    std::fs::write(layer.join("scripts/lib.sh"), "helper\n").unwrap();
    std::fs::write(layer.join("bundle.json"), "{}").unwrap();
    let mut bundle = bundle(dir, single_body(vec!["must-not-run".into()]));
    bundle.dir = layer.clone();
    bundle.roots = vec![layer.clone()];
    let mut files = Map::new();
    for rel in ["bundle.json", "scripts/gate.sh", "scripts/lib.sh"] {
        files.insert(
            rel.to_string(),
            Value::String(sha256_bytes(&std::fs::read(layer.join(rel)).unwrap())),
        );
    }
    bundle.manifest["files"] = Value::Object(files);
    (layer, bundle)
}

#[test]
fn an_unboxed_exec_dispatch_is_refused_at_spawn_when_its_layer_moved() {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        return;
    }
    let (dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let (layer, pinned) = pinned_layer(dir.path());
    engine.bundle = pinned;
    let spawn = SiteSpawn {
        argv: vec!["must-not-run".into()],
        env: SpawnEnv::Inherit,
        rewalk: Some(layer.join("scripts")),
    };
    let run = |engine: &mut Engine| {
        engine
            .run_driver(
                "effect",
                "attempt",
                "work",
                &spawn,
                json!({}),
                std::time::Duration::from_secs(1),
                None,
                None,
            )
            .unwrap()
    };
    // Untouched: the layer re-walks clean and the spawn is attempted —
    // and fails as a missing binary, which is the spawn, not the walk.
    match run(&mut engine) {
        DriverRun::SpawnFailed(error) => assert!(error.contains("did not spawn"), "{error}"),
        DriverRun::Ran(_) => panic!("must-not-run ran"),
    }
    // The script edited: refused before anything spawns, naming the
    // layer and the key.
    std::fs::write(layer.join("scripts/gate.sh"), "#!/bin/sh\nfalse\n").unwrap();
    match run(&mut engine) {
        DriverRun::SpawnFailed(error) => {
            assert!(error.contains("unboxed exec dispatch refused"), "{error}");
            assert!(error.contains("layer 'test' moved"), "{error}");
            assert!(error.contains("changed: scripts/gate.sh"), "{error}");
            assert!(error.contains("decision 0046 ruling 4"), "{error}");
        }
        DriverRun::Ran(_) => panic!("a moved layer spawned"),
    }
    std::fs::write(layer.join("scripts/gate.sh"), "#!/bin/sh\ntrue\n").unwrap();
    // A sibling the script sources, edited: the same refusal.
    std::fs::write(layer.join("scripts/lib.sh"), "changed\n").unwrap();
    match run(&mut engine) {
        DriverRun::SpawnFailed(error) => {
            assert!(error.contains("changed: scripts/lib.sh"), "{error}")
        }
        DriverRun::Ran(_) => panic!("a moved sibling spawned"),
    }
    std::fs::write(layer.join("scripts/lib.sh"), "helper\n").unwrap();
    // A pinned file deleted, then a file added.
    std::fs::remove_file(layer.join("scripts/lib.sh")).unwrap();
    match run(&mut engine) {
        DriverRun::SpawnFailed(error) => {
            assert!(error.contains("missing: scripts/lib.sh"), "{error}")
        }
        DriverRun::Ran(_) => panic!("a deleted file spawned"),
    }
    std::fs::write(layer.join("scripts/lib.sh"), "helper\n").unwrap();
    std::fs::write(layer.join("scripts/extra.sh"), "new\n").unwrap();
    match run(&mut engine) {
        DriverRun::SpawnFailed(error) => {
            assert!(error.contains("added: scripts/extra.sh"), "{error}")
        }
        DriverRun::Ran(_) => panic!("an added file spawned"),
    }
    std::fs::remove_file(layer.join("scripts/extra.sh")).unwrap();
    // A layer the bundle neither owns nor composed is no layer to walk:
    // nothing is refused on its account.
    let elsewhere = SiteSpawn {
        rewalk: Some(dir.path().join("elsewhere")),
        ..spawn.clone()
    };
    match engine
        .run_driver(
            "effect",
            "attempt",
            "work",
            &elsewhere,
            json!({}),
            std::time::Duration::from_secs(1),
            None,
            None,
        )
        .unwrap()
    {
        DriverRun::SpawnFailed(error) => assert!(error.contains("did not spawn"), "{error}"),
        DriverRun::Ran(_) => panic!("must-not-run ran"),
    }
    // A layer that cannot be walked at all — the leaf's directory gone,
    // or an ancestor's — refuses naming the walk's own error.
    let kept = engine.bundle.dir.clone();
    engine.bundle.dir = dir.path().join("vanished");
    engine.bundle.roots = vec![engine.bundle.dir.clone()];
    let vanished = SiteSpawn {
        rewalk: Some(engine.bundle.dir.clone()),
        ..spawn.clone()
    };
    match engine
        .run_driver(
            "effect",
            "attempt",
            "work",
            &vanished,
            json!({}),
            std::time::Duration::from_secs(1),
            None,
            None,
        )
        .unwrap()
    {
        DriverRun::SpawnFailed(error) => {
            assert!(error.contains("unboxed exec dispatch refused"), "{error}")
        }
        DriverRun::Ran(_) => panic!("a vanished layer spawned"),
    }
    engine.bundle.dir = kept.clone();
    engine.bundle.roots = vec![kept];
    engine.bundle.roots.push(dir.path().join("no-such-base"));
    engine.bundle.chain.push(crate::bundle::compose::Ancestor {
        name: "base".into(),
        reached_as: None,
        dir: dir.path().join("no-such-base"),
        files: Map::new(),
        digest: "a".repeat(64),
    });
    let ancestral = SiteSpawn {
        rewalk: Some(dir.path().join("no-such-base")),
        ..spawn.clone()
    };
    match engine
        .run_driver(
            "effect",
            "attempt",
            "work",
            &ancestral,
            json!({}),
            std::time::Duration::from_secs(1),
            None,
            None,
        )
        .unwrap()
    {
        DriverRun::SpawnFailed(error) => assert!(error.contains("layer 'base' moved"), "{error}"),
        DriverRun::Ran(_) => panic!("a vanished ancestor spawned"),
    }
    engine.bundle.chain.clear();
    // The refusal is journaled as the attempt's failure through the
    // ordinary conclusion.
    let spawn_failed =
        DriverRun::SpawnFailed("unboxed exec dispatch refused: layer 'test' moved".into());
    engine
        .conclude_single(
            "effect",
            "attempt",
            spawn_failed,
            &Selection::new(),
            Some(Boundary::Open),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let failed = events
        .iter()
        .find(|event| event.event_type == EventType::EffectFailed)
        .unwrap();
    assert!(failed.payload["error"]
        .as_str()
        .unwrap()
        .contains("layer 'test' moved"));
    // A `namespace` gate over the same edited layer composes as today:
    // the box is its admission, and no re-walk is marked.
    let boxed = compose_site(
        BuiltBoundary::Namespace,
        SeatClass::Gate,
        exec_dispatch(&layer.join("scripts/gate.sh")),
        Some(&HandsSpec::default()),
        None,
        dir.path(),
        std::slice::from_ref(&layer),
        "/r/p.json",
        None,
    );
    assert_eq!(boxed.rewalk, None);
    assert_eq!(boxed.argv[1], "hands");
}

// ───────────────────────────────── boundary-record: effect/started

#[test]
fn effect_started_carries_the_boundary_beside_provenance() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    // A plain bundle: no site has hands, no key.
    let body = single_body(vec!["driver".into()]);
    let (executable, _) = body.selected(None).unwrap();
    assert_eq!(engine.boundary_entries(executable, "work", true), None);

    // A gate-class boxed single seat under `harness`: one entry.
    engine.boundary = Boundary::Harness;
    engine
        .bundle
        .hands
        .insert("work".into(), HandsSpec::default());
    assert_eq!(
        engine.boundary_entries(executable, "work", true),
        Some(json!([{"member": null, "boundary": "harness", "gate": true}]))
    );
    // The same site as a work seat.
    assert_eq!(
        engine.boundary_entries(executable, "work", false),
        Some(json!([{"member": null, "boundary": "harness", "gate": false}]))
    );

    // A sequence of a hands-less author step and a boxed dialect
    // validate step under `namespace`: the author `not applicable`, the
    // validate `namespace` with `gate` true, and a step panel's members
    // read their step's class.
    engine.boundary = Boundary::Namespace;
    engine.bundle.hands.clear();
    engine
        .bundle
        .hands
        .insert("design:validate".into(), HandsSpec::default());
    engine
        .bundle
        .hands
        .insert("design:review:left".into(), HandsSpec::default());
    let steps = vec![
        SequenceStep {
            name: "author".into(),
            class: SeatClass::Work,
            results: vec!["drafted".into()],
            body: StepBody::Single {
                role_path: "role.md".into(),
                command: vec!["driver".into()],
                candidates: Vec::new(),
            },
        },
        SequenceStep {
            name: "validate".into(),
            class: SeatClass::Gate,
            results: vec!["pass".into()],
            body: StepBody::Dialect {
                execution: crate::bundle::DialectExecution {
                    argv: vec!["validator".into()],
                    state: None,
                },
            },
        },
        SequenceStep {
            name: "review".into(),
            class: SeatClass::Gate,
            results: vec!["clean".into()],
            body: StepBody::Panel {
                members: vec![
                    member("left", vec!["driver".into()]),
                    member("right", vec!["driver".into()]),
                ],
                aggregate: Aggregate::UnanimousPass,
            },
        },
    ];
    let sequence = SeatBody::Sequence { steps };
    let (executable, _) = sequence.selected(None).unwrap();
    assert_eq!(
        engine.boundary_entries(executable, "design", false),
        Some(json!([
            {"member": "author", "boundary": "not applicable", "gate": false},
            {"member": "validate", "boundary": "namespace", "gate": true},
            {"member": "review:left", "boundary": "namespace", "gate": true},
            {"member": "review:right", "boundary": "not applicable", "gate": true},
        ]))
    );

    // A panel seat's members take the seat's class.
    let panel = SeatBody::Panel {
        members: vec![member("a", vec!["driver".into()])],
        aggregate: Aggregate::UnanimousPass,
    };
    engine
        .bundle
        .hands
        .insert("review:a".into(), HandsSpec::default());
    let (executable, _) = panel.selected(None).unwrap();
    assert_eq!(
        engine.boundary_entries(executable, "review", true),
        Some(json!([{"member": "a", "boundary": "namespace", "gate": true}]))
    );

    // Through `execute`: the payload carries the entries beside the
    // driver label, and folding the journal is blind to them.
    let (_dir, mut driven) = super::tests::engine(single_body(driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result": "complete"}),
        },
    )));
    driven.boundary = Boundary::Open;
    driven
        .bundle
        .hands
        .insert("work".into(), HandsSpec::default());
    let requested = super::tests::requested(&driven, "effect");
    driven
        .execute(
            std::slice::from_ref(&requested),
            &state(Some("work"), Cursor::Idle),
            "effect",
            "work",
        )
        .unwrap();
    let events = driven.store.load(&driven.run_id).unwrap();
    let started = events
        .iter()
        .find(|event| event.event_type == EventType::EffectStarted)
        .unwrap();
    assert_eq!(
        started.payload["boundary"],
        json!([{"member": null, "boundary": "open", "gate": false}])
    );
    // `fold` never reads the field: a journal with it folds to the state
    // the same journal without it folds to.
    let mut carrying = vec![
        super::tests::event(
            EventType::RunStarted,
            json!({"feature": "f", "manifest": {"hands": {"work": {}}, "boundary": {"work": "open"}}}),
        ),
        super::tests::event(EventType::PhaseEntered, json!({"phase": "work"})),
        super::tests::event(
            EventType::EffectRequested,
            json!({"effect_id": "e", "phase": "work", "seat": "work",
                   "idempotency_key": "k", "input_digest": "d"}),
        ),
        super::tests::event(EventType::EffectStarted, started.payload.clone()),
        super::tests::event(
            EventType::EffectSucceeded,
            json!({"effect_id": "e", "attempt_id": "a", "result": {"result": "complete", "model": "m", "boundary": "open"}}),
        ),
    ];
    carrying[3].payload["effect_id"] = json!("e");
    for (index, envelope) in carrying.iter_mut().enumerate() {
        envelope.seq = index as u64 + 1;
    }
    let mut stripped = carrying.clone();
    for envelope in &mut stripped {
        if let Some(payload) = envelope.payload.as_object_mut() {
            payload.remove("boundary");
        }
    }
    assert!(carrying[3].payload.get("boundary").is_some());
    assert!(stripped[3].payload.get("boundary").is_none());
    assert_eq!(
        format!("{:?}", fold(&carrying).unwrap()),
        format!("{:?}", fold(&stripped).unwrap())
    );
}

// ─────────────────────────────── boundary-record: the stamp beside model

#[test]
fn the_stamp_rides_beside_the_model_and_replaces_a_drivers_word() {
    // The rule itself.
    assert_eq!(
        stamp_boundary(
            json!({"step": "x", "model": "m"}),
            Some(Boundary::Namespace)
        ),
        json!({"step": "x", "model": "m", "boundary": "namespace"})
    );
    assert_eq!(
        stamp_boundary(json!({"step": "x", "model": "m", "boundary": "open"}), None),
        json!({"step": "x", "model": "m", "boundary": "not applicable"})
    );
    assert_eq!(
        stamp_boundary(
            json!({"step": "x", "boundary": "open"}),
            Some(Boundary::Harness)
        ),
        json!({"step": "x"})
    );
    assert_eq!(
        stamp_boundary(json!("prose"), Some(Boundary::Open)),
        json!("prose")
    );
    assert_eq!(site_boundary_of(&HandsSpec::default()), Some(()));

    // Through the pass-through every driver record takes: a per-turn
    // checkpoint naming a model carries the engine's word, one naming
    // none is appended without the driver's, and the successful result
    // carries the word beside its model.
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine
        .bundle
        .hands
        .insert("work".into(), HandsSpec::default());
    let command = checkpointing_command(
        "effect",
        "attempt",
        &[
            json!({"step": "seat-turn", "turn": 1, "model": "m-1", "boundary": "open"}),
            json!({"step": "seat-turn", "turn": 2, "boundary": "open"}),
            json!({"step": "exec-session-finished", "model": "not applicable"}),
        ],
        AttemptOutcome::Succeeded {
            result: json!({"result": "complete", "model": "m-1", "boundary": "chroot"}),
        },
    );
    let run = engine
        .run_driver(
            "effect",
            "attempt",
            "work",
            &SiteSpawn::inherit(command),
            json!({}),
            std::time::Duration::from_secs(5),
            None,
            None,
        )
        .unwrap();
    engine
        .conclude_single(
            "effect",
            "attempt",
            run,
            &Selection::new(),
            engine.site_boundary("work"),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let checkpoints: Vec<&Value> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| &event.payload["checkpoint"])
        .collect();
    assert_eq!(checkpoints[0]["boundary"], "namespace");
    assert!(checkpoints[1].get("boundary").is_none());
    assert_eq!(checkpoints[2]["boundary"], "namespace");
    let result = &events
        .iter()
        .find(|event| event.event_type == EventType::EffectSucceeded)
        .unwrap()
        .payload["result"];
    assert_eq!(result["boundary"], "namespace");
    assert_eq!(result["model"], "m-1");

    // A site without hands stamps the sentinel.
    let (_dir, mut plain) = super::tests::engine(single_body(vec!["driver".into()]));
    let command = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Succeeded {
            result: json!({"result": "complete", "model": "not applicable"}),
        },
    );
    let run = plain
        .run_driver(
            "effect",
            "attempt",
            "work",
            &SiteSpawn::inherit(command),
            json!({}),
            std::time::Duration::from_secs(5),
            None,
            None,
        )
        .unwrap();
    plain
        .conclude_single(
            "effect",
            "attempt",
            run,
            &Selection::new(),
            plain.site_boundary("work"),
        )
        .unwrap();
    let events = plain.store.load(&plain.run_id).unwrap();
    let result = &events
        .iter()
        .find(|event| event.event_type == EventType::EffectSucceeded)
        .unwrap()
        .payload["result"];
    assert_eq!(result["boundary"], "not applicable");
}

/// The unit the stamp's site half reads: a site with hands has a word.
fn site_boundary_of(spec: &HandsSpec) -> Option<()> {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    assert_eq!(engine.site_boundary("work"), None);
    engine.bundle.hands.insert("work".into(), spec.clone());
    engine.boundary = Boundary::Open;
    assert_eq!(engine.site_boundary("work"), Some(Boundary::Open));
    Some(())
}

#[test]
fn a_panels_members_and_a_sequences_steps_carry_their_own_word() {
    // Panel members under `harness`: each member's checkpoints and the
    // engine's own `panel-member-finished` marker carry the member's
    // word; a member without hands carries the sentinel; the panel's
    // aggregate carries none.
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Harness;
    engine
        .bundle
        .hands
        .insert("work:boxed".into(), HandsSpec::default());
    let pass = |member: &str| {
        checkpointing_command(
            "effect",
            "attempt",
            &[json!({"step": "seat-turn", "turn": 1, "model": format!("m-{member}")})],
            AttemptOutcome::Succeeded {
                result: json!({"result": "pass", "model": format!("m-{member}")}),
            },
        )
    };
    let members = [member("boxed", pass("boxed")), member("bare", pass("bare"))];
    let input = json!({
        "feature": "f", "phase": "work", "workdir": "/w", "allowed_results": ["pass"],
        "members": {
            "boxed": {"role_path": "r.md", "result_path": "/r/boxed.json"},
            "bare": {"role_path": "r.md", "result_path": "/r/bare.json"},
        },
        "context": {},
    });
    engine
        .execute_panel(
            "effect",
            "attempt",
            "work",
            &members,
            Aggregate::UnanimousPass,
            &input,
            std::time::Duration::from_secs(5),
            &Selection::new(),
            true,
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let word_of = |member: &str, step: &str| -> Value {
        events
            .iter()
            .filter(|event| event.event_type == EventType::EffectCheckpointed)
            .map(|event| &event.payload["checkpoint"])
            .find(|checkpoint| checkpoint["member"] == member && checkpoint["step"] == step)
            .unwrap_or_else(|| panic!("no {step} for {member}"))["boundary"]
            .clone()
    };
    assert_eq!(word_of("boxed", "seat-turn"), "harness");
    assert_eq!(word_of("boxed", "panel-member-finished"), "harness");
    assert_eq!(word_of("bare", "seat-turn"), "not applicable");
    assert_eq!(word_of("bare", "panel-member-finished"), "not applicable");
    let aggregate = &events
        .iter()
        .find(|event| event.event_type == EventType::EffectSucceeded)
        .unwrap()
        .payload["result"];
    assert!(aggregate.get("boundary").is_none(), "{aggregate}");
    assert!(aggregate.get("model").is_none(), "{aggregate}");

    // A sequence whose ending step is boxed under `namespace`: the
    // ending result and the `sequence-step-finished` marker of the
    // first step carry each step's word.
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine
        .bundle
        .hands
        .insert("work:second".into(), HandsSpec::default());
    let step = |name: &str, results: Vec<&str>, result: &str| SequenceStep {
        name: name.into(),
        class: SeatClass::Gate,
        results: results.iter().map(|r| r.to_string()).collect(),
        body: StepBody::Single {
            role_path: "role.md".into(),
            command: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result": result, "model": format!("m-{name}")}),
                },
            ),
            candidates: Vec::new(),
        },
    };
    let steps = vec![
        step("first", vec!["done"], "done"),
        step("second", vec!["complete"], "complete"),
    ];
    let input = json!({
        "feature": "f", "phase": "work", "workdir": "/w", "allowed_results": ["complete"],
        "steps": [
            {"name": "first", "allowed_results": ["done"], "role_path": "r.md", "result_path": "/r/1.json"},
            {"name": "second", "allowed_results": ["complete"], "role_path": "r.md", "result_path": "/r/2.json"},
        ],
        "context": {},
    });
    engine
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &steps,
            &input,
            std::time::Duration::from_secs(5),
            &Selection::new(),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let marker = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectCheckpointed)
        .map(|event| &event.payload["checkpoint"])
        .find(|checkpoint| checkpoint["step"] == "sequence-step-finished")
        .unwrap();
    assert_eq!(marker["step_name"], "first");
    assert_eq!(marker["boundary"], "not applicable");
    let result = &events
        .iter()
        .find(|event| event.event_type == EventType::EffectSucceeded)
        .unwrap()
        .payload["result"];
    assert_eq!(result["model"], "m-second");
    assert_eq!(result["boundary"], "namespace");
}

// ───────────────────────────── boundary-record: the seat input's word

#[test]
fn the_seat_input_names_the_boundary_and_the_marker_only_under_a_box() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let codex = candidate("codex", CODEX_FRAGMENT.to_vec(), codex_harness());
    let mut file_door = codex_harness();
    file_door.result = ResultDoor::File;
    let filed = candidate("codex", CODEX_FRAGMENT.to_vec(), file_door);

    // No hands: neither field, under any boundary.
    for boundary in brokkr_core::realms::BOUNDARIES {
        engine.boundary = boundary;
        let mut input = json!({});
        engine.mark_hands("work", &mut input);
        engine.mark_delivery("work", true, Some(&codex), &mut input);
        assert_eq!(input, json!({}));
    }
    engine
        .bundle
        .hands
        .insert("work".into(), HandsSpec::default());
    for boundary in [Boundary::Namespace, Boundary::Seatbelt, Boundary::Container] {
        engine.boundary = boundary;
        let mut input = json!({});
        engine.mark_hands("work", &mut input);
        engine.mark_delivery("work", true, Some(&codex), &mut input);
        assert_eq!(
            input,
            json!({"hands": "boxed", "boundary": boundary.word()})
        );
    }
    // `harness`: the word, no marker; the door only for a gate whose
    // link captures the final message.
    engine.boundary = Boundary::Harness;
    let mut input = json!({});
    engine.mark_hands("work", &mut input);
    engine.mark_delivery("work", true, Some(&codex), &mut input);
    assert_eq!(
        input,
        json!({"boundary": "harness", "result_delivery": "last-message"})
    );
    let mut input = json!({});
    engine.mark_hands("work", &mut input);
    engine.mark_delivery("work", true, Some(&filed), &mut input);
    assert_eq!(input, json!({"boundary": "harness"}));
    let mut input = json!({});
    engine.mark_hands("work", &mut input);
    engine.mark_delivery("work", false, Some(&codex), &mut input);
    assert_eq!(input, json!({"boundary": "harness"}));
    let mut input = json!({});
    engine.mark_hands("work", &mut input);
    engine.mark_delivery("work", true, None, &mut input);
    assert_eq!(input, json!({"boundary": "harness"}));
    // `open`: the word and nothing else.
    engine.boundary = Boundary::Open;
    let mut input = json!({});
    engine.mark_hands("work", &mut input);
    engine.mark_delivery("work", true, Some(&codex), &mut input);
    assert_eq!(input, json!({"boundary": "open"}));

    // The requested input carries the word through `seat_input`, and a
    // panel member's derived input through `member_runs`.
    engine.boundary = Boundary::Harness;
    let seat = engine
        .seat_input(&state(Some("work"), Cursor::Idle), "work", "effect")
        .unwrap();
    assert_eq!(seat["boundary"], "harness");
    assert!(seat.get("hands").is_none());
    engine
        .bundle
        .hands
        .insert("review:left".into(), HandsSpec::default());
    let members = vec![
        member("left", vec!["driver".into()]),
        member("right", vec!["driver".into()]),
    ];
    let meta = json!({
        "left": {"role_path": "l.md", "result_path": "/r/l.json"},
        "right": {"role_path": "r.md", "result_path": "/r/r.json"},
    });
    let seat_input = json!({
        "feature": "f", "phase": "review", "workdir": "/w",
        "allowed_results": ["clean"], "house_rules": Value::Null, "spec_dialect": Value::Null,
    });
    let runs = engine.member_runs(
        "attempt",
        "review",
        &members,
        &meta,
        &seat_input,
        &json!({}),
        &Selection::default(),
        "",
        true,
    );
    assert_eq!(runs[0].input["boundary"], "harness");
    assert_eq!(runs[0].boundary, Some(Boundary::Harness));
    assert!(runs[1].input.get("boundary").is_none());
    assert_eq!(runs[1].boundary, None);
}

// ─────────────────────── boundary-manifest-pin: resume names the word

#[test]
fn a_resume_under_another_word_is_refused_naming_boundary() {
    let pinned = json!({"files": {}, "hands": {"work": {}}, "boundary": {"work": "namespace"}});
    let current = json!({"files": {}, "hands": {"work": {}}, "boundary": {"work": "harness"}});
    let diff = manifest_diff(&pinned, &current);
    assert!(diff.contains("boundary differs"), "{diff}");
    assert!(diff.contains("{\"work\":\"namespace\"}"), "{diff}");
    assert!(diff.contains("{\"work\":\"harness\"}"), "{diff}");
    let old = json!({"files": {}, "hands": {"work": {}}});
    let diff = manifest_diff(&old, &current);
    assert!(diff.contains("the run pinned no boundary"), "{diff}");
    let diff = manifest_diff(&current, &old);
    assert!(
        diff.contains("the bundle compiled under no boundary"),
        "{diff}"
    );
    assert_eq!(
        manifest_diff(
            &json!({"files": {}, "engine": "1"}),
            &json!({"files": {}, "engine": "2"})
        ),
        "non-file manifest fields differ (engine or contract version)"
    );

    // Through `Engine::resume`: a run started under `namespace` handed
    // a bundle compiled under `harness` refuses with a diff naming it.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("work")).unwrap();
    let work = dir.path().join("work");
    let started = Engine::start(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Namespace, vec!["driver".into()]),
        "f",
        Some(work.clone()),
    )
    .unwrap();
    let run_id = started.run_id.clone();
    drop(started);
    let refused = Engine::resume(
        store_at(dir.path()),
        boxed_bundle(dir.path(), Boundary::Harness, vec!["driver".into()]),
        &run_id,
        Some(work),
    );
    let error = refused.err().expect("the word moved").to_string();
    assert!(error.contains("boundary differs"), "{error}");
}

// ────────────────────────────── the judge's door: the last message

/// Under `harness` with a `last-message` door the seat's final message
/// reaches the engine as the result file the harness writes; a final
/// message that is not the bare object is a missing result exactly as a
/// malformed file is. The driver reads the file as today under both
/// doors, which the codex driver's own tests prove; here the composed
/// argv names the path and the input names the door.
#[test]
fn a_harness_gate_on_a_last_message_door_names_its_result_path() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Harness;
    engine
        .bundle
        .hands
        .insert("work".into(), HandsSpec::default());
    let codex = candidate("codex", CODEX_FRAGMENT.to_vec(), codex_harness());
    let spawn = engine.compose(
        "attempt",
        true,
        codex.argv.clone(),
        Some(&HandsSpec::default()),
        Some(&codex),
        "/r/p.json",
    );
    assert_eq!(
        &spawn.argv[spawn.argv.len() - 2..],
        ["--output-last-message", "/r/p.json"]
    );
    assert_eq!(spawn.env, SpawnEnv::Inherit);
    let mut input = json!({"result_path": "/r/p.json"});
    engine.mark_hands("work", &mut input);
    engine.mark_delivery("work", true, Some(&codex), &mut input);
    assert_eq!(input["result_delivery"], "last-message");
}

#[test]
fn every_panel_spawn_rechecks_its_layer_and_journals_a_moved_member_failure() {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        return;
    }
    let (dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let (layer, pinned) = pinned_layer(dir.path());
    engine.bundle = pinned;
    let runs = [MemberRun {
        name: "judge".into(),
        driver_seat: "work:judge".into(),
        boundary: Some(Boundary::Harness),
        spawn: SiteSpawn {
            argv: driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"pass"}),
                },
            ),
            env: SpawnEnv::Inherit,
            rewalk: Some(layer.join("scripts")),
        },
        input: json!({}),
    }];
    let deadline = std::time::Duration::from_secs(5);
    let clean = engine
        .run_panel("effect", "attempt", &runs, deadline, "")
        .unwrap();
    assert!(clean[0].1.accepted);
    assert!(matches!(
        clean[0].1.outcome,
        AttemptOutcome::Succeeded { .. }
    ));
    for key in ["scripts/gate.sh", "scripts/lib.sh"] {
        let path = layer.join(key);
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, "moved\n").unwrap();
        for prefix in ["", "step:"] {
            let reports = engine
                .run_panel("effect", "attempt", &runs, deadline, prefix)
                .unwrap();
            assert!(!reports[0].1.accepted, "nothing spawned");
            let AttemptOutcome::Failed { error } = &reports[0].1.outcome else {
                panic!("moved member ran")
            };
            assert!(error.contains(&format!("changed: {key}")), "{error}");
            let outcome = panel_outcome(Aggregate::UnanimousPass, reports);
            engine
                .conclude_single(
                    "effect",
                    "attempt",
                    DriverRun::Ran(super::tests::report(outcome, "")),
                    &Selection::new(),
                    None,
                )
                .unwrap();
            assert!(engine
                .store
                .load(&engine.run_id)
                .unwrap()
                .iter()
                .any(|event| event.event_type == EventType::EffectFailed
                    && event.payload["error"]
                        .as_str()
                        .is_some_and(|error| error.contains(key))));
        }
        std::fs::write(path, original).unwrap();
    }
}

#[test]
fn emitted_boundary_entries_validate_and_plain_started_payloads_keep_their_shape() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../contracts/effect-boundary.v1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::draft7::new(&schema).unwrap();
    let body = single_body(vec!["driver".into()]);
    let (executable, _) = body.selected(None).unwrap();
    assert!(engine.boundary_entries(executable, "work", true).is_none());
    assert!(validator.is_valid(&json!({})));
    for word in brokkr_core::realms::BOUNDARIES {
        engine.boundary = word;
        engine
            .bundle
            .hands
            .insert("work".into(), HandsSpec::default());
        let entries = engine.boundary_entries(executable, "work", true).unwrap();
        assert!(validator.is_valid(&json!({"boundary":entries})));
    }
    for entries in [
        json!([]),
        json!([{"member":null,"boundary":"chroot","gate":true}]),
        json!([{"boundary":"harness","gate":true}]),
    ] {
        assert!(!validator.is_valid(&json!({"boundary":entries})));
    }
}

#[test]
fn an_inherited_dispatch_rewalks_its_script_layer_even_when_an_argument_names_the_leaf() {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        return;
    }
    let (dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let (layer, pinned) = pinned_layer(dir.path());
    let leaf = dir.path().join("child");
    engine.bundle.chain.push(crate::bundle::compose::Ancestor {
        name: pinned.name.clone(),
        reached_as: None,
        dir: layer.clone(),
        digest: pinned.manifest_digest(),
        files: pinned.manifest["files"].as_object().unwrap().clone(),
    });
    engine.bundle.dir = leaf.clone();
    engine.bundle.roots = vec![leaf.clone(), layer.clone()];
    let mut command = exec_dispatch(&layer.join("scripts/gate.sh"));
    command.push(leaf.join("result.json").display().to_string());
    assert_eq!(
        script_directory(&command, &engine.bundle.roots),
        Some(layer.join("scripts"))
    );
    let spawn = SiteSpawn {
        argv: driver_command(
            "effect",
            "attempt",
            AttemptOutcome::Succeeded {
                result: json!({"result":"complete"}),
            },
        ),
        env: SpawnEnv::Inherit,
        rewalk: script_directory(&command, &engine.bundle.roots),
    };
    let run = |engine: &mut Engine| {
        engine
            .run_driver(
                "effect",
                "attempt",
                "work",
                &spawn,
                json!({}),
                std::time::Duration::from_secs(5),
                None,
                None,
            )
            .unwrap()
    };
    let DriverRun::Ran(clean) = run(&mut engine) else {
        panic!("unchanged ancestor refused")
    };
    assert!(clean.accepted);
    std::fs::write(layer.join("scripts/lib.sh"), "changed\n").unwrap();
    let moved = run(&mut engine);
    let DriverRun::SpawnFailed(error) = &moved else {
        panic!("changed ancestor ran")
    };
    assert!(error.contains("changed: scripts/lib.sh"), "{error}");
    engine
        .conclude_single("effect", "attempt", moved, &Selection::new(), None)
        .unwrap();
    assert!(engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == EventType::EffectFailed
            && event.payload["error"]
                .as_str()
                .unwrap()
                .contains("changed: scripts/lib.sh")));
}

#[test]
fn an_invalid_boundary_record_fails_at_append_without_writing_the_result() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine
        .append_succeeded(
            "effect",
            "attempt",
            json!({"result":"complete", "model":"m", "boundary":"chroot"}),
            |refusal| json!({"effect_id":"effect", "attempt_id":"attempt", "error":refusal}),
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    assert!(!events
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded));
    let failed = events.last().unwrap();
    assert_eq!(failed.event_type, EventType::EffectFailed);
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("seat-record.v4"),
        "{failed:?}"
    );
    assert!(
        !failed.payload["error"].as_str().unwrap().contains("chroot"),
        "invalid values stay out of diagnostics"
    );
    assert!(failed.payload.get("result").is_none());
}

#[test]
fn a_sequence_panel_cannot_continue_when_its_member_marker_cannot_be_journaled() {
    let (dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let steps = [SequenceStep {
        name: "panel".into(),
        class: SeatClass::Work,
        results: vec!["complete".into()],
        body: StepBody::Panel {
            aggregate: Aggregate::UnanimousPass,
            members: vec![member(
                "judge",
                driver_command(
                    "effect",
                    "attempt",
                    AttemptOutcome::Succeeded {
                        result: json!({"result":"pass"}),
                    },
                ),
            )],
        },
    }];
    let input = json!({"workdir":".", "allowed_results":["complete"], "context":{},
        "steps":[{"name":"panel", "members":{"judge":{"role_path":"role.md", "result_path":"result.json"}}}]});
    super::tests::fail_event(&dir.path().join("forge.db"), "panel-member-finished");
    let error = engine
        .execute_sequence(
            "effect",
            "attempt",
            "work",
            &steps,
            &input,
            std::time::Duration::from_secs(5),
            &Selection::new(),
        )
        .unwrap_err();
    assert!(matches!(error, EngineError::Store(_)), "{error}");
    assert!(!engine
        .store
        .load(&engine.run_id)
        .unwrap()
        .iter()
        .any(|event| event.event_type == EventType::EffectSucceeded));
}

#[test]
fn a_plain_attempt_emits_the_original_started_payload() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["missing-driver".into()]));
    let requested = super::tests::requested(&engine, "effect");
    engine
        .execute(
            &[requested],
            &state(Some("work"), Cursor::Idle),
            "effect",
            "work",
        )
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let started = events
        .iter()
        .find(|event| event.event_type == EventType::EffectStarted)
        .unwrap();
    let expected = json!({"effect_id":"effect", "attempt_id":started.payload["attempt_id"], "driver":"missing-driver"});
    assert_eq!(
        serde_json::to_vec(&started.payload).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
}

#[test]
fn the_shipped_verify_input_and_prompt_name_no_workspace_tool_under_any_built_boundary() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiled = Bundle::compile_with(
        &root.join("bundles/self"),
        &root.join("agents"),
        &root.join("adapters"),
    )
    .unwrap();
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine.bundle = compiled;
    for boundary in [Boundary::Namespace, Boundary::Harness, Boundary::Open] {
        engine.boundary = boundary;
        let input = engine
            .seat_input(&state(Some("verify"), Cursor::Idle), "verify", "effect")
            .unwrap();
        assert_eq!(input["boundary"], boundary.word());
        assert_eq!(
            input.get("hands").is_some(),
            boundary == Boundary::Namespace
        );
        let prompt = brokkr_protocol::adapters::render_prompt(
            &input,
            brokkr_protocol::adapters::AdapterKind::Exec,
        );
        assert!(!prompt.contains("mcp__brokkr__workspace"));
        assert!(!prompt.contains("Your hands"));
        assert!(prompt.contains(input["result_path"].as_str().unwrap()));
    }
}
