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
fn seed_agy_adapters_and_login(root: &Path) {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (from, to) in [
        ("docs/examples/adapters/agy.json", "adapters/agy.json"),
        ("adapters/exec.json", "adapters/exec.json"),
    ] {
        write(&root.join(to), &std::fs::read(shipped.join(from)).unwrap());
    }
    let login = ".gemini/antigravity-cli/antigravity-oauth-token";
    write_mode(&root.join(login), b"the operator's login", 0o600);
    write_scout_agent(root);
}

fn world(root: &Path) -> PathBuf {
    seed_agy_adapters_and_login(root);
    let recipe = root.join("recipe");
    let (policy, bundle) = standard_policy_and_bundle("agy");
    write(&recipe.join("policy.json"), policy.to_string().as_bytes());
    write(&recipe.join("bundle.json"), bundle.to_string().as_bytes());
    recipe
}

fn write_scout_agent(root: &Path) {
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
}

fn standard_policy_and_bundle(name: &str) -> (Value, Value) {
    let policy = json!({
        "phases": ["scout", "review", "done"], "initial": "scout", "terminal": ["done"],
        "rules": [
            {"id": "scouted", "from": "scout", "result": "complete", "next": "review", "reason": "s"},
            {"id": "reviewed", "from": "review", "result": "complete", "next": "done", "reason": "r"},
        ],
    });
    // A provisional model holds no gate: the review is a command.
    let review = json!({"command": ["{brokkr}", "driver", "exec", "--", "true"]});
    let bundle = json!({
        "name": name, "policy": "policy.json",
        "seats": {
            "scout": {"results": ["complete"], "agent": "scout"},
            "review": {"results": ["complete"], "driver": review},
        },
    });
    (policy, bundle)
}

fn fake_agy_with_kept_transcript(root: &Path, uuid: &str, log_home: bool) -> PathBuf {
    write(&root.join("turn.ndjson"), NEON.as_bytes());
    write(&root.join("transcript.src"), PRINTED_TRANSCRIPT.as_bytes());
    let home_log = if log_home {
        format!("printf '%s' \"$HOME\" > '{}/home.log'\n", root.display())
    } else {
        String::new()
    };
    let script = format!(
        r#"#!/bin/sh
{home_log}logs="$HOME/.gemini/antigravity-cli/brain/{uuid}/.system_generated/logs"
mkdir -p "$logs"
cp '{root}/transcript.src' "$logs/transcript.jsonl"
chmod 0644 "$logs/transcript.jsonl"
line=$(cat)
target=$(printf '%s' "$line" | sed -n 's/.*\\n    \(\/[^\\"]*\.json\)\\n.*/\1/p' | head -1)
[ -n "$target" ] && printf '{{"result":"complete"}}' > "$target"
cat '{root}/turn.ndjson'
exit 0
"#,
        root = root.display(),
    );
    let bin = root.join("agy");
    write_mode(&bin, script.as_bytes(), 0o755);
    bin
}

fn drive_recipe(
    root: &Path,
    recipe: &Path,
    bin: &Path,
    feature: &str,
) -> (
    String,
    Result<brokkr_runtime::DriveEnd, brokkr_runtime::EngineError>,
    Vec<Value>,
) {
    let law = RealmLaw {
        boundary: Boundary::Namespace,
        provisional_offices: vec!["scout".to_string()],
    };
    let (agents, adapters) = (root.join("agents"), root.join("adapters"));
    let mut bundle = Bundle::compile_under(recipe, &agents, &adapters, law)
        .unwrap_or_else(|e| panic!("{feature}: bundle compiles: {e}"));
    patch(&mut bundle);
    let mut env = EnvGuard::lock();
    env.set("BROKKR_AGY_BIN", bin);
    env.set("HOME", root);
    std::fs::create_dir_all(root.join("work")).unwrap();
    let store = Store::open(&root.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, feature, Some(root.join("work"))).unwrap();
    let run_id = engine.run_id.clone();
    let outcome = engine.drive();
    let events = engine
        .store
        .load(&run_id)
        .unwrap()
        .iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .collect();
    (run_id, outcome, events)
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
    let bin = fake_agy(root, ending);
    let (_run_id, outcome, events) = drive_recipe(root, &recipe, &bin, "agy");
    eprintln!("the drive ended {outcome:?}");
    for event in &events {
        eprintln!("{event}");
    }
    events
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
    let gate = crate::built_shell_line!([
        "driver",
        "agy",
        "--",
        "--gate",
        "read_file",
        "write_file",
        "edit_file",
        "run_command",
        "manage_task",
    ]);
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
    let printed = format!("agy transcript not kept: transcript file is absent\n{words}; ")
        + "the command was spawned, so no result is sent\n";
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
        let mut gate =
            crate::built::brokkr(["driver", "agy", "--", "--gate", "read_file", "run_command"])
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

const PRINTED_TRANSCRIPT: &str =
    include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2b-printed-value.transcript.ndjson");
const PRINTED_SENTINEL: &str = "l2b-sentinel-value-7f3a9c2e41d8";

/// End to end (#1227): an agy turn writes `transcript.jsonl` (mode 0644) into
/// its engine HOME, completes, removes the engine HOME, keeps `transcript.jsonl`
/// at mode 0600 inside a mode 0700 directory under
/// `<HOME>/.local/state/brokkr/agy-conversations/<uuid>`, records
/// `agy-conversation` in the journal, and `brokkr runs transcript` reads and
/// masks the sentinel value against `secrets.env` beside `forge.db`.
#[test]
fn an_engine_run_keeps_the_agy_transcript_and_runs_transcript_masks_it() {
    let test = "agy_driver::an_engine_run_keeps_the_agy_transcript_and_runs_transcript_masks_it";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    use std::os::unix::fs::MetadataExt;
    let (_home, root) = root();
    let uuid = "466b4fc2-eae8-497b-8306-5c8958fac45f";
    let bin = fake_agy_with_kept_transcript(&root, uuid, true);
    let recipe = world(&root);
    let db = root.join("forge.db");
    let (run_id, outcome, events) = drive_recipe(&root, &recipe, &bin, "agy");
    outcome.unwrap();

    // Engine HOME was removed; kept transcript survives with 0700/0600 modes.
    let engine_home = logged(&root, "home.log");
    assert!(!Path::new(&engine_home).exists(), "{engine_home}");
    let kept_parent = root.join(".local/state/brokkr/agy-conversations");
    let kept_dir = kept_parent.join(uuid);
    let kept_file = kept_dir.join("transcript.jsonl");
    assert!(kept_file.exists(), "kept transcript missing");
    assert_eq!(
        std::fs::metadata(&kept_parent).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(std::fs::metadata(&kept_dir).unwrap().mode() & 0o777, 0o700);
    assert_eq!(std::fs::metadata(&kept_file).unwrap().mode() & 0o777, 0o600);

    let finished = checkpoints(&events)
        .into_iter()
        .find(|row| row["step"] == "agy-cli-session-finished")
        .expect("finished checkpoint");
    assert_eq!(
        finished["transcript"],
        json!({
            "kind": "agy-conversation",
            "locator": uuid,
            "home": kept_parent.to_string_lossy(),
        })
    );

    // Secrets store beside forge.db masks the sentinel on `brokkr runs transcript`.
    brokkr_protocol::secret::store_set(&root.join("secrets.env"), "L2B_SENTINEL", PRINTED_SENTINEL)
        .unwrap();
    let out = crate::built::brokkr([
        "runs",
        "transcript",
        "--run",
        &run_id,
        "--seat",
        "scout",
        "--db",
    ])
    .arg(&db)
    .env("HOME", &root)
    .output()
    .unwrap();
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("kind  agy-conversation"), "{text}");
    assert!(text.contains("[secret:L2B_SENTINEL]"), "{text}");
    assert!(!text.contains(PRINTED_SENTINEL), "{text}");
}

fn write_undeclared_agy_fixture(root: &Path, case: &str, uuid: &str) -> (PathBuf, PathBuf) {
    let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let login = ".gemini/antigravity-cli/antigravity-oauth-token";
    write_mode(&root.join(login), b"the operator's login", 0o600);
    let bin = fake_agy_with_kept_transcript(root, uuid, false);

    let mut agy_adapter: Value = serde_json::from_slice(
        &std::fs::read(shipped.join("docs/examples/adapters/agy.json")).unwrap(),
    )
    .unwrap();
    if case == "adapter-without-record-kinds" {
        agy_adapter.as_object_mut().unwrap().remove("record_kinds");
    }
    write(
        &root.join("adapters/agy.json"),
        agy_adapter.to_string().as_bytes(),
    );
    write(
        &root.join("adapters/exec.json"),
        &std::fs::read(shipped.join("adapters/exec.json")).unwrap(),
    );
    write_scout_agent(root);

    let recipe = root.join("recipe");
    write(&recipe.join("scout.md"), b"# inline agy scout\n");
    let review = json!({"command": ["{brokkr}", "driver", "exec", "--", "true"]});
    let (policy, bundle_json) = if case == "inline-adapterless" {
        (
            json!({
                "phases": ["scout", "helper", "review", "done"], "initial": "scout", "terminal": ["done"],
                "rules": [
                    {"id": "scouted", "from": "scout", "result": "complete", "next": "review", "reason": "s"},
                    {"id": "scout-retry", "from": "scout", "result": "retry", "next": "helper", "reason": "sr"},
                    {"id": "helped", "from": "helper", "result": "complete", "next": "review", "reason": "h"},
                    {"id": "reviewed", "from": "review", "result": "complete", "next": "done", "reason": "r"},
                ],
            }),
            json!({
                "name": "inline-agy", "policy": "policy.json",
                "seats": {
                    "scout": {
                        "results": ["complete", "retry"],
                        "role": "scout.md",
                        "driver": {
                            "command": ["{brokkr}", "driver", "agy", "--", "--model", "gemini-4-neon"]
                        }
                    },
                    "review": {"results": ["complete"], "driver": review},
                    "helper": {"results": ["complete"], "agent": "scout"},
                }
            }),
        )
    } else {
        standard_policy_and_bundle("bare-agy")
    };
    write(&recipe.join("policy.json"), policy.to_string().as_bytes());
    write(
        &recipe.join("bundle.json"),
        bundle_json.to_string().as_bytes(),
    );
    (bin, recipe)
}

/// #1227 U2(b): both an inline adapterless `{brokkr} driver agy` seat and an
/// agent-backed `agy` seat whose adapter omits `record_kinds` complete cleanly,
/// record `transcript.kind: "none"`, and keep no file under
/// `~/.local/state/brokkr/agy-conversations/`, matching main.
#[test]
fn undeclared_agy_seats_complete_record_none_and_keep_nothing() {
    let test = "agy_driver::undeclared_agy_seats_complete_record_none_and_keep_nothing";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let uuid = "466b4fc2-eae8-497b-8306-5c8958fac45f";
    for case in ["inline-adapterless", "adapter-without-record-kinds"] {
        let (_home, root) = root();
        let (bin, recipe) = write_undeclared_agy_fixture(&root, case, uuid);
        let (_run_id, outcome, events) = drive_recipe(&root, &recipe, &bin, case);
        outcome.unwrap_or_else(|e| panic!("{case}: completes cleanly: {e}"));
        let succeeded = events
            .iter()
            .find(|e| e["type"] == "effect/succeeded")
            .unwrap_or_else(|| panic!("{case}: scout succeeded: {events:?}"));
        assert_eq!(
            succeeded["payload"]["result"]["result"], "complete",
            "{case}"
        );
        assert_eq!(
            succeeded["payload"]["result"]["transcript"],
            json!({"kind": "none", "locator": "", "home": ""}),
            "{case}"
        );
        let finished = checkpoints(&events)
            .into_iter()
            .find(|row| row["step"] == "agy-cli-session-finished")
            .unwrap_or_else(|| panic!("{case}: missing finished checkpoint"));
        assert_eq!(
            finished["transcript"],
            json!({"kind": "none", "locator": "", "home": ""}),
            "{case}"
        );
        assert!(
            !root.join(".local/state/brokkr/agy-conversations").exists(),
            "{case}: must keep no file"
        );
    }
}

/// End to end (#1227 slice 1): a work seat with shipped `agy.json` keeps
/// `conversations/<uuid>.db` and `brain/<uuid>/` operator-only under
/// `<HOME>/.local/state/brokkr/agy-conversations/<uuid>/rejoin/`, removes the
/// engine HOME, and records `harness-started` with `root_session`
/// (`kind: "agy-conversation"`, `harness_version: "1.5.14"`, `persistent: true`);
/// a gate seat with the same adapter keeps no rejoin state and records no
/// `root_session`.
fn seed_rejoin_e2e_fixture(root: &Path, work_uuid: &str, gate_uuid: &str) -> (PathBuf, PathBuf) {
    write(&root.join("work.ndjson"), NEON.as_bytes());
    write(
        &root.join("gate.ndjson"),
        NEON.replace(work_uuid, gate_uuid).as_bytes(),
    );
    write(&root.join("transcript.src"), PRINTED_TRANSCRIPT.as_bytes());
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
  printf '1.5.14\n'
  exit 0
fi
line=$(cat)
case "$line" in
  *"review gate"*) uuid="{gate_uuid}"; ndjson="{root}/gate.ndjson";;
  *) uuid="{work_uuid}"; ndjson="{root}/work.ndjson";;
esac
convs="$HOME/.gemini/antigravity-cli/conversations"
logs="$HOME/.gemini/antigravity-cli/brain/$uuid/.system_generated/logs"
steps="$HOME/.gemini/antigravity-cli/brain/$uuid/.system_generated/steps/1"
mkdir -p "$convs" "$logs" "$steps"
printf 'sqlite-rejoin-db' > "$convs/$uuid.db"
chmod 0600 "$convs/$uuid.db"
cp '{root}/transcript.src' "$logs/transcript.jsonl"
chmod 0644 "$logs/transcript.jsonl"
printf 'step-output' > "$steps/output.txt"
chmod 0644 "$steps/output.txt"
target=$(printf '%s' "$line" | sed -n 's/.*\\n    \(\/[^\\"]*\.json\)\\n.*/\1/p' | head -1)
[ -n "$target" ] && printf '{{"result":"complete"}}' > "$target"
cat "$ndjson"
exit 0
"#,
        root = root.display(),
    );
    let bin = root.join("agy");
    write_mode(&bin, script.as_bytes(), 0o755);

    seed_agy_adapters_and_login(root);
    write(&root.join("agents/charters/judge.md"), b"# review gate\n");
    let judge = json!({
        "description": "the agy judge", "charter": "charters/judge.md",
        "models": ["agy-neon"], "efforts": {"agy-neon": "low"},
        "limits": {"max_attempts": 1, "timeout_seconds": 20},
    });
    write(
        &root.join("agents/judge.json"),
        judge.to_string().as_bytes(),
    );
    let recipe = root.join("recipe");
    let (policy, _) = standard_policy_and_bundle("agy-rejoin-e2e");
    let bundle = json!({
        "name": "agy-rejoin-e2e", "policy": "policy.json",
        "seats": {
            "scout": {"results": ["complete"], "class": "work", "agent": "scout"},
            "review": {"results": ["complete"], "class": "gate", "agent": "judge"},
        },
    });
    write(&recipe.join("policy.json"), policy.to_string().as_bytes());
    write(&recipe.join("bundle.json"), bundle.to_string().as_bytes());
    (bin, recipe)
}

#[test]
fn an_engine_run_keeps_rejoin_state_for_work_seats_and_not_gate_seats() {
    let test = "agy_driver::an_engine_run_keeps_rejoin_state_for_work_seats_and_not_gate_seats";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    use std::os::unix::fs::MetadataExt;
    let (_home, root) = root();
    let work_uuid = "466b4fc2-eae8-497b-8306-5c8958fac45f";
    let gate_uuid = "77777777-8888-4999-8aaa-bbbbbbbbbbbb";
    let (bin, recipe) = seed_rejoin_e2e_fixture(&root, work_uuid, gate_uuid);

    let (_run_id, outcome, events) = drive_recipe(&root, &recipe, &bin, "agy-rejoin");
    outcome.unwrap();
    let succeeded = events
        .iter()
        .filter(|e| e["type"] == "effect/succeeded")
        .count();
    assert_eq!(succeeded, 2, "both work and gate seats must succeed");

    let kept_parent = root.join(".local/state/brokkr/agy-conversations");
    let work_rejoin = kept_parent.join(work_uuid).join("rejoin");
    let work_db = work_rejoin.join("conversation.db");
    let work_step = work_rejoin.join("brain/.system_generated/steps/1/output.txt");
    assert_eq!(std::fs::read(&work_db).unwrap(), b"sqlite-rejoin-db");
    assert_eq!(std::fs::read(&work_step).unwrap(), b"step-output");
    assert_eq!(
        std::fs::metadata(&work_rejoin).unwrap().mode() & 0o777,
        0o700
    );
    assert_eq!(std::fs::metadata(&work_db).unwrap().mode() & 0o777, 0o600);
    assert_eq!(std::fs::metadata(&work_step).unwrap().mode() & 0o777, 0o600);

    let gate_dir = kept_parent.join(gate_uuid);
    assert!(gate_dir.join("transcript.jsonl").exists());
    assert!(
        !gate_dir.join("rejoin").exists(),
        "gate seat must keep no rejoin state"
    );

    let started_rows: Vec<&Value> = checkpoints(&events)
        .into_iter()
        .filter(|row| row["step"] == "harness-started")
        .collect();
    assert_eq!(started_rows.len(), 1, "{started_rows:?}");
    assert_eq!(
        started_rows[0]["root_session"],
        json!({
            "kind": "agy-conversation",
            "id": work_uuid,
            "harness_version": "1.5.14",
            "persistent": true,
        })
    );
    assert_eq!(started_rows[0]["transcript"]["kind"], "agy-conversation");
    assert_eq!(started_rows[0]["transcript"]["locator"], work_uuid);
}

fn seed_rejoin_driver_fixture(
    dir: &Path,
    offered: &str,
    keep_db: bool,
) -> (PathBuf, PathBuf, Value) {
    use std::os::unix::fs::PermissionsExt;
    const L2C_02: &str =
        include_str!("../../../docs/evidence/adapters/agy-cli-1-5/l2c-02-rejoin-kept.ndjson");

    let (op_home, workdir, seat_dir) = (dir.join("home"), dir.join("work"), dir.join("seat"));
    std::fs::create_dir_all(&workdir).unwrap();
    std::fs::create_dir_all(&seat_dir).unwrap();
    let charter = seat_dir.join("charter.md");
    std::fs::write(&charter, "# charter\n").unwrap();
    let result_path = seat_dir.join("result.json");

    let login_dir = op_home.join(".gemini/antigravity-cli");
    std::fs::create_dir_all(&login_dir).unwrap();
    std::fs::write(login_dir.join("antigravity-oauth-token"), b"fake-token").unwrap();

    let parent = op_home.join(".local/state/brokkr/agy-conversations");
    let conv = parent.join(offered);
    let brain = conv.join("rejoin/brain");
    std::fs::create_dir_all(&brain).unwrap();
    for d in [&parent, &conv, &conv.join("rejoin"), &brain] {
        std::fs::set_permissions(d, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let db = conv.join("rejoin/conversation.db");
    std::fs::write(&db, b"turn-1-db").unwrap();
    std::fs::set_permissions(&db, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(conv.join("transcript.jsonl"), b"{\"turn\":1}\n").unwrap();
    std::fs::write(brain.join("note.txt"), b"t1").unwrap();

    let stdout_path = seat_dir.join("turn.stdout");
    std::fs::write(&stdout_path, L2C_02).unwrap();
    let db_action = if keep_db {
        format!("mkdir -p \"$HOME/.gemini/antigravity-cli/conversations\"\nprintf 't2-db' > \"$HOME/.gemini/antigravity-cli/conversations/{offered}.db\"\n")
    } else {
        format!("rm -f \"$HOME/.gemini/antigravity-cli/conversations/{offered}.db\"\n")
    };
    let bin = seat_dir.join("agy");
    write_mode(
        &bin,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = \"--version\" ]; then printf '1.5.18\\n'; exit 0; fi\n\
             cat >/dev/null\n\
             {db_action}\
             mkdir -p \"$HOME/.gemini/antigravity-cli/brain/{offered}/.system_generated/logs\"\n\
             printf 't2' > \"$HOME/.gemini/antigravity-cli/brain/{offered}/note.txt\"\n\
             printf '{{\"turn\":2}}\\n' > \"$HOME/.gemini/antigravity-cli/brain/{offered}/.system_generated/logs/transcript.jsonl\"\n\
             printf '{{\"result\":\"complete\"}}' > '{}'\n\
             cat '{}'\n",
            result_path.display(),
            stdout_path.display(),
        )
        .as_bytes(),
        0o755,
    );
    let input = json!({
        "workdir": workdir,
        "role_path": charter,
        "allowed_results": ["complete"],
        "result_path": result_path,
        "context": {},
        "boundary": "not applicable",
        "hands": "none",
        "resume_context": {
            "assessment": {
                "stream-work-site": {
                    "status": "supported",
                    "identity": { "version": "1.5.18", "applies_to": "1.5.18" },
                    "classes": ["work"],
                    "boundaries": ["not applicable"],
                    "hands": "none",
                    "evidence": {
                        "interface": "l2c-01..03",
                        "restrictions": "l2c-19..22",
                        "root": "l2c-02",
                        "accounting": "l2c-01..13"
                    }
                }
            },
            "declared_transcript_kind": "agy-conversation",
            "declared_root_session_kind": "agy-conversation",
            "originating_harness_version": "1.5.18",
            "owned_target": {
                "provider_id": offered,
                "persistence_locator": offered,
                "persistence_home": parent,
            }
        }
    });
    (op_home, bin, input)
}

#[test]
fn rejoin_emitted_resumed_row_passes_v8_seat_record_with_site_ref() {
    use brokkr_store::{validate_seat_record, SeatRecordVersion};
    use std::process::{Command, Stdio};

    const OFFERED: &str = "764607c1-0f06-4973-9fa7-1a1054494a3b";

    for (label, keep_db, expect_persistent) in
        [("persistent", true, true), ("non-persistent", false, false)]
    {
        let dir = tempfile::tempdir().unwrap();
        let (op_home, bin, input) = seed_rejoin_driver_fixture(dir.path(), OFFERED, keep_db);
        let payload: String = [
            json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello", "engine_version": "test"}),
            json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "resume", "effect_id": "fx", "attempt_id": "a1", "session_ref": OFFERED}),
            json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "start", "effect_id": "fx", "attempt_id": "a1", "seat": "work", "input": input}),
            json!({"proto": "forge-driver/v1", "msg_id": "m4", "type": "shutdown"}),
        ]
        .iter()
        .map(|row| format!("{row}\n"))
        .collect();

        let mut proc = Command::new(env!("CARGO_BIN_EXE_brokkr"));
        proc.args(["driver", "agy", "--", "--model", "gemini-3.8-flash-low"])
            .envs([
                ("BROKKR_AGY_BIN", bin.as_os_str()),
                ("HOME", op_home.as_os_str()),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = proc.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(payload.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{label}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let rows: Vec<Value> = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        let started = rows
            .iter()
            .find(|m| m["type"] == "checkpoint" && m["data"]["step"] == "harness-started")
            .unwrap_or_else(|| panic!("{label}: missing harness-started in {rows:?}"))["data"]
            .clone();
        assert_eq!(started["launch"], "resumed", "{label}");
        assert_eq!(
            started["root_session"]["persistent"],
            json!(expect_persistent),
            "{label}"
        );

        let mut stamped = started.clone();
        let obj = stamped.as_object_mut().unwrap();
        obj.insert("site_ref".into(), json!("a".repeat(64)));
        obj.insert("instance_ref".into(), json!("b".repeat(64)));
        validate_seat_record(&stamped, 1, SeatRecordVersion::V8)
            .unwrap_or_else(|e| panic!("{label}: v8 rejected stamped resumed row: {e}: {stamped}"));

        let mut without_root = stamped.clone();
        without_root.as_object_mut().unwrap().remove("root_session");
        assert!(
            validate_seat_record(&without_root, 1, SeatRecordVersion::V8).is_err(),
            "{label}: v8 must refuse stamped launch: resumed without root_session"
        );
    }
}
