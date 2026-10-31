//! Task 46.1 (U9b1b; GP1, MB2–MB5, CR2, CR4, SD3): the enabling matrix
//! through real compile, dispatch, broker, fold and inspect. A refusal before
//! the store is read is pinned to no lookup and no server start; one after it
//! keeps its later cause. The harness reports success in every case.

use brokkr_protocol::broker::ledger::Failure;
use brokkr_protocol::broker::Refusal;
use serde_json::{json, Value};

use super::{answering, boxed, put, World, FRAMES, STARTS};

/// The server's marker, which only a started server leaves.
pub(super) const STARTED: &str = "server-started";

/// `world`'s run refused by `cause` before any lookup or server start, past
/// the harness's report of success, with no call row.
pub(super) fn refused_unread(
    world: &World,
    reads: crate::capability_broker::readiness::Reads,
    cause: &str,
) {
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.cause.as_deref()),
        ("awaiting_operator", Some(cause)),
        "{ended:?}"
    );
    assert_eq!(reads.lookups(), 0, "no store lookup");
    assert!(!world.base.join(STARTED).exists(), "no server start");
    assert_eq!(world.calls(&ended.run), []);
}

/// A server of `world` that marks its start, then starts and answers.
pub(super) fn marking(world: &World) {
    let mark = format!("touch '{}'\n", world.base.join(STARTED).display());
    world.server(&answering("").replacen("#!/bin/sh\n", &format!("#!/bin/sh\n{mark}"), 1));
}

/// MB3's program tree: a second link of the server's program, whoever owns
/// it, refuses at the box's own check.
#[test]
fn a_multiply_linked_program_refuses_with_no_lookup_or_start() {
    let Some(world) = World::new() else {
        return;
    };
    marking(&world);
    std::fs::hard_link(world.program(), world.program().with_extension("again")).unwrap();
    let reads = world.store_reads();
    refused_unread(&world, reads, &Refusal::Linked.to_string());
}

/// MB4: a store inside the seat's writable reach is reachable by its hands,
/// whatever mask the box puts over it. `.brokkr/state/secrets.env` is the
/// default store of a run with no `--secrets-file` (U9b1b's finding F2).
#[test]
fn a_store_in_the_worktree_refuses_with_no_lookup_or_start() {
    let Some(world) = World::new() else {
        return;
    };
    marking(&world);
    let store = world.repo.join(".brokkr/state/secrets.env");
    put(&store, format!("DOCS_TOKEN={}\n", super::TOKEN));
    super::chmod(&store, 0o600);
    for at in [".", ".brokkr", ".brokkr/state"] {
        super::chmod(&world.repo.join(at), 0o755);
    }
    let reads = crate::capability_broker::readiness::Reads::on(store);
    let out = world.brokkr(&["run", "--feature", "enabled"]).output();
    let ended = super::ended(out.unwrap());
    assert_eq!(
        (ended.status.as_str(), ended.cause),
        (
            "awaiting_operator",
            Some(Refusal::StoreReachable.to_string())
        )
    );
    assert_eq!(reads.lookups(), 0);
    assert!(!world.base.join(STARTED).exists());
}

/// MB4's reserved names through real compile and selected launch: a dialect
/// binding HOME or TMPDIR compiles, and its attempt refuses at provisioning
/// with the startup cause, no store lookup and no child start, its harness
/// never spawned. `DOCS_TOKEN` is the control in the next test.
#[test]
fn a_home_or_tmpdir_binding_refuses_the_launch_with_no_lookup_or_child() {
    for name in ["HOME", "TMPDIR"] {
        let Some(world) = World::new() else {
            return;
        };
        marking(&world);
        world.store(format!("{name}=value\n"));
        world.dialect(&json!({"secrets": [name]}));
        let reads = world.store_reads();
        refused_unread(&world, reads, &Refusal::StartupInputs.to_string());
        assert_eq!(world.starts(), 0, "{name}: no harness");
    }
}

/// The control beside those collisions: the server starts with MB4's fixed
/// names and its one binding, its HOME and TMPDIR private and empty, what it
/// reports relayed, and its bound value masked from everything it says.
#[test]
fn a_normal_binding_and_private_directories_reach_the_server() {
    let Some(world) = World::new() else {
        return;
    };
    let probe = r#"tr "\0" "\n" < /proc/$$/environ | cut -d= -f1 | sort | tr "\n" " " >&2
echo >&2
pwd >&2
ls -A "$TMPDIR" "$HOME" >&2
printf 'token %s\n' "$DOCS_TOKEN" >&2"#;
    world.server(&answering(probe));
    let ended = world.run();
    assert_eq!(ended.status, "completed", "{ended:?}");
    let said = std::fs::read_to_string(world.bin("broker.stderr")).unwrap();
    let relayed = |line: &str| format!("MCP server stderr: {line}\n");
    let names = "BROKKR_HANDS_BOX DOCS_TOKEN HOME LANG LC_ALL LOGNAME PATH TMPDIR USER ";
    let expected = [names, "/runtime/home", "/runtime/home:", "", "/tmp:"]
        .into_iter()
        .map(relayed)
        .collect::<String>();
    assert!(said.starts_with(&expected), "{said}");
    assert!(!said.contains(super::TOKEN), "masked: {said}");
}

/// GP1 at the harness: the same-name native powers stay OFF beside an MCP
/// holding, the hands are network-false, and the whole set is the hands and
/// the one broker, wanted as required.
#[test]
fn a_wanted_grant_is_served_beside_the_hands_with_native_power_off() {
    let Some(world) = World::new() else {
        return;
    };
    world.worker(Some(boxed()), "wants");
    let ended = world.run();
    assert_eq!(ended.status, "completed", "{ended:?}");
    let config = std::fs::read_to_string(world.bin("config.json")).unwrap();
    let config: Value = serde_json::from_str(&config).unwrap();
    let servers = config["mcpServers"].as_object().unwrap();
    let names: Vec<&str> = servers.keys().map(String::as_str).collect();
    assert_eq!(names, ["brokkr", "cap-library-docs"]);
    let hands = servers["brokkr"]["args"].as_array().unwrap();
    let spec = hands.iter().position(|arg| arg == "--spec").unwrap();
    assert_eq!(
        hands[spec + 1],
        r#"{"binds":[],"kind":"workspace","network":false}"#
    );
    let argv = std::fs::read_to_string(world.bin("argv")).unwrap();
    let argv: Vec<&str> = argv.lines().collect();
    let flag = |name: &str| argv[argv.iter().position(|arg| *arg == name).unwrap() + 1];
    assert_eq!(
        (flag("--tools"), flag("--disallowedTools")),
        ("", "WebFetch,WebSearch")
    );
}

/// CR4: a server that fails the protocol after its first call fails the
/// attempt with that cause past the harness's success, and keeps the one call
/// it took as failed, with no digest.
#[test]
fn a_fatal_protocol_failure_after_a_call_is_not_hidden_by_the_harness() {
    let Some(world) = World::new() else {
        return;
    };
    world.server(&format!("{STARTS}echo 'not a frame'\nread -r line\n"));
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.cause),
        ("awaiting_operator", Some(Failure::Protocol.to_string()))
    );
    assert_eq!(
        world.calls(&ended.run),
        [("resolve".to_string(), "failed".to_string(), None)]
    );
}

/// CR4's capacity through dispatch: a harness that makes 4,097 calls has
/// 4,096 settled, then the ledger's limit, past its report of success.
#[test]
fn the_call_past_the_ledgers_limit_fails_its_attempt_with_its_rows_kept() {
    let Some(world) = World::new() else {
        return;
    };
    let answers = r#"n=3
while :; do
  echo "{\"jsonrpc\":\"2.0\",\"id\":$n,\"result\":{\"content\":[]}}"
  n=$((n + 1))
  read -r line || exit 0
done
"#;
    world.server(&format!("{STARTS}{answers}"));
    let call = |id: usize| {
        format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"resolve","arguments":{{}}}}}}"#
        )
    };
    let head = FRAMES.lines().take(3).collect::<Vec<_>>().join("\n");
    let calls: Vec<String> = (0..4097).map(|at| call(at + 10)).collect();
    world.frames(&format!("{head}\n{}\n", calls.join("\n")));
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.cause),
        ("awaiting_operator", Some(Failure::Limit.to_string()))
    );
    let rows = world.calls(&ended.run);
    assert_eq!(rows.len(), 4096);
    assert!(rows.iter().all(|(tool, _, _)| tool == "resolve"));
}

/// CR2, MB4 on the real retention and inspect path: a server that answers
/// with the bound value as a scalar cannot be masked. The call fails with
/// that cause, its row has no digest, no body is retained or delivered, and
/// the bound value is in nothing the attempt left. A response holding it in
/// text is the control: it is masked and retained.
#[test]
fn a_scalar_secret_is_refused_and_no_unsafe_body_is_retained() {
    let Some(world) = World::new() else {
        return;
    };
    world.store("DOCS_TOKEN=424242424242\n");
    let scalar = r#"echo '{"jsonrpc":"2.0","id":3,"result":{"content":[],"count":424242424242}}'"#;
    world.server(&format!("{STARTS}{scalar}\nread -r line\n"));
    let ended = world.run();
    assert_eq!(
        (ended.status.as_str(), ended.cause),
        ("awaiting_operator", Some(Failure::Masking.to_string()))
    );
    assert_eq!(
        world.calls(&ended.run),
        [("resolve".to_string(), "failed".to_string(), None)]
    );
    let attempt = world.plan();
    let attempt = attempt.parent().unwrap();
    for left in std::fs::read_dir(attempt).unwrap().flatten() {
        let bytes = std::fs::read(left.path()).unwrap();
        let held = bytes.windows(12).any(|window| window == b"424242424242");
        assert!(!held, "{:?}", left.path());
        let name = left.file_name().into_string().unwrap();
        assert!(!name.ends_with(".response"), "no stage: {name}");
    }
    let heard = std::fs::read_to_string(world.bin("heard")).unwrap();
    assert!(!heard.contains("424242424242"), "{heard}");
}

/// The zero-grant sibling beside retained evidence: a boxed review seat that
/// strikes at the plan, its ledger and the attempt's directory reaches none
/// of them, and every retained byte stands.
#[test]
fn a_boxed_sibling_cannot_alter_the_attempts_evidence() {
    let Some(world) = World::new() else {
        return;
    };
    let args = world.run_args();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let ended = super::ended(world.brokkr(&args).output().unwrap());
    assert_eq!(ended.status, "completed", "{ended:?}");
    let plan = world.plan();
    let ledger = plan.with_extension("json.ledger");
    let held = |path: &std::path::Path| std::fs::read(path).unwrap();
    let before = (held(&plan), held(&ledger));
    let attempt = plan.parent().unwrap().display();
    let strike = format!(
        "ls '{attempt}' >/dev/null 2>&1 && touch '{}'; echo forged > '{}' 2>/dev/null; \
         rm -rf '{attempt}' 2>/dev/null; true",
        world.base.join("reached").display(),
        ledger.display(),
    );
    world.review_runs(&strike);
    let second = world.run();
    assert_eq!(second.status, "completed", "{second:?}");
    assert!(!world.base.join("reached").exists(), "kept apart");
    assert_eq!(before, (held(&plan), held(&ledger)));
}
