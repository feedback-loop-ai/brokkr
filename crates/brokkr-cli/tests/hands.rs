//! Decision 0040 through the binary: `brokkr hands serve` speaks MCP on
//! stdio and `brokkr hands exec` runs a command whole inside the box.
//! Linux only, like the boundary.

#![cfg(target_os = "linux")]

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

#[test]
fn hands_serve_lists_one_tool_and_runs_it_in_the_box() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("hello.txt"), "from the worktree\n").unwrap();
    let mut child = Command::new(brokkr_bin())
        .args(["hands", "serve", "--workdir", work.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"protocolVersion":"2025-06-18"}}}}"#).unwrap();
        writeln!(
            stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list"}}"#).unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"workspace","arguments":{{"command":"cat hello.txt; ls /home | wc -l"}}}}}}"#).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let replies: Vec<Value> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[1]["result"]["tools"][0]["name"], "workspace");
    let text = replies[2]["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("from the worktree"), "{text}");
    assert!(
        text.contains("\n0\n"),
        "the host's homes are not in the box: {text}"
    );
    assert!(text.ends_with("[exit code: 0]"), "{text}");
}

#[test]
fn hands_exec_runs_the_command_whole_and_returns_its_code() {
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let secret = dir.path().join("secret");
    std::fs::write(&secret, "x").unwrap();
    let ok = Command::new(brokkr_bin())
        .args([
            "hands",
            "exec",
            "--workdir",
            work.to_str().unwrap(),
            "--",
            "bash",
            "-c",
            "echo boxed > out.txt",
        ])
        .status()
        .unwrap();
    assert!(ok.success());
    assert_eq!(
        std::fs::read_to_string(work.join("out.txt")).unwrap(),
        "boxed\n"
    );
    let hidden = Command::new(brokkr_bin())
        .args([
            "hands",
            "exec",
            "--workdir",
            work.to_str().unwrap(),
            "--",
            "cat",
            secret.to_str().unwrap(),
        ])
        .stderr(Stdio::null())
        .status()
        .unwrap();
    assert_eq!(hidden.code(), Some(1), "the host file is not in the box");
    let code = Command::new(brokkr_bin())
        .args([
            "hands",
            "exec",
            "--workdir",
            work.to_str().unwrap(),
            "--",
            "bash",
            "-c",
            "exit 7",
        ])
        .status()
        .unwrap();
    assert_eq!(code.code(), Some(7));
    // A spec that does not parse is refused before anything spawns.
    let bad = Command::new(brokkr_bin())
        .args([
            "hands",
            "exec",
            "--workdir",
            work.to_str().unwrap(),
            "--spec",
            "{\"kind\":\"mitten\"}",
            "--",
            "true",
        ])
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("--spec"));
    // And the boxed binary can call itself: the exec driver dispatch shape.
    let selfcall = Command::new(brokkr_bin())
        .args([
            "hands",
            "exec",
            "--workdir",
            work.to_str().unwrap(),
            "--",
            brokkr_bin(),
            "--version",
        ])
        .output()
        .unwrap();
    assert!(
        selfcall.status.success(),
        "{}",
        String::from_utf8_lossy(&selfcall.stderr)
    );
    assert!(String::from_utf8_lossy(&selfcall.stdout).starts_with("brokkr "));
}
