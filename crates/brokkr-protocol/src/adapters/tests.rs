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
