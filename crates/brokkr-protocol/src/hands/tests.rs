use super::*;
use std::io::Cursor;

fn spec_of(raw: Value) -> HandsSpec {
    HandsSpec::parse(&raw).unwrap()
}

fn one(text: &str) -> Vec<String> {
    vec![text.to_string()]
}

#[cfg(target_os = "linux")]
fn can_create_namespace() -> bool {
    if std::env::var_os(HANDS_BOX_ENV).is_some() {
        return false;
    }
    let Ok(bwrap) = require_bwrap() else {
        return false;
    };
    Command::new(bwrap)
        .args(["--ro-bind", "/", "/", "--", "true"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn the_spec_vocabulary_is_closed_and_the_string_is_the_default() {
    assert_eq!(spec_of(json!("workspace")), HandsSpec::default());
    let full = spec_of(json!({
        "kind": "workspace",
        "network": true,
        "binds": [
            {"path": "~/.cargo", "mode": "rw", "mask": ["credentials.toml"]},
            {"path": "/opt/toolchain", "mode": "ro"},
            {"path": "~/.cache", "mode": "overlay"}
        ]
    }));
    assert!(full.network);
    assert_eq!(full.binds.len(), 3);
    assert_eq!(full.binds[0].mode, BindMode::Rw);
    assert_eq!(full.binds[0].mask, ["credentials.toml"]);
    assert_eq!(full.binds[1].mode, BindMode::Ro);
    assert_eq!(full.binds[2].mode, BindMode::Overlay);
    assert_eq!(HandsSpec::parse(&full.to_value()).unwrap(), full);
    assert_eq!(spec_of(json!({"kind": "workspace"})), HandsSpec::default());

    for (bad, names) in [
        (json!("gloves"), "not a known kind"),
        (json!(7), "must be \"workspace\" or an object"),
        (
            json!({"kind": "workspace", "gloves": 1}),
            "unknown key 'gloves'",
        ),
        (json!({"kind": "mitten"}), "kind must be"),
        (json!({}), "kind must be"),
        (
            json!({"kind": "workspace", "network": "yes"}),
            "network must be a boolean",
        ),
        (
            json!({"kind": "workspace", "binds": {}}),
            "binds must be an array",
        ),
        (
            json!({"kind": "workspace", "binds": [7]}),
            "entry must be an object",
        ),
        (
            json!({"kind": "workspace", "binds": [{"path": "/x", "mode": "ro", "extra": 1}]}),
            "unknown key 'extra'",
        ),
        (
            json!({"kind": "workspace", "binds": [{"mode": "ro"}]}),
            "needs a non-empty 'path'",
        ),
        (
            json!({"kind": "workspace", "binds": [{"path": "", "mode": "ro"}]}),
            "needs a non-empty 'path'",
        ),
        (
            json!({"kind": "workspace", "binds": [{"path": "/x", "mode": "rwx"}]}),
            "needs mode",
        ),
        (
            json!({"kind": "workspace", "binds": [{"path": "/x", "mode": "ro", "mask": "a"}]}),
            "mask must be an array",
        ),
        (
            json!({"kind": "workspace", "binds": [{"path": "/x", "mode": "ro", "mask": ["a/b"]}]}),
            "without a slash",
        ),
        (
            json!({"kind": "workspace", "binds": [{"path": "/x", "mode": "ro", "mask": [""]}]}),
            "without a slash",
        ),
    ] {
        let error = HandsSpec::parse(&bad).unwrap_err();
        assert!(error.contains(names), "{bad}: {error}");
    }
}

#[test]
fn home_expands_only_the_tilde_prefix() {
    let home = Path::new("/home/runner");
    assert_eq!(
        expand_home("~/.cargo", home),
        PathBuf::from("/home/runner/.cargo")
    );
    assert_eq!(expand_home("/opt/x", home), PathBuf::from("/opt/x"));
    assert_eq!(expand_home("~tilde", home), PathBuf::from("~tilde"));
}

/// Unix only: the argv it checks names Unix paths, and the box is Linux's.
#[cfg(unix)]
#[test]
fn the_namespace_is_built_from_an_empty_root_and_binds_what_the_spec_names() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let cargo = home.join(".cargo");
    let npm = home.join(".npm");
    std::fs::create_dir_all(&cargo).unwrap();
    std::fs::create_dir_all(&npm).unwrap();
    std::fs::write(cargo.join("credentials.toml"), "secret").unwrap();
    let workdir = dir.path().join("work");
    std::fs::create_dir_all(&workdir).unwrap();
    let bundle = dir.path().join("bundle");
    std::fs::create_dir_all(&bundle).unwrap();
    let scratch = dir.path().join("scratch");
    let session = dir.path().join("session");
    let spec = spec_of(json!({
        "kind": "workspace",
        "binds": [
            {"path": "~/.cargo", "mode": "overlay", "mask": ["credentials.toml", "absent.toml"]},
            {"path": "~/.rustup", "mode": "ro"},
            {"path": "/opt/scratchpad", "mode": "rw"},
            {"path": "~/.npm", "mode": "overlay"}
        ]
    }));
    let none = GitFacts::default();
    let argv = box_argv(
        &spec,
        &workdir,
        &home,
        &scratch,
        &session,
        &none,
        Some(&bundle),
        &one("true"),
    )
    .unwrap();
    let text = argv.join(" ");
    assert_eq!(argv[0], "bwrap");
    assert!(text.contains("--unshare-net"), "no network by default");
    assert!(text.contains("--clearenv"));
    assert!(text.contains("--setenv BROKKR_HANDS_BOX 1"));
    assert!(!text.contains("--bind / /"), "never the host root");
    assert!(text.contains(&format!("--bind {w} {w}", w = workdir.display())));
    assert!(text.contains(&format!("--ro-bind {} {SANDBOX_BUNDLE}", bundle.display())));
    let upper = session.join("overlay/0/upper");
    let work = session.join("overlay/0/work");
    assert!(
        text.contains(&format!(
            "--overlay-src {c} --overlay {u} {k} {c}",
            c = cargo.display(),
            u = upper.display(),
            k = work.display()
        )),
        "{text}"
    );
    assert!(
        upper.is_dir() && work.is_dir(),
        "the upper layer lives in the session"
    );
    assert!(text.contains(&format!(
        "--ro-bind /dev/null {}",
        cargo.join("credentials.toml").display()
    )));
    assert!(
        !text.contains("absent.toml"),
        "a mask over nothing binds nothing"
    );
    assert!(text.contains(&format!(
        "--ro-bind-try {r} {r}",
        r = home.join(".rustup").display()
    )));
    assert!(text.contains("--bind-try /opt/scratchpad /opt/scratchpad"));
    assert!(text.contains(&format!("--setenv NPM_CONFIG_CACHE {}", npm.display())));
    assert!(
        text.contains("--setenv GIT_CONFIG_KEY_0 commit.gpgsign --setenv GIT_CONFIG_VALUE_0 false")
    );
    assert!(!text.contains("--tmpfs"), "no git, nothing to mask");
    assert!(text.ends_with(&format!("--chdir {} -- true", workdir.display())));
    assert!(scratch.join("etc/passwd").is_file());
    assert!(scratch.join("home").is_dir());

    // A primary checkout: the git dir sits inside the worktree, so it is
    // not bound again, but its hooks are hidden and its config read-only.
    let inside = GitFacts {
        git_dir: Some(workdir.join(".git")),
        identity: vec![("GIT_AUTHOR_NAME".into(), "Seat".into())],
    };
    let argv = box_argv(
        &spec,
        &workdir,
        &home,
        &scratch,
        &session,
        &inside,
        None,
        &one("true"),
    )
    .unwrap();
    let text = argv.join(" ");
    assert!(!text.contains(&format!(
        "--bind {g} {g}",
        g = workdir.join(".git").display()
    )));
    assert!(text.contains(&format!("--tmpfs {}", workdir.join(".git/hooks").display())));
    assert!(text.contains(&format!(
        "--ro-bind-try {c} {c}",
        c = workdir.join(".git/config").display()
    )));
    assert!(text.contains("--setenv GIT_AUTHOR_NAME Seat"));

    // A `git worktree`: the common dir is elsewhere and must be bound.
    let common = dir.path().join("main/.git");
    let outside = GitFacts {
        git_dir: Some(common.clone()),
        identity: Vec::new(),
    };
    let argv = box_argv(
        &spec,
        &workdir,
        &home,
        &scratch,
        &session,
        &outside,
        None,
        &one("true"),
    )
    .unwrap();
    let text = argv.join(" ");
    assert!(text.contains(&format!("--bind {g} {g}", g = common.display())));
    assert!(text.contains(&format!("--tmpfs {}", common.join("hooks").display())));

    let open = spec_of(json!({"kind": "workspace", "network": true}));
    let argv = box_argv(
        &open,
        &workdir,
        &home,
        &scratch,
        &session,
        &none,
        None,
        &one("true"),
    )
    .unwrap();
    assert!(!argv.iter().any(|part| part == "--unshare-net"));

    // A scratch that cannot be created is an error, never a half box;
    // and so is an overlay layer that cannot be made.
    let blocked = dir.path().join("file");
    std::fs::write(&blocked, "").unwrap();
    assert!(box_argv(
        &open,
        &workdir,
        &home,
        &blocked.join("etc"),
        &session,
        &none,
        None,
        &one("true")
    )
    .is_err());
    let overlay_only =
        spec_of(json!({"kind": "workspace", "binds": [{"path": "/opt/x", "mode": "overlay"}]}));
    assert!(box_argv(
        &overlay_only,
        &workdir,
        &home,
        &scratch,
        &blocked.join("session"),
        &none,
        None,
        &one("true")
    )
    .is_err());
}

#[test]
fn overlays_need_a_bubblewrap_that_has_them() {
    assert_eq!(parse_version("bubblewrap 0.11.0"), Some((0, 11, 0)));
    assert_eq!(parse_version("bubblewrap 0.9.0"), Some((0, 9, 0)));
    assert_eq!(parse_version("bubblewrap 1.2"), Some((1, 2, 0)));
    assert_eq!(parse_version("bubblewrap"), None);
    assert_eq!(parse_version(""), None);
    assert_eq!(parse_version("bubblewrap x.y"), None);

    let plain = HandsSpec::default();
    let overlaid =
        spec_of(json!({"kind": "workspace", "binds": [{"path": "/opt/x", "mode": "overlay"}]}));
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-bwrap");
    assert!(
        overlay_supported(&plain, &missing).is_ok(),
        "no overlay, no question"
    );
    let refusal = overlay_supported(&overlaid, &missing).unwrap_err();
    assert!(refusal.contains("0.10 or newer"), "{refusal}");
    // The rule on the reported string: no script is written and executed
    // here, because another test's fork can hold a fresh file open and
    // turn its exec into "text file busy".
    for (reported, ok) in [
        ("bubblewrap 0.9.0", false),
        ("bubblewrap 0.10.0", true),
        ("bubblewrap 0.11.0", true),
        ("", false),
    ] {
        assert_eq!(
            overlay_supported_by(reported, &missing).is_ok(),
            ok,
            "{reported:?}"
        );
    }
    #[cfg(target_os = "linux")]
    if let Ok(real) = require_bwrap() {
        assert!(
            overlay_supported(&overlaid, &real).is_ok(),
            "this machine's bwrap has overlays"
        );
    }
}

#[test]
fn bwrap_is_required_not_simulated() {
    let dir = tempfile::tempdir().unwrap();
    let error = bwrap_on(dir.path().as_os_str()).unwrap_err();
    assert!(error.contains("never simulated"));
    let fake = dir.path().join("bwrap");
    std::fs::write(&fake, "").unwrap();
    assert_eq!(bwrap_on(dir.path().as_os_str()).unwrap(), fake);
    assert_eq!(
        io_context::<()>(Err(std::io::Error::other("boom")), "doing"),
        Err("hands doing: boom".to_string())
    );
    assert_eq!(io_context(Ok(1), "doing"), Ok(1));
    let session = session_dir("test").unwrap();
    assert!(session.is_dir());
    let _ = std::fs::remove_dir_all(&session);
}

#[test]
fn the_rendering_labels_every_section_and_output_is_bounded_while_draining() {
    let full = Executed {
        stdout: "out".into(),
        stderr: "err".into(),
        exit_code: 3,
        timed_out: true,
    };
    assert_eq!(
        full.render(),
        "out\n[stderr]\nerr\n[timed out]\n[exit code: 3]"
    );
    let quiet = Executed {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: 0,
        timed_out: false,
    };
    assert_eq!(quiet.render(), "[exit code: 0]");
    assert_eq!(rendered(b"small", false), "small");
    let big = vec![b'a'; OUTPUT_BYTES + 10];
    let (kept, truncated) = drain_bounded(Cursor::new(big));
    assert_eq!(kept.len(), OUTPUT_BYTES);
    assert!(truncated);
    assert!(rendered(&kept, truncated)
        .ends_with(&format!("[output truncated at {OUTPUT_BYTES} bytes]")));
    let (kept, truncated) = drain_bounded(Cursor::new(b"tiny".to_vec()));
    assert_eq!((kept.as_slice(), truncated), (&b"tiny"[..], false));
}

fn fake_run(
    _: &HandsSpec,
    _: &Path,
    _: &Path,
    command: &str,
    timeout: Duration,
) -> Result<Executed, String> {
    if command == "explode" {
        return Err("the box refused".to_string());
    }
    Ok(Executed {
        stdout: format!("ran {command} for {}ms", timeout.as_millis()),
        stderr: String::new(),
        exit_code: if command == "false" { 1 } else { 0 },
        timed_out: false,
    })
}

#[test]
fn the_server_speaks_mcp_over_newline_delimited_json_rpc() {
    let workdir = Path::new("/work");
    let spec = HandsSpec::default();
    let input = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
        "\n",
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"workspace","arguments":{"command":"ls","timeoutMs":500}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"workspace","arguments":{"command":"false"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"workspace","arguments":{"command":"explode"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":6,"method":"ping"}"#,
        "\n",
        r#"{"jsonrpc":"2.0","id":7,"method":"resources/list"}"#,
        "\n",
        "not json",
        "\n",
    );
    let mut output = Vec::new();
    serve(
        Cursor::new(input),
        &mut output,
        workdir,
        workdir,
        &spec,
        &fake_run,
    )
    .unwrap();
    let replies: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        replies.len(),
        8,
        "one reply per request, none per notification"
    );
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "brokkr-hands");
    assert_eq!(replies[1]["result"]["tools"][0]["name"], TOOL_NAME);
    assert_eq!(
        replies[2]["result"]["content"][0]["text"],
        "ran ls for 500ms\n[exit code: 0]"
    );
    assert_eq!(replies[2]["result"]["isError"], false);
    assert_eq!(replies[3]["result"]["isError"], true);
    assert_eq!(
        replies[4]["result"]["content"][0]["text"],
        "the box refused"
    );
    assert_eq!(replies[4]["result"]["isError"], true);
    assert_eq!(replies[5]["result"], json!({}));
    assert_eq!(replies[6]["error"]["code"], -32601);
    assert_eq!(replies[7]["error"]["code"], -32700);
    assert_eq!(replies[7]["id"], Value::Null);

    // The default protocol version when the client names none.
    let reply = handle(
        r#"{"jsonrpc":"2.0","id":9,"method":"initialize"}"#,
        workdir,
        workdir,
        &spec,
        &fake_run,
    )
    .unwrap();
    assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
}

#[test]
fn tool_arguments_are_exactly_the_two_the_schema_names() {
    let workdir = Path::new("/work");
    let spec = HandsSpec::default();
    for (params, names) in [
        (
            json!({"name": "other", "arguments": {"command": "ls"}}),
            "unknown tool",
        ),
        (
            json!({"name": "workspace", "arguments": {"command": "ls", "shell": "zsh"}}),
            "unknown argument",
        ),
        (
            json!({"name": "workspace", "arguments": {"command": "   "}}),
            "1-16384",
        ),
        (
            json!({"name": "workspace", "arguments": {"command": "a".repeat(16_385)}}),
            "1-16384",
        ),
        (json!({"name": "workspace"}), "1-16384"),
        (
            json!({"name": "workspace", "arguments": {"command": "ls", "timeoutMs": 5}}),
            "timeoutMs",
        ),
        (
            json!({"name": "workspace", "arguments": {"command": "ls", "timeoutMs": "long"}}),
            "timeoutMs",
        ),
    ] {
        let line = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params})
            .to_string();
        let reply = handle(&line, workdir, workdir, &spec, &fake_run).unwrap();
        assert_eq!(reply["error"]["code"], -32602, "{params}");
        assert!(
            reply["error"]["message"].as_str().unwrap().contains(names),
            "{params}: {reply}"
        );
    }
    let (command, timeout) =
        call_arguments(&json!({"name": "workspace", "arguments": {"command": "ls"}})).unwrap();
    assert_eq!(
        (command.as_str(), timeout),
        ("ls", Duration::from_millis(DEFAULT_TIMEOUT_MS))
    );
}

struct FailingWriter;
impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("closed"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn transport_errors_surface_instead_of_being_swallowed() {
    let spec = HandsSpec::default();
    let w = Path::new("/w");
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    assert!(serve(Cursor::new(request), FailingWriter, w, w, &spec, &fake_run).is_err());
    let invalid: &[u8] = &[0xff, b'\n'];
    let mut sink = Vec::new();
    assert!(serve(Cursor::new(invalid), &mut sink, w, w, &spec, &fake_run).is_err());
}

#[test]
fn the_harness_config_names_this_binary_and_the_spec() {
    let spec = spec_of(json!({"kind": "workspace", "network": true}));
    let config = mcp_config(
        Path::new("/usr/local/bin/brokkr"),
        Path::new("/work"),
        &spec,
    );
    assert_eq!(
        config["mcpServers"][SERVER_NAME]["command"],
        "/usr/local/bin/brokkr"
    );
    let args = serve_args(Path::new("/work"), &spec);
    assert_eq!(&args[..4], ["hands", "serve", "--workdir", "/work"]);
    assert_eq!(
        HandsSpec::parse(&serde_json::from_str(&args[5]).unwrap()).unwrap(),
        spec
    );
    let definition = tool_definition();
    assert_eq!(definition["inputSchema"]["required"], json!(["command"]));
}

/// The box, for real: Linux with bubblewrap on PATH, which is where the
/// boundary is meant to hold and the only place it is claimed to.
#[cfg(target_os = "linux")]
#[test]
fn the_box_hides_the_host_and_holds_the_worktree() {
    if !can_create_namespace() {
        eprintln!("skipped: this environment cannot create a bubblewrap namespace");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().join("work");
    std::fs::create_dir_all(&workdir).unwrap();
    std::fs::write(workdir.join("note.txt"), "inside\n").unwrap();
    let host_secret = dir.path().join("host-secret");
    std::fs::write(&host_secret, "never\n").unwrap();
    let spec = HandsSpec::default();
    let session = session_dir("test").unwrap();
    let long = Duration::from_secs(30);

    let seen = execute(
        &spec,
        &workdir,
        &session,
        "cat note.txt && echo $HOME && pwd",
        long,
    )
    .unwrap();
    assert_eq!(seen.exit_code, 0, "{}", seen.render());
    assert!(seen.stdout.contains("inside"));
    assert!(seen.stdout.contains(SANDBOX_HOME));
    assert!(seen.stdout.contains(&workdir.display().to_string()));

    let hidden = execute(
        &spec,
        &workdir,
        &session,
        &format!("cat {}", host_secret.display()),
        long,
    )
    .unwrap();
    assert_ne!(hidden.exit_code, 0, "the host file must be unreachable");
    assert!(!hidden.stdout.contains("never"));

    let wrote = execute(&spec, &workdir, &session, "echo written > made.txt", long).unwrap();
    assert_eq!(wrote.exit_code, 0, "{}", wrote.render());
    assert_eq!(
        std::fs::read_to_string(workdir.join("made.txt")).unwrap(),
        "written\n"
    );

    let killed = execute(&spec, &workdir, &session, "kill -9 $$", long).unwrap();
    assert!(!killed.timed_out);
    // bwrap reports a child killed by a signal as 128 + the signal.
    assert_eq!(killed.exit_code, 137, "{}", killed.render());

    let slow = execute(
        &spec,
        &workdir,
        &session,
        "sleep 5",
        Duration::from_millis(300),
    )
    .unwrap();
    assert!(slow.timed_out);
    assert!(slow.render().contains("[timed out]"));

    // Overlay: a write lands in the session's upper layer, never on the
    // host, and it is still there on the next call of the same session.
    let tool = dir.path().join("toolchain");
    std::fs::create_dir_all(tool.join("bin")).unwrap();
    std::fs::write(tool.join("bin/cargo"), "#!/bin/sh\necho host cargo\n").unwrap();
    let overlaid = spec_of(json!({
        "kind": "workspace",
        "binds": [{"path": tool.to_string_lossy(), "mode": "overlay"}]
    }));
    let bin = tool.join("bin/cargo");
    let wrote = execute(
        &overlaid,
        &workdir,
        &session,
        &format!("echo evil > {b} && cat {b}", b = bin.display()),
        long,
    )
    .unwrap();
    assert_eq!(wrote.exit_code, 0, "{}", wrote.render());
    assert!(wrote.stdout.contains("evil"), "the box sees its own write");
    assert!(
        std::fs::read_to_string(&bin)
            .unwrap()
            .contains("host cargo"),
        "the host does not"
    );
    let again = execute(
        &overlaid,
        &workdir,
        &session,
        &format!("cat {}", bin.display()),
        long,
    )
    .unwrap();
    assert!(
        again.stdout.contains("evil"),
        "the upper layer lives for the session"
    );

    // A bwrap that is not there, and a scratch that cannot be made.
    let none = GitFacts::default();
    let missing = execute_in(
        &dir.path().join("no-bwrap"),
        &spec,
        &workdir,
        dir.path(),
        &dir.path().join("scratch"),
        &session,
        &none,
        "true",
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(missing.contains("could not spawn bwrap"));
    let file = dir.path().join("plain");
    std::fs::write(&file, "").unwrap();
    let blocked = execute_in(
        &require_bwrap().unwrap(),
        &spec,
        &workdir,
        dir.path(),
        &file.join("scratch"),
        &session,
        &none,
        "true",
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(blocked.contains("namespace"), "{blocked}");
    let _ = std::fs::remove_dir_all(&session);
}

/// Ruling 6, for real: git works inside the box for a `git worktree`,
/// the seat commits unsigned under the host's identity, and neither
/// hooks nor config can be written from inside.
#[cfg(target_os = "linux")]
#[test]
fn git_works_in_the_box_and_cannot_plant_a_hook() {
    if !can_create_namespace() {
        eprintln!("skipped: this environment cannot create a bubblewrap namespace");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let main = dir.path().join("main");
    std::fs::create_dir_all(&main).unwrap();
    let git = |cwd: &Path, args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    git(&main, &["init", "-q", "-b", "main"]);
    git(&main, &["config", "user.name", "Host Operator"]);
    git(&main, &["config", "user.email", "host@example.invalid"]);
    git(&main, &["config", "commit.gpgsign", "true"]);
    git(&main, &["config", "gpg.program", "/nonexistent/gpg"]);
    std::fs::write(main.join("a.txt"), "a\n").unwrap();
    git(&main, &["add", "-A"]);
    git(&main, &["commit", "-q", "--no-gpg-sign", "-m", "base"]);
    let worktree = dir.path().join("wt");
    git(
        &main,
        &[
            "worktree",
            "add",
            "-q",
            worktree.to_str().unwrap(),
            "-b",
            "slice",
        ],
    );
    let hooks = main.join(".git/hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(hooks.join("pre-commit"), "#!/bin/sh\nexit 0\n").unwrap();

    let facts = git_facts(&worktree);
    assert_eq!(facts.git_dir.as_deref(), Some(main.join(".git").as_path()));
    assert!(facts
        .identity
        .iter()
        .any(|(k, v)| k == "GIT_COMMITTER_EMAIL" && v == "host@example.invalid"));
    git(&main, &["config", "user.name", ""]);
    assert!(!git_facts(&worktree)
        .identity
        .iter()
        .any(|(key, _)| key == "GIT_AUTHOR_NAME"));
    git(&main, &["config", "user.name", "Host Operator"]);
    // Not a repository: no git dir to mask (the identity is the host's
    // global one either way).
    assert_eq!(git_facts(dir.path()).git_dir, None);

    let spec = HandsSpec::default();
    let session = session_dir("git").unwrap();
    let long = Duration::from_secs(30);
    let status = execute(
        &spec,
        &worktree,
        &session,
        "git status --porcelain && echo ok",
        long,
    )
    .unwrap();
    assert_eq!(status.exit_code, 0, "{}", status.render());
    let committed = execute(
        &spec,
        &worktree,
        &session,
        "echo b > b.txt && git add b.txt && git commit -q -m boxed && git log -1 --format='%an <%ae> %G?'",
        long,
    )
    .unwrap();
    assert_eq!(committed.exit_code, 0, "{}", committed.render());
    assert!(
        committed
            .stdout
            .contains("Host Operator <host@example.invalid> N"),
        "unsigned, as the host: {}",
        committed.render()
    );
    assert_eq!(
        git(&worktree, &["log", "-1", "--format=%s"]),
        "boxed",
        "the commit reached the host's common dir"
    );
    let common = main.join(".git");
    let hooked = execute(
        &spec,
        &worktree,
        &session,
        &format!(
            "ls {c}/hooks | wc -l; echo evil > {c}/hooks/post-checkout",
            c = common.display()
        ),
        long,
    )
    .unwrap();
    assert!(
        hooked.stdout.starts_with('0'),
        "hooks are an empty tmpfs: {}",
        hooked.render()
    );
    assert!(
        !hooks.join("post-checkout").exists(),
        "nothing written to a hook reaches the host"
    );
    let config = execute(
        &spec,
        &worktree,
        &session,
        "git config --local core.hooksPath /evil",
        long,
    )
    .unwrap();
    assert_ne!(
        config.exit_code,
        0,
        "config is read-only: {}",
        config.render()
    );
    assert!(!std::fs::read_to_string(common.join("config"))
        .unwrap()
        .contains("evil"));
    let _ = std::fs::remove_dir_all(&session);
}
