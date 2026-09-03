use super::*;
use std::io::Cursor;

fn spec_of(raw: Value) -> HandsSpec {
    HandsSpec::parse(&raw).unwrap()
}

#[test]
fn the_spec_vocabulary_is_closed_and_the_string_is_the_default() {
    assert_eq!(spec_of(json!("workspace")), HandsSpec::default());
    let full = spec_of(json!({
        "kind": "workspace",
        "network": true,
        "binds": [
            {"path": "~/.cargo", "mode": "rw", "mask": ["credentials.toml"]},
            {"path": "/opt/toolchain", "mode": "ro"}
        ]
    }));
    assert!(full.network);
    assert_eq!(full.binds.len(), 2);
    assert!(full.binds[0].writable);
    assert_eq!(full.binds[0].mask, ["credentials.toml"]);
    assert!(!full.binds[1].writable);
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

#[test]
fn the_namespace_is_built_from_an_empty_root_and_binds_what_the_spec_names() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let cargo = home.join(".cargo");
    std::fs::create_dir_all(&cargo).unwrap();
    std::fs::write(cargo.join("credentials.toml"), "secret").unwrap();
    let workdir = dir.path().join("work");
    std::fs::create_dir_all(&workdir).unwrap();
    let scratch = dir.path().join("scratch");
    let spec = spec_of(json!({
        "kind": "workspace",
        "binds": [
            {"path": "~/.cargo", "mode": "rw", "mask": ["credentials.toml", "absent.toml"]},
            {"path": "~/.rustup", "mode": "ro"}
        ]
    }));
    let argv = box_argv(&spec, &workdir, &home, &scratch, &["true".to_string()]).unwrap();
    let text = argv.join(" ");
    assert_eq!(argv[0], "bwrap");
    assert!(text.contains("--unshare-net"), "no network by default");
    assert!(text.contains("--clearenv"));
    assert!(!text.contains("--bind / /"), "never the host root");
    assert!(text.contains(&format!("--bind {w} {w}", w = workdir.display())));
    assert!(text.contains(&format!("--bind-try {c} {c}", c = cargo.display())));
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
    assert!(text.ends_with(&format!("--chdir {} -- true", workdir.display())));
    assert!(scratch.join("etc/passwd").is_file());
    assert!(scratch.join("home").is_dir());

    let open = spec_of(json!({"kind": "workspace", "network": true}));
    let argv = box_argv(&open, &workdir, &home, &scratch, &["true".to_string()]).unwrap();
    assert!(!argv.iter().any(|part| part == "--unshare-net"));

    // A scratch that cannot be created is an error, never a half box.
    let blocked = dir.path().join("file");
    std::fs::write(&blocked, "").unwrap();
    assert!(box_argv(
        &open,
        &workdir,
        &home,
        &blocked.join("etc"),
        &["true".to_string()]
    )
    .is_err());
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
}

#[test]
fn the_rendering_labels_every_section() {
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
    assert_eq!(bounded(b"small"), "small");
    let big = format!("a{}", "é".repeat(OUTPUT_BYTES));
    let cut = bounded(big.as_bytes());
    assert!(cut.ends_with(&format!("[output truncated at {OUTPUT_BYTES} bytes]")));
    assert!(cut.len() < big.len());
}

fn fake_run(_: &HandsSpec, _: &Path, command: &str, timeout: Duration) -> Result<Executed, String> {
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
    serve(Cursor::new(input), &mut output, workdir, &spec, &fake_run).unwrap();
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
        let reply = handle(&line, workdir, &spec, &fake_run).unwrap();
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
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    assert!(serve(
        Cursor::new(request),
        FailingWriter,
        Path::new("/w"),
        &spec,
        &fake_run
    )
    .is_err());
    let invalid: &[u8] = &[0xff, b'\n'];
    let mut sink = Vec::new();
    assert!(serve(
        Cursor::new(invalid),
        &mut sink,
        Path::new("/w"),
        &spec,
        &fake_run
    )
    .is_err());
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
    if require_bwrap().is_err() {
        panic!("bubblewrap is required on Linux for the hands tests: install bwrap");
    }
    let dir = tempfile::tempdir().unwrap();
    let workdir = dir.path().join("work");
    std::fs::create_dir_all(&workdir).unwrap();
    std::fs::write(workdir.join("note.txt"), "inside\n").unwrap();
    let host_secret = dir.path().join("host-secret");
    std::fs::write(&host_secret, "never\n").unwrap();
    let spec = HandsSpec::default();

    let seen = execute(
        &spec,
        &workdir,
        "cat note.txt && echo $HOME && pwd",
        Duration::from_secs(30),
    )
    .unwrap();
    assert_eq!(seen.exit_code, 0, "{}", seen.render());
    assert!(seen.stdout.contains("inside"));
    assert!(seen.stdout.contains(SANDBOX_HOME));
    assert!(seen.stdout.contains(&workdir.display().to_string()));

    let hidden = execute(
        &spec,
        &workdir,
        &format!("cat {}", host_secret.display()),
        Duration::from_secs(30),
    )
    .unwrap();
    assert_ne!(hidden.exit_code, 0, "the host file must be unreachable");
    assert!(!hidden.stdout.contains("never"));

    let wrote = execute(
        &spec,
        &workdir,
        "echo written > made.txt",
        Duration::from_secs(30),
    )
    .unwrap();
    assert_eq!(wrote.exit_code, 0, "{}", wrote.render());
    assert_eq!(
        std::fs::read_to_string(workdir.join("made.txt")).unwrap(),
        "written\n"
    );

    let killed = execute(&spec, &workdir, "kill -9 $$", Duration::from_secs(30)).unwrap();
    assert!(!killed.timed_out);
    // bwrap reports a child killed by a signal as 128 + the signal.
    assert_eq!(killed.exit_code, 137, "{}", killed.render());

    let slow = execute(&spec, &workdir, "sleep 5", Duration::from_millis(300)).unwrap();
    assert!(slow.timed_out);
    assert!(slow.render().contains("[timed out]"));

    // A bwrap that is not there, and a scratch that cannot be made.
    let missing = execute_in(
        &dir.path().join("no-bwrap"),
        &spec,
        &workdir,
        dir.path(),
        &dir.path().join("scratch"),
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
        "true",
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert!(blocked.contains("namespace"), "{blocked}");
}
