#![cfg(unix)]

//! `brokkr driver agy`, end to end (#1200): Agy's example adapter in a
//! world that lists its office, a compiled seat hiring `agy-neon`, the
//! engine's own start, and the real driver over a fake `agy` that prints one
//! of Antigravity CLI 1.5's recorded turns; and the driver's gate mode as
//! the engine HOME's hook runs it.

use std::io::Write;
use std::path::{Path, PathBuf};

use brokkr_core::realms::Boundary;
use brokkr_protocol::process::DriverStderr;
use brokkr_runtime::bundle::RealmLaw;
use brokkr_runtime::{Bundle, Engine};
use brokkr_store::Store;
use serde_json::{json, Value};

use crate::qoder_driver::env_guard::EnvGuard;
use crate::qoder_driver::{checkpoints, patch, root, write, write_mode};

const NEON: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/named-model-neon-base.ndjson");
const TOOL_LESS: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/tool-less-turn.ndjson");

const SIGNED_OUT: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/signed-out.ndjson");
const SIGNED_OUT_STDERR: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/signed-out.stderr");
const PRINT_TIMEOUT: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/print-timeout.ndjson");
const PRINT_TIMEOUT_STDERR: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/print-timeout.stderr");

/// How the fake `agy` ends a turn: its stdout, its stderr, its exit, and
/// whether it `delivers` the seat's result.
struct Ending<'a> {
    stdout: &'a str,
    stderr: &'a str,
    code: i32,
    delivers: bool,
}

/// A fake `agy` that logs its argv, its HOME and the engine's two
/// configuration files there, writes the seat's result where the prompt's
/// one stdin line names it if `ending` delivers, prints its stdout and
/// stderr, and exits its code. A seat stopped after its CLI delivered and
/// exited on its own ends late, not refused (#1178 H3), so a stream the
/// driver stops delivers nothing.
fn fake_agy(root: &Path, ending: &Ending<'_>) -> PathBuf {
    write(&root.join("turn.ndjson"), ending.stdout.as_bytes());
    write(&root.join("turn.stderr"), ending.stderr.as_bytes());
    let undelivered = if ending.delivers { "" } else { "target=\n" };
    let script = format!(
        r#"#!/bin/sh
printf '%s\n' "$@" > '{root}/argv.log'
printf '%s' "$HOME" > '{root}/home.log'
cat "$HOME/.gemini/config/mcp_config.json" > '{root}/mcp.log'
cat "$HOME/.gemini/config/hooks.json" > '{root}/hooks.log'
line=$(cat)
target=$(printf '%s' "$line" | sed -n 's/.*\\n    \(\/[^\\"]*\.json\)\\n.*/\1/p' | head -1)
{undelivered}[ -n "$target" ] && printf '{{"result":"complete"}}' > "$target"
cat '{root}/turn.stderr' >&2
cat '{root}/turn.ndjson'
exit {code}
"#,
        root = root.display(),
        code = ending.code
    );
    // Written before the engine starts, in a process that runs this test
    // alone, so no forked child holds it open for writing when it runs.
    let path = root.join("agy");
    write_mode(&path, script.as_bytes(), 0o755);
    path
}

/// Agy's example adapter copied as the guide ships it, its own `mcp` block
/// included, a `scout` office hiring `agy-neon` at `low` with one grant,
/// a recipe whose work seat it holds, and the operator's login under
/// `root`, which is the operator's HOME.
fn world(root: &Path) -> PathBuf {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (from, to) in [
        ("docs/examples/adapters/agy.json", "adapters/agy.json"),
        ("adapters/exec.json", "adapters/exec.json"),
    ] {
        write(&root.join(to), &std::fs::read(shipped.join(from)).unwrap());
    }
    let login = ".gemini/antigravity-cli/antigravity-oauth-token";
    write_mode(&root.join(login), b"the operator's login", 0o600);
    write(&root.join("agents/charters/scout.md"), b"# scout\n");
    let office = json!({
        "description": "the agy scout", "charter": "charters/scout.md",
        "models": ["agy-neon"], "efforts": {"agy-neon": "low"},
        "tools": {"allow": ["git"]},
        "limits": {"max_attempts": 1, "timeout_seconds": 20},
    });
    write(
        &root.join("agents/scout.json"),
        office.to_string().as_bytes(),
    );
    let recipe = root.join("recipe");
    let policy = json!({
        "phases": ["scout", "review", "done"], "initial": "scout", "terminal": ["done"],
        "rules": [
            {"id": "scouted", "from": "scout", "result": "complete", "next": "review", "reason": "s"},
            {"id": "reviewed", "from": "review", "result": "complete", "next": "done", "reason": "r"},
        ],
    });
    write(&recipe.join("policy.json"), policy.to_string().as_bytes());
    // A provisional model holds no gate: the review is a command.
    let review = json!({"command": ["{brokkr}", "driver", "exec", "--", "true"]});
    let bundle = json!({
        "name": "agy", "policy": "policy.json",
        "seats": {
            "scout": {"results": ["complete"], "agent": "scout"},
            "review": {"results": ["complete"], "driver": review},
        },
    });
    write(&recipe.join("bundle.json"), bundle.to_string().as_bytes());
    recipe
}

/// The recipe compiled where the world lists `scout`, run once over a fake
/// `agy` that prints `stdout`, delivers where it `delivers`, and exits 0:
/// the journal's events.
fn engine_run(root: &Path, stdout: &str, delivers: bool) -> Vec<Value> {
    let ending = Ending {
        stdout,
        stderr: "",
        code: 0,
        delivers,
    };
    engine_run_ending(root, &ending)
}

/// [`engine_run`] over a fake `agy` that ends as `ending` says.
fn engine_run_ending(root: &Path, ending: &Ending<'_>) -> Vec<Value> {
    let recipe = world(root);
    let law = RealmLaw {
        boundary: Boundary::Namespace,
        provisional_offices: vec!["scout".to_string()],
    };
    let (agents, adapters) = (root.join("agents"), root.join("adapters"));
    let compiled = Bundle::compile_under(&recipe, &agents, &adapters, law);
    let mut bundle = compiled.expect("an agy seat in a listed office compiles");
    patch(&mut bundle);
    let mut env = EnvGuard::lock();
    env.set("BROKKR_AGY_BIN", fake_agy(root, ending));
    env.set("HOME", root);
    std::fs::create_dir_all(root.join("work")).unwrap();
    let store = Store::open(&root.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "agy", Some(root.join("work"))).unwrap();
    eprintln!("the drive ended {:?}", engine.drive());
    let events = engine.store.load(&engine.run_id).unwrap();
    let events = events
        .iter()
        .map(|event| serde_json::to_value(event).unwrap());
    events.inspect(|event| eprintln!("{event}")).collect()
}

fn logged(root: &Path, name: &str) -> String {
    std::fs::read_to_string(root.join(name)).unwrap()
}

#[test]
fn an_engine_run_of_an_agy_seat_answers_on_its_pin_under_an_engine_home() {
    // In an engine of its own (#403): an engine is its attempts' subreaper.
    let test = "agy_driver::an_engine_run_of_an_agy_seat_answers_on_its_pin_under_an_engine_home";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let (_home, root) = root();
    let events = engine_run(&root, NEON, true);
    let settings = r#"{"permissions":{"allow":["command(git)"]}}"#;
    assert_eq!(
        logged(&root, "argv.log").lines().collect::<Vec<_>>(),
        [
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--mode",
            "accept-edits",
            "--model",
            "gemini-4-neon",
            "--effort",
            "low",
            "--settings",
            settings,
        ]
    );
    // The engine's HOME held the empty set and the gate, and is gone.
    assert_eq!(logged(&root, "mcp.log"), r#"{"mcpServers":{}}"#);
    let hooks: Value = serde_json::from_str(&logged(&root, "hooks.log")).unwrap();
    let hook = &hooks["brokkr-gate"]["PreToolUse"][0]["hooks"][0];
    let gate = format!(
        "'{}' 'driver' 'agy' '--' '--gate' 'read_file' 'write_file' 'edit_file' 'run_command' \
         'manage_task'",
        env!("CARGO_BIN_EXE_brokkr")
    );
    assert_eq!(hook["command"], gate);
    let home = logged(&root, "home.log");
    assert_ne!(Path::new(&home), root);
    assert!(!Path::new(&home).exists(), "{home}");
    let finished = checkpoints(&events)
        .into_iter()
        .find(|row| row["step"] == "agy-cli-session-finished")
        .expect("the seat finished");
    let figures = [
        "model",
        "effort",
        "input_tokens",
        "output_tokens",
        "reasoning_output_tokens",
    ];
    assert_eq!(
        figures.map(|key| finished[key].clone()),
        [
            json!("gemini-4-neon"),
            json!("not reported"),
            json!(8875),
            json!(21),
            json!(19)
        ]
    );
    let none = json!({"kind": "none", "locator": "", "home": ""});
    assert_eq!(finished["transcript"], none);
    for absent in [
        "total_cost_usd",
        "cache_write_tokens",
        "root_session",
        "capture",
    ] {
        assert_eq!(finished.get(absent), None, "{absent}");
    }
    let succeeded = events
        .iter()
        .any(|event| event["type"] == "effect/succeeded");
    assert!(succeeded, "the seat's effect succeeded");
}

#[test]
fn an_engine_run_of_an_agy_seat_served_another_model_journals_no_turn() {
    let test = "agy_driver::an_engine_run_of_an_agy_seat_served_another_model_journals_no_turn";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let (_home, root) = root();
    // The driver stops the stream at its init, so the fake delivers nothing.
    let events = engine_run(&root, TOOL_LESS, false);
    assert_eq!(checkpoints(&events), Vec::<&Value>::new());
    let failures: Vec<&Value> = (events.iter())
        .filter(|event| event["type"] == "effect/failed")
        .collect();
    let [failed] = failures[..] else {
        panic!("not one failed effect: {failures:?}");
    };
    let error = failed["payload"]["error"].as_str().unwrap();
    let words = "model_substituted: the seat pinned \"gemini-4-neon\" and agy served \
                 \"gemini-3.8-flash-low\"";
    assert!(error.contains(words), "{error}");
    assert_eq!(failed["payload"]["start_failure"], true);
}

/// A signed-out agy, end to end: the driver process's own output, which
/// the engine journals as the attempt's stderr by evidence (#1199), is the
/// driver's words in place of agy's sign-in block, and the journal holds no
/// piece of its URL (#1200 K8).
#[test]
fn an_engine_run_of_a_signed_out_agy_journals_no_piece_of_the_sign_in_url() {
    let test = "agy_driver::an_engine_run_of_a_signed_out_agy_journals_no_piece_of_the_sign_in_url";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let (_home, root) = root();
    // The recorded sign-in line, its link built from pieces no journal
    // holds otherwise: its host, each path segment and each query value.
    let host = "qz7signin.brokkr-host.test";
    let segments = ["o7kpath", "zq4auth"];
    let values = ["xq7offline", "qv81client", "wz3challenge", "kq5state"];
    let query = ["access_type", "client_id", "code_challenge", "state"]
        .iter()
        .zip(values)
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    let built = format!("https://{host}/{}?{query}", segments.join("/"));
    let recorded = SIGNED_OUT_STDERR
        .lines()
        .find(|line| line.contains("https://"));
    let link = recorded.unwrap().trim();
    let stderr = SIGNED_OUT_STDERR.replace(link, &built);
    assert!(stderr.contains(&built) && !stderr.contains(link));
    let ending = Ending {
        stdout: SIGNED_OUT,
        stderr: &stderr,
        code: 1,
        delivers: true,
    };
    let journal = serde_json::to_string(&engine_run_ending(&root, &ending)).unwrap();
    // The journal names the driver's printed stderr by its evidence alone
    // (#1199): these are the evidence of the driver's own words.
    let printed = "agy wrote its sign-in request on stderr, which brokkr does not keep: it holds \
                   a link made for one attempt (#1200)\n";
    let named = DriverStderr::of(printed).to_string();
    assert!(
        journal.contains(&named),
        "the driver's printed stderr is journaled as {named}: {journal}"
    );
    assert!(
        journal.contains(
            "not_logged_in: agy's result had status ERROR; Antigravity CLI is signed out"
        ),
        "{journal}"
    );
    let scheme_and_host = format!("https://{host}");
    let pieces = [host, scheme_and_host.as_str()].into_iter();
    let pieces = pieces.chain(segments).chain(values);
    let journaled: Vec<&str> = pieces.filter(|piece| journal.contains(piece)).collect();
    assert_eq!(journaled, Vec::<&str>::new());
}

/// A print timeout behind work, end to end: the attempt ends with no
/// result, and what the driver prints for it is the print-timeout refusal's
/// words, which the engine journals by evidence (#1200 K9, #1199).
#[test]
fn an_engine_run_of_an_agy_print_timeout_journals_its_words() {
    let test = "agy_driver::an_engine_run_of_an_agy_print_timeout_journals_its_words";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let (_home, root) = root();
    let stdout = PRINT_TIMEOUT.replace("gemini-3.8-flash-low", "gemini-4-neon");
    let ending = Ending {
        stdout: &stdout,
        stderr: PRINT_TIMEOUT_STDERR,
        code: 3,
        delivers: true,
    };
    let events = engine_run_ending(&root, &ending);
    let journal = serde_json::to_string(&events).unwrap();
    let words = "print_timeout: agy's result had status ERROR; agy exited 3 at its own print \
                 timeout, which this launch never asks for";
    // The journal names what the driver printed by its evidence alone
    // (#1199): these are the evidence of the refusal's words.
    let printed = format!("{words}; the command was spawned, so no result is sent\n");
    let named = DriverStderr::of(&printed).to_string();
    assert!(journal.contains(&named), "{named}: {journal}");
    let succeeded = (events.iter()).any(|event| event["type"] == "effect/succeeded");
    assert!(!succeeded, "{journal}");
}

/// The gate mode `brokkr driver agy -- --gate <name>...` answers one hook
/// call: a listed tool is left to agy's rules, another is denied by name.
#[test]
fn the_gate_mode_answers_one_hook_call_by_the_tools_own_name() {
    let answer = |name: &str| {
        let mut gate = std::process::Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(["driver", "agy", "--", "--gate", "read_file", "run_command"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let call = json!({"hookEventName": "PreToolUse", "toolCall": {"name": name}});
        let mut stdin = gate.stdin.take().unwrap();
        stdin.write_all(call.to_string().as_bytes()).unwrap();
        drop(stdin);
        let output = gate.wait_with_output().unwrap();
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    };
    assert_eq!(
        answer("run_command"),
        "{\"decision\":\"deny_unless_prior_grant\"}\n"
    );
    assert_eq!(
        answer("search_web"),
        "{\"decision\":\"deny\",\"reason\":\"the engine's gate does not offer the tool \
         search_web\"}\n"
    );
}
