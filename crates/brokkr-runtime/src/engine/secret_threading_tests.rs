use super::*;
// The two-seat table and the adapter tree the bundle compiles against,
// shared with the compiler's own secret-binding tests: one granted driver
// under a name no vendor answers to (decision 0021 ruling 4).
use crate::bundle::secret_binding_tests::{write_adapters, POLICY};

/// An engine over a compiled two-seat bundle; `work` optionally
/// declares secret bindings.
fn engine_with(dir: &std::path::Path, secrets: Option<Vec<&str>>) -> Engine {
    let bundle_dir = dir.join("bundle");
    std::fs::create_dir_all(bundle_dir.join("roles")).unwrap();
    std::fs::write(bundle_dir.join("policy.json"), POLICY).unwrap();
    std::fs::write(bundle_dir.join("roles/role.md"), "# role\n").unwrap();
    let mut work = json!({
        "role": "roles/role.md",
        "results": ["built"],
        "driver": {"command": ["{brokkr}", "driver", "granted", "--", "true"]},
    });
    if let Some(secrets) = secrets {
        work["secrets"] = json!(secrets);
    }
    let config = json!({
        "name": "threading",
        "policy": "policy.json",
        "seats": {
            "work": work,
            "review": {
                "role": "roles/role.md",
                "results": ["clean"],
                "driver": {"command": ["true"]},
            },
        }
    });
    std::fs::write(
        bundle_dir.join("bundle.json"),
        serde_json::to_string(&config).unwrap(),
    )
    .unwrap();
    let bundle = Bundle::compile_with(
        &bundle_dir,
        std::path::Path::new("/nonexistent-library"),
        &write_adapters(dir),
    )
    .unwrap();
    let store = Store::open(&dir.join("forge.db")).unwrap();
    Engine::start(store, bundle, "threading proof", Some(dir.join("work"))).unwrap()
}

fn work_input(engine: &Engine) -> Value {
    let state = fold(&engine.store.load(&engine.run_id).unwrap()).unwrap();
    engine.seat_input(&state, "work", "fx").unwrap()
}

#[test]
fn declared_names_and_store_path_ride_the_driver_input() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let engine = engine_with(dir.path(), Some(vec!["GH_TOKEN", "API_KEY"]));
    let input = work_input(&engine);
    assert_eq!(input["secrets"], json!(["GH_TOKEN", "API_KEY"]));
    let store_path = input["secrets_file"].as_str().unwrap();
    let store_path = std::path::Path::new(store_path);
    assert!(
        store_path.ends_with(std::path::Path::new(".forge").join("secrets.env")),
        "default store path under the workdir: {}",
        store_path.display()
    );
    assert!(
        store_path.starts_with(dir.path().join("work")),
        "workdir-relative default: {}",
        store_path.display()
    );
}

#[test]
fn secrets_file_override_wins_over_the_default() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let mut engine = engine_with(dir.path(), Some(vec!["GH_TOKEN"]));
    engine.secrets_file = Some(dir.path().join("elsewhere.env"));
    let input = work_input(&engine);
    assert_eq!(
        input["secrets_file"].as_str().unwrap(),
        dir.path().join("elsewhere.env").to_str().unwrap()
    );
}

#[test]
fn a_folder_run_threads_the_store_in_its_state() {
    // Decision 0083 ruling 3: handed the folder's state, the default
    // store is the state's, and no `.forge` is joined.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let mut engine = engine_with(dir.path(), Some(vec!["GH_TOKEN"]));
    engine.state = Some(dir.path().join(".brokkr/state"));
    let input = work_input(&engine);
    assert_eq!(
        input["secrets_file"].as_str().unwrap(),
        dir.path()
            .join(".brokkr/state/secrets.env")
            .to_str()
            .unwrap()
    );
}

#[test]
fn seats_without_bindings_carry_no_secret_keys() {
    // Pre-0012 bundles must rebuild byte-identical seat inputs, or
    // resumed runs would refuse on a digest mismatch.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("work")).unwrap();
    let engine = engine_with(dir.path(), None);
    let input = work_input(&engine);
    assert!(input.get("secrets").is_none(), "{input}");
    assert!(input.get("secrets_file").is_none(), "{input}");
}

/// Decision 0065 slice two, U9c2 (45.5): a Codex seat binding no secret is
/// handed the store's path its permission profile denies, with an empty list
/// of names that carries both to each member and step, wherever its body
/// runs Codex: an inline command, an agent's candidate, a panel member or a
/// sequence step. Its launch denies exactly that store. A body running no
/// Codex is handed neither.
#[test]
fn a_codex_seat_binding_nothing_is_handed_the_store_its_profile_denies() {
    use super::capability_tests::link;
    use super::tests::{draft_then_finish, engine, member, single_body};
    use crate::bundle::{Aggregate, DialectExecution, SeatClass, SequenceStep, StepBody};
    let argv = |kind: &str| -> Vec<String> {
        ["/bin/brokkr", "driver", kind, "--"]
            .map(String::from)
            .to_vec()
    };
    let check = SequenceStep {
        name: "check".into(),
        class: SeatClass::Work,
        results: Vec::new(),
        body: StepBody::Dialect {
            execution: DialectExecution {
                argv: Vec::new(),
                state: None,
            },
        },
    };
    let sequence = |kind: &str| SeatBody::Sequence {
        steps: [
            vec![check.clone()],
            draft_then_finish("role.md", argv(kind), &[], Vec::new()),
        ]
        .concat(),
    };
    // A candidate is Codex's by the harness its command dispatches, never by
    // its provider's name: a renamed Codex fallback behind a Claude primary
    // is, and a Claude command on a provider named `codex` is not.
    let agent = |links: [(&str, &str); 2]| {
        let mut body = single_body(argv("claude"));
        if let SeatBody::Invocation(Invocation::Single { candidates, .. }) = &mut body {
            let link = |(provider, kind)| Candidate {
                argv: argv(kind),
                ..link(provider, "astra")
            };
            candidates.extend(links.map(link));
        }
        body
    };
    let renamed = agent([("claude", "claude"), ("zeta", "codex")]);
    let misnamed = agent([("claude", "claude"), ("codex", "claude")]);
    let panel = SeatBody::Invocation(Invocation::Panel {
        members: vec![member("a", argv("claude")), member("b", argv("codex"))],
        aggregate: Aggregate::UnanimousPass,
    });
    let codex = [
        single_body(argv("codex")),
        renamed,
        panel,
        sequence("codex"),
    ];
    for (body, runs) in codex.into_iter().map(|body| (body, true)).chain([
        (single_body(argv("claude")), false),
        (misnamed, false),
        (sequence("dsh"), false),
    ]) {
        let (_dir, engine) = engine(body);
        let input = work_input(&engine);
        let store = engine.secrets_store_path().to_string_lossy().into_owned();
        let handed = (input.get("secrets"), input.get("secrets_file"));
        match runs {
            true => assert_eq!(handed, (Some(&json!([])), Some(&json!(store)))),
            false => assert_eq!(handed, (None, None), "{input}"),
        }
    }
    let (_dir, engine) = engine(single_body(argv("codex")));
    let input = work_input(&engine);
    let store = engine.secrets_store_path().to_string_lossy().into_owned();
    let class = ["--sandbox".to_string(), "workspace-write".to_string()];
    let launched = brokkr_protocol::adapters::codex_command("codex", &class, "/w", None, &input);
    let profile = format!(
        r#"permissions.brokkr={{filesystem={{":root"="read","/w"="write","{store}"="deny"}}}}"#
    );
    let isolation = [
        "--ignore-user-config",
        "--disable",
        "apps",
        "--disable",
        "plugins",
    ];
    let expected = [
        ["codex", "exec", "--json", "-C", "/w", "-c"]
            .map(String::from)
            .to_vec(),
        vec![
            profile,
            "-c".into(),
            r#"default_permissions="brokkr""#.into(),
        ],
        isolation.map(String::from).to_vec(),
    ];
    assert_eq!(launched, Ok(expected.concat()));
}

/// U9c2: a Codex seat whose workdir or store path is not valid UTF-8 is
/// refused, naming which, before a lossy spelling reaches its profile.
#[cfg(target_os = "linux")]
#[test]
fn a_codex_seat_refuses_a_run_path_no_text_can_name() {
    use super::input::{RunPath, Unspelled};
    use super::tests::{engine, single_body};
    use std::os::unix::ffi::OsStrExt;
    let root = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(root.path()).unwrap();
    let named = |bytes: &[u8]| root.join(std::ffi::OsStr::from_bytes(bytes));
    let refused = |engine: &Engine| {
        let state = fold(&engine.store.load(&engine.run_id).unwrap()).unwrap();
        match engine.seat_input(&state, "work", "fx") {
            Err(EngineError::Unspelled(unspelled)) => Ok(unspelled),
            other => Err(other
                .map(|input| input.to_string())
                .map_err(|e| e.to_string())),
        }
    };
    let codex = ["/bin/brokkr", "driver", "codex", "--"].map(String::from);
    let (_dir, mut engine) = engine(single_body(codex.to_vec()));
    let store = named(b"secrets-\xff.env");
    engine.secrets_file = Some(store.clone());
    let unspelled = refused(&engine).unwrap();
    let path = store.clone();
    assert_eq!(
        unspelled,
        Unspelled {
            which: RunPath::Store,
            path
        }
    );
    let text = format!(
        "the secrets store's path '{}/secrets-\u{fffd}.env' is not valid UTF-8, so a seat's \
         input could only name another path; the seat is refused before it is requested \
         (decision 0065 slice two, U9c2)",
        root.display()
    );
    assert_eq!(EngineError::from(unspelled).to_string(), text);
    engine.secrets_file = None;
    engine.repo = named(b"work-\xff");
    let path = engine.repo.clone();
    let unspelled = Unspelled {
        which: RunPath::Workdir,
        path,
    };
    assert_eq!(refused(&engine), Ok(unspelled.clone()));
    let text = format!(
        "the seat's workdir '{}/work-\u{fffd}' is not valid UTF-8, so a seat's input could \
         only name another path; the seat is refused before it is requested (decision 0065 \
         slice two, U9c2)",
        root.display()
    );
    assert_eq!(EngineError::from(unspelled).to_string(), text);
}

#[test]
fn brokkr_runtime_production_code_never_touches_the_secret_store() {
    // Grep-style invariant: resolution lives in the exec driver, so
    // no store-reading function is ever named in brokkr-runtime's
    // production code (test modules after #[cfg(test)] excluded).
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in std::fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let production = source.split("#[cfg(test)]").next().unwrap();
        for needle in ["resolve_bindings", "read_store", "store_names", "store_set"] {
            assert!(
                !production.contains(needle),
                "{} names secret-store function '{needle}' in production code",
                path.display()
            );
        }
    }
}
