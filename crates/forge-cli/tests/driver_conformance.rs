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
    for (case, shim_body) in [("obedient", OBEDIENT_SHIM), ("silent", SILENT_SHIM)] {
        let dir = tempfile::tempdir().unwrap();
        let shim = make_shim(dir.path(), shim_body);
        for (label, args) in all_adapters(&shim) {
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            let out = drive(&args, &shim, dir.path());
            let kinds: Vec<&str> =
                out.iter().map(|m| m["type"].as_str().unwrap()).collect();
            assert_eq!(
                kinds,
                ["capabilities", "accepted", "checkpoint", "result"],
                "{label}/{case}: {kinds:?}"
            );
            for message in &out {
                assert_eq!(message["proto"], "forge-driver/v1", "{label}");
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
