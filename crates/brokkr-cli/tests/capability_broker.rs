//! `brokkr broker serve` (decision 0065 slice two U6b; MB3, MB4, SD3),
//! driven through the real binary. The command takes a bounded plan
//! locator and digest and nothing else: no server argv, grant or secret
//! value is an option. No plan is bound to any attempt before the later
//! units bind one, so a manual invocation refuses before anything starts,
//! and the compile fence still refuses every MCP grant.

use std::path::PathBuf;
use std::process::Command;

use serde_json::json;

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// What one invocation left: its exit code, stdout and stderr.
struct Ran {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// A canonicalised temporary root every fixture is built under.
struct Root {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl Root {
    fn new() -> Root {
        let dir = tempfile::tempdir().unwrap();
        Root {
            path: dir.path().canonicalize().unwrap(),
            _dir: dir,
        }
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.path.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    fn brokkr(&self, args: &[&str]) -> Ran {
        let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(args)
            .current_dir(&self.path)
            .output()
            .unwrap();
        Ran {
            code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

/// The first line of a refusal clap printed, and the usage exit.
fn usage(ran: &Ran) -> (Option<i32>, &str) {
    (ran.code, ran.stderr.lines().next().unwrap_or_default())
}

/// A plan file whose server would leave a marker if it were ever started.
fn planted(root: &Root) -> (PathBuf, PathBuf) {
    let marker = root.path.join("started");
    let server = root.write(
        "server.sh",
        &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    );
    let plan = root.write(
        "plan.json",
        &json!({"server": {"argv": [server]}, "tools": ["lookup"]}).to_string(),
    );
    (plan, marker)
}

#[test]
fn a_manual_invocation_with_a_real_file_and_its_digest_is_unbound() {
    let root = Root::new();
    let (plan, marker) = planted(&root);
    let digest = brokkr_core::canonical::sha256_bytes(&std::fs::read(&plan).unwrap());
    let ran = root.brokkr(&[
        "broker",
        "serve",
        "--plan",
        plan.to_str().unwrap(),
        "--plan-digest",
        &digest,
    ]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        "error: broker plan is not bound to this attempt\n"
    );
    assert_eq!(ran.stdout, "");
    // The locator's server was never started.
    assert!(!marker.exists());
}

#[test]
fn the_plan_locator_and_digest_are_bounded_where_they_are_parsed() {
    let root = Root::new();
    let longest = format!("/{}", "a".repeat(1023));
    let over = format!("/{}", "a".repeat(1024));
    let refused = |plan: &str, digest: &str| {
        let ran = root.brokkr(&["broker", "serve", "--plan", plan, "--plan-digest", digest]);
        let (code, line) = usage(&ran);
        (code, line.to_string())
    };
    let invalid = |value: &str, arg: &str, why: &str| {
        (
            Some(2),
            format!("error: invalid value '{value}' for '{arg}': {why}"),
        )
    };
    assert_eq!(
        refused("plan.json", DIGEST),
        invalid(
            "plan.json",
            "--plan <PLAN>",
            "a plan locator is an absolute path"
        )
    );
    assert_eq!(
        refused("/run/../plan.json", DIGEST),
        invalid(
            "/run/../plan.json",
            "--plan <PLAN>",
            "a plan locator names no parent directory"
        )
    );
    assert_eq!(
        refused(&over, DIGEST),
        invalid(
            &over,
            "--plan <PLAN>",
            "a plan locator is at most 1024 bytes"
        )
    );
    for digest in [
        &DIGEST.to_uppercase(),
        &DIGEST[1..],
        &format!("sha256:{DIGEST}"),
    ] {
        assert_eq!(
            refused("/plan.json", digest),
            invalid(
                digest,
                "--plan-digest <PLAN_DIGEST>",
                "a plan digest is 64 lowercase hex characters"
            )
        );
    }
    // At the bound the locator parses, and the plan is still unbound.
    assert_eq!(
        refused(&longest, DIGEST),
        (
            Some(1),
            "error: broker plan is not bound to this attempt".to_string()
        )
    );
}

#[test]
fn no_server_argv_grant_or_secret_value_is_an_option() {
    let root = Root::new();
    let serve = |extra: &[&str]| {
        let mut args = vec![
            "broker",
            "serve",
            "--plan",
            "/plan.json",
            "--plan-digest",
            DIGEST,
        ];
        args.extend(extra);
        let ran = root.brokkr(&args);
        let (code, line) = usage(&ran);
        (code, line.to_string())
    };
    for option in [
        "--server", "--argv", "--grant", "--tools", "--secret", "--env",
    ] {
        assert_eq!(
            serve(&[option, "x"]),
            (
                Some(2),
                format!("error: unexpected argument '{option}' found")
            )
        );
    }
    assert_eq!(
        serve(&["--", "/usr/bin/docs-mcp"]),
        (
            Some(2),
            "error: unexpected argument '/usr/bin/docs-mcp' found".to_string()
        )
    );
    // Neither half of the locator may be left out.
    let bare = root.brokkr(&["broker", "serve", "--plan", "/plan.json"]);
    assert_eq!(
        usage(&bare),
        (
            Some(2),
            "error: the following required arguments were not provided:"
        )
    );
    assert_eq!(
        bare.stderr.lines().nth(1).map(str::trim),
        Some("--plan-digest <PLAN_DIGEST>")
    );
}

#[test]
fn the_grouped_library_verbs_still_dispatch() {
    let root = Root::new();
    let store = root.write("secrets.env", "DOCS_TOKEN=value\n");
    let private = std::os::unix::fs::PermissionsExt::from_mode(0o600);
    std::fs::set_permissions(&store, private).unwrap();
    let ran = root.brokkr(&["secrets", "list", "--secrets-file", "secrets.env"]);
    assert_eq!((ran.code, ran.stdout.as_str()), (Some(0), "DOCS_TOKEN\n"));
}

/// `brokkr init`'s workspace, whose map grants its one realm
/// `library-docs` through an MCP dialect that no seat asks for.
fn granting_mcp(root: &Root) {
    assert_eq!(root.brokkr(&["init", "."]).code, Some(0));
    let map = root.path.join("realms.json");
    let mut realms: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&map).unwrap()).unwrap();
    realms["schema"] = json!("forge.realms/v6");
    realms["realms"][0]["capabilities"] =
        json!({"library-docs": {"dialect": "docs-mcp", "offices": []}});
    root.write("realms.json", &realms.to_string());
    root.write(
        "capabilities/library-docs.json",
        &json!({"name": "library-docs", "classes": ["reads", "egress"]}).to_string(),
    );
    root.write(
        "dialects/tools/docs-mcp.json",
        &json!({"schema": "brokkr.tool-dialect/v1", "name": "docs-mcp",
                "serves": "library-docs", "kind": "mcp",
                "connection": {"argv": ["/nonexistent/docs-mcp"]}, "version": "1.4.2",
                "secrets": ["DOCS_TOKEN"], "tools": ["resolve", "read"], "retained": true,
                "sends": {"description": "a library name", "seat_composed": true}})
        .to_string(),
    );
}

#[test]
fn the_compile_fence_still_refuses_an_unused_mcp_grant() {
    let root = Root::new();
    granting_mcp(&root);
    let ran = root.brokkr(&["compile", "--bundle", "."]);
    assert_eq!(ran.code, Some(1));
    assert_eq!(
        ran.stderr,
        "error: bundle: realm 'starter' grants capability 'library-docs' through dialect 'docs-mcp' of \
         kind 'mcp', whose broker support is not implemented until decision 0065 slice two\n"
    );
    assert_eq!(ran.stdout, "");
}
