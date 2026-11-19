#![cfg(unix)]

//! `brokkr driver qoder`, end to end (#1178): Qoder's example adapter in a
//! world that lists its office, a compiled seat hiring `qoder-fast`, the
//! engine's own start, and the real driver over a fake `qodercli` that
//! prints one of Qoder CLI 1.1's recorded turns and exits with its code.

use std::path::{Path, PathBuf};

use brokkr_core::realms::Boundary;
use brokkr_runtime::agents::{Candidate, Lowering};
use brokkr_runtime::bundle::{RealmLaw, SeatBody};
use brokkr_runtime::{Bundle, Engine, Invocation};
use brokkr_store::Store;
use serde_json::{json, Value};

#[path = "../../../tests/support/env_guard.rs"]
mod env_guard;
use env_guard::EnvGuard;

const TOOL_LESS: &str =
    include_str!("../../../docs/evidence/adapters/qoder-cli-1-1/tool-less-turn.ndjson");
const UNKNOWN_MODEL: &str =
    include_str!("../../../docs/evidence/adapters/qoder-cli-1-1/unknown-model-fallback.ndjson");

/// `body` at `path`, its directories made, under `mode`.
fn write_mode(path: &Path, body: &[u8], mode: u32) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut options = std::fs::OpenOptions::new();
    let file = options.write(true).create_new(true).mode(mode).open(path);
    file.and_then(|mut file| file.write_all(body)).unwrap();
}

fn write(path: &Path, body: &[u8]) {
    write_mode(path, body, 0o644);
}

/// A fake `qodercli` that logs its argv, writes the seat's result where
/// the prompt names it, prints `stdout` and exits 0.
fn fake_qoder(root: &Path, stdout: &str) -> PathBuf {
    write(&root.join("recorded.ndjson"), stdout.as_bytes());
    let body = format!(
        "#!/bin/sh\n\
         printf '%s\\n' \"$@\" > '{root}/argv.log'\n\
         prompt=$(cat)\n\
         target=$(printf '%s\\n' \"$prompt\" | sed -n 's/^    \\(.*\\.json\\)$/\\1/p' | head -1)\n\
         [ -n \"$target\" ] && printf '{{\"result\":\"complete\"}}' > \"$target\"\n\
         cat '{root}/recorded.ndjson'\n",
        root = root.display()
    );
    // Written before the engine starts, in a process that runs this test
    // alone, so no forked child holds it open for writing when it runs.
    let path = root.join("qodercli");
    write_mode(&path, body.as_bytes(), 0o755);
    path
}

/// Qoder's example adapter as the guide ships it.
fn example() -> Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/examples/adapters/qoder.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// A test-only measurement of Qoder's cold and replacement shapes with no
/// hands, which nobody has taken (`qoder-cli-1-1.md`, Not measured). It
/// stands in so the engine compiles the seat where SI2's fence is lifted
/// and the driver can be handed the engine's own start; it is no evidence.
fn fixture_mcp() -> Value {
    let fixture = json!({"measured": "test fixture only: no ambient MCP sentinel was planted"});
    let shape = |invocation: &str| {
        json!({
            "invocation": invocation, "hands": "none",
            "measured_on": {"harness": "qoder", "binary": "qodercli", "version": "1.1.92",
                            "host": if cfg!(target_os = "macos") { "macos" } else { "linux" }},
            "ambient": fixture, "native_write": fixture,
            "store_read": fixture, "process_read": fixture,
        })
    };
    json!({"carriage": {"unmeasured": "test fixture"}, "shapes": [shape("cold"), shape("replacement")]})
}

/// `adapter` as Qoder's, a `tinker` office hiring `qoder-fast` at `low`
/// with one grant, and a recipe whose work seat it holds.
fn tree(root: &Path, adapter: &Value) -> PathBuf {
    write(
        &root.join("adapters/qoder.json"),
        &serde_json::to_vec(adapter).unwrap(),
    );
    let exec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/exec.json");
    write(
        &root.join("adapters/exec.json"),
        &std::fs::read(exec).unwrap(),
    );
    write(&root.join("agents/charters/tinker.md"), b"# tinker\n");
    let agent = json!({
        "description": "the qoder tinker", "charter": "charters/tinker.md",
        "models": ["qoder-fast"], "efforts": {"qoder-fast": "low"},
        "tools": {"allow": ["git"]},
        "limits": {"max_attempts": 1, "timeout_seconds": 20},
    });
    write(
        &root.join("agents/tinker.json"),
        &serde_json::to_vec(&agent).unwrap(),
    );
    let recipe = root.join("recipe");
    let policy = json!({
        "phases": ["work", "review", "done"], "initial": "work", "terminal": ["done"],
        "rules": [
            {"id": "advance", "from": "work", "result": "complete", "next": "review", "reason": "w"},
            {"id": "close", "from": "review", "result": "complete", "next": "done", "reason": "r"},
        ],
    });
    write(
        &recipe.join("policy.json"),
        &serde_json::to_vec(&policy).unwrap(),
    );
    let bundle = json!({
        "name": "qoder", "policy": "policy.json",
        "seats": {
            "work": {"results": ["complete"], "agent": "tinker"},
            // The protected review gate asks a strict MCP configuration,
            // which Qoder's unmeasured isolation cannot give.
            "review": {"results": ["complete"],
                       "driver": {"command": ["{brokkr}", "driver", "exec", "--", "true"]}},
        },
    });
    write(
        &recipe.join("bundle.json"),
        &serde_json::to_vec(&bundle).unwrap(),
    );
    recipe
}

/// Every composed `{brokkr}`, which the compiler expanded to this test's
/// executable, pointed at the built binary.
fn patch(bundle: &mut Bundle) {
    let from = std::env::current_exe().unwrap();
    let from = from.to_string_lossy();
    let retarget = |words: &mut [String]| {
        let engine = words.iter_mut().filter(|part| **part == from);
        engine.for_each(|part| *part = env!("CARGO_BIN_EXE_brokkr").to_string());
    };
    let singles = bundle
        .seats
        .values_mut()
        .filter_map(|seat| match &mut seat.body {
            SeatBody::Invocation(Invocation::Single {
                command,
                candidates,
                ..
            }) => Some((command, candidates)),
            _ => None,
        });
    for (command, candidates) in singles {
        retarget(command);
        for candidate in candidates.iter_mut() {
            retarget(&mut candidate.argv);
            segments(candidate).for_each(|argv| retarget(argv));
        }
    }
}

/// The argv of each segment a composed candidate carries.
fn segments(candidate: &mut Candidate) -> impl Iterator<Item = &mut Vec<String>> {
    let composed = match &mut candidate.lowering {
        Lowering::Composed(composition) => Some(composition.segments.iter_mut()),
        _ => None,
    };
    composed
        .into_iter()
        .flatten()
        .map(|segment| &mut segment.argv)
}

/// The recipe compiled on `adapter` where the world lists `tinker`.
fn compile(root: &Path, adapter: &Value) -> Result<Bundle, brokkr_runtime::bundle::CompileError> {
    let recipe = tree(root, adapter);
    let law = RealmLaw {
        boundary: Boundary::Namespace,
        provisional_offices: vec!["tinker".to_string()],
    };
    Bundle::compile_under(&recipe, &root.join("agents"), &root.join("adapters"), law)
}

/// The recipe compiled on the fixture measurement and run once over
/// `stdout` in `root`. The checkpoints the journal kept are returned.
fn run(root: &Path, stdout: &str) -> Vec<Value> {
    let mut adapter = example();
    adapter["mcp"] = fixture_mcp();
    let mut bundle = compile(root, &adapter).expect("a qoder seat in a listed office compiles");
    patch(&mut bundle);
    let mut env = EnvGuard::lock();
    env.set("BROKKR_QODER_BIN", fake_qoder(root, stdout));
    env.set("HOME", root);
    std::fs::create_dir_all(root.join("work")).unwrap();
    let store = Store::open(&root.join("forge.db")).unwrap();
    let mut engine = Engine::start(store, bundle, "qoder", Some(root.join("work"))).unwrap();
    let driven = engine.drive();
    let events = engine.store.load(&engine.run_id).unwrap();
    eprintln!("the drive ended {driven:?}");
    events
        .iter()
        .map(|event| serde_json::to_value(event).unwrap())
        .inspect(|event| eprintln!("{event}"))
        .collect()
}

/// The checkpoints among `events`.
fn checkpoints(events: &[Value]) -> Vec<&Value> {
    events
        .iter()
        .filter_map(|event| event["payload"].get("checkpoint"))
        .collect()
}

/// A canonical, owner-only temporary root for one run.
fn root() -> (tempfile::TempDir, PathBuf) {
    let home = tempfile::tempdir().unwrap();
    let root = home.path().canonicalize().unwrap();
    rustix::fs::chmod(&root, rustix::fs::Mode::RWXU).unwrap();
    (home, root)
}

/// SI2 on Linux, where its fence is lifted: the example declares Qoder's
/// ambient MCP isolation unmeasured, as the record leaves it, so a seat on
/// it is refused at compile and nothing is launched.
#[cfg(target_os = "linux")]
#[test]
fn the_example_is_refused_on_linux_until_its_mcp_isolation_is_measured() {
    let (_home, root) = root();
    let refused = compile(&root, &example()).map(drop).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "bundle: seat 'work' (office 'tinker') in realm '<unmapped>': provider 'qoder' has no \
         measured strict MCP configuration for 'cold'"
    );
}

#[test]
fn an_engine_run_of_a_qoder_seat_launches_its_pin_and_journals_the_served_model() {
    // In an engine of its own (#403): an engine is its attempts' subreaper.
    let test = "qoder_driver::an_engine_run_of_a_qoder_seat_launches_its_pin_and_journals_the_served_model";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let (_home, root) = root();
    let events = run(&root, TOOL_LESS);
    let checkpoints = checkpoints(&events);
    let argv = std::fs::read_to_string(root.join("argv.log")).unwrap();
    assert_eq!(
        argv.lines().collect::<Vec<_>>(),
        [
            "-p",
            "--output-format",
            "stream-json",
            "--no-session-persistence",
            "--permission-mode",
            "accept_edits",
            "-m",
            "Fast",
            "--reasoning-effort",
            "low",
            "--allowed-tools",
            "Bash(git:*)",
            "--tools",
            "Read",
            "Edit",
            "Write",
            "Glob",
            "Grep",
            "Bash",
            "--disallowed-tools",
            "mcp_call,mcp_get,mcp_list",
        ]
    );
    let finished = checkpoints
        .iter()
        .find(|row| row["step"] == "qoder-cli-session-finished")
        .expect("the seat finished");
    assert_eq!(finished["model"], "Fast");
    assert_eq!(finished["effort"], "not reported");
    assert_eq!(
        finished["transcript"],
        json!({"kind": "none", "locator": "", "home": ""})
    );
    for absent in [
        "total_cost_usd",
        "input_tokens",
        "output_tokens",
        "root_session",
        "capture",
    ] {
        assert_eq!(finished.get(absent), None, "{absent}");
    }
}

#[test]
fn an_engine_run_of_a_qoder_seat_served_another_model_journals_no_turn() {
    let test = "qoder_driver::an_engine_run_of_a_qoder_seat_served_another_model_journals_no_turn";
    if !brokkr_protocol::process::in_its_own_engine(test) {
        return;
    }
    let (_home, root) = root();
    let events = run(&root, UNKNOWN_MODEL);
    assert_eq!(checkpoints(&events), Vec::<&Value>::new());
    let failed = events
        .iter()
        .find(|event| event["type"] == "effect/failed")
        .expect("the seat failed");
    assert_eq!(
        (
            &failed["payload"]["error"],
            &failed["payload"]["start_failure"]
        ),
        (
            &json!(
                "provider refused before the first turn: model_substituted: the seat pinned \
                 \"Fast\" and Qoder served another: init named \"Auto\"; stderr tail: "
            ),
            &json!(true)
        )
    );
}
