//! Decision 0043 through the binary: `brokkr hands serve` speaks MCP on
//! stdio and `brokkr hands exec` runs a command whole inside the box.
//! Linux only, like the boundary.

#![cfg(target_os = "linux")]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Ask the boundary itself, quietly, before a test claims it can build a box.
/// Nesting is refused by the engine-owned marker, not by an accidental kernel
/// policy or namespace nesting limit.
fn namespace_probe(program: &str) -> bool {
    Command::new(program)
        .args(["--ro-bind", "/", "/", "--", "true"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn can_create_namespace() -> bool {
    if std::env::var_os(brokkr_protocol::hands::HANDS_BOX_ENV).is_some() {
        return false;
    }
    namespace_probe("bwrap")
}

#[test]
fn namespace_probe_requires_a_successful_namespace_not_only_a_binary() {
    assert!(!namespace_probe("/bin/false"));
    assert!(!namespace_probe("/definitely/not/a/binary"));
}

#[test]
fn hands_serve_lists_one_tool_and_runs_it_in_the_box() {
    if !can_create_namespace() {
        return;
    }
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
    if !can_create_namespace() {
        return;
    }
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

    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::write(
        bundle.join("outside.sh"),
        "#!/bin/sh\nprintf 'bundle script ran\\n' > bundle-ran.txt\n",
    )
    .unwrap();
    let outside = Command::new(brokkr_bin())
        .args([
            "hands",
            "exec",
            "--workdir",
            work.to_str().unwrap(),
            "--bundle-root",
            bundle.to_str().unwrap(),
            "--",
            "sh",
            "/runtime/bundle/outside.sh",
        ])
        .output()
        .unwrap();
    assert!(
        outside.status.success(),
        "{}",
        String::from_utf8_lossy(&outside.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(work.join("bundle-ran.txt")).unwrap(),
        "bundle script ran\n"
    );
}

fn verifier_workspace() -> tempfile::TempDir {
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
    std::fs::write(
        dir.path().join("src/main.rs"),
        r#"fn main() {}
#[test]
fn named_pass() { assert_eq!(2 + 2, 4); }
#[test]
fn named_fail_when_requested() {
    if std::path::Path::new("FAIL").exists() {
        panic!("FAILED DECISIVE\tRED\rLINE");
    }
}
"#,
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

/// A deliberately tiny named suite proves both result arms. If its Cargo
/// process encounters any box-building test, that test sees `BROKKR_HANDS_BOX`
/// and skips explicitly; termination never depends on the host kernel refusing
/// another namespace.
#[test]
fn boxed_verify_seat_reports_pass_and_quotes_a_real_failure() {
    if !can_create_namespace() {
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
    let work = verifier_workspace();
    for (failing, expected) in [(false, "pass"), (true, "fail")] {
        if failing {
            std::fs::write(work.path().join("FAIL"), "fail this named test\n").unwrap();
        }
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
            let failed_test = notes.find("named_fail_when_requested").unwrap();
            let panic = notes.find("panicked at").unwrap();
            let failed_summary = notes.find("test result: FAILED").unwrap();
            let counts = notes.find("counts:").unwrap();
            assert!(failed_test < panic, "{result}");
            assert!(panic < failed_summary, "{result}");
            assert!(failed_summary < counts, "{result}");
        }
    }
}

/// Compile and execute a shipped exec site, not a hand-built `hands exec`
/// argv. The repository deliberately has no scripts directory: reaching the
/// terminal verify-fail rule proves the `./` entry resolved through the
/// shipped bundle root inside the box and wrote a valid result.
#[test]
fn a_shipped_compiled_exec_site_runs_its_script_outside_the_worktree() {
    if !can_create_namespace() {
        return;
    }
    let work = verifier_workspace();
    std::fs::remove_dir_all(work.path().join("scripts")).unwrap();
    std::fs::write(work.path().join("FAIL"), "select the named failing test\n").unwrap();
    let journal = tempfile::tempdir().unwrap();
    let root = workspace().canonicalize().unwrap();
    let output = Command::new(brokkr_bin())
        .args([
            "run",
            "--bundle",
            root.join("bundles/verify").to_str().unwrap(),
            "--feature",
            "prove the shipped bundle script mount",
            "--db",
            journal.path().join("canonical.db").to_str().unwrap(),
            "--repo",
            work.path().to_str().unwrap(),
        ])
        .current_dir(&root)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(3),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

fn closeout_result(recorded: Option<&str>, dirty: bool) -> Value {
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join("scripts")).unwrap();
    std::fs::copy(
        workspace().join("scripts/ship-seat.sh"),
        work.path().join("scripts/ship-seat.sh"),
    )
    .unwrap();
    for args in [
        ["init", "-q"].as_slice(),
        ["config", "user.email", "proof@example.invalid"].as_slice(),
        ["config", "user.name", "Proof"].as_slice(),
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(work.path())
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(work.path().join("tracked"), "clean\n").unwrap();
    std::fs::write(
        work.path().join(".gitignore"),
        ".forge/\nprompt.md\nresult.json\nscripts/\n",
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["add", "tracked", ".gitignore"])
        .current_dir(work.path())
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .args(["-c", "commit.gpgsign=false", "commit", "-qm", "base"])
        .current_dir(work.path())
        .status()
        .unwrap()
        .success());
    let head = String::from_utf8(
        Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(work.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    if let Some(recorded) = recorded {
        let recorded = if recorded == "HEAD" {
            head.trim()
        } else {
            recorded
        };
        std::fs::create_dir_all(work.path().join(".forge/ledger")).unwrap();
        std::fs::write(
            work.path().join(".forge/ledger/known-run.md"),
            format!("Repository head: `{recorded}`\n"),
        )
        .unwrap();
    }
    if dirty {
        std::fs::write(work.path().join("tracked"), "dirty\n").unwrap();
    }
    let result = work.path().join("result.json");
    std::fs::write(
        work.path().join("prompt.md"),
        format!(
            "Run context (journal-derived, read-only):\n```json\n{{\n  \"last_decision\": {{\n    \"rule_id\": \"SHIP-READY\"\n  }},\n  \"run_id\": \"known-run\"\n}}\n```\n\n    {}\n",
            result.display()
        ),
    )
    .unwrap();
    let output = Command::new("bash")
        .args(["scripts/ship-seat.sh", "prompt.md"])
        .current_dir(work.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap()
}

#[test]
fn ship_closeout_states_every_discrepancy_plainly() {
    for (recorded, dirty, phrase) in [
        (None, false, "ledger .forge/ledger/known-run.md is missing"),
        (Some("0000000"), true, "close-out discrepancies"),
        (Some("HEAD"), true, "HEAD still matches ledger"),
        (Some("0000000"), false, "worktree is clean"),
        (Some("HEAD"), false, "close-out confirmed"),
    ] {
        let result = closeout_result(recorded, dirty);
        assert_eq!(result["result"], "shipped");
        assert!(
            result["notes"].as_str().unwrap().contains(phrase),
            "{result}"
        );
    }
}

#[test]
fn ship_first_entry_records_a_dirty_tree_after_writing_the_ledger() {
    let work = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(work.path().join("scripts")).unwrap();
    std::fs::copy(
        workspace().join("scripts/ship-seat.sh"),
        work.path().join("scripts/ship-seat.sh"),
    )
    .unwrap();
    assert!(Command::new("git")
        .args(["init", "-q"])
        .current_dir(work.path())
        .status()
        .unwrap()
        .success());
    std::fs::write(work.path().join("dirty"), "yes\n").unwrap();
    let fake = work.path().join("ledger-command");
    std::fs::write(
        &fake,
        "#!/bin/sh\nmkdir -p .forge/ledger\nprintf 'Repository head: `none`\\n' > .forge/ledger/known-run.md\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(&fake).unwrap().permissions();
    use std::os::unix::fs::PermissionsExt;
    permissions.set_mode(0o755);
    std::fs::set_permissions(&fake, permissions).unwrap();
    let result = work.path().join("result.json");
    std::fs::write(
        work.path().join("prompt.md"),
        format!(
            "```json\n{{\n  \"journal\": \"journal.db\",\n  \"last_decision\": {{\n    \"rule_id\": \"REVIEW-CLEAN\"\n  }},\n  \"run_id\": \"known-run\"\n}}\n```\n\n    {}\n",
            result.display()
        ),
    )
    .unwrap();
    let output = Command::new("bash")
        .args(["scripts/ship-seat.sh", "prompt.md", fake.to_str().unwrap()])
        .current_dir(work.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap();
    assert_eq!(result["result"], "ready");
    assert!(result["notes"]
        .as_str()
        .unwrap()
        .contains("worktree discrepancy"));
}

#[test]
fn a_machine_proof_runs_the_boxed_verify_and_ship_scripts_end_to_end() {
    if !can_create_namespace() {
        return;
    }
    let work = verifier_workspace();
    let journal = tempfile::tempdir().unwrap();
    let journal_db = journal.path().join("canonical.db");
    std::fs::remove_dir_all(work.path().join("scripts")).unwrap();
    std::fs::create_dir_all(work.path().join("adapters")).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/exec.json"),
        work.path().join("adapters/exec.json"),
    )
    .unwrap();
    let strategy = tempfile::tempdir().unwrap();
    let bundle = strategy.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    std::fs::create_dir_all(bundle.join("scripts")).unwrap();
    for script in ["verify-seat.sh", "ship-seat.sh"] {
        std::fs::copy(
            workspace().join("scripts").join(script),
            bundle.join("scripts").join(script),
        )
        .unwrap();
    }
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
          "driver":{"command":["{brokkr}","driver","exec","--","bash","./scripts/verify-seat.sh","{prompt_file}"]}},
        "ship":{"results":["ready","shipped"],"class":"gate","hands":{"kind":"workspace","network":false,"binds":[]},
          "driver":{"command":["{brokkr}","driver","exec","--","bash","./scripts/ship-seat.sh","{prompt_file}","{brokkr}"]}}
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
            bundle.to_str().unwrap(),
            "--feature",
            "script proof",
            "--db",
            journal_db.to_str().unwrap(),
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
