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
        None,
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
        None,
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
        None,
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
        None,
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
        assert!(invoke(kind, &[], "prompt", &json!({}), None, &[], &mut |_| {}).is_ok());

        std::env::set_var(variable, &closed);
        let prompt = "x".repeat(1_000_000);
        let error = match invoke(kind, &[], &prompt, &json!({}), None, &[], &mut |_| {}) {
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
    run_seat(AdapterKind::Lanetally, &[], &start, None, &mut |b| {
        bodies.push(b)
    });
    run_seat(AdapterKind::Claude, &[], &start, None, &mut |b| {
        bodies.push(b)
    });
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
    std::env::set_var("BROKKR_DSH_BIN", &fake);
    std::env::remove_var("FORGE_DSH_BIN");

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
        None,
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

    // No pair, no overlay: the profile's own default model boots, and
    // the journal says so by naming none.
    let mut started = Vec::new();
    let invocation = invoke(
        AdapterKind::Dsh,
        &[],
        "p",
        &json!({"workdir": dir.path()}),
        None,
        &[],
        &mut |event| started.push(event.clone()),
    )
    .unwrap();
    let lines: Vec<String> = std::fs::read_to_string(&argv)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(lines, ["--profile", "headless", "p"]);
    assert!(started[0]["model"].is_null());
    assert!(invocation.session_meta.get("model").is_none());

    match prior {
        Some(value) => std::env::set_var("BROKKR_DSH_BIN", value),
        None => std::env::remove_var("BROKKR_DSH_BIN"),
    }
    if let Some(value) = prior_legacy {
        std::env::set_var("FORGE_DSH_BIN", value);
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
        assert!(dsh_model_overlay(bad).is_err(), "{bad:?} must be refused");
    }
    assert!(dsh_model_overlay("deepseek-v4-flash").is_ok());
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
    let file = dsh_model_overlay("dashscope/qwen3.8-max").unwrap();
    let written = std::fs::read_to_string(file.path()).unwrap();
    assert!(written.contains("provider: dashscope\n"), "{written}");
    assert!(written.contains("model: qwen3.8-max\n"), "{written}");
    assert!(!written.contains("deepseek-official"), "{written}");
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

/// A codex thread id in the shape codex writes it, used wherever a test
/// needs a plausible one.
const THREAD: &str = "01a06183-5173-7aa2-8fd6-c2f4923a93a1";

/// A stand-in codex that records the argv it was given and the prompt it
/// was fed, then answers with the two events the fold reads.
#[cfg(unix)]
fn codex_shim(dir: &std::path::Path, name: &str, argv: &std::path::Path) -> std::path::PathBuf {
    let argv = argv.display();
    executable(
        dir,
        name,
        &format!(
            "#!/bin/sh\nfor a in \"$@\"; do printf '%s\\n' \"$a\" >> {argv}; done\n\
             cat >> {argv}.stdin\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{THREAD}\"}}\\n'\n\
             printf '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":100,\
             \"cached_input_tokens\":96,\"output_tokens\":4}}}}\\n'\n"
        ),
    )
}

#[cfg(unix)]
fn recorded(argv: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(argv)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// Pin the codex binary for one test and put back whatever was there.
#[cfg(unix)]
fn with_codex_bin<T>(shim: &std::path::Path, body: impl FnOnce() -> T) -> T {
    let prior = std::env::var_os("BROKKR_CODEX_BIN");
    let prior_legacy = std::env::var_os("FORGE_CODEX_BIN");
    std::env::set_var("BROKKR_CODEX_BIN", shim);
    std::env::remove_var("FORGE_CODEX_BIN");
    let outcome = body();
    match prior {
        Some(value) => std::env::set_var("BROKKR_CODEX_BIN", value),
        None => std::env::remove_var("BROKKR_CODEX_BIN"),
    }
    if let Some(value) = prior_legacy {
        std::env::set_var("FORGE_CODEX_BIN", value);
    }
    outcome
}

/// The resume argv, whole: the subcommand, the seat's own sandbox class
/// re-expressed as the config override `codex exec resume` accepts (it
/// takes neither `-C` nor `-s`, verified against codex-cli 0.148.0), the
/// rest of the seat's passthrough in order, the thread positionally, and
/// `-` for the prompt — the only spelling that makes a resume read the
/// prompt this driver writes to its stdin.
#[cfg(unix)]
#[test]
fn a_codex_resume_carries_the_thread_the_class_and_the_prompt() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let argv = dir.path().join("argv");
    let shim = codex_shim(dir.path(), "codex", &argv);
    let extra: Vec<String> = ["--sandbox", "read-only", "--model", "gpt-5.6-sol"]
        .iter()
        .map(|part| part.to_string())
        .collect();
    let mut emitted = Vec::new();
    let invocation = with_codex_bin(&shim, || {
        invoke(
            AdapterKind::Codex,
            &extra,
            "the prompt",
            &json!({"workdir": dir.path()}),
            Some(THREAD),
            &[],
            &mut |event| emitted.push(event.clone()),
        )
        .unwrap()
    });

    assert_eq!(
        recorded(&argv),
        [
            "exec",
            "resume",
            "--json",
            "-c",
            "sandbox_mode=\"read-only\"",
            "--model",
            "gpt-5.6-sol",
            THREAD,
            "-",
        ]
    );
    assert_eq!(
        std::fs::read_to_string(format!("{}.stdin", argv.display())).unwrap(),
        "the prompt"
    );
    assert_eq!(
        emitted[0],
        json!({"step":"harness-started", "harness":"codex", "launch":"resumed",
               "session_id": THREAD, "sandbox":"read-only"}),
        "the launch says which thread it rejoined under which class"
    );
    // The fold still reads the thread out of the resumed stream, so the
    // NEXT attempt of this seat has an id to be offered in its turn.
    assert_eq!(invocation.session_meta["session_id"], THREAD);
    assert_eq!(invocation.session_meta["cache_read_tokens"], 96);
    assert_eq!(invocation.exit_code, 0);
}

/// The sandbox travels or the resume does not (decision 0030 ruling 2).
/// Every way a class can fail to travel ends in the same place: the cold
/// argv, unchanged from what it has always been, and a checkpoint saying
/// why the offer could not be taken.
#[cfg(unix)]
#[test]
fn a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cases: [(&str, Vec<&str>, &str); 5] = [
        // Nothing declared: a codex resume does not inherit the class
        // its thread was opened under, so there is nothing to re-impose.
        (
            "undeclared",
            vec!["--model", "sol"],
            "declares no sandbox class",
        ),
        // A flag with nothing after it declares nothing either.
        ("dangling", vec!["--sandbox"], "declares no sandbox class"),
        // A class this driver cannot spell as a config override.
        (
            "invented",
            vec!["--sandbox", "invented"],
            "is not one codex exec resume",
        ),
        // A second expression that could outrank the re-imposed one.
        (
            "doubled",
            vec![
                "--sandbox",
                "read-only",
                "-c",
                "sandbox_mode=\"danger-full-access\"",
            ],
            "second sandbox expression",
        ),
        // An id that is not a plain thread id never reaches an argv.
        (
            "forged",
            vec!["--sandbox", "read-only"],
            "not a plain thread id",
        ),
    ];
    for (case, extra, reason) in cases {
        let argv = dir.path().join(format!("argv-{case}"));
        let shim = codex_shim(dir.path(), &format!("codex-{case}"), &argv);
        let extra: Vec<String> = extra.iter().map(|part| part.to_string()).collect();
        let session = if case == "forged" {
            "not a thread"
        } else {
            THREAD
        };
        let mut emitted = Vec::new();
        with_codex_bin(&shim, || {
            invoke(
                AdapterKind::Codex,
                &extra,
                "prompt",
                &json!({"workdir": dir.path()}),
                Some(session),
                &[],
                &mut |event| emitted.push(event.clone()),
            )
            .unwrap()
        });
        let mut cold = vec![
            "exec".to_string(),
            "--json".into(),
            "-C".into(),
            dir.path().to_string_lossy().into_owned(),
        ];
        cold.extend(extra.iter().cloned());
        assert_eq!(recorded(&argv), cold, "{case}: the cold argv, unchanged");
        assert_eq!(emitted[0]["launch"], "cold", "{case}: {}", emitted[0]);
        assert!(
            emitted[0]["reason"].as_str().unwrap().contains(reason),
            "{case}: {}",
            emitted[0]
        );
        assert!(
            emitted[0].get("session_id").is_none(),
            "{case}: a cold launch rejoined nothing: {}",
            emitted[0]
        );
    }
}

/// A resume codex refuses — an unknown or expired thread — is a cold
/// spawn with the refusal journaled, never the attempt's failure
/// (decision 0030 ruling 3). The predicate is structural: a non-zero
/// exit with no thread ever announced.
#[cfg(unix)]
#[test]
fn a_refused_resume_is_a_cold_spawn_with_the_refusal_journaled() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let argv = dir.path().join("argv");
    let shim = executable(
        dir.path(),
        "codex-refusing",
        &format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' \"$*\" >> {argv}\n\
             case \"$*\" in\n\
             *resume*) printf 'Error: no rollout found for thread id\\n' >&2; exit 1 ;;\n\
             esac\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{THREAD}\"}}\\n'\n",
            argv = argv.display()
        ),
    );
    let extra = vec!["--sandbox".to_string(), "read-only".into()];
    let mut emitted = Vec::new();
    let invocation = with_codex_bin(&shim, || {
        invoke(
            AdapterKind::Codex,
            &extra,
            "prompt",
            &json!({"workdir": dir.path()}),
            Some(THREAD),
            &[],
            &mut |event| emitted.push(event.clone()),
        )
        .unwrap()
    });

    let attempts = recorded(&argv);
    assert_eq!(attempts.len(), 2, "the refusal is followed by a cold spawn");
    assert!(attempts[0].contains("resume"), "{attempts:?}");
    assert!(!attempts[1].contains("resume"), "{attempts:?}");
    assert!(attempts[1].contains("--sandbox read-only"), "{attempts:?}");
    assert_eq!(emitted[0]["launch"], "resumed");
    assert_eq!(
        emitted[1],
        json!({"step":"harness-started", "harness":"codex", "launch":"cold",
               "reason":"codex refused the offered thread"})
    );
    // What the seat gets is the COLD session, not the refusal: exit 0,
    // and the thread the cold spawn opened.
    assert_eq!(invocation.exit_code, 0);
    assert_eq!(invocation.session_meta["session_id"], THREAD);
    assert!(invocation.stderr.is_empty(), "{}", invocation.stderr);
}

/// A resumed session that starts and THEN fails is an ordinary attempt
/// failure: the seat's work began inside it, and re-running it cold
/// would be a second billed session for one attempt. Only a refusal —
/// nothing started — falls back.
#[cfg(unix)]
#[test]
fn a_resume_that_started_and_failed_is_not_respawned_cold() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let argv = dir.path().join("argv");
    let shim = executable(
        dir.path(),
        "codex-failing",
        &format!(
            "#!/bin/sh\ncat >/dev/null\nprintf '%s\\n' \"$*\" >> {argv}\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{THREAD}\"}}\\n'\n\
             exit 3\n",
            argv = argv.display()
        ),
    );
    let extra = vec!["--sandbox".to_string(), "workspace-write".into()];
    let mut emitted = Vec::new();
    let invocation = with_codex_bin(&shim, || {
        invoke(
            AdapterKind::Codex,
            &extra,
            "prompt",
            &json!({"workdir": dir.path()}),
            Some(THREAD),
            &[],
            &mut |event| emitted.push(event.clone()),
        )
        .unwrap()
    });
    assert_eq!(recorded(&argv).len(), 1, "one session, one attempt");
    assert_eq!(invocation.exit_code, 3);
    assert_eq!(emitted[0]["sandbox"], "workspace-write");
    assert!(
        !emitted.iter().any(|event| event["launch"] == "cold"),
        "{emitted:?}"
    );
}

/// The `--sandbox=<class>` spelling is the same declaration as the
/// separated pair, and a class declared twice is the one declared last —
/// the reading `codex exec` itself gives them.
#[test]
fn the_sandbox_declaration_is_read_in_both_of_its_spellings() {
    let split = |parts: &[&str]| {
        let extra: Vec<String> = parts.iter().map(|part| part.to_string()).collect();
        split_codex_sandbox(&extra)
    };
    assert_eq!(
        split(&["--sandbox=read-only", "--model", "sol"]),
        (
            Some("read-only".into()),
            vec!["--model".into(), "sol".into()]
        )
    );
    assert_eq!(
        split(&["-s", "workspace-write"]),
        (Some("workspace-write".into()), Vec::new())
    );
    assert_eq!(
        split(&["--sandbox", "read-only", "--sandbox", "danger-full-access"]),
        (Some("danger-full-access".into()), Vec::new())
    );
    assert_eq!(
        split(&["--model", "sol"]),
        (None, vec!["--model".into(), "sol".into()])
    );

    // A session handle reaches an argv, so it stays a plain identifier.
    assert!(plain_thread_id(THREAD));
    assert!(!plain_thread_id(""));
    assert!(!plain_thread_id(&"a".repeat(129)));
    assert!(!plain_thread_id("thread id"));
    assert!(!plain_thread_id("thread;rm"));
    // A positional argument that could be read as a flag is not an id.
    assert!(!plain_thread_id("--last"));
}

/// The offer reaches the seat through the protocol's own vocabulary: a
/// `resume` arrives ahead of the `start` it belongs to, is spent by that
/// one seat, and a `start` with nothing in front of it is a cold start.
#[cfg(unix)]
#[test]
fn the_resume_message_hands_one_session_to_the_next_seat_and_no_other() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let argv = dir.path().join("argv");
    let shim = codex_shim(dir.path(), "codex", &argv);
    let result = dir.path().join("result.json");
    std::fs::write(&result, "{\"result\":\"complete\"}").unwrap();
    let start = |id: &str| {
        serde_json::to_string(&json!({
            "proto":"forge-driver/v1", "msg_id":id, "type":"start",
            "effect_id":"fx", "attempt_id":id, "seat":"work",
            "input": {"workdir": dir.path(), "result_path": result,
                      "allowed_results":["complete"], "feature":"f", "phase":"work"},
        }))
        .unwrap()
    };
    let hello = serde_json::to_string(&Message::new(Body::Hello {
        engine_version: "test".into(),
    }))
    .unwrap();
    let resume = serde_json::to_string(&Message::new(Body::Resume {
        effect_id: "fx".into(),
        attempt_id: "a1".into(),
        session_ref: THREAD.into(),
    }))
    .unwrap();
    let extra = vec!["--sandbox".to_string(), "read-only".into()];
    let input = format!("{hello}\n{resume}\n{}\n{}\n", start("a1"), start("a2"));
    let mut output = Vec::new();
    with_codex_bin(&shim, || {
        serve_io(AdapterKind::Codex, &extra, input.as_bytes(), &mut output).unwrap()
    });
    let messages: Vec<Value> = String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    // Codex declares what it can honour; the others still declare none.
    assert_eq!(messages[0]["supports"], json!(["resume"]));
    assert_eq!(AdapterKind::Claude.supports(), Vec::<String>::new());
    assert_eq!(AdapterKind::Dsh.supports(), Vec::<String>::new());

    let launches: Vec<&Value> = messages
        .iter()
        .filter(|message| message["data"]["step"] == "harness-started")
        .collect();
    assert_eq!(launches.len(), 2, "one launch per seat");
    assert_eq!(launches[0]["data"]["launch"], "resumed");
    assert_eq!(launches[0]["data"]["session_id"], THREAD);
    assert_eq!(
        launches[1]["data"]["launch"], "cold",
        "the offer was spent by the first seat: {}",
        launches[1]
    );
    assert!(
        launches[1]["data"].get("reason").is_none(),
        "nobody offered the second seat anything to refuse: {}",
        launches[1]
    );
}
