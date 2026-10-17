//! The engine smith's launch as composed (issue #307), moved here from
//! `boundary_tests.rs` when its box's hands gained the artifact root the
//! engine carries (decision 0065 slice two, U8a).

use super::*;

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
    let SeatBody::Invocation(Invocation::Single { candidates, .. }) = &bundle.seats["work"].body
    else {
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
        Path::new("/a"),
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
    assert_eq!(served.len(), 8, "{served:?}");
    let carried = ["--workdir", workdir.to_str().unwrap(), "--artifacts", "/a"];
    let head = [&["hands", "serve"][..], &carried, &["--spec"]].concat();
    assert_eq!(served[..7], head, "{served:?}");
    let policy: Value = serde_json::from_str(&served[7]).expect("the spec is JSON");
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
