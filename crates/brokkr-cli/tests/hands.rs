//! Decision 0043 through the binary: `brokkr hands serve` speaks MCP on
//! stdio and `brokkr hands exec` runs a command whole inside the box.
//! Linux only, like the boundary.

#![cfg(target_os = "linux")]

use std::io::Write;
use std::path::Path;
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

fn verifier_workspace(failing: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(dir.path().join("scripts")).unwrap();
    std::fs::create_dir_all(dir.path().join("bundles/self")).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/verify-seat.sh"),
        dir.path().join("scripts/verify-seat.sh"),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname='brokkr-cli'\nversion='0.0.0'\nedition='2021'\n",
    )
    .unwrap();
    let assertion = if failing {
        "assert_eq!(2 + 2, 5, \"DECISIVE\\tRED\\rLINE\");"
    } else {
        "assert_eq!(2 + 2, 4);"
    };
    std::fs::write(
        dir.path().join("src/main.rs"),
        format!("fn main() {{}}\n#[test]\nfn trivial() {{ {assertion} }}\n"),
    )
    .unwrap();
    let result = dir.path().join("result.json");
    std::fs::write(
        dir.path().join("prompt.md"),
        format!("result contract\n\n    {}\n", result.display()),
    )
    .unwrap();
    dir
}

#[test]
fn boxed_verify_seat_reports_pass_and_quotes_a_real_failure() {
    if Command::new("bwrap").arg("--version").output().is_err() {
        return;
    }
    let home = std::env::var("HOME").unwrap();
    let spec = serde_json::json!({
        "kind": "workspace", "network": false,
        "binds": [
            {"path": format!("{home}/.cargo"), "mode": "overlay", "mask": ["credentials.toml", "credentials"]},
            {"path": format!("{home}/.rustup"), "mode": "ro"}
        ]
    })
    .to_string();
    for (failing, expected) in [(false, "pass"), (true, "fail")] {
        let work = verifier_workspace(failing);
        let output = Command::new(brokkr_bin())
            .args([
                "hands",
                "exec",
                "--workdir",
                work.path().to_str().unwrap(),
                "--spec",
                &spec,
                "--",
                "bash",
                "scripts/verify-seat.sh",
                "prompt.md",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value =
            serde_json::from_slice(&std::fs::read(work.path().join("result.json")).unwrap())
                .unwrap();
        assert_eq!(result["result"], expected, "{result}");
        if failing {
            let notes = result["notes"].as_str().unwrap();
            assert!(notes.contains("DECISIVE\tRED\rLINE"), "{result}");
        }
    }
}

#[test]
fn ship_seat_exits_nonzero_when_ledger_generation_fails() {
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join("scripts")).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ship-seat.sh"),
        work.path().join("scripts/ship-seat.sh"),
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(work.path())
        .status()
        .unwrap()
        .success());
    let result = work.path().join("result.json");
    std::fs::write(
        work.path().join("prompt.md"),
        format!(
            "Run context (journal-derived, read-only):\n```json\n{{\n  \"journal\": \"state/custom.db\",\n  \"last_decision\": {{\n    \"inputs\": {{\n      \"rule_id\": \"SHIP-READY\"\n    }},\n    \"rule_id\": \"REVIEW-CLEAN\"\n  }},\n  \"run_id\": \"known-run\"\n}}\n```\n\n## Result contract — MANDATORY\n\n    {}\n",
            result.display()
        ),
    )
    .unwrap();
    let output = Command::new("bash")
        .args(["scripts/ship-seat.sh", "prompt.md", "/bin/false"])
        .current_dir(work.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!result.exists(), "a failed ledger must not type ready");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ledger generation failed"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_machine_proof_runs_the_boxed_verify_and_ship_scripts_end_to_end() {
    if Command::new("bwrap").arg("--version").output().is_err() {
        return;
    }
    let work = verifier_workspace(false);
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ship-seat.sh"),
        work.path().join("scripts/ship-seat.sh"),
    )
    .unwrap();
    std::fs::create_dir_all(work.path().join("adapters")).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/exec.json"),
        work.path().join("adapters/exec.json"),
    )
    .unwrap();
    let bundle = work.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("policy.json"),
        r#"{
      "schema":"forge.phase-machine/v1","phases":["verify","ship","done","stop"],
      "initial":"verify","terminal":["done","stop"],"shippable_from":["ship"],"rules":[
        {"id":"V","from":"verify","result":"pass","next":"ship","reason":"green"},
        {"id":"VF","from":"verify","result":"fail","next":"stop","reason":"red"},
        {"id":"SHIP-READY","from":"ship","result":"ready","next":"ship","reason":"ledger"},
        {"id":"S","from":"ship","result":"shipped","next":"done","reason":"closed"}
      ]}"#,
    )
    .unwrap();
    let home = std::env::var("HOME").unwrap();
    let hands = serde_json::json!({"kind":"workspace","network":false,"binds":[
        {"path":format!("{home}/.cargo"),"mode":"overlay","mask":["credentials.toml","credentials"]},
        {"path":format!("{home}/.rustup"),"mode":"ro"}
    ]});
    let config = serde_json::json!({"name":"script-proof","policy":"policy.json","protected_phase":"ship","seats":{
        "verify":{"results":["pass","fail"],"class":"gate","hands":hands.clone(),
          "driver":{"command":["{brokkr}","driver","exec","--","bash","scripts/verify-seat.sh","{prompt_file}"]}},
        "ship":{"results":["ready","shipped"],"class":"gate","hands":hands,
          "driver":{"command":["{brokkr}","driver","exec","--","bash","scripts/ship-seat.sh","{prompt_file}","{brokkr}"]}}
    }});
    std::fs::write(
        bundle.join("bundle.json"),
        serde_json::to_vec_pretty(&config).unwrap(),
    )
    .unwrap();
    std::fs::write(work.path().join(".gitignore"), ".forge/\ntarget/\n").unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["config", "user.email", "proof@example.invalid"],
        vec!["config", "user.name", "Proof"],
        vec!["config", "commit.gpgsign", "false"],
        vec!["add", "."],
        vec!["commit", "-q", "-m", "proof"],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(work.path())
            .status()
            .unwrap()
            .success());
    }
    let output = Command::new(brokkr_bin())
        .args([
            "run",
            "--bundle",
            "bundle",
            "--feature",
            "script proof",
            "--db",
            "state/custom.db",
            "--repo",
            work.path().to_str().unwrap(),
        ])
        .current_dir(work.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let ledger_dir = work.path().join(".forge/ledger");
    assert_eq!(std::fs::read_dir(ledger_dir).unwrap().count(), 1);
}
