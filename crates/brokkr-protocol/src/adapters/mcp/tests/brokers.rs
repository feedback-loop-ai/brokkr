//! U7d (MB1, MB4, SI2) at the serving edge, against hand-sealed serving
//! inputs as U7a's suite seals them: a brokered set beside the box's hands
//! is served to Claude's grammar exactly as an independent literal of the
//! whole set, cold, rejoining and as the cold replacement; every departure
//! refuses at the final boundary; Codex and dsh, which carry no broker,
//! refuse; and the spawn removes exactly the sealed removals. The engine's
//! own composition of a brokered set is U7d2's.

use super::*;
use crate::adapters::{invoke_with_stager, stage_prompt, AdapterKind, Pinned};
use crate::env_guard::EnvGuard;

/// The Claude workspace fragment `adapters/claude.json` measures.
const FRAGMENT: [&str; 7] = [
    "--tools",
    "",
    "--strict-mcp-config",
    "--mcp-config",
    "{hands_mcp_json}",
    "--allowedTools",
    "mcp__brokkr__workspace",
];

/// The set sealing two brokers beside the hands: `library-docs` and `web`,
/// the second sharing one removal with the first.
fn brokers() -> Value {
    json!({"brokered": [
        {"capability": "library-docs", "plan": "/state/plans/library-docs.json",
         "plan_digest": "a".repeat(64), "removals": ["LIBRARY_DOCS_TOKEN"]},
        {"capability": "web", "plan": "/state/plans/web.json",
         "plan_digest": "b".repeat(64), "removals": ["WEB_TOKEN", "LIBRARY_DOCS_TOKEN"]},
    ]})
}

/// The whole set's document, written out by hand: the box's hands as the
/// hands-only literal names them, then each broker run by the same
/// executable on its own plan alone, every name sorted.
fn document(brokkr: &str) -> String {
    document_at(brokkr, "/w", &[("library-docs", 'a'), ("web", 'b')])
}

/// [`document`] for the hands bound to `workdir` and `brokers`, each by its
/// capability and its plan digest's one repeated character.
fn document_at(brokkr: &str, workdir: &str, brokers: &[(&str, char)]) -> String {
    let hands = format!(
        r#"["hands","serve","--workdir","{workdir}","--artifacts","/a","--reach","{{\"home\":\"/h\",\"worktree\":{{\"path\":\"/w\",\"identity\":null}},\"git\":null,\"binds\":[],\"store\":null}}","--spec","{{\"binds\":[],\"kind\":\"workspace\",\"network\":false}}"]"#
    );
    let broker = |(name, digest): &(&str, char)| {
        format!(
            r#","cap-{name}":{{"args":["broker","serve","--plan","/state/plans/{name}.json","--plan-digest","{}"],"command":"{brokkr}"}}"#,
            digest.to_string().repeat(64)
        )
    };
    let brokers: String = brokers.iter().map(broker).collect();
    format!(r#"{{"mcpServers":{{"brokkr":{{"args":{hands},"command":"{brokkr}"}}{brokers}}}}}"#)
}

/// This test binary, the executable a sealed launch binds its servers to.
fn exe() -> String {
    std::env::current_exe()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// A launch of `provider` dispatched with the box's hands and `servers`
/// beside them, its handed hands fragment carrying `document`, the intent
/// assessed as `assessments` past the lifted fence: its handed arguments
/// and its input.
fn brokered(
    provider: &str,
    servers: Value,
    document: &str,
    assessments: [Value; 3],
) -> (Vec<String>, Value) {
    authored_beside(&[], provider, servers, document, assessments)
}

/// [`brokered`] with the recipe's `authored` words leading the handed
/// arguments.
fn authored_beside(
    authored: &[String],
    provider: &str,
    servers: Value,
    document: &str,
    assessments: [Value; 3],
) -> (Vec<String>, Value) {
    let mut seal = Seal::authored(authored.len());
    let mut hands = seal.hands(&FRAGMENT);
    hands[4] = document.to_string();
    let mut plan = claude_plan(&[], &[], &["WebSearch", "WebFetch"]);
    (plan["local"], plan["hands"]) = (json!([]), json!(hands.len()));
    (plan["provider"], plan["harness"]) = (json!(provider), json!(provider));
    let mut input = json!({"workdir": "/w", "native_controls": plan});
    input["launch_arguments"] = json!({"authored": authored, "managed": hands});
    let extra = [authored.to_vec(), hands].concat();
    let input = sealed_pair(input, &extra, seal);
    let mut input = intended(input, "hands", assessments);
    input[SERVING_INPUTS]["isolation"]["servers"] = servers;
    (extra, input)
}

/// U9b1b's ruling F1 at the driver's own loop: a start sealed with brokers
/// whose driver cannot confine itself refuses with no `accepted` and no spawn;
/// a start sealed with none never asks, whatever asking would answer.
#[test]
fn a_brokered_start_confines_its_driver_first_and_refuses_where_it_cannot() {
    use crate::adapters::serve_with;
    use crate::{Body, Message};
    fn refuses() -> std::io::Result<()> {
        Err(std::io::ErrorKind::PermissionDenied.into())
    }
    let _env = EnvGuard::lock();
    let line = |body: Body| serde_json::to_string(&Message::new(body)).unwrap();
    let drive = |input: Value| -> Vec<Value> {
        let hello = line(Body::Hello {
            engine_version: "test".into(),
        });
        let start = line(Body::Start {
            effect_id: "e".into(),
            attempt_id: "a".into(),
            seat: "work".into(),
            input,
        });
        let mut output = Vec::new();
        let frames = format!("{hello}\n{start}\n");
        serve_with(
            AdapterKind::Claude,
            &[],
            frames.as_bytes(),
            &mut output,
            refuses,
        )
        .unwrap();
        let said = String::from_utf8(output).unwrap();
        let said = said.lines().map(|said| serde_json::from_str(said).unwrap());
        said.collect()
    };
    let measured = || json!("measured");
    let assessments = [measured(), measured(), measured()];
    let (_, input) = brokered("claude", brokers(), &document(&exe()), assessments);
    let answers = drive(input.clone());
    let types: Vec<&str> = answers
        .iter()
        .map(|said| said["type"].as_str().unwrap())
        .collect();
    assert_eq!(types, ["capabilities", "result"]);
    assert_eq!(answers[1]["status"], "failed");
    assert_eq!(
        answers[1]["error"],
        "seat refused to start: its driver could not set no_new_privs before a harness the \
         engine brokers MCP for: permission denied"
    );
    let mut hands_only = input;
    hands_only[SERVING_INPUTS]["isolation"]["servers"] = json!("hands");
    let answers = drive(hands_only);
    let result = answers.iter().find(|said| said["type"] == "result");
    // Past a refusing `Confine`, to the launch's own next step.
    assert_eq!(
        result.unwrap()["error"],
        "seat refused to start: charter '' is unreadable: No such file or directory (os error 2)",
        "{answers:?}"
    );
}

/// The names in force for the attempt in progress.
fn recorded_removals() -> Vec<String> {
    REMOVED.with_borrow(Clone::clone)
}

/// MB1 for Claude's grammar: the cold launch carries the hands and both
/// brokers exactly as the independent literal names them, in the measured
/// fragment's place; LaneTally's wrapper is served the same set. A rejoin
/// keeps it and adds only its selector, and a declined rejoin's cold
/// replacement is the cold command. The attempt's removals are each sealed
/// name once, in force only while the attempt is.
#[test]
fn a_brokered_set_is_served_cold_rejoining_and_replacing_as_its_literal() {
    let removals = s(&["LIBRARY_DOCS_TOKEN", "WEB_TOKEN"]);
    let (_, input) = brokered("claude", brokers(), &document(&exe()), cold_only());
    let attempt = Removing::attempt(&input);
    assert_eq!(recorded_removals(), removals);
    drop(attempt);
    assert_eq!(recorded_removals(), Vec::<String>::new());
    let denied = s(&["--disallowedTools", "WebSearch,WebFetch"]);
    let set = |document: &str| {
        let parts = [&FRAGMENT[..4], &[document], &FRAGMENT[5..]].concat();
        s(&parts)
    };
    for (provider, shape) in [("claude", CLAUDE_SHAPE), ("lanetally", LANETALLY_SHAPE)] {
        let (extra, input) = brokered(provider, brokers(), &document(&exe()), cold_only());
        let launch = claude_launch(provider, &extra, None, &input, shape, None);
        let expected = command(provider, &[set(&document(&exe())), denied.clone()]);
        assert_eq!(launch.map(|plan| plan.command), Ok(expected), "{provider}");
    }
    let dir = tempfile::tempdir().unwrap();
    let bin = claude_shim(&dir.path().canonicalize().unwrap(), "2.1.287");
    let session = "019c4b7e-0000-7000-8000-000000000001";
    let assessment = enabled_input(CLAUDE_SHAPE, "2.1.287", Path::new("/w"));
    let offered = |resume: Value| {
        let measured = [json!("measured"), json!("measured"), resume];
        let (extra, mut input) = brokered("claude", brokers(), &document(&exe()), measured);
        for key in ["boundary", "hands", "resume_context"] {
            input[key] = assessment[key].clone();
        }
        let plan = claude_launch(&bin, &extra, Some(session), &input, CLAUDE_SHAPE, None);
        let plan = plan.unwrap();
        (plan.command, plan.rejoining, plan.refusal)
    };
    let cold = command(&bin, &[set(&document(&exe())), denied]);
    let rejoin = [cold.clone(), s(&["--resume", session])].concat();
    assert_eq!(
        offered(json!("measured")),
        (rejoin, Some(session.to_string()), None)
    );
    assert_eq!(
        offered(json!("unmeasured")),
        (cold, None, Some("restrictions-unavailable"))
    );
}

/// Every departure from the sealed set refuses at the final boundary with
/// the final-set cause before anything spawns: a broker missing, one extra, a name written twice, another executable,
/// plan or digest, the strict isolation dropped, a broker list that names
/// none, and brokers beside no hands.
#[test]
fn each_departure_from_the_brokered_set_refuses_at_the_final_boundary() {
    let refused = |servers: Value, document: &str| {
        let (extra, input) = brokered("claude", servers, document, cold_only());
        let launch = claude_launch("claude", &extra, None, &input, CLAUDE_SHAPE, None);
        launch.map(|plan| plan.command)
    };
    let not_sealed = McpRefusal::NotSealed { provider: "claude" }.at_launch();
    let whole = document(&exe());
    // `whole` closes its table and itself with `}}`; `web` is its last entry.
    let (body, web) = (
        &whole[..whole.len() - 2],
        whole.find(r#","cap-web""#).unwrap(),
    );
    let entry = &body[web..];
    let library = format!("{}}}}}", &whole[..web]);
    let hands = format!(
        "{}}}}}",
        &whole[..whole.find(r#","cap-library-docs""#).unwrap()]
    );
    let x = r#","cap-x":{"args":["broker","serve"],"command":"x"}"#;
    for (case, document) in [
        ("a broker missing", library),
        ("the hands alone", hands),
        ("a broker extra", format!("{body}{x}}}}}")),
        ("a name written twice", format!("{body}{entry}}}}}")),
        ("another executable", whole.replace(&exe(), "/usr/bin/npx")),
        (
            "another plan",
            whole.replace("plans/web.json", "plans/other.json"),
        ),
        (
            "another digest",
            whole.replacen(&"b".repeat(64), &"c".repeat(64), 1),
        ),
    ] {
        assert_eq!(
            refused(brokers(), &document),
            Err(not_sealed.clone()),
            "{case}"
        );
    }
    // The served command itself, its strict isolation dropped or another
    // document appended after the check's input was composed.
    let (extra, input) = brokered("claude", brokers(), &whole, cold_only());
    let cold = claude_launch("claude", &extra, None, &input, CLAUDE_SHAPE, None);
    let (cold, edge) = (cold.unwrap().command, Edge::new(&input));
    isolated("claude", &edge, &extra, false).unwrap();
    let chosen = crate::native_controls::Serving {
        program: "claude",
        workdir: "/w",
        ..Default::default()
    };
    let mut stripped = cold.clone();
    stripped.retain(|part| part != "--strict-mcp-config");
    let appended = [
        cold.clone(),
        s(&["--mcp-config", "/home/operator/.claude.json"]),
    ]
    .concat();
    for (case, command) in [("strict dropped", stripped), ("appended", appended)] {
        assert_eq!(
            served("claude", command, &extra, &edge, chosen),
            Err(not_sealed.clone()),
            "{case}"
        );
    }
    assert_eq!(
        served("claude", cold.clone(), &extra, &edge, chosen),
        Ok(cold)
    );
    // An empty broker list is the hands set spelt again, and brokers beside
    // no sealed hands have nothing to stand beside: neither is declared.
    let unboxed = intended(json!({"workdir": "/w"}), "hands", cold_only());
    let mut unboxed = unboxed;
    unboxed[SERVING_INPUTS]["isolation"]["servers"] = brokers();
    let (_, mut emptied) = brokered("claude", brokers(), &whole, cold_only());
    emptied[SERVING_INPUTS]["isolation"]["servers"] = json!({"brokered": []});
    for (case, input) in [("no broker", emptied), ("no hands", unboxed)] {
        let refusal = isolated("claude", &Edge::new(&input), &extra, false).map(|i| i.argv);
        assert_eq!(
            refusal,
            Err(McpRefusal::NotSealed { provider: "claude" }),
            "{case}"
        );
    }
    // A capability its builder cannot name as one bare server name.
    let mut unbare = brokers();
    unbare["brokered"][1]["capability"] = json!("web search");
    assert_eq!(refused(unbare, &whole), Err(not_sealed.clone()));
    // A broker sealed with a member beside its four is no intent at all
    // (moved here from U7a's serving-edge test).
    let mut counterfeit = brokers();
    counterfeit["brokered"][0]["command"] = json!("npx");
    let (_, input) = brokered("claude", counterfeit, &whole, cold_only());
    let unread = isolated("claude", &Edge::new(&input), &extra, false).map(|i| i.argv);
    assert_eq!(unread, Err(McpRefusal::Unreadable));
}

/// The recipe may not configure a server, even one byte for byte the
/// engine's: the whole brokered set written as the recipe's own words
/// refuses before any provider work (decision 0066 ruling 4).
#[test]
fn an_authored_copy_of_the_sealed_set_refuses() {
    let whole = document(&exe());
    let authored = s(&["--strict-mcp-config", "--mcp-config", &whole]);
    let (extra, input) = authored_beside(&authored, "claude", brokers(), &whole, cold_only());
    let launch = claude_launch("claude", &extra, None, &input, CLAUDE_SHAPE, None);
    assert_eq!(
        launch.map(|plan| plan.command),
        Err(
            "refusing to invoke the agent CLI: the seat's arguments carry '--strict-mcp-config' \
             more than once, and the CLI resolves a duplicate last-wins against the current \
             restriction plan the engine composed (proposed decision 0056 ruling 6)"
                .to_string()
        )
    );
}

/// Codex's measured fragment and dsh's engine-only home carry no broker,
/// so a brokered set refuses for either before any provider work, under
/// the standing fence as past the lifted one, never served without its
/// brokers; Claude's grammar is served it under both.
#[test]
fn codex_and_dsh_refuse_a_brokered_set_whichever_way_the_fence_stands() {
    let measured = [json!("measured"), json!("measured"), json!("measured")];
    for fence in ["standing", "lifted"] {
        let (extra, mut input) = brokered("claude", brokers(), &document(&exe()), measured.clone());
        input[SERVING_INPUTS]["isolation"]["fence"] = json!(fence);
        let edge = Edge::new(&input);
        for provider in ["codex", "dsh"] {
            let refusal = isolated(provider, &edge, &extra, false).map(|i| i.argv);
            assert_eq!(
                refusal,
                Err(unmeasured_cause(provider, "brokers")),
                "{provider}, {fence}"
            );
        }
        let claude = isolated("claude", &edge, &extra, false).map(|i| i.argv);
        assert_eq!(claude, Ok(extra.clone()), "{fence}");
    }
}

/// MB4 at the spawn: within an attempt, its harness is spawned without each
/// sealed removal — exported by the operator, set by the driver or bound
/// from the store alike — and so is any server the harness spawns in turn,
/// while every other name reaches both.
#[test]
fn the_spawn_removes_each_sealed_removal_from_the_harness_and_its_servers() {
    let mut env = EnvGuard::lock();
    env.set("LIBRARY_DOCS_TOKEN", "exported");
    env.set("U7D_KEPT", "kept");
    let (_, input) = brokered("claude", brokers(), &document(&exe()), cold_only());
    let _attempt = Removing::attempt(&input);
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().to_str().unwrap();
    let bound = [super::super::super::tests::binding(
        "WEB_TOKEN",
        "bound-value",
    )];
    let set = [("LIBRARY_DOCS_TOKEN", "set"), ("U7D_SET", "set")];
    let names = ["LIBRARY_DOCS_TOKEN", "WEB_TOKEN", "U7D_KEPT", "U7D_SET"];
    let spawned = |program: &[&str]| {
        let child = crate::adapters::spawn_harness(
            &s(program),
            workdir,
            std::process::Stdio::null(),
            std::process::Stdio::piped(),
            &set,
            &bound,
        );
        let output = child.unwrap().wait_with_output().unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        let mut names: Vec<String> = (text.lines())
            .filter_map(|line| line.split_once('=').map(|(name, _)| name.to_string()))
            .filter(|name| names.contains(&name.as_str()))
            .collect();
        names.sort();
        names
    };
    let reached = s(&["U7D_KEPT", "U7D_SET"]);
    assert_eq!(spawned(&["/usr/bin/env"]), reached, "the harness");
    let server = ["/bin/sh", "-c", "/usr/bin/env"];
    assert_eq!(spawned(&server), reached, "a server the harness spawns");
    drop(env);
}

/// The names each spawn of a [`watching`] harness reports, every one of
/// them exported into this test's environment.
const WATCHED: [&str; 3] = ["U7D_DIALECT_A", "U7D_DIALECT_B", "U7D_KEPT"];

/// A synthetic Claude reporting `2.1.287`, as the review chief's was: each
/// spawn logs to `log`, by its role (`probe` for the version probe, `model`
/// otherwise), `<role>:<name>` for each [`WATCHED`] name it holds and
/// `<role>:resume` where it was handed `--resume`. No provider is run.
fn watching(dir: &Path, log: &Path) -> String {
    let log = log.display();
    let held: String = (WATCHED.iter())
        .map(|name| format!("[ -n \"${{{name}+x}}\" ] && echo \"$role:{name}\" >> {log}\n"))
        .collect();
    let body = format!(
        "#!/bin/sh\nrole=model\n[ \"$1\" = --version ] && role=probe\n{held}\
         case \" $* \" in *\" --resume \"*) echo \"$role:resume\" >> {log} ;; esac\n\
         [ $role = probe ] && {{ echo '2.1.287 (Claude Code)'; exit 0; }}\n\
         cat > /dev/null\nexit 0\n"
    );
    executable(dir, "claude", &body)
        .to_str()
        .unwrap()
        .to_string()
}

/// One brokered holding of an attempt: its capability, its plan digest's
/// repeated character and its removals.
type Holding<'a> = (&'a str, char, &'a [&'a str]);

/// One Claude attempt through the invocation every seat's attempt is
/// (`invoke_with_stager`), offered `session` or none, in a fresh workdir:
/// its hands and `holdings` sealed beside them, or the hands alone where
/// there are none, every shape measured and its resume assessment enabled
/// for the harness's version. The lines its spawns logged, in order.
fn attempt(holdings: &[Holding<'_>], session: Option<&str>) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path().canonicalize().unwrap();
    let (workdir, log) = (dir.to_str().unwrap(), dir.join("spawns.log"));
    let bin = watching(&dir, &log);
    let servers = match holdings {
        [] => json!("hands"),
        _ => json!({"brokered": (holdings.iter()).map(|(name, digest, removals)| json!({
            "capability": name, "plan": format!("/state/plans/{name}.json"),
            "plan_digest": digest.to_string().repeat(64), "removals": removals,
        })).collect::<Vec<_>>()}),
    };
    let named: Vec<(&str, char)> = (holdings.iter()).map(|(name, d, _)| (*name, *d)).collect();
    let document = document_at(&exe(), workdir, &named);
    let measured = [json!("measured"), json!("measured"), json!("measured")];
    let (extra, mut input) = brokered("claude", servers, &document, measured);
    let assessment = enabled_input(CLAUDE_SHAPE, "2.1.287", &dir);
    for key in ["boundary", "hands", "resume_context", "workdir"] {
        input[key] = assessment[key].clone();
    }
    let pinned = Pinned {
        binary: &bin,
        extra: &extra,
    };
    let (prompt, emit) = ("prompt", &mut |_: &Value| {});
    let claude = AdapterKind::Claude;
    let invoked = invoke_with_stager(
        claude,
        pinned,
        prompt,
        &input,
        session,
        &[],
        emit,
        stage_prompt,
    );
    assert_eq!(invoked.map(|invocation| invocation.exit_code), Ok(0));
    let logged = std::fs::read_to_string(&log).unwrap();
    logged.lines().map(str::to_string).collect()
}

/// Every [`WATCHED`] name exported into the environment the guard holds.
fn exported() -> EnvGuard {
    let mut env = EnvGuard::lock();
    for name in WATCHED {
        env.set(name, "exported");
    }
    env
}

/// The two holdings sealing both dialect names, one each.
const BOTH: [Holding<'static>; 2] = [
    ("library-docs", 'a', &["U7D_DIALECT_A"]),
    ("web", 'b', &["U7D_DIALECT_B"]),
];

/// `role`'s line for each of `names`, in order.
fn held(role: &str, names: &[&str]) -> Vec<String> {
    names.iter().map(|name| format!("{role}:{name}")).collect()
}

/// H1 (MB4; decision 0077 ruling 3): a first cold attempt whose resume
/// assessment is enabled probes the harness's version before its final
/// command is checked, and that probe, like the model, holds none of the
/// attempt's sealed names.
#[test]
fn a_first_cold_attempt_probes_and_launches_without_its_sealed_names() {
    let env = exported();
    assert_eq!(
        attempt(&BOTH, None),
        s(&["probe:U7D_KEPT", "model:U7D_KEPT"])
    );
    drop(env);
}

/// H1 for a qualified rejoin: the probe that qualifies it and the rejoining
/// model both hold none of the attempt's sealed names.
#[test]
fn a_qualified_rejoin_probes_and_rejoins_without_its_sealed_names() {
    let env = exported();
    let session = "019c4b7e-0000-7000-8000-000000000001";
    assert_eq!(
        attempt(&BOTH, Some(session)),
        s(&["probe:U7D_KEPT", "model:U7D_KEPT", "model:resume"])
    );
    drop(env);
}

/// H1 across attempts: each spawn removes exactly its own attempt's names,
/// none of the attempt before it, and none outlive the attempt.
#[test]
fn each_attempt_removes_exactly_its_own_names_and_none_outlive_it() {
    let env = exported();
    let first = attempt(&[("library-docs", 'a', &["U7D_DIALECT_A"])], None);
    assert_eq!(recorded_removals(), Vec::<String>::new(), "after the first");
    let second = attempt(&[("web", 'b', &["U7D_DIALECT_B"])], None);
    let kept = |name: &str| {
        let names = [name, "U7D_KEPT"];
        [held("probe", &names), held("model", &names)].concat()
    };
    assert_eq!(
        (first, second),
        (kept("U7D_DIALECT_B"), kept("U7D_DIALECT_A"))
    );
    drop(env);
}

/// H1 with no brokers: an attempt sealing the box's hands alone, after one
/// sealing both names, removes nothing from its probe or its model.
#[test]
fn an_attempt_with_no_brokers_removes_nothing() {
    let env = exported();
    attempt(&BOTH, None);
    let every = [held("probe", &WATCHED), held("model", &WATCHED)].concat();
    assert_eq!(attempt(&[], None), every);
    drop(env);
}
