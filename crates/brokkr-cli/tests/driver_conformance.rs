#![cfg(unix)]

//! Conformance for the built-in adapters (`brokkr driver <kind>`) — the
//! Rust port of the retired Python suite. Shims stand in for the agent
//! CLIs; conformance means capabilities on hello, accepted + checkpoint
//! + exactly one result per start, and the result-file contract honored.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const OBEDIENT_SHIM: &str = r#"#!/bin/sh
# honor the result contract: find the result path in the prompt
# (stdin or the last .md argument) and write a typed result there.
last=""
for a in "$@"; do last="$a"; done
case "$last" in
  *.md) prompt=$(cat "$last") ;;
  *Result?contract*) prompt=$last ;;
  *) prompt=$(cat) ;;
esac
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work"}' > "$target"
printf '{"type":"result","session_id":"s1","num_turns":1,"total_cost_usd":0.0}\n'
printf 'session id: deadbeef1234\n'
"#;

// The claude flavor speaks stream-json: an init with the session id, two
// tool-using assistant turns (the second carrying two tool_use blocks in
// one message), a noise line the adapter must drop, and a final result
// with the session totals — while still honoring the result-file contract.
const CLAUDE_STREAM_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work"}' > "$target"
printf '{"type":"system","subtype":"init","session_id":"stream-1"}\n'
printf '{"type":"assistant","message":{"content":[{"type":"text","text":"looking"},{"type":"tool_use","name":"Read","input":{"file_path":"src/lib.rs"}}]}}\n'
printf 'not json, ignorable noise\n'
printf '{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"src/main.rs"}},{"type":"tool_use","name":"Write","input":{"file_path":"src/out.rs"}}]}}\n'
printf '{"type":"result","num_turns":2,"total_cost_usd":0.125}\n'
"#;

const CODEX_JSON_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work"}' > "$target"
printf '{"type":"thread.started","thread_id":"codex-thread-1"}\n'
printf '{"type":"turn.started"}\n'
printf '{"type":"item.started","item":{"type":"command_execution","command":"secret command"}}\n'
printf '{"type":"item.completed","item":{"type":"command_execution","aggregated_output":"private output"}}\n'
printf '{"type":"turn.completed","usage":{"input_tokens":21,"cached_input_tokens":8,"output_tokens":5}}\n'
"#;

const SILENT_SHIM: &str = "#!/bin/sh\ncat > /dev/null 2>&1 || true\necho did nothing\n";

fn make_shim(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("shim");
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

fn drive(kind_args: &[&str], shim: &Path, workdir: &Path) -> Vec<Value> {
    let result_path = workdir.join("results/fx.json");
    let input = json!({
        "feature": "conformance", "phase": "intake", "seat": "intake",
        "role_path": workdir.join("missing-role.md"),
        "workdir": workdir,
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
    });
    let messages = [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
               "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ];
    let mut child = Command::new(brokkr_bin())
        .arg("driver")
        .args(kind_args)
        .env("FORGE_CLAUDE_BIN", shim)
        // Pinned unconditionally: no conformance test may ever spawn a
        // real claude-lanetally on a LaneTally-equipped machine.
        .env("FORGE_LANETALLY_BIN", shim)
        // Deliberately split: codex is pinned through the new spelling
        // and dsh through the old one, so a conformance run proves both
        // reach the same adapter for the release the old names survive
        // (decision 0019). The new dsh spelling is REMOVED rather than
        // left inherited: it outranks the old one, so an operator who
        // has it exported would otherwise send this test at a real dsh.
        .env("BROKKR_CODEX_BIN", shim)
        .env_remove("BROKKR_DSH_BIN")
        .env("FORGE_DSH_BIN", shim)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let _ = std::fs::remove_file(&result_path);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn all_adapters(shim: &Path) -> Vec<(&'static str, Vec<String>)> {
    vec![
        ("claude", vec!["claude".into()]),
        ("lanetally", vec!["lanetally".into()]),
        ("codex", vec!["codex".into()]),
        ("dsh", vec!["dsh".into()]),
        (
            "exec-stdin",
            vec![
                "exec".into(),
                "--".into(),
                shim.to_string_lossy().into_owned(),
            ],
        ),
        (
            "exec-promptfile",
            vec![
                "exec".into(),
                "--".into(),
                shim.to_string_lossy().into_owned(),
                "{prompt_file}".into(),
            ],
        ),
    ]
}

#[test]
fn conformance_across_all_builtin_adapters() {
    for case in ["obedient", "silent"] {
        let dir = tempfile::tempdir().unwrap();
        let shim = make_shim(
            dir.path(),
            if case == "obedient" {
                OBEDIENT_SHIM
            } else {
                SILENT_SHIM
            },
        );
        let claude_dir = dir.path().join("claude-stream");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let claude_shim = make_shim(&claude_dir, CLAUDE_STREAM_SHIM);
        let codex_dir = dir.path().join("codex-json");
        std::fs::create_dir_all(&codex_dir).unwrap();
        let codex_shim = make_shim(&codex_dir, CODEX_JSON_SHIM);
        for (label, args) in all_adapters(&shim) {
            // The claude adapter streams its session: its obedient shim
            // speaks stream-json and yields three seat-turn checkpoints
            // (one per tool_use block, the last two sharing a turn)
            // before the session-finished one.
            let claude = label == "claude";
            // The lanetally leg reuses CLAUDE_STREAM_SHIM verbatim: the
            // reuse IS the argv-compatibility proof — the wrapper is
            // driven exactly as the claude binary would be.
            let lanetally = label == "lanetally";
            let codex = label == "codex";
            let dsh = label == "dsh";
            let exec = label.starts_with("exec");
            let shim = if (claude || lanetally) && case == "obedient" {
                &claude_shim
            } else if codex && case == "obedient" {
                &codex_shim
            } else {
                &shim
            };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let out = drive(&args, shim, dir.path());
            let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
            let expected: &[&str] = if (claude || lanetally) && case == "obedient" {
                // One more checkpoint than before: session-started,
                // journaled at init so a WORKING seat's transcript is
                // locatable and live-streamable (the id used to arrive
                // only with session-finished, at the end).
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if codex && case == "obedient" {
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else if dsh || exec {
                // exec: the exec-started template checkpoint (decision
                // 0012 amendment) then session-finished.
                &[
                    "capabilities",
                    "accepted",
                    "checkpoint",
                    "checkpoint",
                    "result",
                ]
            } else {
                &["capabilities", "accepted", "checkpoint", "result"]
            };
            assert_eq!(kinds, expected, "{label}/{case}: {kinds:?}");
            for message in &out {
                assert_eq!(message["proto"], "forge-driver/v1", "{label}");
            }
            if claude && case == "obedient" {
                assert_eq!(
                    out[2]["data"]["step"], "session-started",
                    "{label}: the id is journaled at init: {}",
                    out[2]
                );
                assert!(
                    out[2]["data"]["session_id"].is_string(),
                    "{label}: {}",
                    out[2]
                );
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs"}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs"}),
                    "{label}: {}",
                    out[4]
                );
                assert_eq!(
                    out[5]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs"}),
                    "{label}: {}",
                    out[5]
                );
                let finished = &out[6]["data"];
                assert_eq!(finished["step"], "claude-code-session-finished", "{label}");
                assert_eq!(finished["session_id"], "stream-1", "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
                // The capture guard is kind-scoped: only lanetally's
                // finished checkpoint ever carries the marker.
                assert!(finished.get("capture").is_none(), "{label}: {finished}");
            } else if lanetally && case == "obedient" {
                // Same stream, same fold, same disciplines as claude —
                // plus the constant ledger-capture marker and the
                // list-price cost flowing through unchanged.
                assert_eq!(
                    out[2]["data"]["step"], "session-started",
                    "{label}: {}",
                    out[2]
                );
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs"}),
                    "{label}: {}",
                    out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs"}),
                    "{label}: {}",
                    out[4]
                );
                assert_eq!(
                    out[5]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs"}),
                    "{label}: {}",
                    out[5]
                );
                let finished = &out[6]["data"];
                assert_eq!(
                    finished["step"], "claude-lanetally-session-finished",
                    "{label}"
                );
                assert_eq!(finished["capture"], "lanetally", "{label}: {finished}");
                assert_eq!(finished["session_id"], "stream-1", "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
            } else if codex && case == "obedient" {
                assert_eq!(
                    out[2]["data"],
                    json!({"step":"turn-started", "turn":1, "harness":"codex"})
                );
                assert_eq!(out[3]["data"]["tool"], "command_execution");
                assert!(out[3]["data"].get("command").is_none());
                assert_eq!(out[5]["data"]["input_tokens"], 21);
                assert_eq!(out[5]["data"]["cache_read_tokens"], 8);
                assert_eq!(out[6]["data"]["session_id"], "codex-thread-1");
                assert_eq!(out[6]["data"]["output_tokens"], 5);
            } else if dsh {
                assert_eq!(out[2]["data"]["step"], "harness-started");
                assert_eq!(out[2]["data"]["harness"], "deepseek");
                assert_eq!(out[3]["data"]["step"], "deepseek-harness-session-finished");
            } else if exec {
                // The journaled target is the UNRESOLVED template within
                // the 80-char clamp (decision 0012 amendment).
                assert_eq!(out[2]["data"]["step"], "exec-started", "{label}");
                let target = out[2]["data"]["target"].as_str().unwrap();
                assert!(target.chars().count() <= 80, "{label}: {target}");
                let template = args[2..].join(" ");
                assert_eq!(
                    target,
                    template.chars().take(80).collect::<String>(),
                    "{label}"
                );
            }
            let result = out.last().unwrap();
            if case == "obedient" {
                assert_eq!(result["status"], "succeeded", "{label}: {result}");
                assert_eq!(result["result"]["result"], "resolved", "{label}");
            } else {
                assert_eq!(result["status"], "failed", "{label}: {result}");
                assert!(
                    result["error"].as_str().unwrap().contains("no result file"),
                    "{label}: {result}"
                );
            }
        }
    }
}

#[test]
fn adapters_name_themselves_and_exec_requires_template() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), OBEDIENT_SHIM);
    let expected = [
        ("claude", "claude-code"),
        ("lanetally", "claude-lanetally"),
        ("codex", "codex"),
        ("dsh", "deepseek-harness"),
        ("exec-stdin", "exec"),
        ("exec-promptfile", "exec"),
    ];
    for ((label, args), (_, name)) in all_adapters(&shim).iter().zip(expected) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = drive(&args, &shim, dir.path());
        assert_eq!(out[0]["driver"], name, "{label}");
    }
    let out = drive(&["exec"], &shim, dir.path());
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["error"]
        .as_str()
        .unwrap()
        .contains("command template"));
}

// Records its argv, then emits the claude stream shape with an
// adversarial `"capture":"evil"` smuggled into the result event: the
// run_seat source literal must still win on the finished checkpoint.
const LANETALLY_ADVERSARIAL_SHIM: &str = r#"#!/bin/sh
printf '%s\n' "$*" > lanetally-argv.txt
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
[ -n "$target" ] && printf '{"result": "resolved", "notes": "shim did the work"}' > "$target"
printf '{"type":"system","subtype":"init","session_id":"adv-1"}\n'
printf '{"type":"result","num_turns":1,"total_cost_usd":0.5,"capture":"evil"}\n'
"#;

#[test]
fn lanetally_argv_is_claude_shaped_and_the_capture_constant_survives_adversarial_streams() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), LANETALLY_ADVERSARIAL_SHIM);
    let out = drive(&["lanetally"], &shim, dir.path());
    // The wrapper is invoked exactly as claude would be: the stream-json
    // argv, prompt on stdin (the shim finds the result path only if the
    // prompt arrived there — the succeeded result below proves it).
    let argv = std::fs::read_to_string(dir.path().join("lanetally-argv.txt")).unwrap();
    assert_eq!(argv.trim_end(), "-p --output-format stream-json --verbose");
    let finished = &out[out.len() - 2]["data"];
    assert_eq!(finished["step"], "claude-lanetally-session-finished");
    // The constant is inserted after the session_meta extend: no
    // stream-derived key can shadow it.
    assert_eq!(finished["capture"], "lanetally", "{finished}");
    assert_eq!(finished["total_cost_usd"], 0.5, "{finished}");
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
}

// ------------------------------------------------------------------
// Sealed secret bindings (decision 0012): injection discipline and the
// masking choke point, exercised against the real exec adapter — and,
// for the result-payload surface, against the lanetally adapter through
// the same shared run_seat choke point.
// ------------------------------------------------------------------

const SECRET_VALUE: &str = "tok3n+v4lue!7";

/// Writes what it can see: the injected environment, the literal argv
/// it was handed, and a stderr leak of the value.
const SECRET_PROBE_SHIM: &str = r#"#!/bin/sh
cat > /dev/null
printf '%s' "$API_TOKEN" > env.txt
printf '%s' "$UNREF" > unref.txt
printf '%s' "$1" > argv.txt
printf 'stderr-leak %s\n' "$API_TOKEN" 1>&2
"#;

/// Honors the result contract but echoes the injected value into its
/// result notes — the leg of the choke point that would otherwise ride
/// EffectSucceeded into the append-only journal.
const NOTES_LEAK_SHIM: &str = r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
printf '{"result": "resolved", "notes": "leaked %s"}' "$API_TOKEN" > "$target"
"#;

fn write_store(dir: &Path, lines: &str) -> PathBuf {
    let store = dir.join("secrets.env");
    std::fs::write(&store, lines).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    store
}

/// Drive an adapter over one attempt with secret bindings in the start
/// input, returning the protocol messages and the driver's own stderr
/// (which carries the masked child-stderr re-emit). `driver_args` is
/// everything after `brokkr driver` — e.g. `["exec", "--", template…]`
/// or `["lanetally"]`.
fn drive_with_secrets(
    driver_args: &[&str],
    workdir: &Path,
    store: &Path,
    secrets: &[&str],
    parent_env: &[(&str, &str)],
) -> (Vec<Value>, String) {
    let result_path = workdir.join("results/fx.json");
    let input = json!({
        "feature": "conformance", "phase": "intake", "seat": "intake",
        "role_path": workdir.join("missing-role.md"),
        "workdir": workdir,
        "result_path": result_path,
        "allowed_results": ["resolved"], "context": {},
        "secrets": secrets,
        "secrets_file": store,
    });
    let messages = [
        json!({"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
               "engine_version": "test"}),
        json!({"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
               "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
               "input": input}),
        json!({"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"}),
    ];
    let mut command = Command::new(brokkr_bin());
    command.arg("driver").args(driver_args);
    for (key, value) in parent_env {
        command.env(key, value);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in &messages {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    (parsed, String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn exec_injects_via_env_only_and_masks_the_stderr_reemit() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), SECRET_PROBE_SHIM);
    let store = write_store(
        dir.path(),
        &format!("API_TOKEN={SECRET_VALUE}\nUNREF=unref-value-99\n"),
    );
    let (out, stderr) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap(), "{{secret:API_TOKEN}}"],
        dir.path(),
        &store,
        &["API_TOKEN", "UNREF"],
        // A pre-existing child env entry: the declared secret must win.
        &[("API_TOKEN", "from-parent-env")],
    );
    // Env injection carried the value; the declared name overrode the
    // inherited entry.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("env.txt")).unwrap(),
        SECRET_VALUE
    );
    // Every DECLARED name injects, referenced in the template or not.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("unref.txt")).unwrap(),
        "unref-value-99"
    );
    // argv carries the literal shell reference, never the value.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("argv.txt")).unwrap(),
        "$API_TOKEN"
    );
    // The checkpointed target is the unresolved template.
    let target = out[2]["data"]["target"].as_str().unwrap();
    assert!(target.contains("{{secret:API_TOKEN}}") || target.chars().count() == 80);
    assert!(!target.contains(SECRET_VALUE));
    // The child's stderr leak reaches the driver's re-emit masked.
    assert!(stderr.contains("[secret:API_TOKEN]"), "stderr: {stderr}");
    assert!(!stderr.contains(SECRET_VALUE), "stderr: {stderr}");
}

#[test]
fn exec_missing_secret_refuses_before_spawn_naming_name_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), SECRET_PROBE_SHIM);
    let store = write_store(dir.path(), "OTHER=some-other-value\n");
    let (out, _) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap()],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[],
    );
    let kinds: Vec<&str> = out.iter().map(|m| m["type"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        vec!["capabilities", "accepted", "result"],
        "no checkpoint, no spawn: {kinds:?}"
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    let error = result["error"].as_str().unwrap();
    assert!(error.contains("API_TOKEN"), "{error}");
    assert!(error.contains("secrets.env"), "{error}");
    assert!(
        !error.contains("some-other-value"),
        "never the contents: {error}"
    );
    assert!(
        !dir.path().join("env.txt").exists(),
        "the child must never have spawned"
    );
}

#[test]
fn exec_masks_the_child_written_result_payload() {
    let dir = tempfile::tempdir().unwrap();
    let shim = make_shim(dir.path(), NOTES_LEAK_SHIM);
    let store = write_store(dir.path(), &format!("API_TOKEN={SECRET_VALUE}\n"));
    let (out, _) = drive_with_secrets(
        &["exec", "--", shim.to_str().unwrap()],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[],
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
    assert_eq!(result["result"]["notes"], "leaked [secret:API_TOKEN]");
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains(SECRET_VALUE),
        "{result}"
    );
}

#[test]
fn lanetally_result_payload_reaches_the_shared_masking_choke_point() {
    // The claude/lanetally arm injects no secret env (bindings feed only
    // the exec arm's spawn), so a shim echoing $API_TOKEN would leak
    // nothing and prove nothing: the store's known plaintext is baked
    // literally into the result notes instead, and run_seat's shared
    // known-plaintext masking — zero lanetally-specific code — must
    // catch it on the way into the journal.
    let dir = tempfile::tempdir().unwrap();
    let leak_shim = format!(
        r#"#!/bin/sh
prompt=$(cat)
target=$(printf '%s\n' "$prompt" | sed -n 's/^    \(.*\.json\)$/\1/p' | head -1)
printf '{{"result": "resolved", "notes": "leaked {SECRET_VALUE}"}}' > "$target"
printf '{{"type":"result","num_turns":1,"total_cost_usd":0.0}}\n'
"#
    );
    let shim = make_shim(dir.path(), &leak_shim);
    let store = write_store(dir.path(), &format!("API_TOKEN={SECRET_VALUE}\n"));
    let (out, _) = drive_with_secrets(
        &["lanetally"],
        dir.path(),
        &store,
        &["API_TOKEN"],
        &[("FORGE_LANETALLY_BIN", shim.to_str().unwrap())],
    );
    let result = out.last().unwrap();
    assert_eq!(result["status"], "succeeded", "{result}");
    assert_eq!(result["result"]["notes"], "leaked [secret:API_TOKEN]");
    assert!(
        !serde_json::to_string(result)
            .unwrap()
            .contains(SECRET_VALUE),
        "{result}"
    );
}
