use super::*;
use std::sync::Mutex;

static ADAPTER_ENV: Mutex<()> = Mutex::new(());

fn binding(name: &str, value: &str) -> secret::BoundSecret {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("secrets.env");
    secret::store_set(&store, name, value).unwrap();
    secret::resolve_bindings(&store, &[name.to_string()])
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
}

#[test]
fn exec_template_resolves_secret_refs_to_env_references_never_values() {
    let bindings = vec![binding("GH_TOKEN", "tok3n+v4lue!")];
    let resolved = resolve_exec_part(
        "curl -H 'auth: {{secret:GH_TOKEN}}' {workdir}/x",
        "/w",
        "",
        &bindings,
    );
    assert_eq!(resolved, "curl -H 'auth: $GH_TOKEN' /w/x");
    assert!(
        !resolved.contains("tok3n"),
        "the value never enters argv text"
    );
}

#[test]
fn exec_template_leaves_undeclared_refs_untouched() {
    // Compile refuses these in real bundles; standalone driver use
    // must still never invent a resolution.
    let resolved = resolve_exec_part("{{secret:OTHER}}", "/w", "", &[]);
    assert_eq!(resolved, "{{secret:OTHER}}");
}

#[test]
fn claude_fold_journals_file_paths_only_and_bash_stays_targetless() {
    // The 0012 amendment leaves the claude fold untouched: a Bash
    // tool_use (model-authored command) journals NO target; only
    // input.file_path ever becomes one.
    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    let event = json!({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": "Bash",
         "input": {"command": "curl -H 'auth: hunter22' https://x"}},
        {"type": "tool_use", "name": "Edit",
         "input": {"file_path": "src/lib.rs"}},
    ]}});
    fold_stream_event(&event, &mut turns, &mut meta, &mut |c| {
        emitted.push(c.clone())
    });
    assert_eq!(emitted.len(), 2);
    assert_eq!(emitted[0]["tool"], "Bash");
    assert!(emitted[0].get("target").is_none(), "{}", emitted[0]);
    assert!(
        !serde_json::to_string(&emitted[0])
            .unwrap()
            .contains("hunter22"),
        "the command text never reaches the checkpoint"
    );
    assert_eq!(emitted[1]["target"], "src/lib.rs");
}

#[test]
fn adapter_vocabulary_prompt_and_fold_edges_are_closed() {
    assert_eq!(AdapterKind::parse("claude"), Some(AdapterKind::Claude));
    assert_eq!(
        AdapterKind::parse("lanetally"),
        Some(AdapterKind::Lanetally)
    );
    assert_eq!(AdapterKind::parse("codex"), Some(AdapterKind::Codex));
    assert_eq!(AdapterKind::parse("dsh"), Some(AdapterKind::Dsh));
    assert_eq!(AdapterKind::parse("exec"), Some(AdapterKind::Exec));
    assert_eq!(AdapterKind::parse("invented"), None);
    assert_eq!(AdapterKind::Claude.driver_name(), "claude-code");
    assert_eq!(AdapterKind::Lanetally.driver_name(), "claude-lanetally");
    assert_eq!(AdapterKind::Codex.driver_name(), "codex");
    assert_eq!(AdapterKind::Dsh.driver_name(), "deepseek-harness");
    assert_eq!(AdapterKind::Exec.driver_name(), "exec");

    let dir = tempfile::tempdir().unwrap();
    let role = dir.path().join("role.md");
    std::fs::write(&role, "trusted role").unwrap();
    let prompt = compose_prompt(&json!({
        "role_path": role,
        "feature": "feature",
        "phase": "review",
        "workdir": "/work",
        "result_path": "/result.json",
        "context": {"fact": true},
        "allowed_results": ["clean", 2, "residual"],
    }));
    assert!(prompt.contains("trusted role"));
    assert!(prompt.contains("clean, residual"));
    assert!(prompt.contains("\"fact\": true"));

    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted = Vec::new();
    for event in [
        json!({"type": "system", "subtype": "init", "session_id": "session"}),
        json!({"type": "system", "subtype": "other"}),
        json!({"type": "system", "subtype": "init"}),
        json!({"type": "result", "num_turns": 2, "total_cost_usd": 1.5}),
        json!({"type": "result"}),
        json!({"type": "ignored"}),
    ] {
        fold_stream_event(&event, &mut turns, &mut meta, &mut |value| {
            emitted.push(value.clone())
        });
    }
    assert_eq!(meta["session_id"], "session");
    assert_eq!(meta["num_turns"], 2);

    for event in [
        json!({"type": "thread.started", "thread_id": "thread"}),
        json!({"type": "thread.started"}),
        json!({"type": "turn.started"}),
        json!({"type": "item.started", "item": {"type": "command"}}),
        json!({"type": "item.completed", "item": {}}),
        json!({"type": "turn.completed", "usage": {
            "input_tokens": 3, "cached_input_tokens": 2, "output_tokens": 1
        }}),
        json!({"type": "turn.completed"}),
        json!({"type": "result", "session_id": "final", "num_turns": 1}),
        json!({"type": "result"}),
        json!({"type": "ignored"}),
    ] {
        fold_codex_event(&event, &mut turns, &mut meta, &mut |value| {
            emitted.push(value.clone())
        });
    }
    assert_eq!(meta["session_id"], "final");
    assert_eq!(meta["cache_read_tokens"], 2);
    assert!(emitted.iter().any(|event| event["step"] == "item-started"));
    assert!(emitted.iter().any(|event| event["tool"] == "unknown"));
}

#[test]
fn cli_and_stderr_helpers_cover_empty_stdin_and_unicode_boundaries() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    struct BrokenWriter;
    impl Write for BrokenWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("closed"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(write_prompt(&mut BrokenWriter, "prompt")
        .unwrap_err()
        .contains("could not write the prompt"));
    assert!(run_cli(&[], None, "", &[])
        .unwrap_err()
        .contains("empty command"));
    let output = run_cli(
        &[
            "sh".into(),
            "-c".into(),
            "read value; printf %s \"$value\"".into(),
        ],
        Some("payload\n"),
        "",
        &[],
    )
    .unwrap();
    assert_eq!(output.stdout, b"payload");
    let output = run_cli(&["true".into()], None, "", &[]).unwrap();
    assert!(output.status.success());
    assert!(
        run_cli(&["forge-command-does-not-exist".into()], None, "", &[])
            .unwrap_err()
            .contains("could not invoke")
    );

    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("raw.env");
    std::fs::write(&store, b"TOKEN=abcd\xff\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let invalid = secret::resolve_bindings(&store, &["TOKEN".into()]).unwrap();
    assert!(run_cli(&["true".into()], None, "", &invalid)
        .unwrap_err()
        .contains("not valid UTF-8"));

    assert_eq!(
        adapter_binary("BROKKR_TEST_BINARY_NEVER_DEFINED", None, "fallback"),
        "fallback"
    );
    assert!(!adapter_binary("PATH", None, "fallback").is_empty());
    // The renamed overrides: the new spelling wins, the old one answers
    // when the new one is absent (decision 0019, one release).
    std::env::set_var("BROKKR_TEST_BINARY_RENAMED", "new");
    std::env::set_var("FORGE_TEST_BINARY_RENAMED", "old");
    assert_eq!(
        adapter_binary(
            "BROKKR_TEST_BINARY_RENAMED",
            Some("FORGE_TEST_BINARY_RENAMED"),
            "fallback"
        ),
        "new"
    );
    std::env::remove_var("BROKKR_TEST_BINARY_RENAMED");
    assert_eq!(
        adapter_binary(
            "BROKKR_TEST_BINARY_RENAMED",
            Some("FORGE_TEST_BINARY_RENAMED"),
            "fallback"
        ),
        "old"
    );
    std::env::remove_var("FORGE_TEST_BINARY_RENAMED");
    assert_eq!(io_context::<()>(Ok(()), "ok"), Ok(()));
    assert!(
        io_context::<()>(Err(std::io::Error::other("no")), "context")
            .unwrap_err()
            .contains("context: no")
    );
    let create_error = stage_prompt_with(
        "prompt",
        || Err(std::io::Error::other("create")),
        |_, _| Ok(()),
    );
    assert!(matches!(create_error, Err(message) if message.contains("create")));
    let write_error = stage_prompt_with("prompt", tempfile::NamedTempFile::new, |_, _| {
        Err(std::io::Error::other("write"))
    });
    assert!(matches!(write_error, Err(message) if message.contains("write")));

    let prompt_template = vec!["true".into(), "{prompt_file}".into()];
    let staged = invoke_with_stager(
        AdapterKind::Exec,
        &prompt_template,
        "prompt",
        &json!({}),
        &[],
        &mut |_| {},
        |_| Err("staging refused".into()),
    );
    assert!(matches!(staged, Err(message) if message == "staging refused"));

    assert_eq!(stderr_tail_start("short"), 0);
    let stderr = format!("{}é{}", "x".repeat(2), "y".repeat(3999));
    let start = stderr_tail_start(&stderr);
    assert_eq!(start, 2);
    assert!(stderr.is_char_boundary(start));
    assert_eq!(stderr.len() - start, 4001);

    let error = match invoke(
        AdapterKind::Exec,
        &[],
        "prompt",
        &json!({}),
        &[],
        &mut |_| {},
    ) {
        Ok(_) => panic!("empty exec template must fail"),
        Err(error) => error,
    };
    assert!(error.contains("command template"));
}

#[test]
fn run_seat_covers_absent_input_and_unparseable_result_evidence() {
    let mut messages = Vec::new();
    run_seat(
        AdapterKind::Exec,
        &[],
        &json!({"effect_id":"effect", "attempt_id":"attempt"}),
        &mut |body| messages.push(body),
    );
    assert!(matches!(messages[0], Body::Accepted { .. }));
    assert!(matches!(
        messages.last().unwrap(),
        Body::Result {
            status: ResultStatus::Failed,
            ..
        }
    ));

    let dir = tempfile::tempdir().unwrap();
    let result = dir.path().join("result.json");
    std::fs::write(&result, "nope").unwrap();
    let mut messages = Vec::new();
    run_seat(
        AdapterKind::Exec,
        &["sh".into(), "-c".into(), "cat >/dev/null".into()],
        &json!({
            "effect_id":"effect",
            "attempt_id":"attempt",
            "input": {
                "feature":"feature", "phase":"work", "workdir":dir.path(),
                "result_path":result, "allowed_results":["complete"]
            }
        }),
        &mut |body| messages.push(body),
    );
    assert!(matches!(
        messages.last().unwrap(),
        Body::Result {
            status: ResultStatus::Succeeded,
            result: Some(value),
            ..
        } if value.get("__unparseable_result_file__").is_some()
    ));
}

#[cfg(unix)]
fn executable(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&path, permissions).unwrap();
    path
}

#[cfg(unix)]
#[test]
fn claude_and_codex_cover_empty_workdir_stream_errors_and_prompt_pipe_refusals() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let invalid = executable(
        dir.path(),
        "invalid-output",
        "#!/bin/sh\ncat >/dev/null\nprintf '\\377'\n",
    );
    let closed = executable(
        dir.path(),
        "closed-stdin",
        "#!/bin/sh\nexec 0<&-\nsleep 0.1\n",
    );
    let prior_claude = std::env::var_os("FORGE_CLAUDE_BIN");
    let prior_codex = std::env::var_os("FORGE_CODEX_BIN");
    // Codex is pinned here through its OLD spelling, which the new one
    // outranks (decision 0019): an inherited BROKKR_CODEX_BIN would
    // send this test at a real codex, so it goes for the duration.
    let prior_brokkr_codex = std::env::var_os("BROKKR_CODEX_BIN");
    std::env::remove_var("BROKKR_CODEX_BIN");

    for (kind, variable) in [
        (AdapterKind::Claude, "FORGE_CLAUDE_BIN"),
        (AdapterKind::Codex, "FORGE_CODEX_BIN"),
    ] {
        std::env::set_var(variable, &invalid);
        assert!(invoke(kind, &[], "prompt", &json!({}), &[], &mut |_| {}).is_ok());

        std::env::set_var(variable, &closed);
        let prompt = "x".repeat(1_000_000);
        let error = match invoke(kind, &[], &prompt, &json!({}), &[], &mut |_| {}) {
            Ok(_) => panic!("closed stdin must refuse prompt delivery"),
            Err(error) => error,
        };
        assert!(error.contains("could not write the prompt"));
    }

    match prior_claude {
        Some(value) => std::env::set_var("FORGE_CLAUDE_BIN", value),
        None => std::env::remove_var("FORGE_CLAUDE_BIN"),
    }
    match prior_codex {
        Some(value) => std::env::set_var("FORGE_CODEX_BIN", value),
        None => std::env::remove_var("FORGE_CODEX_BIN"),
    }
    if let Some(value) = prior_brokkr_codex {
        std::env::set_var("BROKKR_CODEX_BIN", value);
    }
}

#[cfg(unix)]
#[test]
fn lanetally_capture_constant_is_inserted_after_the_session_meta_extend() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    // A stand-in wrapper whose result event tries to smuggle a
    // stream-derived `capture`: the run_seat source literal is inserted
    // AFTER the session_meta extend, so it wins by last-write-wins even
    // if the fold ever widens to copy such a key.
    let shim = executable(
        dir.path(),
        "stream-shim",
        "#!/bin/sh\ncat >/dev/null\nprintf '{\"type\":\"result\",\"num_turns\":1,\
         \"total_cost_usd\":0.5,\"capture\":\"evil\"}\\n'\n",
    );
    let result = dir.path().join("result.json");
    std::fs::write(&result, "{\"result\": \"complete\"}").unwrap();
    let start = json!({
        "effect_id":"fx", "attempt_id":"a1",
        "input": {"workdir": dir.path(), "result_path": result,
                  "allowed_results": ["complete"]}
    });
    let prior_lanetally = std::env::var_os("FORGE_LANETALLY_BIN");
    let prior_claude = std::env::var_os("FORGE_CLAUDE_BIN");
    std::env::set_var("FORGE_LANETALLY_BIN", &shim);
    std::env::set_var("FORGE_CLAUDE_BIN", &shim);
    let mut bodies = Vec::new();
    run_seat(AdapterKind::Lanetally, &[], &start, &mut |b| bodies.push(b));
    run_seat(AdapterKind::Claude, &[], &start, &mut |b| bodies.push(b));
    match prior_lanetally {
        Some(value) => std::env::set_var("FORGE_LANETALLY_BIN", value),
        None => std::env::remove_var("FORGE_LANETALLY_BIN"),
    }
    match prior_claude {
        Some(value) => std::env::set_var("FORGE_CLAUDE_BIN", value),
        None => std::env::remove_var("FORGE_CLAUDE_BIN"),
    }
    let finished: Vec<&Value> = bodies
        .iter()
        .filter_map(|body| match body {
            Body::Checkpoint { data, .. } => Some(data),
            _ => None,
        })
        .collect();
    assert_eq!(finished.len(), 2);
    assert_eq!(finished[0]["step"], "claude-lanetally-session-finished");
    assert_eq!(finished[0]["capture"], "lanetally");
    assert_eq!(finished[0]["total_cost_usd"], 0.5);
    // The guard is kind-scoped: claude's checkpoint carries no capture.
    assert_eq!(finished[1]["step"], "claude-code-session-finished");
    assert!(finished[1].get("capture").is_none(), "{}", finished[1]);
}

struct BrokenReader;

impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken input"))
    }
}

impl BufRead for BrokenReader {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        Err(std::io::Error::other("broken input"))
    }

    fn consume(&mut self, _: usize) {}
}

#[test]
fn adapter_stdio_propagates_reader_errors() {
    assert!(serve_io(AdapterKind::Exec, &[], BrokenReader, Vec::new()).is_err());
}

#[test]
fn adapter_stdio_ignores_noise_and_handles_control_messages() {
    let hello = serde_json::to_string(&Message::new(Body::Hello {
        engine_version: "test".into(),
    }))
    .unwrap();
    let cancel = serde_json::to_string(&Message::new(Body::Cancel {
        effect_id: "effect-1".into(),
    }))
    .unwrap();
    let input = format!("\nnot-json\n{{\"type\":\"unknown\"}}\n{hello}\n{cancel}\n");
    let mut output = Vec::new();
    serve_io(AdapterKind::Exec, &[], input.as_bytes(), &mut output).unwrap();
    let messages: Vec<Message> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(matches!(messages[0].body, Body::Capabilities { .. }));
    assert!(matches!(messages[1].body, Body::Cancelled { .. }));

    let shutdown = serde_json::to_string(&Message::new(Body::Shutdown)).unwrap();
    serve_io(AdapterKind::Exec, &[], shutdown.as_bytes(), Vec::new()).unwrap();
    serve_io(AdapterKind::Exec, &[], "".as_bytes(), Vec::new()).unwrap();
}

#[test]
fn init_journals_a_session_started_checkpoint_with_the_id() {
    // The id used to be stashed for the session-finished checkpoint
    // only, which meant a WORKING seat had no session id in the
    // journal — so the transcript drilldowns could not locate, let
    // alone live-stream, the prose being written. init carries the id
    // in the first stream message; it is journaled immediately.
    let mut turns = 0;
    let mut meta = serde_json::Map::new();
    let mut emitted: Vec<serde_json::Value> = Vec::new();
    fold_stream_event(
        &serde_json::json!({"type": "system", "subtype": "init",
                            "session_id": "abcd-1234-ef"}),
        &mut turns,
        &mut meta,
        &mut |value| emitted.push(value.clone()),
    );
    assert_eq!(emitted.len(), 1, "one checkpoint, immediately");
    assert_eq!(emitted[0]["step"], "session-started");
    assert_eq!(emitted[0]["session_id"], "abcd-1234-ef");
    assert_eq!(
        meta["session_id"], "abcd-1234-ef",
        "and the meta still feeds the finish"
    );

    // A non-string id is refused wholesale, not stringified.
    let mut emitted: Vec<serde_json::Value> = Vec::new();
    fold_stream_event(
        &serde_json::json!({"type": "system", "subtype": "init", "session_id": 7}),
        &mut turns,
        &mut meta,
        &mut |value| emitted.push(value.clone()),
    );
    assert!(emitted.is_empty(), "no string, no checkpoint");
}

#[cfg(unix)]
#[test]
fn dsh_driver_turns_the_model_pair_into_the_overlay_the_launcher_reads() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let argv = dir.path().join("argv");
    let overlay = dir.path().join("overlay.yml");
    // A stand-in launcher: records its argv one per line and keeps a
    // copy of whatever file follows `--patch`, which is gone by the
    // time the driver returns.
    let fake = executable(
        dir.path(),
        "dsh",
        &format!(
            "#!/bin/sh\ncat >/dev/null\n: > {argv}\nprev=\n\
             for a in \"$@\"; do printf '%s\\n' \"$a\" >> {argv}; \
             if [ \"$prev\" = --patch ]; then cp \"$a\" {overlay}; fi; prev=$a; done\n",
            argv = argv.display(),
            overlay = overlay.display()
        ),
    );
    let prior = std::env::var_os("BROKKR_DSH_BIN");
    let prior_legacy = std::env::var_os("FORGE_DSH_BIN");
    let prior_home = std::env::var_os("DSH_HOME");
    std::env::set_var("BROKKR_DSH_BIN", &fake);
    std::env::remove_var("FORGE_DSH_BIN");
    // The seat's transcript is kept under the harness home; in a test
    // that home is this test's own directory, never the operator's.
    std::env::set_var("DSH_HOME", dir.path());

    let extra: Vec<String> = ["--model", "deepseek-v4-flash", "--other", "kept"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut started = Vec::new();
    let invocation = invoke(
        AdapterKind::Dsh,
        &extra,
        "the prompt",
        &json!({"workdir": dir.path()}),
        &[],
        &mut |event| started.push(event.clone()),
    )
    .unwrap();

    let lines: Vec<String> = std::fs::read_to_string(&argv)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(&lines[..3], ["--profile", "headless", "--patch"]);
    assert!(lines[3].ends_with(".yml"), "{lines:?}");
    // The pair itself never reaches the launcher, which has no such
    // flag; the rest of `extra` does, in order, before the task.
    assert_eq!(&lines[4..], ["--other", "kept", "the prompt"]);
    assert!(!lines.iter().any(|l| l == "--model"), "{lines:?}");

    let written = std::fs::read_to_string(&overlay).unwrap();
    assert!(written.contains("- id: agent-default-model\n"), "{written}");
    assert!(
        written.contains("provider: deepseek-official\n"),
        "{written}"
    );
    assert!(written.contains("model: deepseek-v4-flash\n"), "{written}");
    assert!(
        !std::path::Path::new(&lines[3]).exists(),
        "the overlay is the seat's, not the host's: gone when the seat is"
    );

    assert_eq!(started[0]["step"], "harness-started");
    assert_eq!(started[0]["model"], "deepseek-v4-flash");
    assert_eq!(invocation.session_meta["model"], "deepseek-v4-flash");
    assert_eq!(invocation.session_meta["harness"], "deepseek");

    // No pair, no model row: the profile's own default model boots, and
    // the journal says so by naming none. The overlay itself is still
    // there — every seat pins its own transcript root through it, which
    // is how the driver follows the right session at all.
    let mut started = Vec::new();
    let invocation = invoke(
        AdapterKind::Dsh,
        &[],
        "p",
        &json!({"workdir": dir.path()}),
        &[],
        &mut |event| started.push(event.clone()),
    )
    .unwrap();
    let lines: Vec<String> = std::fs::read_to_string(&argv)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(&lines[..3], ["--profile", "headless", "--patch"]);
    assert_eq!(&lines[4..], ["p"]);
    let written = std::fs::read_to_string(&overlay).unwrap();
    assert!(
        written.contains("- id: session-persistence-jsonl\n"),
        "{written}"
    );
    assert!(!written.contains("agent-default-model"), "{written}");
    assert!(started[0]["model"].is_null());
    assert!(invocation.session_meta.get("model").is_none());

    match prior {
        Some(value) => std::env::set_var("BROKKR_DSH_BIN", value),
        None => std::env::remove_var("BROKKR_DSH_BIN"),
    }
    if let Some(value) = prior_legacy {
        std::env::set_var("FORGE_DSH_BIN", value);
    }
    match prior_home {
        Some(value) => std::env::set_var("DSH_HOME", value),
        None => std::env::remove_var("DSH_HOME"),
    }
}

#[test]
fn dsh_driver_refuses_a_dangling_or_doubled_or_malformed_model() {
    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(split_dsh_model(&s(&["--model"]))
        .unwrap_err()
        .contains("needs a model id"));
    assert!(split_dsh_model(&s(&["--model", "a", "--model", "b"]))
        .unwrap_err()
        .contains("twice"));
    let (model, rest) = split_dsh_model(&s(&["--x", "--model", "deepseek-v4-flash", "y"])).unwrap();
    assert_eq!(model.as_deref(), Some("deepseek-v4-flash"));
    assert_eq!(rest, s(&["--x", "y"]));
    // YAML is the overlay's grammar, so an id that could open a second
    // row or a value of its own is refused rather than written.
    let root = std::path::Path::new("/nonexistent/dsh-root");
    for bad in [
        "",
        "v4\n- id: hmr",
        "v4 # x",
        "a:b c",
        "\"quoted\"",
        "/qwen3.8-max",
        "dashscope/",
        "a/b/c",
    ] {
        assert!(
            dsh_seat_overlay(Some(bad), root).is_err(),
            "{bad:?} must be refused"
        );
    }
    assert!(dsh_seat_overlay(Some("deepseek-v4-flash"), root).is_ok());
}

#[test]
fn dsh_model_names_a_route_before_the_slash_and_the_official_one_without() {
    let official = parse_dsh_model("deepseek-v4-flash").unwrap();
    assert_eq!(
        (official.provider, official.model),
        ("deepseek-official", "deepseek-v4-flash")
    );
    let studio = parse_dsh_model("dashscope/deepseek-v4-flash-0731").unwrap();
    assert_eq!(
        (studio.provider, studio.model),
        ("dashscope", "deepseek-v4-flash-0731")
    );
    // The overlay carries the named route, not the default one.
    let file = dsh_seat_overlay(
        Some("dashscope/qwen3.8-max"),
        std::path::Path::new("/nonexistent/dsh-root"),
    )
    .unwrap();
    let written = std::fs::read_to_string(file.path()).unwrap();
    assert!(written.contains("provider: dashscope\n"), "{written}");
    assert!(written.contains("model: qwen3.8-max\n"), "{written}");
    assert!(!written.contains("deepseek-official"), "{written}");
}

// A stand-in launcher that behaves like dsh's headless profile does:
// it writes NOTHING to stdout until it is done, and appends its session
// transcript to the root the seat overlay pinned. It stops halfway and
// waits for `dsh-seen` — the file the driver's checkpoint sink touches —
// before writing its second turn. A driver that only folded at exit
// would never touch it, so the wait runs out; the shim then FAILS the
// seat (`exit 9`) rather than writing the turn anyway, because a turn
// written after the timeout is indistinguishable in the transcript from
// one written live. The bound is only there so a broken driver fails in
// seconds instead of hanging: the handshake IS the liveness proof, and
// it has to be enforced, not merely waited for.
const DSH_TRANSCRIPT_SHIM: &str = r#"#!/bin/sh
root=
prev=
for a in "$@"; do
  if [ "$prev" = --patch ]; then root=$(awk -F"'" '/^    root: /{print $2}' "$a"); fi
  prev=$a
done
d="$root/--project--/session-fake"
mkdir -p "$d"
f="$d/session.jsonl"
printf '{"type":"session","version":0,"id":"session-fake-1","cwd":"/w"}\n' > "$f"
printf 'not json, ignorable noise\n' >> "$f"
printf '{"type":"assistant/message","data":{"turn":1,"step":1,"usage":{"inputTokens":10,"outputTokens":2,"cacheReadTokens":4}}}\n' >> "$f"
printf '{"type":"tool/call","data":{"turn":1,"step":1,"name":"fs_write","arguments":"hunter22"}}\n' >> "$f"
printf '{"type":"assistant/chunk","data":{"turn":1,"step":2}}\n' >> "$f"
i=0
while [ ! -f dsh-seen ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done
[ -f dsh-seen ] || exit 9
printf '{"type":"assistant/message","data":{"turn":1,"step":2,"usage":{"inputTokens":20,"outputTokens":3}}}\n' >> "$f"
printf '{"type":"turn/end","data":{"turn":1,"reason":{"kind":"completed"}}}\n' >> "$f"
printf 'a last line still being written' >> "$f"
"#;

#[cfg(unix)]
#[test]
fn dsh_seat_journals_one_checkpoint_per_turn_while_the_child_still_runs() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = executable(dir.path(), "dsh", DSH_TRANSCRIPT_SHIM);
    let prior = std::env::var_os("BROKKR_DSH_BIN");
    let prior_legacy = std::env::var_os("FORGE_DSH_BIN");
    let prior_home = std::env::var_os("DSH_HOME");
    std::env::set_var("BROKKR_DSH_BIN", &fake);
    std::env::remove_var("FORGE_DSH_BIN");
    // The seat's transcript is kept under the harness home; in a test
    // that home is this test's own directory, never the operator's.
    std::env::set_var("DSH_HOME", dir.path());

    let seen = dir.path().join("dsh-seen");
    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke(
        AdapterKind::Dsh,
        &[],
        "the prompt",
        &json!({"workdir": dir.path()}),
        &[],
        &mut |event| {
            emitted.push(event.clone());
            if event["step"] == "seat-turn" {
                let _ = std::fs::write(&seen, b"");
            }
        },
    )
    .unwrap();

    // 9 is the shim's verdict that the driver never spoke while it ran:
    // the checkpoint sink touched nothing before the handshake ran out.
    assert_eq!(invocation.exit_code, 0, "{emitted:?}");
    assert_eq!(emitted[0]["step"], "harness-started");
    // Retention: the seat's root sits under this test's DSH_HOME, the
    // journal's clamped target is its prefix, and it is still there now
    // that the seat has concluded — the operator's, not the void's.
    let kept: Vec<_> = std::fs::read_dir(dir.path().join("sessions").join("brokkr"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(kept[0].file_name().to_string_lossy().starts_with("seat-"));
    assert!(kept[0].path().is_dir());
    let target = emitted[0]["target"].as_str().unwrap();
    assert!(
        kept[0].path().to_string_lossy().starts_with(target),
        "{target} vs {:?}",
        kept[0].path()
    );
    // The transcript root rides the started checkpoint as an ordinary
    // clamped target, so the seat's session stays addressable from the
    // journal alone once the run is over.
    assert!(target.contains("sessions/brokkr/seat-"), "{target}");
    assert!(target.chars().count() <= 80, "{target}");

    assert_eq!(emitted[1]["step"], "session-started");
    assert_eq!(emitted[1]["session_id"], "session-fake-1");
    let turns: Vec<&Value> = emitted
        .iter()
        .filter(|event| event["step"] == "seat-turn")
        .collect();
    assert_eq!(turns.len(), 3, "{emitted:?}");
    assert_eq!(turns[0]["turn"], 1);
    // 10 + 4: dsh counts a cache read beside its input, the journal
    // counts it inside — the key means one thing across drivers.
    assert_eq!(turns[0]["input_tokens"], 14);
    assert_eq!(turns[0]["cache_read_tokens"], 4);
    assert_eq!(turns[1]["tool"], "fs_write");
    assert_eq!(turns[2]["turn"], 2, "the second turn was written LIVE");
    assert!(turns[2].get("cache_read_tokens").is_none());
    assert!(
        !serde_json::to_string(&emitted)
            .unwrap()
            .contains("hunter22"),
        "the model's own tool arguments never reach a checkpoint"
    );

    let meta = &invocation.session_meta;
    assert_eq!(meta["session_id"], "session-fake-1");
    assert_eq!(meta["num_turns"], 2);
    // Summed across the session, not the last turn's counts: (10+4)+20.
    assert_eq!(meta["input_tokens"], 34);
    assert_eq!(meta["output_tokens"], 5);
    assert_eq!(meta["cache_read_tokens"], 4);
    assert_eq!(meta["harness"], "deepseek");
    assert_eq!(meta["profile"], "headless");

    // The transcript holds the prompt, the tool arguments and the tool
    // results the journal deliberately refuses to carry — which is why
    // it is the operator's, kept under the harness home, and outlives
    // the seat. The checkpoints folded out of it are the journal's part.
    assert!(
        kept[0].path().is_dir(),
        "the seat's transcript root did not survive the seat: {target}"
    );

    // A harness that writes no transcript at all still concludes: the
    // seat is silent, not broken. Named workdir absent too, so the
    // driver falls back to its own directory as every other arm does.
    let quiet = executable(dir.path(), "quiet-dsh", "#!/bin/sh\nexit 0\n");
    std::env::set_var("BROKKR_DSH_BIN", &quiet);
    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke(AdapterKind::Dsh, &[], "p", &json!({}), &[], &mut |event| {
        emitted.push(event.clone())
    })
    .unwrap();
    assert_eq!(emitted.len(), 1, "{emitted:?}");
    assert_eq!(invocation.exit_code, 0);
    assert!(invocation.session_meta.get("num_turns").is_none());

    match prior {
        Some(value) => std::env::set_var("BROKKR_DSH_BIN", value),
        None => std::env::remove_var("BROKKR_DSH_BIN"),
    }
    if let Some(value) = prior_legacy {
        std::env::set_var("FORGE_DSH_BIN", value);
    }
    match prior_home {
        Some(value) => std::env::set_var("DSH_HOME", value),
        None => std::env::remove_var("DSH_HOME"),
    }
}

#[test]
fn dsh_fold_refuses_a_nameless_session_and_journals_no_transcript() {
    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    for event in [
        // A header with no string id names nothing, so nothing is
        // journaled — the same refusal the claude init fold makes.
        json!({"type": "session", "id": 7}),
        json!({"type": "session", "id": "session-1"}),
        // An assistant turn the adapter reported no accounting for.
        json!({"type": "assistant/message", "data": {"turn": 1, "step": 1}}),
        // A tool call whose name the log did not carry.
        json!({"type": "tool/call", "data": {"turn": 1}}),
        json!({"type": "assistant/chunk", "data": {"turn": 1}}),
        json!({"type": "turn/end", "data": {"turn": 1}}),
        json!({}),
    ] {
        fold_dsh_event(&event, &mut turns, &mut meta, &mut |value| {
            emitted.push(value.clone())
        });
    }
    assert_eq!(emitted.len(), 3, "{emitted:?}");
    assert_eq!(emitted[0]["step"], "session-started");
    assert_eq!(meta["session_id"], "session-1");
    assert_eq!(emitted[1]["turn"], 1);
    assert!(emitted[1].get("input_tokens").is_none());
    assert_eq!(emitted[2]["tool"], "");
    assert_eq!(meta["num_turns"], 1);
    assert!(meta.get("input_tokens").is_none());
}

#[cfg(unix)]
#[test]
fn the_transcript_is_found_by_construction_and_never_by_a_directory_scan() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    // No root yet: nothing to follow, and no error either — dsh
    // materializes the root on its first append, not at boot.
    assert!(find_dsh_transcript(&root).is_none());
    std::fs::create_dir_all(&root).unwrap();
    // A plain file where a project directory would be is not one.
    std::fs::write(root.join("stray"), b"x").unwrap();
    assert!(find_dsh_transcript(&root).is_none());
    let session = root.join("--project--").join("session-1");
    std::fs::create_dir_all(&session).unwrap();
    // A session directory that holds other artifacts but no transcript.
    std::fs::write(session.join("notes.txt"), b"x").unwrap();
    assert!(find_dsh_transcript(&root).is_none());

    // dsh creates the file before its first append, and a header still
    // being written is not JSON. Neither names a session yet, so neither
    // is the answer yet — the poll loop asks again.
    let transcript = session.join("session.jsonl");
    std::fs::write(&transcript, b"").unwrap();
    assert!(find_dsh_transcript(&root).is_none());
    std::fs::write(&transcript, b"{\"type\":\"session\",\"id\":\"s").unwrap();
    assert!(find_dsh_transcript(&root).is_none());
    // A line that parses but is not the header names nothing either.
    std::fs::write(&transcript, b"{\"type\":\"turn/start\"}\n").unwrap();
    assert!(find_dsh_transcript(&root).is_none());
    // Nor does a header line that is not text at all: the read itself
    // fails, and a file that cannot be read names no session.
    std::fs::write(&transcript, b"\xff\xfe not utf-8 at all\n").unwrap();
    assert!(find_dsh_transcript(&root).is_none());

    let header = b"{\"type\":\"session\",\"id\":\"s\",\"delegationDepth\":0}\n";
    std::fs::write(&transcript, header).unwrap();
    assert_eq!(find_dsh_transcript(&root).as_deref(), Some(&*transcript));

    // A transcript the seat cannot read is left alone and retried; it is
    // never guessed at from a neighbouring file.
    std::fs::set_permissions(&transcript, std::fs::Permissions::from_mode(0o000)).unwrap();
    let mut tail = DshTail::default();
    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    drain_dsh_transcript(&mut tail, &root, &mut turns, &mut meta, &mut |value| {
        emitted.push(value.clone())
    });
    assert!(tail.file.is_none());
    assert!(emitted.is_empty());

    std::fs::set_permissions(&transcript, std::fs::Permissions::from_mode(0o644)).unwrap();
    std::fs::write(
        &transcript,
        b"{\"type\":\"session\",\"id\":\"s\"}\n\xff not utf-8\nhalf a line",
    )
    .unwrap();
    // A header with no depth at all is the seat's own session: dsh
    // writes `delegationDepth` from the root session onward, and an
    // absent one has never meant "delegated".
    drain_dsh_transcript(&mut tail, &root, &mut turns, &mut meta, &mut |value| {
        emitted.push(value.clone())
    });
    assert!(tail.file.is_some());
    assert_eq!(emitted.len(), 1, "{emitted:?}");
    assert_eq!(tail.pending, b"half a line");
}

#[test]
fn a_delegated_sub_session_never_becomes_the_one_the_seat_reports() {
    // One root is not the same claim as one session: dsh's header
    // carries a `delegationDepth`, so a session the seat delegates
    // writes a SECOND transcript under the same root. `read_dir` yields
    // entries in whatever order the filesystem likes, so the seat's own
    // session is picked by what its header SAYS, not by which entry came
    // back first. Both orders are written here and both must answer the
    // same way.
    for (seat, delegated) in [("a-seat", "b-delegated"), ("z-seat", "a-delegated")] {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("root").join("--project--");
        let mut wanted = std::path::PathBuf::new();
        for (name, depth) in [(seat, 0), (delegated, 1)] {
            let transcript = project.join(name).join("session.jsonl");
            std::fs::create_dir_all(transcript.parent().unwrap()).unwrap();
            std::fs::write(
                &transcript,
                format!("{{\"type\":\"session\",\"id\":\"{name}\",\"delegationDepth\":{depth}}}\n"),
            )
            .unwrap();
            if depth == 0 {
                wanted = transcript;
            }
        }
        assert_eq!(
            find_dsh_transcript(&dir.path().join("root")).as_deref(),
            Some(&*wanted),
            "the seat's own session is {seat}, not {delegated}"
        );
    }
}

#[test]
fn the_seat_overlay_reports_a_file_it_cannot_stage_or_write() {
    let root = std::path::Path::new("/nonexistent/dsh-root");
    let refused =
        dsh_seat_overlay_in(None, root, || Err(std::io::Error::other("no tmp"))).unwrap_err();
    assert!(
        refused.contains("could not stage the dsh seat overlay"),
        "{refused}"
    );
    let sealed = dsh_seat_overlay_in(None, root, || {
        let staged = tempfile::NamedTempFile::new()?;
        let (_, path) = staged.into_parts();
        let readonly = std::fs::File::open(&path)?;
        Ok(tempfile::NamedTempFile::from_parts(readonly, path))
    })
    .unwrap_err();
    assert!(
        sealed.contains("could not write the dsh seat overlay"),
        "{sealed}"
    );
    // YAML is the overlay's grammar for the root as much as the model:
    // a path that could open a line of its own is refused, not written.
    for bad in ["/tmp/a\nb", "/tmp/a\rb"] {
        assert!(
            dsh_transcript_row(std::path::Path::new(bad)).is_err(),
            "{bad:?} must be refused"
        );
    }
    // A quote in the path is doubled inside the single-quoted scalar,
    // so it closes nothing.
    let row = dsh_transcript_row(std::path::Path::new("/tmp/it's")).unwrap();
    assert!(row.contains("root: '/tmp/it''s'\n"), "{row}");
    assert!(row.contains("compression: none\n"), "{row}");
    assert!(row.contains("packChunks: false\n"), "{row}");
}

#[test]
fn the_transcript_root_reports_a_directory_it_cannot_stage() {
    let refused = dsh_transcript_root_in(|| Err(std::io::Error::other("no tmp"))).unwrap_err();
    assert!(
        refused.contains("could not stage the dsh session transcript root"),
        "{refused}"
    );
}

#[test]
fn the_transcript_root_is_kept_under_the_harness_home_and_survives_the_seat() {
    let home = tempfile::tempdir().unwrap();
    let root = dsh_transcript_root_under(Some(home.path().to_path_buf())).unwrap();
    assert!(
        root.starts_with(home.path().join("sessions").join("brokkr")),
        "{root:?}"
    );
    assert!(root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("seat-"));
    // The creating handle is gone; the directory is not.
    assert!(root.is_dir(), "{root:?}");
    let other = dsh_transcript_root_under(Some(home.path().to_path_buf())).unwrap();
    assert_ne!(root, other, "one root per seat");

    let refused = dsh_transcript_root_under(None).unwrap_err();
    assert!(
        refused.to_string().contains("set DSH_HOME or HOME"),
        "{refused}"
    );
    // A file where the base must be a directory is the staging failure.
    let blocked = tempfile::tempdir().unwrap();
    std::fs::write(blocked.path().join("sessions"), b"not a directory").unwrap();
    assert!(dsh_transcript_root_under(Some(blocked.path().to_path_buf())).is_err());
}

#[test]
fn the_dsh_home_is_dsh_home_when_set_else_dot_dsh_under_home() {
    use std::ffi::OsString;
    assert_eq!(
        dsh_home_from(
            Some(OsString::from("/opt/dsh")),
            Some(OsString::from("/home/x"))
        ),
        Some(std::path::PathBuf::from("/opt/dsh"))
    );
    assert_eq!(
        dsh_home_from(Some(OsString::new()), Some(OsString::from("/home/x"))),
        Some(std::path::PathBuf::from("/home/x/.dsh"))
    );
    assert_eq!(
        dsh_home_from(None, Some(OsString::from("/home/x"))),
        Some(std::path::PathBuf::from("/home/x/.dsh"))
    );
    assert_eq!(dsh_home_from(None, None), None);
    assert!(dsh_home().is_some(), "a test process has a home");
}

#[test]
fn codex_journals_its_turn_count_and_sums_usage_without_a_result_event() {
    // Verified against codex-cli 0.148.0: a real `codex exec --json`
    // run ends at `turn.completed` and emits no `result` event, so the
    // turn count has to be journaled there or a codex seat reaches the
    // cost surfaces with no turns at all. Usage sums across turns —
    // inserting per turn left the session holding its last turn's
    // counts.
    let mut turn = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    for event in [
        json!({"type": "thread.started", "thread_id": "thread-1"}),
        json!({"type": "turn.started"}),
        json!({"type": "turn.completed", "usage": {
            "input_tokens": 10, "cached_input_tokens": 4, "output_tokens": 2
        }}),
        json!({"type": "turn.started"}),
        json!({"type": "turn.completed", "usage": {
            "input_tokens": 7, "cached_input_tokens": 1, "output_tokens": 3
        }}),
    ] {
        fold_codex_event(&event, &mut turn, &mut meta, &mut |value| {
            emitted.push(value.clone())
        });
    }
    assert_eq!(meta["num_turns"], 2);
    assert_eq!(meta["input_tokens"], 17);
    assert_eq!(meta["cache_read_tokens"], 5);
    assert_eq!(meta["output_tokens"], 3 + 2);
    // Codex reports no cost in USD anywhere in that stream, so none is
    // invented: the record says nothing rather than claiming zero.
    assert!(meta.get("total_cost_usd").is_none());
    // The per-turn checkpoint keeps that turn's own counts.
    let last = emitted.last().unwrap();
    assert_eq!(last["step"], "turn-completed");
    assert_eq!(last["input_tokens"], 7);
}

#[test]
fn dsh_model_overlay_reports_a_file_it_cannot_stage_or_write() {
    let refused =
        dsh_model_overlay_in("deepseek-v4-flash", || Err(std::io::Error::other("no tmp")))
            .unwrap_err();
    assert!(
        refused.contains("could not stage the dsh model overlay"),
        "{refused}"
    );

    // A file that exists but takes no bytes: the same path reopened
    // read-only, handed over as the staged file.
    let sealed = dsh_model_overlay_in("deepseek-v4-flash", || {
        let staged = tempfile::NamedTempFile::new()?;
        let (_, path) = staged.into_parts();
        let readonly = std::fs::File::open(&path)?;
        Ok(tempfile::NamedTempFile::from_parts(readonly, path))
    })
    .unwrap_err();
    assert!(
        sealed.contains("could not write the dsh model overlay"),
        "{sealed}"
    );
}

/// A codex whose thread opens long before its first turn does, and which
/// refuses to finish until the driver has already said so. Exit 9 is its
/// verdict that the thread id reached the journal only at the end.
const CODEX_THREAD_SHIM: &str = r#"#!/bin/sh
cat >/dev/null
printf '{"type":"thread.started","thread_id":"01a0619c-928b-7ad3-8cc9-9eaa94c3aec1"}\n'
i=0
while [ ! -f codex-seen ] && [ $i -lt 400 ]; do sleep 0.05; i=$((i+1)); done
[ -f codex-seen ] || exit 9
printf '{"type":"turn.started"}\n'
printf '{"type":"item.completed","item":{"id":"item_0","type":"agent_message"}}\n'
printf '{"type":"turn.completed","usage":{"input_tokens":14620,"cached_input_tokens":11264,"output_tokens":5}}\n'
"#;

#[cfg(unix)]
#[test]
fn a_running_codex_seat_journals_its_thread_id_before_its_first_turn() {
    // The shape of the shim's stream is the installed binary's own,
    // captured from one real `codex exec --json` run (codex-cli
    // 0.148.0): `thread.started` first, `turn.completed` last, no
    // `result` event at all. Before this the id lived in session_meta
    // only, which the journal sees once — inside the finishing
    // checkpoint — so `brokkr inspect --seat` on a WORKING codex seat
    // showed no session id and the drilldown had nothing to open.
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = executable(dir.path(), "codex", CODEX_THREAD_SHIM);
    let prior = std::env::var_os("BROKKR_CODEX_BIN");
    let prior_legacy = std::env::var_os("FORGE_CODEX_BIN");
    std::env::set_var("BROKKR_CODEX_BIN", &fake);
    std::env::remove_var("FORGE_CODEX_BIN");

    let seen = dir.path().join("codex-seen");
    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke(
        AdapterKind::Codex,
        &[],
        "the prompt",
        &json!({"workdir": dir.path()}),
        &[],
        &mut |event| {
            emitted.push(event.clone());
            if event["step"] == "session-started" {
                let _ = std::fs::write(&seen, b"");
            }
        },
    )
    .unwrap();

    match prior {
        Some(value) => std::env::set_var("BROKKR_CODEX_BIN", value),
        None => std::env::remove_var("BROKKR_CODEX_BIN"),
    }
    if let Some(value) = prior_legacy {
        std::env::set_var("FORGE_CODEX_BIN", value);
    }

    let thread = "01a0619c-928b-7ad3-8cc9-9eaa94c3aec1";
    assert_eq!(invocation.exit_code, 0, "{emitted:?}");
    assert_eq!(emitted[0]["step"], "session-started");
    assert_eq!(emitted[0]["harness"], "codex");
    assert_eq!(emitted[0]["session_id"], thread);
    assert_eq!(emitted.last().unwrap()["step"], "turn-completed");
    let meta = &invocation.session_meta;
    assert_eq!(meta["session_id"], thread);
    assert_eq!(meta["num_turns"], 1);
    // codex's own `input_tokens` already contains the cache read, so
    // nothing is added back to it — the opposite of the dsh fold, and
    // the reason both drivers agree on what the journal key means.
    assert_eq!(meta["input_tokens"], 14620);
    assert_eq!(meta["cache_read_tokens"], 11264);
}

#[test]
fn a_thread_id_too_long_for_the_journal_is_clamped_in_both_places() {
    let mut turn = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    fold_codex_event(
        &json!({"type": "thread.started", "thread_id": "x".repeat(200)}),
        &mut turn,
        &mut meta,
        &mut |value| emitted.push(value.clone()),
    );
    assert_eq!(meta["session_id"].as_str().unwrap().chars().count(), 128);
    assert_eq!(emitted[0]["session_id"], meta["session_id"]);
}

#[test]
fn a_dsh_tool_call_before_any_assembled_message_invents_no_turn() {
    // dsh 0.1.0-rc.6 always writes the `assistant/message` that asked
    // for a tool before the `tool/call` itself, so this is the shape of
    // a log that got cut short or reordered. The fold reports turn 0
    // rather than claiming a turn dsh never assembled; the turn cell is
    // a maximum over the seat's checkpoints, so a zero never lowers it.
    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    fold_dsh_event(
        &json!({"type": "tool/call", "data": {"name": "fs_read"}}),
        &mut turns,
        &mut meta,
        &mut |value| emitted.push(value.clone()),
    );
    assert_eq!(emitted[0]["turn"], 0);
    assert_eq!(emitted[0]["tool"], "fs_read");
    assert!(meta.get("num_turns").is_none());
}

#[cfg(unix)]
#[test]
fn a_seat_whose_child_cannot_be_waited_on_concludes_instead_of_spinning() {
    // The failure the poll loop must never absorb: a `wait` that keeps
    // erroring is not "still running". Folded into the not-yet arm it
    // would spin at DSH_POLL_IDLE forever — no result, no error, no
    // exit — which is exactly the invisible seat the per-turn
    // checkpoints exist to end, reached by a longer road. A real
    // `waitpid` cannot be made to fail from here, so the question is
    // injected the way this driver's other syscalls already are.
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let fake = executable(dir.path(), "dsh", "#!/bin/sh\nexit 0\n");
    let prior = std::env::var_os("BROKKR_DSH_BIN");
    let prior_legacy = std::env::var_os("FORGE_DSH_BIN");
    let prior_home = std::env::var_os("DSH_HOME");
    std::env::set_var("BROKKR_DSH_BIN", &fake);
    std::env::remove_var("FORGE_DSH_BIN");
    // The seat's transcript is kept under the harness home; in a test
    // that home is this test's own directory, never the operator's.
    std::env::set_var("DSH_HOME", dir.path());

    let mut passes = 0;
    let mut emitted: Vec<Value> = Vec::new();
    let refused = invoke_dsh_with(
        &[],
        "the prompt",
        dir.path().to_str().unwrap(),
        &mut |event| emitted.push(event.clone()),
        |child| {
            passes += 1;
            if passes == 1 {
                return Ok(None);
            }
            // The child is reaped here rather than left behind: the
            // refusal is about what the driver does when it cannot ask,
            // not about leaking the process it was asking about.
            let _ = child.wait();
            Err(std::io::Error::other("no child processes"))
        },
    );

    match prior {
        Some(value) => std::env::set_var("BROKKR_DSH_BIN", value),
        None => std::env::remove_var("BROKKR_DSH_BIN"),
    }
    if let Some(value) = prior_legacy {
        std::env::set_var("FORGE_DSH_BIN", value);
    }
    match prior_home {
        Some(value) => std::env::set_var("DSH_HOME", value),
        None => std::env::remove_var("DSH_HOME"),
    }

    let refused = match refused {
        Ok(_) => panic!("a child that cannot be waited on is not a success"),
        Err(refused) => refused,
    };
    assert!(refused.contains("agent CLI did not conclude"), "{refused}");
    assert!(refused.contains("no child processes"), "{refused}");
    // The seat still said it had started: what it cannot do is go quiet
    // and never come back.
    assert_eq!(emitted[0]["step"], "harness-started");
}
