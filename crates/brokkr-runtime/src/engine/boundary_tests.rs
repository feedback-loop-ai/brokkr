//! Decision 0046 at the engine: the entry fences (rulings 1 and 6), the
//! argv and environment every site with hands is composed from under
//! each boundary (ruling 4), the spawn-time re-walk of an unboxed exec
//! dispatch, and the record — `effect/started.boundary`, the stamp
//! beside every model, the seat input's word and marker (ruling 3).

use super::tests::{bundle, checkpointing_command, driver_command, member, single_body, state};
use super::*;
use crate::agents::{Adapters, Availability, HarnessHands, Library, ResultDoor};
use crate::bundle::{HandsState, SiteFacts};
use crate::realms::World;
use brokkr_core::canonical::sha256_bytes;
use brokkr_protocol::hands::network_prefix;

fn candidate(provider: &str, hands_fragment: Vec<&str>, harness: HarnessHands) -> Candidate {
    let template: Vec<String> = ["{brokkr}", "driver", provider, "--", "--model", "m-1"]
        .iter()
        .map(|part| part.to_string())
        .collect();
    let hands_fragment: Vec<String> = hands_fragment.iter().map(|part| part.to_string()).collect();
    // The segments the resolver would have carried: the adapter's template,
    // then the workspace fragment it appended as hands.
    let mut segments = vec![Segment::new(Origin::Template, &template)];
    if !hands_fragment.is_empty() {
        segments.push(Segment::new(Origin::Hands, &hands_fragment));
    }
    let hands = match hands_fragment.is_empty() {
        true => HandsIntent::None,
        false => HandsIntent::Required,
    };
    Candidate {
        agent: "judge".into(),
        model: "m".into(),
        effort: Some("high".into()),
        provider: provider.into(),
        argv: flatten(&segments),
        hands_fragment,
        harness,
        resume: Default::default(),
        lowering: Lowering::Composed(crate::agents::Composition {
            template: crate::agents::declared_template(&template),
            segments,
            effort: Some("high".into()),
            intent: crate::agents::Intent {
                allow: AllowIntent::Unspecified,
                sandbox: SandboxIntent::Unspecified,
                hands,
            },
            application: Application::Unrestricted,
            serving: Default::default(),
        }),
        hands_notice: None,
    }
}

/// The seat input for `work` after `mark_hands` and the result-door mark,
/// under the engine's current boundary.
fn marked(engine: &Engine, gate: bool, door: Option<&Candidate>) -> Value {
    let mut input = json!({});
    engine.mark_hands("work", &mut input);
    engine.marks().door("work", gate, door, &mut input);
    input
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
    bundle.sites.insert(
        "work".into(),
        SiteFacts {
            hands: HandsState::Hands(HandsSpec::default()),
            ..Default::default()
        },
    );
    bundle.manifest["hands"] = json!({"work": HandsSpec::default().to_value()});
    bundle.manifest["boundary"] = json!({"work": boundary.word()});
    bundle
}

/// The same bundle, compiled in `realm` and granted nothing: the start
/// fence reads the realm a bundle was resolved in beside the boundary it
/// was compiled under (decision 0065 ruling 3).
fn compiled_in(realm: &str, mut bundle: Bundle) -> Bundle {
    bundle.manifest["capabilities"] = json!({"realm": realm, "grants": {}});
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
        compiled_in(
            "app",
            boxed_bundle(dir.path(), Boundary::Harness, command.clone()),
        ),
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
        compiled_in("here", boxed_bundle(dir.path(), Boundary::Open, command)),
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
        compiled_in("app", plain),
        "f",
        Some(work),
        Some(world(dir.path(), Some("seatbelt"))),
    )
    .unwrap();
}

// ────────────────────────────── gate-boundary-policy: argv composition

/// Unit 4 (design D5.7): every boundary arm carries the selected link's
/// segments through its own composition — `hands_command`'s placeholder
/// expansion, the box's exec prefix, the harness fragment, the network
/// prefix — and labels only what it composed itself as `hands`. An inline
/// command that spells exactly those bytes stays the author's, and a
/// command that is not the link's own composition is refused.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn every_boundary_arm_carries_the_links_segments_and_labels_its_own_as_hands() {
    let workdir = Path::new("/work");
    let roots = vec![PathBuf::from("/bundle")];
    let spec = HandsSpec::default();
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let compose = |boundary, command: Vec<String>, link: Option<&Candidate>| {
        compose_site(
            boundary,
            SeatClass::Gate,
            command,
            Some(&spec),
            link,
            workdir,
            &roots,
            "/r/p.json",
            Some(&Unboxed {
                env: BTreeMap::new(),
                prefix: strings(&["unshare", "--net", "--"]),
            }),
        )
    };
    let origins = |spawn: &SiteSpawn| {
        spawn
            .segments
            .iter()
            .map(|segment| segment.origin)
            .collect::<Vec<_>>()
    };

    // The default workspace spec as the box and the server are told it.
    let spec_json = r#"{"binds":[],"kind":"workspace","network":false}"#;

    // `namespace`, a boxed link: the template and its hands, each over its
    // own expanded tokens — the MCP server's arguments spelled in full.
    let boxed = candidate("codex", CODEX_FRAGMENT.to_vec(), codex_harness());
    let spawn = compose(BuiltBoundary::Namespace, boxed.argv.clone(), Some(&boxed));
    assert_eq!(
        spawn.segments,
        [
            Segment::new(
                Origin::Template,
                &strings(&[&exe, "driver", "codex", "--", "--model", "m-1"])
            ),
            Segment::new(
                Origin::Hands,
                &strings(&[
                    "--sandbox",
                    "read-only",
                    "-c",
                    r#"mcp_servers.brokkr.args=["hands","serve","--workdir","/work","--spec","{\"binds\":[],\"kind\":\"workspace\",\"network\":false}"]"#,
                ])
            ),
        ]
    );
    assert_eq!(spawn.refusal, None);

    // `namespace`, an inline exec dispatch: the box's own prefix is the
    // engine's, the dispatched command stays the author's — over its own
    // tokens, the script mapped into the box and every other token as
    // written.
    let exec = exec_dispatch(Path::new("/bundle/scripts/verify.sh"));
    let spawn = compose(BuiltBoundary::Namespace, exec.clone(), None);
    assert_eq!(
        spawn.segments,
        [
            Segment::new(
                Origin::Hands,
                &strings(&[
                    &exe,
                    "hands",
                    "exec",
                    "--workdir",
                    "/work",
                    "--spec",
                    spec_json,
                    "--bundle-root",
                    "/bundle",
                    "--",
                ])
            ),
            Segment::new(
                Origin::Authored,
                &strings(&[
                    "/usr/local/bin/brokkr",
                    "driver",
                    "exec",
                    "--",
                    "bash",
                    "/runtime/bundle/scripts/verify.sh",
                    "{prompt_file}",
                ])
            ),
        ]
    );

    // `harness`, the unboxed link: its template, then the gate fragment the
    // engine appended, expanded — the legacy pair's managed half.
    let unboxed = candidate("codex", Vec::new(), codex_harness());
    let gate = compose(BuiltBoundary::Harness, unboxed.argv.clone(), Some(&unboxed));
    assert_eq!(
        gate.segments,
        [
            Segment::new(
                Origin::Template,
                &strings(&["{brokkr}", "driver", "codex", "--", "--model", "m-1"])
            ),
            Segment::new(
                Origin::Hands,
                &strings(&[
                    "--sandbox",
                    "read-only",
                    "--output-last-message",
                    "/r/p.json"
                ])
            ),
        ]
    );
    assert_eq!(
        gate.launch_arguments(),
        json!({"authored": ["--model", "m-1"],
               "managed": ["--sandbox", "read-only", "--output-last-message", "/r/p.json"]})
    );

    // An inline command spelling exactly those bytes is the author's.
    let copied = compose(BuiltBoundary::Open, gate.argv.clone(), None);
    assert_eq!(copied.argv, gate.argv);
    assert_eq!(
        copied.segments,
        [Segment::new(Origin::Authored, &gate.argv)]
    );
    assert_eq!(
        copied.launch_arguments(),
        json!({"authored": ["--model", "m-1", "--sandbox", "read-only",
                            "--output-last-message", "/r/p.json"],
               "managed": []})
    );

    // `harness`, an inline exec dispatch: the network prefix is the
    // engine's, in front of the author's command.
    let spawn = compose(BuiltBoundary::Harness, exec.clone(), None);
    assert_eq!(
        spawn.segments,
        [
            Segment::new(Origin::Hands, &strings(&["unshare", "--net", "--"])),
            Segment::new(Origin::Authored, &exec),
        ]
    );
    assert_eq!(spawn.rewalk, Some(PathBuf::from("/bundle/scripts")));

    // A command that is not the link's own composition — the boxed link's
    // command with its hands stripped — keeps its argv, all of it
    // authored, and is refused; so is a link that never composed.
    let stripped = compose(BuiltBoundary::Harness, unboxed.argv.clone(), Some(&boxed));
    assert_eq!(
        stripped.refusal.as_deref(),
        Some(
            "dispatch refused: the command handed to composition is not the selected \
             candidate's own composition, and an argument whose origin is not carried is never \
             trusted by its bytes (decision 0065 slice one, design D5.7)"
        )
    );
    assert_eq!(origins(&stripped), [Origin::Authored, Origin::Hands]);
    let uncomposed = Candidate {
        lowering: Lowering::Unavailable,
        ..unboxed.clone()
    };
    let spawn = compose(BuiltBoundary::Open, unboxed.argv.clone(), Some(&uncomposed));
    assert_eq!(
        spawn.refusal.as_deref(),
        Some(
            "dispatch refused: the selected candidate carries no composition, so who supplied \
             its arguments is unknown (decision 0065 slice one, design D5.7)"
        )
    );
    assert_eq!(
        spawn.segments,
        [Segment::new(Origin::Authored, &unboxed.argv)]
    );
}

/// Unit 4 (design D5.7): the driver's extras begin where the driver itself
/// reads them — after whatever the engine put in front of the launch for
/// the boundary, carried as its leading `hands` segments, and after the
/// driver verb, dropping only an escape `--` directly behind the verb, as
/// its trailing-argument parser does. A later `--` is an argument, and a
/// wrapper's own `--` is never the driver's.
#[test]
fn the_driver_extras_begin_after_the_engines_prefix_and_the_verbs_own_escape() {
    let spec = HandsSpec::default();
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    // The real prefix's eight tokens, spelled here for uid and gid 1000.
    let prefix = strings(&[
        "unshare",
        "--map-root-user",
        "--net",
        "--",
        "sh",
        "-c",
        "ip link set lo up && exec unshare --map-user=1000 --map-group=1000 -- \"$@\"",
        "sh",
    ]);
    let compose = |boundary, command: Vec<String>| {
        compose_site(
            boundary,
            SeatClass::Gate,
            command,
            Some(&spec),
            None,
            Path::new("/work"),
            &[PathBuf::from("/bundle")],
            "/r/p.json",
            Some(&Unboxed {
                env: BTreeMap::new(),
                prefix: prefix.clone(),
            }),
        )
    };
    let exec = exec_dispatch(Path::new("/bundle/scripts/verify.sh"));

    // Inside the box: neither the box's prefix nor the dispatched launcher.
    let boxed = compose(BuiltBoundary::Namespace, exec.clone());
    assert_eq!(
        boxed.extras(),
        [Segment::new(
            Origin::Authored,
            &strings(&["bash", "/runtime/bundle/scripts/verify.sh", "{prompt_file}"])
        )]
    );
    // Behind the network prefix: neither its wrapper nor the launcher.
    let unboxed = compose(BuiltBoundary::Harness, exec.clone());
    assert_eq!(unboxed.argv[..8], prefix[..]);
    assert_eq!(
        unboxed.extras(),
        [Segment::new(
            Origin::Authored,
            &strings(&["bash", "/bundle/scripts/verify.sh", "{prompt_file}"])
        )]
    );
    assert_eq!(
        unboxed.launch_arguments(),
        json!({"authored": ["bash", "/bundle/scripts/verify.sh", "{prompt_file}"],
               "managed": []})
    );

    // An authored driver argument before a later `--` is the driver's.
    let inline = strings(&[
        "/bin/brokkr",
        "driver",
        "dsh",
        "--model",
        "p/m",
        "--",
        "--effort",
        "high",
    ]);
    let spawn = compose(BuiltBoundary::Open, inline);
    assert_eq!(
        spawn.extras(),
        [Segment::new(
            Origin::Authored,
            &strings(&["--model", "p/m", "--", "--effort", "high"])
        )]
    );
    // Only the escape directly behind the verb is the parser's.
    let escaped = compose(
        BuiltBoundary::Open,
        strings(&["/bin/brokkr", "driver", "dsh", "--", "--", "x"]),
    );
    assert_eq!(
        escaped.extras(),
        [Segment::new(Origin::Authored, &strings(&["--", "x"]))]
    );
    // An argv that ends at the verb, or at its escape, hands no extras.
    for bare in [
        strings(&["/bin/brokkr", "driver", "dsh"]),
        strings(&["/bin/brokkr", "driver", "dsh", "--"]),
    ] {
        assert_eq!(compose(BuiltBoundary::Open, bare).extras(), []);
    }
}

/// Unit 4 (design D5.7): the compile expands `{brokkr}` and `./` one
/// segment at a time, so every expanded token keeps the origin of the
/// segment that supplied it; a lowering that never composed stays exactly
/// what it was, never an empty composition.
#[test]
fn the_compiles_expansion_keeps_every_segments_origin() {
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let strings = |parts: &[&str]| {
        parts
            .iter()
            .map(|part| part.to_string())
            .collect::<Vec<_>>()
    };
    let intent = crate::agents::Intent {
        allow: AllowIntent::Listed(strings(&["cargo"])),
        sandbox: SandboxIntent::ReadOnly,
        hands: HandsIntent::Required,
    };
    let composed = |segments: Vec<Segment>| {
        Lowering::Composed(crate::agents::Composition {
            segments,
            effort: Some("high".into()),
            intent: intent.clone(),
            application: Application::Dormant,
            template: TemplateExpectation::None,
            serving: Default::default(),
        })
    };
    let written = composed(vec![
        Segment::new(
            Origin::Template,
            &strings(&["{brokkr}", "driver", "codex", "--"]),
        ),
        Segment::new(Origin::Local, &strings(&["./scripts/check.sh", ""])),
        Segment::new(Origin::Hands, &strings(&["--sandbox", "read-only"])),
    ]);
    assert_eq!(
        crate::bundle::expand_lowering(Path::new("/bundle"), &written),
        composed(vec![
            Segment::new(Origin::Template, &strings(&[&exe, "driver", "codex", "--"])),
            Segment::new(Origin::Local, &strings(&["/bundle/scripts/check.sh", ""])),
            Segment::new(Origin::Hands, &strings(&["--sandbox", "read-only"])),
        ])
    );
    for never in [Lowering::Unavailable, Lowering::Refused(intent.clone())] {
        assert_eq!(
            crate::bundle::expand_lowering(Path::new("/bundle"), &never),
            never
        );
    }
}

#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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
    // The same link resolved unboxed: no workspace fragment composed.
    let unboxed = candidate("codex", Vec::new(), codex_harness());
    assert_eq!(unboxed.argv, base);

    // A site without hands: its command untouched, under every boundary.
    for boundary in [Boundary::Namespace, Boundary::Harness, Boundary::Open] {
        let spawn = compose_site(
            built_boundary(boundary).unwrap(),
            SeatClass::Gate,
            base.clone(),
            None,
            Some(&unboxed),
            workdir,
            &roots,
            "/r/p.json",
            None,
        );
        // Its tokens keep the origin the link carried them with.
        assert_eq!(
            spawn,
            SiteSpawn {
                segments: vec![Segment::new(Origin::Template, &base)],
                class: Some(SeatClass::Gate),
                ..SiteSpawn::inherit(base.clone())
            }
        );
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
        Ok(boxed_model.argv),
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
        Ok(boxed_exec.argv.clone()),
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
        assert_eq!(
            as_gate,
            SiteSpawn {
                class: Some(SeatClass::Gate),
                ..as_work
            }
        );
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
        assert_eq!(
            single,
            SiteSpawn {
                class: Some(SeatClass::Work),
                ..SiteSpawn::inherit(command.clone())
            }
        );
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
/// Linux with the probe passing, token for token; with an empty prefix,
/// the compiled command alone under both `harness` and `open`. These are
/// composition assertions: no interpreter starts, so they cannot prove
/// Windows/MSYS or PowerShell parsing, or an execution guarantee through
/// an unpinned interpreter (decision 0049 ruling 3).
#[test]
fn the_unboxed_exec_dispatch_composes_the_expected_argv_and_rewalk_directory() {
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
    let script_argv = script.display().to_string();
    let mut expected_command = command.clone();
    expected_command[5] = script_argv.clone();
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
        &script_argv,
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
        assert_eq!(alone.argv, expected_command);
    }
}

/// No filesystem or child process. The pin is the script's canonical
/// directory and the argv is compile's spelling, untouched: a root that
/// resembles another platform's syntax is one literal component. These
/// assertions prove composition only: the interpreter remains unpinned
/// under `harness` and `open`, with no execution guarantee (decision
/// 0049 ruling 3).
#[test]
fn exec_composition_keeps_the_canonical_pin_separate_from_the_script_argument() {
    for root in [
        r"\\?\C:\Users\gate bundle",
        r"\\?\UNC\server\share\gate bundle",
    ] {
        let root = PathBuf::from(root);
        let directory = root.join("scripts");
        let script = directory.join("gate.sh");
        let mut command = exec_dispatch(&script);
        // An inherited script precedes an unjudged argument naming a
        // different layer. Neither that argument nor argv[0] is rewritten.
        let leaf = PathBuf::from(r"\\?\C:\leaf");
        command.push(leaf.join("result.json").display().to_string());
        let roots = [leaf, root];
        let spawn = exec_spawn(command.clone(), &roots);
        assert_eq!(spawn.argv, command);
        assert_eq!(spawn.rewalk, Some(directory.clone()));
        assert_eq!(spawn.refusal, None);
        assert_eq!(spawn.env, SpawnEnv::Inherit);
        assert_eq!(command[5], script.display().to_string());
        // Unit 4: the same command supplied in two segments. Each segment
        // keeps its origin, and the segments stay the argv.
        let split = exec_segments(
            vec![
                Segment::new(Origin::Template, &command[..4]),
                Segment::new(Origin::Authored, &command[4..]),
            ],
            &roots,
        );
        assert_eq!(split.argv, command);
        assert_eq!(split.rewalk, Some(directory.clone()));
        assert_eq!(
            split.segments,
            [
                Segment::new(Origin::Template, &command[..4]),
                Segment::new(Origin::Authored, &command[4..]),
            ]
        );
    }

    // A backslash belongs to the filename, even if it resembles another
    // platform's separator. The public composer also exercises its wiring.
    let root = PathBuf::from("/bundle");
    let directory = root.join("scripts");
    let script = directory.join("gate.sh");
    let command = exec_dispatch(&script);
    let expected = command.clone();
    let spec = HandsSpec::parse(&json!("workspace")).unwrap();
    for boundary in [
        BuiltBoundary::Harness,
        BuiltBoundary::Open,
        BuiltBoundary::Namespace,
    ] {
        let plain = compose_site(
            boundary,
            SeatClass::Work,
            command.clone(),
            None,
            None,
            Path::new("/repo"),
            std::slice::from_ref(&root),
            "/result.json",
            None,
        );
        assert_eq!(plain.argv, expected);
        assert_eq!(plain.rewalk, None);
        assert_eq!(plain.refusal, None);
        if boundary != BuiltBoundary::Namespace {
            let spawn = compose_site(
                boundary,
                SeatClass::Gate,
                command.clone(),
                Some(&spec),
                None,
                Path::new("/repo"),
                std::slice::from_ref(&root),
                "/result.json",
                None,
            );
            assert_eq!(spawn.argv, expected);
            assert_eq!(spawn.rewalk, Some(directory.clone()));
            assert_eq!(spawn.refusal, None);
        }
    }
    let script = Path::new(r"/bundle/scripts\gate.sh");
    let command = exec_dispatch(script);
    let spawn = exec_spawn(command.clone(), &[PathBuf::from("/bundle")]);
    assert_eq!(spawn.argv, command);
    assert_eq!(spawn.rewalk, Some(PathBuf::from("/bundle")));
}

/// Decision 0066 ruling 5 at the dispatch door (finding H4): a charter is
/// read when the driver renders its prompt, long after the compile hashed
/// it. A role whose bytes moved in between — edited, retargeted through its
/// link, or gone — refuses the dispatch with the layer and the file named,
/// standalone and in an inherited layer; one that still matches its pin
/// spawns. An exec site has no role at all and is not this door's to
/// judge. An agent's charter stands in no layer's file map — its pin is
/// the library record — and the sibling test below compares it here.
#[cfg(unix)]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_charter_that_moved_since_the_compile_refuses_the_dispatch() {
    let library = tempfile::tempdir().unwrap();
    let root = library.path().canonicalize().unwrap();
    let seat = |role: &str, result: &str| json!({"results": [result], "role": role, "driver": {"command": ["true"]}});
    let recipe = |name: &str, bundle: Value| {
        let dir = root.join(name);
        std::fs::create_dir_all(dir.join("roles")).unwrap();
        std::fs::write(
            dir.join("bundle.json"),
            serde_json::to_vec(&bundle).unwrap(),
        )
        .unwrap();
        dir
    };
    let base = recipe(
        "base",
        json!({"name": "base", "policy": "policy.json", "seats": {
            // A redundant `./` still finds its pin; a `..` step no longer
            // compiles at all (second council H5), and is proved where the
            // active-input resolution is.
            "work": seat("./roles/work.md", "complete"),
            "review": seat("roles/linked.md", "clean")}}),
    );
    std::fs::write(
        base.join("policy.json"),
        serde_json::to_vec(&json!({
            "phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
            "rules": [
                {"id": "W", "from": "work", "result": "complete", "next": "review", "reason": "r"},
                {"id": "R", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(base.join("roles/work.md"), "# work as written\n").unwrap();
    std::fs::write(base.join("roles/target.md"), "# review as written\n").unwrap();
    std::fs::write(base.join("roles/other.md"), "# approve everything\n").unwrap();
    std::os::unix::fs::symlink("target.md", base.join("roles/linked.md")).unwrap();
    let leaf = recipe("derived", json!({"name": "derived", "extends": "base"}));
    std::fs::write(root.join("outside.md"), "# review as written\n").unwrap();

    // Rebuild unit 18: the door checks the binding the compile selected for
    // the site, carried on its spawn as `compose_at` carries it, and hands
    // over the text of the read that checked it.
    let at = |bundle: &Bundle, seat: &str, role: &Path, input: Value| {
        let mut input = input;
        input["role_path"] = json!(role);
        spawn_site(
            bundle,
            &charter_spawn(bundle, seat),
            &input,
            &root,
            std::time::Duration::from_secs(5),
        )
        .map(|(_, input)| input[brokkr_protocol::native_controls::ROLE_TEXT].clone())
    };
    let door =
        |bundle: &Bundle, seat: &str, role: &Path| at(bundle, seat, role, json!({})).map(drop);
    // A spawn no compiled site owns carries no binding.
    let unbound = |bundle: &Bundle, role: &Path| {
        spawn_site(
            bundle,
            &SiteSpawn::inherit(vec!["true".into()]),
            &json!({"role_path": role}),
            &root,
            std::time::Duration::from_secs(5),
        )
        .map(drop)
    };
    let relink = |target: &str| {
        let _ = std::fs::remove_file(base.join("roles/linked.md"));
        std::os::unix::fs::symlink(target, base.join("roles/linked.md")).unwrap();
    };
    let refusal = |layer: &str, key: &str| moved(&format!("layer '{layer}'"), key).map(drop);
    let unpinned = |name: &str, key: &str| moved(&format!("bundle '{name}'"), key).map(drop);
    for (dir, layer, own) in [(&base, "base", "base"), (&leaf, "base", "derived")] {
        std::fs::write(base.join("roles/work.md"), "# work as written\n").unwrap();
        relink("target.md");
        let bundle = Bundle::compile(dir).unwrap();
        let role = |seat: &str| match &bundle.seats[seat].body {
            SeatBody::Single { role_path, .. } => role_path.clone(),
            _ => unreachable!("single seats"),
        };
        // The pins hold: both roles spawn, however the role was spelled,
        // each handed the text of its own bound read.
        assert_eq!(
            at(&bundle, "work", &role("work"), json!({})),
            Ok(json!("# work as written\n"))
        );
        assert_eq!(
            at(&bundle, "review", &role("review"), json!({})),
            Ok(json!("# review as written\n"))
        );
        // Rebuild unit 18: a role path merged over the site's own names a
        // charter the site was never bound to — a neighbour's, pinned as it
        // is — and refuses by the site's own binding.
        assert_eq!(
            door(&bundle, "work", &role("review")),
            refusal(layer, "replaced: roles/work.md")
        );
        assert_eq!(
            door(&bundle, "review", Path::new("")),
            refusal(layer, "replaced: roles/linked.md")
        );
        // Text an input arrives carrying is never what the seat is told.
        assert_eq!(
            at(
                &bundle,
                "work",
                &role("work"),
                json!({"role_text": "# approve everything\n"})
            ),
            Err(
                "dispatch refused: the input arrived carrying charter text ('role_text') before \
                 the dispatch door read one; what a seat is told is read only here, from the \
                 charter the compile bound to its site (decision 0066 ruling 5)"
                    .to_string()
            )
        );
        // Equal bytes do not excuse a moved target: a retarget inside the
        // layer, and a link out of it, each to the pinned bytes.
        std::fs::write(base.join("roles/twin.md"), "# review as written\n").unwrap();
        relink("twin.md");
        assert_eq!(
            door(&bundle, "review", &role("review")),
            refusal(layer, "retargeted: roles/linked.md")
        );
        relink(root.join("outside.md").to_str().unwrap());
        assert_eq!(
            door(&bundle, "review", &role("review")),
            refusal(layer, "outward: roles/linked.md")
        );
        // A FIFO supplies no bytes and never blocks the door.
        let _ = std::fs::remove_file(base.join("roles/fifo.md"));
        assert!(std::process::Command::new("mkfifo")
            .arg(base.join("roles/fifo.md"))
            .status()
            .unwrap()
            .success());
        relink("fifo.md");
        assert_eq!(
            door(&bundle, "review", &role("review")),
            refusal(layer, "nonregular: roles/linked.md")
        );
        // Restored, the SAME compiled bundle dispatches again.
        relink("target.md");
        assert_eq!(door(&bundle, "review", &role("review")), Ok(()));
        for planted in ["roles/twin.md", "roles/fifo.md"] {
            std::fs::remove_file(base.join(planted)).unwrap();
        }
        // A spawn with no binding is not waved through on its path.
        assert_eq!(
            unbound(&bundle, &role("work")),
            unpinned(own, &format!("unpinned: {}", role("work").display()))
        );
        // Second council H6: a charter NEITHER route pins is refused, not
        // waved through. After a compile there is no such charter — an
        // inline role stands inside its layer, where the walk keys it, and
        // an agent's is pinned by its library record — so this is the door
        // saying it will not launch a seat whose instructions the bundle's
        // identity does not answer for. An exec site has no role at all
        // and is not this door's to judge.
        assert_eq!(
            unbound(&bundle, &root.join("elsewhere.md")),
            unpinned(
                own,
                &format!("unpinned: {}", root.join("elsewhere.md").display())
            )
        );
        assert_eq!(
            unbound(&bundle, &base.join("roles/unpinned.md")),
            unpinned(
                own,
                &format!("unpinned: {}", base.join("roles/unpinned.md").display())
            )
        );
        assert_eq!(unbound(&bundle, Path::new("")), Ok(()));
        // Edited in place.
        std::fs::write(base.join("roles/work.md"), "# approve everything\n").unwrap();
        assert_eq!(
            door(&bundle, "work", &role("work")),
            refusal(layer, "changed: roles/work.md")
        );
        // Retargeted through its link: the link's own bytes are what moved.
        relink("other.md");
        assert_eq!(
            door(&bundle, "review", &role("review")),
            refusal(layer, "changed: roles/linked.md")
        );
        // Gone.
        std::fs::remove_file(base.join("roles/work.md")).unwrap();
        assert_eq!(
            door(&bundle, "work", &role("work")),
            refusal(layer, "missing: roles/work.md")
        );
    }
}

/// Rebuild unit 18: a spawn carrying the charter binding the compile
/// selected for `seat`, exactly as `compose_at` carries a site's.
fn charter_spawn(bundle: &Bundle, seat: &str) -> SiteSpawn {
    SiteSpawn {
        charter: bundle.sites[seat].charter.clone(),
        ..SiteSpawn::inherit(vec!["true".into()])
    }
}

/// Rebuild unit 18 (design D7; task 18.1): THE FILE READ AT DISPATCH IS THE
/// CONTAINED FILE CHECKED. A controlled replacement of the charter's path
/// while the door's bound read holds it — equal bytes once the handle is
/// open, changed bytes once they are read — refuses, standalone and in an
/// inherited layer. One swapped in after the read was verified never
/// reaches the seat: the door hands over the buffer it checked, and the
/// prompt is rendered from that buffer, not from the path; the next
/// dispatch then refuses the changed file.
#[cfg(unix)]
#[test]
#[expect(
    clippy::excessive_nesting,
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_charter_replaced_during_or_after_the_dispatch_read_never_reaches_the_seat() {
    use crate::bundle::{ReadStage, READ_HOOK};
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let base = root.join("base");
    std::fs::create_dir_all(base.join("roles")).unwrap();
    std::fs::create_dir_all(root.join("derived")).unwrap();
    std::fs::write(base.join("roles/work.md"), "# work as written\n").unwrap();
    std::fs::write(
        base.join("policy.json"),
        serde_json::to_vec(&json!({
            "phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
            "rules": [
                {"id": "W", "from": "work", "result": "complete", "next": "review", "reason": "r"},
                {"id": "R", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        base.join("bundle.json"),
        serde_json::to_vec(&json!({"name": "base", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "role": "roles/work.md",
                     "driver": {"command": ["true"]}},
            "review": {"results": ["clean"], "role": "roles/work.md",
                       "driver": {"command": ["true"]}}}}))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("derived/bundle.json"),
        serde_json::to_vec(&json!({"name": "derived", "extends": "base"})).unwrap(),
    )
    .unwrap();
    let charter = base.join("roles/work.md");
    // Put `bytes` at the charter's path by renaming a new file over it, so
    // the file the door already holds keeps what it held.
    let swap = |bytes: &'static str| {
        let (charter, staged) = (charter.clone(), base.join("roles/staged.md"));
        move || {
            std::fs::write(&staged, bytes).unwrap();
            std::fs::rename(&staged, &charter).unwrap();
        }
    };
    let replacing = |bundle: &Bundle, stage: ReadStage, act: Box<dyn FnOnce()>| {
        let watched = charter.clone();
        let mut act = Some(act);
        READ_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |at, target| {
                if at == stage && target == watched {
                    if let Some(act) = act.take() {
                        act();
                    }
                }
            }));
        });
        let outcome = spawn_site(
            bundle,
            &charter_spawn(bundle, "work"),
            &json!({"role_path": charter, "native_controls": {}, "result_path": "/r.json",
                    "allowed_results": ["complete"]}),
            &root,
            std::time::Duration::from_secs(5),
        )
        .map(|(_, input)| input);
        READ_HOOK.with(|hook| *hook.borrow_mut() = None);
        outcome
    };
    let refusal = |key: &str| {
        format!(
            "dispatch refused: a charter of layer 'base' moved since the compile ({key}); what a \
             seat is told must be the bytes the bundle's identity names (decision 0066 ruling 5)"
        )
    };
    for recipe in ["base", "derived"] {
        std::fs::write(&charter, "# work as written\n").unwrap();
        let bundle = Bundle::compile(&root.join(recipe)).unwrap();
        // Rebuild unit 18-fix-b: the file the compile read is put back at
        // its path, because equal bytes in another file are not it.
        let original = root.join(format!("{recipe}.original.md"));
        std::fs::hard_link(&charter, &original).unwrap();
        let restore = || {
            std::fs::remove_file(&charter).unwrap();
            std::fs::hard_link(&original, &charter).unwrap();
        };
        for (stage, bytes) in [
            (ReadStage::Opened, "# work as written\n"),
            (ReadStage::Read, "# approve everything\n"),
        ] {
            restore();
            assert_eq!(
                replacing(&bundle, stage, Box::new(swap(bytes))).map(drop),
                Err(refusal("replaced: roles/work.md")),
                "{recipe} {stage:?}"
            );
        }
        restore();
        let input = replacing(
            &bundle,
            ReadStage::Verified,
            Box::new(swap("# approve everything\n")),
        )
        .expect("a replacement after the verified read is not what the door read");
        assert_eq!(
            input[brokkr_protocol::native_controls::ROLE_TEXT],
            "# work as written\n"
        );
        let prompt = brokkr_protocol::adapters::render_prompt(
            &input,
            brokkr_protocol::adapters::AdapterKind::Dsh,
        )
        .unwrap();
        assert!(
            prompt.starts_with("# work as written\n\n\n---\n"),
            "{prompt}"
        );
        assert!(!prompt.contains("approve everything"), "{prompt}");
        assert_eq!(
            replacing(&bundle, ReadStage::Verified, Box::new(|| ())).map(drop),
            Err(refusal("changed: roles/work.md"))
        );
    }
}

/// Second council H6: DISPATCH DOES NOT ENFORCE LIBRARY CHARTER PINS.
///
/// The chief compiled an agent-backed bundle, changed
/// `agents/charters/worker.md`, and watched `charter_drift` answer `None`
/// while `render_prompt` consumed the changed text: the charter stands in
/// the library, outside every layer's file map, so the door had nothing
/// to compare it against and let the launch through. The library record's
/// own `charter_digest` was already there.
///
/// Compile once, edit the charter, dispatch WITHOUT recompiling: the
/// refusal names the agent and the file, before any provider work.
/// Restoring the bytes restores the dispatch, and a recompile is not
/// what makes it pass.
#[cfg(unix)]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_library_charter_that_moved_since_the_compile_refuses_the_dispatch() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let write = |relative: &str, body: &str| {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write("agents/charters/worker.md", "# work as written\n");
    write(
        "agents/worker.json",
        &serde_json::to_string(&json!({
            "description": "the worker",
            "charter": "charters/worker.md",
            "models": ["opus"],
            "efforts": {"opus": "high"}
        }))
        .unwrap(),
    );
    write(
        "adapters/claude.json",
        &std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("adapters/claude.json"),
        )
        .unwrap(),
    );
    write("recipe/roles/review.md", "# review\n");
    write(
        "recipe/policy.json",
        &serde_json::to_string(&json!({
            "phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
            "rules": [
                {"id": "W", "from": "work", "result": "complete", "next": "review", "reason": "r"},
                {"id": "R", "from": "review", "result": "clean", "next": "done", "reason": "r"}]}))
        .unwrap(),
    );
    write(
        "recipe/bundle.json",
        &serde_json::to_string(
            &json!({"name": "recipe", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "worker"},
            "review": {"results": ["clean"], "role": "roles/review.md",
                       "driver": {"command": ["true"]}}}}),
        )
        .unwrap(),
    );
    // A leaf that inherits the agent-backed seat, so the pin is proved
    // where the charter's owner is an ancestor as well as where it is the
    // recipe itself.
    write(
        "derived/bundle.json",
        &serde_json::to_string(&json!({"name": "derived", "extends": "recipe"})).unwrap(),
    );
    let compile = |recipe: &str| {
        Bundle::compile_with_realm(
            &root.join(recipe),
            &root.join("agents"),
            &root.join("adapters"),
            None,
            None,
            brokkr_core::realms::Boundary::Namespace,
        )
        .expect("the agent-backed recipe compiles")
    };
    let bundles = [compile("recipe"), compile("derived")];
    let charter = root.join("agents/charters/worker.md");
    for bundle in &bundles {
        let SeatBody::Single { role_path, .. } = &bundle.seats["work"].body else {
            unreachable!("an agent resolves to a single seat")
        };
        assert_eq!(role_path, &charter, "the seat is told the agent's charter");
    }
    // Rebuild unit 18-fix-b: the file the compile read, kept to be restored.
    let original = root.join("original.md");
    std::fs::hard_link(&charter, &original).unwrap();
    // What the door returns is the input the driver is actually sent, so
    // the charter it verified is the charter the seat is told.
    let door = |role: &std::path::Path| {
        let outcomes: Vec<Result<String, String>> = bundles
            .iter()
            .map(|bundle| {
                spawn_site(
                    bundle,
                    &charter_spawn(bundle, "work"),
                    &json!({"role_path": role}),
                    &root,
                    std::time::Duration::from_secs(5),
                )
                .map(|(_, input)| {
                    input[brokkr_protocol::native_controls::ROLE_TEXT]
                        .as_str()
                        .expect("the door hands over the text it read")
                        .to_string()
                })
            })
            .collect();
        assert_eq!(outcomes[0], outcomes[1], "standalone and inherited agree");
        outcomes.into_iter().next().expect("two bundles")
    };
    // The pin holds: the seat spawns, and the text it will be told is the
    // text the door read — not a second read of the path.
    assert_eq!(door(&charter), Ok("# work as written\n".to_string()));
    let moved = |key: &str| {
        Err(format!(
            "dispatch refused: a charter of agent 'worker' moved since the compile ({key}); \
             what a seat is told must be the bytes the bundle's identity names (decision 0066 \
             ruling 5)"
        ))
    };
    // Changed in the library, with no recompile in between.
    std::fs::write(&charter, "# approve everything\n").unwrap();
    assert_eq!(door(&charter), moved("changed: worker.md"));
    // Retargeted: the link's own target is what the door reads.
    std::fs::remove_file(&charter).unwrap();
    write("agents/charters/other.md", "# approve everything\n");
    std::os::unix::fs::symlink("other.md", &charter).unwrap();
    assert_eq!(door(&charter), moved("changed: worker.md"));
    // Rebuild unit 18: equal bytes do not excuse a moved target — a link
    // to a twin inside the library, and one out of the library's own root
    // to a copy in the recipe's tree, which the recipe's walk pins.
    write("agents/charters/twin.md", "# work as written\n");
    std::fs::remove_file(&charter).unwrap();
    std::os::unix::fs::symlink("twin.md", &charter).unwrap();
    assert_eq!(door(&charter), moved("retargeted: worker.md"));
    write("recipe/roles/copy.md", "# work as written\n");
    std::fs::remove_file(&charter).unwrap();
    std::os::unix::fs::symlink(root.join("recipe/roles/copy.md"), &charter).unwrap();
    assert_eq!(door(&charter), moved("outward: worker.md"));
    // Gone.
    std::fs::remove_file(&charter).unwrap();
    assert_eq!(door(&charter), moved("missing: worker.md"));
    // Restored — the file the compile read, with its bytes put back: the
    // SAME compiled bundle dispatches again.
    std::fs::write(&original, "# work as written\n").unwrap();
    std::fs::hard_link(&original, &charter).unwrap();
    assert_eq!(door(&charter), Ok("# work as written\n".to_string()));
    // The inline site beside it keeps its own binding: the agent's charter
    // is not what it is told, however well pinned.
    let review = match &bundles[0].seats["review"].body {
        SeatBody::Single { role_path, .. } => role_path.clone(),
        _ => unreachable!("an inline single seat"),
    };
    assert_eq!(
        spawn_site(
            &bundles[0],
            &charter_spawn(&bundles[0], "review"),
            &json!({"role_path": charter}),
            &root,
            std::time::Duration::from_secs(5),
        )
        .map(drop),
        Err(
            "dispatch refused: a charter of layer 'recipe' moved since the compile (replaced: \
             roles/review.md); what a seat is told must be the bytes the bundle's identity names \
             (decision 0066 ruling 5)"
                .to_string()
        )
    );
    assert_eq!(review, root.join("recipe/roles/review.md"));

    // The pin is over BYTES, and what a seat is told is TEXT. A charter
    // pinned as bytes nobody can decode is refused at the door rather
    // than rendered with its undecodable parts replaced: what the seat
    // would then read is not what the digest names.
    std::fs::write(&charter, [b'#', b' ', 0xff, b'\n']).unwrap();
    let bundle = compile("recipe");
    assert_eq!(
        spawn_site(
            &bundle,
            &charter_spawn(&bundle, "work"),
            &json!({"role_path": charter}),
            &root,
            std::time::Duration::from_secs(5),
        )
        .map(drop),
        Err(
            "dispatch refused: a charter of agent 'worker' moved since the compile (unreadable: \
             worker.md); what a seat is told must be the bytes the bundle's identity names \
             (decision 0066 ruling 5)"
                .to_string()
        )
    );
}

/// Rebuild unit 18-fix (design D7; task 18.1; council F1): THE OWNER IS THE
/// DIRECTORY THE COMPILE BOUND, NOT WHATEVER ITS PATH NOW NAMES. One
/// compiled bundle whose inline charter is owned by its layer and whose
/// agent's charter is owned by an external library, both under one realm
/// directory. Each owner's root, and then the realm directory above both,
/// is renamed away and replaced — by a link to an equal-byte copy outside,
/// and by an equal-byte copy standing at the very path — and every
/// dispatch refuses as `replaced` before its driver starts. An owner that
/// is only gone is `missing`. Renamed back, the SAME bundle dispatches
/// again with the text its door read.
#[cfg(unix)]
#[test]
fn an_owner_or_an_ancestor_replaced_since_the_compile_refuses_the_dispatch() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    let bundle = two_owners(&realm, "charters/worker.md");
    // What each site's door hands its driver, or why it refused; a spawn
    // that got past the door leaves its own marker.
    let spawns = std::cell::Cell::new(0);
    let door = |seat: &str| {
        spawns.set(spawns.get() + 1);
        dispatched(
            &bundle,
            seat,
            &root.join(format!("provider-work-{}", spawns.get())),
        )
    };
    let both = || (door("review"), door("work"));
    let intact = || {
        (
            Ok(json!("# review as written\n")),
            Ok(json!("# work as written\n")),
        )
    };
    assert_eq!(both(), intact());
    let run = Engine::start(store_at(&root), bundle.clone(), "f", None);
    let run = run.unwrap().run_id;
    let before = written_in(&root, &run);
    let copy = |from: &Path, to: &Path| {
        assert!(std::process::Command::new("cp")
            .arg("-R")
            .arg(from)
            .arg(to)
            .status()
            .unwrap()
            .success());
    };
    let outside = root.join("outside");
    std::fs::create_dir(&outside).unwrap();
    for (moving, layer, library) in [
        ("recipe", "replaced: roles/review.md", None),
        ("agents", "", Some("replaced: worker.md")),
        ("", "replaced: roles/review.md", Some("replaced: worker.md")),
    ] {
        let (path, away) = match moving {
            "" => (realm.clone(), root.join("realm.away")),
            name => (realm.join(name), realm.join(format!("{name}.away"))),
        };
        let expected = |cause: &str| {
            let layer = match layer.is_empty() {
                true => intact().0,
                false => moved("layer 'recipe'", &layer.replace("replaced", cause)),
            };
            let library = match library {
                None => intact().1,
                Some(key) => moved("agent 'worker'", &key.replace("replaced", cause)),
            };
            (layer, library)
        };
        let twin = outside.join(path.file_name().unwrap());
        copy(&path, &twin);
        std::fs::rename(&path, &away).unwrap();
        // Gone: the door names what it cannot find.
        assert_eq!(both(), expected("missing"), "{moving:?} gone");
        // A link to an equal-byte copy outside, in its place.
        std::os::unix::fs::symlink(&twin, &path).unwrap();
        assert_eq!(both(), expected("replaced"), "{moving:?} linked out");
        std::fs::remove_file(&path).unwrap();
        // An equal-byte copy standing at the very path.
        copy(&twin, &path);
        assert_eq!(both(), expected("replaced"), "{moving:?} copied in");
        std::fs::remove_dir_all(&path).unwrap();
        std::fs::remove_dir_all(&twin).unwrap();
        // Renamed back: the same bundle dispatches again.
        std::fs::rename(&away, &path).unwrap();
        assert_eq!(both(), intact(), "{moving:?} restored");
    }
    // The owners' own directories, unchanged, moved under a new realm
    // directory: the directory above them is not the one bound.
    let away = root.join("realm.away");
    std::fs::rename(&realm, &away).unwrap();
    std::fs::create_dir(&realm).unwrap();
    for owner in ["recipe", "agents", "adapters"] {
        std::fs::rename(away.join(owner), realm.join(owner)).unwrap();
    }
    assert_eq!(
        both(),
        (
            moved("layer 'recipe'", "replaced: roles/review.md"),
            moved("agent 'worker'", "replaced: worker.md")
        ),
        "an ancestor replaced"
    );
    // Rebuild unit 24: so do a start, a dispatch-bound start and the run's
    // resume, by the first binding, and nothing is written.
    let refused = charter_moved("agent 'worker'", "replaced: worker.md");
    let doors = doors(&root, &bundle, &run, None).map(answer);
    assert_eq!(doors, [refused.clone(), refused.clone(), refused], "doors");
    assert_eq!(written_in(&root, &run), before, "nothing is written");
}

/// Rebuild unit 18-fix (task 18.1; council F3 and F4): EVERY WAY THE DOOR'S
/// READ FAILS IS REFUSED BY ITS OWN KIND, for both owner kinds, before the
/// driver starts. A charter that is there but cannot be read is a failed
/// read (`unreadable`), not a decoding. A charter now linked under a
/// top-level name the walk skips is `unbound`. A binding whose layer is not
/// one of the bundle's is a charter the bundle does not answer for.
/// Rebuild unit 18-fix-b: a pin is built only from a bound read, so an owner
/// the compile did not bind can no longer be compiled, and an agent charter
/// the library names through a `..` is refused by the compile.
#[cfg(unix)]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn every_way_the_dispatch_read_fails_is_refused_by_its_own_kind() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    let bundle = two_owners(&realm, "charters/worker.md");
    let spawns = std::cell::Cell::new(0);
    let both = |bundle: &Bundle| {
        let door = |seat: &str| {
            spawns.set(spawns.get() + 1);
            let marker = root.join(format!("provider-work-{}", spawns.get()));
            dispatched(bundle, seat, &marker)
        };
        (door("review"), door("work"))
    };
    let (review, worker) = (
        realm.join("recipe/roles/review.md"),
        realm.join("agents/charters/worker.md"),
    );
    let mode = |mode: u32| {
        for path in [&review, &worker] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
        }
    };
    mode(0o000);
    assert_eq!(
        both(&bundle),
        (
            moved("layer 'recipe'", "unreadable: roles/review.md"),
            moved("agent 'worker'", "unreadable: worker.md")
        )
    );
    mode(0o644);
    let intact = (
        Ok(json!("# review as written\n")),
        Ok(json!("# work as written\n")),
    );
    assert_eq!(both(&bundle), intact);
    // A link that climbs out of its owner by `..`.
    std::fs::write(root.join("outside.md"), "# work as written\n").unwrap();
    for path in [&review, &worker] {
        let aside = path.with_extension("aside");
        std::fs::rename(path, &aside).unwrap();
        std::os::unix::fs::symlink("../../../outside.md", path).unwrap();
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            "# work as written\n"
        );
        let refused = both(&bundle);
        std::fs::remove_file(path).unwrap();
        std::fs::rename(&aside, path).unwrap();
        assert_eq!(
            refused,
            match path == &review {
                true => (
                    moved("layer 'recipe'", "outward: roles/review.md"),
                    intact.1.clone()
                ),
                false => (
                    intact.0.clone(),
                    moved("agent 'worker'", "outward: worker.md")
                ),
            }
        );
    }
    // Rebuild unit 18-fix-b: each charter linked, within its owner, to equal
    // bytes under a top-level name the walk skips.
    for (path, owner) in [
        (&review, realm.join("recipe")),
        (&worker, realm.join("agents")),
    ] {
        std::fs::create_dir(owner.join("dialects")).unwrap();
        std::fs::rename(path, owner.join("dialects/charter.md")).unwrap();
        let text = owner.join("dialects/charter.md");
        std::os::unix::fs::symlink(&text, path).unwrap();
    }
    let unbound = both(&bundle);
    for (path, owner) in [
        (&review, realm.join("recipe")),
        (&worker, realm.join("agents")),
    ] {
        std::fs::remove_file(path).unwrap();
        std::fs::rename(owner.join("dialects/charter.md"), path).unwrap();
        std::fs::remove_dir(owner.join("dialects")).unwrap();
    }
    assert_eq!(
        unbound,
        (
            moved("layer 'recipe'", "unbound: roles/review.md"),
            moved("agent 'worker'", "unbound: worker.md")
        )
    );
    let mut foreign = bundle.clone();
    for pin in foreign
        .sites
        .values_mut()
        .filter_map(|site| site.charter.as_mut())
    {
        pin.owner = crate::bundle::CharterOwner::Layer {
            dir: root.join("elsewhere"),
            key: pin.reference.clone(),
        };
    }
    assert_eq!(
        both(&foreign),
        (
            moved(
                "bundle 'recipe'",
                &format!("unpinned: {}", review.display())
            ),
            moved(
                "bundle 'recipe'",
                &format!("unpinned: {}", worker.display())
            )
        )
    );
    // Rebuild unit 18-fix-b: an agent charter named through a `..` is one no
    // bound read binds, so no pin is built for it: the compile refuses it.
    let dotted = owners_compiled(&root.join("dotted"), "charters/../charters/worker.md");
    assert_eq!(
        dotted.map(|bundle| bundle.name),
        Err(
            "bundle: seat 'work': agent 'worker' names charter 'charters/../charters/worker.md', \
             which reaches its file through a '..' step — never a path the bundle's file walk \
             takes, so a link earlier in it can put the file a reader opens outside everything \
             the walk pinned. What a seat is told must be what the bundle's identity names, so \
             it is refused (decision 0065 slice one, design D7)"
                .to_string()
        )
    );
    assert_eq!(both(&bundle), intact);
}

/// Rebuild unit 18-fix-b (design D7; task 18.1; council F1): THE PIN CARRIES
/// THE BINDING THE COMPILE READ, the file included, not a path that names
/// it. For both owner kinds, the charter is renamed aside and equal bytes
/// are written at its very path (a different file), then the directory
/// holding it is renamed aside and an equal-byte copy stands in its place:
/// every such dispatch refuses as `replaced` before its driver starts, and
/// renamed back, the SAME bundle dispatches again.
#[cfg(unix)]
#[test]
fn a_charter_replaced_by_equal_bytes_at_its_own_path_refuses_the_dispatch() {
    use std::os::unix::fs::MetadataExt;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    let bundle = two_owners(&realm, "charters/worker.md");
    let spawns = std::cell::Cell::new(0);
    let both = || {
        let door = |seat: &str| {
            spawns.set(spawns.get() + 1);
            let marker = root.join(format!("provider-work-{}", spawns.get()));
            dispatched(&bundle, seat, &marker)
        };
        (door("review"), door("work"))
    };
    let intact = (
        Ok(json!("# review as written\n")),
        Ok(json!("# work as written\n")),
    );
    assert_eq!(both(), intact);
    let (review, worker) = ("recipe/roles/review.md", "agents/charters/worker.md");
    let layer = moved("layer 'recipe'", "replaced: roles/review.md");
    let library = moved("agent 'worker'", "replaced: worker.md");
    for (moving, charter, refused) in [
        (review, review, (layer.clone(), intact.1.clone())),
        (worker, worker, (intact.0.clone(), library.clone())),
        ("recipe/roles", review, (layer.clone(), intact.1.clone())),
        (
            "agents/charters",
            worker,
            (intact.0.clone(), library.clone()),
        ),
    ] {
        let (path, away) = (realm.join(moving), realm.join(format!("{moving}.away")));
        let charter = realm.join(charter);
        let (bytes, file) = (
            std::fs::read(&charter).unwrap(),
            std::fs::metadata(&charter).unwrap().ino(),
        );
        std::fs::rename(&path, &away).unwrap();
        assert!(std::process::Command::new("cp")
            .arg("-R")
            .arg(&away)
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert_eq!(
            std::fs::read(&charter).unwrap(),
            bytes,
            "{moving}: equal bytes"
        );
        assert_ne!(
            std::fs::metadata(&charter).unwrap().ino(),
            file,
            "{moving}: another file"
        );
        assert_eq!(both(), refused, "{moving} replaced by equal bytes");
        match path.is_dir() {
            true => std::fs::remove_dir_all(&path).unwrap(),
            false => std::fs::remove_file(&path).unwrap(),
        }
        std::fs::rename(&away, &path).unwrap();
        assert_eq!(both(), intact, "{moving} restored");
    }
}

/// Rebuild unit 18-fix-b return (design D7; task 18.1; F1): THE PIN CARRIES
/// EVERY DIRECTORY ITS READ WALKED, not only the file it read. For both
/// owner kinds, the directory holding the charter is renamed aside, a new
/// directory is made at its path and the ORIGINAL charter is moved into it:
/// the charter is the very file the compile read, at the very path, under a
/// directory the compile never walked. Every such dispatch refuses as
/// `replaced` before its driver starts, and moved back, the SAME bundle
/// dispatches again.
#[cfg(unix)]
#[test]
fn a_charter_moved_into_a_replaced_directory_refuses_the_dispatch() {
    use std::os::unix::fs::MetadataExt;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    let bundle = two_owners(&realm, "charters/worker.md");
    let spawns = std::cell::Cell::new(0);
    let both = || {
        let door = |seat: &str| {
            spawns.set(spawns.get() + 1);
            let marker = root.join(format!("provider-work-{}", spawns.get()));
            dispatched(&bundle, seat, &marker)
        };
        (door("review"), door("work"))
    };
    let intact = (
        Ok(json!("# review as written\n")),
        Ok(json!("# work as written\n")),
    );
    assert_eq!(both(), intact);
    let layer = moved("layer 'recipe'", "replaced: roles/review.md");
    let library = moved("agent 'worker'", "replaced: worker.md");
    for (directory, name, refused) in [
        ("recipe/roles", "review.md", (layer, intact.1.clone())),
        ("agents/charters", "worker.md", (intact.0.clone(), library)),
    ] {
        let (path, away) = (
            realm.join(directory),
            realm.join(format!("{directory}.away")),
        );
        let charter = path.join(name);
        let (was, file) = (
            std::fs::metadata(&path).unwrap().ino(),
            std::fs::metadata(&charter).unwrap().ino(),
        );
        std::fs::rename(&path, &away).unwrap();
        std::fs::create_dir(&path).unwrap();
        std::fs::rename(away.join(name), &charter).unwrap();
        assert_ne!(
            std::fs::metadata(&path).unwrap().ino(),
            was,
            "{directory}: another directory"
        );
        assert_eq!(
            std::fs::metadata(&charter).unwrap().ino(),
            file,
            "{directory}: the same charter"
        );
        assert_eq!(both(), refused, "{directory} replaced around its charter");
        std::fs::rename(&charter, away.join(name)).unwrap();
        std::fs::remove_dir(&path).unwrap();
        std::fs::rename(&away, &path).unwrap();
        assert_eq!(both(), intact, "{directory} restored");
    }
}

/// Rebuild unit 18-fix-b return (design D7; task 18.1; F2): A LIBRARY'S
/// CHARTER READ IS HELD UNTIL THE BUNDLE IS SEALED, and its owner is checked
/// there as a layer's is. A controlled replacement as soon as the library
/// charter's read was verified — the library directory renamed aside, a new
/// one made at its path and every entry of the old one, the charter's own
/// directory among them, moved into it — refuses the compile; nothing is
/// moved back until the compile has answered. Restored, the recipe compiles.
#[cfg(unix)]
#[test]
fn a_library_owner_replaced_after_its_charter_was_read_refuses_the_compile() {
    use crate::bundle::{ReadStage, READ_HOOK};
    use std::os::unix::fs::MetadataExt;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    two_owners(&realm, "charters/worker.md");
    let (agents, away) = (realm.join("agents"), realm.join("agents.away"));
    let worker = agents.join("charters/worker.md");
    let compile = || {
        Bundle::compile_with_realm(
            &realm.join("recipe"),
            &agents,
            &realm.join("adapters"),
            None,
            None,
            brokkr_core::realms::Boundary::Namespace,
        )
        .map(|bundle| bundle.name)
        .map_err(|error| error.to_string())
    };
    fn shift(from: &Path, to: &Path) {
        for entry in std::fs::read_dir(from).unwrap() {
            let name = entry.unwrap().file_name();
            std::fs::rename(from.join(&name), to.join(&name)).unwrap();
        }
    }
    let (was, file) = (
        std::fs::metadata(&agents).unwrap().ino(),
        std::fs::metadata(&worker).unwrap().ino(),
    );
    {
        let (agents, away, worker) = (agents.clone(), away.clone(), worker.clone());
        let mut done = false;
        READ_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |stage, target: &Path| {
                if !done && stage == ReadStage::Verified && target == worker {
                    done = true;
                    std::fs::rename(&agents, &away).unwrap();
                    std::fs::create_dir(&agents).unwrap();
                    shift(&away, &agents);
                }
            }));
        });
    }
    let refused = compile();
    READ_HOOK.with(|hook| *hook.borrow_mut() = None);
    let replaced = (
        std::fs::metadata(&agents).unwrap().ino() != was,
        std::fs::metadata(&worker).unwrap().ino() == file,
    );
    shift(&agents, &away);
    std::fs::remove_dir(&agents).unwrap();
    std::fs::rename(&away, &agents).unwrap();
    assert_eq!(replaced, (true, true), "another owner, the same charter");
    assert_eq!(
        refused,
        Err(
            "bundle: seat 'work': agent 'worker' names charter 'charters/worker.md', which the \
             compile no longer holds as it was read: its library's directory, an entry on its \
             way, or its bytes changed after the read that bound it. What a seat is told must \
             be what the bundle's identity names, so it is refused (decision 0065 slice one, \
             design D7)"
                .to_string()
        )
    );
    assert_eq!(compile(), Ok("recipe".to_string()));
}

/// Rebuild unit 18-fix-b (design D7; task 18.1; council F3): AN OWNER THE
/// COMPILE CANNOT OBSERVE IS REFUSED BY THE COMPILE, NAMING WHAT IT COULD
/// NOT OBSERVE, never compiled into a bundle whose every dispatch then
/// refuses. The realm directory above both owners is made execute-only, so
/// every path under it still resolves but the directory itself cannot be
/// opened; with its mode restored the same recipe compiles. Its return
/// (F3): the remedy named is the directory's, never a move of the charter.
#[cfg(unix)]
#[test]
fn an_owner_whose_ancestor_the_compile_cannot_observe_refuses_the_compile() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    two_owners(&realm, "charters/worker.md");
    let compile = || {
        Bundle::compile_with_realm(
            &realm.join("recipe"),
            &realm.join("agents"),
            &realm.join("adapters"),
            None,
            None,
            brokkr_core::realms::Boundary::Namespace,
        )
        .map(|bundle| bundle.name)
        .map_err(|error| error.to_string())
    };
    let mode = |mode: u32| {
        std::fs::set_permissions(&realm, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    mode(0o111);
    let refused = compile();
    mode(0o755);
    assert_eq!(
        refused,
        Err(format!(
            "bundle: {}: seat 'review' names role 'roles/review.md', whose owner's directory \
             cannot be reached: '{}' cannot be opened (permission denied), so the directory the \
             charter is read from cannot be bound. The compile opens every directory from '/' \
             down to the charter's owner, so that directory must be readable by the user who \
             compiles; grant it, or compile from a realm under directories that user can read \
             (decision 0065 slice one, design D7)",
            realm.join("recipe/bundle.json").display(),
            realm.display()
        ))
    );
    assert_eq!(compile(), Ok("recipe".to_string()));
}

/// Rebuild unit 18-fix-b (design D7; task 18.1; council F2): WHO AN OWNER
/// IS, IS TAKEN BY THE READ THAT BOUND ITS CHARTER, and every later look at
/// it is compared with that. A controlled replacement at an observation of
/// the layer's owner — the realm directory above it swapped for another
/// holding the very same layer directory when the observation starts, and
/// swapped back as soon as the owner was taken — refuses the compile, both
/// during the charter's own read and at the first observation after that
/// read was verified: no bundle is bound to an owner its read did not walk.
/// The library's charter is read the same way, so bytes changed or removed
/// at its owner's observation after its library was loaded refuse too.
#[cfg(unix)]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn the_owner_a_charter_was_read_through_is_the_one_its_binding_records() {
    use crate::bundle::{ReadStage, READ_HOOK};
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    two_owners(&realm, "charters/worker.md");
    let (recipe, agents) = (realm.join("recipe"), realm.join("agents"));
    let (orig, twin) = (root.join("realm.orig"), root.join("twin"));
    std::fs::create_dir(&twin).unwrap();
    let worker = agents.join("charters/worker.md");
    let compile = || {
        Bundle::compile_with_realm(
            &recipe,
            &agents,
            &realm.join("adapters"),
            None,
            None,
            brokkr_core::realms::Boundary::Namespace,
        )
        .map(|bundle| bundle.name)
        .map_err(|error| error.to_string())
    };
    // Compile with `act` run when the first observation of `owner` starts
    // and again when it ends, counting only once `armed` was reached.
    type Act = Box<dyn FnMut(ReadStage)>;
    type Armed = Option<(ReadStage, PathBuf)>;
    let observed = |armed: Armed, owner: &Path, mut act: Act| {
        let (mut armed, owner, mut seen) = (armed, owner.to_path_buf(), 0);
        READ_HOOK.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move |stage, target: &Path| {
                if armed.as_ref() == Some(&(stage, target.to_path_buf())) {
                    armed = None;
                }
                let turn = matches!(
                    (seen, stage),
                    (0, ReadStage::Owning) | (1, ReadStage::Owned)
                );
                if armed.is_none() && target == owner && turn {
                    seen += 1;
                    act(stage);
                }
            }));
        });
        let outcome = compile();
        READ_HOOK.with(|hook| *hook.borrow_mut() = None);
        outcome
    };
    let swap = || -> Act {
        let (realm, orig, twin) = (realm.clone(), orig.clone(), twin.clone());
        Box::new(move |stage| {
            let moves = [
                (&realm, &orig),
                (&twin, &realm),
                (&orig.join("recipe"), &realm.join("recipe")),
            ];
            match stage {
                ReadStage::Owning => {
                    for (from, to) in moves {
                        std::fs::rename(from, to).unwrap();
                    }
                }
                _ => {
                    for (to, from) in moves.into_iter().rev() {
                        std::fs::rename(from, to).unwrap();
                    }
                }
            }
        })
    };
    let at_start = |change: fn(&Path)| -> Act {
        let worker = worker.clone();
        Box::new(move |stage| {
            if stage == ReadStage::Owning {
                change(&worker);
            }
        })
    };
    let file = recipe.join("bundle.json");
    let file = file.display();
    let library = |clause: &str| {
        format!(
            "bundle: seat 'work': agent 'worker' names charter 'charters/worker.md', {clause}. \
             What a seat is told must be what the bundle's identity names, so it is refused \
             (decision 0065 slice one, design D7)"
        )
    };
    let read = Some((ReadStage::Verified, recipe.join("roles/review.md")));
    let rows: [(&str, Armed, &Path, Act, String); 4] = [
        (
            "during the read",
            None,
            &recipe,
            swap(),
            format!(
                "bundle: {file}: seat 'review' names role 'roles/review.md', which was replaced \
                 while it was read: the file the read holds is no longer the contained target \
                 that was checked, so its bytes are not the ones verified. A charter there could \
                 change what the seat is told without moving the bundle's identity, so it is \
                 refused; move it to a path the bundle pins, such as 'roles/' (decision 0066 \
                 ruling 5)"
            ),
        ),
        (
            "after the read",
            read,
            &recipe,
            swap(),
            format!(
                "bundle: {file}: seat 'review' names role 'roles/review.md', which the walk that \
                 pinned the layer does not hold as it was read: its entry was replaced, \
                 retargeted or removed, or its bytes changed, after the read that bound it. What \
                 a seat is told must be what its identity names, so it is refused (decision 0065 \
                 slice one, design D7)"
            ),
        ),
        (
            "library charter changed",
            None,
            &agents,
            at_start(|worker| std::fs::write(worker, "# work changed\n").unwrap()),
            library("whose bytes changed after its library was loaded"),
        ),
        (
            "library charter removed",
            None,
            &agents,
            at_start(|worker| std::fs::remove_file(worker).unwrap()),
            library("which does not exist"),
        ),
    ];
    let (seen, expected): (Vec<_>, Vec<_>) = rows
        .into_iter()
        .map(|(row, armed, owner, act, refused)| {
            let outcome = observed(armed, owner, act);
            std::fs::write(&worker, "# work as written\n").unwrap();
            ((row, outcome), (row, Err(refused)))
        })
        .unzip();
    assert_eq!(seen, expected);
    assert_eq!(compile(), Ok("recipe".to_string()));
}

/// Rebuild unit 18-fix: a recipe under `realm` whose `review` seat's inline
/// charter its layer owns, and whose `work` seat is the external library's
/// `worker`, whose definition names its charter as `charter`.
fn two_owners(realm: &Path, charter: &str) -> Bundle {
    owners_compiled(realm, charter).expect("the recipe compiles")
}

/// [`two_owners`]'s recipe, written and compiled, or why it was refused.
fn owners_compiled(realm: &Path, charter: &str) -> Result<Bundle, String> {
    let write = |relative: &str, body: &str| {
        let path = realm.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    };
    write("agents/charters/worker.md", "# work as written\n");
    write(
        "agents/worker.json",
        &json!({"description": "the worker", "charter": charter,
                "models": ["opus"], "efforts": {"opus": "high"}})
        .to_string(),
    );
    write(
        "adapters/claude.json",
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/claude.json"),
        )
        .unwrap(),
    );
    write("recipe/roles/review.md", "# review as written\n");
    write(
        "recipe/policy.json",
        &json!({"phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
                "rules": [
                    {"id": "W", "from": "work", "result": "complete", "next": "review",
                     "reason": "r"},
                    {"id": "R", "from": "review", "result": "clean", "next": "done",
                     "reason": "r"}]})
        .to_string(),
    );
    write(
        "recipe/bundle.json",
        &json!({"name": "recipe", "policy": "policy.json", "seats": {
            "work": {"results": ["complete"], "agent": "worker"},
            "review": {"results": ["clean"], "role": "roles/review.md",
                       "driver": {"command": ["true"]}}}})
        .to_string(),
    );
    Bundle::compile_with_realm(
        &realm.join("recipe"),
        &realm.join("agents"),
        &realm.join("adapters"),
        None,
        None,
        brokkr_core::realms::Boundary::Namespace,
    )
    .map_err(|error| error.to_string())
}

/// What `seat`'s dispatch door hands its driver, or why it refused. The
/// driver would leave `marker`; a refusal must leave none.
fn dispatched(bundle: &Bundle, seat: &str, marker: &Path) -> Result<Value, String> {
    let SeatBody::Single { role_path, .. } = &bundle.seats[seat].body else {
        unreachable!("single seats")
    };
    let outcome = spawn_site(
        bundle,
        &SiteSpawn {
            charter: bundle.sites[seat].charter.clone(),
            ..SiteSpawn::inherit(vec!["touch".into(), marker.to_string_lossy().into()])
        },
        &json!({"role_path": role_path}),
        marker.parent().unwrap(),
        std::time::Duration::from_secs(5),
    )
    .map(|(_, input)| input[brokkr_protocol::native_controls::ROLE_TEXT].clone());
    if outcome.is_err() {
        assert!(
            !marker.exists(),
            "{seat}: no provider work before the refusal"
        );
    }
    outcome
}

/// The dispatch refusal for a charter of `owner` that moved for `cause`.
fn moved(owner: &str, cause: &str) -> Result<Value, String> {
    Err(format!(
        "dispatch refused: a charter of {owner} moved since the compile ({cause}); what a seat \
         is told must be the bytes the bundle's identity names (decision 0066 ruling 5)"
    ))
}

/// The fields of [`EngineError::CharterMoved`] for a charter of `owner` that
/// moved for `cause` (its text is pinned once, in `capability_tests`).
fn charter_moved(owner: &str, cause: &str) -> Result<String, (String, String)> {
    Err((owner.to_string(), cause.to_string()))
}

/// A door's run id, or the owner and key of its `CharterMoved`: any other
/// refusal fails the test by its variant (decision 0071 ruling 8).
fn answer(door: Door) -> Result<String, (String, String)> {
    match door {
        Ok(engine) => Ok(engine.run_id),
        Err(EngineError::CharterMoved { owner, key }) => Err((owner, key)),
        Err(other) => panic!("not a charter refusal: {other:?}"),
    }
}

/// What a start or resume door answers, before [`answer`] reads it.
type Door = Result<Engine, EngineError>;

/// What a start, a dispatch-bound start and `run`'s resume of `bundle`
/// answer, in that order, over the store under `root` (rebuild unit 24).
fn doors(root: &Path, bundle: &Bundle, run: &str, work: Option<&Path>) -> [Door; 3] {
    let (store, work) = (|| store_at(root), || work.map(Path::to_path_buf));
    let dispatch = super::tests::dispatch(bundle);
    [
        Engine::start(store(), bundle.clone(), "f", work()),
        Engine::start_with_dispatch(store(), bundle.clone(), "f", work(), dispatch),
        Engine::resume(store(), bundle.clone(), run, work()),
    ]
}

/// The store under `root`'s run count and `run`'s event ids: what a refused
/// door must leave as it found it.
fn written_in(root: &Path, run: &str) -> (usize, Vec<String>) {
    let store = store_at(root);
    let events = store.load(run).unwrap();
    let ids = events.into_iter().map(|event| event.event_id);
    (store.list_runs().unwrap().len(), ids.collect())
}

/// Rebuild unit 19 (design D7; task 19.1): A RUN IS NEITHER STARTED NOR
/// RESUMED OVER A CHARTER THAT MOVED SINCE THE COMPILE. One compiled bundle
/// whose `review` charter its layer owns and whose `work` charter the
/// external library owns; a run is started from it. Then, one at a time and
/// WITHOUT recompiling, each charter is changed, relinked into its owner's
/// `capabilities/` (a tree the walk excludes), relinked to an equal-byte twin
/// inside its owner, relinked to an equal-byte copy outside it, replaced at
/// its path by a new file of equal bytes (rebuild unit 24), and removed.
/// Every row refuses a new start, a dispatch-bound start and the run's
/// resume by the owner and the cause, and writes nothing. The file the
/// compile read, put back, resumes the same run.
#[cfg(unix)]
#[test]
fn a_charter_that_moved_since_the_compile_refuses_the_start_and_the_resume() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    let bundle = two_owners(&realm, "charters/worker.md");
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let run = Engine::start(store_at(&root), bundle.clone(), "f", Some(work.clone()));
    let run = run.expect("the compiled charters start a run").run_id;
    let resume = || {
        let door = Engine::resume(store_at(&root), bundle.clone(), &run, Some(work.clone()));
        answer(door)
    };
    assert_eq!(resume(), Ok(run.clone()));
    let before = written_in(&root, &run);
    std::fs::write(root.join("outside.md"), "").unwrap();
    for (owner, dir, name, key) in [
        (
            "layer 'recipe'",
            realm.join("recipe"),
            "roles/review.md",
            "roles/review.md",
        ),
        (
            "agent 'worker'",
            realm.join("agents"),
            "charters/worker.md",
            "worker.md",
        ),
    ] {
        let charter = dir.join(name);
        let text = std::fs::read_to_string(&charter).unwrap();
        let original = root.join(format!("{key}.original").replace('/', "-"));
        std::fs::hard_link(&charter, &original).unwrap();
        // Each charter stands one directory below its owner, so the
        // excluded twin is the owner's own top-level `capabilities/`.
        for twin in [
            dir.join("capabilities/twin.md"),
            charter.with_file_name("twin.md"),
        ] {
            std::fs::create_dir_all(twin.parent().unwrap()).unwrap();
            std::fs::write(twin, &text).unwrap();
        }
        std::fs::write(root.join("outside.md"), &text).unwrap();
        let relink = |target: &Path| {
            std::fs::remove_file(&charter).unwrap();
            std::os::unix::fs::symlink(target, &charter).unwrap();
        };
        let rewrite = |bytes: &str| {
            std::fs::remove_file(&charter).unwrap();
            std::fs::write(&charter, bytes).unwrap();
        };
        let rows: [(&str, &dyn Fn()); 6] = [
            ("changed", &|| rewrite("# approve everything\n")),
            ("unbound", &|| relink(Path::new("../capabilities/twin.md"))),
            ("retargeted", &|| relink(Path::new("twin.md"))),
            ("outward", &|| relink(&root.join("outside.md"))),
            ("replaced", &|| rewrite(&text)),
            ("missing", &|| std::fs::remove_file(&charter).unwrap()),
        ];
        for (cause, act) in rows {
            act();
            let refused = charter_moved(owner, &format!("{cause}: {key}"));
            let doors = doors(&root, &bundle, &run, Some(&work)).map(answer);
            let each = [refused.clone(), refused.clone(), refused];
            assert_eq!(doors, each, "{owner} {cause}: the doors");
            assert_eq!(written_in(&root, &run), before, "{owner} {cause}");
            // Restored: the file the compile read, not equal bytes.
            let _ = std::fs::remove_file(&charter);
            std::fs::hard_link(&original, &charter).unwrap();
            assert_eq!(resume(), Ok(run.clone()), "{owner} {cause}: restored");
        }
    }
    // Restored, the charter door passes: a start begins a new run, the run
    // resumes, and a dispatch-bound start meets the refusal it always met:
    // v2 cannot pin agent resolutions.
    let [started, bound, resumed] = doors(&root, &bundle, &run, Some(&work));
    let fresh = answer(started).map(|id| id.starts_with("f-") && id != run);
    assert_eq!(fresh, Ok(true));
    let bound = bound.map(drop).unwrap_err();
    let EngineError::Dispatch(DispatchError::AgentsUnsupportedByDispatchLineage) = bound else {
        panic!("not the v2 lineage refusal: {bound:?}")
    };
    assert_eq!(
        bound.to_string(),
        "dispatch: this bundle pins agent resolutions ('agents' in its manifest) and the \
         Looper-bound run-manifest/v2 lineage cannot carry them: the v2 round-trip \
         reconstructs the bundle manifest from six named keys, so the pin would be dropped \
         and the run would become unresumable. Run this bundle without --dispatch until a \
         jointly agreed v2-lineage manifest version exists"
    );
    assert_eq!(answer(resumed), Ok(run.clone()));
}

/// Rebuild unit 19, review return (F1): A RESUME IS HELD TO THE BINDINGS
/// ITS RUN STARTED OVER, NOT TO A RECOMPILE'S. The resume verb compiles the
/// bundle again, and a recompile binds each charter afresh: relinked to an
/// equal-byte twin already inside its owner, or replaced by a new file of
/// equal bytes, a charter compiles to the very manifest the run pinned. The
/// run's `run/started` event records each binding, and a resume over the
/// recompiled bundle refuses by the owner and the cause, and writes nothing;
/// the file the run started over, put back, resumes it. A run with no record
/// (started before one was kept) and a record naming a charter the bundle no
/// longer selects refuse too.
#[cfg(unix)]
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
)]
fn a_resume_over_a_recompile_is_held_to_the_bindings_the_run_started_over() {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    let realm = root.join("realm");
    two_owners(&realm, "charters/worker.md");
    let owners = [
        (
            "layer 'recipe'",
            realm.join("recipe/roles/review.md"),
            "roles/review.md",
            "roles/twin.md",
        ),
        (
            "agent 'worker'",
            realm.join("agents/charters/worker.md"),
            "worker.md",
            "charters/twin.md",
        ),
    ];
    // Each twin is there, with its charter's bytes, before the compile.
    for (_, charter, _, _) in &owners {
        std::fs::copy(charter, charter.with_file_name("twin.md")).unwrap();
    }
    let recompile = || {
        Bundle::compile_with_realm(
            &realm.join("recipe"),
            &realm.join("agents"),
            &realm.join("adapters"),
            None,
            None,
            brokkr_core::realms::Boundary::Namespace,
        )
        .expect("the recipe compiles")
    };
    let bundle = recompile();
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let run = Engine::start(store_at(&root), bundle.clone(), "f", Some(work.clone()))
        .expect("the compiled charters start a run")
        .run_id;
    let record = store_at(&root).load(&run).unwrap()[0].payload["charters"].clone();
    let recorded: Vec<(&str, &str, &str, &str, bool)> = record
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let binding = entry["binding"].as_str().unwrap();
            (
                entry["owner"].as_str().unwrap(),
                entry["reference"].as_str().unwrap(),
                entry["key"].as_str().unwrap(),
                entry["target"].as_str().unwrap(),
                binding.len() == 64 && binding.bytes().all(|b| b.is_ascii_hexdigit()),
            )
        })
        .collect();
    assert_eq!(
        recorded,
        [
            (
                "agent 'worker'",
                "charters/worker.md",
                "worker.md",
                "charters/worker.md",
                true
            ),
            (
                "layer 'recipe'",
                "roles/review.md",
                "roles/review.md",
                "roles/review.md",
                true
            ),
        ]
    );
    let resume = |run: &str| {
        let door = Engine::resume(store_at(&root), recompile(), run, Some(work.clone()));
        answer(door)
    };
    assert_eq!(resume(&run), Ok(run.clone()));
    let written = || written_in(&root, &run);
    let before = written();
    for (owner, charter, key, twin) in &owners {
        let original = root.join(format!("{key}.original").replace('/', "-"));
        std::fs::hard_link(charter, &original).unwrap();
        let text = std::fs::read(charter).unwrap();
        let rows: [(&str, &dyn Fn()); 2] = [
            ("retargeted", &|| {
                std::fs::remove_file(charter).unwrap();
                std::os::unix::fs::symlink("twin.md", charter).unwrap();
            }),
            ("replaced", &|| {
                std::fs::remove_file(charter).unwrap();
                std::fs::write(charter, &text).unwrap();
            }),
        ];
        for (cause, act) in rows {
            act();
            let moved = recompile();
            assert_eq!(
                moved.manifest, bundle.manifest,
                "{owner} {cause}: the recompile moves no identity"
            );
            let bound: std::collections::BTreeSet<&str> = moved
                .charters
                .values()
                .flatten()
                .map(|pin| pin.binding.keys()[1])
                .collect();
            let mut targets =
                std::collections::BTreeSet::from(["roles/review.md", "charters/worker.md"]);
            if cause == "retargeted" {
                targets.retain(|target| !target.ends_with(*key));
                targets.insert(twin);
            }
            assert_eq!(bound, targets, "{owner} {cause}: what the recompile bound");
            assert_eq!(
                resume(&run),
                charter_moved(owner, &format!("{cause}: {key}")),
                "{owner} {cause}: resume"
            );
            assert_eq!(written(), before, "{owner} {cause}: nothing is written");
            std::fs::remove_file(charter).unwrap();
            std::fs::hard_link(&original, charter).unwrap();
            assert_eq!(resume(&run), Ok(run.clone()), "{owner} {cause}: restored");
        }
    }
    // A run whose start recorded no bindings, one whose record lacks a
    // charter this bundle selects, and one whose record names a charter
    // this bundle does not select, planted beside it.
    let mut extra = record.clone();
    extra.as_array_mut().unwrap().push(
        json!({"owner": "layer 'gone'", "reference": "roles/gone.md", "key": "roles/gone.md",
               "target": "roles/gone.md", "binding": "0".repeat(64)}),
    );
    let mut partial = record.clone();
    partial
        .as_array_mut()
        .unwrap()
        .retain(|entry| entry["owner"] != "agent 'worker'");
    assert_eq!(
        partial.as_array().unwrap().len(),
        1,
        "only the layer's left"
    );
    for (id, charters, refused) in [
        (
            "f-unrecorded",
            None,
            charter_moved("agent 'worker'", "unrecorded: worker.md"),
        ),
        (
            "f-partial",
            Some(partial),
            charter_moved("agent 'worker'", "unrecorded: worker.md"),
        ),
        (
            "f-unselected",
            Some(extra),
            charter_moved(
                "bundle 'recipe'",
                "unselected: a charter the run started over",
            ),
        ),
    ] {
        let mut store = store_at(&root);
        let mut payload = json!({"feature": "f", "manifest": bundle.manifest});
        if let Some(charters) = charters {
            payload["charters"] = charters;
        }
        store
            .create_run(id, "f", &bundle.name, &bundle.manifest)
            .unwrap();
        store
            .append_next(id, EventType::RunStarted, payload, None, None)
            .unwrap();
        assert_eq!(resume(id), refused, "{id}");
    }
}

/// Rebuild unit 19, second review return (F1; operator ruling 2026-09-29,
/// point 1): NO GRANDFATHERING FOR AN UNRECORDED RUN, EVEN ONE THAT BINDS
/// NOTHING. A bundle with no charter still has a run started over it record
/// its empty set; a run whose `run/started` carries no record at all is
/// refused `unrecorded` by the bundle's name, and writes nothing, while the
/// run a start recorded resumes.
#[test]
fn a_run_that_recorded_no_bindings_is_refused_even_by_a_bundle_that_binds_none() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let bundle = bundle(&root, single_body(vec!["driver".into()]));
    assert!(bundle.charters.is_empty(), "the bundle binds no charter");
    let started = Engine::start(store_at(&root), bundle.clone(), "f", None)
        .unwrap()
        .run_id;
    let resume = |run: &str| answer(Engine::resume(store_at(&root), bundle.clone(), run, None));
    assert_eq!(resume(&started), Ok(started.clone()));
    let mut store = store_at(&root);
    store
        .create_run("legacy", "f", &bundle.name, &bundle.manifest)
        .unwrap();
    store
        .append_next(
            "legacy",
            EventType::RunStarted,
            json!({"feature": "f", "manifest": bundle.manifest}),
            None,
            None,
        )
        .unwrap();
    let unrecorded = "unrecorded: the run started with no charter record";
    assert_eq!(resume("legacy"), charter_moved("bundle 'test'", unrecorded));
    assert_eq!(store_at(&root).load("legacy").unwrap().len(), 1);
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn an_unboxed_exec_dispatch_is_refused_at_spawn_when_its_layer_moved() {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        // A nested box cannot open the namespace this proof needs. A host
        // that declared it must produce boundary evidence fails here
        // instead of printing `ok` (decision 0054 ruling 9).
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "this environment is already a box",
        );
        return;
    }
    let (dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    let (layer, pinned) = pinned_layer(dir.path());
    engine.bundle = pinned;
    let spawn = SiteSpawn {
        rewalk: Some(layer.join("scripts")),
        ..SiteSpawn::inherit(vec!["must-not-run".into()])
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn effect_started_carries_the_boundary_beside_provenance() {
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    // A plain bundle: no site has hands, no key.
    let body = single_body(vec!["driver".into()]);
    let (executable, _) = body.selected(None).unwrap();
    assert_eq!(engine.boundary_entries(executable, "work", true), None);

    // A gate-class boxed single seat under `harness`: one entry.
    engine.boundary = Boundary::Harness;
    super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
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
    engine.bundle.sites.clear();
    super::tests::set_site_hands(&mut engine.bundle, "design:validate", HandsSpec::default());
    super::tests::set_site_hands(
        &mut engine.bundle,
        "design:review:left",
        HandsSpec::default(),
    );
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
    super::tests::set_site_hands(&mut engine.bundle, "review:a", HandsSpec::default());
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
    super::tests::set_site_hands(&mut driven.bundle, "work", HandsSpec::default());
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
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
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
    super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
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
    super::tests::set_site_hands(&mut engine.bundle, "work", spec.clone());
    engine.boundary = Boundary::Open;
    assert_eq!(engine.site_boundary("work"), Some(Boundary::Open));
    Some(())
}

#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn a_panels_members_and_a_sequences_steps_carry_their_own_word() {
    // Panel members under `harness`: each member's checkpoints and the
    // engine's own `panel-member-finished` marker carry the member's
    // word; a member without hands carries the sentinel; the panel's
    // aggregate carries none.
    let (_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Harness;
    super::tests::set_site_hands(&mut engine.bundle, "work:boxed", HandsSpec::default());
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
    super::tests::set_site_hands(&mut engine.bundle, "work:second", HandsSpec::default());
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

    // Unknown confinement: no affirmative marker, under any boundary.
    for boundary in brokkr_core::realms::BOUNDARIES {
        engine.boundary = boundary;
        let input = marked(&engine, true, Some(&codex));
        assert_eq!(input, json!({"boundary": null, "hands": null}));
    }
    // A registered, resolved no-hands site names both fields
    // affirmatively, under every boundary.
    engine.bundle.sites.insert(
        "work".into(),
        SiteFacts {
            hands: HandsState::NoHands,
            ..Default::default()
        },
    );
    for boundary in brokkr_core::realms::BOUNDARIES {
        engine.boundary = boundary;
        let mut input = json!({});
        engine.mark_hands("work", &mut input);
        assert_eq!(
            input,
            json!({"boundary": "not applicable", "hands": "none"})
        );
    }
    engine.bundle.sites.insert(
        "work".into(),
        SiteFacts {
            hands: HandsState::Hands(HandsSpec::default()),
            ..Default::default()
        },
    );
    super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
    for boundary in [Boundary::Namespace, Boundary::Seatbelt, Boundary::Container] {
        engine.boundary = boundary;
        let input = marked(&engine, true, Some(&codex));
        assert_eq!(
            input,
            json!({"hands": "boxed", "boundary": boundary.word()})
        );
    }
    // `harness`: the word, no marker; the door only for a gate whose
    // link captures the final message.
    engine.boundary = Boundary::Harness;
    let input = marked(&engine, true, Some(&codex));
    assert_eq!(
        input,
        json!({"boundary": "harness", "hands": "none", "result_delivery": "last-message"})
    );
    let input = marked(&engine, true, Some(&filed));
    assert_eq!(input, json!({"boundary": "harness", "hands": "none"}));
    let input = marked(&engine, false, Some(&codex));
    assert_eq!(input, json!({"boundary": "harness", "hands": "none"}));
    let input = marked(&engine, true, None);
    assert_eq!(input, json!({"boundary": "harness", "hands": "none"}));
    // `open`: the word and nothing else.
    engine.boundary = Boundary::Open;
    let input = marked(&engine, true, Some(&codex));
    assert_eq!(input, json!({"boundary": "open", "hands": "none"}));

    // The requested input carries the word through `seat_input`, and a
    // panel member's derived input through `member_runs`.
    engine.boundary = Boundary::Harness;
    let seat = engine
        .seat_input(&state(Some("work"), Cursor::Idle), "work", "effect")
        .unwrap();
    assert_eq!(seat["boundary"], "harness");
    assert_eq!(seat["hands"], "none");
    super::tests::set_site_hands(&mut engine.bundle, "review:left", HandsSpec::default());
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
    assert_eq!(runs[1].input["boundary"], Value::Null);
    assert_eq!(runs[1].boundary, None);
}

// ─────────── the shipped Codex harness work seat keeps main's rejoin

/// The operator's 2026-09-15 ruling: the Codex `work-site` rejoin main
/// already performs under decision 0030 stays live across the drift
/// between its 0.148.0 measurement and the proof-exercised installed
/// 0.154.0, now named by both declared identity fields. This loads
/// the SHIPPED adapter through `Adapters::load` and composes the real
/// candidate under `harness`, so the declaration's `harness`/`none`
/// scope is checked against the argv and input facts the engine actually
/// builds. Reverting the shipped status to `unmeasured`, or moving the
/// declared hands back to `boxed`, breaks the driver proof that consumes
/// this composition; the boxed coordinate is composed here as the
/// negative, with its full MCP-bearing fragment, and is not covered by
/// the narrowed declaration.
#[test]
#[expect(clippy::too_many_lines, reason = "baseline 2026-09, #288")]
fn the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let adapters = Adapters::load(&root.join("adapters")).expect("the shipped adapters load");
    let library = Library::load(&root.join("agents")).expect("the shipped library loads");
    let hands = library
        .agent("reviewer")
        .expect("the shipped reviewer")
        .hands
        .clone();
    let shape = adapters
        .adapter("codex")
        .unwrap()
        .resume
        .shape("work-site")
        .expect("codex declares work-site");
    assert_eq!(shape.status.word(), "supported");
    assert_eq!(
        shape.boundaries,
        vec!["harness".to_string(), "not applicable".to_string()]
    );
    assert_eq!(shape.hands, "none");

    let _dir = tempfile::tempdir().unwrap();
    let workdir = tempfile::tempdir().unwrap();
    let result_path = workdir.path().join("results/fx.json");
    let result_path = result_path.to_str().unwrap();

    // Harness resolution is unboxed: no workspace MCP fragment travels.
    let report = crate::agents::report_under(
        &library,
        &adapters,
        &Availability::unspecified(),
        "reviewer",
        brokkr_core::realms::Boundary::Harness,
    )
    .unwrap();
    let resolution = crate::agents::resolve_report(report, &adapters).unwrap();
    let candidate = resolution
        .candidates
        .iter()
        .find(|candidate| candidate.provider == "codex")
        .expect("reviewer chains the codex lane");
    assert!(
        candidate.hands_fragment.is_empty(),
        "harness resolution appends no boxed fragment: {:?}",
        candidate.argv
    );
    let spawn = compose_site(
        BuiltBoundary::Harness,
        SeatClass::Work,
        candidate.argv.clone(),
        hands.as_ref(),
        Some(candidate),
        workdir.path(),
        &[],
        result_path,
        None,
    );
    assert_eq!(
        &spawn.argv[spawn.argv.len() - 2..],
        ["--sandbox", "workspace-write"],
        "the shipped harness.work fragment: {:?}",
        spawn.argv
    );
    assert!(
        !spawn
            .argv
            .iter()
            .any(|part| part.contains("mcp_servers.brokkr")),
        "a harness work seat serves no workspace tool: {:?}",
        spawn.argv
    );

    // The input facts the driver proof pins: the word, no boxed marker.
    let (_bundle_dir, mut engine) = super::tests::engine(single_body(vec!["driver".into()]));
    engine.boundary = Boundary::Harness;
    super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
    let seat = engine
        .seat_input(&state(Some("work"), Cursor::Idle), "work", "effect")
        .unwrap();
    assert_eq!(seat["boundary"], "harness");
    assert_eq!(seat["hands"], "none");

    // The boxed coordinate composes its complete MCP-bearing fragment and
    // the narrowed declaration does not cover it.
    let boxed_report = crate::agents::report_under(
        &library,
        &adapters,
        &Availability::unspecified(),
        "reviewer",
        brokkr_core::realms::Boundary::Namespace,
    )
    .unwrap();
    let boxed = crate::agents::resolve_report(boxed_report, &adapters).unwrap();
    let boxed_candidate = boxed
        .candidates
        .iter()
        .find(|candidate| candidate.provider == "codex")
        .expect("reviewer chains the codex lane");
    assert!(boxed_candidate
        .argv
        .ends_with(&boxed_candidate.hands_fragment));
    assert!(
        boxed_candidate
            .argv
            .iter()
            .any(|part| part.contains("mcp_servers.brokkr")),
        "the full boxed workspace argv: {:?}",
        boxed_candidate.argv
    );
    assert!(
        !shape.boundaries.iter().any(|word| word == "namespace"),
        "the boxed coordinate is not covered by the narrowed declaration"
    );
    engine.boundary = Boundary::Namespace;
    let mut boxed_input = json!({});
    engine.mark_hands("work", &mut boxed_input);
    assert_eq!(boxed_input["hands"], "boxed");
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
    super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
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
    engine.marks().door("work", true, Some(&codex), &mut input);
    assert_eq!(input["result_delivery"], "last-message");
}

#[test]
fn every_panel_spawn_rechecks_its_layer_and_journals_a_moved_member_failure() {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        // A nested box cannot open the namespace this proof needs. A host
        // that declared it must produce boundary evidence fails here
        // instead of printing `ok` (decision 0054 ruling 9).
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "this environment is already a box",
        );
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
            rewalk: Some(layer.join("scripts")),
            ..SiteSpawn::inherit(driver_command(
                "effect",
                "attempt",
                AttemptOutcome::Succeeded {
                    result: json!({"result":"pass"}),
                },
            ))
        },
        offer: None,
        context: None,
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
        super::tests::set_site_hands(&mut engine.bundle, "work", HandsSpec::default());
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
        // A nested box cannot open the namespace this proof needs. A host
        // that declared it must produce boundary evidence fails here
        // instead of printing `ok` (decision 0054 ruling 9).
        brokkr_protocol::hands::skip_boundary_proof(
            brokkr_protocol::hands::boundary_evidence_required(),
            "this environment is already a box",
        );
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
        rewalk: script_directory(&command, &engine.bundle.roots),
        ..SiteSpawn::inherit(driver_command(
            "effect",
            "attempt",
            AttemptOutcome::Succeeded {
                result: json!({"result":"complete"}),
            },
        ))
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
    // The contract named is the one this run's engine wrote: the 0.10
    // line reads v5 (proposed decision 0056 ruling 7's amendment to the
    // boundary-record dispatch). The boundary's own authority is
    // unchanged — `chroot` is not one of decision 0046's five words
    // under either version.
    assert!(
        failed.payload["error"]
            .as_str()
            .unwrap()
            .contains("seat-record.v5"),
        "{failed:?}"
    );
    for version in [
        brokkr_store::SeatRecordVersion::V4,
        brokkr_store::SeatRecordVersion::V5,
    ] {
        assert!(
            brokkr_store::validate_seat_record(
                &json!({"result":"complete", "model":"m", "boundary":"chroot"}),
                2,
                version,
            )
            .is_err(),
            "{version:?} refuses a word that is not the realm's"
        );
    }
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
            input["hands"],
            if boundary == Boundary::Namespace {
                json!("boxed")
            } else {
                json!("none")
            }
        );
        let prompt = brokkr_protocol::adapters::render_prompt(
            &input,
            brokkr_protocol::adapters::AdapterKind::Exec,
        )
        .unwrap();
        assert!(!prompt.contains("mcp__brokkr__workspace"));
        assert!(!prompt.contains("Your hands"));
        assert!(prompt.contains(input["result_path"].as_str().unwrap()));
    }
}

// ───────────────── issue #307: the engine smith's launch, as composed

const SMITH_POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["work", "review", "done", "stop"],
  "initial": "work",
  "terminal": ["done", "stop"],
  "shippable_from": ["review"],
  "rules": [
    {"id": "W-PASS", "from": "work", "result": "pass", "next": "review", "reason": "work concluded"},
    {"id": "W-FAIL", "from": "work", "result": "fail", "next": "stop", "reason": "work failed"},
    {"id": "R-OK", "from": "review", "result": "clean", "next": "done", "reason": "review concluded"}
  ]
}"#;

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A two-seat bundle under `dir` whose `work` seat hires `agent` as a
/// work-class office, compiled under `boundary` against `agents` and the
/// SHIPPED adapters. No provider is probed and nothing is spawned.
fn smith_bundle(dir: &Path, agents: &Path, agent: &str, boundary: Boundary) -> Bundle {
    let bundle = dir.join("bundle");
    std::fs::create_dir_all(bundle.join("roles")).unwrap();
    std::fs::write(bundle.join("policy.json"), SMITH_POLICY).unwrap();
    std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
    let config = json!({
        "name": "smith",
        "policy": "policy.json",
        "seats": {
            "work": {"results": ["pass", "fail"], "class": "work", "agent": agent},
            "review": {
                "role": "roles/role.md",
                "results": ["clean"],
                "driver": {"command": ["true"]},
            },
        }
    });
    std::fs::write(bundle.join("bundle.json"), config.to_string()).unwrap();
    Bundle::compile_under(&bundle, agents, &repository().join("adapters"), boundary)
        .unwrap_or_else(|error| panic!("the smith compiles under `{boundary}`: {error}"))
}

fn smith_site(bundle: &Bundle) -> (&[Candidate], &HandsSpec) {
    let SeatBody::Single { candidates, .. } = &bundle.seats["work"].body else {
        panic!("the smith is a single seat")
    };
    let hands = bundle.sites["work"]
        .hands_spec()
        .expect("the compiler recorded the smith's hands");
    (candidates, hands)
}

/// The SHIPPED engine smith compiled under `namespace` in a fresh root,
/// beside the checkout its launch runs in.
fn shipped_smith() -> (tempfile::TempDir, PathBuf, Bundle) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let workdir = root.join("checkout");
    std::fs::create_dir_all(workdir.join("results")).unwrap();
    let agents = repository().join("agents");
    let bundle = smith_bundle(&root, &agents, "implementer-engine", Boundary::Namespace);
    (dir, workdir, bundle)
}

/// One link of the smith's chain composed as its work site under
/// `boundary` by production `compose_site`, as the engine launches it.
fn compose_smith(
    boundary: BuiltBoundary,
    link: &Candidate,
    hands: &HandsSpec,
    workdir: &Path,
    bundle: &Bundle,
) -> SiteSpawn {
    let result_path = workdir.join("results/fx.json");
    compose_site(
        boundary,
        SeatClass::Work,
        link.argv.clone(),
        Some(hands),
        Some(link),
        workdir,
        &bundle.roots,
        result_path.to_str().unwrap(),
        None,
    )
}

fn engine_exe() -> String {
    std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// The box the shipped smith declares, written out by hand: the expected
/// side of every assertion below is a literal, never the serialiser.
fn assert_serves_the_smiths_box(served: &[String], workdir: &Path) {
    assert_eq!(served.len(), 6, "{served:?}");
    assert_eq!(
        served[..5],
        [
            "hands",
            "serve",
            "--workdir",
            workdir.to_str().unwrap(),
            "--spec"
        ],
        "{served:?}"
    );
    let policy: Value = serde_json::from_str(&served[5]).expect("the spec is JSON");
    assert_eq!(
        policy,
        json!({
            "kind": "workspace",
            "network": false,
            "binds": [
                {
                    "path": "~/.cargo",
                    "mode": "overlay",
                    "mask": ["credentials.toml", "credentials"]
                },
                {"path": "~/.rustup", "mode": "ro", "mask": []}
            ]
        })
    );
}

fn assert_no_token_is_left_unexpanded(argv: &[String]) {
    for token in [
        "{brokkr}",
        "{hands_mcp_json}",
        "{hands_args_toml}",
        "{result_path}",
    ] {
        assert!(
            !argv.iter().any(|part| part.contains(token)),
            "{token} in {argv:?}"
        );
    }
}

/// The shipped engine smith, compiled under `namespace` from the shipped
/// agent and adapters and composed by production `compose_site`, first
/// link: sol on Codex. The native sandbox is the read-only one
/// `hands.workspace` declares, the one writable surface is the MCP hands
/// server carrying the declared box, the harness work fragment is absent
/// and no tool-list flag appears anywhere. This is what the launch SAYS;
/// what Codex enforces natively is the controller's live measurement.
#[test]
fn the_shipped_engine_smith_launches_sol_read_only_with_the_boxed_hands_server() {
    let (_dir, workdir, bundle) = shipped_smith();
    assert_eq!(bundle.manifest["boundary"], json!({"work": "namespace"}));
    let (candidates, hands) = smith_site(&bundle);
    let sol = &candidates[0];
    assert_eq!(
        (sol.provider.as_str(), sol.model.as_str()),
        ("codex", "sol")
    );

    let spawn = compose_smith(BuiltBoundary::Namespace, sol, hands, &workdir, &bundle);
    let exe = engine_exe();
    assert_eq!(spawn.argv.len(), 16, "{:?}", spawn.argv);
    let served = spawn.argv[13]
        .strip_prefix("mcp_servers.brokkr.args=")
        .expect("the MCP server's arguments");
    // A TOML array of basic strings escaped with `\\` and `\"` alone is
    // also a JSON array, so it is decoded here without the serialiser.
    let served: Vec<String> = serde_json::from_str(served).expect("an array of strings");
    assert_serves_the_smiths_box(&served, &workdir);
    assert_eq!(
        spawn.argv,
        [
            exe.as_str(),
            "driver",
            "codex",
            "--",
            "--model",
            "gpt-6.1-sol",
            "--effort",
            "medium",
            "--sandbox",
            "read-only",
            "-c",
            &format!("mcp_servers.brokkr.command=\"{exe}\""),
            "-c",
            spawn.argv[13].as_str(),
            "-c",
            "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"",
        ]
    );
    assert!(spawn.refusal.is_none() && spawn.rewalk.is_none());
    for absent in [
        "workspace-write",
        "danger-full-access",
        "sandbox_mode",
        "--allowedTools",
        "--disallowedTools",
        "--tools",
        "Bash(",
    ] {
        assert!(
            !spawn.argv.iter().any(|part| part.contains(absent)),
            "{absent} in {:?}",
            spawn.argv
        );
    }
    assert_no_token_is_left_unexpanded(&spawn.argv);
}

/// The same compile's second link: fable on Claude. Its complete
/// workspace fragment survives in order, its one `--allowedTools` grant
/// is the MCP workspace tool and nothing of the retired Cargo/Git list,
/// and its MCP server carries the same box.
#[test]
fn the_shipped_engine_smith_falls_back_to_fable_with_the_mcp_grant_alone() {
    let (_dir, workdir, bundle) = shipped_smith();
    let (candidates, hands) = smith_site(&bundle);
    assert_eq!(candidates.len(), 2);
    let fable = &candidates[1];
    assert_eq!(
        (fable.provider.as_str(), fable.model.as_str()),
        ("claude", "fable")
    );

    let spawn = compose_smith(BuiltBoundary::Namespace, fable, hands, &workdir, &bundle);
    let exe = engine_exe();
    assert_eq!(spawn.argv.len(), 17, "{:?}", spawn.argv);
    let config: Value = serde_json::from_str(&spawn.argv[14]).expect("the MCP config is JSON");
    let server = &config["mcpServers"]["brokkr"];
    assert_eq!(
        config,
        json!({"mcpServers": {"brokkr": {"command": exe, "args": server["args"]}}}),
        "one server, and nothing beside its command and arguments"
    );
    let served: Vec<String> = serde_json::from_value(server["args"].clone()).unwrap();
    assert_serves_the_smiths_box(&served, &workdir);
    assert_eq!(
        spawn.argv,
        [
            exe.as_str(),
            "driver",
            "claude",
            "--",
            "--permission-mode",
            "acceptEdits",
            "--model",
            "claude-fable-5-1",
            "--effort",
            "high",
            "--tools",
            "",
            "--strict-mcp-config",
            "--mcp-config",
            spawn.argv[14].as_str(),
            "--allowedTools",
            "mcp__brokkr__workspace",
        ]
    );
    // The MCP config names `~/.cargo` as a bind, so it is read apart.
    for absent in ["Bash(", "cargo", "git", "workspace-write", "--sandbox"] {
        assert!(
            !spawn.argv[1..14]
                .iter()
                .chain(&spawn.argv[15..])
                .any(|part| part.contains(absent)),
            "{absent} in {:?}",
            spawn.argv
        );
    }
    assert_no_token_is_left_unexpanded(&spawn.argv);
}

/// H1's launch control, the separate unboxed path (decision 0046 ruling
/// 4). A Codex-only smith with the same hands is COMPILED under `harness`
/// and composed from that compile: exactly the declared writable
/// fragment, no workspace MCP registration and no tool list. The hands
/// stay on the record, and under this boundary Brokkr enforces none of
/// them — the harness's own sandbox is what stands.
#[test]
fn a_codex_only_smith_compiled_under_harness_launches_the_work_fragment_alone() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let agents = root.join("agents");
    std::fs::create_dir_all(agents.join("charters")).unwrap();
    std::fs::write(agents.join("charters/smith.md"), "# charter\n").unwrap();
    let mut smith: Value = serde_json::from_slice(
        &std::fs::read(repository().join("agents/implementer-engine.json")).unwrap(),
    )
    .unwrap();
    smith["charter"] = json!("charters/smith.md");
    smith["models"] = json!(["sol"]);
    smith["efforts"] = json!({"sol": "medium"});
    std::fs::write(agents.join("smith.json"), smith.to_string()).unwrap();
    let workdir = root.join("checkout");
    std::fs::create_dir_all(workdir.join("results")).unwrap();

    let bundle = smith_bundle(&root, &agents, "smith", Boundary::Harness);
    assert_eq!(bundle.boundary, Boundary::Harness);
    assert_eq!(bundle.manifest["boundary"], json!({"work": "harness"}));
    assert_eq!(bundle.manifest["hands"]["work"]["network"], json!(false));
    let (candidates, hands) = smith_site(&bundle);
    assert_eq!(hands.binds.len(), 2);
    let sol = &candidates[0];
    assert!(sol.hands_fragment.is_empty(), "{:?}", sol.argv);

    let spawn = compose_smith(BuiltBoundary::Harness, sol, hands, &workdir, &bundle);
    let exe = engine_exe();
    assert_eq!(
        spawn.argv,
        [
            exe.as_str(),
            "driver",
            "codex",
            "--",
            "--model",
            "gpt-6.1-sol",
            "--effort",
            "medium",
            "--sandbox",
            "workspace-write",
        ]
    );
}
