#![cfg(unix)]

//! Conformance for the built-in adapters (`forge driver <kind>`) — the
//! Rust port of the retired Python suite. Shims stand in for the agent
//! CLIs; conformance means capabilities on hello, accepted + checkpoint
//! + exactly one result per start, and the result-file contract honored.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn forge_bin() -> &'static str {
    env!("CARGO_BIN_EXE_forge")
}

const OBEDIENT_SHIM: &str = r#"#!/bin/sh
# honor the result contract: find the result path in the prompt
# (stdin or the last .md argument) and write a typed result there.
last=""
for a in "$@"; do last="$a"; done
case "$last" in
  *.md) prompt=$(cat "$last") ;;
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
    let mut child = Command::new(forge_bin())
        .arg("driver")
        .args(kind_args)
        .env("FORGE_CLAUDE_BIN", shim)
        .env("FORGE_CODEX_BIN", shim)
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
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let _ = std::fs::remove_file(&result_path);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn all_adapters<'a>(shim: &'a Path) -> Vec<(&'static str, Vec<String>)> {
    vec![
        ("claude", vec!["claude".into()]),
        ("codex", vec!["codex".into()]),
        ("exec-stdin", vec!["exec".into(), "--".into(), shim.to_string_lossy().into_owned()]),
        ("exec-promptfile", vec![
            "exec".into(), "--".into(),
            shim.to_string_lossy().into_owned(), "{prompt_file}".into(),
        ]),
    ]
}

#[test]
fn conformance_across_all_builtin_adapters() {
    for case in ["obedient", "silent"] {
        let dir = tempfile::tempdir().unwrap();
        let shim = make_shim(dir.path(), if case == "obedient" { OBEDIENT_SHIM } else { SILENT_SHIM });
        let claude_dir = dir.path().join("claude-stream");
        std::fs::create_dir_all(&claude_dir).unwrap();
        let claude_shim = make_shim(&claude_dir, CLAUDE_STREAM_SHIM);
        for (label, args) in all_adapters(&shim) {
            // The claude adapter streams its session: its obedient shim
            // speaks stream-json and yields three seat-turn checkpoints
            // (one per tool_use block, the last two sharing a turn)
            // before the session-finished one.
            let claude = label == "claude";
            let shim = if claude && case == "obedient" { &claude_shim } else { &shim };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let out = drive(&args, shim, dir.path());
            let kinds: Vec<&str> =
                out.iter().map(|m| m["type"].as_str().unwrap()).collect();
            let expected: &[&str] = if claude && case == "obedient" {
                &["capabilities", "accepted", "checkpoint", "checkpoint", "checkpoint", "checkpoint", "result"]
            } else {
                &["capabilities", "accepted", "checkpoint", "result"]
            };
            assert_eq!(kinds, expected, "{label}/{case}: {kinds:?}");
            for message in &out {
                assert_eq!(message["proto"], "forge-driver/v1", "{label}");
            }
            if claude && case == "obedient" {
                assert_eq!(
                    out[2]["data"],
                    json!({"step": "seat-turn", "turn": 1, "tool": "Read",
                           "target": "src/lib.rs"}),
                    "{label}: {}", out[2]
                );
                assert_eq!(
                    out[3]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Edit",
                           "target": "src/main.rs"}),
                    "{label}: {}", out[3]
                );
                assert_eq!(
                    out[4]["data"],
                    json!({"step": "seat-turn", "turn": 2, "tool": "Write",
                           "target": "src/out.rs"}),
                    "{label}: {}", out[4]
                );
                let finished = &out[5]["data"];
                assert_eq!(finished["step"], "claude-code-session-finished", "{label}");
                assert_eq!(finished["session_id"], "stream-1", "{label}");
                assert_eq!(finished["num_turns"], 2, "{label}");
                assert_eq!(finished["total_cost_usd"], 0.125, "{label}");
                assert_eq!(finished["exit_code"], 0, "{label}");
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
        ("claude", "claude-code"), ("codex", "codex"),
        ("exec-stdin", "exec"), ("exec-promptfile", "exec"),
    ];
    for ((label, args), (_, name)) in all_adapters(&shim).iter().zip(expected) {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = drive(&args, &shim, dir.path());
        assert_eq!(out[0]["driver"], name, "{label}");
    }
    let out = drive(&["exec"], &shim, dir.path());
    let result = out.last().unwrap();
    assert_eq!(result["status"], "failed");
    assert!(result["error"].as_str().unwrap().contains("command template"));
}
