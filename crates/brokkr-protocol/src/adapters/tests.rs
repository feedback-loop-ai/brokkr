use super::*;
use crate::transcript::{dsh_home, dsh_home_from};
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
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    let event = json!({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "name": "Bash",
         "input": {"command": "curl -H 'auth: hunter22' https://x"}},
        {"type": "tool_use", "name": "Edit",
         "input": {"file_path": "src/lib.rs"}},
    ]}});
    fold_stream_event(&event, &mut turns, &mut meta, &mut transcript, &mut |c| {
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
    let mut claude_transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    for event in [
        json!({"type": "system", "subtype": "init", "session_id": "session"}),
        json!({"type": "system", "subtype": "other"}),
        json!({"type": "system", "subtype": "init"}),
        // A shim that names its effort only on `result` is read there
        // too, on the same terms as the model beside it.
        json!({"type": "result", "model": "claude-served", "effort": "xhigh",
               "num_turns": 2, "total_cost_usd": 1.5}),
        json!({"type": "result"}),
        json!({"type": "ignored"}),
    ] {
        fold_stream_event(
            &event,
            &mut turns,
            &mut meta,
            &mut claude_transcript,
            &mut |value| emitted.push(value.clone()),
        );
    }
    assert_eq!(meta["transcript"]["locator"], "session");
    assert_eq!(meta["num_turns"], 2);
    assert_eq!(meta["model"], "claude-served");
    assert_eq!(meta["effort"], "xhigh");

    let mut codex_transcript = Transcript::resolve(TranscriptKind::CodexThread).unwrap();
    let mut echo = CodexThreadEcho::default();
    for event in [
        json!({"type": "thread.started", "thread_id": "thread"}),
        // An empty thread id locates nothing rather than walking the
        // whole harness home looking for a file named after nothing.
        json!({"type": "thread.started", "thread_id": ""}),
        json!({"type": "thread.started"}),
        json!({"type": "turn.started"}),
        json!({"type": "item.started", "item": {"type": "command"}}),
        json!({"type": "item.completed", "item": {}}),
        json!({"type": "turn.completed", "usage": {
            "input_tokens": 3, "cached_input_tokens": 2, "output_tokens": 1
        }}),
        json!({"type": "turn.completed"}),
        json!({"type": "result", "model": "codex-served", "session_id": "final",
               "turn_context": {"effort": "medium"},
               "num_turns": 1, "total_cost_usd": 0.25}),
        json!({"type": "result"}),
        json!({"type": "ignored"}),
    ] {
        fold_codex_event(
            &event,
            &mut turns,
            &mut meta,
            &mut codex_transcript,
            &mut echo,
            &mut |value| emitted.push(value.clone()),
        );
    }
    assert_eq!(meta["transcript"]["locator"], "final");
    assert_eq!(meta["model"], "codex-served");
    assert_eq!(meta["effort"], "medium");
    assert_eq!(meta["cache_read_tokens"], 2);
    // A shim reporting money on `result` is the only codex path that
    // carries cost at all; a zero or absent report stays absent.
    assert_eq!(meta["total_cost_usd"], 0.25);
    assert!(emitted.iter().any(|event| event["step"] == "item-started"));
    assert!(emitted.iter().any(|event| event["tool"] == "unknown"));
}

#[test]
fn served_model_evidence_is_strict_and_dsh_reads_nested_usage_chunks() {
    assert_eq!(
        model_token(" claude-fable-5-1 ").as_deref(),
        Some("claude-fable-5-1")
    );
    assert_eq!(
        model_token("deepseek/deepseek-v4-flash").as_deref(),
        Some("deepseek/deepseek-v4-flash")
    );
    assert_eq!(model_token(""), None);
    assert_eq!(model_token(&"x".repeat(81)), None);
    assert_eq!(model_token("model with spaces"), None);

    assert_eq!(
        model_in_json(&json!({"model":"gpt-5.6-sol"})).as_deref(),
        Some("gpt-5.6-sol")
    );
    assert_eq!(
        model_in_json(&json!({"data":{"message":{"source":{"model":"served-by-dsh"}}}})).as_deref(),
        Some("served-by-dsh")
    );
    assert_eq!(model_in_json(&json!({"model":7})), None);
    assert_eq!(
        model_in_header("noise\nmodel: first\n model: second ").as_deref(),
        Some("second")
    );
    assert_eq!(model_in_header("models: guess\nmodel: not valid"), None);
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
            "read -r value; printf %s \"$value\"".into(),
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

    std::fs::write(&result, "7").unwrap();
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
        } if value == &json!(7)
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
    let prior_claude = std::env::var_os("BROKKR_CLAUDE_BIN");
    let prior_codex = std::env::var_os("FORGE_CODEX_BIN");
    // Codex is pinned here through its OLD spelling, which the new one
    // outranks (decision 0019): an inherited BROKKR_CODEX_BIN would
    // send this test at a real codex, so it goes for the duration.
    let prior_brokkr_codex = std::env::var_os("BROKKR_CODEX_BIN");
    std::env::remove_var("BROKKR_CODEX_BIN");

    for (kind, variable) in [
        (AdapterKind::Claude, "BROKKR_CLAUDE_BIN"),
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
        Some(value) => std::env::set_var("BROKKR_CLAUDE_BIN", value),
        None => std::env::remove_var("BROKKR_CLAUDE_BIN"),
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
fn claude_and_lanetally_accept_their_one_release_legacy_overrides() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "legacy-override",
        "#!/bin/sh\ncat >/dev/null\nprintf '{\"type\":\"result\"}\\n'\n",
    );

    for (kind, primary, legacy) in [
        (AdapterKind::Claude, "BROKKR_CLAUDE_BIN", "FORGE_CLAUDE_BIN"),
        (
            AdapterKind::Lanetally,
            "BROKKR_LANETALLY_BIN",
            "FORGE_LANETALLY_BIN",
        ),
    ] {
        let prior_primary = std::env::var_os(primary);
        let prior_legacy = std::env::var_os(legacy);
        std::env::remove_var(primary);
        std::env::set_var(legacy, &shim);

        let invocation = invoke(kind, &[], "prompt", &json!({}), None, &[], &mut |_| {})
            .expect("the legacy override must select the shim");
        assert_eq!(invocation.exit_code, 0);

        match prior_primary {
            Some(value) => std::env::set_var(primary, value),
            None => std::env::remove_var(primary),
        }
        match prior_legacy {
            Some(value) => std::env::set_var(legacy, value),
            None => std::env::remove_var(legacy),
        }
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
    let prior_lanetally = std::env::var_os("BROKKR_LANETALLY_BIN");
    let prior_claude = std::env::var_os("BROKKR_CLAUDE_BIN");
    std::env::set_var("BROKKR_LANETALLY_BIN", &shim);
    std::env::set_var("BROKKR_CLAUDE_BIN", &shim);
    let mut bodies = Vec::new();
    run_seat(AdapterKind::Lanetally, &[], &start, None, &mut |b| {
        bodies.push(b)
    });
    run_seat(AdapterKind::Claude, &[], &start, None, &mut |b| {
        bodies.push(b)
    });
    match prior_lanetally {
        Some(value) => std::env::set_var("BROKKR_LANETALLY_BIN", value),
        None => std::env::remove_var("BROKKR_LANETALLY_BIN"),
    }
    match prior_claude {
        Some(value) => std::env::set_var("BROKKR_CLAUDE_BIN", value),
        None => std::env::remove_var("BROKKR_CLAUDE_BIN"),
    }
    let finished: Vec<&Value> = bodies
        .iter()
        .filter_map(|body| match body {
            Body::Checkpoint { data, .. }
                if data
                    .get("step")
                    .and_then(Value::as_str)
                    .is_some_and(|step| step.ends_with("-session-finished")) =>
            {
                Some(data)
            }
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
fn init_journals_the_shared_transcript_checkpoint_with_the_id() {
    // The id used to be stashed for the session-finished checkpoint
    // only, which meant a WORKING seat had no session id in the
    // journal — so the transcript drilldowns could not locate, let
    // alone live-stream, the prose being written. init carries the id
    // in the first stream message; it is journaled immediately.
    let mut turns = 0;
    let mut meta = serde_json::Map::new();
    let mut emitted: Vec<serde_json::Value> = Vec::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();
    fold_stream_event(
        &serde_json::json!({"type": "system", "subtype": "init",
                            "session_id": "abcd-1234-ef"}),
        &mut turns,
        &mut meta,
        &mut transcript,
        &mut |value| emitted.push(value.clone()),
    );
    assert_eq!(emitted.len(), 1, "one checkpoint, immediately");
    assert_eq!(emitted[0]["step"], "transcript");
    assert_eq!(emitted[0]["transcript"]["locator"], "abcd-1234-ef");
    assert_eq!(
        meta["transcript"]["locator"], "abcd-1234-ef",
        "and the meta still feeds the finish"
    );

    // A non-string id is refused wholesale, not stringified.
    let mut emitted: Vec<serde_json::Value> = Vec::new();
    fold_stream_event(
        &serde_json::json!({"type": "system", "subtype": "init", "session_id": 7}),
        &mut turns,
        &mut meta,
        &mut transcript,
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
             if [ \"$prev\" = --patch ]; then cp \"$a\" {overlay}; fi; prev=$a; done\n\
             root=$(awk -F\"'\" '/^    root: /{{print $2}}' {overlay}); d=\"$root/--p--/s\"; mkdir -p \"$d\"\n\
             printf '{{\"type\":\"session\",\"version\":0,\"id\":\"session-served\"}}\n' > \"$d/session.jsonl\"\n\
             printf '{{\"type\":\"assistant/message\",\"data\":{{\"turn\":1,\"step\":1,\"message\":{{\"source\":{{\"model\":\"served-by-dsh\"}}}}}}}}\n' >> \"$d/session.jsonl\"\n",
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

    assert_eq!(started[0]["step"], "transcript");
    assert_eq!(started[0]["transcript"]["kind"], "dsh-session");
    assert_eq!(started[1]["step"], "harness-started");
    assert!(started[1].get("model").is_none(), "{:?}", started[1]);
    let turn = started
        .iter()
        .find(|event| event["step"] == "seat-turn")
        .expect("the transcript's assistant message became a checkpoint");
    assert_eq!(turn["model"], "served-by-dsh");
    assert_eq!(invocation.session_meta["model"], "served-by-dsh");
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
    assert_eq!(&lines[4..], ["p"]);
    let written = std::fs::read_to_string(&overlay).unwrap();
    assert!(
        written.contains("- id: session-persistence-jsonl\n"),
        "{written}"
    );
    assert!(!written.contains("agent-default-model"), "{written}");
    assert!(started[1].get("model").is_none());
    // No pin, still a served model: the record carries what the harness
    // reported, never a default (decision 0031).
    assert_eq!(invocation.session_meta["model"], "served-by-dsh");

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

/// The one fact codex does NOT put on the stream its adapter folds, read
/// from the thread record through decision 0032's retained locator. The
/// file is found by the thread id codex itself announced — never by "the
/// newest file" under the home, so a concurrent seat's thread cannot lend
/// this one its effort — and the LAST `turn_context` wins, because codex
/// writes one per turn and a thread may change effort mid-seat.
#[test]
fn the_codex_thread_echo_reads_the_last_effort_by_thread_id_and_never_by_scan() {
    let home = tempfile::tempdir().unwrap();
    let dated = home.path().join("sessions/2026/09/03");
    std::fs::create_dir_all(&dated).unwrap();
    // A concurrent seat's thread, filed first and named differently.
    std::fs::write(
        dated.join("rollout-0199other.jsonl"),
        "{\"turn_context\":{\"effort\":\"minimal\"}}\n",
    )
    .unwrap();
    std::fs::write(
        dated.join("rollout-0199mine.jsonl"),
        // Two turns: the thread was opened at `low` and raised to
        // `xhigh`, and the record must say the level that ended up
        // applying rather than the one it started under.
        "{\"thread_settings_applied\":{\"reasoning_effort\":\"low\"}}\n\
         {\"turn_context\":{\"effort\":\"xhigh\"}}\n\
         not json at all\n",
    )
    .unwrap();

    let mut echo = CodexThreadEcho::default();
    echo.locate(home.path(), "0199mine");
    assert_eq!(echo.effort().as_deref(), Some("xhigh"));

    // The other seat's thread is reachable by ITS id, and only by it.
    let mut other = CodexThreadEcho::default();
    other.locate(home.path(), "0199other");
    assert_eq!(other.effort().as_deref(), Some("minimal"));

    // An id nobody filed, an empty id, and a home with no sessions tree
    // at all each leave the record saying nothing rather than guessing.
    let mut missing = CodexThreadEcho::default();
    missing.locate(home.path(), "0199absent");
    assert_eq!(missing.effort(), None);
    let mut empty = CodexThreadEcho::default();
    empty.locate(home.path(), "");
    assert_eq!(empty.effort(), None);
    let mut homeless = CodexThreadEcho::default();
    homeless.locate(std::path::Path::new("/nonexistent/codex-home"), "0199mine");
    assert_eq!(homeless.effort(), None);

    // The walk is BOUNDED: a thread filed deeper than the depth allows
    // is not found, so a large harness home cannot turn one turn into a
    // full filesystem scan.
    let deep = home.path().join("sessions/a/b/c/d/e/f/g");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(
        deep.join("rollout-0199deep.jsonl"),
        "{\"turn_context\":{\"effort\":\"high\"}}\n",
    )
    .unwrap();
    let mut too_deep = CodexThreadEcho::default();
    too_deep.locate(home.path(), "0199deep");
    assert_eq!(too_deep.effort(), None);

    // A file that is not a thread record does not answer for one.
    std::fs::write(dated.join("rollout-0199plain.txt"), "effort: high\n").unwrap();
    let mut wrong_suffix = CodexThreadEcho::default();
    wrong_suffix.locate(home.path(), "0199plain");
    assert_eq!(wrong_suffix.effort(), None);

    // A thread record that names no effort at all reports none rather
    // than the level of whatever else it could find.
    std::fs::write(dated.join("rollout-0199quiet.jsonl"), "{\"turn\":1}\n").unwrap();
    let mut quiet = CodexThreadEcho::default();
    quiet.locate(home.path(), "0199quiet");
    assert_eq!(quiet.effort(), None);
}

/// The effort clamp, at both edges. A level crosses the driver boundary
/// into an append-only journal, so it is bounded exactly as a model id
/// is and tighter: an effort is a level, never a path, an id, or a
/// sentence (decision 0035 ruling 3).
#[test]
fn an_effort_token_is_one_bounded_word_or_nothing() {
    assert_eq!(effort_token("xhigh").as_deref(), Some("xhigh"));
    assert_eq!(effort_token("  medium\n").as_deref(), Some("medium"));
    // The vocabularies actually measured spell levels with these.
    assert_eq!(
        effort_token("gpt-5.6:max_1").as_deref(),
        Some("gpt-5.6:max_1")
    );
    assert_eq!(
        effort_token(&"a".repeat(40)).as_deref(),
        Some("a".repeat(40)).as_deref()
    );
    for refused in [
        "",
        "   ",
        // A sentence, a path, and a shell fragment are none of them a level.
        "think very hard",
        "levels/high",
        "high; rm -rf /",
        "hıgh",
    ] {
        assert_eq!(effort_token(refused), None, "{refused:?} must be refused");
    }
    assert_eq!(effort_token(&"a".repeat(41)), None, "41 is over the clamp");
}

/// `--effort <level>` is the pinning grammar every adapter declares, and
/// this is where an arm whose harness spells it differently takes it out
/// of the argv. Both spellings are read; anything that is not one
/// bounded level stays in the argv so the HARNESS refuses it loudly
/// rather than this adapter dropping a pin in silence.
#[test]
fn the_effort_pin_is_split_out_in_both_spellings_and_never_dropped_silently() {
    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();

    let (effort, rest) = split_effort(&s(&["--x", "--effort", "high", "y"]));
    assert_eq!(effort.as_deref(), Some("high"));
    assert_eq!(rest, s(&["--x", "y"]));

    // The attached spelling carries its level inside the one part.
    let (effort, rest) = split_effort(&s(&["--effort=low", "--x"]));
    assert_eq!(effort.as_deref(), Some("low"));
    assert_eq!(rest, s(&["--x"]));

    // A bare `--effort` at the end of the argv carries no level, so the
    // flag travels on and codex says so itself.
    let (effort, rest) = split_effort(&s(&["--x", "--effort"]));
    assert_eq!(effort, None);
    assert_eq!(rest, s(&["--x", "--effort"]));

    // A level that is not one bounded word is not a pin: BOTH parts stay,
    // in order, so nothing is lost on the way to the harness.
    let (effort, rest) = split_effort(&s(&["--effort", "think hard", "--x"]));
    assert_eq!(effort, None);
    assert_eq!(rest, s(&["--effort", "think hard", "--x"]));
    let (effort, rest) = split_effort(&s(&["--effort=", "--x"]));
    assert_eq!(effort, None);
    assert_eq!(rest, s(&["--effort=", "--x"]));

    // The first pin wins and a second one travels on rather than
    // silently outranking it.
    let (effort, rest) = split_effort(&s(&["--effort", "high", "--effort", "low"]));
    assert_eq!(effort.as_deref(), Some("high"));
    assert_eq!(rest, s(&["--effort", "low"]));

    // Nothing to split is not an error, and changes nothing.
    let (effort, rest) = split_effort(&s(&["--x", "y"]));
    assert_eq!(effort, None);
    assert_eq!(rest, s(&["--x", "y"]));
}

/// `codex exec` takes no effort FLAG — the level is the
/// `model_reasoning_effort` config key — so the shared `--effort` pin
/// becomes the `-c key=value` override codex actually reads, cold and on
/// a resume alike. A resume that dropped it would rejoin the thread at
/// the provider's default, which is the silent-substitution case the
/// whole decision refuses.
#[test]
fn the_codex_arms_turn_the_effort_pin_into_the_config_override_it_reads() {
    assert_eq!(
        codex_effort_config("xhigh"),
        "model_reasoning_effort=\"xhigh\""
    );

    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let cold = codex_cold("codex", &s(&["--effort", "high", "--x"]), "/w");
    assert_eq!(
        cold,
        s(&[
            "codex",
            "exec",
            "--json",
            "-C",
            "/w",
            "-c",
            "model_reasoning_effort=\"high\"",
            "--x"
        ])
    );
    // No pin, no override: an unpinned seat's argv is what it always was.
    assert_eq!(
        codex_cold("codex", &s(&["--x"]), "/w"),
        s(&["codex", "exec", "--json", "-C", "/w", "--x"])
    );

    // The resume re-expresses the pin, exactly as it re-expresses the
    // sandbox class, because `codex exec resume` inherits neither.
    let launch = codex_launch(
        "codex",
        &s(&["--effort", "high", "-s", "workspace-write"]),
        "/w",
        Some("0199aaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
    );
    assert!(
        launch
            .command
            .windows(2)
            .any(|pair| pair[0] == "-c" && pair[1] == "model_reasoning_effort=\"high\""),
        "{:?}",
        launch.command
    );
    // And the pin does not read as an argv a resume cannot carry: a
    // dropped session over its own effort pin would be a regression.
    assert!(launch.resume_refusal.is_none(), "{:?}", launch.command);
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

#[cfg(unix)]
#[test]
fn codex_uses_its_own_model_header_when_the_event_stream_omits_it() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let shim = executable(
        dir.path(),
        "codex-model-header",
        "#!/bin/sh\ncat >/dev/null\nprintf 'model: gpt-from-header\\n' >&2\n",
    );
    let invocation = with_codex_bin(&shim, || {
        invoke(
            AdapterKind::Codex,
            &["--model".into(), "gpt-pinned".into()],
            "prompt",
            &json!({"workdir":dir.path()}),
            None,
            &[],
            &mut |_| {},
        )
        .unwrap()
    });
    assert_eq!(invocation.session_meta["model"], "gpt-from-header");
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
               "sandbox":"read-only"}),
        "the launch says it rejoined under the declared class"
    );
    // The fold still reads the thread out of the resumed stream, so the
    // NEXT attempt of this seat has an id to be offered in its turn.
    assert_eq!(invocation.session_meta["transcript"]["locator"], THREAD);
    assert_eq!(invocation.session_meta["cache_read_tokens"], 96);
    assert_eq!(invocation.exit_code, 0);
}

/// The sandbox travels or the resume does not (decision 0030 ruling 2),
/// and neither does anything else the seat declared that a resume cannot
/// carry. Every way an offer can fail to be taken ends in the same
/// place: the cold argv, unchanged from what it has always been, and a
/// checkpoint saying why.
#[cfg(unix)]
#[test]
fn a_class_that_cannot_travel_spawns_cold_with_the_reason_journaled() {
    let _guard = ADAPTER_ENV.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cases: [(&str, Vec<&str>, &str); 9] = [
        // Nothing declared: a codex resume does not inherit the class
        // its thread was opened under, so there is nothing to re-impose.
        ("undeclared", vec!["--model", "sol"], "sandbox-unavailable"),
        // A flag with nothing after it declares nothing either.
        ("dangling", vec!["--sandbox"], "sandbox-unavailable"),
        // A class this driver cannot spell as a config override.
        (
            "invented",
            vec!["--sandbox", "invented"],
            "unsupported-sandbox",
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
            "incompatible-argv",
        ),
        // A bypass flag beside a declared class: the class is not the
        // only way to spend the sandbox.
        (
            "bypassed",
            vec![
                "--sandbox",
                "read-only",
                "--dangerously-bypass-approvals-and-sandbox",
            ],
            "incompatible-argv",
        ),
        // A flag that never says "sandbox" and sets one anyway: a config
        // profile may carry `sandbox_mode`, and a profile outranks a `-c`
        // root override. `codex exec resume` refuses the flag outright
        // (verified, 0.148.0) — this driver refuses it first, and says so
        // instead of spending a spawn to be told.
        (
            "profiled",
            vec!["--sandbox", "read-only", "--profile", "loose"],
            "incompatible-argv",
        ),
        // `--last` picks the newest recorded session instead of the one
        // on offer. The seat's argv never redirects the engine's offer.
        (
            "redirected",
            vec!["--sandbox", "read-only", "--last"],
            "incompatible-argv",
        ),
        // A bare word lands positionally, where `codex exec resume
        // [SESSION_ID] [PROMPT]` reads it as the session — ahead of the
        // thread this driver appends.
        (
            "positional",
            vec!["--sandbox", "read-only", "some-other-thread"],
            "incompatible-argv",
        ),
        // An id that is not a plain thread id never reaches an argv.
        (
            "forged",
            vec!["--sandbox", "read-only"],
            "invalid-session-id",
        ),
    ];
    for (case, extra, refusal) in cases {
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
        assert_eq!(
            emitted[0]["resume_refusal"], refusal,
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
    let launches: Vec<&Value> = emitted
        .iter()
        .filter(|event| event["step"] == "harness-started")
        .collect();
    assert_eq!(launches[0]["launch"], "resumed");
    assert_eq!(
        *launches[1],
        json!({"step":"harness-started", "harness":"codex", "launch":"cold",
               "resume_refusal":"harness-refused"})
    );
    // What the seat gets is the COLD session, not the refusal: exit 0,
    // and the thread the cold spawn opened.
    assert_eq!(invocation.exit_code, 0);
    assert_eq!(invocation.session_meta["transcript"]["locator"], THREAD);
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

/// What the rest of the seat's argv is allowed to be on a resume: only
/// flags `codex exec resume` takes AND that cannot reach the sandbox or
/// choose the session (verified against codex-cli 0.148.0). Both
/// spellings of a value flag are the same declaration, and a value is
/// never read as a part in its own right.
#[test]
fn only_the_flags_a_resume_can_safely_carry_travel_with_it() {
    let blocker = |parts: &[&str]| {
        let passthrough: Vec<String> = parts.iter().map(|part| part.to_string()).collect();
        codex_resume_blocker(&passthrough)
    };
    assert_eq!(blocker(&[]), None);
    assert_eq!(
        blocker(&[
            "--model",
            "gpt-5.6-sol",
            "--output-schema=/tmp/s.json",
            "--json",
            "--skip-git-repo-check",
            "-i",
            "/tmp/a.png",
        ]),
        None
    );
    // A value that spells a refused flag is still just a value.
    assert_eq!(blocker(&["-m", "--last"]), None);
    // A value flag with nothing after it declares nothing.
    assert_eq!(blocker(&["--model"]), Some("--model".into()));
    for refused in [
        "-c",
        "--config",
        "--enable",
        "--disable",
        "--last",
        "--all",
        "--profile",
        "--add-dir",
        "--approve-for-me",
        "-C",
        "--ignore-rules",
        "--dangerously-bypass-approvals-and-sandbox",
        "a-bare-word",
        "--a-flag-codex-has-not-invented-yet",
        // The joined spelling is the same declaration, and gets the
        // same answer: only the flag's NAME decides.
        "--profile=loose",
        "-c=sandbox_mode=\"danger-full-access\"",
    ] {
        assert_eq!(
            blocker(&["--model", "sol", refused]),
            Some(refused.to_string()),
            "{refused} may not travel to a resume"
        );
    }
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
    assert!(launches[0]["data"].get("session_id").is_none());
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
printf '{"type":"assistant/message","data":{"turn":1,"step":1,"message":{"source":{"model":"served-by-dsh"}},"usage":{"inputTokens":10,"outputTokens":2,"cacheReadTokens":4}}}\n' >> "$f"
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
        None,
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
    assert_eq!(emitted[0]["step"], "transcript");
    assert_eq!(emitted[1]["step"], "harness-started");
    // Retention: the seat's root sits under this test's DSH_HOME, the
    // journal's relative locator names it, and it is still there now
    // that the seat has concluded — the operator's, not the void's.
    let kept: Vec<_> = std::fs::read_dir(dir.path().join("sessions").join("brokkr"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert!(kept[0].file_name().to_string_lossy().starts_with("seat-"));
    assert!(kept[0].path().is_dir());
    let transcript = &emitted[0]["transcript"];
    assert_eq!(transcript["kind"], "dsh-session");
    assert_eq!(transcript["home"], dir.path().to_string_lossy().as_ref());
    let locator = transcript["locator"].as_str().unwrap();
    assert_eq!(dir.path().join(locator), kept[0].path());
    assert!(locator.contains("sessions/brokkr/"), "{locator}");
    assert!(locator.chars().count() <= 80, "{locator}");
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
    assert_eq!(meta["transcript"], *transcript);
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
        "the seat's transcript root did not survive the seat: {locator}"
    );

    // A harness that writes no transcript at all still concludes: the
    // seat is silent, not broken. Named workdir absent too, so the
    // driver falls back to its own directory as every other arm does.
    let quiet = executable(dir.path(), "quiet-dsh", "#!/bin/sh\nexit 0\n");
    std::env::set_var("BROKKR_DSH_BIN", &quiet);
    let mut emitted: Vec<Value> = Vec::new();
    let invocation = invoke(
        AdapterKind::Dsh,
        &[],
        "p",
        &json!({}),
        None,
        &[],
        &mut |event| emitted.push(event.clone()),
    )
    .unwrap();
    assert_eq!(emitted.len(), 2, "{emitted:?}");
    assert_eq!(emitted[0]["step"], "transcript");
    assert_eq!(emitted[1]["step"], "harness-started");
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
fn dsh_fold_uses_the_root_locator_and_ignores_internal_session_ids() {
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
    assert_eq!(emitted.len(), 1, "{emitted:?}");
    assert!(meta.get("session_id").is_none());
    assert_eq!(emitted[0]["turn"], 1);
    assert!(emitted[0].get("input_tokens").is_none());
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
    assert!(emitted.is_empty(), "{emitted:?}");
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
    let mut transcript = Transcript::resolve(TranscriptKind::CodexThread).unwrap();
    let mut echo = CodexThreadEcho::default();
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
        fold_codex_event(
            &event,
            &mut turn,
            &mut meta,
            &mut transcript,
            &mut echo,
            &mut |value| emitted.push(value.clone()),
        );
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
        None,
        &[],
        &mut |event| {
            emitted.push(event.clone());
            if event["step"] == "transcript" {
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
    // The launch checkpoint (decision 0030) comes first; the thread id
    // is the very next thing journaled, before any turn.
    assert_eq!(emitted[0]["step"], "harness-started");
    assert_eq!(emitted[1]["step"], "transcript");
    assert_eq!(emitted[1]["transcript"]["kind"], "codex-thread");
    assert_eq!(emitted[1]["transcript"]["locator"], thread);
    assert_eq!(emitted.last().unwrap()["step"], "turn-completed");
    let meta = &invocation.session_meta;
    assert_eq!(meta["transcript"]["locator"], thread);
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
    let mut transcript = Transcript::resolve(TranscriptKind::CodexThread).unwrap();
    fold_codex_event(
        &json!({"type": "thread.started", "thread_id": "x".repeat(200)}),
        &mut turn,
        &mut meta,
        &mut transcript,
        &mut CodexThreadEcho::default(),
        &mut |value| emitted.push(value.clone()),
    );
    assert_eq!(
        meta["transcript"]["locator"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        80
    );
    assert_eq!(emitted[0]["transcript"], meta["transcript"]);
}

#[test]
fn a_dsh_tool_call_before_any_assembled_message_invents_no_turn() {
    // dsh 0.1.0-rc.6 always writes the `assistant/message` that asked
    // for a tool before the `tool/call` itself, so this is the shape of
    // a log that got cut short or reordered. The fold reports turn 0
    // without claiming a turn dsh never assembled. A zero-valued turn
    // is not a measurement under the seat-record contract.
    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    fold_dsh_event(
        &json!({"type": "tool/call", "data": {"name": "fs_read"}}),
        &mut turns,
        &mut meta,
        &mut |value| emitted.push(value.clone()),
    );
    assert!(emitted.is_empty());
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
    assert_eq!(emitted[0]["step"], "transcript");
    assert_eq!(emitted[1]["step"], "harness-started");
}

#[test]
fn claude_fold_journals_a_toolless_turn_once_and_a_nameless_tool_use() {
    // Two shapes the stream really produces: an assistant turn that
    // calls nothing, and a tool_use whose name the harness omitted.
    // The first must still journal its turn exactly once, from the
    // base record; the second keeps its target and gains no empty tool.
    let mut turns = 0;
    let mut meta = Map::new();
    let mut emitted: Vec<Value> = Vec::new();
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession).unwrap();

    let event = json!({"type": "assistant", "message": {"content": [
        {"type": "text", "text": "reasoning the operator never sees"},
    ]}});
    fold_stream_event(&event, &mut turns, &mut meta, &mut transcript, &mut |c| {
        emitted.push(c.clone())
    });
    assert_eq!(emitted.len(), 1, "the toolless turn is journaled once");
    assert!(emitted[0].get("tool").is_none());
    assert!(
        !serde_json::to_string(&emitted[0])
            .unwrap()
            .contains("reasoning the operator never sees"),
        "prose never reaches the record"
    );

    emitted.clear();
    let event = json!({"type": "assistant", "message": {"content": [
        {"type": "tool_use", "input": {"file_path": "src/main.rs"}},
    ]}});
    fold_stream_event(&event, &mut turns, &mut meta, &mut transcript, &mut |c| {
        emitted.push(c.clone())
    });
    assert_eq!(emitted.len(), 1);
    assert!(
        emitted[0].get("tool").is_none(),
        "an unnamed tool is absent, never the empty string"
    );
    assert_eq!(emitted[0]["target"], "src/main.rs");
}
